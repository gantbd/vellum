use super::*;
#[test]
fn a_native_receipt_seal_fault_closes_only_the_receipt_facility_without_publication_or_refund() {
    let mut o = SourceCoordinator::with_invocation_limits(
        CoordinatorLimits {
            max_document_incarnations: 4,
            max_participants: 4,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 8,
                max_edit_payload_bytes: 4096,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 4,
            max_retained: 4,
            max_intent_bytes: 8192,
        },
    );
    let mut r = Recorder::disabled();
    o.configure_retention_budget(32768, &mut r).unwrap();
    o.configure_workspaces(
        WorkspaceLimits {
            max_issued: 4,
            max_live: 4,
            max_memberships: 16,
            max_work: 4096,
        },
        &mut r,
    )
    .unwrap();
    let c = SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "p".into(),
        destination: "d".into(),
        profile: "receipt/1".into(),
        continuity: "c".into(),
    };
    let p = InvocationObservationPolicyV1 {
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
    let af = TrustedInvocationPolicyFactsV1 {
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
    let id = o.issue_invocation(&mut r).unwrap();
    let other = o.issue_invocation(&mut r).unwrap();
    o.bind_invocation_observation_policy_v1(
        &id,
        &c,
        &p,
        InvocationObservationPolicyLimitsV1 {
            max_context_bytes: 2048,
            max_result_bytes: 512,
        },
        &af,
        &mut r,
    )
    .unwrap();
    let native_admit = TrustedRelationResultAdmissionV1 {
        context: &c,
        max_result_bytes: 512,
        result_admission_allowed: Some(true),
        context_current: Some(true),
        relation_result_identifiers_allowed: Some(true),
    };
    for target in [&id, &other] {
        o.submit_relation_with_result_v1(
            target,
            &RelationCommandV1::CreateWorkspace,
            &native_admit,
            &mut r,
        )
        .unwrap();
        o.run(target, &mut r).unwrap();
    }
    let mut observer = o
        .open_invocation_observation_with_receipts_v1(
            &id,
            &c,
            &p,
            InvocationObservationLimitsV1 {
                max_waiters: 0,
                max_issued: 0,
                max_wait_steps: 0,
                max_context_bytes: 2048,
                max_wait_bytes: 256,
                max_reference_bytes: 256,
                max_reply_bytes: 2048,
                max_control_reply_bytes: 256,
                max_held_bytes: 8192,
            },
            InvocationReceiptAdmissionV1 {
                observation: &af,
                capabilities: InvocationReceiptCapabilitiesV1 {
                    relation: true,
                    ..Default::default()
                },
                operation_receipt_admission_allowed: Some(true),
            },
            &mut r,
        )
        .unwrap();
    let Phase::Terminal(Some(InvocationOutcome::RelationCommitted(receipt))) =
        &o.invocations.slots[other.slot].as_ref().unwrap().phase
    else {
        panic!()
    };
    let replacement = Phase::Terminal(Some(InvocationOutcome::RelationCommitted(receipt.clone())));
    let original = std::mem::replace(
        &mut o.invocations.slots[id.slot].as_mut().unwrap().phase,
        replacement,
    );
    let f = TrustedInvocationObservationReleaseV1 {
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
        current: &f,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    let release = TrustedInvocationReceiptReleaseV1 {
        observation: &ordinary,
        operation_receipt_allowed: Some(true),
    };
    let n = TrustedRelationResultReleaseV1 {
        context: &c,
        max_result_bytes: 512,
        outcome_readable: Some(true),
        relation_result_readable: Some(true),
        relation_result_context_current: Some(true),
        historical_target_readable: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
    };
    let generation = o.generation();
    let held = o.retention_usage().reserved;
    assert_eq!(
        o.query_invocation_receipt_v1(
            &mut observer,
            InvocationReceiptRequestV1::Relation(&n),
            &p,
            &release,
            &mut r
        )
        .err(),
        Some(InvocationError::Faulted)
    );
    assert!(observer.receipt_faulted);
    assert!(observer.live);
    assert_eq!(observer.ordinary_issued, 0);
    assert_eq!(
        o.query_invocation_receipt_v1(
            &mut observer,
            InvocationReceiptRequestV1::Relation(&n),
            &p,
            &release,
            &mut r
        )
        .err(),
        Some(InvocationError::Unavailable)
    );
    assert_eq!(
        o.minimal_invocation_control_status_v1(&mut observer, &p, &f, &mut r)
            .unwrap()
            .phase,
        InvocationObservationPhaseV1::Terminal
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(o.retention_usage().reserved, held);
    o.invocations.slots[id.slot].as_mut().unwrap().phase = original; // Restore only injected fixture corruption.
    let control = InvocationObservationPolicyControlV1 {
        context: &c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    o.retire_invocation_observation_v1(&mut observer, &control, &mut r)
        .unwrap();
    o.retire_invocation_observation_policy_v1(&id, &control, &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}
