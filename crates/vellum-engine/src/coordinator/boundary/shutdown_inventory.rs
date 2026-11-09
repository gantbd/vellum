//! Retained exact-source captures/evidence, separate from current semantic text.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, LocalOwnerSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_boundaries(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::BoundaryEvidence;
        let state = &self.boundaries;
        let retained = b.slots(&state.rows)?;
        if state.limits.is_none() && !state.rows.is_empty() {
            return Err(CoordinatorError::Faulted);
        }
        // Pay bounded inspection work before native structural validation.
        for row in state.rows.iter().flatten() {
            b.step()?;
            if let Some(table) = &row.table {
                for _ in &table.graphemes {
                    b.step()?;
                }
                for _ in &table.words {
                    b.step()?;
                }
                for _ in &table.significant_words {
                    b.step()?;
                }
            }
            self.record_shutdown_pool_document(b, role, &row.version.document)?;
        }
        let bytes = if state.limits.is_some() {
            Some(
                self.boundary_inventory()
                    .map_err(|_| CoordinatorError::Faulted)?
                    .1,
            )
        } else {
            None
        };
        self.record_shutdown_local_owner(
            b,
            role,
            LocalOwnerSnapshot {
                configured: state.limits.is_some(),
                issued: Some(state.rows.len()),
                retained,
                held: bytes,
                reserved: None,
                limit: state.limits.map(|l| l.max_retained_bytes),
                additional_reserved: None,
            },
        )
    }
}
