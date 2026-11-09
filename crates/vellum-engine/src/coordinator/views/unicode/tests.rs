use super::*;
fn budget() -> UnicodeCommandBudget {
    UnicodeCommandBudget {
        max_steps: 64,
        max_derived_bytes: 4096,
    }
}
fn s(a: usize, h: usize) -> Selection {
    Selection {
        anchor: BytePosition(a),
        head: BytePosition(h),
        affinity: Affinity::After,
    }
}
fn request(direction: UnicodeDirectionV1) -> UnicodeCommandV1 {
    UnicodeCommandV1 {
        unit: UnicodeUnitV1::Grapheme,
        direction,
        count: 1,
        extend: false,
    }
}
fn fixture() -> (
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
    owner.configure_unicode_commands(budget(), &mut r).unwrap();
    let source = owner.create("a\u{301} hi!".as_bytes(), &mut r).unwrap();
    let capture = owner.begin_boundary_capture(&source, &mut r).unwrap();
    let data = BoundaryTableV1 {
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
        .accept_boundary_evidence(&capture, &data, &mut r)
        .unwrap();
    let view = owner.create_logical_view(&mut r).unwrap();
    let stamp = owner.logical_view(&view, &mut r).unwrap().stamp;
    owner
        .attach_logical_view(&view, stamp, &source, &[s(0, 0)], 0, &mut r)
        .unwrap();
    let context = owner
        .logical_view(&view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    (owner, context, evidence, r)
}
#[test]
fn movement_and_deletion_match_an_independent_strict_grid_oracle_for_all_small_selections() {
    let source = Utf8Source::decode("a\u{301}\r\nb".as_bytes(), 1024).unwrap();
    let grid = [0, 3, 5, 6];
    let data = BoundaryTableV1 {
        profile: BoundaryProfileV1::Unicode18Uax29Revision49,
        graphemes: grid.into_iter().map(BytePosition).collect(),
        words: vec![BytePosition(0), BytePosition(6)],
        significant_words: vec![],
    };
    let normalization = SelectionBudget {
        max_input_selections: 16,
        max_output_selections: 16,
        max_work: 65536,
    };
    for a in [0, 1, 3, 4, 5, 6] {
        for h in [0, 1, 3, 4, 5, 6] {
            for direction in [UnicodeDirectionV1::Forward, UnicodeDirectionV1::Backward] {
                for count in 1..=3 {
                    for extend in [false, true] {
                        let selection = s(a, h);
                        let before =
                            normalize_selections(&source, &[selection], 0, normalization).unwrap();
                        let command = UnicodeCommandV1 {
                            unit: UnicodeUnitV1::Grapheme,
                            direction,
                            count,
                            extend,
                        };
                        let mut at = h;
                        let mut steps = count;
                        if a != h && !extend {
                            at = if direction == UnicodeDirectionV1::Forward {
                                a.max(h)
                            } else {
                                a.min(h)
                            };
                            steps -= 1;
                        }
                        for _ in 0..steps {
                            at = if direction == UnicodeDirectionV1::Forward {
                                *grid.iter().find(|p| **p > at).unwrap_or(&6)
                            } else {
                                *grid.iter().rev().find(|p| **p < at).unwrap_or(&0)
                            };
                        }
                        let after =
                            movement(&source, &before, &data, command, budget(), normalization)
                                .unwrap();
                        assert_eq!(after.selections(), &[s(if extend { a } else { at }, at)]);
                        if !extend {
                            let deleted = deletion(&source, &before, &data, command, budget());
                            if a == h && !grid.contains(&a) {
                                assert_eq!(deleted, Err(UnicodeCommandError::InvalidBoundary));
                            } else {
                                let (lo, hi) = if a != h {
                                    (a.min(h), a.max(h))
                                } else {
                                    (h.min(at), h.max(at))
                                };
                                let edits = deleted.unwrap();
                                assert_eq!((edits[0].start, edits[0].end), (lo, hi));
                                assert_eq!(
                                    source
                                        .prepare(
                                            &edits,
                                            Limits {
                                                max_source_bytes: 1024,
                                                max_edits: 8,
                                                max_edit_payload_bytes: 1024
                                            }
                                        )
                                        .unwrap()
                                        .text(),
                                    format!("{}{}", &source.text()[..lo], &source.text()[hi..])
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn unicode_deletion_maps_real_annotation_peer_and_history_or_refuses_the_last_counter_wholly() {
    for exhausted in [false, true] {
        let (mut owner, context, evidence, mut r) = fixture();
        owner.annotations.limits = Some(AnnotationLimits {
            max_issued: 8,
            max_records: 8,
            max_body_bytes: 128,
            max_retained_body_bytes: 1024,
            max_mapping_steps: 65536,
            max_live_snapshots: 8,
            max_snapshot_bytes: 4096,
        });
        let peer = owner.create_logical_view(&mut r).unwrap();
        let stamp = owner.logical_view(&peer, &mut r).unwrap().stamp;
        owner
            .attach_logical_view(&peer, stamp, &context.source, &[s(7, 7)], 0, &mut r)
            .unwrap();
        let note = owner
            .prepare_compound(
                &[],
                &[],
                &[AnnotationMutation::Create {
                    target: SourceTarget {
                        source: context.source.clone(),
                        basis: TargetBasis::Before,
                        target: crate::attachment::AttachmentTarget::ContentRange {
                            start: 4,
                            end: 6,
                        },
                    },
                    body: "word note".into(),
                }],
                &mut r,
            )
            .unwrap();
        let annotation = owner.commit(note, &mut r).unwrap().annotation_changes[0]
            .annotation
            .clone();
        if exhausted {
            owner.views.rows[peer.slot]
                .as_mut()
                .unwrap()
                .stamp
                .selection_generation = u64::MAX;
        }
        let plan = owner
            .prepare_unicode_deletion(
                &context,
                &evidence,
                request(UnicodeDirectionV1::Forward),
                &mut r,
            )
            .unwrap();
        let generation = owner.generation();
        let result = owner.commit(plan, &mut r);
        if exhausted {
            assert_eq!(result, Err(CoordinatorError::VersionExhausted));
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
        } else {
            assert_eq!(
                owner
                    .read(&context.source.document, &mut r)
                    .unwrap()
                    .source
                    .text(),
                " hi!"
            );
            assert_eq!(
                owner
                    .logical_view(&peer, &mut r)
                    .unwrap()
                    .selections
                    .selections(),
                &[s(4, 4)]
            );
            assert_eq!(
                owner.annotation(&annotation, &mut r).unwrap().placement,
                crate::attachment::Placement::Located {
                    target: crate::attachment::AttachmentTarget::ContentRange { start: 1, end: 3 },
                    evidence: crate::attachment::LocationEvidence::Located
                }
            );
            assert_eq!(owner.history_usage().retained_groups, 2);
            let undo = owner
                .prepare_reversal(
                    &result.unwrap().history_group.unwrap(),
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
                owner.annotation(&annotation, &mut r).unwrap().body,
                "word note"
            );
        }
        assert!(!owner.is_faulted());
    }
}
#[test]
fn private_movement_result_invalid_delete_intent_and_multiple_capture_slots_are_faults() {
    for mode in 0..4 {
        let (mut owner, context, evidence, mut r) = fixture();
        let generation = owner.generation();
        if mode == 0 {
            let mut plan = owner
                .prepare_unicode_movement(
                    &context,
                    &evidence,
                    request(UnicodeDirectionV1::Forward),
                    &mut r,
                )
                .unwrap();
            plan.after = plan.before.copy().unwrap();
            assert_eq!(
                owner.commit_unicode_movement(plan, &mut r),
                Err(UnicodeCommandError::Faulted)
            );
        } else {
            let mut plan = owner
                .prepare_unicode_deletion(
                    &context,
                    &evidence,
                    request(UnicodeDirectionV1::Forward),
                    &mut r,
                )
                .unwrap();
            if mode == 1 {
                let DerivedEdit::Unicode(capture) = plan.derived_edit.first_mut().unwrap() else {
                    panic!("not unicode")
                };
                capture.request.count = 0;
            } else if mode == 2 {
                plan.derived_edit.push(DerivedEdit::Unicode(UnicodeEdit {
                    context: context.clone(),
                    evidence: evidence.clone(),
                    request: request(UnicodeDirectionV1::Forward),
                    budget: budget(),
                }));
            } else {
                plan.derived_edit.clear();
            }
            assert_eq!(owner.commit(plan, &mut r), Err(CoordinatorError::Faulted));
        }
        assert!(owner.is_faulted());
        assert_eq!(owner.generation(), generation);
        assert_eq!(owner.history_usage().retained_groups, 0);
        assert_eq!(
            owner.documents[context.source.document.slot]
                .as_ref()
                .unwrap()
                .source
                .text(),
            "a\u{301} hi!"
        );
    }
}
#[test]
fn exhausted_counters_allow_complete_motion_and_deletion_identity_but_no_changed_prefix() {
    let (mut owner, context, evidence, mut r) = fixture();
    owner.views.rows[context.view.slot]
        .as_mut()
        .unwrap()
        .stamp
        .selection_generation = u64::MAX;
    owner.generation = u64::MAX;
    let context = owner
        .logical_view(&context.view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    let movement = owner
        .prepare_unicode_movement(
            &context,
            &evidence,
            request(UnicodeDirectionV1::Backward),
            &mut r,
        )
        .unwrap();
    assert!(
        !owner
            .commit_unicode_movement(movement, &mut r)
            .unwrap()
            .changed
    );
    let deletion = owner
        .prepare_unicode_deletion(
            &context,
            &evidence,
            request(UnicodeDirectionV1::Backward),
            &mut r,
        )
        .unwrap();
    let outcome = owner.commit(deletion, &mut r).unwrap();
    assert!(outcome.operation.is_none());
    assert!(outcome.history_group.is_none());
    let changed = owner
        .prepare_unicode_movement(
            &context,
            &evidence,
            request(UnicodeDirectionV1::Forward),
            &mut r,
        )
        .unwrap();
    assert_eq!(
        owner.commit_unicode_movement(changed, &mut r),
        Err(UnicodeCommandError::Selection(
            SelectionError::VersionExhausted
        ))
    );
    assert!(!owner.is_faulted());
    assert_eq!(owner.generation(), u64::MAX);
}
