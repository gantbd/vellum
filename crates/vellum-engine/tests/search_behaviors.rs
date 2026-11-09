use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::source::{Limits, Replacement};
#[test]
fn document_read_pins_preserve_whole_history_groups_and_live_result_holders() {
    let mut o = owner(limits());
    let mut r = Recorder::disabled();
    let a = o.create(b"A", &mut r).unwrap();
    let b = o.create(b"B", &mut r).unwrap();
    let c = o.create(b"C", &mut r).unwrap();
    let plan = o
        .prepare(
            &[
                DocumentEdits {
                    expected: a.clone(),
                    edits: vec![Replacement {
                        start: 0,
                        end: 1,
                        expected: "A".into(),
                        inserted: "a".into(),
                    }],
                },
                DocumentEdits {
                    expected: b.clone(),
                    edits: vec![Replacement {
                        start: 0,
                        end: 1,
                        expected: "B".into(),
                        inserted: "b".into(),
                    }],
                },
            ],
            &[],
            &mut r,
        )
        .unwrap();
    let group = o.commit(plan, &mut r).unwrap().history_group.unwrap();
    let pin = o.pin_history(&group, &mut r).unwrap();
    for doc in [&a.document, &b.document] {
        let pins = o.document_read_pins_v1(doc, &mut r).unwrap();
        assert_eq!((pins.history_groups, pins.history_pins), (1, 1));
    }
    assert_eq!(
        o.document_read_pins_v1(&c.document, &mut r)
            .unwrap()
            .total(),
        Ok(0)
    );
    let current = o.read(&a.document, &mut r).unwrap().version;
    let session = o.open_search(&a.document, &mut r).unwrap();
    let result = o.search(&session, &current, &request("a"), &mut r).unwrap();
    assert_eq!(
        o.document_read_pins_v1(&a.document, &mut r)
            .unwrap()
            .legacy_search_results,
        1
    );
    o.close_search(&session, &mut r).unwrap();
    let pins = o.document_read_pins_v1(&a.document, &mut r).unwrap();
    assert_eq!(
        (pins.legacy_search_sessions, pins.legacy_search_results),
        (0, 1)
    );
    let undo = o
        .prepare_reversal(&group, HistoryDirection::Undo, &mut r)
        .unwrap();
    o.commit(undo, &mut r).unwrap();
    let current = o.read(&a.document, &mut r).unwrap().version;
    let branch = o
        .prepare(
            &[DocumentEdits {
                expected: current,
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: "branch".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    let branch_group = o.commit(branch, &mut r).unwrap().history_group.unwrap();
    assert!(o.history_usage().invalidated_groups > 0);
    let invalidated = o.document_read_pins_v1(&b.document, &mut r).unwrap();
    assert_eq!(
        (invalidated.history_groups, invalidated.history_pins),
        (1, 1)
    );
    assert!(o.retire_history_group(&group, &mut r).is_err());
    o.release_history_pin(&pin, &mut r).unwrap();
    o.retire_history_group(&group, &mut r).unwrap();
    o.retire_history_group(&branch_group, &mut r).unwrap();
    assert_eq!(
        o.document_read_pins_v1(&b.document, &mut r)
            .unwrap()
            .history_groups,
        0
    );
    drop(result);
    assert_eq!(
        o.document_read_pins_v1(&a.document, &mut r)
            .unwrap()
            .total(),
        Ok(0)
    );
}

fn limits() -> SearchLimits {
    SearchLimits {
        max_sessions_issued: 8,
        max_live_sessions: 4,
        max_live_results: 8,
        max_root_bytes: 4096,
        max_scope_ranges: 16,
        max_query_bytes: 1024,
        max_comparisons: 65536,
        max_matches: 128,
        max_match_bytes: 4096,
        max_replacement_bytes: 8192,
    }
}
fn owner(search: SearchLimits) -> SourceCoordinator {
    SourceCoordinator::configured(CoordinatorConfig {
        limits: CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 8192,
            source: Limits {
                max_source_bytes: 4096,
                max_edits: 128,
                max_edit_payload_bytes: 8192,
            },
        },
        invocation: Some(InvocationLimits {
            max_lifetime_issued: 16,
            max_retained: 16,
            max_intent_bytes: 16384,
        }),
        history: Some(HistoryLimits {
            max_retained_groups: 16,
            max_journal_bytes: 65536,
            max_pins: 8,
        }),
        resources: None,
        search: Some(search),
        annotations: None,
    })
}
fn request(query: &str) -> SearchRequest {
    SearchRequest {
        query: query.into(),
        scope: SearchScope::Document,
    }
}
fn range(start: usize, end: usize) -> ByteRange {
    ByteRange { start, end }
}

#[test]
fn given_unicode_prose_code_and_mixed_endings_then_exact_matches_preserve_original_bytes() {
    let mut owner = owner(limits());
    let mut recorder = Recorder::disabled();
    let input = "Ä\r\n```mermaid\nÄaÄ\r```\n";
    let version = owner.create(input.as_bytes(), &mut recorder).unwrap();
    let session = owner.open_search(&version.document, &mut recorder).unwrap();
    let result = owner
        .search(&session, &version, &request("Ä"), &mut recorder)
        .unwrap();
    let expected: Vec<_> = input
        .match_indices('Ä')
        .map(|(i, text)| range(i, i + text.len()))
        .collect();
    assert_eq!(result.ranges(), expected);
    assert_eq!(result.exact_total(), Some(3));
    assert_eq!(result.status(), SearchStatus::Complete);
    assert_eq!(
        owner
            .read(&version.document, &mut recorder)
            .unwrap()
            .source
            .text(),
        input
    );
}

#[test]
fn given_overlapping_and_adjacent_scopes_then_only_overlap_is_merged_and_disjoint_text_is_not_joined()
 {
    let mut owner = owner(limits());
    let mut recorder = Recorder::disabled();
    let version = owner.create(b"ababa", &mut recorder).unwrap();
    let session = owner.open_search(&version.document, &mut recorder).unwrap();
    for (scopes, expected) in [
        (vec![range(0, 3), range(2, 5)], vec![range(0, 3)]),
        (vec![range(0, 2), range(2, 5)], vec![range(2, 5)]),
        (vec![range(0, 1), range(2, 3)], vec![]),
    ] {
        let request = SearchRequest {
            query: "aba".into(),
            scope: SearchScope::Captured(scopes),
        };
        let result = owner
            .search(&session, &version, &request, &mut recorder)
            .unwrap();
        assert_eq!(result.ranges(), expected);
    }
}

#[test]
fn given_literal_special_characters_empty_query_and_combining_text_then_no_implicit_regex_or_grapheme_widening_occurs()
 {
    let mut owner = owner(limits());
    let mut recorder = Recorder::disabled();
    let version = owner
        .create("a\u{301}.*\r\na.*".as_bytes(), &mut recorder)
        .unwrap();
    let session = owner.open_search(&version.document, &mut recorder).unwrap();
    for (query, expected) in [
        ("", vec![]),
        (".*", vec![range(3, 5), range(8, 10)]),
        ("\u{301}", vec![range(1, 3)]),
        ("\n", vec![range(6, 7)]),
        ("A", vec![]),
    ] {
        let result = owner
            .search(&session, &version, &request(query), &mut recorder)
            .unwrap();
        assert_eq!(result.ranges(), expected, "query {query:?}");
    }
}

#[test]
fn given_navigation_then_start_is_inclusive_current_is_excluded_and_wrap_is_explicit() {
    let mut owner = owner(limits());
    let mut recorder = Recorder::disabled();
    let version = owner.create(b"aaa", &mut recorder).unwrap();
    let session = owner.open_search(&version.document, &mut recorder).unwrap();
    let result = owner
        .search(&session, &version, &request("aa"), &mut recorder)
        .unwrap();
    assert_eq!(result.ranges(), [range(0, 2)]);
    for direction in [SearchDirection::Next, SearchDirection::Previous] {
        let found = owner
            .navigate_search(
                &result,
                SearchNavigation {
                    direction,
                    start: 0,
                    current: None,
                    wrap: false,
                },
                &mut recorder,
            )
            .unwrap()
            .unwrap();
        assert_eq!(found.range, range(0, 2));
        assert!(!found.wrapped);
        assert!(
            owner
                .navigate_search(
                    &result,
                    SearchNavigation {
                        direction,
                        start: 0,
                        current: Some(&found.occurrence),
                        wrap: false
                    },
                    &mut recorder
                )
                .unwrap()
                .is_none()
        );
        let wrapped = owner
            .navigate_search(
                &result,
                SearchNavigation {
                    direction,
                    start: 0,
                    current: Some(&found.occurrence),
                    wrap: true,
                },
                &mut recorder,
            )
            .unwrap()
            .unwrap();
        assert!(wrapped.wrapped);
        assert_eq!(wrapped.range, range(0, 2));
    }
}

#[test]
fn given_a_new_query_then_old_matches_and_already_prepared_replacements_are_superseded() {
    let mut owner = owner(limits());
    let mut recorder = Recorder::disabled();
    let version = owner.create(b"a a", &mut recorder).unwrap();
    let session = owner.open_search(&version.document, &mut recorder).unwrap();
    let result = owner
        .search(&session, &version, &request("a"), &mut recorder)
        .unwrap();
    let prepared = owner
        .prepare_search_replace(&SearchSelection::All(result.clone()), "b", &mut recorder)
        .unwrap();
    let newer = owner
        .search(&session, &version, &request(" "), &mut recorder)
        .unwrap();
    assert!(newer.generation() > result.generation());
    assert_eq!(
        owner.commit(prepared, &mut recorder),
        Err(CoordinatorError::Search(SearchError::Superseded))
    );
    assert_eq!(
        owner
            .navigate_search(
                &result,
                SearchNavigation {
                    direction: SearchDirection::Next,
                    start: 0,
                    current: None,
                    wrap: false
                },
                &mut recorder
            )
            .unwrap_err(),
        SearchError::Superseded
    );
    assert_eq!(
        owner
            .read(&version.document, &mut recorder)
            .unwrap()
            .version,
        version
    );
}

#[test]
fn given_exhausted_search_then_subset_has_no_exact_total_navigation_or_partial_replace_all() {
    for bound in [
        SearchBudget::Comparisons,
        SearchBudget::Matches,
        SearchBudget::ReplyBytes,
    ] {
        let mut configured = limits();
        match bound {
            SearchBudget::Comparisons => configured.max_comparisons = 1,
            SearchBudget::Matches => configured.max_matches = 1,
            SearchBudget::ReplyBytes => {
                configured.max_match_bytes = std::mem::size_of::<ByteRange>()
            }
        }
        let mut owner = owner(configured);
        let mut recorder = Recorder::disabled();
        let version = owner.create(b"a a a", &mut recorder).unwrap();
        let session = owner.open_search(&version.document, &mut recorder).unwrap();
        let result = owner
            .search(&session, &version, &request("a"), &mut recorder)
            .unwrap();
        assert_eq!(result.status(), SearchStatus::Exhausted(bound));
        assert_eq!(result.exact_total(), None);
        assert!(
            owner
                .prepare_search_replace(&SearchSelection::All(result), "b", &mut recorder)
                .is_err()
        );
        assert_eq!(
            owner
                .read(&version.document, &mut recorder)
                .unwrap()
                .version,
            version
        );
    }
}

#[test]
fn given_replace_all_then_one_transaction_and_one_linked_history_group_round_trip_literal_text() {
    let mut owner = owner(limits());
    let mut recorder = Recorder::disabled();
    let version = owner.create(b"a a a", &mut recorder).unwrap();
    let session = owner.open_search(&version.document, &mut recorder).unwrap();
    let result = owner
        .search(&session, &version, &request("a"), &mut recorder)
        .unwrap();
    let prepared = owner
        .prepare_search_replace(&SearchSelection::All(result), "$1\\x", &mut recorder)
        .unwrap();
    let committed = owner.commit(prepared, &mut recorder).unwrap();
    assert_eq!(committed.changes.len(), 1);
    assert_eq!(
        owner
            .read(&version.document, &mut recorder)
            .unwrap()
            .source
            .text(),
        "$1\\x $1\\x $1\\x"
    );
    let undo = owner
        .prepare_reversal(
            &committed.history_group.unwrap(),
            HistoryDirection::Undo,
            &mut recorder,
        )
        .unwrap();
    owner.commit(undo, &mut recorder).unwrap();
    assert_eq!(
        owner
            .read(&version.document, &mut recorder)
            .unwrap()
            .source
            .text(),
        "a a a"
    );
}

#[test]
fn given_replace_current_then_only_the_owned_occurrence_is_replaced_and_foreign_current_is_refused()
{
    let mut owner = owner(limits());
    let mut recorder = Recorder::disabled();
    let version = owner.create(b"a a", &mut recorder).unwrap();
    let a = owner.open_search(&version.document, &mut recorder).unwrap();
    let b = owner.open_search(&version.document, &mut recorder).unwrap();
    let first = owner
        .search(&a, &version, &request("a"), &mut recorder)
        .unwrap();
    let second = owner
        .search(&b, &version, &request("a"), &mut recorder)
        .unwrap();
    let current = first.match_id(1).unwrap();
    assert_eq!(
        owner
            .navigate_search(
                &second,
                SearchNavigation {
                    direction: SearchDirection::Next,
                    start: 0,
                    current: Some(&current),
                    wrap: false
                },
                &mut recorder
            )
            .unwrap_err(),
        SearchError::MatchUnavailable
    );
    let prepared = owner
        .prepare_search_replace(&SearchSelection::Current(current), "b", &mut recorder)
        .unwrap();
    owner.commit(prepared, &mut recorder).unwrap();
    assert_eq!(
        owner
            .read(&version.document, &mut recorder)
            .unwrap()
            .source
            .text(),
        "a b"
    );
}

#[test]
fn given_read_only_or_stale_source_then_replacement_does_not_skip_matches_or_rebase() {
    let mut owner = owner(limits());
    let mut recorder = Recorder::disabled();
    let version = owner.create(b"a a", &mut recorder).unwrap();
    let session = owner.open_search(&version.document, &mut recorder).unwrap();
    let result = owner
        .search(&session, &version, &request("a"), &mut recorder)
        .unwrap();
    let prepared = owner
        .prepare_search_replace(&SearchSelection::All(result.clone()), "b", &mut recorder)
        .unwrap();
    owner
        .set_read_only(&version.document, true, &mut recorder)
        .unwrap();
    assert!(owner.commit(prepared, &mut recorder).is_err());
    owner
        .set_read_only(&version.document, false, &mut recorder)
        .unwrap();
    let change = owner
        .prepare(
            &[DocumentEdits {
                expected: version.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "a".into(),
                    inserted: "c".into(),
                }],
            }],
            &[],
            &mut recorder,
        )
        .unwrap();
    owner.commit(change, &mut recorder).unwrap();
    assert!(
        owner
            .prepare_search_replace(&SearchSelection::All(result), "d", &mut recorder)
            .is_err()
    );
    assert_eq!(
        owner
            .read(&version.document, &mut recorder)
            .unwrap()
            .source
            .text(),
        "c a"
    );
}

#[test]
fn given_replayed_replacement_then_original_intent_is_retained_even_with_no_matches() {
    let mut owner = owner(limits());
    let mut recorder = Recorder::disabled();
    let version = owner.create(b"a", &mut recorder).unwrap();
    let session = owner.open_search(&version.document, &mut recorder).unwrap();
    let result = owner
        .search(&session, &version, &request("missing"), &mut recorder)
        .unwrap();
    let selection = SearchSelection::All(result);
    let invocation = owner.issue_invocation(&mut recorder).unwrap();
    owner
        .submit_search_replace(&invocation, &selection, "unused original", &mut recorder)
        .unwrap();
    owner.run(&invocation, &mut recorder).unwrap();
    assert_eq!(
        owner.submit_search_replace(&invocation, &selection, "changed unused", &mut recorder),
        Err(InvocationError::IntentMismatch)
    );
    owner
        .search(&session, &version, &request("a"), &mut recorder)
        .unwrap();
    assert_eq!(
        owner
            .submit_search_replace(&invocation, &selection, "unused original", &mut recorder)
            .unwrap(),
        InvocationState::Finished
    );
    let InvocationOutcome::Committed(outcome) = owner
        .invocation_outcome(&invocation, &mut recorder)
        .unwrap()
    else {
        panic!("not committed")
    };
    assert!(outcome.changes.is_empty());
    assert_eq!(outcome.operation, None);
}

#[test]
fn given_retained_results_and_match_handles_then_storage_is_bounded_until_actual_release() {
    let mut configured = limits();
    configured.max_live_results = 1;
    let mut owner = owner(configured);
    let mut recorder = Recorder::disabled();
    let version = owner.create(b"a", &mut recorder).unwrap();
    let session = owner.open_search(&version.document, &mut recorder).unwrap();
    let result = owner
        .search(&session, &version, &request("a"), &mut recorder)
        .unwrap();
    let occurrence = result.match_id(0).unwrap();
    drop(result);
    assert_eq!(owner.search_usage().live_results, 1);
    assert_eq!(
        owner
            .search(&session, &version, &request("b"), &mut recorder)
            .unwrap_err(),
        SearchError::Limit
    );
    drop(occurrence);
    assert!(
        owner
            .search(&session, &version, &request("b"), &mut recorder)
            .is_ok()
    );
    owner.close_search(&session, &mut recorder).unwrap();
    assert_eq!(
        owner
            .search(&session, &version, &request("a"), &mut recorder)
            .unwrap_err(),
        SearchError::Unavailable
    );
}

#[test]
fn given_bad_scopes_and_capture_modes_then_no_source_mutation_or_sensitive_diagnostics_occurs() {
    for capacity in [None, Some(64), Some(1)] {
        let mut recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let mut owner = owner(limits());
        let version = owner.create("PRIVATE_Ä".as_bytes(), &mut recorder).unwrap();
        let session = owner.open_search(&version.document, &mut recorder).unwrap();
        let bad = SearchRequest {
            query: "PRIVATE_".into(),
            scope: SearchScope::Captured(vec![range(0, 9)]),
        };
        assert_eq!(
            owner
                .search(&session, &version, &bad, &mut recorder)
                .unwrap_err(),
            SearchError::InvalidBoundary
        );
        let good = request("PRIVATE_");
        let result = owner
            .search(&session, &version, &good, &mut recorder)
            .unwrap();
        assert_eq!(result.exact_total(), Some(1));
        assert!(!format!("{good:?} {result:?}").contains("PRIVATE_"));
        assert_eq!(
            owner
                .read(&version.document, &mut recorder)
                .unwrap()
                .version,
            version
        );
        let invocation = owner.issue_invocation(&mut recorder).unwrap();
        owner
            .submit_search_replace(
                &invocation,
                &SearchSelection::All(result),
                "PUBLIC_",
                &mut recorder,
            )
            .unwrap();
        owner.run(&invocation, &mut recorder).unwrap();
        let InvocationOutcome::Committed(outcome) = owner
            .invocation_outcome(&invocation, &mut recorder)
            .unwrap()
        else {
            panic!("not committed")
        };
        assert_eq!(outcome.changes.len(), 1);
        assert!(outcome.history_group.is_some());
        assert_eq!(
            owner
                .read(&version.document, &mut recorder)
                .unwrap()
                .source
                .text(),
            "PUBLIC_Ä"
        );
        while let Some(record) = recorder.pop_front() {
            assert!(!format!("{record:?}").contains("PRIVATE_"));
        }
        if capacity == Some(1) {
            assert!(recorder.dropped() > 0);
        }
    }
}

#[test]
fn given_small_binary_sources_and_queries_then_an_independent_standard_library_oracle_agrees() {
    fn words(maximum: usize) -> Vec<String> {
        let mut words = vec![String::new()];
        for length in 1..=maximum {
            for bits in 0..1usize << length {
                words.push(
                    (0..length)
                        .map(|index| if bits & (1 << index) == 0 { 'a' } else { 'b' })
                        .collect(),
                );
            }
        }
        words
    }
    let queries = words(3);
    for text in words(5) {
        let mut owner = owner(limits());
        let mut recorder = Recorder::disabled();
        let version = owner.create(text.as_bytes(), &mut recorder).unwrap();
        let session = owner.open_search(&version.document, &mut recorder).unwrap();
        for query in &queries {
            let result = owner
                .search(&session, &version, &request(query), &mut recorder)
                .unwrap();
            let expected: Vec<_> = if query.is_empty() {
                vec![]
            } else {
                text.match_indices(query)
                    .map(|(start, matched)| range(start, start + matched.len()))
                    .collect()
            };
            assert_eq!(result.ranges(), expected, "text {text:?}, query {query:?}");
            assert_eq!(result.exact_total(), Some(expected.len()));
        }
    }
}

#[test]
fn given_query_supersession_after_invocation_admission_then_run_retains_rejection_without_reexecution()
 {
    let mut owner = owner(limits());
    let mut recorder = Recorder::disabled();
    let version = owner.create(b"a a", &mut recorder).unwrap();
    let session = owner.open_search(&version.document, &mut recorder).unwrap();
    let result = owner
        .search(&session, &version, &request("a"), &mut recorder)
        .unwrap();
    let selected = SearchSelection::All(result);
    let invocation = owner.issue_invocation(&mut recorder).unwrap();
    owner
        .submit_search_replace(&invocation, &selected, "b", &mut recorder)
        .unwrap();
    owner
        .search(&session, &version, &request(" "), &mut recorder)
        .unwrap();
    owner.run(&invocation, &mut recorder).unwrap();
    assert_eq!(
        owner
            .invocation_outcome(&invocation, &mut recorder)
            .unwrap(),
        &InvocationOutcome::Rejected(CoordinatorError::Search(SearchError::Superseded))
    );
    assert_eq!(
        owner
            .submit_search_replace(&invocation, &selected, "b", &mut recorder)
            .unwrap(),
        InvocationState::Finished
    );
    assert_eq!(
        owner
            .read(&version.document, &mut recorder)
            .unwrap()
            .version,
        version
    );
}

#[test]
fn given_equal_or_deleting_adjacent_replacements_then_noop_and_journal_inverse_use_ordinary_rules()
{
    for replacement in ["a", ""] {
        let mut owner = owner(limits());
        let mut recorder = Recorder::disabled();
        let version = owner.create(b"aaa", &mut recorder).unwrap();
        let session = owner.open_search(&version.document, &mut recorder).unwrap();
        let result = owner
            .search(&session, &version, &request("a"), &mut recorder)
            .unwrap();
        let prepared = owner
            .prepare_search_replace(&SearchSelection::All(result), replacement, &mut recorder)
            .unwrap();
        let committed = owner.commit(prepared, &mut recorder).unwrap();
        if replacement == "a" {
            assert_eq!(committed.operation, None);
            assert_eq!(committed.history_group, None);
            assert_eq!(
                owner
                    .read(&version.document, &mut recorder)
                    .unwrap()
                    .version,
                version
            );
        } else {
            assert_eq!(
                owner
                    .read(&version.document, &mut recorder)
                    .unwrap()
                    .source
                    .text(),
                ""
            );
            let undo = owner
                .prepare_reversal(
                    &committed.history_group.unwrap(),
                    HistoryDirection::Undo,
                    &mut recorder,
                )
                .unwrap();
            owner.commit(undo, &mut recorder).unwrap();
            assert_eq!(
                owner
                    .read(&version.document, &mut recorder)
                    .unwrap()
                    .source
                    .text(),
                "aaa"
            );
        }
    }
}

#[test]
fn given_foreign_closed_or_disposed_sessions_then_occurrences_cannot_attach_to_another_incarnation()
{
    let mut a = owner(limits());
    let mut b = owner(limits());
    let mut recorder = Recorder::disabled();
    let version = a.create(b"a", &mut recorder).unwrap();
    let session = a.open_search(&version.document, &mut recorder).unwrap();
    let result = a
        .search(&session, &version, &request("a"), &mut recorder)
        .unwrap();
    assert_eq!(
        b.search(&session, &version, &request("a"), &mut recorder)
            .unwrap_err(),
        SearchError::Unavailable
    );
    assert_eq!(
        b.prepare_search_replace(&SearchSelection::All(result.clone()), "b", &mut recorder)
            .unwrap_err(),
        CoordinatorError::Search(SearchError::Unavailable)
    );
    a.dispose(&version.document, &mut recorder).unwrap();
    let reopened = a.create(b"a", &mut recorder).unwrap();
    assert_eq!(
        a.prepare_search_replace(&SearchSelection::All(result), "b", &mut recorder)
            .unwrap_err(),
        CoordinatorError::Search(SearchError::Unavailable)
    );
    assert_eq!(
        a.read(&reopened.document, &mut recorder)
            .unwrap()
            .source
            .text(),
        "a"
    );
    a.close_search(&session, &mut recorder).unwrap();
}

#[test]
fn given_complete_and_exhausted_search_then_diagnostics_and_bound_gauges_distinguish_them_without_query_labels()
 {
    use vellum_engine::instrumentation::{Boundary, Disposition};
    let mut configured = limits();
    configured.max_comparisons = 1;
    let mut owner = owner(configured);
    let mut recorder = Recorder::try_with_capacity(64).unwrap();
    let version = owner.create(b"aaa", &mut recorder).unwrap();
    let session = owner.open_search(&version.document, &mut recorder).unwrap();
    let empty = owner
        .search(&session, &version, &request(""), &mut recorder)
        .unwrap();
    let partial = owner
        .search(&session, &version, &request("a"), &mut recorder)
        .unwrap();
    assert_eq!(empty.exact_total(), Some(0));
    assert_eq!(partial.exact_total(), None);
    assert_eq!(owner.search_usage().live_results, 2);
    let mut observations = Vec::new();
    while let Some(record) = recorder.pop_front() {
        if record.boundary == Boundary::SearchEvaluation {
            observations.push(record.disposition);
        }
    }
    assert_eq!(observations, [Disposition::Completed, Disposition::Partial]);
}

#[test]
fn given_pending_or_retained_invocations_then_search_result_capacity_counts_their_actual_origin_hold()
 {
    let mut configured = limits();
    configured.max_live_results = 1;
    let mut owner = owner(configured);
    let mut recorder = Recorder::disabled();
    let version = owner.create(b"a", &mut recorder).unwrap();
    let session = owner.open_search(&version.document, &mut recorder).unwrap();
    let result = owner
        .search(&session, &version, &request("missing"), &mut recorder)
        .unwrap();
    let selected = SearchSelection::All(result);
    let invocation = owner.issue_invocation(&mut recorder).unwrap();
    owner
        .submit_search_replace(&invocation, &selected, "unused", &mut recorder)
        .unwrap();
    drop(selected);
    owner.run(&invocation, &mut recorder).unwrap();
    owner
        .forget_invocation_outcome(&invocation, &mut recorder)
        .unwrap();
    assert_eq!(
        owner
            .search(&session, &version, &request("a"), &mut recorder)
            .unwrap_err(),
        SearchError::Limit
    );
    owner.retire_invocation(&invocation, &mut recorder).unwrap();
    assert!(
        owner
            .search(&session, &version, &request("a"), &mut recorder)
            .is_ok()
    );
}

#[test]
fn given_replacement_capacity_failure_then_no_prefix_or_history_group_is_published() {
    let mut configured = limits();
    configured.max_replacement_bytes = 3;
    let mut owner = owner(configured);
    let mut recorder = Recorder::disabled();
    let version = owner.create(b"a a", &mut recorder).unwrap();
    let session = owner.open_search(&version.document, &mut recorder).unwrap();
    let result = owner
        .search(&session, &version, &request("a"), &mut recorder)
        .unwrap();
    let before = owner.history_usage();
    assert_eq!(
        owner
            .prepare_search_replace(&SearchSelection::All(result.clone()), "b", &mut recorder)
            .unwrap_err(),
        CoordinatorError::Search(SearchError::Limit)
    );
    assert_eq!(owner.history_usage(), before);
    assert_eq!(
        owner
            .read(&version.document, &mut recorder)
            .unwrap()
            .version,
        version
    );
    // One occurrence fits; refusing all is not an implicit selection of this prefix.
    let selected = SearchSelection::Current(result.match_id(0).unwrap());
    let prepared = owner
        .prepare_search_replace(&selected, "b", &mut recorder)
        .unwrap();
    owner.commit(prepared, &mut recorder).unwrap();
    assert_eq!(
        owner
            .read(&version.document, &mut recorder)
            .unwrap()
            .source
            .text(),
        "b a"
    );
}

#[test]
fn given_closed_search_sessions_then_live_capacity_is_released_but_lifetime_ids_never_recycle() {
    let mut configured = limits();
    configured.max_live_sessions = 1;
    configured.max_sessions_issued = 2;
    let mut owner = owner(configured);
    let mut recorder = Recorder::disabled();
    let version = owner.create(b"a", &mut recorder).unwrap();
    let first = owner.open_search(&version.document, &mut recorder).unwrap();
    assert_eq!(
        owner
            .open_search(&version.document, &mut recorder)
            .unwrap_err(),
        SearchError::Limit
    );
    owner.close_search(&first, &mut recorder).unwrap();
    let second = owner.open_search(&version.document, &mut recorder).unwrap();
    assert_eq!(
        owner
            .search(&first, &version, &request("a"), &mut recorder)
            .unwrap_err(),
        SearchError::Unavailable
    );
    owner.close_search(&second, &mut recorder).unwrap();
    assert_eq!(
        owner
            .open_search(&version.document, &mut recorder)
            .unwrap_err(),
        SearchError::Limit
    );
    assert_eq!(owner.search_usage().issued_lifetime, 2);
    assert_eq!(owner.search_usage().live_sessions, 0);
}
