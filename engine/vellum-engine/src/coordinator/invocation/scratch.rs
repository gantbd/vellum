//! One original scratch allocation. Host premises are not recipient credentials.
use super::super::settings::basis::{copy_context, labels};
use super::*;
use std::mem::size_of;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScratchInvocationLimitsV1 {
    pub max_capture_bytes: usize,
    pub max_result_bytes: usize,
}
impl ScratchInvocationLimitsV1 {
    pub fn required_capture_bytes(
        c: &SettingsExecutionContextV1,
        q: &ScratchDocumentRequestV1<'_>,
    ) -> Result<usize, InvocationError> {
        let b = size_of::<Capture>()
            .checked_add(size_of::<Hold>())
            .and_then(|n| n.checked_add(q.source.len()))
            .ok_or(InvocationError::Limit)?;
        labels(c).into_iter().try_fold(b, |n, s| {
            n.checked_add(s.len()).ok_or(InvocationError::Limit)
        })
    }
}
/// Trusted embedding premises, not credentials accepted from an untrusted caller.
/// Every applicable premise must be known true at admission and actual service.
pub struct TrustedScratchFactsV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub interpretation_current: Option<bool>,
    pub resources_ready: Option<bool>,
    pub companions_ready: Option<bool>,
    pub versions_available: Option<bool>,
    pub reply_ready: Option<bool>,
    pub lifecycle_allowed: Option<bool>,
    pub coverage_complete: Option<bool>,
    pub creation_allowed: Option<bool>,
}
pub struct ScratchInvocationAdmissionV1<'a> {
    pub facts: &'a TrustedScratchFactsV1<'a>,
    pub limits: ScratchInvocationLimitsV1,
    pub publication_epoch: u64,
    pub publication_allowed: Option<bool>,
    pub result_admission_allowed: Option<bool>,
    pub result_identifiers_allowed: Option<bool>,
}
/// Current authorization for historical identity only; no current row is read.
pub struct TrustedScratchResultV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub publication_epoch: u64,
    pub publication_allowed: Option<bool>,
    pub outcome_readable: Option<bool>,
    pub result_readable: Option<bool>,
    pub result_context_current: Option<bool>,
    pub historical_document_readable: Option<bool>,
    pub reply_ready: Option<bool>,
    pub resources_ready: Option<bool>,
    pub max_result_bytes: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScratchResultV1 {
    pub invocation: InvocationId,
    pub document: DocumentId,
}
pub(super) struct Capture {
    pub(super) owner: Arc<()>,
    pub(super) context: SettingsExecutionContextV1,
    invocation: InvocationId,
    source: String,
    serialization: SourceSerializationV1,
    insertion_policy: GeneratedNewlineV1,
    profile: CoordinatorLimits,
    limits: ScratchInvocationLimitsV1,
    epoch: u64,
    publication: bool,
    admission: bool,
    result: bool,
    marker: Arc<()>,
}
pub(super) struct Hold {
    reservation: retention::Reservation,
    marker: Arc<()>,
    pub(super) candidate: Option<DocumentVersion>,
}
impl std::fmt::Debug for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ScratchInvocation(<redacted>)")
    }
}
impl std::fmt::Debug for Hold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ScratchHold(<owned>)")
    }
}
fn parent(e: RetentionErrorV1) -> InvocationError {
    match e {
        RetentionErrorV1::Faulted => InvocationError::Faulted,
        RetentionErrorV1::Limit => InvocationError::Limit,
        RetentionErrorV1::Unavailable => InvocationError::Unavailable,
    }
}
fn allowed(f: &TrustedScratchFactsV1<'_>) -> bool {
    [
        f.authorized,
        f.scope,
        f.continuous,
        f.live,
        f.not_stopped,
        f.interpretation_current,
        f.resources_ready,
        f.companions_ready,
        f.versions_available,
        f.reply_ready,
        f.lifecycle_allowed,
        f.coverage_complete,
        f.creation_allowed,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn faulted(e: &CoordinatorError) -> bool {
    matches!(
        e,
        CoordinatorError::Faulted
            | CoordinatorError::History(HistoryError::Faulted)
            | CoordinatorError::Resource(crate::resource::ResourceError::Faulted)
    )
}
shutdown_capture_reservation!();
impl Capture {
    pub(super) fn intact(
        &self,
        owner: &Arc<()>,
        id: &InvocationId,
        h: &Option<Hold>,
        ledger: &retention::RetentionState,
    ) -> bool {
        Arc::ptr_eq(&self.owner, owner)
            && self.invocation == *id
            && h.as_ref().is_some_and(|h| {
                Arc::ptr_eq(&self.marker, &h.marker)
                    && ledger
                        .validate_exact(&h.reservation, self.limits.max_capture_bytes)
                        .is_ok()
            })
    }
    fn request(&self) -> ScratchDocumentRequestV1<'_> {
        ScratchDocumentRequestV1 {
            source: self.source.as_bytes(),
            serialization: self.serialization,
            insertion_policy: self.insertion_policy,
        }
    }
}
impl SourceCoordinator {
    /// Fund and capture one immutable creation intent without allocating a document.
    /// Matching replay observes this ticket; changed intent cannot replace it.
    pub fn submit_scratch_v1(
        &mut self,
        id: &InvocationId,
        q: &ScratchDocumentRequestV1<'_>,
        a: &ScratchInvocationAdmissionV1<'_>,
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
                || l.max_result_bytes < size_of::<ScratchResultV1>()
                || l.max_result_bytes > self.invocations.limits.max_intent_bytes
                || ScratchInvocationLimitsV1::required_capture_bytes(a.facts.context, q)?
                    > l.max_capture_bytes
            {
                return Err(InvocationError::Limit);
            }
            if labels(a.facts.context).iter().any(|s| s.is_empty()) || !allowed(a.facts) {
                return Err(InvocationError::Unavailable);
            }
            let (Some(publication), Some(admission), Some(result)) = (
                a.publication_allowed,
                a.result_admission_allowed,
                a.result_identifiers_allowed,
            ) else {
                return Err(InvocationError::Unavailable);
            };
            if let Some(original) = &s.intent {
                let BoundIntent::Scratch(c) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                if !c.intact(&self.namespace, id, &s.scratch_hold, &self.retention) {
                    return Err(InvocationError::Faulted);
                }
                if c.source.as_bytes() != q.source
                    || c.serialization != q.serialization
                    || c.insertion_policy != q.insertion_policy
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
            provisional = Some(
                self.retention
                    .reserve(l.max_capture_bytes)
                    .map_err(parent)?,
            );
            let text =
                std::str::from_utf8(q.source).map_err(|_| InvocationError::MalformedIntent)?;
            self.preflight_scratch_document(q, r).map_err(|e| {
                if faulted(&e) {
                    InvocationError::Faulted
                } else if matches!(
                    e,
                    CoordinatorError::Limit | CoordinatorError::Source(SourceError::SourceLimit)
                ) {
                    InvocationError::Limit
                } else {
                    InvocationError::Unavailable
                }
            })?;
            let context = copy_context(a.facts.context).map_err(|_| InvocationError::Capacity)?;
            let mut source = String::new();
            source
                .try_reserve_exact(text.len())
                .map_err(|_| InvocationError::Capacity)?;
            source.push_str(text);
            let marker = Arc::new(());
            let c = Arc::new(Capture {
                owner: self.namespace.clone(),
                context,
                invocation: id.clone(),
                source,
                serialization: q.serialization,
                insertion_policy: q.insertion_policy,
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
                .expect("issued scratch")
                .scratch_hold = Some(Hold {
                reservation: provisional.take().expect("funded scratch"),
                marker,
                candidate: None,
            });
            self.install_invocation_admission(
                id,
                BoundIntent::Scratch(c.clone()),
                Phase::ScratchPending(c),
            );
            Ok(InvocationState::Pending)
        })();
        if let Some(mut p) = provisional
            && self.retention.release(&mut p).is_err()
        {
            result = Err(InvocationError::Faulted)
        }
        if result == Err(InvocationError::Faulted) {
            self.faulted = true
        }
        capture_timed(
            r,
            Boundary::ScratchInvocationAdmission,
            &result,
            timing.finish(),
        );
        result
    }
    /// Execute only this original intent under fresh premises and native birth guards.
    /// An interrupted publisher remains Unknown and cannot be safely replayed.
    pub fn run_scratch_v1(
        &mut self,
        id: &InvocationId,
        f: &TrustedScratchFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::Scratch(c)) = &s.intent else {
                return Err(InvocationError::Unavailable);
            };
            if c.context != *f.context {
                return Err(InvocationError::Unavailable);
            }
            if !c.intact(&self.namespace, id, &s.scratch_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            match &s.phase {
                Phase::Terminal(_) => return Ok(InvocationState::Finished),
                Phase::ScratchPending(p) if Arc::ptr_eq(c, p) => {}
                _ => return Err(InvocationError::Faulted),
            }
            let c = c.clone();
            if self.service_requested_stop(id, r)? {
                return Ok(InvocationState::Finished);
            }
            if !allowed(f) || self.limits != c.profile {
                return Err(InvocationError::Unavailable);
            }
            self.invocations.slots[id.slot]
                .as_mut()
                .expect("pending scratch")
                .phase = Phase::Running;
            let outcome =
                match self.create_scratch_document_for_invocation(&c.request(), Some(id), r) {
                    Ok(v) => {
                        if self
                            .invocation_slot(id)?
                            .scratch_hold
                            .as_ref()
                            .and_then(|h| h.candidate.as_ref())
                            != Some(&v)
                        {
                            return Err(InvocationError::Faulted);
                        }
                        InvocationOutcome::ScratchCommitted(v)
                    }
                    Err(e) if faulted(&e) => return Err(InvocationError::Faulted),
                    Err(e) => InvocationOutcome::ScratchRejected(e),
                };
            self.install_invocation_terminal(id, outcome);
            Ok(InvocationState::Finished)
        }));
        let result = match caught {
            Ok(v) => v,
            Err(p) => {
                self.faulted = true;
                capture_timed(
                    r,
                    Boundary::ScratchInvocationService,
                    &Err::<InvocationState, _>(InvocationError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(p)
            }
        };
        if result == Err(InvocationError::Faulted) {
            self.faulted = true
        }
        capture_timed(
            r,
            Boundary::ScratchInvocationService,
            &result,
            timing.finish(),
        );
        result
    }
    pub(in crate::coordinator) fn stage_scratch_candidate(
        &mut self,
        id: &InvocationId,
        v: &DocumentVersion,
        d: &Document,
    ) -> Result<(), InvocationError> {
        let s = self.invocation_slot(id)?;
        let Some(BoundIntent::Scratch(c)) = &s.intent else {
            return Err(InvocationError::Faulted);
        };
        if !matches!(s.phase, Phase::Running)
            || !c.intact(&self.namespace, id, &s.scratch_hold, &self.retention)
            || s.scratch_hold
                .as_ref()
                .is_none_or(|h| h.candidate.is_some())
            || !Arc::ptr_eq(&v.document.namespace, &self.namespace)
            || v.document.slot != self.documents.len()
            || v.revision != 1
            || d.revision != 1
            || d.source.text() != c.source
            || d.source.initial_bom()
            || d.serialization
                != (SourceSerializationStateV1 {
                    generation: 1,
                    serialization: c.serialization,
                })
            || d.insertion_policy != c.insertion_policy
            || d.annotation_generation != 1
            || d.lifecycle
                != (LifecycleState {
                    phase: LifecyclePhase::Live,
                    generation: 1,
                    pair_generation: 1,
                })
            || d.read_only
            || d.annotation_read_only
            || d.policy_generation != 0
            || d.annotation_policy != 0
            || d.history != history::Frontiers::default()
            || !d.retained_bindings.is_empty()
        {
            return Err(InvocationError::Faulted);
        }
        self.invocations.slots[id.slot]
            .as_mut()
            .expect("running scratch")
            .scratch_hold
            .as_mut()
            .expect("funded scratch")
            .candidate = Some(v.clone());
        Ok(())
    }
    /// Release the exact original allocation under captured and current rights.
    /// Historical identity is not proof of present document liveness/readability.
    pub fn scratch_result_v1(
        &self,
        id: &InvocationId,
        f: &TrustedScratchResultV1<'_>,
        r: &mut Recorder,
    ) -> Result<ScratchResultV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::Scratch(c)) = &s.intent else {
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
                    f.result_readable,
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
            if f.max_result_bytes < size_of::<ScratchResultV1>() {
                return Err(InvocationError::Limit);
            }
            if !c.intact(&self.namespace, id, &s.scratch_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            let Phase::Terminal(Some(InvocationOutcome::ScratchCommitted(v))) = &s.phase else {
                return Err(InvocationError::OutcomeUnavailable);
            };
            if s.scratch_hold.as_ref().and_then(|h| h.candidate.as_ref()) != Some(v)
                || v.revision != 1
                || !Arc::ptr_eq(&v.document.namespace, &self.namespace)
            {
                return Err(InvocationError::Faulted);
            }
            Ok(ScratchResultV1 {
                invocation: id.clone(),
                document: v.document.clone(),
            })
        })();
        capture_timed(r, Boundary::ScratchResultRelease, &result, timing.finish());
        result
    }
    /// Discharge only this original capture hold after protected release/drain.
    /// Does not dispose the document, settle Unknown, or rearm the spent ticket.
    pub fn retire_scratch_v1(
        &mut self,
        id: &InvocationId,
        f: &InvocationObservationPolicyControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::Scratch(c)) = &s.intent else {
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
            if !c.intact(&self.namespace, id, &s.scratch_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            if !(matches!(s.phase, Phase::Terminal(_))
                || self.faulted && matches!(s.phase, Phase::Running))
            {
                return Err(InvocationError::Pending);
            }
            let s = self.invocations.slots[id.slot]
                .as_mut()
                .expect("controlled scratch");
            self.retention
                .release(&mut s.scratch_hold.as_mut().expect("funded scratch").reservation)
                .map_err(parent)?;
            s.scratch_hold = None;
            s.intent = Some(BoundIntent::RetiredScratch);
            Ok(())
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true
        }
        capture_timed(
            r,
            Boundary::ScratchInvocationRetirement,
            &result,
            timing.finish(),
        );
        result
    }
}
