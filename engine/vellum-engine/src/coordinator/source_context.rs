//! Funded single-document decoded-source projections. No registry/provider I/O.
use super::*;
mod shutdown_inventory;
use crate::coordinates::{
    BytePosition, CoordinateBudget, CoordinateError, LineSpan, SourceCoordinates,
};
mod control;
pub use facade::ReadObservationStopPortV1;
pub use facade::{
    AnnotationArgumentFactsV1, AnnotationArgumentQueryV1, AnnotationArgumentTargetV1,
    AnnotationEnvelopeValueV1, DecodedAnnotationFacadeRequestV1, NormalizedAnnotationArgumentsV1,
    normalize_annotation_envelope_v1,
};
pub use facade::{
    DecodedHistoryFacadeRequestV1, HistoryEnvelopeValueV1, HistoryFacadeRequestV1,
    NormalizedHistoryArgumentsV1, normalize_history_envelope_v1,
};
pub use facade::{
    DecodedReadAcquisitionFacadeRequestV1, NormalizedReadAcquisitionArgumentsV1,
    ReadAcquisitionEnvelopeValueV1, ReadAcquisitionParticipantV1, ReadAcquisitionSelectionV1,
    normalize_read_acquisition_envelope_v1,
};
pub use facade::{
    DecodedReadObservationStopRequestV1, NormalizedReadObservationStopArgumentsV1,
    ReadObservationStopEnvelopeValueV1, ReadObservationStopTargetV1,
    normalize_read_observation_stop_envelope_v1,
};
pub use facade::{
    DecodedSourceFacadeRequestV1, NormalizedSourceArgumentsV1, SourceEnvelopeValueV1,
    normalize_source_envelope_v1,
};
pub use facade::{
    ReadAcquisitionFacadePortV1, ReadAcquisitionFacadeReplyV1, ReadAcquisitionFacadeRequestV1,
};
pub use facade::{SearchFacadePortV1, SearchFacadeReplyV1, SearchFacadeRequestV1};
pub(in crate::coordinator) mod facade;
pub use facade::{
    AnnotationFacadePortV1, AnnotationFacadeReplyV1, AnnotationFacadeRequestV1, ReadFacadeUsageV1,
};
pub use facade::{SourceFacadeReplyV1, SourceFacadeRequestV1, SourceFacadeUsageV1};
pub(in crate::coordinator) struct SearchSnapshot<'a> {
    pub source: &'a Utf8Source,
    pub version: &'a DocumentVersion,
    pub max_root_bytes: usize,
    pub max_request_bytes: usize,
    pub max_reply_bytes: usize,
}
pub(in crate::coordinator) use search::State as ManagedSearchState;
pub use search::normalize_literal_search_envelope_v1;
pub use search::{
    BoundLiteralSearchErrorV1, BoundLiteralSearchProductV1, BoundLiteralSearchReplyV1,
    BoundLiteralSearchResultV1, LITERAL_EXACT_MATCHING_PROFILE_V1, LogicalSearchBindingV1,
    TrustedLogicalSearchBindingV1,
};
pub(in crate::coordinator) use search::{ConsumerCapture, ConsumerPlan};
pub use search::{
    LiteralSearchArgumentLimitsV1, LiteralSearchEnvelopeValueV1,
    LiteralSearchNavigationArgumentsV1, LiteralSearchOperationV1, LiteralSearchReferenceV1,
    NormalizedLiteralSearchArgumentsV1, normalize_literal_search_arguments_v1,
};
pub use search::{LiteralSearchValueFieldsV1, literal_search_value_fields_v1};
pub use search::{
    SearchReadContextV1, SearchReadFactsV1, SearchResultControlV1, SourceSearchFoundV1,
    SourceSearchNavigationReplyV1, SourceSearchNavigationV1, SourceSearchOccurrenceId,
    TrustedVersionSetSearchFactsV1,
};
pub use search::{SourceSearchConsumerV1, TrustedSourceSearchConsumerFactsV1};
pub use search::{
    SourceSearchLimitsV1, SourceSearchReplyV1, SourceSearchUsageV1, TrustedSourceSearchFactsV1,
};
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceReadContextLimitsV1 {
    pub max_contexts: usize,
    pub max_holds: usize,
    /// Shared lifetime issuance for contexts and internal holds; never refunded.
    pub max_issued: usize,
    pub max_root_bytes: usize,
    pub max_capture_bytes: usize,
    pub max_context_bytes: usize,
    pub max_request_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_label_bytes: usize,
    pub max_scan_bytes: usize,
    pub max_pool_bytes: usize,
}
impl SourceReadContextLimitsV1 {
    /// Native inline record/payload units. Includes independent copy/reply
    /// workspace even with zero context capacity, and lifetime tombstones.
    pub fn required_pool_bytes(self) -> Result<usize, SourceReadContextErrorV1> {
        let mut n = std::mem::size_of::<Pool>();
        for (count, size) in [
            (self.max_issued, std::mem::size_of::<Option<Record>>()),
            (self.max_issued, std::mem::size_of::<Option<Hold>>()),
            (2, self.max_request_bytes),
            (1, self.max_capture_bytes),
            (2, self.max_context_bytes),
            (2, self.max_reply_bytes),
            (self.max_contexts, self.max_capture_bytes),
        ] {
            n = n
                .checked_add(count.checked_mul(size).ok_or(Error::Limit)?)
                .ok_or(Error::Limit)?;
        }
        if n > self.max_pool_bytes {
            return Err(Error::Limit);
        }
        Ok(n)
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct SourceReadScopeV1 {
    pub document: DocumentId,
    pub context: SettingsExecutionContextV1,
    pub policy: String,
}
impl std::fmt::Debug for SourceReadScopeV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceReadScopeV1(<redacted>)")
    }
}
/// Separately attributed current host-policy facts, not requester assertions.
pub struct TrustedSourceReadFactsV1<'a> {
    pub scope: &'a SourceReadScopeV1,
    pub authorized: Option<bool>,
    pub scope_allowed: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub retained: Option<bool>,
    pub source_readable: Option<bool>,
    pub resources_ready: Option<bool>,
    pub reply_ready: Option<bool>,
    pub transfer_ready: Option<bool>,
}
pub struct SourceReadContextControlV1<'a> {
    pub scope: &'a SourceReadScopeV1,
    pub owner_control_allowed: Option<bool>,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
}
pub struct SourceReadPoolControlV1 {
    pub owner_control_allowed: Option<bool>,
    pub resources_ready: Option<bool>,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
}
pub struct SourceReadHoldFactsV1 {
    pub owner_control_allowed: Option<bool>,
    pub retained: Option<bool>,
    pub resources_ready: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceReadContextErrorV1 {
    Unavailable,
    Limit,
    Capacity,
    InvalidRequest,
    Faulted,
}
type Error = SourceReadContextErrorV1;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceReadContextUsageV1 {
    pub contexts: usize,
    pub holds: usize,
    pub retained_snapshots: usize,
    pub issued_lifetime: usize,
    pub reserved_pool_bytes: usize,
    pub closed: bool,
}
macro_rules! identity {
    ($name:ident) => {
        #[derive(Clone)]
        pub struct $name {
            owner: Arc<()>,
            slot: usize,
        }
        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
            }
        }
        impl Eq for $name {}
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(concat!(stringify!($name), "(<opaque>)"))
            }
        }
    };
}
identity!(SourceReadContextId);
identity!(SourceReadHoldId);
identity!(SourceSearchResultId);
pub(in crate::coordinator) mod search;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceContextQueryV1 {
    Range {
        start: BytePosition,
        end: BytePosition,
    },
    Line {
        index: usize,
    },
}
pub enum SourceContextContentV1 {
    Range(SourceRangeReply),
    Line(SourceLineReply),
}
pub struct SourceContextReplyV1 {
    pub context: SourceReadContextId,
    pub content: SourceContextContentV1,
}
impl std::fmt::Debug for SourceContextReplyV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceContextReplyV1(<redacted>)")
    }
}
struct Record {
    scope: SourceReadScopeV1,
    version: DocumentVersion,
    source: Option<Utf8Source>,
    closed: bool,
    holds: usize,
}
impl Record {
    fn close(&mut self) {
        self.closed = true;
        if self.holds == 0 {
            self.source = None;
        }
    }
}
struct Hold {
    context: usize,
}
pub(super) struct Pool {
    authority: String,
    engine: String,
    limits: SourceReadContextLimitsV1,
    reservation: retention::Reservation,
    charge: usize,
    rows: Vec<Option<Record>>,
    holds: Vec<Option<Hold>>,
}
impl Pool {
    fn actual_holds(&self, context: usize) -> Result<usize, Error> {
        let mut count = 0usize;
        for hold in self.holds.iter().flatten() {
            if self
                .rows
                .get(hold.context)
                .and_then(Option::as_ref)
                .is_none()
            {
                return Err(Error::Faulted);
            }
            if hold.context == context {
                count = count.checked_add(1).ok_or(Error::Faulted)?;
            }
        }
        Ok(count)
    }
    fn close_context(&mut self, context: usize) -> Result<(), Error> {
        let actual = self.actual_holds(context)?;
        let row = self
            .rows
            .get_mut(context)
            .and_then(Option::as_mut)
            .ok_or(Error::Faulted)?;
        if row.holds != actual {
            return Err(Error::Faulted);
        }
        row.close();
        Ok(())
    }
    fn issued(&self) -> Result<usize, Error> {
        self.rows
            .len()
            .checked_add(self.holds.len())
            .ok_or(Error::Faulted)
    }
    fn can_issue(&self) -> Result<(), Error> {
        if self.issued()? >= self.limits.max_issued {
            Err(Error::Limit)
        } else {
            Ok(())
        }
    }
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
impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceReadContextState(<redacted>)")
    }
}
impl State {
    pub(in crate::coordinator) fn document_pins(
        &self,
        document: &DocumentId,
    ) -> Result<(usize, usize), CoordinatorError> {
        let p = match self {
            Self::Unconfigured | Self::Closed { issued: _ } => return Ok((0, 0)),
            Self::Active(p) => p,
        };
        for h in p.holds.iter().flatten() {
            if p.rows.get(h.context).and_then(Option::as_ref).is_none() {
                return Err(CoordinatorError::Faulted);
            }
        }
        let mut contexts = 0;
        let mut holds = 0usize;
        for (index, row) in p
            .rows
            .iter()
            .enumerate()
            .filter_map(|(i, r)| r.as_ref().map(|r| (i, r)))
        {
            let actual = p
                .actual_holds(index)
                .map_err(|_| CoordinatorError::Faulted)?;
            if actual != row.holds || row.version.document != row.scope.document {
                return Err(CoordinatorError::Faulted);
            }
            if row.scope.document == *document {
                contexts += 1;
                holds = holds.checked_add(actual).ok_or(CoordinatorError::Faulted)?;
            }
        }
        Ok((contexts, holds))
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
fn source_error(e: crate::source::SourceError) -> Error {
    match e {
        crate::source::SourceError::Capacity => Error::Capacity,
        _ => Error::Faulted,
    }
}
pub(in crate::coordinator) fn admission_error(e: CoordinatorError) -> Error {
    match e {
        CoordinatorError::Faulted => Error::Faulted,
        _ => Error::Unavailable,
    }
}
fn coordinate(e: CoordinateError) -> Error {
    match e {
        CoordinateError::Limit => Error::Limit,
        CoordinateError::Capacity => Error::Capacity,
        _ => Error::InvalidRequest,
    }
}
fn observe<T>(r: &mut Recorder, boundary: Boundary, result: &Result<T, Error>) {
    r.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(Error::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time: None,
    });
}
fn allowed(f: &TrustedSourceReadFactsV1<'_>) -> bool {
    [
        f.authorized,
        f.scope_allowed,
        f.continuous,
        f.live,
        f.not_stopped,
        f.retained,
        f.source_readable,
        f.resources_ready,
        f.reply_ready,
        f.transfer_ready,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn lost(f: &TrustedSourceReadFactsV1<'_>) -> bool {
    [
        f.authorized,
        f.scope_allowed,
        f.continuous,
        f.live,
        f.retained,
        f.source_readable,
    ]
    .contains(&Some(false))
}
pub(in crate::coordinator) fn scope_bytes(s: &SourceReadScopeV1) -> Result<usize, Error> {
    settings::basis::labels(&s.context).into_iter().try_fold(
        std::mem::size_of::<SourceReadScopeV1>()
            .checked_add(s.policy.len())
            .ok_or(Error::Limit)?,
        |n, label| n.checked_add(label.len()).ok_or(Error::Limit),
    )
}
fn valid_scope(s: &SourceReadScopeV1, p: &Pool) -> bool {
    s.context.authority == p.authority
        && s.context.engine == p.engine
        && !s.policy.is_empty()
        && s.policy.len() <= p.limits.max_label_bytes
        && settings::basis::labels(&s.context)
            .into_iter()
            .all(|v| !v.is_empty() && v.len() <= p.limits.max_label_bytes)
        && scope_bytes(s).is_ok_and(|n| n <= p.limits.max_context_bytes)
}
fn live(rows: &[Option<Record>]) -> usize {
    rows.iter().filter(|r| r.is_some()).count()
}
fn held(rows: &[Option<Hold>]) -> usize {
    rows.iter().filter(|r| r.is_some()).count()
}
#[derive(Clone, Copy)]
pub(in crate::coordinator) enum Resolved {
    Range(BytePosition, BytePosition),
    Line(usize, LineSpan),
}
#[derive(Clone, Copy)]
enum Selection<'a> {
    Current,
    Retained(&'a SourceReadContextId),
}
impl Resolved {
    pub(in crate::coordinator) fn resolve(
        source: &Utf8Source,
        q: SourceContextQueryV1,
    ) -> Result<Self, Error> {
        let c = SourceCoordinates::new(source);
        match q {
            SourceContextQueryV1::Range { start, end } => {
                c.validate_range(start, end).map_err(coordinate)?;
                Ok(Self::Range(start, end))
            }
            SourceContextQueryV1::Line { index } => {
                // Semantic shape validation has its own finite captured-source
                // bound; tighter delivery scan budgets cannot hide bound loss.
                let span = c
                    .line_span(
                        index,
                        CoordinateBudget {
                            max_scan_bytes: source.text().len(),
                        },
                    )
                    .map_err(coordinate)?;
                Ok(Self::Line(index, span))
            }
        }
    }
    pub(in crate::coordinator) fn payload_bytes(self) -> usize {
        match self {
            Self::Range(a, b) => b.0 - a.0,
            Self::Line(_, s) => s.end.0 - s.start.0,
        }
    }
}
impl SourceCoordinator {
    fn source_pool(&self) -> Result<&Pool, Error> {
        match &self.source_contexts {
            State::Active(p) => Ok(p),
            _ => Err(Error::Unavailable),
        }
    }
    fn source_context_row(
        &self,
        id: &SourceReadContextId,
        s: &SourceReadScopeV1,
    ) -> Result<&Record, Error> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return Err(Error::Unavailable);
        }
        let p = self.source_pool()?;
        let row = p
            .rows
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)?;
        if row.scope != *s {
            return Err(Error::Unavailable);
        }
        Ok(row)
    }
    pub fn configure_source_read_contexts(
        &mut self,
        authority: &str,
        engine: &str,
        p: SourceReadContextLimitsV1,
        f: &SourceReadPoolControlV1,
        r: &mut Recorder,
    ) -> Result<bool, Error> {
        let result = (|| {
            if self.faulted {
                return Err(Error::Faulted);
            }
            if f.owner_control_allowed != Some(true) || f.resources_ready != Some(true) {
                return Err(Error::Unavailable);
            }
            if authority.is_empty()
                || engine.is_empty()
                || authority.len() > p.max_label_bytes
                || engine.len() > p.max_label_bytes
            {
                return Err(Error::InvalidRequest);
            }
            match &self.source_contexts {
                State::Active(old) => {
                    self.retention.validate(&old.reservation).map_err(parent)?;
                    return if old.authority == authority && old.engine == engine && old.limits == p
                    {
                        Ok(false)
                    } else {
                        Err(Error::Unavailable)
                    };
                }
                State::Closed { .. } => return Err(Error::Unavailable),
                State::Unconfigured => {}
            }
            self.ensure_admission_unfenced().map_err(admission_error)?;
            let config = std::mem::size_of::<SourceReadContextLimitsV1>()
                .checked_add(authority.len())
                .and_then(|n| n.checked_add(engine.len()))
                .ok_or(Error::Limit)?;
            if config > p.max_request_bytes || config > p.max_context_bytes {
                return Err(Error::Limit);
            }
            let charge = p
                .required_pool_bytes()?
                .checked_add(authority.len())
                .and_then(|n| n.checked_add(engine.len()))
                .filter(|n| *n <= p.max_pool_bytes)
                .ok_or(Error::Limit)?;
            self.retention.check(charge).map_err(parent)?;
            let authority = owned(authority)?;
            let engine = owned(engine)?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(p.max_issued)
                .map_err(|_| Error::Capacity)?;
            let mut holds = Vec::new();
            holds
                .try_reserve_exact(p.max_issued)
                .map_err(|_| Error::Capacity)?;
            let reservation = self.retention.reserve(charge).map_err(parent)?;
            self.source_contexts = State::Active(Box::new(Pool {
                authority,
                engine,
                limits: p,
                reservation,
                charge,
                rows,
                holds,
            }));
            Ok(true)
        })();
        if result == Err(Error::Faulted) {
            self.faulted = true;
        }
        observe(r, Boundary::SourceContextConfiguration, &result);
        result
    }
    pub fn source_read_context_usage(&self) -> SourceReadContextUsageV1 {
        match &self.source_contexts {
            State::Unconfigured => SourceReadContextUsageV1::default(),
            State::Closed { issued, .. } => SourceReadContextUsageV1 {
                issued_lifetime: *issued,
                closed: true,
                ..SourceReadContextUsageV1::default()
            },
            State::Active(p) => SourceReadContextUsageV1 {
                contexts: live(&p.rows),
                holds: held(&p.holds),
                retained_snapshots: p
                    .rows
                    .iter()
                    .flatten()
                    .filter(|r| r.source.is_some())
                    .count(),
                issued_lifetime: p.rows.len() + p.holds.len(),
                reserved_pool_bytes: p.charge,
                closed: false,
            },
        }
    }
    /// Current source only; no annotation/combined or version-set widening.
    pub fn capture_source_read_context(
        &mut self,
        s: &SourceReadScopeV1,
        f: &TrustedSourceReadFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<SourceReadContextId> {
        self.capture_source_read_context_result(s, f, r).ok()
    }
    pub(in crate::coordinator) fn capture_source_read_context_result(
        &mut self,
        s: &SourceReadScopeV1,
        f: &TrustedSourceReadFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourceReadContextId, Error> {
        self.acquire_source_context(Selection::Current, s, f, r)
    }
    /// Copy an original available source projection, not arbitrary historical roots.
    pub fn capture_retained_source_read_context(
        &mut self,
        original: &SourceReadContextId,
        s: &SourceReadScopeV1,
        f: &TrustedSourceReadFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<SourceReadContextId> {
        self.capture_retained_source_read_context_result(original, s, f, r)
            .ok()
    }
    pub(in crate::coordinator) fn capture_retained_source_read_context_result(
        &mut self,
        original: &SourceReadContextId,
        s: &SourceReadScopeV1,
        f: &TrustedSourceReadFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourceReadContextId, Error> {
        self.acquire_source_context(Selection::Retained(original), s, f, r)
    }
    fn selected_source<'a>(
        &'a self,
        selection: Selection<'_>,
        s: &SourceReadScopeV1,
    ) -> Result<(&'a Utf8Source, DocumentVersion), Error> {
        match selection {
            Selection::Current => {
                let d = self.document(&s.document).map_err(|_| Error::Unavailable)?;
                Ok((
                    &d.source,
                    DocumentVersion {
                        document: s.document.clone(),
                        revision: d.revision,
                    },
                ))
            }
            Selection::Retained(id) => {
                let row = self.source_context_row(id, s)?;
                if row.closed {
                    return Err(Error::Unavailable);
                }
                if row.version.document != s.document {
                    return Err(Error::Faulted);
                }
                Ok((
                    row.source.as_ref().ok_or(Error::Faulted)?,
                    row.version.clone(),
                ))
            }
        }
    }
    fn acquire_source_context(
        &mut self,
        selection: Selection<'_>,
        s: &SourceReadScopeV1,
        f: &TrustedSourceReadFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourceReadContextId, Error> {
        let result = (|| {
            let p = self.source_pool()?;
            if s != f.scope || !valid_scope(s, p) {
                return Err(Error::Unavailable);
            }
            let request_bytes = scope_bytes(s)?
                .checked_add(std::mem::size_of::<DocumentId>())
                .and_then(|n| {
                    n.checked_add(if matches!(selection, Selection::Retained(_)) {
                        std::mem::size_of::<SourceReadContextId>()
                    } else {
                        0
                    })
                })
                .ok_or(Error::Limit)?;
            if request_bytes > p.limits.max_request_bytes {
                return Err(Error::Limit);
            }
            if let Selection::Retained(original) = selection {
                let row = self.source_context_row(original, s)?;
                if row.closed {
                    return Err(Error::Unavailable);
                }
                if row.source.is_none() || row.version.document != s.document {
                    return Err(Error::Faulted);
                }
                // Shape/identity/scope precede accepted loss. Loss precedes new
                // occupancy/issuance checks, even when those budgets are exhausted.
                if self.faulted || self.document(&s.document).is_err() || lost(f) {
                    let State::Active(p) = &mut self.source_contexts else {
                        return Err(Error::Faulted);
                    };
                    p.close_context(original.slot)?;
                    observe(r, Boundary::SourceContextInvalidation, &Ok::<_, Error>(()));
                    return Err(Error::Unavailable);
                }
            }
            if self.faulted {
                return Err(Error::Faulted);
            }
            let p = self.source_pool()?;
            if !allowed(f) {
                return Err(Error::Unavailable);
            }
            // A retained copy is fresh issuance too. Existing result service
            // and accepted loss above retain their independent control path.
            self.ensure_admission_unfenced().map_err(admission_error)?;
            self.retention.validate(&p.reservation).map_err(parent)?;
            p.can_issue()?;
            if live(&p.rows) >= p.limits.max_contexts {
                return Err(Error::Limit);
            }
            let (source, version) = self.selected_source(selection, s)?;
            let capture_bytes = std::mem::size_of::<Record>()
                .checked_add(scope_bytes(s)? - std::mem::size_of::<SourceReadScopeV1>())
                .and_then(|n| n.checked_add(source.text().len()))
                .ok_or(Error::Limit)?;
            if capture_bytes > p.limits.max_capture_bytes
                || std::mem::size_of::<Option<SourceReadContextId>>() > p.limits.max_reply_bytes
                || scope_bytes(s)?
                    .checked_add(std::mem::size_of::<DocumentId>())
                    .is_none_or(|n| n > p.limits.max_request_bytes)
            {
                return Err(Error::Limit);
            }
            let root_ceiling = p.limits.max_root_bytes;
            if matches!(selection, Selection::Current)
                && self
                    .semantic_root_usage(r)
                    .map_err(|_| Error::Faulted)?
                    .total_bytes
                    > root_ceiling
            {
                return Err(Error::Limit);
            }
            let source = source.copy_owned().map_err(source_error)?;
            let context = settings::basis::copy_context(&s.context).map_err(|e| {
                if e == crate::settings::SettingsError::Capacity {
                    Error::Capacity
                } else {
                    Error::Limit
                }
            })?;
            let policy = owned(&s.policy)?;
            let expected_slot = self.source_pool()?.rows.len();
            #[allow(unused_mut)]
            let mut id = SourceReadContextId {
                owner: self.namespace.clone(),
                slot: expected_slot,
            };
            #[cfg(test)]
            match self.acquisition_result_fault {
                Some(source_query::result_contract::AcquisitionResultFault::Context) => {
                    id.slot = usize::MAX
                }
                Some(source_query::result_contract::AcquisitionResultFault::RetainedParent) => {
                    if let Selection::Retained(original) = selection {
                        id = original.clone();
                    }
                }
                _ => (),
            }
            let accepted = Arc::ptr_eq(&id.owner, &self.namespace)
                && id.slot == expected_slot
                && match selection {
                    Selection::Current => true,
                    Selection::Retained(original) => id != *original,
                };
            if !self.finish_read_result(
                accepted,
                ReadResultInvariantV1::NativeReadAcquisitionResultInvariant,
                Boundary::ReadAcquisitionResultValidation,
                r,
            ) {
                return Err(Error::Faulted);
            }
            let State::Active(p) = &mut self.source_contexts else {
                return Err(Error::Faulted);
            };
            p.rows.push(Some(Record {
                scope: SourceReadScopeV1 {
                    document: s.document.clone(),
                    context,
                    policy,
                },
                version,
                source: Some(source),
                closed: false,
                holds: 0,
            }));
            Ok(id)
        })();
        if result == Err(Error::Faulted) {
            self.faulted = true;
        }
        observe(
            r,
            if matches!(selection, Selection::Current) {
                Boundary::SourceContextCapture
            } else {
                Boundary::SourceContextRetainedCapture
            },
            &result,
        );
        result
    }
    pub fn query_source_read_context(
        &mut self,
        id: &SourceReadContextId,
        s: &SourceReadScopeV1,
        q: SourceContextQueryV1,
        f: &TrustedSourceReadFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<SourceContextReplyV1> {
        self.query_source_read_context_result(id, s, q, f, r).ok()
    }
    #[allow(clippy::too_many_arguments)]
    pub(in crate::coordinator) fn query_source_read_context_result(
        &mut self,
        id: &SourceReadContextId,
        s: &SourceReadScopeV1,
        q: SourceContextQueryV1,
        f: &TrustedSourceReadFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourceContextReplyV1, Error> {
        let result = (|| {
            let row = self.source_context_row(id, s)?;
            let p = self.source_pool()?;
            if s != f.scope || row.closed {
                return Err(Error::Unavailable);
            }
            if scope_bytes(s)?
                .checked_add(std::mem::size_of::<SourceContextQueryV1>())
                .and_then(|n| n.checked_add(std::mem::size_of::<SourceReadContextId>()))
                .is_none_or(|n| n > p.limits.max_request_bytes)
            {
                return Err(Error::Limit);
            }
            let source = row.source.as_ref().ok_or(Error::Faulted)?;
            let resolved = Resolved::resolve(source, q)?;
            let invalid = self.faulted || self.document(&s.document).is_err() || lost(f);
            if invalid {
                let State::Active(p) = &mut self.source_contexts else {
                    return Err(Error::Faulted);
                };
                p.close_context(id.slot)?;
                observe(r, Boundary::SourceContextInvalidation, &Ok::<_, Error>(()));
                return Err(Error::Unavailable);
            }
            self.retention.validate(&p.reservation).map_err(parent)?;
            if !allowed(f) {
                return Err(Error::Unavailable);
            }
            let reply_bytes = std::mem::size_of::<SourceContextReplyV1>()
                .checked_add(resolved.payload_bytes())
                .ok_or(Error::Limit)?;
            if reply_bytes > p.limits.max_reply_bytes {
                return Err(Error::Limit);
            }
            let content = match resolved {
                Resolved::Range(a, b) => SourceContextContentV1::Range(
                    SourceRangeReply::from_source(source, &row.version, a, b).map_err(
                        |e| match e {
                            SourceQueryError::Coordinate(CoordinateError::Capacity) => {
                                Error::Capacity
                            }
                            _ => Error::Faulted,
                        },
                    )?,
                ),
                Resolved::Line(index, _) => {
                    let span = SourceCoordinates::new(source)
                        .line_span(
                            index,
                            CoordinateBudget {
                                max_scan_bytes: p.limits.max_scan_bytes,
                            },
                        )
                        .map_err(coordinate)?;
                    SourceContextContentV1::Line(
                        SourceLineReply::from_source(source, &row.version, index, span).map_err(
                            |e| match e {
                                SourceQueryError::Coordinate(CoordinateError::Capacity) => {
                                    Error::Capacity
                                }
                                _ => Error::Faulted,
                            },
                        )?,
                    )
                }
            };
            let version = row.version.clone();
            #[allow(unused_mut)]
            let mut reply = SourceContextReplyV1 {
                context: id.clone(),
                content,
            };
            #[cfg(test)]
            {
                super::source_query::result_contract::corrupt(
                    &mut reply.content,
                    self.source_read_result_fault,
                );
                if matches!(
                    self.source_read_result_fault,
                    Some(super::source_query::result_contract::SourceResultFault::Alias)
                ) {
                    reply.context.slot = usize::MAX;
                }
            }
            self.guard_source_result(resolved, &version, &reply.content, reply.context == *id, r)?;
            Ok(reply)
        })();
        if matches!(result, Err(Error::Faulted)) {
            self.faulted = true;
        }
        observe(r, Boundary::SourceContextQuery, &result);
        result
    }
}
