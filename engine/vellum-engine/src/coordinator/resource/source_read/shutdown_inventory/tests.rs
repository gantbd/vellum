use super::*;
use crate::coordinator::resource::shutdown_bindings::tests::{capture, fence, read};

#[test]
fn child_completion_does_not_close_the_reader_or_erase_its_reservation() {
    let (mut o, mut r, _, p) = super::super::super::serialization_tests::fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let limits = super::super::tests::limits();
    o.configure_source_reader_v1(limits, &mut r).unwrap();
    let id = o
        .admit_source_read_v1(
            p.source_read_target_v1(b"new").unwrap(),
            ReadStrength::CapturedRead,
            &mut r,
        )
        .unwrap();
    let dispatch = o.submit_source_read_v1(&id, &mut r).unwrap();
    let version = SourceReadVersionV1::Opaque(b"v");
    for e in [
        p.source_read_start_v1(&dispatch, version).unwrap(),
        p.source_read_end_v1(&dispatch, version, 0, 0).unwrap(),
    ] {
        o.record_source_read_v1(e, &mut r).unwrap();
    }
    let cut = capture(&o, &mut r);
    let view = read(&o, &cut, &mut r);
    assert_eq!(view.reads[0].read, ShutdownReadIdV1::Source(id.clone()));
    assert_eq!(view.reads[0].terminal, ShutdownReadTerminalV1::End);
    assert!(!view.reads[0].quiescent);
    assert_eq!(view.reads[0].dispatch_evidence_references, 2);
    assert_eq!(
        o.retire_source_read_v1(&id, &mut r),
        Err(ResourceError::Retained)
    );
    o.record_source_read_v1(p.source_read_quiescence_v1(&dispatch).unwrap(), &mut r)
        .unwrap();
    drop(dispatch);
    o.retire_source_read_v1(&id, &mut r).unwrap();
    let empty = capture(&o, &mut r);
    let view = read(&o, &empty, &mut r);
    assert!(view.reads.is_empty());
    let owner = view
        .owners
        .iter()
        .find(|x| x.owner.role == ShutdownOwnerRoleV1::SourceReader)
        .unwrap();
    assert_eq!(owner.state, ShutdownOwnerStateV1::Active);
    assert_eq!(owner.issued_slots, Some(1));
    assert_eq!(owner.held_bytes, Some(0));
    assert_eq!(owner.reserved_bytes, Some(limits.reserved_bytes().unwrap()));
    o.stop_source_read_admission_v1(&mut r).unwrap();
    o.retire_source_reader_v1(Some(true), Some(true), Some(true), &mut r)
        .unwrap();
    let closed = capture(&o, &mut r);
    let view = read(&o, &closed, &mut r);
    let owner = view
        .owners
        .iter()
        .find(|x| x.owner.role == ShutdownOwnerRoleV1::SourceReader)
        .unwrap();
    assert_eq!(owner.state, ShutdownOwnerStateV1::Closed);
    assert_eq!(owner.held_bytes, None);
    assert_eq!(read(&o, &cut, &mut r).reads.len(), 1);
}

#[test]
fn fencing_refuses_new_reads_but_allows_existing_delivery_and_matching_configuration() {
    for configured in [false, true] {
        let (mut o, mut r, _, p) = super::super::super::serialization_tests::fixture();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        let limits = super::super::tests::limits();
        let existing = if configured {
            o.configure_source_reader_v1(limits, &mut r).unwrap();
            Some(
                o.admit_source_read_v1(
                    p.source_read_target_v1(b"new").unwrap(),
                    ReadStrength::CapturedRead,
                    &mut r,
                )
                .unwrap(),
            )
        } else {
            None
        };
        fence(&mut o, &mut r);
        let before = o.retention_usage();
        assert_eq!(
            o.configure_source_reader_v1(limits, &mut r),
            if configured {
                Ok(())
            } else {
                Err(ResourceError::Unavailable)
            }
        );
        assert_eq!(
            o.admit_source_read_v1(
                p.source_read_target_v1(b"new").unwrap(),
                ReadStrength::CapturedRead,
                &mut r
            ),
            Err(ResourceError::Unavailable)
        );
        if let Some(id) = existing {
            let dispatch = o.submit_source_read_v1(&id, &mut r).unwrap();
            o.record_source_read_v1(p.source_read_quiescence_v1(&dispatch).unwrap(), &mut r)
                .unwrap();
            drop(dispatch);
            o.retire_source_read_v1(&id, &mut r).unwrap();
        }
        assert_eq!(o.retention_usage(), before);
    }
}

#[test]
fn unrelated_parent_credit_cannot_substitute_for_the_actual_reader_reservation() {
    let (mut o, mut r, _, _) = super::super::super::serialization_tests::fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    o.configure_source_reader_v1(super::super::tests::limits(), &mut r)
        .unwrap();
    let unrelated = o.retention.reserve(0).unwrap();
    let State::Active(store) = &mut o.resources.source_reader else {
        panic!("active reader")
    };
    store.reservation = unrelated;
    assert_eq!(o.source_reader_usage_v1(), Err(ResourceError::Faulted));
}

#[test]
fn reader_operation_registry_acquisition_cannot_bypass_the_fence() {
    for configured in [false, true] {
        let (mut o, mut r, _, _) = super::super::super::serialization_tests::fixture();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        o.configure_source_reader_v1(super::super::tests::limits(), &mut r)
            .unwrap();
        let limits = SourceReadOperationLimitsV1 {
            max_capture_bytes: 4096,
            max_reply_bytes: 512,
            max_label_bytes: 64,
            max_store_bytes: 65536,
        };
        if configured {
            o.configure_source_read_operations_v1(limits, &mut r)
                .unwrap();
        }
        fence(&mut o, &mut r);
        let before = o.retention_usage();
        assert_eq!(
            o.configure_source_read_operations_v1(limits, &mut r),
            if configured {
                Ok(())
            } else {
                Err(ResourceError::Unavailable)
            }
        );
        assert_eq!(o.retention_usage(), before);
    }
}
