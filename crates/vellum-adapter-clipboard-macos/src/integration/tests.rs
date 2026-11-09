use super::*;
use crate::{Backend, Error, Limits, Provider};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use vellum_adapter_local_fs::{FilesystemLimits, LocalFilesystemFactory};
use vellum_engine::coordinator::InvocationLimits;
use vellum_model::{Action, Editor};
use vellum_session::host::TextTransferCaptureLimitsV1;

static NEXT: AtomicU64 = AtomicU64::new(0);
pub(crate) fn reference_profile() -> vellum_model::ReferenceSessionConfig {
    let mut config = vellum_model::reference_profile(
        LocalFilesystemFactory::new(FilesystemLimits {
            max_resources: 16,
            max_bytes: 65536,
            max_stage_attempts: 16,
            max_metadata_bytes: 65536,
        })
        .erase(),
        Box::new(vellum_adapter_annotation_json::JsonAnnotationCodec),
    );
    // Clipboard qualification exercises the largest admitted source and must
    // explicitly supply the invocation algebra that the generic model leaves
    // unconfigured. This is test composition, not adapter-owned policy.
    config.engine.invocation = Some(InvocationLimits {
        max_lifetime_issued: 64,
        max_retained: 8,
        max_intent_bytes: 262144,
    });
    config
}
pub(super) struct Fixture(pub(super) PathBuf);
impl Fixture {
    pub(super) fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "vellum-native-clip-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::write(path.join("a"), b"A\r\nB").unwrap();
        fs::write(path.join("b"), b"other").unwrap();
        Self(path)
    }
    pub(super) fn editor(&self, capture: usize) -> Editor {
        let mut config = reference_profile();
        config.capture_capacity = capture;
        let mut editor = Editor::open(&[self.0.join("a"), self.0.join("b")], config).unwrap();
        editor
            .configure_clipboard(
                CopyLimitsV1 {
                    max_provider_registrations: 2,
                    max_profile_bytes: 64,
                    max_tasks: 4,
                    max_held_bytes: 32768,
                    capture: TextTransferCaptureLimitsV1 {
                        max_fragments: 16,
                        max_plain_bytes: 128,
                        max_logical_bytes: 1024,
                    },
                },
                Some(PasteLimitsV1 {
                    max_targets: 16,
                    max_plain_bytes: 128,
                    max_expanded_bytes: 4096,
                }),
            )
            .unwrap();
        editor
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_file(self.0.join("a")).unwrap();
        fs::remove_file(self.0.join("b")).unwrap();
        fs::remove_dir(&self.0).unwrap();
    }
}
struct Fake {
    writes: usize,
    reads: usize,
    result: Result<bool, ()>,
    value: Option<String>,
    panic_write: bool,
}
impl Backend for Fake {
    fn write(&mut self, _: &str) -> Result<bool, ()> {
        self.writes += 1;
        assert!(!self.panic_write, "test provider interrupted");
        self.result
    }
    fn read(&mut self, _: usize) -> Result<Option<String>, Error> {
        self.reads += 1;
        Ok(self.value.clone())
    }
}
fn controller(editor: &mut Editor, value: Option<&str>) -> Controller<Fake> {
    let provider = Provider::new(
        Fake {
            writes: 0,
            reads: 0,
            result: Ok(true),
            value: value.map(str::to_owned),
            panic_write: false,
        },
        Limits {
            max_plain_bytes: 128,
        },
        false,
    )
    .unwrap();
    Controller::register(
        editor,
        provider,
        CopyProviderRegistrationV1 {
            destination: CopyDestinationV1::SessionClipboard,
            readable: true,
            writable: true,
            acknowledgment_profile: "test-controller/1".into(),
        },
    )
    .unwrap()
}
fn copy() -> Admission {
    Admission::Copy(CopyAdmissionFactsV1 {
        read_allowed: Some(true),
        disclosure_allowed: Some(true),
        retention_allowed: Some(true),
    })
}
fn cut() -> Admission {
    let Admission::Copy(transfer) = copy() else {
        unreachable!()
    };
    Admission::Cut(CutAdmissionFactsV1 {
        transfer,
        edit_allowed: Some(true),
        view_write_allowed: Some(true),
    })
}
fn paste() -> Admission {
    Admission::Paste(PasteAdmissionFactsV1 {
        target_read_allowed: Some(true),
        edit_allowed: Some(true),
        view_write_allowed: Some(true),
        retention_allowed: Some(true),
    })
}
fn launch() -> CopyDispatchFactsV1 {
    CopyDispatchFactsV1 {
        launch_allowed: Some(true),
        disclosure_allowed: Some(true),
    }
}
fn release() -> CopyReleaseFactsV1 {
    CopyReleaseFactsV1 {
        release_allowed: Some(true),
        disclosure_allowed: Some(true),
    }
}
fn ingress() -> PasteEvidenceFactsV1 {
    PasteEvidenceFactsV1 {
        evidence: CopyEvidenceFactsV1 {
            authentic: Some(true),
            evidence_allowed: Some(true),
            ingress_ready: Some(true),
        },
        payload_retention_allowed: Some(true),
    }
}
fn retirement() -> CopyRetirementFactsV1 {
    CopyRetirementFactsV1 {
        retention_released: Some(true),
        delivery_drained: Some(true),
        session_references_released: Some(true),
    }
}
fn cut_rights() -> Publication {
    Publication::Cut(CutPublicationFactsV1 {
        edit_allowed: Some(true),
        view_write_allowed: Some(true),
    })
}
fn paste_rights() -> Publication {
    Publication::Paste(PastePublicationFactsV1 {
        edit_allowed: Some(true),
        view_write_allowed: Some(true),
        payload_use_allowed: Some(true),
    })
}

#[test]
fn original_cut_receipt_retries_delivery_not_platform_work_or_local_publication() {
    for capture in [0, 1, 256] {
        let f = Fixture::new();
        let mut editor = f.editor(capture);
        let version = editor.version().unwrap();
        editor.set_selection(&version, 0, 1).unwrap();
        let mut service = controller(&mut editor, None);
        service.begin(&mut editor, cut()).unwrap();
        service.export(&mut editor, launch(), release()).unwrap();
        assert_eq!(service.provider.backend.writes, 1);
        assert!(service.export(&mut editor, launch(), release()).is_err());
        let mut denied = ingress();
        denied.evidence.evidence_allowed = Some(false);
        assert!(service.deliver(&mut editor, denied).is_err());
        assert_eq!(editor.snapshot().unwrap().text, "A\r\nB");
        service.deliver(&mut editor, ingress()).unwrap();
        service.deliver(&mut editor, ingress()).unwrap();
        let first = service.complete(&mut editor, cut_rights()).unwrap();
        assert_eq!(first.local, Some(TransferLocalDispositionV1::Applied));
        editor.apply(Action::Undo).unwrap();
        service.complete(&mut editor, cut_rights()).unwrap();
        assert_eq!(editor.snapshot().unwrap().text, "A\r\nB");
        assert_eq!(service.provider.backend.writes, 1);
        service.retire(&mut editor, retirement()).unwrap();
        service.close(&mut editor, retirement()).unwrap();
        editor.close(true).unwrap();
    }
}

#[test]
fn paste_keeps_original_background_target_and_one_read_across_ingress_refusal() {
    let f = Fixture::new();
    let mut editor = f.editor(1);
    let version = editor.version().unwrap();
    editor.set_selection(&version, 0, 1).unwrap();
    let mut service = controller(&mut editor, Some("\u{feff}Grüße\r\n\0"));
    service.begin(&mut editor, paste()).unwrap();
    editor.apply(Action::NextDocument).unwrap();
    service
        .read(
            &mut editor,
            PasteDispatchFactsV1 {
                launch_allowed: Some(true),
                clipboard_read_allowed: Some(true),
            },
        )
        .unwrap();
    let mut denied = ingress();
    denied.payload_retention_allowed = Some(false);
    assert!(service.deliver(&mut editor, denied).is_err());
    assert_eq!(service.provider.backend.reads, 1);
    service.deliver(&mut editor, ingress()).unwrap();
    let report = service.complete(&mut editor, paste_rights()).unwrap();
    assert!(!report.target_is_active);
    assert_eq!(editor.snapshot().unwrap().text, "other");
    editor.apply(Action::NextDocument).unwrap();
    assert_eq!(editor.snapshot().unwrap().text, "\u{feff}Grüße\r\n\0\r\nB");
    assert!(
        service
            .read(&mut editor, PasteDispatchFactsV1::default())
            .is_err()
    );
    assert_eq!(service.provider.backend.reads, 1);
    service.retire(&mut editor, retirement()).unwrap();
    service.close(&mut editor, retirement()).unwrap();
    editor.close(true).unwrap();
}

#[test]
fn unknown_export_is_retained_without_cut_or_fake_retirement() {
    let f = Fixture::new();
    let mut editor = f.editor(0);
    let version = editor.version().unwrap();
    editor.set_selection(&version, 0, 1).unwrap();
    let mut service = controller(&mut editor, None);
    service.provider.backend.result = Err(());
    service.begin(&mut editor, cut()).unwrap();
    service.export(&mut editor, launch(), release()).unwrap();
    service.deliver(&mut editor, ingress()).unwrap();
    assert_eq!(
        service.report(&mut editor).unwrap().external.effect,
        CopyEffectV1::Unknown
    );
    assert!(service.complete(&mut editor, cut_rights()).is_err());
    assert!(service.retire(&mut editor, retirement()).is_err());
    assert_eq!(editor.snapshot().unwrap().text, "A\r\nB");
    assert!(service.begin(&mut editor, copy()).is_err());
    assert_eq!(service.provider.backend.writes, 1);
    // A later explicitly attributed receipt, not current clipboard contents,
    // may reconcile this original operation through the same native gateway.
    service
        .record_export_evidence(&mut editor, CopyEvidenceV1::NotApplied, ingress().evidence)
        .unwrap();
    service.stop(&mut editor).unwrap();
    service.retire(&mut editor, retirement()).unwrap();
    service.close(&mut editor, retirement()).unwrap();
    editor.close(true).unwrap();
}

#[test]
fn empty_copy_never_clears_the_clipboard_and_discharge_does_not_need_fake_receipts() {
    let f = Fixture::new();
    let mut editor = f.editor(0);
    let mut service = controller(&mut editor, None);
    service.begin(&mut editor, copy()).unwrap();
    service.export(&mut editor, launch(), release()).unwrap();
    service
        .deliver(&mut editor, PasteEvidenceFactsV1::default())
        .unwrap();
    assert!(service.report(&mut editor).unwrap().external.no_export);
    assert_eq!(service.provider.backend.writes, 0);
    assert_eq!(service.stage(), ClipboardStage::Admitted);
    service.retire(&mut editor, retirement()).unwrap();
    service.close(&mut editor, retirement()).unwrap();
    editor.close(false).unwrap();
}

#[test]
fn release_refusal_retains_dispatch_without_io_and_foreign_editor_cannot_accept_delivery_replay() {
    let f = Fixture::new();
    let mut editor = f.editor(0);
    let mut foreign = f.editor(0);
    let version = editor.version().unwrap();
    editor.set_selection(&version, 0, 1).unwrap();
    let mut service = controller(&mut editor, None);
    service.begin(&mut editor, copy()).unwrap();
    assert!(
        service
            .export(&mut editor, launch(), CopyReleaseFactsV1::default())
            .is_err()
    );
    assert_eq!(service.stage(), ClipboardStage::Dispatched);
    assert_eq!(service.provider.backend.writes, 0);
    service.export(&mut editor, launch(), release()).unwrap();
    service.deliver(&mut editor, ingress()).unwrap();
    assert!(service.deliver(&mut foreign, ingress()).is_err());
    assert!(service.retire(&mut foreign, retirement()).is_err());
    assert_eq!(service.provider.backend.writes, 1);
    service.retire(&mut editor, retirement()).unwrap();
    service.close(&mut editor, retirement()).unwrap();
    editor.close(false).unwrap();
    foreign.close(false).unwrap();
}

#[test]
fn revocation_after_export_preserves_copied_not_cut_and_spends_local_publication() {
    let f = Fixture::new();
    let mut editor = f.editor(0);
    let version = editor.version().unwrap();
    editor.set_selection(&version, 0, 1).unwrap();
    let mut service = controller(&mut editor, None);
    service.begin(&mut editor, cut()).unwrap();
    service.export(&mut editor, launch(), release()).unwrap();
    service.deliver(&mut editor, ingress()).unwrap();
    service.revoke(&mut editor).unwrap();
    let report = service.complete(&mut editor, cut_rights()).unwrap();
    assert!(report.copied_not_cut);
    assert_eq!(report.local, Some(TransferLocalDispositionV1::NotApplied));
    assert_eq!(editor.snapshot().unwrap().text, "A\r\nB");
    service.complete(&mut editor, cut_rights()).unwrap();
    assert_eq!(service.provider.backend.writes, 1);
    service.retire(&mut editor, retirement()).unwrap();
    service.close(&mut editor, retirement()).unwrap();
    editor.close(false).unwrap();
}

#[test]
fn provider_unwind_stays_running_and_neither_stop_nor_retry_fabricates_quiescence() {
    let f = Fixture::new();
    let mut editor = f.editor(0);
    let version = editor.version().unwrap();
    editor.set_selection(&version, 0, 1).unwrap();
    let mut service = controller(&mut editor, None);
    service.provider.backend.panic_write = true;
    service.begin(&mut editor, copy()).unwrap();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| service.export(
            &mut editor,
            launch(),
            release()
        )))
        .is_err()
    );
    assert_eq!(service.stage(), ClipboardStage::Running);
    assert!(service.export(&mut editor, launch(), release()).is_err());
    service.stop(&mut editor).unwrap();
    assert!(!service.report(&mut editor).unwrap().external.quiescent);
    assert!(service.retire(&mut editor, retirement()).is_err());
    assert_eq!(service.provider.backend.writes, 1);
    // Explicit independent provider evidence is required to discharge this work.
    service
        .record_export_evidence(&mut editor, CopyEvidenceV1::NotApplied, ingress().evidence)
        .unwrap();
    service
        .record_export_evidence(&mut editor, CopyEvidenceV1::Quiescent, ingress().evidence)
        .unwrap();
    service.retire(&mut editor, retirement()).unwrap();
    service.close(&mut editor, retirement()).unwrap();
    editor.close(false).unwrap();
}

#[test]
fn rejected_export_preserves_platform_caveat_and_cannot_delete_source() {
    let f = Fixture::new();
    let mut editor = f.editor(0);
    let version = editor.version().unwrap();
    editor.set_selection(&version, 0, 1).unwrap();
    let mut service = controller(&mut editor, None);
    service.provider.backend.result = Ok(false);
    service.begin(&mut editor, cut()).unwrap();
    service.export(&mut editor, launch(), release()).unwrap();
    assert_eq!(
        service.receipt(),
        Some(ProviderReceiptSummary::Export(
            crate::ExportOutcome::RejectedAfterClear
        ))
    );
    service.deliver(&mut editor, ingress()).unwrap();
    assert_eq!(
        service.report(&mut editor).unwrap().external.effect,
        CopyEffectV1::NotApplied
    );
    assert!(service.complete(&mut editor, cut_rights()).is_err());
    assert_eq!(editor.snapshot().unwrap().text, "A\r\nB");
    service.stop(&mut editor).unwrap();
    service.retire(&mut editor, retirement()).unwrap();
    service.close(&mut editor, retirement()).unwrap();
    editor.close(false).unwrap();
}

#[test]
fn no_plain_data_does_not_turn_into_an_empty_paste_but_present_empty_text_is_literal_data() {
    for value in [None, Some("")] {
        let f = Fixture::new();
        let mut editor = f.editor(0);
        let version = editor.version().unwrap();
        editor.set_selection(&version, 0, 1).unwrap();
        let mut service = controller(&mut editor, value);
        service.begin(&mut editor, paste()).unwrap();
        service
            .read(
                &mut editor,
                PasteDispatchFactsV1 {
                    launch_allowed: Some(true),
                    clipboard_read_allowed: Some(true),
                },
            )
            .unwrap();
        service.deliver(&mut editor, ingress()).unwrap();
        if value.is_some() {
            assert_eq!(
                service.receipt(),
                Some(ProviderReceiptSummary::ReadData { bytes: 0 })
            );
            service.complete(&mut editor, paste_rights()).unwrap();
            assert_eq!(editor.snapshot().unwrap().text, "\r\nB");
        } else {
            assert_eq!(service.receipt(), Some(ProviderReceiptSummary::ReadNoData));
            assert!(service.complete(&mut editor, paste_rights()).is_err());
            assert_eq!(editor.snapshot().unwrap().text, "A\r\nB");
            service.stop(&mut editor).unwrap();
        }
        assert_eq!(service.provider.backend.reads, 1);
        service.retire(&mut editor, retirement()).unwrap();
        service.close(&mut editor, retirement()).unwrap();
        editor.close(true).unwrap();
    }
}

#[test]
fn external_evidence_permanently_prevents_later_platform_service_of_that_dispatch() {
    let f = Fixture::new();
    let mut editor = f.editor(0);
    let version = editor.version().unwrap();
    editor.set_selection(&version, 0, 1).unwrap();
    let mut service = controller(&mut editor, None);
    service.begin(&mut editor, copy()).unwrap();
    assert!(
        service
            .export(&mut editor, launch(), CopyReleaseFactsV1::default())
            .is_err()
    );
    service
        .record_export_evidence(&mut editor, CopyEvidenceV1::NotApplied, ingress().evidence)
        .unwrap();
    service
        .record_export_evidence(&mut editor, CopyEvidenceV1::Quiescent, ingress().evidence)
        .unwrap();
    assert!(service.export(&mut editor, launch(), release()).is_err());
    assert_eq!(service.provider.backend.writes, 0);
    service.retire(&mut editor, retirement()).unwrap();
    service.close(&mut editor, retirement()).unwrap();
    editor.close(false).unwrap();
}
