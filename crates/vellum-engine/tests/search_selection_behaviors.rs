use vellum_engine::attachment::Affinity;
#[path = "support/discovered_context_fixture.rs"]
mod discovered_context;
#[path = "support/ordinary_result_fixture.rs"]
mod ordinary_result;
#[path = "support/selection_discovery_fixture.rs"]
mod selection_route;
use vellum_engine::coordinates::BytePosition;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, Recorder};
use vellum_engine::selection::{Selection, SelectionBudget, SelectionError};
use vellum_engine::source::Limits;
#[path = "search_selection_behaviors/arguments.rs"]
mod arguments;
fn s(a: usize, h: usize, affinity: Affinity) -> Selection {
    Selection {
        anchor: BytePosition(a),
        head: BytePosition(h),
        affinity,
    }
}
fn fixture(
    matches: usize,
    selections: &[Selection],
) -> (
    SourceCoordinator,
    LogicalViewContext,
    SearchSession,
    SearchResult,
    Recorder,
) {
    let mut owner = SourceCoordinator::configured(CoordinatorConfig {
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
        annotations: None,
        search: Some(SearchLimits {
            max_sessions_issued: 8,
            max_live_sessions: 4,
            max_live_results: 8,
            max_root_bytes: 4096,
            max_scope_ranges: 16,
            max_query_bytes: 1024,
            max_comparisons: 65536,
            max_matches: matches,
            max_match_bytes: 4096,
            max_replacement_bytes: 8192,
        }),
    });
    let mut r = Recorder::disabled();
    owner
        .configure_logical_views(
            LogicalViewLimits {
                max_live_views: 8,
                max_issued_views: 8,
                max_total_selections: 32,
                normalization: SelectionBudget {
                    max_input_selections: 16,
                    max_output_selections: 16,
                    max_work: 65536,
                },
                max_mapping_steps: 65536,
            },
            &mut r,
        )
        .unwrap();
    let source = owner.create(b"aa aa", &mut r).unwrap();
    let view = owner.create_logical_view(&mut r).unwrap();
    let stamp = owner.logical_view(&view, &mut r).unwrap().stamp;
    owner
        .attach_logical_view(&view, stamp, &source, selections, 0, &mut r)
        .unwrap();
    let context = owner
        .logical_view(&view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    let session = owner.open_search(&source.document, &mut r).unwrap();
    let result = owner.search(&session, &source, &query(), &mut r).unwrap();
    (owner, context, session, result, r)
}
fn query() -> SearchRequest {
    SearchRequest {
        query: "aa".into(),
        scope: SearchScope::Document,
    }
}
fn command(occurrence: SearchMatch, mode: SearchSelectionModeV1) -> SelectionCommandV1 {
    SelectionCommandV1::SearchMatch {
        occurrence,
        direction: SelectionDirectionV1::Backward,
        affinity: Affinity::Before,
        mode,
    }
}
#[test]
fn explicit_modes_preserve_direction_affinity_primary_and_read_only_source_without_history() {
    for mode in [
        SearchSelectionModeV1::ReplaceAll,
        SearchSelectionModeV1::ReplacePrimary,
        SearchSelectionModeV1::AddPrimary,
    ] {
        let (mut owner, context, _, result, mut r) =
            fixture(128, &[s(0, 1, Affinity::After), s(4, 5, Affinity::After)]);
        owner
            .set_read_only(&context.source.document, true, &mut r)
            .unwrap();
        let id = owner.issue_invocation(&mut r).unwrap();
        owner
            .submit_selection_command(
                &id,
                &context,
                &command(result.match_id(1).unwrap(), mode),
                &mut r,
            )
            .unwrap();
        owner.run(&id, &mut r).unwrap();
        assert!(matches!(
            owner.invocation_outcome(&id, &mut r).unwrap(),
            InvocationOutcome::SelectionCommitted(_)
        ));
        let view = owner.logical_view(&context.view, &mut r).unwrap();
        let expected = if mode == SearchSelectionModeV1::AddPrimary {
            vec![s(0, 1, Affinity::After), s(5, 3, Affinity::Before)]
        } else {
            vec![s(5, 3, Affinity::Before)]
        };
        assert_eq!(view.selections.selections(), expected);
        assert_eq!(
            view.selections.primary(),
            usize::from(mode == SearchSelectionModeV1::AddPrimary)
        );
        assert_eq!(
            owner
                .read(&context.source.document, &mut r)
                .unwrap()
                .version,
            context.source
        );
        assert_eq!(owner.history_usage().retained_groups, 0);
    }
}
#[test]
fn normalized_painted_range_never_changes_original_replacement_extent_or_search_scope() {
    let (mut owner, context, _, result, mut r) = fixture(128, &[s(0, 4, Affinity::After)]);
    let occurrence = result.match_id(1).unwrap();
    let id = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit_selection_command(
            &id,
            &context,
            &command(occurrence.clone(), SearchSelectionModeV1::AddPrimary),
            &mut r,
        )
        .unwrap();
    owner.run(&id, &mut r).unwrap();
    assert_eq!(
        owner
            .logical_view(&context.view, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[s(5, 0, Affinity::Before)]
    );
    assert_eq!(
        result.ranges(),
        &[
            ByteRange { start: 0, end: 2 },
            ByteRange { start: 3, end: 5 }
        ]
    );
    let plan = owner
        .prepare_search_replace(&SearchSelection::Current(occurrence), "X", &mut r)
        .unwrap();
    owner.commit(plan, &mut r).unwrap();
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        "aa X"
    );
}
#[test]
fn known_partial_occurrence_can_select_and_replace_but_cannot_navigate_or_replace_all() {
    let (mut owner, context, _, result, mut r) = fixture(1, &[s(4, 4, Affinity::After)]);
    assert!(matches!(result.status(), SearchStatus::Exhausted(_)));
    assert_eq!(result.exact_total(), None);
    let occurrence = result.match_id(0).unwrap();
    let id = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit_selection_command(
            &id,
            &context,
            &command(occurrence.clone(), SearchSelectionModeV1::ReplaceAll),
            &mut r,
        )
        .unwrap();
    owner.run(&id, &mut r).unwrap();
    assert_eq!(
        owner
            .logical_view(&context.view, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[s(2, 0, Affinity::Before)]
    );
    assert_eq!(
        owner
            .navigate_search(
                &result,
                SearchNavigation {
                    direction: SearchDirection::Next,
                    start: 0,
                    current: None,
                    wrap: true
                },
                &mut r
            )
            .err(),
        Some(SearchError::Incomplete)
    );
    assert_eq!(
        owner
            .prepare_search_replace(&SearchSelection::All(result.clone()), "X", &mut r)
            .err(),
        Some(CoordinatorError::Search(SearchError::Incomplete))
    );
    let plan = owner
        .prepare_search_replace(&SearchSelection::Current(occurrence), "X", &mut r)
        .unwrap();
    owner.commit(plan, &mut r).unwrap();
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        "X aa"
    );
}
#[test]
fn supersession_view_change_and_closed_search_refuse_original_pending_selection_without_requery() {
    for mode in 0..3 {
        let (mut owner, context, session, result, mut r) =
            fixture(128, &[s(4, 4, Affinity::After)]);
        let request = command(
            result.match_id(0).unwrap(),
            SearchSelectionModeV1::ReplaceAll,
        );
        let id = owner.issue_invocation(&mut r).unwrap();
        owner
            .submit_selection_command(&id, &context, &request, &mut r)
            .unwrap();
        match mode {
            0 => {
                owner
                    .search(&session, &context.source, &query(), &mut r)
                    .unwrap();
            }
            1 => {
                owner
                    .set_logical_selections(&context, &[s(3, 3, Affinity::After)], 0, &mut r)
                    .unwrap();
            }
            _ => {
                owner.close_search(&session, &mut r).unwrap();
            }
        }
        let generation = owner.generation();
        owner.run(&id, &mut r).unwrap();
        assert_eq!(
            owner.invocation_outcome(&id, &mut r),
            Ok(&InvocationOutcome::SelectionRejected(if mode == 2 {
                SelectionError::Unavailable
            } else {
                SelectionError::Stale
            }))
        );
        assert_eq!(owner.generation(), generation);
        assert!(!owner.is_faulted());
        assert_eq!(
            owner.submit_selection_command(&id, &context, &request, &mut r),
            Ok(InvocationState::Finished)
        );
    }
}
#[test]
fn match_lookup_identity_is_stable_but_new_query_result_and_changed_modes_are_not_retries() {
    let (mut owner, context, session, result, mut r) = fixture(128, &[s(4, 4, Affinity::After)]);
    assert_eq!(result.match_id(0), result.match_id(0));
    let request = command(
        result.match_id(0).unwrap(),
        SearchSelectionModeV1::ReplaceAll,
    );
    let id = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit_selection_command(&id, &context, &request, &mut r)
        .unwrap();
    let changed = command(
        result.match_id(0).unwrap(),
        SearchSelectionModeV1::AddPrimary,
    );
    assert_eq!(
        owner.submit_selection_command(&id, &context, &changed, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    let fresh = owner
        .search(&session, &context.source, &query(), &mut r)
        .unwrap();
    assert_eq!(
        owner.submit_selection_command(
            &id,
            &context,
            &command(
                fresh.match_id(0).unwrap(),
                SearchSelectionModeV1::ReplaceAll
            ),
            &mut r
        ),
        Err(InvocationError::IntentMismatch)
    );
}
#[test]
fn optional_capture_observes_resolution_without_replay_evaluation_or_payload() {
    for capacity in [None, Some(64), Some(1)] {
        let (mut owner, context, _, result, _) = fixture(128, &[s(4, 4, Affinity::After)]);
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let request = command(
            result.match_id(0).unwrap(),
            SearchSelectionModeV1::ReplaceAll,
        );
        assert!(!format!("{request:?}").contains("aa"));
        let id = owner.issue_invocation(&mut r).unwrap();
        owner
            .submit_selection_command(&id, &context, &request, &mut r)
            .unwrap();
        owner
            .submit_selection_command(&id, &context, &request, &mut r)
            .unwrap();
        owner.run(&id, &mut r).unwrap();
        owner.run(&id, &mut r).unwrap();
        assert_eq!(
            owner
                .logical_view(&context.view, &mut r)
                .unwrap()
                .selections
                .selections(),
            &[s(2, 0, Affinity::Before)]
        );
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(64) {
            assert_eq!(
                records
                    .iter()
                    .filter(|o| o.boundary == Boundary::SearchOccurrenceResolution)
                    .count(),
                2
            );
            assert!(
                !records
                    .iter()
                    .any(|o| o.boundary == Boundary::SearchEvaluation)
            );
        }
    }
}
