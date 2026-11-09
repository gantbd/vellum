//! Integration ownership of original native invocation lifetimes. No provider,
//! task, receipt, effect disposition or new semantic-engine identity lives here.
use super::*;
use vellum_engine::coordinator::{InvocationId, InvocationState, SourceIntent};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationClaimKindV1 {
    /// Cannot submit/run source work or synthesize a source terminal outcome.
    SourceNeutral,
    SourcePublication,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationClaimErrorV1 {
    Owned,
    Unavailable,
    WrongKind,
    Capacity,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvocationClaimUsageV1 {
    pub retained: usize,
    pub limit: usize,
    pub accepted: u64,
    /// Ownership/kind/claim-admission refusals, not underlying native errors.
    pub refused: u64,
    pub retired: u64,
}
/// Non-cloneable claim minted only for this host's pristine native invocation.
/// Drop does not release ownership. Identity alone cannot recreate this claim.
/// This is trusted integration ownership, NOT recipient or provider authority.
pub struct InvocationClaimV1 {
    origin: InvocationId,
    kind: InvocationClaimKindV1,
}
impl std::fmt::Debug for InvocationClaimV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationClaimV1(<redacted>)")
    }
}
impl InvocationClaimV1 {
    pub fn origin(&self) -> &InvocationId {
        &self.origin
    }
    pub fn kind(&self) -> InvocationClaimKindV1 {
        self.kind
    }
}
pub(super) struct Claims {
    entries: Vec<(InvocationId, InvocationClaimKindV1)>,
    usage: InvocationClaimUsageV1,
}
impl Claims {
    pub(super) fn new(limit: usize) -> Self {
        Self {
            entries: Vec::new(),
            usage: InvocationClaimUsageV1 {
                retained: 0,
                limit,
                accepted: 0,
                refused: 0,
                retired: 0,
            },
        }
    }
    pub(super) fn retained(&self) -> usize {
        self.entries.len()
    }
    pub(super) fn remove_retired(&mut self, origin: &InvocationId) {
        self.entries.retain(|(id, _)| id != origin);
        self.usage.retained = self.entries.len();
        self.usage.retired = self.usage.retired.saturating_add(1);
    }
    fn contains(&self, id: &InvocationId) -> bool {
        self.entries.iter().any(|(origin, _)| origin == id)
    }
    fn refuse(&mut self, error: InvocationClaimErrorV1) -> SessionError {
        self.usage.refused = self.usage.refused.saturating_add(1);
        SessionError::InvocationClaim(error)
    }
    fn validate(&mut self, claim: &InvocationClaimV1) -> Result<(), SessionError> {
        if !self
            .entries
            .iter()
            .any(|(id, kind)| id == &claim.origin && kind == &claim.kind)
        {
            return Err(self.refuse(InvocationClaimErrorV1::Unavailable));
        }
        Ok(())
    }
    fn publication(&mut self, claim: &InvocationClaimV1) -> Result<(), SessionError> {
        self.validate(claim)?;
        if claim.kind != InvocationClaimKindV1::SourcePublication {
            return Err(self.refuse(InvocationClaimErrorV1::WrongKind));
        }
        Ok(())
    }
}
impl<A: ResourceAdapter> LocalHost<A> {
    pub(super) fn require_unclaimed_invocation(
        &mut self,
        id: &InvocationId,
    ) -> Result<(), SessionError> {
        if self.invocation_claims.contains(id) {
            return Err(self.invocation_claims.refuse(InvocationClaimErrorV1::Owned));
        }
        Ok(())
    }
    /// Claim only an issued, unsubmitted native lifetime. Claim capacity is the
    /// configured native retained-invocation bound, not a new product budget.
    /// No release-to-Issued operation exists: discharge retires the original ID.
    pub fn claim_invocation(
        &mut self,
        origin: &InvocationId,
        kind: InvocationClaimKindV1,
    ) -> Result<InvocationClaimV1, SessionError> {
        self.require_unclaimed_invocation(origin)?;
        let state = self.owner.invocation_state(origin, &mut self.diagnostics);
        if !matches!(state, Ok(InvocationState::Issued)) {
            return Err(self
                .invocation_claims
                .refuse(InvocationClaimErrorV1::Unavailable));
        }
        if self.invocation_claims.entries.len() >= self.invocation_claims.usage.limit
            || self.invocation_claims.entries.try_reserve_exact(1).is_err()
        {
            return Err(self
                .invocation_claims
                .refuse(InvocationClaimErrorV1::Capacity));
        }
        self.invocation_claims.entries.push((origin.clone(), kind));
        self.invocation_claims.usage.retained = self.invocation_claims.entries.len();
        self.invocation_claims.usage.accepted =
            self.invocation_claims.usage.accepted.saturating_add(1);
        Ok(InvocationClaimV1 {
            origin: origin.clone(),
            kind,
        })
    }
    pub fn submit_claimed_source_invocation(
        &mut self,
        claim: &InvocationClaimV1,
        intent: &SourceIntent,
    ) -> Result<InvocationState, SessionError> {
        self.invocation_claims.publication(claim)?;
        Ok(self
            .owner
            .submit(&claim.origin, intent, &mut self.diagnostics)?)
    }
    /// The native publisher installs source/history and its terminal outcome.
    /// Integrations must query that outcome, not maintain a shadow committed bit.
    pub fn run_claimed_invocation(
        &mut self,
        claim: &InvocationClaimV1,
    ) -> Result<InvocationState, SessionError> {
        self.invocation_claims.publication(claim)?;
        Ok(self.owner.run(&claim.origin, &mut self.diagnostics)?)
    }
    pub fn stop_claimed_invocation(
        &mut self,
        claim: &InvocationClaimV1,
    ) -> Result<(), SessionError> {
        self.invocation_claims.publication(claim)?;
        Ok(self
            .owner
            .stop_invocation(&claim.origin, &mut self.diagnostics)?)
    }
    pub fn forget_claimed_invocation_outcome(
        &mut self,
        claim: &InvocationClaimV1,
    ) -> Result<(), SessionError> {
        self.invocation_claims.publication(claim)?;
        Ok(self
            .owner
            .forget_invocation_outcome(&claim.origin, &mut self.diagnostics)?)
    }
    /// The trusted integration must first discharge any external obligations.
    /// This method does not establish provider acknowledgment or quiescence.
    /// Native retirement refuses pending/faulted/pinned work; refusal retains
    /// the claim. Success burns the original lifetime rather than making reuse possible.
    pub fn retire_claimed_invocation(
        &mut self,
        claim: &InvocationClaimV1,
    ) -> Result<(), SessionError> {
        self.invocation_claims.validate(claim)?;
        self.owner
            .retire_invocation(&claim.origin, &mut self.diagnostics)?;
        self.invocation_claims.remove_retired(&claim.origin);
        Ok(())
    }
    /// Content-free integration gauges/counters; no document, path or task IDs.
    /// Retained counts are logical claims, not allocator/RSS measurements.
    pub fn invocation_claim_usage(&self) -> InvocationClaimUsageV1 {
        self.invocation_claims.usage
    }
}
