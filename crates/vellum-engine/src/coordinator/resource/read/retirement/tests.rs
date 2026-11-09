use super::*;
use crate::coordinator::resource::shutdown_bindings::tests::{capture, read};
fn empty() -> (SourceCoordinator, ProviderPort, DocumentVersion, Recorder) {
    let (mut o, p, id, mut r) = super::super::tests::fixture();
    let d = o.read_task(&id).unwrap().source.clone();
    o.stop_annotation_read(&id, &mut r).unwrap();
    o.retire_annotation_read(&id, &mut r).unwrap();
    (o, p, d, r)
}
fn retire(o: &mut SourceCoordinator, r: &mut Recorder) -> Result<(), ResourceError> {
    o.retire_annotation_reader_v1(Some(true), Some(true), Some(true), r)
}

#[test]
fn whole_reader_retirement_is_absorbing_but_leaves_semantic_and_writer_services_available() {
    for mode in 0..4 {
        let (mut o, p, d, mut r) = empty();
        let _inert_restorer = o.register_annotation_restorer(&mut r).unwrap();
        let before = o.native_root_identity().unwrap();
        o.stop_annotation_read_admission_v1(&mut r);
        r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 64 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(
                crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
            );
        }
        retire(&mut o, &mut r).unwrap();
        assert!(o.resources.annotation_reader_closed);
        assert!(o.resources.restorers.is_empty());
        assert!(o.resources.reads.is_empty());
        assert_ne!(o.native_root_identity().unwrap(), before);
        let inventory = capture(&o, &mut r);
        let view = read(&o, &inventory, &mut r);
        let reader = view
            .owners
            .iter()
            .find(|v| v.owner.role() == ShutdownOwnerRoleV1::AnnotationReader)
            .unwrap();
        assert_eq!(reader.state, ShutdownOwnerStateV1::Closed);
        assert!(reader.retirement_supported);
        assert_eq!(reader.issued_slots, None);
        assert!(
            !view
                .dependencies
                .iter()
                .any(|v| v.dependent == ShutdownInventoryTargetV1::Owner(reader.owner.clone()))
        );
        assert!(
            view.owners
                .iter()
                .any(|v| v.owner.role() == ShutdownOwnerRoleV1::SharedWriter
                    && v.state == ShutdownOwnerStateV1::Active)
        );
        assert!(view.coverage.all_owner_obligations);
        assert!(matches!(
            o.register_annotation_restorer(&mut r),
            Err(ResourceError::Unavailable)
        ));
        assert_eq!(
            o.attach_annotation_read_baselines(&mut r),
            Err(ResourceError::Unavailable)
        );
        assert_eq!(
            o.admit_annotation_read(
                &d,
                0,
                p.annotation_read_target(b"comments").unwrap(),
                ReadStrength::CapturedRead,
                None,
                &mut r
            ),
            Err(ResourceError::Unavailable)
        );
        assert_eq!(
            o.configure_annotation_read_operations_v1(
                AnnotationReadOperationLimitsV1 {
                    max_capture_bytes: 8192,
                    max_reply_bytes: 1024,
                    max_label_bytes: 64,
                    max_store_bytes: 1048576,
                },
                &mut r
            ),
            Err(ResourceError::Unavailable)
        );
        assert_eq!(retire(&mut o, &mut r), Err(ResourceError::Unavailable));
        o.register_annotation_encoder(&mut r).unwrap();
        assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), "source");
        o.create(b"still editable", &mut r).unwrap();
        assert!(!o.is_faulted());
        if mode == 1 || mode == 2 {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            let retired = events
                .iter()
                .find(|e| {
                    e.boundary == Boundary::AnnotationReaderRetirement
                        && e.disposition == Disposition::Completed
                })
                .unwrap();
            assert_eq!(retired.service_time.is_some(), mode == 2);
            assert!(!format!("{events:?}").contains("editable"));
        } else if mode == 3 {
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}

#[test]
fn owner_close_needs_current_control_and_explicit_admission_stop_not_just_empty_rows() {
    let (mut o, _, _, mut r) = empty();
    assert!(retire(&mut o, &mut r).is_err());
    o.stop_annotation_read_admission_v1(&mut r);
    for index in 0..3 {
        for denied in [None, Some(false)] {
            let mut facts = [Some(true); 3];
            facts[index] = denied;
            let before = o.native_root_identity().unwrap();
            assert!(
                o.retire_annotation_reader_v1(facts[0], facts[1], facts[2], &mut r)
                    .is_err()
            );
            assert_eq!(o.native_root_identity().unwrap(), before);
            assert!(!o.resources.annotation_reader_closed);
        }
    }
    let before = o.native_root_identity().unwrap();
    retire(&mut o, &mut r).unwrap();
    assert_ne!(o.native_root_identity().unwrap(), before);
}

#[test]
fn live_dispatch_and_capture_pins_must_really_drain_before_the_owner_can_retire() {
    let (mut o, p, id, mut r) = super::super::tests::fixture();
    o.stop_annotation_read_admission_v1(&mut r);
    assert!(retire(&mut o, &mut r).is_err());
    let dispatch = o.submit_annotation_read(&id, None, &mut r).unwrap();
    for evidence in [
        p.annotation_read_start(&dispatch, b"v0").unwrap(),
        p.annotation_read_end(&dispatch, b"v0", 0, 0).unwrap(),
        p.annotation_read_quiescence(&dispatch).unwrap(),
    ] {
        o.record_annotation_read(evidence, &mut r).unwrap();
    }
    let held = o.annotation_read_capture(&id, None, &mut r).unwrap();
    assert_eq!(
        o.retire_annotation_read(&id, &mut r),
        Err(ResourceError::Retained)
    );
    assert!(retire(&mut o, &mut r).is_err());
    drop(dispatch);
    assert_eq!(
        o.retire_annotation_read(&id, &mut r),
        Err(ResourceError::Retained)
    );
    assert!(retire(&mut o, &mut r).is_err());
    drop(held);
    o.retire_annotation_read(&id, &mut r).unwrap();
    retire(&mut o, &mut r).unwrap();
}

#[test]
fn active_operation_store_requires_actual_parent_discharge_before_whole_reader_retirement() {
    let (mut o, _, _, mut r) = empty();
    o.configure_retention_budget(4194304, &mut r).unwrap();
    let _other = o.retention.reserve(73).unwrap();
    o.configure_annotation_read_operations_v1(
        AnnotationReadOperationLimitsV1 {
            max_capture_bytes: 8192,
            max_reply_bytes: 1024,
            max_label_bytes: 64,
            max_store_bytes: 1048576,
        },
        &mut r,
    )
    .unwrap();
    o.stop_annotation_read_admission_v1(&mut r);
    let before = o.retention_usage();
    assert!(retire(&mut o, &mut r).is_err());
    assert_eq!(o.retention_usage(), before);
    o.retire_annotation_read_operations_v1(Some(true), Some(true), Some(true), &mut r)
        .unwrap();
    let before = o.retention_usage();
    assert_eq!(before.reserved, 73);
    retire(&mut o, &mut r).unwrap();
    assert_eq!(o.retention_usage(), before);
}

#[test]
fn attachment_requires_explicit_both_endpoint_discharge_and_close_survives_fault_and_counter_exhaustion()
 {
    let (mut o, _, _, mut r) = empty();
    o.attach_annotation_read_baselines(&mut r).unwrap();
    o.stop_annotation_read_admission_v1(&mut r);
    let before = o.resource_usage();
    assert!(retire(&mut o, &mut r).is_err());
    assert_eq!(o.resource_usage(), before);
    o.detach_annotation_read_baselines(&mut r).unwrap();
    o.faulted = true;
    o.generation = u64::MAX;
    retire(&mut o, &mut r).unwrap();
    assert!(o.is_faulted());
    assert_eq!(o.generation(), u64::MAX);
    assert_eq!(o.resource_usage().held_read_bytes, 0);
    assert_eq!(o.resource_usage().held_artifact_bytes, 0);
}

#[test]
fn released_historical_bindings_do_not_resurrect_retired_producers_in_shutdown_reporting() {
    let (mut o, id, save, mut r) = super::super::tests::checkpointed_read_mode(false);
    let historical = capture(&o, &mut r);
    assert!(!read(&o, &historical, &mut r).bindings.is_empty());
    super::super::tests::release_checkpointed_root(&mut o, &id, &mut r);
    o.retire_disposed_annotation_read(&id, &mut r).unwrap();
    o.retire_save(&save, &mut r).unwrap();
    o.detach_annotation_read_baselines(&mut r).unwrap();
    o.stop_annotation_read_admission_v1(&mut r);
    retire(&mut o, &mut r).unwrap();
    let current = capture(&o, &mut r);
    let now = read(&o, &current, &mut r);
    assert!(now.reads.is_empty() && now.writes.is_empty() && now.artifacts.is_empty());
    assert!(now.bindings.is_empty());
    assert_eq!(
        now.documents[0].lifecycle.phase,
        LifecyclePhase::ProvenanceReleased
    );
    assert!(now.coverage.all_owner_obligations);
    // Original reports remain historical metadata; release does not query or
    // resurrect their now-retired original producer rows.
    assert!(!read(&o, &historical, &mut r).bindings.is_empty());
    assert!(!o.is_faulted());
}

#[test]
fn orphaned_reader_charge_faults_instead_of_being_erased_by_owner_close() {
    let (mut o, _, _, mut r) = empty();
    o.stop_annotation_read_admission_v1(&mut r);
    o.resources.held_read_bytes = 1;
    assert_eq!(retire(&mut o, &mut r), Err(ResourceError::Faulted));
    assert!(o.is_faulted());
    assert!(!o.resources.annotation_reader_closed);
    assert_eq!(o.resources.held_read_bytes, 1);
}
