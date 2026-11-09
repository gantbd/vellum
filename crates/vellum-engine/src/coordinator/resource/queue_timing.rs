//! Optional original-owner queue ages. No scheduler, deadline, payload labels,
//! effect evidence or dynamic storage. Retired aggregates have capped counts.
use super::*;
use crate::instrumentation::{NativeTimingProfileV1, process_instant};
use std::time::{Duration, Instant};
mod result;
#[cfg(test)]
mod tests;
pub use result::ResourceResultAttemptTimingV1;
use result::ResultStamp;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceQueueKindV1 {
    SourceRead,
    AnnotationRead,
    SourceSave,
    AnnotationSave,
}
impl ResourceQueueKindV1 {
    pub fn index(self) -> usize {
        match self {
            Self::SourceRead => 0,
            Self::AnnotationRead => 1,
            Self::SourceSave => 2,
            Self::AnnotationSave => 3,
        }
    }
    fn artifact(kind: ArtifactKind) -> Self {
        match kind {
            ArtifactKind::Source => Self::SourceSave,
            ArtifactKind::Annotations => Self::AnnotationSave,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResourceQueuePopulationV1 {
    pub observed_admissions: usize,
    pub pending: usize,
    pub unknown_pending_ages: usize,
    /// Not the true oldest if any pending ages are unknown.
    pub oldest_known_pending_age: Option<Duration>,
    pub dispatched: usize,
    pub not_dispatched: usize,
    pub unknown_queue_durations: usize,
    pub max_known_queue_duration: Option<Duration>,
    pub retired_rows: usize,
    /// Native rows with missing/inconsistent observation membership, not zero work.
    pub untracked_rows: usize,
    /// Dispatched original tasks with no accepted attributed result yet.
    pub awaiting_first_result: usize,
    pub unknown_first_result_ages: usize,
    pub oldest_known_first_result_age: Option<Duration>,
    pub first_results: usize,
    pub unknown_first_result_durations: usize,
    pub max_known_first_result_duration: Option<Duration>,
    /// Discharged tasks without a result observation are censored, not fast results.
    pub retired_without_first_result: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceQueueSnapshotV1 {
    /// Indexed by ResourceQueueKindV1::index. No identities or recipient data.
    pub families: [ResourceQueuePopulationV1; 4],
    pub population_complete: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
// Four fixed native populations stay inline: boxing would allocate on a metrics
// read and introduce an unrelated failure path. This is not an unbounded payload.
#[allow(clippy::large_enum_variant)]
pub enum ResourceQueueResponsivenessV1 {
    Off,
    Observed(ResourceQueueSnapshotV1),
}
#[derive(Clone, Copy)]
pub enum ResourceQueueAttemptV1<'a> {
    SourceRead(&'a SourceReadIdV1),
    AnnotationRead(&'a AnnotationReadId),
    Save(&'a SaveId),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceQueuePhaseV1 {
    Off,
    Pending,
    Dispatched,
    NotDispatched,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceQueueAttemptTimingV1 {
    pub kind: ResourceQueueKindV1,
    /// Observation state, not an authoritative effect or quiescence status.
    pub phase: ResourceQueuePhaseV1,
    pub pending_age: Option<Duration>,
    pub queue_duration: Option<Duration>,
}
#[derive(Clone, Copy)]
pub(super) enum Stamp {
    Off,
    Pending(Option<Instant>),
    End {
        dispatched: bool,
        elapsed: Option<Duration>,
        result: ResultStamp,
    },
}
impl std::fmt::Debug for Stamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("QueueTiming(<private>)")
    }
}
impl Stamp {
    pub(super) fn admitted(profile: Option<NativeTimingProfileV1>) -> Self {
        match profile {
            Some(NativeTimingProfileV1::ProcessMonotonicV1) => Self::Pending(process_instant()),
            _ => Self::Off,
        }
    }
    pub(super) fn finish(&mut self, dispatched: bool) {
        if matches!(self, Self::Pending(_)) {
            self.finish_at(dispatched, process_instant());
        }
    }
    fn finish_at(&mut self, dispatched: bool, now: Option<Instant>) {
        if let Self::Pending(start) = *self {
            *self = Self::End {
                dispatched,
                elapsed: start
                    .zip(now)
                    .and_then(|(a, b)| b.checked_duration_since(a)),
                result: ResultStamp::new(now),
            };
        }
    }
}
#[derive(Debug, Default)]
pub(super) struct State {
    pub(super) profile: Option<NativeTimingProfileV1>,
    retired: [ResourceQueuePopulationV1; 4],
    inexact: bool,
}
fn increment(n: &mut usize, complete: &mut bool) {
    if let Some(next) = n.checked_add(1) {
        *n = next;
    } else {
        *complete = false;
    }
}
fn maximum(v: &mut Option<Duration>, value: Duration) {
    *v = Some(v.map_or(value, |old| old.max(value)));
}
impl ResourceQueueSnapshotV1 {
    pub(super) fn observe(
        &mut self,
        kind: ResourceQueueKindV1,
        stamp: Stamp,
        retired: bool,
        now: Option<Instant>,
    ) {
        let p = &mut self.families[kind.index()];
        let exact = &mut self.population_complete;
        if retired {
            increment(&mut p.retired_rows, exact);
        }
        match stamp {
            Stamp::Off => {
                increment(&mut p.untracked_rows, exact);
                *exact = false;
            }
            Stamp::Pending(start) if !retired => {
                increment(&mut p.observed_admissions, exact);
                increment(&mut p.pending, exact);
                if let Some(age) = start
                    .zip(now)
                    .and_then(|(a, b)| b.checked_duration_since(a))
                {
                    maximum(&mut p.oldest_known_pending_age, age);
                } else {
                    increment(&mut p.unknown_pending_ages, exact);
                }
            }
            Stamp::Pending(_) => {
                increment(&mut p.untracked_rows, exact);
                *exact = false;
            }
            Stamp::End {
                dispatched,
                elapsed,
                result,
            } => {
                increment(&mut p.observed_admissions, exact);
                increment(
                    if dispatched {
                        &mut p.dispatched
                    } else {
                        &mut p.not_dispatched
                    },
                    exact,
                );
                if let Some(v) = elapsed {
                    maximum(&mut p.max_known_queue_duration, v);
                } else {
                    increment(&mut p.unknown_queue_durations, exact);
                }
                if dispatched {
                    result.observe(p, retired, now, exact);
                }
            }
        }
    }
}
impl State {
    pub(super) fn retire(&mut self, kind: ResourceQueueKindV1, stamp: Stamp) {
        if self.profile != Some(NativeTimingProfileV1::ProcessMonotonicV1) {
            return;
        }
        let mut out = ResourceQueueSnapshotV1 {
            families: self.retired,
            population_complete: !self.inexact,
        };
        out.observe(kind, stamp, true, None);
        self.retired = out.families;
        self.inexact = !out.population_complete;
    }
}
impl SourceCoordinator {
    fn resource_timing_stamp(
        &self,
        target: ResourceQueueAttemptV1<'_>,
    ) -> Result<(ResourceQueueKindV1, Stamp), ResourceError> {
        match target {
            ResourceQueueAttemptV1::SourceRead(id) => Ok((
                ResourceQueueKindV1::SourceRead,
                self.source_read_queue_stamp(id)?,
            )),
            ResourceQueueAttemptV1::AnnotationRead(id) => Ok((
                ResourceQueueKindV1::AnnotationRead,
                self.annotation_read_queue_stamp(id)?,
            )),
            ResourceQueueAttemptV1::Save(id) => {
                let t = self.resource_task(id)?;
                Ok((ResourceQueueKindV1::artifact(t.kind), t.queue_timing))
            }
        }
    }
    /// Exact original-owner, trusted-host timing inspection. An integration can
    /// aggregate completed queue_duration values into the declared queue metric;
    /// pending_age is censored, not a completed latency. No payload or clock stamp.
    pub fn resource_queue_attempt_timing_v1(
        &self,
        target: ResourceQueueAttemptV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResourceQueueAttemptTimingV1, ResourceError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let (kind, stamp) = self.resource_timing_stamp(target)?;
            let (phase, pending_age, queue_duration) = match stamp {
                Stamp::Off => (ResourceQueuePhaseV1::Off, None, None),
                Stamp::Pending(start) => (
                    ResourceQueuePhaseV1::Pending,
                    start
                        .zip(process_instant())
                        .and_then(|(a, b)| b.checked_duration_since(a)),
                    None,
                ),
                Stamp::End {
                    dispatched,
                    elapsed,
                    ..
                } => (
                    if dispatched {
                        ResourceQueuePhaseV1::Dispatched
                    } else {
                        ResourceQueuePhaseV1::NotDispatched
                    },
                    None,
                    elapsed,
                ),
            };
            Ok(ResourceQueueAttemptTimingV1 {
                kind,
                phase,
                pending_age,
                queue_duration,
            })
        })();
        capture_service(
            r,
            Boundary::ResourceQueueAttemptTiming,
            &result,
            timing.finish(),
        );
        result
    }
    /// Trusted-owner immutable instrumentation profile, before any resource task
    /// has been issued. Repeating the same profile never resets the population.
    pub fn configure_resource_queue_timing_v1(
        &mut self,
        profile: NativeTimingProfileV1,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            if self.resources.limits.is_none() {
                return Err(ResourceError::Unsupported);
            }
            if let Some(original) = self.resources.queue_timing.profile {
                return if original == profile {
                    Ok(())
                } else {
                    Err(ResourceError::Unavailable)
                };
            }
            self.ensure_admission_unfenced().map_err(owner_error)?;
            if !self.resources.tasks.is_empty()
                || !self.resources.reads.is_empty()
                || self.resources.writer_closed
                || self.resources.annotation_reader_closed
                || !self.resources.source_reader.queue_timing_unused()
            {
                return Err(ResourceError::Unavailable);
            }
            self.resources.queue_timing.profile = Some(profile);
            Ok(())
        })();
        capture_service(
            r,
            Boundary::ResourceQueueTimingConfiguration,
            &result,
            timing.finish(),
        );
        result
    }
    /// Trusted-owner aggregate, not recipient disclosure or an engine metric
    /// series/wire frame. One optional process clock; independent of Recorder loss.
    pub fn resource_queue_responsiveness_v1(
        &self,
        r: &mut Recorder,
    ) -> ResourceQueueResponsivenessV1 {
        let timing = r.begin_service_timing();
        let out = if self.resources.queue_timing.profile
            != Some(NativeTimingProfileV1::ProcessMonotonicV1)
        {
            ResourceQueueResponsivenessV1::Off
        } else {
            let now = process_instant();
            let mut out = ResourceQueueSnapshotV1 {
                families: self.resources.queue_timing.retired,
                population_complete: !self.resources.queue_timing.inexact,
            };
            for t in self.resources.tasks.iter().flatten() {
                out.observe(
                    ResourceQueueKindV1::artifact(t.kind),
                    t.queue_timing,
                    false,
                    now,
                );
            }
            for t in self.resources.reads.iter().flatten() {
                out.observe(
                    ResourceQueueKindV1::AnnotationRead,
                    t.queue_timing,
                    false,
                    now,
                );
            }
            self.resources.source_reader.queue_snapshot(&mut out, now);
            ResourceQueueResponsivenessV1::Observed(out)
        };
        r.record(Observation {
            boundary: Boundary::ResourceQueueTimingSnapshot,
            disposition: Disposition::Completed,
            service_time: timing.finish(),
        });
        out
    }
    pub(super) fn resource_queue_admitted(&self) -> Stamp {
        Stamp::admitted(self.resources.queue_timing.profile)
    }
    pub(super) fn retire_save_queue_timing(&mut self, kind: ArtifactKind, stamp: Stamp) {
        self.resources
            .queue_timing
            .retire(ResourceQueueKindV1::artifact(kind), stamp);
    }
}
