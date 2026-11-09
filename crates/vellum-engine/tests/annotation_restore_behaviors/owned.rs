use super::*;
use std::mem::size_of;
use std::sync::Arc;
use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1, Recorder};
pub(super) fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "load".into(),
        destination: "local".into(),
        profile: "annotation-load/1".into(),
        continuity: "c".into(),
    }
}
pub(super) fn facts(c: &SettingsExecutionContextV1) -> TrustedAnnotationLoadFactsV1<'_> {
    TrustedAnnotationLoadFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        interpretation_current: Some(true),
        resources_ready: Some(true),
        preparation_reserved: Some(true),
        owner_control_allowed: Some(true),
        companions_ready: Some(true),
        reply_ready: Some(true),
        disposition_readable: Some(true),
        annotation_writable: Some(true),
        restoration_allowed: Some(true),
        identity_restorable: Some(true),
        source_evidence_ready: Some(true),
        aliases_current: Some(true),
        target_evidence_ready: Some(true),
        history_restoration_ready: Some(true),
        annotation_readable: Some(true),
        binding_coverage_complete: Some(true),
        writer_coverage_complete: Some(true),
        baseline_publication_allowed: Some(true),
        historical_artifact_readable: Some(true),
        baseline_coverage_complete: Some(true),
    }
}
pub(super) fn admission<'a>(
    f: &'a TrustedAnnotationLoadFactsV1<'a>,
) -> AnnotationLoadAdmissionV1<'a> {
    AnnotationLoadAdmissionV1 {
        facts: f,
        limits: AnnotationLoadLimitsV1 {
            max_capture_bytes: 8192,
            max_result_bytes: 256,
        },
        publication_epoch: 7,
        publication_allowed: Some(true),
        result_admission_allowed: Some(true),
        result_allowed: Some(true),
    }
}
pub(super) fn release(c: &SettingsExecutionContextV1) -> TrustedAnnotationLoadResultV1<'_> {
    TrustedAnnotationLoadResultV1 {
        context: c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        outcome_readable: Some(true),
        result_allowed: Some(true),
        result_context_current: Some(true),
        historical_document_readable: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
    }
}
pub(super) fn control(c: &SettingsExecutionContextV1) -> InvocationObservationPolicyControlV1<'_> {
    InvocationObservationPolicyControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(super) fn ready(
    count: usize,
) -> (
    Fixture,
    AnnotationReadId,
    AnnotationReadDispatch,
    AnnotationLoadAuthorityV1,
    AnnotationLoadRequestV1,
) {
    ready_with_retention(count, 1048576)
}
pub(super) fn ready_with_retention(
    count: usize,
    budget: usize,
) -> (
    Fixture,
    AnnotationReadId,
    AnnotationReadDispatch,
    AnnotationLoadAuthorityV1,
    AnnotationLoadRequestV1,
) {
    let mut f = Fixture::with_invocations();
    f.owner
        .configure_retention_budget(budget, &mut f.recorder)
        .unwrap();
    let (read, dispatch, capture) = captured(&mut f);
    let encoder = f
        .owner
        .register_annotation_encoder(&mut f.recorder)
        .unwrap();
    let restorer = f
        .owner
        .register_annotation_restorer_for_encoding(&encoder.context(), &mut f.recorder)
        .unwrap();
    let snapshot = source(&mut f);
    let records = if count == 0 {
        vec![]
    } else {
        vec![row(&restorer, &f.doc)]
    };
    let package = Arc::new(restorer.seal(capture, vec![snapshot], records).unwrap());
    let pair = f.pair();
    let auth = f
        .owner
        .borrow_annotation_load_authority_v1(
            &read,
            &AnnotationLoadAuthorityFactsV1 {
                owner_control_allowed: Some(true),
                preparation_reserved: Some(true),
                resources_ready: Some(true),
            },
            &mut f.recorder,
        )
        .unwrap();
    (
        f,
        read,
        dispatch,
        auth,
        AnnotationLoadRequestV1::Restore { package, pair },
    )
}
#[test]
fn both_phases_preserve_all_four_native_dispositions_original_replay_and_exact_cleanup() {
    for count in [0, 1] {
        let (mut f, read, _, auth, request) = ready(count);
        let c = context();
        let current = facts(&c);
        let a = admission(&current);
        let before = f.owner.retention_usage();
        let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        let generation = f.owner.generation();
        assert_eq!(
            f.owner
                .submit_annotation_load_v1(&id, &auth, &request, &a, &mut f.recorder),
            Ok(InvocationState::Pending)
        );
        assert_eq!(f.owner.generation(), generation);
        assert_eq!(
            f.owner.run(&id, &mut f.recorder),
            Err(InvocationError::Unavailable)
        );
        f.owner
            .run_annotation_load_v1(&id, &current, &mut f.recorder)
            .unwrap();
        let restored = f
            .owner
            .annotation_load_result_v1(&id, &release(&c), &mut f.recorder)
            .unwrap();
        assert_eq!(
            restored.outcome,
            AnnotationLoadOutcomeV1::Restoration {
                document: f.doc.document.clone(),
                changed: count != 0
            }
        );
        assert_eq!(f.owner.generation(), generation + u64::from(count != 0));
        let set = f
            .owner
            .annotation_restoration_outcome(&read, &mut f.recorder)
            .unwrap();
        assert_eq!(set.annotations.len(), count);
        assert_eq!(
            f.owner
                .submit_annotation_load_v1(&id, &auth, &request, &a, &mut f.recorder),
            Ok(InvocationState::Finished)
        );
        f.owner
            .run_annotation_load_v1(&id, &current, &mut f.recorder)
            .unwrap();
        f.owner
            .attach_annotation_read_baselines(&mut f.recorder)
            .unwrap();
        f.owner
            .stop_annotation_read(&read, &mut f.recorder)
            .unwrap();
        let baseline = AnnotationLoadRequestV1::AdoptBaseline {
            read: read.clone(),
            pair: f.pair(),
            max_baseline_bytes: 65536,
        };
        let mut ids = vec![id.clone()];
        for changed in [true, false] {
            let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
            let generation = f.owner.generation();
            f.owner
                .submit_annotation_load_v1(&id, &auth, &baseline, &a, &mut f.recorder)
                .unwrap();
            f.owner
                .run_annotation_load_v1(&id, &current, &mut f.recorder)
                .unwrap();
            assert_eq!(
                f.owner
                    .annotation_load_result_v1(&id, &release(&c), &mut f.recorder)
                    .unwrap()
                    .outcome,
                AnnotationLoadOutcomeV1::Baseline {
                    document: f.doc.document.clone(),
                    changed
                }
            );
            assert_eq!(f.owner.generation(), generation + u64::from(changed));
            ids.push(id);
        }
        assert_eq!(
            f.owner
                .annotation_load_result_v1(&id, &release(&c), &mut f.recorder)
                .unwrap(),
            restored
        );
        let resources = f.owner.resource_usage();
        for id in ids {
            f.owner
                .forget_invocation_outcome(&id, &mut f.recorder)
                .unwrap();
            f.owner
                .retire_annotation_load_v1(&id, &control(&c), &mut f.recorder)
                .unwrap();
            assert_eq!(
                f.owner
                    .retire_annotation_load_v1(&id, &control(&c), &mut f.recorder),
                Err(InvocationError::Unavailable)
            );
            f.owner.retire_invocation(&id, &mut f.recorder).unwrap();
        }
        assert_eq!(f.owner.retention_usage(), before);
        assert_eq!(f.owner.resource_usage(), resources);
    }
}
#[test]
fn capture_and_result_bounds_original_artifact_and_disclosure_ceilings_are_not_upgradeable() {
    let (mut f, _, _, auth, request) = ready(1);
    let c = context();
    let current = facts(&c);
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    let before = f.owner.retention_usage();
    let mut a = admission(&current);
    let exact = AnnotationLoadLimitsV1::required_capture_bytes(&c, &request).unwrap();
    a.limits.max_capture_bytes = exact - 1;
    assert_eq!(
        f.owner
            .submit_annotation_load_v1(&id, &auth, &request, &a, &mut f.recorder),
        Err(InvocationError::Limit)
    );
    assert_eq!(f.owner.retention_usage(), before);
    a.limits.max_capture_bytes = exact;
    a.limits.max_result_bytes = size_of::<AnnotationLoadResultV1>() - 1;
    assert_eq!(
        f.owner
            .submit_annotation_load_v1(&id, &auth, &request, &a, &mut f.recorder),
        Err(InvocationError::Limit)
    );
    a.limits.max_result_bytes += 1;
    a.result_allowed = Some(false);
    f.owner
        .submit_annotation_load_v1(&id, &auth, &request, &a, &mut f.recorder)
        .unwrap();
    a.result_allowed = Some(true);
    assert_eq!(
        f.owner
            .submit_annotation_load_v1(&id, &auth, &request, &a, &mut f.recorder),
        Err(InvocationError::IntentMismatch)
    );
    f.owner
        .run_annotation_load_v1(&id, &current, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .annotation_load_result_v1(&id, &release(&c), &mut f.recorder),
        Err(InvocationError::Unavailable)
    );
    let AnnotationLoadRequestV1::Restore { package, pair: _ } = request else {
        panic!()
    };
    let changed = AnnotationLoadRequestV1::Restore {
        package,
        pair: None,
    };
    a.result_allowed = Some(false);
    assert_eq!(
        f.owner
            .submit_annotation_load_v1(&id, &auth, &changed, &a, &mut f.recorder),
        Err(InvocationError::IntentMismatch)
    );
}
#[test]
fn real_read_stop_and_late_conflict_refuse_without_partial_restoration_and_do_not_revive() {
    for conflict in [false, true] {
        let (mut f, read, dispatch, auth, request) = ready(1);
        let c = context();
        let current = facts(&c);
        let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        f.owner
            .submit_annotation_load_v1(&id, &auth, &request, &admission(&current), &mut f.recorder)
            .unwrap();
        let generation = f.owner.generation();
        if conflict {
            f.record(
                f.port
                    .annotation_read_chunk(&dispatch, 0, 0, b"changed")
                    .unwrap(),
            )
            .unwrap();
        } else {
            f.owner
                .stop_annotation_read(&read, &mut f.recorder)
                .unwrap();
        }
        f.owner
            .run_annotation_load_v1(&id, &current, &mut f.recorder)
            .unwrap();
        assert!(matches!(
            f.owner.invocation_outcome(&id, &mut f.recorder),
            Ok(InvocationOutcome::AnnotationLoadRejected(_))
        ));
        assert_eq!(f.owner.generation(), generation);
        assert_eq!(
            f.owner
                .annotation_set_version(&f.doc.document, &mut f.recorder)
                .unwrap(),
            0
        );
        f.owner
            .run_annotation_load_v1(&id, &current, &mut f.recorder)
            .unwrap();
        assert_eq!(f.owner.generation(), generation);
        assert!(!f.owner.is_faulted());
    }
}
#[test]
fn protected_stop_is_independent_of_execution_and_never_consumes_the_read() {
    let (mut f, read, _, auth, request) = ready(0);
    let c = context();
    let mut current = facts(&c);
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    f.owner
        .submit_annotation_load_v1(&id, &auth, &request, &admission(&current), &mut f.recorder)
        .unwrap();
    f.owner
        .request_invocation_stop(
            &id,
            &TrustedInvocationControlV1 {
                context: &c,
                owner_control_allowed: Some(true),
                control_context_current: Some(true),
                control_scope_valid: Some(true),
                protected_control_ready: Some(true),
            },
            &mut f.recorder,
        )
        .unwrap();
    current.authorized = None;
    f.owner
        .run_annotation_load_v1(&id, &current, &mut f.recorder)
        .unwrap();
    assert!(matches!(
        f.owner.invocation_outcome(&id, &mut f.recorder),
        Ok(InvocationOutcome::StoppedBeforePublication)
    ));
    assert_eq!(
        f.owner
            .annotation_restoration_outcome(&read, &mut f.recorder)
            .err(),
        Some(ResourceError::MissingEvidence)
    );
}
#[test]
fn terminal_publication_discharges_the_pending_sealed_proof_but_retains_historical_identity() {
    let (mut f, _, _, auth, request) = ready(1);
    let c = context();
    let current = facts(&c);
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    let AnnotationLoadRequestV1::Restore { package, .. } = &request else {
        panic!()
    };
    let original = Arc::downgrade(package);
    assert_eq!(Arc::strong_count(package), 1);
    f.owner
        .submit_annotation_load_v1(&id, &auth, &request, &admission(&current), &mut f.recorder)
        .unwrap();
    assert_eq!(Arc::strong_count(package), 2);
    f.owner
        .run_annotation_load_v1(&id, &current, &mut f.recorder)
        .unwrap();
    assert_eq!(Arc::strong_count(package), 1);
    drop(request);
    assert!(original.upgrade().is_none());
    f.owner
        .run_annotation_load_v1(&id, &current, &mut f.recorder)
        .unwrap();
    assert!(
        f.owner
            .annotation_load_result_v1(&id, &release(&c), &mut f.recorder)
            .unwrap()
            .outcome
            .changed()
    );
}
pub(super) fn baseline_request(
    f: &mut Fixture,
    read: &AnnotationReadId,
    request: AnnotationLoadRequestV1,
) -> AnnotationLoadRequestV1 {
    let AnnotationLoadRequestV1::Restore { package, pair } = request else {
        panic!()
    };
    f.owner
        .restore_annotations(Arc::try_unwrap(package).unwrap(), pair, &mut f.recorder)
        .unwrap();
    f.owner
        .attach_annotation_read_baselines(&mut f.recorder)
        .unwrap();
    AnnotationLoadRequestV1::AdoptBaseline {
        read: read.clone(),
        pair: f.pair(),
        max_baseline_bytes: 65536,
    }
}
#[test]
fn every_applicable_common_and_phase_fact_is_required_at_admission_and_fresh_service() {
    for baseline in [false, true] {
        for field in 0..if baseline { 18 } else { 19 } {
            for value in [None, Some(false)] {
                let (mut f, read, _, auth, request) = ready(0);
                let request = if baseline {
                    baseline_request(&mut f, &read, request)
                } else {
                    request
                };
                let c = context();
                let current = facts(&c);
                let mut denied = facts(&c);
                let slot = match field {
                    0 => &mut denied.authorized,
                    1 => &mut denied.scope,
                    2 => &mut denied.continuous,
                    3 => &mut denied.live,
                    4 => &mut denied.not_stopped,
                    5 => &mut denied.interpretation_current,
                    6 => &mut denied.resources_ready,
                    7 => &mut denied.preparation_reserved,
                    8 => &mut denied.owner_control_allowed,
                    9 => &mut denied.companions_ready,
                    10 => &mut denied.reply_ready,
                    11 => &mut denied.disposition_readable,
                    _ if baseline => match field {
                        12 => &mut denied.annotation_readable,
                        13 => &mut denied.binding_coverage_complete,
                        14 => &mut denied.writer_coverage_complete,
                        15 => &mut denied.baseline_publication_allowed,
                        16 => &mut denied.historical_artifact_readable,
                        _ => &mut denied.baseline_coverage_complete,
                    },
                    _ => match field {
                        12 => &mut denied.annotation_writable,
                        13 => &mut denied.restoration_allowed,
                        14 => &mut denied.identity_restorable,
                        15 => &mut denied.source_evidence_ready,
                        16 => &mut denied.aliases_current,
                        17 => &mut denied.target_evidence_ready,
                        _ => &mut denied.history_restoration_ready,
                    },
                };
                *slot = value;
                let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
                let before = f.owner.retention_usage();
                assert_eq!(
                    f.owner.submit_annotation_load_v1(
                        &id,
                        &auth,
                        &request,
                        &admission(&denied),
                        &mut f.recorder
                    ),
                    Err(InvocationError::Unavailable)
                );
                assert_eq!(f.owner.retention_usage(), before);
                f.owner
                    .submit_annotation_load_v1(
                        &id,
                        &auth,
                        &request,
                        &admission(&current),
                        &mut f.recorder,
                    )
                    .unwrap();
                let generation = f.owner.generation();
                assert_eq!(
                    f.owner
                        .run_annotation_load_v1(&id, &denied, &mut f.recorder),
                    Err(InvocationError::Unavailable)
                );
                assert_eq!(
                    f.owner.invocation_state(&id, &mut f.recorder),
                    Ok(InvocationState::Pending)
                );
                assert_eq!(f.owner.generation(), generation);
                f.owner
                    .run_annotation_load_v1(&id, &current, &mut f.recorder)
                    .unwrap();
                assert!(!f.owner.is_faulted());
            }
        }
    }
}
#[test]
fn actual_writer_blocks_adoption_terminally_and_historical_adoption_preserves_later_source_edits() {
    for writer in [false, true] {
        let (mut f, read, _, auth, request) = ready(1);
        let request = baseline_request(&mut f, &read, request);
        let c = context();
        let current = facts(&c);
        let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        f.owner
            .submit_annotation_load_v1(&id, &auth, &request, &admission(&current), &mut f.recorder)
            .unwrap();
        if writer {
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
            // A source writer is distinct from the annotation destination writer.
            f.owner.stop_save(&save, &mut f.recorder).unwrap();
            let source = f
                .owner
                .capture_annotations(&f.doc, 1, &mut f.recorder)
                .unwrap();
            let encoder = f
                .owner
                .register_annotation_encoder(&mut f.recorder)
                .unwrap();
            let encoded = encoder.seal(source, b"loaded".to_vec()).unwrap();
            let pair = f.pair();
            let save = f
                .owner
                .queue_annotation_save(
                    encoded,
                    f.port.read_result(b"comments", b"v0", b"loaded").unwrap(),
                    SaveRequirements {
                        protection: Protection::BestEffort,
                        acknowledgment: Acknowledgment::Ordinary,
                    },
                    pair,
                    &mut f.recorder,
                )
                .unwrap();
            f.owner
                .run_annotation_load_v1(&id, &current, &mut f.recorder)
                .unwrap();
            assert!(matches!(
                f.owner.invocation_outcome(&id, &mut f.recorder),
                Ok(InvocationOutcome::AnnotationLoadRejected(
                    ResourceError::Blocked
                ))
            ));
            f.owner.stop_save(&save, &mut f.recorder).unwrap();
            f.owner
                .run_annotation_load_v1(&id, &current, &mut f.recorder)
                .unwrap();
            assert!(
                f.owner
                    .annotation_load_result_v1(&id, &release(&c), &mut f.recorder)
                    .is_err()
            );
        } else {
            let edit = f
                .owner
                .prepare(
                    &[DocumentEdits {
                        expected: f.doc.clone(),
                        edits: vec![vellum_engine::source::Replacement {
                            start: 0,
                            end: 0,
                            expected: "".into(),
                            inserted: "later".into(),
                        }],
                    }],
                    &[],
                    &mut f.recorder,
                )
                .unwrap();
            f.owner.commit(edit, &mut f.recorder).unwrap();
            f.owner
                .run_annotation_load_v1(&id, &current, &mut f.recorder)
                .unwrap();
            assert!(
                f.owner
                    .annotation_load_result_v1(&id, &release(&c), &mut f.recorder)
                    .unwrap()
                    .outcome
                    .changed()
            );
            assert_eq!(
                f.owner
                    .read(&f.doc.document, &mut f.recorder)
                    .unwrap()
                    .source
                    .text(),
                "latertext"
            );
            let pair = f.pair();
            let Some(AnnotationBaselineEvidence::Loaded(fact)) = f
                .owner
                .annotation_baseline_evidence(&f.doc.document, pair, &mut f.recorder)
                .unwrap()
            else {
                panic!()
            };
            assert_eq!(fact.source.revision, 0);
        }
        let _ = read;
    }
}
#[test]
fn result_release_and_instrumentation_are_separate_from_authoritative_execution_and_measurements() {
    use vellum_engine::instrumentation::measurement::{PointStateV1, ValueLimitsV1, ValueV1};
    for capacity in [0, 1, 256] {
        for timing in [
            NativeTimingProfileV1::Off,
            NativeTimingProfileV1::ProcessMonotonicV1,
        ] {
            let (mut f, _, _, auth, request) = ready(0);
            let c = context();
            let current = facts(&c);
            let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
            f.owner
                .configure_invocation_measurements_v1(
                    InvocationMeasurementLimitsV1 {
                        values: ValueLimitsV1 {
                            max_integer: 100,
                            max_seconds: 0.0,
                        },
                        max_pool_bytes: InvocationMeasurementLimitsV1::required_pool_bytes(
                            f.owner.invocation_usage().max_lifetime_issued,
                        )
                        .unwrap(),
                    },
                    Some(true),
                    &mut f.recorder,
                )
                .unwrap();
            f.recorder = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(timing);
            f.owner
                .submit_annotation_load_v1(
                    &id,
                    &auth,
                    &request,
                    &admission(&current),
                    &mut f.recorder,
                )
                .unwrap();
            f.owner
                .run_annotation_load_v1(&id, &current, &mut f.recorder)
                .unwrap();
            for field in 0..7 {
                for value in [None, Some(false)] {
                    let mut denied = release(&c);
                    *match field {
                        0 => &mut denied.publication_allowed,
                        1 => &mut denied.outcome_readable,
                        2 => &mut denied.result_allowed,
                        3 => &mut denied.result_context_current,
                        4 => &mut denied.historical_document_readable,
                        5 => &mut denied.reply_ready,
                        _ => &mut denied.resources_ready,
                    } = value;
                    assert_eq!(
                        f.owner
                            .annotation_load_result_v1(&id, &denied, &mut f.recorder),
                        Err(InvocationError::Unavailable)
                    );
                }
            }
            f.owner
                .annotation_load_result_v1(&id, &release(&c), &mut f.recorder)
                .unwrap();
            f.owner
                .run_annotation_load_v1(&id, &current, &mut f.recorder)
                .unwrap();
            f.owner
                .retire_annotation_load_v1(&id, &control(&c), &mut f.recorder)
                .unwrap();
            let m = f
                .owner
                .invocation_measurements_v1(&mut f.recorder)
                .unwrap()
                .unwrap();
            let total = |p: &vellum_engine::instrumentation::measurement::MeasurementPointV1| {
                let PointStateV1::Observed {
                    value: ValueV1::Counter(v),
                    ..
                } = p.state()
                else {
                    panic!()
                };
                v.total
            };
            assert_eq!(total(&m.admissions), 1);
            for (i, p) in m.dispositions.iter().enumerate() {
                assert_eq!(total(p), u64::from(i == 1));
            }
            let events: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
            assert!(events.len() <= capacity);
            assert!(!format!("{events:?}").contains("loaded body"));
            if capacity == 256 {
                for boundary in [
                    Boundary::AnnotationLoadAdmission,
                    Boundary::AnnotationLoadService,
                    Boundary::AnnotationLoadResultRelease,
                    Boundary::AnnotationLoadRetirement,
                ] {
                    assert!(events.iter().any(|e| e.boundary == boundary
                        && e.service_time.is_some()
                            == (timing == NativeTimingProfileV1::ProcessMonotonicV1)));
                }
            }
        }
    }
}
