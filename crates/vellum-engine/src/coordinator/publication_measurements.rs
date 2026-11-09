//! Actual source/annotation transaction publisher population, not all engine
//! publications or diagnostic-record traffic. No timing is inferred.
use super::*;
mod shutdown_inventory;
use crate::instrumentation::measurement::{self as m, MeasurementErrorV1 as Error};
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(test)]
mod tests;
const INEXACT: u64 = 1 << 63;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PublicationMeasurementLimitsV1 {
    pub values: m::ValueLimitsV1,
    pub max_pool_bytes: usize,
}
impl PublicationMeasurementLimitsV1 {
    /// Native pool, six counters and one complete six-point reply frame.
    /// Not allocator/Arc control blocks, RSS or caller-retained copies.
    pub fn required_pool_bytes() -> usize {
        std::mem::size_of::<Pool>()
            + std::mem::size_of::<Counts>()
            + std::mem::size_of::<[m::MeasurementPointV1; 6]>()
    }
}
pub struct PublicationMeasurementControlV1 {
    pub owner_control_allowed: Option<bool>,
    pub delivery_drained: Option<bool>,
    pub retention_released: Option<bool>,
}
struct Counts {
    values: [AtomicU64; 6],
    maximum: u64,
}
impl Counts {
    fn increment(&self, index: usize) {
        // The sole public owner is exclusively borrowed. Private unwind probes
        // cannot escape that call. Atomic storage preserves Send/Sync without
        // callbacks, a mutex, a CAS loop or authority-bearing shared mutation.
        let old = self.values[index].load(Ordering::Relaxed);
        let value = old & !INEXACT;
        let next = if value == self.maximum {
            self.maximum | INEXACT
        } else {
            (value + 1) | (old & INEXACT)
        };
        self.values[index].store(next, Ordering::Relaxed);
    }
    fn read(&self) -> [m::CounterV1; 6] {
        self.values.each_ref().map(|v| {
            let n = v.load(Ordering::Relaxed);
            m::CounterV1 {
                total: n & !INEXACT,
                exact: n & INEXACT == 0,
            }
        })
    }
}
pub(super) struct Pool {
    profile: PublicationMeasurementLimitsV1,
    series: [m::MeasurementSeriesV1; 6],
    counts: Arc<Counts>,
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
        f.write_str("PublicationMeasurements(<owned>)")
    }
}
pub(super) struct Probe {
    counts: Option<Arc<Counts>>,
    finished: bool,
}
impl State {
    pub(super) fn probe(&self) -> Probe {
        Probe {
            counts: match self {
                Self::Active(p) if !p.stopped => Some(p.counts.clone()),
                _ => None,
            },
            finished: false,
        }
    }
}
impl Probe {
    pub(super) fn finish(mut self, result: &Result<CommitOutcome, CoordinatorError>) {
        let disposition = match result {
            Ok(o) if o.operation.is_some() => m::DispositionV1::Success,
            Ok(_) => m::DispositionV1::Unchanged,
            Err(CoordinatorError::Faulted | CoordinatorError::History(HistoryError::Faulted)) => {
                m::DispositionV1::Fault
            }
            Err(_) => m::DispositionV1::Refused,
        };
        if let Some(c) = &self.counts {
            c.increment(index(disposition));
        }
        self.finished = true;
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        if !self.finished
            && let Some(c) = &self.counts
        {
            c.increment(index(m::DispositionV1::Fault));
        }
    }
}
fn index(d: m::DispositionV1) -> usize {
    match d {
        m::DispositionV1::Success => 0,
        m::DispositionV1::Unchanged => 1,
        m::DispositionV1::Refused => 2,
        m::DispositionV1::Stopped => 3,
        m::DispositionV1::Unknown => 4,
        m::DispositionV1::Fault => 5,
    }
}
fn parent(e: RetentionErrorV1) -> Error {
    match e {
        RetentionErrorV1::Faulted => Error::Faulted,
        RetentionErrorV1::Limit => Error::Limit,
        RetentionErrorV1::Unavailable => Error::Unavailable,
    }
}
fn observe<T>(r: &mut Recorder, b: Boundary, out: &Result<T, Error>) {
    r.record(Observation {
        boundary: b,
        disposition: match out {
            Ok(_) => Disposition::Completed,
            Err(Error::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time: None,
    });
}
impl SourceCoordinator {
    fn finish_publication_measurement<T>(
        &mut self,
        b: Boundary,
        out: Result<T, Error>,
        r: &mut Recorder,
    ) -> Result<T, Error> {
        // Only actual parent-accounting corruption fences the authoritative
        // owner. Ordinary optional profile/snapshot refusal never does.
        if matches!(out, Err(Error::Faulted)) {
            self.faulted = true;
        }
        observe(r, b, &out);
        out
    }
    pub fn configure_publication_measurements_v1(
        &mut self,
        profile: PublicationMeasurementLimitsV1,
        owner_control_allowed: Option<bool>,
        r: &mut Recorder,
    ) -> Result<bool, Error> {
        let out = (|| {
            if self.faulted || owner_control_allowed != Some(true) {
                return Err(Error::Unavailable);
            }
            profile.values.validate()?;
            match &self.publication_measurements {
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
            let bytes = PublicationMeasurementLimitsV1::required_pool_bytes();
            if bytes > profile.max_pool_bytes {
                return Err(Error::Limit);
            }
            self.retention.check(bytes).map_err(parent)?;
            let series = |d| {
                m::MeasurementSeriesV1::new(
                    m::MeasureV1::PublicationDispositions,
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
            let counts = Arc::new(Counts {
                values: std::array::from_fn(|_| AtomicU64::new(0)),
                maximum: profile.values.max_integer,
            });
            let reservation = self.retention.reserve(bytes).map_err(parent)?;
            self.publication_measurements = State::Active(Box::new(Pool {
                profile,
                series,
                counts,
                reservation,
                stopped: false,
            }));
            Ok(true)
        })();
        self.finish_publication_measurement(Boundary::PublicationMeasurementConfiguration, out, r)
    }
    /// Privileged native publisher cut. No recipient grant or whole-engine
    /// population claim follows. Available under the native fault fence.
    pub fn publication_measurements_v1(
        &mut self,
        r: &mut Recorder,
    ) -> Result<Option<[m::MeasurementPointV1; 6]>, Error> {
        let out = (|| {
            let p = match &mut self.publication_measurements {
                State::Active(p) => p,
                State::Unconfigured => return Ok(None),
                State::Closed => return Err(Error::Unavailable),
            };
            self.retention.validate(&p.reservation).map_err(parent)?;
            let counters = p.counts.read();
            if counters
                .iter()
                .any(|v| v.total > p.profile.values.max_integer)
            {
                return Err(Error::InvalidValue);
            }
            if p.series.iter().any(|s| !s.can_observe()) {
                return Err(Error::Exhausted);
            }
            let states = counters.map(|v| {
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
        self.finish_publication_measurement(Boundary::PublicationMeasurementSnapshot, out, r)
    }
    pub fn stop_publication_measurements_v1(
        &mut self,
        owner_control_allowed: Option<bool>,
        r: &mut Recorder,
    ) -> bool {
        let out = (|| {
            if owner_control_allowed != Some(true) {
                return Err(Error::Unavailable);
            }
            let State::Active(p) = &mut self.publication_measurements else {
                return Err(Error::Unavailable);
            };
            p.stopped = true;
            Ok(())
        })();
        self.finish_publication_measurement(Boundary::PublicationMeasurementStop, out, r)
            .is_ok()
    }
    pub fn retire_publication_measurements_v1(
        &mut self,
        f: &PublicationMeasurementControlV1,
        r: &mut Recorder,
    ) -> bool {
        let out = (|| {
            if [
                f.owner_control_allowed,
                f.delivery_drained,
                f.retention_released,
            ]
            .into_iter()
            .any(|v| v != Some(true))
            {
                return Err(Error::Unavailable);
            }
            let State::Active(p) = &mut self.publication_measurements else {
                return Err(Error::Unavailable);
            };
            if !p.stopped || Arc::strong_count(&p.counts) != 1 {
                return Err(Error::Unavailable);
            }
            self.retention.release(&mut p.reservation).map_err(parent)?;
            self.publication_measurements = State::Closed;
            Ok(())
        })();
        self.finish_publication_measurement(Boundary::PublicationMeasurementRetirement, out, r)
            .is_ok()
    }
}
