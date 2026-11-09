//! Owned pure source rebinding over the one existing native preparer and publisher.
//! Host premises are not caller authentication or substrate/provider authority.
use super::super::settings::basis::{copy_context, labels};
use super::*;
use crate::resource::ResourceError;
use std::mem::size_of;

/// Original native roles and bounds; no path, locator lookup or effect authority.
pub struct SourceRebindRequestV1 {
    pub document: DocumentId,
    pub target: crate::resource::SourceBindingTargetV1,
    pub options: crate::resource::SourceRebindOptionsV1,
}
impl SourceRebindRequestV1 {
    /// Descriptive native roles only, not a transport grammar or execution grant.
    pub const fn argument_declaration_v1() -> NativeArgumentDeclarationV1 {
        NativeArgumentDeclarationV1 {
            contract: "engine-source-rebind-declaration/1",
            variant: NativeHandlerVariantDeclarationV1 {
                operation: "rebind",
                fields: &[
                    ("document", "DocumentIdentity"),
                    ("target", "OriginalSourceBindingTargetV1"),
                    ("options", "OriginalSourceRebindOptionsV1"),
                ],
            },
        }
    }
    fn retained_bytes(&self) -> Result<usize, InvocationError> {
        let mut bytes = size_of::<Self>();
        bytes = bytes
            .checked_add(self.target.identity.key.0.len())
            .ok_or(InvocationError::Limit)?;
        if let Some(pair) = &self.options.pair {
            for key in [
                &pair.check.source,
                &pair.check.annotations,
                &pair.check.proposed,
            ]
            .into_iter()
            .flatten()
            {
                bytes = bytes
                    .checked_add(key.identity.key.0.len())
                    .ok_or(InvocationError::Limit)?;
            }
        }
        Ok(bytes)
    }
    fn owned_by(&self, owner: &Arc<()>) -> bool {
        Arc::ptr_eq(&self.document.namespace, owner)
            && Arc::ptr_eq(&self.target.registration.owner, owner)
            && self.options.pair.as_ref().is_none_or(|p| {
                Arc::ptr_eq(&p.check.owner, owner) && p.check.document == self.document
            })
    }
}
fn copy_request(q: &SourceRebindRequestV1) -> SourceRebindRequestV1 {
    use crate::resource::{
        OrdinaryPairCheck, OrdinaryPairEvidence, SourceBindingTargetV1, SourceRebindOptionsV1,
    };
    SourceRebindRequestV1 {
        document: q.document.clone(),
        target: SourceBindingTargetV1 {
            registration: q.target.registration.clone(),
            identity: q.target.identity.clone(),
        },
        options: SourceRebindOptionsV1 {
            expected_binding: q.options.expected_binding,
            max_binding_history: q.options.max_binding_history,
            pair: q.options.pair.as_ref().map(|p| OrdinaryPairEvidence {
                relation: p.relation,
                check: OrdinaryPairCheck {
                    owner: p.check.owner.clone(),
                    marker: p.check.marker.clone(),
                    document: p.check.document.clone(),
                    source: p.check.source.clone(),
                    annotations: p.check.annotations.clone(),
                    proposed: p.check.proposed.clone(),
                },
            }),
        },
    }
}
fn same_request(a: &SourceRebindRequestV1, b: &SourceRebindRequestV1) -> bool {
    a.document == b.document
        && a.target.identity == b.target.identity
        && Arc::ptr_eq(&a.target.registration, &b.target.registration)
        && same_options(&a.options, &b.options)
}
fn same_options(
    a: &crate::resource::SourceRebindOptionsV1,
    b: &crate::resource::SourceRebindOptionsV1,
) -> bool {
    a.expected_binding == b.expected_binding
        && a.max_binding_history == b.max_binding_history
        && match (&a.pair, &b.pair) {
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceRebindLimitsV1 {
    pub max_capture_bytes: usize,
    pub max_result_bytes: usize,
}
impl SourceRebindLimitsV1 {
    /// Complete native capture, retained role payload and service-copy staging;
    /// shared registration storage, caller input and allocator/RSS are excluded.
    pub fn required_capture_bytes(
        c: &SettingsExecutionContextV1,
        request: &SourceRebindRequestV1,
    ) -> Result<usize, InvocationError> {
        let base = (size_of::<Capture>() + size_of::<Hold>())
            .checked_add(request.retained_bytes()?)
            .ok_or(InvocationError::Limit)?;
        labels(c).into_iter().try_fold(base, |n, s| {
            n.checked_add(s.len()).ok_or(InvocationError::Limit)
        })
    }
}
pub struct TrustedSourceRebindInvocationFactsV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub native: &'a TrustedSourceRebindFactsV1,
}
pub struct SourceRebindAdmissionV1<'a> {
    pub facts: &'a TrustedSourceRebindInvocationFactsV1<'a>,
    pub limits: SourceRebindLimitsV1,
    pub publication_epoch: u64,
    pub publication_allowed: Option<bool>,
    pub result_admission_allowed: Option<bool>,
    pub result_allowed: Option<bool>,
}
pub struct TrustedSourceRebindResultV1<'a> {
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
/// Applied/unchanged historical rebind; never a savedness or current-binding claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRebindResultV1 {
    pub invocation: InvocationId,
    pub document: DocumentId,
    pub changed: bool,
}
pub(super) struct Capture {
    pub(super) owner: Arc<()>,
    pub(super) context: SettingsExecutionContextV1,
    invocation: InvocationId,
    request: SourceRebindRequestV1,
    limits: SourceRebindLimitsV1,
    epoch: u64,
    publication: bool,
    admission: bool,
    result: bool,
    marker: Arc<()>,
}
pub(super) struct Hold {
    reservation: retention::Reservation,
    marker: Arc<()>,
    pub(super) candidate: Option<bool>,
}
impl std::fmt::Debug for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceRebind(<redacted>)")
    }
}
impl std::fmt::Debug for Hold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceRebindHold(<owned>)")
    }
}
fn parent(e: RetentionErrorV1) -> InvocationError {
    match e {
        RetentionErrorV1::Faulted => InvocationError::Faulted,
        RetentionErrorV1::Limit => InvocationError::Limit,
        RetentionErrorV1::Unavailable => InvocationError::Unavailable,
    }
}
fn fits(
    c: &SettingsExecutionContextV1,
    request: &SourceRebindRequestV1,
    l: SourceRebindLimitsV1,
) -> Result<(), InvocationError> {
    if SourceRebindLimitsV1::required_capture_bytes(c, request)? > l.max_capture_bytes {
        Err(InvocationError::Limit)
    } else {
        Ok(())
    }
}
fn allowed(f: &TrustedSourceRebindInvocationFactsV1<'_>) -> bool {
    super::super::resource::source_binding::rebind_authorized(f.native)
}
shutdown_capture_reservation!();
impl Capture {
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.request.document == *document
    }
    pub(super) fn intact(
        &self,
        owner: &Arc<()>,
        id: &InvocationId,
        h: &Option<Hold>,
        ledger: &retention::RetentionState,
    ) -> bool {
        Arc::ptr_eq(&self.owner, owner)
            && Arc::ptr_eq(&self.request.document.namespace, owner)
            && self.invocation == *id
            && self.request.owned_by(owner)
            && SourceRebindLimitsV1::required_capture_bytes(&self.context, &self.request)
                .ok()
                .is_some_and(|n| n <= self.limits.max_capture_bytes)
            && h.as_ref().is_some_and(|h| {
                Arc::ptr_eq(&self.marker, &h.marker)
                    && ledger
                        .validate_exact(&h.reservation, self.limits.max_capture_bytes)
                        .is_ok()
            })
    }
}
impl SourceCoordinator {
    pub fn submit_source_rebind_v1(
        &mut self,
        id: &InvocationId,
        request: &SourceRebindRequestV1,
        a: &SourceRebindAdmissionV1<'_>,
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
            if l.max_capture_bytes == 0
                || l.max_capture_bytes > self.invocations.limits.max_intent_bytes
                || l.max_result_bytes < size_of::<SourceRebindResultV1>()
                || l.max_result_bytes > self.invocations.limits.max_intent_bytes
            {
                return Err(InvocationError::Limit);
            }
            fits(a.facts.context, request, l)?;
            if labels(a.facts.context).iter().any(|s| s.is_empty()) || !allowed(a.facts) {
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
                let BoundIntent::SourceRebind(c) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                if !c.intact(&self.namespace, id, &s.rebind_hold, &self.retention) {
                    return Err(InvocationError::Faulted);
                }
                if !same_request(&c.request, request)
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
            self.retained_document(&request.document).map_err(|e| {
                if matches!(e, CoordinatorError::Faulted) {
                    InvocationError::Faulted
                } else {
                    InvocationError::Unavailable
                }
            })?;
            if !request.owned_by(&self.namespace) {
                return Err(InvocationError::Unavailable);
            }
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
                request: copy_request(request),
                limits: l,
                epoch: a.publication_epoch,
                publication,
                admission,
                result,
                marker: marker.clone(),
            });
            self.invocations.slots[id.slot]
                .as_mut()
                .expect("issued rebind")
                .rebind_hold = Some(Hold {
                reservation: provisional.take().expect("funded rebind"),
                marker,
                candidate: None,
            });
            self.install_invocation_admission(
                id,
                BoundIntent::SourceRebind(c.clone()),
                Phase::SourceRebindPending(c),
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
        capture_timed(r, Boundary::SourceRebindAdmission, &result, timing.finish());
        result
    }
    pub fn run_source_rebind_v1(
        &mut self,
        id: &InvocationId,
        f: &TrustedSourceRebindInvocationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::SourceRebind(c)) = &s.intent else {
                return Err(InvocationError::Unavailable);
            };
            fits(f.context, &c.request, c.limits)?;
            if c.context != *f.context {
                return Err(InvocationError::Unavailable);
            }
            if !c.intact(&self.namespace, id, &s.rebind_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            match &s.phase {
                Phase::Terminal(_) => return Ok(InvocationState::Finished),
                Phase::Running => return Err(InvocationError::Faulted),
                Phase::SourceRebindPending(p) if Arc::ptr_eq(c, p) => {}
                _ => return Err(InvocationError::Faulted),
            }
            let c = c.clone();
            if self.service_requested_stop(id, r)? {
                return Ok(InvocationState::Finished);
            }
            if !allowed(f) {
                return Err(InvocationError::Unavailable);
            }
            self.invocations.slots[id.slot]
                .as_mut()
                .expect("pending rebind")
                .phase = Phase::Running;
            let outcome = match self.source_rebind_for_invocation(id, &c.request, f.native, r) {
                Ok(changed) => {
                    if self
                        .invocation_slot(id)?
                        .rebind_hold
                        .as_ref()
                        .and_then(|h| h.candidate)
                        != Some(changed)
                    {
                        return Err(InvocationError::Faulted);
                    }
                    InvocationOutcome::SourceRebindCommitted(changed)
                }
                Err(ResourceError::Faulted) => return Err(InvocationError::Faulted),
                Err(e) => InvocationOutcome::SourceRebindRejected(e),
            };
            self.install_invocation_terminal(id, outcome);
            Ok(InvocationState::Finished)
        }));
        let result = match caught {
            Ok(v) => v,
            Err(panic) => {
                self.faulted = true;
                capture_timed(
                    r,
                    Boundary::SourceRebindService,
                    &Err::<InvocationState, _>(InvocationError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(panic)
            }
        };
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(r, Boundary::SourceRebindService, &result, timing.finish());
        result
    }
    fn source_rebind_for_invocation(
        &mut self,
        id: &InvocationId,
        request: &SourceRebindRequestV1,
        facts: &TrustedSourceRebindFactsV1,
        r: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        let copy = copy_request(request);
        let prepared =
            self.prepare_source_rebind_native(&copy.document, copy.target, copy.options, facts, r)?;
        self.publish_source_rebind_native_for_invocation(prepared, facts, Some(id), r)
    }
    pub(in crate::coordinator) fn stage_source_rebind_candidate(
        &mut self,
        id: &InvocationId,
        document: &DocumentId,
        target: &crate::resource::SourceBindingTargetV1,
        options: &crate::resource::SourceRebindOptionsV1,
        changed: bool,
    ) -> Result<(), InvocationError> {
        let s = self.invocation_slot(id)?;
        let Some(BoundIntent::SourceRebind(c)) = &s.intent else {
            return Err(InvocationError::Faulted);
        };
        if !matches!(s.phase, Phase::Running)
            || c.request.document != *document
            || c.request.target.identity != target.identity
            || !Arc::ptr_eq(&c.request.target.registration, &target.registration)
            || !same_options(&c.request.options, options)
            || !c.intact(&self.namespace, id, &s.rebind_hold, &self.retention)
            || s.rebind_hold.as_ref().is_none_or(|h| h.candidate.is_some())
        {
            return Err(InvocationError::Faulted);
        }
        self.invocations.slots[id.slot]
            .as_mut()
            .expect("running rebind")
            .rebind_hold
            .as_mut()
            .expect("funded rebind")
            .candidate = Some(changed);
        Ok(())
    }
    pub fn source_rebind_result_v1(
        &self,
        id: &InvocationId,
        f: &TrustedSourceRebindResultV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourceRebindResultV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::SourceRebind(c)) = &s.intent else {
                return Err(InvocationError::OutcomeUnavailable);
            };
            fits(f.context, &c.request, c.limits)?;
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
            if !c.intact(&self.namespace, id, &s.rebind_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            match &s.phase {
                Phase::Terminal(Some(InvocationOutcome::SourceRebindCommitted(changed))) => {
                    if s.rebind_hold.as_ref().and_then(|h| h.candidate) != Some(*changed) {
                        return Err(InvocationError::Faulted);
                    }
                    Ok(SourceRebindResultV1 {
                        invocation: id.clone(),
                        document: c.request.document.clone(),
                        changed: *changed,
                    })
                }
                Phase::Running => Err(InvocationError::Faulted),
                _ => Err(InvocationError::OutcomeUnavailable),
            }
        })();
        capture_timed(
            r,
            Boundary::SourceRebindResultRelease,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn retire_source_rebind_v1(
        &mut self,
        id: &InvocationId,
        f: &InvocationObservationPolicyControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::SourceRebind(c)) = &s.intent else {
                return Err(InvocationError::Unavailable);
            };
            fits(f.context, &c.request, c.limits)?;
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
            if !c.intact(&self.namespace, id, &s.rebind_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            if !(matches!(s.phase, Phase::Terminal(_))
                || self.faulted && matches!(s.phase, Phase::Running))
            {
                return Err(InvocationError::Pending);
            }
            let s = self.invocations.slots[id.slot]
                .as_mut()
                .expect("controlled rebind");
            self.retention
                .release(&mut s.rebind_hold.as_mut().expect("funded rebind").reservation)
                .map_err(parent)?;
            s.rebind_hold = None;
            s.intent = Some(BoundIntent::RetiredRebind);
            Ok(())
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(
            r,
            Boundary::SourceRebindRetirement,
            &result,
            timing.finish(),
        );
        result
    }
}
