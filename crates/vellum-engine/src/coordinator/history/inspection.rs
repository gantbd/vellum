//! Trusted native F04 owner. No wire token, UI policy or reversal capability.
use super::*;
mod shutdown_inventory;
use std::mem::size_of;
#[cfg(test)]
mod tests;

#[derive(Clone, PartialEq, Eq)]
pub struct HistoryInspectionScopeV1 {
    pub context: SettingsExecutionContextV1,
    pub policy: String,
}
impl std::fmt::Debug for HistoryInspectionScopeV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HistoryInspectionScopeV1(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistoryInspectionLimitsV1 {
    pub max_contexts: usize,
    pub max_issued: usize,
    pub basis: HistoryBasisLimitsV1,
    pub max_details: usize,
    pub max_capture_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_held_bytes: usize,
}
impl HistoryInspectionLimitsV1 {
    /// Native logical records/workspace, not JSON bytes or allocator capacity.
    /// Scope label bytes are added at configuration.
    pub fn required_pool_bytes(self) -> Result<usize, HistoryError> {
        let unit = self
            .max_capture_bytes
            .checked_add(self.basis.max_basis_bytes)
            .ok_or(HistoryError::Limit)?;
        let slots = self
            .max_contexts
            .checked_add(1)
            .ok_or(HistoryError::Limit)?;
        [
            (slots, unit),
            (2, self.max_reply_bytes),
            (self.max_issued, size_of::<Option<Entry>>()),
        ]
        .into_iter()
        .try_fold(size_of::<Pool>(), |n, (count, bytes)| {
            n.checked_add(count.checked_mul(bytes).ok_or(HistoryError::Limit)?)
                .ok_or(HistoryError::Limit)
        })?
        .checked_add(size_of::<HistoryInspectionScopeV1>())
        .filter(|n| *n <= self.max_held_bytes)
        .ok_or(HistoryError::Limit)
    }
}
pub struct HistoryScopeProjectionV1 {
    pub scope: HistoryBasisScopeV1,
    pub alias: String,
    pub undo_complete: bool,
    pub redo_complete: bool,
}
pub struct HistoryGroupProjectionV1 {
    pub group: HistoryGroupId,
    pub alias: String,
    pub evidence_ready: bool,
}
pub struct HistoryProjectionV1 {
    pub details: bool,
    pub scopes: Vec<HistoryScopeProjectionV1>,
    pub groups: Vec<HistoryGroupProjectionV1>,
}
/// Separately attributed current host facts. Alias/coverage/evidence truth is a
/// host premise; native capture equality cannot establish any of these grants.
pub struct TrustedHistoryInspectionFactsV1<'a> {
    pub scope: &'a HistoryInspectionScopeV1,
    pub granted: Option<bool>,
    pub retained: Option<bool>,
    pub live: Option<bool>,
    pub continuous: Option<bool>,
    pub transfer_ready: Option<bool>,
    pub history_current: Option<bool>,
    pub capture_complete: Option<bool>,
    pub preparation_reserved: Option<bool>,
    pub registry_ready: Option<bool>,
    pub retention_ready: Option<bool>,
    pub not_stopped: Option<bool>,
    pub reply_ready: Option<bool>,
}
pub struct HistoryInspectionControlV1<'a> {
    pub scope: &'a HistoryInspectionScopeV1,
    pub owner_control_allowed: Option<bool>,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
}
#[derive(Clone)]
pub struct HistoryInspectionIdV1 {
    owner: Arc<()>,
    slot: usize,
}
impl PartialEq for HistoryInspectionIdV1 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for HistoryInspectionIdV1 {}
impl std::fmt::Debug for HistoryInspectionIdV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HistoryInspectionIdV1(<opaque>)")
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HistoryInspectionQueryV1<'a> {
    Frontier {
        scope: &'a str,
        direction: HistoryDirection,
    },
    Availability {
        scope: &'a str,
        direction: HistoryDirection,
        group: Option<&'a str>,
    },
    PermittedBlockingDetail {
        scope: &'a str,
        direction: HistoryDirection,
        group: Option<&'a str>,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryInspectionAvailabilityV1 {
    Empty,
    Eligible,
    Blocked,
}
#[derive(Debug, PartialEq, Eq)]
pub struct HistoryBlockerV1 {
    pub scope: String,
    pub group: String,
}
#[derive(Debug, PartialEq, Eq)]
pub enum HistoryInspectionReplyV1 {
    Frontier {
        context: HistoryInspectionIdV1,
        group: Option<String>,
    },
    Availability {
        context: HistoryInspectionIdV1,
        value: HistoryInspectionAvailabilityV1,
    },
    PermittedBlockingDetail {
        context: HistoryInspectionIdV1,
        blockers: Vec<HistoryBlockerV1>,
    },
    Unavailable,
}
/// Trusted-owner gauges, not a permission-filtered recipient response.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HistoryInspectionUsageV1 {
    pub contexts: usize,
    pub live: usize,
    pub issued_lifetime: usize,
    pub reserved_pool_bytes: usize,
    pub stopped: bool,
    pub closed: bool,
}
struct Entry {
    basis: HistoryBasisV1,
    projection: HistoryProjectionV1,
    live: bool,
}
pub(in crate::coordinator) struct Pool {
    scope: HistoryInspectionScopeV1,
    limits: HistoryInspectionLimitsV1,
    rows: Vec<Option<Entry>>,
    reservation: retention::Reservation,
    charge: usize,
    stopped: bool,
    #[cfg(test)]
    result_fault: Option<ResultFault>,
}
#[cfg(test)]
#[derive(Clone, Copy)]
enum ResultFault {
    Context,
    Variant,
    Alias,
    Oversized,
}
#[derive(Default)]
pub(in crate::coordinator) enum State {
    #[default]
    Unconfigured,
    Active(Box<Pool>),
    Closed {
        issued: usize,
    },
}
impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HistoryInspectionState(<redacted>)")
    }
}
impl State {
    pub(in crate::coordinator) fn document_pins(&self, document: &DocumentId) -> usize {
        match self {
            Self::Unconfigured | Self::Closed { .. } => 0,
            Self::Active(p) => p
                .rows
                .iter()
                .flatten()
                .filter(|r| r.basis.refers_to_document(document))
                .count(),
        }
    }
}
fn parent(e: RetentionErrorV1) -> HistoryError {
    match e {
        RetentionErrorV1::Unavailable => HistoryError::Unavailable,
        RetentionErrorV1::Limit => HistoryError::Limit,
        RetentionErrorV1::Faulted => HistoryError::Faulted,
    }
}
fn labels(s: &HistoryInspectionScopeV1) -> impl Iterator<Item = &str> {
    settings::basis::labels(&s.context)
        .into_iter()
        .chain(std::iter::once(s.policy.as_str()))
}
pub(in crate::coordinator) fn scope_bytes(
    s: &HistoryInspectionScopeV1,
) -> Result<usize, HistoryError> {
    labels(s).try_fold(size_of::<HistoryInspectionScopeV1>(), |n, s| {
        n.checked_add(s.len()).ok_or(HistoryError::Limit)
    })
}
fn owned(s: &str) -> Result<String, HistoryError> {
    let mut out = String::new();
    out.try_reserve_exact(s.len())
        .map_err(|_| HistoryError::Capacity)?;
    out.push_str(s);
    Ok(out)
}
pub(in crate::coordinator) fn copy_scope(
    s: &HistoryInspectionScopeV1,
) -> Result<HistoryInspectionScopeV1, HistoryError> {
    let c = &s.context;
    Ok(HistoryInspectionScopeV1 {
        context: SettingsExecutionContextV1 {
            authority: owned(&c.authority)?,
            engine: owned(&c.engine)?,
            recipient: owned(&c.recipient)?,
            purpose: owned(&c.purpose)?,
            destination: owned(&c.destination)?,
            profile: owned(&c.profile)?,
            continuity: owned(&c.continuity)?,
        },
        policy: owned(&s.policy)?,
    })
}
fn acquisition(f: &TrustedHistoryInspectionFactsV1<'_>) -> bool {
    [
        f.granted,
        f.retained,
        f.live,
        f.continuous,
        f.transfer_ready,
        f.history_current,
        f.capture_complete,
        f.preparation_reserved,
        f.registry_ready,
        f.retention_ready,
        f.not_stopped,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn release(f: &TrustedHistoryInspectionFactsV1<'_>) -> bool {
    [
        f.granted,
        f.retained,
        f.live,
        f.continuous,
        f.transfer_ready,
        f.history_current,
        f.reply_ready,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn lost(f: &TrustedHistoryInspectionFactsV1<'_>) -> bool {
    [
        f.granted,
        f.retained,
        f.live,
        f.continuous,
        f.history_current,
    ]
    .contains(&Some(false))
}
fn observe<T>(r: &mut Recorder, b: Boundary, out: &Result<T, HistoryError>) {
    r.record(Observation {
        boundary: b,
        disposition: match out {
            Ok(_) => Disposition::Completed,
            Err(HistoryError::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time: None,
    });
}
fn projection_bytes(p: &HistoryProjectionV1) -> Result<usize, HistoryError> {
    let n = p
        .scopes
        .len()
        .checked_mul(size_of::<HistoryScopeProjectionV1>())
        .and_then(|n| {
            n.checked_add(
                p.groups
                    .len()
                    .checked_mul(size_of::<HistoryGroupProjectionV1>())?,
            )
        })
        .and_then(|n| n.checked_add(size_of::<HistoryProjectionV1>()))
        .ok_or(HistoryError::Limit)?;
    p.scopes
        .iter()
        .map(|s| s.alias.as_str())
        .chain(p.groups.iter().map(|g| g.alias.as_str()))
        .try_fold(n, |n, s| {
            if s.is_empty() {
                Err(HistoryError::Unavailable)
            } else {
                n.checked_add(s.len()).ok_or(HistoryError::Limit)
            }
        })
}
fn copy_projection(p: &HistoryProjectionV1) -> Result<HistoryProjectionV1, HistoryError> {
    let mut scopes = Vec::new();
    let mut groups = Vec::new();
    scopes
        .try_reserve_exact(p.scopes.len())
        .map_err(|_| HistoryError::Capacity)?;
    groups
        .try_reserve_exact(p.groups.len())
        .map_err(|_| HistoryError::Capacity)?;
    for s in &p.scopes {
        scopes.push(HistoryScopeProjectionV1 {
            scope: s.scope.clone(),
            alias: owned(&s.alias)?,
            undo_complete: s.undo_complete,
            redo_complete: s.redo_complete,
        });
    }
    for g in &p.groups {
        groups.push(HistoryGroupProjectionV1 {
            group: g.group.clone(),
            alias: owned(&g.alias)?,
            evidence_ready: g.evidence_ready,
        });
    }
    Ok(HistoryProjectionV1 {
        details: p.details,
        scopes,
        groups,
    })
}
// Independent retained binding/shape check, not a second history/evidence oracle.
fn valid_reply(
    id: &HistoryInspectionIdV1,
    q: HistoryInspectionQueryV1<'_>,
    e: &Entry,
    l: HistoryInspectionLimitsV1,
    out: &HistoryInspectionReplyV1,
) -> bool {
    let mut bytes = Some(size_of::<HistoryInspectionReplyV1>());
    let shape = match (q, out) {
        (
            HistoryInspectionQueryV1::Frontier { .. },
            HistoryInspectionReplyV1::Frontier { context, group },
        ) => {
            context == id
                && group.as_ref().is_none_or(|alias| {
                    bytes = bytes.and_then(|n| n.checked_add(alias.len()));
                    !alias.is_empty() && e.projection.groups.iter().any(|g| g.alias == *alias)
                })
        }
        (
            HistoryInspectionQueryV1::Availability { .. },
            HistoryInspectionReplyV1::Availability { context, .. },
        ) => context == id,
        (
            HistoryInspectionQueryV1::PermittedBlockingDetail { .. },
            HistoryInspectionReplyV1::PermittedBlockingDetail { context, blockers },
        ) => {
            context == id
                && e.projection.details
                && blockers.len() <= l.max_details
                && blockers.iter().enumerate().all(|(i, b)| {
                    bytes = bytes.and_then(|n| {
                        n.checked_add(size_of::<HistoryBlockerV1>())?
                            .checked_add(b.scope.len())?
                            .checked_add(b.group.len())
                    });
                    !b.scope.is_empty()
                        && !b.group.is_empty()
                        && e.projection.scopes.iter().any(|s| s.alias == b.scope)
                        && e.projection.groups.iter().any(|g| g.alias == b.group)
                        && (i == 0
                            || blockers[i - 1]
                                .scope
                                .encode_utf16()
                                .cmp(b.scope.encode_utf16())
                                .is_lt())
                })
        }
        _ => false,
    };
    shape && bytes.is_some_and(|n| n <= l.max_reply_bytes)
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn history_facade_headroom(&self) -> Result<usize, HistoryError> {
        self.ensure_operational().map_err(owner_error)?;
        let p = self.history_inspection_pool()?;
        if p.stopped {
            return Err(HistoryError::Unavailable);
        }
        self.retention.validate(&p.reservation).map_err(parent)?;
        p.limits
            .max_held_bytes
            .checked_sub(p.charge)
            .ok_or(HistoryError::Faulted)
    }
    pub(in crate::coordinator) fn history_facade_scope_matches(
        &self,
        s: &HistoryInspectionScopeV1,
    ) -> bool {
        self.history_inspection_pool().is_ok_and(|p| p.scope == *s)
    }
    pub(in crate::coordinator) fn history_facade_result_valid(
        &self,
        id: &HistoryInspectionIdV1,
        s: &HistoryInspectionScopeV1,
        q: HistoryInspectionQueryV1<'_>,
        out: &HistoryInspectionReplyV1,
    ) -> bool {
        self.history_inspection_pool().is_ok_and(|p| {
            self.history_inspection_entry(id, s)
                .is_ok_and(|e| valid_reply(id, q, e, p.limits, out))
        })
    }
    fn history_inspection_pool(&self) -> Result<&Pool, HistoryError> {
        match &self.history_inspection {
            State::Active(p) => Ok(p),
            _ => Err(HistoryError::Unavailable),
        }
    }
    fn history_inspection_entry(
        &self,
        id: &HistoryInspectionIdV1,
        s: &HistoryInspectionScopeV1,
    ) -> Result<&Entry, HistoryError> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return Err(HistoryError::Unavailable);
        }
        let p = self.history_inspection_pool()?;
        if p.scope != *s {
            return Err(HistoryError::Unavailable);
        }
        p.rows
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(HistoryError::Unavailable)
    }
    fn finish_history_inspection<T>(
        &mut self,
        b: Boundary,
        out: Result<T, HistoryError>,
        r: &mut Recorder,
    ) -> Result<T, HistoryError> {
        if matches!(out, Err(HistoryError::Faulted)) {
            self.faulted = true;
        }
        observe(r, b, &out);
        out
    }
    /// Trusted owner configuration; not a public F04 acquisition variant.
    pub fn configure_history_inspection_v1(
        &mut self,
        s: &HistoryInspectionScopeV1,
        l: HistoryInspectionLimitsV1,
        owner_control_allowed: Option<bool>,
        r: &mut Recorder,
    ) -> Result<bool, HistoryError> {
        let out = (|| {
            self.ensure_operational().map_err(owner_error)?;
            if owner_control_allowed != Some(true) {
                return Err(HistoryError::Unavailable);
            }
            if labels(s).any(str::is_empty) {
                return Err(HistoryError::Unavailable);
            }
            match &self.history_inspection {
                State::Active(p) => {
                    self.retention.validate(&p.reservation).map_err(parent)?;
                    return if p.scope == *s && p.limits == l {
                        Ok(false)
                    } else {
                        Err(HistoryError::Unavailable)
                    };
                }
                State::Closed { .. } => return Err(HistoryError::Unavailable),
                State::Unconfigured => {}
            }
            let scope_charge = scope_bytes(s)?;
            if scope_charge > l.max_capture_bytes
                || size_of::<HistoryInspectionReplyV1>() > l.max_reply_bytes
            {
                return Err(HistoryError::Limit);
            }
            let charge = l
                .required_pool_bytes()?
                .checked_add(scope_charge - size_of::<HistoryInspectionScopeV1>())
                .filter(|n| *n <= l.max_held_bytes)
                .ok_or(HistoryError::Limit)?;
            self.retention.check(charge).map_err(parent)?;
            let scope = copy_scope(s)?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(l.max_issued)
                .map_err(|_| HistoryError::Capacity)?;
            let reservation = self.retention.reserve(charge).map_err(parent)?;
            self.history_inspection = State::Active(Box::new(Pool {
                scope,
                limits: l,
                rows,
                reservation,
                charge,
                stopped: false,
                #[cfg(test)]
                result_fault: None,
            }));
            Ok(true)
        })();
        self.finish_history_inspection(Boundary::HistoryInspectionConfiguration, out, r)
    }
    pub fn history_inspection_usage_v1(&self) -> HistoryInspectionUsageV1 {
        match &self.history_inspection {
            State::Unconfigured => HistoryInspectionUsageV1::default(),
            State::Closed { issued } => HistoryInspectionUsageV1 {
                issued_lifetime: *issued,
                closed: true,
                stopped: true,
                ..Default::default()
            },
            State::Active(p) => HistoryInspectionUsageV1 {
                contexts: p.rows.iter().flatten().count(),
                live: p.rows.iter().flatten().filter(|e| e.live).count(),
                issued_lifetime: p.rows.len(),
                reserved_pool_bytes: p.charge,
                stopped: p.stopped,
                closed: false,
            },
        }
    }
    pub fn acquire_history_inspection_v1(
        &mut self,
        projection: &HistoryProjectionV1,
        f: &TrustedHistoryInspectionFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<HistoryInspectionIdV1, HistoryError> {
        let out = (|| {
            self.ensure_operational().map_err(owner_error)?;
            let p = self.history_inspection_pool()?;
            if p.stopped || p.scope != *f.scope || !acquisition(f) {
                return Err(HistoryError::Unavailable);
            }
            self.retention.validate(&p.reservation).map_err(parent)?;
            let l = p.limits;
            if p.rows.len() >= l.max_issued || p.rows.iter().flatten().count() >= l.max_contexts {
                return Err(HistoryError::Limit);
            }
            if projection.scopes.len() > l.basis.max_scopes
                || projection.groups.len() > l.basis.max_groups
                || projection_bytes(projection)? > l.max_capture_bytes
            {
                return Err(HistoryError::Limit);
            }
            for (i, s) in projection.scopes.iter().enumerate() {
                if !s.scope.owned(self)
                    || projection.scopes[..i]
                        .iter()
                        .any(|old| old.scope == s.scope || old.alias == s.alias)
                {
                    return Err(HistoryError::Unavailable);
                }
            }
            for (i, g) in projection.groups.iter().enumerate() {
                if projection.groups[..i]
                    .iter()
                    .any(|old| old.group == g.group || old.alias == g.alias)
                {
                    return Err(HistoryError::Unavailable);
                }
            }
            // Temporary lists and the resulting projection share the capture envelope.
            let temp = projection
                .groups
                .len()
                .checked_mul(size_of::<HistoryGroupId>())
                .and_then(|n| {
                    n.checked_add(
                        projection
                            .scopes
                            .len()
                            .checked_mul(size_of::<HistoryBasisScopeV1>())?,
                    )
                })
                .and_then(|n| n.checked_add(projection_bytes(projection).ok()?))
                .and_then(|n| {
                    n.checked_add(
                        size_of::<Vec<HistoryGroupId>>() + size_of::<Vec<HistoryBasisScopeV1>>(),
                    )
                })
                .ok_or(HistoryError::Limit)?;
            if temp > l.max_capture_bytes {
                return Err(HistoryError::Limit);
            }
            let mut groups = Vec::new();
            let mut scopes = Vec::new();
            groups
                .try_reserve_exact(projection.groups.len())
                .map_err(|_| HistoryError::Capacity)?;
            scopes
                .try_reserve_exact(projection.scopes.len())
                .map_err(|_| HistoryError::Capacity)?;
            groups.extend(projection.groups.iter().map(|g| g.group.clone()));
            scopes.extend(projection.scopes.iter().map(|s| s.scope.clone()));
            let basis = self.capture_history_basis_v1(&groups, &scopes, l.basis, r)?;
            let projection = copy_projection(projection)?;
            let State::Active(p) = &mut self.history_inspection else {
                return Err(HistoryError::Faulted);
            };
            let id = HistoryInspectionIdV1 {
                owner: self.namespace.clone(),
                slot: p.rows.len(),
            };
            p.rows.push(Some(Entry {
                basis,
                projection,
                live: true,
            }));
            Ok(id)
        })();
        self.finish_history_inspection(Boundary::HistoryInspectionAcquisition, out, r)
    }
    fn invalidate_history_inspection(&mut self, id: &HistoryInspectionIdV1, r: &mut Recorder) {
        if let State::Active(p) = &mut self.history_inspection
            && let Some(Some(e)) = p.rows.get_mut(id.slot)
        {
            e.live = false;
        }
        observe(r, Boundary::HistoryInspectionInvalidation, &Ok(()));
    }
    fn current_history_inspection(
        &mut self,
        id: &HistoryInspectionIdV1,
        f: &TrustedHistoryInspectionFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), HistoryError> {
        self.ensure_operational().map_err(owner_error)?;
        let e = self.history_inspection_entry(id, f.scope)?;
        let p = self.history_inspection_pool()?;
        if !e.live || p.stopped {
            return Err(HistoryError::Unavailable);
        }
        self.retention.validate(&p.reservation).map_err(parent)?;
        if lost(f) {
            self.invalidate_history_inspection(id, r);
            return Err(HistoryError::Unavailable);
        }
        if !release(f) {
            return Err(HistoryError::Unavailable);
        }
        let comparison = e.basis.compare(self);
        // False is stale, not successful validation; limits also invalidate under
        // the declared metadata-pressure allowance. Unknown grants do not.
        observe(
            r,
            Boundary::HistoryInspectionValidation,
            &if comparison == Ok(true) {
                Ok(())
            } else {
                Err(comparison.err().unwrap_or(HistoryError::Unavailable))
            },
        );
        match comparison {
            Ok(true) => Ok(()),
            Err(HistoryError::Faulted) => Err(HistoryError::Faulted),
            _ => {
                self.invalidate_history_inspection(id, r);
                Err(HistoryError::Unavailable)
            }
        }
    }
    pub fn query_history_inspection_v1(
        &mut self,
        id: &HistoryInspectionIdV1,
        q: HistoryInspectionQueryV1<'_>,
        f: &TrustedHistoryInspectionFactsV1<'_>,
        r: &mut Recorder,
    ) -> HistoryInspectionReplyV1 {
        let out = (|| {
            // Complete semantic envelope/aliases before applying any bound loss.
            let e = self.history_inspection_entry(id, f.scope)?;
            let (scope, direction, group) = match q {
                HistoryInspectionQueryV1::Frontier { scope, direction } => (scope, direction, None),
                HistoryInspectionQueryV1::Availability {
                    scope,
                    direction,
                    group,
                }
                | HistoryInspectionQueryV1::PermittedBlockingDetail {
                    scope,
                    direction,
                    group,
                } => (scope, direction, group),
            };
            let index = e
                .projection
                .scopes
                .iter()
                .position(|s| s.alias == scope)
                .ok_or(HistoryError::Unavailable)?;
            if group.is_some_and(|g| !e.projection.groups.iter().any(|row| row.alias == g)) {
                return Err(HistoryError::Unavailable);
            }
            self.current_history_inspection(id, f, r)?;
            let p = self.history_inspection_pool()?;
            let e = self.history_inspection_entry(id, f.scope)?;
            let selected = &e.projection.scopes[index];
            if !(match direction {
                HistoryDirection::Undo => selected.undo_complete,
                HistoryDirection::Redo => selected.redo_complete,
            }) {
                return Err(HistoryError::Unavailable);
            }
            let stacks = e
                .basis
                .scope_stacks(&selected.scope)
                .ok_or(HistoryError::Unavailable)?;
            let (stack, lost) = match direction {
                HistoryDirection::Undo => (&stacks.undo, stacks.undo_lost),
                HistoryDirection::Redo => (&stacks.redo, stacks.redo_lost),
            };
            let chosen = match group {
                Some(alias) => Some(
                    &e.projection
                        .groups
                        .iter()
                        .find(|g| g.alias == alias)
                        .ok_or(HistoryError::Unavailable)?
                        .group,
                ),
                None => stack.last(),
            };
            let header = size_of::<HistoryInspectionReplyV1>();
            let Some(chosen) = chosen else {
                if lost {
                    return Err(HistoryError::Unavailable);
                }
                return match q {
                    HistoryInspectionQueryV1::Frontier { .. } => {
                        Ok(HistoryInspectionReplyV1::Frontier {
                            context: id.clone(),
                            group: None,
                        })
                    }
                    HistoryInspectionQueryV1::Availability { .. } => {
                        Ok(HistoryInspectionReplyV1::Availability {
                            context: id.clone(),
                            value: HistoryInspectionAvailabilityV1::Empty,
                        })
                    }
                    _ => Err(HistoryError::Unavailable),
                };
            };
            let public = e
                .projection
                .groups
                .iter()
                .find(|g| g.group == *chosen && g.evidence_ready)
                .ok_or(HistoryError::Unavailable)?;
            let actual = e
                .basis
                .selected_group(chosen)
                .filter(|g| g.valid && g.scopes.contains(&selected.scope))
                .ok_or(HistoryError::Unavailable)?;
            if matches!(q, HistoryInspectionQueryV1::Frontier { .. }) {
                if header
                    .checked_add(public.alias.len())
                    .is_none_or(|n| n > p.limits.max_reply_bytes)
                {
                    return Err(HistoryError::Limit);
                }
                return Ok(HistoryInspectionReplyV1::Frontier {
                    context: id.clone(),
                    group: Some(owned(&public.alias)?),
                });
            }
            let blockers = actual.scopes.iter().filter(|s| {
                e.basis.scope_stacks(s).and_then(|s| match direction {
                    HistoryDirection::Undo => s.undo.last(),
                    HistoryDirection::Redo => s.redo.last(),
                }) != Some(chosen)
            });
            if matches!(q, HistoryInspectionQueryV1::Availability { .. }) {
                return Ok(HistoryInspectionReplyV1::Availability {
                    context: id.clone(),
                    value: if blockers.count() == 0 {
                        HistoryInspectionAvailabilityV1::Eligible
                    } else {
                        HistoryInspectionAvailabilityV1::Blocked
                    },
                });
            }
            if !e.projection.details {
                return Err(HistoryError::Unavailable);
            }
            let visible = || {
                blockers.clone().filter_map(|scope| {
                    let s = e.projection.scopes.iter().find(|s| s.scope == *scope)?;
                    let top = e.basis.scope_stacks(scope).and_then(|s| match direction {
                        HistoryDirection::Undo => s.undo.last(),
                        HistoryDirection::Redo => s.redo.last(),
                    })?;
                    let g = e.projection.groups.iter().find(|g| g.group == *top)?;
                    Some((s.alias.as_str(), g.alias.as_str()))
                })
            };
            let count = visible().count();
            if count > p.limits.max_details {
                return Err(HistoryError::Limit);
            }
            let bytes = visible()
                .try_fold(header, |n, (s, g)| {
                    n.checked_add(size_of::<HistoryBlockerV1>())?
                        .checked_add(s.len())?
                        .checked_add(g.len())
                })
                .ok_or(HistoryError::Limit)?;
            if bytes > p.limits.max_reply_bytes {
                return Err(HistoryError::Limit);
            }
            let mut rows = Vec::new();
            rows.try_reserve_exact(count)
                .map_err(|_| HistoryError::Capacity)?;
            for (s, g) in visible() {
                rows.push(HistoryBlockerV1 {
                    scope: owned(s)?,
                    group: owned(g)?,
                });
            }
            // Match the reference's UTF-16 label order, not Rust UTF-8 order.
            rows.sort_unstable_by(|a, b| a.scope.encode_utf16().cmp(b.scope.encode_utf16()));
            Ok(HistoryInspectionReplyV1::PermittedBlockingDetail {
                context: id.clone(),
                blockers: rows,
            })
        })();
        #[cfg(test)]
        let out = {
            let mut out = out;
            if let State::Active(p) = &mut self.history_inspection
                && let Some(fault) = p.result_fault.take()
                && let Ok(HistoryInspectionReplyV1::Frontier { context, group }) = &mut out
            {
                match fault {
                    ResultFault::Context => context.slot = context.slot.wrapping_add(1),
                    ResultFault::Variant => {
                        out = Ok(HistoryInspectionReplyV1::Availability {
                            context: id.clone(),
                            value: HistoryInspectionAvailabilityV1::Empty,
                        })
                    }
                    ResultFault::Alias => *group = Some("unprojected-private-alias".into()),
                    ResultFault::Oversized => {
                        *group = Some("group".repeat(p.limits.max_reply_bytes))
                    }
                }
            }
            out
        };
        let out = if let Ok(reply) = &out {
            let accepted = self.history_inspection_pool().is_ok_and(|p| {
                self.history_inspection_entry(id, f.scope)
                    .is_ok_and(|e| valid_reply(id, q, e, p.limits, reply))
            });
            if self.finish_read_result(
                accepted,
                ReadResultInvariantV1::NativeHistoryResultInvariant,
                Boundary::HistoryInspectionResultValidation,
                r,
            ) {
                out
            } else {
                Err(HistoryError::Faulted)
            }
        } else {
            out
        };
        self.finish_history_inspection(Boundary::HistoryInspectionQuery, out, r)
            .unwrap_or(HistoryInspectionReplyV1::Unavailable)
    }
    /// Privileged preparation bridge, never an F04 reply or a reversal grant.
    /// The actual F14 preparation still independently checks the returned group.
    pub fn resolve_history_group_alias_v1(
        &mut self,
        id: &HistoryInspectionIdV1,
        alias: &str,
        f: &TrustedHistoryInspectionFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<HistoryGroupId> {
        let out = (|| {
            let e = self.history_inspection_entry(id, f.scope)?;
            let index = e
                .projection
                .groups
                .iter()
                .position(|g| g.alias == alias)
                .ok_or(HistoryError::Unavailable)?;
            self.current_history_inspection(id, f, r)?;
            Ok(self
                .history_inspection_entry(id, f.scope)?
                .projection
                .groups[index]
                .group
                .clone())
        })();
        self.finish_history_inspection(Boundary::HistoryInspectionAliasResolution, out, r)
            .ok()
    }
    /// Privileged scope bridge with the same current checks; not recipient data.
    pub fn resolve_history_scope_alias_v1(
        &mut self,
        id: &HistoryInspectionIdV1,
        alias: &str,
        f: &TrustedHistoryInspectionFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<HistoryBasisScopeV1> {
        let out = (|| {
            let e = self.history_inspection_entry(id, f.scope)?;
            let index = e
                .projection
                .scopes
                .iter()
                .position(|s| s.alias == alias)
                .ok_or(HistoryError::Unavailable)?;
            self.current_history_inspection(id, f, r)?;
            Ok(self
                .history_inspection_entry(id, f.scope)?
                .projection
                .scopes[index]
                .scope
                .clone())
        })();
        self.finish_history_inspection(Boundary::HistoryInspectionAliasResolution, out, r)
            .ok()
    }
    pub fn stop_history_inspection_v1(
        &mut self,
        id: &HistoryInspectionIdV1,
        f: &HistoryInspectionControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        self.stop_history_inspection_v1_result(id, f, r).is_ok()
    }
    pub(in crate::coordinator) fn stop_history_inspection_v1_result(
        &mut self,
        id: &HistoryInspectionIdV1,
        f: &HistoryInspectionControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), HistoryError> {
        let out = (|| {
            self.history_inspection_entry(id, f.scope)?;
            if f.owner_control_allowed != Some(true) {
                return Err(HistoryError::Unavailable);
            }
            self.invalidate_history_inspection(id, r);
            Ok(())
        })();
        self.finish_history_inspection(Boundary::HistoryInspectionStop, out, r)
    }
    pub fn retire_history_inspection_v1(
        &mut self,
        id: &HistoryInspectionIdV1,
        f: &HistoryInspectionControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let out = (|| {
            self.history_inspection_entry(id, f.scope)?;
            if [
                f.owner_control_allowed,
                f.retention_released,
                f.delivery_drained,
            ]
            .into_iter()
            .any(|v| v != Some(true))
            {
                return Err(HistoryError::Unavailable);
            }
            let State::Active(p) = &mut self.history_inspection else {
                return Err(HistoryError::Unavailable);
            };
            p.rows[id.slot] = None;
            Ok(())
        })();
        self.finish_history_inspection(Boundary::HistoryInspectionRetirement, out, r)
            .is_ok()
    }
    pub fn stop_history_inspection_owner_v1(
        &mut self,
        f: &HistoryInspectionControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let out = (|| {
            let State::Active(p) = &mut self.history_inspection else {
                return Err(HistoryError::Unavailable);
            };
            if p.scope != *f.scope || f.owner_control_allowed != Some(true) {
                return Err(HistoryError::Unavailable);
            }
            p.stopped = true;
            for e in p.rows.iter_mut().flatten() {
                e.live = false;
            }
            Ok(())
        })();
        self.finish_history_inspection(Boundary::HistoryInspectionStop, out, r)
            .is_ok()
    }
    pub fn retire_history_inspection_owner_v1(
        &mut self,
        f: &HistoryInspectionControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let out = (|| {
            let State::Active(p) = &mut self.history_inspection else {
                return Err(HistoryError::Unavailable);
            };
            if p.scope != *f.scope
                || !p.stopped
                || p.rows.iter().any(Option::is_some)
                || [
                    f.owner_control_allowed,
                    f.retention_released,
                    f.delivery_drained,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(HistoryError::Unavailable);
            }
            self.retention.release(&mut p.reservation).map_err(parent)?;
            let issued = p.rows.len();
            self.history_inspection = State::Closed { issued };
            Ok(())
        })();
        self.finish_history_inspection(Boundary::HistoryInspectionPoolRetirement, out, r)
            .is_ok()
    }
}
