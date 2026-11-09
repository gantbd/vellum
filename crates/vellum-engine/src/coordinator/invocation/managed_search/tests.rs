use super::*;
pub(super) fn ready() -> (
    SourceCoordinator,
    Recorder,
    DocumentVersion,
    SourceReadScopeV1,
    SourceSearchConsumerV1,
) {
    let mut o = SourceCoordinator::with_invocation_limits(
        CoordinatorLimits {
            max_document_incarnations: 4,
            max_participants: 4,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 4096,
                max_edits: 16,
                max_edit_payload_bytes: 4096,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 8,
            max_retained: 8,
            max_intent_bytes: 65536,
        },
    );
    let mut r = Recorder::disabled();
    let d = o.create(b"aba", &mut r).unwrap();
    let s = SourceReadScopeV1 {
        document: d.document.clone(),
        context: crate::coordinator::scoped_definition::tests::context(),
        policy: "p1".into(),
    };
    o.configure_retention_budget(usize::MAX, &mut r).unwrap();
    o.configure_source_read_contexts(
        "engine",
        "local",
        SourceReadContextLimitsV1 {
            max_contexts: 4,
            max_holds: 4,
            max_issued: 8,
            max_root_bytes: 1048576,
            max_capture_bytes: 4096,
            max_context_bytes: 2048,
            max_request_bytes: 4096,
            max_reply_bytes: 4096,
            max_label_bytes: 128,
            max_scan_bytes: 4096,
            max_pool_bytes: 1048576,
        },
        &SourceReadPoolControlV1 {
            owner_control_allowed: Some(true),
            resources_ready: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        },
        &mut r,
    )
    .unwrap();
    o.configure_source_search(
        SourceSearchLimitsV1 {
            max_results: 4,
            max_issued: 8,
            max_request_bytes: 4096,
            max_capture_bytes: 8192,
            max_reply_bytes: 4096,
            max_pool_bytes: 1048576,
            max_query_bytes: 128,
            max_scope_ranges: 8,
            max_comparisons: 4096,
            max_matches: 32,
            max_replacement_bytes: 4096,
        },
        &SourceReadPoolControlV1 {
            owner_control_allowed: Some(true),
            resources_ready: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        },
        &mut r,
    )
    .unwrap();
    let source = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let q = request();
    let result = o
        .query_source_search(&source, &s, &q, &facts(&s), &query(&q), &mut r)
        .unwrap();
    let c = SourceSearchConsumerV1::ReplaceAll {
        result,
        replacement: "Z".into(),
    };
    (o, r, d, s, c)
}
pub(super) fn request() -> SearchRequest {
    SearchRequest {
        query: "a".into(),
        scope: SearchScope::Document,
    }
}
pub(super) fn facts(s: &SourceReadScopeV1) -> TrustedSourceReadFactsV1<'_> {
    TrustedSourceReadFactsV1 {
        scope: s,
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        retained: Some(true),
        source_readable: Some(true),
        resources_ready: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
    }
}
pub(super) fn query(q: &SearchRequest) -> TrustedSourceSearchFactsV1<'_> {
    TrustedSourceSearchFactsV1 {
        request: q,
        grant: Some(true),
        scope: Some(true),
        continuity: Some(true),
        capacity: Some(true),
        retained: Some(true),
        not_stopped: Some(true),
    }
}
pub(super) fn mutation<'a>(
    c: &'a SourceSearchConsumerV1,
    s: &'a SourceReadScopeV1,
) -> TrustedSourceSearchConsumerFactsV1<'a> {
    TrustedSourceSearchConsumerFactsV1 {
        scope: s,
        command: c,
        mutation_allowed: Some(true),
        resources_ready: Some(true),
    }
}
#[test]
fn private_candidate_drift_is_faulted_before_service_loss_can_be_reported_as_not_applied() {
    let (mut o, _, d, s, c) = ready();
    let mut r = timed_service_recorder();
    let q = request();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_source_search_consumer(
        &id,
        &c,
        &s,
        &facts(&s),
        &query(&q),
        &mutation(&c, &s),
        &mut r,
    )
    .unwrap();
    let slot = o.invocations.slots[id.slot].as_mut().unwrap();
    let Phase::ManagedSearchPending(p) = &mut slot.phase else {
        unreachable!()
    };
    let ConsumerPlan::Source(Ok(plan)) = &mut p.plan else {
        unreachable!()
    };
    plan.candidates[0].edits[0].inserted = "INJECTED".into();
    let mut loss = query(&q);
    loss.grant = Some(false);
    assert_eq!(
        o.run_source_search_consumer(&id, &s, &facts(&s), &loss, &mutation(&c, &s), &mut r),
        Err(InvocationError::Faulted)
    );
    assert!(o.is_faulted());
    assert!(o.invocation_outcome(&id, &mut r).is_err());
    assert_eq!(o.document(&d.document).unwrap().source.text(), "aba");
    assert_service_markers(
        &mut r,
        &[(Boundary::InvocationExecution, Disposition::Unknown)],
    );
}
#[test]
fn invocation_capture_and_complete_request_envelopes_refuse_before_binding_and_never_spend_a_source_hold()
 {
    let (mut probe, mut r, _, s, c) = ready();
    let q = request();
    let p = probe
        .capture_source_search_consumer(
            &c,
            &s,
            SearchReadFactsV1::Source(&facts(&s)),
            &query(&q),
            &mutation(&c, &s),
            65536,
            &mut r,
        )
        .unwrap();
    let capture_size = std::mem::size_of_val(p.0.as_ref());
    assert!(capture_size > 0);
    // Find the exact native boundary without assuming a platform header size.
    let mut lo = 0usize;
    let mut hi = 65536usize;
    while lo < hi {
        let n = lo + (hi - lo) / 2;
        let (mut o, mut r, _, s, c) = ready();
        let admitted = o
            .capture_source_search_consumer(
                &c,
                &s,
                SearchReadFactsV1::Source(&facts(&s)),
                &query(&q),
                &mutation(&c, &s),
                n,
                &mut r,
            )
            .is_ok();
        if admitted { hi = n } else { lo = n + 1 }
    }
    for n in [lo - 1, lo, lo + 1] {
        let (mut o, mut r, _, s, c) = ready();
        o.invocations.limits.max_intent_bytes = n;
        let id = o.issue_invocation(&mut r).unwrap();
        let result = o.submit_source_search_consumer(
            &id,
            &c,
            &s,
            &facts(&s),
            &query(&q),
            &mutation(&c, &s),
            &mut r,
        );
        assert_eq!(result.is_ok(), n >= lo);
        assert_eq!(o.source_read_context_usage().holds, 1);
        assert_eq!(o.source_read_context_usage().issued_lifetime, 2);
        if n < lo {
            assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
        }
    }
}

#[test]
fn interruption_after_real_source_installation_remains_unknown_and_cannot_retry_or_stop_as_unapplied()
 {
    let (mut o, mut r, d, s, c) = ready();
    let q = request();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_source_search_consumer(
        &id,
        &c,
        &s,
        &facts(&s),
        &query(&q),
        &mutation(&c, &s),
        &mut r,
    )
    .unwrap();
    o.publication_fault_after = Some(1);
    let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        o.run_source_search_consumer(&id, &s, &facts(&s), &query(&q), &mutation(&c, &s), &mut r)
    }));
    assert!(interrupted.is_err());
    assert!(o.is_faulted());
    assert_eq!(o.document(&d.document).unwrap().source.text(), "ZbZ");
    assert_eq!(
        o.invocation_outcome(&id, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.run_source_search_consumer(&id, &s, &facts(&s), &query(&q), &mutation(&c, &s), &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.stop_invocation(&id, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(o.invocation_usage().faulted_executions, 1);
    assert_eq!(o.source_read_context_usage().holds, 1);
}
