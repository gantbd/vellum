//! Owned acknowledged Save As activation. No write dispatch or UI.
use super::super::settings::basis::{copy_context, labels};
use super::*;
use crate::resource::{ResourceError, SaveId};
use std::mem::size_of;

pub struct SourceSaveAsRequestV1 {
    pub probe: crate::resource::SourceSaveAsProbeV1,
    pub pair: Option<crate::resource::OrdinaryPairEvidence>,
    pub max_binding_history: usize,
}
impl SourceSaveAsRequestV1 {
    /// Native SDK roles only, not a wire grammar or execution grant.
    pub const fn argument_declaration_v1() -> NativeArgumentDeclarationV1 {
        NativeArgumentDeclarationV1 {
            contract: "engine-source-saveas-declaration/1",
            variant: NativeHandlerVariantDeclarationV1 {
                operation: "activate",
                fields: &[
                    ("probe", "OriginalSourceSaveAsProbeV1"),
                    ("pair", "OptionalOriginalOrdinaryPairEvidence"),
                    ("max_binding_history", "ExplicitHistoryRowBound"),
                ],
            },
        }
    }
    fn retained_bytes(&self) -> Result<usize, InvocationError> {
        let mut n = size_of::<Self>()
            .checked_add(
                self.probe
                    .identity
                    .key
                    .0
                    .len()
                    .checked_mul(2)
                    .ok_or(InvocationError::Limit)?,
            )
            .ok_or(InvocationError::Limit)?;
        for value in [&self.probe.epoch, &self.probe.version]
            .into_iter()
            .flatten()
        {
            n = n
                .checked_add(value.0.len().checked_mul(2).ok_or(InvocationError::Limit)?)
                .ok_or(InvocationError::Limit)?;
        }
        if let Some(p) = &self.pair {
            for k in [&p.check.source, &p.check.annotations, &p.check.proposed]
                .into_iter()
                .flatten()
            {
                n = n
                    .checked_add(
                        k.identity
                            .key
                            .0
                            .len()
                            .checked_mul(2)
                            .ok_or(InvocationError::Limit)?,
                    )
                    .ok_or(InvocationError::Limit)?;
            }
        }
        Ok(n)
    }
}
fn copy_probe(p: &crate::resource::SourceSaveAsProbeV1) -> crate::resource::SourceSaveAsProbeV1 {
    crate::resource::SourceSaveAsProbeV1 {
        registration: p.registration.clone(),
        identity: p.identity.clone(),
        epoch: p.epoch.clone(),
        version: p.version.clone(),
    }
}
fn same_probe(
    a: &crate::resource::SourceSaveAsProbeV1,
    b: &crate::resource::SourceSaveAsProbeV1,
) -> bool {
    Arc::ptr_eq(&a.registration, &b.registration)
        && a.identity == b.identity
        && a.epoch == b.epoch
        && a.version == b.version
}
fn copy_pair(
    p: &Option<crate::resource::OrdinaryPairEvidence>,
) -> Option<crate::resource::OrdinaryPairEvidence> {
    p.as_ref().map(|p| crate::resource::OrdinaryPairEvidence {
        relation: p.relation,
        check: crate::resource::OrdinaryPairCheck {
            owner: p.check.owner.clone(),
            marker: p.check.marker.clone(),
            document: p.check.document.clone(),
            source: p.check.source.clone(),
            annotations: p.check.annotations.clone(),
            proposed: p.check.proposed.clone(),
        },
    })
}
fn same_pair(
    a: &Option<crate::resource::OrdinaryPairEvidence>,
    b: &Option<crate::resource::OrdinaryPairEvidence>,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.relation == b.relation
                && Arc::ptr_eq(&a.check.owner, &b.check.owner)
                && Arc::ptr_eq(&a.check.marker, &b.check.marker)
                && a.check.document == b.check.document
                && a.check.source == b.check.source
                && a.check.annotations == b.check.annotations
                && a.check.proposed == b.check.proposed
        }
        _ => false,
    }
}
fn copy_request(q: &SourceSaveAsRequestV1) -> SourceSaveAsRequestV1 {
    SourceSaveAsRequestV1 {
        probe: copy_probe(&q.probe),
        pair: copy_pair(&q.pair),
        max_binding_history: q.max_binding_history,
    }
}
fn same_request(a: &SourceSaveAsRequestV1, b: &SourceSaveAsRequestV1) -> bool {
    same_probe(&a.probe, &b.probe)
        && same_pair(&a.pair, &b.pair)
        && a.max_binding_history == b.max_binding_history
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceSaveAsLimitsV1 {
    pub max_capture_bytes: usize,
    pub max_result_bytes: usize,
}
impl SourceSaveAsLimitsV1 {
    /// Fixed retained native capsule/receipt plus copied context labels. No
    /// retained second source buffer; native preparation uses existing limits.
    pub fn required_capture_bytes(
        c: &SettingsExecutionContextV1,
        request: &SourceSaveAsRequestV1,
    ) -> Result<usize, InvocationError> {
        labels(c).into_iter().try_fold(
            (size_of::<Capture>() + size_of::<Hold>())
                .checked_add(request.retained_bytes()?)
                .ok_or(InvocationError::Limit)?,
            |n, s| n.checked_add(s.len()).ok_or(InvocationError::Limit),
        )
    }
}
pub struct TrustedSourceSaveAsInvocationFactsV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub native: &'a TrustedSourceSaveAsFactsV1,
    pub authority: &'a SourceSaveAsAuthorityFactsV1,
    pub interpretation_current: Option<bool>,
}
impl TrustedSourceSaveAsInvocationFactsV1<'_> {
    fn allowed(&self) -> bool {
        self.interpretation_current == Some(true)
            && resource::source_save_as_authorized(self.native)
            && [
                self.authority.owner_control_allowed,
                self.authority.preparation_reserved,
                self.authority.resources_ready,
            ]
            .into_iter()
            .all(|v| v == Some(true))
    }
}
pub struct SourceSaveAsAdmissionV1<'a> {
    pub facts: &'a TrustedSourceSaveAsInvocationFactsV1<'a>,
    pub limits: SourceSaveAsLimitsV1,
    pub publication_epoch: u64,
    pub publication_allowed: Option<bool>,
    pub result_admission_allowed: Option<bool>,
    pub result_allowed: Option<bool>,
}
pub struct TrustedSourceSaveAsResultV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub publication_epoch: u64,
    pub publication_allowed: Option<bool>,
    pub outcome_readable: Option<bool>,
    pub result_allowed: Option<bool>,
    pub result_context_current: Option<bool>,
    pub historical_document_readable: Option<bool>,
    pub reply_ready: Option<bool>,
    pub resources_ready: Option<bool>,
}
/// Historical identity and exact native disposition, never source/currentness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSaveAsResultV1 {
    pub invocation: InvocationId,
    pub document: DocumentId,
    pub changed: bool,
}
pub(super) struct Capture {
    pub(super) owner: Arc<()>,
    pub(super) context: SettingsExecutionContextV1,
    invocation: InvocationId,
    authority: SourceSaveAsAuthorityV1,
    request: SourceSaveAsRequestV1,
    document: DocumentId,
    profile: CoordinatorLimits,
    limits: SourceSaveAsLimitsV1,
    epoch: u64,
    publication: bool,
    admission: bool,
    result: bool,
    marker: Arc<()>,
}
pub(super) struct Hold {
    reservation: retention::Reservation,
    marker: Arc<()>,
    pub(super) candidate: Option<(DocumentId, bool)>,
}
impl std::fmt::Debug for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceSaveAs(<redacted>)")
    }
}
impl std::fmt::Debug for Hold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceSaveAsHold(<owned>)")
    }
}
fn parent(e: RetentionErrorV1) -> InvocationError {
    match e {
        RetentionErrorV1::Faulted => InvocationError::Faulted,
        RetentionErrorV1::Limit => InvocationError::Limit,
        RetentionErrorV1::Unavailable => InvocationError::Unavailable,
    }
}
fn resource(e: ResourceError) -> InvocationError {
    match e {
        ResourceError::Faulted => InvocationError::Faulted,
        ResourceError::Capacity => InvocationError::Capacity,
        ResourceError::Limit => InvocationError::Limit,
        _ => InvocationError::Unavailable,
    }
}
shutdown_capture_reservation!();
impl Capture {
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.document == *document
    }
    pub(super) fn intact(
        &self,
        owner: &Arc<()>,
        id: &InvocationId,
        h: &Option<Hold>,
        ledger: &retention::RetentionState,
    ) -> bool {
        Arc::ptr_eq(&self.owner, owner)
            && self.invocation == *id
            && self.authority.belongs_to(owner)
            && Arc::ptr_eq(&self.document.namespace, owner)
            && SourceSaveAsLimitsV1::required_capture_bytes(&self.context, &self.request)
                .is_ok_and(|n| n <= self.limits.max_capture_bytes)
            && h.as_ref().is_some_and(|h| {
                Arc::ptr_eq(&self.marker, &h.marker)
                    && ledger
                        .validate_exact(&h.reservation, self.limits.max_capture_bytes)
                        .is_ok()
            })
    }
}
impl SourceCoordinator {
    fn prepare_authorized_source_save_as(
        &mut self,
        authority: &SourceSaveAsAuthorityV1,
        request: &SourceSaveAsRequestV1,
        f: &TrustedSourceSaveAsInvocationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<(resource::PreparedSourceSaveAs, DocumentId), ResourceError> {
        if !f.allowed() {
            return Err(ResourceError::Unavailable);
        }
        let (task, document) =
            self.source_save_as_identity_for_invocation(authority, f.authority, r)?;
        let plan = self.prepare_source_save_as_native(
            &task,
            copy_probe(&request.probe),
            copy_pair(&request.pair),
            request.max_binding_history,
            f.native,
            r,
        )?;
        Ok((plan, document))
    }
    pub fn submit_source_save_as_v1(
        &mut self,
        id: &InvocationId,
        authority: &SourceSaveAsAuthorityV1,
        request: &SourceSaveAsRequestV1,
        a: &SourceSaveAsAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        let timing = r.begin_service_timing();
        let mut provisional = None;
        let mut result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let s = self.invocation_slot(id)?;
            let l = a.limits;
            if SourceSaveAsLimitsV1::required_capture_bytes(a.facts.context, request)?
                > l.max_capture_bytes
                || l.max_capture_bytes > self.invocations.limits.max_intent_bytes
                || l.max_result_bytes < size_of::<SourceSaveAsResultV1>()
                || l.max_result_bytes > self.invocations.limits.max_intent_bytes
            {
                return Err(InvocationError::Limit);
            }
            if labels(a.facts.context).iter().any(|s| s.is_empty()) || !a.facts.allowed() {
                return Err(InvocationError::Unavailable);
            }
            let (Some(publication), Some(admission), Some(result)) = (
                a.publication_allowed,
                a.result_admission_allowed,
                a.result_allowed,
            ) else {
                return Err(InvocationError::Unavailable);
            };
            if let Some(original) = &s.intent {
                let BoundIntent::SourceSaveAs(c) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                if !c.intact(&self.namespace, id, &s.saveas_hold, &self.retention) {
                    return Err(InvocationError::Faulted);
                }
                if !same_request(&c.request, request)
                    || c.authority != *authority
                    || c.context != *a.facts.context
                    || c.limits != l
                    || c.epoch != a.publication_epoch
                    || c.publication != publication
                    || c.admission != admission
                    || c.result != result
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return s.phase.state();
            }
            if !matches!(s.phase, Phase::Issued(_)) {
                return Err(InvocationError::Unavailable);
            }
            let (_, document) = self
                .prepare_authorized_source_save_as(authority, request, a.facts, r)
                .map_err(resource)?;
            provisional = Some(
                self.retention
                    .reserve(l.max_capture_bytes)
                    .map_err(parent)?,
            );
            let context = copy_context(a.facts.context).map_err(|e| {
                if e == crate::settings::SettingsError::Capacity {
                    InvocationError::Capacity
                } else {
                    InvocationError::Limit
                }
            })?;
            let marker = Arc::new(());
            let c = Arc::new(Capture {
                owner: self.namespace.clone(),
                context,
                invocation: id.clone(),
                authority: authority.capture_for_invocation(),
                request: copy_request(request),
                document,
                profile: self.limits,
                limits: l,
                epoch: a.publication_epoch,
                publication,
                admission,
                result,
                marker: marker.clone(),
            });
            self.invocations.slots[id.slot]
                .as_mut()
                .expect("issued load")
                .saveas_hold = Some(Hold {
                reservation: provisional.take().expect("funded load"),
                marker,
                candidate: None,
            });
            self.install_invocation_admission(
                id,
                BoundIntent::SourceSaveAs(c.clone()),
                Phase::SourceSaveAsPending(c),
            );
            Ok(InvocationState::Pending)
        })();
        if let Some(mut p) = provisional
            && self.retention.release(&mut p).is_err()
        {
            result = Err(InvocationError::Faulted);
        }
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(r, Boundary::SourceSaveAsAdmission, &result, timing.finish());
        result
    }
    pub fn run_source_save_as_v1(
        &mut self,
        id: &InvocationId,
        f: &TrustedSourceSaveAsInvocationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::SourceSaveAs(c)) = &s.intent else {
                return Err(InvocationError::Unavailable);
            };
            if c.context != *f.context || c.profile != self.limits {
                return Err(InvocationError::Unavailable);
            }
            if !c.intact(&self.namespace, id, &s.saveas_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            match &s.phase {
                Phase::Terminal(_) => return Ok(InvocationState::Finished),
                Phase::Running => return Err(InvocationError::Faulted),
                Phase::SourceSaveAsPending(p) if Arc::ptr_eq(c, p) => {}
                _ => return Err(InvocationError::Faulted),
            }
            let c = c.clone();
            if self.service_requested_stop(id, r)? {
                return Ok(InvocationState::Finished);
            }
            // Host denial is retryable before service; native refusal is terminal.
            if !f.allowed() {
                return Err(InvocationError::Unavailable);
            }
            self.invocations.slots[id.slot]
                .as_mut()
                .expect("pending load")
                .phase = Phase::Running;
            let result = self
                .prepare_authorized_source_save_as(&c.authority, &c.request, f, r)
                .and_then(|(p, document)| {
                    if document != c.document {
                        return Err(ResourceError::Faulted);
                    }
                    self.commit_source_save_as_for_invocation(p, f.native, Some(id), r)
                });
            match result {
                Ok(_) => {} // The native installer settled the terminal under its fence.
                Err(ResourceError::Faulted) => return Err(InvocationError::Faulted),
                Err(e) => {
                    self.install_invocation_terminal(id, InvocationOutcome::SourceSaveAsRejected(e))
                }
            }
            Ok(InvocationState::Finished)
        }));
        let result = match caught {
            Ok(v) => v,
            Err(p) => {
                self.faulted = true;
                capture_timed(
                    r,
                    Boundary::SourceSaveAsService,
                    &Err::<InvocationState, _>(InvocationError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(p)
            }
        };
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(r, Boundary::SourceSaveAsService, &result, timing.finish());
        result
    }
    pub(in crate::coordinator) fn stage_source_save_as_candidate(
        &mut self,
        id: &InvocationId,
        task: &SaveId,
        document: &DocumentId,
        request: (
            &crate::resource::SourceSaveAsProbeV1,
            &Option<crate::resource::OrdinaryPairEvidence>,
            usize,
        ),
        changed: bool,
    ) -> Result<InvocationOutcome, ResourceError> {
        let (probe, pair, maximum) = request;
        let slot = self
            .invocation_slot(id)
            .map_err(|_| ResourceError::Faulted)?;
        let Some(BoundIntent::SourceSaveAs(c)) = &slot.intent else {
            return Err(ResourceError::Faulted);
        };
        if !matches!(slot.phase, Phase::Running)
            || !c.authority.names_task(task)
            || c.document != *document
            || !same_probe(&c.request.probe, probe)
            || !same_pair(&c.request.pair, pair)
            || c.request.max_binding_history != maximum
            || !c.intact(&self.namespace, id, &slot.saveas_hold, &self.retention)
            || slot
                .saveas_hold
                .as_ref()
                .is_none_or(|h| h.candidate.is_some())
        {
            return Err(ResourceError::Faulted);
        }
        let candidate = (document.clone(), changed);
        let terminal = InvocationOutcome::SourceSaveAsCommitted(document.clone(), changed);
        self.invocations.slots[id.slot]
            .as_mut()
            .expect("running Save As")
            .saveas_hold
            .as_mut()
            .expect("funded Save As")
            .candidate = Some(candidate);
        Ok(terminal)
    }
    pub fn source_save_as_result_v1(
        &self,
        id: &InvocationId,
        f: &TrustedSourceSaveAsResultV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourceSaveAsResultV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::SourceSaveAs(c)) = &s.intent else {
                return Err(InvocationError::OutcomeUnavailable);
            };
            if c.context != *f.context
                || c.epoch != f.publication_epoch
                || !c.publication
                || !c.admission
                || !c.result
                || [
                    f.publication_allowed,
                    f.outcome_readable,
                    f.result_allowed,
                    f.result_context_current,
                    f.historical_document_readable,
                    f.reply_ready,
                    f.resources_ready,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(InvocationError::Unavailable);
            }
            if !c.intact(&self.namespace, id, &s.saveas_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            match &s.phase {
                Phase::Terminal(Some(InvocationOutcome::SourceSaveAsCommitted(
                    document,
                    changed,
                ))) => {
                    if s.saveas_hold.as_ref().and_then(|h| h.candidate.as_ref())
                        != Some(&(document.clone(), *changed))
                    {
                        return Err(InvocationError::Faulted);
                    }
                    Ok(SourceSaveAsResultV1 {
                        invocation: id.clone(),
                        document: document.clone(),
                        changed: *changed,
                    })
                }
                Phase::Running => Err(InvocationError::Faulted),
                _ => Err(InvocationError::OutcomeUnavailable),
            }
        })();
        capture_timed(
            r,
            Boundary::SourceSaveAsResultRelease,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn retire_source_save_as_v1(
        &mut self,
        id: &InvocationId,
        f: &InvocationObservationPolicyControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::SourceSaveAs(c)) = &s.intent else {
                return Err(InvocationError::Unavailable);
            };
            if c.context != *f.context
                || [
                    f.owner_control_allowed,
                    f.retention_released,
                    f.delivery_drained,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(InvocationError::Unavailable);
            }
            if !c.intact(&self.namespace, id, &s.saveas_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            if !(matches!(s.phase, Phase::Terminal(_))
                || self.faulted && matches!(s.phase, Phase::Running))
            {
                return Err(InvocationError::Pending);
            }
            let s = self.invocations.slots[id.slot]
                .as_mut()
                .expect("controlled load");
            self.retention
                .release(&mut s.saveas_hold.as_mut().expect("funded load").reservation)
                .map_err(parent)?;
            s.saveas_hold = None;
            s.intent = Some(BoundIntent::RetiredSaveAs);
            Ok(())
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(
            r,
            Boundary::SourceSaveAsRetirement,
            &result,
            timing.finish(),
        );
        result
    }
}
