use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Waker},
    time::Duration,
};

use vellum_adapter_semantic_lsp::{
    LspIndexReadinessPortV1, LspIndexReadinessV1, LspLimitsV1, LspReadinessEvidenceSourceV1,
    LspReceiveOutcomeV1, LspSendOutcomeV1, LspServerIdentityDispositionV1, LspTransportPortV1,
    LspTransportTerminalV1, LspVirtualDocumentIssuerV1, LspVirtualDocumentV1,
    SemanticGenerationIssuerV1,
};
use vellum_host::semantic::{
    SemanticCapabilitySetV1, SemanticDocumentRefV1, SemanticIndexGenerationV1, SemanticLimitsV1,
    SemanticProviderProvenanceV1, SemanticProviderRefV1, SemanticQueryV1, SemanticRequestRefV1,
    SemanticRequestV1, SemanticSourceRevisionV1, SemanticTargetV1, SemanticTextV1,
};
use vellum_semantic_service::{
    SemanticAdmissionOutcomeV1, SemanticBridgeForcedEffectPolicyV1, SemanticBridgeOpenAdmissionV1,
    SemanticBridgeServiceV1, SemanticClientRefV1, SemanticDeliveryOutcomeV1,
    SemanticProviderAdmissionV1, SemanticProviderCompletionV1, SemanticProviderPortV1,
    SemanticProviderReadinessV1, SemanticProviderStateV1, SemanticProviderSubmissionV1,
    SemanticServiceLimitsV1, SemanticServiceRefusalV1, SemanticServiceV1, SemanticSnapshotUpdateV1,
    SemanticSnapshotV1, SemanticSubmitOutcomeV1,
};

use super::*;
use crate::{SemanticProcessDeadlinesV1, SemanticProcessLimitsV1, SemanticProviderProfileV1};

fn run<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("model semantic future unexpectedly pending"),
    }
}

fn provenance() -> SemanticProviderProvenanceV1 {
    provenance_for(31)
}

fn provenance_for(seed: u8) -> SemanticProviderProvenanceV1 {
    SemanticProviderProvenanceV1::try_new(
        SemanticProviderRefV1::new([seed; 16]),
        "runtime-test".into(),
        "1.0.0".into(),
    )
    .unwrap()
}

fn runtime_bridge_limits() -> SemanticServiceLimitsV1 {
    let mut limits = SemanticServiceLimitsV1::reference();
    limits.max_clients = 1;
    limits.max_provider_polls_per_pump = 1;
    limits
}

fn profile_without_live_paths() -> SemanticProviderProfileV1 {
    let root = std::env::current_dir().unwrap();
    SemanticProviderProfileV1::try_new(
        provenance(),
        root.join("runtime-test-missing-executable"),
        Vec::new(),
        BTreeMap::new(),
        root.join("runtime-test-private-cwd"),
        root.join("runtime-test-workspace"),
        SemanticProcessDeadlinesV1 {
            launch: Duration::from_millis(10),
            graceful_shutdown: Duration::from_millis(10),
            kill_and_reap: Duration::from_millis(10),
        },
        SemanticProcessLimitsV1::reference(),
    )
    .unwrap()
}

#[derive(Default)]
struct GenerationIssuer(u64);

impl SemanticGenerationIssuerV1 for GenerationIssuer {
    fn issue(&mut self) -> Option<SemanticIndexGenerationV1> {
        self.0 = self.0.checked_add(1)?;
        SemanticIndexGenerationV1::new(self.0)
    }
}

struct RefusingTransport(LspSendOutcomeV1);

impl LspTransportPortV1 for RefusingTransport {
    fn try_send(&mut self, _frame: &[u8]) -> LspSendOutcomeV1 {
        self.0
    }

    fn poll_receive(&mut self) -> LspReceiveOutcomeV1 {
        LspReceiveOutcomeV1::Pending
    }

    fn close(&mut self) {}
}

struct LifecycleTransport {
    terminal: LspTransportTerminalV1,
    abort_terminal: LspTransportTerminalV1,
}

struct GracefulTransport {
    received: VecDeque<Vec<u8>>,
    terminal: LspTransportTerminalV1,
    shutdown_response: ShutdownResponseFixture,
}

#[derive(Clone, Copy)]
struct CurrentReadiness;

impl LspIndexReadinessPortV1 for CurrentReadiness {
    fn poll_readiness(&mut self) -> LspIndexReadinessV1 {
        LspIndexReadinessV1::Current
    }
}

#[derive(Clone, Copy)]
struct SingleDocumentIssuer;

impl LspVirtualDocumentIssuerV1 for SingleDocumentIssuer {
    fn issue(&mut self, _document: SemanticDocumentRefV1) -> Option<LspVirtualDocumentV1> {
        LspVirtualDocumentV1::try_new(
            "vellum-lsp://runtime-test-document".to_owned(),
            LspLimitsV1::reference(),
        )
    }
}

struct TombstoneTransport {
    received: VecDeque<Vec<u8>>,
    terminal: LspTransportTerminalV1,
}

impl Default for TombstoneTransport {
    fn default() -> Self {
        Self {
            received: VecDeque::new(),
            terminal: LspTransportTerminalV1::Active,
        }
    }
}

impl TombstoneTransport {
    fn queue(&mut self, response: &str) {
        self.received.push_back(
            format!("Content-Length: {}\r\n\r\n{response}", response.len()).into_bytes(),
        );
    }
}

impl LspTransportPortV1 for TombstoneTransport {
    fn try_send(&mut self, frame: &[u8]) -> LspSendOutcomeV1 {
        let body = String::from_utf8_lossy(frame);
        if body.contains("\"method\":\"initialize\"") {
            self.queue(
                r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{"positionEncoding":"utf-8","diagnosticProvider":{},"documentSymbolProvider":true}}}"#,
            );
        } else if body.contains("\"method\":\"textDocument/diagnostic\"") {
            self.queue(r#"{"jsonrpc":"2.0","id":2,"result":{"kind":"full","items":[]}}"#);
        } else if body.contains("\"method\":\"shutdown\"") {
            self.queue(r#"{"jsonrpc":"2.0","id":3,"result":null}"#);
        }
        LspSendOutcomeV1::Accepted
    }

    fn poll_receive(&mut self) -> LspReceiveOutcomeV1 {
        self.received
            .pop_front()
            .map_or(LspReceiveOutcomeV1::Pending, LspReceiveOutcomeV1::Chunk)
    }

    fn close(&mut self) {
        self.terminal = LspTransportTerminalV1::GracefulClosed;
    }

    fn abort(&mut self) {
        self.terminal = LspTransportTerminalV1::ForcedClosed;
    }

    fn terminal_state(&self) -> LspTransportTerminalV1 {
        self.terminal
    }
}

struct PanicOnEffectTransport {
    inner: TombstoneTransport,
    panic_on_send: Arc<AtomicBool>,
}

impl LspTransportPortV1 for PanicOnEffectTransport {
    fn try_send(&mut self, frame: &[u8]) -> LspSendOutcomeV1 {
        assert!(
            !self.panic_on_send.load(Ordering::SeqCst),
            "scripted provider effect panic"
        );
        self.inner.try_send(frame)
    }

    fn poll_receive(&mut self) -> LspReceiveOutcomeV1 {
        self.inner.poll_receive()
    }

    fn close(&mut self) {
        self.inner.close();
    }

    fn abort(&mut self) {
        self.inner.abort();
    }

    fn terminal_state(&self) -> LspTransportTerminalV1 {
        self.inner.terminal_state()
    }
}

#[derive(Clone, Copy)]
enum ShutdownResponseFixture {
    Valid,
    Missing,
    Malformed,
}

impl Default for GracefulTransport {
    fn default() -> Self {
        Self {
            received: VecDeque::new(),
            terminal: LspTransportTerminalV1::Active,
            shutdown_response: ShutdownResponseFixture::Valid,
        }
    }
}

impl GracefulTransport {
    fn with_shutdown_response(shutdown_response: ShutdownResponseFixture) -> Self {
        Self {
            shutdown_response,
            ..Self::default()
        }
    }
}

impl LspTransportPortV1 for GracefulTransport {
    fn try_send(&mut self, frame: &[u8]) -> LspSendOutcomeV1 {
        let body = String::from_utf8_lossy(frame);
        let response = if body.contains("\"method\":\"initialize\"") {
            Some(
                r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{"positionEncoding":"utf-8"}}}"#,
            )
        } else if body.contains("\"method\":\"shutdown\"") {
            match self.shutdown_response {
                ShutdownResponseFixture::Valid => Some(r#"{"jsonrpc":"2.0","id":2,"result":null}"#),
                ShutdownResponseFixture::Missing => None,
                ShutdownResponseFixture::Malformed => Some("not-json"),
            }
        } else {
            None
        };
        if let Some(response) = response {
            self.received.push_back(
                format!("Content-Length: {}\r\n\r\n{response}", response.len()).into_bytes(),
            );
        }
        LspSendOutcomeV1::Accepted
    }

    fn poll_receive(&mut self) -> LspReceiveOutcomeV1 {
        self.received
            .pop_front()
            .map_or(LspReceiveOutcomeV1::Pending, LspReceiveOutcomeV1::Chunk)
    }

    fn close(&mut self) {
        self.terminal = LspTransportTerminalV1::GracefulClosed;
    }

    fn abort(&mut self) {
        self.terminal = LspTransportTerminalV1::ForcedClosed;
    }

    fn terminal_state(&self) -> LspTransportTerminalV1 {
        self.terminal
    }
}

impl LspTransportPortV1 for LifecycleTransport {
    fn try_send(&mut self, _frame: &[u8]) -> LspSendOutcomeV1 {
        LspSendOutcomeV1::Accepted
    }

    fn poll_receive(&mut self) -> LspReceiveOutcomeV1 {
        match self.terminal {
            LspTransportTerminalV1::ForcedClosed => LspReceiveOutcomeV1::Closed,
            LspTransportTerminalV1::Failed => LspReceiveOutcomeV1::Failed,
            _ => LspReceiveOutcomeV1::Pending,
        }
    }

    fn close(&mut self) {
        self.terminal = LspTransportTerminalV1::GracefulClosed;
    }

    fn abort(&mut self) {
        self.terminal = self.abort_terminal;
    }

    fn terminal_state(&self) -> LspTransportTerminalV1 {
        self.terminal
    }
}

#[test]
fn pure_limit_refusals_happen_before_process_launch() {
    let mut lsp_limits = LspLimitsV1::reference();
    lsp_limits.max_headers = 0;
    let failure = LocalSemanticRuntimeV1::try_launch(
        profile_without_live_paths(),
        GenerationIssuer::default(),
        vellum_adapter_semantic_lsp::LspNeverReadyV1,
        vellum_adapter_semantic_lsp::LspExplicitVirtualDocumentsV1,
        lsp_limits,
        SemanticLimitsV1::reference(),
        SemanticServiceLimitsV1::reference(),
    )
    .err()
    .unwrap();
    assert_eq!(
        failure.refusal(),
        LocalSemanticRuntimeRefusalV1::InvalidAdapter
    );
    assert!(failure.into_cleanup().is_none());

    let mut service_limits = SemanticServiceLimitsV1::reference();
    service_limits.max_clients = 0;
    let failure = LocalSemanticRuntimeV1::try_launch(
        profile_without_live_paths(),
        GenerationIssuer::default(),
        vellum_adapter_semantic_lsp::LspNeverReadyV1,
        vellum_adapter_semantic_lsp::LspExplicitVirtualDocumentsV1,
        LspLimitsV1::reference(),
        SemanticLimitsV1::reference(),
        service_limits,
    )
    .err()
    .unwrap();
    assert_eq!(
        failure.refusal(),
        LocalSemanticRuntimeRefusalV1::InvalidService
    );

    let mut mismatched_peer = SemanticServiceLimitsV1::reference();
    mismatched_peer.peer.max_results -= 1;
    let failure = LocalSemanticRuntimeV1::try_launch(
        profile_without_live_paths(),
        GenerationIssuer::default(),
        vellum_adapter_semantic_lsp::LspNeverReadyV1,
        vellum_adapter_semantic_lsp::LspExplicitVirtualDocumentsV1,
        LspLimitsV1::reference(),
        SemanticLimitsV1::reference(),
        mismatched_peer,
    )
    .err()
    .unwrap();
    assert_eq!(
        failure.refusal(),
        LocalSemanticRuntimeRefusalV1::InvalidService
    );
}

#[test]
fn replacement_limits_cover_replay_and_provider_contracts() {
    let service = SemanticServiceLimitsV1::reference();
    let lsp = LspLimitsV1::reference();
    let semantic = SemanticLimitsV1::reference();
    assert!(replacement_limits_compatible(service, lsp, semantic));

    let mut wrong_peer = semantic;
    wrong_peer.max_results -= 1;
    assert!(!replacement_limits_compatible(service, lsp, wrong_peer));

    let mut too_few_documents = lsp;
    too_few_documents.max_documents = service.max_documents - 1;
    assert!(!replacement_limits_compatible(
        service,
        too_few_documents,
        semantic
    ));

    let mut too_few_source_bytes = lsp;
    too_few_source_bytes.max_total_source_bytes = service.max_total_snapshot_bytes - 1;
    assert!(!replacement_limits_compatible(
        service,
        too_few_source_bytes,
        semantic
    ));
}

#[test]
fn adapter_start_refusals_retain_unconfirmed_cleanup_evidence() {
    for (send, expected) in [
        (
            LspSendOutcomeV1::Unavailable,
            LocalSemanticRuntimeRefusalV1::StartCleanupUnconfirmed(LspStartOutcomeV1::Unavailable),
        ),
        (
            LspSendOutcomeV1::Refused,
            LocalSemanticRuntimeRefusalV1::StartCleanupUnconfirmed(LspStartOutcomeV1::Refused),
        ),
    ] {
        let adapter = LspAdapterV1::try_new(
            RefusingTransport(send),
            GenerationIssuer::default(),
            provenance(),
            LspLimitsV1::reference(),
        )
        .unwrap();
        let failure = try_start_adapter_v1(adapter, SemanticServiceLimitsV1::reference())
            .err()
            .unwrap();
        assert_eq!(failure.refusal, expected);
        let mut cleanup = failure.cleanup.expect("unconfirmed cleanup stays owned");
        assert_eq!(
            cleanup.transport_terminal_state(),
            LspTransportTerminalV1::Unobserved
        );
        assert_eq!(cleanup.abort(), LspAbortOutcomeV1::AlreadyTerminal);
    }
}

#[test]
fn adapter_escalation_uses_transport_abort_and_requires_terminal_evidence() {
    let transport = LifecycleTransport {
        terminal: LspTransportTerminalV1::Active,
        abort_terminal: LspTransportTerminalV1::ForcedClosed,
    };
    let mut adapter = LspAdapterV1::try_new(
        transport,
        GenerationIssuer::default(),
        provenance(),
        LspLimitsV1::reference(),
    )
    .unwrap();
    assert_eq!(adapter.start(), LspStartOutcomeV1::Started);
    assert_eq!(adapter.abort(), LspAbortOutcomeV1::Aborted);
    assert_eq!(
        adapter.transport_terminal_state(),
        LspTransportTerminalV1::ForcedClosed
    );
}

#[test]
fn protocol_shutdown_refusal_requires_explicit_escalation() {
    let adapter = LspAdapterV1::try_new(
        RefusingTransport(LspSendOutcomeV1::Refused),
        GenerationIssuer::default(),
        provenance(),
        LspLimitsV1::reference(),
    )
    .unwrap();
    let mut retirement = SemanticRetirementDriverV1 {
        retirement: None,
        adapter: Some(adapter),
        forced_shutdown: false,
        outcome: None,
        terminal_evidence: None,
        forced_abandonment: None,
        usage: LocalSemanticRetirementUsageV1::default(),
        quarantined_requests: 0,
    };
    assert_eq!(
        retirement.drive_once(),
        LocalSemanticRetirementStateV1::ProtocolShutdown
    );
    assert_eq!(retirement.usage().protocol_shutdown_refusals, 1);
    assert_eq!(retirement.usage().escalations, 0);
    assert_eq!(
        retirement.escalate(),
        LocalSemanticRetirementStateV1::ForcedShutdownPending
    );
    assert_eq!(retirement.usage().escalations, 1);
}

#[test]
fn retirement_escalation_waits_for_transport_terminal_evidence() {
    for (abort_terminal, expected) in [
        (
            LspTransportTerminalV1::ForcedClosed,
            LocalSemanticRetirementStateV1::ForcedConfirmed,
        ),
        (
            LspTransportTerminalV1::Failed,
            LocalSemanticRetirementStateV1::CleanupFailed,
        ),
    ] {
        let adapter = LspAdapterV1::try_new(
            LifecycleTransport {
                terminal: LspTransportTerminalV1::Active,
                abort_terminal,
            },
            GenerationIssuer::default(),
            provenance(),
            LspLimitsV1::reference(),
        )
        .unwrap();
        let mut retirement = SemanticRetirementDriverV1 {
            retirement: None,
            adapter: Some(adapter),
            forced_shutdown: false,
            outcome: None,
            terminal_evidence: None,
            forced_abandonment: None,
            usage: LocalSemanticRetirementUsageV1::default(),
            quarantined_requests: 7,
        };
        assert_eq!(
            retirement.state(),
            LocalSemanticRetirementStateV1::ProtocolShutdown
        );
        assert_eq!(
            retirement.escalate(),
            LocalSemanticRetirementStateV1::ForcedShutdownPending
        );
        assert_eq!(retirement.drive_once(), expected);
        assert_eq!(retirement.usage().escalations, 1);
        match expected {
            LocalSemanticRetirementStateV1::ForcedConfirmed => {
                assert_eq!(retirement.usage().forced_confirmed, 1)
            }
            LocalSemanticRetirementStateV1::CleanupFailed => {
                assert_eq!(retirement.usage().cleanup_failed, 1)
            }
            _ => unreachable!("fixture only selects terminal escalation outcomes"),
        }
        let receipt = match retirement.try_into_terminal_receipt() {
            Ok(receipt) => receipt,
            Err(_) => panic!("terminal retirement must yield affine evidence"),
        };
        assert_eq!(receipt.quarantined_requests(), 7);
        assert_eq!(receipt.usage().escalations, 1);
        let provider_evidence = receipt.provider_qualification();
        assert!(!provider_evidence.workspace_bound());
        assert_eq!(
            provider_evidence.server_identity(),
            LspServerIdentityDispositionV1::UnpinnedAbsent
        );
        assert_eq!(
            provider_evidence.readiness_source(),
            LspReadinessEvidenceSourceV1::Polling
        );
        assert_eq!(
            receipt.provider_usage().server_info_captured,
            0,
            "terminal receipt must retain the exact adapter's usage"
        );
        assert_eq!(
            receipt.terminal(),
            match expected {
                LocalSemanticRetirementStateV1::ForcedConfirmed => {
                    LocalSemanticRetirementTerminalV1::ForcedConfirmed
                }
                LocalSemanticRetirementStateV1::CleanupFailed => {
                    LocalSemanticRetirementTerminalV1::CleanupFailed
                }
                _ => unreachable!("fixture only selects terminal escalation outcomes"),
            }
        );
        let debug = format!("{receipt:?}");
        assert!(debug.contains("quarantined_requests: 7"));
        assert!(!debug.contains("runtime-test"));
    }
}

#[test]
fn ordered_bridge_provider_enters_affine_retirement_without_reuse() {
    let adapter = LspAdapterV1::try_new(
        LifecycleTransport {
            terminal: LspTransportTerminalV1::Active,
            abort_terminal: LspTransportTerminalV1::ForcedClosed,
        },
        GenerationIssuer::default(),
        provenance(),
        LspLimitsV1::reference(),
    )
    .unwrap();
    let fresh = adapter.try_start_fresh().unwrap();
    let mut limits = SemanticServiceLimitsV1::reference();
    limits.max_clients = 1;
    limits.max_provider_polls_per_pump = 1;
    let bridge = SemanticBridgeServiceV1::try_new(fresh, limits).unwrap();
    let provider = bridge.try_into_retiring_provider().unwrap();
    let retirement = SemanticRetirementDriverV1::from_bridge_provider(provider);
    assert_eq!(
        retirement.state(),
        LocalSemanticRetirementStateV1::ProtocolShutdown
    );
    let mut retirement = match retirement.try_into_terminal_receipt() {
        Ok(_) => panic!("nonterminal retirement must retain its driver"),
        Err(retirement) => *retirement,
    };
    assert_eq!(
        retirement.escalate(),
        LocalSemanticRetirementStateV1::ForcedShutdownPending
    );
    assert_eq!(
        retirement.drive_once(),
        LocalSemanticRetirementStateV1::ForcedConfirmed
    );
    let receipt = match retirement.try_into_terminal_receipt() {
        Ok(receipt) => receipt,
        Err(_) => panic!("forced terminal evidence must be consumable"),
    };
    assert_eq!(
        receipt.terminal(),
        LocalSemanticRetirementTerminalV1::ForcedConfirmed
    );
    assert_eq!(receipt.quarantined_requests(), 0);
}

#[test]
fn ordered_bridge_retirement_requires_protocol_close_before_graceful_receipt() {
    let adapter = LspAdapterV1::try_new(
        GracefulTransport::default(),
        GenerationIssuer::default(),
        provenance(),
        LspLimitsV1::reference(),
    )
    .unwrap();
    let fresh = adapter.try_start_fresh().unwrap();
    let mut limits = SemanticServiceLimitsV1::reference();
    limits.max_clients = 1;
    limits.max_provider_polls_per_pump = 1;
    let bridge = SemanticBridgeServiceV1::try_new(fresh, limits).unwrap();
    let provider = bridge.try_into_retiring_provider().unwrap();
    let mut retirement = SemanticRetirementDriverV1::from_bridge_provider(provider);

    assert_eq!(
        retirement.drive_once(),
        LocalSemanticRetirementStateV1::ProtocolShutdown
    );
    assert_eq!(
        retirement.drive_once(),
        LocalSemanticRetirementStateV1::ProtocolShutdown
    );
    assert_eq!(
        retirement.drive_once(),
        LocalSemanticRetirementStateV1::GracefulConfirmed
    );
    let receipt = match retirement.try_into_terminal_receipt() {
        Ok(receipt) => receipt,
        Err(_) => panic!("graceful terminal evidence must be consumable"),
    };
    assert_eq!(
        receipt.terminal(),
        LocalSemanticRetirementTerminalV1::GracefulConfirmed
    );
    assert_eq!(receipt.usage().graceful_confirmed, 1);
    assert_eq!(
        receipt.terminal_evidence(),
        LspTransportTerminalV1::GracefulClosed
    );
    assert_eq!(receipt.forced_abandonment(), None);
}

#[test]
fn nonzero_quarantine_drains_provider_tombstone_before_graceful_retirement() {
    let adapter = LspAdapterV1::try_new_with_readiness_and_documents(
        TombstoneTransport::default(),
        GenerationIssuer::default(),
        CurrentReadiness,
        SingleDocumentIssuer,
        provenance(),
        LspLimitsV1::reference(),
        SemanticLimitsV1::reference(),
    )
    .unwrap();
    let fresh = adapter.try_start_fresh().unwrap();
    let mut bridge = SemanticBridgeServiceV1::try_new(fresh, runtime_bridge_limits()).unwrap();
    let _ = bridge.poll_once();
    let client = SemanticClientRefV1::new([71; 16]);
    bridge.admit_client(client).unwrap();
    assert_eq!(
        bridge.open_snapshot(client, snapshot()).outcome(),
        &SemanticBridgeOpenAdmissionV1::Accepted
    );
    let offered = request(72);
    assert_eq!(
        bridge.submit(client, offered.clone()).outcome(),
        &SemanticSubmitOutcomeV1::Accepted
    );
    let _ = bridge.cancel(client, offered.request);
    let _ = bridge.poll_once();
    let _ = bridge.retire_client(client);
    let provider = bridge.try_into_retiring_provider().unwrap();
    assert_eq!(provider.quarantined_requests(), 1);

    let mut retirement = SemanticRetirementDriverV1::from_bridge_provider(provider);
    assert_eq!(
        retirement.drive_once(),
        LocalSemanticRetirementStateV1::ProtocolShutdown
    );
    assert_eq!(
        retirement.drive_once(),
        LocalSemanticRetirementStateV1::GracefulConfirmed
    );
    let receipt = match retirement.try_into_terminal_receipt() {
        Ok(receipt) => receipt,
        Err(_) => panic!("graceful tombstone retirement must yield evidence"),
    };
    assert_eq!(receipt.quarantined_requests(), 1);
    assert_eq!(
        receipt.terminal(),
        LocalSemanticRetirementTerminalV1::GracefulConfirmed
    );
}

#[test]
fn escalation_observes_completed_graceful_close_before_abort_effect() {
    let adapter = LspAdapterV1::try_new(
        GracefulTransport::default(),
        GenerationIssuer::default(),
        provenance(),
        LspLimitsV1::reference(),
    )
    .unwrap();
    let fresh = adapter.try_start_fresh().unwrap();
    let bridge = SemanticBridgeServiceV1::try_new(fresh, runtime_bridge_limits()).unwrap();
    let provider = bridge.try_into_retiring_provider().unwrap();
    let mut retirement = SemanticRetirementDriverV1::from_bridge_provider(provider);
    assert_eq!(
        retirement.drive_once(),
        LocalSemanticRetirementStateV1::ProtocolShutdown
    );
    assert_eq!(
        retirement.drive_once(),
        LocalSemanticRetirementStateV1::ProtocolShutdown
    );
    assert_eq!(
        retirement.escalate(),
        LocalSemanticRetirementStateV1::GracefulConfirmed
    );
    assert_eq!(retirement.usage().escalations, 0);
    let receipt = match retirement.try_into_terminal_receipt() {
        Ok(receipt) => receipt,
        Err(_) => panic!("pre-abort graceful closure must yield evidence"),
    };
    assert_eq!(
        receipt.terminal(),
        LocalSemanticRetirementTerminalV1::GracefulConfirmed
    );
    assert_eq!(
        receipt.terminal_evidence(),
        LspTransportTerminalV1::GracefulClosed
    );
}

#[test]
fn missing_or_malformed_shutdown_requires_explicit_forced_retirement() {
    for shutdown_response in [
        ShutdownResponseFixture::Missing,
        ShutdownResponseFixture::Malformed,
    ] {
        let adapter = LspAdapterV1::try_new(
            GracefulTransport::with_shutdown_response(shutdown_response),
            GenerationIssuer::default(),
            provenance(),
            LspLimitsV1::reference(),
        )
        .unwrap();
        let fresh = adapter.try_start_fresh().unwrap();
        let mut limits = SemanticServiceLimitsV1::reference();
        limits.max_clients = 1;
        limits.max_provider_polls_per_pump = 1;
        let bridge = SemanticBridgeServiceV1::try_new(fresh, limits).unwrap();
        let provider = bridge.try_into_retiring_provider().unwrap();
        let mut retirement = SemanticRetirementDriverV1::from_bridge_provider(provider);

        assert_eq!(
            retirement.drive_once(),
            LocalSemanticRetirementStateV1::ProtocolShutdown
        );
        assert_eq!(
            retirement.drive_once(),
            LocalSemanticRetirementStateV1::ProtocolShutdown
        );
        assert_eq!(
            retirement.escalate(),
            LocalSemanticRetirementStateV1::ForcedShutdownPending
        );
        assert_eq!(
            retirement.drive_once(),
            LocalSemanticRetirementStateV1::ForcedConfirmed
        );
        let receipt = match retirement.try_into_terminal_receipt() {
            Ok(receipt) => receipt,
            Err(_) => panic!("forced cleanup proof must yield a receipt"),
        };
        assert_eq!(
            receipt.terminal(),
            LocalSemanticRetirementTerminalV1::ForcedConfirmed
        );
        assert_eq!(receipt.usage().escalations, 1);
    }
}

#[test]
fn forced_skip_preserves_exact_abandonment_and_only_confirms_forced_cleanup() {
    let adapter = LspAdapterV1::try_new_with_readiness_and_documents(
        TombstoneTransport::default(),
        GenerationIssuer::default(),
        CurrentReadiness,
        SingleDocumentIssuer,
        provenance(),
        LspLimitsV1::reference(),
        SemanticLimitsV1::reference(),
    )
    .unwrap();
    let fresh = adapter.try_start_fresh().unwrap();
    let mut bridge = SemanticBridgeServiceV1::try_new(fresh, runtime_bridge_limits()).unwrap();
    let _ = bridge.poll_once();
    let client = SemanticClientRefV1::new([73; 16]);
    bridge.admit_client(client).unwrap();
    let _ = bridge.open_snapshot(client, snapshot());
    let offered = request(74);
    let _ = bridge.submit(client, offered);
    let forced =
        bridge.force_into_retiring_provider(SemanticBridgeForcedEffectPolicyV1::SkipUnsafeProvider);
    let mut retirement = SemanticRetirementDriverV1::from_forced_bridge_provider(forced);
    assert_eq!(
        retirement.state(),
        LocalSemanticRetirementStateV1::ForcedShutdownPending
    );
    assert_eq!(
        retirement.drive_once(),
        LocalSemanticRetirementStateV1::ForcedConfirmed
    );
    let receipt = match retirement.try_into_terminal_receipt() {
        Ok(receipt) => receipt,
        Err(_) => panic!("forced terminal proof must yield joined evidence"),
    };
    assert_eq!(
        receipt.terminal(),
        LocalSemanticRetirementTerminalV1::ForcedConfirmed
    );
    assert_eq!(
        receipt.terminal_evidence(),
        LspTransportTerminalV1::ForcedClosed
    );
    let abandonment = receipt
        .forced_abandonment()
        .expect("forced receipt retains bridge abandonment");
    assert_eq!(abandonment.clients_abandoned, 1);
    assert_eq!(abandonment.documents_abandoned, 1);
    assert_eq!(abandonment.pending_requests_abandoned, 1);
    assert_eq!(abandonment.cancellation_signals_skipped, 1);
    assert_eq!(abandonment.closes_skipped, 1);
    assert!(!abandonment.provider_panicked);
    assert_eq!(receipt.usage().forced_only_starts, 1);
    let debug = format!("{receipt:?}");
    assert!(debug.contains("forced_abandonment"));
    assert!(!debug.contains("runtime-test"));
    assert!(!debug.contains("fn main"));
}

#[test]
fn forced_provider_panic_is_preserved_through_terminal_cleanup() {
    let panic_on_send = Arc::new(AtomicBool::new(false));
    let adapter = LspAdapterV1::try_new_with_readiness_and_documents(
        PanicOnEffectTransport {
            inner: TombstoneTransport::default(),
            panic_on_send: Arc::clone(&panic_on_send),
        },
        GenerationIssuer::default(),
        CurrentReadiness,
        SingleDocumentIssuer,
        provenance(),
        LspLimitsV1::reference(),
        SemanticLimitsV1::reference(),
    )
    .unwrap();
    let fresh = adapter.try_start_fresh().unwrap();
    let mut bridge = SemanticBridgeServiceV1::try_new(fresh, runtime_bridge_limits()).unwrap();
    let _ = bridge.poll_once();
    let client = SemanticClientRefV1::new([75; 16]);
    bridge.admit_client(client).unwrap();
    let _ = bridge.open_snapshot(client, snapshot());
    let offered = request(76);
    let _ = bridge.submit(client, offered);
    panic_on_send.store(true, Ordering::SeqCst);
    let forced =
        bridge.force_into_retiring_provider(SemanticBridgeForcedEffectPolicyV1::AttemptBounded);
    let mut retirement = SemanticRetirementDriverV1::from_forced_bridge_provider(forced);
    assert_eq!(
        retirement.drive_once(),
        LocalSemanticRetirementStateV1::ForcedConfirmed
    );
    let receipt = match retirement.try_into_terminal_receipt() {
        Ok(receipt) => receipt,
        Err(_) => panic!("forced cleanup after provider panic must retain evidence"),
    };
    let abandonment = receipt
        .forced_abandonment()
        .expect("provider panic evidence remains joined");
    assert!(abandonment.provider_panicked);
    assert_eq!(abandonment.cancellation_signals_attempted, 1);
    assert_eq!(abandonment.closes_skipped, 1);
    assert_eq!(
        receipt.terminal(),
        LocalSemanticRetirementTerminalV1::ForcedConfirmed
    );
}

#[test]
fn forced_malformed_or_unconfirmed_transport_never_claims_graceful_cleanup() {
    let forced = |abort_terminal| {
        let adapter = LspAdapterV1::try_new(
            LifecycleTransport {
                terminal: LspTransportTerminalV1::Active,
                abort_terminal,
            },
            GenerationIssuer::default(),
            provenance(),
            LspLimitsV1::reference(),
        )
        .unwrap();
        let fresh = adapter.try_start_fresh().unwrap();
        let bridge = SemanticBridgeServiceV1::try_new(fresh, runtime_bridge_limits()).unwrap();
        bridge.force_into_retiring_provider(SemanticBridgeForcedEffectPolicyV1::SkipUnsafeProvider)
    };

    let mut malformed = SemanticRetirementDriverV1::from_forced_bridge_provider(forced(
        LspTransportTerminalV1::GracefulClosed,
    ));
    assert_eq!(
        malformed.drive_once(),
        LocalSemanticRetirementStateV1::CleanupFailed
    );
    let malformed = match malformed.try_into_terminal_receipt() {
        Ok(receipt) => receipt,
        Err(_) => panic!("malformed forced closure is terminal failure evidence"),
    };
    assert_eq!(
        malformed.terminal(),
        LocalSemanticRetirementTerminalV1::CleanupFailed
    );
    assert_eq!(
        malformed.terminal_evidence(),
        LspTransportTerminalV1::GracefulClosed
    );
    assert!(malformed.forced_abandonment().is_some());

    let mut unconfirmed = SemanticRetirementDriverV1::from_forced_bridge_provider(forced(
        LspTransportTerminalV1::Active,
    ));
    assert_eq!(
        unconfirmed.drive_once(),
        LocalSemanticRetirementStateV1::ForcedShutdownPending
    );
    let unconfirmed = match unconfirmed.try_into_terminal_receipt() {
        Ok(_) => panic!("active transport cannot prove forced cleanup"),
        Err(unconfirmed) => unconfirmed,
    };
    assert_eq!(
        unconfirmed.state(),
        LocalSemanticRetirementStateV1::ForcedShutdownPending
    );
}

#[test]
fn transport_exit_without_protocol_shutdown_is_not_graceful() {
    let adapter = LspAdapterV1::try_new(
        LifecycleTransport {
            terminal: LspTransportTerminalV1::GracefulClosed,
            abort_terminal: LspTransportTerminalV1::ForcedClosed,
        },
        GenerationIssuer::default(),
        provenance(),
        LspLimitsV1::reference(),
    )
    .unwrap();
    let mut retirement = SemanticRetirementDriverV1 {
        retirement: None,
        adapter: Some(adapter),
        forced_shutdown: false,
        outcome: None,
        terminal_evidence: None,
        forced_abandonment: None,
        usage: LocalSemanticRetirementUsageV1::default(),
        quarantined_requests: 0,
    };
    assert_eq!(
        retirement.drive_once(),
        LocalSemanticRetirementStateV1::CleanupFailed
    );
    assert_eq!(retirement.usage().graceful_confirmed, 0);
    assert_eq!(retirement.usage().cleanup_failed, 1);
    let receipt = match retirement.try_into_terminal_receipt() {
        Ok(receipt) => receipt,
        Err(_) => panic!("terminal failure must yield affine evidence"),
    };
    assert_eq!(
        receipt.terminal(),
        LocalSemanticRetirementTerminalV1::CleanupFailed
    );
}

struct ModelProvider {
    state: SemanticProviderStateV1,
    after_poll: Option<SemanticProviderStateV1>,
    admissions: usize,
}

struct CapacityDeadlineCore {
    client: SemanticClientRefV1,
    request: SemanticRequestRefV1,
    pending: bool,
    refuse_capacity: bool,
    drives: usize,
}

impl SemanticDeadlineCoreV1 for CapacityDeadlineCore {
    fn deadline_is_pending_for(
        &self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> Result<bool, SemanticServiceRefusalV1> {
        if client != self.client {
            return Err(SemanticServiceRefusalV1::ClientIsolation);
        }
        Ok(self.pending && request == self.request)
    }

    fn deadline_expire(
        &mut self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> Result<bool, SemanticServiceRefusalV1> {
        assert_eq!((client, request), (self.client, self.request));
        if self.refuse_capacity {
            return Err(SemanticServiceRefusalV1::Capacity);
        }
        let was_pending = self.pending;
        self.pending = false;
        Ok(was_pending)
    }

    fn deadline_drive_once(&mut self) {
        self.drives += 1;
    }
}

struct FairDeadlineCore {
    client: SemanticClientRefV1,
    pending: BTreeSet<SemanticRequestRefV1>,
    saturated: SemanticRequestRefV1,
}

impl SemanticDeadlineCoreV1 for FairDeadlineCore {
    fn deadline_is_pending_for(
        &self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> Result<bool, SemanticServiceRefusalV1> {
        if client != self.client {
            return Err(SemanticServiceRefusalV1::ClientIsolation);
        }
        Ok(self.pending.contains(&request))
    }

    fn deadline_expire(
        &mut self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> Result<bool, SemanticServiceRefusalV1> {
        assert_eq!(client, self.client);
        if request == self.saturated {
            return Err(SemanticServiceRefusalV1::Capacity);
        }
        Ok(self.pending.remove(&request))
    }

    fn deadline_drive_once(&mut self) {}
}

impl SemanticProviderPortV1 for ModelProvider {
    fn state(&self) -> SemanticProviderStateV1 {
        self.state.clone()
    }

    fn synchronize(&mut self, _update: &SemanticSnapshotUpdateV1) -> SemanticProviderAdmissionV1 {
        self.admissions += 1;
        SemanticProviderAdmissionV1::Accepted
    }

    fn submit(&mut self, _request: &SemanticRequestV1) -> SemanticProviderSubmissionV1 {
        SemanticProviderSubmissionV1::Accepted
    }

    fn cancel(&mut self, _request: SemanticRequestRefV1) {}

    fn poll(&mut self) -> Option<SemanticProviderCompletionV1> {
        if let Some(state) = self.after_poll.take() {
            self.state = state;
        }
        None
    }
}

fn provider_state(readiness: SemanticProviderReadinessV1) -> SemanticProviderStateV1 {
    SemanticProviderStateV1 {
        provider: provenance(),
        readiness,
        capabilities: SemanticCapabilitySetV1::read_only(),
    }
}

fn snapshot() -> SemanticSnapshotV1 {
    SemanticSnapshotV1::try_new(
        SemanticTargetV1 {
            document: SemanticDocumentRefV1::new([41; 16]),
            source_revision: SemanticSourceRevisionV1::new(0),
        },
        "rust".into(),
        "fn main() {}".into(),
        SemanticServiceLimitsV1::reference(),
    )
    .unwrap()
}

fn request(reference: u8) -> SemanticRequestV1 {
    SemanticRequestV1 {
        request: SemanticRequestRefV1::new([reference; 16]),
        target: snapshot().target(),
        expected_index: Some(SemanticIndexGenerationV1::new(1).unwrap()),
        language: SemanticTextV1::try_new("rust".into(), 16).unwrap(),
        query: SemanticQueryV1::Diagnostics,
    }
}

#[test]
fn facade_does_not_admit_or_retarget_clients_and_drive_is_bounded() {
    let ready = provider_state(SemanticProviderReadinessV1::Ready {
        generation: SemanticIndexGenerationV1::new(1).unwrap(),
    });
    let provider = ModelProvider {
        state: provider_state(SemanticProviderReadinessV1::Indexing),
        after_poll: Some(ready.clone()),
        admissions: 0,
    };
    let service =
        SemanticServiceV1::try_new(provider, SemanticServiceLimitsV1::reference()).unwrap();
    let mut core = SemanticRuntimeCoreV1 { service };
    let admitted = SemanticClientRefV1::new([51; 16]);
    let stranger = SemanticClientRefV1::new([52; 16]);

    {
        let mut client = core.bind_client(stranger);
        assert_eq!(
            run(client.open_snapshot(snapshot())),
            SemanticAdmissionOutcomeV1::Refused(SemanticServiceRefusalV1::UnknownClient)
        );
    }
    assert_eq!(core.usage().snapshots_admitted, 0);

    core.admit_client(admitted).unwrap();
    {
        let mut client = core.bind_client(admitted);
        assert_eq!(
            run(client.open_snapshot(snapshot())),
            SemanticAdmissionOutcomeV1::Accepted
        );
    }
    {
        let mut client = core.bind_client(stranger);
        assert_eq!(
            run(client.close_snapshot(snapshot().target())),
            SemanticAdmissionOutcomeV1::Refused(SemanticServiceRefusalV1::UnknownClient)
        );
    }
    assert_eq!(core.usage().snapshots_closed, 0);
    assert_eq!(
        core.state().readiness,
        SemanticProviderReadinessV1::Indexing
    );
    core.drive_once();
    assert_eq!(core.state().readiness, ready.readiness);
    assert_eq!(core.state().capabilities, ready.capabilities);
}

#[test]
fn composition_ticks_expire_bounded_work_without_retry_or_clock_inference() {
    let provider = ModelProvider {
        state: provider_state(SemanticProviderReadinessV1::Ready {
            generation: SemanticIndexGenerationV1::new(1).unwrap(),
        }),
        after_poll: None,
        admissions: 0,
    };
    let service =
        SemanticServiceV1::try_new(provider, SemanticServiceLimitsV1::reference()).unwrap();
    let mut core = SemanticRuntimeCoreV1 { service };
    let client_ref = SemanticClientRefV1::new([61; 16]);
    core.admit_client(client_ref).unwrap();
    {
        let mut client = core.bind_client(client_ref);
        assert_eq!(
            run(client.open_snapshot(snapshot())),
            SemanticAdmissionOutcomeV1::Accepted
        );
        assert_eq!(
            run(client.submit(request(71))),
            SemanticSubmitOutcomeV1::Accepted
        );
        assert_eq!(
            run(client.submit(request(72))),
            SemanticSubmitOutcomeV1::Accepted
        );
    }
    let mut scheduler = LocalSemanticDeadlineSchedulerV1::try_new(LocalSemanticSchedulerLimitsV1 {
        max_deadlines: 2,
        max_expirations_per_drive: 1,
    })
    .unwrap();
    let at_ten = LocalSemanticInstantV1::from_composition_tick(10);
    let at_twenty = LocalSemanticInstantV1::from_composition_tick(20);
    assert_eq!(
        scheduler.schedule(&core, client_ref, request(71).request, at_twenty),
        LocalSemanticDeadlineAdmissionV1::Scheduled
    );
    assert_eq!(
        scheduler.schedule(&core, client_ref, request(71).request, at_twenty),
        LocalSemanticDeadlineAdmissionV1::AlreadyScheduled
    );
    assert_eq!(
        scheduler.schedule(
            &core,
            client_ref,
            request(71).request,
            LocalSemanticInstantV1::from_composition_tick(21)
        ),
        LocalSemanticDeadlineAdmissionV1::Conflict
    );
    assert_eq!(
        scheduler.schedule(&core, client_ref, request(72).request, at_ten),
        LocalSemanticDeadlineAdmissionV1::Scheduled
    );
    let fabricated = request(73).request;
    assert_eq!(
        scheduler.schedule(&core, client_ref, fabricated, at_twenty),
        LocalSemanticDeadlineAdmissionV1::NotPending
    );
    let other_client = SemanticClientRefV1::new([62; 16]);
    core.admit_client(other_client).unwrap();
    assert_eq!(
        scheduler.schedule(&core, other_client, request(72).request, at_twenty),
        LocalSemanticDeadlineAdmissionV1::Isolation
    );
    assert_eq!(scheduler.usage.current, 2);
    let first = scheduler.drive(&mut core, at_twenty).unwrap();
    assert_eq!(first.attempted, 1);
    assert_eq!(first.expired, 1);
    assert!(first.remaining_due);
    let second = scheduler.drive(&mut core, at_twenty).unwrap();
    assert_eq!(second.expired, 1);
    assert!(!second.remaining_due);
    assert_eq!(scheduler.usage.current, 0);
    assert_eq!(scheduler.usage.high_water, 2);
    assert_eq!(scheduler.usage.activation_lateness_ticks_max, 10);
    {
        let mut client = core.bind_client(client_ref);
        assert_eq!(
            run(client.submit(request(74))),
            SemanticSubmitOutcomeV1::Accepted
        );
    }
    assert_eq!(
        scheduler.schedule(
            &core,
            client_ref,
            request(74).request,
            LocalSemanticInstantV1::from_composition_tick(30)
        ),
        LocalSemanticDeadlineAdmissionV1::Scheduled
    );
    assert!(scheduler.clear(client_ref, request(74).request));
    assert!(!scheduler.clear(client_ref, request(74).request));
    assert_eq!(scheduler.usage.cleared_before_deadline, 1);
    assert_eq!(
        scheduler.drive(&mut core, LocalSemanticInstantV1::from_composition_tick(9)),
        Err(LocalSemanticDriveRefusalV1::ClockRegressed)
    );
    {
        let mut client = core.bind_client(client_ref);
        for expected in [request(72).request, request(71).request] {
            let delivery = run(client.take_delivery()).unwrap().unwrap();
            assert_eq!(delivery.request, expected);
            assert_eq!(delivery.outcome, SemanticDeliveryOutcomeV1::TimedOut);
        }
    }
}

#[test]
fn retryable_expiry_capacity_retains_the_exact_deadline() {
    let client = SemanticClientRefV1::new([91; 16]);
    let request = SemanticRequestRefV1::new([92; 16]);
    let mut core = CapacityDeadlineCore {
        client,
        request,
        pending: true,
        refuse_capacity: true,
        drives: 0,
    };
    let mut scheduler = LocalSemanticDeadlineSchedulerV1::try_new(LocalSemanticSchedulerLimitsV1 {
        max_deadlines: 1,
        max_expirations_per_drive: 1,
    })
    .unwrap();
    let deadline = LocalSemanticInstantV1::from_composition_tick(5);
    assert_eq!(
        scheduler.schedule(&core, client, request, deadline),
        LocalSemanticDeadlineAdmissionV1::Scheduled
    );
    let first = scheduler.drive(&mut core, deadline).unwrap();
    assert_eq!(first.retained_retryable, 1);
    assert!(first.remaining_due);
    assert_eq!(scheduler.usage.current, 1);
    assert_eq!(scheduler.usage.retryable_expiry_refusals, 1);

    core.refuse_capacity = false;
    let second = scheduler.drive(&mut core, deadline).unwrap();
    assert_eq!(second.expired, 1);
    assert!(!second.remaining_due);
    assert_eq!(scheduler.usage.current, 0);
    assert_eq!(core.drives, 2);
}

#[test]
fn saturated_earliest_deadline_cannot_starve_later_due_work() {
    let client = SemanticClientRefV1::new([93; 16]);
    let saturated = SemanticRequestRefV1::new([1; 16]);
    let later = SemanticRequestRefV1::new([2; 16]);
    let mut core = FairDeadlineCore {
        client,
        pending: BTreeSet::from([saturated, later]),
        saturated,
    };
    let mut scheduler = LocalSemanticDeadlineSchedulerV1::try_new(LocalSemanticSchedulerLimitsV1 {
        max_deadlines: 2,
        max_expirations_per_drive: 1,
    })
    .unwrap();
    assert_eq!(
        scheduler.schedule(
            &core,
            client,
            saturated,
            LocalSemanticInstantV1::from_composition_tick(5)
        ),
        LocalSemanticDeadlineAdmissionV1::Scheduled
    );
    assert_eq!(
        scheduler.schedule(
            &core,
            client,
            later,
            LocalSemanticInstantV1::from_composition_tick(6)
        ),
        LocalSemanticDeadlineAdmissionV1::Scheduled
    );
    let now = LocalSemanticInstantV1::from_composition_tick(10);
    let first = scheduler.drive(&mut core, now).unwrap();
    assert_eq!((first.retained_retryable, first.expired), (1, 0));
    let second = scheduler.drive(&mut core, now).unwrap();
    assert_eq!((second.retained_retryable, second.expired), (0, 1));
    assert!(!core.pending.contains(&later));
    assert!(core.pending.contains(&saturated));
    assert!(second.remaining_due);
}

#[test]
fn replacement_core_replays_before_swap_and_returns_exact_old_retirement() {
    let current_state = provider_state(SemanticProviderReadinessV1::Ready {
        generation: SemanticIndexGenerationV1::new(1).unwrap(),
    });
    let current = ModelProvider {
        state: current_state,
        after_poll: None,
        admissions: 0,
    };
    let service =
        SemanticServiceV1::try_new(current, SemanticServiceLimitsV1::reference()).unwrap();
    let mut core = SemanticRuntimeCoreV1 { service };
    let client = SemanticClientRefV1::new([81; 16]);
    core.admit_client(client).unwrap();
    {
        let mut bound = core.bind_client(client);
        assert_eq!(
            run(bound.open_snapshot(snapshot())),
            SemanticAdmissionOutcomeV1::Accepted
        );
    }
    let replacement = ModelProvider {
        state: SemanticProviderStateV1 {
            provider: provenance_for(32),
            readiness: SemanticProviderReadinessV1::Ready {
                generation: SemanticIndexGenerationV1::new(2).unwrap(),
            },
            capabilities: SemanticCapabilitySetV1::read_only(),
        },
        after_poll: None,
        admissions: 0,
    };
    let retirement = core.try_replace_provider(replacement).unwrap();
    assert_eq!(
        retirement.state(),
        SemanticProviderRetirementStateV1::Drained
    );
    assert_eq!(retirement.usage().close_attempts, 1);
    assert_eq!(retirement.usage().closes_accepted, 1);
    assert_eq!(
        core.state().readiness,
        SemanticProviderReadinessV1::Ready {
            generation: SemanticIndexGenerationV1::new(2).unwrap()
        }
    );
    assert_eq!(core.usage().snapshots_replayed, 1);
    assert_eq!(core.usage().provider_replacements, 1);
}
