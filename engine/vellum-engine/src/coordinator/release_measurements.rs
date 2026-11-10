//! Prospective native read-facade handoff attempts, not all engine reads,
//! invocation dispositions, recipient telemetry or diagnostic-record traffic.
use super::*;
mod shutdown_inventory;
use crate::instrumentation::measurement::{self as m, MeasurementErrorV1 as Error};
#[cfg(test)]
pub(super) mod tests;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReleaseMeasurementLimitsV1 {
    pub values: m::ValueLimitsV1,
    pub max_pool_bytes: usize,
}
impl ReleaseMeasurementLimitsV1 {
    /// Native inline pool and one complete six-point snapshot frame. Not
    /// allocator/Arc control blocks, RSS or caller-retained returned points.
    pub fn required_pool_bytes() -> usize {
        std::mem::size_of::<Pool>() + std::mem::size_of::<[m::MeasurementPointV1; 6]>()
    }
}
pub struct ReleaseMeasurementControlV1 {
    pub owner_control_allowed: Option<bool>,
    pub delivery_drained: Option<bool>,
    pub retention_released: Option<bool>,
}
pub(super) struct Pool {
    profile: ReleaseMeasurementLimitsV1,
    series: [m::MeasurementSeriesV1; 6],
    counts: [m::CounterV1; 6],
    reservation: retention::Reservation,
    stopped: bool,
}
#[derive(Default)]
pub(super) enum State {
    #[default]
    Unconfigured,
    Active(Box<Pool>),
    Closed,
}
impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReleaseMeasurements(<owned>)")
    }
}
impl State {
    /// Called once after the real read gate's protected refund and health fence.
    /// No allocation, callback, clock, authority decision or fallible telemetry.
    pub(super) fn record<T>(&mut self, out: &Result<T, SourceReadContextErrorV1>) {
        let Self::Active(p) = self else {
            return;
        };
        if p.stopped {
            return;
        }
        // The native read profile reports only handoff success, refusal or
        // fault. An unknown authority premise is a refusal, not inferred stop,
        // loss, quiescence, publication or external-effect truth.
        let index = match out {
            Ok(_) => 0,
            Err(SourceReadContextErrorV1::Faulted) => 5,
            Err(_) => 2,
        };
        let value = p.counts[index];
        p.counts[index] = if value.total == p.profile.values.max_integer {
            m::CounterV1 {
                total: value.total,
                exact: false,
            }
        } else {
            m::CounterV1 {
                total: value.total + 1,
                exact: value.exact,
            }
        };
    }
}
fn parent(e: RetentionErrorV1) -> Error {
    match e {
        RetentionErrorV1::Faulted => Error::Faulted,
        RetentionErrorV1::Limit => Error::Limit,
        RetentionErrorV1::Unavailable => Error::Unavailable,
    }
}
impl SourceCoordinator {
    fn finish_release_measurement<T>(
        &mut self,
        boundary: Boundary,
        out: Result<T, Error>,
        r: &mut Recorder,
    ) -> Result<T, Error> {
        // Optional profile/snapshot refusals never settle or fault engine work.
        // Actual parent-accounting corruption still fences the owner.
        if matches!(out, Err(Error::Faulted)) {
            self.faulted = true;
        }
        r.record(Observation {
            boundary,
            disposition: match &out {
                Ok(_) => Disposition::Completed,
                Err(Error::Faulted) => Disposition::Unknown,
                Err(_) => Disposition::Refused,
            },
            service_time: None,
        });
        out
    }
    /// Starts a prospective native read-adapter cohort, not historical totals.
    pub fn configure_release_measurements_v1(
        &mut self,
        profile: ReleaseMeasurementLimitsV1,
        owner_control_allowed: Option<bool>,
        r: &mut Recorder,
    ) -> Result<bool, Error> {
        let out = (|| {
            if self.faulted || self.source_facade_busy || owner_control_allowed != Some(true) {
                return Err(Error::Unavailable);
            }
            profile.values.validate()?;
            match &self.release_measurements {
                State::Active(p) => {
                    self.retention.validate(&p.reservation).map_err(parent)?;
                    return if p.profile == profile {
                        Ok(false)
                    } else {
                        Err(Error::Unavailable)
                    };
                }
                State::Closed => return Err(Error::Unavailable),
                State::Unconfigured => {}
            }
            let bytes = ReleaseMeasurementLimitsV1::required_pool_bytes();
            if bytes > profile.max_pool_bytes {
                return Err(Error::Limit);
            }
            self.retention.check(bytes).map_err(parent)?;
            let series = |d| {
                m::MeasurementSeriesV1::new(
                    m::MeasureV1::ReleaseDispositions,
                    m::DimensionsV1::Disposition(d),
                    profile.values,
                )
            };
            let series = [
                series(m::DispositionV1::Success)?,
                series(m::DispositionV1::Unchanged)?,
                series(m::DispositionV1::Refused)?,
                series(m::DispositionV1::Stopped)?,
                series(m::DispositionV1::Unknown)?,
                series(m::DispositionV1::Fault)?,
            ];
            let reservation = self.retention.reserve(bytes).map_err(parent)?;
            self.release_measurements = State::Active(Box::new(Pool {
                profile,
                series,
                counts: [m::CounterV1 {
                    total: 0,
                    exact: true,
                }; 6],
                reservation,
                stopped: false,
            }));
            Ok(true)
        })();
        self.finish_release_measurement(Boundary::ReleaseMeasurementConfiguration, out, r)
    }
    /// Privileged local six-point cut, also available under the owner fault fence.
    /// Does not authorize recipient delivery or count its own snapshot as a release.
    pub fn release_measurements_v1(
        &mut self,
        r: &mut Recorder,
    ) -> Result<Option<[m::MeasurementPointV1; 6]>, Error> {
        let out = (|| {
            let p = match &mut self.release_measurements {
                State::Active(p) => p,
                State::Unconfigured => return Ok(None),
                State::Closed => return Err(Error::Unavailable),
            };
            self.retention.validate(&p.reservation).map_err(parent)?;
            if p.counts
                .iter()
                .any(|v| v.total > p.profile.values.max_integer)
            {
                return Err(Error::InvalidValue);
            }
            if p.series.iter().any(|s| !s.can_observe()) {
                return Err(Error::Exhausted);
            }
            let states = p.counts.map(|v| {
                if p.stopped {
                    m::PointStateV1::Off
                } else {
                    m::PointStateV1::Observed {
                        freshness: m::FreshnessV1::Current,
                        coverage: if v.exact {
                            m::CoverageV1::Complete
                        } else {
                            m::CoverageV1::Inexact
                        },
                        value: m::ValueV1::Counter(v),
                    }
                }
            });
            Ok(Some([
                p.series[0].observe(states[0])?,
                p.series[1].observe(states[1])?,
                p.series[2].observe(states[2])?,
                p.series[3].observe(states[3])?,
                p.series[4].observe(states[4])?,
                p.series[5].observe(states[5])?,
            ]))
        })();
        self.finish_release_measurement(Boundary::ReleaseMeasurementSnapshot, out, r)
    }
    pub fn stop_release_measurements_v1(
        &mut self,
        owner_control_allowed: Option<bool>,
        r: &mut Recorder,
    ) -> bool {
        let out = (|| {
            if owner_control_allowed != Some(true) || self.source_facade_busy {
                return Err(Error::Unavailable);
            }
            let State::Active(p) = &mut self.release_measurements else {
                return Err(Error::Unavailable);
            };
            p.stopped = true;
            Ok(())
        })();
        self.finish_release_measurement(Boundary::ReleaseMeasurementStop, out, r)
            .is_ok()
    }
    pub fn retire_release_measurements_v1(
        &mut self,
        control: &ReleaseMeasurementControlV1,
        r: &mut Recorder,
    ) -> bool {
        let out = (|| {
            if self.source_facade_busy
                || [
                    control.owner_control_allowed,
                    control.delivery_drained,
                    control.retention_released,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(Error::Unavailable);
            }
            let State::Active(p) = &mut self.release_measurements else {
                return Err(Error::Unavailable);
            };
            if !p.stopped {
                return Err(Error::Unavailable);
            }
            self.retention.release(&mut p.reservation).map_err(parent)?;
            self.release_measurements = State::Closed;
            Ok(())
        })();
        self.finish_release_measurement(Boundary::ReleaseMeasurementRetirement, out, r)
            .is_ok()
    }
}
