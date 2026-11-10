use super::*;
use crate::coordinator::shutdown::tests::{context, facts, limits as fence_limits};
use crate::resource::{Acknowledgment, Application, Protection, SaveRequirements, Synchronization};
fn limits() -> ShutdownInventoryLimitsV1 {
    ShutdownInventoryLimitsV1 {
        max_documents: 16,
        max_writes: 16,
        max_artifacts: 16,
        max_bindings: 16,
        max_reads: 16,
        max_owners: 64,
        max_attachments: 16,
        max_dependencies: 64,
        max_work: 256,
        max_capture_bytes: 65536,
    }
}

#[test]
fn post_fence_reports_preserve_unknown_effects_original_as_of_state_and_owner_obligations() {
    for capacity in [None, Some(1), Some(64)] {
        let (mut o, mut r, d, port) = resource::source_save_test_fixture();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        let save = o
            .queue_pinned_save(
                &d,
                SaveRequirements {
                    protection: Protection::BestEffort,
                    acknowledgment: Acknowledgment::Ordinary,
                },
                &mut r,
            )
            .unwrap();
        let c = context();
        let control = facts(&c);
        let f = TrustedShutdownInventoryFactsV1 {
            control: &control,
            inventory_readable: Some(true),
        };
        let basis = o
            .capture_shutdown_fence_basis_v1(fence_limits(), &control, &mut r)
            .unwrap();
        let fence = o
            .install_shutdown_fence_v1(&basis, &control, &mut r)
            .unwrap();
        let dispatch = o
            .dispatch_save(
                &save,
                port.read_result(b"key", b"v0", b"A").unwrap(),
                &mut r,
            )
            .unwrap();
        r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let before = o.resource_usage();
        let held = o.retention_usage();
        let original = o
            .capture_shutdown_progress_v1(&fence, limits(), &f, &mut r)
            .unwrap();
        assert_eq!(o.resource_usage(), before);
        assert_eq!(o.retention_usage(), held);
        let first = o.read_shutdown_progress_v1(&original, &f, &mut r).unwrap();
        assert_eq!(first.writes.len(), 1);
        assert_eq!(first.writes[0].write, save);
        assert_eq!(first.writes[0].application, Application::Unknown);
        assert!(!first.writes[0].no_future_effects);
        assert!(!first.writes[0].quiescent);
        assert!(first.coverage.all_owner_obligations); // Unknown effects are still inventoried obligations.
        for e in [
            port.no_future_effects(&dispatch).unwrap(),
            port.quiescence(&dispatch).unwrap(),
        ] {
            o.record_save_evidence(e, &mut r).unwrap();
        }
        let current = o
            .capture_shutdown_progress_v1(&fence, limits(), &f, &mut r)
            .unwrap();
        let now = o.read_shutdown_progress_v1(&current, &f, &mut r).unwrap();
        assert_eq!(now.writes[0].application, Application::Unknown);
        assert!(now.writes[0].no_future_effects);
        assert!(now.writes[0].quiescent);
        assert!(
            now.owners
                .iter()
                .any(|owner| owner.state == ShutdownOwnerStateV1::Active)
        );
        assert!(now.coverage.all_owner_obligations);
        assert!(
            !o.read_shutdown_progress_v1(&original, &f, &mut r)
                .unwrap()
                .writes[0]
                .quiescent
        );
        o.record_save_evidence(
            port.result(
                &dispatch,
                Application::Written,
                1,
                Synchronization::NotRequested,
                Some(b"v1"),
            )
            .unwrap(),
            &mut r,
        )
        .unwrap();
        o.publish_save_baseline(&save, &mut r).unwrap();
        let settled = o
            .capture_shutdown_progress_v1(&fence, limits(), &f, &mut r)
            .unwrap();
        let settled = o.read_shutdown_progress_v1(&settled, &f, &mut r).unwrap();
        assert_eq!(
            settled.writes[0].baseline_capacity,
            Some(crate::resource::BaselineCapacityState::Installed)
        );
        assert_eq!(settled.writes[0].application, Application::Written);
        assert!(settled.coverage.all_owner_obligations);
        assert_eq!(
            o.read_shutdown_progress_v1(&original, &f, &mut r)
                .unwrap()
                .writes[0]
                .application,
            Application::Unknown
        );
        assert!(!o.is_faulted());
        if capacity == Some(64) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert!(
                events
                    .iter()
                    .any(|e| e.boundary == Boundary::ShutdownProgressCapture
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
fn progress_refuses_foreign_fences_missing_roles_and_insufficient_record_byte_or_work_bounds() {
    let (mut o, mut r, _, _) = resource::source_save_test_fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let c = context();
    let control = facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let basis = o
        .capture_shutdown_fence_basis_v1(fence_limits(), &control, &mut r)
        .unwrap();
    let fence = o
        .install_shutdown_fence_v1(&basis, &control, &mut r)
        .unwrap();
    let before = o.native_root_identity().unwrap();
    let held = o.retention_usage();
    for category in 0..3 {
        let mut short = limits();
        match category {
            0 => short.max_documents = 0,
            1 => short.max_work = 0,
            _ => short.max_capture_bytes = size_of::<ShutdownProgressV1>() - 1,
        }
        assert!(matches!(
            o.capture_shutdown_progress_v1(&fence, short, &f, &mut r),
            Err(CoordinatorError::Limit)
        ));
        assert_eq!(o.native_root_identity().unwrap(), before);
        assert_eq!(o.retention_usage(), held);
    }
    let denied = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: None,
    };
    assert!(matches!(
        o.capture_shutdown_progress_v1(&fence, limits(), &denied, &mut r),
        Err(CoordinatorError::Unavailable)
    ));
    let (mut peer, mut pr, _, _) = resource::source_save_test_fixture();
    peer.configure_retention_budget(1048576, &mut pr).unwrap();
    let pb = peer
        .capture_shutdown_fence_basis_v1(fence_limits(), &control, &mut pr)
        .unwrap();
    let pf = peer
        .install_shutdown_fence_v1(&pb, &control, &mut pr)
        .unwrap();
    assert!(matches!(
        o.capture_shutdown_progress_v1(&pf, limits(), &f, &mut r),
        Err(CoordinatorError::Unavailable)
    ));
    assert_eq!(o.native_root_identity().unwrap(), before);
    assert_eq!(o.retention_usage(), held);
}

#[test]
fn historical_progress_release_is_separate_current_authority_and_does_not_heal_integrity_faults() {
    let (mut o, mut r, _, _) = resource::source_save_test_fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let c = context();
    let control = facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let basis = o
        .capture_shutdown_fence_basis_v1(fence_limits(), &control, &mut r)
        .unwrap();
    let fence = o
        .install_shutdown_fence_v1(&basis, &control, &mut r)
        .unwrap();
    let progress = o
        .capture_shutdown_progress_v1(&fence, limits(), &f, &mut r)
        .unwrap();
    let denied = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(false),
    };
    assert!(matches!(
        o.read_shutdown_progress_v1(&progress, &denied, &mut r),
        Err(CoordinatorError::Unavailable)
    ));
    o.faulted = true;
    assert!(matches!(
        o.capture_shutdown_progress_v1(&fence, limits(), &f, &mut r),
        Err(CoordinatorError::Faulted)
    ));
    assert!(o.read_shutdown_progress_v1(&progress, &f, &mut r).is_ok());
    assert!(o.is_faulted());
}
