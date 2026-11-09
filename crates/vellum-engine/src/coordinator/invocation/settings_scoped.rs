//! Scoped jobs require current host facts at service, not captured permission reuse.
use super::super::settings_reads::observe;
use super::*;
use crate::settings::SettingsError;
pub(super) mod arguments;
pub use arguments::*;
#[cfg(test)]
mod tests;
pub(super) fn error(value: SettingsChangeErrorV2) -> InvocationError {
    use SettingsChangeErrorV2::{Reads, Settings};
    match value {
        Settings(SettingsError::Limit)
        | Reads(CoordinatorError::Limit)
        | Reads(CoordinatorError::ReadDependency(ReadDependencyError::Limit)) => {
            InvocationError::Limit
        }
        Settings(SettingsError::Capacity)
        | Reads(CoordinatorError::Capacity)
        | Reads(CoordinatorError::ReadDependency(ReadDependencyError::Capacity)) => {
            InvocationError::Capacity
        }
        e if e.faulted() => InvocationError::Faulted,
        Settings(
            SettingsError::Unavailable | SettingsError::Stale | SettingsError::VersionExhausted,
        )
        | Reads(
            CoordinatorError::Unavailable
            | CoordinatorError::Stale
            | CoordinatorError::VersionExhausted,
        ) => InvocationError::Unavailable,
        _ => InvocationError::MalformedIntent,
    }
}
impl SourceCoordinator {
    pub fn submit_scoped_settings_change(
        &mut self,
        id: &InvocationId,
        request: &ScopedSettingsRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        g: &TrustedSettingsMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        let repeated = self.invocation_slot(id).is_ok_and(|s| s.intent.is_some());
        let result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(id)?;
            if let Some(original) = &slot.intent {
                let BoundIntent::ScopedSettings(original) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                if !original
                    .replay_matches(request, c, self.settings_job_parent_limit())
                    .map_err(error)?
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let prepared = self
                .capture_scoped_settings(request, c, g)
                .and_then(|captured| self.prepare_scoped_capture(captured, request, c, g, r));
            observe(r, Boundary::ScopedSettingsPreparation, &prepared);
            let prepared = prepared.map_err(error)?;
            let captured = prepared.capture.clone();
            self.install_invocation_admission(
                id,
                BoundIntent::ScopedSettings(captured),
                Phase::ScopedSettingsPending(prepared),
            );
            Ok(InvocationState::Pending)
        })();
        if matches!(result, Err(InvocationError::Faulted)) {
            self.faulted = true;
        }
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
    pub fn run_scoped_settings_change(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        g: &TrustedSettingsMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let terminal = self
            .invocation_slot(id)
            .is_ok_and(|s| matches!(s.phase, Phase::Terminal(_)));
        let result = self.run_scoped_settings_inner(id, c, g, r);
        capture_timed(
            r,
            if terminal {
                Boundary::InvocationReplay
            } else {
                Boundary::InvocationExecution
            },
            &result,
            timing.finish(),
        );
        result
    }
    fn run_scoped_settings_inner(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        g: &TrustedSettingsMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        if self.faulted {
            return Err(InvocationError::Faulted);
        }
        let slot = self.invocation_slot(id)?;
        let Some(BoundIntent::ScopedSettings(original)) = &slot.intent else {
            return Err(InvocationError::IntentMismatch);
        };
        if original.context != *c {
            return Err(InvocationError::Unavailable);
        }
        match slot.phase {
            Phase::Terminal(_) => return Ok(InvocationState::Finished),
            Phase::Running => return Err(InvocationError::Faulted),
            Phase::ScopedSettingsPending(_) => {}
            _ => return Err(InvocationError::IntentMismatch),
        }
        if self.service_requested_stop(id, r)? {
            return Ok(InvocationState::Finished);
        }
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated scoped settings slot");
        let Phase::ScopedSettingsPending(prepared) =
            std::mem::replace(&mut slot.phase, Phase::Running)
        else {
            unreachable!("scoped settings phase")
        };
        self.faulted = true;
        let intact = matches!(&slot.intent, Some(BoundIntent::ScopedSettings(original)) if Arc::ptr_eq(original, &prepared.capture) && Arc::ptr_eq(&original.owner, &self.namespace));
        if !intact {
            observe::<()>(
                r,
                Boundary::ScopedSettingsPublication,
                &Err(SettingsChangeErrorV2::Settings(SettingsError::Faulted)),
            );
            return Err(InvocationError::Faulted);
        }
        let decision = self.stage_scoped_settings(prepared, c, g, r);
        if decision.as_ref().err().is_some_and(|e| e.faulted()) {
            observe(r, Boundary::ScopedSettingsPublication, &decision);
            return Err(InvocationError::Faulted);
        }
        let observation = decision.as_ref().map(|_| ()).map_err(|e| *e);
        let outcome = match decision {
            Ok(stage) => {
                let outcome = InvocationOutcome::ScopedSettingsCommitted(ScopedSettingsUpdateV1 {
                    changed: stage.outcome.changed,
                });
                stage.install(self);
                #[cfg(test)]
                if self.settings_publication_fault {
                    panic!("injected scoped settings publication interruption");
                }
                outcome
            }
            Err(e) => InvocationOutcome::ScopedSettingsRejected(e),
        };
        self.install_invocation_terminal(id, outcome);
        self.faulted = false;
        observe(r, Boundary::ScopedSettingsPublication, &observation);
        Ok(InvocationState::Finished)
    }
}
