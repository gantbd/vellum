use super::*;
use crate::coordinator::shutdown::tests::install;
#[test]
fn shutdown_inventory_reports_native_preparation_context_and_hold_owners_without_new_pins() {
    use crate::coordinator::shutdown::tests::{inspect, inspect_read};
    let (mut o, mut r, _, s, g) = ready(limits());
    o.configure_annotation_contexts(profile(), &control(), &mut r)
        .unwrap();
    let prepared = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Combined,
            &s,
            &facts(&s, &g),
            &mut r,
        )
        .unwrap();
    let ctx = o
        .install_annotation_context(&prepared, &s, &current(&prepared, &s), &mut r)
        .unwrap()
        .context;
    let _hold = o
        .hold_annotation_search_source(&ctx, &s, &eligible(&ctx, &s), &mut r)
        .unwrap();
    let _pending = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Combined,
            &s,
            &facts(&s, &g),
            &mut r,
        )
        .unwrap();
    let before = (
        o.annotation_preparation_usage(),
        o.annotation_context_usage(),
        o.retention_usage(),
    );
    let cut = inspect(&o, &mut r);
    let view = inspect_read(&o, &cut, &mut r);
    for (role, retained, charge) in [
        (
            ShutdownOwnerRoleV1::AnnotationPreparations,
            1,
            before.0.reserved_pool_bytes,
        ),
        (
            ShutdownOwnerRoleV1::AnnotationContexts,
            2,
            before.1.reserved_pool_bytes,
        ),
    ] {
        let row = view.owners.iter().find(|r| r.owner.role() == role).unwrap();
        assert_eq!(row.state, ShutdownOwnerStateV1::Active);
        assert_eq!(row.retained_records, Some(retained));
        assert_eq!(row.reserved_bytes, Some(charge));
        assert!(view.dependencies.iter().any(|e| e.dependent
            == ShutdownInventoryTargetV1::Owner(row.owner.clone())
            && e.required == ShutdownInventoryTargetV1::Document(s.document.clone())));
    }
    assert_eq!(
        (
            o.annotation_preparation_usage(),
            o.annotation_context_usage(),
            o.retention_usage()
        ),
        before
    );
}

#[test]
fn shutdown_blocks_combined_capture_install_copy_and_hold_without_consuming_prepared_work() {
    for capacity in [None, Some(1), Some(64)] {
        let (mut o, mut r, _, s, g) = ready(limits());
        o.configure_annotation_contexts(profile(), &control(), &mut r)
            .unwrap();
        let classes = AnnotationProjectionClassesV1::Combined;
        let first = o
            .prepare_annotation_projection(classes, &s, &facts(&s, &g), &mut r)
            .unwrap();
        let original = o
            .install_annotation_context(&first, &s, &current(&first, &s), &mut r)
            .unwrap()
            .context;
        let hold = o
            .hold_annotation_search_source(&original, &s, &eligible(&original, &s), &mut r)
            .unwrap();
        let pending = o
            .prepare_annotation_projection(classes, &s, &facts(&s, &g), &mut r)
            .unwrap();
        install(&mut o, &mut r);
        r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let before = (
            o.annotation_preparation_usage(),
            o.annotation_context_usage(),
            o.retention_usage(),
        );
        assert!(
            o.prepare_annotation_projection(classes, &s, &facts(&s, &g), &mut r)
                .is_none()
        );
        assert!(
            o.install_annotation_context(&pending, &s, &current(&pending, &s), &mut r)
                .is_none()
        );
        assert!(
            o.capture_retained_annotation_context(
                &original,
                classes,
                &s,
                &eligible(&original, &s),
                Some(true),
                &mut r
            )
            .is_none()
        );
        assert!(matches!(
            o.hold_annotation_search_source(&original, &s, &eligible(&original, &s), &mut r),
            Err(Error::Unavailable)
        ));
        assert_eq!(
            (
                o.annotation_preparation_usage(),
                o.annotation_context_usage(),
                o.retention_usage()
            ),
            before
        );
        assert!(o.prepared_annotation_cut(&pending, &s).is_ok());
        assert!(
            o.query_annotation_context_source(
                &original,
                &s,
                SourceContextQueryV1::Range {
                    start: crate::coordinates::BytePosition(0),
                    end: crate::coordinates::BytePosition(1)
                },
                &eligible(&original, &s),
                &mut r
            )
            .is_some()
        );
        assert_eq!(
            o.configure_annotation_preparation("engine", "local", limits(), &control(), &mut r),
            Ok(false)
        );
        assert_eq!(
            o.configure_annotation_contexts(profile(), &control(), &mut r),
            Ok(false)
        );
        assert!(o.discard_prepared_annotation_projection(&pending, &cleanup(&s), &mut r));
        assert!(o.stop_annotation_context(&original, &cleanup(&s), &mut r));
        o.release_annotation_source_hold(&hold, &cleanup(&s))
            .unwrap();
        assert!(o.retire_annotation_context(&original, &cleanup(&s), &mut r));
        assert_eq!(o.retire_annotation_contexts(&control(), &mut r), Ok(true));
        assert_eq!(
            o.retire_annotation_preparation(&control(), &mut r),
            Ok(true)
        );
        assert!(o.retention_usage().reserved > 0);
        assert!(!o.is_faulted());
        if capacity == Some(64) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for boundary in [
                Boundary::AnnotationProjectionPreparation,
                Boundary::AnnotationContextInstallation,
                Boundary::AnnotationContextRetainedCapture,
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
                    .any(|e| e.boundary == Boundary::AnnotationContextPoolRetirement
                        && e.disposition == Disposition::Completed)
            );
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
        if capacity.is_none() {
            assert_eq!(r.queued(), 0);
        }
    }
}

#[test]
fn shutdown_refuses_new_annotation_preparation_and_installed_pools_without_funding_changes() {
    for prepared in [false, true] {
        let (mut o, mut r, _, _, _) = if prepared { ready(limits()) } else { base() };
        if !prepared {
            o.configure_retention_budget(1048576, &mut r).unwrap();
        }
        install(&mut o, &mut r);
        let before = (
            o.annotation_preparation_usage(),
            o.annotation_context_usage(),
            o.retention_usage(),
        );
        assert_eq!(
            o.configure_annotation_preparation("engine", "local", limits(), &control(), &mut r),
            if prepared {
                Ok(false)
            } else {
                Err(Error::Unavailable)
            }
        );
        assert_eq!(
            o.configure_annotation_contexts(profile(), &control(), &mut r),
            Err(Error::Unavailable)
        );
        assert_eq!(
            (
                o.annotation_preparation_usage(),
                o.annotation_context_usage(),
                o.retention_usage()
            ),
            before
        );
    }
}
