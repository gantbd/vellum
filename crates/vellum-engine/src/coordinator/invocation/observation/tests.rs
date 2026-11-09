use super::*;
#[test]
fn shutdown_inventory_tracks_host_observer_until_explicit_retirement_not_drop_or_stop() {
    use crate::coordinator::shutdown::tests::{inspect, inspect_read, inspection_current};
    for dropped in [false, true] {
        let (mut o, id, c, _, mut observer, mut r) = setup();
        let exact = observer.context_id_v1();
        let charge = observer.limits.required_hold_bytes().unwrap();
        let before = o.retention_usage();
        let cut = inspect(&o, &mut r);
        let view = inspect_read(&o, &cut, &mut r);
        assert_eq!(view.observation_contexts.len(), 1);
        assert_eq!(view.observation_contexts[0].context, exact);
        assert_eq!(view.observation_contexts[0].invocation, id);
        assert_eq!(view.observation_contexts[0].reserved_bytes, charge);
        assert!(view.observation_contexts[0].host_handle_retained);
        let owner = view
            .owners
            .iter()
            .find(|row| row.owner.role() == ShutdownOwnerRoleV1::InvocationObservers)
            .unwrap();
        assert_eq!(
            (owner.retained_records, owner.additional_reserved_bytes),
            (Some(1), Some(charge))
        );
        o.close_invocation_observation_v1(&mut observer, &control(&c), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage(), before);
        if dropped {
            drop(observer);
            let now = inspect(&o, &mut r);
            assert!(!inspect_read(&o, &now, &mut r).observation_contexts[0].host_handle_retained);
            assert_eq!(
                inspection_current(&o, &cut, &mut r),
                Err(CoordinatorError::Stale)
            );
            assert_eq!(o.retention_usage(), before);
            o.stop_invocation(&id, &mut r).unwrap();
            o.forget_invocation_outcome(&id, &mut r).unwrap();
            o.retire_invocation_observation_policy_v1(&id, &control(&c), &mut r)
                .unwrap();
            o.retire_invocation(&id, &mut r).unwrap();
            let lost = inspect(&o, &mut r);
            let lost = inspect_read(&o, &lost, &mut r);
            assert!(lost.invocations.is_empty());
            assert_eq!(lost.observation_contexts[0].reserved_bytes, charge);
            assert_eq!(o.retention_usage().reserved, charge);
        } else {
            o.retire_invocation_observation_v1(&mut observer, &control(&c), &mut r)
                .unwrap();
            let now = inspect(&o, &mut r);
            assert!(
                inspect_read(&o, &now, &mut r)
                    .observation_contexts
                    .is_empty()
            );
            assert_eq!(o.retention_usage().reserved, before.reserved - charge);
            assert!(
                o.retire_invocation_observation_v1(&mut observer, &control(&c), &mut r)
                    .is_err()
            );
            assert_eq!(o.retention_usage().reserved, before.reserved - charge);
        }
    }
}
#[test]
fn shutdown_fence_refuses_fresh_invocation_observers_without_refunding_original_contexts() {
    let (mut o, id, c, p, mut observer, mut r) = setup();
    let domain = crate::coordinator::shutdown::tests::context();
    let current = crate::coordinator::shutdown::tests::facts(&domain);
    let basis = o
        .capture_shutdown_fence_basis_v1(
            ShutdownFenceLimitsV1 {
                max_context_bytes: 512,
                max_root_bytes: 8192,
                max_capture_bytes: 16384,
            },
            &current,
            &mut r,
        )
        .unwrap();
    o.install_shutdown_fence_v1(&basis, &current, &mut r)
        .unwrap();
    let before = o.retention_usage();
    assert!(matches!(
        o.open_invocation_observation_v1(&id, &c, &p, observer.limits, &facts(&c), &mut r),
        Err(InvocationError::Unavailable)
    ));
    assert_eq!(o.retention_usage(), before);
    o.faulted = true;
    o.retire_invocation_observation_v1(&mut observer, &control(&c), &mut r)
        .unwrap();
    assert!(o.is_faulted());
    assert_eq!(
        o.retention_usage().reserved,
        before.reserved - observer.limits.required_hold_bytes().unwrap()
    );
}
#[test]
fn observer_registry_slot_reuse_never_rebinds_a_retired_context_or_its_credit() {
    let (mut o, id, c, p, mut old, mut r) = setup();
    let limits = old.limits;
    let slot = old.registry_slot;
    let identity = old.context_id_v1();
    o.retire_invocation_observation_v1(&mut old, &control(&c), &mut r)
        .unwrap();
    let mut current = o
        .open_invocation_observation_v1(&id, &c, &p, limits, &facts(&c), &mut r)
        .unwrap();
    assert_eq!(current.registry_slot, slot);
    assert_ne!(current.context_id_v1(), identity);
    let held = o.retention_usage();
    assert!(
        o.retire_invocation_observation_v1(&mut old, &control(&c), &mut r)
            .is_err()
    );
    assert_eq!(o.retention_usage(), held);
    o.retire_invocation_observation_v1(&mut current, &control(&c), &mut r)
        .unwrap();
    assert_eq!(
        o.retention_usage().reserved,
        held.reserved - limits.required_hold_bytes().unwrap()
    );
}
#[test]
fn versioned_stop_binds_invocation_context_not_progress_or_another_context() {
    for typed in [false, true] {
        let (mut o, id, c, _, mut v, mut r) = setup();
        let (_, _, _, _, other, _) = setup();
        let exact = v.context_id_v1();
        let foreign = other.context_id_v1();
        assert_eq!(exact, v.context_id_v1());
        assert_ne!(exact, foreign);
        let debt = o.retention_usage();
        let ctl = control(&c);
        for target in [&foreign, &exact, &exact] {
            let result = crate::coordinator::observation_control::request(
                ObservationStopTargetV2::Invocation(target),
                typed,
                "stop-observation",
                "stop-observation",
                |envelope| {
                    o.stop_decoded_observation_facade_v2(
                        DecodedObservationStopRequestV2 {
                            envelope,
                            native: ObservationStopPortV2::Existing(
                                ObservationStopPortV1::Invocation {
                                    observer: &mut v,
                                    control: &ctl,
                                },
                            ),
                        },
                        &mut r,
                    )
                },
            );
            assert_eq!(result.is_ok(), target == &exact);
            assert_eq!(v.live, target == &foreign);
            assert!(!o.invocation_slot(&id).unwrap().stop_requested);
            assert_eq!(o.retention_usage(), debt);
        }
    }
}
#[test]
fn heterogeneous_stop_closes_only_the_original_invocation_observer_under_protected_control() {
    for allowed in [None, Some(false), Some(true)] {
        for faulted in [false, true] {
            let (mut o, id, c, _, mut v, mut r) = setup();
            let debt = o.retention_usage();
            let generation = o.generation();
            o.faulted = faulted;
            let control = InvocationObservationPolicyControlV1 {
                context: &c,
                owner_control_allowed: allowed,
                retention_released: None,
                delivery_drained: Some(false),
            };
            let result = o.stop_observation_facade_v1(
                ObservationStopPortV1::Invocation {
                    observer: &mut v,
                    control: &control,
                },
                &mut r,
            );
            assert_eq!(result.is_ok(), allowed == Some(true));
            assert_eq!(v.live, allowed != Some(true));
            let slot = o.invocation_slot(&id).unwrap();
            assert!(!slot.stop_requested);
            assert_eq!(slot.phase.state(), Ok(InvocationState::Pending));
            assert_eq!(o.is_faulted(), faulted);
            assert_eq!(o.retention_usage(), debt);
            assert_eq!(o.generation(), generation);
            let (mut foreign, _, _, _, _, mut fr) = setup();
            assert!(
                foreign
                    .stop_observation_facade_v1(
                        ObservationStopPortV1::Invocation {
                            observer: &mut v,
                            control: &control
                        },
                        &mut fr
                    )
                    .is_err()
            );
        }
    }
}
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "p".into(),
        destination: "d".into(),
        profile: "observation/1".into(),
        continuity: "c".into(),
    }
}
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
fn release(c: &SettingsExecutionContextV1) -> TrustedInvocationObservationReleaseV1<'_> {
    TrustedInvocationObservationReleaseV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        control_ready: Some(true),
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
pub(super) fn setup() -> (
    SourceCoordinator,
    InvocationId,
    SettingsExecutionContextV1,
    InvocationObservationPolicyV1,
    InvocationObservationContextV1,
    Recorder,
) {
    setup_with_retention_budget(32768)
}
pub(in crate::coordinator) fn setup_with_retention_budget(
    maximum: usize,
) -> (
    SourceCoordinator,
    InvocationId,
    SettingsExecutionContextV1,
    InvocationObservationPolicyV1,
    InvocationObservationContextV1,
    Recorder,
) {
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
    o.configure_retention_budget(maximum, &mut r).unwrap();
    let c = context();
    let p = policy(&c);
    let id = o.issue_invocation(&mut r).unwrap();
    o.bind_invocation_observation_policy_v1(
        &id,
        &c,
        &p,
        InvocationObservationPolicyLimitsV1 {
            max_context_bytes: 2048,
            max_result_bytes: 512,
        },
        &facts(&c),
        &mut r,
    )
    .unwrap();
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
    let v = o
        .open_invocation_observation_v1(
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
            &facts(&c),
            &mut r,
        )
        .unwrap();
    (o, id, c, p, v, r)
}
#[test]
fn private_same_owner_ticket_capture_substitution_faults_without_disclosing_or_refunding() {
    let (mut o, _id, c, p, mut v, mut r) = setup();
    let other = o.issue_invocation(&mut r).unwrap();
    o.bind_invocation_observation_policy_v1(
        &other,
        &c,
        &p,
        InvocationObservationPolicyLimitsV1 {
            max_context_bytes: 2048,
            max_result_bytes: 512,
        },
        &facts(&c),
        &mut r,
    )
    .unwrap();
    let observation_policy::State::Held(b) = &o.invocations.slots[other.slot]
        .as_ref()
        .unwrap()
        .observation_policy
    else {
        panic!()
    };
    let original = v.original.replace(b.capture.clone()).unwrap();
    let reserved = o.retention_usage().reserved;
    assert_eq!(
        o.minimal_invocation_control_status_v1(&mut v, &p, &release(&c), &mut r),
        Err(InvocationError::Faulted)
    );
    assert!(!v.live);
    assert_eq!(
        o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(o.retention_usage().reserved, reserved);
    v.original = Some(original); // Restore only injected test corruption for cleanup.
    o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 4096);
}
#[test]
fn same_parent_wrong_size_hold_cannot_sponsor_status_or_refund_the_observer() {
    let (mut o, _id, c, p, mut v, mut r) = setup();
    let tiny = o.retention.reserve(1).unwrap();
    let original = std::mem::replace(&mut v.hold, tiny);
    let reserved = o.retention_usage().reserved;
    assert_eq!(
        o.minimal_invocation_control_status_v1(&mut v, &p, &release(&c), &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(o.retention_usage().reserved, reserved);
    let mut tiny = std::mem::replace(&mut v.hold, original);
    o.retention.release(&mut tiny).unwrap();
    o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 2048);
}
#[test]
fn publication_interruption_is_not_a_terminal_phase_and_protected_cleanup_does_not_settle_it() {
    let (mut o, id, c, p, mut v, mut r) = setup();
    let mut protected = o
        .open_invocation_observation_v1(&id, &c, &p, v.limits, &facts(&c), &mut r)
        .unwrap();
    o.publication_fault_after = Some(1);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| o.run(&id, &mut r))).is_err());
    let f = release(&c);
    let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &f,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    assert_eq!(
        o.query_invocation_observation_v1(
            &mut v,
            InvocationObservationQueryV1::Status,
            &p,
            &ordinary,
            &mut r
        ),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.minimal_invocation_control_status_v1(&mut protected, &p, &release(&c), &mut r),
        Err(InvocationError::Faulted)
    );
    o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
        .unwrap();
    o.retire_invocation_observation_v1(&mut protected, &control(&c), &mut r)
        .unwrap();
    o.retire_invocation_observation_policy_v1(&id, &control(&c), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(
        o.invocation_state(&id, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.retire_invocation(&id, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(o.run(&id, &mut r), Err(InvocationError::Faulted));
}
