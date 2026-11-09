use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{NativeTimingProfileV1, Recorder};
use vellum_engine::resource::*;
#[path = "support/annotation_read_fixture.rs"]
#[allow(dead_code)]
mod support;
#[test]
fn queue_timing_is_explicit_and_survives_capture_pressure_without_changing_work() {
    for capacity in [0, 1, 128] {
        let mut f = support::Fixture::new();
        assert_eq!(
            f.owner.resource_queue_responsiveness_v1(&mut f.recorder),
            ResourceQueueResponsivenessV1::Off
        );
        f.owner
            .configure_resource_queue_timing_v1(
                NativeTimingProfileV1::ProcessMonotonicV1,
                &mut f.recorder,
            )
            .unwrap();
        f.recorder = Recorder::try_with_capacity(capacity).unwrap();
        let generation = f.owner.generation();
        let id = f.admit();
        let ResourceQueueResponsivenessV1::Observed(before) =
            f.owner.resource_queue_responsiveness_v1(&mut f.recorder)
        else {
            panic!("observed")
        };
        let q = &before.families[ResourceQueueKindV1::AnnotationRead.index()];
        assert_eq!(
            (q.observed_admissions, q.pending, q.unknown_pending_ages),
            (1, 1, 0)
        );
        assert!(q.oldest_known_pending_age.is_some());
        let dispatch = f.submit(&id);
        let first = f
            .owner
            .resource_queue_attempt_timing_v1(
                ResourceQueueAttemptV1::AnnotationRead(&id),
                &mut f.recorder,
            )
            .unwrap();
        assert_eq!(first.phase, ResourceQueuePhaseV1::Dispatched);
        assert!(first.queue_duration.is_some() && first.pending_age.is_none());
        f.owner.stop_annotation_read(&id, &mut f.recorder).unwrap();
        assert_eq!(
            f.owner
                .resource_queue_attempt_timing_v1(
                    ResourceQueueAttemptV1::AnnotationRead(&id),
                    &mut f.recorder
                )
                .unwrap(),
            first
        );
        let ResourceQueueResponsivenessV1::Observed(after) =
            f.owner.resource_queue_responsiveness_v1(&mut f.recorder)
        else {
            panic!("observed")
        };
        let q = &after.families[ResourceQueueKindV1::AnnotationRead.index()];
        assert_eq!(
            (
                q.observed_admissions,
                q.pending,
                q.dispatched,
                q.not_dispatched
            ),
            (1, 0, 1, 0)
        );
        assert!(q.max_known_queue_duration.is_some());
        f.owner
            .record_annotation_read(
                f.port.annotation_read_quiescence(&dispatch).unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        drop(dispatch);
        f.owner
            .retire_annotation_read(&id, &mut f.recorder)
            .unwrap();
        assert_eq!(
            f.owner.resource_queue_attempt_timing_v1(
                ResourceQueueAttemptV1::AnnotationRead(&id),
                &mut f.recorder
            ),
            Err(ResourceError::Unavailable)
        );
        let ResourceQueueResponsivenessV1::Observed(retired) =
            f.owner.resource_queue_responsiveness_v1(&mut f.recorder)
        else {
            panic!("observed")
        };
        let q = &retired.families[ResourceQueueKindV1::AnnotationRead.index()];
        assert_eq!(
            (q.observed_admissions, q.dispatched, q.retired_rows),
            (1, 1, 1)
        );
        assert_eq!(f.owner.generation(), generation);
    }
}
#[test]
fn source_read_and_source_save_stopped_before_dispatch_keep_original_queue_duration_after_retirement()
 {
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
    let read = f
        .owner
        .admit_source_read_v1(
            f.port.source_read_target_v1(b"new").unwrap(),
            ReadStrength::CapturedRead,
            &mut f.recorder,
        )
        .unwrap();
    let pair = f.pair().unwrap();
    let save = f
        .owner
        .queue_pinned_save_with_pair(
            &f.doc,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            pair,
            &mut f.recorder,
        )
        .unwrap();
    let ResourceQueueResponsivenessV1::Observed(pending) =
        f.owner.resource_queue_responsiveness_v1(&mut f.recorder)
    else {
        panic!("observed")
    };
    assert_eq!(
        pending.families[ResourceQueueKindV1::SourceRead.index()].pending,
        1
    );
    assert_eq!(
        pending.families[ResourceQueueKindV1::SourceSave.index()].pending,
        1
    );
    let generation = f.owner.generation();
    f.owner.stop_source_read_v1(&read, &mut f.recorder).unwrap();
    f.owner.stop_save(&save, &mut f.recorder).unwrap();
    for target in [
        ResourceQueueAttemptV1::SourceRead(&read),
        ResourceQueueAttemptV1::Save(&save),
    ] {
        let a = f
            .owner
            .resource_queue_attempt_timing_v1(target, &mut f.recorder)
            .unwrap();
        let b = f
            .owner
            .resource_queue_attempt_timing_v1(target, &mut f.recorder)
            .unwrap();
        assert_eq!(a, b);
        assert_eq!(a.phase, ResourceQueuePhaseV1::NotDispatched);
        assert!(a.queue_duration.is_some());
    }
    f.owner
        .retire_source_read_v1(&read, &mut f.recorder)
        .unwrap();
    f.owner.retire_save(&save, &mut f.recorder).unwrap();
    let ResourceQueueResponsivenessV1::Observed(done) =
        f.owner.resource_queue_responsiveness_v1(&mut f.recorder)
    else {
        panic!("observed")
    };
    for kind in [
        ResourceQueueKindV1::SourceRead,
        ResourceQueueKindV1::SourceSave,
    ] {
        let q = done.families[kind.index()];
        assert_eq!(
            (
                q.observed_admissions,
                q.pending,
                q.dispatched,
                q.not_dispatched,
                q.retired_rows
            ),
            (1, 0, 0, 1, 1)
        );
    }
    assert!(done.population_complete);
    f.owner
        .configure_resource_queue_timing_v1(
            NativeTimingProfileV1::ProcessMonotonicV1,
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner.resource_queue_responsiveness_v1(&mut f.recorder),
        ResourceQueueResponsivenessV1::Observed(done)
    );
    assert_eq!(f.owner.generation(), generation);
}
#[test]
fn unknown_membership_is_not_retroactively_invented_and_foreign_targets_do_not_read_timing() {
    let mut f = support::Fixture::new();
    let id = f.admit();
    assert_eq!(
        f.owner.configure_resource_queue_timing_v1(
            NativeTimingProfileV1::ProcessMonotonicV1,
            &mut f.recorder
        ),
        Err(ResourceError::Unavailable)
    );
    let off = f
        .owner
        .resource_queue_attempt_timing_v1(
            ResourceQueueAttemptV1::AnnotationRead(&id),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(off.phase, ResourceQueuePhaseV1::Off);
    let mut g = support::Fixture::new();
    g.owner
        .configure_resource_queue_timing_v1(
            NativeTimingProfileV1::ProcessMonotonicV1,
            &mut g.recorder,
        )
        .unwrap();
    assert_eq!(
        g.owner.resource_queue_attempt_timing_v1(
            ResourceQueueAttemptV1::AnnotationRead(&id),
            &mut g.recorder
        ),
        Err(ResourceError::Unavailable)
    );
    f.owner.stop_annotation_read(&id, &mut f.recorder).unwrap();
    f.owner
        .retire_annotation_read(&id, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner.configure_resource_queue_timing_v1(
            NativeTimingProfileV1::ProcessMonotonicV1,
            &mut f.recorder
        ),
        Err(ResourceError::Unavailable)
    );
}
#[test]
fn dispatched_source_queues_end_before_evidence_and_stop_cannot_rewrite_them() {
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
    let read = f
        .owner
        .admit_source_read_v1(
            f.port.source_read_target_v1(b"new").unwrap(),
            ReadStrength::CapturedRead,
            &mut f.recorder,
        )
        .unwrap();
    let pair = f.pair().unwrap();
    let save = f
        .owner
        .queue_pinned_save_with_pair(
            &f.doc,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            pair,
            &mut f.recorder,
        )
        .unwrap();
    let rd = f
        .owner
        .submit_source_read_v1(&read, &mut f.recorder)
        .unwrap();
    let pair = f.pair().unwrap();
    let wd = f
        .owner
        .dispatch_save_with_pair(
            &save,
            f.port.read_result(b"source", b"s0", b"text").unwrap(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    let a = f
        .owner
        .resource_queue_attempt_timing_v1(
            ResourceQueueAttemptV1::SourceRead(&read),
            &mut f.recorder,
        )
        .unwrap();
    let b = f
        .owner
        .resource_queue_attempt_timing_v1(ResourceQueueAttemptV1::Save(&save), &mut f.recorder)
        .unwrap();
    assert_eq!(
        (a.phase, b.phase),
        (
            ResourceQueuePhaseV1::Dispatched,
            ResourceQueuePhaseV1::Dispatched
        )
    );
    assert!(a.queue_duration.is_some() && b.queue_duration.is_some());
    assert_eq!(
        f.owner
            .save_outcome(&save, &mut f.recorder)
            .unwrap()
            .application,
        Application::Unknown
    );
    f.owner.stop_source_read_v1(&read, &mut f.recorder).unwrap();
    f.owner.stop_save(&save, &mut f.recorder).unwrap();
    assert_eq!(
        f.owner
            .resource_queue_attempt_timing_v1(
                ResourceQueueAttemptV1::SourceRead(&read),
                &mut f.recorder
            )
            .unwrap(),
        a
    );
    assert_eq!(
        f.owner
            .resource_queue_attempt_timing_v1(ResourceQueueAttemptV1::Save(&save), &mut f.recorder)
            .unwrap(),
        b
    );
    f.owner
        .record_source_read_v1(
            f.port.source_read_quiescence_v1(&rd).unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    for e in [
        f.port
            .result(
                &wd,
                Application::NotWritten,
                0,
                Synchronization::NotRequested,
                None,
            )
            .unwrap(),
        f.port.no_future_effects(&wd).unwrap(),
        f.port.quiescence(&wd).unwrap(),
    ] {
        f.owner.record_save_evidence(e, &mut f.recorder).unwrap();
    }
    drop((rd, wd));
    f.owner
        .retire_source_read_v1(&read, &mut f.recorder)
        .unwrap();
    f.owner.retire_save(&save, &mut f.recorder).unwrap();
    let ResourceQueueResponsivenessV1::Observed(s) =
        f.owner.resource_queue_responsiveness_v1(&mut f.recorder)
    else {
        panic!("observed")
    };
    for kind in [
        ResourceQueueKindV1::SourceRead,
        ResourceQueueKindV1::SourceSave,
    ] {
        assert_eq!(
            (
                s.families[kind.index()].dispatched,
                s.families[kind.index()].retired_rows
            ),
            (1, 1)
        );
    }
}
