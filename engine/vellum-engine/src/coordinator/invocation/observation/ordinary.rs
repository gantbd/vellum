//! Fixed-size filtered local outcomes and bounded ordinary progress identities.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationObservationQueryV1 {
    Status,
    RetainedOutcome,
}
#[derive(Clone)]
pub struct InvocationObservationReferenceV1 {
    owner: Arc<()>,
    index: usize,
}
impl InvocationObservationReferenceV1 {
    #[cfg(test)]
    pub(in crate::coordinator::invocation::observation) fn test_reference(
        observer: &InvocationObservationContextV1,
        index: usize,
    ) -> Self {
        Self {
            owner: Arc::clone(&observer.ordinary_owner),
            index,
        }
    }
    pub(super) fn belongs_to(&self, observer: &InvocationObservationContextV1) -> bool {
        Arc::ptr_eq(&self.owner, &observer.ordinary_owner) && self.index < observer.ordinary_issued
    }
}
impl PartialEq for InvocationObservationReferenceV1 {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && Arc::ptr_eq(&self.owner, &other.owner)
    }
}
impl Eq for InvocationObservationReferenceV1 {}
impl std::fmt::Debug for InvocationObservationReferenceV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationObservationReferenceV1(<opaque>)")
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvocationObservationReplyV1 {
    pub phase: Option<InvocationObservationPhaseV1>,
    pub outcome: Option<InvocationOutcomeProjectionV1>,
    pub reference: InvocationObservationReferenceV1,
}
pub struct TrustedInvocationOrdinaryObservationReleaseV1<'a> {
    pub current: &'a TrustedInvocationObservationReleaseV1<'a>,
    pub ordinary_ready: Option<bool>,
    pub resources_ready: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Signature {
    phase: Option<InvocationObservationPhaseV1>,
    outcome: Option<InvocationOutcomeProjectionV1>,
}
impl SourceCoordinator {
    pub(super) fn current_invocation_observation(
        &self,
        observer: &mut InvocationObservationContextV1,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationOrdinaryObservationReleaseV1<'_>,
    ) -> Result<bool, InvocationError> {
        self.validate_invocation_observer_ownership(observer)?;
        let original = observer
            .original
            .as_ref()
            .ok_or(InvocationError::Unavailable)?;
        let f = facts.current;
        if !observer.live
            || observer.context != *f.context
            || current.recipient != observer.ceiling.recipient
            || current.epoch != observer.ceiling.epoch
        {
            return Err(InvocationError::Unavailable);
        }
        if !current.grant
            || [f.authorized, f.scope, f.continuous, f.live, f.retained].contains(&Some(false))
        {
            observer.close();
            return Err(InvocationError::Unavailable);
        }
        if [
            f.authorized,
            f.scope,
            f.continuous,
            f.live,
            f.retained,
            f.transfer_ready,
            facts.ordinary_ready,
            facts.resources_ready,
        ]
        .into_iter()
        .any(|v| v != Some(true))
        {
            return Err(InvocationError::Unavailable);
        }
        let actual = self.original_observation_capture(&observer.invocation, &observer.context)?;
        if !Arc::ptr_eq(original, actual) {
            return Err(InvocationError::Faulted);
        }

        Ok(original.policy.control && observer.ceiling.control && current.control)
    }
    pub(super) fn query_invocation_observation_inner(
        &self,
        observer: &mut InvocationObservationContextV1,
        query: InvocationObservationQueryV1,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationOrdinaryObservationReleaseV1<'_>,
    ) -> Result<InvocationObservationReplyV1, InvocationError> {
        let control = self.current_invocation_observation(observer, current, facts)?;
        let original = observer
            .original
            .as_ref()
            .ok_or(InvocationError::Unavailable)?;
        let slot = self.invocation_slot(&observer.invocation)?;
        let phase = admitted_phase(slot)?;
        let outcome = match &slot.phase {
            Phase::Terminal(Some(v)) => Some(projection::project(
                v,
                &original.policy,
                &observer.ceiling,
                current,
            )),
            _ => None,
        };
        if (query == InvocationObservationQueryV1::Status && !control)
            || (query == InvocationObservationQueryV1::RetainedOutcome && outcome.is_none())
        {
            return Err(InvocationError::Unavailable);
        }
        let signature = Signature {
            phase: control.then_some(phase),
            outcome,
        };
        let fresh = observer.ordinary_last != Some(signature);
        if fresh {
            if observer.ordinary_issued >= observer.limits.max_issued {
                return Err(InvocationError::Limit);
            }
            // Complete fixed native frame and projection bounds were checked
            // at admission. No partial issuance on missing outcome/refusal.
            observer.ordinary_issued += 1;
            observer.ordinary_last = Some(signature);
        }
        let index = observer
            .ordinary_issued
            .checked_sub(1)
            .ok_or(InvocationError::Faulted)?;
        Ok(InvocationObservationReplyV1 {
            phase: (query == InvocationObservationQueryV1::Status).then_some(phase),
            outcome,
            reference: InvocationObservationReferenceV1 {
                owner: Arc::clone(&observer.ordinary_owner),
                index,
            },
        })
    }
    /// Ordinary recipient projection of retained facts, never command replay.
    /// No outcome clone, dynamic payload traversal, new attempt or global counter.
    pub fn query_invocation_observation_v1(
        &self,
        observer: &mut InvocationObservationContextV1,
        query: InvocationObservationQueryV1,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationOrdinaryObservationReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationObservationReplyV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = self.query_invocation_observation_inner(observer, query, current, facts);
        if result == Err(InvocationError::Faulted) {
            observer.close();
        }
        capture_timed(
            r,
            Boundary::InvocationObservationQuery,
            &result,
            timing.finish(),
        );
        result
    }
}
