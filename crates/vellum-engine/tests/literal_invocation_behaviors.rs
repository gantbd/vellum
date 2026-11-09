//! Literal editing uses the shared invocation ledger, never a new execution mode.
#[path = "literal_invocation_behaviors/arguments.rs"]
mod arguments;
use vellum_engine::attachment::Affinity;
use vellum_engine::coordinates::BytePosition;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, Recorder};
use vellum_engine::selection::{Selection, SelectionBudget};
use vellum_engine::source::Limits;

#[test]
fn owned_literal_uses_the_same_intent_replay_denial_and_publisher_as_borrowed_input() {
    for allowed in [Some(true), None] {
        let (mut owner, c, mut r) = fixture(4096);
        let id = owner.issue_invocation(&mut r).unwrap();
        let text = "Grüße\r\n:quit\u{1b}";
        let mut reserved = String::new();
        reserved.try_reserve_exact(256).unwrap();
        reserved.push_str(text);
        assert_eq!(
            owner
                .submit_owned_literal_edit_with_permission_v1(
                    &id,
                    &c,
                    reserved,
                    SelectionAfterV1::CaretsAfter,
                    allowed,
                    &mut r
                )
                .unwrap(),
            InvocationState::Pending
        );
        assert_eq!(
            owner
                .submit_literal_edit(&id, &c, text, SelectionAfterV1::CaretsAfter, &mut r)
                .unwrap(),
            InvocationState::Pending
        );
        assert_eq!(
            owner.submit_owned_literal_edit_with_permission_v1(
                &id,
                &c,
                "different".into(),
                SelectionAfterV1::CaretsAfter,
                allowed,
                &mut r
            ),
            Err(InvocationError::IntentMismatch)
        );
        owner.run(&id, &mut r).unwrap();
        let read = owner.read(&c.source.document, &mut r).unwrap();
        assert_eq!(
            read.source.text(),
            if allowed == Some(true) { text } else { "abc" }
        );
        let version = read.version;
        assert_eq!(
            owner
                .submit_owned_literal_edit_with_permission_v1(
                    &id,
                    &c,
                    text.into(),
                    SelectionAfterV1::CaretsAfter,
                    Some(true),
                    &mut r
                )
                .unwrap(),
            InvocationState::Finished
        );
        owner.run(&id, &mut r).unwrap();
        assert_eq!(
            owner.read(&c.source.document, &mut r).unwrap().version,
            version
        );
        assert_eq!(owner.invocation_usage().issued_lifetime, 1);
    }
}

#[test]
fn permission_denial_is_retained_once_and_cannot_refresh_into_a_later_edit() {
    for allowed in [None, Some(false)] {
        let (mut owner, context, mut r) = fixture(4096);
        assert!(owner.can_retain_literal_intent_v1(0));
        assert!(!owner.can_retain_literal_intent_v1(usize::MAX));
        let id = owner.issue_invocation(&mut r).unwrap();
        assert_eq!(
            owner
                .submit_literal_edit_with_permission_v1(
                    &id,
                    &context,
                    "Z",
                    SelectionAfterV1::CaretsAfter,
                    allowed,
                    &mut r
                )
                .unwrap(),
            InvocationState::Pending
        );
        // Matching replay while still pending cannot replace the refusal with
        // a newer positive premise before the original runner observes it.
        assert_eq!(
            owner
                .submit_literal_edit(&id, &context, "Z", SelectionAfterV1::CaretsAfter, &mut r)
                .unwrap(),
            InvocationState::Pending
        );
        assert_eq!(owner.run(&id, &mut r).unwrap(), InvocationState::Finished);
        assert!(matches!(
            owner.invocation_outcome(&id, &mut r).unwrap(),
            InvocationOutcome::Rejected(CoordinatorError::Unavailable)
        ));
        assert_eq!(
            owner
                .submit_literal_edit(&id, &context, "Z", SelectionAfterV1::CaretsAfter, &mut r)
                .unwrap(),
            InvocationState::Finished
        );
        owner.run(&id, &mut r).unwrap();
        assert_eq!(
            owner
                .read(&context.source.document, &mut r)
                .unwrap()
                .version,
            context.source
        );
        assert_eq!(
            owner
                .read(&context.source.document, &mut r)
                .unwrap()
                .source
                .text(),
            "abc"
        );
        assert_eq!(owner.history_usage().retained_groups, 0);
    }
    let (owner, _, _) = fixture(0);
    assert!(!owner.can_retain_literal_intent_v1(0));
}

fn fixture(intent_bytes: usize) -> (SourceCoordinator, LogicalViewContext, Recorder) {
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
            max_lifetime_issued: 8,
            max_retained: 8,
            max_intent_bytes: intent_bytes,
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
    let source = owner.create(b"abc", &mut r).unwrap();
    let view = owner.create_logical_view(&mut r).unwrap();
    let stamp = owner.logical_view(&view, &mut r).unwrap().stamp;
    owner
        .attach_logical_view(&view, stamp, &source, &[selection(0, 3)], 0, &mut r)
        .unwrap();
    let context = owner
        .logical_view(&view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    (owner, context, r)
}
fn selection(anchor: usize, head: usize) -> Selection {
    Selection {
        anchor: BytePosition(anchor),
        head: BytePosition(head),
        affinity: Affinity::After,
    }
}

#[test]
fn a_terminal_noop_cannot_become_a_later_edit_and_foreign_identities_are_not_admitted() {
    let (mut owner, context, mut r) = fixture(4096);
    let id = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit_literal_edit(&id, &context, "abc", SelectionAfterV1::Mapped, &mut r)
        .unwrap();
    owner.run(&id, &mut r).unwrap();
    let InvocationOutcome::Committed(outcome) = owner.invocation_outcome(&id, &mut r).unwrap()
    else {
        panic!("not committed")
    };
    assert!(outcome.operation.is_none());
    owner
        .set_logical_selections(&context, &[selection(1, 1)], 0, &mut r)
        .unwrap();
    let generation = owner.generation();
    assert_eq!(
        owner.submit_literal_edit(&id, &context, "abc", SelectionAfterV1::Mapped, &mut r),
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
        "abc"
    );
    let (mut other, _, _) = fixture(4096);
    let foreign = other.issue_invocation(&mut r).unwrap();
    assert_eq!(
        owner.submit_literal_edit(&foreign, &context, "X", SelectionAfterV1::Mapped, &mut r),
        Err(InvocationError::Unavailable)
    );
}

#[test]
fn matching_retries_keep_original_literal_and_caret_policy_after_commit_and_forgetting() {
    let (mut owner, context, mut r) = fixture(4096);
    let id = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit_literal_edit(
            &id,
            &context,
            "PRIVATE",
            SelectionAfterV1::CaretsAfter,
            &mut r,
        )
        .unwrap();
    assert_eq!(
        owner.submit_literal_edit(
            &id,
            &context,
            "PRIVATE",
            SelectionAfterV1::CaretsAfter,
            &mut r
        ),
        Ok(InvocationState::Pending)
    );
    assert_eq!(
        owner.submit_literal_edit(&id, &context, "PRIVATE", SelectionAfterV1::Mapped, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    assert_eq!(
        owner.submit_literal_edit(
            &id,
            &context,
            "OTHER",
            SelectionAfterV1::CaretsAfter,
            &mut r
        ),
        Err(InvocationError::IntentMismatch)
    );
    owner.run(&id, &mut r).unwrap();
    let generation = owner.generation();
    let InvocationOutcome::Committed(outcome) = owner.invocation_outcome(&id, &mut r).unwrap()
    else {
        panic!("not committed")
    };
    assert!(outcome.operation.is_some());
    assert!(outcome.history_group.is_some());
    assert_eq!(
        owner
            .logical_view(&context.view, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[selection(7, 7)]
    );
    assert_eq!(
        owner.submit_literal_edit(
            &id,
            &context,
            "PRIVATE",
            SelectionAfterV1::CaretsAfter,
            &mut r
        ),
        Ok(InvocationState::Finished)
    );
    owner.forget_invocation_outcome(&id, &mut r).unwrap();
    owner.run(&id, &mut r).unwrap();
    assert_eq!(
        owner.submit_literal_edit(
            &id,
            &context,
            "PRIVATE",
            SelectionAfterV1::CaretsAfter,
            &mut r
        ),
        Ok(InvocationState::Finished)
    );
    assert_eq!(owner.generation(), generation);
    assert_eq!(owner.history_usage().retained_groups, 1);
    assert_eq!(
        owner.invocation_outcome(&id, &mut r),
        Err(InvocationError::OutcomeUnavailable)
    );
}

#[test]
fn changed_selection_after_admission_refuses_original_plan_without_reacquisition() {
    let (mut owner, context, mut r) = fixture(4096);
    let id = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit_literal_edit(&id, &context, "X", SelectionAfterV1::CaretsAfter, &mut r)
        .unwrap();
    owner
        .set_logical_selections(&context, &[selection(1, 1)], 0, &mut r)
        .unwrap();
    let generation = owner.generation();
    owner.run(&id, &mut r).unwrap();
    assert_eq!(
        owner.invocation_outcome(&id, &mut r),
        Ok(&InvocationOutcome::Rejected(CoordinatorError::Stale))
    );
    assert_eq!(owner.generation(), generation);
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        "abc"
    );
    let new_context = owner
        .logical_view(&context.view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    assert_eq!(
        owner.submit_literal_edit(
            &id,
            &new_context,
            "X",
            SelectionAfterV1::CaretsAfter,
            &mut r
        ),
        Err(InvocationError::IntentMismatch)
    );
    assert_eq!(
        owner.submit_literal_edit(&id, &context, "X", SelectionAfterV1::CaretsAfter, &mut r),
        Ok(InvocationState::Finished)
    );
}

#[test]
fn captured_refusal_is_not_retried_after_policy_changes_and_stop_never_publishes() {
    let (mut owner, context, mut r) = fixture(4096);
    owner
        .set_read_only(&context.source.document, true, &mut r)
        .unwrap();
    let id = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit_literal_edit(&id, &context, "X", SelectionAfterV1::Mapped, &mut r)
        .unwrap();
    owner
        .set_read_only(&context.source.document, false, &mut r)
        .unwrap();
    owner.run(&id, &mut r).unwrap();
    assert_eq!(
        owner.invocation_outcome(&id, &mut r),
        Ok(&InvocationOutcome::Rejected(CoordinatorError::Unavailable))
    );
    let stopped = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit_literal_edit(
            &stopped,
            &context,
            "X",
            SelectionAfterV1::CaretsAfter,
            &mut r,
        )
        .unwrap();
    owner.stop_invocation(&stopped, &mut r).unwrap();
    owner.run(&stopped, &mut r).unwrap();
    assert_eq!(
        owner.invocation_outcome(&stopped, &mut r),
        Ok(&InvocationOutcome::StoppedBeforePublication)
    );
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        "abc"
    );
    assert_eq!(owner.history_usage().retained_groups, 0);
}

#[test]
fn descriptor_limit_leaves_identity_unbound_and_kinds_cannot_alias() {
    let (mut owner, context, mut r) = fixture(0);
    let id = owner.issue_invocation(&mut r).unwrap();
    assert_eq!(
        owner.submit_literal_edit(&id, &context, "", SelectionAfterV1::Mapped, &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(
        owner.invocation_state(&id, &mut r),
        Ok(InvocationState::Issued)
    );
    assert_eq!(owner.run(&id, &mut r), Err(InvocationError::NotSubmitted));
    let (mut owner, context, mut r) = fixture(4096);
    let id = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit(
            &id,
            &SourceIntent {
                writes: vec![],
                reads: vec![],
            },
            &mut r,
        )
        .unwrap();
    assert_eq!(
        owner.submit_literal_edit(&id, &context, "", SelectionAfterV1::Mapped, &mut r),
        Err(InvocationError::IntentMismatch)
    );
}

#[test]
fn equal_text_view_only_effect_is_spent_once_and_diagnostics_cannot_change_it() {
    for capacity in [None, Some(64), Some(1)] {
        let (mut owner, context, _) = fixture(4096);
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let id = owner.issue_invocation(&mut r).unwrap();
        owner
            .submit_literal_edit(&id, &context, "abc", SelectionAfterV1::CaretsAfter, &mut r)
            .unwrap();
        owner.run(&id, &mut r).unwrap();
        let InvocationOutcome::Committed(outcome) = owner.invocation_outcome(&id, &mut r).unwrap()
        else {
            panic!("not committed")
        };
        assert!(outcome.changes.is_empty());
        assert!(outcome.operation.is_some());
        assert!(outcome.history_group.is_none());
        assert_eq!(
            owner
                .read(&context.source.document, &mut r)
                .unwrap()
                .version,
            context.source
        );
        let generation = owner.generation();
        owner.run(&id, &mut r).unwrap();
        assert_eq!(owner.generation(), generation);
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(64) {
            assert!(
                records
                    .iter()
                    .any(|o| o.boundary == Boundary::LiteralEditPreparation)
            );
            assert!(
                records
                    .iter()
                    .any(|o| o.boundary == Boundary::InvocationReplay)
            );
            assert_eq!(
                records
                    .iter()
                    .filter(|o| o.boundary == Boundary::Publication)
                    .count(),
                1
            );
        }
    }
}
