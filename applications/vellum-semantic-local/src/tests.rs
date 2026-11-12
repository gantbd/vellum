use std::{
    env,
    io::{Read as _, Write as _},
    os::unix::{ffi::OsStrExt as _, fs::PermissionsExt as _},
    path::PathBuf,
    sync::{Arc, Barrier, atomic::Ordering},
    thread,
    time::{Duration, Instant},
};

use tokio::io::AsyncWriteExt as _;
use vellum_adapter_semantic_lsp::{
    LspExplicitVirtualDocumentsV1, LspLimitsV1, LspNeverReadyV1, LspProviderProtocolProfileV1,
    SemanticGenerationIssuerV1,
};
use vellum_adapter_semantic_lsp::{LspReceiveOutcomeV1, LspSendOutcomeV1, LspTransportPortV1};
use vellum_host::semantic::{
    SemanticIndexGenerationV1, SemanticProviderProvenanceV1, SemanticProviderRefV1,
};
use vellum_semantic_protocol::{SemanticSessionIncarnationV1, SemanticSessionLimitsV1};
use vellum_semantic_service::{SemanticClientRefV1, SemanticServiceLimitsV1};

use super::*;

fn raw_transport_for_freshness(shared: Arc<SharedState>) -> LocalLspTransportV1 {
    let (controls, _control_rx) = mpsc::sync_channel(1);
    let (_events_tx, events) = mpsc::sync_channel(1);
    LocalLspTransportV1 {
        controls,
        events,
        prefetched: None,
        shared,
        max_frame_bytes: 1024,
        max_queued_bytes: 1024,
    }
}

#[test]
fn transport_only_cleanup_is_forced_idempotent_and_content_safe() {
    let shared = Arc::new(SharedState::default());
    shared.live.store(true, Ordering::Release);
    let failure = LocalLspFreshTransportFailureV1 {
        reason: LocalLspFreshTransportRefusalV1::BufferedOutput,
        transport: raw_transport_for_freshness(Arc::clone(&shared)),
    };
    let mut owner = LocalSupervisedTransportCleanupOwnerV1::new(
        failure,
        SupervisedGenerations(0),
        LspNeverReadyV1,
        LspExplicitVirtualDocumentsV1,
    );
    assert!(shared.abort.load(Ordering::Acquire));
    {
        let mut usage = shared.usage.lock().unwrap();
        usage.group_hard_kills_accepted = 1;
    }
    shared.terminal.store(TERMINAL_CLOSED, Ordering::Release);
    assert_eq!(
        owner.drive_once(),
        LocalSupervisedTransportCleanupStateV1::ForcedConfirmed
    );
    let usage = owner.usage();
    assert_eq!(
        owner.drive_once(),
        LocalSupervisedTransportCleanupStateV1::ForcedConfirmed
    );
    assert_eq!(owner.usage(), usage);
    let receipt = owner.try_into_terminal_receipt().unwrap();
    assert_eq!(receipt.cleanup_usage().abort_requests, 1);
    assert_eq!(receipt.cleanup_usage().forced_confirmed, 1);
    assert!(format!("{receipt:?}").contains("collaborators: \"<owned>\""));
}

#[test]
fn supervised_fresh_transport_claim_observes_bytes_still_buffered_before_event_forwarding() {
    let shared = Arc::new(SharedState::default());
    shared.live.store(true, Ordering::Release);
    let (pump_tx, mut pump_rx) = async_mpsc::channel(1);
    let mut writer = ChannelWriter::new(pump_tx, true, Arc::clone(&shared), 1024);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { writer.write_all(b"pre-construction").await.unwrap() });

    let failure = raw_transport_for_freshness(shared)
        .try_claim_fresh()
        .unwrap_err();
    assert_eq!(
        failure.reason(),
        LocalLspFreshTransportRefusalV1::BufferedOutput
    );
    assert!(
        matches!(pump_rx.try_recv(), Ok(PumpEvent::Stdout(bytes)) if bytes == b"pre-construction")
    );
    let transport = failure.into_transport();
    assert_eq!(transport.usage().fresh_transport_refusals, 1);
}

#[test]
fn supervised_output_after_wrapper_before_initialize_is_refused() {
    let shared = Arc::new(SharedState::default());
    shared.live.store(true, Ordering::Release);
    let (pump_tx, mut pump_rx) = async_mpsc::channel(1);
    let mut writer = ChannelWriter::new(pump_tx, true, Arc::clone(&shared), 1024);
    let mut fresh = raw_transport_for_freshness(Arc::clone(&shared))
        .try_claim_fresh()
        .unwrap();
    let witness = fresh.fresh_start_witness();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { writer.write_all(b"between-claim-and-init").await.unwrap() });
    assert_eq!(fresh.try_send(b"initialize"), LspSendOutcomeV1::Refused);
    assert_eq!(
        witness.refusal(),
        Some(LocalLspFreshTransportRefusalV1::BufferedOutput)
    );
    assert!(matches!(
        pump_rx.try_recv(),
        Ok(PumpEvent::Stdout(bytes)) if bytes == b"between-claim-and-init"
    ));
    let usage = fresh.inner.usage();
    assert_eq!(usage.fresh_transport_refusals, 1);
    assert_eq!(usage.frame_refusals, 1);
}

#[test]
fn supervised_initialize_admission_linearizes_against_stdout_acceptance() {
    for _ in 0..128 {
        let shared = Arc::new(SharedState::default());
        shared.live.store(true, Ordering::Release);
        let (controls, control_rx) = mpsc::sync_channel(1);
        let (_events_tx, events) = mpsc::sync_channel(1);
        let mut fresh = LocalLspTransportV1 {
            controls,
            events,
            prefetched: None,
            shared: Arc::clone(&shared),
            max_frame_bytes: 1024,
            max_queued_bytes: 1024,
        }
        .try_claim_fresh()
        .unwrap();
        let witness = fresh.fresh_start_witness();
        let (pump_tx, mut pump_rx) = async_mpsc::channel(1);
        let mut writer = ChannelWriter::new(pump_tx, true, Arc::clone(&shared), 1024);
        let barrier = Arc::new(Barrier::new(2));
        let producer_barrier = Arc::clone(&barrier);
        let producer = thread::spawn(move || {
            producer_barrier.wait();
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async { writer.write_all(b"provider-output").await.unwrap() });
        });
        barrier.wait();
        let initialize = fresh.try_send(b"initialize");
        producer.join().unwrap();
        let output = pump_rx.try_recv();
        match (shared.stdout_freshness.load(Ordering::Acquire), initialize) {
            (STDOUT_TAINTED, LspSendOutcomeV1::Refused) => {
                assert!(control_rx.try_recv().is_err());
                assert_eq!(
                    witness.refusal(),
                    Some(LocalLspFreshTransportRefusalV1::BufferedOutput)
                );
            }
            (STDOUT_CLAIMED, LspSendOutcomeV1::Accepted) => {
                assert!(matches!(
                    control_rx.try_recv(),
                    Ok(Control::Write(bytes)) if bytes == b"initialize"
                ));
                assert_eq!(witness.refusal(), None);
            }
            _ => panic!("initialize/stdout admission gate admitted inconsistent winners"),
        }
        assert!(matches!(output, Ok(PumpEvent::Stdout(bytes)) if bytes == b"provider-output"));
    }
}

#[test]
fn supervised_late_stdout_cannot_reclassify_an_unrelated_initialize_refusal() {
    let shared = Arc::new(SharedState::default());
    shared.live.store(true, Ordering::Release);
    let (controls, _control_rx) = mpsc::sync_channel(1);
    controls.try_send(Control::GracefulClose).unwrap();
    let (_events_tx, events) = mpsc::sync_channel(1);
    let mut fresh = LocalLspTransportV1 {
        controls,
        events,
        prefetched: None,
        shared: Arc::clone(&shared),
        max_frame_bytes: 1024,
        max_queued_bytes: 1024,
    }
    .try_claim_fresh()
    .unwrap();
    let witness = fresh.fresh_start_witness();
    assert_eq!(fresh.try_send(b"initialize"), LspSendOutcomeV1::Refused);
    assert_eq!(witness.refusal(), None);

    let (pump_tx, mut pump_rx) = async_mpsc::channel(1);
    let mut writer = ChannelWriter::new(pump_tx, true, Arc::clone(&shared), 1024);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { writer.write_all(b"later-output").await.unwrap() });
    assert!(matches!(
        pump_rx.try_recv(),
        Ok(PumpEvent::Stdout(bytes)) if bytes == b"later-output"
    ));
    assert_eq!(witness.refusal(), None);
    let usage = fresh.inner.usage();
    assert_eq!(usage.fresh_transport_refusals, 0);
    assert_eq!(usage.frame_refusals, 1);
}

fn provenance() -> SemanticProviderProvenanceV1 {
    SemanticProviderProvenanceV1::try_new(
        SemanticProviderRefV1::new([9; 16]),
        "qualification".into(),
        "1.0.0".into(),
    )
    .unwrap()
}

struct SupervisedGenerations(u64);

impl SemanticGenerationIssuerV1 for SupervisedGenerations {
    fn issue(&mut self) -> Option<SemanticIndexGenerationV1> {
        self.0 = self.0.checked_add(1)?;
        SemanticIndexGenerationV1::new(self.0)
    }
}

#[derive(Clone, Copy)]
struct SupervisedClock;

impl LocalOrderedMonotonicClockV1 for SupervisedClock {
    fn sample(&mut self) -> Result<LocalOrderedInstantV1, LocalOrderedClockFailureV1> {
        Ok(LocalOrderedInstantV1::from_composition_tick(1))
    }
}

fn supervised_limits() -> (SemanticSessionLimitsV1, SemanticServiceLimitsV1) {
    let session = SemanticSessionLimitsV1::reference();
    let service = SemanticServiceLimitsV1 {
        max_clients: 1,
        max_provider_polls_per_pump: 1,
        ..SemanticServiceLimitsV1::reference()
    };
    (session, service)
}

fn supervised_client() -> SemanticClientRefV1 {
    SemanticClientRefV1::new([31; 16])
}

fn supervised_incarnation() -> SemanticSessionIncarnationV1 {
    SemanticSessionIncarnationV1::new([41; 16]).unwrap()
}

fn supervised_profile(mode: &str, scratch: &Scratch) -> SemanticProviderProfileV1 {
    let mut profile = profile(mode, scratch);
    profile.limits.max_frame_bytes = MAX_STDIN_FRAME_BYTES_V1;
    profile
}

fn deadlines() -> SemanticProcessDeadlinesV1 {
    SemanticProcessDeadlinesV1 {
        launch: Duration::from_secs(5),
        graceful_shutdown: Duration::from_millis(75),
        kill_and_reap: Duration::from_secs(3),
    }
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let requested = env::temp_dir().join(format!(
            "vellum-semantic-local-{}-{label}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir(&requested);
        std::fs::create_dir(&requested).unwrap();
        std::fs::set_permissions(&requested, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(std::fs::canonicalize(requested).unwrap())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir(&self.0);
    }
}

fn profile(mode: &str, scratch: &Scratch) -> SemanticProviderProfileV1 {
    let executable = std::fs::canonicalize(env::current_exe().unwrap()).unwrap();
    let workspace = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap();
    SemanticProviderProfileV1::try_new(
        provenance(),
        executable,
        vec![
            "--exact".into(),
            "tests::fake_lsp_child".into(),
            "--nocapture".into(),
        ],
        BTreeMap::from([
            ("VELLUM_FAKE_MODE".into(), mode.into()),
            (
                "VELLUM_EXPECTED_CWD".into(),
                scratch.0.to_string_lossy().into_owned(),
            ),
        ]),
        scratch.0.clone(),
        workspace,
        deadlines(),
        SemanticProcessLimitsV1 {
            command_queue: 8,
            stdout_queue: 16,
            stderr_queue: 4,
            max_chunk_bytes: 4096,
            max_frame_bytes: 4096,
            max_stderr_bytes: 1024,
        },
    )
    .unwrap()
}

#[test]
fn supervised_preflight_retains_qualification_when_transport_frame_is_too_narrow() {
    let scratch = Scratch::new("supervised-frame-preflight");
    let qualified = qualify_provider_launch_v1(profile("lsp", &scratch)).unwrap();
    let (session, service) = supervised_limits();
    let failure = launch_supervised_ordered_lsp_without_deadlines_v1(
        qualified,
        SupervisedGenerations(0),
        LspNeverReadyV1,
        LspExplicitVirtualDocumentsV1,
        LspLimitsV1::reference(),
        session.semantic,
        supervised_client(),
        supervised_incarnation(),
        session,
        service,
    )
    .unwrap_err();
    assert_eq!(
        failure.reason(),
        LocalSupervisedOrderedLspBuildRefusalV1::InsufficientTransportFrame
    );
    assert!(matches!(
        failure.into_residue(),
        LocalSupervisedOrderedLspBuildResidueV1::PreLaunch { .. }
    ));
}

#[test]
fn rust_analyzer_standalone_selector_binds_src_lib_before_process_effects() {
    let scratch = Scratch::new("rust-analyzer-standalone-preflight");
    let requested = env::temp_dir().join(format!(
        "vellum-semantic-local-{}-long-workspace",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&requested);
    std::fs::create_dir(&requested).unwrap();
    let mut workspace = std::fs::canonicalize(&requested).unwrap();
    while workspace.as_os_str().as_bytes().len().saturating_add(102) < 505 {
        workspace = workspace.join("x".repeat(100));
        std::fs::create_dir(&workspace).unwrap();
    }
    let remaining = 505_usize.saturating_sub(workspace.as_os_str().as_bytes().len());
    if remaining >= 2 {
        workspace = workspace.join("x".repeat(remaining - 1));
        std::fs::create_dir(&workspace).unwrap();
    }
    workspace = std::fs::canonicalize(workspace).unwrap();
    let root_uri_bytes = 7 + workspace.as_os_str().as_bytes().len();
    assert!(root_uri_bytes <= LspLimitsV1::reference().max_uri_bytes);
    assert!(root_uri_bytes + "/src/lib.rs".len() > LspLimitsV1::reference().max_uri_bytes);

    let profile = SemanticProviderProfileV1::try_new(
        provenance(),
        std::fs::canonicalize(env::current_exe().unwrap()).unwrap(),
        Vec::new(),
        BTreeMap::new(),
        scratch.0.clone(),
        workspace,
        deadlines(),
        SemanticProcessLimitsV1 {
            max_frame_bytes: MAX_STDIN_FRAME_BYTES_V1,
            ..SemanticProcessLimitsV1::reference()
        },
    )
    .unwrap();
    let qualified = qualify_provider_launch_v1(profile).unwrap();
    let profile_digest = qualified.profile_identity().digest();
    let protocol = LspProviderProtocolProfileV1::try_new(
        profile_digest,
        "rust-analyzer".to_owned(),
        None,
        "experimental/serverStatus".to_owned(),
    )
    .unwrap();
    let (session, service) = supervised_limits();
    let failure = launch_supervised_ordered_lsp_with_deadlines_rust_analyzer_standalone_v1(
        qualified,
        SupervisedGenerations(0),
        LspNeverReadyV1,
        LspExplicitVirtualDocumentsV1,
        LspLimitsV1::reference(),
        session.semantic,
        supervised_client(),
        supervised_incarnation(),
        session,
        service,
        SupervisedClock,
        LocalOrderedDeadlinePolicyV1 {
            relative_budget_ticks: std::num::NonZeroU64::new(10).unwrap(),
            max_deadlines: service.max_pending,
        },
        &protocol,
    )
    .unwrap_err();
    assert_eq!(
        failure.reason(),
        LocalSupervisedOrderedTimedLspBuildRefusalV1::Base(
            LocalSupervisedOrderedLspBuildRefusalV1::InvalidInitializationProfile
        )
    );
    let LocalSupervisedOrderedTimedLspBuildResidueV1::Base { failure, .. } = failure.into_residue()
    else {
        panic!("standalone initialization refusal crossed the process-effect boundary")
    };
    let LocalSupervisedOrderedLspBuildResidueV1::PreLaunch { qualified, .. } =
        failure.into_residue()
    else {
        panic!("standalone initialization refusal crossed the process-effect boundary")
    };
    assert_eq!(qualified.profile_identity().digest(), profile_digest);
    drop(qualified);
    std::fs::remove_dir_all(requested).unwrap();
}

#[test]
fn supervised_launch_failure_retains_nonprocess_collaborators() {
    let scratch = Scratch::new("supervised-launch-refusal");
    let qualified = qualify_provider_launch_v1(supervised_profile("lsp", &scratch)).unwrap();
    std::fs::remove_dir(&scratch.0).unwrap();
    let (session, service) = supervised_limits();
    let failure = launch_supervised_ordered_lsp_without_deadlines_v1(
        qualified,
        SupervisedGenerations(0),
        LspNeverReadyV1,
        LspExplicitVirtualDocumentsV1,
        LspLimitsV1::reference(),
        session.semantic,
        supervised_client(),
        supervised_incarnation(),
        session,
        service,
    )
    .unwrap_err();
    assert_eq!(
        failure.reason(),
        LocalSupervisedOrderedLspBuildRefusalV1::Launch(
            SemanticProcessLaunchRefusalV1::WorkingDirectoryChanged
        )
    );
    let LocalSupervisedOrderedLspBuildResidueV1::LaunchAttempted {
        failure,
        generations: _,
        readiness: _,
        virtual_documents: _,
    } = failure.into_residue()
    else {
        panic!("launch refusal must retain exact owned launch failure")
    };
    assert_eq!(
        failure.reason(),
        SemanticProcessLaunchRefusalV1::WorkingDirectoryChanged
    );
}

#[test]
fn supervised_transport_taint_and_fresh_start_failure_retain_cleanup() {
    let tainted_scratch = Scratch::new("supervised-tainted");
    let mut tainted = supervised_profile("echo", &tainted_scratch);
    tainted.environment.insert(
        "VELLUM_TEST_DELAY_BEFORE_FRESH_CLAIM_MILLIS".into(),
        "100".into(),
    );
    let (session, service) = supervised_limits();
    let failure = launch_supervised_ordered_lsp_without_deadlines_v1(
        qualify_provider_launch_v1(tainted).unwrap(),
        SupervisedGenerations(0),
        LspNeverReadyV1,
        LspExplicitVirtualDocumentsV1,
        LspLimitsV1::reference(),
        session.semantic,
        supervised_client(),
        supervised_incarnation(),
        session,
        service,
    )
    .unwrap_err();
    assert_eq!(
        failure.reason(),
        LocalSupervisedOrderedLspBuildRefusalV1::TransportFreshness(
            LocalLspFreshTransportRefusalV1::BufferedOutput
        )
    );
    let LocalSupervisedOrderedLspCleanupV1::Transport(mut cleanup) = failure.into_cleanup() else {
        panic!("tainted transport must enter transport-only cleanup")
    };
    let mut transport_receipt = None;
    for _ in 0..200 {
        let _ = cleanup.drive_once();
        match cleanup.try_into_terminal_receipt() {
            Ok(receipt) => {
                transport_receipt = Some(receipt);
                break;
            }
            Err(owner) => cleanup = *owner,
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let transport_receipt = transport_receipt.expect("transport cleanup must terminate");
    assert_eq!(
        transport_receipt.state(),
        LocalSupervisedTransportCleanupStateV1::ForcedConfirmed
    );
    assert_eq!(
        transport_receipt.transport_terminal(),
        vellum_adapter_semantic_lsp::LspTransportTerminalV1::ForcedClosed
    );
    assert_eq!(transport_receipt.cleanup_usage().abort_requests, 1);
    assert_eq!(transport_receipt.cleanup_usage().forced_confirmed, 1);
    assert!(format!("{transport_receipt:?}").contains("collaborators: \"<owned>\""));

    let refused_scratch = Scratch::new("supervised-fresh-refusal");
    let mut refused = supervised_profile("lsp", &refused_scratch);
    refused
        .environment
        .insert("VELLUM_TEST_TAINT_AFTER_FRESH_CLAIM".into(), "1".into());
    let failure = launch_supervised_ordered_lsp_without_deadlines_v1(
        qualify_provider_launch_v1(refused).unwrap(),
        SupervisedGenerations(0),
        LspNeverReadyV1,
        LspExplicitVirtualDocumentsV1,
        LspLimitsV1::reference(),
        session.semantic,
        supervised_client(),
        supervised_incarnation(),
        session,
        service,
    )
    .unwrap_err();
    assert_eq!(
        failure.reason(),
        LocalSupervisedOrderedLspBuildRefusalV1::TransportFreshness(
            LocalLspFreshTransportRefusalV1::BufferedOutput
        )
    );
    let LocalSupervisedOrderedLspCleanupV1::Provider(cleanup) = failure.into_cleanup() else {
        panic!("fresh-start refusal must enter forced-only provider cleanup")
    };
    let mut cleanup = *cleanup;
    for _ in 0..100 {
        let _ = cleanup.drive_once();
        match cleanup.try_into_terminal_receipt() {
            Ok(_) => return,
            Err(owner) => cleanup = *owner,
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("fresh-start cleanup did not reach terminal evidence")
}

#[test]
fn supervised_disabled_and_timed_constructors_close_policy_before_exposure() {
    let (session, service) = supervised_limits();
    let disabled_scratch = Scratch::new("supervised-disabled-success");
    let prepared = launch_supervised_ordered_lsp_without_deadlines_v1(
        qualify_provider_launch_v1(supervised_profile("lsp", &disabled_scratch)).unwrap(),
        SupervisedGenerations(0),
        LspNeverReadyV1,
        LspExplicitVirtualDocumentsV1,
        LspLimitsV1::reference(),
        session.semantic,
        supervised_client(),
        supervised_incarnation(),
        session,
        service,
    )
    .unwrap();
    let mut cleanup = prepared.into_cleanup();
    let _ = cleanup.retire_client();
    drop(
        cleanup
            .try_into_retiring_provider()
            .unwrap()
            .into_provider_for_retirement(),
    );

    let invalid_scratch = Scratch::new("supervised-timed-preflight");
    let failure = launch_supervised_ordered_lsp_with_deadlines_v1(
        qualify_provider_launch_v1(supervised_profile("lsp", &invalid_scratch)).unwrap(),
        SupervisedGenerations(0),
        LspNeverReadyV1,
        LspExplicitVirtualDocumentsV1,
        LspLimitsV1::reference(),
        session.semantic,
        supervised_client(),
        supervised_incarnation(),
        session,
        service,
        SupervisedClock,
        LocalOrderedDeadlinePolicyV1 {
            relative_budget_ticks: std::num::NonZeroU64::new(1).unwrap(),
            max_deadlines: 0,
        },
    )
    .unwrap_err();
    assert_eq!(
        failure.reason(),
        LocalSupervisedOrderedTimedLspBuildRefusalV1::Deadlines(
            LocalOrderedDeadlineBuildRefusalV1::InvalidPolicy
        )
    );
    assert!(matches!(
        failure.into_residue(),
        LocalSupervisedOrderedTimedLspBuildResidueV1::PreLaunch { .. }
    ));

    let timed_scratch = Scratch::new("supervised-timed-success");
    let timed = launch_supervised_ordered_lsp_with_deadlines_v1(
        qualify_provider_launch_v1(supervised_profile("lsp", &timed_scratch)).unwrap(),
        SupervisedGenerations(0),
        LspNeverReadyV1,
        LspExplicitVirtualDocumentsV1,
        LspLimitsV1::reference(),
        session.semantic,
        supervised_client(),
        supervised_incarnation(),
        session,
        service,
        SupervisedClock,
        LocalOrderedDeadlinePolicyV1 {
            relative_budget_ticks: std::num::NonZeroU64::new(10).unwrap(),
            max_deadlines: service.max_pending,
        },
    )
    .unwrap();
    let cleanup = timed.into_cleanup();
    let (mut cleanup, _) = cleanup.retire_deadlines().unwrap();
    let _ = cleanup.retire_client();
    drop(
        cleanup
            .try_into_retiring_provider()
            .unwrap()
            .into_provider_for_retirement(),
    );
}

fn await_contains(transport: &mut LocalLspTransportV1, needle: &[u8]) {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut observed = Vec::new();
    loop {
        match transport.poll_receive() {
            LspReceiveOutcomeV1::Chunk(bytes) => observed.extend_from_slice(&bytes),
            LspReceiveOutcomeV1::Pending => thread::sleep(Duration::from_millis(5)),
            other => panic!("stream ended before marker: {other:?}"),
        }
        assert!(Instant::now() < deadline, "semantic child marker deadline");
        if observed
            .windows(needle.len())
            .any(|window| window == needle)
        {
            return;
        }
    }
}

fn await_closed(transport: &mut LocalLspTransportV1) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match transport.poll_receive() {
            LspReceiveOutcomeV1::Closed => return,
            LspReceiveOutcomeV1::Chunk(_) | LspReceiveOutcomeV1::Pending => {
                thread::sleep(Duration::from_millis(5));
            }
            other => panic!(
                "semantic child failed while closing: {other:?}; {:?}",
                transport.usage()
            ),
        }
        assert!(
            Instant::now() < deadline,
            "semantic child close deadline: {:?}",
            (
                transport.usage(),
                transport.shared.graceful_close.load(Ordering::Acquire),
                transport.shared.abort.load(Ordering::Acquire)
            )
        );
    }
}

fn await_failed(transport: &mut LocalLspTransportV1) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match transport.poll_receive() {
            LspReceiveOutcomeV1::Failed => return,
            LspReceiveOutcomeV1::Chunk(_) | LspReceiveOutcomeV1::Pending => {
                thread::sleep(Duration::from_millis(5));
            }
            LspReceiveOutcomeV1::Closed => panic!("semantic failure was reported as closed"),
        }
        assert!(
            Instant::now() < deadline,
            "semantic child failure deadline: {:?}",
            transport.usage()
        );
    }
}

#[test]
fn profile_rejects_ambient_or_aliased_authority() {
    let scratch = Scratch::new("profile");
    let workspace = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap();
    let valid = profile("echo", &scratch);
    assert!(!format!("{valid:?}").contains(scratch.0.to_str().unwrap()));
    assert_eq!(
        SemanticProviderProfileV1::try_new(
            provenance(),
            "relative".into(),
            vec![],
            BTreeMap::new(),
            scratch.0.clone(),
            workspace.clone(),
            deadlines(),
            SemanticProcessLimitsV1::reference(),
        )
        .unwrap_err(),
        SemanticProfileRefusalV1::InvalidExecutable
    );
    assert_eq!(
        SemanticProviderProfileV1::try_new(
            provenance(),
            std::fs::canonicalize(env::current_exe().unwrap()).unwrap(),
            vec![],
            BTreeMap::new(),
            workspace.join("target"),
            workspace.clone(),
            deadlines(),
            SemanticProcessLimitsV1::reference(),
        )
        .unwrap_err(),
        SemanticProfileRefusalV1::WorkspaceWorkingDirectory
    );
    let invalid = BTreeMap::from([("BAD=KEY".into(), "value".into())]);
    assert_eq!(
        SemanticProviderProfileV1::try_new(
            provenance(),
            std::fs::canonicalize(env::current_exe().unwrap()).unwrap(),
            vec![],
            invalid,
            scratch.0.clone(),
            workspace,
            deadlines(),
            SemanticProcessLimitsV1::reference(),
        )
        .unwrap_err(),
        SemanticProfileRefusalV1::InvalidEnvironment
    );

    let loader_injection = BTreeMap::from([("DYLD_INSERT_LIBRARIES".into(), "x".into())]);
    assert_eq!(
        SemanticProviderProfileV1::try_new(
            provenance(),
            std::fs::canonicalize(env::current_exe().unwrap()).unwrap(),
            vec![],
            loader_injection,
            scratch.0.clone(),
            std::fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap(),
            deadlines(),
            SemanticProcessLimitsV1::reference(),
        )
        .unwrap_err(),
        SemanticProfileRefusalV1::InvalidEnvironment
    );
}

#[test]
fn qualified_launch_is_affine_redacted_and_revalidates_private_cwd() {
    let scratch = Scratch::new("qualified");
    let qualified = qualify_provider_launch_v1(profile("echo", &scratch)).unwrap();
    let artifact_digest = qualified.artifact_identity().digest();
    let profile_digest = qualified.profile_identity().digest();
    assert_ne!(artifact_digest, [0; 32]);
    assert_ne!(profile_digest, [0; 32]);
    assert_ne!(artifact_digest, profile_digest);
    let debug = format!("{qualified:?}");
    assert!(!debug.contains(scratch.0.to_str().unwrap()));
    assert!(debug.contains("<redacted>"));
    assert_eq!(
        qualified.assurance().filesystem_confinement,
        SemanticProviderSupportV1::Unsupported
    );
    assert_eq!(
        qualified.assurance().core_dumps_disabled,
        SemanticProviderSupportV1::Enforced
    );
    assert_eq!(
        qualified.assurance().whole_tree_memory_limit,
        SemanticProviderSupportV1::Unsupported
    );
    assert_eq!(
        qualified.assurance().whole_tree_process_limit,
        SemanticProviderSupportV1::Unsupported
    );
    assert_eq!(
        qualified.assurance().whole_tree_cpu_limit,
        SemanticProviderSupportV1::Unsupported
    );
    std::fs::set_permissions(&scratch.0, std::fs::Permissions::from_mode(0o755)).unwrap();
    let refusal = match launch_qualified_local_lsp_v1(qualified) {
        Ok(_) => panic!("changed private cwd unexpectedly launched"),
        Err(refusal) => refusal,
    };
    assert_eq!(
        refusal,
        SemanticProcessLaunchRefusalV1::WorkingDirectoryChanged
    );
    std::fs::set_permissions(&scratch.0, std::fs::Permissions::from_mode(0o700)).unwrap();
}

fn strict_resource_policy() -> SemanticProviderResourcePolicyV1 {
    SemanticProviderResourcePolicyV1::RequiredWholeTree {
        max_memory_bytes: NonZeroU64::new(512 * 1024 * 1024).unwrap(),
        max_processes: NonZeroU32::new(8).unwrap(),
        cpu_millicores: NonZeroU32::new(750).unwrap(),
    }
}

#[test]
fn resource_policy_is_profile_bound_and_refuses_unsupported_hosts_before_spawn() {
    let scratch = Scratch::new("resource-policy-preflight");
    let unbounded_profile = profile("echo", &scratch);
    let unbounded = qualify_provider_launch_for_mechanism_v1(
        unbounded_profile.clone(),
        Mechanism::ProcessGroup,
    )
    .unwrap();

    let mut strict_profile = unbounded_profile;
    strict_profile.resource_policy = strict_resource_policy();
    let refused =
        qualify_provider_launch_for_mechanism_v1(strict_profile.clone(), Mechanism::ProcessGroup)
            .unwrap_err();
    assert_eq!(
        refused,
        SemanticProcessLaunchRefusalV1::ResourcePolicyUnsupported
    );

    let strict =
        qualify_provider_launch_for_mechanism_v1(strict_profile, Mechanism::JobObject).unwrap();
    assert_ne!(
        unbounded.profile_identity().digest(),
        strict.profile_identity().digest()
    );
    assert_eq!(
        strict.assurance().whole_tree_memory_limit,
        SemanticProviderSupportV1::Enforced
    );
    assert_eq!(
        strict.assurance().whole_tree_process_limit,
        SemanticProviderSupportV1::Enforced
    );
    assert_eq!(
        strict.assurance().whole_tree_cpu_limit,
        SemanticProviderSupportV1::Enforced
    );
    assert_eq!(
        strict.assurance().filesystem_confinement,
        SemanticProviderSupportV1::Unsupported
    );
    assert_eq!(
        strict.assurance().network_confinement,
        SemanticProviderSupportV1::Unsupported
    );
}

#[test]
fn resource_policy_projects_exact_processkit_options_and_closed_host_support() {
    assert!(mechanism_supports_resource_policy(
        SemanticProviderResourcePolicyV1::Unbounded,
        Mechanism::ProcessGroup
    ));
    for mechanism in [Mechanism::JobObject, Mechanism::CgroupV2] {
        assert!(mechanism_supports_resource_policy(
            strict_resource_policy(),
            mechanism
        ));
    }
    for mechanism in [Mechanism::ProcessGroup, Mechanism::ProcessReaper] {
        assert!(!mechanism_supports_resource_policy(
            strict_resource_policy(),
            mechanism
        ));
    }

    let options = strict_resource_policy().process_group_options();
    assert_eq!(options.limits.max_memory, Some(512 * 1024 * 1024));
    assert_eq!(options.limits.max_processes, Some(8));
    assert_eq!(options.limits.cpu_quota, Some(0.75));
}

#[test]
fn resource_policy_constructor_rejects_values_outside_the_closed_ceiling() {
    let scratch = Scratch::new("resource-policy-bounds");
    let base = profile("echo", &scratch);
    let refusal = SemanticProviderProfileV1::try_new_with_resource_policy(
        base.provenance,
        base.executable,
        base.arguments,
        base.environment,
        base.private_cwd,
        base.workspace_root,
        base.deadlines,
        base.limits,
        SemanticProviderResourcePolicyV1::RequiredWholeTree {
            max_memory_bytes: NonZeroU64::new(MAX_PROVIDER_MEMORY_BYTES_V1 + 1).unwrap(),
            max_processes: NonZeroU32::new(1).unwrap(),
            cpu_millicores: NonZeroU32::new(1).unwrap(),
        },
    )
    .unwrap_err();
    assert_eq!(refusal, SemanticProfileRefusalV1::InvalidResourcePolicy);
}

#[test]
fn production_promotion_is_closed_affine_and_preserves_typed_nonclaims() {
    let scratch = Scratch::new("production-promotion");
    let qualified = qualify_provider_launch_v1(profile("echo", &scratch)).unwrap();
    let artifact = qualified.artifact_identity();
    let profile = qualified.profile_identity();
    let failure = qualify_provider_launch_for_production_v1(qualified).unwrap_err();

    assert_eq!(
        failure.support_for(SemanticProviderProductionRequirementV1::ExecutableIdentity),
        SemanticProviderSupportV1::ObservedOnly
    );
    assert_eq!(
        failure.support_for(SemanticProviderProductionRequirementV1::WorkingDirectoryIdentity),
        SemanticProviderSupportV1::ObservedOnly
    );
    assert_eq!(
        failure.support_for(SemanticProviderProductionRequirementV1::AmbientEnvironmentCleared),
        SemanticProviderSupportV1::Enforced
    );
    assert_eq!(
        failure.support_for(SemanticProviderProductionRequirementV1::CoreDumpsDisabled),
        SemanticProviderSupportV1::Enforced
    );
    for requirement in [
        SemanticProviderProductionRequirementV1::FilesystemConfinement,
        SemanticProviderProductionRequirementV1::NetworkConfinement,
        SemanticProviderProductionRequirementV1::SubprocessConfinement,
        SemanticProviderProductionRequirementV1::WholeTreeMemoryLimit,
        SemanticProviderProductionRequirementV1::WholeTreeProcessLimit,
        SemanticProviderProductionRequirementV1::WholeTreeCpuLimit,
    ] {
        assert_eq!(
            failure.support_for(requirement),
            SemanticProviderSupportV1::Unsupported
        );
    }
    assert_eq!(
        failure.support_for(SemanticProviderProductionRequirementV1::AbruptParentCleanup),
        SemanticProviderLaunchAssuranceV1::current().abrupt_parent_cleanup
    );

    let debug = format!("{failure:?}");
    assert!(debug.contains("filesystem_confinement: Unsupported"));
    assert!(!debug.contains(scratch.0.to_str().unwrap()));
    let recovered = failure.into_probe_qualified_launch();
    assert_eq!(recovered.artifact_identity(), artifact);
    assert_eq!(recovered.profile_identity(), profile);
}

#[test]
fn production_promotion_requires_the_meet_of_every_fixed_assurance_axis() {
    let scratch = Scratch::new("production-meet");
    let mut qualified = qualify_provider_launch_v1(profile("echo", &scratch)).unwrap();
    qualified.assurance = SemanticProviderLaunchAssuranceV1 {
        executable_identity: SemanticProviderSupportV1::Enforced,
        working_directory_identity: SemanticProviderSupportV1::Enforced,
        ambient_environment_cleared: SemanticProviderSupportV1::Enforced,
        core_dumps_disabled: SemanticProviderSupportV1::Enforced,
        filesystem_confinement: SemanticProviderSupportV1::Enforced,
        network_confinement: SemanticProviderSupportV1::Enforced,
        subprocess_confinement: SemanticProviderSupportV1::Enforced,
        abrupt_parent_cleanup: SemanticProviderSupportV1::Enforced,
        whole_tree_memory_limit: SemanticProviderSupportV1::Enforced,
        whole_tree_process_limit: SemanticProviderSupportV1::Enforced,
        whole_tree_cpu_limit: SemanticProviderSupportV1::Enforced,
    };
    let artifact = qualified.artifact_identity();
    let profile = qualified.profile_identity();

    let production = qualify_provider_launch_for_production_v1(qualified).unwrap();
    assert_eq!(production.artifact_identity(), artifact);
    assert_eq!(production.profile_identity(), profile);
    assert!(
        SemanticProviderProductionRequirementV1::ALL
            .into_iter()
            .all(
                |requirement| production.assurance().support_for(requirement)
                    == SemanticProviderSupportV1::Enforced
            )
    );
}

#[test]
fn launch_timeout_fence_is_qualified_in_an_isolated_process() {
    let status = std::process::Command::new(env::current_exe().unwrap())
        .args([
            "--exact",
            "tests::launch_timeout_fence_child",
            "--ignored",
            "--nocapture",
        ])
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
#[ignore = "isolated child for the launch-timeout fence qualification"]
fn launch_timeout_fence_child() {
    let scratch = Scratch::new("launch-timeout-fence");
    let mut configured = profile("hang", &scratch);
    configured.deadlines.launch = Duration::from_millis(20);
    configured.deadlines.kill_and_reap = Duration::from_secs(3);
    configured.environment.insert(
        "VELLUM_TEST_DELAY_BEFORE_START_MILLIS".into(),
        "3000".into(),
    );
    let qualified = qualify_provider_launch_v1(configured).unwrap();
    let started = Instant::now();
    let refusal = match launch_qualified_local_lsp_v1(qualified) {
        Ok(_) => panic!("delayed launch unexpectedly escaped"),
        Err(refusal) => refusal,
    };
    assert_eq!(refusal, SemanticProcessLaunchRefusalV1::LaunchTimedOut);
    assert!(started.elapsed() < Duration::from_millis(2500));
    let fenced = match launch_local_lsp_v1(profile("echo", &scratch)) {
        Ok(_) => panic!("fenced generation unexpectedly launched"),
        Err(refusal) => refusal,
    };
    assert_eq!(fenced, SemanticProcessLaunchRefusalV1::LaunchFenced);
    reset_launch_fence_after_terminal_for_test();
}

#[test]
fn owned_unresolved_launch_receipts_are_qualified_in_isolated_processes() {
    for child in [
        "tests::owned_launch_timeout_resolution_child",
        "tests::owned_launch_restart_required_child",
    ] {
        let status = std::process::Command::new(env::current_exe().unwrap())
            .args(["--exact", child, "--ignored", "--nocapture"])
            .status()
            .unwrap();
        assert!(
            status.success(),
            "isolated owned-launch child failed: {child}"
        );
    }
}

fn resolve_owned_launch(
    mut owner: SemanticUnresolvedLaunchOwnerV1,
) -> SemanticUnresolvedLaunchReceiptV1 {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match owner.try_resolve() {
            Ok(receipt) => return receipt,
            Err(returned) => owner = returned,
        }
        assert!(
            Instant::now() < deadline,
            "unresolved launch did not finish"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[ignore = "isolated child for owned timeout resolution"]
fn owned_launch_timeout_resolution_child() {
    let scratch = Scratch::new("owned-launch-timeout");
    let mut configured = supervised_profile("hang", &scratch);
    configured.deadlines.launch = Duration::from_millis(20);
    configured
        .environment
        .insert("VELLUM_TEST_DELAY_BEFORE_START_MILLIS".into(), "150".into());
    let qualified = qualify_provider_launch_v1(configured).unwrap();
    let (session, service) = supervised_limits();
    let failure = launch_supervised_ordered_lsp_without_deadlines_v1(
        qualified,
        SupervisedGenerations(0),
        LspNeverReadyV1,
        LspExplicitVirtualDocumentsV1,
        LspLimitsV1::reference(),
        session.semantic,
        supervised_client(),
        supervised_incarnation(),
        session,
        service,
    )
    .unwrap_err();
    assert_eq!(
        failure.reason(),
        LocalSupervisedOrderedLspBuildRefusalV1::Launch(
            SemanticProcessLaunchRefusalV1::LaunchTimedOut
        )
    );
    let LocalSupervisedOrderedLspBuildResidueV1::LaunchAttempted { failure, .. } =
        failure.into_residue()
    else {
        panic!("delayed launch must retain owned launch failure")
    };
    let SemanticProcessOwnedLaunchFailureV1::Unresolved(owner) = failure else {
        panic!("launch timeout must retain unresolved ownership")
    };
    assert_eq!(
        owner.initial_refusal(),
        SemanticProcessLaunchRefusalV1::LaunchTimedOut
    );
    drop(owner);
    let owner = recover_unresolved_local_lsp_launch_v1()
        .expect("dropped unresolved owner must be globally recoverable");
    let receipt = resolve_owned_launch(owner);
    assert_eq!(
        receipt.disposition(),
        SemanticUnresolvedLaunchDispositionV1::NoApplicationConfirmed
    );
    assert_eq!(receipt.fence(), SemanticLaunchFenceDispositionV1::Cleared);
    assert!(!SEMANTIC_LAUNCH_FENCED.load(Ordering::Acquire));
    assert!(recover_unresolved_local_lsp_launch_v1().is_none());
    assert!(!format!("{receipt:?}").contains(scratch.0.to_str().unwrap()));
}

#[test]
#[ignore = "isolated child for restart-required launch evidence"]
fn owned_launch_restart_required_child() {
    let scratch = Scratch::new("owned-launch-restart");
    let mut configured = profile("echo", &scratch);
    configured
        .environment
        .insert("VELLUM_TEST_UNRESOLVED_RESTART_REQUIRED".into(), "1".into());
    let qualified = qualify_provider_launch_v1(configured).unwrap();
    let failure = match launch_qualified_local_lsp_owned_v1(qualified) {
        Ok(_) => panic!("uncertain launch unexpectedly escaped"),
        Err(failure) => failure,
    };
    let SemanticProcessOwnedLaunchFailureV1::Unresolved(owner) = failure else {
        panic!("cleanup uncertainty must retain unresolved ownership")
    };
    let receipt = resolve_owned_launch(owner);
    assert_eq!(
        receipt.initial_refusal(),
        SemanticProcessLaunchRefusalV1::CleanupUnconfirmed
    );
    assert_eq!(
        receipt.disposition(),
        SemanticUnresolvedLaunchDispositionV1::RestartRequired
    );
    assert_eq!(receipt.fence(), SemanticLaunchFenceDispositionV1::Retained);
    assert!(SEMANTIC_LAUNCH_FENCED.load(Ordering::Acquire));
    reset_launch_fence_after_terminal_for_test();
}

#[test]
fn post_start_launch_refusal_confirms_hard_kill_and_leader_reap() {
    let scratch = Scratch::new("launch-cleanup");
    let mut configured = profile("hang", &scratch);
    configured
        .environment
        .insert("VELLUM_TEST_FAIL_AFTER_START".into(), "1".into());
    let refusal = match launch_local_lsp_v1(configured) {
        Ok(_) => panic!("post-start launch seam unexpectedly succeeded"),
        Err(refusal) => refusal,
    };
    // `SpawnRefused` is emitted at this seam only when the bounded cleanup
    // confirms both the process-group hard kill and the supervised leader
    // reap. Measuring the whole launch call against the inner cleanup budget
    // is invalid because runtime setup, revalidation, and spawn precede it.
    assert_eq!(refusal, SemanticProcessLaunchRefusalV1::SpawnRefused);
}

#[test]
fn fixed_argv_cwd_environment_and_bytes_round_trip_then_reap() {
    let scratch = Scratch::new("echo");
    let mut transport = launch_local_lsp_v1(profile("echo", &scratch)).unwrap();
    await_contains(&mut transport, b"profile-ok\n");
    assert_eq!(transport.try_send(b"frame-one"), LspSendOutcomeV1::Accepted);
    await_contains(&mut transport, b"frame-one");
    transport.close();
    await_closed(&mut transport);
    let usage = transport.usage();
    assert_eq!(
        (
            usage.launches,
            usage.frames_admitted,
            usage.graceful_shutdowns,
            usage.leader_reaped
        ),
        (1, 1, 1, 1)
    );
    assert_eq!(usage.group_hard_kills_accepted, 0);
    assert!(usage.resource_samples >= 1);
    assert!(usage.active_process_units_high_water >= 1);
}

#[test]
fn stderr_saturation_never_blocks_stdout() {
    let scratch = Scratch::new("stderr");
    let mut transport = launch_local_lsp_v1(profile("stderr", &scratch)).unwrap();
    await_contains(&mut transport, b"stderr-drained\n");
    transport.close();
    await_closed(&mut transport);
    let usage = transport.usage();
    assert_eq!(usage.stderr_bytes, 1024);
    assert!(usage.stderr_dropped_bytes > 0);
}

#[test]
fn ignored_close_is_killed_and_reaped_while_crash_is_distinct() {
    let scratch = Scratch::new("hang");
    let mut transport = launch_local_lsp_v1(profile("hang", &scratch)).unwrap();
    await_contains(&mut transport, b"hanging\n");
    transport.close();
    await_closed(&mut transport);
    let usage = transport.usage();
    assert_eq!(
        (
            usage.group_hard_kills_accepted,
            usage.leader_reaped,
            usage.crashes
        ),
        (1, 1, 0)
    );

    let crash_scratch = Scratch::new("crash");
    let mut crashed = launch_local_lsp_v1(profile("crash", &crash_scratch)).unwrap();
    await_failed(&mut crashed);
    let usage = crashed.usage();
    assert_eq!(
        (usage.crashes, usage.leader_reaped, usage.graceful_shutdowns),
        (1, 1, 0)
    );
}

#[test]
fn owner_loss_forces_group_kill_and_reap() {
    let scratch = Scratch::new("owner-loss");
    let mut transport = launch_local_lsp_v1(profile("hang", &scratch)).unwrap();
    await_contains(&mut transport, b"hanging\n");
    let shared = Arc::clone(&transport.shared);
    drop(transport);
    let deadline = Instant::now() + Duration::from_secs(5);
    while shared.live.load(Ordering::Acquire) {
        assert!(Instant::now() < deadline, "owner-loss reap deadline");
        thread::sleep(Duration::from_millis(5));
    }
    let usage = *shared.usage.lock().unwrap();
    assert_eq!(
        (
            usage.group_hard_kills_accepted,
            usage.leader_reaped,
            usage.owner_loss_abort_requests,
            usage.explicit_abort_requests,
        ),
        (1, 1, 1, 0)
    );
}

#[test]
fn nonreading_child_cannot_block_owner_loss_or_exceed_byte_credit() {
    let scratch = Scratch::new("nonreading");
    let mut configured = profile("nonreading", &scratch);
    configured.limits.max_frame_bytes = 1024 * 1024;
    let mut transport = launch_local_lsp_v1(configured).unwrap();
    await_contains(&mut transport, b"not-reading\n");
    let frame = vec![b'x'; 1024 * 1024];
    let mut admitted = 0;
    for _ in 0..32 {
        if transport.try_send(&frame) == LspSendOutcomeV1::Accepted {
            admitted += 1;
        }
    }
    assert!(admitted <= 8, "byte credit admitted {admitted} MiB frames");
    let shared = Arc::clone(&transport.shared);
    drop(transport);
    let deadline = Instant::now() + Duration::from_secs(5);
    while shared.live.load(Ordering::Acquire) {
        assert!(
            Instant::now() < deadline,
            "blocked-stdin owner-loss deadline"
        );
        thread::sleep(Duration::from_millis(5));
    }
    let usage = *shared.usage.lock().unwrap();
    assert_eq!(
        (usage.group_hard_kills_accepted, usage.leader_reaped),
        (1, 1)
    );
}

#[test]
fn full_stdout_queue_cannot_hide_terminal_failure() {
    let scratch = Scratch::new("stdout-full");
    let mut configured = profile("stdout-flood", &scratch);
    configured.limits.stdout_queue = 1;
    let mut transport = launch_local_lsp_v1(configured).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if !transport.shared.live.load(Ordering::Acquire) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "stdout saturation teardown deadline"
        );
        thread::sleep(Duration::from_millis(5));
    }
    loop {
        match transport.poll_receive() {
            LspReceiveOutcomeV1::Chunk(_) | LspReceiveOutcomeV1::Pending => {}
            LspReceiveOutcomeV1::Failed => break,
            LspReceiveOutcomeV1::Closed => panic!("stdout saturation was reported as closed"),
        }
        assert!(
            Instant::now() < deadline,
            "terminal failure delivery deadline"
        );
    }
    assert!(transport.usage().stdout_dropped_bytes > 0);
}

#[test]
fn natural_exit_drains_tail_and_nonzero_close_is_not_graceful() {
    let scratch = Scratch::new("tail");
    let mut transport = launch_local_lsp_v1(profile("tail", &scratch)).unwrap();
    await_contains(&mut transport, b"tail-marker\n");
    await_failed(&mut transport);
    assert_eq!(transport.usage().leader_reaped, 1);

    let bad_scratch = Scratch::new("bad-close");
    let mut bad = launch_local_lsp_v1(profile("bad-close", &bad_scratch)).unwrap();
    await_contains(&mut bad, b"bad-close-ready\n");
    bad.close();
    await_failed(&mut bad);
    let usage = bad.usage();
    assert_eq!((usage.graceful_shutdowns, usage.nonzero_exits), (0, 1));
}

#[test]
fn close_and_abort_are_monotone_admission_fences() {
    let close_scratch = Scratch::new("close-fence");
    let mut closing = launch_local_lsp_v1(profile("hang", &close_scratch)).unwrap();
    await_contains(&mut closing, b"hanging\n");
    closing.close();
    assert_eq!(
        closing.try_send(b"after-close"),
        LspSendOutcomeV1::Unavailable
    );
    await_closed(&mut closing);
    assert_eq!(closing.shared.queued_stdin_bytes.load(Ordering::Acquire), 0);

    let abort_scratch = Scratch::new("abort-fence");
    let mut aborting = launch_local_lsp_v1(profile("hang", &abort_scratch)).unwrap();
    await_contains(&mut aborting, b"hanging\n");
    aborting.abort();
    assert_eq!(
        aborting.try_send(b"after-abort"),
        LspSendOutcomeV1::Unavailable
    );
    await_closed(&mut aborting);
    assert_eq!(aborting.usage().explicit_abort_requests, 1);
    assert_eq!(aborting.usage().owner_loss_abort_requests, 0);
    assert_eq!(
        aborting.shared.queued_stdin_bytes.load(Ordering::Acquire),
        0
    );
}

#[test]
fn leader_exit_with_descendant_inherited_pipes_is_bounded_and_failed() {
    let scratch = Scratch::new("descendant-pipes");
    let mut transport = launch_local_lsp_v1(profile("descendant-exit", &scratch)).unwrap();
    await_contains(&mut transport, b"leader-exiting\n");
    await_failed(&mut transport);
    let usage = transport.usage();
    assert_eq!(
        (
            usage.crashes,
            usage.group_hard_kills_accepted,
            usage.leader_reaped
        ),
        (1, 1, 1)
    );
}

#[test]
fn resource_samples_preserve_support_and_high_water_semantics() {
    let mut usage = SemanticProcessUsageV1::default();
    record_resource_sample(
        &mut usage,
        Some(SemanticProcessResourceSampleV1 {
            active_process_units: 3,
            cpu_time_nanos: Some(11),
            peak_memory_bytes: Some(13),
            io_read_bytes: Some(17),
            io_write_bytes: Some(19),
            peak_process_units: Some(5),
        }),
    );
    record_resource_sample(
        &mut usage,
        Some(SemanticProcessResourceSampleV1 {
            active_process_units: 1,
            cpu_time_nanos: Some(7),
            peak_memory_bytes: None,
            io_read_bytes: None,
            io_write_bytes: None,
            peak_process_units: None,
        }),
    );
    record_resource_sample(&mut usage, None);

    assert_eq!(usage.resource_samples, 2);
    assert_eq!(usage.resource_sample_failures, 1);
    assert_eq!(usage.active_process_units_latest, 1);
    assert_eq!(usage.active_process_units_high_water, 3);
    assert_eq!(usage.cpu_time_observations, 2);
    assert_eq!(usage.cpu_time_nanos_high_water, 11);
    assert_eq!(usage.peak_memory_observations, 1);
    assert_eq!(usage.peak_memory_bytes_high_water, 13);
    assert_eq!(usage.io_read_observations, 1);
    assert_eq!(usage.io_read_bytes_high_water, 17);
    assert_eq!(usage.io_write_observations, 1);
    assert_eq!(usage.io_write_bytes_high_water, 19);
    assert_eq!(usage.peak_process_observations, 1);
    assert_eq!(usage.peak_process_units_high_water, 5);
    assert!(!usage.saturated);
}

#[test]
fn oversized_resource_observations_saturate_without_wrapping() {
    let mut usage = SemanticProcessUsageV1::default();
    record_resource_sample(
        &mut usage,
        Some(SemanticProcessResourceSampleV1 {
            active_process_units: 1,
            cpu_time_nanos: Some(u128::MAX),
            peak_memory_bytes: None,
            io_read_bytes: None,
            io_write_bytes: None,
            peak_process_units: None,
        }),
    );

    assert_eq!(usage.cpu_time_nanos_high_water, u64::MAX);
    assert!(usage.saturated);
}

#[test]
fn fake_lsp_child() {
    let Ok(mode) = env::var("VELLUM_FAKE_MODE") else {
        return;
    };
    assert_eq!(
        env::current_dir().unwrap(),
        PathBuf::from(env::var("VELLUM_EXPECTED_CWD").unwrap())
    );
    assert!(env::var_os("HOME").is_none());
    match mode.as_str() {
        "lsp" => {
            let mut buffer = [0_u8; 4096];
            loop {
                let read = std::io::stdin().read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                std::io::stdout().write_all(&buffer[..read]).unwrap();
                std::io::stdout().flush().unwrap();
            }
        }
        "echo" => {
            println!("profile-ok");
            std::io::stdout().flush().unwrap();
            let mut buffer = [0_u8; 4096];
            loop {
                let read = std::io::stdin().read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                std::io::stdout().write_all(&buffer[..read]).unwrap();
                std::io::stdout().flush().unwrap();
            }
        }
        "stderr" => {
            for _ in 0..256 {
                std::io::stderr()
                    .write_all(b"secret-stderr-secret-stderr-secret-stderr\n")
                    .unwrap();
            }
            std::io::stderr().flush().unwrap();
            println!("stderr-drained");
            std::io::stdout().flush().unwrap();
            let mut byte = [0_u8; 1];
            let _ = std::io::stdin().read(&mut byte);
        }
        "hang" => {
            println!("hanging");
            std::io::stdout().flush().unwrap();
            loop {
                thread::sleep(Duration::from_secs(1));
            }
        }
        "nonreading" => {
            println!("not-reading");
            std::io::stdout().flush().unwrap();
            loop {
                thread::sleep(Duration::from_secs(1));
            }
        }
        "stdout-flood" => {
            for _ in 0..128 {
                std::io::stdout().write_all(&[b'z'; 4096]).unwrap();
            }
            std::io::stdout().flush().unwrap();
        }
        "tail" => {
            for _ in 0..32 {
                std::io::stdout().write_all(b"tail-prefix\n").unwrap();
            }
            std::io::stdout().write_all(b"tail-marker\n").unwrap();
            std::io::stdout().flush().unwrap();
        }
        "bad-close" => {
            println!("bad-close-ready");
            std::io::stdout().flush().unwrap();
            let mut byte = [0_u8; 1];
            while std::io::stdin().read(&mut byte).unwrap() != 0 {}
            std::process::exit(7);
        }
        "descendant-exit" => {
            let executable = env::current_exe().unwrap();
            let expected_cwd = env::var("VELLUM_EXPECTED_CWD").unwrap();
            let descendant = std::process::Command::new(executable)
                .args(["--exact", "tests::fake_lsp_child", "--nocapture"])
                .env("VELLUM_FAKE_MODE", "pipe-holder")
                .env("VELLUM_EXPECTED_CWD", expected_cwd)
                .spawn()
                .unwrap();
            // The hostile fixture intentionally abandons the live child while it
            // holds inherited output descriptors; the supervisor's process group
            // must kill it, not this leader's `Child` handle.
            std::mem::forget(descendant);
            println!("leader-exiting");
            std::io::stdout().flush().unwrap();
            std::process::exit(0);
        }
        "pipe-holder" => loop {
            thread::sleep(Duration::from_secs(1));
        },
        "crash" => std::process::exit(7),
        _ => panic!("unknown fake mode"),
    }
}
