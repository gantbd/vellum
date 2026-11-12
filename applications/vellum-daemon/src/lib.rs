//! Local Unix service composition for Vellum.
#![forbid(unsafe_code)]

pub mod cli;

#[cfg(unix)]
use keystone_lifecycle::{LifecycleError, LifecyclePhase, ObligationLedger};
#[cfg(unix)]
use keystone_observe::{FixedDurationHistogram, FixedDurationHistogramSnapshot};
#[cfg(unix)]
use std::panic::{AssertUnwindSafe, catch_unwind};
#[cfg(unix)]
use std::{
    collections::HashSet,
    fs,
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    net::Shutdown,
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        fs::{FileExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
#[cfg(target_os = "macos")]
use vellum_adapter_local_fs::{
    FilesystemLimits, LocalDirectoryDiscovery, LocalExportCopyEffectV1, LocalRequestJournalV1,
    LocalWorkspaceIdentityMetadataV1, install_local_export_destination,
};
#[cfg(unix)]
use vellum_editor::Workspace;
#[cfg(target_os = "macos")]
use vellum_editor::{WorkspaceConfig, WorkspaceError};
#[cfg(target_os = "macos")]
use vellum_host::authority::{
    AuthorityActionSetV1, AuthorityActionV1, AuthorityDecisionRefV1, AuthorityResourceRefV1,
    SaveAsDestinationDecisionRefV1, SaveAsDestinationEffectV1, SaveAsDestinationEvidenceV1,
    SaveAsDestinationRequestV1,
};
#[cfg(target_os = "macos")]
use vellum_host::export_copy::{
    ExportCopyAuthorityDecisionV1, ExportCopyAuthorityPortV1, ExportCopySourceDecisionEvidenceV1,
    ExportCopySourceRequestV1,
};
#[cfg(unix)]
use vellum_host::request_journal::{
    CanonicalDurableEffectV1, CanonicalDurableOutcomeV1, DurableOperationCompletionV1,
    DurableOperationIntentV1, DurableOperationLookupV1, DurableOperationOwnerV1,
    DurableOperationRecordV1, DurableOperationRecoveryV1, DurableOperationRetirementRecordV1,
    DurableOperationRetirementRecoveryV1, DurableOperationRetirementResultV1,
    DurableOperationRetirementV1, RequestJournalCompletionV1, RequestJournalIntentV1,
    RequestJournalLimitsV1, RequestJournalPortV1, RequestJournalRecordV1, RequestJournalRecoveryV1,
    RequestJournalRefusalV1, RequestJournalScopeV1, lookup_recovered_operation_v1,
    validate_recovered_durable_effect_v1,
};
#[cfg(unix)]
use vellum_host::workspace::{
    WorkspaceIdentityLimitsV1, WorkspaceIdentityMetadataPortV1, WorkspaceIdentityMetadataV1,
    WorkspaceIdentityRecoveryRefusalV1, WorkspaceIdentityRecoveryV1,
};
#[cfg(unix)]
use vellum_host::{
    authentication::{
        AuthenticatedSessionV1, AuthenticationChallengeRefV1, AuthenticationClaimRefV1,
        AuthenticationGateV1, AuthenticationPortV1, AuthenticationRefusalV1,
        AuthenticationRequestV1,
    },
    authority::AuthoritySessionRefV1,
};
#[cfg(all(unix, test))]
use vellum_protocol::DurableEffectReceiptV1;
#[cfg(unix)]
use vellum_protocol::{
    CommandV1, DurableEffectIntentV1, DurableExecuteOutcomeV1, DurableExecuteRequestV1,
    DurableReceiptLookupOutcomeV1, DurableReceiptLookupV1, DurableReceiptRetirementOutcomeV1,
    DurableReceiptRetirementV1, OutcomeV1, ReplyV1, RequestV1, ServiceAccessErrorV1,
    ServiceControlErrorV1, ServicePort,
};
#[cfg(unix)]
use vellum_service::{
    AuthenticatedExportExecutionV1, RequestRetentionOutcome, Service, ServiceBuildError,
    ServiceLimits, ServiceUsage,
};
#[cfg(unix)]
pub use vellum_transport::{AuthenticatedUnixConnector, LocalAuthenticationClaimV1};
#[cfg(unix)]
use vellum_transport::{TransportError, TransportLimits, serve_one};
#[cfg(unix)]
use vellum_wire::{
    WireReplyV1, WireRequestV1, decode_reply_v1, decode_request_v1, encode_reply_v1,
    encode_request_v1,
};

#[cfg(unix)]
const AUTHENTICATION_HANDSHAKE_VERSION_V1: u8 = 1;
#[cfg(unix)]
const AUTHENTICATION_HELLO_BYTES_V1: usize = 33;
#[cfg(unix)]
const AUTHENTICATION_RESPONSE_BYTES_V1: usize = 49;
#[cfg(unix)]
const CONTROL_SHUTDOWN_REQUEST_V1: u8 = 1;
#[cfg(unix)]
const CONTROL_STATUS_REQUEST_V1: u8 = 2;
#[cfg(unix)]
const CONTROL_PROTOCOL_VERSION_V1: u8 = 1;
#[cfg(unix)]
const CONTROL_STATUS_MAGIC_V1: [u8; 4] = *b"VCS1";
#[cfg(unix)]
const CONTROL_STATUS_UTILIZATION_FIELDS_V1: usize = 13;
#[cfg(unix)]
const CONTROL_STATUS_REPLY_BYTES_V1: usize =
    4 + 1 + 4 + CONTROL_STATUS_UTILIZATION_FIELDS_V1 * 8 + 2;
#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DaemonLimits {
    pub service: ServiceLimits,
    pub transport: TransportLimits,
    pub io_timeout: Duration,
    pub max_in_flight_connections: usize,
    pub max_startup_roots: usize,
    /// Lifetime credential consultations. This bounds authentication work
    /// independently of the service authority-decision budget.
    pub max_authentication_attempts: u64,
    pub workspace_identity: WorkspaceIdentityLimitsV1,
    pub request_journal: RequestJournalLimitsV1,
    pub request_latency: Option<RequestLatencyProfile>,
}

#[cfg(unix)]
pub const REQUEST_LATENCY_BUCKETS: usize = 5;

#[cfg(unix)]
const SERIALIZED_CONNECTION_CAPACITY: usize = 1;

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestLatencyProfile {
    pub upper_bounds: [Duration; REQUEST_LATENCY_BUCKETS],
}

#[cfg(unix)]
impl RequestLatencyProfile {
    pub const fn reference() -> Self {
        Self {
            upper_bounds: [
                Duration::from_millis(1),
                Duration::from_millis(5),
                Duration::from_millis(25),
                Duration::from_millis(100),
                Duration::from_secs(1),
            ],
        }
    }
}

#[cfg(unix)]
impl DaemonLimits {
    pub const fn reference() -> Self {
        Self {
            service: ServiceLimits {
                // 256 ordinary requests * 2 decisions plus every one of 256
                // active and 4096 retired durable identities * (2 effect + 1
                // retirement) decisions. Checked at construction below.
                max_authority_decisions: 13_568,
                ..ServiceLimits::reference()
            },
            transport: TransportLimits::reference(),
            io_timeout: Duration::from_secs(5),
            max_in_flight_connections: 1,
            max_startup_roots: 16,
            max_authentication_attempts: 1_024,
            workspace_identity: WorkspaceIdentityLimitsV1 {
                max_documents: 256,
                max_roots: 256,
                max_encoded_bytes: 1024,
            },
            request_journal: RequestJournalLimitsV1 {
                max_entries: 256,
                max_retired_operations: 4096,
                max_request_bytes: 65_600,
                max_reply_bytes: 65_600,
                max_encoded_bytes: 131_264,
            },
            request_latency: Some(RequestLatencyProfile::reference()),
        }
    }
}

/// Exact operator-installed create-exclusive export policy.
///
/// The document ordinal, source revision and absolute destination are fixed
/// before endpoint publication. The caller cannot substitute the durable
/// operation identity, source, revision, or destination.
#[cfg(target_os = "macos")]
#[derive(Clone)]
pub struct DaemonExportPolicyV1 {
    operation: vellum_protocol::DurableOperationRefV1,
    document_ordinal: u64,
    expected_revision: u64,
    destination: PathBuf,
    filesystem: FilesystemLimits,
    diagnostic_capacity: usize,
}

#[cfg(target_os = "macos")]
impl DaemonExportPolicyV1 {
    pub fn exact_create_exclusive(
        operation: vellum_protocol::DurableOperationRefV1,
        document_ordinal: u64,
        expected_revision: u64,
        destination: PathBuf,
        filesystem: FilesystemLimits,
        diagnostic_capacity: usize,
    ) -> Option<Self> {
        if document_ordinal == 0
            || diagnostic_capacity < 4
            || !destination.is_absolute()
            || destination.to_str().is_none()
            || destination.file_name().is_none()
            || destination.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir | std::path::Component::Prefix(_)
                )
            })
        {
            return None;
        }
        Some(Self {
            operation,
            document_ordinal,
            expected_revision,
            destination,
            filesystem,
            diagnostic_capacity,
        })
    }
}

#[cfg(target_os = "macos")]
impl std::fmt::Debug for DaemonExportPolicyV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DaemonExportPolicyV1")
            .field("operation", &"<opaque>")
            .field("document_ordinal", &self.document_ordinal)
            .field("expected_revision", &self.expected_revision)
            .field("destination", &"<redacted>")
            .field("diagnostic_capacity", &self.diagnostic_capacity)
            .finish_non_exhaustive()
    }
}

#[cfg(target_os = "macos")]
struct ConfiguredExportCopyAuthorityV1 {
    context: vellum_host::authority::AuthorityContextV1,
    expected_revision: u64,
    destination: PathBuf,
    operation_resource: AuthorityResourceRefV1,
    available: bool,
}

#[cfg(target_os = "macos")]
impl ExportCopyAuthorityPortV1 for ConfiguredExportCopyAuthorityV1 {
    fn authorize_export_copy(
        &mut self,
        source: ExportCopySourceRequestV1,
        destination: SaveAsDestinationRequestV1,
    ) -> ExportCopyAuthorityDecisionV1 {
        if !self.available
            || source.authority().context != self.context
            || destination.context != self.context
            || source.authority().resource != destination.resource
            || source.authority().resource != self.operation_resource
            || source.authority().action != AuthorityActionV1::ExportSourceCopy
            || source.source().revision != self.expected_revision
            || destination.destination() != self.destination
            || destination.effect != SaveAsDestinationEffectV1::CreateExclusive
        {
            return ExportCopyAuthorityDecisionV1::Denied;
        }
        let object = match install_local_export_destination(&destination) {
            Ok(object) => object,
            Err(_) => return ExportCopyAuthorityDecisionV1::Unavailable,
        };
        self.available = false;
        ExportCopyAuthorityDecisionV1::Granted {
            source: ExportCopySourceDecisionEvidenceV1::new(
                AuthorityDecisionRefV1::new(*b"vellum-daemonsrc"),
                source,
            ),
            destination: SaveAsDestinationEvidenceV1::with_object(
                SaveAsDestinationDecisionRefV1::new(*b"vellum-daemondst"),
                destination,
                object,
            ),
        }
    }
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaemonBuildError {
    InvalidLimits,
    ParentMissing,
    ParentNotDirectory,
    ParentPermissions,
    ParentOwnership,
    LockUnsafe,
    LockBusy,
    LockLost,
    SocketExists,
    Inspection,
    Bind(io::ErrorKind),
    SocketPermissions,
    SocketOwnership,
    WorkspaceIdentityReservation,
    WorkspaceIdentity(WorkspaceIdentityRecoveryRefusalV1),
    RequestJournal(RequestJournalRefusalV1),
    StaleSocketUnproven,
    Service(ServiceBuildError),
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceIdentityStartupV1 {
    Initial,
    Recovered,
}

#[cfg(target_os = "macos")]
#[derive(Debug)]
pub enum StartupWorkspaceError {
    RootCapacity,
    LocatorLimit,
    Inspection,
    UnsupportedResource,
    Workspace(WorkspaceError),
}

#[cfg(target_os = "macos")]
pub fn compose_startup_workspace(
    roots: &[PathBuf],
    config: WorkspaceConfig,
    limits: DaemonLimits,
) -> Result<Workspace, StartupWorkspaceError> {
    if roots.len() > limits.max_startup_roots {
        return Err(StartupWorkspaceError::RootCapacity);
    }
    let mut workspace = Workspace::empty(config).map_err(StartupWorkspaceError::Workspace)?;
    let mut discovery = LocalDirectoryDiscovery;
    for root in roots {
        if root.as_os_str().as_encoded_bytes().len() > limits.service.max_locator_bytes {
            return Err(StartupWorkspaceError::LocatorLimit);
        }
        let metadata = fs::symlink_metadata(root).map_err(|_| StartupWorkspaceError::Inspection)?;
        if metadata.file_type().is_symlink() {
            return Err(StartupWorkspaceError::UnsupportedResource);
        }
        if metadata.file_type().is_file() {
            workspace
                .add_file(root)
                .map_err(StartupWorkspaceError::Workspace)?;
        } else if metadata.file_type().is_dir() {
            workspace
                .add_directory(root, limits.service.directory_discovery, &mut discovery)
                .map_err(StartupWorkspaceError::Workspace)?;
        } else {
            return Err(StartupWorkspaceError::UnsupportedResource);
        }
    }
    Ok(workspace)
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaemonServeError {
    OwnershipLost,
    ShutdownRequested,
    NotReady,
    Capacity,
    LifecycleInvariant,
    Accept,
    /// A required local authentication handshake did not establish the exact
    /// server-issued connection session. Rejection, provider unavailability,
    /// replay and malformed/transplanted evidence share this projection.
    Authentication,
    Request(TransportError),
}

/// Closed local-daemon authentication profile. Owner-only is the explicit
/// offline default; it is endpoint access control, not principal identity.
#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaemonAuthenticationProfileV1 {
    OwnerOnlyOffline,
    Required,
}

/// Content-free RED/USE evidence for the local authentication boundary.
/// Refusal reasons are intentionally coalesced to avoid a principal oracle.
#[cfg(unix)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DaemonAuthenticationUsageV1 {
    pub handshakes_started: u64,
    pub handshakes_completed: u64,
    pub handshakes_refused: u64,
    pub handshake_io_failures: u64,
    pub sessions_issued: u64,
    pub sessions_closed: u64,
    pub active_sessions: usize,
    pub session_capacity: usize,
    pub session_identity_exhausted: bool,
    pub randomness_unavailable: bool,
    pub counter_saturated: bool,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownControlError {
    DifferentParent,
    OwnershipLost,
    SocketExists,
    Inspection,
    Bind(io::ErrorKind),
    SocketPermissions,
    SocketOwnership,
    Accept,
    Io,
    InvalidRequest,
    Wake,
    StaleSocketUnproven,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DaemonUsage {
    pub accepted_connections: u64,
    pub completed_connections: u64,
    pub accept_failures: u64,
    /// Accepted exchanges whose I/O returned the closed OS timeout classes
    /// under the configured daemon deadline.
    pub request_deadline_exhausted: u64,
    pub request_failures: u64,
    pub drain_started: u64,
    pub drain_replayed: u64,
    pub drain_completed: u64,
    pub operational_probes: u64,
    pub health_failures: u64,
    /// SIGINT/SIGTERM notifications observed by the installed supervisor.
    pub termination_signals: u64,
    /// First termination notifications which successfully entered the ordinary
    /// owner-only shutdown-control path.
    pub signal_shutdown_requests: u64,
    /// Later termination notifications after that path was selected.
    pub replayed_termination_signals: u64,
    /// Signal notifications whose ordinary shutdown-control request failed.
    pub signal_shutdown_failures: u64,
    pub stale_data_socket_recoveries: u64,
    pub stale_control_socket_recoveries: u64,
    pub durable_intents_started: u64,
    pub durable_receipts_completed: u64,
    pub durable_receipts_replayed: u64,
    pub durable_receipt_conflicts: u64,
    pub durable_receipt_capacity_full: u64,
    pub durable_journal_failures: u64,
    pub durable_operation_executes: u64,
    pub durable_operation_lookups: u64,
    pub durable_operation_retired_replays: u64,
    pub durable_lookup_complete: u64,
    pub durable_lookup_conflict: u64,
    pub durable_lookup_pending: u64,
    pub durable_lookup_absent_or_foreign: u64,
    pub durable_lookup_unavailable: u64,
    pub durable_retirement_requests: u64,
    pub durable_retirement_retired: u64,
    pub durable_retirement_absent_or_foreign: u64,
    pub durable_retirement_conflicts: u64,
    pub durable_retirement_pending: u64,
    pub durable_retirement_unavailable: u64,
    pub durable_retirement_outcome_unknown: u64,
    pub authentication: DaemonAuthenticationUsageV1,
    pub counter_saturated: bool,
    pub in_flight_connections: usize,
    pub in_flight_capacity: usize,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaemonLifecycle {
    Running,
    Draining,
    Stopped,
    Fenced,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaemonHealth {
    Healthy,
    Fenced,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaemonReadiness {
    Ready,
    NotReady,
}

/// Process liveness proven by a reply from the bound private control socket.
#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaemonLiveness {
    Live,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DaemonOperationalStatus {
    pub lifecycle: DaemonLifecycle,
    pub health: DaemonHealth,
    pub readiness: DaemonReadiness,
    pub in_flight: usize,
    pub in_flight_capacity: usize,
}

/// Fixed-cardinality resource gauges sampled at one in-process observation point.
#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationalUtilization {
    pub in_flight_connections: usize,
    pub in_flight_capacity: usize,
    pub active_clients: usize,
    pub client_capacity: usize,
    pub issued_clients: usize,
    pub issued_client_capacity: usize,
    pub retired_clients: usize,
    pub retained_requests: usize,
    pub request_capacity: usize,
    pub retained_witnesses: usize,
    pub witness_capacity: usize,
    /// Retained durable journal slots across both versioned relations. This
    /// includes pending operation intents because they consume the same shared
    /// daemon capacity. It is an aggregate gauge with no principal, request,
    /// path, source, or operation dimensions.
    pub durable_receipts: usize,
    pub durable_receipt_capacity: usize,
    pub retired_durable_operations: usize,
    pub retired_durable_operation_capacity: usize,
}

/// Closed saturation predicates; no field is derived from user-controlled labels.
#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationalSaturation {
    pub in_flight: bool,
    pub clients: bool,
    pub client_lifetime: bool,
    pub requests: bool,
    pub witnesses: bool,
    pub durable_receipts: bool,
    pub retired_durable_operations: bool,
    pub daemon_counters: bool,
    pub service_counters: bool,
    pub counters: bool,
    pub request_latency: bool,
    pub workspace_identity: bool,
    pub events: bool,
    pub authentication: bool,
}

/// One bounded, content-free USE/RED and golden-signal projection.
///
/// This is a passive in-process sample. It neither revalidates ownership nor
/// updates probe counters, so taking it cannot affect lifecycle or service
/// decisions. Endpoint verification remains part of admission and lifecycle
/// transition operations.
#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DaemonOperationalSnapshot {
    pub status: DaemonOperationalStatus,
    pub daemon: DaemonUsage,
    pub service: ServiceUsage,
    pub utilization: OperationalUtilization,
    pub saturation: OperationalSaturation,
    pub request_latency: Option<FixedDurationHistogramSnapshot<REQUEST_LATENCY_BUCKETS>>,
    pub workspace_identity: Option<WorkspaceIdentityOperationalUsageV1>,
    pub events: OperationalEventUsage,
}

/// Version-one, content-free projection exported by the private control socket.
///
/// It deliberately omits counters, paths, principals, identities, source,
/// request arguments, and raw errors. A sample is observationally pure: only
/// daemon state transitions publish a replacement value.
#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DaemonControlStatusV1 {
    pub liveness: DaemonLiveness,
    pub lifecycle: DaemonLifecycle,
    pub health: DaemonHealth,
    pub readiness: DaemonReadiness,
    pub utilization: OperationalUtilization,
    pub saturation: OperationalSaturation,
}

/// Content-free identity persistence evidence. It covers the pre-publication
/// startup load/commit and any pre-reply runtime membership checkpoints; it is
/// never authorization input.
#[cfg(unix)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkspaceIdentityOperationalUsageV1 {
    pub initial: bool,
    pub recovered: bool,
    pub load_attempts: u64,
    pub load_refused: u64,
    pub store_attempts: u64,
    pub committed: u64,
    pub store_refused: u64,
    pub outcome_unknown: u64,
    /// True only for a composition which has no retained metadata checkpoint
    /// port. Such a composition cannot durably expose runtime membership.
    pub runtime_membership_mutation_disabled: bool,
    pub counter_saturated: bool,
}

/// Process-local correlation token, deliberately unrelated to any principal,
/// filesystem locator, document, client, or protocol request identity.
#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationalCorrelationId(pub u64);

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationalLifecycleEvent {
    Ready,
    DrainStarted,
    DrainReplayed,
    DrainCompleted,
    DrainPending,
    Fenced,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationalRequestEvent {
    Accepted,
    Completed,
}

/// Coarse operational classes which cannot disclose service refusal evidence.
#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationalRefusalEvent {
    Shutdown,
    NotReady,
    Capacity,
    Ownership,
    Accept,
    Authentication,
    Transport,
    LifecycleInvariant,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationalEventKind {
    Lifecycle(OperationalLifecycleEvent),
    Request(OperationalRequestEvent),
    Refusal(OperationalRefusalEvent),
}

/// Fixed-cardinality, content-free operational event.
#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationalEvent {
    pub sequence: u64,
    pub correlation: Option<OperationalCorrelationId>,
    pub kind: OperationalEventKind,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationalEventSinkError;

#[cfg(unix)]
pub trait OperationalEventSink: Send {
    fn emit(&mut self, event: OperationalEvent) -> Result<(), OperationalEventSinkError>;
}

/// Reference newline-delimited sink for the standalone process. The schema has
/// only fixed keys and closed enum values; correlation is process-local.
#[cfg(unix)]
#[derive(Clone, Copy, Debug, Default)]
pub struct StderrOperationalEventSink;

#[cfg(unix)]
impl OperationalEventSink for StderrOperationalEventSink {
    fn emit(&mut self, event: OperationalEvent) -> Result<(), OperationalEventSinkError> {
        let correlation = event
            .correlation
            .map_or_else(|| "none".to_owned(), |id| id.0.to_string());
        writeln!(
            io::stderr().lock(),
            "vellum_event sequence={} correlation={} kind={}",
            event.sequence,
            correlation,
            operational_event_name(event.kind)
        )
        .map_err(|_| OperationalEventSinkError)
    }
}

#[cfg(unix)]
const fn operational_event_name(event: OperationalEventKind) -> &'static str {
    match event {
        OperationalEventKind::Lifecycle(OperationalLifecycleEvent::Ready) => "lifecycle.ready",
        OperationalEventKind::Lifecycle(OperationalLifecycleEvent::DrainStarted) => {
            "lifecycle.drain_started"
        }
        OperationalEventKind::Lifecycle(OperationalLifecycleEvent::DrainReplayed) => {
            "lifecycle.drain_replayed"
        }
        OperationalEventKind::Lifecycle(OperationalLifecycleEvent::DrainCompleted) => {
            "lifecycle.drain_completed"
        }
        OperationalEventKind::Lifecycle(OperationalLifecycleEvent::DrainPending) => {
            "lifecycle.drain_pending"
        }
        OperationalEventKind::Lifecycle(OperationalLifecycleEvent::Fenced) => "lifecycle.fenced",
        OperationalEventKind::Request(OperationalRequestEvent::Accepted) => "request.accepted",
        OperationalEventKind::Request(OperationalRequestEvent::Completed) => "request.completed",
        OperationalEventKind::Refusal(OperationalRefusalEvent::Shutdown) => "refusal.shutdown",
        OperationalEventKind::Refusal(OperationalRefusalEvent::NotReady) => "refusal.not_ready",
        OperationalEventKind::Refusal(OperationalRefusalEvent::Capacity) => "refusal.capacity",
        OperationalEventKind::Refusal(OperationalRefusalEvent::Ownership) => "refusal.ownership",
        OperationalEventKind::Refusal(OperationalRefusalEvent::Accept) => "refusal.accept",
        OperationalEventKind::Refusal(OperationalRefusalEvent::Authentication) => {
            "refusal.authentication"
        }
        OperationalEventKind::Refusal(OperationalRefusalEvent::Transport) => "refusal.transport",
        OperationalEventKind::Refusal(OperationalRefusalEvent::LifecycleInvariant) => {
            "refusal.lifecycle_invariant"
        }
    }
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OperationalEventUsage {
    pub attempted: u64,
    pub delivered: u64,
    pub failed: u64,
    pub panicked: u64,
    pub counter_saturated: bool,
    pub correlation_exhausted: bool,
    pub sequence_exhausted: bool,
}

#[cfg(unix)]
struct OperationalEventPort {
    sink: Box<dyn OperationalEventSink>,
    usage: OperationalEventUsage,
    next_sequence: u64,
    next_correlation: u64,
}

#[cfg(unix)]
impl OperationalEventPort {
    fn new(sink: impl OperationalEventSink + 'static) -> Self {
        Self {
            sink: Box::new(sink),
            usage: OperationalEventUsage::default(),
            next_sequence: 0,
            next_correlation: 0,
        }
    }

    fn next_correlation(&mut self) -> Option<OperationalCorrelationId> {
        if self.usage.correlation_exhausted {
            return None;
        }
        let correlation = OperationalCorrelationId(self.next_correlation);
        if self.next_correlation == u64::MAX {
            self.usage.correlation_exhausted = true;
        } else {
            self.next_correlation += 1;
        }
        Some(correlation)
    }

    fn emit(&mut self, correlation: Option<OperationalCorrelationId>, kind: OperationalEventKind) {
        if self.usage.sequence_exhausted {
            return;
        }
        let sequence = self.next_sequence;
        if self.next_sequence == u64::MAX {
            self.usage.sequence_exhausted = true;
        } else {
            self.next_sequence += 1;
        }
        bump(&mut self.usage.attempted, &mut self.usage.counter_saturated);
        let event = OperationalEvent {
            sequence,
            correlation,
            kind,
        };
        match catch_unwind(AssertUnwindSafe(|| self.sink.emit(event))) {
            Ok(Ok(())) => bump(&mut self.usage.delivered, &mut self.usage.counter_saturated),
            Ok(Err(_)) => bump(&mut self.usage.failed, &mut self.usage.counter_saturated),
            Err(_) => bump(&mut self.usage.panicked, &mut self.usage.counter_saturated),
        }
    }
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrainTransition {
    Started,
    AlreadyDraining,
    AlreadyStopped,
    Fenced,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrainCompletion {
    NotDraining,
    Pending,
    Complete,
    AlreadyComplete,
    Fenced,
}

#[cfg(unix)]
struct LifecycleState {
    obligations: ObligationLedger<ConnectionObligation>,
    fenced: bool,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Ord, PartialOrd, Eq, PartialEq)]
struct ConnectionObligation;

#[cfg(unix)]
impl LifecycleState {
    fn new() -> Self {
        Self {
            obligations: ObligationLedger::new(SERIALIZED_CONNECTION_CAPACITY),
            fenced: false,
        }
    }

    fn in_flight(&self) -> usize {
        self.obligations.obligations().len()
    }

    fn lifecycle(&self) -> DaemonLifecycle {
        if self.fenced {
            DaemonLifecycle::Fenced
        } else {
            match self.obligations.phase() {
                LifecyclePhase::Open => DaemonLifecycle::Running,
                LifecyclePhase::Draining => DaemonLifecycle::Draining,
                LifecyclePhase::Closed => DaemonLifecycle::Stopped,
            }
        }
    }

    fn status(&self) -> DaemonOperationalStatus {
        DaemonOperationalStatus {
            lifecycle: self.lifecycle(),
            health: if self.fenced {
                DaemonHealth::Fenced
            } else {
                DaemonHealth::Healthy
            },
            readiness: if !self.fenced
                && self.obligations.phase() == LifecyclePhase::Open
                && self.in_flight() < SERIALIZED_CONNECTION_CAPACITY
            {
                DaemonReadiness::Ready
            } else {
                DaemonReadiness::NotReady
            },
            in_flight: self.in_flight(),
            in_flight_capacity: SERIALIZED_CONNECTION_CAPACITY,
        }
    }

    fn begin_drain(&mut self) -> DrainTransition {
        match self.lifecycle() {
            DaemonLifecycle::Running => {
                self.obligations.begin_draining();
                DrainTransition::Started
            }
            DaemonLifecycle::Draining => DrainTransition::AlreadyDraining,
            DaemonLifecycle::Stopped => DrainTransition::AlreadyStopped,
            DaemonLifecycle::Fenced => DrainTransition::Fenced,
        }
    }

    fn complete_drain(&mut self) -> DrainCompletion {
        match self.lifecycle() {
            DaemonLifecycle::Running => DrainCompletion::NotDraining,
            DaemonLifecycle::Draining if self.in_flight() != 0 => DrainCompletion::Pending,
            DaemonLifecycle::Draining => {
                self.obligations
                    .close()
                    .expect("an empty draining obligation ledger must close");
                DrainCompletion::Complete
            }
            DaemonLifecycle::Stopped => DrainCompletion::AlreadyComplete,
            DaemonLifecycle::Fenced => DrainCompletion::Fenced,
        }
    }

    fn admit(&mut self) -> Result<(), DaemonServeError> {
        if self.lifecycle() != DaemonLifecycle::Running {
            return Err(if self.fenced {
                DaemonServeError::OwnershipLost
            } else {
                DaemonServeError::NotReady
            });
        }
        match self.obligations.register(ConnectionObligation) {
            Ok(()) => Ok(()),
            Err(LifecycleError::Duplicate(_) | LifecycleError::ObligationLimit) => {
                Err(DaemonServeError::Capacity)
            }
            Err(LifecycleError::NotOpen) => Err(DaemonServeError::NotReady),
            Err(LifecycleError::Unknown(_) | LifecycleError::OutstandingObligations(_)) => {
                Err(DaemonServeError::LifecycleInvariant)
            }
        }
    }

    fn complete_connection(&mut self) -> Result<(), DaemonServeError> {
        self.obligations
            .discharge(ConnectionObligation)
            .map_err(|_| DaemonServeError::LifecycleInvariant)
    }

    fn fence(&mut self) -> bool {
        let changed = !self.fenced;
        self.fenced = true;
        changed
    }
}

#[cfg(unix)]
pub struct LocalDaemon {
    listener: UnixListener,
    service: Service,
    limits: DaemonLimits,
    usage: DaemonUsage,
    socket: SocketGuard,
    ownership: OwnershipGuard,
    lifecycle: LifecycleState,
    shutdown_requested: Arc<AtomicBool>,
    control_status: Arc<Mutex<DaemonControlStatusV1>>,
    request_latency: Option<FixedDurationHistogram<REQUEST_LATENCY_BUCKETS>>,
    workspace_identity: Option<WorkspaceIdentityOperationalUsageV1>,
    workspace_identity_metadata: Option<Box<dyn WorkspaceIdentityMetadataPortV1>>,
    workspace_identity_generation: Option<u64>,
    request_journal: Option<Box<dyn RequestJournalPortV1>>,
    request_journal_scope: Option<RequestJournalScopeV1>,
    durable_receipts: Vec<RequestJournalRecordV1>,
    durable_operations: Vec<DurableOperationRecordV1>,
    retired_durable_operations: Vec<DurableOperationRetirementRecordV1>,
    events: Option<OperationalEventPort>,
    supervision: Arc<SupervisionEvidence>,
    stale_data_socket_recovered: bool,
    authentication: AuthenticationRuntime,
    #[cfg(target_os = "macos")]
    export_policy: Option<DaemonExportPolicyV1>,
}

#[cfg(unix)]
struct CheckpointingService<'a> {
    service: &'a mut Service,
    metadata: &'a mut dyn WorkspaceIdentityMetadataPortV1,
    directory: &'a Path,
    limits: WorkspaceIdentityLimitsV1,
    committed_generation: &'a mut u64,
    usage: &'a mut WorkspaceIdentityOperationalUsageV1,
    checkpoint_failed: &'a mut bool,
    journal: &'a mut dyn RequestJournalPortV1,
    journal_scope: RequestJournalScopeV1,
    journal_limits: RequestJournalLimitsV1,
    durable_receipts: &'a mut Vec<RequestJournalRecordV1>,
    durable_operations: &'a mut Vec<DurableOperationRecordV1>,
    retired_durable_operations: &'a mut Vec<DurableOperationRetirementRecordV1>,
    authenticated_subject: Option<[u8; 16]>,
    authenticated_session: Option<AuthenticatedSessionV1>,
    #[cfg(target_os = "macos")]
    export_policy: Option<&'a DaemonExportPolicyV1>,
    daemon_usage: &'a mut DaemonUsage,
}

#[cfg(unix)]
impl CheckpointingService<'_> {
    fn membership_command(command: &CommandV1) -> bool {
        matches!(
            command,
            CommandV1::AddFile { .. }
                | CommandV1::AddDirectory { .. }
                | CommandV1::RemoveDocument { .. }
                | CommandV1::RemoveRoot { .. }
        )
    }

    fn durable_command(command: &CommandV1) -> bool {
        Self::membership_command(command)
            || matches!(
                command,
                CommandV1::Save { .. } | CommandV1::ReplaceAndSave { .. }
            )
    }

    fn recovered_reply(
        &self,
        request: &RequestV1,
        frozen: &[u8],
        subject: [u8; 16],
    ) -> Option<Result<ReplyV1, ReplyV1>> {
        recovered_durable_reply(self.durable_receipts, request, frozen, subject)
    }

    fn checkpoint_fresh_acceptance(
        &mut self,
        reply: &ReplyV1,
        fresh: bool,
        identity_before: (u64, u64, u64),
    ) -> Result<(), ServiceAccessErrorV1> {
        if !fresh
            || !matches!(reply, ReplyV1::Accepted(_))
            || self.service.identity_revision_evidence() == identity_before
        {
            return Ok(());
        }
        self.checkpoint_membership_state()
    }

    fn checkpoint_membership_state(&mut self) -> Result<(), ServiceAccessErrorV1> {
        let candidate = self.service.identity_metadata(self.limits).map_err(|_| {
            *self.checkpoint_failed = true;
            ServiceAccessErrorV1::OutcomeUnknown
        })?;
        let expected = self.committed_generation.checked_add(1);
        if candidate.generation() != *self.committed_generation
            && Some(candidate.generation()) != expected
        {
            *self.checkpoint_failed = true;
            return Err(ServiceAccessErrorV1::OutcomeUnknown);
        }
        // Even an accepted semantic no-op is checked against the exact
        // already-committed bytes. The adapter's same-generation path accepts
        // only an identical record, so no unchecked acceptance can escape.
        bump(
            &mut self.usage.store_attempts,
            &mut self.usage.counter_saturated,
        );
        match self.metadata.store(self.directory, &candidate, self.limits) {
            Ok(()) => {
                *self.committed_generation = candidate.generation();
                self.journal_scope = RequestJournalScopeV1::new(
                    self.journal_scope.workspace(),
                    candidate.generation(),
                )
                .expect("validated identity metadata has nonzero generation");
                bump(&mut self.usage.committed, &mut self.usage.counter_saturated);
                Ok(())
            }
            Err(WorkspaceIdentityRecoveryRefusalV1::OutcomeUnknown) => {
                bump(
                    &mut self.usage.outcome_unknown,
                    &mut self.usage.counter_saturated,
                );
                *self.checkpoint_failed = true;
                Err(ServiceAccessErrorV1::OutcomeUnknown)
            }
            Err(_) => {
                bump(
                    &mut self.usage.store_refused,
                    &mut self.usage.counter_saturated,
                );
                *self.checkpoint_failed = true;
                Err(ServiceAccessErrorV1::OutcomeUnknown)
            }
        }
    }

    fn checkpoint_durable_acceptance(
        &mut self,
        outcome: &DurableExecuteOutcomeV1,
        identity_before: (u64, u64, u64),
    ) -> Result<(), ServiceAccessErrorV1> {
        if !matches!(outcome, DurableExecuteOutcomeV1::Completed(_))
            || self.service.identity_revision_evidence() == identity_before
        {
            return Ok(());
        }
        self.checkpoint_membership_state()
    }
}

#[cfg(unix)]
fn recovered_durable_reply(
    records: &[RequestJournalRecordV1],
    request: &RequestV1,
    frozen: &[u8],
    subject: [u8; 16],
) -> Option<Result<ReplyV1, ReplyV1>> {
    let record = records.iter().find(|record| {
        record.client.get() == request.client.get() && record.request.get() == request.request.get()
    })?;
    // A subject mismatch is deliberately indistinguishable from absence. It
    // must not disclose whether another principal has a retained receipt.
    if record.authenticated_subject != subject {
        return None;
    }
    if record.frozen_request != frozen {
        return Some(Err(ReplyV1::Refused(
            vellum_protocol::RefusalV1::RequestConflict,
        )));
    }
    let decoded = decode_reply_v1(&record.frozen_reply, vellum_wire::WireLimits::reference())
        .ok()
        .and_then(|reply| match reply {
            WireReplyV1::Executed(reply) => Some(reply),
            _ => None,
        });
    Some(match decoded {
        Some(reply) => Ok(reply),
        None => Err(ReplyV1::Refused(
            vellum_protocol::RefusalV1::EditorUnavailable,
        )),
    })
}

#[cfg(unix)]
fn document_is_issued(
    document: vellum_protocol::WorkspaceDocumentIdV1,
    identity: &WorkspaceIdentityMetadataV1,
) -> bool {
    document.workspace().get() == identity.workspace()
        && document.get() <= identity.document_issued_through()
}

#[cfg(unix)]
fn root_is_issued(
    root: vellum_protocol::WorkspaceRootIdV1,
    identity: &WorkspaceIdentityMetadataV1,
) -> bool {
    root.workspace().get() == identity.workspace() && root.get() <= identity.root_issued_through()
}

#[cfg(unix)]
fn file_admission_matches_identity(
    receipt: &vellum_protocol::FileAdmissionReceiptV1,
    identity: &WorkspaceIdentityMetadataV1,
) -> bool {
    document_is_issued(receipt.document, identity)
        && match (receipt.status, receipt.root) {
            (vellum_protocol::FileAdmissionStatusV1::Added, Some(root)) => {
                root_is_issued(root, identity)
            }
            (vellum_protocol::FileAdmissionStatusV1::AlreadyPresent, None) => true,
            _ => false,
        }
}

#[cfg(unix)]
fn directory_admission_matches_identity(
    receipt: &vellum_protocol::DirectoryAdmissionReceiptV1,
    identity: &WorkspaceIdentityMetadataV1,
) -> bool {
    root_is_issued(receipt.root, identity)
        && receipt
            .added_documents
            .iter()
            .all(|document| document_is_issued(*document, identity))
        && u64::try_from(receipt.added_documents.len()).ok() == Some(receipt.added)
        && receipt
            .added_documents
            .iter()
            .enumerate()
            .all(|(index, document)| !receipt.added_documents[..index].contains(document))
}

#[cfg(unix)]
fn durable_intent_matches_identity(
    intent: &DurableEffectIntentV1,
    identity: &WorkspaceIdentityMetadataV1,
) -> bool {
    match intent {
        DurableEffectIntentV1::AddFile { .. } | DurableEffectIntentV1::AddDirectory { .. } => true,
        DurableEffectIntentV1::RemoveDocument { document }
        | DurableEffectIntentV1::SaveDocument { document, .. }
        | DurableEffectIntentV1::ReplaceDocumentAndSave { document, .. }
        | DurableEffectIntentV1::ExportDocumentCopyExclusive { document, .. } => {
            document_is_issued(*document, identity)
        }
        DurableEffectIntentV1::RemoveRoot { root } => root_is_issued(*root, identity),
    }
}

#[cfg(all(unix, test))]
fn durable_outcome_matches_intent(
    intent: &DurableEffectIntentV1,
    outcome: &DurableExecuteOutcomeV1,
    identity: &WorkspaceIdentityMetadataV1,
) -> bool {
    let receipt = match outcome {
        DurableExecuteOutcomeV1::Completed(receipt) => receipt,
        DurableExecuteOutcomeV1::Refused(_) => return true,
        DurableExecuteOutcomeV1::Retired => return false,
    };
    match (intent, receipt) {
        (DurableEffectIntentV1::AddFile { .. }, DurableEffectReceiptV1::FileAdmitted(receipt)) => {
            file_admission_matches_identity(receipt, identity)
        }
        (
            DurableEffectIntentV1::AddDirectory { .. },
            DurableEffectReceiptV1::DirectoryAdmitted(receipt),
        ) => directory_admission_matches_identity(receipt, identity),
        (
            DurableEffectIntentV1::RemoveDocument { document },
            DurableEffectReceiptV1::DocumentRemoved(receipt),
        ) => receipt.document == *document,
        (
            DurableEffectIntentV1::RemoveRoot { root },
            DurableEffectReceiptV1::RootRemoved(receipt),
        ) => receipt.root == *root,
        (
            DurableEffectIntentV1::SaveDocument {
                document,
                expected_revision,
            },
            DurableEffectReceiptV1::DocumentSaved(receipt),
        ) => receipt.document == *document && receipt.source_revision == *expected_revision,
        (
            DurableEffectIntentV1::ReplaceDocumentAndSave {
                document,
                expected_revision,
                ..
            },
            DurableEffectReceiptV1::DocumentReplacedAndSaved(receipt),
        ) => {
            receipt.document == *document
                && expected_revision
                    .checked_add(1)
                    .is_some_and(|successor| receipt.source_revision == successor)
        }
        (
            DurableEffectIntentV1::ExportDocumentCopyExclusive {
                document,
                expected_revision,
                ..
            },
            DurableEffectReceiptV1::DocumentCopyExportedExclusive(receipt),
        ) => {
            receipt.document() == *document
                && receipt.source_revision() == *expected_revision
                && document_is_issued(*document, identity)
        }
        _ => false,
    }
}

#[cfg(unix)]
fn ordinary_reply_matches_identity(
    command: &CommandV1,
    reply: &ReplyV1,
    identity: &WorkspaceIdentityMetadataV1,
) -> bool {
    let ReplyV1::Accepted(outcome) = reply else {
        return true;
    };
    match (command, outcome) {
        (CommandV1::AddFile { .. }, OutcomeV1::FileAdmitted(receipt)) => {
            file_admission_matches_identity(receipt, identity)
        }
        (CommandV1::AddDirectory { .. }, OutcomeV1::DirectoryAdmitted(receipt)) => {
            directory_admission_matches_identity(receipt, identity)
        }
        (CommandV1::RemoveDocument { document }, OutcomeV1::DocumentRemoved(receipt)) => {
            receipt.document == *document && document_is_issued(*document, identity)
        }
        (CommandV1::RemoveRoot { root }, OutcomeV1::RootRemoved(receipt)) => {
            receipt.root == *root && root_is_issued(*root, identity)
        }
        (CommandV1::Save { .. }, OutcomeV1::Saved(_))
        | (CommandV1::ReplaceAndSave { .. }, OutcomeV1::ReplacedAndSaved(_)) => true,
        _ => false,
    }
}

#[cfg(unix)]
fn ordinary_command_matches_identity(
    command: &CommandV1,
    identity: &WorkspaceIdentityMetadataV1,
) -> bool {
    match command {
        CommandV1::RemoveDocument { document } => document_is_issued(*document, identity),
        CommandV1::RemoveRoot { root } => root_is_issued(*root, identity),
        _ => true,
    }
}

#[cfg(unix)]
impl ServicePort for CheckpointingService<'_> {
    fn connect(&mut self) -> Result<vellum_protocol::ClientId, ServiceControlErrorV1> {
        self.service
            .connect()
            .map_err(ServiceControlErrorV1::Refused)
    }

    fn execute(&mut self, request: RequestV1) -> Result<ReplyV1, ServiceAccessErrorV1> {
        if !Self::durable_command(&request.command) {
            let fresh = self.service.classify_request(&request) == RequestRetentionOutcome::New;
            let identity_before = self.service.identity_revision_evidence();
            let reply = self.service.execute(request);
            self.checkpoint_fresh_acceptance(&reply, fresh, identity_before)?;
            return Ok(reply);
        }
        let Some(authenticated_subject) = self.authenticated_subject else {
            let fresh = self.service.classify_request(&request) == RequestRetentionOutcome::New;
            let identity_before = self.service.identity_revision_evidence();
            let reply = self.service.execute(request.clone());
            self.checkpoint_fresh_acceptance(&reply, fresh, identity_before)?;
            return Ok(reply);
        };
        let frozen_request = encode_request_v1(
            &WireRequestV1::Execute(request.clone()),
            vellum_wire::WireLimits::reference(),
        )
        .map_err(|_| ServiceAccessErrorV1::OutcomeUnknown)?;
        if self.durable_receipts.iter().any(|record| {
            record.client.get() == request.client.get()
                && record.request.get() == request.request.get()
                && record.authenticated_subject != authenticated_subject
        }) {
            // Do not reveal or overwrite another authenticated subject's
            // receipt. Ordinary service validation supplies the same public
            // outcome as an absent inactive client.
            return Ok(self.service.execute(request));
        }
        if let Some(recovered) =
            self.recovered_reply(&request, &frozen_request, authenticated_subject)
        {
            return Ok(match recovered {
                Ok(reply) => {
                    bump(
                        &mut self.daemon_usage.durable_receipts_replayed,
                        &mut self.daemon_usage.counter_saturated,
                    );
                    reply
                }
                Err(reply) => {
                    bump(
                        &mut self.daemon_usage.durable_receipt_conflicts,
                        &mut self.daemon_usage.counter_saturated,
                    );
                    reply
                }
            });
        }
        if !self.service.is_client_active(request.client) {
            return Ok(self.service.execute(request));
        }
        let fresh = self.service.classify_request(&request) == RequestRetentionOutcome::New;
        if !fresh {
            return Ok(self.service.execute(request));
        }
        if self
            .durable_receipts
            .len()
            .checked_add(self.durable_operations.len())
            .is_none_or(|retained| retained >= self.journal_limits.max_entries)
        {
            bump(
                &mut self.daemon_usage.durable_receipt_capacity_full,
                &mut self.daemon_usage.counter_saturated,
            );
            return Ok(ReplyV1::Refused(
                vellum_protocol::RefusalV1::RequestCapacity,
            ));
        }
        let client = std::num::NonZeroU64::new(request.client.get())
            .ok_or(ServiceAccessErrorV1::OutcomeUnknown)?;
        let operation = std::num::NonZeroU64::new(request.request.get())
            .ok_or(ServiceAccessErrorV1::OutcomeUnknown)?;
        let operation_scope = self.journal_scope;
        if let Err(reason) = self.journal.begin(
            self.directory,
            RequestJournalIntentV1 {
                scope: operation_scope,
                client,
                request: operation,
                authenticated_subject,
                frozen_request: &frozen_request,
            },
            self.journal_limits,
        ) {
            return match reason {
                RequestJournalRefusalV1::Full | RequestJournalRefusalV1::Limit => {
                    bump(
                        &mut self.daemon_usage.durable_receipt_capacity_full,
                        &mut self.daemon_usage.counter_saturated,
                    );
                    Ok(ReplyV1::Refused(
                        vellum_protocol::RefusalV1::RequestCapacity,
                    ))
                }
                RequestJournalRefusalV1::Conflict => {
                    bump(
                        &mut self.daemon_usage.durable_receipt_conflicts,
                        &mut self.daemon_usage.counter_saturated,
                    );
                    Ok(ReplyV1::Refused(
                        vellum_protocol::RefusalV1::RequestConflict,
                    ))
                }
                _ => {
                    bump(
                        &mut self.daemon_usage.durable_journal_failures,
                        &mut self.daemon_usage.counter_saturated,
                    );
                    *self.checkpoint_failed = true;
                    Err(ServiceAccessErrorV1::OutcomeUnknown)
                }
            };
        }
        bump(
            &mut self.daemon_usage.durable_intents_started,
            &mut self.daemon_usage.counter_saturated,
        );
        let identity_before = self.service.identity_revision_evidence();
        let reply = self.service.execute(request.clone());
        self.checkpoint_fresh_acceptance(&reply, fresh, identity_before)?;
        let frozen_reply = encode_reply_v1(
            &WireReplyV1::Executed(reply.clone()),
            vellum_wire::WireLimits::reference(),
        )
        .map_err(|_| {
            *self.checkpoint_failed = true;
            ServiceAccessErrorV1::OutcomeUnknown
        })?;
        if self
            .journal
            .complete(
                self.directory,
                RequestJournalCompletionV1 {
                    intent: RequestJournalIntentV1 {
                        scope: operation_scope,
                        client,
                        request: operation,
                        authenticated_subject,
                        frozen_request: &frozen_request,
                    },
                    frozen_reply: &frozen_reply,
                },
                self.journal_limits,
            )
            .is_err()
        {
            bump(
                &mut self.daemon_usage.durable_journal_failures,
                &mut self.daemon_usage.counter_saturated,
            );
            *self.checkpoint_failed = true;
            return Err(ServiceAccessErrorV1::OutcomeUnknown);
        }
        bump(
            &mut self.daemon_usage.durable_receipts_completed,
            &mut self.daemon_usage.counter_saturated,
        );
        self.durable_receipts.push(RequestJournalRecordV1 {
            scope: operation_scope,
            client,
            request: operation,
            authenticated_subject,
            frozen_request,
            frozen_reply,
        });
        Ok(reply)
    }

    fn execute_durable(
        &mut self,
        request: DurableExecuteRequestV1,
    ) -> Result<DurableExecuteOutcomeV1, ServiceAccessErrorV1> {
        bump(
            &mut self.daemon_usage.durable_operation_executes,
            &mut self.daemon_usage.counter_saturated,
        );
        let Some(authenticated_subject) = self.authenticated_subject else {
            return Err(ServiceAccessErrorV1::Unavailable);
        };
        if !self.service.is_client_active(request.client) {
            return Err(ServiceAccessErrorV1::Unavailable);
        }
        let frozen_request = vellum_wire::encode_durable_intent_v1(
            &request.intent,
            vellum_wire::WireLimits::reference(),
        )
        .map_err(|_| ServiceAccessErrorV1::Malformed)?;
        if let Some(retired) = self.retired_durable_operations.iter().find(|record| {
            record.authenticated_subject == authenticated_subject
                && record.operation == request.operation
        }) {
            // Both exact and changed-input delayed execution are absorbing;
            // neither may create a second effect under the retired identity.
            return Ok(if retired.frozen_request == frozen_request {
                bump(
                    &mut self.daemon_usage.durable_operation_retired_replays,
                    &mut self.daemon_usage.counter_saturated,
                );
                DurableExecuteOutcomeV1::Retired
            } else {
                bump(
                    &mut self.daemon_usage.durable_receipt_conflicts,
                    &mut self.daemon_usage.counter_saturated,
                );
                DurableExecuteOutcomeV1::Refused(vellum_protocol::RefusalV1::RequestConflict)
            });
        }
        match lookup_recovered_operation_v1(
            self.durable_operations,
            self.journal_scope,
            authenticated_subject,
            request.operation,
            &frozen_request,
        ) {
            DurableOperationLookupV1::Complete(reply) => {
                bump(
                    &mut self.daemon_usage.durable_receipts_replayed,
                    &mut self.daemon_usage.counter_saturated,
                );
                return vellum_wire::decode_durable_outcome_v1(
                    &reply,
                    vellum_wire::WireLimits::reference(),
                )
                .map_err(|_| ServiceAccessErrorV1::Unavailable);
            }
            DurableOperationLookupV1::RequestConflict => {
                bump(
                    &mut self.daemon_usage.durable_receipt_conflicts,
                    &mut self.daemon_usage.counter_saturated,
                );
                return Ok(DurableExecuteOutcomeV1::Refused(
                    vellum_protocol::RefusalV1::RequestConflict,
                ));
            }
            DurableOperationLookupV1::OutcomeUnknown => {
                return Err(ServiceAccessErrorV1::OutcomeUnknown);
            }
            DurableOperationLookupV1::Unavailable => {
                return Err(ServiceAccessErrorV1::Unavailable);
            }
            DurableOperationLookupV1::AbsentOrForeign => {}
        }
        if self
            .durable_receipts
            .len()
            .checked_add(self.durable_operations.len())
            .is_none_or(|retained| retained >= self.journal_limits.max_entries)
        {
            bump(
                &mut self.daemon_usage.durable_receipt_capacity_full,
                &mut self.daemon_usage.counter_saturated,
            );
            return Ok(DurableExecuteOutcomeV1::Refused(
                vellum_protocol::RefusalV1::RequestCapacity,
            ));
        }
        let operation_scope = self.journal_scope;
        let intent = DurableOperationIntentV1 {
            scope: operation_scope,
            operation: request.operation,
            authenticated_subject,
            frozen_request: &frozen_request,
        };
        match self
            .journal
            .begin_operation(self.directory, intent, self.journal_limits)
        {
            Ok(()) => {}
            Err(RequestJournalRefusalV1::Full | RequestJournalRefusalV1::Limit) => {
                bump(
                    &mut self.daemon_usage.durable_receipt_capacity_full,
                    &mut self.daemon_usage.counter_saturated,
                );
                return Ok(DurableExecuteOutcomeV1::Refused(
                    vellum_protocol::RefusalV1::RequestCapacity,
                ));
            }
            Err(RequestJournalRefusalV1::Conflict) => {
                return Err(ServiceAccessErrorV1::Unavailable);
            }
            Err(_) => {
                bump(
                    &mut self.daemon_usage.durable_journal_failures,
                    &mut self.daemon_usage.counter_saturated,
                );
                *self.checkpoint_failed = true;
                return Err(ServiceAccessErrorV1::OutcomeUnknown);
            }
        }
        bump(
            &mut self.daemon_usage.durable_intents_started,
            &mut self.daemon_usage.counter_saturated,
        );
        let identity_before = self.service.identity_revision_evidence();
        #[cfg(target_os = "macos")]
        let outcome = if let (
            DurableEffectIntentV1::ExportDocumentCopyExclusive {
                document,
                expected_revision,
                destination,
            },
            Some(policy),
            Some(authenticated),
        ) = (
            &request.intent,
            self.export_policy,
            self.authenticated_session.take(),
        ) {
            if request.operation != policy.operation
                || document.get() != policy.document_ordinal
                || *expected_revision != policy.expected_revision
                || Path::new(destination) != policy.destination
            {
                DurableExecuteOutcomeV1::Refused(vellum_protocol::RefusalV1::AuthorityDenied)
            } else {
                let context = authenticated.authority_context();
                self.service.execute_authenticated_export_for_operation(
                    request.client,
                    request.operation,
                    &request.intent,
                    AuthenticatedExportExecutionV1::new(
                        authenticated,
                        ConfiguredExportCopyAuthorityV1 {
                            context,
                            expected_revision: policy.expected_revision,
                            destination: policy.destination.clone(),
                            operation_resource: AuthorityResourceRefV1::new(
                                policy.operation.to_bytes(),
                            ),
                            available: true,
                        },
                        LocalExportCopyEffectV1::new(policy.filesystem),
                        policy.diagnostic_capacity,
                    ),
                )
            }
        } else {
            self.service.execute_durable_effect_for_operation(
                request.client,
                request.operation,
                &request.intent,
            )
        };
        #[cfg(not(target_os = "macos"))]
        let outcome = self.service.execute_durable_effect_for_operation(
            request.client,
            request.operation,
            &request.intent,
        );
        self.checkpoint_durable_acceptance(&outcome, identity_before)?;
        let frozen_reply =
            vellum_wire::encode_durable_outcome_v1(&outcome, vellum_wire::WireLimits::reference())
                .map_err(|_| {
                    *self.checkpoint_failed = true;
                    ServiceAccessErrorV1::OutcomeUnknown
                })?;
        if self
            .journal
            .complete_operation(
                self.directory,
                DurableOperationCompletionV1 {
                    intent,
                    frozen_reply: &frozen_reply,
                },
                self.journal_limits,
            )
            .is_err()
        {
            bump(
                &mut self.daemon_usage.durable_journal_failures,
                &mut self.daemon_usage.counter_saturated,
            );
            *self.checkpoint_failed = true;
            return Err(ServiceAccessErrorV1::OutcomeUnknown);
        }
        bump(
            &mut self.daemon_usage.durable_receipts_completed,
            &mut self.daemon_usage.counter_saturated,
        );
        self.durable_operations.push(DurableOperationRecordV1 {
            scope: operation_scope,
            operation: request.operation,
            authenticated_subject,
            frozen_request,
            frozen_reply: Some(frozen_reply),
        });
        Ok(outcome)
    }

    fn lookup_durable_receipt(
        &mut self,
        lookup: DurableReceiptLookupV1,
    ) -> Result<DurableReceiptLookupOutcomeV1, ServiceAccessErrorV1> {
        bump(
            &mut self.daemon_usage.durable_operation_lookups,
            &mut self.daemon_usage.counter_saturated,
        );
        let Some(authenticated_subject) = self.authenticated_subject else {
            bump(
                &mut self.daemon_usage.durable_lookup_unavailable,
                &mut self.daemon_usage.counter_saturated,
            );
            return Ok(DurableReceiptLookupOutcomeV1::Unavailable);
        };
        if !self.service.is_client_active(lookup.client) {
            bump(
                &mut self.daemon_usage.durable_lookup_unavailable,
                &mut self.daemon_usage.counter_saturated,
            );
            return Ok(DurableReceiptLookupOutcomeV1::Unavailable);
        }
        let frozen_request = match vellum_wire::encode_durable_intent_v1(
            &lookup.intent,
            vellum_wire::WireLimits::reference(),
        ) {
            Ok(request) => request,
            Err(_) => {
                bump(
                    &mut self.daemon_usage.durable_lookup_unavailable,
                    &mut self.daemon_usage.counter_saturated,
                );
                return Ok(DurableReceiptLookupOutcomeV1::Unavailable);
            }
        };
        if let Some(retired) = self.retired_durable_operations.iter().find(|record| {
            record.authenticated_subject == authenticated_subject
                && record.operation == lookup.operation
        }) {
            let outcome = if retired.frozen_request == frozen_request {
                DurableReceiptLookupOutcomeV1::AbsentOrForeign
            } else {
                DurableReceiptLookupOutcomeV1::RequestConflict
            };
            let counter = if matches!(outcome, DurableReceiptLookupOutcomeV1::RequestConflict) {
                &mut self.daemon_usage.durable_lookup_conflict
            } else {
                &mut self.daemon_usage.durable_lookup_absent_or_foreign
            };
            bump(counter, &mut self.daemon_usage.counter_saturated);
            return Ok(outcome);
        }
        let outcome = match lookup_recovered_operation_v1(
            self.durable_operations,
            self.journal_scope,
            authenticated_subject,
            lookup.operation,
            &frozen_request,
        ) {
            DurableOperationLookupV1::Complete(reply) => {
                match vellum_wire::decode_durable_outcome_v1(
                    &reply,
                    vellum_wire::WireLimits::reference(),
                ) {
                    Ok(DurableExecuteOutcomeV1::Retired) => {
                        DurableReceiptLookupOutcomeV1::Unavailable
                    }
                    Ok(outcome) => DurableReceiptLookupOutcomeV1::Complete(outcome),
                    Err(_) => DurableReceiptLookupOutcomeV1::Unavailable,
                }
            }
            DurableOperationLookupV1::RequestConflict => {
                DurableReceiptLookupOutcomeV1::RequestConflict
            }
            DurableOperationLookupV1::OutcomeUnknown => {
                DurableReceiptLookupOutcomeV1::OutcomeUnknown
            }
            DurableOperationLookupV1::AbsentOrForeign => {
                DurableReceiptLookupOutcomeV1::AbsentOrForeign
            }
            DurableOperationLookupV1::Unavailable => DurableReceiptLookupOutcomeV1::Unavailable,
        };
        let counter = match &outcome {
            DurableReceiptLookupOutcomeV1::Complete(_) => {
                &mut self.daemon_usage.durable_lookup_complete
            }
            DurableReceiptLookupOutcomeV1::RequestConflict => {
                &mut self.daemon_usage.durable_lookup_conflict
            }
            DurableReceiptLookupOutcomeV1::OutcomeUnknown => {
                &mut self.daemon_usage.durable_lookup_pending
            }
            DurableReceiptLookupOutcomeV1::AbsentOrForeign => {
                &mut self.daemon_usage.durable_lookup_absent_or_foreign
            }
            DurableReceiptLookupOutcomeV1::Unavailable => {
                &mut self.daemon_usage.durable_lookup_unavailable
            }
        };
        bump(counter, &mut self.daemon_usage.counter_saturated);
        Ok(outcome)
    }

    fn retire_durable_receipt(
        &mut self,
        retirement: DurableReceiptRetirementV1,
    ) -> Result<DurableReceiptRetirementOutcomeV1, ServiceAccessErrorV1> {
        bump(
            &mut self.daemon_usage.durable_retirement_requests,
            &mut self.daemon_usage.counter_saturated,
        );
        let Some(subject) = self.authenticated_subject else {
            bump(
                &mut self.daemon_usage.durable_retirement_unavailable,
                &mut self.daemon_usage.counter_saturated,
            );
            return Ok(DurableReceiptRetirementOutcomeV1::Unavailable);
        };
        if !self.service.is_client_active(retirement.client) {
            bump(
                &mut self.daemon_usage.durable_retirement_unavailable,
                &mut self.daemon_usage.counter_saturated,
            );
            return Ok(DurableReceiptRetirementOutcomeV1::Unavailable);
        }
        let frozen_request = vellum_wire::encode_durable_intent_v1(
            &retirement.intent,
            vellum_wire::WireLimits::reference(),
        )
        .map_err(|_| ServiceAccessErrorV1::Malformed)?;
        if let Some(record) = self.retired_durable_operations.iter().find(|record| {
            record.authenticated_subject == subject && record.operation == retirement.operation
        }) {
            let outcome = if record.frozen_request == frozen_request {
                DurableReceiptRetirementOutcomeV1::AbsentOrForeign
            } else {
                DurableReceiptRetirementOutcomeV1::RequestConflict
            };
            let counter = if matches!(outcome, DurableReceiptRetirementOutcomeV1::RequestConflict) {
                &mut self.daemon_usage.durable_retirement_conflicts
            } else {
                &mut self.daemon_usage.durable_retirement_absent_or_foreign
            };
            bump(counter, &mut self.daemon_usage.counter_saturated);
            return Ok(outcome);
        }
        let Some(index) = self.durable_operations.iter().position(|record| {
            record.authenticated_subject == subject && record.operation == retirement.operation
        }) else {
            bump(
                &mut self.daemon_usage.durable_retirement_absent_or_foreign,
                &mut self.daemon_usage.counter_saturated,
            );
            return Ok(DurableReceiptRetirementOutcomeV1::AbsentOrForeign);
        };
        if self.durable_operations[index].frozen_request != frozen_request {
            bump(
                &mut self.daemon_usage.durable_retirement_conflicts,
                &mut self.daemon_usage.counter_saturated,
            );
            return Ok(DurableReceiptRetirementOutcomeV1::RequestConflict);
        }
        if self.durable_operations[index].frozen_reply.is_none() {
            bump(
                &mut self.daemon_usage.durable_retirement_pending,
                &mut self.daemon_usage.counter_saturated,
            );
            return Ok(DurableReceiptRetirementOutcomeV1::OutcomeUnknown);
        }
        if self.retired_durable_operations.len() >= self.journal_limits.max_retired_operations
            || self
                .retired_durable_operations
                .try_reserve_exact(1)
                .is_err()
        {
            bump(
                &mut self.daemon_usage.durable_retirement_unavailable,
                &mut self.daemon_usage.counter_saturated,
            );
            return Ok(DurableReceiptRetirementOutcomeV1::Unavailable);
        }
        // Passive owner-scoped classification and aggregate capacity checks do
        // not consume the bounded one-shot authority budget. The distinct
        // action gates the complete, memory-reserved mutation candidate.
        if self
            .service
            .authorize_durable_receipt_retirement(retirement.client)
            .is_err()
        {
            bump(
                &mut self.daemon_usage.durable_retirement_unavailable,
                &mut self.daemon_usage.counter_saturated,
            );
            return Ok(DurableReceiptRetirementOutcomeV1::Unavailable);
        }
        let result = self.journal.retire_operation(
            self.directory,
            DurableOperationRetirementV1 {
                scope: self.journal_scope,
                operation: retirement.operation,
                authenticated_subject: subject,
                frozen_request: &frozen_request,
            },
            self.journal_limits,
        );
        match result {
            Ok(DurableOperationRetirementResultV1::Retired) => {
                let record = self.durable_operations.remove(index);
                self.retired_durable_operations
                    .push(DurableOperationRetirementRecordV1 {
                        scope: record.scope,
                        operation: record.operation,
                        authenticated_subject: record.authenticated_subject,
                        frozen_request: record.frozen_request,
                    });
                bump(
                    &mut self.daemon_usage.durable_retirement_retired,
                    &mut self.daemon_usage.counter_saturated,
                );
                Ok(DurableReceiptRetirementOutcomeV1::Retired)
            }
            Ok(DurableOperationRetirementResultV1::AbsentOrForeign) => {
                bump(
                    &mut self.daemon_usage.durable_retirement_absent_or_foreign,
                    &mut self.daemon_usage.counter_saturated,
                );
                Ok(DurableReceiptRetirementOutcomeV1::AbsentOrForeign)
            }
            Ok(DurableOperationRetirementResultV1::RequestConflict) => {
                bump(
                    &mut self.daemon_usage.durable_retirement_conflicts,
                    &mut self.daemon_usage.counter_saturated,
                );
                Ok(DurableReceiptRetirementOutcomeV1::RequestConflict)
            }
            Ok(DurableOperationRetirementResultV1::Pending) => {
                bump(
                    &mut self.daemon_usage.durable_retirement_pending,
                    &mut self.daemon_usage.counter_saturated,
                );
                Ok(DurableReceiptRetirementOutcomeV1::OutcomeUnknown)
            }
            Err(RequestJournalRefusalV1::Full) => {
                bump(
                    &mut self.daemon_usage.durable_retirement_unavailable,
                    &mut self.daemon_usage.counter_saturated,
                );
                Ok(DurableReceiptRetirementOutcomeV1::Unavailable)
            }
            Err(_) => {
                bump(
                    &mut self.daemon_usage.durable_retirement_outcome_unknown,
                    &mut self.daemon_usage.counter_saturated,
                );
                *self.checkpoint_failed = true;
                Err(ServiceAccessErrorV1::OutcomeUnknown)
            }
        }
    }

    fn disconnect(
        &mut self,
        client: vellum_protocol::ClientId,
    ) -> Result<(), ServiceControlErrorV1> {
        self.service
            .disconnect(client)
            .map_err(ServiceControlErrorV1::Refused)
    }
}

#[cfg(unix)]
trait ErasedAuthenticationGate {
    fn establish(
        &mut self,
        request: AuthenticationRequestV1,
    ) -> Result<AuthenticatedSessionV1, AuthenticationRefusalV1>;
}

#[cfg(unix)]
struct OwnedAuthenticationGate<P: AuthenticationPortV1>
where
    P::Context: Sized,
{
    gate: AuthenticationGateV1<P>,
    context: P::Context,
}

#[cfg(unix)]
impl<P> ErasedAuthenticationGate for OwnedAuthenticationGate<P>
where
    P: AuthenticationPortV1,
    P::Context: Sized,
{
    fn establish(
        &mut self,
        request: AuthenticationRequestV1,
    ) -> Result<AuthenticatedSessionV1, AuthenticationRefusalV1> {
        self.gate.establish(request, &self.context)
    }
}

#[cfg(unix)]
enum AuthenticationRuntime {
    OwnerOnlyOffline,
    Required {
        gate: Box<dyn ErasedAuthenticationGate>,
        next_session: u64,
        /// One OS-CSPRNG incarnation. Absence means randomness was unavailable
        /// at configuration and every handshake must fail closed.
        incarnation: Option<[u8; 16]>,
        usage: DaemonAuthenticationUsageV1,
    },
}

#[cfg(unix)]
pub struct ShutdownControl {
    listener: UnixListener,
    socket: SocketGuard,
    wake_socket: PathBuf,
    requested: Arc<AtomicBool>,
    status: Arc<Mutex<DaemonControlStatusV1>>,
    timeout: Duration,
}

/// Installed SIGINT/SIGTERM supervision. Dropping closes the signal iterator
/// and joins its single worker; it never detaches a process-global thread.
#[cfg(unix)]
pub struct SignalSupervisor {
    handle: signal_hook::iterator::Handle,
    worker: Option<JoinHandle<()>>,
}

#[cfg(unix)]
#[derive(Default)]
struct SupervisionEvidence {
    termination_signals: AtomicU64,
    signal_shutdown_requests: AtomicU64,
    replayed_termination_signals: AtomicU64,
    signal_shutdown_failures: AtomicU64,
    stale_control_socket_recoveries: AtomicU64,
    counter_saturated: AtomicBool,
    selected: AtomicBool,
}

/// Preserves the transport's closed error vocabulary while retaining the
/// daemon-local fact that the OS classified an I/O operation as timed out.
#[cfg(unix)]
struct DeadlineStream<'a> {
    stream: &'a mut UnixStream,
    deadline_exhausted: bool,
}

#[cfg(unix)]
impl<'a> DeadlineStream<'a> {
    fn new(stream: &'a mut UnixStream) -> Self {
        Self {
            stream,
            deadline_exhausted: false,
        }
    }

    fn observe<T>(&mut self, result: io::Result<T>) -> io::Result<T> {
        if result.as_ref().is_err_and(|error| {
            matches!(
                error.kind(),
                io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
            )
        }) {
            self.deadline_exhausted = true;
        }
        result
    }
}

#[cfg(unix)]
impl Read for DeadlineStream<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let result = self.stream.read(buffer);
        self.observe(result)
    }
}

#[cfg(unix)]
impl Write for DeadlineStream<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let result = self.stream.write(buffer);
        self.observe(result)
    }

    fn flush(&mut self) -> io::Result<()> {
        let result = self.stream.flush();
        self.observe(result)
    }
}

#[cfg(unix)]
impl LocalDaemon {
    pub fn bind(
        socket: &Path,
        workspace: Workspace,
        limits: DaemonLimits,
    ) -> Result<Self, DaemonBuildError> {
        Self::bind_read_only(socket, workspace, limits)
    }

    fn bind_read_only(
        socket: &Path,
        workspace: Workspace,
        limits: DaemonLimits,
    ) -> Result<Self, DaemonBuildError> {
        #[cfg(target_os = "macos")]
        return Self::bind_inner(socket, workspace, limits, AuthorityActionSetV1::read_only());
        #[cfg(not(target_os = "macos"))]
        Self::bind_inner(socket, workspace, limits)
    }

    /// Trusted local composition with an explicit finite service-action ceiling.
    #[cfg(target_os = "macos")]
    pub fn bind_with_action_ceiling(
        socket: &Path,
        workspace: Workspace,
        limits: DaemonLimits,
        actions: AuthorityActionSetV1,
    ) -> Result<Self, DaemonBuildError> {
        Self::bind_inner(socket, workspace, limits, actions)
    }

    /// Standalone composition with restart-stable workspace scope and
    /// allocator non-reuse. The identity record lives in the already private
    /// socket parent. Its next reservation is committed before the socket is
    /// bound, so a store refusal cannot publish a client endpoint.
    #[cfg(target_os = "macos")]
    pub fn bind_with_persistent_identity(
        socket: &Path,
        workspace: Workspace,
        limits: DaemonLimits,
    ) -> Result<Self, DaemonBuildError> {
        Self::bind_with_persistent_identity_and_action_ceiling(
            socket,
            workspace,
            limits,
            AuthorityActionSetV1::read_only(),
        )
    }

    /// Persistent-identity composition with an explicit finite action ceiling.
    ///
    /// Fresh accepted membership transitions are durably checkpointed through
    /// the retained metadata port before a wire reply can escape. A refused or
    /// indeterminate checkpoint fences the daemon and closes that exchange.
    #[cfg(target_os = "macos")]
    pub fn bind_with_persistent_identity_and_action_ceiling(
        socket: &Path,
        workspace: Workspace,
        limits: DaemonLimits,
        actions: AuthorityActionSetV1,
    ) -> Result<Self, DaemonBuildError> {
        Self::bind_with_workspace_identity_metadata_and_action_ceiling(
            socket,
            workspace,
            limits,
            actions,
            LocalWorkspaceIdentityMetadataV1::default(),
        )
    }

    /// Trusted adapter composition seam. The supplied port remains owned by
    /// the daemon for pre-reply membership checkpoints.
    #[cfg(target_os = "macos")]
    pub fn bind_with_workspace_identity_metadata_and_action_ceiling<
        M: WorkspaceIdentityMetadataPortV1 + 'static,
    >(
        socket: &Path,
        workspace: Workspace,
        limits: DaemonLimits,
        actions: AuthorityActionSetV1,
        metadata: M,
    ) -> Result<Self, DaemonBuildError> {
        Self::bind_with_persistence_ports_and_action_ceiling(
            socket,
            workspace,
            limits,
            actions,
            metadata,
            LocalRequestJournalV1::default(),
        )
    }

    /// Trusted composition seam for deterministic durability faults and
    /// alternative private journal mechanisms. Domain replay policy remains
    /// Vellum-owned; this port stores only canonical bounded bytes.
    #[cfg(target_os = "macos")]
    pub fn bind_with_persistence_ports_and_action_ceiling<
        M: WorkspaceIdentityMetadataPortV1 + 'static,
        J: RequestJournalPortV1 + 'static,
    >(
        socket: &Path,
        workspace: Workspace,
        limits: DaemonLimits,
        actions: AuthorityActionSetV1,
        metadata: M,
        journal: J,
    ) -> Result<Self, DaemonBuildError> {
        Self::bind_with_service(
            socket,
            workspace,
            limits,
            Some(Box::new(metadata)),
            Some(Box::new(journal)),
            move |workspace, limits, identity| match (
                actions.contains(AuthorityActionV1::DiscoverDirectory),
                identity,
            ) {
                (true, Some(identity)) => {
                    Service::new_with_directory_discovery_recovered_identity_and_action_ceiling(
                        workspace,
                        limits,
                        LocalDirectoryDiscovery,
                        identity,
                        actions,
                    )
                }
                (true, None) => Service::new_with_directory_discovery_and_action_ceiling(
                    workspace,
                    limits,
                    LocalDirectoryDiscovery,
                    actions,
                ),
                (false, Some(identity)) => Service::new_with_recovered_identity_and_action_ceiling(
                    workspace, limits, identity, actions,
                ),
                (false, None) => Service::new_with_action_ceiling(workspace, limits, actions),
            },
        )
    }

    #[cfg(target_os = "macos")]
    fn bind_inner(
        socket: &Path,
        workspace: Workspace,
        limits: DaemonLimits,
        actions: AuthorityActionSetV1,
    ) -> Result<Self, DaemonBuildError> {
        Self::bind_with_service(
            socket,
            workspace,
            limits,
            None,
            None,
            |workspace, limits, _| {
                if actions.contains(AuthorityActionV1::DiscoverDirectory) {
                    Service::new_with_directory_discovery_and_action_ceiling(
                        workspace,
                        limits,
                        LocalDirectoryDiscovery,
                        actions,
                    )
                } else {
                    Service::new_with_action_ceiling(workspace, limits, actions)
                }
            },
        )
    }

    #[cfg(not(target_os = "macos"))]
    fn bind_inner(
        socket: &Path,
        workspace: Workspace,
        limits: DaemonLimits,
    ) -> Result<Self, DaemonBuildError> {
        Self::bind_with_service(
            socket,
            workspace,
            limits,
            None,
            None,
            |workspace, limits, _| Service::new(workspace, limits),
        )
    }

    fn bind_with_service(
        socket: &Path,
        workspace: Workspace,
        limits: DaemonLimits,
        mut identity_metadata: Option<Box<dyn WorkspaceIdentityMetadataPortV1>>,
        mut request_journal: Option<Box<dyn RequestJournalPortV1>>,
        build_service: impl FnOnce(
            Workspace,
            ServiceLimits,
            Option<WorkspaceIdentityMetadataV1>,
        ) -> Result<Service, ServiceBuildError>,
    ) -> Result<Self, DaemonBuildError> {
        validate_limits(limits)?;
        let request_latency = limits
            .request_latency
            .map(|profile| FixedDurationHistogram::try_new(profile.upper_bounds))
            .transpose()
            .map_err(|_| DaemonBuildError::InvalidLimits)?;
        let parent = socket.parent().ok_or(DaemonBuildError::ParentMissing)?;
        let ownership = OwnershipGuard::acquire(parent)?;
        let persistent_identity = identity_metadata.is_some();
        let stale_data_socket_recovered = if persistent_identity {
            ownership
                .recover_stale_endpoint(socket, EndpointKind::Data)
                .map_err(|()| DaemonBuildError::StaleSocketUnproven)?
        } else {
            false
        };
        match fs::symlink_metadata(socket) {
            Ok(_) => return Err(DaemonBuildError::SocketExists),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return Err(DaemonBuildError::Inspection),
        }
        #[cfg(target_os = "macos")]
        let (
            mut service,
            workspace_identity,
            workspace_identity_generation,
            request_journal_scope,
            workspace_identity_evidence,
        ) = if let Some(metadata) = identity_metadata.as_deref_mut() {
            let mut usage = WorkspaceIdentityOperationalUsageV1 {
                load_attempts: 1,
                ..WorkspaceIdentityOperationalUsageV1::default()
            };
            let recovered =
                match metadata.load(&ownership.directory_path, limits.workspace_identity) {
                    WorkspaceIdentityRecoveryV1::Recovered(identity) => Some(identity),
                    WorkspaceIdentityRecoveryV1::Refused(
                        WorkspaceIdentityRecoveryRefusalV1::Missing,
                    ) => {
                        if !ownership.lock_created {
                            return Err(DaemonBuildError::WorkspaceIdentity(
                                WorkspaceIdentityRecoveryRefusalV1::Missing,
                            ));
                        }
                        usage.load_refused = 1;
                        None
                    }
                    WorkspaceIdentityRecoveryV1::Refused(reason) => {
                        return Err(DaemonBuildError::WorkspaceIdentity(reason));
                    }
                };
            let startup = if recovered.is_some() {
                WorkspaceIdentityStartupV1::Recovered
            } else {
                WorkspaceIdentityStartupV1::Initial
            };
            let mut service = build_service(workspace, limits.service, recovered)
                .map_err(DaemonBuildError::Service)?;
            let reservation = service
                .identity_metadata(limits.workspace_identity)
                .map_err(|_| DaemonBuildError::WorkspaceIdentityReservation)?;
            usage.store_attempts = 1;
            if let Err(reason) = metadata.store(
                &ownership.directory_path,
                &reservation,
                limits.workspace_identity,
            ) {
                return Err(DaemonBuildError::WorkspaceIdentity(reason));
            }
            usage.committed = 1;
            usage.initial = startup == WorkspaceIdentityStartupV1::Initial;
            usage.recovered = startup == WorkspaceIdentityStartupV1::Recovered;
            usage.runtime_membership_mutation_disabled = false;
            ownership.verify().map_err(|_| DaemonBuildError::LockLost)?;
            let journal_scope =
                RequestJournalScopeV1::new(reservation.workspace(), reservation.generation())
                    .ok_or(DaemonBuildError::InvalidLimits)?;
            (
                service,
                Some(usage),
                Some(reservation.generation()),
                Some(journal_scope),
                Some(reservation),
            )
        } else {
            (
                build_service(workspace, limits.service, None)
                    .map_err(DaemonBuildError::Service)?,
                None,
                None,
                None,
                None,
            )
        };
        #[cfg(not(target_os = "macos"))]
        let (
            mut service,
            workspace_identity,
            workspace_identity_generation,
            request_journal_scope,
            workspace_identity_evidence,
        ) = {
            debug_assert!(identity_metadata.is_none());
            (
                build_service(workspace, limits.service, None)
                    .map_err(DaemonBuildError::Service)?,
                None,
                None,
                None,
                None,
            )
        };
        #[cfg(target_os = "macos")]
        let (request_journal, durable_receipts, durable_operations, retired_durable_operations) =
            if let Some(scope) = request_journal_scope {
                let identity = workspace_identity_evidence
                    .as_ref()
                    .expect("persistent journal scope has identity evidence");
                let mut journal = request_journal
                    .take()
                    .ok_or(DaemonBuildError::InvalidLimits)?;
                let receipts =
                    match journal.recover(&ownership.directory_path, scope, limits.request_journal)
                    {
                        RequestJournalRecoveryV1::Empty => Vec::new(),
                        RequestJournalRecoveryV1::Recovered(receipts) => receipts,
                        RequestJournalRecoveryV1::Refused(reason) => {
                            return Err(DaemonBuildError::RequestJournal(reason));
                        }
                    };
                let mut highest_client = 0;
                for record in &receipts {
                    let request = decode_request_v1(
                        &record.frozen_request,
                        vellum_wire::WireLimits::reference(),
                    )
                    .map_err(|_| {
                        DaemonBuildError::RequestJournal(RequestJournalRefusalV1::Malformed)
                    })?;
                    let reply =
                        decode_reply_v1(&record.frozen_reply, vellum_wire::WireLimits::reference())
                            .map_err(|_| {
                                DaemonBuildError::RequestJournal(RequestJournalRefusalV1::Malformed)
                            })?;
                    let WireRequestV1::Execute(decoded) = request else {
                        return Err(DaemonBuildError::RequestJournal(
                            RequestJournalRefusalV1::Malformed,
                        ));
                    };
                    let WireReplyV1::Executed(decoded_reply) = reply else {
                        return Err(DaemonBuildError::RequestJournal(
                            RequestJournalRefusalV1::Conflict,
                        ));
                    };
                    if decoded.client.get() != record.client.get()
                        || decoded.request.get() != record.request.get()
                        || !CheckpointingService::durable_command(&decoded.command)
                        || !ordinary_command_matches_identity(&decoded.command, identity)
                        || !ordinary_reply_matches_identity(
                            &decoded.command,
                            &decoded_reply,
                            identity,
                        )
                    {
                        return Err(DaemonBuildError::RequestJournal(
                            RequestJournalRefusalV1::Conflict,
                        ));
                    }
                    highest_client = highest_client.max(record.client.get());
                }
                service
                    .reserve_client_ordinals_through(highest_client)
                    .map_err(|_| {
                        DaemonBuildError::RequestJournal(RequestJournalRefusalV1::Limit)
                    })?;
                let operations = match journal.recover_operations(
                    &ownership.directory_path,
                    scope,
                    limits.request_journal,
                ) {
                    DurableOperationRecoveryV1::Empty => Vec::new(),
                    DurableOperationRecoveryV1::Recovered(records) => records,
                    DurableOperationRecoveryV1::Refused(reason) => {
                        return Err(DaemonBuildError::RequestJournal(reason));
                    }
                };
                let mut operation_keys = HashSet::with_capacity(operations.len());
                for record in &operations {
                    if !operation_keys
                        .insert((record.authenticated_subject, record.operation.to_bytes()))
                    {
                        return Err(DaemonBuildError::RequestJournal(
                            RequestJournalRefusalV1::Conflict,
                        ));
                    }
                    let intent = vellum_wire::decode_durable_intent_v1(
                        &record.frozen_request,
                        vellum_wire::WireLimits::reference(),
                    )
                    .map_err(|_| {
                        DaemonBuildError::RequestJournal(RequestJournalRefusalV1::Malformed)
                    })?;
                    let canonical_intent = vellum_wire::encode_durable_intent_v1(
                        &intent,
                        vellum_wire::WireLimits::reference(),
                    )
                    .map_err(|_| {
                        DaemonBuildError::RequestJournal(RequestJournalRefusalV1::Malformed)
                    })?;
                    let owner = DurableOperationOwnerV1 {
                        scope,
                        authenticated_subject: record.authenticated_subject,
                    };
                    let validation = if let Some(reply) = &record.frozen_reply {
                        let outcome = vellum_wire::decode_durable_outcome_v1(
                            reply,
                            vellum_wire::WireLimits::reference(),
                        )
                        .map_err(|_| {
                            DaemonBuildError::RequestJournal(RequestJournalRefusalV1::Malformed)
                        })?;
                        let canonical_outcome = vellum_wire::encode_durable_outcome_v1(
                            &outcome,
                            vellum_wire::WireLimits::reference(),
                        )
                        .map_err(|_| {
                            DaemonBuildError::RequestJournal(RequestJournalRefusalV1::Malformed)
                        })?;
                        validate_recovered_durable_effect_v1(
                            record,
                            owner,
                            CanonicalDurableEffectV1 {
                                intent: &intent,
                                canonical_intent: &canonical_intent,
                                outcome: CanonicalDurableOutcomeV1::Complete {
                                    outcome: &outcome,
                                    canonical: &canonical_outcome,
                                },
                            },
                            identity,
                        )
                    } else {
                        validate_recovered_durable_effect_v1(
                            record,
                            owner,
                            CanonicalDurableEffectV1 {
                                intent: &intent,
                                canonical_intent: &canonical_intent,
                                outcome: CanonicalDurableOutcomeV1::Pending,
                            },
                            identity,
                        )
                    };
                    validation.map_err(|_| {
                        DaemonBuildError::RequestJournal(RequestJournalRefusalV1::Conflict)
                    })?;
                }
                if receipts
                    .len()
                    .checked_add(operations.len())
                    .is_none_or(|retained| retained > limits.request_journal.max_entries)
                {
                    return Err(DaemonBuildError::RequestJournal(
                        RequestJournalRefusalV1::Limit,
                    ));
                }
                let retired = match journal.recover_operation_retirements(
                    &ownership.directory_path,
                    scope,
                    limits.request_journal,
                ) {
                    DurableOperationRetirementRecoveryV1::Empty => Vec::new(),
                    DurableOperationRetirementRecoveryV1::Recovered(records) => records,
                    DurableOperationRetirementRecoveryV1::Refused(reason) => {
                        return Err(DaemonBuildError::RequestJournal(reason));
                    }
                };
                let mut retired_keys = HashSet::with_capacity(retired.len());
                for record in &retired {
                    let retired_intent = vellum_wire::decode_durable_intent_v1(
                        &record.frozen_request,
                        vellum_wire::WireLimits::reference(),
                    )
                    .map_err(|_| {
                        DaemonBuildError::RequestJournal(RequestJournalRefusalV1::Malformed)
                    })?;
                    if !retired_keys
                        .insert((record.authenticated_subject, record.operation.to_bytes()))
                        || operation_keys
                            .contains(&(record.authenticated_subject, record.operation.to_bytes()))
                        || !durable_intent_matches_identity(&retired_intent, identity)
                    {
                        return Err(DaemonBuildError::RequestJournal(
                            RequestJournalRefusalV1::Conflict,
                        ));
                    }
                }
                (Some(journal), receipts, operations, retired)
            } else {
                (None, Vec::new(), Vec::new(), Vec::new())
            };
        #[cfg(not(target_os = "macos"))]
        let (request_journal, durable_receipts, durable_operations, retired_durable_operations) =
            (None, Vec::new(), Vec::new(), Vec::new());
        let listener =
            UnixListener::bind(socket).map_err(|error| DaemonBuildError::Bind(error.kind()))?;
        let metadata = fs::symlink_metadata(socket).map_err(|_| DaemonBuildError::Inspection)?;
        if !metadata.file_type().is_socket() {
            return Err(DaemonBuildError::Inspection);
        }
        let socket = SocketGuard {
            path: socket.to_path_buf(),
            identity: Identity::of(&metadata),
        };
        fs::set_permissions(&socket.path, fs::Permissions::from_mode(0o600))
            .map_err(|_| DaemonBuildError::SocketPermissions)?;
        socket.verify()?;
        if persistent_identity {
            ownership
                .record_endpoint(EndpointKind::Data, &socket)
                .map_err(|()| DaemonBuildError::StaleSocketUnproven)?;
        }
        ownership.verify().map_err(|_| DaemonBuildError::LockLost)?;
        let daemon = Self {
            listener,
            service,
            limits,
            usage: DaemonUsage::default(),
            socket,
            ownership,
            lifecycle: LifecycleState::new(),
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            control_status: Arc::new(Mutex::new(DaemonControlStatusV1 {
                liveness: DaemonLiveness::Live,
                lifecycle: DaemonLifecycle::Running,
                health: DaemonHealth::Healthy,
                readiness: DaemonReadiness::Ready,
                utilization: OperationalUtilization {
                    in_flight_connections: 0,
                    in_flight_capacity: 0,
                    active_clients: 0,
                    client_capacity: 0,
                    issued_clients: 0,
                    issued_client_capacity: 0,
                    retired_clients: 0,
                    retained_requests: 0,
                    request_capacity: 0,
                    retained_witnesses: 0,
                    witness_capacity: 0,
                    durable_receipts: 0,
                    durable_receipt_capacity: 0,
                    retired_durable_operations: 0,
                    retired_durable_operation_capacity: 0,
                },
                saturation: OperationalSaturation {
                    in_flight: false,
                    clients: false,
                    client_lifetime: false,
                    requests: false,
                    witnesses: false,
                    durable_receipts: false,
                    retired_durable_operations: false,
                    daemon_counters: false,
                    service_counters: false,
                    counters: false,
                    request_latency: false,
                    workspace_identity: false,
                    events: false,
                    authentication: false,
                },
            })),
            request_latency,
            workspace_identity,
            workspace_identity_metadata: identity_metadata,
            workspace_identity_generation,
            request_journal,
            request_journal_scope,
            durable_receipts,
            durable_operations,
            retired_durable_operations,
            events: None,
            supervision: Arc::new(SupervisionEvidence::default()),
            stale_data_socket_recovered,
            authentication: AuthenticationRuntime::OwnerOnlyOffline,
            #[cfg(target_os = "macos")]
            export_policy: None,
        };
        daemon.publish_control_status();
        Ok(daemon)
    }

    /// Requires an exact per-connection authentication handshake before any
    /// Vellum frame is decoded. The opaque adapter context remains local to
    /// this composition; the resulting subject/session is deliberately not
    /// used as service action authority.
    pub fn require_authentication<P>(
        mut self,
        port: P,
        context: P::Context,
        max_attempts: u64,
    ) -> Self
    where
        P: AuthenticationPortV1 + 'static,
        P::Context: Sized + 'static,
    {
        let incarnation = OsAuthenticationIncarnationV1::allocate();
        self.authentication = AuthenticationRuntime::Required {
            gate: Box::new(OwnedAuthenticationGate {
                gate: AuthenticationGateV1::new(port, max_attempts),
                context,
            }),
            next_session: 0,
            incarnation,
            usage: DaemonAuthenticationUsageV1 {
                session_capacity: 1,
                randomness_unavailable: incarnation.is_none(),
                ..DaemonAuthenticationUsageV1::default()
            },
        };
        self.publish_control_status();
        self
    }

    /// Install one exact operator policy. Export remains unreachable unless a
    /// required authentication handshake succeeds and the durable journal is
    /// present; owner-only and non-persistent compositions stay unchanged.
    #[cfg(target_os = "macos")]
    pub fn with_exact_export_policy(
        mut self,
        policy: DaemonExportPolicyV1,
    ) -> Result<Self, DaemonBuildError> {
        if self.request_journal.is_none()
            || policy.destination.as_os_str().as_bytes().len()
                > self.limits.service.max_locator_bytes
        {
            return Err(DaemonBuildError::InvalidLimits);
        }
        self.export_policy = Some(policy);
        Ok(self)
    }

    pub const fn authentication_profile(&self) -> DaemonAuthenticationProfileV1 {
        match self.authentication {
            AuthenticationRuntime::OwnerOnlyOffline => {
                DaemonAuthenticationProfileV1::OwnerOnlyOffline
            }
            AuthenticationRuntime::Required { .. } => DaemonAuthenticationProfileV1::Required,
        }
    }

    /// Installs one passive, at-most-once event sink and emits `Ready`.
    /// Sink errors and panics are contained and only affect event counters.
    pub fn with_event_sink(mut self, sink: impl OperationalEventSink + 'static) -> Self {
        self.events = Some(OperationalEventPort::new(sink));
        self.emit_event(
            None,
            OperationalEventKind::Lifecycle(OperationalLifecycleEvent::Ready),
        );
        self.publish_control_status();
        self
    }

    pub fn bind_shutdown_control(
        &self,
        control_socket: &Path,
    ) -> Result<ShutdownControl, ShutdownControlError> {
        let parent = control_socket
            .parent()
            .ok_or(ShutdownControlError::DifferentParent)?;
        let normalized: PathBuf = parent.components().collect();
        if normalized != self.ownership.directory_path {
            return Err(ShutdownControlError::DifferentParent);
        }
        self.ownership
            .verify()
            .map_err(|()| ShutdownControlError::OwnershipLost)?;
        self.socket
            .verify()
            .map_err(|_| ShutdownControlError::OwnershipLost)?;
        let stale_control_socket_recovered = self
            .ownership
            .recover_stale_endpoint(control_socket, EndpointKind::Control)
            .map_err(|()| ShutdownControlError::StaleSocketUnproven)?;
        match fs::symlink_metadata(control_socket) {
            Ok(_) => return Err(ShutdownControlError::SocketExists),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return Err(ShutdownControlError::Inspection),
        }
        let listener = UnixListener::bind(control_socket)
            .map_err(|error| ShutdownControlError::Bind(error.kind()))?;
        let metadata =
            fs::symlink_metadata(control_socket).map_err(|_| ShutdownControlError::Inspection)?;
        if !metadata.file_type().is_socket() {
            return Err(ShutdownControlError::Inspection);
        }
        let socket = SocketGuard {
            path: control_socket.to_path_buf(),
            identity: Identity::of(&metadata),
        };
        fs::set_permissions(&socket.path, fs::Permissions::from_mode(0o600))
            .map_err(|_| ShutdownControlError::SocketPermissions)?;
        socket
            .verify()
            .map_err(|_| ShutdownControlError::SocketOwnership)?;
        self.ownership
            .record_endpoint(EndpointKind::Control, &socket)
            .map_err(|()| ShutdownControlError::StaleSocketUnproven)?;
        if stale_control_socket_recovered {
            atomic_bump(
                &self.supervision.stale_control_socket_recoveries,
                &self.supervision.counter_saturated,
            );
        }
        Ok(ShutdownControl {
            listener,
            socket,
            wake_socket: self.socket.path.clone(),
            requested: Arc::clone(&self.shutdown_requested),
            status: Arc::clone(&self.control_status),
            timeout: self.limits.io_timeout,
        })
    }

    /// Installs process signal supervision through the already-bound
    /// owner-only shutdown socket. Signal observation grants no new control
    /// capability: it only asks the existing shutdown path to perform its
    /// ordinary ownership checks, wake, admission close, and drain.
    pub fn install_signal_supervisor(
        &self,
        control_socket: &Path,
    ) -> Result<SignalSupervisor, ShutdownControlError> {
        let parent = control_socket
            .parent()
            .ok_or(ShutdownControlError::DifferentParent)?;
        let normalized: PathBuf = parent.components().collect();
        if normalized != self.ownership.directory_path {
            return Err(ShutdownControlError::DifferentParent);
        }
        self.ownership
            .verify()
            .map_err(|()| ShutdownControlError::OwnershipLost)?;
        let control_metadata =
            fs::symlink_metadata(control_socket).map_err(|_| ShutdownControlError::Inspection)?;
        if !control_metadata.file_type().is_socket()
            || control_metadata.file_type().is_symlink()
            || control_metadata.uid() != rustix::process::geteuid().as_raw()
            || control_metadata.mode() & 0o7777 != 0o600
            || control_metadata.nlink() != 1
        {
            return Err(ShutdownControlError::SocketOwnership);
        }

        let mut signals = signal_hook::iterator::Signals::new([
            signal_hook::consts::signal::SIGINT,
            signal_hook::consts::signal::SIGTERM,
        ])
        .map_err(|_| ShutdownControlError::Io)?;
        let handle = signals.handle();
        let socket = control_socket.to_path_buf();
        let timeout = self.limits.io_timeout;
        let evidence = Arc::clone(&self.supervision);
        let worker = std::thread::spawn(move || {
            for _signal in signals.forever() {
                atomic_bump(&evidence.termination_signals, &evidence.counter_saturated);
                if evidence.selected.load(Ordering::Acquire) {
                    atomic_bump(
                        &evidence.replayed_termination_signals,
                        &evidence.counter_saturated,
                    );
                    continue;
                }
                match request_shutdown(&socket, timeout) {
                    Ok(()) => {
                        evidence.selected.store(true, Ordering::Release);
                        atomic_bump(
                            &evidence.signal_shutdown_requests,
                            &evidence.counter_saturated,
                        );
                    }
                    Err(_) => atomic_bump(
                        &evidence.signal_shutdown_failures,
                        &evidence.counter_saturated,
                    ),
                }
            }
        });
        Ok(SignalSupervisor {
            handle,
            worker: Some(worker),
        })
    }

    pub fn serve_next(&mut self) -> Result<(), DaemonServeError> {
        let result = self.serve_next_inner();
        self.publish_control_status();
        result
    }

    fn serve_next_inner(&mut self) -> Result<(), DaemonServeError> {
        let correlation = self.next_event_correlation();
        if self.shutdown_requested.load(Ordering::Acquire) {
            self.emit_refusal(correlation, OperationalRefusalEvent::Shutdown);
            return Err(DaemonServeError::ShutdownRequested);
        }
        match self.lifecycle.lifecycle() {
            DaemonLifecycle::Running => {}
            DaemonLifecycle::Fenced => {
                self.emit_refusal(correlation, OperationalRefusalEvent::Ownership);
                return Err(DaemonServeError::OwnershipLost);
            }
            DaemonLifecycle::Draining | DaemonLifecycle::Stopped => {
                self.emit_refusal(correlation, OperationalRefusalEvent::NotReady);
                return Err(DaemonServeError::NotReady);
            }
        }
        if self.ownership.verify().is_err() || self.socket.verify().is_err() {
            self.record_fence();
            self.emit_refusal(correlation, OperationalRefusalEvent::Ownership);
            return Err(DaemonServeError::OwnershipLost);
        }
        let (mut stream, _) = match self.listener.accept() {
            Ok(connection) => connection,
            Err(_) => {
                bump(
                    &mut self.usage.accept_failures,
                    &mut self.usage.counter_saturated,
                );
                self.emit_refusal(correlation, OperationalRefusalEvent::Accept);
                return Err(DaemonServeError::Accept);
            }
        };
        if self.shutdown_requested.load(Ordering::Acquire) {
            self.emit_refusal(correlation, OperationalRefusalEvent::Shutdown);
            return Err(DaemonServeError::ShutdownRequested);
        }
        bump(
            &mut self.usage.accepted_connections,
            &mut self.usage.counter_saturated,
        );
        if let Err(error) = self.lifecycle.admit() {
            self.emit_refusal(correlation, refusal_for_serve_error(error));
            return Err(error);
        }
        self.publish_control_status();
        self.emit_event(
            correlation,
            OperationalEventKind::Request(OperationalRequestEvent::Accepted),
        );
        let request_started = Instant::now();
        let (result, deadline_exhausted) = if stream
            .set_read_timeout(Some(self.limits.io_timeout))
            .and_then(|()| stream.set_write_timeout(Some(self.limits.io_timeout)))
            .is_err()
        {
            (Err(DaemonServeError::Request(TransportError::Io)), false)
        } else {
            let mut stream = DeadlineStream::new(&mut stream);
            let authenticated = self.authenticate_connection(&mut stream);
            let mut checkpoint_failed = false;
            let result = match authenticated {
                Ok(session) => {
                    self.publish_control_status();
                    let authenticated_subject = session
                        .as_ref()
                        .map(|session| session.authority_context().subject.to_bytes());
                    let result = if let (
                        Some(metadata),
                        Some(generation),
                        Some(identity_usage),
                        Some(journal),
                        Some(journal_scope),
                    ) = (
                        self.workspace_identity_metadata.as_deref_mut(),
                        self.workspace_identity_generation.as_mut(),
                        self.workspace_identity.as_mut(),
                        self.request_journal.as_deref_mut(),
                        self.request_journal_scope,
                    ) {
                        let mut checkpointing = CheckpointingService {
                            service: &mut self.service,
                            metadata,
                            directory: &self.ownership.directory_path,
                            limits: self.limits.workspace_identity,
                            committed_generation: generation,
                            usage: identity_usage,
                            checkpoint_failed: &mut checkpoint_failed,
                            journal,
                            journal_scope,
                            journal_limits: self.limits.request_journal,
                            durable_receipts: &mut self.durable_receipts,
                            durable_operations: &mut self.durable_operations,
                            retired_durable_operations: &mut self.retired_durable_operations,
                            authenticated_subject,
                            authenticated_session: session,
                            #[cfg(target_os = "macos")]
                            export_policy: self.export_policy.as_ref(),
                            daemon_usage: &mut self.usage,
                        };
                        serve_one(&mut checkpointing, &mut stream, self.limits.transport)
                    } else {
                        serve_one(&mut self.service, &mut stream, self.limits.transport)
                    }
                    .map_err(DaemonServeError::Request);
                    if authenticated_subject.is_some() {
                        self.close_authenticated_session();
                    }
                    result
                }
                Err(error) => Err(error),
            };
            if checkpoint_failed {
                self.record_fence();
            }
            (result, stream.deadline_exhausted)
        };
        let request_elapsed = request_started.elapsed();
        if let Some(request_latency) = &mut self.request_latency {
            let _ = request_latency.observe(request_elapsed);
        }
        if self.lifecycle.complete_connection().is_err() {
            self.record_fence();
            self.emit_refusal(correlation, OperationalRefusalEvent::LifecycleInvariant);
            return Err(DaemonServeError::LifecycleInvariant);
        }
        if let Err(error) = result {
            if deadline_exhausted {
                bump(
                    &mut self.usage.request_deadline_exhausted,
                    &mut self.usage.counter_saturated,
                );
            }
            bump(
                &mut self.usage.request_failures,
                &mut self.usage.counter_saturated,
            );
            self.emit_refusal(correlation, refusal_for_serve_error(error));
            return Err(error);
        }
        bump(
            &mut self.usage.completed_connections,
            &mut self.usage.counter_saturated,
        );
        self.emit_event(
            correlation,
            OperationalEventKind::Request(OperationalRequestEvent::Completed),
        );
        Ok(())
    }

    fn authenticate_connection(
        &mut self,
        stream: &mut DeadlineStream<'_>,
    ) -> Result<Option<AuthenticatedSessionV1>, DaemonServeError> {
        let AuthenticationRuntime::Required {
            gate,
            next_session,
            incarnation,
            usage,
        } = &mut self.authentication
        else {
            return Ok(None);
        };
        bump(&mut usage.handshakes_started, &mut usage.counter_saturated);
        if usage.session_identity_exhausted || incarnation.is_none() {
            bump(&mut usage.handshakes_refused, &mut usage.counter_saturated);
            return Err(DaemonServeError::Authentication);
        }

        let ordinal = *next_session;
        if ordinal == u64::MAX {
            usage.session_identity_exhausted = true;
        } else {
            *next_session += 1;
        }
        let incarnation = incarnation.expect("checked above");
        let challenge_bytes = authentication_challenge(incarnation, ordinal);
        let session_bytes = authentication_session(incarnation, ordinal);
        let mut hello = [0_u8; AUTHENTICATION_HELLO_BYTES_V1];
        hello[0] = AUTHENTICATION_HANDSHAKE_VERSION_V1;
        hello[1..17].copy_from_slice(&challenge_bytes);
        hello[17..33].copy_from_slice(&session_bytes);
        if stream
            .write_all(&hello)
            .and_then(|()| stream.flush())
            .is_err()
        {
            bump(
                &mut usage.handshake_io_failures,
                &mut usage.counter_saturated,
            );
            bump(&mut usage.handshakes_refused, &mut usage.counter_saturated);
            return Err(DaemonServeError::Authentication);
        }

        let mut response = [0_u8; AUTHENTICATION_RESPONSE_BYTES_V1];
        if stream.read_exact(&mut response).is_err() {
            bump(
                &mut usage.handshake_io_failures,
                &mut usage.counter_saturated,
            );
            bump(&mut usage.handshakes_refused, &mut usage.counter_saturated);
            return Err(DaemonServeError::Authentication);
        }
        let exact = response[0] == AUTHENTICATION_HANDSHAKE_VERSION_V1
            && response[1..17] == challenge_bytes
            && response[17..33] == session_bytes;
        let established = if exact {
            let mut claim = [0_u8; 16];
            claim.copy_from_slice(&response[33..49]);
            gate.establish(AuthenticationRequestV1 {
                challenge: AuthenticationChallengeRefV1::new(challenge_bytes),
                claim: AuthenticationClaimRefV1::new(claim),
                session: AuthoritySessionRefV1::new(session_bytes),
            })
            .ok()
        } else {
            None
        };
        let accepted = established.is_some();
        let result = [
            AUTHENTICATION_HANDSHAKE_VERSION_V1,
            if accepted { 1 } else { 0 },
        ];
        if stream
            .write_all(&result)
            .and_then(|()| stream.flush())
            .is_err()
        {
            bump(
                &mut usage.handshake_io_failures,
                &mut usage.counter_saturated,
            );
            bump(&mut usage.handshakes_refused, &mut usage.counter_saturated);
            return Err(DaemonServeError::Authentication);
        }
        if let Some(session) = established {
            bump(
                &mut usage.handshakes_completed,
                &mut usage.counter_saturated,
            );
            bump(&mut usage.sessions_issued, &mut usage.counter_saturated);
            usage.active_sessions = 1;
            Ok(Some(session))
        } else {
            bump(&mut usage.handshakes_refused, &mut usage.counter_saturated);
            Err(DaemonServeError::Authentication)
        }
    }

    fn close_authenticated_session(&mut self) {
        if let AuthenticationRuntime::Required { usage, .. } = &mut self.authentication {
            usage.active_sessions = 0;
            bump(&mut usage.sessions_closed, &mut usage.counter_saturated);
        }
    }

    pub fn usage(&self) -> DaemonUsage {
        let authentication = match &self.authentication {
            AuthenticationRuntime::OwnerOnlyOffline => DaemonAuthenticationUsageV1::default(),
            AuthenticationRuntime::Required { usage, .. } => *usage,
        };
        DaemonUsage {
            termination_signals: self.supervision.termination_signals.load(Ordering::Relaxed),
            signal_shutdown_requests: self
                .supervision
                .signal_shutdown_requests
                .load(Ordering::Relaxed),
            replayed_termination_signals: self
                .supervision
                .replayed_termination_signals
                .load(Ordering::Relaxed),
            signal_shutdown_failures: self
                .supervision
                .signal_shutdown_failures
                .load(Ordering::Relaxed),
            stale_data_socket_recoveries: u64::from(self.stale_data_socket_recovered),
            stale_control_socket_recoveries: self
                .supervision
                .stale_control_socket_recoveries
                .load(Ordering::Relaxed),
            authentication,
            counter_saturated: self.usage.counter_saturated
                || self.supervision.counter_saturated.load(Ordering::Relaxed),
            in_flight_connections: self.lifecycle.in_flight(),
            in_flight_capacity: SERIALIZED_CONNECTION_CAPACITY,
            ..self.usage
        }
    }

    pub fn service_usage(&self) -> ServiceUsage {
        self.service.usage()
    }

    /// Samples all daemon-local operational evidence under one shared borrow.
    ///
    /// `LocalDaemon` is driven serially, so this fixed-point projection cannot
    /// interleave with a mutable admission, request, or lifecycle transition.
    pub fn operational_snapshot(&self) -> DaemonOperationalSnapshot {
        let status = self.lifecycle.status();
        let daemon = self.usage();
        let service = self.service.usage();
        let utilization = OperationalUtilization {
            in_flight_connections: daemon.in_flight_connections,
            in_flight_capacity: daemon.in_flight_capacity,
            active_clients: service.active_clients,
            client_capacity: service.client_capacity,
            issued_clients: service.issued_clients,
            issued_client_capacity: service.issued_client_capacity,
            retired_clients: service.retired_clients,
            retained_requests: service.retained_requests,
            request_capacity: service.request_capacity,
            retained_witnesses: service.retained_witnesses,
            witness_capacity: service.witness_capacity,
            durable_receipts: self
                .durable_receipts
                .len()
                .saturating_add(self.durable_operations.len()),
            durable_receipt_capacity: self.limits.request_journal.max_entries,
            retired_durable_operations: self.retired_durable_operations.len(),
            retired_durable_operation_capacity: self.limits.request_journal.max_retired_operations,
        };
        let saturation = OperationalSaturation {
            in_flight: at_capacity(
                utilization.in_flight_connections,
                utilization.in_flight_capacity,
            ),
            clients: at_capacity(utilization.active_clients, utilization.client_capacity),
            client_lifetime: at_capacity(
                utilization.issued_clients,
                utilization.issued_client_capacity,
            ),
            requests: at_capacity(utilization.retained_requests, utilization.request_capacity),
            witnesses: at_capacity(utilization.retained_witnesses, utilization.witness_capacity),
            durable_receipts: at_capacity(
                utilization.durable_receipts,
                utilization.durable_receipt_capacity,
            ),
            retired_durable_operations: at_capacity(
                utilization.retired_durable_operations,
                utilization.retired_durable_operation_capacity,
            ),
            daemon_counters: daemon.counter_saturated,
            service_counters: service.counter_saturated,
            counters: daemon.counter_saturated || service.counter_saturated,
            request_latency: self
                .request_latency
                .as_ref()
                .is_some_and(|histogram| histogram.snapshot().fidelity_lost),
            workspace_identity: self
                .workspace_identity
                .is_some_and(|usage| usage.counter_saturated),
            events: self.events.as_ref().is_some_and(|events| {
                events.usage.counter_saturated
                    || events.usage.correlation_exhausted
                    || events.usage.sequence_exhausted
            }),
            authentication: daemon.authentication.counter_saturated
                || daemon.authentication.session_identity_exhausted
                || daemon.authentication.randomness_unavailable,
        };
        DaemonOperationalSnapshot {
            status,
            daemon,
            service,
            utilization,
            saturation,
            request_latency: self
                .request_latency
                .as_ref()
                .map(FixedDurationHistogram::snapshot),
            workspace_identity: self.workspace_identity,
            events: self
                .events
                .as_ref()
                .map_or_else(OperationalEventUsage::default, |events| events.usage),
        }
    }

    fn control_status_snapshot(&self) -> DaemonControlStatusV1 {
        let snapshot = self.operational_snapshot();
        DaemonControlStatusV1 {
            liveness: DaemonLiveness::Live,
            lifecycle: snapshot.status.lifecycle,
            health: snapshot.status.health,
            readiness: snapshot.status.readiness,
            utilization: snapshot.utilization,
            saturation: snapshot.saturation,
        }
    }

    fn publish_control_status(&self) {
        if let Ok(mut published) = self.control_status.lock() {
            *published = self.control_status_snapshot();
        }
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket.path
    }

    pub fn probe_operational_status(&mut self) -> DaemonOperationalStatus {
        bump(
            &mut self.usage.operational_probes,
            &mut self.usage.counter_saturated,
        );
        self.verify_or_fence();
        let status = self.lifecycle.status();
        self.publish_control_status();
        status
    }

    /// Starts the in-process drain algebra after revalidating endpoint ownership.
    ///
    /// This method does not itself install an OS signal handler or wake a
    /// blocked `accept`. The optional owner-only [`ShutdownControl`] performs
    /// that wake before the composition root invokes this transition.
    pub fn begin_drain(&mut self) -> DrainTransition {
        let transition = if self.verify_or_fence() {
            self.lifecycle.begin_drain()
        } else {
            DrainTransition::Fenced
        };
        let counter = if transition == DrainTransition::Started {
            &mut self.usage.drain_started
        } else {
            &mut self.usage.drain_replayed
        };
        bump(counter, &mut self.usage.counter_saturated);
        let event = match transition {
            DrainTransition::Started => OperationalLifecycleEvent::DrainStarted,
            DrainTransition::AlreadyDraining | DrainTransition::AlreadyStopped => {
                OperationalLifecycleEvent::DrainReplayed
            }
            DrainTransition::Fenced => OperationalLifecycleEvent::Fenced,
        };
        self.emit_event(None, OperationalEventKind::Lifecycle(event));
        self.publish_control_status();
        transition
    }

    /// Advances an already requested in-process drain after ownership checks.
    pub fn complete_drain(&mut self) -> DrainCompletion {
        let completion = if self.verify_or_fence() {
            self.lifecycle.complete_drain()
        } else {
            DrainCompletion::Fenced
        };
        if completion == DrainCompletion::Complete {
            bump(
                &mut self.usage.drain_completed,
                &mut self.usage.counter_saturated,
            );
        } else if completion == DrainCompletion::AlreadyComplete {
            bump(
                &mut self.usage.drain_replayed,
                &mut self.usage.counter_saturated,
            );
        }
        let event = match completion {
            DrainCompletion::Pending => Some(OperationalLifecycleEvent::DrainPending),
            DrainCompletion::Complete => Some(OperationalLifecycleEvent::DrainCompleted),
            DrainCompletion::AlreadyComplete => Some(OperationalLifecycleEvent::DrainReplayed),
            DrainCompletion::Fenced => Some(OperationalLifecycleEvent::Fenced),
            DrainCompletion::NotDraining => None,
        };
        if let Some(event) = event {
            self.emit_event(None, OperationalEventKind::Lifecycle(event));
        }
        self.publish_control_status();
        completion
    }

    fn record_fence(&mut self) {
        if self.lifecycle.fence() {
            bump(
                &mut self.usage.health_failures,
                &mut self.usage.counter_saturated,
            );
            self.emit_event(
                None,
                OperationalEventKind::Lifecycle(OperationalLifecycleEvent::Fenced),
            );
        }
    }

    fn verify_or_fence(&mut self) -> bool {
        if self.ownership.verify().is_ok() && self.socket.verify().is_ok() {
            true
        } else {
            self.record_fence();
            false
        }
    }

    fn next_event_correlation(&mut self) -> Option<OperationalCorrelationId> {
        self.events
            .as_mut()
            .and_then(OperationalEventPort::next_correlation)
    }

    fn emit_refusal(
        &mut self,
        correlation: Option<OperationalCorrelationId>,
        refusal: OperationalRefusalEvent,
    ) {
        self.emit_event(correlation, OperationalEventKind::Refusal(refusal));
    }

    fn emit_event(
        &mut self,
        correlation: Option<OperationalCorrelationId>,
        kind: OperationalEventKind,
    ) {
        if let Some(events) = &mut self.events {
            events.emit(correlation, kind);
        }
    }
}

#[cfg(unix)]
struct OsAuthenticationIncarnationV1;

#[cfg(unix)]
impl OsAuthenticationIncarnationV1 {
    /// Opens the exact Unix CSPRNG device without following a final symlink,
    /// verifies the opened descriptor is a character device, and accepts only
    /// one complete incarnation read. Every failure is `None`; bytes and raw
    /// OS errors never cross this boundary or enter operational evidence.
    fn allocate() -> Option<[u8; 16]> {
        let no_follow = i32::try_from(rustix::fs::OFlags::NOFOLLOW.bits()).ok()?;
        let mut random = OpenOptions::new()
            .read(true)
            .custom_flags(no_follow)
            .open("/dev/urandom")
            .ok()?;
        if !random.metadata().ok()?.file_type().is_char_device() {
            return None;
        }
        let mut value = [0_u8; 16];
        random.read_exact(&mut value).ok().map(|_| value)
    }
}

#[cfg(unix)]
fn authentication_challenge(incarnation: [u8; 16], ordinal: u64) -> [u8; 16] {
    let mut value = incarnation;
    let ordinal = ordinal.to_be_bytes();
    for (index, byte) in value.iter_mut().enumerate() {
        *byte ^= ordinal[index % ordinal.len()] ^ 0x43;
    }
    value
}

#[cfg(unix)]
fn authentication_session(incarnation: [u8; 16], ordinal: u64) -> [u8; 16] {
    let mut value = incarnation;
    let ordinal = ordinal.to_be_bytes();
    for (index, byte) in value.iter_mut().enumerate() {
        *byte ^= ordinal[index % ordinal.len()] ^ 0x53;
    }
    value
}

#[cfg(unix)]
impl Drop for SignalSupervisor {
    fn drop(&mut self) {
        self.handle.close();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(unix)]
fn atomic_bump(counter: &AtomicU64, saturated: &AtomicBool) {
    let _ = counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
        if value == u64::MAX {
            saturated.store(true, Ordering::Relaxed);
            None
        } else {
            Some(value + 1)
        }
    });
}

#[cfg(unix)]
const fn refusal_for_serve_error(error: DaemonServeError) -> OperationalRefusalEvent {
    match error {
        DaemonServeError::OwnershipLost => OperationalRefusalEvent::Ownership,
        DaemonServeError::ShutdownRequested => OperationalRefusalEvent::Shutdown,
        DaemonServeError::NotReady => OperationalRefusalEvent::NotReady,
        DaemonServeError::Capacity => OperationalRefusalEvent::Capacity,
        DaemonServeError::LifecycleInvariant => OperationalRefusalEvent::LifecycleInvariant,
        DaemonServeError::Accept => OperationalRefusalEvent::Accept,
        DaemonServeError::Authentication => OperationalRefusalEvent::Authentication,
        DaemonServeError::Request(_) => OperationalRefusalEvent::Transport,
    }
}

#[cfg(unix)]
fn encode_control_status_v1(status: DaemonControlStatusV1) -> [u8; CONTROL_STATUS_REPLY_BYTES_V1] {
    let mut encoded = [0_u8; CONTROL_STATUS_REPLY_BYTES_V1];
    encoded[..4].copy_from_slice(&CONTROL_STATUS_MAGIC_V1);
    encoded[4] = CONTROL_PROTOCOL_VERSION_V1;
    encoded[5] = match status.liveness {
        DaemonLiveness::Live => 1,
    };
    encoded[6] = match status.lifecycle {
        DaemonLifecycle::Running => 1,
        DaemonLifecycle::Draining => 2,
        DaemonLifecycle::Stopped => 3,
        DaemonLifecycle::Fenced => 4,
    };
    encoded[7] = match status.health {
        DaemonHealth::Healthy => 1,
        DaemonHealth::Fenced => 2,
    };
    encoded[8] = match status.readiness {
        DaemonReadiness::Ready => 1,
        DaemonReadiness::NotReady => 2,
    };
    let utilization = [
        status.utilization.in_flight_connections,
        status.utilization.in_flight_capacity,
        status.utilization.active_clients,
        status.utilization.client_capacity,
        status.utilization.issued_clients,
        status.utilization.issued_client_capacity,
        status.utilization.retired_clients,
        status.utilization.retained_requests,
        status.utilization.request_capacity,
        status.utilization.retained_witnesses,
        status.utilization.witness_capacity,
        status.utilization.durable_receipts,
        status.utilization.durable_receipt_capacity,
    ];
    let mut offset = 9;
    for value in utilization {
        let value = u64::try_from(value).unwrap_or(u64::MAX);
        encoded[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
        offset += 8;
    }
    let flags = [
        status.saturation.in_flight,
        status.saturation.clients,
        status.saturation.client_lifetime,
        status.saturation.requests,
        status.saturation.witnesses,
        status.saturation.durable_receipts,
        status.saturation.daemon_counters,
        status.saturation.service_counters,
        status.saturation.counters,
        status.saturation.request_latency,
        status.saturation.workspace_identity,
        status.saturation.events,
        status.saturation.authentication,
    ];
    for (index, saturated) in flags.into_iter().enumerate() {
        if saturated {
            encoded[offset + index / 8] |= 1 << (index % 8);
        }
    }
    encoded
}

#[cfg(unix)]
fn decode_control_status_v1(
    encoded: &[u8; CONTROL_STATUS_REPLY_BYTES_V1],
) -> Result<DaemonControlStatusV1, ShutdownControlError> {
    if encoded[..4] != CONTROL_STATUS_MAGIC_V1
        || encoded[4] != CONTROL_PROTOCOL_VERSION_V1
        || encoded[5] != 1
    {
        return Err(ShutdownControlError::InvalidRequest);
    }
    let lifecycle = match encoded[6] {
        1 => DaemonLifecycle::Running,
        2 => DaemonLifecycle::Draining,
        3 => DaemonLifecycle::Stopped,
        4 => DaemonLifecycle::Fenced,
        _ => return Err(ShutdownControlError::InvalidRequest),
    };
    let health = match encoded[7] {
        1 => DaemonHealth::Healthy,
        2 => DaemonHealth::Fenced,
        _ => return Err(ShutdownControlError::InvalidRequest),
    };
    let readiness = match encoded[8] {
        1 => DaemonReadiness::Ready,
        2 => DaemonReadiness::NotReady,
        _ => return Err(ShutdownControlError::InvalidRequest),
    };
    let mut values = [0_usize; CONTROL_STATUS_UTILIZATION_FIELDS_V1];
    let mut offset = 9;
    for value in &mut values {
        let raw = u64::from_be_bytes(
            encoded[offset..offset + 8]
                .try_into()
                .map_err(|_| ShutdownControlError::InvalidRequest)?,
        );
        *value = usize::try_from(raw).map_err(|_| ShutdownControlError::InvalidRequest)?;
        offset += 8;
    }
    if encoded[offset + 1] & 0b1110_0000 != 0 {
        return Err(ShutdownControlError::InvalidRequest);
    }
    let flag = |index: usize| encoded[offset + index / 8] & (1 << (index % 8)) != 0;
    Ok(DaemonControlStatusV1 {
        liveness: DaemonLiveness::Live,
        lifecycle,
        health,
        readiness,
        utilization: OperationalUtilization {
            in_flight_connections: values[0],
            in_flight_capacity: values[1],
            active_clients: values[2],
            client_capacity: values[3],
            issued_clients: values[4],
            issued_client_capacity: values[5],
            retired_clients: values[6],
            retained_requests: values[7],
            request_capacity: values[8],
            retained_witnesses: values[9],
            witness_capacity: values[10],
            durable_receipts: values[11],
            durable_receipt_capacity: values[12],
            retired_durable_operations: 0,
            retired_durable_operation_capacity: 0,
        },
        saturation: OperationalSaturation {
            in_flight: flag(0),
            clients: flag(1),
            client_lifetime: flag(2),
            requests: flag(3),
            witnesses: flag(4),
            durable_receipts: flag(5),
            retired_durable_operations: false,
            daemon_counters: flag(6),
            service_counters: flag(7),
            counters: flag(8),
            request_latency: flag(9),
            workspace_identity: flag(10),
            events: flag(11),
            authentication: flag(12),
        },
    })
}

#[cfg(unix)]
fn connect_control_endpoint(
    control_socket: &Path,
    timeout: Duration,
) -> Result<UnixStream, ShutdownControlError> {
    if timeout.is_zero() {
        return Err(ShutdownControlError::Io);
    }
    let parent = control_socket
        .parent()
        .ok_or(ShutdownControlError::Inspection)?;
    let parent_metadata =
        fs::symlink_metadata(parent).map_err(|_| ShutdownControlError::Inspection)?;
    if !parent_metadata.file_type().is_dir()
        || parent_metadata.file_type().is_symlink()
        || parent_metadata.uid() != rustix::process::geteuid().as_raw()
        || parent_metadata.mode() & 0o7777 != 0o700
    {
        return Err(ShutdownControlError::SocketOwnership);
    }
    let inspect = || {
        let metadata =
            fs::symlink_metadata(control_socket).map_err(|_| ShutdownControlError::Inspection)?;
        if !metadata.file_type().is_socket()
            || metadata.file_type().is_symlink()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.mode() & 0o7777 != 0o600
            || metadata.nlink() != 1
        {
            return Err(ShutdownControlError::SocketOwnership);
        }
        Ok(Identity::of(&metadata))
    };
    let expected = inspect()?;
    let stream = UnixStream::connect(control_socket).map_err(|_| ShutdownControlError::Io)?;
    if inspect()? != expected {
        return Err(ShutdownControlError::SocketOwnership);
    }
    stream
        .set_read_timeout(Some(timeout))
        .and_then(|()| stream.set_write_timeout(Some(timeout)))
        .map_err(|_| ShutdownControlError::Io)?;
    Ok(stream)
}

#[cfg(unix)]
impl ShutdownControl {
    /// Waits for one valid fixed-size shutdown request and wakes data acceptance.
    pub fn wait(self) -> Result<(), ShutdownControlError> {
        loop {
            self.socket
                .verify()
                .map_err(|_| ShutdownControlError::SocketOwnership)?;
            let (mut stream, _) = self
                .listener
                .accept()
                .map_err(|_| ShutdownControlError::Accept)?;
            if stream
                .set_read_timeout(Some(self.timeout))
                .and_then(|()| stream.set_write_timeout(Some(self.timeout)))
                .is_err()
            {
                continue;
            }
            let mut operation = [0_u8; 1];
            if stream.read_exact(&mut operation).is_err() {
                continue;
            }
            match operation[0] {
                CONTROL_SHUTDOWN_REQUEST_V1 => {
                    self.requested.store(true, Ordering::Release);
                    UnixStream::connect(&self.wake_socket)
                        .map_err(|_| ShutdownControlError::Wake)?;
                    let _ = stream.write_all(&[1]);
                    return Ok(());
                }
                CONTROL_STATUS_REQUEST_V1 => {
                    let mut version = [0_u8; 1];
                    if stream.read_exact(&mut version).is_err()
                        || version != [CONTROL_PROTOCOL_VERSION_V1]
                    {
                        let _ = stream.write_all(&[0]);
                        continue;
                    }
                    let mut trailing = [0_u8; 1];
                    if !matches!(stream.read(&mut trailing), Ok(0)) {
                        let _ = stream.write_all(&[0]);
                        continue;
                    }
                    let Ok(status) = self.status.lock().map(|status| *status) else {
                        let _ = stream.write_all(&[0]);
                        continue;
                    };
                    let _ = stream.write_all(&encode_control_status_v1(status));
                }
                _ => {
                    let _ = stream.write_all(&[0]);
                }
            }
        }
    }
}

#[cfg(unix)]
pub fn request_shutdown(
    control_socket: &Path,
    timeout: Duration,
) -> Result<(), ShutdownControlError> {
    let mut stream = connect_control_endpoint(control_socket, timeout)?;
    stream
        .write_all(&[CONTROL_SHUTDOWN_REQUEST_V1])
        .map_err(|_| ShutdownControlError::Io)?;
    let mut response = [0_u8; 1];
    stream
        .read_exact(&mut response)
        .map_err(|_| ShutdownControlError::Io)?;
    if response == [1] {
        Ok(())
    } else {
        Err(ShutdownControlError::InvalidRequest)
    }
}

/// Reads one versioned, fixed-size, content-free status reply from the private
/// control socket. This is a passive sample and never probes or revalidates the
/// daemon's readiness on behalf of the caller.
#[cfg(unix)]
pub fn request_status(
    control_socket: &Path,
    timeout: Duration,
) -> Result<DaemonControlStatusV1, ShutdownControlError> {
    let mut stream = connect_control_endpoint(control_socket, timeout)?;
    stream
        .write_all(&[CONTROL_STATUS_REQUEST_V1, CONTROL_PROTOCOL_VERSION_V1])
        .map_err(|_| ShutdownControlError::Io)?;
    stream
        .shutdown(Shutdown::Write)
        .map_err(|_| ShutdownControlError::Io)?;
    let mut response = [0_u8; CONTROL_STATUS_REPLY_BYTES_V1];
    stream
        .read_exact(&mut response)
        .map_err(|_| ShutdownControlError::Io)?;
    decode_control_status_v1(&response)
}

#[cfg(unix)]
struct SocketGuard {
    path: PathBuf,
    identity: Identity,
}

#[cfg(unix)]
impl SocketGuard {
    fn verify(&self) -> Result<(), DaemonBuildError> {
        let metadata =
            fs::symlink_metadata(&self.path).map_err(|_| DaemonBuildError::SocketOwnership)?;
        if !metadata.file_type().is_socket()
            || metadata.file_type().is_symlink()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.mode() & 0o7777 != 0o600
            || metadata.nlink() != 1
            || Identity::of(&metadata) != self.identity
        {
            return Err(DaemonBuildError::SocketOwnership);
        }
        Ok(())
    }
}

#[cfg(unix)]
impl Drop for SocketGuard {
    fn drop(&mut self) {
        let Ok(metadata) = fs::symlink_metadata(&self.path) else {
            return;
        };
        if metadata.file_type().is_socket() && Identity::of(&metadata) == self.identity {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(unix)]
const LOCK_NAME: &str = ".vellum-daemon.lock";

#[cfg(unix)]
const ENDPOINT_RECORD_MAGIC: &[u8; 8] = b"VELLEND1";

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EndpointKind {
    Data = 1,
    Control = 2,
}

#[cfg(unix)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct EndpointEvidence {
    kind: EndpointKind,
    identity: Identity,
    path: PathBuf,
}

#[cfg(unix)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct EndpointRecords {
    data: Option<EndpointEvidence>,
    control: Option<EndpointEvidence>,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Identity {
    device: u64,
    inode: u64,
}

#[cfg(unix)]
impl Identity {
    fn of(metadata: &fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
        }
    }
}

/// Owns one private endpoint directory for the daemon lifetime. The advisory
/// lock is released by the OS when this file descriptor closes; its inode is
/// deliberately persistent so two processes can never lock different files
/// after an ordinary shutdown.
#[cfg(unix)]
struct OwnershipGuard {
    directory_path: PathBuf,
    lock_path: PathBuf,
    directory: File,
    lock: File,
    directory_identity: Identity,
    lock_identity: Identity,
    lock_created: bool,
    endpoint_records: Mutex<EndpointRecords>,
}

#[cfg(unix)]
impl OwnershipGuard {
    fn acquire(directory_path: &Path) -> Result<Self, DaemonBuildError> {
        // Remove trailing separators and dot components before `O_NOFOLLOW` so
        // a symlink leaf cannot be disguised as `alias/` or `alias/.`.
        let normalized: PathBuf = directory_path.components().collect();
        let directory_path = normalized.as_path();
        let directory_flags =
            i32::try_from((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::DIRECTORY).bits())
                .map_err(|_| DaemonBuildError::Inspection)?;
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(directory_flags)
            .open(directory_path)
            .map_err(|error| match error.kind() {
                io::ErrorKind::NotFound => DaemonBuildError::ParentMissing,
                _ => DaemonBuildError::ParentNotDirectory,
            })?;
        let directory_metadata = directory
            .metadata()
            .map_err(|_| DaemonBuildError::Inspection)?;
        validate_directory(&directory_metadata)?;

        let lock_path = directory_path.join(LOCK_NAME);
        let lock_flags = i32::try_from(rustix::fs::OFlags::NOFOLLOW.bits())
            .map_err(|_| DaemonBuildError::Inspection)?;
        let (lock, lock_created) = match OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .mode(0o600)
            .custom_flags(lock_flags)
            .open(&lock_path)
        {
            Ok(lock) => (lock, true),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .custom_flags(lock_flags)
                    .open(&lock_path)
                    .map_err(|_| DaemonBuildError::LockUnsafe)?,
                false,
            ),
            Err(_) => return Err(DaemonBuildError::LockUnsafe),
        };
        let lock_metadata = lock.metadata().map_err(|_| DaemonBuildError::LockUnsafe)?;
        validate_lock(&lock_metadata)?;
        lock.try_lock().map_err(|_| DaemonBuildError::LockBusy)?;

        let endpoint_records =
            read_endpoint_records(&lock).map_err(|()| DaemonBuildError::LockUnsafe)?;
        let guard = Self {
            directory_path: directory_path.to_path_buf(),
            lock_path,
            directory_identity: Identity::of(&directory_metadata),
            lock_identity: Identity::of(&lock_metadata),
            lock_created,
            endpoint_records: Mutex::new(endpoint_records),
            directory,
            lock,
        };
        guard.verify().map_err(|_| DaemonBuildError::LockLost)?;
        Ok(guard)
    }

    fn verify(&self) -> Result<(), ()> {
        let directory_metadata = self.directory.metadata().map_err(|_| ())?;
        validate_directory(&directory_metadata).map_err(|_| ())?;
        if Identity::of(&directory_metadata) != self.directory_identity {
            return Err(());
        }
        let directory_path_metadata = fs::symlink_metadata(&self.directory_path).map_err(|_| ())?;
        validate_directory(&directory_path_metadata).map_err(|_| ())?;
        if Identity::of(&directory_path_metadata) != self.directory_identity {
            return Err(());
        }

        let lock_metadata = self.lock.metadata().map_err(|_| ())?;
        validate_lock(&lock_metadata).map_err(|_| ())?;
        if Identity::of(&lock_metadata) != self.lock_identity {
            return Err(());
        }
        let lock_path_metadata = fs::symlink_metadata(&self.lock_path).map_err(|_| ())?;
        validate_lock(&lock_path_metadata).map_err(|_| ())?;
        if Identity::of(&lock_path_metadata) != self.lock_identity {
            return Err(());
        }
        Ok(())
    }

    fn recover_stale_endpoint(&self, path: &Path, kind: EndpointKind) -> Result<bool, ()> {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(_) => return Err(()),
        };
        let records = self.endpoint_records.lock().map_err(|_| ())?;
        let Some(evidence) = (match kind {
            EndpointKind::Data => records.data.as_ref(),
            EndpointKind::Control => records.control.as_ref(),
        }) else {
            return Ok(false);
        };
        if evidence.path != path
            || !metadata.file_type().is_socket()
            || metadata.file_type().is_symlink()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.mode() & 0o7777 != 0o600
            || metadata.nlink() != 1
            || Identity::of(&metadata) != evidence.identity
        {
            return Err(());
        }
        match UnixStream::connect(path) {
            Ok(stream) => {
                drop(stream);
                return Err(());
            }
            Err(error) if error.kind() == io::ErrorKind::ConnectionRefused => {}
            Err(_) => return Err(()),
        }
        self.verify()?;
        let current = fs::symlink_metadata(path).map_err(|_| ())?;
        if !current.file_type().is_socket()
            || current.uid() != rustix::process::geteuid().as_raw()
            || current.mode() & 0o7777 != 0o600
            || current.nlink() != 1
            || Identity::of(&current) != evidence.identity
        {
            return Err(());
        }
        fs::remove_file(path).map_err(|_| ())?;
        self.directory.sync_all().map_err(|_| ())?;
        Ok(true)
    }

    fn record_endpoint(&self, kind: EndpointKind, socket: &SocketGuard) -> Result<(), ()> {
        socket.verify().map_err(|_| ())?;
        self.verify()?;
        let mut records = self.endpoint_records.lock().map_err(|_| ())?;
        let evidence = EndpointEvidence {
            kind,
            identity: socket.identity,
            path: socket.path.clone(),
        };
        match kind {
            EndpointKind::Data => records.data = Some(evidence),
            EndpointKind::Control => records.control = Some(evidence),
        }
        let encoded = encode_endpoint_records(&records)?;
        self.lock.set_len(0).map_err(|_| ())?;
        write_all_at(&self.lock, &encoded, 0)?;
        self.lock.sync_all().map_err(|_| ())?;
        self.verify()?;
        Ok(())
    }
}

#[cfg(unix)]
fn read_endpoint_records(file: &File) -> Result<EndpointRecords, ()> {
    let length = usize::try_from(file.metadata().map_err(|_| ())?.len()).map_err(|_| ())?;
    if length == 0 {
        return Ok(EndpointRecords::default());
    }
    if length > 512 {
        return Err(());
    }
    let mut bytes = vec![0_u8; length];
    let mut offset = 0;
    while offset < length {
        let read = file
            .read_at(&mut bytes[offset..], u64::try_from(offset).map_err(|_| ())?)
            .map_err(|_| ())?;
        if read == 0 {
            return Err(());
        }
        offset += read;
    }
    decode_endpoint_records(&bytes)
}

#[cfg(unix)]
fn encode_endpoint_records(records: &EndpointRecords) -> Result<Vec<u8>, ()> {
    let mut encoded = Vec::with_capacity(128);
    encoded.extend_from_slice(ENDPOINT_RECORD_MAGIC);
    encoded.push(u8::from(records.data.is_some()) + u8::from(records.control.is_some()));
    for evidence in [records.data.as_ref(), records.control.as_ref()]
        .into_iter()
        .flatten()
    {
        let path = evidence.path.as_os_str().as_bytes();
        let length = u16::try_from(path.len()).map_err(|_| ())?;
        encoded.push(evidence.kind as u8);
        encoded.extend_from_slice(&evidence.identity.device.to_le_bytes());
        encoded.extend_from_slice(&evidence.identity.inode.to_le_bytes());
        encoded.extend_from_slice(&length.to_le_bytes());
        encoded.extend_from_slice(path);
    }
    if encoded.len() > 512 {
        return Err(());
    }
    Ok(encoded)
}

#[cfg(unix)]
fn decode_endpoint_records(bytes: &[u8]) -> Result<EndpointRecords, ()> {
    if bytes.len() <= ENDPOINT_RECORD_MAGIC.len()
        || &bytes[..ENDPOINT_RECORD_MAGIC.len()] != ENDPOINT_RECORD_MAGIC
    {
        return Err(());
    }
    let expected = usize::from(bytes[ENDPOINT_RECORD_MAGIC.len()]);
    if expected > 2 {
        return Err(());
    }
    let mut records = EndpointRecords::default();
    let mut decoded = 0;
    let mut offset = ENDPOINT_RECORD_MAGIC.len() + 1;
    while offset < bytes.len() {
        if bytes.len() - offset < 19 {
            return Err(());
        }
        let kind = match bytes[offset] {
            1 => EndpointKind::Data,
            2 => EndpointKind::Control,
            _ => return Err(()),
        };
        offset += 1;
        let device = u64::from_le_bytes(bytes[offset..offset + 8].try_into().map_err(|_| ())?);
        offset += 8;
        let inode = u64::from_le_bytes(bytes[offset..offset + 8].try_into().map_err(|_| ())?);
        offset += 8;
        let length = usize::from(u16::from_le_bytes(
            bytes[offset..offset + 2].try_into().map_err(|_| ())?,
        ));
        offset += 2;
        let end = offset
            .checked_add(length)
            .filter(|end| *end <= bytes.len())
            .ok_or(())?;
        let evidence = EndpointEvidence {
            kind,
            identity: Identity { device, inode },
            path: PathBuf::from(std::ffi::OsString::from_vec(bytes[offset..end].to_vec())),
        };
        let slot = match kind {
            EndpointKind::Data => &mut records.data,
            EndpointKind::Control => &mut records.control,
        };
        if slot.replace(evidence).is_some() {
            return Err(());
        }
        decoded += 1;
        offset = end;
    }
    if decoded == expected {
        Ok(records)
    } else {
        Err(())
    }
}

#[cfg(unix)]
fn write_all_at(file: &File, mut bytes: &[u8], mut offset: u64) -> Result<(), ()> {
    while !bytes.is_empty() {
        let written = file.write_at(bytes, offset).map_err(|_| ())?;
        if written == 0 {
            return Err(());
        }
        bytes = &bytes[written..];
        offset = offset
            .checked_add(u64::try_from(written).map_err(|_| ())?)
            .ok_or(())?;
    }
    Ok(())
}

#[cfg(unix)]
fn validate_directory(metadata: &fs::Metadata) -> Result<(), DaemonBuildError> {
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(DaemonBuildError::ParentNotDirectory);
    }
    if metadata.uid() != rustix::process::geteuid().as_raw() {
        return Err(DaemonBuildError::ParentOwnership);
    }
    if metadata.mode() & 0o7777 != 0o700 {
        return Err(DaemonBuildError::ParentPermissions);
    }
    Ok(())
}

#[cfg(unix)]
fn validate_lock(metadata: &fs::Metadata) -> Result<(), DaemonBuildError> {
    if !metadata.file_type().is_file()
        || metadata.file_type().is_symlink()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o7777 != 0o600
        || metadata.nlink() != 1
    {
        return Err(DaemonBuildError::LockUnsafe);
    }
    Ok(())
}

#[cfg(unix)]
fn validate_limits(limits: DaemonLimits) -> Result<(), DaemonBuildError> {
    let required_authority_decisions = limits
        .service
        .max_requests
        .checked_mul(2)
        .and_then(|ordinary| {
            limits
                .request_journal
                .max_entries
                .checked_add(limits.request_journal.max_retired_operations)
                .and_then(|identities| identities.checked_mul(3))
                .and_then(|durable| ordinary.checked_add(durable))
        })
        .and_then(|value| u64::try_from(value).ok());
    if limits.io_timeout.is_zero()
        || limits.max_in_flight_connections != SERIALIZED_CONNECTION_CAPACITY
        || limits.max_startup_roots == 0
        || limits.max_authentication_attempts == 0
        || limits.transport.max_frame_bytes == 0
        || limits.transport.wire.max_frame_bytes == 0
        || limits.transport.wire.max_frame_bytes > limits.transport.max_frame_bytes
        || limits.workspace_identity.max_documents != limits.service.max_inventory_documents
        || limits.workspace_identity.max_roots != limits.service.max_inventory_roots
        || limits.workspace_identity.max_encoded_bytes < 72
        || limits.request_journal.max_entries == 0
        || limits.request_journal.max_retired_operations == 0
        || required_authority_decisions
            .is_none_or(|required| limits.service.max_authority_decisions < required)
        || limits.request_journal.max_entries > limits.service.max_requests
        || limits.request_journal.max_request_bytes < limits.transport.wire.max_frame_bytes
        || limits.request_journal.max_reply_bytes < limits.transport.wire.max_frame_bytes
        || limits.request_journal.max_encoded_bytes
            < limits
                .request_journal
                .max_request_bytes
                .saturating_add(limits.request_journal.max_reply_bytes)
                .saturating_add(64)
    {
        return Err(DaemonBuildError::InvalidLimits);
    }
    Ok(())
}

#[cfg(unix)]
fn bump(value: &mut u64, saturated: &mut bool) {
    let next = value.saturating_add(1);
    if next == *value {
        *saturated = true;
    }
    *value = next;
}

#[cfg(unix)]
const fn at_capacity(used: usize, capacity: usize) -> bool {
    used >= capacity
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::num::NonZeroU64;

    struct FailingSink;

    impl OperationalEventSink for FailingSink {
        fn emit(&mut self, _: OperationalEvent) -> Result<(), OperationalEventSinkError> {
            Err(OperationalEventSinkError)
        }
    }

    struct PanickingSink;

    #[test]
    fn endpoint_record_round_trip_preserves_both_roles_and_refuses_truncation() {
        let records = EndpointRecords {
            data: Some(EndpointEvidence {
                kind: EndpointKind::Data,
                identity: Identity {
                    device: 11,
                    inode: 13,
                },
                path: PathBuf::from("/private/data.sock"),
            }),
            control: Some(EndpointEvidence {
                kind: EndpointKind::Control,
                identity: Identity {
                    device: 17,
                    inode: 19,
                },
                path: PathBuf::from("/private/control.sock"),
            }),
        };
        let encoded = encode_endpoint_records(&records).unwrap();
        assert_eq!(decode_endpoint_records(&encoded), Ok(records));
        for end in 0..encoded.len() {
            assert_eq!(decode_endpoint_records(&encoded[..end]), Err(()));
        }
        let mut trailing = encoded;
        trailing.push(0);
        assert_eq!(decode_endpoint_records(&trailing), Err(()));
    }

    impl OperationalEventSink for PanickingSink {
        fn emit(&mut self, _: OperationalEvent) -> Result<(), OperationalEventSinkError> {
            panic!("sink panic")
        }
    }

    #[test]
    fn event_sink_failure_and_panic_are_contained_as_passive_evidence() {
        let mut failing = OperationalEventPort::new(FailingSink);
        let correlation = failing.next_correlation();
        failing.emit(
            correlation,
            OperationalEventKind::Request(OperationalRequestEvent::Completed),
        );
        assert_eq!(failing.usage.attempted, 1);
        assert_eq!(failing.usage.failed, 1);
        assert_eq!(failing.usage.delivered, 0);

        let mut panicking = OperationalEventPort::new(PanickingSink);
        panicking.emit(
            None,
            OperationalEventKind::Lifecycle(OperationalLifecycleEvent::Ready),
        );
        assert_eq!(panicking.usage.attempted, 1);
        assert_eq!(panicking.usage.panicked, 1);

        // Event disposition is not an input to the lifecycle algebra.
        let mut lifecycle = LifecycleState::new();
        lifecycle.admit().unwrap();
        lifecycle.complete_connection().unwrap();
        assert_eq!(lifecycle.status().lifecycle, DaemonLifecycle::Running);
    }

    #[test]
    fn admitted_connection_is_an_obligation_that_delays_drain_completion() {
        let mut lifecycle = LifecycleState::new();
        lifecycle.admit().unwrap();
        assert_eq!(lifecycle.begin_drain(), DrainTransition::Started);
        assert_eq!(lifecycle.admit(), Err(DaemonServeError::NotReady));
        assert_eq!(lifecycle.status().in_flight, 1);
        assert_eq!(lifecycle.complete_drain(), DrainCompletion::Pending);
        assert_eq!(lifecycle.status().readiness, DaemonReadiness::NotReady);

        lifecycle.complete_connection().unwrap();

        assert_eq!(lifecycle.complete_drain(), DrainCompletion::Complete);
        assert_eq!(lifecycle.status().lifecycle, DaemonLifecycle::Stopped);
        assert_eq!(lifecycle.complete_drain(), DrainCompletion::AlreadyComplete);
        assert_eq!(lifecycle.admit(), Err(DaemonServeError::NotReady));
    }

    #[test]
    fn connection_obligation_underflow_is_an_explicit_invariant_failure() {
        let mut lifecycle = LifecycleState::new();
        assert_eq!(
            lifecycle.complete_connection(),
            Err(DaemonServeError::LifecycleInvariant)
        );
        assert_eq!(lifecycle.status().in_flight, 0);
    }

    #[test]
    fn keystone_phase_projection_preserves_vellum_status_and_capacity_model() {
        let mut lifecycle = LifecycleState::new();
        let open = lifecycle.status();
        assert_eq!(open.lifecycle, DaemonLifecycle::Running);
        assert_eq!(open.readiness, DaemonReadiness::Ready);
        assert_eq!(open.in_flight, 0);
        assert_eq!(open.in_flight_capacity, 1);

        lifecycle.admit().unwrap();
        assert_eq!(lifecycle.status().in_flight, 1);
        assert_eq!(lifecycle.admit(), Err(DaemonServeError::Capacity));
        assert_eq!(lifecycle.begin_drain(), DrainTransition::Started);
        assert_eq!(lifecycle.begin_drain(), DrainTransition::AlreadyDraining);
        assert_eq!(lifecycle.admit(), Err(DaemonServeError::NotReady));
        assert_eq!(lifecycle.complete_drain(), DrainCompletion::Pending);
        assert_eq!(lifecycle.status().in_flight, 1);

        lifecycle.complete_connection().unwrap();
        assert_eq!(lifecycle.complete_drain(), DrainCompletion::Complete);
        let closed = lifecycle.status();
        assert_eq!(closed.lifecycle, DaemonLifecycle::Stopped);
        assert_eq!(closed.readiness, DaemonReadiness::NotReady);
        assert_eq!(closed.in_flight, 0);
        assert_eq!(closed.in_flight_capacity, 1);
    }

    #[test]
    fn fencing_is_orthogonal_to_keystone_obligation_disposition() {
        let mut lifecycle = LifecycleState::new();
        lifecycle.admit().unwrap();
        assert!(lifecycle.fence());
        let fenced_with_obligation = lifecycle.status();
        assert_eq!(fenced_with_obligation.lifecycle, DaemonLifecycle::Fenced);
        assert_eq!(fenced_with_obligation.health, DaemonHealth::Fenced);
        assert_eq!(fenced_with_obligation.readiness, DaemonReadiness::NotReady);
        assert_eq!(fenced_with_obligation.in_flight, 1);
        assert_eq!(lifecycle.complete_drain(), DrainCompletion::Fenced);

        lifecycle.complete_connection().unwrap();
        assert_eq!(lifecycle.status().in_flight, 0);
        assert_eq!(lifecycle.complete_drain(), DrainCompletion::Fenced);
        assert!(!lifecycle.fence());
    }

    #[test]
    fn durable_receipt_subject_mismatch_is_indistinguishable_from_absence() {
        let request = RequestV1 {
            client: vellum_protocol::ClientId::new(3).unwrap(),
            request: vellum_protocol::RequestId::new(5).unwrap(),
            command: CommandV1::Save {
                expected: vellum_protocol::VersionWitness::new(7).unwrap(),
            },
        };
        let frozen_request = encode_request_v1(
            &WireRequestV1::Execute(request.clone()),
            vellum_wire::WireLimits::reference(),
        )
        .unwrap();
        let expected = ReplyV1::Refused(vellum_protocol::RefusalV1::Stale);
        let frozen_reply = encode_reply_v1(
            &WireReplyV1::Executed(expected.clone()),
            vellum_wire::WireLimits::reference(),
        )
        .unwrap();
        let records = [RequestJournalRecordV1 {
            scope: RequestJournalScopeV1::new(11, 13).unwrap(),
            client: NonZeroU64::new(3).unwrap(),
            request: NonZeroU64::new(5).unwrap(),
            authenticated_subject: [17; 16],
            frozen_request: frozen_request.clone(),
            frozen_reply,
        }];

        assert_eq!(
            recovered_durable_reply(&records, &request, &frozen_request, [19; 16]),
            None
        );
        assert_eq!(
            recovered_durable_reply(&[], &request, &frozen_request, [19; 16]),
            None
        );
        assert_eq!(
            recovered_durable_reply(&records, &request, &frozen_request, [17; 16]),
            Some(Ok(expected))
        );

        let changed = encode_request_v1(
            &WireRequestV1::Execute(RequestV1 {
                command: CommandV1::Save {
                    expected: vellum_protocol::VersionWitness::new(8).unwrap(),
                },
                ..request.clone()
            }),
            vellum_wire::WireLimits::reference(),
        )
        .unwrap();
        assert_eq!(
            recovered_durable_reply(&records, &request, &changed, [17; 16]),
            Some(Err(ReplyV1::Refused(
                vellum_protocol::RefusalV1::RequestConflict
            )))
        );
    }

    #[test]
    fn identity_evidence_validates_each_resource_sort_against_issued_not_reserved() {
        let limits = WorkspaceIdentityLimitsV1 {
            max_documents: 4,
            max_roots: 4,
            max_encoded_bytes: 512,
        };
        let identity = WorkspaceIdentityMetadataV1::try_new(
            11,
            3,
            vellum_host::workspace::WorkspaceIdentityAllocationV1::try_new(2, 9, 1, 8).unwrap(),
            limits,
        )
        .unwrap();
        let workspace = vellum_protocol::WorkspaceIdV1::new(11).unwrap();
        let document =
            |ordinal| vellum_protocol::WorkspaceDocumentIdV1::new(workspace, ordinal).unwrap();
        let root = |ordinal| vellum_protocol::WorkspaceRootIdV1::new(workspace, ordinal).unwrap();

        assert!(durable_intent_matches_identity(
            &DurableEffectIntentV1::RemoveDocument {
                document: document(1)
            },
            &identity,
        ));
        assert!(!durable_intent_matches_identity(
            &DurableEffectIntentV1::RemoveDocument {
                document: document(3)
            },
            &identity,
        ));
        assert!(durable_intent_matches_identity(
            &DurableEffectIntentV1::RemoveRoot { root: root(1) },
            &identity,
        ));
        assert!(!durable_intent_matches_identity(
            &DurableEffectIntentV1::RemoveRoot { root: root(2) },
            &identity,
        ));

        let directory =
            DurableExecuteOutcomeV1::Completed(DurableEffectReceiptV1::DirectoryAdmitted(
                vellum_protocol::DirectoryAdmissionReceiptV1 {
                    root: root(1),
                    added: 2,
                    joined: 0,
                    added_documents: vec![document(1), document(2)],
                },
            ));
        assert!(durable_outcome_matches_intent(
            &DurableEffectIntentV1::AddDirectory {
                locator: "opaque".into()
            },
            &directory,
            &identity,
        ));
        let forged = DurableExecuteOutcomeV1::Completed(DurableEffectReceiptV1::DirectoryAdmitted(
            vellum_protocol::DirectoryAdmissionReceiptV1 {
                root: root(1),
                added: 1,
                joined: 0,
                added_documents: vec![document(3)],
            },
        ));
        assert!(!durable_outcome_matches_intent(
            &DurableEffectIntentV1::AddDirectory {
                locator: "opaque".into()
            },
            &forged,
            &identity,
        ));
        let duplicate = vellum_protocol::DirectoryAdmissionReceiptV1 {
            root: root(1),
            added: 2,
            joined: 0,
            added_documents: vec![document(1), document(1)],
        };
        assert!(!directory_admission_matches_identity(&duplicate, &identity));
        let incoherent_file = vellum_protocol::FileAdmissionReceiptV1 {
            document: document(1),
            root: None,
            status: vellum_protocol::FileAdmissionStatusV1::Added,
        };
        assert!(!file_admission_matches_identity(
            &incoherent_file,
            &identity
        ));
    }

    #[test]
    fn daemon_limits_require_the_complete_v3_identity_header() {
        let mut insufficient = DaemonLimits::reference();
        insufficient.workspace_identity.max_encoded_bytes = 71;
        assert_eq!(
            validate_limits(insufficient),
            Err(DaemonBuildError::InvalidLimits)
        );
        let mut exact = DaemonLimits::reference();
        exact.workspace_identity.max_encoded_bytes = 72;
        assert_eq!(validate_limits(exact), Ok(()));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn exact_export_policy_debug_never_discloses_destination() {
        let destination = PathBuf::from("/private/tmp/vellum-secret-export-name.txt");
        let policy = DaemonExportPolicyV1::exact_create_exclusive(
            vellum_protocol::DurableOperationRefV1::new([9; 16]).unwrap(),
            1,
            0,
            destination.clone(),
            FilesystemLimits {
                max_resources: 1,
                max_bytes: 16,
                max_stage_attempts: 1,
                max_metadata_bytes: 16,
            },
            4,
        )
        .unwrap();
        let debug = format!("{policy:?}");
        assert!(!debug.contains(destination.to_str().unwrap()));
        assert!(debug.contains("<redacted>"));
    }
}
