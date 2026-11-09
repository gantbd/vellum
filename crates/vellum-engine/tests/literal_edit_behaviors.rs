use vellum_engine::attachment::Affinity;
use vellum_engine::coordinates::BytePosition;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, Recorder};
use vellum_engine::selection::*;
use vellum_engine::source::{Limits, SourceError};
fn owner(payload: usize) -> (SourceCoordinator, Recorder) {
    let mut owner = SourceCoordinator::with_history_limits(
        CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 8,
                max_edit_payload_bytes: payload,
            },
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
    (owner, r)
}
fn s(anchor: usize, head: usize, affinity: Affinity) -> Selection {
    Selection {
        anchor: BytePosition(anchor),
        head: BytePosition(head),
        affinity,
    }
}
fn view(
    owner: &mut SourceCoordinator,
    source: &DocumentVersion,
    input: &[Selection],
    primary: usize,
    r: &mut Recorder,
) -> LogicalViewId {
    let id = owner.create_logical_view(r).unwrap();
    let stamp = owner.logical_view(&id, r).unwrap().stamp;
    owner
        .attach_logical_view(&id, stamp, source, input, primary, r)
        .unwrap();
    id
}
#[test]
fn carets_after_edit_multiple_ranges_and_map_current_peers_in_one_history_publication() {
    let (mut owner, mut r) = owner(4096);
    let source = owner.create(b"abcdef", &mut r).unwrap();
    let target = view(
        &mut owner,
        &source,
        &[s(2, 0, Affinity::Before), s(4, 6, Affinity::After)],
        1,
        &mut r,
    );
    let peer = view(&mut owner, &source, &[s(5, 1, Affinity::Before)], 0, &mut r);
    let context = owner
        .logical_view(&target, &mut r)
        .unwrap()
        .context()
        .unwrap();
    let plan = owner
        .prepare_literal_edit(&context, "X", SelectionAfterV1::CaretsAfter, &mut r)
        .unwrap();
    let generation = owner.generation();
    let outcome = owner.commit(plan, &mut r).unwrap();
    assert_eq!(outcome.generation, generation + 1);
    assert_eq!(
        owner.read(&source.document, &mut r).unwrap().source.text(),
        "XcdX"
    );
    assert_eq!(owner.history_usage().retained_groups, 1);
    let after = owner.logical_view(&target, &mut r).unwrap();
    assert_eq!(
        after.selections.selections(),
        &[s(1, 1, Affinity::Before), s(4, 4, Affinity::After)]
    );
    assert_eq!(after.selections.primary(), 1);
    assert_eq!(
        owner
            .logical_view(&peer, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[s(4, 0, Affinity::Before)]
    );
    let undo = owner
        .prepare_reversal(
            &outcome.history_group.unwrap(),
            HistoryDirection::Undo,
            &mut r,
        )
        .unwrap();
    owner.commit(undo, &mut r).unwrap();
    assert_eq!(
        owner.read(&source.document, &mut r).unwrap().source.text(),
        "abcdef"
    );
}
#[test]
fn equal_text_distinguishes_mapped_identity_from_carets_after_without_source_revision_or_history() {
    for after in [SelectionAfterV1::Mapped, SelectionAfterV1::CaretsAfter] {
        let (mut owner, mut r) = owner(4096);
        let source = owner.create(b"abc", &mut r).unwrap();
        let id = view(&mut owner, &source, &[s(0, 3, Affinity::Before)], 0, &mut r);
        let context = owner.logical_view(&id, &mut r).unwrap().context().unwrap();
        let generation = owner.generation();
        let plan = owner
            .prepare_literal_edit(&context, "abc", after, &mut r)
            .unwrap();
        let result = owner.commit(plan, &mut r).unwrap();
        assert!(result.changes.is_empty());
        assert!(result.history_group.is_none());
        assert_eq!(
            owner.read(&source.document, &mut r).unwrap().version,
            source
        );
        if after == SelectionAfterV1::Mapped {
            assert!(result.operation.is_none());
            assert_eq!(owner.generation(), generation);
        } else {
            assert!(result.operation.is_some());
            assert_eq!(owner.generation(), generation + 1);
            assert_eq!(
                owner
                    .logical_view(&id, &mut r)
                    .unwrap()
                    .selections
                    .selections(),
                &[s(3, 3, Affinity::Before)]
            );
        }
    }
}
#[test]
fn stale_target_or_new_peer_and_changed_write_policy_cannot_publish_an_old_derived_edit() {
    for change in 0..3 {
        let (mut owner, mut r) = owner(4096);
        let source = owner.create(b"abc", &mut r).unwrap();
        let id = view(&mut owner, &source, &[s(1, 1, Affinity::After)], 0, &mut r);
        let context = owner.logical_view(&id, &mut r).unwrap().context().unwrap();
        let plan = owner
            .prepare_literal_edit(&context, "X", SelectionAfterV1::CaretsAfter, &mut r)
            .unwrap();
        match change {
            0 => {
                owner
                    .set_logical_selections(&context, &[s(2, 2, Affinity::After)], 0, &mut r)
                    .unwrap();
            }
            1 => {
                view(&mut owner, &source, &[s(0, 0, Affinity::Before)], 0, &mut r);
            }
            _ => owner.set_read_only(&source.document, true, &mut r).unwrap(),
        }
        let generation = owner.generation();
        assert_eq!(
            owner.commit(plan, &mut r),
            Err(if change == 2 {
                CoordinatorError::Unavailable
            } else {
                CoordinatorError::Stale
            })
        );
        assert_eq!(owner.generation(), generation);
        assert_eq!(
            owner.read(&source.document, &mut r).unwrap().source.text(),
            "abc"
        );
        assert_eq!(owner.history_usage().retained_groups, 0);
    }
}
#[test]
fn retained_overlapping_carets_do_not_legalize_an_ambiguous_source_batch() {
    let (mut owner, mut r) = owner(4096);
    let source = owner.create(b"abc", &mut r).unwrap();
    let id = view(
        &mut owner,
        &source,
        &[s(0, 2, Affinity::Before), s(1, 1, Affinity::After)],
        0,
        &mut r,
    );
    let context = owner.logical_view(&id, &mut r).unwrap().context().unwrap();
    let generation = owner.generation();
    assert_eq!(
        owner
            .prepare_literal_edit(&context, "X", SelectionAfterV1::CaretsAfter, &mut r)
            .err(),
        Some(SelectionError::Source(SourceError::AmbiguousEdits))
    );
    assert_eq!(owner.generation(), generation);
    assert_eq!(owner.history_usage().retained_groups, 0);
}
#[test]
fn complete_payload_budget_and_literal_feff_are_not_trimmed_or_treated_as_a_file_signature() {
    let (mut owner, mut r) = owner(8);
    let source = owner.create(b"ab", &mut r).unwrap();
    let id = view(
        &mut owner,
        &source,
        &[s(0, 0, Affinity::After), s(2, 2, Affinity::Before)],
        1,
        &mut r,
    );
    let context = owner.logical_view(&id, &mut r).unwrap().context().unwrap();
    assert_eq!(
        owner
            .prepare_literal_edit(&context, "12345", SelectionAfterV1::Mapped, &mut r)
            .err(),
        Some(SelectionError::Source(SourceError::PayloadLimit))
    );
    let plan = owner
        .prepare_literal_edit(&context, "\u{feff}", SelectionAfterV1::CaretsAfter, &mut r)
        .unwrap();
    owner.commit(plan, &mut r).unwrap();
    assert_eq!(
        owner.read(&source.document, &mut r).unwrap().source.text(),
        "\u{feff}ab\u{feff}"
    );
}
#[test]
fn diagnostics_disabled_available_or_full_preserve_the_edit_and_never_disclose_its_literal() {
    for capacity in [None, Some(256), Some(1)] {
        let (mut owner, _) = owner(4096);
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let source = owner.create(b"abc", &mut r).unwrap();
        let id = view(&mut owner, &source, &[s(1, 1, Affinity::Before)], 0, &mut r);
        let context = owner.logical_view(&id, &mut r).unwrap().context().unwrap();
        let plan = owner
            .prepare_literal_edit(&context, "PRIVATE", SelectionAfterV1::CaretsAfter, &mut r)
            .unwrap();
        assert!(!format!("{plan:?}").contains("PRIVATE"));
        owner.commit(plan, &mut r).unwrap();
        assert_eq!(
            owner.read(&source.document, &mut r).unwrap().source.text(),
            "aPRIVATEbc"
        );
        assert_eq!(
            owner
                .logical_view(&id, &mut r)
                .unwrap()
                .selections
                .selections(),
            &[s(8, 8, Affinity::Before)]
        );
        let mut observations = Vec::new();
        while let Some(observation) = r.pop_front() {
            observations.push(observation);
        }
        if capacity == Some(256) {
            assert!(
                observations
                    .iter()
                    .any(|o| o.boundary == Boundary::LiteralEditPreparation)
            );
        }
        assert!(!format!("{observations:?}").contains("PRIVATE"));
        assert!(observations.iter().all(|o| o.service_time.is_none()));
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
    }
}
