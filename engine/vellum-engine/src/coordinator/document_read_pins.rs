//! Retained read/search/history target incidence, not permission to reclaim.
use super::*;
#[cfg(test)]
mod tests;
/// Trusted native diagnostic. Counts are not unique allocations or bytes, and
/// do not cover every engine owner. Stopped/invalidated retained rows still count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DocumentReadPinsV1 {
    pub source_contexts: usize,
    pub source_holds: usize,
    pub annotation_preparations: usize,
    pub annotation_contexts: usize,
    pub annotation_holds: usize,
    pub version_set_preparations: usize,
    pub version_set_contexts: usize,
    pub version_set_holds: usize,
    pub managed_search_results: usize,
    pub legacy_search_sessions: usize,
    pub legacy_search_results: usize,
    pub history_groups: usize,
    pub history_pins: usize,
    pub history_inspections: usize,
}
impl DocumentReadPinsV1 {
    pub fn total(self) -> Result<usize, CoordinatorError> {
        [
            self.source_contexts,
            self.source_holds,
            self.annotation_preparations,
            self.annotation_contexts,
            self.annotation_holds,
            self.version_set_preparations,
            self.version_set_contexts,
            self.version_set_holds,
            self.managed_search_results,
            self.legacy_search_sessions,
            self.legacy_search_results,
            self.history_groups,
            self.history_pins,
            self.history_inspections,
        ]
        .into_iter()
        .try_fold(0usize, |n, v| {
            n.checked_add(v).ok_or(CoordinatorError::Faulted)
        })
    }
}
impl SourceCoordinator {
    /// Exact retained native document, not an untrusted recipient disclosure or
    /// a caller-supplied discharge witness. No admission, hold change or payload
    /// copy. A zero subtotal excludes resources, views, invocations and other owners.
    pub fn document_read_pins_v1(
        &self,
        document: &DocumentId,
        r: &mut Recorder,
    ) -> Result<DocumentReadPinsV1, CoordinatorError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational()?;
            self.retained_document(document)?;
            let (source_contexts, source_holds) = self.source_contexts.document_pins(document)?;
            let (annotation_contexts, annotation_holds) =
                self.annotation_contexts.document_pins(document)?;
            let (version_set_contexts, version_set_holds) =
                self.version_set_contexts.document_pins(document)?;
            let (legacy_search_sessions, legacy_search_results) =
                self.searches.document_pins(document);
            let (history_groups, history_pins) = self.history.document_pins(document)?;
            let pins = DocumentReadPinsV1 {
                source_contexts,
                source_holds,
                annotation_contexts,
                annotation_holds,
                version_set_contexts,
                version_set_holds,
                legacy_search_sessions,
                legacy_search_results,
                history_groups,
                history_pins,
                annotation_preparations: self.annotation_preparations.document_pins(document),
                version_set_preparations: self.version_set_preparations.document_pins(document),
                managed_search_results: self.managed_search.document_pins(document)?,
                history_inspections: self.history_inspection.document_pins(document),
            };
            pins.total()?;
            Ok(pins)
        })();
        observe_service(r, Boundary::DocumentReadPins, &result, timing.finish());
        result
    }
}
