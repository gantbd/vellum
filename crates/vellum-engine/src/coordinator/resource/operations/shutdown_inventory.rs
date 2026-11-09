//! Actual resource ownership and original budgets; not physical memory or effect completion.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, LocalOwnerSnapshot, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_owned_saves(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        self.validate_save_operations()
            .map_err(CoordinatorError::Resource)?;
        let role = ShutdownOwnerRoleV1::OwnedSaves;
        let s = &self.resources.operations;
        if self.resources.writer_closed {
            return self.record_shutdown_pool(b, role, PoolSnapshot::Closed { issued: None });
        }
        for row in &s.rows {
            b.step()?;
            if let Some(capture) = &row.capture {
                self.record_shutdown_pool_document(b, role, &capture.source.document)?;
                b.dependency(
                    ShutdownInventoryTargetV1::Owner(self.shutdown_owner_id(role)),
                    ShutdownInventoryTargetV1::Write(capture.task.clone()),
                )?;
            }
        }
        self.record_shutdown_local_owner(
            b,
            role,
            LocalOwnerSnapshot {
                additional_reserved: None,
                configured: s.limits.is_some(),
                issued: Some(s.rows.len()),
                retained: s.rows.len(),
                held: None,
                reserved: s.limits.map(|_| s.reserved),
                limit: s.limits.map(|l| l.max_store_bytes),
            },
        )
    }
}
