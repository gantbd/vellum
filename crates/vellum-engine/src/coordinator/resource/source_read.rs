//! Source reader owns pending transcripts, not documents. Activation is separate.
use super::*;
use std::mem::size_of;
mod activation;
pub(in crate::coordinator) use activation::PreparedSourceReadActivation;
mod activation_authority;
mod operations;
mod shutdown_inventory;
pub use activation_authority::{SourceLoadAuthorityFactsV1, SourceLoadAuthorityV1};
#[cfg(test)]
mod activation_tests;
#[cfg(test)]
mod tests;
pub use operations::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceReaderLimitsV1 {
    pub max_live: usize,
    pub max_issued: usize,
    pub max_input_bytes: usize,
    pub max_chunks: usize,
    pub max_held_bytes: usize,
    pub max_store_bytes: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceReaderUsageV1 {
    pub issued: usize,
    pub live: usize,
    pub outstanding: usize,
    pub held_bytes: usize,
    pub reserved_bytes: usize,
    pub stopped: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceReadActivationDispositionV1 {
    Created,
    Joined,
    AlreadyActivated,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceReadActivationV1 {
    pub disposition: SourceReadActivationDispositionV1,
    pub document: DocumentVersion,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceReadStatusV1 {
    pub phase: ReadPhase,
    pub stopped: bool,
    pub quiescent: bool,
    pub contested: bool,
    pub decode_refused: bool,
    pub candidate_ready: bool,
    pub received_chunks: usize,
    pub received_bytes: usize,
    pub failure: Option<SourceReadFailureV1>,
    pub activated: Option<SourceReadActivationV1>,
}
#[derive(Default)]
pub(super) enum State {
    #[default]
    Unconfigured,
    Active(Box<Store>),
    Closed,
}
impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceReader(<owned>)")
    }
}
impl State {
    pub(super) fn queue_timing_unused(&self) -> bool {
        match self {
            Self::Unconfigured => true,
            Self::Active(s) => s.rows.is_empty(),
            Self::Closed => false,
        }
    }
    pub(super) fn queue_snapshot(
        &self,
        out: &mut queue_timing::ResourceQueueSnapshotV1,
        now: Option<std::time::Instant>,
    ) {
        if let Self::Active(s) = self {
            for t in s.rows.iter().flatten() {
                out.observe(
                    queue_timing::ResourceQueueKindV1::SourceRead,
                    t.queue_timing,
                    false,
                    now,
                );
            }
        }
    }
    pub(super) fn document_pins(&self, document: &DocumentId) -> usize {
        match self {
            Self::Unconfigured | Self::Closed => 0,
            Self::Active(store) => store
                .rows
                .iter()
                .flatten()
                .filter(|task| {
                    task.activated
                        .as_ref()
                        .is_some_and(|(activation, _)| activation.document.document == *document)
                })
                .count(),
        }
    }
}
pub(super) struct Store {
    operations: operations::State,
    limits: SourceReaderLimitsV1,
    rows: Vec<Option<Task>>,
    held: usize,
    stopped: bool,
    reservation: retention::Reservation,
}
struct Task {
    queue_timing: queue_timing::Stamp,
    catalog: Option<Arc<ReadCatalogCaptureV1>>,
    operation: Option<usize>,
    registration: Arc<Registration>,
    identity: Identity,
    nonce: Option<Arc<()>>,
    phase: ReadPhase,
    stopped: bool,
    quiescent: bool,
    contested: bool,
    refused: bool,
    start: Option<SourceReadVersion>,
    terminal: Option<SourceReadAtom>,
    witnesses: [Option<SourceReadAtom>; 2],
    chunks: Vec<(usize, usize)>,
    bytes: Vec<u8>,
    candidate: Option<Utf8Source>,
    candidate_marker: Option<Arc<()>>,
    activated: Option<(SourceReadActivationV1, BindingKey)>,
    charge: usize,
}
fn add(a: usize, b: usize) -> Result<usize, ResourceError> {
    a.checked_add(b).ok_or(ResourceError::Limit)
}
fn parent(e: RetentionErrorV1) -> ResourceError {
    match e {
        RetentionErrorV1::Limit => ResourceError::Limit,
        RetentionErrorV1::Unavailable => ResourceError::Unavailable,
        RetentionErrorV1::Faulted => ResourceError::Faulted,
    }
}
fn task_charge(
    l: SourceReaderLimitsV1,
    r: ResourceLimits,
    key: usize,
    source: usize,
) -> Result<usize, ResourceError> {
    [
        size_of::<Task>(),
        key.checked_mul(2).ok_or(ResourceError::Limit)?,
        source,
        l.max_input_bytes,
        l.max_chunks
            .checked_mul(size_of::<(usize, usize)>())
            .ok_or(ResourceError::Limit)?,
        r.max_blob_bytes
            .checked_mul(2)
            .ok_or(ResourceError::Limit)?,
        r.max_validator_bytes
            .checked_mul(8)
            .ok_or(ResourceError::Limit)?,
    ]
    .into_iter()
    .try_fold(0, add)
}
impl SourceReaderLimitsV1 {
    pub fn reserved_bytes(self) -> Result<usize, ResourceError> {
        add(
            add(size_of::<State>(), size_of::<Store>())?,
            add(
                self.max_held_bytes,
                self.max_issued
                    .checked_mul(size_of::<Option<Task>>())
                    .ok_or(ResourceError::Limit)?,
            )?,
        )
    }
}
impl Task {
    fn conflict(
        &mut self,
        prior: SourceReadAtom,
        incoming: SourceReadAtom,
    ) -> Result<EvidenceDisposition, ResourceError> {
        if self.contested {
            return Err(ResourceError::EvidenceCapacity);
        }
        self.witnesses = [Some(prior), Some(incoming)];
        self.contested = true;
        self.candidate = None;
        self.candidate_marker = None;
        Ok(EvidenceDisposition::Contradiction)
    }
    fn accept(
        &mut self,
        atom: SourceReadAtom,
        limits: SourceReaderLimitsV1,
        source_limit: usize,
        r: &mut Recorder,
    ) -> Result<EvidenceDisposition, ResourceError> {
        if atom == SourceReadAtom::Quiescence {
            let old = self.quiescent;
            self.quiescent = true;
            return Ok(if old {
                EvidenceDisposition::Duplicate
            } else {
                EvidenceDisposition::Accepted
            });
        }
        if self.witnesses.iter().flatten().any(|v| v == &atom) {
            return Ok(EvidenceDisposition::Duplicate);
        }
        let prior = match &atom {
            SourceReadAtom::Start(_) => self
                .start
                .as_ref()
                .map(|v| SourceReadAtom::Start(v.clone())),
            SourceReadAtom::Chunk {
                sequence,
                offset,
                bytes,
            } => match self.chunks.get(*sequence) {
                Some((old_offset, len))
                    if old_offset == offset
                        && self.bytes.get(*old_offset..old_offset + len)
                            == Some(bytes.0.as_ref()) =>
                {
                    Some(atom.clone())
                }
                Some((old_offset, len)) => Some(SourceReadAtom::Chunk {
                    sequence: *sequence,
                    offset: *old_offset,
                    bytes: blob(
                        &self.bytes[*old_offset..old_offset + len],
                        self.registration.limits.max_blob_bytes,
                        false,
                    )?,
                }),
                None => None,
            },
            SourceReadAtom::End { .. } | SourceReadAtom::Failure(_) => self.terminal.clone(),
            SourceReadAtom::Quiescence => unreachable!(),
        };
        if let Some(prior) = prior {
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
        if self.refused && !matches!(atom, SourceReadAtom::Failure(_)) {
            return Err(ResourceError::Unavailable);
        }
        match atom {
            SourceReadAtom::Start(version) => self.start = Some(version),
            SourceReadAtom::Chunk {
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
                if self.chunks.len() >= limits.max_chunks
                    || self
                        .bytes
                        .len()
                        .checked_add(bytes.0.len())
                        .is_none_or(|n| n > limits.max_input_bytes)
                {
                    self.refused = true;
                    return Err(ResourceError::Limit);
                }
                let before = self.bytes.len();
                self.bytes.extend_from_slice(&bytes.0);
                if std::str::from_utf8(&self.bytes).is_err_and(|e| e.error_len().is_some()) {
                    self.bytes.truncate(before);
                    self.refused = true;
                    return Err(ResourceError::Source(
                        crate::source::SourceError::InvalidUtf8,
                    ));
                }
                self.chunks.push((offset, bytes.0.len()));
            }
            SourceReadAtom::End {
                version,
                chunks,
                bytes,
            } => {
                if self.start.as_ref() != Some(&version)
                    || (!self.stopped && (chunks != self.chunks.len() || bytes != self.bytes.len()))
                {
                    return Err(ResourceError::Malformed);
                }
                self.terminal = Some(SourceReadAtom::End {
                    version,
                    chunks,
                    bytes,
                });
                if !self.stopped {
                    match Utf8Source::decode_observed(&self.bytes, source_limit, r) {
                        Ok(candidate) => {
                            self.candidate_marker = Some(Arc::new(()));
                            self.candidate = Some(candidate);
                        }
                        Err(error) => {
                            self.refused = true;
                            return Err(ResourceError::Source(error));
                        }
                    }
                }
            }
            SourceReadAtom::Failure(reason) => {
                self.candidate = None;
                self.candidate_marker = None;
                self.terminal = Some(SourceReadAtom::Failure(reason));
            }
            SourceReadAtom::Quiescence => unreachable!(),
        }
        Ok(EvidenceDisposition::Accepted)
    }
}
impl SourceCoordinator {
    pub(super) fn source_read_queue_stamp(
        &self,
        id: &SourceReadIdV1,
    ) -> Result<queue_timing::Stamp, ResourceError> {
        self.source_read_task(id).map(|t| t.queue_timing)
    }
    #[cfg(test)]
    pub(in crate::coordinator::resource) fn test_corrupt_source_terminal(
        &mut self,
        id: &SourceReadIdV1,
    ) {
        self.source_read_task_mut(id).unwrap().terminal = Some(SourceReadAtom::End {
            version: SourceReadVersion {
                epoch: None,
                token: Blob(Arc::from([])),
            },
            chunks: 0,
            bytes: 0,
        });
    }
    fn validate_source_reader(&self) -> Result<(), ResourceError> {
        let s = self.source_reader()?;
        self.retention
            .validate_exact(&s.reservation, s.limits.reserved_bytes()?)
            .map_err(parent)?;
        if s.rows.len() > s.limits.max_issued
            || s.rows.iter().flatten().count() > s.limits.max_live
            || s.limits
                .reserved_bytes()
                .ok()
                .is_none_or(|n| n > s.limits.max_store_bytes)
        {
            return Err(ResourceError::Faulted);
        }
        let mut held = 0usize;
        for t in s.rows.iter().flatten() {
            if !Arc::ptr_eq(&t.registration.owner, &self.namespace)
                || t.charge
                    != add(
                        task_charge(
                            s.limits,
                            t.registration.limits,
                            t.identity.key.0.len(),
                            self.limits.source.max_source_bytes,
                        )?,
                        t.catalog
                            .as_ref()
                            .map(|c| c.source_intact(&self.namespace, &t.registration, &t.identity))
                            .transpose()?
                            .unwrap_or(0),
                    )?
                || t.bytes.len() > s.limits.max_input_bytes
                || t.chunks.len() > s.limits.max_chunks
                || (t.phase == ReadPhase::Ready) != t.nonce.is_none()
                || t.candidate.is_some() != t.candidate_marker.is_some()
            {
                return Err(ResourceError::Faulted);
            }
            held = held.checked_add(t.charge).ok_or(ResourceError::Faulted)?;
        }
        if held != s.held || held > s.limits.max_held_bytes {
            return Err(ResourceError::Faulted);
        }
        self.validate_source_read_operations()?;
        Ok(())
    }
    fn source_reader(&self) -> Result<&Store, ResourceError> {
        match &self.resources.source_reader {
            State::Active(s) => Ok(s),
            _ => Err(ResourceError::Unavailable),
        }
    }
    fn source_reader_mut(&mut self) -> Result<&mut Store, ResourceError> {
        match &mut self.resources.source_reader {
            State::Active(s) => Ok(s),
            _ => Err(ResourceError::Unavailable),
        }
    }
    fn source_read_task(&self, id: &SourceReadIdV1) -> Result<&Task, ResourceError> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return Err(ResourceError::Unavailable);
        }
        self.source_reader()?
            .rows
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(ResourceError::Unavailable)
    }
    fn source_read_task_mut(&mut self, id: &SourceReadIdV1) -> Result<&mut Task, ResourceError> {
        self.source_read_task(id)?;
        Ok(self.source_reader_mut()?.rows[id.slot]
            .as_mut()
            .expect("validated source read"))
    }
    pub(super) fn outstanding_source_reads(&self) -> usize {
        self.source_reader().map_or(0, |s| {
            s.rows
                .iter()
                .flatten()
                .filter(|t| t.phase == ReadPhase::Submitted && !t.quiescent)
                .count()
        })
    }
    pub fn configure_source_reader_v1(
        &mut self,
        limits: SourceReaderLimitsV1,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            if self.resources.limits.is_none() {
                return Err(ResourceError::Unsupported);
            }
            match &self.resources.source_reader {
                State::Active(s) => {
                    self.validate_source_reader()?;
                    return if s.limits == limits {
                        Ok(())
                    } else {
                        Err(ResourceError::Unavailable)
                    };
                }
                State::Closed => return Err(ResourceError::Unavailable),
                State::Unconfigured => {}
            }
            self.ensure_admission_unfenced().map_err(owner_error)?;
            let total = limits.reserved_bytes()?;
            if limits.max_live > limits.max_issued || total > limits.max_store_bytes {
                return Err(ResourceError::Limit);
            }
            self.retention.check(total).map_err(parent)?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(limits.max_issued)
                .map_err(|_| ResourceError::Capacity)?;
            let reservation = self.retention.reserve(total).map_err(parent)?;
            self.resources.source_reader = State::Active(Box::new(Store {
                operations: Default::default(),
                limits,
                rows,
                held: 0,
                stopped: false,
                reservation,
            }));
            Ok(())
        })();
        if matches!(&result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture(r, Boundary::SourceReadConfiguration, &result);
        result
    }
    pub fn admit_source_read_v1(
        &mut self,
        target: SourceReadTargetV1,
        strength: ReadStrength,
        r: &mut Recorder,
    ) -> Result<SourceReadIdV1, ResourceError> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            self.ensure_admission_unfenced().map_err(owner_error)?;
            self.provider_current(&target.registration)?;
            self.source_read_catalog_current(
                target.catalog.as_ref(),
                &target.registration,
                &target.identity,
                false,
                r,
            )?;
            if strength != ReadStrength::CapturedRead {
                return Err(ResourceError::Unsupported);
            }
            self.validate_source_reader()?;
            let s = self.source_reader()?;
            let l = s.limits;
            if s.stopped {
                return Err(ResourceError::Unavailable);
            }
            if s.rows.len() >= l.max_issued || s.rows.iter().flatten().count() >= l.max_live {
                return Err(ResourceError::Limit);
            }
            let charge = add(
                task_charge(
                    l,
                    target.registration.limits,
                    target.identity.key.0.len(),
                    self.limits.source.max_source_bytes,
                )?,
                target
                    .catalog
                    .as_ref()
                    .map(|c| c.native_bytes())
                    .transpose()?
                    .unwrap_or(0),
            )?;
            if add(s.held, charge)? > l.max_held_bytes {
                return Err(ResourceError::Limit);
            }
            if s.rows.len() >= s.rows.capacity() {
                return Err(ResourceError::Faulted);
            }
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(l.max_input_bytes)
                .map_err(|_| ResourceError::Capacity)?;
            let mut chunks = Vec::new();
            chunks
                .try_reserve_exact(l.max_chunks)
                .map_err(|_| ResourceError::Capacity)?;
            let id = SourceReadIdV1 {
                owner: self.namespace.clone(),
                slot: s.rows.len(),
            };
            let queue_profile = self.resources.queue_timing.profile;
            let s = self.source_reader_mut()?;
            s.rows.push(Some(Task {
                queue_timing: queue_timing::Stamp::Off,
                catalog: target.catalog,
                operation: None,
                registration: target.registration,
                identity: target.identity,
                nonce: None,
                phase: ReadPhase::Ready,
                stopped: false,
                quiescent: false,
                contested: false,
                refused: false,
                start: None,
                terminal: None,
                witnesses: [None, None],
                chunks,
                bytes,
                candidate: None,
                candidate_marker: None,
                activated: None,
                charge,
            }));
            s.held += charge;
            // All fallible admission checks precede installation. Optional
            // clock observation cannot add a post-install refusal or callback.
            s.rows[id.slot]
                .as_mut()
                .expect("installed source read")
                .queue_timing = queue_timing::Stamp::admitted(queue_profile);
            Ok(id)
        })();
        if matches!(&result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture(r, Boundary::SourceReadAdmission, &result);
        result
    }
    pub fn submit_source_read_v1(
        &mut self,
        id: &SourceReadIdV1,
        r: &mut Recorder,
    ) -> Result<SourceReadDispatchV1, ResourceError> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            self.validate_source_reader()?;
            let task = self.source_read_task(id)?;
            if task.phase != ReadPhase::Ready || task.stopped {
                return Err(ResourceError::Unavailable);
            }
            self.provider_current(&task.registration)?;
            self.source_read_catalog_current(
                task.catalog.as_ref(),
                &task.registration,
                &task.identity,
                false,
                r,
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
            let l = self.source_reader()?.limits;
            let dispatch = SourceReadDispatchV1 {
                id: id.clone(),
                registration: task.registration.clone(),
                identity: task.identity.clone(),
                nonce: Arc::new(()),
                max_input_bytes: l.max_input_bytes,
                max_chunks: l.max_chunks,
            };
            let task = self.source_read_task_mut(id)?;
            task.phase = ReadPhase::Submitted;
            task.queue_timing.finish(true);
            task.nonce = Some(dispatch.nonce.clone());
            Ok(dispatch)
        })();
        if matches!(&result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture(r, Boundary::SourceReadSubmission, &result);
        result
    }
    pub fn record_source_read_v1(
        &mut self,
        evidence: SourceReadEvidenceV1,
        r: &mut Recorder,
    ) -> Result<EvidenceDisposition, ResourceError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let t = self.source_read_task(&evidence.id)?;
            if t.phase != ReadPhase::Submitted
                || !Arc::ptr_eq(&t.registration, &evidence.registration)
                || t.nonce
                    .as_ref()
                    .is_none_or(|n| !Arc::ptr_eq(n, &evidence.nonce))
            {
                return Err(ResourceError::Unavailable);
            }
            let limits = self.source_reader()?.limits;
            let source_limit = self.limits.source.max_source_bytes;
            let result_claim = match &evidence.atom {
                SourceReadAtom::Chunk { bytes, .. } => !bytes.0.is_empty(),
                SourceReadAtom::End { .. } | SourceReadAtom::Failure(_) => true,
                SourceReadAtom::Start(_) | SourceReadAtom::Quiescence => false,
            };
            let task = self.source_read_task_mut(&evidence.id)?;
            let result = task.accept(evidence.atom, limits, source_limit, r);
            if result.is_ok() && result_claim {
                task.queue_timing.accepted_result();
            }
            result
        })();
        if result == Ok(EvidenceDisposition::Contradiction) {
            r.record(Observation {
                boundary: Boundary::SourceReadEvidence,
                disposition: Disposition::Unknown,
                service_time: timing.finish(),
            });
        } else {
            capture_service(r, Boundary::SourceReadEvidence, &result, timing.finish());
        }
        result
    }
    pub fn source_read_status_v1(
        &self,
        id: &SourceReadIdV1,
        r: &mut Recorder,
    ) -> Result<SourceReadStatusV1, ResourceError> {
        let timing = r.begin_service_timing();
        let result = self.source_read_task(id).map(|t| SourceReadStatusV1 {
            phase: t.phase,
            stopped: t.stopped,
            quiescent: t.quiescent,
            contested: t.contested,
            decode_refused: t.refused,
            candidate_ready: t.candidate.is_some(),
            received_chunks: t.chunks.len(),
            received_bytes: t.bytes.len(),
            failure: match t.terminal {
                Some(SourceReadAtom::Failure(f)) => Some(f),
                _ => None,
            },
            activated: t.activated.as_ref().map(|(v, _)| v.clone()),
        });
        capture_service(r, Boundary::SourceReadQuery, &result, timing.finish());
        result
    }
    pub fn activate_source_read_v1(
        &mut self,
        id: &SourceReadIdV1,
        r: &mut Recorder,
    ) -> Result<SourceReadActivationV1, ResourceError> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let plan = self.prepare_source_read_activation_native(id, r)?;
            self.commit_source_read_activation_native(plan, r)
        }));
        let result = match caught {
            Ok(v) => v,
            Err(p) => {
                self.faulted = true;
                capture_service(
                    r,
                    Boundary::SourceReadActivation,
                    &Err::<SourceReadActivationV1, _>(ResourceError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(p)
            }
        };
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(r, Boundary::SourceReadActivation, &result, timing.finish());
        result
    }
    pub fn stop_source_read_v1(
        &mut self,
        id: &SourceReadIdV1,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let timing = r.begin_service_timing();
        let result = self.source_read_task_mut(id).map(|t| {
            t.stopped = true;
            t.queue_timing.finish(false);
            t.candidate = None;
            t.candidate_marker = None;
        });
        capture_service(r, Boundary::SourceReadControl, &result, timing.finish());
        result
    }
    pub fn retire_source_read_v1(
        &mut self,
        id: &SourceReadIdV1,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let timing = r.begin_service_timing();
        let mut retired_operation = false;
        let result = (|| {
            self.validate_source_reader()?;
            let task = self.source_read_task(id)?;
            if (task.phase == ReadPhase::Ready && !task.stopped)
                || (task.phase == ReadPhase::Submitted && !task.quiescent)
                || task
                    .nonce
                    .as_ref()
                    .is_some_and(|n| Arc::strong_count(n) != 1)
            {
                return Err(ResourceError::Retained);
            }
            let held = self
                .source_reader()?
                .held
                .checked_sub(task.charge)
                .ok_or(ResourceError::Faulted)?;
            let operation = task.operation;
            let queue_timing = task.queue_timing;
            let was_faulted = self.faulted;
            self.faulted = true;
            if let Some(index) = operation {
                self.spend_source_read_operation(index);
            }
            let State::Active(s) = &mut self.resources.source_reader else {
                unreachable!("validated source reader")
            };
            s.rows[id.slot] = None;
            s.held = held;
            self.resources
                .queue_timing
                .retire(queue_timing::ResourceQueueKindV1::SourceRead, queue_timing);
            self.faulted = was_faulted;
            retired_operation = operation.is_some();
            Ok(())
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        let elapsed = timing.finish();
        if retired_operation {
            capture_service(r, Boundary::SourceReadOperationRetirement, &result, elapsed);
        }
        capture_service(r, Boundary::SourceReadRetirement, &result, elapsed);
        result
    }
    pub fn source_reader_usage_v1(&self) -> Result<SourceReaderUsageV1, ResourceError> {
        self.validate_source_reader()?;
        let s = self.source_reader()?;
        self.retention.validate(&s.reservation).map_err(parent)?;
        Ok(SourceReaderUsageV1 {
            issued: s.rows.len(),
            live: s.rows.iter().flatten().count(),
            outstanding: self.outstanding_source_reads(),
            held_bytes: s.held,
            reserved_bytes: s.limits.reserved_bytes()?,
            stopped: s.stopped,
        })
    }
    pub fn stop_source_read_admission_v1(&mut self, r: &mut Recorder) -> Result<(), ResourceError> {
        let timing = r.begin_service_timing();
        let result = self.source_reader_mut().map(|s| s.stopped = true);
        capture_service(
            r,
            Boundary::SourceReadConfiguration,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn retire_source_reader_v1(
        &mut self,
        owner_control: Option<bool>,
        retained_released: Option<bool>,
        delivery_drained: Option<bool>,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let result = (|| {
            self.validate_source_reader()?;
            let s = self.source_reader()?;
            if !s.stopped
                || matches!(s.operations, operations::State::Active(_))
                || s.rows.iter().any(Option::is_some)
                || s.held != 0
                || [owner_control, retained_released, delivery_drained]
                    .into_iter()
                    .any(|v| v != Some(true))
            {
                return Err(ResourceError::Retained);
            }
            self.retention.validate(&s.reservation).map_err(parent)?;
            let State::Active(mut s) =
                std::mem::replace(&mut self.resources.source_reader, State::Closed)
            else {
                unreachable!("validated source reader")
            };
            self.retention.release(&mut s.reservation).map_err(parent)
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture(r, Boundary::SourceReadRetirement, &result);
        result
    }
}
