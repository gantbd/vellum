//! Native ownership/shape acceptance is conditional on trusted provider semantics.
use vellum_engine::coordinates::BytePosition;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, Recorder};
use vellum_engine::source::{Limits, Replacement};
fn limits() -> BoundaryLimits {
    BoundaryLimits {
        max_issued: 8,
        max_retained: 4,
        max_source_bytes: 1024,
        max_boundaries: 64,
        max_words: 8,
        max_evidence_bytes: 4096,
        max_retained_bytes: 8192,
    }
}
fn fixture(bounds: BoundaryLimits) -> (SourceCoordinator, DocumentVersion, Recorder) {
    let mut owner = SourceCoordinator::with_history_limits(
        CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 16,
                max_edit_payload_bytes: 1024,
            },
        },
        HistoryLimits {
            max_retained_groups: 8,
            max_journal_bytes: 65536,
            max_pins: 8,
        },
    );
    let mut r = Recorder::disabled();
    owner.configure_boundaries(bounds, &mut r).unwrap();
    let source = owner.create("a\u{301} b".as_bytes(), &mut r).unwrap();
    (owner, source, r)
}
fn table() -> BoundaryTableV1 {
    BoundaryTableV1 {
        profile: BoundaryProfileV1::Unicode18Uax29Revision49,
        graphemes: vec![0, 3, 4, 5].into_iter().map(BytePosition).collect(),
        words: vec![0, 3, 4, 5].into_iter().map(BytePosition).collect(),
        significant_words: vec![
            ByteRange { start: 0, end: 3 },
            ByteRange { start: 4, end: 5 },
        ],
    }
}
#[test]
fn capture_and_evidence_bind_exact_source_without_source_or_history_publication() {
    let (mut owner, source, mut r) = fixture(limits());
    let generation = owner.generation();
    let capture = owner.begin_boundary_capture(&source, &mut r).unwrap();
    let input = owner.boundary_source(&capture, &mut r).unwrap();
    assert_eq!(input.version, source);
    assert_eq!(input.text, "a\u{301} b");
    assert!(!format!("{input:?}").contains("a\u{301}"));
    assert_eq!(owner.boundary_usage().pending, 1);
    let mut data = table();
    let evidence = owner
        .accept_boundary_evidence(&capture, &data, &mut r)
        .unwrap();
    data.graphemes.clear();
    assert_eq!(
        owner
            .boundary_evidence(&evidence, &mut r)
            .unwrap()
            .table
            .graphemes
            .len(),
        4
    );
    assert_eq!(owner.boundary_usage().pending, 0);
    assert_eq!(owner.boundary_usage().ready, 1);
    assert_eq!(owner.generation(), generation);
    assert_eq!(owner.history_usage().retained_groups, 0);
    assert_eq!(
        owner.boundary_source(&capture, &mut r).err(),
        Some(BoundaryError::Unavailable)
    );
    assert_eq!(
        owner.accept_boundary_evidence(&capture, &table(), &mut r),
        Err(BoundaryError::Unavailable)
    );
    assert!(
        !format!("{:?}", owner.boundary_evidence(&evidence, &mut r).unwrap())
            .contains("BytePosition")
    );
}
#[test]
fn malformed_vectors_and_word_intervals_refuse_wholly_but_do_not_consume_capture() {
    let (mut owner, source, mut r) = fixture(limits());
    let capture = owner.begin_boundary_capture(&source, &mut r).unwrap();
    let before = owner.boundary_usage();
    for mode in 0..8 {
        let mut bad = table();
        match mode {
            0 => {
                bad.graphemes.remove(0);
            }
            1 => {
                bad.words.pop();
            }
            2 => bad.graphemes[1] = BytePosition(2),
            3 => bad.words[2] = BytePosition(3),
            4 => bad.significant_words[0] = ByteRange { start: 0, end: 4 },
            5 => bad.significant_words.reverse(),
            6 => bad.significant_words[0] = ByteRange { start: 3, end: 3 },
            _ => bad.graphemes.swap(1, 2),
        }
        assert_eq!(
            owner.accept_boundary_evidence(&capture, &bad, &mut r),
            Err(BoundaryError::InvalidTable)
        );
        assert_eq!(owner.boundary_usage(), before);
    }
    owner
        .accept_boundary_evidence(&capture, &table(), &mut r)
        .unwrap();
}
#[test]
fn raw_and_significant_word_data_remain_exact_without_a_global_grapheme_subset_claim() {
    let (mut owner, _, mut r) = fixture(limits());
    let source = owner.create("a\u{301}".as_bytes(), &mut r).unwrap();
    let capture = owner.begin_boundary_capture(&source, &mut r).unwrap();
    let mut data = BoundaryTableV1 {
        profile: BoundaryProfileV1::Unicode18Uax29Revision49,
        graphemes: vec![BytePosition(0), BytePosition(3)],
        words: vec![BytePosition(0), BytePosition(1), BytePosition(3)],
        significant_words: vec![],
    };
    data.significant_words.push(ByteRange { start: 0, end: 1 });
    let evidence = owner
        .accept_boundary_evidence(&capture, &data, &mut r)
        .unwrap();
    assert_eq!(
        owner
            .boundary_evidence(&evidence, &mut r)
            .unwrap()
            .table
            .significant_words,
        vec![ByteRange { start: 0, end: 1 }]
    );
    data.significant_words.clear();
}
#[test]
fn source_change_and_byte_restoration_do_not_revive_old_evidence_and_cleanup_survives_disposal() {
    let (mut owner, source, mut r) = fixture(limits());
    let pending = owner.begin_boundary_capture(&source, &mut r).unwrap();
    let capture = owner.begin_boundary_capture(&source, &mut r).unwrap();
    let ready = owner
        .accept_boundary_evidence(&capture, &table(), &mut r)
        .unwrap();
    let plan = owner
        .prepare(
            &[DocumentEdits {
                expected: source.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 0,
                    expected: String::new(),
                    inserted: "x".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    let outcome = owner.commit(plan, &mut r).unwrap();
    assert_eq!(
        owner.boundary_source(&pending, &mut r).err(),
        Some(BoundaryError::Stale)
    );
    assert_eq!(
        owner.accept_boundary_evidence(&pending, &table(), &mut r),
        Err(BoundaryError::Stale)
    );
    assert_eq!(
        owner.boundary_evidence(&ready, &mut r).err(),
        Some(BoundaryError::Stale)
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
        "a\u{301} b"
    );
    assert_eq!(
        owner.boundary_evidence(&ready, &mut r).err(),
        Some(BoundaryError::Stale)
    );
    let current = owner.read(&source.document, &mut r).unwrap().version;
    let disposal = owner
        .prepare_retained_disposal(&current, 0, &mut r)
        .unwrap();
    owner.commit_retained_disposal(disposal, &mut r).unwrap();
    owner.retire_boundary_capture(&pending, &mut r).unwrap();
    owner.retire_boundary_evidence(&ready, &mut r).unwrap();
    assert_eq!(owner.boundary_usage().retained_bytes, 0);
    assert_eq!(owner.boundary_usage().issued_lifetime, 2);
}
#[test]
fn bounds_and_foreign_or_retired_identities_cannot_create_unaccounted_evidence() {
    for bounds in [
        BoundaryLimits {
            max_issued: 0,
            ..limits()
        },
        BoundaryLimits {
            max_retained: 0,
            ..limits()
        },
        BoundaryLimits {
            max_source_bytes: 0,
            ..limits()
        },
        BoundaryLimits {
            max_evidence_bytes: 0,
            ..limits()
        },
        BoundaryLimits {
            max_retained_bytes: 0,
            ..limits()
        },
    ] {
        let (mut owner, source, mut r) = fixture(bounds);
        assert_eq!(
            owner.begin_boundary_capture(&source, &mut r),
            Err(BoundaryError::Limit)
        );
        assert_eq!(owner.boundary_usage().issued_lifetime, 0);
    }
    let (mut owner, source, mut r) = fixture(BoundaryLimits {
        max_issued: 1,
        max_retained: 1,
        ..limits()
    });
    let capture = owner.begin_boundary_capture(&source, &mut r).unwrap();
    let (mut other, other_source, _) = fixture(limits());
    assert_eq!(
        other.boundary_source(&capture, &mut r).err(),
        Some(BoundaryError::Unavailable)
    );
    let foreign = other.begin_boundary_capture(&other_source, &mut r).unwrap();
    assert_ne!(capture, foreign);
    owner.retire_boundary_capture(&capture, &mut r).unwrap();
    assert_eq!(
        owner.begin_boundary_capture(&source, &mut r),
        Err(BoundaryError::Limit)
    );
    assert_eq!(
        owner.accept_boundary_evidence(&capture, &table(), &mut r),
        Err(BoundaryError::Unavailable)
    );
}
#[test]
fn optional_capture_and_saturation_do_not_change_evidence_or_retirement() {
    for capacity in [None, Some(64), Some(1)] {
        let (mut owner, source, _) = fixture(limits());
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let capture = owner.begin_boundary_capture(&source, &mut r).unwrap();
        let evidence = owner
            .accept_boundary_evidence(&capture, &table(), &mut r)
            .unwrap();
        let clone = evidence.clone();
        assert_eq!(
            owner.boundary_evidence(&clone, &mut r).unwrap().version,
            source
        );
        owner.retire_boundary_evidence(&evidence, &mut r).unwrap();
        assert_eq!(
            owner.boundary_evidence(&clone, &mut r).err(),
            Some(BoundaryError::Unavailable)
        );
        assert_eq!(owner.boundary_usage().retained_bytes, 0);
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(64) {
            assert!(
                records
                    .iter()
                    .any(|o| o.boundary == Boundary::BoundaryRetention)
            );
        }
    }
}
