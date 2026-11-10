//! Explicit disposal expectations and applicable coverage, not erasure or a wire codec.
use super::*;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisposalDispositionV1 {
    Retain,
    Discard,
}
/// All original native expectations are explicit; legacy bound-at-zero is not unbound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplicitDisposalRequestV1 {
    pub source: DocumentVersion,
    pub annotation_generation: u64,
    pub lifecycle: LifecycleState,
    pub serialization_generation: u64,
    /// None is unbound; Some(0) preserves an existing privileged legacy binding.
    pub binding_generation: Option<u64>,
    pub source_disposition: DisposalDispositionV1,
    pub annotation_disposition: DisposalDispositionV1,
}
/// An exact existing-root preparation plus immutable explicit choices.
#[derive(Clone)]
pub struct PreparedExplicitDisposalV1 {
    pub(super) plan: PreparedDisposal,
    pub(super) request: ExplicitDisposalRequestV1,
}
impl std::fmt::Debug for PreparedExplicitDisposalV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreparedExplicitDisposal(<redacted>)")
    }
}
/// Truthful embedding premises supplement native disposal checks. Only the chosen
/// retain/discard branch is applicable. Neither branch promises physical erasure.
pub struct TrustedDisposalCoverageV1 {
    pub not_stopped: Option<bool>,
    pub companions_ready: Option<bool>,
    pub versions_available: Option<bool>,
    pub reply_ready: Option<bool>,
    pub lifecycle_allowed: Option<bool>,
    pub coverage_complete: Option<bool>,
    pub owner_control_allowed: Option<bool>,
    pub source_retention_ready: Option<bool>,
    pub source_discard_allowed: Option<bool>,
    pub annotation_retention_ready: Option<bool>,
    pub annotation_discard_allowed: Option<bool>,
    pub retention_complete: Option<bool>,
    pub effects_retained: Option<bool>,
    pub history_retained: Option<bool>,
    pub snapshots_retained: Option<bool>,
    pub providers_retained: Option<bool>,
    pub views_detachable: Option<bool>,
    pub memberships_detachable: Option<bool>,
    pub observer_scopes_fenced: Option<bool>,
    pub binding_coverage_complete: Option<bool>,
}
impl TrustedDisposalCoverageV1 {
    pub(in crate::coordinator) fn allows(&self, q: &ExplicitDisposalRequestV1) -> bool {
        [
            self.not_stopped,
            self.companions_ready,
            self.versions_available,
            self.reply_ready,
            self.lifecycle_allowed,
            self.coverage_complete,
            self.owner_control_allowed,
            self.retention_complete,
            self.effects_retained,
            self.history_retained,
            self.snapshots_retained,
            self.providers_retained,
            self.views_detachable,
            self.memberships_detachable,
            self.observer_scopes_fenced,
            self.binding_coverage_complete,
            match q.source_disposition {
                DisposalDispositionV1::Retain => self.source_retention_ready,
                DisposalDispositionV1::Discard => self.source_discard_allowed,
            },
            match q.annotation_disposition {
                DisposalDispositionV1::Retain => self.annotation_retention_ready,
                DisposalDispositionV1::Discard => self.annotation_discard_allowed,
            },
        ]
        .into_iter()
        .all(|v| v == Some(true))
    }
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn validate_explicit_disposal_request(
        &self,
        q: &ExplicitDisposalRequestV1,
    ) -> Result<(), CoordinatorError> {
        self.ensure_operational()?;
        let d = self.validate_version(&q.source)?;
        if d.annotation_generation != q.annotation_generation
            || d.lifecycle != q.lifecycle
            || d.serialization.generation != q.serialization_generation
            || self
                .explicit_disposal_source_binding_generation(&q.source.document)
                .map_err(CoordinatorError::Resource)?
                != q.binding_generation
        {
            return Err(CoordinatorError::Stale);
        }
        Ok(())
    }
    /// Pure native preparation, not authority, installation or a new identity.
    pub fn prepare_explicit_disposal_v1(
        &self,
        q: &ExplicitDisposalRequestV1,
        r: &mut Recorder,
    ) -> Result<PreparedExplicitDisposalV1, CoordinatorError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.validate_explicit_disposal_request(q)?;
            let plan = self.prepare_retained_disposal(&q.source, q.annotation_generation, r)?;
            Ok(PreparedExplicitDisposalV1 {
                plan,
                request: q.clone(),
            })
        })();
        observe_service(
            r,
            Boundary::ExplicitDisposalPreparation,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn submit_explicit_disposal_v1(
        &mut self,
        id: &InvocationId,
        p: &PreparedExplicitDisposalV1,
        a: &RetainedDisposalAdmissionV1<'_>,
        coverage: &TrustedDisposalCoverageV1,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_retained_disposal_impl(
            id,
            DisposalInput::Prepared {
                plan: &p.plan,
                explicit: Some(&p.request),
            },
            a,
            Some(coverage),
            r,
        )
    }
    /// Data-first native admission. Matching replay uses only the original sealed
    /// preparation, even after disposal/forgetting; it never reparses a later root.
    /// Fresh preparation is authorized and parent-funded before metadata access.
    pub fn submit_explicit_disposal_request_v1(
        &mut self,
        id: &InvocationId,
        q: &ExplicitDisposalRequestV1,
        a: &RetainedDisposalAdmissionV1<'_>,
        coverage: &TrustedDisposalCoverageV1,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_retained_disposal_impl(id, DisposalInput::Request(q), a, Some(coverage), r)
    }
    pub fn run_explicit_disposal_v1(
        &mut self,
        id: &InvocationId,
        f: &TrustedRetainedDisposalFactsV1<'_>,
        coverage: &TrustedDisposalCoverageV1,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.run_retained_disposal_impl(id, f, Some(coverage), r)
    }
}
