//! Every captured participant, including hidden/invalid group participants.
use super::*;
use crate::coordinator::shutdown::inventory::Builder;
impl HistoryBasisV1 {
    pub(in crate::coordinator) fn capture_shutdown_dependencies(
        &self,
        o: &SourceCoordinator,
        b: &mut Builder,
        role: ShutdownOwnerRoleV1,
    ) -> Result<(), CoordinatorError> {
        let mut emit = |scope: &HistoryBasisScopeV1| {
            b.step()?;
            let document = match scope {
                HistoryBasisScopeV1::Source(d) => d,
                HistoryBasisScopeV1::Annotation(a) => &a.document,
            };
            o.record_shutdown_pool_document(b, role, document)
        };
        for scope in &self.scopes {
            emit(&scope.scope)?;
        }
        for group in &self.groups {
            for scope in &group.scopes {
                emit(scope)?;
            }
        }
        Ok(())
    }
}
