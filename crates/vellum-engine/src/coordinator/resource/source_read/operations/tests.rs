use super::*;
#[test]
fn shutdown_inventory_preserves_the_owned_source_registry_parent_envelope_and_native_task() {
    use crate::coordinator::shutdown::tests::inventory_owner;
    let (mut o, mut r, _, p) = super::super::super::serialization_tests::fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    o.configure_source_reader_v1(reader_limits(), &mut r)
        .unwrap();
    o.configure_source_read_operations_v1(limits(), &mut r)
        .unwrap();
    let c = crate::coordinator::serialization::tests::context();
    let target = p.source_read_target_v1(b"new").unwrap();
    o.request_owned_source_read_v1(&request(&target), &facts(&c), &mut r)
        .unwrap();
    let before = o.source_read_operation_usage_v1().unwrap();
    let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::OwnedSourceReads, &[]);
    assert_eq!(row.retained_records, Some(1));
    assert_eq!(
        row.reserved_bytes,
        Some(reserved(limits(), reader_limits().max_issued).unwrap())
    );
    assert_eq!(row.local_reserved_bytes, None);
    assert_eq!(o.source_read_operation_usage_v1().unwrap(), before);
}
#[test]
fn shutdown_source_operation_store_refuses_unrelated_zero_parent_credit() {
    let (mut o, mut r, _, _) = super::super::super::serialization_tests::fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    o.configure_source_reader_v1(reader_limits(), &mut r)
        .unwrap();
    o.configure_source_read_operations_v1(limits(), &mut r)
        .unwrap();
    o.stop_source_read_admission_v1(&mut r).unwrap();
    let substitute = o.retention.reserve(0).unwrap();
    let super::super::State::Active(reader) = &mut o.resources.source_reader else {
        panic!()
    };
    let State::Active(store) = &mut reader.operations else {
        panic!()
    };
    let original = std::mem::replace(&mut store.reservation, substitute);
    let held = o.retention_usage();
    assert_eq!(
        o.retire_source_read_operations_v1(Some(true), Some(true), Some(true), &mut r),
        Err(ResourceError::Faulted)
    );
    assert_eq!(o.retention_usage(), held);
    assert!(o.is_faulted());
    let super::super::State::Active(reader) = &mut o.resources.source_reader else {
        panic!()
    };
    let State::Active(store) = &mut reader.operations else {
        panic!()
    };
    let mut substitute = std::mem::replace(&mut store.reservation, original);
    o.retention.release(&mut substitute).unwrap();
    o.retire_source_read_operations_v1(Some(true), Some(true), Some(true), &mut r)
        .unwrap();
    assert!(o.is_faulted());
}
fn reader_limits() -> SourceReaderLimitsV1 {
    SourceReaderLimitsV1 {
        max_live: 2,
        max_issued: 4,
        max_input_bytes: 128,
        max_chunks: 8,
        max_held_bytes: 65536,
        max_store_bytes: 131072,
    }
}
fn limits() -> SourceReadOperationLimitsV1 {
    SourceReadOperationLimitsV1 {
        max_capture_bytes: 4096,
        max_reply_bytes: 512,
        max_label_bytes: 64,
        max_store_bytes: 65536,
    }
}
fn facts(c: &SettingsExecutionContextV1) -> TrustedSourceReadOperationFactsV1<'_> {
    TrustedSourceReadOperationFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        read_allowed: Some(true),
        retained: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
        resources_ready: Some(true),
        identity_allowed: Some(true),
        binding_coverage_complete: Some(true),
        provider_available: Some(true),
        launch_allowed: Some(true),
    }
}
fn request(target: &SourceReadTargetV1) -> OwnedSourceReadRequestV1<'_> {
    OwnedSourceReadRequestV1 {
        origin: SourceReadOperationOriginV1 {
            operation: b"load",
            effect: b"read",
            attempt: b"one",
        },
        target,
        strength: ReadStrength::CapturedRead,
    }
}
#[test]
fn exact_capsule_reply_registry_and_shared_retention_bounds_precede_child_issuance() {
    for field in 0..4 {
        for delta in [-1, 0, 1] {
            let (mut o, mut r, _, p) = super::super::super::serialization_tests::fixture();
            let c = crate::coordinator::serialization::tests::context();
            let target = p.source_read_target_v1(b"new").unwrap();
            let request = request(&target);
            let mut l = limits();
            let native = reader_limits();
            match field {
                0 => {
                    l.max_capture_bytes = capsule_bytes(&request, Some(&c))
                        .unwrap()
                        .checked_add_signed(delta)
                        .unwrap()
                }
                1 => {
                    l.max_reply_bytes = size_of::<SourceReadOperationAdmissionV1>()
                        .checked_add_signed(delta)
                        .unwrap()
                }
                2 => {
                    l.max_store_bytes = reserved(l, native.max_issued)
                        .unwrap()
                        .checked_add_signed(delta)
                        .unwrap()
                }
                _ => {}
            }
            let budget = if field == 3 {
                (native.reserved_bytes().unwrap() + reserved(l, native.max_issued).unwrap())
                    .checked_add_signed(delta)
                    .unwrap()
            } else {
                1048576
            };
            o.configure_retention_budget(budget, &mut r).unwrap();
            o.configure_source_reader_v1(native, &mut r).unwrap();
            let before = o.retention_usage();
            let configured = o.configure_source_read_operations_v1(l, &mut r);
            if field >= 2 && delta < 0 {
                assert_eq!(configured, Err(ResourceError::Limit));
                assert_eq!(o.retention_usage(), before);
                continue;
            }
            configured.unwrap();
            let funded = o.retention_usage();
            let result = o.request_owned_source_read_v1(&request, &facts(&c), &mut r);
            if field < 2 && delta < 0 {
                assert_eq!(result, Err(ResourceError::Limit));
                assert_eq!(o.source_reader_usage_v1().unwrap().issued, 0);
                assert_eq!(o.source_read_operation_usage_v1().unwrap().spent_origins, 0);
            } else {
                result.unwrap();
            }
            assert_eq!(o.retention_usage(), funded);
        }
    }
    let (mut o, mut r, _, _) = super::super::super::serialization_tests::fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    o.configure_source_reader_v1(reader_limits(), &mut r)
        .unwrap();
    let before = o.retention_usage();
    let mut l = limits();
    l.max_capture_bytes = usize::MAX;
    assert_eq!(
        o.configure_source_read_operations_v1(l, &mut r),
        Err(ResourceError::Limit)
    );
    assert_eq!(o.retention_usage(), before);
}
#[test]
fn matching_metadata_survives_root_fault_but_parent_child_drift_never_tombstones_or_refunds() {
    for field in 0..3 {
        let (mut o, mut r, _, p) = super::super::super::serialization_tests::fixture();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        o.configure_source_reader_v1(reader_limits(), &mut r)
            .unwrap();
        o.configure_source_read_operations_v1(limits(), &mut r)
            .unwrap();
        let c = crate::coordinator::serialization::tests::context();
        let target = p.source_read_target_v1(b"new").unwrap();
        let req = request(&target);
        let task = o
            .request_owned_source_read_v1(&req, &facts(&c), &mut r)
            .unwrap()
            .task;
        let funding = o.retention_usage();
        o.faulted = true;
        assert_eq!(
            o.request_owned_source_read_v1(&req, &facts(&c), &mut r)
                .unwrap()
                .task,
            task
        );
        o.faulted = false;
        if field == 0 {
            o.source_read_task_mut(&task).unwrap().operation = None;
        } else {
            let State::Active(s) = &mut o.source_reader_mut().unwrap().operations else {
                unreachable!()
            };
            if field == 1 {
                s.rows[0].task.slot = usize::MAX;
            } else {
                s.rows[0].capture.as_mut().unwrap().identity.key =
                    blob(b"wrong", 64, true).unwrap();
            }
        }
        assert_eq!(
            o.retire_source_read_v1(&task, &mut r),
            Err(ResourceError::Faulted)
        );
        assert!(o.is_faulted());
        assert!(o.source_read_task(&task).is_ok());
        assert_eq!(o.retention_usage(), funding);
        let State::Active(s) = &o.source_reader().unwrap().operations else {
            unreachable!()
        };
        assert!(s.rows[0].live && s.rows[0].capture.is_some());
    }
}
