use super::*;
#[test]
fn shutdown_inventory_distinguishes_owned_save_local_credit_from_shared_parent_reservation() {
    use crate::coordinator::shutdown::tests::inventory_owner;
    let (mut o, mut r, d, p) = super::super::serialization_tests::fixture();
    o.configure_save_operations_v1(limits(), &mut r).unwrap();
    admit(&mut o, &mut r, &d, &p).unwrap();
    let before = o.save_operation_usage_v1();
    let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::OwnedSaves, &[&d.document]);
    assert_eq!(row.retained_records, Some(1));
    assert_eq!(row.reserved_bytes, None);
    assert_eq!(row.local_reserved_bytes, Some(before.reserved_store_bytes));
    assert_eq!(o.save_operation_usage_v1(), before);
}
pub(in crate::coordinator::resource) fn source_capsule_bytes_for_test(
    c: &SettingsExecutionContextV1,
) -> usize {
    capsule_bytes(2, 1, 1, 3, c).unwrap()
}
fn limits() -> SaveOperationLimitsV1 {
    SaveOperationLimitsV1 {
        max_issued: 8,
        max_label_bytes: 64,
        max_capture_bytes: 8192,
        max_reply_bytes: 1024,
        max_store_bytes: 131072,
    }
}
#[test]
fn source_admission_interruption_fences_artifact_child_and_parent_without_a_success_reply() {
    use crate::instrumentation::{Disposition, NativeTimingProfileV1};
    for cut in 0..5 {
        let (mut o, mut r, d, p) = super::super::serialization_tests::fixture();
        o.configure_save_operations_v1(limits(), &mut r).unwrap();
        r = Recorder::try_with_capacity(128)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let tasks = o.resources.tasks.len();
        let artifacts = o.resources.artifacts.len();
        let funding = o.save_operation_usage_v1().reserved_store_bytes;
        o.publication_fault_after = Some(cut);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| admit(
                &mut o, &mut r, &d, &p
            )))
            .is_err()
        );
        assert!(o.is_faulted());
        assert_eq!(
            o.resources.artifacts.len(),
            artifacts + usize::from(cut >= 1)
        );
        assert_eq!(o.resources.tasks.len(), tasks + usize::from(cut >= 2));
        assert_eq!(o.save_operation_usage_v1().issued, usize::from(cut >= 4));
        assert_eq!(o.save_operation_usage_v1().reserved_store_bytes, funding);
        if cut >= 2 {
            let task = o.resources.tasks[tasks].as_ref().unwrap();
            assert_eq!(task.operation, Some(0));
            assert_eq!(task.phase, SavePhase::Queued);
            assert!(!task.no_future);
            assert!(!task.quiescent);
        }
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        for boundary in [Boundary::SaveAdmission, Boundary::SaveOperationAdmission] {
            assert!(events.iter().any(|e| e.boundary == boundary
                && e.disposition == Disposition::Unknown
                && e.service_time.is_some()));
            assert!(
                !events
                    .iter()
                    .any(|e| e.boundary == boundary && e.disposition == Disposition::Completed)
            );
        }
        assert_eq!(
            o.create(b"must refuse", &mut r),
            Err(CoordinatorError::Faulted)
        );
    }
}
fn release(c: &SettingsExecutionContextV1) -> TrustedSaveOperationReleaseV1<'_> {
    TrustedSaveOperationReleaseV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        write_allowed: Some(true),
        source_readable: Some(true),
        disposition_readable: Some(true),
        retained: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
        resources_ready: Some(true),
    }
}
fn common() -> TrustedSourceExportFactsV1 {
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
        destination_allowed: None,
        identity_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        create_allowed: None,
        overwrite_allowed: None,
    }
}
pub(in crate::coordinator) fn admit(
    o: &mut SourceCoordinator,
    r: &mut Recorder,
    d: &DocumentVersion,
    p: &ProviderPort,
) -> Result<SaveOperationAdmissionV1, ResourceError> {
    let c = super::super::super::serialization::tests::context();
    let common = common();
    let a = TrustedOrdinarySourceSaveAdmissionV1 {
        current: &common,
        owner_control_allowed: Some(true),
        preparation_reserved: Some(true),
        ordinary_bindings_current: Some(true),
        ordinary_identity_current: Some(true),
    };
    o.request_owned_source_save_v1(
        &OwnedSourceSaveRequestV1 {
            origin: SaveOperationOriginV1 {
                operation: b"op",
                effect: b"e",
                attempt: b"a",
            },
            source: d,
            expected_serialization: 1,
            expected_binding: 0,
            target: &p.source_binding_target(b"key").unwrap(),
            requirements: SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            condition: SaveOperationConditionV1::Pinned,
        },
        &release(&c),
        Some(&a),
        None,
        r,
    )
}
#[test]
fn exact_capture_reply_and_lifetime_store_boundaries_precede_child_issuance() {
    let c = super::super::super::serialization::tests::context();
    let capture = capsule_bytes(2, 1, 1, 3, &c).unwrap();
    for field in 0..3 {
        for delta in [-1, 0, 1] {
            let (mut o, mut r, d, p) = super::super::serialization_tests::fixture();
            let mut l = limits();
            let exact = match field {
                0 => capture,
                1 => std::mem::size_of::<SaveOperationAdmissionV1>(),
                _ => std::mem::size_of::<State>() + l.max_issued * l.max_capture_bytes,
            };
            let bound = exact.checked_add_signed(delta).unwrap();
            match field {
                0 => l.max_capture_bytes = bound,
                1 => l.max_reply_bytes = bound,
                _ => l.max_store_bytes = bound,
            }
            let configured = o.configure_save_operations_v1(l, &mut r);
            if field == 2 && delta < 0 {
                assert_eq!(configured, Err(ResourceError::Limit));
                assert!(o.resources.operations.limits.is_none());
            } else {
                configured.unwrap();
                let before = o.resource_usage();
                let parent = o.save_operation_usage_v1();
                let out = admit(&mut o, &mut r, &d, &p);
                if delta < 0 {
                    assert_eq!(out, Err(ResourceError::Limit));
                    assert_eq!(o.resource_usage(), before);
                    assert_eq!(o.save_operation_usage_v1(), parent);
                } else {
                    out.unwrap();
                }
            }
        }
    }
    let (mut o, mut r, _, _) = super::super::serialization_tests::fixture();
    let mut l = limits();
    l.max_capture_bytes = usize::MAX;
    l.max_store_bytes = usize::MAX;
    assert_eq!(
        o.configure_save_operations_v1(l, &mut r),
        Err(ResourceError::Limit)
    );
    assert_eq!(o.save_operation_usage_v1().reserved_store_bytes, 0);
}
#[test]
fn parent_child_or_reservation_drift_faults_without_erasing_retained_obligations() {
    for mode in 0..3 {
        let (mut o, mut r, d, p) = super::super::serialization_tests::fixture();
        o.configure_save_operations_v1(limits(), &mut r).unwrap();
        let id = admit(&mut o, &mut r, &d, &p).unwrap().task;
        o.stop_save(&id, &mut r).unwrap();
        match mode {
            0 => o.resources.tasks[id.slot].as_mut().unwrap().operation = None,
            1 => {
                o.resources.operations.rows[0]
                    .capture
                    .as_mut()
                    .unwrap()
                    .binding += 1
            }
            _ => o.resources.operations.reserved -= 1,
        }
        let before = o.resource_usage();
        let parent = o.save_operation_usage_v1();
        assert_eq!(o.retire_save(&id, &mut r), Err(ResourceError::Faulted));
        assert!(o.is_faulted());
        assert_eq!(o.resource_usage(), before);
        assert_eq!(o.save_operation_usage_v1(), parent);
    }
    let (mut o, mut r, d, _) = super::super::serialization_tests::fixture();
    let id = o
        .queue_pinned_save(
            &d,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            &mut r,
        )
        .unwrap();
    o.stop_save(&id, &mut r).unwrap();
    o.resources.tasks[id.slot].as_mut().unwrap().operation = Some(0);
    let before = o.resource_usage();
    assert_eq!(o.retire_save(&id, &mut r), Err(ResourceError::Faulted));
    assert!(o.is_faulted());
    assert_eq!(o.resource_usage(), before);
}
