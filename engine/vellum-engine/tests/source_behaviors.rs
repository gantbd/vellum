//! Executable Given/When/Then scenarios for source values, not engine admission.
use vellum_engine::source::{Limits, Replacement, SourceError, Utf8Source};

#[test]
fn literal_leading_feff_without_bom_metadata_is_valid_edit_text_but_not_lossless_file_output() {
    let original = Utf8Source::decode(b"A", 4096).unwrap();
    let limits = Limits {
        max_source_bytes: 4096,
        max_edits: 8,
        max_edit_payload_bytes: 4096,
    };
    let edited = original
        .prepare(
            &[Replacement {
                start: 0,
                end: 0,
                expected: "".into(),
                inserted: "\u{feff}".into(),
            }],
            limits,
        )
        .unwrap();
    assert_eq!(edited.text(), "\u{feff}A");
    assert!(edited.encode().is_err());
    assert_eq!(edited.text(), "\u{feff}A");
    let imported = Utf8Source::decode("\u{feff}\u{feff}A".as_bytes(), 4096).unwrap();
    assert_eq!(imported.text(), "\u{feff}A");
    assert_eq!(imported.encode().unwrap(), "\u{feff}\u{feff}A".as_bytes());
}

fn limits() -> Limits {
    Limits {
        max_source_bytes: 1024,
        max_edits: 16,
        max_edit_payload_bytes: 1024,
    }
}

fn edit(start: usize, end: usize, expected: &str, inserted: &str) -> Replacement {
    Replacement {
        start,
        end,
        expected: expected.into(),
        inserted: inserted.into(),
    }
}

#[test]
fn given_valid_utf8_when_roundtripped_then_every_byte_is_preserved() {
    for bytes in [
        b"".as_slice(),
        b"\xef\xbb\xbfEnglish\r\nDeutsch: \xc3\xbc\rtext\n  ",
        "e\u{301}\0\u{feff}終".as_bytes(),
        b"\xef\xbb\xbf\xef\xbb\xbf",
    ] {
        let source = Utf8Source::decode(bytes, 1024).unwrap();
        assert_eq!(source.encode().unwrap(), bytes);
    }
}

#[test]
fn given_invalid_utf8_when_decoded_then_no_lossy_source_is_produced() {
    for bytes in [
        b"\xff".as_slice(),
        b"\xc0\xaf",
        b"\xed\xa0\x80",
        b"\xe2\x82",
    ] {
        assert_eq!(
            Utf8Source::decode(bytes, 1024),
            Err(SourceError::InvalidUtf8)
        );
    }
}

#[test]
fn given_bom_when_editing_then_offsets_address_text_not_serialized_signature() {
    let source = Utf8Source::decode(b"\xef\xbb\xbfhello", 1024).unwrap();
    let candidate = source.prepare(&[edit(0, 1, "h", "H")], limits()).unwrap();
    assert_eq!(candidate.text(), "Hello");
    assert_eq!(candidate.encode().unwrap(), b"\xef\xbb\xbfHello");
    assert_eq!(source.text(), "hello");
}

#[test]
fn given_unsorted_edits_when_prepared_then_all_ranges_use_the_original_snapshot() {
    let source = Utf8Source::decode(b"abcdef", 1024).unwrap();
    let edits = [
        edit(4, 6, "ef", "!"),
        edit(0, 1, "a", "ALPHA"),
        edit(1, 2, "b", "B"),
    ];
    let result = source.prepare(&edits, limits()).unwrap();
    assert_eq!(result.text(), "ALPHABcd!");
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let permutation = order.map(|index| edits[index].clone());
        assert_eq!(source.prepare(&permutation, limits()).unwrap(), result);
    }
    assert_eq!(source.text(), "abcdef");
}

#[test]
fn given_overlap_or_insertion_ties_when_prepared_then_the_whole_candidate_refuses() {
    let source = Utf8Source::decode(b"abcd", 1024).unwrap();
    for edits in [
        vec![edit(0, 2, "ab", "x"), edit(1, 3, "bc", "y")],
        vec![edit(1, 1, "", "x"), edit(1, 1, "", "y")],
        vec![edit(0, 2, "ab", "x"), edit(0, 0, "", "y")],
        vec![edit(0, 2, "ab", "x"), edit(1, 1, "", "y")],
        vec![edit(0, 2, "ab", "x"), edit(2, 2, "", "y")],
    ] {
        assert_eq!(
            source.prepare(&edits, limits()),
            Err(SourceError::AmbiguousEdits)
        );
        assert_eq!(source.text(), "abcd");
    }
}

#[test]
fn given_a_bad_member_when_prepared_then_other_edits_do_not_change_the_source() {
    let source = Utf8Source::decode("aü終".as_bytes(), 1024).unwrap();
    for (bad, error) in [
        (edit(1, 3, "wrong", "u"), SourceError::ExpectedMismatch),
        (edit(1, 2, "", "u"), SourceError::InvalidBoundary),
        (edit(4, 3, "", "u"), SourceError::InvalidRange),
        (edit(6, 7, "", "u"), SourceError::InvalidRange),
    ] {
        assert_eq!(
            source.prepare(&[edit(0, 1, "a", "A"), bad], limits()),
            Err(error)
        );
        assert_eq!(source.text(), "aü終");
    }
}

#[test]
fn given_scalar_boundaries_inside_a_grapheme_when_edited_then_no_rounding_occurs() {
    let source = Utf8Source::decode("e\u{301}".as_bytes(), 1024).unwrap();
    let candidate = source
        .prepare(&[edit(1, 3, "\u{301}", "")], limits())
        .unwrap();
    assert_eq!(candidate.text(), "e");
}

#[test]
fn given_empty_or_equal_replacements_when_prepared_then_source_value_is_equal() {
    let source = Utf8Source::decode(b"abc", 1024).unwrap();
    for edits in [
        vec![],
        vec![edit(0, 3, "abc", "abc")],
        vec![edit(3, 3, "", "")],
    ] {
        assert_eq!(source.prepare(&edits, limits()).unwrap(), source);
    }
    assert_eq!(
        source.prepare(&[edit(3, 3, "wrong", "")], limits()),
        Err(SourceError::ExpectedMismatch)
    );
}

#[test]
fn given_limits_when_exceeded_then_preparation_refuses_without_a_partial_result() {
    let source = Utf8Source::decode(b"abc", 3).unwrap();
    assert_eq!(
        Utf8Source::decode(b"abcd", 3),
        Err(SourceError::SourceLimit)
    );
    assert_eq!(
        source.prepare(
            &[edit(0, 0, "", "x")],
            Limits {
                max_source_bytes: 3,
                ..limits()
            }
        ),
        Err(SourceError::SourceLimit)
    );
    assert_eq!(
        source.prepare(
            &[edit(0, 1, "a", "x")],
            Limits {
                max_edits: 0,
                ..limits()
            }
        ),
        Err(SourceError::EditLimit)
    );
    assert_eq!(
        source.prepare(
            &[edit(0, 1, "a", "x")],
            Limits {
                max_edit_payload_bytes: 1,
                ..limits()
            }
        ),
        Err(SourceError::PayloadLimit)
    );
    assert_eq!(source.text(), "abc");
}

#[test]
fn given_diagnostics_off_on_or_full_when_prepared_then_results_are_identical() {
    use vellum_engine::instrumentation::{Boundary, Disposition, Observation, Recorder};
    let source = Utf8Source::decode(b"secret", 1024).unwrap();
    for edits in [
        vec![edit(0, 6, "secret", "private")],
        vec![edit(0, 6, "wrong", "private")],
    ] {
        let expected = source.prepare(&edits, limits());
        for capacity in [0, 1, 2] {
            let mut recorder = Recorder::try_with_capacity(capacity).unwrap();
            recorder.record(Observation {
                boundary: Boundary::Query,
                disposition: Disposition::Completed,
                service_time: None,
            });
            assert_eq!(
                source.prepare_observed(&edits, limits(), &mut recorder),
                expected
            );
            let retained: Vec<_> = std::iter::from_fn(|| recorder.pop_front()).collect();
            if capacity == 2 {
                assert_eq!(retained[1].boundary, Boundary::Preparation);
                assert_eq!(retained[1].service_time, None);
                assert_eq!(
                    retained[1].disposition,
                    if expected.is_ok() {
                        Disposition::Completed
                    } else {
                        Disposition::Refused
                    }
                );
            }
            let diagnostic = format!("{recorder:?}{retained:?}");
            assert!(!diagnostic.contains("secret"));
            assert!(!diagnostic.contains("private"));
        }
    }
}

#[test]
fn given_source_and_replacements_when_debugged_then_payloads_are_not_disclosed() {
    let source = Utf8Source::decode(b"TOP_SECRET", 1024).unwrap();
    let replacement = edit(0, 10, "TOP_SECRET", "PRIVATE_INSERTION");
    let diagnostic = format!("{source:?} {replacement:?}");
    assert!(!diagnostic.contains("TOP_SECRET"));
    assert!(!diagnostic.contains("PRIVATE_INSERTION"));
}

#[test]
fn given_all_small_pair_geometries_when_prepared_then_an_independent_oracle_agrees() {
    let source = Utf8Source::decode(b"abcd", 1024).unwrap();
    for a in 0..=4 {
        for b in a..=4 {
            for c in 0..=4 {
                for d in c..=4 {
                    let edits = [
                        edit(a, b, &source.text()[a..b], "X"),
                        edit(c, d, &source.text()[c..d], "Y"),
                    ];
                    // Symmetric set geometry, not the production neighbor scan.
                    let ambiguous = if a == b {
                        c <= a && a <= d
                    } else if c == d {
                        a <= c && c <= b
                    } else {
                        a.max(c) < b.min(d)
                    };
                    let actual = source.prepare(&edits, limits());
                    if ambiguous {
                        assert_eq!(actual, Err(SourceError::AmbiguousEdits));
                    } else {
                        // Independently emit insertions at each original site,
                        // retaining a source byte only if neither range removes it.
                        let mut expected = String::new();
                        for site in 0..=4 {
                            if site == a {
                                expected.push('X');
                            }
                            if site == c {
                                expected.push('Y');
                            }
                            if site < 4 && !(a..b).contains(&site) && !(c..d).contains(&site) {
                                expected.push(source.text().as_bytes()[site] as char);
                            }
                        }
                        assert_eq!(actual.unwrap().text(), expected);
                    }
                    assert_eq!(source.text(), "abcd");
                }
            }
        }
    }
}
