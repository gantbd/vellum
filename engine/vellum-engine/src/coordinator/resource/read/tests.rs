use super::*;

pub(in crate::coordinator::resource) fn original_source(
    o: &SourceCoordinator,
    id: &AnnotationReadId,
) -> DocumentVersion {
    o.read_task(id).unwrap().source.clone()
}

pub(in crate::coordinator::resource) fn release_checkpointed_root(
    owner: &mut SourceCoordinator,
    read: &AnnotationReadId,
    recorder: &mut Recorder,
) {
    let source = owner.read_task(read).unwrap().source.clone();
    let set = owner
        .document(&source.document)
        .unwrap()
        .annotation_generation;
    let plan = owner
        .prepare_retained_disposal(&source, set, recorder)
        .unwrap();
    let state = owner.commit_retained_disposal(plan, recorder).unwrap();
    owner
        .release_disposed_annotation_binding(&source.document, state, recorder)
        .unwrap();
    let state = owner.retained_document(&source.document).unwrap().lifecycle;
    owner
        .release_disposed_provenance(&source.document, state, recorder)
        .unwrap();
}

#[test]
fn disposed_read_and_write_control_survive_fault_counter_exhaustion_and_external_capture_pins() {
    let (mut owner, read, save, mut recorder) = checkpointed_read();
    release_checkpointed_root(&mut owner, &read, &mut recorder);
    let held = owner
        .read_task(&read)
        .unwrap()
        .restoration
        .as_ref()
        .unwrap()
        .package
        .capture
        .clone();
    owner.faulted = true;
    owner.generation = u64::MAX;
    let before = owner.resource_usage();
    assert_eq!(
        owner.retire_disposed_annotation_read(&read, &mut recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(owner.resource_usage(), before);
    drop(held);
    owner
        .retire_disposed_annotation_read(&read, &mut recorder)
        .unwrap();
    owner.retire_save(&save, &mut recorder).unwrap();
    owner
        .detach_annotation_read_baselines(&mut recorder)
        .unwrap();
    assert_eq!(owner.generation, u64::MAX);
    assert!(owner.faulted);
    assert_eq!(owner.resource_usage().held_read_bytes, 0);
    assert_eq!(owner.resource_usage().held_artifact_bytes, 0);
}

#[test]
fn a_known_peer_archived_read_reference_blocks_retirement_even_with_no_writer_tasks() {
    let (mut owner, read, save, mut recorder) = checkpointed_read();
    let baseline = owner.original_loaded_baseline(&read).unwrap();
    let peer = owner.create(b"peer", &mut recorder).unwrap();
    release_checkpointed_root(&mut owner, &read, &mut recorder);
    owner.retire_save(&save, &mut recorder).unwrap();
    let peer_owner = owner.documents[peer.document.slot].as_mut().unwrap();
    peer_owner.lifecycle.phase = LifecyclePhase::DisposedRetained;
    peer_owner.lifecycle.generation = 1;
    peer_owner.retained_bindings.annotations = Some(AnnotationBinding {
        document: peer.document.clone(),
        key: baseline.association.binding.clone(),
        baseline: Some(AnnotationBaselineSlot::Loaded(baseline)),
    });
    assert!(owner.resources.tasks.iter().all(Option::is_none));
    assert_eq!(
        owner.retire_disposed_annotation_read(&read, &mut recorder),
        Err(ResourceError::Retained)
    );
    let state = owner.retained_document(&peer.document).unwrap().lifecycle;
    owner
        .release_disposed_provenance(&peer.document, state, &mut recorder)
        .unwrap();
    owner
        .retire_disposed_annotation_read(&read, &mut recorder)
        .unwrap();
}

#[test]
fn disposed_retirement_accounting_or_lifecycle_drift_fences_without_partial_removal() {
    for fault in 0..6 {
        let (mut owner, read, save, mut recorder) = checkpointed_read();
        release_checkpointed_root(&mut owner, &read, &mut recorder);
        match fault {
            0 => owner.resources.held_read_bytes = 0,
            1 => owner.resources.held_artifact_bytes = 0,
            2 => {
                let source = owner.read_task(&read).unwrap().source.clone();
                owner.documents[source.document.slot]
                    .as_mut()
                    .unwrap()
                    .lifecycle
                    .generation = 0;
            }
            3 => {
                let artifact = owner.resource_task(&save).unwrap().artifact.clone();
                owner.resources.artifacts[artifact.slot]
                    .as_mut()
                    .unwrap()
                    .bytes = Blob(Arc::from(b"drift".as_slice()));
            }
            4 => {
                owner.resources.tasks[save.slot]
                    .as_mut()
                    .unwrap()
                    .captured_annotation_set = Some(u64::MAX)
            }
            _ => owner.resources.read_baseline_attachment = None,
        }
        let before = owner.resource_usage();
        let result = if matches!(fault, 1 | 3 | 4) {
            owner.retire_save(&save, &mut recorder)
        } else {
            owner.retire_disposed_annotation_read(&read, &mut recorder)
        };
        assert_eq!(result, Err(ResourceError::Faulted));
        assert!(owner.faulted);
        assert_eq!(owner.resource_usage(), before);
        assert!(owner.read_task(&read).is_ok());
        assert!(owner.resource_task(&save).is_ok());
    }
}

fn checkpointed_read() -> (SourceCoordinator, AnnotationReadId, SaveId, Recorder) {
    checkpointed_read_mode(false)
}
pub(in crate::coordinator::resource) fn checkpointed_read_mode(
    owned: bool,
) -> (SourceCoordinator, AnnotationReadId, SaveId, Recorder) {
    let (mut owner, port, id, mut recorder) = fixture_mode(owned);
    let dispatch = owner
        .submit_annotation_read(&id, None, &mut recorder)
        .unwrap();
    for evidence in [
        port.annotation_read_start(&dispatch, b"v0").unwrap(),
        port.annotation_read_end(&dispatch, b"v0", 0, 0).unwrap(),
        port.annotation_read_quiescence(&dispatch).unwrap(),
    ] {
        owner
            .record_annotation_read(evidence, &mut recorder)
            .unwrap();
    }
    let capture = owner
        .annotation_read_capture(&id, None, &mut recorder)
        .unwrap();
    let source = capture.source().clone();
    let witness = owner
        .capture_annotations(&source, 0, &mut recorder)
        .unwrap();
    let encoder = owner.register_annotation_encoder(&mut recorder).unwrap();
    let restorer = owner
        .register_annotation_restorer_for_encoding(&encoder.context(), &mut recorder)
        .unwrap();
    let package = restorer.seal(capture, vec![witness], vec![]).unwrap();
    owner
        .restore_annotations(package, None, &mut recorder)
        .unwrap();
    owner
        .attach_annotation_read_baselines(&mut recorder)
        .unwrap();
    owner
        .adopt_loaded_annotation_baseline(&id, None, 16384, &mut recorder)
        .unwrap();
    let current = owner
        .capture_annotations(&source, 0, &mut recorder)
        .unwrap();
    let encoded = encoder.seal(current, b"[]".to_vec()).unwrap();
    let save = owner
        .queue_annotation_save(
            encoded,
            port.read_result(b"comments", b"v0", b"").unwrap(),
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            None,
            &mut recorder,
        )
        .unwrap();
    let write = owner
        .dispatch_save(
            &save,
            port.read_result(b"comments", b"v0", b"").unwrap(),
            &mut recorder,
        )
        .unwrap();
    for evidence in [
        port.result(
            &write,
            Application::Written,
            2,
            Synchronization::NotRequested,
            Some(b"v1"),
        )
        .unwrap(),
        port.no_future_effects(&write).unwrap(),
        port.quiescence(&write).unwrap(),
    ] {
        owner.record_save_evidence(evidence, &mut recorder).unwrap();
    }
    owner
        .install_annotation_save_baseline(&save, None, &mut recorder)
        .unwrap();
    (owner, id, save, recorder)
}

#[test]
fn external_original_capture_blocks_discharge_and_native_fault_does_not_block_safe_control() {
    let (mut owner, id, save, mut recorder) = checkpointed_read();
    let held = owner
        .read_task(&id)
        .unwrap()
        .restoration
        .as_ref()
        .unwrap()
        .package
        .capture
        .clone();
    owner.faulted = true;
    owner.generation = u64::MAX;
    let before = owner.resource_usage();
    assert_eq!(
        owner.retire_superseded_annotation_read(&id, &mut recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(owner.resource_usage(), before);
    drop(held);
    owner
        .retire_superseded_annotation_read(&id, &mut recorder)
        .unwrap();
    assert!(
        owner
            .detach_annotation_read_baselines(&mut recorder)
            .unwrap()
    );
    assert_eq!(owner.generation, u64::MAX);
    assert!(owner.faulted);
    assert!(owner.resource_task(&save).unwrap().qualified());
    assert_eq!(owner.resource_usage().held_read_bytes, 0);
}

#[test]
fn discharge_accounting_underflow_fences_without_removing_provenance() {
    let (mut owner, id, _, mut recorder) = checkpointed_read();
    owner.resources.held_read_bytes = 0;
    assert_eq!(
        owner.retire_superseded_annotation_read(&id, &mut recorder),
        Err(ResourceError::Faulted)
    );
    assert!(owner.faulted);
    assert!(owner.read_task(&id).unwrap().restoration.is_some());
    assert_eq!(owner.resources.held_read_bytes, 0);
}

#[test]
fn attachment_reserves_both_endpoints_and_detach_preserves_unconsumed_reads_with_fresh_incarnation()
{
    let (mut owner, _, id, mut recorder) = fixture();
    let before = owner.resource_usage();
    owner
        .resources
        .limits
        .as_mut()
        .unwrap()
        .max_held_artifact_bytes = 0;
    assert_eq!(
        owner.attach_annotation_read_baselines(&mut recorder),
        Err(ResourceError::Limit)
    );
    assert_eq!(owner.resource_usage(), before);
    assert!(owner.resources.read_baseline_attachment.is_none());
    owner
        .resources
        .limits
        .as_mut()
        .unwrap()
        .max_held_artifact_bytes = 4096;
    owner
        .attach_annotation_read_baselines(&mut recorder)
        .unwrap();
    let old = owner
        .resources
        .read_baseline_attachment
        .as_ref()
        .unwrap()
        .reader
        .incarnation
        .clone();
    let generation = owner.generation;
    owner
        .detach_annotation_read_baselines(&mut recorder)
        .unwrap();
    assert_eq!(owner.resource_usage(), before);
    assert!(owner.read_task(&id).is_ok());
    owner
        .attach_annotation_read_baselines(&mut recorder)
        .unwrap();
    let new = &owner
        .resources
        .read_baseline_attachment
        .as_ref()
        .unwrap()
        .reader
        .incarnation;
    assert!(!Arc::ptr_eq(&old, new));
    assert_eq!(owner.generation, generation);
}

#[test]
fn inconsistent_attachment_or_detach_charge_fences_without_partial_unlink() {
    for half_link in [false, true] {
        let (mut owner, _, _, mut recorder) = fixture();
        owner
            .attach_annotation_read_baselines(&mut recorder)
            .unwrap();
        if half_link {
            owner
                .resources
                .read_baseline_attachment
                .as_mut()
                .unwrap()
                .writer
                .incarnation = Arc::new(());
        } else {
            owner.resources.held_artifact_bytes = 0;
        }
        let before = owner.resource_usage();
        assert_eq!(
            owner.detach_annotation_read_baselines(&mut recorder),
            Err(ResourceError::Faulted)
        );
        assert_eq!(owner.resource_usage(), before);
        assert!(owner.resources.read_baseline_attachment.is_some());
        assert!(owner.faulted);
    }
}

#[test]
fn read_baseline_attachment_is_bounded_idempotent_and_spends_no_origin_or_publication() {
    let (mut owner, _, _, mut recorder) = fixture();
    let generation = owner.generation;
    let before = owner.resource_usage();
    let held = owner.resources.held_read_bytes;
    owner.resources.limits.as_mut().unwrap().max_held_read_bytes = held;
    assert_eq!(
        owner.attach_annotation_read_baselines(&mut recorder),
        Err(ResourceError::Limit)
    );
    assert_eq!(owner.resource_usage(), before);
    owner.resources.limits.as_mut().unwrap().max_held_read_bytes =
        held + std::mem::size_of::<ReadBaselineEndpoint>();
    assert!(
        owner
            .attach_annotation_read_baselines(&mut recorder)
            .unwrap()
    );
    let attached = owner.resource_usage();
    assert!(
        !owner
            .attach_annotation_read_baselines(&mut recorder)
            .unwrap()
    );
    assert_eq!(owner.resource_usage(), attached);
    assert_eq!(owner.generation, generation);
}

#[test]
fn loaded_baseline_counter_refusal_is_atomic_and_identical_adoption_needs_no_counter() {
    let (mut owner, port, id, mut recorder) = fixture();
    let dispatch = owner
        .submit_annotation_read(&id, None, &mut recorder)
        .unwrap();
    for evidence in [
        port.annotation_read_start(&dispatch, b"v0").unwrap(),
        port.annotation_read_end(&dispatch, b"v0", 0, 0).unwrap(),
    ] {
        owner
            .record_annotation_read(evidence, &mut recorder)
            .unwrap();
    }
    let capture = owner
        .annotation_read_capture(&id, None, &mut recorder)
        .unwrap();
    let source = capture.source().clone();
    let encoder = owner.register_annotation_encoder(&mut recorder).unwrap();
    let boundary = owner
        .register_annotation_restorer_for_encoding(&encoder.context(), &mut recorder)
        .unwrap();
    let witness = owner
        .capture_annotations(&source, 0, &mut recorder)
        .unwrap();
    let package = boundary.seal(capture, vec![witness], vec![]).unwrap();
    owner
        .restore_annotations(package, None, &mut recorder)
        .unwrap();
    owner
        .attach_annotation_read_baselines(&mut recorder)
        .unwrap();
    owner.generation = u64::MAX;
    let before = owner.resource_usage();
    assert_eq!(
        owner.adopt_loaded_annotation_baseline(&id, None, 16384, &mut recorder),
        Err(ResourceError::VersionExhausted)
    );
    assert_eq!(owner.resource_usage(), before);
    assert!(
        !owner
            .annotation_restoration_outcome(&id, &mut recorder)
            .unwrap()
            .loaded_baseline_adopted
    );
    owner.generation -= 1;
    assert!(
        owner
            .adopt_loaded_annotation_baseline(&id, None, 16384, &mut recorder)
            .unwrap()
    );
    assert!(
        !owner
            .adopt_loaded_annotation_baseline(&id, None, 16384, &mut recorder)
            .unwrap()
    );
    assert_eq!(owner.generation, u64::MAX);
}
pub(in crate::coordinator::resource) fn fixture()
-> (SourceCoordinator, ProviderPort, AnnotationReadId, Recorder) {
    fixture_mode(false)
}
fn fixture_mode(owned: bool) -> (SourceCoordinator, ProviderPort, AnnotationReadId, Recorder) {
    let mut recorder = Recorder::disabled();
    let mut owner = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 4,
        max_participants: 4,
        max_staged_bytes: 4096,
        source: Limits {
            max_source_bytes: 128,
            max_edits: 4,
            max_edit_payload_bytes: 128,
        },
    });
    owner.resources = ResourceState::new(ResourceLimits {
        max_providers: 4,
        max_live_tasks: 4,
        max_issued_tasks: 8,
        max_queued_per_destination: 4,
        max_outstanding: 4,
        max_blob_bytes: 128,
        max_key_bytes: 64,
        max_validator_bytes: 64,
        max_encoders: 4,
        max_live_artifacts: 4,
        max_issued_artifacts: 8,
        max_held_artifact_bytes: 4096,
        max_future_baseline_bytes: 4096,
        max_live_reads: 4,
        max_issued_reads: 8,
        max_read_chunks: 4,
        max_held_read_bytes: 16384,
        max_restorers: 4,
        max_restoration_bytes: 8192,
    });
    owner.annotations.limits = Some(AnnotationLimits {
        max_issued: 8,
        max_records: 8,
        max_body_bytes: 128,
        max_retained_body_bytes: 1024,
        max_mapping_steps: 1024,
        max_live_snapshots: 8,
        max_snapshot_bytes: 4096,
    });
    let port = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut recorder,
        )
        .unwrap();
    let source = owner.create(b"source", &mut recorder).unwrap();
    let plan = owner
        .prepare_annotation_binding(
            &source.document,
            port.read_result(b"comments", b"v0", b"{}").unwrap(),
            &mut recorder,
        )
        .unwrap();
    owner
        .activate_annotation_binding(plan, None, &mut recorder)
        .unwrap();
    let id = if owned {
        owner
            .configure_retention_budget(2097152, &mut recorder)
            .unwrap();
        owner
            .configure_annotation_read_operations_v1(
                AnnotationReadOperationLimitsV1 {
                    max_capture_bytes: 8192,
                    max_reply_bytes: 1024,
                    max_label_bytes: 64,
                    max_store_bytes: 1048576,
                },
                &mut recorder,
            )
            .unwrap();
        operations::test_admit_owned_read(&mut owner, &source, &port, &mut recorder)
            .unwrap()
            .task
    } else {
        owner
            .admit_annotation_read(
                &source,
                0,
                port.annotation_read_target(b"comments").unwrap(),
                ReadStrength::CapturedRead,
                None,
                &mut recorder,
            )
            .unwrap()
    };
    (owner, port, id, recorder)
}
#[test]
fn native_fault_fences_new_reads_but_not_original_receipts_and_safe_control() {
    let (mut owner, port, id, mut recorder) = fixture();
    let dispatch = owner
        .submit_annotation_read(&id, None, &mut recorder)
        .unwrap();
    owner.faulted = true;
    owner
        .record_annotation_read(
            port.annotation_read_failure(&dispatch, ReadFailure::Interrupted)
                .unwrap(),
            &mut recorder,
        )
        .unwrap();
    owner
        .record_annotation_read(
            port.annotation_read_quiescence(&dispatch).unwrap(),
            &mut recorder,
        )
        .unwrap();
    assert!(matches!(
        owner.annotation_read_capture(&id, None, &mut recorder),
        Err(ResourceError::Faulted)
    ));
    owner.stop_annotation_read(&id, &mut recorder).unwrap();
    assert_eq!(
        owner.retire_annotation_read(&id, &mut recorder),
        Err(ResourceError::Retained)
    );
    drop(dispatch);
    owner.retire_annotation_read(&id, &mut recorder).unwrap();
    assert!(owner.faulted);
    assert_eq!(owner.resource_usage().held_read_bytes, 0);
}
#[test]
fn charged_read_retirement_underflow_fences_without_removing_retained_evidence() {
    let (mut owner, _, id, mut recorder) = fixture();
    owner.stop_annotation_read(&id, &mut recorder).unwrap();
    owner.resources.held_read_bytes = 0;
    assert_eq!(
        owner.retire_annotation_read(&id, &mut recorder),
        Err(ResourceError::Faulted)
    );
    assert!(owner.faulted);
    assert!(owner.resources.reads[id.slot].is_some());
}

#[test]
fn empty_native_restoration_needs_no_unused_generation_but_still_consumes_and_pins_its_read() {
    let (mut owner, port, id, mut recorder) = fixture();
    let dispatch = owner
        .submit_annotation_read(&id, None, &mut recorder)
        .unwrap();
    for evidence in [
        port.annotation_read_start(&dispatch, b"v0").unwrap(),
        port.annotation_read_end(&dispatch, b"v0", 0, 0).unwrap(),
        port.annotation_read_quiescence(&dispatch).unwrap(),
    ] {
        owner
            .record_annotation_read(evidence, &mut recorder)
            .unwrap();
    }
    let capture = owner
        .annotation_read_capture(&id, None, &mut recorder)
        .unwrap();
    let source = capture.source().clone();
    let boundary = owner.register_annotation_restorer(&mut recorder).unwrap();
    owner.generation = u64::MAX;
    let witness = owner
        .capture_annotations(&source, 0, &mut recorder)
        .unwrap();
    let invalid = boundary
        .seal(
            capture.clone(),
            vec![witness.clone()],
            vec![RestoredAnnotation {
                identity: boundary.identity(b"record", 1).unwrap(),
                body: "loaded".into(),
                content_version: 1,
                target_generation: 1,
                authored: SourceTarget {
                    source: source.clone(),
                    basis: TargetBasis::Before,
                    target: crate::attachment::AttachmentTarget::Document,
                },
                placement: crate::attachment::Placement::Detached,
            }],
        )
        .unwrap();
    assert!(matches!(
        owner.restore_annotations(invalid, None, &mut recorder),
        Err(ResourceError::VersionExhausted)
    ));
    assert_eq!(owner.annotation_usage().issued_lifetime, 0);
    assert!(
        !owner
            .annotation_read_status(&id, &mut recorder)
            .unwrap()
            .restored
    );
    let package = boundary.seal(capture, vec![witness], vec![]).unwrap();
    let outcome = owner
        .restore_annotations(package, None, &mut recorder)
        .unwrap();
    assert!(!outcome.changed);
    assert_eq!(owner.generation, u64::MAX);
    drop(dispatch);
    assert_eq!(
        owner.retire_annotation_read(&id, &mut recorder),
        Err(ResourceError::Retained)
    );
    assert!(
        owner
            .annotation_read_status(&id, &mut recorder)
            .unwrap()
            .restored
    );
}

#[test]
fn lifetime_ownership_drift_is_fenced_not_repaired_to_approve_a_load() {
    let (mut owner, port, id, mut recorder) = fixture();
    let dispatch = owner
        .submit_annotation_read(&id, None, &mut recorder)
        .unwrap();
    for evidence in [
        port.annotation_read_start(&dispatch, b"v0").unwrap(),
        port.annotation_read_end(&dispatch, b"v0", 0, 0).unwrap(),
    ] {
        owner
            .record_annotation_read(evidence, &mut recorder)
            .unwrap();
    }
    let capture = owner
        .annotation_read_capture(&id, None, &mut recorder)
        .unwrap();
    let source = capture.source().clone();
    let boundary = owner.register_annotation_restorer(&mut recorder).unwrap();
    let witness = owner
        .capture_annotations(&source, 0, &mut recorder)
        .unwrap();
    let package = boundary.seal(capture, vec![witness], vec![]).unwrap();
    // Simulate internal ledger drift, never a permissible restoration request.
    owner.annotations.records.push(None);
    let generation = owner.generation;
    assert!(matches!(
        owner.restore_annotations(package, None, &mut recorder),
        Err(ResourceError::Faulted)
    ));
    assert!(owner.faulted);
    assert_eq!(owner.generation, generation);
    assert_eq!(owner.annotations.records.len(), 1);
}
