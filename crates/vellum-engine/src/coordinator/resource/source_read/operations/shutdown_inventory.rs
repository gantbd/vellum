//! Actual resource ownership and original budgets; not physical memory or effect completion.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_owned_source_reads(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::OwnedSourceReads;
        let reader = match &self.resources.source_reader {
            super::super::State::Unconfigured => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Unconfigured);
            }
            super::super::State::Closed => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Closed { issued: None });
            }
            super::super::State::Active(reader) => reader,
        };
        self.validate_source_read_operations()
            .map_err(CoordinatorError::Resource)?;
        let s = match &reader.operations {
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
            if row.owned {
                b.dependency(
                    ShutdownInventoryTargetV1::Owner(self.shutdown_owner_id(role)),
                    ShutdownInventoryTargetV1::SourceRead(row.task.clone()),
                )?;
            }
        }
        let charge = reserved(s.limits, s.max_issued).map_err(|_| CoordinatorError::Faulted)?;
        self.record_shutdown_pool(
            b,
            role,
            PoolSnapshot::Active {
                issued: s.rows.len(),
                retained: s.rows.len(),
                charge,
                expected: charge,
                reservation: &s.reservation,
                stopped: reader.stopped,
                additional_reserved: 0,
            },
        )
    }
}
