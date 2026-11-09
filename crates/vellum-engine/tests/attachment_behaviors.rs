//! Executable attachment candidate laws; not annotation authoring acceptance.
use vellum_engine::attachment::*;
use vellum_engine::instrumentation::{Boundary, Disposition, Recorder};
use vellum_engine::source::{Limits, Replacement, Utf8Source};

fn limits() -> AttachmentLimits {
    AttachmentLimits {
        max_attachments: 128,
        max_mapping_steps: 4096,
    }
}
fn source_limits() -> Limits {
    Limits {
        max_source_bytes: 4096,
        max_edits: 64,
        max_edit_payload_bytes: 8192,
    }
}
fn source(text: &str) -> Utf8Source {
    Utf8Source::decode(text.as_bytes(), 4096).unwrap()
}
fn edit(start: usize, end: usize, expected: &str, inserted: &str) -> Replacement {
    Replacement {
        start,
        end,
        expected: expected.into(),
        inserted: inserted.into(),
    }
}
fn point(offset: usize, affinity: Affinity, profile: PointProfile) -> Placement {
    Placement::Located {
        target: AttachmentTarget::Point {
            offset,
            affinity,
            profile,
        },
        evidence: LocationEvidence::Located,
    }
}
fn range(start: usize, end: usize) -> Placement {
    Placement::Located {
        target: AttachmentTarget::ContentRange { start, end },
        evidence: LocationEvidence::Located,
    }
}
fn prepare(
    original: &Utf8Source,
    edits: &[Replacement],
    placements: &[Placement],
) -> PreparedAttachments {
    prepare_attachments(
        original,
        edits,
        placements,
        source_limits(),
        limits(),
        &mut Recorder::disabled(),
    )
    .unwrap()
}

#[test]
fn given_outer_boundary_insertions_then_content_ranges_exclude_neighbors_and_interior_insertions_are_affected()
 {
    let original = source("abcd");
    for (at, expected) in [(1, range(2, 4)), (3, range(1, 3))] {
        let result = prepare(&original, &[edit(at, at, "", "X")], &[range(1, 3)]);
        assert_eq!(result.placements(), [expected]);
    }
    let result = prepare(&original, &[edit(2, 2, "", "X")], &[range(1, 3)]);
    assert_eq!(
        result.placements(),
        [Placement::Located {
            target: AttachmentTarget::ContentRange { start: 1, end: 4 },
            evidence: LocationEvidence::Affected
        }]
    );
}

#[test]
fn given_complete_adjacent_removal_even_with_equal_inserted_bytes_then_original_content_target_detaches()
 {
    let original = source("abcd");
    let result = prepare(
        &original,
        &[edit(1, 2, "b", "b"), edit(2, 3, "c", "c")],
        &[range(1, 3)],
    );
    assert_eq!(result.source(), &original);
    assert!(!result.source_changed());
    assert!(result.placements_changed());
    assert_eq!(result.placements(), [Placement::Detached]);
    assert_eq!(original.text(), "abcd");
}

#[test]
fn given_partial_coverage_then_a_surviving_content_target_has_affected_not_revalidated_location() {
    let original = source("abcd");
    let result = prepare(&original, &[edit(1, 2, "b", "🙂")], &[range(1, 3)]);
    assert_eq!(
        result.placements(),
        [Placement::Located {
            target: AttachmentTarget::ContentRange { start: 1, end: 6 },
            evidence: LocationEvidence::Affected
        }]
    );
    assert_eq!(result.source().text(), "a🙂cd");
}

#[test]
fn given_one_collapsed_point_then_explicit_affinity_maps_once_without_widening_or_inversion() {
    let original = source("ab");
    for profile in [PointProfile::Position, PointProfile::Content] {
        let result = prepare(
            &original,
            &[edit(1, 1, "", "🙂")],
            &[
                point(1, Affinity::Before, profile),
                point(1, Affinity::After, profile),
            ],
        );
        assert_eq!(
            result.placements(),
            [
                point(1, Affinity::Before, profile),
                point(5, Affinity::After, profile)
            ]
        );
    }
}

#[test]
fn given_removed_point_context_then_content_detaches_but_position_collapses_by_explicit_affinity() {
    let original = source("abcd");
    for (affinity, offset) in [(Affinity::Before, 1), (Affinity::After, 3)] {
        let result = prepare(
            &original,
            &[edit(1, 3, "bc", "XY")],
            &[
                point(2, affinity, PointProfile::Content),
                point(2, affinity, PointProfile::Position),
            ],
        );
        assert_eq!(
            result.placements(),
            [
                Placement::Detached,
                Placement::Located {
                    target: AttachmentTarget::Point {
                        offset,
                        affinity,
                        profile: PointProfile::Position
                    },
                    evidence: LocationEvidence::Affected
                }
            ]
        );
    }
}

#[test]
fn given_replacement_outer_edges_and_an_adjacent_boundary_then_the_existing_edges_survive() {
    let original = source("abcd");
    let edits = [edit(0, 2, "ab", "X"), edit(2, 4, "cd", "YYY")];
    let result = prepare(
        &original,
        &edits,
        &[
            point(0, Affinity::After, PointProfile::Content),
            point(2, Affinity::Before, PointProfile::Content),
            point(4, Affinity::Before, PointProfile::Content),
        ],
    );
    assert_eq!(
        result.placements(),
        [
            point(0, Affinity::After, PointProfile::Content),
            point(1, Affinity::Before, PointProfile::Content),
            point(4, Affinity::Before, PointProfile::Content)
        ]
    );
}

#[test]
fn given_detached_unavailable_or_affected_evidence_then_restored_bytes_never_silently_reattach_or_refresh_it()
 {
    let original = source("abcd");
    let affected = Placement::Located {
        target: AttachmentTarget::ContentRange { start: 1, end: 3 },
        evidence: LocationEvidence::Affected,
    };
    let result = prepare(
        &original,
        &[edit(0, 0, "", "X")],
        &[Placement::Detached, Placement::Unavailable, affected],
    );
    assert_eq!(
        result.placements()[..2],
        [Placement::Detached, Placement::Unavailable]
    );
    assert_eq!(
        result.placements()[2],
        Placement::Located {
            target: AttachmentTarget::ContentRange { start: 2, end: 4 },
            evidence: LocationEvidence::Affected
        }
    );
    let undone = prepare(result.source(), &[edit(0, 1, "X", "")], result.placements());
    assert_eq!(undone.source(), &original);
    assert_eq!(
        undone.placements(),
        [Placement::Detached, Placement::Unavailable, affected]
    );
}

#[test]
fn given_document_scope_empty_insertion_or_empty_batch_then_no_affected_evidence_is_manufactured() {
    let original = source("abcd");
    let document = Placement::Located {
        target: AttachmentTarget::Document,
        evidence: LocationEvidence::Located,
    };
    for edits in [vec![], vec![edit(2, 2, "", "")]] {
        let placements = [document, range(1, 3)];
        let result = prepare(&original, &edits, &placements);
        assert_eq!(result.placements(), placements);
        assert!(!result.source_changed());
        assert!(!result.placements_changed());
    }
    let result = prepare(&original, &[edit(0, 4, "abcd", "X")], &[document]);
    assert_eq!(result.placements(), [document]);
}

#[test]
fn given_invalid_geometry_or_expected_bytes_then_the_whole_source_and_mapping_candidate_refuses() {
    let original = source("aÄb");
    for placement in [
        range(1, 1),
        range(3, 2),
        range(0, 5),
        range(0, 2),
        point(2, Affinity::Before, PointProfile::Position),
    ] {
        assert!(
            prepare_attachments(
                &original,
                &[edit(0, 1, "a", "X")],
                &[range(0, 1), placement],
                source_limits(),
                limits(),
                &mut Recorder::disabled()
            )
            .is_err()
        );
        assert_eq!(original.text(), "aÄb");
    }
    assert!(
        prepare_attachments(
            &original,
            &[edit(0, 1, "wrong", "X")],
            &[range(0, 1)],
            source_limits(),
            limits(),
            &mut Recorder::disabled()
        )
        .is_err()
    );
}

#[test]
fn given_unsorted_multi_edits_then_mapping_uses_original_coordinates_and_is_permutation_invariant()
{
    let original = source("aÄb🙂c");
    let a = edit(0, 1, "a", "XX");
    let b = edit(8, 9, "c", "");
    let placements = [
        range(1, 8),
        point(3, Affinity::After, PointProfile::Position),
    ];
    let first = prepare(&original, &[a.clone(), b.clone()], &placements);
    let second = prepare(&original, &[b, a], &placements);
    assert_eq!(first.source(), second.source());
    assert_eq!(first.placements(), second.placements());
    assert_eq!(first.placements()[0], range(2, 9));
}

#[test]
fn given_fanout_or_mapping_step_exhaustion_then_there_is_no_partial_candidate() {
    let original = source("abc");
    for configured in [
        AttachmentLimits {
            max_attachments: 1,
            max_mapping_steps: 99,
        },
        AttachmentLimits {
            max_attachments: 99,
            max_mapping_steps: 0,
        },
    ] {
        assert_eq!(
            prepare_attachments(
                &original,
                &[edit(0, 0, "", "X")],
                &[range(0, 1), range(1, 2)],
                source_limits(),
                configured,
                &mut Recorder::disabled()
            )
            .unwrap_err(),
            AttachmentError::Limit
        );
        assert_eq!(original.text(), "abc");
    }
}

#[test]
fn given_capture_off_on_or_full_then_candidates_match_and_diagnostics_hold_no_target_or_content() {
    let original = source("PRIVATE_abcd");
    for capacity in [None, Some(32), Some(1)] {
        let mut recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let result = prepare_attachments(
            &original,
            &[edit(8, 9, "a", "X")],
            &[range(8, 10)],
            source_limits(),
            limits(),
            &mut recorder,
        )
        .unwrap();
        assert_eq!(result.source().text(), "PRIVATE_Xbcd");
        assert_eq!(
            result.placements(),
            [Placement::Located {
                target: AttachmentTarget::ContentRange { start: 8, end: 10 },
                evidence: LocationEvidence::Affected
            }]
        );
        assert!(!format!("{result:?}").contains("PRIVATE_"));
        assert!(!format!("{:?}", result.placements()).contains("start"));
        let mut mapped = false;
        while let Some(record) = recorder.pop_front() {
            assert!(!format!("{record:?}").contains("PRIVATE_"));
            if record.boundary == Boundary::AttachmentMapping {
                mapped = true;
                assert_eq!(record.disposition, Disposition::Completed);
            }
        }
        if capacity == Some(32) {
            assert!(mapped);
        }
        if capacity == Some(1) {
            assert!(recorder.dropped() > 0);
        }
    }
}

#[test]
fn given_unicode_batches_then_an_original_byte_origin_oracle_agrees_with_content_survival() {
    fn words(depth: usize, text: String, output: &mut Vec<String>) {
        output.push(text.clone());
        if depth > 0 {
            for scalar in ['a', 'é', '🙂'] {
                words(depth - 1, format!("{text}{scalar}"), output);
            }
        }
    }
    let mut texts = Vec::new();
    words(2, String::new(), &mut texts);
    for text in texts {
        let original = source(&text);
        let boundaries: Vec<_> = text
            .char_indices()
            .map(|(p, _)| p)
            .chain(std::iter::once(text.len()))
            .collect();
        let mut batches = vec![vec![]];
        for &lo in &boundaries {
            for &hi in &boundaries {
                if lo > hi {
                    continue;
                }
                for inserted in ["", "X", "🙂"] {
                    let first = edit(lo, hi, &text[lo..hi], inserted);
                    batches.push(vec![first.clone()]);
                    // Two distinct nonempty adjacent replacements exercise full
                    // target removal without an interval-coverage oracle.
                    if lo < hi {
                        for &end in &boundaries {
                            if end > hi {
                                batches
                                    .push(vec![first.clone(), edit(hi, end, &text[hi..end], "Y")]);
                            }
                        }
                    }
                }
            }
        }
        for edits in batches {
            // Independent forward byte emitter: each unchanged original byte
            // carries its origin, inserted bytes carry none. No interval-union
            // or endpoint-map implementation is reused here.
            let mut bytes = Vec::new();
            let mut origins = Vec::new();
            let mut cursor = 0;
            for edit in &edits {
                for p in cursor..edit.start {
                    bytes.push(text.as_bytes()[p]);
                    origins.push(Some(p));
                }
                for byte in edit.inserted.bytes() {
                    bytes.push(byte);
                    origins.push(None);
                }
                cursor = edit.end;
            }
            for p in cursor..text.len() {
                bytes.push(text.as_bytes()[p]);
                origins.push(Some(p));
            }
            let mut targets = Vec::new();
            for &lo in &boundaries {
                for &hi in &boundaries {
                    if lo < hi {
                        targets.push(range(lo, hi));
                    }
                }
                for affinity in [Affinity::Before, Affinity::After] {
                    for profile in [PointProfile::Position, PointProfile::Content] {
                        targets.push(point(lo, affinity, profile));
                    }
                }
            }
            let result = prepare(&original, &edits, &targets);
            assert_eq!(result.source().text().as_bytes(), bytes);
            for (old, mapped) in targets.iter().zip(result.placements()) {
                if let Placement::Located {
                    target: AttachmentTarget::ContentRange { start, end },
                    ..
                } = old
                {
                    let survives = origins.iter().flatten().any(|p| start <= p && p < end);
                    assert_eq!(*mapped == Placement::Detached, !survives);
                }
                if let Placement::Located { target, .. } = mapped {
                    let (lo, hi) = match target {
                        AttachmentTarget::Document => continue,
                        AttachmentTarget::Point { offset, .. } => (*offset, *offset),
                        AttachmentTarget::ContentRange { start, end } => {
                            assert!(start < end);
                            (*start, *end)
                        }
                    };
                    assert!(lo <= hi && hi <= bytes.len());
                    assert!(result.source().text().is_char_boundary(lo));
                    assert!(result.source().text().is_char_boundary(hi));
                }
            }
        }
    }
}

#[test]
fn given_mapping_summary_then_cost_and_each_evidence_kind_are_explicit_not_default_labels() {
    let original = source("abc");
    let document = Placement::Located {
        target: AttachmentTarget::Document,
        evidence: LocationEvidence::Located,
    };
    let result = prepare(
        &original,
        &[edit(1, 2, "b", "X")],
        &[document, range(0, 3), range(1, 2), Placement::Unavailable],
    );
    let usage = result.usage();
    assert_eq!(
        (
            usage.located,
            usage.affected,
            usage.detached,
            usage.unavailable
        ),
        (1, 1, 1, 1)
    );
    assert!(usage.mapping_steps > 0 && usage.mapping_steps <= usage.max_mapping_steps);
}
