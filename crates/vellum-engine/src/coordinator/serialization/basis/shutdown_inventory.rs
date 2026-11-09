//! Actual native pool state and funding; no body/snapshot acquisition.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_serialization_basis_pool(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::SerializationBases;
        let p = match &self.serialization_bases {
            BasisPool::Unconfigured => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Unconfigured);
            }
            BasisPool::Closed { issued } => {
                return self.record_shutdown_pool(
                    b,
                    role,
                    PoolSnapshot::Closed {
                        issued: Some(*issued),
                    },
                );
            }
            BasisPool::Active(p) => p,
        };

        let retained = b.slots(&p.rows)?;
        for row in p.rows.iter().flatten() {
            b.step()?;
            self.record_shutdown_pool_document(b, role, &row.document)?;
        }

        self.record_shutdown_pool(
            b,
            role,
            PoolSnapshot::Active {
                issued: p.rows.len(),
                retained,
                charge: p.charge,
                expected: p
                    .limits
                    .required_pool_bytes()
                    .map_err(|_| CoordinatorError::Faulted)?,
                reservation: &p.reservation,
                stopped: false,
                additional_reserved: 0,
            },
        )
    }
}
