//! Synthetic trusted-owner performance gate. No UI, provider, runtime or exporter.
use std::hint::black_box;
use std::time::Instant;
use vellum_engine::coordinator::{CoordinatorLimits, DocumentEdits, SourceCoordinator};
use vellum_engine::instrumentation::{
    Boundary, Disposition, NativeTimingProfileV1, Observation, Recorder,
};
use vellum_engine::source::{Limits, Replacement};
#[path = "responsiveness/mod.rs"]
mod support;
use support::{Config, Distribution};

#[derive(Clone, Copy)]
struct Workload {
    name: &'static str,
    bytes: usize,
    documents: usize,
    edits: usize,
}
const WORKLOADS: [Workload; 3] = [
    Workload {
        name: "resident-small",
        bytes: 4096,
        documents: 1,
        edits: 1,
    },
    Workload {
        name: "agentic-multi",
        bytes: 65536,
        documents: 2,
        edits: 16,
    },
    Workload {
        name: "resident-large",
        bytes: 1048576,
        documents: 1,
        edits: 1,
    },
];
#[derive(Clone, Copy)]
enum Mode {
    Off,
    Capture,
    Timed,
    Saturated,
}
impl Mode {
    fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Capture => "capture",
            Self::Timed => "timed",
            Self::Saturated => "saturated",
        }
    }
    fn recorder(self) -> Recorder {
        let capacity = match self {
            Self::Off => 0,
            Self::Saturated => 1,
            _ => 64,
        };
        let profile = match self {
            Self::Off | Self::Capture => NativeTimingProfileV1::Off,
            Self::Timed | Self::Saturated => NativeTimingProfileV1::ProcessMonotonicV1,
        };
        let mut r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(profile);
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
    phases: [Distribution; 4],
    elapsed_ns: u128,
    timed_records: usize,
    dropped: u64,
}
fn run(w: Workload, mode: Mode, config: Config) -> Result<Report, &'static str> {
    let mut owner = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: w.documents,
        max_participants: w.documents,
        max_staged_bytes: w.bytes * w.documents,
        source: Limits {
            max_source_bytes: w.bytes,
            max_edits: w.edits,
            max_edit_payload_bytes: 2 * w.edits,
        },
    });
    let data = vec![b'a'; w.bytes];
    let mut writes = Vec::with_capacity(w.documents);
    for _ in 0..w.documents {
        let expected = owner
            .create(&data, &mut Recorder::disabled())
            .map_err(|_| "fixture creation failed")?;
        writes.push(DocumentEdits {
            expected,
            edits: (0..w.edits)
                .map(|i| Replacement {
                    start: i * w.bytes / w.edits,
                    end: i * w.bytes / w.edits + 1,
                    expected: "a".into(),
                    inserted: "b".into(),
                })
                .collect(),
        });
    }
    let mut recorder = mode.recorder();
    let mut phases: [Vec<u128>; 4] = std::array::from_fn(|_| Vec::with_capacity(config.samples));
    let mut timed_records = 0;
    let mut dropped_before = 0;
    let mut measured = Instant::now();
    for iteration in 0..config.warmup + config.samples {
        if iteration == config.warmup {
            dropped_before = recorder.dropped();
            measured = Instant::now();
        }
        // Original owned request already exists. No queue or ingress is implied.
        let start = Instant::now();
        let prepared = owner
            .prepare(black_box(&writes), &[], &mut recorder)
            .map_err(|_| "preparation failed; cohort incomplete")?;
        let preparation = start.elapsed().as_nanos();
        let commit_start = Instant::now();
        let outcome = owner
            .commit(prepared, &mut recorder)
            .map_err(|_| "commit failed; cohort incomplete")?;
        let commit = commit_start.elapsed().as_nanos();
        let query_start = Instant::now();
        if outcome.changes.len() != w.documents {
            return Err("missing changed participant");
        }
        for (request, change) in writes.iter().zip(&outcome.changes) {
            let read = owner
                .read(&request.expected.document, &mut recorder)
                .map_err(|_| "query failed; cohort incomplete")?;
            if read.version != change.after || read.source.text().len() != w.bytes {
                return Err("query did not observe the committed revision");
            }
            for edit in &request.edits {
                if read.source.text().as_bytes()[edit.start] != edit.inserted.as_bytes()[0] {
                    return Err("committed edit meaning differs");
                }
            }
            black_box(read.source.text());
        }
        let query = query_start.elapsed().as_nanos();
        let total = start.elapsed().as_nanos();
        if iteration >= config.warmup {
            for (samples, value) in phases.iter_mut().zip([total, preparation, commit, query]) {
                samples.push(value);
            }
        }
        // Bookkeeping/drain are outside per-operation latency, inside throughput.
        for (request, change) in writes.iter_mut().zip(outcome.changes) {
            request.expected = change.after;
            for edit in &mut request.edits {
                std::mem::swap(&mut edit.expected, &mut edit.inserted);
            }
        }
        if !matches!(mode, Mode::Saturated) {
            while let Some(record) = recorder.pop_front() {
                if iteration >= config.warmup && record.service_time.is_some() {
                    timed_records += 1;
                }
            }
        }
    }
    let elapsed_ns = measured.elapsed().as_nanos();
    Ok(Report {
        phases: phases.map(|mut v| Distribution::from_samples(&mut v).unwrap()),
        elapsed_ns,
        timed_records,
        dropped: recorder.dropped() - dropped_before,
    })
}
fn main() {
    let entered = Instant::now();
    println!("# entered-main; process-local timing excludes launch and pre-main initialization");
    let result = execute();
    println!("# in_process_elapsed_ns={}", entered.elapsed().as_nanos());
    if let Err(error) = result {
        eprintln!("responsiveness: {error}");
        std::process::exit(1);
    }
}
fn execute() -> Result<(), String> {
    let config = Config::parse(std::env::args().skip(1))?;
    if cfg!(debug_assertions) {
        return Err("run this gate with --release".into());
    }
    println!("# responsiveness-v1; synthetic UTF-8 ASCII; trusted-owner; process monotonic ns");
    println!(
        "# samples={}; warmup={}; p99_budget_ns={}; arch={}; os={}",
        config.samples,
        config.warmup,
        config.max_p99_ns.map_or("unset".into(), |n| n.to_string()),
        std::env::consts::ARCH,
        std::env::consts::OS
    );
    println!(
        "# total=owned-request-to-query; no queue/UI/I/O; clock and correctness-check overhead included"
    );
    println!(
        "# allocations/RSS unmeasured; cohort time includes bounded drain/bookkeeping; no incomplete cohort is a pass"
    );
    println!(
        "workload,bytes_per_document,documents,edits_per_document,mode,phase,count,p50_ns,p95_ns,p99_ns,max_ns,cohort_elapsed_ns,timed_records,diagnostic_drops"
    );
    let mut failed = false;
    for workload in WORKLOADS {
        for mode in [Mode::Off, Mode::Capture, Mode::Timed, Mode::Saturated] {
            let report = run(workload, mode, config).map_err(str::to_owned)?;
            // External timing samples every operation even when diagnostics drop.
            if !config.accepts_total(report.phases[0]) {
                failed = true;
            }
            for (phase, d) in ["total", "preparation", "commit", "query"]
                .into_iter()
                .zip(report.phases)
            {
                println!(
                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                    workload.name,
                    workload.bytes,
                    workload.documents,
                    workload.edits,
                    mode.name(),
                    phase,
                    d.count,
                    d.p50,
                    d.p95,
                    d.p99,
                    d.maximum,
                    report.elapsed_ns,
                    report.timed_records,
                    report.dropped
                );
            }
        }
    }
    if failed {
        Err("caller-supplied total p99 budget exceeded; see all cohorts above".into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_external_samples_survive_saturation_and_warmup_is_not_counted() {
        let workload = Workload {
            name: "test",
            bytes: 64,
            documents: 2,
            edits: 2,
        };
        let config = Config {
            samples: 3,
            warmup: 2,
            max_p99_ns: None,
        };
        for mode in [Mode::Off, Mode::Capture, Mode::Timed, Mode::Saturated] {
            let r = run(workload, mode, config).unwrap();
            assert!(r.phases.iter().all(|d| d.count == 3));
            if matches!(mode, Mode::Timed) {
                assert_eq!(r.timed_records, 12);
            } else {
                assert_eq!(r.timed_records, 0);
            }
            if matches!(mode, Mode::Saturated) {
                assert!(r.dropped >= 12);
            } else {
                assert_eq!(r.dropped, 0);
            }
        }
    }
}
