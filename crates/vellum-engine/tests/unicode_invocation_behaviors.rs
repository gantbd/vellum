use vellum_engine::attachment::Affinity;
use vellum_engine::coordinates::BytePosition;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, Recorder};
use vellum_engine::selection::{Selection, SelectionBudget};
use vellum_engine::source::Limits;
#[path = "unicode_invocation_behaviors/arguments.rs"]
mod arguments;
#[path = "support/discovered_context_fixture.rs"]
mod discovered_context;
#[path = "support/ordinary_result_fixture.rs"]
mod ordinary_result;
#[path = "unicode_invocation_behaviors/selection_arguments.rs"]
mod selection_arguments;
#[path = "support/selection_discovery_fixture.rs"]
mod selection_route;
fn point(at: usize) -> Selection {
    Selection {
        anchor: BytePosition(at),
        head: BytePosition(at),
        affinity: Affinity::After,
    }
}
fn request() -> UnicodeCommandV1 {
    UnicodeCommandV1 {
        unit: UnicodeUnitV1::Grapheme,
        direction: UnicodeDirectionV1::Forward,
        count: 1,
        extend: false,
    }
}
fn table() -> BoundaryTableV1 {
    BoundaryTableV1 {
        profile: BoundaryProfileV1::Unicode18Uax29Revision49,
        graphemes: vec![0, 3, 4, 5, 6, 7]
            .into_iter()
            .map(BytePosition)
            .collect(),
        words: vec![0, 3, 4, 6, 7].into_iter().map(BytePosition).collect(),
        significant_words: vec![
            ByteRange { start: 0, end: 3 },
            ByteRange { start: 4, end: 6 },
        ],
    }
}
fn fixture(
    bytes: usize,
) -> (
    SourceCoordinator,
    LogicalViewContext,
    BoundaryEvidenceId,
    Recorder,
) {
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
        .configure_boundaries(
            BoundaryLimits {
                max_issued: 8,
                max_retained: 4,
                max_source_bytes: 2048,
                max_boundaries: 64,
                max_words: 8,
                max_evidence_bytes: 4096,
                max_retained_bytes: 8192,
            },
            &mut r,
        )
        .unwrap();
    owner
        .configure_unicode_commands(
            UnicodeCommandBudget {
                max_steps: 64,
                max_derived_bytes: 4096,
            },
            &mut r,
        )
        .unwrap();
    let source = owner.create("a\u{301} hi!".as_bytes(), &mut r).unwrap();
    let capture = owner.begin_boundary_capture(&source, &mut r).unwrap();
    let evidence = owner
        .accept_boundary_evidence(&capture, &table(), &mut r)
        .unwrap();
    let view = owner.create_logical_view(&mut r).unwrap();
    let stamp = owner.logical_view(&view, &mut r).unwrap().stamp;
    owner
        .attach_logical_view(&view, stamp, &source, &[point(0)], 0, &mut r)
        .unwrap();
    let context = owner
        .logical_view(&view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    (owner, context, evidence, r)
}
#[test]
fn motion_retries_do_not_reacquire_retired_evidence_or_reexecute_after_forgetting() {
    let (mut owner, context, evidence, mut r) = fixture(4096);
    owner
        .set_read_only(&context.source.document, true, &mut r)
        .unwrap();
    let id = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit_unicode_command(
            &id,
            &context,
            &evidence,
            UnicodeOperationV1::Movement,
            request(),
            &mut r,
        )
        .unwrap();
    assert_eq!(
        owner.submit_unicode_command(
            &id,
            &context,
            &evidence,
            UnicodeOperationV1::Movement,
            request(),
            &mut r
        ),
        Ok(InvocationState::Pending)
    );
    owner.run(&id, &mut r).unwrap();
    let InvocationOutcome::UnicodeMovementCommitted(update) =
        owner.invocation_outcome(&id, &mut r).unwrap()
    else {
        panic!("not movement")
    };
    assert!(update.changed);
    assert_eq!(
        owner
            .logical_view(&context.view, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[point(3)]
    );
    owner.retire_boundary_evidence(&evidence, &mut r).unwrap();
    owner.forget_invocation_outcome(&id, &mut r).unwrap();
    let generation = owner.generation();
    assert_eq!(
        owner.submit_unicode_command(
            &id,
            &context,
            &evidence,
            UnicodeOperationV1::Movement,
            request(),
            &mut r
        ),
        Ok(InvocationState::Finished)
    );
    owner.run(&id, &mut r).unwrap();
    assert_eq!(owner.generation(), generation);
    assert_eq!(owner.history_usage().retained_groups, 0);
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .version,
        context.source
    );
}
#[test]
fn deletion_replay_retains_one_source_history_commit_even_after_undo() {
    let (mut owner, context, evidence, mut r) = fixture(4096);
    let id = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit_unicode_command(
            &id,
            &context,
            &evidence,
            UnicodeOperationV1::Deletion,
            request(),
            &mut r,
        )
        .unwrap();
    owner.run(&id, &mut r).unwrap();
    let InvocationOutcome::Committed(outcome) = owner.invocation_outcome(&id, &mut r).unwrap()
    else {
        panic!("not source")
    };
    let group = outcome.history_group.as_ref().unwrap().clone();
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        " hi!"
    );
    let undo = owner
        .prepare_reversal(&group, HistoryDirection::Undo, &mut r)
        .unwrap();
    owner.commit(undo, &mut r).unwrap();
    let generation = owner.generation();
    owner
        .submit_unicode_command(
            &id,
            &context,
            &evidence,
            UnicodeOperationV1::Deletion,
            request(),
            &mut r,
        )
        .unwrap();
    owner.run(&id, &mut r).unwrap();
    assert_eq!(owner.generation(), generation);
    assert_eq!(owner.history_usage().retained_groups, 1);
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        "a\u{301} hi!"
    );
}
#[test]
fn equivalent_new_evidence_cannot_replace_original_and_retirement_refuses_pending_effects() {
    for operation in [UnicodeOperationV1::Movement, UnicodeOperationV1::Deletion] {
        let (mut owner, context, evidence, mut r) = fixture(4096);
        let id = owner.issue_invocation(&mut r).unwrap();
        owner
            .submit_unicode_command(&id, &context, &evidence, operation, request(), &mut r)
            .unwrap();
        assert_eq!(
            owner.retire_invocation(&id, &mut r),
            Err(InvocationError::Pending)
        );
        owner.retire_boundary_evidence(&evidence, &mut r).unwrap();
        let capture = owner
            .begin_boundary_capture(&context.source, &mut r)
            .unwrap();
        let new = owner
            .accept_boundary_evidence(&capture, &table(), &mut r)
            .unwrap();
        assert_eq!(
            owner.submit_unicode_command(&id, &context, &new, operation, request(), &mut r),
            Err(InvocationError::IntentMismatch)
        );
        assert_eq!(
            owner.submit_unicode_command(&id, &context, &evidence, operation, request(), &mut r),
            Ok(InvocationState::Pending)
        );
        let generation = owner.generation();
        owner.run(&id, &mut r).unwrap();
        let expected = if operation == UnicodeOperationV1::Movement {
            UnicodeCommandError::Boundary(BoundaryError::Unavailable)
        } else {
            UnicodeCommandError::Coordinator(CoordinatorError::Unavailable)
        };
        assert_eq!(
            owner.invocation_outcome(&id, &mut r),
            Ok(&InvocationOutcome::UnicodeRejected(expected))
        );
        assert_eq!(owner.generation(), generation);
        assert!(!owner.is_faulted());
    }
}
#[test]
fn operation_and_all_request_fields_remain_intent_and_descriptor_overflow_does_not_bind() {
    let (mut owner, context, evidence, mut r) = fixture(0);
    let id = owner.issue_invocation(&mut r).unwrap();
    assert_eq!(
        owner.submit_unicode_command(
            &id,
            &context,
            &evidence,
            UnicodeOperationV1::Movement,
            request(),
            &mut r
        ),
        Err(InvocationError::Limit)
    );
    assert_eq!(
        owner.invocation_state(&id, &mut r),
        Ok(InvocationState::Issued)
    );
    let (mut owner, context, evidence, mut r) = fixture(4096);
    let id = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit_unicode_command(
            &id,
            &context,
            &evidence,
            UnicodeOperationV1::Movement,
            request(),
            &mut r,
        )
        .unwrap();
    for changed in [
        UnicodeCommandV1 {
            unit: UnicodeUnitV1::WordStart,
            ..request()
        },
        UnicodeCommandV1 {
            direction: UnicodeDirectionV1::Backward,
            ..request()
        },
        UnicodeCommandV1 {
            count: 2,
            ..request()
        },
        UnicodeCommandV1 {
            extend: true,
            ..request()
        },
    ] {
        assert_eq!(
            owner.submit_unicode_command(
                &id,
                &context,
                &evidence,
                UnicodeOperationV1::Movement,
                changed,
                &mut r
            ),
            Err(InvocationError::IntentMismatch)
        );
    }
    assert_eq!(
        owner.submit_unicode_command(
            &id,
            &context,
            &evidence,
            UnicodeOperationV1::Deletion,
            request(),
            &mut r
        ),
        Err(InvocationError::IntentMismatch)
    );
    assert_eq!(
        owner.submit_literal_edit(&id, &context, "", SelectionAfterV1::Mapped, &mut r),
        Err(InvocationError::IntentMismatch)
    );
}
#[test]
fn invalid_original_request_stays_rejected_and_stop_uses_shared_protected_lifecycle() {
    let (mut owner, context, evidence, mut r) = fixture(4096);
    let id = owner.issue_invocation(&mut r).unwrap();
    let invalid = UnicodeCommandV1 {
        count: 0,
        ..request()
    };
    owner
        .submit_unicode_command(
            &id,
            &context,
            &evidence,
            UnicodeOperationV1::Movement,
            invalid,
            &mut r,
        )
        .unwrap();
    owner.run(&id, &mut r).unwrap();
    assert_eq!(
        owner.invocation_outcome(&id, &mut r),
        Ok(&InvocationOutcome::UnicodeRejected(
            UnicodeCommandError::InvalidRequest
        ))
    );
    assert_eq!(
        owner.submit_unicode_command(
            &id,
            &context,
            &evidence,
            UnicodeOperationV1::Movement,
            request(),
            &mut r
        ),
        Err(InvocationError::IntentMismatch)
    );
    let id = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit_unicode_command(
            &id,
            &context,
            &evidence,
            UnicodeOperationV1::Deletion,
            request(),
            &mut r,
        )
        .unwrap();
    owner.stop_invocation(&id, &mut r).unwrap();
    owner.run(&id, &mut r).unwrap();
    assert_eq!(
        owner.invocation_outcome(&id, &mut r),
        Ok(&InvocationOutcome::StoppedBeforePublication)
    );
    owner.retire_invocation(&id, &mut r).unwrap();
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        "a\u{301} hi!"
    );
}
#[test]
fn spent_directional_noops_cannot_target_later_selection_and_stale_pending_requests_refuse() {
    for operation in [UnicodeOperationV1::Movement, UnicodeOperationV1::Deletion] {
        let (mut owner, context, evidence, mut r) = fixture(4096);
        let id = owner.issue_invocation(&mut r).unwrap();
        let backward = UnicodeCommandV1 {
            direction: UnicodeDirectionV1::Backward,
            ..request()
        };
        owner
            .submit_unicode_command(&id, &context, &evidence, operation, backward, &mut r)
            .unwrap();
        let generation = owner.generation();
        owner.run(&id, &mut r).unwrap();
        assert_eq!(owner.generation(), generation);
        owner
            .set_logical_selections(&context, &[point(3)], 0, &mut r)
            .unwrap();
        let generation = owner.generation();
        owner
            .submit_unicode_command(&id, &context, &evidence, operation, backward, &mut r)
            .unwrap();
        owner.run(&id, &mut r).unwrap();
        assert_eq!(owner.generation(), generation);
        let id = owner.issue_invocation(&mut r).unwrap();
        owner
            .submit_unicode_command(&id, &context, &evidence, operation, request(), &mut r)
            .unwrap();
        owner.run(&id, &mut r).unwrap();
        assert!(matches!(
            owner.invocation_outcome(&id, &mut r).unwrap(),
            InvocationOutcome::UnicodeRejected(_)
        ));
    }
}
#[test]
fn optional_capture_preserves_both_operations_and_does_not_double_count_publication() {
    for operation in [UnicodeOperationV1::Movement, UnicodeOperationV1::Deletion] {
        for capacity in [None, Some(64), Some(1)] {
            let (mut owner, context, evidence, _) = fixture(4096);
            let mut r = capacity.map_or_else(Recorder::disabled, |n| {
                Recorder::try_with_capacity(n).unwrap()
            });
            let id = owner.issue_invocation(&mut r).unwrap();
            owner
                .submit_unicode_command(&id, &context, &evidence, operation, request(), &mut r)
                .unwrap();
            owner.run(&id, &mut r).unwrap();
            owner
                .submit_unicode_command(&id, &context, &evidence, operation, request(), &mut r)
                .unwrap();
            owner.run(&id, &mut r).unwrap();
            assert_eq!(
                owner
                    .logical_view(&context.view, &mut r)
                    .unwrap()
                    .selections
                    .selections(),
                &[point(if operation == UnicodeOperationV1::Movement {
                    3
                } else {
                    0
                })]
            );
            let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            if capacity == Some(64) {
                assert_eq!(
                    records
                        .iter()
                        .filter(|o| o.boundary == Boundary::Publication)
                        .count(),
                    1
                );
                assert_eq!(
                    records
                        .iter()
                        .filter(|o| o.boundary == Boundary::UnicodePreparation)
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
}
