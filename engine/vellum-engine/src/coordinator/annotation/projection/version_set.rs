//! One-cut private coherent preparation; no independently installed child contexts.
use super::*;
mod installed;
mod shutdown_inventory;
pub(in crate::coordinator) use installed::VersionSetContextState;
pub(in crate::coordinator) use installed::VersionSetSourceHoldId;
#[cfg(test)]
pub(in crate::coordinator) use installed::{
    AnnotationFacadeFixture, native_facade_version_fixture,
};
pub use installed::{
    TrustedVersionSetContextFactsV1, VersionSetAcquisitionV1, VersionSetAnnotationContinuationId,
    VersionSetAnnotationQueryV1, VersionSetAnnotationRecordId, VersionSetAnnotationRecordV1,
    VersionSetAnnotationReplyV1, VersionSetContextId, VersionSetContextLimitsV1,
    VersionSetContextUsageV1, VersionSetControlTargetV1, VersionSetMemberAcquisitionV1,
    VersionSetMemberId, VersionSetSourceReplyV1,
};
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VersionSetPreparationLimitsV1 {
    pub max_prepared: usize,
    pub max_issued: usize,
    pub max_members: usize,
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
impl VersionSetPreparationLimitsV1 {
    /// Parent measurement workspace uses the actual immutable parent ceiling,
    /// independently of this profile's smaller root-refusal threshold.
    pub fn required_pool_bytes(self, parent_root_ceiling: usize) -> Result<usize, Error> {
        let mut n = std::mem::size_of::<SetPool>();
        for (count, size) in [
            (self.max_issued, std::mem::size_of::<Option<PreparedSet>>()),
            (2, parent_root_ceiling),
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
    fn projection(self) -> AnnotationPreparationLimitsV1 {
        AnnotationPreparationLimitsV1 {
            max_prepared: 0,
            max_issued: 0,
            max_rows: self.max_rows,
            max_root_bytes: self.max_root_bytes,
            max_capture_bytes: self.max_capture_bytes,
            max_context_bytes: self.max_context_bytes,
            max_request_bytes: self.max_request_bytes,
            max_selector_bytes: self.max_selector_bytes,
            max_label_bytes: self.max_label_bytes,
            max_pool_bytes: 0,
            source: self.source,
        }
    }
}
/// Native trusted-owner input, not the public wire envelope or a new execution verb.
pub struct VersionSetParticipantV1<'a> {
    pub scope: &'a SourceReadScopeV1,
    pub data: AnnotationProjectionClassesV1,
}
#[derive(Clone)]
pub struct PreparedVersionSetId {
    owner: Arc<()>,
    slot: usize,
}
impl PartialEq for PreparedVersionSetId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for PreparedVersionSetId {}
pub struct TrustedPreparedVersionSetFactsV1<'a> {
    pub projection: &'a PreparedVersionSetId,
    pub members: &'a [TrustedVersionSetMemberFactsV1<'a>],
}
impl std::fmt::Debug for PreparedVersionSetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreparedVersionSetId(<opaque>)")
    }
}
/// Fresh facts for one required member of the exact original prepared cut.
pub struct TrustedVersionSetMemberFactsV1<'a> {
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
struct Member {
    scope: SourceReadScopeV1,
    value: Projection,
}
struct PreparedSet {
    cut: u64,
    members: Vec<Member>,
    invalidated: bool,
}
pub(in crate::coordinator) struct SetPool {
    authority: String,
    engine: String,
    limits: VersionSetPreparationLimitsV1,
    parent_root_ceiling: usize,
    reservation: retention::Reservation,
    charge: usize,
    cuts: Vec<Option<PreparedSet>>,
}
#[derive(Default)]
pub(in crate::coordinator) enum VersionSetPreparationState {
    #[default]
    Unconfigured,
    Active(Box<SetPool>),
    Closed {
        issued: usize,
    },
}
impl std::fmt::Debug for VersionSetPreparationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("VersionSetPreparationState(<redacted>)")
    }
}
impl VersionSetPreparationState {
    pub(in crate::coordinator) fn document_pins(&self, document: &DocumentId) -> usize {
        match self {
            Self::Unconfigured | Self::Closed { .. } => 0,
            Self::Active(p) => p
                .cuts
                .iter()
                .flatten()
                .filter(|c| c.members.iter().any(|m| m.scope.document == *document))
                .count(),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VersionSetPreparationUsageV1 {
    pub prepared: usize,
    pub invalidated: usize,
    pub issued_lifetime: usize,
    pub reserved_pool_bytes: usize,
    pub members: usize,
    pub source_bytes: usize,
    pub body_bytes: usize,
    pub projected_rows: usize,
    pub closed: bool,
}
fn valid_scope(s: &SourceReadScopeV1, p: &SetPool) -> bool {
    s.context.authority == p.authority
        && s.context.engine == p.engine
        && !s.policy.is_empty()
        && s.policy.len() <= p.limits.max_label_bytes
        && settings::basis::labels(&s.context)
            .into_iter()
            .all(|v| !v.is_empty() && v.len() <= p.limits.max_label_bytes)
}
fn shared(a: &SourceReadScopeV1, b: &SourceReadScopeV1) -> bool {
    a.context.authority == b.context.authority
        && a.context.engine == b.context.engine
        && a.context.recipient == b.context.recipient
        && a.context.destination == b.context.destination
        && a.context.purpose == b.context.purpose
        && a.context.profile == b.context.profile
}
impl SourceCoordinator {
    /// Exact ordered captured meaning only; never returns a projection or grant.
    pub(in crate::coordinator) fn prepared_version_set_matches(
        &self,
        id: &PreparedVersionSetId,
        participants: &[VersionSetParticipantV1<'_>],
    ) -> bool {
        self.prepared_set(id).is_ok_and(|cut| {
            !participants.is_empty()
                && participants.len() == cut.members.len()
                && participants
                    .iter()
                    .zip(&cut.members)
                    .all(|(request, member)| {
                        request.scope == &member.scope && request.data == member.value.classes
                    })
        })
    }
    fn version_preparation_pool(&self) -> Result<&SetPool, Error> {
        match &self.version_set_preparations {
            VersionSetPreparationState::Active(p) => Ok(p),
            _ => Err(Error::Unavailable),
        }
    }
    fn prepared_set(&self, id: &PreparedVersionSetId) -> Result<&PreparedSet, Error> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return Err(Error::Unavailable);
        }
        self.version_preparation_pool()?
            .cuts
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)
    }
    pub fn configure_version_set_preparation(
        &mut self,
        authority: &str,
        engine: &str,
        limits: VersionSetPreparationLimitsV1,
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
            match &self.version_set_preparations {
                VersionSetPreparationState::Closed { .. } => return Err(Error::Unavailable),
                VersionSetPreparationState::Active(p) => {
                    self.retention.validate(&p.reservation).map_err(parent)?;
                    return if p.authority == authority && p.engine == engine && p.limits == limits {
                        Ok(false)
                    } else {
                        Err(Error::Unavailable)
                    };
                }
                VersionSetPreparationState::Unconfigured => {}
            }
            self.ensure_admission_unfenced()
                .map_err(source_context::admission_error)?;
            let parent_root_ceiling = self.root_byte_ceiling.ok_or(Error::Unavailable)?;
            let labels = authority
                .len()
                .checked_add(engine.len())
                .ok_or(Error::Limit)?;
            if std::mem::size_of::<VersionSetPreparationLimitsV1>()
                .checked_add(labels)
                .is_none_or(|n| n > limits.max_request_bytes)
            {
                return Err(Error::Limit);
            }
            let charge = limits
                .required_pool_bytes(parent_root_ceiling)?
                .checked_add(labels)
                .filter(|n| *n <= limits.max_pool_bytes)
                .ok_or(Error::Limit)?;
            self.retention.check(charge).map_err(parent)?;
            let authority = owned(authority)?;
            let engine = owned(engine)?;
            let mut cuts = Vec::new();
            cuts.try_reserve_exact(limits.max_issued)
                .map_err(|_| Error::Capacity)?;
            let reservation = self.retention.reserve(charge).map_err(parent)?;
            self.version_set_preparations = VersionSetPreparationState::Active(Box::new(SetPool {
                authority,
                engine,
                limits,
                parent_root_ceiling,
                reservation,
                charge,
                cuts,
            }));
            Ok(true)
        })();
        self.finish_annotation_preparation(
            Boundary::VersionSetPreparationConfiguration,
            &result,
            r,
        );
        result
    }
    pub fn version_set_preparation_usage(&self) -> VersionSetPreparationUsageV1 {
        let mut u = VersionSetPreparationUsageV1::default();
        match &self.version_set_preparations {
            VersionSetPreparationState::Unconfigured => {}
            VersionSetPreparationState::Closed { issued } => {
                u.closed = true;
                u.issued_lifetime = *issued;
            }
            VersionSetPreparationState::Active(p) => {
                u.reserved_pool_bytes = p.charge;
                u.issued_lifetime = p.cuts.len();
                for cut in p.cuts.iter().flatten() {
                    u.prepared += 1;
                    u.invalidated += usize::from(cut.invalidated);
                    for member in &cut.members {
                        u.members += 1;
                        u.projected_rows += member.value.rows.len();
                        if let Some((s, _, _)) = &member.value.source {
                            u.source_bytes += s.text().len();
                        }
                        for row in &member.value.rows {
                            u.body_bytes += row.body.as_ref().map_or(0, |v| v.0.len());
                        }
                    }
                }
            }
        }
        u
    }
    pub fn prepare_version_set(
        &mut self,
        participants: &[VersionSetParticipantV1<'_>],
        facts: &[TrustedAnnotationCaptureFactsV1<'_>],
        r: &mut Recorder,
    ) -> Option<PreparedVersionSetId> {
        let result = (|| {
            if self.faulted {
                return Err(Error::Faulted);
            }
            self.ensure_admission_unfenced()
                .map_err(source_context::admission_error)?;
            let p = self.version_preparation_pool()?;
            let limits = p.limits;
            if participants.is_empty()
                || participants.len() > limits.max_members
                || facts.len() != participants.len()
            {
                return Err(Error::InvalidRequest);
            }
            let mut request = std::mem::size_of::<PreparedVersionSetId>();
            let mut scopes = 0usize;
            for (i, (m, f)) in participants.iter().zip(facts).enumerate() {
                if !valid_scope(m.scope, p)
                    || m.scope != f.scope
                    || !shared(participants[0].scope, m.scope)
                    || participants[..i]
                        .iter()
                        .any(|x| x.scope.document == m.scope.document)
                {
                    return Err(Error::InvalidRequest);
                }
                scopes = scopes
                    .checked_add(scope_bytes(m.scope)?)
                    .ok_or(Error::Limit)?;
                request = request
                    .checked_add(scope_bytes(m.scope)?)
                    .and_then(|n| {
                        n.checked_add(std::mem::size_of::<AnnotationProjectionClassesV1>())
                    })
                    .and_then(|n| {
                        n.checked_add(
                            f.grants
                                .len()
                                .checked_mul(std::mem::size_of::<AnnotationFacetGrantV1>())?,
                        )
                    })
                    .ok_or(Error::Limit)?;
            }
            if request > limits.max_request_bytes || scopes > limits.max_context_bytes {
                return Err(Error::Limit);
            }
            self.retention.validate(&p.reservation).map_err(parent)?;
            if self.root_byte_ceiling != Some(p.parent_root_ceiling)
                || p.cuts.capacity() < limits.max_issued
            {
                return Err(Error::Faulted);
            }
            if p.cuts.len() >= limits.max_issued
                || p.cuts.iter().flatten().count() >= limits.max_prepared
            {
                return Err(Error::Limit);
            }
            if std::mem::size_of::<PreparedVersionSetId>() > limits.max_selector_bytes {
                return Err(Error::Limit);
            }
            let cut = self.generation;
            if participants
                .len()
                .checked_mul(std::mem::size_of::<Member>())
                .and_then(|n| n.checked_add(std::mem::size_of::<PreparedSet>()))
                .is_none_or(|n| n > limits.max_capture_bytes)
            {
                return Err(Error::Limit);
            }
            let mut members = Vec::new();
            members
                .try_reserve_exact(participants.len())
                .map_err(|_| Error::Capacity)?;
            let mut capture = std::mem::size_of::<PreparedSet>();
            let mut projected_rows = 0usize;
            for (m, f) in participants.iter().zip(facts) {
                let value = self.stage_current_projection(
                    m.data,
                    m.scope,
                    f,
                    limits.projection(),
                    None,
                    r,
                )?;
                projected_rows = projected_rows
                    .checked_add(value.rows.len())
                    .ok_or(Error::Limit)?;
                if projected_rows > limits.max_rows {
                    return Err(Error::Limit);
                }
                capture = capture
                    .checked_add(std::mem::size_of::<Member>())
                    .and_then(|n| n.checked_add(value.bytes().ok()?))
                    .ok_or(Error::Limit)?;
                if capture > limits.max_capture_bytes {
                    return Err(Error::Limit);
                }
                let scope = SourceReadScopeV1 {
                    document: m.scope.document.clone(),
                    context: settings::basis::copy_context(&m.scope.context)
                        .map_err(|_| Error::Capacity)?,
                    policy: owned(&m.scope.policy)?,
                };
                members.push(Member { scope, value });
            }
            if self.generation != cut {
                return Err(Error::Faulted);
            }
            #[cfg(test)]
            if self.version_set_preparation_fault {
                self.faulted = true;
                panic!("injected coherent preparation staging fault");
            }
            let VersionSetPreparationState::Active(p) = &mut self.version_set_preparations else {
                return Err(Error::Faulted);
            };
            let id = PreparedVersionSetId {
                owner: self.namespace.clone(),
                slot: p.cuts.len(),
            };
            p.cuts.push(Some(PreparedSet {
                cut,
                members,
                invalidated: false,
            }));
            Ok(id)
        })();
        self.finish_annotation_preparation(Boundary::VersionSetPreparation, &result, r);
        result.ok()
    }
    fn validate_version_set_shape(&self, cut: &PreparedSet) -> Result<(), Error> {
        if cut.members.is_empty()
            || cut.members.iter().enumerate().any(|(i, m)| {
                !shared(&cut.members[0].scope, &m.scope)
                    || cut.members[..i]
                        .iter()
                        .any(|p| p.scope.document == m.scope.document)
            })
            || cut.cut > self.generation
            || cut
                .members
                .iter()
                .any(|m| !m.value.valid(&m.scope, &self.namespace))
        {
            return Err(Error::Faulted);
        }
        Ok(())
    }
    fn version_set_gate(
        &self,
        cut: &PreparedSet,
        facts: &[TrustedVersionSetMemberFactsV1<'_>],
    ) -> Result<(bool, bool), Error> {
        if facts.len() != cut.members.len()
            || facts
                .iter()
                .zip(&cut.members)
                .any(|(f, m)| f.scope != &m.scope)
        {
            return Err(Error::Unavailable);
        }
        self.validate_version_set_shape(cut)?;
        let mut lost = self.faulted;
        let mut allowed = true;
        for (m, f) in cut.members.iter().zip(facts) {
            let gates = [
                f.authorized,
                f.scope_allowed,
                f.continuous,
                f.live,
                f.not_stopped,
                f.projection_current,
                f.retained,
            ];
            lost |= self.document(&m.scope.document).is_err()
                || gates.contains(&Some(false))
                || m.value.classes.includes_source() && f.source_readable == Some(false)
                || m.value.classes.includes_annotations() && f.annotation_readable == Some(false);
            allowed &= gates
                .into_iter()
                .chain([f.resources_ready, f.reply_ready, f.transfer_ready])
                .all(|v| v == Some(true))
                && (!m.value.classes.includes_source() || f.source_readable == Some(true))
                && (!m.value.classes.includes_annotations() || f.annotation_readable == Some(true));
        }
        Ok((lost, allowed))
    }
    pub fn validate_prepared_version_set(
        &mut self,
        id: &PreparedVersionSetId,
        f: &TrustedPreparedVersionSetFactsV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let result = (|| {
            if f.projection != id {
                return Err(Error::Unavailable);
            }
            let facts = f.members;
            let cut = self.prepared_set(id)?;
            let p = self.version_preparation_pool()?;
            if facts.len() != cut.members.len()
                || facts
                    .iter()
                    .zip(&cut.members)
                    .any(|(f, m)| f.scope != &m.scope)
            {
                return Err(Error::Unavailable);
            }
            let request = cut
                .members
                .iter()
                .try_fold(std::mem::size_of::<PreparedVersionSetId>(), |n, m| {
                    n.checked_add(scope_bytes(&m.scope).ok()?).and_then(|n| {
                        n.checked_add(std::mem::size_of::<TrustedVersionSetMemberFactsV1>())
                    })
                })
                .ok_or(Error::Limit)?;
            if request > p.limits.max_request_bytes {
                return Err(Error::Limit);
            }
            if cut.invalidated {
                return Err(Error::Unavailable);
            }
            let (lost, allowed) = self.version_set_gate(cut, facts)?;
            if lost {
                let VersionSetPreparationState::Active(p) = &mut self.version_set_preparations
                else {
                    return Err(Error::Faulted);
                };
                p.cuts[id.slot].as_mut().ok_or(Error::Faulted)?.invalidated = true;
                self.finish_annotation_preparation(
                    Boundary::VersionSetPreparationInvalidation,
                    &Ok::<_, Error>(()),
                    r,
                );
                return Err(Error::Unavailable);
            }
            self.retention.validate(&p.reservation).map_err(parent)?;
            if !allowed {
                return Err(Error::Unavailable);
            }
            Ok(())
        })();
        self.finish_annotation_preparation(Boundary::VersionSetPreparationValidation, &result, r);
        result.is_ok()
    }
    pub fn discard_prepared_version_set(
        &mut self,
        id: &PreparedVersionSetId,
        controls: &[SourceReadContextControlV1<'_>],
        r: &mut Recorder,
    ) -> bool {
        let result = (|| {
            let cut = self.prepared_set(id)?;
            if controls.len() != cut.members.len()
                || controls
                    .iter()
                    .zip(&cut.members)
                    .any(|(f, m)| f.scope != &m.scope || !discharge(f))
            {
                return Err(Error::Unavailable);
            }
            self.retention
                .validate(&self.version_preparation_pool()?.reservation)
                .map_err(parent)?;
            let VersionSetPreparationState::Active(p) = &mut self.version_set_preparations else {
                return Err(Error::Faulted);
            };
            p.cuts[id.slot] = None;
            Ok(())
        })();
        self.finish_annotation_preparation(Boundary::VersionSetPreparationDiscard, &result, r);
        result.is_ok()
    }
    pub fn retire_version_set_preparation(
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
            let VersionSetPreparationState::Active(p) = &mut self.version_set_preparations else {
                return Ok(false);
            };
            if p.cuts.iter().any(Option::is_some) {
                return Err(Error::Unavailable);
            }
            let issued = p.cuts.len();
            self.retention.release(&mut p.reservation).map_err(parent)?;
            self.version_set_preparations = VersionSetPreparationState::Closed { issued };
            Ok(true)
        })();
        self.finish_annotation_preparation(
            Boundary::VersionSetPreparationPoolRetirement,
            &result,
            r,
        );
        result
    }
}
