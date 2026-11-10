use super::*;
#[test]
fn publication_admission_fence_blocks_source_command_and_receipt_pool_configuration() {
    for receipt in [false, true] {
        let (mut o, mut r, _) = crate::coordinator::publication_stream::tests::owner_with_budget(
            stream_limits(),
            8388608,
        );
        if receipt {
            o.configure_source_publication_commands_v1(limits(), &pool_control(), &mut r)
                .unwrap();
        }
        crate::coordinator::shutdown::tests::install(&mut o, &mut r);
        let debt = o.retention_usage();
        let result = if receipt {
            o.configure_source_publication_receipts_v1(
                super::receipts::tests::limits(),
                &pool_control(),
                &mut r,
            )
        } else {
            o.configure_source_publication_commands_v1(limits(), &pool_control(), &mut r)
        };
        assert_eq!(result, Err(Error::Unavailable));
        assert_eq!(o.retention_usage(), debt);
    }
}
use crate::coordinator::publication_stream::tests::{
    facts, limits as stream_limits, native, owner,
};
use crate::instrumentation::NativeTimingProfileV1;
pub(super) fn limits() -> SourcePublicationCommandLimitsV1 {
    SourcePublicationCommandLimitsV1 {
        max_commands: 4,
        max_issued: 8,
        max_context_bytes: 2048,
        max_result_bytes: 16384,
        max_pool_bytes: 524288,
    }
}
pub(super) fn pool_control() -> SourceReadPoolControlV1 {
    SourceReadPoolControlV1 {
        owner_control_allowed: Some(true),
        resources_ready: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(super) fn control(s: &SourceReadScopeV1) -> SourceReadContextControlV1<'_> {
    SourceReadContextControlV1 {
        scope: s,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(super) fn edit(
    o: &mut SourceCoordinator,
    scope: &SourceReadScopeV1,
    from: &str,
    to: &str,
    r: &mut Recorder,
) {
    let current = o.read(&scope.document, r).unwrap().version;
    let p = o
        .prepare(
            &[DocumentEdits {
                expected: current,
                edits: vec![Replacement {
                    start: 0,
                    end: from.len(),
                    expected: from.into(),
                    inserted: to.into(),
                }],
            }],
            &[],
            r,
        )
        .unwrap();
    o.commit(p, r).unwrap();
}
#[test]
fn original_establishment_is_once_only_with_protected_replay_and_actual_retained_pins() {
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
        assert_eq!(
            o.document_context_pins_v1(&s.document, &mut setup)
                .unwrap()
                .publication_streams,
            1
        );
        let debt = o.retention_usage();
        let mut r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let mut q = SourcePublicationCommandRequestV1 {
            scope: &s,
            operation: &op,
            current: Some(&f),
            owner_control_allowed: Some(true),
            max_reply_bytes: 16384,
        };
        q.current = None;
        assert_eq!(
            o.request_source_publication_command_v1(&id, &q, &mut r),
            Err(Error::Unavailable)
        );
        assert!(
            !o.source_publication_command_facts_v1(&id, &s, &mut setup)
                .unwrap()
                .native_started
        );
        q.current = Some(&f);
        q.max_reply_bytes = size_of::<SourcePublicationCommandReplyV1>() - 1;
        assert_eq!(
            o.request_source_publication_command_v1(&id, &q, &mut r),
            Err(Error::Limit)
        );
        assert_eq!(o.source_publication_usage_v1().unwrap().streams, 0);
        q.max_reply_bytes = 16384;
        assert_eq!(
            o.request_source_publication_command_v1(&id, &q, &mut r),
            Ok(SourcePublicationCommandReplyV1::Settled {
                command: id.clone()
            })
        );
        let b = o
            .source_publication_command_facts_v1(&id, &s, &mut setup)
            .unwrap()
            .as_observed
            .unwrap()
            .as_ref()
            .unwrap()
            .clone();
        assert_eq!(b.projection.text(), "A");
        assert_eq!(
            o.document_context_pins_v1(&s.document, &mut setup)
                .unwrap()
                .publication_streams,
            2
        );
        edit(&mut o, &s, "A", "B", &mut setup);
        o.sample_source_publication_stream_v1(&b.stream, &s, &f, &mut setup)
            .unwrap();
        let usage = o.source_publication_usage_v1().unwrap();
        q.current = None;
        q.owner_control_allowed = None;
        assert_eq!(
            o.request_source_publication_command_v1(&id, &q, &mut r),
            Err(Error::Unavailable)
        );
        q.owner_control_allowed = Some(true);
        assert_eq!(
            o.request_source_publication_command_v1(&id, &q, &mut r),
            Ok(SourcePublicationCommandReplyV1::ObserveExisting {
                command: id.clone()
            })
        );
        assert_eq!(o.source_publication_usage_v1().unwrap(), usage);
        o.stop_source_publication_stream_v1(&b.stream, &control(&s), &mut setup)
            .unwrap();
        o.retire_source_publication_stream_v1(&b.stream, &control(&s), &mut setup)
            .unwrap();
        assert_eq!(
            o.document_context_pins_v1(&s.document, &mut setup)
                .unwrap()
                .publication_streams,
            1
        );
        o.faulted = true;
        assert!(matches!(
            o.request_source_publication_command_v1(&id, &q, &mut r),
            Ok(SourcePublicationCommandReplyV1::ObserveExisting { .. })
        ));
        assert_eq!(
            o.source_publication_command_facts_v1(&id, &s, &mut setup)
                .unwrap()
                .as_observed
                .unwrap()
                .as_ref()
                .unwrap()
                .projection
                .text(),
            "A"
        );
        assert_eq!(o.retention_usage(), debt);
        assert_eq!(
            o.retire_source_publication_pool_v1(&pool_control(), &mut setup),
            Err(Error::Unavailable)
        );
        o.retire_source_publication_command_v1(&id, &control(&s), &mut r)
            .unwrap();
        o.retire_source_publication_commands_v1(&pool_control(), &mut setup)
            .unwrap();
        assert!(o.source_publication_command_usage_v1().unwrap().closed);
        o.retire_source_publication_pool_v1(&pool_control(), &mut setup)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
        if capacity == 64 {
            assert!(
                std::iter::from_fn(|| r.pop_front())
                    .any(|e| e.boundary == Boundary::SourcePublicationCommandRequest
                        && e.service_time.is_some())
            );
        }
        if capacity == 1 {
            assert!(r.dropped() > 0);
        }
    }
}

#[test]
fn first_execution_requires_original_root_and_generation_without_consuming_stale_intent() {
    let (mut o, mut r, s) = owner(stream_limits());
    o.configure_source_publication_commands_v1(limits(), &pool_control(), &mut r)
        .unwrap();
    let n = native(&s);
    let f = facts(&s, &n);
    let op = SourcePublicationCommandOperationV1::Establish;
    let stale = o
        .issue_source_publication_command_v1(&op, &s, &f, Some(true), &mut r)
        .unwrap();
    edit(&mut o, &s, "A", "B", &mut r);
    let q = SourcePublicationCommandRequestV1 {
        scope: &s,
        operation: &op,
        current: Some(&f),
        owner_control_allowed: Some(true),
        max_reply_bytes: 16384,
    };
    assert_eq!(
        o.request_source_publication_command_v1(&stale, &q, &mut r),
        Err(Error::Unavailable)
    );
    assert!(
        !o.source_publication_command_facts_v1(&stale, &s, &mut r)
            .unwrap()
            .native_started
    );
    let b = o.open_source_publication_stream_v1(&s, &f, &mut r).unwrap();
    let op = SourcePublicationCommandOperationV1::Resynchronize {
        stream: b.stream.clone(),
    };
    let stale = o
        .issue_source_publication_command_v1(&op, &s, &f, Some(true), &mut r)
        .unwrap();
    o.resync_source_publication_stream_v1(&b.stream, &s, &f, &mut r)
        .unwrap();
    let q = SourcePublicationCommandRequestV1 {
        scope: &s,
        operation: &op,
        current: Some(&f),
        owner_control_allowed: Some(true),
        max_reply_bytes: 16384,
    };
    assert_eq!(
        o.request_source_publication_command_v1(&stale, &q, &mut r),
        Err(Error::Unavailable)
    );
    assert!(
        !o.source_publication_command_facts_v1(&stale, &s, &mut r)
            .unwrap()
            .native_started
    );
}

#[test]
fn interrupted_native_acknowledgment_retains_uncertainty_and_fences_ordinary_publisher_work() {
    let (mut o, mut r, s) = owner(stream_limits());
    o.configure_source_publication_commands_v1(limits(), &pool_control(), &mut r)
        .unwrap();
    let n = native(&s);
    let f = facts(&s, &n);
    let op = SourcePublicationCommandOperationV1::Establish;
    let id = o
        .issue_source_publication_command_v1(&op, &s, &f, Some(true), &mut r)
        .unwrap();
    let super::super::State::Active(p) = &mut o.publication_streams else {
        panic!()
    };
    let State::Active(t) = &mut p.commands else {
        panic!()
    };
    t.fault_after_native = true;
    let q = SourcePublicationCommandRequestV1 {
        scope: &s,
        operation: &op,
        current: Some(&f),
        owner_control_allowed: Some(true),
        max_reply_bytes: 16384,
    };
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || o.request_source_publication_command_v1(&id, &q, &mut r)
        ))
        .is_err()
    );
    let facts = o
        .source_publication_command_facts_v1(&id, &s, &mut r)
        .unwrap();
    assert_eq!(facts.phase, PhaseV1::Faulted);
    assert!(facts.native_started && facts.as_observed.is_none());
    assert_eq!(o.source_publication_usage_v1().unwrap().streams, 1);
    let debt = o.retention_usage();
    assert_eq!(
        o.open_source_publication_stream_v1(&s, &f, &mut r)
            .unwrap_err(),
        Error::Faulted
    );
    assert_eq!(
        o.retire_source_publication_command_v1(&id, &control(&s), &mut r),
        Err(Error::Unavailable)
    );
    assert_eq!(
        o.request_source_publication_command_v1(&id, &q, &mut r),
        Err(Error::Unavailable)
    );
    assert_eq!(o.retention_usage(), debt);
    assert!(!o.faulted);
    let super::super::State::Active(p) = &o.publication_streams else {
        panic!()
    };
    let stream = SourcePublicationStreamIdV1 {
        owner: p.owner.clone(),
        slot: 0,
    };
    assert!(matches!(
        o.release_source_publication_event_v1(&stream, &s, &f, &mut r),
        Err(Error::Faulted)
    ));
    o.stop_source_publication_stream_v1(&stream, &control(&s), &mut r)
        .unwrap();
    o.retire_source_publication_stream_v1(&stream, &control(&s), &mut r)
        .unwrap();
    assert_eq!(
        o.document_context_pins_v1(&s.document, &mut r)
            .unwrap()
            .publication_streams,
        1
    );
}

#[test]
fn original_resync_retains_its_result_without_resetting_later_events_or_accepting_other_roles() {
    let (mut o, mut r, s) = owner(stream_limits());
    o.configure_source_publication_commands_v1(limits(), &pool_control(), &mut r)
        .unwrap();
    let n = native(&s);
    let f = facts(&s, &n);
    let stream = o
        .open_source_publication_stream_v1(&s, &f, &mut r)
        .unwrap()
        .stream;
    edit(&mut o, &s, "A", "B", &mut r);
    o.sample_source_publication_stream_v1(&stream, &s, &f, &mut r)
        .unwrap();
    let op = SourcePublicationCommandOperationV1::Resynchronize {
        stream: stream.clone(),
    };
    let id = o
        .issue_source_publication_command_v1(&op, &s, &f, Some(true), &mut r)
        .unwrap();
    let mut q = SourcePublicationCommandRequestV1 {
        scope: &s,
        operation: &op,
        current: Some(&f),
        owner_control_allowed: Some(true),
        max_reply_bytes: 16384,
    };
    o.request_source_publication_command_v1(&id, &q, &mut r)
        .unwrap();
    assert_eq!(o.source_publication_usage_v1().unwrap().queued, 0);
    let result = o
        .source_publication_command_facts_v1(&id, &s, &mut r)
        .unwrap()
        .as_observed
        .unwrap()
        .as_ref()
        .unwrap()
        .clone();
    assert_eq!(result.projection.text(), "B");
    edit(&mut o, &s, "B", "C", &mut r);
    o.sample_source_publication_stream_v1(&stream, &s, &f, &mut r)
        .unwrap();
    let before = o.source_publication_usage_v1().unwrap();
    assert_eq!(before.queued, 1);
    q.current = None;
    assert!(matches!(
        o.request_source_publication_command_v1(&id, &q, &mut r),
        Ok(SourcePublicationCommandReplyV1::ObserveExisting { .. })
    ));
    assert_eq!(o.source_publication_usage_v1().unwrap(), before);
    assert!(Arc::ptr_eq(
        &result,
        o.source_publication_command_facts_v1(&id, &s, &mut r)
            .unwrap()
            .as_observed
            .unwrap()
            .as_ref()
            .unwrap()
    ));
    q.operation = &SourcePublicationCommandOperationV1::Establish;
    assert_eq!(
        o.request_source_publication_command_v1(&id, &q, &mut r),
        Err(Error::Unavailable)
    );
    let mut wrong_scope = s.clone();
    wrong_scope.context.recipient = "other".into();
    q.operation = &op;
    q.scope = &wrong_scope;
    assert_eq!(
        o.request_source_publication_command_v1(&id, &q, &mut r),
        Err(Error::Unavailable)
    );
    let (mut foreign, mut setup, other_scope) = owner(stream_limits());
    foreign
        .configure_source_publication_commands_v1(limits(), &pool_control(), &mut setup)
        .unwrap();
    q.scope = &other_scope;
    assert_eq!(
        foreign.request_source_publication_command_v1(&id, &q, &mut setup),
        Err(Error::Unavailable)
    );
    assert_eq!(o.source_publication_usage_v1().unwrap(), before);
}

#[test]
fn command_capacity_is_prepaid_and_retirement_does_not_recycle_original_identities() {
    let (mut o, mut r, s) = owner(stream_limits());
    let mut l = limits();
    l.max_commands = 1;
    l.max_issued = 2;
    let exact = l.required_pool_bytes().unwrap();
    let before = o.retention_usage();
    l.max_pool_bytes = exact - 1;
    assert_eq!(
        o.configure_source_publication_commands_v1(l, &pool_control(), &mut r),
        Err(Error::Limit)
    );
    assert_eq!(o.retention_usage(), before);
    l.max_pool_bytes = exact;
    o.configure_source_publication_commands_v1(l, &pool_control(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, before.reserved + exact);
    let n = native(&s);
    let f = facts(&s, &n);
    let op = SourcePublicationCommandOperationV1::Establish;
    let first = o
        .issue_source_publication_command_v1(&op, &s, &f, Some(true), &mut r)
        .unwrap();
    assert_eq!(
        o.issue_source_publication_command_v1(&op, &s, &f, Some(true), &mut r)
            .unwrap_err(),
        Error::Limit
    );
    let mut denied = control(&s);
    denied.delivery_drained = None;
    assert_eq!(
        o.retire_source_publication_command_v1(&first, &denied, &mut r),
        Err(Error::Unavailable)
    );
    o.retire_source_publication_command_v1(&first, &control(&s), &mut r)
        .unwrap();
    let second = o
        .issue_source_publication_command_v1(&op, &s, &f, Some(true), &mut r)
        .unwrap();
    assert_ne!(first, second);
    o.retire_source_publication_command_v1(&second, &control(&s), &mut r)
        .unwrap();
    assert_eq!(
        o.issue_source_publication_command_v1(&op, &s, &f, Some(true), &mut r)
            .unwrap_err(),
        Error::Limit
    );
    assert_eq!(
        o.source_publication_command_usage_v1()
            .unwrap()
            .issued_lifetime,
        2
    );
    o.retire_source_publication_commands_v1(&pool_control(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage(), before);
    assert_eq!(
        o.configure_source_publication_commands_v1(l, &pool_control(), &mut r),
        Err(Error::Unavailable)
    );
}

#[test]
fn command_accounting_and_phase_corruption_refuse_instead_of_reporting_zero_pins() {
    for corrupt_charge in [false, true] {
        let (mut o, mut r, s) = owner(stream_limits());
        o.configure_source_publication_commands_v1(limits(), &pool_control(), &mut r)
            .unwrap();
        let n = native(&s);
        let f = facts(&s, &n);
        let id = o
            .issue_source_publication_command_v1(
                &SourcePublicationCommandOperationV1::Establish,
                &s,
                &f,
                Some(true),
                &mut r,
            )
            .unwrap();
        let super::super::State::Active(p) = &mut o.publication_streams else {
            panic!()
        };
        let State::Active(t) = &mut p.commands else {
            panic!()
        };
        if corrupt_charge {
            t.charge += 1;
        } else {
            t.rows[id.slot].as_mut().unwrap().life.started = true;
        }
        assert_eq!(
            o.document_context_pins_v1(&s.document, &mut r),
            Err(CoordinatorError::Faulted)
        );
        assert_eq!(o.source_publication_command_usage_v1(), Err(Error::Faulted));
        assert_eq!(
            o.retire_source_publication_command_v1(&id, &control(&s), &mut r),
            Err(Error::Faulted)
        );
    }
}

#[test]
fn actual_reclamation_waits_for_original_command_retirement_even_after_stream_retirement() {
    for execute in [false, true] {
        let (mut o, mut r, s) = owner(stream_limits());
        o.configure_source_publication_commands_v1(limits(), &pool_control(), &mut r)
            .unwrap();
        let n = native(&s);
        let f = facts(&s, &n);
        let op = SourcePublicationCommandOperationV1::Establish;
        let id = o
            .issue_source_publication_command_v1(&op, &s, &f, Some(true), &mut r)
            .unwrap();
        if execute {
            let q = SourcePublicationCommandRequestV1 {
                scope: &s,
                operation: &op,
                current: Some(&f),
                owner_control_allowed: Some(true),
                max_reply_bytes: 16384,
            };
            o.request_source_publication_command_v1(&id, &q, &mut r)
                .unwrap();
            let stream = o
                .source_publication_command_facts_v1(&id, &s, &mut r)
                .unwrap()
                .as_observed
                .unwrap()
                .as_ref()
                .unwrap()
                .stream
                .clone();
            o.stop_source_publication_stream_v1(&stream, &control(&s), &mut r)
                .unwrap();
            o.retire_source_publication_stream_v1(&stream, &control(&s), &mut r)
                .unwrap();
        }
        let version = o.read(&s.document, &mut r).unwrap().version;
        let plan = o.prepare_retained_disposal(&version, 0, &mut r).unwrap();
        let state = o.commit_retained_disposal(plan, &mut r).unwrap();
        o.release_disposed_provenance(&s.document, state, &mut r)
            .unwrap();
        let state = o.lifecycle_state(&s.document, &mut r).unwrap();
        let discharge = TrustedDocumentReclamationV1 {
            authorized: Some(true),
            authored_disposition_ready: Some(true),
            session_references_released: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        };
        assert_eq!(
            o.reclaim_disposed_document_v1(&s.document, state, &discharge, &mut r),
            Err(CoordinatorError::Resource(
                crate::resource::ResourceError::Retained
            ))
        );
        o.retire_source_publication_command_v1(&id, &control(&s), &mut r)
            .unwrap();
        o.reclaim_disposed_document_v1(&s.document, state, &discharge, &mut r)
            .unwrap();
        o.retire_source_publication_commands_v1(&pool_control(), &mut r)
            .unwrap();
        o.retire_source_publication_pool_v1(&pool_control(), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
    }
}
