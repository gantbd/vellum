//! Native membership containers and exact document edges; no storage locations.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, LocalOwnerSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_workspaces(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let state = &self.workspaces;
        let role = ShutdownOwnerRoleV1::Workspaces;
        let retained = b.slots(&state.rows)?;
        if state.limits.is_none() && !state.rows.is_empty() {
            return Err(CoordinatorError::Faulted);
        }
        let mut memberships = 0usize;
        let mut seen = b.hold_bitmap(if retained == 0 {
            0
        } else {
            self.documents.len()
        })?;
        for row in state.rows.iter().flatten() {
            b.step()?;
            if row.generation == 0 {
                return Err(CoordinatorError::Faulted);
            }
            for member in &row.members {
                b.step()?;
                self.record_shutdown_pool_document(b, role, member)?;
                let visited = seen.get_mut(member.slot).ok_or(CoordinatorError::Faulted)?;
                if *visited != 0 {
                    return Err(CoordinatorError::Faulted);
                }
                *visited = 1;
                memberships = memberships
                    .checked_add(1)
                    .ok_or(CoordinatorError::Faulted)?;
            }
            for member in &row.members {
                b.step()?;
                seen[member.slot] = 0;
            }
        }
        if retained != state.live
            || memberships != state.memberships
            || state.limits.is_some_and(|l| {
                state.rows.len() > l.max_issued
                    || retained > l.max_live
                    || memberships > l.max_memberships
            })
        {
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
