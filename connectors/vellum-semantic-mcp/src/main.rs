#![forbid(unsafe_code)]

use std::{ffi::OsString, process::ExitCode};

use serde_json::json;
use tokio::io::BufReader;
use vellum_semantic_client::{
    SemanticCapabilitySetV1, SemanticClientRefV1, SemanticProviderProvenanceV1,
    SemanticProviderRefV1, SemanticProviderStateV1, SemanticRequestRefV1, SemanticRequestV1,
    bind_semantic_client_v1, try_bind_semantic_session_query_v1,
};
use vellum_semantic_mcp::{
    SemanticMcpDocumentV1, SemanticMcpLimitsV1, SemanticMcpServerV1, SemanticMcpStdioFailureV1,
    SemanticMcpStdioLimitsV1, serve_stdio_v1,
};
use vellum_semantic_service::{
    SemanticClientRetirementStateV1, SemanticProviderAdmissionV1, SemanticProviderCompletionV1,
    SemanticProviderPortV1, SemanticProviderSubmissionV1, SemanticServiceLimitsV1,
    SemanticServiceV1, SemanticSnapshotUpdateV1,
};

const HELP: &str = "vellum-semantic-mcp 0.1.0\n\
Bounded stdio MCP peer for Vellum semantic queries.\n\n\
Real socket mode is a single-document, single-language rust-analyzer probe.\n\n\
USAGE:\n\
    vellum-semantic-mcp --semantic-socket PRIVATE_SOCKET --semantic-credential PRIVATE_CREDENTIAL --native-socket PRIVATE_SOCKET --native-credential PRIVATE_CREDENTIAL --document-language LANGUAGE [--metrics-stderr]\n\
    vellum-semantic-mcp --unavailable [--metrics-stderr]\n\
    vellum-semantic-mcp --probe\n\n\
OPTIONS:\n\
    --semantic-socket PRIVATE_SOCKET       Persistent semantic service endpoint.\n\
    --semantic-credential PRIVATE_FILE     Private semantic-service credential.\n\
    --native-socket PRIVATE_SOCKET         Native Vellum daemon endpoint.\n\
    --native-credential PRIVATE_FILE       Private native-daemon credential.\n\
    --document-language LANGUAGE           Explicit language for the sole synchronized document;\n\
                                           zero or multiple documents are refused.\n\
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
impl std::fmt::Display for RemoteStage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Credential => "credential",
            Self::SemanticTransport => "semantic_transport",
            Self::NativeTransport => "native_transport",
            Self::SessionDescribe => "session_describe",
            Self::SessionJoin => "session_join",
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
    fn try_new(language: String) -> Option<Self> {
        let mut seed = [0_u8; 16];
        getrandom::fill(&mut seed).ok()?;
        // Reserve half the u128 space for this bounded process lifetime so
        // checked monotone issuance cannot immediately exhaust at a random
        // value near the upper boundary.
        seed[0] &= 0x7f;
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
        let value = self.next;
        self.next = self.next.checked_add(1)?;
        Some(value)
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
) -> Result<(), RemoteStage> {
    use vellum_semantic_protocol::{
        SemanticExchangeDispositionV1, SemanticExchangeRefV1, SemanticSessionCommandV1,
        SemanticSessionOperationRefV1, SemanticSessionOutcomeV1, SemanticSessionRolesV1,
    };
    let disposition = session
        .attempt(SemanticSessionCommandV1::JoinSession {
            exchange: SemanticExchangeRefV1::new(2_u128.to_be_bytes())
                .ok_or(RemoteStage::SessionJoin)?,
            operation: SemanticSessionOperationRefV1::new(3_u128.to_be_bytes())
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
            _ => Err(RemoteStage::SessionJoin),
        },
        SemanticExchangeDispositionV1::NotSent | SemanticExchangeDispositionV1::OutcomeUnknown => {
            Err(RemoteStage::SessionJoin)
        }
    }
}

#[cfg(unix)]
fn leave_session<P: vellum_semantic_session_client::SemanticSessionExchangePortV1>(
    session: &mut vellum_semantic_session_client::SemanticSessionClientV1<P>,
    exchange: u128,
    operation: u128,
) -> Result<(), RemoteStage> {
    use vellum_semantic_protocol::{
        SemanticExchangeDispositionV1, SemanticExchangeRefV1, SemanticSessionCommandV1,
        SemanticSessionOperationRefV1, SemanticSessionOutcomeV1,
    };
    let binding = session.binding().ok_or(RemoteStage::Cleanup)?;
    let command = SemanticSessionCommandV1::LeaveSession {
        exchange: SemanticExchangeRefV1::new(exchange.to_be_bytes()).ok_or(RemoteStage::Cleanup)?,
        operation: SemanticSessionOperationRefV1::new(operation.to_be_bytes())
            .ok_or(RemoteStage::Cleanup)?,
        incarnation: binding.incarnation,
        membership: binding.membership,
    };
    match session.attempt(command).map_err(|_| RemoteStage::Cleanup)? {
        SemanticExchangeDispositionV1::Reply(reply)
            if matches!(reply.outcome(), SemanticSessionOutcomeV1::Accepted) =>
        {
            Ok(())
        }
        SemanticExchangeDispositionV1::Reply(_)
        | SemanticExchangeDispositionV1::NotSent
        | SemanticExchangeDispositionV1::OutcomeUnknown => Err(RemoteStage::Cleanup),
    }
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
fn run_remote(configuration: RemoteArguments) -> Result<(), RemoteStage> {
    use vellum_adapter_local_authentication::{
        LocalCredentialClaimProviderV1, LocalCredentialStoreV1,
    };
    use vellum_sdk_client::{
        client::Client,
        transport::{AuthenticatedUnixConnector, RemoteTransport, TransportLimits},
    };
    use vellum_semantic_protocol::SemanticSessionLimitsV1;
    use vellum_semantic_session_client::{
        SemanticSessionClientLimitsV1, SemanticSessionClientV1,
        VellumSnapshotSynchronizationCoordinatorV1, VellumSnapshotSynchronizationExecutionV1,
        synchronize_vellum_snapshots,
    };
    use vellum_semantic_transport::{
        SemanticRemoteSessionV1, SemanticTransportLimitsV1, UnixSemanticFrameDeadlinesV1,
        UnixSemanticFrameExchangeV1,
    };

    let mut store = LocalCredentialStoreV1::default();
    let semantic_record = store
        .load(&configuration.semantic_credential)
        .map_err(|_| RemoteStage::Credential)?;
    let native_record = store
        .load(&configuration.native_credential)
        .map_err(|_| RemoteStage::Credential)?;

    let exchange = UnixSemanticFrameExchangeV1::try_new_authenticated(
        configuration.semantic_socket,
        UnixSemanticFrameDeadlinesV1::reference(),
        semantic_authentication(semantic_record),
    )
    .map_err(|_| RemoteStage::SemanticTransport)?;
    let remote = SemanticRemoteSessionV1::try_new(exchange, SemanticTransportLimitsV1::reference())
        .map_err(|_| RemoteStage::SemanticTransport)?;
    let mut semantic =
        SemanticSessionClientV1::try_new(remote, SemanticSessionClientLimitsV1::reference())
            .map_err(|_| RemoteStage::SemanticTransport)?;
    let description = describe_session(&mut semantic, 1)?;
    join_session(&mut semantic, description)?;
    macro_rules! fail_joined {
        ($stage:expr) => {{
            let cleanup = leave_session(&mut semantic, u128::MAX - 1, u128::MAX);
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
    let mut composition =
        UniformSynchronizationComposition::try_new(configuration.language.clone())
            .ok_or(RemoteStage::Runtime)?;
    let first_synchronization = synchronize_vellum_snapshots(
        &mut native,
        &mut semantic,
        &mut coordinator,
        &mut composition,
        SemanticSessionLimitsV1::reference(),
    );
    match first_synchronization {
        VellumSnapshotSynchronizationExecutionV1::Synchronized => {}
        VellumSnapshotSynchronizationExecutionV1::Suspended(ref suspension) => {
            eprintln!(
                "vellum-semantic-mcp: synchronization_refused pass=initial result={suspension:?}"
            );
            let _ = native.disconnect();
            fail_joined!(RemoteStage::Synchronization);
        }
        VellumSnapshotSynchronizationExecutionV1::Refused(ref refusal) => {
            eprintln!(
                "vellum-semantic-mcp: synchronization_refused pass=initial result={refusal:?}"
            );
            let _ = native.disconnect();
            fail_joined!(RemoteStage::Synchronization);
        }
    }
    if coordinator.synchronized_documents().len() != 1 {
        let _ = native.disconnect();
        fail_joined!(RemoteStage::Catalog);
    }
    // Provider readiness may require bounded polling after the first complete
    // snapshot. Observing Indexing intentionally unseals the local projection,
    // so perform one exact second synchronization after readiness before
    // constructing the query-only sovereign port.
    if wait_for_ready(&mut semantic, &mut composition).is_err() {
        let _ = native.disconnect();
        fail_joined!(RemoteStage::Synchronization);
    }
    let final_synchronization = synchronize_vellum_snapshots(
        &mut native,
        &mut semantic,
        &mut coordinator,
        &mut composition,
        SemanticSessionLimitsV1::reference(),
    );
    match final_synchronization {
        VellumSnapshotSynchronizationExecutionV1::Synchronized => {}
        VellumSnapshotSynchronizationExecutionV1::Suspended(ref suspension) => {
            eprintln!(
                "vellum-semantic-mcp: synchronization_refused pass=final result={suspension:?}"
            );
            let _ = native.disconnect();
            fail_joined!(RemoteStage::Synchronization);
        }
        VellumSnapshotSynchronizationExecutionV1::Refused(ref refusal) => {
            eprintln!("vellum-semantic-mcp: synchronization_refused pass=final result={refusal:?}");
            let _ = native.disconnect();
            fail_joined!(RemoteStage::Synchronization);
        }
    }
    let final_tag = match composition.next_value() {
        Some(tag) => tag,
        None => {
            let _ = native.disconnect();
            fail_joined!(RemoteStage::Synchronization);
        }
    };
    let final_description = match describe_session(&mut semantic, final_tag) {
        Ok(description) => description,
        Err(_) => {
            let _ = native.disconnect();
            fail_joined!(RemoteStage::Synchronization);
        }
    };
    if !matches!(
        final_description.readiness(),
        vellum_semantic_protocol::SemanticSessionReadinessV1::Ready { .. }
    ) || !matches!(
        final_description.provider().readiness(),
        vellum_semantic_protocol::SemanticProviderReadinessProjectionV1::Ready { .. }
    ) {
        let _ = native.disconnect();
        fail_joined!(RemoteStage::Synchronization);
    }

    let limits = SemanticMcpLimitsV1::reference();
    let documents: Option<Vec<_>> = coordinator
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
        })
        .collect();
    let documents = match documents {
        Some(documents) => documents,
        None => {
            let _ = native.disconnect();
            fail_joined!(RemoteStage::Catalog);
        }
    };
    if native.disconnect().is_err() {
        fail_joined!(RemoteStage::NativeDisconnect);
    }
    let client = match try_bind_semantic_session_query_v1(semantic, composition.next) {
        Ok(client) => client,
        Err(failure) => {
            let mut semantic = failure.into_session();
            let cleanup = leave_session(&mut semantic, u128::MAX - 1, u128::MAX);
            return Err(cleanup.err().unwrap_or(RemoteStage::QueryBinding));
        }
    };
    let mut server = SemanticMcpServerV1::try_new_with_documents(client, limits, documents)
        .map_err(|_| RemoteStage::Mcp)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .map_err(|_| RemoteStage::Runtime)?;
    let mut input = BufReader::new(tokio::io::stdin());
    let mut output = tokio::io::stdout();
    let stdio_result = runtime.block_on(serve_stdio_v1(
        &mut server,
        &mut input,
        &mut output,
        SemanticMcpStdioLimitsV1::reference(),
    ));
    let usage = server.usage();
    let cleanup_result = (|| {
        let query = server.into_client().into_port();
        let mut extraction = query.try_into_session().map_err(|_| RemoteStage::Cleanup)?;
        let binding = extraction.session.binding().ok_or(RemoteStage::Cleanup)?;
        let leave = vellum_semantic_protocol::SemanticSessionCommandV1::LeaveSession {
            exchange: vellum_semantic_protocol::SemanticExchangeRefV1::new(
                extraction.next_exchange.to_be_bytes(),
            )
            .ok_or(RemoteStage::Cleanup)?,
            operation: vellum_semantic_protocol::SemanticSessionOperationRefV1::new(
                extraction.next_operation.to_be_bytes(),
            )
            .ok_or(RemoteStage::Cleanup)?,
            incarnation: binding.incarnation,
            membership: binding.membership,
        };
        match extraction
            .session
            .attempt(leave)
            .map_err(|_| RemoteStage::Cleanup)?
        {
            vellum_semantic_protocol::SemanticExchangeDispositionV1::Reply(reply)
                if matches!(
                    reply.outcome(),
                    vellum_semantic_protocol::SemanticSessionOutcomeV1::Accepted
                ) =>
            {
                Ok(())
            }
            vellum_semantic_protocol::SemanticExchangeDispositionV1::Reply(_)
            | vellum_semantic_protocol::SemanticExchangeDispositionV1::NotSent
            | vellum_semantic_protocol::SemanticExchangeDispositionV1::OutcomeUnknown => {
                Err(RemoteStage::Cleanup)
            }
        }
    })();
    cleanup_result?;
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
    use std::os::unix::ffi::OsStringExt as _;

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
        assert_ne!(first, 0);
        assert!(first < (1_u128 << 127));
        assert_eq!(second, first + 1);
    }
}
