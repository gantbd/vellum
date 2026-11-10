//! Actual native owner state, original funding and retained dependencies.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_collection_publication_owner(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::CollectionPublication;
        let p = match &self.collection_publication_streams {
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
        let retained = b.slots(&p.rows)?;
        if p.rows.len() > p.issued
            || p.issued > p.limits.max_issued
            || retained > p.limits.max_streams
        {
            return Err(CoordinatorError::Faulted);
        }
        let (commands, additional) = p.commands.shutdown_summary(self, b, &p.owner)?;
        let retained = retained
            .checked_add(commands)
            .ok_or(CoordinatorError::Faulted)?;
        for (slot, row) in p
            .rows
            .iter()
            .enumerate()
            .filter_map(|(i, r)| r.as_ref().map(|r| (i, r)))
        {
            b.step()?;
            if !row.valid(
                &CollectionPublicationStreamIdV1 {
                    owner: p.owner.clone(),
                    slot,
                },
                p.limits,
                p.issued,
            ) {
                return Err(CoordinatorError::Faulted);
            }
            self.record_shutdown_pool_document(b, role, &row.scope.source.document)?;
        }
        let expected = p
            .limits
            .required_pool_bytes()
            .map_err(|_| CoordinatorError::Faulted)?
            .checked_add(p.authority.len())
            .and_then(|n| n.checked_add(p.engine.len()))
            .filter(|n| *n <= p.limits.max_pool_bytes)
            .ok_or(CoordinatorError::Faulted)?;
        self.record_shutdown_pool(
            b,
            role,
            PoolSnapshot::Active {
                issued: p.issued,
                retained,
                charge: p.charge,
                expected,
                reservation: &p.reservation,
                stopped: false,
                additional_reserved: additional,
            },
        )
    }
}
