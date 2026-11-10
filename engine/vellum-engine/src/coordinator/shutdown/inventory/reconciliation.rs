//! Closed native-owner census and logical parent reconciliation, not drain truth.
use super::*;
enum FundingKind {
    Pool,
    Primary,
    Descendants,
    Local,
}
impl ShutdownOwnerRoleV1 {
    fn funding(self) -> FundingKind {
        use ShutdownOwnerRoleV1::*;
        match self {
            SourceReader => FundingKind::Primary,
            SourceContexts
            | AnnotationPreparations
            | AnnotationContexts
            | VersionSetPreparations
            | VersionSetContexts
            | ManagedSearch
            | SettingsRetention
            | SettingsReader
            | ResourceCatalogReads
            | DiagnosticCapture
            | SerializationBases
            | OperationObservers
            | SourcePublication
            | FacetPublication
            | CollectionPublication
            | RegistryObservation
            | RegistrationBases
            | Discovery
            | HistoryInspection
            | PublicationMeasurements
            | ReleaseMeasurements
            | InvocationMeasurements
            | ShutdownControl
            | OwnedSourceReads
            | OwnedAnnotationReads => FundingKind::Pool,
            Invocations | InvocationObservers => FundingKind::Descendants,
            SharedWriter | AnnotationReader | History | SearchSessions | Annotations
            | ResourceMetadata | OwnedSaves | SourceContinuity | SemanticRoot | LogicalViews
            | Workspaces | BoundaryEvidence | SettingsPreparations => FundingKind::Local,
        }
    }
}
impl Builder {
    pub(super) fn account_owner(
        &mut self,
        row: &ShutdownOwnerRecordV1,
    ) -> Result<(), CoordinatorError> {
        let ordinal = row.owner.role as u32;
        if ordinal >= SHUTDOWN_OWNER_COUNT
            || self
                .owner_namespace
                .as_ref()
                .is_some_and(|owner| !Arc::ptr_eq(owner, &row.owner.owner))
        {
            return Err(CoordinatorError::Faulted);
        }
        let bit = 1u64 << ordinal;
        if self.owner_mask & bit != 0 {
            return Err(CoordinatorError::Faulted);
        }
        let charge = match row.owner.role.funding() {
            FundingKind::Pool | FundingKind::Primary => match row.state {
                ShutdownOwnerStateV1::Active => {
                    let primary = row.reserved_bytes.ok_or(CoordinatorError::Faulted)?;
                    let extra = match row.owner.role.funding() {
                        FundingKind::Primary => {
                            if row.additional_reserved_bytes.is_some() {
                                return Err(CoordinatorError::Faulted);
                            }
                            0
                        }
                        _ => row
                            .additional_reserved_bytes
                            .ok_or(CoordinatorError::Faulted)?,
                    };
                    primary
                        .checked_add(extra)
                        .ok_or(CoordinatorError::Faulted)?
                }
                ShutdownOwnerStateV1::Unconfigured | ShutdownOwnerStateV1::Closed => {
                    if row.reserved_bytes.is_some() || row.additional_reserved_bytes.is_some() {
                        return Err(CoordinatorError::Faulted);
                    }
                    0
                }
            },
            FundingKind::Descendants => {
                if row.reserved_bytes.is_some() {
                    return Err(CoordinatorError::Faulted);
                }
                row.additional_reserved_bytes
                    .ok_or(CoordinatorError::Faulted)?
            }
            FundingKind::Local => {
                if row.reserved_bytes.is_some() || row.additional_reserved_bytes.is_some() {
                    return Err(CoordinatorError::Faulted);
                }
                0
            }
        };
        self.parent_bytes = self
            .parent_bytes
            .checked_add(charge)
            .ok_or(CoordinatorError::Faulted)?;
        self.owner_mask |= bit;
        if self.owner_namespace.is_none() {
            self.owner_namespace = Some(row.owner.owner.clone());
        }
        Ok(())
    }
}
impl SourceCoordinator {
    pub(super) fn reconcile_shutdown_owners(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        self.classify_shutdown_owners();
        // Collection already validates and accumulates each closed role once.
        // No second traversal/work debit is needed for the census or funding.
        if b.owner_mask != (1u64 << SHUTDOWN_OWNER_COUNT) - 1
            || b.owner_namespace
                .as_ref()
                .is_none_or(|owner| !Arc::ptr_eq(owner, &self.namespace))
        {
            return Err(CoordinatorError::Faulted);
        }
        let parent = self.retention_usage();
        if parent
            .maximum
            .map_or(parent.reserved != 0, |maximum| parent.reserved > maximum)
        {
            return Err(CoordinatorError::Faulted);
        }
        let unknown = parent
            .reserved
            .checked_sub(b.parent_bytes)
            .ok_or(CoordinatorError::Faulted)?;
        b.cut.funding = ShutdownFundingV1 {
            parent_reserved_bytes: parent.reserved,
            inventoried_parent_bytes: b.parent_bytes,
            unrepresented_parent_bytes: unknown,
        };
        b.cut.owner_coverage = unknown == 0;
        Ok(())
    }
    /// Compile-time census: new coordinator fields must be assigned an actual
    /// native projector or an explicit non-owner category here, not silently
    /// omitted from a complete-coverage claim. Nested native projectors and
    /// root accounting retain their own exhaustive owning-type classifications.
    fn classify_shutdown_owners(&self) {
        let Self {
            // Dedicated native pool/store projections.
            diagnostic_capture: _,
            shutdown: _,
            registration_bases: _,
            registry_observations: _,
            discovery: _,
            publication_measurements: _,
            release_measurements: _,
            invocation_measurements: _,
            source_contexts: _,
            publication_streams: _,
            facet_publication_streams: _,
            collection_publication_streams: _,
            history_inspection: _,
            managed_search: _,
            annotation_preparations: _,
            annotation_contexts: _,
            version_set_preparations: _,
            version_set_contexts: _,
            settings_retention: _,
            settings_reader: _,
            catalog_reads: _,
            settings_plan_life: _,
            serialization_bases: _,
            workspaces: _,
            boundaries: _,
            views: _,
            invocations: _,
            history: _,
            resources: _,
            searches: _,
            annotations: _,
            // Current semantic root projection and inert configuration/identity.
            settings: _,
            documents: _,
            root_byte_ceiling: _,
            configured_line_limits: _,
            settings_activation: _,
            settings_definition_limits: _,
            settings_layer_limits: _,
            settings_basis_profile: _,
            serialization_profile: _,
            namespace: _,
            limits: _,
            // Funding/health/publication and synchronous first-party gateway
            // state. No external temporary claim becomes covered by assertion:
            // actual parent reconciliation above must still balance.
            retention: _,
            generation: _,
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
