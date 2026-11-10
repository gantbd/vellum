//! Actual native pool state and funding; no body/snapshot acquisition.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn shutdown_version_hold_slots(&self) -> usize {
        match &self.version_set_contexts {
            VersionSetContextState::Active(p) => p.holds.len(),
            VersionSetContextState::Unconfigured | VersionSetContextState::Closed { .. } => 0,
        }
    }
    pub(in crate::coordinator) fn capture_shutdown_version_context_pool(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::VersionSetContexts;
        let p = match &self.version_set_contexts {
            VersionSetContextState::Unconfigured => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Unconfigured);
            }
            VersionSetContextState::Closed { issued } => {
                return self.record_shutdown_pool(
                    b,
                    role,
                    PoolSnapshot::Closed {
                        issued: Some(*issued),
                    },
                );
            }
            VersionSetContextState::Active(p) => p,
        };

        let row_count = b.slots(&p.rows)?;
        let aliases = b.slots(&p.aliases)?;
        let selectors = b.slots(&p.selectors)?;
        let holds = b.slots(&p.holds)?;
        let retained = row_count
            .checked_add(aliases)
            .and_then(|n| n.checked_add(selectors))
            .and_then(|n| n.checked_add(holds))
            .ok_or(CoordinatorError::Faulted)?;
        if row_count == 0 && retained != 0 {
            return Err(CoordinatorError::Faulted);
        }
        for alias in p.aliases.iter().flatten() {
            b.step()?;
            if p.rows
                .get(alias.parent)
                .and_then(Option::as_ref)
                .is_none_or(|r| alias.index >= r.value.members.len())
            {
                return Err(CoordinatorError::Faulted);
            }
        }
        for selector in p.selectors.iter().flatten() {
            b.step()?;
            if p.aliases
                .get(selector.context())
                .and_then(Option::as_ref)
                .is_none()
            {
                return Err(CoordinatorError::Faulted);
            }
        }
        for (i, row) in p
            .rows
            .iter()
            .enumerate()
            .filter_map(|(i, r)| r.as_ref().map(|r| (i, r)))
        {
            b.step()?;
            self.validate_version_aliases(i)
                .map_err(|_| CoordinatorError::Faulted)?;
            if let Some(member) = row.value.members.first() {
                p.document_pins(&member.scope.document)?;
            }
            for member in &row.value.members {
                b.step()?;
                self.record_shutdown_pool_document(b, role, &member.scope.document)?;
            }
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
