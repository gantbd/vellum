//! Fixed-slot whole-document payload retirement; identity/floors remain spent.
use super::*;
impl AnnotationState {
    #[cfg(test)]
    pub(in crate::coordinator) fn reclaimed_fact_for_test(
        &self,
        a: &AnnotationId,
    ) -> Option<&AnnotationFact> {
        self.retired
            .get(a.slot)?
            .as_ref()
            .filter(|r| r.document_reclaimed && r.fact.annotation == *a)
            .map(|r| &r.fact)
    }
    pub(in crate::coordinator) fn validate_document_reclamation(
        &self,
        d: &DocumentId,
    ) -> Result<(), CoordinatorError> {
        if self.records.len() != self.retired.len()
            || self.records.len() != self.issued_documents.len()
        {
            return Err(CoordinatorError::Faulted);
        }
        for (slot, record) in self.records.iter().enumerate() {
            if let Some(record) = record {
                let id = &record.value.id;
                if id.slot != slot
                    || self.issued_documents[slot] != id.document
                    || self.retired[slot].is_some()
                {
                    return Err(CoordinatorError::Faulted);
                }
                if id.document == *d && !record.history.released() {
                    return Err(CoordinatorError::Faulted);
                }
            }
        }
        Ok(())
    }
    /// No allocation/callback. Preserve last fact and restoration identity by move.
    pub(in crate::coordinator) fn reclaim_document_payload(&mut self, d: &DocumentId) {
        for (slot, row) in self.records.iter_mut().enumerate() {
            if row.as_ref().is_some_and(|r| r.value.id.document == *d) {
                let record = row.take().expect("validated retained annotation");
                self.retired[slot] = Some(RetiredAnnotation {
                    fact: record.value.fact(),
                    restored_identity: record.value.restored_identity,
                    document_reclaimed: true,
                });
            }
        }
    }
}
