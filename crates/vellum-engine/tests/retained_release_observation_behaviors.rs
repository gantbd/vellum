#[path = "support/discovered_context_fixture.rs"]
mod discovery;
#[path = "support/retained_release_fixture.rs"]
mod producer;
use std::mem::size_of;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
#[path = "retained_release_observation_behaviors/discovered.rs"]
mod discovered;
#[path = "retained_release_observation_behaviors/ingress.rs"]
mod ingress;

fn policy(c: &SettingsExecutionContextV1) -> InvocationObservationPolicyV1 {
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
fn facts(c: &SettingsExecutionContextV1) -> TrustedInvocationPolicyFactsV1<'_> {
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
fn limits() -> InvocationObservationLimitsV1 {
    let mut l = InvocationObservationLimitsV1 {
        max_waiters: 0,
        max_issued: 0,
        max_wait_steps: 0,
        max_context_bytes: 2048,
        max_wait_bytes: InvocationObservationLimitsV1::required_wait_bytes(),
        max_reference_bytes: size_of::<InvocationControlReferenceV1>()
            .max(size_of::<InvocationObservationReferenceV1>()),
        max_reply_bytes: InvocationReceiptV3::required_reply_bytes(512).unwrap(),
        max_control_reply_bytes: size_of::<InvocationControlObservationV1>(),
        max_held_bytes: 0,
    };
    l.max_held_bytes = l.required_hold_bytes().unwrap();
    l
}
fn query<'a>(
    o: &'a SourceCoordinator,
    v: &mut InvocationObservationContextV1,
    c: &SettingsExecutionContextV1,
    r: &mut Recorder,
) -> Result<InvocationReceiptV3<'a>, InvocationError> {
    query_with(o, v, c, &producer::release(c), &policy(c), Some(true), r)
}
fn query_with<'a>(
    o: &'a SourceCoordinator,
    v: &mut InvocationObservationContextV1,
    c: &SettingsExecutionContextV1,
    native: &TrustedRetainedReleaseResultV1<'_>,
    current_policy: &InvocationObservationPolicyV1,
    permission: Option<bool>,
    r: &mut Recorder,
) -> Result<InvocationReceiptV3<'a>, InvocationError> {
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
    o.query_invocation_receipt_v3(
        v,
        InvocationReceiptRequestV3::RetainedRelease(native),
        current_policy,
        &TrustedInvocationReceiptReleaseV1 {
            observation: &ordinary,
            operation_receipt_allowed: permission,
        },
        r,
    )
}
#[test]
fn release_results_compose_through_an_explicit_profile_without_executing_or_reconstructing() {
    let mut f = producer::disposed();
    let c = producer::context();
    let current = producer::facts(&c);
    let mut results = Vec::new();
    for operation in [
        RetainedReleaseOperationV1::SourceBinding,
        RetainedReleaseOperationV1::SourceBinding,
        RetainedReleaseOperationV1::AnnotationBinding,
        RetainedReleaseOperationV1::AnnotationBinding,
        RetainedReleaseOperationV1::Provenance,
        RetainedReleaseOperationV1::Provenance,
    ] {
        let q = producer::request(&mut f, operation);
        let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        f.owner
            .bind_invocation_observation_policy_v1(
                &id,
                &c,
                &policy(&c),
                InvocationObservationPolicyLimitsV1 {
                    max_context_bytes: 2048,
                    max_result_bytes: 512,
                },
                &facts(&c),
                &mut f.recorder,
            )
            .unwrap();
        f.owner
            .submit_retained_release_v1(&id, &q, &producer::admission(&current), &mut f.recorder)
            .unwrap();
        let mut observer = f
            .owner
            .open_invocation_observation_with_receipts_v3(
                &id,
                &c,
                &policy(&c),
                limits(),
                InvocationReceiptAdmissionV1 {
                    observation: &facts(&c),
                    capabilities: InvocationReceiptCapabilitiesV1 {
                        lifecycle: true,
                        ..Default::default()
                    },
                    operation_receipt_admission_allowed: Some(true),
                },
                &mut f.recorder,
            )
            .unwrap();
        assert!(matches!(
            query(&f.owner, &mut observer, &c, &mut f.recorder),
            Err(InvocationError::Unavailable)
        ));
        assert_eq!(
            f.owner.invocation_state(&id, &mut f.recorder),
            Ok(InvocationState::Pending)
        );
        f.owner
            .run_retained_release_v1(&id, &current, &mut f.recorder)
            .unwrap();
        let generation = f.owner.generation();
        let InvocationReceiptV3::RetainedRelease(result) =
            query(&f.owner, &mut observer, &c, &mut f.recorder).unwrap()
        else {
            panic!("wrong family")
        };
        assert_eq!(result.operation, operation);
        assert_eq!(result.invocation, id);
        assert_eq!(f.owner.generation(), generation);
        results.push((observer, result));
    }
    for (mut observer, result) in results {
        let InvocationReceiptV3::RetainedRelease(historical) =
            query(&f.owner, &mut observer, &c, &mut f.recorder).unwrap()
        else {
            panic!("wrong family")
        };
        assert_eq!(historical, result);
        f.owner
            .forget_invocation_outcome(&result.invocation, &mut f.recorder)
            .unwrap();
        assert!(matches!(
            query(&f.owner, &mut observer, &c, &mut f.recorder),
            Err(InvocationError::Unavailable)
        ));
        let control = InvocationObservationPolicyControlV1 {
            context: &c,
            owner_control_allowed: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        };
        f.owner
            .retire_invocation_observation_v1(&mut observer, &control, &mut f.recorder)
            .unwrap();
        f.owner
            .retire_retained_release_v1(&result.invocation, &control, &mut f.recorder)
            .unwrap();
        f.owner
            .retire_invocation_observation_policy_v1(&result.invocation, &control, &mut f.recorder)
            .unwrap();
        f.owner
            .retire_invocation(&result.invocation, &mut f.recorder)
            .unwrap();
    }
    assert_eq!(f.owner.retention_usage().reserved, 0);
}

fn bind(
    o: &mut SourceCoordinator,
    id: &InvocationId,
    c: &SettingsExecutionContextV1,
    p: &InvocationObservationPolicyV1,
    r: &mut Recorder,
) {
    o.bind_invocation_observation_policy_v1(
        id,
        c,
        p,
        InvocationObservationPolicyLimitsV1 {
            max_context_bytes: 2048,
            max_result_bytes: 512,
        },
        &facts(c),
        r,
    )
    .unwrap();
}
fn open(
    o: &mut SourceCoordinator,
    id: &InvocationId,
    c: &SettingsExecutionContextV1,
    p: &InvocationObservationPolicyV1,
    enabled: bool,
    r: &mut Recorder,
) -> InvocationObservationContextV1 {
    o.open_invocation_observation_with_receipts_v3(
        id,
        c,
        p,
        limits(),
        InvocationReceiptAdmissionV1 {
            observation: &facts(c),
            capabilities: InvocationReceiptCapabilitiesV1 {
                lifecycle: enabled,
                ..Default::default()
            },
            operation_receipt_admission_allowed: Some(true),
        },
        r,
    )
    .unwrap()
}
#[test]
fn original_captured_current_and_native_release_checks_are_conjunctive_not_upgrades() {
    for bits in 0..8 {
        let mut f = producer::disposed();
        let c = producer::context();
        let current = producer::facts(&c);
        let q = producer::request(&mut f, RetainedReleaseOperationV1::SourceBinding);
        let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        let mut original = policy(&c);
        original.publication = bits & 1 != 0;
        bind(&mut f.owner, &id, &c, &original, &mut f.recorder);
        f.owner
            .submit_retained_release_v1(&id, &q, &producer::admission(&current), &mut f.recorder)
            .unwrap();
        let mut ceiling = policy(&c);
        ceiling.publication = bits & 2 != 0;
        let mut v = open(&mut f.owner, &id, &c, &ceiling, true, &mut f.recorder);
        f.owner
            .run_retained_release_v1(&id, &current, &mut f.recorder)
            .unwrap();
        let mut current_policy = policy(&c);
        current_policy.publication = bits & 4 != 0;
        assert_eq!(
            query_with(
                &f.owner,
                &mut v,
                &c,
                &producer::release(&c),
                &current_policy,
                Some(true),
                &mut f.recorder
            )
            .is_ok(),
            bits == 7
        );
        if bits == 7 {
            let before = f.owner.retention_usage();
            let generation = f.owner.generation();
            for field in 0..7 {
                for denied in [None, Some(false)] {
                    let mut native = producer::release(&c);
                    match field {
                        0 => native.publication_allowed = denied,
                        1 => native.outcome_readable = denied,
                        2 => native.result_allowed = denied,
                        3 => native.result_context_current = denied,
                        4 => native.historical_document_readable = denied,
                        5 => native.reply_ready = denied,
                        _ => native.resources_ready = denied,
                    }
                    assert!(matches!(
                        query_with(
                            &f.owner,
                            &mut v,
                            &c,
                            &native,
                            &current_policy,
                            Some(true),
                            &mut f.recorder
                        ),
                        Err(InvocationError::Unavailable)
                    ));
                }
            }
            for denied in [None, Some(false)] {
                assert!(matches!(
                    query_with(
                        &f.owner,
                        &mut v,
                        &c,
                        &producer::release(&c),
                        &current_policy,
                        denied,
                        &mut f.recorder
                    ),
                    Err(InvocationError::Unavailable)
                ));
            }
            let mut native = producer::release(&c);
            native.publication_epoch += 1;
            assert!(matches!(
                query_with(
                    &f.owner,
                    &mut v,
                    &c,
                    &native,
                    &current_policy,
                    Some(true),
                    &mut f.recorder
                ),
                Err(InvocationError::Unavailable)
            ));
            assert!(query(&f.owner, &mut v, &c, &mut f.recorder).is_ok());
            assert_eq!(f.owner.generation(), generation);
            assert_eq!(f.owner.retention_usage(), before);
        }
    }
}
#[test]
fn release_capability_and_complete_frame_are_admission_bound_and_old_profiles_cannot_upgrade() {
    let mut f = producer::disposed();
    let c = producer::context();
    let current = producer::facts(&c);
    let q = producer::request(&mut f, RetainedReleaseOperationV1::SourceBinding);
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    bind(&mut f.owner, &id, &c, &policy(&c), &mut f.recorder);
    f.owner
        .submit_retained_release_v1(&id, &q, &producer::admission(&current), &mut f.recorder)
        .unwrap();
    let before = f.owner.retention_usage();
    for atomic in [false, true] {
        let mut l = limits();
        if !atomic {
            l.max_reply_bytes -= 1;
            l.max_held_bytes = l.required_hold_bytes().unwrap();
        }
        let result = f.owner.open_invocation_observation_with_receipts_v3(
            &id,
            &c,
            &policy(&c),
            l,
            InvocationReceiptAdmissionV1 {
                observation: &facts(&c),
                capabilities: InvocationReceiptCapabilitiesV1 {
                    lifecycle: true,
                    atomic_annotation: atomic,
                    ..Default::default()
                },
                operation_receipt_admission_allowed: Some(true),
            },
            &mut f.recorder,
        );
        assert!(
            matches!(result,Err(e) if e==if atomic {InvocationError::Unavailable}else{InvocationError::Limit})
        );
        assert_eq!(f.owner.retention_usage(), before);
    }
    let mut v = open(&mut f.owner, &id, &c, &policy(&c), false, &mut f.recorder);
    f.owner
        .run_retained_release_v1(&id, &current, &mut f.recorder)
        .unwrap();
    assert!(matches!(
        query(&f.owner, &mut v, &c, &mut f.recorder),
        Err(InvocationError::Unavailable)
    ));
    let mut old = f
        .owner
        .open_invocation_observation_with_receipts_v2(
            &id,
            &c,
            &policy(&c),
            limits(),
            InvocationReceiptAdmissionV1 {
                observation: &facts(&c),
                capabilities: InvocationReceiptCapabilitiesV1 {
                    lifecycle: true,
                    ..Default::default()
                },
                operation_receipt_admission_allowed: Some(true),
            },
            &mut f.recorder,
        )
        .unwrap();
    assert!(matches!(
        query(&f.owner, &mut old, &c, &mut f.recorder),
        Err(InvocationError::Unavailable)
    ));
    assert_eq!(
        InvocationReceiptV3::required_reply_bytes(usize::MAX),
        Err(InvocationError::Limit)
    );
}
#[test]
fn capture_saturation_preserves_actual_release_receipt_admission_and_delivery() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for mode in 0..4 {
        let mut f = producer::disposed();
        let c = producer::context();
        let current = producer::facts(&c);
        let q = producer::request(&mut f, RetainedReleaseOperationV1::SourceBinding);
        let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        bind(&mut f.owner, &id, &c, &policy(&c), &mut f.recorder);
        f.owner
            .submit_retained_release_v1(&id, &q, &producer::admission(&current), &mut f.recorder)
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
        let mut v = open(&mut f.owner, &id, &c, &policy(&c), true, &mut r);
        f.owner
            .run_retained_release_v1(&id, &current, &mut f.recorder)
            .unwrap();
        assert_eq!(
            query(&f.owner, &mut v, &c, &mut r).unwrap().invocation(),
            &id
        );
        if mode == 1 || mode == 2 {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for boundary in [
                Boundary::InvocationReceiptAdmission,
                Boundary::InvocationReceiptRelease,
            ] {
                let matching: Vec<_> = events.iter().filter(|e| e.boundary == boundary).collect();
                assert_eq!(matching.len(), 1);
                assert_eq!(matching[0].disposition, Disposition::Completed);
                assert_eq!(matching[0].service_time.is_some(), mode == 2);
            }
            assert!(!format!("{events:?}").contains("session"));
        }
    }
}

#[test]
fn observer_permission_never_overrides_original_native_release_result_ceilings() {
    for bits in 0..8 {
        let mut f = producer::disposed();
        let c = producer::context();
        let current = producer::facts(&c);
        let q = producer::request(&mut f, RetainedReleaseOperationV1::SourceBinding);
        let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        bind(&mut f.owner, &id, &c, &policy(&c), &mut f.recorder);
        let mut a = producer::admission(&current);
        a.publication_allowed = Some(bits & 1 != 0);
        a.result_admission_allowed = Some(bits & 2 != 0);
        a.result_allowed = Some(bits & 4 != 0);
        f.owner
            .submit_retained_release_v1(&id, &q, &a, &mut f.recorder)
            .unwrap();
        let mut v = open(&mut f.owner, &id, &c, &policy(&c), true, &mut f.recorder);
        f.owner
            .run_retained_release_v1(&id, &current, &mut f.recorder)
            .unwrap();
        assert_eq!(
            query(&f.owner, &mut v, &c, &mut f.recorder).is_ok(),
            bits == 7
        );
    }
}
#[test]
fn known_stop_or_stale_release_has_no_identity_result_and_does_not_fault_the_observer() {
    for stopped in [false, true] {
        let mut f = producer::disposed();
        let c = producer::context();
        let current = producer::facts(&c);
        let mut q = producer::request(&mut f, RetainedReleaseOperationV1::SourceBinding);
        if !stopped {
            q.expected.generation += 1;
        }
        let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        bind(&mut f.owner, &id, &c, &policy(&c), &mut f.recorder);
        f.owner
            .submit_retained_release_v1(&id, &q, &producer::admission(&current), &mut f.recorder)
            .unwrap();
        let mut v = open(&mut f.owner, &id, &c, &policy(&c), true, &mut f.recorder);
        if stopped {
            f.owner.stop_invocation(&id, &mut f.recorder).unwrap();
        } else {
            f.owner
                .run_retained_release_v1(&id, &current, &mut f.recorder)
                .unwrap();
        }
        assert!(matches!(
            query(&f.owner, &mut v, &c, &mut f.recorder),
            Err(InvocationError::OutcomeUnavailable)
        ));
        assert!(!f.owner.is_faulted());
        let current = TrustedInvocationObservationReleaseV1 {
            context: &c,
            authorized: Some(true),
            scope: Some(true),
            continuous: Some(true),
            live: Some(true),
            retained: Some(true),
            transfer_ready: Some(true),
            control_ready: Some(true),
        };
        assert_eq!(
            f.owner
                .minimal_invocation_control_status_v1(
                    &mut v,
                    &policy(&c),
                    &current,
                    &mut f.recorder
                )
                .unwrap()
                .phase,
            InvocationObservationPhaseV1::Terminal
        );
    }
}

#[test]
fn prior_disposal_and_relation_results_delegate_without_cross_version_context_reinterpretation() {
    let mut f = producer::Fixture::with_invocations();
    f.owner
        .configure_retention_budget(131072, &mut f.recorder)
        .unwrap();
    let c = producer::context();
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    bind(&mut f.owner, &id, &c, &policy(&c), &mut f.recorder);
    let plan = f
        .owner
        .prepare_retained_disposal(&f.doc, 0, &mut f.recorder)
        .unwrap();
    let df = TrustedRetainedDisposalFactsV1 {
        context: &c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        disposal_allowed: Some(true),
        resources_ready: Some(true),
        preparation_reserved: Some(true),
    };
    f.owner
        .submit_retained_disposal_v1(
            &id,
            &plan,
            &RetainedDisposalAdmissionV1 {
                facts: &df,
                limits: RetainedDisposalLimitsV1 {
                    max_capture_bytes: 4096,
                    max_result_bytes: 512,
                },
                publication_epoch: 7,
                publication_allowed: Some(true),
                result_admission_allowed: Some(true),
                result_allowed: Some(true),
            },
            &mut f.recorder,
        )
        .unwrap();
    let mut v = open(&mut f.owner, &id, &c, &policy(&c), true, &mut f.recorder);
    f.owner
        .run_retained_disposal_v1(&id, &df, &mut f.recorder)
        .unwrap();
    let native = TrustedRetainedDisposalResultV1 {
        context: &c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        outcome_readable: Some(true),
        result_allowed: Some(true),
        result_context_current: Some(true),
        historical_document_readable: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
    };
    let current = TrustedInvocationObservationReleaseV1 {
        context: &c,
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
    let result_facts = TrustedInvocationReceiptReleaseV1 {
        observation: &ordinary,
        operation_receipt_allowed: Some(true),
    };
    assert!(matches!(
        f.owner.query_invocation_receipt_v2(
            &mut v,
            InvocationReceiptRequestV2::RetainedDisposal(&native),
            &policy(&c),
            &result_facts,
            &mut f.recorder
        ),
        Err(InvocationError::Unavailable)
    ));
    let InvocationReceiptV3::Existing(InvocationReceiptV2::RetainedDisposal(result)) = f
        .owner
        .query_invocation_receipt_v3(
            &mut v,
            InvocationReceiptRequestV3::Existing(InvocationReceiptRequestV2::RetainedDisposal(
                &native,
            )),
            &policy(&c),
            &result_facts,
            &mut f.recorder,
        )
        .unwrap()
    else {
        panic!("wrong family")
    };
    assert_eq!(result.invocation, id);
    f.owner
        .configure_workspaces(
            WorkspaceLimits {
                max_issued: 2,
                max_live: 2,
                max_memberships: 4,
                max_work: 128,
            },
            &mut f.recorder,
        )
        .unwrap();
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    bind(&mut f.owner, &id, &c, &policy(&c), &mut f.recorder);
    f.owner
        .submit_relation_with_result_v1(
            &id,
            &RelationCommandV1::CreateWorkspace,
            &TrustedRelationResultAdmissionV1 {
                context: &c,
                max_result_bytes: 512,
                result_admission_allowed: Some(true),
                context_current: Some(true),
                relation_result_identifiers_allowed: Some(true),
            },
            &mut f.recorder,
        )
        .unwrap();
    let mut v = f
        .owner
        .open_invocation_observation_with_receipts_v3(
            &id,
            &c,
            &policy(&c),
            limits(),
            InvocationReceiptAdmissionV1 {
                observation: &facts(&c),
                capabilities: InvocationReceiptCapabilitiesV1 {
                    relation: true,
                    ..Default::default()
                },
                operation_receipt_admission_allowed: Some(true),
            },
            &mut f.recorder,
        )
        .unwrap();
    f.owner.run(&id, &mut f.recorder).unwrap();
    let native = TrustedRelationResultReleaseV1 {
        context: &c,
        max_result_bytes: 512,
        outcome_readable: Some(true),
        relation_result_readable: Some(true),
        relation_result_context_current: Some(true),
        historical_target_readable: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
    };
    assert!(matches!(
        f.owner.query_invocation_receipt_v1(
            &mut v,
            InvocationReceiptRequestV1::Relation(&native),
            &policy(&c),
            &result_facts,
            &mut f.recorder
        ),
        Err(InvocationError::Unavailable)
    ));
    let InvocationReceiptV3::Existing(InvocationReceiptV2::Existing(
        InvocationReceiptV1::Relation(result),
    )) = f
        .owner
        .query_invocation_receipt_v3(
            &mut v,
            InvocationReceiptRequestV3::Existing(InvocationReceiptRequestV2::Existing(
                InvocationReceiptRequestV1::Relation(&native),
            )),
            &policy(&c),
            &result_facts,
            &mut f.recorder,
        )
        .unwrap()
    else {
        panic!("wrong family")
    };
    assert_eq!(result.invocation, id);
}
