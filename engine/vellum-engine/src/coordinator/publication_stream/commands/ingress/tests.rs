use super::*;
use crate::coordinator::publication_stream::commands::receipts::tests as receipt_fixture;
use crate::coordinator::publication_stream::commands::tests::{edit, limits, pool_control};
use crate::coordinator::publication_stream::tests::{
    facts, limits as stream_limits, native, owner,
};
use crate::instrumentation::NativeTimingProfileV1;
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
    id: &'h SourcePublicationCommandIdV1,
    stream: Option<&'h SourcePublicationStreamIdV1>,
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
            V::Data(InertValueV1::Text(SOURCE_PUBLICATION_COMMAND_PROFILE_V1)),
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
    command: &'h SourcePublicationCommandIdV1,
    context: &'h SourcePublicationReceiptContextIdV1,
    call: impl FnOnce(&V<'_, 'h>) -> T,
) -> T {
    let facet = [("kind", V::Data(InertValueV1::Text("source-baseline")))];
    let args = [
        (
            "contract",
            V::Data(InertValueV1::Text(SOURCE_PUBLICATION_RECEIPT_PROFILE_V1)),
        ),
        ("operation", V::Data(InertValueV1::Text("retained-outcome"))),
        ("command", V::Command(command)),
        ("context", V::ReceiptContext(context)),
        ("facet", V::Object(&facet)),
    ];
    envelope(typed, true, "retained-outcome", &args, call)
}
#[test]
fn both_command_forms_execute_original_establish_and_resync_then_replay_without_data_facts() {
    for typed in [false, true] {
        for capacity in [0, 1, 64] {
            let (mut o, mut setup, s) = owner(stream_limits());
            o.configure_source_publication_commands_v1(limits(), &pool_control(), &mut setup)
                .unwrap();
            let n = native(&s);
            let f = facts(&s, &n);
            let op = SourcePublicationCommandOperationV1::Establish;
            let id = o
                .issue_source_publication_command_v1(&op, &s, &f, Some(true), &mut setup)
                .unwrap();
            let mut q = SourcePublicationCommandRequestV1 {
                scope: &s,
                operation: &op,
                current: Some(&f),
                owner_control_allowed: Some(true),
                max_reply_bytes: 16384,
            };
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            command_envelope(typed, &id, None, |v| {
                assert_eq!(
                    o.request_decoded_source_publication_command_v1(
                        v,
                        &SourcePublicationCommandPortV1 {
                            command: &id,
                            request: &q
                        },
                        &mut r
                    ),
                    Ok(SourcePublicationCommandReplyV1::Settled {
                        command: id.clone()
                    })
                )
            });
            let stream = o
                .source_publication_command_facts_v1(&id, &s, &mut setup)
                .unwrap()
                .as_observed
                .unwrap()
                .as_ref()
                .unwrap()
                .stream
                .clone();
            edit(&mut o, &s, "A", "B", &mut setup);
            let resync = SourcePublicationCommandOperationV1::Resynchronize {
                stream: stream.clone(),
            };
            let second = o
                .issue_source_publication_command_v1(&resync, &s, &f, Some(true), &mut setup)
                .unwrap();
            let resync_q = SourcePublicationCommandRequestV1 {
                operation: &resync,
                ..q
            };
            command_envelope(typed, &second, Some(&stream), |v| {
                assert!(
                    o.request_decoded_source_publication_command_v1(
                        v,
                        &SourcePublicationCommandPortV1 {
                            command: &second,
                            request: &resync_q
                        },
                        &mut r
                    )
                    .is_ok()
                )
            });
            let before = o.source_publication_usage_v1().unwrap();
            q.current = None;
            o.faulted = true;
            command_envelope(typed, &id, None, |v| {
                assert_eq!(
                    o.request_decoded_source_publication_command_v1(
                        v,
                        &SourcePublicationCommandPortV1 {
                            command: &id,
                            request: &q
                        },
                        &mut r
                    ),
                    Ok(SourcePublicationCommandReplyV1::ObserveExisting {
                        command: id.clone()
                    })
                )
            });
            assert_eq!(o.source_publication_usage_v1().unwrap(), before);
            if capacity == 64 {
                assert!(
                    std::iter::from_fn(|| r.pop_front())
                        .any(|e| e.boundary == Boundary::SourcePublicationCommandFacade
                            && e.service_time.is_some())
                );
            }
            if capacity == 1 {
                assert!(r.dropped() > 0);
            }
        }
    }
}
#[test]
fn both_receipt_forms_use_current_historical_authorization_without_another_command() {
    for typed in [false, true] {
        for capacity in [0, 1, 64] {
            let (mut o, mut setup, s, command, _) =
                receipt_fixture::setup(receipt_fixture::limits());
            let context = receipt_fixture::open(&mut o, &command, &s, &mut setup);
            edit(&mut o, &s, "A", "B", &mut setup);
            let basis = o
                .source_publication_receipt_basis_v1(&context, &command, &s, &mut setup)
                .unwrap()
                .basis;
            let f = receipt_fixture::facts(&s);
            let historical = TrustedSourcePublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &f,
            };
            let port = SourcePublicationReceiptPortV1 {
                command: &command,
                context: &context,
                scope: &s,
                current: &historical,
            };
            let before = o.source_publication_usage_v1().unwrap();
            let debt = o.retention_usage();
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            receipt_envelope(typed, &command, &context, |v| {
                let reply = o
                    .release_decoded_source_publication_receipt_v1(v, &port, &mut r)
                    .unwrap();
                assert_eq!(reply.result.as_ref().unwrap().projection.text(), "A");
                assert_eq!(reply.command, command);
            });
            assert_eq!(o.source_publication_usage_v1().unwrap(), before);
            assert_eq!(o.retention_usage(), debt);
            if capacity == 64 {
                assert!(
                    std::iter::from_fn(|| r.pop_front())
                        .any(|e| e.boundary == Boundary::SourcePublicationReceiptFacade
                            && e.service_time.is_some())
                );
            }
            if capacity == 1 {
                assert!(r.dropped() > 0);
            }
        }
    }
}
// Mutate only the known fixed test envelopes, never arbitrary production data.
fn mutations<'h>(v: &V<'_, 'h>, call: &mut dyn FnMut(&V<'_, 'h>)) {
    let V::Object(rows) = v else {
        return;
    };
    call(&V::Data(InertValueV1::Null));
    for change in 0..3 {
        let mut altered = rows.to_vec();
        match change {
            0 => {
                altered.pop();
            }
            1 => altered.push(rows[0]),
            _ => altered.push(("authority", V::Data(InertValueV1::Boolean(true)))),
        }
        call(&V::Object(&altered));
    }
    for index in 0..rows.len() {
        let mut altered = rows.to_vec();
        altered[index].1 = if matches!(rows[index].1, V::Data(InertValueV1::Null)) {
            V::Data(InertValueV1::Boolean(true))
        } else {
            V::Data(InertValueV1::Null)
        };
        call(&V::Object(&altered));
        if matches!(rows[index].1, V::Object(_)) {
            mutations(&rows[index].1, &mut |changed| {
                let mut parent = rows.to_vec();
                parent[index].1 = *changed;
                call(&V::Object(&parent));
            });
        }
    }
}
#[test]
fn malformed_or_wrong_original_roles_never_reach_command_execution_or_receipt_loss() {
    for typed in [false, true] {
        let (mut o, mut setup, s, command, _) = receipt_fixture::setup(receipt_fixture::limits());
        let context = receipt_fixture::open(&mut o, &command, &s, &mut setup);
        let sibling = receipt_fixture::open(&mut o, &command, &s, &mut setup);
        let basis = o
            .source_publication_receipt_basis_v1(&context, &command, &s, &mut setup)
            .unwrap()
            .basis;
        let mut denied = receipt_fixture::facts(&s);
        denied.complete_result_allowed = Some(false);
        let f = TrustedSourcePublicationReceiptHistoricalFactsV1 {
            basis: &basis,
            current: &denied,
        };
        let receipt_port = SourcePublicationReceiptPortV1 {
            command: &command,
            context: &context,
            scope: &s,
            current: &f,
        };
        let op = SourcePublicationCommandOperationV1::Establish;
        let q = SourcePublicationCommandRequestV1 {
            scope: &s,
            operation: &op,
            current: None,
            owner_control_allowed: Some(true),
            max_reply_bytes: 16384,
        };
        let command_port = SourcePublicationCommandPortV1 {
            command: &command,
            request: &q,
        };
        let before = o.source_publication_usage_v1().unwrap();
        receipt_envelope(typed, &command, &context, |v| {
            mutations(v, &mut |bad| {
                let mut r = Recorder::try_with_capacity(64).unwrap();
                assert_eq!(
                    o.release_decoded_source_publication_receipt_v1(bad, &receipt_port, &mut r)
                        .unwrap_err(),
                    E::InvalidRequest
                );
                assert!(
                    !std::iter::from_fn(|| r.pop_front())
                        .any(|e| e.boundary == Boundary::SourcePublicationReceiptRelease)
                );
            })
        });
        command_envelope(typed, &command, None, |v| {
            mutations(v, &mut |bad| {
                let mut r = Recorder::try_with_capacity(64).unwrap();
                assert_eq!(
                    o.request_decoded_source_publication_command_v1(bad, &command_port, &mut r)
                        .unwrap_err(),
                    E::InvalidRequest
                );
                assert!(
                    !std::iter::from_fn(|| r.pop_front())
                        .any(|e| e.boundary == Boundary::SourcePublicationCommandRequest)
                );
            })
        });
        receipt_envelope(typed, &command, &sibling, |v| {
            assert_eq!(
                o.release_decoded_source_publication_receipt_v1(v, &receipt_port, &mut setup)
                    .unwrap_err(),
                E::Native(Error::Unavailable)
            )
        });
        assert_eq!(o.source_publication_receipt_usage_v1().unwrap().stopped, 0);
        assert_eq!(o.source_publication_usage_v1().unwrap(), before);
    }
}
#[test]
fn receipt_result_guard_refuses_copied_allocation_or_wrong_command_and_preserves_claims() {
    for wrong_command in [false, true] {
        let (mut o, mut r, s, command, _) = receipt_fixture::setup(receipt_fixture::limits());
        let context = receipt_fixture::open(&mut o, &command, &s, &mut r);
        let basis = o
            .source_publication_receipt_basis_v1(&context, &command, &s, &mut r)
            .unwrap()
            .basis;
        let good = o
            .release_source_publication_receipt_v1(
                &context,
                &command,
                &s,
                &TrustedSourcePublicationReceiptHistoricalFactsV1 {
                    basis: &basis,
                    current: &receipt_fixture::facts(&s),
                },
                &mut r,
            )
            .unwrap();
        let b = good.result.as_ref().unwrap();
        let n = native(&s);
        let f = facts(&s, &n);
        let other = o
            .issue_source_publication_command_v1(
                &SourcePublicationCommandOperationV1::Establish,
                &s,
                &f,
                Some(true),
                &mut r,
            )
            .unwrap();
        let forged = SourcePublicationReceiptReplyV1 {
            command: if wrong_command {
                other
            } else {
                command.clone()
            },
            result: if wrong_command {
                good.result.clone()
            } else {
                Ok(Arc::new(SourcePublicationBaselineV1 {
                    stream: b.stream.clone(),
                    generation: b.generation.clone(),
                    projection: b.projection.clone(),
                }))
            },
        };
        assert_eq!(
            o.validate_source_publication_receipt_reply_v1(&context, &command, &s, &forged, &mut r),
            Err(Error::Faulted)
        );
        assert_eq!(o.source_publication_receipt_usage_v1().unwrap().stopped, 1);
        assert_eq!(
            o.retire_source_publication_command_v1(
                &command,
                &super::super::tests::control(&s),
                &mut r
            ),
            Err(Error::Unavailable)
        );
    }
}
