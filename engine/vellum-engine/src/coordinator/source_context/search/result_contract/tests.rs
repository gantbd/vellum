use super::*;
use crate::coordinator::source_query::result_contract::tests::{
    Fixture, cleanup, member_facts, source_facts,
};
pub(in crate::coordinator) fn with_facts<T>(
    f: &mut Fixture,
    kind: usize,
    call: impl for<'a> FnOnce(
        &mut SourceCoordinator,
        SearchReadContextV1<'a>,
        &'a SourceReadScopeV1,
        SearchReadFactsV1<'a>,
        &mut Recorder,
    ) -> T,
) -> T {
    let sf = source_facts(&f.s);
    let cf = TrustedAnnotationContextFactsV1 {
        context: &f.combined,
        scope: &f.s,
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        annotation_readable: Some(true),
        source_readable: Some(true),
        projection_current: Some(true),
        retained: Some(true),
        resources_ready: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
    };
    let parent = TrustedVersionSetContextFactsV1 {
        context: &f.set.context,
        members: &[member_facts(&f.s)],
    };
    let vf = TrustedVersionSetSearchFactsV1 {
        member: &f.set.participants[0].snapshot,
        scope: &f.s,
        parent: &parent,
    };
    let (source, facts) = match kind {
        0 => (
            SearchReadContextV1::Source(&f.source),
            SearchReadFactsV1::Source(&sf),
        ),
        1 => (
            SearchReadContextV1::Combined(&f.combined),
            SearchReadFactsV1::Combined(&cf),
        ),
        _ => (
            SearchReadContextV1::VersionSet(&f.set.participants[0].snapshot),
            SearchReadFactsV1::VersionSet(&vf),
        ),
    };
    call(&mut f.o, source, &f.s, facts, &mut f.r)
}
pub(in crate::coordinator) fn holds(f: &Fixture) -> [usize; 3] {
    [
        f.o.source_read_context_usage().holds,
        f.o.annotation_context_usage().holds,
        f.o.version_set_context_usage().holds,
    ]
}
pub(in crate::coordinator) fn retire(f: &mut Fixture, kind: usize, id: &SourceSearchResultId) {
    let single = cleanup(&f.s);
    let multiple = [cleanup(&f.s)];
    let c = if kind == 2 {
        SearchResultControlV1::VersionSet(&multiple)
    } else {
        SearchResultControlV1::Single(&single)
    };
    assert!(f.o.retire_managed_search_result(id, c, &mut f.r));
}
pub(in crate::coordinator) fn last_result(o: &SourceCoordinator) -> Option<SourceSearchResultId> {
    let p = o.search_pool().ok()?;
    let slot = p.rows.len().checked_sub(1)?;
    p.rows.get(slot)?.as_ref()?;
    Some(SourceSearchResultId {
        owner: o.namespace.clone(),
        slot,
    })
}
pub(in crate::coordinator) fn owner() -> Fixture {
    let mut f = crate::coordinator::source_query::result_contract::tests::owner();
    f.o.configure_source_search(
        SourceSearchLimitsV1 {
            max_results: 8,
            max_issued: 16,
            max_request_bytes: 4096,
            max_capture_bytes: 8192,
            max_reply_bytes: 4096,
            max_pool_bytes: 1048576,
            max_query_bytes: 128,
            max_scope_ranges: 16,
            max_comparisons: 8192,
            max_matches: 32,
            max_replacement_bytes: 4096,
        },
        &SourceReadPoolControlV1 {
            owner_control_allowed: Some(true),
            resources_ready: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        },
        &mut f.r,
    )
    .unwrap();
    f
}
pub(in crate::coordinator) fn request() -> SearchRequest {
    SearchRequest {
        query: "o".into(),
        scope: SearchScope::Document,
    }
}
pub(in crate::coordinator) fn qfacts(q: &SearchRequest) -> TrustedSourceSearchFactsV1<'_> {
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

fn gauges_match_retained_rows(o: &SourceCoordinator) {
    let p = o.search_pool().unwrap();
    let live = p.rows.iter().flatten().count();
    let invalidated = p.rows.iter().flatten().filter(|row| row.stopped).count();
    assert_eq!((p.live, p.invalidated), (live, invalidated));
    let usage = o.source_search_usage();
    assert_eq!((usage.results, usage.invalidated), (live, invalidated));
    assert_eq!(usage.issued_lifetime, p.issued);
}

#[test]
fn constant_time_search_gauges_match_row_oracle_across_all_issuers_loss_refusal_and_retirement() {
    for kind in 0..3 {
        let mut f = owner();
        let q = request();
        gauges_match_retained_rows(&f.o);
        let a = with_facts(&mut f, kind, |o, source, s, sf, r| {
            o.query_managed_search(source, s, &q, sf, &qfacts(&q), r)
        })
        .unwrap();
        let b = with_facts(&mut f, kind, |o, source, s, sf, r| {
            o.query_managed_search(source, s, &q, sf, &qfacts(&q), r)
        })
        .unwrap();
        gauges_match_retained_rows(&f.o);
        assert_eq!(f.o.source_search_usage().results, 2);
        let mut denied = cleanup(&f.s);
        denied.retention_released = None;
        let frame = [denied];
        let control = if kind == 2 {
            SearchResultControlV1::VersionSet(&frame)
        } else {
            SearchResultControlV1::Single(&frame[0])
        };
        let before_holds = holds(&f);
        assert!(!f.o.retire_managed_search_result(&a, control, &mut f.r));
        gauges_match_retained_rows(&f.o);
        assert_eq!(holds(&f), before_holds);
        for value in [None, Some(false)] {
            let mut facts = qfacts(&q);
            facts.capacity = value;
            assert!(
                with_facts(&mut f, kind, |o, _, s, sf, r| o
                    .release_managed_search(&a, s, sf, &facts, r))
                .is_none()
            );
            gauges_match_retained_rows(&f.o);
            assert_eq!(f.o.source_search_usage().invalidated, 0);
        }
        let mut loss = qfacts(&q);
        loss.grant = Some(false);
        for _ in 0..2 {
            assert!(
                with_facts(&mut f, kind, |o, _, s, sf, r| o
                    .release_managed_search(&a, s, sf, &loss, r))
                .is_none()
            );
            gauges_match_retained_rows(&f.o);
            assert_eq!(f.o.source_search_usage().invalidated, 1);
        }
        assert!(
            with_facts(&mut f, kind, |o, _, s, sf, r| o.release_managed_search(
                &b,
                s,
                sf,
                &qfacts(&q),
                r
            ))
            .is_some()
        );
        retire(&mut f, kind, &a);
        gauges_match_retained_rows(&f.o);
        assert_eq!(f.o.source_search_usage().invalidated, 0);
        retire(&mut f, kind, &b);
        gauges_match_retained_rows(&f.o);
        assert_eq!(f.o.source_search_usage().results, 0);
        let lifetime = f.o.source_search_usage().issued_lifetime;
        let c = with_facts(&mut f, kind, |o, source, s, sf, r| {
            o.query_managed_search(source, s, &q, sf, &qfacts(&q), r)
        })
        .unwrap();
        gauges_match_retained_rows(&f.o);
        assert_eq!(f.o.source_search_usage().issued_lifetime, lifetime + 1);
        retire(&mut f, kind, &c);
        gauges_match_retained_rows(&f.o);
    }
}

#[test]
fn impossible_search_gauges_fail_hard_before_new_holds_or_issuance() {
    for bad in 0..3 {
        let mut f = owner();
        let q = request();
        let before_holds = holds(&f);
        let p = f.o.search_pool_mut().unwrap();
        match bad {
            0 => p.live = usize::MAX,
            1 => p.invalidated = 1,
            _ => p.live = 1,
        }
        assert!(
            with_facts(&mut f, 0, |o, source, s, sf, r| o.query_managed_search(
                source,
                s,
                &q,
                sf,
                &qfacts(&q),
                r
            ))
            .is_none()
        );
        assert!(f.o.is_faulted());
        assert_eq!(holds(&f), before_holds);
        assert_eq!(f.o.source_search_usage().issued_lifetime, 0);
    }
}

#[test]
fn retirement_gauge_underflow_preserves_the_original_retained_hold_and_result() {
    for invalidated in [false, true] {
        let mut f = owner();
        let q = request();
        let id = with_facts(&mut f, 0, |o, source, s, sf, r| {
            o.query_managed_search(source, s, &q, sf, &qfacts(&q), r)
        })
        .unwrap();
        let before_holds = holds(&f);
        let p = f.o.search_pool_mut().unwrap();
        if invalidated {
            p.rows[id.slot].as_mut().unwrap().stopped = true;
        } else {
            p.live = 0;
        }
        assert!(!f.o.retire_source_search_result(&id, &cleanup(&f.s), &mut f.r));
        assert!(f.o.is_faulted());
        assert_eq!(holds(&f), before_holds);
        assert!(f.o.search_row(&id).is_ok());
        assert_eq!(f.o.source_search_usage().issued_lifetime, 1);
    }
}

#[test]
fn cold_pool_retirement_reconciles_actual_rows_instead_of_trusting_an_empty_gauge() {
    let mut f = owner();
    let q = request();
    let id = with_facts(&mut f, 0, |o, source, s, sf, r| {
        o.query_managed_search(source, s, &q, sf, &qfacts(&q), r)
    })
    .unwrap();
    let before_holds = holds(&f);
    let reserved = f.o.retention_usage().reserved;
    f.o.search_pool_mut().unwrap().live = 0;
    let control = SourceReadPoolControlV1 {
        owner_control_allowed: Some(true),
        resources_ready: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    assert_eq!(
        f.o.retire_source_search(&control, &mut f.r),
        Err(Error::Faulted)
    );
    assert!(f.o.is_faulted());
    assert!(f.o.search_row(&id).is_ok());
    assert_eq!(holds(&f), before_holds);
    assert_eq!(f.o.retention_usage().reserved, reserved);
}
#[test]
fn malformed_search_producer_products_are_fenced_before_result_installation() {
    for fault in [
        Fault::ProducerRange,
        Fault::ProducerOrder,
        Fault::ProducerStatus,
    ] {
        let mut f = owner();
        f.r = Recorder::try_with_capacity(64)
            .unwrap()
            .with_native_timing(crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1);
        let q = request();
        let generation = f.o.generation();
        let before = f.o.source_read_context_usage();
        f.o.search_result_fault = Some(fault);
        assert!(
            f.o.query_source_search(
                &f.source,
                &f.s,
                &q,
                &source_facts(&f.s),
                &qfacts(&q),
                &mut f.r
            )
            .is_none()
        );
        assert!(f.o.is_faulted());
        assert_eq!(
            f.o.read_result_invariant(),
            Some(ReadResultInvariantV1::NativeSearchResultInvariant)
        );
        assert_eq!(f.o.source_search_usage().results, 0);
        assert_eq!(f.o.source_search_usage().issued_lifetime, 1);
        assert_eq!(f.o.source_read_context_usage().holds, before.holds);
        gauges_match_retained_rows(&f.o);
        assert_eq!(f.o.generation(), generation);
        let mut observed = false;
        while let Some(record) = f.r.pop_front() {
            if record.boundary == Boundary::SourceSearchEvaluation {
                assert_eq!(record.disposition, Disposition::Unknown);
                assert!(record.service_time.is_some());
                observed = true;
            }
        }
        assert!(observed);
    }
}
#[test]
fn malformed_search_release_products_are_withheld_and_retained_holds_remain_protected() {
    for fault in [
        Fault::ReleaseAlias,
        Fault::ReleaseVersion,
        Fault::ReleaseRanges,
        Fault::ReleaseStatus,
    ] {
        let mut f = owner();
        let q = request();
        let id =
            f.o.query_source_search(
                &f.source,
                &f.s,
                &q,
                &source_facts(&f.s),
                &qfacts(&q),
                &mut f.r,
            )
            .unwrap();
        let before = f.o.source_search_usage();
        let holds = f.o.source_read_context_usage().holds;
        f.o.search_result_fault = Some(fault);
        assert!(
            f.o.release_source_search(&id, &f.s, &source_facts(&f.s), &qfacts(&q), &mut f.r)
                .is_none()
        );
        assert!(f.o.is_faulted());
        assert_eq!(
            f.o.read_result_invariant(),
            Some(ReadResultInvariantV1::NativeSearchResultInvariant)
        );
        assert_eq!(f.o.source_search_usage(), before);
        assert_eq!(f.o.source_read_context_usage().holds, holds);
        let c = crate::coordinator::source_query::result_contract::tests::cleanup(&f.s);
        assert!(f.o.retire_source_search_result(&id, &c, &mut f.r));
        assert_eq!(f.o.source_read_context_usage().holds, holds - 1);
        assert_eq!(
            f.o.source_search_usage().issued_lifetime,
            before.issued_lifetime
        );
    }
}
#[test]
fn search_product_faults_are_fenced_for_all_three_original_issuers_without_rollback_or_refilling() {
    for kind in 0..3 {
        for fault in [
            Fault::ProducerRange,
            Fault::ProducerOrder,
            Fault::ProducerStatus,
            Fault::ReleaseAlias,
            Fault::ReleaseVersion,
            Fault::ReleaseRanges,
            Fault::ReleaseStatus,
            Fault::OccurrenceAlias,
            Fault::OccurrenceIndex,
            Fault::NavigationAlias,
            Fault::NavigationVersion,
            Fault::NavigationRange,
            Fault::NavigationIndex,
            Fault::NavigationWrapped,
            Fault::NavigationMissing,
        ] {
            let mut f = owner();
            let q = request();
            let baseline = holds(&f);
            let base = f.o.retention_usage().reserved;
            let generation = f.o.generation();
            let producer = matches!(
                fault,
                Fault::ProducerRange | Fault::ProducerOrder | Fault::ProducerStatus
            );
            if producer {
                f.o.search_result_fault = Some(fault);
            }
            let id = with_facts(&mut f, kind, |o, source, s, sf, r| {
                o.query_managed_search(source, s, &q, sf, &qfacts(&q), r)
            });
            if !producer {
                let id = id.as_ref().unwrap();
                f.o.search_result_fault = Some(fault);
                let rejected = with_facts(&mut f, kind, |o, _, s, sf, r| match fault {
                    Fault::ReleaseAlias
                    | Fault::ReleaseVersion
                    | Fault::ReleaseRanges
                    | Fault::ReleaseStatus => o
                        .release_managed_search(id, s, sf, &qfacts(&q), r)
                        .is_none(),
                    Fault::OccurrenceAlias | Fault::OccurrenceIndex => o
                        .managed_search_occurrence(id, 0, s, sf, &qfacts(&q), r)
                        .is_none(),
                    _ => o
                        .navigate_managed_search(
                            id,
                            s,
                            SourceSearchNavigationV1 {
                                direction: SearchDirection::Next,
                                start: 0,
                                current: None,
                                wrap: false,
                            },
                            sf,
                            &qfacts(&q),
                            r,
                        )
                        .is_none(),
                });
                assert!(rejected, "{kind}/{fault:?}");
            } else {
                assert!(id.is_none());
            }
            assert!(f.o.is_faulted(), "{kind}/{fault:?}");
            assert_eq!(
                f.o.read_result_invariant(),
                Some(ReadResultInvariantV1::NativeSearchResultInvariant)
            );
            assert_eq!(
                ReadResultInvariantV1::NativeSearchResultInvariant.code(),
                "NATIVE_SEARCH_RESULT_INVARIANT"
            );
            assert_eq!(f.o.source_search_usage().results, usize::from(!producer));
            assert_eq!(f.o.source_search_usage().issued_lifetime, 1);
            assert_eq!(f.o.retention_usage().reserved, base);
            assert_eq!(f.o.generation(), generation);
            if let Some(id) = id {
                retire(&mut f, kind, &id);
            }
            assert_eq!(holds(&f), baseline);
            assert_eq!(f.o.source_search_usage().issued_lifetime, 1);
            assert!(f.o.is_faulted());
        }
    }
}
#[test]
fn search_candidate_guard_accepts_native_complete_partial_and_empty_shapes_with_exact_caps() {
    let text = "éaba";
    let all = [ByteRange { start: 0, end: 5 }];
    let m = [
        ByteRange { start: 2, end: 3 },
        ByteRange { start: 4, end: 5 },
    ];
    let bytes = 2 * std::mem::size_of::<ByteRange>();
    for status in [
        SearchStatus::Complete,
        SearchStatus::Exhausted(SearchBudget::Comparisons),
    ] {
        assert!(candidate_valid(text, "a", &all, &m, status, 2, bytes));
    }
    assert!(candidate_valid(
        text,
        "a",
        &all,
        &m,
        SearchStatus::Exhausted(SearchBudget::Matches),
        2,
        bytes
    ));
    assert!(candidate_valid(
        text,
        "a",
        &all,
        &m,
        SearchStatus::Exhausted(SearchBudget::ReplyBytes),
        32,
        bytes
    ));
    assert!(candidate_valid(
        text,
        "",
        &all,
        &[],
        SearchStatus::Complete,
        0,
        0
    ));
    assert!(candidate_valid(
        text,
        "a",
        &[m[0], m[1]],
        &m,
        SearchStatus::Complete,
        2,
        bytes
    ));
}
#[test]
fn search_candidate_guard_rejects_scalar_extent_order_scope_cap_and_status_contradictions() {
    let text = "éaba";
    let all = [ByteRange { start: 0, end: 5 }];
    let a = ByteRange { start: 2, end: 3 };
    let b = ByteRange { start: 4, end: 5 };
    for m in [
        vec![ByteRange { start: 0, end: 1 }],
        vec![ByteRange { start: 2, end: 4 }],
        vec![ByteRange { start: 4, end: 6 }],
        vec![b, a],
        vec![a, a],
    ] {
        assert!(!candidate_valid(
            text,
            "a",
            &all,
            &m,
            SearchStatus::Complete,
            32,
            512
        ));
    }
    assert!(!candidate_valid(
        text,
        "a",
        &[b],
        &[a],
        SearchStatus::Complete,
        32,
        512
    ));
    assert!(!candidate_valid(
        text,
        "a",
        &all,
        &[a, b],
        SearchStatus::Complete,
        1,
        512
    ));
    assert!(!candidate_valid(
        text,
        "a",
        &all,
        &[a],
        SearchStatus::Complete,
        32,
        0
    ));
    assert!(!candidate_valid(
        text,
        "a",
        &all,
        &[a],
        SearchStatus::Exhausted(SearchBudget::Matches),
        32,
        512
    ));
    assert!(!candidate_valid(
        text,
        "a",
        &all,
        &[a],
        SearchStatus::Exhausted(SearchBudget::ReplyBytes),
        32,
        512
    ));
    assert!(!candidate_valid(
        text,
        "",
        &all,
        &[a],
        SearchStatus::Complete,
        32,
        512
    ));
    assert!(!candidate_valid(
        text,
        "",
        &all,
        &[],
        SearchStatus::Exhausted(SearchBudget::Comparisons),
        32,
        512
    ));
}
#[test]
fn search_product_shape_guard_is_not_a_second_matching_or_completeness_oracle() {
    let all = [ByteRange { start: 0, end: 3 }];
    // Deliberately accepted: shape alone neither establishes literal equality
    // at this range nor proves that a complete result omitted no other hits.
    assert!(candidate_valid(
        "aba",
        "a",
        &all,
        &[ByteRange { start: 1, end: 2 }],
        SearchStatus::Complete,
        32,
        512
    ));
    assert!(candidate_valid(
        "aba",
        "a",
        &all,
        &[],
        SearchStatus::Complete,
        32,
        512
    ));
}
#[test]
fn search_guards_preserve_available_empty_navigation_and_incomplete_literal_prefixes() {
    for query in ["", "absent", "o"] {
        let mut f = owner();
        let q = SearchRequest {
            query: query.into(),
            scope: SearchScope::Document,
        };
        let id =
            f.o.query_source_search(
                &f.source,
                &f.s,
                &q,
                &source_facts(&f.s),
                &qfacts(&q),
                &mut f.r,
            )
            .unwrap();
        let reply =
            f.o.release_source_search(&id, &f.s, &source_facts(&f.s), &qfacts(&q), &mut f.r)
                .unwrap();
        assert_eq!(reply.matches.len(), usize::from(query == "o"));
        let nav =
            f.o.navigate_source_search(
                &id,
                &f.s,
                SourceSearchNavigationV1 {
                    direction: SearchDirection::Next,
                    start: 0,
                    current: None,
                    wrap: false,
                },
                &source_facts(&f.s),
                &qfacts(&q),
                &mut f.r,
            )
            .unwrap();
        assert_eq!(nav.found.is_some(), query == "o");
        assert!(!f.o.is_faulted());
    }
    let mut f = owner();
    f.o.search_pool_mut().unwrap().limits.max_matches = 0;
    let q = request();
    let id =
        f.o.query_source_search(
            &f.source,
            &f.s,
            &q,
            &source_facts(&f.s),
            &qfacts(&q),
            &mut f.r,
        )
        .unwrap();
    let reply =
        f.o.release_source_search(&id, &f.s, &source_facts(&f.s), &qfacts(&q), &mut f.r)
            .unwrap();
    assert_eq!(reply.status, SearchStatus::Exhausted(SearchBudget::Matches));
    assert!(reply.matches.is_empty());
    assert!(!f.o.is_faulted());
}
#[test]
fn search_product_observation_is_optional_bounded_and_never_changes_the_fault_or_discharge() {
    for cap in [None, Some(128), Some(1)] {
        let mut f = owner();
        f.r = cap.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let q = request();
        let id =
            f.o.query_source_search(
                &f.source,
                &f.s,
                &q,
                &source_facts(&f.s),
                &qfacts(&q),
                &mut f.r,
            )
            .unwrap();
        f.o.search_result_fault = Some(Fault::NavigationWrapped);
        assert!(
            f.o.navigate_source_search(
                &id,
                &f.s,
                SourceSearchNavigationV1 {
                    direction: SearchDirection::Next,
                    start: 0,
                    current: None,
                    wrap: false
                },
                &source_facts(&f.s),
                &qfacts(&q),
                &mut f.r
            )
            .is_none()
        );
        assert!(f.o.is_faulted());
        retire(&mut f, 0, &id);
        match cap {
            None => assert_eq!(f.r.queued(), 0),
            Some(1) => assert!(f.r.dropped() > 0),
            _ => {
                let mut seen = false;
                while let Some(e) = f.r.pop_front() {
                    seen |= e.boundary == Boundary::SourceSearchNavigationResultValidation
                        && e.disposition == Disposition::Unknown;
                    assert!(e.service_time.is_none());
                }
                assert!(seen);
            }
        }
    }
}
