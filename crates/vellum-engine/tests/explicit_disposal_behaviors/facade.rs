use super::*;
#[test]
fn v2_facade_executes_explicit_disposal_with_one_original_owner_and_separate_result() {
    for bits in 0..4 {
        let (mut o, mut r) = fixture::owner();
        let c = fixture::context();
        let f = facts(&c);
        let a = admission(&f);
        let g = coverage();
        let doc = o.create(b"original", &mut r).unwrap();
        let mut q = request(&doc, o.lifecycle_state(&doc.document, &mut r).unwrap());
        if bits & 1 != 0 {
            q.source_disposition = DisposalDispositionV1::Discard;
        }
        if bits & 2 != 0 {
            q.annotation_disposition = DisposalDispositionV1::Discard;
        }
        let p = o.prepare_explicit_disposal_v1(&q, &mut r).unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        let generation = o.generation();
        o.submit_native_document_lifecycle_v2(
            &id,
            NativeDocumentLifecycleCommandV2::ExplicitDisposal(&p),
            NativeDocumentLifecycleAdmissionV2::ExplicitDisposal {
                native: &a,
                coverage: &g,
            },
            &mut r,
        )
        .unwrap();
        assert_eq!(o.generation(), generation);
        o.run_native_document_lifecycle_v2(
            &id,
            NativeDocumentLifecycleServiceV2::ExplicitDisposal {
                native: &f,
                coverage: &g,
            },
            &mut r,
        )
        .unwrap();
        let result = o
            .retained_disposal_result_v1(&id, &release(&c), &mut r)
            .unwrap();
        assert_eq!(result.document, doc.document);
        assert_eq!(result.invocation, id);
        o.submit_explicit_disposal_v1(&id, &p, &a, &g, &mut r)
            .unwrap();
        o.run_native_document_lifecycle_v2(
            &id,
            NativeDocumentLifecycleServiceV2::ExplicitDisposal {
                native: &f,
                coverage: &g,
            },
            &mut r,
        )
        .unwrap();
        assert_eq!(o.generation(), generation + 1);
        assert_eq!(o.invocation_usage().issued_lifetime, 1);
        o.retire_retained_disposal_v1(&id, &fixture::control(&c), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
    }
}

#[test]
fn v2_facade_mismatches_and_lost_coverage_do_not_admit_or_execute() {
    let (mut o, mut r) = fixture::owner();
    let c = fixture::context();
    let f = facts(&c);
    let a = admission(&f);
    let g = coverage();
    let doc = o.create(b"original", &mut r).unwrap();
    let q = request(&doc, o.lifecycle_state(&doc.document, &mut r).unwrap());
    let p = o.prepare_explicit_disposal_v1(&q, &mut r).unwrap();
    let legacy = o.prepare_retained_disposal(&doc, 0, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let held = o.retention_usage();
    let generation = o.generation();
    let existing = NativeDocumentLifecycleAdmissionV2::Existing(
        NativeDocumentLifecycleAdmissionV1::PreparedDisposal(&a),
    );
    let explicit = NativeDocumentLifecycleAdmissionV2::ExplicitDisposal {
        native: &a,
        coverage: &g,
    };
    assert_eq!(
        o.submit_native_document_lifecycle_v2(
            &id,
            NativeDocumentLifecycleCommandV2::ExplicitDisposal(&p),
            existing,
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        o.submit_native_document_lifecycle_v2(
            &id,
            NativeDocumentLifecycleCommandV2::Existing(
                NativeDocumentLifecycleCommandV1::PreparedDisposal(&legacy)
            ),
            explicit,
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    let mut denied = coverage();
    denied.coverage_complete = None;
    assert_eq!(
        o.submit_native_document_lifecycle_v2(
            &id,
            NativeDocumentLifecycleCommandV2::ExplicitDisposal(&p),
            NativeDocumentLifecycleAdmissionV2::ExplicitDisposal {
                native: &a,
                coverage: &denied
            },
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    assert_eq!(o.retention_usage(), held);
    assert_eq!(o.generation(), generation);
    o.submit_native_document_lifecycle_v2(
        &id,
        NativeDocumentLifecycleCommandV2::ExplicitDisposal(&p),
        explicit,
        &mut r,
    )
    .unwrap();
    let held = o.retention_usage();
    for service in [
        NativeDocumentLifecycleServiceV2::Existing(
            NativeDocumentLifecycleServiceV1::PreparedDisposal(&f),
        ),
        NativeDocumentLifecycleServiceV2::ExplicitDisposal {
            native: &f,
            coverage: &denied,
        },
    ] {
        assert_eq!(
            o.run_native_document_lifecycle_v2(&id, service, &mut r),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(
            o.invocation_state(&id, &mut r),
            Ok(InvocationState::Pending)
        );
        assert_eq!(o.retention_usage(), held);
        assert_eq!(o.generation(), generation);
    }
    o.run_native_document_lifecycle_v2(
        &id,
        NativeDocumentLifecycleServiceV2::ExplicitDisposal {
            native: &f,
            coverage: &g,
        },
        &mut r,
    )
    .unwrap();
    let mut no = release(&c);
    no.result_allowed = None;
    assert_eq!(
        o.retained_disposal_result_v1(&id, &no, &mut r),
        Err(InvocationError::Unavailable)
    );
    assert!(
        o.retained_disposal_result_v1(&id, &release(&c), &mut r)
            .is_ok()
    );
}

#[test]
fn v2_facade_preserves_protected_stop_known_stale_and_spent_forgetting() {
    for stopped in [false, true] {
        let (mut o, mut r) = fixture::owner();
        let c = fixture::context();
        let f = facts(&c);
        let a = admission(&f);
        let g = coverage();
        let doc = o.create(b"original", &mut r).unwrap();
        let life = o.lifecycle_state(&doc.document, &mut r).unwrap();
        let p = o
            .prepare_explicit_disposal_v1(&request(&doc, life), &mut r)
            .unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_native_document_lifecycle_v2(
            &id,
            NativeDocumentLifecycleCommandV2::ExplicitDisposal(&p),
            NativeDocumentLifecycleAdmissionV2::ExplicitDisposal {
                native: &a,
                coverage: &g,
            },
            &mut r,
        )
        .unwrap();
        if stopped {
            o.request_invocation_stop(
                &id,
                &TrustedInvocationControlV1 {
                    context: &c,
                    owner_control_allowed: Some(true),
                    control_context_current: Some(true),
                    control_scope_valid: Some(true),
                    protected_control_ready: Some(true),
                },
                &mut r,
            )
            .unwrap();
            let mut lost = facts(&c);
            lost.disposal_allowed = None;
            let mut lost_coverage = coverage();
            lost_coverage.owner_control_allowed = Some(false);
            o.run_native_document_lifecycle_v2(
                &id,
                NativeDocumentLifecycleServiceV2::ExplicitDisposal {
                    native: &lost,
                    coverage: &lost_coverage,
                },
                &mut r,
            )
            .unwrap();
            assert!(matches!(
                o.invocation_outcome(&id, &mut r).unwrap(),
                InvocationOutcome::StoppedBeforePublication
            ));
        } else {
            o.create(b"intervening publication", &mut r).unwrap();
            o.run_native_document_lifecycle_v2(
                &id,
                NativeDocumentLifecycleServiceV2::ExplicitDisposal {
                    native: &f,
                    coverage: &g,
                },
                &mut r,
            )
            .unwrap();
            assert!(matches!(
                o.invocation_outcome(&id, &mut r).unwrap(),
                InvocationOutcome::RetainedDisposalRejected(CoordinatorError::Stale)
            ));
        }
        assert_eq!(o.lifecycle_state(&doc.document, &mut r).unwrap(), life);
        let generation = o.generation();
        let held = o.retention_usage();
        o.forget_invocation_outcome(&id, &mut r).unwrap();
        o.submit_native_document_lifecycle_v2(
            &id,
            NativeDocumentLifecycleCommandV2::ExplicitDisposal(&p),
            NativeDocumentLifecycleAdmissionV2::ExplicitDisposal {
                native: &a,
                coverage: &g,
            },
            &mut r,
        )
        .unwrap();
        o.run_native_document_lifecycle_v2(
            &id,
            NativeDocumentLifecycleServiceV2::ExplicitDisposal {
                native: &f,
                coverage: &g,
            },
            &mut r,
        )
        .unwrap();
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage(), held);
        o.retire_retained_disposal_v1(&id, &fixture::control(&c), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
    }
}

#[test]
fn v2_facade_existing_and_explicit_paths_each_record_one_outer_attempt_in_all_capture_modes() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation, Recorder,
    };
    for explicit in [false, true] {
        for mode in 0..4 {
            let (mut o, mut setup) = fixture::owner();
            let c = fixture::context();
            let f = facts(&c);
            let a = admission(&f);
            let g = coverage();
            let s = fixture::facts(&c);
            let sa = fixture::admission(&s);
            let sq = fixture::request(b"PRIVATE");
            let doc = o.create(b"PRIVATE", &mut setup).unwrap();
            let p = o
                .prepare_explicit_disposal_v1(
                    &request(&doc, o.lifecycle_state(&doc.document, &mut setup).unwrap()),
                    &mut setup,
                )
                .unwrap();
            let id = o.issue_invocation(&mut setup).unwrap();
            let generation = o.generation();
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
            if explicit {
                o.submit_native_document_lifecycle_v2(
                    &id,
                    NativeDocumentLifecycleCommandV2::ExplicitDisposal(&p),
                    NativeDocumentLifecycleAdmissionV2::ExplicitDisposal {
                        native: &a,
                        coverage: &g,
                    },
                    &mut r,
                )
                .unwrap();
                o.run_native_document_lifecycle_v2(
                    &id,
                    NativeDocumentLifecycleServiceV2::ExplicitDisposal {
                        native: &f,
                        coverage: &g,
                    },
                    &mut r,
                )
                .unwrap();
            } else {
                // Compile-time exhaustiveness check: V1 has no new executable variant.
                let old = NativeDocumentLifecycleCommandV1::Scratch(&sq);
                let name = match old {
                    NativeDocumentLifecycleCommandV1::Scratch(_) => "scratch",
                    NativeDocumentLifecycleCommandV1::LiveRelation(_) => "relation",
                    NativeDocumentLifecycleCommandV1::PreparedDisposal(_) => "prepared",
                };
                assert_eq!(name, "scratch");
                o.submit_native_document_lifecycle_v2(
                    &id,
                    NativeDocumentLifecycleCommandV2::Existing(old),
                    NativeDocumentLifecycleAdmissionV2::Existing(
                        NativeDocumentLifecycleAdmissionV1::Scratch(&sa),
                    ),
                    &mut r,
                )
                .unwrap();
                o.run_native_document_lifecycle_v2(
                    &id,
                    NativeDocumentLifecycleServiceV2::Existing(
                        NativeDocumentLifecycleServiceV1::Scratch(&s),
                    ),
                    &mut r,
                )
                .unwrap();
            }
            assert_eq!(o.generation(), generation + 1);
            assert_eq!(o.invocation_usage().issued_lifetime, 1);
            let mut counts = [0; 4];
            while let Some(e) = r.pop_front() {
                let i = match e.boundary {
                    Boundary::DocumentLifecycleFacadeAdmission => Some(0),
                    Boundary::DocumentLifecycleFacadeService => Some(1),
                    Boundary::RetainedDisposalAdmission | Boundary::ScratchInvocationAdmission => {
                        Some(2)
                    }
                    Boundary::RetainedDisposalService | Boundary::ScratchInvocationService => {
                        Some(3)
                    }
                    _ => None,
                };
                if let Some(i) = i {
                    counts[i] += 1;
                    assert_eq!(e.disposition, Disposition::Completed);
                    assert_eq!(e.service_time.is_some(), mode == 2);
                }
                assert!(!format!("{e:?}").contains("PRIVATE"));
            }
            assert_eq!(
                counts,
                if mode == 1 || mode == 2 {
                    [1, 1, 1, 1]
                } else {
                    [0, 0, 0, 0]
                }
            );
        }
    }
}
