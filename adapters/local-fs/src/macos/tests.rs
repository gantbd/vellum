use super::*;
use std::fs as stdfs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::sync::atomic::{AtomicU64, Ordering};
use vellum_engine::coordinator::{
    CoordinatorConfig, CoordinatorLimits, DocumentEdits, DocumentVersion, SourceCoordinator,
};
use vellum_engine::resource::{
    Acknowledgment, ContentComparison, Protection, ProviderProfile, ResourceLimits, SaveId,
    SaveRequirements,
};
use vellum_engine::source::{Limits, Replacement};

static FIXTURE: AtomicU64 = AtomicU64::new(0);

#[test]
fn observation_equality_is_exact_and_issuer_bound_not_a_freshness_or_authority_claim() {
    let mut recorder = Recorder::disabled();
    let (_, local) = configured(&mut recorder);
    let (_, foreign) = configured(&mut recorder);
    let original = local
        .port
        .as_ref()
        .unwrap()
        .read_result(b"key", b"v1", b"value")
        .unwrap();
    assert!(original.same_observation(&original.clone()));
    for other in [
        local
            .port
            .as_ref()
            .unwrap()
            .read_result(b"other", b"v1", b"value")
            .unwrap(),
        local
            .port
            .as_ref()
            .unwrap()
            .read_result(b"key", b"v2", b"value")
            .unwrap(),
        local
            .port
            .as_ref()
            .unwrap()
            .read_result(b"key", b"v1", b"changed")
            .unwrap(),
        foreign
            .port
            .as_ref()
            .unwrap()
            .read_result(b"key", b"v1", b"value")
            .unwrap(),
    ] {
        assert!(!original.same_observation(&other));
    }
}

#[test]
fn typed_causal_condition_is_refused_before_native_replacement_without_a_false_write_or_lost_receipts()
 {
    use vellum_engine::resource::{
        ProviderCausalCapabilitiesV1, ProviderDestinationCapabilitiesV1, ResourceVersionV1,
    };
    let directory = Directory::new();
    let path = directory.path();
    stdfs::write(&path, b"OLD").unwrap();
    let mut r = Recorder::disabled();
    let (mut owner, base) = configured(&mut r);
    // Deliberately advertised by the fixture host; this does not make the native
    // staged procedure implement causal observations or non-reused tokens.
    let port = owner
        .register_provider_with_causal_capabilities(
            ProviderProfile {
                synchronization_supported: false,
            },
            ProviderDestinationCapabilitiesV1::default(),
            ProviderCausalCapabilitiesV1 {
                causal_receipts: true,
                non_reuse_tokens: true,
            },
            None,
            &mut r,
        )
        .unwrap();
    let mut fs = LocalFilesystem::new(port.clone(), base.limits);
    let (handle, _) = fs.open(&path, &mut r).unwrap();
    let entry = fs.entry(&handle).unwrap();
    let snapshot = snapshot(&entry.directory, &entry.name, fs.limits.max_bytes).unwrap();
    let validator = snapshot.stamp.validator();
    let version = ResourceVersionV1 {
        epoch: b"fixture-epoch",
        token: &validator,
    };
    let key = (handle.slot as u64).to_le_bytes();
    let source = owner
        .load_resource(
            port.read_result_causal(&key, version, &snapshot.bytes)
                .unwrap(),
            &mut r,
        )
        .unwrap();
    let task = owner
        .queue_pinned_save(
            &source,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            &mut r,
        )
        .unwrap();
    let (_, probe) = fs.probe(&handle, &mut r).unwrap();
    let dispatch = owner
        .dispatch_save(
            &task,
            port.read_result_causal(&key, version, &snapshot.bytes)
                .unwrap(),
            &mut r,
        )
        .unwrap();
    let completion = fs.write(dispatch, probe, &mut r).unwrap();
    assert_eq!(
        completion.failure,
        Some(FilesystemError::Resource(ResourceError::Unsupported))
    );
    intake(&mut owner, completion, &mut r);
    let out = owner.save_outcome(&task, &mut r).unwrap();
    assert_eq!(out.application, Application::NotWritten);
    assert!(out.no_future_effects && out.quiescent);
    assert!(!out.causal_edge_available);
    assert_eq!(stdfs::read(&path).unwrap(), b"OLD");
    assert_eq!(directory.stage_count(), 0);
}

#[test]
fn scratch_copy_export_uses_real_staged_replacement_without_binding_the_scratch_document() {
    scratch_export(false);
}
#[test]
fn scratch_save_as_uses_real_write_receipts_and_a_fresh_native_probe_before_binding() {
    scratch_export(true);
}
fn scratch_export(save_as: bool) {
    use vellum_engine::coordinator::{TrustedSourceExportControlV1, TrustedSourceExportFactsV1};
    use vellum_engine::resource::{SourceExportOptionsV1, SourceExportPurposeV1};
    let directory = Directory::new();
    let path = directory.path();
    stdfs::write(&path, b"OLD").unwrap();
    stdfs::set_permissions(&path, stdfs::Permissions::from_mode(0o640)).unwrap();
    let mut r = Recorder::disabled();
    let (mut owner, mut fs) = configured(&mut r);
    let (handle, read) = fs.open(&path, &mut r).unwrap();
    let source = owner.create(b"\xef\xbb\xbf\xc3\xa9\r\n", &mut r).unwrap();
    let serialization = owner
        .source_serialization(&source.document, &mut r)
        .unwrap();
    let prepared = owner
        .prepare_source_artifact(&source, serialization, &mut r)
        .unwrap();
    // Scripted authorization for this isolated fixture; the actual destination
    // observation, stamp/probe, replacement and receipts use the native adapter.
    let facts = TrustedSourceExportFactsV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        source_readable: Some(true),
        write_allowed: Some(true),
        binding_coverage_complete: Some(true),
        writer_coverage_complete: Some(true),
        provider_available: Some(true),
        resources_ready: Some(true),
        launch_allowed: Some(true),
        destination_allowed: Some(true),
        identity_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        create_allowed: None,
        overwrite_allowed: Some(true),
    };
    let task = owner
        .queue_source_export(
            &prepared,
            read.into_source_export_destination(),
            SourceExportOptionsV1 {
                purpose: if save_as {
                    SourceExportPurposeV1::SaveAs
                } else {
                    SourceExportPurposeV1::CopyExport
                },
                requirements: SaveRequirements {
                    protection: Protection::BestEffort,
                    acknowledgment: Acknowledgment::Synchronized,
                },
                pair: None,
            },
            &facts,
            &mut r,
        )
        .unwrap();
    let (probe_read, probe) = fs.probe(&handle, &mut r).unwrap();
    let dispatch = owner
        .dispatch_source_export(
            &task,
            probe_read.into_source_export_destination(),
            None,
            &facts,
            &mut r,
        )
        .unwrap();
    let completion = fs.write(dispatch, probe, &mut r).unwrap();
    assert_eq!(completion.failure, None);
    intake(&mut owner, completion, &mut r);
    assert_eq!(stdfs::read(&path).unwrap(), b"\xef\xbb\xbf\xc3\xa9\r\n");
    assert_eq!(
        stdfs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(directory.stage_count(), 0);
    assert_eq!(
        owner.saved_state(&source.document, &mut r).unwrap().content,
        ContentComparison::Unbound
    );
    assert_eq!(
        owner.publish_save_baseline(&task, &mut r),
        Err(ResourceError::Unsupported)
    );
    let control = TrustedSourceExportControlV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        binding_coverage_complete: Some(true),
        writer_coverage_complete: Some(true),
        binding_activation_allowed: Some(true),
        identity_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        disposition_readable: Some(true),
        reservation_release_allowed: Some(true),
    };
    if save_as {
        let (fresh, _) = fs.probe(&handle, &mut r).unwrap();
        assert!(
            owner
                .activate_source_save_as(
                    &task,
                    fresh.into_source_save_as_probe(),
                    None,
                    0,
                    &vellum_engine::coordinator::TrustedSourceSaveAsFactsV1 {
                        control,
                        probe_attested: Some(true),
                        resource_continuity_attested: Some(true),
                        create_allowed: None,
                        overwrite_allowed: Some(true),
                    },
                    &mut r
                )
                .unwrap()
        );
        assert_eq!(
            owner.saved_state(&source.document, &mut r).unwrap().content,
            ContentComparison::Unchanged
        );
        assert_eq!(
            owner.retire_save(&task, &mut r),
            Err(ResourceError::Retained)
        );
        assert_eq!(owner.resource_usage().source_binding_history_entries, 0);
    } else {
        owner
            .release_source_export_reservation(&task, &control, &mut r)
            .unwrap();
        owner.retire_save(&task, &mut r).unwrap();
        assert_eq!(owner.resource_usage().held_artifact_bytes, 0);
    }
}
struct Directory(std::path::PathBuf);
impl Directory {
    fn new() -> Self {
        let root = std::env::temp_dir();
        loop {
            let path = root.join(format!(
                "vellum-fs-test-{}-{}",
                std::process::id(),
                FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            match stdfs::create_dir(&path) {
                Ok(()) => return Self(path.canonicalize().expect("fixture canonicalization")),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("fixture creation: {e}"),
            }
        }
    }
    fn path(&self) -> std::path::PathBuf {
        self.0.join("source")
    }
    fn stage_count(&self) -> usize {
        stdfs::read_dir(&self.0)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(".vellum-stage-")
            })
            .count()
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        // Exact exclusively created fixture directory, no recursive traversal.
        for entry in stdfs::read_dir(&self.0).unwrap() {
            let _ = stdfs::remove_file(entry.unwrap().path());
        }
        let _ = stdfs::remove_dir(&self.0);
    }
}
fn configured(recorder: &mut Recorder) -> (SourceCoordinator, LocalFilesystem) {
    let mut owner = SourceCoordinator::configured(CoordinatorConfig {
        search: None,
        annotations: None,
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
        resources: Some(ResourceLimits {
            max_providers: 4,
            max_live_tasks: 16,
            max_issued_tasks: 32,
            max_queued_per_destination: 4,
            max_outstanding: 4,
            max_blob_bytes: 8192,
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
    });
    let port = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            None,
            recorder,
        )
        .unwrap();
    let filesystem = LocalFilesystem::new(
        port,
        FilesystemLimits {
            max_resources: 8,
            max_bytes: 4096,
            max_stage_attempts: 16,
            max_metadata_bytes: 4096,
        },
    );
    (owner, filesystem)
}
fn edited(
    owner: &mut SourceCoordinator,
    expected: DocumentVersion,
    recorder: &mut Recorder,
) -> DocumentVersion {
    let preparation = owner
        .prepare(
            &[DocumentEdits {
                expected: expected.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: "Ä changed".into(),
                }],
            }],
            &[],
            recorder,
        )
        .unwrap();
    owner.commit(preparation, recorder).unwrap();
    owner.read(&expected.document, recorder).unwrap().version
}
fn queued(
    owner: &mut SourceCoordinator,
    version: &DocumentVersion,
    recorder: &mut Recorder,
) -> SaveId {
    owner
        .queue_pinned_save(
            version,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Synchronized,
            },
            recorder,
        )
        .unwrap()
}
fn launch(
    owner: &mut SourceCoordinator,
    filesystem: &LocalFilesystem,
    handle: &ResourceHandle,
    task: &SaveId,
    recorder: &mut Recorder,
) -> (SaveDispatch, WriteProbe) {
    let (read, probe) = filesystem.probe(handle, recorder).unwrap();
    (owner.dispatch_save(task, read, recorder).unwrap(), probe)
}
fn intake(owner: &mut SourceCoordinator, completion: WriteCompletion, recorder: &mut Recorder) {
    assert!(!completion.cleanup_failed);
    owner
        .record_save_evidence(completion.result, recorder)
        .unwrap();
    owner
        .record_save_evidence(completion.no_future_effects, recorder)
        .unwrap();
    owner
        .record_save_evidence(completion.quiescence, recorder)
        .unwrap();
}

#[test]
fn ordinary_pair_allows_source_save_without_changing_the_annotation_resource() {
    let directory = Directory::new();
    let annotations = directory.0.join("comments");
    stdfs::write(directory.path(), b"A").unwrap();
    stdfs::write(&annotations, b"{}").unwrap();
    let mut recorder = Recorder::try_with_capacity(128).unwrap();
    let (mut owner, mut filesystem) = configured(&mut recorder);
    let (handle, read) = filesystem.open(&directory.path(), &mut recorder).unwrap();
    let source = owner.load_resource(read, &mut recorder).unwrap();
    let (_, destination) = filesystem.open(&annotations, &mut recorder).unwrap();
    let plan = owner
        .prepare_annotation_binding(&source.document, destination, &mut recorder)
        .unwrap();
    let proof = filesystem
        .compare_ordinary_pair(plan.pair_check(), &mut recorder)
        .unwrap();
    owner
        .activate_annotation_binding(plan, Some(proof), &mut recorder)
        .unwrap();
    let version = edited(&mut owner, source.clone(), &mut recorder);
    let check = owner
        .ordinary_pair_check(&source.document, &mut recorder)
        .unwrap()
        .unwrap();
    let proof = filesystem
        .compare_ordinary_pair(&check, &mut recorder)
        .unwrap();
    let task = owner
        .queue_pinned_save_with_pair(
            &version,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Synchronized,
            },
            proof,
            &mut recorder,
        )
        .unwrap();
    let check = owner
        .ordinary_pair_check(&source.document, &mut recorder)
        .unwrap()
        .unwrap();
    let proof = filesystem
        .compare_ordinary_pair(&check, &mut recorder)
        .unwrap();
    let (read, probe) = filesystem.probe(&handle, &mut recorder).unwrap();
    let dispatch = owner
        .dispatch_save_with_pair(&task, read, proof, &mut recorder)
        .unwrap();
    let completion = filesystem.write(dispatch, probe, &mut recorder).unwrap();
    assert_eq!(completion.failure, None);
    intake(&mut owner, completion, &mut recorder);
    let check = owner
        .ordinary_pair_check(&source.document, &mut recorder)
        .unwrap()
        .unwrap();
    let proof = filesystem
        .compare_ordinary_pair(&check, &mut recorder)
        .unwrap();
    owner
        .publish_save_baseline_with_pair(&task, proof, &mut recorder)
        .unwrap();
    assert_eq!(
        owner
            .saved_state(&source.document, &mut recorder)
            .unwrap()
            .content,
        ContentComparison::Unchanged
    );
    assert_eq!(
        stdfs::read(directory.path()).unwrap(),
        "Ä changed".as_bytes()
    );
    assert_eq!(stdfs::read(annotations).unwrap(), b"{}");
}

#[test]
fn ordinary_pair_uses_native_identity_and_rechecks_alias_changes_before_dispatch() {
    for capacity in [None, Some(128), Some(1)] {
        let directory = Directory::new();
        let annotation_path = directory.0.join("comments");
        stdfs::write(directory.path(), b"A").unwrap();
        stdfs::write(&annotation_path, b"{}").unwrap();
        let mut recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let (mut owner, mut filesystem) = configured(&mut recorder);
        let (source_handle, read) = filesystem.open(&directory.path(), &mut recorder).unwrap();
        let source = owner.load_resource(read, &mut recorder).unwrap();
        let (_, destination) = filesystem.open(&annotation_path, &mut recorder).unwrap();
        let plan = owner
            .prepare_annotation_binding(&source.document, destination, &mut recorder)
            .unwrap();
        let proof = filesystem
            .compare_ordinary_pair(plan.pair_check(), &mut recorder)
            .unwrap();
        owner
            .activate_annotation_binding(plan, Some(proof), &mut recorder)
            .unwrap();
        let check = owner
            .ordinary_pair_check(&source.document, &mut recorder)
            .unwrap()
            .unwrap();
        let proof = filesystem
            .compare_ordinary_pair(&check, &mut recorder)
            .unwrap();
        let task = owner
            .queue_pinned_save_with_pair(
                &source,
                SaveRequirements {
                    protection: Protection::BestEffort,
                    acknowledgment: Acknowledgment::Synchronized,
                },
                proof,
                &mut recorder,
            )
            .unwrap();
        // Exact test-owned file only; change the physical alias after binding.
        stdfs::remove_file(&annotation_path).unwrap();
        stdfs::hard_link(directory.path(), &annotation_path).unwrap();
        let check = owner
            .ordinary_pair_check(&source.document, &mut recorder)
            .unwrap()
            .unwrap();
        let proof = filesystem
            .compare_ordinary_pair(&check, &mut recorder)
            .unwrap();
        let (read, _) = filesystem.probe(&source_handle, &mut recorder).unwrap();
        assert!(matches!(
            owner.dispatch_save_with_pair(&task, read, proof, &mut recorder),
            Err(ResourceError::Conflict)
        ));
        assert_eq!(stdfs::read(directory.path()).unwrap(), b"A");
        assert_eq!(directory.stage_count(), 0);
        stdfs::remove_file(&annotation_path).unwrap();
        stdfs::write(&annotation_path, b"{}").unwrap();
        let check = owner
            .ordinary_pair_check(&source.document, &mut recorder)
            .unwrap()
            .unwrap();
        let proof = filesystem
            .compare_ordinary_pair(&check, &mut recorder)
            .unwrap();
        let (read, probe) = filesystem.probe(&source_handle, &mut recorder).unwrap();
        // Linking changed the source's ctime, so the original pinned condition
        // still refuses. Pair distinction cannot authorize token refresh.
        assert!(matches!(
            owner.dispatch_save_with_pair(&task, read, proof, &mut recorder),
            Err(ResourceError::Conflict)
        ));
        drop(probe);
        assert_eq!(stdfs::read(&annotation_path).unwrap(), b"{}");
        if capacity == Some(1) {
            assert!(recorder.dropped() > 0);
        }
    }
}

#[test]
fn given_plain_file_when_edited_and_saved_then_staged_replacement_preserves_bytes_and_permissions()
{
    let directory = Directory::new();
    stdfs::write(directory.path(), b"\xef\xbb\xbfA\r\n").unwrap();
    stdfs::set_permissions(directory.path(), stdfs::Permissions::from_mode(0o640)).unwrap();
    let original = stdfs::metadata(directory.path()).unwrap();
    let mut telemetry = Recorder::try_with_capacity(128).unwrap();
    let (mut owner, mut filesystem) = configured(&mut telemetry);
    let (handle, read) = filesystem.open(&directory.path(), &mut telemetry).unwrap();
    let loaded = owner.load_resource(read, &mut telemetry).unwrap();
    let version = edited(&mut owner, loaded.clone(), &mut telemetry);
    let task = queued(&mut owner, &version, &mut telemetry);
    let (dispatch, probe) = launch(&mut owner, &filesystem, &handle, &task, &mut telemetry);
    let completion = filesystem.write(dispatch, probe, &mut telemetry).unwrap();
    assert_eq!(completion.failure, None);
    intake(&mut owner, completion, &mut telemetry);
    owner.publish_save_baseline(&task, &mut telemetry).unwrap();
    assert_eq!(
        stdfs::read(directory.path()).unwrap(),
        "\u{feff}Ä changed\r\n".as_bytes()
    );
    let saved = stdfs::metadata(directory.path()).unwrap();
    assert_ne!(saved.ino(), original.ino());
    assert_eq!(
        (saved.mode(), saved.uid(), saved.gid()),
        (original.mode(), original.uid(), original.gid())
    );
    assert_eq!(directory.stage_count(), 0);
    assert_eq!(
        owner
            .saved_state(&loaded.document, &mut telemetry)
            .unwrap()
            .content,
        ContentComparison::Unchanged
    );
    let (again, read) = filesystem.open(&directory.path(), &mut telemetry).unwrap();
    assert_eq!(handle.slot, again.slot);
    assert_eq!(
        owner.load_resource(read, &mut telemetry).unwrap().document,
        loaded.document
    );
    // A subsequent ordinary capture/probe recognizes the new inode's baseline.
    let task2 = queued(&mut owner, &version, &mut telemetry);
    let (dispatch, probe) = launch(&mut owner, &filesystem, &handle, &task2, &mut telemetry);
    intake(
        &mut owner,
        filesystem.write(dispatch, probe, &mut telemetry).unwrap(),
        &mut telemetry,
    );
    owner.publish_save_baseline(&task2, &mut telemetry).unwrap();
    let mut phases = Vec::new();
    while let Some(record) = telemetry.pop_front() {
        phases.push(record.boundary);
    }
    for phase in [
        Boundary::SubstrateRead,
        Boundary::SubstrateValidation,
        Boundary::SubstrateStaging,
        Boundary::SubstrateReplacement,
        Boundary::SubstrateSynchronization,
        Boundary::SubstrateWrite,
    ] {
        assert!(phases.contains(&phase));
    }
}

#[test]
fn given_a_hardlink_when_saved_then_no_alias_is_split_or_overwritten() {
    let directory = Directory::new();
    stdfs::write(directory.path(), b"A").unwrap();
    let alias = directory.0.join("alias");
    stdfs::hard_link(directory.path(), &alias).unwrap();
    let mut telemetry = Recorder::disabled();
    let (mut owner, mut filesystem) = configured(&mut telemetry);
    let (handle, read) = filesystem.open(&directory.path(), &mut telemetry).unwrap();
    let loaded = owner.load_resource(read, &mut telemetry).unwrap();
    let (_, aliased_read) = filesystem.open(&alias, &mut telemetry).unwrap();
    assert_eq!(
        owner
            .load_resource(aliased_read, &mut telemetry)
            .unwrap()
            .document,
        loaded.document
    );
    let version = edited(&mut owner, loaded, &mut telemetry);
    let task = queued(&mut owner, &version, &mut telemetry);
    let (dispatch, probe) = launch(&mut owner, &filesystem, &handle, &task, &mut telemetry);
    let completion = filesystem.write(dispatch, probe, &mut telemetry).unwrap();
    assert_eq!(completion.failure, Some(FilesystemError::UnsupportedFile));
    intake(&mut owner, completion, &mut telemetry);
    assert_eq!(
        owner
            .save_outcome(&task, &mut telemetry)
            .unwrap()
            .application,
        Application::NotWritten
    );
    assert_eq!(stdfs::read(directory.path()).unwrap(), b"A");
    assert_eq!(stdfs::read(alias).unwrap(), b"A");
}

#[test]
fn given_symlink_or_directory_when_opened_then_the_adapter_refuses_instead_of_following() {
    let directory = Directory::new();
    stdfs::write(directory.path(), b"A").unwrap();
    let alias = directory.0.join("link");
    symlink(directory.path(), &alias).unwrap();
    let mut telemetry = Recorder::disabled();
    let (_, mut filesystem) = configured(&mut telemetry);
    assert!(filesystem.open(&alias, &mut telemetry).is_err());
    assert!(filesystem.open(&directory.0, &mut telemetry).is_err());
}

#[test]
fn given_xattrs_when_saved_then_all_attribute_values_survive_replacement() {
    let directory = Directory::new();
    stdfs::write(directory.path(), b"A").unwrap();
    let file = File::open(directory.path()).unwrap();
    fs::fsetxattr(
        &file,
        "org.vellum.test",
        b"PRIVATE_ATTRIBUTE",
        fs::XattrFlags::empty(),
    )
    .unwrap();
    let mut telemetry = Recorder::disabled();
    let (mut owner, mut filesystem) = configured(&mut telemetry);
    let (handle, read) = filesystem.open(&directory.path(), &mut telemetry).unwrap();
    let version = owner.load_resource(read, &mut telemetry).unwrap();
    let version = edited(&mut owner, version, &mut telemetry);
    let task = queued(&mut owner, &version, &mut telemetry);
    let (dispatch, probe) = launch(&mut owner, &filesystem, &handle, &task, &mut telemetry);
    let completion = filesystem.write(dispatch, probe, &mut telemetry).unwrap();
    assert_eq!(completion.failure, None);
    intake(&mut owner, completion, &mut telemetry);
    let replaced = File::open(directory.path()).unwrap();
    let mut bytes = [0; 64];
    let len = fs::fgetxattr(&replaced, "org.vellum.test", &mut bytes[..]).unwrap();
    assert_eq!(&bytes[..len], b"PRIVATE_ATTRIBUTE");
}

fn assert_refused_metadata(directory: &Directory, expected: FilesystemError) {
    let mut telemetry = Recorder::disabled();
    let (mut owner, mut filesystem) = configured(&mut telemetry);
    let (handle, read) = filesystem.open(&directory.path(), &mut telemetry).unwrap();
    let version = owner.load_resource(read, &mut telemetry).unwrap();
    let version = edited(&mut owner, version, &mut telemetry);
    let task = queued(&mut owner, &version, &mut telemetry);
    let (dispatch, probe) = launch(&mut owner, &filesystem, &handle, &task, &mut telemetry);
    let completion = filesystem.write(dispatch, probe, &mut telemetry).unwrap();
    assert_eq!(completion.failure, Some(expected));
    intake(&mut owner, completion, &mut telemetry);
    assert_eq!(
        owner
            .save_outcome(&task, &mut telemetry)
            .unwrap()
            .application,
        Application::NotWritten
    );
    assert_eq!(stdfs::read(directory.path()).unwrap(), b"A");
    assert_eq!(directory.stage_count(), 0);
}

#[test]
fn given_a_native_acl_when_saved_then_it_is_not_mistaken_for_absent_xattrs() {
    let directory = Directory::new();
    stdfs::write(directory.path(), b"A").unwrap();
    let status = std::process::Command::new("/bin/chmod")
        .args(["+a", "everyone allow read"])
        .arg(directory.path())
        .status()
        .unwrap();
    assert!(status.success());
    assert!(!native_acl::is_empty(&File::open(directory.path()).unwrap()).unwrap());
    assert_refused_metadata(&directory, FilesystemError::UnsupportedMetadata);
}

#[test]
fn given_external_change_after_dispatch_then_replacement_refuses_and_new_external_content_survives()
{
    let directory = Directory::new();
    stdfs::write(directory.path(), b"A").unwrap();
    let mut telemetry = Recorder::disabled();
    let (mut owner, mut filesystem) = configured(&mut telemetry);
    let (handle, read) = filesystem.open(&directory.path(), &mut telemetry).unwrap();
    let version = owner.load_resource(read, &mut telemetry).unwrap();
    let version = edited(&mut owner, version, &mut telemetry);
    let task = queued(&mut owner, &version, &mut telemetry);
    let (dispatch, probe) = launch(&mut owner, &filesystem, &handle, &task, &mut telemetry);
    stdfs::write(directory.path(), b"external").unwrap();
    let completion = filesystem.write(dispatch, probe, &mut telemetry).unwrap();
    assert_eq!(completion.failure, Some(FilesystemError::Conflict));
    intake(&mut owner, completion, &mut telemetry);
    assert_eq!(stdfs::read(directory.path()).unwrap(), b"external");
    assert_eq!(directory.stage_count(), 0);
}

#[test]
fn given_fault_before_or_after_replacement_then_application_and_sync_facts_do_not_collapse() {
    for fault in [
        Fault::BeforeReplace,
        Fault::AfterReplace,
        Fault::Synchronization,
    ] {
        let directory = Directory::new();
        stdfs::write(directory.path(), b"A").unwrap();
        let mut telemetry = Recorder::disabled();
        let (mut owner, mut filesystem) = configured(&mut telemetry);
        let (handle, read) = filesystem.open(&directory.path(), &mut telemetry).unwrap();
        let version = owner.load_resource(read, &mut telemetry).unwrap();
        let version = edited(&mut owner, version, &mut telemetry);
        let task = queued(&mut owner, &version, &mut telemetry);
        let (dispatch, probe) = launch(&mut owner, &filesystem, &handle, &task, &mut telemetry);
        filesystem.fault = Some(fault);
        let completion = filesystem.write(dispatch, probe, &mut telemetry).unwrap();
        assert_eq!(completion.failure, Some(FilesystemError::Io));
        intake(&mut owner, completion, &mut telemetry);
        let outcome = owner.save_outcome(&task, &mut telemetry).unwrap();
        assert!(outcome.no_future_effects && outcome.quiescent);
        if fault == Fault::BeforeReplace {
            assert_eq!(outcome.application, Application::NotWritten);
            assert_eq!(stdfs::read(directory.path()).unwrap(), b"A");
        } else {
            assert_eq!(outcome.application, Application::Written);
            assert_eq!(
                stdfs::read(directory.path()).unwrap(),
                "Ä changed".as_bytes()
            );
            assert_eq!(
                outcome.synchronization,
                if fault == Fault::Synchronization {
                    Synchronization::Failed
                } else {
                    Synchronization::Succeeded
                }
            );
        }
        if fault == Fault::Synchronization {
            assert_eq!(
                owner.publish_save_baseline(&task, &mut telemetry),
                Err(ResourceError::AcknowledgmentUnsatisfied)
            );
        }
        assert_eq!(directory.stage_count(), 0);
    }
}

#[test]
fn given_disabled_available_or_saturated_telemetry_then_physical_write_and_receipts_match() {
    for capacity in [None, Some(128), Some(1)] {
        let directory = Directory::new();
        stdfs::write(directory.path(), b"A").unwrap();
        let mut telemetry = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let (mut owner, mut filesystem) = configured(&mut telemetry);
        let (handle, read) = filesystem.open(&directory.path(), &mut telemetry).unwrap();
        let version = owner.load_resource(read, &mut telemetry).unwrap();
        let version = edited(&mut owner, version, &mut telemetry);
        let task = queued(&mut owner, &version, &mut telemetry);
        let (dispatch, probe) = launch(&mut owner, &filesystem, &handle, &task, &mut telemetry);
        let completion = filesystem.write(dispatch, probe, &mut telemetry).unwrap();
        assert_eq!(completion.failure, None);
        intake(&mut owner, completion, &mut telemetry);
        owner.publish_save_baseline(&task, &mut telemetry).unwrap();
        assert_eq!(
            stdfs::read(directory.path()).unwrap(),
            "Ä changed".as_bytes()
        );
        if capacity == Some(1) {
            assert!(telemetry.dropped() > 0);
        }
        while let Some(record) = telemetry.pop_front() {
            let rendered = format!("{record:?}");
            assert!(!rendered.contains("changed"));
            assert!(!rendered.contains("vellum-fs-test"));
        }
    }
}

#[test]
fn given_bound_or_foreign_resources_then_limits_and_namespace_checks_do_not_write() {
    let directory = Directory::new();
    stdfs::write(directory.path(), b"A").unwrap();
    let mut telemetry = Recorder::disabled();
    let (_, mut filesystem) = configured(&mut telemetry);
    filesystem.limits.max_bytes = 0;
    assert!(matches!(
        filesystem.open(&directory.path(), &mut telemetry),
        Err(FilesystemError::Limit)
    ));
    assert!(filesystem.entries.is_empty());
    filesystem.limits.max_bytes = 4096;
    let (handle, _) = filesystem.open(&directory.path(), &mut telemetry).unwrap();
    let (_, other) = configured(&mut telemetry);
    assert!(matches!(
        other.probe(&handle, &mut telemetry),
        Err(FilesystemError::Unavailable)
    ));
    assert_eq!(stdfs::read(directory.path()).unwrap(), b"A");
}

#[test]
fn given_a_newer_probe_after_launch_then_it_cannot_replace_the_original_pinned_condition() {
    let directory = Directory::new();
    stdfs::write(directory.path(), b"A").unwrap();
    let mut telemetry = Recorder::disabled();
    let (mut owner, mut filesystem) = configured(&mut telemetry);
    let (handle, read) = filesystem.open(&directory.path(), &mut telemetry).unwrap();
    let version = owner.load_resource(read, &mut telemetry).unwrap();
    let version = edited(&mut owner, version, &mut telemetry);
    let task = queued(&mut owner, &version, &mut telemetry);
    let (dispatch, old_probe) = launch(&mut owner, &filesystem, &handle, &task, &mut telemetry);
    stdfs::write(directory.path(), b"external").unwrap();
    let (_, newer_probe) = filesystem.probe(&handle, &mut telemetry).unwrap();
    drop(old_probe);
    let completion = filesystem
        .write(dispatch, newer_probe, &mut telemetry)
        .unwrap();
    assert_eq!(completion.failure, Some(FilesystemError::Conflict));
    intake(&mut owner, completion, &mut telemetry);
    assert_eq!(stdfs::read(directory.path()).unwrap(), b"external");
}

#[test]
fn given_unpreservable_or_over_budget_metadata_then_old_file_survives_without_staging() {
    for (mode, metadata_limit) in [(0o444, 4096), (0o1640, 4096), (0o640, 0)] {
        let directory = Directory::new();
        stdfs::write(directory.path(), b"A").unwrap();
        if metadata_limit == 0 {
            fs::fsetxattr(
                File::open(directory.path()).unwrap(),
                "org.vellum.budget",
                b"attribute",
                fs::XattrFlags::empty(),
            )
            .unwrap();
        }
        stdfs::set_permissions(directory.path(), stdfs::Permissions::from_mode(mode)).unwrap();
        let mut telemetry = Recorder::disabled();
        let (mut owner, mut filesystem) = configured(&mut telemetry);
        filesystem.limits.max_metadata_bytes = metadata_limit;
        let (handle, read) = filesystem.open(&directory.path(), &mut telemetry).unwrap();
        let version = owner.load_resource(read, &mut telemetry).unwrap();
        let version = edited(&mut owner, version, &mut telemetry);
        let task = queued(&mut owner, &version, &mut telemetry);
        let (dispatch, probe) = launch(&mut owner, &filesystem, &handle, &task, &mut telemetry);
        let completion = filesystem.write(dispatch, probe, &mut telemetry).unwrap();
        assert!(
            completion.failure.is_some(),
            "mode {mode:o}, budget {metadata_limit}, actual mode {:o}",
            stdfs::metadata(directory.path()).unwrap().mode()
        );
        intake(&mut owner, completion, &mut telemetry);
        assert_eq!(
            owner
                .save_outcome(&task, &mut telemetry)
                .unwrap()
                .application,
            Application::NotWritten
        );
        assert_eq!(stdfs::read(directory.path()).unwrap(), b"A");
        assert_eq!(
            stdfs::metadata(directory.path()).unwrap().mode() & 0o7777,
            mode
        );
        assert_eq!(directory.stage_count(), 0);
    }
}

#[test]
fn given_a_stage_name_collision_then_the_adapter_never_truncates_or_deletes_it() {
    let directory = Directory::new();
    stdfs::write(directory.path(), b"A").unwrap();
    let collision = directory
        .0
        .join(format!(".vellum-stage-{}-1", std::process::id()));
    stdfs::write(&collision, b"not ours").unwrap();
    let mut telemetry = Recorder::disabled();
    let (mut owner, mut filesystem) = configured(&mut telemetry);
    filesystem.limits.max_stage_attempts = 1;
    let (handle, read) = filesystem.open(&directory.path(), &mut telemetry).unwrap();
    let version = owner.load_resource(read, &mut telemetry).unwrap();
    let version = edited(&mut owner, version, &mut telemetry);
    let task = queued(&mut owner, &version, &mut telemetry);
    let (dispatch, probe) = launch(&mut owner, &filesystem, &handle, &task, &mut telemetry);
    let completion = filesystem.write(dispatch, probe, &mut telemetry).unwrap();
    assert_eq!(completion.failure, Some(FilesystemError::Limit));
    intake(&mut owner, completion, &mut telemetry);
    assert_eq!(stdfs::read(collision).unwrap(), b"not ours");
    assert_eq!(stdfs::read(directory.path()).unwrap(), b"A");
}

#[test]
fn given_a_failed_replacement_call_then_nonapplication_is_not_invented() {
    let directory = Directory::new();
    stdfs::write(directory.path(), b"A").unwrap();
    let mut telemetry = Recorder::disabled();
    let (mut owner, mut filesystem) = configured(&mut telemetry);
    let (handle, read) = filesystem.open(&directory.path(), &mut telemetry).unwrap();
    let version = owner.load_resource(read, &mut telemetry).unwrap();
    let version = edited(&mut owner, version, &mut telemetry);
    let task = queued(&mut owner, &version, &mut telemetry);
    let (dispatch, probe) = launch(&mut owner, &filesystem, &handle, &task, &mut telemetry);
    filesystem.fault = Some(Fault::Replacement);
    let completion = filesystem.write(dispatch, probe, &mut telemetry).unwrap();
    assert_eq!(completion.failure, Some(FilesystemError::Io));
    intake(&mut owner, completion, &mut telemetry);
    let outcome = owner.save_outcome(&task, &mut telemetry).unwrap();
    assert_eq!(outcome.application, Application::Unknown);
    assert!(outcome.no_future_effects && outcome.quiescent);
    assert!(owner.publish_save_baseline(&task, &mut telemetry).is_err());
    assert!(owner.retire_save(&task, &mut telemetry).is_err());
    assert_eq!(directory.stage_count(), 0);
}

struct PhysicalExportTestAuthority {
    source: vellum_host::export_copy::ExportCopySourceRequestV1,
    destination: vellum_host::authority::SaveAsDestinationRequestV1,
    object: Option<vellum_host::authority::SaveAsDestinationObjectV1>,
}

impl vellum_host::export_copy::ExportCopyAuthorityPortV1 for PhysicalExportTestAuthority {
    fn authorize_export_copy(
        &mut self,
        source: vellum_host::export_copy::ExportCopySourceRequestV1,
        destination: vellum_host::authority::SaveAsDestinationRequestV1,
    ) -> vellum_host::export_copy::ExportCopyAuthorityDecisionV1 {
        use vellum_host::authority::{
            AuthorityDecisionRefV1, SaveAsDestinationDecisionRefV1, SaveAsDestinationEvidenceV1,
        };
        use vellum_host::export_copy::{
            ExportCopyAuthorityDecisionV1, ExportCopySourceDecisionEvidenceV1,
        };
        if source != self.source || destination != self.destination {
            return ExportCopyAuthorityDecisionV1::Denied;
        }
        let Some(object) = self.object.take() else {
            return ExportCopyAuthorityDecisionV1::Unavailable;
        };
        ExportCopyAuthorityDecisionV1::Granted {
            source: ExportCopySourceDecisionEvidenceV1::new(
                AuthorityDecisionRefV1::new([5; 16]),
                source,
            ),
            destination: SaveAsDestinationEvidenceV1::with_object(
                SaveAsDestinationDecisionRefV1::new([6; 16]),
                destination,
                object,
            ),
        }
    }
}

fn authorized_export_copy(
    path: std::path::PathBuf,
    disposition: vellum_host::export_copy::ExportCopyDispositionV1,
    bytes: &[u8],
) -> vellum_host::export_copy::AuthorizedExportCopyV1 {
    use vellum_host::authority::{
        AuthorityContextV1, AuthorityResourceRefV1, AuthoritySubjectRefV1,
        SaveAsDestinationEffectV1, SaveAsDestinationRequestV1,
    };
    let context = AuthorityContextV1 {
        subject: AuthoritySubjectRefV1::new([1; 16]),
        session: None,
    };
    let resource = AuthorityResourceRefV1::new([2; 16]);
    let effect = match disposition {
        vellum_host::export_copy::ExportCopyDispositionV1::OverwriteExisting => {
            SaveAsDestinationEffectV1::OverwriteExisting
        }
        vellum_host::export_copy::ExportCopyDispositionV1::CreateExclusive => {
            SaveAsDestinationEffectV1::CreateExclusive
        }
    };
    let request = SaveAsDestinationRequestV1::new(context, resource, path.clone(), effect);
    let object = crate::install_local_export_destination(&request).unwrap();
    authorized_export_copy_with_object(path, disposition, bytes, object)
}

fn authorized_export_copy_with_object(
    path: std::path::PathBuf,
    disposition: vellum_host::export_copy::ExportCopyDispositionV1,
    bytes: &[u8],
    object: vellum_host::authority::SaveAsDestinationObjectV1,
) -> vellum_host::export_copy::AuthorizedExportCopyV1 {
    use vellum_host::authority::{
        AuthorityActionV1, AuthorityContextV1, AuthorityRequestV1, AuthorityResourceRefV1,
        AuthoritySubjectRefV1, SaveAsDestinationEffectV1, SaveAsDestinationRequestV1,
    };
    use vellum_host::export_copy::AuthorizedExportCopyPlanV1;
    let context = AuthorityContextV1 {
        subject: AuthoritySubjectRefV1::new([1; 16]),
        session: None,
    };
    let resource = AuthorityResourceRefV1::new([2; 16]);
    let effect = match disposition {
        vellum_host::export_copy::ExportCopyDispositionV1::OverwriteExisting => {
            SaveAsDestinationEffectV1::OverwriteExisting
        }
        vellum_host::export_copy::ExportCopyDispositionV1::CreateExclusive => {
            SaveAsDestinationEffectV1::CreateExclusive
        }
    };
    let source_request = AuthorityRequestV1 {
        context,
        resource,
        action: AuthorityActionV1::ExportSourceCopy,
    };
    let destination_request = SaveAsDestinationRequestV1::new(context, resource, path, effect);
    let mut recorder = Recorder::disabled();
    let (mut owner, _) = configured(&mut recorder);
    let source = owner.create(bytes, &mut recorder).unwrap();
    let mut authority = PhysicalExportTestAuthority {
        source: vellum_host::export_copy::ExportCopySourceRequestV1::new(
            source_request,
            source.clone(),
        )
        .unwrap(),
        destination: destination_request.clone(),
        object: Some(object),
    };
    let serialization = owner
        .source_serialization(&source.document, &mut recorder)
        .unwrap();
    let plan = AuthorizedExportCopyPlanV1::authorize(
        &mut authority,
        source_request,
        destination_request,
        source.clone(),
        serialization,
    )
    .unwrap();
    let prepared = owner
        .prepare_source_artifact(&source, serialization, &mut recorder)
        .unwrap();
    let capture = owner
        .capture_prepared_source_serialization(&prepared, &mut recorder)
        .unwrap();
    plan.attach_source_capture(capture).unwrap()
}

#[test]
fn export_effect_port_rejects_foreign_destination_objects_without_publication() {
    use vellum_host::authority::{
        AuthorityContextV1, AuthorityResourceRefV1, AuthoritySubjectRefV1,
        SaveAsDestinationEffectV1, SaveAsDestinationObjectV1, SaveAsDestinationRequestV1,
    };
    use vellum_host::export_copy::{
        ExportCopyDispositionV1, ExportCopyEffectPortV1, ExportCopyFailureV1,
        ExportCopyPersistenceV1,
    };
    let directory = Directory::new();
    let path = directory.path();
    let secret_path = path.to_string_lossy().into_owned();
    let secret_content = b"foreign-object-private-content";
    let authorization = authorized_export_copy_with_object(
        path.clone(),
        ExportCopyDispositionV1::CreateExclusive,
        secret_content,
        SaveAsDestinationObjectV1::install(
            SaveAsDestinationRequestV1::new(
                AuthorityContextV1 {
                    subject: AuthoritySubjectRefV1::new([1; 16]),
                    session: None,
                },
                AuthorityResourceRefV1::new([2; 16]),
                path.clone(),
                SaveAsDestinationEffectV1::CreateExclusive,
            ),
            String::from("not-a-filesystem-capability"),
        ),
    );
    let mut telemetry = Recorder::try_with_capacity(16).unwrap();
    let (_, mut filesystem) = configured(&mut telemetry);

    let receipt = filesystem.execute_export_copy(authorization, &mut telemetry);

    assert_eq!(receipt.persistence(), ExportCopyPersistenceV1::NotAttempted);
    assert_eq!(receipt.failure(), Some(ExportCopyFailureV1::Unsupported));
    assert!(!path.exists());
    assert_eq!(filesystem.export_copy_usage().attempts, 1);
    assert_eq!(filesystem.export_copy_usage().not_attempted, 1);
    let receipt_debug = format!("{receipt:?}");
    assert!(!receipt_debug.contains(&secret_path));
    assert!(!receipt_debug.contains("foreign-object-private-content"));
    while let Some(observation) = telemetry.pop_front() {
        let observation_debug = format!("{observation:?}");
        assert!(!observation_debug.contains(&secret_path));
        assert!(!observation_debug.contains("foreign-object-private-content"));
    }
}

#[test]
fn export_plan_refuses_a_codec_capture_from_another_source_incarnation() {
    use vellum_host::authority::{
        AuthorityActionV1, AuthorityContextV1, AuthorityRequestV1, AuthorityResourceRefV1,
        AuthoritySubjectRefV1, SaveAsDestinationEffectV1, SaveAsDestinationObjectV1,
        SaveAsDestinationRequestV1,
    };
    use vellum_host::export_copy::{AuthorizedExportCopyPlanV1, ExportCopyAttachmentRefusalV1};
    let context = AuthorityContextV1 {
        subject: AuthoritySubjectRefV1::new([1; 16]),
        session: None,
    };
    let resource = AuthorityResourceRefV1::new([2; 16]);
    let source_request = AuthorityRequestV1 {
        context,
        resource,
        action: AuthorityActionV1::ExportSourceCopy,
    };
    let destination_request = SaveAsDestinationRequestV1::new(
        context,
        resource,
        std::path::PathBuf::from("/private/tmp/vellum-never-published"),
        SaveAsDestinationEffectV1::CreateExclusive,
    );
    let mut recorder = Recorder::disabled();
    let (mut owner, _) = configured(&mut recorder);
    let expected = owner.create(b"expected", &mut recorder).unwrap();
    let other = owner.create(b"other", &mut recorder).unwrap();
    let mut authority = PhysicalExportTestAuthority {
        source: vellum_host::export_copy::ExportCopySourceRequestV1::new(
            source_request,
            expected.clone(),
        )
        .unwrap(),
        destination: destination_request.clone(),
        object: Some(SaveAsDestinationObjectV1::install(
            destination_request.clone(),
            (),
        )),
    };
    let expected_serialization = owner
        .source_serialization(&expected.document, &mut recorder)
        .unwrap();
    let other_serialization = owner
        .source_serialization(&other.document, &mut recorder)
        .unwrap();
    let other_prepared = owner
        .prepare_source_artifact(&other, other_serialization, &mut recorder)
        .unwrap();
    let other_capture = owner
        .capture_prepared_source_serialization(&other_prepared, &mut recorder)
        .unwrap();
    let plan = AuthorizedExportCopyPlanV1::authorize(
        &mut authority,
        source_request,
        destination_request,
        expected,
        expected_serialization,
    )
    .unwrap();
    assert!(matches!(
        plan.attach_source_capture(other_capture),
        Err(ExportCopyAttachmentRefusalV1::SourceChanged)
    ));
}

#[test]
fn qualified_exclusive_export_is_durable_and_never_overwrites() {
    use vellum_host::export_copy::{
        ExportCopyDispositionV1, ExportCopyEffectPortV1, ExportCopyFailureV1,
        ExportCopyPersistenceV1,
    };
    let directory = Directory::new();
    let path = directory.path();
    let mut telemetry = Recorder::disabled();
    let (_, mut filesystem) = configured(&mut telemetry);
    let authorization = authorized_export_copy(
        path.clone(),
        ExportCopyDispositionV1::CreateExclusive,
        b"payload-with-private-length",
    );
    let debug = format!("{authorization:?}");
    assert!(!debug.contains("payload-with-private-length"));
    assert!(!debug.contains("27"));
    let receipt = filesystem.execute_export_copy(authorization, &mut telemetry);
    assert_eq!(receipt.persistence(), ExportCopyPersistenceV1::Durable);
    assert_eq!(receipt.failure(), None);
    assert_eq!(stdfs::read(&path).unwrap(), b"payload-with-private-length");
    assert_eq!(filesystem.export_copy_usage().durable, 1);
    let refusal = filesystem.execute_export_copy(
        authorized_export_copy(
            path.clone(),
            ExportCopyDispositionV1::CreateExclusive,
            b"lost",
        ),
        &mut telemetry,
    );
    assert_eq!(refusal.persistence(), ExportCopyPersistenceV1::NotAttempted);
    assert_eq!(refusal.failure(), Some(ExportCopyFailureV1::Conflict));
    assert_eq!(stdfs::read(path).unwrap(), b"payload-with-private-length");
}

#[test]
fn ancestor_rename_after_authorization_cannot_redirect_the_destination_object() {
    use vellum_host::export_copy::{ExportCopyDispositionV1, ExportCopyPersistenceV1};
    let fixture = Directory::new();
    let installed = fixture.0.join("installed");
    let moved = fixture.0.join("moved");
    stdfs::create_dir(&installed).unwrap();
    let display_path = installed.join("copy");
    let authorization = authorized_export_copy(
        display_path.clone(),
        ExportCopyDispositionV1::CreateExclusive,
        b"anchored",
    );
    stdfs::rename(&installed, &moved).unwrap();
    stdfs::create_dir(&installed).unwrap();

    let mut telemetry = Recorder::disabled();
    let (_, mut filesystem) = configured(&mut telemetry);
    let prepared = filesystem
        .prepare_export_copy(authorization, &mut telemetry)
        .unwrap();
    let receipt = filesystem.service_export_copy(prepared, &mut telemetry);
    assert_eq!(receipt.persistence(), ExportCopyPersistenceV1::Durable);
    assert_eq!(stdfs::read(moved.join("copy")).unwrap(), b"anchored");
    assert!(!display_path.exists());

    stdfs::remove_file(moved.join("copy")).unwrap();
    stdfs::remove_dir(moved).unwrap();
    stdfs::remove_dir(installed).unwrap();
}

#[test]
fn ancestor_swap_after_preparation_cannot_redirect_publication() {
    use vellum_host::export_copy::{ExportCopyDispositionV1, ExportCopyPersistenceV1};
    let fixture = Directory::new();
    let installed = fixture.0.join("installed");
    let moved = fixture.0.join("moved");
    stdfs::create_dir(&installed).unwrap();
    let display_path = installed.join("copy");
    let authorization = authorized_export_copy(
        display_path.clone(),
        ExportCopyDispositionV1::CreateExclusive,
        b"anchored",
    );
    let mut telemetry = Recorder::disabled();
    let (_, mut filesystem) = configured(&mut telemetry);
    let prepared = filesystem
        .prepare_export_copy(authorization, &mut telemetry)
        .unwrap();
    stdfs::rename(&installed, &moved).unwrap();
    stdfs::create_dir(&installed).unwrap();

    let receipt = filesystem.service_export_copy(prepared, &mut telemetry);
    assert_eq!(receipt.persistence(), ExportCopyPersistenceV1::Durable);
    assert_eq!(stdfs::read(moved.join("copy")).unwrap(), b"anchored");
    assert!(!display_path.exists());

    stdfs::remove_file(moved.join("copy")).unwrap();
    stdfs::remove_dir(moved).unwrap();
    stdfs::remove_dir(installed).unwrap();
}

#[test]
fn exclusive_export_collision_after_preparation_uses_noreplace_nonapplication() {
    use vellum_host::export_copy::{
        ExportCopyDispositionV1, ExportCopyFailureV1, ExportCopyPersistenceV1,
    };
    let directory = Directory::new();
    let path = directory.path();
    let mut telemetry = Recorder::disabled();
    let (_, mut filesystem) = configured(&mut telemetry);
    let prepared = filesystem
        .prepare_export_copy(
            authorized_export_copy(
                path.clone(),
                ExportCopyDispositionV1::CreateExclusive,
                b"ours",
            ),
            &mut telemetry,
        )
        .unwrap();
    stdfs::write(&path, b"external").unwrap();
    let receipt = filesystem.service_export_copy(prepared, &mut telemetry);
    assert_eq!(receipt.persistence(), ExportCopyPersistenceV1::NotAttempted);
    assert_eq!(receipt.failure(), Some(ExportCopyFailureV1::Conflict));
    assert_eq!(stdfs::read(path).unwrap(), b"external");
    assert_eq!(directory.stage_count(), 0);
}

#[test]
fn postpublication_sync_failure_is_unknown_and_receipt_is_content_free() {
    use vellum_host::export_copy::{ExportCopyDispositionV1, ExportCopyPersistenceV1};
    let directory = Directory::new();
    let path = directory.path();
    let mut telemetry = Recorder::disabled();
    let (_, mut filesystem) = configured(&mut telemetry);
    let prepared = filesystem
        .prepare_export_copy(
            authorized_export_copy(
                path.clone(),
                ExportCopyDispositionV1::CreateExclusive,
                b"private payload",
            ),
            &mut telemetry,
        )
        .unwrap();
    filesystem.fault = Some(Fault::ExportSynchronization);
    let receipt = filesystem.service_export_copy(prepared, &mut telemetry);
    assert_eq!(receipt.persistence(), ExportCopyPersistenceV1::Unknown);
    assert_eq!(stdfs::read(&path).unwrap(), b"private payload");
    let debug = format!("{receipt:?}");
    assert!(!debug.contains(path.to_string_lossy().as_ref()));
    assert!(!debug.contains("private payload"));
}

#[test]
fn intermediate_symlink_and_parent_traversal_are_closed_refusals() {
    use std::os::unix::fs::symlink;
    use vellum_host::authority::{
        AuthorityContextV1, AuthorityResourceRefV1, AuthoritySubjectRefV1,
        SaveAsDestinationEffectV1, SaveAsDestinationRequestV1,
    };
    let directory = Directory::new();
    let real = directory.0.join("real");
    let link = directory.0.join("link");
    stdfs::create_dir(&real).unwrap();
    symlink(&real, &link).unwrap();
    let path = link.join("copy");
    let context = AuthorityContextV1 {
        subject: AuthoritySubjectRefV1::new([1; 16]),
        session: None,
    };
    let request = SaveAsDestinationRequestV1::new(
        context,
        AuthorityResourceRefV1::new([2; 16]),
        path,
        SaveAsDestinationEffectV1::CreateExclusive,
    );
    assert!(crate::install_local_export_destination(&request).is_err());
    assert!(!real.join("copy").exists());
    stdfs::remove_file(link).unwrap();
    stdfs::remove_dir(real).unwrap();
}
