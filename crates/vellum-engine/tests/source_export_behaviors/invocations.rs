use super::*;
use vellum_engine::instrumentation::{NativeTimingProfileV1, Recorder};
#[test]
fn original_rebind_result_ceiling_and_protected_cleanup_cannot_be_upgraded_or_refunded_twice() {
    let mut f = Fixture::with_invocations();
    f.owner
        .configure_retention_budget(262144, &mut f.recorder)
        .unwrap();
    let c = context();
    let native = rebind_facts();
    let facts = TrustedSourceRebindInvocationFactsV1 {
        context: &c,
        native: &native,
    };
    let q = request(&mut f, b"new-source");
    let job = f.owner.issue_invocation(&mut f.recorder).unwrap();
    let before = f.owner.retention_usage();
    let mut a = admission(&facts);
    a.result_allowed = Some(false);
    f.owner
        .submit_source_rebind_v1(&job, &q, &a, &mut f.recorder)
        .unwrap();
    let control = InvocationObservationPolicyControlV1 {
        context: &c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    let funded = f.owner.retention_usage();
    assert_eq!(
        f.owner
            .retire_source_rebind_v1(&job, &control, &mut f.recorder),
        Err(InvocationError::Pending)
    );
    assert_eq!(f.owner.retention_usage(), funded);
    f.owner
        .run_source_rebind_v1(&job, &facts, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .source_rebind_result_v1(&job, &release(&c), &mut f.recorder)
            .err(),
        Some(InvocationError::Unavailable)
    );
    a.result_allowed = Some(true);
    assert_eq!(
        f.owner
            .submit_source_rebind_v1(&job, &q, &a, &mut f.recorder),
        Err(InvocationError::IntentMismatch)
    );
    let denied = InvocationObservationPolicyControlV1 {
        delivery_drained: None,
        ..control
    };
    assert_eq!(
        f.owner
            .retire_source_rebind_v1(&job, &denied, &mut f.recorder),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(f.owner.retention_usage(), funded);
    f.owner
        .forget_invocation_outcome(&job, &mut f.recorder)
        .unwrap();
    f.owner
        .retire_source_rebind_v1(&job, &control, &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.retention_usage(), before);
    assert_eq!(
        f.owner
            .retire_source_rebind_v1(&job, &control, &mut f.recorder),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(f.owner.retention_usage(), before);
    f.owner.retire_invocation(&job, &mut f.recorder).unwrap();
}
#[test]
fn rebind_admission_bounds_current_service_rights_and_terminal_writer_refusal_are_separate() {
    for blocked in [false, true] {
        let mut f = Fixture::with_invocations();
        f.owner
            .configure_retention_budget(262144, &mut f.recorder)
            .unwrap();
        let c = context();
        let native = rebind_facts();
        let current = TrustedSourceRebindInvocationFactsV1 {
            context: &c,
            native: &native,
        };
        let q = request(&mut f, b"new-source");
        let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        let before = f.owner.retention_usage();
        let mut a = admission(&current);
        let exact = SourceRebindLimitsV1::required_capture_bytes(&c, &q).unwrap();
        a.limits.max_capture_bytes = exact - 1;
        assert_eq!(
            f.owner
                .submit_source_rebind_v1(&id, &q, &a, &mut f.recorder),
            Err(InvocationError::Limit)
        );
        assert_eq!(
            f.owner.invocation_state(&id, &mut f.recorder),
            Ok(InvocationState::Issued)
        );
        assert_eq!(f.owner.retention_usage(), before);
        a.limits.max_capture_bytes = exact;
        f.owner
            .submit_source_rebind_v1(&id, &q, &a, &mut f.recorder)
            .unwrap();
        let mut denied = native;
        denied.rebind_allowed = None;
        let denied = TrustedSourceRebindInvocationFactsV1 {
            context: &c,
            native: &denied,
        };
        assert_eq!(
            f.owner.run_source_rebind_v1(&id, &denied, &mut f.recorder),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(
            f.owner.invocation_state(&id, &mut f.recorder),
            Ok(InvocationState::Pending)
        );
        let generation = f.owner.generation();
        if blocked {
            let pair = f.pair().unwrap();
            let task = f
                .owner
                .queue_pinned_save_with_pair(&f.doc, requirements(), pair, &mut f.recorder)
                .unwrap();
            f.owner
                .run_source_rebind_v1(&id, &current, &mut f.recorder)
                .unwrap();
            assert!(matches!(
                f.owner.invocation_outcome(&id, &mut f.recorder),
                Ok(InvocationOutcome::SourceRebindRejected(
                    ResourceError::Retained
                ))
            ));
            f.owner.stop_save(&task, &mut f.recorder).unwrap();
            assert_eq!(
                f.owner
                    .submit_source_rebind_v1(&id, &q, &a, &mut f.recorder),
                Ok(InvocationState::Finished)
            );
            f.owner
                .run_source_rebind_v1(&id, &current, &mut f.recorder)
                .unwrap();
            assert_eq!(f.owner.generation(), generation);
        } else {
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
            f.owner
                .run_source_rebind_v1(&id, &current, &mut f.recorder)
                .unwrap();
            assert!(matches!(
                f.owner.invocation_outcome(&id, &mut f.recorder),
                Ok(InvocationOutcome::StoppedBeforePublication)
            ));
            assert_eq!(f.owner.generation(), generation);
        }
        assert!(
            f.owner
                .source_rebind_result_v1(&id, &release(&c), &mut f.recorder)
                .is_err()
        );
    }
}
pub(super) fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "host".into(),
        engine: "engine".into(),
        recipient: "owner".into(),
        purpose: "rebind".into(),
        destination: "local".into(),
        profile: "source-rebind/1".into(),
        continuity: "session".into(),
    }
}
pub(super) fn request(f: &mut Fixture, key: &[u8]) -> SourceRebindRequestV1 {
    let target = f.port.source_binding_target(key).unwrap();
    let pair = f
        .owner
        .source_rebind_pair_check(&f.doc.document, &target, &mut f.recorder)
        .unwrap()
        .map(|check| {
            f.port
                .attest_ordinary_pair(&check, ResourceRelation::Distinct)
                .unwrap()
        });
    SourceRebindRequestV1 {
        document: f.doc.document.clone(),
        target,
        options: SourceRebindOptionsV1 {
            expected_binding: Some(0),
            max_binding_history: 4,
            pair,
        },
    }
}
pub(super) fn admission<'a>(
    facts: &'a TrustedSourceRebindInvocationFactsV1<'a>,
) -> SourceRebindAdmissionV1<'a> {
    SourceRebindAdmissionV1 {
        facts,
        limits: SourceRebindLimitsV1 {
            max_capture_bytes: 8192,
            max_result_bytes: 512,
        },
        publication_epoch: 7,
        publication_allowed: Some(true),
        result_admission_allowed: Some(true),
        result_allowed: Some(true),
    }
}
pub(super) fn release(c: &SettingsExecutionContextV1) -> TrustedSourceRebindResultV1<'_> {
    TrustedSourceRebindResultV1 {
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
#[test]
fn owned_rebind_preserves_later_edits_original_intent_and_independent_retained_results() {
    use vellum_engine::instrumentation::measurement::{PointStateV1, ValueLimitsV1, ValueV1};
    for capacity in [0, 1, 512] {
        for unchanged in [false, true] {
            let mut f = Fixture::with_invocations();
            f.owner
                .configure_retention_budget(262144, &mut f.recorder)
                .unwrap();
            if capacity == 512 {
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
            }
            f.recorder = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            let c = context();
            let native = rebind_facts();
            let facts = TrustedSourceRebindInvocationFactsV1 {
                context: &c,
                native: &native,
            };
            let q = request(&mut f, if unchanged { b"source" } else { b"new-source" });
            let job = f.owner.issue_invocation(&mut f.recorder).unwrap();
            let original_credit = f.owner.retention_usage();
            f.owner
                .submit_source_rebind_v1(&job, &q, &admission(&facts), &mut f.recorder)
                .unwrap();
            assert_eq!(
                f.owner.run(&job, &mut f.recorder),
                Err(InvocationError::Unavailable)
            );
            let changed = request(&mut f, b"other-source");
            assert_eq!(
                f.owner.submit_source_rebind_v1(
                    &job,
                    &changed,
                    &admission(&facts),
                    &mut f.recorder
                ),
                Err(InvocationError::IntentMismatch)
            );
            let edit = f
                .owner
                .prepare(
                    &[DocumentEdits {
                        expected: f.doc.clone(),
                        edits: vec![vellum_engine::source::Replacement {
                            start: 0,
                            end: 4,
                            expected: "text".into(),
                            inserted: "later prose".into(),
                        }],
                    }],
                    &[],
                    &mut f.recorder,
                )
                .unwrap();
            f.owner.commit(edit, &mut f.recorder).unwrap();
            let before_service = f.owner.generation();
            f.owner
                .run_source_rebind_v1(&job, &facts, &mut f.recorder)
                .unwrap();
            assert_eq!(f.owner.generation(), before_service + u64::from(!unchanged));
            assert_eq!(
                f.owner
                    .read(&f.doc.document, &mut f.recorder)
                    .unwrap()
                    .source
                    .text(),
                "later prose"
            );
            let generation = f.owner.generation();
            let mut denied = release(&c);
            denied.result_allowed = None;
            assert_eq!(
                f.owner
                    .source_rebind_result_v1(&job, &denied, &mut f.recorder)
                    .err(),
                Some(InvocationError::Unavailable)
            );
            let result = f
                .owner
                .source_rebind_result_v1(&job, &release(&c), &mut f.recorder)
                .unwrap();
            assert_eq!(result.invocation, job);
            assert_eq!(result.document, f.doc.document);
            assert_eq!(result.changed, !unchanged);
            assert_eq!(
                f.owner
                    .submit_source_rebind_v1(&job, &q, &admission(&facts), &mut f.recorder),
                Ok(InvocationState::Finished)
            );
            f.owner
                .run_source_rebind_v1(&job, &facts, &mut f.recorder)
                .unwrap();
            assert_eq!(f.owner.generation(), generation);
            f.owner
                .forget_invocation_outcome(&job, &mut f.recorder)
                .unwrap();
            assert!(
                f.owner
                    .source_rebind_result_v1(&job, &release(&c), &mut f.recorder)
                    .is_err()
            );
            let control = InvocationObservationPolicyControlV1 {
                context: &c,
                owner_control_allowed: Some(true),
                retention_released: Some(true),
                delivery_drained: Some(true),
            };
            f.owner
                .retire_source_rebind_v1(&job, &control, &mut f.recorder)
                .unwrap();
            f.owner.retire_invocation(&job, &mut f.recorder).unwrap();
            assert_eq!(f.owner.retention_usage(), original_credit);
            if capacity == 512 {
                let measured = f
                    .owner
                    .invocation_measurements_v1(&mut f.recorder)
                    .unwrap()
                    .unwrap();
                let total =
                    |p: &vellum_engine::instrumentation::measurement::MeasurementPointV1| {
                        let PointStateV1::Observed {
                            value: ValueV1::Counter(counter),
                            ..
                        } = p.state()
                        else {
                            panic!()
                        };
                        counter.total
                    };
                assert_eq!(total(&measured.admissions), 1);
                for (i, point) in measured.dispositions.iter().enumerate() {
                    assert_eq!(total(point), u64::from(i == usize::from(unchanged)));
                }
            }
            let records: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
            assert!(!format!("{records:?}").contains("later prose"));
            if capacity == 512 {
                use vellum_engine::instrumentation::Boundary;
                for boundary in [
                    Boundary::SourceRebindAdmission,
                    Boundary::SourceRebindService,
                    Boundary::SourceRebindResultRelease,
                    Boundary::SourceRebindRetirement,
                ] {
                    assert!(
                        records
                            .iter()
                            .any(|e| e.boundary == boundary && e.service_time.is_some()),
                        "{boundary:?}"
                    );
                }
            }
        }
    }
}
