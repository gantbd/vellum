use super::*;
fn snapshot() -> ResourceQueueSnapshotV1 {
    ResourceQueueSnapshotV1 {
        families: [Default::default(); 4],
        population_complete: true,
    }
}
#[test]
fn zero_unknown_regressed_clock_and_once_only_settlement_are_distinct() {
    let now = Instant::now();
    for (start, end, expected) in [
        (Some(now), Some(now), Some(Duration::ZERO)),
        (None, Some(now), None),
        (Some(now), None, None),
        (Some(now + Duration::from_secs(1)), Some(now), None),
    ] {
        let mut s = Stamp::Pending(start);
        s.finish_at(true, end);
        let Stamp::End {
            dispatched,
            elapsed,
            ..
        } = s
        else {
            panic!("ended")
        };
        assert!(dispatched);
        assert_eq!(elapsed, expected);
        s.finish_at(false, Some(now + Duration::from_secs(5)));
        let Stamp::End {
            dispatched,
            elapsed,
            ..
        } = s
        else {
            panic!("ended")
        };
        assert!(dispatched);
        assert_eq!(elapsed, expected);
    }
}
#[test]
fn pending_population_has_explicit_unknown_ages_and_retirement_does_not_disappear() {
    let now = Instant::now();
    let mut out = snapshot();
    out.observe(
        ResourceQueueKindV1::SourceRead,
        Stamp::Pending(Some(now)),
        false,
        Some(now),
    );
    out.observe(
        ResourceQueueKindV1::SourceRead,
        Stamp::Pending(None),
        false,
        Some(now),
    );
    let p = out.families[0];
    assert_eq!(
        (p.observed_admissions, p.pending, p.unknown_pending_ages),
        (2, 2, 1)
    );
    assert_eq!(p.oldest_known_pending_age, Some(Duration::ZERO));
    assert!(out.population_complete);
    let mut state = State {
        profile: Some(NativeTimingProfileV1::ProcessMonotonicV1),
        ..Default::default()
    };
    state.retire(
        ResourceQueueKindV1::SourceRead,
        Stamp::End {
            dispatched: true,
            elapsed: None,
            result: ResultStamp::new(None),
        },
    );
    let mut out = ResourceQueueSnapshotV1 {
        families: state.retired,
        population_complete: !state.inexact,
    };
    out.observe(
        ResourceQueueKindV1::SourceRead,
        Stamp::Pending(Some(now)),
        false,
        Some(now),
    );
    let p = out.families[0];
    assert_eq!(
        (
            p.observed_admissions,
            p.pending,
            p.retired_rows,
            p.dispatched,
            p.unknown_queue_durations
        ),
        (2, 1, 1, 1, 1)
    );
}
#[test]
fn capped_counts_and_untracked_rows_never_claim_complete_population() {
    let mut state = State {
        profile: Some(NativeTimingProfileV1::ProcessMonotonicV1),
        ..Default::default()
    };
    state.retired[0].observed_admissions = usize::MAX;
    state.retire(
        ResourceQueueKindV1::SourceRead,
        Stamp::End {
            dispatched: false,
            elapsed: Some(Duration::ZERO),
            result: ResultStamp::new(None),
        },
    );
    assert_eq!(state.retired[0].observed_admissions, usize::MAX);
    assert!(state.inexact);
    let mut out = snapshot();
    out.observe(ResourceQueueKindV1::SourceRead, Stamp::Off, false, None);
    assert_eq!(
        (
            out.families[0].observed_admissions,
            out.families[0].untracked_rows
        ),
        (0, 1)
    );
    assert!(!out.population_complete);
}
