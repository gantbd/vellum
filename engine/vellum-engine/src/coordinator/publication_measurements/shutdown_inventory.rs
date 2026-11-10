//! Actual native owner state, original funding and retained dependencies.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_publication_measurements_owner(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::PublicationMeasurements;
        let p = match &self.publication_measurements {
            State::Unconfigured => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Unconfigured);
            }
            State::Closed => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Closed { issued: None });
            }
            State::Active(p) => p,
        };

        self.record_shutdown_pool(
            b,
            role,
            PoolSnapshot::Active {
                issued: 0,
                retained: 0,
                charge: PublicationMeasurementLimitsV1::required_pool_bytes(),
                expected: PublicationMeasurementLimitsV1::required_pool_bytes(),
                reservation: &p.reservation,
                stopped: p.stopped,
                additional_reserved: 0,
            },
        )
    }
}
