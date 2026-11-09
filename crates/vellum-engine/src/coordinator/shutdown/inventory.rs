//! Bounded host-held inspection metadata. No body, effect lease, producer pin,
//! shutdown acceptance or authority grant is created by inspecting an owner.
use super::*;
use crate::resource::{
    Application, ArtifactKind, BaselineCapacityState, ResourceArtifactId, SaveId, SavePhase,
    SavePurpose, Synchronization,
};
mod progress;
#[cfg(test)]
mod tests;
pub use progress::*;
mod pools;
mod reconciliation;
pub(in crate::coordinator) use pools::LocalOwnerSnapshot;
pub(in crate::coordinator) use pools::PoolSnapshot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShutdownInventoryLimitsV1 {
    pub max_documents: usize,
    pub max_writes: usize,
    pub max_artifacts: usize,
    pub max_bindings: usize,
    pub max_reads: usize,
    pub max_owners: usize,
    pub max_attachments: usize,
    pub max_dependencies: usize,
    /// Projection visits: issued slots (including tombstones) and dependencies.
    /// Existing native integrity guards retain their own configured bounds.
    pub max_work: usize,
    pub max_capture_bytes: usize,
}
pub struct TrustedShutdownInventoryFactsV1<'a> {
    pub control: &'a TrustedShutdownFenceFactsV1<'a>,
    /// Control authority alone does not authorize inventory disclosure.
    pub inventory_readable: Option<bool>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShutdownDocumentRecordV1 {
    pub source: DocumentVersion,
    pub source_binding_generation: Option<u64>,
    pub annotation_binding_generation: Option<u64>,
    pub lifecycle: LifecycleState,
    pub source_policy_generation: u64,
    pub source_read_only: bool,
    pub serialization: SourceSerializationStateV1,
    pub annotation_generation: u64,
    pub annotation_policy_generation: u64,
    pub annotation_read_only: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShutdownWriteRecordV1 {
    pub write: SaveId,
    pub artifact: ResourceArtifactId,
    pub source: DocumentVersion,
    pub kind: ArtifactKind,
    pub purpose: SavePurpose,
    pub binding_generation: u64,
    pub annotation_generation: Option<u64>,
    pub serialization: Option<SourceSerializationStateV1>,
    pub phase: SavePhase,
    pub application: Application,
    pub observed_bytes: usize,
    pub version_available: bool,
    pub epoch_available: bool,
    pub causal_edge_available: bool,
    pub retained_evidence_witnesses: usize,
    /// Includes the owner's private reference; not effect/worker completion.
    pub dispatch_evidence_references: usize,
    pub synchronization: Synchronization,
    pub synchronization_required: bool,
    pub no_future_effects: bool,
    pub quiescent: bool,
    pub contested: bool,
    pub stop_requested: bool,
    pub baseline_capacity: Option<BaselineCapacityState>,
    pub baseline_envelope_bytes: Option<usize>,
    pub dispatch_serial: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShutdownArtifactRecordV1 {
    pub artifact: ResourceArtifactId,
    pub source: DocumentVersion,
    pub kind: ArtifactKind,
    pub stopped: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShutdownBindingIdV1 {
    pub(in crate::coordinator) document: DocumentId,
    pub(in crate::coordinator) kind: ArtifactKind,
    pub(in crate::coordinator) generation: u64,
}
#[derive(Clone, PartialEq, Eq)]
pub struct ShutdownDestinationIdV1(pub(in crate::coordinator) crate::resource::Identity);
impl std::fmt::Debug for ShutdownDestinationIdV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ShutdownDestinationIdV1(<opaque>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownBindingLocationV1 {
    /// Current resource association registry; document editing may be disposed.
    Active,
    DisposedRetained,
    AssociationHistory,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShutdownBaselineOriginV1 {
    Absent,
    /// A source baseline without a native writer anchor. This does not assert
    /// that no reader/provenance obligation exists elsewhere in the engine.
    Unanchored,
    Write(SaveId),
    AnnotationRead(crate::resource::AnnotationReadId),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShutdownBindingRecordV1 {
    pub binding: ShutdownBindingIdV1,
    pub document: DocumentId,
    pub kind: ArtifactKind,
    pub generation: u64,
    pub location: ShutdownBindingLocationV1,
    pub destination: ShutdownDestinationIdV1,
    pub provider: crate::resource::ReadProviderContext,
    pub baseline: ShutdownBaselineOriginV1,
    pub baseline_bytes: usize,
    pub version_available: bool,
    pub epoch_available: bool,
    pub contested_destination: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownOwnerRoleV1 {
    SharedWriter,
    SourceReader,
    AnnotationReader,
    SourceContexts,
    AnnotationPreparations,
    AnnotationContexts,
    VersionSetPreparations,
    VersionSetContexts,
    ManagedSearch,
    SettingsRetention,
    SerializationBases,
    OperationObservers,
    SourcePublication,
    FacetPublication,
    CollectionPublication,
    RegistryObservation,
    RegistrationBases,
    Discovery,
    HistoryInspection,
    PublicationMeasurements,
    ReleaseMeasurements,
    InvocationMeasurements,
    ShutdownControl,
    Invocations,
    History,
    SearchSessions,
    Annotations,
    ResourceMetadata,
    OwnedSaves,
    OwnedSourceReads,
    OwnedAnnotationReads,
    SourceContinuity,
    SemanticRoot,
    LogicalViews,
    Workspaces,
    BoundaryEvidence,
    SettingsPreparations,
    InvocationObservers,
    SettingsReader,
    ResourceCatalogReads,
    DiagnosticCapture,
}
const SHUTDOWN_OWNER_COUNT: u32 = ShutdownOwnerRoleV1::DiagnosticCapture as u32 + 1;
#[derive(Clone, PartialEq, Eq)]
pub struct ShutdownOwnerIdV1 {
    pub(in crate::coordinator) owner: Arc<()>,
    pub(in crate::coordinator) role: ShutdownOwnerRoleV1,
}
impl ShutdownOwnerIdV1 {
    pub fn role(&self) -> ShutdownOwnerRoleV1 {
        self.role
    }
}
impl std::fmt::Debug for ShutdownOwnerIdV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ShutdownOwnerIdV1(<opaque>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownOwnerStateV1 {
    Unconfigured,
    Active,
    Closed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShutdownOwnerRecordV1 {
    pub owner: ShutdownOwnerIdV1,
    pub state: ShutdownOwnerStateV1,
    pub admission_stopped: bool,
    pub authority_fenced: bool,
    /// Closed native owners need not retain historical counter values. None is
    /// not zero and cannot be used as proof of an empty previously issued table.
    /// This is the primary owner's counter, not a sum of independent child issuers.
    pub issued_slots: Option<usize>,
    /// Retained entries in the represented family and inventoried child stores,
    /// including stopped records/helper handles;
    /// neither active workers nor current body availability.
    pub retained_records: Option<usize>,
    /// Actual native owner-store reservation, excluding separately funded
    /// operation registries. None does not assert zero parent obligations.
    pub reserved_bytes: Option<usize>,
    /// Separately funded inventoried descendants; None is not zero.
    pub additional_reserved_bytes: Option<usize>,
    /// Actual native local accounting credit, never a parent reservation or RSS.
    pub local_reserved_bytes: Option<usize>,
    /// A local ceiling is not an aggregate parent reservation.
    pub local_byte_limit: Option<usize>,
    pub operation_registry_state: Option<ShutdownOwnerStateV1>,
    pub held_bytes: Option<usize>,
    pub retirement_supported: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShutdownReadIdV1 {
    Source(crate::resource::SourceReadIdV1),
    Annotation(crate::resource::AnnotationReadId),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownReadTerminalV1 {
    Pending,
    End,
    Failure,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShutdownReadRecordV1 {
    pub read: ShutdownReadIdV1,
    pub destination: ShutdownDestinationIdV1,
    pub provider: crate::resource::ReadProviderContext,
    pub source: Option<DocumentVersion>,
    pub annotation_generation: Option<u64>,
    pub phase: crate::resource::ReadPhase,
    pub binding_generation: Option<u64>,
    pub source_binding_generation: Option<u64>,
    pub terminal: ShutdownReadTerminalV1,
    pub stopped: bool,
    pub quiescent: bool,
    pub contested: bool,
    pub refused: bool,
    pub candidate_ready: bool,
    pub received_bytes: usize,
    pub received_chunks: usize,
    pub witness_count: usize,
    pub dispatch_evidence_references: usize,
    pub capture_references: usize,
    pub held_bytes: usize,
    pub restoration_bytes: usize,
    pub restored_annotation_generation: Option<u64>,
    pub loaded_baseline_adopted: bool,
}
#[derive(Clone, PartialEq, Eq)]
pub struct ShutdownReadWriterAttachmentIdV1 {
    pub(in crate::coordinator) owner: Arc<()>,
    pub(in crate::coordinator) generation: u64,
}
impl std::fmt::Debug for ShutdownReadWriterAttachmentIdV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ShutdownReadWriterAttachmentIdV1(<opaque>)")
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShutdownReadWriterAttachmentV1 {
    pub attachment: ShutdownReadWriterAttachmentIdV1,
    pub reader: ShutdownOwnerIdV1,
    pub writer: ShutdownOwnerIdV1,
    pub reader_endpoint_bytes: usize,
    pub writer_endpoint_bytes: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShutdownInventoryTargetV1 {
    Document(DocumentId),
    Write(SaveId),
    Artifact(ResourceArtifactId),
    Binding(ShutdownBindingIdV1),
    AnnotationRead(crate::resource::AnnotationReadId),
    SourceRead(crate::resource::SourceReadIdV1),
    Owner(ShutdownOwnerIdV1),
    ReadWriterAttachment(ShutdownReadWriterAttachmentIdV1),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShutdownDependencyV1 {
    /// Retained dependent -> original required owner, never teardown order.
    pub dependent: ShutdownInventoryTargetV1,
    pub required: ShutdownInventoryTargetV1,
}
/// This projection is intentionally explicit about coverage. Other owners are
/// not inferred empty from an empty writer table. No acceptance API consumes a
/// partial projection as proof of complete shutdown readiness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShutdownInventoryCoverageV1 {
    pub documents: bool,
    pub writer_tasks: bool,
    pub artifacts: bool,
    pub bindings: bool,
    pub reader_tasks: bool,
    pub all_owner_obligations: bool,
}
/// Trusted-owner ledger metadata, not an original outcome or recipient grant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShutdownInvocationRecordV1 {
    pub invocation: InvocationId,
    pub state: Result<InvocationState, InvocationError>,
    pub stop_requested: bool,
    /// Includes nonrenewing retired intent markers, not necessarily input bodies.
    pub intent_bound: bool,
    pub outcome_available: bool,
    /// Actual separately funded native holders; forgetting an outcome is not release.
    pub retained_holds: usize,
    pub reserved_bytes: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShutdownObservationContextRecordV1 {
    pub context: InvocationObservationContextIdV1,
    pub invocation: InvocationId,
    pub reserved_bytes: usize,
    /// Native host-context object retention only, not grant, delivery or worker truth.
    pub host_handle_retained: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShutdownFundingV1 {
    pub parent_reserved_bytes: usize,
    pub inventoried_parent_bytes: usize,
    /// Unexplained credit prevents complete coverage; never inferred discharge.
    pub unrepresented_parent_bytes: usize,
}
pub(super) struct Cut {
    pub(super) funding: ShutdownFundingV1,
    pub(super) owner_coverage: bool,
    pub(super) observation_contexts: Vec<ShutdownObservationContextRecordV1>,
    pub(super) invocations: Vec<ShutdownInvocationRecordV1>,
    pub(super) documents: Vec<ShutdownDocumentRecordV1>,
    pub(super) writes: Vec<ShutdownWriteRecordV1>,
    pub(super) artifacts: Vec<ShutdownArtifactRecordV1>,
    pub(super) bindings: Vec<ShutdownBindingRecordV1>,
    pub(super) reads: Vec<ShutdownReadRecordV1>,
    pub(super) owners: Vec<ShutdownOwnerRecordV1>,
    pub(super) read_writer_attachments: Vec<ShutdownReadWriterAttachmentV1>,
    pub(super) dependencies: Vec<ShutdownDependencyV1>,
}
/// An immutable historical inspection, not an engine-owned subscription. Copies
/// retain only bounded metadata/opaque IDs, not source/annotation/file payloads.
pub struct ShutdownInventoryV1 {
    pub(super) marker: Arc<()>,
    pub(super) basis: Arc<Basis>,
    limits: ShutdownInventoryLimitsV1,
    cut: Arc<Cut>,
}
impl std::fmt::Debug for ShutdownInventoryV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ShutdownInventoryV1(<opaque>)")
    }
}
impl ShutdownInventoryV1 {
    pub(super) fn retain_metadata(&self) -> Self {
        Self {
            marker: self.marker.clone(),
            basis: self.basis.clone(),
            limits: self.limits,
            cut: self.cut.clone(),
        }
    }
    pub(super) fn capture_ceiling(&self) -> usize {
        self.limits.max_capture_bytes
    }
}
pub struct ShutdownInventoryReadV1<'a> {
    pub funding: ShutdownFundingV1,
    pub observation_contexts: &'a [ShutdownObservationContextRecordV1],
    /// Bounded by projection work, capture bytes and the original ledger profile.
    pub invocations: &'a [ShutdownInvocationRecordV1],
    pub documents: &'a [ShutdownDocumentRecordV1],
    pub writes: &'a [ShutdownWriteRecordV1],
    pub artifacts: &'a [ShutdownArtifactRecordV1],
    pub bindings: &'a [ShutdownBindingRecordV1],
    pub reads: &'a [ShutdownReadRecordV1],
    pub owners: &'a [ShutdownOwnerRecordV1],
    pub read_writer_attachments: &'a [ShutdownReadWriterAttachmentV1],
    pub dependencies: &'a [ShutdownDependencyV1],
    pub coverage: ShutdownInventoryCoverageV1,
}
pub(in crate::coordinator) struct Builder {
    limits: ShutdownInventoryLimitsV1,
    cut: Cut,
    work: usize,
    bytes: usize,
    owner_namespace: Option<Arc<()>>,
    owner_mask: u64,
    parent_bytes: usize,
}
impl Builder {
    pub(in crate::coordinator) fn observation_context(
        &mut self,
        row: ShutdownObservationContextRecordV1,
    ) -> Result<(), CoordinatorError> {
        self.step()?;
        self.charge::<ShutdownObservationContextRecordV1>(
            self.cut.observation_contexts.len(),
            self.limits.max_work,
        )?;
        self.cut
            .observation_contexts
            .try_reserve(1)
            .map_err(|_| CoordinatorError::Capacity)?;
        self.cut.observation_contexts.push(row);
        Ok(())
    }
    /// Query-local visited slots, bounded before allocation and traversal.
    pub(in crate::coordinator) fn hold_bitmap(
        &mut self,
        slots: usize,
    ) -> Result<Vec<u8>, CoordinatorError> {
        self.work = self
            .work
            .checked_add(slots)
            .filter(|n| *n <= self.limits.max_work)
            .ok_or(CoordinatorError::Limit)?;
        self.bytes = self
            .bytes
            .checked_add(slots)
            .filter(|n| *n <= self.limits.max_capture_bytes)
            .ok_or(CoordinatorError::Limit)?;
        let mut seen = Vec::new();
        seen.try_reserve_exact(slots)
            .map_err(|_| CoordinatorError::Capacity)?;
        seen.resize(slots, 0);
        Ok(seen)
    }
    pub(in crate::coordinator) fn invocation(
        &mut self,
        row: ShutdownInvocationRecordV1,
    ) -> Result<(), CoordinatorError> {
        self.step()?;
        self.charge::<ShutdownInvocationRecordV1>(
            self.cut.invocations.len(),
            self.limits.max_work,
        )?;
        self.cut
            .invocations
            .try_reserve(1)
            .map_err(|_| CoordinatorError::Capacity)?;
        self.cut.invocations.push(row);
        Ok(())
    }
    fn new(limits: ShutdownInventoryLimitsV1) -> Result<Self, CoordinatorError> {
        let bytes = size_of::<ShutdownInventoryV1>()
            .checked_add(size_of::<Cut>())
            .ok_or(CoordinatorError::Limit)?;
        if bytes > limits.max_capture_bytes {
            return Err(CoordinatorError::Limit);
        }
        Ok(Self {
            limits,
            cut: Cut {
                funding: ShutdownFundingV1::default(),
                owner_coverage: false,
                observation_contexts: Vec::new(),
                invocations: Vec::new(),
                documents: Vec::new(),
                writes: Vec::new(),
                artifacts: Vec::new(),
                bindings: Vec::new(),
                reads: Vec::new(),
                owners: Vec::new(),
                read_writer_attachments: Vec::new(),
                dependencies: Vec::new(),
            },
            work: 0,
            bytes,
            owner_namespace: None,
            owner_mask: 0,
            parent_bytes: 0,
        })
    }
    pub(in crate::coordinator) fn step(&mut self) -> Result<(), CoordinatorError> {
        self.work = self
            .work
            .checked_add(1)
            .filter(|n| *n <= self.limits.max_work)
            .ok_or(CoordinatorError::Limit)?;
        Ok(())
    }
    fn charge<T>(&mut self, count: usize, maximum: usize) -> Result<(), CoordinatorError> {
        if count >= maximum {
            return Err(CoordinatorError::Limit);
        }
        self.bytes = self
            .bytes
            .checked_add(size_of::<T>())
            .filter(|n| *n <= self.limits.max_capture_bytes)
            .ok_or(CoordinatorError::Limit)?;
        Ok(())
    }
    pub(in crate::coordinator) fn binding(
        &mut self,
        value: ShutdownBindingRecordV1,
        key_bytes: usize,
    ) -> Result<(), CoordinatorError> {
        self.charge::<ShutdownBindingRecordV1>(self.cut.bindings.len(), self.limits.max_bindings)?;
        self.bytes = self
            .bytes
            .checked_add(key_bytes)
            .filter(|n| *n <= self.limits.max_capture_bytes)
            .ok_or(CoordinatorError::Limit)?;
        self.cut
            .bindings
            .try_reserve(1)
            .map_err(|_| CoordinatorError::Capacity)?;
        self.cut.bindings.push(value);
        Ok(())
    }
    pub(in crate::coordinator) fn read(
        &mut self,
        value: ShutdownReadRecordV1,
        key_bytes: usize,
    ) -> Result<(), CoordinatorError> {
        self.charge::<ShutdownReadRecordV1>(self.cut.reads.len(), self.limits.max_reads)?;
        self.bytes = self
            .bytes
            .checked_add(key_bytes)
            .filter(|n| *n <= self.limits.max_capture_bytes)
            .ok_or(CoordinatorError::Limit)?;
        self.cut
            .reads
            .try_reserve(1)
            .map_err(|_| CoordinatorError::Capacity)?;
        self.cut.reads.push(value);
        Ok(())
    }
    pub(in crate::coordinator) fn owner(
        &mut self,
        value: ShutdownOwnerRecordV1,
    ) -> Result<(), CoordinatorError> {
        self.step()?;
        self.charge::<ShutdownOwnerRecordV1>(self.cut.owners.len(), self.limits.max_owners)?;
        self.account_owner(&value)?;
        self.cut
            .owners
            .try_reserve(1)
            .map_err(|_| CoordinatorError::Capacity)?;
        self.cut.owners.push(value);
        Ok(())
    }
    pub(in crate::coordinator) fn read_writer_attachment(
        &mut self,
        value: ShutdownReadWriterAttachmentV1,
    ) -> Result<(), CoordinatorError> {
        self.step()?;
        self.charge::<ShutdownReadWriterAttachmentV1>(
            self.cut.read_writer_attachments.len(),
            self.limits.max_attachments,
        )?;
        self.cut
            .read_writer_attachments
            .try_reserve(1)
            .map_err(|_| CoordinatorError::Capacity)?;
        self.cut.read_writer_attachments.push(value);
        Ok(())
    }
    fn document(&mut self, value: ShutdownDocumentRecordV1) -> Result<(), CoordinatorError> {
        self.charge::<ShutdownDocumentRecordV1>(
            self.cut.documents.len(),
            self.limits.max_documents,
        )?;
        self.cut
            .documents
            .try_reserve(1)
            .map_err(|_| CoordinatorError::Capacity)?;
        self.cut.documents.push(value);
        Ok(())
    }
    pub(in crate::coordinator) fn write(
        &mut self,
        value: ShutdownWriteRecordV1,
    ) -> Result<(), CoordinatorError> {
        self.charge::<ShutdownWriteRecordV1>(self.cut.writes.len(), self.limits.max_writes)?;
        self.cut
            .writes
            .try_reserve(1)
            .map_err(|_| CoordinatorError::Capacity)?;
        self.cut.writes.push(value);
        Ok(())
    }
    pub(in crate::coordinator) fn artifact(
        &mut self,
        value: ShutdownArtifactRecordV1,
    ) -> Result<(), CoordinatorError> {
        self.charge::<ShutdownArtifactRecordV1>(
            self.cut.artifacts.len(),
            self.limits.max_artifacts,
        )?;
        self.cut
            .artifacts
            .try_reserve(1)
            .map_err(|_| CoordinatorError::Capacity)?;
        self.cut.artifacts.push(value);
        Ok(())
    }
    pub(in crate::coordinator) fn dependency(
        &mut self,
        dependent: ShutdownInventoryTargetV1,
        required: ShutdownInventoryTargetV1,
    ) -> Result<(), CoordinatorError> {
        self.step()?;
        self.charge::<ShutdownDependencyV1>(
            self.cut.dependencies.len(),
            self.limits.max_dependencies,
        )?;
        self.cut
            .dependencies
            .try_reserve(1)
            .map_err(|_| CoordinatorError::Capacity)?;
        self.cut.dependencies.push(ShutdownDependencyV1 {
            dependent,
            required,
        });
        Ok(())
    }
}
impl SourceCoordinator {
    pub(super) fn shutdown_inventory_roles(
        &self,
        basis: &Basis,
        f: &TrustedShutdownInventoryFactsV1<'_>,
    ) -> Result<(), CoordinatorError> {
        if !Arc::ptr_eq(&basis.owner, &self.namespace)
            || basis.context != *f.control.context
            || !facts(f.control)
            || f.inventory_readable != Some(true)
        {
            return Err(CoordinatorError::Unavailable);
        }
        Ok(())
    }
    fn capture_shutdown_cut(
        &self,
        limits: ShutdownInventoryLimitsV1,
    ) -> Result<Cut, CoordinatorError> {
        let mut b = Builder::new(limits)?;
        for (slot, d) in self.documents.iter().enumerate() {
            b.step()?;
            if let Some(d) = d {
                let id = DocumentId {
                    namespace: self.namespace.clone(),
                    slot,
                };
                b.document(ShutdownDocumentRecordV1 {
                    source_binding_generation: self
                        .explicit_disposal_source_binding_generation(&id)
                        .map_err(CoordinatorError::Resource)?,
                    annotation_binding_generation: self
                        .shutdown_annotation_binding_generation(&id)?,
                    source: DocumentVersion {
                        document: DocumentId {
                            namespace: self.namespace.clone(),
                            slot,
                        },
                        revision: d.revision,
                    },
                    lifecycle: d.lifecycle,
                    source_policy_generation: d.policy_generation,
                    source_read_only: d.read_only,
                    serialization: d.serialization,
                    annotation_generation: d.annotation_generation,
                    annotation_policy_generation: d.annotation_policy,
                    annotation_read_only: d.annotation_read_only,
                })?;
            }
        }
        self.capture_shutdown_writer_inventory(&mut b)?;
        self.capture_shutdown_binding_inventory(&mut b)?;
        self.capture_shutdown_reader_inventory(&mut b)?;
        self.capture_shutdown_managed_pools(&mut b)?;
        self.reconcile_shutdown_owners(&mut b)?;
        b.cut.bindings.sort_unstable_by_key(|r| {
            (
                r.document.slot,
                match r.kind {
                    ArtifactKind::Source => 0,
                    ArtifactKind::Annotations => 1,
                },
                r.generation,
            )
        });
        if b.cut
            .bindings
            .windows(2)
            .any(|w| w[0].binding == w[1].binding)
        {
            return Err(CoordinatorError::Faulted);
        }
        Ok(b.cut)
    }
    pub fn capture_shutdown_inventory_v1(
        &self,
        basis: &ShutdownFenceBasisV1,
        limits: ShutdownInventoryLimitsV1,
        f: &TrustedShutdownInventoryFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ShutdownInventoryV1, CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.shutdown_inventory_roles(&basis.0, f)?;
            self.ensure_ordinary_admission()?;
            if self
                .native_root_identity()
                .map_err(|_| CoordinatorError::Faulted)?
                != basis.0.root
            {
                return Err(CoordinatorError::Stale);
            }
            let cut = self.capture_shutdown_cut(limits)?;
            Ok(ShutdownInventoryV1 {
                marker: Arc::new(()),
                basis: basis.0.clone(),
                limits,
                cut: Arc::new(cut),
            })
        })();
        observe_service(
            r,
            Boundary::ShutdownInventoryCapture,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn read_shutdown_inventory_v1<'a>(
        &self,
        inventory: &'a ShutdownInventoryV1,
        f: &TrustedShutdownInventoryFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ShutdownInventoryReadV1<'a>, CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = self
            .shutdown_inventory_roles(&inventory.basis, f)
            .map(|()| ShutdownInventoryReadV1 {
                funding: inventory.cut.funding,
                observation_contexts: &inventory.cut.observation_contexts,
                invocations: &inventory.cut.invocations,
                documents: &inventory.cut.documents,
                writes: &inventory.cut.writes,
                artifacts: &inventory.cut.artifacts,
                bindings: &inventory.cut.bindings,
                reads: &inventory.cut.reads,
                owners: &inventory.cut.owners,
                read_writer_attachments: &inventory.cut.read_writer_attachments,
                dependencies: &inventory.cut.dependencies,
                coverage: ShutdownInventoryCoverageV1 {
                    documents: true,
                    writer_tasks: true,
                    artifacts: true,
                    bindings: true,
                    reader_tasks: true,
                    all_owner_obligations: inventory.cut.owner_coverage,
                },
            });
        observe_service(
            r,
            Boundary::ShutdownInventoryRelease,
            &result,
            timer.finish(),
        );
        result
    }
    /// Recompute actual writer state too: late evidence/stop/retirement can change
    /// obligations without changing authored publication or current-root identity.
    /// This checks captured projection currentness, not omitted owner coverage.
    pub fn validate_shutdown_inventory_current_v1(
        &self,
        inventory: &ShutdownInventoryV1,
        f: &TrustedShutdownInventoryFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.shutdown_inventory_roles(&inventory.basis, f)?;
            self.ensure_operational()?;
            if self
                .native_root_identity()
                .map_err(|_| CoordinatorError::Faulted)?
                != inventory.basis.root
            {
                return Err(CoordinatorError::Stale);
            }
            let current = self.capture_shutdown_cut(inventory.limits)?;
            if current.invocations != inventory.cut.invocations
                || current.funding != inventory.cut.funding
                || current.owner_coverage != inventory.cut.owner_coverage
                || current.observation_contexts != inventory.cut.observation_contexts
                || current.documents != inventory.cut.documents
                || current.writes != inventory.cut.writes
                || current.artifacts != inventory.cut.artifacts
                || current.bindings != inventory.cut.bindings
                || current.reads != inventory.cut.reads
                || current.owners != inventory.cut.owners
                || current.read_writer_attachments != inventory.cut.read_writer_attachments
                || current.dependencies != inventory.cut.dependencies
            {
                return Err(CoordinatorError::Stale);
            }
            Ok(())
        })();
        observe_service(
            r,
            Boundary::ShutdownInventoryValidation,
            &result,
            timer.finish(),
        );
        result
    }
}
