//! Acknowledged cut uses the original task's private native invocation claim.
//! No second root publisher, clipboard undo or shadow "committed" bit.
use super::*;

impl<A: ResourceAdapter> LocalHost<A> {
    /// Current task evidence + retained native local decision under one exclusive
    /// host borrow. Later contradictions change a later report, not source/undo.
    pub fn cut_report(&mut self, task: &CopyTaskV1) -> Result<CutReportV1, SessionError> {
        let local = self.cut_disposition(task)?;
        let external = self.copy_status(task)?;
        Ok(CutReportV1 {
            external,
            local,
            copied_not_cut: external.effect == CopyEffectV1::Accepted
                && matches!(
                    local,
                    TransferLocalDispositionV1::NotApplied | TransferLocalDispositionV1::Stopped
                ),
        })
    }
    /// Historical local decision from the native retained outcome, independent
    /// of current external evidence. Undo or a late conflicting receipt never
    /// rewrites that decision. This is privileged embedding-host disclosure.
    pub fn cut_disposition(
        &mut self,
        task: &CopyTaskV1,
    ) -> Result<TransferLocalDispositionV1, SessionError> {
        let c = self.copies()?;
        let t = &c.tasks[c.task(task)?];
        if t.operation != TextTransferKindV1::Cut {
            return Err(fail(CopyErrorV1::Unavailable));
        }
        if t.local_faulted {
            return Ok(TransferLocalDispositionV1::Unknown);
        }
        if !t.local_started {
            return Ok(if t.capture.plain().is_none() {
                TransferLocalDispositionV1::Unchanged
            } else if t.stopped {
                TransferLocalDispositionV1::Stopped
            } else {
                TransferLocalDispositionV1::Pending
            });
        }
        let origin = t.claim.origin().clone();
        Ok(
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
            },
        )
    }
    /// Acknowledgment is necessary, quiescence insufficient. Revalidate exact
    /// source/view/selection plus all native companions through literal editing.
    /// Current denial/refusal is terminal: restoring rights cannot retry this
    /// old target. This call performs no provider I/O or arbitrary callbacks.
    pub fn continue_cut(
        &mut self,
        task: &CopyTaskV1,
        facts: CutPublicationFactsV1,
    ) -> Result<TransferLocalDispositionV1, SessionError> {
        let result = (|| {
            let prior = self.cut_disposition(task)?;
            if prior != TransferLocalDispositionV1::Pending {
                return Ok(prior);
            }
            let c = self.copies_mut()?;
            if c.usage.cut_fenced {
                return Err(fail(CopyErrorV1::Unavailable));
            }
            let i = c.task(task)?;
            let t = &c.tasks[i];
            if t.effect() != CopyEffectV1::Accepted {
                return Err(fail(CopyErrorV1::Outstanding));
            }
            let allowed = c.providers[t.provider].active
                && yes(facts.edit_allowed)
                && yes(facts.view_write_allowed);
            let origin = t.claim.origin().clone();
            let context = t.capture.context().clone();
            c.tasks[i].local_started = true;
            // Text is empty, so the immutable native descriptor needs no text
            // allocation; its fixed width was checked before export admission.
            // Any unexpected admission/service failure retains Unknown and the
            // original task/claim. Never reinterpret it as safe nonapplication.
            let service = self
                .owner
                .submit_literal_edit_with_permission_v1(
                    &origin,
                    &context,
                    "",
                    SelectionAfterV1::CaretsAfter,
                    Some(allowed),
                    &mut self.diagnostics,
                )
                .and_then(|_| self.owner.run(&origin, &mut self.diagnostics));
            if let Err(error) = service {
                let c = self.copies_mut()?;
                c.tasks[i].local_faulted = true;
                c.usage.cut_fenced = true;
                c.usage.cut_unknown = c.usage.cut_unknown.saturating_add(1);
                return Err(error.into());
            }
            // Local disposition is derived from the native terminal facts, not
            // assigned separately after a successful source commit. The sole
            // mutable host borrow excludes intermediate observers/reentrancy.
            let disposition = self.cut_disposition(task)?;
            let c = self.copies_mut()?;
            match disposition {
                TransferLocalDispositionV1::Applied => {
                    c.usage.cut_applied = c.usage.cut_applied.saturating_add(1)
                }
                TransferLocalDispositionV1::NotApplied | TransferLocalDispositionV1::Stopped => {
                    c.usage.cut_not_applied = c.usage.cut_not_applied.saturating_add(1)
                }
                TransferLocalDispositionV1::Unknown => {
                    c.tasks[i].local_faulted = true;
                    c.usage.cut_fenced = true;
                    c.usage.cut_unknown = c.usage.cut_unknown.saturating_add(1)
                }
                TransferLocalDispositionV1::Pending | TransferLocalDispositionV1::Unchanged => {}
            }
            Ok(disposition)
        })();
        self.copy_result(CopyOperationV1::ContinueCut, result)
    }
}
