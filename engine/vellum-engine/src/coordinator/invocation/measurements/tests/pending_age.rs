use super::*;
#[test]
fn shutdown_inventory_includes_actual_timing_storage_in_the_original_measurement_reservation() {
    use crate::coordinator::shutdown::tests::inventory_owner;
    for timing in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        let (mut o, mut r) = owner();
        let mut p = timed_profile();
        p.timing = timing;
        p.max_pool_bytes = InvocationMeasurementLimitsV2::required_pool_bytes(16, timing).unwrap();
        o.configure_invocation_measurements_v2(p, Some(true), &mut r)
            .unwrap();
        let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::InvocationMeasurements, &[]);
        assert_eq!(row.reserved_bytes, Some(p.max_pool_bytes));
    }
}
use crate::instrumentation::NativeTimingProfileV1;

fn timed_profile() -> InvocationMeasurementLimitsV2 {
    InvocationMeasurementLimitsV2 {
        values: profile(100).values,
        max_pool_bytes: InvocationMeasurementLimitsV2::required_pool_bytes(
            16,
            NativeTimingProfileV1::ProcessMonotonicV1,
        )
        .unwrap(),
        timing: NativeTimingProfileV1::ProcessMonotonicV1,
    }
}

#[test]
fn pending_cohort_survives_diagnostic_saturation_and_replay_without_restarting_age() {
    let (mut o, mut r) = owner();
    let d = o.create(b"A", &mut r).unwrap().document;
    let q = intent(&o, &d, "A", "a", &mut r);
    let old = o.issue_invocation(&mut r).unwrap();
    o.submit(&old, &q, &mut r).unwrap();
    let held = o.retention_usage().reserved;
    let p = timed_profile();
    for allowed in [None, Some(false)] {
        assert_eq!(
            o.configure_invocation_measurements_v2(p, allowed, &mut r),
            Err(Error::Unavailable)
        );
        assert_eq!(o.retention_usage().reserved, held);
    }
    let mut short = p;
    short.max_pool_bytes -= 1;
    assert_eq!(
        o.configure_invocation_measurements_v2(short, Some(true), &mut r),
        Err(Error::Limit)
    );
    assert_eq!(o.retention_usage().reserved, held);
    assert_eq!(o.invocation_responsiveness_v1(&mut r).unwrap(), None);
    assert_eq!(
        o.configure_invocation_measurements_v2(p, Some(true), &mut r),
        Ok(true)
    );
    assert_eq!(o.retention_usage().reserved, held + p.max_pool_bytes);
    assert_eq!(
        o.configure_invocation_measurements_v2(p, Some(true), &mut r),
        Ok(false)
    );
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit(&id, &q, &mut r).unwrap();
    o.submit(&id, &q, &mut r).unwrap();
    let first = o.invocation_responsiveness_v1(&mut r).unwrap().unwrap();
    let InvocationResponsivenessV1::Observed(first) = first else {
        panic!("timed cohort");
    };
    assert_eq!((first.admissions, first.pending, first.terminal), (1, 1, 0));
    assert_eq!(first.pending_ages_unknown, 0);
    assert!(first.oldest_known_pending_age.is_some());
    let mut full = Recorder::try_with_capacity(1).unwrap();
    full.record(Observation {
        boundary: Boundary::Query,
        disposition: Disposition::Completed,
        service_time: None,
    });
    let InvocationResponsivenessV1::Observed(second) =
        o.invocation_responsiveness_v1(&mut full).unwrap().unwrap()
    else {
        panic!("timed cohort");
    };
    assert_eq!(second.pending, 1);
    assert!(second.oldest_known_pending_age >= first.oldest_known_pending_age);
    assert!(full.dropped() > 0);
    o.run(&id, &mut full).unwrap();
    o.run(&id, &mut full).unwrap();
    let InvocationResponsivenessV1::Observed(done) =
        o.invocation_responsiveness_v1(&mut r).unwrap().unwrap()
    else {
        panic!("timed cohort");
    };
    assert_eq!((done.admissions, done.pending, done.terminal), (1, 0, 1));
    assert_eq!(done.oldest_known_pending_age, None);
    assert!(done.max_known_terminal_duration.is_some());
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    o.retire_invocation(&id, &mut r).unwrap();
    let InvocationResponsivenessV1::Observed(after) =
        o.invocation_responsiveness_v1(&mut r).unwrap().unwrap()
    else {
        panic!("timed cohort");
    };
    assert_eq!(after, done);
    assert!(!format!("{after:?}").contains("engine"));
    assert!(o.stop_invocation_measurements_v1(Some(true), &mut r));
    assert_eq!(
        o.invocation_responsiveness_v1(&mut r).unwrap(),
        Some(InvocationResponsivenessV1::Off)
    );
    assert!(o.retire_invocation_measurements_v1(&control(), &mut r));
    assert_eq!(o.retention_usage().reserved, held);
    assert_eq!(
        o.invocation_responsiveness_v1(&mut r),
        Err(Error::Unavailable)
    );
    assert_eq!(
        o.invocation_state(&old, &mut r),
        Ok(InvocationState::Pending)
    );
}

#[test]
fn old_counter_profile_is_not_silently_upgraded_and_disabled_timing_is_not_zero_age() {
    let (mut o, mut r) = owner();
    enable(&mut o, &mut r);
    assert_eq!(o.invocation_responsiveness_v1(&mut r).unwrap(), None);
    assert_eq!(
        o.configure_invocation_measurements_v2(timed_profile(), Some(true), &mut r),
        Err(Error::Unavailable)
    );
    let (mut o, mut r) = owner();
    let mut p = timed_profile();
    p.timing = NativeTimingProfileV1::Off;
    p.max_pool_bytes = InvocationMeasurementLimitsV2::required_pool_bytes(16, p.timing).unwrap();
    o.configure_invocation_measurements_v2(p, Some(true), &mut r)
        .unwrap();
    assert_eq!(
        o.invocation_responsiveness_v1(&mut r).unwrap(),
        Some(InvocationResponsivenessV1::Off)
    );
}

#[test]
fn missing_regressed_and_zero_clock_samples_are_distinct_without_reclassifying_work() {
    use std::time::{Duration, Instant};
    let (mut o, mut r) = owner();
    let d = o.create(b"A", &mut r).unwrap().document;
    let q = intent(&o, &d, "A", "a", &mut r);
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit(&id, &q, &mut r).unwrap();
    let start = Instant::now();
    let end = start.checked_add(Duration::from_millis(3)).unwrap();
    let mut configured = Timing::new(16, Some(NativeTimingProfileV1::ProcessMonotonicV1)).unwrap();
    let Timing::Active(ages) = &mut configured else {
        panic!();
    };
    ages.admission_at(id.slot, Some(start));
    let known = ages.snapshot_at(&o.invocations.slots, Some(end));
    assert_eq!(
        known.oldest_known_pending_age,
        Some(Duration::from_millis(3))
    );
    assert_eq!(
        ages.snapshot_at(&o.invocations.slots, Some(start))
            .oldest_known_pending_age,
        Some(Duration::ZERO)
    );
    assert_eq!(
        ages.snapshot_at(&o.invocations.slots, None)
            .pending_ages_unknown,
        1
    );
    ages.terminal_at(id.slot, Some(start));
    assert_eq!(
        ages.snapshot_at(&o.invocations.slots, Some(end))
            .unclassified,
        1
    );
    o.run(&id, &mut r).unwrap();
    let known = ages.snapshot_at(&o.invocations.slots, Some(end));
    assert_eq!(known.max_known_terminal_duration, Some(Duration::ZERO));
    assert_eq!(known.terminal_durations_unknown, 0);
    for (a, b) in [
        (None, Some(end)),
        (Some(start), None),
        (Some(end), Some(start)),
    ] {
        let mut configured =
            Timing::new(16, Some(NativeTimingProfileV1::ProcessMonotonicV1)).unwrap();
        let Timing::Active(ages) = &mut configured else {
            panic!();
        };
        ages.admission_at(id.slot, a);
        ages.terminal_at(id.slot, b);
        let observed = ages.snapshot_at(&o.invocations.slots, Some(end));
        assert_eq!(
            (observed.terminal, observed.terminal_durations_unknown),
            (1, 1)
        );
        assert_eq!(observed.max_known_terminal_duration, None);
    }
    assert_eq!(o.invocation_usage().finished, 1);
}

#[test]
fn interrupted_publication_is_not_a_terminal_duration_and_observer_loss_does_not_publish() {
    let (mut o, mut r) = owner();
    o.configure_invocation_measurements_v2(timed_profile(), Some(true), &mut r)
        .unwrap();
    let d = o.create(b"A", &mut r).unwrap().document;
    let q = intent(&o, &d, "A", "a", &mut r);
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit(&id, &q, &mut r).unwrap();
    let State::Active(p) = &mut o.invocation_measurements else {
        panic!();
    };
    let Timing::Active(ages) = &mut p.timing else {
        panic!();
    };
    ages.admission_at(usize::MAX, None);
    let generation = o.generation();
    let InvocationResponsivenessV1::Observed(before) =
        o.invocation_responsiveness_v1(&mut r).unwrap().unwrap()
    else {
        panic!();
    };
    assert!(!before.population_complete);
    assert_eq!(before.pending, 1);
    assert_eq!(o.generation(), generation);
    assert!(!o.is_faulted());
    o.publication_fault_after = Some(1);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| o.run(&id, &mut r))).is_err());
    let InvocationResponsivenessV1::Observed(after) =
        o.invocation_responsiveness_v1(&mut r).unwrap().unwrap()
    else {
        panic!();
    };
    assert_eq!(
        (
            after.admissions,
            after.interrupted,
            after.pending,
            after.terminal
        ),
        (1, 1, 0, 0)
    );
    assert_eq!(after.max_known_terminal_duration, None);
    assert_eq!(o.documents[d.slot].as_ref().unwrap().source.text(), "a");
    assert!(o.stop_invocation_measurements_v1(Some(true), &mut r));
    assert!(o.retire_invocation_measurements_v1(&control(), &mut r));
    assert!(o.is_faulted());
    assert_eq!(o.invocation_usage().faulted_executions, 1);
}
