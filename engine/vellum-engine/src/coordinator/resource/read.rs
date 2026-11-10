//! Bounded owned read transcript; byte capture is neither syntax nor restoration.
use super::*;
mod discharge;
mod load_authority;
mod retirement;
mod shutdown_inventory;
pub use load_authority::{AnnotationLoadAuthorityFactsV1, AnnotationLoadAuthorityV1};
pub(super) mod operations;
pub use operations::*;
#[cfg(test)]
pub(super) mod tests;
pub(super) struct ReadTask {
    pub(super) queue_timing: queue_timing::Stamp,
    catalog: Option<Arc<ReadCatalogCaptureV1>>,
    operation: Option<usize>,
    source: DocumentVersion,
    set: u64,
    association: AnnotationAssociation,
    registration: Arc<Registration>,
    identity: Identity,
    strength: ReadStrength,
    pub(super) phase: ReadPhase,
    pub(super) quiescent: bool,
    pub(super) contested: bool,
    stopped: bool,
    refused: bool,
    nonce: Option<Arc<()>>,
    start: Option<Blob>,
    chunks: Vec<(usize, usize)>,
    bytes: Vec<u8>,
    capture: Option<Arc<ReadCaptureData>>,
    terminal: Option<ReadAtom>,
    witnesses: [Option<ReadAtom>; 2],
    charge: usize,
    pub(super) restoration: Option<RestorationProvenance>,
}
impl ReadTask {
    pub(super) fn retains_document(&self, document: &DocumentId) -> bool {
        self.source.document == *document
    }
}
impl std::fmt::Debug for ReadTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReadTask(<redacted>)")
    }
}
fn read_task_charge(
    l: ResourceLimits,
    identity: &Identity,
    a: &AnnotationAssociation,
    catalog: usize,
) -> Option<usize> {
    [
        std::mem::size_of::<ReadTask>(),
        l.max_restoration_bytes,
        std::mem::size_of::<ReadCaptureData>(),
        l.max_blob_bytes,
        l.max_blob_bytes.max(l.max_validator_bytes).checked_mul(2)?,
        l.max_validator_bytes.checked_mul(2)?,
        l.max_read_chunks
            .checked_mul(std::mem::size_of::<(usize, usize)>())?,
        identity.key.0.len(),
        a.binding.identity.key.0.len(),
        a.source_binding
            .as_ref()
            .map_or(0, |b| b.identity.key.0.len()),
    ]
    .into_iter()
    .try_fold(catalog, usize::checked_add)
}
impl ReadTask {
    fn data(&self) -> &[u8] {
        self.capture.as_ref().map_or(&self.bytes, |c| &c.bytes.0)
    }
    fn prior(&self, atom: &ReadAtom) -> Result<Option<ReadAtom>, ResourceError> {
        Ok(match atom {
            ReadAtom::Start { .. } => self.start.as_ref().map(|version| ReadAtom::Start {
                version: version.clone(),
            }),
            ReadAtom::Chunk {
                sequence,
                offset,
                bytes,
            } => match self.chunks.get(*sequence) {
                Some((old_offset, len))
                    if old_offset == offset
                        && self.data().get(*old_offset..old_offset + len)
                            == Some(bytes.0.as_ref()) =>
                {
                    Some(atom.clone())
                }
                Some((old_offset, len)) => Some(ReadAtom::Chunk {
                    sequence: *sequence,
                    offset: *old_offset,
                    bytes: crate::resource::blob(
                        &self.data()[*old_offset..old_offset + len],
                        self.registration.limits.max_blob_bytes,
                        false,
                    )?,
                }),
                None => None,
            },
            ReadAtom::End { .. } | ReadAtom::Failure(_) => self.terminal.clone(),
            ReadAtom::Quiescence => None,
        })
    }
    fn conflict(
        &mut self,
        prior: ReadAtom,
        incoming: ReadAtom,
    ) -> Result<EvidenceDisposition, ResourceError> {
        if self.contested {
            return Err(ResourceError::EvidenceCapacity);
        }
        self.witnesses = [Some(prior), Some(incoming)];
        self.contested = true;
        Ok(EvidenceDisposition::Contradiction)
    }
    fn status(&self) -> AnnotationReadStatus {
        AnnotationReadStatus {
            restored: self.restoration.is_some(),
            phase: self.phase,
            stopped: self.stopped,
            quiescent: self.quiescent,
            contested: self.contested,
            refused: self.refused,
            complete: self.capture.is_some(),
            failure: match self.terminal {
                Some(ReadAtom::Failure(f)) => Some(f),
                _ => None,
            },
            received_chunks: self.chunks.len(),
            received_bytes: self.data().len(),
            witness_count: self.witnesses.iter().flatten().count(),
        }
    }
    fn accept(
        &mut self,
        id: &AnnotationReadId,
        atom: ReadAtom,
    ) -> Result<EvidenceDisposition, ResourceError> {
        if atom == ReadAtom::Quiescence {
            let old = self.quiescent;
            self.quiescent = true;
            return Ok(if old {
                EvidenceDisposition::Duplicate
            } else {
                EvidenceDisposition::Accepted
            });
        }
        if self.witnesses.iter().flatten().any(|w| w == &atom) {
            return Ok(EvidenceDisposition::Duplicate);
        }
        if let Some(prior) = self.prior(&atom)? {
            return if prior == atom {
                Ok(EvidenceDisposition::Duplicate)
            } else {
                self.conflict(prior, atom)
            };
        }
        if self.contested {
            return Err(ResourceError::EvidenceCapacity);
        }
        if let Some(terminal) = &self.terminal {
            return self.conflict(terminal.clone(), atom);
        }
        if self.refused && !matches!(atom, ReadAtom::Failure(_)) {
            return Err(ResourceError::Unavailable);
        }
        match atom {
            ReadAtom::Start { version } => {
                self.start = Some(version);
            }
            ReadAtom::Chunk {
                sequence,
                offset,
                bytes,
            } => {
                if self.stopped {
                    return Ok(EvidenceDisposition::Accepted);
                }
                if self.start.is_none()
                    || sequence != self.chunks.len()
                    || offset != self.bytes.len()
                {
                    return Err(ResourceError::Malformed);
                }
                let limits = self.registration.limits;
                if self.chunks.len() >= limits.max_read_chunks
                    || self
                        .bytes
                        .len()
                        .checked_add(bytes.0.len())
                        .is_none_or(|n| n > limits.max_blob_bytes)
                {
                    self.refused = true;
                    return Err(ResourceError::Limit);
                }
                self.chunks.push((offset, bytes.0.len()));
                self.bytes.extend_from_slice(&bytes.0);
            }
            ReadAtom::End {
                version,
                chunks,
                bytes,
            } => {
                if self.start.as_ref() != Some(&version)
                    || (!self.stopped && (chunks != self.chunks.len() || bytes != self.bytes.len()))
                {
                    return Err(ResourceError::Malformed);
                }
                if !self.stopped {
                    self.capture = Some(Arc::new(ReadCaptureData {
                        id: id.clone(),
                        source: self.source.clone(),
                        set: self.set,
                        bytes: Blob(Arc::from(std::mem::take(&mut self.bytes))),
                        version: version.clone(),
                        strength: self.strength,
                    }));
                }
                self.terminal = Some(ReadAtom::End {
                    version,
                    chunks,
                    bytes,
                });
            }
            ReadAtom::Failure(f) => self.terminal = Some(ReadAtom::Failure(f)),
            ReadAtom::Quiescence => unreachable!("handled control fact"),
        }
        Ok(EvidenceDisposition::Accepted)
    }
}
impl SourceCoordinator {
    pub(super) fn annotation_read_queue_stamp(
        &self,
        id: &AnnotationReadId,
    ) -> Result<queue_timing::Stamp, ResourceError> {
        self.read_task(id).map(|t| t.queue_timing)
    }
    pub(super) fn original_loaded_baseline(
        &self,
        id: &AnnotationReadId,
    ) -> Result<LoadedAnnotationBaseline, ResourceError> {
        let attached = self
            .resources
            .read_baseline_attachment
            .as_ref()
            .ok_or(ResourceError::MissingEvidence)?;
        let incarnation = attached.current(&self.namespace)?;
        let task = self.read_task(id)?;
        let provenance = task
            .restoration
            .as_ref()
            .ok_or(ResourceError::MissingEvidence)?;
        if task.contested {
            return Err(ResourceError::Contested);
        }
        if task.refused {
            return Err(ResourceError::Unavailable);
        }
        let capture = task.capture.as_ref().ok_or(ResourceError::Faulted)?;
        if !Arc::ptr_eq(capture, &provenance.package.capture.data)
            || provenance.outcome.source != task.source
            || !matches!(task.terminal, Some(ReadAtom::End { .. }))
        {
            return Err(ResourceError::Faulted);
        }
        self.document(&task.source.document).map_err(owner_error)?;
        if self
            .current_annotation_association(&task.source.document)
            .as_ref()
            != Some(&task.association)
        {
            return Err(ResourceError::Stale);
        }
        self.provider_current(&task.registration)?;
        self.provider_current(&task.association.binding.registration)?;
        let encoding = provenance
            .package
            .registration
            .encoding
            .clone()
            .ok_or(ResourceError::MissingEvidence)?;
        self.encoder_current(&encoding)?;
        Ok(LoadedAnnotationBaseline {
            attachment: incarnation.clone(),
            fact: LoadedAnnotationBaselineFact {
                origin: id.clone(),
                source: task.source.clone(),
                set_version: provenance.outcome.set_version,
                restoration_context: provenance.package.registration.context.clone(),
                encoding_context: encoding.context.clone(),
                reader: ReadProviderContext {
                    registration: task.registration.clone(),
                },
                ordinary_binding_generation: task.association.binding.generation,
                source_binding_generation: task
                    .association
                    .source_binding
                    .as_ref()
                    .map(|b| b.generation),
                coherence: task.strength,
            },
            association: task.association.clone(),
            encoding,
            registration: task.registration.clone(),
            bytes: capture.bytes.clone(),
            version: capture.version.clone(),
        })
    }
    /// Historical ownership check, not current read/adoption eligibility. A
    /// disposed source or late contested evidence cannot erase the original
    /// loaded-baseline obligation from shutdown inspection.
    pub(super) fn shutdown_loaded_baseline_intact(
        &self,
        binding: &AnnotationBinding,
        baseline: &LoadedAnnotationBaseline,
    ) -> Result<(), ResourceError> {
        let attached = self
            .resources
            .read_baseline_attachment
            .as_ref()
            .ok_or(ResourceError::Faulted)?;
        let incarnation = attached.current(&self.namespace)?;
        let task = self
            .read_task(&baseline.fact.origin)
            .map_err(|_| ResourceError::Faulted)?;
        let provenance = task.restoration.as_ref().ok_or(ResourceError::Faulted)?;
        let capture = task.capture.as_ref().ok_or(ResourceError::Faulted)?;
        let encoding = provenance
            .package
            .registration
            .encoding
            .as_ref()
            .ok_or(ResourceError::Faulted)?;
        self.retained_document(&task.source.document)
            .map_err(owner_error)?;
        if !Arc::ptr_eq(incarnation, &baseline.attachment)
            || !Arc::ptr_eq(capture, &provenance.package.capture.data)
            || !Arc::ptr_eq(encoding, &baseline.encoding)
            || !Arc::ptr_eq(&task.registration, &baseline.registration)
            || !Arc::ptr_eq(&task.registration, &baseline.fact.reader.registration)
            || binding.document != task.source.document
            || task.source != baseline.fact.source
            || provenance.outcome.source != task.source
            || provenance.outcome.set_version != baseline.fact.set_version
            || task.association != baseline.association
            || binding.key != baseline.association.binding
            || baseline.fact.ordinary_binding_generation != task.association.binding.generation
            || baseline.fact.source_binding_generation
                != task
                    .association
                    .source_binding
                    .as_ref()
                    .map(|b| b.generation)
            || provenance.package.registration.context != baseline.fact.restoration_context
            || encoding.context != baseline.fact.encoding_context
            || task.strength != baseline.fact.coherence
            || baseline.bytes != capture.bytes
            || baseline.version != capture.version
            || !matches!(task.terminal, Some(ReadAtom::End { .. }))
        {
            return Err(ResourceError::Faulted);
        }
        Ok(())
    }
    pub(super) fn loaded_companion_size(
        &self,
        id: &AnnotationReadId,
    ) -> Result<usize, ResourceError> {
        self.read_task(id)?
            .restoration
            .as_ref()
            .map(|p| p.package.charge)
            .ok_or(ResourceError::MissingEvidence)
    }
    pub(super) fn loaded_companion_limit(
        &self,
        id: &AnnotationReadId,
    ) -> Result<usize, ResourceError> {
        Ok(self
            .read_task(id)?
            .registration
            .limits
            .max_restoration_bytes)
    }
    pub(super) fn mark_loaded_baseline_adopted(&mut self, id: &AnnotationReadId) {
        self.resources.reads[id.slot]
            .as_mut()
            .expect("validated read")
            .restoration
            .as_mut()
            .expect("consumed read")
            .outcome
            .loaded_baseline_adopted = true;
    }
    pub fn register_annotation_restorer(
        &mut self,
        recorder: &mut Recorder,
    ) -> Result<AnnotationRestorerPort, ResourceError> {
        self.register_restorer(None, recorder)
    }
    /// Captures the intended serialization context before restoration. This
    /// read-only identity does not grant the restorer an encoder issuer port.
    pub fn register_annotation_restorer_for_encoding(
        &mut self,
        context: &AnnotationEncodingContext,
        recorder: &mut Recorder,
    ) -> Result<AnnotationRestorerPort, ResourceError> {
        self.register_restorer(Some(context), recorder)
    }
    fn register_restorer(
        &mut self,
        context: Option<&AnnotationEncodingContext>,
        recorder: &mut Recorder,
    ) -> Result<AnnotationRestorerPort, ResourceError> {
        let result = (|| {
            self.ensure_ordinary_admission().map_err(owner_error)?;
            self.ensure_annotation_reader_open()?;
            let limits = self.resources.limits.ok_or(ResourceError::Unsupported)?;
            let annotations = self.annotations.limits.ok_or(ResourceError::Unsupported)?;
            let encoding = context
                .map(|context| {
                    self.resources
                        .encoders
                        .iter()
                        .find(|e| &e.context == context)
                        .cloned()
                        .ok_or(ResourceError::Unavailable)
                })
                .transpose()?;
            if self.resources.restorers.len() >= limits.max_restorers {
                return Err(ResourceError::Limit);
            }
            let growth = super::root_accounting::registration_bytes::<RestorationRegistration>()
                .map_err(|_| ResourceError::Limit)?;
            self.check_root_growth(growth, recorder)
                .map_err(owner_error)?;
            self.resources
                .restorers
                .try_reserve(1)
                .map_err(|_| ResourceError::Capacity)?;
            let registration = Arc::new(RestorationRegistration::new(
                self.namespace.clone(),
                limits,
                annotations,
                encoding,
            ));
            self.resources.restorers.push(registration.clone());
            Ok(AnnotationRestorerPort { registration })
        })();
        if matches!(result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture(
            recorder,
            Boundary::AnnotationRestorationPreparation,
            &result,
        );
        result
    }
    pub(in crate::coordinator) fn validate_restoration_read(
        &self,
        package: &AnnotationRestoration,
        pair: Option<&OrdinaryPairEvidence>,
        recorder: &mut Recorder,
    ) -> Result<(), ResourceError> {
        if !self
            .resources
            .restorers
            .iter()
            .any(|r| Arc::ptr_eq(r, &package.registration))
        {
            return Err(ResourceError::Unavailable);
        }
        let task = self.read_task(package.capture.origin())?;
        if task.restoration.is_some() {
            return Err(ResourceError::Retained);
        }
        if task.contested {
            return Err(ResourceError::Contested);
        }
        if task.stopped || task.refused {
            return Err(ResourceError::Unavailable);
        }
        if task
            .capture
            .as_ref()
            .is_none_or(|c| !Arc::ptr_eq(c, &package.capture.data))
        {
            return Err(ResourceError::Unavailable);
        }
        self.validate_read_basis(task, pair)?;
        self.annotation_read_catalog_current(
            task.catalog.as_ref(),
            &task.registration,
            &task.identity,
            true,
            recorder,
        )?;
        if package.logical_size() != Some(package.charge) {
            return Err(ResourceError::Faulted);
        }
        if package.charge > task.registration.limits.max_restoration_bytes {
            return Err(ResourceError::Limit);
        }
        if self.resources.tasks.iter().flatten().any(|t| {
            t.identity == task.identity
                && (t.phase == SavePhase::Queued || !t.safe() || !t.quiescent)
        }) {
            return Err(ResourceError::Blocked);
        }
        Ok(())
    }
    pub(in crate::coordinator) fn finish_annotation_restoration(
        &mut self,
        package: AnnotationRestoration,
        outcome: AnnotationRestorationOutcome,
    ) {
        let slot = package.capture.origin().slot;
        self.resources.reads[slot]
            .as_mut()
            .expect("validated read")
            .restoration = Some(RestorationProvenance { package, outcome });
    }
    pub fn annotation_restoration_outcome(
        &self,
        id: &AnnotationReadId,
        recorder: &mut Recorder,
    ) -> Result<AnnotationRestorationOutcome, ResourceError> {
        let result = (|| {
            let task = self.read_task(id)?;
            let provenance = task
                .restoration
                .as_ref()
                .ok_or(ResourceError::MissingEvidence)?;
            let original = &provenance.outcome;
            let mut annotations = Vec::new();
            annotations
                .try_reserve_exact(original.annotations.len())
                .map_err(|_| ResourceError::Capacity)?;
            annotations.extend(original.annotations.iter().cloned());
            Ok(AnnotationRestorationOutcome {
                source: original.source.clone(),
                set_version: original.set_version,
                changed: original.changed,
                context: provenance.package.registration.context.clone(),
                annotations,
                evidence_contested: task.contested,
                loaded_baseline_adopted: original.loaded_baseline_adopted,
            })
        })();
        capture(recorder, Boundary::AnnotationRestorationQuery, &result);
        result
    }
    pub(super) fn outstanding_resource_work(&self) -> usize {
        self.outstanding_source_reads()
            + self
                .resources
                .tasks
                .iter()
                .flatten()
                .filter(|t| t.phase == SavePhase::Dispatched && !t.quiescent)
                .count()
            + self
                .resources
                .reads
                .iter()
                .flatten()
                .filter(|t| t.phase == ReadPhase::Submitted && !t.quiescent)
                .count()
    }
    fn read_task(&self, id: &AnnotationReadId) -> Result<&ReadTask, ResourceError> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return Err(ResourceError::Unavailable);
        }
        let task = self
            .resources
            .reads
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(ResourceError::Unavailable)?;
        let catalog_bytes = task
            .catalog
            .as_ref()
            .map(|c| c.annotation_intact(&self.namespace, &task.registration, &task.identity))
            .transpose()?
            .unwrap_or(0);
        if read_task_charge(
            self.resources.limits.ok_or(ResourceError::Faulted)?,
            &task.identity,
            &task.association,
            catalog_bytes,
        ) != Some(task.charge)
        {
            return Err(ResourceError::Faulted);
        }
        Ok(task)
    }
    fn validate_read_basis(
        &self,
        task: &ReadTask,
        pair: Option<&OrdinaryPairEvidence>,
    ) -> Result<(), ResourceError> {
        let doc = self.validate_version(&task.source).map_err(owner_error)?;
        if doc.annotation_generation != task.set
            || self
                .current_annotation_association(&task.source.document)
                .as_ref()
                != Some(&task.association)
        {
            return Err(ResourceError::Stale);
        }
        self.provider_current(&task.registration)?;
        self.validate_ordinary_pair(&task.source.document, pair)
    }
    pub fn admit_annotation_read(
        &mut self,
        expected: &DocumentVersion,
        expected_set: u64,
        target: AnnotationReadTarget,
        strength: ReadStrength,
        pair: Option<OrdinaryPairEvidence>,
        recorder: &mut Recorder,
    ) -> Result<AnnotationReadId, ResourceError> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            self.ensure_admission_unfenced().map_err(owner_error)?;
            self.ensure_annotation_reader_open()?;
            let limits = self.resources.limits.ok_or(ResourceError::Unsupported)?;
            if self.resources.annotation_read_admission_stopped
                || matches!(self.resources.read_operations, operations::State::Closed)
            {
                return Err(ResourceError::Unavailable);
            }
            let doc = self.validate_version(expected).map_err(owner_error)?;
            if doc.annotation_generation != expected_set {
                return Err(ResourceError::Stale);
            }
            if strength != ReadStrength::CapturedRead {
                return Err(ResourceError::Unsupported);
            }
            self.provider_current(&target.registration)?;
            self.annotation_read_catalog_current(
                target.catalog.as_ref(),
                &target.registration,
                &target.identity,
                false,
                recorder,
            )?;
            self.validate_ordinary_pair(&expected.document, pair.as_ref())?;
            let association = self
                .current_annotation_association(&expected.document)
                .ok_or(ResourceError::Unbound)?;
            if target.identity != association.binding.identity {
                return Err(ResourceError::Unavailable);
            }
            if self.resources.reads.len() >= limits.max_issued_reads
                || self.resources.reads.iter().flatten().count() >= limits.max_live_reads
            {
                return Err(ResourceError::Limit);
            }
            let catalog_bytes = target
                .catalog
                .as_ref()
                .map(|c| c.native_bytes())
                .transpose()?
                .unwrap_or(0);
            let size = read_task_charge(limits, &target.identity, &association, catalog_bytes)
                .ok_or(ResourceError::Limit)?;
            if self
                .resources
                .held_read_bytes
                .checked_add(size)
                .is_none_or(|n| n > limits.max_held_read_bytes)
            {
                return Err(ResourceError::Limit);
            }
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(limits.max_blob_bytes)
                .map_err(|_| ResourceError::Capacity)?;
            let mut chunks = Vec::new();
            chunks
                .try_reserve_exact(limits.max_read_chunks)
                .map_err(|_| ResourceError::Capacity)?;
            self.resources
                .reads
                .try_reserve(1)
                .map_err(|_| ResourceError::Capacity)?;
            let id = AnnotationReadId {
                owner: self.namespace.clone(),
                slot: self.resources.reads.len(),
            };
            self.resources.reads.push(Some(ReadTask {
                queue_timing: queue_timing::Stamp::Off,
                catalog: target.catalog,
                operation: None,
                source: expected.clone(),
                set: expected_set,
                association,
                registration: target.registration,
                identity: target.identity,
                strength,
                phase: ReadPhase::Ready,
                quiescent: false,
                contested: false,
                stopped: false,
                refused: false,
                nonce: None,
                start: None,
                chunks,
                bytes,
                capture: None,
                terminal: None,
                witnesses: [None, None],
                charge: size,
                restoration: None,
            }));
            self.resources.held_read_bytes += size;
            let stamp = self.resource_queue_admitted();
            self.resources.reads[id.slot]
                .as_mut()
                .expect("installed annotation read")
                .queue_timing = stamp;
            Ok(id)
        })();
        if matches!(result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture(recorder, Boundary::AnnotationReadAdmission, &result);
        result
    }
    pub fn submit_annotation_read(
        &mut self,
        id: &AnnotationReadId,
        pair: Option<OrdinaryPairEvidence>,
        recorder: &mut Recorder,
    ) -> Result<AnnotationReadDispatch, ResourceError> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            let task = self.read_task(id)?;
            if task.phase != ReadPhase::Ready || task.stopped {
                return Err(ResourceError::Unavailable);
            }
            self.validate_read_basis(task, pair.as_ref())?;
            self.annotation_read_catalog_current(
                task.catalog.as_ref(),
                &task.registration,
                &task.identity,
                false,
                recorder,
            )?;
            if self.outstanding_resource_work()
                >= self
                    .resources
                    .limits
                    .ok_or(ResourceError::Unsupported)?
                    .max_outstanding
            {
                return Err(ResourceError::Limit);
            }
            let dispatch = AnnotationReadDispatch {
                id: id.clone(),
                registration: task.registration.clone(),
                identity: task.identity.clone(),
                nonce: Arc::new(()),
                strength: task.strength,
            };
            let task = self.resources.reads[id.slot]
                .as_mut()
                .expect("validated task");
            task.phase = ReadPhase::Submitted;
            task.queue_timing.finish(true);
            task.nonce = Some(dispatch.nonce.clone());
            Ok(dispatch)
        })();
        if matches!(result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture(recorder, Boundary::AnnotationReadSubmission, &result);
        result
    }
    /// Protected original-issuer intake, independent of later disclosure or fault.
    pub fn record_annotation_read(
        &mut self,
        evidence: AnnotationReadEvidence,
        recorder: &mut Recorder,
    ) -> Result<EvidenceDisposition, ResourceError> {
        let timing = recorder.begin_service_timing();
        let result = (|| {
            let task = self.read_task(&evidence.id)?;
            if task.phase != ReadPhase::Submitted
                || !Arc::ptr_eq(&task.registration, &evidence.registration)
                || task
                    .nonce
                    .as_ref()
                    .is_none_or(|n| !Arc::ptr_eq(n, &evidence.nonce))
            {
                return Err(ResourceError::Unavailable);
            }
            let result_claim = match &evidence.atom {
                ReadAtom::Chunk { bytes, .. } => !bytes.0.is_empty(),
                ReadAtom::End { .. } | ReadAtom::Failure(_) => true,
                ReadAtom::Start { .. } | ReadAtom::Quiescence => false,
            };
            let task = self.resources.reads[evidence.id.slot]
                .as_mut()
                .expect("validated task");
            let result = task.accept(&evidence.id, evidence.atom);
            if result.is_ok() && result_claim {
                task.queue_timing.accepted_result();
            }
            result
        })();
        if result == Ok(EvidenceDisposition::Contradiction) {
            recorder.record(Observation {
                boundary: Boundary::AnnotationReadEvidence,
                disposition: Disposition::Unknown,
                service_time: timing.finish(),
            });
        } else {
            capture_service(
                recorder,
                Boundary::AnnotationReadEvidence,
                &result,
                timing.finish(),
            );
        }
        result
    }
    pub fn annotation_read_capture(
        &self,
        id: &AnnotationReadId,
        pair: Option<OrdinaryPairEvidence>,
        recorder: &mut Recorder,
    ) -> Result<CapturedAnnotationRead, ResourceError> {
        let timing = recorder.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            let task = self.read_task(id)?;
            if task.contested {
                return Err(ResourceError::Contested);
            }
            if task.stopped || task.refused {
                return Err(ResourceError::Unavailable);
            }
            self.validate_read_basis(task, pair.as_ref())?;
            self.annotation_read_catalog_current(
                task.catalog.as_ref(),
                &task.registration,
                &task.identity,
                false,
                recorder,
            )?;
            Ok(CapturedAnnotationRead {
                data: task.capture.clone().ok_or(ResourceError::MissingEvidence)?,
            })
        })();
        capture_service(
            recorder,
            Boundary::AnnotationReadCapture,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn annotation_read_status(
        &self,
        id: &AnnotationReadId,
        recorder: &mut Recorder,
    ) -> Result<AnnotationReadStatus, ResourceError> {
        let timing = recorder.begin_service_timing();
        let result = self.read_task(id).map(ReadTask::status);
        capture_service(recorder, Boundary::Outcome, &result, timing.finish());
        result
    }
    pub fn stop_annotation_read(
        &mut self,
        id: &AnnotationReadId,
        recorder: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let timing = recorder.begin_service_timing();
        let result = self.read_task(id).map(|_| ());
        if result.is_ok() {
            self.resources.reads[id.slot]
                .as_mut()
                .expect("validated task")
                .stopped = true;
            self.resources.reads[id.slot]
                .as_mut()
                .expect("validated task")
                .queue_timing
                .finish(false);
        }
        capture_service(
            recorder,
            Boundary::AnnotationReadControl,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn retire_annotation_read(
        &mut self,
        id: &AnnotationReadId,
        recorder: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let timing = recorder.begin_service_timing();
        let mut retired_operation = false;
        let result = (|| {
            let task = self.read_task(id)?;
            let operation = self.annotation_read_operation_retirement(id)?;
            let queue_timing = task.queue_timing;
            if (task.phase == ReadPhase::Ready && !task.stopped)
                || task.restoration.is_some()
                || (task.phase == ReadPhase::Submitted && !task.quiescent)
                || task
                    .nonce
                    .as_ref()
                    .is_some_and(|n| Arc::strong_count(n) != 1)
                || task
                    .capture
                    .as_ref()
                    .is_some_and(|c| Arc::strong_count(c) != 1)
            {
                return Err(ResourceError::Retained);
            }
            let Some(remaining) = self.resources.held_read_bytes.checked_sub(task.charge) else {
                self.faulted = true;
                return Err(ResourceError::Faulted);
            };
            if let Some(index) = operation {
                self.spend_annotation_read_operation(index);
            }
            self.resources.reads[id.slot] = None;
            self.resources.queue_timing.retire(
                queue_timing::ResourceQueueKindV1::AnnotationRead,
                queue_timing,
            );
            self.resources.held_read_bytes = remaining;
            retired_operation = operation.is_some();
            Ok(())
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        let elapsed = timing.finish();
        if retired_operation {
            capture_service(
                recorder,
                Boundary::AnnotationReadOperationRetirement,
                &result,
                elapsed,
            );
        }
        capture_service(recorder, Boundary::AnnotationReadControl, &result, elapsed);
        result
    }
}
