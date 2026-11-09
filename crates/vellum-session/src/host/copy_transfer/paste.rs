//! Original-target, plain literal paste through the shared transfer owner.
use super::*;
use vellum_engine::coordinator::InvocationState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PasteLimitsV1 {
    pub max_targets: usize,
    pub max_plain_bytes: usize,
    pub max_expanded_bytes: usize,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PasteAdmissionFactsV1 {
    pub target_read_allowed: Option<bool>,
    pub edit_allowed: Option<bool>,
    pub view_write_allowed: Option<bool>,
    pub retention_allowed: Option<bool>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PasteDispatchFactsV1 {
    pub launch_allowed: Option<bool>,
    pub clipboard_read_allowed: Option<bool>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PasteEvidenceFactsV1 {
    pub evidence: CopyEvidenceFactsV1,
    pub payload_retention_allowed: Option<bool>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PastePublicationFactsV1 {
    pub edit_allowed: Option<bool>,
    pub view_write_allowed: Option<bool>,
    pub payload_use_allowed: Option<bool>,
}
/// Borrowed UTF-8 insertion data, never command text or a key-event stream.
/// Deliberately no Debug implementation that might print clipboard content.
pub enum PasteEvidenceV1<'a> {
    Accepted(&'a str),
    NotApplied,
    Quiescent,
}
impl std::fmt::Debug for PasteEvidenceV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PasteEvidenceV1(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PasteReportV1 {
    pub external: CopyStatusV1,
    pub local: TransferLocalDispositionV1,
}
pub(super) struct Payloads {
    canonical: String,
    conflict: String,
    conflict_present: bool,
    native: Option<String>,
}
fn reserved(bytes: usize) -> Result<String, SessionError> {
    let mut text = String::new();
    text.try_reserve_exact(bytes)
        .map_err(|_| fail(CopyErrorV1::Capacity))?;
    Ok(text)
}
impl LocalHost {
    /// Explicit bounded plain-paste profile. Provider registrations and total
    /// task/held-byte bounds are shared with copy/cut, not a second registry.
    pub fn configure_paste_transfers(&mut self, limits: PasteLimitsV1) -> Result<(), SessionError> {
        let c = self.copies_mut()?;
        if c.paste_limits.is_some() {
            return Err(fail(CopyErrorV1::Unavailable));
        }
        c.paste_limits = Some(limits);
        Ok(())
    }
    fn validate_paste_target(
        &mut self,
        context: &LogicalViewContext,
    ) -> Result<vellum_engine::selection::SelectionSet, SessionError> {
        let snapshot = self
            .owner
            .logical_view(&context.view, &mut self.diagnostics)?;
        if snapshot.context().as_ref() != Some(context) {
            return Err(fail(CopyErrorV1::Unavailable));
        }
        let read = self
            .owner
            .read(&context.source.document, &mut self.diagnostics)?;
        if read.version != context.source {
            return Err(fail(CopyErrorV1::Unavailable));
        }
        Ok(snapshot.selections)
    }
    /// Capture only original target identity/context/selections, not selected
    /// source fragments. Reserve canonical/conflicting/native text buffers before
    /// any provider work. The original native lifetime is privately claimed.
    pub fn admit_paste(
        &mut self,
        origin: &InvocationId,
        context: &LogicalViewContext,
        provider: &CopyProviderPortV1,
        facts: PasteAdmissionFactsV1,
    ) -> Result<CopyTaskV1, SessionError> {
        let result = (|| {
            if !yes(facts.target_read_allowed)
                || !yes(facts.edit_allowed)
                || !yes(facts.view_write_allowed)
                || !yes(facts.retention_allowed)
            {
                return Err(fail(CopyErrorV1::Denied));
            }
            let c = self.copies_mut()?;
            if c.usage.paste_fenced {
                return Err(fail(CopyErrorV1::Unavailable));
            }
            let limits = c.paste_limits.ok_or(fail(CopyErrorV1::Unavailable))?;
            let provider = c.provider(provider)?;
            if let Some(t) = c.tasks.iter().find(|t| t.claim.origin() == origin) {
                if t.operation != TextTransferKindV1::Paste
                    || t.provider != provider
                    || t.capture.context() != context
                {
                    return Err(fail(CopyErrorV1::IntentMismatch));
                }
                c.usage.request_replays = c.usage.request_replays.saturating_add(1);
                return Ok(CopyTaskV1 {
                    origin: origin.clone(),
                });
            }
            let p = &c.providers[provider];
            if !p.active || !p.registration.as_ref().is_some_and(|r| r.readable) {
                return Err(fail(CopyErrorV1::Unavailable));
            }
            if c.tasks.len() >= c.limits.max_tasks {
                return Err(fail(CopyErrorV1::Limit));
            }
            if !self
                .owner
                .can_retain_literal_intent_v1(limits.max_plain_bytes)
            {
                return Err(fail(CopyErrorV1::Limit));
            }
            if self.owner.invocation_state(origin, &mut self.diagnostics)?
                != InvocationState::Issued
            {
                return Err(fail(CopyErrorV1::Unavailable));
            }
            let selections = self.validate_paste_target(context)?;
            if selections.selections().len() > limits.max_targets {
                return Err(fail(CopyErrorV1::Limit));
            }
            let target_bytes = selections
                .selections()
                .len()
                .checked_mul(std::mem::size_of::<vellum_engine::selection::Selection>())
                .ok_or(fail(CopyErrorV1::Limit))?;
            let body_bytes = limits
                .max_plain_bytes
                .checked_mul(3)
                .ok_or(fail(CopyErrorV1::Limit))?;
            let charge = add(add(std::mem::size_of::<Task>(), target_bytes)?, body_bytes)?;
            let c = self.copies_mut()?;
            let total = c.charge(charge)?;
            c.tasks
                .try_reserve_exact(1)
                .map_err(|_| fail(CopyErrorV1::Capacity))?;
            let payloads = Payloads {
                canonical: reserved(limits.max_plain_bytes)?,
                conflict: reserved(limits.max_plain_bytes)?,
                conflict_present: false,
                native: Some(reserved(limits.max_plain_bytes)?),
            };
            let claim = self.claim_invocation(origin, InvocationClaimKindV1::SourcePublication)?;
            let c = self.copies_mut()?;
            c.tasks.push(Task {
                operation: TextTransferKindV1::Paste,
                local_started: false,
                local_faulted: false,
                claim,
                capture: Capture::Target {
                    context: context.clone(),
                    selections,
                },
                paste: Some(payloads),
                provider,
                charge,
                dispatched: false,
                stopped: false,
                accepted: false,
                not_applied: false,
                contested: false,
                quiescent: true,
                start: None,
                first_result_after: None,
                quiescent_after: None,
            });
            c.usage.held_tasks = c.tasks.len();
            c.usage.held_bytes = total;
            c.usage.admitted = c.usage.admitted.saturating_add(1);
            Ok(CopyTaskV1 {
                origin: origin.clone(),
            })
        })();
        self.copy_result(CopyOperationV1::AdmitPaste, result)
    }
    /// One dispatch after renewed original-target and current full-registration/
    /// launch/read checks. No OS clipboard access or implicit retry occurs here.
    pub fn dispatch_paste(
        &mut self,
        task: &CopyTaskV1,
        facts: PasteDispatchFactsV1,
    ) -> Result<CopyDispatchV1, SessionError> {
        let result = (|| {
            if !yes(facts.launch_allowed) || !yes(facts.clipboard_read_allowed) {
                return Err(fail(CopyErrorV1::Denied));
            }
            let c = self.copies()?;
            let i = c.task(task)?;
            let t = &c.tasks[i];
            if t.operation != TextTransferKindV1::Paste {
                return Err(fail(CopyErrorV1::Unavailable));
            }
            if t.dispatched {
                return Err(fail(CopyErrorV1::AlreadyDispatched));
            }
            if t.stopped
                || !c.providers[t.provider].active
                || !c.providers[t.provider]
                    .registration
                    .as_ref()
                    .is_some_and(|r| r.readable)
            {
                return Err(fail(CopyErrorV1::Unavailable));
            }
            let context = t.capture.context().clone();
            self.validate_paste_target(&context)?;
            let c = self.copies_mut()?;
            let t = &mut c.tasks[i];
            let token = CopyDispatchV1 {
                origin: t.claim.origin().clone(),
                provider: c.providers[t.provider].identity.clone(),
            };
            t.dispatched = true;
            t.quiescent = false;
            t.start = match c.timing {
                NativeTimingProfileV1::Off => None,
                NativeTimingProfileV1::ProcessMonotonicV1 => instant(),
            };
            c.usage.dispatched = c.usage.dispatched.saturating_add(1);
            Ok(token)
        })();
        self.copy_result(CopyOperationV1::DispatchPaste, result)
    }
    /// Authenticate and bound first; invalid input cannot taint a good receipt.
    /// Equal duplicates are inert. A second accepted body or incompatible known
    /// result remains contested with capped witnesses, never last-writer-wins.
    pub fn record_paste_evidence(
        &mut self,
        provider: &CopyProviderPortV1,
        dispatch: &CopyDispatchV1,
        evidence: PasteEvidenceV1<'_>,
        facts: PasteEvidenceFactsV1,
    ) -> Result<CopyEvidenceDispositionV1, SessionError> {
        let result = (|| {
            let f = facts.evidence;
            if !yes(f.authentic) || !yes(f.evidence_allowed) || !yes(f.ingress_ready) {
                return Err(fail(CopyErrorV1::Denied));
            }
            let c = self.copies_mut()?;
            let i = c.dispatch(provider, dispatch)?;
            let limits = c.paste_limits.ok_or(fail(CopyErrorV1::Unavailable))?;
            let t = &mut c.tasks[i];
            if t.operation != TextTransferKindV1::Paste {
                return Err(fail(CopyErrorV1::Unavailable));
            }
            if let PasteEvidenceV1::Accepted(text) = evidence {
                if !yes(facts.payload_retention_allowed) {
                    return Err(fail(CopyErrorV1::Denied));
                }
                let targets = match &t.capture {
                    Capture::Target { selections, .. } => selections.selections().len(),
                    _ => return Err(fail(CopyErrorV1::Unavailable)),
                };
                if text.len() > limits.max_plain_bytes
                    || text
                        .len()
                        .checked_mul(targets)
                        .is_none_or(|n| n > limits.max_expanded_bytes)
                {
                    return Err(fail(CopyErrorV1::Limit));
                }
            }
            let payloads = t.paste.as_mut().ok_or(fail(CopyErrorV1::Unavailable))?;
            let duplicate = match evidence {
                PasteEvidenceV1::Accepted(text) => {
                    t.accepted
                        && (text == payloads.canonical
                            || (payloads.conflict_present && text == payloads.conflict))
                }
                PasteEvidenceV1::NotApplied => t.not_applied,
                PasteEvidenceV1::Quiescent => t.quiescent,
            };
            if duplicate {
                c.usage.receipts_duplicated = c.usage.receipts_duplicated.saturating_add(1);
                return Ok(CopyEvidenceDispositionV1::Duplicate);
            }
            let was_contested = t.contested;
            match evidence {
                PasteEvidenceV1::Accepted(text) => {
                    if t.accepted {
                        if !payloads.conflict_present {
                            payloads.conflict.push_str(text);
                            payloads.conflict_present = true;
                        }
                        t.contested = true;
                    } else {
                        if !t.not_applied {
                            t.first_result_after = age(t.start);
                        }
                        payloads.canonical.push_str(text);
                        // Capacity for this native intent was reserved at admission.
                        payloads
                            .native
                            .as_mut()
                            .ok_or(fail(CopyErrorV1::Unavailable))?
                            .push_str(text);
                        t.accepted = true;
                        t.contested = t.not_applied;
                    }
                }
                PasteEvidenceV1::NotApplied => {
                    if !t.accepted {
                        t.first_result_after = age(t.start);
                    }
                    t.not_applied = true;
                    t.contested = t.accepted;
                }
                PasteEvidenceV1::Quiescent => {
                    t.quiescent = true;
                    t.quiescent_after = age(t.start);
                }
            }
            c.usage.receipts_recorded = c.usage.receipts_recorded.saturating_add(1);
            if !was_contested && t.contested {
                c.usage.conflicts = c.usage.conflicts.saturating_add(1);
            }
            Ok(if t.contested {
                CopyEvidenceDispositionV1::Contested
            } else {
                CopyEvidenceDispositionV1::Recorded
            })
        })();
        self.copy_result(CopyOperationV1::PasteEvidence, result)
    }
    /// One serialized report: current external evidence and historical native
    /// local outcome. Applied includes genuine native view/companion-only effects.
    pub fn paste_report(&mut self, task: &CopyTaskV1) -> Result<PasteReportV1, SessionError> {
        let c = self.copies()?;
        let t = &c.tasks[c.task(task)?];
        if t.operation != TextTransferKindV1::Paste {
            return Err(fail(CopyErrorV1::Unavailable));
        }
        let origin = t.claim.origin().clone();
        let local = if t.local_faulted {
            TransferLocalDispositionV1::Unknown
        } else if !t.local_started {
            if t.stopped {
                TransferLocalDispositionV1::Stopped
            } else {
                TransferLocalDispositionV1::Pending
            }
        } else {
            match self
                .owner
                .invocation_outcome(&origin, &mut self.diagnostics)
            {
                Ok(InvocationOutcome::Committed(o)) => {
                    if o.operation.is_some() {
                        TransferLocalDispositionV1::Applied
                    } else {
                        TransferLocalDispositionV1::Unchanged
                    }
                }
                Ok(InvocationOutcome::Rejected(_)) => TransferLocalDispositionV1::NotApplied,
                Ok(InvocationOutcome::StoppedBeforePublication) => {
                    TransferLocalDispositionV1::Stopped
                }
                _ => TransferLocalDispositionV1::Unknown,
            }
        };
        Ok(PasteReportV1 {
            external: self.copy_status(task)?,
            local,
        })
    }
    /// Broadcast the complete plain literal to every originally captured target.
    /// Source/view/policy/annotations/history use the native publisher. No focus
    /// refresh, newline splitting, normalization, key replay or provider callback.
    pub fn continue_paste(
        &mut self,
        task: &CopyTaskV1,
        facts: PastePublicationFactsV1,
    ) -> Result<TransferLocalDispositionV1, SessionError> {
        let result = (|| {
            let prior = self.paste_report(task)?.local;
            if prior != TransferLocalDispositionV1::Pending {
                return Ok(prior);
            }
            let c = self.copies_mut()?;
            if c.usage.paste_fenced {
                return Err(fail(CopyErrorV1::Unavailable));
            }
            let i = c.task(task)?;
            let t = &mut c.tasks[i];
            if t.effect() != CopyEffectV1::Accepted {
                return Err(fail(CopyErrorV1::Outstanding));
            }
            let allowed = c.providers[t.provider].active
                && yes(facts.edit_allowed)
                && yes(facts.view_write_allowed)
                && yes(facts.payload_use_allowed);
            let origin = t.claim.origin().clone();
            let context = t.capture.context().clone();
            let text = t
                .paste
                .as_mut()
                .and_then(|p| p.native.take())
                .ok_or(fail(CopyErrorV1::Unavailable))?;
            t.local_started = true;
            let service = self
                .owner
                .submit_owned_literal_edit_with_permission_v1(
                    &origin,
                    &context,
                    text,
                    SelectionAfterV1::CaretsAfter,
                    Some(allowed),
                    &mut self.diagnostics,
                )
                .and_then(|_| self.owner.run(&origin, &mut self.diagnostics));
            if let Err(error) = service {
                let c = self.copies_mut()?;
                c.tasks[i].local_faulted = true;
                c.usage.paste_fenced = true;
                c.usage.paste_unknown = c.usage.paste_unknown.saturating_add(1);
                return Err(error.into());
            }
            let local = self.paste_report(task)?.local;
            let c = self.copies_mut()?;
            match local {
                TransferLocalDispositionV1::Applied => {
                    c.usage.paste_applied = c.usage.paste_applied.saturating_add(1)
                }
                TransferLocalDispositionV1::Unchanged => {
                    c.usage.paste_unchanged = c.usage.paste_unchanged.saturating_add(1)
                }
                TransferLocalDispositionV1::NotApplied | TransferLocalDispositionV1::Stopped => {
                    c.usage.paste_not_applied = c.usage.paste_not_applied.saturating_add(1)
                }
                TransferLocalDispositionV1::Unknown | TransferLocalDispositionV1::Pending => {
                    c.tasks[i].local_faulted = true;
                    c.usage.paste_fenced = true;
                    c.usage.paste_unknown = c.usage.paste_unknown.saturating_add(1);
                }
            }
            Ok(local)
        })();
        self.copy_result(CopyOperationV1::ContinuePaste, result)
    }
}
