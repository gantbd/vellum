//! Internal trusted-owner cleanup, not a public authoring command or erasure claim.
use super::*;
#[cfg(test)]
mod tests;

/// Current host attestations, not self-issued authority. Native absence, exact
/// floors, history and captured-record obligations are checked independently.
#[derive(Clone, Copy)]
pub struct TrustedAnnotationRetirementV1 {
    pub authorized: Option<bool>,
    pub membership_current: Option<bool>,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
    pub session_references_released: Option<bool>,
    pub authored_disposition_ready: Option<bool>,
    pub history_released: Option<bool>,
    pub versions_available: Option<bool>,
    pub resources_ready: Option<bool>,
    pub bookkeeping_ready: Option<bool>,
}

impl SourceCoordinator {
    /// Final, absorbing payload release for an exact retained-absent record.
    /// No public cleanup verb, history eviction, capacity refund of lifetime IDs,
    /// source mutation, set revision or external erasure is implied.
    pub fn retire_absent_annotation(
        &mut self,
        expected: &AnnotationFact,
        facts: &TrustedAnnotationRetirementV1,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let out = (|| {
            self.ensure_operational()?;
            if [
                facts.authorized,
                facts.membership_current,
                facts.retention_released,
                facts.delivery_drained,
                facts.session_references_released,
                facts.authored_disposition_ready,
                facts.history_released,
                facts.versions_available,
                facts.resources_ready,
                facts.bookkeeping_ready,
            ]
            .into_iter()
            .any(|f| f != Some(true))
            {
                return Err(CoordinatorError::Unavailable);
            }
            if self.annotations.retired.len() != self.annotations.records.len()
                || self.annotations.issued_documents.len() != self.annotations.records.len()
            {
                return Err(CoordinatorError::Faulted);
            }
            let id = &expected.annotation;
            let record = self.annotation_record(id)?;
            if self.annotations.retired[id.slot].is_some()
                || self.annotations.issued_documents[id.slot] != id.document
            {
                return Err(CoordinatorError::Faulted);
            }
            if record.value.present {
                return Err(CoordinatorError::Annotation(AnnotationError::Retained));
            }
            if record.value.fact() != *expected {
                return Err(CoordinatorError::Stale);
            }
            if self.history_retains_annotation(id)
                || self
                    .annotations
                    .snapshots
                    .iter()
                    .filter_map(|s| s.upgrade())
                    .any(|s| s.records.iter().any(|r| r.annotation == *id))
            {
                return Err(CoordinatorError::Annotation(AnnotationError::Retained));
            }
            if !record.history.released() {
                // No retained group accounts for these frontier obligations.
                return Err(CoordinatorError::Faulted);
            }
            let generation = self.next_retention_publication_generation(None)?;
            // Both fixed slot arrays were funded at original issuance. Moving
            // the restoration key retains identity without allocating/copying it.
            let record = self.annotations.records[id.slot]
                .take()
                .expect("validated absent record");
            self.annotations.retired[id.slot] = Some(RetiredAnnotation {
                fact: record.value.fact(),
                restored_identity: record.value.restored_identity,
                document_reclaimed: false,
            });
            self.generation = generation;
            Ok(())
        })();
        if matches!(out, Err(CoordinatorError::Faulted)) {
            self.faulted = true;
        }
        observe(r, Boundary::AnnotationRetirement, &out);
        out
    }
}
