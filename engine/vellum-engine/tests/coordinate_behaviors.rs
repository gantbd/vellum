//! Logical coordinates are source projections, never file or display positions.
use vellum_engine::coordinates::*;
use vellum_engine::source::Utf8Source;
fn source(text: &str) -> Utf8Source {
    Utf8Source::decode(text.as_bytes(), 65536).unwrap()
}
fn budget() -> CoordinateBudget {
    CoordinateBudget {
        max_scan_bytes: 65536,
    }
}
// Independent scalar iterator oracle; production scans canonical byte prefixes.
fn line_oracle(text: &str) -> Vec<LineSpan> {
    let mut chars = text.char_indices().peekable();
    let mut spans = Vec::new();
    let mut start = 0;
    while let Some((offset, scalar)) = chars.next() {
        if matches!(scalar, '\r' | '\n') {
            let mut end = offset + scalar.len_utf8();
            if scalar == '\r' && chars.peek().is_some_and(|(_, next)| *next == '\n') {
                end = chars.next().unwrap().0 + 1;
            }
            spans.push(LineSpan {
                start: BytePosition(start),
                body_end: BytePosition(offset),
                end: BytePosition(end),
            });
            start = end;
        }
    }
    spans.push(LineSpan {
        start: BytePosition(start),
        body_end: BytePosition(text.len()),
        end: BytePosition(text.len()),
    });
    spans
}

#[test]
fn logical_lines_partition_exact_mixed_delimiters_and_keep_the_trailing_empty_line() {
    let source = source("A\r\nβ\rC\n");
    let coordinates = SourceCoordinates::new(&source);
    let expected = [(0, 1, 3), (3, 5, 6), (6, 7, 8), (8, 8, 8)];
    for (line, (start, body_end, end)) in expected.into_iter().enumerate() {
        assert_eq!(
            coordinates.line_span(line, budget()),
            Ok(LineSpan {
                start: BytePosition(start),
                body_end: BytePosition(body_end),
                end: BytePosition(end)
            })
        );
    }
    assert_eq!(
        coordinates.line_span(4, budget()),
        Err(CoordinateError::InvalidLine)
    );
    let empty = Utf8Source::decode(b"", 0).unwrap();
    assert_eq!(
        SourceCoordinates::new(&empty)
            .line_span(0, CoordinateBudget { max_scan_bytes: 0 })
            .unwrap(),
        LineSpan {
            start: BytePosition(0),
            body_end: BytePosition(0),
            end: BytePosition(0)
        }
    );
}

#[test]
fn line_columns_refuse_scalar_splits_delimiter_interiors_and_out_of_range_without_rounding() {
    let source = source("é\r\nX");
    let c = SourceCoordinates::new(&source);
    assert_eq!(
        c.byte_to_line(BytePosition(2), budget()),
        Ok(LinePosition {
            line: 0,
            body_byte_column: 2
        })
    );
    assert_eq!(
        c.byte_to_line(BytePosition(3), budget()),
        Err(CoordinateError::UnavailableProjection)
    );
    assert_eq!(
        c.byte_to_line(BytePosition(1), budget()),
        Err(CoordinateError::InvalidBoundary)
    );
    assert_eq!(
        c.line_to_byte(
            LinePosition {
                line: 1,
                body_byte_column: 0
            },
            budget()
        ),
        Ok(BytePosition(4))
    );
    for position in [
        LinePosition {
            line: 0,
            body_byte_column: 3,
        },
        LinePosition {
            line: 1,
            body_byte_column: 2,
        },
    ] {
        assert_eq!(
            c.line_to_byte(position, budget()),
            Err(CoordinateError::InvalidColumn)
        );
    }
    assert_eq!(
        c.line_to_byte(
            LinePosition {
                line: 0,
                body_byte_column: 1
            },
            budget()
        ),
        Err(CoordinateError::InvalidBoundary)
    );
    assert_eq!(
        c.byte_to_line(BytePosition(5), budget()),
        Ok(LinePosition {
            line: 1,
            body_byte_column: 1
        })
    );
}

#[test]
fn scalar_and_utf16_native_indices_are_distinct_and_surrogate_interiors_refuse() {
    let source = source("A😀e\u{301}\r\n");
    let c = SourceCoordinates::new(&source);
    let positions = [
        (0, 0, 0),
        (1, 1, 1),
        (5, 2, 3),
        (6, 3, 4),
        (8, 4, 5),
        (9, 5, 6),
        (10, 6, 7),
    ];
    for (byte, scalar, utf16) in positions {
        assert_eq!(
            c.byte_to_scalar(BytePosition(byte), budget()),
            Ok(ScalarIndex(scalar))
        );
        assert_eq!(
            c.scalar_to_byte(ScalarIndex(scalar), budget()),
            Ok(BytePosition(byte))
        );
        assert_eq!(
            c.byte_to_utf16(BytePosition(byte), budget()),
            Ok(Utf16Index(utf16))
        );
        assert_eq!(
            c.utf16_to_byte(Utf16Index(utf16), budget()),
            Ok(BytePosition(byte))
        );
    }
    assert_eq!(
        c.utf16_to_byte(Utf16Index(2), budget()),
        Err(CoordinateError::InvalidBoundary)
    );
    assert_eq!(
        c.scalar_to_byte(ScalarIndex(7), budget()),
        Err(CoordinateError::InvalidScalar)
    );
    assert_eq!(
        c.utf16_to_byte(Utf16Index(8), budget()),
        Err(CoordinateError::InvalidUtf16)
    );
    // CRLF interior remains a valid canonical/native-index position.
    assert_eq!(
        c.byte_to_line(BytePosition(9), budget()),
        Err(CoordinateError::UnavailableProjection)
    );
}

#[test]
fn non_profile_unicode_separators_combining_marks_tabs_and_bom_metadata_do_not_become_lines_or_columns()
 {
    let source = source("\u{feff}A\u{85}\u{2028}\u{2029}\t");
    let c = SourceCoordinates::new(&source);
    assert_eq!(source.text(), "A\u{85}\u{2028}\u{2029}\t");
    assert_eq!(
        c.line_span(0, budget()).unwrap().end,
        BytePosition(source.text().len())
    );
    assert_eq!(c.line_span(1, budget()), Err(CoordinateError::InvalidLine));
    let original = source.encode().unwrap();
    c.byte_to_scalar(BytePosition(source.text().len()), budget())
        .unwrap();
    assert_eq!(source.encode().unwrap(), original);
    let literal = Utf8Source::decode("\u{feff}\u{feff}x".as_bytes(), 64).unwrap();
    assert_eq!(
        SourceCoordinates::new(&literal).byte_to_scalar(BytePosition(3), budget()),
        Ok(ScalarIndex(1))
    );
}

#[test]
fn exact_scan_bounds_refuse_without_a_partial_projection_and_simple_prefix_queries_do_not_scan_the_suffix()
 {
    let source = source("A\r\nlong suffix");
    let c = SourceCoordinates::new(&source);
    assert_eq!(
        c.byte_to_line(BytePosition(0), CoordinateBudget { max_scan_bytes: 0 }),
        Ok(LinePosition {
            line: 0,
            body_byte_column: 0
        })
    );
    assert_eq!(
        c.line_span(0, CoordinateBudget { max_scan_bytes: 2 }),
        Err(CoordinateError::Limit)
    );
    assert_eq!(
        c.line_span(0, CoordinateBudget { max_scan_bytes: 3 })
            .unwrap()
            .end,
        BytePosition(3)
    );
    assert_eq!(
        c.line_to_byte(
            LinePosition {
                line: 1,
                body_byte_column: 0
            },
            CoordinateBudget { max_scan_bytes: 3 }
        ),
        Ok(BytePosition(3))
    );
    assert_eq!(
        c.scalar_to_byte(ScalarIndex(3), CoordinateBudget { max_scan_bytes: 2 }),
        Err(CoordinateError::Limit)
    );
    assert_eq!(
        c.byte_to_utf16(BytePosition(3), CoordinateBudget { max_scan_bytes: 3 }),
        Ok(Utf16Index(3))
    );
}

#[test]
fn generated_small_sources_obey_native_inverse_and_logical_line_partition_laws() {
    let alphabet = ["a", "é", "😀", "\r", "\n", "\u{301}"];
    for a in alphabet {
        for b in alphabet {
            for d in alphabet {
                let text = format!("{a}{b}{d}");
                let source = source(&text);
                let c = SourceCoordinates::new(&source);
                let oracle = line_oracle(&text);
                let mut previous_end = 0;
                for (line, expected) in oracle.iter().enumerate() {
                    let span = c.line_span(line, budget()).unwrap();
                    assert_eq!(&span, expected);
                    assert_eq!(span.start.0, previous_end);
                    assert!(span.start.0 <= span.body_end.0 && span.body_end.0 <= span.end.0);
                    let delimiter = &text[span.body_end.0..span.end.0];
                    assert!(["", "\r", "\n", "\r\n"].contains(&delimiter));
                    previous_end = span.end.0;
                }
                assert_eq!(
                    c.line_span(oracle.len(), budget()),
                    Err(CoordinateError::InvalidLine)
                );
                assert_eq!(previous_end, text.len());
                assert_eq!(
                    c.scalar_to_byte(ScalarIndex(usize::MAX), budget()),
                    Err(CoordinateError::InvalidScalar)
                );
                assert_eq!(
                    c.utf16_to_byte(Utf16Index(usize::MAX), budget()),
                    Err(CoordinateError::InvalidUtf16)
                );
                for byte in (0..=text.len()).filter(|&byte| text.is_char_boundary(byte)) {
                    let scalar = text[..byte].chars().count();
                    let utf16 = text[..byte].encode_utf16().count();
                    assert_eq!(
                        c.byte_to_scalar(BytePosition(byte), budget()),
                        Ok(ScalarIndex(scalar))
                    );
                    assert_eq!(
                        c.utf16_to_byte(Utf16Index(utf16), budget()),
                        Ok(BytePosition(byte))
                    );
                    assert_eq!(
                        c.scalar_to_byte(ScalarIndex(scalar), budget()),
                        Ok(BytePosition(byte))
                    );
                    match c.byte_to_line(BytePosition(byte), budget()) {
                        Ok(line) => {
                            assert_eq!(c.line_to_byte(line, budget()), Ok(BytePosition(byte)))
                        }
                        Err(CoordinateError::UnavailableProjection) => assert!(
                            byte > 0 && text.as_bytes().get(byte - 1..byte + 1) == Some(b"\r\n")
                        ),
                        other => panic!("unexpected {other:?}"),
                    }
                }
            }
        }
    }
}
