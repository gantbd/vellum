use super::*;
#[test]
fn publication_admission_fence_source_preserves_existing_receipt_replay_and_protected_cleanup() {
    for mode in 0..4 {
        let (mut o, mut setup, s, command, _stream) = setup_with_budget(limits(), 8388608);
        let context = open(&mut o, &command, &s, &mut setup);
        let n = native(&s);
        let f = stream_facts(&s, &n);
        let op = SourcePublicationCommandOperationV1::Establish;
        let pending = o
            .issue_source_publication_command_v1(&op, &s, &f, Some(true), &mut setup)
            .unwrap();
        crate::coordinator::shutdown::tests::install(&mut o, &mut setup);
        let before = (
            o.source_publication_usage_v1().unwrap(),
            o.source_publication_command_usage_v1().unwrap(),
            o.source_publication_receipt_usage_v1().unwrap(),
            o.retention_usage(),
        );
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 128 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        }
        let sentinel = Observation {
            boundary: Boundary::Control,
            disposition: Disposition::Completed,
            service_time: None,
        };
        if mode == 3 {
            r.record(sentinel);
        }
        assert_eq!(
            o.issue_source_publication_command_v1(&op, &s, &f, Some(true), &mut r),
            Err(Error::Unavailable)
        );
        assert!(matches!(
            o.open_source_publication_stream_v1(&s, &f, &mut r),
            Err(Error::Unavailable)
        ));
        let mut q = SourcePublicationCommandRequestV1 {
            scope: &s,
            operation: &op,
            current: Some(&f),
            owner_control_allowed: Some(true),
            max_reply_bytes: 65536,
        };
        assert_eq!(
            o.request_source_publication_command_v1(&pending, &q, &mut r),
            Err(Error::Unavailable)
        );
        assert!(
            !o.source_publication_command_facts_v1(&pending, &s, &mut setup)
                .unwrap()
                .native_started
        );
        let basis = o
            .source_publication_command_receipt_basis_v1(&command, &s, &mut setup)
            .unwrap()
            .basis;
        assert_eq!(
            o.open_source_publication_receipt_v1(
                &command,
                &s,
                SourcePublicationReceiptCeilingV1 {
                    complete_original_result: true
                },
                &TrustedSourcePublicationReceiptHistoricalFactsV1 {
                    basis: &basis,
                    current: &facts(&s)
                },
                Some(true),
                &mut r
            ),
            Err(Error::Unavailable)
        );
        assert_eq!(
            (
                o.source_publication_usage_v1().unwrap(),
                o.source_publication_command_usage_v1().unwrap(),
                o.source_publication_receipt_usage_v1().unwrap(),
                o.retention_usage()
            ),
            before
        );
        q.current = None;
        assert!(matches!(
            o.request_source_publication_command_v1(&command, &q, &mut r),
            Ok(SourcePublicationCommandReplyV1::ObserveExisting { .. })
        ));
        let basis = o
            .source_publication_receipt_basis_v1(&context, &command, &s, &mut setup)
            .unwrap()
            .basis;
        assert!(
            o.release_source_publication_receipt_v1(
                &context,
                &command,
                &s,
                &TrustedSourcePublicationReceiptHistoricalFactsV1 {
                    basis: &basis,
                    current: &facts(&s)
                },
                &mut r
            )
            .is_ok()
        );
        o.faulted = true;
        o.stop_source_publication_receipt_v1(&context, &command, &control(&s), &mut r)
            .unwrap();
        o.retire_source_publication_receipt_v1(&context, &command, &control(&s), &mut r)
            .unwrap();
        o.retire_source_publication_command_v1(&command, &control(&s), &mut r)
            .unwrap();
        if mode == 0 {
            assert_eq!(r.queued(), 0);
            assert_eq!(r.dropped(), 0);
        } else if mode == 3 {
            assert_eq!(r.pop_front(), Some(sentinel));
            assert!(r.dropped() > 0);
        } else {
            let mut ends = Vec::new();
            while let Some(e) = r.pop_front() {
                if e.boundary == Boundary::SourcePublicationCommandRequest {
                    assert_eq!(e.service_time.is_some(), mode == 2);
                    ends.push(e.disposition);
                }
            }
            assert_eq!(ends, [Disposition::Refused, Disposition::Completed]);
            assert_eq!(r.dropped(), 0);
        }
    }
}
#[test]
fn shutdown_inventory_captures_stream_command_receipt_and_all_three_original_reservations() {
    use crate::coordinator::shutdown::tests::inventory_owner;
    let (mut o, mut r, s, command, _stream) = setup(limits());
    open(&mut o, &command, &s, &mut r);
    let row = inventory_owner(
        &o,
        &mut r,
        ShutdownOwnerRoleV1::SourcePublication,
        &[&s.document],
    );
    assert_eq!(row.retained_records, Some(3));
    let expected =
        command_limits().required_pool_bytes().unwrap() + limits().required_pool_bytes().unwrap();
    assert_eq!(row.additional_reserved_bytes, Some(expected));
}
use crate::coordinator::publication_stream::commands::tests::{
    control, edit, limits as command_limits, pool_control,
};
use crate::coordinator::publication_stream::tests::{
    facts as stream_facts, limits as stream_limits, native, owner,
};
use crate::instrumentation::NativeTimingProfileV1;

pub(in crate::coordinator) fn limits() -> SourcePublicationReceiptLimitsV1 {
    SourcePublicationReceiptLimitsV1 {
        max_contexts: 2,
        max_issued: 8,
        max_context_bytes: 2048,
        max_reply_bytes: 16384,
        max_pool_bytes: 262144,
    }
}
pub(in crate::coordinator) fn facts(
    s: &SourceReadScopeV1,
) -> TrustedSourcePublicationReceiptFactsV1<'_> {
    TrustedSourcePublicationReceiptFactsV1 {
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
pub(in crate::coordinator) fn setup(
    l: SourcePublicationReceiptLimitsV1,
) -> (
    SourceCoordinator,
    Recorder,
    SourceReadScopeV1,
    SourcePublicationCommandIdV1,
    SourcePublicationStreamIdV1,
) {
    setup_with_budget(l, 1048576)
}
pub(in crate::coordinator) fn setup_with_budget(
    l: SourcePublicationReceiptLimitsV1,
    maximum: usize,
) -> (
    SourceCoordinator,
    Recorder,
    SourceReadScopeV1,
    SourcePublicationCommandIdV1,
    SourcePublicationStreamIdV1,
) {
    let (mut o, mut r, s) =
        crate::coordinator::publication_stream::tests::owner_with_budget(stream_limits(), maximum);
    o.configure_source_publication_commands_v1(command_limits(), &pool_control(), &mut r)
        .unwrap();
    let n = native(&s);
    let f = stream_facts(&s, &n);
    let op = SourcePublicationCommandOperationV1::Establish;
    let command = o
        .issue_source_publication_command_v1(&op, &s, &f, Some(true), &mut r)
        .unwrap();
    o.request_source_publication_command_v1(
        &command,
        &SourcePublicationCommandRequestV1 {
            scope: &s,
            operation: &op,
            current: Some(&f),
            owner_control_allowed: Some(true),
            max_reply_bytes: 16384,
        },
        &mut r,
    )
    .unwrap();
    let stream = o
        .source_publication_command_facts_v1(&command, &s, &mut r)
        .unwrap()
        .as_observed
        .unwrap()
        .as_ref()
        .unwrap()
        .stream
        .clone();
    o.configure_source_publication_receipts_v1(l, &pool_control(), &mut r)
        .unwrap();
    (o, r, s, command, stream)
}
pub(in crate::coordinator) fn open(
    o: &mut SourceCoordinator,
    command: &SourcePublicationCommandIdV1,
    s: &SourceReadScopeV1,
    r: &mut Recorder,
) -> SourcePublicationReceiptContextIdV1 {
    let basis = o
        .source_publication_command_receipt_basis_v1(command, s, r)
        .unwrap()
        .basis;
    o.open_source_publication_receipt_v1(
        command,
        s,
        SourcePublicationReceiptCeilingV1 {
            complete_original_result: true,
        },
        &TrustedSourcePublicationReceiptHistoricalFactsV1 {
            basis: &basis,
            current: &facts(s),
        },
        Some(true),
        r,
    )
    .unwrap()
}

#[test]
fn original_result_release_is_separately_authorized_and_claim_survives_stop_until_retirement() {
    for capacity in [0, 1, 64] {
        let (mut o, mut setup, s, command, stream) = setup(limits());
        let original_version = o.read(&s.document, &mut setup).unwrap().version;
        let mut r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let context = open(&mut o, &command, &s, &mut r);
        let debt = o.retention_usage();
        edit(&mut o, &s, "A", "B", &mut setup);
        let n = native(&s);
        let f = stream_facts(&s, &n);
        o.sample_source_publication_stream_v1(&stream, &s, &f, &mut setup)
            .unwrap();
        let before = o.source_publication_usage_v1().unwrap();
        let basis = o
            .source_publication_receipt_basis_v1(&context, &command, &s, &mut setup)
            .unwrap();
        assert_eq!(basis.original_source, Some(&original_version));
        let basis = basis.basis;
        let ResultIdentity::Baseline(weak_result) = &basis.result else {
            panic!()
        };
        let weak_result = weak_result.clone();
        let reply = o
            .release_source_publication_receipt_v1(
                &context,
                &command,
                &s,
                &TrustedSourcePublicationReceiptHistoricalFactsV1 {
                    basis: &basis,
                    current: &facts(&s),
                },
                &mut r,
            )
            .unwrap();
        assert_eq!(reply.command, command);
        assert_eq!(reply.status(), "operation-receipt");
        assert_eq!(reply.result.as_ref().unwrap().projection.text(), "A");
        assert!(Arc::ptr_eq(
            reply.result.as_ref().unwrap(),
            o.source_publication_command_facts_v1(&command, &s, &mut setup)
                .unwrap()
                .as_observed
                .unwrap()
                .as_ref()
                .unwrap()
        ));
        assert_eq!(o.source_publication_usage_v1().unwrap(), before);
        assert_eq!(
            o.document_context_pins_v1(&s.document, &mut setup)
                .unwrap()
                .publication_streams,
            3
        );
        o.stop_source_publication_receipt_v1(&context, &command, &control(&s), &mut r)
            .unwrap();
        assert!(
            o.release_source_publication_receipt_v1(
                &context,
                &command,
                &s,
                &TrustedSourcePublicationReceiptHistoricalFactsV1 {
                    basis: &basis,
                    current: &facts(&s)
                },
                &mut r
            )
            .is_err()
        );
        assert_eq!(o.source_publication_usage_v1().unwrap(), before);
        assert_eq!(
            o.retire_source_publication_command_v1(&command, &control(&s), &mut r),
            Err(Error::Unavailable)
        );
        o.stop_source_publication_stream_v1(&stream, &control(&s), &mut setup)
            .unwrap();
        o.retire_source_publication_stream_v1(&stream, &control(&s), &mut setup)
            .unwrap();
        assert_eq!(o.retention_usage(), debt);
        o.retire_source_publication_receipt_v1(&context, &command, &control(&s), &mut r)
            .unwrap();
        o.retire_source_publication_command_v1(&command, &control(&s), &mut setup)
            .unwrap();
        // The authorized reply is an explicit raw disclosure. Only the caller's
        // snapshot, not its opaque authorization basis, can keep that body alive.
        drop(reply);
        assert!(weak_result.upgrade().is_none());
        assert_eq!(
            o.retire_source_publication_commands_v1(&pool_control(), &mut setup),
            Err(Error::Unavailable)
        );
        o.retire_source_publication_receipts_v1(&pool_control(), &mut setup)
            .unwrap();
        o.retire_source_publication_commands_v1(&pool_control(), &mut setup)
            .unwrap();
        o.retire_source_publication_pool_v1(&pool_control(), &mut setup)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
        if capacity == 64 {
            assert!(
                std::iter::from_fn(|| r.pop_front())
                    .any(|e| e.boundary == Boundary::SourcePublicationReceiptRelease
                        && e.service_time.is_some())
            );
        }
        if capacity == 1 {
            assert!(r.dropped() > 0);
        }
        if capacity == 0 {
            assert_eq!(r.queued(), 0);
        }
    }
}

#[test]
fn stale_foreign_or_missing_authorization_cannot_apply_loss_but_current_denial_is_absorbing() {
    let (mut o, mut r, s, command, _) = setup(limits());
    let one = open(&mut o, &command, &s, &mut r);
    let two = open(&mut o, &command, &s, &mut r);
    let old = o
        .source_publication_receipt_basis_v1(&one, &command, &s, &mut r)
        .unwrap()
        .basis;
    edit(&mut o, &s, "A", "B", &mut r);
    let foreign = o
        .source_publication_receipt_basis_v1(&two, &command, &s, &mut r)
        .unwrap()
        .basis;
    let admission = o
        .source_publication_command_receipt_basis_v1(&command, &s, &mut r)
        .unwrap()
        .basis;
    let mut denied = facts(&s);
    denied.complete_result_allowed = Some(false);
    let mut audit = Recorder::try_with_capacity(64)
        .unwrap()
        .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
    for basis in [&old, &foreign, &admission] {
        assert!(
            o.release_source_publication_receipt_v1(
                &one,
                &command,
                &s,
                &TrustedSourcePublicationReceiptHistoricalFactsV1 {
                    basis,
                    current: &denied
                },
                &mut audit
            )
            .is_err()
        );
        assert_eq!(o.source_publication_receipt_usage_v1().unwrap().stopped, 0);
    }
    let rejected: Vec<_> = std::iter::from_fn(|| audit.pop_front()).collect();
    assert_eq!(
        rejected
            .iter()
            .filter(
                |e| e.boundary == Boundary::SourcePublicationReceiptBasisValidation
                    && e.disposition == Disposition::Refused
                    && e.service_time.is_some()
            )
            .count(),
        3
    );
    assert!(
        !rejected
            .iter()
            .any(|e| e.boundary == Boundary::SourcePublicationReceiptClosure)
    );
    let fresh = o
        .source_publication_receipt_basis_v1(&one, &command, &s, &mut r)
        .unwrap()
        .basis;
    denied.authorization_current = None;
    assert!(
        o.release_source_publication_receipt_v1(
            &one,
            &command,
            &s,
            &TrustedSourcePublicationReceiptHistoricalFactsV1 {
                basis: &fresh,
                current: &denied
            },
            &mut r
        )
        .is_err()
    );
    assert_eq!(o.source_publication_receipt_usage_v1().unwrap().stopped, 0);
    denied.authorization_current = Some(true);
    assert!(
        o.release_source_publication_receipt_v1(
            &one,
            &command,
            &s,
            &TrustedSourcePublicationReceiptHistoricalFactsV1 {
                basis: &fresh,
                current: &denied
            },
            &mut audit
        )
        .is_err()
    );
    assert_eq!(o.source_publication_receipt_usage_v1().unwrap().stopped, 1);
    assert_eq!(
        std::iter::from_fn(|| audit.pop_front())
            .filter(|e| e.boundary == Boundary::SourcePublicationReceiptClosure
                && e.disposition == Disposition::Completed)
            .count(),
        1
    );
    assert!(
        o.release_source_publication_receipt_v1(
            &one,
            &command,
            &s,
            &TrustedSourcePublicationReceiptHistoricalFactsV1 {
                basis: &fresh,
                current: &facts(&s)
            },
            &mut r
        )
        .is_err()
    );
    assert!(
        o.release_source_publication_receipt_v1(
            &two,
            &command,
            &s,
            &TrustedSourcePublicationReceiptHistoricalFactsV1 {
                basis: &foreign,
                current: &facts(&s)
            },
            &mut r
        )
        .is_ok()
    );
    assert_eq!(o.source_publication_usage_v1().unwrap().stopped, 0);
}

#[test]
fn ceiling_reply_and_lifetime_bounds_fail_before_claim_or_payload_and_do_not_recycle_contexts() {
    let mut l = limits();
    l.max_contexts = 1;
    l.max_issued = 2;
    let (mut o, mut r, s, command, _) = setup(l);
    let basis = o
        .source_publication_command_receipt_basis_v1(&command, &s, &mut r)
        .unwrap()
        .basis;
    assert_eq!(
        o.open_source_publication_receipt_v1(
            &command,
            &s,
            SourcePublicationReceiptCeilingV1 {
                complete_original_result: false
            },
            &TrustedSourcePublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &facts(&s)
            },
            Some(true),
            &mut r
        )
        .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(o.source_publication_receipt_usage_v1().unwrap().contexts, 0);
    let one = open(&mut o, &command, &s, &mut r);
    assert_eq!(
        o.open_source_publication_receipt_v1(
            &command,
            &s,
            SourcePublicationReceiptCeilingV1 {
                complete_original_result: true
            },
            &TrustedSourcePublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &facts(&s)
            },
            Some(true),
            &mut r
        )
        .unwrap_err(),
        Error::Limit
    );
    let release = o
        .source_publication_receipt_basis_v1(&one, &command, &s, &mut r)
        .unwrap()
        .basis;
    let mut small = facts(&s);
    small.max_reply_bytes = 1;
    assert_eq!(
        o.release_source_publication_receipt_v1(
            &one,
            &command,
            &s,
            &TrustedSourcePublicationReceiptHistoricalFactsV1 {
                basis: &release,
                current: &small
            },
            &mut r
        )
        .unwrap_err(),
        Error::Limit
    );
    assert_eq!(o.source_publication_receipt_usage_v1().unwrap().stopped, 0);
    small.max_reply_bytes = usize::MAX;
    assert!(
        o.release_source_publication_receipt_v1(
            &one,
            &command,
            &s,
            &TrustedSourcePublicationReceiptHistoricalFactsV1 {
                basis: &release,
                current: &small
            },
            &mut r
        )
        .is_ok()
    );
    assert_eq!(
        o.retire_source_publication_receipt_v1(&one, &command, &control(&s), &mut r),
        Err(Error::Unavailable)
    );
    o.stop_source_publication_receipt_v1(&one, &command, &control(&s), &mut r)
        .unwrap();
    o.retire_source_publication_receipt_v1(&one, &command, &control(&s), &mut r)
        .unwrap();
    let two = open(&mut o, &command, &s, &mut r);
    assert_ne!(one, two);
    o.stop_source_publication_receipt_v1(&two, &command, &control(&s), &mut r)
        .unwrap();
    o.retire_source_publication_receipt_v1(&two, &command, &control(&s), &mut r)
        .unwrap();
    assert_eq!(
        o.open_source_publication_receipt_v1(
            &command,
            &s,
            SourcePublicationReceiptCeilingV1 {
                complete_original_result: true
            },
            &TrustedSourcePublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &facts(&s)
            },
            Some(true),
            &mut r
        )
        .unwrap_err(),
        Error::Limit
    );
    assert_eq!(
        o.source_publication_receipt_usage_v1()
            .unwrap()
            .issued_lifetime,
        2
    );
}

#[test]
fn real_reclamation_and_protected_cleanup_respect_receipt_claims_without_ordinary_authority() {
    let (mut o, mut r, s, command, stream) = setup(limits());
    let context = open(&mut o, &command, &s, &mut r);
    o.stop_source_publication_stream_v1(&stream, &control(&s), &mut r)
        .unwrap();
    o.retire_source_publication_stream_v1(&stream, &control(&s), &mut r)
        .unwrap();
    let version = o.read(&s.document, &mut r).unwrap().version;
    let p = o.prepare_retained_disposal(&version, 0, &mut r).unwrap();
    let state = o.commit_retained_disposal(p, &mut r).unwrap();
    o.release_disposed_provenance(&s.document, state, &mut r)
        .unwrap();
    let state = o.lifecycle_state(&s.document, &mut r).unwrap();
    let f = TrustedDocumentReclamationV1 {
        authorized: Some(true),
        authored_disposition_ready: Some(true),
        session_references_released: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    assert_eq!(
        o.reclaim_disposed_document_v1(&s.document, state, &f, &mut r),
        Err(CoordinatorError::Resource(
            crate::resource::ResourceError::Retained
        ))
    );
    o.faulted = true;
    o.stop_source_publication_receipt_v1(&context, &command, &control(&s), &mut r)
        .unwrap();
    assert_eq!(
        o.retire_source_publication_command_v1(&command, &control(&s), &mut r),
        Err(Error::Unavailable)
    );
    o.retire_source_publication_receipt_v1(&context, &command, &control(&s), &mut r)
        .unwrap();
    o.retire_source_publication_command_v1(&command, &control(&s), &mut r)
        .unwrap();
    assert!(o.faulted);
    o.faulted = false; // Test resumes native reclamation; cleanup did not heal it.
    o.reclaim_disposed_document_v1(&s.document, state, &f, &mut r)
        .unwrap();
    o.retire_source_publication_receipts_v1(&pool_control(), &mut r)
        .unwrap();
    o.retire_source_publication_commands_v1(&pool_control(), &mut r)
        .unwrap();
    o.retire_source_publication_pool_v1(&pool_control(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}

#[test]
fn orphaned_receipt_or_lost_budget_is_not_a_zero_pin_or_command_discharge() {
    for orphan in [false, true] {
        let (mut o, mut r, s, command, _) = setup(limits());
        open(&mut o, &command, &s, &mut r);
        let debt = o.retention_usage();
        let super::super::super::State::Active(p) = &mut o.publication_streams else {
            panic!()
        };
        let super::super::State::Active(t) = &mut p.commands else {
            panic!()
        };
        if orphan {
            t.rows[command.slot] = None;
        } else {
            let State::Active(p) = &mut t.receipts else {
                panic!()
            };
            p.charge += 1;
        }
        assert_eq!(
            o.document_context_pins_v1(&s.document, &mut r),
            Err(CoordinatorError::Faulted)
        );
        assert_eq!(o.source_publication_receipt_usage_v1(), Err(Error::Faulted));
        assert_eq!(
            o.retire_source_publication_command_v1(&command, &control(&s), &mut r),
            Err(Error::Faulted)
        );
        assert_eq!(o.retention_usage(), debt);
    }
}

#[test]
fn receipt_preserves_known_native_error_as_original_result_not_outer_authorization_failure() {
    let (mut o, mut r, s) = owner(stream_limits());
    o.configure_source_publication_commands_v1(command_limits(), &pool_control(), &mut r)
        .unwrap();
    let n = native(&s);
    let f = stream_facts(&s, &n);
    let op = SourcePublicationCommandOperationV1::Establish;
    let command = o
        .issue_source_publication_command_v1(&op, &s, &f, Some(true), &mut r)
        .unwrap();
    // Known-returned-error producer fixture, not a real allocator failure claim.
    let c = o.source_command_mut_v1(&command);
    c.life.begin();
    c.life.settle(Err(Error::Capacity));
    o.configure_source_publication_receipts_v1(limits(), &pool_control(), &mut r)
        .unwrap();
    let context = open(&mut o, &command, &s, &mut r);
    let input = o
        .source_publication_receipt_basis_v1(&context, &command, &s, &mut r)
        .unwrap();
    assert_eq!(input.native_error, Some(Error::Capacity));
    assert!(input.original_source.is_none());
    let basis = input.basis;
    let reply = o
        .release_source_publication_receipt_v1(
            &context,
            &command,
            &s,
            &TrustedSourcePublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &facts(&s),
            },
            &mut r,
        )
        .unwrap();
    assert_eq!(reply.result.unwrap_err(), Error::Capacity);
    assert_eq!(o.source_publication_usage_v1().unwrap().streams, 0);
}
