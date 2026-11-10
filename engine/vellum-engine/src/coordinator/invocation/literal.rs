//! Trusted-owner literal editing admission into the existing invocation ledger.
use super::*;
pub(in crate::coordinator) mod arguments;
pub use arguments::*;

#[derive(PartialEq, Eq)]
pub(super) struct LiteralIntent {
    pub(super) context: LogicalViewContext,
    pub(super) text: String,
    pub(super) after: SelectionAfterV1,
}
enum LiteralText<'a> {
    Borrowed(&'a str),
    Owned(String),
}
impl LiteralText<'_> {
    fn text(&self) -> &str {
        match self {
            Self::Borrowed(text) => text,
            Self::Owned(text) => text,
        }
    }
    fn into_owned(self) -> Result<String, InvocationError> {
        match self {
            Self::Owned(text) => Ok(text),
            Self::Borrowed(text) => {
                let mut owned = String::new();
                owned
                    .try_reserve_exact(text.len())
                    .map_err(|_| InvocationError::Capacity)?;
                owned.push_str(text);
                Ok(owned)
            }
        }
    }
}
impl std::fmt::Debug for LiteralIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LiteralIntent(<redacted>)")
    }
}
impl SourceCoordinator {
    /// Descriptor-width preflight only: no reservation, authority, currentness
    /// or execution guarantee. The invocation profile remains caller-selected.
    pub fn can_retain_literal_intent_v1(&self, text_bytes: usize) -> bool {
        std::mem::size_of::<LiteralIntent>()
            .checked_add(text_bytes)
            .is_some_and(|bytes| bytes <= self.invocations.limits.max_intent_bytes)
    }
    #[cfg(test)]
    pub(in crate::coordinator) fn corrupt_literal_invocation_for_test(
        &mut self,
        id: &InvocationId,
        remove: bool,
    ) {
        let Phase::Pending {
            prepared: Ok(prepared),
            ..
        } = &mut self.invocations.slots[id.slot].as_mut().unwrap().phase
        else {
            panic!("not prepared")
        };
        Self::corrupt_literal_plan_for_test(prepared, remove);
    }
    /// Bind a literal request once, retaining its original preparation or refusal.
    /// Retries compare semantic intent before any current-source/view acquisition.
    /// Outcomes use the existing trusted-owner retained-outcome facet; this API
    /// neither returns an authorized recipient projection nor interprets UI input.
    pub fn submit_literal_edit(
        &mut self,
        id: &InvocationId,
        context: &LogicalViewContext,
        text: &str,
        after: SelectionAfterV1,
        recorder: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.submit_literal_edit_with_permission_v1(id, context, text, after, Some(true), recorder)
    }
    /// Trusted embedding admission premise, not a credential evaluator. Missing
    /// or denied permission retains a known refusal for this exact intent; run
    /// terminalizes it without source publication. Replay never refreshes the
    /// premise or reacquires a newer target. This does not replace current native
    /// source/view/policy/companion checks at publication. Integrations combining
    /// current external gates with service must submit/run in one exclusive call.
    pub fn submit_literal_edit_with_permission_v1(
        &mut self,
        id: &InvocationId,
        context: &LogicalViewContext,
        text: &str,
        after: SelectionAfterV1,
        allowed: Option<bool>,
        recorder: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.submit_literal_input(
            id,
            context,
            LiteralText::Borrowed(text),
            after,
            allowed,
            recorder,
        )
    }
    /// Same exact intent/replay/publisher, consuming caller-reserved command
    /// text without a fresh descriptor-text allocation. Candidate preparation
    /// retains its own native limits and known refusal behavior. No grants are
    /// inferred from owning the String; this remains a trusted-owner seam.
    pub fn submit_owned_literal_edit_with_permission_v1(
        &mut self,
        id: &InvocationId,
        context: &LogicalViewContext,
        text: String,
        after: SelectionAfterV1,
        allowed: Option<bool>,
        recorder: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.submit_literal_input(
            id,
            context,
            LiteralText::Owned(text),
            after,
            allowed,
            recorder,
        )
    }
    fn submit_literal_input(
        &mut self,
        id: &InvocationId,
        context: &LogicalViewContext,
        text: LiteralText<'_>,
        after: SelectionAfterV1,
        allowed: Option<bool>,
        recorder: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, recorder)?;
        let repeated = self
            .invocation_slot(id)
            .is_ok_and(|slot| slot.intent.is_some());
        let result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(id)?;
            // Logical request descriptor storage, not allocator overhead or the
            // separately bounded source/companion preparation's transient peak.
            let bytes = std::mem::size_of::<LiteralIntent>()
                .checked_add(text.text().len())
                .ok_or(InvocationError::Limit)?;
            if bytes > self.invocations.limits.max_intent_bytes {
                return Err(InvocationError::Limit);
            }
            if let Some(original) = &slot.intent {
                if !matches!(original, BoundIntent::LiteralEdit(intent)
                    if intent.context == *context && intent.text == text.text() && intent.after == after)
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let owned = text.into_owned()?;
            let prepared = if allowed == Some(true) {
                self.prepare_literal_edit(context, &owned, after, recorder)
                    .map_err(views::owner_error)
            } else {
                Err(CoordinatorError::Unavailable)
            };
            let intent = LiteralIntent {
                context: context.clone(),
                text: owned,
                after,
            };
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .expect("validated issued slot");
            let Phase::Issued(reserved) = std::mem::replace(&mut slot.phase, Phase::Running) else {
                unreachable!("unbound slot is issued")
            };
            self.install_invocation_admission(
                id,
                BoundIntent::LiteralEdit(intent),
                Phase::Pending {
                    prepared: prepared.map_err(NativePreparationError::Coordinator),
                    reserved,
                },
            );
            Ok(InvocationState::Pending)
        })();
        capture(
            recorder,
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
