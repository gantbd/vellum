use super::{
    DigestV1, FixtureManifestV1, PreflightRefusalV1, ProviderPreflightReceiptV1, ProviderProfileV1,
    QualificationExpectationV1, ResourcePolicyExpectationV1, digest, preflight, read_package_file,
    safe_relative_path, strict_json,
};
use serde::Serialize;
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::Write;
use std::num::NonZeroU64;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use vellum_adapter_local_fs::{FilesystemLimits, LocalFilesystemFactory};
use vellum_adapter_semantic_lsp::{
    LspIndexReadinessPortV1, LspIndexReadinessV1, LspLimitsV1, LspProviderProtocolProfileV1,
    LspReadinessEvidenceSourceV1, LspReadinessNotificationSchemaV1, LspReadinessNotificationV1,
    LspServerHealthV1, LspServerIdentityDispositionV1, LspTransportTerminalV1,
    LspVirtualDocumentIssuerV1, LspVirtualDocumentV1, SemanticGenerationIssuerV1,
};
use vellum_workspace::{Workspace, reference_workspace_profile};
use vellum_host::{
    authority::{
        AuthorityActionSetV1, AuthorityActionV1, AuthorityContextV1, AuthorityResourceRefV1,
        AuthoritySessionRefV1, AuthoritySubjectRefV1, FixedAuthorityV1,
    },
    semantic::{
        SemanticCapabilitySetV1, SemanticDiagnosticSeverityV1, SemanticDocumentRefV1,
        SemanticIndexGenerationV1, SemanticProviderProvenanceV1, SemanticProviderRefV1,
        SemanticQueryV1, SemanticReplyV1, SemanticRequestRefV1, SemanticRequestV1,
        SemanticSourceRevisionV1, SemanticSpanV1, SemanticSymbolKindV1, SemanticTargetV1,
        SemanticTextV1,
    },
};
use vellum_semantic_apply_service::{
    SemanticApplyLimitsV1, SemanticApplyOutcomeV1, SemanticApplyRefusalV1, SemanticApplyRequestV1,
    SemanticApplyServiceV1, SemanticPersistenceV1,
};
use vellum_semantic_local::{
    LocalLspFreshTransportV1, LocalOrderedActorRecoveryJoinV1, LocalOrderedActorTerminalStopV1,
    LocalOrderedClockFailureV1, LocalOrderedDeadlineDispositionV1, LocalOrderedDeadlinePolicyV1,
    LocalOrderedInstantV1, LocalOrderedLspAdapterV1, LocalOrderedManagedTimedSemanticActorV1,
    LocalOrderedManagedTimedStopResolutionV1, LocalOrderedMonotonicClockV1,
    LocalOrderedProviderExtractionV1, LocalOrderedTimedStoppedRecoveryAdvanceV1,
    LocalSemanticRetirementTerminalV1, LocalSupervisedOrderedTimedLspBuildRefusalV1,
    SemanticProcessDeadlinesV1, SemanticProcessLimitsV1, SemanticProviderLaunchAssuranceV1,
    SemanticProviderProfileV1, SemanticProviderResourcePolicyV1, SemanticProviderSupportV1,
    launch_supervised_ordered_lsp_with_deadlines_rust_analyzer_standalone_v1,
    qualify_provider_launch_v1,
};
use vellum_semantic_protocol::{
    SemanticExchangeDispositionV1, SemanticExchangeRefV1, SemanticMembershipRefV1,
    SemanticProviderReadinessProjectionV1, SemanticSessionCommandV1, SemanticSessionIncarnationV1,
    SemanticSessionLimitsV1, SemanticSessionOperationRefV1, SemanticSessionOutcomeV1,
    SemanticSessionRolesV1, SemanticSessionSnapshotV1, SemanticSynchronizationEpochV1,
    SemanticSynchronizationManifestV1,
};
use vellum_semantic_service::{
    SemanticBridgeForcedEffectPolicyV1, SemanticClientRefV1, SemanticProviderReadinessV1,
    SemanticProviderStateV1, SemanticServiceLimitsV1, SemanticSnapshotV1,
};
use vellum_semantic_session_actor::{
    SemanticSessionActorAdmissionV1, SemanticSessionActorControlCompletionV1,
    SemanticSessionActorLimitsV1, SemanticSessionActorPhaseV1, SemanticSessionActorPollV1,
    SemanticSessionActorSenderV1,
};
use vellum_semantic_session_broker::SemanticBrokerContextV1;

static NEXT_SCRATCH: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiveQualificationRefusalV1 {
    Preflight(PreflightRefusalV1),
    Profile,
    Workspace,
    Protocol,
    Launch,
    Composition(LocalSupervisedOrderedTimedLspBuildRefusalV1),
    Actor,
    Control,
    ProviderDrive {
        phase: SemanticSessionActorPhaseV1,
        effect_failures: u64,
        frames_received: u64,
        malformed_messages: u64,
        server_info_refused: u64,
        notifications_refused: u64,
        inbound_server_requests: u64,
        transport_unavailable: u64,
        transport_refused: u64,
        transport_closed: u64,
        terminal: LspTransportTerminalV1,
    },
    Exchange,
    Readiness,
    Synchronization,
    Diagnostics,
    Symbols,
    Rename,
    Apply,
    Leave,
    Stop,
    Recovery,
    Cleanup,
    DeadlineCleanup {
        unresolved: usize,
        discarded_live: usize,
    },
    ExtractionCleanup {
        clients: usize,
        retiring_clients: usize,
        documents: usize,
        pending_requests: usize,
        queued_deliveries: usize,
    },
    ProviderCleanupState {
        state: vellum_semantic_local::LocalSemanticRetirementStateV1,
    },
    GracefulShutdownFailed {
        state_before_escalation: vellum_semantic_local::LocalSemanticRetirementStateV1,
        shutdown_started: u64,
        shutdown_completed: u64,
        shutdown_refused: u64,
        shutdown_aborted: u64,
        frames_received: u64,
        terminal: LspTransportTerminalV1,
    },
    TerminalEvidence,
}

impl std::fmt::Display for LiveQualificationRefusalV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Self::Preflight(refusal) = self {
            return refusal.fmt(formatter);
        }
        formatter.write_str(match self {
            Self::Preflight(_) => unreachable!("preflight returned above"),
            Self::Profile => "live provider profile is invalid",
            Self::Workspace => "private qualification workspace failed",
            Self::Protocol => "pinned provider protocol is invalid",
            Self::Launch => "qualified provider launch refused",
            Self::Composition(_) => "ordered provider composition refused",
            Self::Actor => "managed semantic actor refused",
            Self::Control => "managed actor control failed",
            Self::ProviderDrive { .. } => "provider drive failed",
            Self::Exchange => "semantic exchange failed",
            Self::Readiness => "provider did not prove current readiness",
            Self::Synchronization => "fixture synchronization failed",
            Self::Diagnostics => "diagnostic witness mismatch",
            Self::Symbols => "nested symbol witness mismatch",
            Self::Rename => "versioned rename witness mismatch",
            Self::Apply => "stale-safe Vellum apply qualification failed",
            Self::Leave => "session leave failed",
            Self::Stop => "managed actor stop failed",
            Self::Recovery => "managed actor recovery failed",
            Self::Cleanup => "provider cleanup failed",
            Self::DeadlineCleanup { .. } => "deadline cleanup remained unresolved",
            Self::ExtractionCleanup { .. } => "provider extraction remained blocked",
            Self::ProviderCleanupState { .. } => "provider cleanup did not become terminal",
            Self::GracefulShutdownFailed { .. } => "provider graceful shutdown did not terminate",
            Self::TerminalEvidence => "terminal provider evidence mismatch",
        })
    }
}

impl std::error::Error for LiveQualificationRefusalV1 {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum SupportReceiptV1 {
    Enforced,
    ObservedOnly,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct AssuranceReceiptV1 {
    executable_identity: SupportReceiptV1,
    working_directory_identity: SupportReceiptV1,
    ambient_environment_cleared: SupportReceiptV1,
    core_dumps_disabled: SupportReceiptV1,
    filesystem_confinement: SupportReceiptV1,
    network_confinement: SupportReceiptV1,
    subprocess_confinement: SupportReceiptV1,
    abrupt_parent_cleanup: SupportReceiptV1,
    whole_tree_memory_limit: SupportReceiptV1,
    whole_tree_process_limit: SupportReceiptV1,
    whole_tree_cpu_limit: SupportReceiptV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct SpanReceiptV1 {
    start: u64,
    end: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct DiagnosticReceiptV1 {
    severity: String,
    span: SpanReceiptV1,
    message_sha256: DigestV1,
    code_sha256: Option<DigestV1>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct SymbolReceiptV1 {
    kind: String,
    span: SpanReceiptV1,
    name_sha256: DigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct RenameReceiptV1 {
    source_revision: u64,
    span: SpanReceiptV1,
    replacement_sha256: DigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ApplyReceiptV1 {
    stale_source_refused: bool,
    provider_transplant_refused: bool,
    client_isolation_refused: bool,
    changed_documents: usize,
    applied_edits: usize,
    persistence_not_attempted: bool,
    linked_history_groups_published: usize,
    source_revision_before: u64,
    source_revision_after: u64,
    source_after_sha256: DigestV1,
    disk_unchanged: bool,
    attempts: u64,
    applied: u64,
    refused: u64,
    saturated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct TerminalReceiptV1 {
    actor_stop_clean: bool,
    recovery_compatible: bool,
    deadlines_retired: bool,
    provider_extraction_checked: bool,
    provider_graceful: bool,
    transport_graceful: bool,
    profile_bound: bool,
    workspace_bound: bool,
    server_identity_exact: bool,
    notification_bound: bool,
    notification_current: bool,
    notifications_observed: u64,
    readiness_promotions: u64,
    readiness_demotions: u64,
    readiness_unavailable: u64,
    requests_submitted: u64,
    requests_completed: u64,
    snapshots_opened: u64,
    rename_plans_completed: u64,
    malformed_messages: u64,
    orphan_messages: u64,
    inbound_server_requests: u64,
    saturated: bool,
}

/// Full live qualification product. The embedded preflight receipt remains a
/// distinct factor and cannot by itself establish any semantic claim.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LiveQualificationReceiptV1 {
    schema_version: u32,
    preflight: ProviderPreflightReceiptV1,
    launch_artifact_identity: DigestV1,
    launch_profile_identity: DigestV1,
    assurance: AssuranceReceiptV1,
    source_sha256: DigestV1,
    source_revision: u64,
    index_generation: u64,
    diagnostics: Vec<DiagnosticReceiptV1>,
    symbols: Vec<SymbolReceiptV1>,
    rename: Vec<RenameReceiptV1>,
    apply: ApplyReceiptV1,
    terminal: TerminalReceiptV1,
}

struct LiveGenerations(u64);

impl SemanticGenerationIssuerV1 for LiveGenerations {
    fn issue(&mut self) -> Option<SemanticIndexGenerationV1> {
        self.0 = self.0.checked_add(1)?;
        SemanticIndexGenerationV1::new(self.0)
    }
}

#[derive(Clone, Copy)]
struct LiveReadiness;

impl LspIndexReadinessPortV1 for LiveReadiness {
    fn poll_readiness(&mut self) -> LspIndexReadinessV1 {
        LspIndexReadinessV1::Indexing
    }

    fn observe_notification(
        &mut self,
        notification: LspReadinessNotificationV1,
    ) -> Option<LspIndexReadinessV1> {
        Some(match (notification.health, notification.quiescent) {
            (LspServerHealthV1::Ok, true) => LspIndexReadinessV1::Current,
            (LspServerHealthV1::Error, _) => LspIndexReadinessV1::Unavailable,
            _ => LspIndexReadinessV1::Indexing,
        })
    }
}

#[derive(Clone, Copy)]
struct LiveDocuments;

impl LspVirtualDocumentIssuerV1 for LiveDocuments {
    fn issue(&mut self, _document: SemanticDocumentRefV1) -> Option<LspVirtualDocumentV1> {
        LspVirtualDocumentV1::try_workspace_relative(
            "src/lib.rs".to_owned(),
            LspLimitsV1::reference(),
        )
    }
}

struct LiveClock {
    start: Instant,
}

impl LocalOrderedMonotonicClockV1 for LiveClock {
    fn sample(&mut self) -> Result<LocalOrderedInstantV1, LocalOrderedClockFailureV1> {
        let millis = self.start.elapsed().as_millis();
        Ok(LocalOrderedInstantV1::from_composition_tick(
            u64::try_from(millis).unwrap_or(u64::MAX),
        ))
    }
}

type LiveAdapter = LocalOrderedLspAdapterV1<
    LocalLspFreshTransportV1,
    LiveGenerations,
    LiveReadiness,
    LiveDocuments,
>;
type LiveActor = LocalOrderedManagedTimedSemanticActorV1<LiveAdapter, LiveClock>;

struct LiveScratch {
    root: PathBuf,
    workspace: PathBuf,
    private_cwd: PathBuf,
}

impl Drop for LiveScratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl LiveScratch {
    fn create(profile: &ProviderProfileV1) -> Result<Self, LiveQualificationRefusalV1> {
        let sequence = NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "vellum-semantic-live-{}-{sequence}",
            std::process::id()
        ));
        let mut root_builder = DirBuilder::new();
        root_builder.mode(0o700);
        root_builder
            .create(&root)
            .map_err(|_| LiveQualificationRefusalV1::Workspace)?;
        let workspace = root.join("workspace");
        let private_cwd = root.join("private-cwd");
        for path in [&workspace, &private_cwd] {
            let mut builder = DirBuilder::new();
            builder.mode(0o700);
            builder
                .create(path)
                .map_err(|_| LiveQualificationRefusalV1::Workspace)?;
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))
                .map_err(|_| LiveQualificationRefusalV1::Workspace)?;
        }
        let scratch = Self {
            root: fs::canonicalize(root).map_err(|_| LiveQualificationRefusalV1::Workspace)?,
            workspace: fs::canonicalize(workspace)
                .map_err(|_| LiveQualificationRefusalV1::Workspace)?,
            private_cwd: fs::canonicalize(private_cwd)
                .map_err(|_| LiveQualificationRefusalV1::Workspace)?,
        };
        scratch.copy_fixture(profile)?;
        Ok(scratch)
    }

    fn copy_fixture(&self, profile: &ProviderProfileV1) -> Result<(), LiveQualificationRefusalV1> {
        let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
        let manifest_relative = safe_relative_path(&profile.document.fixture.manifest)
            .map_err(|_| LiveQualificationRefusalV1::Workspace)?;
        let manifest_bytes = read_package_file(&fixture_root, &manifest_relative, 65_536)
            .map_err(|_| LiveQualificationRefusalV1::Workspace)?;
        let manifest: FixtureManifestV1 = strict_json::from_slice(&manifest_bytes)
            .map_err(|_| LiveQualificationRefusalV1::Workspace)?;
        let base = manifest_relative
            .parent()
            .ok_or(LiveQualificationRefusalV1::Workspace)?;
        for file in manifest.files {
            let relative = safe_relative_path(&file.path)
                .map_err(|_| LiveQualificationRefusalV1::Workspace)?;
            let bytes = read_package_file(&fixture_root, &base.join(&relative), file.bytes)
                .map_err(|_| LiveQualificationRefusalV1::Workspace)?;
            let destination = self.workspace.join(relative);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|_| LiveQualificationRefusalV1::Workspace)?;
                fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
                    .map_err(|_| LiveQualificationRefusalV1::Workspace)?;
            }
            let mut options = OpenOptions::new();
            options.write(true).create_new(true).mode(0o600);
            let mut output = options
                .open(destination)
                .map_err(|_| LiveQualificationRefusalV1::Workspace)?;
            output
                .write_all(&bytes)
                .map_err(|_| LiveQualificationRefusalV1::Workspace)?;
            output
                .sync_all()
                .map_err(|_| LiveQualificationRefusalV1::Workspace)?;
        }
        Ok(())
    }
}

pub fn qualify_live(
    profile: &ProviderProfileV1,
    executable: &Path,
) -> Result<LiveQualificationReceiptV1, LiveQualificationRefusalV1> {
    let preflight =
        preflight(profile, executable).map_err(LiveQualificationRefusalV1::Preflight)?;
    let QualificationExpectationV1::Pinned {
        artifact: _,
        platform: _,
        resource_policy,
        provider,
        ceilings,
    } = &profile.document.qualification
    else {
        return Err(LiveQualificationRefusalV1::Profile);
    };
    if provider.readiness.schema_version != 1 {
        return Err(LiveQualificationRefusalV1::Protocol);
    }
    let scratch = LiveScratch::create(profile)?;
    let source = fs::read_to_string(scratch.workspace.join("src/lib.rs"))
        .map_err(|_| LiveQualificationRefusalV1::Workspace)?;
    let provenance = SemanticProviderProvenanceV1::try_new(
        SemanticProviderRefV1::new([0x51; 16]),
        profile.document.profile_id.clone(),
        provider_version_token(&provider.server_version),
    )
    .map_err(|_| LiveQualificationRefusalV1::Profile)?;
    let resource_policy = match resource_policy {
        ResourcePolicyExpectationV1::Unbounded => SemanticProviderResourcePolicyV1::Unbounded,
    };
    let launch_profile = SemanticProviderProfileV1::try_new_with_resource_policy(
        provenance.clone(),
        executable.to_owned(),
        provider.argv.clone(),
        provider.environment.clone(),
        scratch.private_cwd.clone(),
        scratch.workspace.clone(),
        SemanticProcessDeadlinesV1 {
            launch: Duration::from_millis(ceilings.launch_ms),
            graceful_shutdown: Duration::from_millis(ceilings.graceful_stop_ms),
            kill_and_reap: Duration::from_millis(ceilings.forced_stop_ms),
        },
        SemanticProcessLimitsV1::reference(),
        resource_policy,
    )
    .map_err(|_| LiveQualificationRefusalV1::Profile)?;
    let qualified = qualify_provider_launch_v1(launch_profile)
        .map_err(|_| LiveQualificationRefusalV1::Launch)?;
    let artifact_identity = DigestV1(qualified.artifact_identity().digest());
    let launch_profile_digest = qualified.profile_identity().digest();
    let assurance = assurance_receipt(qualified.assurance());
    if assurance.whole_tree_memory_limit != SupportReceiptV1::Unsupported
        || assurance.whole_tree_process_limit != SupportReceiptV1::Unsupported
        || assurance.whole_tree_cpu_limit != SupportReceiptV1::Unsupported
    {
        return Err(LiveQualificationRefusalV1::Profile);
    }
    let protocol = LspProviderProtocolProfileV1::try_new_with_readiness_schema(
        launch_profile_digest,
        provider.server_name.clone(),
        Some(provider.server_version.clone()),
        provider.readiness.method.clone(),
        LspReadinessNotificationSchemaV1::HealthAndQuiescentWithOptionalMessage,
    )
    .ok_or(LiveQualificationRefusalV1::Protocol)?;
    let session_limits = SemanticSessionLimitsV1::reference();
    let service_limits = SemanticServiceLimitsV1 {
        max_clients: 1,
        max_provider_polls_per_pump: 1,
        ..SemanticServiceLimitsV1::reference()
    };
    let prepared = launch_supervised_ordered_lsp_with_deadlines_rust_analyzer_standalone_v1(
        qualified,
        LiveGenerations(0),
        LiveReadiness,
        LiveDocuments,
        LspLimitsV1::reference(),
        session_limits.semantic,
        SemanticClientRefV1::new([0x52; 16]),
        incarnation(),
        session_limits,
        service_limits,
        LiveClock {
            start: Instant::now(),
        },
        LocalOrderedDeadlinePolicyV1 {
            relative_budget_ticks: NonZeroU64::new(ceilings.operation_ms)
                .ok_or(LiveQualificationRefusalV1::Profile)?,
            max_deadlines: service_limits.max_pending,
        },
        &protocol,
    )
    .map_err(|failure| LiveQualificationRefusalV1::Composition(failure.reason()))?;
    let (actor, sender) = prepared
        .spawn_managed(SemanticSessionActorLimitsV1::reference())
        .map_err(|_| LiveQualificationRefusalV1::Actor)?;
    run_actor(
        actor,
        &sender,
        preflight,
        artifact_identity,
        DigestV1(launch_profile_digest),
        assurance,
        &scratch.workspace.join("src/lib.rs"),
        &source,
        provenance,
        ceilings.readiness_ms,
        ceilings.operation_ms,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_actor(
    actor: LiveActor,
    sender: &SemanticSessionActorSenderV1,
    preflight: ProviderPreflightReceiptV1,
    artifact_identity: DigestV1,
    launch_profile_identity: DigestV1,
    assurance: AssuranceReceiptV1,
    workspace_document: &Path,
    source: &str,
    provenance: SemanticProviderProvenanceV1,
    readiness_ms: u64,
    operation_ms: u64,
) -> Result<LiveQualificationReceiptV1, LiveQualificationRefusalV1> {
    let generation = match await_readiness(&actor, sender, readiness_ms) {
        Ok(generation) => generation,
        Err(LiveQualificationRefusalV1::ProviderDrive {
            phase,
            effect_failures,
            ..
        }) if phase == SemanticSessionActorPhaseV1::Failed => {
            return Err(failed_provider_evidence(
                actor,
                operation_ms,
                phase,
                effect_failures,
            ));
        }
        Err(refusal) => return Err(refusal),
    };
    let joined = receive(
        sender,
        SemanticSessionCommandV1::JoinSession {
            exchange: exchange(10),
            operation: operation(11),
            incarnation: incarnation(),
            requested_roles: SemanticSessionRolesV1::top(),
        },
        operation_ms,
    )?;
    let SemanticSessionOutcomeV1::Joined { membership, .. } = joined.outcome() else {
        return Err(LiveQualificationRefusalV1::Exchange);
    };
    let membership = *membership;
    let target = SemanticTargetV1 {
        document: SemanticDocumentRefV1::new([0x53; 16]),
        source_revision: SemanticSourceRevisionV1::new(0),
    };
    synchronize(sender, membership, target, source, operation_ms)?;
    let mut tag = 100_u64;
    let diagnostics = submit_collect_release(
        &actor,
        sender,
        membership,
        request(0x61, target, generation, SemanticQueryV1::Diagnostics)?,
        &mut tag,
        operation_ms,
    )?;
    validate_reply(&diagnostics, target, generation, &provenance)?;
    let diagnostics_receipt = diagnostic_receipt(&diagnostics, target)?;

    let symbols = submit_collect_release(
        &actor,
        sender,
        membership,
        request(
            0x62,
            target,
            generation,
            SemanticQueryV1::Symbols {
                needle: SemanticTextV1::try_new("a".to_owned(), 16)
                    .map_err(|_| LiveQualificationRefusalV1::Symbols)?,
            },
        )?,
        &mut tag,
        operation_ms,
    )?;
    validate_reply(&symbols, target, generation, &provenance)?;
    let symbols_receipt = symbol_receipt(&symbols, target)?;

    let rename = submit_collect_release(
        &actor,
        sender,
        membership,
        request(
            0x63,
            target,
            generation,
            SemanticQueryV1::PlanRename {
                offset: 32,
                new_name: SemanticTextV1::try_new("sum_values".to_owned(), 32)
                    .map_err(|_| LiveQualificationRefusalV1::Rename)?,
            },
        )?,
        &mut tag,
        operation_ms,
    )?;
    validate_reply(&rename, target, generation, &provenance)?;
    let rename_receipt = rename_receipt(&rename, target)?;
    let apply = qualify_apply_product(
        workspace_document,
        source,
        target,
        generation,
        &provenance,
        &rename,
    )?;

    let left = receive(
        sender,
        SemanticSessionCommandV1::LeaveSession {
            exchange: exchange(tag),
            operation: operation(tag + 1),
            incarnation: incarnation(),
            membership,
        },
        operation_ms,
    )?;
    if left.outcome() != &SemanticSessionOutcomeV1::Accepted {
        return Err(LiveQualificationRefusalV1::Leave);
    }
    await_control(
        &actor
            .try_drive_retirement()
            .map_err(|_| LiveQualificationRefusalV1::Control)?,
        operation_ms,
    )?;
    let terminal = finish_actor(actor, operation_ms, launch_profile_identity.0)?;
    Ok(LiveQualificationReceiptV1 {
        schema_version: 2,
        preflight,
        launch_artifact_identity: artifact_identity,
        launch_profile_identity,
        assurance,
        source_sha256: digest(source.as_bytes()),
        source_revision: target.source_revision.get(),
        index_generation: generation.get(),
        diagnostics: diagnostics_receipt,
        symbols: symbols_receipt,
        rename: rename_receipt,
        apply,
        terminal,
    })
}

fn incarnation() -> SemanticSessionIncarnationV1 {
    SemanticSessionIncarnationV1::new([0x54; 16]).expect("nonzero fixed incarnation")
}

fn context() -> SemanticBrokerContextV1 {
    SemanticBrokerContextV1 {
        subject: AuthoritySubjectRefV1::new([0x55; 16]),
        session: AuthoritySessionRefV1::new([0x56; 16]),
        allowed_roles: SemanticSessionRolesV1::top(),
        policy_epoch: 1,
    }
}

fn exchange(tag: u64) -> SemanticExchangeRefV1 {
    let mut value = [0_u8; 16];
    value[..8].copy_from_slice(&tag.to_be_bytes());
    value[8..].copy_from_slice(&(!tag).to_be_bytes());
    SemanticExchangeRefV1::new(value).expect("fixed exchange is nonzero")
}

fn operation(tag: u64) -> SemanticSessionOperationRefV1 {
    let mut value = [0_u8; 16];
    value[..8].copy_from_slice(&tag.to_be_bytes());
    value[8..].copy_from_slice(&(!tag).to_be_bytes());
    SemanticSessionOperationRefV1::new(value).expect("fixed operation is nonzero")
}

fn receive(
    sender: &SemanticSessionActorSenderV1,
    command: SemanticSessionCommandV1,
    timeout_ms: u64,
) -> Result<vellum_semantic_protocol::SemanticSessionReplyV1, LiveQualificationRefusalV1> {
    let ticket = match sender.try_exchange(context(), command) {
        SemanticSessionActorAdmissionV1::Submitted(ticket) => ticket,
        SemanticSessionActorAdmissionV1::NotSent { .. } => {
            return Err(LiveQualificationRefusalV1::Exchange);
        }
    };
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        match ticket.try_receive() {
            Ok(SemanticExchangeDispositionV1::Reply(reply)) => return Ok(reply),
            Ok(_) | Err(SemanticSessionActorPollV1::OutcomeUnknown) => {
                return Err(LiveQualificationRefusalV1::Exchange);
            }
            Err(SemanticSessionActorPollV1::Pending) if Instant::now() < deadline => {
                std::thread::yield_now();
            }
            Err(SemanticSessionActorPollV1::Pending) => {
                return Err(LiveQualificationRefusalV1::Exchange);
            }
        }
    }
}

fn await_control(
    ticket: &vellum_semantic_session_actor::SemanticSessionActorControlTicketV1,
    timeout_ms: u64,
) -> Result<(), LiveQualificationRefusalV1> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        match ticket.try_receive() {
            Ok(SemanticSessionActorControlCompletionV1::Applied) => return Ok(()),
            Ok(_) | Err(SemanticSessionActorPollV1::OutcomeUnknown) => {
                return Err(LiveQualificationRefusalV1::Control);
            }
            Err(SemanticSessionActorPollV1::Pending) if Instant::now() < deadline => {
                std::thread::yield_now();
            }
            Err(SemanticSessionActorPollV1::Pending) => {
                return Err(LiveQualificationRefusalV1::Control);
            }
        }
    }
}

fn await_readiness(
    actor: &LiveActor,
    sender: &SemanticSessionActorSenderV1,
    timeout_ms: u64,
) -> Result<SemanticIndexGenerationV1, LiveQualificationRefusalV1> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let mut tag = 1_u64;
    loop {
        if await_control(
            &actor
                .try_drive_provider()
                .map_err(|_| LiveQualificationRefusalV1::Control)?,
            timeout_ms,
        )
        .is_err()
        {
            return Err(LiveQualificationRefusalV1::ProviderDrive {
                phase: actor.phase(),
                effect_failures: actor.usage().effect_failures,
                frames_received: 0,
                malformed_messages: 0,
                server_info_refused: 0,
                notifications_refused: 0,
                inbound_server_requests: 0,
                transport_unavailable: 0,
                transport_refused: 0,
                transport_closed: 0,
                terminal: LspTransportTerminalV1::Unobserved,
            });
        }
        let description = receive(
            sender,
            SemanticSessionCommandV1::Describe {
                exchange: exchange(tag),
            },
            timeout_ms,
        )?;
        if let SemanticSessionOutcomeV1::Description(description) = description.outcome()
            && let SemanticProviderReadinessProjectionV1::Ready { generation } =
                description.provider().readiness()
        {
            return Ok(generation);
        }
        if Instant::now() >= deadline {
            return Err(LiveQualificationRefusalV1::Readiness);
        }
        tag = tag
            .checked_add(1)
            .ok_or(LiveQualificationRefusalV1::Readiness)?;
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn synchronize(
    sender: &SemanticSessionActorSenderV1,
    membership: SemanticMembershipRefV1,
    target: SemanticTargetV1,
    source: &str,
    timeout_ms: u64,
) -> Result<(), LiveQualificationRefusalV1> {
    let limits = SemanticSessionLimitsV1::reference();
    let snapshot =
        SemanticSessionSnapshotV1::try_new(target, "rust".to_owned(), source.to_owned(), limits)
            .ok_or(LiveQualificationRefusalV1::Synchronization)?;
    let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
        std::slice::from_ref(&snapshot),
        limits,
    )
    .ok_or(LiveQualificationRefusalV1::Synchronization)?;
    let epoch = SemanticSynchronizationEpochV1::new([0x57; 16])
        .ok_or(LiveQualificationRefusalV1::Synchronization)?;
    let commands = [
        SemanticSessionCommandV1::BeginSynchronization {
            exchange: exchange(20),
            operation: operation(21),
            incarnation: incarnation(),
            membership,
            epoch,
            manifest,
        },
        SemanticSessionCommandV1::SyncOpen {
            exchange: exchange(22),
            operation: operation(23),
            incarnation: incarnation(),
            membership,
            epoch,
            snapshot,
        },
        SemanticSessionCommandV1::CompleteSynchronization {
            exchange: exchange(24),
            operation: operation(25),
            incarnation: incarnation(),
            membership,
            epoch,
            manifest,
        },
    ];
    for command in commands {
        if receive(sender, command, timeout_ms)?.outcome() != &SemanticSessionOutcomeV1::Accepted {
            return Err(LiveQualificationRefusalV1::Synchronization);
        }
    }
    Ok(())
}

fn request(
    tag: u8,
    target: SemanticTargetV1,
    generation: SemanticIndexGenerationV1,
    query: SemanticQueryV1,
) -> Result<SemanticRequestV1, LiveQualificationRefusalV1> {
    Ok(SemanticRequestV1 {
        request: SemanticRequestRefV1::new([tag; 16]),
        target,
        expected_index: Some(generation),
        language: SemanticTextV1::try_new("rust".to_owned(), 16)
            .map_err(|_| LiveQualificationRefusalV1::Exchange)?,
        query,
    })
}

fn submit_collect_release(
    actor: &LiveActor,
    sender: &SemanticSessionActorSenderV1,
    membership: SemanticMembershipRefV1,
    request: SemanticRequestV1,
    next_tag: &mut u64,
    timeout_ms: u64,
) -> Result<SemanticReplyV1, LiveQualificationRefusalV1> {
    let request_ref = request.request;
    let submitted = receive(
        sender,
        SemanticSessionCommandV1::Submit {
            exchange: exchange(*next_tag),
            operation: operation(*next_tag + 1),
            incarnation: incarnation(),
            membership,
            request,
        },
        timeout_ms,
    )?;
    *next_tag += 2;
    if submitted.outcome() != &SemanticSessionOutcomeV1::Accepted {
        return Err(LiveQualificationRefusalV1::Exchange);
    }
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let reply = loop {
        await_control(
            &actor
                .try_drive_provider()
                .map_err(|_| LiveQualificationRefusalV1::Control)?,
            timeout_ms,
        )?;
        let collected = receive(
            sender,
            SemanticSessionCommandV1::Collect {
                exchange: exchange(*next_tag),
                incarnation: incarnation(),
                membership,
                request: request_ref,
            },
            timeout_ms,
        )?;
        *next_tag += 1;
        match collected.outcome() {
            SemanticSessionOutcomeV1::Complete(reply) => break reply.clone(),
            SemanticSessionOutcomeV1::Pending(_) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(2));
            }
            _ => return Err(LiveQualificationRefusalV1::Exchange),
        }
    };
    let released = receive(
        sender,
        SemanticSessionCommandV1::ReleaseRequest {
            exchange: exchange(*next_tag),
            operation: operation(*next_tag + 1),
            incarnation: incarnation(),
            membership,
            request: request_ref,
        },
        timeout_ms,
    )?;
    *next_tag += 2;
    if released.outcome() != &SemanticSessionOutcomeV1::Accepted {
        return Err(LiveQualificationRefusalV1::Exchange);
    }
    Ok(reply)
}

fn validate_reply(
    reply: &SemanticReplyV1,
    target: SemanticTargetV1,
    generation: SemanticIndexGenerationV1,
    provenance: &SemanticProviderProvenanceV1,
) -> Result<(), LiveQualificationRefusalV1> {
    if reply.target != target
        || reply.index_generation != generation
        || &reply.provider != provenance
    {
        return Err(LiveQualificationRefusalV1::Exchange);
    }
    Ok(())
}

fn diagnostic_receipt(
    reply: &SemanticReplyV1,
    target: SemanticTargetV1,
) -> Result<Vec<DiagnosticReceiptV1>, LiveQualificationRefusalV1> {
    let values = reply
        .result
        .diagnostics()
        .ok_or(LiveQualificationRefusalV1::Diagnostics)?;
    if values.is_empty() || values.iter().any(|value| value.location.target != target) {
        return Err(LiveQualificationRefusalV1::Diagnostics);
    }
    Ok(values
        .iter()
        .map(|value| DiagnosticReceiptV1 {
            severity: match value.severity {
                SemanticDiagnosticSeverityV1::Error => "error",
                SemanticDiagnosticSeverityV1::Warning => "warning",
                SemanticDiagnosticSeverityV1::Information => "information",
                SemanticDiagnosticSeverityV1::Hint => "hint",
            }
            .to_owned(),
            span: span(value.location.span),
            message_sha256: digest(value.message.as_str().as_bytes()),
            code_sha256: value
                .code
                .as_ref()
                .map(|code| digest(code.as_str().as_bytes())),
        })
        .collect())
}

fn symbol_receipt(
    reply: &SemanticReplyV1,
    target: SemanticTargetV1,
) -> Result<Vec<SymbolReceiptV1>, LiveQualificationRefusalV1> {
    let values = reply
        .result
        .symbols()
        .ok_or(LiveQualificationRefusalV1::Symbols)?;
    let expected = [
        (
            "arithmetic",
            SemanticSymbolKindV1::Module,
            SemanticSpanV1::try_new(8, 18).unwrap(),
        ),
        (
            "calculate_total",
            SemanticSymbolKindV1::Function,
            SemanticSpanV1::try_new(32, 47).unwrap(),
        ),
    ];
    if values.len() != expected.len()
        || values
            .iter()
            .zip(expected)
            .any(|(value, (name, kind, expected_span))| {
                value.name.as_str() != name
                    || value.kind != kind
                    || value.location.target != target
                    || value.location.span != expected_span
            })
    {
        return Err(LiveQualificationRefusalV1::Symbols);
    }
    Ok(values
        .iter()
        .map(|value| SymbolReceiptV1 {
            kind: match value.kind {
                SemanticSymbolKindV1::Module => "module",
                SemanticSymbolKindV1::Type => "type",
                SemanticSymbolKindV1::Function => "function",
                SemanticSymbolKindV1::Method => "method",
                SemanticSymbolKindV1::Value => "value",
                SemanticSymbolKindV1::Field => "field",
                SemanticSymbolKindV1::Unknown => "unknown",
            }
            .to_owned(),
            span: span(value.location.span),
            name_sha256: digest(value.name.as_str().as_bytes()),
        })
        .collect())
}

fn rename_receipt(
    reply: &SemanticReplyV1,
    target: SemanticTargetV1,
) -> Result<Vec<RenameReceiptV1>, LiveQualificationRefusalV1> {
    let values = reply
        .result
        .rename_plan()
        .ok_or(LiveQualificationRefusalV1::Rename)?;
    let expected = [(32, 47), (205, 220)];
    if values.len() != expected.len()
        || values.iter().zip(expected).any(|(value, (start, end))| {
            value.location.target != target
                || value.location.span != SemanticSpanV1::try_new(start, end).unwrap()
                || value.replacement.as_str() != "sum_values"
        })
    {
        return Err(LiveQualificationRefusalV1::Rename);
    }
    Ok(values
        .iter()
        .map(|value| RenameReceiptV1 {
            source_revision: value.location.target.source_revision.get(),
            span: span(value.location.span),
            replacement_sha256: digest(value.replacement.as_str().as_bytes()),
        })
        .collect())
}

fn qualify_apply_product(
    workspace_document: &Path,
    source: &str,
    target: SemanticTargetV1,
    generation: SemanticIndexGenerationV1,
    provenance: &SemanticProviderProvenanceV1,
    reply: &SemanticReplyV1,
) -> Result<ApplyReceiptV1, LiveQualificationRefusalV1> {
    let mut workspace = Workspace::open(
        &[workspace_document.to_owned()],
        reference_workspace_profile(
            LocalFilesystemFactory::new(FilesystemLimits {
                max_resources: 8,
                max_bytes: 4_096,
                max_stage_attempts: 8,
                max_metadata_bytes: 4_096,
            })
            .erase(),
        ),
    )
    .map_err(|_| LiveQualificationRefusalV1::Apply)?;
    let mut documents = workspace
        .inventory()
        .map_err(|_| LiveQualificationRefusalV1::Apply)?
        .documents;
    if documents.len() != 1 {
        return Err(LiveQualificationRefusalV1::Apply);
    }
    let document = documents.pop().ok_or(LiveQualificationRefusalV1::Apply)?;
    let before = workspace
        .document_snapshot(&document)
        .map_err(|_| LiveQualificationRefusalV1::Apply)?;
    if before.version.revision != target.source_revision.get() || before.text != source {
        return Err(LiveQualificationRefusalV1::Apply);
    }
    let client = SemanticClientRefV1::new([0x52; 16]);
    let authority_context = AuthorityContextV1 {
        subject: AuthoritySubjectRefV1::new([0x58; 16]),
        session: None,
    };
    let authority_resource = AuthorityResourceRefV1::new([0x59; 16]);
    let authority = FixedAuthorityV1::try_new(
        authority_context,
        authority_resource,
        AuthorityActionSetV1::singleton(AuthorityActionV1::EditSource),
        1,
    )
    .map_err(|_| LiveQualificationRefusalV1::Apply)?;
    let mut apply = SemanticApplyServiceV1::try_new(
        authority,
        authority_resource,
        SemanticApplyLimitsV1::reference(),
    )
    .map_err(|_| LiveQualificationRefusalV1::Apply)?;
    let snapshot = SemanticSnapshotV1::try_new(
        target,
        "rust".to_owned(),
        source.to_owned(),
        SemanticServiceLimitsV1::reference(),
    )
    .map_err(|_| LiveQualificationRefusalV1::Apply)?;
    apply
        .bind_snapshot(client, &snapshot, &mut workspace, &document)
        .map_err(|_| LiveQualificationRefusalV1::Apply)?;
    let current_provider = SemanticProviderStateV1 {
        provider: provenance.clone(),
        readiness: SemanticProviderReadinessV1::Ready { generation },
        capabilities: SemanticCapabilitySetV1::top(),
    };

    let mut stale_reply = reply.clone();
    stale_reply.target.source_revision = SemanticSourceRevisionV1::new(
        target
            .source_revision
            .get()
            .checked_add(1)
            .ok_or(LiveQualificationRefusalV1::Apply)?,
    );
    let stale_source_refused =
        apply.apply(
            &mut workspace,
            SemanticApplyRequestV1 {
                client,
                authority_context,
                current_provider: &current_provider,
                reply: &stale_reply,
            },
        ) == SemanticApplyOutcomeV1::Refused(SemanticApplyRefusalV1::SourceStale);

    let mut transplanted_provider = current_provider.clone();
    transplanted_provider.provider = SemanticProviderProvenanceV1::try_new(
        SemanticProviderRefV1::new([0x5a; 16]),
        "transplanted-profile".to_owned(),
        "1".to_owned(),
    )
    .map_err(|_| LiveQualificationRefusalV1::Apply)?;
    let provider_transplant_refused =
        apply.apply(
            &mut workspace,
            SemanticApplyRequestV1 {
                client,
                authority_context,
                current_provider: &transplanted_provider,
                reply,
            },
        ) == SemanticApplyOutcomeV1::Refused(SemanticApplyRefusalV1::ProviderStale);

    let client_isolation_refused =
        apply.apply(
            &mut workspace,
            SemanticApplyRequestV1 {
                client: SemanticClientRefV1::new([0x5b; 16]),
                authority_context,
                current_provider: &current_provider,
                reply,
            },
        ) == SemanticApplyOutcomeV1::Refused(SemanticApplyRefusalV1::ClientIsolation);
    if !stale_source_refused
        || !provider_transplant_refused
        || !client_isolation_refused
        || workspace.linked_history_group_count() != 0
        || workspace
            .document_snapshot(&document)
            .map_err(|_| LiveQualificationRefusalV1::Apply)?
            .text
            != source
    {
        return Err(LiveQualificationRefusalV1::Apply);
    }

    let SemanticApplyOutcomeV1::Applied(applied) = apply.apply(
        &mut workspace,
        SemanticApplyRequestV1 {
            client,
            authority_context,
            current_provider: &current_provider,
            reply,
        },
    ) else {
        return Err(LiveQualificationRefusalV1::Apply);
    };
    let after = workspace
        .document_snapshot(&document)
        .map_err(|_| LiveQualificationRefusalV1::Apply)?;
    let mut expected = source.to_owned();
    let edits = reply
        .result
        .rename_plan()
        .ok_or(LiveQualificationRefusalV1::Apply)?;
    for edit in edits.iter().rev() {
        let start = usize::try_from(edit.location.span.start)
            .map_err(|_| LiveQualificationRefusalV1::Apply)?;
        let end = usize::try_from(edit.location.span.end)
            .map_err(|_| LiveQualificationRefusalV1::Apply)?;
        expected.replace_range(start..end, edit.replacement.as_str());
    }
    let usage = apply.usage();
    let disk_unchanged =
        fs::read_to_string(workspace_document).is_ok_and(|on_disk| on_disk == source);
    let persistence_not_attempted = applied.persistence == SemanticPersistenceV1::NotAttempted;
    if applied.changed_documents != 1
        || applied.applied_edits != 2
        || !persistence_not_attempted
        || workspace.linked_history_group_count() != 1
        || after.text != expected
        || after.version.revision != before.version.revision.saturating_add(1)
        || !disk_unchanged
        || usage.attempts != 4
        || usage.applied != 1
        || usage.refused != 3
        || usage.saturated
    {
        return Err(LiveQualificationRefusalV1::Apply);
    }
    Ok(ApplyReceiptV1 {
        stale_source_refused,
        provider_transplant_refused,
        client_isolation_refused,
        changed_documents: applied.changed_documents,
        applied_edits: applied.applied_edits,
        persistence_not_attempted,
        linked_history_groups_published: workspace.linked_history_group_count(),
        source_revision_before: before.version.revision,
        source_revision_after: after.version.revision,
        source_after_sha256: digest(after.text.as_bytes()),
        disk_unchanged,
        attempts: usage.attempts,
        applied: usage.applied,
        refused: usage.refused,
        saturated: usage.saturated,
    })
}

fn finish_actor(
    actor: LiveActor,
    timeout_ms: u64,
    profile_digest: [u8; 32],
) -> Result<TerminalReceiptV1, LiveQualificationRefusalV1> {
    if !actor.begin_drain() {
        return Err(LiveQualificationRefusalV1::Stop);
    }
    let mut ticket = actor
        .try_begin_stop()
        .map_err(|_| LiveQualificationRefusalV1::Stop)?;
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let mut stopped = loop {
        match ticket.try_complete() {
            Ok(LocalOrderedManagedTimedStopResolutionV1::Terminal(stopped)) => break stopped,
            Ok(LocalOrderedManagedTimedStopResolutionV1::Refused(_)) => {
                return Err(LiveQualificationRefusalV1::Stop);
            }
            Err(pending) if Instant::now() < deadline => ticket = *pending,
            Err(_) => return Err(LiveQualificationRefusalV1::Stop),
        }
        std::thread::yield_now();
    };
    let stop_clean = matches!(
        stopped.stop().terminal(),
        LocalOrderedActorTerminalStopV1::Clean(_)
    );
    let timed_cleanup = loop {
        match stopped.try_advance() {
            LocalOrderedTimedStoppedRecoveryAdvanceV1::Pending(pending)
                if Instant::now() < deadline =>
            {
                stopped = *pending;
            }
            LocalOrderedTimedStoppedRecoveryAdvanceV1::Ready(cleanup) => break *cleanup,
            _ => return Err(LiveQualificationRefusalV1::Recovery),
        }
        std::thread::yield_now();
    };
    let mut cleanup = match timed_cleanup.try_retire_deadlines() {
        Ok(cleanup) => cleanup,
        Err(unresolved) => {
            let receipt = unresolved
                .unresolved()
                .expect("failed deadline retirement retains evidence");
            return Err(LiveQualificationRefusalV1::DeadlineCleanup {
                unresolved: receipt.unresolved().len(),
                discarded_live: receipt.discarded_live().len(),
            });
        }
    };
    let recovery_compatible =
        cleanup.origin().actor_recovery_join() == LocalOrderedActorRecoveryJoinV1::Compatible;
    let deadlines_retired =
        cleanup.deadlines().disposition() == LocalOrderedDeadlineDispositionV1::Retired;
    let _ = cleanup.drive_client_retirement_once();
    let mut provider = match cleanup.try_into_provider_cleanup() {
        Ok(provider) => provider,
        Err(blocked) => {
            let blockers = blocked.blockers();
            return Err(LiveQualificationRefusalV1::ExtractionCleanup {
                clients: blockers.clients,
                retiring_clients: blockers.retiring_clients,
                documents: blockers.documents,
                pending_requests: blockers.pending_requests,
                queued_deliveries: blockers.queued_deliveries,
            });
        }
    };
    let receipt = loop {
        let _ = provider.drive_once();
        match provider.try_into_terminal_receipt() {
            Ok(receipt) => break receipt,
            Err(returned) if Instant::now() < deadline => provider = *returned,
            Err(mut returned) => {
                let state_before_escalation = returned.state();
                let _ = returned.escalate();
                let forced_deadline = Instant::now() + Duration::from_millis(timeout_ms);
                let forced = loop {
                    let _ = returned.drive_once();
                    match returned.try_into_terminal_receipt() {
                        Ok(receipt) => break receipt,
                        Err(pending) if Instant::now() < forced_deadline => returned = pending,
                        Err(pending) => {
                            return Err(LiveQualificationRefusalV1::ProviderCleanupState {
                                state: pending.state(),
                            });
                        }
                    }
                    std::thread::sleep(Duration::from_millis(2));
                };
                let usage = forced.provider().provider_usage();
                return Err(LiveQualificationRefusalV1::GracefulShutdownFailed {
                    state_before_escalation,
                    shutdown_started: usage.shutdown_started,
                    shutdown_completed: usage.shutdown_completed,
                    shutdown_refused: usage.shutdown_refused,
                    shutdown_aborted: usage.shutdown_aborted,
                    frames_received: usage.frames_received,
                    terminal: forced.provider().terminal_evidence(),
                });
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    let evidence = receipt.provider().provider_qualification();
    let usage = receipt.provider().provider_usage();
    let transitions = evidence.readiness_transitions();
    let terminal = TerminalReceiptV1 {
        actor_stop_clean: stop_clean,
        recovery_compatible,
        deadlines_retired,
        provider_extraction_checked: receipt.extraction()
            == LocalOrderedProviderExtractionV1::Checked,
        provider_graceful: receipt.provider().terminal()
            == LocalSemanticRetirementTerminalV1::GracefulConfirmed,
        transport_graceful: receipt.provider().terminal_evidence()
            == LspTransportTerminalV1::GracefulClosed,
        profile_bound: evidence.profile_matches(&profile_digest),
        workspace_bound: evidence.workspace_bound(),
        server_identity_exact: evidence.server_identity()
            == LspServerIdentityDispositionV1::PinnedExact,
        notification_bound: evidence.readiness_source()
            == LspReadinessEvidenceSourceV1::NotificationBound,
        notification_current: evidence.last_notification_readiness()
            == Some(LspIndexReadinessV1::Current),
        notifications_observed: evidence.notifications_observed(),
        readiness_promotions: transitions.0,
        readiness_demotions: transitions.1,
        readiness_unavailable: transitions.2,
        requests_submitted: usage.requests_submitted,
        requests_completed: usage.requests_completed,
        snapshots_opened: usage.snapshots_opened,
        rename_plans_completed: usage.rename_plans_completed,
        malformed_messages: usage.malformed_messages,
        orphan_messages: usage.orphan_messages,
        inbound_server_requests: usage.inbound_server_requests,
        saturated: usage.saturated || evidence.saturated(),
    };
    if !terminal.actor_stop_clean
        || !terminal.recovery_compatible
        || !terminal.deadlines_retired
        || !terminal.provider_extraction_checked
        || !terminal.provider_graceful
        || !terminal.transport_graceful
        || !terminal.profile_bound
        || !terminal.workspace_bound
        || !terminal.server_identity_exact
        || !terminal.notification_bound
        || !terminal.notification_current
        || terminal.notifications_observed == 0
        || terminal.requests_submitted != 3
        || terminal.requests_completed != 3
        || terminal.snapshots_opened != 1
        || terminal.rename_plans_completed != 1
        || terminal.malformed_messages != 0
        || terminal.orphan_messages != 0
        || terminal.inbound_server_requests != 0
        || terminal.saturated
    {
        return Err(LiveQualificationRefusalV1::TerminalEvidence);
    }
    Ok(terminal)
}

fn failed_provider_evidence(
    actor: LiveActor,
    timeout_ms: u64,
    phase: SemanticSessionActorPhaseV1,
    effect_failures: u64,
) -> LiveQualificationRefusalV1 {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let Ok(mut stopped) = actor.try_recover_failed() else {
        return LiveQualificationRefusalV1::Recovery;
    };
    let timed_cleanup = loop {
        match stopped.try_advance() {
            LocalOrderedTimedStoppedRecoveryAdvanceV1::Pending(pending)
                if Instant::now() < deadline =>
            {
                stopped = *pending;
            }
            LocalOrderedTimedStoppedRecoveryAdvanceV1::Ready(cleanup) => break *cleanup,
            _ => return LiveQualificationRefusalV1::Recovery,
        }
        std::thread::yield_now();
    };
    let mut cleanup = match timed_cleanup.try_retire_deadlines() {
        Ok(cleanup) => cleanup,
        Err(unresolved) => unresolved.abandon_deadlines(),
    };
    let _ = cleanup.drive_client_retirement_once();
    let mut provider = match cleanup.try_into_provider_cleanup() {
        Ok(provider) => provider,
        Err(blocked) => blocked
            .into_owner()
            .force_into_provider_cleanup(SemanticBridgeForcedEffectPolicyV1::SkipUnsafeProvider),
    };
    let _ = provider.escalate();
    let receipt = loop {
        let _ = provider.drive_once();
        match provider.try_into_terminal_receipt() {
            Ok(receipt) => break receipt,
            Err(returned) if Instant::now() < deadline => provider = *returned,
            Err(returned) => {
                return LiveQualificationRefusalV1::ProviderCleanupState {
                    state: returned.state(),
                };
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    let usage = receipt.provider().provider_usage();
    LiveQualificationRefusalV1::ProviderDrive {
        phase,
        effect_failures,
        frames_received: usage.frames_received,
        malformed_messages: usage.malformed_messages,
        server_info_refused: usage.server_info_refused,
        notifications_refused: usage.notifications_refused,
        inbound_server_requests: usage.inbound_server_requests,
        transport_unavailable: usage.transport_unavailable,
        transport_refused: usage.transport_refused,
        transport_closed: usage.transport_closed,
        terminal: receipt.provider().terminal_evidence(),
    }
}

fn span(value: SemanticSpanV1) -> SpanReceiptV1 {
    SpanReceiptV1 {
        start: value.start,
        end: value.end,
    }
}

fn provider_version_token(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-') {
                char::from(byte)
            } else {
                '-'
            }
        })
        .collect()
}

fn assurance_receipt(value: SemanticProviderLaunchAssuranceV1) -> AssuranceReceiptV1 {
    AssuranceReceiptV1 {
        executable_identity: support(value.executable_identity),
        working_directory_identity: support(value.working_directory_identity),
        ambient_environment_cleared: support(value.ambient_environment_cleared),
        core_dumps_disabled: support(value.core_dumps_disabled),
        filesystem_confinement: support(value.filesystem_confinement),
        network_confinement: support(value.network_confinement),
        subprocess_confinement: support(value.subprocess_confinement),
        abrupt_parent_cleanup: support(value.abrupt_parent_cleanup),
        whole_tree_memory_limit: support(value.whole_tree_memory_limit),
        whole_tree_process_limit: support(value.whole_tree_process_limit),
        whole_tree_cpu_limit: support(value.whole_tree_cpu_limit),
    }
}

fn support(value: SemanticProviderSupportV1) -> SupportReceiptV1 {
    match value {
        SemanticProviderSupportV1::Enforced => SupportReceiptV1::Enforced,
        SemanticProviderSupportV1::ObservedOnly => SupportReceiptV1::ObservedOnly,
        SemanticProviderSupportV1::Unsupported => SupportReceiptV1::Unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vellum_host::semantic::{
        SemanticLimitsV1, SemanticLocationV1, SemanticResultV1, SemanticTextEditV1,
    };

    struct TestDocument(PathBuf);

    impl TestDocument {
        fn create(source: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "vellum-semantic-apply-product-{}-{}-src.rs",
                std::process::id(),
                NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed)
            ));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true).mode(0o600);
            let mut file = options.open(&path).unwrap();
            file.write_all(source.as_bytes()).unwrap();
            file.sync_all().unwrap();
            Self(path)
        }
    }

    impl Drop for TestDocument {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn real_shaped_rename_is_stale_safe_and_publishes_one_linked_history_group() {
        let source = "pub mod arithmetic {\n    pub fn calculate_total(left: i32, right: i32) -> i32 {\n        let typed: i32 = \"diagnostic witness\";\n        left + right\n    }\n}\n\npub fn fixture_entry() -> i32 {\n    arithmetic::calculate_total(20, 22)\n}\n";
        let file = TestDocument::create(source);
        let target = SemanticTargetV1 {
            document: SemanticDocumentRefV1::new([0x53; 16]),
            source_revision: SemanticSourceRevisionV1::new(0),
        };
        let generation = SemanticIndexGenerationV1::new(1).unwrap();
        let provenance = SemanticProviderProvenanceV1::try_new(
            SemanticProviderRefV1::new([0x51; 16]),
            "rust-analyzer-v1".to_owned(),
            "0.3.3113".to_owned(),
        )
        .unwrap();
        let edit = |start, end| SemanticTextEditV1 {
            location: SemanticLocationV1 {
                target,
                span: SemanticSpanV1::try_new(start, end).unwrap(),
            },
            replacement: SemanticTextV1::try_new("sum_values".to_owned(), 32).unwrap(),
        };
        let reply = SemanticReplyV1 {
            request: SemanticRequestRefV1::new([0x63; 16]),
            target,
            index_generation: generation,
            provider: provenance.clone(),
            result: SemanticResultV1::try_rename_plan(
                vec![edit(32, 47), edit(205, 220)],
                SemanticLimitsV1::reference(),
            )
            .unwrap(),
        };

        let receipt =
            qualify_apply_product(&file.0, source, target, generation, &provenance, &reply)
                .unwrap();
        assert!(receipt.stale_source_refused);
        assert!(receipt.provider_transplant_refused);
        assert!(receipt.client_isolation_refused);
        assert_eq!(receipt.changed_documents, 1);
        assert_eq!(receipt.applied_edits, 2);
        assert!(receipt.persistence_not_attempted);
        assert_eq!(receipt.linked_history_groups_published, 1);
        assert_eq!(receipt.source_revision_before, 0);
        assert_eq!(receipt.source_revision_after, 1);
        assert_eq!(
            receipt.source_after_sha256,
            digest(source.replace("calculate_total", "sum_values").as_bytes())
        );
        assert!(receipt.disk_unchanged);
        assert_eq!(receipt.attempts, 4);
        assert_eq!(receipt.applied, 1);
        assert_eq!(receipt.refused, 3);
        assert!(!receipt.saturated);
        assert_eq!(fs::read_to_string(&file.0).unwrap(), source);
    }
}
