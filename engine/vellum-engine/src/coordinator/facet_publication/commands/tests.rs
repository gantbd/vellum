use super::super::tests::{cleanup, edit};
use super::super::tests::{ctl, facts, fixture, grants};
use super::*;
#[test]
fn publication_admission_fence_blocks_facet_command_and_receipt_pool_configuration() {
    for receipt in [false, true] {
        let (mut o, mut r, _) = source_fixture();
        if receipt {
            configure(&mut o, &mut r);
        }
        crate::coordinator::shutdown::tests::install(&mut o, &mut r);
        let debt = o.retention_usage();
        let result = if receipt {
            o.configure_facet_publication_receipts_v1(
                FacetPublicationReceiptLimitsV1 {
                    max_contexts: 2,
                    max_issued: 8,
                    max_context_bytes: 8192,
                    max_reply_bytes: 65536,
                    max_pool_bytes: 524288,
                },
                &ctl(),
                &mut r,
            )
        } else {
            o.configure_facet_publication_commands_v1(
                FacetPublicationCommandLimitsV1 {
                    max_commands: 4,
                    max_issued: 8,
                    max_context_bytes: 8192,
                    max_result_bytes: 65536,
                    max_pool_bytes: 524288,
                },
                &ctl(),
                &mut r,
            )
        };
        assert_eq!(result, Err(Error::Unavailable));
        assert_eq!(o.retention_usage(), debt);
    }
}
use crate::instrumentation::NativeTimingProfileV1;

fn source_fixture() -> (SourceCoordinator, Recorder, FacetPublicationScopeV1) {
    let (o, r, mut s) = fixture(true, false, false, false);
    s.annotations.clear();
    (o, r, s)
}

fn configure(o: &mut SourceCoordinator, r: &mut Recorder) -> usize {
    let mut limits = FacetPublicationCommandLimitsV1 {
        max_commands: 4,
        max_issued: 8,
        max_context_bytes: 8192,
        max_result_bytes: 65536,
        max_pool_bytes: usize::MAX,
    };
    limits.max_pool_bytes = limits.required_pool_bytes().unwrap();
    o.configure_facet_publication_commands_v1(limits, &ctl(), r)
        .unwrap();
    limits.max_pool_bytes
}

#[test]
fn facet_command_known_native_refusal_is_a_retained_result_not_a_retriable_new_attempt() {
    let (mut o, mut r, s) = fixture(false, true, false, false);
    configure(&mut o, &mut r);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let g = grants(&s);
    let mut f = facts(&s, &n, &[], &[]);
    let op = FacetPublicationCommandOperationV1::Establish;
    let usage = o.facet_publication_command_usage_v1().unwrap();
    assert_eq!(
        o.issue_facet_publication_command_v1(&op, &s, &f, Some(true), &mut r),
        Err(Error::Unavailable)
    );
    assert_eq!(o.facet_publication_command_usage_v1().unwrap(), usage);
    f.annotation_grants = &g;
    let id = o
        .issue_facet_publication_command_v1(&op, &s, &f, Some(true), &mut r)
        .unwrap();
    f.max_reply_bytes = 0;
    let q = FacetPublicationCommandRequestV1 {
        scope: &s,
        operation: &op,
        current: Some(&f),
        owner_control_allowed: Some(true),
        max_reply_bytes: 65536,
    };
    assert!(matches!(
        o.request_facet_publication_command_v1(&id, &q, &mut r),
        Ok(FacetPublicationCommandReplyV1::Settled { .. })
    ));
    let observed = o
        .facet_publication_command_facts_v1(&id, &s, &mut r)
        .unwrap();
    assert!(matches!(observed.as_observed, Some(Err(Error::Limit))));
    assert!(observed.native_started);
    assert_eq!(o.facet_publication_usage_v1().unwrap().streams, 0);
    let retry = FacetPublicationCommandRequestV1 { current: None, ..q };
    assert!(matches!(
        o.request_facet_publication_command_v1(&id, &retry, &mut r),
        Ok(FacetPublicationCommandReplyV1::ObserveExisting { .. })
    ));
    assert_eq!(o.facet_publication_usage_v1().unwrap().streams, 0);
}

#[test]
fn facet_command_replay_preserves_original_baseline_without_allocating_another_stream() {
    let (mut o, mut r, s) = fixture(false, true, false, false);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let g = grants(&s);
    let f = facts(&s, &n, &g, &[]);
    let mut limits = FacetPublicationCommandLimitsV1 {
        max_commands: 4,
        max_issued: 8,
        max_context_bytes: 8192,
        max_result_bytes: 65536,
        max_pool_bytes: usize::MAX,
    };
    limits.max_pool_bytes = limits.required_pool_bytes().unwrap();
    o.configure_facet_publication_commands_v1(limits, &ctl(), &mut r)
        .unwrap();
    let op = FacetPublicationCommandOperationV1::Establish;
    let id = o
        .issue_facet_publication_command_v1(&op, &s, &f, Some(true), &mut r)
        .unwrap();
    let q = FacetPublicationCommandRequestV1 {
        scope: &s,
        operation: &op,
        current: Some(&f),
        owner_control_allowed: Some(true),
        max_reply_bytes: 65536,
    };
    assert!(matches!(
        o.request_facet_publication_command_v1(&id, &q, &mut r),
        Ok(FacetPublicationCommandReplyV1::Settled { .. })
    ));
    let first = o
        .facet_publication_command_facts_v1(&id, &s, &mut r)
        .unwrap()
        .as_observed
        .unwrap()
        .as_ref()
        .unwrap()
        .clone();
    let usage = o.facet_publication_usage_v1().unwrap();
    let debt = o.retention_usage();
    let retry = FacetPublicationCommandRequestV1 { current: None, ..q };
    assert!(matches!(
        o.request_facet_publication_command_v1(&id, &retry, &mut r),
        Ok(FacetPublicationCommandReplyV1::ObserveExisting { .. })
    ));
    let again = o
        .facet_publication_command_facts_v1(&id, &s, &mut r)
        .unwrap()
        .as_observed
        .unwrap()
        .as_ref()
        .unwrap()
        .clone();
    assert!(Arc::ptr_eq(&first, &again));
    assert_eq!(o.facet_publication_usage_v1().unwrap(), usage);
    assert_eq!(o.retention_usage(), debt);
}

#[test]
fn facet_command_resynchronization_binds_original_generation_and_subsequent_event_chain() {
    let (mut o, mut r, s) = source_fixture();
    configure(&mut o, &mut r);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let f = facts(&s, &n, &[], &[]);
    let opened = o.open_facet_publication_stream_v1(&s, &f, &mut r).unwrap();
    let op = FacetPublicationCommandOperationV1::Resynchronize {
        stream: opened.stream.clone(),
    };
    let id = o
        .issue_facet_publication_command_v1(&op, &s, &f, Some(true), &mut r)
        .unwrap();
    let q = FacetPublicationCommandRequestV1 {
        scope: &s,
        operation: &op,
        current: Some(&f),
        owner_control_allowed: Some(true),
        max_reply_bytes: 65536,
    };
    o.request_facet_publication_command_v1(&id, &q, &mut r)
        .unwrap();
    let baseline = o
        .facet_publication_command_facts_v1(&id, &s, &mut r)
        .unwrap()
        .as_observed
        .unwrap()
        .as_ref()
        .unwrap()
        .clone();
    assert_eq!(baseline.stream, opened.stream);
    assert_eq!(baseline.generation.serial, opened.generation.serial + 1);
    edit(&mut o, &s.source, &mut r);
    o.sample_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r)
        .unwrap();
    let ObservationEventDeliveryV1::NamedFacets(FacetPublicationDeliveryV1::Projection(event)) = o
        .release_observation_event_v1(
            ObservationEventTargetV1::NamedFacets(&opened.stream),
            ObservationEventPortV1::NamedFacets {
                stream: &opened.stream,
                scope: &s,
                facts: &f,
            },
            &mut r,
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(event.before, baseline.generation);
    let issued = o.facet_publication_usage_v1().unwrap().issued_lifetime;
    let retry = FacetPublicationCommandRequestV1 { current: None, ..q };
    assert!(matches!(
        o.request_facet_publication_command_v1(&id, &retry, &mut r),
        Ok(FacetPublicationCommandReplyV1::ObserveExisting { .. })
    ));
    assert_eq!(
        o.facet_publication_usage_v1().unwrap().issued_lifetime,
        issued
    );
    assert!(Arc::ptr_eq(
        &baseline,
        o.facet_publication_command_facts_v1(&id, &s, &mut r)
            .unwrap()
            .as_observed
            .unwrap()
            .as_ref()
            .unwrap()
    ));
}

#[test]
fn facet_command_refuses_stale_root_or_generation_before_native_service_and_has_no_grant_bypass() {
    for stale_root in [false, true] {
        let (mut o, mut r, s) = source_fixture();
        configure(&mut o, &mut r);
        let n = crate::coordinator::publication_stream::tests::native(&s.source);
        let mut f = facts(&s, &n, &[], &[]);
        let opened = o.open_facet_publication_stream_v1(&s, &f, &mut r).unwrap();
        let op = FacetPublicationCommandOperationV1::Resynchronize {
            stream: opened.stream.clone(),
        };
        let id = o
            .issue_facet_publication_command_v1(&op, &s, &f, Some(true), &mut r)
            .unwrap();
        if stale_root {
            edit(&mut o, &s.source, &mut r);
        } else {
            o.resync_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r)
                .unwrap();
        }
        let before = o.facet_publication_usage_v1().unwrap();
        let mut q = FacetPublicationCommandRequestV1 {
            scope: &s,
            operation: &op,
            current: Some(&f),
            owner_control_allowed: Some(true),
            max_reply_bytes: 65536,
        };
        assert_eq!(
            o.request_facet_publication_command_v1(&id, &q, &mut r),
            Err(Error::Unavailable)
        );
        q.current = None;
        assert_eq!(
            o.request_facet_publication_command_v1(&id, &q, &mut r),
            Err(Error::Unavailable)
        );
        assert!(
            !o.facet_publication_command_facts_v1(&id, &s, &mut r)
                .unwrap()
                .native_started
        );
        assert_eq!(o.facet_publication_usage_v1().unwrap(), before);
        f.baseline_allowed = Some(false);
        assert_eq!(
            o.issue_facet_publication_command_v1(&op, &s, &f, Some(true), &mut r),
            Err(Error::Unavailable)
        );
    }
}

#[test]
fn facet_command_retained_obligations_are_counted_until_real_retirement_and_pool_refund() {
    let (mut o, mut r, s) = source_fixture();
    let before = o.retention_usage();
    let charge = configure(&mut o, &mut r);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let f = facts(&s, &n, &[], &[]);
    let op = FacetPublicationCommandOperationV1::Establish;
    let id = o
        .issue_facet_publication_command_v1(&op, &s, &f, Some(true), &mut r)
        .unwrap();
    assert_eq!(
        o.document_context_pins_v1(&s.source.document, &mut r)
            .unwrap()
            .publication_streams,
        1
    );
    let row = crate::coordinator::shutdown::tests::inventory_owner(
        &o,
        &mut r,
        ShutdownOwnerRoleV1::FacetPublication,
        &[&s.source.document],
    );
    assert_eq!(row.retained_records, Some(1));
    assert_eq!(row.additional_reserved_bytes, Some(charge));
    assert_eq!(o.retention_usage().reserved, before.reserved + charge);
    assert_eq!(
        o.retire_facet_publication_pool_v1(&ctl(), &mut r),
        Err(Error::Unavailable)
    );
    let mut control = cleanup(&s);
    control.delivery_drained = Some(false);
    assert_eq!(
        o.retire_facet_publication_command_v1(&id, &control, &mut r),
        Err(Error::Unavailable)
    );
    control.delivery_drained = Some(true);
    o.retire_facet_publication_command_v1(&id, &control, &mut r)
        .unwrap();
    assert_eq!(
        o.document_context_pins_v1(&s.source.document, &mut r)
            .unwrap()
            .publication_streams,
        0
    );
    assert_eq!(o.retention_usage().reserved, before.reserved + charge);
    o.retire_facet_publication_commands_v1(&ctl(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage(), before);
    assert!(o.facet_publication_command_usage_v1().unwrap().closed);
    assert_eq!(
        o.retire_facet_publication_commands_v1(&ctl(), &mut r),
        Err(Error::Unavailable)
    );
    assert_eq!(
        o.request_facet_publication_command_v1(
            &id,
            &FacetPublicationCommandRequestV1 {
                scope: &s,
                operation: &op,
                current: Some(&f),
                owner_control_allowed: Some(true),
                max_reply_bytes: 65536
            },
            &mut r
        ),
        Err(Error::Unavailable)
    );
}

#[test]
fn facet_command_interruption_preserves_unknown_native_effect_and_protected_cleanup() {
    let (mut o, mut r, s) = source_fixture();
    configure(&mut o, &mut r);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let f = facts(&s, &n, &[], &[]);
    let op = FacetPublicationCommandOperationV1::Establish;
    let id = o
        .issue_facet_publication_command_v1(&op, &s, &f, Some(true), &mut r)
        .unwrap();
    let super::super::State::Active(p) = &mut o.facet_publication_streams else {
        unreachable!()
    };
    let State::Active(t) = &mut p.commands else {
        unreachable!()
    };
    t.fault_after_native = true;
    let q = FacetPublicationCommandRequestV1 {
        scope: &s,
        operation: &op,
        current: Some(&f),
        owner_control_allowed: Some(true),
        max_reply_bytes: 65536,
    };
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || o.request_facet_publication_command_v1(&id, &q, &mut r)
        ))
        .is_err()
    );
    let original = o
        .facet_publication_command_facts_v1(&id, &s, &mut r)
        .unwrap();
    assert_eq!(original.phase, PhaseV1::Faulted);
    assert!(original.native_started);
    assert!(original.as_observed.is_none());
    assert_eq!(o.facet_publication_usage_v1().unwrap().streams, 1);
    assert_eq!(
        o.request_facet_publication_command_v1(&id, &q, &mut r),
        Err(Error::Unavailable)
    );
    assert!(matches!(
        o.open_facet_publication_stream_v1(&s, &f, &mut r),
        Err(Error::Faulted)
    ));
    assert_eq!(
        o.retire_facet_publication_command_v1(&id, &cleanup(&s), &mut r),
        Err(Error::Unavailable)
    );
    let stream = FacetPublicationStreamIdV1 {
        owner: id.owner.clone(),
        slot: 0,
    };
    o.stop_facet_publication_stream_v1(&stream, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_facet_publication_stream_v1(&stream, &cleanup(&s), &mut r)
        .unwrap();
    assert_eq!(
        o.document_context_pins_v1(&s.source.document, &mut r)
            .unwrap()
            .publication_streams,
        1
    );
}

#[test]
fn facet_command_capture_modes_do_not_change_execution_and_time_only_actual_services() {
    for mode in 0..4 {
        let (mut o, mut setup, s) = source_fixture();
        configure(&mut o, &mut setup);
        let n = crate::coordinator::publication_stream::tests::native(&s.source);
        let f = facts(&s, &n, &[], &[]);
        let op = FacetPublicationCommandOperationV1::Establish;
        let id = o
            .issue_facet_publication_command_v1(&op, &s, &f, Some(true), &mut setup)
            .unwrap();
        let debt = o.retention_usage();
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 64 }).unwrap()
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
        let mut q = FacetPublicationCommandRequestV1 {
            scope: &s,
            operation: &op,
            current: Some(&f),
            owner_control_allowed: Some(false),
            max_reply_bytes: 65536,
        };
        assert_eq!(
            o.request_facet_publication_command_v1(&id, &q, &mut r),
            Err(Error::Unavailable)
        );
        q.owner_control_allowed = Some(true);
        o.request_facet_publication_command_v1(&id, &q, &mut r)
            .unwrap();
        q.current = None;
        o.request_facet_publication_command_v1(&id, &q, &mut r)
            .unwrap();
        assert_eq!(o.facet_publication_usage_v1().unwrap().streams, 1);
        assert_eq!(o.retention_usage(), debt);
        if mode == 0 {
            assert_eq!(r.queued(), 0);
            assert_eq!(r.dropped(), 0);
        } else if mode == 3 {
            assert_eq!(r.pop_front(), Some(sentinel));
            assert!(r.dropped() > 0);
        } else {
            let mut dispositions = Vec::new();
            while let Some(e) = r.pop_front() {
                if e.boundary == Boundary::FacetPublicationCommandRequest {
                    assert_eq!(e.service_time.is_some(), mode == 2);
                    dispositions.push(e.disposition);
                }
            }
            assert_eq!(
                dispositions,
                [
                    Disposition::Refused,
                    Disposition::Completed,
                    Disposition::Completed
                ]
            );
        }
    }
}
