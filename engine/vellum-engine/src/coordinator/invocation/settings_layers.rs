//! Original layer lifecycle intent uses the common spent invocation ledger.
use super::super::settings_reads::observe;
use super::*;
#[cfg(test)]
mod tests;
use crate::settings::{LayerCapture, SettingsError, SettingsLayerBasisV1, SettingsLayerChangeV1};
#[derive(Debug)]
pub(super) struct LayerIntent {
    capture: Arc<LayerCapture>,
    expected: SettingsLayerBasisV1,
    reads: read_dependency::ReadCapture,
    refusal: Option<SettingsChangeErrorV2>,
}
impl LayerIntent {
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.reads.refers_to_document(document)
    }
    pub(super) fn from_prepared(
        capture: Arc<LayerCapture>,
        expected: SettingsLayerBasisV1,
        reads: read_dependency::ReadCapture,
    ) -> Self {
        Self {
            capture,
            expected,
            reads,
            refusal: None,
        }
    }
}
fn settings_error(e: SettingsError) -> InvocationError {
    match e {
        SettingsError::Limit => InvocationError::Limit,
        SettingsError::Capacity => InvocationError::Capacity,
        _ => InvocationError::MalformedIntent,
    }
}
fn read_error(e: ReadDependencyError) -> InvocationError {
    match e {
        ReadDependencyError::Limit => InvocationError::Limit,
        ReadDependencyError::Capacity => InvocationError::Capacity,
        _ => InvocationError::MalformedIntent,
    }
}
impl SourceCoordinator {
    pub fn submit_settings_layers(
        &mut self,
        id: &InvocationId,
        expected: SettingsLayerBasisV1,
        input: &SettingsLayerChangeV1,
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
            expected.validate().map_err(settings_error)?;
            let reads = read_dependency::ReadInput::V3(request);
            let bytes = reads.retained_bytes(&[]).ok_or(InvocationError::Limit)?;
            let header = std::mem::size_of::<LayerIntent>()
                - std::mem::size_of::<read_dependency::ReadCapture>();
            let ceiling = self
                .invocations
                .limits
                .max_intent_bytes
                .checked_sub(header)
                .and_then(|n| n.checked_sub(bytes))
                .ok_or(InvocationError::Limit)?;
            let reads = reads.copy(&[]).map_err(read_error)?;
            if let Some(original) = &slot.intent {
                let BoundIntent::SettingsLayers(original) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                let copy = original
                    .capture
                    .recopy(input, ceiling)
                    .map_err(settings_error)?;
                if original.expected != expected
                    || original.reads != reads
                    || !original.capture.same(&copy).map_err(settings_error)?
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let limits = self
                .settings_layer_limits
                .ok_or(InvocationError::Unavailable)?;
            let current = self.settings.as_ref().ok_or(InvocationError::Unavailable)?;
            let capture = Arc::new(
                LayerCapture::copy(input, current, limits, ceiling).map_err(settings_error)?,
            );
            if capture.bytes() > ceiling {
                return Err(InvocationError::Limit);
            }
            let prepared = reads
                .copy_owned()
                .map_err(|e| SettingsChangeErrorV2::Reads(e.into()))
                .and_then(|reads| self.prepare_layer_capture(expected, capture.clone(), reads, r));
            observe(r, Boundary::SettingsLayerPreparation, &prepared);
            let refusal = prepared.as_ref().err().copied();
            self.install_invocation_admission(
                id,
                BoundIntent::SettingsLayers(LayerIntent {
                    capture,
                    expected,
                    reads,
                    refusal,
                }),
                Phase::SettingsLayersPending(prepared),
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
    pub(super) fn run_settings_layers(
        &mut self,
        id: &InvocationId,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated layer slot");
        let Phase::SettingsLayersPending(prepared) =
            std::mem::replace(&mut slot.phase, Phase::Running)
        else {
            unreachable!("layer phase")
        };
        self.faulted = true;
        let intact = match (&slot.intent, &prepared) {
            (Some(BoundIntent::SettingsLayers(original)), Ok(p)) => {
                original.refusal.is_none()
                    && Arc::ptr_eq(&p.owner, &self.namespace)
                    && p.read_count == original.reads.predicates.len()
                    && p.reads == original.reads
                    && p.plan.matches(&original.capture, original.expected)
            }
            (Some(BoundIntent::SettingsLayers(original)), Err(e)) => original.refusal == Some(*e),
            _ => false,
        };
        let original_capture = match &slot.intent {
            Some(BoundIntent::SettingsLayers(original)) => Some(original.capture.clone()),
            _ => None,
        };
        if !intact {
            observe::<()>(
                r,
                Boundary::SettingsLayerPublication,
                &Err(SettingsChangeErrorV2::Settings(SettingsError::Faulted)),
            );
            return Err(InvocationError::Faulted);
        }
        let decision = prepared
            .and_then(|p| self.stage_settings_layers(p, r))
            .and_then(|stage| {
                let capture = original_capture
                    .as_ref()
                    .ok_or(SettingsChangeErrorV2::Settings(SettingsError::Faulted))?;
                let result = self.stage_created_layer_results(id, capture, &stage, r)?;
                Ok((stage, result))
            });
        if decision.as_ref().err().is_some_and(|e| e.faulted()) {
            observe(r, Boundary::SettingsLayerPublication, &decision);
            return Err(InvocationError::Faulted);
        }
        let observation = decision.as_ref().map(|_| ()).map_err(|e| *e);
        let mut created_result = None;
        let outcome = match decision {
            Ok((stage, result)) => {
                created_result = result;
                let outcome = InvocationOutcome::SettingsLayersCommitted(stage.outcome);
                stage.install(self);
                #[cfg(test)]
                if self.settings_publication_fault {
                    panic!("injected layer lifecycle publication interruption");
                }
                outcome
            }
            Err(e) => InvocationOutcome::SettingsLayersRejected(e),
        };
        self.install_invocation_terminal(id, outcome);
        self.publish_created_layer_results(id, created_result, r);
        self.faulted = false;
        observe(r, Boundary::SettingsLayerPublication, &observation);
        Ok(InvocationState::Finished)
    }
}
