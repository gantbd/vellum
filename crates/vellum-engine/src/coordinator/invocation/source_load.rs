//! Owned completed-read activation. No read dispatch, reload, root injection or UI.
use super::super::settings::basis::{copy_context, labels};
use super::*;
use crate::resource::{ResourceError, SourceReadIdV1};
use std::mem::size_of;
/// In-process declaration only, not a serialized authority or wire grammar.
pub const fn native_source_load_argument_declaration_v1() -> NativeArgumentDeclarationV1 {
    NativeArgumentDeclarationV1 {
        contract: "engine-source-load-declaration/1",
        variant: NativeHandlerVariantDeclarationV1 {
            operation: "activate-loaded-candidate",
            fields: &[],
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceLoadLimitsV1 {
    pub max_capture_bytes: usize,
    pub max_result_bytes: usize,
}
impl SourceLoadLimitsV1 {
    /// Fixed retained native capsule/receipt plus copied context labels. No
    /// retained second source buffer; native preparation uses existing limits.
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
/// Independent trusted-host premises, not assertions accepted from scripts.
/// Creation/owner access are conditional on actual current native resolution.
pub struct TrustedSourceLoadFactsV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub interpretation_current: Option<bool>,
    pub identity_allowed: Option<bool>,
    pub binding_coverage_complete: Option<bool>,
    pub activation_allowed: Option<bool>,
    pub companions_ready: Option<bool>,
    pub versions_available: Option<bool>,
    pub reply_ready: Option<bool>,
    pub resources_ready: Option<bool>,
    pub owner_control_allowed: Option<bool>,
    pub preparation_reserved: Option<bool>,
    pub writer_coverage_complete: Option<bool>,
    pub disposition_readable: Option<bool>,
    pub creation_allowed: Option<bool>,
    pub owner_access_allowed: Option<bool>,
}
impl TrustedSourceLoadFactsV1<'_> {
    fn allowed(&self) -> bool {
        [
            self.authorized,
            self.scope,
            self.continuous,
            self.live,
            self.not_stopped,
            self.interpretation_current,
            self.identity_allowed,
            self.binding_coverage_complete,
            self.activation_allowed,
            self.companions_ready,
            self.versions_available,
            self.reply_ready,
            self.resources_ready,
            self.owner_control_allowed,
            self.preparation_reserved,
            self.writer_coverage_complete,
            self.disposition_readable,
        ]
        .into_iter()
        .all(|f| f == Some(true))
    }
    fn authority(&self) -> SourceLoadAuthorityFactsV1 {
        SourceLoadAuthorityFactsV1 {
            owner_control_allowed: self.owner_control_allowed,
            preparation_reserved: self.preparation_reserved,
            resources_ready: self.resources_ready,
        }
    }
}
pub struct SourceLoadAdmissionV1<'a> {
    pub facts: &'a TrustedSourceLoadFactsV1<'a>,
    pub limits: SourceLoadLimitsV1,
    pub publication_epoch: u64,
    pub publication_allowed: Option<bool>,
    pub result_admission_allowed: Option<bool>,
    pub result_allowed: Option<bool>,
}
pub struct TrustedSourceLoadResultV1<'a> {
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
pub struct SourceLoadResultV1 {
    pub invocation: InvocationId,
    pub document: DocumentId,
    pub disposition: SourceReadActivationDispositionV1,
}
pub(super) struct Capture {
    pub(super) owner: Arc<()>,
    pub(super) context: SettingsExecutionContextV1,
    invocation: InvocationId,
    authority: SourceLoadAuthorityV1,
    profile: CoordinatorLimits,
    limits: SourceLoadLimitsV1,
    epoch: u64,
    publication: bool,
    admission: bool,
    result: bool,
    marker: Arc<()>,
}
pub(super) struct Hold {
    reservation: retention::Reservation,
    marker: Arc<()>,
    pub(super) candidate: Option<SourceReadActivationV1>,
}
impl std::fmt::Debug for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceLoad(<redacted>)")
    }
}
impl std::fmt::Debug for Hold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceLoadHold(<owned>)")
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
            && SourceLoadLimitsV1::required_capture_bytes(&self.context)
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
    fn prepare_authorized_source_load(
        &mut self,
        authority: &SourceLoadAuthorityV1,
        f: &TrustedSourceLoadFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<resource::source_read::PreparedSourceReadActivation, ResourceError> {
        if !f.allowed() {
            return Err(ResourceError::Unavailable);
        }
        let task = self.source_load_task_for_invocation(authority, &f.authority(), r)?;
        self.source_load_writer_guard(authority, &task)?;
        let plan = self.prepare_source_read_activation_native(&task, r)?;
        let allowed = if plan.outcome().disposition == SourceReadActivationDispositionV1::Created {
            f.creation_allowed
        } else {
            f.owner_access_allowed
        };
        if allowed != Some(true) {
            return Err(ResourceError::Unavailable);
        }
        Ok(plan)
    }
    pub fn submit_source_load_v1(
        &mut self,
        id: &InvocationId,
        authority: &SourceLoadAuthorityV1,
        a: &SourceLoadAdmissionV1<'_>,
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
            if SourceLoadLimitsV1::required_capture_bytes(a.facts.context)? > l.max_capture_bytes
                || l.max_capture_bytes > self.invocations.limits.max_intent_bytes
                || l.max_result_bytes < size_of::<SourceLoadResultV1>()
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
                let BoundIntent::SourceLoad(c) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                if !c.intact(&self.namespace, id, &s.load_hold, &self.retention) {
                    return Err(InvocationError::Faulted);
                }
                if c.authority != *authority
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
            // Validate current resolution without promising allocation or retaining a root.
            drop(
                self.prepare_authorized_source_load(authority, a.facts, r)
                    .map_err(resource)?,
            );
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
                .load_hold = Some(Hold {
                reservation: provisional.take().expect("funded load"),
                marker,
                candidate: None,
            });
            self.install_invocation_admission(
                id,
                BoundIntent::SourceLoad(c.clone()),
                Phase::SourceLoadPending(c),
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
        capture_timed(r, Boundary::SourceLoadAdmission, &result, timing.finish());
        result
    }
    pub fn run_source_load_v1(
        &mut self,
        id: &InvocationId,
        f: &TrustedSourceLoadFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::SourceLoad(c)) = &s.intent else {
                return Err(InvocationError::Unavailable);
            };
            if c.context != *f.context || c.profile != self.limits {
                return Err(InvocationError::Unavailable);
            }
            if !c.intact(&self.namespace, id, &s.load_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            match &s.phase {
                Phase::Terminal(_) => return Ok(InvocationState::Finished),
                Phase::Running => return Err(InvocationError::Faulted),
                Phase::SourceLoadPending(p) if Arc::ptr_eq(c, p) => {}
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
                .prepare_authorized_source_load(&c.authority, f, r)
                .and_then(|p| self.commit_source_read_activation_for_invocation(p, Some(id), r));
            match result {
                Ok(_) => {} // The native installer settled the terminal under its fence.
                Err(ResourceError::Faulted) => return Err(InvocationError::Faulted),
                Err(e) => {
                    self.install_invocation_terminal(id, InvocationOutcome::SourceLoadRejected(e))
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
                    Boundary::SourceLoadService,
                    &Err::<InvocationState, _>(InvocationError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(p)
            }
        };
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(r, Boundary::SourceLoadService, &result, timing.finish());
        result
    }
    pub(in crate::coordinator) fn stage_source_load_candidate(
        &mut self,
        id: &InvocationId,
        task: &SourceReadIdV1,
        outcome: &SourceReadActivationV1,
    ) -> Result<InvocationOutcome, ResourceError> {
        let s = self
            .invocation_slot(id)
            .map_err(|_| ResourceError::Faulted)?;
        let Some(BoundIntent::SourceLoad(c)) = &s.intent else {
            return Err(ResourceError::Faulted);
        };
        if !matches!(s.phase, Phase::Running)
            || !c.authority.names_task(task)
            || !c.intact(&self.namespace, id, &s.load_hold, &self.retention)
            || s.load_hold.as_ref().is_none_or(|h| h.candidate.is_some())
        {
            return Err(ResourceError::Faulted);
        }
        // Actual shared writer, not host coverage assertions alone; repeated at final stage.
        self.source_load_writer_guard(&c.authority, task)?;
        let candidate = outcome.clone();
        let terminal = InvocationOutcome::SourceLoadCommitted(outcome.clone());
        self.invocations.slots[id.slot]
            .as_mut()
            .expect("running load")
            .load_hold
            .as_mut()
            .expect("funded load")
            .candidate = Some(candidate);
        Ok(terminal)
    }
    pub fn source_load_result_v1(
        &self,
        id: &InvocationId,
        f: &TrustedSourceLoadResultV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourceLoadResultV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::SourceLoad(c)) = &s.intent else {
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
            if !c.intact(&self.namespace, id, &s.load_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            match &s.phase {
                Phase::Terminal(Some(InvocationOutcome::SourceLoadCommitted(v))) => {
                    if s.load_hold.as_ref().and_then(|h| h.candidate.as_ref()) != Some(v) {
                        return Err(InvocationError::Faulted);
                    }
                    Ok(SourceLoadResultV1 {
                        invocation: id.clone(),
                        document: v.document.document.clone(),
                        disposition: v.disposition,
                    })
                }
                Phase::Running => Err(InvocationError::Faulted),
                _ => Err(InvocationError::OutcomeUnavailable),
            }
        })();
        capture_timed(
            r,
            Boundary::SourceLoadResultRelease,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn retire_source_load_v1(
        &mut self,
        id: &InvocationId,
        f: &InvocationObservationPolicyControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::SourceLoad(c)) = &s.intent else {
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
            if !c.intact(&self.namespace, id, &s.load_hold, &self.retention) {
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
                .release(&mut s.load_hold.as_mut().expect("funded load").reservation)
                .map_err(parent)?;
            s.load_hold = None;
            s.intent = Some(BoundIntent::RetiredLoad);
            Ok(())
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(r, Boundary::SourceLoadRetirement, &result, timing.finish());
        result
    }
}
