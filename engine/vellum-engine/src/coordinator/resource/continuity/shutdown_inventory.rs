//! Actual resource ownership and original budgets; not physical memory or effect completion.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, LocalOwnerSnapshot, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_continuity(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        self.validate_continuity_credit()
            .map_err(CoordinatorError::Resource)?;
        let role = ShutdownOwnerRoleV1::SourceContinuity;
        let s = &self.resources.continuity;
        if self.resources.writer_closed {
            return self.record_shutdown_pool(b, role, PoolSnapshot::Closed { issued: None });
        }
        let mut retained = 0usize;
        for (slot, t) in self.resources.tasks.iter().enumerate() {
            b.step()?;
            if let Some(t) = t
                && t.continuity.is_some()
            {
                retained = retained.checked_add(1).ok_or(CoordinatorError::Faulted)?;
                b.dependency(
                    ShutdownInventoryTargetV1::Owner(self.shutdown_owner_id(role)),
                    ShutdownInventoryTargetV1::Write(SaveId {
                        owner: self.namespace.clone(),
                        slot,
                    }),
                )?;
            }
        }
        self.record_shutdown_local_owner(
            b,
            role,
            LocalOwnerSnapshot {
                additional_reserved: None,
                configured: s.limits.is_some(),
                issued: None,
                retained,
                held: None,
                reserved: s.limits.map(|_| s.charged),
                limit: s.limits.map(|l| l.max_retained_bytes),
            },
        )
    }
}
