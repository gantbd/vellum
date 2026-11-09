//! Pinned source/annotation artifacts and shared destination evidence. No host I/O.
use super::*;
pub(in crate::coordinator) mod catalog;
mod root_accounting;
use crate::resource::*;
pub use catalog::*;
pub(super) use root_accounting::BaselineRootProjection;
pub(super) use root_accounting::ResourceRootIdentity;
mod annotation_baseline;
mod artifact;
mod comparison;
mod continuity;
pub use continuity::{SourceContinuityLimitsV1, TrustedSourceContinuityLaunchV1};
mod document_pins;
mod lifecycle;
pub use document_pins::DocumentResourcePinsV1;
mod loaded_baseline;
pub(in crate::coordinator) use loaded_baseline::PreparedLoadedBaseline;
mod observation;
mod operations;
mod ordinary;
mod ordinary_save;
pub use observation::*;
pub use operations::{
    ExpectedSaveBindingV1, OwnedAnnotationSaveRequestV1, OwnedSourceSaveRequestV1,
    SaveOperationAdmissionStatusV1, SaveOperationAdmissionV1, SaveOperationConditionV1,
    SaveOperationLimitsV1, SaveOperationOriginV1, SaveOperationUsageV1,
    TrustedAnnotationSaveOperationReleaseV1, TrustedOrdinaryAnnotationSaveAdmissionV1,
    TrustedOrdinarySourceSaveAdmissionV1, TrustedSaveOperationReleaseV1,
};
mod read;
mod source_materialization;
pub(in crate::coordinator) mod source_read;
pub use read::{
    AnnotationLoadAuthorityFactsV1, AnnotationLoadAuthorityV1,
    AnnotationReadOperationAdmissionStatusV1, AnnotationReadOperationAdmissionV1,
    AnnotationReadOperationLimitsV1, AnnotationReadOperationOriginV1,
    AnnotationReadOperationUsageV1, ExpectedAnnotationReadBindingV1, NativeAnnotationReadReceiptV1,
    NativeReadTerminalWitnessV1, ObservedReadVersionV1, OwnedAnnotationReadRequestV1,
    TrustedAnnotationReadOperationFactsV1,
};
pub use source_read::{
    NativeSourceReadReceiptV1, NativeSourceReadTerminalWitnessV1, ObservedSourceReadVersionV1,
    OwnedSourceReadRequestV1, SourceLoadAuthorityFactsV1, SourceLoadAuthorityV1,
    SourceReadActivationDispositionV1, SourceReadActivationV1,
    SourceReadOperationAdmissionStatusV1, SourceReadOperationAdmissionV1,
    SourceReadOperationLimitsV1, SourceReadOperationOriginV1, SourceReadOperationUsageV1,
    SourceReadStatusV1, SourceReaderLimitsV1, SourceReaderUsageV1,
    TrustedSourceReadOperationFactsV1,
};
#[cfg(test)]
mod serialization_tests;
#[cfg(test)]
pub(in crate::coordinator) use serialization_tests::fixture as source_save_test_fixture;
#[cfg(test)]
pub(in crate::coordinator) use serialization_tests::pending_root_fixture as annotation_save_test_fixture;
mod source_activation;
#[cfg(test)]
mod source_capacity_tests;
pub(in crate::coordinator) use source_activation::PreparedSourceSaveAs;
pub(in crate::coordinator) use source_activation::source_save_as_authorized;
pub(in crate::coordinator) mod source_binding;
pub use source_binding::{TrustedSourceBindingHistoryReleaseV1, TrustedSourceRebindFactsV1};
mod source_export;
pub use source_activation::{
    SourceSaveAsAuthorityFactsV1, SourceSaveAsAuthorityV1, TrustedSourceSaveAsFactsV1,
};
mod causal_evidence;
mod evidence;
mod queue_timing;
pub use queue_timing::{
    ResourceQueueAttemptTimingV1, ResourceQueueAttemptV1, ResourceQueuePhaseV1,
    ResourceResultAttemptTimingV1,
};
pub use queue_timing::{
    ResourceQueueKindV1, ResourceQueuePopulationV1, ResourceQueueResponsivenessV1,
    ResourceQueueSnapshotV1,
};
mod shutdown_bindings;
#[cfg(test)]
mod shutdown_factories_tests;
mod shutdown_inventory;
mod shutdown_readers;
mod source_preparation;
mod source_retention;
mod writer_retirement;
pub use source_export::{TrustedSourceExportControlV1, TrustedSourceExportFactsV1};

#[derive(Clone, Debug)]
struct Baseline {
    epoch: Option<Blob>,
    bytes: Blob,
    validator: Option<Blob>,
    anchor: Option<SaveId>,
}
#[derive(Debug)]
struct Binding {
    document: DocumentId,
    identity: Identity,
    registration: Arc<Registration>,
    generation: u64,
    baseline: Option<Baseline>,
}
impl Binding {
    fn baseline_anchor(&self) -> Option<&SaveId> {
        self.baseline.as_ref().and_then(|b| b.anchor.as_ref())
    }
    fn key(&self) -> BindingKey {
        BindingKey {
            identity: self.identity.clone(),
            registration: self.registration.clone(),
            generation: self.generation,
        }
    }
}
#[derive(Debug)]
struct AnnotationBinding {
    document: DocumentId,
    key: BindingKey,
    baseline: Option<AnnotationBaselineSlot>,
}
/// Disposed current bindings and explicit source association history. History
/// rows are separately bounded at publication and retain their producer pins.
#[derive(Debug, Default)]
pub(super) struct RetainedBindings {
    source: Option<Binding>,
    annotations: Option<AnnotationBinding>,
    source_history: Vec<Binding>,
}
impl RetainedBindings {
    pub(super) fn is_empty(&self) -> bool {
        self.source.is_none() && self.annotations.is_none() && self.source_history.is_empty()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct AnnotationAssociation {
    binding: BindingKey,
    source_binding: Option<BindingKey>,
}
#[derive(Debug)]
struct AnnotationBaseline {
    fact: AnnotationBaselineFact,
    association: AnnotationAssociation,
    bytes: Blob,
    external_version: Option<Blob>,
}
#[derive(Debug)]
enum AnnotationBaselineSlot {
    Written(AnnotationBaseline),
    Loaded(LoadedAnnotationBaseline),
}
#[derive(Clone, Debug)]
struct LoadedAnnotationBaseline {
    attachment: Arc<()>,
    fact: LoadedAnnotationBaselineFact,
    association: AnnotationAssociation,
    encoding: Arc<EncoderRegistration>,
    registration: Arc<Registration>,
    bytes: Blob,
    version: Blob,
}
#[derive(Debug)]
struct ReadBaselineAttachment {
    reader: ReadBaselineEndpoint,
    writer: ReadBaselineEndpoint,
}
#[derive(Debug)]
struct ReadBaselineEndpoint {
    owner: Arc<()>,
    incarnation: Arc<()>,
}
impl ReadBaselineAttachment {
    fn current(&self, owner: &Arc<()>) -> Result<&Arc<()>, ResourceError> {
        if !Arc::ptr_eq(&self.reader.owner, owner)
            || !Arc::ptr_eq(&self.writer.owner, owner)
            || !Arc::ptr_eq(&self.reader.incarnation, &self.writer.incarnation)
        {
            return Err(ResourceError::Faulted);
        }
        Ok(&self.reader.incarnation)
    }
}
#[derive(Clone, Debug, Default)]
struct Knowledge {
    data: ResultData,
    witnesses: [Option<ResultData>; 4],
    conflict: Option<(ResultData, ResultData)>,
}
impl Knowledge {
    fn merge(&mut self, incoming: ResultData) -> Result<EvidenceDisposition, ResourceError> {
        if let Some((first, second)) = &self.conflict {
            return if &incoming == first
                || &incoming == second
                || self.witnesses.iter().flatten().any(|w| w == &incoming)
            {
                Ok(EvidenceDisposition::Duplicate)
            } else {
                Err(ResourceError::EvidenceCapacity)
            };
        }
        let old = &self.data;
        let first = if old.application != Application::Unknown
            && incoming.application != Application::Unknown
            && old.application != incoming.application
        {
            self.witnesses[0].as_ref()
        } else if old.synchronization != Synchronization::Unknown
            && incoming.synchronization != Synchronization::Unknown
            && old.synchronization != incoming.synchronization
        {
            self.witnesses[1].as_ref()
        } else if old.version.is_some()
            && incoming.version.is_some()
            && (old.version != incoming.version || old.epoch != incoming.epoch)
        {
            self.witnesses[2].as_ref()
        } else if old.application == Application::NotWritten && incoming.observed > 0 {
            self.witnesses[0].as_ref()
        } else if incoming.application == Application::NotWritten && old.observed > 0 {
            self.witnesses[3].as_ref()
        } else {
            None
        };
        if let Some(first) = first {
            self.conflict = Some((first.clone(), incoming));
            return Ok(EvidenceDisposition::Contradiction);
        }
        let before = self.data.clone();
        if incoming.application != Application::Unknown
            && self.data.application == Application::Unknown
        {
            self.data.application = incoming.application;
            self.witnesses[0] = Some(incoming.clone());
        }
        if incoming.synchronization != Synchronization::Unknown
            && self.data.synchronization == Synchronization::Unknown
        {
            self.data.synchronization = incoming.synchronization;
            self.witnesses[1] = Some(incoming.clone());
        }
        if incoming.version.is_some() && self.data.version.is_none() {
            self.data.version = incoming.version.clone();
            self.data.epoch = incoming.epoch.clone();
            self.witnesses[2] = Some(incoming.clone());
        }
        if incoming.observed > self.data.observed {
            self.data.observed = incoming.observed;
            self.witnesses[3] = Some(incoming);
        }
        Ok(if self.data == before {
            EvidenceDisposition::Duplicate
        } else {
            EvidenceDisposition::Accepted
        })
    }
}
#[derive(Debug)]
struct Task {
    queue_timing: queue_timing::Stamp,
    catalog: Option<Arc<WriteCatalogCaptureV1>>,
    operation: Option<usize>,
    result_version_profile: SaveResultVersionProfileV1,
    continuity: Option<continuity::Pending>,
    launched_condition: Option<DispatchCondition>,
    causal: causal_evidence::CausalKnowledge,
    source_export: Option<source_export::ExportState>,
    protection: Protection,
    serialization: Option<SourceSerializationStateV1>,
    baseline_claim: Option<BaselineClaim>,
    dispatch_serial: Option<u64>,
    annotation_association: Option<AnnotationAssociation>,
    kind: ArtifactKind,
    purpose: SavePurpose,
    artifact: ResourceArtifactId,
    captured_annotation_set: Option<u64>,
    encoding_context: Option<AnnotationEncodingContext>,
    captured: DocumentVersion,
    binding_generation: u64,
    identity: Identity,
    registration: Arc<Registration>,
    payload: Blob,
    condition: WriteCondition,
    synchronize: bool,
    phase: SavePhase,
    nonce: Option<Arc<()>>,
    knowledge: Knowledge,
    no_future: bool,
    quiescent: bool,
    stop_requested: bool,
}
#[derive(Debug)]
enum WriteCondition {
    Present(Baseline),
    Absent,
}
#[derive(Debug)]
struct BaselineClaim {
    state: BaselineCapacityState,
    charge: usize,
}
impl Task {
    fn source_baseline_candidate(&self) -> bool {
        match self.purpose {
            SavePurpose::OrdinarySource => true,
            SavePurpose::SourceSaveAs => self
                .source_export
                .as_ref()
                .is_some_and(|s| s.binding_applied),
            SavePurpose::OrdinaryAnnotations
            | SavePurpose::AnnotationExport
            | SavePurpose::SourceCopyExport => false,
        }
    }
    fn safe(&self) -> bool {
        self.phase == SavePhase::Stopped
            || (self.phase == SavePhase::Dispatched
                && !self.contested()
                && self.knowledge.data.application != Application::Unknown
                && self.no_future)
    }
    fn qualified(&self) -> bool {
        self.phase == SavePhase::Dispatched
            && !self.contested()
            && self.knowledge.data.application == Application::Written
            && (!self.synchronize
                || self.knowledge.data.synchronization == Synchronization::Succeeded)
    }
    fn snapshot(&self) -> SaveOutcome {
        SaveOutcome {
            result_version_profile: self.result_version_profile,
            own_chain: self.continuity.is_some(),
            continuity_edges: self
                .continuity
                .as_ref()
                .and_then(|c| c.receipt.as_ref().map(|r| r.edges.len())),
            causal_edge_available: self.causal.edge_available(),
            causal_conflict_witnesses: self.causal.conflicts(),
            source_save_as_binding_applied: match self.purpose {
                SavePurpose::SourceSaveAs => self.source_export.as_ref().map(|s| s.binding_applied),
                SavePurpose::OrdinarySource
                | SavePurpose::OrdinaryAnnotations
                | SavePurpose::AnnotationExport
                | SavePurpose::SourceCopyExport => None,
            },
            source_export_reservation_released: self.source_export.as_ref().map(|s| s.released),
            baseline_capacity: self.baseline_claim.as_ref().map(|c| c.state),
            dispatch_serial: self.dispatch_serial,
            ordinary_annotation_generation: self
                .annotation_association
                .as_ref()
                .map(|a| a.binding.generation),
            kind: self.kind,
            purpose: self.purpose,
            artifact: self.artifact.clone(),
            captured_annotation_set: self.captured_annotation_set,
            captured: self.captured.clone(),
            binding_generation: match self.purpose {
                SavePurpose::OrdinarySource => Some(self.binding_generation),
                SavePurpose::OrdinaryAnnotations => self
                    .annotation_association
                    .as_ref()
                    .map(|a| a.binding.generation),
                SavePurpose::AnnotationExport
                | SavePurpose::SourceCopyExport
                | SavePurpose::SourceSaveAs => None,
            },
            serialization_generation: self.serialization.map(|s| s.generation),
            source_serialization: self.serialization,
            encoding_context: self.encoding_context.clone(),
            phase: self.phase,
            synchronization_required: self.synchronize,
            application: self.knowledge.data.application,
            observed_bytes: self.knowledge.data.observed,
            synchronization: self.knowledge.data.synchronization,
            version_available: self.knowledge.data.version.is_some(),
            no_future_effects: self.no_future,
            quiescent: self.quiescent,
            stop_requested: self.stop_requested,
            contested: self.contested(),
        }
    }
}
#[derive(Debug)]
pub(super) struct ResourceState {
    queue_timing: queue_timing::State,
    catalog: catalog::State,
    source_reader: source_read::State,
    read_operations: read::operations::State,
    annotation_read_admission_stopped: bool,
    annotation_reader_closed: bool,
    writer_closed: bool,
    observation: observation::State,
    operations: operations::State,
    continuity: continuity::State,
    limits: Option<ResourceLimits>,
    providers: Vec<Arc<Registration>>,
    bindings: Vec<Binding>,
    annotation_bindings: Vec<AnnotationBinding>,
    tasks: Vec<Option<Task>>,
    encoders: Vec<Arc<EncoderRegistration>>,
    artifacts: Vec<Option<ArtifactRow>>,
    source_preparation: Option<source_preparation::SourcePreparation>,
    source_export_reservations: Vec<source_export::Reservation>,
    held_artifact_bytes: usize,
    last_dispatch_serial: u64,
    annotation_save_admission_stopped: bool,
    source_save_admission_stopped: bool,
    reads: Vec<Option<read::ReadTask>>,
    held_read_bytes: usize,
    restorers: Vec<Arc<RestorationRegistration>>,
    read_baseline_attachment: Option<ReadBaselineAttachment>,
    // Original attachment lifecycle, including absence. Attach reserves a future
    // detach step before any endpoint credit/identity is installed.
    read_baseline_generation: u64,
}
#[derive(Debug)]
enum ArtifactBasis {
    Source {
        source: DocumentVersion,
        serialization: SourceSerializationStateV1,
    },
    Annotations {
        snapshot: AnnotationSnapshot,
        encoder: Arc<EncoderRegistration>,
    },
}
impl ArtifactBasis {
    fn source(&self) -> &DocumentVersion {
        match self {
            Self::Source { source, .. } => source,
            Self::Annotations { snapshot, .. } => snapshot.source(),
        }
    }
}
#[derive(Debug)]
struct ArtifactRow {
    catalog: Option<Arc<WriteCatalogCaptureV1>>,
    basis: ArtifactBasis,
    bytes: Blob,
    stopped: bool,
    loaded_condition: Option<LoadedAnnotationBaseline>,
}
impl ArtifactRow {
    fn held_size(&self) -> Option<usize> {
        self.bytes
            .0
            .len()
            .checked_add(match &self.loaded_condition {
                Some(condition) => condition.retained_size()?,
                None => 0,
            })
            .and_then(|n| {
                n.checked_add(
                    self.catalog
                        .as_ref()
                        .map(|c| c.native_bytes())
                        .transpose()
                        .ok()?
                        .unwrap_or(0),
                )
            })
    }
}
impl ResourceState {
    pub(super) fn disabled() -> Self {
        Self {
            catalog: Default::default(),
            queue_timing: Default::default(),
            limits: None,
            read_operations: Default::default(),
            source_reader: Default::default(),
            annotation_read_admission_stopped: false,
            annotation_reader_closed: false,
            writer_closed: false,
            observation: Default::default(),
            operations: Default::default(),
            continuity: Default::default(),
            providers: Vec::new(),
            bindings: Vec::new(),
            annotation_bindings: Vec::new(),
            tasks: Vec::new(),
            encoders: Vec::new(),
            artifacts: Vec::new(),
            source_preparation: None,
            source_export_reservations: Vec::new(),
            held_artifact_bytes: 0,
            last_dispatch_serial: 0,
            annotation_save_admission_stopped: false,
            source_save_admission_stopped: false,
            reads: Vec::new(),
            held_read_bytes: 0,
            restorers: Vec::new(),
            read_baseline_attachment: None,
            read_baseline_generation: 0,
        }
    }
    pub(super) fn new(limits: ResourceLimits) -> Self {
        Self {
            continuity: Default::default(),
            limits: Some(limits),
            ..Self::disabled()
        }
    }
}
pub(super) fn owner_error(error: CoordinatorError) -> ResourceError {
    match error {
        CoordinatorError::Faulted => ResourceError::Faulted,
        CoordinatorError::Unavailable => ResourceError::Unavailable,
        CoordinatorError::Stale => ResourceError::Stale,
        CoordinatorError::Limit => ResourceError::Limit,
        CoordinatorError::VersionExhausted => ResourceError::VersionExhausted,
        CoordinatorError::Capacity => ResourceError::Capacity,
        CoordinatorError::Source(error) => ResourceError::Source(error),
        CoordinatorError::Resource(error) => error,
        CoordinatorError::History(HistoryError::Faulted) => ResourceError::Faulted,
        CoordinatorError::History(_) => ResourceError::Retained,
        CoordinatorError::DuplicateParticipant => ResourceError::Malformed,
        CoordinatorError::Search(_) => ResourceError::Malformed,
        CoordinatorError::Scope(_) => ResourceError::Malformed,
        CoordinatorError::ReadDependency(_) => ResourceError::Malformed,
        CoordinatorError::Annotation(_) => ResourceError::Retained,
    }
}
fn capture<T>(recorder: &mut Recorder, boundary: Boundary, result: &Result<T, ResourceError>) {
    capture_service(recorder, boundary, result, None);
}
fn capture_service<T>(
    recorder: &mut Recorder,
    boundary: Boundary,
    result: &Result<T, ResourceError>,
    service_time: Option<std::time::Duration>,
) {
    recorder.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(ResourceError::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time,
    });
}

impl SourceCoordinator {
    pub(super) fn explicit_disposal_source_binding_generation(
        &self,
        document: &DocumentId,
    ) -> Result<Option<u64>, ResourceError> {
        let mut rows = self
            .resources
            .bindings
            .iter()
            .filter(|b| b.document == *document);
        let generation = rows.next().map(|b| b.generation);
        if rows.next().is_some() {
            return Err(ResourceError::Faulted);
        }
        Ok(generation)
    }
    fn validate_source_capture(&self, task: &Task) -> Result<(), ResourceError> {
        let row = self
            .resource_artifact(&task.artifact)
            .map_err(|_| ResourceError::Faulted)?;
        match &row.basis {
            ArtifactBasis::Source {
                source,
                serialization,
            } if *source == task.captured
                && Some(*serialization) == task.serialization
                && serialization.valid()
                && row.bytes == task.payload =>
            {
                Ok(())
            }
            _ => Err(ResourceError::Faulted),
        }
    }
    pub fn resource_usage(&self) -> ResourceUsage {
        let mut usage = ResourceUsage {
            continuity_retained_bytes: self.resources.continuity.charged,
            source_binding_history_entries: self
                .documents
                .iter()
                .flatten()
                .map(|d| d.retained_bindings.source_history.len())
                .sum(),
            source_export_reservations: self.resources.source_export_reservations.len(),
            retained_source_bindings: self
                .documents
                .iter()
                .flatten()
                .filter(|d| d.retained_bindings.source.is_some())
                .count(),
            retained_annotation_bindings: self
                .documents
                .iter()
                .flatten()
                .filter(|d| d.retained_bindings.annotations.is_some())
                .count(),
            read_baseline_attachments: usize::from(
                self.resources.read_baseline_attachment.is_some(),
            ),
            restoration_boundaries: self.resources.restorers.len(),
            restored_reads: self
                .resources
                .reads
                .iter()
                .flatten()
                .filter(|r| r.restoration.is_some())
                .count(),
            retained_reads: self.resources.reads.iter().flatten().count(),
            issued_reads_lifetime: self.resources.reads.len(),
            held_read_bytes: self.resources.held_read_bytes,
            submitted_reads: self
                .resources
                .reads
                .iter()
                .flatten()
                .filter(|t| t.phase == ReadPhase::Submitted && !t.quiescent)
                .count(),
            contested_reads: self
                .resources
                .reads
                .iter()
                .flatten()
                .filter(|t| t.contested)
                .count(),
            annotation_save_admission_stopped: self.resources.annotation_save_admission_stopped,
            pending_baseline_publications: self.pending_baseline_units(None),
            reserved_baseline_bytes: self
                .resources
                .tasks
                .iter()
                .flatten()
                .filter_map(|t| t.baseline_claim.as_ref())
                .filter(|c| c.state == BaselineCapacityState::Pending)
                .fold(0usize, |n, c| n.saturating_add(c.charge)),
            annotation_baselines: self
                .resources
                .annotation_bindings
                .iter()
                .filter(|b| b.baseline.is_some())
                .count(),
            annotation_baseline_logical_bytes: self
                .resources
                .annotation_bindings
                .iter()
                .filter_map(|b| b.baseline.as_ref())
                .fold(0usize, |sum, baseline| {
                    sum.saturating_add(baseline.retained_size().unwrap_or(usize::MAX))
                }),
            annotation_bindings: self.resources.annotation_bindings.len(),
            encoders: self.resources.encoders.len(),
            issued_artifacts_lifetime: self.resources.artifacts.len(),
            retained_artifacts: self.resources.artifacts.iter().flatten().count(),
            held_artifact_bytes: self.resources.held_artifact_bytes,
            prepared_sources: usize::from(self.resources.source_preparation.is_some()),
            prepared_source_bytes: self
                .resources
                .source_preparation
                .as_ref()
                .map_or(0, |p| p.bytes.0.len()),
            enabled: self.resources.limits.is_some(),
            providers: self.resources.providers.len(),
            bindings: self.resources.bindings.len(),
            issued_lifetime: self.resources.tasks.len(),
            retained: 0,
            queued: 0,
            outstanding: 0,
            unresolved: 0,
            contested_tasks: 0,
            max_live_tasks: 0,
            max_issued_tasks: 0,
            max_outstanding: 0,
        };
        if let Some(limits) = self.resources.limits {
            usage.max_live_tasks = limits.max_live_tasks;
            usage.max_issued_tasks = limits.max_issued_tasks;
            usage.max_outstanding = limits.max_outstanding;
        }
        for task in self.resources.tasks.iter().flatten() {
            usage.retained += 1;
            usage.queued += usize::from(task.phase == SavePhase::Queued);
            usage.outstanding +=
                usize::from(task.phase == SavePhase::Dispatched && !task.quiescent);
            usage.unresolved += usize::from(task.phase == SavePhase::Dispatched && !task.safe());
            usage.contested_tasks += usize::from(task.contested());
        }
        usage.outstanding = usage
            .outstanding
            .saturating_add(usage.submitted_reads)
            .saturating_add(self.outstanding_source_reads());
        usage
    }

    pub fn configured(config: CoordinatorConfig) -> Self {
        let mut owner = Self::new(config.limits);
        owner.searches.limits = config.search;
        owner.annotations.limits = config.annotations;
        if let Some(limits) = config.invocation {
            owner.invocations = invocation::InvocationTable::new(limits);
        }
        if let Some(limits) = config.history {
            owner.history = history::HistoryState::new(limits);
        }
        if let Some(limits) = config.resources {
            owner.resources = ResourceState::new(limits);
        }
        owner
    }
    fn provider_current(&self, registration: &Arc<Registration>) -> Result<(), ResourceError> {
        if !Arc::ptr_eq(&self.namespace, &registration.owner)
            || !self
                .resources
                .providers
                .iter()
                .any(|p| Arc::ptr_eq(p, registration))
        {
            return Err(ResourceError::Unavailable);
        }
        Ok(())
    }
    pub fn register_provider(
        &mut self,
        profile: ProviderProfile,
        domain: Option<&IdentityDomain>,
        recorder: &mut Recorder,
    ) -> Result<ProviderPort, ResourceError> {
        self.register_provider_with_destination_capabilities(
            profile,
            ProviderDestinationCapabilitiesV1::default(),
            domain,
            recorder,
        )
    }
    pub fn register_provider_with_destination_capabilities(
        &mut self,
        profile: ProviderProfile,
        destinations: ProviderDestinationCapabilitiesV1,
        domain: Option<&IdentityDomain>,
        recorder: &mut Recorder,
    ) -> Result<ProviderPort, ResourceError> {
        self.register_provider_with_causal_capabilities(
            profile,
            destinations,
            ProviderCausalCapabilitiesV1::default(),
            domain,
            recorder,
        )
    }
    pub fn register_provider_with_causal_capabilities(
        &mut self,
        profile: ProviderProfile,
        destinations: ProviderDestinationCapabilitiesV1,
        causal: ProviderCausalCapabilitiesV1,
        domain: Option<&IdentityDomain>,
        recorder: &mut Recorder,
    ) -> Result<ProviderPort, ResourceError> {
        let result = (|| {
            self.ensure_ordinary_admission().map_err(owner_error)?;
            let limits = self.resources.limits.ok_or(ResourceError::Unsupported)?;
            if self.resources.providers.len() >= limits.max_providers {
                return Err(ResourceError::Limit);
            }
            if domain.is_some_and(|d| !Arc::ptr_eq(&self.namespace, &d.owner)) {
                return Err(ResourceError::Unavailable);
            }
            let growth = root_accounting::registration_bytes::<Registration>()
                .map_err(|_| ResourceError::Limit)?;
            self.check_root_growth(growth, recorder)
                .map_err(owner_error)?;
            self.resources
                .providers
                .try_reserve(1)
                .map_err(|_| ResourceError::Capacity)?;
            let domain = domain.cloned().unwrap_or_else(|| IdentityDomain {
                owner: self.namespace.clone(),
                marker: Arc::new(()),
            });
            let registration = Arc::new(Registration {
                causal,
                owner: self.namespace.clone(),
                domain,
                profile,
                destinations,
                limits,
            });
            self.resources.providers.push(registration.clone());
            Ok(ProviderPort { registration })
        })();
        if matches!(result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture(recorder, Boundary::ResourceBinding, &result);
        result
    }
    pub fn load_resource(
        &mut self,
        read: ResourceRead,
        recorder: &mut Recorder,
    ) -> Result<DocumentVersion, ResourceError> {
        self.load_resource_with_insertion_policy_v1(read, GeneratedNewlineV1::Lf, recorder)
    }
    /// Policy belongs to a newly materialized document. An existing owner is
    /// joined unchanged, even when the incoming policy differs.
    pub fn load_resource_with_insertion_policy_v1(
        &mut self,
        read: ResourceRead,
        insertion_policy: GeneratedNewlineV1,
        recorder: &mut Recorder,
    ) -> Result<DocumentVersion, ResourceError> {
        let timing = recorder.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let plan = self.prepare_source_materialization(read, insertion_policy, recorder)?;
            plan.validate(self, recorder)?;
            let version = plan.version.clone();
            self.faulted = true;
            plan.install_fenced(self);
            self.faulted = false;
            Ok(version)
        }));
        let result = match caught {
            Ok(v) => v,
            Err(p) => {
                self.faulted = true;
                capture_service(
                    recorder,
                    Boundary::ResourceBinding,
                    &Err::<DocumentVersion, _>(ResourceError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(p)
            }
        };
        if matches!(result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture_service(
            recorder,
            Boundary::ResourceBinding,
            &result,
            timing.finish(),
        );
        result
    }
    fn resource_task(&self, id: &SaveId) -> Result<&Task, ResourceError> {
        if !Arc::ptr_eq(&self.namespace, &id.owner) {
            return Err(ResourceError::Unavailable);
        }
        let task = self
            .resources
            .tasks
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(ResourceError::Unavailable)?;
        if let Some(c) = &task.catalog {
            c.intact(&self.namespace, &task.registration, &task.identity)?;
            if task.continuity.is_some() && !c.allows_own_chain() {
                return Err(ResourceError::Faulted);
            }
        }
        let artifact = self
            .resources
            .artifacts
            .get(task.artifact.slot)
            .and_then(Option::as_ref)
            .ok_or(ResourceError::Faulted)?;
        if !WriteCatalogCaptureV1::same(task.catalog.as_ref(), artifact.catalog.as_ref()) {
            return Err(ResourceError::Faulted);
        }
        Ok(task)
    }
    fn resource_contested(&self, identity: &Identity) -> bool {
        self.resources
            .tasks
            .iter()
            .flatten()
            .any(|t| t.identity == *identity && t.contested())
    }
    pub fn queue_pinned_save(
        &mut self,
        expected: &DocumentVersion,
        requirements: SaveRequirements,
        recorder: &mut Recorder,
    ) -> Result<SaveId, ResourceError> {
        self.queue_pinned_save_checked(expected, requirements, None, false, None, recorder)
    }
    pub fn queue_pinned_save_with_pair(
        &mut self,
        expected: &DocumentVersion,
        requirements: SaveRequirements,
        pair: OrdinaryPairEvidence,
        recorder: &mut Recorder,
    ) -> Result<SaveId, ResourceError> {
        self.queue_pinned_save_checked(expected, requirements, Some(&pair), false, None, recorder)
    }
    fn queue_pinned_save_checked(
        &mut self,
        expected: &DocumentVersion,
        requirements: SaveRequirements,
        pair: Option<&OrdinaryPairEvidence>,
        own_chain: bool,
        catalog: Option<Arc<WriteCatalogCaptureV1>>,
        recorder: &mut Recorder,
    ) -> Result<SaveId, ResourceError> {
        self.queue_pinned_save_with_parent(
            expected,
            requirements,
            pair,
            own_chain,
            (catalog, None),
            recorder,
        )
    }
    fn queue_pinned_save_with_parent(
        &mut self,
        expected: &DocumentVersion,
        requirements: SaveRequirements,
        pair: Option<&OrdinaryPairEvidence>,
        own_chain: bool,
        installation: (Option<Arc<WriteCatalogCaptureV1>>, Option<operations::Row>),
        recorder: &mut Recorder,
    ) -> Result<SaveId, ResourceError> {
        let timing = recorder.begin_service_timing();
        let (catalog, parent) = installation;
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.ensure_operational().map_err(owner_error)?;
            self.ensure_admission_unfenced().map_err(owner_error)?;
            if self.resources.source_save_admission_stopped {
                return Err(ResourceError::Unavailable);
            }
            self.validate_ordinary_pair(&expected.document, pair)?;
            let limits = self.resources.limits.ok_or(ResourceError::Unsupported)?;
            let document = self.validate_version(expected).map_err(owner_error)?;
            if document.read_only {
                return Err(ResourceError::Permission);
            }
            let binding = self
                .resources
                .bindings
                .iter()
                .find(|b| b.document == expected.document)
                .ok_or(ResourceError::Unbound)?;
            self.writer_catalog_current(
                catalog.as_ref(),
                &binding.registration,
                &binding.identity,
                requirements.acknowledgment == Acknowledgment::Synchronized,
                own_chain,
                recorder,
            )?;
            if requirements.protection != Protection::BestEffort
                || (requirements.acknowledgment == Acknowledgment::Synchronized
                    && !binding.registration.profile.synchronization_supported)
            {
                return Err(ResourceError::Unsupported);
            }
            if self.resource_contested(&binding.identity) {
                return Err(ResourceError::Contested);
            }
            let baseline = binding
                .baseline
                .as_ref()
                .ok_or(ResourceError::MissingEvidence)?;
            if baseline.validator.is_none() {
                return Err(ResourceError::MissingEvidence);
            }
            let continuity = if own_chain {
                Some(self.prepare_continuity(binding)?)
            } else {
                None
            };
            if self.resources.tasks.len() >= limits.max_issued_tasks
                || self.resources.tasks.iter().flatten().count() >= limits.max_live_tasks
                || self
                    .resources
                    .tasks
                    .iter()
                    .flatten()
                    .filter(|t| t.identity == binding.identity && t.phase == SavePhase::Queued)
                    .count()
                    >= limits.max_queued_per_destination
            {
                return Err(ResourceError::Limit);
            }
            let serialization = document.serialization;
            if !serialization.valid() {
                return Err(ResourceError::Faulted);
            }
            if serialization
                .serialization
                .encoded_len(&document.source)
                .ok_or(ResourceError::Limit)?
                > limits.max_blob_bytes
            {
                return Err(ResourceError::Limit);
            }
            let payload = Blob(Arc::from(
                serialization
                    .serialization
                    .encode(&document.source, recorder)
                    .map_err(ResourceError::Source)?,
            ));
            let mut task = Task {
                queue_timing: queue_timing::Stamp::Off,
                catalog: catalog.clone(),
                operation: None,
                result_version_profile: if baseline.epoch.is_some() {
                    SaveResultVersionProfileV1::EpochToken
                } else {
                    SaveResultVersionProfileV1::Opaque
                },
                continuity,
                launched_condition: None,
                causal: Default::default(),
                source_export: None,
                protection: requirements.protection,
                serialization: Some(serialization),
                baseline_claim: None,
                dispatch_serial: None,
                annotation_association: None,
                kind: ArtifactKind::Source,
                purpose: SavePurpose::OrdinarySource,
                artifact: ResourceArtifactId {
                    owner: self.namespace.clone(),
                    slot: self.resources.artifacts.len(),
                },
                captured_annotation_set: None,
                encoding_context: None,
                captured: expected.clone(),
                binding_generation: binding.generation,
                identity: binding.identity.clone(),
                registration: binding.registration.clone(),
                payload: payload.clone(),
                condition: WriteCondition::Present(baseline.clone()),
                synchronize: requirements.acknowledgment == Acknowledgment::Synchronized,
                phase: SavePhase::Queued,
                nonce: None,
                knowledge: Knowledge::default(),
                no_future: false,
                quiescent: false,
                stop_requested: false,
            };
            self.prepare_save_parent(&mut task, parent.as_ref())?;
            let units = self
                .pending_baseline_units(None)
                .checked_add(1)
                .and_then(|n| u64::try_from(n).ok())
                .ok_or(ResourceError::VersionExhausted)?;
            self.generation
                .checked_add(units)
                .ok_or(ResourceError::VersionExhausted)?;
            let charge = task
                .required_baseline_claim_charge()
                .map_err(|_| ResourceError::Limit)?;
            let reserved = self
                .resources
                .tasks
                .iter()
                .flatten()
                .filter_map(|t| t.baseline_claim.as_ref())
                .filter(|c| c.state == BaselineCapacityState::Pending)
                .try_fold(0usize, |n, c| n.checked_add(c.charge))
                .ok_or(ResourceError::Faulted)?;
            if reserved
                .checked_add(charge)
                .is_none_or(|n| n > limits.max_future_baseline_bytes)
            {
                return Err(ResourceError::Limit);
            }
            task.baseline_claim = Some(BaselineClaim {
                state: BaselineCapacityState::Pending,
                charge,
            });
            self.check_root_replacement_projected(
                0,
                0,
                BaselineRootProjection {
                    admission: Some(&task),
                    ..BaselineRootProjection::default()
                },
                recorder,
            )
            .map_err(owner_error)?;
            self.resources
                .tasks
                .try_reserve(1)
                .map_err(|_| ResourceError::Capacity)?;
            let catalog_bytes = catalog
                .as_ref()
                .map(|c| c.native_bytes())
                .transpose()?
                .unwrap_or(0);
            self.preflight_artifact_capture(payload.0.len(), catalog_bytes)?;
            let id = SaveId {
                owner: self.namespace.clone(),
                slot: self.resources.tasks.len(),
            };
            let artifact = ArtifactRow {
                catalog,
                loaded_condition: None,
                basis: ArtifactBasis::Source {
                    source: expected.clone(),
                    serialization,
                },
                bytes: payload,
                stopped: false,
            };
            self.install_save_admission(task, artifact, parent, own_chain);
            Ok(id)
        }));
        let result = match caught {
            Ok(v) => v,
            Err(p) => {
                self.faulted = true;
                capture_service(
                    recorder,
                    if own_chain {
                        Boundary::SourceContinuityAdmission
                    } else {
                        Boundary::SaveAdmission
                    },
                    &Err::<SaveId, _>(ResourceError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(p)
            }
        };
        if matches!(result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture_service(
            recorder,
            if own_chain {
                Boundary::SourceContinuityAdmission
            } else {
                Boundary::SaveAdmission
            },
            &result,
            timing.finish(),
        );
        result
    }
    pub fn dispatch_save(
        &mut self,
        id: &SaveId,
        probe: ResourceRead,
        recorder: &mut Recorder,
    ) -> Result<SaveDispatch, ResourceError> {
        self.dispatch_save_checked(
            id,
            probe.into_source_export_destination(),
            None,
            None,
            None,
            recorder,
        )
    }
    pub fn dispatch_save_with_pair(
        &mut self,
        id: &SaveId,
        probe: ResourceRead,
        pair: OrdinaryPairEvidence,
        recorder: &mut Recorder,
    ) -> Result<SaveDispatch, ResourceError> {
        self.dispatch_save_checked(
            id,
            probe.into_source_export_destination(),
            Some(&pair),
            None,
            None,
            recorder,
        )
    }
    fn dispatch_save_checked(
        &mut self,
        id: &SaveId,
        probe: SourceExportDestinationV1,
        pair: Option<&OrdinaryPairEvidence>,
        export_facts: Option<&TrustedSourceExportFactsV1>,
        continuity_facts: Option<&TrustedSourceContinuityLaunchV1>,
        recorder: &mut Recorder,
    ) -> Result<SaveDispatch, ResourceError> {
        let timing = recorder.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            let task = self.resource_task(id)?;
            if task.phase != SavePhase::Queued {
                return Err(ResourceError::Unavailable);
            }
            self.writer_catalog_current(
                task.catalog.as_ref(),
                &task.registration,
                &task.identity,
                task.synchronize,
                task.continuity.is_some(),
                recorder,
            )?;
            self.provider_current(&probe.registration)?;
            if !Arc::ptr_eq(&probe.registration, &task.registration)
                || probe.identity != task.identity
            {
                return Err(ResourceError::Unavailable);
            }
            let document = self
                .document(&task.captured.document)
                .map_err(owner_error)?;
            if task.purpose == SavePurpose::OrdinarySource && document.read_only {
                return Err(ResourceError::Permission);
            }
            if matches!(
                task.purpose,
                SavePurpose::SourceCopyExport | SavePurpose::SourceSaveAs
            ) {
                self.validate_source_export_dispatch(id, task, export_facts, pair)?;
            } else if task.purpose == SavePurpose::OrdinarySource {
                self.validate_source_capture(task)?;
                self.validate_ordinary_pair(&task.captured.document, pair)?;
                let binding = self
                    .resources
                    .bindings
                    .iter()
                    .find(|b| b.document == task.captured.document)
                    .ok_or(ResourceError::Unavailable)?;
                if binding.generation != task.binding_generation
                    || binding.identity != task.identity
                    || !Arc::ptr_eq(&binding.registration, &task.registration)
                {
                    return Err(ResourceError::Stale);
                }
            } else {
                if task.purpose == SavePurpose::OrdinaryAnnotations {
                    self.validate_ordinary_pair(&task.captured.document, pair)?;
                    if document.annotation_read_only {
                        return Err(ResourceError::Permission);
                    }
                    if task.annotation_association
                        != self.current_annotation_association(&task.captured.document)
                        || task
                            .baseline_claim
                            .as_ref()
                            .is_none_or(|c| c.state != BaselineCapacityState::Pending)
                    {
                        return Err(ResourceError::Stale);
                    }
                }
                let artifact = self.resource_artifact(&task.artifact)?;
                if task.purpose == SavePurpose::OrdinaryAnnotations
                    && let Some(condition) = &artifact.loaded_condition
                {
                    let binding = self
                        .resources
                        .annotation_bindings
                        .iter()
                        .find(|b| b.document == task.captured.document)
                        .ok_or(ResourceError::Unbound)?;
                    self.validate_loaded_baseline(binding, condition)?;
                }
                let ArtifactBasis::Annotations { encoder, .. } = &artifact.basis else {
                    return Err(ResourceError::Unavailable);
                };
                self.encoder_current(encoder)?;
            }
            if self.resource_contested(&task.identity) {
                return Err(ResourceError::Contested);
            }
            if self.resources.tasks[..id.slot]
                .iter()
                .flatten()
                .any(|prior| prior.identity == task.identity && !prior.safe())
            {
                return Err(ResourceError::Blocked);
            }
            let limits = self.resources.limits.ok_or(ResourceError::Unsupported)?;
            if self.outstanding_resource_work() >= limits.max_outstanding {
                return Err(ResourceError::Limit);
            }
            if task.continuity.is_some() && continuity_facts.is_none() {
                return Err(ResourceError::Unavailable);
            }
            let serial = self
                .resources
                .last_dispatch_serial
                .checked_add(1)
                .ok_or(ResourceError::VersionExhausted)?;
            let plan = self.derive_continuity(id, task, continuity_facts)?;
            let expected = plan
                .as_ref()
                .map(|p| &p.condition)
                .unwrap_or(&task.condition);
            let condition = match (expected, &probe.condition) {
                (
                    WriteCondition::Present(expected),
                    DispatchCondition::Present {
                        bytes,
                        validator,
                        epoch,
                    },
                ) if expected.validator.as_ref() == Some(validator)
                    && expected.epoch == *epoch
                    && expected.bytes == *bytes =>
                {
                    DispatchCondition::Present {
                        epoch: epoch.clone(),
                        bytes: bytes.clone(),
                        validator: validator.clone(),
                    }
                }
                (WriteCondition::Absent, DispatchCondition::Absent) => DispatchCondition::Absent,
                _ => return Err(ResourceError::Conflict),
            };
            let dispatch = SaveDispatch {
                result_version_profile: task.result_version_profile,
                serial,
                continuity: self.materialize_continuity(
                    task,
                    serial,
                    plan.as_ref(),
                    continuity_facts,
                )?,
                task: id.clone(),
                registration: task.registration.clone(),
                identity: task.identity.clone(),
                nonce: Arc::new(()),
                payload: task.payload.clone(),
                condition,
                protection: task.protection,
                synchronize: task.synchronize,
            };
            let next_charge = plan.as_ref().map(|p| p.charge);
            let previous_charge = task.continuity.as_ref().map(|c| c.charge);
            let task = self.resources.tasks[id.slot]
                .as_mut()
                .expect("validated queued save");
            task.nonce = Some(dispatch.nonce.clone());
            task.launched_condition = Some(dispatch.condition.clone());
            if let Some(c) = &mut task.continuity {
                c.receipt = dispatch.continuity.clone();
                c.charge = next_charge.expect("validated continuity plan");
            }
            task.phase = SavePhase::Dispatched;
            task.queue_timing.finish(true);
            task.dispatch_serial = Some(serial);
            self.resources.last_dispatch_serial = serial;
            if let (Some(old), Some(new)) = (previous_charge, next_charge) {
                self.resources.continuity.charged = self.resources.continuity.charged - old + new;
            }
            Ok(dispatch)
        })();
        if matches!(result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture_service(recorder, Boundary::SaveDispatch, &result, timing.finish());
        result
    }
    /// Protected intake does not require current source read/write permission or
    /// a healthy authored owner. Exact original issuer/attempt still must match.
    pub fn record_save_evidence(
        &mut self,
        evidence: SaveEvidence,
        recorder: &mut Recorder,
    ) -> Result<EvidenceDisposition, ResourceError> {
        let timing = recorder.begin_service_timing();
        let boundary = if matches!(&evidence.kind, EvidenceKind::CausalEdge(_)) {
            Boundary::CausalSaveEvidence
        } else {
            Boundary::SaveEvidence
        };
        let result = (|| {
            let task = self.resource_task(&evidence.task)?;
            if !Arc::ptr_eq(&task.registration, &evidence.registration)
                || task
                    .nonce
                    .as_ref()
                    .is_none_or(|n| !Arc::ptr_eq(n, &evidence.nonce))
                || task.phase != SavePhase::Dispatched
            {
                return Err(ResourceError::Unavailable);
            }
            let task = self.resources.tasks[evidence.task.slot]
                .as_mut()
                .expect("validated original save");
            let result_claim = matches!(&evidence.kind, EvidenceKind::Result(v)
                if v.application != Application::Unknown || matches!(v.synchronization, Synchronization::Succeeded | Synchronization::Failed));
            let result = match evidence.kind {
                EvidenceKind::Result(result) => task.merge_causal_result(result),
                EvidenceKind::CausalEdge(edge) => task.merge_causal_edge(edge),
                EvidenceKind::NoFuture => {
                    let old = task.no_future;
                    task.no_future = true;
                    Ok(if old {
                        EvidenceDisposition::Duplicate
                    } else {
                        EvidenceDisposition::Accepted
                    })
                }
                EvidenceKind::Quiescence => {
                    let old = task.quiescent;
                    task.quiescent = true;
                    Ok(if old {
                        EvidenceDisposition::Duplicate
                    } else {
                        EvidenceDisposition::Accepted
                    })
                }
            };
            if result.is_ok() && result_claim {
                task.queue_timing.accepted_result();
            }
            result
        })();
        if matches!(result, Ok(EvidenceDisposition::Contradiction)) {
            recorder.record(Observation {
                boundary,
                disposition: Disposition::Unknown,
                service_time: timing.finish(),
            });
        } else {
            capture_service(recorder, boundary, &result, timing.finish());
        }
        result
    }
    pub fn save_outcome(
        &self,
        id: &SaveId,
        recorder: &mut Recorder,
    ) -> Result<SaveOutcome, ResourceError> {
        let timing = recorder.begin_service_timing();
        let result = self.resource_task(id).map(Task::snapshot);
        capture_service(recorder, Boundary::Outcome, &result, timing.finish());
        result
    }
    pub fn stop_save(&mut self, id: &SaveId, recorder: &mut Recorder) -> Result<(), ResourceError> {
        let timing = recorder.begin_service_timing();
        let result = (|| {
            self.resource_task(id)?;
            let task = self.resources.tasks[id.slot]
                .as_mut()
                .expect("validated save");
            task.stop_requested = true;
            if task.phase == SavePhase::Queued {
                task.queue_timing.finish(false);
                task.phase = SavePhase::Stopped;
                task.knowledge.data.application = Application::NotWritten;
                task.no_future = true;
                task.quiescent = true;
                if let Some(claim) = &mut task.baseline_claim {
                    claim.state = BaselineCapacityState::NotWritten;
                }
            }
            Ok(())
        })();
        capture_service(recorder, Boundary::Control, &result, timing.finish());
        result
    }
    pub fn publish_save_baseline(
        &mut self,
        id: &SaveId,
        recorder: &mut Recorder,
    ) -> Result<(), ResourceError> {
        self.publish_save_baseline_checked(id, None, recorder)
    }
    pub fn publish_save_baseline_with_pair(
        &mut self,
        id: &SaveId,
        pair: OrdinaryPairEvidence,
        recorder: &mut Recorder,
    ) -> Result<(), ResourceError> {
        self.publish_save_baseline_checked(id, Some(&pair), recorder)
    }
    fn publish_save_baseline_checked(
        &mut self,
        id: &SaveId,
        pair: Option<&OrdinaryPairEvidence>,
        recorder: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let timing = recorder.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            let task = self.resource_task(id)?;
            if !task.source_baseline_candidate() {
                return Err(ResourceError::Unsupported);
            }
            self.writer_catalog_current(
                task.catalog.as_ref(),
                &task.registration,
                &task.identity,
                task.synchronize,
                task.continuity.is_some(),
                recorder,
            )?;
            self.validate_source_capture(task)?;
            self.validate_ordinary_pair(&task.captured.document, pair)?;
            if self.resource_contested(&task.identity) {
                return Err(ResourceError::Contested);
            }
            if !task.qualified() {
                return Err(ResourceError::AcknowledgmentUnsatisfied);
            }
            let index = self
                .resources
                .bindings
                .iter()
                .position(|b| {
                    b.document == task.captured.document
                        && b.identity == task.identity
                        && b.generation == task.binding_generation
                        && Arc::ptr_eq(&b.registration, &task.registration)
                })
                .ok_or(ResourceError::Unavailable)?;
            if self.resources.tasks.iter().any(|t| {
                t.as_ref().is_some_and(|t| {
                    t.dispatch_serial > task.dispatch_serial
                        && t.source_baseline_candidate()
                        && t.captured.document == task.captured.document
                        && t.binding_generation == task.binding_generation
                        && t.identity == task.identity
                        && Arc::ptr_eq(&t.registration, &task.registration)
                        && t.qualified()
                })
            }) {
                return Err(ResourceError::Superseded);
            }
            let baseline = Baseline {
                epoch: task.knowledge.data.epoch.clone(),
                bytes: task.payload.clone(),
                validator: task.knowledge.data.version.clone(),
                anchor: Some(id.clone()),
            };
            let old = &self.resources.bindings[index].baseline;
            if old.as_ref().is_some_and(|old| {
                old.anchor == baseline.anchor
                    && old.bytes == baseline.bytes
                    && old.validator == baseline.validator
                    && old.epoch == baseline.epoch
            }) {
                return Ok(());
            }
            let removed = old
                .as_ref()
                .map_or(Ok(0), Baseline::root_payload_bytes)
                .map_err(|_| ResourceError::Limit)?;
            let added = baseline
                .root_payload_bytes()
                .map_err(|_| ResourceError::Limit)?;
            let pending = task
                .baseline_claim
                .as_ref()
                .is_some_and(|c| c.state == BaselineCapacityState::Pending);
            if task.purpose == SavePurpose::OrdinarySource && task.baseline_claim.is_none() {
                return Err(ResourceError::Faulted);
            }
            if pending {
                self.validate_root_promise(task)
                    .map_err(|_| ResourceError::Faulted)?;
            }
            if pending
                && added
                    > task
                        .required_baseline_claim_charge()
                        .map_err(|_| ResourceError::Faulted)?
            {
                return Err(ResourceError::Faulted);
            }
            self.check_root_replacement_projected(
                removed,
                added,
                BaselineRootProjection {
                    source_changed: Some((&task.captured.document, added)),
                    consume: pending.then_some(id),
                    ..BaselineRootProjection::default()
                },
                recorder,
            )
            .map_err(owner_error)?;
            let generation = self
                .next_publication_generation(pending.then_some(id))
                .map_err(owner_error)?;
            self.resources.bindings[index].baseline = Some(baseline);
            if pending {
                self.resources.tasks[id.slot]
                    .as_mut()
                    .expect("retained producer")
                    .baseline_claim
                    .as_mut()
                    .expect("checked claim")
                    .state = BaselineCapacityState::Installed;
            }
            self.generation = generation;
            Ok(())
        })();
        if matches!(result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture_service(recorder, Boundary::SaveBaseline, &result, timing.finish());
        result
    }
    /// Exact historical output comparison only; never reports current external
    /// freshness, crash durability or one aggregate success bit.
    /// Uses the configured resource blob ceiling for temporary output and twice
    /// that ceiling for combined comparison. For explicit independent bounds use
    /// `saved_state_with_budget`. Unbound documents need no serialization.
    pub fn saved_state(
        &self,
        document: &DocumentId,
        recorder: &mut Recorder,
    ) -> Result<SavedState, ResourceError> {
        self.saved_state_checked(document, None, recorder)
    }
    pub fn retire_save(
        &mut self,
        id: &SaveId,
        recorder: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let timing = recorder.begin_service_timing();
        let source_task = self
            .resource_task(id)
            .is_ok_and(|t| t.kind == ArtifactKind::Source);
        let disposed_claim = self.resource_task(id).is_ok_and(|task| {
            task.baseline_claim.is_some()
                && self
                    .retained_document(&task.captured.document)
                    .is_ok_and(|owner| owner.lifecycle.phase == LifecyclePhase::ProvenanceReleased)
        });
        let result = (|| {
            let task = self.resource_task(id)?;
            let operation = self.save_operation_retirement(id)?;
            let queue_timing = task.queue_timing;
            let queue_kind = task.kind;
            self.validate_released_inventory()?;
            let continuity_remaining = self.continuity_retirement_remaining(task)?;
            self.validate_export_retirement(id, task)?;
            if self.queued_source_producer_retained(id, task)? {
                return Err(ResourceError::Retained);
            }
            let current_context = task.source_baseline_candidate()
                && self.resources.bindings.iter().any(|binding| {
                    binding.document == task.captured.document
                        && binding.identity == task.identity
                        && binding.generation == task.binding_generation
                        && Arc::ptr_eq(&binding.registration, &task.registration)
                });
            let newer_qualified = self.resources.tasks.iter().any(|other| {
                other.as_ref().is_some_and(|other| {
                    other.dispatch_serial > task.dispatch_serial
                        && other.source_baseline_candidate()
                        && other.captured.document == task.captured.document
                        && other.binding_generation == task.binding_generation
                        && other.identity == task.identity
                        && Arc::ptr_eq(&other.registration, &task.registration)
                        && other.qualified()
                })
            });
            if !task.safe()
                || self.resource_contested(&task.identity)
                || (task.baseline_claim.as_ref().is_some_and(|c| c.state == BaselineCapacityState::Pending)
                    && (task.kind != ArtifactKind::Source || self.releasable_baseline_capacity(id, task).is_err()))
                || self.annotation_producer_retained(id, task)
                || self.documents.iter().flatten().any(|document| {
                    document.lifecycle.phase != LifecyclePhase::ProvenanceReleased
                        && document.retained_bindings.source.iter()
                            .chain(document.retained_bindings.source_history.iter())
                            .any(|binding| binding.baseline_anchor() == Some(id))
                })
                || !task.quiescent
                // No Weak/exported raw-Arc API exists for this private nonce.
                // External strong refs are owned dispatch/evidence leases only.
                // Count=1 under exclusive owner access proves their drain, not
                // physical completion, which has separate guards above.
                || task.nonce.as_ref().is_some_and(|nonce|Arc::strong_count(nonce)!=1)
                || (task.qualified() && current_context && !newer_qualified)
                || self
                    .resources
                    .bindings
                    .iter()
                    .any(|b| b.baseline_anchor() == Some(id))
            {
                return Err(ResourceError::Retained);
            }
            let owns_retiring_artifact = task.kind == ArtifactKind::Source
                || (task.catalog.is_some() && task.purpose == SavePurpose::AnnotationExport)
                || (task.baseline_claim.is_some()
                    && self.disposed_provenance_released(&task.captured.document)?);
            let retiring_artifact = owns_retiring_artifact.then(|| task.artifact.clone());
            let release = if let Some(artifact) = &retiring_artifact {
                let row = self
                    .resource_artifact(artifact)
                    .map_err(|_| ResourceError::Faulted)?;
                if row.bytes != task.payload || row.basis.source() != &task.captured {
                    return Err(ResourceError::Faulted);
                }
                match (&row.basis, task.kind) {
                    (ArtifactBasis::Source { serialization, .. }, ArtifactKind::Source)
                        if Some(*serialization) == task.serialization => {}
                    (
                        ArtifactBasis::Annotations { snapshot, encoder },
                        ArtifactKind::Annotations,
                    ) if Some(snapshot.set_version()) == task.captured_annotation_set
                        && Some(&encoder.context) == task.encoding_context.as_ref()
                        && self.knows_annotation_snapshot(snapshot) => {}
                    _ => return Err(ResourceError::Faulted),
                }
                if self.resources.tasks.iter().enumerate().any(|(slot, t)| {
                    slot != id.slot && t.as_ref().is_some_and(|t| t.artifact == *artifact)
                }) {
                    return Err(ResourceError::Retained);
                }
                let bytes = row.held_size().ok_or(ResourceError::Faulted)?;
                let Some(remaining) = self.resources.held_artifact_bytes.checked_sub(bytes) else {
                    self.faulted = true;
                    return Err(ResourceError::Faulted);
                };
                Some((artifact.slot, remaining))
            } else {
                None
            };
            if let Some(index) = operation {
                self.retire_save_operation(index);
            }
            self.resources.tasks[id.slot] = None;
            self.retire_save_queue_timing(queue_kind, queue_timing);
            self.resources.continuity.charged = continuity_remaining;
            if let Some((slot, remaining)) = release {
                self.resources.artifacts[slot] = None;
                self.resources.held_artifact_bytes = remaining;
            }
            if operation.is_some() {
                capture(
                    recorder,
                    Boundary::SaveOperationRetirement,
                    &Ok::<(), ResourceError>(()),
                );
            }
            Ok(())
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(
            recorder,
            if disposed_claim {
                Boundary::DisposedWriteRetirement
            } else if source_task {
                Boundary::SaveProducerRetirement
            } else {
                Boundary::Control
            },
            &result,
            timing.finish(),
        );
        result
    }
    pub(super) fn can_dispose_resource(&self, document: &DocumentId) -> Result<(), ResourceError> {
        if self
            .resources
            .annotation_bindings
            .iter()
            .any(|b| b.document == *document)
        {
            return Err(ResourceError::Retained);
        }
        if self
            .resources
            .tasks
            .iter()
            .flatten()
            .any(|t| t.captured.document == *document && (!t.safe() || !t.quiescent))
        {
            return Err(ResourceError::Retained);
        }
        Ok(())
    }
    pub(super) fn remove_resource_binding(&mut self, document: &DocumentId) {
        self.resources.bindings.retain(|b| b.document != *document);
    }
}
