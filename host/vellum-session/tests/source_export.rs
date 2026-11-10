#![cfg(target_os = "macos")]
#[path = "../examples/support/mod.rs"]
mod support;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use support::{LocalDocument, LocalHost, SessionConfig};
use vellum_engine::coordinator::{
    TrustedSourceExportControlV1, TrustedSourceExportFactsV1, TrustedSourceSaveAsFactsV1,
};
use vellum_engine::resource::{
    Acknowledgment, Application, ContentComparison, Protection, SaveRequirements,
    SourceExportPurposeV1, Synchronization,
};
use vellum_engine::source::Replacement;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        loop {
            let p = std::env::temp_dir().join(format!(
                "vellum-sdk-export-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&p) {
                Ok(()) => {
                    fs::write(p.join("source"), "\u{feff}A\r\n".as_bytes()).unwrap();
                    for name in ["copy", "as"] {
                        fs::write(p.join(name), b"OLD").unwrap();
                    }
                    return Self(p);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("fixture: {e}"),
            }
        }
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for name in ["source", "copy", "as"] {
            fs::remove_file(self.path(name)).unwrap();
        }
        fs::remove_dir(&self.0).unwrap();
    }
}
fn config(capacity: usize) -> SessionConfig {
    let mut c = support::config(capacity, true);
    c.resource_adapter.limits_mut().max_resources = 3;
    c
}
fn requirements() -> SaveRequirements {
    SaveRequirements {
        protection: Protection::BestEffort,
        acknowledgment: Acknowledgment::Synchronized,
    }
}
// Actual scripted authority for these exclusively created test fixtures. No
// destination/currentness/receipt is fabricated: those come from the real adapter.
fn facts() -> TrustedSourceExportFactsV1 {
    TrustedSourceExportFactsV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        source_readable: Some(true),
        write_allowed: Some(true),
        binding_coverage_complete: Some(true),
        writer_coverage_complete: Some(true),
        provider_available: Some(true),
        resources_ready: Some(true),
        launch_allowed: Some(true),
        destination_allowed: Some(true),
        identity_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        create_allowed: None,
        overwrite_allowed: Some(true),
    }
}
fn control() -> TrustedSourceExportControlV1 {
    TrustedSourceExportControlV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        binding_coverage_complete: Some(true),
        writer_coverage_complete: Some(true),
        binding_activation_allowed: Some(true),
        identity_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        disposition_readable: Some(true),
        reservation_release_allowed: Some(true),
    }
}
fn activation() -> TrustedSourceSaveAsFactsV1 {
    TrustedSourceSaveAsFactsV1 {
        control: control(),
        probe_attested: Some(true),
        resource_continuity_attested: Some(true),
        create_allowed: None,
        overwrite_allowed: Some(true),
    }
}
fn edited(host: &mut LocalHost, f: &Fixture) -> LocalDocument {
    let doc = host.open(&f.path("source")).unwrap();
    let version = host.read(&doc).unwrap().version;
    host.edit(
        &version,
        vec![Replacement {
            start: 0,
            end: 1,
            expected: "A".into(),
            inserted: "Ä".into(),
        }],
    )
    .unwrap();
    doc
}

#[test]
fn copy_export_is_lossless_but_never_rebinds_or_marks_the_original_source_saved() {
    for capacity in [0, 1, 256] {
        let f = Fixture::new();
        let mut host = LocalHost::new(config(capacity)).unwrap();
        let doc = edited(&mut host, &f);
        let version = host.read(&doc).unwrap().version;
        let prepared = host.prepare_source_export(&version).unwrap();
        let attempt = host
            .queue_source_export(
                &prepared,
                &f.path("copy"),
                SourceExportPurposeV1::CopyExport,
                requirements(),
                &facts(),
            )
            .unwrap();
        let report = host.service_source_export(&attempt, &facts()).unwrap();
        assert_eq!(report.outcome.application, Application::Written);
        assert_eq!(report.outcome.synchronization, Synchronization::Succeeded);
        assert!(report.failure.is_none() && !report.cleanup_failed);
        assert_eq!(
            fs::read(f.path("copy")).unwrap(),
            "\u{feff}Ä\r\n".as_bytes()
        );
        assert_eq!(
            fs::read(f.path("source")).unwrap(),
            "\u{feff}A\r\n".as_bytes()
        );
        assert_eq!(
            host.saved_state(&doc).unwrap().content,
            ContentComparison::Modified
        );
        assert!(host.publish_save_baseline(attempt.native_id()).is_err());
        assert!(
            host.activate_source_save_as(&attempt, 1, &activation())
                .is_err()
        );
        assert!(host.service_source_export(&attempt, &facts()).is_err());
        host.release_source_export(&attempt, &control()).unwrap();
        host.retire_save(attempt.native_id()).unwrap();
        assert_eq!(host.resource_usage().source_export_reservations, 0);
        assert_eq!(host.resource_usage().held_artifact_bytes, 0);
        if capacity == 1 {
            assert!(host.dropped_diagnostics() > 0);
        }
    }
}

#[test]
fn save_as_requires_activation_and_the_returned_handle_routes_subsequent_ordinary_saves() {
    let f = Fixture::new();
    let mut host = LocalHost::new(config(256)).unwrap();
    let old = edited(&mut host, &f);
    let version = host.read(&old).unwrap().version;
    let prepared = host.prepare_source_export(&version).unwrap();
    let attempt = host
        .queue_source_export(
            &prepared,
            &f.path("as"),
            SourceExportPurposeV1::SaveAs,
            requirements(),
            &facts(),
        )
        .unwrap();
    assert!(
        host.activate_source_save_as(&attempt, 1, &activation())
            .is_err()
    );
    host.service_source_export(&attempt, &facts()).unwrap();
    assert_eq!(
        host.saved_state(&old).unwrap().content,
        ContentComparison::Modified
    );
    let mut denied = activation();
    denied.probe_attested = None;
    assert!(host.activate_source_save_as(&attempt, 1, &denied).is_err());
    let active = host
        .activate_source_save_as(&attempt, 1, &activation())
        .unwrap();
    assert!(active.changed);
    assert_eq!(
        host.saved_state(&active.document).unwrap().content,
        ContentComparison::Unchanged
    );
    assert_eq!(
        host.save_outcome(attempt.native_id())
            .unwrap()
            .source_save_as_binding_applied,
        Some(true)
    );
    let current = host.read(&active.document).unwrap().version;
    assert_eq!(current.document, version.document);
    host.edit(
        &current,
        vec![Replacement {
            start: 0,
            end: 2,
            expected: "Ä".into(),
            inserted: "Z".into(),
        }],
    )
    .unwrap();
    let current = host.read(&active.document).unwrap().version;
    let ordinary = host.queue_save(&current, requirements()).unwrap();
    assert!(host.service_save(&old, &ordinary).is_err());
    assert_eq!(
        fs::read(f.path("source")).unwrap(),
        "\u{feff}A\r\n".as_bytes()
    );
    let report = host.service_save(&active.document, &ordinary).unwrap();
    assert_eq!(report.outcome.application, Application::Written);
    host.publish_save_baseline(&ordinary).unwrap();
    assert_eq!(fs::read(f.path("as")).unwrap(), "\u{feff}Z\r\n".as_bytes());
}

#[test]
fn changed_destination_after_write_blocks_activation_without_claiming_rollback() {
    let f = Fixture::new();
    let mut host = LocalHost::new(config(128)).unwrap();
    let doc = edited(&mut host, &f);
    let version = host.read(&doc).unwrap().version;
    let prepared = host.prepare_source_export(&version).unwrap();
    let attempt = host
        .queue_source_export(
            &prepared,
            &f.path("as"),
            SourceExportPurposeV1::SaveAs,
            requirements(),
            &facts(),
        )
        .unwrap();
    host.service_source_export(&attempt, &facts()).unwrap();
    fs::write(f.path("as"), b"external author").unwrap();
    assert!(
        host.activate_source_save_as(&attempt, 1, &activation())
            .is_err()
    );
    let outcome = host.save_outcome(attempt.native_id()).unwrap();
    assert_eq!(outcome.application, Application::Written);
    assert_eq!(outcome.source_save_as_binding_applied, Some(false));
    assert_eq!(
        host.saved_state(&doc).unwrap().content,
        ContentComparison::Modified
    );
    let mut denied = control();
    denied.authorized = None;
    assert!(host.release_source_export(&attempt, &denied).is_err());
    host.release_source_export(&attempt, &control()).unwrap();
    host.retire_save(attempt.native_id()).unwrap();
    assert_eq!(fs::read(f.path("as")).unwrap(), b"external author");
    assert_eq!(
        fs::read(f.path("source")).unwrap(),
        "\u{feff}A\r\n".as_bytes()
    );
}

#[test]
fn missing_files_and_annotation_aliases_preserve_native_preparation_consumption_semantics() {
    let f = Fixture::new();
    let mut host = LocalHost::new(config(128)).unwrap();
    let doc = host.open(&f.path("source")).unwrap();
    host.bind_annotations(&doc, &f.path("copy")).unwrap();
    let version = host.read(&doc).unwrap().version;
    let prepared = host.prepare_source_export(&version).unwrap();
    assert!(
        host.queue_source_export(
            &prepared,
            &f.path("missing"),
            SourceExportPurposeV1::SaveAs,
            requirements(),
            &facts()
        )
        .is_err()
    );
    assert_eq!(host.resource_usage().prepared_sources, 1);
    assert!(
        host.queue_source_export(
            &prepared,
            &f.path("copy"),
            SourceExportPurposeV1::SaveAs,
            requirements(),
            &facts()
        )
        .is_err()
    );
    // Native admission refuses the alias and spends its original preparation;
    // filesystem preflight above did not. Neither case admits a save attempt.
    assert_eq!(host.resource_usage().prepared_sources, 0);
    assert_eq!(host.resource_usage().source_export_reservations, 0);
    assert!(host.discard_source_export_preparation(&prepared).is_err());
    let next = host.prepare_source_export(&version).unwrap();
    host.discard_source_export_preparation(&next).unwrap();
    assert_eq!(host.resource_usage().prepared_sources, 0);
    assert!(!f.path("missing").exists());
    assert_eq!(fs::read(f.path("copy")).unwrap(), b"OLD");
}
