use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, Disposition, NativeTimingProfileV1, Recorder};
use vellum_engine::resource::*;
#[path = "support/annotation_read_fixture.rs"]
#[allow(dead_code)]
mod support;

#[test]
fn read_inspection_and_protected_stop_time_success_and_refusal_without_changing_ownership() {
    for timed in [false, true] {
        for capacity in [0, 1, 256] {
            let mut f = support::Fixture::new();
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
            let source = f
                .owner
                .admit_source_read_v1(
                    f.port.source_read_target_v1(b"PRIVATE-source").unwrap(),
                    ReadStrength::CapturedRead,
                    &mut f.recorder,
                )
                .unwrap();
            let annotation = f.admit();
            let generation = f.owner.generation();
            let funding = f.owner.retention_usage();
            f.recorder = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(if timed {
                    NativeTimingProfileV1::ProcessMonotonicV1
                } else {
                    NativeTimingProfileV1::Off
                });
            assert_eq!(
                f.owner
                    .source_read_status_v1(&source, &mut f.recorder)
                    .unwrap()
                    .phase,
                ReadPhase::Ready
            );
            assert_eq!(
                f.owner
                    .annotation_read_status(&annotation, &mut f.recorder)
                    .unwrap()
                    .phase,
                ReadPhase::Ready
            );
            assert!(f.capture(&annotation).is_err());
            f.owner
                .stop_source_read_v1(&source, &mut f.recorder)
                .unwrap();
            f.owner
                .stop_annotation_read(&annotation, &mut f.recorder)
                .unwrap();
            assert!(
                f.owner
                    .source_read_status_v1(&source, &mut f.recorder)
                    .unwrap()
                    .stopped
            );
            assert!(
                f.owner
                    .annotation_read_status(&annotation, &mut f.recorder)
                    .unwrap()
                    .stopped
            );
            assert_eq!(f.owner.generation(), generation);
            assert_eq!(f.owner.retention_usage(), funding);
            f.owner
                .retire_source_read_v1(&source, &mut f.recorder)
                .unwrap();
            f.owner
                .retire_annotation_read(&annotation, &mut f.recorder)
                .unwrap();
            assert_eq!(
                f.owner.source_read_status_v1(&source, &mut f.recorder),
                Err(ResourceError::Unavailable)
            );
            assert_eq!(
                f.owner.annotation_read_status(&annotation, &mut f.recorder),
                Err(ResourceError::Unavailable)
            );
            assert_eq!(
                f.owner.stop_source_read_v1(&source, &mut f.recorder),
                Err(ResourceError::Unavailable)
            );
            assert_eq!(
                f.owner.stop_annotation_read(&annotation, &mut f.recorder),
                Err(ResourceError::Unavailable)
            );
            assert!(f.capture(&annotation).is_err());
            assert_eq!(f.owner.source_reader_usage_v1().unwrap().live, 0);
            let events: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
            assert!(!format!("{events:?}").contains("PRIVATE"));
            if capacity == 256 {
                for boundary in [
                    Boundary::SourceReadQuery,
                    Boundary::SourceReadControl,
                    Boundary::Outcome,
                    Boundary::AnnotationReadControl,
                    Boundary::AnnotationReadCapture,
                ] {
                    let matching: Vec<_> =
                        events.iter().filter(|e| e.boundary == boundary).collect();
                    assert!(!matching.is_empty(), "{boundary:?}");
                    assert!(
                        matching.iter().all(|e| e.service_time.is_some() == timed),
                        "{boundary:?}"
                    );
                    assert!(
                        matching
                            .iter()
                            .any(|e| e.disposition == Disposition::Refused),
                        "{boundary:?}"
                    );
                }
            } else if capacity == 1 {
                assert!(f.recorder.dropped() > 0);
            }
        }
    }
}

#[test]
fn completed_annotation_capture_has_timed_success_without_manufacturing_quiescence() {
    let mut f = support::Fixture::new();
    let id = f.admit();
    let dispatch = f.submit(&id);
    for evidence in [
        f.port.annotation_read_start(&dispatch, b"v0").unwrap(),
        f.port.annotation_read_end(&dispatch, b"v0", 0, 0).unwrap(),
    ] {
        f.record(evidence).unwrap();
    }
    f.recorder = Recorder::try_with_capacity(64)
        .unwrap()
        .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
    let captured = f.capture(&id).unwrap();
    assert!(captured.bytes().is_empty());
    assert!(
        !f.owner
            .annotation_read_status(&id, &mut f.recorder)
            .unwrap()
            .quiescent
    );
    let events: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
    assert!(
        events
            .iter()
            .any(|e| e.boundary == Boundary::AnnotationReadCapture
                && e.disposition == Disposition::Completed
                && e.service_time.is_some())
    );
    assert_eq!(
        f.owner.retire_annotation_read(&id, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    f.owner.stop_annotation_read(&id, &mut f.recorder).unwrap();
    f.record(f.port.annotation_read_quiescence(&dispatch).unwrap())
        .unwrap();
    drop(captured);
    drop(dispatch);
    f.owner
        .retire_annotation_read(&id, &mut f.recorder)
        .unwrap();
}
