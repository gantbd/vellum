//! Fact-bearing F11/F12 jobs in the existing ledger and native publishers.
use super::*;
use source_context::{ConsumerCapture, ConsumerPlan};
pub(super) mod replacement;
#[cfg(test)]
mod tests;
#[derive(Debug)]
pub(super) struct Pending {
    pub capture: Arc<ConsumerCapture>,
    pub plan: ConsumerPlan,
    reserved: Vec<SourceChange>,
}
fn error(e: SourceReadContextErrorV1) -> InvocationError {
    match e {
        SourceReadContextErrorV1::Faulted => InvocationError::Faulted,
        SourceReadContextErrorV1::Limit => InvocationError::Limit,
        SourceReadContextErrorV1::Capacity => InvocationError::Capacity,
        _ => InvocationError::Unavailable,
    }
}
impl SourceCoordinator {
    #[allow(clippy::too_many_arguments)]
    pub fn submit_source_search_consumer(
        &mut self,
        id: &InvocationId,
        command: &SourceSearchConsumerV1,
        s: &SourceReadScopeV1,
        sf: &TrustedSourceReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        cf: &TrustedSourceSearchConsumerFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_managed_search_consumer(id, command, s, SearchReadFactsV1::Source(sf), f, cf, r)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn submit_managed_search_consumer(
        &mut self,
        id: &InvocationId,
        command: &SourceSearchConsumerV1,
        s: &SourceReadScopeV1,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        cf: &TrustedSourceSearchConsumerFactsV1<'_>,
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
                let BoundIntent::ManagedSearch(c) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                if c.command != *command
                    || c.scope != *s
                    || c.request != *f.request
                    || sf.scope() != s
                    || !c.read_matches(sf)
                    || cf.scope != s
                    || cf.command != command
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let (capture, plan) = self
                .capture_source_search_consumer(
                    command,
                    s,
                    sf,
                    f,
                    cf,
                    self.invocations.limits.max_intent_bytes,
                    r,
                )
                .map_err(error)?;
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .expect("validated issued consumer");
            let Phase::Issued(reserved) = std::mem::replace(&mut slot.phase, Phase::Running) else {
                return Err(InvocationError::Faulted);
            };
            self.install_invocation_admission(
                id,
                BoundIntent::ManagedSearch(capture.clone()),
                Phase::ManagedSearchPending(Pending {
                    capture,
                    plan,
                    reserved,
                }),
            );
            Ok(InvocationState::Pending)
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture(r, Boundary::SourceSearchConsumerPreparation, &result);
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
    #[allow(clippy::too_many_arguments)]
    pub fn run_source_search_consumer(
        &mut self,
        id: &InvocationId,
        s: &SourceReadScopeV1,
        sf: &TrustedSourceReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        cf: &TrustedSourceSearchConsumerFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.run_managed_search_consumer(id, s, SearchReadFactsV1::Source(sf), f, cf, r)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn run_managed_search_consumer(
        &mut self,
        id: &InvocationId,
        s: &SourceReadScopeV1,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        cf: &TrustedSourceSearchConsumerFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let terminal = self
            .invocation_slot(id)
            .is_ok_and(|slot| matches!(slot.phase, Phase::Terminal(_)));
        let result = self.run_source_search_consumer_inner(id, s, sf, f, cf, r);
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        if result.is_err() {
            capture(r, Boundary::SourceSearchConsumerPublication, &result);
        }
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
    #[allow(clippy::too_many_arguments)]
    fn run_source_search_consumer_inner(
        &mut self,
        id: &InvocationId,
        s: &SourceReadScopeV1,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        cf: &TrustedSourceSearchConsumerFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        if self.faulted {
            return Err(InvocationError::Faulted);
        }
        let slot = self.invocation_slot(id)?;
        let original = match &slot.intent {
            Some(BoundIntent::ManagedSearch(c)) => c,
            Some(BoundIntent::ReturnedSearchReplacement(c)) => {
                if !c.intact(&self.namespace, id, &self.retention) {
                    self.faulted = true;
                    return Err(InvocationError::Faulted);
                }
                &c.consumer
            }
            _ => return Err(InvocationError::IntentMismatch),
        };
        if original.scope != *s
            || original.request != *f.request
            || sf.scope() != s
            || !original.read_matches(sf)
            || cf.scope != s
            || cf.command != &original.command
        {
            return Err(InvocationError::Unavailable);
        }
        let original = original.clone();
        match &slot.phase {
            Phase::Terminal(_) => return Ok(InvocationState::Finished),
            Phase::Running => return Err(InvocationError::Faulted),
            Phase::ManagedSearchPending(p)
                if Arc::ptr_eq(&original, &p.capture) && original.intact(&p.plan) => {}
            _ => {
                self.invocations.slots[id.slot]
                    .as_mut()
                    .expect("owned corrupt consumer")
                    .phase = Phase::Running;
                self.faulted = true;
                return Err(InvocationError::Faulted);
            }
        }
        if self.service_requested_stop(id, r)? {
            return Ok(InvocationState::Finished);
        }
        let eligible = self
            .validate_source_search_consumer(&original, s, sf, f, r)
            .and_then(|()| {
                if cf.mutation_allowed == Some(true) && cf.resources_ready == Some(true) {
                    Ok(())
                } else {
                    Err(SourceReadContextErrorV1::Unavailable)
                }
            });
        let checked = eligible.as_ref().map(|_| ()).map_err(|e| error(*e));
        capture(r, Boundary::SourceSearchConsumerValidation, &checked);
        if matches!(eligible, Err(SourceReadContextErrorV1::Faulted)) {
            self.faulted = true;
            return Err(InvocationError::Faulted);
        }
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated managed consumer");
        let Phase::ManagedSearchPending(p) = std::mem::replace(&mut slot.phase, Phase::Running)
        else {
            unreachable!("validated phase")
        };
        self.faulted = true;
        let outcome = match p.plan {
            ConsumerPlan::Source(prepared) => {
                let decision = if eligible.is_ok() {
                    prepared.and_then(|plan| self.commit_with_storage(plan, p.reserved, r))
                } else {
                    Err(CoordinatorError::Unavailable)
                };
                if decision.as_ref().err().is_some_and(|e| {
                    matches!(
                        e,
                        CoordinatorError::Faulted
                            | CoordinatorError::History(HistoryError::Faulted)
                    )
                }) {
                    return Err(InvocationError::Faulted);
                }
                match decision {
                    Ok(value) => InvocationOutcome::Committed(value),
                    Err(e) => InvocationOutcome::Rejected(e),
                }
            }
            ConsumerPlan::Selection { prepared, .. } => {
                let decision = if eligible.is_ok() {
                    prepared.and_then(|p| self.publish_selection_command(p, r))
                } else {
                    Err(crate::selection::SelectionError::Unavailable)
                };
                if matches!(decision, Err(crate::selection::SelectionError::Faulted)) {
                    return Err(InvocationError::Faulted);
                }
                match decision {
                    Ok(value) => InvocationOutcome::SelectionCommitted(value),
                    Err(e) => InvocationOutcome::SelectionRejected(e),
                }
            }
        };
        let (disposition, published, annotations) = match &outcome {
            InvocationOutcome::Committed(v) => (
                Disposition::Completed,
                v.operation.is_some(),
                !v.annotation_changes.is_empty(),
            ),
            InvocationOutcome::SelectionCommitted(v) => (Disposition::Completed, v.changed, false),
            _ => (Disposition::Refused, false, false),
        };
        self.install_invocation_terminal(id, outcome);
        self.faulted = false;
        r.record(Observation {
            boundary: Boundary::Admission,
            disposition,
            service_time: None,
        });
        r.record(Observation {
            boundary: Boundary::SourceSearchConsumerPublication,
            disposition,
            service_time: None,
        });
        if published {
            r.record(Observation {
                boundary: Boundary::Publication,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        if annotations {
            r.record(Observation {
                boundary: Boundary::AnnotationPublication,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        Ok(InvocationState::Finished)
    }
}
