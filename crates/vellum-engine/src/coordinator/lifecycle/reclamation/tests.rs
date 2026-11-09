use super::*;
use crate::attachment::AttachmentTarget;
use crate::instrumentation::{Disposition, NativeTimingProfileV1};
fn facts() -> TrustedDocumentReclamationV1 {
    TrustedDocumentReclamationV1 {
        authorized: Some(true),
        authored_disposition_ready: Some(true),
        session_references_released: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
#[test]
fn document_reclamation_waits_for_real_read_contexts_and_the_last_native_hold() {
    use crate::coordinator::source_query::result_contract::tests::{cleanup, owner as read_owner};
    let mut f = read_owner();
    let held =
        f.o.hold_source_read_context(
            &f.source,
            &f.s,
            &SourceReadHoldFactsV1 {
                owner_control_allowed: Some(true),
                retained: Some(true),
                resources_ready: Some(true),
            },
            &mut f.r,
        )
        .unwrap();
    let state = release(&mut f.o, &f.version, &mut f.r);
    let retained = Err(CoordinatorError::Resource(
        crate::resource::ResourceError::Retained,
    ));
    assert_eq!(
        f.o.reclaim_disposed_document_v1(&f.s.document, state, &facts(), &mut f.r),
        retained
    );
    assert!(f.o.stop_source_read_context(&f.source, &cleanup(&f.s), &mut f.r));
    assert!(!f.o.retire_source_read_context(&f.source, &cleanup(&f.s), &mut f.r));
    assert!(f.o.stop_annotation_context(&f.combined, &cleanup(&f.s), &mut f.r));
    assert!(f.o.retire_annotation_context(&f.combined, &cleanup(&f.s), &mut f.r));
    let controls = [cleanup(&f.s)];
    assert!(f.o.stop_version_set_context(
        VersionSetControlTargetV1::Context(&f.set.context),
        &controls,
        &mut f.r
    ));
    assert!(f.o.retire_version_set_context(
        VersionSetControlTargetV1::Context(&f.set.context),
        &controls,
        &mut f.r
    ));
    // Stop is neither hold release nor retirement, even after disposal.
    assert_eq!(
        f.o.reclaim_disposed_document_v1(&f.s.document, state, &facts(), &mut f.r),
        retained
    );
    assert!(f.o.release_source_read_hold(&held, &cleanup(&f.s), &mut f.r));
    assert!(f.o.retire_source_read_context(&f.source, &cleanup(&f.s), &mut f.r));
    assert_eq!(
        f.o.document_read_pins_v1(&f.s.document, &mut f.r)
            .unwrap()
            .total(),
        Ok(0)
    );
    f.o.reclaim_disposed_document_v1(&f.s.document, state, &facts(), &mut f.r)
        .unwrap();
}
#[test]
fn document_reclamation_tombstone_cannot_describe_a_reintroduced_live_owner() {
    let (mut o, mut r, d, _) = owner();
    let state = release(&mut o, &d, &mut r);
    o.reclaim_disposed_document_v1(&d.document, state, &facts(), &mut r)
        .unwrap();
    let peer = o.create(b"peer", &mut r).unwrap();
    o.documents[d.document.slot] = o.documents[peer.document.slot].take();
    assert_eq!(
        o.semantic_root_usage(&mut r),
        Err(RootAccountingErrorV1::Faulted)
    );
}
fn owner() -> (SourceCoordinator, Recorder, DocumentVersion, AnnotationId) {
    let mut o = SourceCoordinator::configured(CoordinatorConfig {
        limits: CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: crate::source::Limits {
                max_source_bytes: 4096,
                max_edits: 8,
                max_edit_payload_bytes: 8192,
            },
        },
        invocation: Some(InvocationLimits {
            max_lifetime_issued: 8,
            max_retained: 8,
            max_intent_bytes: 16384,
        }),
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
    let d = o.create(b"private prose", &mut r).unwrap();
    let p = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                body: "private annotation".into(),
                target: SourceTarget {
                    source: d.clone(),
                    basis: TargetBasis::Before,
                    target: AttachmentTarget::ContentRange { start: 0, end: 1 },
                },
            }],
            &mut r,
        )
        .unwrap();
    let a = p.created_annotations().next().unwrap().clone();
    o.commit(p, &mut r).unwrap();
    (o, r, d, a)
}
fn release(o: &mut SourceCoordinator, d: &DocumentVersion, r: &mut Recorder) -> LifecycleState {
    let set = o.annotation_set_version(&d.document, r).unwrap();
    let p = o.prepare_retained_disposal(d, set, r).unwrap();
    let state = o.commit_retained_disposal(p, r).unwrap();
    o.release_disposed_provenance(&d.document, state, r)
        .unwrap();
    o.lifecycle_state(&d.document, r).unwrap()
}
#[test]
fn document_reclamation_removes_payload_once_preserves_floors_identity_and_unrelated_document() {
    for timing in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        for capacity in [0, 1, 16] {
            let (mut o, mut r, d, a) = owner();
            let peer = o.create(b"untouched", &mut r).unwrap();
            let read = o.annotation(&a, &mut r).unwrap();
            let fact = AnnotationFact {
                annotation: a.clone(),
                present: true,
                content_version: read.content_version,
                target_generation: read.target_generation,
                placement_generation: read.placement_generation,
            };
            let state = release(&mut o, &d, &mut r);
            let before = o.semantic_root_usage(&mut r).unwrap();
            let publication = o.generation();
            let lifetime = o.lifecycle_usage().issued_lifetime;
            let held = o.retention_usage();
            let mut capture = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(timing);
            o.reclaim_disposed_document_v1(&d.document, state, &facts(), &mut capture)
                .unwrap();
            assert!(o.documents[d.document.slot].is_none());
            assert_eq!(o.lifecycle_usage().issued_lifetime, lifetime);
            assert_eq!(o.generation(), publication + 1);
            assert_eq!(o.retention_usage(), held);
            assert_eq!(
                o.read(&peer.document, &mut r).unwrap().source.text(),
                "untouched"
            );
            assert_eq!(o.annotations.reclaimed_fact_for_test(&a), Some(&fact));
            let after = o.semantic_root_usage(&mut r).unwrap();
            assert!(after.total_bytes < before.total_bytes);
            assert_eq!(
                o.annotation(&a, &mut r).err(),
                Some(CoordinatorError::Unavailable)
            );
            assert_eq!(
                o.reclaim_disposed_document_v1(&d.document, state, &facts(), &mut r),
                Err(CoordinatorError::Unavailable)
            );
            assert_eq!(capture.queued(), usize::from(capacity > 0));
            if let Some(event) = capture.pop_front() {
                assert_eq!(event.boundary, Boundary::DocumentReclamation);
                assert_eq!(event.disposition, Disposition::Completed);
                assert_eq!(
                    event.service_time.is_some(),
                    timing == NativeTimingProfileV1::ProcessMonotonicV1
                );
            }
            let next = o.create(b"next", &mut r).unwrap();
            assert!(next.document.slot > d.document.slot);
            assert_eq!(
                o.read(&d.document, &mut r).err(),
                Some(CoordinatorError::Unavailable)
            );
        }
    }
}
#[test]
fn document_reclamation_native_snapshot_and_invocation_pins_override_positive_host_claims() {
    let (mut o, mut r, d, _) = owner();
    let set = o.annotation_set_version(&d.document, &mut r).unwrap();
    let snapshot = o.capture_annotations(&d, set, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit(
        &id,
        &SourceIntent {
            writes: vec![],
            reads: vec![d.clone()],
        },
        &mut r,
    )
    .unwrap();
    let state = release(&mut o, &d, &mut r);
    let generation = o.generation();
    let retained = Err(CoordinatorError::Resource(
        crate::resource::ResourceError::Retained,
    ));
    assert_eq!(
        o.reclaim_disposed_document_v1(&d.document, state, &facts(), &mut r),
        retained
    );
    drop(snapshot);
    assert_eq!(
        o.reclaim_disposed_document_v1(&d.document, state, &facts(), &mut r),
        retained
    );
    assert_eq!(o.generation(), generation);
    assert!(!o.is_faulted());
    o.stop_invocation(&id, &mut r).unwrap();
    assert_eq!(
        o.reclaim_disposed_document_v1(&d.document, state, &facts(), &mut r),
        retained
    );
    o.retire_invocation(&id, &mut r).unwrap();
    o.reclaim_disposed_document_v1(&d.document, state, &facts(), &mut r)
        .unwrap();
}
#[test]
fn document_reclamation_refuses_live_stale_foreign_unknown_authority_and_exhausted_publication() {
    let (mut o, mut r, d, _) = owner();
    let live = o.lifecycle_state(&d.document, &mut r).unwrap();
    assert_eq!(
        o.reclaim_disposed_document_v1(&d.document, live, &facts(), &mut r),
        Err(CoordinatorError::Unavailable)
    );
    let (_, _, foreign, _) = owner();
    assert_eq!(
        o.reclaim_disposed_document_v1(&foreign.document, live, &facts(), &mut r),
        Err(CoordinatorError::Unavailable)
    );
    let state = release(&mut o, &d, &mut r);
    let publication = o.generation();
    for field in 0..5 {
        for value in [None, Some(false)] {
            let mut f = facts();
            match field {
                0 => f.authorized = value,
                1 => f.authored_disposition_ready = value,
                2 => f.session_references_released = value,
                3 => f.retention_released = value,
                _ => f.delivery_drained = value,
            }
            assert_eq!(
                o.reclaim_disposed_document_v1(&d.document, state, &f, &mut r),
                Err(CoordinatorError::Unavailable)
            );
        }
    }
    assert_eq!(
        o.reclaim_disposed_document_v1(&d.document, live, &facts(), &mut r),
        Err(CoordinatorError::Stale)
    );
    assert_eq!(o.generation(), publication);
    assert!(o.documents[d.document.slot].is_some());
    o.generation = u64::MAX;
    assert_eq!(
        o.reclaim_disposed_document_v1(&d.document, state, &facts(), &mut r),
        Err(CoordinatorError::VersionExhausted)
    );
    assert!(o.documents[d.document.slot].is_some());
    assert!(!o.is_faulted());
}
#[test]
fn document_reclamation_every_interrupted_publication_cut_fences_unknown_and_never_rearms_identity()
{
    for cut in 0..4 {
        let (mut o, mut r, d, _) = owner();
        let state = release(&mut o, &d, &mut r);
        o.publication_fault_after = Some(cut);
        let mut capture = Recorder::try_with_capacity(16)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || o.reclaim_disposed_document_v1(&d.document, state, &facts(), &mut capture)
            ))
            .is_err()
        );
        assert!(o.is_faulted());
        assert_eq!(
            o.create(b"forbidden", &mut r),
            Err(CoordinatorError::Faulted)
        );
        assert_eq!(
            o.reclaim_disposed_document_v1(&d.document, state, &facts(), &mut r),
            Err(CoordinatorError::Faulted)
        );
        let event = capture.pop_front().unwrap();
        assert_eq!(event.boundary, Boundary::DocumentReclamation);
        assert_eq!(event.disposition, Disposition::Unknown);
        assert!(event.service_time.is_some());
        assert!(capture.pop_front().is_none());
    }
}
