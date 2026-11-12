#![forbid(unsafe_code)]

use std::{ffi::OsString, process::ExitCode};

use serde_json::json;
use tokio::io::BufReader;
use vellum_sdk_semantic_client::{
    client as vellum_semantic_client, protocol as vellum_semantic_protocol,
    service as vellum_semantic_service, session as vellum_semantic_session_client,
    transport as vellum_semantic_transport,
};
use vellum_semantic_client::{
    SemanticCapabilitySetV1, SemanticClientRefV1, SemanticProviderProvenanceV1,
    SemanticProviderRefV1, SemanticProviderStateV1, SemanticRequestRefV1, SemanticRequestV1,
    bind_semantic_client_v1, try_bind_semantic_session_query_v1,
};
use vellum_semantic_mcp::{
    SemanticMcpDocumentV1, SemanticMcpLimitsV1, SemanticMcpServerSuspensionV1, SemanticMcpServerV1,
    SemanticMcpStdioFailureV1, SemanticMcpStdioFrameV1, SemanticMcpStdioLimitsV1,
    SemanticMcpStdioStepV1, SemanticMcpStdioUsageV1, read_stdio_frame_v1,
    serve_stdio_read_frame_v1, serve_stdio_v1,
};
use vellum_semantic_service::{
    SemanticClientRetirementStateV1, SemanticProviderAdmissionV1, SemanticProviderCompletionV1,
    SemanticProviderPortV1, SemanticProviderSubmissionV1, SemanticServiceLimitsV1,
    SemanticServiceV1, SemanticSnapshotUpdateV1,
};

const HELP: &str = "vellum-semantic-mcp 0.1.0\n\
Bounded stdio MCP peer for Vellum semantic queries.\n\n\
Real socket mode is a bounded uniform-language rust-analyzer composition.\n\n\
USAGE:\n\
    vellum-semantic-mcp --semantic-socket PRIVATE_SOCKET --semantic-credential PRIVATE_CREDENTIAL --native-socket PRIVATE_SOCKET --native-credential PRIVATE_CREDENTIAL --document-language LANGUAGE [--metrics-stderr]\n\
    vellum-semantic-mcp --unavailable [--metrics-stderr]\n\
    vellum-semantic-mcp --probe\n\n\
OPTIONS:\n\
    --semantic-socket PRIVATE_SOCKET       Persistent semantic service endpoint.\n\
    --semantic-credential PRIVATE_FILE     Private semantic-service credential.\n\
    --native-socket PRIVATE_SOCKET         Native Vellum daemon endpoint.\n\
    --native-credential PRIVATE_FILE       Private native-daemon credential.\n\
    --document-language LANGUAGE           Explicit language registration applied to every\n\
                                           synchronized document in this composition.\n\
    --unavailable      Run an explicit no-provider composition for MCP discovery and\n\
                       harness qualification. Semantic queries fail closed.\n\
    --metrics-stderr   Emit one content-safe JSON usage record after clean EOF.\n\
    --probe            Print the executable protocol identity and exit.\n\
    -h, --help         Print this help.\n\n\
Credential material is loaded only from private files and is never accepted\n\
through argv or environment. --unavailable never claims provider readiness or\n\
workspace membership.\n";

fn main() -> ExitCode {
    let arguments: Vec<OsString> = std::env::args_os().skip(1).collect();
    let exact: Option<Vec<&str>> = arguments.iter().map(|value| value.to_str()).collect();
    match exact.as_deref() {
        Some(["--probe"]) => {
            println!("vellum-semantic-mcp/0.1 stdio/2026-07-28");
            ExitCode::SUCCESS
        }
        Some(["-h"]) | Some(["--help"]) => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        Some(["--unavailable"]) => run_unavailable(false),
        Some(["--unavailable", "--metrics-stderr"])
        | Some(["--metrics-stderr", "--unavailable"]) => run_unavailable(true),
        _ => run_remote_arguments(&arguments),
    }
}

#[cfg(unix)]
#[derive(Debug, Eq, PartialEq)]
struct RemoteArguments {
    semantic_socket: std::path::PathBuf,
    semantic_credential: std::path::PathBuf,
    native_socket: std::path::PathBuf,
    native_credential: std::path::PathBuf,
    language: String,
    metrics: bool,
}

#[cfg(unix)]
fn parse_remote_arguments(arguments: &[OsString]) -> Result<RemoteArguments, &'static str> {
    let mut semantic_socket = None;
    let mut semantic_credential = None;
    let mut native_socket = None;
    let mut native_credential = None;
    let mut language = None;
    let mut metrics = false;
    let mut index = 0;
    while index < arguments.len() {
        let option = arguments[index]
            .to_str()
            .ok_or("option name is not UTF-8")?;
        if option == "--metrics-stderr" {
            if metrics {
                return Err("duplicate metrics option");
            }
            metrics = true;
            index += 1;
            continue;
        }
        let value = arguments.get(index + 1).ok_or("missing option value")?;
        if value.as_encoded_bytes().starts_with(b"--") {
            return Err("missing option value");
        }
        let slot = match option {
            "--semantic-socket" => &mut semantic_socket,
            "--semantic-credential" => &mut semantic_credential,
            "--native-socket" => &mut native_socket,
            "--native-credential" => &mut native_credential,
            "--document-language" => {
                if language.is_some() {
                    return Err("duplicate language option");
                }
                let value = value.to_str().ok_or("language is not UTF-8")?;
                if value.is_empty()
                    || value.len() > vellum_semantic_client::MAX_SEMANTIC_LABEL_BYTES_V1
                {
                    return Err("language exceeds semantic limit");
                }
                language = Some(value.to_owned());
                index += 2;
                continue;
            }
            _ => return Err("unknown option"),
        };
        if slot.replace(std::path::PathBuf::from(value)).is_some() {
            return Err("duplicate path option");
        }
        index += 2;
    }
    Ok(RemoteArguments {
        semantic_socket: semantic_socket.ok_or("missing semantic socket")?,
        semantic_credential: semantic_credential.ok_or("missing semantic credential")?,
        native_socket: native_socket.ok_or("missing native socket")?,
        native_credential: native_credential.ok_or("missing native credential")?,
        language: language.ok_or("missing language")?,
        metrics,
    })
}

#[cfg(unix)]
fn run_remote_arguments(arguments: &[OsString]) -> ExitCode {
    let configuration = match parse_remote_arguments(arguments) {
        Ok(configuration) => configuration,
        Err(reason) => {
            eprintln!("vellum-semantic-mcp: {reason}; use --help");
            return ExitCode::from(2);
        }
    };
    match run_remote(configuration) {
        Ok(()) => ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("vellum-semantic-mcp: remote composition refused at {reason}");
            ExitCode::from(2)
        }
    }
}

#[cfg(not(unix))]
fn run_remote_arguments(_arguments: &[OsString]) -> ExitCode {
    eprintln!("vellum-semantic-mcp: the semantic local transport requires Unix");
    ExitCode::from(2)
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RemoteStage {
    Credential,
    SemanticTransport,
    NativeTransport,
    SessionDescribe,
    SessionJoin,
    SessionReconnect,
    Synchronization,
    Catalog,
    NativeDisconnect,
    QueryBinding,
    Runtime,
    Mcp,
    Stdio,
    Cleanup,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct RemoteSynchronizationUsage {
    feed_pages: u64,
    feed_events: u64,
    feed_gaps: u64,
    refreshes: u64,
    rollovers: u64,
    reconnects: u64,
    quiescent_observations: u64,
    saturated: bool,
}

#[cfg(unix)]
impl RemoteSynchronizationUsage {
    fn bump(counter: &mut u64, saturated: &mut bool) {
        let next = counter.saturating_add(1);
        *saturated |= next == u64::MAX;
        *counter = next;
    }

    fn add(counter: &mut u64, amount: usize, saturated: &mut bool) {
        let amount = u64::try_from(amount).unwrap_or(u64::MAX);
        let next = counter.saturating_add(amount);
        *saturated |= next == u64::MAX;
        *counter = next;
    }
}

#[cfg(unix)]
impl std::fmt::Display for RemoteStage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Credential => "credential",
            Self::SemanticTransport => "semantic_transport",
            Self::NativeTransport => "native_transport",
            Self::SessionDescribe => "session_describe",
            Self::SessionJoin => "session_join",
            Self::SessionReconnect => "session_reconnect",
            Self::Synchronization => "synchronization",
            Self::Catalog => "catalog",
            Self::NativeDisconnect => "native_disconnect",
            Self::QueryBinding => "query_binding",
            Self::Runtime => "runtime",
            Self::Mcp => "mcp",
            Self::Stdio => "stdio",
            Self::Cleanup => "cleanup",
        })
    }
}

#[cfg(unix)]
struct UniformSynchronizationComposition {
    language: String,
    next: u128,
}

#[cfg(unix)]
impl UniformSynchronizationComposition {
    const QUERY_PARTITION: u128 = 1_u128 << 127;

    fn try_new(language: String) -> Option<Self> {
        let mut seed = [0_u8; 16];
        getrandom::fill(&mut seed).ok()?;
        // Start in the lower quarter. Composition-owned synchronization and
        // lifecycle identities remain below bit 127; query identities use the
        // corresponding value with bit 127 set. Each partition therefore has
        // at least one quarter of the u128 space of monotone headroom.
        seed[0] &= 0x3f;
        let mut next = u128::from_be_bytes(seed);
        if next == 0 {
            next = 1;
        }
        Some(Self { language, next })
    }

    fn next_bytes(&mut self) -> Option<[u8; 16]> {
        Some(self.next_value()?.to_be_bytes())
    }

    fn next_value(&mut self) -> Option<u128> {
        if self.next >= Self::QUERY_PARTITION {
            return None;
        }
        let value = self.next;
        self.next = self.next.checked_add(1)?;
        Some(value)
    }

    fn query_seed(&self) -> Option<u128> {
        let seed = self.next.checked_add(Self::QUERY_PARTITION)?;
        (seed <= u128::MAX - 3).then_some(seed)
    }
}

#[cfg(unix)]
impl vellum_semantic_session_client::VellumSnapshotSynchronizationCompositionV1
    for UniformSynchronizationComposition
{
    fn language(
        &mut self,
        _document: vellum_sdk_client::protocol::WorkspaceDocumentIdV1,
    ) -> Option<String> {
        Some(self.language.clone())
    }

    fn next_epoch(&mut self) -> Option<vellum_semantic_protocol::SemanticSynchronizationEpochV1> {
        vellum_semantic_protocol::SemanticSynchronizationEpochV1::new(self.next_bytes()?)
    }

    fn next_identity(
        &mut self,
    ) -> Option<vellum_semantic_session_client::SemanticSynchronizationCommandIdentityV1> {
        Some(
            vellum_semantic_session_client::SemanticSynchronizationCommandIdentityV1 {
                exchange: vellum_semantic_protocol::SemanticExchangeRefV1::new(self.next_bytes()?)?,
                operation: vellum_semantic_protocol::SemanticSessionOperationRefV1::new(
                    self.next_bytes()?,
                )?,
            },
        )
    }
}

#[cfg(unix)]
fn semantic_authentication(
    record: vellum_adapter_local_authentication::LocalCredentialRecordV1,
) -> impl FnMut(
    &mut std::os::unix::net::UnixStream,
) -> Result<(), vellum_semantic_transport::UnixSemanticAuthenticationRefusalV1>
+ Send {
    use std::io::{Read as _, Write as _};
    use vellum_sdk_client::transport::LocalAuthenticationClaimProvider as _;
    use vellum_semantic_transport::UnixSemanticAuthenticationRefusalV1 as Refusal;

    let mut provider =
        vellum_adapter_local_authentication::LocalCredentialClaimProviderV1::new(record);
    move |stream| {
        let mut hello = [0_u8; 33];
        stream.read_exact(&mut hello).map_err(|_| {
            eprintln!("vellum-semantic-mcp: semantic_authentication_refused phase=hello_read");
            Refusal::Io
        })?;
        if hello[0] != 1 {
            eprintln!("vellum-semantic-mcp: semantic_authentication_refused phase=hello_version");
            return Err(Refusal::Refused);
        }
        let mut challenge = [0_u8; 16];
        challenge.copy_from_slice(&hello[1..17]);
        let mut session = [0_u8; 16];
        session.copy_from_slice(&hello[17..33]);
        let claim = provider.claim(challenge, session).map_err(|_| {
            eprintln!("vellum-semantic-mcp: semantic_authentication_refused phase=claim");
            Refusal::Refused
        })?;
        let mut response = [0_u8; 49];
        response[0] = 1;
        response[1..33].copy_from_slice(&hello[1..33]);
        response[33..49].copy_from_slice(&claim.to_bytes());
        stream
            .write_all(&response)
            .and_then(|()| stream.flush())
            .map_err(|_| {
                eprintln!(
                    "vellum-semantic-mcp: semantic_authentication_refused phase=response_write"
                );
                Refusal::Io
            })?;
        let mut result = [0_u8; 2];
        stream.read_exact(&mut result).map_err(|_| {
            eprintln!("vellum-semantic-mcp: semantic_authentication_refused phase=result_read");
            Refusal::Io
        })?;
        if result == [1, 1] {
            Ok(())
        } else {
            eprintln!("vellum-semantic-mcp: semantic_authentication_refused phase=result_refused");
            Err(Refusal::Refused)
        }
    }
}

#[cfg(unix)]
fn describe_session<P: vellum_semantic_session_client::SemanticSessionExchangePortV1>(
    session: &mut vellum_semantic_session_client::SemanticSessionClientV1<P>,
    tag: u128,
) -> Result<vellum_semantic_protocol::SemanticSessionDescriptionV1, RemoteStage> {
    use vellum_semantic_protocol::{
        SemanticExchangeDispositionV1, SemanticExchangeRefV1, SemanticSessionCommandV1,
        SemanticSessionOutcomeV1,
    };
    let exchange =
        SemanticExchangeRefV1::new(tag.to_be_bytes()).ok_or(RemoteStage::SessionDescribe)?;
    let disposition = session
        .attempt(SemanticSessionCommandV1::Describe { exchange })
        .map_err(|_| RemoteStage::SessionDescribe)?;
    match disposition {
        SemanticExchangeDispositionV1::Reply(reply) => match reply.outcome() {
            SemanticSessionOutcomeV1::Description(description) => Ok(*description),
            _ => Err(RemoteStage::SessionDescribe),
        },
        SemanticExchangeDispositionV1::NotSent | SemanticExchangeDispositionV1::OutcomeUnknown => {
            Err(RemoteStage::SessionDescribe)
        }
    }
}

#[cfg(unix)]
fn join_session<P: vellum_semantic_session_client::SemanticSessionExchangePortV1>(
    session: &mut vellum_semantic_session_client::SemanticSessionClientV1<P>,
    description: vellum_semantic_protocol::SemanticSessionDescriptionV1,
    exchange: u128,
    operation: u128,
) -> Result<(), RemoteStage> {
    use vellum_semantic_protocol::{
        SemanticExchangeDispositionV1, SemanticExchangeRefV1, SemanticSessionCommandV1,
        SemanticSessionOperationRefV1, SemanticSessionOutcomeV1, SemanticSessionRolesV1,
    };
    let disposition = session
        .attempt(SemanticSessionCommandV1::JoinSession {
            exchange: SemanticExchangeRefV1::new(exchange.to_be_bytes())
                .ok_or(RemoteStage::SessionJoin)?,
            operation: SemanticSessionOperationRefV1::new(operation.to_be_bytes())
                .ok_or(RemoteStage::SessionJoin)?,
            incarnation: description.incarnation(),
            requested_roles: SemanticSessionRolesV1::top(),
        })
        .map_err(|_| RemoteStage::SessionJoin)?;
    match disposition {
        SemanticExchangeDispositionV1::Reply(reply) => match reply.outcome() {
            SemanticSessionOutcomeV1::Joined { granted_roles, .. }
                if granted_roles.contains(SemanticSessionRolesV1::query())
                    && granted_roles.contains(SemanticSessionRolesV1::synchronize()) =>
            {
                Ok(())
            }
            SemanticSessionOutcomeV1::ReplayCapacity => Err(RemoteStage::SessionReconnect),
            _ => Err(RemoteStage::SessionJoin),
        },
        SemanticExchangeDispositionV1::NotSent | SemanticExchangeDispositionV1::OutcomeUnknown => {
            Err(RemoteStage::SessionJoin)
        }
    }
}

#[cfg(unix)]
fn retirement_progress_is_bounded(
    prior: Option<(u32, u32)>,
    current: (u32, u32),
    budget: u32,
) -> bool {
    current
        .0
        .checked_add(current.1)
        .is_some_and(|total| total <= budget)
        && prior
            .is_none_or(|prior| current.0 <= prior.0 && current.1 <= prior.1 && current != prior)
}

#[cfg(unix)]
fn leave_session_issuing<P: vellum_semantic_session_client::SemanticSessionExchangePortV1>(
    session: &mut vellum_semantic_session_client::SemanticSessionClientV1<P>,
    mut issue: impl FnMut() -> Option<(u128, u128)>,
) -> Result<(), RemoteStage> {
    use vellum_semantic_protocol::{
        SemanticExchangeDispositionV1, SemanticExchangeRefV1, SemanticSessionCommandV1,
        SemanticSessionOperationRefV1, SemanticSessionOutcomeV1, SemanticSessionPendingV1,
    };
    let retirement_budget = session
        .description()
        .and_then(|description| description.limits().retirement_operation_reserve())
        .ok_or(RemoteStage::Cleanup)?;
    let mut prior_obligations = None;
    for _ in 0..retirement_budget {
        let binding = session.binding().ok_or(RemoteStage::Cleanup)?;
        let (exchange, operation) = issue().ok_or(RemoteStage::Cleanup)?;
        let command = SemanticSessionCommandV1::LeaveSession {
            exchange: SemanticExchangeRefV1::new(exchange.to_be_bytes())
                .ok_or(RemoteStage::Cleanup)?,
            operation: SemanticSessionOperationRefV1::new(operation.to_be_bytes())
                .ok_or(RemoteStage::Cleanup)?,
            incarnation: binding.incarnation,
            membership: binding.membership,
        };
        match session.attempt(command).map_err(|_| RemoteStage::Cleanup)? {
            SemanticExchangeDispositionV1::Reply(reply)
                if matches!(reply.outcome(), SemanticSessionOutcomeV1::Accepted) =>
            {
                return Ok(());
            }
            SemanticExchangeDispositionV1::Reply(reply) => {
                let SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Retirement {
                    remaining_documents,
                    quarantined_requests,
                }) = reply.outcome()
                else {
                    return Err(RemoteStage::Cleanup);
                };
                let obligations = (*remaining_documents, *quarantined_requests);
                if !retirement_progress_is_bounded(
                    prior_obligations,
                    obligations,
                    retirement_budget,
                ) {
                    return Err(RemoteStage::Cleanup);
                }
                prior_obligations = Some(obligations);
            }
            SemanticExchangeDispositionV1::NotSent
            | SemanticExchangeDispositionV1::OutcomeUnknown => {
                return Err(RemoteStage::Cleanup);
            }
        }
    }
    Err(RemoteStage::Cleanup)
}

#[cfg(unix)]
fn leave_session<P: vellum_semantic_session_client::SemanticSessionExchangePortV1>(
    session: &mut vellum_semantic_session_client::SemanticSessionClientV1<P>,
    mut exchange: u128,
    mut operation: u128,
) -> Result<(), RemoteStage> {
    leave_session_issuing(session, || {
        let identity = (exchange, operation);
        exchange = exchange.checked_add(1)?;
        operation = operation.checked_add(1)?;
        Some(identity)
    })
}

#[cfg(unix)]
fn leave_session_with_composition<
    P: vellum_semantic_session_client::SemanticSessionExchangePortV1,
>(
    session: &mut vellum_semantic_session_client::SemanticSessionClientV1<P>,
    composition: &mut UniformSynchronizationComposition,
) -> Result<(), RemoteStage> {
    leave_session_issuing(session, || {
        Some((composition.next_value()?, composition.next_value()?))
    })
}

#[cfg(unix)]
fn wait_for_ready<P: vellum_semantic_session_client::SemanticSessionExchangePortV1>(
    session: &mut vellum_semantic_session_client::SemanticSessionClientV1<P>,
    composition: &mut UniformSynchronizationComposition,
) -> Result<(), RemoteStage> {
    use vellum_semantic_protocol::{
        SemanticProviderReadinessProjectionV1, SemanticSessionReadinessV1,
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    for _ in 0..1_024 {
        let tag = composition
            .next_value()
            .ok_or(RemoteStage::Synchronization)?;
        let description = describe_session(session, tag)?;
        if matches!(
            description.readiness(),
            SemanticSessionReadinessV1::Ready { .. }
        ) && matches!(
            description.provider().readiness(),
            SemanticProviderReadinessProjectionV1::Ready { .. }
        ) {
            return Ok(());
        }
        if matches!(
            description.provider().readiness(),
            SemanticProviderReadinessProjectionV1::Unavailable
        ) || std::time::Instant::now() >= deadline
        {
            eprintln!(
                "vellum-semantic-mcp: readiness_refused session={:?} provider={:?}",
                description.readiness(),
                description.provider().readiness()
            );
            return Err(RemoteStage::Synchronization);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    Err(RemoteStage::Synchronization)
}

#[cfg(unix)]
fn semantic_documents(
    coordinator: &vellum_semantic_session_client::VellumSnapshotSynchronizationCoordinatorV1,
    limits: SemanticMcpLimitsV1,
) -> Result<Vec<SemanticMcpDocumentV1>, RemoteStage> {
    coordinator
        .synchronized_documents()
        .into_iter()
        .map(|document| {
            let workspace_document = document.workspace_document();
            let target = document.target();
            SemanticMcpDocumentV1::try_new(
                workspace_document.workspace().get(),
                workspace_document.get(),
                target.document,
                target.source_revision,
                document.language().to_owned(),
                limits.semantic,
            )
            .ok_or(RemoteStage::Catalog)
        })
        .collect()
}

#[cfg(unix)]
fn synchronize_ready<T, P>(
    native: &mut vellum_sdk_client::client::Client<T>,
    semantic: &mut vellum_semantic_session_client::SemanticSessionClientV1<P>,
    coordinator: &mut vellum_semantic_session_client::VellumSnapshotSynchronizationCoordinatorV1,
    composition: &mut UniformSynchronizationComposition,
) -> Result<(), RemoteStage>
where
    T: vellum_sdk_client::protocol::ServicePort,
    P: vellum_semantic_session_client::SemanticSessionExchangePortV1,
{
    use vellum_semantic_protocol::{
        SemanticProviderReadinessProjectionV1, SemanticSessionLimitsV1, SemanticSessionReadinessV1,
    };
    use vellum_semantic_session_client::{
        VellumSnapshotSynchronizationExecutionV1, synchronize_vellum_snapshots,
    };

    let synchronize = |native: &mut vellum_sdk_client::client::Client<T>,
                       semantic: &mut vellum_semantic_session_client::SemanticSessionClientV1<P>,
                       coordinator: &mut vellum_semantic_session_client::VellumSnapshotSynchronizationCoordinatorV1,
                       composition: &mut UniformSynchronizationComposition|
     -> Result<(), RemoteStage> {
        match synchronize_vellum_snapshots(
            native,
            semantic,
            coordinator,
            composition,
            SemanticSessionLimitsV1::reference(),
        ) {
            VellumSnapshotSynchronizationExecutionV1::Synchronized => Ok(()),
            VellumSnapshotSynchronizationExecutionV1::Suspended(ref suspension) => {
                eprintln!("vellum-semantic-mcp: synchronization_refused result={suspension:?}");
                Err(RemoteStage::Synchronization)
            }
            VellumSnapshotSynchronizationExecutionV1::Refused(ref refusal) => {
                eprintln!("vellum-semantic-mcp: synchronization_refused result={refusal:?}");
                Err(RemoteStage::Synchronization)
            }
        }
    };

    synchronize(native, semantic, coordinator, composition)?;
    wait_for_ready(semantic, composition)?;
    synchronize(native, semantic, coordinator, composition)?;
    let final_tag = composition
        .next_value()
        .ok_or(RemoteStage::Synchronization)?;
    let description = describe_session(semantic, final_tag)?;
    if matches!(
        description.readiness(),
        SemanticSessionReadinessV1::Ready { .. }
    ) && matches!(
        description.provider().readiness(),
        SemanticProviderReadinessProjectionV1::Ready { .. }
    ) {
        Ok(())
    } else {
        Err(RemoteStage::Synchronization)
    }
}

#[cfg(unix)]
fn poll_workspace_changes<T>(
    native: &mut vellum_sdk_client::client::Client<T>,
    cursor: &mut Option<vellum_sdk_client::protocol::WorkspaceChangeCursorV1>,
    usage: &mut RemoteSynchronizationUsage,
) -> Result<bool, RemoteStage>
where
    T: vellum_sdk_client::protocol::ServicePort,
{
    use vellum_sdk_client::protocol::{OutcomeV1, ReplyV1, WorkspaceChangesV1};

    let mut dirty = false;
    for _ in 0..4_096 {
        let reply = native
            .workspace_changes(*cursor, 1)
            .map_err(|_| RemoteStage::Synchronization)?;
        match reply {
            ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(WorkspaceChangesV1::Page(page))) => {
                RemoteSynchronizationUsage::bump(&mut usage.feed_pages, &mut usage.saturated);
                RemoteSynchronizationUsage::add(
                    &mut usage.feed_events,
                    page.changes.len(),
                    &mut usage.saturated,
                );
                dirty |= !page.changes.is_empty();
                *cursor = Some(page.next);
                if page.next.revision() >= page.observed_head {
                    if !dirty {
                        RemoteSynchronizationUsage::bump(
                            &mut usage.quiescent_observations,
                            &mut usage.saturated,
                        );
                    }
                    return Ok(dirty);
                }
            }
            ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(WorkspaceChangesV1::Gap(gap))) => {
                RemoteSynchronizationUsage::bump(&mut usage.feed_gaps, &mut usage.saturated);
                *cursor = vellum_sdk_client::protocol::WorkspaceChangeCursorV1::new(
                    gap.workspace,
                    gap.epoch(),
                    gap.observed_head,
                );
                return cursor
                    .is_some()
                    .then_some(true)
                    .ok_or(RemoteStage::Synchronization);
            }
            _ => return Err(RemoteStage::Synchronization),
        }
    }
    Err(RemoteStage::Synchronization)
}

#[cfg(unix)]
fn converge_workspace<T, P>(
    native: &mut vellum_sdk_client::client::Client<T>,
    semantic: &mut vellum_semantic_session_client::SemanticSessionClientV1<P>,
    coordinator: &mut vellum_semantic_session_client::VellumSnapshotSynchronizationCoordinatorV1,
    composition: &mut UniformSynchronizationComposition,
    cursor: &mut Option<vellum_sdk_client::protocol::WorkspaceChangeCursorV1>,
    usage: &mut RemoteSynchronizationUsage,
) -> Result<(), RemoteStage>
where
    T: vellum_sdk_client::protocol::ServicePort,
    P: vellum_semantic_session_client::SemanticSessionExchangePortV1,
{
    for _ in 0..8 {
        if !poll_workspace_changes(native, cursor, usage)? {
            return Ok(());
        }
        RemoteSynchronizationUsage::bump(&mut usage.refreshes, &mut usage.saturated);
        synchronize_ready(native, semantic, coordinator, composition)?;
    }
    Err(RemoteStage::Synchronization)
}

#[cfg(unix)]
fn membership_needs_rollover<P: vellum_semantic_session_client::SemanticSessionExchangePortV1>(
    session: &vellum_semantic_session_client::SemanticSessionClientV1<P>,
) -> Result<bool, RemoteStage> {
    let limits = session
        .description()
        .ok_or(RemoteStage::SessionDescribe)?
        .limits();
    request_boundary_requires_rollover(session.usage().remaining_operations, limits.max_documents)
}

#[cfg(unix)]
fn request_boundary_requires_rollover(
    remaining_operations: u64,
    max_documents: u32,
) -> Result<bool, RemoteStage> {
    // One worst-case complete epoch (begin, every document, complete) plus
    // the already-arrived MCP frame's single mutation operation.
    let required = u64::from(max_documents)
        .checked_add(3)
        .ok_or(RemoteStage::Runtime)?;
    Ok(remaining_operations < required)
}

#[cfg(unix)]
fn rollover_membership<T, P>(
    native: &mut vellum_sdk_client::client::Client<T>,
    suspended: &mut SemanticMcpServerSuspensionV1<P>,
    coordinator: &mut vellum_semantic_session_client::VellumSnapshotSynchronizationCoordinatorV1,
    composition: &mut UniformSynchronizationComposition,
    usage: &mut RemoteSynchronizationUsage,
) -> Result<(), RemoteStage>
where
    T: vellum_sdk_client::protocol::ServicePort,
    P: vellum_semantic_session_client::SemanticSessionExchangePortV1 + Send,
{
    leave_session_with_composition(suspended.session_mut(), composition)?;
    let description = describe_session(
        suspended.session_mut(),
        composition
            .next_value()
            .ok_or(RemoteStage::SessionDescribe)?,
    )?;
    join_session(
        suspended.session_mut(),
        description,
        composition.next_value().ok_or(RemoteStage::SessionJoin)?,
        composition.next_value().ok_or(RemoteStage::SessionJoin)?,
    )?;
    *coordinator =
        vellum_semantic_session_client::VellumSnapshotSynchronizationCoordinatorV1::default();
    synchronize_ready(native, suspended.session_mut(), coordinator, composition)?;
    if suspended.session_mut().usage().remaining_operations == 0 {
        return Err(RemoteStage::Synchronization);
    }
    RemoteSynchronizationUsage::bump(&mut usage.rollovers, &mut usage.saturated);
    Ok(())
}

#[cfg(unix)]
fn reconnect_membership<T, P, F>(
    native: &mut vellum_sdk_client::client::Client<T>,
    suspended: SemanticMcpServerSuspensionV1<P>,
    coordinator: &mut vellum_semantic_session_client::VellumSnapshotSynchronizationCoordinatorV1,
    composition: &mut UniformSynchronizationComposition,
    usage: &mut RemoteSynchronizationUsage,
    connect: F,
) -> Result<SemanticMcpServerSuspensionV1<P>, RemoteStage>
where
    T: vellum_sdk_client::protocol::ServicePort,
    P: vellum_semantic_session_client::SemanticSessionExchangePortV1 + Send,
    F: FnOnce() -> Result<vellum_semantic_session_client::SemanticSessionClientV1<P>, RemoteStage>,
{
    let (reconnect, retired_port) = suspended
        .try_into_reconnect()
        .map_err(|_| RemoteStage::SessionReconnect)?;
    // Closing the old transport is the observable boundary that lets the
    // service retire authority and rotate its quiescent broker incarnation.
    drop(retired_port);
    let mut replacement = connect()?;
    let description = describe_session(
        &mut replacement,
        composition
            .next_value()
            .ok_or(RemoteStage::SessionDescribe)?,
    )?;
    join_session(
        &mut replacement,
        description,
        composition.next_value().ok_or(RemoteStage::SessionJoin)?,
        composition.next_value().ok_or(RemoteStage::SessionJoin)?,
    )?;
    *coordinator =
        vellum_semantic_session_client::VellumSnapshotSynchronizationCoordinatorV1::default();
    synchronize_ready(native, &mut replacement, coordinator, composition)?;
    if replacement.usage().remaining_operations == 0 {
        return Err(RemoteStage::Synchronization);
    }
    let suspended = reconnect
        .try_bind(replacement)
        .map_err(|_| RemoteStage::QueryBinding)?;
    RemoteSynchronizationUsage::bump(&mut usage.rollovers, &mut usage.saturated);
    RemoteSynchronizationUsage::bump(&mut usage.reconnects, &mut usage.saturated);
    Ok(suspended)
}

#[cfg(unix)]
fn retire_suspended_server<P>(
    suspended: SemanticMcpServerSuspensionV1<P>,
) -> Result<(), RemoteStage>
where
    P: vellum_semantic_session_client::SemanticSessionExchangePortV1 + Send,
{
    let mut extraction = suspended.into_query_suspension().into_extraction();
    if extraction.session.binding().is_some() {
        leave_session(
            &mut extraction.session,
            extraction.next_exchange,
            extraction.next_operation,
        )
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn run_remote(configuration: RemoteArguments) -> Result<(), RemoteStage> {
    use vellum_adapter_local_authentication::{
        LocalCredentialClaimProviderV1, LocalCredentialStoreV1,
    };
    use vellum_sdk_client::{
        client::Client,
        transport::{AuthenticatedUnixConnector, RemoteTransport, TransportLimits},
    };
    use vellum_semantic_session_client::{
        SemanticSessionClientLimitsV1, SemanticSessionClientV1,
        VellumSnapshotSynchronizationCoordinatorV1,
    };
    use vellum_semantic_transport::{
        SemanticRemoteSessionV1, SemanticTransportLimitsV1, UnixSemanticFrameDeadlinesV1,
        UnixSemanticFrameExchangeV1,
    };

    let mut store = LocalCredentialStoreV1::default();
    let native_record = store
        .load(&configuration.native_credential)
        .map_err(|_| RemoteStage::Credential)?;
    let semantic_socket = configuration.semantic_socket.clone();
    let semantic_credential = configuration.semantic_credential.clone();
    let mut connect_semantic = || {
        let semantic_record = store
            .load(&semantic_credential)
            .map_err(|_| RemoteStage::Credential)?;
        let exchange = UnixSemanticFrameExchangeV1::try_new_authenticated(
            semantic_socket.clone(),
            UnixSemanticFrameDeadlinesV1::reference(),
            semantic_authentication(semantic_record),
        )
        .map_err(|_| RemoteStage::SemanticTransport)?;
        let remote =
            SemanticRemoteSessionV1::try_new(exchange, SemanticTransportLimitsV1::reference())
                .map_err(|_| RemoteStage::SemanticTransport)?;
        SemanticSessionClientV1::try_new(remote, SemanticSessionClientLimitsV1::reference())
            .map_err(|_| RemoteStage::SemanticTransport)
    };
    let mut semantic = connect_semantic()?;
    let mut composition =
        UniformSynchronizationComposition::try_new(configuration.language.clone())
            .ok_or(RemoteStage::Runtime)?;
    let description = describe_session(
        &mut semantic,
        composition
            .next_value()
            .ok_or(RemoteStage::SessionDescribe)?,
    )?;
    join_session(
        &mut semantic,
        description,
        composition.next_value().ok_or(RemoteStage::SessionJoin)?,
        composition.next_value().ok_or(RemoteStage::SessionJoin)?,
    )?;
    macro_rules! fail_joined {
        ($stage:expr) => {{
            let cleanup = leave_session(&mut semantic, u128::MAX - 2_048, u128::MAX - 1_024);
            return Err(cleanup.err().unwrap_or($stage));
        }};
    }

    let native_connector = AuthenticatedUnixConnector::with_provider(
        configuration.native_socket,
        LocalCredentialClaimProviderV1::new(native_record),
    );
    let native_transport =
        match RemoteTransport::new(native_connector, TransportLimits::reference()) {
            Ok(transport) => transport,
            Err(_) => fail_joined!(RemoteStage::NativeTransport),
        };
    let mut native = match Client::connect(native_transport) {
        Ok(native) => native,
        Err(_) => fail_joined!(RemoteStage::NativeTransport),
    };
    let mut coordinator = VellumSnapshotSynchronizationCoordinatorV1::default();
    if synchronize_ready(
        &mut native,
        &mut semantic,
        &mut coordinator,
        &mut composition,
    )
    .is_err()
    {
        let _ = native.disconnect();
        fail_joined!(RemoteStage::Synchronization);
    }

    let limits = SemanticMcpLimitsV1::reference();
    let documents = match semantic_documents(&coordinator, limits) {
        Ok(documents) => documents,
        Err(_) => {
            let _ = native.disconnect();
            fail_joined!(RemoteStage::Catalog);
        }
    };
    let query_seed = composition.query_seed().ok_or(RemoteStage::QueryBinding)?;
    let client = match try_bind_semantic_session_query_v1(semantic, query_seed) {
        Ok(client) => client,
        Err(failure) => {
            let mut semantic = failure.into_session();
            let cleanup = leave_session(&mut semantic, u128::MAX - 2_048, u128::MAX - 1_024);
            return Err(cleanup.err().unwrap_or(RemoteStage::QueryBinding));
        }
    };
    let server = SemanticMcpServerV1::try_new_with_documents(client, limits, documents)
        .map_err(|_| RemoteStage::Mcp)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .map_err(|_| RemoteStage::Runtime)?;
    let mut input = BufReader::new(tokio::io::stdin());
    let mut output = tokio::io::stdout();
    let mut server = Some(server);
    let mut cursor = None;
    let mut synchronization_usage = RemoteSynchronizationUsage::default();
    let mut stdio_usage = SemanticMcpStdioUsageV1::default();
    let stdio_result = loop {
        let frame = match runtime.block_on(read_stdio_frame_v1(
            &mut input,
            SemanticMcpStdioLimitsV1::reference(),
            limits.max_request_bytes,
            &mut stdio_usage,
        )) {
            Ok(SemanticMcpStdioFrameV1::Eof) => break Ok(stdio_usage),
            Ok(frame) => frame,
            Err(failure) => break Err(failure),
        };
        if !matches!(&frame, SemanticMcpStdioFrameV1::Complete { .. }) {
            let step = runtime.block_on(serve_stdio_read_frame_v1(
                server.as_mut().ok_or(RemoteStage::Mcp)?,
                frame,
                &mut output,
                &mut stdio_usage,
            ));
            match step {
                Ok(SemanticMcpStdioStepV1::Served) => continue,
                Ok(SemanticMcpStdioStepV1::Eof) => break Ok(stdio_usage),
                Err(failure) => break Err(failure),
            }
        }
        let current = server.take().ok_or(RemoteStage::Mcp)?;
        let mut suspended = match current.try_suspend_client() {
            Ok(suspended) => suspended,
            Err(current) => {
                // A transport-unknown query mutation must be replayed exactly
                // before synchronization authority can safely cross the seam.
                server = Some(*current);
                let step = runtime.block_on(serve_stdio_read_frame_v1(
                    server.as_mut().ok_or(RemoteStage::Mcp)?,
                    frame,
                    &mut output,
                    &mut stdio_usage,
                ));
                match step {
                    Ok(SemanticMcpStdioStepV1::Served) => continue,
                    Ok(SemanticMcpStdioStepV1::Eof) => break Ok(stdio_usage),
                    Err(failure) => break Err(failure),
                }
            }
        };
        let rollover_required = match membership_needs_rollover(suspended.session_mut()) {
            Ok(required) => required,
            Err(stage) => {
                let cleanup = retire_suspended_server(suspended);
                let _ = native.disconnect();
                return Err(cleanup.err().unwrap_or(stage));
            }
        };
        if rollover_required {
            match rollover_membership(
                &mut native,
                &mut suspended,
                &mut coordinator,
                &mut composition,
                &mut synchronization_usage,
            ) {
                Ok(()) => {}
                Err(RemoteStage::SessionReconnect) => {
                    suspended = match reconnect_membership(
                        &mut native,
                        suspended,
                        &mut coordinator,
                        &mut composition,
                        &mut synchronization_usage,
                        &mut connect_semantic,
                    ) {
                        Ok(suspended) => suspended,
                        Err(stage) => {
                            let _ = native.disconnect();
                            return Err(stage);
                        }
                    };
                }
                Err(stage) => {
                    let cleanup = retire_suspended_server(suspended);
                    let _ = native.disconnect();
                    return Err(cleanup.err().unwrap_or(stage));
                }
            }
        }
        if let Err(stage) = converge_workspace(
            &mut native,
            suspended.session_mut(),
            &mut coordinator,
            &mut composition,
            &mut cursor,
            &mut synchronization_usage,
        ) {
            let cleanup = retire_suspended_server(suspended);
            let _ = native.disconnect();
            return Err(cleanup.err().unwrap_or(stage));
        }
        let refreshed_documents = match semantic_documents(&coordinator, limits) {
            Ok(documents) => documents,
            Err(stage) => {
                let cleanup = retire_suspended_server(suspended);
                let _ = native.disconnect();
                return Err(cleanup.err().unwrap_or(stage));
            }
        };
        let resumed = match suspended.try_resume(refreshed_documents) {
            Ok(resumed) => resumed,
            Err(failure) => {
                let cleanup = retire_suspended_server(failure.into_suspension());
                let _ = native.disconnect();
                return Err(cleanup.err().unwrap_or(RemoteStage::QueryBinding));
            }
        };
        server = Some(resumed);
        let step = runtime.block_on(serve_stdio_read_frame_v1(
            server.as_mut().ok_or(RemoteStage::Mcp)?,
            frame,
            &mut output,
            &mut stdio_usage,
        ));
        match step {
            Ok(SemanticMcpStdioStepV1::Served) => {}
            Ok(SemanticMcpStdioStepV1::Eof) => break Ok(stdio_usage),
            Err(failure) => break Err(failure),
        }
    };
    let server = server.ok_or(RemoteStage::Mcp)?;
    let usage = server.usage();
    let cleanup_result = (|| {
        let query = server.into_client().into_port();
        let mut extraction = query.try_into_session().map_err(|_| RemoteStage::Cleanup)?;
        leave_session(
            &mut extraction.session,
            extraction.next_exchange,
            extraction.next_operation,
        )
    })();
    let native_disconnect = native
        .disconnect()
        .map_err(|_| RemoteStage::NativeDisconnect);
    cleanup_result?;
    native_disconnect?;
    let stdio = stdio_result.map_err(|_| RemoteStage::Stdio)?;
    if configuration.metrics {
        eprintln!(
            "{}",
            json!({
                "event":"vellum_semantic_mcp_exit",
                "backend":"semantic_session",
                "stdio":{
                    "frames":stdio.frames,
                    "frame_bytes":stdio.frame_bytes,
                    "responses":stdio.responses,
                    "response_bytes":stdio.response_bytes,
                    "notifications":stdio.notifications,
                    "oversized_frames":stdio.oversized_frames,
                    "truncated_frames":stdio.truncated_frames,
                    "io_failures":stdio.io_failures,
                    "saturated":stdio.saturated
                },
                "mcp":{
                    "requests":usage.requests,
                    "tool_calls":usage.tool_calls,
                    "queries_submitted":usage.queries_submitted,
                    "released":usage.released,
                    "refused":usage.refused,
                    "protocol_errors":usage.protocol_errors,
                    "saturated":usage.saturated
                },
                "synchronization":{
                    "feed_pages":synchronization_usage.feed_pages,
                    "feed_events":synchronization_usage.feed_events,
                    "feed_gaps":synchronization_usage.feed_gaps,
                    "refreshes":synchronization_usage.refreshes,
                    "rollovers":synchronization_usage.rollovers,
                    "reconnects":synchronization_usage.reconnects,
                    "quiescent_observations":synchronization_usage.quiescent_observations,
                    "saturated":synchronization_usage.saturated
                },
                "synchronized_documents":coordinator.usage().synchronized_documents
            })
        );
    }
    Ok(())
}

fn run_unavailable(metrics: bool) -> ExitCode {
    let provider = UnavailableProvider::new();
    let mut service =
        match SemanticServiceV1::try_new(provider, SemanticServiceLimitsV1::reference()) {
            Ok(service) => service,
            Err(_) => {
                eprintln!("vellum-semantic-mcp: local composition refused");
                return ExitCode::from(2);
            }
        };
    let client_ref = SemanticClientRefV1::new([0x6d; 16]);
    if service.admit_client(client_ref).is_err() {
        eprintln!("vellum-semantic-mcp: client admission refused");
        return ExitCode::from(2);
    }

    let runtime = match tokio::runtime::Builder::new_current_thread().build() {
        Ok(runtime) => runtime,
        Err(_) => {
            eprintln!("vellum-semantic-mcp: runtime initialization failed");
            return ExitCode::from(2);
        }
    };
    let (stdio_result, mcp_usage) = {
        let client = bind_semantic_client_v1(&mut service, client_ref);
        let mut server =
            match SemanticMcpServerV1::try_new(client, SemanticMcpLimitsV1::reference()) {
                Ok(server) => server,
                Err(_) => {
                    eprintln!("vellum-semantic-mcp: MCP composition refused");
                    return ExitCode::from(2);
                }
            };
        let mut input = BufReader::new(tokio::io::stdin());
        let mut output = tokio::io::stdout();
        let result = runtime.block_on(serve_stdio_v1(
            &mut server,
            &mut input,
            &mut output,
            SemanticMcpStdioLimitsV1::reference(),
        ));
        let mcp_usage = server.usage();
        (result, mcp_usage)
    };

    let retirement = service.retire_client(client_ref);
    let stdio_usage = match stdio_result {
        Ok(usage) => usage,
        Err(failure) => {
            report_failure(&failure);
            return ExitCode::FAILURE;
        }
    };
    if metrics {
        eprintln!(
            "{}",
            json!({
                "event":"vellum_semantic_mcp_exit",
                "backend":"unavailable",
                "stdio":{
                    "frames":stdio_usage.frames,
                    "frame_bytes":stdio_usage.frame_bytes,
                    "responses":stdio_usage.responses,
                    "response_bytes":stdio_usage.response_bytes,
                    "notifications":stdio_usage.notifications,
                    "oversized_frames":stdio_usage.oversized_frames,
                    "truncated_frames":stdio_usage.truncated_frames,
                    "io_failures":stdio_usage.io_failures,
                    "saturated":stdio_usage.saturated
                },
                "mcp":{
                    "requests":mcp_usage.requests,
                    "tool_calls":mcp_usage.tool_calls,
                    "queries_submitted":mcp_usage.queries_submitted,
                    "released":mcp_usage.released,
                    "refused":mcp_usage.refused,
                    "protocol_errors":mcp_usage.protocol_errors,
                    "saturated":mcp_usage.saturated
                },
                "retirement":retirement_state(retirement.state)
            })
        );
    }
    ExitCode::SUCCESS
}

const fn retirement_state(state: SemanticClientRetirementStateV1) -> &'static str {
    match state {
        SemanticClientRetirementStateV1::Absent => "absent",
        SemanticClientRetirementStateV1::Pending { .. } => "pending",
        SemanticClientRetirementStateV1::Retired { .. } => "retired",
    }
}

fn report_failure(failure: &SemanticMcpStdioFailureV1) {
    match failure {
        SemanticMcpStdioFailureV1::InvalidLimits => {
            eprintln!("vellum-semantic-mcp: invalid stdio limits");
        }
        SemanticMcpStdioFailureV1::Io { .. } => {
            eprintln!("vellum-semantic-mcp: stdio failed");
        }
    }
}

struct UnavailableProvider {
    state: SemanticProviderStateV1,
}

impl UnavailableProvider {
    fn new() -> Self {
        let provenance = SemanticProviderProvenanceV1::try_new(
            SemanticProviderRefV1::new([0x75; 16]),
            "unavailable-composition".to_owned(),
            env!("CARGO_PKG_VERSION").to_owned(),
        )
        .expect("fixed unavailable provider provenance is valid");
        Self {
            state: SemanticProviderStateV1 {
                provider: provenance,
                readiness: vellum_semantic_client::SemanticProviderReadinessV1::Unavailable,
                capabilities: SemanticCapabilitySetV1::bottom(),
            },
        }
    }
}

impl SemanticProviderPortV1 for UnavailableProvider {
    fn state(&self) -> SemanticProviderStateV1 {
        self.state.clone()
    }

    fn synchronize(&mut self, _update: &SemanticSnapshotUpdateV1) -> SemanticProviderAdmissionV1 {
        SemanticProviderAdmissionV1::Unavailable
    }

    fn submit(&mut self, _request: &SemanticRequestV1) -> SemanticProviderSubmissionV1 {
        SemanticProviderSubmissionV1::Unavailable
    }

    fn cancel(&mut self, _request: SemanticRequestRefV1) {}

    fn poll(&mut self) -> Option<SemanticProviderCompletionV1> {
        None
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::os::unix::ffi::OsStringExt as _;
    use std::sync::Arc;
    use vellum_sdk_client::protocol::{
        ClientId, CommandV1, OutcomeV1, ReplyV1, RequestV1, ServiceAccessErrorV1,
        ServiceControlErrorV1, ServicePort, SnapshotV1, VersionWitness, WorkspaceChangeCursorV1,
        WorkspaceChangeGapV1, WorkspaceChangeKindV1, WorkspaceChangePageV1, WorkspaceChangeV1,
        WorkspaceChangesV1, WorkspaceDocumentIdV1, WorkspaceIdV1, WorkspaceInventoryV1,
    };
    use vellum_semantic_protocol::{
        SemanticExchangeDispositionV1, SemanticMembershipRefV1, SemanticProviderObservationV1,
        SemanticProviderReadinessProjectionV1, SemanticSessionCommandV1,
        SemanticSessionDescriptionV1, SemanticSessionIncarnationV1, SemanticSessionLimitsV1,
        SemanticSessionOutcomeV1, SemanticSessionReadinessV1, SemanticSessionReplyV1,
        SemanticSessionRolesV1,
    };
    use vellum_semantic_session_client::{
        SemanticSessionClientLimitsV1, SemanticSessionClientV1, SemanticSessionExchangePortV1,
        VellumSnapshotSynchronizationCoordinatorV1,
    };

    struct ChangeTransport {
        replies: VecDeque<ReplyV1>,
    }

    struct SnapshotTransport {
        workspace: WorkspaceIdV1,
        document: WorkspaceDocumentIdV1,
    }

    struct RolloverSemanticPort {
        incarnation: SemanticSessionIncarnationV1,
        next_membership: u8,
        membership: Option<SemanticMembershipRefV1>,
        epoch: Option<vellum_semantic_protocol::SemanticSynchronizationEpochV1>,
        limits: SemanticSessionLimitsV1,
        replay_capacity_on_join: bool,
    }

    impl ServicePort for ChangeTransport {
        fn connect(&mut self) -> Result<ClientId, ServiceControlErrorV1> {
            Ok(ClientId::new(7).unwrap())
        }

        fn execute(&mut self, _request: RequestV1) -> Result<ReplyV1, ServiceAccessErrorV1> {
            self.replies
                .pop_front()
                .ok_or(ServiceAccessErrorV1::Unavailable)
        }

        fn disconnect(&mut self, _client: ClientId) -> Result<(), ServiceControlErrorV1> {
            Ok(())
        }
    }

    impl ServicePort for SnapshotTransport {
        fn connect(&mut self) -> Result<ClientId, ServiceControlErrorV1> {
            Ok(ClientId::new(7).unwrap())
        }

        fn execute(&mut self, request: RequestV1) -> Result<ReplyV1, ServiceAccessErrorV1> {
            Ok(match request.command {
                CommandV1::WorkspaceInventory => {
                    ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(WorkspaceInventoryV1 {
                        workspace: self.workspace,
                        documents: vec![self.document],
                        roots: Vec::new(),
                    }))
                }
                CommandV1::SelectDocument { document } if document == self.document => {
                    ReplyV1::Accepted(OutcomeV1::DocumentSelected(document))
                }
                CommandV1::Snapshot => ReplyV1::Accepted(OutcomeV1::Snapshot(SnapshotV1 {
                    witness: VersionWitness::new(1).unwrap(),
                    revision: 1,
                    text: Arc::from("pub fn answer() -> i32 { 42 }\n"),
                })),
                _ => panic!("unexpected native rollover command"),
            })
        }

        fn disconnect(&mut self, _client: ClientId) -> Result<(), ServiceControlErrorV1> {
            Ok(())
        }
    }

    impl RolloverSemanticPort {
        fn description(&self) -> SemanticSessionDescriptionV1 {
            let (readiness, provider_readiness) = self.epoch.map_or(
                (
                    SemanticSessionReadinessV1::Unavailable,
                    SemanticProviderReadinessProjectionV1::Indexing,
                ),
                |epoch| {
                    (
                        SemanticSessionReadinessV1::Ready { epoch },
                        SemanticProviderReadinessProjectionV1::Ready {
                            generation: vellum_semantic_client::SemanticIndexGenerationV1::new(1)
                                .unwrap(),
                        },
                    )
                },
            );
            SemanticSessionDescriptionV1::try_new(
                self.incarnation,
                SemanticSessionRolesV1::top(),
                readiness,
                SemanticProviderObservationV1::new(
                    provider_readiness,
                    SemanticCapabilitySetV1::top(),
                ),
                self.limits,
            )
            .unwrap()
        }

        fn reply(
            &self,
            command: &SemanticSessionCommandV1,
            outcome: SemanticSessionOutcomeV1,
        ) -> SemanticSessionReplyV1 {
            let membership = match &outcome {
                SemanticSessionOutcomeV1::Joined { membership, .. } => Some(*membership),
                _ => command.binding().map(|(_, membership)| membership),
            };
            SemanticSessionReplyV1::try_new(
                command.kind(),
                command.exchange(),
                self.incarnation,
                membership,
                command.operation(),
                outcome,
            )
            .unwrap()
        }
    }

    impl SemanticSessionExchangePortV1 for RolloverSemanticPort {
        fn exchange(
            &mut self,
            command: &SemanticSessionCommandV1,
        ) -> SemanticExchangeDispositionV1<SemanticSessionReplyV1> {
            let outcome = match command {
                SemanticSessionCommandV1::Describe { .. } => {
                    SemanticSessionOutcomeV1::Description(self.description())
                }
                SemanticSessionCommandV1::JoinSession { .. } => {
                    if self.replay_capacity_on_join {
                        self.replay_capacity_on_join = false;
                        return SemanticExchangeDispositionV1::Reply(
                            self.reply(command, SemanticSessionOutcomeV1::ReplayCapacity),
                        );
                    }
                    self.next_membership = self.next_membership.checked_add(1).unwrap();
                    let membership = SemanticMembershipRefV1::new([self.next_membership; 16])
                        .expect("nonzero membership");
                    self.membership = Some(membership);
                    SemanticSessionOutcomeV1::Joined {
                        membership,
                        granted_roles: SemanticSessionRolesV1::top(),
                        remaining_operations: self.limits.ordinary_operation_budget().unwrap(),
                    }
                }
                SemanticSessionCommandV1::BeginSynchronization { .. }
                | SemanticSessionCommandV1::SyncOpen { .. }
                | SemanticSessionCommandV1::SyncChange { .. }
                | SemanticSessionCommandV1::SyncClose { .. } => SemanticSessionOutcomeV1::Accepted,
                SemanticSessionCommandV1::CompleteSynchronization { epoch, .. } => {
                    self.epoch = Some(*epoch);
                    SemanticSessionOutcomeV1::Accepted
                }
                SemanticSessionCommandV1::Submit { .. } => SemanticSessionOutcomeV1::Accepted,
                SemanticSessionCommandV1::Collect { request, .. } => {
                    SemanticSessionOutcomeV1::Pending(
                        vellum_semantic_protocol::SemanticSessionPendingV1::Request(*request),
                    )
                }
                SemanticSessionCommandV1::ReleaseRequest { .. } => {
                    SemanticSessionOutcomeV1::Accepted
                }
                SemanticSessionCommandV1::LeaveSession { .. } => {
                    self.membership = None;
                    self.epoch = None;
                    SemanticSessionOutcomeV1::Accepted
                }
                _ => panic!("unexpected semantic rollover command"),
            };
            SemanticExchangeDispositionV1::Reply(self.reply(command, outcome))
        }
    }

    fn valid_arguments() -> Vec<OsString> {
        [
            "--semantic-socket",
            "/private/semantic.sock",
            "--semantic-credential",
            "/private/semantic.credential",
            "--native-socket",
            "/private/native.sock",
            "--native-credential",
            "/private/native.credential",
            "--document-language",
            "rust",
        ]
        .into_iter()
        .map(OsString::from)
        .collect()
    }

    #[test]
    fn real_mode_requires_every_explicit_connection_and_language_input() {
        let parsed = parse_remote_arguments(&valid_arguments()).unwrap();
        assert_eq!(parsed.language, "rust");
        assert!(!parsed.metrics);

        let mut missing = valid_arguments();
        missing.drain(2..4);
        assert_eq!(
            parse_remote_arguments(&missing),
            Err("missing semantic credential")
        );
    }

    #[test]
    fn duplicate_unknown_and_ambiguous_language_options_refuse() {
        let mut duplicate = valid_arguments();
        duplicate.extend([
            OsString::from("--metrics-stderr"),
            OsString::from("--metrics-stderr"),
        ]);
        assert_eq!(
            parse_remote_arguments(&duplicate),
            Err("duplicate metrics option")
        );

        let mut unknown = valid_arguments();
        unknown.extend([OsString::from("--language"), OsString::from("rust")]);
        assert_eq!(parse_remote_arguments(&unknown), Err("unknown option"));
    }

    #[test]
    fn non_utf8_option_names_and_language_refuse_without_disappearing() {
        let mut option = valid_arguments();
        option.push(OsString::from_vec(vec![0xff]));
        assert_eq!(
            parse_remote_arguments(&option),
            Err("option name is not UTF-8")
        );

        let mut language = valid_arguments();
        *language.last_mut().unwrap() = OsString::from_vec(vec![0xff]);
        assert_eq!(
            parse_remote_arguments(&language),
            Err("language is not UTF-8")
        );
    }

    #[test]
    fn synchronization_correlations_start_fresh_with_bounded_headroom() {
        let mut composition =
            UniformSynchronizationComposition::try_new("rust".to_owned()).unwrap();
        let first = composition.next_value().unwrap();
        let second = composition.next_value().unwrap();
        let query = composition.query_seed().unwrap();
        assert_ne!(first, 0);
        assert!(first < (1_u128 << 126));
        assert_eq!(second, first + 1);
        assert!(second < UniformSynchronizationComposition::QUERY_PARTITION);
        assert_eq!(query, composition.next + (1_u128 << 127));
        assert!(query > UniformSynchronizationComposition::QUERY_PARTITION);
    }

    #[test]
    fn retirement_progress_must_decrease_within_the_declared_tail() {
        assert!(retirement_progress_is_bounded(None, (2, 1), 3));
        assert!(retirement_progress_is_bounded(Some((2, 1)), (1, 1), 3));
        assert!(retirement_progress_is_bounded(Some((1, 1)), (1, 0), 3));
        assert!(!retirement_progress_is_bounded(Some((1, 1)), (1, 1), 3));
        assert!(!retirement_progress_is_bounded(Some((1, 1)), (2, 0), 3));
        assert!(!retirement_progress_is_bounded(None, (3, 1), 3));
    }

    #[test]
    fn request_boundary_reserves_one_full_epoch_and_one_frame_operation() {
        assert!(!request_boundary_requires_rollover(7, 4).unwrap());
        assert!(request_boundary_requires_rollover(6, 4).unwrap());
        assert!(!request_boundary_requires_rollover(3, 0).unwrap());
        assert!(request_boundary_requires_rollover(2, 0).unwrap());
    }

    #[test]
    fn definitive_join_replay_capacity_selects_the_reconnect_path() {
        let limits = SemanticSessionLimitsV1::reference();
        let mut semantic = SemanticSessionClientV1::try_new(
            RolloverSemanticPort {
                incarnation: SemanticSessionIncarnationV1::new([7; 16]).unwrap(),
                next_membership: 1,
                membership: None,
                epoch: None,
                limits,
                replay_capacity_on_join: true,
            },
            SemanticSessionClientLimitsV1::reference(),
        )
        .unwrap();
        let description = describe_session(&mut semantic, 1).unwrap();
        assert_eq!(
            join_session(&mut semantic, description, 2, 3),
            Err(RemoteStage::SessionReconnect)
        );
        assert!(semantic.binding().is_none());
        assert!(semantic.pending_mutation().is_none());
    }

    #[test]
    fn rollover_leaves_rejoins_reopens_and_resumes_the_same_mcp_owner() {
        let workspace = WorkspaceIdV1::new(1).unwrap();
        let document = WorkspaceDocumentIdV1::new(workspace, 1).unwrap();
        let limits = SemanticSessionLimitsV1::reference();
        let port = RolloverSemanticPort {
            incarnation: SemanticSessionIncarnationV1::new([1; 16]).unwrap(),
            next_membership: 1,
            membership: None,
            epoch: None,
            limits,
            replay_capacity_on_join: false,
        };
        let mut semantic =
            SemanticSessionClientV1::try_new(port, SemanticSessionClientLimitsV1::reference())
                .unwrap();
        let mut composition =
            UniformSynchronizationComposition::try_new("rust".to_owned()).unwrap();
        let description = describe_session(&mut semantic, composition.next_value().unwrap())
            .expect("description");
        join_session(
            &mut semantic,
            description,
            composition.next_value().unwrap(),
            composition.next_value().unwrap(),
        )
        .expect("join");
        let first_membership = semantic.binding().unwrap().membership;
        let mut native = vellum_sdk_client::client::Client::connect(SnapshotTransport {
            workspace,
            document,
        })
        .unwrap();
        let mut coordinator = VellumSnapshotSynchronizationCoordinatorV1::default();
        synchronize_ready(
            &mut native,
            &mut semantic,
            &mut coordinator,
            &mut composition,
        )
        .expect("initial synchronization");
        let query = try_bind_semantic_session_query_v1(semantic, composition.query_seed().unwrap())
            .unwrap();
        let mcp_limits = SemanticMcpLimitsV1::reference();
        let documents = semantic_documents(&coordinator, mcp_limits).unwrap();
        let server =
            SemanticMcpServerV1::try_new_with_documents(query, mcp_limits, documents).unwrap();
        let mut suspended = match server.try_suspend_client() {
            Ok(suspended) => suspended,
            Err(_) => panic!("clean MCP owner must suspend"),
        };
        let mut usage = RemoteSynchronizationUsage::default();

        rollover_membership(
            &mut native,
            &mut suspended,
            &mut coordinator,
            &mut composition,
            &mut usage,
        )
        .expect("rollover");

        let second_membership = suspended.session_mut().binding().unwrap().membership;
        assert_ne!(first_membership, second_membership);
        assert_eq!(usage.rollovers, 1);
        assert_eq!(coordinator.synchronized_documents().len(), 1);
        let documents = semantic_documents(&coordinator, mcp_limits).unwrap();
        let server = suspended.try_resume(documents).unwrap();
        let query = server.into_client().into_port();
        let mut extraction = match query.try_into_session() {
            Ok(extraction) => extraction,
            Err(_) => panic!("resumed query owner must extract"),
        };
        leave_session(
            &mut extraction.session,
            extraction.next_exchange,
            extraction.next_operation,
        )
        .unwrap();
    }

    #[test]
    fn reconnect_rebinds_a_retired_transport_without_losing_the_mcp_owner() {
        let workspace = WorkspaceIdV1::new(21).unwrap();
        let document = WorkspaceDocumentIdV1::new(workspace, 22).unwrap();
        let limits = SemanticSessionLimitsV1::reference();
        let port = RolloverSemanticPort {
            incarnation: SemanticSessionIncarnationV1::new([1; 16]).unwrap(),
            next_membership: 1,
            membership: None,
            epoch: None,
            limits,
            replay_capacity_on_join: false,
        };
        let mut semantic =
            SemanticSessionClientV1::try_new(port, SemanticSessionClientLimitsV1::reference())
                .unwrap();
        let mut composition =
            UniformSynchronizationComposition::try_new("rust".to_owned()).unwrap();
        let description = describe_session(&mut semantic, composition.next_value().unwrap())
            .expect("description");
        join_session(
            &mut semantic,
            description,
            composition.next_value().unwrap(),
            composition.next_value().unwrap(),
        )
        .expect("join");
        let original = semantic.binding().unwrap();
        let mut native = vellum_sdk_client::client::Client::connect(SnapshotTransport {
            workspace,
            document,
        })
        .unwrap();
        let mut coordinator = VellumSnapshotSynchronizationCoordinatorV1::default();
        synchronize_ready(
            &mut native,
            &mut semantic,
            &mut coordinator,
            &mut composition,
        )
        .expect("initial synchronization");
        let query = try_bind_semantic_session_query_v1(semantic, composition.query_seed().unwrap())
            .unwrap();
        let mcp_limits = SemanticMcpLimitsV1::reference();
        let documents = semantic_documents(&coordinator, mcp_limits).unwrap();
        let semantic_document = documents[0].document();
        let source_revision = documents[0].source_revision().get();
        let document_hex: String = semantic_document
            .as_bytes()
            .into_iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let request = serde_json::to_vec(&json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"tools/call",
            "params":{
                "_meta":{
                    "io.modelcontextprotocol/protocolVersion":"2026-07-28",
                    "io.modelcontextprotocol/clientCapabilities":{},
                    "io.modelcontextprotocol/clientInfo":{"name":"reconnect-test","version":"1"}
                },
                "name":"vellum.semantic.diagnostics",
                "arguments":{
                    "request":"11111111111111111111111111111111",
                    "document":document_hex,
                    "source_revision":source_revision,
                    "index_generation":1,
                    "language":"rust"
                }
            }
        }))
        .unwrap();
        let mut server =
            SemanticMcpServerV1::try_new_with_documents(query, mcp_limits, documents).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let pending: serde_json::Value = serde_json::from_slice(
            &runtime
                .block_on(server.handle(&request))
                .expect("query response"),
        )
        .unwrap();
        assert_eq!(
            pending.pointer("/result/structuredContent/status"),
            Some(&json!("pending"))
        );
        let mut suspended = match server.try_suspend_client() {
            Ok(suspended) => suspended,
            Err(_) => panic!("clean MCP owner must suspend"),
        };
        leave_session_with_composition(suspended.session_mut(), &mut composition).unwrap();
        assert!(suspended.session_mut().binding().is_none());

        let mut usage = RemoteSynchronizationUsage::default();
        let mut suspended = reconnect_membership(
            &mut native,
            suspended,
            &mut coordinator,
            &mut composition,
            &mut usage,
            || {
                SemanticSessionClientV1::try_new(
                    RolloverSemanticPort {
                        incarnation: SemanticSessionIncarnationV1::new([2; 16]).unwrap(),
                        next_membership: 9,
                        membership: None,
                        epoch: None,
                        limits,
                        replay_capacity_on_join: false,
                    },
                    SemanticSessionClientLimitsV1::reference(),
                )
                .map_err(|_| RemoteStage::SemanticTransport)
            },
        )
        .expect("reconnect");
        let rebound = suspended.session_mut().binding().unwrap();
        assert_ne!(rebound.incarnation, original.incarnation);
        assert_ne!(rebound.membership, original.membership);
        assert_eq!(usage.reconnects, 1);
        assert_eq!(usage.rollovers, 1);
        assert_eq!(coordinator.synchronized_documents().len(), 1);

        let documents = semantic_documents(&coordinator, mcp_limits).unwrap();
        let mut server = suspended.try_resume(documents).unwrap();
        let cancelled: serde_json::Value = serde_json::from_slice(
            &runtime
                .block_on(server.handle(&request))
                .expect("cancelled replay response"),
        )
        .unwrap();
        assert_eq!(
            cancelled.pointer("/result/structuredContent/status"),
            Some(&json!("cancelled"))
        );
        assert_eq!(server.usage().cancelled, 1);
        let query = server.into_client().into_port();
        let mut extraction = match query.try_into_session() {
            Ok(extraction) => extraction,
            Err(_) => panic!("reconnected query owner must extract"),
        };
        leave_session(
            &mut extraction.session,
            extraction.next_exchange,
            extraction.next_operation,
        )
        .unwrap();
    }

    #[test]
    fn change_feed_marks_events_and_then_observes_quiescence() {
        let workspace = WorkspaceIdV1::new(11).unwrap();
        let document = WorkspaceDocumentIdV1::new(workspace, 13).unwrap();
        let next = WorkspaceChangeCursorV1::new(workspace, 5, 1).unwrap();
        let page = ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(WorkspaceChangesV1::Page(
            WorkspaceChangePageV1 {
                workspace,
                observed_head: 1,
                changes: vec![
                    WorkspaceChangeV1::new(1, WorkspaceChangeKindV1::DocumentAdded(document))
                        .unwrap(),
                ],
                next,
            },
        )));
        let empty = ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(WorkspaceChangesV1::Page(
            WorkspaceChangePageV1 {
                workspace,
                observed_head: 1,
                changes: Vec::new(),
                next,
            },
        )));
        let transport = ChangeTransport {
            replies: VecDeque::from([page, empty]),
        };
        let mut client = vellum_sdk_client::client::Client::connect(transport).unwrap();
        let mut cursor = None;
        let mut usage = RemoteSynchronizationUsage::default();

        assert!(poll_workspace_changes(&mut client, &mut cursor, &mut usage).unwrap());
        assert!(!poll_workspace_changes(&mut client, &mut cursor, &mut usage).unwrap());
        assert_eq!(cursor, Some(next));
        assert_eq!(usage.feed_pages, 2);
        assert_eq!(usage.feed_events, 1);
        assert_eq!(usage.quiescent_observations, 1);
    }

    #[test]
    fn retained_change_gap_advances_to_observed_head_and_requires_refresh() {
        let workspace = WorkspaceIdV1::new(11).unwrap();
        let gap = WorkspaceChangeGapV1::new(workspace, 5, 1, 3, 7).unwrap();
        let transport = ChangeTransport {
            replies: VecDeque::from([ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(
                WorkspaceChangesV1::Gap(gap),
            ))]),
        };
        let mut client = vellum_sdk_client::client::Client::connect(transport).unwrap();
        let mut cursor = WorkspaceChangeCursorV1::new(workspace, 5, 1);
        let mut usage = RemoteSynchronizationUsage::default();

        assert!(poll_workspace_changes(&mut client, &mut cursor, &mut usage).unwrap());
        assert_eq!(cursor.unwrap().revision(), 7);
        assert_eq!(usage.feed_gaps, 1);
    }
}
