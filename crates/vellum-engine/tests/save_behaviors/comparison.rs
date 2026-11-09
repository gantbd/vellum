use super::*;
use vellum_engine::resource::SourceComparisonBudgetV1;
fn budget(encoded: usize, combined: usize) -> SourceComparisonBudgetV1 {
    SourceComparisonBudgetV1 {
        max_encoded_bytes: encoded,
        max_combined_bytes: combined,
    }
}

fn load(
    o: &mut SourceCoordinator,
    bytes: &[u8],
    r: &mut Recorder,
) -> vellum_engine::coordinator::DocumentVersion {
    let port = o
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            r,
        )
        .unwrap();
    o.load_resource(port.read_result(b"key", b"v0", bytes).unwrap(), r)
        .unwrap()
}

#[test]
fn given_bom_and_multibyte_mixed_newlines_then_budget_counts_exact_encoded_octets() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let bytes = b"\xef\xbb\xbf\xc3\xa9\r\n\rX\n";
    let d = load(&mut o, bytes, &mut r);
    assert_eq!(
        o.saved_state_with_budget(&d.document, budget(bytes.len(), bytes.len() * 2), &mut r)
            .unwrap()
            .content,
        ContentComparison::Unchanged
    );
    for b in [
        budget(bytes.len() - 1, bytes.len() * 2),
        budget(bytes.len(), bytes.len() * 2 - 1),
    ] {
        r = Recorder::try_with_capacity(8).unwrap();
        assert_eq!(
            o.saved_state_with_budget(&d.document, b, &mut r),
            Err(ResourceError::Limit)
        );
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].boundary,
            vellum_engine::instrumentation::Boundary::Query
        );
        assert_eq!(
            events[0].disposition,
            vellum_engine::instrumentation::Disposition::Refused
        );
    }
    assert_eq!(
        o.read(&d.document, &mut r).unwrap().source.text(),
        "é\r\n\rX\n"
    );
}

#[test]
fn given_equal_length_change_and_undo_then_exact_octets_not_lengths_or_revision_identity_decide_equality()
 {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let d = load(&mut o, b"abc", &mut r);
    let p = o
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![Replacement {
                    start: 2,
                    end: 3,
                    expected: "c".into(),
                    inserted: "d".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    let out = o.commit(p, &mut r).unwrap();
    assert_eq!(
        o.saved_state_with_budget(&d.document, budget(3, 6), &mut r)
            .unwrap()
            .content,
        ContentComparison::Modified
    );
    let undo = o
        .prepare_reversal(
            &out.history_group.unwrap(),
            vellum_engine::coordinator::HistoryDirection::Undo,
            &mut r,
        )
        .unwrap();
    o.commit(undo, &mut r).unwrap();
    let current = o.read(&d.document, &mut r).unwrap().version;
    assert!(current.revision > d.revision);
    let usage = o.resource_usage();
    let generation = o.generation();
    assert_eq!(
        o.saved_state_with_budget(&d.document, budget(3, 6), &mut r)
            .unwrap()
            .content,
        ContentComparison::Unchanged
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(o.resource_usage(), usage);
}

#[test]
fn given_compatibility_query_then_configured_blob_ceiling_is_not_an_unbounded_encoding_escape() {
    let mut o = owner_with_annotations_and_blob(None, 1);
    let mut r = Recorder::disabled();
    let d = load(&mut o, b"A", &mut r);
    let p = o
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![Replacement {
                    start: 1,
                    end: 1,
                    expected: "".into(),
                    inserted: "A".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(p, &mut r).unwrap();
    let usage = o.resource_usage();
    let generation = o.generation();
    r = Recorder::try_with_capacity(8).unwrap();
    assert_eq!(
        o.saved_state(&d.document, &mut r),
        Err(ResourceError::Limit)
    );
    assert_eq!(r.queued(), 1); // No SourceEncode marker: refused before copying.
    assert_eq!(
        o.saved_state_with_budget(&d.document, budget(2, 3), &mut r)
            .unwrap()
            .content,
        ContentComparison::Modified
    );
    assert_eq!(o.resource_usage(), usage);
    assert_eq!(o.generation(), generation);
    assert!(!o.is_faulted());
}

#[test]
fn given_empty_or_unbound_output_and_ambiguous_bom_then_bounds_never_guess_or_repair_content() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let scratch = o.create(b"nonempty scratch", &mut r).unwrap();
    assert_eq!(
        o.saved_state_with_budget(&scratch.document, budget(0, 0), &mut r)
            .unwrap()
            .content,
        ContentComparison::Unbound
    );
    let d = load(&mut o, b"", &mut r);
    assert_eq!(
        o.saved_state_with_budget(&d.document, budget(0, 0), &mut r)
            .unwrap()
            .content,
        ContentComparison::Unchanged
    );
    let p = o
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 0,
                    expected: "".into(),
                    inserted: "\u{feff}".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(p, &mut r).unwrap();
    assert_eq!(
        o.saved_state_with_budget(&d.document, budget(3, 3), &mut r),
        Err(ResourceError::Source(
            vellum_engine::source::SourceError::AmbiguousBom
        ))
    );
    assert_eq!(
        o.read(&d.document, &mut r).unwrap().source.text(),
        "\u{feff}"
    );
    assert!(!o.is_faulted());
}

#[test]
fn given_disabled_available_or_full_capture_then_budget_refusal_and_exact_comparison_are_identical()
{
    use vellum_engine::instrumentation::{Boundary, Disposition};
    for capacity in [None, Some(8), Some(1)] {
        let mut o = owner();
        let mut r = Recorder::disabled();
        let d = load(&mut o, b"PRIVATE_COMPARE", &mut r);
        let usage = o.resource_usage();
        let generation = o.generation();
        r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        assert_eq!(
            o.saved_state_with_budget(&d.document, budget(13, 28), &mut r),
            Err(ResourceError::Limit)
        );
        assert_eq!(
            o.saved_state_with_budget(&d.document, budget(15, 30), &mut r)
                .unwrap()
                .content,
            ContentComparison::Unchanged
        );
        assert_eq!(o.resource_usage(), usage);
        assert_eq!(o.generation(), generation);
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        assert!(!format!("{events:?}").contains("PRIVATE_COMPARE"));
        if capacity == Some(8) {
            assert_eq!(
                events
                    .iter()
                    .map(|v| (v.boundary, v.disposition))
                    .collect::<Vec<_>>(),
                vec![
                    (Boundary::Query, Disposition::Refused),
                    (Boundary::SourceEncode, Disposition::Completed),
                    (Boundary::Query, Disposition::Completed)
                ]
            );
        }
        if capacity == Some(1) {
            assert_eq!(r.dropped(), 2);
        }
    }
}

#[test]
fn given_a_loaded_baseline_then_output_comparison_obeys_both_bounds_without_spending_writer_state()
{
    let mut o = owner();
    let mut r = Recorder::disabled();
    let port = o
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut r,
        )
        .unwrap();
    let d = o
        .load_resource(port.read_result(b"key", b"v0", b"abc").unwrap(), &mut r)
        .unwrap();
    let usage = o.resource_usage();
    let generation = o.generation();
    assert_eq!(
        o.saved_state_with_budget(&d.document, budget(3, 6), &mut r)
            .unwrap()
            .content,
        ContentComparison::Unchanged
    );
    for b in [budget(2, 6), budget(3, 5)] {
        assert_eq!(
            o.saved_state_with_budget(&d.document, b, &mut r),
            Err(ResourceError::Limit)
        );
    }
    for _ in 0..40 {
        assert_eq!(
            o.saved_state_with_budget(&d.document, budget(4, 7), &mut r)
                .unwrap()
                .content,
            ContentComparison::Unchanged
        );
    }
    assert_eq!(o.resource_usage(), usage);
    assert_eq!(o.generation(), generation);
    assert_eq!(o.read(&d.document, &mut r).unwrap().version, d);
    assert!(!o.is_faulted());
}
