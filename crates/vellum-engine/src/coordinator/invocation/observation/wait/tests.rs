use super::*;
fn control(c: &SettingsExecutionContextV1) -> InvocationObservationPolicyControlV1<'_> {
    InvocationObservationPolicyControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
fn facts(c: &SettingsExecutionContextV1) -> TrustedInvocationPolicyFactsV1<'_> {
    TrustedInvocationPolicyFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        resources_ready: Some(true),
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
fn fixture() -> (
    SourceCoordinator,
    InvocationId,
    SettingsExecutionContextV1,
    InvocationObservationPolicyV1,
    InvocationObservationContextV1,
    Recorder,
) {
    let (mut o, id, c, p, mut old, mut r) = super::super::tests::setup();
    let mut l = old.limits;
    l.max_waiters = 1;
    l.max_wait_steps = 4;
    l.max_issued = 2;
    l.max_held_bytes = 16384;
    o.retire_invocation_observation_v1(&mut old, &control(&c), &mut r)
        .unwrap();
    let v = o
        .open_invocation_observation_v1(&id, &c, &p, l, &facts(&c), &mut r)
        .unwrap();
    (o, id, c, p, v, r)
}
#[test]
fn each_valid_tick_strictly_decreases_rank_and_polls_preserve_it_until_absorbing_cutoff() {
    for budget in 1..=4 {
        let (o, id, c, p, mut v, mut r) = fixture();
        let f = release(&c);
        let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
            current: &f,
            ordinary_ready: Some(true),
            resources_ready: Some(true),
        };
        let wf = TrustedInvocationWaitFactsV1 {
            observation: &ordinary,
            wait_capacity: Some(true),
            wait_clock_valid: Some(true),
            retention_ready: Some(true),
            owner_control_allowed: Some(true),
        };
        let after = o
            .query_invocation_observation_v1(
                &mut v,
                InvocationObservationQueryV1::Status,
                &p,
                &ordinary,
                &mut r,
            )
            .unwrap()
            .reference;
        let wait = o
            .start_invocation_observation_wait_v1(
                &mut v,
                InvocationWaitRequestV1 {
                    after: &after,
                    budget,
                    clock: InvocationWaitClockV1 { epoch: 9, tick: 0 },
                },
                &p,
                &wf,
                &mut r,
            )
            .unwrap();
        assert_eq!(v.ordinary_issued, 1); // Wait admission must not stage a result.
        for step in 1..=budget {
            let prior = v.waiters[wait.slot].as_ref().unwrap().remaining;
            assert_eq!(prior, budget - step + 1);
            for _ in 0..17 {
                assert_eq!(
                    o.advance_invocation_observation_wait_v1(
                        &mut v,
                        &wait,
                        InvocationWaitEventV1::Poll,
                        &p,
                        &wf,
                        &mut r
                    ),
                    Ok(InvocationWaitReplyV1::Waiting(wait.clone()))
                );
                assert_eq!(v.waiters[wait.slot].as_ref().unwrap().remaining, prior);
            }
            let out = o
                .advance_invocation_observation_wait_v1(
                    &mut v,
                    &wait,
                    InvocationWaitEventV1::Tick(InvocationWaitClockV1 {
                        epoch: 9,
                        tick: step as u64,
                    }),
                    &p,
                    &wf,
                    &mut r,
                )
                .unwrap();
            let row = v.waiters[wait.slot].as_ref().unwrap();
            assert_eq!(row.remaining, prior - 1);
            assert_eq!(row.waiting, step < budget);
            assert_eq!(
                out,
                if step == budget {
                    InvocationWaitReplyV1::Ended
                } else {
                    InvocationWaitReplyV1::Waiting(wait.clone())
                }
            );
            assert_eq!(
                o.invocation_state(&id, &mut r),
                Ok(InvocationState::Pending)
            );
        }
        assert_eq!(
            o.advance_invocation_observation_wait_v1(
                &mut v,
                &wait,
                InvocationWaitEventV1::Poll,
                &p,
                &wf,
                &mut r
            ),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(v.ordinary_issued, 1);
    }
}
#[test]
fn unissued_ordinary_index_and_wrong_wait_marker_cannot_create_or_address_a_wait() {
    let (o, _id, c, p, mut v, mut r) = fixture();
    let f = release(&c);
    let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &f,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    let wf = TrustedInvocationWaitFactsV1 {
        observation: &ordinary,
        wait_capacity: Some(true),
        wait_clock_valid: Some(true),
        retention_ready: Some(true),
        owner_control_allowed: Some(true),
    };
    let forged = ordinary::InvocationObservationReferenceV1::test_reference(&v, v.ordinary_issued);
    assert_eq!(
        o.start_invocation_observation_wait_v1(
            &mut v,
            InvocationWaitRequestV1 {
                after: &forged,
                budget: 1,
                clock: InvocationWaitClockV1 { epoch: 0, tick: 0 }
            },
            &p,
            &wf,
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    assert!(v.waiters.iter().all(Option::is_none));
    let after = o
        .query_invocation_observation_v1(
            &mut v,
            InvocationObservationQueryV1::Status,
            &p,
            &ordinary,
            &mut r,
        )
        .unwrap()
        .reference;
    let wait = o
        .start_invocation_observation_wait_v1(
            &mut v,
            InvocationWaitRequestV1 {
                after: &after,
                budget: 1,
                clock: InvocationWaitClockV1 { epoch: 0, tick: 0 },
            },
            &p,
            &wf,
            &mut r,
        )
        .unwrap();
    for forged in [
        InvocationWaitIdV1 {
            owner: wait.owner.clone(),
            marker: Arc::new(()),
            slot: wait.slot,
        },
        InvocationWaitIdV1 {
            owner: wait.owner.clone(),
            marker: wait.marker.clone(),
            slot: usize::MAX,
        },
    ] {
        assert_eq!(
            o.advance_invocation_observation_wait_v1(
                &mut v,
                &forged,
                InvocationWaitEventV1::Stop,
                &p,
                &wf,
                &mut r
            ),
            Err(InvocationError::Unavailable)
        );
    }
    assert!(v.waiters[wait.slot].as_ref().unwrap().waiting);
}
#[test]
fn impossible_live_zero_rank_faults_and_closes_the_observer_without_settling_the_producer() {
    let (mut o, id, c, p, mut v, mut r) = fixture();
    let f = release(&c);
    let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &f,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    let wf = TrustedInvocationWaitFactsV1 {
        observation: &ordinary,
        wait_capacity: Some(true),
        wait_clock_valid: Some(true),
        retention_ready: Some(true),
        owner_control_allowed: Some(true),
    };
    let after = o
        .query_invocation_observation_v1(
            &mut v,
            InvocationObservationQueryV1::Status,
            &p,
            &ordinary,
            &mut r,
        )
        .unwrap()
        .reference;
    let wait = o
        .start_invocation_observation_wait_v1(
            &mut v,
            InvocationWaitRequestV1 {
                after: &after,
                budget: 1,
                clock: InvocationWaitClockV1 { epoch: 0, tick: 0 },
            },
            &p,
            &wf,
            &mut r,
        )
        .unwrap();
    v.waiters[wait.slot].as_mut().unwrap().remaining = 0; // Inject impossible private state.
    let held = o.retention_usage().reserved;
    assert_eq!(
        o.advance_invocation_observation_wait_v1(
            &mut v,
            &wait,
            InvocationWaitEventV1::Poll,
            &p,
            &wf,
            &mut r
        ),
        Err(InvocationError::Faulted)
    );
    assert!(!v.live);
    assert!(!v.waiters[wait.slot].as_ref().unwrap().waiting);
    assert_eq!(o.retention_usage().reserved, held);
    assert_eq!(
        o.invocation_state(&id, &mut r),
        Ok(InvocationState::Pending)
    );
    o.retire_invocation_observation_wait_v1(&mut v, &wait, &control(&c), &mut r)
        .unwrap();
    o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 2048);
}
