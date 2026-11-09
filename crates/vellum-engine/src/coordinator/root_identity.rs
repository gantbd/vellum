//! Build-local identity of native-semantic-root/1, not a hash, byte count,
//! authority grant, retained root, wire token or external registry attestation.
use super::*;
#[derive(Clone)]
pub(super) struct NativeRootIdentity {
    owner: Arc<()>,
    publication: u64,
    faulted: bool,
    shutdown: Option<Arc<()>>,
    views: views::ViewRootIdentity,
    workspaces: Option<WorkspaceLimits>,
    resources: resource::ResourceRootIdentity,
}
impl PartialEq for NativeRootIdentity {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
            && self.publication == other.publication
            && self.faulted == other.faulted
            && match (&self.shutdown, &other.shutdown) {
                (None, None) => true,
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                _ => false,
            }
            && self.views == other.views
            && self.workspaces == other.workspaces
            && self.resources == other.resources
    }
}
impl Eq for NativeRootIdentity {}
impl std::fmt::Debug for NativeRootIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeRootIdentity(<opaque>)")
    }
}
impl SourceCoordinator {
    pub(super) fn native_root_identity(&self) -> Result<NativeRootIdentity, RootAccountingErrorV1> {
        // Exhaustive owner census. New fields require reviewing both the byte
        // projection and this identity's mutation coverage; no Debug/serialization
        // fallback or silently growing tuple is permitted.
        let Self {
            diagnostic_capture: _, // Optional observations are not semantic publications.
            shutdown: _,           // Exact monotonic admission transition stamped below.
            registration_bases: _, // Conditional authority pool, not a semantic publication.
            namespace: _,
            generation: _,
            faulted: _,
            documents: _,
            history: _,
            annotations: _,
            settings: _,
            views: _,
            workspaces: _,
            resources: _,
            limits: _, // Immutable construction profile, bound by owner incarnation.
            root_byte_ceiling: _, // Admission policy, not native current-root payload.
            configured_line_limits: _,
            settings_activation: _,
            settings_definition_limits: _,
            settings_layer_limits: _,
            settings_basis_profile: _,
            serialization_profile: _,
            // Separate retained work, admission profiles and diagnostics are not
            // native current-root payload. This does not stamp an external accepted
            // command registry or imply complete public-facade interpretation truth.
            discovery: _,
            retention: _,
            publication_measurements: _,
            release_measurements: _,
            invocation_measurements: _,
            source_contexts: _,
            publication_streams: _, // Separate bounded sampled projection owner.
            facet_publication_streams: _, // Separate bounded named-facet observer owner.
            collection_publication_streams: _, // Separate bounded collection observer owner.
            registry_observations: _, // Bounded catalog metadata, no authored root identity.
            history_inspection: _,
            managed_search: _,
            annotation_preparations: _,
            annotation_contexts: _,
            version_set_preparations: _,
            version_set_contexts: _,
            settings_retention: _,
            settings_reader: _, // Separately funded recipient retention, not semantic state.
            catalog_reads: _,   // Separately funded metadata frontiers, not semantic publication.
            settings_plan_life: _,
            serialization_bases: _,
            boundaries: _,
            invocations: _,
            searches: _,
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
        // Source/policy/format/lifecycle, annotations, linked history, settings
        // installation, view/workspace rows, bindings/baselines and export
        // reservations publish through the checked non-reused owner generation.
        // Non-publication root headers are captured exactly by their native owners.
        Ok(NativeRootIdentity {
            shutdown: self.shutdown_fence_marker(),
            owner: self.namespace.clone(),
            publication: self.generation,
            faulted: self.faulted,
            views: self.views.root_identity(),
            workspaces: self.workspaces.root_identity(),
            resources: self.resources.root_identity(&self.namespace)?,
        })
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn native_identity_binds_original_owner_and_excludes_separately_funded_retention() {
        let (mut a, mut r, _) = crate::coordinator::serialization::tests::setup();
        let (b, mut rb, _) = crate::coordinator::serialization::tests::setup();
        assert_eq!(a.generation(), b.generation());
        assert_eq!(
            a.semantic_root_usage(&mut r).unwrap(),
            b.semantic_root_usage(&mut rb).unwrap()
        );
        let before = a.native_root_identity().unwrap();
        assert_ne!(before, b.native_root_identity().unwrap());
        assert_eq!(before, a.native_root_identity().unwrap());
        assert!(a.configure_retention_budget(1048576, &mut r).unwrap());
        assert_eq!(before, a.native_root_identity().unwrap());
        a.create(b"another authored object", &mut r).unwrap();
        assert_ne!(before, a.native_root_identity().unwrap());
        assert_eq!(format!("{before:?}"), "NativeRootIdentity(<opaque>)");
    }
}
