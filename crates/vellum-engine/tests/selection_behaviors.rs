use vellum_engine::attachment::Affinity;
use vellum_engine::coordinates::BytePosition;
use vellum_engine::selection::*;
use vellum_engine::source::Utf8Source;
fn s(anchor: usize, head: usize, affinity: Affinity) -> Selection {
    Selection {
        anchor: BytePosition(anchor),
        head: BytePosition(head),
        affinity,
    }
}
fn budget() -> SelectionBudget {
    SelectionBudget {
        max_input_selections: 64,
        max_output_selections: 64,
        max_work: 65536,
    }
}
#[test]
fn overlapping_ranges_merge_but_adjacency_and_explicit_carets_remain_separate() {
    let source = Utf8Source::decode(b"abcdef", 64).unwrap();
    let input = [
        s(0, 3, Affinity::After),
        s(2, 4, Affinity::Before),
        s(4, 6, Affinity::Before),
        s(2, 2, Affinity::Before),
        s(4, 4, Affinity::After),
    ];
    let out = normalize_selections(&source, &input, 1, budget()).unwrap();
    assert_eq!(
        out.selections(),
        &[
            s(0, 4, Affinity::Before),
            s(2, 2, Affinity::Before),
            s(4, 4, Affinity::After),
            s(4, 6, Affinity::Before)
        ]
    );
    assert_eq!(out.primary(), 0);
}
#[test]
fn merged_primary_uses_its_direction_and_other_unions_use_earliest_listed_direction() {
    let source = Utf8Source::decode(b"abcdef", 64).unwrap();
    let input = [
        s(5, 2, Affinity::After),
        s(0, 3, Affinity::Before),
        s(6, 6, Affinity::After),
    ];
    let out = normalize_selections(&source, &input, 2, budget()).unwrap();
    assert_eq!(
        out.selections(),
        &[s(5, 0, Affinity::After), s(6, 6, Affinity::After)]
    );
    assert_eq!(out.primary(), 1);
    let out = normalize_selections(&source, &input, 1, budget()).unwrap();
    assert_eq!(out.selections()[0], s(0, 5, Affinity::Before));
    assert_eq!(out.primary(), 0);
}
#[test]
fn only_identical_same_affinity_carets_deduplicate_and_primary_tracks_the_survivor() {
    let source = Utf8Source::decode(b"abc", 64).unwrap();
    let input = [
        s(1, 1, Affinity::After),
        s(1, 1, Affinity::Before),
        s(1, 1, Affinity::After),
    ];
    let out = normalize_selections(&source, &input, 2, budget()).unwrap();
    assert_eq!(
        out.selections(),
        &[s(1, 1, Affinity::Before), s(1, 1, Affinity::After)]
    );
    assert_eq!(out.primary(), 1);
}
#[test]
fn invalid_boundaries_empty_input_primary_and_work_capacity_refuse_wholly() {
    let source = Utf8Source::decode("é".as_bytes(), 64).unwrap();
    assert_eq!(
        normalize_selections(&source, &[], 0, budget()),
        Err(SelectionError::InvalidSelection)
    );
    assert_eq!(
        normalize_selections(&source, &[s(0, 0, Affinity::After)], 1, budget()),
        Err(SelectionError::InvalidSelection)
    );
    assert_eq!(
        normalize_selections(&source, &[s(1, 1, Affinity::After)], 0, budget()),
        Err(SelectionError::InvalidBoundary)
    );
    assert_eq!(
        normalize_selections(
            &source,
            &[s(0, 2, Affinity::After)],
            0,
            SelectionBudget {
                max_work: 0,
                ..budget()
            }
        ),
        Err(SelectionError::Limit)
    );
    assert_eq!(
        normalize_selections(
            &source,
            &[s(0, 0, Affinity::After)],
            0,
            SelectionBudget {
                max_output_selections: 0,
                ..budget()
            }
        ),
        Err(SelectionError::Limit)
    );
}
#[test]
fn normalized_sets_are_idempotent_and_every_primary_survives_small_input_combinations() {
    let source = Utf8Source::decode(b"abcd", 64).unwrap();
    let choices = [
        s(0, 2, Affinity::After),
        s(3, 1, Affinity::Before),
        s(2, 4, Affinity::Before),
        s(2, 2, Affinity::After),
        s(2, 2, Affinity::Before),
    ];
    for a in choices {
        for b in choices {
            for c in choices {
                for primary in 0..3 {
                    let input = [a, b, c];
                    let out = normalize_selections(&source, &input, primary, budget()).unwrap();
                    assert!(out.primary() < out.selections().len());
                    assert_eq!(
                        normalize_selections(&source, out.selections(), out.primary(), budget())
                            .unwrap(),
                        out
                    );
                    let picked = input[primary];
                    let selected = out.selections()[out.primary()];
                    assert!(
                        selected.anchor.0.min(selected.head.0)
                            <= picked.anchor.0.min(picked.head.0)
                    );
                    assert!(
                        selected.anchor.0.max(selected.head.0)
                            >= picked.anchor.0.max(picked.head.0)
                    );
                    assert_eq!(selected.affinity, picked.affinity);
                    assert!(!format!("{out:?}").contains("anchor"));
                }
            }
        }
    }
}
