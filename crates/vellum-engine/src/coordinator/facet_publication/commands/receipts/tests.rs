use super::*;
#[test]
fn publication_admission_fence_facet_preserves_existing_receipt_replay_and_protected_cleanup() {
    for mode in 0..4 {
        let (mut o, mut setup, s, command) = setup_with_budget(8388608);
        let context = open(&mut o, &command, &s, &mut setup);
        let n = crate::coordinator::publication_stream::tests::native(&s.source);
        let g = grants(&s);
        let v: Vec<_> = s.views.iter().map(|v| v.view.clone()).collect();
        let f = stream_facts(&s, &n, &g, &v);
        let op = FacetPublicationCommandOperationV1::Establish;
        let pending = o
            .issue_facet_publication_command_v1(&op, &s, &f, Some(true), &mut setup)
            .unwrap();
        crate::coordinator::shutdown::tests::install(&mut o, &mut setup);
        let before = (
            o.facet_publication_usage_v1().unwrap(),
            o.facet_publication_command_usage_v1().unwrap(),
            o.facet_publication_receipt_usage_v1().unwrap(),
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
            o.issue_facet_publication_command_v1(&op, &s, &f, Some(true), &mut r),
            Err(Error::Unavailable)
        );
        assert!(matches!(
            o.open_facet_publication_stream_v1(&s, &f, &mut r),
            Err(Error::Unavailable)
        ));
        let mut q = FacetPublicationCommandRequestV1 {
            scope: &s,
            operation: &op,
            current: Some(&f),
            owner_control_allowed: Some(true),
            max_reply_bytes: 65536,
        };
        assert_eq!(
            o.request_facet_publication_command_v1(&pending, &q, &mut r),
            Err(Error::Unavailable)
        );
        assert!(
            !o.facet_publication_command_facts_v1(&pending, &s, &mut setup)
                .unwrap()
                .native_started
        );
        let basis = o
            .facet_publication_command_receipt_basis_v1(&command, &s, &mut setup)
            .unwrap()
            .basis;
        assert_eq!(
            o.open_facet_publication_receipt_v1(
                &command,
                &s,
                FacetPublicationReceiptCeilingV1 {
                    complete_original_result: true
                },
                &TrustedFacetPublicationReceiptHistoricalFactsV1 {
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
                o.facet_publication_usage_v1().unwrap(),
                o.facet_publication_command_usage_v1().unwrap(),
                o.facet_publication_receipt_usage_v1().unwrap(),
                o.retention_usage()
            ),
            before
        );
        q.current = None;
        assert!(matches!(
            o.request_facet_publication_command_v1(&command, &q, &mut r),
            Ok(FacetPublicationCommandReplyV1::ObserveExisting { .. })
        ));
        let basis = o
            .facet_publication_receipt_basis_v1(&context, &command, &s, &mut setup)
            .unwrap()
            .basis;
        assert!(
            o.release_facet_publication_receipt_v1(
                &context,
                &command,
                &s,
                &TrustedFacetPublicationReceiptHistoricalFactsV1 {
                    basis: &basis,
                    current: &facts(&s)
                },
                &mut r
            )
            .is_ok()
        );
        o.faulted = true;
        o.stop_facet_publication_receipt_v1(&context, &command, &cleanup(&s), &mut r)
            .unwrap();
        o.retire_facet_publication_receipt_v1(&context, &command, &cleanup(&s), &mut r)
            .unwrap();
        o.retire_facet_publication_command_v1(&command, &cleanup(&s), &mut r)
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
                if e.boundary == Boundary::FacetPublicationCommandRequest {
                    assert_eq!(e.service_time.is_some(), mode == 2);
                    ends.push(e.disposition);
                }
            }
            assert_eq!(ends, [Disposition::Refused, Disposition::Completed]);
            assert_eq!(r.dropped(), 0);
        }
    }
}
use crate::coordinator::facet_publication::tests::{
    cleanup, ctl, facts as stream_facts, fixture_with_budget, grants,
};
use crate::instrumentation::NativeTimingProfileV1;

pub(in crate::coordinator) fn open(
    o: &mut SourceCoordinator,
    command: &FacetPublicationCommandIdV1,
    s: &FacetPublicationScopeV1,
    r: &mut Recorder,
) -> FacetPublicationReceiptContextIdV1 {
    let basis = o
        .facet_publication_command_receipt_basis_v1(command, s, r)
        .unwrap()
        .basis;
    o.open_facet_publication_receipt_v1(
        command,
        s,
        FacetPublicationReceiptCeilingV1 {
            complete_original_result: true,
        },
        &TrustedFacetPublicationReceiptHistoricalFactsV1 {
            basis: &basis,
            current: &facts(s),
        },
        Some(true),
        r,
    )
    .unwrap()
}

#[test]
fn facet_receipt_unknown_and_transient_pressure_withhold_but_attributed_loss_closes_only_original_context()
 {
    let (mut o, mut r, s, command) = setup();
    let one = open(&mut o, &command, &s, &mut r);
    let two = open(&mut o, &command, &s, &mut r);
    let basis = o
        .facet_publication_receipt_basis_v1(&one, &command, &s, &mut r)
        .unwrap()
        .basis;
    let mut f = facts(&s);
    f.complete_result_allowed = Some(false);
    // Even explicit supplied loss is not attributable to a different context.
    assert!(matches!(
        o.release_facet_publication_receipt_v1(
            &two,
            &command,
            &s,
            &TrustedFacetPublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &f
            },
            &mut r
        ),
        Err(Error::Unavailable)
    ));
    assert_eq!(o.facet_publication_receipt_usage_v1().unwrap().stopped, 0);
    f.authorization_current = None;
    assert!(matches!(
        o.release_facet_publication_receipt_v1(
            &one,
            &command,
            &s,
            &TrustedFacetPublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &f
            },
            &mut r
        ),
        Err(Error::Unavailable)
    ));
    assert_eq!(o.facet_publication_receipt_usage_v1().unwrap().stopped, 0);
    f = facts(&s);
    f.transfer_ready = Some(false);
    assert!(matches!(
        o.release_facet_publication_receipt_v1(
            &one,
            &command,
            &s,
            &TrustedFacetPublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &f
            },
            &mut r
        ),
        Err(Error::Unavailable)
    ));
    assert_eq!(o.facet_publication_receipt_usage_v1().unwrap().stopped, 0);
    f = facts(&s);
    f.complete_result_allowed = Some(false);
    assert!(matches!(
        o.release_facet_publication_receipt_v1(
            &one,
            &command,
            &s,
            &TrustedFacetPublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &f
            },
            &mut r
        ),
        Err(Error::Unavailable)
    ));
    assert_eq!(o.facet_publication_receipt_usage_v1().unwrap().stopped, 1);
    assert!(
        o.release_facet_publication_receipt_v1(
            &one,
            &command,
            &s,
            &TrustedFacetPublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &facts(&s)
            },
            &mut r
        )
        .is_err()
    );
    let other_basis = o
        .facet_publication_receipt_basis_v1(&two, &command, &s, &mut r)
        .unwrap()
        .basis;
    assert!(
        o.release_facet_publication_receipt_v1(
            &two,
            &command,
            &s,
            &TrustedFacetPublicationReceiptHistoricalFactsV1 {
                basis: &other_basis,
                current: &facts(&s)
            },
            &mut r
        )
        .is_ok()
    );
}

#[test]
fn facet_receipt_stale_or_foreign_basis_cannot_apply_loss_and_current_basis_releases_historical_projection()
 {
    let (mut o, mut r, s, command) = setup();
    let context = open(&mut o, &command, &s, &mut r);
    let original = o
        .facet_publication_command_facts_v1(&command, &s, &mut r)
        .unwrap()
        .as_observed
        .unwrap()
        .as_ref()
        .unwrap()
        .clone();
    assert!(original.projection.source().is_none());
    assert_eq!(original.projection.annotations().len(), 1);
    assert_eq!(original.projection.views().len(), 1);
    let old = o
        .facet_publication_receipt_basis_v1(&context, &command, &s, &mut r)
        .unwrap()
        .basis;
    crate::coordinator::facet_publication::tests::edit(&mut o, &s.source, &mut r);
    let mut f = facts(&s);
    f.complete_result_allowed = Some(false);
    assert!(matches!(
        o.release_facet_publication_receipt_v1(
            &context,
            &command,
            &s,
            &TrustedFacetPublicationReceiptHistoricalFactsV1 {
                basis: &old,
                current: &f
            },
            &mut r
        ),
        Err(Error::Unavailable)
    ));
    assert_eq!(o.facet_publication_receipt_usage_v1().unwrap().stopped, 0);
    let (foreign, mut foreign_r, foreign_s, foreign_command) = setup();
    let wrong = foreign
        .facet_publication_command_receipt_basis_v1(&foreign_command, &foreign_s, &mut foreign_r)
        .unwrap()
        .basis;
    assert!(
        o.release_facet_publication_receipt_v1(
            &context,
            &command,
            &s,
            &TrustedFacetPublicationReceiptHistoricalFactsV1 {
                basis: &wrong,
                current: &f
            },
            &mut r
        )
        .is_err()
    );
    assert_eq!(o.facet_publication_receipt_usage_v1().unwrap().stopped, 0);
    let current = o
        .facet_publication_receipt_basis_v1(&context, &command, &s, &mut r)
        .unwrap();
    assert!(std::ptr::eq(
        current.original_projection.unwrap(),
        original.projection.as_ref()
    ));
    let current = current.basis;
    let reply = o
        .release_facet_publication_receipt_v1(
            &context,
            &command,
            &s,
            &TrustedFacetPublicationReceiptHistoricalFactsV1 {
                basis: &current,
                current: &facts(&s),
            },
            &mut r,
        )
        .unwrap();
    assert!(Arc::ptr_eq(&reply.result.unwrap(), &original));
}

#[test]
fn facet_receipt_all_authority_gates_are_exact_and_reply_pressure_never_spends_a_context() {
    let (mut o, mut r, s, command) = setup();
    let basis = o
        .facet_publication_command_receipt_basis_v1(&command, &s, &mut r)
        .unwrap()
        .basis;
    let before = o.facet_publication_receipt_usage_v1().unwrap();
    let debt = o.retention_usage();
    for gate in 0..9 {
        for value in [None, Some(false)] {
            let mut f = facts(&s);
            match gate {
                0 => f.authorized = value,
                1 => f.scope_current = value,
                2 => f.continuous = value,
                3 => f.live = value,
                4 => f.not_stopped = value,
                5 => f.retained = value,
                6 => f.receipt_grant_active = value,
                7 => f.complete_result_allowed = value,
                8 => f.authorization_current = value,
                _ => unreachable!(),
            }
            assert_eq!(
                o.open_facet_publication_receipt_v1(
                    &command,
                    &s,
                    FacetPublicationReceiptCeilingV1 {
                        complete_original_result: true
                    },
                    &TrustedFacetPublicationReceiptHistoricalFactsV1 {
                        basis: &basis,
                        current: &f
                    },
                    Some(true),
                    &mut r
                ),
                Err(Error::Unavailable)
            );
            assert_eq!(o.facet_publication_receipt_usage_v1().unwrap(), before);
            assert_eq!(o.retention_usage(), debt);
        }
    }
    let context = open(&mut o, &command, &s, &mut r);
    let basis = o
        .facet_publication_receipt_basis_v1(&context, &command, &s, &mut r)
        .unwrap()
        .basis;
    let original = o
        .facet_publication_command_facts_v1(&command, &s, &mut r)
        .unwrap()
        .as_observed
        .unwrap();
    let exact = reply_bytes(original).unwrap();
    let mut f = facts(&s);
    f.max_reply_bytes = exact - 1;
    assert!(matches!(
        o.release_facet_publication_receipt_v1(
            &context,
            &command,
            &s,
            &TrustedFacetPublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &f
            },
            &mut r
        ),
        Err(Error::Limit)
    ));
    assert_eq!(o.facet_publication_receipt_usage_v1().unwrap().stopped, 0);
    f.max_reply_bytes = exact;
    assert!(
        o.release_facet_publication_receipt_v1(
            &context,
            &command,
            &s,
            &TrustedFacetPublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &f
            },
            &mut r
        )
        .is_ok()
    );
}

#[test]
fn facet_receipt_shutdown_pins_and_parent_funding_survive_stop_and_refund_only_on_pool_retirement()
{
    let (mut o, mut r, s, command) = setup();
    let context = open(&mut o, &command, &s, &mut r);
    let charge = o
        .facet_publication_receipt_usage_v1()
        .unwrap()
        .reserved_pool_bytes;
    let command_charge = o
        .facet_publication_command_usage_v1()
        .unwrap()
        .reserved_pool_bytes;
    let debt = o.retention_usage();
    let row = crate::coordinator::shutdown::tests::inventory_owner(
        &o,
        &mut r,
        ShutdownOwnerRoleV1::FacetPublication,
        &[&s.source.document],
    );
    assert_eq!(row.retained_records, Some(3));
    assert_eq!(row.additional_reserved_bytes, Some(charge + command_charge));
    assert_eq!(
        o.document_context_pins_v1(&s.source.document, &mut r)
            .unwrap()
            .publication_streams,
        3
    );
    o.stop_facet_publication_receipt_v1(&context, &command, &cleanup(&s), &mut r)
        .unwrap();
    assert_eq!(
        o.document_context_pins_v1(&s.source.document, &mut r)
            .unwrap()
            .publication_streams,
        3
    );
    assert_eq!(o.retention_usage(), debt);
    let mut control = cleanup(&s);
    control.delivery_drained = None;
    assert_eq!(
        o.retire_facet_publication_receipt_v1(&context, &command, &control, &mut r),
        Err(Error::Unavailable)
    );
    o.retire_facet_publication_receipt_v1(&context, &command, &cleanup(&s), &mut r)
        .unwrap();
    assert_eq!(
        o.document_context_pins_v1(&s.source.document, &mut r)
            .unwrap()
            .publication_streams,
        2
    );
    assert_eq!(o.retention_usage(), debt);
    o.retire_facet_publication_receipts_v1(&ctl(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, debt.reserved - charge);
    assert_eq!(
        o.retire_facet_publication_receipts_v1(&ctl(), &mut r),
        Err(Error::Unavailable)
    );
    assert_eq!(o.retention_usage().reserved, debt.reserved - charge);
}

#[test]
fn facet_receipt_recording_modes_preserve_delivery_identity_and_native_authority_refusal() {
    for mode in 0..4 {
        let (mut o, mut setup, s, command) = setup();
        let context = open(&mut o, &command, &s, &mut setup);
        let basis = o
            .facet_publication_receipt_basis_v1(&context, &command, &s, &mut setup)
            .unwrap()
            .basis;
        let debt = o.retention_usage();
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 32 }).unwrap()
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
        let mut f = facts(&s);
        f.reply_ready = None;
        assert!(
            o.release_facet_publication_receipt_v1(
                &context,
                &command,
                &s,
                &TrustedFacetPublicationReceiptHistoricalFactsV1 {
                    basis: &basis,
                    current: &f
                },
                &mut r
            )
            .is_err()
        );
        f.reply_ready = Some(true);
        let reply = o
            .release_facet_publication_receipt_v1(
                &context,
                &command,
                &s,
                &TrustedFacetPublicationReceiptHistoricalFactsV1 {
                    basis: &basis,
                    current: &f,
                },
                &mut r,
            )
            .unwrap();
        assert_eq!(reply.command, command);
        assert_eq!(o.retention_usage(), debt);
        assert_eq!(o.facet_publication_receipt_usage_v1().unwrap().stopped, 0);
        if mode == 0 {
            assert_eq!(r.queued(), 0);
            assert_eq!(r.dropped(), 0);
        } else if mode == 3 {
            assert_eq!(r.pop_front(), Some(sentinel));
            assert!(r.dropped() > 0);
        } else {
            let mut ends = Vec::new();
            while let Some(e) = r.pop_front() {
                if e.boundary == Boundary::FacetPublicationReceiptRelease {
                    assert_eq!(e.service_time.is_some(), mode == 2);
                    ends.push(e.disposition);
                }
            }
            assert_eq!(ends, [Disposition::Refused, Disposition::Completed]);
        }
    }
}

pub(in crate::coordinator) fn facts(
    s: &FacetPublicationScopeV1,
) -> TrustedFacetPublicationReceiptFactsV1<'_> {
    TrustedFacetPublicationReceiptFactsV1 {
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
        max_reply_bytes: 65536,
    }
}
pub(in crate::coordinator) fn setup() -> (
    SourceCoordinator,
    Recorder,
    FacetPublicationScopeV1,
    FacetPublicationCommandIdV1,
) {
    setup_with_budget(1048576)
}
pub(in crate::coordinator) fn setup_with_budget(
    maximum: usize,
) -> (
    SourceCoordinator,
    Recorder,
    FacetPublicationScopeV1,
    FacetPublicationCommandIdV1,
) {
    let (mut o, mut r, s) = fixture_with_budget(false, true, true, true, maximum);
    let mut c = FacetPublicationCommandLimitsV1 {
        max_commands: 4,
        max_issued: 8,
        max_context_bytes: 8192,
        max_result_bytes: 65536,
        max_pool_bytes: usize::MAX,
    };
    c.max_pool_bytes = c.required_pool_bytes().unwrap();
    o.configure_facet_publication_commands_v1(c, &ctl(), &mut r)
        .unwrap();
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let g = grants(&s);
    let v: Vec<_> = s.views.iter().map(|v| v.view.clone()).collect();
    let f = stream_facts(&s, &n, &g, &v);
    let op = FacetPublicationCommandOperationV1::Establish;
    let command = o
        .issue_facet_publication_command_v1(&op, &s, &f, Some(true), &mut r)
        .unwrap();
    o.request_facet_publication_command_v1(
        &command,
        &FacetPublicationCommandRequestV1 {
            scope: &s,
            operation: &op,
            current: Some(&f),
            owner_control_allowed: Some(true),
            max_reply_bytes: 65536,
        },
        &mut r,
    )
    .unwrap();
    let mut l = FacetPublicationReceiptLimitsV1 {
        max_contexts: 2,
        max_issued: 8,
        max_context_bytes: 8192,
        max_reply_bytes: 65536,
        max_pool_bytes: usize::MAX,
    };
    l.max_pool_bytes = l.required_pool_bytes().unwrap();
    o.configure_facet_publication_receipts_v1(l, &ctl(), &mut r)
        .unwrap();
    (o, r, s, command)
}
#[test]
fn facet_receipt_requires_separate_grant_and_retains_exact_mixed_projection_until_cleanup() {
    let (mut o, mut r, s, command) = setup();
    let original = o
        .facet_publication_command_facts_v1(&command, &s, &mut r)
        .unwrap()
        .as_observed
        .unwrap()
        .as_ref()
        .unwrap()
        .clone();
    let input = o
        .facet_publication_command_receipt_basis_v1(&command, &s, &mut r)
        .unwrap();
    assert!(std::ptr::eq(
        input.original_projection.unwrap(),
        original.projection.as_ref()
    ));
    let basis = input.basis;
    let mut f = facts(&s);
    f.complete_result_allowed = Some(false);
    let ceiling = FacetPublicationReceiptCeilingV1 {
        complete_original_result: true,
    };
    assert_eq!(
        o.open_facet_publication_receipt_v1(
            &command,
            &s,
            ceiling,
            &TrustedFacetPublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &f
            },
            Some(true),
            &mut r
        ),
        Err(Error::Unavailable)
    );
    f.complete_result_allowed = Some(true);
    let context = o
        .open_facet_publication_receipt_v1(
            &command,
            &s,
            ceiling,
            &TrustedFacetPublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &f,
            },
            Some(true),
            &mut r,
        )
        .unwrap();
    assert_eq!(
        o.retire_facet_publication_command_v1(&command, &cleanup(&s), &mut r),
        Err(Error::Unavailable)
    );
    let basis = o
        .facet_publication_receipt_basis_v1(&context, &command, &s, &mut r)
        .unwrap()
        .basis;
    let reply = o
        .release_facet_publication_receipt_v1(
            &context,
            &command,
            &s,
            &TrustedFacetPublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &f,
            },
            &mut r,
        )
        .unwrap();
    assert_eq!(reply.command, command);
    assert!(Arc::ptr_eq(&reply.result.unwrap(), &original));
    o.stop_facet_publication_receipt_v1(&context, &command, &cleanup(&s), &mut r)
        .unwrap();
    assert_eq!(
        o.retire_facet_publication_command_v1(&command, &cleanup(&s), &mut r),
        Err(Error::Unavailable)
    );
    o.retire_facet_publication_receipt_v1(&context, &command, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_facet_publication_receipts_v1(&ctl(), &mut r)
        .unwrap();
    o.retire_facet_publication_command_v1(&command, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_facet_publication_commands_v1(&ctl(), &mut r)
        .unwrap();
}
