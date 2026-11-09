//! Native named-facet/source/logical-view sampled full-projection streams.
//! Post-publication service, not a subscriber, lossless audit or F19 command facade.
use super::*;
mod shutdown_inventory;
use std::collections::VecDeque;
use std::mem::size_of;
#[cfg(test)]
pub(in crate::coordinator) mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FacetPublicationErrorV1 {
    Unavailable,
    Limit,
    Capacity,
    Faulted,
}
type Error = FacetPublicationErrorV1;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FacetPublicationLimitsV1 {
    pub max_streams: usize,
    pub max_annotations: usize,
    pub max_views: usize,
    /// Per selected semantic view, also charged to projection bytes.
    pub max_selections: usize,
    /// Shared lifetime issuance for stream identities and visible generations.
    pub max_issued: usize,
    pub max_context_bytes: usize,
    pub max_projection_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_queue: usize,
    pub max_pool_bytes: usize,
}
impl FacetPublicationLimitsV1 {
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
pub struct FacetPublicationStreamIdV1 {
    owner: Arc<()>,
    slot: usize,
}
impl PartialEq for FacetPublicationStreamIdV1 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for FacetPublicationStreamIdV1 {}
#[derive(Clone, PartialEq, Eq)]
pub struct FacetPublicationGenerationV1 {
    stream: FacetPublicationStreamIdV1,
    serial: usize,
}
#[derive(Clone)]
pub struct FacetPublicationEventV1 {
    pub before: FacetPublicationGenerationV1,
    pub after: FacetPublicationGenerationV1,
    pub projection: Arc<FacetPublicationProjectionV1>,
}
pub struct FacetPublicationBaselineV1 {
    pub stream: FacetPublicationStreamIdV1,
    pub generation: FacetPublicationGenerationV1,
    pub projection: Arc<FacetPublicationProjectionV1>,
}
pub enum FacetPublicationDeliveryV1 {
    Projection(FacetPublicationEventV1),
    Empty,
    Gap,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FacetPublicationSampleV1 {
    Unchanged,
    Queued,
    Gap,
}
macro_rules! redacted { ($($t:ty),+) => { $(impl std::fmt::Debug for $t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(concat!(stringify!($t), "(<redacted>)")) }
})+ }; }
redacted!(
    FacetPublicationStreamIdV1,
    FacetPublicationGenerationV1,
    FacetPublicationProjectionV1,
    FacetPublicationEventV1,
    FacetPublicationBaselineV1,
    FacetPublicationDeliveryV1
);
pub(in crate::coordinator) mod commands;
mod projection;
pub use commands::*;
pub use projection::{
    FacetPublicationControlV1, FacetPublicationProjectionV1, FacetPublicationScopeV1,
    PublicationAnnotationFacetV1, PublicationAnnotationSelectionV1, PublicationSourceFacetV1,
    PublicationViewFacetV1, PublicationViewSelectionV1,
};
use projection::{copy_scope, facet_rights, scope_bytes};
/// Independently attributed host facts. The original scope binds these facts to
/// the retained observer; a lowered caller claim is not accepted loss evidence.
pub struct TrustedFacetPublicationFactsV1<'a> {
    pub original_scope: &'a FacetPublicationScopeV1,
    /// Shared current context gates; source readability is tested only when
    /// selected. Named annotation/view authority is supplied separately below.
    pub current: &'a TrustedSourceReadFactsV1<'a>,
    pub projection_allowed: Option<bool>,
    pub baseline_allowed: Option<bool>,
    pub max_reply_bytes: usize,
    pub annotation_grants: &'a [AnnotationFacetGrantV1],
    pub view_grants: &'a [LogicalViewId],
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FacetPublicationUsageV1 {
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
    scope: FacetPublicationScopeV1,
    generation: FacetPublicationGenerationV1,
    baseline: Option<Arc<FacetPublicationProjectionV1>>,
    queue: VecDeque<FacetPublicationEventV1>,
    phase: Phase,
}
impl Record {
    fn valid(
        &self,
        id: &FacetPublicationStreamIdV1,
        limits: FacetPublicationLimitsV1,
        issued: usize,
    ) -> bool {
        if self.generation.stream != *id
            || self.generation.serial >= issued
            || self.queue.len() > limits.max_queue
            || scope_bytes(&self.scope, limits).is_err()
        {
            return false;
        }
        let projection_valid = |p: &FacetPublicationProjectionV1| {
            p.valid(&self.scope, limits)
                && p.bytes().is_ok_and(|n| n <= limits.max_projection_bytes)
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
    limits: FacetPublicationLimitsV1,
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
                    let id = FacetPublicationStreamIdV1 {
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
fn ordinary(f: &TrustedFacetPublicationFactsV1<'_>) -> bool {
    [
        f.current.authorized,
        f.current.scope_allowed,
        f.current.continuous,
        f.current.live,
        f.current.not_stopped,
        f.current.retained,
        if f.original_scope.include_source {
            f.current.source_readable
        } else {
            Some(true)
        },
        f.projection_allowed,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn loss(f: &TrustedFacetPublicationFactsV1<'_>) -> bool {
    [
        f.current.authorized,
        f.current.scope_allowed,
        f.current.continuous,
        f.current.live,
        f.current.not_stopped,
        f.current.retained,
        if f.original_scope.include_source {
            f.current.source_readable
        } else {
            Some(true)
        },
        f.projection_allowed,
    ]
    .contains(&Some(false))
}
fn ready(f: &TrustedFacetPublicationFactsV1<'_>) -> bool {
    [
        f.current.resources_ready,
        f.current.reply_ready,
        f.current.transfer_ready,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn frame_bytes(p: &FacetPublicationProjectionV1) -> Result<usize, Error> {
    size_of::<FacetPublicationDeliveryV1>()
        .checked_add(p.bytes()?)
        .ok_or(Error::Limit)
}
impl SourceCoordinator {
    fn facet_publication_pool(&self) -> Result<&Pool, Error> {
        let State::Active(p) = &self.facet_publication_streams else {
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
    fn facet_publication_record(&self, id: &FacetPublicationStreamIdV1) -> Result<&Record, Error> {
        let row = self.facet_publication_record_control(id)?;
        let p = self.facet_publication_pool()?;
        if !row.valid(id, p.limits, p.issued) {
            return Err(Error::Faulted);
        }
        Ok(row)
    }
    fn facet_publication_record_control(
        &self,
        id: &FacetPublicationStreamIdV1,
    ) -> Result<&Record, Error> {
        let p = self.facet_publication_pool()?;
        if !Arc::ptr_eq(&p.owner, &id.owner) {
            return Err(Error::Unavailable);
        }
        p.rows
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)
    }
    fn facet_publication_record_mut(&mut self, id: &FacetPublicationStreamIdV1) -> &mut Record {
        let State::Active(p) = &mut self.facet_publication_streams else {
            unreachable!("checked publication owner")
        };
        p.rows[id.slot]
            .as_mut()
            .expect("checked publication context")
    }
    fn facet_publication_gap(&mut self, id: &FacetPublicationStreamIdV1, r: &mut Recorder) {
        self.facet_publication_record_mut(id).close(Phase::Gap);
        // A transition marker, not an invented duration or missing-event count.
        r.record(Observation {
            boundary: Boundary::FacetPublicationGap,
            disposition: Disposition::Completed,
            service_time: None,
        });
    }
    fn facet_publication_gate(
        &mut self,
        id: &FacetPublicationStreamIdV1,
        claimed: &FacetPublicationScopeV1,
        f: &TrustedFacetPublicationFactsV1<'_>,
    ) -> Result<(), Error> {
        self.ensure_operational().map_err(|_| Error::Faulted)?;
        self.facet_command_ordinary_gate_v1()?;
        if self.source_facade_busy {
            return Err(Error::Unavailable);
        }
        let row = self.facet_publication_record_control(id)?;
        if row.scope != *claimed || row.scope != *f.original_scope {
            return Err(Error::Unavailable);
        }
        let row = self.facet_publication_record(id)?;
        if row.phase == Phase::Stopped {
            return Err(Error::Unavailable);
        }
        if row.scope.source != *f.current.scope
            || loss(f)
            || facet_rights(&row.scope, f) == Some(false)
            || self.document(&row.scope.source.document).is_err()
        {
            self.facet_publication_record_mut(id).close(Phase::Stopped);
            return Err(Error::Unavailable);
        }
        if !ordinary(f) || facet_rights(&row.scope, f) != Some(true) {
            return Err(Error::Unavailable);
        }
        // Validate actual attached targets at every release as well as sample.
        // Supplied positive liveness never overrides a detached/retired/rebound view.
        let mut invalid = false;
        for selected in &row.scope.views {
            match self.publication_view(&selected.view) {
                Ok((source, _, _)) => {
                    invalid |= source.is_none_or(|v| v.document != row.scope.source.document)
                }
                Err(crate::selection::SelectionError::Faulted) => return Err(Error::Faulted),
                Err(_) => invalid = true,
            }
        }
        if invalid {
            self.facet_publication_record_mut(id).close(Phase::Stopped);
            return Err(Error::Unavailable);
        }
        Ok(())
    }
    fn finish_facet_publication<T>(
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
    pub fn configure_facet_publication_streams_v1(
        &mut self,
        authority: &str,
        engine: &str,
        limits: FacetPublicationLimitsV1,
        control: &SourceReadPoolControlV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(admission_error)?;
            if control.owner_control_allowed != Some(true)
                || control.resources_ready != Some(true)
                || !matches!(self.facet_publication_streams, State::Unconfigured)
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
            self.facet_publication_streams = State::Active(Box::new(Pool {
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
        Self::finish_facet_publication(
            r,
            Boundary::FacetPublicationConfiguration,
            result,
            timer.finish(),
        )
    }
    /// Raw technical establishment. F19 original-command/replay and recipient
    /// facade assembly remain separate; matching retries must not call this again.
    pub fn open_facet_publication_stream_v1(
        &mut self,
        s: &FacetPublicationScopeV1,
        f: &TrustedFacetPublicationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<FacetPublicationBaselineV1, Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(admission_error)?;
            self.facet_command_ordinary_gate_v1()?;
            if self.source_facade_busy
                || f.original_scope != s
                || f.current.scope != &s.source
                || !ordinary(f)
                || facet_rights(s, f) != Some(true)
                || !ready(f)
                || f.baseline_allowed != Some(true)
            {
                return Err(Error::Unavailable);
            }
            let p = self.facet_publication_pool()?;
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
            if scope_bytes(s, limits)? > limits.max_context_bytes {
                return Err(Error::Limit);
            }
            let projection = self.facet_publication_projection(s, limits)?;
            if frame_bytes(&projection)? > f.max_reply_bytes {
                return Err(Error::Limit);
            }
            let scope = copy_scope(s, limits)?;
            let mut queue = VecDeque::new();
            queue
                .try_reserve_exact(limits.max_queue)
                .map_err(|_| Error::Capacity)?;
            let State::Active(p) = &mut self.facet_publication_streams else {
                unreachable!()
            };
            let id = FacetPublicationStreamIdV1 {
                owner: p.owner.clone(),
                slot: p.rows.len(),
            };
            let generation = FacetPublicationGenerationV1 {
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
            Ok(FacetPublicationBaselineV1 {
                stream: id,
                generation,
                projection,
            })
        })();
        Self::finish_facet_publication(r, Boundary::FacetPublicationOpen, result, timer.finish())
    }
    /// Samples actual committed state. No publisher hook or subscriber callback;
    /// skipped intermediate roots are not represented as a lossless audit.
    pub fn sample_facet_publication_stream_v1(
        &mut self,
        id: &FacetPublicationStreamIdV1,
        claimed: &FacetPublicationScopeV1,
        f: &TrustedFacetPublicationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<FacetPublicationSampleV1, Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.facet_publication_gate(id, claimed, f)?;
            if self.facet_publication_record(id)?.phase == Phase::Gap {
                return Ok(FacetPublicationSampleV1::Gap);
            }
            if f.current.resources_ready != Some(true) {
                self.facet_publication_gap(id, r);
                return Ok(FacetPublicationSampleV1::Gap);
            }
            let limits = self.facet_publication_pool()?.limits;
            let row = self.facet_publication_record(id)?;
            if let Some(baseline) = &row.baseline
                && self.facet_publication_matches(&row.scope, baseline)?
            {
                return Ok(FacetPublicationSampleV1::Unchanged);
            }
            if row.queue.len() >= limits.max_queue
                || self.facet_publication_pool()?.issued >= limits.max_issued
            {
                self.facet_publication_gap(id, r);
                return Ok(FacetPublicationSampleV1::Gap);
            }
            let projection = match self.facet_publication_projection(claimed, limits) {
                Ok(p) => p,
                Err(Error::Limit | Error::Capacity) => {
                    self.facet_publication_gap(id, r);
                    return Ok(FacetPublicationSampleV1::Gap);
                }
                Err(e) => return Err(e),
            };
            let row = self.facet_publication_record(id)?;
            if row.baseline.as_ref().is_some_and(|b| **b == *projection) {
                return Ok(FacetPublicationSampleV1::Unchanged);
            }
            if row.queue.len() >= limits.max_queue
                || self.facet_publication_pool()?.issued >= limits.max_issued
            {
                self.facet_publication_gap(id, r);
                return Ok(FacetPublicationSampleV1::Gap);
            }
            let before = row.generation.clone();
            let after = FacetPublicationGenerationV1 {
                stream: id.clone(),
                serial: before.serial.checked_add(1).ok_or(Error::Limit)?,
            };
            let State::Active(p) = &mut self.facet_publication_streams else {
                unreachable!()
            };
            p.issued += 1;
            let row = p.rows[id.slot].as_mut().expect("checked stream");
            row.queue.push_back(FacetPublicationEventV1 {
                before,
                after: after.clone(),
                projection: projection.clone(),
            });
            row.baseline = Some(projection);
            row.generation = after;
            Ok(FacetPublicationSampleV1::Queued)
        })();
        Self::finish_facet_publication(r, Boundary::FacetPublicationSample, result, timer.finish())
    }
    pub fn release_facet_publication_event_v1(
        &mut self,
        id: &FacetPublicationStreamIdV1,
        claimed: &FacetPublicationScopeV1,
        f: &TrustedFacetPublicationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<FacetPublicationDeliveryV1, Error> {
        self.release_facet_publication_event_frame_v1(id, claimed, f, 0, r)
    }
    pub(in crate::coordinator) fn release_facet_publication_event_frame_v1(
        &mut self,
        id: &FacetPublicationStreamIdV1,
        claimed: &FacetPublicationScopeV1,
        f: &TrustedFacetPublicationFactsV1<'_>,
        extra_header: usize,
        r: &mut Recorder,
    ) -> Result<FacetPublicationDeliveryV1, Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.facet_publication_gate(id, claimed, f)?;
            if !ready(f) {
                return Err(Error::Unavailable);
            }
            let row = self.facet_publication_record(id)?;
            let reply_limit = f
                .max_reply_bytes
                .min(self.facet_publication_pool()?.limits.max_reply_bytes);
            if extra_header != 0
                && size_of::<FacetPublicationDeliveryV1>()
                    .checked_add(extra_header)
                    .is_none_or(|n| n > reply_limit)
            {
                return Err(Error::Limit);
            }
            if row.phase == Phase::Gap {
                return Ok(FacetPublicationDeliveryV1::Gap);
            }
            if let Some(head) = row.queue.front()
                && frame_bytes(&head.projection)?
                    .checked_add(extra_header)
                    .is_none_or(|n| n > reply_limit)
            {
                return Err(Error::Limit);
            }
            Ok(
                match self.facet_publication_record_mut(id).queue.pop_front() {
                    Some(event) => FacetPublicationDeliveryV1::Projection(event),
                    None => FacetPublicationDeliveryV1::Empty,
                },
            )
        })();
        Self::finish_facet_publication(r, Boundary::FacetPublicationRelease, result, timer.finish())
    }
    pub fn resync_facet_publication_stream_v1(
        &mut self,
        id: &FacetPublicationStreamIdV1,
        claimed: &FacetPublicationScopeV1,
        f: &TrustedFacetPublicationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<FacetPublicationBaselineV1, Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.facet_publication_gate(id, claimed, f)?;
            if !ready(f) || f.baseline_allowed != Some(true) {
                return Err(Error::Unavailable);
            }
            let p = self.facet_publication_pool()?;
            if p.issued >= p.limits.max_issued {
                return Err(Error::Limit);
            }
            let projection = self.facet_publication_projection(claimed, p.limits)?;
            if frame_bytes(&projection)? > f.max_reply_bytes {
                return Err(Error::Limit);
            }
            let generation = FacetPublicationGenerationV1 {
                stream: id.clone(),
                serial: self
                    .facet_publication_record(id)?
                    .generation
                    .serial
                    .checked_add(1)
                    .ok_or(Error::Limit)?,
            };
            let State::Active(p) = &mut self.facet_publication_streams else {
                unreachable!()
            };
            p.issued += 1;
            let row = p.rows[id.slot].as_mut().expect("checked stream");
            row.queue.clear();
            row.baseline = Some(projection.clone());
            row.generation = generation.clone();
            row.phase = Phase::Active;
            Ok(FacetPublicationBaselineV1 {
                stream: id.clone(),
                generation,
                projection,
            })
        })();
        Self::finish_facet_publication(r, Boundary::FacetPublicationResync, result, timer.finish())
    }
    pub fn stop_facet_publication_stream_v1(
        &mut self,
        id: &FacetPublicationStreamIdV1,
        control: &FacetPublicationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let row = self.facet_publication_record_control(id)?;
            if control.owner_control_allowed != Some(true) || *control.scope != row.scope {
                return Err(Error::Unavailable);
            }
            self.facet_publication_record_mut(id).close(Phase::Stopped);
            Ok(())
        })();
        Self::finish_facet_publication(r, Boundary::FacetPublicationStop, result, timer.finish())
    }
    pub fn retire_facet_publication_stream_v1(
        &mut self,
        id: &FacetPublicationStreamIdV1,
        control: &FacetPublicationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let row = self.facet_publication_record_control(id)?;
            if control.owner_control_allowed != Some(true)
                || control.retention_released != Some(true)
                || control.delivery_drained != Some(true)
                || *control.scope != row.scope
                || row.phase != Phase::Stopped
            {
                return Err(Error::Unavailable);
            }
            let State::Active(p) = &mut self.facet_publication_streams else {
                unreachable!()
            };
            p.rows[id.slot] = None;
            Ok(())
        })();
        Self::finish_facet_publication(
            r,
            Boundary::FacetPublicationRetirement,
            result,
            timer.finish(),
        )
    }
    pub fn retire_facet_publication_pool_v1(
        &mut self,
        control: &SourceReadPoolControlV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let p = self.facet_publication_pool()?;
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
            let State::Active(p) = &mut self.facet_publication_streams else {
                unreachable!()
            };
            self.retention.release(&mut p.reservation).map_err(parent)?;
            self.facet_publication_streams = State::Closed { issued: p.issued };
            Ok(())
        })();
        Self::finish_facet_publication(
            r,
            Boundary::FacetPublicationPoolRetirement,
            result,
            timer.finish(),
        )
    }
    /// Trusted fixed gauges, never scoped recipient progress or quiescence proof.
    pub fn facet_publication_usage_v1(&self) -> Result<FacetPublicationUsageV1, Error> {
        match &self.facet_publication_streams {
            State::Unconfigured => Ok(FacetPublicationUsageV1::default()),
            State::Closed { issued } => Ok(FacetPublicationUsageV1 {
                issued_lifetime: *issued,
                closed: true,
                ..Default::default()
            }),
            State::Active(_) => {
                let p = self.facet_publication_pool()?;
                let mut u = FacetPublicationUsageV1 {
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
