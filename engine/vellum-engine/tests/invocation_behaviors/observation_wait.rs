use super::*;
fn setup_wait(
    max_waiters: usize,
) -> (
    SourceCoordinator,
    InvocationId,
    SettingsExecutionContextV1,
    InvocationObservationPolicyV1,
    InvocationObservationContextV1,
    Recorder,
) {
    let (mut o, id, c, p, mut r) = pending();
    let mut l = limits();
    l.max_waiters = max_waiters;
    l.max_wait_steps = 3;
    l.max_issued = 4;
    l.max_held_bytes = 32768;
    let v = o
        .open_invocation_observation_v1(&id, &c, &p, l, &facts(&c), &mut r)
        .unwrap();
    (o, id, c, p, v, r)
}
fn wait_facts<'a>(
    ordinary: &'a TrustedInvocationOrdinaryObservationReleaseV1<'a>,
) -> TrustedInvocationWaitFactsV1<'a> {
    TrustedInvocationWaitFactsV1 {
        observation: ordinary,
        wait_capacity: Some(true),
        wait_clock_valid: Some(true),
        retention_ready: Some(true),
        owner_control_allowed: Some(true),
    }
}
fn request(after: &InvocationObservationReferenceV1, budget: usize) -> InvocationWaitRequestV1<'_> {
    InvocationWaitRequestV1 {
        after,
        budget,
        clock: InvocationWaitClockV1 { epoch: 7, tick: 0 },
    }
}
#[test]
fn logical_ticks_end_only_the_wait_and_capacity_requires_separate_protected_discharge() {
    let (mut o, id, c, p, mut r) = pending();
    let mut l = limits();
    l.max_waiters = 1;
    l.max_wait_steps = 2;
    l.max_issued = 2;
    l.max_held_bytes = 16384;
    let mut v = o
        .open_invocation_observation_v1(&id, &c, &p, l, &facts(&c), &mut r)
        .unwrap();
    let held = o.retention_usage().reserved;
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
    let request = InvocationWaitRequestV1 {
        after: &after,
        budget: 2,
        clock: InvocationWaitClockV1 { epoch: 7, tick: 0 },
    };
    let wait = o
        .start_invocation_observation_wait_v1(&mut v, request, &p, &wf, &mut r)
        .unwrap();
    for _ in 0..20 {
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
    }
    assert_eq!(
        o.advance_invocation_observation_wait_v1(
            &mut v,
            &wait,
            InvocationWaitEventV1::Tick(InvocationWaitClockV1 { epoch: 7, tick: 1 }),
            &p,
            &wf,
            &mut r
        ),
        Ok(InvocationWaitReplyV1::Waiting(wait.clone()))
    );
    assert_eq!(
        o.advance_invocation_observation_wait_v1(
            &mut v,
            &wait,
            InvocationWaitEventV1::Tick(InvocationWaitClockV1 { epoch: 7, tick: 2 }),
            &p,
            &wf,
            &mut r
        ),
        Ok(InvocationWaitReplyV1::Ended)
    );
    assert_eq!(
        o.invocation_state(&id, &mut r),
        Ok(InvocationState::Pending)
    );
    assert_eq!(
        o.start_invocation_observation_wait_v1(&mut v, request, &p, &wf, &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(
        o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        o.invocation_observation_wait_usage_v1(&v, &mut r).unwrap(),
        InvocationWaitUsageV1 {
            retained: 1,
            waiting: 0,
            max_waiters: 1
        }
    );
    o.retire_invocation_observation_wait_v1(&mut v, &wait, &control(&c), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, held);
    let new_wait = o
        .start_invocation_observation_wait_v1(&mut v, request, &p, &wf, &mut r)
        .unwrap();
    assert_ne!(wait, new_wait);
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
    o.close_invocation_observation_v1(&mut v, &control(&c), &mut r)
        .unwrap();
    o.retire_invocation_observation_wait_v1(&mut v, &new_wait, &control(&c), &mut r)
        .unwrap();
    o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
        .unwrap();
    o.run(&id, &mut r).unwrap();
}

#[test]
fn completion_delivers_once_to_each_wait_without_polling_executing_the_producer() {
    let (mut o, id, c, p, mut v, mut r) = setup_wait(2);
    let f = release(&c);
    let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &f,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    let wf = wait_facts(&ordinary);
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
    let a = o
        .start_invocation_observation_wait_v1(&mut v, request(&after, 3), &p, &wf, &mut r)
        .unwrap();
    let b = o
        .start_invocation_observation_wait_v1(&mut v, request(&after, 3), &p, &wf, &mut r)
        .unwrap();
    assert_eq!(
        o.invocation_state(&id, &mut r),
        Ok(InvocationState::Pending)
    );
    o.run(&id, &mut r).unwrap();
    let mut delivered = None;
    for wait in [&a, &b] {
        let InvocationWaitReplyV1::Observed(out) = o
            .advance_invocation_observation_wait_v1(
                &mut v,
                wait,
                InvocationWaitEventV1::Poll,
                &p,
                &wf,
                &mut r,
            )
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(out.phase, Some(InvocationObservationPhaseV1::Terminal));
        assert_eq!(
            out.outcome.unwrap().publication,
            Some(InvocationPublicationV1::Applied)
        );
        if let Some(prior) = &delivered {
            assert_eq!(prior, &out.reference);
        } else {
            delivered = Some(out.reference);
        }
        assert_eq!(
            o.advance_invocation_observation_wait_v1(
                &mut v,
                wait,
                InvocationWaitEventV1::Poll,
                &p,
                &wf,
                &mut r
            ),
            Err(InvocationError::Unavailable)
        );
    }
    assert_eq!(
        o.invocation_observation_wait_usage_v1(&v, &mut r).unwrap(),
        InvocationWaitUsageV1 {
            retained: 2,
            waiting: 0,
            max_waiters: 2
        }
    );
    assert_eq!(
        o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r),
        Err(InvocationError::Unavailable)
    );
    o.retire_invocation_observation_wait_v1(&mut v, &a, &control(&c), &mut r)
        .unwrap();
    o.retire_invocation_observation_wait_v1(&mut v, &b, &control(&c), &mut r)
        .unwrap();
    o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
        .unwrap();
}

#[test]
fn invalid_clock_ends_only_its_wait_without_fabricating_time_or_poisoning_the_context() {
    for invalid in 0..5 {
        let (o, id, c, p, mut v, mut r) = setup_wait(2);
        let f = release(&c);
        let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
            current: &f,
            ordinary_ready: Some(true),
            resources_ready: Some(true),
        };
        let mut wf = wait_facts(&ordinary);
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
        let a = o
            .start_invocation_observation_wait_v1(&mut v, request(&after, 3), &p, &wf, &mut r)
            .unwrap();
        let b = o
            .start_invocation_observation_wait_v1(&mut v, request(&after, 3), &p, &wf, &mut r)
            .unwrap();
        assert!(
            o.advance_invocation_observation_wait_v1(
                &mut v,
                &a,
                InvocationWaitEventV1::Tick(InvocationWaitClockV1 { epoch: 7, tick: 1 }),
                &p,
                &wf,
                &mut r
            )
            .is_ok()
        );
        let mut clock = InvocationWaitClockV1 { epoch: 7, tick: 2 };
        match invalid {
            0 => clock.epoch = 8,
            1 => clock.tick = 1,
            2 => clock.tick = 0,
            3 => wf.wait_clock_valid = None,
            _ => wf.wait_clock_valid = Some(false),
        }
        assert_eq!(
            o.advance_invocation_observation_wait_v1(
                &mut v,
                &a,
                InvocationWaitEventV1::Tick(clock),
                &p,
                &wf,
                &mut r
            ),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(
            o.invocation_observation_wait_usage_v1(&v, &mut r).unwrap(),
            InvocationWaitUsageV1 {
                retained: 2,
                waiting: 1,
                max_waiters: 2
            }
        );
        assert_eq!(
            o.minimal_invocation_control_status_v1(&mut v, &p, &f, &mut r)
                .unwrap()
                .phase,
            InvocationObservationPhaseV1::Pending
        );
        assert_eq!(
            o.invocation_state(&id, &mut r),
            Ok(InvocationState::Pending)
        );
        wf.wait_clock_valid = Some(true);
        assert_eq!(
            o.advance_invocation_observation_wait_v1(
                &mut v,
                &b,
                InvocationWaitEventV1::Poll,
                &p,
                &wf,
                &mut r
            ),
            Ok(InvocationWaitReplyV1::Waiting(b.clone()))
        );
    }
}

#[test]
fn ordinary_issuance_exhaustion_ends_the_wait_but_protected_terminal_control_survives() {
    let (mut o, id, c, p, mut r) = pending();
    let mut l = limits();
    l.max_waiters = 1;
    l.max_wait_steps = 2;
    l.max_issued = 1;
    l.max_held_bytes = 16384;
    let mut v = o
        .open_invocation_observation_v1(&id, &c, &p, l, &facts(&c), &mut r)
        .unwrap();
    let f = release(&c);
    let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &f,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    let wf = wait_facts(&ordinary);
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
        .start_invocation_observation_wait_v1(&mut v, request(&after, 2), &p, &wf, &mut r)
        .unwrap();
    o.run(&id, &mut r).unwrap();
    assert_eq!(
        o.advance_invocation_observation_wait_v1(
            &mut v,
            &wait,
            InvocationWaitEventV1::Poll,
            &p,
            &wf,
            &mut r
        ),
        Err(InvocationError::Limit)
    );
    assert_eq!(
        o.invocation_observation_wait_usage_v1(&v, &mut r)
            .unwrap()
            .waiting,
        0
    );
    assert_eq!(
        o.minimal_invocation_control_status_v1(&mut v, &p, &f, &mut r)
            .unwrap()
            .phase,
        InvocationObservationPhaseV1::Terminal
    );
    o.retire_invocation_observation_wait_v1(&mut v, &wait, &control(&c), &mut r)
        .unwrap();
    o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
        .unwrap();
}

#[test]
fn admission_requires_owned_ordinary_reference_finite_budget_and_exact_wait_facts() {
    let (mut o, id, c, p, mut v, mut r) = setup_wait(1);
    let f = release(&c);
    let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &f,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
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
    for gate in 0..3 {
        for value in [None, Some(false)] {
            let mut wf = wait_facts(&ordinary);
            match gate {
                0 => wf.wait_capacity = value,
                1 => wf.wait_clock_valid = value,
                _ => wf.retention_ready = value,
            }
            assert_eq!(
                o.start_invocation_observation_wait_v1(&mut v, request(&after, 2), &p, &wf, &mut r),
                Err(InvocationError::Unavailable)
            );
            assert_eq!(
                o.invocation_observation_wait_usage_v1(&v, &mut r)
                    .unwrap()
                    .retained,
                0
            );
        }
    }
    let wf = wait_facts(&ordinary);
    for budget in [0, 4, usize::MAX] {
        assert_eq!(
            o.start_invocation_observation_wait_v1(
                &mut v,
                request(&after, budget),
                &p,
                &wf,
                &mut r
            ),
            Err(InvocationError::Limit)
        );
    }
    // A separate actual context on the same invocation has a different origin.
    let mut l = limits();
    l.max_issued = 1;
    l.max_held_bytes = 16384;
    let mut other = o
        .open_invocation_observation_v1(&id, &c, &p, l, &facts(&c), &mut r)
        .unwrap();
    let foreign = o
        .query_invocation_observation_v1(
            &mut other,
            InvocationObservationQueryV1::Status,
            &p,
            &ordinary,
            &mut r,
        )
        .unwrap()
        .reference;
    assert_eq!(
        o.start_invocation_observation_wait_v1(&mut v, request(&foreign, 2), &p, &wf, &mut r),
        Err(InvocationError::Unavailable)
    );
    let wait = o
        .start_invocation_observation_wait_v1(&mut v, request(&after, 2), &p, &wf, &mut r)
        .unwrap();
    assert_eq!(
        o.advance_invocation_observation_wait_v1(
            &mut other,
            &wait,
            InvocationWaitEventV1::Stop,
            &p,
            &wf,
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    let foreign_owner = owner();
    assert_eq!(
        foreign_owner.advance_invocation_observation_wait_v1(
            &mut v,
            &wait,
            InvocationWaitEventV1::Stop,
            &p,
            &wf,
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        o.invocation_observation_wait_usage_v1(&v, &mut r)
            .unwrap()
            .waiting,
        1
    );
}

#[test]
fn revocation_or_owner_close_ends_all_waiters_but_preserves_each_retained_row() {
    for trigger in 0..4 {
        let (o, id, c, p, mut v, mut r) = setup_wait(2);
        let f = release(&c);
        let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
            current: &f,
            ordinary_ready: Some(true),
            resources_ready: Some(true),
        };
        let wf = wait_facts(&ordinary);
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
        let a = o
            .start_invocation_observation_wait_v1(&mut v, request(&after, 3), &p, &wf, &mut r)
            .unwrap();
        let b = o
            .start_invocation_observation_wait_v1(&mut v, request(&after, 3), &p, &wf, &mut r)
            .unwrap();
        let mut revoked = p.clone();
        revoked.grant = false;
        match trigger {
            0 => assert_eq!(
                o.query_invocation_observation_v1(
                    &mut v,
                    InvocationObservationQueryV1::Status,
                    &revoked,
                    &ordinary,
                    &mut r
                ),
                Err(InvocationError::Unavailable)
            ),
            1 => assert_eq!(
                o.minimal_invocation_control_status_v1(&mut v, &revoked, &f, &mut r),
                Err(InvocationError::Unavailable)
            ),
            2 => assert_eq!(
                o.advance_invocation_observation_wait_v1(
                    &mut v,
                    &a,
                    InvocationWaitEventV1::Tick(InvocationWaitClockV1 { epoch: 7, tick: 1 }),
                    &revoked,
                    &wf,
                    &mut r
                ),
                Err(InvocationError::Unavailable)
            ),
            _ => o
                .close_invocation_observation_v1(&mut v, &control(&c), &mut r)
                .unwrap(),
        }
        assert_eq!(
            o.invocation_observation_wait_usage_v1(&v, &mut r).unwrap(),
            InvocationWaitUsageV1 {
                retained: 2,
                waiting: 0,
                max_waiters: 2
            }
        );
        for wait in [&a, &b] {
            assert_eq!(
                o.advance_invocation_observation_wait_v1(
                    &mut v,
                    wait,
                    InvocationWaitEventV1::Poll,
                    &p,
                    &wf,
                    &mut r
                ),
                Err(InvocationError::Unavailable)
            );
            o.retire_invocation_observation_wait_v1(&mut v, wait, &control(&c), &mut r)
                .unwrap();
        }
        assert_eq!(
            o.invocation_state(&id, &mut r),
            Ok(InvocationState::Pending)
        );
    }
}

#[test]
fn protected_stop_expiry_and_retirement_need_no_ordinary_capacity_or_disclosure() {
    for event in [InvocationWaitEventV1::Stop, InvocationWaitEventV1::Expire] {
        let (o, id, c, p, mut v, mut r) = setup_wait(1);
        let f = release(&c);
        let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
            current: &f,
            ordinary_ready: Some(true),
            resources_ready: Some(true),
        };
        let wf = wait_facts(&ordinary);
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
            .start_invocation_observation_wait_v1(&mut v, request(&after, 3), &p, &wf, &mut r)
            .unwrap();
        let denied = TrustedInvocationOrdinaryObservationReleaseV1 {
            current: &f,
            ordinary_ready: Some(false),
            resources_ready: Some(false),
        };
        let mut wf = wait_facts(&denied);
        let mut revoked = p.clone();
        revoked.grant = false;
        for owner in [None, Some(false)] {
            wf.owner_control_allowed = owner;
            assert_eq!(
                o.advance_invocation_observation_wait_v1(
                    &mut v, &wait, event, &revoked, &wf, &mut r
                ),
                Err(InvocationError::Unavailable)
            );
            assert_eq!(
                o.invocation_observation_wait_usage_v1(&v, &mut r)
                    .unwrap()
                    .waiting,
                1
            );
        }
        wf.owner_control_allowed = Some(true);
        assert_eq!(
            o.advance_invocation_observation_wait_v1(&mut v, &wait, event, &revoked, &wf, &mut r),
            Ok(InvocationWaitReplyV1::Ended)
        );
        assert_eq!(
            o.minimal_invocation_control_status_v1(&mut v, &p, &f, &mut r)
                .unwrap()
                .phase,
            InvocationObservationPhaseV1::Pending
        );
        for gate in 0..3 {
            for value in [None, Some(false)] {
                let mut control = control(&c);
                match gate {
                    0 => control.owner_control_allowed = value,
                    1 => control.retention_released = value,
                    _ => control.delivery_drained = value,
                }
                assert_eq!(
                    o.retire_invocation_observation_wait_v1(&mut v, &wait, &control, &mut r),
                    Err(InvocationError::Unavailable)
                );
            }
        }
        o.retire_invocation_observation_wait_v1(&mut v, &wait, &control(&c), &mut r)
            .unwrap();
        assert_eq!(
            o.retire_invocation_observation_wait_v1(&mut v, &wait, &control(&c), &mut r),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(
            o.invocation_state(&id, &mut r),
            Ok(InvocationState::Pending)
        );
    }
}

#[test]
fn an_unavailable_poll_ends_only_its_wait_and_a_mismatched_context_cannot_consume_it() {
    let (o, _id, c, p, mut v, mut r) = setup_wait(2);
    let f = release(&c);
    let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &f,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    let wf = wait_facts(&ordinary);
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
    let a = o
        .start_invocation_observation_wait_v1(&mut v, request(&after, 3), &p, &wf, &mut r)
        .unwrap();
    let b = o
        .start_invocation_observation_wait_v1(&mut v, request(&after, 3), &p, &wf, &mut r)
        .unwrap();
    let mut wrong = c.clone();
    wrong.purpose.push('!');
    let wrong_f = release(&wrong);
    let wrong_ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &wrong_f,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    let wrong_wf = wait_facts(&wrong_ordinary);
    assert_eq!(
        o.advance_invocation_observation_wait_v1(
            &mut v,
            &a,
            InvocationWaitEventV1::Stop,
            &p,
            &wrong_wf,
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        o.invocation_observation_wait_usage_v1(&v, &mut r)
            .unwrap()
            .waiting,
        2
    );
    let mut unknown = release(&c);
    unknown.authorized = None;
    let unknown_ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &unknown,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    let unknown_wf = wait_facts(&unknown_ordinary);
    assert_eq!(
        o.advance_invocation_observation_wait_v1(
            &mut v,
            &a,
            InvocationWaitEventV1::Poll,
            &p,
            &unknown_wf,
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        o.invocation_observation_wait_usage_v1(&v, &mut r)
            .unwrap()
            .waiting,
        1
    );
    assert_eq!(
        o.advance_invocation_observation_wait_v1(
            &mut v,
            &b,
            InvocationWaitEventV1::Poll,
            &p,
            &wf,
            &mut r
        ),
        Ok(InvocationWaitReplyV1::Waiting(b.clone()))
    );
}

#[test]
fn native_wait_record_size_is_checked_before_allocating_or_holding_a_context() {
    for delta in [-1isize, 0, 1] {
        let (mut o, id, c, p, mut r) = pending();
        let mut l = limits();
        l.max_waiters = 1;
        l.max_wait_steps = 1;
        l.max_issued = 1;
        l.max_held_bytes = 16384;
        l.max_wait_bytes = InvocationObservationLimitsV1::required_wait_bytes()
            .checked_add_signed(delta)
            .unwrap();
        let result = o.open_invocation_observation_v1(&id, &c, &p, l, &facts(&c), &mut r);
        if delta < 0 {
            assert_eq!(result.unwrap_err(), InvocationError::Limit);
        } else {
            o.retire_invocation_observation_v1(&mut result.unwrap(), &control(&c), &mut r)
                .unwrap();
        }
        assert_eq!(o.retention_usage().reserved, 2048);
    }
}

#[test]
fn wait_instrumentation_is_actual_optional_bounded_and_does_not_change_cutoff() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for mode in 0..4 {
        let (o, id, c, p, mut v, mut r) = setup_wait(1);
        let f = release(&c);
        let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
            current: &f,
            ordinary_ready: Some(true),
            resources_ready: Some(true),
        };
        let wf = wait_facts(&ordinary);
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
        r = if mode == 0 {
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
        let wait = o
            .start_invocation_observation_wait_v1(&mut v, request(&after, 1), &p, &wf, &mut r)
            .unwrap();
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
        assert_eq!(
            o.advance_invocation_observation_wait_v1(
                &mut v,
                &wait,
                InvocationWaitEventV1::Tick(InvocationWaitClockV1 { epoch: 7, tick: 1 }),
                &p,
                &wf,
                &mut r
            ),
            Ok(InvocationWaitReplyV1::Ended)
        );
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
        assert_eq!(
            o.invocation_observation_wait_usage_v1(&v, &mut r)
                .unwrap()
                .waiting,
            0
        );
        o.retire_invocation_observation_wait_v1(&mut v, &wait, &control(&c), &mut r)
            .unwrap();
        assert_eq!(
            o.invocation_state(&id, &mut Recorder::disabled()),
            Ok(InvocationState::Pending)
        );
        let mut counts = [0; 4];
        let mut refused = 0;
        while let Some(e) = r.pop_front() {
            let i = match e.boundary {
                Boundary::InvocationWaitAdmission => Some(0),
                Boundary::InvocationWaitService => Some(1),
                Boundary::InvocationWaitRetirement => Some(2),
                Boundary::InvocationWaitUsage => Some(3),
                _ => None,
            };
            if let Some(i) = i {
                counts[i] += 1;
                refused += usize::from(e.disposition == Disposition::Refused);
                assert_eq!(e.service_time.is_some(), mode == 2);
            }
            assert!(!format!("{e:?}").contains("native-observation/1"));
        }
        assert_eq!(
            counts,
            if mode == 1 || mode == 2 {
                [1, 3, 1, 1]
            } else {
                [0; 4]
            }
        );
        if mode == 1 || mode == 2 {
            assert_eq!(refused, 1);
        }
        if mode == 3 {
            assert!(r.dropped() > 0);
        }
    }
}
