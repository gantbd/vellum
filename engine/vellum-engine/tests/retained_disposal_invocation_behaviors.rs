use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;

fn observation_policy(c: &SettingsExecutionContextV1) -> InvocationObservationPolicyV1 {
    InvocationObservationPolicyV1 {
        recipient: c.recipient.clone(),
        epoch: 7,
        grant: true,
        publication: true,
        control: true,
        effect: true,
        reasons: true,
        details: false,
        recovery: true,
    }
}
fn observation_facts(c: &SettingsExecutionContextV1) -> TrustedInvocationPolicyFactsV1<'_> {
    TrustedInvocationPolicyFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        resources_ready: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        preparation_reserved: Some(true),
        registry_ready: Some(true),
        retention_ready: Some(true),
    }
}
fn observer_limits() -> InvocationObservationLimitsV1 {
    use std::mem::size_of;
    let mut l = InvocationObservationLimitsV1 {
        max_waiters: 0,
        max_issued: 1,
        max_wait_steps: 0,
        max_context_bytes: 2048,
        max_wait_bytes: InvocationObservationLimitsV1::required_wait_bytes(),
        max_reference_bytes: size_of::<InvocationObservationReferenceV1>()
            .max(size_of::<InvocationControlReferenceV1>()),
        max_reply_bytes: InvocationReceiptV2::required_reply_bytes(512).unwrap(),
        max_control_reply_bytes: size_of::<InvocationControlObservationV1>(),
        max_held_bytes: 0,
    };
    l.max_held_bytes = l.required_hold_bytes().unwrap();
    l
}
fn observed_disposal<'a>(
    o: &'a SourceCoordinator,
    observer: &mut InvocationObservationContextV1,
    c: &SettingsExecutionContextV1,
    r: &mut Recorder,
) -> Result<InvocationReceiptV2<'a>, InvocationError> {
    observed_disposal_with(
        o,
        observer,
        c,
        &release(c),
        &observation_policy(c),
        Some(true),
        r,
    )
}
fn observed_disposal_with<'a>(
    o: &'a SourceCoordinator,
    observer: &mut InvocationObservationContextV1,
    c: &SettingsExecutionContextV1,
    native: &TrustedRetainedDisposalResultV1<'_>,
    policy: &InvocationObservationPolicyV1,
    permission: Option<bool>,
    r: &mut Recorder,
) -> Result<InvocationReceiptV2<'a>, InvocationError> {
    let current = TrustedInvocationObservationReleaseV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        control_ready: Some(true),
    };
    let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &current,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    o.query_invocation_receipt_v2(
        observer,
        InvocationReceiptRequestV2::RetainedDisposal(native),
        policy,
        &TrustedInvocationReceiptReleaseV1 {
            observation: &ordinary,
            operation_receipt_allowed: permission,
        },
        r,
    )
}

#[test]
fn disposal_facet_requires_all_three_publication_ceilings_and_native_release_rights() {
    for bits in 0..8 {
        let (mut o, mut r) = owner();
        let c = context();
        let f = facts(&c);
        let d = o.create(b"a", &mut r).unwrap();
        let p = o.prepare_retained_disposal(&d, 0, &mut r).unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        let mut original = observation_policy(&c);
        original.publication = bits & 1 != 0;
        o.bind_invocation_observation_policy_v1(
            &id,
            &c,
            &original,
            InvocationObservationPolicyLimitsV1 {
                max_context_bytes: 2048,
                max_result_bytes: 512,
            },
            &observation_facts(&c),
            &mut r,
        )
        .unwrap();
        o.submit_retained_disposal_v1(&id, &p, &admission(&f), &mut r)
            .unwrap();
        let mut ceiling = observation_policy(&c);
        ceiling.publication = bits & 2 != 0;
        let mut observer = o
            .open_invocation_observation_with_receipts_v2(
                &id,
                &c,
                &ceiling,
                observer_limits(),
                InvocationReceiptAdmissionV1 {
                    observation: &observation_facts(&c),
                    capabilities: InvocationReceiptCapabilitiesV1 {
                        lifecycle: true,
                        ..Default::default()
                    },
                    operation_receipt_admission_allowed: Some(true),
                },
                &mut r,
            )
            .unwrap();
        o.run_retained_disposal_v1(&id, &f, &mut r).unwrap();
        let mut current = observation_policy(&c);
        current.publication = bits & 4 != 0;
        assert_eq!(
            observed_disposal_with(
                &o,
                &mut observer,
                &c,
                &release(&c),
                &current,
                Some(true),
                &mut r
            )
            .is_ok(),
            bits == 7
        );
        if bits == 7 {
            for unknown in [None, Some(false)] {
                assert!(matches!(
                    observed_disposal_with(
                        &o,
                        &mut observer,
                        &c,
                        &release(&c),
                        &current,
                        unknown,
                        &mut r
                    ),
                    Err(InvocationError::Unavailable)
                ));
                let mut denied = release(&c);
                denied.historical_document_readable = unknown;
                assert!(matches!(
                    observed_disposal_with(
                        &o,
                        &mut observer,
                        &c,
                        &denied,
                        &current,
                        Some(true),
                        &mut r
                    ),
                    Err(InvocationError::Unavailable)
                ));
            }
            let mut epoch = release(&c);
            epoch.publication_epoch += 1;
            assert!(matches!(
                observed_disposal_with(&o, &mut observer, &c, &epoch, &current, Some(true), &mut r),
                Err(InvocationError::Unavailable)
            ));
            assert!(observed_disposal(&o, &mut observer, &c, &mut r).is_ok());
        }
    }
}

#[test]
fn expanded_profile_bounds_and_capabilities_are_admission_bound_without_charging_refusals() {
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let d = o.create(b"a", &mut r).unwrap();
    let p = o.prepare_retained_disposal(&d, 0, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    o.bind_invocation_observation_policy_v1(
        &id,
        &c,
        &observation_policy(&c),
        InvocationObservationPolicyLimitsV1 {
            max_context_bytes: 2048,
            max_result_bytes: 512,
        },
        &observation_facts(&c),
        &mut r,
    )
    .unwrap();
    o.submit_retained_disposal_v1(&id, &p, &admission(&f), &mut r)
        .unwrap();
    let held = o.retention_usage();
    for atomic in [false, true] {
        let mut limits = observer_limits();
        if !atomic {
            limits.max_reply_bytes -= 1;
            limits.max_held_bytes = limits.required_hold_bytes().unwrap();
        }
        let result = o.open_invocation_observation_with_receipts_v2(
            &id,
            &c,
            &observation_policy(&c),
            limits,
            InvocationReceiptAdmissionV1 {
                observation: &observation_facts(&c),
                capabilities: InvocationReceiptCapabilitiesV1 {
                    lifecycle: true,
                    atomic_annotation: atomic,
                    ..Default::default()
                },
                operation_receipt_admission_allowed: Some(true),
            },
            &mut r,
        );
        assert!(
            matches!(result,Err(e) if e == if atomic {InvocationError::Unavailable} else {InvocationError::Limit})
        );
        assert_eq!(o.retention_usage(), held);
    }
    let mut observer = o
        .open_invocation_observation_with_receipts_v2(
            &id,
            &c,
            &observation_policy(&c),
            observer_limits(),
            InvocationReceiptAdmissionV1 {
                observation: &observation_facts(&c),
                capabilities: Default::default(),
                operation_receipt_admission_allowed: Some(true),
            },
            &mut r,
        )
        .unwrap();
    o.run_retained_disposal_v1(&id, &f, &mut r).unwrap();
    assert!(matches!(
        observed_disposal(&o, &mut observer, &c, &mut r),
        Err(InvocationError::Unavailable)
    ));
    assert_eq!(
        InvocationReceiptV2::required_reply_bytes(usize::MAX),
        Err(InvocationError::Limit)
    );
}

#[test]
fn expanded_receipt_admission_and_release_have_optional_bounded_actual_path_timing() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for mode in 0..4 {
        let (mut o, mut setup) = owner();
        let c = context();
        let f = facts(&c);
        let d = o.create(b"private body", &mut setup).unwrap();
        let p = o.prepare_retained_disposal(&d, 0, &mut setup).unwrap();
        let id = o.issue_invocation(&mut setup).unwrap();
        o.bind_invocation_observation_policy_v1(
            &id,
            &c,
            &observation_policy(&c),
            InvocationObservationPolicyLimitsV1 {
                max_context_bytes: 2048,
                max_result_bytes: 512,
            },
            &observation_facts(&c),
            &mut setup,
        )
        .unwrap();
        o.submit_retained_disposal_v1(&id, &p, &admission(&f), &mut setup)
            .unwrap();
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 32 }).unwrap()
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
        let mut observer = o
            .open_invocation_observation_with_receipts_v2(
                &id,
                &c,
                &observation_policy(&c),
                observer_limits(),
                InvocationReceiptAdmissionV1 {
                    observation: &observation_facts(&c),
                    capabilities: InvocationReceiptCapabilitiesV1 {
                        lifecycle: true,
                        ..Default::default()
                    },
                    operation_receipt_admission_allowed: Some(true),
                },
                &mut r,
            )
            .unwrap();
        o.run_retained_disposal_v1(&id, &f, &mut setup).unwrap();
        assert_eq!(
            observed_disposal(&o, &mut observer, &c, &mut r)
                .unwrap()
                .invocation(),
            &id
        );
        if mode == 1 || mode == 2 {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for boundary in [
                Boundary::InvocationReceiptAdmission,
                Boundary::InvocationReceiptRelease,
            ] {
                let selected: Vec<_> = events.iter().filter(|e| e.boundary == boundary).collect();
                assert_eq!(selected.len(), 1);
                assert_eq!(selected[0].disposition, Disposition::Completed);
                assert_eq!(selected[0].service_time.is_some(), mode == 2);
            }
            let text = format!("{events:?}");
            assert!(!text.contains("private body"));
            assert!(!text.contains("session"));
        }
    }
}

#[test]
fn versioned_observation_facets_deliver_actual_disposal_without_replay_or_v1_expansion() {
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let d = o.create(b"a", &mut r).unwrap();
    let p = o.prepare_retained_disposal(&d, 0, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    o.bind_invocation_observation_policy_v1(
        &id,
        &c,
        &observation_policy(&c),
        InvocationObservationPolicyLimitsV1 {
            max_context_bytes: 2048,
            max_result_bytes: 512,
        },
        &observation_facts(&c),
        &mut r,
    )
    .unwrap();
    o.submit_retained_disposal_v1(&id, &p, &admission(&f), &mut r)
        .unwrap();
    let caps = InvocationReceiptCapabilitiesV1 {
        lifecycle: true,
        ..Default::default()
    };
    assert!(matches!(
        o.open_invocation_observation_with_receipts_v1(
            &id,
            &c,
            &observation_policy(&c),
            observer_limits(),
            InvocationReceiptAdmissionV1 {
                observation: &observation_facts(&c),
                capabilities: caps,
                operation_receipt_admission_allowed: Some(true)
            },
            &mut r
        ),
        Err(InvocationError::Unavailable)
    ));
    let mut observer = o
        .open_invocation_observation_with_receipts_v2(
            &id,
            &c,
            &observation_policy(&c),
            observer_limits(),
            InvocationReceiptAdmissionV1 {
                observation: &observation_facts(&c),
                capabilities: caps,
                operation_receipt_admission_allowed: Some(true),
            },
            &mut r,
        )
        .unwrap();
    assert!(matches!(
        observed_disposal(&o, &mut observer, &c, &mut r),
        Err(InvocationError::Unavailable)
    ));
    assert_eq!(
        o.invocation_state(&id, &mut r),
        Ok(InvocationState::Pending)
    );
    o.run_retained_disposal_v1(&id, &f, &mut r).unwrap();
    let generation = o.generation();
    let InvocationReceiptV2::RetainedDisposal(result) =
        observed_disposal(&o, &mut observer, &c, &mut r).unwrap()
    else {
        panic!("wrong result family")
    };
    assert_eq!(result.document, d.document);
    assert_eq!(result.invocation, id);
    assert_eq!(o.generation(), generation);
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    assert!(matches!(
        observed_disposal(&o, &mut observer, &c, &mut r),
        Err(InvocationError::Unavailable)
    ));
}

fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "host".into(),
        engine: "engine".into(),
        recipient: "owner".into(),
        purpose: "edit".into(),
        destination: "local".into(),
        profile: "native".into(),
        continuity: "session".into(),
    }
}
fn facts(c: &SettingsExecutionContextV1) -> TrustedRetainedDisposalFactsV1<'_> {
    TrustedRetainedDisposalFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        disposal_allowed: Some(true),
        resources_ready: Some(true),
        preparation_reserved: Some(true),
    }
}
fn admission<'a>(f: &'a TrustedRetainedDisposalFactsV1<'a>) -> RetainedDisposalAdmissionV1<'a> {
    RetainedDisposalAdmissionV1 {
        facts: f,
        limits: RetainedDisposalLimitsV1 {
            max_capture_bytes: 4096,
            max_result_bytes: 512,
        },
        publication_epoch: 7,
        publication_allowed: Some(true),
        result_admission_allowed: Some(true),
        result_allowed: Some(true),
    }
}
fn release(c: &SettingsExecutionContextV1) -> TrustedRetainedDisposalResultV1<'_> {
    TrustedRetainedDisposalResultV1 {
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
fn owner() -> (SourceCoordinator, Recorder) {
    let mut o = SourceCoordinator::with_invocation_limits(
        CoordinatorLimits {
            max_document_incarnations: 4,
            max_participants: 4,
            max_staged_bytes: 256,
            source: vellum_engine::source::Limits {
                max_source_bytes: 128,
                max_edits: 4,
                max_edit_payload_bytes: 128,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 8,
            max_retained: 8,
            max_intent_bytes: 8192,
        },
    );
    let mut r = Recorder::disabled();
    o.configure_retention_budget(32768, &mut r).unwrap();
    (o, r)
}
#[test]
fn disposal_result_is_original_publication_not_a_later_lifecycle_lookup() {
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let d = o.create(b"original", &mut r).unwrap();
    let p = o.prepare_retained_disposal(&d, 0, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let generation = o.generation();
    assert_eq!(
        o.submit_retained_disposal_v1(&id, &p, &admission(&f), &mut r),
        Ok(InvocationState::Pending)
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(o.run(&id, &mut r), Err(InvocationError::Unavailable));
    assert_eq!(
        o.run_retained_disposal_v1(&id, &f, &mut r),
        Ok(InvocationState::Finished)
    );
    let result = o
        .retained_disposal_result_v1(&id, &release(&c), &mut r)
        .unwrap();
    assert_eq!(result.invocation, id);
    assert_eq!(result.document, d.document);
    let applied = o.generation();
    assert_eq!(
        o.submit_retained_disposal_v1(&id, &p, &admission(&f), &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), applied);
    let disposed = o.lifecycle_state(&d.document, &mut r).unwrap();
    o.release_disposed_provenance(&d.document, disposed, &mut r)
        .unwrap();
    assert_eq!(
        o.retained_disposal_result_v1(&id, &release(&c), &mut r)
            .unwrap(),
        result
    );
    assert_eq!(
        o.retire_invocation(&id, &mut r),
        Err(InvocationError::Unavailable)
    );
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    assert_eq!(
        o.retained_disposal_result_v1(&id, &release(&c), &mut r),
        Err(InvocationError::OutcomeUnavailable)
    );
    o.retire_retained_disposal_v1(
        &id,
        &InvocationObservationPolicyControlV1 {
            context: &c,
            owner_control_allowed: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        },
        &mut r,
    )
    .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    o.retire_invocation(&id, &mut r).unwrap();
    assert_eq!(o.invocation_usage().issued_lifetime, 1);
}

#[test]
fn stale_preparation_is_a_retained_known_refusal_not_a_reprepare() {
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let d = o.create(b"a", &mut r).unwrap();
    let p = o.prepare_retained_disposal(&d, 0, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_retained_disposal_v1(&id, &p, &admission(&f), &mut r)
        .unwrap();
    o.create(b"unrelated publication", &mut r).unwrap();
    let generation = o.generation();
    o.run_retained_disposal_v1(&id, &f, &mut r).unwrap();
    assert_eq!(
        o.invocation_outcome(&id, &mut r),
        Ok(&InvocationOutcome::RetainedDisposalRejected(
            CoordinatorError::Stale
        ))
    );
    assert_eq!(
        o.retained_disposal_result_v1(&id, &release(&c), &mut r),
        Err(InvocationError::OutcomeUnavailable)
    );
    assert_eq!(
        o.submit_retained_disposal_v1(&id, &p, &admission(&f), &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(
        o.lifecycle_state(&d.document, &mut r).unwrap().phase,
        LifecyclePhase::Live
    );
}

#[test]
fn fresh_service_authority_fails_closed_but_protected_stop_survives_content_rights_loss() {
    for field in 0..7 {
        for denied in [None, Some(false)] {
            let (mut o, mut r) = owner();
            let c = context();
            let f = facts(&c);
            let d = o.create(b"a", &mut r).unwrap();
            let p = o.prepare_retained_disposal(&d, 0, &mut r).unwrap();
            let id = o.issue_invocation(&mut r).unwrap();
            o.submit_retained_disposal_v1(&id, &p, &admission(&f), &mut r)
                .unwrap();
            let mut current = facts(&c);
            match field {
                0 => current.authorized = denied,
                1 => current.scope = denied,
                2 => current.continuous = denied,
                3 => current.live = denied,
                4 => current.disposal_allowed = denied,
                5 => current.resources_ready = denied,
                _ => current.preparation_reserved = denied,
            }
            let generation = o.generation();
            assert_eq!(
                o.run_retained_disposal_v1(&id, &current, &mut r),
                Err(InvocationError::Unavailable)
            );
            assert_eq!(
                o.invocation_state(&id, &mut r),
                Ok(InvocationState::Pending)
            );
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
            assert_eq!(
                o.run_retained_disposal_v1(&id, &current, &mut r),
                Ok(InvocationState::Finished)
            );
            assert_eq!(
                o.invocation_outcome(&id, &mut r),
                Ok(&InvocationOutcome::StoppedBeforePublication)
            );
            assert_eq!(o.generation(), generation);
            assert_eq!(
                o.retained_disposal_result_v1(&id, &release(&c), &mut r),
                Err(InvocationError::OutcomeUnavailable)
            );
        }
    }
}

#[test]
fn original_disclosure_ceilings_cannot_be_upgraded_by_retry_or_current_rights() {
    for bits in 0..8 {
        let (mut o, mut r) = owner();
        let c = context();
        let f = facts(&c);
        let d = o.create(b"a", &mut r).unwrap();
        let p = o.prepare_retained_disposal(&d, 0, &mut r).unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        let mut a = admission(&f);
        a.publication_allowed = Some(bits & 1 != 0);
        a.result_admission_allowed = Some(bits & 2 != 0);
        a.result_allowed = Some(bits & 4 != 0);
        o.submit_retained_disposal_v1(&id, &p, &a, &mut r).unwrap();
        o.run_retained_disposal_v1(&id, &f, &mut r).unwrap();
        assert_eq!(
            o.lifecycle_state(&d.document, &mut r).unwrap().phase,
            LifecyclePhase::DisposedRetained
        );
        assert_eq!(
            o.retained_disposal_result_v1(&id, &release(&c), &mut r)
                .is_ok(),
            bits == 7
        );
        if bits != 7 {
            assert_eq!(
                o.submit_retained_disposal_v1(&id, &p, &admission(&f), &mut r),
                Err(InvocationError::IntentMismatch)
            );
        }
    }
}

#[test]
fn every_release_gate_is_current_and_separate_from_execution() {
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let d = o.create(b"a", &mut r).unwrap();
    let p = o.prepare_retained_disposal(&d, 0, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_retained_disposal_v1(&id, &p, &admission(&f), &mut r)
        .unwrap();
    o.run_retained_disposal_v1(&id, &f, &mut r).unwrap();
    let generation = o.generation();
    let retained = o.retention_usage();
    for field in 0..7 {
        for value in [None, Some(false)] {
            let mut current = release(&c);
            match field {
                0 => current.publication_allowed = value,
                1 => current.outcome_readable = value,
                2 => current.result_allowed = value,
                3 => current.result_context_current = value,
                4 => current.historical_document_readable = value,
                5 => current.reply_ready = value,
                _ => current.resources_ready = value,
            }
            assert_eq!(
                o.retained_disposal_result_v1(&id, &current, &mut r),
                Err(InvocationError::Unavailable)
            );
        }
    }
    let mut current = release(&c);
    current.publication_epoch += 1;
    assert_eq!(
        o.retained_disposal_result_v1(&id, &current, &mut r),
        Err(InvocationError::Unavailable)
    );
    let mut foreign = context();
    foreign.continuity.push('x');
    current = release(&foreign);
    assert_eq!(
        o.retained_disposal_result_v1(&id, &current, &mut r),
        Err(InvocationError::Unavailable)
    );
    assert!(
        o.retained_disposal_result_v1(&id, &release(&c), &mut r)
            .is_ok()
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(o.retention_usage(), retained);
}

#[test]
fn admission_bounds_unknown_ceiling_and_foreign_plan_do_not_charge_or_bind() {
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let d = o.create(b"a", &mut r).unwrap();
    let p = o.prepare_retained_disposal(&d, 0, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let mut a = admission(&f);
    let required = RetainedDisposalLimitsV1::required_capture_bytes(&c).unwrap();
    a.limits.max_capture_bytes = required - 1;
    assert_eq!(
        o.submit_retained_disposal_v1(&id, &p, &a, &mut r),
        Err(InvocationError::Limit)
    );
    a = admission(&f);
    a.limits.max_result_bytes = std::mem::size_of::<RetainedDisposalResultV1>() - 1;
    assert_eq!(
        o.submit_retained_disposal_v1(&id, &p, &a, &mut r),
        Err(InvocationError::Limit)
    );
    for field in 0..3 {
        a = admission(&f);
        match field {
            0 => a.publication_allowed = None,
            1 => a.result_admission_allowed = None,
            _ => a.result_allowed = None,
        }
        assert_eq!(
            o.submit_retained_disposal_v1(&id, &p, &a, &mut r),
            Err(InvocationError::Unavailable)
        );
    }
    let (mut other, mut rr) = owner();
    let other_d = other.create(b"a", &mut rr).unwrap();
    let other_p = other
        .prepare_retained_disposal(&other_d, 0, &mut rr)
        .unwrap();
    assert_eq!(
        o.submit_retained_disposal_v1(&id, &other_p, &admission(&f), &mut r),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    a = admission(&f);
    a.limits.max_capture_bytes = required;
    a.limits.max_result_bytes = std::mem::size_of::<RetainedDisposalResultV1>();
    o.submit_retained_disposal_v1(&id, &p, &a, &mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, required);
    let mut changed = context();
    changed.purpose.push('x');
    let changed_f = facts(&changed);
    assert_eq!(
        o.submit_retained_disposal_v1(&id, &p, &admission(&changed_f), &mut r),
        Err(InvocationError::IntentMismatch)
    );
    assert_eq!(
        o.retire_retained_disposal_v1(
            &id,
            &InvocationObservationPolicyControlV1 {
                context: &c,
                owner_control_allowed: Some(true),
                retention_released: Some(true),
                delivery_drained: Some(true)
            },
            &mut r
        ),
        Err(InvocationError::Pending)
    );
}
