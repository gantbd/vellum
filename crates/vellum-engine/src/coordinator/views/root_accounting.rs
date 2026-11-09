use super::*;
use crate::coordinator::root_accounting::{Bytes, Result};
impl SourceCoordinator {
    pub(super) fn check_selection_root(
        &self,
        before: &SelectionSet,
        after: &SelectionSet,
        r: &mut Recorder,
    ) -> std::result::Result<(), SelectionError> {
        let removed = before
            .selections()
            .len()
            .checked_mul(std::mem::size_of::<Selection>())
            .ok_or(SelectionError::Limit)?;
        let added = after
            .selections()
            .len()
            .checked_mul(std::mem::size_of::<Selection>())
            .ok_or(SelectionError::Limit)?;
        self.check_root_replacement(removed, added, r)
            .map_err(error)
    }
}
impl ViewState {
    pub(in crate::coordinator) fn root_identity(&self) -> ViewRootIdentity {
        let Self {
            limits,
            line_limits,
            unicode_limits,
            rows: _,
        } = self;
        // Row creation/retirement/attachment/selection/companions use the owner
        // publication generation. Only these immutable optional profiles do not.
        (*limits, *line_limits, *unicode_limits)
    }
    pub(in crate::coordinator) fn root_bytes_after(&self, decision: &StagedViews) -> Result<usize> {
        self.root_bytes_with(Some(decision))
    }
    pub(in crate::coordinator) fn root_bytes(&self) -> Result<usize> {
        self.root_bytes_with(None)
    }
    fn root_bytes_with(&self, after: Option<&StagedViews>) -> Result<usize> {
        let mut bytes = Bytes::default();
        bytes.rows::<Self>(1)?;
        bytes.rows::<Option<View>>(self.rows.len())?;
        for (slot, view) in self.rows.iter().enumerate() {
            let view = after
                .and_then(|decision| decision.updates.iter().find(|(id, _)| *id == slot))
                .map(|(_, view)| view)
                .or(view.as_ref());
            let Some(view) = view else {
                continue;
            };
            bytes.rows::<Selection>(view.selections.selections().len())?;
        }
        Ok(bytes.finish())
    }
}
