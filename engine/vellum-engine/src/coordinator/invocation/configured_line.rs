//! Configured line jobs retain a charged interpretation until protected retirement.
use super::super::configured_line::{self, Witness};
use super::*;
mod arguments;
pub use arguments::*;
#[cfg(test)]
mod tests;
#[derive(Debug)]
pub(super) struct ConfiguredLineIntent {
    context: LogicalViewContext,
    request: ConfiguredLineEditV1,
    profile: ConfiguredLineLimitsV1,
    line_budget: Option<LineEditBudget>,
    captured: Option<(Witness, LineEditV1)>,
    lease: Option<SettingsInterpretationId>,
    refusal: Option<ConfiguredLineErrorV1>,
}
impl ConfiguredLineIntent {
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.context.source.document == *document
    }
}
impl SourceCoordinator {
    pub fn submit_configured_line_edit(
        &mut self,
        id: &InvocationId,
        context: &LogicalViewContext,
        request: &ConfiguredLineEditV1,
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
                let BoundIntent::ConfiguredLine(original) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                let owned = request.copy(original.profile)?;
                if original.context != *context || original.request != owned {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let profile = self
                .configured_line_limits
                .ok_or(InvocationError::Unavailable)?;
            let bytes = std::mem::size_of::<ConfiguredLineIntent>()
                .checked_add(request.payload_bytes().ok_or(InvocationError::Limit)?)
                .ok_or(InvocationError::Limit)?;
            let remaining = self
                .invocations
                .limits
                .max_intent_bytes
                .checked_sub(bytes)
                .ok_or(InvocationError::Limit)?;
            let owned = request.copy(profile)?;
            let plan_request = request.copy(profile)?;
            let budget = self.line_edit_profile();
            let decision =
                self.prepare_configured_line(context, plan_request, profile, remaining, r);
            let (prepared, captured, lease, refusal) = match decision {
                Ok((plan, witness, explicit)) => {
                    let lease = witness.lease.clone();
                    (Ok(plan), Some((witness, explicit)), Some(lease), None)
                }
                Err(error) => (Err(error), None, None, Some(error)),
            };
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .expect("validated issued slot");
            let Phase::Issued(reserved) = std::mem::replace(&mut slot.phase, Phase::Running) else {
                unreachable!("unbound issued slot")
            };
            self.install_invocation_admission(
                id,
                BoundIntent::ConfiguredLine(ConfiguredLineIntent {
                    context: context.clone(),
                    request: owned,
                    profile,
                    line_budget: budget,
                    captured,
                    lease,
                    refusal,
                }),
                Phase::ConfiguredLinePending { prepared, reserved },
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
    pub(super) fn run_configured_line(
        &mut self,
        id: &InvocationId,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated configured line slot");
        let Phase::ConfiguredLinePending { prepared, reserved } =
            std::mem::replace(&mut slot.phase, Phase::Running)
        else {
            unreachable!("configured line phase")
        };
        self.faulted = true;
        let intact = match (&slot.intent, &prepared) {
            (Some(BoundIntent::ConfiguredLine(original)), Ok(plan)) => {
                match (&original.captured, &plan.configured_line, &original.lease) {
                    (Some((witness, explicit)), Some(captured), Some(lease)) => {
                        original.refusal.is_none()
                            && plan.configured_required
                            && captured.request == original.request
                            && captured.witness == *witness
                            && witness.lease == *lease
                            && captured.limits == original.profile
                            && captured.line_budget == original.line_budget
                            && Self::line_intent_matches(
                                plan,
                                &original.context,
                                explicit,
                                original.line_budget,
                            )
                    }
                    _ => false,
                }
            }
            (Some(BoundIntent::ConfiguredLine(original)), Err(error)) => {
                original.refusal == Some(*error)
                    && original.captured.is_none()
                    && original.lease.is_none()
            }
            _ => false,
        };
        if !intact {
            configured_line::observe::<()>(
                r,
                Boundary::ConfiguredLineValidation,
                &Err(ConfiguredLineErrorV1::Coordinator(
                    CoordinatorError::Faulted,
                )),
            );
            return Err(InvocationError::Faulted);
        }
        let decision = prepared.and_then(|plan| {
            self.commit_with_storage(plan, reserved, r)
                .map_err(ConfiguredLineErrorV1::Coordinator)
        });
        if decision.as_ref().err().is_some_and(|error| error.faulted()) {
            configured_line::observe(r, Boundary::ConfiguredLineValidation, &decision);
            return Err(InvocationError::Faulted);
        }
        let disposition = if decision.is_ok() {
            Disposition::Completed
        } else {
            Disposition::Refused
        };
        let published = decision
            .as_ref()
            .is_ok_and(|outcome| outcome.operation.is_some());
        let annotations = decision
            .as_ref()
            .is_ok_and(|outcome| !outcome.annotation_changes.is_empty());
        let outcome = match decision {
            Ok(outcome) => InvocationOutcome::Committed(outcome),
            Err(error) => InvocationOutcome::ConfiguredLineRejected(error),
        };
        self.install_invocation_terminal(id, outcome);
        self.faulted = false;
        r.record(Observation {
            boundary: Boundary::Admission,
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
    pub(super) fn release_configured_line_lease(
        &mut self,
        id: &InvocationId,
        r: &mut Recorder,
    ) -> Result<bool, InvocationError> {
        let slot = self.invocation_slot(id)?;
        let Some(BoundIntent::ConfiguredLine(intent)) = &slot.intent else {
            return Ok(false);
        };
        let Some(lease) = intent.lease.clone() else {
            return Ok(false);
        };
        self.release_settings_interpretation(&lease, r)
            .map_err(|_| {
                self.faulted = true;
                InvocationError::Faulted
            })?;
        let Some(BoundIntent::ConfiguredLine(intent)) = &mut self.invocations.slots[id.slot]
            .as_mut()
            .expect("retained slot")
            .intent
        else {
            unreachable!("same owned intent")
        };
        intent.lease = None;
        Ok(true)
    }
    /// Protected discharge of a terminal or uncertain local job. Does not reopen
    /// execution, clear a fault, forget an outcome or refund lifetime issuance.
    pub fn release_configured_line_retention(
        &mut self,
        id: &InvocationId,
        r: &mut Recorder,
    ) -> Result<bool, InvocationError> {
        let result = (|| {
            let slot = self.invocation_slot(id)?;
            if !matches!(slot.intent, Some(BoundIntent::ConfiguredLine(_))) {
                return Err(InvocationError::Unavailable);
            }
            if !matches!(slot.phase, Phase::Terminal(_) | Phase::Running) {
                return Err(InvocationError::Pending);
            }
            self.release_configured_line_lease(id, r)
        })();
        capture(r, Boundary::ConfiguredLineRetention, &result);
        result
    }
}
