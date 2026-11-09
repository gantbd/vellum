#![cfg(target_os = "macos")]

#[path = "../examples/support/mod.rs"]
mod support;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use vellum_engine::coordinator::{CoordinatorError, HistoryDirection};
use vellum_engine::instrumentation::Boundary;
use vellum_engine::resource::{Acknowledgment, Application, Protection, SaveRequirements};
use vellum_engine::source::Replacement;
use vellum_session::host::LocalHost;
use vellum_session::local::{LocalSession, SessionError};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new(bytes: &[u8]) -> Self {
        loop {
            let dir = std::env::temp_dir().join(format!(
                "vellum-session-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&dir) {
                Ok(()) => {
                    fs::write(dir.join("source"), bytes).unwrap();
                    return Self(dir);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("fixture directory: {e}"),
            }
        }
    }
    fn path(&self) -> PathBuf {
        self.0.join("source")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // Only the known file and exact exclusively created directory; no traversal.
        fs::remove_file(self.path()).unwrap();
        fs::remove_dir(&self.0).unwrap();
    }
}
fn edit() -> Vec<Replacement> {
    vec![Replacement {
        start: 0,
        end: 1,
        expected: "A".into(),
        inserted: "Ä".into(),
    }]
}
fn requirements() -> SaveRequirements {
    SaveRequirements {
        protection: Protection::BestEffort,
        acknowledgment: Acknowledgment::Synchronized,
    }
}

#[test]
fn given_mixed_prose_when_edited_undone_redone_saved_and_reopened_then_bytes_are_lossless() {
    for (capacity, timed) in [(0, false), (256, false), (256, true), (1, true)] {
        let f = Fixture::new("\u{feff}A\r\nDeutsch: Grüße\n```mermaid\ngraph LR\n```\r".as_bytes());
        let mut session = LocalSession::open(&f.path(), support::config(capacity, timed)).unwrap();
        let original = session.read().unwrap().source.encode().unwrap();
        let version = session.read().unwrap().version;
        session.edit(&version, edit()).unwrap();
        assert!(session.reverse(HistoryDirection::Undo).unwrap());
        assert_eq!(session.read().unwrap().source.encode().unwrap(), original);
        assert!(session.reverse(HistoryDirection::Redo).unwrap());
        let changed = session.read().unwrap().source.encode().unwrap();
        let version = session.read().unwrap().version;
        let save = session.queue_save(&version, requirements()).unwrap();
        let report = session.service_save(&save).unwrap();
        assert_eq!(report.failure, None);
        assert!(!report.cleanup_failed);
        assert_eq!(report.outcome.application, Application::Written);
        assert!(report.outcome.quiescent && report.outcome.no_future_effects);
        session.publish_save_baseline(&save).unwrap();
        assert_eq!(fs::read(f.path()).unwrap(), changed);
        let mut reopened = LocalSession::open(&f.path(), support::config(0, false)).unwrap();
        assert_eq!(reopened.read().unwrap().source.encode().unwrap(), changed);
        assert_eq!(
            session.save_outcome(&save).unwrap().application,
            Application::Written
        );
        if capacity == 1 {
            assert!(session.dropped_diagnostics() > 0);
        }
        if capacity == 256 {
            let mut phases = Vec::new();
            while let Some(record) = session.pop_diagnostic() {
                phases.push(record.boundary);
            }
            for boundary in [
                Boundary::SubstrateRead,
                Boundary::Publication,
                Boundary::SubstrateReplacement,
                Boundary::SubstrateSynchronization,
            ] {
                assert!(phases.contains(&boundary), "missing {boundary:?}");
            }
        }
    }
}

#[test]
fn stale_edits_and_stale_save_captures_refuse_without_overwriting() {
    let f = Fixture::new(b"A");
    let mut session = LocalSession::open(&f.path(), support::config(0, false)).unwrap();
    let stale = session.read().unwrap().version;
    session.edit(&stale, edit()).unwrap();
    assert!(matches!(
        session.edit(&stale, edit()),
        Err(SessionError::Engine(CoordinatorError::Stale))
    ));
    assert!(session.queue_save(&stale, requirements()).is_err());
    assert_eq!(session.read().unwrap().source.text(), "Ä");
    assert_eq!(fs::read(f.path()).unwrap(), b"A");
}

#[test]
fn external_changes_refuse_save_and_original_task_remains_controllable() {
    let f = Fixture::new(b"A");
    let mut session = LocalSession::open(&f.path(), support::config(16, true)).unwrap();
    let version = session.read().unwrap().version;
    session.edit(&version, edit()).unwrap();
    let version = session.read().unwrap().version;
    let save = session.queue_save(&version, requirements()).unwrap();
    fs::write(f.path(), b"external").unwrap();
    assert!(session.service_save(&save).is_err());
    assert_eq!(fs::read(f.path()).unwrap(), b"external");
    session.stop_save(&save).unwrap();
    assert_eq!(
        session.save_outcome(&save).unwrap().application,
        Application::NotWritten
    );
    session.retire_save(&save).unwrap();
}

#[test]
fn foreign_save_cannot_be_serviced_or_disclosed_by_another_session() {
    let f = Fixture::new(b"A");
    let g = Fixture::new(b"A");
    let mut one = LocalSession::open(&f.path(), support::config(0, false)).unwrap();
    let mut two = LocalSession::open(&g.path(), support::config(0, false)).unwrap();
    let version = one.read().unwrap().version;
    assert!(two.edit(&version, edit()).is_err());
    let save = one.queue_save(&version, requirements()).unwrap();
    assert!(two.service_save(&save).is_err());
    assert!(two.save_outcome(&save).is_err());
    assert_eq!(fs::read(g.path()).unwrap(), b"A");
    one.stop_save(&save).unwrap();
    one.retire_save(&save).unwrap();
}

#[test]
fn repeating_service_never_reexecutes_a_completed_write() {
    let f = Fixture::new(b"A");
    let mut session = LocalSession::open(&f.path(), support::config(0, false)).unwrap();
    let version = session.read().unwrap().version;
    let save = session.queue_save(&version, requirements()).unwrap();
    session.service_save(&save).unwrap();
    session.publish_save_baseline(&save).unwrap();
    fs::write(f.path(), b"external after save").unwrap();
    assert!(session.service_save(&save).is_err());
    assert_eq!(fs::read(f.path()).unwrap(), b"external after save");
    assert_eq!(
        session.save_outcome(&save).unwrap().application,
        Application::Written
    );
    // The baseline retains its original task. Refusal is not a leaked wrapper handle.
    assert!(session.retire_save(&save).is_err());
}

#[test]
fn unsupported_protection_refuses_at_admission_without_a_write() {
    let f = Fixture::new(b"A");
    let mut session = LocalSession::open(&f.path(), support::config(0, false)).unwrap();
    let version = session.read().unwrap().version;
    // Native filesystem intentionally does not claim atomic conditional writes.
    let mut requirement = requirements();
    requirement.protection = Protection::AtomicConditional;
    assert!(matches!(
        session.queue_save(&version, requirement),
        Err(SessionError::Resource(
            vellum_engine::resource::ResourceError::Unsupported
        ))
    ));
    assert_eq!(fs::read(f.path()).unwrap(), b"A");
}

#[test]
fn invalid_utf8_and_oversized_input_refuse_without_touching_disk() {
    for bytes in [vec![0xff], vec![b'A'; 65537]] {
        let f = Fixture::new(&bytes);
        assert!(LocalSession::open(&f.path(), support::config(0, false)).is_err());
        assert_eq!(fs::read(f.path()).unwrap(), bytes);
    }
}

#[test]
fn no_op_has_no_history_and_failed_edit_does_not_publish() {
    let f = Fixture::new(b"A");
    let mut session = LocalSession::open(&f.path(), support::config(1, true)).unwrap();
    let version = session.read().unwrap().version;
    let out = session.edit(&version, Vec::new()).unwrap();
    assert!(out.operation.is_none());
    assert!(!session.reverse(HistoryDirection::Undo).unwrap());
    assert!(
        session
            .edit(
                &version,
                vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "wrong".into(),
                    inserted: "discard".into(),
                }]
            )
            .is_err()
    );
    assert_eq!(session.read().unwrap().source.text(), "A");
    assert_eq!(fs::read(f.path()).unwrap(), b"A");
}

#[test]
fn one_host_publishes_cross_file_edits_and_reverses_linked_history_atomically() {
    use vellum_engine::coordinator::DocumentEdits;
    let f = Fixture::new(b"A");
    let g = Fixture::new(b"A");
    let mut config = support::config(256, true);
    config.filesystem.max_resources = 4;
    let mut host = LocalHost::new(config).unwrap();
    let first = host.open(&f.path()).unwrap();
    let second = host.open(&g.path()).unwrap();
    let a = host.read(&first).unwrap().version;
    let b = host.read(&second).unwrap().version;
    host.edit_batch(
        &[
            DocumentEdits {
                expected: a.clone(),
                edits: edit(),
            },
            DocumentEdits {
                expected: b.clone(),
                edits: edit(),
            },
        ],
        &[],
    )
    .unwrap();
    assert_eq!(host.read(&first).unwrap().source.text(), "Ä");
    assert_eq!(host.read(&second).unwrap().source.text(), "Ä");
    assert!(host.reverse(&first, HistoryDirection::Undo).unwrap());
    assert_eq!(host.read(&first).unwrap().source.text(), "A");
    assert_eq!(host.read(&second).unwrap().source.text(), "A");
    assert!(host.reverse(&second, HistoryDirection::Redo).unwrap());
    assert_eq!(host.read(&first).unwrap().source.text(), "Ä");
    assert_eq!(host.read(&second).unwrap().source.text(), "Ä");
    let current_a = host.read(&first).unwrap().version;
    assert!(
        host.edit_batch(
            &[
                DocumentEdits {
                    expected: current_a,
                    edits: Vec::new()
                },
                DocumentEdits {
                    expected: b,
                    edits: edit()
                },
            ],
            &[]
        )
        .is_err()
    );
    assert_eq!(host.read(&first).unwrap().source.text(), "Ä");
    // A stale declared read also fences an otherwise valid write to another file.
    let read_dependency = host.read(&second).unwrap().version;
    host.edit(
        &read_dependency,
        vec![Replacement {
            start: 0,
            end: 2,
            expected: "Ä".into(),
            inserted: "B".into(),
        }],
    )
    .unwrap();
    let current_a = host.read(&first).unwrap().version;
    assert!(matches!(
        host.edit_batch(
            &[DocumentEdits {
                expected: current_a,
                edits: vec![Replacement {
                    start: 0,
                    end: 2,
                    expected: "Ä".into(),
                    inserted: "not published".into(),
                }],
            }],
            &[read_dependency]
        ),
        Err(SessionError::Engine(CoordinatorError::Stale))
    ));
    assert_eq!(host.read(&first).unwrap().source.text(), "Ä");
    assert_eq!(host.read(&second).unwrap().source.text(), "B");
    assert_eq!(fs::read(f.path()).unwrap(), b"A");
    assert_eq!(fs::read(g.path()).unwrap(), b"A");
}

#[test]
fn save_cannot_be_routed_to_another_document_in_the_same_host() {
    let f = Fixture::new(b"A");
    let g = Fixture::new(b"A");
    let mut config = support::config(0, false);
    config.filesystem.max_resources = 2;
    let mut host = LocalHost::new(config).unwrap();
    let first = host.open(&f.path()).unwrap();
    let second = host.open(&g.path()).unwrap();
    let version = host.read(&first).unwrap().version;
    let save = host.queue_save(&version, requirements()).unwrap();
    assert!(matches!(
        host.service_save(&second, &save),
        Err(SessionError::WrongDocument)
    ));
    assert_eq!(fs::read(g.path()).unwrap(), b"A");
    host.stop_save(&save).unwrap();
    host.retire_save(&save).unwrap();
}

#[test]
fn protected_document_retirement_requires_releasing_original_save_and_host_premises() {
    use vellum_engine::coordinator::{LifecyclePhase, TrustedDocumentReclamationV1};
    let f = Fixture::new(b"A");
    let mut host = LocalHost::new(support::config(128, true)).unwrap();
    let doc = host.open(&f.path()).unwrap();
    let version = host.read(&doc).unwrap().version;
    let history = host.edit(&version, edit()).unwrap().history_group.unwrap();
    let version = host.read(&doc).unwrap().version;
    let save = host.queue_save(&version, requirements()).unwrap();
    host.service_save(&doc, &save).unwrap();
    host.publish_save_baseline(&save).unwrap();
    let disposed = host.dispose(&version).unwrap();
    assert_eq!(disposed.phase, LifecyclePhase::DisposedRetained);
    assert!(host.release_provenance(&doc, disposed).is_err());
    let unbound = host.release_source_binding(&doc, disposed).unwrap();
    assert!(host.release_provenance(&doc, unbound).is_err());
    host.retire_history(&history).unwrap();
    let released = host.release_provenance(&doc, unbound).unwrap();
    assert_eq!(released.phase, LifecyclePhase::ProvenanceReleased);
    let mut facts = TrustedDocumentReclamationV1 {
        authorized: Some(true),
        authored_disposition_ready: Some(true),
        session_references_released: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    assert!(host.reclaim(&doc, released, &facts).is_err());
    host.retire_save(&save).unwrap();
    facts.authorized = None;
    assert!(host.reclaim(&doc, released, &facts).is_err());
    facts.authorized = Some(true);
    host.reclaim(&doc, released, &facts).unwrap();
    assert!(host.read(&doc).is_err());
    assert!(host.lifecycle(&doc).is_err());
    assert_eq!(fs::read(f.path()).unwrap(), "Ä".as_bytes());
}
