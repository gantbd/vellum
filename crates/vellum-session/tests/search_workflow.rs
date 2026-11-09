#![cfg(target_os = "macos")]
#[path = "../examples/support/mod.rs"]
mod support;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use support::LocalHost;
use vellum_engine::coordinator::{
    HistoryDirection, SearchBudget, SearchDirection, SearchLimits, SearchNavigation, SearchRequest,
    SearchScope, SearchSelection, SearchStatus,
};
use vellum_engine::instrumentation::Boundary;
use vellum_engine::resource::{
    Acknowledgment, Application, Protection, SaveRequirements, Synchronization,
};
use vellum_engine::source::Replacement;

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new(bytes: &[u8]) -> Self {
        loop {
            let p = std::env::temp_dir().join(format!(
                "vellum-sdk-search-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&p) {
                Ok(()) => {
                    fs::write(p.join("source"), bytes).unwrap();
                    return Self(p);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("fixture: {e}"),
            }
        }
    }
    fn path(&self) -> PathBuf {
        self.0.join("source")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_file(self.path()).unwrap();
        fs::remove_dir(&self.0).unwrap();
    }
}
fn config(capacity: usize) -> support::SessionConfig {
    let mut config = support::config(capacity, true);
    config.engine.search = Some(SearchLimits {
        max_sessions_issued: 8,
        max_live_sessions: 4,
        max_live_results: 8,
        max_root_bytes: 4096,
        max_scope_ranges: 16,
        max_query_bytes: 1024,
        max_comparisons: 65536,
        max_matches: 128,
        max_match_bytes: 4096,
        max_replacement_bytes: 8192,
    });
    config
}
fn query(text: &str) -> SearchRequest {
    SearchRequest {
        query: text.into(),
        scope: SearchScope::Document,
    }
}
fn next() -> SearchNavigation<'static> {
    SearchNavigation {
        direction: SearchDirection::Next,
        start: 0,
        current: None,
        wrap: false,
    }
}

#[test]
fn native_literal_replace_round_trips_history_and_lossless_save_in_all_capture_modes() {
    for capacity in [0, 1, 256] {
        let original = "\u{feff}name\r\nDeutsch: name\n```mermaid\rname\r```";
        let f = Fixture::new(original.as_bytes());
        let mut host = LocalHost::new(config(capacity)).unwrap();
        let doc = host.open(&f.path()).unwrap();
        let version = host.read(&doc).unwrap().version;
        let session = host.open_search(&doc).unwrap();
        let result = host.search(&session, &version, &query("name")).unwrap();
        assert_eq!(result.status(), SearchStatus::Complete);
        assert_eq!(result.exact_total(), Some(3));
        let first = host.navigate_search(&result, next()).unwrap().unwrap();
        assert_eq!((first.range.start, first.range.end), (0, 4));
        let commit = host
            .replace_search(&SearchSelection::All(result.clone()), "$1\\literal")
            .unwrap();
        assert!(commit.history_group.is_some());
        let changed = original.replace("name", "$1\\literal");
        assert_eq!(
            host.read(&doc).unwrap().source.encode().unwrap(),
            changed.as_bytes()
        );
        assert!(host.navigate_search(&result, next()).is_err());
        assert!(host.reverse(&doc, HistoryDirection::Undo).unwrap());
        assert_eq!(
            host.read(&doc).unwrap().source.encode().unwrap(),
            original.as_bytes()
        );
        assert!(host.reverse(&doc, HistoryDirection::Redo).unwrap());
        let version = host.read(&doc).unwrap().version;
        let save = host
            .queue_save(
                &version,
                SaveRequirements {
                    protection: Protection::BestEffort,
                    acknowledgment: Acknowledgment::Synchronized,
                },
            )
            .unwrap();
        let report = host.service_save(&doc, &save).unwrap();
        assert!(report.failure.is_none() && !report.cleanup_failed);
        assert_eq!(report.outcome.application, Application::Written);
        assert_eq!(report.outcome.synchronization, Synchronization::Succeeded);
        host.publish_save_baseline(&save).unwrap();
        assert_eq!(fs::read(f.path()).unwrap(), changed.as_bytes());
        let mut reopened = LocalHost::new(config(0)).unwrap();
        let reopened_doc = reopened.open(&f.path()).unwrap();
        assert_eq!(
            reopened
                .read(&reopened_doc)
                .unwrap()
                .source
                .encode()
                .unwrap(),
            changed.as_bytes()
        );
        host.close_search(&session).unwrap();
        assert!(host.navigate_search(&result, next()).is_err());
        assert_eq!(host.search_usage().live_sessions, 0);
        assert_eq!(host.search_usage().live_results, 1);
        drop(first);
        drop(result);
        assert_eq!(host.search_usage().live_results, 0);
        if capacity == 1 {
            assert!(host.dropped_diagnostics() > 0);
        }
        if capacity == 256 {
            let mut boundaries = Vec::new();
            while let Some(record) = host.pop_diagnostic() {
                boundaries.push(record.boundary);
            }
            for expected in [
                Boundary::SearchControl,
                Boundary::SearchEvaluation,
                Boundary::SearchNavigation,
                Boundary::SearchReplacement,
                Boundary::Publication,
            ] {
                assert!(boundaries.contains(&expected), "missing {expected:?}");
            }
        }
    }
}

#[test]
fn superseded_foreign_stale_and_locked_results_cannot_mutate_or_refresh_the_target() {
    let f = Fixture::new(b"name name");
    let mut host = LocalHost::new(config(256)).unwrap();
    let doc = host.open(&f.path()).unwrap();
    let version = host.read(&doc).unwrap().version;
    let session = host.open_search(&doc).unwrap();
    let old = host.search(&session, &version, &query("name")).unwrap();
    let current = host.search(&session, &version, &query("name")).unwrap();
    assert!(
        host.replace_search(&SearchSelection::All(old), "wrong")
            .is_err()
    );
    let mut foreign = LocalHost::new(config(0)).unwrap();
    assert!(foreign.navigate_search(&current, next()).is_err());
    assert!(
        foreign
            .replace_search(&SearchSelection::All(current.clone()), "wrong")
            .is_err()
    );
    host.set_editing_locked(&doc, true).unwrap();
    assert!(
        host.replace_search(&SearchSelection::All(current.clone()), "wrong")
            .is_err()
    );
    host.set_editing_locked(&doc, false).unwrap();
    host.edit(
        &version,
        vec![Replacement {
            start: 0,
            end: 4,
            expected: "name".into(),
            inserted: "renamed".into(),
        }],
    )
    .unwrap();
    assert!(
        host.replace_search(&SearchSelection::All(current.clone()), "wrong")
            .is_err()
    );
    assert_eq!(host.read(&doc).unwrap().source.text(), "renamed name");
    host.close_search(&session).unwrap();
    assert!(host.search(&session, &version, &query("name")).is_err());
    assert_eq!(fs::read(f.path()).unwrap(), b"name name");
}

#[test]
fn partial_results_are_explicit_and_cannot_be_used_for_replace_all() {
    let f = Fixture::new(b"name name name");
    let mut cfg = config(32);
    cfg.engine.search.as_mut().unwrap().max_matches = 1;
    let mut host = LocalHost::new(cfg).unwrap();
    let doc = host.open(&f.path()).unwrap();
    let version = host.read(&doc).unwrap().version;
    let session = host.open_search(&doc).unwrap();
    let result = host.search(&session, &version, &query("name")).unwrap();
    assert_eq!(
        result.status(),
        SearchStatus::Exhausted(SearchBudget::Matches)
    );
    assert_eq!(result.exact_total(), None);
    assert!(
        host.replace_search(&SearchSelection::All(result.clone()), "wrong")
            .is_err()
    );
    assert_eq!(host.read(&doc).unwrap().version, version);
    host.close_search(&session).unwrap();
    drop(result);
    assert_eq!(host.search_usage().live_results, 0);
    assert_eq!(fs::read(f.path()).unwrap(), b"name name name");
}
