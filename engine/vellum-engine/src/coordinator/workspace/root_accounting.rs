use super::*;
use crate::coordinator::root_accounting::{Bytes, Result};
impl WorkspaceState {
    pub(in crate::coordinator) fn root_identity(&self) -> Option<WorkspaceLimits> {
        let Self {
            limits,
            rows: _,
            live: _,
            memberships: _,
        } = self;
        // All row/membership publication uses the owner generation.
        *limits
    }
    pub(in crate::coordinator) fn root_bytes(&self) -> Result<usize> {
        let mut bytes = Bytes::default();
        bytes.rows::<Self>(1)?;
        bytes.rows::<Option<Workspace>>(self.rows.len())?;
        for workspace in self.rows.iter().flatten() {
            bytes.rows::<DocumentId>(workspace.members.len())?;
        }
        Ok(bytes.finish())
    }
}
