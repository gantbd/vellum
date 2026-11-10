//! Actual separately funded source receipt owner, not returned host copies.
use super::*;
use crate::coordinator::shutdown::inventory::Builder;
impl State {
    pub(in crate::coordinator::publication_stream::commands) fn shutdown_summary(
        &self,
        o: &SourceCoordinator,
        b: &mut Builder,
    ) -> Result<(usize, usize), CoordinatorError> {
        let Self::Active(_) = self else {
            return Ok((0, 0));
        };
        let p = self
            .pool(&o.retention)
            .map_err(|_| CoordinatorError::Faulted)?;
        let retained = b.slots(&p.rows)?;
        for row in p.rows.iter().flatten() {
            b.step()?;
            o.record_shutdown_pool_document(
                b,
                ShutdownOwnerRoleV1::SourcePublication,
                &row.scope.document,
            )?;
        }
        Ok((retained, p.charge))
    }
}
