//! Native trusted-owner queries, not recipient-scoped snapshot release.
use vellum_engine::coordinates::*;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, Recorder};
use vellum_engine::source::{Limits, Replacement};
fn owner() -> SourceCoordinator {
    SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 8,
        max_participants: 8,
        max_staged_bytes: 4096,
        source: Limits {
            max_source_bytes: 4096,
            max_edits: 8,
            max_edit_payload_bytes: 4096,
        },
    })
}
fn budget() -> SourceQueryBudget {
    SourceQueryBudget {
        max_scan_bytes: 4096,
        max_result_bytes: 4096,
    }
}

#[test]
fn line_and_range_products_keep_exact_source_context_and_separate_body_from_delimiter() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner
        .create("\u{feff}é\r\n😀\rZ\n".as_bytes(), &mut recorder)
        .unwrap();
    let line = owner
        .read_source_line(&source, 0, budget(), &mut recorder)
        .unwrap();
    assert_eq!(line.source, source);
    assert_eq!(line.line, 0);
    assert_eq!(line.body(), "é".as_bytes());
    assert_eq!(line.delimiter(), b"\r\n");
    assert_eq!(
        line.span,
        LineSpan {
            start: BytePosition(0),
            body_end: BytePosition(2),
            end: BytePosition(4)
        }
    );
    let range = owner
        .read_source_range(&source, BytePosition(3), BytePosition(4), 1, &mut recorder)
        .unwrap();
    assert_eq!(range.bytes(), b"\n");
    assert_eq!(range.source, source);
    assert_eq!(range.start, BytePosition(3));
    assert_eq!(range.end, BytePosition(4));
    let last = owner
        .read_source_line(&source, 3, budget(), &mut recorder)
        .unwrap();
    assert_eq!(last.body(), b"");
    assert_eq!(last.delimiter(), b"");
    assert_eq!(
        owner.convert_source_coordinate(
            &source,
            CoordinateRequestV1::ByteToLine(BytePosition(3)),
            CoordinateBudget {
                max_scan_bytes: 4096
            },
            &mut recorder
        ),
        Err(SourceQueryError::Coordinate(
            CoordinateError::UnavailableProjection
        ))
    );
}

#[test]
fn whole_reply_and_scan_limits_never_return_a_prefix_or_spend_a_publication() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner
        .create(b"PRIVATE\r\nremaining", &mut recorder)
        .unwrap();
    let generation = owner.generation();
    assert_eq!(
        owner.read_source_line(
            &source,
            0,
            SourceQueryBudget {
                max_scan_bytes: 9,
                max_result_bytes: 8
            },
            &mut recorder
        ),
        Err(SourceQueryError::Coordinate(CoordinateError::Limit))
    );
    assert_eq!(
        owner.read_source_line(
            &source,
            0,
            SourceQueryBudget {
                max_scan_bytes: 8,
                max_result_bytes: 9
            },
            &mut recorder
        ),
        Err(SourceQueryError::Coordinate(CoordinateError::Limit))
    );
    let line = owner
        .read_source_line(
            &source,
            0,
            SourceQueryBudget {
                max_scan_bytes: 9,
                max_result_bytes: 9,
            },
            &mut recorder,
        )
        .unwrap();
    assert_eq!(line.body(), b"PRIVATE");
    assert_eq!(line.delimiter(), b"\r\n");
    assert_eq!(
        owner.read_source_range(&source, BytePosition(0), BytePosition(7), 6, &mut recorder),
        Err(SourceQueryError::Coordinate(CoordinateError::Limit))
    );
    assert_eq!(owner.generation(), generation);
}

#[test]
fn scalar_splits_inverted_ranges_and_unknown_lines_refuse_while_empty_eof_ranges_are_valid() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create("é\r\n".as_bytes(), &mut recorder).unwrap();
    assert_eq!(
        owner.read_source_range(&source, BytePosition(0), BytePosition(1), 64, &mut recorder),
        Err(SourceQueryError::Coordinate(
            CoordinateError::InvalidBoundary
        ))
    );
    assert_eq!(
        owner.read_source_range(&source, BytePosition(3), BytePosition(2), 64, &mut recorder),
        Err(SourceQueryError::Coordinate(CoordinateError::InvalidRange))
    );
    assert_eq!(
        owner.read_source_line(&source, 2, budget(), &mut recorder),
        Err(SourceQueryError::Coordinate(CoordinateError::InvalidLine))
    );
    assert_eq!(
        owner
            .read_source_range(&source, BytePosition(4), BytePosition(4), 0, &mut recorder)
            .unwrap()
            .bytes(),
        b""
    );
    assert_eq!(
        owner.read_source_range(
            &source,
            BytePosition(usize::MAX),
            BytePosition(usize::MAX),
            0,
            &mut recorder
        ),
        Err(SourceQueryError::Coordinate(CoordinateError::InvalidByte))
    );
}

#[test]
fn source_revision_namespace_and_retained_disposal_are_checked_even_for_empty_queries() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let original = owner.create(b"A", &mut recorder).unwrap();
    let copy = owner
        .read_source_line(&original, 0, budget(), &mut recorder)
        .unwrap();
    let edit = owner
        .prepare(
            &[DocumentEdits {
                expected: original.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: "B".into(),
                }],
            }],
            &[],
            &mut recorder,
        )
        .unwrap();
    let changed = owner.commit(edit, &mut recorder).unwrap().changes[0]
        .after
        .clone();
    assert_eq!(
        owner.read_source_range(
            &original,
            BytePosition(0),
            BytePosition(0),
            0,
            &mut recorder
        ),
        Err(SourceQueryError::Owner(CoordinatorError::Stale))
    );
    let foreign = crate::owner().create(b"B", &mut recorder).unwrap();
    assert_eq!(
        owner.read_source_line(&foreign, 0, budget(), &mut recorder),
        Err(SourceQueryError::Owner(CoordinatorError::Unavailable))
    );
    let plan = owner
        .prepare_retained_disposal(&changed, 0, &mut recorder)
        .unwrap();
    owner.commit_retained_disposal(plan, &mut recorder).unwrap();
    assert_eq!(
        owner.read_source_line(&changed, 0, budget(), &mut recorder),
        Err(SourceQueryError::Owner(CoordinatorError::Unavailable))
    );
    assert_eq!(copy.body(), b"A");
}

#[test]
fn versioned_coordinate_facade_is_typed_and_read_only_source_can_still_be_queried() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create("😀".as_bytes(), &mut recorder).unwrap();
    owner
        .set_read_only(&source.document, true, &mut recorder)
        .unwrap();
    let generation = owner.generation();
    let cases = [
        (
            CoordinateRequestV1::ByteToUtf16(BytePosition(4)),
            CoordinateResultV1::Utf16(Utf16Index(2)),
        ),
        (
            CoordinateRequestV1::Utf16ToByte(Utf16Index(2)),
            CoordinateResultV1::Byte(BytePosition(4)),
        ),
        (
            CoordinateRequestV1::ByteToScalar(BytePosition(4)),
            CoordinateResultV1::Scalar(ScalarIndex(1)),
        ),
        (
            CoordinateRequestV1::ScalarToByte(ScalarIndex(1)),
            CoordinateResultV1::Byte(BytePosition(4)),
        ),
        (
            CoordinateRequestV1::ByteToLine(BytePosition(4)),
            CoordinateResultV1::Line(LinePosition {
                line: 0,
                body_byte_column: 4,
            }),
        ),
        (
            CoordinateRequestV1::LineToByte(LinePosition {
                line: 0,
                body_byte_column: 4,
            }),
            CoordinateResultV1::Byte(BytePosition(4)),
        ),
    ];
    for (request, result) in cases {
        assert_eq!(
            owner
                .convert_source_coordinate(
                    &source,
                    request,
                    CoordinateBudget { max_scan_bytes: 4 },
                    &mut recorder
                )
                .unwrap(),
            CoordinateReplyV1 {
                source: source.clone(),
                result
            }
        );
    }
    assert_eq!(owner.generation(), generation);
}

#[test]
fn diagnostics_disabled_available_or_full_preserve_query_results_and_redact_owned_payloads() {
    for capacity in [None, Some(32), Some(1)] {
        let mut owner = owner();
        let mut recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let source = owner.create(b"PRIVATE_BODY\r\n", &mut recorder).unwrap();
        let line = owner
            .read_source_line(&source, 0, budget(), &mut recorder)
            .unwrap();
        let range = owner
            .read_source_range(
                &source,
                BytePosition(0),
                BytePosition(12),
                12,
                &mut recorder,
            )
            .unwrap();
        assert_eq!(line.body(), range.bytes());
        assert!(!format!("{line:?} {range:?}").contains("PRIVATE_BODY"));
        owner
            .convert_source_coordinate(
                &source,
                CoordinateRequestV1::ByteToScalar(BytePosition(12)),
                CoordinateBudget { max_scan_bytes: 12 },
                &mut recorder,
            )
            .unwrap();
        let mut observations = Vec::new();
        while let Some(observation) = recorder.pop_front() {
            observations.push(observation);
        }
        if capacity == Some(32) {
            for boundary in [
                Boundary::SourceLineQuery,
                Boundary::SourceRangeQuery,
                Boundary::SourceCoordinateConversion,
            ] {
                assert!(
                    observations
                        .iter()
                        .any(|observation| observation.boundary == boundary)
                );
            }
        }
        assert!(
            observations
                .iter()
                .all(|observation| observation.service_time.is_none())
        );
        assert!(!format!("{observations:?}").contains("PRIVATE_BODY"));
        if capacity == Some(1) {
            assert!(recorder.dropped() > 0);
        }
    }
}
