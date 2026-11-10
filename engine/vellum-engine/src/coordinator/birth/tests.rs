use super::*;
use crate::instrumentation::{NativeTimingProfileV1, Observation};
fn owner() -> SourceCoordinator {
    SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 8,
        max_participants: 8,
        max_staged_bytes: 4096,
        source: Limits {
            max_source_bytes: 4096,
            max_edits: 16,
            max_edit_payload_bytes: 8192,
        },
    })
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
fn scratch_output_edge_refuses_ambiguous_absent_signature_instead_of_losing_authored_feff() {
    for bom in [false, true] {
        let mut o = owner();
        let mut r = Recorder::disabled();
        let text = "\u{feff}\u{feff}logical\r\ntext";
        let v = o
            .create_scratch_document_v1(&request(text.as_bytes(), bom), &mut r)
            .unwrap();
        let d = o.document(&v.document).unwrap();
        let encoded = d.serialization.serialization.encode(&d.source, &mut r);
        if bom {
            let bytes = encoded.unwrap();
            assert_eq!(&bytes[..3], b"\xef\xbb\xbf");
            assert_eq!(Utf8Source::decode(&bytes, 4096).unwrap().text(), text)
        } else {
            assert_eq!(encoded, Err(SourceError::AmbiguousBom))
        }
        assert_eq!(o.document(&v.document).unwrap().source.text(), text);
        assert_eq!(
            o.document(&v.document)
                .unwrap()
                .serialization
                .serialization
                .bom,
            bom
        );
    }
}
#[test]
fn scratch_birth_installs_known_empty_ownership_without_relations_bindings_or_baselines() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let v = o
        .create_scratch_document_v1(&request(b"authored", true), &mut r)
        .unwrap();
    let d = o.document(&v.document).unwrap();
    assert_eq!(d.revision, 1);
    assert_eq!(d.annotation_generation, 1);
    assert_eq!(d.lifecycle.generation, 1);
    assert_eq!(d.lifecycle.pair_generation, 1);
    assert_eq!(d.serialization.generation, 1);
    assert!(!d.read_only);
    assert!(!d.annotation_read_only);
    assert_eq!(o.resource_usage().bindings, 0);
    assert_eq!(o.resource_usage().annotation_bindings, 0);
    assert_eq!(o.resource_usage().annotation_baselines, 0);
    assert_eq!(o.resource_usage().pending_baseline_publications, 0);
    assert_eq!(o.logical_view_usage().attached, 0);
    assert_eq!(o.workspace_usage().memberships, 0);
    assert_eq!(o.history_usage().retained_groups, 0);
    assert_eq!(o.annotation_usage().retained_records, 0);
}
#[test]
fn scratch_and_import_share_actual_parent_root_headroom_before_identity_installation() {
    let mut r = Recorder::disabled();
    let probe = owner();
    let before = probe.semantic_root_usage(&mut r).unwrap().total_bytes;
    let needed = before + std::mem::size_of::<Option<Document>>() + 3;
    for scratch in [false, true] {
        for short in [false, true] {
            let mut o = owner();
            o.configure_semantic_root_budget(needed - usize::from(short), &mut r)
                .unwrap();
            let generation = o.generation();
            let result = if scratch {
                o.create_scratch_document_v1(&request(b"abc", false), &mut r)
            } else {
                o.create(b"abc", &mut r)
            };
            if short {
                assert_eq!(result, Err(CoordinatorError::Limit));
                assert_eq!(o.lifecycle_usage().issued_lifetime, 0);
                assert_eq!(o.generation(), generation);
                assert_eq!(o.semantic_root_usage(&mut r).unwrap().total_bytes, before)
            } else {
                result.unwrap();
                assert_eq!(o.semantic_root_usage(&mut r).unwrap().total_bytes, needed)
            }
        }
    }
}
#[test]
fn interrupted_scratch_and_import_births_are_unknown_not_retryable_or_refunded() {
    for scratch in [false, true] {
        for cut in 0..3 {
            for mode in 0..4 {
                let mut o = owner();
                let mut setup = Recorder::disabled();
                let peer = o.create(b"peer", &mut setup).unwrap();
                o.configure_retention_budget(1048576, &mut setup).unwrap();
                let _other = o.retention.reserve(71).unwrap();
                let generation = o.generation();
                let history = o.history_usage();
                let held = o.retention_usage();
                let mut r = if mode == 0 {
                    Recorder::disabled()
                } else {
                    Recorder::try_with_capacity(if mode == 3 { 1 } else { 64 }).unwrap()
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
                o.publication_fault_after = Some(cut);
                assert!(
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| if scratch {
                        o.create_scratch_document_v1(&request(b"PRIVATE", false), &mut r)
                    } else {
                        o.create(b"PRIVATE", &mut r)
                    }))
                    .is_err()
                );
                assert!(o.is_faulted());
                assert_eq!(o.retention_usage(), held);
                assert_eq!(o.history_usage(), history);
                assert_eq!(
                    o.documents[peer.document.slot]
                        .as_ref()
                        .unwrap()
                        .source
                        .text(),
                    "peer"
                );
                assert_eq!(o.documents.len(), if cut == 0 { 1 } else { 2 });
                assert_eq!(o.generation(), generation + u64::from(cut == 2));
                assert_eq!(
                    o.create_scratch_document_v1(&request(b"retry", false), &mut r),
                    Err(CoordinatorError::Faulted)
                );
                assert_eq!(o.create(b"retry", &mut r), Err(CoordinatorError::Faulted));
                let mut unknown = false;
                while let Some(e) = r.pop_front() {
                    if e.boundary == Boundary::DocumentBirthPublication {
                        assert_eq!(e.disposition, Disposition::Unknown);
                        assert_eq!(e.service_time.is_some(), mode == 2);
                        unknown = true;
                    }
                    assert!(!format!("{e:?}").contains("PRIVATE"));
                }
                if mode == 1 || mode == 2 {
                    assert!(unknown)
                }
            }
        }
    }
}

#[test]
fn scratch_root_or_counter_refusal_precedes_logical_source_construction() {
    for exhausted_counter in [false, true] {
        let mut o = owner();
        let mut setup = Recorder::disabled();
        let root = o.semantic_root_usage(&mut setup).unwrap().total_bytes;
        o.configure_semantic_root_budget(
            if exhausted_counter { 1048576 } else { root },
            &mut setup,
        )
        .unwrap();
        if exhausted_counter {
            o.generation = u64::MAX;
        }
        let mut r = Recorder::try_with_capacity(32).unwrap();
        let generation = o.generation();
        let result =
            o.create_scratch_document_v1(&request(b"valid authored payload", false), &mut r);
        assert_eq!(
            result,
            Err(if exhausted_counter {
                CoordinatorError::VersionExhausted
            } else {
                CoordinatorError::Limit
            })
        );
        assert_eq!(o.documents.len(), 0);
        assert_eq!(o.generation(), generation);
        while let Some(e) = r.pop_front() {
            assert_ne!(e.boundary, Boundary::SourceAuthoringValidation);
            assert_ne!(e.boundary, Boundary::DocumentBirthPublication);
        }
    }
}
