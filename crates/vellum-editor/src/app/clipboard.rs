//! Presentation task ownership above the existing trusted transfer facade.
//! No OS provider, clipboard keymap or independent semantic/root publisher.
use super::*;
use vellum_engine::coordinator::{InvocationId, InvocationOutcome};
use vellum_session::host::copy_transfer::*;

/// Clones identify this same original operation, never a fresh attempt or grant.
#[derive(Clone)]
pub struct ClipboardPlan {
    origin: InvocationId,
}
impl std::fmt::Debug for ClipboardPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ClipboardPlan(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipboardReport {
    pub external: CopyStatusV1,
    pub local: Option<TransferLocalDispositionV1>,
    pub copied_not_cut: bool,
    pub target_is_active: bool,
    /// First presentation adoption, not a new source publication on replay.
    pub fresh_local_decision: bool,
    /// Source publication is not rolled back or relabeled on UI refresh failure.
    pub view_refresh_failed: bool,
}
pub(super) struct Attempt {
    origin: InvocationId,
    task: Option<CopyTaskV1>,
    tab: usize,
    kind: TextTransferKindV1,
    history_recorded: bool,
}
enum Admission {
    Copy(CopyAdmissionFactsV1),
    Cut(CutAdmissionFactsV1),
    Paste(PasteAdmissionFactsV1),
}
impl Admission {
    fn kind(&self) -> TextTransferKindV1 {
        match self {
            Self::Copy(_) => TextTransferKindV1::Copy,
            Self::Cut(_) => TextTransferKindV1::Cut,
            Self::Paste(_) => TextTransferKindV1::Paste,
        }
    }
}
impl Editor {
    /// Explicit opt-in; native invocation and view profiles must also be enabled.
    /// Limits are caller policy, not new product defaults or OS authorization.
    pub fn configure_clipboard(
        &mut self,
        limits: CopyLimitsV1,
        paste: Option<PasteLimitsV1>,
    ) -> Result<(), EditorError> {
        self.live()?;
        self.host.configure_copy_transfers(limits)?;
        self.clipboard_limit = Some(limits.max_tasks);
        if let Some(paste) = paste {
            self.host.configure_paste_transfers(paste)?;
        }
        Ok(())
    }
    pub fn register_clipboard_provider(
        &mut self,
        registration: CopyProviderRegistrationV1,
    ) -> Result<CopyProviderPortV1, EditorError> {
        self.live()?;
        Ok(self.host.register_copy_provider(registration)?)
    }
    pub fn revoke_clipboard_provider(
        &mut self,
        provider: &CopyProviderPortV1,
    ) -> Result<(), EditorError> {
        self.live()?;
        Ok(self.host.revoke_copy_provider(provider)?)
    }
    pub fn retire_clipboard_provider(
        &mut self,
        provider: &CopyProviderPortV1,
        facts: CopyRetirementFactsV1,
    ) -> Result<(), EditorError> {
        self.live()?;
        let result = self.host.retire_copy_provider(provider, facts);
        self.collect_diagnostics();
        Ok(result?)
    }
    pub fn begin_copy(
        &mut self,
        provider: &CopyProviderPortV1,
        facts: CopyAdmissionFactsV1,
    ) -> Result<ClipboardPlan, EditorError> {
        self.begin_clipboard(provider, Admission::Copy(facts))
    }
    pub fn begin_cut(
        &mut self,
        provider: &CopyProviderPortV1,
        facts: CutAdmissionFactsV1,
    ) -> Result<ClipboardPlan, EditorError> {
        self.begin_clipboard(provider, Admission::Cut(facts))
    }
    pub fn begin_paste(
        &mut self,
        provider: &CopyProviderPortV1,
        facts: PasteAdmissionFactsV1,
    ) -> Result<ClipboardPlan, EditorError> {
        self.begin_clipboard(provider, Admission::Paste(facts))
    }
    fn begin_clipboard(
        &mut self,
        provider: &CopyProviderPortV1,
        admission: Admission,
    ) -> Result<ClipboardPlan, EditorError> {
        self.live()?;
        let limit = self
            .clipboard_limit
            .ok_or(EditorError::ClipboardUnavailable)?;
        if self.clipboard_attempts.len() >= limit {
            return Err(EditorError::Capacity);
        }
        let context = self
            .native_selection()?
            .and_then(|s| s.context())
            .ok_or(EditorError::Position)?;
        let kind = admission.kind();
        // Locked cut/paste must refuse before any external disclosure/request.
        if kind != TextTransferKindV1::Copy && self.tabs[self.active].editing_locked {
            return Err(SessionError::CopyTransfer(CopyErrorV1::Denied).into());
        }
        self.clipboard_attempts
            .try_reserve(1)
            .map_err(|_| EditorError::Capacity)?;
        let origin = self.host.issue_invocation()?;
        let index = self.clipboard_attempts.len();
        // Retain the original control before the next fallible admission step.
        self.clipboard_attempts.push(Attempt {
            origin: origin.clone(),
            task: None,
            tab: self.active,
            kind,
            history_recorded: false,
        });
        let result = match admission {
            Admission::Copy(facts) => self.host.admit_copy(&origin, &context, provider, facts),
            Admission::Cut(facts) => self.host.admit_cut(&origin, &context, provider, facts),
            Admission::Paste(facts) => self.host.admit_paste(&origin, &context, provider, facts),
        };
        self.collect_diagnostics();
        match result {
            Ok(task) => {
                self.clipboard_attempts[index].task = Some(task);
                Ok(ClipboardPlan { origin })
            }
            Err(error) => {
                // Ordinary refusals leave an unsubmitted native identity. If
                // retirement fails, retain it for explicit cleanup/close refusal.
                if self.host.retire_invocation(&origin).is_ok() {
                    self.clipboard_attempts.swap_remove(index);
                }
                Err(error.into())
            }
        }
    }
    fn clipboard_index(&self, plan: &ClipboardPlan) -> Result<usize, EditorError> {
        self.clipboard_attempts
            .iter()
            .position(|a| a.origin == plan.origin)
            .ok_or(EditorError::Position)
    }
    fn clipboard_task(&self, plan: &ClipboardPlan) -> Result<CopyTaskV1, EditorError> {
        self.clipboard_attempts[self.clipboard_index(plan)?]
            .task
            .clone()
            .ok_or(EditorError::ClipboardPending)
    }
    /// Exactly one provider dispatch. These calls perform no provider I/O.
    pub fn dispatch_clipboard_export(
        &mut self,
        plan: &ClipboardPlan,
        facts: CopyDispatchFactsV1,
    ) -> Result<CopyDispatchV1, EditorError> {
        self.live()?;
        let task = self.clipboard_task(plan)?;
        let result = self.host.dispatch_copy(&task, facts);
        self.collect_diagnostics();
        Ok(result?)
    }
    pub fn dispatch_clipboard_paste(
        &mut self,
        plan: &ClipboardPlan,
        facts: PasteDispatchFactsV1,
    ) -> Result<CopyDispatchV1, EditorError> {
        self.live()?;
        let task = self.clipboard_task(plan)?;
        let result = self.host.dispatch_paste(&task, facts);
        self.collect_diagnostics();
        Ok(result?)
    }
    pub fn release_clipboard_export(
        &mut self,
        provider: &CopyProviderPortV1,
        dispatch: &CopyDispatchV1,
        facts: CopyReleaseFactsV1,
    ) -> Result<String, EditorError> {
        self.live()?;
        Ok(self.host.release_copy_plain(provider, dispatch, facts)?)
    }
    pub fn clipboard_export_evidence(
        &mut self,
        provider: &CopyProviderPortV1,
        dispatch: &CopyDispatchV1,
        evidence: CopyEvidenceV1,
        facts: CopyEvidenceFactsV1,
    ) -> Result<CopyEvidenceDispositionV1, EditorError> {
        self.live()?;
        let result = self
            .host
            .record_copy_evidence(provider, dispatch, evidence, facts);
        self.collect_diagnostics();
        Ok(result?)
    }
    pub fn clipboard_paste_evidence(
        &mut self,
        provider: &CopyProviderPortV1,
        dispatch: &CopyDispatchV1,
        evidence: PasteEvidenceV1<'_>,
        facts: PasteEvidenceFactsV1,
    ) -> Result<CopyEvidenceDispositionV1, EditorError> {
        self.live()?;
        let result = self
            .host
            .record_paste_evidence(provider, dispatch, evidence, facts);
        self.collect_diagnostics();
        Ok(result?)
    }
    pub fn clipboard_report(
        &mut self,
        plan: &ClipboardPlan,
    ) -> Result<ClipboardReport, EditorError> {
        self.live()?;
        let index = self.clipboard_index(plan)?;
        let a = &self.clipboard_attempts[index];
        let task = a.task.clone().ok_or(EditorError::ClipboardPending)?;
        let (kind, target_is_active) = (a.kind, a.tab == self.active);
        let (external, local, copied_not_cut) = match kind {
            TextTransferKindV1::Copy => (self.host.copy_status(&task)?, None, false),
            TextTransferKindV1::Cut => {
                let r = self.host.cut_report(&task)?;
                (r.external, Some(r.local), r.copied_not_cut)
            }
            TextTransferKindV1::Paste => {
                let r = self.host.paste_report(&task)?;
                (r.external, Some(r.local), false)
            }
        };
        Ok(ClipboardReport {
            external,
            local,
            copied_not_cut,
            target_is_active,
            fresh_local_decision: false,
            view_refresh_failed: false,
        })
    }
    pub fn complete_cut(
        &mut self,
        plan: &ClipboardPlan,
        facts: CutPublicationFactsV1,
    ) -> Result<ClipboardReport, EditorError> {
        self.complete_clipboard(plan, Some(facts), None)
    }
    pub fn complete_paste(
        &mut self,
        plan: &ClipboardPlan,
        facts: PastePublicationFactsV1,
    ) -> Result<ClipboardReport, EditorError> {
        self.complete_clipboard(plan, None, Some(facts))
    }
    fn complete_clipboard(
        &mut self,
        plan: &ClipboardPlan,
        cut: Option<CutPublicationFactsV1>,
        paste: Option<PastePublicationFactsV1>,
    ) -> Result<ClipboardReport, EditorError> {
        self.live()?;
        let index = self.clipboard_index(plan)?;
        let task = self.clipboard_task(plan)?;
        if !matches!(
            (self.clipboard_attempts[index].kind, cut, paste),
            (TextTransferKindV1::Cut, Some(_), None) | (TextTransferKindV1::Paste, None, Some(_))
        ) {
            return Err(EditorError::Position);
        }
        if !self.clipboard_attempts[index].history_recorded && self.groups.try_reserve(1).is_err() {
            // UI bookkeeping readiness is not an engine write grant. Explicitly
            // stop this owned intent; restoring memory must not retry it later.
            self.host.stop_copy(&task)?;
            return Err(EditorError::Capacity);
        }
        let local = match (cut, paste) {
            (Some(facts), None) => self.host.continue_cut(&task, facts),
            (None, Some(facts)) => self.host.continue_paste(&task, facts),
            _ => unreachable!("validated presentation operation"),
        };
        self.collect_diagnostics();
        let local = local?;
        let fresh = !self.clipboard_attempts[index].history_recorded
            && !matches!(
                local,
                TransferLocalDispositionV1::Pending | TransferLocalDispositionV1::Unknown
            );
        if !matches!(
            local,
            TransferLocalDispositionV1::Pending | TransferLocalDispositionV1::Unknown
        ) && !self.clipboard_attempts[index].history_recorded
        {
            if let Ok(InvocationOutcome::Committed(outcome)) =
                self.host.invocation_outcome(&plan.origin)
                && let Some(group) = &outcome.history_group
            {
                self.groups.push(group.clone());
            }
            self.clipboard_attempts[index].history_recorded = true;
        }
        let mut report = self.clipboard_report(plan)?;
        report.fresh_local_decision = fresh;
        if fresh && report.target_is_active && local == TransferLocalDispositionV1::Applied {
            report.view_refresh_failed = self.clamp_cursor().is_err();
        }
        self.status = if report.view_refresh_failed {
            "Clipboard local decision recorded; presentation refresh failed"
        } else if report.copied_not_cut {
            "Clipboard export accepted; source not cut"
        } else {
            "Clipboard local decision recorded; provider evidence remains separate"
        }
        .into();
        Ok(report)
    }
    pub fn stop_clipboard(&mut self, plan: &ClipboardPlan) -> Result<(), EditorError> {
        self.live()?;
        let task = self.clipboard_task(plan)?;
        Ok(self.host.stop_copy(&task)?)
    }
    pub fn retire_clipboard(
        &mut self,
        plan: &ClipboardPlan,
        facts: CopyRetirementFactsV1,
    ) -> Result<(), EditorError> {
        self.live()?;
        let index = self.clipboard_index(plan)?;
        let task = self.clipboard_task(plan)?;
        self.host.retire_copy(&task, facts)?;
        self.clipboard_attempts.swap_remove(index);
        self.collect_diagnostics();
        Ok(())
    }
    /// Retry control retirement only for admissions which never returned a task.
    /// No retry of provider work, source execution or an original task is implied.
    pub fn retire_unadmitted_clipboard_controls(&mut self) -> Result<(), EditorError> {
        self.live()?;
        let mut i = 0;
        while i < self.clipboard_attempts.len() {
            if self.clipboard_attempts[i].task.is_none() {
                self.host
                    .retire_invocation(&self.clipboard_attempts[i].origin)?;
                self.clipboard_attempts.swap_remove(i);
            } else {
                i += 1;
            }
        }
        Ok(())
    }
    /// Disabled is None, not zero utilization or a qualified provider profile.
    pub fn clipboard_usage(&self) -> Option<CopyUsageV1> {
        self.host.copy_usage().ok()
    }
}
