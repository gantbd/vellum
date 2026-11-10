use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::resource::*;
#[path = "support/annotation_read_fixture.rs"]
mod fixture;
use fixture::Fixture;
#[test]
fn only_complete_ordered_transcripts_release_original_bytes_and_short_chunks_are_not_eof() {
    for capacity in [None, Some(128), Some(1)] {
        let mut f = Fixture::new();
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let id = f.admit();
        let dispatch = f.submit(&id);
        assert!(f.capture(&id).is_err());
        f.record(f.port.annotation_read_start(&dispatch, b"v0").unwrap())
            .unwrap();
        f.record(f.port.annotation_read_chunk(&dispatch, 0, 0, b"{").unwrap())
            .unwrap();
        assert!(f.capture(&id).is_err());
        assert_eq!(
            f.record(f.port.annotation_read_chunk(&dispatch, 0, 0, b"{").unwrap()),
            Ok(EvidenceDisposition::Duplicate)
        );
        assert_eq!(
            f.record(f.port.annotation_read_chunk(&dispatch, 2, 1, b"}").unwrap()),
            Err(ResourceError::Malformed)
        );
        f.record(f.port.annotation_read_chunk(&dispatch, 1, 1, b"}").unwrap())
            .unwrap();
        assert_eq!(
            f.record(f.port.annotation_read_end(&dispatch, b"v0", 2, 3).unwrap()),
            Err(ResourceError::Malformed)
        );
        f.record(f.port.annotation_read_end(&dispatch, b"v0", 2, 2).unwrap())
            .unwrap();
        let capture = f.capture(&id).unwrap();
        assert_eq!(capture.bytes(), b"{}");
        assert_eq!(capture.source(), &f.doc);
        assert_eq!(
            f.owner
                .annotation_read_status(&id, &mut f.recorder)
                .unwrap()
                .received_bytes,
            2
        );
        assert!(
            !f.owner
                .annotation_read_status(&id, &mut f.recorder)
                .unwrap()
                .quiescent
        );
        if capacity == Some(1) {
            assert!(f.recorder.dropped() > 0);
        }
    }
}
#[test]
fn quiescence_stop_and_actual_delivery_drain_are_independent_of_received_completion() {
    let mut f = Fixture::new();
    let id = f.admit();
    let dispatch = f.submit(&id);
    f.record(f.port.annotation_read_quiescence(&dispatch).unwrap())
        .unwrap();
    assert!(f.capture(&id).is_err());
    f.record(f.port.annotation_read_start(&dispatch, b"v0").unwrap())
        .unwrap();
    f.record(f.port.annotation_read_end(&dispatch, b"v0", 0, 0).unwrap())
        .unwrap();
    let capture = f.capture(&id).unwrap();
    assert!(capture.bytes().is_empty());
    f.owner.stop_annotation_read(&id, &mut f.recorder).unwrap();
    assert!(f.capture(&id).is_err());
    assert_eq!(capture.bytes(), b"");
    assert_eq!(
        f.owner.retire_annotation_read(&id, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    drop(dispatch);
    assert_eq!(
        f.owner.retire_annotation_read(&id, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    drop(capture);
    f.owner
        .retire_annotation_read(&id, &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.resource_usage().retained_reads, 0);
    assert_eq!(f.owner.resource_usage().issued_reads_lifetime, 1);
}
#[test]
fn contradictory_receipts_disable_release_and_keep_only_bounded_witnesses() {
    let mut f = Fixture::new();
    let id = f.admit();
    let dispatch = f.submit(&id);
    f.record(f.port.annotation_read_start(&dispatch, b"v0").unwrap())
        .unwrap();
    assert_eq!(
        f.record(f.port.annotation_read_start(&dispatch, b"v1").unwrap()),
        Ok(EvidenceDisposition::Contradiction)
    );
    assert_eq!(
        f.record(f.port.annotation_read_start(&dispatch, b"v1").unwrap()),
        Ok(EvidenceDisposition::Duplicate)
    );
    assert_eq!(
        f.record(f.port.annotation_read_start(&dispatch, b"v0").unwrap()),
        Ok(EvidenceDisposition::Duplicate)
    );
    assert_eq!(
        f.record(f.port.annotation_read_start(&dispatch, b"v2").unwrap()),
        Err(ResourceError::EvidenceCapacity)
    );
    assert!(matches!(f.capture(&id), Err(ResourceError::Contested)));
    assert!(
        f.owner
            .annotation_read_status(&id, &mut f.recorder)
            .unwrap()
            .contested
    );
}

#[test]
fn independent_read_registration_is_attributed_exactly_and_peer_changes_do_not_stale_it() {
    let mut f = Fixture::new();
    let reader = f
        .owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            Some(&f.port.identity_domain()),
            &mut f.recorder,
        )
        .unwrap();
    let pair = f.pair();
    let id = f
        .owner
        .admit_annotation_read(
            &f.doc,
            0,
            reader.annotation_read_target(b"comments").unwrap(),
            ReadStrength::CapturedRead,
            pair,
            &mut f.recorder,
        )
        .unwrap();
    let dispatch = f.submit(&id);
    assert!(matches!(
        f.port.annotation_read_start(&dispatch, b"v0"),
        Err(ResourceError::Unavailable)
    ));
    f.record(reader.annotation_read_start(&dispatch, b"r1").unwrap())
        .unwrap();
    f.record(reader.annotation_read_end(&dispatch, b"r1", 0, 0).unwrap())
        .unwrap();
    f.owner.create(b"unrelated peer", &mut f.recorder).unwrap();
    let capture = f.capture(&id).unwrap();
    assert_eq!(capture.version(), b"r1");
    let edit = f
        .owner
        .prepare(
            &[DocumentEdits {
                expected: f.doc.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 4,
                    expected: "text".into(),
                    inserted: "changed".into(),
                }],
            }],
            &[],
            &mut f.recorder,
        )
        .unwrap();
    f.owner.commit(edit, &mut f.recorder).unwrap();
    assert!(matches!(f.capture(&id), Err(ResourceError::Stale)));
    assert!(capture.bytes().is_empty());
    f.record(reader.annotation_read_quiescence(&dispatch).unwrap())
        .unwrap();
}

#[test]
fn unsupported_strength_missing_pair_wrong_resource_and_capacity_refuse_without_issuing_origins() {
    let mut f = Fixture::new();
    for case in 0..3 {
        let before = f.owner.resource_usage();
        let pair = if case == 1 { None } else { f.pair() };
        let target = f
            .port
            .annotation_read_target(if case == 2 { b"source" } else { b"comments" })
            .unwrap();
        let strength = if case == 0 {
            ReadStrength::AtomicResourceSnapshot
        } else {
            ReadStrength::CapturedRead
        };
        assert!(
            f.owner
                .admit_annotation_read(&f.doc, 0, target, strength, pair, &mut f.recorder)
                .is_err()
        );
        assert_eq!(f.owner.resource_usage(), before);
    }
    let mut ids = Vec::new();
    for _ in 0..4 {
        ids.push(f.admit());
    }
    let before = f.owner.resource_usage();
    let pair = f.pair();
    assert_eq!(
        f.owner.admit_annotation_read(
            &f.doc,
            0,
            f.port.annotation_read_target(b"comments").unwrap(),
            ReadStrength::CapturedRead,
            pair,
            &mut f.recorder
        ),
        Err(ResourceError::Limit)
    );
    assert_eq!(f.owner.resource_usage(), before);
    f.owner
        .stop_annotation_read(&ids[0], &mut f.recorder)
        .unwrap();
    f.owner
        .retire_annotation_read(&ids[0], &mut f.recorder)
        .unwrap();
    let fresh = f.admit();
    assert_ne!(fresh, ids[0]);
    assert_eq!(f.owner.resource_usage().issued_reads_lifetime, 5);
}

#[test]
fn overflow_refuses_capture_without_publishing_a_prefix_or_discarding_the_original_evidence() {
    let mut f = Fixture::new();
    let id = f.admit();
    let dispatch = f.submit(&id);
    let generation = f.owner.generation();
    f.record(f.port.annotation_read_start(&dispatch, b"v0").unwrap())
        .unwrap();
    f.record(
        f.port
            .annotation_read_chunk(&dispatch, 0, 0, &[b'x'; 40])
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        f.record(
            f.port
                .annotation_read_chunk(&dispatch, 1, 40, &[b'y'; 40])
                .unwrap()
        ),
        Err(ResourceError::Limit)
    );
    let status = f
        .owner
        .annotation_read_status(&id, &mut f.recorder)
        .unwrap();
    assert!(status.refused);
    assert_eq!(status.received_bytes, 40);
    assert!(!status.complete);
    assert!(f.capture(&id).is_err());
    assert_eq!(f.owner.generation(), generation);
    f.record(
        f.port
            .annotation_read_failure(&dispatch, ReadFailure::Limit)
            .unwrap(),
    )
    .unwrap();
    f.record(f.port.annotation_read_quiescence(&dispatch).unwrap())
        .unwrap();
    drop(dispatch);
    f.owner
        .retire_annotation_read(&id, &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.resource_usage().held_read_bytes, 0);
}

#[test]
fn stopped_submission_discards_new_content_but_preserves_terminal_and_quiescence_facts() {
    let mut f = Fixture::new();
    let id = f.admit();
    let dispatch = f.submit(&id);
    f.owner.stop_annotation_read(&id, &mut f.recorder).unwrap();
    f.record(f.port.annotation_read_start(&dispatch, b"v0").unwrap())
        .unwrap();
    f.record(
        f.port
            .annotation_read_chunk(&dispatch, 0, 0, b"{}")
            .unwrap(),
    )
    .unwrap();
    f.record(f.port.annotation_read_end(&dispatch, b"v0", 1, 2).unwrap())
        .unwrap();
    let status = f
        .owner
        .annotation_read_status(&id, &mut f.recorder)
        .unwrap();
    assert!(status.stopped);
    assert!(!status.complete);
    assert_eq!(status.received_bytes, 0);
    f.record(f.port.annotation_read_quiescence(&dispatch).unwrap())
        .unwrap();
    drop(dispatch);
    f.owner
        .retire_annotation_read(&id, &mut f.recorder)
        .unwrap();
}

#[test]
fn a_successful_end_cannot_replace_a_failed_read_and_foreign_receipts_cannot_settle_it() {
    let mut f = Fixture::new();
    let id = f.admit();
    let dispatch = f.submit(&id);
    let mut other = Fixture::new();
    let other_id = other.admit();
    let other_dispatch = other.submit(&other_id);
    assert_eq!(
        f.record(
            other
                .port
                .annotation_read_quiescence(&other_dispatch)
                .unwrap()
        ),
        Err(ResourceError::Unavailable)
    );
    f.record(f.port.annotation_read_start(&dispatch, b"v0").unwrap())
        .unwrap();
    f.record(
        f.port
            .annotation_read_failure(&dispatch, ReadFailure::Interrupted)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        f.record(f.port.annotation_read_end(&dispatch, b"v0", 0, 0).unwrap()),
        Ok(EvidenceDisposition::Contradiction)
    );
    assert!(matches!(f.capture(&id), Err(ResourceError::Contested)));
    assert!(
        !f.owner
            .annotation_read_status(&id, &mut f.recorder)
            .unwrap()
            .complete
    );
}

#[test]
fn read_and_write_launches_share_execution_headroom_without_confusing_quiescence_with_outcome() {
    for read_first in [true, false] {
        let mut f = Fixture::with_outstanding(1);
        let read = f.admit();
        let pair = f.pair().unwrap();
        let write = f
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
        if read_first {
            let dispatch = f.submit(&read);
            let pair = f.pair().unwrap();
            assert!(matches!(
                f.owner.dispatch_save_with_pair(
                    &write,
                    f.port.read_result(b"source", b"s0", b"text").unwrap(),
                    pair,
                    &mut f.recorder
                ),
                Err(ResourceError::Limit)
            ));
            f.record(f.port.annotation_read_quiescence(&dispatch).unwrap())
                .unwrap();
            let pair = f.pair().unwrap();
            f.owner
                .dispatch_save_with_pair(
                    &write,
                    f.port.read_result(b"source", b"s0", b"text").unwrap(),
                    pair,
                    &mut f.recorder,
                )
                .unwrap();
        } else {
            let pair = f.pair().unwrap();
            let dispatch = f
                .owner
                .dispatch_save_with_pair(
                    &write,
                    f.port.read_result(b"source", b"s0", b"text").unwrap(),
                    pair,
                    &mut f.recorder,
                )
                .unwrap();
            let pair = f.pair();
            assert!(matches!(
                f.owner.submit_annotation_read(&read, pair, &mut f.recorder),
                Err(ResourceError::Limit)
            ));
            f.owner
                .record_save_evidence(f.port.quiescence(&dispatch).unwrap(), &mut f.recorder)
                .unwrap();
            f.submit(&read);
            assert_eq!(
                f.owner
                    .save_outcome(&write, &mut f.recorder)
                    .unwrap()
                    .application,
                Application::Unknown
            );
        }
    }
}
