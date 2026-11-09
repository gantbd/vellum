//! Line settings and native preparations are retained in the common ledger.
use vellum_engine::attachment::Affinity;
use vellum_engine::coordinates::BytePosition;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, Recorder};
use vellum_engine::selection::{Selection, SelectionBudget};
use vellum_engine::source::Limits;
#[path = "line_invocation_behaviors/arguments.rs"]
mod arguments;
#[path = "line_invocation_behaviors/configured.rs"]
mod configured;
fn point(at: usize) -> Selection {
    Selection {
        anchor: BytePosition(at),
        head: BytePosition(at),
        affinity: Affinity::After,
    }
}
fn fixture(bytes: usize) -> (SourceCoordinator, LogicalViewContext, Recorder) {
    let mut owner = SourceCoordinator::with_invocation_and_history_limits(
        CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 8192,
            source: Limits {
                max_source_bytes: 2048,
                max_edits: 32,
                max_edit_payload_bytes: 4096,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 16,
            max_retained: 8,
            max_intent_bytes: bytes,
        },
        HistoryLimits {
            max_retained_groups: 16,
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
    owner
        .configure_line_edits(
            LineEditBudget {
                max_lines: 16,
                max_prefix_bytes: 1024,
                max_indent_columns: 128,
                max_unit_bytes: 8,
                max_derived_bytes: 4096,
                max_source_scan_bytes: 1024,
            },
            &mut r,
        )
        .unwrap();
    let source = owner.create(b" \talpha\r\n", &mut r).unwrap();
    let view = owner.create_logical_view(&mut r).unwrap();
    let stamp = owner.logical_view(&view, &mut r).unwrap().stamp;
    owner
        .attach_logical_view(&view, stamp, &source, &[point(1)], 0, &mut r)
        .unwrap();
    let context = owner
        .logical_view(&view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    (owner, context, r)
}
fn outdent() -> LineEditV1 {
    LineEditV1::Outdent {
        unit: "  ".into(),
        tab_width: 4,
    }
}
#[test]
fn an_empty_eof_outdent_is_spent_and_cannot_retarget_a_later_selection() {
    let (mut owner, context, mut r) = fixture(4096);
    owner
        .set_logical_selections(&context, &[point(9)], 0, &mut r)
        .unwrap();
    let context = owner
        .logical_view(&context.view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    let id = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit_line_edit(&id, &context, &outdent(), &mut r)
        .unwrap();
    let generation = owner.generation();
    owner.run(&id, &mut r).unwrap();
    let InvocationOutcome::Committed(outcome) = owner.invocation_outcome(&id, &mut r).unwrap()
    else {
        panic!("not committed")
    };
    assert!(outcome.operation.is_none());
    assert!(outcome.history_group.is_none());
    assert_eq!(owner.generation(), generation);
    owner
        .set_logical_selections(&context, &[point(1)], 0, &mut r)
        .unwrap();
    let generation = owner.generation();
    owner
        .submit_line_edit(&id, &context, &outdent(), &mut r)
        .unwrap();
    owner.run(&id, &mut r).unwrap();
    assert_eq!(owner.generation(), generation);
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        " \talpha\r\n"
    );
}
#[test]
fn original_line_preparation_executes_once_and_retries_after_undo_and_forgetting_do_not_rederive() {
    let (mut owner, context, mut r) = fixture(4096);
    let id = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit_line_edit(&id, &context, &outdent(), &mut r)
        .unwrap();
    assert_eq!(
        owner.submit_line_edit(&id, &context, &outdent(), &mut r),
        Ok(InvocationState::Pending)
    );
    owner.run(&id, &mut r).unwrap();
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        "  alpha\r\n"
    );
    let InvocationOutcome::Committed(outcome) = owner.invocation_outcome(&id, &mut r).unwrap()
    else {
        panic!("not committed")
    };
    let group = outcome.history_group.as_ref().unwrap().clone();
    let undo = owner
        .prepare_reversal(&group, HistoryDirection::Undo, &mut r)
        .unwrap();
    owner.commit(undo, &mut r).unwrap();
    let generation = owner.generation();
    owner.forget_invocation_outcome(&id, &mut r).unwrap();
    assert_eq!(
        owner.submit_line_edit(&id, &context, &outdent(), &mut r),
        Ok(InvocationState::Finished)
    );
    owner.run(&id, &mut r).unwrap();
    assert_eq!(owner.generation(), generation);
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        " \talpha\r\n"
    );
    assert_eq!(
        owner.invocation_outcome(&id, &mut r),
        Err(InvocationError::OutcomeUnavailable)
    );
}
#[test]
fn every_original_setting_remains_intent_even_if_the_derived_batch_is_equal() {
    let (mut owner, context, mut r) = fixture(4096);
    let id = owner.issue_invocation(&mut r).unwrap();
    let command = LineEditV1::Newline {
        delimiter: GeneratedNewlineV1::Lf,
        indent: NewlineIndentV1::None,
    };
    owner
        .submit_line_edit(&id, &context, &command, &mut r)
        .unwrap();
    for changed in [
        LineEditV1::Newline {
            delimiter: GeneratedNewlineV1::Cr,
            indent: NewlineIndentV1::None,
        },
        LineEditV1::Newline {
            delimiter: GeneratedNewlineV1::Lf,
            indent: NewlineIndentV1::CopyPrefix,
        },
        outdent(),
    ] {
        assert_eq!(
            owner.submit_line_edit(&id, &context, &changed, &mut r),
            Err(InvocationError::IntentMismatch)
        );
    }
    let id = owner.issue_invocation(&mut r).unwrap();
    let indent = LineEditV1::Indent {
        unit: " ".into(),
        tab_width: 4,
    };
    owner
        .submit_line_edit(&id, &context, &indent, &mut r)
        .unwrap();
    assert_eq!(
        owner.submit_line_edit(
            &id,
            &context,
            &LineEditV1::Indent {
                unit: " ".into(),
                tab_width: 8
            },
            &mut r
        ),
        Err(InvocationError::IntentMismatch)
    );
    owner.stop_invocation(&id, &mut r).unwrap();
    owner.run(&id, &mut r).unwrap();
    assert_eq!(
        owner.invocation_outcome(&id, &mut r),
        Ok(&InvocationOutcome::StoppedBeforePublication)
    );
    assert_eq!(
        owner.submit_line_edit(&id, &context, &indent, &mut r),
        Ok(InvocationState::Finished)
    );
}
#[test]
fn changed_views_peers_and_write_policy_refuse_the_original_line_plan_wholly() {
    for mode in 0..3 {
        let (mut owner, context, mut r) = fixture(4096);
        let id = owner.issue_invocation(&mut r).unwrap();
        owner
            .submit_line_edit(&id, &context, &outdent(), &mut r)
            .unwrap();
        match mode {
            0 => {
                owner
                    .set_logical_selections(&context, &[point(3)], 0, &mut r)
                    .unwrap();
            }
            1 => {
                let peer = owner.create_logical_view(&mut r).unwrap();
                let stamp = owner.logical_view(&peer, &mut r).unwrap().stamp;
                owner
                    .attach_logical_view(&peer, stamp, &context.source, &[point(0)], 0, &mut r)
                    .unwrap();
            }
            _ => {
                owner
                    .set_read_only(&context.source.document, true, &mut r)
                    .unwrap();
            }
        }
        let generation = owner.generation();
        owner.run(&id, &mut r).unwrap();
        assert!(matches!(
            owner.invocation_outcome(&id, &mut r).unwrap(),
            InvocationOutcome::LineRejected(LineEditError::Coordinator(_))
        ));
        assert_eq!(owner.generation(), generation);
        assert_eq!(owner.history_usage().retained_groups, 0);
        assert_eq!(
            owner
                .read(&context.source.document, &mut r)
                .unwrap()
                .source
                .text(),
            " \talpha\r\n"
        );
    }
}
#[test]
fn original_invalid_settings_are_retained_and_descriptor_refusal_does_not_bind_a_slot() {
    let (mut owner, context, mut r) = fixture(0);
    let id = owner.issue_invocation(&mut r).unwrap();
    assert_eq!(
        owner.submit_line_edit(&id, &context, &outdent(), &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(owner.run(&id, &mut r), Err(InvocationError::NotSubmitted));
    let (mut owner, context, mut r) = fixture(4096);
    let id = owner.issue_invocation(&mut r).unwrap();
    let invalid = LineEditV1::Outdent {
        unit: "x".into(),
        tab_width: 4,
    };
    owner
        .submit_line_edit(&id, &context, &invalid, &mut r)
        .unwrap();
    owner.run(&id, &mut r).unwrap();
    assert_eq!(
        owner.invocation_outcome(&id, &mut r),
        Ok(&InvocationOutcome::LineRejected(
            LineEditError::InvalidSettings
        ))
    );
    assert_eq!(
        owner.submit_line_edit(&id, &context, &invalid, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(
        owner.submit_line_edit(&id, &context, &outdent(), &mut r),
        Err(InvocationError::IntentMismatch)
    );
}
#[test]
fn newline_replay_keeps_caret_intent_and_optional_diagnostics_do_not_change_results() {
    for capacity in [None, Some(64), Some(1)] {
        let (mut owner, context, _) = fixture(4096);
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let id = owner.issue_invocation(&mut r).unwrap();
        let request = LineEditV1::Newline {
            delimiter: GeneratedNewlineV1::CrLf,
            indent: NewlineIndentV1::CopyPrefix,
        };
        owner
            .submit_line_edit(&id, &context, &request, &mut r)
            .unwrap();
        owner.run(&id, &mut r).unwrap();
        let generation = owner.generation();
        assert_eq!(
            owner
                .logical_view(&context.view, &mut r)
                .unwrap()
                .selections
                .selections(),
            &[point(4)]
        );
        owner
            .submit_line_edit(&id, &context, &request, &mut r)
            .unwrap();
        owner.run(&id, &mut r).unwrap();
        assert_eq!(owner.generation(), generation);
        assert_eq!(owner.history_usage().retained_groups, 1);
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(64) {
            assert_eq!(
                records
                    .iter()
                    .filter(|o| o.boundary == Boundary::LineEditPreparation)
                    .count(),
                1
            );
            assert_eq!(
                records
                    .iter()
                    .filter(|o| o.boundary == Boundary::Publication)
                    .count(),
                1
            );
            assert!(
                records
                    .iter()
                    .any(|o| o.boundary == Boundary::InvocationReplay)
            );
        }
    }
}
