//! Command semantics over host-attested tables, not segmenter conformance.
use vellum_engine::attachment::Affinity;
use vellum_engine::coordinates::BytePosition;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, Recorder};
use vellum_engine::selection::{Selection, SelectionBudget};
use vellum_engine::source::{Limits, SourceError};
fn s(a: usize, h: usize) -> Selection {
    Selection {
        anchor: BytePosition(a),
        head: BytePosition(h),
        affinity: Affinity::After,
    }
}
fn command(
    unit: UnicodeUnitV1,
    direction: UnicodeDirectionV1,
    count: usize,
    extend: bool,
) -> UnicodeCommandV1 {
    UnicodeCommandV1 {
        unit,
        direction,
        count,
        extend,
    }
}
fn bounds() -> UnicodeCommandBudget {
    UnicodeCommandBudget {
        max_steps: 64,
        max_derived_bytes: 4096,
    }
}
fn fixture(
    selections: &[Selection],
    budget: UnicodeCommandBudget,
) -> (
    SourceCoordinator,
    LogicalViewContext,
    BoundaryEvidenceId,
    Recorder,
) {
    let mut owner = SourceCoordinator::with_history_limits(
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
    owner.configure_unicode_commands(budget, &mut r).unwrap();
    let source = owner.create("a\u{301} hi!".as_bytes(), &mut r).unwrap();
    let capture = owner.begin_boundary_capture(&source, &mut r).unwrap();
    let table = BoundaryTableV1 {
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
    };
    let evidence = owner
        .accept_boundary_evidence(&capture, &table, &mut r)
        .unwrap();
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
    (owner, context, evidence, r)
}
#[test]
fn incompatible_word_end_does_not_disable_graphemes_or_compatible_word_starts() {
    let (mut owner, context, _, mut r) = fixture(&[s(0, 0)], bounds());
    let capture = owner
        .begin_boundary_capture(&context.source, &mut r)
        .unwrap();
    let data = BoundaryTableV1 {
        profile: BoundaryProfileV1::Unicode18Uax29Revision49,
        graphemes: vec![0, 3, 4, 5, 6, 7]
            .into_iter()
            .map(BytePosition)
            .collect(),
        words: vec![0, 1, 3, 4, 6, 7]
            .into_iter()
            .map(BytePosition)
            .collect(),
        significant_words: vec![
            ByteRange { start: 0, end: 1 },
            ByteRange { start: 4, end: 6 },
        ],
    };
    let evidence = owner
        .accept_boundary_evidence(&capture, &data, &mut r)
        .unwrap();
    let bad = command(
        UnicodeUnitV1::WordEnd,
        UnicodeDirectionV1::Forward,
        1,
        false,
    );
    assert_eq!(
        owner
            .prepare_unicode_movement(&context, &evidence, bad, &mut r)
            .err(),
        Some(UnicodeCommandError::InconsistentProfile)
    );
    assert_eq!(
        owner
            .prepare_unicode_deletion(&context, &evidence, bad, &mut r)
            .err(),
        Some(UnicodeCommandError::InconsistentProfile)
    );
    let movement = owner
        .prepare_unicode_movement(
            &context,
            &evidence,
            command(
                UnicodeUnitV1::WordStart,
                UnicodeDirectionV1::Forward,
                1,
                false,
            ),
            &mut r,
        )
        .unwrap();
    owner.commit_unicode_movement(movement, &mut r).unwrap();
    assert_eq!(
        owner
            .logical_view(&context.view, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[s(4, 4)]
    );
    let current = owner
        .logical_view(&context.view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    owner
        .set_logical_selections(&current, &[s(0, 0)], 0, &mut r)
        .unwrap();
    let current = owner
        .logical_view(&context.view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    let deletion = owner
        .prepare_unicode_deletion(
            &current,
            &evidence,
            command(
                UnicodeUnitV1::Grapheme,
                UnicodeDirectionV1::Forward,
                1,
                false,
            ),
            &mut r,
        )
        .unwrap();
    owner.commit(deletion, &mut r).unwrap();
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        " hi!"
    );
}
#[test]
fn movement_can_leave_cluster_interior_but_implicit_deletion_cannot_split_it() {
    let (mut owner, context, evidence, mut r) = fixture(&[s(1, 1)], bounds());
    let request = command(
        UnicodeUnitV1::Grapheme,
        UnicodeDirectionV1::Forward,
        1,
        false,
    );
    assert_eq!(
        owner
            .prepare_unicode_deletion(&context, &evidence, request, &mut r)
            .err(),
        Some(UnicodeCommandError::InvalidBoundary)
    );
    owner
        .set_read_only(&context.source.document, true, &mut r)
        .unwrap();
    let plan = owner
        .prepare_unicode_movement(&context, &evidence, request, &mut r)
        .unwrap();
    assert!(owner.commit_unicode_movement(plan, &mut r).unwrap().changed);
    assert_eq!(
        owner
            .logical_view(&context.view, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[s(3, 3)]
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
#[test]
fn selection_collapse_consumes_one_count_and_extension_keeps_anchor_and_affinity() {
    for (direction, count, extend, expected) in [
        (UnicodeDirectionV1::Forward, 1, false, s(6, 6)),
        (UnicodeDirectionV1::Forward, 2, false, s(7, 7)),
        (UnicodeDirectionV1::Backward, 1, false, s(3, 3)),
        (UnicodeDirectionV1::Forward, 1, true, s(6, 4)),
    ] {
        let (mut owner, context, evidence, mut r) = fixture(&[s(6, 3)], bounds());
        let plan = owner
            .prepare_unicode_movement(
                &context,
                &evidence,
                command(UnicodeUnitV1::Grapheme, direction, count, extend),
                &mut r,
            )
            .unwrap();
        owner.commit_unicode_movement(plan, &mut r).unwrap();
        assert_eq!(
            owner
                .logical_view(&context.view, &mut r)
                .unwrap()
                .selections
                .selections(),
            &[expected]
        );
    }
}
#[test]
fn word_start_and_end_traversals_use_significant_intervals_and_terminal_edges() {
    for (at, unit, direction, expected) in [
        (3, UnicodeUnitV1::WordStart, UnicodeDirectionV1::Forward, 4),
        (3, UnicodeUnitV1::WordEnd, UnicodeDirectionV1::Forward, 6),
        (4, UnicodeUnitV1::WordStart, UnicodeDirectionV1::Backward, 0),
        (7, UnicodeUnitV1::WordEnd, UnicodeDirectionV1::Backward, 6),
        (7, UnicodeUnitV1::WordStart, UnicodeDirectionV1::Forward, 7),
    ] {
        let (mut owner, context, evidence, mut r) = fixture(&[s(at, at)], bounds());
        let plan = owner
            .prepare_unicode_movement(
                &context,
                &evidence,
                command(unit, direction, 1, false),
                &mut r,
            )
            .unwrap();
        owner.commit_unicode_movement(plan, &mut r).unwrap();
        assert_eq!(
            owner
                .logical_view(&context.view, &mut r)
                .unwrap()
                .selections
                .selections(),
            &[s(expected, expected)]
        );
    }
}
#[test]
fn explicit_partial_cluster_selection_deletes_exactly_and_shared_undo_restores_bytes() {
    let (mut owner, context, evidence, mut r) = fixture(&[s(0, 1)], bounds());
    let request = command(
        UnicodeUnitV1::Grapheme,
        UnicodeDirectionV1::Forward,
        5,
        false,
    );
    let plan = owner
        .prepare_unicode_deletion(&context, &evidence, request, &mut r)
        .unwrap();
    let outcome = owner.commit(plan, &mut r).unwrap();
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        "\u{301} hi!"
    );
    assert_eq!(
        owner
            .logical_view(&context.view, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[s(0, 0)]
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
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        "a\u{301} hi!"
    );
    assert_eq!(
        owner
            .prepare_unicode_deletion(&context, &evidence, request, &mut r)
            .err(),
        Some(UnicodeCommandError::Selection(
            vellum_engine::selection::SelectionError::Stale
        ))
    );
}
#[test]
fn implicit_deletion_is_cluster_safe_and_colliding_ranges_refuse_wholly() {
    let (mut owner, context, evidence, mut r) = fixture(&[s(3, 3)], bounds());
    let request = command(
        UnicodeUnitV1::Grapheme,
        UnicodeDirectionV1::Backward,
        1,
        false,
    );
    let plan = owner
        .prepare_unicode_deletion(&context, &evidence, request, &mut r)
        .unwrap();
    owner.commit(plan, &mut r).unwrap();
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        " hi!"
    );
    let (owner, context, evidence, mut r) = fixture(&[s(0, 0), s(3, 3)], bounds());
    assert_eq!(
        owner
            .prepare_unicode_deletion(
                &context,
                &evidence,
                command(
                    UnicodeUnitV1::Grapheme,
                    UnicodeDirectionV1::Forward,
                    2,
                    false
                ),
                &mut r
            )
            .err(),
        Some(UnicodeCommandError::Coordinator(CoordinatorError::Source(
            SourceError::AmbiguousEdits
        )))
    );
}
#[test]
fn retired_evidence_and_stale_selection_cannot_publish_prepared_commands() {
    let request = command(
        UnicodeUnitV1::Grapheme,
        UnicodeDirectionV1::Forward,
        1,
        false,
    );
    let (mut owner, context, evidence, mut r) = fixture(&[s(0, 0)], bounds());
    let movement = owner
        .prepare_unicode_movement(&context, &evidence, request, &mut r)
        .unwrap();
    let deletion = owner
        .prepare_unicode_deletion(&context, &evidence, request, &mut r)
        .unwrap();
    owner.retire_boundary_evidence(&evidence, &mut r).unwrap();
    let generation = owner.generation();
    assert_eq!(
        owner.commit_unicode_movement(movement, &mut r).err(),
        Some(UnicodeCommandError::Boundary(BoundaryError::Unavailable))
    );
    assert_eq!(
        owner.commit(deletion, &mut r),
        Err(CoordinatorError::Unavailable)
    );
    assert_eq!(owner.generation(), generation);
    let (mut owner, context, evidence, mut r) = fixture(&[s(0, 0)], bounds());
    let plan = owner
        .prepare_unicode_movement(&context, &evidence, request, &mut r)
        .unwrap();
    owner
        .set_logical_selections(&context, &[s(3, 3)], 0, &mut r)
        .unwrap();
    assert!(owner.commit_unicode_movement(plan, &mut r).is_err());
}
#[test]
fn positive_total_steps_derived_bytes_and_delete_extension_are_explicit_bounds() {
    for budget in [
        UnicodeCommandBudget {
            max_steps: 1,
            ..bounds()
        },
        UnicodeCommandBudget {
            max_derived_bytes: 0,
            ..bounds()
        },
    ] {
        let (owner, context, evidence, mut r) = fixture(&[s(0, 0), s(3, 3)], budget);
        assert_eq!(
            owner
                .prepare_unicode_movement(
                    &context,
                    &evidence,
                    command(
                        UnicodeUnitV1::Grapheme,
                        UnicodeDirectionV1::Forward,
                        1,
                        false
                    ),
                    &mut r
                )
                .err(),
            Some(UnicodeCommandError::Limit)
        );
    }
    let (owner, context, evidence, mut r) = fixture(&[s(0, 0)], bounds());
    assert_eq!(
        owner
            .prepare_unicode_movement(
                &context,
                &evidence,
                command(
                    UnicodeUnitV1::Grapheme,
                    UnicodeDirectionV1::Forward,
                    0,
                    false
                ),
                &mut r
            )
            .err(),
        Some(UnicodeCommandError::InvalidRequest)
    );
    assert_eq!(
        owner
            .prepare_unicode_deletion(
                &context,
                &evidence,
                command(
                    UnicodeUnitV1::Grapheme,
                    UnicodeDirectionV1::Forward,
                    1,
                    true
                ),
                &mut r
            )
            .err(),
        Some(UnicodeCommandError::InvalidRequest)
    );
}
#[test]
fn optional_diagnostics_cannot_change_deletion_or_leak_payload_and_directional_noop_is_identity() {
    for capacity in [None, Some(64), Some(1)] {
        let (mut owner, context, evidence, _) = fixture(&[s(0, 0)], bounds());
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let plan = owner
            .prepare_unicode_deletion(
                &context,
                &evidence,
                command(
                    UnicodeUnitV1::Grapheme,
                    UnicodeDirectionV1::Backward,
                    1,
                    false,
                ),
                &mut r,
            )
            .unwrap();
        assert!(!format!("{plan:?}").contains("hi!"));
        let generation = owner.generation();
        let outcome = owner.commit(plan, &mut r).unwrap();
        assert!(outcome.operation.is_none());
        assert!(outcome.history_group.is_none());
        assert_eq!(owner.generation(), generation);
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(64) {
            assert!(
                records
                    .iter()
                    .any(|o| o.boundary == Boundary::UnicodePreparation)
            );
        }
    }
}
