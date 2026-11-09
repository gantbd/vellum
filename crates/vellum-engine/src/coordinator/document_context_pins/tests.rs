use super::*;
use crate::instrumentation::NativeTimingProfileV1;

fn owner() -> (SourceCoordinator, Recorder, DocumentVersion) {
    let mut o = SourceCoordinator::configured(CoordinatorConfig {
        limits: CoordinatorLimits {
            max_document_incarnations: 4,
            max_participants: 4,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 4096,
                max_edits: 8,
                max_edit_payload_bytes: 4096,
            },
        },
        invocation: None,
        resources: None,
        search: None,
        history: None,
        annotations: Some(AnnotationLimits {
            max_issued: 8,
            max_records: 8,
            max_body_bytes: 1024,
            max_retained_body_bytes: 8192,
            max_mapping_steps: 8192,
            max_live_snapshots: 4,
            max_snapshot_bytes: 16384,
        }),
    });
    let mut r = Recorder::disabled();
    o.configure_logical_views(
        LogicalViewLimits {
            max_live_views: 4,
            max_issued_views: 8,
            max_total_selections: 16,
            normalization: crate::selection::SelectionBudget {
                max_input_selections: 8,
                max_output_selections: 8,
                max_work: 1024,
            },
            max_mapping_steps: 1024,
        },
        &mut r,
    )
    .unwrap();
    o.configure_workspaces(
        WorkspaceLimits {
            max_issued: 4,
            max_live: 4,
            max_memberships: 8,
            max_work: 1024,
        },
        &mut r,
    )
    .unwrap();
    o.configure_boundaries(
        BoundaryLimits {
            max_issued: 8,
            max_retained: 4,
            max_source_bytes: 1024,
            max_boundaries: 64,
            max_words: 8,
            max_evidence_bytes: 4096,
            max_retained_bytes: 8192,
        },
        &mut r,
    )
    .unwrap();
    let d = o.create(b"abc", &mut r).unwrap();
    (o, r, d)
}

#[test]
fn context_pins_follow_actual_attachments_memberships_evidence_and_snapshot_lifetimes() {
    let (mut o, mut r, d) = owner();
    let peer = o.create(b"peer", &mut r).unwrap();
    let view = o.create_logical_view(&mut r).unwrap();
    let stamp = o.logical_view(&view, &mut r).unwrap().stamp;
    // A detached view and empty workspace do not pin every document.
    let ws = o.create_workspace(&mut r).unwrap();
    assert_eq!(
        o.document_context_pins_v1(&d.document, &mut r)
            .unwrap()
            .total(),
        Ok(0)
    );
    let update = o
        .attach_logical_view(
            &view,
            stamp,
            &d,
            &[crate::selection::Selection {
                anchor: crate::coordinates::BytePosition(0),
                head: crate::coordinates::BytePosition(0),
                affinity: crate::attachment::Affinity::After,
            }],
            0,
            &mut r,
        )
        .unwrap();
    let membership = o
        .set_workspace_membership(&ws, &d.document, true, &mut r)
        .unwrap();
    let pending = o.begin_boundary_capture(&d, &mut r).unwrap();
    let ready = o.begin_boundary_capture(&d, &mut r).unwrap();
    let evidence = o
        .accept_boundary_evidence(
            &ready,
            &BoundaryTableV1 {
                profile: BoundaryProfileV1::Unicode18Uax29Revision49,
                graphemes: (0..=3).map(crate::coordinates::BytePosition).collect(),
                words: vec![
                    crate::coordinates::BytePosition(0),
                    crate::coordinates::BytePosition(3),
                ],
                significant_words: vec![ByteRange { start: 0, end: 3 }],
            },
            &mut r,
        )
        .unwrap();
    let set = o.annotation_set_version(&d.document, &mut r).unwrap();
    let snapshot = o.capture_annotations(&d, set, &mut r).unwrap();
    let clone = snapshot.clone();
    let pins = o.document_context_pins_v1(&d.document, &mut r).unwrap();
    assert_eq!(
        pins,
        DocumentContextPinsV1 {
            attached_views: 1,
            workspace_memberships: 1,
            boundary_captures: 1,
            boundary_evidence: 1,
            annotation_snapshots: 1,
            serialization_bases: 0,
            retained_invocations: 0,
            publication_streams: 0
        }
    );
    assert_eq!(
        o.document_context_pins_v1(&peer.document, &mut r)
            .unwrap()
            .total(),
        Ok(0)
    );
    let publication = o.generation();
    let held = o.retention_usage();
    for (role, count) in [
        (ShutdownOwnerRoleV1::LogicalViews, 1),
        (ShutdownOwnerRoleV1::Workspaces, 1),
        (ShutdownOwnerRoleV1::BoundaryEvidence, 2),
    ] {
        let row =
            crate::coordinator::shutdown::tests::inventory_owner(&o, &mut r, role, &[&d.document]);
        assert_eq!(row.retained_records, Some(count));
        assert_eq!(row.reserved_bytes, None);
        if role == ShutdownOwnerRoleV1::BoundaryEvidence {
            assert_eq!(row.held_bytes, Some(o.boundary_usage().retained_bytes));
        }
    }
    for timing in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        for capacity in [0, 1, 16] {
            let mut capture = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(timing);
            assert_eq!(
                o.document_context_pins_v1(&d.document, &mut capture),
                Ok(pins)
            );
            assert_eq!(capture.queued(), usize::from(capacity > 0));
            if let Some(event) = capture.pop_front() {
                assert_eq!(event.boundary, Boundary::DocumentContextPins);
                assert_eq!(
                    event.service_time.is_some(),
                    timing == NativeTimingProfileV1::ProcessMonotonicV1
                );
            }
        }
    }
    assert_eq!(o.generation(), publication);
    assert_eq!(o.retention_usage(), held);
    o.detach_logical_view(&view, update.stamp, &mut r).unwrap();
    o.set_workspace_membership(&membership.context, &d.document, false, &mut r)
        .unwrap();
    let edit = o
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "a".into(),
                    inserted: "z".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(edit, &mut r).unwrap();
    assert!(o.boundary_evidence(&evidence, &mut r).is_err());
    // Losing currentness is not retirement of retained evidence or snapshots.
    let stale = o.document_context_pins_v1(&d.document, &mut r).unwrap();
    assert_eq!(
        (
            stale.boundary_captures,
            stale.boundary_evidence,
            stale.annotation_snapshots
        ),
        (1, 1, 1)
    );
    o.retire_boundary_capture(&pending, &mut r).unwrap();
    o.retire_boundary_evidence(&evidence, &mut r).unwrap();
    let boundary_owner = crate::coordinator::shutdown::tests::inventory_owner(
        &o,
        &mut r,
        ShutdownOwnerRoleV1::BoundaryEvidence,
        &[],
    );
    assert_eq!(boundary_owner.retained_records, Some(0));
    assert_eq!(boundary_owner.held_bytes, Some(0));
    assert_eq!(boundary_owner.state, ShutdownOwnerStateV1::Active);
    drop(snapshot);
    assert_eq!(
        o.document_context_pins_v1(&d.document, &mut r)
            .unwrap()
            .annotation_snapshots,
        1
    );
    drop(clone);
    assert_eq!(
        o.document_context_pins_v1(&d.document, &mut r)
            .unwrap()
            .total(),
        Ok(0)
    );
}

#[test]
fn context_pins_refuse_foreign_faulted_and_overflow_without_fabricating_zero() {
    let (mut o, mut r, d) = owner();
    let (_, _, foreign) = owner();
    assert_eq!(
        o.document_context_pins_v1(&foreign.document, &mut r),
        Err(CoordinatorError::Unavailable)
    );
    o.faulted = true;
    assert_eq!(
        o.document_context_pins_v1(&d.document, &mut r),
        Err(CoordinatorError::Faulted)
    );
    assert_eq!(
        DocumentContextPinsV1 {
            attached_views: usize::MAX,
            annotation_snapshots: 1,
            ..Default::default()
        }
        .total(),
        Err(CoordinatorError::Faulted)
    );
}
