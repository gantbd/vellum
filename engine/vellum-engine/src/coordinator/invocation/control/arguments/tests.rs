use super::super::tests::{control, queued};
use super::*;

pub(in crate::coordinator) fn request<T>(
    id: &InvocationId,
    typed: bool,
    consume: impl FnOnce(&InvocationCancellationEnvelopeValueV1<'_, '_>) -> T,
) -> T {
    use InvocationCancellationEnvelopeValueV1 as V;
    let args = [
        (
            "contract",
            V::Data(InertValueV1::Text("invocation-control/1")),
        ),
        (
            "operation",
            V::Data(InertValueV1::Text("request-cancellation")),
        ),
        ("invocation", V::Invocation(id)),
    ];
    let header = [
        if typed {
            ("family", V::Data(InertValueV1::Number(17.0)))
        } else {
            ("name", V::Data(InertValueV1::Text("control-execution")))
        },
        (
            "variant",
            V::Data(InertValueV1::Text("request-cancellation")),
        ),
        ("version", V::Data(InertValueV1::Number(1.0))),
        ("arguments", V::Object(&args)),
    ];
    consume(&V::Object(&[
        (
            "route",
            V::Data(InertValueV1::Text(if typed { "typed" } else { "dynamic" })),
        ),
        ("request", V::Object(&header)),
    ]))
}

#[test]
fn decoded_cancellation_rechecks_authority_then_requests_intent_without_settlement() {
    for typed in [false, true] {
        for kind in 0..4 {
            let (mut o, mut r, c, ticket) = queued(kind);
            let generation = o.generation();
            let usage = o.retention_usage();
            let mut f = control(&c);
            f.owner_control_allowed = Some(false);
            request(&ticket, typed, |e| {
                assert_eq!(
                    o.request_decoded_invocation_cancellation_v1(e, &f, &mut r),
                    Err(InvocationCancellationIngressErrorV1::Execution(
                        InvocationError::Unavailable
                    ))
                );
            });
            assert!(
                !o.invocations.slots[ticket.slot]
                    .as_ref()
                    .unwrap()
                    .stop_requested
            );
            f.owner_control_allowed = Some(true);
            for _ in 0..2 {
                request(&ticket, typed, |e| {
                    assert_eq!(
                        o.request_decoded_invocation_cancellation_v1(e, &f, &mut r),
                        Ok(())
                    );
                });
                assert_eq!(
                    o.invocation_state(&ticket, &mut r),
                    Ok(InvocationState::Pending)
                );
                assert_eq!(o.generation(), generation);
                assert_eq!(o.retention_usage(), usage);
            }
            assert_eq!(o.run(&ticket, &mut r), Ok(InvocationState::Finished));
            assert!(matches!(
                o.invocation_outcome(&ticket, &mut r),
                Ok(InvocationOutcome::StoppedBeforePublication)
            ));
        }
    }
}

#[test]
fn malformed_cancellation_never_reaches_native_control_or_decodes_an_identity() {
    use InvocationCancellationEnvelopeValueV1 as V;
    for typed in [false, true] {
        for case in 0..22 {
            let (mut o, mut setup, c, ticket) = queued(0);
            let generation = o.generation();
            let debt = o.retention_usage();
            let mut r = Recorder::try_with_capacity(32).unwrap();
            request(&ticket, typed, |valid| {
                let V::Object(rows) = valid else {
                    unreachable!()
                };
                let mut outer = rows.to_vec();
                let V::Object(rows) = outer[1].1 else {
                    unreachable!()
                };
                let mut header = rows.to_vec();
                let V::Object(rows) = header[3].1 else {
                    unreachable!()
                };
                let mut args = rows.to_vec();
                match case {
                    0 => args[0].1 = V::Data(InertValueV1::Text("invocation-control/2")),
                    1 => args[1].1 = V::Data(InertValueV1::Text("stop-observation")),
                    2 => args[2].1 = V::Data(InertValueV1::Text("forged-ticket")),
                    3 => args[2].1 = V::Data(InertValueV1::Null),
                    4 => args[2].1 = V::Object(&[]),
                    5 => args[1].0 = "contract",
                    6 => args[2].0 = "context",
                    7 => {
                        args.pop();
                    }
                    8 => args.push(("extra", V::Data(InertValueV1::Null))),
                    _ => (),
                }
                header[3].1 = V::Object(&args);
                match case {
                    9 => header[0].1 = V::Data(InertValueV1::Text("17")),
                    10 => header[1].1 = V::Data(InertValueV1::Text("shutdown-disposition")),
                    11 => header[2].1 = V::Data(InertValueV1::Number(2.0)),
                    12 => header[2].1 = V::Data(InertValueV1::Number(f64::NAN)),
                    13 => header[3].1 = V::Data(InertValueV1::Object(&[])),
                    14 => header[2].0 = "variant",
                    15 => {
                        header.pop();
                    }
                    16 => header.push(("authority", V::Data(InertValueV1::Boolean(true)))),
                    _ => (),
                }
                outer[1].1 = V::Object(&header);
                match case {
                    17 => outer[0].1 = V::Data(InertValueV1::Text("http")),
                    18 => outer[1].0 = "route",
                    19 => {
                        outer.pop();
                    }
                    20 => outer.push(("extra", V::Data(InertValueV1::Null))),
                    21 => outer[1].1 = V::Data(InertValueV1::Object(&[])),
                    _ => (),
                }
                assert_eq!(
                    o.request_decoded_invocation_cancellation_v1(
                        &V::Object(&outer),
                        &control(&c),
                        &mut r
                    ),
                    Err(InvocationCancellationIngressErrorV1::InvalidRequest),
                    "case {case}"
                );
            });
            let event = r.pop_front().unwrap();
            assert_eq!(
                event.boundary,
                Boundary::InvocationCancellationArgumentNormalization
            );
            assert_eq!(event.disposition, Disposition::Refused);
            assert!(
                r.pop_front().is_none(),
                "no fabricated native-control service"
            );
            assert!(
                !o.invocations.slots[ticket.slot]
                    .as_ref()
                    .unwrap()
                    .stop_requested
            );
            assert_eq!(
                o.invocation_state(&ticket, &mut setup),
                Ok(InvocationState::Pending)
            );
            assert_eq!(o.generation(), generation);
            assert_eq!(o.retention_usage(), debt);
        }
    }
}

#[test]
fn decoded_cancellation_does_not_turn_ticket_possession_into_current_control_authority() {
    for gate in 0..4 {
        for denied in [None, Some(false)] {
            let (mut o, mut r, c, ticket) = queued(0);
            let mut f = control(&c);
            match gate {
                0 => f.owner_control_allowed = denied,
                1 => f.control_context_current = denied,
                2 => f.control_scope_valid = denied,
                3 => f.protected_control_ready = denied,
                _ => unreachable!(),
            }
            request(&ticket, true, |e| {
                assert_eq!(
                    o.request_decoded_invocation_cancellation_v1(e, &f, &mut r),
                    Err(InvocationCancellationIngressErrorV1::Execution(
                        InvocationError::Unavailable
                    ))
                )
            });
            assert!(
                !o.invocations.slots[ticket.slot]
                    .as_ref()
                    .unwrap()
                    .stop_requested
            );
        }
    }
    let (mut o, mut r, c, ticket) = queued(0);
    let mut wrong = c.clone();
    wrong.profile.push_str("-other");
    request(&ticket, false, |e| {
        assert_eq!(
            o.request_decoded_invocation_cancellation_v1(e, &control(&wrong), &mut r),
            Err(InvocationCancellationIngressErrorV1::Execution(
                InvocationError::Unavailable
            ))
        )
    });
    let (mut foreign, mut foreign_r, _, _) = queued(0);
    request(&ticket, true, |e| {
        assert_eq!(
            foreign.request_decoded_invocation_cancellation_v1(e, &control(&c), &mut foreign_r),
            Err(InvocationCancellationIngressErrorV1::Execution(
                InvocationError::Unavailable
            ))
        )
    });
    assert!(
        !o.invocations.slots[ticket.slot]
            .as_ref()
            .unwrap()
            .stop_requested
    );
}
