//! Fact-bearing declaration-role service over the common ledger/publisher.
use super::super::settings_reads::observe;
use super::settings_scoped::error;
use super::*;
use crate::settings::SettingsError;
pub(super) mod arguments;
pub use arguments::*;
#[cfg(test)]
mod tests;
impl SourceCoordinator {
    pub fn submit_scoped_settings_definition(
        &mut self,
        id: &InvocationId,
        request: &ScopedSettingsDefinitionRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsDefinitionMutationFactsV1<'_>,
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
                let BoundIntent::ScopedDefinition(original) = original else {
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
                .capture_scoped_definition(request, c, f)
                .and_then(|capture| self.prepare_scoped_definition_capture(capture, c, f, r));
            observe(r, Boundary::ScopedDefinitionPreparation, &prepared);
            let prepared = prepared.map_err(error)?;
            let original = prepared.capture.clone();
            self.install_invocation_admission(
                id,
                BoundIntent::ScopedDefinition(original),
                Phase::ScopedDefinitionPending(prepared),
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
    pub fn run_scoped_settings_definition(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsDefinitionMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let terminal = self
            .invocation_slot(id)
            .is_ok_and(|s| matches!(s.phase, Phase::Terminal(_)));
        let result = self.run_scoped_definition_inner(id, c, f, r);
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
    fn run_scoped_definition_inner(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsDefinitionMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        if self.faulted {
            return Err(InvocationError::Faulted);
        }
        let slot = self.invocation_slot(id)?;
        let Some(BoundIntent::ScopedDefinition(original)) = &slot.intent else {
            return Err(InvocationError::IntentMismatch);
        };
        if original.context != *c {
            return Err(InvocationError::Unavailable);
        }
        match slot.phase {
            Phase::Terminal(_) => return Ok(InvocationState::Finished),
            Phase::Running => return Err(InvocationError::Faulted),
            Phase::ScopedDefinitionPending(_) => {}
            _ => return Err(InvocationError::IntentMismatch),
        }
        if self.service_requested_stop(id, r)? {
            return Ok(InvocationState::Finished);
        }
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated scoped definition slot");
        let Phase::ScopedDefinitionPending(p) = std::mem::replace(&mut slot.phase, Phase::Running)
        else {
            unreachable!("scoped definition phase")
        };
        self.faulted = true;
        if !matches!(&slot.intent,Some(BoundIntent::ScopedDefinition(original))if Arc::ptr_eq(original,&p.capture)&&Arc::ptr_eq(&original.owner,&self.namespace))
        {
            observe::<()>(
                r,
                Boundary::ScopedDefinitionPublication,
                &Err(SettingsChangeErrorV2::Settings(SettingsError::Faulted)),
            );
            return Err(InvocationError::Faulted);
        }
        let decision = self.stage_scoped_definition(p, c, f, r);
        if decision.as_ref().err().is_some_and(|e| e.faulted()) {
            observe(r, Boundary::ScopedDefinitionPublication, &decision);
            return Err(InvocationError::Faulted);
        }
        let observation = decision.as_ref().map(|_| ()).map_err(|e| *e);
        let outcome = match decision {
            Ok(stage) => {
                let outcome =
                    InvocationOutcome::ScopedDefinitionCommitted(ScopedSettingsUpdateV1 {
                        changed: stage.outcome.changed,
                    });
                stage.install(self);
                #[cfg(test)]
                if self.settings_publication_fault {
                    panic!("injected scoped definition publication interruption");
                }
                outcome
            }
            Err(e) => InvocationOutcome::ScopedDefinitionRejected(e),
        };
        self.install_invocation_terminal(id, outcome);
        self.faulted = false;
        observe(r, Boundary::ScopedDefinitionPublication, &observation);
        Ok(InvocationState::Finished)
    }
}
