//! Actual native pool state and funding; no body/snapshot acquisition.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_annotation_preparation_pool(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::AnnotationPreparations;
        let p = match &self.annotation_preparations {
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

        let retained = b.slots(&p.cuts)?;
        for cut in p.cuts.iter().flatten() {
            b.step()?;
            self.record_shutdown_pool_document(b, role, &cut.scope.document)?;
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
                    .required_pool_bytes()
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
