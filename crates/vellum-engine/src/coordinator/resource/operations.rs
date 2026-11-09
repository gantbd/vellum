//! Shared writer-owned one-attempt parents. Admission metadata is not an effect
//! acknowledgment, terminal receipt, invocation executor or retry procedure.
use super::*;
mod annotations;
mod shutdown_inventory;
#[cfg(test)]
pub(in crate::coordinator::resource) mod tests;
pub use annotations::{
    ExpectedSaveBindingV1, OwnedAnnotationSaveRequestV1, TrustedAnnotationSaveOperationReleaseV1,
    TrustedOrdinaryAnnotationSaveAdmissionV1,
};
#[cfg(test)]
pub(super) use tests::admit as test_admit_source_operation;

/// Conservative native logical metadata envelopes, not wire bytes, RSS or an
/// allocator guarantee. The lifetime parent envelope is independent of payloads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveOperationLimitsV1 {
    pub max_issued: usize,
    pub max_label_bytes: usize,
    pub max_capture_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_store_bytes: usize,
}
#[derive(Clone, Copy)]
pub struct SaveOperationOriginV1<'a> {
    pub operation: &'a [u8],
    pub effect: &'a [u8],
    pub attempt: &'a [u8],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveOperationConditionV1 {
    Pinned,
    OwnChain,
}
pub struct OwnedSourceSaveRequestV1<'a> {
    pub origin: SaveOperationOriginV1<'a>,
    pub source: &'a DocumentVersion,
    pub expected_serialization: u64,
    pub expected_binding: u64,
    pub target: &'a SourceBindingTargetV1,
    pub requirements: SaveRequirements,
    pub condition: SaveOperationConditionV1,
}
/// Same complete native context value used by other engine domains. This does
/// not make persistence a settings operation or authenticate the supplied facts.
pub struct TrustedSaveOperationReleaseV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub write_allowed: Option<bool>,
    pub source_readable: Option<bool>,
    pub disposition_readable: Option<bool>,
    pub retained: Option<bool>,
    pub reply_ready: Option<bool>,
    pub transfer_ready: Option<bool>,
    pub resources_ready: Option<bool>,
}
/// Common current-fact packet, excluding export destination/create/overwrite
/// fields, plus the four ordinary native publication premises (§16.3.4).
pub struct TrustedOrdinarySourceSaveAdmissionV1<'a> {
    pub current: &'a TrustedSourceExportFactsV1,
    pub owner_control_allowed: Option<bool>,
    pub preparation_reserved: Option<bool>,
    pub ordinary_bindings_current: Option<bool>,
    pub ordinary_identity_current: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveOperationAdmissionStatusV1 {
    Admitted,
    ObserveExisting,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveOperationAdmissionV1 {
    pub status: SaveOperationAdmissionStatusV1,
    pub task: SaveId,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveOperationUsageV1 {
    pub issued: usize,
    pub owned: usize,
    pub retired: usize,
    pub reserved_store_bytes: usize,
}
#[derive(Debug, Default)]
pub(super) struct State {
    limits: Option<SaveOperationLimitsV1>,
    rows: Vec<Row>,
    reserved: usize,
}
impl State {
    pub(super) fn retired_empty(&self) -> bool {
        let Self {
            limits,
            rows,
            reserved,
        } = self;
        limits.is_none() && rows.is_empty() && *reserved == 0
    }
    pub(super) fn root_profile(&self) -> Option<SaveOperationLimitsV1> {
        let Self {
            limits,
            rows: _,
            reserved: _,
        } = self;
        *limits
    }
}
#[derive(Debug)]
pub(super) struct Row {
    operation: Blob,
    capture: Option<Capture>,
}
#[derive(Debug)]
struct Capture {
    catalog: Option<Arc<WriteCatalogCaptureV1>>,
    effect: Blob,
    attempt: Blob,
    source: DocumentVersion,
    flavor: Flavor,
    binding: u64,
    identity: Identity,
    registration: Arc<Registration>,
    requirements: SaveRequirements,
    context: SettingsExecutionContextV1,
    task: SaveId,
}
#[derive(Debug)]
enum Flavor {
    Source {
        serialization: u64,
        condition: SaveOperationConditionV1,
    },
    Annotations {
        set: u64,
        encoding: AnnotationEncodingContext,
        association: AnnotationAssociation,
        before_bytes: Blob,
        before_validator: Blob,
    },
}
impl Flavor {
    fn extra_bytes(&self) -> Option<usize> {
        match self {
            Self::Source { .. } => Some(0),
            Self::Annotations {
                association,
                before_bytes,
                before_validator,
                ..
            } => before_bytes
                .0
                .len()
                .checked_add(before_validator.0.len())?
                .checked_add(association.binding.identity.key.0.len())?
                .checked_add(
                    association
                        .source_binding
                        .as_ref()
                        .map_or(0, |b| b.identity.key.0.len()),
                ),
        }
    }
}
fn release_current(f: &TrustedSaveOperationReleaseV1<'_>) -> bool {
    [
        f.authorized,
        f.scope,
        f.continuous,
        f.live,
        f.not_stopped,
        f.write_allowed,
        f.source_readable,
        f.disposition_readable,
        f.retained,
        f.reply_ready,
        f.transfer_ready,
        f.resources_ready,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn admission_current(f: &TrustedOrdinarySourceSaveAdmissionV1<'_>) -> bool {
    let c = f.current;
    [
        c.authorized,
        c.scope,
        c.continuous,
        c.live,
        c.not_stopped,
        c.source_readable,
        c.write_allowed,
        c.binding_coverage_complete,
        c.writer_coverage_complete,
        c.provider_available,
        c.resources_ready,
        c.launch_allowed,
        c.identity_allowed,
        c.companions_ready,
        c.versions_available,
        c.reply_ready,
        f.owner_control_allowed,
        f.preparation_reserved,
        f.ordinary_bindings_current,
        f.ordinary_identity_current,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn capsule_bytes(
    operation: usize,
    effect: usize,
    attempt: usize,
    key: usize,
    context: &SettingsExecutionContextV1,
) -> Option<usize> {
    [operation, effect, attempt, key]
        .into_iter()
        .chain(settings::basis::labels(context).map(str::len))
        .try_fold(std::mem::size_of::<Row>(), usize::checked_add)
}
impl Capture {
    fn matches(&self, r: &OwnedSourceSaveRequestV1<'_>, c: &SettingsExecutionContextV1) -> bool {
        self.effect.0.as_ref() == r.origin.effect
            && self.attempt.0.as_ref() == r.origin.attempt
            && self.source == *r.source
            && matches!(self.flavor, Flavor::Source { serialization, condition }
                if serialization == r.expected_serialization && condition == r.condition)
            && self.binding == r.expected_binding
            && self.identity == r.target.identity
            && Arc::ptr_eq(&self.registration, &r.target.registration)
            && self.requirements == r.requirements
            && self.context == *c
    }
    fn intact(&self, task: &Task) -> bool {
        WriteCatalogCaptureV1::same(self.catalog.as_ref(), task.catalog.as_ref())
            && task.captured == self.source
            && task.binding_generation == self.binding
            && task.identity == self.identity
            && Arc::ptr_eq(&task.registration, &self.registration)
            && task.protection == self.requirements.protection
            && task.synchronize
                == (self.requirements.acknowledgment == Acknowledgment::Synchronized)
            && match &self.flavor {
                Flavor::Source {
                    serialization,
                    condition,
                } => {
                    task.serialization
                        .is_some_and(|s| s.generation == *serialization)
                        && task.continuity.is_some()
                            == (*condition == SaveOperationConditionV1::OwnChain)
                        && task.kind == ArtifactKind::Source
                        && task.purpose == SavePurpose::OrdinarySource
                }
                Flavor::Annotations {
                    set,
                    encoding,
                    association,
                    before_bytes,
                    before_validator,
                } => {
                    task.kind == ArtifactKind::Annotations
                        && task.purpose == SavePurpose::OrdinaryAnnotations
                        && task.serialization.is_none()
                        && task.continuity.is_none()
                        && task.baseline_claim.is_some()
                        && task.captured_annotation_set == Some(*set)
                        && task.encoding_context.as_ref() == Some(encoding)
                        && task.annotation_association.as_ref() == Some(association)
                        && matches!(&task.condition, WriteCondition::Present(b)
                            if b.epoch.is_none() && b.bytes == *before_bytes && b.validator.as_ref() == Some(before_validator))
                }
            }
    }
}
impl SourceCoordinator {
    pub(super) fn save_operation_document_pins(
        &self,
        document: &DocumentId,
    ) -> Result<usize, ResourceError> {
        self.validate_save_operations()?;
        Ok(self
            .resources
            .operations
            .rows
            .iter()
            .filter(|row| {
                row.capture
                    .as_ref()
                    .is_some_and(|capture| capture.source.document == *document)
            })
            .count())
    }
    pub(super) fn install_save_parent(&mut self, parent: Option<Row>) {
        if let Some(parent) = parent {
            self.resources.operations.rows.push(parent);
        }
    }
    /// Validate the already-owned parent before child issuance. This is a native
    /// installation prerequisite, not authority inferred from origin labels.
    pub(super) fn prepare_save_parent(
        &self,
        task: &mut Task,
        parent: Option<&Row>,
    ) -> Result<(), ResourceError> {
        let Some(parent) = parent else { return Ok(()) };
        self.validate_save_operations()?;
        let s = &self.resources.operations;
        let limits = s.limits.ok_or(ResourceError::Unsupported)?;
        let c = parent.capture.as_ref().ok_or(ResourceError::Faulted)?;
        if s.rows.len() >= limits.max_issued
            || s.rows.len() >= s.rows.capacity()
            || parent.operation.0.is_empty()
            || parent.operation.0.len() > limits.max_label_bytes
            || s.rows.iter().any(|row| row.operation == parent.operation)
            || c.task.slot != self.resources.tasks.len()
            || !Arc::ptr_eq(&c.task.owner, &self.namespace)
            || !c.intact(task)
            || task.operation.is_some()
        {
            return Err(ResourceError::Faulted);
        }
        task.operation = Some(s.rows.len());
        Ok(())
    }
    pub(super) fn owned_save_observation_kind(
        &self,
        id: &SaveId,
        context: &SettingsExecutionContextV1,
    ) -> Result<ArtifactKind, ResourceError> {
        self.validate_save_operations()?;
        let task = self.resource_task(id)?;
        let index = task.operation.ok_or(ResourceError::Unavailable)?;
        let original = self
            .resources
            .operations
            .rows
            .get(index)
            .and_then(|r| r.capture.as_ref())
            .ok_or(ResourceError::Unavailable)?;
        if original.task != *id || original.context != *context {
            return Err(ResourceError::Unavailable);
        }
        Ok(task.kind)
    }
    pub(super) fn validate_save_operations(&self) -> Result<(), ResourceError> {
        let s = &self.resources.operations;
        let Some(limits) = s.limits else {
            return if s.rows.is_empty()
                && s.reserved == 0
                && self
                    .resources
                    .tasks
                    .iter()
                    .flatten()
                    .all(|t| t.operation.is_none())
            {
                Ok(())
            } else {
                Err(ResourceError::Faulted)
            };
        };
        let reserved = limits
            .max_issued
            .checked_mul(limits.max_capture_bytes)
            .and_then(|n| n.checked_add(std::mem::size_of::<State>()))
            .ok_or(ResourceError::Faulted)?;
        if reserved != s.reserved
            || reserved > limits.max_store_bytes
            || s.rows.len() > limits.max_issued
        {
            return Err(ResourceError::Faulted);
        }
        for (index, row) in s.rows.iter().enumerate() {
            if row.operation.0.is_empty()
                || row.operation.0.len() > limits.max_label_bytes
                || s.rows[..index]
                    .iter()
                    .any(|old| old.operation == row.operation)
            {
                return Err(ResourceError::Faulted);
            }
            if let Some(c) = &row.capture {
                let task = self
                    .resource_task(&c.task)
                    .map_err(|_| ResourceError::Faulted)?;
                if task.operation != Some(index)
                    || !c.intact(task)
                    || capsule_bytes(
                        row.operation.0.len(),
                        c.effect.0.len(),
                        c.attempt.0.len(),
                        c.identity.key.0.len(),
                        &c.context,
                    )
                    .and_then(|n| n.checked_add(c.flavor.extra_bytes()?))
                    .and_then(|n| {
                        n.checked_add(
                            c.catalog
                                .as_ref()
                                .map(|c| c.native_bytes())
                                .transpose()
                                .ok()?
                                .unwrap_or(0),
                        )
                    })
                    .is_none_or(|n| n > limits.max_capture_bytes)
                {
                    return Err(ResourceError::Faulted);
                }
            } else if std::mem::size_of::<Row>()
                .checked_add(row.operation.0.len())
                .is_none_or(|n| n > limits.max_capture_bytes)
            {
                return Err(ResourceError::Faulted);
            }
        }
        for (slot, t) in self.resources.tasks.iter().enumerate() {
            if let Some(index) = t.as_ref().and_then(|t| t.operation)
                && s.rows
                    .get(index)
                    .and_then(|r| r.capture.as_ref())
                    .is_none_or(|c| {
                        c.task.slot != slot || !Arc::ptr_eq(&c.task.owner, &self.namespace)
                    })
            {
                return Err(ResourceError::Faulted);
            }
        }
        Ok(())
    }
    pub(super) fn save_operation_retirement(
        &self,
        id: &SaveId,
    ) -> Result<Option<usize>, ResourceError> {
        self.validate_save_operations()?;
        Ok(self.resource_task(id)?.operation)
    }
    pub(super) fn retire_save_operation(&mut self, index: usize) {
        // Called only after all actual child retirement preflights, in the same
        // serialized infallible assignment section. The origin remains spent.
        self.resources.operations.rows[index].capture = None;
    }
    pub fn configure_save_operations_v1(
        &mut self,
        limits: SaveOperationLimitsV1,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            let resources = self.resources.limits.ok_or(ResourceError::Unsupported)?;
            self.validate_save_operations()?;
            if let Some(old) = self.resources.operations.limits {
                return if old == limits {
                    Ok(())
                } else {
                    Err(ResourceError::Unavailable)
                };
            }
            self.ensure_admission_unfenced().map_err(owner_error)?;
            if limits.max_issued > resources.max_issued_tasks
                || (limits.max_issued != 0 && limits.max_capture_bytes < std::mem::size_of::<Row>())
            {
                return Err(ResourceError::Limit);
            }
            let reserved = limits
                .max_issued
                .checked_mul(limits.max_capture_bytes)
                .and_then(|n| n.checked_add(std::mem::size_of::<State>()))
                .filter(|n| *n <= limits.max_store_bytes)
                .ok_or(ResourceError::Limit)?;
            self.resources
                .operations
                .rows
                .try_reserve_exact(limits.max_issued)
                .map_err(|_| ResourceError::Capacity)?;
            self.resources.operations.limits = Some(limits);
            self.resources.operations.reserved = reserved;
            Ok(())
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture(r, Boundary::SaveOperationConfiguration, &result);
        result
    }
    pub fn save_operation_usage_v1(&self) -> SaveOperationUsageV1 {
        let s = &self.resources.operations;
        let owned = s.rows.iter().filter(|r| r.capture.is_some()).count();
        SaveOperationUsageV1 {
            issued: s.rows.len(),
            owned,
            retired: s.rows.len() - owned,
            reserved_store_bytes: s.reserved,
        }
    }
    pub fn request_owned_source_save_v1(
        &mut self,
        request: &OwnedSourceSaveRequestV1<'_>,
        release: &TrustedSaveOperationReleaseV1<'_>,
        admission: Option<&TrustedOrdinarySourceSaveAdmissionV1<'_>>,
        pair: Option<&OrdinaryPairEvidence>,
        r: &mut Recorder,
    ) -> Result<SaveOperationAdmissionV1, ResourceError> {
        self.request_owned_source_save_catalog((request, None), release, admission, pair, r)
    }
    pub(in crate::coordinator) fn request_owned_source_save_catalog(
        &mut self,
        input: (
            &OwnedSourceSaveRequestV1<'_>,
            Option<&CatalogWriterBindingV1>,
        ),
        release: &TrustedSaveOperationReleaseV1<'_>,
        admission: Option<&TrustedOrdinarySourceSaveAdmissionV1<'_>>,
        pair: Option<&OrdinaryPairEvidence>,
        r: &mut Recorder,
    ) -> Result<SaveOperationAdmissionV1, ResourceError> {
        let (request, writer) = input;
        let catalog = writer.map(CatalogWriterBindingV1::retained_capture);
        let timing = r.begin_service_timing();
        let existing = self
            .resources
            .operations
            .rows
            .iter()
            .any(|row| row.operation.0.as_ref() == request.origin.operation);
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let limits = self
                .resources
                .operations
                .limits
                .ok_or(ResourceError::Unsupported)?;
            // Pure complete request/context bounds precede facts, root reads or
            // codec work. Rust borrows and exclusive owner access exclude caller
            // callbacks/mutation; fallible owned copies precede child issuance.
            for label in [
                request.origin.operation,
                request.origin.effect,
                request.origin.attempt,
            ] {
                if std::str::from_utf8(label).is_err() {
                    return Err(ResourceError::Malformed);
                }
                if label.is_empty() || label.len() > limits.max_label_bytes {
                    return Err(ResourceError::Limit);
                }
            }
            if settings::basis::labels(release.context)
                .iter()
                .any(|v| v.is_empty() || v.len() > limits.max_label_bytes)
                || capsule_bytes(
                    request.origin.operation.len(),
                    request.origin.effect.len(),
                    request.origin.attempt.len(),
                    request.target.identity.key.0.len(),
                    release.context,
                )
                .and_then(|n| {
                    n.checked_add(
                        catalog
                            .as_ref()
                            .map(|c| c.native_bytes())
                            .transpose()
                            .ok()?
                            .unwrap_or(0),
                    )
                })
                .is_none_or(|n| n > limits.max_capture_bytes)
                || std::mem::size_of::<SaveOperationAdmissionV1>() > limits.max_reply_bytes
            {
                return Err(ResourceError::Limit);
            }
            if !Arc::ptr_eq(&request.target.registration.owner, &self.namespace)
                || !Arc::ptr_eq(&request.source.document.namespace, &self.namespace)
                || request.expected_serialization == 0
            {
                return Err(ResourceError::Unavailable);
            }
            if !release_current(release) {
                return Err(ResourceError::Unavailable);
            }
            self.validate_save_operations()?;
            if let Some(row) = self
                .resources
                .operations
                .rows
                .iter()
                .find(|row| row.operation.0.as_ref() == request.origin.operation)
            {
                let c = row.capture.as_ref().ok_or(ResourceError::Unavailable)?;
                if !c.matches(request, release.context)
                    || !WriteCatalogCaptureV1::same_intent(c.catalog.as_ref(), catalog.as_ref())
                {
                    return Err(ResourceError::Unavailable);
                }
                return Ok(SaveOperationAdmissionV1 {
                    status: SaveOperationAdmissionStatusV1::ObserveExisting,
                    task: c.task.clone(),
                });
            }
            self.ensure_operational().map_err(owner_error)?;
            if !admission.is_some_and(admission_current) {
                return Err(ResourceError::Unavailable);
            }
            if self.resources.operations.rows.len() >= limits.max_issued {
                return Err(ResourceError::Limit);
            }
            if self.resources.operations.rows.len() >= self.resources.operations.rows.capacity() {
                return Err(ResourceError::Faulted);
            }
            let document = self.validate_version(request.source).map_err(owner_error)?;
            if document.serialization.generation != request.expected_serialization {
                return Err(ResourceError::Stale);
            }
            let binding = self
                .resources
                .bindings
                .iter()
                .find(|b| b.document == request.source.document)
                .ok_or(ResourceError::Unbound)?;
            if binding.generation != request.expected_binding
                || binding.identity != request.target.identity
                || !Arc::ptr_eq(&binding.registration, &request.target.registration)
            {
                return Err(ResourceError::Stale);
            }
            let operation = blob(request.origin.operation, limits.max_label_bytes, true)?;
            let effect = blob(request.origin.effect, limits.max_label_bytes, true)?;
            let attempt = blob(request.origin.attempt, limits.max_label_bytes, true)?;
            let context = settings::basis::copy_context(release.context)
                .map_err(|_| ResourceError::Capacity)?;
            // Configuration preallocates the complete parent row inventory.
            // No fallible step follows successful child admission; no callback,
            // suspension, root publication or provider I/O occurs in this section.
            let task = SaveId {
                owner: self.namespace.clone(),
                slot: self.resources.tasks.len(),
            };
            let parent = Row {
                operation,
                capture: Some(Capture {
                    catalog: catalog.clone(),
                    effect,
                    attempt,
                    source: request.source.clone(),
                    flavor: Flavor::Source {
                        serialization: request.expected_serialization,
                        condition: request.condition,
                    },
                    binding: request.expected_binding,
                    identity: request.target.identity.clone(),
                    registration: request.target.registration.clone(),
                    requirements: request.requirements,
                    context,
                    task: task.clone(),
                }),
            };
            let task = self.queue_pinned_save_with_parent(
                request.source,
                request.requirements,
                pair,
                request.condition == SaveOperationConditionV1::OwnChain,
                (catalog.clone(), Some(parent)),
                r,
            )?;
            Ok(SaveOperationAdmissionV1 {
                status: SaveOperationAdmissionStatusV1::Admitted,
                task,
            })
        }));
        let result = match caught {
            Ok(v) => v,
            Err(p) => {
                self.faulted = true;
                capture_service(
                    r,
                    if existing {
                        Boundary::SaveOperationRecovery
                    } else {
                        Boundary::SaveOperationAdmission
                    },
                    &Err::<SaveOperationAdmissionV1, _>(ResourceError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(p)
            }
        };
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(
            r,
            if existing {
                Boundary::SaveOperationRecovery
            } else {
                Boundary::SaveOperationAdmission
            },
            &result,
            timing.finish(),
        );
        result
    }
}
