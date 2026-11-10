use super::*;
#[test]
fn invocation_detects_missing_capture_or_changed_policy_as_unknown_before_publication() {
    for remove in [false, true] {
        let (mut owner, source, view, mut r) = owner();
        owner.invocations = invocation::InvocationTable::new(InvocationLimits {
            max_lifetime_issued: 4,
            max_retained: 4,
            max_intent_bytes: 4096,
        });
        let context = owner
            .logical_view(&view, &mut r)
            .unwrap()
            .context()
            .unwrap();
        let id = owner.issue_invocation(&mut r).unwrap();
        owner
            .submit_literal_edit(&id, &context, "X", SelectionAfterV1::CaretsAfter, &mut r)
            .unwrap();
        // This test module can reach the capture, but not the ledger phase. The
        // invocation test hook below deliberately mutates the retained plan.
        owner.corrupt_literal_invocation_for_test(&id, remove);
        let generation = owner.generation;
        assert_eq!(owner.run(&id, &mut r), Err(InvocationError::Faulted));
        assert_eq!(
            owner.invocation_outcome(&id, &mut r),
            Err(InvocationError::Faulted)
        );
        assert_eq!(owner.invocation_usage().faulted_executions, 1);
        assert_eq!(owner.generation, generation);
        assert!(owner.is_faulted());
        assert_eq!(
            owner.documents[source.document.slot]
                .as_ref()
                .unwrap()
                .source
                .text(),
            "abc"
        );
        assert_eq!(owner.history_usage().retained_groups, 0);
    }
}
fn owner() -> (SourceCoordinator, DocumentVersion, LogicalViewId, Recorder) {
    let mut owner = SourceCoordinator::with_history_limits(
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
                max_issued_views: 16,
                max_total_selections: 64,
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
    let source = owner.create(b"abc", &mut r).unwrap();
    let view = owner.create_logical_view(&mut r).unwrap();
    let stamp = owner.logical_view(&view, &mut r).unwrap().stamp;
    owner
        .attach_logical_view(
            &view,
            stamp,
            &source,
            &[Selection {
                anchor: BytePosition(0),
                head: BytePosition(3),
                affinity: Affinity::Before,
            }],
            0,
            &mut r,
        )
        .unwrap();
    (owner, source, view, r)
}
#[test]
fn mutated_frozen_literal_refuses_as_an_integrity_fault_without_publishing_a_prefix() {
    let (mut owner, source, view, mut r) = owner();
    let context = owner
        .logical_view(&view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    let mut prepared = owner
        .prepare_literal_edit(&context, "X", SelectionAfterV1::CaretsAfter, &mut r)
        .unwrap();
    let DerivedEdit::Literal(capture) = prepared.derived_edit.first_mut().unwrap() else {
        panic!("not literal")
    };
    capture.text = "different private intent".into();
    let generation = owner.generation;
    let history = owner.history_usage();
    assert_eq!(
        owner.commit(prepared, &mut r),
        Err(CoordinatorError::Faulted)
    );
    assert!(owner.faulted);
    assert_eq!(owner.generation, generation);
    assert_eq!(owner.history_usage(), history);
    assert_eq!(
        owner.documents[source.document.slot]
            .as_ref()
            .unwrap()
            .source
            .text(),
        "abc"
    );
    assert_eq!(
        owner.views.rows[view.slot]
            .as_ref()
            .unwrap()
            .selections
            .selections()[0]
            .head,
        BytePosition(3)
    );
}
#[test]
fn carets_after_overflow_cannot_publish_source_or_history() {
    let (mut owner, source, view, mut r) = owner();
    owner.views.rows[view.slot]
        .as_mut()
        .unwrap()
        .stamp
        .selection_generation = u64::MAX;
    let context = owner
        .logical_view(&view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    let prepared = owner
        .prepare_literal_edit(&context, "X", SelectionAfterV1::CaretsAfter, &mut r)
        .unwrap();
    let generation = owner.generation;
    let history = owner.history_usage();
    assert_eq!(
        owner.commit(prepared, &mut r),
        Err(CoordinatorError::VersionExhausted)
    );
    assert!(!owner.faulted);
    assert_eq!(owner.generation, generation);
    assert_eq!(owner.history_usage(), history);
    assert_eq!(
        owner.read(&source.document, &mut r).unwrap().version,
        source
    );
    assert_eq!(
        owner
            .logical_view(&view, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[Selection {
            anchor: BytePosition(0),
            head: BytePosition(3),
            affinity: Affinity::Before
        }]
    );
}
#[test]
fn equal_byte_caret_changes_need_no_source_counter_and_full_identity_needs_no_unused_counters() {
    let (mut owner, mut source, view, mut r) = owner();
    source.revision = u64::MAX;
    owner.documents[source.document.slot]
        .as_mut()
        .unwrap()
        .revision = u64::MAX;
    owner.views.rows[view.slot].as_mut().unwrap().source = Some(source.clone());
    let context = owner
        .logical_view(&view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    let plan = owner
        .prepare_literal_edit(&context, "abc", SelectionAfterV1::CaretsAfter, &mut r)
        .unwrap();
    let outcome = owner.commit(plan, &mut r).unwrap();
    assert!(outcome.changes.is_empty());
    assert!(outcome.operation.is_some());
    assert!(outcome.history_group.is_none());
    owner.generation = u64::MAX;
    owner.views.rows[view.slot]
        .as_mut()
        .unwrap()
        .stamp
        .selection_generation = u64::MAX;
    let context = owner
        .logical_view(&view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    let plan = owner
        .prepare_literal_edit(&context, "", SelectionAfterV1::CaretsAfter, &mut r)
        .unwrap();
    let outcome = owner.commit(plan, &mut r).unwrap();
    assert_eq!(outcome.generation, u64::MAX);
    assert!(outcome.operation.is_none());
    assert!(outcome.history_group.is_none());
    assert_eq!(
        owner.read(&source.document, &mut r).unwrap().version,
        source
    );
}
