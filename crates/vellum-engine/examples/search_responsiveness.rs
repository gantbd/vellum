//! Trusted native retained-search experiment; not the unfinished /4 facade.
use std::time::Instant;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{
    Boundary, Disposition, NativeTimingProfileV1, Observation, Recorder,
};
use vellum_engine::source::Limits;
#[path = "responsiveness/mod.rs"]
mod support;
use support::{Config, Distribution};

#[derive(Clone, Copy)]
struct Workload {
    name: &'static str,
    unit: &'static str,
    repeats: usize,
    captured: bool,
}
const QUERY: &str = "needleΩ";
const WORKLOADS: [Workload; 3] = [
    Workload {
        name: "unicode-prose",
        unit: "English café. Ein deutscher Abschnitt. 🙂 needleΩ\r\n",
        repeats: 64,
        captured: false,
    },
    Workload {
        name: "unicode-code-scopes",
        unit: "fn sample() { /* λ needleΩ */ let n = 42; }\n",
        repeats: 1536,
        captured: true,
    },
    Workload {
        name: "mixed-large",
        unit: "# Notes\r\n```rust\n// needleΩ 🙂\nlet x = 1;\n```\nDeutsch: Grüße.\r\n",
        repeats: 16384,
        captured: false,
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
        let mut r = Recorder::try_with_capacity(match self {
            Self::Off => 0,
            Self::Saturated => 1,
            _ => 64,
        })
        .unwrap()
        .with_native_timing(match self {
            Self::Timed | Self::Saturated => NativeTimingProfileV1::ProcessMonotonicV1,
            _ => NativeTimingProfileV1::Off,
        });
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
    bytes: usize,
    matches: usize,
    elapsed_ns: u128,
    timed_records: usize,
    dropped: u64,
    issued: usize,
}
fn iterations(config: Config) -> Result<usize, &'static str> {
    config
        .samples
        .checked_add(config.warmup)
        .filter(|n| *n > 0 && *n <= 4096)
        .ok_or("search experiment lifetime must be 1..4096; no workload has run")
}
fn run(w: Workload, mode: Mode, config: Config) -> Result<Report, &'static str> {
    let count = iterations(config)?;
    if config.samples == 0 || w.repeats == 0 || w.repeats > 16384 || w.unit.len() > 256 {
        return Err("unsupported/empty experimental cohort");
    }
    let text = w.unit.repeat(w.repeats);
    // Independent standard-library oracle, constructed outside measured service.
    let expected: Vec<_> = text
        .match_indices(QUERY)
        .map(|(start, matched)| ByteRange {
            start,
            end: start + matched.len(),
        })
        .collect();
    if expected.is_empty() {
        return Err("fixture has no expected match");
    }
    let mut owner = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 1,
        max_participants: 1,
        max_staged_bytes: text.len(),
        source: Limits {
            max_source_bytes: text.len(),
            max_edits: 1,
            max_edit_payload_bytes: 0,
        },
    });
    let mut setup = Recorder::disabled();
    owner
        .configure_retention_budget(268_435_456, &mut setup)
        .map_err(|_| "parent configuration refused")?;
    owner
        .configure_semantic_root_budget(16_777_216, &mut setup)
        .map_err(|_| "root configuration refused")?;
    let version = owner
        .create(text.as_bytes(), &mut setup)
        .map_err(|_| "source creation refused")?;
    let pool_control = SourceReadPoolControlV1 {
        owner_control_allowed: Some(true),
        resources_ready: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    owner
        .configure_source_read_contexts(
            "experiment",
            "local",
            SourceReadContextLimitsV1 {
                max_contexts: 1,
                max_holds: 1,
                max_issued: count + 1,
                max_root_bytes: 16_777_216,
                max_capture_bytes: text.len() + 8192,
                max_context_bytes: 8192,
                max_request_bytes: 8192,
                max_reply_bytes: 4_194_304,
                max_label_bytes: 128,
                max_scan_bytes: text.len(),
                max_pool_bytes: 67_108_864,
            },
            &pool_control,
            &mut setup,
        )
        .map_err(|_| "source pool configuration refused")?;
    owner
        .configure_source_search(
            SourceSearchLimitsV1 {
                max_results: 1,
                max_issued: count,
                max_request_bytes: 8192,
                max_capture_bytes: 4_194_304,
                max_reply_bytes: 4_194_304,
                max_pool_bytes: 67_108_864,
                max_query_bytes: 128,
                max_scope_ranges: 2,
                max_comparisons: text.len() * QUERY.len() + 1,
                max_matches: expected.len() + 1,
                max_replacement_bytes: 0,
            },
            &pool_control,
            &mut setup,
        )
        .map_err(|_| "search pool configuration refused")?;
    let scope = SourceReadScopeV1 {
        document: version.document.clone(),
        context: SettingsExecutionContextV1 {
            authority: "experiment".into(),
            engine: "local".into(),
            recipient: "native-host".into(),
            purpose: "search-experiment".into(),
            destination: "local".into(),
            profile: "source-read/1".into(),
            continuity: "one-run".into(),
        },
        policy: "synthetic-only".into(),
    };
    let sf = TrustedSourceReadFactsV1 {
        scope: &scope,
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        retained: Some(true),
        source_readable: Some(true),
        resources_ready: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
    };
    let source = owner
        .capture_source_read_context(&scope, &sf, &mut setup)
        .ok_or("source context refused")?;
    let split = (w.repeats / 2) * w.unit.len();
    let request = SearchRequest {
        query: QUERY.into(),
        scope: if w.captured {
            SearchScope::Captured(vec![
                ByteRange {
                    start: split,
                    end: text.len(),
                },
                ByteRange {
                    start: 0,
                    end: split,
                },
            ])
        } else {
            SearchScope::Document
        },
    };
    let qf = TrustedSourceSearchFactsV1 {
        request: &request,
        grant: Some(true),
        scope: Some(true),
        continuity: Some(true),
        capacity: Some(true),
        retained: Some(true),
        not_stopped: Some(true),
    };
    let control = SourceReadContextControlV1 {
        scope: &scope,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    let generation = owner.generation();
    let mut r = mode.recorder();
    let mut samples: [Vec<u128>; 5] = std::array::from_fn(|_| Vec::with_capacity(config.samples));
    let mut measured = Instant::now();
    let mut dropped_before = 0;
    let mut timed_records = 0;
    for iteration in 0..count {
        if iteration == config.warmup {
            measured = Instant::now();
            dropped_before = r.dropped();
        }
        let start = Instant::now();
        let result = owner
            .query_source_search(
                &source,
                &scope,
                std::hint::black_box(&request),
                &sf,
                &qf,
                &mut r,
            )
            .ok_or("evaluation refused; cohort incomplete")?;
        let evaluation = start.elapsed().as_nanos();
        let release_start = Instant::now();
        let reply = owner
            .release_source_search(&result, &scope, &sf, &qf, &mut r)
            .ok_or("release refused; cohort incomplete")?;
        if reply.status != SearchStatus::Complete
            || reply.version != version
            || reply.matches != expected
        {
            return Err("partial or incorrect search; cohort incomplete");
        }
        let release = release_start.elapsed().as_nanos();
        let navigation_start = Instant::now();
        let navigation = owner
            .navigate_source_search(
                &result,
                &scope,
                SourceSearchNavigationV1 {
                    direction: SearchDirection::Next,
                    start: 0,
                    current: None,
                    wrap: false,
                },
                &sf,
                &qf,
                &mut r,
            )
            .ok_or("navigation refused; cohort incomplete")?;
        if navigation.version != version
            || navigation
                .found
                .as_ref()
                .is_none_or(|found| found.range != expected[0] || found.wrapped)
        {
            return Err("incorrect navigation; cohort incomplete");
        }
        let navigation_time = navigation_start.elapsed().as_nanos();
        drop(navigation);
        drop(reply);
        let retirement_start = Instant::now();
        if !owner.retire_source_search_result(&result, &control, &mut r)
            || owner.source_read_context_usage().holds != 0
            || owner.source_search_usage().results != 0
            || owner.source_search_usage().issued_lifetime != iteration + 1
            || owner.generation() != generation
        {
            return Err("retirement/hold/issuance/root invariant failed");
        }
        let retirement = retirement_start.elapsed().as_nanos();
        let total = start.elapsed().as_nanos();
        if iteration >= config.warmup {
            for (samples, duration) in
                samples
                    .iter_mut()
                    .zip([total, evaluation, release, navigation_time, retirement])
            {
                samples.push(duration);
            }
        }
        if !matches!(mode, Mode::Saturated) {
            while let Some(record) = r.pop_front() {
                if iteration >= config.warmup && record.service_time.is_some() {
                    timed_records += 1;
                }
            }
        }
    }
    let elapsed_ns = measured.elapsed().as_nanos();
    let dropped = r.dropped() - dropped_before;
    let issued = owner.source_search_usage().issued_lifetime;
    if !owner
        .retire_source_search(&pool_control, &mut setup)
        .map_err(|_| "search pool retirement refused")?
        || !owner.stop_source_read_context(&source, &control, &mut setup)
        || !owner.retire_source_read_context(&source, &control, &mut setup)
        || !owner
            .retire_source_read_contexts(&pool_control, &mut setup)
            .map_err(|_| "source pool retirement refused")?
        || owner.retention_usage().reserved != 0
    {
        return Err("final protected cleanup left parent reservation");
    }
    let mut phases = Vec::with_capacity(5);
    for values in &mut samples {
        phases.push(Distribution::from_samples(values).ok_or("empty latency population")?);
    }
    Ok(Report {
        phases: phases
            .try_into()
            .map_err(|_| "incorrect phase population")?,
        bytes: text.len(),
        matches: expected.len(),
        elapsed_ns,
        timed_records,
        dropped,
        issued,
    })
}
fn main() {
    let entered = Instant::now();
    println!("# entered-main; excludes launch/pre-main");
    let result = execute();
    println!("# in_process_elapsed_ns={}", entered.elapsed().as_nanos());
    if let Err(error) = result {
        eprintln!("search_responsiveness: {error}");
        std::process::exit(1);
    }
}
fn execute() -> Result<(), String> {
    let config = Config::parse(std::env::args().skip(1))?;
    iterations(config)?;
    if cfg!(debug_assertions) {
        return Err("run this gate with --release".into());
    }
    println!(
        "# native-retained-search-v1; synthetic Unicode; exact UTF-8; trusted local owner; process monotonic ns"
    );
    println!(
        "# samples={}; warmup={}; p99_budget_ns={}; arch={}; os={}",
        config.samples,
        config.warmup,
        config.max_p99_ns.map_or("unset".into(), |n| n.to_string()),
        std::env::consts::ARCH,
        std::env::consts::OS
    );
    println!(
        "# total=evaluate+release/check+navigation/check+protected-retirement/check; no ingress/queue/UI/I/O"
    );
    println!(
        "# cohort includes drain/bookkeeping; allocations/RSS unmeasured; lifetime pressure grows without reset; refused/partial/incomplete work fails"
    );
    println!(
        "workload,bytes,matches,mode,phase,count,p50_ns,p95_ns,p99_ns,max_ns,cohort_elapsed_ns,timed_records,diagnostic_drops,issued_including_warmup"
    );
    let mut failed = false;
    for w in WORKLOADS {
        for mode in [Mode::Off, Mode::Capture, Mode::Timed, Mode::Saturated] {
            let report = run(w, mode, config)?;
            failed |= !config.accepts_total(report.phases[0]);
            for (phase, d) in [
                "total",
                "evaluate",
                "release-check",
                "navigation-check",
                "retirement-check",
            ]
            .into_iter()
            .zip(report.phases)
            {
                println!(
                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                    w.name,
                    report.bytes,
                    report.matches,
                    mode.name(),
                    phase,
                    d.count,
                    d.p50,
                    d.p95,
                    d.p99,
                    d.maximum,
                    report.elapsed_ns,
                    report.timed_records,
                    report.dropped,
                    report.issued
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
    fn unicode_native_search_cohorts_keep_all_samples_and_cleanup_under_every_capture_mode() {
        for captured in [false, true] {
            let w = Workload {
                name: "test",
                unit: "café needleΩ 🙂\r\n",
                repeats: 4,
                captured,
            };
            let config = Config {
                samples: 3,
                warmup: 2,
                max_p99_ns: None,
            };
            for mode in [Mode::Off, Mode::Capture, Mode::Timed, Mode::Saturated] {
                let report = run(w, mode, config).unwrap();
                assert_eq!(report.matches, 4);
                assert_eq!(report.bytes, w.unit.len() * 4);
                assert!(report.phases.iter().all(|d| d.count == 3));
                assert_eq!(report.issued, 5);
                assert_eq!(
                    report.timed_records,
                    if matches!(mode, Mode::Timed) { 12 } else { 0 }
                );
                if matches!(mode, Mode::Saturated) {
                    assert!(report.dropped >= 12);
                } else {
                    assert_eq!(report.dropped, 0);
                }
            }
        }
    }
    #[test]
    fn impossible_or_excessive_lifetime_is_refused_before_work_and_empty_samples_do_not_pass() {
        for (samples, warmup) in [(0, 0), (4097, 0), (usize::MAX, 1)] {
            assert!(
                iterations(Config {
                    samples,
                    warmup,
                    max_p99_ns: None
                })
                .is_err()
            );
        }
        assert!(
            run(
                WORKLOADS[0],
                Mode::Off,
                Config {
                    samples: 0,
                    warmup: 1,
                    max_p99_ns: None
                }
            )
            .is_err()
        );
    }
}
