//! Fact-bearing service over the common invocation/root/outcome boundary.
use super::super::serialization::observe;
use super::*;
#[cfg(test)]
mod tests;
fn error(e: SourceSerializationErrorV1) -> InvocationError {
    match e {
        SourceSerializationErrorV1::Faulted => InvocationError::Faulted,
        e if e.faulted() => InvocationError::Faulted,
        SourceSerializationErrorV1::Limit
        | SourceSerializationErrorV1::Dependency(CoordinatorError::ReadDependency(
            ReadDependencyError::Limit,
        )) => InvocationError::Limit,
        SourceSerializationErrorV1::Capacity
        | SourceSerializationErrorV1::Dependency(CoordinatorError::ReadDependency(
            ReadDependencyError::Capacity,
        )) => InvocationError::Capacity,
        SourceSerializationErrorV1::Unavailable
        | SourceSerializationErrorV1::Stale
        | SourceSerializationErrorV1::VersionExhausted => InvocationError::Unavailable,
        _ => InvocationError::MalformedIntent,
    }
}
impl SourceCoordinator {
    pub fn submit_scoped_source_serialization(
        &mut self,
        id: &InvocationId,
        request: &ScopedSourceSerializationRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationBasisFactsV1<'_>,
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
                let BoundIntent::Serialization(original) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                if !original.scoped_replay_matches(request, c).map_err(error)? {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let prepared = self.prepare_scoped_serialization_inner(request, c, f, r);
            observe(r, Boundary::ScopedSerializationPreparation, &prepared);
            let prepared = prepared.map_err(error)?;
            let original = prepared.capture.clone();
            self.install_invocation_admission(
                id,
                BoundIntent::Serialization(original),
                Phase::SerializationPending(prepared),
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
    pub fn submit_source_serialization(
        &mut self,
        id: &InvocationId,
        request: &SourceSerializationRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        let repeated = self
            .invocation_slot(id)
            .is_ok_and(|slot| slot.intent.is_some());
        let result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(id)?;
            if let Some(original) = &slot.intent {
                let BoundIntent::Serialization(original) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                if !original.replay_matches(request, c).map_err(error)? {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let prepared = self
                .capture_source_serialization(request, c, f)
                .and_then(|capture| self.prepare_serialization_capture(capture, r));
            observe(r, Boundary::SerializationPreparation, &prepared);
            let prepared = prepared.map_err(error)?;
            let original = prepared.capture.clone();
            self.install_invocation_admission(
                id,
                BoundIntent::Serialization(original),
                Phase::SerializationPending(prepared),
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
    pub fn run_source_serialization(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let terminal = self
            .invocation_slot(id)
            .is_ok_and(|slot| matches!(slot.phase, Phase::Terminal(_)));
        let result = self.run_serialization_inner(id, c, f, None, r);
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
    pub fn run_scoped_source_serialization(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationBasisFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let terminal = self
            .invocation_slot(id)
            .is_ok_and(|s| matches!(s.phase, Phase::Terminal(_)));
        let result = self.run_serialization_inner(id, c, f.native, Some(f), r);
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
    fn run_serialization_inner(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationFactsV1<'_>,
        scoped: Option<&TrustedSerializationBasisFactsV1<'_>>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        if self.faulted {
            return Err(InvocationError::Faulted);
        }
        let slot = self.invocation_slot(id)?;
        let Some(BoundIntent::Serialization(original)) = &slot.intent else {
            return Err(InvocationError::IntentMismatch);
        };
        if original.context != *c {
            return Err(InvocationError::Unavailable);
        }
        if original.basis.is_some() != scoped.is_some() {
            return Err(InvocationError::IntentMismatch);
        }
        match slot.phase {
            Phase::Terminal(_) => return Ok(InvocationState::Finished),
            Phase::Running => return Err(InvocationError::Faulted),
            Phase::SerializationPending(_) => {}
            _ => return Err(InvocationError::IntentMismatch),
        }
        if self.service_requested_stop(id, r)? {
            return Ok(InvocationState::Finished);
        }
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated serialization slot");
        let Phase::SerializationPending(p) = std::mem::replace(&mut slot.phase, Phase::Running)
        else {
            unreachable!("serialization phase")
        };
        self.faulted = true;
        let publication = if scoped.is_some() {
            Boundary::ScopedSerializationPublication
        } else {
            Boundary::SerializationPublication
        };
        if !matches!(&slot.intent, Some(BoundIntent::Serialization(original)) if Arc::ptr_eq(original, &p.capture) && Arc::ptr_eq(&original.owner, &self.namespace))
        {
            observe::<()>(r, publication, &Err(SourceSerializationErrorV1::Faulted));
            return Err(InvocationError::Faulted);
        }
        let decision = if let Some(scoped) = scoped {
            self.stage_scoped_source_serialization(p, c, scoped, r)
        } else {
            self.stage_source_serialization(p, c, f, r)
        };
        if decision.as_ref().err().is_some_and(|e| e.faulted()) {
            observe(r, publication, &decision);
            return Err(InvocationError::Faulted);
        }
        let observation = decision.as_ref().map(|_| ()).map_err(|e| *e);
        let outcome = match decision {
            Ok(stage) => {
                let outcome = InvocationOutcome::SerializationCommitted(stage.outcome);
                stage.install(self);
                #[cfg(test)]
                if self.settings_publication_fault {
                    panic!("injected serialization publication interruption");
                }
                outcome
            }
            Err(e) => InvocationOutcome::SerializationRejected(e),
        };
        self.install_invocation_terminal(id, outcome);
        self.faulted = false;
        observe(r, publication, &observation);
        Ok(InvocationState::Finished)
    }
}
