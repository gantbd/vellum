use super::*;
#[test]
fn shutdown_inventory_tracks_original_invocation_phase_result_loss_and_retirement_without_republication()
 {
    use crate::coordinator::shutdown::tests::{
        inspect, inspect_read, inspection_current, inventory_owner,
    };
    let (mut o, mut r) = owner();
    let a = o.create(b"a", &mut r).unwrap();
    let b = o.create(b"b", &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit(
        &id,
        &SourceIntent {
            writes: vec![DocumentEdits {
                expected: a.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "a".into(),
                    inserted: "A".into(),
                }],
            }],
            reads: vec![b.clone()],
        },
        &mut r,
    )
    .unwrap();
    let old = inspect(&o, &mut r);
    let generation = o.generation();
    let held = o.retention_usage();
    let row = inventory_owner(
        &o,
        &mut r,
        ShutdownOwnerRoleV1::Invocations,
        &[&a.document, &b.document],
    );
    assert_eq!(row.retained_records, Some(1));
    assert_eq!(row.reserved_bytes, None);
    o.stop_invocation(&id, &mut r).unwrap();
    assert_eq!(
        inspection_current(&o, &old, &mut r),
        Err(CoordinatorError::Stale)
    );
    let now = inspect(&o, &mut r);
    let now = inspect_read(&o, &now, &mut r);
    assert_eq!(now.invocations[0].state, Ok(InvocationState::Finished));
    assert!(now.invocations[0].outcome_available && now.invocations[0].intent_bound);
    assert_eq!(
        inspect_read(&o, &old, &mut r).invocations[0].state,
        Ok(InvocationState::Pending)
    );
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    let lost = inspect(&o, &mut r);
    let lost = inspect_read(&o, &lost, &mut r);
    assert!(!lost.invocations[0].outcome_available);
    assert!(lost.invocations[0].intent_bound);
    o.retire_invocation(&id, &mut r).unwrap();
    let gone = inspect(&o, &mut r);
    assert!(inspect_read(&o, &gone, &mut r).invocations.is_empty());
    let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::Invocations, &[]);
    assert_eq!(row.state, ShutdownOwnerStateV1::Active);
    assert_eq!(row.retained_records, Some(0));
    assert_eq!(row.issued_slots, Some(1));
    assert_eq!((o.generation(), o.retention_usage()), (generation, held));
}

#[test]
fn shutdown_stop_rechecks_control_and_original_identity_and_preserves_a_committed_outcome() {
    use crate::coordinator::shutdown::tests::{context, facts, limits};
    let (mut o, mut r) = owner();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let d = o.create(b"a", &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit(
        &id,
        &SourceIntent {
            writes: vec![DocumentEdits {
                expected: d,
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "a".into(),
                    inserted: "A".into(),
                }],
            }],
            reads: vec![],
        },
        &mut r,
    )
    .unwrap();
    o.run(&id, &mut r).unwrap();
    let unsubmitted = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let f = facts(&c);
    let basis = o
        .capture_shutdown_fence_basis_v1(limits(), &f, &mut r)
        .unwrap();
    let fence = o.install_shutdown_fence_v1(&basis, &f, &mut r).unwrap();
    let before = (o.generation(), o.invocation_usage(), o.retention_usage());
    for allowed in [None, Some(false)] {
        let mut denied = facts(&c);
        denied.owner_control_allowed = allowed;
        assert_eq!(
            o.stop_shutdown_invocation_v1(&fence, &id, &denied, &mut r),
            Err(CoordinatorError::Unavailable)
        );
    }
    let (mut foreign, mut fr) = owner();
    let foreign_id = foreign.issue_invocation(&mut fr).unwrap();
    assert_eq!(
        o.stop_shutdown_invocation_v1(&fence, &foreign_id, &f, &mut r),
        Err(CoordinatorError::Unavailable)
    );
    assert_eq!(
        o.stop_shutdown_invocation_v1(&fence, &unsubmitted, &f, &mut r),
        Err(CoordinatorError::Unavailable)
    );
    o.stop_shutdown_invocation_v1(&fence, &id, &f, &mut r)
        .unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r),
        Ok(InvocationOutcome::Committed(_))
    ));
    assert_eq!(
        (o.generation(), o.invocation_usage(), o.retention_usage()),
        before
    );
    assert!(!o.is_faulted());
}
use crate::instrumentation::NativeTimingProfileV1;
fn owner() -> (SourceCoordinator, Recorder) {
    (
        SourceCoordinator::with_invocation_limits(
            CoordinatorLimits {
                max_document_incarnations: 8,
                max_participants: 8,
                max_staged_bytes: 4096,
                source: crate::source::Limits {
                    max_source_bytes: 4096,
                    max_edits: 8,
                    max_edit_payload_bytes: 4096,
                },
            },
            InvocationLimits {
                max_lifetime_issued: 16,
                max_retained: 8,
                max_intent_bytes: 32768,
            },
        ),
        Recorder::disabled(),
    )
}
#[test]
fn invocation_document_pins_follow_original_read_write_inputs_through_forget_and_retire() {
    let (mut o, mut r) = owner();
    let a = o.create(b"a", &mut r).unwrap();
    let b = o.create(b"b", &mut r).unwrap();
    let peer = o.create(b"peer", &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    assert_eq!(o.invocations.document_pins(&a.document), Ok(0));
    let intent = SourceIntent {
        writes: vec![DocumentEdits {
            expected: a.clone(),
            edits: vec![Replacement {
                start: 0,
                end: 1,
                expected: "a".into(),
                inserted: "A".into(),
            }],
        }],
        reads: vec![b.clone()],
    };
    o.submit(&id, &intent, &mut r).unwrap();
    for d in [&a.document, &b.document] {
        assert_eq!(o.invocations.document_pins(d), Ok(1));
    }
    assert_eq!(o.invocations.document_pins(&peer.document), Ok(0));
    o.run(&id, &mut r).unwrap();
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    let generation = o.generation();
    let held = o.retention_usage();
    // Forgetting a result does not forget the original input/replay seal.
    for timing in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        for capacity in [0, 1, 16] {
            let mut capture = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(timing);
            assert_eq!(
                o.document_context_pins_v1(&a.document, &mut capture)
                    .unwrap()
                    .retained_invocations,
                1
            );
            assert_eq!(
                o.document_context_pins_v1(&b.document, &mut capture)
                    .unwrap()
                    .retained_invocations,
                1
            );
        }
    }
    assert_eq!(o.generation(), generation);
    assert_eq!(o.retention_usage(), held);
    o.retire_invocation(&id, &mut r).unwrap();
    assert_eq!(o.invocations.document_pins(&a.document), Ok(0));
    assert_eq!(o.invocations.document_pins(&b.document), Ok(0));
}
#[test]
fn invocation_document_pins_keep_original_view_source_after_detach_and_reattach() {
    use crate::attachment::Affinity;
    use crate::coordinates::BytePosition;
    use crate::selection::{Selection, SelectionBudget};
    let (mut o, mut r) = owner();
    o.configure_logical_views(
        LogicalViewLimits {
            max_live_views: 4,
            max_issued_views: 8,
            max_total_selections: 16,
            normalization: SelectionBudget {
                max_input_selections: 8,
                max_output_selections: 8,
                max_work: 1024,
            },
            max_mapping_steps: 1024,
        },
        &mut r,
    )
    .unwrap();
    let a = o.create(b"a", &mut r).unwrap();
    let b = o.create(b"b", &mut r).unwrap();
    let view = o.create_logical_view(&mut r).unwrap();
    let stamp = o.logical_view(&view, &mut r).unwrap().stamp;
    let selections = [Selection {
        anchor: BytePosition(0),
        head: BytePosition(0),
        affinity: Affinity::After,
    }];
    o.attach_logical_view(&view, stamp, &a, &selections, 0, &mut r)
        .unwrap();
    let context = o.logical_view(&view, &mut r).unwrap().context().unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_literal_edit(&id, &context, "x", SelectionAfterV1::CaretsAfter, &mut r)
        .unwrap();
    let detached = o.detach_logical_view(&view, context.stamp, &mut r).unwrap();
    o.attach_logical_view(&view, detached.stamp, &b, &selections, 0, &mut r)
        .unwrap();
    assert_eq!(o.invocations.document_pins(&a.document), Ok(1));
    assert_eq!(o.invocations.document_pins(&b.document), Ok(0));
    o.stop_invocation(&id, &mut r).unwrap();
    assert_eq!(o.invocations.document_pins(&a.document), Ok(1));
    o.retire_invocation(&id, &mut r).unwrap();
    assert_eq!(o.invocations.document_pins(&a.document), Ok(0));
}
#[test]
fn invocation_document_pins_refuse_running_unknown_and_inventory_drift_for_every_target() {
    let (mut o, mut r) = owner();
    let a = o.create(b"a", &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    o.invocations.retained += 1;
    assert_eq!(
        o.invocations.document_pins(&a.document),
        Err(CoordinatorError::Faulted)
    );
    o.invocations.retained -= 1;
    o.invocations.slots[id.slot].as_mut().unwrap().phase = Phase::Running;
    assert_eq!(
        o.invocations.document_pins(&a.document),
        Err(CoordinatorError::Faulted)
    );
}
#[test]
fn invocation_document_pins_map_every_native_read_predicate_without_inventing_container_members() {
    use read_dependency::*;
    let (mut o, mut r) = owner();
    let a = o.create(b"a", &mut r).unwrap();
    let b = o.create(b"b", &mut r).unwrap();
    let c = o.create(b"c", &mut r).unwrap();
    let peer = o.create(b"peer", &mut r).unwrap();
    let annotation = |v: &DocumentVersion| AnnotationId {
        namespace: o.namespace.clone(),
        slot: 0,
        document: v.document.clone(),
    };
    let request = ReadDependencyRequest {
        predicates: vec![
            ReadDependencyV1::AnnotationSet {
                document: a.document.clone(),
                expected: 0,
            },
            ReadDependencyV1::AnnotationBody {
                annotation: annotation(&b),
                expected: 1,
            },
            ReadDependencyV1::AnnotationTarget {
                annotation: annotation(&c),
                expected: 1,
            },
        ],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 1024,
        },
    };
    let capture = ReadCapture::copy(std::slice::from_ref(&a), &request).unwrap();
    for d in [&a.document, &b.document, &c.document] {
        assert!(capture.refers_to_document(d));
    }
    assert!(!capture.refers_to_document(&peer.document));
}
