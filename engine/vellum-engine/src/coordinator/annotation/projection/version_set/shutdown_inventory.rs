//! Actual native pool state and funding; no body/snapshot acquisition.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_version_preparation_pool(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::VersionSetPreparations;
        let p = match &self.version_set_preparations {
            VersionSetPreparationState::Unconfigured => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Unconfigured);
            }
            VersionSetPreparationState::Closed { issued } => {
                return self.record_shutdown_pool(
                    b,
                    role,
                    PoolSnapshot::Closed {
                        issued: Some(*issued),
                    },
                );
            }
            VersionSetPreparationState::Active(p) => p,
        };

        let retained = b.slots(&p.cuts)?;
        for cut in p.cuts.iter().flatten() {
            b.step()?;
            for member in &cut.members {
                b.step()?;
                self.record_shutdown_pool_document(b, role, &member.scope.document)?;
            }
        }

        self.record_shutdown_pool(
            b,
            role,
            PoolSnapshot::Active {
                issued: p.cuts.len(),
                retained,
                charge: p.charge,
                expected: p
                    .limits
                    .required_pool_bytes(p.parent_root_ceiling)
                    .map_err(|_| CoordinatorError::Faulted)?
                    .checked_add(p.authority.len())
                    .and_then(|n| n.checked_add(p.engine.len()))
                    .filter(|n| *n <= p.limits.max_pool_bytes)
                    .ok_or(CoordinatorError::Faulted)?,
                reservation: &p.reservation,
                stopped: false,
                additional_reserved: 0,
            },
        )
    }
}
