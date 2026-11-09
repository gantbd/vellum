//! Optional process-local cohort ages. Never a queue, deadline or effect ledger.
use super::*;
use crate::instrumentation::{NativeTimingProfileV1, process_instant};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InvocationMeasurementLimitsV2 {
    /// Bounds the existing counter series, not a clock deadline or duration SLO.
    pub values: m::ValueLimitsV1,
    pub max_pool_bytes: usize,
    pub timing: NativeTimingProfileV1,
}
impl InvocationMeasurementLimitsV2 {
    pub fn required_pool_bytes(n: usize, timing: NativeTimingProfileV1) -> Result<usize, Error> {
        InvocationMeasurementLimitsV1::required_pool_bytes(n)?
            .checked_add(Timing::additional_bytes(n, Some(timing))?)
            .ok_or(Error::Limit)
    }
}

/// Trusted-owner aggregate only: no ticket, document, raw stamp or recipient grant.
/// Counts are native bounded cardinalities, not new engine-measurement/1 series.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InvocationResponsivenessSnapshotV1 {
    pub admissions: usize,
    pub pending: usize,
    pub pending_ages_unknown: usize,
    /// Not the true oldest if pending_ages_unknown is nonzero.
    pub oldest_known_pending_age: Option<Duration>,
    pub terminal: usize,
    pub terminal_durations_unknown: usize,
    /// Admission-hook to local terminal-hook, including host delay and service.
    /// Not queue-only, result delivery, persistence or provider quiescence.
    pub max_known_terminal_duration: Option<Duration>,
    pub interrupted: usize,
    pub unclassified: usize,
    /// Admission membership completeness; unknown duration counts are separate.
    pub population_complete: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationResponsivenessV1 {
    Off,
    Observed(InvocationResponsivenessSnapshotV1),
}

#[derive(Clone, Copy)]
enum Entry {
    Outside,
    Pending(Option<Instant>),
    Terminal(Option<Duration>),
}
pub(super) struct AgePool {
    entries: Vec<Entry>,
    // Prepaid membership, including retired terminal members. Never remove or
    // rebuild from live slots: disappearance is not evidence of nonmembership.
    admitted: Vec<usize>,
    lost: bool,
}
pub(super) enum Timing {
    Legacy,
    Off,
    Active(Box<AgePool>),
}
impl Timing {
    pub(super) fn additional_bytes(
        n: usize,
        timing: Option<NativeTimingProfileV1>,
    ) -> Result<usize, Error> {
        let output = std::mem::size_of::<InvocationResponsivenessV1>();
        match timing {
            None => Ok(0),
            Some(NativeTimingProfileV1::Off) => Ok(output),
            Some(NativeTimingProfileV1::ProcessMonotonicV1) => n
                .checked_mul(std::mem::size_of::<Entry>() + std::mem::size_of::<usize>())
                .and_then(|v| v.checked_add(std::mem::size_of::<AgePool>()))
                .and_then(|v| v.checked_add(output))
                .ok_or(Error::Limit),
        }
    }
    pub(super) fn new(n: usize, profile: Option<NativeTimingProfileV1>) -> Result<Self, Error> {
        Ok(match profile {
            None => Self::Legacy,
            Some(NativeTimingProfileV1::Off) => Self::Off,
            Some(NativeTimingProfileV1::ProcessMonotonicV1) => {
                let mut entries = Vec::new();
                entries.try_reserve_exact(n).map_err(|_| Error::Capacity)?;
                entries.resize(n, Entry::Outside);
                let mut admitted = Vec::new();
                admitted.try_reserve_exact(n).map_err(|_| Error::Capacity)?;
                Self::Active(Box::new(AgePool {
                    entries,
                    admitted,
                    lost: false,
                }))
            }
        })
    }
    pub(super) fn profile(&self) -> Option<NativeTimingProfileV1> {
        match self {
            Self::Legacy => None,
            Self::Off => Some(NativeTimingProfileV1::Off),
            Self::Active(_) => Some(NativeTimingProfileV1::ProcessMonotonicV1),
        }
    }
    pub(super) fn admission(&mut self, index: usize) {
        if let Self::Active(p) = self {
            p.admission_at(index, process_instant());
        }
    }
    pub(super) fn terminal(&mut self, index: usize) {
        if let Self::Active(p) = self {
            p.terminal_at(index, process_instant());
        }
    }
    pub(super) fn snapshot(&self, slots: &[Option<Slot>]) -> Option<InvocationResponsivenessV1> {
        match self {
            Self::Legacy => None,
            Self::Off => Some(InvocationResponsivenessV1::Off),
            Self::Active(p) => Some(InvocationResponsivenessV1::Observed(
                p.snapshot_at(slots, process_instant()),
            )),
        }
    }
}
impl AgePool {
    pub(super) fn admission_at(&mut self, index: usize, now: Option<Instant>) {
        match self.entries.get_mut(index) {
            Some(entry @ Entry::Outside) => {
                // At most one index per entry; capacity was reserved at setup.
                self.admitted.push(index);
                *entry = Entry::Pending(now);
            }
            _ => self.lost = true, // Observer loss must not select engine results.
        }
    }
    pub(super) fn terminal_at(&mut self, index: usize, now: Option<Instant>) {
        match self.entries.get_mut(index) {
            Some(entry @ Entry::Pending(_)) => {
                let Entry::Pending(start) = *entry else {
                    return;
                };
                *entry = Entry::Terminal(
                    start
                        .zip(now)
                        .and_then(|(a, b)| b.checked_duration_since(a)),
                );
            }
            _ => self.lost = true,
        }
    }
    pub(super) fn snapshot_at(
        &self,
        slots: &[Option<Slot>],
        now: Option<Instant>,
    ) -> InvocationResponsivenessSnapshotV1 {
        let mut out = InvocationResponsivenessSnapshotV1 {
            population_complete: !self.lost,
            ..Default::default()
        };
        for &index in &self.admitted {
            out.admissions += 1;
            let Some(entry) = self.entries.get(index) else {
                out.unclassified += 1;
                out.population_complete = false;
                continue;
            };
            let phase = slots.get(index).and_then(Option::as_ref).map(|s| &s.phase);
            match entry {
                Entry::Outside => {
                    out.unclassified += 1;
                    out.population_complete = false;
                }
                Entry::Pending(start) => match phase {
                    Some(Phase::Running) => out.interrupted += 1,
                    Some(p) if p.state() == Ok(InvocationState::Pending) => {
                        out.pending += 1;
                        let age = start
                            .zip(now)
                            .and_then(|(a, b)| b.checked_duration_since(a));
                        if let Some(age) = age {
                            out.oldest_known_pending_age =
                                Some(out.oldest_known_pending_age.map_or(age, |old| old.max(age)));
                        } else {
                            out.pending_ages_unknown += 1;
                        }
                    }
                    _ => {
                        out.unclassified += 1;
                        out.population_complete = false;
                    }
                },
                Entry::Terminal(duration) => {
                    if phase.is_some_and(|p| !matches!(p, Phase::Terminal(_))) {
                        out.unclassified += 1;
                        out.population_complete = false;
                    } else {
                        out.terminal += 1;
                        if let Some(duration) = duration {
                            out.max_known_terminal_duration = Some(
                                out.max_known_terminal_duration
                                    .map_or(*duration, |old| old.max(*duration)),
                            );
                        } else {
                            out.terminal_durations_unknown += 1;
                        }
                    }
                }
            }
        }
        out
    }
}

impl SourceCoordinator {
    pub fn configure_invocation_measurements_v2(
        &mut self,
        profile: InvocationMeasurementLimitsV2,
        allowed: Option<bool>,
        r: &mut Recorder,
    ) -> Result<bool, Error> {
        self.configure_invocation_measurements(
            InvocationMeasurementLimitsV1 {
                values: profile.values,
                max_pool_bytes: profile.max_pool_bytes,
            },
            Some(profile.timing),
            allowed,
            r,
        )
    }
    /// Safe bounded owner-ledger observation, independent of the diagnostic queue.
    /// Collection starts only at activation; old admitted work is excluded.
    /// A retained Running slot is unknown/interrupted, never a known completion.
    pub fn invocation_responsiveness_v1(
        &mut self,
        r: &mut Recorder,
    ) -> Result<Option<InvocationResponsivenessV1>, Error> {
        let out = (|| {
            let p = match &self.invocation_measurements {
                State::Unconfigured => return Ok(None),
                State::Closed => return Err(Error::Unavailable),
                State::Active(p) => p,
            };
            self.retention.validate(&p.reservation).map_err(parent)?;
            if p.stopped && p.timing.profile().is_some() {
                return Ok(Some(InvocationResponsivenessV1::Off));
            }
            Ok(p.timing.snapshot(&self.invocations.slots))
        })();
        self.finish_invocation_measurement(Boundary::InvocationResponsivenessSnapshot, out, r)
    }
}

#[cfg(test)]
mod index_tests {
    use super::*;
    #[test]
    fn timing_membership_is_prepaid_sparse_and_never_forgotten_or_duplicated() {
        let mut timing =
            Timing::new(100_000, Some(NativeTimingProfileV1::ProcessMonotonicV1)).unwrap();
        let Timing::Active(pool) = &mut timing else {
            panic!()
        };
        let capacity = pool.admitted.capacity();
        assert!(capacity >= 100_000);
        assert!(pool.admitted.is_empty());
        assert!(pool.snapshot_at(&[], None).population_complete);
        let start = Instant::now();
        let end = start.checked_add(Duration::from_millis(3)).unwrap();
        pool.admission_at(99_999, Some(start));
        pool.terminal_at(99_999, Some(end));
        pool.admission_at(3, None);
        assert_eq!(pool.admitted, [99_999, 3]);
        let snapshot = pool.snapshot_at(&[], Some(end));
        assert_eq!(
            (
                snapshot.admissions,
                snapshot.terminal,
                snapshot.unclassified
            ),
            (2, 1, 1)
        );
        assert_eq!(
            snapshot.max_known_terminal_duration,
            Some(Duration::from_millis(3))
        );
        assert!(!snapshot.population_complete);
        pool.admission_at(3, Some(start));
        pool.admission_at(usize::MAX, Some(start));
        assert_eq!(pool.admitted, [99_999, 3]);
        assert!(pool.lost);
        assert_eq!(pool.admitted.capacity(), capacity);
        assert_eq!(pool.entries.len(), 100_000);
    }
    #[test]
    fn every_possible_member_fits_without_record_time_growth_and_index_faults_are_incomplete() {
        let mut timing = Timing::new(64, Some(NativeTimingProfileV1::ProcessMonotonicV1)).unwrap();
        let Timing::Active(pool) = &mut timing else {
            panic!()
        };
        let capacity = pool.admitted.capacity();
        for i in (0..64).rev() {
            pool.admission_at(i, None);
            pool.terminal_at(i, None);
        }
        assert_eq!(pool.admitted.capacity(), capacity);
        let snapshot = pool.snapshot_at(&[], None);
        assert_eq!(
            (
                snapshot.admissions,
                snapshot.terminal,
                snapshot.terminal_durations_unknown
            ),
            (64, 64, 64)
        );
        assert!(snapshot.population_complete);
        // A broken internal index is not a valid empty/complete population.
        pool.admitted[0] = usize::MAX;
        let invalid = pool.snapshot_at(&[], None);
        assert!(!invalid.population_complete);
        assert_eq!(
            invalid.admissions,
            invalid.pending + invalid.terminal + invalid.interrupted + invalid.unclassified
        );
        pool.admitted[0] = 0;
        pool.entries[0] = Entry::Outside;
        assert!(!pool.snapshot_at(&[], None).population_complete);
    }
}
