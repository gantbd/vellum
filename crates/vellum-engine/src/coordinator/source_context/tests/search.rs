use super::super::search::{ResultRow, request_bytes};
use super::*;
#[test]
fn shutdown_inventory_search_records_its_actual_issuer_hold_dependency_without_acquiring_another() {
    use crate::coordinator::shutdown::tests::{inspect, inspect_read};
    let (mut o, mut r, s) = configured(limits());
    let ctx = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    o.configure_source_search(profile(), &controls(), &mut r)
        .unwrap();
    let q = request();
    let _result = o
        .query_source_search(&ctx, &s, &q, &facts(&s), &query_facts(&q), &mut r)
        .unwrap();
    let before = (
        o.source_search_usage(),
        o.source_read_context_usage(),
        o.retention_usage(),
    );
    let cut = inspect(&o, &mut r);
    let view = inspect_read(&o, &cut, &mut r);
    let search = view
        .owners
        .iter()
        .find(|r| r.owner.role() == ShutdownOwnerRoleV1::ManagedSearch)
        .unwrap();
    let source = view
        .owners
        .iter()
        .find(|r| r.owner.role() == ShutdownOwnerRoleV1::SourceContexts)
        .unwrap();
    assert_eq!(search.retained_records, Some(1));
    assert_eq!(source.retained_records, Some(2));
    assert!(view.dependencies.iter().any(|e| e.dependent
        == ShutdownInventoryTargetV1::Owner(search.owner.clone())
        && e.required == ShutdownInventoryTargetV1::Owner(source.owner.clone())));
    assert_eq!(
        (
            o.source_search_usage(),
            o.source_read_context_usage(),
            o.retention_usage()
        ),
        before
    );
}
#[test]
fn shutdown_refuses_search_acquisition_but_preserves_original_result_release_and_retirement() {
    for configured_search in [false, true] {
        let (mut o, mut r, s) = configured(limits());
        let source = o
            .capture_source_read_context(&s, &facts(&s), &mut r)
            .unwrap();
        let q = request();
        let original = if configured_search {
            o.configure_source_search(profile(), &controls(), &mut r)
                .unwrap();
            Some(
                o.query_source_search(&source, &s, &q, &facts(&s), &query_facts(&q), &mut r)
                    .unwrap(),
            )
        } else {
            None
        };
        crate::coordinator::shutdown::tests::install(&mut o, &mut r);
        let before = (
            o.source_search_usage(),
            o.source_read_context_usage(),
            o.retention_usage(),
        );
        assert_eq!(
            o.configure_source_search(profile(), &controls(), &mut r),
            if configured_search {
                Ok(false)
            } else {
                Err(Error::Unavailable)
            }
        );
        assert!(
            o.query_source_search(&source, &s, &q, &facts(&s), &query_facts(&q), &mut r)
                .is_none()
        );
        assert_eq!(
            (
                o.source_search_usage(),
                o.source_read_context_usage(),
                o.retention_usage()
            ),
            before
        );
        if let Some(original) = original {
            assert!(
                o.release_source_search(&original, &s, &facts(&s), &query_facts(&q), &mut r)
                    .is_some()
            );
            assert!(o.retire_source_search_result(&original, &discharge(&s), &mut r));
            assert_eq!(o.retire_source_search(&controls(), &mut r), Ok(true));
        }
        assert!(o.stop_source_read_context(&source, &discharge(&s), &mut r));
        assert!(o.retire_source_read_context(&source, &discharge(&s), &mut r));
        assert_eq!(o.retire_source_read_contexts(&controls(), &mut r), Ok(true));
        assert!(o.retention_usage().reserved > 0);
        assert!(!o.is_faulted());
    }
}

fn profile() -> SourceSearchLimitsV1 {
    SourceSearchLimitsV1 {
        max_results: 4,
        max_issued: 8,
        max_request_bytes: 4096,
        max_capture_bytes: 8192,
        max_reply_bytes: 4096,
        max_pool_bytes: 1_048_576,
        max_query_bytes: 128,
        max_scope_ranges: 8,
        max_comparisons: 4096,
        max_matches: 32,
        max_replacement_bytes: 4096,
    }
}
fn request() -> SearchRequest {
    SearchRequest {
        query: "a".into(),
        scope: SearchScope::Document,
    }
}
fn query_facts(q: &SearchRequest) -> TrustedSourceSearchFactsV1<'_> {
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

#[test]
fn managed_result_reserves_original_hold_and_protected_retirement_discharges_once() {
    let (mut o, mut r, s) = configured(limits());
    let source = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let before = o.retention_usage().reserved;
    assert_eq!(
        o.configure_source_search(profile(), &controls(), &mut r),
        Ok(true)
    );
    let funded = o.retention_usage().reserved;
    assert!(funded > before);
    let q = request();
    let result = o
        .query_source_search(&source, &s, &q, &facts(&s), &query_facts(&q), &mut r)
        .unwrap();
    assert_eq!(o.source_read_context_usage().holds, 1);
    let reply = o
        .release_source_search(&result, &s, &facts(&s), &query_facts(&q), &mut r)
        .unwrap();
    assert_eq!(reply.matches, vec![ByteRange { start: 0, end: 1 }]);
    assert_eq!(reply.status, SearchStatus::Complete);
    assert!(o.stop_source_read_context(&source, &discharge(&s), &mut r));
    assert!(!o.retire_source_read_context(&source, &discharge(&s), &mut r));
    assert!(
        o.release_source_search(&result, &s, &facts(&s), &query_facts(&q), &mut r)
            .is_none()
    );
    assert!(o.retire_source_search_result(&result, &discharge(&s), &mut r));
    assert!(!o.retire_source_search_result(&result, &discharge(&s), &mut r));
    assert_eq!(o.source_read_context_usage().holds, 0);
    assert_eq!(o.retention_usage().reserved, funded);
    assert_eq!(o.source_search_usage().issued_lifetime, 1);
    assert!(o.retire_source_read_context(&source, &discharge(&s), &mut r));
    assert_eq!(o.retire_source_read_contexts(&controls(), &mut r), Ok(true));
    assert_eq!(o.retire_source_search(&controls(), &mut r), Ok(true));
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(
        o.configure_source_search(profile(), &controls(), &mut r),
        Err(Error::Unavailable)
    );
    assert_eq!(
        o.retire_source_read_contexts(&controls(), &mut r),
        Ok(false)
    );
    assert_eq!(o.source_search_usage().issued_lifetime, 1);
    assert!(o.source_search_usage().closed);
}

#[test]
fn query_loss_is_absorbing_but_never_revokes_independent_source_reader() {
    for which in 0..6 {
        for value in [None, Some(false)] {
            let (mut o, mut r, s) = configured(limits());
            o.configure_source_search(profile(), &controls(), &mut r)
                .unwrap();
            let source = o
                .capture_source_read_context(&s, &facts(&s), &mut r)
                .unwrap();
            let q = request();
            let result = o
                .query_source_search(&source, &s, &q, &facts(&s), &query_facts(&q), &mut r)
                .unwrap();
            let mut f = query_facts(&q);
            match which {
                0 => f.grant = value,
                1 => f.scope = value,
                2 => f.continuity = value,
                3 => f.capacity = value,
                4 => f.retained = value,
                _ => f.not_stopped = value,
            }
            assert!(
                o.release_source_search(&result, &s, &facts(&s), &f, &mut r)
                    .is_none()
            );
            let lost = value == Some(false) && which != 3;
            assert_eq!(
                o.release_source_search(&result, &s, &facts(&s), &query_facts(&q), &mut r)
                    .is_none(),
                lost
            );
            assert!(
                o.query_source_read_context(&source, &s, range(1), &facts(&s), &mut r)
                    .is_some()
            );
        }
    }
}

#[test]
fn malformed_changed_and_foreign_requests_cannot_poison_original_result_or_reader() {
    let (mut o, mut r, s) = configured(limits());
    o.configure_source_search(profile(), &controls(), &mut r)
        .unwrap();
    let source = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let q = request();
    let result = o
        .query_source_search(&source, &s, &q, &facts(&s), &query_facts(&q), &mut r)
        .unwrap();
    let mut changed = request();
    changed.query = "x".into();
    let mut loss = query_facts(&changed);
    loss.grant = Some(false);
    assert!(
        o.release_source_search(&result, &s, &facts(&s), &loss, &mut r)
            .is_none()
    );
    let mut foreign_scope = s.clone();
    foreign_scope.policy.push('x');
    let mut source_loss = facts(&foreign_scope);
    source_loss.authorized = Some(false);
    assert!(
        o.release_source_search(
            &result,
            &foreign_scope,
            &source_loss,
            &query_facts(&q),
            &mut r
        )
        .is_none()
    );
    let malformed = SearchRequest {
        query: "a".into(),
        scope: SearchScope::Captured(vec![ByteRange { start: 3, end: 2 }]),
    };
    let mut source_loss = facts(&s);
    source_loss.source_readable = Some(false);
    assert!(
        o.query_source_search(
            &source,
            &s,
            &malformed,
            &source_loss,
            &query_facts(&malformed),
            &mut r
        )
        .is_none()
    );
    let forged = SourceSearchResultId {
        owner: Arc::new(()),
        slot: result.slot,
    };
    let mut loss = query_facts(&q);
    loss.grant = Some(false);
    assert!(
        o.release_source_search(&forged, &s, &facts(&s), &loss, &mut r)
            .is_none()
    );
    assert!(!o.retire_source_search_result(&forged, &discharge(&s), &mut r));
    assert!(
        o.release_source_search(&result, &s, &facts(&s), &query_facts(&q), &mut r)
            .is_some()
    );
    assert_eq!(o.source_read_context_usage().issued_lifetime, 2);
    assert_eq!(o.source_search_usage().invalidated, 0);
    assert!(!o.faulted);
}

#[test]
fn source_loss_absorbs_under_full_result_occupancy_but_query_loss_does_not_close_source() {
    let mut p = profile();
    p.max_results = 1;
    p.max_issued = 1;
    let (mut o, mut r, s) = configured(limits());
    o.configure_source_search(p, &controls(), &mut r).unwrap();
    let source = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let q = request();
    let result = o
        .query_source_search(&source, &s, &q, &facts(&s), &query_facts(&q), &mut r)
        .unwrap();
    let mut source_loss = facts(&s);
    source_loss.authorized = Some(false);
    assert!(
        o.query_source_search(&source, &s, &q, &source_loss, &query_facts(&q), &mut r)
            .is_none()
    );
    assert!(
        o.release_source_search(&result, &s, &facts(&s), &query_facts(&q), &mut r)
            .is_none()
    );
    assert_eq!(o.source_read_context_usage().retained_snapshots, 1);
    assert!(o.retire_source_search_result(&result, &discharge(&s), &mut r));
    assert_eq!(o.source_read_context_usage().retained_snapshots, 0);
    assert_eq!(o.source_search_usage().issued_lifetime, 1);
}

#[test]
fn complete_request_capture_and_reply_envelopes_use_native_units_and_source_intersection() {
    let (_, _, s) = base();
    let q = request();
    let request_bound = request_bytes(&s, &q).unwrap();
    let capture_bound = std::mem::size_of::<ResultRow>() + scope_bytes(&s).unwrap()
        - std::mem::size_of::<SourceReadScopeV1>()
        + q.query.len()
        + std::mem::size_of::<ByteRange>();
    let reply_bound = std::mem::size_of::<SourceSearchReplyV1>() + std::mem::size_of::<ByteRange>();
    for coordinate in 0..4 {
        for delta in [-1isize, 0, 1] {
            let mut p = profile();
            let mut source_p = limits();
            match coordinate {
                0 => p.max_request_bytes = request_bound.checked_add_signed(delta).unwrap(),
                1 => p.max_capture_bytes = capture_bound.checked_add_signed(delta).unwrap(),
                2 => p.max_reply_bytes = reply_bound.checked_add_signed(delta).unwrap(),
                _ => source_p.max_reply_bytes = reply_bound.checked_add_signed(delta).unwrap(),
            }
            let (mut o, mut r, s) = configured(source_p);
            o.configure_source_search(p, &controls(), &mut r).unwrap();
            let source = o
                .capture_source_read_context(&s, &facts(&s), &mut r)
                .unwrap();
            let result =
                o.query_source_search(&source, &s, &q, &facts(&s), &query_facts(&q), &mut r);
            if coordinate == 0 && delta < 0 {
                assert!(result.is_none());
                assert_eq!(o.source_read_context_usage().holds, 0);
                assert_eq!(o.source_search_usage().issued_lifetime, 0);
                continue;
            }
            let result = result.unwrap();
            let reply = o
                .release_source_search(&result, &s, &facts(&s), &query_facts(&q), &mut r)
                .unwrap();
            assert_eq!(reply.matches.len(), usize::from(delta >= 0));
            assert_eq!(
                reply.status,
                if delta >= 0 {
                    SearchStatus::Complete
                } else {
                    SearchStatus::Exhausted(SearchBudget::ReplyBytes)
                }
            );
        }
    }
}

#[test]
fn pool_funding_is_checked_at_exact_parent_and_local_bounds_including_zero_result_workspace() {
    for zero in [false, true] {
        for coordinate in 0..2 {
            for delta in [-1isize, 0, 1] {
                let (mut o, mut r, _) = base();
                let mut p = profile();
                if zero {
                    p.max_results = 0;
                }
                let charge = p.required_pool_bytes().unwrap() + "engine".len() + "local".len();
                let source_charge =
                    limits().required_pool_bytes().unwrap() + "engine".len() + "local".len();
                let bound = charge.checked_add_signed(delta).unwrap();
                if coordinate == 0 {
                    p.max_pool_bytes = bound;
                }
                o.configure_retention_budget(
                    if coordinate == 1 {
                        source_charge + bound
                    } else {
                        usize::MAX
                    },
                    &mut r,
                )
                .unwrap();
                o.configure_source_read_contexts("engine", "local", limits(), &controls(), &mut r)
                    .unwrap();
                assert_eq!(
                    o.configure_source_search(p, &controls(), &mut r),
                    if delta >= 0 {
                        Ok(true)
                    } else {
                        Err(Error::Limit)
                    }
                );
                assert_eq!(
                    o.retention_usage().reserved,
                    source_charge + if delta >= 0 { charge } else { 0 }
                );
                if delta >= 0 {
                    assert_eq!(o.configure_source_search(p, &controls(), &mut r), Ok(false));
                    assert!(charge > 0);
                }
            }
        }
    }
}

#[test]
fn owned_hold_corruption_faults_without_refund_and_protected_cleanup_survives_owner_fault() {
    let (mut o, mut r, s) = configured(limits());
    o.configure_source_search(profile(), &controls(), &mut r)
        .unwrap();
    let source = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let q = request();
    let result = o
        .query_source_search(&source, &s, &q, &facts(&s), &query_facts(&q), &mut r)
        .unwrap();
    let reserved = o.retention_usage().reserved;
    let State::Active(p) = &mut o.source_contexts else {
        unreachable!()
    };
    p.rows[source.slot].as_mut().unwrap().holds += 1;
    assert!(
        o.release_source_search(&result, &s, &facts(&s), &query_facts(&q), &mut r)
            .is_none()
    );
    assert!(o.faulted);
    assert!(!o.retire_source_search_result(&result, &discharge(&s), &mut r));
    assert_eq!(o.retention_usage().reserved, reserved);
    assert_eq!(o.source_search_usage().results, 1);
    let State::Active(p) = &mut o.source_contexts else {
        unreachable!()
    };
    p.rows[source.slot].as_mut().unwrap().holds -= 1;
    assert!(o.stop_source_read_context(&source, &discharge(&s), &mut r));
    assert!(o.retire_source_search_result(&result, &discharge(&s), &mut r));
    assert_eq!(o.retire_source_search(&controls(), &mut r), Ok(true));
    assert!(o.faulted);
}

#[test]
fn navigation_complete_request_and_reply_bounds_intersect_both_actual_profiles() {
    let (_, _, s) = base();
    let q = request();
    let request_bound = request_bytes(&s, &q).unwrap()
        + std::mem::size_of::<SourceSearchNavigationV1<'_>>()
        + std::mem::size_of::<SourceSearchOccurrenceId>();
    let reply_bound = std::mem::size_of::<SourceSearchNavigationReplyV1>();
    for coordinate in 0..4 {
        for delta in [-1isize, 0, 1] {
            let mut p = profile();
            let mut sp = limits();
            match coordinate {
                0 => p.max_request_bytes = request_bound.checked_add_signed(delta).unwrap(),
                1 => sp.max_request_bytes = request_bound.checked_add_signed(delta).unwrap(),
                2 => p.max_reply_bytes = reply_bound.checked_add_signed(delta).unwrap(),
                _ => sp.max_reply_bytes = reply_bound.checked_add_signed(delta).unwrap(),
            }
            let (mut o, mut r, s) = configured(sp);
            o.configure_source_search(p, &controls(), &mut r).unwrap();
            let source = o
                .capture_source_read_context(&s, &facts(&s), &mut r)
                .unwrap();
            let result = o
                .query_source_search(&source, &s, &q, &facts(&s), &query_facts(&q), &mut r)
                .unwrap();
            let n = SourceSearchNavigationV1 {
                direction: SearchDirection::Next,
                start: 0,
                current: None,
                wrap: true,
            };
            let first =
                o.navigate_source_search(&result, &s, n, &facts(&s), &query_facts(&q), &mut r);
            if coordinate >= 2 && delta < 0 {
                assert!(first.is_none());
            } else {
                let occurrence = first.unwrap().found.unwrap().occurrence;
                let n = SourceSearchNavigationV1 {
                    current: Some(&occurrence),
                    ..n
                };
                assert_eq!(
                    o.navigate_source_search(&result, &s, n, &facts(&s), &query_facts(&q), &mut r)
                        .is_some(),
                    delta >= 0
                );
                if coordinate < 2 && delta < 0 {
                    let mut loss = query_facts(&q);
                    loss.grant = Some(false);
                    assert!(
                        o.navigate_source_search(&result, &s, n, &facts(&s), &loss, &mut r)
                            .is_none()
                    );
                    assert_eq!(o.source_search_usage().invalidated, 0);
                }
            }
            if coordinate >= 2 && delta < 0 {
                let mut loss = query_facts(&q);
                loss.grant = Some(false);
                assert!(
                    o.navigate_source_search(&result, &s, n, &facts(&s), &loss, &mut r)
                        .is_none()
                );
                assert_eq!(o.source_search_usage().invalidated, 1);
            }
            assert_eq!(o.source_read_context_usage().holds, 1);
            assert_eq!(o.source_search_usage().issued_lifetime, 1);
        }
    }
}

#[test]
fn finite_navigation_kernel_matches_an_independent_materialized_directional_walk() {
    for mask in 0..32u32 {
        let matches: Vec<_> = (0..5)
            .filter(|i| mask & (1 << i) != 0)
            .map(|i| ByteRange {
                start: i,
                end: i + 1,
            })
            .collect();
        for direction in [SearchDirection::Next, SearchDirection::Previous] {
            let mut order: Vec<_> = (0..matches.len()).collect();
            if direction == SearchDirection::Previous {
                order.reverse();
            }
            for start in 0..=5 {
                for current in std::iter::once(None).chain((0..matches.len()).map(Some)) {
                    for wrap in [false, true] {
                        let candidate = match current {
                            Some(i) => order
                                .iter()
                                .position(|v| *v == i)
                                .and_then(|p| order.get(p + 1))
                                .copied(),
                            None => order.iter().copied().find(|i| match direction {
                                SearchDirection::Next => matches[*i].start >= start,
                                SearchDirection::Previous => matches[*i].start <= start,
                            }),
                        };
                        let expected = candidate.map(|i| (i, false)).or_else(|| {
                            if wrap {
                                order.first().map(|i| (*i, true))
                            } else {
                                None
                            }
                        });
                        assert_eq!(
                            crate::coordinator::search::navigate_ranges(
                                &matches, direction, start, current, wrap
                            ),
                            expected
                        );
                    }
                }
            }
        }
    }
}
