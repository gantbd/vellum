//! The original evidence ID is part of intent, never resolved on matching retry.
use super::*;
mod arguments;
pub use arguments::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnicodeOperationV1 {
    Movement,
    Deletion,
}
#[derive(PartialEq, Eq)]
pub(super) struct UnicodeIntent {
    pub(super) context: LogicalViewContext,
    pub(super) evidence: BoundaryEvidenceId,
    pub(super) operation: UnicodeOperationV1,
    pub(super) request: UnicodeCommandV1,
    pub(super) budget: Option<UnicodeCommandBudget>,
}
impl std::fmt::Debug for UnicodeIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("UnicodeIntent(<redacted>)")
    }
}
impl SourceCoordinator {
    /// Bind once before consulting current source/evidence. Results are trusted-
    /// owner retained facts, not full selection snapshots or recipient release.
    pub fn submit_unicode_command(
        &mut self,
        id: &InvocationId,
        context: &LogicalViewContext,
        evidence: &BoundaryEvidenceId,
        operation: UnicodeOperationV1,
        request: UnicodeCommandV1,
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
            if std::mem::size_of::<UnicodeIntent>() > self.invocations.limits.max_intent_bytes {
                return Err(InvocationError::Limit);
            }
            if let Some(original) = &slot.intent {
                if !matches!(original,BoundIntent::Unicode(intent) if intent.context==*context && intent.evidence==*evidence && intent.operation==operation && intent.request==request)
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let intent = UnicodeIntent {
                context: context.clone(),
                evidence: evidence.clone(),
                operation,
                request,
                budget: self.unicode_command_profile(),
            };
            match operation {
                UnicodeOperationV1::Movement => {
                    let prepared = self.prepare_unicode_movement(context, evidence, request, r);
                    let slot = self.invocations.slots[id.slot]
                        .as_mut()
                        .expect("validated issued slot");
                    let Phase::Issued(_) = std::mem::replace(&mut slot.phase, Phase::Running)
                    else {
                        unreachable!("unbound slot is issued")
                    };
                    self.install_invocation_admission(
                        id,
                        BoundIntent::Unicode(intent),
                        Phase::UnicodeMovementPending(prepared),
                    );
                }
                UnicodeOperationV1::Deletion => {
                    let prepared = self
                        .prepare_unicode_deletion(context, evidence, request, r)
                        .map_err(NativePreparationError::Unicode);
                    let slot = self.invocations.slots[id.slot]
                        .as_mut()
                        .expect("validated issued slot");
                    let Phase::Issued(reserved) =
                        std::mem::replace(&mut slot.phase, Phase::Running)
                    else {
                        unreachable!("unbound slot is issued")
                    };
                    self.install_invocation_admission(
                        id,
                        BoundIntent::Unicode(intent),
                        Phase::Pending { prepared, reserved },
                    );
                }
            }
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
    pub(super) fn run_unicode_movement(
        &mut self,
        id: &InvocationId,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated pending slot");
        let Phase::UnicodeMovementPending(prepared) =
            std::mem::replace(&mut slot.phase, Phase::Running)
        else {
            unreachable!("validated motion job")
        };
        let intact = match (&slot.intent, &prepared) {
            (Some(BoundIntent::Unicode(intent)), Ok(plan))
                if intent.operation == UnicodeOperationV1::Movement =>
            {
                plan.matches(
                    &intent.context,
                    &intent.evidence,
                    intent.request,
                    intent.budget,
                )
            }
            (Some(BoundIntent::Unicode(intent)), Err(_))
                if intent.operation == UnicodeOperationV1::Movement =>
            {
                true
            }
            _ => false,
        };
        self.faulted = true;
        let result = if intact {
            prepared.and_then(|plan| self.publish_unicode_movement(plan, r))
        } else {
            Err(UnicodeCommandError::Faulted)
        };
        if result
            .as_ref()
            .err()
            .is_some_and(UnicodeCommandError::is_faulted)
        {
            r.record(Observation {
                boundary: Boundary::UnicodePublication,
                disposition: Disposition::Unknown,
                service_time: None,
            });
            return Err(InvocationError::Faulted);
        }
        let changed = result.as_ref().is_ok_and(|update| update.changed);
        let disposition = if result.is_ok() {
            Disposition::Completed
        } else {
            Disposition::Refused
        };
        let outcome = match result {
            Ok(update) => InvocationOutcome::UnicodeMovementCommitted(update),
            Err(error) => InvocationOutcome::UnicodeRejected(error),
        };
        self.install_invocation_terminal(id, outcome);
        self.faulted = false;
        r.record(Observation {
            boundary: Boundary::UnicodePublication,
            disposition,
            service_time: None,
        });
        if changed {
            r.record(Observation {
                boundary: Boundary::Publication,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        Ok(InvocationState::Finished)
    }
}
