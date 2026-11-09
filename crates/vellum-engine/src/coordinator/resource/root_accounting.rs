use super::*;
use crate::coordinator::root_accounting::{Bytes, Result, RootAccountingErrorV1};
#[derive(Clone)]
pub(in crate::coordinator) struct ResourceRootIdentity {
    header: RootHeader,
    attachment: Option<Arc<()>>,
    catalog: catalog::RootStamp,
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct RootHeader {
    providers: usize,
    encoders: usize,
    restorers: usize,
    continuity: Option<SourceContinuityLimitsV1>,
    operations: Option<SaveOperationLimitsV1>,
    save_stopped: bool,
    source_save_stopped: bool,
    read_stopped: bool,
    reader_closed: bool,
    writer_closed: bool,
    dispatch_serial: u64,
    attachment_generation: u64,
}
impl PartialEq for ResourceRootIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.header == other.header
            && match (&self.attachment, &other.attachment) {
                (None, None) => true,
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                _ => false,
            }
            && self.catalog == other.catalog
    }
}
impl Eq for ResourceRootIdentity {}
#[derive(Clone, Copy, Default)]
pub(in crate::coordinator) struct BaselineRootProjection<'a> {
    pub(super) changed: Option<(&'a DocumentId, usize)>,
    pub(super) source_changed: Option<(&'a DocumentId, usize)>,
    pub(super) consume: Option<&'a SaveId>,
    pub(super) admission: Option<&'a Task>,
}
impl SourceCoordinator {
    pub(super) fn validate_root_promise(&self, task: &Task) -> Result<()> {
        let correspondence = match task.kind {
            ArtifactKind::Annotations => {
                task.purpose == SavePurpose::OrdinaryAnnotations
                    && task.annotation_association.is_some()
                    && self
                        .current_annotation_association(&task.captured.document)
                        .as_ref()
                        == task.annotation_association.as_ref()
                    && self
                        .resources
                        .annotation_bindings
                        .iter()
                        .filter(|b| b.document == task.captured.document)
                        .count()
                        == 1
            }
            ArtifactKind::Source => {
                task.purpose == SavePurpose::OrdinarySource
                    && self
                        .resources
                        .bindings
                        .iter()
                        .filter(|b| b.document == task.captured.document)
                        .count()
                        == 1
                    && self.resources.bindings.iter().any(|b| {
                        b.document == task.captured.document
                            && b.generation == task.binding_generation
                            && b.identity == task.identity
                            && Arc::ptr_eq(&b.registration, &task.registration)
                    })
            }
        };
        if !correspondence
            || self.document(&task.captured.document).is_err()
            || !task
                .baseline_claim
                .as_ref()
                .is_some_and(|c| c.state == BaselineCapacityState::Pending)
        {
            return Err(RootAccountingErrorV1::Faulted);
        }
        if task.required_baseline_claim_charge()?
            != task
                .baseline_claim
                .as_ref()
                .ok_or(RootAccountingErrorV1::Faulted)?
                .charge
        {
            return Err(RootAccountingErrorV1::Faulted);
        }
        Ok(())
    }
    fn pending_root_tasks<'a>(
        &'a self,
        p: BaselineRootProjection<'a>,
    ) -> impl Iterator<Item = &'a Task> {
        self.resources
            .tasks
            .iter()
            .enumerate()
            .filter_map(move |(slot, t)| {
                if p.consume.is_some_and(|id| id.slot == slot) {
                    None
                } else {
                    t.as_ref()
                }
            })
            .chain(p.admission)
            .filter(|t| {
                t.baseline_claim
                    .as_ref()
                    .is_some_and(|c| c.state == BaselineCapacityState::Pending)
            })
    }
    pub(in crate::coordinator) fn pending_baseline_root_growth(
        &self,
        p: BaselineRootProjection<'_>,
    ) -> Result<usize> {
        if let Some(id) = p.consume
            && (!Arc::ptr_eq(&id.owner, &self.namespace)
                || self
                    .resources
                    .tasks
                    .get(id.slot)
                    .and_then(Option::as_ref)
                    .is_none())
        {
            return Err(RootAccountingErrorV1::Faulted);
        }
        if let Some(id) = p.consume {
            self.validate_root_promise(
                self.resources.tasks[id.slot]
                    .as_ref()
                    .ok_or(RootAccountingErrorV1::Faulted)?,
            )?;
        }
        if p.admission.is_some_and(|t| {
            !t.baseline_claim
                .as_ref()
                .is_some_and(|c| c.state == BaselineCapacityState::Pending)
        }) {
            return Err(RootAccountingErrorV1::Faulted);
        }
        for task in self.pending_root_tasks(p) {
            self.validate_root_promise(task)?;
        }
        if let Some((document, _)) = p.changed
            && self
                .resources
                .annotation_bindings
                .iter()
                .filter(|b| b.document == *document)
                .count()
                != 1
        {
            return Err(RootAccountingErrorV1::Faulted);
        }
        let mut growth = Bytes::default();
        for binding in &self.resources.annotation_bindings {
            let current =
                if let Some((_, bytes)) = p.changed.filter(|(d, _)| **d == binding.document) {
                    bytes
                } else {
                    binding
                        .baseline
                        .as_ref()
                        .map_or(Ok(0), AnnotationBaselineSlot::root_payload_bytes)?
                };
            let mut required = current;
            for task in self.pending_root_tasks(p).filter(|t| {
                t.kind == ArtifactKind::Annotations && t.captured.document == binding.document
            }) {
                let future = task.future_baseline_root_bytes()?;
                required = required.max(future);
            }
            growth.add(required - current)?;
        }
        if let Some((document, _)) = p.source_changed
            && self
                .resources
                .bindings
                .iter()
                .filter(|b| b.document == *document)
                .count()
                != 1
        {
            return Err(RootAccountingErrorV1::Faulted);
        }
        for binding in &self.resources.bindings {
            let current = if let Some((_, bytes)) =
                p.source_changed.filter(|(d, _)| **d == binding.document)
            {
                bytes
            } else {
                binding
                    .baseline
                    .as_ref()
                    .map_or(Ok(0), Baseline::root_payload_bytes)?
            };
            let mut required = current;
            for task in self.pending_root_tasks(p).filter(|t| {
                t.kind == ArtifactKind::Source && t.captured.document == binding.document
            }) {
                required = required.max(task.future_baseline_root_bytes()?);
            }
            growth.add(required - current)?;
        }
        Ok(growth.finish())
    }
}
impl Task {
    fn future_baseline_root_bytes(&self) -> Result<usize> {
        if self.kind == ArtifactKind::Source {
            return future_source_baseline_payload(
                &self.payload,
                &self.registration,
                self.result_version_profile,
            );
        }
        future_baseline_payload(
            &self.payload,
            self.annotation_association
                .as_ref()
                .ok_or(RootAccountingErrorV1::Faulted)?,
            &self.registration,
        )
    }
    pub(super) fn required_baseline_claim_charge(&self) -> Result<usize> {
        match self.kind {
            ArtifactKind::Annotations => baseline_claim_charge(self.future_baseline_root_bytes()?),
            ArtifactKind::Source => {
                source_baseline_claim_charge(self.future_baseline_root_bytes()?)
            }
        }
    }
}
pub(super) fn future_source_baseline_payload(
    payload: &Blob,
    registration: &Registration,
    profile: SaveResultVersionProfileV1,
) -> Result<usize> {
    let mut bytes = Bytes::default();
    bytes.add(payload.0.len())?;
    bytes.add(registration.limits.max_validator_bytes)?;
    if profile == SaveResultVersionProfileV1::EpochToken {
        bytes.add(registration.limits.max_validator_bytes)?;
    }
    Ok(bytes.finish())
}
pub(super) fn source_baseline_claim_charge(payload_bytes: usize) -> Result<usize> {
    let mut bytes = Bytes::default();
    bytes.rows::<Baseline>(1)?;
    bytes.rows::<BaselineClaim>(1)?;
    bytes.add(payload_bytes)?;
    Ok(bytes.finish())
}
pub(super) fn future_baseline_payload(
    payload: &Blob,
    captured: &AnnotationAssociation,
    registration: &Registration,
) -> Result<usize> {
    let mut bytes = Bytes::default();
    association(&mut bytes, captured)?;
    bytes.add(payload.0.len())?;
    bytes.add(registration.limits.max_validator_bytes)?;
    Ok(bytes.finish())
}
pub(super) fn baseline_claim_charge(payload_bytes: usize) -> Result<usize> {
    let mut bytes = Bytes::default();
    bytes.rows::<AnnotationBaselineSlot>(1)?;
    bytes.rows::<BaselineClaim>(1)?;
    bytes.add(payload_bytes)?;
    Ok(bytes.finish())
}
impl Baseline {
    pub(super) fn root_payload_bytes(&self) -> Result<usize> {
        let mut bytes = Bytes::default();
        if let Some(epoch) = &self.epoch {
            bytes.add(epoch.0.len())?;
        }
        bytes.add(self.bytes.0.len())?;
        if let Some(validator) = &self.validator {
            bytes.add(validator.0.len())?;
        }
        Ok(bytes.finish())
    }
}
impl AnnotationBaselineSlot {
    pub(super) fn root_payload_bytes(&self) -> Result<usize> {
        let mut bytes = Bytes::default();
        match self {
            Self::Written(baseline) => {
                association(&mut bytes, &baseline.association)?;
                bytes.add(baseline.bytes.0.len())?;
                if let Some(version) = &baseline.external_version {
                    bytes.add(version.0.len())?;
                }
            }
            Self::Loaded(baseline) => {
                association(&mut bytes, &baseline.association)?;
                bytes.add(baseline.bytes.0.len())?;
                bytes.add(baseline.version.0.len())?;
            }
        }
        Ok(bytes.finish())
    }
}
pub(super) fn annotation_binding_bytes(key: &BindingKey) -> Result<usize> {
    let mut bytes = Bytes::default();
    bytes.rows::<AnnotationBinding>(1)?;
    bytes.add(key.identity.key.0.len())?;
    Ok(bytes.finish())
}
pub(super) fn registration_bytes<T>() -> Result<usize> {
    let mut bytes = Bytes::default();
    bytes.rows::<Arc<T>>(1)?;
    bytes.rows::<T>(1)?;
    Ok(bytes.finish())
}
pub(super) fn source_load_bytes(read: &ResourceRead) -> Result<usize> {
    let mut bytes = Bytes::default();
    if let Some(epoch) = &read.epoch {
        bytes.add(epoch.0.len())?;
    }
    bytes.rows::<Binding>(1)?;
    bytes.add(read.identity.key.0.len())?;
    bytes.add(read.bytes.0.len())?;
    bytes.add(read.validator.0.len())?;
    Ok(bytes.finish())
}
fn association(bytes: &mut Bytes, value: &AnnotationAssociation) -> Result<()> {
    bytes.add(value.binding.identity.key.0.len())?;
    if let Some(source) = &value.source_binding {
        bytes.add(source.identity.key.0.len())?;
    }
    Ok(())
}
fn source_binding(bytes: &mut Bytes, value: &Binding) -> Result<()> {
    bytes.add(value.identity.key.0.len())?;
    bytes.add(
        value
            .baseline
            .as_ref()
            .map_or(Ok(0), Baseline::root_payload_bytes)?,
    )?;
    Ok(())
}
fn annotation_binding(bytes: &mut Bytes, value: &AnnotationBinding) -> Result<()> {
    bytes.add(value.key.identity.key.0.len())?;
    bytes.add(
        value
            .baseline
            .as_ref()
            .map_or(Ok(0), AnnotationBaselineSlot::root_payload_bytes)?,
    )?;
    Ok(())
}
impl RetainedBindings {
    pub(super) fn source_history_bytes(&self) -> Result<usize> {
        let mut bytes = Bytes::default();
        bytes.rows::<Binding>(self.source_history.len())?;
        for binding in &self.source_history {
            source_binding(&mut bytes, binding)?;
        }
        Ok(bytes.finish())
    }
    pub(in crate::coordinator) fn root_payload_bytes(&self) -> Result<usize> {
        let mut bytes = Bytes::default();
        bytes.add(self.source_history_bytes()?)?;
        if let Some(binding) = &self.source {
            source_binding(&mut bytes, binding)?;
        }
        if let Some(binding) = &self.annotations {
            annotation_binding(&mut bytes, binding)?;
        }
        Ok(bytes.finish())
    }
}
impl ResourceState {
    pub(in crate::coordinator) fn root_identity(
        &self,
        owner: &Arc<()>,
    ) -> Result<ResourceRootIdentity> {
        self.writer_lifecycle_intact()
            .map_err(|_| RootAccountingErrorV1::Faulted)?;
        let Self {
            catalog: _,
            queue_timing: _, // Fixed diagnostic scalars; no semantic root/payload.
            limits: _,       // Immutable constructor profile, bound by owner incarnation.
            providers: _,
            encoders: _,
            restorers: _,
            bindings: _,
            annotation_bindings: _,
            source_export_reservations: _,
            continuity: _,
            operations: _,
            last_dispatch_serial: _,
            annotation_save_admission_stopped: _,
            source_save_admission_stopped: _,
            annotation_read_admission_stopped: _,
            annotation_reader_closed: _,
            writer_closed: _,
            read_baseline_attachment: _,
            read_baseline_generation: _,
            // Separate work/retention metadata, excluded by root_bytes as well.
            source_reader: _,
            read_operations: _,
            observation: _,
            tasks: _,
            artifacts: _,
            source_preparation: _,
            held_artifact_bytes: _,
            reads: _,
            held_read_bytes: _,
        } = self;
        if self.read_baseline_attachment.is_some() && self.read_baseline_generation == u64::MAX {
            return Err(RootAccountingErrorV1::Faulted);
        }
        let attachment = self
            .read_baseline_attachment
            .as_ref()
            .map(|a| {
                a.current(owner)
                    .cloned()
                    .map_err(|_| RootAccountingErrorV1::Faulted)
            })
            .transpose()?;
        let catalog = self
            .catalog
            .stamp()
            .map_err(|_| RootAccountingErrorV1::Faulted)?;
        Ok(ResourceRootIdentity {
            // Registrations are append-only while their owner is active. The
            // reader's absorbing closed stamp prevents ABA when its restorer
            // registry is dropped at whole-owner retirement. Prefix lengths are
            // native identity positions, not hashes or caller-supplied counts.
            header: RootHeader {
                providers: self.providers.len(),
                encoders: self.encoders.len(),
                restorers: self.restorers.len(),
                continuity: self.continuity.limits,
                operations: self.operations.root_profile(),
                save_stopped: self.annotation_save_admission_stopped,
                source_save_stopped: self.source_save_admission_stopped,
                read_stopped: self.annotation_read_admission_stopped,
                reader_closed: self.annotation_reader_closed,
                writer_closed: self.writer_closed,
                dispatch_serial: self.last_dispatch_serial,
                attachment_generation: self.read_baseline_generation,
            },
            // Capture the original incarnation, not Some/None or an address cast:
            // detach/re-attach with identical bounds cannot produce an ABA match.
            // The lifecycle generation above also stamps absent -> active -> absent.
            attachment,
            catalog,
        })
    }
    pub(in crate::coordinator) fn root_bytes(&self) -> Result<usize> {
        self.writer_lifecycle_intact()
            .map_err(|_| RootAccountingErrorV1::Faulted)?;
        let Self {
            catalog: _,
            queue_timing: _, // No binding, immutable capture or document lease.
            source_reader: _,
            read_operations: _,
            annotation_read_admission_stopped: _,
            annotation_reader_closed: _,
            writer_closed: _,
            observation: _,
            operations: _,
            continuity: _,
            limits: _,
            providers: _,
            bindings: _,
            annotation_bindings: _,
            tasks: _,
            encoders: _,
            artifacts: _,
            source_preparation: _,
            source_export_reservations: _,
            held_artifact_bytes: _,
            last_dispatch_serial: _,
            annotation_save_admission_stopped: _,
            source_save_admission_stopped: _,
            reads: _,
            held_read_bytes: _,
            restorers: _,
            read_baseline_attachment: _,
            read_baseline_generation: _,
        } = self;
        let mut bytes = Bytes::default();
        // Explicit projection excludes pending tasks, artifacts and read captures.
        bytes.rows::<(
            Option<SourceContinuityLimitsV1>,
            bool,
            bool,
            Option<SaveOperationLimitsV1>,
            bool,
            Option<ResourceLimits>,
            Vec<Arc<Registration>>,
            Vec<Binding>,
            Vec<AnnotationBinding>,
            Vec<source_export::Reservation>,
            Vec<Arc<EncoderRegistration>>,
            Vec<Arc<RestorationRegistration>>,
            u64,
            bool,
            Option<ReadBaselineAttachment>,
            u64,
            catalog::State,
        )>(1)?;
        bytes.add(
            self.catalog
                .root_bytes()
                .map_err(|_| RootAccountingErrorV1::Overflow)?,
        )?;
        bytes.rows::<Arc<Registration>>(self.providers.len())?;
        bytes.rows::<Registration>(self.providers.len())?;
        bytes.rows::<Arc<EncoderRegistration>>(self.encoders.len())?;
        bytes.rows::<EncoderRegistration>(self.encoders.len())?;
        bytes.rows::<Arc<RestorationRegistration>>(self.restorers.len())?;
        bytes.rows::<RestorationRegistration>(self.restorers.len())?;
        bytes.rows::<Binding>(self.bindings.len())?;
        for binding in &self.bindings {
            source_binding(&mut bytes, binding)?;
        }
        bytes.rows::<AnnotationBinding>(self.annotation_bindings.len())?;
        bytes.rows::<source_export::Reservation>(self.source_export_reservations.len())?;
        for r in &self.source_export_reservations {
            bytes.add(r.identity.key.0.len())?;
        }
        for binding in &self.annotation_bindings {
            annotation_binding(&mut bytes, binding)?;
        }
        Ok(bytes.finish())
    }
}
