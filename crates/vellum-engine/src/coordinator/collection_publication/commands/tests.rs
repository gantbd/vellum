use super::super::tests::{body, cleanup, fixture_with_budget};
use super::super::tests::{control, facts, fixture, limits, packet};
use super::ingress::CollectionPublicationEnvelopeValueV1 as V;
use super::*;
#[test]
fn collection_shutdown_fences_fresh_admission_and_unexecuted_establishment_but_preserves_existing_receipt_and_replay()
 {
    let (mut o, mut r, s, g) = setup();
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let f = facts(&s, &n);
    let p = packet(&mut o, &s, &g, &mut r);
    let op = CollectionPublicationCommandOperationV1::Establish;
    let id = o
        .issue_collection_publication_command_v1(&op, &p, &s, &f, Some(true), &mut r)
        .unwrap();
    let mut q = CollectionPublicationCommandRequestV1 {
        scope: &s,
        operation: &op,
        preparation: &p,
        current: Some(&f),
        owner_control_allowed: Some(true),
        max_reply_bytes: 16384,
    };
    o.request_collection_publication_command_v1(&id, &q, &mut r)
        .unwrap();
    let context = open_receipt(&mut o, &id, &s, &mut r);
    let waiting = packet(&mut o, &s, &g, &mut r);
    let pending = o
        .issue_collection_publication_command_v1(&op, &waiting, &s, &f, Some(true), &mut r)
        .unwrap();
    crate::coordinator::shutdown::tests::install(&mut o, &mut r);
    let before = (
        o.annotation_preparation_usage(),
        o.collection_publication_command_usage_v1().unwrap(),
        o.collection_publication_receipt_usage_v1().unwrap(),
        o.retention_usage(),
    );
    assert_eq!(
        o.issue_collection_publication_command_v1(&op, &waiting, &s, &f, Some(true), &mut r),
        Err(Error::Unavailable)
    );
    assert_eq!(
        o.request_collection_publication_command_v1(
            &pending,
            &CollectionPublicationCommandRequestV1 {
                preparation: &waiting,
                ..q
            },
            &mut r
        ),
        Err(Error::Unavailable)
    );
    assert!(
        !o.collection_publication_command_facts_v1(&pending, &s, &mut r)
            .unwrap()
            .native_started
    );
    let basis = o
        .collection_publication_command_receipt_basis_v1(&id, &s, &mut r)
        .unwrap()
        .basis;
    assert_eq!(
        o.open_collection_publication_receipt_v1(
            &id,
            &s,
            CollectionPublicationReceiptCeilingV1 {
                complete_original_result: true
            },
            &TrustedCollectionPublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &receipt_facts(&s)
            },
            Some(true),
            &mut r
        ),
        Err(Error::Unavailable)
    );
    assert_eq!(
        (
            o.annotation_preparation_usage(),
            o.collection_publication_command_usage_v1().unwrap(),
            o.collection_publication_receipt_usage_v1().unwrap(),
            o.retention_usage()
        ),
        before
    );
    q.current = None;
    assert!(matches!(
        o.request_collection_publication_command_v1(&id, &q, &mut r),
        Ok(CollectionPublicationCommandReplyV1::ObserveExisting { .. })
    ));
    let basis = o
        .collection_publication_receipt_basis_v1(&context, &id, &s, &mut r)
        .unwrap()
        .basis;
    assert!(
        o.release_collection_publication_receipt_v1(
            &context,
            &id,
            &s,
            &TrustedCollectionPublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &receipt_facts(&s)
            },
            &mut r
        )
        .is_ok()
    );
    o.stop_collection_publication_receipt_v1(&context, &id, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_collection_publication_receipt_v1(&context, &id, &cleanup(&s), &mut r)
        .unwrap();
}

#[test]
fn collection_decoded_preparation_must_be_original_opaque_role_before_any_native_attempt() {
    for typed in [false, true] {
        let (mut o, mut setup, s, g) = setup();
        let p = packet(&mut o, &s, &g, &mut setup);
        let n = crate::coordinator::publication_stream::tests::native(&s.source);
        let f = facts(&s, &n);
        let op = CollectionPublicationCommandOperationV1::Establish;
        let id = o
            .issue_collection_publication_command_v1(&op, &p, &s, &f, Some(true), &mut setup)
            .unwrap();
        let q = CollectionPublicationCommandRequestV1 {
            scope: &s,
            operation: &op,
            preparation: &p,
            current: Some(&f),
            owner_control_allowed: Some(true),
            max_reply_bytes: 16384,
        };
        let before = o.annotation_preparation_usage();
        let mut r = Recorder::try_with_capacity(64).unwrap();
        for case in 0..5 {
            command_envelope(typed, &id, None, &p, |v| {
                let V::Object(outer) = v else { unreachable!() };
                let mut outer = outer.to_vec();
                let V::Object(header) = outer[1].1 else {
                    unreachable!()
                };
                let mut header = header.to_vec();
                let V::Object(args) = header[3].1 else {
                    unreachable!()
                };
                let mut args = args.to_vec();
                let row = args.iter_mut().find(|(k, _)| *k == "preparation").unwrap();
                match case {
                    0 => row.1 = V::Data(InertValueV1::Text("forged-packet")),
                    1 => row.1 = V::Command(&id),
                    2 => row.0 = "command",
                    3 => {
                        args.retain(|(k, _)| *k != "preparation");
                    }
                    4 => args.push(("authority", V::Data(InertValueV1::Boolean(true)))),
                    _ => unreachable!(),
                }
                header[3].1 = V::Object(&args);
                outer[1].1 = V::Object(&header);
                assert!(matches!(
                    o.request_decoded_collection_publication_command_v1(
                        &V::Object(&outer),
                        &CollectionPublicationCommandPortV1 {
                            command: &id,
                            request: &q
                        },
                        &mut r
                    ),
                    Err(CollectionPublicationIngressErrorV1::InvalidRequest)
                ));
            });
        }
        while let Some(e) = r.pop_front() {
            assert_ne!(e.boundary, Boundary::CollectionPublicationCommandRequest);
        }
        assert_eq!(o.annotation_preparation_usage(), before);
        assert!(
            !o.collection_publication_command_facts_v1(&id, &s, &mut setup)
                .unwrap()
                .native_started
        );
    }
}
use crate::capture::InertValueV1;
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
    id: &'h CollectionPublicationCommandIdV1,
    stream: Option<&'h CollectionPublicationStreamIdV1>,
    preparation: &'h CollectionVisibilityPreparationIdV1,
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
            V::Data(InertValueV1::Text(
                COLLECTION_PUBLICATION_COMMAND_PROFILE_V1,
            )),
        ),
        ("operation", V::Data(InertValueV1::Text(operation))),
        ("command", V::Command(id)),
        ("preparation", V::Preparation(preparation)),
        (
            "stream",
            stream.map_or(V::Data(InertValueV1::Null), V::Stream),
        ),
    ];
    envelope(typed, false, operation, &args, call)
}
pub(in crate::coordinator) fn receipt_envelope<'h, T>(
    typed: bool,
    command: &'h CollectionPublicationCommandIdV1,
    context: &'h CollectionPublicationReceiptContextIdV1,
    call: impl FnOnce(&V<'_, 'h>) -> T,
) -> T {
    let facet = [("kind", V::Data(InertValueV1::Text("collection-baseline")))];
    let args = [
        (
            "contract",
            V::Data(InertValueV1::Text(
                COLLECTION_PUBLICATION_RECEIPT_PROFILE_V1,
            )),
        ),
        ("operation", V::Data(InertValueV1::Text("retained-outcome"))),
        ("command", V::Command(command)),
        ("context", V::ReceiptContext(context)),
        ("facet", V::Object(&facet)),
    ];
    envelope(typed, true, "retained-outcome", &args, call)
}

pub(in crate::coordinator) fn setup() -> (
    SourceCoordinator,
    Recorder,
    CollectionPublicationScopeV1,
    Vec<AnnotationFacetGrantV1>,
) {
    let (mut o, mut r, s, g) = fixture_with_budget(limits(), 8388608);
    let mut l = CollectionPublicationCommandLimitsV1 {
        max_commands: 4,
        max_issued: 8,
        max_context_bytes: 4096,
        max_result_bytes: 16384,
        max_pool_bytes: usize::MAX,
    };
    l.max_pool_bytes = l.required_pool_bytes().unwrap();
    o.configure_collection_publication_commands_v1(l, &control(), &mut r)
        .unwrap();
    let mut l = CollectionPublicationReceiptLimitsV1 {
        max_contexts: 4,
        max_issued: 8,
        max_context_bytes: 4096,
        max_reply_bytes: 16384,
        max_pool_bytes: usize::MAX,
    };
    l.max_pool_bytes = l.required_pool_bytes().unwrap();
    o.configure_collection_publication_receipts_v1(l, &control(), &mut r)
        .unwrap();
    (o, r, s, g)
}
pub(in crate::coordinator) fn receipt_facts(
    s: &CollectionPublicationScopeV1,
) -> TrustedCollectionPublicationReceiptFactsV1<'_> {
    TrustedCollectionPublicationReceiptFactsV1 {
        scope: s,
        authorized: Some(true),
        scope_current: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        retained: Some(true),
        receipt_grant_active: Some(true),
        complete_result_allowed: Some(true),
        authorization_current: Some(true),
        resources_ready: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
        max_reply_bytes: 16384,
    }
}
pub(in crate::coordinator) fn open_receipt(
    o: &mut SourceCoordinator,
    id: &CollectionPublicationCommandIdV1,
    s: &CollectionPublicationScopeV1,
    r: &mut Recorder,
) -> CollectionPublicationReceiptContextIdV1 {
    let basis = o
        .collection_publication_command_receipt_basis_v1(id, s, r)
        .unwrap()
        .basis;
    o.open_collection_publication_receipt_v1(
        id,
        s,
        CollectionPublicationReceiptCeilingV1 {
            complete_original_result: true,
        },
        &TrustedCollectionPublicationReceiptHistoricalFactsV1 {
            basis: &basis,
            current: &receipt_facts(s),
        },
        Some(true),
        r,
    )
    .unwrap()
}

#[test]
fn collection_command_changed_preparation_is_not_original_intent_and_neither_packet_is_spent_by_refusal()
 {
    let (mut o, mut r, s, g) = setup();
    let p = packet(&mut o, &s, &g, &mut r);
    let other = packet(&mut o, &s, &g, &mut r);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let f = facts(&s, &n);
    let op = CollectionPublicationCommandOperationV1::Establish;
    let id = o
        .issue_collection_publication_command_v1(&op, &p, &s, &f, Some(true), &mut r)
        .unwrap();
    assert_eq!(o.annotation_preparation_usage().prepared, 2);
    let mut q = CollectionPublicationCommandRequestV1 {
        scope: &s,
        operation: &op,
        preparation: &other,
        current: Some(&f),
        owner_control_allowed: Some(true),
        max_reply_bytes: 16384,
    };
    assert_eq!(
        o.request_collection_publication_command_v1(&id, &q, &mut r),
        Err(Error::Unavailable)
    );
    assert_eq!(o.annotation_preparation_usage().prepared, 2);
    assert!(
        !o.collection_publication_command_facts_v1(&id, &s, &mut r)
            .unwrap()
            .native_started
    );
    q.preparation = &p;
    o.request_collection_publication_command_v1(&id, &q, &mut r)
        .unwrap();
    assert_eq!(o.annotation_preparation_usage().prepared, 1);
    q.preparation = &other;
    q.current = None;
    assert_eq!(
        o.request_collection_publication_command_v1(&id, &q, &mut r),
        Err(Error::Unavailable)
    );
    assert_eq!(o.annotation_preparation_usage().prepared, 1);
}

#[test]
fn collection_command_known_failure_after_consumption_is_retained_without_restoring_or_retrying_preparation()
 {
    let (mut o, mut r, s, g) = setup();
    let p = packet(&mut o, &s, &g, &mut r);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let mut f = facts(&s, &n);
    let op = CollectionPublicationCommandOperationV1::Establish;
    let id = o
        .issue_collection_publication_command_v1(&op, &p, &s, &f, Some(true), &mut r)
        .unwrap();
    f.max_reply_bytes = 0;
    let mut q = CollectionPublicationCommandRequestV1 {
        scope: &s,
        operation: &op,
        preparation: &p,
        current: Some(&f),
        owner_control_allowed: Some(true),
        max_reply_bytes: 16384,
    };
    o.request_collection_publication_command_v1(&id, &q, &mut r)
        .unwrap();
    assert_eq!(o.annotation_preparation_usage().prepared, 0);
    assert_eq!(o.collection_publication_usage_v1().unwrap().streams, 0);
    assert!(matches!(
        o.collection_publication_command_facts_v1(&id, &s, &mut r)
            .unwrap()
            .as_observed,
        Some(Err(Error::Limit))
    ));
    q.current = None;
    assert!(matches!(
        o.request_collection_publication_command_v1(&id, &q, &mut r),
        Ok(CollectionPublicationCommandReplyV1::ObserveExisting { .. })
    ));
    assert_eq!(o.annotation_preparation_usage().prepared, 0);
    let context = open_receipt(&mut o, &id, &s, &mut r);
    let basis = o
        .collection_publication_receipt_basis_v1(&context, &id, &s, &mut r)
        .unwrap()
        .basis;
    assert!(matches!(
        o.release_collection_publication_receipt_v1(
            &context,
            &id,
            &s,
            &TrustedCollectionPublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &receipt_facts(&s)
            },
            &mut r
        )
        .unwrap()
        .result,
        Err(Error::Limit)
    ));
}

#[test]
fn collection_command_resync_and_receipts_preserve_original_local_references_and_subsequent_event_chain()
 {
    let (mut o, mut r, s, g) = setup();
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let f = facts(&s, &n);
    let p = packet(&mut o, &s, &g, &mut r);
    let op = CollectionPublicationCommandOperationV1::Establish;
    let id = o
        .issue_collection_publication_command_v1(&op, &p, &s, &f, Some(true), &mut r)
        .unwrap();
    o.request_collection_publication_command_v1(
        &id,
        &CollectionPublicationCommandRequestV1 {
            scope: &s,
            operation: &op,
            preparation: &p,
            current: Some(&f),
            owner_control_allowed: Some(true),
            max_reply_bytes: 16384,
        },
        &mut r,
    )
    .unwrap();
    let original = o
        .collection_publication_command_facts_v1(&id, &s, &mut r)
        .unwrap()
        .as_observed
        .unwrap()
        .as_ref()
        .unwrap()
        .clone();
    let context = open_receipt(&mut o, &id, &s, &mut r);
    let stale = o
        .collection_publication_receipt_basis_v1(&context, &id, &s, &mut r)
        .unwrap()
        .basis;
    body(&mut o, &g[0].annotation, "two", &mut r);
    let p2 = packet(&mut o, &s, &g, &mut r);
    let resync = CollectionPublicationCommandOperationV1::Resynchronize {
        stream: original.stream.clone(),
    };
    let second = o
        .issue_collection_publication_command_v1(&resync, &p2, &s, &f, Some(true), &mut r)
        .unwrap();
    o.request_collection_publication_command_v1(
        &second,
        &CollectionPublicationCommandRequestV1 {
            scope: &s,
            operation: &resync,
            preparation: &p2,
            current: Some(&f),
            owner_control_allowed: Some(true),
            max_reply_bytes: 16384,
        },
        &mut r,
    )
    .unwrap();
    let newer = o
        .collection_publication_command_facts_v1(&second, &s, &mut r)
        .unwrap()
        .as_observed
        .unwrap()
        .as_ref()
        .unwrap()
        .clone();
    assert_eq!(
        original.projection.rows()[0].reference,
        newer.projection.rows()[0].reference
    );
    assert_eq!(
        original.projection.rows()[0].content.as_ref().unwrap().0,
        "one"
    );
    assert_eq!(
        newer.projection.rows()[0].content.as_ref().unwrap().0,
        "two"
    );
    let mut denied = receipt_facts(&s);
    denied.complete_result_allowed = Some(false);
    assert!(
        o.release_collection_publication_receipt_v1(
            &context,
            &id,
            &s,
            &TrustedCollectionPublicationReceiptHistoricalFactsV1 {
                basis: &stale,
                current: &denied
            },
            &mut r
        )
        .is_err()
    );
    assert_eq!(
        o.collection_publication_receipt_usage_v1().unwrap().stopped,
        0
    );
    let current = o
        .collection_publication_receipt_basis_v1(&context, &id, &s, &mut r)
        .unwrap();
    assert!(std::ptr::eq(
        current.original_projection.unwrap(),
        original.projection.as_ref()
    ));
    let basis = current.basis;
    let reply = o
        .release_collection_publication_receipt_v1(
            &context,
            &id,
            &s,
            &TrustedCollectionPublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &receipt_facts(&s),
            },
            &mut r,
        )
        .unwrap();
    assert!(Arc::ptr_eq(&reply.result.unwrap(), &original));
    body(&mut o, &g[0].annotation, "three", &mut r);
    let p3 = packet(&mut o, &s, &g, &mut r);
    o.sample_collection_publication_stream_v1(&original.stream, &s, &p3, &f, &mut r)
        .unwrap();
    let ObservationEventDeliveryV1::Collection(CollectionPublicationDeliveryV1::Projection(event)) =
        o.release_observation_event_v1(
            ObservationEventTargetV1::Collection(&original.stream),
            ObservationEventPortV1::Collection {
                stream: &original.stream,
                scope: &s,
                facts: &f,
            },
            &mut r,
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(event.before, newer.generation);
    assert_eq!(
        event.projection.rows()[0].reference,
        original.projection.rows()[0].reference
    );
}

#[test]
fn collection_command_interrupted_acknowledgment_preserves_consumed_input_unknown_effect_and_protected_cleanup()
 {
    let (mut o, mut r, s, g) = setup();
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let f = facts(&s, &n);
    let p = packet(&mut o, &s, &g, &mut r);
    let op = CollectionPublicationCommandOperationV1::Establish;
    let id = o
        .issue_collection_publication_command_v1(&op, &p, &s, &f, Some(true), &mut r)
        .unwrap();
    let super::super::State::Active(pool) = &mut o.collection_publication_streams else {
        unreachable!()
    };
    let State::Active(t) = &mut pool.commands else {
        unreachable!()
    };
    t.fault_after_native = true;
    let q = CollectionPublicationCommandRequestV1 {
        scope: &s,
        operation: &op,
        preparation: &p,
        current: Some(&f),
        owner_control_allowed: Some(true),
        max_reply_bytes: 16384,
    };
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || o.request_collection_publication_command_v1(&id, &q, &mut r)
        ))
        .is_err()
    );
    assert_eq!(o.annotation_preparation_usage().prepared, 0);
    assert_eq!(o.collection_publication_usage_v1().unwrap().streams, 1);
    let original = o
        .collection_publication_command_facts_v1(&id, &s, &mut r)
        .unwrap();
    assert_eq!(original.phase, PhaseV1::Faulted);
    assert!(original.native_started && original.as_observed.is_none());
    assert!(
        o.request_collection_publication_command_v1(&id, &q, &mut r)
            .is_err()
    );
    assert_eq!(
        o.retire_collection_publication_command_v1(&id, &cleanup(&s), &mut r),
        Err(Error::Unavailable)
    );
    let stream = CollectionPublicationStreamIdV1 {
        owner: id.owner.clone(),
        slot: 0,
    };
    o.stop_collection_publication_stream_v1(&stream, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_collection_publication_stream_v1(&stream, &cleanup(&s), &mut r)
        .unwrap();
    assert_eq!(
        o.document_context_pins_v1(&s.source.document, &mut r)
            .unwrap()
            .publication_streams,
        1
    );
}

#[test]
fn collection_receipt_authority_and_funding_are_independent_and_claims_block_premature_retirement()
{
    let (mut o, mut r, s, g) = setup();
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let f = facts(&s, &n);
    let p = packet(&mut o, &s, &g, &mut r);
    let op = CollectionPublicationCommandOperationV1::Establish;
    let id = o
        .issue_collection_publication_command_v1(&op, &p, &s, &f, Some(true), &mut r)
        .unwrap();
    o.request_collection_publication_command_v1(
        &id,
        &CollectionPublicationCommandRequestV1 {
            scope: &s,
            operation: &op,
            preparation: &p,
            current: Some(&f),
            owner_control_allowed: Some(true),
            max_reply_bytes: 16384,
        },
        &mut r,
    )
    .unwrap();
    let basis = o
        .collection_publication_command_receipt_basis_v1(&id, &s, &mut r)
        .unwrap()
        .basis;
    let mut denied = receipt_facts(&s);
    denied.complete_result_allowed = None;
    assert_eq!(
        o.open_collection_publication_receipt_v1(
            &id,
            &s,
            CollectionPublicationReceiptCeilingV1 {
                complete_original_result: true
            },
            &TrustedCollectionPublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &denied
            },
            Some(true),
            &mut r
        ),
        Err(Error::Unavailable)
    );
    let context = open_receipt(&mut o, &id, &s, &mut r);
    let charge = o
        .collection_publication_receipt_usage_v1()
        .unwrap()
        .reserved_pool_bytes;
    let command_charge = o
        .collection_publication_command_usage_v1()
        .unwrap()
        .reserved_pool_bytes;
    let debt = o.retention_usage();
    let row = crate::coordinator::shutdown::tests::inventory_owner(
        &o,
        &mut r,
        ShutdownOwnerRoleV1::CollectionPublication,
        &[&s.source.document],
    );
    assert_eq!(row.retained_records, Some(3));
    assert_eq!(row.additional_reserved_bytes, Some(charge + command_charge));
    assert_eq!(
        o.retire_collection_publication_command_v1(&id, &cleanup(&s), &mut r),
        Err(Error::Unavailable)
    );
    o.stop_collection_publication_receipt_v1(&context, &id, &cleanup(&s), &mut r)
        .unwrap();
    assert_eq!(
        o.retire_collection_publication_command_v1(&id, &cleanup(&s), &mut r),
        Err(Error::Unavailable)
    );
    assert_eq!(o.retention_usage(), debt);
    let mut bad = cleanup(&s);
    bad.delivery_drained = Some(false);
    assert_eq!(
        o.retire_collection_publication_receipt_v1(&context, &id, &bad, &mut r),
        Err(Error::Unavailable)
    );
    o.retire_collection_publication_receipt_v1(&context, &id, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_collection_publication_receipts_v1(&control(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, debt.reserved - charge);
    o.retire_collection_publication_command_v1(&id, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_collection_publication_commands_v1(&control(), &mut r)
        .unwrap();
    assert_eq!(
        o.retention_usage().reserved,
        debt.reserved - charge - command_charge
    );
    assert!(o.collection_publication_command_usage_v1().unwrap().closed);
}

#[test]
fn collection_command_identity_binds_one_original_preparation_and_replay_does_not_consume_again() {
    let (mut o, mut r, s, g) = fixture(limits());
    let mut l = CollectionPublicationCommandLimitsV1 {
        max_commands: 2,
        max_issued: 4,
        max_context_bytes: 2048,
        max_result_bytes: 16384,
        max_pool_bytes: usize::MAX,
    };
    l.max_pool_bytes = l.required_pool_bytes().unwrap();
    o.configure_collection_publication_commands_v1(l, &control(), &mut r)
        .unwrap();
    let p = packet(&mut o, &s, &g, &mut r);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let f = facts(&s, &n);
    let op = CollectionPublicationCommandOperationV1::Establish;
    let id = o
        .issue_collection_publication_command_v1(&op, &p, &s, &f, Some(true), &mut r)
        .unwrap();
    let mut q = CollectionPublicationCommandRequestV1 {
        scope: &s,
        operation: &op,
        preparation: &p,
        current: Some(&f),
        owner_control_allowed: Some(true),
        max_reply_bytes: 65536,
    };
    o.request_collection_publication_command_v1(&id, &q, &mut r)
        .unwrap();
    let original = o
        .collection_publication_command_facts_v1(&id, &s, &mut r)
        .unwrap()
        .as_observed
        .unwrap()
        .as_ref()
        .unwrap()
        .clone();
    let before = o.collection_publication_usage_v1().unwrap();
    q.current = None;
    assert!(matches!(
        o.request_collection_publication_command_v1(&id, &q, &mut r),
        Ok(CollectionPublicationCommandReplyV1::ObserveExisting { .. })
    ));
    assert_eq!(o.collection_publication_usage_v1().unwrap(), before);
    assert!(Arc::ptr_eq(
        &original,
        o.collection_publication_command_facts_v1(&id, &s, &mut r)
            .unwrap()
            .as_observed
            .unwrap()
            .as_ref()
            .unwrap()
    ));
}
