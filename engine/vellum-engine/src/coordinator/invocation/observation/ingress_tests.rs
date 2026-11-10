use super::*;
use crate::capture::InertValueV1 as D;
use crate::instrumentation::NativeTimingProfileV1;

#[test]
fn malformed_or_foreign_observation_input_cannot_apply_current_loss_to_a_valid_context() {
    use InvocationObservationEnvelopeValueV1 as V;
    for typed in [false, true] {
        let (o, id, c, p, mut v, mut setup) = tests::setup();
        let (_, foreign, _, _, other, _) = tests::setup();
        let context = v.context_id_v1();
        let foreign_context = other.context_id_v1();
        let denied = TrustedInvocationObservationReleaseV1 {
            authorized: Some(false),
            ..release(&c)
        };
        let mut r = Recorder::try_with_capacity(64)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let before = o.retention_usage();
        for case in 0..15 {
            request(
                &id,
                &context,
                &c.profile,
                typed,
                "minimal-control-status",
                None,
                0.0,
                |e| {
                    let V::Object(outer) = e else { unreachable!() };
                    let mut outer = outer.to_vec();
                    let V::Object(header) = outer[1].1 else {
                        unreachable!()
                    };
                    let mut header = header.to_vec();
                    let V::Object(args) = header[3].1 else {
                        unreachable!()
                    };
                    let mut args = args.to_vec();
                    match case {
                        6 => args[0].1 = V::Data(D::Text("execution-observation/2")),
                        7 => args[1].1 = V::Data(D::Text("status")),
                        8 => args[2].1 = V::Invocation(&foreign),
                        9 => args[3].1 = V::Context(&foreign_context),
                        10 => args[4].1 = V::Data(D::Text("foreign")),
                        11 => args[0].0 = "unknown",
                        12 => args[0].0 = "profile",
                        13 => args.push(("after", V::Context(&context))),
                        _ => (),
                    }
                    header[3].1 = V::Object(&args);
                    match case {
                        3 => header[0].1 = V::Data(D::Text("wrong-family")),
                        4 => header[2].1 = V::Data(D::Number(2.0)),
                        5 => header[1].1 = V::Data(D::Text("unknown")),
                        _ => (),
                    }
                    outer[1].1 = V::Object(&header);
                    match case {
                        0 => {
                            outer.pop();
                        }
                        1 => outer[0].0 = "request",
                        2 => outer[0].1 = V::Data(D::Text("unknown")),
                        14 => outer[1].1 = V::Data(D::Null),
                        _ => (),
                    }
                    assert!(
                        o.query_decoded_invocation_observation_v1(
                            &mut v,
                            &V::Object(&outer),
                            InvocationObservationReadPortV1::MinimalControl {
                                current: &p,
                                facts: &denied
                            },
                            &mut r
                        )
                        .is_err()
                    );
                    assert!(v.live);
                },
            );
        }
        let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
            current: &denied,
            ordinary_ready: Some(true),
            resources_ready: Some(true),
        };
        assert!(
            request(
                &id,
                &context,
                &c.profile,
                typed,
                "minimal-control-status",
                None,
                0.0,
                |e| o.query_decoded_invocation_observation_v1(
                    &mut v,
                    e,
                    InvocationObservationReadPortV1::Ordinary {
                        current: &p,
                        facts: &ordinary
                    },
                    &mut r
                )
            )
            .is_err()
        );
        assert!(v.live);
        while let Some(event) = r.pop_front() {
            assert_ne!(event.boundary, Boundary::InvocationControlObservation);
            assert_ne!(event.boundary, Boundary::InvocationObservationQuery);
            assert_eq!(event.disposition, Disposition::Refused);
        }
        let valid = release(&c);
        assert!(
            request(
                &id,
                &context,
                &c.profile,
                typed,
                "minimal-control-status",
                None,
                0.0,
                |e| o.query_decoded_invocation_observation_v1(
                    &mut v,
                    e,
                    InvocationObservationReadPortV1::MinimalControl {
                        current: &p,
                        facts: &valid
                    },
                    &mut setup
                )
            )
            .is_ok()
        );
        assert!(
            request(
                &id,
                &context,
                &c.profile,
                typed,
                "minimal-control-status",
                None,
                0.0,
                |e| o.query_decoded_invocation_observation_v1(
                    &mut v,
                    e,
                    InvocationObservationReadPortV1::MinimalControl {
                        current: &p,
                        facts: &denied
                    },
                    &mut setup
                )
            )
            .is_err()
        );
        assert!(!v.live);
        assert_eq!(o.retention_usage(), before);
        assert_eq!(
            o.invocation_state(&id, &mut setup),
            Ok(InvocationState::Pending)
        );
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
#[allow(clippy::too_many_arguments)] // Explicit native scenario inputs, not an SDK request builder.
pub(in crate::coordinator) fn request<T>(
    id: &InvocationId,
    context: &InvocationObservationContextIdV1,
    profile: &str,
    typed: bool,
    op: &str,
    wait: Option<InvocationObservationEnvelopeValueV1<'_, '_>>,
    budget: f64,
    use_it: impl FnOnce(&InvocationObservationEnvelopeValueV1<'_, '_>) -> T,
) -> T {
    use InvocationObservationEnvelopeValueV1 as V;
    let mut args = vec![
        ("contract", V::Data(D::Text("execution-observation/1"))),
        ("operation", V::Data(D::Text(op))),
        ("invocation", V::Invocation(id)),
        ("context", V::Context(context)),
        ("profile", V::Data(D::Text(profile))),
    ];
    if let Some(after) = wait {
        args.push(("after", after));
        args.push(("budget", V::Data(D::Number(budget))));
    }
    let header = [
        (
            if typed { "family" } else { "name" },
            if typed {
                V::Data(D::Number(5.0))
            } else {
                V::Data(D::Text("inspect-execution"))
            },
        ),
        ("variant", V::Data(D::Text(op))),
        ("version", V::Data(D::Number(1.0))),
        ("arguments", V::Object(&args)),
    ];
    use_it(&V::Object(&[
        (
            "route",
            V::Data(D::Text(if typed { "typed" } else { "dynamic" })),
        ),
        ("request", V::Object(&header)),
    ]))
}

#[test]
fn four_observation_ingress_forms_preserve_wait_producer_and_protected_lifetimes() {
    for typed in [false, true] {
        for mode in 0..4 {
            let (mut o, id, c, p, mut old, mut setup) = tests::setup();
            let mut l = old.limits;
            l.max_waiters = 1;
            l.max_issued = 4;
            l.max_wait_steps = 4;
            l.max_held_bytes = 16384;
            o.retire_invocation_observation_v1(&mut old, &control(&c), &mut setup)
                .unwrap();
            let facts = TrustedInvocationPolicyFactsV1 {
                context: &c,
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
            };
            let mut v = o
                .open_invocation_observation_v1(&id, &c, &p, l, &facts, &mut setup)
                .unwrap();
            let context = v.context_id_v1();
            let f = release(&c);
            let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
                current: &f,
                ordinary_ready: Some(true),
                resources_ready: Some(true),
            };
            let unavailable = TrustedInvocationOrdinaryObservationReleaseV1 {
                current: &f,
                ordinary_ready: Some(false),
                resources_ready: Some(false),
            };
            let wf = TrustedInvocationWaitFactsV1 {
                observation: &ordinary,
                wait_capacity: Some(true),
                wait_clock_valid: Some(true),
                retention_ready: Some(true),
                owner_control_allowed: Some(true),
            };
            let clock = InvocationWaitClockV1 { epoch: 9, tick: 0 };
            let mut r = if mode == 0 {
                Recorder::disabled()
            } else {
                Recorder::try_with_capacity(if mode == 3 { 1 } else { 256 }).unwrap()
            };
            if mode >= 2 {
                r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            }
            if mode == 3 {
                r.record(Observation {
                    boundary: Boundary::Query,
                    disposition: Disposition::Completed,
                    service_time: None,
                });
            }
            let before = o.retention_usage();
            let status = request(&id, &context, &c.profile, typed, "status", None, 0.0, |e| {
                o.query_decoded_invocation_observation_v1(
                    &mut v,
                    e,
                    InvocationObservationReadPortV1::Ordinary {
                        current: &p,
                        facts: &ordinary,
                    },
                    &mut r,
                )
            })
            .unwrap();
            let InvocationObservationReadReplyV1::Ordinary(status) = status else {
                panic!("wrong status family")
            };
            assert_eq!(status.phase, Some(InvocationObservationPhaseV1::Pending));
            // Ordinary readiness failure must not disable separately authorized minimal control.
            assert!(
                request(&id, &context, &c.profile, typed, "status", None, 0.0, |e| o
                    .query_decoded_invocation_observation_v1(
                        &mut v,
                        e,
                        InvocationObservationReadPortV1::Ordinary {
                            current: &p,
                            facts: &unavailable
                        },
                        &mut r
                    ))
                .is_err()
            );
            assert!(matches!(
                request(
                    &id,
                    &context,
                    &c.profile,
                    typed,
                    "minimal-control-status",
                    None,
                    0.0,
                    |e| o.query_decoded_invocation_observation_v1(
                        &mut v,
                        e,
                        InvocationObservationReadPortV1::MinimalControl {
                            current: &p,
                            facts: &f
                        },
                        &mut r
                    )
                )
                .unwrap(),
                InvocationObservationReadReplyV1::MinimalControl(_)
            ));
            for budget in [
                0.0,
                -1.0,
                0.5,
                f64::NAN,
                f64::INFINITY,
                9_007_199_254_740_992.0,
                5.0,
            ] {
                assert!(
                    request(
                        &id,
                        &context,
                        &c.profile,
                        typed,
                        "bounded-observation-wait",
                        Some(InvocationObservationEnvelopeValueV1::After(
                            &status.reference
                        )),
                        budget,
                        |e| o.query_decoded_invocation_observation_v1(
                            &mut v,
                            e,
                            InvocationObservationReadPortV1::Wait {
                                current: &p,
                                facts: &wf,
                                clock
                            },
                            &mut r
                        )
                    )
                    .is_err()
                );
            }
            assert!(
                request(
                    &id,
                    &context,
                    &c.profile,
                    typed,
                    "bounded-observation-wait",
                    Some(InvocationObservationEnvelopeValueV1::Context(&context)),
                    2.0,
                    |e| o.query_decoded_invocation_observation_v1(
                        &mut v,
                        e,
                        InvocationObservationReadPortV1::Wait {
                            current: &p,
                            facts: &wf,
                            clock
                        },
                        &mut r
                    )
                )
                .is_err()
            );
            let wait = request(
                &id,
                &context,
                &c.profile,
                typed,
                "bounded-observation-wait",
                Some(InvocationObservationEnvelopeValueV1::After(
                    &status.reference,
                )),
                2.0,
                |e| {
                    o.query_decoded_invocation_observation_v1(
                        &mut v,
                        e,
                        InvocationObservationReadPortV1::Wait {
                            current: &p,
                            facts: &wf,
                            clock,
                        },
                        &mut r,
                    )
                },
            )
            .unwrap();
            let InvocationObservationReadReplyV1::Waiting(wait) = wait else {
                panic!("wrong wait family")
            };
            assert_eq!(
                o.invocation_state(&id, &mut setup),
                Ok(InvocationState::Pending)
            );
            assert_eq!(
                o.advance_invocation_observation_wait_v1(
                    &mut v,
                    &wait,
                    InvocationWaitEventV1::Poll,
                    &p,
                    &wf,
                    &mut r
                )
                .unwrap(),
                InvocationWaitReplyV1::Waiting(wait.clone())
            );
            o.run(&id, &mut setup).unwrap();
            let generation = o.generation();
            assert!(matches!(
                o.advance_invocation_observation_wait_v1(
                    &mut v,
                    &wait,
                    InvocationWaitEventV1::Poll,
                    &p,
                    &wf,
                    &mut r
                )
                .unwrap(),
                InvocationWaitReplyV1::Observed(_)
            ));
            assert!(matches!(
                request(
                    &id,
                    &context,
                    &c.profile,
                    typed,
                    "retained-outcome",
                    None,
                    0.0,
                    |e| o.query_decoded_invocation_observation_v1(
                        &mut v,
                        e,
                        InvocationObservationReadPortV1::Ordinary {
                            current: &p,
                            facts: &ordinary
                        },
                        &mut r
                    )
                )
                .unwrap(),
                InvocationObservationReadReplyV1::Ordinary(InvocationObservationReplyV1 {
                    outcome: Some(_),
                    ..
                })
            ));
            assert_eq!(o.generation(), generation);
            assert_eq!(o.retention_usage(), before);
            o.retire_invocation_observation_wait_v1(&mut v, &wait, &control(&c), &mut r)
                .unwrap();
            o.close_invocation_observation_v1(&mut v, &control(&c), &mut r)
                .unwrap();
            o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
                .unwrap();
            assert_eq!(
                o.retention_usage().reserved,
                before.reserved - l.required_hold_bytes().unwrap()
            );
            let mut accepted = 0;
            while let Some(event) = r.pop_front() {
                if event.boundary == Boundary::InvocationObservationIngress {
                    assert_eq!(event.service_time.is_some(), mode == 2);
                    accepted += usize::from(event.disposition == Disposition::Completed);
                }
            }
            if mode == 1 || mode == 2 {
                assert_eq!(accepted, 4);
            }
            if mode == 3 {
                assert!(r.dropped() > 0);
            }
            if mode == 0 {
                assert_eq!(r.dropped(), 0);
            }
        }
    }
}
