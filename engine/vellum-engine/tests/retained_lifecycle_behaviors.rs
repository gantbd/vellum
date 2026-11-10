//! Trusted-owner lifecycle: no UI, filesystem operations or inferred erasure.
use vellum_engine::attachment::AttachmentTarget;
use vellum_engine::coordinator::*;
use vellum_engine::resource::*;
#[path = "support/annotation_read_fixture.rs"]
mod fixture;
use fixture::Fixture;
fn dispose(f: &mut Fixture, set: u64) -> LifecycleState {
    let plan = f
        .owner
        .prepare_retained_disposal(&f.doc, set, &mut f.recorder)
        .unwrap();
    f.owner
        .commit_retained_disposal(plan, &mut f.recorder)
        .unwrap()
}
fn state(f: &mut Fixture) -> LifecycleState {
    f.owner
        .lifecycle_state(&f.doc.document, &mut f.recorder)
        .unwrap()
}
fn requirements() -> SaveRequirements {
    SaveRequirements {
        protection: Protection::BestEffort,
        acknowledgment: Acknowledgment::Ordinary,
    }
}

#[test]
fn birth_and_immediate_disposal_timing_preserves_bounds_ownership_and_spent_identity() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation, Recorder,
    };
    for mode in 0..4 {
        let mut owner = SourceCoordinator::new(CoordinatorLimits {
            max_document_incarnations: 2,
            max_participants: 2,
            max_staged_bytes: 64,
            source: vellum_engine::source::Limits {
                max_source_bytes: 16,
                max_edits: 2,
                max_edit_payload_bytes: 32,
            },
        });
        let mut foreign = SourceCoordinator::new(CoordinatorLimits {
            max_document_incarnations: 1,
            max_participants: 1,
            max_staged_bytes: 64,
            source: vellum_engine::source::Limits {
                max_source_bytes: 16,
                max_edits: 2,
                max_edit_payload_bytes: 32,
            },
        });
        let foreign_id = foreign
            .create(b"OTHER", &mut Recorder::disabled())
            .unwrap()
            .document;
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 64 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        }
        if mode == 3 {
            r.record(Observation {
                boundary: Boundary::Control,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        let first = owner.create(b"PRIVATE_ONE", &mut r).unwrap();
        let generation = owner.generation();
        assert!(
            owner
                .create(b"PRIVATE_EXCEEDS_SOURCE_BOUND", &mut r)
                .is_err()
        );
        assert_eq!(
            owner.dispose(&foreign_id, &mut r),
            Err(CoordinatorError::Unavailable)
        );
        assert_eq!(owner.generation(), generation);
        assert_eq!(owner.lifecycle_usage().issued_lifetime, 1);
        owner.dispose(&first.document, &mut r).unwrap();
        let second = owner.create(b"PRIVATE_TWO", &mut r).unwrap();
        assert_ne!(first.document, second.document);
        assert_eq!(owner.create(b"third", &mut r), Err(CoordinatorError::Limit));
        assert_eq!(owner.lifecycle_usage().issued_lifetime, 2);
        assert_eq!(owner.lifecycle_usage().live, 1);
        assert_eq!(
            owner.read(&second.document, &mut r).unwrap().source.text(),
            "PRIVATE_TWO"
        );
        let mut seen = 0;
        let mut refused = 0;
        while let Some(event) = r.pop_front() {
            if event.boundary == Boundary::Lifecycle {
                seen += 1;
                refused += usize::from(event.disposition == Disposition::Refused);
                assert_eq!(event.service_time.is_some(), mode == 2);
            }
            assert!(!format!("{event:?}").contains("PRIVATE"));
        }
        assert_eq!(seen, if mode == 1 || mode == 2 { 6 } else { 0 });
        if mode == 1 || mode == 2 {
            assert_eq!(refused, 3);
        }
        if mode == 3 {
            assert!(r.dropped() > 0);
        }
    }
}

#[test]
fn retained_lifecycle_service_timing_preserves_success_refusal_replay_and_peer_state() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation, Recorder,
    };
    let boundaries = [
        Boundary::DisposalPreparation,
        Boundary::RetainedDisposal,
        Boundary::LifecycleQuery,
        Boundary::DisposedBindingRelease,
        Boundary::DisposedProvenanceRelease,
    ];
    for mode in 0..4 {
        let mut f = Fixture::new();
        f.recorder = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 128 }).unwrap()
        };
        if mode >= 2 {
            f.recorder = f
                .recorder
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        }
        if mode == 3 {
            f.recorder.record(Observation {
                boundary: Boundary::Control,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        assert_eq!(
            f.owner
                .prepare_retained_disposal(&f.doc, 99, &mut f.recorder)
                .err(),
            Some(CoordinatorError::Stale)
        );
        let old = f
            .owner
            .prepare_retained_disposal(&f.doc, 0, &mut f.recorder)
            .unwrap();
        let peer = f.owner.create(b"PRIVATE_PEER", &mut f.recorder).unwrap();
        let generation = f.owner.generation();
        assert_eq!(
            f.owner.commit_retained_disposal(old, &mut f.recorder),
            Err(CoordinatorError::Stale)
        );
        assert_eq!(f.owner.generation(), generation);
        let disposed = dispose(&mut f, 0);
        assert_eq!(state(&mut f), disposed);
        let doc = f.doc.document.clone();
        assert!(
            f.owner
                .release_disposed_annotation_binding(&doc, disposed, &mut f.recorder)
                .unwrap()
        );
        assert_eq!(
            f.owner
                .release_disposed_source_binding(&doc, disposed, &mut f.recorder),
            Err(ResourceError::Stale)
        );
        let current = state(&mut f);
        assert!(
            f.owner
                .release_disposed_source_binding(&doc, current, &mut f.recorder)
                .unwrap()
        );
        let current = state(&mut f);
        assert!(
            !f.owner
                .release_disposed_source_binding(&doc, current, &mut f.recorder)
                .unwrap()
        );
        assert!(
            f.owner
                .release_disposed_provenance(&doc, current, &mut f.recorder)
                .unwrap()
        );
        let released = state(&mut f);
        assert_eq!(released.phase, LifecyclePhase::ProvenanceReleased);
        let generation = f.owner.generation();
        assert!(
            !f.owner
                .release_disposed_provenance(&doc, released, &mut f.recorder)
                .unwrap()
        );
        assert_eq!(f.owner.generation(), generation);
        assert_eq!(
            f.owner
                .read(&peer.document, &mut f.recorder)
                .unwrap()
                .source
                .text(),
            "PRIVATE_PEER"
        );
        assert_eq!(f.owner.lifecycle_usage().live, 1);
        assert_eq!(f.owner.lifecycle_usage().provenance_released, 1);
        let mut seen = [0; 5];
        let mut refusals = 0;
        while let Some(event) = f.recorder.pop_front() {
            if let Some(index) = boundaries.iter().position(|b| *b == event.boundary) {
                seen[index] += 1;
                refusals += usize::from(event.disposition == Disposition::Refused);
                assert_eq!(event.service_time.is_some(), mode == 2);
            }
            assert!(!format!("{event:?}").contains("PRIVATE"));
        }
        if mode == 1 || mode == 2 {
            assert!(seen.iter().all(|n| *n > 0));
            assert_eq!(refusals, 3);
        } else {
            assert_eq!(seen, [0; 5]);
        }
        if mode == 3 {
            assert!(f.recorder.dropped() > 0);
        }
    }
}

#[test]
fn lifecycle_capture_off_available_or_full_has_identical_peer_framing_and_no_sensitive_labels() {
    use vellum_engine::instrumentation::{Boundary, Recorder};
    for capacity in [None, Some(64), Some(1)] {
        let mut f = Fixture::new();
        let peer = f.owner.create(b"PRIVATE_PEER", &mut f.recorder).unwrap();
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let lifecycle = dispose(&mut f, 0);
        let id = f.doc.document.clone();
        assert!(
            f.owner
                .release_disposed_annotation_binding(&id, lifecycle, &mut f.recorder)
                .unwrap()
        );
        let before = f.owner.resource_usage();
        let generation = f.owner.generation();
        assert_eq!(
            f.owner
                .release_disposed_source_binding(&id, lifecycle, &mut f.recorder),
            Err(ResourceError::Stale)
        );
        assert_eq!(f.owner.resource_usage(), before);
        assert_eq!(f.owner.generation(), generation);
        let current = state(&mut f);
        assert!(
            f.owner
                .release_disposed_source_binding(&id, current, &mut f.recorder)
                .unwrap()
        );
        let actual = f.owner.read(&peer.document, &mut f.recorder).unwrap();
        assert_eq!(actual.version, peer);
        assert_eq!(actual.source.text(), "PRIVATE_PEER");
        assert_eq!(f.owner.lifecycle_usage().live, 1);
        assert_eq!(f.owner.lifecycle_usage().disposed_retained, 1);
        let mut observations = Vec::new();
        while let Some(observation) = f.recorder.pop_front() {
            observations.push(observation);
        }
        if capacity == Some(64) {
            for expected in [
                Boundary::DisposalPreparation,
                Boundary::RetainedDisposal,
                Boundary::DisposedBindingRelease,
            ] {
                assert!(
                    observations
                        .iter()
                        .any(|observation| observation.boundary == expected)
                );
            }
        }
        assert!(
            observations
                .iter()
                .all(|observation| observation.service_time.is_none())
        );
        assert!(!format!("{observations:?}").contains("PRIVATE"));
        if capacity == Some(1) {
            assert!(f.recorder.dropped() > 0);
        }
    }
}

#[test]
fn quiescence_and_stop_cannot_release_a_key_with_unknown_application() {
    let mut f = Fixture::new();
    let pair = f.pair().unwrap();
    let save = f
        .owner
        .queue_pinned_save_with_pair(&f.doc, requirements(), pair, &mut f.recorder)
        .unwrap();
    let pair = f.pair().unwrap();
    let dispatch = f
        .owner
        .dispatch_save_with_pair(
            &save,
            f.port.read_result(b"source", b"s0", b"text").unwrap(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner
            .prepare_retained_disposal(&f.doc, 0, &mut f.recorder)
            .err(),
        Some(CoordinatorError::Resource(ResourceError::Retained))
    );
    let id = f.doc.document.clone();
    f.owner.stop_save(&save, &mut f.recorder).unwrap();
    for evidence in [
        f.port.no_future_effects(&dispatch).unwrap(),
        f.port.quiescence(&dispatch).unwrap(),
    ] {
        f.owner
            .record_save_evidence(evidence, &mut f.recorder)
            .unwrap();
    }
    let before = f.owner.resource_usage();
    let generation = f.owner.generation();
    assert_eq!(
        f.owner
            .release_source_save_capacity_v1(&save, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(
        f.owner
            .prepare_retained_disposal(&f.doc, 0, &mut f.recorder)
            .err(),
        Some(CoordinatorError::Resource(ResourceError::Retained))
    );
    assert_eq!(f.owner.resource_usage(), before);
    assert_eq!(f.owner.generation(), generation);
    f.owner
        .record_save_evidence(
            f.port
                .result(
                    &dispatch,
                    Application::NotWritten,
                    0,
                    Synchronization::NotRequested,
                    None,
                )
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner
            .release_source_save_capacity_v1(&save, &mut f.recorder)
            .unwrap(),
        BaselineCapacityState::NotWritten
    );
    let lifecycle = dispose(&mut f, 0);
    assert!(
        f.owner
            .release_disposed_source_binding(&id, lifecycle, &mut f.recorder)
            .unwrap()
    );
}

#[test]
fn annotation_binding_archive_keeps_baseline_and_artifact_until_separate_provenance_discharge() {
    let mut f = Fixture::new();
    let encoder = f
        .owner
        .register_annotation_encoder(&mut f.recorder)
        .unwrap();
    let snapshot = f
        .owner
        .capture_annotations(&f.doc, 0, &mut f.recorder)
        .unwrap();
    let artifact = f
        .owner
        .retain_annotation_artifact(
            encoder.seal(snapshot, b"[]".to_vec()).unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let pair = f.pair();
    let save = f
        .owner
        .queue_bound_annotation_export(
            &artifact,
            f.port.read_result(b"comments", b"a0", b"{}").unwrap(),
            requirements(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    let write = f
        .owner
        .dispatch_save(
            &save,
            f.port.read_result(b"comments", b"a0", b"{}").unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    for evidence in [
        f.port
            .result(
                &write,
                Application::Written,
                2,
                Synchronization::NotRequested,
                Some(b"a1"),
            )
            .unwrap(),
        f.port.no_future_effects(&write).unwrap(),
        f.port.quiescence(&write).unwrap(),
    ] {
        f.owner
            .record_save_evidence(evidence, &mut f.recorder)
            .unwrap();
    }
    let pair = f.pair();
    f.owner
        .publish_annotation_baseline(&save, pair, 65536, &mut f.recorder)
        .unwrap();
    let lifecycle = dispose(&mut f, 0);
    let id = f.doc.document.clone();
    assert!(
        f.owner
            .release_disposed_annotation_binding(&id, lifecycle, &mut f.recorder)
            .unwrap()
    );
    drop(write);
    assert_eq!(
        f.owner.retire_save(&save, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(f.owner.resource_usage().retained_annotation_bindings, 1);
    assert_eq!(
        f.owner
            .resource_artifact_bytes(&artifact, 0, 2, &mut f.recorder)
            .unwrap(),
        b"[]"
    );
}

#[test]
fn disposal_consumption_rechecks_a_new_baseline_obligation_without_relying_on_publication_change() {
    let mut f = Fixture::new();
    let plan = f
        .owner
        .prepare_retained_disposal(&f.doc, 0, &mut f.recorder)
        .unwrap();
    let encoder = f
        .owner
        .register_annotation_encoder(&mut f.recorder)
        .unwrap();
    let snapshot = f
        .owner
        .capture_annotations(&f.doc, 0, &mut f.recorder)
        .unwrap();
    let pair = f.pair();
    f.owner
        .queue_annotation_save(
            encoder.seal(snapshot, b"[]".to_vec()).unwrap(),
            f.port.read_result(b"comments", b"a0", b"{}").unwrap(),
            requirements(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    let generation = f.owner.generation();
    assert_eq!(
        f.owner.commit_retained_disposal(plan, &mut f.recorder),
        Err(CoordinatorError::Resource(ResourceError::Retained))
    );
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(state(&mut f).phase, LifecyclePhase::Live);
}

#[test]
fn retained_disposal_fences_live_content_but_preserves_annotations_history_and_existing_copies() {
    let mut f = Fixture::new();
    let pair = f.pair().unwrap();
    let queued = f
        .owner
        .queue_pinned_save_with_pair(&f.doc, requirements(), pair, &mut f.recorder)
        .unwrap();
    let launch_pair = f.pair().unwrap();
    let peer = f.owner.create(b"peer", &mut f.recorder).unwrap();
    let cross_object = f
        .owner
        .prepare(
            &[DocumentEdits {
                expected: peer.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 4,
                    expected: "peer".into(),
                    inserted: "PEER".into(),
                }],
            }],
            std::slice::from_ref(&f.doc),
            &mut f.recorder,
        )
        .unwrap();
    let change = f
        .owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                target: SourceTarget {
                    source: f.doc.clone(),
                    basis: TargetBasis::Before,
                    target: AttachmentTarget::Document,
                },
                body: "PRIVATE_BODY".into(),
            }],
            &mut f.recorder,
        )
        .unwrap();
    let authored = f.owner.commit(change, &mut f.recorder).unwrap();
    let annotation = authored.annotation_changes[0].annotation.clone();
    let snapshot = f
        .owner
        .capture_annotations(&f.doc, 1, &mut f.recorder)
        .unwrap();
    let edit = f
        .owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: annotation.clone(),
                expected_content: 1,
                body: "later".into(),
            }],
            &mut f.recorder,
        )
        .unwrap();
    let history = f.owner.history_usage();
    let records = f.owner.annotation_usage();
    assert_eq!(
        f.owner
            .prepare_retained_disposal(&f.doc, 1, &mut f.recorder)
            .err(),
        Some(CoordinatorError::Resource(ResourceError::Retained))
    );
    // An undispatched source save now carries the same explicit future-baseline
    // obligation as an annotation save. Stop proves NotWritten; disposal cannot
    // simply strand a still-pending capacity promise.
    f.owner.stop_save(&queued, &mut f.recorder).unwrap();
    assert_eq!(
        f.owner
            .release_source_save_capacity_v1(&queued, &mut f.recorder)
            .unwrap(),
        BaselineCapacityState::NotWritten
    );
    let lifecycle = dispose(&mut f, 1);
    assert_eq!(lifecycle.phase, LifecyclePhase::DisposedRetained);
    assert_eq!(
        f.owner.read(&f.doc.document, &mut f.recorder).err(),
        Some(CoordinatorError::Unavailable)
    );
    assert_eq!(
        f.owner.annotation(&annotation, &mut f.recorder).err(),
        Some(CoordinatorError::Unavailable)
    );
    assert!(f.owner.commit(edit, &mut f.recorder).is_err());
    assert!(f.owner.commit(cross_object, &mut f.recorder).is_err());
    assert_eq!(
        f.owner
            .read(&peer.document, &mut f.recorder)
            .unwrap()
            .source
            .text(),
        "peer"
    );
    assert_eq!(
        f.owner
            .dispatch_save_with_pair(
                &queued,
                f.port.read_result(b"source", b"s0", b"text").unwrap(),
                launch_pair,
                &mut f.recorder
            )
            .err(),
        Some(ResourceError::Unavailable)
    );
    assert!(
        f.owner
            .capture_annotations(&f.doc, 1, &mut f.recorder)
            .is_err()
    );
    assert!(
        f.owner
            .set_read_only(&f.doc.document, false, &mut f.recorder)
            .is_err()
    );
    assert_eq!(f.owner.history_usage(), history);
    assert_eq!(f.owner.annotation_usage(), records);
    assert_eq!(snapshot.records()[0].body(), "PRIVATE_BODY");
    f.owner
        .retire_history_group(&authored.history_group.unwrap(), &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.history_usage().retained_groups, 0);
}

#[test]
fn disposed_keys_remain_reserved_until_independent_release_in_either_order() {
    for annotation_first in [false, true] {
        let mut f = Fixture::new();
        let lifecycle = dispose(&mut f, 0);
        assert_eq!(
            f.owner
                .release_disposed_provenance(&f.doc.document, lifecycle, &mut f.recorder),
            Err(ResourceError::Retained)
        );
        assert_eq!(
            f.owner.load_resource(
                f.port.read_result(b"source", b"s0", b"text").unwrap(),
                &mut f.recorder
            ),
            Err(ResourceError::Unavailable)
        );
        assert_eq!(
            f.owner.load_resource(
                f.port.read_result(b"comments", b"a0", b"{}").unwrap(),
                &mut f.recorder
            ),
            Err(ResourceError::Conflict)
        );
        let id = f.doc.document.clone();
        let first = if annotation_first {
            f.owner
                .release_disposed_annotation_binding(&id, lifecycle, &mut f.recorder)
        } else {
            f.owner
                .release_disposed_source_binding(&id, lifecycle, &mut f.recorder)
        };
        assert_eq!(first, Ok(true));
        let next = state(&mut f);
        assert_eq!(
            f.owner
                .release_disposed_provenance(&f.doc.document, next, &mut f.recorder),
            Err(ResourceError::Retained)
        );
        assert_eq!(next.generation, lifecycle.generation + 1);
        let second = if annotation_first {
            f.owner
                .release_disposed_source_binding(&id, next, &mut f.recorder)
        } else {
            f.owner
                .release_disposed_annotation_binding(&id, next, &mut f.recorder)
        };
        assert_eq!(second, Ok(true));
        let current = state(&mut f);
        let generation = f.owner.generation();
        assert_eq!(
            f.owner
                .release_disposed_annotation_binding(&id, current, &mut f.recorder),
            Ok(false)
        );
        assert_eq!(f.owner.generation(), generation);
        let fresh = f
            .owner
            .load_resource(
                f.port.read_result(b"source", b"s1", b"different").unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        assert_ne!(fresh.document, id);
        assert_eq!(state(&mut f), current);
    }
}

#[test]
fn source_key_release_needs_known_disposition_not_quiescence_and_archived_baseline_keeps_its_producer()
 {
    let mut f = Fixture::new();
    let pair = f.pair().unwrap();
    let save = f
        .owner
        .queue_pinned_save_with_pair(&f.doc, requirements(), pair, &mut f.recorder)
        .unwrap();
    let pair = f.pair().unwrap();
    let write = f
        .owner
        .dispatch_save_with_pair(
            &save,
            f.port.read_result(b"source", b"s0", b"text").unwrap(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    // A qualified historical baseline may be retained before worker quiescence.
    f.owner
        .record_save_evidence(
            f.port
                .result(
                    &write,
                    Application::Written,
                    4,
                    Synchronization::NotRequested,
                    Some(b"s1"),
                )
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let pair = f.pair().unwrap();
    f.owner
        .publish_save_baseline_with_pair(&save, pair, &mut f.recorder)
        .unwrap();
    let lifecycle = dispose(&mut f, 0);
    let id = f.doc.document.clone();
    assert_eq!(
        f.owner
            .release_disposed_source_binding(&id, lifecycle, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    f.owner
        .record_save_evidence(f.port.no_future_effects(&write).unwrap(), &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .release_disposed_source_binding(&id, lifecycle, &mut f.recorder),
        Ok(true)
    );
    f.owner
        .record_save_evidence(f.port.quiescence(&write).unwrap(), &mut f.recorder)
        .unwrap();
    drop(write);
    assert_eq!(
        f.owner.retire_save(&save, &mut f.recorder),
        Err(ResourceError::Retained)
    );
}

#[test]
fn outstanding_annotation_baseline_promises_block_disposal_until_safe_capacity_discharge() {
    let mut f = Fixture::new();
    let encoder = f
        .owner
        .register_annotation_encoder(&mut f.recorder)
        .unwrap();
    let snapshot = f
        .owner
        .capture_annotations(&f.doc, 0, &mut f.recorder)
        .unwrap();
    let encoded = encoder.seal(snapshot, b"[]".to_vec()).unwrap();
    let pair = f.pair();
    let save = f
        .owner
        .queue_annotation_save(
            encoded,
            f.port.read_result(b"comments", b"a0", b"{}").unwrap(),
            requirements(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner
            .prepare_retained_disposal(&f.doc, 0, &mut f.recorder)
            .err(),
        Some(CoordinatorError::Resource(ResourceError::Retained))
    );
    f.owner.stop_save(&save, &mut f.recorder).unwrap();
    f.owner
        .release_annotation_save_capacity(&save, &mut f.recorder)
        .unwrap();
    dispose(&mut f, 0);
    assert_eq!(
        f.owner.save_outcome(&save, &mut f.recorder).unwrap().phase,
        SavePhase::Stopped
    );
}

#[test]
fn exact_root_disposal_preparation_is_spent_by_intervening_publication_and_read_evidence_survives_disposal()
 {
    let mut f = Fixture::new();
    let plan = f
        .owner
        .prepare_retained_disposal(&f.doc, 0, &mut f.recorder)
        .unwrap();
    f.owner.create(b"peer", &mut f.recorder).unwrap();
    assert_eq!(
        f.owner.commit_retained_disposal(plan, &mut f.recorder),
        Err(CoordinatorError::Stale)
    );
    let read = f.admit();
    let dispatch = f.submit(&read);
    for evidence in [
        f.port.annotation_read_start(&dispatch, b"a0").unwrap(),
        f.port
            .annotation_read_chunk(&dispatch, 0, 0, b"{}")
            .unwrap(),
        f.port.annotation_read_end(&dispatch, b"a0", 1, 2).unwrap(),
    ] {
        f.record(evidence).unwrap();
    }
    let captured = f.capture(&read).unwrap();
    dispose(&mut f, 0);
    f.record(f.port.annotation_read_quiescence(&dispatch).unwrap())
        .unwrap();
    assert_eq!(captured.bytes(), b"{}");
    assert!(
        f.owner
            .annotation_read_status(&read, &mut f.recorder)
            .unwrap()
            .quiescent
    );
    assert_eq!(
        f.owner
            .annotation_read_capture(&read, None, &mut f.recorder)
            .err(),
        Some(ResourceError::Unavailable)
    );
}
