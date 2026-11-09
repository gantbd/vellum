//! Exact native admission cohorts and local disposition counts. Not ingress
//! offers, external effects, provider quiescence or recipient-safe telemetry.
use super::*;
mod shutdown_inventory;
use crate::instrumentation::measurement::{self as m, MeasurementErrorV1 as Error};
mod pending_age;
#[cfg(test)]
mod tests;
use pending_age::Timing;
pub use pending_age::{
    InvocationMeasurementLimitsV2, InvocationResponsivenessSnapshotV1, InvocationResponsivenessV1,
};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InvocationMeasurementLimitsV1 {
    pub values: m::ValueLimitsV1,
    pub max_pool_bytes: usize,
}
impl InvocationMeasurementLimitsV1 {
    pub fn required_pool_bytes(maximum_issued: usize) -> Result<usize, Error> {
        std::mem::size_of::<Pool>()
            .checked_add(std::mem::size_of::<InvocationMeasurementsV1>())
            .and_then(|n| n.checked_add(maximum_issued.checked_mul(std::mem::size_of::<bool>())?))
            .ok_or(Error::Limit)
    }
}
pub struct InvocationMeasurementControlV1 {
    pub owner_control_allowed: Option<bool>,
    pub delivery_drained: Option<bool>,
    pub retention_released: Option<bool>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct InvocationMeasurementsV1 {
    pub admissions: m::MeasurementPointV1,
    pub dispositions: [m::MeasurementPointV1; 6],
}
pub(in crate::coordinator) struct Pool {
    profile: InvocationMeasurementLimitsV1,
    series: [m::MeasurementSeriesV1; 7],
    counts: [m::CounterV1; 7],
    reservation: retention::Reservation,
    stopped: bool,
    timing: Timing,
}
#[derive(Default)]
pub(in crate::coordinator) enum State {
    #[default]
    Unconfigured,
    Active(Box<Pool>),
    Closed,
}
impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationMeasurements(<owned>)")
    }
}
impl State {
    fn collecting(&self) -> bool {
        matches!(self,Self::Active(p) if !p.stopped)
    }
    fn increment(&mut self, index: usize) {
        if let Self::Active(p) = self
            && !p.stopped
        {
            let v = p.counts[index];
            let max = p.profile.values.max_integer;
            p.counts[index] = if v.total == max {
                m::CounterV1 {
                    total: max,
                    exact: false,
                }
            } else {
                m::CounterV1 {
                    total: v.total + 1,
                    exact: v.exact,
                }
            };
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
fn classified(out: &InvocationOutcome) -> m::DispositionV1 {
    use super::observation::projection::{TerminalKind, classify};
    match classify(out) {
        TerminalKind::Applied => m::DispositionV1::Success,
        TerminalKind::Unchanged => m::DispositionV1::Unchanged,
        TerminalKind::Rejected => m::DispositionV1::Refused,
        TerminalKind::Stopped => m::DispositionV1::Stopped,
        TerminalKind::Unknown => m::DispositionV1::Fault,
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
    /// One internal admission transition, independent of observer availability.
    pub(super) fn install_invocation_admission(
        &mut self,
        id: &InvocationId,
        intent: BoundIntent,
        phase: Phase,
    ) {
        let valid = self.invocation_slot(id).is_ok_and(|s| {
            s.intent.is_none() && matches!(s.phase, Phase::Issued(_) | Phase::Running)
        });
        if !valid {
            self.faulted = true;
            panic!("native invocation admission invariant");
        }
        let cohort = self.invocation_measurements.collecting();
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated native admission");
        slot.intent = Some(intent);
        slot.phase = phase;
        slot.measurement_cohort = cohort;
        if cohort {
            self.invocation_measurements.increment(6);
            if let State::Active(p) = &mut self.invocation_measurements {
                p.timing.admission(id.slot);
            }
        }
    }
    /// Forgetting a terminal result is not another terminal transition.
    pub(in crate::coordinator) fn install_invocation_terminal(
        &mut self,
        id: &InvocationId,
        outcome: InvocationOutcome,
    ) {
        let valid = self.invocation_slot(id).is_ok_and(|s| {
            s.intent.is_some() && !matches!(s.phase, Phase::Issued(_) | Phase::Terminal(_))
        });
        if !valid {
            self.faulted = true;
            panic!("native invocation terminal invariant");
        }
        let disposition = classified(&outcome);
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated native terminal");
        let cohort = slot.measurement_cohort;
        if let Some(BoundIntent::Annotation(capture)) = &slot.intent
            && matches!(
                outcome,
                InvocationOutcome::StoppedBeforePublication | InvocationOutcome::Rejected(_)
            )
        {
            slot.annotation_hold
                .as_mut()
                .expect("funded annotation terminal")
                .receipt = Some(super::annotation_scoped::receipt::Receipt::not_published(
                id,
                &capture.mutation,
            ));
        }
        slot.phase = Phase::Terminal(Some(outcome));
        // Original weak roles and fixed receipt survive; the charged pending
        // restoration proof is no longer an engine-owned producer/snapshot pin.
        if let Some(hold) = &mut slot.annotation_load_hold {
            hold.pending = None;
        }
        if cohort {
            self.invocation_measurements.increment(index(disposition));
            if let State::Active(p) = &mut self.invocation_measurements
                && !p.stopped
            {
                p.timing.terminal(id.slot);
            }
        }
    }
    fn finish_invocation_measurement<T>(
        &mut self,
        b: Boundary,
        out: Result<T, Error>,
        r: &mut Recorder,
    ) -> Result<T, Error> {
        if matches!(out, Err(Error::Faulted)) {
            self.faulted = true;
        }
        r.record(Observation {
            boundary: b,
            disposition: match &out {
                Ok(_) => Disposition::Completed,
                Err(Error::Faulted) => Disposition::Unknown,
                Err(_) => Disposition::Refused,
            },
            service_time: None,
        });
        out
    }
    /// Collection begins at activation. Old admitted work is outside this
    /// cohort; old issued-but-unbound tickets may still be first-admitted into it.
    pub fn configure_invocation_measurements_v1(
        &mut self,
        profile: InvocationMeasurementLimitsV1,
        owner_control_allowed: Option<bool>,
        r: &mut Recorder,
    ) -> Result<bool, Error> {
        self.configure_invocation_measurements(profile, None, owner_control_allowed, r)
    }
    fn configure_invocation_measurements(
        &mut self,
        profile: InvocationMeasurementLimitsV1,
        timing: Option<crate::instrumentation::NativeTimingProfileV1>,
        owner_control_allowed: Option<bool>,
        r: &mut Recorder,
    ) -> Result<bool, Error> {
        let out = (|| {
            if self.faulted || owner_control_allowed != Some(true) {
                return Err(Error::Unavailable);
            }
            profile.values.validate()?;
            match &self.invocation_measurements {
                State::Active(p) => {
                    self.retention.validate(&p.reservation).map_err(parent)?;
                    return if p.profile == profile && p.timing.profile() == timing {
                        Ok(false)
                    } else {
                        Err(Error::Unavailable)
                    };
                }
                State::Closed => return Err(Error::Unavailable),
                State::Unconfigured => {}
            }
            let bytes = InvocationMeasurementLimitsV1::required_pool_bytes(
                self.invocations.limits.max_lifetime_issued,
            )?
            .checked_add(Timing::additional_bytes(
                self.invocations.limits.max_lifetime_issued,
                timing,
            )?)
            .ok_or(Error::Limit)?;
            if bytes > profile.max_pool_bytes {
                return Err(Error::Limit);
            }
            self.retention.check(bytes).map_err(parent)?;
            let series = |d| {
                m::MeasurementSeriesV1::new(
                    m::MeasureV1::InvocationDispositions,
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
                m::MeasurementSeriesV1::new(
                    m::MeasureV1::InvocationAdmissions,
                    m::DimensionsV1::None,
                    profile.values,
                )?,
            ];
            let timing = Timing::new(self.invocations.limits.max_lifetime_issued, timing)?;
            let reservation = self.retention.reserve(bytes).map_err(parent)?;
            self.invocation_measurements = State::Active(Box::new(Pool {
                profile,
                series,
                counts: [m::CounterV1 {
                    total: 0,
                    exact: true,
                }; 7],
                reservation,
                stopped: false,
                timing,
            }));
            Ok(true)
        })();
        self.finish_invocation_measurement(Boundary::InvocationMeasurementConfiguration, out, r)
    }
    /// Trusted native ledger cohort, not engine-wide requests, a denominator for
    /// another population, current recipient permission or quiescence evidence.
    pub fn invocation_measurements_v1(
        &mut self,
        r: &mut Recorder,
    ) -> Result<Option<InvocationMeasurementsV1>, Error> {
        // Running is only transient inside an exclusive service call. Outside
        // that call it is retained execution uncertainty. AnnotationFaulted also
        // preserves uncertainty after protected pre-service payload discharge;
        // it does not assert physical execution. Neither state can be forgotten,
        // retired or reexecuted by the public lifecycle. This census is cumulative,
        // not a guess from a global fault flag or log volume.
        let faulted = self
            .invocations
            .slots
            .iter()
            .flatten()
            .filter(|s| {
                s.measurement_cohort
                    && s.intent.is_some()
                    && matches!(s.phase, Phase::Running | Phase::AnnotationFaulted)
            })
            .count();
        let out = (|| {
            let p = match &mut self.invocation_measurements {
                State::Active(p) => p,
                State::Unconfigured => return Ok(None),
                State::Closed => return Err(Error::Unavailable),
            };
            self.retention.validate(&p.reservation).map_err(parent)?;
            let mut counters = p.counts;
            let max = p.profile.values.max_integer;
            if counters.iter().any(|v| v.total > max) {
                return Err(Error::InvalidValue);
            }
            let extra = u64::try_from(faulted).unwrap_or(u64::MAX);
            let old = counters[5];
            counters[5] = if extra > max - old.total {
                m::CounterV1 {
                    total: max,
                    exact: false,
                }
            } else {
                m::CounterV1 {
                    total: old.total + extra,
                    exact: old.exact,
                }
            };
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
            let dispositions = [
                p.series[0].observe(states[0])?,
                p.series[1].observe(states[1])?,
                p.series[2].observe(states[2])?,
                p.series[3].observe(states[3])?,
                p.series[4].observe(states[4])?,
                p.series[5].observe(states[5])?,
            ];
            let admissions = p.series[6].observe(states[6])?;
            Ok(Some(InvocationMeasurementsV1 {
                admissions,
                dispositions,
            }))
        })();
        self.finish_invocation_measurement(Boundary::InvocationMeasurementSnapshot, out, r)
    }
    pub fn stop_invocation_measurements_v1(
        &mut self,
        owner_control_allowed: Option<bool>,
        r: &mut Recorder,
    ) -> bool {
        let out = (|| {
            if owner_control_allowed != Some(true) {
                return Err(Error::Unavailable);
            }
            let State::Active(p) = &mut self.invocation_measurements else {
                return Err(Error::Unavailable);
            };
            p.stopped = true;
            Ok(())
        })();
        self.finish_invocation_measurement(Boundary::InvocationMeasurementStop, out, r)
            .is_ok()
    }
    pub fn retire_invocation_measurements_v1(
        &mut self,
        f: &InvocationMeasurementControlV1,
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
            let State::Active(p) = &mut self.invocation_measurements else {
                return Err(Error::Unavailable);
            };
            if !p.stopped {
                return Err(Error::Unavailable);
            }
            self.retention.release(&mut p.reservation).map_err(parent)?;
            // No fallible step or callback follows discharge. Remove this
            // observer's membership metadata, never the original jobs/results.
            for slot in self.invocations.slots.iter_mut().flatten() {
                slot.measurement_cohort = false;
            }
            self.invocation_measurements = State::Closed;
            Ok(())
        })();
        self.finish_invocation_measurement(Boundary::InvocationMeasurementRetirement, out, r)
            .is_ok()
    }
}
