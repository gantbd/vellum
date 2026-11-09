//! v2 settings jobs retain complete additional read meaning in the shared ledger.
use super::*;
use crate::settings::{SettingChangeV1, SettingsError, SettingsRequestCapture};
#[cfg(test)]
mod tests;
#[derive(Debug)]
pub(super) struct SettingsReadIntent {
    request: SettingsRequestCapture,
    reads: read_dependency::ReadCapture,
    refusal: Option<SettingsChangeErrorV2>,
}
impl SettingsReadIntent {
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.reads.refers_to_document(document)
    }
}
fn read_error(error: ReadDependencyError) -> InvocationError {
    match error {
        ReadDependencyError::Capacity => InvocationError::Capacity,
        ReadDependencyError::Limit => InvocationError::Limit,
        ReadDependencyError::InvalidVersion | ReadDependencyError::DuplicateSubject => {
            InvocationError::MalformedIntent
        }
    }
}
fn settings_error(error: SettingsError) -> InvocationError {
    match error {
        SettingsError::Limit => InvocationError::Limit,
        SettingsError::Capacity => InvocationError::Capacity,
        _ => InvocationError::MalformedIntent,
    }
}
fn request_budget(maximum: usize, read_bytes: usize) -> Result<usize, InvocationError> {
    let header = std::mem::size_of::<SettingsReadIntent>()
        - std::mem::size_of::<SettingsRequestCapture>()
        - std::mem::size_of::<read_dependency::ReadCapture>();
    maximum
        .checked_sub(header)
        .and_then(|n| n.checked_sub(read_bytes))
        .ok_or(InvocationError::Limit)
}
impl SourceCoordinator {
    pub fn submit_settings_change_with_reads(
        &mut self,
        id: &InvocationId,
        changes: &[SettingChangeV1],
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
            let input = read_dependency::ReadInput::V3(request);
            let read_bytes = input.retained_bytes(&[]).ok_or(InvocationError::Limit)?;
            let budget = request_budget(self.invocations.limits.max_intent_bytes, read_bytes)?;
            let reads = read_dependency::ReadInput::V3(request)
                .copy(&[])
                .map_err(read_error)?;
            if let Some(original) = &slot.intent {
                let BoundIntent::DependentSettings(original) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                let settings =
                    SettingsRequestCapture::copy(changes, original.request.profile, budget)
                        .map_err(settings_error)?;
                if original.reads != reads
                    || !original.request.same(&settings).map_err(settings_error)?
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let profile = self
                .settings_activation
                .ok_or(InvocationError::Unavailable)?;
            let settings =
                SettingsRequestCapture::copy(changes, profile, budget).map_err(settings_error)?;
            let prepared = (|| {
                let copy = reads
                    .copy_owned()
                    .map_err(|e| SettingsChangeErrorV2::Reads(e.into()))?;
                self.prepare_settings_read_capture(changes, copy, r)
            })();
            super::super::settings_reads::observe(
                r,
                Boundary::SettingsDependencyPreparation,
                &prepared,
            );
            let refusal = prepared.as_ref().err().copied();
            self.install_invocation_admission(
                id,
                BoundIntent::DependentSettings(SettingsReadIntent {
                    request: settings,
                    reads,
                    refusal,
                }),
                Phase::DependentSettingsPending(prepared),
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
    pub(super) fn run_settings_with_reads(
        &mut self,
        id: &InvocationId,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated settings v2 slot");
        let Phase::DependentSettingsPending(prepared) =
            std::mem::replace(&mut slot.phase, Phase::Running)
        else {
            unreachable!("settings v2 phase")
        };
        self.faulted = true;
        let intact = match (&slot.intent, &prepared) {
            (Some(BoundIntent::DependentSettings(original)), Ok(prepared)) => {
                original.refusal.is_none()
                    && prepared.reads_required
                    && prepared.read_count == original.reads.predicates.len()
                    && Arc::ptr_eq(&prepared.settings.owner, &self.namespace)
                    && prepared.reads == original.reads
                    && prepared
                        .settings
                        .plan
                        .matches(&original.request)
                        .unwrap_or(false)
            }
            (Some(BoundIntent::DependentSettings(original)), Err(error)) => {
                original.refusal == Some(*error)
            }
            _ => false,
        };
        if !intact {
            super::super::settings_reads::observe::<()>(
                r,
                Boundary::SettingsDependencyPublication,
                &Err(SettingsChangeErrorV2::Settings(SettingsError::Faulted)),
            );
            return Err(InvocationError::Faulted);
        }
        let decision = prepared.and_then(|p| self.stage_settings_with_reads(p, r));
        if decision.as_ref().err().is_some_and(|error| error.faulted()) {
            super::super::settings_reads::observe(
                r,
                Boundary::SettingsDependencyPublication,
                &decision,
            );
            return Err(InvocationError::Faulted);
        }
        let observation = decision.as_ref().map(|_| ()).map_err(|e| *e);
        let outcome = match decision {
            Ok(stage) => {
                let outcome = InvocationOutcome::SettingsCommitted(stage.outcome);
                stage.install(self);
                #[cfg(test)]
                if self.settings_publication_fault {
                    panic!("injected dependent settings publication interruption")
                };
                outcome
            }
            Err(error) => InvocationOutcome::SettingsRejectedV2(error),
        };
        self.install_invocation_terminal(id, outcome);
        self.faulted = false;
        super::super::settings_reads::observe(
            r,
            Boundary::SettingsDependencyPublication,
            &observation,
        );
        Ok(InvocationState::Finished)
    }
}
