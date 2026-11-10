//! Trusted native original-invocation seam. No wrapper request registry, inferred
//! recipient authority, automatic retry, external-effect ledger or clipboard task.
use super::*;
use vellum_engine::coordinator::{
    InvocationId, InvocationOutcome, InvocationState, InvocationUsage, SourceIntent,
};

impl<A: ResourceAdapter> LocalHost<A> {
    /// Caller must explicitly configure CoordinatorConfig::invocation. This
    /// returns the actual engine-issued identity, never a string correlation ID.
    pub fn issue_invocation(&mut self) -> Result<InvocationId, SessionError> {
        Ok(self.owner.issue_invocation(&mut self.diagnostics)?)
    }
    /// Matching replay keeps this original identity and exact semantic intent.
    /// A different attempt is a new explicitly issued operation, not a retry.
    pub fn submit_source_invocation(
        &mut self,
        id: &InvocationId,
        intent: &SourceIntent,
    ) -> Result<InvocationState, SessionError> {
        self.require_unclaimed_invocation(id)?;
        Ok(self.owner.submit(id, intent, &mut self.diagnostics)?)
    }
    /// Execute through the same native source/annotation/view/history publisher.
    /// Terminal calls only observe; this never dispatches an external effect.
    pub fn run_invocation(&mut self, id: &InvocationId) -> Result<InvocationState, SessionError> {
        self.require_unclaimed_invocation(id)?;
        Ok(self.owner.run(id, &mut self.diagnostics)?)
    }
    pub fn invocation_state(&mut self, id: &InvocationId) -> Result<InvocationState, SessionError> {
        Ok(self.owner.invocation_state(id, &mut self.diagnostics)?)
    }
    /// Immediate trusted-host disclosure, not a recipient grant. This outcome
    /// describes the original operation then, not the document's state now.
    pub fn invocation_outcome(
        &mut self,
        id: &InvocationId,
    ) -> Result<&InvocationOutcome, SessionError> {
        Ok(self.owner.invocation_outcome(id, &mut self.diagnostics)?)
    }
    pub fn stop_invocation(&mut self, id: &InvocationId) -> Result<(), SessionError> {
        self.require_unclaimed_invocation(id)?;
        Ok(self.owner.stop_invocation(id, &mut self.diagnostics)?)
    }
    /// Forgetting disclosure neither retires identity nor makes replay execute.
    pub fn forget_invocation_outcome(&mut self, id: &InvocationId) -> Result<(), SessionError> {
        self.require_unclaimed_invocation(id)?;
        Ok(self
            .owner
            .forget_invocation_outcome(id, &mut self.diagnostics)?)
    }
    /// Original lifetime/retention guards remain native. This does not release
    /// history, source bindings, provider work or an integration-owned task.
    pub fn retire_invocation(&mut self, id: &InvocationId) -> Result<(), SessionError> {
        self.require_unclaimed_invocation(id)?;
        Ok(self.owner.retire_invocation(id, &mut self.diagnostics)?)
    }
    pub fn invocation_usage(&self) -> InvocationUsage {
        self.owner.invocation_usage()
    }
}
