//! Generic qualification workflow. Platform bindings live in unpublished
//! repository-only runners so production packages need no patched SDK surface.
use super::*;
use std::{fs, path::PathBuf};
use vellum_editor::Action;

/// Explicit port for repository-only live qualification backends.
///
/// This is feature-gated and not a production provider-selection mechanism.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QualificationWriteError;

pub trait QualificationBackend {
    fn write(&mut self, text: &str) -> Result<bool, QualificationWriteError>;
    fn read(&mut self, max_bytes: usize) -> Result<Option<String>, Error>;
    fn clear(&mut self) -> Result<(), Error>;
    fn release(&mut self) -> Result<(), Error>;
}

impl<T: QualificationBackend> Backend for T {
    fn write(&mut self, text: &str) -> Result<bool, ()> {
        QualificationBackend::write(self, text).map_err(|_| ())
    }
    fn read(&mut self, max_bytes: usize) -> Result<Option<String>, Error> {
        QualificationBackend::read(self, max_bytes)
    }
}

const RAW: &str = "\u{feff}Grüße\r\n\0e\u{301}👩‍💻";

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        for suffix in 0..256 {
            let dir = std::env::temp_dir().join(format!(
                "vellum-private-pasteboard-{}-{suffix}",
                std::process::id()
            ));
            match fs::create_dir(&dir) {
                Ok(()) => {
                    let fixture = Self(dir);
                    // The file decoder owns one leading encoding BOM; the
                    // second is literal document content and must be copied.
                    fs::write(fixture.0.join("source"), format!("\u{feff}{RAW}"))?;
                    return Ok(fixture);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.into()),
            }
        }
        Err("temporary fixture names exhausted".into())
    }
    fn remove(self) -> Result<(), Box<dyn std::error::Error>> {
        fs::remove_file(self.0.join("source"))?;
        fs::remove_dir(&self.0)?;
        Ok(())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // Exact agent-created targets only; never recursive cleanup or globbing.
        let _ = fs::remove_file(self.0.join("source"));
        let _ = fs::remove_dir(&self.0);
    }
}

fn require(condition: bool, message: &'static str) -> Result<(), Box<dyn std::error::Error>> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
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

/// Runs the exact production-controller workflow against fresh caller-owned
/// backends. The caller remains responsible for operator approval and platform
/// confinement. Each backend is cleared and explicitly released.
pub fn qualify_backend<B: QualificationBackend>(
    mut make_backend: impl FnMut() -> Result<B, Error>,
    config: fn() -> vellum_editor::ReferenceSessionConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    for capture_capacity in [0, 1, 256] {
        let backend = make_backend().map_err(|_| "qualification backend unavailable")?;
        qualify_mode(capture_capacity, backend, config)?;
    }
    Ok(())
}

fn qualify_mode<B: QualificationBackend>(
    capture_capacity: usize,
    backend: B,
    config: fn() -> vellum_editor::ReferenceSessionConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut provider = Provider::new(
        backend,
        Limits {
            max_plain_bytes: 65536,
        },
        true,
    )
    .map_err(|_| "qualification provider limits refused")?;
    provider
        .backend
        .clear()
        .map_err(|_| "private clear failed")?;
    require(
        matches!(provider.read(), Ok(None)),
        "fresh private board returned plain data",
    )?;
    for raw in ["", RAW, "\u{feff}\u{feff}", "界", &"x".repeat(65536)] {
        require(
            provider.export(raw) == ExportOutcome::Accepted,
            "private export not accepted",
        )?;
        require(
            provider.read().ok().flatten().as_deref() == Some(raw),
            "private raw-text readback mismatch",
        )?;
    }
    require(
        provider.export(&"x".repeat(65537)) == ExportOutcome::Refused(Error::Limit),
        "oversized export not refused",
    )?;
    let mut provider = qualify_workflow(capture_capacity, provider, config())?;
    provider
        .backend
        .clear()
        .map_err(|_| "private final clear failed")?;
    provider
        .backend
        .release()
        .map_err(|_| "private server-resource release failed or uncertain")?;
    require(
        provider.read() == Err(Error::Unavailable),
        "released private target remained readable",
    )?;
    require(
        provider.backend.write("must not reach General").is_err(),
        "released private target did not refuse export",
    )?;
    Ok(())
}

// The exact manual workflow also runs against an in-memory port in ordinary
// tests. That verifies the probe's oracles, not real AppKit behavior.
fn qualify_workflow<B: Backend>(
    capture_capacity: usize,
    provider: Provider<B>,
    mut config: vellum_editor::ReferenceSessionConfig,
) -> Result<Provider<B>, Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    config.capture_capacity = capture_capacity;
    let mut editor = Editor::open(&[fixture.0.join("source")], config)?;
    require(
        editor.snapshot()?.text == RAW,
        "fixture source decoding mismatch",
    )?;
    // Configure the same transfer bounds as both apps, without binding General.
    let _bounds = ReferenceClipboard::new(&mut editor)?;
    let mut c = Controller::register(
        &mut editor,
        provider,
        CopyProviderRegistrationV1 {
            destination: CopyDestinationV1::SessionClipboard,
            readable: true,
            writable: true,
            acknowledgment_profile: "macos.appkit.private-qualification/1".into(),
        },
    )?;
    editor.apply(Action::SelectAll)?;
    c.begin(
        &mut editor,
        Admission::Cut(CutAdmissionFactsV1 {
            transfer: CopyAdmissionFactsV1 {
                read_allowed: Some(true),
                disclosure_allowed: Some(true),
                retention_allowed: Some(true),
            },
            edit_allowed: Some(true),
            view_write_allowed: Some(true),
        }),
    )?;
    c.export(
        &mut editor,
        CopyDispatchFactsV1 {
            launch_allowed: Some(true),
            disclosure_allowed: Some(true),
        },
        CopyReleaseFactsV1 {
            release_allowed: Some(true),
            disclosure_allowed: Some(true),
        },
    )?;
    require(
        c.receipt() == Some(ProviderReceiptSummary::Export(ExportOutcome::Accepted)),
        "native cut export not accepted",
    )?;
    let offers = c.provider.usage.export_offers;
    let mut denied = ingress();
    denied.evidence.evidence_allowed = Some(false);
    require(
        c.deliver(&mut editor, denied).is_err(),
        "denied receipt delivery accepted",
    )?;
    require(
        editor.snapshot()?.text == RAW,
        "source changed before receipt acceptance",
    )?;
    c.deliver(&mut editor, ingress())?;
    c.deliver(&mut editor, ingress())?;
    let rights = Publication::Cut(CutPublicationFactsV1 {
        edit_allowed: Some(true),
        view_write_allowed: Some(true),
    });
    require(
        c.complete(&mut editor, rights)?.local == Some(TransferLocalDispositionV1::Applied),
        "cut not applied",
    )?;
    require(
        editor.snapshot()?.text.is_empty(),
        "cut did not remove exact source",
    )?;
    editor.apply(Action::Undo)?;
    c.complete(&mut editor, rights)?;
    require(
        editor.snapshot()?.text == RAW,
        "cut replay repeated after undo",
    )?;
    require(
        c.provider.usage.export_offers == offers,
        "receipt replay repeated platform export",
    )?;
    c.retire(&mut editor, retirement())?;

    editor.set_caret(0)?;
    c.begin(
        &mut editor,
        Admission::Paste(PasteAdmissionFactsV1 {
            target_read_allowed: Some(true),
            edit_allowed: Some(true),
            view_write_allowed: Some(true),
            retention_allowed: Some(true),
        }),
    )?;
    c.read(
        &mut editor,
        PasteDispatchFactsV1 {
            launch_allowed: Some(true),
            clipboard_read_allowed: Some(true),
        },
    )?;
    require(
        c.receipt() == Some(ProviderReceiptSummary::ReadData { bytes: RAW.len() }),
        "native paste did not read original raw text",
    )?;
    let reads = c.provider.usage.read_offers;
    require(
        c.provider.export("newer private value") == ExportOutcome::Accepted,
        "later private export failed",
    )?;
    c.deliver(&mut editor, ingress())?;
    let rights = Publication::Paste(PastePublicationFactsV1 {
        edit_allowed: Some(true),
        view_write_allowed: Some(true),
        payload_use_allowed: Some(true),
    });
    require(
        c.complete(&mut editor, rights)?.local == Some(TransferLocalDispositionV1::Applied),
        "paste not applied",
    )?;
    require(
        editor.snapshot()?.text == RAW.repeat(2),
        "paste replaced retained data with current clipboard",
    )?;
    editor.apply(Action::Undo)?;
    c.complete(&mut editor, rights)?;
    require(
        editor.snapshot()?.text == RAW,
        "paste replay repeated after undo",
    )?;
    require(
        c.provider.usage.read_offers == reads,
        "receipt replay repeated platform read",
    )?;
    c.retire(&mut editor, retirement())?;
    c.close(&mut editor, retirement())?;
    require(
        c.provider.read().ok().flatten().as_deref() == Some("newer private value"),
        "undo overwrote later clipboard value",
    )?;
    let usage = editor
        .clipboard_usage()
        .ok_or("missing native clipboard usage")?;
    require(
        usage.held_tasks == 0 && usage.providers_retained == 0,
        "native transfer owners not discharged",
    )?;
    editor.close(false)?;
    require(
        fs::read(fixture.0.join("source"))? == format!("\u{feff}{RAW}").as_bytes(),
        "clipboard workflow wrote source file",
    )?;
    fixture.remove()?;
    Ok(c.provider)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Memory(Option<String>);
    impl QualificationBackend for Memory {
        fn write(&mut self, value: &str) -> Result<bool, QualificationWriteError> {
            self.0 = Some(value.to_owned());
            Ok(true)
        }
        fn read(&mut self, _: usize) -> Result<Option<String>, Error> {
            Ok(self.0.clone())
        }
        fn clear(&mut self) -> Result<(), Error> {
            self.0 = None;
            Ok(())
        }
        fn release(&mut self) -> Result<(), Error> {
            self.0 = None;
            Ok(())
        }
    }

    #[test]
    fn manual_probe_workflow_and_cleanup_oracles_execute_without_any_pasteboard() {
        for capacity in [0, 1, 256] {
            let provider = Provider::new(
                Memory::default(),
                Limits {
                    max_plain_bytes: 65536,
                },
                false,
            )
            .unwrap();
            let provider = qualify_workflow(
                capacity,
                provider,
                crate::integration::tests::reference_profile(),
            )
            .unwrap();
            assert_eq!(provider.usage.export_offers, 2);
            assert_eq!(provider.usage.read_offers, 2);
            assert_eq!(provider.usage.export_service_total, None);
        }
    }
}
