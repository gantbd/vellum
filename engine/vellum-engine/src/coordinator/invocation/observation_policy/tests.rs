use super::*;
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "p".into(),
        destination: "d".into(),
        profile: "policy/1".into(),
        continuity: "c".into(),
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
fn control(c: &SettingsExecutionContextV1) -> InvocationObservationPolicyControlV1<'_> {
    InvocationObservationPolicyControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
fn setup() -> (
    SourceCoordinator,
    InvocationId,
    SettingsExecutionContextV1,
    InvocationObservationPolicyV1,
    InvocationObservationPolicyLimitsV1,
    Recorder,
) {
    let mut o = SourceCoordinator::with_invocation_limits(
        CoordinatorLimits {
            max_document_incarnations: 2,
            max_participants: 2,
            max_staged_bytes: 1024,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 8,
                max_edit_payload_bytes: 1024,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 4,
            max_retained: 4,
            max_intent_bytes: 8192,
        },
    );
    let mut r = Recorder::disabled();
    o.configure_retention_budget(16384, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let p = InvocationObservationPolicyV1 {
        recipient: c.recipient.clone(),
        epoch: 0,
        grant: true,
        publication: true,
        control: true,
        effect: true,
        reasons: true,
        details: false,
        recovery: true,
    };
    let l = InvocationObservationPolicyLimitsV1 {
        max_context_bytes: 2048,
        max_result_bytes: 512,
    };
    o.bind_invocation_observation_policy_v1(&id, &c, &p, l, &facts(&c), &mut r)
        .unwrap();
    (o, id, c, p, l, r)
}
#[test]
fn actual_private_capture_users_prevent_metadata_discharge_until_released() {
    let (mut o, id, c, _, _, mut r) = setup();
    crate::coordinator::shutdown::tests::invocation_funding(&o, &id, 1, 2048, &mut r);
    let State::Held(b) = &o.invocations.slots[id.slot]
        .as_ref()
        .unwrap()
        .observation_policy
    else {
        panic!()
    };
    // Models a real private capture user, not an implemented F06 context.
    let attached = b.capture.clone();
    assert_eq!(
        o.invocation_observation_policy_usage_v1(&id, &mut r)
            .unwrap()
            .attached_capture_users,
        1
    );
    assert_eq!(
        o.retire_invocation_observation_policy_v1(&id, &control(&c), &mut r),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(o.retention_usage().reserved, 2048);
    drop(attached);
    o.retire_invocation_observation_policy_v1(&id, &control(&c), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    crate::coordinator::shutdown::tests::invocation_funding(&o, &id, 0, 0, &mut r);
    assert!(
        o.invocation_observation_policy_usage_v1(&id, &mut r)
            .unwrap()
            .spent
    );
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
}
#[test]
fn same_owner_foreign_ticket_capture_is_not_original_policy_provenance() {
    let (mut o, id, c, p, l, mut r) = setup();
    let other = o.issue_invocation(&mut r).unwrap();
    o.bind_invocation_observation_policy_v1(&other, &c, &p, l, &facts(&c), &mut r)
        .unwrap();
    let swap = |o: &mut SourceCoordinator| {
        let (left, right) = o.invocations.slots.split_at_mut(other.slot);
        let State::Held(a) = &mut left[id.slot].as_mut().unwrap().observation_policy else {
            panic!()
        };
        let State::Held(b) = &mut right[0].as_mut().unwrap().observation_policy else {
            panic!()
        };
        std::mem::swap(&mut a.capture, &mut b.capture);
    };
    swap(&mut o);
    assert_eq!(
        o.invocation_observation_policy_usage_v1(&id, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.bind_invocation_observation_policy_v1(&id, &c, &p, l, &facts(&c), &mut r),
        Err(InvocationError::Faulted)
    );
    assert!(o.is_faulted());
    assert_eq!(o.retention_usage().reserved, 4096);
    // Restore fixture witnesses only; production does not repair provenance.
    swap(&mut o);
    o.retire_invocation_observation_policy_v1(&id, &control(&c), &mut r)
        .unwrap();
    o.retire_invocation_observation_policy_v1(&other, &control(&c), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}
#[test]
fn same_parent_wrong_sized_reservation_is_not_policy_funding_and_fault_cleanup_is_once_only() {
    let (mut o, id, c, p, l, mut r) = setup();
    let wrong = o.retention.reserve(1).unwrap();
    let State::Held(b) = &mut o.invocations.slots[id.slot]
        .as_mut()
        .unwrap()
        .observation_policy
    else {
        panic!()
    };
    let original = std::mem::replace(&mut b.reservation, wrong);
    assert_eq!(
        o.bind_invocation_observation_policy_v1(&id, &c, &p, l, &facts(&c), &mut r),
        Err(InvocationError::Faulted)
    );
    assert!(o.is_faulted());
    assert_eq!(o.retention_usage().reserved, 2049);
    assert_eq!(
        o.retire_invocation_observation_policy_v1(&id, &control(&c), &mut r),
        Err(InvocationError::Faulted)
    );
    let State::Held(b) = &mut o.invocations.slots[id.slot]
        .as_mut()
        .unwrap()
        .observation_policy
    else {
        panic!()
    };
    // Fixture repair only; no production path replaces a reservation witness.
    let mut wrong = std::mem::replace(&mut b.reservation, original);
    o.retention.release(&mut wrong).unwrap();
    o.retire_invocation_observation_policy_v1(&id, &control(&c), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(
        o.retire_invocation_observation_policy_v1(&id, &control(&c), &mut r),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(o.invocation_usage().issued_lifetime, 1);
}
#[test]
fn policy_discharge_after_actual_publication_interruption_does_not_settle_or_reexecute_the_producer()
 {
    let (mut o, id, c, _, _, mut r) = setup();
    let d = o.create(b"A", &mut r).unwrap();
    o.submit(
        &id,
        &SourceIntent {
            writes: vec![DocumentEdits {
                expected: d,
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: "B".into(),
                }],
            }],
            reads: vec![],
        },
        &mut r,
    )
    .unwrap();
    o.publication_fault_after = Some(1);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| o.run(&id, &mut r))).is_err());
    assert!(o.is_faulted());
    assert_eq!(
        o.invocation_outcome(&id, &mut r),
        Err(InvocationError::Faulted)
    );
    o.retire_invocation_observation_policy_v1(&id, &control(&c), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(o.run(&id, &mut r), Err(InvocationError::Faulted));
    assert_eq!(
        o.retire_invocation(&id, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(o.invocation_usage().faulted_executions, 1);
}
