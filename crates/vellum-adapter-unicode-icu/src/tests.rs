use super::*;

fn limits() -> Limits {
    Limits {
        max_source_bytes: 65536,
        max_boundaries: 262144,
        max_words: 65536,
        max_logical_bytes: 4 * 1024 * 1024,
    }
}
fn provider() -> Provider {
    Provider::new(limits(), Timing::Off, &mut Recorder::disabled()).unwrap()
}
fn points(values: &[BytePosition]) -> Vec<usize> {
    values.iter().map(|p| p.0).collect()
}

#[test]
fn raw_unicode_bom_surrogates_combining_and_mixed_endings_are_not_normalized() {
    let mut p = provider();
    let empty = p.segment("", &mut Recorder::disabled()).unwrap();
    assert_eq!(points(&empty.graphemes), [0]);
    assert_eq!(points(&empty.words), [0]);
    let table = p
        .segment("\u{feff}é🇺🇸e\u{301}\r\n", &mut Recorder::disabled())
        .unwrap();
    assert_eq!(points(&table.graphemes), [0, 3, 5, 13, 16, 18]);
    assert_eq!(table.profile, BoundaryProfileV1::Unicode18Uax29Revision49);
    assert_eq!(
        table.significant_words,
        [
            ByteRange { start: 3, end: 5 },
            ByteRange { start: 13, end: 16 }
        ]
    );
    assert!(!format!("{table:?} {p:?}").contains("🇺🇸"));
    let nul = p.segment("a\0b", &mut Recorder::disabled()).unwrap();
    assert_eq!(points(&nul.graphemes), [0, 1, 2, 3]);
    assert_eq!(
        nul.significant_words,
        [
            ByteRange { start: 0, end: 1 },
            ByteRange { start: 2, end: 3 }
        ]
    );
}

#[test]
fn raw_default_words_are_not_dictionary_or_significant_word_grids() {
    let mut p = provider();
    let thai = p.segment("ภาษาไทย", &mut Recorder::disabled()).unwrap();
    assert_eq!(points(&thai.words), [0, 3, 6, 9, 12, 15, 18, 21]);
    let words = p.segment("A 12 !", &mut Recorder::disabled()).unwrap();
    assert_eq!(
        words.significant_words,
        [
            ByteRange { start: 0, end: 1 },
            ByteRange { start: 2, end: 4 }
        ]
    );
    // Alphabetic includes some combining marks; numeric includes Nl and No, not
    // just ASCII digits or a Rust compiler's potentially older Unicode tables.
    for text in ["\u{345}", "Ⅷ", "½"] {
        assert_eq!(
            p.segment(text, &mut Recorder::disabled())
                .unwrap()
                .significant_words,
            [ByteRange {
                start: 0,
                end: text.len()
            }]
        );
    }
}

#[test]
fn invalid_input_and_logical_capacity_refuse_before_native_admission() {
    let mut l = limits();
    l.max_source_bytes = 0;
    l.max_logical_bytes = 16;
    let mut p = Provider::new(l, Timing::Off, &mut Recorder::disabled()).unwrap();
    p.segment("", &mut Recorder::disabled()).unwrap();
    assert_eq!(
        p.segment("a", &mut Recorder::disabled()).unwrap_err(),
        Error::InputLimit
    );
    assert_eq!(
        p.usage(),
        Usage {
            offered: 2,
            admitted: 1,
            completed: 1,
            refused: 1,
            limit_refusals: 1,
            ..Usage::default()
        }
    );
    let mut l = limits();
    l.max_logical_bytes = 15;
    let mut p = Provider::new(l, Timing::Off, &mut Recorder::disabled()).unwrap();
    assert_eq!(
        p.segment("", &mut Recorder::disabled()).unwrap_err(),
        Error::LogicalLimit
    );
    assert_eq!(p.usage().admitted, 0);
}

#[test]
fn unicode18_revision49_gb9c_extends_the_stable_segmenter_without_overjoining() {
    let mut p = provider();
    for joined in [
        "\u{094d}\u{0915}",
        "\u{094d}\u{0308}\u{0915}",
        "\u{1cf5}\u{0995}",
    ] {
        assert_eq!(
            points(
                &p.segment(joined, &mut Recorder::disabled())
                    .unwrap()
                    .graphemes
            ),
            [0, joined.len()]
        );
    }
    let zwnj = "\u{1cf5}\u{200c}\u{0995}";
    assert_eq!(
        points(
            &p.segment(zwnj, &mut Recorder::disabled())
                .unwrap()
                .graphemes
        ),
        [0, "\u{1cf5}\u{200c}".len(), zwnj.len()]
    );
}

#[test]
fn combined_endpoint_and_significant_word_limits_never_return_partial_tables() {
    let mut l = limits();
    l.max_boundaries = 2;
    l.max_words = 0;
    let mut p = Provider::new(l, Timing::Off, &mut Recorder::disabled()).unwrap();
    p.segment("", &mut Recorder::disabled()).unwrap();
    assert_eq!(
        p.segment("a", &mut Recorder::disabled()).unwrap_err(),
        Error::OutputLimit
    );
    let mut l = limits();
    l.max_words = 0;
    let mut p = Provider::new(l, Timing::Off, &mut Recorder::disabled()).unwrap();
    assert_eq!(
        p.segment("a", &mut Recorder::disabled()).unwrap_err(),
        Error::OutputLimit
    );
    assert_eq!(p.usage().native_refusals, 0);
    // No retained source or poisoned incremental state: a later independent
    // request still succeeds under the same finite profile.
    assert!(p.segment("!", &mut Recorder::disabled()).is_ok());
}

#[test]
fn capture_disabled_timed_untimed_and_saturated_are_behaviorally_identical() {
    let mut expected = None;
    for timing in [Timing::Off, Timing::ProcessService] {
        for capacity in [None, Some(8), Some(1)] {
            let mut r = capacity.map_or_else(Recorder::disabled, |n| {
                Recorder::try_with_capacity(n).unwrap()
            });
            let mut p = Provider::new(limits(), timing, &mut r).unwrap();
            let table = p.segment("A 12 !\r\n🇺🇸", &mut r).unwrap();
            if let Some(ref original) = expected {
                assert_eq!(&table, original);
            } else {
                expected = Some(table);
            }
            assert_eq!(p.usage().completed, 1);
            assert_eq!(p.usage().refused, 0);
            assert_eq!(r.dropped(), if capacity == Some(1) { 2 } else { 0 });
            while let Some(event) = r.pop_front() {
                assert_eq!(
                    event.service_time.is_some(),
                    timing == Timing::ProcessService
                );
            }
        }
    }
}

#[test]
fn full_reference_source_budget_is_supported_and_counters_do_not_wrap() {
    let mut p = provider();
    let text = "a".repeat(65536);
    let table = p.segment(&text, &mut Recorder::disabled()).unwrap();
    assert_eq!(table.graphemes.len(), 65537);
    assert_eq!(points(&table.words), [0, 65536]);
    p.usage.offered = u64::MAX;
    p.usage.completed = u64::MAX;
    p.segment("", &mut Recorder::disabled()).unwrap();
    assert_eq!(p.usage.offered, u64::MAX);
    assert_eq!(p.usage.completed, u64::MAX);
    assert!(p.usage.overflowed);
}

#[test]
fn malformed_generated_positions_are_refused_instead_of_widened_or_repaired() {
    let mut grid = reserved(4).unwrap();
    assert_eq!(
        push_point(&mut grid, 1, "é", 4),
        Err(Error::InvalidNativeBoundary)
    );
    push_point(&mut grid, 0, "é", 4).unwrap();
    assert_eq!(
        push_point(&mut grid, 0, "é", 4),
        Err(Error::InvalidNativeBoundary)
    );
    assert_eq!(
        push_point(&mut grid, 1, "é", 4),
        Err(Error::InvalidNativeBoundary)
    );
    assert_eq!(push_point(&mut grid, 2, "é", 1), Err(Error::OutputLimit));
}
