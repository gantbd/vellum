//! Trusted native annotation access. No UI policy, codec or file ownership here.
use super::*;
use vellum_engine::coordinator::{
    AnnotationFact, AnnotationId, AnnotationMutation, AnnotationRead, AnnotationSnapshot,
    AnnotationUsage, TrustedAnnotationRetirementV1,
};

/// Original creation identities in preparation order, disclosed only after the
/// actual compound publication succeeds. No wrapper identity registry is added.
pub struct AnnotationCommit {
    pub outcome: CommitOutcome,
    pub created: Vec<AnnotationId>,
}
impl std::fmt::Debug for AnnotationCommit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationCommit(<redacted>)")
    }
}
impl<A: ResourceAdapter> LocalHost<A> {
    pub fn annotation_document_summary(
        &mut self,
        document: &LocalDocument<A::Handle>,
    ) -> Result<vellum_engine::coordinator::AnnotationDocumentSummaryV1, SessionError> {
        Ok(self
            .owner
            .annotation_document_summary_v1(&document.loaded.document, &mut self.diagnostics)?)
    }
    pub fn annotation_document_facts(
        &mut self,
        expected: &DocumentVersion,
        expected_set: u64,
    ) -> Result<Vec<AnnotationFact>, SessionError> {
        Ok(self.owner.annotation_document_facts_v1(
            expected,
            expected_set,
            &mut self.diagnostics,
        )?)
    }
    /// One native publication for exact source writes, declared reads and
    /// annotation mutations. Bodies are inert text; target profiles and separate
    /// body/target versions remain native. Annotation limits are caller-selected.
    pub fn commit_compound(
        &mut self,
        writes: &[DocumentEdits],
        reads: &[DocumentVersion],
        mutations: &[AnnotationMutation],
    ) -> Result<AnnotationCommit, SessionError> {
        let prepared =
            self.owner
                .prepare_compound(writes, reads, mutations, &mut self.diagnostics)?;
        let mut created = Vec::new();
        created
            .try_reserve_exact(prepared.created_annotations().count())
            .map_err(|_| SessionError::Capacity)?;
        created.extend(prepared.created_annotations().cloned());
        let outcome = self.owner.commit(prepared, &mut self.diagnostics)?;
        Ok(AnnotationCommit { outcome, created })
    }

    /// Annotation history has its own frontier. Reversal still publishes the
    /// entire original linked group, including source edits in other files.
    pub fn reverse_annotation(
        &mut self,
        id: &AnnotationId,
        direction: HistoryDirection,
    ) -> Result<bool, SessionError> {
        let status = self
            .owner
            .annotation_history_status(id, direction, &mut self.diagnostics)?;
        self.reverse_available(status, direction)
    }

    /// Borrowed trusted-host disclosure, not a recipient-safe result or grant.
    pub fn annotation(&mut self, id: &AnnotationId) -> Result<AnnotationRead<'_>, SessionError> {
        Ok(self.owner.annotation(id, &mut self.diagnostics)?)
    }

    pub fn annotation_set_version(
        &mut self,
        document: &LocalDocument<A::Handle>,
    ) -> Result<u64, SessionError> {
        Ok(self
            .owner
            .annotation_set_version(&document.loaded.document, &mut self.diagnostics)?)
    }

    /// Exact caller-supplied source and set captures. A stale set refuses rather
    /// than being silently refreshed. Snapshots retain native record obligations.
    pub fn capture_annotations(
        &mut self,
        expected: &DocumentVersion,
        expected_set: u64,
    ) -> Result<AnnotationSnapshot, SessionError> {
        Ok(self
            .owner
            .capture_annotations(expected, expected_set, &mut self.diagnostics)?)
    }

    /// Separate annotation policy; source read-only does not forbid comments.
    pub fn set_annotations_locked(
        &mut self,
        document: &LocalDocument<A::Handle>,
        locked: bool,
    ) -> Result<(), SessionError> {
        Ok(self.owner.set_annotations_read_only(
            &document.loaded.document,
            locked,
            &mut self.diagnostics,
        )?)
    }

    pub fn annotation_usage(&self) -> AnnotationUsage {
        self.owner.annotation_usage()
    }

    /// Release an exact retained-absent record only after the host supplies its
    /// actual disposition facts. Native history/snapshot pins remain authoritative.
    /// This neither deletes a persisted annotation file nor refunds lifetime IDs.
    pub fn retire_annotation(
        &mut self,
        expected: &AnnotationFact,
        facts: &TrustedAnnotationRetirementV1,
    ) -> Result<(), SessionError> {
        Ok(self
            .owner
            .retire_absent_annotation(expected, facts, &mut self.diagnostics)?)
    }
}
