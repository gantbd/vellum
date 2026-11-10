//! Closed first-party issuer sum. No caller-supplied provider or ownership label.
use super::*;
#[derive(Clone, Copy)]
pub enum SearchReadContextV1<'a> {
    Source(&'a SourceReadContextId),
    /// Installed projection owner, including source-only retained narrowing.
    Combined(&'a AnnotationReadContextId),
    VersionSet(&'a VersionSetMemberId),
}
pub struct TrustedVersionSetSearchFactsV1<'a> {
    /// Original member attribution, distinct from fresh whole-parent host facts.
    pub member: &'a VersionSetMemberId,
    pub scope: &'a SourceReadScopeV1,
    pub parent: &'a TrustedVersionSetContextFactsV1<'a>,
}
#[derive(Clone, Copy)]
pub enum SearchResultControlV1<'a> {
    Single(&'a SourceReadContextControlV1<'a>),
    /// Complete ordered protected frame; no content grant or public lease.
    VersionSet(&'a [SourceReadContextControlV1<'a>]),
}
#[derive(Clone, Copy)]
pub enum SearchReadFactsV1<'a> {
    Source(&'a TrustedSourceReadFactsV1<'a>),
    Combined(&'a TrustedAnnotationContextFactsV1<'a>),
    VersionSet(&'a TrustedVersionSetSearchFactsV1<'a>),
}
impl SearchReadFactsV1<'_> {
    pub(in crate::coordinator) fn scope(&self) -> &SourceReadScopeV1 {
        match self {
            Self::Source(f) => f.scope,
            Self::Combined(f) => f.scope,
            Self::VersionSet(f) => f.scope,
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(super) enum Issuer {
    Source(SourceReadContextId),
    Combined(AnnotationReadContextId),
    VersionSet {
        member: VersionSetMemberId,
        parent: VersionSetContextId,
    },
}
#[derive(Clone)]
pub(super) enum IssuerHold {
    Source(SourceReadHoldId),
    Combined(annotation::projection::AnnotationSourceHoldId),
    VersionSet(annotation::projection::version_set::VersionSetSourceHoldId),
}
impl Issuer {
    pub(super) fn matches(&self, f: SearchReadFactsV1<'_>) -> bool {
        match (self, f) {
            (Self::Source(_), SearchReadFactsV1::Source(_)) => true,
            (Self::Combined(id), SearchReadFactsV1::Combined(f)) => id == f.context,
            (Self::VersionSet { member, parent }, SearchReadFactsV1::VersionSet(f)) => {
                member == f.member && parent == f.parent.context
            }
            _ => false,
        }
    }
}
impl SourceCoordinator {
    pub(super) fn own_search_issuer(
        &self,
        source: SearchReadContextV1<'_>,
    ) -> Result<Issuer, Error> {
        Ok(match source {
            SearchReadContextV1::Source(id) => Issuer::Source(id.clone()),
            SearchReadContextV1::Combined(id) => Issuer::Combined(id.clone()),
            SearchReadContextV1::VersionSet(id) => Issuer::VersionSet {
                member: id.clone(),
                parent: self.version_search_parent(id)?,
            },
        })
    }
    /// Full current-host frame is bounded independently of the selected source envelope.
    pub(super) fn check_search_request(
        &self,
        source: &Issuer,
        s: &SourceReadScopeV1,
        q: &SearchRequest,
        f: SearchReadFactsV1<'_>,
        tail: usize,
    ) -> Result<usize, Error> {
        if !source.matches(f) || f.scope() != s {
            return Err(Error::Unavailable);
        }
        let inner = request_bytes(s, q)?.checked_add(tail).ok_or(Error::Limit)?;
        let snapshot = self.search_snapshot(source, s)?;
        let (extra, outer_limit) = match (source, f) {
            (Issuer::VersionSet { member, .. }, SearchReadFactsV1::VersionSet(f)) => {
                self.version_search_frame(member, s, f.parent)?
            }
            _ => (0, usize::MAX),
        };
        let outer = inner.checked_add(extra).ok_or(Error::Limit)?;
        if inner > snapshot.max_request_bytes
            || outer
                > self
                    .search_pool()?
                    .limits
                    .max_request_bytes
                    .min(outer_limit)
        {
            return Err(Error::Limit);
        }
        Ok(outer)
    }
    pub(super) fn search_snapshot<'a>(
        &'a self,
        source: &Issuer,
        s: &SourceReadScopeV1,
    ) -> Result<SearchSnapshot<'a>, Error> {
        let p = self.search_pool()?;
        if p.authority != s.context.authority || p.engine != s.context.engine {
            return Err(Error::Unavailable);
        }
        match source {
            Issuer::Source(id) => {
                let p = self.source_pool()?;
                if !valid_scope(s, p) {
                    return Err(Error::Unavailable);
                }
                let row = self.source_context_row(id, s)?;
                if row.closed {
                    return Err(Error::Unavailable);
                }
                Ok(SearchSnapshot {
                    source: row.source.as_ref().ok_or(Error::Faulted)?,
                    version: &row.version,
                    max_root_bytes: p.limits.max_root_bytes,
                    max_request_bytes: p.limits.max_request_bytes,
                    max_reply_bytes: p.limits.max_reply_bytes,
                })
            }
            Issuer::Combined(id) => self.annotation_search_snapshot(id, s),
            Issuer::VersionSet { member, .. } => self.version_search_snapshot(member, s),
        }
    }
    pub(super) fn check_search_issuer(
        &mut self,
        source: &Issuer,
        s: &SourceReadScopeV1,
        f: SearchReadFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        if !source.matches(f) || f.scope() != s {
            return Err(Error::Unavailable);
        }
        match (source, f) {
            (Issuer::Source(id), SearchReadFactsV1::Source(f)) => {
                self.check_search_source(id, s, f, r)
            }
            (Issuer::Combined(id), SearchReadFactsV1::Combined(f)) => {
                self.source_search_funding()?;
                self.check_annotation_context(id, s, f, r)
            }
            (Issuer::VersionSet { member, .. }, SearchReadFactsV1::VersionSet(f)) => {
                self.source_search_funding()?;
                self.check_version_search(member, s, f.parent, r)
            }
            _ => Err(Error::Unavailable),
        }
    }
    pub(super) fn hold_search_issuer(
        &mut self,
        source: &Issuer,
        s: &SourceReadScopeV1,
        f: SearchReadFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<IssuerHold, Error> {
        match (source, f) {
            (Issuer::Source(id), SearchReadFactsV1::Source(_)) => self
                .hold_source_read_context(
                    id,
                    s,
                    &SourceReadHoldFactsV1 {
                        owner_control_allowed: Some(true),
                        retained: Some(true),
                        resources_ready: Some(true),
                    },
                    r,
                )
                .map(IssuerHold::Source)
                .ok_or(Error::Unavailable),
            (Issuer::Combined(id), SearchReadFactsV1::Combined(f)) => {
                let result = self
                    .hold_annotation_search_source(id, s, f, r)
                    .map(IssuerHold::Combined);
                observe(r, Boundary::SourceContextHold, &result);
                result
            }
            (Issuer::VersionSet { member, .. }, SearchReadFactsV1::VersionSet(f)) => {
                let result = self
                    .hold_version_search(member, s, f.parent, r)
                    .map(IssuerHold::VersionSet);
                observe(r, Boundary::SourceContextHold, &result);
                result
            }
            _ => Err(Error::Unavailable),
        }
    }
    pub(super) fn validate_search_issuer_hold(
        &self,
        source: &Issuer,
        hold: &IssuerHold,
        s: &SourceReadScopeV1,
    ) -> Result<(), Error> {
        match (source, hold) {
            (Issuer::Source(id), IssuerHold::Source(hold)) => {
                let p = self.source_pool()?;
                let row = self.source_context_row(id, s)?;
                if p.actual_holds(id.slot)? != row.holds
                    || !Arc::ptr_eq(&hold.owner, &self.namespace)
                    || p.holds
                        .get(hold.slot)
                        .and_then(Option::as_ref)
                        .is_none_or(|h| h.context != id.slot)
                {
                    return Err(Error::Faulted);
                }
                Ok(())
            }
            (Issuer::Combined(id), IssuerHold::Combined(hold)) => {
                self.validate_annotation_source_hold(hold, id, s)
            }
            (Issuer::VersionSet { member, parent }, IssuerHold::VersionSet(hold)) => {
                if self.version_search_parent(member)? != *parent {
                    return Err(Error::Faulted);
                }
                self.validate_version_search_hold(hold, member, s)
            }
            _ => Err(Error::Faulted),
        }
    }
    pub(super) fn release_search_issuer_hold(
        &mut self,
        hold: &IssuerHold,
        f: SearchResultControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        match (hold, f) {
            (IssuerHold::Source(h), SearchResultControlV1::Single(f)) => {
                if self.release_source_read_hold(h, f, r) {
                    Ok(())
                } else {
                    Err(Error::Faulted)
                }
            }
            (IssuerHold::Combined(h), SearchResultControlV1::Single(f)) => {
                let result = self.release_annotation_source_hold(h, f);
                observe(r, Boundary::SourceContextRelease, &result);
                result
            }
            (IssuerHold::VersionSet(h), SearchResultControlV1::VersionSet(f)) => {
                let result = self.release_version_search_hold(h, f);
                observe(r, Boundary::SourceContextRelease, &result);
                result
            }
            _ => Err(Error::Unavailable),
        }
    }
    pub(super) fn discard_unpublished_search_hold(
        &mut self,
        hold: &IssuerHold,
        s: &SourceReadScopeV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        match hold {
            IssuerHold::VersionSet(h) => {
                let result = self.discard_unpublished_version_search_hold(h);
                observe(r, Boundary::SourceContextRelease, &result);
                result
            }
            _ => {
                self.release_search_issuer_hold(hold, SearchResultControlV1::Single(&control(s)), r)
            }
        }
    }
}
