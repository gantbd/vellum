#![cfg(target_os = "macos")]
#[path = "../examples/support/mod.rs"]
mod support;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use vellum_engine::{
    coordinator::{
        DocumentEdits, HistoryDirection, InvocationLimits, InvocationOutcome, InvocationState,
        SourceIntent, TrustedDocumentReclamationV1,
    },
    instrumentation::Boundary,
    resource::{Acknowledgment, Application, Protection, SaveRequirements, Synchronization},
    source::Replacement,
};
use vellum_session::host::{LocalDocument, LocalHost};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "vellum-sdk-invoke-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    fs::write(path.join("a"), "\u{feff}A\r\n").unwrap();
                    fs::write(path.join("b"), b"B\n").unwrap();
                    return Self(path);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("fixture: {e}"),
            }
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_file(self.0.join("a")).unwrap();
        fs::remove_file(self.0.join("b")).unwrap();
        fs::remove_dir(&self.0).unwrap();
    }
}
fn config(capacity: usize) -> vellum_session::host::SessionConfig {
    let mut config = support::config(capacity, true);
    config.filesystem.max_resources = 2;
    config.engine.invocation = Some(InvocationLimits {
        max_lifetime_issued: 8,
        max_retained: 4,
        max_intent_bytes: 65536,
    });
    config
}
fn intent(
    host: &mut LocalHost,
    document: &LocalDocument,
    expected: &str,
    inserted: &str,
) -> SourceIntent {
    SourceIntent {
        writes: vec![DocumentEdits {
            expected: host.read(document).unwrap().version,
            edits: vec![Replacement {
                start: 0,
                end: expected.len(),
                expected: expected.into(),
                inserted: inserted.into(),
            }],
        }],
        reads: vec![],
    }
}
fn cleanup(host: &mut LocalHost, document: &LocalDocument) {
    let version = host.read(document).unwrap().version;
    let disposed = host.dispose(&version).unwrap();
    let unbound = host.release_source_binding(document, disposed).unwrap();
    let released = host.release_provenance(document, unbound).unwrap();
    host.reclaim(
        document,
        released,
        &TrustedDocumentReclamationV1 {
            authorized: Some(true),
            authored_disposition_ready: Some(true),
            session_references_released: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        },
    )
    .unwrap();
}

#[test]
fn original_invocation_replay_preserves_cross_file_publication_history_save_and_retained_outcome() {
    for capacity in [0, 1, 256] {
        let fixture = Fixture::new();
        let mut host = LocalHost::new(config(capacity)).unwrap();
        let a = host.open(&fixture.0.join("a")).unwrap();
        let b = host.open(&fixture.0.join("b")).unwrap();
        let mut request = intent(&mut host, &a, "A", "Ä");
        request
            .writes
            .extend(intent(&mut host, &b, "B", "β").writes);
        let id = host.issue_invocation().unwrap();
        assert_eq!(host.invocation_state(&id).unwrap(), InvocationState::Issued);
        assert_eq!(
            host.submit_source_invocation(&id, &request).unwrap(),
            InvocationState::Pending
        );
        assert_eq!(
            host.submit_source_invocation(&id, &request).unwrap(),
            InvocationState::Pending
        );
        let mut changed = request.clone();
        changed.writes[0].edits[0].inserted = "different".into();
        assert!(host.submit_source_invocation(&id, &changed).is_err());
        assert_eq!(host.run_invocation(&id).unwrap(), InvocationState::Finished);
        let group = match host.invocation_outcome(&id).unwrap() {
            InvocationOutcome::Committed(outcome) => {
                assert_eq!(outcome.changes.len(), 2);
                outcome.history_group.clone().unwrap()
            }
            _ => panic!("not committed"),
        };
        let a_version = host.read(&a).unwrap().version;
        let b_version = host.read(&b).unwrap().version;
        assert_eq!(host.run_invocation(&id).unwrap(), InvocationState::Finished);
        assert_eq!(
            host.submit_source_invocation(&id, &request).unwrap(),
            InvocationState::Finished
        );
        assert_eq!(host.read(&a).unwrap().version, a_version);
        assert_eq!(host.read(&b).unwrap().version, b_version);
        assert_eq!(host.invocation_usage().issued_lifetime, 1);
        assert!(host.reverse(&a, HistoryDirection::Undo).unwrap());
        assert_eq!(host.read(&a).unwrap().source.text(), "A\r\n");
        assert_eq!(host.read(&b).unwrap().source.text(), "B\n");
        let undone = host.read(&a).unwrap().version;
        host.run_invocation(&id).unwrap();
        assert_eq!(host.read(&a).unwrap().version, undone);
        assert!(matches!(
            host.invocation_outcome(&id).unwrap(),
            InvocationOutcome::Committed(_)
        ));
        assert!(host.reverse(&b, HistoryDirection::Redo).unwrap());
        let mut saves = Vec::new();
        for doc in [&a, &b] {
            let version = host.read(doc).unwrap().version;
            let task = host
                .queue_save(
                    &version,
                    SaveRequirements {
                        protection: Protection::BestEffort,
                        acknowledgment: Acknowledgment::Synchronized,
                    },
                )
                .unwrap();
            let report = host.service_save(doc, &task).unwrap();
            assert_eq!(report.outcome.application, Application::Written);
            assert_eq!(report.outcome.synchronization, Synchronization::Succeeded);
            assert!(report.failure.is_none() && !report.cleanup_failed);
            assert!(report.outcome.quiescent && report.outcome.no_future_effects);
            host.publish_save_baseline(&task).unwrap();
            saves.push(task);
        }
        assert_eq!(
            fs::read(fixture.0.join("a")).unwrap(),
            "\u{feff}Ä\r\n".as_bytes()
        );
        assert_eq!(fs::read(fixture.0.join("b")).unwrap(), "β\n".as_bytes());
        host.retire_invocation(&id).unwrap();
        let a_current = host.read(&a).unwrap().version;
        let b_current = host.read(&b).unwrap().version;
        let da = host.dispose(&a_current).unwrap();
        let db = host.dispose(&b_current).unwrap();
        let ua = host.release_source_binding(&a, da).unwrap();
        let ub = host.release_source_binding(&b, db).unwrap();
        host.retire_history(&group).unwrap();
        for (doc, state, task) in [(&a, ua, &saves[0]), (&b, ub, &saves[1])] {
            let released = host.release_provenance(doc, state).unwrap();
            host.stop_save(task).unwrap();
            host.retire_save(task).unwrap();
            host.reclaim(
                doc,
                released,
                &TrustedDocumentReclamationV1 {
                    authorized: Some(true),
                    authored_disposition_ready: Some(true),
                    session_references_released: Some(true),
                    retention_released: Some(true),
                    delivery_drained: Some(true),
                },
            )
            .unwrap();
        }
        if capacity == 1 {
            assert!(host.dropped_diagnostics() > 0);
        }
        if capacity == 256 {
            let mut boundaries = Vec::new();
            while let Some(record) = host.pop_diagnostic() {
                boundaries.push(record.boundary);
            }
            assert!(boundaries.contains(&Boundary::InvocationIssuance));
            assert!(boundaries.contains(&Boundary::InvocationReplay));
        }
    }
}

#[test]
fn pending_stop_forget_and_retirement_do_not_reexecute_or_refund_lifetime_identity() {
    let fixture = Fixture::new();
    let mut profile = config(0);
    profile
        .engine
        .invocation
        .as_mut()
        .unwrap()
        .max_lifetime_issued = 2;
    let mut host = LocalHost::new(profile).unwrap();
    let doc = host.open(&fixture.0.join("a")).unwrap();
    let request = intent(&mut host, &doc, "A", "must not publish");
    let issued = host.issue_invocation().unwrap();
    assert!(host.stop_invocation(&issued).is_err());
    host.retire_invocation(&issued).unwrap();
    let id = host.issue_invocation().unwrap();
    host.submit_source_invocation(&id, &request).unwrap();
    assert!(host.retire_invocation(&id).is_err());
    host.stop_invocation(&id).unwrap();
    assert!(matches!(
        host.invocation_outcome(&id).unwrap(),
        InvocationOutcome::StoppedBeforePublication
    ));
    host.run_invocation(&id).unwrap();
    host.forget_invocation_outcome(&id).unwrap();
    assert!(host.invocation_outcome(&id).is_err());
    assert_eq!(
        host.submit_source_invocation(&id, &request).unwrap(),
        InvocationState::Finished
    );
    assert_eq!(host.read(&doc).unwrap().source.text(), "A\r\n");
    host.retire_invocation(&id).unwrap();
    assert!(host.run_invocation(&id).is_err());
    assert!(host.issue_invocation().is_err());
    assert_eq!(host.invocation_usage().retained, 0);
    cleanup(&mut host, &doc);
}

#[test]
fn disabled_foreign_and_stale_operations_refuse_without_retargeting() {
    let fixture = Fixture::new();
    let mut disabled = LocalHost::new(support::config(0, false)).unwrap();
    assert!(disabled.issue_invocation().is_err());
    let mut host = LocalHost::new(config(0)).unwrap();
    let doc = host.open(&fixture.0.join("a")).unwrap();
    let request = intent(&mut host, &doc, "A", "B");
    let id = host.issue_invocation().unwrap();
    let mut foreign = LocalHost::new(config(0)).unwrap();
    assert!(foreign.submit_source_invocation(&id, &request).is_err());
    assert!(foreign.run_invocation(&id).is_err());
    assert!(foreign.invocation_outcome(&id).is_err());
    host.submit_source_invocation(&id, &request).unwrap();
    host.edit(
        &request.writes[0].expected,
        vec![Replacement {
            start: 0,
            end: 1,
            expected: "A".into(),
            inserted: "C".into(),
        }],
    )
    .unwrap();
    host.run_invocation(&id).unwrap();
    assert!(matches!(
        host.invocation_outcome(&id).unwrap(),
        InvocationOutcome::Rejected(_)
    ));
    assert_eq!(host.read(&doc).unwrap().source.text(), "C\r\n");
    host.retire_invocation(&id).unwrap();
    assert_eq!(
        fs::read(fixture.0.join("a")).unwrap(),
        "\u{feff}A\r\n".as_bytes()
    );
}
