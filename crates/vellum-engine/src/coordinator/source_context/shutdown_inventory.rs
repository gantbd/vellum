//! Actual native pool state and funding; no body/snapshot acquisition.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_source_context_pool(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::SourceContexts;
        let p = match &self.source_contexts {
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

        let retained = b
            .slots(&p.rows)?
            .checked_add(b.slots(&p.holds)?)
            .ok_or(CoordinatorError::Faulted)?;
        if let Some(row) = p.rows.iter().flatten().next() {
            self.source_contexts.document_pins(&row.scope.document)?;
        } else if retained != 0 {
            return Err(CoordinatorError::Faulted);
        }
        for row in p.rows.iter().flatten() {
            b.step()?;
            self.record_shutdown_pool_document(b, role, &row.scope.document)?;
        }

        self.record_shutdown_pool(
            b,
            role,
            PoolSnapshot::Active {
                issued: p.issued().map_err(|_| CoordinatorError::Faulted)?,
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
