//! Trusted local payload retirement, not a public command, erasure or I/O.
use super::*;
#[cfg(test)]
mod tests;

/// Host premises outside managed native owners. These cannot override any
/// actual native retained incidence or lifecycle guard.
#[derive(Clone, Copy)]
pub struct TrustedDocumentReclamationV1 {
    pub authorized: Option<bool>,
    pub authored_disposition_ready: Option<bool>,
    pub session_references_released: Option<bool>,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
}
impl SourceCoordinator {
    /// Absorbing native root removal. Lifetime slots and annotation facts/floors
    /// stay spent. Unmanaged copies are not erased and cannot revalidate this
    /// removed incarnation. No substrate delete is issued.
    pub fn reclaim_disposed_document_v1(
        &mut self,
        document: &DocumentId,
        expected: LifecycleState,
        f: &TrustedDocumentReclamationV1,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let timing = r.begin_service_timing();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.ensure_operational()?;
            self.classify_reclamation_owners();
            if self.source_facade_busy {
                return Err(CoordinatorError::Unavailable);
            }
            self.measure_semantic_root()
                .map_err(|_| CoordinatorError::Faulted)?;
            if [
                f.authorized,
                f.authored_disposition_ready,
                f.session_references_released,
                f.retention_released,
                f.delivery_drained,
            ]
            .into_iter()
            .any(|v| v != Some(true))
            {
                return Err(CoordinatorError::Unavailable);
            }
            let owner = self.retained_document(document)?;
            if owner.lifecycle != expected {
                return Err(CoordinatorError::Stale);
            }
            if expected.phase != LifecyclePhase::ProvenanceReleased {
                return Err(CoordinatorError::Unavailable);
            }
            self.disposed_provenance_released(document)
                .map_err(CoordinatorError::Resource)?;
            if !owner.history.released() {
                return Err(CoordinatorError::Faulted);
            }
            self.annotations.validate_document_reclamation(document)?;
            // Recompute under the actual exclusive publisher, never from a
            // caller's copied count or an external discharge assertion.
            let mut quiet = Recorder::disabled();
            if self
                .document_resource_pins_v1(document, &mut quiet)
                .map_err(CoordinatorError::Resource)?
                .total()
                .map_err(CoordinatorError::Resource)?
                != 0
                || self.document_read_pins_v1(document, &mut quiet)?.total()? != 0
                || self
                    .document_context_pins_v1(document, &mut quiet)?
                    .total()?
                    != 0
            {
                return Err(CoordinatorError::Resource(
                    crate::resource::ResourceError::Retained,
                ));
            }
            let publication = self.next_retention_publication_generation(None)?;
            self.faulted = true;
            self.reclamation_fault_cut(0);
            self.annotations.reclaim_document_payload(document);
            self.reclamation_fault_cut(1);
            let removed = self.documents[document.slot]
                .take()
                .expect("validated retained owner");
            self.reclamation_fault_cut(2);
            drop(removed);
            self.generation = publication;
            self.reclamation_fault_cut(3);
            self.faulted = false;
            Ok(())
        }));
        match result {
            Ok(result) => {
                if matches!(result, Err(CoordinatorError::Faulted)) {
                    self.faulted = true;
                }
                observe_service(r, Boundary::DocumentReclamation, &result, timing.finish());
                result
            }
            Err(panic) => {
                self.faulted = true;
                observe_service(
                    r,
                    Boundary::DocumentReclamation,
                    &Err::<(), _>(CoordinatorError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(panic)
            }
        }
    }
    fn reclamation_fault_cut(&self, _cut: usize) {
        #[cfg(test)]
        assert_ne!(
            self.publication_fault_after,
            Some(_cut),
            "injected reclamation interruption"
        );
    }
    /// Exhaustive owner census. New fields force review at this final publisher,
    /// not just at a diagnostic projection or root-size accounting boundary.
    fn classify_reclamation_owners(&self) {
        let Self {
            diagnostic_capture: _, // Closed markers retain no document targets or bodies.
            shutdown: _,           // Control retention is not a direct document payload pin.
            // Direct retained targets are checked by the three native censuses.
            documents: _,
            annotations: _,
            views: _,
            workspaces: _,
            boundaries: _,
            invocations: _,
            history: _,
            resources: _,
            searches: _,
            source_contexts: _,
            publication_streams: _, // Included by the context census, even stopped.
            facet_publication_streams: _, // Included by the context census, even stopped.
            collection_publication_streams: _, // Included by the context census, even stopped.
            registry_observations: _, // Catalog metadata has no native document target or payload.
            history_inspection: _,
            managed_search: _,
            annotation_preparations: _,
            annotation_contexts: _,
            version_set_preparations: _,
            version_set_contexts: _,
            serialization_bases: _,
            // These are registry identities/opaque inert values and global
            // settings roots, not native document owners. Bound plans are in
            // their actual invocation captures, not inferred from label strings.
            registration_bases: _,
            discovery: _,
            settings_retention: _,
            settings_reader: _, // Retains settings data, never native document payloads.
            catalog_reads: _, // Retains catalog declarations/identity stamps, not document payloads.
            settings_plan_life: _,
            settings: _,
            // Profiles, owner identity, funding, fixed diagnostic cohorts and
            // publication/health state have no additional document references.
            root_byte_ceiling: _,
            configured_line_limits: _,
            retention: _,
            publication_measurements: _,
            release_measurements: _,
            invocation_measurements: _,
            settings_activation: _,
            settings_definition_limits: _,
            settings_layer_limits: _,
            settings_basis_profile: _,
            serialization_profile: _,
            namespace: _,
            generation: _,
            limits: _,
            faulted: _,
            read_result_invariant: _,
            source_facade_busy: _,
            source_facade_workspace: _,
            #[cfg(test)]
                source_facade_fault: _,
            #[cfg(test)]
                search_result_fault: _,
            #[cfg(test)]
                source_read_result_fault: _,
            #[cfg(test)]
                annotation_read_result_fault: _,
            #[cfg(test)]
                acquisition_result_fault: _,
            #[cfg(test)]
                publication_fault_after: _,
            #[cfg(test)]
                reversal_stage_fault: _,
            #[cfg(test)]
                annotation_context_install_fault: _,
            #[cfg(test)]
                version_set_preparation_fault: _,
            #[cfg(test)]
                version_set_install_fault: _,
            #[cfg(test)]
                version_set_selector_fault: _,
            #[cfg(test)]
                annotation_selector_registration_fault: _,
            #[cfg(test)]
                relation_publication_fault: _,
            #[cfg(test)]
                settings_publication_fault: _,
            #[cfg(test)]
                created_result_fault_after: _,
        } = self;
    }
}
