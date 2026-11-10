#![cfg(target_os = "macos")]
#[path = "support/mod.rs"]
mod support;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use support::reference_profile;
use vellum_editor::{Action, Editor, EditorError, ReferenceSessionConfig};
use vellum_engine::coordinator::InvocationLimits;
use vellum_session::host::{TextTransferCaptureLimitsV1, copy_transfer::*};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new(a: &str) -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "vellum-present-clip-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    fs::write(path.join("a"), a).unwrap();
                    fs::write(path.join("b"), "B\n").unwrap();
                    return Self(path);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("fixture: {e}"),
            }
        }
    }
    fn paths(&self) -> Vec<PathBuf> {
        vec![self.0.join("a"), self.0.join("b")]
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_file(self.0.join("a")).unwrap();
        fs::remove_file(self.0.join("b")).unwrap();
        fs::remove_dir(&self.0).unwrap();
    }
}
fn config(capacity: usize) -> ReferenceSessionConfig {
    let mut c = reference_profile();
    c.capture_capacity = capacity;
    c.engine.invocation = Some(InvocationLimits {
        max_lifetime_issued: 32,
        max_retained: 8,
        max_intent_bytes: 262144,
    });
    c
}
fn limits() -> CopyLimitsV1 {
    CopyLimitsV1 {
        max_provider_registrations: 2,
        max_profile_bytes: 64,
        max_tasks: 4,
        max_held_bytes: 32768,
        capture: TextTransferCaptureLimitsV1 {
            max_fragments: 16,
            max_plain_bytes: 4096,
            max_logical_bytes: 8192,
        },
    }
}
fn paste_limits() -> PasteLimitsV1 {
    PasteLimitsV1 {
        max_targets: 16,
        max_plain_bytes: 256,
        max_expanded_bytes: 4096,
    }
}
fn registration() -> CopyProviderRegistrationV1 {
    CopyProviderRegistrationV1 {
        destination: CopyDestinationV1::SessionClipboard,
        readable: true,
        writable: true,
        acknowledgment_profile: "test-presentation/1".into(),
    }
}
fn app(f: &Fixture, capacity: usize) -> (Editor, CopyProviderPortV1) {
    let mut a = Editor::open(&f.paths(), config(capacity)).unwrap();
    a.configure_clipboard(limits(), Some(paste_limits()))
        .unwrap();
    let p = a.register_clipboard_provider(registration()).unwrap();
    (a, p)
}
fn admission() -> CopyAdmissionFactsV1 {
    CopyAdmissionFactsV1 {
        read_allowed: Some(true),
        disclosure_allowed: Some(true),
        retention_allowed: Some(true),
    }
}
fn cut_admission() -> CutAdmissionFactsV1 {
    CutAdmissionFactsV1 {
        transfer: admission(),
        edit_allowed: Some(true),
        view_write_allowed: Some(true),
    }
}
fn paste_admission() -> PasteAdmissionFactsV1 {
    PasteAdmissionFactsV1 {
        target_read_allowed: Some(true),
        edit_allowed: Some(true),
        view_write_allowed: Some(true),
        retention_allowed: Some(true),
    }
}
fn launch() -> CopyDispatchFactsV1 {
    CopyDispatchFactsV1 {
        launch_allowed: Some(true),
        disclosure_allowed: Some(true),
    }
}
fn paste_launch() -> PasteDispatchFactsV1 {
    PasteDispatchFactsV1 {
        launch_allowed: Some(true),
        clipboard_read_allowed: Some(true),
    }
}
fn evidence() -> CopyEvidenceFactsV1 {
    CopyEvidenceFactsV1 {
        authentic: Some(true),
        evidence_allowed: Some(true),
        ingress_ready: Some(true),
    }
}
fn receipt() -> PasteEvidenceFactsV1 {
    PasteEvidenceFactsV1 {
        evidence: evidence(),
        payload_retention_allowed: Some(true),
    }
}
fn release() -> CopyReleaseFactsV1 {
    CopyReleaseFactsV1 {
        release_allowed: Some(true),
        disclosure_allowed: Some(true),
    }
}
fn publish() -> PastePublicationFactsV1 {
    PastePublicationFactsV1 {
        edit_allowed: Some(true),
        view_write_allowed: Some(true),
        payload_use_allowed: Some(true),
    }
}
fn cut_publish() -> CutPublicationFactsV1 {
    CutPublicationFactsV1 {
        edit_allowed: Some(true),
        view_write_allowed: Some(true),
    }
}
fn retire() -> CopyRetirementFactsV1 {
    CopyRetirementFactsV1 {
        retention_released: Some(true),
        delivery_drained: Some(true),
        session_references_released: Some(true),
    }
}

#[test]
fn late_paste_updates_original_background_tab_and_tracks_native_history_through_save_and_close() {
    for capacity in [0, 1, 256] {
        let f = Fixture::new("\u{feff}A\r\nGrüße");
        let (mut a, p) = app(&f, capacity);
        let v = a.version().unwrap();
        a.set_selection(&v, 0, 1).unwrap();
        let plan = a.begin_paste(&p, paste_admission()).unwrap();
        let retry = plan.clone();
        a.apply(Action::NextDocument).unwrap();
        a.apply(Action::Insert("!".into())).unwrap();
        let background = a.version().unwrap();
        let d = a.dispatch_clipboard_paste(&plan, paste_launch()).unwrap();
        a.clipboard_paste_evidence(&p, &d, PasteEvidenceV1::Accepted("ß\nX"), receipt())
            .unwrap();
        a.clipboard_paste_evidence(&p, &d, PasteEvidenceV1::Quiescent, receipt())
            .unwrap();
        let r = a.complete_paste(&plan, publish()).unwrap();
        assert_eq!(r.local, Some(TransferLocalDispositionV1::Applied));
        assert!(r.fresh_local_decision && !r.target_is_active);
        assert_eq!(a.active_document(), 1);
        assert_eq!(a.text().unwrap(), "!B\n");
        assert_eq!(a.version().unwrap(), background);
        assert!(
            !a.complete_paste(&retry, publish())
                .unwrap()
                .fresh_local_decision
        );
        a.retire_clipboard(&plan, retire()).unwrap();
        a.retire_clipboard_provider(&p, retire()).unwrap();
        a.apply(Action::NextDocument).unwrap();
        assert_eq!(a.text().unwrap(), "ß\nX\r\nGrüße");
        a.apply(Action::Undo).unwrap();
        assert_eq!(a.text().unwrap(), "A\r\nGrüße");
        a.apply(Action::Redo).unwrap();
        a.apply(Action::Save).unwrap();
        assert_eq!(
            fs::read(f.0.join("a")).unwrap(),
            "\u{feff}ß\nX\r\nGrüße".as_bytes()
        );
        assert_eq!(fs::read(f.0.join("b")).unwrap(), b"B\n");
        assert_eq!(a.clipboard_usage().unwrap().paste_applied, 1);
        a.close(true).unwrap();
        assert_eq!(a.native_view_usage().live, 0);
    }
}

#[test]
fn copy_on_locked_source_and_copied_not_cut_keep_source_save_state_and_spent_refusal_truthful() {
    let f = Fixture::new("\u{feff}A\r\nGrüße");
    let (mut a, p) = app(&f, 1);
    let v = a.version().unwrap();
    a.set_selection(&v, 0, 1).unwrap();
    a.apply(Action::ToggleEditingLock).unwrap();
    assert!(a.begin_cut(&p, cut_admission()).is_err());
    let copy = a.begin_copy(&p, admission()).unwrap();
    let d = a.dispatch_clipboard_export(&copy, launch()).unwrap();
    assert_eq!(a.release_clipboard_export(&p, &d, release()).unwrap(), "A");
    a.clipboard_export_evidence(&p, &d, CopyEvidenceV1::Accepted, evidence())
        .unwrap();
    a.clipboard_export_evidence(&p, &d, CopyEvidenceV1::Quiescent, evidence())
        .unwrap();
    assert!(a.clipboard_report(&copy).unwrap().local.is_none());
    assert!(!a.is_dirty().unwrap());
    a.retire_clipboard(&copy, retire()).unwrap();
    a.apply(Action::ToggleEditingLock).unwrap();
    let cut = a.begin_cut(&p, cut_admission()).unwrap();
    let d = a.dispatch_clipboard_export(&cut, launch()).unwrap();
    assert_eq!(a.release_clipboard_export(&p, &d, release()).unwrap(), "A");
    a.clipboard_export_evidence(&p, &d, CopyEvidenceV1::Accepted, evidence())
        .unwrap();
    a.apply(Action::ToggleEditingLock).unwrap();
    let r = a.complete_cut(&cut, cut_publish()).unwrap();
    assert!(r.copied_not_cut && r.fresh_local_decision);
    assert_eq!(r.local, Some(TransferLocalDispositionV1::NotApplied));
    a.apply(Action::ToggleEditingLock).unwrap();
    assert!(
        !a.complete_cut(&cut, cut_publish())
            .unwrap()
            .fresh_local_decision
    );
    assert_eq!(a.text().unwrap(), "A\r\nGrüße");
    assert!(!a.is_dirty().unwrap());
    a.clipboard_export_evidence(&p, &d, CopyEvidenceV1::Quiescent, evidence())
        .unwrap();
    a.retire_clipboard(&cut, retire()).unwrap();
    a.retire_clipboard_provider(&p, retire()).unwrap();
    a.close(false).unwrap();
}

#[test]
fn unknown_work_prevents_even_discard_close_and_stop_does_not_fabricate_acceptance() {
    let f = Fixture::new("A");
    let (mut a, p) = app(&f, 0);
    let v = a.version().unwrap();
    a.set_selection(&v, 0, 1).unwrap();
    let copy = a.begin_copy(&p, admission()).unwrap();
    let d = a.dispatch_clipboard_export(&copy, launch()).unwrap();
    a.release_clipboard_export(&p, &d, release()).unwrap();
    assert!(matches!(a.close(true), Err(EditorError::ClipboardPending)));
    assert_eq!(a.native_view_usage().live, 2);
    a.stop_clipboard(&copy).unwrap();
    a.clipboard_export_evidence(&p, &d, CopyEvidenceV1::Quiescent, evidence())
        .unwrap();
    assert_eq!(
        a.clipboard_report(&copy).unwrap().external.effect,
        CopyEffectV1::Unknown
    );
    assert!(a.retire_clipboard(&copy, retire()).is_err());
    assert!(matches!(a.close(false), Err(EditorError::ClipboardPending)));
    assert_eq!(a.version().unwrap(), v);
    a.clipboard_export_evidence(&p, &d, CopyEvidenceV1::Accepted, evidence())
        .unwrap();
    a.retire_clipboard(&copy, retire()).unwrap();
    assert!(matches!(a.close(false), Err(EditorError::ClipboardPending))); // provider control remains separate
    a.retire_clipboard_provider(&p, retire()).unwrap();
    a.close(false).unwrap();
}

#[test]
fn failed_admission_retires_original_control_without_resetting_capacity_or_accepting_foreign_plans()
{
    let f = Fixture::new("A");
    let mut cfg = config(0);
    cfg.engine.invocation.as_mut().unwrap().max_retained = 1;
    let mut a = Editor::open(&f.paths(), cfg).unwrap();
    let mut bounds = limits();
    bounds.max_tasks = 1;
    a.configure_clipboard(bounds, Some(paste_limits())).unwrap();
    let p = a.register_clipboard_provider(registration()).unwrap();
    let (mut b, foreign) = app(&f, 0);
    assert!(a.begin_copy(&p, CopyAdmissionFactsV1::default()).is_err());
    assert!(a.begin_copy(&foreign, admission()).is_err());
    let plan = a.begin_copy(&p, admission()).unwrap();
    assert!(a.begin_copy(&p, admission()).is_err());
    assert!(b.dispatch_clipboard_export(&plan, launch()).is_err());
    assert!(b.clipboard_report(&plan).is_err());
    assert_eq!(format!("{plan:?}"), "ClipboardPlan(<redacted>)");
    a.stop_clipboard(&plan).unwrap();
    a.retire_clipboard(&plan, retire()).unwrap();
    assert!(a.dispatch_clipboard_export(&plan, launch()).is_err());
    a.retire_unadmitted_clipboard_controls().unwrap();
    a.retire_clipboard_provider(&p, retire()).unwrap();
    b.retire_clipboard_provider(&foreign, retire()).unwrap();
    a.close(false).unwrap();
    b.close(false).unwrap();
}

#[test]
fn terminal_capable_facade_pastes_every_native_selection_and_closes_after_native_undo() {
    let f = Fixture::new("A A");
    let (mut a, p) = app(&f, 256);
    #[cfg(feature = "native-unicode")]
    a.enable_native_unicode(
        vellum_adapter_unicode_icu::IcuFactory::reference(),
        vellum_editor::NativeUnicodeProfile::reference(),
    )
    .unwrap();
    a.find("A").unwrap();
    a.select_matches().unwrap();
    let plan = a.begin_paste(&p, paste_admission()).unwrap();
    let d = a.dispatch_clipboard_paste(&plan, paste_launch()).unwrap();
    a.clipboard_paste_evidence(&p, &d, PasteEvidenceV1::Accepted("ß"), receipt())
        .unwrap();
    assert_eq!(
        a.complete_paste(&plan, publish()).unwrap().local,
        Some(TransferLocalDispositionV1::Applied)
    );
    assert_eq!(a.text().unwrap(), "ß ß");
    assert_eq!(
        a.native_selection()
            .unwrap()
            .unwrap()
            .selections
            .selections()
            .len(),
        2
    );
    a.apply(Action::Undo).unwrap();
    assert_eq!(a.text().unwrap(), "A A");
    assert!(
        !a.complete_paste(&plan, publish())
            .unwrap()
            .fresh_local_decision
    );
    assert_eq!(a.text().unwrap(), "A A");
    a.clipboard_paste_evidence(&p, &d, PasteEvidenceV1::Quiescent, receipt())
        .unwrap();
    a.retire_clipboard(&plan, retire()).unwrap();
    a.retire_clipboard_provider(&p, retire()).unwrap();
    a.close(true).unwrap();
}
