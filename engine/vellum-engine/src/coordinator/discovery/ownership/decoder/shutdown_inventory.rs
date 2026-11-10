//! Actual decoder child reservation, never an inferred aggregate headroom.
use super::*;
impl State {
    pub(in crate::coordinator::discovery::ownership) fn shutdown_reservation(
        &self,
        ledger: &retention::RetentionState,
    ) -> Result<Option<usize>, CoordinatorError> {
        let Self::Active(d) = self else {
            return Ok(None);
        };
        self.validate(ledger)
            .map_err(|_| CoordinatorError::Faulted)?;
        ledger
            .validate_exact(&d.reservation, d.charge)
            .map_err(|_| CoordinatorError::Faulted)?;
        Ok(Some(d.charge))
    }
}
