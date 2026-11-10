use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_catalog_read_owner(
        &self,
        b: &mut Builder,
    ) -> std::result::Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::ResourceCatalogReads;
        let p = match &self.catalog_reads {
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
        let packets = b.slots(&p.packets)?;
        let contexts = b.slots(&p.contexts)?;
        let mut selectors = 0usize;
        for c in p.contexts.iter().flatten() {
            for entry in &c.entries {
                b.step()?;
                selectors = selectors
                    .checked_add(usize::from(*entry))
                    .ok_or(CoordinatorError::Limit)?;
            }
            for _ in &c.cursors {
                b.step()?;
                selectors = selectors.checked_add(1).ok_or(CoordinatorError::Limit)?;
            }
        }
        if (packets, contexts, selectors) != (p.live_packets, p.live_contexts, p.live_selectors) {
            return Err(CoordinatorError::Faulted);
        }
        self.record_shutdown_pool(
            b,
            role,
            PoolSnapshot::Active {
                issued: p.issued,
                retained: packets
                    .checked_add(contexts)
                    .ok_or(CoordinatorError::Limit)?,
                charge: p.charge,
                expected: p
                    .limits
                    .required_pool_bytes()
                    .map_err(|_| CoordinatorError::Faulted)?,
                reservation: &p.reservation,
                stopped: false,
                additional_reserved: p.decode_charge,
            },
        )
    }
}
