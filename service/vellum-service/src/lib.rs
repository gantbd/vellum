//! Bounded in-process Vellum service interpreter.
#![forbid(unsafe_code)]

use keystone_replay::{ReplayDecision, ReplayRegister};
use std::{
    collections::VecDeque,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use vellum_editor::{
    DirectoryDiscoveryLimits, DirectoryDiscoveryPort, EditorView, MembershipRemovalStatus,
    Workspace, WorkspaceDocumentEdits, WorkspaceDocumentId, WorkspaceError,
    WorkspaceExportCopyErrorV1, WorkspaceRootId, WorkspaceRootKind, WorkspaceSaveReceipt,
};
use vellum_engine::{
    coordinator::DocumentVersion,
    instrumentation::{Boundary, Disposition, Recorder},
    resource::{Application, Synchronization},
    source::Replacement,
};
use vellum_host::authentication::AuthenticatedSessionV1;
use vellum_host::authority::{
    AuthorityActionSetV1, AuthorityActionV1, AuthorityContextV1, AuthorityDecisionV1,
    AuthorityPortV1, AuthorityRequestV1, AuthorityResourceRefV1, AuthoritySessionRefV1,
    AuthoritySubjectRefV1, FixedAuthorityV1,
};
use vellum_host::authority::{SaveAsDestinationEffectV1, SaveAsDestinationRequestV1};
use vellum_host::export_copy::{
    ExportCopyAuthorityPortV1, ExportCopyAuthorizationRefusalV1, ExportCopyEffectPortV1,
    ExportCopyFailureV1, ExportCopyPersistenceV1, ExportCopyReceiptV1,
};
use vellum_host::workspace::{
    WorkspaceDocumentBindingV1, WorkspaceIdentityAllocationV1, WorkspaceIdentityLimitsV1,
    WorkspaceIdentityMetadataV1, WorkspaceRootBindingKindV1, WorkspaceRootBindingV1,
};
use vellum_protocol::{
    ClientId, CommandV1, DirectoryAdmissionReceiptV1, DocumentRemovalReceiptV1,
    DurableEffectIntentV1, DurableEffectReceiptV1, DurableExecuteOutcomeV1,
    DurableExportCopyFailureV1, DurableExportCopyPersistenceV1, DurableExportCopyReceiptV1,
    DurableOperationRefV1, DurableReplaceAndSaveReceiptV1, DurableSaveReceiptV1, EditPersistenceV1,
    FileAdmissionReceiptV1, FileAdmissionStatusV1, MAX_WORKSPACE_CHANGE_PAGE_SIZE_V1,
    MAX_WORKSPACE_EDIT_DOCUMENTS_V1, MAX_WORKSPACE_EDIT_PAYLOAD_BYTES_V1,
    MAX_WORKSPACE_EDIT_REPLACEMENTS_V1, MAX_WORKSPACE_INVENTORY_PAGE_SIZE_V1,
    MembershipRemovalStatusV1, OutcomeV1, RefusalV1, ReplaceAndSaveReceiptV1, ReplyV1, RequestId,
    RequestV1, RootRemovalReceiptV1, SaveApplicationV1, SaveReceiptV1, SaveSynchronizationV1,
    SnapshotV1, VersionWitness, ViewCloseStatusV1, ViewOpenReceiptV1, ViewOpenStatusV1,
    WorkspaceChangeCursorV1, WorkspaceChangeGapV1, WorkspaceChangeKindV1, WorkspaceChangePageV1,
    WorkspaceChangeV1, WorkspaceChangesV1, WorkspaceDocumentEditV1, WorkspaceDocumentIdV1,
    WorkspaceEditReceiptV1, WorkspaceEditedDocumentV1, WorkspaceIdV1, WorkspaceInventoryCursorV1,
    WorkspaceInventoryPageV1, WorkspaceInventoryV1, WorkspaceRootIdV1, WorkspaceRootKindV1,
    WorkspaceRootV1,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceLimits {
    pub max_clients: usize,
    pub max_issued_clients: usize,
    pub max_requests: usize,
    /// Lifetime decisions available to the fixed trusted-composition
    /// authority. This independently bounds durable operations which do not
    /// enter the ordinary request replay register.
    pub max_authority_decisions: u64,
    pub max_witnesses: usize,
    pub max_inventory_documents: usize,
    pub max_inventory_roots: usize,
    pub max_inventory_page_size: usize,
    pub max_change_events: usize,
    pub max_change_page_size: usize,
    pub max_locator_bytes: usize,
    pub directory_discovery: DirectoryDiscoveryLimits,
    pub max_snapshot_bytes: usize,
    pub max_insert_bytes: usize,
}

impl vellum_protocol::ServicePort for Service {
    fn connect(&mut self) -> Result<ClientId, vellum_protocol::ServiceControlErrorV1> {
        self.connect()
            .map_err(vellum_protocol::ServiceControlErrorV1::Refused)
    }

    fn execute(
        &mut self,
        request: RequestV1,
    ) -> Result<ReplyV1, vellum_protocol::ServiceAccessErrorV1> {
        Ok(self.execute(request))
    }

    fn disconnect(
        &mut self,
        client: ClientId,
    ) -> Result<(), vellum_protocol::ServiceControlErrorV1> {
        self.disconnect(client)
            .map_err(vellum_protocol::ServiceControlErrorV1::Refused)
    }
}

impl ServiceLimits {
    pub const fn reference() -> Self {
        Self {
            max_clients: 8,
            max_issued_clients: 64,
            max_requests: 256,
            max_authority_decisions: 16_384,
            max_witnesses: 256,
            max_inventory_documents: 256,
            max_inventory_roots: 256,
            // Canonical wire profiles can encode four roots within 16 fields.
            max_inventory_page_size: MAX_WORKSPACE_INVENTORY_PAGE_SIZE_V1 as usize,
            max_change_events: 256,
            max_change_page_size: MAX_WORKSPACE_CHANGE_PAGE_SIZE_V1 as usize,
            max_locator_bytes: 4096,
            directory_discovery: DirectoryDiscoveryLimits {
                max_entries: 4096,
                max_files: 256,
                max_depth: 32,
                max_path_bytes: 4096,
                max_total_path_bytes: 262_144,
            },
            max_snapshot_bytes: 65_536,
            max_insert_bytes: 65_536,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceBuildError {
    InvalidLimits,
    Capacity,
    CounterExhausted,
    IdentityMismatch,
}

static NEXT_WORKSPACE_SCOPE: AtomicU64 = AtomicU64::new(1);

/// The four closed outcomes of consulting the bounded request register.
///
/// This classification contains no request identifiers, command content, or
/// replies, so it is safe to use as operational evidence. A full register is
/// consulted only after checking for an existing key: exact replay and
/// changed-input conflict remain observable at capacity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestRetentionOutcome {
    New,
    MatchingReplay,
    ChangedInputConflict,
    CapacityFull,
}

/// Saturating, non-authoritative operational evidence.
///
/// Counters never participate in admission or semantic decisions. Retention
/// gauges are sampled when [`Service::usage`] is called. `counter_saturated`
/// is sticky so an observer can detect loss of numeric fidelity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ServiceUsage {
    pub connect_attempts: u64,
    pub connected: u64,
    pub connection_refused: u64,
    pub disconnect_attempts: u64,
    pub disconnected: u64,
    pub disconnect_replayed: u64,
    pub disconnect_refused: u64,
    pub requests: u64,
    pub request_new: u64,
    pub request_matching_replay: u64,
    pub request_changed_input: u64,
    pub request_capacity_full: u64,
    pub accepted: u64,
    pub refused: u64,
    pub replayed: u64,
    pub conflicts: u64,
    pub stale: u64,
    pub invalid: u64,
    pub limited: u64,
    pub unavailable: u64,
    pub no_active_document: u64,
    pub views_opened: u64,
    pub views_already_open: u64,
    pub views_closed: u64,
    pub views_already_closed: u64,
    pub file_admission_requests: u64,
    pub files_added: u64,
    pub files_already_present: u64,
    pub directory_admission_requests: u64,
    pub directory_documents_added: u64,
    pub directory_documents_joined: u64,
    pub document_removal_requests: u64,
    pub documents_removed: u64,
    pub documents_already_removed: u64,
    pub document_removal_in_use: u64,
    pub root_removal_requests: u64,
    pub roots_removed: u64,
    pub roots_already_removed: u64,
    pub witnesses_revoked_by_removal: u64,
    pub authority_checks: u64,
    pub authority_granted: u64,
    pub authority_denied: u64,
    pub authority_unavailable: u64,
    pub inventory_page_requests: u64,
    pub inventory_page_started: u64,
    pub inventory_page_continued: u64,
    pub inventory_page_completed: u64,
    pub inventory_page_stale: u64,
    pub inventory_page_foreign: u64,
    pub inventory_page_invalid: u64,
    pub inventory_page_limited: u64,
    pub change_feed_requests: u64,
    pub change_feed_pages: u64,
    pub change_feed_gaps: u64,
    pub change_feed_events_published: u64,
    pub change_feed_events_evicted: u64,
    pub replace_and_save_requests: u64,
    pub replace_and_save_durable: u64,
    pub replace_and_save_not_durable: u64,
    pub replace_and_save_unknown: u64,
    pub replace_and_save_not_attempted: u64,
    pub workspace_edit_requests: u64,
    pub workspace_edit_refused: u64,
    pub workspace_edit_documents_requested: u64,
    pub workspace_edit_documents_changed: u64,
    pub workspace_edit_documents_unchanged: u64,
    pub workspace_edit_transactions_changed: u64,
    pub workspace_edit_transactions_unchanged: u64,
    pub durable_effect_requests: u64,
    pub durable_effect_completed: u64,
    pub durable_effect_refused: u64,
    pub export_copy_requests: u64,
    pub export_copy_completed: u64,
    pub export_copy_refused: u64,
    pub export_copy_durable: u64,
    pub export_copy_not_durable: u64,
    pub export_copy_unknown: u64,
    pub export_copy_not_attempted: u64,
    pub export_copy_cleanup_failed: u64,
    pub export_observations_drained: u64,
    pub export_observations_dropped: u64,
    pub export_validation_observations: u64,
    pub export_staging_observations: u64,
    pub export_publication_observations: u64,
    pub export_synchronization_observations: u64,
    pub export_observation_completed: u64,
    pub export_observation_refused: u64,
    pub export_observation_failed: u64,
    pub export_observation_unknown: u64,
    pub export_observation_partial: u64,
    pub export_observation_latency_samples: u64,
    pub export_observation_latency_nanos: u64,
    pub durable_retirement_authorizations: u64,
    pub durable_retirement_refused: u64,
    pub counter_saturated: bool,
    pub active_clients: usize,
    pub client_capacity: usize,
    pub issued_clients: usize,
    pub issued_client_capacity: usize,
    pub retired_clients: usize,
    pub retained_requests: usize,
    pub request_capacity: usize,
    pub retained_witnesses: usize,
    pub witness_capacity: usize,
}

struct WitnessRecord {
    client: ClientId,
    id: VersionWitness,
    version: DocumentVersion,
    text: Arc<str>,
}

enum RequestRetentionDecision {
    New,
    MatchingReplay(ReplyV1),
    ChangedInputConflict,
    CapacityFull,
}

impl RequestRetentionDecision {
    const fn outcome(&self) -> RequestRetentionOutcome {
        match self {
            Self::New => RequestRetentionOutcome::New,
            Self::MatchingReplay(_) => RequestRetentionOutcome::MatchingReplay,
            Self::ChangedInputConflict => RequestRetentionOutcome::ChangedInputConflict,
            Self::CapacityFull => RequestRetentionOutcome::CapacityFull,
        }
    }
}

struct ClientRecord {
    id: ClientId,
    view: Option<EditorView>,
    authority_context: AuthorityContextV1,
}

struct DocumentProjection {
    native: WorkspaceDocumentId,
    protocol: WorkspaceDocumentIdV1,
}

struct RootProjection {
    native: WorkspaceRootId,
    protocol: WorkspaceRootIdV1,
}

enum AuthorityComposition {
    Fixed(AuthorityActionSetV1),
    Supplied {
        port: Box<dyn AuthorityPortV1>,
        context: AuthorityContextV1,
        resource: AuthorityResourceRefV1,
    },
}

struct ExportCopyComposition {
    authority: Box<dyn ExportCopyAuthorityPortV1>,
    effect: Box<dyn ExportCopyEffectPortV1>,
    context: AuthorityContextV1,
    exact_document: Option<WorkspaceDocumentIdV1>,
    exact_revision: Option<u64>,
}

/// Affine daemon-to-service join for one authenticated export execution.
///
/// Keeping the session, composite authority, effect interpreter and diagnostic
/// bound in one owned product prevents a caller from accidentally pairing
/// authority from one connection with the effect of another call.
pub struct AuthenticatedExportExecutionV1<X, E> {
    authenticated: AuthenticatedSessionV1,
    authority: X,
    effect: E,
    diagnostic_capacity: usize,
}

impl<X, E> AuthenticatedExportExecutionV1<X, E> {
    pub fn new(
        authenticated: AuthenticatedSessionV1,
        authority: X,
        effect: E,
        diagnostic_capacity: usize,
    ) -> Self {
        Self {
            authenticated,
            authority,
            effect,
            diagnostic_capacity,
        }
    }
}

/// One serialized owner for a shared workspace and bounded client views.
///
/// Semantic replacements are version-bound and never borrow another client's
/// cursor, selection, active document, request namespace, or witness namespace.
pub struct Service {
    workspace: Workspace,
    workspace_id: WorkspaceIdV1,
    directory_discovery: Option<Box<dyn DirectoryDiscoveryPort>>,
    authority: Box<dyn AuthorityPortV1>,
    authority_context: AuthorityContextV1,
    authority_resource: AuthorityResourceRefV1,
    export_copy: Option<ExportCopyComposition>,
    export_diagnostics: Recorder,
    limits: ServiceLimits,
    clients: Vec<ClientRecord>,
    retired_clients: Vec<ClientId>,
    requests: ReplayRegister<(ClientId, RequestId), RequestV1, ReplyV1>,
    witnesses: Vec<WitnessRecord>,
    document_projections: Vec<DocumentProjection>,
    root_projections: Vec<RootProjection>,
    next_client: u64,
    issued_clients: usize,
    next_witness: u64,
    next_document: u64,
    next_root: u64,
    document_issued_through: u64,
    document_reserved_through: u64,
    root_issued_through: u64,
    root_reserved_through: u64,
    identity_generation: u64,
    identity_materialized: bool,
    changes: VecDeque<WorkspaceChangeV1>,
    change_epoch: u64,
    next_change_revision: u64,
    usage: ServiceUsage,
}

impl Service {
    pub const fn workspace_id(&self) -> WorkspaceIdV1 {
        self.workspace_id
    }

    /// Authorizes the separate receipt-retirement action for an active client.
    /// Storage mutation remains the daemon composition's responsibility; this
    /// method cannot retire membership or execute an editor effect.
    pub fn authorize_durable_receipt_retirement(
        &mut self,
        client: ClientId,
    ) -> Result<(), RefusalV1> {
        bump(
            &mut self.usage.durable_retirement_authorizations,
            &mut self.usage.counter_saturated,
        );
        let result = self.authorize_plan(
            client,
            AuthorityActionPlan {
                actions: [Some(AuthorityActionV1::RetireReceipt), None],
            },
        );
        if result.is_err() {
            bump(
                &mut self.usage.durable_retirement_refused,
                &mut self.usage.counter_saturated,
            );
        }
        result
    }

    pub fn new(workspace: Workspace, limits: ServiceLimits) -> Result<Self, ServiceBuildError> {
        Self::build(
            workspace,
            limits,
            None,
            AuthorityComposition::Fixed(AuthorityActionSetV1::read_only()),
            None,
        )
    }

    pub fn new_with_directory_discovery<D: DirectoryDiscoveryPort + 'static>(
        workspace: Workspace,
        limits: ServiceLimits,
        discovery: D,
    ) -> Result<Self, ServiceBuildError> {
        Self::build(
            workspace,
            limits,
            Some(Box::new(discovery)),
            AuthorityComposition::Fixed(AuthorityActionSetV1::read_only()),
            None,
        )
    }

    /// Trusted local composition with an explicit finite action ceiling.
    pub fn new_with_action_ceiling(
        workspace: Workspace,
        limits: ServiceLimits,
        actions: AuthorityActionSetV1,
    ) -> Result<Self, ServiceBuildError> {
        Self::build(
            workspace,
            limits,
            None,
            AuthorityComposition::Fixed(actions),
            None,
        )
    }

    pub fn new_with_directory_discovery_and_action_ceiling<D: DirectoryDiscoveryPort + 'static>(
        workspace: Workspace,
        limits: ServiceLimits,
        discovery: D,
        actions: AuthorityActionSetV1,
    ) -> Result<Self, ServiceBuildError> {
        Self::build(
            workspace,
            limits,
            Some(Box::new(discovery)),
            AuthorityComposition::Fixed(actions),
            None,
        )
    }

    /// Host-authenticated composition. Context and resource are retained by the
    /// service and never accepted from protocol callers.
    pub fn new_with_authority<A: AuthorityPortV1 + 'static>(
        workspace: Workspace,
        limits: ServiceLimits,
        authority: A,
        context: AuthorityContextV1,
        resource: AuthorityResourceRefV1,
    ) -> Result<Self, ServiceBuildError> {
        Self::build(
            workspace,
            limits,
            None,
            AuthorityComposition::Supplied {
                port: Box::new(authority),
                context,
                resource,
            },
            None,
        )
    }

    /// Trusted composition for authenticated create-exclusive copy export.
    ///
    /// The authenticated session is consumed so protocol callers cannot
    /// substitute a subject or session. The export authority must already
    /// contain any destination object capability installed by the host; this
    /// constructor never derives filesystem authority from a caller locator.
    pub fn new_with_authenticated_export<
        A: AuthorityPortV1 + 'static,
        X: ExportCopyAuthorityPortV1 + 'static,
        E: ExportCopyEffectPortV1 + 'static,
    >(
        workspace: Workspace,
        limits: ServiceLimits,
        authority: A,
        export_authority: X,
        export_effect: E,
        authenticated: AuthenticatedSessionV1,
        resource: AuthorityResourceRefV1,
    ) -> Result<Self, ServiceBuildError> {
        let context = authenticated.authority_context();
        if context.session.is_none() {
            return Err(ServiceBuildError::InvalidLimits);
        }
        let mut service = Self::build(
            workspace,
            limits,
            None,
            AuthorityComposition::Supplied {
                port: Box::new(authority),
                context,
                resource,
            },
            None,
        )?;
        service.export_copy = Some(ExportCopyComposition {
            authority: Box::new(export_authority),
            effect: Box::new(export_effect),
            context,
            exact_document: None,
            exact_revision: None,
        });
        Ok(service)
    }

    pub fn new_with_directory_discovery_and_authority<
        D: DirectoryDiscoveryPort + 'static,
        A: AuthorityPortV1 + 'static,
    >(
        workspace: Workspace,
        limits: ServiceLimits,
        discovery: D,
        authority: A,
        context: AuthorityContextV1,
        resource: AuthorityResourceRefV1,
    ) -> Result<Self, ServiceBuildError> {
        Self::build(
            workspace,
            limits,
            Some(Box::new(discovery)),
            AuthorityComposition::Supplied {
                port: Box::new(authority),
                context,
                resource,
            },
            None,
        )
    }

    /// Restores protocol projection identity from adapter-validated exact
    /// locator bindings. The caller must independently reconstruct and
    /// authorize the workspace; any locator, kind, or inventory-shape mismatch
    /// refuses construction.
    pub fn new_with_recovered_identity(
        workspace: Workspace,
        limits: ServiceLimits,
        identity: WorkspaceIdentityMetadataV1,
    ) -> Result<Self, ServiceBuildError> {
        Self::build(
            workspace,
            limits,
            None,
            AuthorityComposition::Fixed(AuthorityActionSetV1::read_only()),
            Some(identity),
        )
    }

    /// Restores the content-free projection identity while retaining the
    /// trusted composition root's explicit finite action ceiling.
    pub fn new_with_recovered_identity_and_action_ceiling(
        workspace: Workspace,
        limits: ServiceLimits,
        identity: WorkspaceIdentityMetadataV1,
        actions: AuthorityActionSetV1,
    ) -> Result<Self, ServiceBuildError> {
        Self::build(
            workspace,
            limits,
            None,
            AuthorityComposition::Fixed(actions),
            Some(identity),
        )
    }

    /// Restores exact startup projection bindings and the explicitly installed
    /// discovery adapter.
    pub fn new_with_directory_discovery_recovered_identity_and_action_ceiling<
        D: DirectoryDiscoveryPort + 'static,
    >(
        workspace: Workspace,
        limits: ServiceLimits,
        discovery: D,
        identity: WorkspaceIdentityMetadataV1,
        actions: AuthorityActionSetV1,
    ) -> Result<Self, ServiceBuildError> {
        Self::build(
            workspace,
            limits,
            Some(Box::new(discovery)),
            AuthorityComposition::Fixed(actions),
            Some(identity),
        )
    }

    fn build(
        workspace: Workspace,
        limits: ServiceLimits,
        directory_discovery: Option<Box<dyn DirectoryDiscoveryPort>>,
        authority_composition: AuthorityComposition,
        recovered_identity: Option<WorkspaceIdentityMetadataV1>,
    ) -> Result<Self, ServiceBuildError> {
        if limits.max_clients == 0
            || limits.max_issued_clients < limits.max_clients
            || limits.max_requests == 0
            || limits.max_authority_decisions == 0
            || limits
                .max_requests
                .checked_mul(MAX_ACTIONS_PER_COMMAND)
                .and_then(|value| u64::try_from(value).ok())
                .is_none()
            || limits.max_witnesses == 0
            || limits.max_inventory_documents == 0
            || limits.max_inventory_roots == 0
            || limits.max_inventory_page_size == 0
            || limits.max_inventory_page_size > usize::from(MAX_WORKSPACE_INVENTORY_PAGE_SIZE_V1)
            || limits.max_change_events == 0
            || limits.max_change_page_size == 0
            || limits.max_change_page_size > usize::from(MAX_WORKSPACE_CHANGE_PAGE_SIZE_V1)
            || limits.max_locator_bytes == 0
            || limits.directory_discovery.max_entries == 0
            || limits.directory_discovery.max_files == 0
            || limits.directory_discovery.max_path_bytes == 0
            || limits.directory_discovery.max_total_path_bytes == 0
            || limits.max_snapshot_bytes == 0
            || limits.max_insert_bytes == 0
            || limits.max_inventory_documents < workspace.document_capacity()
            || limits.max_inventory_roots < workspace.root_capacity()
        {
            return Err(ServiceBuildError::InvalidLimits);
        }
        let mut clients = Vec::new();
        clients
            .try_reserve_exact(limits.max_clients)
            .map_err(|_| ServiceBuildError::Capacity)?;
        let mut retired_clients = Vec::new();
        retired_clients
            .try_reserve_exact(limits.max_issued_clients)
            .map_err(|_| ServiceBuildError::Capacity)?;
        let requests = ReplayRegister::try_with_capacity(limits.max_requests)
            .map_err(|_| ServiceBuildError::Capacity)?;
        let mut witnesses = Vec::new();
        witnesses
            .try_reserve_exact(limits.max_witnesses)
            .map_err(|_| ServiceBuildError::Capacity)?;
        let mut document_projections = Vec::new();
        document_projections
            .try_reserve_exact(limits.max_inventory_documents)
            .map_err(|_| ServiceBuildError::Capacity)?;
        let mut root_projections = Vec::new();
        root_projections
            .try_reserve_exact(limits.max_inventory_roots)
            .map_err(|_| ServiceBuildError::Capacity)?;
        let mut changes = VecDeque::new();
        changes
            .try_reserve_exact(limits.max_change_events)
            .map_err(|_| ServiceBuildError::Capacity)?;
        let workspace_scope = match &recovered_identity {
            Some(identity) => identity.workspace(),
            None => NEXT_WORKSPACE_SCOPE
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                    value.checked_add(1)
                })
                .map_err(|_| ServiceBuildError::CounterExhausted)?,
        };
        let workspace_id =
            WorkspaceIdV1::new(workspace_scope).ok_or(ServiceBuildError::CounterExhausted)?;
        let (authority, authority_context, authority_resource) = match authority_composition {
            AuthorityComposition::Fixed(actions) => {
                let (context, resource) = fixed_authority_scope(workspace_scope);
                let authority = FixedAuthorityV1::try_new(
                    context,
                    resource,
                    actions,
                    limits.max_authority_decisions,
                )
                .map_err(|_| ServiceBuildError::InvalidLimits)?;
                (
                    Box::new(authority) as Box<dyn AuthorityPortV1>,
                    context,
                    resource,
                )
            }
            AuthorityComposition::Supplied {
                port,
                context,
                resource,
            } => (port, context, resource),
        };
        let mut next_document = 1;
        let mut next_root = 1;
        let mut document_issued_through = 0;
        let mut document_reserved_through = 0;
        let mut root_issued_through = 0;
        let mut root_reserved_through = 0;
        let mut identity_generation = 1;
        let identity_materialized = false;
        if let Some(identity) = recovered_identity.as_ref() {
            next_document = identity
                .document_issued_through()
                .checked_add(1)
                .ok_or(ServiceBuildError::CounterExhausted)?;
            next_root = identity
                .root_issued_through()
                .checked_add(1)
                .ok_or(ServiceBuildError::CounterExhausted)?;
            document_issued_through = identity.document_issued_through();
            document_reserved_through = identity.document_reserved_through();
            root_issued_through = identity.root_issued_through();
            root_reserved_through = identity.root_reserved_through();
            identity_generation = identity
                .generation()
                .checked_add(1)
                .ok_or(ServiceBuildError::CounterExhausted)?;
        }
        if let Some(identity) = recovered_identity.as_ref() {
            let bindings = workspace
                .binding_inventory()
                .map_err(|_| ServiceBuildError::Capacity)?;
            if bindings.documents.len() != identity.document_bindings().len()
                || bindings.roots.len() != identity.root_bindings().len()
            {
                return Err(ServiceBuildError::IdentityMismatch);
            }
            for recovered in identity.document_bindings() {
                let native = bindings
                    .documents
                    .iter()
                    .find(|binding| binding.locator == recovered.locator())
                    .ok_or(ServiceBuildError::IdentityMismatch)?;
                let protocol = WorkspaceDocumentIdV1::new(workspace_id, recovered.ordinal())
                    .ok_or(ServiceBuildError::IdentityMismatch)?;
                document_projections.push(DocumentProjection {
                    native: native.document.clone(),
                    protocol,
                });
            }
            for recovered in identity.root_bindings() {
                let expected_kind = match recovered.kind() {
                    WorkspaceRootBindingKindV1::File => WorkspaceRootKind::File,
                    WorkspaceRootBindingKindV1::Directory => WorkspaceRootKind::Directory,
                };
                let native = bindings
                    .roots
                    .iter()
                    .find(|binding| {
                        binding.locator == recovered.locator() && binding.kind == expected_kind
                    })
                    .ok_or(ServiceBuildError::IdentityMismatch)?;
                let protocol = WorkspaceRootIdV1::new(workspace_id, recovered.ordinal())
                    .ok_or(ServiceBuildError::IdentityMismatch)?;
                root_projections.push(RootProjection {
                    native: native.root.clone(),
                    protocol,
                });
            }
        }
        Ok(Self {
            workspace,
            workspace_id,
            directory_discovery,
            authority,
            authority_context,
            authority_resource,
            export_copy: None,
            export_diagnostics: Recorder::disabled(),
            limits,
            clients,
            retired_clients,
            requests,
            witnesses,
            document_projections,
            root_projections,
            next_client: 1,
            issued_clients: 0,
            next_witness: 1,
            next_document,
            next_root,
            document_issued_through,
            document_reserved_through,
            root_issued_through,
            root_reserved_through,
            identity_generation,
            identity_materialized,
            changes,
            change_epoch: identity_generation,
            next_change_revision: 1,
            usage: ServiceUsage::default(),
        })
    }

    pub fn connect(&mut self) -> Result<ClientId, RefusalV1> {
        self.connect_with_context(self.authority_context)
    }

    /// Establish one client whose authority context is the exact consumed
    /// authentication result and install one bounded, exact export policy.
    /// This is a trusted composition seam; protocol callers cannot provide any
    /// of these values. A later connection replaces no policy implicitly.
    pub fn connect_with_authenticated_export<
        X: ExportCopyAuthorityPortV1 + 'static,
        E: ExportCopyEffectPortV1 + 'static,
    >(
        &mut self,
        authenticated: AuthenticatedSessionV1,
        document: WorkspaceDocumentIdV1,
        expected_revision: u64,
        export_authority: X,
        export_effect: E,
        diagnostic_capacity: usize,
    ) -> Result<ClientId, RefusalV1> {
        let context = authenticated.authority_context();
        if context.session.is_none()
            || document.workspace() != self.workspace_id
            || self.export_copy.is_some()
        {
            return Err(RefusalV1::AuthorityUnavailable);
        }
        let recorder = Recorder::try_with_capacity(diagnostic_capacity)
            .map_err(|_| RefusalV1::EditorCapacity)?;
        let client = self.connect_with_context(context)?;
        self.export_diagnostics = recorder;
        self.export_copy = Some(ExportCopyComposition {
            authority: Box::new(export_authority),
            effect: Box::new(export_effect),
            context,
            exact_document: Some(document),
            exact_revision: Some(expected_revision),
        });
        Ok(client)
    }

    fn connect_with_context(&mut self, context: AuthorityContextV1) -> Result<ClientId, RefusalV1> {
        bump(
            &mut self.usage.connect_attempts,
            &mut self.usage.counter_saturated,
        );
        if self.clients.len() == self.limits.max_clients {
            bump(
                &mut self.usage.connection_refused,
                &mut self.usage.counter_saturated,
            );
            return Err(RefusalV1::ClientCapacity);
        }
        if self.issued_clients == self.limits.max_issued_clients {
            bump(
                &mut self.usage.connection_refused,
                &mut self.usage.counter_saturated,
            );
            return Err(RefusalV1::ClientLifetimeCapacity);
        }
        let Some(client) = ClientId::new(self.next_client) else {
            bump(
                &mut self.usage.connection_refused,
                &mut self.usage.counter_saturated,
            );
            return Err(RefusalV1::CounterExhausted);
        };
        let view = match self.workspace.open_view() {
            Ok(view) => Some(view),
            Err(WorkspaceError::Empty) => None,
            Err(error) => return Err(classify_workspace(error)),
        };
        self.next_client = self.next_client.checked_add(1).unwrap_or(0);
        self.issued_clients += 1;
        self.clients.push(ClientRecord {
            id: client,
            view,
            authority_context: context,
        });
        bump(&mut self.usage.connected, &mut self.usage.counter_saturated);
        Ok(client)
    }

    /// Prevents recovered durable operation-receipt owners from ever being
    /// reassigned to a new session. This is a pre-publication composition
    /// operation: it is refused after any client has been issued.
    pub fn reserve_client_ordinals_through(&mut self, ordinal: u64) -> Result<(), RefusalV1> {
        if self.issued_clients != 0 || !self.clients.is_empty() {
            return Err(RefusalV1::EditorUnavailable);
        }
        let next = ordinal.checked_add(1).ok_or(RefusalV1::CounterExhausted)?;
        if next > self.next_client {
            self.next_client = next;
        }
        Ok(())
    }

    pub fn disconnect(&mut self, client: ClientId) -> Result<(), RefusalV1> {
        bump(
            &mut self.usage.disconnect_attempts,
            &mut self.usage.counter_saturated,
        );
        if self.retired_clients.contains(&client) {
            bump(
                &mut self.usage.disconnect_replayed,
                &mut self.usage.counter_saturated,
            );
            return Ok(());
        }
        let Some(index) = self.clients.iter().position(|record| record.id == client) else {
            bump(
                &mut self.usage.disconnect_refused,
                &mut self.usage.counter_saturated,
            );
            return Err(RefusalV1::UnknownClient);
        };
        let retired_context = self.clients.swap_remove(index).authority_context;
        if self
            .export_copy
            .as_ref()
            .is_some_and(|composition| composition.context == retired_context)
        {
            self.export_copy = None;
        }
        self.requests
            .remove_where(|(owner, _), _, _| *owner == client);
        self.witnesses.retain(|record| record.client != client);
        self.retired_clients.push(client);
        bump(
            &mut self.usage.disconnected,
            &mut self.usage.counter_saturated,
        );
        Ok(())
    }

    pub fn execute(&mut self, request: RequestV1) -> ReplyV1 {
        bump(&mut self.usage.requests, &mut self.usage.counter_saturated);
        if !self
            .clients
            .iter()
            .any(|client| client.id == request.client)
        {
            return self.observed(ReplyV1::Refused(RefusalV1::UnknownClient));
        }
        let decision = self.request_retention_decision(&request);
        observe_request_retention(&mut self.usage, decision.outcome());
        match decision {
            RequestRetentionDecision::New => {
                let reply = match self.authorize_plan(request.client, action_plan(&request.command))
                {
                    Ok(()) => self.execute_fresh(request.client, &request.command),
                    Err(refusal) => ReplyV1::Refused(refusal),
                };
                self.requests
                    .record_new((request.client, request.request), request, reply.clone())
                    .expect("serialized replay decision must remain new until retention");
                self.observed(reply)
            }
            RequestRetentionDecision::MatchingReplay(reply) => self.observed(reply),
            RequestRetentionDecision::ChangedInputConflict => {
                self.observed(ReplyV1::Refused(RefusalV1::RequestConflict))
            }
            RequestRetentionDecision::CapacityFull => {
                self.observed(ReplyV1::Refused(RefusalV1::RequestCapacity))
            }
        }
    }

    /// Content-free pre-execution classification used by compositions which
    /// must place a durable boundary around a fresh state transition. This
    /// does not retain, authorize, or execute the request.
    pub fn classify_request(&self, request: &RequestV1) -> RequestRetentionOutcome {
        self.request_retention_decision(request).outcome()
    }

    /// Content-free session-membership query for a composition which must not
    /// create durable intent for an inactive historical identity.
    pub fn is_client_active(&self, client: ClientId) -> bool {
        self.clients.iter().any(|record| record.id == client)
    }

    pub fn usage(&self) -> ServiceUsage {
        ServiceUsage {
            active_clients: self.clients.len(),
            client_capacity: self.limits.max_clients,
            issued_clients: self.issued_clients,
            issued_client_capacity: self.limits.max_issued_clients,
            retired_clients: self.retired_clients.len(),
            retained_requests: self.requests.len(),
            request_capacity: self.limits.max_requests,
            retained_witnesses: self.witnesses.len(),
            witness_capacity: self.limits.max_witnesses,
            ..self.usage
        }
    }

    /// Content-free durable identity revision used by a composition to detect
    /// whether an accepted command must cross the metadata checkpoint before
    /// its reply is published.
    pub fn identity_revision_evidence(&self) -> (u64, u64, u64) {
        (
            self.identity_generation,
            self.document_issued_through,
            self.root_issued_through,
        )
    }

    /// Prepares a bounded allocator reservation and exact startup locator
    /// bindings suitable for the host metadata adapter. A composition must
    /// durably store this value before exposing the identifiers. Recovery
    /// reuses only adapter-validated exact bindings and never inventory order;
    /// genuinely new identifiers begin above the exact issued prefix; the
    /// reservation remains a separate capacity ceiling.
    pub fn identity_metadata(
        &mut self,
        limits: WorkspaceIdentityLimitsV1,
    ) -> Result<WorkspaceIdentityMetadataV1, RefusalV1> {
        let inventory = self.workspace_inventory()?;
        let document_reserved_through = self
            .document_issued_through
            .checked_add(
                u64::try_from(self.limits.max_inventory_documents)
                    .map_err(|_| RefusalV1::CounterExhausted)?,
            )
            .ok_or(RefusalV1::CounterExhausted)?
            .max(self.document_reserved_through);
        let root_reserved_through = self
            .root_issued_through
            .checked_add(
                u64::try_from(self.limits.max_inventory_roots)
                    .map_err(|_| RefusalV1::CounterExhausted)?,
            )
            .ok_or(RefusalV1::CounterExhausted)?
            .max(self.root_reserved_through);
        self.document_reserved_through = document_reserved_through;
        self.root_reserved_through = root_reserved_through;
        let bindings = self
            .workspace
            .binding_inventory()
            .map_err(classify_workspace)?;
        let mut documents = Vec::new();
        documents
            .try_reserve_exact(bindings.documents.len())
            .map_err(|_| RefusalV1::EditorCapacity)?;
        for binding in bindings.documents {
            let projection = self
                .document_projections
                .iter()
                .find(|projection| projection.native == binding.document)
                .ok_or(RefusalV1::EditorUnavailable)?;
            documents.push(
                WorkspaceDocumentBindingV1::try_new(projection.protocol.get(), binding.locator)
                    .map_err(|_| RefusalV1::EditorCapacity)?,
            );
        }
        let mut roots = Vec::new();
        roots
            .try_reserve_exact(bindings.roots.len())
            .map_err(|_| RefusalV1::EditorCapacity)?;
        for binding in bindings.roots {
            let projection = self
                .root_projections
                .iter()
                .find(|projection| projection.native == binding.root)
                .ok_or(RefusalV1::EditorUnavailable)?;
            roots.push(
                WorkspaceRootBindingV1::try_new(
                    projection.protocol.get(),
                    match binding.kind {
                        WorkspaceRootKind::File => WorkspaceRootBindingKindV1::File,
                        WorkspaceRootKind::Directory => WorkspaceRootBindingKindV1::Directory,
                    },
                    binding.locator,
                )
                .map_err(|_| RefusalV1::EditorCapacity)?,
            );
        }
        WorkspaceIdentityMetadataV1::try_new_with_bindings(
            inventory.workspace.get(),
            self.identity_generation,
            WorkspaceIdentityAllocationV1::try_new(
                self.document_issued_through,
                document_reserved_through,
                self.root_issued_through,
                root_reserved_through,
            )
            .map_err(|_| RefusalV1::CounterExhausted)?,
            documents,
            roots,
            limits,
        )
        .map_err(|_| RefusalV1::EditorCapacity)
    }

    /// Executes the closed restart-stable effect algebra without consulting or
    /// mutating a client's selected view. The daemon owns durable identity and
    /// must synchronize its intent before calling this method.
    pub fn execute_durable_effect(
        &mut self,
        client: ClientId,
        intent: &DurableEffectIntentV1,
    ) -> DurableExecuteOutcomeV1 {
        self.execute_durable_effect_with_resource(client, intent, self.authority_resource)
    }

    /// Daemon-only durable execution join. For export, the operation identity
    /// becomes the exact authority resource, so subject/session, source,
    /// destination and durable owner are one grant product rather than merely
    /// adjacent journal facts.
    pub fn execute_durable_effect_for_operation(
        &mut self,
        client: ClientId,
        operation: DurableOperationRefV1,
        intent: &DurableEffectIntentV1,
    ) -> DurableExecuteOutcomeV1 {
        self.execute_durable_effect_with_resource(
            client,
            intent,
            AuthorityResourceRefV1::new(operation.to_bytes()),
        )
    }

    /// Execute one exact export under the current daemon-authenticated
    /// connection session. The client identity remains durable across wire
    /// exchanges, but this affine authority context exists only for this call.
    pub fn execute_authenticated_export_for_operation<
        X: ExportCopyAuthorityPortV1 + 'static,
        E: ExportCopyEffectPortV1 + 'static,
    >(
        &mut self,
        client: ClientId,
        operation: DurableOperationRefV1,
        intent: &DurableEffectIntentV1,
        execution: AuthenticatedExportExecutionV1<X, E>,
    ) -> DurableExecuteOutcomeV1 {
        let AuthenticatedExportExecutionV1 {
            authenticated,
            authority: export_authority,
            effect: export_effect,
            diagnostic_capacity,
        } = execution;
        let DurableEffectIntentV1::ExportDocumentCopyExclusive {
            document,
            expected_revision,
            ..
        } = intent
        else {
            return DurableExecuteOutcomeV1::Refused(RefusalV1::AuthorityUnavailable);
        };
        let context = authenticated.authority_context();
        let Some(index) = self.clients.iter().position(|record| record.id == client) else {
            return DurableExecuteOutcomeV1::Refused(RefusalV1::UnknownClient);
        };
        if context.session.is_none() || self.export_copy.is_some() {
            return DurableExecuteOutcomeV1::Refused(RefusalV1::AuthorityUnavailable);
        }
        let recorder = match Recorder::try_with_capacity(diagnostic_capacity) {
            Ok(recorder) => recorder,
            Err(_) => return DurableExecuteOutcomeV1::Refused(RefusalV1::EditorCapacity),
        };
        let prior = self.clients[index].authority_context;
        self.clients[index].authority_context = context;
        self.export_diagnostics = recorder;
        self.export_copy = Some(ExportCopyComposition {
            authority: Box::new(export_authority),
            effect: Box::new(export_effect),
            context,
            exact_document: Some(*document),
            exact_revision: Some(*expected_revision),
        });
        let outcome = self.execute_durable_effect_for_operation(client, operation, intent);
        self.export_copy = None;
        if let Some(record) = self
            .clients
            .get_mut(index)
            .filter(|record| record.id == client)
        {
            record.authority_context = prior;
        }
        outcome
    }

    fn execute_durable_effect_with_resource(
        &mut self,
        client: ClientId,
        intent: &DurableEffectIntentV1,
        export_resource: AuthorityResourceRefV1,
    ) -> DurableExecuteOutcomeV1 {
        bump(
            &mut self.usage.durable_effect_requests,
            &mut self.usage.counter_saturated,
        );
        let result = match intent {
            DurableEffectIntentV1::ExportDocumentCopyExclusive {
                document,
                expected_revision,
                destination,
            } => self.execute_durable_export_copy_exclusive(
                client,
                *document,
                *expected_revision,
                destination,
                export_resource,
            ),
            _ => self
                .authorize_plan(client, durable_action_plan(intent))
                .and_then(|()| self.execute_durable_effect_authorized(intent)),
        };
        match result {
            Ok(receipt) => {
                bump(
                    &mut self.usage.durable_effect_completed,
                    &mut self.usage.counter_saturated,
                );
                DurableExecuteOutcomeV1::Completed(receipt)
            }
            Err(refusal) => {
                bump(
                    &mut self.usage.durable_effect_refused,
                    &mut self.usage.counter_saturated,
                );
                DurableExecuteOutcomeV1::Refused(refusal)
            }
        }
    }

    fn execute_durable_effect_authorized(
        &mut self,
        intent: &DurableEffectIntentV1,
    ) -> Result<DurableEffectReceiptV1, RefusalV1> {
        match intent {
            DurableEffectIntentV1::AddFile { locator } => self
                .add_file(locator)
                .map(DurableEffectReceiptV1::FileAdmitted),
            DurableEffectIntentV1::AddDirectory { locator } => self
                .add_directory(locator)
                .map(DurableEffectReceiptV1::DirectoryAdmitted),
            DurableEffectIntentV1::RemoveDocument { document } => self
                .remove_document(*document)
                .map(DurableEffectReceiptV1::DocumentRemoved),
            DurableEffectIntentV1::RemoveRoot { root } => self
                .remove_root(*root)
                .map(DurableEffectReceiptV1::RootRemoved),
            DurableEffectIntentV1::SaveDocument {
                document,
                expected_revision,
            } => self
                .durable_save_document(*document, *expected_revision)
                .map(DurableEffectReceiptV1::DocumentSaved),
            DurableEffectIntentV1::ReplaceDocumentAndSave {
                document,
                expected_revision,
                start,
                end,
                inserted,
            } => self
                .durable_replace_document_and_save(
                    *document,
                    *expected_revision,
                    *start,
                    *end,
                    inserted,
                )
                .map(DurableEffectReceiptV1::DocumentReplacedAndSaved),
            DurableEffectIntentV1::ExportDocumentCopyExclusive { .. } => {
                Err(RefusalV1::AuthorityUnavailable)
            }
        }
    }

    fn execute_durable_export_copy_exclusive(
        &mut self,
        client: ClientId,
        document: WorkspaceDocumentIdV1,
        expected_revision: u64,
        destination: &str,
        resource: AuthorityResourceRefV1,
    ) -> Result<DurableEffectReceiptV1, RefusalV1> {
        bump(
            &mut self.usage.export_copy_requests,
            &mut self.usage.counter_saturated,
        );
        let result = self.execute_durable_export_copy_exclusive_inner(
            client,
            document,
            expected_revision,
            destination,
            resource,
        );
        match &result {
            Ok(DurableEffectReceiptV1::DocumentCopyExportedExclusive(receipt)) => {
                bump(
                    &mut self.usage.export_copy_completed,
                    &mut self.usage.counter_saturated,
                );
                let counter = match receipt.persistence() {
                    DurableExportCopyPersistenceV1::Durable => &mut self.usage.export_copy_durable,
                    DurableExportCopyPersistenceV1::NotDurable => {
                        &mut self.usage.export_copy_not_durable
                    }
                    DurableExportCopyPersistenceV1::Unknown => &mut self.usage.export_copy_unknown,
                    DurableExportCopyPersistenceV1::NotAttempted => {
                        &mut self.usage.export_copy_not_attempted
                    }
                };
                bump(counter, &mut self.usage.counter_saturated);
                if receipt.cleanup_failed() {
                    bump(
                        &mut self.usage.export_copy_cleanup_failed,
                        &mut self.usage.counter_saturated,
                    );
                }
            }
            Ok(_) => unreachable!("export interpreter returns only export receipts"),
            Err(_) => bump(
                &mut self.usage.export_copy_refused,
                &mut self.usage.counter_saturated,
            ),
        }
        result
    }

    fn execute_durable_export_copy_exclusive_inner(
        &mut self,
        client: ClientId,
        document: WorkspaceDocumentIdV1,
        expected_revision: u64,
        destination: &str,
        resource: AuthorityResourceRefV1,
    ) -> Result<DurableEffectReceiptV1, RefusalV1> {
        let context = self
            .clients
            .iter()
            .find(|record| record.id == client)
            .map(|record| record.authority_context)
            .ok_or(RefusalV1::UnknownClient)?;
        // Export is intentionally unavailable to synthetic/anonymous service
        // contexts. Only a host-established authenticated session may enter
        // the composite authority interpreter.
        if context.session.is_none() {
            return Err(RefusalV1::AuthorityUnavailable);
        }
        if destination.len() > self.limits.max_locator_bytes {
            return Err(RefusalV1::InputLimit);
        }
        if document.workspace() != self.workspace_id {
            return Err(RefusalV1::UnknownDocument);
        }
        let native = self
            .document_projections
            .iter()
            .find(|projection| projection.protocol == document)
            .map(|projection| projection.native.clone())
            .ok_or(RefusalV1::UnknownDocument)?;
        let mut composition = self
            .export_copy
            .take()
            .ok_or(RefusalV1::AuthorityUnavailable)?;
        if context != composition.context
            || composition
                .exact_document
                .is_some_and(|expected| expected != document)
            || composition
                .exact_revision
                .is_some_and(|expected| expected != expected_revision)
        {
            self.export_copy = Some(composition);
            return Err(RefusalV1::AuthorityDenied);
        }
        let source_request = AuthorityRequestV1 {
            context,
            resource,
            action: AuthorityActionV1::ExportSourceCopy,
        };
        let destination_request = SaveAsDestinationRequestV1::new(
            context,
            resource,
            destination.into(),
            SaveAsDestinationEffectV1::CreateExclusive,
        );
        let authorization = self.workspace.authorize_document_export_copy(
            &native,
            expected_revision,
            &mut *composition.authority,
            source_request,
            destination_request,
        );
        let result = match authorization {
            Ok(authorization) => {
                let receipt = composition
                    .effect
                    .execute_export_copy(authorization, &mut self.export_diagnostics);
                self.drain_export_diagnostics();
                project_export_copy_receipt(document, receipt)
                    .map(DurableEffectReceiptV1::DocumentCopyExportedExclusive)
            }
            Err(WorkspaceExportCopyErrorV1::Stale) => Err(RefusalV1::Stale),
            Err(WorkspaceExportCopyErrorV1::Authority(
                ExportCopyAuthorizationRefusalV1::SourceDenied,
            )) => Err(RefusalV1::AuthorityDenied),
            Err(WorkspaceExportCopyErrorV1::Authority(
                ExportCopyAuthorizationRefusalV1::SourceUnavailable,
            ))
            | Err(WorkspaceExportCopyErrorV1::Preparation(_)) => {
                Err(RefusalV1::AuthorityUnavailable)
            }
            Err(WorkspaceExportCopyErrorV1::Workspace(error)) => Err(classify_workspace(error)),
        };
        self.export_copy = Some(composition);
        result
    }

    fn drain_export_diagnostics(&mut self) {
        while let Some(observation) = self.export_diagnostics.pop_front() {
            bump(
                &mut self.usage.export_observations_drained,
                &mut self.usage.counter_saturated,
            );
            let boundary = match observation.boundary {
                Boundary::SubstrateValidation => {
                    Some(&mut self.usage.export_validation_observations)
                }
                Boundary::SubstrateStaging => Some(&mut self.usage.export_staging_observations),
                Boundary::SubstrateReplacement => {
                    Some(&mut self.usage.export_publication_observations)
                }
                Boundary::SubstrateSynchronization => {
                    Some(&mut self.usage.export_synchronization_observations)
                }
                _ => None,
            };
            if let Some(counter) = boundary {
                bump(counter, &mut self.usage.counter_saturated);
            }
            let disposition = match observation.disposition {
                Disposition::Completed => &mut self.usage.export_observation_completed,
                Disposition::Refused => &mut self.usage.export_observation_refused,
                Disposition::Failed => &mut self.usage.export_observation_failed,
                Disposition::Unknown => &mut self.usage.export_observation_unknown,
                Disposition::Partial => &mut self.usage.export_observation_partial,
            };
            bump(disposition, &mut self.usage.counter_saturated);
            if let Some(duration) = observation.service_time {
                bump(
                    &mut self.usage.export_observation_latency_samples,
                    &mut self.usage.counter_saturated,
                );
                let nanos = u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX);
                let (sum, overflow) = self
                    .usage
                    .export_observation_latency_nanos
                    .overflowing_add(nanos);
                self.usage.export_observation_latency_nanos = if overflow { u64::MAX } else { sum };
                self.usage.counter_saturated |= overflow;
            }
        }
        self.usage.export_observations_dropped = self.export_diagnostics.dropped();
    }

    fn durable_document_view(
        &self,
        document: WorkspaceDocumentIdV1,
    ) -> Result<EditorView, RefusalV1> {
        if document.workspace() != self.workspace_id {
            return Err(RefusalV1::UnknownDocument);
        }
        let native = self
            .document_projections
            .iter()
            .find(|projection| projection.protocol == document)
            .map(|projection| projection.native.clone())
            .ok_or(RefusalV1::UnknownDocument)?;
        let mut view = self.workspace.open_view().map_err(classify_workspace)?;
        self.workspace
            .select_document_id(&mut view, &native)
            .map_err(|_| RefusalV1::UnknownDocument)?;
        Ok(view)
    }

    fn durable_save_document(
        &mut self,
        document: WorkspaceDocumentIdV1,
        expected_revision: u64,
    ) -> Result<DurableSaveReceiptV1, RefusalV1> {
        let view = self.durable_document_view(document)?;
        let version = self.workspace.version(&view).map_err(classify_workspace)?;
        if version.revision != expected_revision {
            return Err(RefusalV1::Stale);
        }
        let persistence = project_save_receipt(
            self.workspace
                .save_snapshot(&view, &version)
                .map_err(classify_workspace)?,
        );
        Ok(DurableSaveReceiptV1 {
            document,
            source_revision: version.revision,
            persistence,
        })
    }

    fn durable_replace_document_and_save(
        &mut self,
        document: WorkspaceDocumentIdV1,
        expected_revision: u64,
        start: u64,
        end: u64,
        inserted: &str,
    ) -> Result<DurableReplaceAndSaveReceiptV1, RefusalV1> {
        let view = self.durable_document_view(document)?;
        self.preflight_changes(1)?;
        let captured = self.workspace.snapshot(&view).map_err(classify_workspace)?;
        if captured.version.revision != expected_revision {
            return Err(RefusalV1::Stale);
        }
        if inserted.len() > self.limits.max_insert_bytes {
            return Err(RefusalV1::InputLimit);
        }
        let start = usize::try_from(start).map_err(|_| RefusalV1::InvalidRange)?;
        let end = usize::try_from(end).map_err(|_| RefusalV1::InvalidRange)?;
        if start > end
            || end > captured.text.len()
            || !captured.text.is_char_boundary(start)
            || !captured.text.is_char_boundary(end)
        {
            return Err(RefusalV1::InvalidRange);
        }
        let new_len = captured
            .text
            .len()
            .checked_sub(end - start)
            .and_then(|length| length.checked_add(inserted.len()))
            .ok_or(RefusalV1::InputLimit)?;
        if new_len > self.limits.max_snapshot_bytes {
            return Err(RefusalV1::SnapshotLimit);
        }
        let mut projected = String::new();
        projected
            .try_reserve_exact(new_len)
            .map_err(|_| RefusalV1::EditorCapacity)?;
        projected.push_str(&captured.text[..start]);
        projected.push_str(inserted);
        projected.push_str(&captured.text[end..]);
        let replacement = Replacement {
            start,
            end,
            expected: copy_string(&captured.text[start..end])?,
            inserted: copy_string(inserted)?,
        };
        self.workspace
            .edit_snapshot(&view, &captured.version, vec![replacement])
            .map_err(classify_workspace)?;
        let version = self.workspace.version(&view).map_err(classify_workspace)?;
        self.publish_change(WorkspaceChangeKindV1::DocumentChanged {
            document,
            source_revision: version.revision,
        });
        let persistence = match self.workspace.save_snapshot(&view, &version) {
            Ok(receipt) => classify_edit_persistence(project_save_receipt(receipt)),
            Err(error) => EditPersistenceV1::NotAttempted(classify_workspace(error)),
        };
        Ok(DurableReplaceAndSaveReceiptV1 {
            document,
            source_revision: version.revision,
            text: Arc::from(projected),
            persistence,
        })
    }

    fn observed(&mut self, reply: ReplyV1) -> ReplyV1 {
        match &reply {
            ReplyV1::Accepted(_) => {
                bump(&mut self.usage.accepted, &mut self.usage.counter_saturated)
            }
            ReplyV1::Refused(refusal) => {
                bump(&mut self.usage.refused, &mut self.usage.counter_saturated);
                let category = match refusal {
                    RefusalV1::RequestConflict
                    | RefusalV1::EditorConflict
                    | RefusalV1::DocumentInUse => Some(&mut self.usage.conflicts),
                    RefusalV1::Stale | RefusalV1::StaleInventoryCursor => {
                        Some(&mut self.usage.stale)
                    }
                    RefusalV1::UnknownClient
                    | RefusalV1::UnknownWitness
                    | RefusalV1::UnknownDocument
                    | RefusalV1::UnknownRoot
                    | RefusalV1::ForeignInventoryCursor
                    | RefusalV1::InvalidInventoryCursor
                    | RefusalV1::ForeignChangeCursor
                    | RefusalV1::InvalidChangeCursor
                    | RefusalV1::InvalidRange => Some(&mut self.usage.invalid),
                    RefusalV1::NoActiveDocument => Some(&mut self.usage.no_active_document),
                    RefusalV1::AuthorityDenied | RefusalV1::AuthorityUnavailable => None,
                    RefusalV1::ClientCapacity
                    | RefusalV1::ClientLifetimeCapacity
                    | RefusalV1::RequestCapacity
                    | RefusalV1::WitnessCapacity
                    | RefusalV1::InputLimit
                    | RefusalV1::SnapshotLimit
                    | RefusalV1::EditorCapacity
                    | RefusalV1::CounterExhausted => Some(&mut self.usage.limited),
                    RefusalV1::EditorClosed
                    | RefusalV1::EditorIncomplete
                    | RefusalV1::EditorUnavailable => Some(&mut self.usage.unavailable),
                };
                if let Some(category) = category {
                    bump(category, &mut self.usage.counter_saturated);
                }
            }
        }
        reply
    }

    fn request_retention_decision(&self, request: &RequestV1) -> RequestRetentionDecision {
        project_request_decision(
            self.requests
                .classify(&(request.client, request.request), request),
        )
    }

    fn authorize_plan(
        &mut self,
        client: ClientId,
        plan: AuthorityActionPlan,
    ) -> Result<(), RefusalV1> {
        let context = self
            .clients
            .iter()
            .find(|record| record.id == client)
            .map(|record| record.authority_context)
            .ok_or(RefusalV1::UnknownClient)?;
        for action in plan.actions.into_iter().flatten() {
            let request = AuthorityRequestV1 {
                context,
                resource: self.authority_resource,
                action,
            };
            bump(
                &mut self.usage.authority_checks,
                &mut self.usage.counter_saturated,
            );
            match self.authority.authorize(request) {
                AuthorityDecisionV1::Granted(evidence) if evidence.request() == request => {
                    bump(
                        &mut self.usage.authority_granted,
                        &mut self.usage.counter_saturated,
                    );
                }
                AuthorityDecisionV1::Granted(_) | AuthorityDecisionV1::Unavailable => {
                    bump(
                        &mut self.usage.authority_unavailable,
                        &mut self.usage.counter_saturated,
                    );
                    return Err(RefusalV1::AuthorityUnavailable);
                }
                AuthorityDecisionV1::Denied => {
                    bump(
                        &mut self.usage.authority_denied,
                        &mut self.usage.counter_saturated,
                    );
                    return Err(RefusalV1::AuthorityDenied);
                }
            }
        }
        Ok(())
    }

    fn execute_fresh(&mut self, client: ClientId, command: &CommandV1) -> ReplyV1 {
        let result = match command {
            CommandV1::WorkspaceInventory => self
                .workspace_inventory()
                .map(OutcomeV1::WorkspaceInventory),
            CommandV1::WorkspaceChanges { after, page_size } => self
                .workspace_changes(*after, *page_size)
                .map(OutcomeV1::WorkspaceChanges),
            CommandV1::WorkspaceInventoryPage { cursor, page_size } => self
                .workspace_inventory_page(*cursor, *page_size)
                .map(OutcomeV1::WorkspaceInventoryPage),
            CommandV1::OpenView => self.open_view(client).map(OutcomeV1::ViewOpened),
            CommandV1::CloseView => self.close_view(client).map(OutcomeV1::ViewClosed),
            CommandV1::AddFile { locator } => self.add_file(locator).map(OutcomeV1::FileAdmitted),
            CommandV1::AddDirectory { locator } => self
                .add_directory(locator)
                .map(OutcomeV1::DirectoryAdmitted),
            CommandV1::RemoveDocument { document } => self
                .remove_document(*document)
                .map(OutcomeV1::DocumentRemoved),
            CommandV1::RemoveRoot { root } => self.remove_root(*root).map(OutcomeV1::RootRemoved),
            CommandV1::SelectDocument { document } => self
                .select_document(client, *document)
                .map(OutcomeV1::DocumentSelected),
            CommandV1::Save { expected } => self.save(client, *expected).map(OutcomeV1::Saved),
            CommandV1::ReplaceAndSave {
                expected,
                start,
                end,
                inserted,
            } => self
                .replace_and_save(client, *expected, *start, *end, inserted)
                .map(OutcomeV1::ReplacedAndSaved),
            CommandV1::Snapshot => self.snapshot(client).map(OutcomeV1::Snapshot),
            CommandV1::Replace {
                expected,
                start,
                end,
                inserted,
            } => self
                .replace(client, *expected, *start, *end, inserted)
                .map(OutcomeV1::Replaced),
            CommandV1::WorkspaceEdit { documents } => {
                bump(
                    &mut self.usage.workspace_edit_requests,
                    &mut self.usage.counter_saturated,
                );
                add(
                    &mut self.usage.workspace_edit_documents_requested,
                    u64::try_from(documents.len()).unwrap_or(u64::MAX),
                    &mut self.usage.counter_saturated,
                );
                let result = self.workspace_edit(client, documents);
                match &result {
                    Ok(receipt) => {
                        let changed = receipt.documents.len();
                        add(
                            &mut self.usage.workspace_edit_documents_changed,
                            u64::try_from(changed).unwrap_or(u64::MAX),
                            &mut self.usage.counter_saturated,
                        );
                        add(
                            &mut self.usage.workspace_edit_documents_unchanged,
                            u64::try_from(documents.len().saturating_sub(changed))
                                .unwrap_or(u64::MAX),
                            &mut self.usage.counter_saturated,
                        );
                        let counter = if changed == 0 {
                            &mut self.usage.workspace_edit_transactions_unchanged
                        } else {
                            &mut self.usage.workspace_edit_transactions_changed
                        };
                        bump(counter, &mut self.usage.counter_saturated);
                    }
                    Err(_) => bump(
                        &mut self.usage.workspace_edit_refused,
                        &mut self.usage.counter_saturated,
                    ),
                }
                result.map(OutcomeV1::WorkspaceEdited)
            }
        };
        match result {
            Ok(outcome) => ReplyV1::Accepted(outcome),
            Err(refusal) => ReplyV1::Refused(refusal),
        }
    }

    fn snapshot(&mut self, client: ClientId) -> Result<SnapshotV1, RefusalV1> {
        let view = self.view(client)?;
        self.preflight_witness()?;
        let captured = self.workspace.snapshot(&view).map_err(classify_workspace)?;
        if captured.text.len() > self.limits.max_snapshot_bytes {
            return Err(RefusalV1::SnapshotLimit);
        }
        let text: Arc<str> = Arc::from(captured.text);
        Ok(self.install_witness(client, captured.version, text))
    }

    fn open_view(&mut self, client: ClientId) -> Result<ViewOpenReceiptV1, RefusalV1> {
        let index = self
            .clients
            .iter()
            .position(|record| record.id == client)
            .ok_or(RefusalV1::UnknownClient)?;
        let (view, status) = match self.clients[index].view.clone() {
            Some(view) => (view, ViewOpenStatusV1::AlreadyOpen),
            None => (
                self.workspace.open_view().map_err(classify_workspace)?,
                ViewOpenStatusV1::Opened,
            ),
        };
        let native = view.active_document_id().clone();
        // Materialize the stable scoped projection before publishing it.
        // All fallible preparation precedes the view transition, so a refusal
        // cannot leave a newly opened view hidden behind a refused reply.
        self.workspace_inventory()?;
        let document = self
            .document_projections
            .iter()
            .find(|projection| projection.native == native)
            .map(|projection| projection.protocol)
            .ok_or(RefusalV1::EditorUnavailable)?;
        if status == ViewOpenStatusV1::Opened {
            self.clients[index].view = Some(view);
        }
        match status {
            ViewOpenStatusV1::Opened => bump(
                &mut self.usage.views_opened,
                &mut self.usage.counter_saturated,
            ),
            ViewOpenStatusV1::AlreadyOpen => bump(
                &mut self.usage.views_already_open,
                &mut self.usage.counter_saturated,
            ),
        }
        Ok(ViewOpenReceiptV1 { document, status })
    }

    fn close_view(&mut self, client: ClientId) -> Result<ViewCloseStatusV1, RefusalV1> {
        let view = self
            .clients
            .iter_mut()
            .find(|record| record.id == client)
            .ok_or(RefusalV1::UnknownClient)?;
        // Witnesses are client/source-revision evidence, not capabilities held
        // by the presentation view. Closing selection state intentionally does
        // not revoke them; normal document/version checks still gate later use.
        let status = if view.view.take().is_some() {
            ViewCloseStatusV1::Closed
        } else {
            ViewCloseStatusV1::AlreadyClosed
        };
        match status {
            ViewCloseStatusV1::Closed => bump(
                &mut self.usage.views_closed,
                &mut self.usage.counter_saturated,
            ),
            ViewCloseStatusV1::AlreadyClosed => bump(
                &mut self.usage.views_already_closed,
                &mut self.usage.counter_saturated,
            ),
        }
        Ok(status)
    }

    fn add_file(&mut self, locator: &str) -> Result<FileAdmissionReceiptV1, RefusalV1> {
        bump(
            &mut self.usage.file_admission_requests,
            &mut self.usage.counter_saturated,
        );
        self.validate_locator(locator)?;
        // Establish the pre-effect projection boundary so only membership
        // introduced by this command is published as new.
        self.workspace_inventory()?;
        self.preflight_changes(2)?;
        let document_start = self.document_projections.len();
        let root_start = self.root_projections.len();
        let admission = self
            .workspace
            .add_file(Path::new(locator))
            .map_err(classify_workspace)?;
        self.workspace_inventory()?;
        let document = self
            .document_projections
            .iter()
            .find(|projection| projection.native == admission.document)
            .map(|projection| projection.protocol)
            .ok_or(RefusalV1::EditorUnavailable)?;
        let root = match admission.root {
            Some(native) => Some(
                self.root_projections
                    .iter()
                    .find(|projection| projection.native == native)
                    .map(|projection| projection.protocol)
                    .ok_or(RefusalV1::EditorUnavailable)?,
            ),
            None => None,
        };
        let status = if admission.added {
            if root.is_none() {
                return Err(RefusalV1::EditorUnavailable);
            }
            bump(
                &mut self.usage.files_added,
                &mut self.usage.counter_saturated,
            );
            FileAdmissionStatusV1::Added
        } else {
            if root.is_some() {
                return Err(RefusalV1::EditorUnavailable);
            }
            bump(
                &mut self.usage.files_already_present,
                &mut self.usage.counter_saturated,
            );
            FileAdmissionStatusV1::AlreadyPresent
        };
        for index in document_start..self.document_projections.len() {
            let document = self.document_projections[index].protocol;
            self.publish_change(WorkspaceChangeKindV1::DocumentAdded(document));
        }
        for index in root_start..self.root_projections.len() {
            let root = self.root_projections[index].protocol;
            self.publish_change(WorkspaceChangeKindV1::RootAdded(root));
        }
        Ok(FileAdmissionReceiptV1 {
            document,
            root,
            status,
        })
    }

    fn add_directory(&mut self, locator: &str) -> Result<DirectoryAdmissionReceiptV1, RefusalV1> {
        bump(
            &mut self.usage.directory_admission_requests,
            &mut self.usage.counter_saturated,
        );
        self.validate_locator(locator)?;
        self.workspace_inventory()?;
        self.preflight_changes(
            self.limits
                .max_inventory_documents
                .saturating_add(self.limits.max_inventory_roots),
        )?;
        let document_start = self.document_projections.len();
        let root_start = self.root_projections.len();
        let discovery = self
            .directory_discovery
            .as_deref_mut()
            .ok_or(RefusalV1::EditorUnavailable)?;
        let admission = self
            .workspace
            .add_directory(
                Path::new(locator),
                self.limits.directory_discovery,
                discovery,
            )
            .map_err(classify_workspace)?;
        self.workspace_inventory()?;
        let mut added_documents = Vec::new();
        added_documents
            .try_reserve_exact(admission.documents.len())
            .map_err(|_| RefusalV1::EditorCapacity)?;
        for native in &admission.documents {
            added_documents.push(
                self.document_projections
                    .iter()
                    .find(|projection| projection.native == *native)
                    .map(|projection| projection.protocol)
                    .ok_or(RefusalV1::EditorUnavailable)?,
            );
        }
        if added_documents.len() != admission.added {
            return Err(RefusalV1::EditorUnavailable);
        }
        let root = self
            .root_projections
            .iter()
            .find(|projection| projection.native == admission.root)
            .map(|projection| projection.protocol)
            .ok_or(RefusalV1::EditorUnavailable)?;
        let added = u64::try_from(admission.added).map_err(|_| RefusalV1::CounterExhausted)?;
        let joined = u64::try_from(admission.joined).map_err(|_| RefusalV1::CounterExhausted)?;
        add(
            &mut self.usage.directory_documents_added,
            added,
            &mut self.usage.counter_saturated,
        );
        add(
            &mut self.usage.directory_documents_joined,
            joined,
            &mut self.usage.counter_saturated,
        );
        for index in document_start..self.document_projections.len() {
            let document = self.document_projections[index].protocol;
            self.publish_change(WorkspaceChangeKindV1::DocumentAdded(document));
        }
        for index in root_start..self.root_projections.len() {
            let root = self.root_projections[index].protocol;
            self.publish_change(WorkspaceChangeKindV1::RootAdded(root));
        }
        Ok(DirectoryAdmissionReceiptV1 {
            root,
            added,
            joined,
            added_documents,
        })
    }

    fn remove_document(
        &mut self,
        document: WorkspaceDocumentIdV1,
    ) -> Result<DocumentRemovalReceiptV1, RefusalV1> {
        self.preflight_changes(1)?;
        bump(
            &mut self.usage.document_removal_requests,
            &mut self.usage.counter_saturated,
        );
        if document.workspace() != self.workspace_id {
            return Err(RefusalV1::UnknownDocument);
        }
        let native = self
            .document_projections
            .iter()
            .find(|projection| projection.protocol == document)
            .map(|projection| projection.native.clone())
            .ok_or(RefusalV1::UnknownDocument)?;
        if self.clients.iter().any(|client| {
            client
                .view
                .as_ref()
                .is_some_and(|view| view.active_document_id() == &native)
        }) {
            bump(
                &mut self.usage.document_removal_in_use,
                &mut self.usage.counter_saturated,
            );
            return Err(RefusalV1::DocumentInUse);
        }
        let version = match self.workspace.document_version(&native) {
            Ok(version) => Some(version),
            Err(WorkspaceError::InvalidView) => None,
            Err(error) => return Err(classify_workspace(error)),
        };
        if version.is_some() && self.identity_materialized && self.identity_generation == u64::MAX {
            return Err(RefusalV1::CounterExhausted);
        }
        let revoked = version.as_ref().map_or(0, |version| {
            self.witnesses
                .iter()
                .filter(|record| record.version.document == version.document)
                .count()
        });
        let witnesses_revoked = u64::try_from(revoked).map_err(|_| RefusalV1::EditorCapacity)?;
        let status = self
            .workspace
            .remove_document(&native)
            .map_err(|error| match error {
                WorkspaceError::InvalidView => RefusalV1::UnknownDocument,
                other => classify_workspace(other),
            })?;
        if let Some(version) = version {
            self.witnesses
                .retain(|record| record.version.document != version.document);
        }
        let status = match status {
            MembershipRemovalStatus::Removed => {
                bump(
                    &mut self.usage.documents_removed,
                    &mut self.usage.counter_saturated,
                );
                MembershipRemovalStatusV1::Removed
            }
            MembershipRemovalStatus::AlreadyRemoved => {
                bump(
                    &mut self.usage.documents_already_removed,
                    &mut self.usage.counter_saturated,
                );
                MembershipRemovalStatusV1::AlreadyRemoved
            }
        };
        if status == MembershipRemovalStatusV1::Removed {
            if self.identity_materialized {
                self.identity_generation += 1;
            }
            self.publish_change(WorkspaceChangeKindV1::DocumentRemoved(document));
        }
        add(
            &mut self.usage.witnesses_revoked_by_removal,
            witnesses_revoked,
            &mut self.usage.counter_saturated,
        );
        Ok(DocumentRemovalReceiptV1 { document, status })
    }

    fn remove_root(&mut self, root: WorkspaceRootIdV1) -> Result<RootRemovalReceiptV1, RefusalV1> {
        self.preflight_changes(1)?;
        bump(
            &mut self.usage.root_removal_requests,
            &mut self.usage.counter_saturated,
        );
        if root.workspace() != self.workspace_id {
            return Err(RefusalV1::UnknownRoot);
        }
        let native = self
            .root_projections
            .iter()
            .find(|projection| projection.protocol == root)
            .map(|projection| projection.native.clone())
            .ok_or(RefusalV1::UnknownRoot)?;
        let active = self
            .workspace
            .binding_inventory()
            .map_err(classify_workspace)?
            .roots
            .iter()
            .any(|binding| binding.root == native);
        if active && self.identity_materialized && self.identity_generation == u64::MAX {
            return Err(RefusalV1::CounterExhausted);
        }
        let status = self
            .workspace
            .remove_root(&native)
            .map_err(|error| match error {
                WorkspaceError::InvalidView => RefusalV1::UnknownRoot,
                other => classify_workspace(other),
            })?;
        let status = match status {
            MembershipRemovalStatus::Removed => {
                bump(
                    &mut self.usage.roots_removed,
                    &mut self.usage.counter_saturated,
                );
                MembershipRemovalStatusV1::Removed
            }
            MembershipRemovalStatus::AlreadyRemoved => {
                bump(
                    &mut self.usage.roots_already_removed,
                    &mut self.usage.counter_saturated,
                );
                MembershipRemovalStatusV1::AlreadyRemoved
            }
        };
        if status == MembershipRemovalStatusV1::Removed {
            if self.identity_materialized {
                self.identity_generation += 1;
            }
            self.publish_change(WorkspaceChangeKindV1::RootRemoved(root));
        }
        Ok(RootRemovalReceiptV1 { root, status })
    }

    fn select_document(
        &mut self,
        client: ClientId,
        document: WorkspaceDocumentIdV1,
    ) -> Result<WorkspaceDocumentIdV1, RefusalV1> {
        if document.workspace() != self.workspace_id {
            return Err(RefusalV1::UnknownDocument);
        }
        let native = self
            .document_projections
            .iter()
            .find(|projection| projection.protocol == document)
            .map(|projection| projection.native.clone())
            .ok_or(RefusalV1::UnknownDocument)?;
        let index = self
            .clients
            .iter()
            .position(|record| record.id == client)
            .ok_or(RefusalV1::UnknownClient)?;
        let mut view = match self.clients[index].view.clone() {
            Some(view) => view,
            None => self.workspace.open_view().map_err(classify_workspace)?,
        };
        self.workspace
            .select_document_id(&mut view, &native)
            .map_err(|_| RefusalV1::UnknownDocument)?;
        self.clients[index].view = Some(view);
        Ok(document)
    }

    fn validate_locator(&self, locator: &str) -> Result<(), RefusalV1> {
        if locator.is_empty() || locator.len() > self.limits.max_locator_bytes {
            return Err(RefusalV1::InputLimit);
        }
        Ok(())
    }

    fn replace(
        &mut self,
        client: ClientId,
        expected: VersionWitness,
        start: u64,
        end: u64,
        inserted: &str,
    ) -> Result<SnapshotV1, RefusalV1> {
        let view = self.view(client)?;
        self.preflight_changes(1)?;
        self.workspace_inventory()?;
        let active = view.active_document_id().clone();
        let document = self
            .document_projections
            .iter()
            .find(|projection| projection.native == active)
            .map(|projection| projection.protocol)
            .ok_or(RefusalV1::EditorUnavailable)?;
        if inserted.len() > self.limits.max_insert_bytes {
            return Err(RefusalV1::InputLimit);
        }
        let retained = self
            .witnesses
            .iter()
            .find(|entry| entry.client == client && entry.id == expected)
            .ok_or(RefusalV1::UnknownWitness)?;
        let expected_version = retained.version.clone();
        let original = Arc::clone(&retained.text);
        if self.workspace.version(&view).map_err(classify_workspace)? != expected_version {
            return Err(RefusalV1::Stale);
        }
        let start = usize::try_from(start).map_err(|_| RefusalV1::InvalidRange)?;
        let end = usize::try_from(end).map_err(|_| RefusalV1::InvalidRange)?;
        if start > end
            || end > original.len()
            || !original.is_char_boundary(start)
            || !original.is_char_boundary(end)
        {
            return Err(RefusalV1::InvalidRange);
        }
        let new_len = original
            .len()
            .checked_sub(end - start)
            .and_then(|length| length.checked_add(inserted.len()))
            .ok_or(RefusalV1::InputLimit)?;
        if new_len > self.limits.max_snapshot_bytes {
            return Err(RefusalV1::SnapshotLimit);
        }
        self.preflight_witness()?;

        let expected_fragment = copy_string(&original[start..end])?;
        let inserted_owned = copy_string(inserted)?;
        let mut projected = String::new();
        projected
            .try_reserve_exact(new_len)
            .map_err(|_| RefusalV1::EditorCapacity)?;
        projected.push_str(&original[..start]);
        projected.push_str(inserted);
        projected.push_str(&original[end..]);
        let projected: Arc<str> = Arc::from(projected);
        let mut replacements = Vec::new();
        replacements
            .try_reserve_exact(1)
            .map_err(|_| RefusalV1::EditorCapacity)?;
        replacements.push(Replacement {
            start,
            end,
            expected: expected_fragment,
            inserted: inserted_owned,
        });
        self.workspace
            .edit_snapshot(&view, &expected_version, replacements)
            .map_err(classify_workspace)?;
        let version = self.workspace.version(&view).map_err(classify_workspace)?;
        let source_revision = version.revision;
        let snapshot = self.install_witness(client, version, projected);
        self.publish_change(WorkspaceChangeKindV1::DocumentChanged {
            document,
            source_revision,
        });
        Ok(snapshot)
    }

    fn save(
        &mut self,
        client: ClientId,
        expected: VersionWitness,
    ) -> Result<SaveReceiptV1, RefusalV1> {
        let view = self.view(client)?;
        let retained = self
            .witnesses
            .iter()
            .find(|entry| entry.client == client && entry.id == expected)
            .ok_or(RefusalV1::UnknownWitness)?;
        let expected_version = retained.version.clone();
        if self.workspace.version(&view).map_err(classify_workspace)? != expected_version {
            return Err(RefusalV1::Stale);
        }
        let receipt = self
            .workspace
            .save_snapshot(&view, &expected_version)
            .map_err(classify_workspace)?;
        Ok(project_save_receipt(receipt))
    }

    fn replace_and_save(
        &mut self,
        client: ClientId,
        expected: VersionWitness,
        start: u64,
        end: u64,
        inserted: &str,
    ) -> Result<ReplaceAndSaveReceiptV1, RefusalV1> {
        bump(
            &mut self.usage.replace_and_save_requests,
            &mut self.usage.counter_saturated,
        );
        // Replacement retains all of its ordinary exact witness, range,
        // revision, capacity, and change-feed checks. Persistence begins only
        // after that edit and its new witness are published. A later save
        // refusal is therefore an accepted partial outcome, never a refusal
        // that erases the already-applied edit.
        let snapshot = self.replace(client, expected, start, end, inserted)?;
        let persistence = match self.save(client, snapshot.witness) {
            Ok(receipt) => classify_edit_persistence(receipt),
            Err(refusal) => EditPersistenceV1::NotAttempted(refusal),
        };
        match persistence {
            EditPersistenceV1::Durable(_) => bump(
                &mut self.usage.replace_and_save_durable,
                &mut self.usage.counter_saturated,
            ),
            EditPersistenceV1::NotDurable(_) => bump(
                &mut self.usage.replace_and_save_not_durable,
                &mut self.usage.counter_saturated,
            ),
            EditPersistenceV1::Unknown(_) => bump(
                &mut self.usage.replace_and_save_unknown,
                &mut self.usage.counter_saturated,
            ),
            EditPersistenceV1::NotAttempted(_) => bump(
                &mut self.usage.replace_and_save_not_attempted,
                &mut self.usage.counter_saturated,
            ),
        }
        Ok(ReplaceAndSaveReceiptV1 {
            snapshot,
            persistence,
        })
    }

    fn workspace_edit(
        &mut self,
        client: ClientId,
        documents: &[WorkspaceDocumentEditV1],
    ) -> Result<WorkspaceEditReceiptV1, RefusalV1> {
        if documents.is_empty() || documents.len() > MAX_WORKSPACE_EDIT_DOCUMENTS_V1 {
            return Err(RefusalV1::InputLimit);
        }
        self.preflight_changes(documents.len())?;
        self.preflight_witnesses(documents.len())?;

        let mut writes = Vec::new();
        let mut projections = Vec::new();
        let mut request_payload_bytes = 4usize;
        let mut receipt_payload_bytes = 20usize;
        writes
            .try_reserve_exact(documents.len())
            .map_err(|_| RefusalV1::EditorCapacity)?;
        projections
            .try_reserve_exact(documents.len())
            .map_err(|_| RefusalV1::EditorCapacity)?;

        for (index, requested) in documents.iter().enumerate() {
            if requested.document.workspace() != self.workspace_id
                || documents[..index]
                    .iter()
                    .any(|prior| prior.document == requested.document)
            {
                return Err(RefusalV1::UnknownDocument);
            }
            if requested.replacements.is_empty() {
                return Err(RefusalV1::InputLimit);
            }
            if requested.replacements.len() > MAX_WORKSPACE_EDIT_REPLACEMENTS_V1 {
                return Err(RefusalV1::InputLimit);
            }
            request_payload_bytes = request_payload_bytes
                .checked_add(28)
                .ok_or(RefusalV1::InputLimit)?;
            let native = self
                .document_projections
                .iter()
                .find(|projection| projection.protocol == requested.document)
                .map(|projection| projection.native.clone())
                .ok_or(RefusalV1::UnknownDocument)?;
            let captured = self
                .workspace
                .document_snapshot(&native)
                .map_err(classify_workspace)?;
            let version = captured.version;
            if version.revision != requested.expected_revision {
                return Err(RefusalV1::Stale);
            }
            if captured.text.len() > self.limits.max_snapshot_bytes {
                return Err(RefusalV1::SnapshotLimit);
            }
            let original: Arc<str> = Arc::from(captured.text);
            let mut replacements = Vec::new();
            replacements
                .try_reserve_exact(requested.replacements.len())
                .map_err(|_| RefusalV1::EditorCapacity)?;
            let mut inserted_bytes = 0usize;
            let mut removed_bytes = 0usize;
            let mut previous = None;
            for replacement in &requested.replacements {
                request_payload_bytes = request_payload_bytes
                    .checked_add(20)
                    .and_then(|value| value.checked_add(replacement.inserted.len()))
                    .filter(|value| *value <= MAX_WORKSPACE_EDIT_PAYLOAD_BYTES_V1)
                    .ok_or(RefusalV1::InputLimit)?;
                if replacement.inserted.len() > self.limits.max_insert_bytes {
                    return Err(RefusalV1::InputLimit);
                }
                let start =
                    usize::try_from(replacement.start).map_err(|_| RefusalV1::InvalidRange)?;
                let end = usize::try_from(replacement.end).map_err(|_| RefusalV1::InvalidRange)?;
                if start > end
                    || end > original.len()
                    || !original.is_char_boundary(start)
                    || !original.is_char_boundary(end)
                    || previous.is_some_and(|(prior_start, prior_end)| {
                        start < prior_end
                            || (start == prior_end && (prior_start == prior_end || start == end))
                    })
                {
                    return Err(RefusalV1::InvalidRange);
                }
                previous = Some((start, end));
                inserted_bytes = inserted_bytes
                    .checked_add(replacement.inserted.len())
                    .ok_or(RefusalV1::InputLimit)?;
                removed_bytes = removed_bytes
                    .checked_add(end - start)
                    .ok_or(RefusalV1::InputLimit)?;
                replacements.push(Replacement {
                    start,
                    end,
                    expected: copy_string(&original[start..end])?,
                    inserted: copy_string(&replacement.inserted)?,
                });
            }
            if inserted_bytes > self.limits.max_insert_bytes {
                return Err(RefusalV1::InputLimit);
            }
            let projected_len = original
                .len()
                .checked_sub(removed_bytes)
                .and_then(|length| length.checked_add(inserted_bytes))
                .ok_or(RefusalV1::InputLimit)?;
            if projected_len > self.limits.max_snapshot_bytes {
                return Err(RefusalV1::SnapshotLimit);
            }
            let mut projected = String::new();
            projected
                .try_reserve_exact(projected_len)
                .map_err(|_| RefusalV1::EditorCapacity)?;
            projected.push_str(&original);
            for replacement in replacements.iter().rev() {
                projected.replace_range(replacement.start..replacement.end, &replacement.inserted);
            }
            let changed = projected.as_str() != &*original;
            if changed {
                receipt_payload_bytes = receipt_payload_bytes
                    .checked_add(28)
                    .and_then(|value| value.checked_add(projected.len()))
                    .filter(|value| *value <= MAX_WORKSPACE_EDIT_PAYLOAD_BYTES_V1)
                    .ok_or(RefusalV1::SnapshotLimit)?;
            }
            projections.push((
                requested.document,
                native.clone(),
                Arc::<str>::from(projected),
                changed,
            ));
            writes.push(WorkspaceDocumentEdits {
                document: native,
                expected: version,
                edits: replacements,
            });
        }
        debug_assert!(request_payload_bytes <= MAX_WORKSPACE_EDIT_PAYLOAD_BYTES_V1);
        debug_assert!(receipt_payload_bytes <= MAX_WORKSPACE_EDIT_PAYLOAD_BYTES_V1);

        let publication = self
            .workspace
            .edit_document_batch(&writes)
            .map_err(classify_workspace)?;
        let mut edited = Vec::new();
        edited
            .try_reserve_exact(projections.len())
            .expect("preflight reserved receipt capacity");
        for (document, native, projected, changed) in projections {
            if !changed {
                continue;
            }
            let version = self
                .workspace
                .document_version(&native)
                .expect("committed document remains retained");
            let source_revision = version.revision;
            let snapshot = self.install_witness(client, version, projected);
            self.publish_change(WorkspaceChangeKindV1::DocumentChanged {
                document,
                source_revision,
            });
            edited.push(WorkspaceEditedDocumentV1 { document, snapshot });
        }
        debug_assert_eq!(edited.len(), publication.changed_documents);
        Ok(WorkspaceEditReceiptV1 {
            workspace: self.workspace_id,
            generation: publication.generation,
            documents: edited,
        })
    }

    fn preflight_witness(&self) -> Result<(), RefusalV1> {
        if self.witnesses.len() == self.limits.max_witnesses {
            return Err(RefusalV1::WitnessCapacity);
        }
        if self.next_witness == 0 {
            return Err(RefusalV1::CounterExhausted);
        }
        Ok(())
    }

    fn preflight_witnesses(&self, count: usize) -> Result<(), RefusalV1> {
        if self
            .witnesses
            .len()
            .checked_add(count)
            .is_none_or(|total| total > self.limits.max_witnesses)
        {
            return Err(RefusalV1::WitnessCapacity);
        }
        if !can_issue(self.next_witness, count) {
            return Err(RefusalV1::CounterExhausted);
        }
        Ok(())
    }

    fn install_witness(
        &mut self,
        client: ClientId,
        version: DocumentVersion,
        text: Arc<str>,
    ) -> SnapshotV1 {
        let witness = VersionWitness::new(self.next_witness).expect("preflight rejects zero");
        self.next_witness = self.next_witness.checked_add(1).unwrap_or(0);
        let snapshot = SnapshotV1 {
            witness,
            revision: version.revision,
            text: Arc::clone(&text),
        };
        self.witnesses.push(WitnessRecord {
            client,
            id: witness,
            version,
            text,
        });
        snapshot
    }

    fn view(&mut self, client: ClientId) -> Result<EditorView, RefusalV1> {
        let index = self
            .clients
            .iter()
            .position(|record| record.id == client)
            .ok_or(RefusalV1::UnknownClient)?;
        self.clients[index]
            .view
            .clone()
            .ok_or(RefusalV1::NoActiveDocument)
    }

    fn workspace_inventory(&mut self) -> Result<WorkspaceInventoryV1, RefusalV1> {
        let inventory = self.workspace.inventory().map_err(classify_workspace)?;
        if inventory.documents.len() > self.limits.max_inventory_documents
            || inventory.roots.len() > self.limits.max_inventory_roots
        {
            return Err(RefusalV1::EditorCapacity);
        }
        let missing_documents = inventory
            .documents
            .iter()
            .filter(|native| {
                !self
                    .document_projections
                    .iter()
                    .any(|projection| &projection.native == *native)
            })
            .count();
        let missing_roots = inventory
            .roots
            .iter()
            .filter(|root| {
                !self
                    .root_projections
                    .iter()
                    .any(|projection| projection.native == root.id)
            })
            .count();
        if !can_issue(self.next_document, missing_documents)
            || !can_issue(self.next_root, missing_roots)
        {
            return Err(RefusalV1::CounterExhausted);
        }
        let extends_materialized_identity =
            self.identity_materialized && (missing_documents != 0 || missing_roots != 0);
        if extends_materialized_identity && self.identity_generation == u64::MAX {
            return Err(RefusalV1::CounterExhausted);
        }
        let mut documents = Vec::new();
        documents
            .try_reserve_exact(inventory.documents.len())
            .map_err(|_| RefusalV1::EditorCapacity)?;
        let mut roots = Vec::new();
        roots
            .try_reserve_exact(inventory.roots.len())
            .map_err(|_| RefusalV1::EditorCapacity)?;
        for native in &inventory.documents {
            let protocol = match self
                .document_projections
                .iter()
                .find(|projection| projection.native == *native)
            {
                Some(projection) => projection.protocol,
                None => {
                    let protocol =
                        WorkspaceDocumentIdV1::new(self.workspace_id, self.next_document)
                            .ok_or(RefusalV1::CounterExhausted)?;
                    self.document_issued_through = self.next_document;
                    self.next_document = self.next_document.checked_add(1).unwrap_or(0);
                    self.document_projections.push(DocumentProjection {
                        native: native.clone(),
                        protocol,
                    });
                    protocol
                }
            };
            documents.push(protocol);
        }
        for root in &inventory.roots {
            let protocol = match self
                .root_projections
                .iter()
                .find(|projection| projection.native == root.id)
            {
                Some(projection) => projection.protocol,
                None => {
                    let protocol = WorkspaceRootIdV1::new(self.workspace_id, self.next_root)
                        .ok_or(RefusalV1::CounterExhausted)?;
                    self.root_issued_through = self.next_root;
                    self.next_root = self.next_root.checked_add(1).unwrap_or(0);
                    self.root_projections.push(RootProjection {
                        native: root.id.clone(),
                        protocol,
                    });
                    protocol
                }
            };
            roots.push(WorkspaceRootV1 {
                id: protocol,
                kind: match root.kind {
                    WorkspaceRootKind::File => WorkspaceRootKindV1::File,
                    WorkspaceRootKind::Directory => WorkspaceRootKindV1::Directory,
                },
            });
        }
        if extends_materialized_identity {
            self.identity_generation += 1;
        }
        self.identity_materialized = true;
        Ok(WorkspaceInventoryV1 {
            workspace: self.workspace_id,
            documents,
            roots,
        })
    }

    fn workspace_inventory_page(
        &mut self,
        cursor: Option<WorkspaceInventoryCursorV1>,
        page_size: u16,
    ) -> Result<WorkspaceInventoryPageV1, RefusalV1> {
        bump(
            &mut self.usage.inventory_page_requests,
            &mut self.usage.counter_saturated,
        );
        let requested = usize::from(page_size);
        if requested == 0 || requested > self.limits.max_inventory_page_size {
            bump(
                &mut self.usage.inventory_page_limited,
                &mut self.usage.counter_saturated,
            );
            return Err(RefusalV1::InputLimit);
        }
        let has_cursor = cursor.is_some();
        let (offset, expected_generation) = match cursor {
            None => {
                bump(
                    &mut self.usage.inventory_page_started,
                    &mut self.usage.counter_saturated,
                );
                (0, self.workspace.inventory_generation())
            }
            Some(cursor) => {
                bump(
                    &mut self.usage.inventory_page_continued,
                    &mut self.usage.counter_saturated,
                );
                if cursor.workspace() != self.workspace_id {
                    bump(
                        &mut self.usage.inventory_page_foreign,
                        &mut self.usage.counter_saturated,
                    );
                    return Err(RefusalV1::ForeignInventoryCursor);
                }
                if cursor.generation() != self.workspace.inventory_generation() {
                    bump(
                        &mut self.usage.inventory_page_stale,
                        &mut self.usage.counter_saturated,
                    );
                    return Err(RefusalV1::StaleInventoryCursor);
                }
                let offset = usize::try_from(cursor.offset()).map_err(|_| {
                    bump(
                        &mut self.usage.inventory_page_invalid,
                        &mut self.usage.counter_saturated,
                    );
                    RefusalV1::InvalidInventoryCursor
                })?;
                (offset, cursor.generation())
            }
        };
        let page = self
            .workspace
            .inventory_page(offset, requested)
            .map_err(|_| {
                bump(
                    &mut self.usage.inventory_page_invalid,
                    &mut self.usage.counter_saturated,
                );
                RefusalV1::InvalidInventoryCursor
            })?;
        // A serialized owner makes this equality structural, but retain the
        // explicit check so future concurrency cannot silently weaken it.
        if page.generation != expected_generation {
            bump(
                &mut self.usage.inventory_page_stale,
                &mut self.usage.counter_saturated,
            );
            return Err(RefusalV1::StaleInventoryCursor);
        }
        if has_cursor && offset >= page.total_entries {
            bump(
                &mut self.usage.inventory_page_invalid,
                &mut self.usage.counter_saturated,
            );
            return Err(RefusalV1::InvalidInventoryCursor);
        }
        let missing_documents = page
            .documents
            .iter()
            .filter(|native| {
                !self
                    .document_projections
                    .iter()
                    .any(|projection| &projection.native == *native)
            })
            .count();
        let missing_roots = page
            .roots
            .iter()
            .filter(|root| {
                !self
                    .root_projections
                    .iter()
                    .any(|projection| projection.native == root.id)
            })
            .count();
        if !can_issue(self.next_document, missing_documents)
            || !can_issue(self.next_root, missing_roots)
        {
            return Err(RefusalV1::CounterExhausted);
        }
        let extends_materialized_identity =
            self.identity_materialized && (missing_documents != 0 || missing_roots != 0);
        if extends_materialized_identity && self.identity_generation == u64::MAX {
            return Err(RefusalV1::CounterExhausted);
        }
        let mut documents = Vec::new();
        documents
            .try_reserve_exact(page.documents.len())
            .map_err(|_| RefusalV1::EditorCapacity)?;
        for native in &page.documents {
            let protocol = match self
                .document_projections
                .iter()
                .find(|projection| projection.native == *native)
            {
                Some(projection) => projection.protocol,
                None => {
                    let protocol =
                        WorkspaceDocumentIdV1::new(self.workspace_id, self.next_document)
                            .ok_or(RefusalV1::CounterExhausted)?;
                    self.document_issued_through = self.next_document;
                    self.next_document = self.next_document.checked_add(1).unwrap_or(0);
                    self.document_projections.push(DocumentProjection {
                        native: native.clone(),
                        protocol,
                    });
                    protocol
                }
            };
            documents.push(protocol);
        }
        let mut roots = Vec::new();
        roots
            .try_reserve_exact(page.roots.len())
            .map_err(|_| RefusalV1::EditorCapacity)?;
        for root in &page.roots {
            let protocol = match self
                .root_projections
                .iter()
                .find(|projection| projection.native == root.id)
            {
                Some(projection) => projection.protocol,
                None => {
                    let protocol = WorkspaceRootIdV1::new(self.workspace_id, self.next_root)
                        .ok_or(RefusalV1::CounterExhausted)?;
                    self.root_issued_through = self.next_root;
                    self.next_root = self.next_root.checked_add(1).unwrap_or(0);
                    self.root_projections.push(RootProjection {
                        native: root.id.clone(),
                        protocol,
                    });
                    protocol
                }
            };
            roots.push(WorkspaceRootV1 {
                id: protocol,
                kind: match root.kind {
                    WorkspaceRootKind::File => WorkspaceRootKindV1::File,
                    WorkspaceRootKind::Directory => WorkspaceRootKindV1::Directory,
                },
            });
        }
        let returned = documents
            .len()
            .checked_add(roots.len())
            .ok_or(RefusalV1::EditorCapacity)?;
        let next_offset = offset
            .checked_add(returned)
            .ok_or(RefusalV1::InvalidInventoryCursor)?;
        let next = if next_offset < page.total_entries {
            Some(
                WorkspaceInventoryCursorV1::new(
                    self.workspace_id,
                    page.generation,
                    u64::try_from(next_offset).map_err(|_| RefusalV1::CounterExhausted)?,
                )
                .ok_or(RefusalV1::CounterExhausted)?,
            )
        } else {
            bump(
                &mut self.usage.inventory_page_completed,
                &mut self.usage.counter_saturated,
            );
            None
        };
        if extends_materialized_identity {
            self.identity_generation += 1;
        }
        self.identity_materialized = true;
        Ok(WorkspaceInventoryPageV1 {
            workspace: self.workspace_id,
            documents,
            roots,
            next,
        })
    }

    fn workspace_changes(
        &mut self,
        after: Option<WorkspaceChangeCursorV1>,
        page_size: u16,
    ) -> Result<WorkspaceChangesV1, RefusalV1> {
        bump(
            &mut self.usage.change_feed_requests,
            &mut self.usage.counter_saturated,
        );
        let requested = usize::from(page_size);
        if requested == 0 || requested > self.limits.max_change_page_size {
            return Err(RefusalV1::InputLimit);
        }
        let requested_after = match after {
            Some(cursor)
                if cursor.workspace() != self.workspace_id
                    || cursor.epoch() != self.change_epoch =>
            {
                return Err(RefusalV1::ForeignChangeCursor);
            }
            Some(cursor) => cursor.revision(),
            None => self
                .changes
                .front()
                .map_or(self.next_change_revision.saturating_sub(1), |change| {
                    change.revision().saturating_sub(1)
                }),
        };
        let observed_head = self.next_change_revision.saturating_sub(1);
        if requested_after > observed_head {
            return Err(RefusalV1::InvalidChangeCursor);
        }
        let oldest_available = self
            .changes
            .front()
            .map_or(self.next_change_revision, |change| change.revision());
        if requested_after.saturating_add(1) < oldest_available {
            bump(
                &mut self.usage.change_feed_gaps,
                &mut self.usage.counter_saturated,
            );
            return Ok(WorkspaceChangesV1::Gap(
                WorkspaceChangeGapV1::new(
                    self.workspace_id,
                    self.change_epoch,
                    requested_after,
                    oldest_available,
                    observed_head,
                )
                .expect("service change epoch is nonzero"),
            ));
        }
        let mut changes = Vec::new();
        changes
            .try_reserve_exact(requested)
            .map_err(|_| RefusalV1::EditorCapacity)?;
        changes.extend(
            self.changes
                .iter()
                .copied()
                .filter(|change| change.revision() > requested_after)
                .take(requested),
        );
        let next_revision = changes
            .last()
            .map_or(requested_after, |change| change.revision());
        bump(
            &mut self.usage.change_feed_pages,
            &mut self.usage.counter_saturated,
        );
        Ok(WorkspaceChangesV1::Page(WorkspaceChangePageV1 {
            workspace: self.workspace_id,
            observed_head,
            changes,
            next: WorkspaceChangeCursorV1::new(self.workspace_id, self.change_epoch, next_revision)
                .expect("service change epoch is nonzero"),
        }))
    }

    fn preflight_changes(&self, maximum_new: usize) -> Result<(), RefusalV1> {
        let maximum_new = u64::try_from(maximum_new).map_err(|_| RefusalV1::CounterExhausted)?;
        self.next_change_revision
            .checked_add(maximum_new)
            .ok_or(RefusalV1::CounterExhausted)
            .map(|_| ())
    }

    fn publish_change(&mut self, kind: WorkspaceChangeKindV1) {
        let change = WorkspaceChangeV1::new(self.next_change_revision, kind)
            .expect("change preflight rejects exhausted revisions");
        self.next_change_revision = self
            .next_change_revision
            .checked_add(1)
            .expect("change preflight reserves revisions");
        if self.changes.len() == self.limits.max_change_events {
            self.changes.pop_front();
            bump(
                &mut self.usage.change_feed_events_evicted,
                &mut self.usage.counter_saturated,
            );
        }
        self.changes.push_back(change);
        bump(
            &mut self.usage.change_feed_events_published,
            &mut self.usage.counter_saturated,
        );
    }
}

fn copy_string(value: &str) -> Result<String, RefusalV1> {
    let mut copy = String::new();
    copy.try_reserve_exact(value.len())
        .map_err(|_| RefusalV1::EditorCapacity)?;
    copy.push_str(value);
    Ok(copy)
}

fn project_save_receipt(receipt: WorkspaceSaveReceipt) -> SaveReceiptV1 {
    SaveReceiptV1 {
        application: match receipt.application {
            Application::Unknown => SaveApplicationV1::Unknown,
            Application::NotWritten => SaveApplicationV1::NotWritten,
            Application::Written => SaveApplicationV1::Written,
        },
        synchronization: match receipt.synchronization {
            Synchronization::Unknown => SaveSynchronizationV1::Unknown,
            Synchronization::NotRequested => SaveSynchronizationV1::NotRequested,
            Synchronization::Succeeded => SaveSynchronizationV1::Succeeded,
            Synchronization::Failed => SaveSynchronizationV1::Failed,
        },
        no_future_effects: receipt.no_future_effects,
        quiescent: receipt.quiescent,
        adapter_failure: receipt.adapter_failure,
        cleanup_failed: receipt.cleanup_failed,
        baseline_published: receipt.baseline_published,
        outcome_unknown: receipt.outcome_unknown,
    }
}

fn classify_edit_persistence(receipt: SaveReceiptV1) -> EditPersistenceV1 {
    if receipt.outcome_unknown
        || receipt.application == SaveApplicationV1::Unknown
        || !receipt.no_future_effects
        || !receipt.quiescent
    {
        EditPersistenceV1::Unknown(receipt)
    } else if receipt.application == SaveApplicationV1::Written
        && receipt.synchronization == SaveSynchronizationV1::Succeeded
        && !receipt.adapter_failure
        && !receipt.cleanup_failed
        && receipt.baseline_published
    {
        EditPersistenceV1::Durable(receipt)
    } else {
        EditPersistenceV1::NotDurable(receipt)
    }
}

fn can_issue(next: u64, count: usize) -> bool {
    if count == 0 {
        return true;
    }
    let Some(last_offset) = count
        .checked_sub(1)
        .and_then(|offset| u64::try_from(offset).ok())
    else {
        return false;
    };
    next != 0 && next.checked_add(last_offset).is_some()
}

fn fixed_authority_scope(workspace_scope: u64) -> (AuthorityContextV1, AuthorityResourceRefV1) {
    let mut subject = [0_u8; 16];
    subject[..8].copy_from_slice(b"vellum-s");
    subject[8..].copy_from_slice(&workspace_scope.to_be_bytes());
    let mut session = [0_u8; 16];
    session[..8].copy_from_slice(b"vellum-c");
    session[8..].copy_from_slice(&workspace_scope.to_be_bytes());
    let mut resource = [0_u8; 16];
    resource[..8].copy_from_slice(b"vellum-r");
    resource[8..].copy_from_slice(&workspace_scope.to_be_bytes());
    (
        AuthorityContextV1 {
            subject: AuthoritySubjectRefV1::new(subject),
            session: Some(AuthoritySessionRefV1::new(session)),
        },
        AuthorityResourceRefV1::new(resource),
    )
}

const MAX_ACTIONS_PER_COMMAND: usize = 2;

#[derive(Clone, Copy)]
struct AuthorityActionPlan {
    actions: [Option<AuthorityActionV1>; MAX_ACTIONS_PER_COMMAND],
}

const fn action_plan(command: &CommandV1) -> AuthorityActionPlan {
    let action = match command {
        CommandV1::WorkspaceInventory
        | CommandV1::WorkspaceInventoryPage { .. }
        | CommandV1::WorkspaceChanges { .. } => AuthorityActionV1::InspectWorkspace,
        CommandV1::OpenView | CommandV1::CloseView => AuthorityActionV1::ManageView,
        CommandV1::AddFile { .. } => AuthorityActionV1::AdmitFile,
        CommandV1::AddDirectory { .. } => AuthorityActionV1::DiscoverDirectory,
        CommandV1::RemoveDocument { .. } | CommandV1::RemoveRoot { .. } => {
            AuthorityActionV1::RetireMembership
        }
        CommandV1::SelectDocument { .. } => AuthorityActionV1::SelectDocument,
        CommandV1::Save { .. } => AuthorityActionV1::PersistSource,
        CommandV1::ReplaceAndSave { .. } => {
            return AuthorityActionPlan {
                actions: [
                    Some(AuthorityActionV1::EditSource),
                    Some(AuthorityActionV1::PersistSource),
                ],
            };
        }
        CommandV1::Snapshot => AuthorityActionV1::ReadSource,
        CommandV1::Replace { .. } | CommandV1::WorkspaceEdit { .. } => {
            AuthorityActionV1::EditSource
        }
    };
    AuthorityActionPlan {
        actions: [Some(action), None],
    }
}

const fn durable_action_plan(intent: &DurableEffectIntentV1) -> AuthorityActionPlan {
    let action = match intent {
        DurableEffectIntentV1::AddFile { .. } => AuthorityActionV1::AdmitFile,
        DurableEffectIntentV1::AddDirectory { .. } => AuthorityActionV1::DiscoverDirectory,
        DurableEffectIntentV1::RemoveDocument { .. } | DurableEffectIntentV1::RemoveRoot { .. } => {
            AuthorityActionV1::RetireMembership
        }
        DurableEffectIntentV1::SaveDocument { .. } => AuthorityActionV1::PersistSource,
        DurableEffectIntentV1::ReplaceDocumentAndSave { .. } => {
            return AuthorityActionPlan {
                actions: [
                    Some(AuthorityActionV1::EditSource),
                    Some(AuthorityActionV1::PersistSource),
                ],
            };
        }
        DurableEffectIntentV1::ExportDocumentCopyExclusive { .. } => {
            AuthorityActionV1::ExportSourceCopy
        }
    };
    AuthorityActionPlan {
        actions: [Some(action), None],
    }
}

fn project_export_copy_receipt(
    document: WorkspaceDocumentIdV1,
    receipt: ExportCopyReceiptV1,
) -> Result<DurableExportCopyReceiptV1, RefusalV1> {
    let persistence = match receipt.persistence() {
        ExportCopyPersistenceV1::Durable => DurableExportCopyPersistenceV1::Durable,
        ExportCopyPersistenceV1::NotDurable => DurableExportCopyPersistenceV1::NotDurable,
        ExportCopyPersistenceV1::Unknown => DurableExportCopyPersistenceV1::Unknown,
        ExportCopyPersistenceV1::NotAttempted => DurableExportCopyPersistenceV1::NotAttempted,
    };
    let failure = receipt.failure().map(|failure| match failure {
        ExportCopyFailureV1::Conflict => DurableExportCopyFailureV1::Conflict,
        ExportCopyFailureV1::Unsupported => DurableExportCopyFailureV1::Unsupported,
        ExportCopyFailureV1::Limit => DurableExportCopyFailureV1::Limit,
        ExportCopyFailureV1::Capacity => DurableExportCopyFailureV1::Capacity,
        ExportCopyFailureV1::Io => DurableExportCopyFailureV1::Io,
    });
    DurableExportCopyReceiptV1::try_new(
        document,
        receipt.source_revision(),
        persistence,
        failure,
        receipt.cleanup_failed(),
    )
    .ok_or(RefusalV1::EditorUnavailable)
}

fn classify_workspace(error: WorkspaceError) -> RefusalV1 {
    match error {
        WorkspaceError::Capacity => RefusalV1::EditorCapacity,
        WorkspaceError::Empty => RefusalV1::NoActiveDocument,
        WorkspaceError::InvalidView | WorkspaceError::Discovery(_) | WorkspaceError::Session(_) => {
            RefusalV1::EditorUnavailable
        }
    }
}

fn observe_request_retention(usage: &mut ServiceUsage, outcome: RequestRetentionOutcome) {
    let counter = match outcome {
        RequestRetentionOutcome::New => &mut usage.request_new,
        RequestRetentionOutcome::MatchingReplay => &mut usage.request_matching_replay,
        RequestRetentionOutcome::ChangedInputConflict => &mut usage.request_changed_input,
        RequestRetentionOutcome::CapacityFull => &mut usage.request_capacity_full,
    };
    bump(counter, &mut usage.counter_saturated);
    if outcome == RequestRetentionOutcome::MatchingReplay {
        bump(&mut usage.replayed, &mut usage.counter_saturated);
    }
}

fn project_request_decision(decision: ReplayDecision<&ReplyV1>) -> RequestRetentionDecision {
    match decision {
        ReplayDecision::New => RequestRetentionDecision::New,
        ReplayDecision::MatchingReplay(reply) => {
            RequestRetentionDecision::MatchingReplay(reply.clone())
        }
        ReplayDecision::Conflict => RequestRetentionDecision::ChangedInputConflict,
        ReplayDecision::Full => RequestRetentionDecision::CapacityFull,
    }
}

fn bump(value: &mut u64, saturated: &mut bool) {
    let next = value.saturating_add(1);
    if next == *value {
        *saturated = true;
    }
    *value = next;
}

fn add(value: &mut u64, amount: u64, saturated: &mut bool) {
    let next = value.saturating_add(amount);
    if amount != 0 && next == *value {
        *saturated = true;
    }
    *value = next;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_observation_saturates_without_wrapping_or_exposing_content() {
        let mut usage = ServiceUsage {
            request_new: u64::MAX,
            ..ServiceUsage::default()
        };

        observe_request_retention(&mut usage, RequestRetentionOutcome::New);

        assert_eq!(usage.request_new, u64::MAX);
        assert!(usage.counter_saturated);
        assert_eq!(format!("{:?}", RequestRetentionOutcome::New), "New");
    }

    #[test]
    fn discovery_failures_are_content_safe_service_unavailability() {
        assert_eq!(
            classify_workspace(WorkspaceError::Discovery(
                vellum_editor::DirectoryDiscoveryRefusal::Io,
            )),
            RefusalV1::EditorUnavailable
        );
    }

    #[test]
    fn projection_counter_preflight_rejects_the_whole_batch_before_exhaustion() {
        assert!(can_issue(u64::MAX, 1));
        assert!(!can_issue(u64::MAX, 2));
        assert!(!can_issue(0, 1));
        assert!(can_issue(0, 0));
    }

    #[test]
    fn keystone_replay_outcomes_preserve_the_service_counter_projection() {
        let reply = ReplyV1::Refused(RefusalV1::Stale);
        for (decision, expected) in [
            (ReplayDecision::New, RequestRetentionOutcome::New),
            (
                ReplayDecision::MatchingReplay(&reply),
                RequestRetentionOutcome::MatchingReplay,
            ),
            (
                ReplayDecision::Conflict,
                RequestRetentionOutcome::ChangedInputConflict,
            ),
            (ReplayDecision::Full, RequestRetentionOutcome::CapacityFull),
        ] {
            assert_eq!(project_request_decision(decision).outcome(), expected);
        }
    }

    #[test]
    fn replace_and_save_classification_preserves_adapter_indeterminacy_and_definitive_nonwrite() {
        let unknown = SaveReceiptV1 {
            application: SaveApplicationV1::Unknown,
            synchronization: SaveSynchronizationV1::Unknown,
            no_future_effects: true,
            quiescent: true,
            adapter_failure: true,
            cleanup_failed: false,
            baseline_published: false,
            outcome_unknown: false,
        };
        assert_eq!(
            classify_edit_persistence(unknown),
            EditPersistenceV1::Unknown(unknown)
        );

        let external_change = SaveReceiptV1 {
            application: SaveApplicationV1::NotWritten,
            synchronization: SaveSynchronizationV1::Unknown,
            no_future_effects: true,
            quiescent: true,
            adapter_failure: false,
            cleanup_failed: false,
            baseline_published: false,
            outcome_unknown: false,
        };
        assert_eq!(
            classify_edit_persistence(external_change),
            EditPersistenceV1::NotDurable(external_change)
        );
    }
}
