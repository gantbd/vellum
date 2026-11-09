//! Actual catalog declarations and tombstones, not runtime provider authority.
use super::*;
use crate::coordinator::shutdown::inventory::Builder;
impl State {
    pub(in crate::coordinator::resource) fn shutdown_records(
        &self,
        b: &mut Builder,
    ) -> std::result::Result<(bool, usize), CoordinatorError> {
        if self.root.is_none() && self.limits.is_none() && self.floor == 0 && !self.faulted {
            return Ok((false, 0));
        }
        let (root, _) = self.current().map_err(|_| CoordinatorError::Faulted)?;
        for _ in &root.rows {
            b.step()?;
        }
        Ok((true, root.rows.len()))
    }
}
