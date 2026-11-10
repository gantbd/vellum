//! Native bounded annotation-collection publication streams.
//! Post-publication service, not a subscriber, lossless audit or F19 command facade.
use super::*;
mod shutdown_inventory;
use std::collections::VecDeque;
use std::mem::size_of;
#[cfg(test)]
pub(in crate::coordinator) mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionPublicationErrorV1 {
    Unavailable,
    Limit,
    Capacity,
    Faulted,
}
type Error = CollectionPublicationErrorV1;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollectionPublicationLimitsV1 {
    pub max_streams: usize,
    pub max_bindings: usize,
    pub max_binding_bytes: usize,
    /// Shared lifetime issuance for stream identities, references and visible generations.
    pub max_issued: usize,
    pub max_context_bytes: usize,
    pub max_projection_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_queue: usize,
    pub max_pool_bytes: usize,
}
impl CollectionPublicationLimitsV1 {
    pub fn required_pool_bytes(self) -> Result<usize, Error> {
        let row = self
            .max_context_bytes
            .checked_add(self.max_binding_bytes.checked_mul(2).ok_or(Error::Limit)?)
            .ok_or(Error::Limit)?
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
pub struct CollectionPublicationStreamIdV1 {
    owner: Arc<()>,
    slot: usize,
}
impl PartialEq for CollectionPublicationStreamIdV1 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for CollectionPublicationStreamIdV1 {}
#[derive(Clone, PartialEq, Eq)]
pub struct CollectionPublicationGenerationV1 {
    stream: CollectionPublicationStreamIdV1,
    serial: usize,
}
#[derive(Clone)]
pub struct CollectionPublicationEventV1 {
    pub before: CollectionPublicationGenerationV1,
    pub after: CollectionPublicationGenerationV1,
    pub projection: Arc<CollectionPublicationProjectionV1>,
}
pub struct CollectionPublicationBaselineV1 {
    pub stream: CollectionPublicationStreamIdV1,
    pub generation: CollectionPublicationGenerationV1,
    pub projection: Arc<CollectionPublicationProjectionV1>,
}
pub enum CollectionPublicationDeliveryV1 {
    Projection(CollectionPublicationEventV1),
    Empty,
    Gap,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionPublicationSampleV1 {
    Unchanged,
    Queued,
    Gap,
}
macro_rules! redacted { ($($t:ty),+) => { $(impl std::fmt::Debug for $t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(concat!(stringify!($t), "(<redacted>)")) }
})+ }; }
redacted!(
    CollectionPublicationStreamIdV1,
    CollectionPublicationGenerationV1,
    CollectionPublicationProjectionV1,
    CollectionPublicationEventV1,
    CollectionPublicationBaselineV1,
    CollectionPublicationDeliveryV1
);
pub(in crate::coordinator) mod commands;
mod projection;
pub use commands::*;
use projection::{Binding, copy_scope, scope_bytes};
pub use projection::{
    CollectionAnnotationReferenceV1, CollectionAnnotationRowV1, CollectionPublicationControlV1,
    CollectionPublicationProjectionV1, CollectionPublicationScopeV1,
};
/// Independently attributed host facts. The original scope binds these facts to
/// the retained observer; a lowered caller claim is not accepted loss evidence.
pub struct TrustedCollectionPublicationFactsV1<'a> {
    pub original_scope: &'a CollectionPublicationScopeV1,
    pub current: &'a TrustedSourceReadFactsV1<'a>,
    /// Independent current authorization for the whole projection being used.
    /// At queued release this means the exact historical head, not merely the
    /// membership of the latest native root. Host attribution remains a premise.
    pub projection_allowed: Option<bool>,
    pub baseline_allowed: Option<bool>,
    pub max_reply_bytes: usize,
    pub annotation_readable: Option<bool>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CollectionPublicationUsageV1 {
    pub streams: usize,
    pub queued: usize,
    pub bindings: usize,
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
    scope: CollectionPublicationScopeV1,
    generation: CollectionPublicationGenerationV1,
    baseline: Option<Arc<CollectionPublicationProjectionV1>>,
    queue: VecDeque<CollectionPublicationEventV1>,
    phase: Phase,
    bindings: Vec<Binding>,
}
impl Record {
    fn valid(
        &self,
        id: &CollectionPublicationStreamIdV1,
        limits: CollectionPublicationLimitsV1,
        issued: usize,
    ) -> bool {
        if self.generation.stream != *id
            || self.generation.serial >= issued
            || self.queue.len() > limits.max_queue
        {
            return false;
        }
        if !projection::valid_bindings(&self.bindings, id, &self.scope, limits) {
            return false;
        }
        let projection_valid = |p: &CollectionPublicationProjectionV1| {
            p.valid(&self.scope, &self.bindings)
                && p.bytes().is_ok_and(|n| n <= limits.max_projection_bytes)
                && frame_bytes(p).is_ok_and(|n| n <= limits.max_reply_bytes)
        };
        match self.phase {
            Phase::Gap => return self.baseline.is_none() && self.queue.is_empty(),
            Phase::Stopped => {
                return self.baseline.is_none()
                    && self.queue.is_empty()
                    && self.bindings.is_empty();
            }
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
        if phase == Phase::Stopped {
            self.bindings.clear();
        }
    }
}
pub(super) struct Pool {
    commands: commands::State,
    owner: Arc<()>,
    authority: String,
    engine: String,
    limits: CollectionPublicationLimitsV1,
    rows: Vec<Option<Record>>,
    issued: usize,
    reservation: retention::Reservation,
    charge: usize,
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
                    let id = CollectionPublicationStreamIdV1 {
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
                    .filter(|r| r.scope.source.document == *document)
                    .count()
                    .checked_add(
                        p.commands
                            .document_pins(document, ledger, &p.owner)
                            .map_err(|_| CoordinatorError::Faulted)?,
                    )
                    .ok_or(CoordinatorError::Faulted)
            }
        }
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
fn ordinary(f: &TrustedCollectionPublicationFactsV1<'_>) -> bool {
    [
        f.current.authorized,
        f.current.scope_allowed,
        f.current.continuous,
        f.current.live,
        f.current.not_stopped,
        f.current.retained,
        f.annotation_readable,
        f.projection_allowed,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn loss(f: &TrustedCollectionPublicationFactsV1<'_>) -> bool {
    [
        f.current.authorized,
        f.current.scope_allowed,
        f.current.continuous,
        f.current.live,
        f.current.not_stopped,
        f.current.retained,
        f.annotation_readable,
        f.projection_allowed,
    ]
    .contains(&Some(false))
}
fn ready(f: &TrustedCollectionPublicationFactsV1<'_>) -> bool {
    [
        f.current.resources_ready,
        f.current.reply_ready,
        f.current.transfer_ready,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn frame_bytes(p: &CollectionPublicationProjectionV1) -> Result<usize, Error> {
    size_of::<CollectionPublicationDeliveryV1>()
        .checked_add(p.bytes()?)
        .ok_or(Error::Limit)
}
impl SourceCoordinator {
    fn collection_publication_admission_v1(&self) -> Result<(), Error> {
        self.ensure_admission_unfenced().map_err(|e| match e {
            CoordinatorError::Faulted => Error::Faulted,
            _ => Error::Unavailable,
        })
    }
    fn collection_publication_pool(&self) -> Result<&Pool, Error> {
        let State::Active(p) = &self.collection_publication_streams else {
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
    fn collection_publication_record(
        &self,
        id: &CollectionPublicationStreamIdV1,
    ) -> Result<&Record, Error> {
        let row = self.collection_publication_record_control(id)?;
        let p = self.collection_publication_pool()?;
        if !row.valid(id, p.limits, p.issued) {
            return Err(Error::Faulted);
        }
        Ok(row)
    }
    fn collection_publication_record_control(
        &self,
        id: &CollectionPublicationStreamIdV1,
    ) -> Result<&Record, Error> {
        let p = self.collection_publication_pool()?;
        if !Arc::ptr_eq(&p.owner, &id.owner) {
            return Err(Error::Unavailable);
        }
        p.rows
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)
    }
    fn collection_publication_record_mut(
        &mut self,
        id: &CollectionPublicationStreamIdV1,
    ) -> &mut Record {
        let State::Active(p) = &mut self.collection_publication_streams else {
            unreachable!("checked publication owner")
        };
        p.rows[id.slot]
            .as_mut()
            .expect("checked publication context")
    }
    fn collection_publication_gap(
        &mut self,
        id: &CollectionPublicationStreamIdV1,
        r: &mut Recorder,
    ) {
        self.collection_publication_record_mut(id).close(Phase::Gap);
        // A transition marker, not an invented duration or missing-event count.
        r.record(Observation {
            boundary: Boundary::CollectionPublicationGap,
            disposition: Disposition::Completed,
            service_time: None,
        });
    }
    fn collection_publication_gate(
        &mut self,
        id: &CollectionPublicationStreamIdV1,
        claimed: &CollectionPublicationScopeV1,
        f: &TrustedCollectionPublicationFactsV1<'_>,
    ) -> Result<(), Error> {
        self.ensure_operational().map_err(|_| Error::Faulted)?;
        self.collection_command_ordinary_gate_v1()?;
        if self.source_facade_busy {
            return Err(Error::Unavailable);
        }
        let row = self.collection_publication_record_control(id)?;
        if row.scope != *claimed || row.scope != *f.original_scope {
            return Err(Error::Unavailable);
        }
        let row = self.collection_publication_record(id)?;
        if row.phase == Phase::Stopped {
            return Err(Error::Unavailable);
        }
        if row.scope.source != *f.current.scope
            || loss(f)
            || self.document(&row.scope.source.document).is_err()
        {
            self.collection_publication_record_mut(id)
                .close(Phase::Stopped);
            return Err(Error::Unavailable);
        }
        if !ordinary(f) {
            return Err(Error::Unavailable);
        }
        Ok(())
    }
    fn finish_collection_publication<T>(
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
    pub fn configure_collection_publication_streams_v1(
        &mut self,
        authority: &str,
        engine: &str,
        limits: CollectionPublicationLimitsV1,
        control: &SourceReadPoolControlV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(|_| Error::Faulted)?;
            self.collection_publication_admission_v1()?;
            if control.owner_control_allowed != Some(true)
                || control.resources_ready != Some(true)
                || !matches!(self.collection_publication_streams, State::Unconfigured)
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
            self.collection_publication_streams = State::Active(Box::new(Pool {
                commands: commands::State::default(),
                owner: Arc::new(()),
                authority,
                engine,
                limits,
                rows,
                issued: 0,
                reservation,
                charge,
            }));
            Ok(())
        })();
        Self::finish_collection_publication(
            r,
            Boundary::CollectionPublicationConfiguration,
            result,
            timer.finish(),
        )
    }
    /// Raw technical establishment. F19 original-command/replay and recipient
    /// facade assembly remain separate; matching retries must not call this again.
    pub fn open_collection_publication_stream_v1(
        &mut self,
        s: &CollectionPublicationScopeV1,
        preparation: &CollectionVisibilityPreparationIdV1,
        f: &TrustedCollectionPublicationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<CollectionPublicationBaselineV1, Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(|_| Error::Faulted)?;
            self.collection_publication_admission_v1()?;
            self.collection_command_ordinary_gate_v1()?;
            if self.source_facade_busy
                || f.original_scope != s
                || f.current.scope != &s.source
                || !ordinary(f)
                || !ready(f)
                || f.baseline_allowed != Some(true)
            {
                return Err(Error::Unavailable);
            }
            let p = self.collection_publication_pool()?;
            let limits = p.limits;
            if p.issued >= limits.max_issued
                || p.rows.iter().flatten().count() >= limits.max_streams
            {
                return Err(Error::Limit);
            }
            if s.source.context.authority != p.authority
                || s.source.context.engine != p.engine
                || s.source.policy.is_empty()
                || settings::basis::labels(&s.source.context)
                    .into_iter()
                    .any(str::is_empty)
            {
                return Err(Error::Unavailable);
            }
            if scope_bytes(s)? > limits.max_context_bytes {
                return Err(Error::Limit);
            }
            let id = CollectionPublicationStreamIdV1 {
                owner: p.owner.clone(),
                slot: p.rows.len(),
            };
            let issued = p.issued;
            let capture = self
                .consume_collection_visibility_v1(
                    preparation,
                    &s.source,
                    s.ceiling,
                    &TrustedCollectionVisibilityFactsV1 {
                        current: f.current,
                        annotation_readable: f.annotation_readable,
                        projection_current: f.projection_allowed,
                    },
                    r,
                )
                .map_err(source_error)?;
            let stage = projection::stage(&id, s, &capture, &[], limits)?;
            if issued
                .checked_add(1)
                .and_then(|n| n.checked_add(stage.new_bindings))
                .is_none_or(|n| n > limits.max_issued)
                || frame_bytes(&stage.projection)? > f.max_reply_bytes
            {
                return Err(Error::Limit);
            }
            let projection = stage.projection;
            let scope = copy_scope(s)?;
            let mut queue = VecDeque::new();
            queue
                .try_reserve_exact(limits.max_queue)
                .map_err(|_| Error::Capacity)?;
            let State::Active(p) = &mut self.collection_publication_streams else {
                unreachable!()
            };
            let id = CollectionPublicationStreamIdV1 {
                owner: p.owner.clone(),
                slot: p.rows.len(),
            };
            let generation = CollectionPublicationGenerationV1 {
                stream: id.clone(),
                serial: 0,
            };
            p.rows.push(Some(Record {
                scope,
                generation: generation.clone(),
                baseline: Some(projection.clone()),
                queue,
                phase: Phase::Active,
                bindings: stage.bindings,
            }));
            p.issued += 1 + stage.new_bindings;
            Ok(CollectionPublicationBaselineV1 {
                stream: id,
                generation,
                projection,
            })
        })();
        Self::finish_collection_publication(
            r,
            Boundary::CollectionPublicationOpen,
            result,
            timer.finish(),
        )
    }
    /// Samples actual committed state. No publisher hook or subscriber callback;
    /// skipped intermediate roots are not represented as a lossless audit.
    pub fn sample_collection_publication_stream_v1(
        &mut self,
        id: &CollectionPublicationStreamIdV1,
        claimed: &CollectionPublicationScopeV1,
        preparation: &CollectionVisibilityPreparationIdV1,
        f: &TrustedCollectionPublicationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<CollectionPublicationSampleV1, Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.collection_publication_gate(id, claimed, f)?;
            if self.collection_publication_record(id)?.phase == Phase::Gap {
                return Ok(CollectionPublicationSampleV1::Gap);
            }
            if f.current.resources_ready != Some(true) {
                self.collection_publication_gap(id, r);
                return Ok(CollectionPublicationSampleV1::Gap);
            }
            let limits = self.collection_publication_pool()?.limits;
            let capture = self
                .consume_collection_visibility_v1(
                    preparation,
                    &claimed.source,
                    claimed.ceiling,
                    &TrustedCollectionVisibilityFactsV1 {
                        current: f.current,
                        annotation_readable: f.annotation_readable,
                        projection_current: f.projection_allowed,
                    },
                    r,
                )
                .map_err(source_error)?;
            let row = self.collection_publication_record(id)?;
            if row
                .baseline
                .as_ref()
                .is_some_and(|b| projection::matches(&capture, b, &row.bindings))
            {
                return Ok(CollectionPublicationSampleV1::Unchanged);
            }
            if row.queue.len() >= limits.max_queue
                || self.collection_publication_pool()?.issued >= limits.max_issued
            {
                self.collection_publication_gap(id, r);
                return Ok(CollectionPublicationSampleV1::Gap);
            }
            let stage = match projection::stage(id, claimed, &capture, &row.bindings, limits) {
                Ok(stage) => stage,
                Err(Error::Limit | Error::Capacity) => {
                    self.collection_publication_gap(id, r);
                    return Ok(CollectionPublicationSampleV1::Gap);
                }
                Err(e) => return Err(e),
            };
            if self
                .collection_publication_pool()?
                .issued
                .checked_add(1)
                .and_then(|n| n.checked_add(stage.new_bindings))
                .is_none_or(|n| n > limits.max_issued)
            {
                self.collection_publication_gap(id, r);
                return Ok(CollectionPublicationSampleV1::Gap);
            }
            let projection = stage.projection;
            let row = self.collection_publication_record(id)?;
            let before = row.generation.clone();
            let after = CollectionPublicationGenerationV1 {
                stream: id.clone(),
                serial: before.serial.checked_add(1).ok_or(Error::Limit)?,
            };
            let State::Active(p) = &mut self.collection_publication_streams else {
                unreachable!()
            };
            p.issued += 1 + stage.new_bindings;
            let row = p.rows[id.slot].as_mut().expect("checked stream");
            row.bindings = stage.bindings;
            row.queue.push_back(CollectionPublicationEventV1 {
                before,
                after: after.clone(),
                projection: projection.clone(),
            });
            row.baseline = Some(projection);
            row.generation = after;
            Ok(CollectionPublicationSampleV1::Queued)
        })();
        Self::finish_collection_publication(
            r,
            Boundary::CollectionPublicationSample,
            result,
            timer.finish(),
        )
    }
    pub fn release_collection_publication_event_v1(
        &mut self,
        id: &CollectionPublicationStreamIdV1,
        claimed: &CollectionPublicationScopeV1,
        f: &TrustedCollectionPublicationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<CollectionPublicationDeliveryV1, Error> {
        self.release_collection_publication_event_frame_v1(id, claimed, f, 0, r)
    }
    pub(in crate::coordinator) fn release_collection_publication_event_frame_v1(
        &mut self,
        id: &CollectionPublicationStreamIdV1,
        claimed: &CollectionPublicationScopeV1,
        f: &TrustedCollectionPublicationFactsV1<'_>,
        extra_header: usize,
        r: &mut Recorder,
    ) -> Result<CollectionPublicationDeliveryV1, Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.collection_publication_gate(id, claimed, f)?;
            if !ready(f) {
                return Err(Error::Unavailable);
            }
            let row = self.collection_publication_record(id)?;
            let reply_limit = f
                .max_reply_bytes
                .min(self.collection_publication_pool()?.limits.max_reply_bytes);
            if extra_header != 0
                && size_of::<CollectionPublicationDeliveryV1>()
                    .checked_add(extra_header)
                    .is_none_or(|n| n > reply_limit)
            {
                return Err(Error::Limit);
            }
            if row.phase == Phase::Gap {
                return Ok(CollectionPublicationDeliveryV1::Gap);
            }
            if let Some(head) = row.queue.front()
                && frame_bytes(&head.projection)?
                    .checked_add(extra_header)
                    .is_none_or(|n| n > reply_limit)
            {
                return Err(Error::Limit);
            }
            Ok(
                match self.collection_publication_record_mut(id).queue.pop_front() {
                    Some(event) => CollectionPublicationDeliveryV1::Projection(event),
                    None => CollectionPublicationDeliveryV1::Empty,
                },
            )
        })();
        Self::finish_collection_publication(
            r,
            Boundary::CollectionPublicationRelease,
            result,
            timer.finish(),
        )
    }
    pub fn resync_collection_publication_stream_v1(
        &mut self,
        id: &CollectionPublicationStreamIdV1,
        claimed: &CollectionPublicationScopeV1,
        preparation: &CollectionVisibilityPreparationIdV1,
        f: &TrustedCollectionPublicationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<CollectionPublicationBaselineV1, Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.collection_publication_gate(id, claimed, f)?;
            if !ready(f) || f.baseline_allowed != Some(true) {
                return Err(Error::Unavailable);
            }
            let p = self.collection_publication_pool()?;
            if p.issued >= p.limits.max_issued {
                return Err(Error::Limit);
            }
            let limits = p.limits;
            let issued = p.issued;
            let capture = self
                .consume_collection_visibility_v1(
                    preparation,
                    &claimed.source,
                    claimed.ceiling,
                    &TrustedCollectionVisibilityFactsV1 {
                        current: f.current,
                        annotation_readable: f.annotation_readable,
                        projection_current: f.projection_allowed,
                    },
                    r,
                )
                .map_err(source_error)?;
            let row = self.collection_publication_record(id)?;
            let stage = projection::stage(id, claimed, &capture, &row.bindings, limits)?;
            if issued
                .checked_add(1)
                .and_then(|n| n.checked_add(stage.new_bindings))
                .is_none_or(|n| n > limits.max_issued)
                || frame_bytes(&stage.projection)? > f.max_reply_bytes
            {
                return Err(Error::Limit);
            }
            let projection = stage.projection;
            let generation = CollectionPublicationGenerationV1 {
                stream: id.clone(),
                serial: self
                    .collection_publication_record(id)?
                    .generation
                    .serial
                    .checked_add(1)
                    .ok_or(Error::Limit)?,
            };
            let State::Active(p) = &mut self.collection_publication_streams else {
                unreachable!()
            };
            p.issued += 1 + stage.new_bindings;
            let row = p.rows[id.slot].as_mut().expect("checked stream");
            row.bindings = stage.bindings;
            row.queue.clear();
            row.baseline = Some(projection.clone());
            row.generation = generation.clone();
            row.phase = Phase::Active;
            Ok(CollectionPublicationBaselineV1 {
                stream: id.clone(),
                generation,
                projection,
            })
        })();
        Self::finish_collection_publication(
            r,
            Boundary::CollectionPublicationResync,
            result,
            timer.finish(),
        )
    }
    pub fn stop_collection_publication_stream_v1(
        &mut self,
        id: &CollectionPublicationStreamIdV1,
        control: &CollectionPublicationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let row = self.collection_publication_record_control(id)?;
            if control.owner_control_allowed != Some(true) || *control.scope != row.scope {
                return Err(Error::Unavailable);
            }
            self.collection_publication_record_mut(id)
                .close(Phase::Stopped);
            Ok(())
        })();
        Self::finish_collection_publication(
            r,
            Boundary::CollectionPublicationStop,
            result,
            timer.finish(),
        )
    }
    pub fn retire_collection_publication_stream_v1(
        &mut self,
        id: &CollectionPublicationStreamIdV1,
        control: &CollectionPublicationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let row = self.collection_publication_record_control(id)?;
            if control.owner_control_allowed != Some(true)
                || control.retention_released != Some(true)
                || control.delivery_drained != Some(true)
                || *control.scope != row.scope
                || row.phase != Phase::Stopped
            {
                return Err(Error::Unavailable);
            }
            let State::Active(p) = &mut self.collection_publication_streams else {
                unreachable!()
            };
            p.rows[id.slot] = None;
            Ok(())
        })();
        Self::finish_collection_publication(
            r,
            Boundary::CollectionPublicationRetirement,
            result,
            timer.finish(),
        )
    }
    pub fn retire_collection_publication_pool_v1(
        &mut self,
        control: &SourceReadPoolControlV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let p = self.collection_publication_pool()?;
            if [
                control.owner_control_allowed,
                control.retention_released,
                control.delivery_drained,
            ]
            .into_iter()
            .any(|v| v != Some(true))
                || p.rows.iter().any(Option::is_some)
                || !p.commands.closed()
            {
                return Err(Error::Unavailable);
            }
            let State::Active(p) = &mut self.collection_publication_streams else {
                unreachable!()
            };
            self.retention.release(&mut p.reservation).map_err(parent)?;
            self.collection_publication_streams = State::Closed { issued: p.issued };
            Ok(())
        })();
        Self::finish_collection_publication(
            r,
            Boundary::CollectionPublicationPoolRetirement,
            result,
            timer.finish(),
        )
    }
    /// Trusted fixed gauges, never scoped recipient progress or quiescence proof.
    pub fn collection_publication_usage_v1(&self) -> Result<CollectionPublicationUsageV1, Error> {
        match &self.collection_publication_streams {
            State::Unconfigured => Ok(CollectionPublicationUsageV1::default()),
            State::Closed { issued } => Ok(CollectionPublicationUsageV1 {
                issued_lifetime: *issued,
                closed: true,
                ..Default::default()
            }),
            State::Active(_) => {
                let p = self.collection_publication_pool()?;
                let mut u = CollectionPublicationUsageV1 {
                    issued_lifetime: p.issued,
                    reserved_pool_bytes: p.charge,
                    ..Default::default()
                };
                for row in p.rows.iter().flatten() {
                    u.streams += 1;
                    u.bindings = u
                        .bindings
                        .checked_add(row.bindings.len())
                        .ok_or(Error::Faulted)?;
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
