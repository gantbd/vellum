use std::{
    collections::BTreeMap,
    env, fs,
    os::unix::{ffi::OsStrExt as _, fs::PermissionsExt as _},
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

use vellum_adapter_semantic_lsp::{
    LspIndexReadinessPortV1, LspIndexReadinessV1, LspLimitsV1, LspProviderProtocolProfileV1,
    LspReadinessEvidenceSourceV1, LspReadinessNotificationV1, LspServerIdentityDispositionV1,
    LspVirtualDocumentIssuerV1, LspVirtualDocumentV1, SemanticGenerationIssuerV1,
};
use vellum_host::{
    authority::{AuthoritySessionRefV1, AuthoritySubjectRefV1},
    semantic::{
        SemanticDiagnosticSeverityV1, SemanticDocumentRefV1, SemanticIndexGenerationV1,
        SemanticProviderProvenanceV1, SemanticProviderRefV1, SemanticQueryV1, SemanticReplyV1,
        SemanticRequestRefV1, SemanticRequestV1, SemanticSourceRevisionV1, SemanticSpanV1,
        SemanticSymbolKindV1, SemanticTargetV1, SemanticTextV1,
    },
};
use vellum_semantic_protocol::{
    SemanticExchangeDispositionV1, SemanticExchangeRefV1, SemanticMembershipRefV1,
    SemanticProviderReadinessProjectionV1, SemanticSessionCommandV1, SemanticSessionIncarnationV1,
    SemanticSessionLimitsV1, SemanticSessionOperationRefV1, SemanticSessionOutcomeV1,
    SemanticSessionRolesV1, SemanticSynchronizationEpochV1, SemanticSynchronizationManifestV1,
};
use vellum_semantic_service::{SemanticClientRefV1, SemanticServiceLimitsV1};
use vellum_semantic_session_actor::{
    SemanticSessionActorAdmissionV1, SemanticSessionActorControlCompletionV1,
    SemanticSessionActorLimitsV1, SemanticSessionActorPollV1, SemanticSessionActorTerminationV1,
};
use vellum_semantic_session_broker::SemanticBrokerContextV1;

use super::*;

const MATRIX_LSP_SERVER: &str = r###"
use std::io::{Read, Write};
use std::time::Duration;

fn write_frame(output: &mut impl Write, body: &str) {
    write!(output, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    output.flush().unwrap();
}

fn request_id(body: &str) -> u64 {
    let marker = "\"id\":";
    let start = body.find(marker).unwrap() + marker.len();
    body[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap()
}

fn main() {
    let mode = std::env::var("VELLUM_MATRIX_MODE").unwrap();
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    let mut buffered = Vec::new();
    loop {
        let mut chunk = [0_u8; 4096];
        let read = input.read(&mut chunk).unwrap();
        if read == 0 { return; }
        buffered.extend_from_slice(&chunk[..read]);
        loop {
            let Some(header_end) = buffered.windows(4).position(|bytes| bytes == b"\r\n\r\n") else { break; };
            let headers = std::str::from_utf8(&buffered[..header_end]).unwrap();
            let length = headers.lines()
                .find_map(|line| line.strip_prefix("Content-Length: "))
                .unwrap().parse::<usize>().unwrap();
            let body_start = header_end + 4;
            if buffered.len() < body_start + length { break; }
            let body = String::from_utf8(buffered[body_start..body_start + length].to_vec()).unwrap();
            buffered.drain(..body_start + length);
            if body.contains("\"method\":\"initialize\"") {
                let expected_root = std::env::var("VELLUM_MATRIX_ROOT_URI").unwrap();
                assert!(body.contains(&format!("\"rootUri\":\"{expected_root}\"")));
                assert!(body.contains(&format!("\"uri\":\"{expected_root}\"")));
                write_frame(&mut output, "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"capabilities\":{\"positionEncoding\":\"utf-8\",\"diagnosticProvider\":{},\"documentSymbolProvider\":true,\"renameProvider\":true},\"serverInfo\":{\"name\":\"vellum-matrix-lsp\",\"version\":\"1.0.0\"}}}");
                if mode == "close-after-initialize" {
                    std::thread::sleep(Duration::from_millis(750));
                    return;
                }
            } else if body.contains("\"method\":\"initialized\"") {
                if mode == "pinned-serve" {
                    write_frame(&mut output, "{\"jsonrpc\":\"2.0\",\"method\":\"$/vellumStatus\",\"params\":{\"health\":\"ok\",\"quiescent\":true}}");
                }
            } else if body.contains("\"method\":\"textDocument/diagnostic\"") {
                write_frame(&mut output, &format!("{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{{\"kind\":\"full\",\"items\":[{{\"range\":{{\"start\":{{\"line\":0,\"character\":3}},\"end\":{{\"line\":0,\"character\":8}}}},\"severity\":2,\"message\":\"fixture warning\",\"code\":\"W1\"}}]}}}}", request_id(&body)));
            } else if body.contains("\"method\":\"textDocument/documentSymbol\"") {
                write_frame(&mut output, &format!("{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":[{{\"name\":\"alpha\",\"kind\":12,\"range\":{{\"start\":{{\"line\":0,\"character\":0}},\"end\":{{\"line\":0,\"character\":13}}}},\"selectionRange\":{{\"start\":{{\"line\":0,\"character\":3}},\"end\":{{\"line\":0,\"character\":8}}}},\"children\":[{{\"name\":\"alpha_inner\",\"kind\":13,\"range\":{{\"start\":{{\"line\":0,\"character\":3}},\"end\":{{\"line\":0,\"character\":8}}}},\"selectionRange\":{{\"start\":{{\"line\":0,\"character\":3}},\"end\":{{\"line\":0,\"character\":8}}}}}}]}}]}}", request_id(&body)));
            } else if body.contains("\"method\":\"textDocument/rename\"") {
                write_frame(&mut output, &format!("{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{{\"documentChanges\":[{{\"textDocument\":{{\"uri\":\"vellum-lsp://supervised-matrix-document\",\"version\":1}},\"edits\":[{{\"range\":{{\"start\":{{\"line\":0,\"character\":3}},\"end\":{{\"line\":0,\"character\":8}}}},\"newText\":\"beta\"}}]}}]}}}}", request_id(&body)));
            } else if body.contains("\"method\":\"shutdown\"") {
                write_frame(&mut output, &format!("{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":null}}", request_id(&body)));
            } else if body.contains("\"method\":\"exit\"") {
                return;
            }
        }
    }
}
"###;

struct MatrixScratch {
    root: PathBuf,
    server: PathBuf,
}

impl MatrixScratch {
    fn new(label: &str) -> Self {
        let path = env::temp_dir().join(format!(
            "vellum-real-provider-matrix-{}-{label}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let source = path.join("matrix-lsp-server.rs");
        let server = path.join("matrix-lsp-server");
        fs::write(&source, MATRIX_LSP_SERVER).unwrap();
        let status = Command::new("rustc")
            .args(["--edition=2021", "-o"])
            .arg(&server)
            .arg(&source)
            .status()
            .unwrap();
        assert!(status.success(), "matrix LSP fixture failed to compile");
        Self {
            root: fs::canonicalize(path).unwrap(),
            server: fs::canonicalize(server).unwrap(),
        }
    }
}

impl Drop for MatrixScratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

struct MatrixGenerations(u64);

impl SemanticGenerationIssuerV1 for MatrixGenerations {
    fn issue(&mut self) -> Option<SemanticIndexGenerationV1> {
        self.0 = self.0.checked_add(1)?;
        SemanticIndexGenerationV1::new(self.0)
    }
}

#[derive(Clone, Copy)]
struct MatrixReadiness(LspIndexReadinessV1);

#[derive(Clone, Copy)]
struct MatrixDocuments;

impl LspVirtualDocumentIssuerV1 for MatrixDocuments {
    fn issue(&mut self, _document: SemanticDocumentRefV1) -> Option<LspVirtualDocumentV1> {
        LspVirtualDocumentV1::try_new(
            "vellum-lsp://supervised-matrix-document".to_owned(),
            LspLimitsV1::reference(),
        )
    }
}

type MatrixAdapter = LocalOrderedLspAdapterV1<
    LocalLspFreshTransportV1,
    MatrixGenerations,
    MatrixReadiness,
    MatrixDocuments,
>;

impl LspIndexReadinessPortV1 for MatrixReadiness {
    fn poll_readiness(&mut self) -> LspIndexReadinessV1 {
        self.0
    }

    fn observe_notification(
        &mut self,
        _notification: LspReadinessNotificationV1,
    ) -> Option<LspIndexReadinessV1> {
        Some(self.0)
    }
}

fn matrix_provenance() -> SemanticProviderProvenanceV1 {
    SemanticProviderProvenanceV1::try_new(
        SemanticProviderRefV1::new([91; 16]),
        "supervised-process-fixture".into(),
        "1.0.0".into(),
    )
    .unwrap()
}

fn matrix_file_uri(path: &std::path::Path) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut uri = String::from("file://");
    for byte in path.as_os_str().as_bytes().iter().copied() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'.' | b'_' | b'~') {
            uri.push(char::from(byte));
        } else {
            uri.push('%');
            uri.push(char::from(HEX[usize::from(byte >> 4)]));
            uri.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    uri
}

fn matrix_profile(mode: &str, scratch: &MatrixScratch) -> SemanticProviderProfileV1 {
    let workspace = fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap();
    SemanticProviderProfileV1::try_new(
        matrix_provenance(),
        scratch.server.clone(),
        Vec::new(),
        BTreeMap::from([
            ("VELLUM_MATRIX_MODE".into(), mode.into()),
            ("VELLUM_MATRIX_ROOT_URI".into(), matrix_file_uri(&workspace)),
        ]),
        scratch.root.clone(),
        workspace,
        SemanticProcessDeadlinesV1 {
            launch: Duration::from_secs(3),
            graceful_shutdown: Duration::from_millis(250),
            kill_and_reap: Duration::from_secs(3),
        },
        SemanticProcessLimitsV1 {
            command_queue: 8,
            stdout_queue: 16,
            stderr_queue: 4,
            max_chunk_bytes: 4096,
            max_frame_bytes: MAX_STDIN_FRAME_BYTES_V1,
            max_stderr_bytes: 1024,
        },
    )
    .unwrap()
}

fn matrix_limits() -> (SemanticSessionLimitsV1, SemanticServiceLimitsV1) {
    let session = SemanticSessionLimitsV1::reference();
    (
        session,
        SemanticServiceLimitsV1 {
            max_clients: 1,
            max_provider_polls_per_pump: 1,
            ..SemanticServiceLimitsV1::reference()
        },
    )
}

fn launch_matrix(
    label: &str,
    mode: &str,
    readiness: LspIndexReadinessV1,
) -> (MatrixScratch, LocalOrderedSemanticPreparedV1<MatrixAdapter>) {
    let scratch = MatrixScratch::new(label);
    let (session, service) = matrix_limits();
    let prepared = launch_supervised_ordered_lsp_without_deadlines_v1(
        qualify_provider_launch_v1(matrix_profile(mode, &scratch)).unwrap(),
        MatrixGenerations(0),
        MatrixReadiness(readiness),
        MatrixDocuments,
        LspLimitsV1::reference(),
        session.semantic,
        SemanticClientRefV1::new([92; 16]),
        SemanticSessionIncarnationV1::new([93; 16]).unwrap(),
        session,
        service,
    )
    .unwrap();
    (scratch, prepared)
}

fn launch_pinned_matrix(
    label: &str,
) -> (
    MatrixScratch,
    LocalOrderedSemanticPreparedV1<MatrixAdapter>,
    [u8; 32],
) {
    let scratch = MatrixScratch::new(label);
    let (session, service) = matrix_limits();
    let qualified = qualify_provider_launch_v1(matrix_profile("pinned-serve", &scratch)).unwrap();
    let profile_digest = qualified.profile_identity().digest();
    let protocol = LspProviderProtocolProfileV1::try_new(
        profile_digest,
        "vellum-matrix-lsp".to_owned(),
        Some("1.0.0".to_owned()),
        "$/vellumStatus".to_owned(),
    )
    .unwrap();
    let prepared = launch_supervised_ordered_lsp_without_deadlines_pinned_v1(
        qualified,
        MatrixGenerations(0),
        MatrixReadiness(LspIndexReadinessV1::Current),
        MatrixDocuments,
        LspLimitsV1::reference(),
        session.semantic,
        SemanticClientRefV1::new([92; 16]),
        SemanticSessionIncarnationV1::new([93; 16]).unwrap(),
        session,
        service,
        &protocol,
    )
    .unwrap();
    (scratch, prepared, profile_digest)
}

fn context() -> SemanticBrokerContextV1 {
    SemanticBrokerContextV1 {
        subject: AuthoritySubjectRefV1::new([94; 16]),
        session: AuthoritySessionRefV1::new([95; 16]),
        allowed_roles: SemanticSessionRolesV1::top(),
        policy_epoch: 1,
    }
}

fn exchange(tag: u64) -> SemanticExchangeRefV1 {
    let mut value = [0_u8; 16];
    value[..8].copy_from_slice(&tag.to_be_bytes());
    value[8..].copy_from_slice(&(!tag).to_be_bytes());
    SemanticExchangeRefV1::new(value).unwrap()
}

fn operation(tag: u64) -> SemanticSessionOperationRefV1 {
    let mut value = [0_u8; 16];
    value[..8].copy_from_slice(&tag.to_be_bytes());
    value[8..].copy_from_slice(&(!tag).to_be_bytes());
    SemanticSessionOperationRefV1::new(value).unwrap()
}

fn receive(
    sender: &vellum_semantic_session_actor::SemanticSessionActorSenderV1,
    command: SemanticSessionCommandV1,
) -> vellum_semantic_protocol::SemanticSessionReplyV1 {
    let ticket = match sender.try_exchange(context(), command) {
        SemanticSessionActorAdmissionV1::Submitted(ticket) => ticket,
        SemanticSessionActorAdmissionV1::NotSent { reason, .. } => panic!(
            "matrix command was not admitted: {reason:?}; phase={:?}; usage={:?}",
            sender.phase(),
            sender.usage()
        ),
    };
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match ticket.try_receive() {
            Ok(SemanticExchangeDispositionV1::Reply(reply)) => return reply,
            Ok(SemanticExchangeDispositionV1::OutcomeUnknown) => {
                panic!("matrix command became uncertain")
            }
            Ok(SemanticExchangeDispositionV1::NotSent) => {
                panic!("admitted matrix command reported NotSent")
            }
            Err(SemanticSessionActorPollV1::Pending) => {
                assert!(Instant::now() < deadline, "matrix command reply deadline");
                std::thread::yield_now();
            }
            Err(SemanticSessionActorPollV1::OutcomeUnknown) => {
                panic!("matrix command reply channel disconnected")
            }
        }
    }
}

fn await_control(
    ticket: &vellum_semantic_session_actor::SemanticSessionActorControlTicketV1,
) -> SemanticSessionActorControlCompletionV1 {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match ticket.try_receive() {
            Ok(completion) => return completion,
            Err(SemanticSessionActorPollV1::Pending) => {
                assert!(Instant::now() < deadline, "matrix control reply deadline");
                std::thread::yield_now();
            }
            Err(SemanticSessionActorPollV1::OutcomeUnknown) => {
                return SemanticSessionActorControlCompletionV1::OutcomeUnknown;
            }
        }
    }
}

fn clean_stop(
    actor: LocalOrderedManagedSemanticActorV1<MatrixAdapter>,
) -> LocalOrderedCleanupOwnerV1<MatrixAdapter> {
    assert!(actor.begin_drain());
    let mut ticket = actor.try_begin_stop().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut stopped = loop {
        match ticket.try_complete() {
            Ok(LocalOrderedManagedStopResolutionV1::Terminal(stopped)) => break stopped,
            Ok(other) => panic!("matrix actor stop was not clean: {other:?}"),
            Err(pending) => ticket = *pending,
        }
        assert!(Instant::now() < deadline, "matrix actor stop deadline");
        std::thread::yield_now();
    };
    assert!(matches!(
        stopped.stop().terminal(),
        LocalOrderedActorTerminalStopV1::Clean(_)
    ));
    loop {
        match stopped.try_advance() {
            LocalOrderedStoppedRecoveryAdvanceV1::Pending(pending) => stopped = *pending,
            LocalOrderedStoppedRecoveryAdvanceV1::Ready(cleanup) => return *cleanup,
            LocalOrderedStoppedRecoveryAdvanceV1::Mismatch(_) => {
                panic!("same-spawn matrix stop mismatched recovery")
            }
        }
        assert!(Instant::now() < deadline, "matrix actor recovery deadline");
        std::thread::yield_now();
    }
}

fn observed_readiness(
    sender: &vellum_semantic_session_actor::SemanticSessionActorSenderV1,
    tag: u64,
) -> SemanticProviderReadinessProjectionV1 {
    let reply = receive(
        sender,
        SemanticSessionCommandV1::Describe {
            exchange: exchange(tag),
        },
    );
    let SemanticSessionOutcomeV1::Description(description) = reply.outcome() else {
        panic!("describe did not return the provider projection")
    };
    description.provider().readiness()
}

fn submit_collect_and_release(
    actor: &LocalOrderedManagedSemanticActorV1<MatrixAdapter>,
    sender: &vellum_semantic_session_actor::SemanticSessionActorSenderV1,
    incarnation: SemanticSessionIncarnationV1,
    membership: SemanticMembershipRefV1,
    request: SemanticRequestV1,
    next_tag: &mut u64,
) -> SemanticReplyV1 {
    let request_ref = request.request;
    let submit = receive(
        sender,
        SemanticSessionCommandV1::Submit {
            exchange: exchange(*next_tag),
            operation: operation(*next_tag + 1),
            incarnation,
            membership,
            request,
        },
    );
    *next_tag += 2;
    assert_eq!(submit.outcome(), &SemanticSessionOutcomeV1::Accepted);

    let deadline = Instant::now() + Duration::from_secs(3);
    let completed = loop {
        assert_eq!(
            await_control(&actor.try_drive_provider().unwrap()),
            SemanticSessionActorControlCompletionV1::Applied
        );
        let collected = receive(
            sender,
            SemanticSessionCommandV1::Collect {
                exchange: exchange(*next_tag),
                incarnation,
                membership,
                request: request_ref,
            },
        );
        *next_tag += 1;
        match collected.outcome() {
            SemanticSessionOutcomeV1::Complete(reply) => break reply.clone(),
            SemanticSessionOutcomeV1::Pending(_) => {}
            other => panic!("fixture semantic request did not complete: {other:?}"),
        }
        assert!(
            Instant::now() < deadline,
            "fixture semantic result deadline"
        );
        std::thread::sleep(Duration::from_millis(2));
    };
    let released = receive(
        sender,
        SemanticSessionCommandV1::ReleaseRequest {
            exchange: exchange(*next_tag),
            operation: operation(*next_tag + 1),
            incarnation,
            membership,
            request: request_ref,
        },
    );
    *next_tag += 2;
    assert_eq!(released.outcome(), &SemanticSessionOutcomeV1::Accepted);
    completed
}

#[test]
fn supervised_process_projects_unavailable_indexing_and_ready() {
    for (tag, evidence, expected) in [
        (
            1,
            LspIndexReadinessV1::Unavailable,
            SemanticProviderReadinessProjectionV1::Unavailable,
        ),
        (
            2,
            LspIndexReadinessV1::Indexing,
            SemanticProviderReadinessProjectionV1::Indexing,
        ),
        (
            3,
            LspIndexReadinessV1::Current,
            SemanticProviderReadinessProjectionV1::Ready {
                generation: SemanticIndexGenerationV1::new(1).unwrap(),
            },
        ),
    ] {
        let (_scratch, prepared) = launch_matrix(&format!("bootstrap-{tag}"), "serve", evidence);
        let (owner, sender) = prepared
            .spawn_managed(SemanticSessionActorLimitsV1::reference())
            .unwrap();
        assert!(matches!(
            observed_readiness(&sender, 10 + tag),
            SemanticProviderReadinessProjectionV1::Unavailable
                | SemanticProviderReadinessProjectionV1::Indexing
        ));
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut observation = 30_u64.saturating_add(tag.saturating_mul(20));
        loop {
            assert_eq!(
                await_control(&owner.try_drive_provider().unwrap()),
                SemanticSessionActorControlCompletionV1::Applied
            );
            if observed_readiness(&sender, observation) == expected {
                break;
            }
            observation = observation
                .checked_add(1)
                .expect("bounded matrix exchanges");
            assert!(Instant::now() < deadline, "bootstrap projection deadline");
            std::thread::sleep(Duration::from_millis(2));
        }
        let mut cleanup = clean_stop(owner);
        assert!(matches!(
            cleanup.origin().actor_stop().map(|stop| stop.terminal()),
            Some(LocalOrderedActorTerminalStopV1::Clean(_))
        ));
        let _ = cleanup.drive_client_retirement_once();
        let mut cleanup = cleanup.try_into_provider_cleanup().unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let _ = cleanup.drive_once();
            match cleanup.try_into_terminal_receipt() {
                Ok(receipt) => {
                    assert_eq!(
                        receipt.provider().terminal(),
                        LocalSemanticRetirementTerminalV1::GracefulConfirmed
                    );
                    break;
                }
                Err(returned) => cleanup = *returned,
            }
            assert!(Instant::now() < deadline, "graceful cleanup deadline");
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

#[test]
fn supervised_pinned_protocol_joins_identity_notification_and_terminal_evidence() {
    let (_scratch, prepared, profile_digest) = launch_pinned_matrix("pinned-protocol");
    let (actor, sender) = prepared
        .spawn_managed(SemanticSessionActorLimitsV1::reference())
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut observation = 80_u64;
    loop {
        assert_eq!(
            await_control(&actor.try_drive_provider().unwrap()),
            SemanticSessionActorControlCompletionV1::Applied
        );
        if matches!(
            observed_readiness(&sender, observation),
            SemanticProviderReadinessProjectionV1::Ready { .. }
        ) {
            break;
        }
        observation = observation.checked_add(1).unwrap();
        assert!(Instant::now() < deadline, "pinned readiness deadline");
        std::thread::sleep(Duration::from_millis(2));
    }

    let mut cleanup = clean_stop(actor);
    let _ = cleanup.drive_client_retirement_once();
    let mut cleanup = cleanup.try_into_provider_cleanup().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let receipt = loop {
        let _ = cleanup.drive_once();
        match cleanup.try_into_terminal_receipt() {
            Ok(receipt) => break receipt,
            Err(returned) => cleanup = *returned,
        }
        assert!(Instant::now() < deadline, "pinned cleanup deadline");
        std::thread::sleep(Duration::from_millis(2));
    };
    let evidence = receipt.provider().provider_qualification();
    assert!(evidence.profile_matches(&profile_digest));
    assert!(evidence.workspace_bound());
    assert_eq!(
        evidence.server_identity(),
        LspServerIdentityDispositionV1::PinnedExact
    );
    assert_eq!(
        evidence.readiness_source(),
        LspReadinessEvidenceSourceV1::NotificationBound
    );
    assert_eq!(
        evidence.last_notification_readiness(),
        Some(LspIndexReadinessV1::Current)
    );
    assert_eq!(evidence.notifications_observed(), 1);
    assert_eq!(evidence.capabilities(), (true, true));
    let debug = format!("{receipt:?}");
    assert!(!debug.contains("vellum-matrix-lsp"));
    assert!(!debug.contains("$/vellumStatus"));
    drop(sender);
}

#[test]
fn supervised_pinned_profile_mismatch_refuses_before_launch_and_retains_authority() {
    let scratch = MatrixScratch::new("pinned-profile-mismatch");
    let qualified = qualify_provider_launch_v1(matrix_profile("serve", &scratch)).unwrap();
    let actual_digest = qualified.profile_identity().digest();
    let mut wrong_digest = actual_digest;
    wrong_digest[0] ^= 1;
    let protocol = LspProviderProtocolProfileV1::try_new(
        wrong_digest,
        "vellum-matrix-lsp".to_owned(),
        Some("1.0.0".to_owned()),
        "$/vellumStatus".to_owned(),
    )
    .unwrap();
    let (session, service) = matrix_limits();
    let failure = match launch_supervised_ordered_lsp_without_deadlines_pinned_v1(
        qualified,
        MatrixGenerations(0),
        MatrixReadiness(LspIndexReadinessV1::Current),
        MatrixDocuments,
        LspLimitsV1::reference(),
        session.semantic,
        SemanticClientRefV1::new([92; 16]),
        SemanticSessionIncarnationV1::new([93; 16]).unwrap(),
        session,
        service,
        &protocol,
    ) {
        Ok(_) => panic!("mismatched pinned profile unexpectedly launched"),
        Err(failure) => failure,
    };
    assert_eq!(
        failure.reason(),
        LocalSupervisedOrderedLspBuildRefusalV1::InvalidInitializationProfile
    );
    let LocalSupervisedOrderedLspBuildResidueV1::PreLaunch { qualified, .. } =
        failure.into_residue()
    else {
        panic!("profile mismatch crossed the process-effect boundary")
    };
    assert_eq!(qualified.profile_identity().digest(), actual_digest);
}

#[test]
fn supervised_process_composes_ordered_membership_and_synchronization() {
    let (_scratch, prepared) =
        launch_matrix("ordered-products", "serve", LspIndexReadinessV1::Current);
    let (owner, sender) = prepared
        .spawn_managed(SemanticSessionActorLimitsV1::reference())
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut observation = 100_u64;
    loop {
        assert_eq!(
            await_control(&owner.try_drive_provider().unwrap()),
            SemanticSessionActorControlCompletionV1::Applied
        );
        if matches!(
            observed_readiness(&sender, observation),
            SemanticProviderReadinessProjectionV1::Ready { .. }
        ) {
            break;
        }
        observation = observation
            .checked_add(1)
            .expect("bounded matrix exchanges");
        assert!(Instant::now() < deadline, "provider ready deadline");
        std::thread::sleep(Duration::from_millis(2));
    }
    let incarnation = SemanticSessionIncarnationV1::new([93; 16]).unwrap();
    let joined = receive(
        &sender,
        SemanticSessionCommandV1::JoinSession {
            exchange: exchange(201),
            operation: operation(202),
            incarnation,
            requested_roles: SemanticSessionRolesV1::top(),
        },
    );
    let SemanticSessionOutcomeV1::Joined { membership, .. } = joined.outcome() else {
        panic!("join was not accepted")
    };
    let membership = *membership;
    let epoch = SemanticSynchronizationEpochV1::new([203; 16]).unwrap();
    let manifest =
        SemanticSynchronizationManifestV1::try_from_snapshots(&[], matrix_limits().0).unwrap();
    assert_eq!(
        receive(
            &sender,
            SemanticSessionCommandV1::BeginSynchronization {
                exchange: exchange(204),
                operation: operation(205),
                incarnation,
                membership,
                epoch,
                manifest,
            },
        )
        .outcome(),
        &SemanticSessionOutcomeV1::Accepted
    );
    assert_eq!(
        receive(
            &sender,
            SemanticSessionCommandV1::CompleteSynchronization {
                exchange: exchange(206),
                operation: operation(207),
                incarnation,
                membership,
                epoch,
                manifest,
            },
        )
        .outcome(),
        &SemanticSessionOutcomeV1::Accepted
    );
    let _ = receive(
        &sender,
        SemanticSessionCommandV1::LeaveSession {
            exchange: exchange(208),
            operation: operation(209),
            incarnation,
            membership,
        },
    );
    assert_eq!(
        await_control(&owner.try_drive_retirement().unwrap()),
        SemanticSessionActorControlCompletionV1::Applied
    );
    let mut cleanup = clean_stop(owner);
    let _ = cleanup.drive_client_retirement_once();
    let mut cleanup = cleanup.try_into_provider_cleanup().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let _ = cleanup.drive_once();
        match cleanup.try_into_terminal_receipt() {
            Ok(_) => break,
            Err(returned) => cleanup = *returned,
        }
        assert!(Instant::now() < deadline, "ordered cleanup deadline");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn supervised_fixture_closes_diagnostics_nested_symbols_and_versioned_rename() {
    let (_scratch, prepared) =
        launch_matrix("semantic-operations", "serve", LspIndexReadinessV1::Current);
    let (actor, sender) = prepared
        .spawn_managed(SemanticSessionActorLimitsV1::reference())
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut observation = 300_u64;
    loop {
        assert_eq!(
            await_control(&actor.try_drive_provider().unwrap()),
            SemanticSessionActorControlCompletionV1::Applied
        );
        if matches!(
            observed_readiness(&sender, observation),
            SemanticProviderReadinessProjectionV1::Ready {
                generation
            } if generation == SemanticIndexGenerationV1::new(1).unwrap()
        ) {
            break;
        }
        observation += 1;
        assert!(Instant::now() < deadline, "semantic fixture ready deadline");
        std::thread::sleep(Duration::from_millis(2));
    }

    let incarnation = SemanticSessionIncarnationV1::new([93; 16]).unwrap();
    let joined = receive(
        &sender,
        SemanticSessionCommandV1::JoinSession {
            exchange: exchange(401),
            operation: operation(402),
            incarnation,
            requested_roles: SemanticSessionRolesV1::top(),
        },
    );
    let SemanticSessionOutcomeV1::Joined { membership, .. } = joined.outcome() else {
        panic!("semantic fixture join was not accepted")
    };
    let membership = *membership;
    let target = SemanticTargetV1 {
        document: SemanticDocumentRefV1::new([96; 16]),
        source_revision: SemanticSourceRevisionV1::new(1),
    };
    let session_limits = matrix_limits().0;
    let snapshot = vellum_semantic_protocol::SemanticSessionSnapshotV1::try_new(
        target,
        "rust".into(),
        "fn alpha() {}\n".into(),
        session_limits,
    )
    .unwrap();
    let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
        std::slice::from_ref(&snapshot),
        session_limits,
    )
    .unwrap();
    let epoch = SemanticSynchronizationEpochV1::new([97; 16]).unwrap();
    assert_eq!(
        receive(
            &sender,
            SemanticSessionCommandV1::BeginSynchronization {
                exchange: exchange(403),
                operation: operation(404),
                incarnation,
                membership,
                epoch,
                manifest,
            },
        )
        .outcome(),
        &SemanticSessionOutcomeV1::Accepted
    );
    assert_eq!(
        receive(
            &sender,
            SemanticSessionCommandV1::SyncOpen {
                exchange: exchange(405),
                operation: operation(406),
                incarnation,
                membership,
                epoch,
                snapshot: snapshot.clone(),
            },
        )
        .outcome(),
        &SemanticSessionOutcomeV1::Accepted
    );
    assert_eq!(
        receive(
            &sender,
            SemanticSessionCommandV1::CompleteSynchronization {
                exchange: exchange(407),
                operation: operation(408),
                incarnation,
                membership,
                epoch,
                manifest,
            },
        )
        .outcome(),
        &SemanticSessionOutcomeV1::Accepted
    );

    let request = |tag, query| SemanticRequestV1 {
        request: SemanticRequestRefV1::new([tag; 16]),
        target,
        expected_index: SemanticIndexGenerationV1::new(1),
        language: SemanticTextV1::try_new("rust".to_owned(), 16).unwrap(),
        query,
    };
    let mut next_tag = 500_u64;
    let diagnostics = submit_collect_and_release(
        &actor,
        &sender,
        incarnation,
        membership,
        request(41, SemanticQueryV1::Diagnostics),
        &mut next_tag,
    );
    assert_eq!(diagnostics.request, SemanticRequestRefV1::new([41; 16]));
    assert_eq!(diagnostics.target, target);
    assert_eq!(
        diagnostics.index_generation,
        SemanticIndexGenerationV1::new(1).unwrap()
    );
    assert_eq!(diagnostics.provider, matrix_provenance());
    let values = diagnostics.result.diagnostics().unwrap();
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].severity, SemanticDiagnosticSeverityV1::Warning);
    assert_eq!(values[0].location.target, target);
    assert_eq!(
        values[0].location.span,
        SemanticSpanV1::try_new(3, 8).unwrap()
    );
    assert_eq!(values[0].message.as_str(), "fixture warning");
    assert_eq!(values[0].code.as_ref().unwrap().as_str(), "W1");
    let diagnostic_debug = format!("{diagnostics:?}");
    assert!(!diagnostic_debug.contains("fixture warning"));
    assert!(!diagnostic_debug.contains("W1"));

    let symbols = submit_collect_and_release(
        &actor,
        &sender,
        incarnation,
        membership,
        request(
            42,
            SemanticQueryV1::Symbols {
                needle: SemanticTextV1::try_new("alpha".to_owned(), 16).unwrap(),
            },
        ),
        &mut next_tag,
    );
    assert_eq!(symbols.request, SemanticRequestRefV1::new([42; 16]));
    assert_eq!(symbols.target, target);
    let values = symbols.result.symbols().unwrap();
    assert_eq!(values.len(), 2);
    assert_eq!(values[0].name.as_str(), "alpha");
    assert_eq!(values[0].kind, SemanticSymbolKindV1::Function);
    assert_eq!(values[1].name.as_str(), "alpha_inner");
    assert_eq!(values[1].kind, SemanticSymbolKindV1::Value);
    assert!(values.iter().all(|value| {
        value.location.target == target
            && value.location.span == SemanticSpanV1::try_new(3, 8).unwrap()
    }));
    assert!(!format!("{symbols:?}").contains("alpha"));

    let rename = submit_collect_and_release(
        &actor,
        &sender,
        incarnation,
        membership,
        request(
            43,
            SemanticQueryV1::PlanRename {
                offset: 3,
                new_name: SemanticTextV1::try_new("beta".to_owned(), 16).unwrap(),
            },
        ),
        &mut next_tag,
    );
    assert_eq!(rename.request, SemanticRequestRefV1::new([43; 16]));
    assert_eq!(rename.target, target);
    let edits = rename.result.rename_plan().unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].location.target, target);
    assert_eq!(
        edits[0].location.span,
        SemanticSpanV1::try_new(3, 8).unwrap()
    );
    assert_eq!(edits[0].replacement.as_str(), "beta");
    assert!(!format!("{rename:?}").contains("beta"));

    let _ = receive(
        &sender,
        SemanticSessionCommandV1::LeaveSession {
            exchange: exchange(600),
            operation: operation(601),
            incarnation,
            membership,
        },
    );
    assert_eq!(
        await_control(&actor.try_drive_retirement().unwrap()),
        SemanticSessionActorControlCompletionV1::Applied
    );
    let mut cleanup = clean_stop(actor);
    let _ = cleanup.drive_client_retirement_once();
    let mut cleanup = cleanup.try_into_provider_cleanup().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let receipt = loop {
        let _ = cleanup.drive_once();
        match cleanup.try_into_terminal_receipt() {
            Ok(receipt) => break receipt,
            Err(returned) => cleanup = *returned,
        }
        assert!(
            Instant::now() < deadline,
            "semantic fixture cleanup deadline"
        );
        std::thread::sleep(Duration::from_millis(2));
    };
    assert_eq!(
        receipt.provider().terminal(),
        LocalSemanticRetirementTerminalV1::GracefulConfirmed
    );
    drop(sender);
}

#[test]
fn supervised_process_poll_failure_retains_the_lawful_ready_pretransition() {
    let (_scratch, prepared) = launch_matrix(
        "poll-failure-pretransition",
        "close-after-initialize",
        LspIndexReadinessV1::Current,
    );
    let (owner, sender) = prepared
        .spawn_managed(SemanticSessionActorLimitsV1::reference())
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut observation = 150_u64;
    loop {
        assert_eq!(
            await_control(&owner.try_drive_provider().unwrap()),
            SemanticSessionActorControlCompletionV1::Applied
        );
        if matches!(
            observed_readiness(&sender, observation),
            SemanticProviderReadinessProjectionV1::Ready { .. }
        ) {
            break;
        }
        observation = observation
            .checked_add(1)
            .expect("bounded matrix exchanges");
        assert!(Instant::now() < deadline, "ready pretransition deadline");
        std::thread::sleep(Duration::from_millis(2));
    }

    std::thread::sleep(Duration::from_millis(900));
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if await_control(&owner.try_drive_provider().unwrap())
            == SemanticSessionActorControlCompletionV1::OutcomeUnknown
        {
            break;
        }
        assert!(Instant::now() < deadline, "poll failure deadline");
        std::thread::sleep(Duration::from_millis(2));
    }
    let mut stopped = owner
        .try_recover_failed()
        .expect("failed supervised actor must retain managed recovery authority");
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut cleanup = loop {
        match stopped.try_advance() {
            LocalOrderedStoppedRecoveryAdvanceV1::Pending(pending) => stopped = *pending,
            LocalOrderedStoppedRecoveryAdvanceV1::Ready(cleanup) => break *cleanup,
            LocalOrderedStoppedRecoveryAdvanceV1::Mismatch(_) => {
                panic!("failed supervised actor mismatched failed recovery")
            }
        }
        assert!(Instant::now() < deadline, "failed actor recovery deadline");
        std::thread::yield_now();
    };
    drop(sender);
    assert!(matches!(
        cleanup.origin().origin(),
        LocalOrderedCleanupOriginV1::ActorRecovered(SemanticSessionActorTerminationV1::Failed)
    ));
    let _ = cleanup.drive_client_retirement_once();
    let mut cleanup = cleanup.try_into_provider_cleanup().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let receipt = loop {
        let _ = cleanup.drive_once();
        match cleanup.try_into_terminal_receipt() {
            Ok(receipt) => break receipt,
            Err(returned) => cleanup = *returned,
        }
        assert!(
            Instant::now() < deadline,
            "failed-provider cleanup deadline"
        );
        std::thread::sleep(Duration::from_millis(2));
    };
    assert_eq!(
        receipt.provider().terminal(),
        LocalSemanticRetirementTerminalV1::CleanupFailed
    );
}
