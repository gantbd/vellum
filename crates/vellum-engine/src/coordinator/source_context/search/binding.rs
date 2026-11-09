//! Trusted host correspondence over original native owners. Borrowed, bounded
//! embedding captures: no resolver, registry, new payload pin or search store.
use super::*;
use std::cell::Cell;
use std::mem::size_of;

pub const LITERAL_EXACT_MATCHING_PROFILE_V1: &str = "literal-exact/1";

/// The host establishes this exact pairing, not a path/label-derived capability.
/// Captures borrow immutable host metadata; limits bound each call, not arbitrary
/// caller-held allocations or physical memory. No source-read grant is inferred.
pub struct TrustedLogicalSearchBindingV1<'a> {
    pub source: SearchReadContextV1<'a>,
    pub scope: &'a SourceReadScopeV1,
    pub expected: &'a DocumentVersion,
    pub reference: &'a LiteralSearchReferenceV1,
    pub generation: u64,
    pub binding_allowed: Option<bool>,
    pub correspondence_current: Option<bool>,
    pub max_label_bytes: usize,
    pub max_binding_bytes: usize,
}
pub struct LogicalSearchBindingV1<'a> {
    owner: Arc<()>,
    source: Issuer,
    scope: &'a SourceReadScopeV1,
    expected: &'a DocumentVersion,
    reference: &'a LiteralSearchReferenceV1,
    generation: u64,
    closed: Cell<bool>,
}
/// Original normalized request remains borrowed and immutable: reference,
/// profile and generation cannot disappear in a legacy SearchRequest conversion.
/// Dropping this handle does not retire its native result; use protected control.
pub struct BoundLiteralSearchResultV1<'a> {
    binding: &'a LogicalSearchBindingV1<'a>,
    request: &'a NormalizedLiteralSearchArgumentsV1<'a>,
    native: Option<SourceSearchResultId>,
}
pub enum BoundLiteralSearchProductV1 {
    Matches(SourceSearchReplyV1),
    Navigation(SourceSearchNavigationReplyV1),
}
pub struct BoundLiteralSearchReplyV1<'a> {
    pub reference: &'a LiteralSearchReferenceV1,
    pub matching_profile: &'a str,
    pub generation: u64,
    pub product: BoundLiteralSearchProductV1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundLiteralSearchErrorV1 {
    UnsupportedProfile,
    Native(SourceReadContextErrorV1),
}
impl From<Error> for BoundLiteralSearchErrorV1 {
    fn from(e: Error) -> Self {
        Self::Native(e)
    }
}
macro_rules! redacted { ($($t:ty),+) => { $(impl std::fmt::Debug for $t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<logical-search:redacted>")
    }
})+ }; }
redacted!(
    TrustedLogicalSearchBindingV1<'_>,
    LogicalSearchBindingV1<'_>,
    BoundLiteralSearchResultV1<'_>,
    BoundLiteralSearchProductV1,
    BoundLiteralSearchReplyV1<'_>
);

impl SourceCoordinator {
    pub fn bind_logical_search_source_v1<'a>(
        &self,
        f: TrustedLogicalSearchBindingV1<'a>,
        r: &mut Recorder,
    ) -> Result<LogicalSearchBindingV1<'a>, Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational()
                .map_err(super::super::admission_error)?;
            self.ensure_admission_unfenced()
                .map_err(super::super::admission_error)?;
            if f.binding_allowed != Some(true) || f.correspondence_current != Some(true) {
                return Err(Error::Unavailable);
            }
            self.source_search_funding()?;
            let source = self.own_search_issuer(f.source)?;
            let snapshot = self.search_snapshot(&source, f.scope)?;
            let reference = f.reference;
            let safe = |n: u64| n <= 9_007_199_254_740_991;
            if snapshot.version != f.expected
                || f.reference.revision != f.expected.revision
                || reference.authority != f.scope.context.authority
                || reference.authority.is_empty()
                || reference.document.is_empty()
                || !safe(reference.incarnation)
                || !safe(reference.revision)
                || !safe(f.generation)
            {
                return Err(Error::Unavailable);
            }
            let bytes = size_of::<LogicalSearchBindingV1<'_>>()
                .checked_add(reference.authority.len())
                .and_then(|n| n.checked_add(reference.document.len()))
                .and_then(|n| n.checked_add(scope_bytes(f.scope).ok()?))
                .ok_or(Error::Limit)?;
            if reference.authority.len() > f.max_label_bytes
                || reference.document.len() > f.max_label_bytes
                || bytes > f.max_binding_bytes
                || bytes > self.search_pool()?.limits.max_request_bytes
            {
                return Err(Error::Limit);
            }
            Ok(LogicalSearchBindingV1 {
                owner: self.namespace.clone(),
                source,
                scope: f.scope,
                expected: f.expected,
                reference,
                generation: f.generation,
                closed: Cell::new(false),
            })
        })();
        let observed = result
            .as_ref()
            .map(|_| ())
            .map_err(|e| BoundLiteralSearchErrorV1::Native(*e));
        Self::logical_search_observe(r, Boundary::LogicalSearchBinding, &observed, timer.finish());
        result
    }
    fn validate_logical_search_binding(
        &self,
        binding: &LogicalSearchBindingV1<'_>,
        q: &NormalizedLiteralSearchArgumentsV1<'_>,
        current: Option<bool>,
    ) -> Result<(), BoundLiteralSearchErrorV1> {
        // Original roles before current host facts; foreign input cannot close a
        // legitimate binding. Numeric incarnation labels are host correspondence,
        // never parsed into an engine-private DocumentId.
        if !Arc::ptr_eq(&self.namespace, &binding.owner)
            || self.own_search_issuer(q.source())? != binding.source
            || q.actor() != &binding.scope.context
            || q.reference() != binding.reference
            || q.generation() != binding.generation
            || binding.closed.get()
            || self
                .search_snapshot(&binding.source, binding.scope)?
                .version
                != binding.expected
        {
            return Err(Error::Unavailable.into());
        }
        if current == Some(false) {
            binding.closed.set(true);
        }
        if current != Some(true) {
            return Err(Error::Unavailable.into());
        }
        if q.matching_profile() != LITERAL_EXACT_MATCHING_PROFILE_V1 {
            return Err(BoundLiteralSearchErrorV1::UnsupportedProfile);
        }
        Ok(())
    }
    fn bound_search_request_matches(
        q: &NormalizedLiteralSearchArgumentsV1<'_>,
        native: &SearchRequest,
    ) -> bool {
        native.query == q.query()
            && matches!(&native.scope, SearchScope::Captured(rows) if rows == q.scopes())
    }
    /// Evaluates once at the existing native owner. No recipient result is
    /// released here; the original handle is available even if later release fails.
    pub fn query_bound_literal_search_v1<'a>(
        &mut self,
        binding: &'a LogicalSearchBindingV1<'a>,
        q: &'a NormalizedLiteralSearchArgumentsV1<'a>,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        correspondence_current: Option<bool>,
        r: &mut Recorder,
    ) -> Result<BoundLiteralSearchResultV1<'a>, BoundLiteralSearchErrorV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            if !binding.source.matches(sf) || sf.scope() != binding.scope {
                return Err(Error::Unavailable.into());
            }
            self.validate_logical_search_binding(binding, q, correspondence_current)?;
            if !Self::bound_search_request_matches(q, f.request) {
                return Err(Error::Unavailable.into());
            }
            // Reserve logical reply/header room before evaluation/issuance; the
            // existing result owner funds its capture, matches and native hold.
            let native = self.query_managed_search_result_with_overhead(
                q.source(),
                binding.scope,
                f.request,
                sf,
                f,
                size_of::<BoundLiteralSearchReplyV1<'_>>(),
                r,
            )?;
            Ok(BoundLiteralSearchResultV1 {
                binding,
                request: q,
                native: Some(native),
            })
        })();
        Self::logical_search_observe(
            r,
            Boundary::LogicalSearchEvaluation,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn release_bound_literal_search_v1<'a>(
        &mut self,
        result: &'a BoundLiteralSearchResultV1<'a>,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        correspondence_current: Option<bool>,
        r: &mut Recorder,
    ) -> Result<BoundLiteralSearchReplyV1<'a>, BoundLiteralSearchErrorV1> {
        let timer = r.begin_service_timing();
        let reply = (|| {
            let q = result.request;
            if !result.binding.source.matches(sf) || sf.scope() != result.binding.scope {
                return Err(Error::Unavailable.into());
            }
            self.validate_logical_search_binding(result.binding, q, correspondence_current)?;
            let id = result.native.as_ref().ok_or(Error::Unavailable)?;
            if !Self::bound_search_request_matches(q, f.request)
                || !self.managed_search_result_matches(
                    id,
                    q.source(),
                    result.binding.scope,
                    f.request,
                )
            {
                return Err(Error::Unavailable.into());
            }
            let scope = result.binding.scope;
            let product = if q.operation() == LiteralSearchOperationV1::LiteralMatches {
                BoundLiteralSearchProductV1::Matches(
                    self.release_managed_search_result(id, scope, sf, f, r)?,
                )
            } else {
                let nav = q.navigation().ok_or(Error::InvalidRequest)?;
                let current = nav
                    .current
                    .map(|index| self.managed_search_occurrence_result(id, index, scope, sf, f, r))
                    .transpose()?;
                BoundLiteralSearchProductV1::Navigation(self.navigate_managed_search_result(
                    id,
                    scope,
                    SourceSearchNavigationV1 {
                        direction: if q.operation() == LiteralSearchOperationV1::NextMatch {
                            SearchDirection::Next
                        } else {
                            SearchDirection::Previous
                        },
                        start: nav.start,
                        current: current.as_ref(),
                        wrap: nav.wrap,
                    },
                    sf,
                    f,
                    r,
                )?)
            };
            Ok(BoundLiteralSearchReplyV1 {
                reference: q.reference(),
                matching_profile: q.matching_profile(),
                generation: q.generation(),
                product,
            })
        })();
        Self::logical_search_observe(r, Boundary::LogicalSearchRelease, &reply, timer.finish());
        reply
    }
    /// Protected original-owner retirement needs no current content grant or
    /// live correspondence. A failed native retirement leaves the handle intact.
    pub fn retire_bound_literal_search_v1(
        &mut self,
        result: &mut BoundLiteralSearchResultV1<'_>,
        control: SearchResultControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let timer = r.begin_service_timing();
        let retired = (|| {
            let id = result.native.as_ref()?;
            if !Arc::ptr_eq(&self.namespace, &result.binding.owner) {
                return None;
            }
            if !self.retire_managed_search_result(id, control, r) {
                return None;
            }
            result.native = None;
            Some(())
        })()
        .is_some();
        let observed = if retired {
            Ok(())
        } else {
            Err(BoundLiteralSearchErrorV1::Native(Error::Unavailable))
        };
        Self::logical_search_observe(r, Boundary::LogicalSearchControl, &observed, timer.finish());
        retired
    }
    /// Invalidates only this borrowed correspondence capture, not its source or
    /// retained results. No new payload owner/reservation exists to refund.
    pub fn retire_logical_search_binding_v1(
        &self,
        binding: &LogicalSearchBindingV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let timer = r.begin_service_timing();
        let original = Arc::ptr_eq(&self.namespace, &binding.owner);
        if original {
            binding.closed.set(true);
        }
        let result = if original {
            Ok(())
        } else {
            Err(Error::Unavailable)
        };
        let observed = result.map_err(BoundLiteralSearchErrorV1::Native);
        Self::logical_search_observe(r, Boundary::LogicalSearchControl, &observed, timer.finish());
        original
    }
    fn logical_search_observe<T>(
        r: &mut Recorder,
        boundary: Boundary,
        result: &Result<T, BoundLiteralSearchErrorV1>,
        service_time: Option<std::time::Duration>,
    ) {
        r.record(Observation {
            boundary,
            disposition: match result {
                Ok(_) => Disposition::Completed,
                Err(BoundLiteralSearchErrorV1::Native(Error::Faulted)) => Disposition::Unknown,
                Err(_) => Disposition::Refused,
            },
            service_time,
        });
    }
}
