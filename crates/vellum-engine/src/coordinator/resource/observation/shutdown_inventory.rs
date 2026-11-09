//! Actual native pool state and funding; no body/snapshot acquisition.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_operation_observation_pool(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::OperationObservers;
        let p = match &self.resources.observation {
            State::Unconfigured => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Unconfigured);
            }
            State::Closed => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Closed { issued: None });
            }
            State::Active(p) => p,
        };
        let retained = b.slots(&p.rows)?;
        let mut additional_reserved = 0usize;
        for record in p.rows.iter().flatten() {
            b.step()?;
            self.retention
                .validate_exact(&record.reservation, record.limits.max_held_bytes)
                .map_err(|_| CoordinatorError::Faulted)?;
            let decoder_charge = record.decoder.shutdown_reservation(&self.retention)?;
            additional_reserved = additional_reserved
                .checked_add(record.limits.max_held_bytes)
                .and_then(|n| n.checked_add(decoder_charge))
                .ok_or(CoordinatorError::Faulted)?;
        }
        let charge = p
            .limits
            .required_pool_bytes()
            .map_err(|_| CoordinatorError::Faulted)?;
        self.record_shutdown_pool(
            b,
            role,
            PoolSnapshot::Active {
                issued: p.rows.len(),
                retained,
                charge,
                expected: charge,
                reservation: &p.reservation,
                stopped: p.stopped,
                additional_reserved,
            },
        )
    }
}
