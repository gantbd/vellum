//! Actual trusted-owner hot path timing, not UI latency or a wall-clock SLO.
use vellum_engine::coordinator::{CoordinatorLimits, DocumentEdits, SourceCoordinator};
use vellum_engine::instrumentation::{
    Boundary, Disposition, NativeTimingProfileV1, Observation, Recorder,
};
use vellum_engine::source::{Limits, Replacement};

fn owner() -> (
    SourceCoordinator,
    vellum_engine::coordinator::DocumentVersion,
) {
    let mut owner = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 2,
        max_participants: 2,
        max_staged_bytes: 8192,
        source: Limits {
            max_source_bytes: 4096,
            max_edits: 16,
            max_edit_payload_bytes: 1024,
        },
    });
    let version = owner.create(b"secret", &mut Recorder::disabled()).unwrap();
    (owner, version)
}
fn writes(v: vellum_engine::coordinator::DocumentVersion) -> [DocumentEdits; 1] {
    [DocumentEdits {
        expected: v,
        edits: vec![Replacement {
            start: 0,
            end: 1,
            expected: "s".into(),
            inserted: "S".into(),
        }],
    }]
}

#[test]
fn given_off_available_or_saturated_capture_then_edit_commit_query_meaning_is_identical() {
    let mut signatures = vec![];
    for capacity in [0, 1, 64] {
        let (mut owner, version) = owner();
        let mut recorder = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        if capacity == 1 {
            recorder.record(Observation {
                boundary: Boundary::Query,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        let prepared = owner
            .prepare(&writes(version.clone()), &[], &mut recorder)
            .unwrap();
        let outcome = owner.commit(prepared, &mut recorder).unwrap();
        let read = owner.read(&version.document, &mut recorder).unwrap();
        signatures.push((
            outcome.generation,
            read.version.revision,
            read.source.text().to_owned(),
        ));
        if capacity == 64 {
            let records: Vec<_> = std::iter::from_fn(|| recorder.pop_front()).collect();
            for boundary in [Boundary::Preparation, Boundary::Admission, Boundary::Query] {
                assert!(
                    records
                        .iter()
                        .any(|o| o.boundary == boundary && o.service_time.is_some())
                );
            }
            assert!(records.iter().all(|o| !format!("{o:?}").contains("secret")));
        } else if capacity == 0 {
            assert_eq!(recorder.queued(), 0);
        } else {
            assert!(recorder.dropped() >= 3);
        }
    }
    assert!(signatures.windows(2).all(|pair| pair[0] == pair[1]));
}

#[test]
fn given_refused_preparation_and_stale_commit_then_service_timing_does_not_hide_refusals() {
    let (mut owner, version) = owner();
    let mut recorder = Recorder::try_with_capacity(64)
        .unwrap()
        .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
    let mut bad = writes(version.clone());
    bad[0].edits[0].expected = "x".into();
    assert!(owner.prepare(&bad, &[], &mut recorder).is_err());
    let first = owner
        .prepare(&writes(version.clone()), &[], &mut recorder)
        .unwrap();
    let stale = owner.prepare(&writes(version), &[], &mut recorder).unwrap();
    owner.commit(first, &mut recorder).unwrap();
    assert!(owner.commit(stale, &mut recorder).is_err());
    let records: Vec<_> = std::iter::from_fn(|| recorder.pop_front()).collect();
    for boundary in [Boundary::Preparation, Boundary::Admission] {
        assert!(records.iter().any(|o| o.boundary == boundary
            && o.disposition == Disposition::Refused
            && o.service_time.is_some()));
    }
}
