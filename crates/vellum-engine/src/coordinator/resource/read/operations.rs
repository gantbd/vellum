//! Owned annotation-load admission metadata. Reads stay in the existing reader;
//! this registry neither submits nor decodes/restores/publishes a candidate.
use super::*;
mod shutdown_inventory;
#[cfg(test)]
mod tests;
#[cfg(test)]
pub(super) use tests::admit as test_admit_owned_read;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnnotationReadOperationLimitsV1 {
    pub max_capture_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_label_bytes: usize,
    pub max_store_bytes: usize,
}
#[derive(Clone, Copy)]
pub struct AnnotationReadOperationOriginV1<'a> {
    pub operation: &'a [u8],
    pub effect: &'a [u8],
    pub attempt: &'a [u8],
}
#[derive(Clone, Copy)]
pub struct ExpectedAnnotationReadBindingV1<'a> {
    pub target: &'a SourceBindingTargetV1,
    pub generation: u64,
}
pub struct OwnedAnnotationReadRequestV1<'a> {
    pub origin: AnnotationReadOperationOriginV1<'a>,
    pub source: &'a DocumentVersion,
    pub expected_set: u64,
    pub annotation_binding: ExpectedAnnotationReadBindingV1<'a>,
    pub source_binding: Option<ExpectedAnnotationReadBindingV1<'a>>,
    pub target: &'a AnnotationReadTarget,
    pub strength: ReadStrength,
}
/// Trusted host facts, not requester assertions. Source-read permission is not
/// an annotation metadata/read grant. The last four facts govern fresh work only.
pub struct TrustedAnnotationReadOperationFactsV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub read_allowed: Option<bool>,
    pub annotation_readable: Option<bool>,
    pub retained: Option<bool>,
    pub reply_ready: Option<bool>,
    pub transfer_ready: Option<bool>,
    pub resources_ready: Option<bool>,
    pub identity_allowed: Option<bool>,
    pub binding_coverage_complete: Option<bool>,
    pub provider_available: Option<bool>,
    pub launch_allowed: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationReadOperationAdmissionStatusV1 {
    Admitted,
    ObserveExisting,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnotationReadOperationAdmissionV1 {
    pub status: AnnotationReadOperationAdmissionStatusV1,
    pub task: AnnotationReadId,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnnotationReadOperationUsageV1 {
    pub issued_owned: usize,
    pub owned: usize,
    pub retired: usize,
    pub spent_origins: usize,
    pub issued_native_reads: usize,
    pub held_native_read_bytes: usize,
    pub reserved_total_bytes: usize,
    pub admission_stopped: bool,
}
#[derive(Clone, PartialEq, Eq)]
pub struct ObservedReadVersionV1(Blob);
impl ObservedReadVersionV1 {
    pub fn bytes(&self) -> &[u8] {
        &self.0.0
    }
}
impl std::fmt::Debug for ObservedReadVersionV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ObservedReadVersionV1(<redacted>)")
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeReadTerminalWitnessV1 {
    End {
        version: ObservedReadVersionV1,
        coherence: ReadStrength,
        chunks: usize,
        bytes: usize,
    },
    Failure(ReadFailure),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeAnnotationReadReceiptV1 {
    pub phase: ReadPhase,
    pub stopped: bool,
    pub quiescent: bool,
    pub contested: bool,
    pub decode_refused: bool,
    pub received_chunks: usize,
    pub received_bytes: usize,
    pub witnesses: [Option<NativeReadTerminalWitnessV1>; 2],
}
fn terminal(atom: &ReadAtom, coherence: ReadStrength) -> Option<NativeReadTerminalWitnessV1> {
    match atom {
        ReadAtom::End {
            version,
            chunks,
            bytes,
        } => Some(NativeReadTerminalWitnessV1::End {
            version: ObservedReadVersionV1(version.clone()),
            coherence,
            chunks: *chunks,
            bytes: *bytes,
        }),
        ReadAtom::Failure(reason) => Some(NativeReadTerminalWitnessV1::Failure(*reason)),
        _ => None,
    }
}
#[derive(Default)]
pub(in crate::coordinator::resource) enum State {
    #[default]
    Unconfigured,
    Active(Box<Store>),
    Closed,
}
impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationReadOperations(<owned>)")
    }
}
pub(in crate::coordinator::resource) struct Store {
    limits: AnnotationReadOperationLimitsV1,
    max_issued: usize,
    max_held: usize,
    origins: Vec<Origin>,
    rows: Vec<Row>,
    reservation: retention::Reservation,
}
struct Origin {
    operation: Blob,
    effect: Blob,
    attempt: Blob,
}
struct Row {
    operation: Blob,
    capture: Option<Capture>,
}
struct Capture {
    catalog: Option<Arc<ReadCatalogCaptureV1>>,
    effect: Blob,
    attempt: Blob,
    source: DocumentVersion,
    set: u64,
    association: AnnotationAssociation,
    registration: Arc<Registration>,
    identity: Identity,
    strength: ReadStrength,
    context: SettingsExecutionContextV1,
    task: AnnotationReadId,
}
fn parent(e: RetentionErrorV1) -> ResourceError {
    match e {
        RetentionErrorV1::Limit => ResourceError::Limit,
        RetentionErrorV1::Faulted => ResourceError::Faulted,
        RetentionErrorV1::Unavailable => ResourceError::Unavailable,
    }
}
fn reserved(l: AnnotationReadOperationLimitsV1, r: ResourceLimits) -> Result<usize, ResourceError> {
    r.max_issued_reads
        .checked_mul(l.max_capture_bytes)
        .and_then(|n| n.checked_mul(2))
        .and_then(|n| n.checked_add(r.max_held_read_bytes))
        .and_then(|n| n.checked_add(std::mem::size_of::<Store>()))
        .and_then(|n| n.checked_add(std::mem::size_of::<State>()))
        .ok_or(ResourceError::Limit)
}
fn release_current(f: &TrustedAnnotationReadOperationFactsV1<'_>) -> bool {
    [
        f.authorized,
        f.scope,
        f.continuous,
        f.live,
        f.not_stopped,
        f.read_allowed,
        f.annotation_readable,
        f.retained,
        f.reply_ready,
        f.transfer_ready,
        f.resources_ready,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn binding(b: ExpectedAnnotationReadBindingV1<'_>) -> BindingKey {
    BindingKey {
        identity: b.target.identity.clone(),
        registration: b.target.registration.clone(),
        generation: b.generation,
    }
}
fn association(r: &OwnedAnnotationReadRequestV1<'_>) -> AnnotationAssociation {
    AnnotationAssociation {
        binding: binding(r.annotation_binding),
        source_binding: r.source_binding.map(binding),
    }
}
fn bytes(
    operation: usize,
    effect: usize,
    attempt: usize,
    identity: &Identity,
    a: &AnnotationAssociation,
    c: &SettingsExecutionContextV1,
) -> Option<usize> {
    [
        operation,
        effect,
        attempt,
        identity.key.0.len(),
        a.binding.identity.key.0.len(),
        a.source_binding
            .as_ref()
            .map_or(0, |s| s.identity.key.0.len()),
    ]
    .into_iter()
    .chain(settings::basis::labels(c).map(str::len))
    .try_fold(std::mem::size_of::<Row>(), usize::checked_add)
}
fn origin_bytes(operation: usize, effect: usize, attempt: usize) -> Option<usize> {
    std::mem::size_of::<Origin>()
        .checked_add(operation)?
        .checked_add(effect)?
        .checked_add(attempt)
}
impl Capture {
    fn intact(&self, t: &ReadTask) -> bool {
        self.source == t.source
            && self.set == t.set
            && self.association == t.association
            && Arc::ptr_eq(&self.registration, &t.registration)
            && self.identity == t.identity
            && self.strength == t.strength
            && ReadCatalogCaptureV1::same(self.catalog.as_ref(), t.catalog.as_ref())
    }
    fn matches(
        &self,
        r: &OwnedAnnotationReadRequestV1<'_>,
        context: &SettingsExecutionContextV1,
    ) -> bool {
        self.effect.0.as_ref() == r.origin.effect
            && self.attempt.0.as_ref() == r.origin.attempt
            && self.source == *r.source
            && self.set == r.expected_set
            && self.association == association(r)
            && Arc::ptr_eq(&self.registration, &r.target.registration)
            && self.identity == r.target.identity
            && self.strength == r.strength
            && self.context == *context
            && ReadCatalogCaptureV1::same(self.catalog.as_ref(), r.target.catalog.as_ref())
    }
}
impl SourceCoordinator {
    #[cfg(test)]
    pub(in crate::coordinator::resource) fn test_corrupt_read_terminal(
        &mut self,
        id: &AnnotationReadId,
    ) {
        self.resources.reads[id.slot].as_mut().unwrap().terminal = Some(ReadAtom::End {
            version: Blob(Arc::from([])),
            chunks: 0,
            bytes: 0,
        });
    }
    pub(in crate::coordinator::resource) fn owned_annotation_read_control(
        &self,
        id: &AnnotationReadId,
        context: &SettingsExecutionContextV1,
    ) -> Result<(bool, bool), ResourceError> {
        self.validate_read_operations()?;
        let task = self.read_task(id)?;
        let index = task.operation.ok_or(ResourceError::Unavailable)?;
        let capture = self
            .read_operation_store()?
            .rows
            .get(index)
            .and_then(|r| r.capture.as_ref())
            .ok_or(ResourceError::Unavailable)?;
        if capture.task != *id || capture.context != *context {
            return Err(ResourceError::Unavailable);
        }
        Ok((task.stopped, task.quiescent))
    }
    pub(in crate::coordinator::resource) fn owned_annotation_read_receipt(
        &self,
        id: &AnnotationReadId,
        context: &SettingsExecutionContextV1,
    ) -> Result<NativeAnnotationReadReceiptV1, ResourceError> {
        let (stopped, quiescent) = self.owned_annotation_read_control(id, context)?;
        let task = self.read_task(id)?;
        if task.chunks.len() > task.registration.limits.max_read_chunks
            || task.data().len() > task.registration.limits.max_blob_bytes
        {
            return Err(ResourceError::Faulted);
        }
        let mut witnesses = [None, None];
        let mut count = 0;
        for atom in task.terminal.iter().chain(task.witnesses.iter().flatten()) {
            if let Some(value) = terminal(atom, task.strength)
                && !witnesses.iter().flatten().any(|w| w == &value)
            {
                if matches!(&value, NativeReadTerminalWitnessV1::End { version, .. }
                    if version.bytes().is_empty() || version.bytes().len() > task.registration.limits.max_validator_bytes)
                {
                    return Err(ResourceError::Faulted);
                }
                if count == witnesses.len() {
                    return Err(ResourceError::Faulted);
                }
                witnesses[count] = Some(value);
                count += 1;
            }
        }
        Ok(NativeAnnotationReadReceiptV1 {
            phase: task.phase,
            stopped,
            quiescent,
            contested: task.contested,
            decode_refused: task.refused,
            received_chunks: task.chunks.len(),
            received_bytes: task.data().len(),
            witnesses,
        })
    }
    fn read_operation_store(&self) -> Result<&Store, ResourceError> {
        match &self.resources.read_operations {
            State::Active(s) => Ok(s),
            _ => Err(ResourceError::Unavailable),
        }
    }
    pub(super) fn validate_read_operations(&self) -> Result<(), ResourceError> {
        self.resources.annotation_reader_lifecycle_intact()?;
        if matches!(self.resources.read_operations, State::Closed)
            && (!self.resources.annotation_read_admission_stopped
                || self.resources.reads.iter().any(Option::is_some)
                || self.resources.held_read_bytes != 0)
        {
            return Err(ResourceError::Faulted);
        }
        let State::Active(s) = &self.resources.read_operations else {
            return if self
                .resources
                .reads
                .iter()
                .flatten()
                .all(|t| t.operation.is_none())
            {
                Ok(())
            } else {
                Err(ResourceError::Faulted)
            };
        };
        let limits = self.resources.limits.ok_or(ResourceError::Faulted)?;
        let expected = reserved(s.limits, limits).map_err(|_| ResourceError::Faulted)?;
        self.retention
            .validate_exact(&s.reservation, expected)
            .map_err(parent)?;
        if s.max_issued != limits.max_issued_reads
            || s.max_held != limits.max_held_read_bytes
            || s.rows.len() != s.origins.len()
            || s.rows.len() > s.max_issued
            || self.resources.reads.len() > s.max_issued
            || self.resources.held_read_bytes > s.max_held
            || reserved(s.limits, limits)
                .ok()
                .is_none_or(|n| n > s.limits.max_store_bytes)
        {
            return Err(ResourceError::Faulted);
        }
        for (index, (row, origin)) in s.rows.iter().zip(&s.origins).enumerate() {
            if row.operation != origin.operation
                || s.rows[..index].iter().any(|p| p.operation == row.operation)
                || [&origin.operation, &origin.effect, &origin.attempt]
                    .iter()
                    .any(|b| b.0.is_empty() || b.0.len() > s.limits.max_label_bytes)
                || origin_bytes(
                    origin.operation.0.len(),
                    origin.effect.0.len(),
                    origin.attempt.0.len(),
                )
                .is_none_or(|n| n > s.limits.max_capture_bytes)
            {
                return Err(ResourceError::Faulted);
            }
            if let Some(c) = &row.capture {
                let task = self
                    .read_task(&c.task)
                    .map_err(|_| ResourceError::Faulted)?;
                if c.effect != origin.effect
                    || c.attempt != origin.attempt
                    || task.operation != Some(index)
                    || !c.intact(task)
                    || settings::basis::labels(&c.context)
                        .iter()
                        .any(|v| v.is_empty() || v.len() > s.limits.max_label_bytes)
                    || bytes(
                        row.operation.0.len(),
                        c.effect.0.len(),
                        c.attempt.0.len(),
                        &c.identity,
                        &c.association,
                        &c.context,
                    )
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
                    .is_none_or(|n| n > s.limits.max_capture_bytes)
                {
                    return Err(ResourceError::Faulted);
                }
            }
        }
        for (slot, t) in self.resources.reads.iter().enumerate() {
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
    pub(in crate::coordinator::resource) fn annotation_read_operation_document_pins(
        &self,
        document: &DocumentId,
    ) -> Result<usize, ResourceError> {
        self.validate_read_operations()?;
        Ok(match &self.resources.read_operations {
            State::Unconfigured | State::Closed => 0,
            State::Active(store) => store
                .rows
                .iter()
                .filter(|row| {
                    row.capture
                        .as_ref()
                        .is_some_and(|capture| capture.source.document == *document)
                })
                .count(),
        })
    }
    pub(super) fn annotation_read_operation_retirement(
        &self,
        id: &AnnotationReadId,
    ) -> Result<Option<usize>, ResourceError> {
        self.validate_read_operations()?;
        Ok(self.read_task(id)?.operation)
    }
    pub(super) fn spend_annotation_read_operation(&mut self, index: usize) {
        let State::Active(s) = &mut self.resources.read_operations else {
            unreachable!("validated read parent")
        };
        s.rows[index].capture = None;
    }
    pub fn configure_annotation_read_operations_v1(
        &mut self,
        l: AnnotationReadOperationLimitsV1,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            let limits = self.resources.limits.ok_or(ResourceError::Unsupported)?;
            self.validate_read_operations()?;
            match &self.resources.read_operations {
                State::Active(s) => {
                    return if s.limits == l {
                        Ok(())
                    } else {
                        Err(ResourceError::Unavailable)
                    };
                }
                State::Closed => return Err(ResourceError::Unavailable),
                State::Unconfigured => {}
            }
            self.ensure_admission_unfenced().map_err(owner_error)?;
            if self.resources.annotation_read_admission_stopped {
                return Err(ResourceError::Unavailable);
            }
            let total = reserved(l, limits)?;
            if total > l.max_store_bytes
                || l.max_capture_bytes
                    < std::mem::size_of::<Row>().max(std::mem::size_of::<Origin>())
            {
                return Err(ResourceError::Limit);
            }
            self.retention.check(total).map_err(parent)?;
            let mut origins = Vec::new();
            origins
                .try_reserve_exact(limits.max_issued_reads)
                .map_err(|_| ResourceError::Capacity)?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(limits.max_issued_reads)
                .map_err(|_| ResourceError::Capacity)?;
            let reservation = self.retention.reserve(total).map_err(parent)?;
            self.resources.read_operations = State::Active(Box::new(Store {
                limits: l,
                max_issued: limits.max_issued_reads,
                max_held: limits.max_held_read_bytes,
                origins,
                rows,
                reservation,
            }));
            Ok(())
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(
            r,
            Boundary::AnnotationReadOperationConfiguration,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn request_owned_annotation_read_v1(
        &mut self,
        request: &OwnedAnnotationReadRequestV1<'_>,
        f: &TrustedAnnotationReadOperationFactsV1<'_>,
        pair: Option<OrdinaryPairEvidence>,
        r: &mut Recorder,
    ) -> Result<AnnotationReadOperationAdmissionV1, ResourceError> {
        let timing = r.begin_service_timing();
        let existing = self.read_operation_store().is_ok_and(|s| {
            s.rows
                .iter()
                .any(|p| p.operation.0.as_ref() == request.origin.operation)
        });
        let result = (|| {
            let s = self.read_operation_store()?;
            let l = s.limits;
            for label in [
                request.origin.operation,
                request.origin.effect,
                request.origin.attempt,
            ] {
                if std::str::from_utf8(label).is_err() {
                    return Err(ResourceError::Malformed);
                }
                if label.is_empty() || label.len() > l.max_label_bytes {
                    return Err(ResourceError::Limit);
                }
            }
            let a = association(request);
            if bytes(
                request.origin.operation.len(),
                request.origin.effect.len(),
                request.origin.attempt.len(),
                &request.target.identity,
                &a,
                f.context,
            )
            .and_then(|n| {
                n.checked_add(
                    request
                        .target
                        .catalog
                        .as_ref()
                        .map(|c| c.native_bytes())
                        .transpose()
                        .ok()?
                        .unwrap_or(0),
                )
            })
            .is_none_or(|n| n > l.max_capture_bytes)
                || origin_bytes(
                    request.origin.operation.len(),
                    request.origin.effect.len(),
                    request.origin.attempt.len(),
                )
                .is_none_or(|n| n > l.max_capture_bytes)
                || settings::basis::labels(f.context)
                    .iter()
                    .any(|v| v.is_empty() || v.len() > l.max_label_bytes)
                || std::mem::size_of::<AnnotationReadOperationAdmissionV1>() > l.max_reply_bytes
            {
                return Err(ResourceError::Limit);
            }
            if !Arc::ptr_eq(&request.source.document.namespace, &self.namespace)
                || !Arc::ptr_eq(&request.target.registration.owner, &self.namespace)
                || !Arc::ptr_eq(&a.binding.registration.owner, &self.namespace)
                || a.source_binding
                    .as_ref()
                    .is_some_and(|b| !Arc::ptr_eq(&b.registration.owner, &self.namespace))
                || a.binding.generation == 0
            {
                return Err(ResourceError::Unavailable);
            }
            if !release_current(f) {
                return Err(ResourceError::Unavailable);
            }
            self.validate_read_operations()?;
            if let Some(row) = self
                .read_operation_store()?
                .rows
                .iter()
                .find(|p| p.operation.0.as_ref() == request.origin.operation)
            {
                let c = row.capture.as_ref().ok_or(ResourceError::Unavailable)?;
                if !c.matches(request, f.context) {
                    return Err(ResourceError::Unavailable);
                }
                return Ok(AnnotationReadOperationAdmissionV1 {
                    status: AnnotationReadOperationAdmissionStatusV1::ObserveExisting,
                    task: c.task.clone(),
                });
            }
            self.ensure_operational().map_err(owner_error)?;
            if [
                f.identity_allowed,
                f.binding_coverage_complete,
                f.provider_available,
                f.launch_allowed,
            ]
            .into_iter()
            .any(|v| v != Some(true))
            {
                return Err(ResourceError::Unavailable);
            }
            if self.resources.annotation_read_admission_stopped {
                return Err(ResourceError::Unavailable);
            }
            if self
                .current_annotation_association(&request.source.document)
                .as_ref()
                != Some(&a)
            {
                return Err(ResourceError::Stale);
            }
            let s = self.read_operation_store()?;
            if s.rows.len() >= s.max_issued {
                return Err(ResourceError::Limit);
            }
            if s.rows.len() >= s.rows.capacity() || s.origins.len() >= s.origins.capacity() {
                return Err(ResourceError::Faulted);
            }
            let index = s.rows.len();
            // Only typed, bounded, immutable Rust input is read here; no supplier
            // callbacks or caller mutation can intervene. All fallible copies
            // precede the existing native reader's serialized issuance section.
            let operation = blob(request.origin.operation, l.max_label_bytes, true)?;
            let effect = blob(request.origin.effect, l.max_label_bytes, true)?;
            let attempt = blob(request.origin.attempt, l.max_label_bytes, true)?;
            let context =
                settings::basis::copy_context(f.context).map_err(|_| ResourceError::Capacity)?;
            let target = AnnotationReadTarget {
                catalog: request.target.catalog.clone(),
                registration: request.target.registration.clone(),
                identity: request.target.identity.clone(),
            };
            let task = self.admit_annotation_read(
                request.source,
                request.expected_set,
                target,
                request.strength,
                pair,
                r,
            )?;
            // After native issuance, linkage is allocation-free and infallible.
            // An unexpected unwind must leave the owner fenced, not healthy
            // with an issued child missing its operation parent.
            self.faulted = true;
            self.resources.reads[task.slot]
                .as_mut()
                .expect("new native read")
                .operation = Some(index);
            let State::Active(s) = &mut self.resources.read_operations else {
                unreachable!("validated reader")
            };
            s.origins.push(Origin {
                operation: operation.clone(),
                effect: effect.clone(),
                attempt: attempt.clone(),
            });
            s.rows.push(Row {
                operation,
                capture: Some(Capture {
                    catalog: request.target.catalog.clone(),
                    effect,
                    attempt,
                    source: request.source.clone(),
                    set: request.expected_set,
                    association: a,
                    registration: request.target.registration.clone(),
                    identity: request.target.identity.clone(),
                    strength: request.strength,
                    context,
                    task: task.clone(),
                }),
            });
            self.faulted = false;
            Ok(AnnotationReadOperationAdmissionV1 {
                status: AnnotationReadOperationAdmissionStatusV1::Admitted,
                task,
            })
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(
            r,
            if existing {
                Boundary::AnnotationReadOperationRecovery
            } else {
                Boundary::AnnotationReadOperationAdmission
            },
            &result,
            timing.finish(),
        );
        result
    }
    pub fn annotation_read_operation_usage_v1(
        &self,
    ) -> Result<AnnotationReadOperationUsageV1, ResourceError> {
        self.validate_read_operations()?;
        let s = self.read_operation_store()?;
        let owned = s.rows.iter().filter(|r| r.capture.is_some()).count();
        Ok(AnnotationReadOperationUsageV1 {
            issued_owned: s.rows.len(),
            owned,
            retired: s.rows.len() - owned,
            spent_origins: s.origins.len(),
            issued_native_reads: self.resources.reads.len(),
            held_native_read_bytes: self.resources.held_read_bytes,
            reserved_total_bytes: reserved(
                s.limits,
                self.resources.limits.ok_or(ResourceError::Faulted)?,
            )?,
            admission_stopped: self.resources.annotation_read_admission_stopped,
        })
    }
    /// Trusted-owner fence only; neither existing work nor late evidence is stopped.
    pub fn stop_annotation_read_admission_v1(&mut self, r: &mut Recorder) {
        let timing = r.begin_service_timing();
        self.resources.annotation_read_admission_stopped = true;
        capture_service(
            r,
            Boundary::AnnotationReadOperationConfiguration,
            &Ok::<(), ResourceError>(()),
            timing.finish(),
        );
    }
    pub fn retire_annotation_read_operations_v1(
        &mut self,
        owner_control_allowed: Option<bool>,
        retention_released: Option<bool>,
        delivery_drained: Option<bool>,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.validate_read_operations()?;
            let s = self.read_operation_store()?;
            if !self.resources.annotation_read_admission_stopped
                || self.resources.reads.iter().any(Option::is_some)
                || self.resources.held_read_bytes != 0
                || s.rows.iter().any(|r| r.capture.is_some())
                || [owner_control_allowed, retention_released, delivery_drained]
                    .into_iter()
                    .any(|v| v != Some(true))
            {
                return Err(ResourceError::Retained);
            }
            self.retention.validate(&s.reservation).map_err(parent)?;
            let State::Active(mut s) =
                std::mem::replace(&mut self.resources.read_operations, State::Closed)
            else {
                unreachable!("validated reader")
            };
            self.retention.release(&mut s.reservation).map_err(parent)?;
            Ok(())
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(
            r,
            Boundary::AnnotationReadOperationRetirement,
            &result,
            timing.finish(),
        );
        result
    }
}
