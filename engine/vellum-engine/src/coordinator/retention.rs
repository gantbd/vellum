//! Serialized owner-local logical pool reservations, not physical memory accounting.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetentionErrorV1 {
    Unavailable,
    Limit,
    Faulted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetentionUsageV1 {
    pub maximum: Option<usize>,
    pub reserved: usize,
}
#[derive(Debug, Default)]
pub(super) struct RetentionState {
    owner: Arc<()>,
    maximum: Option<usize>,
    reserved: usize,
}
#[derive(Debug)]
pub(super) struct Reservation {
    owner: Arc<()>,
    bytes: usize,
    spent: bool,
}
impl RetentionState {
    pub(super) fn check(&self, bytes: usize) -> Result<usize, RetentionErrorV1> {
        let maximum = self.maximum.ok_or(RetentionErrorV1::Unavailable)?;
        if self.reserved > maximum {
            return Err(RetentionErrorV1::Faulted);
        }
        self.reserved
            .checked_add(bytes)
            .filter(|n| *n <= maximum)
            .ok_or(RetentionErrorV1::Limit)
    }
    pub(super) fn reserve(&mut self, bytes: usize) -> Result<Reservation, RetentionErrorV1> {
        let next = self.check(bytes)?;
        self.reserved = next;
        Ok(Reservation {
            owner: self.owner.clone(),
            bytes,
            spent: false,
        })
    }
    pub(super) fn release(
        &mut self,
        reservation: &mut Reservation,
    ) -> Result<(), RetentionErrorV1> {
        self.validate(reservation)?;
        self.reserved -= reservation.bytes;
        reservation.spent = true;
        Ok(())
    }
    pub(super) fn validate(&self, reservation: &Reservation) -> Result<(), RetentionErrorV1> {
        let maximum = self.maximum.ok_or(RetentionErrorV1::Faulted)?;
        if !Arc::ptr_eq(&self.owner, &reservation.owner)
            || reservation.spent
            || self.reserved > maximum
            || self.reserved < reservation.bytes
        {
            return Err(RetentionErrorV1::Faulted);
        }
        Ok(())
    }
    pub(super) fn validate_exact(
        &self,
        reservation: &Reservation,
        bytes: usize,
    ) -> Result<(), RetentionErrorV1> {
        self.validate(reservation)?;
        if reservation.bytes != bytes {
            return Err(RetentionErrorV1::Faulted);
        }
        Ok(())
    }
}
impl SourceCoordinator {
    pub fn configure_retention_budget(
        &mut self,
        maximum: usize,
        r: &mut Recorder,
    ) -> Result<bool, RetentionErrorV1> {
        let result = (|| {
            self.ensure_operational()
                .map_err(|_| RetentionErrorV1::Faulted)?;
            if let Some(old) = self.retention.maximum {
                if self.retention.reserved > old {
                    return Err(RetentionErrorV1::Faulted);
                }
                return if old == maximum {
                    Ok(false)
                } else {
                    Err(RetentionErrorV1::Unavailable)
                };
            }
            if self.retention.reserved != 0 {
                return Err(RetentionErrorV1::Faulted);
            }
            self.retention.maximum = Some(maximum);
            Ok(true)
        })();
        if result == Err(RetentionErrorV1::Faulted) {
            self.faulted = true;
        }
        r.record(Observation {
            boundary: Boundary::RetentionConfiguration,
            disposition: match result {
                Ok(_) => Disposition::Completed,
                Err(RetentionErrorV1::Faulted) => Disposition::Unknown,
                Err(_) => Disposition::Refused,
            },
            service_time: None,
        });
        result
    }
    pub fn retention_usage(&self) -> RetentionUsageV1 {
        RetentionUsageV1 {
            maximum: self.retention.maximum,
            reserved: self.retention.reserved,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reservations_consume_actual_shared_headroom_and_cannot_overflow() {
        let mut state = RetentionState {
            maximum: Some(10),
            reserved: 0,
            ..RetentionState::default()
        };
        let mut first = state.reserve(7).unwrap();
        assert_eq!(state.validate_exact(&first, 7), Ok(()));
        assert_eq!(
            state.validate_exact(&first, 6),
            Err(RetentionErrorV1::Faulted)
        );
        let mut foreign = RetentionState {
            maximum: Some(10),
            reserved: 10,
            ..RetentionState::default()
        };
        assert_eq!(foreign.release(&mut first), Err(RetentionErrorV1::Faulted));
        assert_eq!(foreign.reserved, 10);
        assert!(matches!(state.reserve(4), Err(RetentionErrorV1::Limit)));
        let mut second = state.reserve(3).unwrap();
        assert_eq!(state.reserved, 10);
        assert!(matches!(
            state.reserve(usize::MAX),
            Err(RetentionErrorV1::Limit)
        ));
        state.release(&mut first).unwrap();
        assert_eq!(state.reserved, 3);
        assert_eq!(state.release(&mut first), Err(RetentionErrorV1::Faulted));
        state.release(&mut second).unwrap();
        assert_eq!(state.reserved, 0);
        assert_eq!(state.release(&mut second), Err(RetentionErrorV1::Faulted));
    }
}
