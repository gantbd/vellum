use super::super::{facet_receipt_test_facts, facet_receipt_test_open, facet_receipt_test_setup};
use super::*;

#[test]
fn decoded_facet_commands_execute_resync_and_replay_then_release_separately_authorized_original_receipt()
 {
    for typed in [false, true] {
        let (mut o, mut r, s, original) = facet_receipt_test_setup();
        let stream = o
            .facet_publication_command_facts_v1(&original, &s, &mut r)
            .unwrap()
            .as_observed
            .unwrap()
            .as_ref()
            .unwrap()
            .stream
            .clone();
        let original_op = FacetPublicationCommandOperationV1::Establish;
        let original_q = FacetPublicationCommandRequestV1 {
            scope: &s,
            operation: &original_op,
            current: None,
            owner_control_allowed: Some(true),
            max_reply_bytes: 65536,
        };
        command_envelope(typed, &original, None, |e| {
            assert!(matches!(
                o.request_decoded_facet_publication_command_v1(
                    e,
                    &FacetPublicationCommandPortV1 {
                        command: &original,
                        request: &original_q
                    },
                    &mut r
                ),
                Ok(FacetPublicationCommandReplyV1::ObserveExisting { .. })
            ))
        });
        let n = crate::coordinator::publication_stream::tests::native(&s.source);
        let g = crate::coordinator::facet_publication::tests::grants(&s);
        let v: Vec<_> = s.views.iter().map(|v| v.view.clone()).collect();
        let f = crate::coordinator::facet_publication::tests::facts(&s, &n, &g, &v);
        let op = FacetPublicationCommandOperationV1::Resynchronize {
            stream: stream.clone(),
        };
        let command = o
            .issue_facet_publication_command_v1(&op, &s, &f, Some(true), &mut r)
            .unwrap();
        let mut q = FacetPublicationCommandRequestV1 {
            scope: &s,
            operation: &op,
            current: Some(&f),
            owner_control_allowed: Some(true),
            max_reply_bytes: 65536,
        };
        command_envelope(typed, &command, Some(&stream), |e| {
            assert!(matches!(
                o.request_decoded_facet_publication_command_v1(
                    e,
                    &FacetPublicationCommandPortV1 {
                        command: &command,
                        request: &q
                    },
                    &mut r
                ),
                Ok(FacetPublicationCommandReplyV1::Settled { .. })
            ))
        });
        let before = o.facet_publication_usage_v1().unwrap();
        q.current = None;
        command_envelope(typed, &command, Some(&stream), |e| {
            assert!(matches!(
                o.request_decoded_facet_publication_command_v1(
                    e,
                    &FacetPublicationCommandPortV1 {
                        command: &command,
                        request: &q
                    },
                    &mut r
                ),
                Ok(FacetPublicationCommandReplyV1::ObserveExisting { .. })
            ))
        });
        assert_eq!(o.facet_publication_usage_v1().unwrap(), before);
        for command in [&original, &command] {
            let context = facet_receipt_test_open(&mut o, command, &s, &mut r);
            let basis = o
                .facet_publication_receipt_basis_v1(&context, command, &s, &mut r)
                .unwrap()
                .basis;
            let f = facet_receipt_test_facts(&s);
            let current = TrustedFacetPublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &f,
            };
            let port = FacetPublicationReceiptPortV1 {
                command,
                context: &context,
                scope: &s,
                current: &current,
            };
            let reply = receipt_envelope(typed, command, &context, |e| {
                o.release_decoded_facet_publication_receipt_v1(e, &port, &mut r)
            })
            .unwrap();
            assert_eq!(&reply.command, command);
            assert!(Arc::ptr_eq(
                &reply.result.unwrap(),
                o.facet_publication_command_facts_v1(command, &s, &mut r)
                    .unwrap()
                    .as_observed
                    .unwrap()
                    .as_ref()
                    .unwrap()
            ));
        }
    }
}

#[test]
fn decoded_facet_receipt_malformed_or_mismatched_roles_do_not_reach_native_release_or_apply_loss() {
    for typed in [false, true] {
        let (mut o, mut setup, s, command) = facet_receipt_test_setup();
        let one = facet_receipt_test_open(&mut o, &command, &s, &mut setup);
        let two = facet_receipt_test_open(&mut o, &command, &s, &mut setup);
        let basis = o
            .facet_publication_receipt_basis_v1(&two, &command, &s, &mut setup)
            .unwrap()
            .basis;
        let mut f = facet_receipt_test_facts(&s);
        f.complete_result_allowed = Some(false);
        let current = TrustedFacetPublicationReceiptHistoricalFactsV1 {
            basis: &basis,
            current: &f,
        };
        let port = FacetPublicationReceiptPortV1 {
            command: &command,
            context: &two,
            scope: &s,
            current: &current,
        };
        let mut r = Recorder::try_with_capacity(64).unwrap();
        receipt_envelope(typed, &command, &one, |e| {
            assert!(
                o.release_decoded_facet_publication_receipt_v1(e, &port, &mut r)
                    .is_err()
            )
        });
        let facet = [("kind", V::Data(InertValueV1::Text("named-facet-baseline")))];
        for case in 0..7 {
            let mut args = vec![
                (
                    "contract",
                    V::Data(InertValueV1::Text(FACET_PUBLICATION_RECEIPT_PROFILE_V1)),
                ),
                ("operation", V::Data(InertValueV1::Text("retained-outcome"))),
                ("command", V::Command(&command)),
                ("context", V::ReceiptContext(&two)),
                ("facet", V::Object(&facet)),
            ];
            match case {
                0 => args[0].1 = V::Data(InertValueV1::Text("source-publication-receipt/1")),
                1 => args[2].1 = V::Data(InertValueV1::Text("forged-command")),
                2 => args[3].1 = V::Command(&command),
                3 => args[4].1 = V::Data(InertValueV1::Null),
                4 => args[3].0 = "command",
                5 => {
                    args.pop();
                }
                6 => args.push(("authority", V::Data(InertValueV1::Boolean(true)))),
                _ => unreachable!(),
            }
            envelope(typed, true, "retained-outcome", &args, |e| {
                assert!(matches!(
                    o.release_decoded_facet_publication_receipt_v1(e, &port, &mut r),
                    Err(E::InvalidRequest)
                ))
            });
        }
        while let Some(e) = r.pop_front() {
            assert_ne!(e.boundary, Boundary::FacetPublicationReceiptRelease);
        }
        assert_eq!(o.facet_publication_receipt_usage_v1().unwrap().stopped, 0);
    }
}
fn envelope<'h, T>(
    typed: bool,
    is_receipt: bool,
    operation: &str,
    args: &[(&str, V<'_, 'h>)],
    call: impl FnOnce(&V<'_, 'h>) -> T,
) -> T {
    let header = [
        (
            if typed { "family" } else { "name" },
            if typed {
                V::Data(InertValueV1::Number(if is_receipt { 5.0 } else { 18.0 }))
            } else {
                V::Data(InertValueV1::Text(if is_receipt {
                    "inspect-execution"
                } else {
                    "control-observation"
                }))
            },
        ),
        ("variant", V::Data(InertValueV1::Text(operation))),
        ("version", V::Data(InertValueV1::Number(1.0))),
        ("arguments", V::Object(args)),
    ];
    let outer = [
        (
            "route",
            V::Data(InertValueV1::Text(if typed { "typed" } else { "dynamic" })),
        ),
        ("request", V::Object(&header)),
    ];
    call(&V::Object(&outer))
}
pub(in crate::coordinator) fn command_envelope<'h, T>(
    typed: bool,
    id: &'h FacetPublicationCommandIdV1,
    stream: Option<&'h FacetPublicationStreamIdV1>,
    call: impl FnOnce(&V<'_, 'h>) -> T,
) -> T {
    let operation = if stream.is_some() {
        "authorized-resynchronization"
    } else {
        "establish-bounded-stream"
    };
    let args = [
        (
            "contract",
            V::Data(InertValueV1::Text(FACET_PUBLICATION_COMMAND_PROFILE_V1)),
        ),
        ("operation", V::Data(InertValueV1::Text(operation))),
        ("command", V::Command(id)),
        (
            "stream",
            stream.map_or(V::Data(InertValueV1::Null), V::Stream),
        ),
    ];
    envelope(typed, false, operation, &args, call)
}
pub(in crate::coordinator) fn receipt_envelope<'h, T>(
    typed: bool,
    command: &'h FacetPublicationCommandIdV1,
    context: &'h FacetPublicationReceiptContextIdV1,
    call: impl FnOnce(&V<'_, 'h>) -> T,
) -> T {
    let facet = [("kind", V::Data(InertValueV1::Text("named-facet-baseline")))];
    let args = [
        (
            "contract",
            V::Data(InertValueV1::Text(FACET_PUBLICATION_RECEIPT_PROFILE_V1)),
        ),
        ("operation", V::Data(InertValueV1::Text("retained-outcome"))),
        ("command", V::Command(command)),
        ("context", V::ReceiptContext(context)),
        ("facet", V::Object(&facet)),
    ];
    envelope(typed, true, "retained-outcome", &args, call)
}
