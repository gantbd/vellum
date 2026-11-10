//! Semantic view ownership, not a UI or key-map projection.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, LocalOwnerSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_logical_views(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let state = &self.views;
        let role = ShutdownOwnerRoleV1::LogicalViews;
        let retained = b.slots(&state.rows)?;
        if state.limits.is_none() && !state.rows.is_empty() {
            return Err(CoordinatorError::Faulted);
        }
        let mut selections = 0usize;
        for row in state.rows.iter().flatten() {
            b.step()?;
            if row.stamp.attachment_generation == 0
                || row.stamp.selection_generation == 0
                || row.stamp.viewport_generation == 0
            {
                return Err(CoordinatorError::Faulted);
            }
            selections = selections
                .checked_add(row.selections.selections().len())
                .ok_or(CoordinatorError::Faulted)?;
            if let Some(source) = &row.source {
                self.record_shutdown_pool_document(b, role, &source.document)?;
            } else if !row.selections.selections().is_empty() || row.selections.primary() != 0 {
                return Err(CoordinatorError::Faulted);
            }
        }
        if state.limits.is_some_and(|l| {
            state.rows.len() > l.max_issued_views
                || retained > l.max_live_views
                || selections > l.max_total_selections
        }) {
            return Err(CoordinatorError::Faulted);
        }
        self.record_shutdown_local_owner(
            b,
            role,
            LocalOwnerSnapshot {
                configured: state.limits.is_some(),
                issued: Some(state.rows.len()),
                retained,
                held: None,
                reserved: None,
                limit: None,
                additional_reserved: None,
            },
        )
    }
}
