//! Remaining native context incidences. This is not a reclamation grant.
use super::*;
#[cfg(test)]
mod tests;

/// Trusted-owner diagnostic, not unique allocations, bytes or a caller-supplied
/// discharge witness. Closed/stale retained records continue to count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DocumentContextPinsV1 {
    pub attached_views: usize,
    pub workspace_memberships: usize,
    pub boundary_captures: usize,
    pub boundary_evidence: usize,
    pub annotation_snapshots: usize,
    pub serialization_bases: usize,
    pub retained_invocations: usize,
    /// Publisher-owned incidences, including original commands and receipt contexts;
    /// not the number of currently delivering streams or unique allocations.
    pub publication_streams: usize,
}
impl DocumentContextPinsV1 {
    pub fn total(self) -> Result<usize, CoordinatorError> {
        [
            self.attached_views,
            self.workspace_memberships,
            self.boundary_captures,
            self.boundary_evidence,
            self.annotation_snapshots,
            self.serialization_bases,
            self.retained_invocations,
            self.publication_streams,
        ]
        .into_iter()
        .try_fold(0usize, |n, v| {
            n.checked_add(v).ok_or(CoordinatorError::Faulted)
        })
    }
}
impl SourceCoordinator {
    /// No hold changes, payload copies, owner retirement or document removal.
    /// A zero subtotal excludes resource and read/history owners.
    pub fn document_context_pins_v1(
        &self,
        document: &DocumentId,
        r: &mut Recorder,
    ) -> Result<DocumentContextPinsV1, CoordinatorError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational()?;
            self.retained_document(document)?;
            let (boundary_captures, boundary_evidence) = self.boundaries.document_pins(document);
            let pins = DocumentContextPinsV1 {
                attached_views: self.views.document_pins(document),
                workspace_memberships: self.workspaces.document_pins(document)?,
                boundary_captures,
                boundary_evidence,
                annotation_snapshots: self.annotations.document_snapshot_pins(document),
                serialization_bases: self.serialization_bases.document_pins(document),
                retained_invocations: self.invocations.document_pins(document)?,
                publication_streams: self
                    .publication_streams
                    .document_pins(document, &self.retention)?
                    .checked_add(
                        self.facet_publication_streams
                            .document_pins(document, &self.retention)?,
                    )
                    .ok_or(CoordinatorError::Faulted)?
                    .checked_add(
                        self.collection_publication_streams
                            .document_pins(document, &self.retention)?,
                    )
                    .ok_or(CoordinatorError::Faulted)?,
            };
            pins.total()?;
            Ok(pins)
        })();
        observe_service(r, Boundary::DocumentContextPins, &result, timing.finish());
        result
    }
}
