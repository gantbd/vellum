//! Once-only declaration jobs retain original declarations, not newer authored roots.
use super::super::settings_reads::observe;
use super::*;
#[cfg(test)]
mod tests;
use crate::settings::{
    DefinitionCapture, SettingsDeclarationV1, SettingsError, SettingsInterpretationEpochsV1,
};
#[derive(Debug)]
pub(super) struct DefinitionIntent {
    capture: Arc<DefinitionCapture>,
    expected: SettingsInterpretationEpochsV1,
    reads: read_dependency::ReadCapture,
    refusal: Option<SettingsChangeErrorV2>,
}
impl DefinitionIntent {
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.reads.refers_to_document(document)
    }
}
fn settings_error(error: SettingsError) -> InvocationError {
    match error {
        SettingsError::Limit => InvocationError::Limit,
        SettingsError::Capacity => InvocationError::Capacity,
        _ => InvocationError::MalformedIntent,
    }
}
fn read_error(error: ReadDependencyError) -> InvocationError {
    match error {
        ReadDependencyError::Capacity => InvocationError::Capacity,
        ReadDependencyError::Limit => InvocationError::Limit,
        _ => InvocationError::MalformedIntent,
    }
}
impl SourceCoordinator {
    pub fn submit_settings_definition(
        &mut self,
        id: &InvocationId,
        expected: SettingsInterpretationEpochsV1,
        declaration: &SettingsDeclarationV1,
        request: &ReadDependencyRequestV3,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        let repeated = self.invocation_slot(id).is_ok_and(|s| s.intent.is_some());
        let result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(id)?;
            DefinitionCapture::validate_expected(expected).map_err(settings_error)?;
            let input = read_dependency::ReadInput::V3(request);
            let read_bytes = input.retained_bytes(&[]).ok_or(InvocationError::Limit)?;
            let header = std::mem::size_of::<DefinitionIntent>()
                - std::mem::size_of::<read_dependency::ReadCapture>();
            let ceiling = self
                .invocations
                .limits
                .max_intent_bytes
                .checked_sub(header)
                .and_then(|n| n.checked_sub(read_bytes))
                .ok_or(InvocationError::Limit)?;
            let reads = input.copy(&[]).map_err(read_error)?;
            if let Some(original) = &slot.intent {
                let BoundIntent::SettingsDefinition(original) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                let capture = original
                    .capture
                    .recopy(declaration, ceiling)
                    .map_err(settings_error)?;
                if expected != original.expected
                    || original.reads != reads
                    || !original.capture.same(&capture).map_err(settings_error)?
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let limits = self
                .settings_definition_limits
                .ok_or(InvocationError::Unavailable)?;
            let root = self.settings.as_ref().ok_or(InvocationError::Unavailable)?;
            let capture = Arc::new(
                DefinitionCapture::from_state(declaration, root, limits, ceiling)
                    .map_err(settings_error)?,
            );
            // This is checked before binding; all retained request payload is bounded.
            if capture.bytes() > ceiling {
                return Err(InvocationError::Limit);
            }
            let prepared = reads
                .copy_owned()
                .map_err(|e| SettingsChangeErrorV2::Reads(e.into()))
                .and_then(|reads| {
                    self.prepare_definition_capture(expected, capture.clone(), reads, r)
                });
            observe(r, Boundary::SettingsDefinitionPreparation, &prepared);
            let refusal = prepared.as_ref().err().copied();
            self.install_invocation_admission(
                id,
                BoundIntent::SettingsDefinition(DefinitionIntent {
                    capture,
                    expected,
                    reads,
                    refusal,
                }),
                Phase::SettingsDefinitionPending(prepared),
            );
            Ok(InvocationState::Pending)
        })();
        capture(
            r,
            if repeated {
                Boundary::InvocationReplay
            } else {
                Boundary::InvocationAdmission
            },
            &result,
        );
        result
    }
    pub(super) fn run_settings_definition(
        &mut self,
        id: &InvocationId,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated definition slot");
        let Phase::SettingsDefinitionPending(prepared) =
            std::mem::replace(&mut slot.phase, Phase::Running)
        else {
            unreachable!("definition phase")
        };
        self.faulted = true;
        let intact = match (&slot.intent, &prepared) {
            (Some(BoundIntent::SettingsDefinition(original)), Ok(p)) => {
                original.refusal.is_none()
                    && Arc::ptr_eq(&p.owner, &self.namespace)
                    && p.read_count == original.reads.predicates.len()
                    && p.reads == original.reads
                    && p.plan.matches(&original.capture, original.expected)
            }
            (Some(BoundIntent::SettingsDefinition(original)), Err(e)) => {
                original.refusal == Some(*e)
            }
            _ => false,
        };
        if !intact {
            observe::<()>(
                r,
                Boundary::SettingsDefinitionPublication,
                &Err(SettingsChangeErrorV2::Settings(SettingsError::Faulted)),
            );
            return Err(InvocationError::Faulted);
        }
        let decision = prepared.and_then(|p| self.stage_settings_definition(p, r));
        if decision.as_ref().err().is_some_and(|e| e.faulted()) {
            observe(r, Boundary::SettingsDefinitionPublication, &decision);
            return Err(InvocationError::Faulted);
        }
        let observation = decision.as_ref().map(|_| ()).map_err(|e| *e);
        let outcome = match decision {
            Ok(stage) => {
                let outcome = InvocationOutcome::SettingsDefinitionCommitted(stage.outcome);
                stage.install(self);
                #[cfg(test)]
                if self.settings_publication_fault {
                    panic!("injected definition publication interruption");
                }
                outcome
            }
            Err(e) => InvocationOutcome::SettingsDefinitionRejected(e),
        };
        self.install_invocation_terminal(id, outcome);
        self.faulted = false;
        observe(r, Boundary::SettingsDefinitionPublication, &observation);
        Ok(InvocationState::Finished)
    }
}
