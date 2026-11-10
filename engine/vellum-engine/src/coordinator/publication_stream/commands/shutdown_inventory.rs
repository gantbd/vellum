//! Retained command/receipt records are obligations even after observer stop.
use super::*;
use crate::coordinator::shutdown::inventory::Builder;
impl State {
    pub(in crate::coordinator::publication_stream) fn shutdown_summary(
        &self,
        o: &SourceCoordinator,
        b: &mut Builder,
        owner: &Arc<()>,
    ) -> Result<(usize, usize), CoordinatorError> {
        let Self::Active(_) = self else {
            return Ok((0, 0));
        };
        let t = self
            .table(&o.retention)
            .map_err(|_| CoordinatorError::Faulted)?;
        t.receipts
            .validate_claims(owner, &t.rows, &o.retention)
            .map_err(|_| CoordinatorError::Faulted)?;
        let retained = b.slots(&t.rows)?;
        for row in t.rows.iter().flatten() {
            b.step()?;
            o.record_shutdown_pool_document(
                b,
                ShutdownOwnerRoleV1::SourcePublication,
                &row.scope.document,
            )?;
        }
        let (receipts, charge) = t.receipts.shutdown_summary(o, b)?;
        Ok((
            retained
                .checked_add(receipts)
                .ok_or(CoordinatorError::Faulted)?,
            t.charge
                .checked_add(charge)
                .ok_or(CoordinatorError::Faulted)?,
        ))
    }
}
