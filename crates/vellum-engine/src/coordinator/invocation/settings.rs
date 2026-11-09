//! Settings jobs share the existing once-only ledger and direct native staging.
use super::*;
use crate::settings::{SettingChangeV1, SettingsError, SettingsRequestCapture};
#[cfg(test)]
mod tests;
#[derive(Debug)]
pub(super) struct SettingsIntent {
    request: SettingsRequestCapture,
    refusal: Option<SettingsError>,
}
fn admission_error(error: SettingsError) -> InvocationError {
    match error {
        SettingsError::Limit => InvocationError::Limit,
        SettingsError::Capacity => InvocationError::Capacity,
        _ => InvocationError::MalformedIntent,
    }
}
fn capture_budget(maximum: usize) -> Result<usize, InvocationError> {
    maximum
        .checked_sub(
            std::mem::size_of::<SettingsIntent>() - std::mem::size_of::<SettingsRequestCapture>(),
        )
        .ok_or(InvocationError::Limit)
}
impl SourceCoordinator {
    pub fn submit_settings_change(
        &mut self,
        id: &InvocationId,
        changes: &[SettingChangeV1],
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
                let BoundIntent::Settings(original) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                let request = SettingsRequestCapture::copy(
                    changes,
                    original.request.profile,
                    capture_budget(self.invocations.limits.max_intent_bytes)?,
                )
                .map_err(admission_error)?;
                if !original.request.same(&request).map_err(admission_error)? {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let profile = self
                .settings_activation
                .ok_or(InvocationError::Unavailable)?;
            let request = SettingsRequestCapture::copy(
                changes,
                profile,
                capture_budget(self.invocations.limits.max_intent_bytes)?,
            )
            .map_err(admission_error)?;
            let prepared = self.prepare_settings_change(changes, r);
            let refusal = prepared.as_ref().err().copied();
            self.install_invocation_admission(
                id,
                BoundIntent::Settings(SettingsIntent { request, refusal }),
                Phase::SettingsPending(prepared),
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
    pub(super) fn run_settings(
        &mut self,
        id: &InvocationId,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated settings slot");
        let Phase::SettingsPending(prepared) = std::mem::replace(&mut slot.phase, Phase::Running)
        else {
            unreachable!("validated settings job")
        };
        self.faulted = true;
        let intact = match (&slot.intent, &prepared) {
            (Some(BoundIntent::Settings(original)), Ok(prepared)) => {
                original.refusal.is_none()
                    && Arc::ptr_eq(&prepared.owner, &self.namespace)
                    && prepared.plan.matches(&original.request).unwrap_or(false)
            }
            (Some(BoundIntent::Settings(original)), Err(error)) => original.refusal == Some(*error),
            _ => false,
        };
        if !intact {
            crate::settings::observe::<()>(
                r,
                Boundary::SettingsChangePublication,
                &Err(SettingsError::Faulted),
            );
            return Err(InvocationError::Faulted);
        }
        let decision = prepared.and_then(|prepared| self.stage_settings_change(prepared, r));
        if matches!(decision, Err(SettingsError::Faulted)) {
            crate::settings::observe(r, Boundary::SettingsChangePublication, &decision);
            return Err(InvocationError::Faulted);
        }
        let outcome = match decision {
            Ok(stage) => {
                let outcome = InvocationOutcome::SettingsCommitted(stage.outcome);
                stage.install(self);
                #[cfg(test)]
                if self.settings_publication_fault {
                    panic!("injected settings publication interruption")
                }
                outcome
            }
            Err(error) => InvocationOutcome::SettingsRejected(error),
        };
        let observation = match &outcome {
            InvocationOutcome::SettingsCommitted(_) => Ok(()),
            InvocationOutcome::SettingsRejected(error) => Err(*error),
            _ => unreachable!("closed settings outcome"),
        };
        self.install_invocation_terminal(id, outcome);
        self.faulted = false;
        crate::settings::observe(r, Boundary::SettingsChangePublication, &observation);
        Ok(InvocationState::Finished)
    }
}
