use std::time::Duration;
use vellum_engine::instrumentation::{Boundary, Capture, Disposition, Observation, Recorder};

fn sample(boundary: Boundary, disposition: Disposition) -> Observation {
    Observation {
        boundary,
        disposition,
        service_time: Some(Duration::from_micros(7)),
    }
}

#[test]
fn disabled_capture_retains_nothing() {
    for mut recorder in [
        Recorder::disabled(),
        Recorder::try_with_capacity(0).unwrap(),
    ] {
        assert_eq!(
            recorder.record(sample(Boundary::Query, Disposition::Completed)),
            Capture::Disabled
        );
        assert_eq!(recorder.queued(), 0);
        assert_eq!(recorder.dropped(), 0);
        assert_eq!(recorder.pop_front(), None);
    }
}

#[test]
fn overflow_preserves_retained_order_and_reuses_capacity_after_drain() {
    let mut recorder = Recorder::try_with_capacity(2).unwrap();
    let first = sample(Boundary::Admission, Disposition::Refused);
    let second = sample(Boundary::SubstrateWrite, Disposition::Unknown);
    assert_eq!(recorder.record(first), Capture::Recorded);
    assert_eq!(recorder.record(second), Capture::Recorded);
    assert_eq!(recorder.record(first), Capture::Dropped);
    assert_eq!(recorder.pop_front(), Some(first));
    assert_eq!(recorder.record(first), Capture::Recorded);
    assert_eq!(recorder.pop_front(), Some(second));
    assert_eq!(recorder.pop_front(), Some(first));
    assert_eq!(recorder.pop_front(), None);
    assert_eq!(recorder.capacity(), 2);
    assert_eq!(recorder.dropped(), 1);
}

#[test]
fn impossible_reservation_is_an_explicit_error() {
    assert!(Recorder::try_with_capacity(usize::MAX).is_err());
}
