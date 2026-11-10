#![cfg(target_os = "macos")]
#[path = "../examples/support/mod.rs"]
mod support;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use support::LocalHost;
use vellum_engine::{
    attachment::Affinity,
    coordinates::{BytePosition, CoordinateBudget},
    coordinator::{
        GeneratedNewlineV1, HistoryDirection, LineEditBudget, LineEditV1, LogicalEndpointV1,
        LogicalViewContext, LogicalViewId, LogicalViewLimits, NewlineIndentV1, SelectionAfterV1,
        TrustedDocumentReclamationV1,
    },
    instrumentation::Boundary,
    resource::{Acknowledgment, Application, Protection, SaveRequirements, Synchronization},
    selection::{Selection, SelectionBudget, SelectionError},
};
use vellum_session::host::SessionError;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new(text: &str) -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "vellum-sdk-view-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    fs::write(path.join("source"), text.as_bytes()).unwrap();
                    return Self(path);
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
fn limits() -> LogicalViewLimits {
    LogicalViewLimits {
        max_live_views: 4,
        max_issued_views: 8,
        max_total_selections: 16,
        normalization: SelectionBudget {
            max_input_selections: 8,
            max_output_selections: 8,
            max_work: 65536,
        },
        max_mapping_steps: 65536,
    }
}
fn selection(a: usize, b: usize) -> Selection {
    Selection {
        anchor: BytePosition(a),
        head: BytePosition(b),
        affinity: Affinity::After,
    }
}
fn context(host: &mut LocalHost, view: &LogicalViewId) -> LogicalViewContext {
    host.logical_view(view).unwrap().context().unwrap()
}
fn attach(
    host: &mut LocalHost,
    source: &vellum_engine::coordinator::DocumentVersion,
    selections: &[Selection],
) -> LogicalViewId {
    let id = host.create_logical_view().unwrap();
    let stamp = host.logical_view(&id).unwrap().stamp;
    host.attach_logical_view(&id, stamp, source, selections, 0)
        .unwrap();
    id
}
fn retire(host: &mut LocalHost, view: &LogicalViewId) {
    let stamp = host.logical_view(view).unwrap().stamp;
    let detached = host.detach_logical_view(view, stamp).unwrap();
    host.retire_logical_view(view, detached.stamp).unwrap();
}

#[test]
fn multi_selection_edit_maps_peer_views_and_roundtrips_native_history_save_and_reopen() {
    for capacity in [0, 1, 256] {
        let fixture = Fixture::new("\u{feff}red red\r\nProse");
        let mut host = LocalHost::new(support::config(capacity, true)).unwrap();
        host.configure_logical_views(limits()).unwrap();
        let doc = host.open(&fixture.path()).unwrap();
        let original = host.read(&doc).unwrap().version;
        let view = attach(&mut host, &original, &[selection(4, 7), selection(0, 3)]);
        let peer = attach(&mut host, &original, &[selection(7, 7)]);
        let before = context(&mut host, &view);
        assert_eq!(host.logical_view(&view).unwrap().selections.primary(), 1);
        host.set_editing_locked(&doc, true).unwrap();
        assert!(
            host.replace_logical_selections(&before, "blocked", SelectionAfterV1::CaretsAfter)
                .is_err()
        );
        assert_eq!(host.read(&doc).unwrap().version, original);
        host.set_editing_locked(&doc, false).unwrap();
        let edited = host
            .replace_logical_selections(&before, "Grün", SelectionAfterV1::CaretsAfter)
            .unwrap();
        assert!(edited.history_group.is_some());
        assert_eq!(host.read(&doc).unwrap().source.text(), "Grün Grün\r\nProse");
        let initiating = host.logical_view(&view).unwrap();
        assert_eq!(
            initiating.selections.selections(),
            &[selection(5, 5), selection(11, 11)]
        );
        assert_eq!(
            host.logical_view(&peer).unwrap().selections.selections(),
            &[selection(11, 11)]
        );
        assert!(matches!(
            host.replace_logical_selections(&before, "stale", SelectionAfterV1::Mapped),
            Err(SessionError::Selection(SelectionError::Stale))
        ));
        assert!(host.reverse(&doc, HistoryDirection::Undo).unwrap());
        assert_eq!(host.read(&doc).unwrap().source.text(), "red red\r\nProse");
        assert!(host.reverse(&doc, HistoryDirection::Redo).unwrap());
        let current = host.read(&doc).unwrap().version;
        let save = host
            .queue_save(
                &current,
                SaveRequirements {
                    protection: Protection::BestEffort,
                    acknowledgment: Acknowledgment::Synchronized,
                },
            )
            .unwrap();
        let report = host.service_save(&doc, &save).unwrap();
        assert_eq!(report.outcome.application, Application::Written);
        assert_eq!(report.outcome.synchronization, Synchronization::Succeeded);
        assert!(report.outcome.quiescent && report.outcome.no_future_effects);
        assert!(report.failure.is_none() && !report.cleanup_failed);
        host.publish_save_baseline(&save).unwrap();
        assert_eq!(
            fs::read(fixture.path()).unwrap(),
            "\u{feff}Grün Grün\r\nProse".as_bytes()
        );
        let mut reopened = LocalHost::new(support::config(0, false)).unwrap();
        let reopened_doc = reopened.open(&fixture.path()).unwrap();
        assert_eq!(
            reopened.read(&reopened_doc).unwrap().source.text(),
            "Grün Grün\r\nProse"
        );
        retire(&mut host, &view);
        retire(&mut host, &peer);
        assert_eq!(host.logical_view_usage().live, 0);
        let facts = TrustedDocumentReclamationV1 {
            authorized: Some(true),
            authored_disposition_ready: Some(true),
            session_references_released: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        };
        let retired = host.dispose(&current).unwrap();
        let unbound = host.release_source_binding(&doc, retired).unwrap();
        host.retire_history(&edited.history_group.unwrap()).unwrap();
        let released = host.release_provenance(&doc, unbound).unwrap();
        host.stop_save(&save).unwrap();
        host.retire_save(&save).unwrap();
        host.reclaim(&doc, released, &facts).unwrap();
        let version = reopened.read(&reopened_doc).unwrap().version;
        let disposed = reopened.dispose(&version).unwrap();
        let unbound = reopened
            .release_source_binding(&reopened_doc, disposed)
            .unwrap();
        let released = reopened.release_provenance(&reopened_doc, unbound).unwrap();
        reopened.reclaim(&reopened_doc, released, &facts).unwrap();
        if capacity == 1 {
            assert!(host.dropped_diagnostics() > 0);
        }
        if capacity == 256 {
            let mut boundaries = Vec::new();
            while let Some(record) = host.pop_diagnostic() {
                boundaries.push(record.boundary);
            }
            assert!(boundaries.contains(&Boundary::LogicalViewLifecycle));
            assert!(boundaries.contains(&Boundary::LogicalViewQuery));
            assert!(boundaries.contains(&Boundary::LiteralEditPreparation));
        }
    }
}

#[test]
fn exact_contexts_refuse_invalid_utf8_positions_foreign_views_stale_stamps_and_retired_handles() {
    let fixture = Fixture::new("éZ\r\nB");
    let mut host = LocalHost::new(support::config(0, false)).unwrap();
    assert!(host.create_logical_view().is_err());
    assert!(host.configure_logical_views(limits()).unwrap());
    assert!(!host.configure_logical_views(limits()).unwrap());
    let doc = host.open(&fixture.path()).unwrap();
    let original = host.read(&doc).unwrap().version;
    let view = attach(&mut host, &original, &[selection(0, 0)]);
    let before = context(&mut host, &view);
    assert!(
        host.set_logical_selections(&before, &[selection(1, 1)], 0)
            .is_err()
    );
    assert_eq!(context(&mut host, &view), before);
    host.move_logical_selections(
        &before,
        LogicalEndpointV1::LineEnd,
        true,
        CoordinateBudget { max_scan_bytes: 64 },
    )
    .unwrap();
    let moved = host.logical_view(&view).unwrap();
    assert_eq!(moved.selections.selections(), &[selection(0, 3)]);
    assert!(matches!(
        host.set_logical_selections(&before, &[selection(0, 0)], 0),
        Err(SessionError::Selection(SelectionError::Stale))
    ));
    let mut foreign = LocalHost::new(support::config(0, false)).unwrap();
    assert!(foreign.logical_view(&view).is_err());
    assert!(
        foreign
            .replace_logical_selections(&moved.context().unwrap(), "bad", SelectionAfterV1::Mapped)
            .is_err()
    );
    assert_eq!(host.read(&doc).unwrap().version, original);
    retire(&mut host, &view);
    assert!(host.logical_view(&view).is_err());
    assert_eq!(host.logical_view_usage().issued_lifetime, 1);
}

#[test]
fn explicit_line_profiles_indent_prose_and_code_without_normalizing_existing_delimiters() {
    let fixture = Fixture::new(" prose\r\n```mermaid\rgraph LR\n");
    let mut host = LocalHost::new(support::config(256, true)).unwrap();
    host.configure_logical_views(limits()).unwrap();
    let doc = host.open(&fixture.path()).unwrap();
    let source = host.read(&doc).unwrap().version;
    let view = attach(&mut host, &source, &[selection(0, 0)]);
    let before = context(&mut host, &view);
    let request = LineEditV1::Indent {
        unit: "  ".into(),
        tab_width: 4,
    };
    assert!(host.edit_logical_lines(&before, &request).is_err());
    host.configure_line_edits(LineEditBudget {
        max_lines: 32,
        max_prefix_bytes: 64,
        max_indent_columns: 64,
        max_unit_bytes: 8,
        max_derived_bytes: 16384,
        max_source_scan_bytes: 65536,
    })
    .unwrap();
    host.move_logical_selections(
        &before,
        LogicalEndpointV1::DocumentEnd,
        true,
        CoordinateBudget {
            max_scan_bytes: 65536,
        },
    )
    .unwrap();
    let all = context(&mut host, &view);
    host.edit_logical_lines(&all, &request).unwrap();
    assert_eq!(
        host.read(&doc).unwrap().source.text(),
        "   prose\r\n  ```mermaid\r  graph LR\n"
    );
    assert!(host.edit_logical_lines(&all, &request).is_err());
    assert!(host.reverse(&doc, HistoryDirection::Undo).unwrap());
    assert_eq!(
        host.read(&doc).unwrap().source.text(),
        " prose\r\n```mermaid\rgraph LR\n"
    );
    let now = context(&mut host, &view);
    host.set_logical_selections(&now, &[selection(6, 6)], 0)
        .unwrap();
    let now = context(&mut host, &view);
    host.edit_logical_lines(
        &now,
        &LineEditV1::Newline {
            delimiter: GeneratedNewlineV1::Lf,
            indent: NewlineIndentV1::CopyPrefix,
        },
    )
    .unwrap();
    assert_eq!(
        host.read(&doc).unwrap().source.text(),
        " prose\n \r\n```mermaid\rgraph LR\n"
    );
    retire(&mut host, &view);
}
