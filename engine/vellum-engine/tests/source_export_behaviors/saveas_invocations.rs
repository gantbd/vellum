use super::*;
use std::mem::size_of;
use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1, Recorder};
#[test]
fn document_resource_pins_distinguish_export_reservation_release_from_producer_retirement() {
    let (mut f, d, save, _, _, request) = ready();
    let before = f
        .owner
        .document_resource_pins_v1(&d.document, &mut f.recorder)
        .unwrap();
    assert_eq!(
        (
            before.write_tasks,
            before.artifacts,
            before.export_reservations
        ),
        (1, 1, 1)
    );
    f.owner
        .activate_source_save_as(
            &save,
            request.probe,
            request.pair,
            request.max_binding_history,
            &activation_facts(),
            &mut f.recorder,
        )
        .unwrap();
    let after = f
        .owner
        .document_resource_pins_v1(&d.document, &mut f.recorder)
        .unwrap();
    assert_eq!(
        (
            after.write_tasks,
            after.artifacts,
            after.export_reservations,
            after.source_bindings
        ),
        (1, 1, 0, 1)
    );
}
pub(super) fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "activate".into(),
        destination: "local".into(),
        profile: "source-saveas/1".into(),
        continuity: "c".into(),
    }
}
pub(super) fn authority_facts() -> SourceSaveAsAuthorityFactsV1 {
    SourceSaveAsAuthorityFactsV1 {
        owner_control_allowed: Some(true),
        preparation_reserved: Some(true),
        resources_ready: Some(true),
    }
}
pub(super) fn admission<'a>(
    f: &'a TrustedSourceSaveAsInvocationFactsV1<'a>,
) -> SourceSaveAsAdmissionV1<'a> {
    SourceSaveAsAdmissionV1 {
        facts: f,
        limits: SourceSaveAsLimitsV1 {
            max_capture_bytes: 8192,
            max_result_bytes: 256,
        },
        publication_epoch: 7,
        publication_allowed: Some(true),
        result_admission_allowed: Some(true),
        result_allowed: Some(true),
    }
}
pub(super) fn release(c: &SettingsExecutionContextV1) -> TrustedSourceSaveAsResultV1<'_> {
    TrustedSourceSaveAsResultV1 {
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
pub(super) fn release_control(
    c: &SettingsExecutionContextV1,
) -> InvocationObservationPolicyControlV1<'_> {
    InvocationObservationPolicyControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(super) fn ready() -> (
    Fixture,
    DocumentVersion,
    SaveId,
    SaveDispatch,
    SourceSaveAsAuthorityV1,
    SourceSaveAsRequestV1,
) {
    ready_with_retention(1048576)
}
pub(super) fn ready_with_retention(
    retention: usize,
) -> (
    Fixture,
    DocumentVersion,
    SaveId,
    SaveDispatch,
    SourceSaveAsAuthorityV1,
    SourceSaveAsRequestV1,
) {
    let mut f = Fixture::with_invocations();
    f.owner
        .configure_retention_budget(retention, &mut f.recorder)
        .unwrap();
    let d = f.owner.create(b"draft", &mut f.recorder).unwrap();
    let prepared = prepare(&mut f, &d);
    let id = f
        .owner
        .queue_source_export(
            &prepared,
            present(&f.port, b"PRIVATE-OUT"),
            options(SourceExportPurposeV1::SaveAs),
            &facts(),
            &mut f.recorder,
        )
        .unwrap();
    let auth = f
        .owner
        .borrow_source_save_as_authority_v1(&id, &authority_facts(), &mut f.recorder)
        .unwrap();
    let dispatch = f
        .owner
        .dispatch_source_export(
            &id,
            present(&f.port, b"PRIVATE-OUT"),
            None,
            &facts(),
            &mut f.recorder,
        )
        .unwrap();
    for e in [
        f.port
            .result(
                &dispatch,
                Application::Written,
                dispatch.bytes().len(),
                Synchronization::NotRequested,
                Some(b"v1"),
            )
            .unwrap(),
        f.port.no_future_effects(&dispatch).unwrap(),
        f.port.quiescence(&dispatch).unwrap(),
    ] {
        f.owner.record_save_evidence(e, &mut f.recorder).unwrap();
    }
    f.owner
        .source_save_as_authority_current_v1(&auth, &authority_facts(), &mut f.recorder)
        .unwrap();
    let request = SourceSaveAsRequestV1 {
        probe: f
            .port
            .source_save_as_probe(b"PRIVATE-OUT", Some(b"v1"))
            .unwrap(),
        pair: None,
        max_binding_history: 0,
    };
    (f, d, id, dispatch, auth, request)
}
#[test]
fn owned_saveas_preserves_later_edits_original_applied_or_unchanged_result_and_single_write() {
    for repeated in [false, true] {
        let (mut f, d, save, _, auth, request) = ready();
        let c = context();
        let native = activation_facts();
        let grant = authority_facts();
        let current = TrustedSourceSaveAsInvocationFactsV1 {
            context: &c,
            native: &native,
            authority: &grant,
            interpretation_current: Some(true),
        };
        if repeated {
            f.owner
                .activate_source_save_as(
                    &save,
                    f.port
                        .source_save_as_probe(b"PRIVATE-OUT", Some(b"v1"))
                        .unwrap(),
                    None,
                    0,
                    &native,
                    &mut f.recorder,
                )
                .unwrap();
        }
        let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        let before = f.owner.retention_usage();
        let a = admission(&current);
        f.owner
            .submit_source_save_as_v1(&id, &auth, &request, &a, &mut f.recorder)
            .unwrap();
        assert_eq!(
            f.owner.run(&id, &mut f.recorder),
            Err(InvocationError::Unavailable)
        );
        let edit = f
            .owner
            .prepare(
                &[DocumentEdits {
                    expected: d.clone(),
                    edits: vec![vellum_engine::source::Replacement {
                        start: 0,
                        end: 0,
                        expected: "".into(),
                        inserted: "later ".into(),
                    }],
                }],
                &[],
                &mut f.recorder,
            )
            .unwrap();
        f.owner.commit(edit, &mut f.recorder).unwrap();
        let generation = f.owner.generation();
        let writer = f.owner.resource_usage();
        f.owner
            .run_source_save_as_v1(&id, &current, &mut f.recorder)
            .unwrap();
        assert_eq!(f.owner.generation(), generation + u64::from(!repeated));
        let original = f
            .owner
            .source_save_as_result_v1(&id, &release(&c), &mut f.recorder)
            .unwrap();
        assert_eq!(original.document, d.document);
        assert_eq!(original.changed, !repeated);
        assert_eq!(
            f.owner
                .read(&d.document, &mut f.recorder)
                .unwrap()
                .source
                .text(),
            "later draft"
        );
        assert_eq!(
            f.owner
                .saved_state(&d.document, &mut f.recorder)
                .unwrap()
                .content,
            ContentComparison::Modified
        );
        assert_eq!(
            f.owner.resource_usage().issued_lifetime,
            writer.issued_lifetime
        );
        assert_eq!(
            f.owner
                .submit_source_save_as_v1(&id, &auth, &request, &a, &mut f.recorder),
            Ok(InvocationState::Finished)
        );
        f.owner
            .run_source_save_as_v1(&id, &current, &mut f.recorder)
            .unwrap();
        assert_eq!(
            f.owner
                .source_save_as_result_v1(&id, &release(&c), &mut f.recorder)
                .unwrap(),
            original
        );
        let producer = f.owner.resource_usage();
        f.owner
            .forget_invocation_outcome(&id, &mut f.recorder)
            .unwrap();
        f.owner
            .retire_source_save_as_v1(&id, &release_control(&c), &mut f.recorder)
            .unwrap();
        assert_eq!(f.owner.retention_usage(), before);
        assert_eq!(f.owner.resource_usage(), producer);
        assert_eq!(
            f.owner
                .retire_source_save_as_v1(&id, &release_control(&c), &mut f.recorder),
            Err(InvocationError::Unavailable)
        );
        f.owner.retire_invocation(&id, &mut f.recorder).unwrap();
    }
}
#[test]
fn exact_bounds_original_probe_and_captured_result_ceiling_cannot_be_upgraded() {
    let (mut f, _, _, _, auth, mut request) = ready();
    let c = context();
    let native = activation_facts();
    let grant = authority_facts();
    let current = TrustedSourceSaveAsInvocationFactsV1 {
        context: &c,
        native: &native,
        authority: &grant,
        interpretation_current: Some(true),
    };
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    let before = f.owner.retention_usage();
    let mut a = admission(&current);
    let exact = SourceSaveAsLimitsV1::required_capture_bytes(&c, &request).unwrap();
    a.limits.max_capture_bytes = exact - 1;
    assert_eq!(
        f.owner
            .submit_source_save_as_v1(&id, &auth, &request, &a, &mut f.recorder),
        Err(InvocationError::Limit)
    );
    assert_eq!(f.owner.retention_usage(), before);
    a.limits.max_capture_bytes = exact;
    a.limits.max_result_bytes = size_of::<SourceSaveAsResultV1>() - 1;
    assert_eq!(
        f.owner
            .submit_source_save_as_v1(&id, &auth, &request, &a, &mut f.recorder),
        Err(InvocationError::Limit)
    );
    a.limits.max_result_bytes += 1;
    a.result_allowed = Some(false);
    f.owner
        .submit_source_save_as_v1(&id, &auth, &request, &a, &mut f.recorder)
        .unwrap();
    request.probe = f
        .port
        .source_save_as_probe(b"PRIVATE-OUT", Some(b"v2"))
        .unwrap();
    assert_eq!(
        f.owner
            .submit_source_save_as_v1(&id, &auth, &request, &a, &mut f.recorder),
        Err(InvocationError::IntentMismatch)
    );
    f.owner
        .run_source_save_as_v1(&id, &current, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .source_save_as_result_v1(&id, &release(&c), &mut f.recorder),
        Err(InvocationError::Unavailable)
    );
}
#[test]
fn protected_invocation_stop_and_actual_producer_stop_remain_distinct_and_never_rewrite() {
    for producer in [false, true] {
        let (mut f, _, save, _, auth, request) = ready();
        let c = context();
        let native = activation_facts();
        let grant = authority_facts();
        let current = TrustedSourceSaveAsInvocationFactsV1 {
            context: &c,
            native: &native,
            authority: &grant,
            interpretation_current: Some(true),
        };
        let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        f.owner
            .submit_source_save_as_v1(&id, &auth, &request, &admission(&current), &mut f.recorder)
            .unwrap();
        let generation = f.owner.generation();
        if producer {
            f.owner.stop_save(&save, &mut f.recorder).unwrap();
        } else {
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
        }
        f.owner
            .run_source_save_as_v1(&id, &current, &mut f.recorder)
            .unwrap();
        if producer {
            assert!(matches!(
                f.owner.invocation_outcome(&id, &mut f.recorder),
                Ok(InvocationOutcome::SourceSaveAsRejected(
                    ResourceError::AcknowledgmentUnsatisfied
                ))
            ));
        } else {
            assert!(matches!(
                f.owner.invocation_outcome(&id, &mut f.recorder),
                Ok(InvocationOutcome::StoppedBeforePublication)
            ));
        }
        assert_eq!(f.owner.generation(), generation);
        assert!(!f.owner.is_faulted());
        assert_eq!(
            f.owner
                .save_outcome(&save, &mut f.recorder)
                .unwrap()
                .application,
            Application::Written
        );
    }
}
#[test]
fn every_independent_admission_and_service_fact_is_current_without_publishing_on_denial() {
    for field in 0..20 {
        for value in [None, Some(false)] {
            let (mut f, _, _, _, auth, request) = ready();
            let c = context();
            let native = activation_facts();
            let grant = authority_facts();
            let current = TrustedSourceSaveAsInvocationFactsV1 {
                context: &c,
                native: &native,
                authority: &grant,
                interpretation_current: Some(true),
            };
            let mut denied_native = activation_facts();
            let mut denied_grant = authority_facts();
            let mut interpretation = Some(true);
            *match field {
                0 => &mut denied_grant.owner_control_allowed,
                1 => &mut denied_grant.preparation_reserved,
                2 => &mut denied_grant.resources_ready,
                3 => &mut interpretation,
                4 => &mut denied_native.control.authorized,
                5 => &mut denied_native.control.scope,
                6 => &mut denied_native.control.continuous,
                7 => &mut denied_native.control.live,
                8 => &mut denied_native.control.not_stopped,
                9 => &mut denied_native.control.binding_coverage_complete,
                10 => &mut denied_native.control.writer_coverage_complete,
                11 => &mut denied_native.control.binding_activation_allowed,
                12 => &mut denied_native.control.identity_allowed,
                13 => &mut denied_native.control.companions_ready,
                14 => &mut denied_native.control.versions_available,
                15 => &mut denied_native.control.reply_ready,
                16 => &mut denied_native.control.resources_ready,
                17 => &mut denied_native.control.disposition_readable,
                18 => &mut denied_native.probe_attested,
                _ => &mut denied_native.resource_continuity_attested,
            } = value;
            let denied = TrustedSourceSaveAsInvocationFactsV1 {
                context: &c,
                native: &denied_native,
                authority: &denied_grant,
                interpretation_current: interpretation,
            };
            let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
            let before = f.owner.retention_usage();
            let generation = f.owner.generation();
            assert_eq!(
                f.owner.submit_source_save_as_v1(
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
                .submit_source_save_as_v1(
                    &id,
                    &auth,
                    &request,
                    &admission(&current),
                    &mut f.recorder,
                )
                .unwrap();
            assert_eq!(
                f.owner.run_source_save_as_v1(&id, &denied, &mut f.recorder),
                Err(InvocationError::Unavailable)
            );
            assert_eq!(
                f.owner.invocation_state(&id, &mut f.recorder),
                Ok(InvocationState::Pending)
            );
            assert_eq!(f.owner.generation(), generation);
            f.owner
                .run_source_save_as_v1(&id, &current, &mut f.recorder)
                .unwrap();
            assert!(!f.owner.is_faulted());
        }
    }
}
#[test]
fn result_release_and_bounded_timing_do_not_determine_execution_or_measurement_cohorts() {
    use vellum_engine::instrumentation::measurement::{PointStateV1, ValueLimitsV1, ValueV1};
    for capacity in [0, 1, 256] {
        for timing in [
            NativeTimingProfileV1::Off,
            NativeTimingProfileV1::ProcessMonotonicV1,
        ] {
            let (mut f, _, _, _, auth, request) = ready();
            let c = context();
            let native = activation_facts();
            let grant = authority_facts();
            let current = TrustedSourceSaveAsInvocationFactsV1 {
                context: &c,
                native: &native,
                authority: &grant,
                interpretation_current: Some(true),
            };
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
            let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
            f.owner
                .submit_source_save_as_v1(
                    &id,
                    &auth,
                    &request,
                    &admission(&current),
                    &mut f.recorder,
                )
                .unwrap();
            f.owner
                .run_source_save_as_v1(&id, &current, &mut f.recorder)
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
                            .source_save_as_result_v1(&id, &denied, &mut f.recorder),
                        Err(InvocationError::Unavailable)
                    );
                }
            }
            f.owner
                .source_save_as_result_v1(&id, &release(&c), &mut f.recorder)
                .unwrap();
            f.owner
                .run_source_save_as_v1(&id, &current, &mut f.recorder)
                .unwrap();
            f.owner
                .retire_source_save_as_v1(&id, &release_control(&c), &mut f.recorder)
                .unwrap();
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
            let events: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
            assert!(events.len() <= capacity);
            assert!(!format!("{events:?}").contains("PRIVATE"));
            if capacity == 256 {
                for boundary in [
                    Boundary::SourceSaveAsAdmission,
                    Boundary::SourceSaveAsService,
                    Boundary::SourceSaveAsResultRelease,
                    Boundary::SourceSaveAsRetirement,
                ] {
                    assert!(events.iter().any(|e| e.boundary == boundary
                        && e.service_time.is_some()
                            == (timing == NativeTimingProfileV1::ProcessMonotonicV1)));
                }
            }
        }
    }
}
