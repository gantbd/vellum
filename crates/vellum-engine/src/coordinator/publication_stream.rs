//! Native source-only refinement of sampled full-projection publication streams.
//! Post-publication service, not a subscriber, lossless audit or F19 command facade.
use super::*;
mod shutdown_inventory;
use std::collections::VecDeque;
use std::mem::size_of;
#[cfg(test)]
pub(in crate::coordinator) mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourcePublicationErrorV1 {
    Unavailable,
    Limit,
    Capacity,
    Faulted,
}
type Error = SourcePublicationErrorV1;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourcePublicationLimitsV1 {
    pub max_streams: usize,
    /// Shared lifetime issuance for stream identities and visible generations.
    pub max_issued: usize,
    pub max_context_bytes: usize,
    pub max_projection_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_queue: usize,
    pub max_pool_bytes: usize,
}
impl SourcePublicationLimitsV1 {
    pub fn required_pool_bytes(self) -> Result<usize, Error> {
        let row = self
            .max_context_bytes
            .checked_add(
                self.max_projection_bytes
                    .checked_mul(2)
                    .ok_or(Error::Limit)?,
            )
            .and_then(|n| {
                n.checked_add(
                    self.max_queue
                        .checked_add(2)?
                        .checked_mul(self.max_reply_bytes)?,
                )
            })
            .ok_or(Error::Limit)?;
        size_of::<Pool>()
            .checked_add(
                self.max_issued
                    .checked_mul(size_of::<Option<Record>>())
                    .ok_or(Error::Limit)?,
            )
            .and_then(|n| n.checked_add(self.max_streams.checked_mul(row)?))
            .filter(|n| *n <= self.max_pool_bytes)
            .ok_or(Error::Limit)
    }
}
#[derive(Clone)]
pub struct SourcePublicationStreamIdV1 {
    owner: Arc<()>,
    slot: usize,
}
impl PartialEq for SourcePublicationStreamIdV1 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for SourcePublicationStreamIdV1 {}
#[derive(Clone, PartialEq, Eq)]
pub struct SourcePublicationGenerationV1 {
    stream: SourcePublicationStreamIdV1,
    serial: usize,
}
#[derive(PartialEq, Eq)]
pub struct SourcePublicationProjectionV1 {
    version: DocumentVersion,
    text: String,
}
impl SourcePublicationProjectionV1 {
    pub fn version(&self) -> &DocumentVersion {
        &self.version
    }
    pub fn text(&self) -> &str {
        &self.text
    }
}
#[derive(Clone)]
pub struct SourcePublicationEventV1 {
    pub before: SourcePublicationGenerationV1,
    pub after: SourcePublicationGenerationV1,
    pub projection: Arc<SourcePublicationProjectionV1>,
}
pub struct SourcePublicationBaselineV1 {
    pub stream: SourcePublicationStreamIdV1,
    pub generation: SourcePublicationGenerationV1,
    pub projection: Arc<SourcePublicationProjectionV1>,
}
pub enum SourcePublicationDeliveryV1 {
    Projection(SourcePublicationEventV1),
    Empty,
    Gap,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourcePublicationSampleV1 {
    Unchanged,
    Queued,
    Gap,
}
macro_rules! redacted { ($($t:ty),+) => { $(impl std::fmt::Debug for $t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(concat!(stringify!($t), "(<redacted>)")) }
})+ }; }
pub(in crate::coordinator) mod commands;
redacted!(
    SourcePublicationStreamIdV1,
    SourcePublicationGenerationV1,
    SourcePublicationProjectionV1,
    SourcePublicationEventV1,
    SourcePublicationBaselineV1,
    SourcePublicationDeliveryV1
);
/// Independently attributed host facts. The original scope binds these facts to
/// the retained observer; a lowered caller claim is not accepted loss evidence.
pub struct TrustedSourcePublicationFactsV1<'a> {
    pub original_scope: &'a SourceReadScopeV1,
    pub current: &'a TrustedSourceReadFactsV1<'a>,
    pub projection_allowed: Option<bool>,
    pub baseline_allowed: Option<bool>,
    pub max_reply_bytes: usize,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourcePublicationUsageV1 {
    pub streams: usize,
    pub queued: usize,
    pub gapped: usize,
    pub stopped: usize,
    pub issued_lifetime: usize,
    pub reserved_pool_bytes: usize,
    pub closed: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Active,
    Gap,
    Stopped,
}
struct Record {
    scope: SourceReadScopeV1,
    generation: SourcePublicationGenerationV1,
    baseline: Option<Arc<SourcePublicationProjectionV1>>,
    queue: VecDeque<SourcePublicationEventV1>,
    phase: Phase,
}
impl Record {
    fn valid(
        &self,
        id: &SourcePublicationStreamIdV1,
        limits: SourcePublicationLimitsV1,
        issued: usize,
    ) -> bool {
        if self.generation.stream != *id
            || self.generation.serial >= issued
            || self.queue.len() > limits.max_queue
        {
            return false;
        }
        let projection_valid = |p: &SourcePublicationProjectionV1| {
            p.version.document == self.scope.document
                && size_of::<SourcePublicationProjectionV1>()
                    .checked_add(p.text.len())
                    .is_some_and(|n| n <= limits.max_projection_bytes)
                && frame_bytes(p).is_ok_and(|n| n <= limits.max_reply_bytes)
        };
        match self.phase {
            Phase::Gap | Phase::Stopped => return self.baseline.is_none() && self.queue.is_empty(),
            Phase::Active => {
                if !self.baseline.as_ref().is_some_and(|p| projection_valid(p)) {
                    return false;
                }
            }
        }
        let mut previous = None;
        for event in &self.queue {
            if event.before.stream != *id
                || event.after.stream != *id
                || event.before.serial.checked_add(1) != Some(event.after.serial)
                || previous.is_some_and(|g| g != &event.before)
                || !projection_valid(&event.projection)
            {
                return false;
            }
            previous = Some(&event.after);
        }
        self.queue.back().is_none_or(|last| {
            last.after == self.generation
                && self
                    .baseline
                    .as_ref()
                    .is_some_and(|p| Arc::ptr_eq(p, &last.projection))
        })
    }
    fn close(&mut self, phase: Phase) {
        self.phase = phase;
        self.baseline = None;
        self.queue.clear();
    }
}
pub(super) struct Pool {
    owner: Arc<()>,
    authority: String,
    engine: String,
    limits: SourcePublicationLimitsV1,
    rows: Vec<Option<Record>>,
    issued: usize,
    reservation: retention::Reservation,
    charge: usize,
    commands: commands::State,
}
#[derive(Default)]
pub(super) enum State {
    #[default]
    Unconfigured,
    Active(Box<Pool>),
    Closed {
        issued: usize,
    },
}
redacted!(State);
impl State {
    pub(super) fn document_pins(
        &self,
        document: &DocumentId,
        ledger: &retention::RetentionState,
    ) -> Result<usize, CoordinatorError> {
        match self {
            Self::Unconfigured | Self::Closed { .. } => Ok(0),
            Self::Active(p) => {
                let command_pins = p
                    .commands
                    .document_pins(document, ledger, &p.owner)
                    .map_err(|_| CoordinatorError::Faulted)?;
                ledger
                    .validate_exact(&p.reservation, p.charge)
                    .map_err(|_| CoordinatorError::Faulted)?;
                if p.rows.len() > p.issued
                    || p.issued > p.limits.max_issued
                    || p.rows.iter().flatten().count() > p.limits.max_streams
                {
                    return Err(CoordinatorError::Faulted);
                }
                for (slot, row) in p
                    .rows
                    .iter()
                    .enumerate()
                    .filter_map(|(i, r)| r.as_ref().map(|r| (i, r)))
                {
                    let id = SourcePublicationStreamIdV1 {
                        owner: p.owner.clone(),
                        slot,
                    };
                    if !row.valid(&id, p.limits, p.issued) {
                        return Err(CoordinatorError::Faulted);
                    }
                }
                p.rows
                    .iter()
                    .flatten()
                    .filter(|r| r.scope.document == *document)
                    .count()
                    .checked_add(command_pins)
                    .ok_or(CoordinatorError::Faulted)
            }
        }
    }
}
fn admission_error(error: CoordinatorError) -> Error {
    match error {
        CoordinatorError::Faulted => Error::Faulted,
        _ => Error::Unavailable,
    }
}
fn parent(e: RetentionErrorV1) -> Error {
    match e {
        RetentionErrorV1::Unavailable => Error::Unavailable,
        RetentionErrorV1::Limit => Error::Limit,
        RetentionErrorV1::Faulted => Error::Faulted,
    }
}
fn owned(s: &str) -> Result<String, Error> {
    let mut v = String::new();
    v.try_reserve_exact(s.len()).map_err(|_| Error::Capacity)?;
    v.push_str(s);
    Ok(v)
}
fn source_error(e: SourceReadContextErrorV1) -> Error {
    match e {
        SourceReadContextErrorV1::Limit => Error::Limit,
        SourceReadContextErrorV1::Capacity => Error::Capacity,
        SourceReadContextErrorV1::Faulted => Error::Faulted,
        SourceReadContextErrorV1::Unavailable | SourceReadContextErrorV1::InvalidRequest => {
            Error::Unavailable
        }
    }
}
fn ordinary(f: &TrustedSourcePublicationFactsV1<'_>) -> bool {
    [
        f.current.authorized,
        f.current.scope_allowed,
        f.current.continuous,
        f.current.live,
        f.current.not_stopped,
        f.current.retained,
        f.current.source_readable,
        f.projection_allowed,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn loss(f: &TrustedSourcePublicationFactsV1<'_>) -> bool {
    [
        f.current.authorized,
        f.current.scope_allowed,
        f.current.continuous,
        f.current.live,
        f.current.not_stopped,
        f.current.retained,
        f.current.source_readable,
        f.projection_allowed,
    ]
    .contains(&Some(false))
}
fn ready(f: &TrustedSourcePublicationFactsV1<'_>) -> bool {
    [
        f.current.resources_ready,
        f.current.reply_ready,
        f.current.transfer_ready,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn frame_bytes(p: &SourcePublicationProjectionV1) -> Result<usize, Error> {
    size_of::<SourcePublicationDeliveryV1>()
        .checked_add(size_of::<SourcePublicationProjectionV1>())
        .and_then(|n| n.checked_add(p.text.len()))
        .ok_or(Error::Limit)
}
impl SourceCoordinator {
    fn publication_pool(&self) -> Result<&Pool, Error> {
        let State::Active(p) = &self.publication_streams else {
            return Err(Error::Unavailable);
        };
        self.retention
            .validate_exact(&p.reservation, p.charge)
            .map_err(parent)?;
        if p.rows.len() > p.issued
            || p.issued > p.limits.max_issued
            || p.rows.iter().flatten().count() > p.limits.max_streams
        {
            return Err(Error::Faulted);
        }
        Ok(p)
    }
    fn publication_record(&self, id: &SourcePublicationStreamIdV1) -> Result<&Record, Error> {
        let row = self.publication_record_control(id)?;
        let p = self.publication_pool()?;
        if !row.valid(id, p.limits, p.issued) {
            return Err(Error::Faulted);
        }
        Ok(row)
    }
    fn publication_record_control(
        &self,
        id: &SourcePublicationStreamIdV1,
    ) -> Result<&Record, Error> {
        let p = self.publication_pool()?;
        if !Arc::ptr_eq(&p.owner, &id.owner) {
            return Err(Error::Unavailable);
        }
        p.rows
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)
    }
    fn publication_record_mut(&mut self, id: &SourcePublicationStreamIdV1) -> &mut Record {
        let State::Active(p) = &mut self.publication_streams else {
            unreachable!("checked publication owner")
        };
        p.rows[id.slot]
            .as_mut()
            .expect("checked publication context")
    }
    fn publication_gap(&mut self, id: &SourcePublicationStreamIdV1, r: &mut Recorder) {
        self.publication_record_mut(id).close(Phase::Gap);
        // A transition marker, not an invented duration or missing-event count.
        r.record(Observation {
            boundary: Boundary::SourcePublicationGap,
            disposition: Disposition::Completed,
            service_time: None,
        });
    }
    fn publication_gate(
        &mut self,
        id: &SourcePublicationStreamIdV1,
        claimed: &SourceReadScopeV1,
        f: &TrustedSourcePublicationFactsV1<'_>,
    ) -> Result<(), Error> {
        self.ensure_operational().map_err(|_| Error::Faulted)?;
        self.source_command_ordinary_gate_v1()?;
        if self.source_facade_busy {
            return Err(Error::Unavailable);
        }
        let row = self.publication_record_control(id)?;
        if row.scope != *claimed || row.scope != *f.original_scope {
            return Err(Error::Unavailable);
        }
        let row = self.publication_record(id)?;
        if row.phase == Phase::Stopped {
            return Err(Error::Unavailable);
        }
        if row.scope != *f.current.scope || loss(f) || self.document(&row.scope.document).is_err() {
            self.publication_record_mut(id).close(Phase::Stopped);
            return Err(Error::Unavailable);
        }
        if !ordinary(f) {
            return Err(Error::Unavailable);
        }
        Ok(())
    }
    fn publication_projection(
        &self,
        d: &DocumentId,
        limits: SourcePublicationLimitsV1,
    ) -> Result<Arc<SourcePublicationProjectionV1>, Error> {
        let drow = self.document(d).map_err(|_| Error::Unavailable)?;
        let text = drow.source.text();
        if size_of::<SourcePublicationProjectionV1>()
            .checked_add(text.len())
            .is_none_or(|n| n > limits.max_projection_bytes)
        {
            return Err(Error::Limit);
        }
        let projection = SourcePublicationProjectionV1 {
            version: DocumentVersion {
                document: d.clone(),
                revision: drow.revision,
            },
            text: owned(text)?,
        };
        if frame_bytes(&projection)? > limits.max_reply_bytes {
            return Err(Error::Limit);
        }
        Ok(Arc::new(projection))
    }
    fn finish_publication<T>(
        r: &mut Recorder,
        boundary: Boundary,
        result: Result<T, Error>,
        time: Option<std::time::Duration>,
    ) -> Result<T, Error> {
        r.record(Observation {
            boundary,
            disposition: match &result {
                Ok(_) => Disposition::Completed,
                Err(Error::Faulted) => Disposition::Unknown,
                Err(_) => Disposition::Refused,
            },
            service_time: time,
        });
        result
    }
    pub fn configure_source_publication_streams_v1(
        &mut self,
        authority: &str,
        engine: &str,
        limits: SourcePublicationLimitsV1,
        control: &SourceReadPoolControlV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(admission_error)?;
            if control.owner_control_allowed != Some(true)
                || control.resources_ready != Some(true)
                || !matches!(self.publication_streams, State::Unconfigured)
            {
                return Err(Error::Unavailable);
            }
            if authority.is_empty() || engine.is_empty() || limits.max_queue == 0 {
                return Err(Error::Unavailable);
            }
            let charge = limits
                .required_pool_bytes()?
                .checked_add(authority.len())
                .and_then(|n| n.checked_add(engine.len()))
                .filter(|n| *n <= limits.max_pool_bytes)
                .ok_or(Error::Limit)?;
            self.retention.check(charge).map_err(parent)?;
            let authority = owned(authority)?;
            let engine = owned(engine)?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(limits.max_issued)
                .map_err(|_| Error::Capacity)?;
            let reservation = self.retention.reserve(charge).map_err(parent)?;
            self.publication_streams = State::Active(Box::new(Pool {
                owner: Arc::new(()),
                authority,
                engine,
                limits,
                rows,
                issued: 0,
                reservation,
                charge,
                commands: commands::State::default(),
            }));
            Ok(())
        })();
        Self::finish_publication(
            r,
            Boundary::SourcePublicationConfiguration,
            result,
            timer.finish(),
        )
    }
    /// Raw technical establishment. F19 original-command/replay and recipient
    /// facade assembly remain separate; matching retries must not call this again.
    pub fn open_source_publication_stream_v1(
        &mut self,
        s: &SourceReadScopeV1,
        f: &TrustedSourcePublicationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourcePublicationBaselineV1, Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(admission_error)?;
            self.source_command_ordinary_gate_v1()?;
            if self.source_facade_busy
                || f.original_scope != s
                || f.current.scope != s
                || !ordinary(f)
                || !ready(f)
                || f.baseline_allowed != Some(true)
            {
                return Err(Error::Unavailable);
            }
            let p = self.publication_pool()?;
            let limits = p.limits;
            if p.issued >= limits.max_issued
                || p.rows.iter().flatten().count() >= limits.max_streams
            {
                return Err(Error::Limit);
            }
            if s.context.authority != p.authority
                || s.context.engine != p.engine
                || s.policy.is_empty()
                || settings::basis::labels(&s.context)
                    .into_iter()
                    .any(str::is_empty)
            {
                return Err(Error::Unavailable);
            }
            if source_context::scope_bytes(s).map_err(source_error)? > limits.max_context_bytes {
                return Err(Error::Limit);
            }
            let projection = self.publication_projection(&s.document, limits)?;
            if frame_bytes(&projection)? > f.max_reply_bytes {
                return Err(Error::Limit);
            }
            let scope = SourceReadScopeV1 {
                document: s.document.clone(),
                policy: owned(&s.policy)?,
                context: settings::basis::copy_context(&s.context).map_err(|_| Error::Capacity)?,
            };
            let mut queue = VecDeque::new();
            queue
                .try_reserve_exact(limits.max_queue)
                .map_err(|_| Error::Capacity)?;
            let State::Active(p) = &mut self.publication_streams else {
                unreachable!()
            };
            let id = SourcePublicationStreamIdV1 {
                owner: p.owner.clone(),
                slot: p.rows.len(),
            };
            let generation = SourcePublicationGenerationV1 {
                stream: id.clone(),
                serial: 0,
            };
            p.rows.push(Some(Record {
                scope,
                generation: generation.clone(),
                baseline: Some(projection.clone()),
                queue,
                phase: Phase::Active,
            }));
            p.issued += 1;
            Ok(SourcePublicationBaselineV1 {
                stream: id,
                generation,
                projection,
            })
        })();
        Self::finish_publication(r, Boundary::SourcePublicationOpen, result, timer.finish())
    }
    /// Samples actual committed state. No publisher hook or subscriber callback;
    /// skipped intermediate roots are not represented as a lossless audit.
    pub fn sample_source_publication_stream_v1(
        &mut self,
        id: &SourcePublicationStreamIdV1,
        claimed: &SourceReadScopeV1,
        f: &TrustedSourcePublicationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourcePublicationSampleV1, Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.publication_gate(id, claimed, f)?;
            if self.publication_record(id)?.phase == Phase::Gap {
                return Ok(SourcePublicationSampleV1::Gap);
            }
            if f.current.resources_ready != Some(true) {
                self.publication_gap(id, r);
                return Ok(SourcePublicationSampleV1::Gap);
            }
            // An unchanged native source version implies the same canonical
            // source. Hidden companion publications need no source copy or token.
            if self
                .publication_record(id)?
                .baseline
                .as_ref()
                .is_some_and(|b| {
                    self.document(&claimed.document)
                        .is_ok_and(|d| b.version.revision == d.revision)
                })
            {
                return Ok(SourcePublicationSampleV1::Unchanged);
            }
            let limits = self.publication_pool()?.limits;
            // A changed native source revision is itself permitted visible
            // state. Check pressure before allocating a doomed replacement.
            if self.publication_record(id)?.queue.len() >= limits.max_queue
                || self.publication_pool()?.issued >= limits.max_issued
            {
                self.publication_gap(id, r);
                return Ok(SourcePublicationSampleV1::Gap);
            }
            let projection = match self.publication_projection(&claimed.document, limits) {
                Ok(p) => p,
                Err(Error::Limit | Error::Capacity) => {
                    self.publication_gap(id, r);
                    return Ok(SourcePublicationSampleV1::Gap);
                }
                Err(e) => return Err(e),
            };
            let row = self.publication_record(id)?;
            if row.baseline.as_ref().is_some_and(|b| **b == *projection) {
                return Ok(SourcePublicationSampleV1::Unchanged);
            }
            let before = row.generation.clone();
            let after = SourcePublicationGenerationV1 {
                stream: id.clone(),
                serial: before.serial.checked_add(1).ok_or(Error::Limit)?,
            };
            let State::Active(p) = &mut self.publication_streams else {
                unreachable!()
            };
            p.issued += 1;
            let row = p.rows[id.slot].as_mut().expect("checked stream");
            row.queue.push_back(SourcePublicationEventV1 {
                before,
                after: after.clone(),
                projection: projection.clone(),
            });
            row.baseline = Some(projection);
            row.generation = after;
            Ok(SourcePublicationSampleV1::Queued)
        })();
        Self::finish_publication(r, Boundary::SourcePublicationSample, result, timer.finish())
    }
    pub fn release_source_publication_event_v1(
        &mut self,
        id: &SourcePublicationStreamIdV1,
        claimed: &SourceReadScopeV1,
        f: &TrustedSourcePublicationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourcePublicationDeliveryV1, Error> {
        self.release_source_publication_event_frame_v1(id, claimed, f, 0, r)
    }
    pub(in crate::coordinator) fn release_source_publication_event_frame_v1(
        &mut self,
        id: &SourcePublicationStreamIdV1,
        claimed: &SourceReadScopeV1,
        f: &TrustedSourcePublicationFactsV1<'_>,
        extra_header: usize,
        r: &mut Recorder,
    ) -> Result<SourcePublicationDeliveryV1, Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.publication_gate(id, claimed, f)?;
            if !ready(f) {
                return Err(Error::Unavailable);
            }
            let row = self.publication_record(id)?;
            let reply_limit = f
                .max_reply_bytes
                .min(self.publication_pool()?.limits.max_reply_bytes);
            if extra_header != 0
                && size_of::<SourcePublicationDeliveryV1>()
                    .checked_add(extra_header)
                    .is_none_or(|n| n > reply_limit)
            {
                return Err(Error::Limit);
            }
            if row.phase == Phase::Gap {
                return Ok(SourcePublicationDeliveryV1::Gap);
            }
            if let Some(head) = row.queue.front()
                && frame_bytes(&head.projection)?
                    .checked_add(extra_header)
                    .is_none_or(|n| n > reply_limit)
            {
                return Err(Error::Limit);
            }
            Ok(match self.publication_record_mut(id).queue.pop_front() {
                Some(event) => SourcePublicationDeliveryV1::Projection(event),
                None => SourcePublicationDeliveryV1::Empty,
            })
        })();
        Self::finish_publication(
            r,
            Boundary::SourcePublicationRelease,
            result,
            timer.finish(),
        )
    }
    pub fn resync_source_publication_stream_v1(
        &mut self,
        id: &SourcePublicationStreamIdV1,
        claimed: &SourceReadScopeV1,
        f: &TrustedSourcePublicationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourcePublicationBaselineV1, Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.publication_gate(id, claimed, f)?;
            if !ready(f) || f.baseline_allowed != Some(true) {
                return Err(Error::Unavailable);
            }
            let p = self.publication_pool()?;
            if p.issued >= p.limits.max_issued {
                return Err(Error::Limit);
            }
            let projection = self.publication_projection(&claimed.document, p.limits)?;
            if frame_bytes(&projection)? > f.max_reply_bytes {
                return Err(Error::Limit);
            }
            let generation = SourcePublicationGenerationV1 {
                stream: id.clone(),
                serial: self
                    .publication_record(id)?
                    .generation
                    .serial
                    .checked_add(1)
                    .ok_or(Error::Limit)?,
            };
            let State::Active(p) = &mut self.publication_streams else {
                unreachable!()
            };
            p.issued += 1;
            let row = p.rows[id.slot].as_mut().expect("checked stream");
            row.queue.clear();
            row.baseline = Some(projection.clone());
            row.generation = generation.clone();
            row.phase = Phase::Active;
            Ok(SourcePublicationBaselineV1 {
                stream: id.clone(),
                generation,
                projection,
            })
        })();
        Self::finish_publication(r, Boundary::SourcePublicationResync, result, timer.finish())
    }
    pub fn stop_source_publication_stream_v1(
        &mut self,
        id: &SourcePublicationStreamIdV1,
        control: &SourceReadContextControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let row = self.publication_record_control(id)?;
            if control.owner_control_allowed != Some(true) || *control.scope != row.scope {
                return Err(Error::Unavailable);
            }
            self.publication_record_mut(id).close(Phase::Stopped);
            Ok(())
        })();
        Self::finish_publication(r, Boundary::SourcePublicationStop, result, timer.finish())
    }
    pub fn retire_source_publication_stream_v1(
        &mut self,
        id: &SourcePublicationStreamIdV1,
        control: &SourceReadContextControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let row = self.publication_record_control(id)?;
            if control.owner_control_allowed != Some(true)
                || control.retention_released != Some(true)
                || control.delivery_drained != Some(true)
                || *control.scope != row.scope
                || row.phase != Phase::Stopped
            {
                return Err(Error::Unavailable);
            }
            let State::Active(p) = &mut self.publication_streams else {
                unreachable!()
            };
            p.rows[id.slot] = None;
            Ok(())
        })();
        Self::finish_publication(
            r,
            Boundary::SourcePublicationRetirement,
            result,
            timer.finish(),
        )
    }
    pub fn retire_source_publication_pool_v1(
        &mut self,
        control: &SourceReadPoolControlV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let p = self.publication_pool()?;
            if !p.commands.closed() {
                return Err(Error::Unavailable);
            }
            if [
                control.owner_control_allowed,
                control.retention_released,
                control.delivery_drained,
            ]
            .into_iter()
            .any(|v| v != Some(true))
                || p.rows.iter().any(Option::is_some)
            {
                return Err(Error::Unavailable);
            }
            let State::Active(p) = &mut self.publication_streams else {
                unreachable!()
            };
            self.retention.release(&mut p.reservation).map_err(parent)?;
            self.publication_streams = State::Closed { issued: p.issued };
            Ok(())
        })();
        Self::finish_publication(
            r,
            Boundary::SourcePublicationPoolRetirement,
            result,
            timer.finish(),
        )
    }
    /// Trusted fixed gauges, never scoped recipient progress or quiescence proof.
    pub fn source_publication_usage_v1(&self) -> Result<SourcePublicationUsageV1, Error> {
        match &self.publication_streams {
            State::Unconfigured => Ok(SourcePublicationUsageV1::default()),
            State::Closed { issued } => Ok(SourcePublicationUsageV1 {
                issued_lifetime: *issued,
                closed: true,
                ..Default::default()
            }),
            State::Active(_) => {
                let p = self.publication_pool()?;
                let mut u = SourcePublicationUsageV1 {
                    issued_lifetime: p.issued,
                    reserved_pool_bytes: p.charge,
                    ..Default::default()
                };
                for row in p.rows.iter().flatten() {
                    u.streams += 1;
                    u.queued = u
                        .queued
                        .checked_add(row.queue.len())
                        .ok_or(Error::Faulted)?;
                    match row.phase {
                        Phase::Active => {}
                        Phase::Gap => u.gapped += 1,
                        Phase::Stopped => u.stopped += 1,
                    }
                }
                Ok(u)
            }
        }
    }
}
