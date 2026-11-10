use vellum_engine::attachment::Affinity;
use vellum_engine::coordinates::{BytePosition, CoordinateBudget};
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, Recorder};
use vellum_engine::selection::{Selection, SelectionBudget, SelectionError};
use vellum_engine::source::Limits;
#[path = "selection_invocation_behaviors/arguments.rs"]
mod arguments;
#[path = "selection_invocation_behaviors/composition.rs"]
mod composition;
#[path = "support/discovered_context_fixture.rs"]
mod discovered_context;
#[path = "selection_invocation_behaviors/exact_batch.rs"]
mod exact_batch;
#[path = "support/ordinary_result_fixture.rs"]
mod ordinary_result;
#[path = "support/selection_discovery_fixture.rs"]
mod selection_route;
fn caret(at: usize) -> Selection {
    Selection {
        anchor: BytePosition(at),
        head: BytePosition(at),
        affinity: Affinity::After,
    }
}
fn fixture() -> (SourceCoordinator, LogicalViewContext, Recorder) {
    fixture_with_intent_limit(4096)
}
fn fixture_with_intent_limit(
    max_intent_bytes: usize,
) -> (SourceCoordinator, LogicalViewContext, Recorder) {
    let mut owner = SourceCoordinator::with_invocation_and_history_limits(
        CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 8,
                max_edit_payload_bytes: 4096,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 16,
            max_retained: 8,
            max_intent_bytes,
        },
        HistoryLimits {
            max_retained_groups: 8,
            max_journal_bytes: 65536,
            max_pins: 8,
        },
    );
    let mut r = Recorder::disabled();
    owner
        .configure_logical_views(
            LogicalViewLimits {
                max_live_views: 8,
                max_issued_views: 8,
                max_total_selections: 16,
                normalization: SelectionBudget {
                    max_input_selections: 8,
                    max_output_selections: 8,
                    max_work: 4096,
                },
                max_mapping_steps: 4096,
            },
            &mut r,
        )
        .unwrap();
    let source = owner.create(b"abc\r\ndef", &mut r).unwrap();
    let view = owner.create_logical_view(&mut r).unwrap();
    let stamp = owner.logical_view(&view, &mut r).unwrap().stamp;
    owner
        .attach_logical_view(&view, stamp, &source, &[caret(1)], 0, &mut r)
        .unwrap();
    let context = owner
        .logical_view(&view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    (owner, context, r)
}
fn endpoint(endpoint: LogicalEndpointV1, work: usize) -> SelectionCommandV1 {
    SelectionCommandV1::Endpoint {
        endpoint,
        extend: false,
        budget: CoordinateBudget {
            max_scan_bytes: work,
        },
    }
}
#[test]
fn endpoint_is_prepared_once_works_read_only_and_has_a_typed_retained_outcome() {
    let (mut owner, context, mut r) = fixture();
    owner
        .set_read_only(&context.source.document, true, &mut r)
        .unwrap();
    let id = owner.issue_invocation(&mut r).unwrap();
    let command = endpoint(LogicalEndpointV1::LineEnd, 1024);
    owner
        .submit_selection_command(&id, &context, &command, &mut r)
        .unwrap();
    assert_eq!(
        owner.submit_selection_command(&id, &context, &command, &mut r),
        Ok(InvocationState::Pending)
    );
    assert_eq!(
        owner.submit_selection_command(
            &id,
            &context,
            &endpoint(LogicalEndpointV1::LineEnd, 1023),
            &mut r
        ),
        Err(InvocationError::IntentMismatch)
    );
    owner.run(&id, &mut r).unwrap();
    let InvocationOutcome::SelectionCommitted(update) =
        owner.invocation_outcome(&id, &mut r).unwrap()
    else {
        panic!("not selection outcome")
    };
    assert!(update.changed);
    let stamp = update.stamp;
    assert_eq!(
        owner
            .logical_view(&context.view, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[caret(3)]
    );
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .version,
        context.source
    );
    assert_eq!(owner.history_usage().retained_groups, 0);
    owner.forget_invocation_outcome(&id, &mut r).unwrap();
    owner.run(&id, &mut r).unwrap();
    assert_eq!(
        owner.submit_selection_command(&id, &context, &command, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(
        owner.logical_view(&context.view, &mut r).unwrap().stamp,
        stamp
    );
}
#[test]
fn set_preserves_original_primary_and_raw_intent_even_when_normalized_results_are_equal() {
    let (mut owner, context, mut r) = fixture();
    let id = owner.issue_invocation(&mut r).unwrap();
    let command = SelectionCommandV1::Set {
        selections: vec![caret(5), caret(2)],
        primary: 0,
    };
    owner
        .submit_selection_command(&id, &context, &command, &mut r)
        .unwrap();
    let reordered = SelectionCommandV1::Set {
        selections: vec![caret(2), caret(5)],
        primary: 1,
    };
    assert_eq!(
        owner.submit_selection_command(&id, &context, &reordered, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    owner.run(&id, &mut r).unwrap();
    let view = owner.logical_view(&context.view, &mut r).unwrap();
    assert_eq!(view.selections.selections(), &[caret(2), caret(5)]);
    assert_eq!(view.selections.primary(), 1);
}
#[test]
fn changed_selection_refuses_but_unrelated_source_publication_is_preserved() {
    let (mut owner, context, mut r) = fixture();
    let id = owner.issue_invocation(&mut r).unwrap();
    let command = endpoint(LogicalEndpointV1::DocumentEnd, 0);
    owner
        .submit_selection_command(&id, &context, &command, &mut r)
        .unwrap();
    owner
        .set_logical_selections(&context, &[caret(2)], 0, &mut r)
        .unwrap();
    let other = owner.create(b"unrelated", &mut r).unwrap();
    let generation = owner.generation();
    owner.run(&id, &mut r).unwrap();
    assert_eq!(
        owner.invocation_outcome(&id, &mut r),
        Ok(&InvocationOutcome::SelectionRejected(SelectionError::Stale))
    );
    assert_eq!(owner.generation(), generation);
    assert_eq!(
        owner.read(&other.document, &mut r).unwrap().source.text(),
        "unrelated"
    );
    assert_eq!(
        owner
            .logical_view(&context.view, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[caret(2)]
    );
}

#[test]
fn source_changes_attachment_aba_and_view_retirement_cannot_redirect_a_pending_command() {
    for mode in 0..3 {
        let (mut owner, context, mut r) = fixture();
        let id = owner.issue_invocation(&mut r).unwrap();
        let command = endpoint(LogicalEndpointV1::DocumentEnd, 0);
        owner
            .submit_selection_command(&id, &context, &command, &mut r)
            .unwrap();
        match mode {
            0 => {
                let plan = owner
                    .prepare(
                        &[DocumentEdits {
                            expected: context.source.clone(),
                            edits: vec![vellum_engine::source::Replacement {
                                start: 0,
                                end: 0,
                                expected: String::new(),
                                inserted: "X".into(),
                            }],
                        }],
                        &[],
                        &mut r,
                    )
                    .unwrap();
                owner.commit(plan, &mut r).unwrap();
            }
            1 => {
                let update = owner
                    .detach_logical_view(&context.view, context.stamp, &mut r)
                    .unwrap();
                owner
                    .attach_logical_view(
                        &context.view,
                        update.stamp,
                        &context.source,
                        &[caret(1)],
                        0,
                        &mut r,
                    )
                    .unwrap();
            }
            _ => owner
                .retire_logical_view(&context.view, context.stamp, &mut r)
                .unwrap(),
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
        assert_eq!(
            owner.submit_selection_command(&id, &context, &command, &mut r),
            Ok(InvocationState::Finished)
        );
    }
}
#[test]
fn invalid_preparation_is_retained_and_stop_refuses_to_retire_pending_work() {
    let (mut owner, context, mut r) = fixture();
    let id = owner.issue_invocation(&mut r).unwrap();
    let command = SelectionCommandV1::Set {
        selections: vec![],
        primary: 0,
    };
    owner
        .submit_selection_command(&id, &context, &command, &mut r)
        .unwrap();
    assert_eq!(
        owner.retire_invocation(&id, &mut r),
        Err(InvocationError::Pending)
    );
    owner.run(&id, &mut r).unwrap();
    assert_eq!(
        owner.invocation_outcome(&id, &mut r),
        Ok(&InvocationOutcome::SelectionRejected(
            SelectionError::InvalidSelection
        ))
    );
    let stopped = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit_selection_command(
            &stopped,
            &context,
            &endpoint(LogicalEndpointV1::DocumentEnd, 0),
            &mut r,
        )
        .unwrap();
    owner.stop_invocation(&stopped, &mut r).unwrap();
    owner.run(&stopped, &mut r).unwrap();
    assert_eq!(
        owner.invocation_outcome(&stopped, &mut r),
        Ok(&InvocationOutcome::StoppedBeforePublication)
    );
    owner.retire_invocation(&stopped, &mut r).unwrap();
    assert_eq!(
        owner
            .logical_view(&context.view, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[caret(1)]
    );
}
#[test]
fn noops_are_spent_and_optional_capture_cannot_change_selection_execution() {
    for capacity in [None, Some(64), Some(1)] {
        let (mut owner, context, _) = fixture();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let id = owner.issue_invocation(&mut r).unwrap();
        let command = SelectionCommandV1::Set {
            selections: vec![caret(1)],
            primary: 0,
        };
        let generation = owner.generation();
        owner
            .submit_selection_command(&id, &context, &command, &mut r)
            .unwrap();
        owner.run(&id, &mut r).unwrap();
        let InvocationOutcome::SelectionCommitted(update) =
            owner.invocation_outcome(&id, &mut r).unwrap()
        else {
            panic!("not selection")
        };
        assert!(!update.changed);
        assert_eq!(owner.generation(), generation);
        owner
            .set_logical_selections(&context, &[caret(3)], 0, &mut r)
            .unwrap();
        let generation = owner.generation();
        owner
            .submit_selection_command(&id, &context, &command, &mut r)
            .unwrap();
        owner.run(&id, &mut r).unwrap();
        assert_eq!(owner.generation(), generation);
        assert_eq!(
            owner
                .logical_view(&context.view, &mut r)
                .unwrap()
                .selections
                .selections(),
            &[caret(3)]
        );
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(64) {
            assert!(
                records
                    .iter()
                    .any(|o| o.boundary == Boundary::InvocationReplay)
            );
            assert!(
                records
                    .iter()
                    .any(|o| o.boundary == Boundary::LogicalSelectionPreparation)
            );
            assert!(!records.iter().any(|o| o.boundary == Boundary::Publication));
        }
    }
}
