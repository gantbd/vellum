use super::*;
use vellum_engine::instrumentation::{Boundary, Disposition};
#[test]
fn given_typed_native_search_when_a_budget_or_input_refuses_then_the_caller_can_distinguish_and_retry()
 {
    let (mut o, mut r, _, s) = ready(b"aba");
    o.configure_source_search(search_profile(), &pool_control(), &mut r)
        .unwrap();
    let source = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let q = request("a");
    let current = qfacts(&q);
    let sf = facts(&s);
    let limits = vellum_engine::capture::CaptureLimitsV1 {
        max_bytes: 8192,
        max_nodes: 512,
        max_depth: 16,
    };
    let req = SearchFacadeRequestV1 {
        source: SearchReadContextV1::Source(&source),
        scope: &s,
        actor: &s.context,
        port: SearchFacadePortV1::Evaluate,
        query: &q,
        current: &current,
        facts: SearchReadFactsV1::Source(&sf),
        limits,
    };
    let base = o.retention_usage().reserved;
    for (limits, expected) in [
        (
            vellum_engine::capture::CaptureLimitsV1 {
                max_nodes: 0,
                ..limits
            },
            SourceReadContextErrorV1::InvalidRequest,
        ),
        (
            vellum_engine::capture::CaptureLimitsV1 {
                max_bytes: 1,
                ..limits
            },
            SourceReadContextErrorV1::Limit,
        ),
    ] {
        assert!(
            matches!(o.try_search_facade_v1(SearchFacadeRequestV1 { limits, ..req }, &mut r), Err(e) if e == expected)
        );
        assert_eq!(o.source_search_usage().issued_lifetime, 0);
        assert_eq!(o.retention_usage().reserved, base);
        assert!(!o.is_faulted());
    }
    let Ok(SearchFacadeReplyV1::Evaluated(id)) = o.try_search_facade_v1(req, &mut r) else {
        panic!("retry");
    };
    let Ok(SearchFacadeReplyV1::Released(reply)) = o.try_search_facade_v1(
        SearchFacadeRequestV1 {
            port: SearchFacadePortV1::Release { result: &id },
            ..req
        },
        &mut r,
    ) else {
        panic!("release");
    };
    assert_eq!(
        reply.matches,
        vec![
            ByteRange { start: 0, end: 1 },
            ByteRange { start: 2, end: 3 }
        ]
    );
    assert_eq!(o.source_search_usage().issued_lifetime, 1);
}
fn facade_search(
    o: &mut SourceCoordinator,
    r: &mut Recorder,
    source: &SourceReadContextId,
    s: &SourceReadScopeV1,
    q: &SearchRequest,
    port: SearchFacadePortV1<'_>,
) -> Option<SearchFacadeReplyV1> {
    let mut actor = s.context.clone();
    actor.recipient = "headless-agent".into();
    o.search_facade_v1(
        SearchFacadeRequestV1 {
            source: SearchReadContextV1::Source(source),
            scope: s,
            actor: &actor,
            port,
            query: q,
            current: &qfacts(q),
            facts: SearchReadFactsV1::Source(&facts(s)),
            limits: vellum_engine::capture::CaptureLimitsV1 {
                max_bytes: 8192,
                max_nodes: 512,
                max_depth: 16,
            },
        },
        r,
    )
}
#[test]
fn given_native_search_facade_when_source_changes_then_retained_results_and_navigation_keep_the_original_cut()
 {
    let (mut o, mut r, version, s) = ready(b"aba");
    o.configure_source_search(search_profile(), &pool_control(), &mut r)
        .unwrap();
    let source = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let q = request("a");
    let Some(SearchFacadeReplyV1::Evaluated(id)) = facade_search(
        &mut o,
        &mut r,
        &source,
        &s,
        &q,
        SearchFacadePortV1::Evaluate,
    ) else {
        panic!("evaluation")
    };
    let plan = o
        .prepare(
            &[DocumentEdits {
                expected: version.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 3,
                    expected: "aba".into(),
                    inserted: "zzz".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(plan, &mut r).unwrap();
    let base = o.retention_usage().reserved;
    let held = o.source_read_context_usage().holds;
    let issued = o.source_search_usage().issued_lifetime;
    let Some(SearchFacadeReplyV1::Released(v)) = facade_search(
        &mut o,
        &mut r,
        &source,
        &s,
        &q,
        SearchFacadePortV1::Release { result: &id },
    ) else {
        panic!("release")
    };
    assert_eq!(v.version, version);
    assert_eq!(
        v.matches,
        vec![
            ByteRange { start: 0, end: 1 },
            ByteRange { start: 2, end: 3 }
        ]
    );
    let Some(SearchFacadeReplyV1::Occurrence(first)) = facade_search(
        &mut o,
        &mut r,
        &source,
        &s,
        &q,
        SearchFacadePortV1::Occurrence {
            result: &id,
            index: 0,
        },
    ) else {
        panic!("occurrence")
    };
    let Some(SearchFacadeReplyV1::Navigation(v)) = facade_search(
        &mut o,
        &mut r,
        &source,
        &s,
        &q,
        SearchFacadePortV1::Navigate {
            result: &id,
            navigation: SourceSearchNavigationV1 {
                direction: SearchDirection::Next,
                start: 0,
                current: Some(&first),
                wrap: false,
            },
        },
    ) else {
        panic!("navigation")
    };
    let found = v.found.unwrap();
    assert_eq!(found.range, ByteRange { start: 2, end: 3 });
    assert!(!found.wrapped);
    let Some(SearchFacadeReplyV1::Navigation(v)) = facade_search(
        &mut o,
        &mut r,
        &source,
        &s,
        &q,
        SearchFacadePortV1::Navigate {
            result: &id,
            navigation: SourceSearchNavigationV1 {
                direction: SearchDirection::Next,
                start: 0,
                current: Some(&found.occurrence),
                wrap: true,
            },
        },
    ) else {
        panic!("wrapped navigation")
    };
    assert!(v.found.unwrap().wrapped);
    assert_eq!(o.source_search_usage().issued_lifetime, issued);
    assert_eq!(o.source_read_context_usage().holds, held);
    assert_eq!(o.retention_usage().reserved, base);
    assert_eq!(
        o.read_facade_usage(),
        ReadFacadeUsageV1 {
            busy: false,
            workspace_bytes: 0
        }
    );
    assert!(o.retire_source_search_result(&id, &controls(&s), &mut r));
    assert!(!o.is_faulted());
}
#[test]
fn given_native_search_facade_when_bound_stop_is_accepted_then_result_release_cannot_resurrect_but_retirement_still_works()
 {
    let (mut o, mut r, _, s) = ready(b"aba");
    o.configure_source_search(search_profile(), &pool_control(), &mut r)
        .unwrap();
    let source = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let q = request("a");
    let Some(SearchFacadeReplyV1::Evaluated(id)) = facade_search(
        &mut o,
        &mut r,
        &source,
        &s,
        &q,
        SearchFacadePortV1::Evaluate,
    ) else {
        panic!("evaluation")
    };
    let mut current = qfacts(&q);
    current.not_stopped = Some(false);
    current.capacity = Some(false);
    assert!(
        o.search_facade_v1(
            SearchFacadeRequestV1 {
                source: SearchReadContextV1::Source(&source),
                scope: &s,
                actor: &s.context,
                port: SearchFacadePortV1::Release { result: &id },
                query: &q,
                current: &current,
                facts: SearchReadFactsV1::Source(&facts(&s)),
                limits: vellum_engine::capture::CaptureLimitsV1 {
                    max_bytes: 8192,
                    max_nodes: 512,
                    max_depth: 16
                }
            },
            &mut r
        )
        .is_none()
    );
    assert!(
        facade_search(
            &mut o,
            &mut r,
            &source,
            &s,
            &q,
            SearchFacadePortV1::Release { result: &id }
        )
        .is_none()
    );
    assert_eq!(o.source_search_usage().invalidated, 1);
    assert_eq!(o.read_result_invariant(), None);
    assert!(!o.is_faulted());
    assert!(o.retire_source_search_result(&id, &controls(&s), &mut r));
    assert_eq!(o.source_read_context_usage().holds, 0);
    assert_eq!(o.source_search_usage().issued_lifetime, 1);
}
#[test]
fn given_search_products_when_delivered_then_validation_is_observed_without_new_holds_or_issuance()
{
    let (mut o, mut r, version, s) = ready(b"secret secret");
    o.configure_source_search(search_profile(), &pool_control(), &mut r)
        .unwrap();
    let source = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let q = request("secret");
    r = Recorder::try_with_capacity(128).unwrap();
    let id = o
        .query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
        .unwrap();
    let issued = o.source_search_usage().issued_lifetime;
    let holds = o.source_read_context_usage().holds;
    let reply = o
        .release_source_search(&id, &s, &facts(&s), &qfacts(&q), &mut r)
        .unwrap();
    assert_eq!(reply.version, version);
    assert_eq!(
        reply.matches,
        vec![
            ByteRange { start: 0, end: 6 },
            ByteRange { start: 7, end: 13 }
        ]
    );
    assert!(
        o.source_search_occurrence(&id, 1, &s, &facts(&s), &qfacts(&q), &mut r)
            .is_some()
    );
    let found = o
        .navigate_source_search(
            &id,
            &s,
            nav(SearchDirection::Previous, 13, false),
            &facts(&s),
            &qfacts(&q),
            &mut r,
        )
        .unwrap()
        .found
        .unwrap();
    assert_eq!(found.range, ByteRange { start: 7, end: 13 });
    assert!(!found.wrapped);
    assert_eq!(o.source_search_usage().issued_lifetime, issued);
    assert_eq!(o.source_read_context_usage().holds, holds);
    let events = std::iter::from_fn(|| r.pop_front()).collect::<Vec<_>>();
    for (boundary, count) in [
        (Boundary::SourceSearchResultValidation, 2),
        (Boundary::SourceSearchOccurrenceResultValidation, 1),
        (Boundary::SourceSearchNavigationResultValidation, 1),
    ] {
        assert_eq!(
            events
                .iter()
                .filter(|e| e.boundary == boundary && e.disposition == Disposition::Completed)
                .count(),
            count
        );
    }
    assert!(events.iter().all(|e| e.service_time.is_none()));
    assert!(!format!("{events:?}").contains("secret"));
    assert!(o.retire_source_search_result(&id, &controls(&s), &mut r));
    assert_eq!(o.source_read_context_usage().holds, 0);
    assert!(!o.is_faulted());
}
#[test]
fn given_exhausted_search_when_releasing_its_prefix_then_navigation_refusal_is_not_an_integrity_fault()
 {
    let (mut o, mut r, version, s) = ready(b"aaa");
    let mut profile = search_profile();
    profile.max_matches = 1;
    o.configure_source_search(profile, &pool_control(), &mut r)
        .unwrap();
    let source = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let q = request("a");
    let id = o
        .query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
        .unwrap();
    let reply = o
        .release_source_search(&id, &s, &facts(&s), &qfacts(&q), &mut r)
        .unwrap();
    assert_eq!(reply.version, version);
    assert_eq!(reply.status, SearchStatus::Exhausted(SearchBudget::Matches));
    assert_eq!(reply.matches, vec![ByteRange { start: 0, end: 1 }]);
    assert!(
        o.navigate_source_search(
            &id,
            &s,
            nav(SearchDirection::Next, 0, false),
            &facts(&s),
            &qfacts(&q),
            &mut r
        )
        .is_none()
    );
    assert!(
        o.source_search_occurrence(&id, 0, &s, &facts(&s), &qfacts(&q), &mut r)
            .is_some()
    );
    assert_eq!(o.read_result_invariant(), None);
    assert!(!o.is_faulted());
    assert!(o.retire_source_search_result(&id, &controls(&s), &mut r));
}
fn nav(direction: SearchDirection, start: usize, wrap: bool) -> SourceSearchNavigationV1<'static> {
    SourceSearchNavigationV1 {
        direction,
        start,
        current: None,
        wrap,
    }
}
#[test]
fn managed_navigation_visits_original_ordered_occurrences_and_explicit_wraps_without_new_holds() {
    let (mut o, mut r, _, s) = ready(b"aba");
    o.configure_source_search(search_profile(), &pool_control(), &mut r)
        .unwrap();
    let source = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let q = request("a");
    let result = o
        .query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
        .unwrap();
    let first = o
        .navigate_source_search(
            &result,
            &s,
            nav(SearchDirection::Next, 0, false),
            &facts(&s),
            &qfacts(&q),
            &mut r,
        )
        .unwrap()
        .found
        .unwrap();
    assert_eq!(first.range, ByteRange { start: 0, end: 1 });
    assert!(!first.wrapped);
    let next = SourceSearchNavigationV1 {
        direction: SearchDirection::Next,
        start: 0,
        current: Some(&first.occurrence),
        wrap: false,
    };
    let last = o
        .navigate_source_search(&result, &s, next, &facts(&s), &qfacts(&q), &mut r)
        .unwrap()
        .found
        .unwrap();
    assert_eq!(last.range, ByteRange { start: 2, end: 3 });
    let next = SourceSearchNavigationV1 {
        direction: SearchDirection::Next,
        start: 0,
        current: Some(&last.occurrence),
        wrap: false,
    };
    assert!(
        o.navigate_source_search(&result, &s, next, &facts(&s), &qfacts(&q), &mut r)
            .unwrap()
            .found
            .is_none()
    );
    let next = SourceSearchNavigationV1 {
        direction: SearchDirection::Next,
        start: 0,
        current: Some(&last.occurrence),
        wrap: true,
    };
    let wrapped = o
        .navigate_source_search(&result, &s, next, &facts(&s), &qfacts(&q), &mut r)
        .unwrap()
        .found
        .unwrap();
    assert_eq!(wrapped.occurrence, first.occurrence);
    assert!(wrapped.wrapped);
    let prev = SourceSearchNavigationV1 {
        direction: SearchDirection::Previous,
        start: 0,
        current: Some(&first.occurrence),
        wrap: true,
    };
    let wrapped = o
        .navigate_source_search(&result, &s, prev, &facts(&s), &qfacts(&q), &mut r)
        .unwrap()
        .found
        .unwrap();
    assert_eq!(wrapped.occurrence, last.occurrence);
    assert!(wrapped.wrapped);
    assert_eq!(o.source_read_context_usage().holds, 1);
    assert_eq!(o.source_read_context_usage().issued_lifetime, 2);
    assert_eq!(o.source_search_usage().issued_lifetime, 1);
}

#[test]
fn original_scalar_start_is_checked_before_loss_and_navigation_never_refreshes_after_publication() {
    let (mut o, mut r, d, s) = ready("éaé".as_bytes());
    o.configure_source_search(search_profile(), &pool_control(), &mut r)
        .unwrap();
    let source = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let q = request("é");
    let result = o
        .query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
        .unwrap();
    let edit = o
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 5,
                    expected: "éaé".into(),
                    inserted: "x".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(edit, &mut r).unwrap();
    let generation = o.generation();
    let mut loss = qfacts(&q);
    loss.grant = Some(false);
    for start in [1, 4, 6, usize::MAX] {
        assert!(
            o.navigate_source_search(
                &result,
                &s,
                nav(SearchDirection::Next, start, true),
                &facts(&s),
                &loss,
                &mut r
            )
            .is_none()
        );
    }
    assert_eq!(o.source_search_usage().invalidated, 0);
    let found = o
        .navigate_source_search(
            &result,
            &s,
            nav(SearchDirection::Previous, 5, false),
            &facts(&s),
            &qfacts(&q),
            &mut r,
        )
        .unwrap();
    assert_eq!(found.version, d);
    assert_eq!(found.found.unwrap().range, ByteRange { start: 3, end: 5 });
    assert_eq!(o.generation(), generation);
    assert!(
        o.navigate_source_search(
            &result,
            &s,
            nav(SearchDirection::Previous, 5, false),
            &facts(&s),
            &loss,
            &mut r
        )
        .is_none()
    );
    assert!(
        o.navigate_source_search(
            &result,
            &s,
            nav(SearchDirection::Previous, 5, false),
            &facts(&s),
            &qfacts(&q),
            &mut r
        )
        .is_none()
    );
    assert!(
        o.query_source_read_context(&source, &s, one_byte(), &facts(&s), &mut r)
            .is_none()
    ); // scalar split
    assert!(
        o.query_source_read_context(
            &source,
            &s,
            SourceContextQueryV1::Range {
                start: BytePosition(0),
                end: BytePosition(2)
            },
            &facts(&s),
            &mut r
        )
        .is_some()
    );
}

#[test]
fn cross_result_and_retired_occurrences_cannot_be_rebound_or_poison_an_identical_peer() {
    let (mut o, mut r, _, s) = ready(b"aba");
    o.configure_source_search(search_profile(), &pool_control(), &mut r)
        .unwrap();
    let source = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let q = request("a");
    let first = o
        .query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
        .unwrap();
    let occurrence = o
        .navigate_source_search(
            &first,
            &s,
            nav(SearchDirection::Next, 0, false),
            &facts(&s),
            &qfacts(&q),
            &mut r,
        )
        .unwrap()
        .found
        .unwrap()
        .occurrence;
    let peer = o
        .query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
        .unwrap();
    let mut loss = qfacts(&q);
    loss.retained = Some(false);
    let navigation = SourceSearchNavigationV1 {
        direction: SearchDirection::Next,
        start: 0,
        current: Some(&occurrence),
        wrap: true,
    };
    assert!(
        o.navigate_source_search(&peer, &s, navigation, &facts(&s), &loss, &mut r)
            .is_none()
    );
    assert!(o.retire_source_search_result(&first, &controls(&s), &mut r));
    assert!(
        o.navigate_source_search(&first, &s, navigation, &facts(&s), &qfacts(&q), &mut r)
            .is_none()
    );
    assert!(
        o.navigate_source_search(&peer, &s, navigation, &facts(&s), &loss, &mut r)
            .is_none()
    );
    assert_eq!(o.source_search_usage().invalidated, 0);
    assert!(
        o.navigate_source_search(
            &peer,
            &s,
            nav(SearchDirection::Next, 0, false),
            &facts(&s),
            &qfacts(&q),
            &mut r
        )
        .unwrap()
        .found
        .is_some()
    );
    assert_eq!(o.source_read_context_usage().holds, 1);
}

#[test]
fn complete_empty_navigation_is_available_but_incomplete_navigation_and_unknown_facts_are_not() {
    for (query, budget, available) in [("z", 4096, true), ("", 0, true), ("a", 0, false)] {
        let (mut o, mut r, _, s) = ready(b"aba");
        let mut p = search_profile();
        p.max_comparisons = budget;
        o.configure_source_search(p, &pool_control(), &mut r)
            .unwrap();
        let source = o
            .capture_source_read_context(&s, &facts(&s), &mut r)
            .unwrap();
        let q = request(query);
        let result = o
            .query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
            .unwrap();
        let n = nav(SearchDirection::Next, 0, true);
        assert_eq!(
            o.navigate_source_search(&result, &s, n, &facts(&s), &qfacts(&q), &mut r)
                .is_some(),
            available
        );
        let mut unknown = qfacts(&q);
        unknown.grant = None;
        assert!(
            o.navigate_source_search(&result, &s, n, &facts(&s), &unknown, &mut r)
                .is_none()
        );
        assert_eq!(o.source_search_usage().invalidated, 0);
        let mut loss = qfacts(&q);
        loss.grant = Some(false);
        loss.capacity = Some(false);
        let mut pressure = facts(&s);
        pressure.resources_ready = Some(false);
        assert!(
            o.navigate_source_search(&result, &s, n, &pressure, &loss, &mut r)
                .is_none()
        );
        assert_eq!(o.source_search_usage().invalidated, 1);
        assert!(
            o.release_source_search(&result, &s, &facts(&s), &qfacts(&q), &mut r)
                .is_none()
        );
    }
}

#[test]
fn navigation_diagnostics_are_optional_bounded_and_redacted_without_affecting_results() {
    use vellum_engine::instrumentation::{Boundary, Disposition};
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _, _, s) = ready(b"secret secret");
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        o.configure_source_search(search_profile(), &pool_control(), &mut r)
            .unwrap();
        let source = o
            .capture_source_read_context(&s, &facts(&s), &mut r)
            .unwrap();
        let q = request("secret");
        let result = o
            .query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
            .unwrap();
        let n = nav(SearchDirection::Previous, 13, true);
        let found = o
            .navigate_source_search(&result, &s, n, &facts(&s), &qfacts(&q), &mut r)
            .unwrap();
        assert_eq!(format!("{n:?}"), "SourceSearchNavigationV1(<redacted>)");
        assert_eq!(
            format!("{found:?}"),
            "SourceSearchNavigationReplyV1(<redacted>)"
        );
        let found = found.found.unwrap();
        assert_eq!(found.range, ByteRange { start: 7, end: 13 });
        assert_eq!(
            format!("{:?}", found.occurrence),
            "SourceSearchOccurrenceId(<opaque>)"
        );
        assert_eq!(format!("{found:?}"), "SourceSearchFoundV1(<redacted>)");
        let mut loss = facts(&s);
        loss.source_readable = Some(false);
        assert!(
            o.navigate_source_search(&result, &s, n, &loss, &qfacts(&q), &mut r)
                .is_none()
        );
        assert!(o.retire_source_search_result(&result, &controls(&s), &mut r));
        assert_eq!(o.source_read_context_usage().holds, 0);
        if capacity == Some(128) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert!(
                events
                    .iter()
                    .any(|e| e.boundary == Boundary::SourceSearchNavigation
                        && e.disposition == Disposition::Completed)
            );
            assert!(
                events
                    .iter()
                    .any(|e| e.boundary == Boundary::SourceSearchNavigation
                        && e.disposition == Disposition::Refused)
            );
            assert!(events.iter().all(|e| e.service_time.is_none()));
            assert!(!format!("{events:?}").contains("secret"));
        } else if capacity == Some(1) {
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}
