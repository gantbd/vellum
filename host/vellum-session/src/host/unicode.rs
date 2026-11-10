//! Original native Unicode owners/commands; provider execution stays above host.
use super::*;
use vellum_engine::coordinator::*;
impl<A: ResourceAdapter> LocalHost<A> {
    pub fn configure_boundaries(&mut self, limits: BoundaryLimits) -> Result<bool, SessionError> {
        self.owner
            .configure_boundaries(limits, &mut self.diagnostics)
            .map_err(SessionError::Boundary)
    }
    pub fn configure_unicode_commands(
        &mut self,
        budget: UnicodeCommandBudget,
    ) -> Result<bool, SessionError> {
        self.owner
            .configure_unicode_commands(budget, &mut self.diagnostics)
            .map_err(SessionError::Unicode)
    }
    pub fn begin_boundary_capture(
        &mut self,
        version: &DocumentVersion,
    ) -> Result<BoundaryCaptureId, SessionError> {
        self.owner
            .begin_boundary_capture(version, &mut self.diagnostics)
            .map_err(SessionError::Boundary)
    }
    /// Trusted-host disclosure, not recipient authorization or a provider grant.
    pub fn boundary_source(
        &mut self,
        capture: &BoundaryCaptureId,
    ) -> Result<BoundarySourceRead<'_>, SessionError> {
        self.owner
            .boundary_source(capture, &mut self.diagnostics)
            .map_err(SessionError::Boundary)
    }
    pub fn accept_boundary_evidence(
        &mut self,
        capture: &BoundaryCaptureId,
        table: &BoundaryTableV1,
    ) -> Result<BoundaryEvidenceId, SessionError> {
        self.owner
            .accept_boundary_evidence(capture, table, &mut self.diagnostics)
            .map_err(SessionError::Boundary)
    }
    pub fn retire_boundary_capture(
        &mut self,
        capture: &BoundaryCaptureId,
    ) -> Result<(), SessionError> {
        self.owner
            .retire_boundary_capture(capture, &mut self.diagnostics)
            .map_err(SessionError::Boundary)
    }
    pub fn retire_boundary_evidence(
        &mut self,
        evidence: &BoundaryEvidenceId,
    ) -> Result<(), SessionError> {
        self.owner
            .retire_boundary_evidence(evidence, &mut self.diagnostics)
            .map_err(SessionError::Boundary)
    }
    pub fn boundary_usage(&self) -> BoundaryUsage {
        self.owner.boundary_usage()
    }
    pub fn move_unicode(
        &mut self,
        context: &LogicalViewContext,
        evidence: &BoundaryEvidenceId,
        request: UnicodeCommandV1,
    ) -> Result<LogicalViewUpdate, SessionError> {
        let plan = self
            .owner
            .prepare_unicode_movement(context, evidence, request, &mut self.diagnostics)
            .map_err(SessionError::Unicode)?;
        self.owner
            .commit_unicode_movement(plan, &mut self.diagnostics)
            .map_err(SessionError::Unicode)
    }
    pub fn delete_unicode(
        &mut self,
        context: &LogicalViewContext,
        evidence: &BoundaryEvidenceId,
        request: UnicodeCommandV1,
    ) -> Result<CommitOutcome, SessionError> {
        let plan = self
            .owner
            .prepare_unicode_deletion(context, evidence, request, &mut self.diagnostics)
            .map_err(SessionError::Unicode)?;
        Ok(self.owner.commit(plan, &mut self.diagnostics)?)
    }
}
