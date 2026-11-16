use super::*;
use crate::tests::Fixture;
use vellum_editor::Action;

struct Fake {
    reads: usize,
    writes: usize,
    result: Result<bool, BackendWriteError>,
    value: Option<String>,
}
impl Backend for Fake {
    fn write(&mut self, _: &str) -> Result<bool, BackendWriteError> {
        self.writes += 1;
        self.result
    }
    fn read(&mut self, _: usize) -> Result<Option<String>, Error> {
        self.reads += 1;
        Ok(self.value.clone())
    }
}
fn make(
    value: Option<&str>,
    result: Result<bool, BackendWriteError>,
) -> Result<(Provider<Fake>, CopyProviderRegistrationV1), Error> {
    Ok((
        Provider::new(
            Fake {
                reads: 0,
                writes: 0,
                result,
                value: value.map(str::to_owned),
            },
            Limits {
                max_plain_bytes: 128,
            },
            false,
        )?,
        CopyProviderRegistrationV1 {
            destination: CopyDestinationV1::SessionClipboard,
            readable: true,
            writable: true,
            acknowledgment_profile: "test-reference-driver/1".into(),
        },
    ))
}
fn access() -> DesktopAccess {
    DesktopAccess {
        control: true,
        source_read: true,
        source_edit: true,
        clipboard_read: true,
        clipboard_write: true,
        retention: true,
    }
}

#[test]
fn reference_driver_reuses_binding_cleans_tasks_and_cannot_retire_on_canceled_dirty_close() {
    let f = Fixture::new();
    let mut editor = f.editor(1);
    let mut d = Desktop { binding: None };
    let version = editor.version().unwrap();
    editor.set_selection(&version, 0, 1).unwrap();
    let copy = d
        .perform(&mut editor, TextTransferKindV1::Copy, access(), || {
            make(Some("X"), Ok(true))
        })
        .unwrap();
    assert!(!copy.retained);
    assert_eq!(d.binding.as_ref().unwrap().stage(), ClipboardStage::Idle);
    let cut = d
        .perform(&mut editor, TextTransferKindV1::Cut, access(), || {
            panic!("binding must be reused")
        })
        .unwrap();
    assert_eq!(cut.report.local, Some(TransferLocalDispositionV1::Applied));
    assert_eq!(editor.snapshot().unwrap().text, "\r\nB");
    editor.apply(Action::NextDocument).unwrap();
    let active = editor.active_document();
    assert!(matches!(
        d.close(&mut editor, false),
        Err(IntegrationError::Editor(EditorError::Unsaved))
    ));
    assert_eq!(editor.active_document(), active);
    assert_eq!(editor.clipboard_usage().unwrap().providers_retained, 1);
    assert_eq!(d.binding.as_ref().unwrap().provider.backend.writes, 2);
    editor.apply(Action::NextDocument).unwrap();
    editor.apply(Action::Undo).unwrap();
    editor.set_caret(0).unwrap();
    let paste = d
        .perform(&mut editor, TextTransferKindV1::Paste, access(), || {
            panic!("binding must be reused")
        })
        .unwrap();
    assert_eq!(
        paste.report.local,
        Some(TransferLocalDispositionV1::Applied)
    );
    assert_eq!(editor.snapshot().unwrap().text, "XA\r\nB");
    assert_eq!(d.binding.as_ref().unwrap().provider.backend.reads, 1);
    assert_eq!(editor.clipboard_usage().unwrap().held_tasks, 0);
    d.close(&mut editor, true).unwrap();
    assert!(editor.is_closed());
}

#[test]
fn missing_data_and_dispatch_denial_leave_no_orphaned_task() {
    let f = Fixture::new();
    let mut editor = f.editor(0);
    let mut d = Desktop { binding: None };
    let version = editor.version().unwrap();
    editor.set_selection(&version, 0, 1).unwrap();
    let r = d
        .perform(&mut editor, TextTransferKindV1::Paste, access(), || {
            make(None, Ok(true))
        })
        .unwrap();
    assert_eq!(r.provider, Some(ProviderReceiptSummary::ReadNoData));
    assert!(!r.retained);
    assert_eq!(editor.snapshot().unwrap().text, "A\r\nB");
    let mut denied = access();
    denied.clipboard_read = false;
    assert!(
        d.perform(&mut editor, TextTransferKindV1::Paste, denied, || panic!(
            "no rebinding"
        ))
        .is_err()
    );
    assert_eq!(editor.clipboard_usage().unwrap().held_tasks, 0);
    assert_eq!(d.binding.as_ref().unwrap().provider.backend.reads, 1);
    d.close(&mut editor, false).unwrap();
}

#[test]
fn default_authority_never_even_constructs_a_provider() {
    let f = Fixture::new();
    let mut editor = f.editor(0);
    let mut d: Desktop<Fake> = Desktop { binding: None };
    assert!(
        d.perform(
            &mut editor,
            TextTransferKindV1::Copy,
            DesktopAccess::default(),
            || panic!("not authorized")
        )
        .is_err()
    );
    assert_eq!(editor.clipboard_usage().unwrap().providers_retained, 0);
    d.close(&mut editor, false).unwrap();
}

#[test]
fn unknown_result_blocks_new_gestures_and_even_discard_close_without_new_io() {
    let f = Fixture::new();
    let mut editor = f.editor(0);
    let mut d = Desktop { binding: None };
    let version = editor.version().unwrap();
    editor.set_selection(&version, 0, 1).unwrap();
    let r = d
        .perform(&mut editor, TextTransferKindV1::Cut, access(), || {
            make(None, Err(BackendWriteError))
        })
        .unwrap();
    assert!(r.retained);
    assert!(r.notice().contains("uncertain"));
    assert!(
        d.perform(&mut editor, TextTransferKindV1::Copy, access(), || panic!(
            "must not rebind"
        ))
        .is_err()
    );
    assert!(d.close(&mut editor, true).is_err());
    let c = d.binding.as_mut().unwrap();
    assert_eq!(c.provider.backend.writes, 1);
    c.record_export_evidence(&mut editor, CopyEvidenceV1::NotApplied, evidence(access()))
        .unwrap();
    c.stop(&mut editor).unwrap();
    c.retire(&mut editor, retirement()).unwrap();
    d.close(&mut editor, false).unwrap();
}

#[test]
fn reference_limits_allow_copying_the_entire_supported_64k_document() {
    let f = Fixture::new();
    std::fs::write(f.0.join("a"), "x".repeat(65536)).unwrap();
    let mut editor = Editor::open(&[f.0.join("a")], crate::tests::reference_profile()).unwrap();
    let _configured = ReferenceClipboard::new(&mut editor).unwrap();
    editor.apply(Action::SelectAll).unwrap();
    let mut d = Desktop { binding: None };
    let result = d.perform(&mut editor, TextTransferKindV1::Copy, access(), || {
        let (mut provider, registration) = make(None, Ok(true))?;
        provider.limits = Limits {
            max_plain_bytes: 65536,
        };
        Ok((provider, registration))
    });
    assert!(
        result.is_ok(),
        "full-size reference copy refused: {result:?}"
    );
    assert_eq!(d.binding.as_ref().unwrap().provider.backend.writes, 1);
    d.close(&mut editor, false).unwrap();
}
