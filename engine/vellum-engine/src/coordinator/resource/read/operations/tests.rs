use super::*;
#[test]
fn shutdown_inventory_distinguishes_an_empty_funded_annotation_registry_from_its_native_retirement()
{
    use crate::coordinator::shutdown::tests::inventory_owner;
    let (mut o, _, _, mut r) = raw();
    o.configure_retention_budget(4194304, &mut r).unwrap();
    o.configure_annotation_read_operations_v1(limits(), &mut r)
        .unwrap();
    let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::OwnedAnnotationReads, &[]);
    assert_eq!(row.state, ShutdownOwnerStateV1::Active);
    assert_eq!(row.retained_records, Some(0));
    assert_eq!(
        row.reserved_bytes,
        Some(reserved(limits(), o.resources.limits.unwrap()).unwrap())
    );
    o.stop_annotation_read_admission_v1(&mut r);
    o.retire_annotation_read_operations_v1(Some(true), Some(true), Some(true), &mut r)
        .unwrap();
    let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::OwnedAnnotationReads, &[]);
    assert_eq!(row.state, ShutdownOwnerStateV1::Closed);
    assert_eq!(row.reserved_bytes, None);
}

#[test]
fn annotation_operation_store_cannot_retire_against_unrelated_zero_parent_credit() {
    let (mut o, _, _, mut r) = raw();
    o.configure_retention_budget(4194304, &mut r).unwrap();
    o.configure_annotation_read_operations_v1(limits(), &mut r)
        .unwrap();
    o.stop_annotation_read_admission_v1(&mut r);
    let imposter = o.retention.reserve(0).unwrap();
    let State::Active(store) = &mut o.resources.read_operations else {
        panic!("active store")
    };
    let original = std::mem::replace(&mut store.reservation, imposter);
    let held = o.retention_usage();
    assert_eq!(
        o.retire_annotation_read_operations_v1(Some(true), Some(true), Some(true), &mut r),
        Err(ResourceError::Faulted)
    );
    assert_eq!(o.retention_usage(), held);
    assert!(matches!(o.resources.read_operations, State::Active(_)));
    assert!(o.is_faulted());
    // Restore the original private token, then demonstrate protected cleanup
    // releases its actual charge without clearing the editing integrity fault.
    let State::Active(store) = &mut o.resources.read_operations else {
        unreachable!()
    };
    let mut imposter = std::mem::replace(&mut store.reservation, original);
    o.retention.release(&mut imposter).unwrap();
    o.retire_annotation_read_operations_v1(Some(true), Some(true), Some(true), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    o.retire_annotation_reader_v1(Some(true), Some(true), Some(true), &mut r)
        .unwrap();
    assert!(o.is_faulted());
}
fn limits() -> AnnotationReadOperationLimitsV1 {
    AnnotationReadOperationLimitsV1 {
        max_capture_bytes: 8192,
        max_reply_bytes: 1024,
        max_label_bytes: 64,
        max_store_bytes: 1048576,
    }
}
pub(in crate::coordinator::resource) fn admit(
    o: &mut SourceCoordinator,
    d: &DocumentVersion,
    p: &ProviderPort,
    r: &mut Recorder,
) -> Result<AnnotationReadOperationAdmissionV1, ResourceError> {
    let a = o.current_annotation_association(&d.document).unwrap();
    let anno = SourceBindingTargetV1 {
        identity: a.binding.identity.clone(),
        registration: a.binding.registration.clone(),
    };
    let source = a.source_binding.as_ref().map(|b| SourceBindingTargetV1 {
        identity: b.identity.clone(),
        registration: b.registration.clone(),
    });
    let target = p.annotation_read_target(b"comments").unwrap();
    let c = crate::coordinator::serialization::tests::context();
    let f = TrustedAnnotationReadOperationFactsV1 {
        context: &c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        read_allowed: Some(true),
        annotation_readable: Some(true),
        retained: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
        resources_ready: Some(true),
        identity_allowed: Some(true),
        binding_coverage_complete: Some(true),
        provider_available: Some(true),
        launch_allowed: Some(true),
    };
    let req = OwnedAnnotationReadRequestV1 {
        origin: AnnotationReadOperationOriginV1 {
            operation: b"owned-load",
            effect: b"read",
            attempt: b"one",
        },
        source: d,
        expected_set: o.document(&d.document).unwrap().annotation_generation,
        annotation_binding: ExpectedAnnotationReadBindingV1 {
            target: &anno,
            generation: a.binding.generation,
        },
        source_binding: source.as_ref().map(|s| ExpectedAnnotationReadBindingV1 {
            target: s,
            generation: a.source_binding.as_ref().unwrap().generation,
        }),
        target: &target,
        strength: ReadStrength::CapturedRead,
    };
    o.request_owned_annotation_read_v1(&req, &f, None, r)
}
fn raw() -> (SourceCoordinator, ProviderPort, DocumentVersion, Recorder) {
    let (mut o, p, id, mut r) = super::super::tests::fixture();
    let d = o.read_task(&id).unwrap().source.clone();
    o.stop_annotation_read(&id, &mut r).unwrap();
    o.retire_annotation_read(&id, &mut r).unwrap();
    (o, p, d, r)
}
#[test]
fn exact_parent_reply_and_combined_sponsorship_bounds_precede_native_read_issuance() {
    let (mut probe, p, d, mut r) = raw();
    let c = crate::coordinator::serialization::tests::context();
    let a = probe.current_annotation_association(&d.document).unwrap();
    let target = p.annotation_read_target(b"comments").unwrap();
    let exact = bytes(10, 4, 3, &target.identity, &a, &c).unwrap();
    for field in 0..3 {
        for delta in [-1, 0, 1] {
            let (mut o, p, d, mut r) = raw();
            let mut l = limits();
            let needed = match field {
                0 => exact,
                1 => std::mem::size_of::<AnnotationReadOperationAdmissionV1>(),
                _ => reserved(l, o.resources.limits.unwrap()).unwrap(),
            };
            let bound = needed.checked_add_signed(delta).unwrap();
            match field {
                0 => l.max_capture_bytes = bound,
                1 => l.max_reply_bytes = bound,
                _ => l.max_store_bytes = bound,
            }
            o.configure_retention_budget(2097152, &mut r).unwrap();
            let before = o.retention_usage();
            let configured = o.configure_annotation_read_operations_v1(l, &mut r);
            if field == 2 && delta < 0 {
                assert_eq!(configured, Err(ResourceError::Limit));
                assert_eq!(o.retention_usage(), before);
            } else {
                configured.unwrap();
                let before = o.resource_usage();
                let funding = o.retention_usage();
                let out = admit(&mut o, &d, &p, &mut r);
                if delta < 0 {
                    assert_eq!(out, Err(ResourceError::Limit));
                    assert_eq!(o.resource_usage(), before);
                    assert_eq!(o.retention_usage(), funding);
                } else {
                    out.unwrap();
                    assert_eq!(o.annotation_read_operation_usage_v1().unwrap().owned, 1);
                }
            }
        }
    }
    let need = reserved(limits(), probe.resources.limits.unwrap()).unwrap();
    probe.configure_retention_budget(need - 1, &mut r).unwrap();
    assert_eq!(
        probe.configure_annotation_read_operations_v1(limits(), &mut r),
        Err(ResourceError::Limit)
    );
    assert_eq!(probe.retention_usage().reserved, 0);
    let mut overflow = limits();
    overflow.max_capture_bytes = usize::MAX;
    assert_eq!(
        reserved(overflow, probe.resources.limits.unwrap()),
        Err(ResourceError::Limit)
    );
}
#[test]
fn read_parent_origin_or_child_drift_faults_without_tombstoning_or_returning_sponsorship() {
    for mode in 0..3 {
        let (mut o, p, d, mut r) = raw();
        o.configure_retention_budget(2097152, &mut r).unwrap();
        o.configure_annotation_read_operations_v1(limits(), &mut r)
            .unwrap();
        let id = admit(&mut o, &d, &p, &mut r).unwrap().task;
        o.stop_annotation_read(&id, &mut r).unwrap();
        match mode {
            0 => o.resources.reads[id.slot].as_mut().unwrap().operation = None,
            1 => {
                let State::Active(s) = &mut o.resources.read_operations else {
                    unreachable!()
                };
                s.origins[0].effect = blob(b"changed", 64, true).unwrap();
            }
            _ => {
                let State::Active(s) = &mut o.resources.read_operations else {
                    unreachable!()
                };
                s.rows[0].capture.as_mut().unwrap().set += 1;
            }
        }
        let before = o.resource_usage();
        let funding = o.retention_usage();
        assert_eq!(
            o.retire_annotation_read(&id, &mut r),
            Err(ResourceError::Faulted)
        );
        assert!(o.is_faulted());
        assert_eq!(o.resource_usage(), before);
        assert_eq!(o.retention_usage(), funding);
        let State::Active(s) = &o.resources.read_operations else {
            unreachable!()
        };
        assert!(s.rows[0].capture.is_some());
    }
}
#[test]
fn actual_superseded_and_disposed_read_cleanup_spends_the_coowned_parent_only_after_real_pin_drain()
{
    for disposed in [false, true] {
        let (mut o, id, _, mut r) = super::super::tests::checkpointed_read_mode(true);
        if disposed {
            super::super::tests::release_checkpointed_root(&mut o, &id, &mut r);
        }
        let held = o
            .read_task(&id)
            .unwrap()
            .restoration
            .as_ref()
            .unwrap()
            .package
            .capture
            .clone();
        o.faulted = true;
        o.generation = u64::MAX;
        let before = o.annotation_read_operation_usage_v1().unwrap();
        let funding = o.retention_usage();
        let out = if disposed {
            o.retire_disposed_annotation_read(&id, &mut r)
        } else {
            o.retire_superseded_annotation_read(&id, &mut r)
        };
        assert_eq!(out, Err(ResourceError::Retained));
        assert_eq!(o.annotation_read_operation_usage_v1().unwrap(), before);
        drop(held);
        if disposed {
            o.retire_disposed_annotation_read(&id, &mut r).unwrap();
        } else {
            o.retire_superseded_annotation_read(&id, &mut r).unwrap();
        }
        let after = o.annotation_read_operation_usage_v1().unwrap();
        assert_eq!((after.owned, after.retired, after.spent_origins), (0, 1, 1));
        assert_eq!(o.retention_usage(), funding);
        assert_eq!(o.generation, u64::MAX);
        assert!(o.is_faulted());
    }
}
