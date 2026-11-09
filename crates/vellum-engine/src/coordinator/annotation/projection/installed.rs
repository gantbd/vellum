//! Installed projection ownership, independent of preparation-pool lifetime.
use super::*;
mod queries;
mod retained;
mod search_hold;
mod shutdown_inventory;
#[cfg(test)]
pub(in crate::coordinator) use queries::tests::native_facade_single_fixture;
pub use queries::{
    AnnotationContentV1, AnnotationContinuationId, AnnotationLocationV1, AnnotationQueryReplyV1,
    AnnotationQueryV1, AnnotationRecordId, AnnotationRecordV1,
};
pub(in crate::coordinator) use search_hold::AnnotationSourceHoldId;
#[cfg(test)]
mod tests;
use crate::coordinates::{BytePosition, CoordinateBudget, LineSpan, SourceCoordinates};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnnotationContextLimitsV1 {
    pub max_contexts: usize,
    /// Live contexts plus record/continuation selectors; excludes returned copies.
    pub max_selectors: usize,
    /// Independently capped live F05 holds, funded before configuration succeeds.
    pub max_holds: usize,
    pub max_page_rows: usize,
    pub max_row_bytes: usize,
    pub max_selector_bytes: usize,
    /// Shared, nonrefilling context, F03 selector and F05 hold issuance.
    pub max_issued: usize,
    pub max_rows: usize,
    pub max_capture_bytes: usize,
    pub max_context_bytes: usize,
    pub max_request_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_label_bytes: usize,
    pub max_pool_bytes: usize,
}
impl AnnotationContextLimitsV1 {
    pub fn required_pool_bytes(self) -> Result<usize, Error> {
        let mut n = std::mem::size_of::<ContextPool>();
        for (count, size) in [
            (self.max_issued, std::mem::size_of::<Option<Context>>()),
            (
                self.max_issued,
                std::mem::size_of::<Option<search_hold::Hold>>(),
            ),
            (
                self.max_issued,
                std::mem::size_of::<Option<queries::Selector>>(),
            ),
            (
                2,
                self.max_page_rows
                    .checked_add(1)
                    .and_then(|n| n.checked_mul(std::mem::size_of::<queries::Selector>()))
                    .ok_or(Error::Limit)?,
            ),
            (2, self.max_request_bytes),
            (2, self.max_capture_bytes),
            (2, self.max_context_bytes),
            (2, self.max_reply_bytes),
            (self.max_contexts, self.max_capture_bytes),
            (self.max_contexts, self.max_context_bytes),
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
#[derive(Clone)]
pub struct AnnotationReadContextId {
    owner: Arc<()>,
    slot: usize,
}
impl PartialEq for AnnotationReadContextId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for AnnotationReadContextId {}
impl std::fmt::Debug for AnnotationReadContextId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationReadContextId(<opaque>)")
    }
}
pub struct AnnotationContextAcquisitionV1 {
    pub context: AnnotationReadContextId,
    pub classes: AnnotationProjectionClassesV1,
}
pub struct AnnotationSourceReplyV1 {
    pub context: AnnotationReadContextId,
    pub content: SourceContextContentV1,
}
macro_rules! redacted {
    ($name:ident) => {
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(concat!(stringify!($name), "(<redacted>)"))
            }
        }
    };
}
redacted!(AnnotationContextAcquisitionV1);
redacted!(AnnotationSourceReplyV1);
/// Fresh authority for this exact original installed whole projection.
pub struct TrustedAnnotationContextFactsV1<'a> {
    pub context: &'a AnnotationReadContextId,
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
struct Context {
    scope: SourceReadScopeV1,
    value: Projection,
    stopped: bool,
    holds: usize,
}
pub(in crate::coordinator) struct ContextPool {
    authority: String,
    engine: String,
    limits: AnnotationContextLimitsV1,
    reservation: retention::Reservation,
    charge: usize,
    rows: Vec<Option<Context>>,
    selectors: Vec<Option<queries::Selector>>,
    holds: Vec<Option<search_hold::Hold>>,
}
impl ContextPool {
    fn issued(&self) -> Result<usize, Error> {
        self.rows
            .len()
            .checked_add(self.selectors.len())
            .and_then(|n| n.checked_add(self.holds.len()))
            .ok_or(Error::Faulted)
    }
    fn selector_count(&self) -> usize {
        self.rows.iter().flatten().count() + self.selectors.iter().flatten().count()
    }
}
#[derive(Default)]
pub(in crate::coordinator) enum InstalledState {
    #[default]
    Unconfigured,
    Active(Box<ContextPool>),
    Closed {
        issued: usize,
    },
}
impl std::fmt::Debug for InstalledState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationContextState(<redacted>)")
    }
}
impl InstalledState {
    pub(in crate::coordinator) fn document_pins(
        &self,
        document: &DocumentId,
    ) -> Result<(usize, usize), CoordinatorError> {
        match self {
            Self::Unconfigured | Self::Closed { .. } => Ok((0, 0)),
            Self::Active(p) => p.document_pins(document),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnnotationContextUsageV1 {
    pub holds: usize,
    pub selectors: usize,
    pub contexts: usize,
    pub stopped: usize,
    pub issued_lifetime: usize,
    pub reserved_pool_bytes: usize,
    pub projected_rows: usize,
    pub source_bytes: usize,
    pub body_bytes: usize,
    pub closed: bool,
}
fn valid_scope(s: &SourceReadScopeV1, p: &ContextPool) -> bool {
    s.context.authority == p.authority
        && s.context.engine == p.engine
        && !s.policy.is_empty()
        && s.policy.len() <= p.limits.max_label_bytes
        && settings::basis::labels(&s.context)
            .into_iter()
            .all(|v| !v.is_empty() && v.len() <= p.limits.max_label_bytes)
}
enum Resolved {
    Range(BytePosition, BytePosition),
    Line(usize, LineSpan),
}
impl Resolved {
    fn resolve(source: &Utf8Source, q: SourceContextQueryV1) -> Result<Self, Error> {
        let c = SourceCoordinates::new(source);
        match q {
            SourceContextQueryV1::Range { start, end } => {
                c.validate_range(start, end)
                    .map_err(|_| Error::InvalidRequest)?;
                Ok(Self::Range(start, end))
            }
            SourceContextQueryV1::Line { index } => Ok(Self::Line(
                index,
                c.line_span(
                    index,
                    CoordinateBudget {
                        max_scan_bytes: source.text().len(),
                    },
                )
                .map_err(|_| Error::InvalidRequest)?,
            )),
        }
    }
    fn payload(&self) -> usize {
        match self {
            Self::Range(a, b) => b.0 - a.0,
            Self::Line(_, s) => s.end.0 - s.start.0,
        }
    }
}
impl SourceCoordinator {
    fn annotation_context_pool(&self) -> Result<&ContextPool, Error> {
        match &self.annotation_contexts {
            InstalledState::Active(p) => Ok(p),
            _ => Err(Error::Unavailable),
        }
    }
    fn annotation_context_row(
        &self,
        id: &AnnotationReadContextId,
        s: &SourceReadScopeV1,
    ) -> Result<&Context, Error> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return Err(Error::Unavailable);
        }
        let row = self
            .annotation_context_pool()?
            .rows
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)?;
        if row.scope != *s {
            return Err(Error::Unavailable);
        }
        Ok(row)
    }
    pub fn configure_annotation_contexts(
        &mut self,
        limits: AnnotationContextLimitsV1,
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
            match &self.annotation_contexts {
                InstalledState::Active(p) => {
                    self.retention.validate(&p.reservation).map_err(parent)?;
                    return if p.limits == limits {
                        Ok(false)
                    } else {
                        Err(Error::Unavailable)
                    };
                }
                InstalledState::Closed { .. } => return Err(Error::Unavailable),
                InstalledState::Unconfigured => {}
            }
            self.ensure_admission_unfenced()
                .map_err(source_context::admission_error)?;
            let preparation = self.annotation_preparation_pool()?;
            self.retention
                .validate(&preparation.reservation)
                .map_err(parent)?;
            if preparation.authority.len() > limits.max_label_bytes
                || preparation.engine.len() > limits.max_label_bytes
            {
                return Err(Error::Limit);
            }
            let labels = preparation
                .authority
                .len()
                .checked_add(preparation.engine.len())
                .ok_or(Error::Limit)?;
            if std::mem::size_of::<AnnotationContextLimitsV1>()
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
            let authority = owned(&preparation.authority)?;
            let engine = owned(&preparation.engine)?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(limits.max_issued)
                .map_err(|_| Error::Capacity)?;
            let mut selectors = Vec::new();
            selectors
                .try_reserve_exact(limits.max_issued)
                .map_err(|_| Error::Capacity)?;
            let mut holds = Vec::new();
            holds
                .try_reserve_exact(limits.max_issued)
                .map_err(|_| Error::Capacity)?;
            let reservation = self.retention.reserve(charge).map_err(parent)?;
            self.annotation_contexts = InstalledState::Active(Box::new(ContextPool {
                authority,
                engine,
                limits,
                reservation,
                charge,
                rows,
                selectors,
                holds,
            }));
            Ok(true)
        })();
        self.finish_annotation_preparation(Boundary::AnnotationContextConfiguration, &result, r);
        result
    }
    pub fn annotation_context_usage(&self) -> AnnotationContextUsageV1 {
        let mut u = AnnotationContextUsageV1::default();
        match &self.annotation_contexts {
            InstalledState::Unconfigured => {}
            InstalledState::Closed { issued } => {
                u.closed = true;
                u.issued_lifetime = *issued;
            }
            InstalledState::Active(p) => {
                u.reserved_pool_bytes = p.charge;
                u.issued_lifetime = p.rows.len() + p.selectors.len() + p.holds.len();
                u.holds = p.holds.iter().flatten().count();
                u.selectors = p.selector_count();
                for row in p.rows.iter().flatten() {
                    u.contexts += 1;
                    u.stopped += usize::from(row.stopped);
                    u.projected_rows += row.value.rows.len();
                    if let Some((s, _, _)) = &row.value.source {
                        u.source_bytes += s.text().len();
                    }
                    for row in &row.value.rows {
                        u.body_bytes += row.body.as_ref().map_or(0, |v| v.0.len());
                    }
                }
            }
        }
        u
    }
    pub fn install_annotation_context(
        &mut self,
        id: &PreparedAnnotationProjectionId,
        s: &SourceReadScopeV1,
        f: &TrustedPreparedAnnotationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<AnnotationContextAcquisitionV1> {
        self.install_annotation_context_result(id, s, f, r).ok()
    }
    #[allow(clippy::too_many_arguments)]
    pub(in crate::coordinator) fn install_annotation_context_result(
        &mut self,
        id: &PreparedAnnotationProjectionId,
        s: &SourceReadScopeV1,
        f: &TrustedPreparedAnnotationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<AnnotationContextAcquisitionV1, Error> {
        let result = (|| {
            let p = self.annotation_context_pool()?;
            if !valid_scope(s, p) || s != f.scope || id != f.projection {
                return Err(Error::Unavailable);
            }
            if scope_bytes(s)?
                .checked_add(std::mem::size_of::<PreparedAnnotationProjectionId>())
                .is_none_or(|n| n > p.limits.max_request_bytes)
            {
                return Err(Error::Limit);
            }
            self.prepared_annotation_cut(id, s)?;
            if !self.validate_prepared_annotation_projection(id, s, f, r) {
                return Err(if self.faulted {
                    Error::Faulted
                } else {
                    Error::Unavailable
                });
            }
            self.ensure_admission_unfenced()
                .map_err(source_context::admission_error)?;
            let cut = self.prepared_annotation_cut(id, s)?;
            let p = self.annotation_context_pool()?;
            self.retention.validate(&p.reservation).map_err(parent)?;
            if p.issued()? >= p.limits.max_issued
                || p.selector_count() >= p.limits.max_selectors
                || p.rows.iter().flatten().count() >= p.limits.max_contexts
            {
                return Err(Error::Limit);
            }
            if cut.value.bytes()? > p.limits.max_capture_bytes
                || scope_bytes(s)? > p.limits.max_context_bytes
                || cut.value.rows.len() > p.limits.max_rows
                || std::mem::size_of::<AnnotationContextAcquisitionV1>() > p.limits.max_reply_bytes
                || std::mem::size_of::<AnnotationReadContextId>() > p.limits.max_selector_bytes
            {
                return Err(Error::Limit);
            }
            if p.rows.capacity() < p.limits.max_issued {
                return Err(Error::Faulted);
            }
            let expected_slot = p.rows.len();
            let expected_classes = cut.value.classes;
            #[allow(unused_mut)]
            let mut reply = AnnotationContextAcquisitionV1 {
                context: AnnotationReadContextId {
                    owner: self.namespace.clone(),
                    slot: p.rows.len(),
                },
                classes: cut.value.classes,
            };
            #[cfg(test)]
            {
                use crate::coordinator::source_query::result_contract::AcquisitionResultFault as F;
                match self.acquisition_result_fault {
                    Some(F::Context) => reply.context.slot = usize::MAX,
                    Some(F::Classes) => {
                        reply.classes = if expected_classes == AnnotationProjectionClassesV1::Source
                        {
                            AnnotationProjectionClassesV1::Annotations
                        } else {
                            AnnotationProjectionClassesV1::Source
                        }
                    }
                    _ => (),
                }
            }
            let accepted = Arc::ptr_eq(&reply.context.owner, &self.namespace)
                && reply.context.slot == expected_slot
                && reply.classes == expected_classes;
            if !self.finish_read_result(
                accepted,
                ReadResultInvariantV1::NativeReadAcquisitionResultInvariant,
                Boundary::ReadAcquisitionResultValidation,
                r,
            ) {
                return Err(Error::Faulted);
            }
            // All fallible work precedes these local, nonallocating owner moves.
            self.faulted = true;
            #[cfg(test)]
            assert!(
                !self.annotation_context_install_fault,
                "injected annotation context staging fault"
            );
            let (State::Active(preparation), InstalledState::Active(installed)) = (
                &mut self.annotation_preparations,
                &mut self.annotation_contexts,
            ) else {
                return Err(Error::Faulted);
            };
            let cut = preparation.cuts[id.slot].take().ok_or(Error::Faulted)?;
            installed.rows.push(Some(Context {
                scope: cut.scope,
                value: cut.value,
                stopped: false,
                holds: 0,
            }));
            self.faulted = false;
            Ok(reply)
        })();
        self.finish_annotation_preparation(Boundary::AnnotationContextInstallation, &result, r);
        result
    }
    pub(in crate::coordinator) fn check_annotation_context(
        &mut self,
        id: &AnnotationReadContextId,
        s: &SourceReadScopeV1,
        f: &TrustedAnnotationContextFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let p = self.annotation_context_pool()?;
        if !valid_scope(s, p) || s != f.scope || id != f.context {
            return Err(Error::Unavailable);
        }
        let row = self.annotation_context_row(id, s)?;
        if row.stopped {
            return Err(Error::Unavailable);
        }
        if !row.value.valid(s, &self.namespace) {
            return Err(Error::Faulted);
        }
        let source = row.value.classes.includes_source();
        let annotations = row.value.classes.includes_annotations();
        let gates = [
            f.authorized,
            f.scope_allowed,
            f.continuous,
            f.live,
            f.not_stopped,
            f.projection_current,
            f.retained,
        ];
        if self.faulted
            || self.document(&s.document).is_err()
            || gates.contains(&Some(false))
            || source && f.source_readable == Some(false)
            || annotations && f.annotation_readable == Some(false)
        {
            let InstalledState::Active(p) = &mut self.annotation_contexts else {
                return Err(Error::Faulted);
            };
            p.rows[id.slot].as_mut().ok_or(Error::Faulted)?.stopped = true;
            self.finish_annotation_preparation(
                Boundary::AnnotationContextInvalidation,
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
            || source && f.source_readable != Some(true)
            || annotations && f.annotation_readable != Some(true)
        {
            return Err(Error::Unavailable);
        }
        Ok(())
    }
    pub fn query_annotation_context_source(
        &mut self,
        id: &AnnotationReadContextId,
        s: &SourceReadScopeV1,
        q: SourceContextQueryV1,
        f: &TrustedAnnotationContextFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<AnnotationSourceReplyV1> {
        self.query_annotation_context_source_result(id, s, q, f, r)
            .ok()
    }
    #[allow(clippy::too_many_arguments)]
    pub(in crate::coordinator) fn query_annotation_context_source_result(
        &mut self,
        id: &AnnotationReadContextId,
        s: &SourceReadScopeV1,
        q: SourceContextQueryV1,
        f: &TrustedAnnotationContextFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<AnnotationSourceReplyV1, Error> {
        let result = (|| {
            let p = self.annotation_context_pool()?;
            if !valid_scope(s, p) || id != f.context || s != f.scope {
                return Err(Error::Unavailable);
            }
            let row = self.annotation_context_row(id, s)?;
            if row.stopped {
                return Err(Error::Unavailable);
            }
            let (source, _, profile) = row.value.source.as_ref().ok_or(Error::Unavailable)?;
            if scope_bytes(s)?
                .checked_add(std::mem::size_of::<AnnotationReadContextId>())
                .and_then(|n| n.checked_add(std::mem::size_of::<SourceContextQueryV1>()))
                .is_none_or(|n| n > p.limits.max_request_bytes.min(profile.max_request_bytes))
            {
                return Err(Error::Limit);
            }
            let resolved = Resolved::resolve(source, q)?;
            self.check_annotation_context(id, s, f, r)?;
            let p = self.annotation_context_pool()?;
            let (source, version, profile) = self
                .annotation_context_row(id, s)?
                .value
                .source
                .as_ref()
                .ok_or(Error::Faulted)?;
            if std::mem::size_of::<AnnotationSourceReplyV1>()
                .checked_add(resolved.payload())
                .is_none_or(|n| n > p.limits.max_reply_bytes.min(profile.max_reply_bytes))
            {
                return Err(Error::Limit);
            }
            let expected = match &resolved {
                Resolved::Range(a, b) => source_context::Resolved::Range(*a, *b),
                Resolved::Line(index, span) => source_context::Resolved::Line(*index, *span),
            };
            let content = match resolved {
                Resolved::Range(a, b) => SourceContextContentV1::Range(
                    SourceRangeReply::from_source(source, version, a, b)
                        .map_err(|_| Error::Capacity)?,
                ),
                Resolved::Line(index, span) => {
                    SourceCoordinates::new(source)
                        .line_span(
                            index,
                            CoordinateBudget {
                                max_scan_bytes: profile.max_scan_bytes,
                            },
                        )
                        .map_err(|_| Error::Limit)?;
                    SourceContextContentV1::Line(
                        SourceLineReply::from_source(source, version, index, span)
                            .map_err(|_| Error::Capacity)?,
                    )
                }
            };
            let version = version.clone();
            #[allow(unused_mut)]
            let mut reply = AnnotationSourceReplyV1 {
                context: id.clone(),
                content,
            };
            #[cfg(test)]
            {
                crate::coordinator::source_query::result_contract::corrupt(
                    &mut reply.content,
                    self.source_read_result_fault,
                );
                if matches!(
                    self.source_read_result_fault,
                    Some(
                        crate::coordinator::source_query::result_contract::SourceResultFault::Alias
                    )
                ) {
                    reply.context.slot = usize::MAX;
                }
            }
            self.guard_source_result(expected, &version, &reply.content, reply.context == *id, r)?;
            Ok(reply)
        })();
        self.finish_annotation_preparation(Boundary::AnnotationContextSourceQuery, &result, r);
        result
    }
    pub fn stop_annotation_context(
        &mut self,
        id: &AnnotationReadContextId,
        f: &SourceReadContextControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        self.stop_annotation_context_result(id, f, r).is_ok()
    }
    pub(in crate::coordinator) fn stop_annotation_context_result(
        &mut self,
        id: &AnnotationReadContextId,
        f: &SourceReadContextControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = (|| {
            if f.owner_control_allowed != Some(true) {
                return Err(Error::Unavailable);
            }
            self.annotation_context_row(id, f.scope)?;
            let InstalledState::Active(p) = &mut self.annotation_contexts else {
                return Err(Error::Faulted);
            };
            p.rows[id.slot].as_mut().ok_or(Error::Faulted)?.stopped = true;
            Ok(())
        })();
        self.finish_annotation_preparation(Boundary::AnnotationContextStop, &result, r);
        result
    }
    pub fn retire_annotation_context(
        &mut self,
        id: &AnnotationReadContextId,
        f: &SourceReadContextControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let result = (|| {
            if !discharge(f) {
                return Err(Error::Unavailable);
            }
            let row = self.annotation_context_row(id, f.scope)?;
            if !row.stopped {
                return Err(Error::Unavailable);
            }
            let actual = self.annotation_context_pool()?.actual_holds(id.slot)?;
            if actual != row.holds {
                return Err(Error::Faulted);
            }
            if actual != 0 {
                return Err(Error::Unavailable);
            }
            self.retention
                .validate(&self.annotation_context_pool()?.reservation)
                .map_err(parent)?;
            let InstalledState::Active(p) = &mut self.annotation_contexts else {
                return Err(Error::Faulted);
            };
            p.rows[id.slot] = None;
            for selector in &mut p.selectors {
                if selector.as_ref().is_some_and(|v| v.context() == id.slot) {
                    *selector = None;
                }
            }
            Ok(())
        })();
        self.finish_annotation_preparation(Boundary::AnnotationContextRetirement, &result, r);
        result.is_ok()
    }
    pub fn retire_annotation_contexts(
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
            let InstalledState::Active(p) = &mut self.annotation_contexts else {
                return Ok(false);
            };
            if p.rows.iter().any(Option::is_some) {
                return Err(Error::Unavailable);
            }
            if p.selectors.iter().any(Option::is_some) {
                return Err(Error::Faulted);
            }
            if p.holds.iter().any(Option::is_some) {
                return Err(Error::Faulted);
            }
            let issued = p.issued()?;
            self.retention.release(&mut p.reservation).map_err(parent)?;
            self.annotation_contexts = InstalledState::Closed { issued };
            Ok(true)
        })();
        self.finish_annotation_preparation(Boundary::AnnotationContextPoolRetirement, &result, r);
        result
    }
}
