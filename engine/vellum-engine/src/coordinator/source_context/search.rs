//! Shared source/combined/version-set literal matches. No raw legacy SearchResult escapes.
use super::super::search as kernel;
use super::*;
mod consumer;
mod issuer;
#[cfg(test)]
#[path = "search/pin_tests.rs"]
mod pin_tests;
mod shutdown_inventory;
use issuer::{Issuer, IssuerHold};
pub use issuer::{
    SearchReadContextV1, SearchReadFactsV1, SearchResultControlV1, TrustedVersionSetSearchFactsV1,
};
mod arguments;
mod binding;
pub use binding::*;
mod facade;
pub use arguments::*;
mod navigation;
pub(in crate::coordinator) use facade::navigation_signature;
pub(in crate::coordinator) mod result_contract;
pub(in crate::coordinator) use consumer::{ConsumerCapture, ConsumerPlan};
pub use consumer::{SourceSearchConsumerV1, TrustedSourceSearchConsumerFactsV1};
pub use navigation::{
    SourceSearchFoundV1, SourceSearchNavigationReplyV1, SourceSearchNavigationV1,
    SourceSearchOccurrenceId,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceSearchLimitsV1 {
    pub max_results: usize,
    pub max_issued: usize,
    pub max_request_bytes: usize,
    pub max_capture_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_pool_bytes: usize,
    pub max_query_bytes: usize,
    pub max_scope_ranges: usize,
    pub max_comparisons: usize,
    pub max_matches: usize,
    pub max_replacement_bytes: usize,
}
impl SourceSearchLimitsV1 {
    /// Native logical inline headers plus bounded payload/workspace; not RSS.
    pub fn required_pool_bytes(self) -> Result<usize, Error> {
        let mut n = std::mem::size_of::<SearchPool>();
        for (count, size) in [
            (self.max_issued, std::mem::size_of::<Option<ResultRow>>()),
            (2, self.max_request_bytes),
            (2, self.max_capture_bytes),
            (2, self.max_reply_bytes),
            (self.max_results, self.max_capture_bytes),
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
/// Host-policy facts separately attributed to this exact original request.
pub struct TrustedSourceSearchFactsV1<'a> {
    pub request: &'a SearchRequest,
    pub grant: Option<bool>,
    pub scope: Option<bool>,
    pub continuity: Option<bool>,
    pub capacity: Option<bool>,
    pub retained: Option<bool>,
    pub not_stopped: Option<bool>,
}
/// Owned disclosure: previously delivered ranges cannot be recalled.
pub struct SourceSearchReplyV1 {
    pub result: SourceSearchResultId,
    pub version: DocumentVersion,
    pub status: SearchStatus,
    pub matches: Vec<ByteRange>,
}
impl std::fmt::Debug for SourceSearchReplyV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceSearchReplyV1(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceSearchUsageV1 {
    pub results: usize,
    pub invalidated: usize,
    pub issued_lifetime: usize,
    pub reserved_pool_bytes: usize,
    pub closed: bool,
}
pub(super) struct ResultRow {
    source: Issuer,
    hold: IssuerHold,
    scope: SourceReadScopeV1,
    request: SearchRequest,
    version: DocumentVersion,
    matches: Vec<ByteRange>,
    status: SearchStatus,
    stopped: bool,
    // Set only after the actual producer reply passes the release guard. Native
    // logical frame units, not caller-supplied size, JSON bytes or a read grant.
    returned_bytes: Option<usize>,
}
pub(in crate::coordinator) struct SearchPool {
    authority: String,
    engine: String,
    limits: SourceSearchLimitsV1,
    reservation: retention::Reservation,
    charge: usize,
    issued: usize,
    // Exact gauges maintained at row install, first absorbing query loss and
    // protected row retirement. Never derive admission pressure by rescanning
    // nonrefunding lifetime tombstones on the hot path.
    live: usize,
    invalidated: usize,
    rows: Vec<Option<ResultRow>>,
}
impl SearchPool {
    fn validate_gauges(&self) -> Result<(), Error> {
        if self.invalidated > self.live
            || self.live > self.limits.max_results
            || self.live > self.rows.len()
            || self.rows.len() > self.issued
            || self.issued > self.limits.max_issued
        {
            Err(Error::Faulted)
        } else {
            Ok(())
        }
    }
}
#[derive(Default)]
pub(in crate::coordinator) enum State {
    #[default]
    Unconfigured,
    Active(Box<SearchPool>),
    Closed {
        issued: usize,
    },
}
impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ManagedSearchState(<redacted>)")
    }
}
impl State {
    pub(in crate::coordinator) fn document_pins(
        &self,
        document: &DocumentId,
    ) -> Result<usize, CoordinatorError> {
        let p = match self {
            Self::Unconfigured | Self::Closed { issued: _ } => return Ok(0),
            Self::Active(p) => p,
        };
        p.validate_gauges().map_err(|_| CoordinatorError::Faulted)?;
        if p.live != p.rows.iter().flatten().count()
            || p.invalidated != p.rows.iter().flatten().filter(|r| r.stopped).count()
        {
            return Err(CoordinatorError::Faulted);
        }
        if p.rows
            .iter()
            .flatten()
            .any(|r| r.scope.document != r.version.document)
        {
            return Err(CoordinatorError::Faulted);
        }
        Ok(p.rows
            .iter()
            .flatten()
            .filter(|r| r.scope.document == *document)
            .count())
    }
}
fn current(f: &TrustedSourceSearchFactsV1<'_>) -> bool {
    [
        f.grant,
        f.scope,
        f.continuity,
        f.capacity,
        f.retained,
        f.not_stopped,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn query_lost(f: &TrustedSourceSearchFactsV1<'_>) -> bool {
    [f.grant, f.scope, f.continuity, f.retained, f.not_stopped].contains(&Some(false))
}
fn search_error(e: SearchError) -> Error {
    match e {
        SearchError::Capacity => Error::Capacity,
        SearchError::Faulted => Error::Faulted,
        SearchError::Limit => Error::Limit,
        _ => Error::InvalidRequest,
    }
}
fn ranges(s: &SearchScope) -> &[ByteRange] {
    match s {
        SearchScope::Document => &[],
        SearchScope::Captured(v) => v,
    }
}
fn copy_ranges(v: &[ByteRange]) -> Result<Vec<ByteRange>, Error> {
    let mut out = Vec::new();
    out.try_reserve_exact(v.len())
        .map_err(|_| Error::Capacity)?;
    out.extend_from_slice(v);
    Ok(out)
}
pub(super) fn request_bytes(s: &SourceReadScopeV1, q: &SearchRequest) -> Result<usize, Error> {
    scope_bytes(s)?
        .checked_add(std::mem::size_of::<Issuer>())
        .and_then(|n| n.checked_add(std::mem::size_of::<SearchRequest>()))
        .and_then(|n| n.checked_add(q.query.len()))
        .and_then(|n| {
            n.checked_add(
                ranges(&q.scope)
                    .len()
                    .checked_mul(std::mem::size_of::<ByteRange>())?,
            )
        })
        .ok_or(Error::Limit)
}
fn control(s: &SourceReadScopeV1) -> SourceReadContextControlV1<'_> {
    SourceReadContextControlV1 {
        scope: s,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
impl SourceCoordinator {
    fn search_pool(&self) -> Result<&SearchPool, Error> {
        match &self.managed_search {
            State::Active(p) => Ok(p),
            _ => Err(Error::Unavailable),
        }
    }
    fn search_pool_mut(&mut self) -> Result<&mut SearchPool, Error> {
        match &mut self.managed_search {
            State::Active(p) => Ok(p),
            _ => Err(Error::Unavailable),
        }
    }
    fn search_row(&self, id: &SourceSearchResultId) -> Result<&ResultRow, Error> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return Err(Error::Unavailable);
        }
        self.search_pool()?
            .rows
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)
    }
    fn source_search_funding(&self) -> Result<(), Error> {
        self.retention
            .validate(&self.search_pool()?.reservation)
            .map_err(parent)
    }
    /// Called only after complete request/identity/shape preflight. Never refreshes.
    fn check_search_source(
        &mut self,
        id: &SourceReadContextId,
        s: &SourceReadScopeV1,
        f: &TrustedSourceReadFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        if s != f.scope {
            return Err(Error::Unavailable);
        }
        let row = self.source_context_row(id, s)?;
        if row.closed {
            return Err(Error::Unavailable);
        }
        if row.version.document != s.document || row.source.is_none() {
            return Err(Error::Faulted);
        }
        if self.faulted || self.document(&s.document).is_err() || lost(f) {
            let super::State::Active(p) = &mut self.source_contexts else {
                return Err(Error::Faulted);
            };
            p.close_context(id.slot)?;
            observe(r, Boundary::SourceContextInvalidation, &Ok::<_, Error>(()));
            return Err(Error::Unavailable);
        }
        self.source_search_funding()?;
        self.retention
            .validate(&self.source_pool()?.reservation)
            .map_err(parent)?;
        if !allowed(f) {
            return Err(Error::Unavailable);
        }
        Ok(())
    }
    pub fn configure_source_search(
        &mut self,
        limits: SourceSearchLimitsV1,
        f: &SourceReadPoolControlV1,
        r: &mut Recorder,
    ) -> Result<bool, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if self.faulted {
                return Err(Error::Faulted);
            }
            if f.owner_control_allowed != Some(true) || f.resources_ready != Some(true) {
                return Err(Error::Unavailable);
            }
            match &self.managed_search {
                State::Active(p) => {
                    self.retention.validate(&p.reservation).map_err(parent)?;
                    return if p.limits == limits {
                        Ok(false)
                    } else {
                        Err(Error::Unavailable)
                    };
                }
                State::Closed { .. } => return Err(Error::Unavailable),
                State::Unconfigured => {}
            }
            self.ensure_admission_unfenced()
                .map_err(super::admission_error)?;
            let (authority, engine) = match &self.source_contexts {
                super::State::Active(source) => {
                    self.retention
                        .validate(&source.reservation)
                        .map_err(parent)?;
                    (&source.authority[..], &source.engine[..])
                }
                super::State::Unconfigured | super::State::Closed { .. } => {
                    match &self.annotation_contexts {
                        annotation::projection::InstalledState::Active(_) => {
                            self.annotation_context_domain()?
                        }
                        _ => self.version_context_domain()?,
                    }
                }
            };
            let labels = authority
                .len()
                .checked_add(engine.len())
                .ok_or(Error::Limit)?;
            if std::mem::size_of::<SourceSearchLimitsV1>()
                .checked_add(labels)
                .is_none_or(|n| n > limits.max_request_bytes)
            {
                return Err(Error::Limit);
            }
            let charge = limits
                .required_pool_bytes()?
                .checked_add(labels)
                .filter(|n| *n <= limits.max_pool_bytes)
                .ok_or(Error::Limit)?;
            self.retention.check(charge).map_err(parent)?;
            let authority = owned(authority)?;
            let engine = owned(engine)?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(limits.max_issued)
                .map_err(|_| Error::Capacity)?;
            let reservation = self.retention.reserve(charge).map_err(parent)?;
            self.managed_search = State::Active(Box::new(SearchPool {
                authority,
                engine,
                limits,
                reservation,
                charge,
                issued: 0,
                live: 0,
                invalidated: 0,
                rows,
            }));
            Ok(true)
        })();
        self.finish_search(
            Boundary::SourceSearchConfiguration,
            &result,
            timing.finish(),
            r,
        );
        result
    }
    pub fn source_search_usage(&self) -> SourceSearchUsageV1 {
        match &self.managed_search {
            State::Unconfigured => SourceSearchUsageV1::default(),
            State::Closed { issued } => SourceSearchUsageV1 {
                closed: true,
                issued_lifetime: *issued,
                ..SourceSearchUsageV1::default()
            },
            State::Active(p) => SourceSearchUsageV1 {
                results: p.live,
                invalidated: p.invalidated,
                issued_lifetime: p.issued,
                reserved_pool_bytes: p.charge,
                closed: false,
            },
        }
    }
    fn preflight_source_search(
        &self,
        source: &Issuer,
        s: &SourceReadScopeV1,
        q: &SearchRequest,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
    ) -> Result<Vec<ByteRange>, Error> {
        let p = self.search_pool()?;
        if s != sf.scope() || q != f.request || !source.matches(sf) {
            return Err(Error::Unavailable);
        }
        self.check_search_request(source, s, q, sf, 0)?;
        let snapshot = self.search_snapshot(source, s)?;
        if q.query.len() > p.limits.max_query_bytes {
            return Err(Error::Limit);
        }
        let text = snapshot.source.text();
        kernel::scopes(text, &q.scope, p.limits.max_scope_ranges).map_err(search_error)
    }
    /// Retains the exact original context; returned identity discloses no ranges.
    #[allow(clippy::too_many_arguments)]
    pub fn query_source_search(
        &mut self,
        source: &SourceReadContextId,
        s: &SourceReadScopeV1,
        q: &SearchRequest,
        sf: &TrustedSourceReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<SourceSearchResultId> {
        self.query_managed_search(
            SearchReadContextV1::Source(source),
            s,
            q,
            SearchReadFactsV1::Source(sf),
            f,
            r,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn query_managed_search(
        &mut self,
        source: SearchReadContextV1<'_>,
        s: &SourceReadScopeV1,
        q: &SearchRequest,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<SourceSearchResultId> {
        self.query_managed_search_result(source, s, q, sf, f, r)
            .ok()
    }
    #[allow(clippy::too_many_arguments)]
    pub(in crate::coordinator) fn query_managed_search_result(
        &mut self,
        source: SearchReadContextV1<'_>,
        s: &SourceReadScopeV1,
        q: &SearchRequest,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourceSearchResultId, Error> {
        self.query_managed_search_result_with_overhead(source, s, q, sf, f, 0, r)
    }
    #[allow(clippy::too_many_arguments)]
    fn query_managed_search_result_with_overhead(
        &mut self,
        source: SearchReadContextV1<'_>,
        s: &SourceReadScopeV1,
        q: &SearchRequest,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        reply_overhead: usize,
        r: &mut Recorder,
    ) -> Result<SourceSearchResultId, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let source = self.own_search_issuer(source)?;
            let scopes = self.preflight_source_search(&source, s, q, sf, f)?;
            self.check_search_issuer(&source, s, sf, r)?;
            if !current(f) {
                return Err(Error::Unavailable);
            }
            self.ensure_admission_unfenced()
                .map_err(super::admission_error)?;
            let p = self.search_pool()?;
            let limits = p.limits;
            p.validate_gauges()?;
            if p.issued >= limits.max_issued || p.live >= limits.max_results {
                return Err(Error::Limit);
            }
            let next_live = p.live.checked_add(1).ok_or(Error::Faulted)?;
            let snapshot = self.search_snapshot(&source, s)?;
            let text = snapshot.source.text();
            let scope_payload = scope_bytes(s)? - std::mem::size_of::<SourceReadScopeV1>();
            let base_capture = std::mem::size_of::<ResultRow>()
                .checked_add(scope_payload)
                .and_then(|n| n.checked_add(q.query.len()))
                .and_then(|n| {
                    n.checked_add(
                        ranges(&q.scope)
                            .len()
                            .checked_mul(std::mem::size_of::<ByteRange>())?,
                    )
                })
                .ok_or(Error::Limit)?;
            let reply_max = limits.max_reply_bytes.min(snapshot.max_reply_bytes);
            if base_capture > limits.max_capture_bytes
                || std::mem::size_of::<SourceSearchReplyV1>()
                    .checked_add(reply_overhead)
                    .is_none_or(|n| n > reply_max)
                || std::mem::size_of::<SourceSearchResultId>() > reply_max
            {
                return Err(Error::Limit);
            }
            let version = snapshot.version.clone();
            let context = settings::basis::copy_context(&s.context).map_err(|_| Error::Capacity)?;
            let owned_scope = SourceReadScopeV1 {
                document: s.document.clone(),
                context,
                policy: owned(&s.policy)?,
            };
            let request = SearchRequest {
                query: owned(&q.query)?,
                scope: match &q.scope {
                    SearchScope::Document => SearchScope::Document,
                    SearchScope::Captured(v) => SearchScope::Captured(copy_ranges(v)?),
                },
            };
            // Work vector has its own max-capture workspace before hold/evaluation.
            if scopes
                .len()
                .checked_mul(std::mem::size_of::<ByteRange>())
                .is_none_or(|n| n > limits.max_capture_bytes)
            {
                return Err(Error::Limit);
            }
            let root_bound = snapshot.max_root_bytes;
            let max_match_bytes = (limits.max_capture_bytes - base_capture)
                .min(reply_max - std::mem::size_of::<SourceSearchReplyV1>() - reply_overhead);
            if text.len() > root_bound {
                return Err(Error::Limit);
            }
            let hold = self.hold_search_issuer(&source, s, sf, r)?;
            // Evaluation issuance is spent even if no publishable result is made.
            self.search_pool_mut()?.issued += 1;
            let evaluated = (|| {
                let snapshot = self.search_snapshot(&source, s)?;
                let text = snapshot.source.text();
                kernel::evaluate(
                    text,
                    &request.query,
                    &scopes,
                    SearchLimits {
                        max_sessions_issued: 0,
                        max_live_sessions: 0,
                        max_live_results: 0,
                        max_root_bytes: root_bound,
                        max_scope_ranges: limits.max_scope_ranges,
                        max_query_bytes: limits.max_query_bytes,
                        max_comparisons: limits.max_comparisons,
                        max_matches: limits.max_matches,
                        max_match_bytes,
                        max_replacement_bytes: 0,
                    },
                )
                .map_err(search_error)
            })();
            #[allow(unused_mut)]
            let (mut matches, mut status) = match evaluated {
                Ok(v) => v,
                Err(e) => {
                    self.discard_unpublished_search_hold(&hold, s, r)?;
                    return Err(e);
                }
            };
            #[cfg(test)]
            match self.search_result_fault {
                Some(result_contract::Fault::ProducerRange) => {
                    if let Some(m) = matches.first_mut() {
                        m.end = m.end.saturating_add(1);
                    }
                }
                Some(result_contract::Fault::ProducerOrder) => {
                    if let Some(m) = matches.first().copied() {
                        matches.push(m);
                    }
                }
                Some(result_contract::Fault::ProducerStatus) => {
                    status = SearchStatus::Exhausted(SearchBudget::Matches)
                }
                _ => {}
            }
            let snapshot = self.search_snapshot(&source, s)?;
            let valid = result_contract::candidate_valid(
                snapshot.source.text(),
                &request.query,
                &scopes,
                &matches,
                status,
                limits.max_matches,
                max_match_bytes,
            );
            if self
                .guard_search_product(valid, Boundary::SourceSearchResultValidation, r)
                .is_err()
            {
                // Already-issued evaluation stays spent. Only this unpublished
                // original hold is discharged; no result row is installed.
                self.discard_unpublished_search_hold(&hold, s, r)?;
                return Err(Error::Faulted);
            }
            let id = SourceSearchResultId {
                owner: self.namespace.clone(),
                slot: self.search_pool()?.rows.len(),
            };
            let p = self.search_pool_mut()?;
            p.rows.push(Some(ResultRow {
                source: source.clone(),
                hold,
                scope: owned_scope,
                request,
                version,
                matches,
                status,
                stopped: false,
                returned_bytes: None,
            }));
            p.live = next_live;
            Ok(id)
        })();
        if result.as_ref().is_ok_and(|id| {
            self.search_row(id)
                .is_ok_and(|row| row.status != SearchStatus::Complete)
        }) {
            r.record(Observation {
                boundary: Boundary::SourceSearchEvaluation,
                disposition: Disposition::Partial,
                service_time: timing.finish(),
            });
        } else {
            self.finish_search(
                Boundary::SourceSearchEvaluation,
                &result,
                timing.finish(),
                r,
            );
        }
        result
    }
    /// Fresh release is bound to original request and complete source scope.
    pub fn release_source_search(
        &mut self,
        id: &SourceSearchResultId,
        s: &SourceReadScopeV1,
        sf: &TrustedSourceReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<SourceSearchReplyV1> {
        self.release_managed_search(id, s, SearchReadFactsV1::Source(sf), f, r)
    }
    pub fn release_managed_search(
        &mut self,
        id: &SourceSearchResultId,
        s: &SourceReadScopeV1,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<SourceSearchReplyV1> {
        self.release_managed_search_result(id, s, sf, f, r).ok()
    }
    pub(in crate::coordinator) fn release_managed_search_result(
        &mut self,
        id: &SourceSearchResultId,
        s: &SourceReadScopeV1,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourceSearchReplyV1, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.check_source_search_release(id, s, sf, f, r)?;
            let row = self.search_row(id)?;
            #[allow(unused_mut)]
            let mut reply = SourceSearchReplyV1 {
                result: id.clone(),
                version: row.version.clone(),
                status: row.status,
                matches: copy_ranges(&row.matches)?,
            };
            #[cfg(test)]
            match self.search_result_fault {
                Some(result_contract::Fault::ReleaseAlias) => {
                    reply.result.slot = reply.result.slot.saturating_add(1)
                }
                Some(result_contract::Fault::ReleaseVersion) => {
                    reply.version.revision = reply.version.revision.saturating_add(1)
                }
                Some(result_contract::Fault::ReleaseRanges) => {
                    reply.matches.pop();
                }
                Some(result_contract::Fault::ReleaseStatus) => {
                    reply.status = SearchStatus::Exhausted(SearchBudget::Comparisons)
                }
                _ => {}
            }
            let valid = result_contract::release_valid(self.search_row(id)?, id, &reply);
            self.guard_search_product(valid, Boundary::SourceSearchResultValidation, r)?;
            let bytes = reply
                .matches
                .len()
                .checked_mul(std::mem::size_of::<ByteRange>())
                .and_then(|n| n.checked_add(std::mem::size_of::<SourceSearchReplyV1>()))
                .ok_or(Error::Limit)?;
            self.search_pool_mut()?.rows[id.slot]
                .as_mut()
                .ok_or(Error::Faulted)?
                .returned_bytes = Some(bytes);
            Ok(reply)
        })();
        self.finish_search(Boundary::SourceSearchRelease, &result, timing.finish(), r);
        result
    }
    pub(in crate::coordinator) fn returned_search_frame_bytes(
        &self,
        id: &SourceSearchResultId,
    ) -> Result<usize, Error> {
        self.search_row(id)?
            .returned_bytes
            .ok_or(Error::Unavailable)
    }
    fn check_source_search_release(
        &mut self,
        id: &SourceSearchResultId,
        s: &SourceReadScopeV1,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let row = self.search_row(id)?;
        if row.scope != *s || row.request != *f.request || row.stopped {
            return Err(Error::Unavailable);
        }
        let source = row.source.clone();
        self.preflight_source_search(&source, s, f.request, sf, f)?;
        // Query loss is independent from source read loss and absorbs even
        // when source delivery is temporarily resource-unavailable.
        let source_ready = self.check_search_issuer(&source, s, sf, r);
        if query_lost(f) {
            let p = self.search_pool_mut()?;
            p.validate_gauges()?;
            let next_invalidated = p
                .invalidated
                .checked_add(1)
                .filter(|n| *n <= p.live)
                .ok_or(Error::Faulted)?;
            p.rows[id.slot].as_mut().ok_or(Error::Faulted)?.stopped = true;
            p.invalidated = next_invalidated;
            observe(r, Boundary::SourceSearchInvalidation, &Ok::<_, Error>(()));
        }
        source_ready?;
        if !current(f) || self.search_row(id)?.stopped {
            return Err(Error::Unavailable);
        }
        let row = self.search_row(id)?;
        self.validate_search_issuer_hold(&source, &row.hold, s)?;
        Ok(())
    }
    pub fn retire_source_search_result(
        &mut self,
        id: &SourceSearchResultId,
        f: &SourceReadContextControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        self.retire_managed_search_result(id, SearchResultControlV1::Single(f), r)
    }
    pub fn retire_managed_search_result(
        &mut self,
        id: &SourceSearchResultId,
        f: SearchResultControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let timing = r.begin_service_timing();
        let result = (|| {
            let row = self.search_row(id)?;
            match f {
                SearchResultControlV1::Single(f)
                    if super::control::discharge(f)
                        && row.scope == *f.scope
                        && !matches!(row.source, Issuer::VersionSet { .. }) => {}
                SearchResultControlV1::VersionSet(_)
                    if matches!(row.source, Issuer::VersionSet { .. }) => {}
                _ => return Err(Error::Unavailable),
            }
            let hold = row.hold.clone();
            let p = self.search_pool()?;
            p.validate_gauges()?;
            let next_live = p.live.checked_sub(1).ok_or(Error::Faulted)?;
            let next_invalidated = p
                .invalidated
                .checked_sub(usize::from(row.stopped))
                .filter(|n| *n <= next_live)
                .ok_or(Error::Faulted)?;
            self.validate_search_issuer_hold(&row.source, &hold, &row.scope)?;
            self.source_search_funding()?;
            self.release_search_issuer_hold(&hold, f, r)?;
            let p = self.search_pool_mut()?;
            p.rows[id.slot] = None;
            p.live = next_live;
            p.invalidated = next_invalidated;
            Ok(())
        })();
        self.finish_search(
            Boundary::SourceSearchRetirement,
            &result,
            timing.finish(),
            r,
        );
        result.is_ok()
    }
    pub fn retire_source_search(
        &mut self,
        f: &SourceReadPoolControlV1,
        r: &mut Recorder,
    ) -> Result<bool, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if [
                f.owner_control_allowed,
                f.retention_released,
                f.delivery_drained,
            ]
            .into_iter()
            .any(|v| v != Some(true))
            {
                return Err(Error::Unavailable);
            }
            let State::Active(p) = &mut self.managed_search else {
                return Ok(false);
            };
            // Protected cold-path reconciliation retains the actual inventory
            // check; cached gauges cannot authorize removal of retained rows.
            p.validate_gauges()?;
            if p.live != p.rows.iter().flatten().count()
                || p.invalidated != p.rows.iter().flatten().filter(|row| row.stopped).count()
            {
                return Err(Error::Faulted);
            }
            if p.live != 0 {
                return Err(Error::Unavailable);
            }
            self.retention.release(&mut p.reservation).map_err(parent)?;
            self.managed_search = State::Closed { issued: p.issued };
            Ok(true)
        })();
        self.finish_search(
            Boundary::SourceSearchPoolRetirement,
            &result,
            timing.finish(),
            r,
        );
        result
    }
    fn finish_search<T>(
        &mut self,
        boundary: Boundary,
        result: &Result<T, Error>,
        service_time: Option<std::time::Duration>,
        r: &mut Recorder,
    ) {
        if matches!(result, Err(Error::Faulted)) {
            self.faulted = true;
        }
        r.record(Observation {
            boundary,
            disposition: match result {
                Ok(_) => Disposition::Completed,
                Err(Error::Faulted) => Disposition::Unknown,
                Err(_) => Disposition::Refused,
            },
            service_time,
        });
    }
}
