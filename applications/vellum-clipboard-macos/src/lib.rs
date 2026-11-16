//! Original-operation clipboard orchestration over Editor.

#![forbid(unsafe_code)]
#![cfg(target_os = "macos")]

use vellum_adapter_clipboard_macos::ACKNOWLEDGMENT_PROFILE;
#[doc(hidden)]
pub use vellum_adapter_clipboard_macos::{AppKit, Backend, BackendWriteError, Provider};
pub use vellum_adapter_clipboard_macos::{Error, ExportOutcome, Limits, Usage};
use vellum_editor::{ClipboardPlan, ClipboardReport, Editor, EditorError};
pub use vellum_session::host::copy_transfer::TextTransferKindV1 as Operation;
use vellum_session::host::copy_transfer::*;
mod desktop;
#[cfg(feature = "qualification-harness")]
mod qualification;
pub use desktop::{DesktopAccess, DesktopResult, ReferenceClipboard};
#[cfg(feature = "qualification-harness")]
pub use qualification::{QualificationBackend, QualificationWriteError, qualify_backend};

/// Current trusted-host admission premises. Registration and identity alone
/// supply none of these permissions; the native facade validates each request.
#[derive(Clone, Copy, Debug)]
pub enum Admission {
    Copy(CopyAdmissionFactsV1),
    Cut(CutAdmissionFactsV1),
    Paste(PasteAdmissionFactsV1),
}
impl Admission {
    fn kind(self) -> TextTransferKindV1 {
        match self {
            Self::Copy(_) => TextTransferKindV1::Copy,
            Self::Cut(_) => TextTransferKindV1::Cut,
            Self::Paste(_) => TextTransferKindV1::Paste,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Publication {
    Copy,
    Cut(CutPublicationFactsV1),
    Paste(PastePublicationFactsV1),
}

/// Delivery stage only. Native `ClipboardReport` remains outcome truth.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipboardStage {
    Idle,
    Admitted,
    Dispatched,
    Running,
    ReceiptHeld,
    ReceiptDelivered,
    Closed,
}

/// Original provider result, not the local publication result. Payloads remain
/// private; a rejected export may have already cleared the previous clipboard.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderReceiptSummary {
    Export(ExportOutcome),
    ReadData { bytes: usize },
    ReadNoData,
    ReadRefused(Error),
}

#[derive(Debug)]
pub enum IntegrationError {
    Editor(EditorError),
    Provider(Error),
    Busy,
    WrongOperation,
    Unavailable,
}
impl From<EditorError> for IntegrationError {
    fn from(value: EditorError) -> Self {
        Self::Editor(value)
    }
}
impl From<Error> for IntegrationError {
    fn from(value: Error) -> Self {
        Self::Provider(value)
    }
}
impl std::fmt::Display for IntegrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "clipboard integration: {self:?}")
    }
}
impl std::error::Error for IntegrationError {}

enum Outcome {
    Export(ExportOutcome),
    Read(Result<Option<String>, Error>),
}
struct Receipt {
    dispatch: CopyDispatchV1,
    outcome: Outcome,
    delivered: bool,
    quiescent: bool,
}
enum Phase {
    Admitted,
    Dispatched(CopyDispatchV1),
    // Installed before provider code. Unwinding cannot make re-service legal.
    Running(CopyDispatchV1),
    Receipt(Receipt),
}
struct Pending {
    plan: ClipboardPlan,
    kind: TextTransferKindV1,
    phase: Option<Phase>,
}
struct Controller<B> {
    provider: Provider<B>,
    port: Option<CopyProviderPortV1>,
    pending: Option<Pending>,
}

impl<B: Backend> Controller<B> {
    fn register(
        editor: &mut Editor,
        provider: Provider<B>,
        registration: CopyProviderRegistrationV1,
    ) -> Result<Self, IntegrationError> {
        let port = editor.register_clipboard_provider(registration)?;
        Ok(Self {
            provider,
            port: Some(port),
            pending: None,
        })
    }
    fn stage(&self) -> ClipboardStage {
        if self.port.is_none() {
            return ClipboardStage::Closed;
        }
        match self.pending.as_ref().and_then(|p| p.phase.as_ref()) {
            None => ClipboardStage::Idle,
            Some(Phase::Admitted) => ClipboardStage::Admitted,
            Some(Phase::Dispatched(_)) => ClipboardStage::Dispatched,
            Some(Phase::Running(_)) => ClipboardStage::Running,
            Some(Phase::Receipt(r)) if r.delivered && r.quiescent => {
                ClipboardStage::ReceiptDelivered
            }
            Some(Phase::Receipt(_)) => ClipboardStage::ReceiptHeld,
        }
    }
    fn begin(&mut self, editor: &mut Editor, facts: Admission) -> Result<(), IntegrationError> {
        if self.pending.is_some() {
            return Err(IntegrationError::Busy);
        }
        let port = self.port.as_ref().ok_or(IntegrationError::Unavailable)?;
        let plan = match facts {
            Admission::Copy(facts) => editor.begin_copy(port, facts)?,
            Admission::Cut(facts) => editor.begin_cut(port, facts)?,
            Admission::Paste(facts) => editor.begin_paste(port, facts)?,
        };
        self.pending = Some(Pending {
            plan,
            kind: facts.kind(),
            phase: Some(Phase::Admitted),
        });
        Ok(())
    }
    fn receipt(&self) -> Option<ProviderReceiptSummary> {
        let Phase::Receipt(r) = self.pending.as_ref()?.phase.as_ref()? else {
            return None;
        };
        Some(match &r.outcome {
            Outcome::Export(outcome) => ProviderReceiptSummary::Export(*outcome),
            Outcome::Read(Ok(Some(text))) => ProviderReceiptSummary::ReadData { bytes: text.len() },
            Outcome::Read(Ok(None)) => ProviderReceiptSummary::ReadNoData,
            Outcome::Read(Err(error)) => ProviderReceiptSummary::ReadRefused(*error),
        })
    }
    fn export(
        &mut self,
        editor: &mut Editor,
        launch: CopyDispatchFactsV1,
        release: CopyReleaseFactsV1,
    ) -> Result<(), IntegrationError> {
        let port = self.port.as_ref().ok_or(IntegrationError::Unavailable)?;
        let p = self.pending.as_mut().ok_or(IntegrationError::Unavailable)?;
        if p.kind == TextTransferKindV1::Paste {
            return Err(IntegrationError::WrongOperation);
        }
        if editor.clipboard_report(&p.plan)?.external.no_export {
            return Ok(());
        }
        if matches!(p.phase, Some(Phase::Admitted)) {
            let dispatch = editor.dispatch_clipboard_export(&p.plan, launch)?;
            p.phase = Some(Phase::Dispatched(dispatch));
        }
        let Some(Phase::Dispatched(dispatch)) = p.phase.as_ref() else {
            return Err(IntegrationError::Busy);
        };
        // A failed current release retains the one original dispatch token.
        let payload = editor.release_clipboard_export(port, dispatch, release)?;
        let Some(Phase::Dispatched(dispatch)) = p.phase.take() else {
            unreachable!()
        };
        p.phase = Some(Phase::Running(dispatch));
        let outcome = self.provider.export(&payload);
        let Some(Phase::Running(dispatch)) = p.phase.take() else {
            unreachable!()
        };
        p.phase = Some(Phase::Receipt(Receipt {
            dispatch,
            outcome: Outcome::Export(outcome),
            delivered: false,
            quiescent: false,
        }));
        Ok(())
    }
    fn read(
        &mut self,
        editor: &mut Editor,
        launch: PasteDispatchFactsV1,
    ) -> Result<(), IntegrationError> {
        let p = self.pending.as_mut().ok_or(IntegrationError::Unavailable)?;
        if p.kind != TextTransferKindV1::Paste {
            return Err(IntegrationError::WrongOperation);
        }
        if !matches!(p.phase, Some(Phase::Admitted)) {
            return Err(IntegrationError::Busy);
        }
        let dispatch = editor.dispatch_clipboard_paste(&p.plan, launch)?;
        p.phase = Some(Phase::Running(dispatch));
        let outcome = self.provider.read();
        let Some(Phase::Running(dispatch)) = p.phase.take() else {
            unreachable!()
        };
        p.phase = Some(Phase::Receipt(Receipt {
            dispatch,
            outcome: Outcome::Read(outcome),
            delivered: false,
            quiescent: false,
        }));
        Ok(())
    }
    fn deliver(
        &mut self,
        editor: &mut Editor,
        facts: PasteEvidenceFactsV1,
    ) -> Result<(), IntegrationError> {
        // Validate original ownership even on delivery replay. No foreign
        // Editor may receive an apparent successful delivery from cached flags.
        let report = self.report(editor)?;
        if report.external.no_export {
            return Ok(());
        }
        let port = self.port.as_ref().ok_or(IntegrationError::Unavailable)?;
        let Some(Pending {
            phase: Some(Phase::Receipt(r)),
            ..
        }) = self.pending.as_mut()
        else {
            return Err(IntegrationError::Unavailable);
        };
        if !r.delivered {
            match &r.outcome {
                Outcome::Export(ExportOutcome::Accepted) => {
                    editor.clipboard_export_evidence(
                        port,
                        &r.dispatch,
                        CopyEvidenceV1::Accepted,
                        facts.evidence,
                    )?;
                }
                Outcome::Export(ExportOutcome::Refused(_) | ExportOutcome::RejectedAfterClear) => {
                    // The requested plain payload was not accepted. This is
                    // not a claim that prior clipboard contents survived;
                    // the original partial-effect caveat remains in receipt().
                    editor.clipboard_export_evidence(
                        port,
                        &r.dispatch,
                        CopyEvidenceV1::NotApplied,
                        facts.evidence,
                    )?;
                }
                // No terminal assertion: the original native effect stays Unknown.
                Outcome::Export(ExportOutcome::Unknown) => {}
                Outcome::Read(Ok(Some(text))) => {
                    editor.clipboard_paste_evidence(
                        port,
                        &r.dispatch,
                        PasteEvidenceV1::Accepted(text),
                        facts,
                    )?;
                }
                Outcome::Read(Ok(None) | Err(_)) => {
                    editor.clipboard_paste_evidence(
                        port,
                        &r.dispatch,
                        PasteEvidenceV1::NotApplied,
                        facts,
                    )?;
                }
            }
            r.delivered = true;
        }
        if !r.quiescent {
            // Only reached after the synchronous provider returned. It schedules
            // no callbacks; this does not resolve an unknown export effect.
            match r.outcome {
                Outcome::Export(_) => {
                    editor.clipboard_export_evidence(
                        port,
                        &r.dispatch,
                        CopyEvidenceV1::Quiescent,
                        facts.evidence,
                    )?;
                }
                Outcome::Read(_) => {
                    editor.clipboard_paste_evidence(
                        port,
                        &r.dispatch,
                        PasteEvidenceV1::Quiescent,
                        facts,
                    )?;
                }
            }
            r.quiescent = true;
        }
        Ok(())
    }
    fn report(&self, editor: &mut Editor) -> Result<ClipboardReport, IntegrationError> {
        Ok(editor.clipboard_report(
            &self
                .pending
                .as_ref()
                .ok_or(IntegrationError::Unavailable)?
                .plan,
        )?)
    }
    fn complete(
        &self,
        editor: &mut Editor,
        facts: Publication,
    ) -> Result<ClipboardReport, IntegrationError> {
        let p = self.pending.as_ref().ok_or(IntegrationError::Unavailable)?;
        match (p.kind, facts) {
            (TextTransferKindV1::Copy, Publication::Copy) => self.report(editor),
            (TextTransferKindV1::Cut, Publication::Cut(facts)) => {
                Ok(editor.complete_cut(&p.plan, facts)?)
            }
            (TextTransferKindV1::Paste, Publication::Paste(facts)) => {
                Ok(editor.complete_paste(&p.plan, facts)?)
            }
            _ => Err(IntegrationError::WrongOperation),
        }
    }
    fn stop(&self, editor: &mut Editor) -> Result<(), IntegrationError> {
        editor.stop_clipboard(
            &self
                .pending
                .as_ref()
                .ok_or(IntegrationError::Unavailable)?
                .plan,
        )?;
        Ok(())
    }
    fn retire(
        &mut self,
        editor: &mut Editor,
        facts: CopyRetirementFactsV1,
    ) -> Result<(), IntegrationError> {
        editor.retire_clipboard(
            &self
                .pending
                .as_ref()
                .ok_or(IntegrationError::Unavailable)?
                .plan,
            facts,
        )?;
        self.pending = None;
        Ok(())
    }
    fn record_export_evidence(
        &mut self,
        editor: &mut Editor,
        evidence: CopyEvidenceV1,
        facts: CopyEvidenceFactsV1,
    ) -> Result<CopyEvidenceDispositionV1, IntegrationError> {
        let port = self.port.as_ref().ok_or(IntegrationError::Unavailable)?;
        let p = self.pending.as_ref().ok_or(IntegrationError::Unavailable)?;
        if p.kind == TextTransferKindV1::Paste {
            return Err(IntegrationError::WrongOperation);
        }
        let dispatch = match p.phase.as_ref() {
            Some(Phase::Dispatched(d) | Phase::Running(d)) => d,
            Some(Phase::Receipt(r)) => &r.dispatch,
            _ => return Err(IntegrationError::Unavailable),
        };
        let result = editor.clipboard_export_evidence(port, dispatch, evidence, facts)?;
        if let Some(p) = self.pending.as_mut()
            && matches!(p.phase, Some(Phase::Dispatched(_)))
        {
            let Some(Phase::Dispatched(dispatch)) = p.phase.take() else {
                unreachable!()
            };
            p.phase = Some(Phase::Running(dispatch));
        }
        Ok(result)
    }
    fn close(
        &mut self,
        editor: &mut Editor,
        facts: CopyRetirementFactsV1,
    ) -> Result<(), IntegrationError> {
        if self.pending.is_some() {
            return Err(IntegrationError::Busy);
        }
        editor.retire_clipboard_provider(
            self.port.as_ref().ok_or(IntegrationError::Unavailable)?,
            facts,
        )?;
        self.port = None;
        Ok(())
    }
    fn revoke(&self, editor: &mut Editor) -> Result<(), IntegrationError> {
        editor
            .revoke_clipboard_provider(self.port.as_ref().ok_or(IntegrationError::Unavailable)?)?;
        Ok(())
    }
}

/// One pending original operation over the native Editor facade. No I/O during
/// binding or queries; no default authority premises or hidden retry loop.
/// Drop does not acknowledge work or discharge the Editor's native obligations.
pub struct EditorClipboard {
    controller: Controller<AppKit>,
}
impl std::fmt::Debug for EditorClipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EditorClipboard(<redacted>)")
    }
}
impl EditorClipboard {
    /// The host must explicitly configure native invocation and transfer limits
    /// on Editor first. AppKit acceptance is the fixed provider profile.
    pub fn bind(
        editor: &mut Editor,
        limits: Limits,
        timed: bool,
    ) -> Result<Self, IntegrationError> {
        let provider = Provider::new(AppKit::new()?, limits, timed)?;
        let controller = Controller::register(
            editor,
            provider,
            CopyProviderRegistrationV1 {
                destination: CopyDestinationV1::SystemClipboard,
                readable: true,
                writable: true,
                acknowledgment_profile: ACKNOWLEDGMENT_PROFILE.into(),
            },
        )?;
        Ok(Self { controller })
    }
    pub fn stage(&self) -> ClipboardStage {
        self.controller.stage()
    }
    pub fn usage(&self) -> Usage {
        self.controller.provider.usage
    }
    pub fn receipt(&self) -> Option<ProviderReceiptSummary> {
        self.controller.receipt()
    }
    pub fn begin(&mut self, editor: &mut Editor, facts: Admission) -> Result<(), IntegrationError> {
        self.controller.begin(editor, facts)
    }
    pub fn export(
        &mut self,
        editor: &mut Editor,
        launch: CopyDispatchFactsV1,
        release: CopyReleaseFactsV1,
    ) -> Result<(), IntegrationError> {
        objc2::rc::autoreleasepool(|_| self.controller.export(editor, launch, release))
    }
    pub fn read(
        &mut self,
        editor: &mut Editor,
        launch: PasteDispatchFactsV1,
    ) -> Result<(), IntegrationError> {
        objc2::rc::autoreleasepool(|_| self.controller.read(editor, launch))
    }
    pub fn deliver(
        &mut self,
        editor: &mut Editor,
        facts: PasteEvidenceFactsV1,
    ) -> Result<(), IntegrationError> {
        self.controller.deliver(editor, facts)
    }
    pub fn report(&self, editor: &mut Editor) -> Result<ClipboardReport, IntegrationError> {
        self.controller.report(editor)
    }
    pub fn complete(
        &self,
        editor: &mut Editor,
        facts: Publication,
    ) -> Result<ClipboardReport, IntegrationError> {
        self.controller.complete(editor, facts)
    }
    pub fn stop(&self, editor: &mut Editor) -> Result<(), IntegrationError> {
        self.controller.stop(editor)
    }
    pub fn retire(
        &mut self,
        editor: &mut Editor,
        facts: CopyRetirementFactsV1,
    ) -> Result<(), IntegrationError> {
        self.controller.retire(editor, facts)
    }
    /// Only independently authenticated, operation-attributed evidence belongs
    /// here. Reading today's clipboard is not evidence about a prior export.
    pub fn record_export_evidence(
        &mut self,
        editor: &mut Editor,
        evidence: CopyEvidenceV1,
        facts: CopyEvidenceFactsV1,
    ) -> Result<CopyEvidenceDispositionV1, IntegrationError> {
        self.controller
            .record_export_evidence(editor, evidence, facts)
    }
    pub fn close(
        &mut self,
        editor: &mut Editor,
        facts: CopyRetirementFactsV1,
    ) -> Result<(), IntegrationError> {
        self.controller.close(editor, facts)
    }
    /// Prevent new launches/releases without erasing outstanding receipts.
    pub fn revoke(&self, editor: &mut Editor) -> Result<(), IntegrationError> {
        self.controller.revoke(editor)
    }
}

#[cfg(test)]
mod tests;
