//! Actual native pool state and funding; no body/snapshot acquisition.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn shutdown_annotation_hold_slots(&self) -> usize {
        match &self.annotation_contexts {
            InstalledState::Active(p) => p.holds.len(),
            InstalledState::Unconfigured | InstalledState::Closed { .. } => 0,
        }
    }
    pub(in crate::coordinator) fn capture_shutdown_annotation_context_pool(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::AnnotationContexts;
        let p = match &self.annotation_contexts {
            InstalledState::Unconfigured => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Unconfigured);
            }
            InstalledState::Closed { issued } => {
                return self.record_shutdown_pool(
                    b,
                    role,
                    PoolSnapshot::Closed {
                        issued: Some(*issued),
                    },
                );
            }
            InstalledState::Active(p) => p,
        };

        let contexts = b.slots(&p.rows)?;
        let selectors = b.slots(&p.selectors)?;
        let holds = b.slots(&p.holds)?;
        let retained = contexts
            .checked_add(selectors)
            .and_then(|n| n.checked_add(holds))
            .ok_or(CoordinatorError::Faulted)?;
        if let Some(row) = p.rows.iter().flatten().next() {
            p.document_pins(&row.scope.document)?;
        } else if retained != 0 {
            return Err(CoordinatorError::Faulted);
        }
        for selector in p.selectors.iter().flatten() {
            b.step()?;
            if p.rows
                .get(selector.context())
                .and_then(Option::as_ref)
                .is_none()
            {
                return Err(CoordinatorError::Faulted);
            }
        }
        for row in p.rows.iter().flatten() {
            b.step()?;
            self.record_shutdown_pool_document(b, role, &row.scope.document)?;
        }

        self.record_shutdown_pool(
            b,
            role,
            PoolSnapshot::Active {
                issued: p.issued().map_err(|_| CoordinatorError::Faulted)?,
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
