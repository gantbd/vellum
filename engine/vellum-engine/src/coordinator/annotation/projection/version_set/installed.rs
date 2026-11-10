//! Atomic ownership of prepared coherent sets and their scoped member aliases.
use super::*;
mod acquisition;
mod retained;
mod search_hold;
mod shutdown_inventory;
mod source;
pub(in crate::coordinator) use search_hold::VersionSetSourceHoldId;
pub use source::VersionSetSourceReplyV1;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VersionSetContextLimitsV1 {
    /// Independently capped live whole-parent search holds.
    pub max_holds: usize,
    /// Positive page requests cannot exceed this funded descriptor-stage ceiling.
    pub max_page_rows: usize,
    /// Complete native record header plus owned body payload.
    pub max_row_bytes: usize,
    pub max_contexts: usize,
    pub max_members: usize,
    /// Live parent, member and query selectors together.
    pub max_selectors: usize,
    /// Shared nonrefilling parent, member, query selector and hold issuance.
    pub max_issued: usize,
    pub max_rows: usize,
    pub max_capture_bytes: usize,
    pub max_context_bytes: usize,
    pub max_request_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_selector_bytes: usize,
    pub max_label_bytes: usize,
    pub max_pool_bytes: usize,
}
impl VersionSetContextLimitsV1 {
    pub fn required_pool_bytes(self) -> Result<usize, Error> {
        let mut n = std::mem::size_of::<ContextPool>();
        for (count, size) in [
            (self.max_issued, std::mem::size_of::<Option<Context>>()),
            (self.max_issued, std::mem::size_of::<Option<Alias>>()),
            (
                self.max_issued,
                std::mem::size_of::<Option<search_hold::Hold>>(),
            ),
            (
                self.max_issued,
                std::mem::size_of::<Option<super::super::reader::Selector>>(),
            ),
            (
                2,
                self.max_page_rows
                    .checked_add(1)
                    .and_then(|n| {
                        n.checked_mul(std::mem::size_of::<super::super::reader::Selector>())
                    })
                    .ok_or(Error::Limit)?,
            ),
            (
                2,
                self.max_members
                    .checked_mul(std::mem::size_of::<Option<Alias>>())
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
macro_rules! opaque {
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
opaque!(VersionSetContextId);
opaque!(VersionSetMemberId);
mod annotations;
#[cfg(test)]
pub(in crate::coordinator) use annotations::tests::{
    Fixture as AnnotationFacadeFixture, native_facade_version_fixture,
};
pub use annotations::{
    VersionSetAnnotationContinuationId, VersionSetAnnotationQueryV1, VersionSetAnnotationRecordId,
    VersionSetAnnotationRecordV1, VersionSetAnnotationReplyV1,
};
pub struct VersionSetMemberAcquisitionV1 {
    pub document: DocumentId,
    pub snapshot: VersionSetMemberId,
    pub data: AnnotationProjectionClassesV1,
}
pub struct VersionSetAcquisitionV1 {
    pub context: VersionSetContextId,
    pub participants: Vec<VersionSetMemberAcquisitionV1>,
}
impl std::fmt::Debug for VersionSetAcquisitionV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("VersionSetAcquisitionV1(<redacted>)")
    }
}
impl std::fmt::Debug for VersionSetMemberAcquisitionV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("VersionSetMemberAcquisitionV1(<redacted>)")
    }
}
#[derive(Clone, Copy)]
pub enum VersionSetControlTargetV1<'a> {
    Context(&'a VersionSetContextId),
    Member(&'a VersionSetMemberId),
}
pub struct TrustedVersionSetContextFactsV1<'a> {
    pub context: &'a VersionSetContextId,
    pub members: &'a [TrustedVersionSetMemberFactsV1<'a>],
}
struct Context {
    value: PreparedSet,
    stopped: bool,
    holds: usize,
}
#[derive(Clone, Copy)]
struct Alias {
    parent: usize,
    index: usize,
}
pub(in crate::coordinator) struct ContextPool {
    authority: String,
    engine: String,
    limits: VersionSetContextLimitsV1,
    reservation: retention::Reservation,
    charge: usize,
    rows: Vec<Option<Context>>,
    aliases: Vec<Option<Alias>>,
    selectors: Vec<Option<super::super::reader::Selector>>,
    holds: Vec<Option<search_hold::Hold>>,
}
impl ContextPool {
    fn issued(&self) -> Result<usize, Error> {
        self.rows
            .len()
            .checked_add(self.aliases.len())
            .and_then(|n| n.checked_add(self.selectors.len()))
            .and_then(|n| n.checked_add(self.holds.len()))
            .ok_or(Error::Faulted)
    }
    fn live_selectors(&self) -> usize {
        self.rows.iter().flatten().count()
            + self.aliases.iter().flatten().count()
            + self.selectors.iter().flatten().count()
    }
    fn selector_slot(&self, key: super::super::reader::Selector) -> Option<usize> {
        self.selectors.iter().position(|v| v.as_ref() == Some(&key))
    }
}
#[derive(Default)]
pub(in crate::coordinator) enum VersionSetContextState {
    #[default]
    Unconfigured,
    Active(Box<ContextPool>),
    Closed {
        issued: usize,
    },
}
impl std::fmt::Debug for VersionSetContextState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("VersionSetContextState(<redacted>)")
    }
}
impl VersionSetContextState {
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
pub struct VersionSetContextUsageV1 {
    pub holds: usize,
    pub query_selectors: usize,
    pub contexts: usize,
    pub aliases: usize,
    pub stopped: usize,
    pub issued_lifetime: usize,
    pub reserved_pool_bytes: usize,
    pub members: usize,
    pub source_bytes: usize,
    pub body_bytes: usize,
    pub projected_rows: usize,
    pub closed: bool,
}
impl PreparedSet {
    fn capture_bytes(&self) -> Result<usize, Error> {
        self.members
            .iter()
            .try_fold(std::mem::size_of::<Self>(), |n, m| {
                n.checked_add(std::mem::size_of::<Member>())
                    .and_then(|n| n.checked_add(m.value.bytes().ok()?))
            })
            .ok_or(Error::Limit)
    }
    fn scope_bytes(&self) -> Result<usize, Error> {
        self.members
            .iter()
            .try_fold(0usize, |n, m| n.checked_add(scope_bytes(&m.scope).ok()?))
            .ok_or(Error::Limit)
    }
    fn facts_request_bytes(&self) -> Result<usize, Error> {
        self.facts_request_bytes_for(std::mem::size_of::<PreparedVersionSetId>())
    }
    fn context_facts_request_bytes(&self) -> Result<usize, Error> {
        self.facts_request_bytes_for(std::mem::size_of::<VersionSetContextId>())
    }
    fn facts_request_bytes_for(&self, selector_bytes: usize) -> Result<usize, Error> {
        self.members
            .iter()
            .try_fold(selector_bytes, |n, m| {
                n.checked_add(scope_bytes(&m.scope).ok()?).and_then(|n| {
                    n.checked_add(std::mem::size_of::<TrustedVersionSetMemberFactsV1>())
                })
            })
            .ok_or(Error::Limit)
    }
    fn stop_controls_match(&self, controls: &[SourceReadContextControlV1<'_>]) -> bool {
        controls.len() == self.members.len()
            && controls
                .iter()
                .zip(&self.members)
                .all(|(f, m)| f.scope == &m.scope && f.owner_control_allowed == Some(true))
    }
    fn controls_match(&self, controls: &[SourceReadContextControlV1<'_>]) -> bool {
        self.stop_controls_match(controls) && controls.iter().all(discharge)
    }
}
impl SourceCoordinator {
    fn version_context_pool(&self) -> Result<&ContextPool, Error> {
        match &self.version_set_contexts {
            VersionSetContextState::Active(p) => Ok(p),
            _ => Err(Error::Unavailable),
        }
    }
    fn version_parent_slot(&self, target: VersionSetControlTargetV1<'_>) -> Result<usize, Error> {
        let p = self.version_context_pool()?;
        let slot = match target {
            VersionSetControlTargetV1::Context(id) => {
                if !Arc::ptr_eq(&id.owner, &self.namespace) {
                    return Err(Error::Unavailable);
                }
                id.slot
            }
            VersionSetControlTargetV1::Member(id) => {
                if !Arc::ptr_eq(&id.owner, &self.namespace) {
                    return Err(Error::Unavailable);
                }
                let a = p
                    .aliases
                    .get(id.slot)
                    .and_then(Option::as_ref)
                    .ok_or(Error::Unavailable)?;
                let row = p
                    .rows
                    .get(a.parent)
                    .and_then(Option::as_ref)
                    .ok_or(Error::Faulted)?;
                if a.index >= row.value.members.len() {
                    return Err(Error::Faulted);
                }
                a.parent
            }
        };
        p.rows
            .get(slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)?;
        Ok(slot)
    }
    fn validate_version_aliases(&self, slot: usize) -> Result<(), Error> {
        let p = self.version_context_pool()?;
        let row = p
            .rows
            .get(slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)?;
        for index in 0..row.value.members.len() {
            if p.aliases
                .iter()
                .flatten()
                .filter(|a| a.parent == slot && a.index == index)
                .count()
                != 1
            {
                return Err(Error::Faulted);
            }
        }
        if p.aliases
            .iter()
            .flatten()
            .any(|a| a.parent == slot && a.index >= row.value.members.len())
        {
            return Err(Error::Faulted);
        }
        for selector in p.selectors.iter().flatten() {
            let alias = p
                .aliases
                .get(selector.context())
                .and_then(Option::as_ref)
                .ok_or(Error::Faulted)?;
            let parent = p
                .rows
                .get(alias.parent)
                .and_then(Option::as_ref)
                .ok_or(Error::Faulted)?;
            let member = parent
                .value
                .members
                .get(alias.index)
                .ok_or(Error::Faulted)?;
            if !member.value.classes.includes_annotations() {
                return Err(Error::Faulted);
            }
            match selector {
                super::super::reader::Selector::Record { index, .. }
                    if *index < member.value.rows.len() => {}
                super::super::reader::Selector::Continuation { limit, next, .. }
                    if *limit > 0
                        && *limit <= p.limits.max_page_rows
                        && *next > 0
                        && *next < member.value.rows.len() => {}
                _ => return Err(Error::Faulted),
            }
        }
        Ok(())
    }
    pub fn configure_version_set_contexts(
        &mut self,
        limits: VersionSetContextLimitsV1,
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
            match &self.version_set_contexts {
                VersionSetContextState::Closed { .. } => return Err(Error::Unavailable),
                VersionSetContextState::Active(p) => {
                    self.retention.validate(&p.reservation).map_err(parent)?;
                    return if p.limits == limits {
                        Ok(false)
                    } else {
                        Err(Error::Unavailable)
                    };
                }
                VersionSetContextState::Unconfigured => {}
            }
            self.ensure_admission_unfenced()
                .map_err(source_context::admission_error)?;
            let prep = self.version_preparation_pool()?;
            self.retention.validate(&prep.reservation).map_err(parent)?;
            let labels = prep
                .authority
                .len()
                .checked_add(prep.engine.len())
                .ok_or(Error::Limit)?;
            if std::mem::size_of::<VersionSetContextLimitsV1>()
                .checked_add(labels)
                .is_none_or(|n| n > limits.max_request_bytes)
                || prep.authority.len() > limits.max_label_bytes
                || prep.engine.len() > limits.max_label_bytes
            {
                return Err(Error::Limit);
            }
            let charge = limits
                .required_pool_bytes()?
                .checked_add(labels)
                .filter(|n| *n <= limits.max_pool_bytes)
                .ok_or(Error::Limit)?;
            self.retention.check(charge).map_err(parent)?;
            let authority = owned(&prep.authority)?;
            let engine = owned(&prep.engine)?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(limits.max_issued)
                .map_err(|_| Error::Capacity)?;
            let mut aliases = Vec::new();
            aliases
                .try_reserve_exact(limits.max_issued)
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
            self.version_set_contexts = VersionSetContextState::Active(Box::new(ContextPool {
                authority,
                engine,
                limits,
                reservation,
                charge,
                rows,
                aliases,
                selectors,
                holds,
            }));
            Ok(true)
        })();
        self.finish_annotation_preparation(Boundary::VersionSetContextConfiguration, &result, r);
        result
    }
    pub fn version_set_context_usage(&self) -> VersionSetContextUsageV1 {
        let mut u = VersionSetContextUsageV1::default();
        match &self.version_set_contexts {
            VersionSetContextState::Unconfigured => {}
            VersionSetContextState::Closed { issued } => {
                u.closed = true;
                u.issued_lifetime = *issued;
            }
            VersionSetContextState::Active(p) => {
                u.issued_lifetime =
                    p.rows.len() + p.aliases.len() + p.selectors.len() + p.holds.len();
                u.holds = p.holds.iter().flatten().count();
                u.query_selectors = p.selectors.iter().flatten().count();
                u.reserved_pool_bytes = p.charge;
                u.aliases = p.aliases.iter().flatten().count();
                for row in p.rows.iter().flatten() {
                    u.contexts += 1;
                    u.stopped += usize::from(row.stopped);
                    for m in &row.value.members {
                        u.members += 1;
                        u.projected_rows += m.value.rows.len();
                        if let Some((s, _, _)) = &m.value.source {
                            u.source_bytes += s.text().len();
                        }
                        for r in &m.value.rows {
                            u.body_bytes += r.body.as_ref().map_or(0, |v| v.0.len());
                        }
                    }
                }
            }
        }
        u
    }
    fn stage_version_install(
        &self,
        cut: &PreparedSet,
        _original: Option<&VersionSetContextId>,
    ) -> Result<(Vec<Option<Alias>>, VersionSetAcquisitionV1), Error> {
        self.validate_version_set_shape(cut)?;
        if cut.invalidated {
            return Err(Error::Faulted);
        }
        let p = self.version_context_pool()?;
        self.retention.validate(&p.reservation).map_err(parent)?;
        let count = cut.members.len();
        let pending = count.checked_add(1).ok_or(Error::Limit)?;
        let reply_bytes = count
            .checked_mul(std::mem::size_of::<VersionSetMemberAcquisitionV1>())
            .and_then(|n| n.checked_add(std::mem::size_of::<VersionSetAcquisitionV1>()))
            .ok_or(Error::Limit)?;
        let rows = cut
            .members
            .iter()
            .try_fold(0usize, |n, m| n.checked_add(m.value.rows.len()))
            .ok_or(Error::Limit)?;
        if count == 0 {
            return Err(Error::Faulted);
        }
        if count > p.limits.max_members
            || rows > p.limits.max_rows
            || cut.capture_bytes()? > p.limits.max_capture_bytes
            || cut.scope_bytes()? > p.limits.max_context_bytes
            || reply_bytes > p.limits.max_reply_bytes
            || std::mem::size_of::<VersionSetContextId>() > p.limits.max_selector_bytes
            || std::mem::size_of::<VersionSetMemberId>() > p.limits.max_selector_bytes
            || p.rows.iter().flatten().count() >= p.limits.max_contexts
            || p.live_selectors()
                .checked_add(pending)
                .is_none_or(|n| n > p.limits.max_selectors)
            || p.issued()?
                .checked_add(pending)
                .is_none_or(|n| n > p.limits.max_issued)
            || cut.members.iter().any(|m| {
                settings::basis::labels(&m.scope.context)
                    .into_iter()
                    .chain([m.scope.policy.as_str()])
                    .any(|s| s.len() > p.limits.max_label_bytes)
            })
        {
            return Err(Error::Limit);
        }
        if p.rows.capacity() < p.limits.max_issued || p.aliases.capacity() < p.limits.max_issued {
            return Err(Error::Faulted);
        }
        let parent = p.rows.len();
        let first = p.aliases.len();
        let mut aliases = Vec::new();
        aliases
            .try_reserve_exact(count)
            .map_err(|_| Error::Capacity)?;
        let mut participants = Vec::new();
        participants
            .try_reserve_exact(count)
            .map_err(|_| Error::Capacity)?;
        for (index, m) in cut.members.iter().enumerate() {
            let slot = first.checked_add(index).ok_or(Error::Faulted)?;
            aliases.push(Some(Alias { parent, index }));
            participants.push(VersionSetMemberAcquisitionV1 {
                document: m.scope.document.clone(),
                snapshot: VersionSetMemberId {
                    owner: self.namespace.clone(),
                    slot,
                },
                data: m.value.classes,
            });
        }
        #[allow(unused_mut)]
        let mut reply = VersionSetAcquisitionV1 {
            context: VersionSetContextId {
                owner: self.namespace.clone(),
                slot: parent,
            },
            participants,
        };
        #[cfg(test)]
        {
            use crate::coordinator::source_query::result_contract::AcquisitionResultFault as F;
            match self.acquisition_result_fault {
                Some(F::Context) => reply.context.slot = usize::MAX,
                Some(F::Classes) => {
                    reply.participants[0].data =
                        if reply.participants[0].data == AnnotationProjectionClassesV1::Source {
                            AnnotationProjectionClassesV1::Annotations
                        } else {
                            AnnotationProjectionClassesV1::Source
                        }
                }
                Some(F::Roster) => {
                    reply.participants.pop();
                }
                Some(F::MemberAlias) => reply.participants[0].snapshot.slot = usize::MAX,
                Some(F::Duplicate) if reply.participants.len() > 1 => {
                    reply.participants[1].snapshot = reply.participants[0].snapshot.clone();
                }
                Some(F::Document) => reply.participants[0].document.slot = usize::MAX,
                Some(F::StagedBinding) => aliases[0].as_mut().unwrap().parent = usize::MAX,
                Some(F::RetainedParent) => {
                    if let Some(original) = _original {
                        reply.context = original.clone();
                    }
                }
                _ => (),
            }
        }
        Ok((aliases, reply))
    }
    pub fn install_version_set(
        &mut self,
        id: &PreparedVersionSetId,
        f: &TrustedPreparedVersionSetFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<VersionSetAcquisitionV1> {
        self.install_version_set_result(id, f, r).ok()
    }
    #[allow(clippy::too_many_arguments)]
    pub(in crate::coordinator) fn install_version_set_result(
        &mut self,
        id: &PreparedVersionSetId,
        f: &TrustedPreparedVersionSetFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<VersionSetAcquisitionV1, Error> {
        let result = (|| {
            if f.projection != id {
                return Err(Error::Unavailable);
            }
            let cut = self.prepared_set(id)?;
            let p = self.version_context_pool()?;
            if f.members.len() != cut.members.len()
                || f.members
                    .iter()
                    .zip(&cut.members)
                    .any(|(f, m)| f.scope != &m.scope)
                || cut.members.iter().any(|m| {
                    m.scope.context.authority != p.authority || m.scope.context.engine != p.engine
                })
            {
                return Err(Error::Unavailable);
            }
            if cut.facts_request_bytes()? > p.limits.max_request_bytes {
                return Err(Error::Limit);
            }
            if !self.validate_prepared_version_set(id, f, r) {
                return Err(if self.faulted {
                    Error::Faulted
                } else {
                    Error::Unavailable
                });
            }
            self.ensure_admission_unfenced()
                .map_err(source_context::admission_error)?;
            let cut = self.prepared_set(id)?;
            let p = self.version_context_pool()?;
            self.retention.validate(&p.reservation).map_err(parent)?;
            let (aliases, reply) = self.stage_version_install(cut, None)?;
            let accepted = self.valid_version_acquisition(cut, &aliases, &reply, None)?;
            if !self.finish_read_result(
                accepted,
                ReadResultInvariantV1::NativeReadAcquisitionResultInvariant,
                Boundary::ReadAcquisitionResultValidation,
                r,
            ) {
                return Err(Error::Faulted);
            }
            self.faulted = true;
            #[cfg(test)]
            assert!(
                !self.version_set_install_fault,
                "injected coherent installation staging fault"
            );
            let (
                VersionSetPreparationState::Active(prep),
                VersionSetContextState::Active(installed),
            ) = (
                &mut self.version_set_preparations,
                &mut self.version_set_contexts,
            )
            else {
                return Err(Error::Faulted);
            };
            let value = prep.cuts[id.slot].take().ok_or(Error::Faulted)?;
            installed.rows.push(Some(Context {
                value,
                stopped: false,
                holds: 0,
            }));
            installed.aliases.extend(aliases);
            self.faulted = false;
            Ok(reply)
        })();
        self.finish_annotation_preparation(Boundary::VersionSetInstallation, &result, r);
        result
    }
    fn check_version_context(
        &mut self,
        target: VersionSetControlTargetV1<'_>,
        f: &TrustedVersionSetContextFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let slot = self.version_parent_slot(target)?;
        if !Arc::ptr_eq(&f.context.owner, &self.namespace) || f.context.slot != slot {
            return Err(Error::Unavailable);
        }
        let p = self.version_context_pool()?;
        let row = p.rows[slot].as_ref().ok_or(Error::Faulted)?;
        if f.members.len() != row.value.members.len()
            || f.members
                .iter()
                .zip(&row.value.members)
                .any(|(f, m)| f.scope != &m.scope)
        {
            return Err(Error::Unavailable);
        }
        if row.value.context_facts_request_bytes()? > p.limits.max_request_bytes {
            return Err(Error::Limit);
        }
        if row.stopped {
            return Err(Error::Unavailable);
        }
        self.validate_version_aliases(slot)?;
        let (lost, ready) = self.version_set_gate(&row.value, f.members)?;
        if lost {
            let VersionSetContextState::Active(p) = &mut self.version_set_contexts else {
                return Err(Error::Faulted);
            };
            p.rows[slot].as_mut().ok_or(Error::Faulted)?.stopped = true;
            self.finish_annotation_preparation(
                Boundary::VersionSetContextInvalidation,
                &Ok::<_, Error>(()),
                r,
            );
            return Err(Error::Unavailable);
        }
        self.retention.validate(&p.reservation).map_err(parent)?;
        if !ready {
            return Err(Error::Unavailable);
        }
        Ok(())
    }
    pub fn validate_version_set_context(
        &mut self,
        target: VersionSetControlTargetV1<'_>,
        f: &TrustedVersionSetContextFactsV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let result = self.check_version_context(target, f, r);
        self.finish_annotation_preparation(Boundary::VersionSetContextValidation, &result, r);
        result.is_ok()
    }
    pub fn stop_version_set_context(
        &mut self,
        target: VersionSetControlTargetV1<'_>,
        controls: &[SourceReadContextControlV1<'_>],
        r: &mut Recorder,
    ) -> bool {
        self.stop_version_set_context_result(target, controls, r)
            .is_ok()
    }
    pub(in crate::coordinator) fn stop_version_set_context_result(
        &mut self,
        target: VersionSetControlTargetV1<'_>,
        controls: &[SourceReadContextControlV1<'_>],
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = (|| {
            let slot = self.version_parent_slot(target)?;
            let p = self.version_context_pool()?;
            if !p.rows[slot]
                .as_ref()
                .ok_or(Error::Faulted)?
                .value
                .stop_controls_match(controls)
            {
                return Err(Error::Unavailable);
            }
            self.validate_version_aliases(slot)?;
            let VersionSetContextState::Active(p) = &mut self.version_set_contexts else {
                return Err(Error::Faulted);
            };
            p.rows[slot].as_mut().ok_or(Error::Faulted)?.stopped = true;
            Ok(())
        })();
        self.finish_annotation_preparation(Boundary::VersionSetContextStop, &result, r);
        result
    }
    pub fn retire_version_set_context(
        &mut self,
        target: VersionSetControlTargetV1<'_>,
        controls: &[SourceReadContextControlV1<'_>],
        r: &mut Recorder,
    ) -> bool {
        let result = (|| {
            let slot = self.version_parent_slot(target)?;
            let p = self.version_context_pool()?;
            let row = p.rows[slot].as_ref().ok_or(Error::Faulted)?;
            if !row.stopped || !row.value.controls_match(controls) {
                return Err(Error::Unavailable);
            }
            if p.actual_holds(slot)? != row.holds {
                return Err(Error::Faulted);
            }
            if row.holds != 0 {
                return Err(Error::Unavailable);
            }
            self.validate_version_aliases(slot)?;
            self.retention.validate(&p.reservation).map_err(parent)?;
            let VersionSetContextState::Active(p) = &mut self.version_set_contexts else {
                return Err(Error::Faulted);
            };
            for s in &mut p.selectors {
                if s.as_ref().is_some_and(|s| {
                    p.aliases
                        .get(s.context())
                        .and_then(Option::as_ref)
                        .is_some_and(|a| a.parent == slot)
                }) {
                    *s = None;
                }
            }
            for a in &mut p.aliases {
                if a.as_ref().is_some_and(|a| a.parent == slot) {
                    *a = None;
                }
            }
            p.rows[slot] = None;
            Ok(())
        })();
        self.finish_annotation_preparation(Boundary::VersionSetContextRetirement, &result, r);
        result.is_ok()
    }
    pub fn retire_version_set_contexts(
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
            let VersionSetContextState::Active(p) = &mut self.version_set_contexts else {
                return Ok(false);
            };
            if p.rows.iter().any(Option::is_some) {
                return Err(Error::Unavailable);
            }
            if p.aliases.iter().any(Option::is_some) {
                return Err(Error::Faulted);
            }
            if p.selectors.iter().any(Option::is_some) {
                return Err(Error::Faulted);
            }
            if p.holds.iter().any(Option::is_some) {
                return Err(Error::Faulted);
            }
            let issued = p.issued()?;
            self.retention.release(&mut p.reservation).map_err(parent)?;
            self.version_set_contexts = VersionSetContextState::Closed { issued };
            Ok(true)
        })();
        self.finish_annotation_preparation(Boundary::VersionSetContextPoolRetirement, &result, r);
        result
    }
}
