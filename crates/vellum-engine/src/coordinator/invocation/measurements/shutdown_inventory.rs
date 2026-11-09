//! Actual native owner state, original funding and retained dependencies.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_invocation_measurements_owner(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::InvocationMeasurements;
        let p = match &self.invocation_measurements {
            State::Unconfigured => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Unconfigured);
            }
            State::Closed => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Closed { issued: None });
            }
            State::Active(p) => p,
        };
        let expected = InvocationMeasurementLimitsV1::required_pool_bytes(
            self.invocations.limits.max_lifetime_issued,
        )
        .map_err(|_| CoordinatorError::Faulted)?
        .checked_add(
            Timing::additional_bytes(
                self.invocations.limits.max_lifetime_issued,
                p.timing.profile(),
            )
            .map_err(|_| CoordinatorError::Faulted)?,
        )
        .ok_or(CoordinatorError::Faulted)?;
        self.record_shutdown_pool(
            b,
            role,
            PoolSnapshot::Active {
                issued: 0,
                retained: 0,
                charge: expected,
                expected,
                reservation: &p.reservation,
                stopped: p.stopped,
                additional_reserved: 0,
            },
        )
    }
}
