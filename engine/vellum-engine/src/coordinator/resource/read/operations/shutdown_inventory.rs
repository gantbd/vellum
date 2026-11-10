//! Actual resource ownership and original budgets; not physical memory or effect completion.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_owned_annotation_reads(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        self.validate_read_operations()
            .map_err(CoordinatorError::Resource)?;
        let role = ShutdownOwnerRoleV1::OwnedAnnotationReads;
        let s = match &self.resources.read_operations {
            State::Unconfigured => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Unconfigured);
            }
            State::Closed => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Closed { issued: None });
            }
            State::Active(s) => s,
        };
        for row in &s.rows {
            b.step()?;
            if let Some(c) = &row.capture {
                self.record_shutdown_pool_document(b, role, &c.source.document)?;
                b.dependency(
                    ShutdownInventoryTargetV1::Owner(self.shutdown_owner_id(role)),
                    ShutdownInventoryTargetV1::AnnotationRead(c.task.clone()),
                )?;
            }
        }
        let limits = self.resources.limits.ok_or(CoordinatorError::Faulted)?;
        let charge = reserved(s.limits, limits).map_err(|_| CoordinatorError::Faulted)?;
        self.record_shutdown_pool(
            b,
            role,
            PoolSnapshot::Active {
                issued: s.rows.len(),
                retained: s.rows.len(),
                charge,
                expected: charge,
                reservation: &s.reservation,
                stopped: self.resources.annotation_read_admission_stopped,
                additional_reserved: 0,
            },
        )
    }
}
