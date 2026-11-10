use super::*;
#[test]
fn document_reclamation_waits_for_real_stopped_save_parent_artifact_and_once_only_refund() {
    let (mut o, mut r, d, p) = super::super::serialization_tests::fixture();
    let peer = o.create(b"unrelated", &mut r).unwrap();
    o.configure_save_operations_v1(
        SaveOperationLimitsV1 {
            max_issued: 4,
            max_label_bytes: 64,
            max_capture_bytes: 8192,
            max_reply_bytes: 1024,
            max_store_bytes: 65536,
        },
        &mut r,
    )
    .unwrap();
    let initial = o.retention_usage();
    let task = super::super::operations::test_admit_source_operation(&mut o, &mut r, &d, &p)
        .unwrap()
        .task;
    o.stop_save(&task, &mut r).unwrap();
    let plan = o.prepare_retained_disposal(&d, 0, &mut r).unwrap();
    let state = o.commit_retained_disposal(plan, &mut r).unwrap();
    o.release_disposed_source_binding(&d.document, state, &mut r)
        .unwrap();
    let state = o.lifecycle_state(&d.document, &mut r).unwrap();
    o.release_disposed_provenance(&d.document, state, &mut r)
        .unwrap();
    let state = o.lifecycle_state(&d.document, &mut r).unwrap();
    let pins = o.document_resource_pins_v1(&d.document, &mut r).unwrap();
    assert_eq!(
        (pins.write_tasks, pins.write_parents, pins.artifacts),
        (1, 1, 1)
    );
    let facts = TrustedDocumentReclamationV1 {
        authorized: Some(true),
        authored_disposition_ready: Some(true),
        session_references_released: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    let held = o.retention_usage();
    let generation = o.generation();
    assert_eq!(
        o.reclaim_disposed_document_v1(&d.document, state, &facts, &mut r),
        Err(CoordinatorError::Resource(ResourceError::Retained))
    );
    assert_eq!(o.retention_usage(), held);
    assert_eq!(o.generation(), generation);
    o.retire_save(&task, &mut r).unwrap();
    assert_eq!(o.retention_usage(), initial);
    assert_eq!(
        o.document_resource_pins_v1(&d.document, &mut r)
            .unwrap()
            .total(),
        Ok(0)
    );
    o.reclaim_disposed_document_v1(&d.document, state, &facts, &mut r)
        .unwrap();
    assert_eq!(o.retention_usage(), initial);
    assert!(o.retire_save(&task, &mut r).is_err());
    assert_eq!(o.retention_usage(), initial);
    assert_eq!(
        o.read(&peer.document, &mut r).unwrap().source.text(),
        "unrelated"
    );
}
#[test]
fn actual_write_artifact_parent_and_preparation_pins_drain_independently_per_document() {
    let (mut o, mut r, d, p) = super::super::serialization_tests::fixture();
    let other = o.create(b"unrelated", &mut r).unwrap();
    let initial = o.document_resource_pins_v1(&d.document, &mut r).unwrap();
    assert_eq!(initial.source_bindings, 1);
    assert_eq!(initial.total(), Ok(1));
    assert_eq!(
        o.document_resource_pins_v1(&other.document, &mut r)
            .unwrap()
            .total(),
        Ok(0)
    );
    let serialization = o.source_serialization(&d.document, &mut r).unwrap();
    let prepared = o
        .prepare_source_artifact(&d, serialization, &mut r)
        .unwrap();
    assert_eq!(
        o.document_resource_pins_v1(&d.document, &mut r)
            .unwrap()
            .source_preparations,
        1
    );
    o.discard_source_artifact_preparation(&prepared, &mut r)
        .unwrap();
    assert_eq!(
        o.document_resource_pins_v1(&d.document, &mut r).unwrap(),
        initial
    );
    o.configure_save_operations_v1(
        SaveOperationLimitsV1 {
            max_issued: 4,
            max_label_bytes: 64,
            max_capture_bytes: 8192,
            max_reply_bytes: 1024,
            max_store_bytes: 65536,
        },
        &mut r,
    )
    .unwrap();
    let task = super::super::operations::test_admit_source_operation(&mut o, &mut r, &d, &p)
        .unwrap()
        .task;
    let pins = o.document_resource_pins_v1(&d.document, &mut r).unwrap();
    assert_eq!(
        (pins.write_tasks, pins.write_parents, pins.artifacts),
        (1, 1, 1)
    );
    o.stop_save(&task, &mut r).unwrap();
    assert_eq!(
        o.document_resource_pins_v1(&d.document, &mut r).unwrap(),
        pins
    );
    o.retire_save(&task, &mut r).unwrap();
    assert_eq!(
        o.document_resource_pins_v1(&d.document, &mut r).unwrap(),
        initial
    );
    assert_eq!(
        o.document_resource_pins_v1(&other.document, &mut r)
            .unwrap()
            .total(),
        Ok(0)
    );
}
#[test]
fn pin_inventory_never_converts_foreign_owner_fault_or_charge_drift_to_empty() {
    let (mut o, mut r, d, _) = super::super::serialization_tests::fixture();
    let (_, _, foreign, _) = super::super::serialization_tests::fixture();
    assert_eq!(
        o.document_resource_pins_v1(&foreign.document, &mut r),
        Err(ResourceError::Unavailable)
    );
    o.faulted = true;
    assert_eq!(
        o.document_resource_pins_v1(&d.document, &mut r),
        Err(ResourceError::Faulted)
    );
    o.faulted = false;
    o.resources.held_artifact_bytes += 1;
    assert_eq!(
        o.document_resource_pins_v1(&d.document, &mut r),
        Err(ResourceError::Faulted)
    );
    assert_eq!(
        DocumentResourcePinsV1 {
            artifacts: usize::MAX,
            write_tasks: 1,
            ..Default::default()
        }
        .total(),
        Err(ResourceError::Faulted)
    );
}
#[test]
fn broken_owned_parent_child_correspondence_is_not_an_empty_pin_inventory() {
    let (mut o, mut r, d, p) = super::super::serialization_tests::fixture();
    o.configure_save_operations_v1(
        SaveOperationLimitsV1 {
            max_issued: 4,
            max_label_bytes: 64,
            max_capture_bytes: 8192,
            max_reply_bytes: 1024,
            max_store_bytes: 65536,
        },
        &mut r,
    )
    .unwrap();
    let id = super::super::operations::test_admit_source_operation(&mut o, &mut r, &d, &p)
        .unwrap()
        .task;
    o.resources.tasks[id.slot].as_mut().unwrap().operation = None;
    assert_eq!(
        o.document_resource_pins_v1(&d.document, &mut r),
        Err(ResourceError::Faulted)
    );
}
#[test]
fn optional_pin_diagnostics_do_not_change_counts_funding_or_native_state() {
    use crate::instrumentation::NativeTimingProfileV1;
    for capacity in [0, 1, 16] {
        for timing in [
            NativeTimingProfileV1::Off,
            NativeTimingProfileV1::ProcessMonotonicV1,
        ] {
            let (o, mut r, d, _) = super::super::serialization_tests::fixture();
            let baseline = o.document_resource_pins_v1(&d.document, &mut r).unwrap();
            let usage = o.resource_usage();
            let funding = o.retention_usage();
            let generation = o.generation();
            r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(timing);
            for _ in 0..3 {
                assert_eq!(
                    o.document_resource_pins_v1(&d.document, &mut r).unwrap(),
                    baseline
                );
            }
            assert_eq!(o.resource_usage(), usage);
            assert_eq!(o.retention_usage(), funding);
            assert_eq!(o.generation(), generation);
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert!(events.len() <= capacity);
            assert!(
                events
                    .iter()
                    .all(|v| v.boundary == Boundary::DocumentResourcePins
                        && v.service_time.is_some()
                            == (timing == NativeTimingProfileV1::ProcessMonotonicV1))
            );
            if capacity == 16 {
                assert_eq!(events.len(), 3);
            }
        }
    }
}
