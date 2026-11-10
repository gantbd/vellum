//! Cooperative host-clock waiting under the original fence. Never worker control
//! or shutdown acceptance/completion; unknown owner coverage remains unknown.
use super::*;
#[cfg(test)]
mod tests;

/// Same host monotonic epoch/tick product as ordinary invocation observation.
pub type ShutdownWaitClockV1 = InvocationWaitClockV1;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShutdownWaitRequestV1 {
    pub clock: ShutdownWaitClockV1,
    /// Declared host-clock ticks, not engine calls, milliseconds or CPU steps.
    pub budget_ticks: u64,
    pub report_limits: ShutdownInventoryLimitsV1,
}
impl ShutdownWaitRequestV1 {
    /// Logical control frame plus one complete maximum cutoff report; not
    /// physical allocator occupancy or producer/effect retention.
    pub fn required_retained_bytes(self) -> Result<usize, CoordinatorError> {
        size_of::<Control>()
            .checked_add(self.report_limits.max_capture_bytes)
            .ok_or(CoordinatorError::Limit)
    }
}
pub struct TrustedShutdownWaitFactsV1<'a> {
    pub inventory: &'a TrustedShutdownInventoryFactsV1<'a>,
    pub clock_valid: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownWaitEventV1 {
    Poll,
    Tick(ShutdownWaitClockV1),
    /// Current owner authority ends waiting, not the engine or its effects.
    End,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownWaitCutoffReasonV1 {
    BudgetEnded,
    ClockUnavailable,
    AuthorizedWaitEnded,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownCutoffReportStateV1 {
    Available,
    Unavailable(CoordinatorError),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownWaitStatusV1 {
    Waiting {
        remaining_ticks: u64,
    },
    Cutoff {
        reason: ShutdownWaitCutoffReasonV1,
        remaining_ticks: Option<u64>,
        report: ShutdownCutoffReportStateV1,
    },
}
pub(super) enum State {
    Unstarted,
    Active(Box<Control>),
    Retired,
}
impl State {
    pub(super) fn shutdown_reservation(
        &self,
        ledger: &retention::RetentionState,
    ) -> Result<Option<usize>, CoordinatorError> {
        let Self::Active(control) = self else {
            return Ok(None);
        };
        let charge = control.request.required_retained_bytes()?;
        ledger
            .validate_exact(&control.reservation, charge)
            .map_err(|_| CoordinatorError::Faulted)?;
        Ok(Some(charge))
    }
}
pub(super) struct Control {
    request: ShutdownWaitRequestV1,
    last: ShutdownWaitClockV1,
    status: ShutdownWaitStatusV1,
    cutoff: Option<Result<ShutdownProgressV1, CoordinatorError>>,
    reservation: retention::Reservation,
}
impl SourceCoordinator {
    fn shutdown_wait_control(
        &self,
        fence: &ShutdownFenceIdV1,
        facts: &TrustedShutdownFenceFactsV1<'_>,
    ) -> Result<&Control, CoordinatorError> {
        self.validate_shutdown_owner_control(fence, facts)?;
        let installed = self
            .shutdown
            .fence
            .as_ref()
            .ok_or(CoordinatorError::Unavailable)?;
        let State::Active(control) = &installed.wait else {
            return Err(CoordinatorError::Unavailable);
        };
        self.retention
            .validate_exact(
                &control.reservation,
                control.request.required_retained_bytes()?,
            )
            .map_err(|_| CoordinatorError::Faulted)?;
        Ok(control)
    }
    /// Start the one cooperative wait associated with this installed fence.
    /// Identical retries return its current status without renewing the clock,
    /// budget or reservation. Changed intent and post-retirement restarts refuse.
    /// Report space and a bounded current capture are checked before admission.
    pub fn start_shutdown_wait_v1(
        &mut self,
        fence: &ShutdownFenceIdV1,
        request: ShutdownWaitRequestV1,
        facts: &TrustedShutdownWaitFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ShutdownWaitStatusV1, CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.validate_shutdown_owner_control(fence, facts.inventory.control)?;
            match &self
                .shutdown
                .fence
                .as_ref()
                .ok_or(CoordinatorError::Unavailable)?
                .wait
            {
                State::Active(_) => {
                    let control = self.shutdown_wait_control(fence, facts.inventory.control)?;
                    return if control.request == request {
                        Ok(control.status)
                    } else {
                        Err(CoordinatorError::Unavailable)
                    };
                }
                State::Retired => return Err(CoordinatorError::Unavailable),
                State::Unstarted => {}
            }
            self.ensure_operational()?;
            if facts.clock_valid != Some(true) {
                return Err(CoordinatorError::Unavailable);
            }
            let charge = request.required_retained_bytes()?;
            self.retention.check(charge).map_err(wait_retention_error)?;
            let initial = self.capture_shutdown_progress_v1(
                fence,
                request.report_limits,
                facts.inventory,
                r,
            )?;
            let reservation = self
                .retention
                .reserve(charge)
                .map_err(wait_retention_error)?;
            let (status, cutoff) = if request.budget_ticks == 0 {
                (
                    ShutdownWaitStatusV1::Cutoff {
                        reason: ShutdownWaitCutoffReasonV1::BudgetEnded,
                        remaining_ticks: Some(0),
                        report: ShutdownCutoffReportStateV1::Available,
                    },
                    Some(Ok(initial)),
                )
            } else {
                (
                    ShutdownWaitStatusV1::Waiting {
                        remaining_ticks: request.budget_ticks,
                    },
                    None,
                )
            };
            self.shutdown
                .fence
                .as_mut()
                .ok_or(CoordinatorError::Faulted)?
                .wait = State::Active(Box::new(Control {
                request,
                last: request.clock,
                status,
                cutoff,
                reservation,
            }));
            Ok(status)
        })();
        if result == Err(CoordinatorError::Faulted) {
            self.faulted = true;
        }
        observe_service(r, Boundary::ShutdownWaitStart, &result, timer.finish());
        result
    }
    /// No sleeping, callbacks or producer actions. Only valid elapsed ticks
    /// consume the budget. Invalid/backwards/foreign epochs end waiting with
    /// unknown remaining time; repeated polls never manufacture time progress.
    /// A cutoff is absorbing even when its report cannot be captured.
    pub fn advance_shutdown_wait_v1(
        &mut self,
        fence: &ShutdownFenceIdV1,
        event: ShutdownWaitEventV1,
        facts: &TrustedShutdownWaitFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ShutdownWaitStatusV1, CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let control = self.shutdown_wait_control(fence, facts.inventory.control)?;
            let ShutdownWaitStatusV1::Waiting { remaining_ticks } = control.status else {
                return Ok(control.status);
            };
            let request = control.request;
            let (reason, remaining, next_clock) = match event {
                ShutdownWaitEventV1::End => (
                    Some(ShutdownWaitCutoffReasonV1::AuthorizedWaitEnded),
                    Some(remaining_ticks),
                    None,
                ),
                _ if facts.clock_valid != Some(true) => (
                    Some(ShutdownWaitCutoffReasonV1::ClockUnavailable),
                    None,
                    None,
                ),
                ShutdownWaitEventV1::Poll => return Ok(control.status),
                ShutdownWaitEventV1::Tick(clock) => {
                    if clock.epoch != request.clock.epoch || clock.tick < control.last.tick {
                        (
                            Some(ShutdownWaitCutoffReasonV1::ClockUnavailable),
                            None,
                            None,
                        )
                    } else {
                        let remaining = request
                            .budget_ticks
                            .saturating_sub(clock.tick - request.clock.tick);
                        (
                            (remaining == 0).then_some(ShutdownWaitCutoffReasonV1::BudgetEnded),
                            Some(remaining),
                            Some(clock),
                        )
                    }
                }
            };
            // Protected capture failure does not extend the wait or discard the
            // actual obligations. Store the exact failure, never an empty report.
            let cutoff = reason.map(|_| {
                self.capture_shutdown_progress_v1(fence, request.report_limits, facts.inventory, r)
            });
            let status = if let Some(reason) = reason {
                let report = match cutoff.as_ref().ok_or(CoordinatorError::Faulted)? {
                    Ok(_) => ShutdownCutoffReportStateV1::Available,
                    Err(error) => ShutdownCutoffReportStateV1::Unavailable(*error),
                };
                ShutdownWaitStatusV1::Cutoff {
                    reason,
                    remaining_ticks: remaining,
                    report,
                }
            } else {
                ShutdownWaitStatusV1::Waiting {
                    remaining_ticks: remaining.ok_or(CoordinatorError::Faulted)?,
                }
            };
            let State::Active(control) = &mut self
                .shutdown
                .fence
                .as_mut()
                .ok_or(CoordinatorError::Faulted)?
                .wait
            else {
                return Err(CoordinatorError::Faulted);
            };
            if let Some(clock) = next_clock {
                control.last = clock;
            }
            control.status = status;
            control.cutoff = cutoff;
            Ok(status)
        })();
        if result == Err(CoordinatorError::Faulted) {
            self.faulted = true;
        }
        observe_service(r, Boundary::ShutdownWaitAdvance, &result, timer.finish());
        result
    }
    /// Separately authorized release of the original cutoff, never a refresh.
    pub fn read_shutdown_wait_cutoff_v1<'a>(
        &'a self,
        fence: &ShutdownFenceIdV1,
        facts: &TrustedShutdownInventoryFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ShutdownInventoryReadV1<'a>, CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let control = self.shutdown_wait_control(fence, facts.control)?;
            let installed = self
                .shutdown
                .fence
                .as_ref()
                .ok_or(CoordinatorError::Unavailable)?;
            self.shutdown_inventory_roles(&installed.basis, facts)?;
            match control
                .cutoff
                .as_ref()
                .ok_or(CoordinatorError::Unavailable)?
            {
                Ok(report) => self.read_shutdown_progress_v1(report, facts, r),
                Err(error) => Err(*error),
            }
        })();
        observe_service(
            r,
            Boundary::ShutdownWaitCutoffRelease,
            &result,
            timer.finish(),
        );
        result
    }
    /// Release only wait/report retention, once, after current explicit discharge.
    /// The original fence, producers and their reservations remain in place.
    pub fn retire_shutdown_wait_v1(
        &mut self,
        fence: &ShutdownFenceIdV1,
        facts: &TrustedShutdownFenceFactsV1<'_>,
        retention_released: Option<bool>,
        delivery_drained: Option<bool>,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let control = self.shutdown_wait_control(fence, facts)?;
            if retention_released != Some(true)
                || delivery_drained != Some(true)
                || !matches!(control.status, ShutdownWaitStatusV1::Cutoff { .. })
            {
                return Err(CoordinatorError::Unavailable);
            }
            let State::Active(control) = &mut self
                .shutdown
                .fence
                .as_mut()
                .ok_or(CoordinatorError::Faulted)?
                .wait
            else {
                return Err(CoordinatorError::Faulted);
            };
            self.retention
                .release(&mut control.reservation)
                .map_err(wait_retention_error)?;
            self.shutdown
                .fence
                .as_mut()
                .ok_or(CoordinatorError::Faulted)?
                .wait = State::Retired;
            Ok(())
        })();
        if result == Err(CoordinatorError::Faulted) {
            self.faulted = true;
        }
        observe_service(r, Boundary::ShutdownWaitRetirement, &result, timer.finish());
        result
    }
}

fn wait_retention_error(error: RetentionErrorV1) -> CoordinatorError {
    match error {
        RetentionErrorV1::Unavailable => CoordinatorError::Unavailable,
        RetentionErrorV1::Limit => CoordinatorError::Limit,
        RetentionErrorV1::Faulted => CoordinatorError::Faulted,
    }
}
