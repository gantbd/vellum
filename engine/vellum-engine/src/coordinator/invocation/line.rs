//! Explicit line requests reuse native source execution and retained outcomes.
use super::*;
mod arguments;
pub use arguments::*;
#[derive(PartialEq, Eq)]
pub(super) struct LineIntent {
    pub(super) context: LogicalViewContext,
    pub(super) request: LineEditV1,
    pub(super) budget: Option<LineEditBudget>,
}
impl std::fmt::Debug for LineIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LineIntent(<redacted>)")
    }
}
impl SourceCoordinator {
    /// Retain the original line request, work profile and native preparation or
    /// refusal. Matching retries never reacquire current source, selections or
    /// settings. Outcomes use the existing trusted-owner retained facet.
    pub fn submit_line_edit(
        &mut self,
        id: &InvocationId,
        context: &LogicalViewContext,
        request: &LineEditV1,
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
            let bytes = std::mem::size_of::<LineIntent>()
                .checked_add(request.unit_bytes())
                .ok_or(InvocationError::Limit)?;
            if bytes > self.invocations.limits.max_intent_bytes {
                return Err(InvocationError::Limit);
            }
            if let Some(original) = &slot.intent {
                if !matches!(original, BoundIntent::LineEdit(intent) if intent.context==*context && intent.request==*request)
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let owned = request.copy().map_err(|_| InvocationError::Capacity)?;
            let budget = self.line_edit_profile();
            let prepared = self
                .prepare_line_edit(context, request, r)
                .map_err(NativePreparationError::Line);
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .expect("validated issued slot");
            let Phase::Issued(reserved) = std::mem::replace(&mut slot.phase, Phase::Running) else {
                unreachable!("unbound slot is issued")
            };
            self.install_invocation_admission(
                id,
                BoundIntent::LineEdit(LineIntent {
                    context: context.clone(),
                    request: owned,
                    budget,
                }),
                Phase::Pending { prepared, reserved },
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
}
