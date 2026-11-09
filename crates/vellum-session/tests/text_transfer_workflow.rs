#![cfg(target_os = "macos")]
#[path = "../examples/support/mod.rs"]
mod support;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use support::{LocalDocument, LocalHost};
use vellum_engine::{
    attachment::Affinity,
    coordinates::BytePosition,
    coordinator::{InvocationLimits, InvocationState, LogicalViewContext, LogicalViewLimits},
    selection::{Selection, SelectionBudget},
};
use vellum_session::host::{InvocationClaimKindV1, TextTransferCaptureLimitsV1};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "vellum-transfer-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    fs::write(path.join("source"), "\u{feff}ab\r\nGrüße\nend").unwrap();
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
        fs::remove_file(self.0.join("source")).unwrap();
        fs::remove_dir(&self.0).unwrap();
    }
}
fn new_host(capacity: usize) -> LocalHost {
    let mut config = support::config(capacity, true);
    config.engine.invocation = Some(InvocationLimits {
        max_lifetime_issued: 8,
        max_retained: 8,
        max_intent_bytes: 65536,
    });
    config.logical_views = Some(LogicalViewLimits {
        max_live_views: 4,
        max_issued_views: 4,
        max_total_selections: 16,
        normalization: SelectionBudget {
            max_input_selections: 8,
            max_output_selections: 8,
            max_work: 65536,
        },
        max_mapping_steps: 65536,
    });
    LocalHost::new(config).unwrap()
}
fn limits() -> TextTransferCaptureLimitsV1 {
    TextTransferCaptureLimitsV1 {
        max_fragments: 8,
        max_plain_bytes: 4096,
        max_logical_bytes: 32768,
    }
}
fn range(a: usize, b: usize) -> Selection {
    Selection {
        anchor: BytePosition(a),
        head: BytePosition(b),
        affinity: Affinity::After,
    }
}
fn view(
    host: &mut LocalHost,
    fixture: &Fixture,
    selections: &[Selection],
) -> (LocalDocument, LogicalViewContext) {
    let doc = host.open(&fixture.0.join("source")).unwrap();
    let source = host.read(&doc).unwrap().version;
    let view = host.create_logical_view().unwrap();
    let stamp = host.logical_view(&view).unwrap().stamp;
    let attached = host
        .attach_logical_view(&view, stamp, &source, selections, 0)
        .unwrap();
    (
        doc,
        LogicalViewContext {
            view,
            source,
            stamp: attached.stamp,
        },
    )
}

#[test]
fn capture_keeps_native_origin_and_verbatim_order_without_source_history_or_external_effect() {
    for capacity in [0, 1, 256] {
        let f = Fixture::new();
        let mut host = new_host(capacity);
        let (doc, context) = view(&mut host, &f, &[range(12, 15), range(0, 1), range(4, 11)]);
        host.set_editing_locked(&doc, true).unwrap();
        let origin = host.issue_invocation().unwrap();
        let claim = host
            .claim_invocation(&origin, InvocationClaimKindV1::SourceNeutral)
            .unwrap();
        let captured = host
            .capture_text_transfer(&origin, &context, limits())
            .unwrap();
        assert_eq!(captured.origin(), &origin);
        assert_eq!(captured.context(), &context);
        assert_eq!(captured.fragments(), &["a", "Grüße", "end"]);
        assert_eq!(captured.selections().primary(), 2);
        assert_eq!(
            captured
                .selections()
                .selections()
                .iter()
                .map(|s| s.anchor.0)
                .collect::<Vec<_>>(),
            vec![0, 4, 12]
        );
        assert_eq!(captured.plain(), Some("a\nGrüße\nend"));
        assert_eq!(captured.usage().fragment_bytes, 11);
        assert_eq!(captured.usage().plain_bytes, 13);
        let exact = TextTransferCaptureLimitsV1 {
            max_logical_bytes: captured.usage().logical_bytes,
            ..limits()
        };
        assert!(host.capture_text_transfer(&origin, &context, exact).is_ok());
        assert!(
            host.capture_text_transfer(
                &origin,
                &context,
                TextTransferCaptureLimitsV1 {
                    max_logical_bytes: exact.max_logical_bytes - 1,
                    ..exact
                }
            )
            .is_err()
        );
        assert_eq!(format!("{captured:?}"), "TextTransferCaptureV1(<redacted>)");
        assert_eq!(host.read(&doc).unwrap().version, context.source);
        assert_eq!(
            host.logical_view(&context.view).unwrap().context(),
            Some(context.clone())
        );
        assert_eq!(host.history_usage().retained_groups, 0);
        assert_eq!(
            host.invocation_state(&origin).unwrap(),
            InvocationState::Issued
        );
        assert_eq!(host.invocation_usage().issued_lifetime, 1);
        // A different pure preview is not a task retry or structured handle.
        host.set_logical_selections(&context, &[range(0, 4), range(12, 15)], 0)
            .unwrap();
        let changed = host.logical_view(&context.view).unwrap().context().unwrap();
        let second = host
            .capture_text_transfer(&origin, &changed, limits())
            .unwrap();
        assert_eq!(second.plain(), Some("ab\r\n\nend"));
        assert_eq!(captured.plain(), Some("a\nGrüße\nend"));
        assert_eq!(
            fs::read(f.0.join("source")).unwrap(),
            "\u{feff}ab\r\nGrüße\nend".as_bytes()
        );
        host.retire_claimed_invocation(&claim).unwrap();
        if capacity == 1 {
            assert!(host.dropped_diagnostics() > 0);
        }
    }
}

#[test]
fn all_empty_selections_are_no_export_not_separator_bytes_or_whole_line_capture() {
    let f = Fixture::new();
    let mut host = new_host(0);
    let (doc, context) = view(&mut host, &f, &[range(0, 0), range(2, 2)]);
    let origin = host.issue_invocation().unwrap();
    let mut bound = limits();
    bound.max_plain_bytes = 0;
    let captured = host
        .capture_text_transfer(&origin, &context, bound)
        .unwrap();
    assert_eq!(captured.fragments(), &["", ""]);
    assert_eq!(captured.plain(), None);
    assert_eq!(captured.usage().plain_bytes, 0);
    assert_eq!(captured.usage().fragment_bytes, 0);
    assert_eq!(host.read(&doc).unwrap().version, context.source);
    assert_eq!(host.history_usage().retained_groups, 0);
    host.retire_invocation(&origin).unwrap();
}

#[test]
fn stale_foreign_spent_origins_and_capture_budget_refusals_never_truncate_or_retarget() {
    let f = Fixture::new();
    let mut host = new_host(0);
    let (doc, context) = view(&mut host, &f, &[range(0, 1), range(4, 11)]);
    let origin = host.issue_invocation().unwrap();
    for bound in [
        TextTransferCaptureLimitsV1 {
            max_fragments: 1,
            ..limits()
        },
        TextTransferCaptureLimitsV1 {
            max_plain_bytes: 8,
            ..limits()
        },
        TextTransferCaptureLimitsV1 {
            max_logical_bytes: 1,
            ..limits()
        },
    ] {
        assert!(
            host.capture_text_transfer(&origin, &context, bound)
                .is_err()
        );
        assert_eq!(
            host.logical_view(&context.view).unwrap().context(),
            Some(context.clone())
        );
    }
    let mut other = new_host(0);
    let foreign = other.issue_invocation().unwrap();
    assert!(
        host.capture_text_transfer(&foreign, &context, limits())
            .is_err()
    );
    assert!(
        other
            .capture_text_transfer(&origin, &context, limits())
            .is_err()
    );
    assert!(
        other
            .capture_text_transfer(&foreign, &context, limits())
            .is_err()
    );
    host.set_logical_selections(&context, &[range(0, 0)], 0)
        .unwrap();
    assert!(
        host.capture_text_transfer(&origin, &context, limits())
            .is_err()
    );
    let current = host.logical_view(&context.view).unwrap().context().unwrap();
    host.submit_source_invocation(
        &origin,
        &vellum_engine::coordinator::SourceIntent {
            writes: vec![],
            reads: vec![context.source.clone()],
        },
    )
    .unwrap();
    assert!(
        host.capture_text_transfer(&origin, &current, limits())
            .is_err()
    );
    host.stop_invocation(&origin).unwrap();
    host.retire_invocation(&origin).unwrap();
    assert!(
        host.capture_text_transfer(&origin, &current, limits())
            .is_err()
    );
    assert_eq!(host.read(&doc).unwrap().version, context.source);
    assert_eq!(host.history_usage().retained_groups, 0);
}
