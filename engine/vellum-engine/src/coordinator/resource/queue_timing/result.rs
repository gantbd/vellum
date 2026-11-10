//! First accepted attributed result intake, not progress, quiescence or physical truth.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceResultAttemptTimingV1 {
    pub kind: ResourceQueueKindV1,
    pub phase: ResourceQueuePhaseV1,
    /// Censored age, not a completed latency. None before dispatch, after first
    /// result, when disabled, or when a process-clock observation is unavailable.
    pub awaiting_result_age: Option<Duration>,
    pub first_result_observed: bool,
    /// Dispatch -> first accepted result-payload/terminal claim intake. Includes
    /// intake service; not wire arrival, provider CPU time or completion truth.
    pub time_to_first_result: Option<Duration>,
}
#[derive(Clone, Copy)]
pub(in crate::coordinator::resource) struct ResultStamp {
    dispatched_at: Option<Instant>,
    // Outer None is not observed; Some(None) is observed with unknown clock.
    first: Option<Option<Duration>>,
}
impl ResultStamp {
    pub(super) fn new(dispatched_at: Option<Instant>) -> Self {
        Self {
            dispatched_at,
            first: None,
        }
    }
    fn accept_at(&mut self, now: Option<Instant>) {
        if self.first.is_none() {
            self.first = Some(
                self.dispatched_at
                    .zip(now)
                    .and_then(|(a, b)| b.checked_duration_since(a)),
            );
        }
    }
    fn age(self, now: Option<Instant>) -> Option<Duration> {
        self.dispatched_at
            .zip(now)
            .and_then(|(a, b)| b.checked_duration_since(a))
    }
    pub(super) fn observe(
        self,
        p: &mut ResourceQueuePopulationV1,
        retired: bool,
        now: Option<Instant>,
        exact: &mut bool,
    ) {
        if let Some(elapsed) = self.first {
            increment(&mut p.first_results, exact);
            if let Some(v) = elapsed {
                maximum(&mut p.max_known_first_result_duration, v);
            } else {
                increment(&mut p.unknown_first_result_durations, exact);
            }
        } else if retired {
            increment(&mut p.retired_without_first_result, exact);
        } else {
            increment(&mut p.awaiting_first_result, exact);
            if let Some(v) = self.age(now) {
                maximum(&mut p.oldest_known_first_result_age, v);
            } else {
                increment(&mut p.unknown_first_result_ages, exact);
            }
        }
    }
}
impl Stamp {
    /// Called only after native original attribution and accepted evidence intake.
    /// The first observation is absorbing, including an unavailable clock.
    pub(in crate::coordinator::resource) fn accepted_result(&mut self) {
        if let Self::End {
            dispatched: true,
            result,
            ..
        } = self
            && result.first.is_none()
        {
            result.accept_at(process_instant());
        }
    }
}
impl SourceCoordinator {
    /// Privileged original-owner observation only. No recipient grant, task
    /// execution, evidence, timeout or scheduling policy is implied by this read.
    pub fn resource_result_attempt_timing_v1(
        &self,
        target: ResourceQueueAttemptV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResourceResultAttemptTimingV1, ResourceError> {
        let timer = r.begin_service_timing();
        let out = (|| {
            let (kind, stamp) = self.resource_timing_stamp(target)?;
            let (phase, result) = match stamp {
                Stamp::Off => (ResourceQueuePhaseV1::Off, None),
                Stamp::Pending(_) => (ResourceQueuePhaseV1::Pending, None),
                Stamp::End {
                    dispatched: false, ..
                } => (ResourceQueuePhaseV1::NotDispatched, None),
                Stamp::End {
                    dispatched: true,
                    result,
                    ..
                } => (ResourceQueuePhaseV1::Dispatched, Some(result)),
            };
            let first = result.and_then(|s| s.first);
            Ok(ResourceResultAttemptTimingV1 {
                kind,
                phase,
                awaiting_result_age: if first.is_none() {
                    result.and_then(|s| s.age(process_instant()))
                } else {
                    None
                },
                first_result_observed: first.is_some(),
                time_to_first_result: first.flatten(),
            })
        })();
        capture_service(
            r,
            Boundary::ResourceResultAttemptTiming,
            &out,
            timer.finish(),
        );
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_result_zero_unknown_regression_and_duplicate_are_distinct_and_absorbing() {
        let t = Instant::now();
        for (start, now, expected) in [
            (Some(t), Some(t), Some(Duration::ZERO)),
            (None, Some(t), None),
            (Some(t), None, None),
            (Some(t + Duration::from_secs(1)), Some(t), None),
        ] {
            let mut s = ResultStamp::new(start);
            s.accept_at(now);
            assert_eq!(s.first, Some(expected));
            s.accept_at(Some(t + Duration::from_secs(10)));
            assert_eq!(s.first, Some(expected));
            let mut p = ResourceQueuePopulationV1::default();
            let mut exact = true;
            s.observe(&mut p, true, None, &mut exact);
            assert_eq!(p.first_results, 1);
            assert_eq!(
                p.unknown_first_result_durations,
                usize::from(expected.is_none())
            );
            assert_eq!(p.retired_without_first_result, 0);
            assert!(exact);
        }
    }
    #[test]
    fn missing_clock_and_retired_without_result_do_not_become_zero_latency_or_pending_work() {
        let s = ResultStamp::new(None);
        let mut p = ResourceQueuePopulationV1::default();
        let mut exact = true;
        s.observe(&mut p, false, Some(Instant::now()), &mut exact);
        assert_eq!(
            (p.awaiting_first_result, p.unknown_first_result_ages),
            (1, 1)
        );
        let mut retired = ResourceQueuePopulationV1::default();
        s.observe(&mut retired, true, None, &mut exact);
        assert_eq!(
            (
                retired.awaiting_first_result,
                retired.retired_without_first_result,
                retired.first_results
            ),
            (0, 1, 0)
        );
    }
    #[test]
    fn result_population_overflow_stays_capped_and_explicitly_inexact() {
        let t = Instant::now();
        let mut s = ResultStamp::new(Some(t));
        s.accept_at(Some(t));
        let mut p = ResourceQueuePopulationV1 {
            first_results: usize::MAX,
            ..Default::default()
        };
        let mut exact = true;
        s.observe(&mut p, true, None, &mut exact);
        assert_eq!(p.first_results, usize::MAX);
        assert!(!exact);
    }
}
