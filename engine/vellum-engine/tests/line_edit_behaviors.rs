use vellum_engine::attachment::Affinity;
use vellum_engine::coordinates::BytePosition;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, Recorder};
use vellum_engine::selection::{Selection, SelectionBudget};
use vellum_engine::source::Limits;
fn s(a: usize, h: usize) -> Selection {
    Selection {
        anchor: BytePosition(a),
        head: BytePosition(h),
        affinity: Affinity::After,
    }
}
fn budget() -> LineEditBudget {
    LineEditBudget {
        max_lines: 16,
        max_prefix_bytes: 1024,
        max_indent_columns: 128,
        max_unit_bytes: 8,
        max_derived_bytes: 4096,
        max_source_scan_bytes: 1024,
    }
}
fn fixture(
    text: &[u8],
    selections: &[Selection],
    bounds: LineEditBudget,
) -> (SourceCoordinator, LogicalViewContext, Recorder) {
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
    owner.configure_line_edits(bounds, &mut r).unwrap();
    let source = owner.create(text, &mut r).unwrap();
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
    (owner, context, r)
}
#[test]
fn shared_lines_shift_once_reverse_ranges_exclude_the_terminal_line_and_undo_is_exact() {
    let (mut owner, context, mut r) = fixture(
        b" \talpha\r\n\tbeta\nz",
        &[s(9, 0), s(1, 1), s(2, 2)],
        budget(),
    );
    let request = LineEditV1::Outdent {
        unit: "  ".into(),
        tab_width: 4,
    };
    let plan = owner.prepare_line_edit(&context, &request, &mut r).unwrap();
    let outcome = owner.commit(plan, &mut r).unwrap();
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        "  alpha\r\n\tbeta\nz"
    );
    assert_eq!(outcome.changes.len(), 1);
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
        " \talpha\r\n\tbeta\nz"
    );
}
#[test]
fn indentation_checks_actual_tab_columns_not_current_plus_unit_and_bounds_whole_work() {
    let mut bounds = budget();
    bounds.max_indent_columns = 4;
    let (mut owner, context, mut r) = fixture(b"\tx", &[s(0, 0)], bounds);
    let plan = owner
        .prepare_line_edit(
            &context,
            &LineEditV1::Indent {
                unit: " ".into(),
                tab_width: 4,
            },
            &mut r,
        )
        .unwrap();
    owner.commit(plan, &mut r).unwrap();
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        " \tx"
    );
    let context = owner
        .logical_view(&context.view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    assert!(
        owner
            .prepare_line_edit(
                &context,
                &LineEditV1::Indent {
                    unit: "\t".into(),
                    tab_width: 4
                },
                &mut r
            )
            .is_err()
    );
    let generation = owner.generation();
    assert!(
        owner
            .prepare_line_edit(
                &context,
                &LineEditV1::Outdent {
                    unit: "x".into(),
                    tab_width: 4
                },
                &mut r
            )
            .is_err()
    );
    assert_eq!(owner.generation(), generation);
}
#[test]
fn newline_copies_only_prior_prefix_and_collapses_origin_carets_without_normalizing_other_endings()
{
    let (mut owner, context, mut r) = fixture(b" \tab\nend\r", &[s(1, 1)], budget());
    let request = LineEditV1::Newline {
        delimiter: GeneratedNewlineV1::CrLf,
        indent: NewlineIndentV1::CopyPrefix,
    };
    let plan = owner.prepare_line_edit(&context, &request, &mut r).unwrap();
    owner.commit(plan, &mut r).unwrap();
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        " \r\n \tab\nend\r"
    );
    assert_eq!(
        owner
            .logical_view(&context.view, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[s(4, 4)]
    );
}
#[test]
fn newline_uses_each_original_prefix_and_eof_line_is_a_real_shift_target() {
    let (mut owner, context, mut r) = fixture(b" a\n\t\tb\r\n", &[s(2, 2), s(5, 5)], budget());
    let plan = owner
        .prepare_line_edit(
            &context,
            &LineEditV1::Newline {
                delimiter: GeneratedNewlineV1::Lf,
                indent: NewlineIndentV1::CopyPrefix,
            },
            &mut r,
        )
        .unwrap();
    owner.commit(plan, &mut r).unwrap();
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        " a\n \n\t\t\n\t\tb\r\n"
    );
    assert_eq!(
        owner
            .logical_view(&context.view, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[s(4, 4), s(10, 10)]
    );
    let (mut owner, context, mut r) = fixture(b"a\r", &[s(2, 2)], budget());
    let plan = owner
        .prepare_line_edit(
            &context,
            &LineEditV1::Indent {
                unit: "  ".into(),
                tab_width: 4,
            },
            &mut r,
        )
        .unwrap();
    owner.commit(plan, &mut r).unwrap();
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        "a\r  "
    );
}
#[test]
fn crlf_interior_stale_view_and_read_only_source_refuse_without_a_prefix() {
    let (owner, context, mut r) = fixture(b"a\r\nb", &[s(2, 2)], budget());
    assert!(
        owner
            .prepare_line_edit(
                &context,
                &LineEditV1::Newline {
                    delimiter: GeneratedNewlineV1::Cr,
                    indent: NewlineIndentV1::None
                },
                &mut r
            )
            .is_err()
    );
    let (mut owner, context, mut r) = fixture(b"abc", &[s(1, 1)], budget());
    let request = LineEditV1::Indent {
        unit: " ".into(),
        tab_width: 4,
    };
    let plan = owner.prepare_line_edit(&context, &request, &mut r).unwrap();
    owner
        .set_logical_selections(&context, &[s(2, 2)], 0, &mut r)
        .unwrap();
    assert_eq!(owner.commit(plan, &mut r), Err(CoordinatorError::Stale));
    let context = owner
        .logical_view(&context.view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    owner
        .set_read_only(&context.source.document, true, &mut r)
        .unwrap();
    assert!(owner.prepare_line_edit(&context, &request, &mut r).is_err());
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        "abc"
    );
}
#[test]
fn every_line_work_bound_refuses_wholly_and_empty_outdent_spends_no_publication_or_history() {
    let base = budget();
    for bounds in [
        LineEditBudget {
            max_lines: 0,
            ..base
        },
        LineEditBudget {
            max_prefix_bytes: 0,
            ..base
        },
        LineEditBudget {
            max_indent_columns: 0,
            ..base
        },
        LineEditBudget {
            max_unit_bytes: 0,
            ..base
        },
        LineEditBudget {
            max_derived_bytes: 0,
            ..base
        },
        LineEditBudget {
            max_source_scan_bytes: 0,
            ..base
        },
    ] {
        let (owner, context, mut r) = fixture(b" x", &[s(1, 1)], bounds);
        assert!(
            owner
                .prepare_line_edit(
                    &context,
                    &LineEditV1::Indent {
                        unit: " ".into(),
                        tab_width: 4
                    },
                    &mut r
                )
                .is_err()
        );
        assert_eq!(
            owner
                .read(&context.source.document, &mut r)
                .unwrap()
                .source
                .text(),
            " x"
        );
    }
    let (mut owner, context, mut r) = fixture(b"abc", &[s(1, 1)], base);
    let generation = owner.generation();
    let plan = owner
        .prepare_line_edit(
            &context,
            &LineEditV1::Outdent {
                unit: " ".into(),
                tab_width: 4,
            },
            &mut r,
        )
        .unwrap();
    let outcome = owner.commit(plan, &mut r).unwrap();
    assert!(outcome.operation.is_none());
    assert!(outcome.history_group.is_none());
    assert_eq!(owner.generation(), generation);
}
#[test]
fn line_preparation_is_redacted_and_diagnostic_capacity_cannot_change_the_edit() {
    for capacity in [None, Some(64), Some(1)] {
        let (mut owner, context, _) = fixture("\t秘密\r\n".as_bytes(), &[s(1, 1)], budget());
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let plan = owner
            .prepare_line_edit(
                &context,
                &LineEditV1::Outdent {
                    unit: "  ".into(),
                    tab_width: 4,
                },
                &mut r,
            )
            .unwrap();
        assert!(!format!("{plan:?}").contains("秘密"));
        owner.commit(plan, &mut r).unwrap();
        assert_eq!(
            owner
                .read(&context.source.document, &mut r)
                .unwrap()
                .source
                .text(),
            "  秘密\r\n"
        );
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(64) {
            assert!(
                records
                    .iter()
                    .any(|o| o.boundary == Boundary::LineEditPreparation)
            );
        }
    }
}
