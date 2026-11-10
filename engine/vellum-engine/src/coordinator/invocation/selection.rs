//! Selection-only jobs share identities, retention and the one-shot runner.
use super::*;
use crate::selection::SelectionError;
mod arguments;
pub use arguments::*;
impl SourceCoordinator {
    /// Retain a set or endpoint preparation in the shared one-shot ledger.
    /// Retry comparison preserves the raw list/primary and explicit scan budget;
    /// it never reacquires newer selections. Results are available only through
    /// the trusted-owner retained outcome facet, not recipient-authorized release.
    pub fn submit_selection_command(
        &mut self,
        id: &InvocationId,
        context: &LogicalViewContext,
        command: &SelectionCommandV1,
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
            let bytes = command
                .retained_bytes()
                .and_then(|n| n.checked_add(std::mem::size_of::<LogicalViewContext>()))
                .ok_or(InvocationError::Limit)?;
            if bytes > self.invocations.limits.max_intent_bytes {
                return Err(InvocationError::Limit);
            }
            if let Some(original) = &slot.intent {
                if !matches!(original, BoundIntent::Selection { context: old, command: old_command } if old == context && old_command == command)
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let owned = command.copy().map_err(|_| InvocationError::Capacity)?;
            let prepared = self.prepare_selection_command(context, command, r);
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .expect("validated issued slot");
            debug_assert!(matches!(slot.phase, Phase::Issued(_)));
            self.install_invocation_admission(
                id,
                BoundIntent::Selection {
                    context: context.clone(),
                    command: owned,
                },
                Phase::SelectionPending(prepared),
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
    pub(super) fn run_selection_command(
        &mut self,
        id: &InvocationId,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated pending slot");
        let Phase::SelectionPending(prepared) = std::mem::replace(&mut slot.phase, Phase::Running)
        else {
            unreachable!("validated selection job")
        };
        let intact = match (&slot.intent, &prepared) {
            (Some(BoundIntent::Selection { context, command }), Ok(prepared)) => {
                prepared.matches(context, command)
            }
            (Some(BoundIntent::Selection { .. }), Err(_)) => true,
            _ => false,
        };
        self.faulted = true;
        let result = if intact {
            prepared.and_then(|prepared| self.publish_selection_command(prepared, r))
        } else {
            Err(SelectionError::Faulted)
        };
        if result == Err(SelectionError::Faulted) {
            r.record(Observation {
                boundary: Boundary::LogicalSelectionPublication,
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
            Ok(update) => InvocationOutcome::SelectionCommitted(update),
            Err(error) => InvocationOutcome::SelectionRejected(error),
        };
        self.install_invocation_terminal(id, outcome);
        self.faulted = false;
        r.record(Observation {
            boundary: Boundary::LogicalSelectionPublication,
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
