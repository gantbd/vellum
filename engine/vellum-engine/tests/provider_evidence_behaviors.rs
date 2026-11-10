//! Trusted native intake, not a remote authentication or filesystem test.
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1, Recorder};
use vellum_engine::resource::*;
#[path = "support/annotation_read_fixture.rs"]
#[allow(dead_code)]
mod support;
use support::Fixture;

fn source(f: &mut Fixture) -> (SourceReadIdV1, SourceReadDispatchV1) {
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
    let d = f.owner.submit_source_read_v1(&id, &mut f.recorder).unwrap();
    (id, d)
}
macro_rules! ingest {
    ($f:ident, $e:expr) => {{
        let evidence = $e;
        assert_eq!(format!("{evidence:?}"), "ProviderEvidenceV1(<redacted>)");
        $f.owner
            .ingest_provider_evidence_v1(evidence, &mut $f.recorder)
    }};
}
#[test]
fn source_progress_completion_duplicate_conflict_and_quiescence_stay_at_the_original_owner() {
    let mut f = Fixture::new();
    let (id, d) = source(&mut f);
    let generation = f.owner.generation();
    let v = SourceReadVersionV1::Opaque(b"original-version");
    assert_eq!(
        ingest!(
            f,
            ProviderEvidenceV1::SourceRead(f.port.source_read_start_v1(&d, v).unwrap())
        ),
        Ok(EvidenceDisposition::Accepted)
    );
    assert_eq!(
        ingest!(
            f,
            ProviderEvidenceV1::SourceRead(
                f.port.source_read_chunk_v1(&d, 0, 0, b"prose").unwrap()
            )
        ),
        Ok(EvidenceDisposition::Accepted)
    );
    assert!(
        !f.owner
            .source_read_status_v1(&id, &mut f.recorder)
            .unwrap()
            .candidate_ready
    );
    assert_eq!(
        ingest!(
            f,
            ProviderEvidenceV1::SourceRead(
                f.port.source_read_chunk_v1(&d, 0, 0, b"prose").unwrap()
            )
        ),
        Ok(EvidenceDisposition::Duplicate)
    );
    assert_eq!(
        ingest!(
            f,
            ProviderEvidenceV1::SourceRead(f.port.source_read_end_v1(&d, v, 1, 5).unwrap())
        ),
        Ok(EvidenceDisposition::Accepted)
    );
    let s = f.owner.source_read_status_v1(&id, &mut f.recorder).unwrap();
    assert!(s.candidate_ready && !s.quiescent && s.activated.is_none());
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(
        ingest!(
            f,
            ProviderEvidenceV1::SourceRead(
                f.port
                    .source_read_end_v1(&d, SourceReadVersionV1::Opaque(b"different-version"), 1, 5)
                    .unwrap()
            )
        ),
        Ok(EvidenceDisposition::Contradiction)
    );
    assert!(
        f.owner
            .source_read_status_v1(&id, &mut f.recorder)
            .unwrap()
            .contested
    );
    f.owner.stop_source_read_v1(&id, &mut f.recorder).unwrap();
    assert_eq!(
        f.owner.retire_source_read_v1(&id, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(
        ingest!(
            f,
            ProviderEvidenceV1::SourceRead(f.port.source_read_quiescence_v1(&d).unwrap())
        ),
        Ok(EvidenceDisposition::Accepted)
    );
    let late = f.port.source_read_quiescence_v1(&d).unwrap();
    drop(d);
    assert_eq!(
        f.owner.retire_source_read_v1(&id, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(
        ingest!(f, ProviderEvidenceV1::SourceRead(late)),
        Ok(EvidenceDisposition::Duplicate)
    );
    f.owner.retire_source_read_v1(&id, &mut f.recorder).unwrap();
}
#[test]
fn annotation_completion_does_not_restore_or_publish_and_late_evidence_survives_stop() {
    let mut f = Fixture::new();
    let id = f.admit();
    let d = f.submit(&id);
    let generation = f.owner.generation();
    for e in [
        f.port.annotation_read_start(&d, b"a0").unwrap(),
        f.port.annotation_read_chunk(&d, 0, 0, b"{}").unwrap(),
        f.port.annotation_read_end(&d, b"a0", 1, 2).unwrap(),
    ] {
        assert_eq!(
            ingest!(f, ProviderEvidenceV1::AnnotationRead(e)),
            Ok(EvidenceDisposition::Accepted)
        );
    }
    let s = f
        .owner
        .annotation_read_status(&id, &mut f.recorder)
        .unwrap();
    assert!(s.complete && !s.restored && !s.quiescent && !s.contested);
    assert_eq!(f.capture(&id).unwrap().bytes(), b"{}");
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(
        ingest!(
            f,
            ProviderEvidenceV1::AnnotationRead(
                f.port.annotation_read_end(&d, b"a0", 1, 2).unwrap()
            )
        ),
        Ok(EvidenceDisposition::Duplicate)
    );
    f.owner.stop_annotation_read(&id, &mut f.recorder).unwrap();
    assert_eq!(
        f.owner.retire_annotation_read(&id, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(
        ingest!(
            f,
            ProviderEvidenceV1::AnnotationRead(f.port.annotation_read_quiescence(&d).unwrap())
        ),
        Ok(EvidenceDisposition::Accepted)
    );
    drop(d);
    f.owner
        .retire_annotation_read(&id, &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.resource_usage().retained_reads, 0);
}
fn save(f: &mut Fixture) -> (SaveId, SaveDispatch) {
    let pair = f.pair().unwrap();
    let id = f
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
    let pair = f.pair().unwrap();
    let d = f
        .owner
        .dispatch_save_with_pair(
            &id,
            f.port.read_result(b"source", b"s0", b"text").unwrap(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    (id, d)
}
#[test]
fn save_progress_unknown_application_failure_and_acknowledgment_are_not_quiescence_or_baseline_publication()
 {
    for result_kind in 0..3 {
        let mut f = Fixture::new();
        let (id, d) = save(&mut f);
        let generation = f.owner.generation();
        let (app, observed, sync, version): (_, _, _, Option<&[u8]>) = match result_kind {
            0 => (Application::Unknown, 1, Synchronization::Unknown, None),
            1 => (
                Application::NotWritten,
                0,
                Synchronization::NotRequested,
                None,
            ),
            _ => (
                Application::Written,
                4,
                Synchronization::NotRequested,
                Some(b"v1"),
            ),
        };
        assert_eq!(
            ingest!(
                f,
                ProviderEvidenceV1::Save(f.port.result(&d, app, observed, sync, version).unwrap())
            ),
            Ok(EvidenceDisposition::Accepted)
        );
        assert_eq!(
            ingest!(
                f,
                ProviderEvidenceV1::Save(f.port.result(&d, app, observed, sync, version).unwrap())
            ),
            Ok(EvidenceDisposition::Duplicate)
        );
        let outcome = f.owner.save_outcome(&id, &mut f.recorder).unwrap();
        assert_eq!(
            (
                outcome.application,
                outcome.observed_bytes,
                outcome.synchronization
            ),
            (app, observed, sync)
        );
        assert!(!outcome.quiescent && !outcome.no_future_effects);
        assert_eq!(f.owner.generation(), generation);
        assert_eq!(
            f.owner.retire_save(&id, &mut f.recorder),
            Err(ResourceError::Retained)
        );
        f.owner.stop_save(&id, &mut f.recorder).unwrap();
        assert_eq!(
            ingest!(
                f,
                ProviderEvidenceV1::Save(f.port.no_future_effects(&d).unwrap())
            ),
            Ok(EvidenceDisposition::Accepted)
        );
        assert!(
            !f.owner
                .save_outcome(&id, &mut f.recorder)
                .unwrap()
                .quiescent
        );
        assert_eq!(
            ingest!(f, ProviderEvidenceV1::Save(f.port.quiescence(&d).unwrap())),
            Ok(EvidenceDisposition::Accepted)
        );
        assert_eq!(
            ingest!(f, ProviderEvidenceV1::Save(f.port.quiescence(&d).unwrap())),
            Ok(EvidenceDisposition::Duplicate)
        );
        drop(d);
        if result_kind == 1 {
            f.owner.retire_save(&id, &mut f.recorder).unwrap();
        } else {
            // Unknown application and a qualified current baseline candidate
            // retain their existing obligations; the facade must not waive them.
            assert_eq!(
                f.owner.retire_save(&id, &mut f.recorder),
                Err(ResourceError::Retained)
            );
        }
    }
}
#[test]
fn foreign_engine_and_wrong_provider_cannot_attribute_evidence_to_matching_local_slots() {
    let mut f = Fixture::new();
    let mut g = Fixture::new();
    let (s, sd) = source(&mut f);
    let (gs, gsd) = source(&mut g);
    let a = f.admit();
    let ad = f.submit(&a);
    let ga = g.admit();
    let gad = g.submit(&ga);
    let (w, wd) = save(&mut f);
    let (gw, gwd) = save(&mut g);
    assert!(f.port.source_read_quiescence_v1(&gsd).is_err());
    assert!(f.port.annotation_read_quiescence(&gad).is_err());
    assert!(f.port.quiescence(&gwd).is_err());
    let source_before = g.owner.source_read_status_v1(&gs, &mut g.recorder).unwrap();
    let annotation_before = g
        .owner
        .annotation_read_status(&ga, &mut g.recorder)
        .unwrap();
    let save_before = g.owner.save_outcome(&gw, &mut g.recorder).unwrap();
    for e in [
        ProviderEvidenceV1::SourceRead(f.port.source_read_quiescence_v1(&sd).unwrap()),
        ProviderEvidenceV1::AnnotationRead(f.port.annotation_read_quiescence(&ad).unwrap()),
        ProviderEvidenceV1::Save(f.port.quiescence(&wd).unwrap()),
    ] {
        assert_eq!(ingest!(g, e), Err(ResourceError::Unavailable));
    }
    assert_eq!(
        g.owner.source_read_status_v1(&gs, &mut g.recorder).unwrap(),
        source_before
    );
    assert_eq!(
        g.owner
            .annotation_read_status(&ga, &mut g.recorder)
            .unwrap(),
        annotation_before
    );
    assert_eq!(
        g.owner.save_outcome(&gw, &mut g.recorder).unwrap(),
        save_before
    );
    // The same original issuer still records its own admitted work after stop.
    f.owner.stop_source_read_v1(&s, &mut f.recorder).unwrap();
    f.owner.stop_annotation_read(&a, &mut f.recorder).unwrap();
    f.owner.stop_save(&w, &mut f.recorder).unwrap();
    for e in [
        ProviderEvidenceV1::SourceRead(
            f.port
                .source_read_failure_v1(&sd, SourceReadFailureV1::IoFailure)
                .unwrap(),
        ),
        ProviderEvidenceV1::AnnotationRead(
            f.port
                .annotation_read_failure(&ad, ReadFailure::Interrupted)
                .unwrap(),
        ),
        ProviderEvidenceV1::Save(
            f.port
                .result(
                    &wd,
                    Application::NotWritten,
                    0,
                    Synchronization::NotRequested,
                    None,
                )
                .unwrap(),
        ),
        ProviderEvidenceV1::SourceRead(f.port.source_read_quiescence_v1(&sd).unwrap()),
        ProviderEvidenceV1::AnnotationRead(f.port.annotation_read_quiescence(&ad).unwrap()),
        ProviderEvidenceV1::Save(f.port.no_future_effects(&wd).unwrap()),
        ProviderEvidenceV1::Save(f.port.quiescence(&wd).unwrap()),
    ] {
        assert_eq!(ingest!(f, e), Ok(EvidenceDisposition::Accepted));
    }
    drop((sd, ad, wd));
    f.owner.retire_source_read_v1(&s, &mut f.recorder).unwrap();
    f.owner.retire_annotation_read(&a, &mut f.recorder).unwrap();
    f.owner.retire_save(&w, &mut f.recorder).unwrap();
}
#[test]
fn optional_capture_and_timing_preserve_all_three_ingress_outcomes_and_fixed_populations() {
    use vellum_engine::instrumentation::Disposition;
    for mode in 0..4 {
        let mut f = Fixture::new();
        let (s, sd) = source(&mut f);
        let a = f.admit();
        let ad = f.submit(&a);
        let (w, wd) = save(&mut f);
        f.recorder = match mode {
            0 => Recorder::disabled(),
            1 => Recorder::try_with_capacity(128).unwrap(),
            2 => Recorder::try_with_capacity(128)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1),
            _ => Recorder::try_with_capacity(1)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1),
        };
        let held = f.owner.retention_usage();
        let generation = f.owner.generation();
        for repeat in 0..2 {
            let expected = if repeat == 0 {
                EvidenceDisposition::Accepted
            } else {
                EvidenceDisposition::Duplicate
            };
            for e in [
                ProviderEvidenceV1::SourceRead(
                    f.port
                        .source_read_start_v1(&sd, SourceReadVersionV1::Opaque(b"private-version"))
                        .unwrap(),
                ),
                ProviderEvidenceV1::AnnotationRead(
                    f.port
                        .annotation_read_start(&ad, b"private-version")
                        .unwrap(),
                ),
                ProviderEvidenceV1::Save(
                    f.port
                        .result(&wd, Application::Unknown, 1, Synchronization::Unknown, None)
                        .unwrap(),
                ),
            ] {
                assert_eq!(ingest!(f, e), Ok(expected));
            }
        }
        for e in [
            ProviderEvidenceV1::SourceRead(
                f.port
                    .source_read_start_v1(&sd, SourceReadVersionV1::Opaque(b"other-version"))
                    .unwrap(),
            ),
            ProviderEvidenceV1::AnnotationRead(
                f.port.annotation_read_start(&ad, b"other-version").unwrap(),
            ),
            ProviderEvidenceV1::Save(
                f.port
                    .result(
                        &wd,
                        Application::NotWritten,
                        0,
                        Synchronization::NotRequested,
                        None,
                    )
                    .unwrap(),
            ),
        ] {
            assert_eq!(ingest!(f, e), Ok(EvidenceDisposition::Contradiction));
        }
        assert!(
            f.owner
                .source_read_status_v1(&s, &mut f.recorder)
                .unwrap()
                .contested
        );
        assert!(
            f.owner
                .annotation_read_status(&a, &mut f.recorder)
                .unwrap()
                .contested
        );
        assert!(f.owner.save_outcome(&w, &mut f.recorder).unwrap().contested);
        assert_eq!(f.owner.retention_usage(), held);
        assert_eq!(f.owner.generation(), generation);
        let mut primary = 0;
        let mut duplicate = 0;
        let mut conflict = 0;
        let mut native_timed = 0;
        while let Some(o) = f.recorder.pop_front() {
            assert!(!format!("{o:?}").contains("private-version"));
            match o.boundary {
                Boundary::ProviderEvidenceIntake => {
                    primary += 1;
                    assert_eq!(o.service_time.is_some(), mode == 2);
                    assert_eq!(
                        o.disposition,
                        if primary <= 6 {
                            Disposition::Completed
                        } else {
                            Disposition::Unknown
                        }
                    );
                }
                Boundary::ProviderEvidenceDuplicate => duplicate += 1,
                Boundary::ProviderEvidenceContradiction => conflict += 1,
                Boundary::SourceReadEvidence
                | Boundary::AnnotationReadEvidence
                | Boundary::SaveEvidence
                    if o.service_time.is_some() =>
                {
                    native_timed += 1;
                }
                _ => {}
            }
        }
        if mode == 1 || mode == 2 {
            assert_eq!((primary, duplicate, conflict), (9, 3, 3));
            assert_eq!(native_timed, if mode == 2 { 9 } else { 0 });
        }
    }
}
#[test]
fn causal_write_edges_are_forwarded_without_becoming_application_or_completion_facts() {
    let mut f = Fixture::new();
    let p = f
        .owner
        .register_provider_with_causal_capabilities(
            ProviderProfile {
                synchronization_supported: false,
            },
            ProviderDestinationCapabilitiesV1::default(),
            ProviderCausalCapabilitiesV1 {
                causal_receipts: true,
                non_reuse_tokens: false,
            },
            None,
            &mut f.recorder,
        )
        .unwrap();
    let before = ResourceVersionV1 {
        epoch: b"epoch",
        token: b"v0",
    };
    let after = ResourceVersionV1 {
        epoch: b"epoch",
        token: b"v1",
    };
    let document = f
        .owner
        .load_resource(
            p.read_result_causal(b"causal", before, b"A").unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let id = f
        .owner
        .queue_pinned_save(
            &document,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            &mut f.recorder,
        )
        .unwrap();
    let d = f
        .owner
        .dispatch_save(
            &id,
            p.read_result_causal(b"causal", before, b"A").unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    for repeat in 0..2 {
        assert_eq!(
            ingest!(
                f,
                ProviderEvidenceV1::Save(p.causal_edge(&d, before, after).unwrap())
            ),
            Ok(if repeat == 0 {
                EvidenceDisposition::Accepted
            } else {
                EvidenceDisposition::Duplicate
            })
        );
    }
    let outcome = f.owner.save_outcome(&id, &mut f.recorder).unwrap();
    assert_eq!(outcome.application, Application::Unknown);
    assert!(!outcome.quiescent && !outcome.no_future_effects);
    assert_eq!(
        ingest!(
            f,
            ProviderEvidenceV1::Save(
                p.result_causal(
                    &d,
                    Application::Written,
                    1,
                    Synchronization::NotRequested,
                    Some(after)
                )
                .unwrap()
            )
        ),
        Ok(EvidenceDisposition::Accepted)
    );
    let outcome = f.owner.save_outcome(&id, &mut f.recorder).unwrap();
    assert!(outcome.version_available && !outcome.quiescent && !outcome.no_future_effects);
}
