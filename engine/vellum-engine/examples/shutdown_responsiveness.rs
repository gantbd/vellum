//! Actual multi-document shutdown work, not an empty-root or repeat-stop benchmark.
#[path = "responsiveness/mod.rs"]
mod support;
use std::time::Instant;
use support::{Config, Distribution};
use vellum_engine::capture::InertValueV1 as D;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{
    Boundary, Disposition, NativeTimingProfileV1, Observation, Recorder,
};

macro_rules! checked {
    ($phase:literal, $expression:expr) => {
        $expression.map_err(|e| format!("{}: {e:?}", $phase))?
    };
}
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
        match self {
            Self::Off => Recorder::disabled(),
            Self::Capture => Recorder::try_with_capacity(1024).unwrap(),
            Self::Timed => Recorder::try_with_capacity(1024)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1),
            Self::Saturated => {
                let mut r = Recorder::try_with_capacity(1)
                    .unwrap()
                    .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
                r.record(Observation {
                    boundary: Boundary::Control,
                    disposition: Disposition::Completed,
                    service_time: None,
                });
                r
            }
        }
    }
}
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "host".into(),
        engine: "native".into(),
        recipient: "host".into(),
        purpose: "shutdown".into(),
        destination: "host".into(),
        profile: "native".into(),
        continuity: "local".into(),
    }
}
fn control(c: &SettingsExecutionContextV1) -> TrustedShutdownFenceFactsV1<'_> {
    TrustedShutdownFenceFactsV1 {
        context: c,
        authorized: Some(true),
        scope_current: Some(true),
        continuous: Some(true),
        owner_control_allowed: Some(true),
        admission_fence_allowed: Some(true),
        resources_ready: Some(true),
    }
}
fn limits() -> ShutdownInventoryLimitsV1 {
    ShutdownInventoryLimitsV1 {
        max_documents: 64,
        max_writes: 0,
        max_artifacts: 0,
        max_bindings: 0,
        max_reads: 0,
        max_owners: 64,
        max_attachments: 0,
        max_dependencies: 128,
        max_work: 8192,
        max_capture_bytes: 262144,
    }
}
fn coverage() -> TrustedDisposalCoverageV1 {
    TrustedDisposalCoverageV1 {
        not_stopped: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        lifecycle_allowed: Some(true),
        coverage_complete: Some(true),
        owner_control_allowed: Some(true),
        source_retention_ready: None,
        source_discard_allowed: Some(true),
        annotation_retention_ready: None,
        annotation_discard_allowed: Some(true),
        retention_complete: Some(true),
        effects_retained: Some(true),
        history_retained: Some(true),
        snapshots_retained: Some(true),
        providers_retained: Some(true),
        views_detachable: Some(true),
        memberships_detachable: Some(true),
        observer_scopes_fenced: Some(true),
        binding_coverage_complete: Some(true),
    }
}
fn setup(
    n: usize,
    c: &SettingsExecutionContextV1,
) -> Result<(SourceCoordinator, ShutdownFenceIdV1, Vec<DocumentVersion>), String> {
    let mut r = Recorder::disabled();
    let mut o = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: n,
        max_participants: n,
        max_staged_bytes: 16384,
        source: vellum_engine::source::Limits {
            max_source_bytes: 128,
            max_edits: 4,
            max_edit_payload_bytes: 128,
        },
    });
    let mut documents = Vec::with_capacity(n);
    for i in 0..n {
        let text = if i % 2 == 0 {
            "fn value() -> u8 { 42 }\n"
        } else {
            "# Note\nProse: Grüße, κόσμε.\n"
        };
        documents.push(checked!("create", o.create(text.as_bytes(), &mut r)));
    }
    checked!("parent", o.configure_retention_budget(8 * 1048576, &mut r));
    let control = control(c);
    let auth = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let basis = checked!(
        "basis",
        o.capture_shutdown_fence_basis_v1(
            ShutdownFenceLimitsV1 {
                max_context_bytes: 1024,
                max_root_bytes: 262144,
                max_capture_bytes: 1048576,
            },
            &control,
            &mut r
        )
    );
    let inventory = checked!(
        "inventory",
        o.capture_shutdown_inventory_v1(&basis, limits(), &auth, &mut r)
    );
    let rows = checked!(
        "inventory release",
        o.read_shutdown_inventory_v1(&inventory, &auth, &mut r)
    );
    let choices: Vec<_> = rows
        .documents
        .iter()
        .map(|row| ShutdownDocumentDecisionV1 {
            expected: row,
            source: Some(ShutdownDocumentChoiceV1::Discard),
            annotations: Some(ShutdownDocumentChoiceV1::Discard),
        })
        .collect();
    let ShutdownDecisionCaptureV1::Captured(decisions) = checked!(
        "consent",
        o.capture_shutdown_decisions_v1(&inventory, &choices, 131072, &auth, &mut r)
    ) else {
        return Err("missing explicit consent".into());
    };
    let bindings: Vec<_> = documents
        .iter()
        .map(|d| ShutdownSaveBindingsV1 {
            document: d.document.clone(),
            source: None,
            annotations: None,
        })
        .collect();
    let ShutdownAcceptanceV1::Accepted(fence) = checked!(
        "accept",
        o.accept_shutdown_disposition_v1(&inventory, &decisions, &bindings, 1048576, &auth, &mut r)
    ) else {
        return Err("disposition not accepted".into());
    };
    Ok((o, fence, documents))
}
fn decoded(
    o: &mut SourceCoordinator,
    fence: &ShutdownFenceIdV1,
    d: &DocumentId,
    auth: &TrustedShutdownInventoryFactsV1<'_>,
    r: &mut Recorder,
) -> Result<LifecycleState, ShutdownDispositionIngressErrorV1> {
    use ShutdownDispositionEnvelopeValueV1 as V;
    let args = [
        (
            "contract",
            V::Data(D::Text(SHUTDOWN_DISPOSITION_PROFILE_V1)),
        ),
        ("operation", V::Data(D::Text("shutdown-disposition"))),
        ("fence", V::Fence(fence)),
        ("document", V::Document(d)),
    ];
    let header = [
        ("family", V::Data(D::Number(17.0))),
        ("variant", V::Data(D::Text("shutdown-disposition"))),
        ("version", V::Data(D::Number(1.0))),
        ("arguments", V::Object(&args)),
    ];
    let outer = [
        ("route", V::Data(D::Text("typed"))),
        ("request", V::Object(&header)),
    ];
    o.request_decoded_shutdown_disposition_v1(&V::Object(&outer), auth, &coverage(), r)
}
struct Report {
    phases: [Distribution; 8],
    records: usize,
    timed_records: usize,
    drops: u64,
    checkpoint_bytes: usize,
}
fn run(n: usize, use_decoder: bool, mode: Mode, config: Config) -> Result<Report, String> {
    let total = config
        .samples
        .checked_add(config.warmup)
        .ok_or("sample overflow")?;
    if !(1..=64).contains(&n) || config.samples == 0 || total > 4096 {
        return Err("experiment requires 1..64 documents and 1..4096 iterations".into());
    }
    let c = context();
    let control = control(&c);
    let auth = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let mut samples: [Vec<u128>; 8] = std::array::from_fn(|_| Vec::with_capacity(config.samples));
    let (mut records, mut timed_records, mut drops, mut checkpoint_bytes) = (0, 0, 0u64, 0);
    for iteration in 0..total {
        let (mut o, fence, documents) = setup(n, &c)?;
        let mut r = mode.recorder();
        let mut timings = [0u128; 8];
        checkpoint_bytes = checkpoint_bytes.max(o.retention_usage().reserved);
        let all = Instant::now();
        let start = Instant::now();
        for d in &documents {
            let state = if use_decoder {
                checked!(
                    "decoded disposition",
                    decoded(&mut o, &fence, &d.document, &auth, &mut r)
                )
            } else {
                checked!(
                    "disposition",
                    o.dispose_accepted_shutdown_document_v1(
                        &fence,
                        &d.document,
                        &auth,
                        &coverage(),
                        &mut r
                    )
                )
            };
            if state.phase != LifecyclePhase::DisposedRetained {
                return Err("not disposed".into());
            }
        }
        timings[1] = start.elapsed().as_nanos();
        let start = Instant::now();
        for d in &documents {
            let state = checked!("lifecycle", o.lifecycle_state(&d.document, &mut r));
            checked!(
                "provenance",
                o.release_disposed_provenance(&d.document, state, &mut r)
            );
            let state = checked!("lifecycle", o.lifecycle_state(&d.document, &mut r));
            checked!(
                "reclaim",
                o.reclaim_disposed_document_v1(
                    &d.document,
                    state,
                    &TrustedDocumentReclamationV1 {
                        authorized: Some(true),
                        authored_disposition_ready: Some(true),
                        session_references_released: Some(true),
                        retention_released: Some(true),
                        delivery_drained: Some(true),
                    },
                    &mut r
                )
            );
        }
        timings[2] = start.elapsed().as_nanos();
        let start = Instant::now();
        checked!(
            "plan retirement",
            o.retire_shutdown_disposition_v1(&fence, &control, Some(true), Some(true), &mut r)
        );
        timings[3] = start.elapsed().as_nanos();
        let start = Instant::now();
        if checked!(
            "report return",
            o.finish_shutdown_attempt_v1(&fence, limits(), &auth, &mut r)
        ) != ShutdownAttemptStatusV1::Drained
        {
            return Err("unresolved work".into());
        }
        timings[4] = start.elapsed().as_nanos();
        checkpoint_bytes = checkpoint_bytes.max(o.retention_usage().reserved);
        let start = Instant::now();
        let (status, report) = checked!(
            "report read",
            o.read_shutdown_attempt_report_v1(&fence, &auth, &mut r)
        );
        if status != ShutdownAttemptStatusV1::Drained
            || !report.coverage.all_owner_obligations
            || !report.documents.is_empty()
            || report.funding.unrepresented_parent_bytes != 0
        {
            return Err("incomplete returned census".into());
        }
        timings[5] = start.elapsed().as_nanos();
        let start = Instant::now();
        checked!(
            "report retirement",
            o.retire_shutdown_attempt_report_v1(&fence, &control, Some(true), Some(true), &mut r)
        );
        timings[6] = start.elapsed().as_nanos();
        let start = Instant::now();
        checked!(
            "control retirement",
            o.retire_shutdown_control_v1(&fence, limits(), &auth, &mut r)
        );
        timings[7] = start.elapsed().as_nanos();
        timings[0] = all.elapsed().as_nanos();
        if o.retention_usage().reserved != 0 || o.is_faulted() {
            return Err("cleanup funding/fault".into());
        }
        let mut quiet = Recorder::disabled();
        if o.create(b"closed", &mut quiet) != Err(CoordinatorError::Unavailable) {
            return Err("closed engine reopened".into());
        }
        let mut counts = [0usize; 8];
        let mut timed = 0;
        while let Some(event) = r.pop_front() {
            let index = match event.boundary {
                Boundary::ShutdownAcceptedDocumentDisposition => Some(0),
                Boundary::DocumentReclamation => Some(1),
                Boundary::ShutdownDispositionRetirement => Some(2),
                Boundary::ShutdownAttemptReturn => Some(3),
                Boundary::ShutdownAttemptRelease => Some(4),
                Boundary::ShutdownAttemptRetirement => Some(5),
                Boundary::ShutdownControlRetirement => Some(6),
                Boundary::ShutdownDispositionArgumentNormalization => Some(7),
                _ => None,
            };
            if let Some(i) = index {
                if event.disposition != Disposition::Completed
                    || event.service_time.is_some() != matches!(mode, Mode::Timed)
                {
                    return Err("native service population/status".into());
                }
                counts[i] += 1;
                timed += usize::from(event.service_time.is_some());
            }
        }
        let expected = if matches!(mode, Mode::Capture | Mode::Timed) {
            [n, n, 1, 1, 1, 1, 1, if use_decoder { n } else { 0 }]
        } else {
            [0; 8]
        };
        if counts != expected || (r.dropped() > 0) != matches!(mode, Mode::Saturated) {
            return Err("missing or hidden service population".into());
        }
        if iteration >= config.warmup {
            for (s, elapsed) in samples.iter_mut().zip(timings) {
                s.push(elapsed);
            }
            records += counts.into_iter().sum::<usize>();
            timed_records += timed;
            drops += r.dropped();
        }
    }
    Ok(Report {
        phases: samples.map(|mut s| Distribution::from_samples(&mut s).unwrap()),
        records,
        timed_records,
        drops,
        checkpoint_bytes,
    })
}
fn execute() -> Result<(), String> {
    let config = Config::parse(std::env::args().skip(1))?;
    if cfg!(debug_assertions) {
        return Err("run with --release".into());
    }
    println!(
        "# native-shutdown-v1; actual alternating code/Unicode prose documents; process monotonic ns"
    );
    println!(
        "# setup/consent, diagnostic drain and final Rust Drop excluded; reclamation includes provenance release; no workers/I/O, wait or crash-persistence claim"
    );
    println!(
        "# complete external sample population, bounded native service counts; checkpoint logical parent bytes, not allocations/RSS; no unspecified latency budget"
    );
    println!(
        "documents,route,mode,phase,count,p50_ns,p95_ns,p99_ns,max_ns,service_records,timed_service_records,diagnostic_drops,max_checkpoint_reserved_bytes"
    );
    let mut failed = false;
    for n in [1, 8, 64] {
        for use_decoder in [false, true] {
            for mode in [Mode::Off, Mode::Capture, Mode::Timed, Mode::Saturated] {
                let report = run(n, use_decoder, mode, config)?;
                failed |= !config.accepts_total(report.phases[0]);
                for (phase, d) in [
                    "total",
                    "disposition",
                    "reclamation",
                    "plan_retirement",
                    "report_return",
                    "report_read",
                    "report_retirement",
                    "control_retirement",
                ]
                .into_iter()
                .zip(report.phases)
                {
                    println!(
                        "{n},{},{},{phase},{},{},{},{},{},{},{},{},{}",
                        if use_decoder { "decoded" } else { "native" },
                        mode.name(),
                        d.count,
                        d.p50,
                        d.p95,
                        d.p99,
                        d.maximum,
                        report.records,
                        report.timed_records,
                        report.drops,
                        report.checkpoint_bytes
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
    if let Err(e) = execute() {
        eprintln!("shutdown-responsiveness: {e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_disposition_and_reclamation_preserve_population_under_capture_pressure() {
        let config = Config {
            samples: 4,
            warmup: 2,
            max_p99_ns: None,
        };
        for use_decoder in [false, true] {
            for mode in [Mode::Off, Mode::Capture, Mode::Timed, Mode::Saturated] {
                let report = run(8, use_decoder, mode, config).unwrap();
                assert!(report.phases.iter().all(|d| d.count == 4));
                let expected = if matches!(mode, Mode::Capture | Mode::Timed) {
                    4 * (16 + 5 + if use_decoder { 8 } else { 0 })
                } else {
                    0
                };
                assert_eq!(report.records, expected);
                assert_eq!(
                    report.timed_records,
                    if matches!(mode, Mode::Timed) {
                        expected
                    } else {
                        0
                    }
                );
                assert_eq!(report.drops > 0, matches!(mode, Mode::Saturated));
            }
        }
    }
}
