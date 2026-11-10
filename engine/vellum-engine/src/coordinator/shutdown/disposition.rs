//! Destructive document progress under original consent, not whole-owner drain.
use super::*;
use crate::resource::{ArtifactKind, SaveId};

/// Original already-admitted save allocations, not new dispatch
/// authority. A discard branch must not supply a save allocation.
pub struct ShutdownDocumentDispositionRequestV1<'a> {
    pub document: &'a DocumentId,
    pub source_save: Option<&'a SaveId>,
    pub annotation_save: Option<&'a SaveId>,
}

#[cfg(test)]
mod tests;

impl SourceCoordinator {
    /// Apply this document's explicitly acknowledged discard choices under the
    /// original installed fence. Save choices are not discard authority: they
    /// require the still-separate save/readiness path and refuse here.
    ///
    /// Retained effects, history, bindings and payloads survive this operation.
    /// Success is document editing disposal, not shutdown acceptance/completion,
    /// persistence, cancellation, erasure or permission to retire any owner.
    pub fn discard_shutdown_document_v1(
        &mut self,
        inventory: &ShutdownInventoryV1,
        decisions: &ShutdownDecisionsV1,
        document: &DocumentId,
        f: &TrustedShutdownInventoryFactsV1<'_>,
        coverage: &TrustedDisposalCoverageV1,
        r: &mut Recorder,
    ) -> Result<LifecycleState, CoordinatorError> {
        self.dispose_shutdown_document_v1(
            inventory,
            decisions,
            &ShutdownDocumentDispositionRequestV1 {
                document,
                source_save: None,
                annotation_save: None,
            },
            f,
            coverage,
            r,
        )
    }
    /// Execute acknowledged independent save/discard choices. Save branches need
    /// an exact original producer of a qualified installed ordinary baseline.
    /// This does not queue writes, infer external freshness or drain effects.
    pub fn dispose_shutdown_document_v1(
        &mut self,
        inventory: &ShutdownInventoryV1,
        decisions: &ShutdownDecisionsV1,
        request: &ShutdownDocumentDispositionRequestV1<'_>,
        f: &TrustedShutdownInventoryFactsV1<'_>,
        coverage: &TrustedDisposalCoverageV1,
        r: &mut Recorder,
    ) -> Result<LifecycleState, CoordinatorError> {
        let document = request.document;
        let timer = r.begin_service_timing();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.ensure_operational()?;
            self.shutdown_inventory_roles(&inventory.basis, f)?;
            self.validate_accepted_disposition(inventory, decisions, request)?;
            if !Arc::ptr_eq(&inventory.marker, &decisions.original_inventory) {
                return Err(CoordinatorError::Unavailable);
            }
            let fence = self
                .shutdown
                .fence
                .as_ref()
                .ok_or(CoordinatorError::Unavailable)?;
            if !Arc::ptr_eq(&inventory.basis.marker, &fence.basis.marker) {
                return Err(CoordinatorError::Unavailable);
            }
            self.retention
                .validate_exact(&fence.reservation, fence.charge)
                .map_err(|_| CoordinatorError::Faulted)?;
            let fence_id = fence.id.clone();
            let choice = decisions
                .decisions
                .binary_search_by_key(&document.slot, |d| d.expected.source.document.slot)
                .ok()
                .and_then(|i| decisions.decisions.get(i))
                .filter(|d| d.expected.source.document == *document)
                .ok_or(CoordinatorError::Unavailable)?;
            let expected = &choice.expected;
            let q = ExplicitDisposalRequestV1 {
                source: expected.source.clone(),
                annotation_generation: expected.annotation_generation,
                lifecycle: expected.lifecycle,
                serialization_generation: expected.serialization.generation,
                binding_generation: expected.source_binding_generation,
                source_disposition: disposition(choice.source),
                annotation_disposition: disposition(choice.annotations),
            };
            if !coverage.allows(&q) {
                return Err(CoordinatorError::Unavailable);
            }
            // Revalidate acknowledged document expectations, not the old whole
            // inventory: protected evidence and other document disposition may
            // legitimately have progressed since fence installation.
            self.validate_explicit_disposal_request(&q)?;
            let d = self.validate_version(&expected.source)?;
            if d.policy_generation != expected.source_policy_generation
                || d.read_only != expected.source_read_only
                || d.annotation_policy != expected.annotation_policy_generation
                || d.annotation_read_only != expected.annotation_read_only
                || d.serialization != expected.serialization
                || self.shutdown_annotation_binding_generation(document)?
                    != expected.annotation_binding_generation
            {
                return Err(CoordinatorError::Stale);
            }
            for (choice, id, kind) in [
                (choice.source, request.source_save, ArtifactKind::Source),
                (
                    choice.annotations,
                    request.annotation_save,
                    ArtifactKind::Annotations,
                ),
            ] {
                match (choice, id) {
                    (ShutdownDocumentChoiceV1::Discard, None) => {}
                    (ShutdownDocumentChoiceV1::Save, Some(id)) => {
                        let original = self.read_shutdown_inventory_v1(inventory, f, r)?;
                        if !original.writes.iter().any(|w| {
                            w.write == *id && w.source == expected.source && w.kind == kind
                        }) {
                            return Err(CoordinatorError::Unavailable);
                        }
                        self.validate_shutdown_save_readiness(expected, kind, id, r)
                            .map_err(CoordinatorError::Resource)?;
                    }
                    _ => return Err(CoordinatorError::Unavailable),
                }
            }
            let plan = self.prepare_retained_disposal(&q.source, q.annotation_generation, r)?;
            self.commit_shutdown_retained_disposal(plan, &fence_id, r)
        }));
        let result = match result {
            Ok(result) => result,
            Err(panic) => {
                self.faulted = true;
                observe_service(
                    r,
                    Boundary::ShutdownDocumentDisposition,
                    &Err::<LifecycleState, _>(CoordinatorError::Faulted),
                    timer.finish(),
                );
                std::panic::resume_unwind(panic)
            }
        };
        if matches!(
            result,
            Err(CoordinatorError::Faulted
                | CoordinatorError::Resource(crate::resource::ResourceError::Faulted))
        ) {
            self.faulted = true;
        }
        observe_service(
            r,
            Boundary::ShutdownDocumentDisposition,
            &result,
            timer.finish(),
        );
        result
    }
}

fn disposition(choice: ShutdownDocumentChoiceV1) -> DisposalDispositionV1 {
    match choice {
        ShutdownDocumentChoiceV1::Save => DisposalDispositionV1::Retain,
        ShutdownDocumentChoiceV1::Discard => DisposalDispositionV1::Discard,
    }
}
