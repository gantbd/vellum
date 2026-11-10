//! Native F14 historical decisions. Host-attested publication scope/epoch is a
//! release premise, not an implemented F06 observation owner or a wire protocol.
use super::*;
use std::mem::size_of;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy)]
pub struct TrustedReversalResultGrantV1<'a> {
    pub invocation: &'a InvocationId,
    pub authority: &'a str,
    pub target: &'a HistoryReversalTargetV1,
    pub disclose_target: bool,
}
pub struct TrustedReversalResultAdmissionV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    /// Host attests the actual original publication policy epoch. This native
    /// port neither creates that F06 policy owner nor authenticates caller labels.
    pub publication_epoch: u64,
    pub publication_allowed: Option<bool>,
    pub context_current: Option<bool>,
    pub result_admission_allowed: Option<bool>,
    pub max_result_bytes: usize,
    pub target_grant: Option<TrustedReversalResultGrantV1<'a>>,
}
pub struct TrustedReversalResultReleaseV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    /// Actual current host policy epoch, not recipient-supplied release authority.
    pub publication_epoch: u64,
    pub publication_allowed: Option<bool>,
    pub outcome_readable: Option<bool>,
    pub result_context_current: Option<bool>,
    pub reply_ready: Option<bool>,
    pub resources_ready: Option<bool>,
    pub max_result_bytes: usize,
    pub target_grant: Option<TrustedReversalResultGrantV1<'a>>,
}
#[derive(PartialEq, Eq)]
pub enum ReversalDecisionV1 {
    Applied {
        publication: InvocationId,
        target: Option<HistoryReversalTargetV1>,
    },
    NotPublished,
}
#[derive(PartialEq, Eq)]
pub struct ReversalResultV1 {
    pub invocation: InvocationId,
    pub operation: HistoryDirection,
    pub decision: ReversalDecisionV1,
}
macro_rules! redacted { ($($t:ty),+) => { $(impl std::fmt::Debug for $t { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("<reversal-result:redacted>") } })+ }; }
redacted!(ReversalDecisionV1, ReversalResultV1);
impl ReversalResultV1 {
    /// Maximum native inline result, including optional opaque target handles;
    /// not JSON bytes, physical heap usage or a disclosure grant.
    pub const fn required_bytes() -> usize {
        size_of::<Self>()
    }
}
pub(in crate::coordinator) struct Capture {
    invocation: InvocationId,
    target: HistoryReversalTargetV1,
    direction: HistoryDirection,
    context: SettingsExecutionContextV1,
    epoch: u64,
    publication: bool,
    target_ceiling: bool,
    maximum: usize,
    intent_bytes: usize,
}
#[derive(Clone, PartialEq, Eq)]
enum Decision {
    Applied(InvocationId),
    NotPublished,
}
#[derive(Clone, PartialEq, Eq)]
pub(in crate::coordinator) struct Receipt {
    invocation: InvocationId,
    operation: HistoryDirection,
    decision: Decision,
}
pub(in crate::coordinator::invocation) struct State {
    original: Arc<Capture>,
    pub(in crate::coordinator::invocation) receipt: Option<Receipt>,
}
redacted!(Capture, Receipt, State);
pub(in crate::coordinator::invocation) fn direct_bytes() -> usize {
    size_of::<HistoryGroupId>() + size_of::<HistoryDirection>() + size_of::<Option<Box<State>>>()
}
fn labels(c: &SettingsExecutionContextV1) -> [&str; 7] {
    [
        &c.authority,
        &c.engine,
        &c.recipient,
        &c.purpose,
        &c.destination,
        &c.profile,
        &c.continuity,
    ]
}
fn extra_bytes(c: &SettingsExecutionContextV1) -> Result<usize, InvocationError> {
    labels(c).into_iter().try_fold(
        size_of::<Capture>() + size_of::<State>() + size_of::<ReversalResultV1>(),
        |n, v| n.checked_add(v.len()).ok_or(InvocationError::Limit),
    )
}
fn read_bytes(reads: &ReadDependencyRequestV3) -> Result<usize, InvocationError> {
    read_dependency::ReadInput::V3(reads)
        .retained_bytes(&[])
        .and_then(|n| {
            n.checked_add(
                size_of::<ReversalReadIntent>() - size_of::<read_dependency::ReadCapture>(),
            )
        })
        .ok_or(InvocationError::Limit)
}
impl TrustedReversalResultAdmissionV1<'_> {
    pub fn required_direct_intent_bytes(
        c: &SettingsExecutionContextV1,
    ) -> Result<usize, InvocationError> {
        direct_bytes()
            .checked_add(extra_bytes(c)?)
            .ok_or(InvocationError::Limit)
    }
    pub fn required_read_intent_bytes(
        c: &SettingsExecutionContextV1,
        reads: &ReadDependencyRequestV3,
    ) -> Result<usize, InvocationError> {
        read_bytes(reads)?
            .checked_add(extra_bytes(c)?)
            .ok_or(InvocationError::Limit)
    }
}
fn state(intent: &Option<BoundIntent>) -> Option<&State> {
    match intent.as_ref()? {
        BoundIntent::Reversal { result, .. } => result.as_deref(),
        BoundIntent::DependentReversal(i) => i.result.as_deref(),
        _ => None,
    }
}
pub(in crate::coordinator::invocation) fn state_mut(
    intent: &mut Option<BoundIntent>,
) -> Option<&mut State> {
    match intent.as_mut()? {
        BoundIntent::Reversal { result, .. } => result.as_deref_mut(),
        BoundIntent::DependentReversal(i) => i.result.as_deref_mut(),
        _ => None,
    }
}
fn original_matches(c: &Capture, intent: &Option<BoundIntent>) -> bool {
    match intent {
        Some(BoundIntent::Reversal {
            group, direction, ..
        }) => {
            c.direction == *direction
                && matches!(&c.target, HistoryReversalTargetV1::Group(g) if g == group)
        }
        Some(BoundIntent::DependentReversal(i)) => {
            c.target == i.target && c.direction == i.direction
        }
        _ => false,
    }
}
pub(in crate::coordinator::invocation) fn capture_for_run(
    id: &InvocationId,
    intent: &Option<BoundIntent>,
) -> Result<Option<Arc<Capture>>, InvocationError> {
    let Some(s) = state(intent) else {
        return Ok(None);
    };
    if s.original.invocation != *id || !original_matches(&s.original, intent) || s.receipt.is_some()
    {
        return Err(InvocationError::Faulted);
    }
    Ok(Some(s.original.clone()))
}
fn grant_value(
    id: &InvocationId,
    authority: &str,
    target: &HistoryReversalTargetV1,
    grant: Option<TrustedReversalResultGrantV1<'_>>,
) -> Result<bool, InvocationError> {
    match grant {
        None => Ok(false),
        Some(g) if g.invocation == id && g.authority == authority && g.target == target => {
            Ok(g.disclose_target)
        }
        _ => Err(InvocationError::Unavailable),
    }
}
impl SourceCoordinator {
    pub fn submit_reversal_with_result_v1(
        &mut self,
        id: &InvocationId,
        group: &HistoryGroupId,
        direction: HistoryDirection,
        f: &TrustedReversalResultAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_reversal_result_input(
            id,
            &HistoryReversalTargetV1::Group(group.clone()),
            direction,
            None,
            f,
            r,
        )
    }
    /// Original frontier/read conjunction stays with the existing V3 handler;
    /// this result opt-in cannot replace it with a newly resolved target.
    #[allow(clippy::too_many_arguments)]
    pub fn submit_reversal_target_with_reads_and_result_v3(
        &mut self,
        id: &InvocationId,
        target: &HistoryReversalTargetV1,
        direction: HistoryDirection,
        reads: &ReadDependencyRequestV3,
        f: &TrustedReversalResultAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_reversal_result_input(id, target, direction, Some(reads), f, r)
    }
    #[allow(clippy::too_many_arguments)]
    fn submit_reversal_result_input(
        &mut self,
        id: &InvocationId,
        target: &HistoryReversalTargetV1,
        direction: HistoryDirection,
        reads: Option<&ReadDependencyRequestV3>,
        f: &TrustedReversalResultAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(id)?;
            if f.context_current != Some(true) || f.result_admission_allowed != Some(true) {
                return Err(InvocationError::Unavailable);
            }
            let publication = f.publication_allowed.ok_or(InvocationError::Unavailable)?;
            if labels(f.context).iter().any(|s| s.is_empty()) {
                return Err(InvocationError::MalformedIntent);
            }
            let target_ceiling = grant_value(id, &f.context.authority, target, f.target_grant)?;
            let base = match reads {
                Some(reads) => read_bytes(reads)?,
                None if matches!(target, HistoryReversalTargetV1::Group(_)) => direct_bytes(),
                None => return Err(InvocationError::MalformedIntent),
            };
            let intent_bytes = base
                .checked_add(extra_bytes(f.context)?)
                .ok_or(InvocationError::Limit)?;
            if intent_bytes > self.invocations.limits.max_intent_bytes
                || f.max_result_bytes < ReversalResultV1::required_bytes()
                || f.max_result_bytes > self.invocations.limits.max_intent_bytes
            {
                return Err(InvocationError::Limit);
            }
            let prepared_state = if slot.intent.is_some() {
                let original = state(&slot.intent).ok_or(InvocationError::Unavailable)?;
                let c = &original.original;
                if c.invocation != *id {
                    return Err(InvocationError::Faulted);
                }
                if c.target != *target
                    || c.direction != direction
                    || c.context != *f.context
                    || c.epoch != f.publication_epoch
                    || c.publication != publication
                    || c.target_ceiling != target_ceiling
                    || c.maximum != f.max_result_bytes
                    || c.intent_bytes != intent_bytes
                {
                    return Err(InvocationError::IntentMismatch);
                }
                None
            } else {
                if !matches!(slot.phase, Phase::Issued(_)) {
                    return Err(InvocationError::Unavailable);
                }
                let context = crate::coordinator::settings::basis::copy_context(f.context)
                    .map_err(|e| {
                        if e == crate::settings::SettingsError::Capacity {
                            InvocationError::Capacity
                        } else {
                            InvocationError::Limit
                        }
                    })?;
                Some(Box::new(State {
                    original: Arc::new(Capture {
                        invocation: id.clone(),
                        target: target.clone(),
                        direction,
                        context,
                        epoch: f.publication_epoch,
                        publication,
                        target_ceiling,
                        maximum: f.max_result_bytes,
                        intent_bytes,
                    }),
                    receipt: None,
                }))
            };
            let status = match reads {
                Some(reads) => {
                    self.submit_reversal_target_with_reads_v3(id, target, direction, reads, r)?
                }
                None => self.submit_reversal(id, target.group(), direction, r)?,
            };
            if let Some(prepared) = prepared_state {
                let slot = self.invocations.slots[id.slot]
                    .as_mut()
                    .ok_or(InvocationError::Faulted)?;
                match slot.intent.as_mut() {
                    Some(BoundIntent::Reversal { result, .. }) => *result = Some(prepared),
                    Some(BoundIntent::DependentReversal(i)) => i.result = Some(prepared),
                    _ => return Err(InvocationError::Faulted),
                }
            }
            Ok(status)
        })();
        capture_timed(
            r,
            Boundary::ReversalResultAdmission,
            &result,
            timing.finish(),
        );
        result
    }
    /// Called only after complete inverse/frontier/outer-root validation, before
    /// any installation. It copies fixed native scalar/opaque identity headers.
    pub(in crate::coordinator) fn stage_reversal_result_for_commit(
        &self,
        original: &Arc<Capture>,
        prepared: &PreparedTransaction,
        history: &history::HistoryDecision,
        recorder: &mut Recorder,
    ) -> Result<Receipt, CoordinatorError> {
        let timing = recorder.begin_service_timing();
        let valid = (|| {
            let slot = self
                .invocation_slot(&original.invocation)
                .map_err(|_| CoordinatorError::Faulted)?;
            let s = state(&slot.intent).ok_or(CoordinatorError::Faulted)?;
            if !Arc::ptr_eq(&s.original, original)
                || s.receipt.is_some()
                || !original_matches(original, &slot.intent)
            {
                return Err(CoordinatorError::Faulted);
            }
            if !matches!(&prepared.history, history::PreparedHistory::Reversal { group, direction, .. } if group == original.target.group() && *direction == original.direction)
                || history.group_id().as_ref() != Some(original.target.group())
            {
                return Err(CoordinatorError::Faulted);
            }
            if original.intent_bytes > self.invocations.limits.max_intent_bytes
                || ReversalResultV1::required_bytes()
                    > original
                        .maximum
                        .min(self.invocations.limits.max_intent_bytes)
            {
                return Err(CoordinatorError::Limit);
            }
            Ok(Receipt {
                invocation: original.invocation.clone(),
                operation: original.direction,
                decision: Decision::Applied(original.invocation.clone()),
            })
        })();
        crate::coordinator::observe_service(
            recorder,
            Boundary::ReversalResultStaging,
            &valid,
            timing.finish(),
        );
        valid
    }
    pub(in crate::coordinator::invocation) fn finish_reversal_result(
        &mut self,
        id: &InvocationId,
        original: &Arc<Capture>,
        decision: &Result<CommitOutcome, NativePreparationError>,
        staged: Option<Receipt>,
    ) -> Result<(), InvocationError> {
        let receipt = match decision {
            Ok(outcome)
                if outcome.operation.is_some()
                    && outcome.history_group.as_ref() == Some(original.target.group()) =>
            {
                staged.ok_or(InvocationError::Faulted)?
            }
            Err(error) if !error.faulted() => Receipt {
                invocation: id.clone(),
                operation: original.direction,
                decision: Decision::NotPublished,
            },
            _ => return Err(InvocationError::Faulted),
        };
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .ok_or(InvocationError::Faulted)?;
        let s = state_mut(&mut slot.intent).ok_or(InvocationError::Faulted)?;
        if !Arc::ptr_eq(&s.original, original)
            || s.receipt.is_some()
            || receipt.invocation != *id
            || receipt.operation != original.direction
        {
            return Err(InvocationError::Faulted);
        }
        s.receipt = Some(receipt);
        Ok(())
    }
    /// Historical native decision projection, not an execution/read grant or a
    /// substitute for an admitted F06 policy context and delivery workspace.
    pub fn retained_reversal_result_v1(
        &self,
        id: &InvocationId,
        f: &TrustedReversalResultReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<ReversalResultV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if self.faulted
                || [
                    f.publication_allowed,
                    f.outcome_readable,
                    f.result_context_current,
                    f.reply_ready,
                    f.resources_ready,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(InvocationError::Unavailable);
            }
            let slot = self.invocation_slot(id)?;
            let s = state(&slot.intent).ok_or(InvocationError::Unavailable)?;
            let c = &s.original;
            if c.invocation != *id || !original_matches(c, &slot.intent) {
                return Err(InvocationError::Faulted);
            }
            if !c.publication || c.context != *f.context || c.epoch != f.publication_epoch {
                return Err(InvocationError::Unavailable);
            }
            if !matches!(slot.phase, Phase::Terminal(Some(_)))
                || matches!(
                    slot.phase,
                    Phase::Terminal(Some(InvocationOutcome::StoppedBeforePublication))
                )
            {
                return Err(InvocationError::OutcomeUnavailable);
            }
            let receipt = s.receipt.as_ref().ok_or(InvocationError::Faulted)?;
            if receipt.invocation != *id || receipt.operation != c.direction {
                return Err(InvocationError::Faulted);
            }
            let decision = match (&receipt.decision, &slot.phase) {
                (
                    Decision::Applied(publication),
                    Phase::Terminal(Some(InvocationOutcome::Committed(outcome))),
                ) if publication == id
                    && outcome.operation.is_some()
                    && outcome.history_group.as_ref() == Some(c.target.group()) =>
                {
                    let current = grant_value(id, &c.context.authority, &c.target, f.target_grant)?;
                    ReversalDecisionV1::Applied {
                        publication: publication.clone(),
                        target: (c.target_ceiling && current).then(|| c.target.clone()),
                    }
                }
                (Decision::NotPublished, Phase::Terminal(Some(InvocationOutcome::Rejected(_)))) => {
                    ReversalDecisionV1::NotPublished
                }
                _ => return Err(InvocationError::Faulted),
            };
            if ReversalResultV1::required_bytes()
                > c.maximum
                    .min(f.max_result_bytes)
                    .min(self.invocations.limits.max_intent_bytes)
            {
                return Err(InvocationError::Limit);
            }
            Ok(ReversalResultV1 {
                invocation: id.clone(),
                operation: c.direction,
                decision,
            })
        })();
        capture_timed(r, Boundary::ReversalResultRelease, &result, timing.finish());
        result
    }
}
