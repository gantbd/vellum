//! Actual native owner state, original funding and retained dependencies.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_history_inspection_owner(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::HistoryInspection;
        let p = match &self.history_inspection {
            State::Unconfigured => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Unconfigured);
            }
            State::Closed { issued } => {
                return self.record_shutdown_pool(
                    b,
                    role,
                    PoolSnapshot::Closed {
                        issued: Some(*issued),
                    },
                );
            }
            State::Active(p) => p,
        };
        let retained = b.slots(&p.rows)?;
        for row in p.rows.iter().flatten() {
            b.step()?;
            row.basis.capture_shutdown_dependencies(self, b, role)?;
        }
        let expected = p
            .limits
            .required_pool_bytes()
            .map_err(|_| CoordinatorError::Faulted)?
            .checked_add(
                scope_bytes(&p.scope).map_err(|_| CoordinatorError::Faulted)?
                    - size_of::<HistoryInspectionScopeV1>(),
            )
            .filter(|n| *n <= p.limits.max_held_bytes)
            .ok_or(CoordinatorError::Faulted)?;
        self.record_shutdown_pool(
            b,
            role,
            PoolSnapshot::Active {
                issued: p.rows.len(),
                retained,
                charge: p.charge,
                expected,
                reservation: &p.reservation,
                stopped: p.stopped,
                additional_reserved: 0,
            },
        )
    }
}
