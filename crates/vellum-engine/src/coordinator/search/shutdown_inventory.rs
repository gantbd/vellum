//! Native local-store obligations; no body copies or invented parent reservations.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, LocalOwnerSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_search_store(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::SearchSessions;
        let state = &self.searches;
        if state.limits.is_none() && (!state.sessions.is_empty() || !state.results.is_empty()) {
            return Err(CoordinatorError::Faulted);
        }
        let sessions = b.slots(&state.sessions)?;
        for session in state.sessions.iter().flatten() {
            b.step()?;
            self.record_shutdown_pool_document(b, role, &session.document)?;
        }
        let mut results = 0usize;
        for result in &state.results {
            b.step()?;
            if let Some(result) = result.upgrade() {
                results = results.checked_add(1).ok_or(CoordinatorError::Faulted)?;
                self.record_shutdown_pool_document(b, role, &result.expected.document)?;
            }
        }
        if state.limits.is_some_and(|l| {
            state.sessions.len() > l.max_sessions_issued
                || sessions > l.max_live_sessions
                || results > l.max_live_results
        }) {
            return Err(CoordinatorError::Faulted);
        }
        self.record_shutdown_local_owner(
            b,
            role,
            LocalOwnerSnapshot {
                additional_reserved: None,
                configured: state.limits.is_some(),
                issued: Some(state.sessions.len()),
                retained: sessions
                    .checked_add(results)
                    .ok_or(CoordinatorError::Faulted)?,
                held: None,
                reserved: None,
                limit: None,
            },
        )
    }
}
