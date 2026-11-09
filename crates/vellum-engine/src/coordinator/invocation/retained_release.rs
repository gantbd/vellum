//! Three closed native retained-release operations over the existing publishers.
//! Host premises are not caller authentication or substrate/provider authority.
use super::super::settings::basis::{copy_context, labels};
use super::*;
use crate::resource::ResourceError;
use std::mem::size_of;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetainedReleaseOperationV1 {
    SourceBinding,
    AnnotationBinding,
    Provenance,
}
impl RetainedReleaseOperationV1 {
    /// Describes the existing typed request, not a wire grammar, new operation
    /// or execution grant. All three forms use the original native owner.
    pub const fn argument_declaration_v1(self) -> NativeArgumentDeclarationV1 {
        NativeArgumentDeclarationV1 {
            contract: "engine-retained-release-declaration/1",
            variant: NativeHandlerVariantDeclarationV1 {
                operation: match self {
                    Self::SourceBinding => "release-binding",
                    Self::AnnotationBinding => "release-annotations",
                    Self::Provenance => "release-provenance",
                },
                fields: &[
                    ("document", "DocumentIdentity"),
                    ("expected", "NativeLifecycleState"),
                    ("operation", "ClosedRetainedReleaseOperation"),
                ],
            },
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetainedReleaseRequestV1 {
    pub document: DocumentId,
    pub expected: LifecycleState,
    pub operation: RetainedReleaseOperationV1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetainedReleaseLimitsV1 {
    pub max_capture_bytes: usize,
    pub max_result_bytes: usize,
}
impl RetainedReleaseLimitsV1 {
    /// Native logical headers and copied context, not RSS/allocator accounting.
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
pub struct TrustedRetainedReleaseFactsV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub release_allowed: Option<bool>,
    pub resources_ready: Option<bool>,
    pub preparation_reserved: Option<bool>,
}
pub struct RetainedReleaseAdmissionV1<'a> {
    pub facts: &'a TrustedRetainedReleaseFactsV1<'a>,
    pub limits: RetainedReleaseLimitsV1,
    pub publication_epoch: u64,
    pub publication_allowed: Option<bool>,
    pub result_admission_allowed: Option<bool>,
    pub result_allowed: Option<bool>,
}
pub struct TrustedRetainedReleaseResultV1<'a> {
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
/// Applied/unchanged historical release; never a current binding/liveness claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetainedReleaseResultV1 {
    pub invocation: InvocationId,
    pub document: DocumentId,
    pub operation: RetainedReleaseOperationV1,
    pub changed: bool,
}
pub(super) struct Capture {
    pub(super) owner: Arc<()>,
    pub(super) context: SettingsExecutionContextV1,
    invocation: InvocationId,
    request: RetainedReleaseRequestV1,
    limits: RetainedReleaseLimitsV1,
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
        f.write_str("RetainedRelease(<redacted>)")
    }
}
impl std::fmt::Debug for Hold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RetainedReleaseHold(<owned>)")
    }
}
fn parent(e: RetentionErrorV1) -> InvocationError {
    match e {
        RetentionErrorV1::Faulted => InvocationError::Faulted,
        RetentionErrorV1::Limit => InvocationError::Limit,
        RetentionErrorV1::Unavailable => InvocationError::Unavailable,
    }
}
fn fits(c: &SettingsExecutionContextV1, l: RetainedReleaseLimitsV1) -> Result<(), InvocationError> {
    if RetainedReleaseLimitsV1::required_capture_bytes(c)? > l.max_capture_bytes {
        Err(InvocationError::Limit)
    } else {
        Ok(())
    }
}
fn allowed(f: &TrustedRetainedReleaseFactsV1<'_>) -> bool {
    [
        f.authorized,
        f.scope,
        f.continuous,
        f.live,
        f.release_allowed,
        f.resources_ready,
        f.preparation_reserved,
    ]
    .into_iter()
    .all(|v| v == Some(true))
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
            && h.as_ref().is_some_and(|h| {
                Arc::ptr_eq(&self.marker, &h.marker)
                    && ledger
                        .validate_exact(&h.reservation, self.limits.max_capture_bytes)
                        .is_ok()
            })
    }
}
impl SourceCoordinator {
    pub fn submit_retained_release_v1(
        &mut self,
        id: &InvocationId,
        request: &RetainedReleaseRequestV1,
        a: &RetainedReleaseAdmissionV1<'_>,
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
                || l.max_result_bytes < size_of::<RetainedReleaseResultV1>()
                || l.max_result_bytes > self.invocations.limits.max_intent_bytes
            {
                return Err(InvocationError::Limit);
            }
            fits(a.facts.context, l)?;
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
                let BoundIntent::RetainedRelease(c) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                if !c.intact(&self.namespace, id, &s.release_hold, &self.retention) {
                    return Err(InvocationError::Faulted);
                }
                if c.request != *request
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
                request: request.clone(),
                limits: l,
                epoch: a.publication_epoch,
                publication,
                admission,
                result,
                marker: marker.clone(),
            });
            self.invocations.slots[id.slot]
                .as_mut()
                .expect("issued release")
                .release_hold = Some(Hold {
                reservation: provisional.take().expect("funded release"),
                marker,
                candidate: None,
            });
            self.install_invocation_admission(
                id,
                BoundIntent::RetainedRelease(c.clone()),
                Phase::RetainedReleasePending(c),
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
            Boundary::RetainedReleaseAdmission,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn run_retained_release_v1(
        &mut self,
        id: &InvocationId,
        f: &TrustedRetainedReleaseFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::RetainedRelease(c)) = &s.intent else {
                return Err(InvocationError::Unavailable);
            };
            fits(f.context, c.limits)?;
            if c.context != *f.context {
                return Err(InvocationError::Unavailable);
            }
            if !c.intact(&self.namespace, id, &s.release_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            match &s.phase {
                Phase::Terminal(_) => return Ok(InvocationState::Finished),
                Phase::Running => return Err(InvocationError::Faulted),
                Phase::RetainedReleasePending(p) if Arc::ptr_eq(c, p) => {}
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
                .expect("pending release")
                .phase = Phase::Running;
            let outcome = match self.release_retained_for_invocation(id, &c.request, r) {
                Ok(changed) => {
                    if self
                        .invocation_slot(id)?
                        .release_hold
                        .as_ref()
                        .and_then(|h| h.candidate)
                        != Some(changed)
                    {
                        return Err(InvocationError::Faulted);
                    }
                    InvocationOutcome::RetainedReleaseCommitted(changed)
                }
                Err(ResourceError::Faulted) => return Err(InvocationError::Faulted),
                Err(e) => InvocationOutcome::RetainedReleaseRejected(e),
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
                    Boundary::RetainedReleaseService,
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
            Boundary::RetainedReleaseService,
            &result,
            timing.finish(),
        );
        result
    }
    pub(in crate::coordinator) fn stage_retained_release_candidate(
        &mut self,
        id: &InvocationId,
        document: &DocumentId,
        expected: LifecycleState,
        operation: RetainedReleaseOperationV1,
        changed: bool,
    ) -> Result<(), InvocationError> {
        let s = self.invocation_slot(id)?;
        let Some(BoundIntent::RetainedRelease(c)) = &s.intent else {
            return Err(InvocationError::Faulted);
        };
        if !matches!(s.phase, Phase::Running)
            || c.request.document != *document
            || c.request.expected != expected
            || c.request.operation != operation
            || !c.intact(&self.namespace, id, &s.release_hold, &self.retention)
            || s.release_hold
                .as_ref()
                .is_none_or(|h| h.candidate.is_some())
        {
            return Err(InvocationError::Faulted);
        }
        self.invocations.slots[id.slot]
            .as_mut()
            .expect("running release")
            .release_hold
            .as_mut()
            .expect("funded release")
            .candidate = Some(changed);
        Ok(())
    }
    pub fn retained_release_result_v1(
        &self,
        id: &InvocationId,
        f: &TrustedRetainedReleaseResultV1<'_>,
        r: &mut Recorder,
    ) -> Result<RetainedReleaseResultV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::RetainedRelease(c)) = &s.intent else {
                return Err(InvocationError::OutcomeUnavailable);
            };
            fits(f.context, c.limits)?;
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
            if !c.intact(&self.namespace, id, &s.release_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            match &s.phase {
                Phase::Terminal(Some(InvocationOutcome::RetainedReleaseCommitted(changed))) => {
                    if s.release_hold.as_ref().and_then(|h| h.candidate) != Some(*changed) {
                        return Err(InvocationError::Faulted);
                    }
                    Ok(RetainedReleaseResultV1 {
                        invocation: id.clone(),
                        document: c.request.document.clone(),
                        operation: c.request.operation,
                        changed: *changed,
                    })
                }
                Phase::Running => Err(InvocationError::Faulted),
                _ => Err(InvocationError::OutcomeUnavailable),
            }
        })();
        capture_timed(
            r,
            Boundary::RetainedReleaseResultRelease,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn retire_retained_release_v1(
        &mut self,
        id: &InvocationId,
        f: &InvocationObservationPolicyControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::RetainedRelease(c)) = &s.intent else {
                return Err(InvocationError::Unavailable);
            };
            fits(f.context, c.limits)?;
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
            if !c.intact(&self.namespace, id, &s.release_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            if !(matches!(s.phase, Phase::Terminal(_))
                || self.faulted && matches!(s.phase, Phase::Running))
            {
                return Err(InvocationError::Pending);
            }
            let s = self.invocations.slots[id.slot]
                .as_mut()
                .expect("controlled release");
            self.retention
                .release(&mut s.release_hold.as_mut().expect("funded release").reservation)
                .map_err(parent)?;
            s.release_hold = None;
            s.intent = Some(BoundIntent::RetiredRelease);
            Ok(())
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(
            r,
            Boundary::RetainedReleaseRetirement,
            &result,
            timing.finish(),
        );
        result
    }
}
