//! Fact-bearing lifecycle-role service over the common ledger/publisher.
use super::super::settings_reads::observe;
use super::settings_scoped::error;
use super::*;
use crate::settings::SettingsError;
pub(super) mod arguments;
pub use arguments::*;
enum LifecycleSubmission<'a, 'b> {
    Native(&'a ScopedSettingsLifecycleRequestV1<'b>),
    Basis(&'a BasisSettingsLifecycleRequestV1<'b>),
}
impl SourceCoordinator {
    pub fn submit_basis_settings_lifecycle(
        &mut self,
        id: &InvocationId,
        request: &BasisSettingsLifecycleRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_lifecycle_request(id, LifecycleSubmission::Basis(request), c, f, r)
    }
    pub fn submit_scoped_settings_lifecycle(
        &mut self,
        id: &InvocationId,
        request: &ScopedSettingsLifecycleRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_lifecycle_request(id, LifecycleSubmission::Native(request), c, f, r)
    }
    fn submit_lifecycle_request(
        &mut self,
        id: &InvocationId,
        request: LifecycleSubmission<'_, '_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let repeated = self.invocation_slot(id).is_ok_and(|s| s.intent.is_some());
        let result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(id)?;
            if let Some(original) = &slot.intent {
                let BoundIntent::ScopedLifecycle(original) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                if !match request {
                    LifecycleSubmission::Native(q) => {
                        original.replay_matches(q, c, self.settings_job_parent_limit())
                    }
                    LifecycleSubmission::Basis(q) => {
                        original.basis_replay_matches(q, c, self.settings_job_parent_limit())
                    }
                }
                .map_err(error)?
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let captured = match request {
                LifecycleSubmission::Native(q) => self.capture_scoped_lifecycle(q, c, f),
                LifecycleSubmission::Basis(q) => self.capture_basis_lifecycle(q, c, f),
            };
            observe(r, Boundary::ScopedLifecycleLowering, &captured);
            let prepared = captured
                .and_then(|capture| self.prepare_scoped_lifecycle_capture(capture, c, f, r));
            observe(r, Boundary::ScopedLifecyclePreparation, &prepared);
            let prepared = prepared.map_err(error)?;
            let original = prepared.capture.clone();
            self.install_invocation_admission(
                id,
                BoundIntent::ScopedLifecycle(original),
                Phase::ScopedLifecyclePending(prepared),
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
    pub fn run_scoped_settings_lifecycle(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let terminal = self
            .invocation_slot(id)
            .is_ok_and(|s| matches!(s.phase, Phase::Terminal(_)));
        let result = self.run_scoped_lifecycle_inner(id, c, f, r);
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
    fn run_scoped_lifecycle_inner(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        if self.faulted {
            return Err(InvocationError::Faulted);
        }
        let slot = self.invocation_slot(id)?;
        let Some(BoundIntent::ScopedLifecycle(original)) = &slot.intent else {
            return Err(InvocationError::IntentMismatch);
        };
        if original.context != *c {
            return Err(InvocationError::Unavailable);
        }
        let original_layer_capture = original.layer_capture().clone();
        match slot.phase {
            Phase::Terminal(_) => return Ok(InvocationState::Finished),
            Phase::Running => return Err(InvocationError::Faulted),
            Phase::ScopedLifecyclePending(_) => {}
            _ => return Err(InvocationError::IntentMismatch),
        }
        if self.service_requested_stop(id, r)? {
            return Ok(InvocationState::Finished);
        }
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated scoped lifecycle slot");
        let Phase::ScopedLifecyclePending(p) = std::mem::replace(&mut slot.phase, Phase::Running)
        else {
            unreachable!("scoped lifecycle phase")
        };
        self.faulted = true;
        if !matches!(&slot.intent,Some(BoundIntent::ScopedLifecycle(original))if Arc::ptr_eq(original,&p.capture)&&Arc::ptr_eq(&original.owner,&self.namespace))
        {
            observe::<()>(
                r,
                Boundary::ScopedLifecyclePublication,
                &Err(SettingsChangeErrorV2::Settings(SettingsError::Faulted)),
            );
            return Err(InvocationError::Faulted);
        }
        let decision = self.stage_scoped_lifecycle(p, c, f, r).and_then(|stage| {
            let result =
                self.stage_created_layer_results(id, &original_layer_capture, &stage, r)?;
            Ok((stage, result))
        });
        if decision.as_ref().err().is_some_and(|e| e.faulted()) {
            observe(r, Boundary::ScopedLifecyclePublication, &decision);
            return Err(InvocationError::Faulted);
        }
        let observation = decision.as_ref().map(|_| ()).map_err(|e| *e);
        let mut created_result = None;
        let outcome = match decision {
            Ok((stage, result)) => {
                created_result = result;
                let outcome = InvocationOutcome::ScopedLifecycleCommitted(ScopedSettingsUpdateV1 {
                    changed: stage.outcome.changed,
                });
                stage.install(self);
                #[cfg(test)]
                if self.settings_publication_fault {
                    panic!("injected scoped lifecycle publication interruption");
                }
                outcome
            }
            Err(e) => InvocationOutcome::ScopedLifecycleRejected(e),
        };
        self.install_invocation_terminal(id, outcome);
        self.publish_created_layer_results(id, created_result, r);
        self.faulted = false;
        observe(r, Boundary::ScopedLifecyclePublication, &observation);
        Ok(InvocationState::Finished)
    }
}
