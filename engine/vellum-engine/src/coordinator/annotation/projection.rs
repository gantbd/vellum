//! Private current-root preparation, separately funded from installed F02/F03 owners.
use super::*;
mod collection;
mod shutdown_inventory;
pub use collection::{
    CollectionFacetCeilingV1, CollectionVisibilityCaptureV1, CollectionVisibilityPreparationIdV1,
    TrustedCollectionVisibilityFactsV1,
};
mod installed;
mod reader;
#[cfg(test)]
pub(in crate::coordinator) use installed::native_facade_single_fixture;
pub(in crate::coordinator) mod version_set;
pub(in crate::coordinator) use installed::AnnotationSourceHoldId;
pub(in crate::coordinator) use installed::InstalledState;
pub use installed::{
    AnnotationContentV1, AnnotationContextAcquisitionV1, AnnotationContextLimitsV1,
    AnnotationContextUsageV1, AnnotationContinuationId, AnnotationLocationV1,
    AnnotationQueryReplyV1, AnnotationQueryV1, AnnotationReadContextId, AnnotationRecordId,
    AnnotationRecordV1, AnnotationSourceReplyV1, TrustedAnnotationContextFactsV1,
};
#[cfg(test)]
mod tests;
type Error = SourceReadContextErrorV1;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceProjectionLimitsV1 {
    pub max_source_bytes: usize,
    pub max_request_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_scan_bytes: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnnotationPreparationLimitsV1 {
    pub max_prepared: usize,
    pub max_issued: usize,
    pub max_rows: usize,
    pub max_root_bytes: usize,
    pub max_capture_bytes: usize,
    pub max_context_bytes: usize,
    pub max_request_bytes: usize,
    pub max_selector_bytes: usize,
    pub max_label_bytes: usize,
    pub max_pool_bytes: usize,
    pub source: SourceProjectionLimitsV1,
}
impl AnnotationPreparationLimitsV1 {
    /// Conservative native inline/payload/workspace units; not JSON or heap peak.
    pub fn required_pool_bytes(self) -> Result<usize, Error> {
        let mut n = std::mem::size_of::<Pool>();
        for (count, size) in [
            (self.max_issued, std::mem::size_of::<Option<Cut>>()),
            (2, self.max_request_bytes),
            (2, self.max_capture_bytes),
            (2, self.max_context_bytes),
            (self.max_prepared, self.max_capture_bytes),
            (self.max_prepared, self.max_context_bytes),
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationProjectionClassesV1 {
    /// Retained narrowing only; current source capture uses the source-context owner.
    Source,
    Annotations,
    Combined,
}
impl AnnotationProjectionClassesV1 {
    fn includes_source(self) -> bool {
        match self {
            Self::Annotations => false,
            Self::Source | Self::Combined => true,
        }
    }
    fn includes_annotations(self) -> bool {
        self != Self::Source
    }
    fn subset_of(self, other: Self) -> bool {
        (!self.includes_source() || other.includes_source())
            && (!self.includes_annotations() || other.includes_annotations())
    }
}
#[derive(Clone)]
pub struct AnnotationFacetGrantV1 {
    pub annotation: AnnotationId,
    pub visible: bool,
    pub body: bool,
    pub target: bool,
}
impl std::fmt::Debug for AnnotationFacetGrantV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationFacetGrantV1(<redacted>)")
    }
}
/// Independently attributed host visibility/facet facts, never requester grants.
pub struct TrustedAnnotationCaptureFactsV1<'a> {
    pub scope: &'a SourceReadScopeV1,
    pub grants: &'a [AnnotationFacetGrantV1],
    pub authorized: Option<bool>,
    pub scope_allowed: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub annotation_readable: Option<bool>,
    pub source_readable: Option<bool>,
    pub visibility_complete: Option<bool>,
    pub projection_order_attested: Option<bool>,
    pub resources_ready: Option<bool>,
}
/// Current authority for the entire actual original prepared facet domain.
pub struct TrustedPreparedAnnotationFactsV1<'a> {
    pub projection: &'a PreparedAnnotationProjectionId,
    pub scope: &'a SourceReadScopeV1,
    pub authorized: Option<bool>,
    pub scope_allowed: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub annotation_readable: Option<bool>,
    pub source_readable: Option<bool>,
    pub projection_current: Option<bool>,
    pub retained: Option<bool>,
    pub resources_ready: Option<bool>,
    pub reply_ready: Option<bool>,
    pub transfer_ready: Option<bool>,
}
#[derive(Clone)]
pub struct PreparedAnnotationProjectionId {
    owner: Arc<()>,
    slot: usize,
}
impl PartialEq for PreparedAnnotationProjectionId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for PreparedAnnotationProjectionId {}
impl std::fmt::Debug for PreparedAnnotationProjectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreparedAnnotationProjectionId(<opaque>)")
    }
}
struct Row {
    id: AnnotationId,
    body: Option<(String, u64)>,
    location: Option<(Placement, Option<u64>, u64)>,
}
struct Projection {
    classes: AnnotationProjectionClassesV1,
    rows: Vec<Row>,
    source: Option<(Utf8Source, DocumentVersion, SourceProjectionLimitsV1)>,
}
impl Projection {
    fn bytes(&self) -> Result<usize, Error> {
        let mut n = std::mem::size_of::<Self>();
        if let Some((source, _, _)) = &self.source {
            n = n.checked_add(source.text().len()).ok_or(Error::Limit)?;
        }
        for row in &self.rows {
            n = n
                .checked_add(std::mem::size_of::<Row>())
                .and_then(|n| n.checked_add(row.body.as_ref().map_or(0, |v| v.0.len())))
                .ok_or(Error::Limit)?;
        }
        Ok(n)
    }
    fn valid(&self, scope: &SourceReadScopeV1, owner: &Arc<()>) -> bool {
        self.classes.includes_source() == self.source.is_some()
            && (self.classes.includes_annotations() || self.rows.is_empty())
            && self.rows.iter().enumerate().all(|(i, r)| {
                r.id.document == scope.document
                    && Arc::ptr_eq(&r.id.namespace, owner)
                    && !self.rows[..i].iter().any(|prior| prior.id == r.id)
                    && r.location.as_ref().is_none_or(|(p, revision, _)| match p {
                        Placement::Located { .. } => revision.is_some(),
                        Placement::Detached => revision.is_none(),
                        Placement::Unavailable => false,
                    })
            })
            && self.source.as_ref().is_none_or(|(text, version, p)| {
                version.document == scope.document && text.text().len() <= p.max_source_bytes
            })
    }
}
struct Cut {
    scope: SourceReadScopeV1,
    value: Projection,
    invalidated: bool,
    collection: Option<collection::Binding>,
}
pub(in crate::coordinator) struct Pool {
    authority: String,
    engine: String,
    limits: AnnotationPreparationLimitsV1,
    reservation: retention::Reservation,
    charge: usize,
    cuts: Vec<Option<Cut>>,
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
        f.write_str("AnnotationPreparationState(<redacted>)")
    }
}
impl State {
    pub(in crate::coordinator) fn document_pins(&self, document: &DocumentId) -> usize {
        match self {
            Self::Unconfigured | Self::Closed { .. } => 0,
            Self::Active(p) => p
                .cuts
                .iter()
                .flatten()
                .filter(|c| c.scope.document == *document)
                .count(),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnnotationPreparationUsageV1 {
    pub prepared: usize,
    pub invalidated: usize,
    pub issued_lifetime: usize,
    pub reserved_pool_bytes: usize,
    pub source_bytes: usize,
    pub body_bytes: usize,
    pub projected_rows: usize,
    pub closed: bool,
}
fn parent(e: RetentionErrorV1) -> Error {
    match e {
        RetentionErrorV1::Unavailable => Error::Unavailable,
        RetentionErrorV1::Limit => Error::Limit,
        RetentionErrorV1::Faulted => Error::Faulted,
    }
}
fn owned(v: &str) -> Result<String, Error> {
    let mut s = String::new();
    s.try_reserve_exact(v.len()).map_err(|_| Error::Capacity)?;
    s.push_str(v);
    Ok(s)
}
fn scope_bytes(s: &SourceReadScopeV1) -> Result<usize, Error> {
    source_context::scope_bytes(s)
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
fn discharge(f: &SourceReadContextControlV1<'_>) -> bool {
    [
        f.owner_control_allowed,
        f.retention_released,
        f.delivery_drained,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
impl SourceCoordinator {
    /// Internal read-only binding seam; matching is neither validation nor authority.
    pub(in crate::coordinator) fn prepared_annotation_matches(
        &self,
        id: &PreparedAnnotationProjectionId,
        scope: &SourceReadScopeV1,
        classes: AnnotationProjectionClassesV1,
    ) -> bool {
        self.prepared_annotation_cut(id, scope)
            .is_ok_and(|cut| cut.value.classes == classes)
    }
    fn annotation_preparation_pool(&self) -> Result<&Pool, Error> {
        match &self.annotation_preparations {
            State::Active(p) => Ok(p),
            _ => Err(Error::Unavailable),
        }
    }
    fn prepared_annotation_cut(
        &self,
        id: &PreparedAnnotationProjectionId,
        s: &SourceReadScopeV1,
    ) -> Result<&Cut, Error> {
        let cut = self.prepared_annotation_cut_any(id, s)?;
        if cut.collection.is_some() {
            return Err(Error::Unavailable);
        }
        Ok(cut)
    }
    fn prepared_annotation_cut_any(
        &self,
        id: &PreparedAnnotationProjectionId,
        s: &SourceReadScopeV1,
    ) -> Result<&Cut, Error> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return Err(Error::Unavailable);
        }
        let p = self.annotation_preparation_pool()?;
        let cut = p
            .cuts
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)?;
        if cut.scope != *s {
            return Err(Error::Unavailable);
        }
        Ok(cut)
    }
    fn finish_annotation_preparation<T>(
        &mut self,
        b: Boundary,
        result: &Result<T, Error>,
        r: &mut Recorder,
    ) {
        if matches!(result, Err(Error::Faulted)) {
            self.faulted = true;
        }
        r.record(Observation {
            boundary: b,
            disposition: match result {
                Ok(_) => Disposition::Completed,
                Err(Error::Faulted) => Disposition::Unknown,
                _ => Disposition::Refused,
            },
            service_time: None,
        });
    }
    pub fn configure_annotation_preparation(
        &mut self,
        authority: &str,
        engine: &str,
        limits: AnnotationPreparationLimitsV1,
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
                || authority.len() > limits.max_label_bytes
                || engine.len() > limits.max_label_bytes
            {
                return Err(Error::InvalidRequest);
            }
            match &self.annotation_preparations {
                State::Active(p) => {
                    self.retention.validate(&p.reservation).map_err(parent)?;
                    return if p.authority == authority && p.engine == engine && p.limits == limits {
                        Ok(false)
                    } else {
                        Err(Error::Unavailable)
                    };
                }
                State::Closed { .. } => return Err(Error::Unavailable),
                State::Unconfigured => {}
            }
            self.ensure_admission_unfenced()
                .map_err(source_context::admission_error)?;
            if std::mem::size_of::<AnnotationPreparationLimitsV1>()
                .checked_add(authority.len())
                .and_then(|n| n.checked_add(engine.len()))
                .is_none_or(|n| n > limits.max_request_bytes)
            {
                return Err(Error::Limit);
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
            let mut cuts = Vec::new();
            cuts.try_reserve_exact(limits.max_issued)
                .map_err(|_| Error::Capacity)?;
            let reservation = self.retention.reserve(charge).map_err(parent)?;
            self.annotation_preparations = State::Active(Box::new(Pool {
                authority,
                engine,
                limits,
                reservation,
                charge,
                cuts,
            }));
            Ok(true)
        })();
        self.finish_annotation_preparation(Boundary::AnnotationProjectionConfiguration, &result, r);
        result
    }
    pub fn annotation_preparation_usage(&self) -> AnnotationPreparationUsageV1 {
        let mut u = AnnotationPreparationUsageV1::default();
        match &self.annotation_preparations {
            State::Unconfigured => {}
            State::Closed { issued } => {
                u.closed = true;
                u.issued_lifetime = *issued;
            }
            State::Active(p) => {
                u.reserved_pool_bytes = p.charge;
                u.issued_lifetime = p.cuts.len();
                for c in p.cuts.iter().flatten() {
                    u.prepared += 1;
                    u.invalidated += usize::from(c.invalidated);
                    u.projected_rows += c.value.rows.len();
                    if let Some((source, _, _)) = &c.value.source {
                        u.source_bytes += source.text().len();
                    }
                    for row in &c.value.rows {
                        u.body_bytes += row.body.as_ref().map_or(0, |v| v.0.len());
                    }
                }
            }
        }
        u
    }
    /// Private staging only: every caller retains one exclusive owner borrow.
    fn stage_current_projection(
        &mut self,
        classes: AnnotationProjectionClassesV1,
        s: &SourceReadScopeV1,
        f: &TrustedAnnotationCaptureFactsV1<'_>,
        limits: AnnotationPreparationLimitsV1,
        ceiling: Option<CollectionFacetCeilingV1>,
        r: &mut Recorder,
    ) -> Result<Projection, Error> {
        if f.scope != s
            || [
                f.authorized,
                f.scope_allowed,
                f.continuous,
                f.live,
                f.not_stopped,
                f.resources_ready,
            ]
            .into_iter()
            .any(|v| v != Some(true))
            || classes.includes_source() && f.source_readable != Some(true)
            || classes.includes_annotations()
                && [
                    f.annotation_readable,
                    f.visibility_complete,
                    f.projection_order_attested,
                ]
                .into_iter()
                .any(|v| v != Some(true))
        {
            return Err(Error::Unavailable);
        }
        self.document(&s.document).map_err(|_| Error::Unavailable)?;
        if !classes.includes_annotations() && !f.grants.is_empty() {
            return Err(Error::InvalidRequest);
        }
        for (i, g) in f.grants.iter().enumerate() {
            if !Arc::ptr_eq(&g.annotation.namespace, &self.namespace)
                || g.annotation.document != s.document
                || !g.visible && (g.body || g.target)
                || f.grants[..i].iter().any(|x| x.annotation == g.annotation)
            {
                return Err(Error::InvalidRequest);
            }
        }
        let actual_count = self
            .annotations
            .records
            .iter()
            .flatten()
            .filter(|v| {
                classes.includes_annotations()
                    && v.value.present
                    && v.value.id.document == s.document
            })
            .count();
        if actual_count > limits.max_rows
            || actual_count != f.grants.len()
            || self
                .annotations
                .records
                .iter()
                .flatten()
                .filter(|v| {
                    classes.includes_annotations()
                        && v.value.present
                        && v.value.id.document == s.document
                })
                .any(|v| !f.grants.iter().any(|g| g.annotation == v.value.id))
        {
            return Err(Error::InvalidRequest);
        }
        // Complete host rows must be a bijection with actual identities,
        // not just equal cardinality plus one-way membership.
        for grant in f.grants {
            let matching = self
                .annotations
                .records
                .iter()
                .flatten()
                .filter(|v| v.value.present && v.value.id == grant.annotation)
                .take(2)
                .count();
            match matching {
                0 => return Err(Error::InvalidRequest),
                1 => {}
                _ => return Err(Error::Faulted),
            }
        }
        if self
            .semantic_root_usage(r)
            .map_err(|_| Error::Faulted)?
            .total_bytes
            > limits.max_root_bytes
        {
            return Err(Error::Limit);
        }
        let mut rows = Vec::new();
        rows.try_reserve_exact(f.grants.iter().filter(|g| g.visible).count())
            .map_err(|_| Error::Capacity)?;
        let mut capture = std::mem::size_of::<Projection>();
        for record in self.annotations.records.iter().flatten().filter(|v| {
            classes.includes_annotations() && v.value.present && v.value.id.document == s.document
        }) {
            let v = &record.value;
            let grant = f
                .grants
                .iter()
                .find(|g| g.annotation == v.id)
                .ok_or(Error::Faulted)?;
            if !grant.visible {
                continue;
            }
            let body = grant.body && ceiling.is_none_or(|c| c.body);
            let target = grant.target && ceiling.is_none_or(|c| c.target);
            if target {
                match v.target.placement {
                    Placement::Unavailable => return Err(Error::Unavailable),
                    Placement::Located { .. } if v.target.revision.is_none() => {
                        return Err(Error::Faulted);
                    }
                    Placement::Detached if v.target.revision.is_some() => {
                        return Err(Error::Faulted);
                    }
                    Placement::Located { .. } | Placement::Detached => {}
                }
            }
            capture = capture
                .checked_add(std::mem::size_of::<Row>())
                .and_then(|n| n.checked_add(if body { v.body.len() } else { 0 }))
                .ok_or(Error::Limit)?;
            if capture > limits.max_capture_bytes {
                return Err(Error::Limit);
            }
            rows.push(Row {
                id: v.id.clone(),
                body: if body {
                    Some((owned(&v.body)?, v.content))
                } else {
                    None
                },
                location: if target {
                    Some((v.target.placement, v.target.revision, v.target_generation))
                } else {
                    None
                },
            });
        }
        if ceiling.is_some() {
            // Host attests this visible order. Hidden row offsets never become
            // recipient references, and native slot order is not substituted.
            rows.sort_unstable_by_key(|r| f.grants.iter().position(|g| g.annotation == r.id));
        }
        let source = if classes.includes_source() {
            let d = self.document(&s.document).map_err(|_| Error::Unavailable)?;
            if d.source.text().len() > limits.source.max_source_bytes {
                return Err(Error::Limit);
            }
            capture = capture
                .checked_add(d.source.text().len())
                .ok_or(Error::Limit)?;
            if capture > limits.max_capture_bytes {
                return Err(Error::Limit);
            }
            Some((
                d.source.copy_owned().map_err(|_| Error::Capacity)?,
                DocumentVersion {
                    document: s.document.clone(),
                    revision: d.revision,
                },
                limits.source,
            ))
        } else {
            None
        };
        if capture > limits.max_capture_bytes {
            return Err(Error::Limit);
        }
        let value = Projection {
            classes,
            rows,
            source,
        };
        if !value.valid(s, &self.namespace) || value.bytes()? != capture {
            return Err(Error::Faulted);
        }
        Ok(value)
    }
    pub fn prepare_annotation_projection(
        &mut self,
        classes: AnnotationProjectionClassesV1,
        s: &SourceReadScopeV1,
        f: &TrustedAnnotationCaptureFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<PreparedAnnotationProjectionId> {
        let result = (|| {
            // This current-capture port admits annotation/combined projections;
            // source-only current capture remains in the source-context owner.
            if classes == AnnotationProjectionClassesV1::Source {
                return Err(Error::InvalidRequest);
            }
            if self.faulted {
                return Err(Error::Faulted);
            }
            self.ensure_admission_unfenced()
                .map_err(source_context::admission_error)?;
            let p = self.annotation_preparation_pool()?;
            if !valid_scope(s, p) || s != f.scope {
                return Err(Error::Unavailable);
            }
            let request = scope_bytes(s)?
                .checked_add(std::mem::size_of::<AnnotationProjectionClassesV1>())
                .and_then(|n| {
                    n.checked_add(
                        f.grants
                            .len()
                            .checked_mul(std::mem::size_of::<AnnotationFacetGrantV1>())?,
                    )
                })
                .ok_or(Error::Limit)?;
            if request > p.limits.max_request_bytes || f.grants.len() > p.limits.max_rows {
                return Err(Error::Limit);
            }
            if [
                f.authorized,
                f.scope_allowed,
                f.continuous,
                f.live,
                f.not_stopped,
                f.annotation_readable,
                f.visibility_complete,
                f.projection_order_attested,
                f.resources_ready,
            ]
            .into_iter()
            .any(|v| v != Some(true))
                || classes.includes_source() && f.source_readable != Some(true)
            {
                return Err(Error::Unavailable);
            }
            self.document(&s.document).map_err(|_| Error::Unavailable)?;
            self.retention.validate(&p.reservation).map_err(parent)?;
            if p.cuts.len() >= p.limits.max_issued
                || p.cuts.iter().flatten().count() >= p.limits.max_prepared
            {
                return Err(Error::Limit);
            }
            let limits = p.limits;
            let value = self.stage_current_projection(classes, s, f, limits, None, r)?;
            if std::mem::size_of::<PreparedAnnotationProjectionId>() > limits.max_selector_bytes {
                return Err(Error::Limit);
            }
            let scope = SourceReadScopeV1 {
                document: s.document.clone(),
                context: settings::basis::copy_context(&s.context).map_err(|_| Error::Capacity)?,
                policy: owned(&s.policy)?,
            };
            let State::Active(p) = &mut self.annotation_preparations else {
                return Err(Error::Faulted);
            };
            let id = PreparedAnnotationProjectionId {
                owner: self.namespace.clone(),
                slot: p.cuts.len(),
            };
            p.cuts.push(Some(Cut {
                scope,
                value,
                invalidated: false,
                collection: None,
            }));
            Ok(id)
        })();
        self.finish_annotation_preparation(Boundary::AnnotationProjectionPreparation, &result, r);
        result.ok()
    }
    pub fn validate_prepared_annotation_projection(
        &mut self,
        id: &PreparedAnnotationProjectionId,
        s: &SourceReadScopeV1,
        f: &TrustedPreparedAnnotationFactsV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let result = (|| {
            let p = self.annotation_preparation_pool()?;
            if !valid_scope(s, p) || id != f.projection || s != f.scope {
                return Err(Error::Unavailable);
            }
            if scope_bytes(s)?
                .checked_add(std::mem::size_of::<PreparedAnnotationProjectionId>())
                .is_none_or(|n| n > p.limits.max_request_bytes)
            {
                return Err(Error::Limit);
            }
            let cut = self.prepared_annotation_cut(id, s)?;
            if cut.invalidated {
                return Err(Error::Unavailable);
            }
            if !cut.value.valid(s, &self.namespace) {
                return Err(Error::Faulted);
            }
            let combined = cut.value.source.is_some();
            let gates = [
                f.authorized,
                f.scope_allowed,
                f.continuous,
                f.live,
                f.not_stopped,
                f.annotation_readable,
                f.projection_current,
                f.retained,
            ];
            if self.faulted
                || self.document(&s.document).is_err()
                || gates.contains(&Some(false))
                || combined && f.source_readable == Some(false)
            {
                let State::Active(p) = &mut self.annotation_preparations else {
                    return Err(Error::Faulted);
                };
                p.cuts[id.slot].as_mut().ok_or(Error::Faulted)?.invalidated = true;
                self.finish_annotation_preparation(
                    Boundary::AnnotationProjectionInvalidation,
                    &Ok::<_, Error>(()),
                    r,
                );
                return Err(Error::Unavailable);
            }
            self.retention.validate(&p.reservation).map_err(parent)?;
            if gates
                .into_iter()
                .chain([f.resources_ready, f.reply_ready, f.transfer_ready])
                .any(|v| v != Some(true))
                || combined && f.source_readable != Some(true)
            {
                return Err(Error::Unavailable);
            }
            Ok(())
        })();
        self.finish_annotation_preparation(Boundary::AnnotationProjectionValidation, &result, r);
        result.is_ok()
    }
    pub fn discard_prepared_annotation_projection(
        &mut self,
        id: &PreparedAnnotationProjectionId,
        f: &SourceReadContextControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let result = (|| {
            if !discharge(f) {
                return Err(Error::Unavailable);
            }
            self.prepared_annotation_cut(id, f.scope)?;
            self.retention
                .validate(&self.annotation_preparation_pool()?.reservation)
                .map_err(parent)?;
            let State::Active(p) = &mut self.annotation_preparations else {
                return Err(Error::Faulted);
            };
            p.cuts[id.slot] = None;
            Ok(())
        })();
        self.finish_annotation_preparation(Boundary::AnnotationProjectionDiscard, &result, r);
        result.is_ok()
    }
    pub fn retire_annotation_preparation(
        &mut self,
        f: &SourceReadPoolControlV1,
        r: &mut Recorder,
    ) -> Result<bool, Error> {
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
            let State::Active(p) = &mut self.annotation_preparations else {
                return Ok(false);
            };
            if p.cuts.iter().any(Option::is_some) {
                return Err(Error::Unavailable);
            }
            let issued = p.cuts.len();
            self.retention.release(&mut p.reservation).map_err(parent)?;
            self.annotation_preparations = State::Closed { issued };
            Ok(true)
        })();
        self.finish_annotation_preparation(Boundary::AnnotationPreparationRetirement, &result, r);
        result
    }
}
