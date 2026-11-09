//! Actual native owner state, original funding and retained dependencies.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_discovery_owner(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::Discovery;
        let p = match &self.discovery {
            State::Unconfigured => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Unconfigured);
            }
            State::Closed => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Closed { issued: None });
            }
            State::Active(p) => p,
        };
        self.validate_discovery()
            .map_err(|_| CoordinatorError::Faulted)?;
        let mut retained = b
            .slots(&p.packets)?
            .checked_add(b.slots(&p.contexts)?)
            .ok_or(CoordinatorError::Faulted)?;
        let mut additional = 0usize;
        for context in p.contexts.iter().flatten() {
            b.step()?;
            for _ in &context.selectors {
                b.step()?;
                retained = retained.checked_add(1).ok_or(CoordinatorError::Faulted)?;
            }
            let charge = context.decoder.shutdown_reservation(&self.retention)?;
            if let Some(charge) = charge {
                retained = retained.checked_add(1).ok_or(CoordinatorError::Faulted)?;
                additional = additional
                    .checked_add(charge)
                    .ok_or(CoordinatorError::Faulted)?;
            }
        }
        self.record_shutdown_pool(
            b,
            role,
            PoolSnapshot::Active {
                issued: p.issued,
                retained,
                charge: p.limits.max_held_bytes,
                expected: p.limits.max_held_bytes,
                reservation: &p.reservation,
                stopped: p.stopped,
                additional_reserved: additional,
            },
        )
    }
}
