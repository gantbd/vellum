use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, Disposition, NativeTimingProfileV1, Recorder};
use vellum_engine::resource::*;
#[path = "support/annotation_read_fixture.rs"]
#[allow(dead_code)]
mod support;

#[test]
fn first_result_observation_is_opt_in_and_original_owner_scoped() {
    let mut f = support::Fixture::new();
    let id = f.admit();
    let t = f
        .owner
        .resource_result_attempt_timing_v1(
            ResourceQueueAttemptV1::AnnotationRead(&id),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(t.phase, ResourceQueuePhaseV1::Off);
    assert!(!t.first_result_observed);
    assert_eq!(t.time_to_first_result, None);
    let mut foreign = support::Fixture::new();
    foreign.recorder = Recorder::try_with_capacity(16)
        .unwrap()
        .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
    assert_eq!(
        foreign.owner.resource_result_attempt_timing_v1(
            ResourceQueueAttemptV1::AnnotationRead(&id),
            &mut foreign.recorder
        ),
        Err(ResourceError::Unavailable)
    );
    let refused = foreign.recorder.pop_front().unwrap();
    assert_eq!(refused.boundary, Boundary::ResourceResultAttemptTiming);
    assert_eq!(refused.disposition, Disposition::Refused);
    assert!(refused.service_time.is_some());
}

fn timing(
    f: &mut support::Fixture,
    target: ResourceQueueAttemptV1<'_>,
) -> ResourceResultAttemptTimingV1 {
    f.owner
        .resource_result_attempt_timing_v1(target, &mut f.recorder)
        .unwrap()
}

#[test]
fn annotation_start_and_quiescence_are_not_results_and_duplicates_do_not_renew() {
    for (capacity, timed) in [(0, false), (128, false), (128, true), (1, true)] {
        let mut f = support::Fixture::new();
        f.owner
            .configure_resource_queue_timing_v1(
                NativeTimingProfileV1::ProcessMonotonicV1,
                &mut f.recorder,
            )
            .unwrap();
        f.recorder = Recorder::try_with_capacity(capacity).unwrap();
        if timed {
            f.recorder = f
                .recorder
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        }
        let generation = f.owner.generation();
        let id = f.admit();
        assert_eq!(
            timing(&mut f, ResourceQueueAttemptV1::AnnotationRead(&id)).phase,
            ResourceQueuePhaseV1::Pending
        );
        let dispatch = f.submit(&id);
        let t = timing(&mut f, ResourceQueueAttemptV1::AnnotationRead(&id));
        assert!(!t.first_result_observed && t.awaiting_result_age.is_some());
        for e in [
            f.port.annotation_read_start(&dispatch, b"v").unwrap(),
            f.port.annotation_read_quiescence(&dispatch).unwrap(),
        ] {
            f.record(e).unwrap();
            let t = timing(&mut f, ResourceQueueAttemptV1::AnnotationRead(&id));
            assert!(!t.first_result_observed && t.awaiting_result_age.is_some());
        }
        f.record(
            f.port
                .annotation_read_chunk(&dispatch, 0, 0, b"{}")
                .unwrap(),
        )
        .unwrap();
        let first = timing(&mut f, ResourceQueueAttemptV1::AnnotationRead(&id));
        assert!(first.first_result_observed && first.time_to_first_result.is_some());
        assert_eq!(first.awaiting_result_age, None);
        assert_eq!(
            f.record(
                f.port
                    .annotation_read_chunk(&dispatch, 0, 0, b"{}")
                    .unwrap()
            ),
            Ok(EvidenceDisposition::Duplicate)
        );
        assert_eq!(
            timing(&mut f, ResourceQueueAttemptV1::AnnotationRead(&id)),
            first
        );
        f.owner.stop_annotation_read(&id, &mut f.recorder).unwrap();
        drop(dispatch);
        f.owner
            .retire_annotation_read(&id, &mut f.recorder)
            .unwrap();
        let missing = f.admit();
        let dispatch = f.submit(&missing);
        f.record(f.port.annotation_read_quiescence(&dispatch).unwrap())
            .unwrap();
        assert!(
            !timing(&mut f, ResourceQueueAttemptV1::AnnotationRead(&missing)).first_result_observed
        );
        f.owner
            .stop_annotation_read(&missing, &mut f.recorder)
            .unwrap();
        drop(dispatch);
        f.owner
            .retire_annotation_read(&missing, &mut f.recorder)
            .unwrap();
        let ResourceQueueResponsivenessV1::Observed(snapshot) =
            f.owner.resource_queue_responsiveness_v1(&mut f.recorder)
        else {
            panic!()
        };
        let p = snapshot.families[ResourceQueueKindV1::AnnotationRead.index()];
        assert_eq!(
            (
                p.awaiting_first_result,
                p.first_results,
                p.retired_without_first_result
            ),
            (0, 1, 1)
        );
        assert_eq!(
            p.max_known_first_result_duration,
            first.time_to_first_result
        );
        assert!(snapshot.population_complete);
        assert_eq!(f.owner.generation(), generation);
        if capacity == 0 {
            assert_eq!((f.recorder.queued(), f.recorder.dropped()), (0, 0));
        }
        if capacity == 1 {
            assert!(f.recorder.dropped() > 0);
        }
        if capacity == 128 {
            let mut observed = 0;
            while let Some(e) = f.recorder.pop_front() {
                if e.boundary == Boundary::ResourceResultAttemptTiming {
                    assert_eq!(e.disposition, Disposition::Completed);
                    assert_eq!(e.service_time.is_some(), timed);
                    observed += 1;
                }
            }
            assert_eq!(observed, 7);
        }
    }
}

#[test]
fn empty_source_terminal_is_a_result_but_start_is_not_and_first_result_survives_retirement() {
    let mut f = support::Fixture::new();
    f.owner
        .configure_resource_queue_timing_v1(
            NativeTimingProfileV1::ProcessMonotonicV1,
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .configure_retention_budget(1048576, &mut f.recorder)
        .unwrap();
    f.owner
        .configure_source_reader_v1(
            SourceReaderLimitsV1 {
                max_live: 4,
                max_issued: 8,
                max_input_bytes: 64,
                max_chunks: 8,
                max_held_bytes: 65536,
                max_store_bytes: 131072,
            },
            &mut f.recorder,
        )
        .unwrap();
    let id = f
        .owner
        .admit_source_read_v1(
            f.port.source_read_target_v1(b"new").unwrap(),
            ReadStrength::CapturedRead,
            &mut f.recorder,
        )
        .unwrap();
    let dispatch = f.owner.submit_source_read_v1(&id, &mut f.recorder).unwrap();
    f.owner
        .record_source_read_v1(
            f.port
                .source_read_start_v1(&dispatch, SourceReadVersionV1::Opaque(b"v"))
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    assert!(!timing(&mut f, ResourceQueueAttemptV1::SourceRead(&id)).first_result_observed);
    let generation = f.owner.generation();
    f.owner
        .record_source_read_v1(
            f.port
                .source_read_end_v1(&dispatch, SourceReadVersionV1::Opaque(b"v"), 0, 0)
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let first = timing(&mut f, ResourceQueueAttemptV1::SourceRead(&id));
    assert!(first.first_result_observed && first.time_to_first_result.is_some());
    f.owner
        .record_source_read_v1(
            f.port.source_read_quiescence_v1(&dispatch).unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        timing(&mut f, ResourceQueueAttemptV1::SourceRead(&id)),
        first
    );
    f.owner.stop_source_read_v1(&id, &mut f.recorder).unwrap();
    drop(dispatch);
    f.owner.retire_source_read_v1(&id, &mut f.recorder).unwrap();
    let ResourceQueueResponsivenessV1::Observed(snapshot) =
        f.owner.resource_queue_responsiveness_v1(&mut f.recorder)
    else {
        panic!()
    };
    let p = snapshot.families[ResourceQueueKindV1::SourceRead.index()];
    assert_eq!(
        (
            p.first_results,
            p.awaiting_first_result,
            p.retired_without_first_result
        ),
        (1, 0, 0)
    );
    assert_eq!(
        p.max_known_first_result_duration,
        first.time_to_first_result
    );
    assert_eq!(f.owner.generation(), generation);
}

#[test]
fn save_progress_no_future_and_quiescence_do_not_manufacture_first_result_for_either_artifact() {
    for annotations in [false, true] {
        let mut f = support::Fixture::new();
        f.owner
            .configure_resource_queue_timing_v1(
                NativeTimingProfileV1::ProcessMonotonicV1,
                &mut f.recorder,
            )
            .unwrap();
        let requirements = SaveRequirements {
            protection: Protection::BestEffort,
            acknowledgment: Acknowledgment::Ordinary,
        };
        let pair = f.pair();
        let id = if annotations {
            let encoder = f
                .owner
                .register_annotation_encoder(&mut f.recorder)
                .unwrap();
            let snapshot = f
                .owner
                .capture_annotations(&f.doc, 0, &mut f.recorder)
                .unwrap();
            f.owner
                .queue_annotation_save(
                    encoder.seal(snapshot, b"[]".to_vec()).unwrap(),
                    f.port.read_result(b"comments", b"a0", b"{}").unwrap(),
                    requirements,
                    pair,
                    &mut f.recorder,
                )
                .unwrap()
        } else {
            f.owner
                .queue_pinned_save_with_pair(&f.doc, requirements, pair.unwrap(), &mut f.recorder)
                .unwrap()
        };
        let pair = f.pair().unwrap();
        let dispatch = f
            .owner
            .dispatch_save_with_pair(
                &id,
                if annotations {
                    f.port.read_result(b"comments", b"a0", b"{}").unwrap()
                } else {
                    f.port.read_result(b"source", b"s0", b"text").unwrap()
                },
                pair,
                &mut f.recorder,
            )
            .unwrap();
        let generation = f.owner.generation();
        for e in [
            f.port
                .result(
                    &dispatch,
                    Application::Unknown,
                    1,
                    Synchronization::Unknown,
                    None,
                )
                .unwrap(),
            f.port.no_future_effects(&dispatch).unwrap(),
            f.port.quiescence(&dispatch).unwrap(),
        ] {
            f.owner.record_save_evidence(e, &mut f.recorder).unwrap();
            let t = timing(&mut f, ResourceQueueAttemptV1::Save(&id));
            assert!(!t.first_result_observed && t.awaiting_result_age.is_some());
        }
        for _ in 0..2 {
            f.owner
                .record_save_evidence(
                    f.port
                        .result(
                            &dispatch,
                            Application::Written,
                            if annotations { 2 } else { 4 },
                            Synchronization::NotRequested,
                            Some(b"v1"),
                        )
                        .unwrap(),
                    &mut f.recorder,
                )
                .unwrap();
        }
        let first = timing(&mut f, ResourceQueueAttemptV1::Save(&id));
        assert!(first.first_result_observed && first.time_to_first_result.is_some());
        assert_eq!(
            first.kind,
            if annotations {
                ResourceQueueKindV1::AnnotationSave
            } else {
                ResourceQueueKindV1::SourceSave
            }
        );
        f.owner
            .record_save_evidence(
                f.port
                    .result(
                        &dispatch,
                        Application::Written,
                        if annotations { 2 } else { 4 },
                        Synchronization::NotRequested,
                        Some(b"v1"),
                    )
                    .unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        assert_eq!(timing(&mut f, ResourceQueueAttemptV1::Save(&id)), first);
        assert_eq!(f.owner.generation(), generation);
        let pair = f.pair();
        if annotations {
            f.owner
                .install_annotation_save_baseline(&id, pair, &mut f.recorder)
                .unwrap();
        } else {
            f.owner
                .publish_save_baseline_with_pair(&id, pair.unwrap(), &mut f.recorder)
                .unwrap();
        }
        f.owner.stop_save(&id, &mut f.recorder).unwrap();
        drop(dispatch);
        // The new measurements cannot override an actual baseline producer hold.
        assert_eq!(
            f.owner.retire_save(&id, &mut f.recorder),
            Err(ResourceError::Retained)
        );
        let plan = f
            .owner
            .prepare_retained_disposal(&f.doc, 0, &mut f.recorder)
            .unwrap();
        f.owner
            .commit_retained_disposal(plan, &mut f.recorder)
            .unwrap();
        let state = f
            .owner
            .lifecycle_state(&f.doc.document, &mut f.recorder)
            .unwrap();
        f.owner
            .release_disposed_source_binding(&f.doc.document, state, &mut f.recorder)
            .unwrap();
        let state = f
            .owner
            .lifecycle_state(&f.doc.document, &mut f.recorder)
            .unwrap();
        f.owner
            .release_disposed_annotation_binding(&f.doc.document, state, &mut f.recorder)
            .unwrap();
        let state = f
            .owner
            .lifecycle_state(&f.doc.document, &mut f.recorder)
            .unwrap();
        f.owner
            .release_disposed_provenance(&f.doc.document, state, &mut f.recorder)
            .unwrap();
        f.owner.retire_save(&id, &mut f.recorder).unwrap();
        let ResourceQueueResponsivenessV1::Observed(snapshot) =
            f.owner.resource_queue_responsiveness_v1(&mut f.recorder)
        else {
            panic!()
        };
        assert_eq!(snapshot.families[first.kind.index()].first_results, 1);
    }
}
