//! Actual native pool state and funding; no body/snapshot acquisition.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_managed_search_pool(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::ManagedSearch;
        let p = match &self.managed_search {
            State::Unconfigured => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Unconfigured);
            }
            State::Closed { issued } => {
                return self.record_shutdown_pool(
                    b,
                    role,
                    PoolSnapshot::Closed {
                        issued: Some(*issued),
                    },
                );
            }
            State::Active(p) => p,
        };

        let retained = b.slots(&p.rows)?;
        p.validate_gauges().map_err(|_| CoordinatorError::Faulted)?;
        if retained != p.live
            || p.rows.iter().flatten().filter(|r| r.stopped).count() != p.invalidated
        {
            return Err(CoordinatorError::Faulted);
        }
        let source_slots = match &self.source_contexts {
            super::super::State::Active(p) => p.holds.len(),
            super::super::State::Unconfigured | super::super::State::Closed { .. } => 0,
        };
        let mut seen = [
            b.hold_bitmap(source_slots)?,
            b.hold_bitmap(self.shutdown_annotation_hold_slots())?,
            b.hold_bitmap(self.shutdown_version_hold_slots())?,
        ];
        for row in p.rows.iter().flatten() {
            b.step()?;
            if row.version.document != row.scope.document {
                return Err(CoordinatorError::Faulted);
            }
            self.validate_search_issuer_hold(&row.source, &row.hold, &row.scope)
                .map_err(|_| CoordinatorError::Faulted)?;
            // Only after native issuer/owner validation may a local ordinal be
            // used. Different issuer families have independent slot spaces.
            let (family, slot) = match &row.hold {
                IssuerHold::Source(h) => (0, h.slot),
                IssuerHold::Combined(h) => (1, h.shutdown_slot()),
                IssuerHold::VersionSet(h) => (2, h.shutdown_slot()),
            };
            let visited = seen[family]
                .get_mut(slot)
                .ok_or(CoordinatorError::Faulted)?;
            if *visited != 0 {
                return Err(CoordinatorError::Faulted);
            }
            *visited = 1;
            self.record_shutdown_pool_document(b, role, &row.scope.document)?;
            let required = match &row.source {
                Issuer::Source(_) => ShutdownOwnerRoleV1::SourceContexts,
                Issuer::Combined(_) => ShutdownOwnerRoleV1::AnnotationContexts,
                Issuer::VersionSet { .. } => ShutdownOwnerRoleV1::VersionSetContexts,
            };
            b.dependency(
                ShutdownInventoryTargetV1::Owner(self.shutdown_owner_id(role)),
                ShutdownInventoryTargetV1::Owner(self.shutdown_owner_id(required)),
            )?;
        }

        self.record_shutdown_pool(
            b,
            role,
            PoolSnapshot::Active {
                issued: p.issued,
                retained,
                charge: p.charge,
                expected: p
                    .limits
                    .required_pool_bytes()
                    .map_err(|_| CoordinatorError::Faulted)?
                    .checked_add(p.authority.len())
                    .and_then(|n| n.checked_add(p.engine.len()))
                    .filter(|n| *n <= p.limits.max_pool_bytes)
                    .ok_or(CoordinatorError::Faulted)?,
                reservation: &p.reservation,
                stopped: false,
                additional_reserved: 0,
            },
        )
    }
}
