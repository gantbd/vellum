//! Actual native owner state, original funding and retained dependencies.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_registration_bases_owner(
        &self,
        b: &mut Builder,
    ) -> std::result::Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::RegistrationBases;
        let p = match &self.registration_bases {
            State::Unconfigured => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Unconfigured);
            }
            State::Closed => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Closed { issued: None });
            }
            State::Active(p) => p,
        };
        self.registration_basis_store()
            .map_err(|_| CoordinatorError::Faulted)?;
        for _ in &p.records {
            b.step()?;
        }
        self.record_shutdown_pool(
            b,
            role,
            PoolSnapshot::Active {
                issued: p.issued,
                retained: p.records.len(),
                charge: p.limits.max_held_bytes,
                expected: p.limits.max_held_bytes,
                reservation: &p.reservation,
                stopped: false,
                additional_reserved: 0,
            },
        )
    }
}
