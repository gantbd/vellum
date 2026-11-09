use super::*;
use crate::coordinator::shutdown::tests::{context, facts, limits as fence_limits};
#[test]
fn inert_provider_configuration_is_not_mistaken_for_a_worker_or_payload_lease() {
    use crate::resource::{ProviderProfile, ResourceLimits};
    let mut o = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 2,
        max_participants: 2,
        max_staged_bytes: 128,
        source: crate::source::Limits {
            max_source_bytes: 64,
            max_edits: 4,
            max_edit_payload_bytes: 64,
        },
    });
    o.resources = resource::ResourceState::new(ResourceLimits {
        max_providers: 1,
        max_live_tasks: 0,
        max_issued_tasks: 0,
        max_queued_per_destination: 0,
        max_outstanding: 0,
        max_blob_bytes: 64,
        max_key_bytes: 64,
        max_validator_bytes: 64,
        max_encoders: 0,
        max_live_artifacts: 0,
        max_issued_artifacts: 0,
        max_held_artifact_bytes: 0,
        max_future_baseline_bytes: 0,
        max_live_reads: 0,
        max_issued_reads: 0,
        max_read_chunks: 0,
        max_held_read_bytes: 0,
        max_restorers: 0,
        max_restoration_bytes: 0,
    });
    let mut r = Recorder::disabled();
    let port = o
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut r,
        )
        .unwrap();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let c = context();
    let control = facts(&c);
    let auth = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let basis = o
        .capture_shutdown_fence_basis_v1(fence_limits(), &control, &mut r)
        .unwrap();
    let fence = o
        .install_shutdown_fence_v1(&basis, &control, &mut r)
        .unwrap();
    // Configuring resources also installs an actual reader -> writer owner
    // dependency. Discharge those owners through their native controls; only
    // the inert provider declaration is allowed to remain at final release.
    o.stop_annotation_read_admission_v1(&mut r);
    o.retire_annotation_reader_v1(Some(true), Some(true), Some(true), &mut r)
        .unwrap();
    o.stop_source_save_admission_v1(&mut r);
    o.stop_annotation_save_admission(&mut r);
    o.retire_shutdown_writer_v1(&fence, &control, Some(true), Some(true), &mut r)
        .unwrap();
    let limits = ShutdownInventoryLimitsV1 {
        max_documents: 2,
        max_writes: 2,
        max_artifacts: 2,
        max_bindings: 2,
        max_reads: 2,
        max_owners: 64,
        max_attachments: 2,
        max_dependencies: 64,
        max_work: 1024,
        max_capture_bytes: 65536,
    };
    assert_eq!(
        o.finish_shutdown_attempt_v1(&fence, limits, &auth, &mut r),
        Ok(ShutdownAttemptStatusV1::Drained)
    );
    let (_, report) = o
        .read_shutdown_attempt_report_v1(&fence, &auth, &mut r)
        .unwrap();
    let metadata = report
        .owners
        .iter()
        .find(|row| row.owner.role() == ShutdownOwnerRoleV1::ResourceMetadata)
        .unwrap();
    assert_eq!(
        metadata.retained_records,
        Some(1),
        "report preserves configuration counts, not zero work by erasure"
    );
    o.retire_shutdown_attempt_report_v1(&fence, &control, Some(true), Some(true), &mut r)
        .unwrap();
    o.retire_shutdown_control_v1(&fence, limits, &auth, &mut r)
        .unwrap();
    assert_eq!(
        o.load_resource(port.read_result(b"key", b"v", b"text").unwrap(), &mut r),
        Err(crate::resource::ResourceError::Unavailable)
    );
    assert_eq!(
        o.install_shutdown_fence_v1(&basis, &control, &mut r),
        Err(CoordinatorError::Unavailable)
    );
}
#[test]
fn returned_report_replays_under_fault_and_never_hides_unrepresented_parent_funding() {
    let mut o = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 2,
        max_participants: 2,
        max_staged_bytes: 128,
        source: crate::source::Limits {
            max_source_bytes: 64,
            max_edits: 4,
            max_edit_payload_bytes: 64,
        },
    });
    let mut r = Recorder::disabled();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let c = context();
    let control = facts(&c);
    let auth = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let basis = o
        .capture_shutdown_fence_basis_v1(fence_limits(), &control, &mut r)
        .unwrap();
    let fence = o
        .install_shutdown_fence_v1(&basis, &control, &mut r)
        .unwrap();
    let limits = ShutdownInventoryLimitsV1 {
        max_documents: 2,
        max_writes: 2,
        max_artifacts: 2,
        max_bindings: 2,
        max_reads: 2,
        max_owners: 64,
        max_attachments: 2,
        max_dependencies: 64,
        max_work: 1024,
        max_capture_bytes: 65536,
    };
    assert_eq!(
        o.finish_shutdown_attempt_v1(&fence, limits, &auth, &mut r),
        Ok(ShutdownAttemptStatusV1::Drained)
    );
    o.faulted = true;
    assert_eq!(
        o.finish_shutdown_attempt_v1(&fence, limits, &auth, &mut r),
        Ok(ShutdownAttemptStatusV1::Drained)
    );
    assert_eq!(
        o.read_shutdown_attempt_report_v1(&fence, &auth, &mut r)
            .unwrap()
            .0,
        ShutdownAttemptStatusV1::Drained
    );
    o.retire_shutdown_attempt_report_v1(&fence, &control, Some(true), Some(true), &mut r)
        .unwrap();
    assert!(o.is_faulted());
    assert_eq!(
        o.check_shutdown_root_release_v1(&fence, limits, &auth, &mut r),
        Err(CoordinatorError::Faulted)
    );
    // Test-only reset isolates ledger incompleteness from the preceding fault;
    // production report/control paths never heal the authored engine.
    o.faulted = false;
    let mut unknown = o.retention.reserve(1).unwrap();
    assert_eq!(
        o.check_shutdown_root_release_v1(&fence, limits, &auth, &mut r),
        Err(CoordinatorError::Unavailable)
    );
    o.retention.release(&mut unknown).unwrap();
    o.check_shutdown_root_release_v1(&fence, limits, &auth, &mut r)
        .unwrap();
    o.retire_shutdown_control_v1(&fence, limits, &auth, &mut r)
        .unwrap();
    assert!(o.shutdown.retired);
    assert!(o.shutdown.fence.is_none());
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(o.ensure_operational(), Err(CoordinatorError::Unavailable));
}
