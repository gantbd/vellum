//! Trusted scripted encoder/provider tests; native codec/filesystem tests are separate.
use vellum_engine::attachment::AttachmentTarget;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::resource::*;
use vellum_engine::source::Limits;
fn owner() -> SourceCoordinator {
    SourceCoordinator::configured(CoordinatorConfig {
        limits: CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
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
            max_blob_bytes: 4096,
            max_key_bytes: 64,
            max_validator_bytes: 64,
            max_encoders: 4,
            max_live_artifacts: 16,
            max_issued_artifacts: 64,
            max_held_artifact_bytes: 32768,
            max_future_baseline_bytes: 65536,
            max_live_reads: 8,
            max_issued_reads: 32,
            max_read_chunks: 16,
            max_held_read_bytes: 1048576,
            max_restorers: 4,
            max_restoration_bytes: 8192,
        }),
    })
}
fn requirements() -> SaveRequirements {
    SaveRequirements {
        protection: Protection::BestEffort,
        acknowledgment: Acknowledgment::Synchronized,
    }
}
fn artifact(
    owner: &mut SourceCoordinator,
    source: &DocumentVersion,
    port: &AnnotationEncoderPort,
    bytes: &[u8],
    recorder: &mut Recorder,
) -> ResourceArtifactId {
    let set = owner
        .annotation_set_version(&source.document, recorder)
        .unwrap();
    let snapshot = owner.capture_annotations(source, set, recorder).unwrap();
    let encoded = port.seal(snapshot, bytes.to_vec()).unwrap();
    owner.retain_annotation_artifact(encoded, recorder).unwrap()
}
#[test]
fn annotation_export_uses_original_artifact_and_never_publishes_a_source_baseline() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let provider = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            None,
            &mut recorder,
        )
        .unwrap();
    let source = owner
        .load_resource(
            provider.read_result(b"source", b"s0", b"A").unwrap(),
            &mut recorder,
        )
        .unwrap();
    let encoder = owner.register_annotation_encoder(&mut recorder).unwrap();
    let artifact = artifact(
        &mut owner,
        &source,
        &encoder,
        b"JSON snapshot",
        &mut recorder,
    );
    let task = owner
        .queue_annotation_export(
            &artifact,
            provider.read_result(b"annotations", b"a0", b"old").unwrap(),
            requirements(),
            &mut recorder,
        )
        .unwrap();
    let update = owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                body: "later".into(),
                target: SourceTarget {
                    source: source.clone(),
                    target: AttachmentTarget::Document,
                    basis: TargetBasis::Before,
                },
            }],
            &mut recorder,
        )
        .unwrap();
    owner.commit(update, &mut recorder).unwrap();
    let dispatch = owner
        .dispatch_save(
            &task,
            provider.read_result(b"annotations", b"a0", b"old").unwrap(),
            &mut recorder,
        )
        .unwrap();
    assert_eq!(dispatch.bytes(), b"JSON snapshot");
    let receipt = provider
        .result(
            &dispatch,
            Application::Written,
            dispatch.bytes().len(),
            Synchronization::Succeeded,
            Some(b"a1"),
        )
        .unwrap();
    owner.record_save_evidence(receipt, &mut recorder).unwrap();
    let result = owner.save_outcome(&task, &mut recorder).unwrap();
    assert_eq!(result.kind, ArtifactKind::Annotations);
    assert_eq!(result.purpose, SavePurpose::AnnotationExport);
    assert_eq!(result.captured_annotation_set, Some(0));
    assert_eq!(
        owner.publish_save_baseline(&task, &mut recorder),
        Err(ResourceError::Unsupported)
    );
    assert_eq!(
        owner
            .saved_state(&source.document, &mut recorder)
            .unwrap()
            .content,
        ContentComparison::Unchanged
    );
}
#[test]
fn source_and_annotation_effects_share_one_destination_barrier_even_for_explicit_overwrite_export()
{
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let provider = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            None,
            &mut recorder,
        )
        .unwrap();
    let source = owner
        .load_resource(
            provider.read_result(b"same", b"v0", b"A").unwrap(),
            &mut recorder,
        )
        .unwrap();
    let encoder = owner.register_annotation_encoder(&mut recorder).unwrap();
    let artifact = artifact(&mut owner, &source, &encoder, b"JSON", &mut recorder);
    let export = owner
        .queue_annotation_export(
            &artifact,
            provider.read_result(b"same", b"v0", b"A").unwrap(),
            requirements(),
            &mut recorder,
        )
        .unwrap();
    let source_save = owner
        .queue_pinned_save(&source, requirements(), &mut recorder)
        .unwrap();
    let dispatch = owner
        .dispatch_save(
            &export,
            provider.read_result(b"same", b"v0", b"A").unwrap(),
            &mut recorder,
        )
        .unwrap();
    assert!(matches!(
        owner.dispatch_save(
            &source_save,
            provider.read_result(b"same", b"v0", b"A").unwrap(),
            &mut recorder
        ),
        Err(ResourceError::Blocked)
    ));
    owner
        .record_save_evidence(provider.quiescence(&dispatch).unwrap(), &mut recorder)
        .unwrap();
    assert!(matches!(
        owner.dispatch_save(
            &source_save,
            provider.read_result(b"same", b"v0", b"A").unwrap(),
            &mut recorder
        ),
        Err(ResourceError::Blocked)
    ));
    owner
        .record_save_evidence(
            provider
                .result(
                    &dispatch,
                    Application::NotWritten,
                    0,
                    Synchronization::NotRequested,
                    None,
                )
                .unwrap(),
            &mut recorder,
        )
        .unwrap();
    owner
        .record_save_evidence(
            provider.no_future_effects(&dispatch).unwrap(),
            &mut recorder,
        )
        .unwrap();
    assert!(
        owner
            .dispatch_save(
                &source_save,
                provider.read_result(b"same", b"v0", b"A").unwrap(),
                &mut recorder
            )
            .is_ok()
    );
}
#[test]
fn stopped_delivery_does_not_recall_copies_cancel_admitted_effects_or_refund_held_bytes() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"A", &mut recorder).unwrap();
    let encoder = owner.register_annotation_encoder(&mut recorder).unwrap();
    let artifact = artifact(
        &mut owner,
        &source,
        &encoder,
        b"PRIVATE_JSON",
        &mut recorder,
    );
    let copy = owner
        .resource_artifact_bytes(&artifact, 0, 7, &mut recorder)
        .unwrap();
    let usage = owner.resource_usage();
    owner
        .stop_resource_artifact(&artifact, &mut recorder)
        .unwrap();
    assert!(
        owner
            .resource_artifact_bytes(&artifact, 0, 1, &mut recorder)
            .is_err()
    );
    assert_eq!(copy, b"PRIVATE");
    assert_eq!(
        owner.resource_usage().held_artifact_bytes,
        usage.held_artifact_bytes
    );
    owner
        .retire_resource_artifact(&artifact, &mut recorder)
        .unwrap();
    assert_eq!(owner.resource_usage().held_artifact_bytes, 0);
    assert!(
        owner
            .resource_artifact_bytes(&artifact, 0, 1, &mut recorder)
            .is_err()
    );
    let next = artifact_from_same_snapshot(&mut owner, &source, &encoder, &mut recorder);
    assert_ne!(artifact, next);
}
fn artifact_from_same_snapshot(
    owner: &mut SourceCoordinator,
    source: &DocumentVersion,
    encoder: &AnnotationEncoderPort,
    recorder: &mut Recorder,
) -> ResourceArtifactId {
    artifact(owner, source, encoder, b"new", recorder)
}
#[test]
fn retained_tasks_pin_artifact_bytes_until_effect_and_delivery_claims_are_discharged() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"A", &mut recorder).unwrap();
    let encoder = owner.register_annotation_encoder(&mut recorder).unwrap();
    let artifact = artifact(&mut owner, &source, &encoder, b"JSON", &mut recorder);
    let provider = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            None,
            &mut recorder,
        )
        .unwrap();
    let task = owner
        .queue_annotation_export(
            &artifact,
            provider.read_result(b"dest", b"v0", b"old").unwrap(),
            requirements(),
            &mut recorder,
        )
        .unwrap();
    owner
        .stop_resource_artifact(&artifact, &mut recorder)
        .unwrap();
    let dispatch = owner
        .dispatch_save(
            &task,
            provider.read_result(b"dest", b"v0", b"old").unwrap(),
            &mut recorder,
        )
        .unwrap();
    assert_eq!(
        owner.retire_resource_artifact(&artifact, &mut recorder),
        Err(ResourceError::Retained)
    );
    owner
        .record_save_evidence(
            provider
                .result(
                    &dispatch,
                    Application::Written,
                    4,
                    Synchronization::Succeeded,
                    Some(b"v1"),
                )
                .unwrap(),
            &mut recorder,
        )
        .unwrap();
    owner
        .record_save_evidence(
            provider.no_future_effects(&dispatch).unwrap(),
            &mut recorder,
        )
        .unwrap();
    owner
        .record_save_evidence(provider.quiescence(&dispatch).unwrap(), &mut recorder)
        .unwrap();
    assert_eq!(
        owner.retire_save(&task, &mut recorder),
        Err(ResourceError::Retained)
    );
    drop(dispatch);
    owner.retire_save(&task, &mut recorder).unwrap();
    owner
        .retire_resource_artifact(&artifact, &mut recorder)
        .unwrap();
}
#[test]
fn foreign_encoder_or_artifact_cannot_attach_to_another_owner() {
    let mut a = owner();
    let mut b = owner();
    let mut recorder = Recorder::disabled();
    let source = a.create(b"A", &mut recorder).unwrap();
    let a_encoder = a.register_annotation_encoder(&mut recorder).unwrap();
    let b_encoder = b.register_annotation_encoder(&mut recorder).unwrap();
    let snapshot = a.capture_annotations(&source, 0, &mut recorder).unwrap();
    assert!(b_encoder.seal(snapshot.clone(), b"JSON".to_vec()).is_err());
    let encoded = a_encoder.seal(snapshot, b"JSON".to_vec()).unwrap();
    assert!(
        b.retain_annotation_artifact(encoded, &mut recorder)
            .is_err()
    );
    let artifact = artifact(&mut a, &source, &a_encoder, b"JSON", &mut recorder);
    assert!(
        b.resource_artifact_bytes(&artifact, 0, 1, &mut recorder)
            .is_err()
    );
}

#[test]
fn source_and_annotation_artifacts_share_capacity_and_source_retirement_releases_only_its_own_bytes()
 {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let provider = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            None,
            &mut recorder,
        )
        .unwrap();
    let source = owner
        .load_resource(
            provider.read_result(b"source", b"v0", b"A").unwrap(),
            &mut recorder,
        )
        .unwrap();
    let encoder = owner.register_annotation_encoder(&mut recorder).unwrap();
    let snapshot = owner
        .capture_annotations(&source, 0, &mut recorder)
        .unwrap();
    let mut artifacts = Vec::new();
    for _ in 0..16 {
        artifacts.push(
            owner
                .retain_annotation_artifact(
                    encoder.seal(snapshot.clone(), b"JSON".to_vec()).unwrap(),
                    &mut recorder,
                )
                .unwrap(),
        );
    }
    let before = owner.resource_usage();
    assert_eq!(
        owner.queue_pinned_save(&source, requirements(), &mut recorder),
        Err(ResourceError::Limit)
    );
    assert_eq!(owner.resource_usage(), before);
    owner
        .retire_resource_artifact(&artifacts[0], &mut recorder)
        .unwrap();
    let task = owner
        .queue_pinned_save(&source, requirements(), &mut recorder)
        .unwrap();
    let outcome = owner.save_outcome(&task, &mut recorder).unwrap();
    assert_eq!(outcome.kind, ArtifactKind::Source);
    assert_eq!(outcome.binding_generation, Some(0));
    assert_eq!(outcome.encoding_context, None);
    assert_eq!(
        owner.queue_annotation_export(
            &outcome.artifact,
            provider.read_result(b"dest", b"d0", b"old").unwrap(),
            requirements(),
            &mut recorder
        ),
        Err(ResourceError::Unsupported)
    );
    owner.stop_save(&task, &mut recorder).unwrap();
    owner.retire_save(&task, &mut recorder).unwrap();
    assert_eq!(owner.resource_usage().held_artifact_bytes, 60);
    assert_eq!(owner.resource_usage().retained_artifacts, 15);
    assert_eq!(owner.resource_usage().issued_artifacts_lifetime, 17);
}

#[test]
fn a_later_qualified_annotation_export_neither_supersedes_nor_releases_the_source_baseline_producer()
 {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let provider = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            None,
            &mut recorder,
        )
        .unwrap();
    let source = owner
        .load_resource(
            provider.read_result(b"same", b"v0", b"A").unwrap(),
            &mut recorder,
        )
        .unwrap();
    let source_task = owner
        .queue_pinned_save(&source, requirements(), &mut recorder)
        .unwrap();
    let source_dispatch = owner
        .dispatch_save(
            &source_task,
            provider.read_result(b"same", b"v0", b"A").unwrap(),
            &mut recorder,
        )
        .unwrap();
    for receipt in [
        provider
            .result(
                &source_dispatch,
                Application::Written,
                1,
                Synchronization::Succeeded,
                Some(b"v1"),
            )
            .unwrap(),
        provider.no_future_effects(&source_dispatch).unwrap(),
        provider.quiescence(&source_dispatch).unwrap(),
    ] {
        owner.record_save_evidence(receipt, &mut recorder).unwrap();
    }
    drop(source_dispatch);
    let encoder = owner.register_annotation_encoder(&mut recorder).unwrap();
    let artifact = artifact(&mut owner, &source, &encoder, b"JSON", &mut recorder);
    let export = owner
        .queue_annotation_export(
            &artifact,
            provider.read_result(b"same", b"v1", b"A").unwrap(),
            requirements(),
            &mut recorder,
        )
        .unwrap();
    let dispatch = owner
        .dispatch_save(
            &export,
            provider.read_result(b"same", b"v1", b"A").unwrap(),
            &mut recorder,
        )
        .unwrap();
    owner
        .record_save_evidence(
            provider
                .result(
                    &dispatch,
                    Application::Written,
                    4,
                    Synchronization::Succeeded,
                    Some(b"v2"),
                )
                .unwrap(),
            &mut recorder,
        )
        .unwrap();
    assert_eq!(
        owner.retire_save(&source_task, &mut recorder),
        Err(ResourceError::Retained)
    );
    owner
        .publish_save_baseline(&source_task, &mut recorder)
        .unwrap();
    assert_eq!(
        owner.publish_save_baseline(&export, &mut recorder),
        Err(ResourceError::Unsupported)
    );
    assert_eq!(
        owner.retire_save(&source_task, &mut recorder),
        Err(ResourceError::Retained)
    );
}

#[test]
fn artifact_chunks_validate_exact_ranges_and_stopped_artifacts_cannot_admit_new_work() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"A", &mut recorder).unwrap();
    let encoder = owner.register_annotation_encoder(&mut recorder).unwrap();
    let artifact = artifact(
        &mut owner,
        &source,
        &encoder,
        "🙂".as_bytes(),
        &mut recorder,
    );
    assert_eq!(
        owner
            .resource_artifact_bytes(&artifact, 1, 1, &mut recorder)
            .unwrap(),
        vec![0x9f]
    );
    assert!(
        owner
            .resource_artifact_bytes(&artifact, 3, 2, &mut recorder)
            .is_err()
    );
    assert!(
        owner
            .resource_artifact_bytes(&artifact, usize::MAX, 1, &mut recorder)
            .is_err()
    );
    let provider = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            None,
            &mut recorder,
        )
        .unwrap();
    owner
        .stop_resource_artifact(&artifact, &mut recorder)
        .unwrap();
    let before = owner.resource_usage();
    assert!(
        owner
            .queue_annotation_export(
                &artifact,
                provider.read_result(b"dest", b"v0", b"old").unwrap(),
                requirements(),
                &mut recorder
            )
            .is_err()
    );
    assert_eq!(owner.resource_usage(), before);
}
