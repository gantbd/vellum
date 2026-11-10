//! Original-ticket retained disposal. Host facts are premises, not authentication.
use super::super::settings::basis::{copy_context, labels};
use super::*;
use std::mem::size_of;
mod explicit;
#[cfg(test)]
mod tests;
pub use explicit::{
    DisposalDispositionV1, ExplicitDisposalRequestV1, PreparedExplicitDisposalV1,
    TrustedDisposalCoverageV1,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetainedDisposalLimitsV1 {
    pub max_capture_bytes: usize,
    pub max_result_bytes: usize,
}
impl RetainedDisposalLimitsV1 {
    /// Logical native headers and copied labels, not allocator/RSS accounting.
    pub fn required_capture_bytes(
        c: &SettingsExecutionContextV1,
    ) -> Result<usize, InvocationError> {
        labels(c)
            .into_iter()
            .try_fold(size_of::<Capture>() + size_of::<Hold>(), |n, s| {
                n.checked_add(s.len()).ok_or(InvocationError::Limit)
            })
    }
}
pub struct TrustedRetainedDisposalFactsV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub disposal_allowed: Option<bool>,
    pub resources_ready: Option<bool>,
    pub preparation_reserved: Option<bool>,
}
pub struct RetainedDisposalAdmissionV1<'a> {
    pub facts: &'a TrustedRetainedDisposalFactsV1<'a>,
    pub limits: RetainedDisposalLimitsV1,
    pub publication_epoch: u64,
    pub publication_allowed: Option<bool>,
    pub result_admission_allowed: Option<bool>,
    pub result_allowed: Option<bool>,
}
pub struct TrustedRetainedDisposalResultV1<'a> {
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
/// This specific type denotes an applied retained-disposal operation only.
/// No current-root lifecycle counters or payload are disclosed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetainedDisposalResultV1 {
    pub invocation: InvocationId,
    pub document: DocumentId,
}
pub(super) struct Capture {
    pub(super) owner: Arc<()>,
    pub(super) context: SettingsExecutionContextV1,
    invocation: InvocationId,
    plan: PreparedDisposal,
    limits: RetainedDisposalLimitsV1,
    epoch: u64,
    publication: bool,
    admission: bool,
    result: bool,
    marker: Arc<()>,
    explicit: Option<ExplicitDisposalRequestV1>,
}
pub(super) struct Hold {
    reservation: retention::Reservation,
    marker: Arc<()>,
    pub(super) candidate: Option<LifecycleState>,
    explicit: Option<ExplicitDisposalRequestV1>,
}
impl std::fmt::Debug for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RetainedDisposal(<redacted>)")
    }
}
impl std::fmt::Debug for Hold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RetainedDisposalHold(<owned>)")
    }
}
fn parent(e: RetentionErrorV1) -> InvocationError {
    match e {
        RetentionErrorV1::Faulted => InvocationError::Faulted,
        RetentionErrorV1::Limit => InvocationError::Limit,
        RetentionErrorV1::Unavailable => InvocationError::Unavailable,
    }
}
fn execution(f: &TrustedRetainedDisposalFactsV1<'_>) -> bool {
    [
        f.authorized,
        f.scope,
        f.continuous,
        f.live,
        f.disposal_allowed,
        f.resources_ready,
        f.preparation_reserved,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn context_fits(
    c: &SettingsExecutionContextV1,
    l: RetainedDisposalLimitsV1,
) -> Result<(), InvocationError> {
    if RetainedDisposalLimitsV1::required_capture_bytes(c)? > l.max_capture_bytes {
        return Err(InvocationError::Limit);
    }
    Ok(())
}
shutdown_capture_reservation!();
impl Capture {
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.plan.document() == document
    }
    pub(super) fn intact(
        &self,
        owner: &Arc<()>,
        id: &InvocationId,
        hold: &Option<Hold>,
        ledger: &retention::RetentionState,
    ) -> bool {
        Arc::ptr_eq(&self.owner, owner)
            && self.invocation == *id
            && self.plan.owned_by(owner)
            && self.explicit.as_ref().is_none_or(|q| {
                self.plan
                    .matches_explicit_core(&q.source, q.annotation_generation, q.lifecycle)
            })
            && hold.as_ref().is_some_and(|h| {
                Arc::ptr_eq(&self.marker, &h.marker)
                    && h.explicit == self.explicit
                    && ledger
                        .validate_exact(&h.reservation, self.limits.max_capture_bytes)
                        .is_ok()
            })
    }
}
/// The data route may prepare only after original-ticket replay and parent funding.
#[derive(Clone, Copy)]
enum DisposalInput<'a> {
    Prepared {
        plan: &'a PreparedDisposal,
        explicit: Option<&'a ExplicitDisposalRequestV1>,
    },
    Request(&'a ExplicitDisposalRequestV1),
}
fn preparation_error(e: CoordinatorError) -> InvocationError {
    use crate::resource::ResourceError;
    match e {
        CoordinatorError::Unavailable
        | CoordinatorError::Stale
        | CoordinatorError::Resource(ResourceError::Retained) => InvocationError::Unavailable,
        CoordinatorError::Limit | CoordinatorError::Resource(ResourceError::Limit) => {
            InvocationError::Limit
        }
        CoordinatorError::Capacity | CoordinatorError::Resource(ResourceError::Capacity) => {
            InvocationError::Capacity
        }
        // The preparation path has no other declared refusal products. A surprise
        // is a fence, not an invented ordinary rejection or permission to refresh.
        _ => InvocationError::Faulted,
    }
}
impl SourceCoordinator {
    pub fn submit_retained_disposal_v1(
        &mut self,
        id: &InvocationId,
        plan: &PreparedDisposal,
        a: &RetainedDisposalAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_retained_disposal_impl(
            id,
            DisposalInput::Prepared {
                plan,
                explicit: None,
            },
            a,
            None,
            r,
        )
    }
    fn submit_retained_disposal_impl(
        &mut self,
        id: &InvocationId,
        input: DisposalInput<'_>,
        a: &RetainedDisposalAdmissionV1<'_>,
        coverage: Option<&TrustedDisposalCoverageV1>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let mut provisional = None;
        let mut result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let s = self.invocation_slot(id)?;
            let (plan, explicit) = match input {
                DisposalInput::Prepared { plan, explicit } => (Some(plan), explicit),
                DisposalInput::Request(q) => (None, Some(q)),
            };
            match (explicit, coverage) {
                (None, None) => {}
                (Some(q), Some(g)) if g.allows(q) => {}
                _ => return Err(InvocationError::Unavailable),
            }
            if plan.is_some_and(|p| {
                explicit.is_some_and(|q| {
                    !p.matches_explicit_core(&q.source, q.annotation_generation, q.lifecycle)
                })
            }) {
                return Err(InvocationError::Faulted);
            }
            let l = a.limits;
            if l.max_capture_bytes == 0
                || l.max_capture_bytes > self.invocations.limits.max_intent_bytes
                || l.max_result_bytes < size_of::<RetainedDisposalResultV1>()
                || l.max_result_bytes > self.invocations.limits.max_intent_bytes
            {
                return Err(InvocationError::Limit);
            }
            context_fits(a.facts.context, l)?;
            if labels(a.facts.context).iter().any(|v| v.is_empty())
                || plan.is_some_and(|p| !p.owned_by(&self.namespace))
                || !execution(a.facts)
            {
                return Err(InvocationError::Unavailable);
            }
            let (Some(publication), Some(admission), Some(allowed)) = (
                a.publication_allowed,
                a.result_admission_allowed,
                a.result_allowed,
            ) else {
                return Err(InvocationError::Unavailable);
            };
            if let Some(intent) = &s.intent {
                let BoundIntent::RetainedDisposal(c) = intent else {
                    return Err(InvocationError::IntentMismatch);
                };
                if !c.intact(&self.namespace, id, &s.disposal_hold, &self.retention) {
                    return Err(InvocationError::Faulted);
                }
                if plan.is_some_and(|p| !c.plan.same_capture(p))
                    || c.explicit.as_ref() != explicit
                    || c.context != *a.facts.context
                    || c.limits != l
                    || c.epoch != a.publication_epoch
                    || c.publication != publication
                    || c.admission != admission
                    || c.result != allowed
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return s.phase.state();
            }
            if !matches!(s.phase, Phase::Issued(_)) {
                return Err(InvocationError::Unavailable);
            }
            provisional = Some(
                self.retention
                    .reserve(l.max_capture_bytes)
                    .map_err(parent)?,
            );
            let plan = match input {
                DisposalInput::Prepared { plan, .. } => plan.clone(),
                DisposalInput::Request(q) => {
                    self.prepare_explicit_disposal_v1(q, r)
                        .map_err(preparation_error)?
                        .plan
                }
            };
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
                plan,
                limits: l,
                epoch: a.publication_epoch,
                publication,
                admission,
                result: allowed,
                marker: marker.clone(),
                explicit: explicit.cloned(),
            });
            let hold = Hold {
                reservation: provisional.take().expect("funded disposal"),
                marker,
                candidate: None,
                explicit: explicit.cloned(),
            };
            self.invocations.slots[id.slot]
                .as_mut()
                .expect("issued disposal")
                .disposal_hold = Some(hold);
            self.install_invocation_admission(
                id,
                BoundIntent::RetainedDisposal(c.clone()),
                Phase::RetainedDisposalPending(c),
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
        capture_timed(
            r,
            Boundary::RetainedDisposalAdmission,
            &result,
            timing.finish(),
        );
        result
    }
    /// Fresh service premises; neither replay nor an observation starts this work.
    pub fn run_retained_disposal_v1(
        &mut self,
        id: &InvocationId,
        f: &TrustedRetainedDisposalFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.run_retained_disposal_impl(id, f, None, r)
    }
    fn run_retained_disposal_impl(
        &mut self,
        id: &InvocationId,
        f: &TrustedRetainedDisposalFactsV1<'_>,
        coverage: Option<&TrustedDisposalCoverageV1>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::RetainedDisposal(c)) = &s.intent else {
                return Err(InvocationError::Unavailable);
            };
            if c.explicit.is_some() != coverage.is_some() {
                return Err(InvocationError::Unavailable);
            }
            context_fits(f.context, c.limits)?;
            if c.context != *f.context {
                return Err(InvocationError::Unavailable);
            }
            if !c.intact(&self.namespace, id, &s.disposal_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            match &s.phase {
                Phase::Terminal(_) => return Ok(InvocationState::Finished),
                Phase::Running => return Err(InvocationError::Faulted),
                Phase::RetainedDisposalPending(p) if Arc::ptr_eq(c, p) => {}
                _ => return Err(InvocationError::Faulted),
            }
            let c = c.clone();
            // Protected stop settlement does not need continuing edit/disclosure rights.
            if self.service_requested_stop(id, r)? {
                return Ok(InvocationState::Finished);
            }
            if !execution(f)
                || c.explicit
                    .as_ref()
                    .is_some_and(|q| coverage.is_none_or(|g| !g.allows(q)))
            {
                return Err(InvocationError::Unavailable);
            }
            self.invocations.slots[id.slot]
                .as_mut()
                .expect("pending disposal")
                .phase = Phase::Running;
            let published =
                self.commit_retained_disposal_for_invocation(c.plan.clone(), Some(id), r);
            let out = match published {
                Ok(state) => {
                    if self
                        .invocation_slot(id)?
                        .disposal_hold
                        .as_ref()
                        .and_then(|h| h.candidate)
                        != Some(state)
                    {
                        return Err(InvocationError::Faulted);
                    }
                    InvocationOutcome::RetainedDisposalCommitted(state)
                }
                Err(
                    CoordinatorError::Faulted
                    | CoordinatorError::Resource(crate::resource::ResourceError::Faulted),
                ) => {
                    return Err(InvocationError::Faulted);
                }
                Err(e) => InvocationOutcome::RetainedDisposalRejected(e),
            };
            self.install_invocation_terminal(id, out);
            Ok(InvocationState::Finished)
        }));
        let result = match caught {
            Ok(v) => v,
            Err(panic) => {
                self.faulted = true;
                capture_timed(
                    r,
                    Boundary::RetainedDisposalService,
                    &Err::<InvocationState, _>(InvocationError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(panic)
            }
        };
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(
            r,
            Boundary::RetainedDisposalService,
            &result,
            timing.finish(),
        );
        result
    }
    /// Staged inside the actual publisher before any root installation.
    pub(in crate::coordinator) fn stage_retained_disposal_candidate(
        &mut self,
        id: &InvocationId,
        plan: &PreparedDisposal,
        next: LifecycleState,
    ) -> Result<(), InvocationError> {
        let s = self.invocation_slot(id)?;
        let Some(BoundIntent::RetainedDisposal(c)) = &s.intent else {
            return Err(InvocationError::Faulted);
        };
        if let Some(q) = &c.explicit {
            self.validate_explicit_disposal_request(q)
                .map_err(|_| InvocationError::Faulted)?;
        }
        if !matches!(s.phase, Phase::Running)
            || !c.plan.same_capture(plan)
            || !c.intact(&self.namespace, id, &s.disposal_hold, &self.retention)
            || s.disposal_hold
                .as_ref()
                .is_none_or(|h| h.candidate.is_some())
        {
            return Err(InvocationError::Faulted);
        }
        self.invocations.slots[id.slot]
            .as_mut()
            .expect("running disposal")
            .disposal_hold
            .as_mut()
            .expect("validated hold")
            .candidate = Some(next);
        Ok(())
    }
    pub fn retained_disposal_result_v1(
        &self,
        id: &InvocationId,
        f: &TrustedRetainedDisposalResultV1<'_>,
        r: &mut Recorder,
    ) -> Result<RetainedDisposalResultV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::RetainedDisposal(c)) = &s.intent else {
                return Err(InvocationError::OutcomeUnavailable);
            };
            context_fits(f.context, c.limits)?;
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
            if !c.intact(&self.namespace, id, &s.disposal_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            match &s.phase {
                Phase::Terminal(Some(InvocationOutcome::RetainedDisposalCommitted(state))) => {
                    if s.disposal_hold.as_ref().and_then(|h| h.candidate) != Some(*state) {
                        return Err(InvocationError::Faulted);
                    }
                    Ok(RetainedDisposalResultV1 {
                        invocation: id.clone(),
                        document: c.plan.document().clone(),
                    })
                }
                Phase::Running => Err(InvocationError::Faulted),
                _ => Err(InvocationError::OutcomeUnavailable),
            }
        })();
        capture_timed(
            r,
            Boundary::RetainedDisposalResultRelease,
            &result,
            timing.finish(),
        );
        result
    }
    /// Discharges only this capture; never settles Unknown or refunds issuance.
    pub fn retire_retained_disposal_v1(
        &mut self,
        id: &InvocationId,
        f: &InvocationObservationPolicyControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::RetainedDisposal(c)) = &s.intent else {
                return Err(InvocationError::Unavailable);
            };
            context_fits(f.context, c.limits)?;
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
            if !c.intact(&self.namespace, id, &s.disposal_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            if !(matches!(s.phase, Phase::Terminal(_))
                || self.faulted && matches!(s.phase, Phase::Running))
            {
                return Err(InvocationError::Pending);
            }
            let s = self.invocations.slots[id.slot]
                .as_mut()
                .expect("controlled disposal");
            self.retention
                .release(
                    &mut s
                        .disposal_hold
                        .as_mut()
                        .expect("funded disposal")
                        .reservation,
                )
                .map_err(parent)?;
            s.disposal_hold = None;
            s.intent = Some(BoundIntent::RetiredDisposal);
            Ok(())
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(
            r,
            Boundary::RetainedDisposalRetirement,
            &result,
            timing.finish(),
        );
        result
    }
}
