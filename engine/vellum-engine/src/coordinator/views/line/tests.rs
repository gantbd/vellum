use super::*;
fn bounds() -> LineEditBudget {
    LineEditBudget {
        max_lines: 16,
        max_prefix_bytes: 1024,
        max_indent_columns: 256,
        max_unit_bytes: 8,
        max_derived_bytes: 4096,
        max_source_scan_bytes: 1024,
    }
}
fn selection(at: usize) -> Selection {
    Selection {
        anchor: BytePosition(at),
        head: BytePosition(at),
        affinity: Affinity::After,
    }
}
pub(in crate::coordinator) fn fixture() -> (SourceCoordinator, LogicalViewContext, Recorder) {
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
    owner.configure_line_edits(bounds(), &mut r).unwrap();
    let source = owner.create(b" \tfoo", &mut r).unwrap();
    let view = owner.create_logical_view(&mut r).unwrap();
    let stamp = owner.logical_view(&view, &mut r).unwrap().stamp;
    owner
        .attach_logical_view(&view, stamp, &source, &[selection(1)], 0, &mut r)
        .unwrap();
    let context = owner
        .logical_view(&view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    (owner, context, r)
}
#[test]
fn original_prefix_identity_preserves_content_annotation_and_shared_history_undo_is_exact() {
    let (mut owner, context, mut r) = fixture();
    owner.annotations.limits = Some(AnnotationLimits {
        max_issued: 8,
        max_records: 8,
        max_body_bytes: 128,
        max_retained_body_bytes: 1024,
        max_mapping_steps: 65536,
        max_live_snapshots: 8,
        max_snapshot_bytes: 4096,
    });
    let plan = owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                target: SourceTarget {
                    source: context.source.clone(),
                    basis: TargetBasis::Before,
                    target: crate::attachment::AttachmentTarget::ContentRange { start: 0, end: 1 },
                },
                body: "prefix comment".into(),
            }],
            &mut r,
        )
        .unwrap();
    let id = owner.commit(plan, &mut r).unwrap().annotation_changes[0]
        .annotation
        .clone();
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
    assert_eq!(plan.candidates[0].edits[0].start, 1);
    let outcome = owner.commit(plan, &mut r).unwrap();
    assert_eq!(
        owner.annotation(&id, &mut r).unwrap().placement,
        crate::attachment::Placement::Located {
            target: crate::attachment::AttachmentTarget::ContentRange { start: 0, end: 1 },
            evidence: crate::attachment::LocationEvidence::Located,
        }
    );
    assert_eq!(owner.history_usage().retained_groups, 2);
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
        " \tfoo"
    );
    assert_eq!(
        owner.annotation(&id, &mut r).unwrap().body,
        "prefix comment"
    );
}
#[test]
fn altered_owned_line_settings_are_a_fault_not_a_known_caller_refusal() {
    let (mut owner, context, mut r) = fixture();
    let mut plan = owner
        .prepare_line_edit(
            &context,
            &LineEditV1::Outdent {
                unit: "  ".into(),
                tab_width: 4,
            },
            &mut r,
        )
        .unwrap();
    let DerivedEdit::Line(capture) = plan.derived_edit.first_mut().unwrap() else {
        panic!("not line")
    };
    capture.request = LineEditV1::Outdent {
        unit: " ".into(),
        tab_width: 4,
    };
    let generation = owner.generation();
    assert_eq!(owner.commit(plan, &mut r), Err(CoordinatorError::Faulted));
    assert!(owner.is_faulted());
    assert_eq!(owner.generation(), generation);
    assert_eq!(owner.history_usage().retained_groups, 0);
    assert_eq!(
        owner.documents[context.source.document.slot]
            .as_ref()
            .unwrap()
            .source
            .text(),
        " \tfoo"
    );
}
#[test]
fn newline_selection_counter_exhaustion_cannot_publish_source_or_history() {
    let (mut owner, context, mut r) = fixture();
    owner.views.rows[context.view.slot]
        .as_mut()
        .unwrap()
        .stamp
        .selection_generation = u64::MAX;
    let context = owner
        .logical_view(&context.view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    let plan = owner
        .prepare_line_edit(
            &context,
            &LineEditV1::Newline {
                delimiter: GeneratedNewlineV1::Lf,
                indent: NewlineIndentV1::None,
            },
            &mut r,
        )
        .unwrap();
    let generation = owner.generation();
    assert_eq!(
        owner.commit(plan, &mut r),
        Err(CoordinatorError::VersionExhausted)
    );
    assert!(!owner.is_faulted());
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
fn expanded(prefix: &str, width: usize) -> String {
    let mut columns = String::new();
    for byte in prefix.bytes() {
        columns.push(' ');
        if byte == b'\t' {
            while !columns.len().is_multiple_of(width) {
                columns.push(' ');
            }
        }
    }
    columns
}
#[test]
fn all_small_prefixes_match_an_independent_expansion_oracle_and_minimal_suffix_edits() {
    let mut prefixes = vec![String::new()];
    for len in 1..=4 {
        for bits in 0..(1usize << len) {
            prefixes.push(
                (0..len)
                    .map(|i| if bits & (1 << i) == 0 { ' ' } else { '\t' })
                    .collect(),
            );
        }
    }
    let source_limits = Limits {
        max_source_bytes: 2048,
        max_edits: 32,
        max_edit_payload_bytes: 4096,
    };
    for prefix in prefixes {
        let source = Utf8Source::decode(format!("{prefix}λ\r\n").as_bytes(), 2048).unwrap();
        let selections = normalize_selections(
            &source,
            &[selection(0)],
            0,
            SelectionBudget {
                max_input_selections: 8,
                max_output_selections: 8,
                max_work: 65536,
            },
        )
        .unwrap();
        for width in 1..=4 {
            for unit in [" ", "  ", "\t"] {
                for indent in [false, true] {
                    let current = expanded(&prefix, width).len();
                    let step = expanded(unit, width).len();
                    let target = current.saturating_sub(step);
                    let replacement = if indent {
                        format!("{unit}{prefix}")
                    } else if unit == "\t" {
                        format!(
                            "{}{}",
                            "\t".repeat(target / width),
                            " ".repeat(target % width)
                        )
                    } else {
                        " ".repeat(target)
                    };
                    let request = if indent {
                        LineEditV1::Indent {
                            unit: unit.into(),
                            tab_width: width,
                        }
                    } else {
                        LineEditV1::Outdent {
                            unit: unit.into(),
                            tab_width: width,
                        }
                    };
                    let edits = derive::edits(&source, &selections, &request, bounds()).unwrap();
                    assert_eq!(
                        source.prepare(&edits, source_limits).unwrap().text(),
                        format!("{replacement}λ\r\n")
                    );
                    if !indent {
                        let keep = prefix
                            .bytes()
                            .zip(replacement.bytes())
                            .take_while(|(a, b)| a == b)
                            .count();
                        if let Some(edit) = edits.first() {
                            assert_eq!(edit.start, keep);
                        } else {
                            assert_eq!(prefix, replacement);
                        }
                    }
                }
            }
        }
    }
}
