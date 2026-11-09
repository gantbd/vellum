use super::*;
use crate::coordinator::shutdown::tests::install;
#[test]
fn shutdown_inventory_tracks_real_source_context_holds_funding_and_closed_owner_state() {
    use crate::coordinator::shutdown::tests::{inspect, inspect_read};
    let (mut o, mut r, s) = configured(limits());
    let original = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let initial = inspect(&o, &mut r);
    let hold = o
        .hold_source_read_context(
            &original,
            &s,
            &SourceReadHoldFactsV1 {
                owner_control_allowed: Some(true),
                retained: Some(true),
                resources_ready: Some(true),
            },
            &mut r,
        )
        .unwrap();
    let usage = o.source_read_context_usage();
    assert_eq!(
        crate::coordinator::shutdown::tests::inspection_current(&o, &initial, &mut r),
        Err(CoordinatorError::Stale)
    );
    let funding = o.retention_usage();
    let current = inspect(&o, &mut r);
    let view = inspect_read(&o, &current, &mut r);
    let row = view
        .owners
        .iter()
        .find(|row| row.owner.role() == ShutdownOwnerRoleV1::SourceContexts)
        .unwrap();
    assert_eq!(row.state, ShutdownOwnerStateV1::Active);
    assert_eq!(row.retained_records, Some(2));
    assert_eq!(row.reserved_bytes, Some(usage.reserved_pool_bytes));
    assert!(view.dependencies.iter().any(|edge| edge.dependent
        == ShutdownInventoryTargetV1::Owner(row.owner.clone())
        && edge.required == ShutdownInventoryTargetV1::Document(s.document.clone())));
    assert_eq!(o.source_read_context_usage(), usage);
    assert_eq!(o.retention_usage(), funding);
    let old = inspect_read(&o, &initial, &mut r);
    assert_eq!(
        old.owners
            .iter()
            .find(|r| r.owner.role() == ShutdownOwnerRoleV1::SourceContexts)
            .unwrap()
            .retained_records,
        Some(1)
    );
    let cleanup = SourceReadContextControlV1 {
        scope: &s,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    assert!(o.release_source_read_hold(&hold, &cleanup, &mut r));
    assert!(o.stop_source_read_context(&original, &cleanup, &mut r));
    assert!(o.retire_source_read_context(&original, &cleanup, &mut r));
    assert!(o.retire_source_read_contexts(&controls(), &mut r).unwrap());
    let closed = inspect(&o, &mut r);
    let closed = inspect_read(&o, &closed, &mut r);
    let row = closed
        .owners
        .iter()
        .find(|r| r.owner.role() == ShutdownOwnerRoleV1::SourceContexts)
        .unwrap();
    assert_eq!(row.state, ShutdownOwnerStateV1::Closed);
    assert_eq!(row.retained_records, Some(0));
}

#[test]
fn shutdown_refuses_new_contexts_copies_and_holds_but_preserves_service_and_drain() {
    for capacity in [None, Some(1), Some(64)] {
        let (mut o, mut r, s) = configured(limits());
        let original = o
            .capture_source_read_context(&s, &facts(&s), &mut r)
            .unwrap();
        let hold_facts = SourceReadHoldFactsV1 {
            owner_control_allowed: Some(true),
            retained: Some(true),
            resources_ready: Some(true),
        };
        let hold = o
            .hold_source_read_context(&original, &s, &hold_facts, &mut r)
            .unwrap();
        install(&mut o, &mut r);
        r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let usage = o.source_read_context_usage();
        let funding = o.retention_usage();
        assert!(
            o.capture_source_read_context(&s, &facts(&s), &mut r)
                .is_none()
        );
        assert!(
            o.capture_retained_source_read_context(&original, &s, &facts(&s), &mut r)
                .is_none()
        );
        assert!(
            o.hold_source_read_context(&original, &s, &hold_facts, &mut r)
                .is_none()
        );
        assert_eq!(o.source_read_context_usage(), usage);
        assert_eq!(o.retention_usage(), funding);
        assert!(
            o.query_source_read_context(&original, &s, range(1), &facts(&s), &mut r)
                .is_some()
        );
        assert_eq!(
            o.configure_source_read_contexts("engine", "local", limits(), &controls(), &mut r),
            Ok(false)
        );
        assert!(o.stop_source_read_context(&original, &discharge(&s), &mut r));
        assert!(o.release_source_read_hold(&hold, &discharge(&s), &mut r));
        assert!(o.retire_source_read_context(&original, &discharge(&s), &mut r));
        o.retire_source_read_contexts(&controls(), &mut r).unwrap();
        assert!(o.source_read_context_usage().closed);
        assert!(o.retention_usage().reserved > 0); // The fence is still owned.
        assert!(!o.is_faulted());
        if capacity == Some(64) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for boundary in [
                Boundary::SourceContextCapture,
                Boundary::SourceContextRetainedCapture,
                Boundary::SourceContextHold,
            ] {
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == boundary && e.disposition == Disposition::Refused)
                );
            }
            assert!(
                events
                    .iter()
                    .any(|e| e.boundary == Boundary::SourceContextPoolRetirement
                        && e.disposition == Disposition::Completed)
            );
        } else if capacity == Some(1) {
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}

#[test]
fn shutdown_refuses_initial_source_context_pool_without_parent_or_issuance_changes() {
    let (mut o, mut r, _) = base();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    install(&mut o, &mut r);
    let funding = o.retention_usage();
    let usage = o.source_read_context_usage();
    assert_eq!(
        o.configure_source_read_contexts("engine", "local", limits(), &controls(), &mut r),
        Err(Error::Unavailable)
    );
    assert_eq!(o.source_read_context_usage(), usage);
    assert_eq!(o.retention_usage(), funding);
}

#[test]
fn shutdown_cannot_acquire_annotation_snapshot_pins_but_original_snapshot_remains_readable() {
    let (mut o, mut r, s) = configured(limits());
    o.annotations.limits = Some(AnnotationLimits {
        max_issued: 8,
        max_records: 8,
        max_body_bytes: 128,
        max_retained_body_bytes: 1024,
        max_mapping_steps: 4096,
        max_live_snapshots: 8,
        max_snapshot_bytes: 4096,
    });
    let source = o.read(&s.document, &mut r).unwrap().version;
    let original = o.capture_annotations(&source, 0, &mut r).unwrap();
    install(&mut o, &mut r);
    let usage = o.annotation_usage();
    assert!(matches!(
        o.capture_annotations(&source, 0, &mut r),
        Err(CoordinatorError::Unavailable)
    ));
    assert_eq!(o.annotation_usage(), usage);
    assert_eq!(original.source_text(), "a\r\nx");
    assert_eq!(original.source(), &source);
    assert!(!o.is_faulted());
}
