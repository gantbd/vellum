//! Explicit reference-app workflow; not generic SDK authority or a new publisher.
use super::*;
use vellum_session::host::TextTransferCaptureLimitsV1;

/// Current embedding premises at a direct source gesture. Default denies all.
#[derive(Clone, Copy, Debug, Default)]
pub struct DesktopAccess {
    pub control: bool,
    pub source_read: bool,
    pub source_edit: bool,
    pub clipboard_read: bool,
    pub clipboard_write: bool,
    pub retention: bool,
}
#[derive(Debug)]
pub struct DesktopResult {
    pub report: ClipboardReport,
    pub provider: Option<ProviderReceiptSummary>,
    pub retained: bool,
    /// Cleanup failure cannot relabel an already applied local edit.
    pub cleanup_error: Option<IntegrationError>,
}
impl DesktopResult {
    pub fn notice(&self) -> String {
        let message = match self.provider {
            Some(ProviderReceiptSummary::Export(ExportOutcome::Unknown)) => {
                "Clipboard export uncertain; operation retained, no automatic retry"
            }
            Some(ProviderReceiptSummary::Export(ExportOutcome::RejectedAfterClear)) => {
                "Export rejected; prior clipboard contents may have changed. Source not cut"
            }
            Some(ProviderReceiptSummary::Export(ExportOutcome::Refused(_))) => {
                "Clipboard export refused before I/O; source not cut"
            }
            Some(ProviderReceiptSummary::ReadNoData) => {
                "No UTF-8 plain text available; source unchanged"
            }
            Some(ProviderReceiptSummary::ReadRefused(_)) => {
                "Clipboard read refused; source unchanged"
            }
            _ if self.report.external.no_export => "No selected text; clipboard unchanged",
            _ if self.report.copied_not_cut => "Platform accepted copy; source not cut",
            _ => match self.report.external.operation {
                TextTransferKindV1::Copy => {
                    "Copy accepted by platform; not a durability or delivery guarantee"
                }
                TextTransferKindV1::Cut
                    if self.report.local == Some(TransferLocalDispositionV1::Applied) =>
                {
                    "Cut applied after platform acceptance"
                }
                TextTransferKindV1::Paste
                    if self.report.local == Some(TransferLocalDispositionV1::Applied) =>
                {
                    "Pasted literal plain text"
                }
                _ => "Clipboard operation completed without source publication",
            },
        };
        format!(
            "{message}{}",
            if self.cleanup_error.is_some() {
                "; cleanup pending"
            } else {
                ""
            }
        )
    }
}

struct Desktop<B> {
    binding: Option<Controller<B>>,
}
impl<B: Backend> Desktop<B> {
    fn perform(
        &mut self,
        editor: &mut Editor,
        operation: TextTransferKindV1,
        access: DesktopAccess,
        make: impl FnOnce() -> Result<(Provider<B>, CopyProviderRegistrationV1), Error>,
    ) -> Result<DesktopResult, IntegrationError> {
        if !access.control {
            return Err(IntegrationError::Unavailable);
        }
        if self.binding.is_none() {
            let (provider, registration) = make()?;
            self.binding = Some(Controller::register(editor, provider, registration)?);
        }
        let c = self.binding.as_mut().expect("bound");
        let admission = match operation {
            TextTransferKindV1::Copy => Admission::Copy(CopyAdmissionFactsV1 {
                read_allowed: Some(access.source_read),
                disclosure_allowed: Some(access.clipboard_write),
                retention_allowed: Some(access.retention),
            }),
            TextTransferKindV1::Cut => Admission::Cut(CutAdmissionFactsV1 {
                transfer: CopyAdmissionFactsV1 {
                    read_allowed: Some(access.source_read),
                    disclosure_allowed: Some(access.clipboard_write),
                    retention_allowed: Some(access.retention),
                },
                edit_allowed: Some(access.source_edit),
                view_write_allowed: Some(access.source_edit),
            }),
            TextTransferKindV1::Paste => Admission::Paste(PasteAdmissionFactsV1 {
                target_read_allowed: Some(access.source_read),
                edit_allowed: Some(access.source_edit),
                view_write_allowed: Some(access.source_edit),
                retention_allowed: Some(access.retention),
            }),
        };
        c.begin(editor, admission)?;
        let service = if operation == TextTransferKindV1::Paste {
            c.read(
                editor,
                PasteDispatchFactsV1 {
                    launch_allowed: Some(access.clipboard_read),
                    clipboard_read_allowed: Some(access.clipboard_read),
                },
            )
        } else {
            c.export(
                editor,
                CopyDispatchFactsV1 {
                    launch_allowed: Some(access.clipboard_write),
                    disclosure_allowed: Some(access.clipboard_write),
                },
                CopyReleaseFactsV1 {
                    release_allowed: Some(access.source_read),
                    disclosure_allowed: Some(access.clipboard_write),
                },
            )
        };
        if let Err(error) = service {
            // These stages prove no kernel I/O ran. Stop before attesting only
            // this canceled original operation. Never do this for Running.
            if matches!(
                c.stage(),
                ClipboardStage::Admitted | ClipboardStage::Dispatched
            ) {
                c.stop(editor)?;
                if c.stage() == ClipboardStage::Dispatched {
                    c.record_export_evidence(editor, CopyEvidenceV1::NotApplied, evidence(access))?;
                    c.record_export_evidence(editor, CopyEvidenceV1::Quiescent, evidence(access))?;
                }
                c.retire(editor, retirement())?;
            }
            return Err(error);
        }
        self.resume(editor, access)
    }
    fn resume(
        &mut self,
        editor: &mut Editor,
        access: DesktopAccess,
    ) -> Result<DesktopResult, IntegrationError> {
        let c = self.binding.as_mut().ok_or(IntegrationError::Unavailable)?;
        c.deliver(
            editor,
            PasteEvidenceFactsV1 {
                evidence: evidence(access),
                payload_retention_allowed: Some(access.retention),
            },
        )?;
        let mut report = c.report(editor)?;
        let provider = c.receipt();
        if matches!(
            report.external.effect,
            CopyEffectV1::Unknown | CopyEffectV1::Contested
        ) {
            return Ok(DesktopResult {
                report,
                provider,
                retained: true,
                cleanup_error: None,
            });
        }
        if report.external.effect == CopyEffectV1::NotApplied && !report.external.no_export {
            c.stop(editor)?;
            report = c.report(editor)?;
        } else {
            report = c.complete(
                editor,
                match report.external.operation {
                    TextTransferKindV1::Copy => Publication::Copy,
                    TextTransferKindV1::Cut => Publication::Cut(CutPublicationFactsV1 {
                        edit_allowed: Some(access.source_edit),
                        view_write_allowed: Some(access.source_edit),
                    }),
                    TextTransferKindV1::Paste => Publication::Paste(PastePublicationFactsV1 {
                        edit_allowed: Some(access.source_edit),
                        view_write_allowed: Some(access.source_edit),
                        payload_use_allowed: Some(access.retention),
                    }),
                },
            )?;
        }
        let cleanup_error = c.retire(editor, retirement()).err();
        Ok(DesktopResult {
            report,
            provider,
            retained: cleanup_error.is_some(),
            cleanup_error,
        })
    }
    fn close(&mut self, editor: &mut Editor, discard: bool) -> Result<(), IntegrationError> {
        if self.binding.as_ref().is_some_and(|c| c.pending.is_some()) {
            return Err(IntegrationError::Busy);
        }
        if !discard && editor.has_unsaved_documents()? {
            return Err(EditorError::Unsaved.into());
        }
        if let Some(c) = self.binding.as_mut() {
            c.close(editor, retirement())?;
        }
        self.binding = None;
        Ok(editor.close(discard)?)
    }
}
fn evidence(access: DesktopAccess) -> CopyEvidenceFactsV1 {
    CopyEvidenceFactsV1 {
        authentic: Some(true),
        evidence_allowed: Some(access.control),
        ingress_ready: Some(true),
    }
}
fn retirement() -> CopyRetirementFactsV1 {
    // Synchronous provider, delivered receipt, no callbacks/session fragments;
    // native retirement independently verifies settlement and local outcome.
    CopyRetirementFactsV1 {
        retention_released: Some(true),
        delivery_drained: Some(true),
        session_references_released: Some(true),
    }
}

/// Shared thin reference workflow. Generic SDK callers retain phase-specific APIs.
/// Construction configures bounds only; AppKit is bound lazily at a gesture.
pub struct ReferenceClipboard {
    desktop: Desktop<crate::appkit::AppKit>,
}
impl ReferenceClipboard {
    pub fn new(editor: &mut Editor) -> Result<Self, IntegrationError> {
        editor.configure_clipboard(
            CopyLimitsV1 {
                max_provider_registrations: 64,
                max_profile_bytes: 64,
                max_tasks: 1,
                max_held_bytes: 1048576,
                capture: TextTransferCaptureLimitsV1 {
                    max_fragments: 16,
                    max_plain_bytes: 65536,
                    max_logical_bytes: 262144,
                },
            },
            Some(PasteLimitsV1 {
                max_targets: 16,
                max_plain_bytes: 65536,
                max_expanded_bytes: 262144,
            }),
        )?;
        Ok(Self {
            desktop: Desktop { binding: None },
        })
    }
    pub fn perform(
        &mut self,
        editor: &mut Editor,
        operation: TextTransferKindV1,
        access: DesktopAccess,
    ) -> Result<DesktopResult, IntegrationError> {
        self.desktop.perform(editor, operation, access, || {
            Ok((
                Provider::new(
                    crate::appkit::AppKit::new()?,
                    Limits {
                        max_plain_bytes: 65536,
                    },
                    true,
                )?,
                CopyProviderRegistrationV1 {
                    destination: CopyDestinationV1::SystemClipboard,
                    readable: true,
                    writable: true,
                    acknowledgment_profile: ACKNOWLEDGMENT_PROFILE.into(),
                },
            ))
        })
    }
    /// Retry native receipt/local delivery only, never platform I/O.
    pub fn resume(
        &mut self,
        editor: &mut Editor,
        access: DesktopAccess,
    ) -> Result<DesktopResult, IntegrationError> {
        self.desktop.resume(editor, access)
    }
    pub fn close(&mut self, editor: &mut Editor, discard: bool) -> Result<(), IntegrationError> {
        self.desktop.close(editor, discard)
    }
    pub fn stage(&self) -> ClipboardStage {
        self.desktop
            .binding
            .as_ref()
            .map_or(ClipboardStage::Idle, Controller::stage)
    }
    pub fn usage(&self) -> Option<Usage> {
        self.desktop.binding.as_ref().map(|c| c.provider.usage)
    }
}

#[cfg(test)]
mod tests;
