use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::source::Limits;
fn owner() -> (SourceCoordinator, Recorder) {
    let mut o = SourceCoordinator::with_history_limits(
        CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 4096,
                max_edits: 16,
                max_edit_payload_bytes: 8192,
            },
        },
        HistoryLimits {
            max_retained_groups: 16,
            max_journal_bytes: 65536,
            max_pins: 8,
        },
    );
    let mut r = Recorder::disabled();
    o.configure_semantic_root_budget(1048576, &mut r).unwrap();
    (o, r)
}
fn request(source: &[u8], bom: bool) -> ScratchDocumentRequestV1<'_> {
    ScratchDocumentRequestV1 {
        source,
        serialization: SourceSerializationV1 {
            profile: SourceSerializationProfileV1::StrictUtf8OneBom,
            bom,
        },
        insertion_policy: GeneratedNewlineV1::CrLf,
    }
}
#[test]
fn scratch_preserves_logical_utf8_and_literal_feff_without_file_signature_interpretation() {
    let (mut o, mut r) = owner();
    let source = "\u{feff}prose\r\n```mermaid\ngraph TD\rA-->B\n```\nDeutsch: Grüße 👩‍💻";
    let history = o.history_usage();
    let held = o.retention_usage();
    let generation = o.generation();
    let version = o
        .create_scratch_document_v1(&request(source.as_bytes(), false), &mut r)
        .unwrap();
    assert_eq!(version.revision, 1);
    assert_eq!(
        o.read(&version.document, &mut r).unwrap().source.text(),
        source
    );
    assert_eq!(
        o.annotation_set_version(&version.document, &mut r).unwrap(),
        1
    );
    assert_eq!(
        o.lifecycle_state(&version.document, &mut r).unwrap().phase,
        LifecyclePhase::Live
    );
    assert_eq!(
        o.source_insertion_policy_v1(&version, &mut r).unwrap(),
        GeneratedNewlineV1::CrLf
    );
    assert_eq!(
        o.source_serialization(&version.document, &mut r)
            .unwrap()
            .serialization,
        request(&[], false).serialization
    );
    assert_eq!(o.generation(), generation + 1);
    assert_eq!(o.history_usage(), history);
    assert_eq!(o.retention_usage(), held);
    let imported = o.create(source.as_bytes(), &mut r).unwrap();
    assert_eq!(imported.revision, 0);
    assert_eq!(
        o.read(&imported.document, &mut r).unwrap().source.text(),
        source.trim_start_matches('\u{feff}')
    );
    assert!(
        o.source_serialization(&imported.document, &mut r)
            .unwrap()
            .serialization
            .bom
    );
}

#[test]
fn scratch_keeps_source_and_output_intent_independent_for_every_delimiter_and_bom_choice() {
    for delimiter in [
        GeneratedNewlineV1::Lf,
        GeneratedNewlineV1::CrLf,
        GeneratedNewlineV1::Cr,
    ] {
        for bom in [false, true] {
            for source in ["", "\u{feff}\u{feff}", "\0a\r\nb\rc\n", "e\u{301} 👩‍💻"] {
                let (mut o, mut r) = owner();
                let mut q = request(source.as_bytes(), bom);
                q.insertion_policy = delimiter;
                let v = o.create_scratch_document_v1(&q, &mut r).unwrap();
                let read = o.read(&v.document, &mut r).unwrap();
                assert_eq!(read.source.text().as_bytes(), source.as_bytes());
                assert_eq!(
                    o.source_serialization(&v.document, &mut r)
                        .unwrap()
                        .serialization,
                    q.serialization
                );
                assert_eq!(o.source_insertion_policy_v1(&v, &mut r).unwrap(), delimiter);
                let life = o.lifecycle_state(&v.document, &mut r).unwrap();
                assert_eq!(life.generation, 1);
                assert_eq!(life.pair_generation, 1);
                assert_eq!(o.lifecycle_usage().issued_lifetime, 1);
                assert_eq!(o.lifecycle_usage().live, 1);
                assert_eq!(o.history_usage().retained_groups, 0);
            }
        }
    }
}
#[test]
fn invalid_logical_utf8_and_full_literal_byte_limits_refuse_without_publishing_or_spending_identity()
 {
    let (mut o, mut r) = owner();
    let valid = o
        .create_scratch_document_v1(&request(b"peer", true), &mut r)
        .unwrap();
    for bytes in [
        vec![0xc3],
        vec![0xc0, 0xaf],
        vec![0xed, 0xa0, 0x80],
        vec![0xef, 0xbb],
        vec![b'x'; 4097],
    ] {
        let generation = o.generation();
        let usage = o.lifecycle_usage();
        let root = o.semantic_root_usage(&mut r).unwrap();
        assert!(
            o.create_scratch_document_v1(&request(&bytes, false), &mut r)
                .is_err()
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.lifecycle_usage(), usage);
        assert_eq!(o.semantic_root_usage(&mut r).unwrap(), root);
        assert_eq!(
            o.read(&valid.document, &mut r).unwrap().source.text(),
            "peer"
        );
        assert!(!o.is_faulted());
    }
    let mut bytes = vec![b'x'; 4096];
    bytes.splice(0..3, [0xef, 0xbb, 0xbf]);
    let v = o
        .create_scratch_document_v1(&request(&bytes, false), &mut r)
        .unwrap();
    assert_eq!(
        o.read(&v.document, &mut r).unwrap().source.text().len(),
        4096
    );
    bytes.push(b'x');
    assert_eq!(
        o.create_scratch_document_v1(&request(&bytes, false), &mut r),
        Err(CoordinatorError::Source(
            vellum_engine::source::SourceError::SourceLimit
        ))
    );
}
#[test]
fn scratch_owns_its_text_has_distinct_identity_and_never_reuses_disposed_slots() {
    let (mut o, mut r) = owner();
    let mut bytes = b"original".to_vec();
    let a = o
        .create_scratch_document_v1(&request(&bytes, false), &mut r)
        .unwrap();
    bytes.fill(b'X');
    let b = o
        .create_scratch_document_v1(&request(b"original", false), &mut r)
        .unwrap();
    assert_ne!(a.document, b.document);
    assert_eq!(
        o.read(&a.document, &mut r).unwrap().source.text(),
        "original"
    );
    o.dispose(&a.document, &mut r).unwrap();
    for _ in 0..6 {
        o.create_scratch_document_v1(&request(b"more", false), &mut r)
            .unwrap();
    }
    let generation = o.generation();
    assert_eq!(
        o.create_scratch_document_v1(&request(b"overflow", false), &mut r),
        Err(CoordinatorError::Limit)
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(o.lifecycle_usage().issued_lifetime, 8);
}
#[test]
fn scratch_capture_modes_have_identical_births_and_bounded_optional_redacted_service_timing() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for mode in 0..4 {
        let (mut o, mut setup) = owner();
        let peer = o.create(b"peer", &mut setup).unwrap();
        let history = o.history_usage();
        let held = o.retention_usage();
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 32 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1)
        }
        if mode == 3 {
            r.record(Observation {
                boundary: Boundary::Control,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        let v = o
            .create_scratch_document_v1(&request("\u{feff}PRIVATE".as_bytes(), false), &mut r)
            .unwrap();
        assert_eq!(
            o.read(&v.document, &mut setup).unwrap().source.text(),
            "\u{feff}PRIVATE"
        );
        assert_eq!(
            o.read(&peer.document, &mut setup).unwrap().source.text(),
            "peer"
        );
        assert_eq!(o.history_usage(), history);
        assert_eq!(o.retention_usage(), held);
        let mut counts = [0; 3];
        while let Some(e) = r.pop_front() {
            let i = match e.boundary {
                Boundary::SourceAuthoringValidation => Some(0),
                Boundary::DocumentBirthPublication => Some(1),
                Boundary::ScratchCreation => Some(2),
                _ => None,
            };
            if let Some(i) = i {
                counts[i] += 1;
                assert_eq!(e.disposition, Disposition::Completed);
                assert_eq!(e.service_time.is_some(), mode == 2 && i != 0)
            }
            assert!(!format!("{e:?}").contains("PRIVATE"));
        }
        if mode == 1 || mode == 2 {
            assert_eq!(counts, [1, 1, 1])
        } else {
            assert_eq!(counts, [0, 0, 0])
        }
    }
}

#[test]
fn scratch_initial_content_is_not_an_edit_group_and_later_edits_reverse_with_fresh_versions() {
    use vellum_engine::source::Replacement;
    let (mut o, mut r) = owner();
    let text = "\u{feff}original\r\n";
    let v = o
        .create_scratch_document_v1(&request(text.as_bytes(), true), &mut r)
        .unwrap();
    assert_eq!(o.history_usage().retained_groups, 0);
    let p = o
        .prepare(
            &[DocumentEdits {
                expected: v.clone(),
                edits: vec![Replacement {
                    start: 3,
                    end: 11,
                    expected: "original".into(),
                    inserted: "changed".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    let out = o.commit(p, &mut r).unwrap();
    assert_eq!(out.changes[0].after.revision, 2);
    let group = out.history_group.unwrap();
    let p = o
        .prepare_reversal(&group, HistoryDirection::Undo, &mut r)
        .unwrap();
    let undone = o.commit(p, &mut r).unwrap();
    assert_eq!(undone.changes[0].after.revision, 3);
    assert_eq!(o.read(&v.document, &mut r).unwrap().source.text(), text);
    assert!(
        o.source_serialization(&v.document, &mut r)
            .unwrap()
            .serialization
            .bom
    );
    assert_eq!(o.annotation_set_version(&v.document, &mut r).unwrap(), 1);
}
