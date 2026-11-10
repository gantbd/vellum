//! Native local-store obligations; no body copies or invented parent reservations.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, LocalOwnerSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_annotation_store(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::Annotations;
        let state = &self.annotations;
        if state.records.len() != state.retired.len()
            || state.records.len() != state.issued_documents.len()
            || state.limits.is_none() && (!state.records.is_empty() || !state.snapshots.is_empty())
        {
            return Err(CoordinatorError::Faulted);
        }
        let mut retained = b
            .slots(&state.records)?
            .checked_add(b.slots(&state.retired)?)
            .ok_or(CoordinatorError::Faulted)?;
        let mut live = 0usize;
        let mut bodies = 0usize;
        for (slot, record) in state
            .records
            .iter()
            .enumerate()
            .filter_map(|(i, r)| r.as_ref().map(|r| (i, r)))
        {
            b.step()?;
            if record.value.id.slot != slot
                || record.value.id.document != state.issued_documents[slot]
                || state.retired[slot].is_some()
            {
                return Err(CoordinatorError::Faulted);
            }
            live = live.checked_add(1).ok_or(CoordinatorError::Faulted)?;
            bodies = bodies
                .checked_add(record.value.body.len())
                .ok_or(CoordinatorError::Faulted)?;
            self.record_shutdown_pool_document(b, role, &record.value.id.document)?;
        }
        let mut snapshots = 0usize;
        for snapshot in &state.snapshots {
            b.step()?;
            if let Some(snapshot) = snapshot.upgrade() {
                snapshots = snapshots.checked_add(1).ok_or(CoordinatorError::Faulted)?;
                self.record_shutdown_pool_document(b, role, &snapshot.source.document)?;
            }
        }
        retained = retained
            .checked_add(snapshots)
            .ok_or(CoordinatorError::Faulted)?;
        if state.limits.is_some_and(|l| {
            state.records.len() > l.max_issued
                || live > l.max_records
                || bodies > l.max_retained_body_bytes
                || snapshots > l.max_live_snapshots
        }) {
            return Err(CoordinatorError::Faulted);
        }
        self.record_shutdown_local_owner(
            b,
            role,
            LocalOwnerSnapshot {
                additional_reserved: None,
                configured: state.limits.is_some(),
                issued: Some(state.records.len()),
                retained,
                held: None,
                reserved: None,
                limit: None,
            },
        )
    }
}
