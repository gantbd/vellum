//! Native local-store obligations; no body copies or invented parent reservations.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, LocalOwnerSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_history_store(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::History;
        let state = &self.history;
        if state.limits.is_none()
            && (!state.groups.is_empty() || !state.pins.is_empty() || state.bytes != 0)
        {
            return Err(CoordinatorError::Faulted);
        }
        let mut bytes = 0usize;
        for group in &state.groups {
            b.step()?;
            bytes = bytes
                .checked_add(group.bytes)
                .ok_or(CoordinatorError::Faulted)?;
            for member in &group.members {
                b.step()?;
                self.record_shutdown_pool_document(b, role, &member.document)?;
            }
            for member in &group.annotations {
                b.step()?;
                self.record_shutdown_pool_document(b, role, &member.id.document)?;
            }
        }
        for pin in &state.pins {
            b.step()?;
            if !state.groups.iter().any(|g| g.id == pin.group) {
                return Err(CoordinatorError::Faulted);
            }
        }
        if bytes != state.bytes
            || state.limits.is_some_and(|l| {
                state.groups.len() > l.max_retained_groups
                    || state.pins.len() > l.max_pins
                    || bytes > l.max_journal_bytes
            })
        {
            return Err(CoordinatorError::Faulted);
        }
        let retained = state
            .groups
            .len()
            .checked_add(state.pins.len())
            .ok_or(CoordinatorError::Faulted)?;
        self.record_shutdown_local_owner(
            b,
            role,
            LocalOwnerSnapshot {
                additional_reserved: None,
                configured: state.limits.is_some(),
                issued: None,
                retained,
                held: state.limits.map(|_| bytes),
                reserved: None,
                limit: state.limits.map(|l| l.max_journal_bytes),
            },
        )
    }
}
