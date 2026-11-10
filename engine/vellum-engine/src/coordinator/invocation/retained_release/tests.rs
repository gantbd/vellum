use super::*;

#[test]
fn release_observer_receipt_fault_is_absorbing_without_poisoning_control_or_refunding_owners() {
    let (mut o, id, q, mut r) = setup(32768);
    let c = context();
    let policy = InvocationObservationPolicyV1 {
        recipient: c.recipient.clone(),
        epoch: 7,
        grant: true,
        publication: true,
        control: true,
        effect: true,
        reasons: true,
        details: false,
        recovery: true,
    };
    let pf = TrustedInvocationPolicyFactsV1 {
        context: &c,
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
    };
    o.bind_invocation_observation_policy_v1(
        &id,
        &c,
        &policy,
        InvocationObservationPolicyLimitsV1 {
            max_context_bytes: 2048,
            max_result_bytes: 512,
        },
        &pf,
        &mut r,
    )
    .unwrap();
    o.submit_retained_release_v1(&id, &q, &admission(&facts(&c)), &mut r)
        .unwrap();
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
    let mut v = o
        .open_invocation_observation_with_receipts_v3(
            &id,
            &c,
            &policy,
            l,
            InvocationReceiptAdmissionV1 {
                observation: &pf,
                capabilities: InvocationReceiptCapabilitiesV1 {
                    lifecycle: true,
                    ..Default::default()
                },
                operation_receipt_admission_allowed: Some(true),
            },
            &mut r,
        )
        .unwrap();
    o.run_retained_release_v1(&id, &facts(&c), &mut r).unwrap();
    let before = o.retention_usage();
    let generation = o.generation();
    o.invocations.slots[id.slot]
        .as_mut()
        .unwrap()
        .release_hold
        .as_mut()
        .unwrap()
        .candidate = Some(false);
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
    let rf = TrustedInvocationReceiptReleaseV1 {
        observation: &ordinary,
        operation_receipt_allowed: Some(true),
    };
    assert!(matches!(
        o.query_invocation_receipt_v3(
            &mut v,
            InvocationReceiptRequestV3::RetainedRelease(&result(&c)),
            &policy,
            &rf,
            &mut r
        ),
        Err(InvocationError::Faulted)
    ));
    assert!(!o.faulted);
    assert_eq!(o.retention_usage(), before);
    assert_eq!(o.generation(), generation);
    // Repair only deliberate test corruption. Receipt delivery remains closed.
    o.invocations.slots[id.slot]
        .as_mut()
        .unwrap()
        .release_hold
        .as_mut()
        .unwrap()
        .candidate = Some(true);
    assert!(matches!(
        o.query_invocation_receipt_v3(
            &mut v,
            InvocationReceiptRequestV3::RetainedRelease(&result(&c)),
            &policy,
            &rf,
            &mut r
        ),
        Err(InvocationError::Unavailable)
    ));
    assert_eq!(
        o.minimal_invocation_control_status_v1(&mut v, &policy, &current, &mut r)
            .unwrap()
            .phase,
        InvocationObservationPhaseV1::Terminal
    );
    o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
        .unwrap();
    o.retire_retained_release_v1(&id, &control(&c), &mut r)
        .unwrap();
    o.retire_invocation_observation_policy_v1(&id, &control(&c), &mut r)
        .unwrap();
    o.retire_invocation(&id, &mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}

#[test]
fn another_native_operation_cannot_stage_a_result_against_the_original_release_ticket() {
    let (mut o, id, q, mut r) = setup(8192);
    let c = context();
    let f = facts(&c);
    o.submit_retained_release_v1(&id, &q, &admission(&f), &mut r)
        .unwrap();
    o.invocations.slots[id.slot].as_mut().unwrap().phase = Phase::Running;
    let mut substituted = q.clone();
    substituted.operation = RetainedReleaseOperationV1::SourceBinding;
    let generation = o.generation();
    let before = o.retention_usage();
    assert_eq!(
        o.release_retained_for_invocation(&id, &substituted, &mut r),
        Err(ResourceError::Faulted)
    );
    assert!(o.faulted);
    assert_eq!(o.generation(), generation);
    assert_eq!(o.retention_usage(), before);
    assert_eq!(
        o.invocation_slot(&id)
            .unwrap()
            .release_hold
            .as_ref()
            .unwrap()
            .candidate,
        None
    );
    assert_eq!(
        o.retained_release_result_v1(&id, &result(&c), &mut r),
        Err(InvocationError::Faulted)
    );
    o.retire_retained_release_v1(&id, &control(&c), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(
        o.invocation_state(&id, &mut r),
        Err(InvocationError::Faulted)
    );
}
#[test]
fn valid_never_started_release_can_be_stopped_and_discharged_through_an_existing_owner_fault() {
    let (mut o, id, q, mut r) = setup(8192);
    let c = context();
    let f = facts(&c);
    o.submit_retained_release_v1(&id, &q, &admission(&f), &mut r)
        .unwrap();
    let generation = o.generation();
    o.faulted = true;
    assert_eq!(
        o.run_retained_release_v1(&id, &f, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.retire_retained_release_v1(&id, &control(&c), &mut r),
        Err(InvocationError::Pending)
    );
    o.stop_invocation(&id, &mut r).unwrap();
    assert_eq!(
        o.invocation_outcome(&id, &mut r),
        Ok(&InvocationOutcome::StoppedBeforePublication)
    );
    o.retire_retained_release_v1(&id, &control(&c), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert!(o.faulted);
    assert_eq!(o.generation(), generation);
}
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "private authority".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "p".into(),
        destination: "d".into(),
        profile: "native".into(),
        continuity: "c".into(),
    }
}
fn facts(c: &SettingsExecutionContextV1) -> TrustedRetainedReleaseFactsV1<'_> {
    TrustedRetainedReleaseFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        release_allowed: Some(true),
        resources_ready: Some(true),
        preparation_reserved: Some(true),
    }
}
fn admission<'a>(f: &'a TrustedRetainedReleaseFactsV1<'a>) -> RetainedReleaseAdmissionV1<'a> {
    RetainedReleaseAdmissionV1 {
        facts: f,
        limits: RetainedReleaseLimitsV1 {
            max_capture_bytes: 2048,
            max_result_bytes: 512,
        },
        publication_epoch: 7,
        publication_allowed: Some(true),
        result_admission_allowed: Some(true),
        result_allowed: Some(true),
    }
}
fn result(c: &SettingsExecutionContextV1) -> TrustedRetainedReleaseResultV1<'_> {
    TrustedRetainedReleaseResultV1 {
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
fn control(c: &SettingsExecutionContextV1) -> InvocationObservationPolicyControlV1<'_> {
    InvocationObservationPolicyControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
fn setup(
    budget: usize,
) -> (
    SourceCoordinator,
    InvocationId,
    RetainedReleaseRequestV1,
    Recorder,
) {
    let mut o = SourceCoordinator::with_invocation_limits(
        CoordinatorLimits {
            max_document_incarnations: 4,
            max_participants: 4,
            max_staged_bytes: 1024,
            source: Limits {
                max_source_bytes: 128,
                max_edits: 4,
                max_edit_payload_bytes: 128,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 4,
            max_retained: 4,
            max_intent_bytes: 8192,
        },
    );
    let mut r = Recorder::disabled();
    o.configure_retention_budget(budget, &mut r).unwrap();
    let d = o.create(b"private payload", &mut r).unwrap();
    let p = o.prepare_retained_disposal(&d, 0, &mut r).unwrap();
    let expected = o.commit_retained_disposal(p, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    (
        o,
        id,
        RetainedReleaseRequestV1 {
            document: d.document,
            expected,
            operation: RetainedReleaseOperationV1::Provenance,
        },
        r,
    )
}
#[test]
fn actual_release_paths_have_capture_equivalent_timing_without_identity_or_content_labels() {
    for mode in 0..4 {
        let (mut o, id, q, _) = setup(8192);
        let c = context();
        let f = facts(&c);
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 32 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(
                crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
            );
        }
        if mode == 3 {
            r.record(Observation {
                boundary: Boundary::Control,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        o.submit_retained_release_v1(&id, &q, &admission(&f), &mut r)
            .unwrap();
        crate::coordinator::shutdown::tests::invocation_funding(
            &o,
            &id,
            1,
            admission(&f).limits.max_capture_bytes,
            &mut Recorder::disabled(),
        );
        o.run_retained_release_v1(&id, &f, &mut r).unwrap();
        assert!(
            o.retained_release_result_v1(&id, &result(&c), &mut r)
                .unwrap()
                .changed
        );
        o.retire_retained_release_v1(&id, &control(&c), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
        if mode == 1 || mode == 2 {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for boundary in [
                Boundary::RetainedReleaseAdmission,
                Boundary::RetainedReleaseService,
                Boundary::RetainedReleaseResultRelease,
                Boundary::RetainedReleaseRetirement,
            ] {
                let matching: Vec<_> = events.iter().filter(|e| e.boundary == boundary).collect();
                assert_eq!(matching.len(), 1);
                assert_eq!(matching[0].disposition, Disposition::Completed);
                assert_eq!(matching[0].service_time.is_some(), mode == 2);
            }
            assert!(!format!("{events:?}").contains("private"));
        }
    }
}
#[test]
fn exact_parent_funding_and_marker_corruption_never_authorize_publication_or_wrong_refund() {
    for budget in [2047, 2048, 2049] {
        let (mut o, id, q, mut r) = setup(budget);
        let c = context();
        let f = facts(&c);
        let outcome = o.submit_retained_release_v1(&id, &q, &admission(&f), &mut r);
        if budget < 2048 {
            assert_eq!(outcome, Err(InvocationError::Limit));
            assert_eq!(o.retention_usage().reserved, 0);
        } else {
            assert_eq!(outcome, Ok(InvocationState::Pending));
            o.stop_invocation(&id, &mut r).unwrap();
            o.retire_retained_release_v1(&id, &control(&c), &mut r)
                .unwrap();
            assert_eq!(o.retention_usage().reserved, 0);
        }
    }
    for wrong_marker in [false, true] {
        let (mut o, id, q, mut r) = setup(8192);
        let c = context();
        let f = facts(&c);
        o.submit_retained_release_v1(&id, &q, &admission(&f), &mut r)
            .unwrap();
        let extra = o.retention.reserve(71).unwrap();
        let h = o.invocations.slots[id.slot]
            .as_mut()
            .unwrap()
            .release_hold
            .as_mut()
            .unwrap();
        let saved_marker = if wrong_marker {
            Some(std::mem::replace(&mut h.marker, Arc::new(())))
        } else {
            None
        };
        let saved_hold = if !wrong_marker {
            Some(std::mem::replace(&mut h.reservation, extra))
        } else {
            None
        };
        let held = o.retention_usage();
        let generation = o.generation();
        assert_eq!(
            o.run_retained_release_v1(&id, &f, &mut r),
            Err(InvocationError::Faulted)
        );
        assert_eq!(
            o.retire_retained_release_v1(&id, &control(&c), &mut r),
            Err(InvocationError::Faulted)
        );
        assert_eq!(o.retention_usage(), held);
        assert_eq!(o.generation(), generation);
        let h = o.invocations.slots[id.slot]
            .as_mut()
            .unwrap()
            .release_hold
            .as_mut()
            .unwrap();
        if let Some(m) = saved_marker {
            h.marker = m;
        }
        if let Some(h0) = saved_hold {
            h.reservation = h0;
        }
        o.stop_invocation(&id, &mut r).unwrap();
        o.retire_retained_release_v1(&id, &control(&c), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 71);
    }
}
