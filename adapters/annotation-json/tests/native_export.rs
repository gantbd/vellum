#![cfg(target_os = "macos")]
use std::sync::atomic::{AtomicU64, Ordering};
use vellum_adapter_annotation_json::*;
use vellum_adapter_local_fs::{FilesystemLimits, LocalFilesystem};
use vellum_engine::attachment::AttachmentTarget;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::resource::*;
use vellum_engine::source::Limits;
static NEXT: AtomicU64 = AtomicU64::new(0);
fn engine_config() -> CoordinatorConfig {
    CoordinatorConfig {
        limits: CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 8192,
            source: Limits {
                max_source_bytes: 4096,
                max_edits: 16,
                max_edit_payload_bytes: 8192,
            },
        },
        invocation: None,
        history: None,
        search: None,
        annotations: Some(AnnotationLimits {
            max_issued: 16,
            max_records: 16,
            max_body_bytes: 1024,
            max_retained_body_bytes: 8192,
            max_mapping_steps: 8192,
            max_live_snapshots: 8,
            max_snapshot_bytes: 16384,
        }),
        resources: Some(ResourceLimits {
            max_providers: 4,
            max_live_tasks: 16,
            max_issued_tasks: 32,
            max_queued_per_destination: 4,
            max_outstanding: 4,
            max_blob_bytes: 16384,
            max_key_bytes: 64,
            max_validator_bytes: 64,
            max_encoders: 4,
            max_live_artifacts: 16,
            max_issued_artifacts: 64,
            max_held_artifact_bytes: 65536,
            max_future_baseline_bytes: 65536,
            max_live_reads: 8,
            max_issued_reads: 32,
            max_read_chunks: 16,
            max_held_read_bytes: 1048576,
            max_restorers: 4,
            max_restoration_bytes: 8192,
        }),
    }
}
struct Fixture(std::path::PathBuf);
impl Fixture {
    fn new() -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "vellum-annotation-export-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match std::fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("fixture setup: {e}"),
            }
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for name in ["source", "annotations"] {
            let _ = std::fs::remove_file(self.0.join(name));
        }
        let _ = std::fs::remove_dir(&self.0);
    }
}
fn codec_limits() -> CodecLimits {
    CodecLimits {
        max_input_bytes: 16384,
        max_output_bytes: 16384,
        max_nodes: 2048,
        max_depth: 32,
        max_string_bytes: 16384,
        max_records: 16,
        max_integer_digits: 20,
        max_body_bytes: 1024,
        max_total_body_bytes: 8192,
        max_associations: 32,
        max_source_bytes: 8192,
    }
}
#[test]
fn real_annotation_export_writes_valid_json_without_touching_source_or_claiming_ordinary_saved_state()
 {
    exercise(false, false);
}
#[test]
fn real_associated_annotation_export_publishes_only_an_explicit_exact_historical_baseline() {
    exercise(true, false);
}
#[test]
fn real_ordinary_annotation_save_installs_its_prepaid_native_baseline_without_a_second_budget() {
    exercise(true, true);
}
fn exercise(associated: bool, ordinary: bool) {
    for capacity in [None, Some(512), Some(1)] {
        let fixture = Fixture::new();
        let source_path = fixture.0.join("source");
        let annotation_path = fixture.0.join("annotations");
        let original = "\u{feff}AÄ\r\n".as_bytes();
        std::fs::write(&source_path, original).unwrap();
        std::fs::write(&annotation_path, b"old destination").unwrap();
        let mut recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let mut owner = SourceCoordinator::configured(engine_config());
        let provider = owner
            .register_provider(
                ProviderProfile {
                    synchronization_supported: true,
                },
                None,
                &mut recorder,
            )
            .unwrap();
        let mut filesystem = LocalFilesystem::new(
            provider,
            FilesystemLimits {
                max_resources: 8,
                max_bytes: 16384,
                max_stage_attempts: 16,
                max_metadata_bytes: 4096,
            },
        );
        let (_, read) = filesystem.open(&source_path, &mut recorder).unwrap();
        let source = owner.load_resource(read, &mut recorder).unwrap();
        let (destination, destination_read) =
            filesystem.open(&annotation_path, &mut recorder).unwrap();
        if associated {
            let plan = owner
                .prepare_annotation_binding(&source.document, destination_read, &mut recorder)
                .unwrap();
            let proof = filesystem
                .compare_ordinary_pair(plan.pair_check(), &mut recorder)
                .unwrap();
            owner
                .activate_annotation_binding(plan, Some(proof), &mut recorder)
                .unwrap();
        }
        let targets = [
            AttachmentTarget::ContentRange { start: 0, end: 3 },
            AttachmentTarget::Document,
            AttachmentTarget::Point {
                offset: 1,
                affinity: vellum_engine::attachment::Affinity::Before,
                profile: vellum_engine::attachment::PointProfile::Position,
            },
            AttachmentTarget::Point {
                offset: 1,
                affinity: vellum_engine::attachment::Affinity::After,
                profile: vellum_engine::attachment::PointProfile::Content,
            },
        ];
        let creates: Vec<_> = targets
            .into_iter()
            .map(|target| AnnotationMutation::Create {
                body: "PRIVATE_REVIEW".into(),
                target: SourceTarget {
                    source: source.clone(),
                    basis: TargetBasis::Before,
                    target,
                },
            })
            .collect();
        let prepared = owner
            .prepare_compound(&[], &[], &creates, &mut recorder)
            .unwrap();
        let ids: Vec<_> = prepared.created_annotations().cloned().collect();
        owner.commit(prepared, &mut recorder).unwrap();
        let set = owner
            .annotation_set_version(&source.document, &mut recorder)
            .unwrap();
        let snapshot = owner
            .capture_annotations(&source, set, &mut recorder)
            .unwrap();
        let associations = CaptureAssociations {
            document: DocumentAssociation {
                native: source.document.clone(),
                identity: DocumentIdentity {
                    namespace: "host".into(),
                    value: "document".into(),
                },
            },
            records: ids
                .into_iter()
                .enumerate()
                .map(|(i, native)| RecordAssociation {
                    native,
                    identity: RecordIdentity {
                        namespace: "host".into(),
                        value: format!("comment-{i}"),
                        incarnation: i as u64 + 1,
                    },
                })
                .collect(),
            sources: vec![SourceAssociation {
                native: source.clone(),
                evidence: SourceEvidence {
                    domain: "canonical".into(),
                    token: "s0".into(),
                    byte_length: snapshot.source_text().len() as u64,
                },
                text: snapshot.source_text().into(),
            }],
        };
        let encoder = owner.register_annotation_encoder(&mut recorder).unwrap();
        let encoded = encode_capture(&snapshot, &associations, codec_limits(), &mut recorder)
            .unwrap()
            .into_encoded(&encoder)
            .unwrap();
        let (read, unused_probe) = filesystem.probe(&destination, &mut recorder).unwrap();
        drop(unused_probe);
        let requirements = SaveRequirements {
            protection: Protection::BestEffort,
            acknowledgment: Acknowledgment::Synchronized,
        };
        let (task, artifact) = if ordinary {
            let pair = current_pair(&owner, &filesystem, &source, &mut recorder);
            let task = owner
                .queue_annotation_save(encoded, read, requirements, pair, &mut recorder)
                .unwrap();
            let artifact = owner.save_outcome(&task, &mut recorder).unwrap().artifact;
            assert_eq!(owner.resource_usage().pending_baseline_publications, 1);
            (task, artifact)
        } else {
            let artifact = owner
                .retain_annotation_artifact(encoded, &mut recorder)
                .unwrap();
            let task = if associated {
                let pair = current_pair(&owner, &filesystem, &source, &mut recorder);
                owner.queue_bound_annotation_export(
                    &artifact,
                    read,
                    requirements,
                    pair,
                    &mut recorder,
                )
            } else {
                owner.queue_annotation_export(&artifact, read, requirements, &mut recorder)
            }
            .unwrap();
            (task, artifact)
        };
        let (read, write_probe) = filesystem.probe(&destination, &mut recorder).unwrap();
        let dispatch = if ordinary {
            let pair = current_pair(&owner, &filesystem, &source, &mut recorder).unwrap();
            owner.dispatch_save_with_pair(&task, read, pair, &mut recorder)
        } else {
            owner.dispatch_save(&task, read, &mut recorder)
        }
        .unwrap();
        let completion = filesystem
            .write(dispatch, write_probe, &mut recorder)
            .unwrap();
        assert_eq!(completion.failure, None);
        assert!(!completion.cleanup_failed);
        owner
            .record_save_evidence(completion.result, &mut recorder)
            .unwrap();
        owner
            .record_save_evidence(completion.no_future_effects, &mut recorder)
            .unwrap();
        owner
            .record_save_evidence(completion.quiescence, &mut recorder)
            .unwrap();
        let result = owner.save_outcome(&task, &mut recorder).unwrap();
        assert_eq!(result.application, Application::Written);
        assert_eq!(result.synchronization, Synchronization::Succeeded);
        assert_eq!(result.binding_generation, ordinary.then_some(1));
        assert_eq!(
            result.ordinary_annotation_generation,
            associated.then_some(1)
        );
        assert_eq!(result.encoding_context, Some(encoder.context()));
        assert_eq!(
            owner.publish_save_baseline(&task, &mut recorder),
            Err(ResourceError::Unsupported)
        );
        assert_eq!(std::fs::read(&source_path).unwrap(), original);
        assert_eq!(
            owner
                .saved_state(&source.document, &mut recorder)
                .unwrap()
                .content,
            ContentComparison::Unchanged
        );
        let bytes = std::fs::read(&annotation_path).unwrap();
        let context = DecodeContext {
            document: associations.document.identity.clone(),
            source: associations.sources[0].evidence.clone(),
            text: snapshot.source_text().into(),
        };
        assert_eq!(
            decode(&bytes, &context, codec_limits(), &mut recorder)
                .unwrap()
                .records[0]
                .body,
            "PRIVATE_REVIEW"
        );
        assert_eq!(
            owner.retire_resource_artifact(&artifact, &mut recorder),
            Err(ResourceError::Retained)
        );
        if associated {
            let pair = current_pair(&owner, &filesystem, &source, &mut recorder);
            assert!(
                owner
                    .annotation_baseline(&source.document, pair, &mut recorder)
                    .unwrap()
                    .is_none()
            );
            let pair = current_pair(&owner, &filesystem, &source, &mut recorder);
            let installed = if ordinary {
                owner.install_annotation_save_baseline(&task, pair, &mut recorder)
            } else {
                owner.publish_annotation_baseline(&task, pair, 65536, &mut recorder)
            }
            .unwrap();
            assert!(installed);
            assert_eq!(owner.resource_usage().pending_baseline_publications, 0);
            let current = encode_capture(&snapshot, &associations, codec_limits(), &mut recorder)
                .unwrap()
                .into_encoded(&encoder)
                .unwrap();
            let usage = owner.resource_usage();
            let generation = owner.generation();
            let pair = current_pair(&owner, &filesystem, &source, &mut recorder);
            assert_eq!(
                owner.compare_annotation_baseline(
                    &source.document,
                    &current,
                    pair,
                    32768,
                    &mut recorder
                ),
                Ok(ContentComparison::Unchanged)
            );
            assert_eq!(owner.resource_usage(), usage);
            assert_eq!(owner.generation(), generation);
            assert_eq!(
                owner.retire_save(&task, &mut recorder),
                Err(ResourceError::Retained)
            );
        } else {
            owner.retire_save(&task, &mut recorder).unwrap();
            owner
                .retire_resource_artifact(&artifact, &mut recorder)
                .unwrap();
            assert_eq!(owner.resource_usage().held_artifact_bytes, 0);
        }
        if associated {
            let generation = owner.generation();
            let pair = current_pair(&owner, &filesystem, &source, &mut recorder);
            let read = owner
                .admit_annotation_read(
                    &source,
                    set,
                    filesystem.annotation_read_target(&destination).unwrap(),
                    ReadStrength::CapturedRead,
                    pair,
                    &mut recorder,
                )
                .unwrap();
            let pair = current_pair(&owner, &filesystem, &source, &mut recorder);
            let dispatch = owner
                .submit_annotation_read(&read, pair, &mut recorder)
                .unwrap();
            let completion = filesystem
                .read_annotation(&dispatch, &mut recorder)
                .unwrap();
            assert_eq!(completion.failure, None);
            for fact in completion.receipts {
                owner.record_annotation_read(fact, &mut recorder).unwrap();
            }
            owner
                .record_annotation_read(completion.quiescence, &mut recorder)
                .unwrap();
            let pair = current_pair(&owner, &filesystem, &source, &mut recorder);
            let capture = owner
                .annotation_read_capture(&read, pair, &mut recorder)
                .unwrap();
            assert_eq!(capture.bytes(), bytes);
            let decoded = decode_read(capture, &context, codec_limits(), &mut recorder).unwrap();
            assert_eq!(decoded.snapshot().records[0].body, "PRIVATE_REVIEW");
            restore_in_fresh_owner(
                &source_path,
                &annotation_path,
                &associations,
                Retirement::Checkpointed,
                &mut recorder,
            );
            // The exact test-owned input may be valid but noncanonical JSON.
            // Semantic hydration must not silently mark canonical bytes saved.
            let mut padded = b" \n".to_vec();
            padded.extend_from_slice(&bytes);
            std::fs::write(&annotation_path, padded).unwrap();
            restore_in_fresh_owner(
                &source_path,
                &annotation_path,
                &associations,
                Retirement::Discard,
                &mut recorder,
            );
            restore_in_fresh_owner(
                &source_path,
                &annotation_path,
                &associations,
                Retirement::Disposed,
                &mut recorder,
            );
            drop(dispatch);
            assert_eq!(
                owner.retire_annotation_read(&read, &mut recorder),
                Err(ResourceError::Retained)
            );
            drop(decoded);
            owner.retire_annotation_read(&read, &mut recorder).unwrap();
            assert_eq!(owner.generation(), generation);
            assert_eq!(owner.resource_usage().held_read_bytes, 0);
            // Exact test-owned destination: invalid bytes remain input data, not
            // lossy text or a partially restored/native annotation collection.
            std::fs::write(&annotation_path, b"\xff").unwrap();
            let pair = current_pair(&owner, &filesystem, &source, &mut recorder);
            let read = owner
                .admit_annotation_read(
                    &source,
                    set,
                    filesystem.annotation_read_target(&destination).unwrap(),
                    ReadStrength::CapturedRead,
                    pair,
                    &mut recorder,
                )
                .unwrap();
            let pair = current_pair(&owner, &filesystem, &source, &mut recorder);
            let dispatch = owner
                .submit_annotation_read(&read, pair, &mut recorder)
                .unwrap();
            let completion = filesystem
                .read_annotation(&dispatch, &mut recorder)
                .unwrap();
            for fact in completion.receipts {
                owner.record_annotation_read(fact, &mut recorder).unwrap();
            }
            owner
                .record_annotation_read(completion.quiescence, &mut recorder)
                .unwrap();
            let pair = current_pair(&owner, &filesystem, &source, &mut recorder);
            let capture = owner
                .annotation_read_capture(&read, pair, &mut recorder)
                .unwrap();
            assert_eq!(capture.bytes(), b"\xff");
            assert!(decode_read(capture, &context, codec_limits(), &mut recorder).is_err());
            drop(dispatch);
            owner.retire_annotation_read(&read, &mut recorder).unwrap();
            assert_eq!(owner.generation(), generation);
            assert_eq!(
                owner
                    .annotation_set_version(&source.document, &mut recorder)
                    .unwrap(),
                set
            );
        }
        while let Some(record) = recorder.pop_front() {
            assert!(!format!("{record:?}").contains("PRIVATE_"));
        }
        if capacity == Some(1) {
            assert!(recorder.dropped() > 0);
        }
    }
}
fn current_pair(
    owner: &SourceCoordinator,
    filesystem: &LocalFilesystem,
    source: &DocumentVersion,
    recorder: &mut Recorder,
) -> Option<OrdinaryPairEvidence> {
    let check = owner
        .ordinary_pair_check(&source.document, recorder)
        .unwrap()
        .unwrap();
    Some(filesystem.compare_ordinary_pair(&check, recorder).unwrap())
}

enum Retirement {
    Checkpointed,
    Discard,
    Disposed,
}
fn restore_in_fresh_owner(
    source_path: &std::path::Path,
    annotation_path: &std::path::Path,
    original: &CaptureAssociations,
    retirement: Retirement,
    recorder: &mut Recorder,
) {
    let original_bytes = std::fs::read(annotation_path).unwrap();
    let mut owner = SourceCoordinator::configured(engine_config());
    let provider = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            None,
            recorder,
        )
        .unwrap();
    let mut fs = LocalFilesystem::new(
        provider.clone(),
        FilesystemLimits {
            max_resources: 8,
            max_bytes: 16384,
            max_stage_attempts: 16,
            max_metadata_bytes: 4096,
        },
    );
    let (_, read) = fs.open(source_path, recorder).unwrap();
    let source = owner.load_resource(read, recorder).unwrap();
    let (destination, read) = fs.open(annotation_path, recorder).unwrap();
    let plan = owner
        .prepare_annotation_binding(&source.document, read, recorder)
        .unwrap();
    let proof = fs
        .compare_ordinary_pair(plan.pair_check(), recorder)
        .unwrap();
    owner
        .activate_annotation_binding(plan, Some(proof), recorder)
        .unwrap();
    let witness = owner.capture_annotations(&source, 0, recorder).unwrap();
    let pair = current_pair(&owner, &fs, &source, recorder);
    let id = owner
        .admit_annotation_read(
            &source,
            0,
            fs.annotation_read_target(&destination).unwrap(),
            ReadStrength::CapturedRead,
            pair,
            recorder,
        )
        .unwrap();
    let pair = current_pair(&owner, &fs, &source, recorder);
    let dispatch = owner.submit_annotation_read(&id, pair, recorder).unwrap();
    let completion = fs.read_annotation(&dispatch, recorder).unwrap();
    assert_eq!(completion.failure, None);
    for fact in completion.receipts {
        owner.record_annotation_read(fact, recorder).unwrap();
    }
    owner
        .record_annotation_read(completion.quiescence, recorder)
        .unwrap();
    let pair = current_pair(&owner, &fs, &source, recorder);
    let capture = owner.annotation_read_capture(&id, pair, recorder).unwrap();
    let context = DecodeContext {
        document: original.document.identity.clone(),
        source: original.sources[0].evidence.clone(),
        text: witness.source_text().into(),
    };
    let decoded = decode_read(capture, &context, codec_limits(), recorder).unwrap();
    let mut approved = RestorationAssociations {
        document: DocumentAssociation {
            native: source.document.clone(),
            identity: original.document.identity.clone(),
        },
        records: original
            .records
            .iter()
            .map(|r| r.identity.clone())
            .collect(),
        sources: vec![RestorationSourceAssociation {
            witness,
            evidence: original.sources[0].evidence.clone(),
        }],
    };
    let encoder = owner.register_annotation_encoder(recorder).unwrap();
    let port = owner
        .register_annotation_restorer_for_encoding(&encoder.context(), recorder)
        .unwrap();
    approved.records[0].incarnation += 1;
    let unapproved = decode_read(
        decoded.capture().clone(),
        &context,
        codec_limits(),
        recorder,
    )
    .unwrap();
    assert!(matches!(
        prepare_restoration(unapproved, &approved, &port, codec_limits(), recorder),
        Err(RestorationBoundaryError::Codec(CodecError::Association))
    ));
    approved.records[0].incarnation -= 1;
    let restricted = decode_read(
        decoded.capture().clone(),
        &context,
        codec_limits(),
        recorder,
    )
    .unwrap();
    assert!(
        prepare_restoration(
            restricted,
            &approved,
            &port,
            CodecLimits {
                max_records: 0,
                ..codec_limits()
            },
            recorder
        )
        .is_err()
    );
    let package = prepare_restoration(decoded, &approved, &port, codec_limits(), recorder).unwrap();
    let pair = current_pair(&owner, &fs, &source, recorder);
    let outcome = owner.restore_annotations(package, pair, recorder).unwrap();
    assert!(outcome.changed);
    assert!(!outcome.loaded_baseline_adopted);
    owner.attach_annotation_read_baselines(recorder).unwrap();
    let pair = current_pair(&owner, &fs, &source, recorder);
    assert!(
        owner
            .adopt_loaded_annotation_baseline(&id, pair, 65536, recorder)
            .unwrap()
    );
    let pair = current_pair(&owner, &fs, &source, recorder);
    assert!(matches!(
        owner
            .annotation_baseline_evidence(&source.document, pair, recorder)
            .unwrap(),
        Some(AnnotationBaselineEvidence::Loaded(_))
    ));
    let imported = &outcome.annotations[0].annotation;
    assert!(
        owner
            .annotation(&original.records[0].native, recorder)
            .is_err()
    );
    let record = owner.annotation(imported, recorder).unwrap();
    assert_eq!(record.body, "PRIVATE_REVIEW");
    assert_eq!(
        record.restoration_identity.unwrap().incarnation(),
        original.records[0].identity.incarnation
    );
    let current = owner
        .capture_annotations(&source, outcome.set_version, recorder)
        .unwrap();
    let mut export = CaptureAssociations {
        document: approved.document.clone(),
        records: outcome
            .annotations
            .iter()
            .zip(&original.records)
            .map(|(native, original)| RecordAssociation {
                native: native.annotation.clone(),
                identity: original.identity.clone(),
            })
            .collect(),
        sources: vec![SourceAssociation {
            native: source.clone(),
            evidence: original.sources[0].evidence.clone(),
            text: current.source_text().into(),
        }],
    };
    let reencoded = encode_capture(&current, &export, codec_limits(), recorder).unwrap();
    let expected_comparison = if reencoded.bytes() == original_bytes {
        ContentComparison::Unchanged
    } else {
        ContentComparison::Modified
    };
    export.records[0].identity.incarnation += 1;
    assert!(matches!(
        encode_capture(&current, &export, codec_limits(), recorder),
        Err(CodecError::Association)
    ));
    export.records[0].identity.incarnation -= 1;
    let encoded = reencoded.into_encoded(&encoder).unwrap();
    let pair = current_pair(&owner, &fs, &source, recorder);
    assert_eq!(
        owner.compare_annotation_baseline(&source.document, &encoded, pair, 65536, recorder),
        Ok(expected_comparison)
    );
    if matches!(retirement, Retirement::Discard) {
        let source_bytes = std::fs::read(source_path).unwrap();
        let plan = owner
            .prepare_retained_disposal(&source, outcome.set_version, recorder)
            .unwrap();
        let state = owner.commit_retained_disposal(plan, recorder).unwrap();
        owner
            .release_disposed_source_binding(&source.document, state, recorder)
            .unwrap();
        let state = owner.lifecycle_state(&source.document, recorder).unwrap();
        owner
            .release_disposed_annotation_binding(&source.document, state, recorder)
            .unwrap();
        let state = owner.lifecycle_state(&source.document, recorder).unwrap();
        owner
            .release_disposed_provenance(&source.document, state, recorder)
            .unwrap();
        owner
            .record_annotation_read(
                provider
                    .annotation_read_start(&dispatch, b"late-contradiction")
                    .unwrap(),
                recorder,
            )
            .unwrap();
        drop(dispatch);
        let generation = owner.generation();
        owner
            .retire_disposed_annotation_read(&id, recorder)
            .unwrap();
        owner.detach_annotation_read_baselines(recorder).unwrap();
        assert_eq!(owner.generation(), generation);
        assert_eq!(owner.resource_usage().held_read_bytes, 0);
        assert_eq!(std::fs::read(annotation_path).unwrap(), original_bytes);
        assert_eq!(std::fs::read(source_path).unwrap(), source_bytes);
        return;
    }
    let (destination_read, unused) = fs.probe(&destination, recorder).unwrap();
    drop(unused);
    let pair = current_pair(&owner, &fs, &source, recorder);
    let save = owner
        .queue_annotation_save(
            encoded,
            destination_read,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Synchronized,
            },
            pair,
            recorder,
        )
        .unwrap();
    let (probe, write_probe) = fs.probe(&destination, recorder).unwrap();
    let pair = current_pair(&owner, &fs, &source, recorder).unwrap();
    let write = owner
        .dispatch_save_with_pair(&save, probe, pair, recorder)
        .unwrap();
    let complete = fs.write(write, write_probe, recorder).unwrap();
    assert_eq!(complete.failure, None);
    for fact in [
        complete.result,
        complete.no_future_effects,
        complete.quiescence,
    ] {
        owner.record_save_evidence(fact, recorder).unwrap();
    }
    let pair = current_pair(&owner, &fs, &source, recorder);
    owner
        .install_annotation_save_baseline(&save, pair, recorder)
        .unwrap();
    owner.stop_annotation_read(&id, recorder).unwrap();
    owner
        .record_annotation_read(
            provider
                .annotation_read_start(&dispatch, b"late-contradiction")
                .unwrap(),
            recorder,
        )
        .unwrap();
    assert!(
        owner
            .annotation_restoration_outcome(&id, recorder)
            .unwrap()
            .evidence_contested
    );
    assert_eq!(
        owner.annotation(imported, recorder).unwrap().body,
        "PRIVATE_REVIEW"
    );
    let pair = current_pair(&owner, &fs, &source, recorder);
    assert_eq!(
        owner
            .annotation_baseline(&source.document, pair, recorder)
            .unwrap()
            .unwrap()
            .producer,
        save
    );
    drop(dispatch);
    if matches!(retirement, Retirement::Disposed) {
        let written_bytes = std::fs::read(annotation_path).unwrap();
        let plan = owner
            .prepare_retained_disposal(&source, outcome.set_version, recorder)
            .unwrap();
        let state = owner.commit_retained_disposal(plan, recorder).unwrap();
        owner
            .release_disposed_annotation_binding(&source.document, state, recorder)
            .unwrap();
        let state = owner.lifecycle_state(&source.document, recorder).unwrap();
        owner
            .release_disposed_source_binding(&source.document, state, recorder)
            .unwrap();
        let state = owner.lifecycle_state(&source.document, recorder).unwrap();
        owner
            .release_disposed_provenance(&source.document, state, recorder)
            .unwrap();
        let generation = owner.generation();
        owner.retire_save(&save, recorder).unwrap();
        owner
            .retire_disposed_annotation_read(&id, recorder)
            .unwrap();
        owner.detach_annotation_read_baselines(recorder).unwrap();
        assert_eq!(owner.generation(), generation);
        assert_eq!(owner.resource_usage().held_read_bytes, 0);
        assert_eq!(owner.resource_usage().held_artifact_bytes, 0);
        assert_eq!(std::fs::read(annotation_path).unwrap(), written_bytes);
        assert_eq!(
            owner
                .lifecycle_state(&source.document, recorder)
                .unwrap()
                .phase,
            LifecyclePhase::ProvenanceReleased
        );
        return;
    }
    let generation = owner.generation();
    let writer = owner.save_outcome(&save, recorder).unwrap();
    owner
        .retire_superseded_annotation_read(&id, recorder)
        .unwrap();
    assert!(owner.detach_annotation_read_baselines(recorder).unwrap());
    assert_eq!(owner.resource_usage().held_read_bytes, 0);
    assert_eq!(owner.generation(), generation);
    assert_eq!(owner.save_outcome(&save, recorder).unwrap(), writer);
    let pair = current_pair(&owner, &fs, &source, recorder);
    assert!(matches!(
        owner
            .annotation_baseline_evidence(&source.document, pair, recorder)
            .unwrap(),
        Some(AnnotationBaselineEvidence::Written(_))
    ));
}
