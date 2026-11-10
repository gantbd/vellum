//! Actual sampled-publication/event facade workload; no UI, I/O or scheduler.
#[path = "responsiveness/mod.rs"]
mod support;
use std::time::Instant;
use support::{Config, Distribution};
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{
    Boundary, Disposition, NativeTimingProfileV1, Observation, Recorder,
};
use vellum_engine::source::{Limits, Replacement};

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
            Self::Off | Self::Capture => NativeTimingProfileV1::Off,
            _ => NativeTimingProfileV1::ProcessMonotonicV1,
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
    phases: [Distribution; 10],
    event_records: usize,
    timed_event_records: usize,
    drops: u64,
    reserved_bytes: usize,
    command_records: usize,
    timed_command_records: usize,
}
fn run(bytes: usize, mode: Mode, commands: bool, config: Config) -> Result<Report, String> {
    let fail = |stage| format!("observation workload failed: {stage}");
    let total = config
        .samples
        .checked_add(config.warmup)
        .ok_or_else(|| fail("iterations"))?;
    let mut setup = Recorder::disabled();
    let mut owner = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 1,
        max_participants: 1,
        max_staged_bytes: bytes,
        source: Limits {
            max_source_bytes: bytes,
            max_edits: 1,
            max_edit_payload_bytes: 2,
        },
    });
    let mut version = owner
        .create(&vec![b'a'; bytes], &mut setup)
        .map_err(|_| fail("create"))?;
    let scope = SourceReadScopeV1 {
        document: version.document.clone(),
        policy: "workload".into(),
        context: SettingsExecutionContextV1 {
            authority: "local".into(),
            engine: "engine".into(),
            recipient: "host".into(),
            purpose: "workload".into(),
            destination: "memory".into(),
            profile: "native-source-publication".into(),
            continuity: "local".into(),
        },
    };
    let native = TrustedSourceReadFactsV1 {
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
    let facts = TrustedSourcePublicationFactsV1 {
        original_scope: &scope,
        current: &native,
        projection_allowed: Some(true),
        baseline_allowed: Some(true),
        max_reply_bytes: bytes + 2048,
    };
    let mut limits = SourcePublicationLimitsV1 {
        max_streams: 1,
        max_issued: total
            .checked_mul(if commands { 2 } else { 1 })
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| fail("issuance"))?,
        max_context_bytes: 2048,
        max_projection_bytes: bytes + 1024,
        max_reply_bytes: bytes + 2048,
        max_queue: 1,
        max_pool_bytes: usize::MAX,
    };
    let mut reserved_bytes = limits
        .required_pool_bytes()
        .map_err(|_| fail("pool calculation"))?
        .checked_add("local".len() + "engine".len())
        .ok_or_else(|| fail("pool identity charge"))?;
    limits.max_pool_bytes = reserved_bytes;
    let mut command_limits = SourcePublicationCommandLimitsV1 {
        max_commands: 1,
        max_issued: total,
        max_context_bytes: 2048,
        max_result_bytes: bytes
            .checked_add(4096)
            .ok_or_else(|| fail("result bytes"))?,
        max_pool_bytes: usize::MAX,
    };
    if commands {
        command_limits.max_pool_bytes = command_limits
            .required_pool_bytes()
            .map_err(|_| fail("command pool calculation"))?;
        reserved_bytes = reserved_bytes
            .checked_add(command_limits.max_pool_bytes)
            .ok_or_else(|| fail("parent bytes"))?;
    }
    owner
        .configure_retention_budget(reserved_bytes, &mut setup)
        .map_err(|_| fail("parent"))?;
    let pool_control = SourceReadPoolControlV1 {
        owner_control_allowed: Some(true),
        resources_ready: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    owner
        .configure_source_publication_streams_v1(
            "local",
            "engine",
            limits,
            &pool_control,
            &mut setup,
        )
        .map_err(|_| fail("configure"))?;
    if commands {
        owner
            .configure_source_publication_commands_v1(command_limits, &pool_control, &mut setup)
            .map_err(|_| fail("command configure"))?;
    }
    let baseline = owner
        .open_source_publication_stream_v1(&scope, &facts, &mut setup)
        .map_err(|_| fail("open"))?;
    let stream = baseline.stream.clone();
    let mut generation = baseline.generation.clone();
    drop(baseline);
    let mut r = mode.recorder();
    let mut samples: [Vec<u128>; 10] = std::array::from_fn(|_| Vec::with_capacity(config.samples));
    let mut drops_before = 0;
    let mut event_records = 0;
    let mut timed_event_records = 0;
    let mut command_counts = [0usize; 4];
    let mut timed_command_records = 0;
    for i in 0..total {
        if i == config.warmup {
            drops_before = r.dropped();
        }
        // Authored edit setup is deliberately outside observation service timing.
        let (from, to) = if i % 2 == 0 { ("a", "b") } else { ("b", "a") };
        let prepared = owner
            .prepare(
                &[DocumentEdits {
                    expected: version.clone(),
                    edits: vec![Replacement {
                        start: 0,
                        end: 1,
                        expected: from.into(),
                        inserted: to.into(),
                    }],
                }],
                &[],
                &mut setup,
            )
            .map_err(|_| fail("prepare edit"))?;
        let committed = owner
            .commit(prepared, &mut setup)
            .map_err(|_| fail("commit edit"))?;
        version = committed.changes[0].after.clone();
        let publication = owner.generation();
        let started = Instant::now();
        let sampled = owner
            .sample_source_publication_stream_v1(&stream, &scope, &facts, &mut r)
            .map_err(|_| fail("sample"))?;
        let sample_ns = started.elapsed().as_nanos();
        let release_start = Instant::now();
        let delivered = owner
            .release_observation_event_v1(
                ObservationEventTargetV1::Source(&stream),
                ObservationEventPortV1::Source {
                    stream: &stream,
                    scope: &scope,
                    facts: &facts,
                },
                &mut r,
            )
            .map_err(|_| fail("release"))?;
        let release_ns = release_start.elapsed().as_nanos();
        let ObservationEventDeliveryV1::Source(SourcePublicationDeliveryV1::Projection(event)) =
            delivered
        else {
            return Err(fail("missing event"));
        };
        if sampled != SourcePublicationSampleV1::Queued
            || event.before != generation
            || event.after == generation
            || *event.projection.version() != version
            || event.projection.text().len() != bytes
            || &event.projection.text()[..1] != to
            || !event.projection.text().as_bytes()[1..]
                .iter()
                .all(|b| *b == b'a')
        {
            return Err(fail("event oracle"));
        }
        generation = event.after;
        let empty_start = Instant::now();
        if !matches!(
            owner.release_observation_event_v1(
                ObservationEventTargetV1::Source(&stream),
                ObservationEventPortV1::Source {
                    stream: &stream,
                    scope: &scope,
                    facts: &facts
                },
                &mut r,
            ),
            Ok(ObservationEventDeliveryV1::Source(
                SourcePublicationDeliveryV1::Empty
            ))
        ) {
            return Err(fail("empty oracle"));
        }
        let empty_ns = empty_start.elapsed().as_nanos();
        let unchanged_start = Instant::now();
        if owner
            .sample_source_publication_stream_v1(&stream, &scope, &facts, &mut r)
            .map_err(|_| fail("unchanged"))?
            != SourcePublicationSampleV1::Unchanged
        {
            return Err(fail("unchanged oracle"));
        }
        let unchanged_ns = unchanged_start.elapsed().as_nanos();
        let mut command_ns = [0u128; 5];
        if commands {
            let op = SourcePublicationCommandOperationV1::Resynchronize {
                stream: stream.clone(),
            };
            let start = Instant::now();
            let id = owner
                .issue_source_publication_command_v1(&op, &scope, &facts, Some(true), &mut r)
                .map_err(|_| fail("command admission"))?;
            command_ns[0] = start.elapsed().as_nanos();
            let request = SourcePublicationCommandRequestV1 {
                scope: &scope,
                operation: &op,
                current: Some(&facts),
                owner_control_allowed: Some(true),
                max_reply_bytes: bytes + 2048,
            };
            let start = Instant::now();
            if !matches!(owner.request_source_publication_command_v1(&id, &request, &mut r),
                Ok(SourcePublicationCommandReplyV1::Settled { command }) if command == id)
            {
                return Err(fail("command execution"));
            }
            command_ns[1] = start.elapsed().as_nanos();
            let start = Instant::now();
            let result = owner
                .source_publication_command_facts_v1(&id, &scope, &mut r)
                .map_err(|_| fail("command inspection"))?
                .as_observed
                .ok_or_else(|| fail("missing command outcome"))?
                .as_ref()
                .map_err(|_| fail("command native failure"))?
                .clone();
            if result.stream != stream
                || *result.projection.version() != version
                || result.projection.text().len() != bytes
                || &result.projection.text()[..1] != to
                || !result.projection.text().as_bytes()[1..]
                    .iter()
                    .all(|b| *b == b'a')
            {
                return Err(fail("command result oracle"));
            }
            generation = result.generation.clone();
            command_ns[2] = start.elapsed().as_nanos();
            let start = Instant::now();
            let retry = SourcePublicationCommandRequestV1 {
                current: None,
                ..request
            };
            if !matches!(owner.request_source_publication_command_v1(&id, &retry, &mut r),
                Ok(SourcePublicationCommandReplyV1::ObserveExisting { command }) if command == id)
            {
                return Err(fail("command replay"));
            }
            let replay = owner
                .source_publication_command_facts_v1(&id, &scope, &mut r)
                .map_err(|_| fail("replay inspection"))?
                .as_observed
                .ok_or_else(|| fail("missing replay outcome"))?
                .as_ref()
                .map_err(|_| fail("replay native failure"))?;
            if !std::sync::Arc::ptr_eq(&result, replay) {
                return Err(fail("replay renewed result"));
            }
            command_ns[3] = start.elapsed().as_nanos();
            drop(result);
            let start = Instant::now();
            owner
                .retire_source_publication_command_v1(
                    &id,
                    &SourceReadContextControlV1 {
                        scope: &scope,
                        owner_control_allowed: Some(true),
                        retention_released: Some(true),
                        delivery_drained: Some(true),
                    },
                    &mut r,
                )
                .map_err(|_| fail("command retirement"))?;
            command_ns[4] = start.elapsed().as_nanos();
            if owner
                .source_publication_command_usage_v1()
                .map_err(|_| fail("command usage"))?
                .commands
                != 0
            {
                return Err(fail("command retained after retirement"));
            }
        }
        let total_ns = started.elapsed().as_nanos();
        if owner.generation() != publication
            || owner.retention_usage().reserved != reserved_bytes
            || owner
                .source_publication_usage_v1()
                .map_err(|_| fail("usage"))?
                .queued
                != 0
        {
            return Err(fail("ownership changed"));
        }
        if i >= config.warmup {
            for (out, value) in samples.iter_mut().zip(
                [total_ns, sample_ns, release_ns, empty_ns, unchanged_ns]
                    .into_iter()
                    .chain(command_ns),
            ) {
                out.push(value);
            }
        }
        if !matches!(mode, Mode::Saturated) {
            while let Some(record) = r.pop_front() {
                if i >= config.warmup && record.boundary == Boundary::ObservationEventDelivery {
                    event_records += 1;
                    timed_event_records += usize::from(record.service_time.is_some());
                }
                let command_index = match record.boundary {
                    Boundary::SourcePublicationCommandIssuance => Some(0),
                    Boundary::SourcePublicationCommandRequest => Some(1),
                    Boundary::SourcePublicationCommandInspection => Some(2),
                    Boundary::SourcePublicationCommandRetirement => Some(3),
                    _ => None,
                };
                if let Some(index) = command_index {
                    if record.disposition != Disposition::Completed
                        || record.service_time.is_some() != matches!(mode, Mode::Timed)
                    {
                        return Err(fail("command service status/timing"));
                    }
                    if i >= config.warmup {
                        command_counts[index] += 1;
                        timed_command_records += usize::from(record.service_time.is_some());
                    }
                }
            }
        }
    }
    let cleanup = SourceReadContextControlV1 {
        scope: &scope,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    if commands {
        owner
            .retire_source_publication_commands_v1(&pool_control, &mut setup)
            .map_err(|_| fail("command pool retire"))?;
    }
    owner
        .stop_source_publication_stream_v1(&stream, &cleanup, &mut setup)
        .map_err(|_| fail("stop"))?;
    owner
        .retire_source_publication_stream_v1(&stream, &cleanup, &mut setup)
        .map_err(|_| fail("retire"))?;
    owner
        .retire_source_publication_pool_v1(&pool_control, &mut setup)
        .map_err(|_| fail("pool retire"))?;
    if owner.retention_usage().reserved != 0 {
        return Err(fail("parent not discharged"));
    }
    let drops = r.dropped() - drops_before;
    let expected_records = if matches!(mode, Mode::Capture | Mode::Timed) {
        2 * config.samples
    } else {
        0
    };
    let expected_timed = if matches!(mode, Mode::Timed) {
        2 * config.samples
    } else {
        0
    };
    if event_records != expected_records
        || timed_event_records != expected_timed
        || (drops > 0) != matches!(mode, Mode::Saturated)
    {
        return Err(fail("diagnostic population"));
    }
    let expected_commands = if commands && matches!(mode, Mode::Capture | Mode::Timed) {
        [
            config.samples,
            2 * config.samples,
            2 * config.samples,
            config.samples,
        ]
    } else {
        [0; 4]
    };
    if command_counts != expected_commands
        || timed_command_records
            != if matches!(mode, Mode::Timed) {
                expected_commands.iter().sum()
            } else {
                0
            }
    {
        return Err(fail("command diagnostic population"));
    }
    Ok(Report {
        phases: samples.map(|mut s| Distribution::from_samples(&mut s).unwrap()),
        event_records,
        timed_event_records,
        drops,
        reserved_bytes,
        command_records: command_counts.iter().sum(),
        timed_command_records,
    })
}
fn execute() -> Result<(), String> {
    let config = Config::parse(std::env::args().skip(1))?;
    if cfg!(debug_assertions) {
        return Err("run with --release".into());
    }
    println!(
        "# observation-event-v2; source-only actual bound facade with raw and enabled-command profiles; process monotonic ns"
    );
    println!(
        "# total=sample+event+empty+unchanged+payload oracle, plus command phases when enabled; edit/setup/pool cleanup excluded; command result inspection is privileged, not recipient F06; no queue wait, UI or I/O"
    );
    println!(
        "# allocations/RSS unmeasured; logical parent charge reported; no unspecified latency budget"
    );
    println!(
        "bytes,profile,mode,phase,count,p50_ns,p95_ns,p99_ns,max_ns,event_records,timed_event_records,command_records,timed_command_records,diagnostic_drops,reserved_bytes"
    );
    let mut failed = false;
    for bytes in [4096, 65536, 1048576] {
        for commands in [false, true] {
            for mode in [Mode::Off, Mode::Capture, Mode::Timed, Mode::Saturated] {
                let report = run(bytes, mode, commands, config)?;
                failed |= !config.accepts_total(report.phases[0]);
                for (phase, d) in [
                    "total",
                    "sample",
                    "event",
                    "empty",
                    "unchanged",
                    "command_admission",
                    "command_execute",
                    "command_inspection_oracle",
                    "command_replay_inspection",
                    "command_retirement",
                ]
                .into_iter()
                .zip(report.phases)
                .take(if commands { 10 } else { 5 })
                {
                    println!(
                        "{bytes},{},{},{phase},{},{},{},{},{},{},{},{},{},{},{}",
                        if commands { "commands" } else { "raw" },
                        mode.name(),
                        d.count,
                        d.p50,
                        d.p95,
                        d.p99,
                        d.maximum,
                        report.event_records,
                        report.timed_event_records,
                        report.command_records,
                        report.timed_command_records,
                        report.drops,
                        report.reserved_bytes
                    );
                }
            }
        }
    }
    if failed {
        Err("explicit total p99 budget exceeded".into())
    } else {
        Ok(())
    }
}
fn main() {
    if let Err(error) = execute() {
        eprintln!("observation-responsiveness: {error}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_events_keep_complete_population_and_cleanup_under_capture_pressure() {
        let config = Config {
            samples: 4,
            warmup: 2,
            max_p99_ns: None,
        };
        for mode in [Mode::Off, Mode::Capture, Mode::Timed, Mode::Saturated] {
            let report = run(64, mode, false, config).unwrap();
            assert!(report.phases.iter().all(|d| d.count == 4));
            assert_eq!(
                report.event_records,
                if matches!(mode, Mode::Capture | Mode::Timed) {
                    8
                } else {
                    0
                }
            );
            assert_eq!(
                report.timed_event_records,
                if matches!(mode, Mode::Timed) { 8 } else { 0 }
            );
            assert_eq!(report.drops > 0, matches!(mode, Mode::Saturated));
        }
    }

    #[test]
    fn enabled_commands_keep_original_results_replay_population_and_full_cleanup() {
        let config = Config {
            samples: 4,
            warmup: 2,
            max_p99_ns: None,
        };
        for mode in [Mode::Off, Mode::Capture, Mode::Timed, Mode::Saturated] {
            let report = run(64, mode, true, config).unwrap();
            assert!(report.phases.iter().all(|d| d.count == 4));
            assert_eq!(
                report.command_records,
                if matches!(mode, Mode::Capture | Mode::Timed) {
                    24
                } else {
                    0
                }
            );
            assert_eq!(
                report.timed_command_records,
                if matches!(mode, Mode::Timed) { 24 } else { 0 }
            );
            assert_eq!(report.drops > 0, matches!(mode, Mode::Saturated));
        }
    }
}
