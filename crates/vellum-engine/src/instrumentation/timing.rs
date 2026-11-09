//! Opt-in local synchronous service timing, not a scheduler, timeout or exporter.
use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NativeTimingProfileV1 {
    #[default]
    Off,
    ProcessMonotonicV1,
}
/// One private process-local episode. No public stamps, persistence, cloning,
/// identity-bearing labels or cross-clock subtraction.
pub(crate) struct ServiceTimer {
    start: Option<Instant>,
}
impl Recorder {
    /// Applies only to subsequently entered instrumented boundaries. Existing
    /// queued records are unchanged. Normal record/drain remain clock-free.
    pub fn with_native_timing(mut self, profile: NativeTimingProfileV1) -> Self {
        self.native_timing = profile;
        self
    }
    pub fn native_timing_profile(&self) -> NativeTimingProfileV1 {
        self.native_timing
    }
    pub(crate) fn begin_service_timing(&self) -> ServiceTimer {
        let enabled = match self.native_timing {
            NativeTimingProfileV1::Off => false,
            NativeTimingProfileV1::ProcessMonotonicV1 => true,
        };
        ServiceTimer {
            start: if enabled && self.capacity() > self.queued() {
                read_clock(Instant::now)
            } else {
                None
            },
        }
    }
}
impl ServiceTimer {
    pub(crate) fn finish(self) -> Option<Duration> {
        self.finish_with(Instant::now)
    }
    fn finish_with(self, read: impl FnOnce() -> Instant) -> Option<Duration> {
        let start = self.start?;
        read_clock(read)?.checked_duration_since(start)
    }
}
// The only production reader is std::time::Instant::now. This private helper is
// not a callback facility: clock failure must never enter engine fault handling.
fn read_clock(read: impl FnOnce() -> Instant) -> Option<Instant> {
    catch_unwind(AssertUnwindSafe(read)).ok()
}
pub(crate) fn process_instant() -> Option<Instant> {
    read_clock(Instant::now)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_timing_is_default_off_and_omitted_when_capture_is_disabled_or_already_full() {
        let mut r = Recorder::try_with_capacity(1).unwrap();
        assert_eq!(r.native_timing_profile(), NativeTimingProfileV1::Off);
        assert!(r.begin_service_timing().start.is_none());
        r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        assert!(r.begin_service_timing().start.is_some());
        assert!(r.begin_service_timing().finish().is_some());
        r.record(Observation {
            boundary: Boundary::ReadObservationStopFacade,
            disposition: Disposition::Completed,
            service_time: None,
        });
        assert!(r.begin_service_timing().start.is_none());
        r.pop_front();
        assert!(r.begin_service_timing().start.is_some());
        let disabled =
            Recorder::disabled().with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        assert!(disabled.begin_service_timing().start.is_none());
    }
    #[test]
    fn native_timing_distinguishes_true_zero_from_missing_regressed_and_panicking_clocks() {
        let start = Instant::now();
        assert_eq!(
            ServiceTimer { start: Some(start) }.finish_with(|| start),
            Some(Duration::ZERO)
        );
        let end = start.checked_add(Duration::from_millis(3)).unwrap();
        assert_eq!(
            ServiceTimer { start: Some(start) }.finish_with(|| end),
            Some(Duration::from_millis(3))
        );
        assert_eq!(
            ServiceTimer { start: Some(end) }.finish_with(|| start),
            None
        );
        assert_eq!(
            ServiceTimer { start: Some(start) }.finish_with(|| panic!("clock unavailable")),
            None
        );
        assert_eq!(
            ServiceTimer { start: None }.finish_with(|| panic!("must not sample a missing start")),
            None
        );
        assert_eq!(read_clock(|| panic!("clock start unavailable")), None);
        let sampled = std::cell::Cell::new(false);
        assert_eq!(
            ServiceTimer { start: None }.finish_with(|| {
                sampled.set(true);
                start
            }),
            None
        );
        assert!(!sampled.get());
    }
}
