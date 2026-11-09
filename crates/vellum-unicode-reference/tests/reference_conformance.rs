//! Explicit reference gate: missing tools/artifacts fail, never silently skip.
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use vellum_engine::attachment::Affinity;
use vellum_engine::coordinates::BytePosition;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, Recorder};
use vellum_engine::selection::{Selection, SelectionBudget};
use vellum_engine::source::{Limits, Replacement};
use vellum_unicode_reference::{ReferenceProvider, ReferenceRow};
fn provider() -> ReferenceProvider {
    let jar = std::env::var_os("VELLUM_ICU4J_JAR")
        .expect("check-unicode requires VELLUM_ICU4J_JAR naming the pinned ICU4J 79.1 jar");
    let java = std::env::var_os("VELLUM_JAVA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("java"));
    ReferenceProvider::qualify(&java, Path::new(&jar), &mut Recorder::disabled())
        .expect("pinned reference artifact qualification failed")
}
struct Case {
    text: String,
    expected: Vec<BytePosition>,
}
fn artifact(name: &str, digest: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../formal")
        .join(name);
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), digest);
    bytes
}
fn cases(bytes: &[u8]) -> Vec<Case> {
    std::str::from_utf8(bytes)
        .unwrap()
        .lines()
        .filter_map(|line| {
            let data = line.split('#').next().unwrap().trim();
            if data.is_empty() {
                return None;
            }
            let mut text = String::new();
            let mut expected = Vec::new();
            for token in data.split_whitespace() {
                match token {
                    "÷" => expected.push(BytePosition(text.len())),
                    "×" => {}
                    hex => {
                        text.push(char::from_u32(u32::from_str_radix(hex, 16).unwrap()).unwrap())
                    }
                }
            }
            Some(Case { text, expected })
        })
        .collect()
}
fn point(at: usize) -> Selection {
    Selection {
        anchor: BytePosition(at),
        head: BytePosition(at),
        affinity: Affinity::After,
    }
}
fn request(unit: UnicodeUnitV1) -> UnicodeCommandV1 {
    UnicodeCommandV1 {
        unit,
        direction: UnicodeDirectionV1::Forward,
        count: 1,
        extend: false,
    }
}
fn context(
    owner: &SourceCoordinator,
    view: &LogicalViewId,
    r: &mut Recorder,
) -> LogicalViewContext {
    owner.logical_view(view, r).unwrap().context().unwrap()
}
fn reset(owner: &mut SourceCoordinator, view: &LogicalViewId, r: &mut Recorder) {
    let before = context(owner, view, r);
    owner
        .set_logical_selections(&before, &[point(0)], 0, r)
        .unwrap();
}
fn engine(text: &str) -> (SourceCoordinator, DocumentVersion, LogicalViewId, Recorder) {
    let mut owner = SourceCoordinator::with_invocation_and_history_limits(
        CoordinatorLimits {
            max_document_incarnations: 1,
            max_participants: 1,
            max_staged_bytes: 65536,
            source: Limits {
                max_source_bytes: 16384,
                max_edits: 4,
                max_edit_payload_bytes: 32768,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 8,
            max_retained: 8,
            max_intent_bytes: 4096,
        },
        HistoryLimits {
            max_retained_groups: 4,
            max_journal_bytes: 65536,
            max_pins: 4,
        },
    );
    let mut r = Recorder::disabled();
    owner
        .configure_logical_views(
            LogicalViewLimits {
                max_live_views: 1,
                max_issued_views: 1,
                max_total_selections: 8,
                normalization: SelectionBudget {
                    max_input_selections: 8,
                    max_output_selections: 8,
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
                max_issued: 1,
                max_retained: 1,
                max_source_bytes: 16384,
                max_boundaries: 16384,
                max_words: 2048,
                max_evidence_bytes: 262144,
                max_retained_bytes: 262144,
            },
            &mut r,
        )
        .unwrap();
    owner
        .configure_unicode_commands(
            UnicodeCommandBudget {
                max_steps: 4096,
                max_derived_bytes: 65536,
            },
            &mut r,
        )
        .unwrap();
    // Canonical corpus text is literal edit data, not a file import: initial FEFF
    // must stay text rather than accidentally becoming BOM metadata.
    let source = owner.create(b"", &mut r).unwrap();
    let plan = owner
        .prepare(
            &[DocumentEdits {
                expected: source.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 0,
                    expected: String::new(),
                    inserted: text.into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    owner.commit(plan, &mut r).unwrap();
    let source = owner.read(&source.document, &mut r).unwrap().version;
    assert_eq!(
        owner.read(&source.document, &mut r).unwrap().source.text(),
        text
    );
    let view = owner.create_logical_view(&mut r).unwrap();
    let stamp = owner.logical_view(&view, &mut r).unwrap().stamp;
    owner
        .attach_logical_view(&view, stamp, &source, &[point(0)], 0, &mut r)
        .unwrap();
    (owner, source, view, r)
}
struct Frame {
    owner: SourceCoordinator,
    source: DocumentVersion,
    view: LogicalViewId,
    capture: BoundaryCaptureId,
    r: Recorder,
}
fn frame(text: &str) -> Frame {
    let (mut owner, source, view, mut r) = engine(text);
    let capture = owner.begin_boundary_capture(&source, &mut r).unwrap();
    Frame {
        owner,
        source,
        view,
        capture,
        r,
    }
}
fn consume(
    frame: Frame,
    text: &str,
    row: &ReferenceRow,
    expected_graphemes: &[BytePosition],
) -> (usize, usize, usize) {
    let Frame {
        mut owner,
        source,
        view,
        capture,
        mut r,
    } = frame;
    let incompatible = |start: bool| {
        row.table.significant_words.iter().any(|word| {
            row.table
                .graphemes
                .binary_search_by_key(&if start { word.start } else { word.end }, |p| p.0)
                .is_err()
        })
    };
    let (start_bad, end_bad) = (incompatible(true), incompatible(false));
    let result = owner.accept_boundary_evidence(&capture, &row.table, &mut r);
    let evidence = result.expect("native table acceptance failed");
    let initial = context(&owner, &view, &mut r);
    let id = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit_unicode_command(
            &id,
            &initial,
            &evidence,
            UnicodeOperationV1::Movement,
            request(UnicodeUnitV1::WordEnd),
            &mut r,
        )
        .unwrap();
    owner.run(&id, &mut r).unwrap();
    if end_bad {
        assert_eq!(
            owner.invocation_outcome(&id, &mut r),
            Ok(&InvocationOutcome::UnicodeRejected(
                UnicodeCommandError::InconsistentProfile
            ))
        );
    }
    let expected = if end_bad {
        0
    } else {
        row.table
            .significant_words
            .first()
            .map_or(text.len(), |w| w.end)
    };
    assert_eq!(
        owner
            .logical_view(&view, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[point(expected)]
    );
    let generation = owner.generation();
    owner
        .submit_unicode_command(
            &id,
            &initial,
            &evidence,
            UnicodeOperationV1::Movement,
            request(UnicodeUnitV1::WordEnd),
            &mut r,
        )
        .unwrap();
    owner.run(&id, &mut r).unwrap();
    assert_eq!(owner.generation(), generation);
    reset(&mut owner, &view, &mut r);
    let plan = owner.prepare_unicode_movement(
        &context(&owner, &view, &mut r),
        &evidence,
        request(UnicodeUnitV1::WordStart),
        &mut r,
    );
    if start_bad {
        assert_eq!(plan.err(), Some(UnicodeCommandError::InconsistentProfile));
    } else {
        owner
            .commit_unicode_movement(plan.unwrap(), &mut r)
            .unwrap();
    }
    let expected = if start_bad {
        0
    } else {
        row.table
            .significant_words
            .iter()
            .find(|w| w.start > 0)
            .map_or(text.len(), |w| w.start)
    };
    assert_eq!(
        owner
            .logical_view(&view, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[point(expected)]
    );
    reset(&mut owner, &view, &mut r);
    let mut movements = 0;
    for expected in expected_graphemes.iter().skip(1) {
        let before = context(&owner, &view, &mut r);
        let plan = owner
            .prepare_unicode_movement(&before, &evidence, request(UnicodeUnitV1::Grapheme), &mut r)
            .unwrap();
        owner.commit_unicode_movement(plan, &mut r).unwrap();
        assert_eq!(
            owner
                .logical_view(&view, &mut r)
                .unwrap()
                .selections
                .selections(),
            &[point(expected.0)]
        );
        movements += 1;
    }
    reset(&mut owner, &view, &mut r);
    let before = context(&owner, &view, &mut r);
    let id = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit_unicode_command(
            &id,
            &before,
            &evidence,
            UnicodeOperationV1::Deletion,
            request(UnicodeUnitV1::Grapheme),
            &mut r,
        )
        .unwrap();
    owner.run(&id, &mut r).unwrap();
    let InvocationOutcome::Committed(outcome) = owner.invocation_outcome(&id, &mut r).unwrap()
    else {
        panic!("not committed")
    };
    let group = outcome.history_group.clone();
    let first = row.table.graphemes.get(1).map_or(0, |p| p.0);
    assert_eq!(
        owner.read(&source.document, &mut r).unwrap().source.text(),
        &text[first..]
    );
    if let Some(group) = group {
        let undo = owner
            .prepare_reversal(&group, HistoryDirection::Undo, &mut r)
            .unwrap();
        owner.commit(undo, &mut r).unwrap();
    } else {
        assert!(text.is_empty());
    }
    assert_eq!(
        owner.read(&source.document, &mut r).unwrap().source.text(),
        text
    );
    owner.retire_boundary_evidence(&evidence, &mut r).unwrap();
    assert_eq!(owner.boundary_usage().retained_bytes, 0);
    (usize::from(start_bad), usize::from(end_bad), movements)
}
#[test]
fn full_pinned_corpora_feed_the_real_rust_capture_commands_replay_and_undo() {
    let provider = provider();
    let graphemes = cases(&artifact(
        "unicode18-grapheme-test.txt",
        "b0cf047ee94485bbdc846de2b902f5f8a815f6b674f9d04223cddadd91c9df31",
    ));
    let words = cases(&artifact(
        "unicode18-word-test.txt",
        "3dd70c071781276067c680d87303f60434adce7b067bd063194af374edad86a5",
    ));
    artifact(
        "unicode-LICENSE.txt",
        "e7a93b009565cfce55919a381437ac4db883e9da2126fa28b91d12732bc53d96",
    );
    assert_eq!(graphemes.len(), 853);
    assert_eq!(words.len(), 1944);
    let additional = [
        "",
        "ภาษาไทย",
        "日本語",
        "\u{feff}",
        "e\u{301}",
        "é",
        "a🇦\u{200d}🇧🇨b",
        "A 12 !",
    ];
    let texts: Vec<_> = graphemes
        .iter()
        .chain(&words)
        .map(|c| c.text.as_str())
        .chain(additional)
        .collect();
    let mut frames: Vec<_> = texts.iter().map(|text| frame(text)).collect();
    let inputs: Vec<_> = frames
        .iter_mut()
        .map(|frame| {
            frame
                .owner
                .boundary_source(&frame.capture, &mut frame.r)
                .unwrap()
                .text
        })
        .collect();
    assert_eq!(inputs, texts);
    let rows = provider
        .run_batch(&inputs, &mut Recorder::disabled())
        .unwrap();
    // Compare every native field against the separately pinned Java reference,
    // then run the existing real-engine command/replay/deletion/undo scenarios
    // with the native tables, not the oracle's tables. Dictionary output remains
    // an oracle-only diagnostic; the native adapter never substitutes it.
    #[cfg(feature = "native-conformance")]
    let rows: Vec<_> = {
        let mut native = native_provider();
        rows.into_iter()
            .zip(&inputs)
            .map(|(oracle, text)| {
                let table = native.segment(text, &mut Recorder::disabled()).unwrap();
                assert_eq!(
                    table, oracle.table,
                    "native table differs from pinned reference"
                );
                ReferenceRow {
                    table,
                    dictionary: oracle.dictionary,
                }
            })
            .collect()
    };
    drop(inputs);
    assert_eq!(rows.len(), texts.len());
    for (row, case) in rows.iter().zip(&graphemes) {
        assert_eq!(row.table.graphemes, case.expected);
    }
    for (row, case) in rows[graphemes.len()..].iter().zip(&words) {
        assert_eq!(row.table.words, case.expected);
    }
    let extra = &rows[graphemes.len() + words.len()..];
    assert_eq!(
        extra[1].table.words.iter().map(|p| p.0).collect::<Vec<_>>(),
        vec![0, 3, 6, 9, 12, 15, 18, 21]
    );
    assert_eq!(
        extra[1].dictionary.iter().map(|p| p.0).collect::<Vec<_>>(),
        vec![0, 12, 21]
    );
    assert_eq!(
        extra[2].table.words.iter().map(|p| p.0).collect::<Vec<_>>(),
        vec![0, 3, 6, 9]
    );
    assert_eq!(
        extra[2].dictionary.iter().map(|p| p.0).collect::<Vec<_>>(),
        vec![0, 9]
    );
    assert_eq!(
        extra[3].table.graphemes,
        vec![BytePosition(0), BytePosition(3)]
    );
    assert_eq!(
        extra[4].table.graphemes,
        vec![BytePosition(0), BytePosition(3)]
    );
    assert_eq!(
        extra[5].table.graphemes,
        vec![BytePosition(0), BytePosition(2)]
    );
    assert!(
        extra[6]
            .table
            .words
            .iter()
            .any(|p| !extra[6].table.graphemes.contains(p))
    );
    assert_eq!(
        extra[7].table.significant_words,
        vec![
            ByteRange { start: 0, end: 1 },
            ByteRange { start: 2, end: 4 }
        ]
    );
    let (mut start_refused, mut end_refused, mut movements) = (0, 0, 0);
    for (i, ((text, row), frame)) in texts.iter().zip(&rows).zip(frames).enumerate() {
        let expected = if i < graphemes.len() {
            &graphemes[i].expected
        } else {
            &row.table.graphemes
        };
        let (start, end, count) = consume(frame, text, row, expected);
        start_refused += start;
        end_refused += end;
        movements += count;
    }
    assert_eq!(texts.len(), 2805);
    assert_eq!((start_refused, end_refused), (8, 5));
    assert_eq!(movements, 7312);
    println!(
        "Reference conformance: 853 grapheme / 1944 word cases; all 2805 native tables accepted; word-start refusals={start_refused}, word-end refusals={end_refused}; grapheme movements={movements}; replay and grapheme deletion/undo exercised for every table"
    );
}

#[cfg(feature = "native-conformance")]
fn native_provider() -> vellum_unicode_icu::Provider {
    vellum_unicode_icu::Provider::new(
        vellum_unicode_icu::Limits {
            max_source_bytes: 16384,
            max_boundaries: 16384,
            max_words: 2048,
            max_logical_bytes: 1024 * 1024,
        },
        vellum_unicode_icu::Timing::Off,
        &mut Recorder::disabled(),
    )
    .unwrap()
}

#[cfg(feature = "native-conformance")]
#[test]
fn native_default_words_match_the_reference_across_ignored_and_complex_script_contexts() {
    let symbols = [
        "a", "א", "1", "'", ".", ":", "\"", "カ", "_", " ", "\r", "\n", "\u{85}", "\u{200d}",
        "\u{308}", "\u{2060}", "🇦", "©", "ภ", "日",
    ];
    let mut texts = Vec::new();
    for left in symbols {
        for right in symbols {
            for middle in ["", "\u{308}", "\u{2060}", "\u{200d}"] {
                texts.push(format!("a{left}{middle}{right}a"));
            }
        }
    }
    texts.push("🇦\u{308}".repeat(512));
    texts.push(format!(
        "a{}'{}a",
        "\u{308}".repeat(512),
        "\u{2060}".repeat(512)
    ));
    let inputs: Vec<_> = texts.iter().map(String::as_str).collect();
    let oracle = provider()
        .run_batch(&inputs, &mut Recorder::disabled())
        .unwrap();
    let mut native = native_provider();
    for (index, (text, row)) in inputs.iter().zip(oracle).enumerate() {
        let table = native.segment(text, &mut Recorder::disabled()).unwrap();
        assert_eq!(table, row.table, "native contextual case {index}");
    }
    assert_eq!(native.usage().completed, 1602);
    assert_eq!(native.usage().refused, 0);
}

#[cfg(feature = "native-conformance")]
#[test]
fn native_attestation_uses_original_current_capture_and_leaves_refusals_owned() {
    use vellum_unicode_icu::{CaptureError, Error};
    let mut native = native_provider();
    let mut f = frame("é🇺🇸");
    let mut other = frame("é🇺🇸");
    assert!(matches!(
        native.attest_capture(&mut other.owner, &f.capture, &mut f.r),
        Err(CaptureError::Engine(_))
    ));
    assert_eq!(native.usage().offered, 0);
    let evidence = native
        .attest_capture(&mut f.owner, &f.capture, &mut f.r)
        .unwrap();
    assert_eq!(f.owner.boundary_usage().ready, 1);
    f.owner
        .retire_boundary_evidence(&evidence, &mut f.r)
        .unwrap();
    assert_eq!(f.owner.boundary_usage().retained_bytes, 0);

    let mut stale = frame("a");
    let plan = stale
        .owner
        .prepare(
            &[DocumentEdits {
                expected: stale.source.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "a".into(),
                    inserted: "b".into(),
                }],
            }],
            &[],
            &mut stale.r,
        )
        .unwrap();
    stale.owner.commit(plan, &mut stale.r).unwrap();
    assert_eq!(
        native
            .attest_capture(&mut stale.owner, &stale.capture, &mut stale.r)
            .err(),
        Some(CaptureError::Engine(BoundaryError::Stale))
    );
    assert_eq!(native.usage().offered, 1);
    stale
        .owner
        .retire_boundary_capture(&stale.capture, &mut stale.r)
        .unwrap();

    let mut f = frame("a");
    let mut too_small = vellum_unicode_icu::Provider::new(
        vellum_unicode_icu::Limits {
            max_source_bytes: 0,
            max_boundaries: 2,
            max_words: 0,
            max_logical_bytes: 24,
        },
        vellum_unicode_icu::Timing::Off,
        &mut f.r,
    )
    .unwrap();
    assert_eq!(
        too_small
            .attest_capture(&mut f.owner, &f.capture, &mut f.r)
            .err(),
        Some(CaptureError::Provider(Error::InputLimit))
    );
    assert_eq!(f.owner.boundary_usage().pending, 1);
    assert_eq!(f.owner.boundary_usage().ready, 0);
    f.owner
        .retire_boundary_capture(&f.capture, &mut f.r)
        .unwrap();
    other
        .owner
        .retire_boundary_capture(&other.capture, &mut other.r)
        .unwrap();
}
#[test]
fn real_reference_capture_off_available_or_saturated_preserves_results_and_measures_service() {
    let provider = provider();
    let texts = ["A 12 !", "\u{feff}", "a🇦\u{200d}🇧🇨b"];
    let mut baseline: Option<Vec<ReferenceRow>> = None;
    for capacity in [None, Some(16), Some(1)] {
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let rows = provider.run_batch(&texts, &mut r).unwrap();
        if let Some(original) = &baseline {
            for (a, b) in rows.iter().zip(original) {
                assert_eq!(a.table, b.table);
                assert_eq!(a.dictionary, b.dictionary);
            }
        } else {
            baseline = Some(rows);
        }
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(16) {
            assert!(
                records
                    .iter()
                    .any(|o| o.boundary == Boundary::UnicodeProviderExecution)
            );
            assert!(
                records
                    .iter()
                    .any(|o| o.boundary == Boundary::UnicodeProviderDecode)
            );
            assert!(records.iter().all(|o| o.service_time.is_some()));
        }
    }
}
