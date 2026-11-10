#![allow(dead_code)]

use vellum_adapter_local_fs::{
    FilesystemLimits, LocalFilesystem, LocalFilesystemFactory, ResourceHandle,
};
use vellum_engine::coordinator::{CoordinatorConfig, CoordinatorLimits, HistoryLimits};
use vellum_engine::instrumentation::NativeTimingProfileV1;
use vellum_engine::resource::ResourceLimits;
use vellum_engine::source::Limits;
use vellum_session::{
    host::{LocalDocument as GenericDocument, LocalHost as GenericHost},
    local::{LocalSession as GenericLocalSession, SessionConfig as GenericSessionConfig},
};
pub type SessionConfig = GenericSessionConfig<LocalFilesystemFactory>;
pub type LocalHost = GenericHost<LocalFilesystem>;
pub type LocalDocument = GenericDocument<ResourceHandle>;
pub type LocalSession = GenericLocalSession<LocalFilesystem>;

// Explicit example-host budgets, not engine defaults or a supported product SLO.
pub fn config(capture_capacity: usize, timed: bool) -> SessionConfig {
    SessionConfig {
        logical_views: None,
        line_edits: None,
        engine: CoordinatorConfig {
            limits: CoordinatorLimits {
                max_document_incarnations: 4,
                max_participants: 4,
                max_staged_bytes: 262144,
                source: Limits {
                    max_source_bytes: 65536,
                    max_edits: 128,
                    max_edit_payload_bytes: 262144,
                },
            },
            invocation: None,
            history: Some(HistoryLimits {
                max_retained_groups: 32,
                max_journal_bytes: 1048576,
                max_pins: 32,
            }),
            resources: Some(ResourceLimits {
                max_providers: 1,
                max_live_tasks: 16,
                max_issued_tasks: 64,
                max_queued_per_destination: 4,
                max_outstanding: 4,
                max_blob_bytes: 65536,
                max_key_bytes: 64,
                max_validator_bytes: 64,
                max_encoders: 1,
                max_live_artifacts: 16,
                max_issued_artifacts: 64,
                max_held_artifact_bytes: 1048576,
                max_future_baseline_bytes: 1048576,
                max_live_reads: 4,
                max_issued_reads: 16,
                max_read_chunks: 16,
                max_held_read_bytes: 1048576,
                max_restorers: 1,
                max_restoration_bytes: 65536,
            }),
            search: None,
            annotations: None,
        },
        resource_adapter: LocalFilesystemFactory::new(FilesystemLimits {
            max_resources: 1,
            max_bytes: 65536,
            max_stage_attempts: 16,
            max_metadata_bytes: 65536,
        }),
        capture_capacity,
        timing: if timed {
            NativeTimingProfileV1::ProcessMonotonicV1
        } else {
            NativeTimingProfileV1::Off
        },
    }
}
