use super::*;
use std::mem::size_of;
use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1};
pub(super) fn setup_owned() -> Fixture {
    let mut f = Fixture::with_invocations();
    f.owner
        .configure_retention_budget(1048576, &mut f.recorder)
        .unwrap();
    f.owner
        .configure_source_reader_v1(limits(), &mut f.recorder)
        .unwrap();
    f
}
pub(super) fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "activate".into(),
        destination: "local".into(),
        profile: "source-load/1".into(),
        continuity: "c".into(),
    }
}
pub(super) fn facts(c: &SettingsExecutionContextV1) -> TrustedSourceLoadFactsV1<'_> {
    TrustedSourceLoadFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        interpretation_current: Some(true),
        identity_allowed: Some(true),
        binding_coverage_complete: Some(true),
        activation_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        owner_control_allowed: Some(true),
        preparation_reserved: Some(true),
        writer_coverage_complete: Some(true),
        disposition_readable: Some(true),
        creation_allowed: Some(true),
        owner_access_allowed: Some(true),
    }
}
pub(super) fn admission<'a>(f: &'a TrustedSourceLoadFactsV1<'a>) -> SourceLoadAdmissionV1<'a> {
    SourceLoadAdmissionV1 {
        facts: f,
        limits: SourceLoadLimitsV1 {
            max_capture_bytes: 4096,
            max_result_bytes: 256,
        },
        publication_epoch: 7,
        publication_allowed: Some(true),
        result_admission_allowed: Some(true),
        result_allowed: Some(true),
    }
}
pub(super) fn release(c: &SettingsExecutionContextV1) -> TrustedSourceLoadResultV1<'_> {
    TrustedSourceLoadResultV1 {
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
pub(super) fn authority(f: &mut Fixture, id: &SourceReadIdV1) -> SourceLoadAuthorityV1 {
    f.owner
        .borrow_source_load_authority_v1(
            id,
            &SourceLoadAuthorityFactsV1 {
                owner_control_allowed: Some(true),
                preparation_reserved: Some(true),
                resources_ready: Some(true),
            },
            &mut f.recorder,
        )
        .unwrap()
}
#[test]
fn source_authorities_cannot_substitute_tasks_or_owners_and_creation_permission_is_not_inferred() {
    let mut f = setup_owned();
    let (task, dispatch) = submitted(&mut f, b"new");
    complete(&mut f, &dispatch, b"candidate");
    let auth = authority(&mut f, &task);
    let c = context();
    let current = facts(&c);
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    for denied in [None, Some(false)] {
        let mut no_create = facts(&c);
        no_create.creation_allowed = denied;
        assert_eq!(
            f.owner
                .submit_source_load_v1(&id, &auth, &admission(&no_create), &mut f.recorder),
            Err(InvocationError::Unavailable)
        );
    }
    f.owner
        .submit_source_load_v1(&id, &auth, &admission(&current), &mut f.recorder)
        .unwrap();
    let (other, dispatch) = submitted(&mut f, b"other");
    complete(&mut f, &dispatch, b"candidate");
    let other_auth = authority(&mut f, &other);
    assert_eq!(
        f.owner
            .submit_source_load_v1(&id, &other_auth, &admission(&current), &mut f.recorder),
        Err(InvocationError::IntentMismatch)
    );
    let mut foreign = setup_owned();
    let next = foreign
        .owner
        .issue_invocation(&mut foreign.recorder)
        .unwrap();
    assert_eq!(
        foreign.owner.submit_source_load_v1(
            &next,
            &auth,
            &admission(&current),
            &mut foreign.recorder
        ),
        Err(InvocationError::Unavailable)
    );
    let mut no_create = facts(&c);
    no_create.creation_allowed = None;
    f.owner
        .run_source_load_v1(&id, &no_create, &mut f.recorder)
        .unwrap();
    assert!(matches!(
        f.owner.invocation_outcome(&id, &mut f.recorder),
        Ok(InvocationOutcome::SourceLoadRejected(
            ResourceError::Unavailable
        ))
    ));
    assert!(
        f.owner
            .source_read_status_v1(&task, &mut f.recorder)
            .unwrap()
            .activated
            .is_none()
    );
}
#[test]
fn each_independent_common_fact_is_required_at_admission_and_fresh_service() {
    for field in 0..17 {
        for value in [None, Some(false)] {
            let mut f = setup_owned();
            let (task, dispatch) = submitted(&mut f, b"new");
            complete(&mut f, &dispatch, b"candidate");
            let auth = authority(&mut f, &task);
            let c = context();
            let current = facts(&c);
            let mut denied = facts(&c);
            *match field {
                0 => &mut denied.authorized,
                1 => &mut denied.scope,
                2 => &mut denied.continuous,
                3 => &mut denied.live,
                4 => &mut denied.not_stopped,
                5 => &mut denied.interpretation_current,
                6 => &mut denied.identity_allowed,
                7 => &mut denied.binding_coverage_complete,
                8 => &mut denied.activation_allowed,
                9 => &mut denied.companions_ready,
                10 => &mut denied.versions_available,
                11 => &mut denied.reply_ready,
                12 => &mut denied.resources_ready,
                13 => &mut denied.owner_control_allowed,
                14 => &mut denied.preparation_reserved,
                15 => &mut denied.writer_coverage_complete,
                16 => &mut denied.disposition_readable,
                _ => unreachable!(),
            } = value;
            let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
            let before = f.owner.retention_usage();
            let generation = f.owner.generation();
            assert_eq!(
                f.owner
                    .submit_source_load_v1(&id, &auth, &admission(&denied), &mut f.recorder),
                Err(InvocationError::Unavailable)
            );
            assert_eq!(f.owner.retention_usage(), before);
            f.owner
                .submit_source_load_v1(&id, &auth, &admission(&current), &mut f.recorder)
                .unwrap();
            assert_eq!(
                f.owner.run_source_load_v1(&id, &denied, &mut f.recorder),
                Err(InvocationError::Unavailable)
            );
            assert_eq!(
                f.owner.invocation_state(&id, &mut f.recorder),
                Ok(InvocationState::Pending)
            );
            assert_eq!(f.owner.generation(), generation);
            f.owner
                .run_source_load_v1(&id, &current, &mut f.recorder)
                .unwrap();
            assert!(!f.owner.is_faulted());
        }
    }
}
#[test]
fn actual_paired_writer_blocks_owned_join_and_draining_it_does_not_revive_a_rejection() {
    let mut f = setup_owned();
    let (task, dispatch) = submitted(&mut f, b"source");
    complete(&mut f, &dispatch, b"candidate");
    let auth = authority(&mut f, &task);
    let c = context();
    let current = facts(&c);
    let a = admission(&current);
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    f.owner
        .submit_source_load_v1(&id, &auth, &a, &mut f.recorder)
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
    let generation = f.owner.generation();
    f.owner
        .run_source_load_v1(&id, &current, &mut f.recorder)
        .unwrap();
    assert!(matches!(
        f.owner.invocation_outcome(&id, &mut f.recorder),
        Ok(InvocationOutcome::SourceLoadRejected(
            ResourceError::Blocked
        ))
    ));
    assert_eq!(f.owner.generation(), generation);
    assert!(
        f.owner
            .source_read_status_v1(&task, &mut f.recorder)
            .unwrap()
            .activated
            .is_none()
    );
    f.owner.stop_save(&save, &mut f.recorder).unwrap();
    f.owner
        .run_source_load_v1(&id, &current, &mut f.recorder)
        .unwrap();
    assert!(
        f.owner
            .source_load_result_v1(&id, &release(&c), &mut f.recorder)
            .is_err()
    );
    let next = f.owner.issue_invocation(&mut f.recorder).unwrap();
    f.owner
        .submit_source_load_v1(&next, &auth, &a, &mut f.recorder)
        .unwrap();
    f.owner
        .run_source_load_v1(&next, &current, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .source_load_result_v1(&next, &release(&c), &mut f.recorder)
            .unwrap()
            .disposition,
        SourceReadActivationDispositionV1::Joined
    );
}
#[test]
fn release_checks_are_independent_and_capture_timing_does_not_determine_execution_or_counters() {
    use vellum_engine::instrumentation::measurement::{PointStateV1, ValueLimitsV1, ValueV1};
    for capacity in [0, 1, 256] {
        for timing in [
            NativeTimingProfileV1::Off,
            NativeTimingProfileV1::ProcessMonotonicV1,
        ] {
            let mut f = setup_owned();
            let (task, dispatch) = submitted(&mut f, b"PRIVATE-SOURCE");
            complete(&mut f, &dispatch, b"PRIVATE-CONTENT");
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
            let auth = authority(&mut f, &task);
            f.recorder = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(timing);
            let c = context();
            let current = facts(&c);
            let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
            let funded = f.owner.retention_usage();
            let a = admission(&current);
            f.owner
                .submit_source_load_v1(&id, &auth, &a, &mut f.recorder)
                .unwrap();
            f.owner
                .run_source_load_v1(&id, &current, &mut f.recorder)
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
                        6 => &mut denied.resources_ready,
                        _ => unreachable!(),
                    } = value;
                    assert_eq!(
                        f.owner.source_load_result_v1(&id, &denied, &mut f.recorder),
                        Err(InvocationError::Unavailable)
                    );
                }
            }
            f.owner
                .source_load_result_v1(&id, &release(&c), &mut f.recorder)
                .unwrap();
            f.owner
                .submit_source_load_v1(&id, &auth, &a, &mut f.recorder)
                .unwrap();
            f.owner
                .run_source_load_v1(&id, &current, &mut f.recorder)
                .unwrap();
            let control = InvocationObservationPolicyControlV1 {
                context: &c,
                owner_control_allowed: Some(true),
                retention_released: Some(true),
                delivery_drained: Some(true),
            };
            f.owner
                .retire_source_load_v1(&id, &control, &mut f.recorder)
                .unwrap();
            assert_eq!(f.owner.retention_usage(), funded);
            let measured = f
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
            assert_eq!(total(&measured.admissions), 1);
            for (i, p) in measured.dispositions.iter().enumerate() {
                assert_eq!(total(p), u64::from(i == 0));
            }
            let records: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
            assert!(records.len() <= capacity);
            assert!(!format!("{records:?}").contains("PRIVATE"));
            if capacity == 256 {
                for boundary in [
                    Boundary::SourceLoadAdmission,
                    Boundary::SourceLoadService,
                    Boundary::SourceLoadResultRelease,
                    Boundary::SourceLoadRetirement,
                ] {
                    assert!(records.iter().any(|e| e.boundary == boundary
                        && e.service_time.is_some()
                            == (timing == NativeTimingProfileV1::ProcessMonotonicV1)));
                }
            }
        }
    }
}
#[test]
fn owned_activation_preserves_all_three_dispositions_and_replay_is_historical_not_reexecution() {
    for mode in 0..3 {
        let mut f = setup_owned();
        let (task, dispatch) = submitted(&mut f, if mode == 1 { b"source" } else { b"new" });
        complete(&mut f, &dispatch, b"\xef\xbb\xbfEnglish\r\nDeutsch\n");
        if mode == 2 {
            f.owner
                .activate_source_read_v1(&task, &mut f.recorder)
                .unwrap();
        }
        let auth = authority(&mut f, &task);
        let c = context();
        let current = facts(&c);
        let a = admission(&current);
        let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        let generation = f.owner.generation();
        assert_eq!(
            f.owner
                .submit_source_load_v1(&id, &auth, &a, &mut f.recorder),
            Ok(InvocationState::Pending)
        );
        assert_eq!(
            f.owner.run(&id, &mut f.recorder),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(
            f.owner.run_source_load_v1(&id, &current, &mut f.recorder),
            Ok(InvocationState::Finished)
        );
        let result = f
            .owner
            .source_load_result_v1(&id, &release(&c), &mut f.recorder)
            .unwrap();
        assert_eq!(
            result.disposition,
            [
                SourceReadActivationDispositionV1::Created,
                SourceReadActivationDispositionV1::Joined,
                SourceReadActivationDispositionV1::AlreadyActivated
            ][mode]
        );
        assert_eq!(f.owner.generation(), generation + u64::from(mode == 0));
        let marker = f
            .owner
            .source_read_status_v1(&task, &mut f.recorder)
            .unwrap()
            .activated;
        let source = f
            .owner
            .read(&result.document, &mut f.recorder)
            .unwrap()
            .source
            .text()
            .to_owned();
        assert_eq!(
            source,
            if mode == 1 {
                "text"
            } else {
                "English\r\nDeutsch\n"
            }
        );
        let d = DocumentVersion {
            document: result.document.clone(),
            revision: f
                .owner
                .read(&result.document, &mut f.recorder)
                .unwrap()
                .version
                .revision,
        };
        let edit = SourceIntent {
            writes: vec![DocumentEdits {
                expected: d,
                edits: vec![Replacement {
                    start: 0,
                    end: 0,
                    expected: "".into(),
                    inserted: "later".into(),
                }],
            }],
            reads: vec![],
        };
        let edit_id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        f.owner.submit(&edit_id, &edit, &mut f.recorder).unwrap();
        f.owner.run(&edit_id, &mut f.recorder).unwrap();
        // Real producer retirement revokes the weak handle, not the historical receipt.
        f.owner
            .record_source_read_v1(
                f.port.source_read_quiescence_v1(&dispatch).unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        drop(dispatch);
        f.owner
            .retire_source_read_v1(&task, &mut f.recorder)
            .unwrap();
        let generation = f.owner.generation();
        assert_eq!(
            f.owner
                .submit_source_load_v1(&id, &auth, &a, &mut f.recorder),
            Ok(InvocationState::Finished)
        );
        assert_eq!(
            f.owner.run_source_load_v1(&id, &current, &mut f.recorder),
            Ok(InvocationState::Finished)
        );
        assert_eq!(
            f.owner
                .source_load_result_v1(&id, &release(&c), &mut f.recorder)
                .unwrap(),
            result
        );
        assert_eq!(f.owner.generation(), generation);
        assert!(marker.is_some());
    }
}
#[test]
fn service_resolves_a_new_owner_under_current_owner_access_without_overwriting_it() {
    for allowed in [None, Some(false), Some(true)] {
        let mut f = setup_owned();
        let (task, dispatch) = submitted(&mut f, b"new");
        complete(&mut f, &dispatch, b"candidate");
        let auth = authority(&mut f, &task);
        let c = context();
        let current = facts(&c);
        let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        f.owner
            .submit_source_load_v1(&id, &auth, &admission(&current), &mut f.recorder)
            .unwrap();
        let other = f
            .owner
            .load_resource(
                f.port.read_result(b"new", b"other", b"authored").unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        let generation = f.owner.generation();
        let mut service = facts(&c);
        service.owner_access_allowed = allowed;
        service.creation_allowed = None;
        f.owner
            .run_source_load_v1(&id, &service, &mut f.recorder)
            .unwrap();
        assert_eq!(f.owner.generation(), generation);
        assert_eq!(
            f.owner
                .read(&other.document, &mut f.recorder)
                .unwrap()
                .source
                .text(),
            "authored"
        );
        if allowed == Some(true) {
            let receipt = f
                .owner
                .source_load_result_v1(&id, &release(&c), &mut f.recorder)
                .unwrap();
            assert_eq!(receipt.document, other.document);
            assert_eq!(
                receipt.disposition,
                SourceReadActivationDispositionV1::Joined
            );
        } else {
            assert!(matches!(
                f.owner.invocation_outcome(&id, &mut f.recorder),
                Ok(InvocationOutcome::SourceLoadRejected(
                    ResourceError::Unavailable
                ))
            ));
            assert!(
                f.owner
                    .source_read_status_v1(&task, &mut f.recorder)
                    .unwrap()
                    .activated
                    .is_none()
            );
        }
    }
}
#[test]
fn bounds_original_authority_and_release_ceilings_are_exact_and_cleanup_is_not_producer_cleanup() {
    let mut f = setup_owned();
    let (task, dispatch) = submitted(&mut f, b"new");
    complete(&mut f, &dispatch, b"candidate");
    let auth = authority(&mut f, &task);
    let c = context();
    let current = facts(&c);
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    let before = f.owner.retention_usage();
    let mut a = admission(&current);
    let exact = SourceLoadLimitsV1::required_capture_bytes(&c).unwrap();
    a.limits.max_capture_bytes = exact - 1;
    assert_eq!(
        f.owner
            .submit_source_load_v1(&id, &auth, &a, &mut f.recorder),
        Err(InvocationError::Limit)
    );
    assert_eq!(f.owner.retention_usage(), before);
    a.limits.max_capture_bytes = exact;
    a.limits.max_result_bytes = size_of::<SourceLoadResultV1>() - 1;
    assert_eq!(
        f.owner
            .submit_source_load_v1(&id, &auth, &a, &mut f.recorder),
        Err(InvocationError::Limit)
    );
    a.limits.max_result_bytes += 1;
    a.result_allowed = Some(false);
    f.owner
        .submit_source_load_v1(&id, &auth, &a, &mut f.recorder)
        .unwrap();
    a.result_allowed = Some(true);
    assert_eq!(
        f.owner
            .submit_source_load_v1(&id, &auth, &a, &mut f.recorder),
        Err(InvocationError::IntentMismatch)
    );
    f.owner
        .run_source_load_v1(&id, &current, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .source_load_result_v1(&id, &release(&c), &mut f.recorder),
        Err(InvocationError::Unavailable)
    );
    let control = InvocationObservationPolicyControlV1 {
        context: &c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    assert_eq!(
        f.owner.retire_invocation(&id, &mut f.recorder),
        Err(InvocationError::Unavailable)
    );
    f.owner
        .forget_invocation_outcome(&id, &mut f.recorder)
        .unwrap();
    let usage = f.owner.source_reader_usage_v1().unwrap();
    f.owner
        .retire_source_load_v1(&id, &control, &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.retention_usage(), before);
    assert_eq!(f.owner.source_reader_usage_v1().unwrap(), usage);
    assert_eq!(
        f.owner
            .retire_source_load_v1(&id, &control, &mut f.recorder),
        Err(InvocationError::Unavailable)
    );
    f.owner.retire_invocation(&id, &mut f.recorder).unwrap();
}
#[test]
fn stopped_or_retired_tasks_cannot_be_resurrected_and_native_refusal_is_terminal() {
    for retire in [false, true] {
        let mut f = setup_owned();
        let (task, dispatch) = submitted(&mut f, b"new");
        complete(&mut f, &dispatch, b"candidate");
        let auth = authority(&mut f, &task);
        let c = context();
        let current = facts(&c);
        let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        f.owner
            .submit_source_load_v1(&id, &auth, &admission(&current), &mut f.recorder)
            .unwrap();
        if retire {
            f.owner
                .record_source_read_v1(
                    f.port.source_read_quiescence_v1(&dispatch).unwrap(),
                    &mut f.recorder,
                )
                .unwrap();
            drop(dispatch);
            f.owner
                .retire_source_read_v1(&task, &mut f.recorder)
                .unwrap();
        } else {
            f.owner.stop_source_read_v1(&task, &mut f.recorder).unwrap();
        }
        let generation = f.owner.generation();
        f.owner
            .run_source_load_v1(&id, &current, &mut f.recorder)
            .unwrap();
        assert!(matches!(
            f.owner.invocation_outcome(&id, &mut f.recorder),
            Ok(InvocationOutcome::SourceLoadRejected(
                ResourceError::Unavailable
            ))
        ));
        assert_eq!(
            f.owner.run_source_load_v1(&id, &current, &mut f.recorder),
            Ok(InvocationState::Finished)
        );
        assert_eq!(f.owner.generation(), generation);
        assert!(!f.owner.is_faulted());
    }
}
#[test]
fn protected_queued_stop_wins_without_needing_current_execution_permission() {
    let mut f = setup_owned();
    let (task, dispatch) = submitted(&mut f, b"new");
    complete(&mut f, &dispatch, b"candidate");
    let auth = authority(&mut f, &task);
    let c = context();
    let mut current = facts(&c);
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    f.owner
        .submit_source_load_v1(&id, &auth, &admission(&current), &mut f.recorder)
        .unwrap();
    let control = TrustedInvocationControlV1 {
        context: &c,
        owner_control_allowed: Some(true),
        control_context_current: Some(true),
        control_scope_valid: Some(true),
        protected_control_ready: Some(true),
    };
    f.owner
        .request_invocation_stop(&id, &control, &mut f.recorder)
        .unwrap();
    current.authorized = None;
    let generation = f.owner.generation();
    f.owner
        .run_source_load_v1(&id, &current, &mut f.recorder)
        .unwrap();
    assert!(matches!(
        f.owner.invocation_outcome(&id, &mut f.recorder),
        Ok(InvocationOutcome::StoppedBeforePublication)
    ));
    assert!(
        f.owner
            .source_read_status_v1(&task, &mut f.recorder)
            .unwrap()
            .activated
            .is_none()
    );
    assert_eq!(f.owner.generation(), generation);
}
