use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::resource::*;
use vellum_engine::source::{Replacement, SourceError};
#[path = "support/discovered_context_fixture.rs"]
mod discovery_support;
#[path = "support/annotation_read_fixture.rs"]
#[allow(dead_code)]
mod support;
use support::Fixture;
#[path = "source_read_behaviors/activation_authority.rs"]
mod activation_authority;
#[path = "source_read_behaviors/invocations.rs"]
mod invocations;
#[path = "source_read_behaviors/load_facade.rs"]
mod load_facade;
#[path = "source_read_behaviors/observation.rs"]
mod observation;
#[path = "source_read_behaviors/operations.rs"]
mod operations;
#[test]
fn activated_source_read_remains_a_document_pin_after_provenance_release_until_actual_retirement() {
    let mut f = setup();
    let (task, dispatch) = submitted(&mut f, b"new-document");
    complete(&mut f, &dispatch, b"loaded");
    f.owner
        .record_source_read_v1(
            f.port.source_read_quiescence_v1(&dispatch).unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    drop(dispatch);
    let activated = f
        .owner
        .activate_source_read_v1(&task, &mut f.recorder)
        .unwrap()
        .document;
    let pins = f
        .owner
        .document_resource_pins_v1(&activated.document, &mut f.recorder)
        .unwrap();
    assert_eq!((pins.source_bindings, pins.activated_source_reads), (1, 1));
    let p = f
        .owner
        .prepare_retained_disposal(&activated, 0, &mut f.recorder)
        .unwrap();
    let state = f
        .owner
        .commit_retained_disposal(p, &mut f.recorder)
        .unwrap();
    f.owner
        .release_disposed_source_binding(&activated.document, state, &mut f.recorder)
        .unwrap();
    let state = f
        .owner
        .lifecycle_state(&activated.document, &mut f.recorder)
        .unwrap();
    f.owner
        .release_disposed_provenance(&activated.document, state, &mut f.recorder)
        .unwrap();
    let pins = f
        .owner
        .document_resource_pins_v1(&activated.document, &mut f.recorder)
        .unwrap();
    assert_eq!((pins.source_bindings, pins.activated_source_reads), (0, 1));
    f.owner.stop_source_read_v1(&task, &mut f.recorder).unwrap();
    assert_eq!(
        f.owner
            .document_resource_pins_v1(&activated.document, &mut f.recorder)
            .unwrap()
            .activated_source_reads,
        1
    );
    f.owner
        .retire_source_read_v1(&task, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .document_resource_pins_v1(&activated.document, &mut f.recorder)
            .unwrap()
            .total(),
        Ok(0)
    );
    assert_eq!(
        f.owner
            .lifecycle_state(&activated.document, &mut f.recorder)
            .unwrap()
            .phase,
        LifecyclePhase::ProvenanceReleased
    );
}
fn limits() -> SourceReaderLimitsV1 {
    SourceReaderLimitsV1 {
        max_live: 4,
        max_issued: 8,
        max_input_bytes: 1024,
        max_chunks: 64,
        max_held_bytes: 65536,
        max_store_bytes: 131072,
    }
}
fn setup() -> Fixture {
    let mut f = Fixture::new();
    f.owner
        .configure_retention_budget(1048576, &mut f.recorder)
        .unwrap();
    f.owner
        .configure_source_reader_v1(limits(), &mut f.recorder)
        .unwrap();
    f
}
fn admit(f: &mut Fixture, key: &[u8]) -> SourceReadIdV1 {
    f.owner
        .admit_source_read_v1(
            f.port.source_read_target_v1(key).unwrap(),
            ReadStrength::CapturedRead,
            &mut f.recorder,
        )
        .unwrap()
}
fn submitted(f: &mut Fixture, key: &[u8]) -> (SourceReadIdV1, SourceReadDispatchV1) {
    let id = admit(f, key);
    let dispatch = f.owner.submit_source_read_v1(&id, &mut f.recorder).unwrap();
    (id, dispatch)
}
fn complete(f: &mut Fixture, dispatch: &SourceReadDispatchV1, bytes: &[u8]) {
    let version = SourceReadVersionV1::Opaque(b"v");
    for e in [
        f.port.source_read_start_v1(dispatch, version).unwrap(),
        f.port.source_read_chunk_v1(dispatch, 0, 0, bytes).unwrap(),
        f.port
            .source_read_end_v1(dispatch, version, 1, bytes.len())
            .unwrap(),
    ] {
        f.owner.record_source_read_v1(e, &mut f.recorder).unwrap();
    }
}
#[test]
fn ready_submission_eof_and_activation_are_distinct_and_source_bytes_are_lossless() {
    let raw = "\u{feff}English\r\nDeutsch: Grüße\rFin\n".as_bytes();
    for split in 0..=raw.len() {
        let mut f = setup();
        let generation = f.owner.generation();
        let (id, dispatch) = submitted(&mut f, b"new");
        assert_eq!(
            f.owner.activate_source_read_v1(&id, &mut f.recorder),
            Err(ResourceError::MissingEvidence)
        );
        let v = SourceReadVersionV1::EpochToken(ResourceVersionV1 {
            epoch: b"epoch",
            token: b"token",
        });
        for e in [
            f.port.source_read_start_v1(&dispatch, v).unwrap(),
            f.port
                .source_read_chunk_v1(&dispatch, 0, 0, &raw[..split])
                .unwrap(),
            f.port
                .source_read_chunk_v1(&dispatch, 1, split, &raw[split..])
                .unwrap(),
            f.port
                .source_read_end_v1(&dispatch, v, 2, raw.len())
                .unwrap(),
        ] {
            f.owner.record_source_read_v1(e, &mut f.recorder).unwrap();
        }
        assert_eq!(f.owner.generation(), generation);
        let status = f.owner.source_read_status_v1(&id, &mut f.recorder).unwrap();
        assert!(status.candidate_ready && !status.quiescent && status.activated.is_none());
        let activated = f
            .owner
            .activate_source_read_v1(&id, &mut f.recorder)
            .unwrap();
        assert_eq!(
            activated.disposition,
            SourceReadActivationDispositionV1::Created
        );
        let read = f
            .owner
            .read(&activated.document.document, &mut f.recorder)
            .unwrap();
        assert_eq!(read.source.encode().unwrap(), raw);
        let generation = f.owner.generation();
        assert_eq!(
            f.owner
                .activate_source_read_v1(&id, &mut f.recorder)
                .unwrap()
                .disposition,
            SourceReadActivationDispositionV1::AlreadyActivated
        );
        assert_eq!(f.owner.generation(), generation);
    }
}
#[test]
fn source_read_order_exact_end_and_original_issuer_are_mandatory() {
    let mut f = setup();
    let (id, dispatch) = submitted(&mut f, b"new");
    assert_eq!(
        f.owner.record_source_read_v1(
            f.port.source_read_chunk_v1(&dispatch, 0, 0, b"x").unwrap(),
            &mut f.recorder
        ),
        Err(ResourceError::Malformed)
    );
    let other = Fixture::new();
    assert!(matches!(
        other
            .port
            .source_read_start_v1(&dispatch, SourceReadVersionV1::Opaque(b"v")),
        Err(ResourceError::Unavailable)
    ));
    let start = f
        .port
        .source_read_start_v1(&dispatch, SourceReadVersionV1::Opaque(b"v"))
        .unwrap();
    assert_eq!(
        f.owner.record_source_read_v1(start, &mut f.recorder),
        Ok(EvidenceDisposition::Accepted)
    );
    assert_eq!(
        f.owner.record_source_read_v1(
            f.port.source_read_chunk_v1(&dispatch, 1, 0, b"x").unwrap(),
            &mut f.recorder
        ),
        Err(ResourceError::Malformed)
    );
    assert_eq!(
        f.owner.record_source_read_v1(
            f.port
                .source_read_end_v1(&dispatch, SourceReadVersionV1::Opaque(b"other"), 0, 0)
                .unwrap(),
            &mut f.recorder
        ),
        Err(ResourceError::Malformed)
    );
    assert!(
        !f.owner
            .source_read_status_v1(&id, &mut f.recorder)
            .unwrap()
            .candidate_ready
    );
}
#[test]
fn invalid_prefix_and_incomplete_eof_refuse_without_publication_or_a_candidate() {
    for bytes in [&b"\xff"[..], &b"\xf0\x9f"[..]] {
        let mut f = setup();
        let generation = f.owner.generation();
        let (id, dispatch) = submitted(&mut f, b"new");
        f.owner
            .record_source_read_v1(
                f.port
                    .source_read_start_v1(&dispatch, SourceReadVersionV1::Opaque(b"v"))
                    .unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        let chunk = f.owner.record_source_read_v1(
            f.port.source_read_chunk_v1(&dispatch, 0, 0, bytes).unwrap(),
            &mut f.recorder,
        );
        if bytes == b"\xff" {
            assert_eq!(chunk, Err(ResourceError::Source(SourceError::InvalidUtf8)));
        } else {
            chunk.unwrap();
            assert_eq!(
                f.owner.record_source_read_v1(
                    f.port
                        .source_read_end_v1(
                            &dispatch,
                            SourceReadVersionV1::Opaque(b"v"),
                            1,
                            bytes.len()
                        )
                        .unwrap(),
                    &mut f.recorder
                ),
                Err(ResourceError::Source(SourceError::InvalidUtf8))
            );
        }
        let status = f.owner.source_read_status_v1(&id, &mut f.recorder).unwrap();
        assert!(status.decode_refused && !status.candidate_ready);
        assert_eq!(
            f.owner.activate_source_read_v1(&id, &mut f.recorder),
            Err(ResourceError::Unavailable)
        );
        assert_eq!(f.owner.generation(), generation);
    }
}
#[test]
fn duplicate_and_conflicting_original_chunks_do_not_publish_or_authorize_activation() {
    let mut f = setup();
    let (id, dispatch) = submitted(&mut f, b"new");
    complete(&mut f, &dispatch, b"first");
    assert_eq!(
        f.owner.record_source_read_v1(
            f.port
                .source_read_chunk_v1(&dispatch, 0, 0, b"first")
                .unwrap(),
            &mut f.recorder
        ),
        Ok(EvidenceDisposition::Duplicate)
    );
    assert_eq!(
        f.owner.record_source_read_v1(
            f.port
                .source_read_chunk_v1(&dispatch, 0, 0, b"other")
                .unwrap(),
            &mut f.recorder
        ),
        Ok(EvidenceDisposition::Contradiction)
    );
    let status = f.owner.source_read_status_v1(&id, &mut f.recorder).unwrap();
    assert!(status.contested && !status.candidate_ready);
    assert_eq!(
        f.owner.activate_source_read_v1(&id, &mut f.recorder),
        Err(ResourceError::Contested)
    );
    assert_eq!(
        f.owner.record_source_read_v1(
            f.port
                .source_read_chunk_v1(&dispatch, 0, 0, b"third")
                .unwrap(),
            &mut f.recorder
        ),
        Err(ResourceError::EvidenceCapacity)
    );
}
#[test]
fn joining_a_dirty_existing_document_preserves_source_and_does_not_publish() {
    let mut f = setup();
    let d = f.doc.clone();
    let prepared = f
        .owner
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 0,
                    expected: "".into(),
                    inserted: "dirty".into(),
                }],
            }],
            &[],
            &mut f.recorder,
        )
        .unwrap();
    f.owner.commit(prepared, &mut f.recorder).unwrap();
    let before = f
        .owner
        .read(&d.document, &mut f.recorder)
        .unwrap()
        .source
        .clone();
    let generation = f.owner.generation();
    let (id, dispatch) = submitted(&mut f, b"source");
    complete(&mut f, &dispatch, b"external");
    let activation = f
        .owner
        .activate_source_read_v1(&id, &mut f.recorder)
        .unwrap();
    assert_eq!(
        activation.disposition,
        SourceReadActivationDispositionV1::Joined
    );
    assert_eq!(activation.document.document, d.document);
    assert_eq!(
        f.owner.read(&d.document, &mut f.recorder).unwrap().source,
        &before
    );
    assert_eq!(f.owner.generation(), generation);
}
#[test]
fn stop_is_not_quiescence_and_dispatch_and_evidence_pins_must_drain_before_retirement() {
    let mut f = setup();
    let (id, dispatch) = submitted(&mut f, b"new");
    let held = f.owner.source_reader_usage_v1().unwrap().held_bytes;
    f.owner.stop_source_read_v1(&id, &mut f.recorder).unwrap();
    assert_eq!(
        f.owner.retire_source_read_v1(&id, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    let late = f
        .port
        .source_read_failure_v1(&dispatch, SourceReadFailureV1::Cancelled)
        .unwrap();
    f.owner
        .record_source_read_v1(
            f.port.source_read_quiescence_v1(&dispatch).unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner.retire_source_read_v1(&id, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    drop(dispatch);
    assert_eq!(
        f.owner.retire_source_read_v1(&id, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    f.owner
        .record_source_read_v1(late, &mut f.recorder)
        .unwrap();
    f.owner.retire_source_read_v1(&id, &mut f.recorder).unwrap();
    let usage = f.owner.source_reader_usage_v1().unwrap();
    assert_eq!((usage.live, usage.issued, usage.held_bytes), (0, 1, 0));
    assert!(held > 0);
    assert!(matches!(
        f.owner.source_read_status_v1(&id, &mut f.recorder),
        Err(ResourceError::Unavailable)
    ));
}
#[test]
fn read_strength_foreign_target_and_fencing_refuse_without_spending_a_task() {
    let mut f = setup();
    let before = f.owner.source_reader_usage_v1().unwrap();
    assert_eq!(
        f.owner.admit_source_read_v1(
            f.port.source_read_target_v1(b"new").unwrap(),
            ReadStrength::AtomicResourceSnapshot,
            &mut f.recorder
        ),
        Err(ResourceError::Unsupported)
    );
    let other = Fixture::new();
    assert_eq!(
        f.owner.admit_source_read_v1(
            other.port.source_read_target_v1(b"new").unwrap(),
            ReadStrength::CapturedRead,
            &mut f.recorder
        ),
        Err(ResourceError::Unavailable)
    );
    assert_eq!(f.owner.source_reader_usage_v1().unwrap(), before);
    let id = admit(&mut f, b"new");
    f.owner
        .stop_source_read_admission_v1(&mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner.admit_source_read_v1(
            f.port.source_read_target_v1(b"later").unwrap(),
            ReadStrength::CapturedRead,
            &mut f.recorder
        ),
        Err(ResourceError::Unavailable)
    );
    f.owner.submit_source_read_v1(&id, &mut f.recorder).unwrap();
}
#[test]
fn reader_sponsorship_requires_protected_actual_drain_and_closed_inventory_never_reopens() {
    let mut f = setup();
    let reservation = f.owner.retention_usage();
    let id = admit(&mut f, b"new");
    f.owner
        .stop_source_read_admission_v1(&mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .retire_source_reader_v1(Some(true), Some(true), Some(true), &mut f.recorder),
        Err(ResourceError::Retained)
    );
    f.owner.stop_source_read_v1(&id, &mut f.recorder).unwrap();
    f.owner.retire_source_read_v1(&id, &mut f.recorder).unwrap();
    assert_eq!(f.owner.retention_usage(), reservation);
    for field in 0..3 {
        let mut facts = [Some(true); 3];
        facts[field] = None;
        assert_eq!(
            f.owner
                .retire_source_reader_v1(facts[0], facts[1], facts[2], &mut f.recorder),
            Err(ResourceError::Retained)
        );
    }
    f.owner
        .retire_source_reader_v1(Some(true), Some(true), Some(true), &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.retention_usage().reserved, 0);
    assert_eq!(
        f.owner
            .configure_source_reader_v1(limits(), &mut f.recorder),
        Err(ResourceError::Unavailable)
    );
}
#[test]
fn capture_off_zero_and_available_leave_source_read_activation_identical_without_private_labels() {
    for capacity in [None, Some(0), Some(128)] {
        let mut f = setup();
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let (id, dispatch) = submitted(&mut f, b"PRIVATE-KEY");
        complete(&mut f, &dispatch, b"PRIVATE-BODY");
        let activation = f
            .owner
            .activate_source_read_v1(&id, &mut f.recorder)
            .unwrap();
        assert_eq!(
            f.owner
                .read(&activation.document.document, &mut f.recorder)
                .unwrap()
                .source
                .text(),
            "PRIVATE-BODY"
        );
        let mut events = String::new();
        while let Some(event) = f.recorder.pop_front() {
            events.push_str(&format!("{event:?}"));
        }
        assert!(!events.contains("PRIVATE-KEY") && !events.contains("PRIVATE-BODY"));
        if capacity == Some(128) {
            assert!(
                events.contains("SourceReadEvidence") && events.contains("SourceReadActivation")
            );
        }
    }
}
#[test]
fn separate_reader_registration_can_join_and_reobserve_the_unchanged_writer_binding() {
    let mut f = setup();
    let domain = f.port.identity_domain();
    let reader = f
        .owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            Some(&domain),
            &mut f.recorder,
        )
        .unwrap();
    let id = f
        .owner
        .admit_source_read_v1(
            reader.source_read_target_v1(b"source").unwrap(),
            ReadStrength::CapturedRead,
            &mut f.recorder,
        )
        .unwrap();
    let dispatch = f.owner.submit_source_read_v1(&id, &mut f.recorder).unwrap();
    let v = SourceReadVersionV1::Opaque(b"v");
    for e in [
        reader.source_read_start_v1(&dispatch, v).unwrap(),
        reader.source_read_end_v1(&dispatch, v, 0, 0).unwrap(),
    ] {
        f.owner.record_source_read_v1(e, &mut f.recorder).unwrap();
    }
    assert_eq!(
        f.owner
            .activate_source_read_v1(&id, &mut f.recorder)
            .unwrap()
            .disposition,
        SourceReadActivationDispositionV1::Joined
    );
    assert_eq!(
        f.owner
            .activate_source_read_v1(&id, &mut f.recorder)
            .unwrap()
            .disposition,
        SourceReadActivationDispositionV1::AlreadyActivated
    );
}
#[test]
fn lifetime_slots_never_recycle_and_invalid_capacity_declarations_install_nothing() {
    let mut f = Fixture::new();
    f.owner
        .configure_retention_budget(1048576, &mut f.recorder)
        .unwrap();
    let mut l = limits();
    l.max_issued = usize::MAX;
    assert_eq!(
        f.owner.configure_source_reader_v1(l, &mut f.recorder),
        Err(ResourceError::Limit)
    );
    assert_eq!(f.owner.retention_usage().reserved, 0);
    let mut l = limits();
    l.max_live = 1;
    l.max_issued = 2;
    f.owner
        .configure_source_reader_v1(l, &mut f.recorder)
        .unwrap();
    let reserved = f.owner.retention_usage();
    for _ in 0..2 {
        let id = admit(&mut f, b"new");
        assert_eq!(
            f.owner.admit_source_read_v1(
                f.port.source_read_target_v1(b"other").unwrap(),
                ReadStrength::CapturedRead,
                &mut f.recorder
            ),
            Err(ResourceError::Limit)
        );
        f.owner.stop_source_read_v1(&id, &mut f.recorder).unwrap();
        f.owner.retire_source_read_v1(&id, &mut f.recorder).unwrap();
        assert_eq!(f.owner.retention_usage(), reserved);
    }
    assert_eq!(
        f.owner.admit_source_read_v1(
            f.port.source_read_target_v1(b"new").unwrap(),
            ReadStrength::CapturedRead,
            &mut f.recorder
        ),
        Err(ResourceError::Limit)
    );
    assert_eq!(f.owner.source_reader_usage_v1().unwrap().issued, 2);
}
#[test]
fn source_and_annotation_workers_share_outstanding_capacity_but_have_separate_read_inventories() {
    let mut f = Fixture::with_outstanding(1);
    f.owner
        .configure_retention_budget(1048576, &mut f.recorder)
        .unwrap();
    f.owner
        .configure_source_reader_v1(limits(), &mut f.recorder)
        .unwrap();
    let (_, dispatch) = submitted(&mut f, b"new");
    assert_eq!(f.owner.resource_usage().outstanding, 1);
    let target = f.port.annotation_read_target(b"comments").unwrap();
    let pair = f.pair();
    let annotation = f
        .owner
        .admit_annotation_read(
            &f.doc,
            0,
            target,
            ReadStrength::CapturedRead,
            pair,
            &mut f.recorder,
        )
        .unwrap();
    let pair = f.pair();
    assert!(matches!(
        f.owner
            .submit_annotation_read(&annotation, pair, &mut f.recorder),
        Err(ResourceError::Limit)
    ));
    f.owner
        .record_source_read_v1(
            f.port.source_read_quiescence_v1(&dispatch).unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let pair = f.pair();
    f.owner
        .submit_annotation_read(&annotation, pair, &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.source_reader_usage_v1().unwrap().issued, 1);
    assert_eq!(f.owner.resource_usage().issued_reads_lifetime, 1);
}
