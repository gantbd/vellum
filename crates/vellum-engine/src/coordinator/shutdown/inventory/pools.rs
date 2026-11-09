//! Native managed-store projections, not declaration/usage defaults.
use super::*;
pub(in crate::coordinator) struct LocalOwnerSnapshot {
    pub configured: bool,
    pub issued: Option<usize>,
    pub retained: usize,
    pub held: Option<usize>,
    pub reserved: Option<usize>,
    pub limit: Option<usize>,
    pub additional_reserved: Option<usize>,
}
pub(in crate::coordinator) enum PoolSnapshot<'a> {
    Unconfigured,
    Closed {
        issued: Option<usize>,
    },
    Active {
        issued: usize,
        retained: usize,
        charge: usize,
        expected: usize,
        reservation: &'a retention::Reservation,
        stopped: bool,
        additional_reserved: usize,
    },
}
impl Builder {
    pub(in crate::coordinator) fn slots<T>(
        &mut self,
        slots: &[Option<T>],
    ) -> Result<usize, CoordinatorError> {
        let mut retained = 0usize;
        for slot in slots {
            self.step()?;
            retained = retained
                .checked_add(usize::from(slot.is_some()))
                .ok_or(CoordinatorError::Limit)?;
        }
        Ok(retained)
    }
}
impl SourceCoordinator {
    fn capture_shutdown_semantic_root(&self, b: &mut Builder) -> Result<(), CoordinatorError> {
        let root = self
            .measure_semantic_root()
            .map_err(|_| CoordinatorError::Faulted)?;
        let role = ShutdownOwnerRoleV1::SemanticRoot;
        for (slot, document) in self.documents.iter().enumerate() {
            b.step()?;
            if document.is_some() {
                self.record_shutdown_pool_document(
                    b,
                    role,
                    &DocumentId {
                        namespace: self.namespace.clone(),
                        slot,
                    },
                )?;
            }
        }
        self.record_shutdown_local_owner(
            b,
            role,
            LocalOwnerSnapshot {
                configured: true,
                issued: None,
                retained: 1,
                held: Some(root.total_bytes),
                reserved: None,
                limit: self.root_byte_ceiling,
                additional_reserved: None,
            },
        )
    }
    pub(in crate::coordinator) fn record_shutdown_local_owner(
        &self,
        b: &mut Builder,
        role: ShutdownOwnerRoleV1,
        s: LocalOwnerSnapshot,
    ) -> Result<(), CoordinatorError> {
        b.owner(ShutdownOwnerRecordV1 {
            owner: self.shutdown_owner_id(role),
            state: if s.configured {
                ShutdownOwnerStateV1::Active
            } else {
                ShutdownOwnerStateV1::Unconfigured
            },
            admission_stopped: self.shutdown_fence_marker().is_some(),
            authority_fenced: self.shutdown_fence_marker().is_some(),
            issued_slots: if s.configured { s.issued } else { None },
            retained_records: if s.configured { Some(s.retained) } else { None },
            reserved_bytes: None,
            additional_reserved_bytes: s.additional_reserved,
            local_reserved_bytes: s.reserved,
            local_byte_limit: s.limit,
            operation_registry_state: None,
            held_bytes: s.held,
            retirement_supported: false,
        })
    }
    fn capture_shutdown_control_owner(&self, b: &mut Builder) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::ShutdownControl;
        let Some(fence) = &self.shutdown.fence else {
            if self.shutdown.retired {
                return self.record_shutdown_pool(
                    b,
                    role,
                    PoolSnapshot::Closed { issued: Some(1) },
                );
            }
            return self.record_shutdown_pool(b, role, PoolSnapshot::Unconfigured);
        };
        if self.shutdown.retired {
            return Err(CoordinatorError::Faulted);
        }
        let wait = fence.wait.shutdown_reservation(&self.retention)?;
        let attempt = fence.attempt.reservation(&self.retention)?;
        let accepted = fence
            .accepted
            .active()
            .map(|a| a.metadata_charge(&self.retention))
            .transpose()?;
        let additional = wait
            .unwrap_or(0)
            .checked_add(accepted.unwrap_or(0))
            .and_then(|n| n.checked_add(attempt.unwrap_or(0)))
            .ok_or(CoordinatorError::Faulted)?;
        self.record_shutdown_pool(
            b,
            role,
            PoolSnapshot::Active {
                issued: 1,
                retained: 1
                    + usize::from(wait.is_some())
                    + usize::from(accepted.is_some())
                    + usize::from(attempt.is_some()),
                charge: fence.charge,
                expected: fence.basis.limits.required_capture_bytes()?,
                reservation: &fence.reservation,
                stopped: true,
                additional_reserved: additional,
            },
        )
    }
    pub(in crate::coordinator) fn record_shutdown_pool(
        &self,
        b: &mut Builder,
        role: ShutdownOwnerRoleV1,
        snapshot: PoolSnapshot<'_>,
    ) -> Result<(), CoordinatorError> {
        let (state, issued, retained, charge, stopped, additional) = match snapshot {
            PoolSnapshot::Unconfigured => (
                ShutdownOwnerStateV1::Unconfigured,
                None,
                None,
                None,
                false,
                None,
            ),
            PoolSnapshot::Closed { issued } => (
                ShutdownOwnerStateV1::Closed,
                issued,
                Some(0),
                None,
                true,
                None,
            ),
            PoolSnapshot::Active {
                issued,
                retained,
                charge,
                expected,
                reservation,
                stopped,
                additional_reserved,
            } => {
                if charge != expected {
                    return Err(CoordinatorError::Faulted);
                }
                self.retention
                    .validate_exact(reservation, expected)
                    .map_err(|_| CoordinatorError::Faulted)?;
                (
                    ShutdownOwnerStateV1::Active,
                    Some(issued),
                    Some(retained),
                    Some(charge),
                    stopped,
                    Some(additional_reserved),
                )
            }
        };
        b.owner(ShutdownOwnerRecordV1 {
            owner: self.shutdown_owner_id(role),
            state,
            admission_stopped: stopped,
            authority_fenced: self.shutdown_fence_marker().is_some(),
            issued_slots: issued,
            retained_records: retained,
            reserved_bytes: charge,
            additional_reserved_bytes: additional,
            local_reserved_bytes: None,
            local_byte_limit: None,
            operation_registry_state: None,
            held_bytes: None,
            retirement_supported: !matches!(
                role,
                ShutdownOwnerRoleV1::ShutdownControl
                    | ShutdownOwnerRoleV1::OwnedSaves
                    | ShutdownOwnerRoleV1::SourceContinuity
            ),
        })
    }
    pub(in crate::coordinator) fn record_shutdown_pool_document(
        &self,
        b: &mut Builder,
        role: ShutdownOwnerRoleV1,
        document: &DocumentId,
    ) -> Result<(), CoordinatorError> {
        self.retained_document(document)
            .map_err(|_| CoordinatorError::Faulted)?;
        b.dependency(
            ShutdownInventoryTargetV1::Owner(self.shutdown_owner_id(role)),
            ShutdownInventoryTargetV1::Document(document.clone()),
        )
    }
    pub(super) fn capture_shutdown_managed_pools(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        self.capture_shutdown_diagnostic_owner(b)?;
        self.capture_shutdown_source_context_pool(b)?;
        self.capture_shutdown_annotation_preparation_pool(b)?;
        self.capture_shutdown_annotation_context_pool(b)?;
        self.capture_shutdown_version_preparation_pool(b)?;
        self.capture_shutdown_version_context_pool(b)?;
        self.capture_shutdown_managed_search_pool(b)?;
        self.capture_shutdown_settings_retention_pool(b)?;
        self.capture_shutdown_settings_reader_pool(b)?;
        self.capture_shutdown_catalog_read_owner(b)?;
        self.capture_shutdown_settings_preparations(b)?;
        self.capture_shutdown_serialization_basis_pool(b)?;
        self.capture_shutdown_operation_observation_pool(b)?;
        self.capture_shutdown_source_publication_owner(b)?;
        self.capture_shutdown_facet_publication_owner(b)?;
        self.capture_shutdown_collection_publication_owner(b)?;
        self.capture_shutdown_registry_observation_owner(b)?;
        self.capture_shutdown_registration_bases_owner(b)?;
        self.capture_shutdown_discovery_owner(b)?;
        self.capture_shutdown_history_inspection_owner(b)?;
        self.capture_shutdown_publication_measurements_owner(b)?;
        self.capture_shutdown_release_measurements_owner(b)?;
        self.capture_shutdown_invocation_measurements_owner(b)?;
        self.capture_shutdown_control_owner(b)?;
        self.capture_shutdown_invocation_ledger(b)?;
        self.capture_shutdown_invocation_observers(b)?;
        self.capture_shutdown_history_store(b)?;
        self.capture_shutdown_search_store(b)?;
        self.capture_shutdown_annotation_store(b)?;
        self.capture_shutdown_resource_metadata(b)?;
        self.capture_shutdown_owned_saves(b)?;
        self.capture_shutdown_owned_source_reads(b)?;
        self.capture_shutdown_owned_annotation_reads(b)?;
        self.capture_shutdown_continuity(b)?;
        self.capture_shutdown_logical_views(b)?;
        self.capture_shutdown_workspaces(b)?;
        self.capture_shutdown_boundaries(b)?;
        self.capture_shutdown_semantic_root(b)
    }
}
