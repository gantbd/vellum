//! Synthetic invocation/cohort overhead gate; not a recipient facade or UI.
#[path = "responsiveness/mod.rs"]
mod support;
use std::hint::black_box;
use std::time::Instant;
use support::{Config, Distribution};
use vellum_engine::coordinator::{
    CoordinatorLimits, DocumentEdits, InvocationLimits, InvocationMeasurementControlV1,
    InvocationMeasurementLimitsV1, InvocationMeasurementLimitsV2, InvocationOutcome,
    InvocationResponsivenessV1, InvocationState, SourceCoordinator, SourceIntent,
};
use vellum_engine::instrumentation::measurement::{PointStateV1, ValueLimitsV1, ValueV1};
use vellum_engine::instrumentation::{
    Boundary, Disposition, NativeTimingProfileV1, Observation, Recorder,
};
use vellum_engine::source::{Limits, Replacement};

#[derive(Clone, Copy)]
enum Cohort {
    Off,
    Counters,
    Timed,
}
impl Cohort {
    fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Counters => "counters",
            Self::Timed => "timed",
        }
    }
}
#[derive(Clone, Copy)]
enum CaptureMode {
    Off,
    Timed,
    Saturated,
}
impl CaptureMode {
    fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Timed => "timed",
            Self::Saturated => "saturated",
        }
    }
    fn recorder(self) -> Recorder {
        let capacity = match self {
            Self::Off => 0,
            Self::Timed => 64,
            Self::Saturated => 1,
        };
        let mut r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        if matches!(self, Self::Saturated) {
            r.record(Observation {
                boundary: Boundary::Query,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        r
    }
}
struct Report {
    phases: [Distribution; 5],
    elapsed_ns: u128,
    observer_bytes: usize,
    observer_admissions: Option<u64>,
    lifetime_tickets: usize,
    retained_jobs: usize,
    diagnostic_drops: u64,
    timed_records: usize,
}
fn run(cohort: Cohort, capture: CaptureMode, config: Config) -> Result<Report, &'static str> {
    let total = config
        .samples
        .checked_add(config.warmup)
        .ok_or("iteration overflow")?;
    // This experiment queries an O(lifetime-bound) observer every operation.
    // Bound that experiment's work, not the engine's supported capacity.
    if config.samples == 0 || total > 4096 {
        return Err("experiment requires 1..4096 lifetime tickets");
    }
    let mut setup = Recorder::disabled();
    let mut owner = SourceCoordinator::with_invocation_limits(
        CoordinatorLimits {
            max_document_incarnations: 2,
            max_participants: 2,
            max_staged_bytes: 8192,
            source: Limits {
                max_source_bytes: 4096,
                max_edits: 4,
                max_edit_payload_bytes: 8,
            },
        },
        InvocationLimits {
            max_lifetime_issued: total,
            max_retained: 1,
            max_intent_bytes: 65536,
        },
    );
    let timing = NativeTimingProfileV1::ProcessMonotonicV1;
    let budget = InvocationMeasurementLimitsV2::required_pool_bytes(total, timing)
        .map_err(|_| "observer byte calculation failed")?;
    owner
        .configure_retention_budget(budget, &mut setup)
        .map_err(|_| "parent configuration failed")?;
    let values = ValueLimitsV1 {
        max_integer: (total + 1) as u64,
        max_seconds: 0.0,
    };
    match cohort {
        Cohort::Off => {}
        Cohort::Counters => {
            owner
                .configure_invocation_measurements_v1(
                    InvocationMeasurementLimitsV1 {
                        values,
                        max_pool_bytes: InvocationMeasurementLimitsV1::required_pool_bytes(total)
                            .map_err(|_| "counter byte calculation failed")?,
                    },
                    Some(true),
                    &mut setup,
                )
                .map_err(|_| "counter configuration failed")?;
        }
        Cohort::Timed => {
            owner
                .configure_invocation_measurements_v2(
                    InvocationMeasurementLimitsV2 {
                        values,
                        max_pool_bytes: budget,
                        timing,
                    },
                    Some(true),
                    &mut setup,
                )
                .map_err(|_| "timing configuration failed")?;
        }
    }
    let observer_bytes = owner.retention_usage().reserved;
    let mut intent = SourceIntent {
        writes: vec![],
        reads: vec![],
    };
    for _ in 0..2 {
        let expected = owner
            .create(&vec![b'a'; 4096], &mut setup)
            .map_err(|_| "fixture failed")?;
        intent.writes.push(DocumentEdits {
            expected,
            edits: (0..4)
                .map(|i| Replacement {
                    start: i * 1024,
                    end: i * 1024 + 1,
                    expected: "a".into(),
                    inserted: "b".into(),
                })
                .collect(),
        });
    }
    let mut r = capture.recorder();
    let mut phases: [Vec<u128>; 5] = std::array::from_fn(|_| Vec::with_capacity(config.samples));
    let mut measured = Instant::now();
    let mut dropped_before = 0;
    let mut timed_records = 0;
    for iteration in 0..total {
        if iteration == config.warmup {
            dropped_before = r.dropped();
            measured = Instant::now();
        }
        let id = owner
            .issue_invocation(&mut r)
            .map_err(|_| "ticket issuance failed")?;
        let start = Instant::now();
        if owner
            .submit(&id, black_box(&intent), &mut r)
            .map_err(|_| "admission failed")?
            != InvocationState::Pending
        {
            return Err("admission did not produce pending work");
        }
        let admission = start.elapsed().as_nanos();
        let service_start = Instant::now();
        if owner.run(&id, &mut r).map_err(|_| "service failed")? != InvocationState::Finished {
            return Err("service did not finish");
        }
        let service = service_start.elapsed().as_nanos();
        let query_start = Instant::now();
        let InvocationOutcome::Committed(outcome) = owner
            .invocation_outcome(&id, &mut r)
            .map_err(|_| "result unavailable")?
        else {
            return Err("unexpected local disposition");
        };
        if outcome.changes.len() != 2 || outcome.operation.is_none() {
            return Err("missing changed participants");
        }
        for (request, change) in intent.writes.iter_mut().zip(&outcome.changes) {
            let read = owner
                .read(&request.expected.document, &mut r)
                .map_err(|_| "source query failed")?;
            if read.version != change.after
                || read.generation != outcome.generation
                || read.source.text().len() != 4096
            {
                return Err("query did not observe coherent committed versions");
            }
            for edit in &mut request.edits {
                if read.source.text().as_bytes()[edit.start] != edit.inserted.as_bytes()[0] {
                    return Err("edit meaning differs");
                }
                std::mem::swap(&mut edit.expected, &mut edit.inserted);
            }
            request.expected = change.after.clone();
            black_box(read.source.text());
        }
        let query = query_start.elapsed().as_nanos();
        let observer_start = Instant::now();
        let observation = owner
            .invocation_responsiveness_v1(&mut r)
            .map_err(|_| "observer query failed")?;
        match cohort {
            Cohort::Off | Cohort::Counters if observation.is_none() => {}
            Cohort::Timed => {
                let Some(InvocationResponsivenessV1::Observed(p)) = observation else {
                    return Err("missing timed population");
                };
                if p.admissions != iteration + 1
                    || p.terminal != iteration + 1
                    || p.pending != 0
                    || p.terminal_durations_unknown != 0
                    || p.interrupted != 0
                    || p.unclassified != 0
                    || !p.population_complete
                    || p.max_known_terminal_duration.is_none()
                {
                    return Err("observer population incomplete or differs");
                }
            }
            _ => return Err("unexpected observer profile"),
        }
        let observer = observer_start.elapsed().as_nanos();
        let elapsed = start.elapsed().as_nanos();
        if iteration >= config.warmup {
            for (samples, n) in phases
                .iter_mut()
                .zip([elapsed, admission, service, query, observer])
            {
                samples.push(n);
            }
        }
        owner
            .forget_invocation_outcome(&id, &mut r)
            .map_err(|_| "result forget failed")?;
        owner
            .retire_invocation(&id, &mut r)
            .map_err(|_| "job retirement failed")?;
        if !matches!(capture, CaptureMode::Saturated) {
            while let Some(record) = r.pop_front() {
                if iteration >= config.warmup && record.service_time.is_some() {
                    timed_records += 1;
                }
            }
        }
    }
    let elapsed_ns = measured.elapsed().as_nanos();
    let usage = owner.invocation_usage();
    if usage.retained != 0 || usage.pending != 0 || usage.issued_lifetime != total {
        return Err("job lifecycle leaked or differs");
    }
    let diagnostic_drops = r.dropped() - dropped_before;
    let counters = owner
        .invocation_measurements_v1(&mut setup)
        .map_err(|_| "counter query failed")?;
    let observer_admissions = if let Some(counters) = counters {
        let count =
            |p: &vellum_engine::instrumentation::measurement::MeasurementPointV1| match p.state() {
                PointStateV1::Observed {
                    value: ValueV1::Counter(c),
                    ..
                } if c.exact => Ok(c.total),
                _ => Err("counter incomplete"),
            };
        if count(&counters.admissions)? != total as u64
            || count(&counters.dispositions[0])? != total as u64
        {
            return Err("counter denominator differs");
        }
        for point in &counters.dispositions[1..] {
            if count(point)? != 0 {
                return Err("unexpected terminal outcome");
            }
        }
        Some(total as u64)
    } else {
        None
    };
    if !matches!(cohort, Cohort::Off)
        && (!owner.stop_invocation_measurements_v1(Some(true), &mut setup)
            || !owner.retire_invocation_measurements_v1(
                &InvocationMeasurementControlV1 {
                    owner_control_allowed: Some(true),
                    delivery_drained: Some(true),
                    retention_released: Some(true),
                },
                &mut setup,
            ))
    {
        return Err("observer cleanup failed");
    }
    if owner.retention_usage().reserved != 0 {
        return Err("observer reservation leaked");
    }
    Ok(Report {
        phases: phases.map(|mut v| Distribution::from_samples(&mut v).unwrap()),
        elapsed_ns,
        observer_bytes,
        observer_admissions,
        lifetime_tickets: usage.issued_lifetime,
        retained_jobs: usage.retained,
        diagnostic_drops,
        timed_records,
    })
}

fn main() {
    let start = Instant::now();
    println!("# entered-main; process-local timing excludes launch/pre-main");
    let result = execute();
    println!("# in_process_elapsed_ns={}", start.elapsed().as_nanos());
    if let Err(error) = result {
        eprintln!("invocation responsiveness: {error}");
        std::process::exit(1);
    }
}
fn execute() -> Result<(), String> {
    let config = Config::parse(std::env::args().skip(1))?;
    if cfg!(debug_assertions) {
        return Err("use --release".into());
    }
    println!(
        "# invocation-responsiveness-v1; synthetic UTF-8 ASCII; trusted-owner; monotonic ns; arch={}; os={}",
        std::env::consts::ARCH,
        std::env::consts::OS
    );
    println!(
        "# samples={}; warmup={}; p99_budget_ns={}",
        config.samples,
        config.warmup,
        config.max_p99_ns.map_or("unset".into(), |n| n.to_string())
    );
    println!(
        "# two 4096-byte documents, four edits each; total includes observer query every operation; no queue/UI/I/O"
    );
    println!(
        "# issuance/cleanup/drain excluded from total, included in cohort time; observer counts include warmup; allocations/RSS unmeasured"
    );
    println!(
        "cohort,capture,phase,count,p50_ns,p95_ns,p99_ns,max_ns,cohort_elapsed_ns,observer_reserved_bytes,observer_admissions,lifetime_tickets,retained_jobs,diagnostic_drops,timed_records"
    );
    let mut failed = false;
    for cohort in [Cohort::Off, Cohort::Counters, Cohort::Timed] {
        for capture in [CaptureMode::Off, CaptureMode::Timed, CaptureMode::Saturated] {
            let r = run(cohort, capture, config).map_err(str::to_owned)?;
            failed |= !config.accepts_total(r.phases[0]);
            for (phase, d) in [
                "total",
                "admission",
                "service",
                "result-query",
                "observer-query",
            ]
            .into_iter()
            .zip(r.phases)
            {
                println!(
                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                    cohort.name(),
                    capture.name(),
                    phase,
                    d.count,
                    d.p50,
                    d.p95,
                    d.p99,
                    d.maximum,
                    r.elapsed_ns,
                    r.observer_bytes,
                    r.observer_admissions
                        .map_or("unmeasured".into(), |n| n.to_string()),
                    r.lifetime_tickets,
                    r.retained_jobs,
                    r.diagnostic_drops,
                    r.timed_records
                );
            }
        }
    }
    if failed {
        Err("caller-supplied total p99 budget exceeded; all cohorts reported".into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_observer_and_capture_modes_measure_every_operation_and_release_jobs() {
        for cohort in [Cohort::Off, Cohort::Counters, Cohort::Timed] {
            for capture in [CaptureMode::Off, CaptureMode::Timed, CaptureMode::Saturated] {
                let r = run(
                    cohort,
                    capture,
                    Config {
                        samples: 3,
                        warmup: 2,
                        max_p99_ns: None,
                    },
                )
                .unwrap();
                assert!(r.phases.iter().all(|p| p.count == 3));
                assert_eq!(r.retained_jobs, 0);
                assert_eq!(r.lifetime_tickets, 5);
                match cohort {
                    Cohort::Off => {
                        assert_eq!(r.observer_bytes, 0);
                        assert_eq!(r.observer_admissions, None);
                    }
                    _ => {
                        assert!(r.observer_bytes > 0);
                        assert_eq!(r.observer_admissions, Some(5));
                    }
                }
                if matches!(capture, CaptureMode::Saturated) {
                    assert!(r.diagnostic_drops > 0);
                    assert_eq!(r.timed_records, 0);
                }
                if matches!(capture, CaptureMode::Timed) {
                    assert!(r.timed_records > 0);
                }
            }
        }
    }
    #[test]
    fn excessive_lifetime_scan_work_refuses_before_running_the_experiment() {
        assert!(
            run(
                Cohort::Timed,
                CaptureMode::Off,
                Config {
                    samples: 4097,
                    warmup: 0,
                    max_p99_ns: None
                }
            )
            .is_err()
        );
    }
}
