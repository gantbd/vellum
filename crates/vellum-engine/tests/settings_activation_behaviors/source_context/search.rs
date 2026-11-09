use super::*;
#[path = "search/consumers.rs"]
mod consumers;
#[path = "search/navigation.rs"]
mod navigation;
#[path = "search/timing.rs"]
mod timing;
pub(super) fn search_profile() -> SourceSearchLimitsV1 {
    SourceSearchLimitsV1 {
        max_results: 4,
        max_issued: 8,
        max_request_bytes: 4096,
        max_capture_bytes: 8192,
        max_reply_bytes: 4096,
        max_pool_bytes: 262144,
        max_query_bytes: 128,
        max_scope_ranges: 16,
        max_comparisons: 4096,
        max_matches: 32,
        max_replacement_bytes: 4096,
    }
}
pub(super) fn qfacts(q: &SearchRequest) -> TrustedSourceSearchFactsV1<'_> {
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
pub(super) fn request(query: &str) -> SearchRequest {
    SearchRequest {
        query: query.into(),
        scope: SearchScope::Document,
    }
}

#[test]
fn search_uses_original_source_cut_after_publication_and_never_grants_stale_edit_authority() {
    let (mut o, mut r, d, s) = ready(b"aba");
    o.configure_source_search(search_profile(), &pool_control(), &mut r)
        .unwrap();
    let source = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let edit = o
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 3,
                    expected: "aba".into(),
                    inserted: "xyz".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(edit, &mut r).unwrap();
    let generation = o.generation();
    let q = request("a");
    let result = o
        .query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
        .unwrap();
    let reply = o
        .release_source_search(&result, &s, &facts(&s), &qfacts(&q), &mut r)
        .unwrap();
    assert_eq!(reply.version, d);
    assert_eq!(
        reply.matches,
        vec![
            ByteRange { start: 0, end: 1 },
            ByteRange { start: 2, end: 3 }
        ]
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(
        o.prepare(
            &[DocumentEdits {
                expected: reply.version.clone(),
                edits: vec![]
            }],
            &[],
            &mut r
        )
        .err(),
        Some(CoordinatorError::Stale)
    );
    assert_eq!(o.source_read_context_usage().holds, 1);
    assert_eq!(format!("{result:?}"), "SourceSearchResultId(<opaque>)");
    assert_eq!(format!("{reply:?}"), "SourceSearchReplyV1(<redacted>)");
}

#[test]
fn all_source_read_facts_remain_independent_and_only_recognized_source_loss_is_absorbing() {
    for index in 0..10 {
        for value in [None, Some(false)] {
            let (mut o, mut r, _, s) = ready(b"ab");
            o.configure_source_search(search_profile(), &pool_control(), &mut r)
                .unwrap();
            let source = o
                .capture_source_read_context(&s, &facts(&s), &mut r)
                .unwrap();
            let q = request("a");
            let result = o
                .query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
                .unwrap();
            let mut f = facts(&s);
            set_fact(&mut f, index, value);
            assert!(
                o.release_source_search(&result, &s, &f, &qfacts(&q), &mut r)
                    .is_none()
            );
            let lost = value == Some(false) && [0, 1, 2, 3, 5, 6].contains(&index);
            assert_eq!(
                o.release_source_search(&result, &s, &facts(&s), &qfacts(&q), &mut r)
                    .is_none(),
                lost
            );
            assert_eq!(o.source_search_usage().invalidated, 0);
            assert!(o.retire_source_search_result(&result, &controls(&s), &mut r));
            assert_eq!(o.source_read_context_usage().holds, 0);
        }
    }
}

#[test]
fn result_retirement_never_refills_lifetime_and_source_holds_share_the_original_issuer() {
    let (mut o, mut r, _, s) = ready(b"ab");
    let mut p = search_profile();
    p.max_results = 1;
    p.max_issued = 2;
    o.configure_source_search(p, &pool_control(), &mut r)
        .unwrap();
    let source = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let q = request("a");
    let mut previous = None;
    for issued in 1..=2 {
        let result = o
            .query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
            .unwrap();
        if let Some(old) = previous {
            assert_ne!(old, result);
            assert!(
                o.release_source_search(&old, &s, &facts(&s), &qfacts(&q), &mut r)
                    .is_none()
            );
        }
        assert!(
            o.query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
                .is_none()
        );
        assert!(o.retire_source_search_result(&result, &controls(&s), &mut r));
        assert_eq!(o.source_search_usage().issued_lifetime, issued);
        assert_eq!(o.source_read_context_usage().issued_lifetime, 1 + issued);
        previous = Some(result);
    }
    assert!(
        o.query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
            .is_none()
    );
    assert_eq!(o.source_read_context_usage().holds, 0);
    assert_eq!(o.source_search_usage().results, 0);
}

#[test]
fn managed_search_shares_literal_kernel_unicode_scope_empty_and_partial_semantics() {
    for (query, scope, comparisons, expected, status) in [
        (
            "é",
            SearchScope::Document,
            4096,
            vec![
                ByteRange { start: 0, end: 2 },
                ByteRange { start: 3, end: 5 },
            ],
            SearchStatus::Complete,
        ),
        ("", SearchScope::Document, 0, vec![], SearchStatus::Complete),
        (
            "é",
            SearchScope::Captured(vec![ByteRange { start: 3, end: 5 }]),
            4096,
            vec![ByteRange { start: 3, end: 5 }],
            SearchStatus::Complete,
        ),
        (
            "é",
            SearchScope::Document,
            1,
            vec![],
            SearchStatus::Exhausted(SearchBudget::Comparisons),
        ),
    ] {
        let (mut o, mut r, _, s) = ready("é\né".as_bytes());
        let mut p = search_profile();
        p.max_comparisons = comparisons;
        o.configure_source_search(p, &pool_control(), &mut r)
            .unwrap();
        let source = o
            .capture_source_read_context(&s, &facts(&s), &mut r)
            .unwrap();
        let q = SearchRequest {
            query: query.into(),
            scope,
        };
        let result = o
            .query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
            .unwrap();
        let reply = o
            .release_source_search(&result, &s, &facts(&s), &qfacts(&q), &mut r)
            .unwrap();
        assert_eq!(reply.matches, expected);
        assert_eq!(reply.status, status);
        assert_eq!(o.source_read_context_usage().holds, 1);
    }
}

#[test]
fn managed_search_instrumentation_is_optional_bounded_redacted_and_does_not_select_outcomes() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _, _, s) = ready(b"secret");
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
        let reply = o
            .release_source_search(&result, &s, &facts(&s), &qfacts(&q), &mut r)
            .unwrap();
        assert_eq!(reply.matches, vec![ByteRange { start: 0, end: 6 }]);
        let mut lost = qfacts(&q);
        lost.grant = Some(false);
        assert!(
            o.release_source_search(&result, &s, &facts(&s), &lost, &mut r)
                .is_none()
        );
        assert!(o.retire_source_search_result(&result, &controls(&s), &mut r));
        assert_eq!(o.retire_source_search(&pool_control(), &mut r), Ok(true));
        assert_eq!(o.source_read_context_usage().holds, 0);
        if capacity == Some(128) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for boundary in [
                Boundary::SourceSearchConfiguration,
                Boundary::SourceSearchEvaluation,
                Boundary::SourceSearchRelease,
                Boundary::SourceSearchInvalidation,
                Boundary::SourceSearchRetirement,
                Boundary::SourceSearchPoolRetirement,
            ] {
                assert!(events.iter().any(|e| e.boundary == boundary));
            }
            assert!(events.iter().all(|e| e.service_time.is_none()));
            assert!(!format!("{events:?}").contains("secret"));
        } else if capacity == Some(1) {
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}
