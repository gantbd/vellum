//! Compare final handoff to actual candidate and staged aliases before installation.
use super::*;
impl SourceCoordinator {
    pub(super) fn valid_version_acquisition(
        &self,
        cut: &PreparedSet,
        aliases: &[Option<Alias>],
        reply: &VersionSetAcquisitionV1,
        original: Option<&VersionSetContextId>,
    ) -> Result<bool, Error> {
        let p = self.version_context_pool()?;
        let parent = p.rows.len();
        Ok(Arc::ptr_eq(&reply.context.owner, &self.namespace)
            && reply.context.slot == parent
            && original.is_none_or(|original| reply.context != *original)
            && !cut.members.is_empty()
            && reply.participants.len() == cut.members.len()
            && aliases.len() == cut.members.len()
            && reply.participants.iter().zip(&cut.members).enumerate().all(
                |(index, (part, member))| {
                    part.document == member.scope.document
                        && part.data == member.value.classes
                        && Arc::ptr_eq(&part.snapshot.owner, &self.namespace)
                        && Some(part.snapshot.slot) == p.aliases.len().checked_add(index)
                        && aliases[index]
                            .as_ref()
                            .is_some_and(|v| v.parent == parent && v.index == index)
                        && reply.participants[..index]
                            .iter()
                            .all(|prior| prior.snapshot != part.snapshot)
                },
            ))
    }
}
