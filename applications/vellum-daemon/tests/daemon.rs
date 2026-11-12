#![cfg(target_os = "macos")]

use std::{
    ffi::OsString,
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::Shutdown,
    os::unix::net::{UnixListener, UnixStream},
    os::unix::{
        ffi::OsStringExt,
        fs::{FileTypeExt, MetadataExt, PermissionsExt},
    },
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use vellum_adapter_local_authentication::{LocalCredentialClaimProviderV1, LocalCredentialStoreV1};
use vellum_adapter_local_fs::{
    FilesystemLimits, LocalFilesystemFactory, LocalRequestJournalV1,
    LocalWorkspaceIdentityMetadataV1,
};
use vellum_client::Client;
use vellum_daemon::{
    AuthenticatedUnixConnector, DaemonAuthenticationProfileV1, DaemonBuildError,
    DaemonExportPolicyV1, DaemonHealth, DaemonLifecycle, DaemonLimits, DaemonLiveness,
    DaemonReadiness, DaemonServeError, DrainCompletion, DrainTransition,
    LocalAuthenticationClaimV1, LocalDaemon, OperationalEvent, OperationalEventKind,
    OperationalEventSink, OperationalEventSinkError, OperationalLifecycleEvent,
    OperationalRequestEvent, RequestLatencyProfile, ShutdownControlError, StartupWorkspaceError,
    compose_startup_workspace, request_shutdown, request_status, request_status_v2,
};
use vellum_editor::{Workspace, WorkspaceRootKind};
use vellum_host::request_journal::{
    DurableOperationRecoveryV1, DurableOperationRetirementRecoveryV1, RequestJournalCompletionV1,
    RequestJournalIntentV1, RequestJournalLimitsV1, RequestJournalPortV1, RequestJournalRecoveryV1,
    RequestJournalRefusalV1, RequestJournalScopeV1,
};
use vellum_host::workspace::WorkspaceIdentityLimitsV1;
use vellum_host::workspace::{
    WorkspaceIdentityMetadataPortV1, WorkspaceIdentityMetadataV1,
    WorkspaceIdentityRecoveryRefusalV1, WorkspaceIdentityRecoveryV1,
};
use vellum_host::{
    authentication::{
        AuthenticationDecisionV1, AuthenticationEvidenceV1, AuthenticationPortV1,
        AuthenticationRequestV1,
    },
    authority::{AuthorityActionSetV1, AuthorityActionV1, AuthoritySubjectRefV1},
};
use vellum_protocol::{
    CommandV1, DurableEffectIntentV1, DurableExecuteRequestV1, DurableOperationRefV1,
    DurableReceiptLookupOutcomeV1, DurableReceiptLookupV1, OutcomeV1, ReplyV1, RequestId,
    RequestV1, ServicePort,
};
use vellum_service::ServiceLimits;
use vellum_transport::{
    FrameExchange, RemoteTransport, TransportError, TransportLimits, UnixConnector,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct LoseDurableExecuteReply<X> {
    inner: X,
}

impl<X: FrameExchange> FrameExchange for LoseDurableExecuteReply<X> {
    fn exchange(
        &mut self,
        request: &[u8],
        max_reply_bytes: usize,
    ) -> Result<Vec<u8>, TransportError> {
        let lose = matches!(
            vellum_wire::decode_request_v1(request, vellum_wire::WireLimits::reference()),
            Ok(vellum_wire::WireRequestV1::DurableExecute(_))
        );
        let reply = self.inner.exchange(request, max_reply_bytes)?;
        if lose {
            Err(TransportError::OutcomeUnknown)
        } else {
            Ok(reply)
        }
    }
}

#[derive(Clone, Copy)]
enum AuthenticationBehavior {
    Accept,
    Reject,
    Unavailable,
    Transplant,
}

struct ScriptedAuthenticationPort;

struct SubjectAuthenticationPort;

impl AuthenticationPortV1 for SubjectAuthenticationPort {
    type Context = ([u8; 16], [u8; 16]);

    fn authenticate(
        &mut self,
        request: AuthenticationRequestV1,
        context: &Self::Context,
    ) -> AuthenticationDecisionV1 {
        if request.claim != vellum_host::authentication::AuthenticationClaimRefV1::new(context.0) {
            return AuthenticationDecisionV1::Rejected;
        }
        AuthenticationDecisionV1::Authenticated(AuthenticationEvidenceV1::from_port(
            request,
            AuthoritySubjectRefV1::new(context.1),
        ))
    }
}

impl AuthenticationPortV1 for ScriptedAuthenticationPort {
    type Context = (AuthenticationBehavior, [u8; 16]);

    fn authenticate(
        &mut self,
        request: AuthenticationRequestV1,
        context: &Self::Context,
    ) -> AuthenticationDecisionV1 {
        if request.claim != vellum_host::authentication::AuthenticationClaimRefV1::new(context.1) {
            return AuthenticationDecisionV1::Rejected;
        }
        match context.0 {
            AuthenticationBehavior::Accept => {
                AuthenticationDecisionV1::Authenticated(AuthenticationEvidenceV1::from_port(
                    request,
                    AuthoritySubjectRefV1::new([0x51; 16]),
                ))
            }
            AuthenticationBehavior::Reject => AuthenticationDecisionV1::Rejected,
            AuthenticationBehavior::Unavailable => AuthenticationDecisionV1::Unavailable,
            AuthenticationBehavior::Transplant => {
                let transplanted = AuthenticationRequestV1 {
                    challenge: vellum_host::authentication::AuthenticationChallengeRefV1::new(
                        [0x77; 16],
                    ),
                    ..request
                };
                AuthenticationDecisionV1::Authenticated(AuthenticationEvidenceV1::from_port(
                    transplanted,
                    AuthoritySubjectRefV1::new([0x51; 16]),
                ))
            }
        }
    }
}

struct RecordingAdversarialSink(Arc<Mutex<Vec<OperationalEvent>>>);

impl OperationalEventSink for RecordingAdversarialSink {
    fn emit(&mut self, event: OperationalEvent) -> Result<(), OperationalEventSinkError> {
        self.0.lock().unwrap().push(event);
        if event.sequence.is_multiple_of(2) {
            Err(OperationalEventSinkError)
        } else {
            panic!("adversarial operational sink")
        }
    }
}

struct Fixture {
    directory: PathBuf,
    source: PathBuf,
    socket: PathBuf,
    control: PathBuf,
    lock: PathBuf,
}

impl Fixture {
    fn new(source: &str) -> Self {
        let directory = std::env::temp_dir().join(format!(
            "vellum-daemon-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let source_path = directory.join("source.txt");
        fs::write(&source_path, source).unwrap();
        let socket = directory.join("service.sock");
        let control = directory.join("control.sock");
        let lock = directory.join(".vellum-daemon.lock");
        Self {
            directory,
            source: source_path,
            socket,
            control,
            lock,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket);
        let _ = fs::remove_file(&self.control);
        let _ = fs::remove_file(&self.lock);
        let _ = fs::remove_file(self.directory.join("workspace-identity-v1"));
        let _ = fs::remove_file(self.directory.join(".workspace-identity-v1.stage"));
        let _ = fs::remove_dir_all(self.directory.join(".vellum-request-journal-v1"));
        let _ = fs::remove_dir_all(self.directory.join(".vellum-durable-operation-v1"));
        let _ = fs::remove_dir_all(self.directory.join(".vellum-durable-operation-v2"));
        let _ = fs::remove_file(&self.source);
        let _ = fs::remove_dir(&self.directory);
    }
}

#[derive(Clone)]
struct ScriptedIdentityMetadata {
    load: WorkspaceIdentityRecoveryV1,
    store: Result<(), WorkspaceIdentityRecoveryRefusalV1>,
}

impl WorkspaceIdentityMetadataPortV1 for ScriptedIdentityMetadata {
    fn load(
        &mut self,
        _: &std::path::Path,
        _: WorkspaceIdentityLimitsV1,
    ) -> WorkspaceIdentityRecoveryV1 {
        self.load.clone()
    }

    fn store(
        &mut self,
        _: &std::path::Path,
        _: &WorkspaceIdentityMetadataV1,
        _: WorkspaceIdentityLimitsV1,
    ) -> Result<(), WorkspaceIdentityRecoveryRefusalV1> {
        self.store
    }
}

struct RuntimeFailingIdentityMetadata {
    stores: Arc<AtomicU64>,
    runtime: WorkspaceIdentityRecoveryRefusalV1,
}

#[derive(Clone, Copy)]
enum JournalFailurePoint {
    Begin,
    Complete,
}

struct FailingRequestJournal(JournalFailurePoint);

impl RequestJournalPortV1 for FailingRequestJournal {
    fn recover(
        &mut self,
        _: &std::path::Path,
        _: RequestJournalScopeV1,
        _: RequestJournalLimitsV1,
    ) -> RequestJournalRecoveryV1 {
        RequestJournalRecoveryV1::Empty
    }

    fn recover_operations(
        &mut self,
        _: &std::path::Path,
        _: RequestJournalScopeV1,
        _: RequestJournalLimitsV1,
    ) -> DurableOperationRecoveryV1 {
        DurableOperationRecoveryV1::Empty
    }

    fn recover_operation_retirements(
        &mut self,
        _: &std::path::Path,
        _: RequestJournalScopeV1,
        _: RequestJournalLimitsV1,
    ) -> DurableOperationRetirementRecoveryV1 {
        DurableOperationRetirementRecoveryV1::Empty
    }

    fn begin(
        &mut self,
        _: &std::path::Path,
        _: RequestJournalIntentV1<'_>,
        _: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1> {
        match self.0 {
            JournalFailurePoint::Begin => Err(RequestJournalRefusalV1::OutcomeUnknown),
            JournalFailurePoint::Complete => Ok(()),
        }
    }

    fn complete(
        &mut self,
        _: &std::path::Path,
        _: RequestJournalCompletionV1<'_>,
        _: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1> {
        Err(RequestJournalRefusalV1::OutcomeUnknown)
    }
}

impl WorkspaceIdentityMetadataPortV1 for RuntimeFailingIdentityMetadata {
    fn load(
        &mut self,
        _: &std::path::Path,
        _: WorkspaceIdentityLimitsV1,
    ) -> WorkspaceIdentityRecoveryV1 {
        WorkspaceIdentityRecoveryV1::Refused(WorkspaceIdentityRecoveryRefusalV1::Missing)
    }

    fn store(
        &mut self,
        _: &std::path::Path,
        _: &WorkspaceIdentityMetadataV1,
        _: WorkspaceIdentityLimitsV1,
    ) -> Result<(), WorkspaceIdentityRecoveryRefusalV1> {
        if self.stores.fetch_add(1, Ordering::Relaxed) == 0 {
            Ok(())
        } else {
            Err(self.runtime)
        }
    }
}

struct DirectoryFixture {
    directory: PathBuf,
    files: Vec<PathBuf>,
}

impl DirectoryFixture {
    fn new(entries: &[(&str, &str)]) -> Self {
        let directory = std::env::temp_dir().join(format!(
            "vellum-daemon-directory-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        let files = entries
            .iter()
            .map(|(name, content)| {
                let path = directory.join(name);
                fs::write(&path, content).unwrap();
                path
            })
            .collect();
        Self { directory, files }
    }
}

impl Drop for DirectoryFixture {
    fn drop(&mut self) {
        for file in &self.files {
            let _ = fs::remove_file(file);
        }
        let _ = fs::remove_dir(&self.directory);
    }
}

fn profile() -> vellum_editor::WorkspaceConfig {
    vellum_editor::reference_workspace_profile(
        LocalFilesystemFactory::new(FilesystemLimits {
            max_resources: 16,
            max_bytes: 4096,
            max_stage_attempts: 8,
            max_metadata_bytes: 4096,
        })
        .erase(),
    )
}

fn workspace(path: &PathBuf) -> Workspace {
    Workspace::open(std::slice::from_ref(path), profile()).unwrap()
}

fn limits() -> DaemonLimits {
    DaemonLimits {
        service: ServiceLimits::reference(),
        transport: TransportLimits::reference(),
        io_timeout: Duration::from_secs(2),
        max_in_flight_connections: 1,
        max_startup_roots: 16,
        max_authentication_attempts: 32,
        workspace_identity: WorkspaceIdentityLimitsV1 {
            max_documents: 256,
            max_roots: 256,
            max_encoded_bytes: 1024,
        },
        request_journal: vellum_host::request_journal::RequestJournalLimitsV1 {
            max_entries: 256,
            max_retired_operations: 4096,
            max_request_bytes: 65_600,
            max_reply_bytes: 65_600,
            max_encoded_bytes: 131_264,
        },
        request_latency: Some(RequestLatencyProfile::reference()),
    }
}

#[test]
fn required_authentication_is_exact_per_exchange_and_closes_on_disconnect() {
    let fixture = Fixture::new("alpha");
    let claim = [0x31; 16];
    let mut daemon = LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), limits())
        .unwrap()
        .require_authentication(
            ScriptedAuthenticationPort,
            (AuthenticationBehavior::Accept, claim),
            3,
        );
    assert_eq!(
        daemon.authentication_profile(),
        DaemonAuthenticationProfileV1::Required
    );
    let socket = fixture.socket.clone();
    let client = std::thread::spawn(move || {
        let connector = AuthenticatedUnixConnector::with_timeout(
            socket,
            LocalAuthenticationClaimV1::new(claim),
            Duration::from_secs(2),
        )
        .unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let mut client = Client::connect(transport).unwrap();
        assert!(matches!(
            client.snapshot().unwrap(),
            ReplyV1::Accepted(OutcomeV1::Snapshot(_))
        ));
        client.disconnect().map_err(|(_, error)| error).unwrap();
    });
    for _ in 0..3 {
        daemon.serve_next().unwrap();
    }
    client.join().unwrap();
    let usage = daemon.usage().authentication;
    assert_eq!(usage.handshakes_started, 3);
    assert_eq!(usage.handshakes_completed, 3);
    assert_eq!(usage.sessions_issued, 3);
    assert_eq!(usage.sessions_closed, 3);
    assert_eq!(usage.active_sessions, 0);
    assert_eq!(daemon.service_usage().active_clients, 0);
}

#[test]
fn rejection_unavailability_and_transplanted_evidence_share_one_external_refusal() {
    for behavior in [
        AuthenticationBehavior::Reject,
        AuthenticationBehavior::Unavailable,
        AuthenticationBehavior::Transplant,
    ] {
        let fixture = Fixture::new("alpha");
        let claim = [0x32; 16];
        let mut daemon = LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), limits())
            .unwrap()
            .require_authentication(ScriptedAuthenticationPort, (behavior, claim), 1);
        let socket = fixture.socket.clone();
        let client = std::thread::spawn(move || {
            let connector = AuthenticatedUnixConnector::with_timeout(
                socket,
                LocalAuthenticationClaimV1::new(claim),
                Duration::from_secs(2),
            )
            .unwrap();
            let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
            Client::connect(transport).err().expect("must refuse")
        });
        assert_eq!(daemon.serve_next(), Err(DaemonServeError::Authentication));
        let _ = client.join().unwrap();
        let usage = daemon.usage().authentication;
        assert_eq!(usage.handshakes_started, 1);
        assert_eq!(usage.handshakes_completed, 0);
        assert_eq!(usage.handshakes_refused, 1);
        assert_eq!(usage.active_sessions, 0);
        assert_eq!(daemon.service_usage().active_clients, 0);
    }
}

#[test]
fn replayed_handshake_and_disconnect_after_authentication_fail_closed() {
    let fixture = Fixture::new("alpha");
    let claim = [0x33; 16];
    let mut daemon = LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), limits())
        .unwrap()
        .require_authentication(
            ScriptedAuthenticationPort,
            (AuthenticationBehavior::Accept, claim),
            2,
        );

    let first_socket = fixture.socket.clone();
    let first = std::thread::spawn(move || {
        let mut stream = UnixStream::connect(first_socket).unwrap();
        let mut hello = [0_u8; 33];
        stream.read_exact(&mut hello).unwrap();
        let mut response = [0_u8; 49];
        response[0] = 1;
        response[1..33].copy_from_slice(&hello[1..33]);
        response[33..].copy_from_slice(&claim);
        stream.write_all(&response).unwrap();
        let mut result = [0_u8; 2];
        stream.read_exact(&mut result).unwrap();
        assert_eq!(result, [1, 1]);
        response
    });
    assert_eq!(
        daemon.serve_next(),
        Err(DaemonServeError::Request(
            vellum_transport::TransportError::Truncated
        ))
    );
    let replay = first.join().unwrap();
    assert_eq!(daemon.usage().authentication.sessions_closed, 1);

    let replay_socket = fixture.socket.clone();
    let second = std::thread::spawn(move || {
        let mut stream = UnixStream::connect(replay_socket).unwrap();
        let mut hello = [0_u8; 33];
        stream.read_exact(&mut hello).unwrap();
        stream.write_all(&replay).unwrap();
        let mut result = [0_u8; 2];
        stream.read_exact(&mut result).unwrap();
        result
    });
    assert_eq!(daemon.serve_next(), Err(DaemonServeError::Authentication));
    assert_eq!(second.join().unwrap(), [1, 0]);
    let usage = daemon.usage().authentication;
    assert_eq!(usage.handshakes_completed, 1);
    assert_eq!(usage.handshakes_refused, 1);
    assert_eq!(usage.active_sessions, 0);
}

#[test]
fn disconnect_after_provider_acceptance_never_leaks_an_active_session() {
    let fixture = Fixture::new("alpha");
    let claim = [0x34; 16];
    let mut daemon = LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), limits())
        .unwrap()
        .require_authentication(
            ScriptedAuthenticationPort,
            (AuthenticationBehavior::Accept, claim),
            1,
        );
    let socket = fixture.socket.clone();
    let peer = std::thread::spawn(move || {
        let mut stream = UnixStream::connect(socket).unwrap();
        let mut hello = [0_u8; 33];
        stream.read_exact(&mut hello).unwrap();
        let mut response = [0_u8; 49];
        response[0] = 1;
        response[1..33].copy_from_slice(&hello[1..33]);
        response[33..].copy_from_slice(&claim);
        stream.write_all(&response).unwrap();
        stream.shutdown(Shutdown::Both).unwrap();
    });
    let outcome = daemon.serve_next();
    peer.join().unwrap();
    let usage = daemon.usage().authentication;
    assert_eq!(usage.active_sessions, 0);
    match outcome {
        // The peer's shutdown can reach the server before its two-byte
        // acknowledgement is accepted. No session was then issued.
        Err(DaemonServeError::Authentication) => {
            assert_eq!(usage.handshakes_completed, 0);
            assert_eq!(usage.sessions_issued, 0);
            assert_eq!(usage.sessions_closed, 0);
            assert_eq!(usage.handshake_io_failures, 1);
        }
        // A successful local write does not prove that the peer consumed the
        // acknowledgement. If it wins the race, authentication completes,
        // frame decoding observes the disconnect, and the session closes.
        Err(DaemonServeError::Request(TransportError::Truncated)) => {
            assert_eq!(usage.handshakes_completed, 1);
            assert_eq!(usage.sessions_issued, 1);
            assert_eq!(usage.sessions_closed, 1);
            assert_eq!(usage.handshake_io_failures, 0);
        }
        other => panic!("unexpected disconnect outcome: {other:?}"),
    }
}

#[test]
fn unix_daemon_serves_remote_snapshot_and_reclaims_the_client() {
    let fixture = Fixture::new("alpha");
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut daemon = LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), limits())
        .unwrap()
        .with_event_sink(RecordingAdversarialSink(Arc::clone(&events)));
    assert_eq!(
        daemon.authentication_profile(),
        DaemonAuthenticationProfileV1::OwnerOnlyOffline
    );
    assert_eq!(
        fs::symlink_metadata(&fixture.socket)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let socket_metadata = fs::symlink_metadata(&fixture.socket).unwrap();
    assert!(socket_metadata.file_type().is_socket());
    assert_eq!(socket_metadata.uid(), rustix::process::geteuid().as_raw());
    assert_eq!(socket_metadata.nlink(), 1);
    let socket = fixture.socket.clone();
    let client = std::thread::spawn(move || {
        let connector = UnixConnector::with_timeout(socket, Duration::from_secs(2)).unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let mut client = Client::connect(transport).unwrap();
        let ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot)) = client.snapshot().unwrap() else {
            panic!("expected snapshot")
        };
        assert_eq!(&*snapshot.text, "alpha");
        if let Err((_, error)) = client.disconnect() {
            panic!("disconnect failed: {error:?}");
        }
    });

    for _ in 0..3 {
        daemon.serve_next().unwrap();
    }
    client.join().unwrap();
    assert_eq!(daemon.usage().accepted_connections, 3);
    assert_eq!(daemon.usage().completed_connections, 3);
    assert_eq!(daemon.service_usage().active_clients, 0);
    assert_eq!(daemon.operational_snapshot().events.failed, 4);
    assert_eq!(daemon.operational_snapshot().events.panicked, 3);
    let recorded = events.lock().unwrap();
    assert_eq!(
        recorded.first().map(|event| event.kind),
        Some(OperationalEventKind::Lifecycle(
            OperationalLifecycleEvent::Ready
        ))
    );
    assert_eq!(
        recorded
            .iter()
            .filter(|event| {
                event.kind == OperationalEventKind::Request(OperationalRequestEvent::Accepted)
            })
            .count(),
        3
    );
    assert_eq!(
        recorded
            .iter()
            .filter(|event| {
                event.kind == OperationalEventKind::Request(OperationalRequestEvent::Completed)
            })
            .count(),
        3
    );
    for request in recorded.iter().skip(1) {
        assert!(request.correlation.is_some());
    }
    drop(recorded);
    drop(daemon);
    assert!(!fixture.socket.exists());
}

#[test]
fn startup_composition_accepts_empty_file_and_directory_roots_without_paths_in_inventory() {
    let empty = compose_startup_workspace(&[], profile(), limits()).unwrap();
    let empty_inventory = empty.inventory().unwrap();
    assert!(empty_inventory.documents.is_empty());
    assert!(empty_inventory.roots.is_empty());

    let file = Fixture::new("explicit");
    let directory = DirectoryFixture::new(&[("b.txt", "second"), ("a.txt", "first")]);
    let workspace = compose_startup_workspace(
        &[file.source.clone(), directory.directory.clone()],
        profile(),
        limits(),
    )
    .unwrap();
    let inventory = workspace.inventory().unwrap();
    assert_eq!(inventory.documents.len(), 3);
    assert_eq!(inventory.roots.len(), 2);
    assert_eq!(inventory.roots[0].kind, WorkspaceRootKind::File);
    assert_eq!(inventory.roots[1].kind, WorkspaceRootKind::Directory);
    assert!(!format!("{inventory:?}").contains(file.directory.to_str().unwrap()));

    let link = file.directory.join("source-link");
    std::os::unix::fs::symlink(&file.source, &link).unwrap();
    assert!(matches!(
        compose_startup_workspace(std::slice::from_ref(&link), profile(), limits()),
        Err(StartupWorkspaceError::UnsupportedResource)
    ));
    fs::remove_file(link).unwrap();
}

fn inventory_from_daemon(
    daemon: &mut LocalDaemon,
    socket: PathBuf,
) -> vellum_protocol::WorkspaceInventoryV1 {
    let client = std::thread::spawn(move || {
        let connector = UnixConnector::with_timeout(socket, Duration::from_secs(2)).unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let mut client = Client::connect(transport).unwrap();
        let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(inventory)) =
            client.workspace_inventory().unwrap()
        else {
            panic!("expected inventory")
        };
        if let Err((_, error)) = client.disconnect() {
            panic!("disconnect failed: {error:?}");
        }
        inventory
    });
    for _ in 0..3 {
        daemon.serve_next().unwrap();
    }
    client.join().unwrap()
}

#[test]
fn persistent_identity_commits_before_publication_and_rebinds_exact_resources() {
    let fixture = Fixture::new("alpha");
    let mut first = LocalDaemon::bind_with_persistent_identity(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
    )
    .unwrap();
    let first_inventory = inventory_from_daemon(&mut first, fixture.socket.clone());
    let first_usage = first.operational_snapshot().workspace_identity.unwrap();
    assert!(first_usage.initial);
    assert!(!first_usage.recovered);
    assert_eq!(first_usage.committed, 1);
    assert!(!first_usage.runtime_membership_mutation_disabled);
    assert!(fixture.directory.join("workspace-identity-v1").is_file());
    drop(first);
    assert!(!fixture.socket.exists());

    let mut restarted = LocalDaemon::bind_with_persistent_identity(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
    )
    .unwrap();
    let restarted_inventory = inventory_from_daemon(&mut restarted, fixture.socket.clone());
    let restarted_usage = restarted.operational_snapshot().workspace_identity.unwrap();
    assert!(!restarted_usage.initial);
    assert!(restarted_usage.recovered);
    assert_eq!(restarted_inventory.workspace, first_inventory.workspace);
    assert_eq!(restarted_inventory.documents, first_inventory.documents);
    assert_eq!(restarted_inventory.roots, first_inventory.roots);

    drop(restarted);
    fs::remove_file(fixture.directory.join("workspace-identity-v1")).unwrap();
    let Err(missing_refusal) = LocalDaemon::bind_with_persistent_identity(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
    ) else {
        panic!("lost identity after first installation must refuse")
    };
    assert_eq!(
        missing_refusal,
        DaemonBuildError::WorkspaceIdentity(WorkspaceIdentityRecoveryRefusalV1::Missing)
    );
    assert!(!fixture.socket.exists());
}

#[test]
fn ordinary_save_replacement_preserves_durable_binding_across_restart() {
    let fixture = Fixture::new("alpha");
    let edit_and_save = AuthorityActionSetV1::read_only()
        .join(AuthorityActionSetV1::singleton(
            AuthorityActionV1::EditSource,
        ))
        .join(AuthorityActionSetV1::singleton(
            AuthorityActionV1::PersistSource,
        ));
    let mut first = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        edit_and_save,
    )
    .unwrap();
    let socket = fixture.socket.clone();
    let client = std::thread::spawn(move || {
        let connector = UnixConnector::with_timeout(socket, Duration::from_secs(2)).unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let mut client = Client::connect(transport).unwrap();
        let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(inventory)) =
            client.workspace_inventory().unwrap()
        else {
            panic!("expected inventory")
        };
        let ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot)) = client.snapshot().unwrap() else {
            panic!("expected snapshot")
        };
        let ReplyV1::Accepted(OutcomeV1::Replaced(replaced)) = client
            .replace(snapshot.witness, 0, 5, "beta".into())
            .unwrap()
        else {
            panic!("expected replacement")
        };
        let ReplyV1::Accepted(OutcomeV1::Saved(receipt)) = client.save(replaced.witness).unwrap()
        else {
            panic!("expected save")
        };
        assert!(receipt.baseline_published);
        assert!(client.disconnect().is_ok());
        inventory
    });
    for _ in 0..6 {
        first.serve_next().unwrap();
    }
    let before = client.join().unwrap();
    drop(first);
    assert_eq!(fs::read_to_string(&fixture.source).unwrap(), "beta");

    let mut restarted = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        edit_and_save,
    )
    .unwrap();
    let after = inventory_from_daemon(&mut restarted, fixture.socket.clone());
    assert_eq!(after.documents, before.documents);
    assert_eq!(after.roots, before.roots);
}

#[test]
fn persistent_composition_retains_a_checkpoint_port_for_every_membership_ceiling() {
    for action in [
        AuthorityActionV1::AdmitFile,
        AuthorityActionV1::DiscoverDirectory,
        AuthorityActionV1::RetireMembership,
    ] {
        let fixture = Fixture::new("alpha");
        let actions =
            AuthorityActionSetV1::read_only().join(AuthorityActionSetV1::singleton(action));
        let port = ScriptedIdentityMetadata {
            load: WorkspaceIdentityRecoveryV1::Refused(WorkspaceIdentityRecoveryRefusalV1::Missing),
            store: Ok(()),
        };
        let daemon = LocalDaemon::bind_with_workspace_identity_metadata_and_action_ceiling(
            &fixture.socket,
            workspace(&fixture.source),
            limits(),
            actions,
            port,
        )
        .unwrap();
        assert!(
            !daemon
                .operational_snapshot()
                .workspace_identity
                .unwrap()
                .runtime_membership_mutation_disabled
        );
        assert!(fixture.socket.exists());
    }
}

#[test]
fn accepted_runtime_file_admission_is_checkpointed_before_reply_and_survives_restart() {
    let fixture = Fixture::new("alpha");
    let actions = AuthorityActionSetV1::read_only().join(AuthorityActionSetV1::singleton(
        AuthorityActionV1::AdmitFile,
    ));
    let mut first = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        Workspace::empty(profile()).unwrap(),
        limits(),
        actions,
    )
    .unwrap();
    let socket = fixture.socket.clone();
    let locator = fixture.source.to_str().unwrap().to_owned();
    let client = std::thread::spawn(move || {
        let connector = UnixConnector::with_timeout(socket, Duration::from_secs(2)).unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let mut client = Client::connect(transport).unwrap();
        let ReplyV1::Accepted(OutcomeV1::FileAdmitted(receipt)) = client.add_file(locator).unwrap()
        else {
            panic!("expected admitted file")
        };
        if let Err((_, error)) = client.disconnect() {
            panic!("disconnect failed: {error:?}");
        }
        receipt
    });
    for _ in 0..3 {
        first.serve_next().unwrap();
    }
    let admitted = client.join().unwrap();
    let usage = first.operational_snapshot().workspace_identity.unwrap();
    assert_eq!(usage.store_attempts, 2);
    assert_eq!(usage.committed, 2);
    assert_eq!(usage.store_refused, 0);
    assert_eq!(usage.outcome_unknown, 0);
    assert!(
        !fixture
            .directory
            .join(".vellum-request-journal-v1")
            .exists(),
        "owner-only endpoint access must not create durable principal receipts"
    );
    drop(first);

    let mut restarted = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions,
    )
    .unwrap();
    let recovered = inventory_from_daemon(&mut restarted, fixture.socket.clone());
    assert_eq!(recovered.documents, vec![admitted.document]);
    assert_eq!(recovered.roots.len(), 1);
    assert_eq!(recovered.roots[0].id, admitted.root.unwrap());
}

#[test]
fn runtime_checkpoint_refusal_or_unknown_fences_without_an_accepted_reply() {
    for refusal in [
        WorkspaceIdentityRecoveryRefusalV1::Conflict,
        WorkspaceIdentityRecoveryRefusalV1::OutcomeUnknown,
    ] {
        let fixture = Fixture::new("alpha");
        let actions = AuthorityActionSetV1::read_only().join(AuthorityActionSetV1::singleton(
            AuthorityActionV1::AdmitFile,
        ));
        let stores = Arc::new(AtomicU64::new(0));
        let mut daemon = LocalDaemon::bind_with_workspace_identity_metadata_and_action_ceiling(
            &fixture.socket,
            Workspace::empty(profile()).unwrap(),
            limits(),
            actions,
            RuntimeFailingIdentityMetadata {
                stores: Arc::clone(&stores),
                runtime: refusal,
            },
        )
        .unwrap();
        let socket = fixture.socket.clone();
        let locator = fixture.source.to_str().unwrap().to_owned();
        let client = std::thread::spawn(move || {
            let connector = UnixConnector::with_timeout(socket, Duration::from_secs(2)).unwrap();
            let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
            let mut client = Client::connect(transport).unwrap();
            client.add_file(locator)
        });
        daemon.serve_next().unwrap();
        assert!(matches!(
            daemon.serve_next(),
            Err(DaemonServeError::Request(
                TransportError::ServiceUnavailable
            ))
        ));
        assert!(matches!(
            client.join().unwrap(),
            Err(vellum_protocol::ServiceAccessErrorV1::OutcomeUnknown)
        ));
        assert_eq!(stores.load(Ordering::Relaxed), 2);
        let snapshot = daemon.operational_snapshot();
        assert_eq!(snapshot.status.lifecycle, DaemonLifecycle::Fenced);
        assert_eq!(snapshot.status.readiness, DaemonReadiness::NotReady);
        assert_eq!(snapshot.daemon.health_failures, 1);
        let identity = snapshot.workspace_identity.unwrap();
        assert_eq!(identity.store_attempts, 2);
        assert_eq!(identity.committed, 1);
        match refusal {
            WorkspaceIdentityRecoveryRefusalV1::OutcomeUnknown => {
                assert_eq!(identity.outcome_unknown, 1);
                assert_eq!(identity.store_refused, 0);
            }
            _ => {
                assert_eq!(identity.store_refused, 1);
                assert_eq!(identity.outcome_unknown, 0);
            }
        }
    }
}

#[test]
fn exact_membership_replay_reuses_the_committed_reply_without_a_duplicate_checkpoint() {
    let fixture = Fixture::new("alpha");
    let actions = AuthorityActionSetV1::read_only().join(AuthorityActionSetV1::singleton(
        AuthorityActionV1::AdmitFile,
    ));
    let mut daemon = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        Workspace::empty(profile()).unwrap(),
        limits(),
        actions,
    )
    .unwrap();
    let socket = fixture.socket.clone();
    let locator = fixture.source.to_str().unwrap().to_owned();
    let client = std::thread::spawn(move || {
        let connector = UnixConnector::with_timeout(socket, Duration::from_secs(2)).unwrap();
        let mut transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let client = transport.connect().unwrap();
        let request = RequestV1 {
            client,
            request: RequestId::new(1).unwrap(),
            command: CommandV1::AddFile { locator },
        };
        let first = transport.execute(request.clone()).unwrap();
        let replay = transport.execute(request).unwrap();
        transport.disconnect(client).unwrap();
        (first, replay)
    });
    for _ in 0..4 {
        daemon.serve_next().unwrap();
    }
    let (first, replay) = client.join().unwrap();
    assert_eq!(replay, first);
    let identity = daemon.operational_snapshot().workspace_identity.unwrap();
    assert_eq!(identity.store_attempts, 2);
    assert_eq!(identity.committed, 2);
}

#[test]
fn authenticated_replace_and_save_receipt_replays_after_restart_without_repeating_write() {
    let fixture = Fixture::new("alpha");
    let claim = [0x41; 16];
    let subject = [0x61; 16];
    let actions = AuthorityActionSetV1::read_only()
        .join(AuthorityActionSetV1::singleton(
            AuthorityActionV1::EditSource,
        ))
        .join(AuthorityActionSetV1::singleton(
            AuthorityActionV1::PersistSource,
        ));
    let mut first = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions,
    )
    .unwrap()
    .require_authentication(SubjectAuthenticationPort, (claim, subject), 3);
    let socket = fixture.socket.clone();
    let operation = std::thread::spawn(move || {
        let connector = AuthenticatedUnixConnector::with_timeout(
            socket,
            LocalAuthenticationClaimV1::new(claim),
            Duration::from_secs(2),
        )
        .unwrap();
        let mut transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let client = transport.connect().unwrap();
        let ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot)) = transport
            .execute(RequestV1 {
                client,
                request: RequestId::new(1).unwrap(),
                command: CommandV1::Snapshot,
            })
            .unwrap()
        else {
            panic!("expected snapshot")
        };
        let request = RequestV1 {
            client,
            request: RequestId::new(2).unwrap(),
            command: CommandV1::ReplaceAndSave {
                expected: snapshot.witness,
                start: 0,
                end: 5,
                inserted: "omega".to_owned(),
            },
        };
        let reply = transport.execute(request.clone()).unwrap();
        (client, request, reply)
    });
    for _ in 0..3 {
        first.serve_next().unwrap();
    }
    let (historical_client, request, original_reply) = operation.join().unwrap();
    assert_eq!(fs::read_to_string(&fixture.source).unwrap(), "omega");
    assert_eq!(first.usage().durable_intents_started, 1);
    assert_eq!(first.usage().durable_receipts_completed, 1);
    let receipt_utilization = first.operational_snapshot();
    assert_eq!(receipt_utilization.utilization.durable_receipts, 1);
    assert_eq!(
        receipt_utilization.utilization.durable_receipt_capacity,
        limits().request_journal.max_entries
    );
    assert!(!receipt_utilization.saturation.durable_receipts);
    drop(first);

    // If recovery accidentally re-executes the old save, this external value
    // is overwritten. Historical retrieval must return only the frozen reply.
    fs::write(&fixture.source, "external").unwrap();
    let mut other_subject = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions,
    )
    .unwrap()
    .require_authentication(SubjectAuthenticationPort, (claim, [0x62; 16]), 2);
    let socket = fixture.socket.clone();
    let hidden_request = request.clone();
    let hidden = std::thread::spawn(move || {
        let connector = AuthenticatedUnixConnector::with_timeout(
            socket,
            LocalAuthenticationClaimV1::new(claim),
            Duration::from_secs(2),
        )
        .unwrap();
        let mut transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let retained = transport.execute(hidden_request.clone()).unwrap();
        let absent = transport
            .execute(RequestV1 {
                request: RequestId::new(99).unwrap(),
                command: CommandV1::Save {
                    expected: vellum_protocol::VersionWitness::new(99).unwrap(),
                },
                ..hidden_request
            })
            .unwrap();
        (retained, absent)
    });
    for _ in 0..2 {
        other_subject.serve_next().unwrap();
    }
    let (hidden_retained, hidden_absent) = hidden.join().unwrap();
    assert_eq!(hidden_retained, hidden_absent);
    assert_eq!(
        hidden_retained,
        ReplyV1::Refused(vellum_protocol::RefusalV1::UnknownClient)
    );
    drop(other_subject);

    let mut restarted = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions,
    )
    .unwrap()
    .require_authentication(SubjectAuthenticationPort, (claim, subject), 3);
    let socket = fixture.socket.clone();
    let replay = std::thread::spawn(move || {
        let connector = AuthenticatedUnixConnector::with_timeout(
            socket,
            LocalAuthenticationClaimV1::new(claim),
            Duration::from_secs(2),
        )
        .unwrap();
        let mut transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let mut changed = request.clone();
        if let CommandV1::ReplaceAndSave { inserted, .. } = &mut changed.command {
            *inserted = "changed".to_owned();
        }
        let conflict = transport.execute(changed).unwrap();
        let replay = transport.execute(request).unwrap();
        let next_client = transport.connect().unwrap();
        (conflict, replay, next_client)
    });
    for _ in 0..3 {
        restarted.serve_next().unwrap();
    }
    let (conflict, replayed, next_client) = replay.join().unwrap();
    assert_eq!(
        conflict,
        ReplyV1::Refused(vellum_protocol::RefusalV1::RequestConflict)
    );
    assert_eq!(replayed, original_reply);
    assert_eq!(fs::read_to_string(&fixture.source).unwrap(), "external");
    assert!(next_client.get() > historical_client.get());
    assert_eq!(restarted.usage().durable_receipts_replayed, 1);
    assert_eq!(restarted.usage().durable_receipt_conflicts, 1);
}

#[test]
fn durable_operation_lookup_survives_restart_with_new_transport_identity_and_no_second_write() {
    let fixture = Fixture::new("alpha");
    let claim = [0x71; 16];
    let subject = [0x72; 16];
    let operation = DurableOperationRefV1::new([0x73; 16]).unwrap();
    let actions = AuthorityActionSetV1::read_only()
        .join(AuthorityActionSetV1::singleton(
            AuthorityActionV1::EditSource,
        ))
        .join(AuthorityActionSetV1::singleton(
            AuthorityActionV1::PersistSource,
        ));
    let mut first = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions,
    )
    .unwrap()
    .require_authentication(SubjectAuthenticationPort, (claim, subject), 5);
    let socket = fixture.socket.clone();
    let original = std::thread::spawn(move || {
        let connector = AuthenticatedUnixConnector::with_timeout(
            socket,
            LocalAuthenticationClaimV1::new(claim),
            Duration::from_secs(2),
        )
        .unwrap();
        let mut transport = RemoteTransport::new(
            LoseDurableExecuteReply { inner: connector },
            TransportLimits::reference(),
        )
        .unwrap();
        let first_client = transport.connect().unwrap();
        let client = transport.connect().unwrap();
        assert_ne!(client, first_client);
        let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(inventory)) = transport
            .execute(RequestV1 {
                client,
                request: RequestId::new(1).unwrap(),
                command: CommandV1::WorkspaceInventory,
            })
            .unwrap()
        else {
            panic!("expected inventory")
        };
        let ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot)) = transport
            .execute(RequestV1 {
                client,
                request: RequestId::new(2).unwrap(),
                command: CommandV1::Snapshot,
            })
            .unwrap()
        else {
            panic!("expected snapshot")
        };
        let intent = DurableEffectIntentV1::ReplaceDocumentAndSave {
            document: inventory.documents[0],
            expected_revision: snapshot.revision,
            start: 0,
            end: 5,
            inserted: "omega".into(),
        };
        assert_eq!(
            transport.execute_durable(DurableExecuteRequestV1 {
                operation,
                client,
                request: RequestId::new(3).unwrap(),
                intent: intent.clone(),
            }),
            Err(vellum_protocol::ServiceAccessErrorV1::OutcomeUnknown)
        );
        (client, intent)
    });
    for _ in 0..5 {
        first.serve_next().unwrap();
    }
    let (old_client, intent) = original.join().unwrap();
    assert_eq!(fs::read_to_string(&fixture.source).unwrap(), "omega");
    assert_eq!(first.service_usage().durable_effect_requests, 1);
    assert_eq!(first.operational_snapshot().utilization.durable_receipts, 1);
    drop(first);

    fs::write(&fixture.source, "external").unwrap();
    let mut restarted = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions,
    )
    .unwrap()
    .require_authentication(SubjectAuthenticationPort, (claim, subject), 3);
    let socket = fixture.socket.clone();
    let lookup = std::thread::spawn(move || {
        let connector = AuthenticatedUnixConnector::with_timeout(
            socket,
            LocalAuthenticationClaimV1::new(claim),
            Duration::from_secs(2),
        )
        .unwrap();
        let mut transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let client = transport.connect().unwrap();
        assert_ne!(client, old_client);
        let complete = transport
            .lookup_durable_receipt(DurableReceiptLookupV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent: intent.clone(),
            })
            .unwrap();
        let conflict = transport
            .lookup_durable_receipt(DurableReceiptLookupV1 {
                operation,
                client,
                request: RequestId::new(2).unwrap(),
                intent: DurableEffectIntentV1::AddFile {
                    locator: "changed".into(),
                },
            })
            .unwrap();
        (complete, conflict)
    });
    for _ in 0..3 {
        restarted.serve_next().unwrap();
    }
    let (complete, conflict) = lookup.join().unwrap();
    let DurableReceiptLookupOutcomeV1::Complete(
        vellum_protocol::DurableExecuteOutcomeV1::Completed(
            vellum_protocol::DurableEffectReceiptV1::DocumentReplacedAndSaved(receipt),
        ),
    ) = complete
    else {
        panic!("lost durable reply must be retrieved after restart")
    };
    assert_eq!(&*receipt.text, "omega");
    assert_eq!(conflict, DurableReceiptLookupOutcomeV1::RequestConflict);
    assert_eq!(fs::read_to_string(&fixture.source).unwrap(), "external");
    assert_eq!(restarted.service_usage().durable_effect_requests, 0);
    assert_eq!(restarted.usage().durable_operation_lookups, 2);
    assert_eq!(restarted.usage().durable_lookup_complete, 1);
    assert_eq!(restarted.usage().durable_lookup_conflict, 1);
}

#[test]
fn authenticated_membership_receipt_replays_after_checkpointed_restart() {
    let fixture = Fixture::new("alpha");
    let claim = [0x43; 16];
    let subject = [0x64; 16];
    let actions = AuthorityActionSetV1::read_only().join(AuthorityActionSetV1::singleton(
        AuthorityActionV1::AdmitFile,
    ));
    let mut first = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        Workspace::empty(profile()).unwrap(),
        limits(),
        actions,
    )
    .unwrap()
    .require_authentication(SubjectAuthenticationPort, (claim, subject), 2);
    let socket = fixture.socket.clone();
    let locator = fixture.source.to_str().unwrap().to_owned();
    let operation = std::thread::spawn(move || {
        let connector = AuthenticatedUnixConnector::with_timeout(
            socket,
            LocalAuthenticationClaimV1::new(claim),
            Duration::from_secs(2),
        )
        .unwrap();
        let mut transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let client = transport.connect().unwrap();
        let request = RequestV1 {
            client,
            request: RequestId::new(1).unwrap(),
            command: CommandV1::AddFile { locator },
        };
        let reply = transport.execute(request.clone()).unwrap();
        (request, reply)
    });
    first.serve_next().unwrap();
    first.serve_next().unwrap();
    let (request, original) = operation.join().unwrap();
    assert_eq!(first.usage().durable_receipts_completed, 1);
    assert_eq!(
        first
            .operational_snapshot()
            .workspace_identity
            .unwrap()
            .committed,
        2
    );
    drop(first);

    let mut restarted = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions,
    )
    .unwrap()
    .require_authentication(SubjectAuthenticationPort, (claim, subject), 1);
    let socket = fixture.socket.clone();
    let replay = std::thread::spawn(move || {
        let connector = AuthenticatedUnixConnector::with_timeout(
            socket,
            LocalAuthenticationClaimV1::new(claim),
            Duration::from_secs(2),
        )
        .unwrap();
        let mut transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        transport.execute(request).unwrap()
    });
    restarted.serve_next().unwrap();
    assert_eq!(replay.join().unwrap(), original);
    assert_eq!(restarted.usage().durable_receipts_replayed, 1);
    assert_eq!(
        restarted
            .operational_snapshot()
            .workspace_identity
            .unwrap()
            .store_attempts,
        1,
        "startup reservation only; replay must not repeat membership checkpoint"
    );
}

#[test]
fn pending_journal_intent_refuses_restart_before_endpoint_publication() {
    let fixture = Fixture::new("alpha");
    let actions = AuthorityActionSetV1::read_only().join(AuthorityActionSetV1::singleton(
        AuthorityActionV1::PersistSource,
    ));
    let first = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions,
    )
    .unwrap();
    drop(first);

    let mut metadata = LocalWorkspaceIdentityMetadataV1::default();
    let WorkspaceIdentityRecoveryV1::Recovered(identity) =
        metadata.load(&fixture.directory, limits().workspace_identity)
    else {
        panic!("expected identity")
    };
    let scope = RequestJournalScopeV1::new(identity.workspace(), identity.generation()).unwrap();
    let request = RequestV1 {
        client: vellum_protocol::ClientId::new(1).unwrap(),
        request: RequestId::new(1).unwrap(),
        command: CommandV1::Save {
            expected: vellum_protocol::VersionWitness::new(1).unwrap(),
        },
    };
    let frozen = vellum_wire::encode_request_v1(
        &vellum_wire::WireRequestV1::Execute(request),
        vellum_wire::WireLimits::reference(),
    )
    .unwrap();
    let mut journal = LocalRequestJournalV1::default();
    journal
        .begin(
            &fixture.directory,
            RequestJournalIntentV1 {
                scope,
                client: std::num::NonZeroU64::new(1).unwrap(),
                request: std::num::NonZeroU64::new(1).unwrap(),
                authenticated_subject: [7; 16],
                frozen_request: &frozen,
            },
            limits().request_journal,
        )
        .unwrap();

    let error = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions,
    )
    .err()
    .expect("pending effect must refuse startup");
    assert_eq!(
        error,
        DaemonBuildError::RequestJournal(
            vellum_host::request_journal::RequestJournalRefusalV1::Pending
        )
    );
    assert!(!fixture.socket.exists());
}

#[test]
fn journal_begin_or_completion_uncertainty_suppresses_reply_and_fences() {
    for failure in [JournalFailurePoint::Begin, JournalFailurePoint::Complete] {
        let fixture = Fixture::new("alpha");
        let claim = [0x42; 16];
        let actions = AuthorityActionSetV1::read_only().join(AuthorityActionSetV1::singleton(
            AuthorityActionV1::PersistSource,
        ));
        let mut daemon = LocalDaemon::bind_with_persistence_ports_and_action_ceiling(
            &fixture.socket,
            workspace(&fixture.source),
            limits(),
            actions,
            ScriptedIdentityMetadata {
                load: WorkspaceIdentityRecoveryV1::Refused(
                    WorkspaceIdentityRecoveryRefusalV1::Missing,
                ),
                store: Ok(()),
            },
            FailingRequestJournal(failure),
        )
        .unwrap()
        .require_authentication(SubjectAuthenticationPort, (claim, [0x63; 16]), 3);
        let socket = fixture.socket.clone();
        let client = std::thread::spawn(move || {
            let connector = AuthenticatedUnixConnector::with_timeout(
                socket,
                LocalAuthenticationClaimV1::new(claim),
                Duration::from_secs(2),
            )
            .unwrap();
            let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
            let mut client = Client::connect(transport).unwrap();
            let ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot)) = client.snapshot().unwrap()
            else {
                panic!("expected snapshot")
            };
            client.save(snapshot.witness)
        });
        daemon.serve_next().unwrap();
        daemon.serve_next().unwrap();
        assert!(matches!(
            daemon.serve_next(),
            Err(DaemonServeError::Request(
                TransportError::ServiceUnavailable
            ))
        ));
        assert!(matches!(
            client.join().unwrap(),
            Err(vellum_protocol::ServiceAccessErrorV1::OutcomeUnknown)
        ));
        assert_eq!(
            daemon.operational_snapshot().status.lifecycle,
            DaemonLifecycle::Fenced
        );
        assert_eq!(daemon.usage().durable_journal_failures, 1);
    }
}

#[test]
fn runtime_directory_admission_checkpoints_every_exact_discovered_binding() {
    let fixture = Fixture::new("unused");
    let directory = DirectoryFixture::new(&[("b.txt", "beta"), ("a.txt", "alpha")]);
    let actions = AuthorityActionSetV1::read_only().join(AuthorityActionSetV1::singleton(
        AuthorityActionV1::DiscoverDirectory,
    ));
    let mut first = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        Workspace::empty(profile()).unwrap(),
        limits(),
        actions,
    )
    .unwrap();
    let socket = fixture.socket.clone();
    let locator = directory.directory.to_str().unwrap().to_owned();
    let client = std::thread::spawn(move || {
        let connector = UnixConnector::with_timeout(socket, Duration::from_secs(2)).unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let mut client = Client::connect(transport).unwrap();
        let ReplyV1::Accepted(OutcomeV1::DirectoryAdmitted(receipt)) =
            client.add_directory(locator).unwrap()
        else {
            panic!("expected admitted directory")
        };
        if let Err((_, error)) = client.disconnect() {
            panic!("disconnect failed: {error:?}");
        }
        receipt
    });
    for _ in 0..3 {
        first.serve_next().unwrap();
    }
    let admitted = client.join().unwrap();
    assert_eq!(admitted.added_documents.len(), 2);
    assert_eq!(admitted.added, 2);
    drop(first);

    let mut restarted = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        compose_startup_workspace(
            std::slice::from_ref(&directory.directory),
            profile(),
            limits(),
        )
        .unwrap(),
        limits(),
        actions,
    )
    .unwrap();
    let recovered = inventory_from_daemon(&mut restarted, fixture.socket.clone());
    assert_eq!(recovered.documents, admitted.added_documents);
    assert_eq!(recovered.roots.len(), 1);
    assert_eq!(recovered.roots[0].id, admitted.root);
}

#[test]
fn runtime_retirement_checkpoints_tombstones_and_never_reuses_ordinals_after_restart() {
    let fixture = Fixture::new("alpha");
    let actions = AuthorityActionSetV1::read_only()
        .join(AuthorityActionSetV1::singleton(
            AuthorityActionV1::RetireMembership,
        ))
        .join(AuthorityActionSetV1::singleton(
            AuthorityActionV1::AdmitFile,
        ));
    let mut first = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions,
    )
    .unwrap();
    let socket = fixture.socket.clone();
    let retired = std::thread::spawn(move || {
        let connector = UnixConnector::with_timeout(socket, Duration::from_secs(2)).unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let mut client = Client::connect(transport).unwrap();
        let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(inventory)) =
            client.workspace_inventory().unwrap()
        else {
            panic!("expected inventory")
        };
        assert!(matches!(
            client.close_view().unwrap(),
            ReplyV1::Accepted(OutcomeV1::ViewClosed(_))
        ));
        assert!(matches!(
            client.remove_root(inventory.roots[0].id).unwrap(),
            ReplyV1::Accepted(OutcomeV1::RootRemoved(_))
        ));
        assert!(matches!(
            client.remove_document(inventory.documents[0]).unwrap(),
            ReplyV1::Accepted(OutcomeV1::DocumentRemoved(_))
        ));
        if let Err((_, error)) = client.disconnect() {
            panic!("disconnect failed: {error:?}");
        }
        inventory
    });
    for _ in 0..6 {
        first.serve_next().unwrap();
    }
    let prior = retired.join().unwrap();
    let usage = first.operational_snapshot().workspace_identity.unwrap();
    assert_eq!(usage.committed, 3);
    drop(first);

    let mut restarted = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        Workspace::empty(profile()).unwrap(),
        limits(),
        actions,
    )
    .unwrap();
    let socket = fixture.socket.clone();
    let locator = fixture.source.to_str().unwrap().to_owned();
    let admitted = std::thread::spawn(move || {
        let connector = UnixConnector::with_timeout(socket, Duration::from_secs(2)).unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let mut client = Client::connect(transport).unwrap();
        let ReplyV1::Accepted(OutcomeV1::FileAdmitted(receipt)) = client.add_file(locator).unwrap()
        else {
            panic!("expected re-admission")
        };
        if let Err((_, error)) = client.disconnect() {
            panic!("disconnect failed: {error:?}");
        }
        receipt
    });
    for _ in 0..3 {
        restarted.serve_next().unwrap();
    }
    let next = admitted.join().unwrap();
    assert!(next.document.get() > prior.documents[0].get());
    assert!(next.root.unwrap().get() > prior.roots[0].id.get());
}

#[test]
fn authority_denial_performs_no_runtime_membership_checkpoint() {
    let fixture = Fixture::new("alpha");
    let mut daemon = LocalDaemon::bind_with_persistent_identity(
        &fixture.socket,
        Workspace::empty(profile()).unwrap(),
        limits(),
    )
    .unwrap();
    let socket = fixture.socket.clone();
    let locator = fixture.source.to_str().unwrap().to_owned();
    let client = std::thread::spawn(move || {
        let connector = UnixConnector::with_timeout(socket, Duration::from_secs(2)).unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let mut client = Client::connect(transport).unwrap();
        let reply = client.add_file(locator).unwrap();
        if let Err((_, error)) = client.disconnect() {
            panic!("disconnect failed: {error:?}");
        }
        reply
    });
    for _ in 0..3 {
        daemon.serve_next().unwrap();
    }
    assert_eq!(
        client.join().unwrap(),
        ReplyV1::Refused(vellum_protocol::RefusalV1::AuthorityDenied)
    );
    let identity = daemon.operational_snapshot().workspace_identity.unwrap();
    assert_eq!(identity.store_attempts, 1);
    assert_eq!(identity.committed, 1);
}

#[test]
fn runtime_identity_capacity_failure_fences_before_an_accepted_reply() {
    let fixture = Fixture::new("alpha");
    let actions = AuthorityActionSetV1::read_only().join(AuthorityActionSetV1::singleton(
        AuthorityActionV1::AdmitFile,
    ));
    let mut bounded = limits();
    bounded.workspace_identity.max_encoded_bytes = 72;
    let mut daemon = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        Workspace::empty(profile()).unwrap(),
        bounded,
        actions,
    )
    .unwrap();
    let socket = fixture.socket.clone();
    let locator = fixture.source.to_str().unwrap().to_owned();
    let client = std::thread::spawn(move || {
        let connector = UnixConnector::with_timeout(socket, Duration::from_secs(2)).unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let mut client = Client::connect(transport).unwrap();
        client.add_file(locator)
    });
    daemon.serve_next().unwrap();
    assert!(matches!(
        daemon.serve_next(),
        Err(DaemonServeError::Request(
            TransportError::ServiceUnavailable
        ))
    ));
    assert!(matches!(
        client.join().unwrap(),
        Err(vellum_protocol::ServiceAccessErrorV1::OutcomeUnknown)
    ));
    let snapshot = daemon.operational_snapshot();
    assert_eq!(snapshot.status.lifecycle, DaemonLifecycle::Fenced);
    let identity = snapshot.workspace_identity.unwrap();
    assert_eq!(identity.store_attempts, 1);
    assert_eq!(identity.committed, 1);
}

#[test]
fn directory_discovered_documents_keep_exact_bindings_across_restart() {
    let fixture = Fixture::new("unused");
    let directory = DirectoryFixture::new(&[("b.txt", "beta"), ("a.txt", "alpha")]);
    let mut first = LocalDaemon::bind_with_persistent_identity(
        &fixture.socket,
        compose_startup_workspace(
            std::slice::from_ref(&directory.directory),
            profile(),
            limits(),
        )
        .unwrap(),
        limits(),
    )
    .unwrap();
    let before = inventory_from_daemon(&mut first, fixture.socket.clone());
    assert_eq!(before.documents.len(), 2);
    assert_eq!(before.roots.len(), 1);
    drop(first);

    let mut restarted = LocalDaemon::bind_with_persistent_identity(
        &fixture.socket,
        compose_startup_workspace(
            std::slice::from_ref(&directory.directory),
            profile(),
            limits(),
        )
        .unwrap(),
        limits(),
    )
    .unwrap();
    let after = inventory_from_daemon(&mut restarted, fixture.socket.clone());
    assert_eq!(after.documents, before.documents);
    assert_eq!(after.roots, before.roots);
}

#[test]
fn corrupt_conflicting_and_unknown_identity_outcomes_refuse_before_socket_publication() {
    let corrupt = Fixture::new("alpha");
    fs::write(corrupt.directory.join("workspace-identity-v1"), b"corrupt").unwrap();
    fs::set_permissions(
        corrupt.directory.join("workspace-identity-v1"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let Err(corrupt_refusal) = LocalDaemon::bind_with_persistent_identity(
        &corrupt.socket,
        workspace(&corrupt.source),
        limits(),
    ) else {
        panic!("corrupt identity must refuse")
    };
    assert_eq!(
        corrupt_refusal,
        DaemonBuildError::WorkspaceIdentity(WorkspaceIdentityRecoveryRefusalV1::Malformed)
    );
    assert!(!corrupt.socket.exists());

    let conflict = Fixture::new("alpha");
    let conflict_port = ScriptedIdentityMetadata {
        load: WorkspaceIdentityRecoveryV1::Refused(WorkspaceIdentityRecoveryRefusalV1::Conflict),
        store: Ok(()),
    };
    let Err(conflict_refusal) =
        LocalDaemon::bind_with_workspace_identity_metadata_and_action_ceiling(
            &conflict.socket,
            workspace(&conflict.source),
            limits(),
            AuthorityActionSetV1::read_only(),
            conflict_port,
        )
    else {
        panic!("conflicting identity must refuse")
    };
    assert_eq!(
        conflict_refusal,
        DaemonBuildError::WorkspaceIdentity(WorkspaceIdentityRecoveryRefusalV1::Conflict)
    );
    assert!(!conflict.socket.exists());

    let unknown = Fixture::new("alpha");
    let unknown_port = ScriptedIdentityMetadata {
        load: WorkspaceIdentityRecoveryV1::Refused(WorkspaceIdentityRecoveryRefusalV1::Missing),
        store: Err(WorkspaceIdentityRecoveryRefusalV1::OutcomeUnknown),
    };
    let Err(refusal) = LocalDaemon::bind_with_workspace_identity_metadata_and_action_ceiling(
        &unknown.socket,
        workspace(&unknown.source),
        limits(),
        AuthorityActionSetV1::read_only(),
        unknown_port,
    ) else {
        panic!("unknown commit outcome must refuse")
    };
    assert_eq!(
        refusal,
        DaemonBuildError::WorkspaceIdentity(WorkspaceIdentityRecoveryRefusalV1::OutcomeUnknown)
    );
    assert!(!unknown.socket.exists());
    assert!(!format!("{refusal:?}").contains(unknown.directory.to_str().unwrap()));
}

#[test]
fn native_directory_addition_uses_the_daemons_qualified_discovery_adapter() {
    let fixture = Fixture::new("unused");
    let directory = DirectoryFixture::new(&[("b.txt", "second"), ("a.txt", "first")]);
    let workspace = compose_startup_workspace(&[], profile(), limits()).unwrap();
    let actions = AuthorityActionSetV1::read_only().join(AuthorityActionSetV1::singleton(
        AuthorityActionV1::DiscoverDirectory,
    ));
    let mut daemon =
        LocalDaemon::bind_with_action_ceiling(&fixture.socket, workspace, limits(), actions)
            .unwrap();
    let socket = fixture.socket.clone();
    let root = directory.directory.clone();
    let client = std::thread::spawn(move || {
        let connector = UnixConnector::with_timeout(socket, Duration::from_secs(2)).unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let mut client = Client::connect(transport).unwrap();
        let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(before)) =
            client.workspace_inventory().unwrap()
        else {
            panic!("expected empty inventory")
        };
        assert!(before.documents.is_empty());
        let ReplyV1::Accepted(OutcomeV1::DirectoryAdmitted(after)) = client
            .add_directory(root.to_string_lossy().into_owned())
            .unwrap()
        else {
            panic!("expected directory admission")
        };
        assert_eq!(after.added_documents.len(), 2);
        assert_eq!(after.added, 2);
        let ReplyV1::Accepted(OutcomeV1::ViewOpened(opened)) = client.open_view().unwrap() else {
            panic!("expected explicit view open after adding directory")
        };
        assert_eq!(opened.document, after.added_documents[0]);
        let ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot)) = client.snapshot().unwrap() else {
            panic!("expected snapshot after adding directory")
        };
        assert_eq!(&*snapshot.text, "first");
        if let Err((_, error)) = client.disconnect() {
            panic!("disconnect failed: {error:?}");
        }
    });

    for _ in 0..6 {
        daemon.serve_next().unwrap();
    }
    client.join().unwrap();
    assert_eq!(daemon.service_usage().active_clients, 0);
}

#[test]
fn default_daemon_is_read_only_and_does_not_install_directory_discovery_authority() {
    let fixture = Fixture::new("unused");
    let directory = DirectoryFixture::new(&[("a.txt", "first")]);
    let workspace = compose_startup_workspace(&[], profile(), limits()).unwrap();
    let mut daemon = LocalDaemon::bind(&fixture.socket, workspace, limits()).unwrap();
    let socket = fixture.socket.clone();
    let root = directory.directory.clone();
    let client = std::thread::spawn(move || {
        let connector = UnixConnector::with_timeout(socket, Duration::from_secs(2)).unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let mut client = Client::connect(transport).unwrap();
        assert_eq!(
            client.add_directory(root.to_string_lossy().into_owned()),
            Ok(ReplyV1::Refused(
                vellum_protocol::RefusalV1::AuthorityDenied
            ))
        );
        if let Err((_, error)) = client.disconnect() {
            panic!("disconnect failed: {error:?}");
        }
    });

    for _ in 0..3 {
        daemon.serve_next().unwrap();
    }
    client.join().unwrap();
    assert_eq!(daemon.service_usage().authority_denied, 1);
}

#[test]
fn operational_state_is_content_free_and_graceful_drain_is_monotone() {
    let fixture = Fixture::new("alpha");
    let mut daemon =
        LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), limits()).unwrap();

    let ready = daemon.probe_operational_status();
    assert_eq!(ready.lifecycle, DaemonLifecycle::Running);
    assert_eq!(ready.health, DaemonHealth::Healthy);
    assert_eq!(ready.readiness, DaemonReadiness::Ready);
    assert_eq!(ready.in_flight, 0);
    assert!(!format!("{ready:?}").contains(fixture.directory.to_str().unwrap()));

    assert_eq!(daemon.begin_drain(), DrainTransition::Started);
    assert_eq!(daemon.begin_drain(), DrainTransition::AlreadyDraining);
    let draining = daemon.probe_operational_status();
    assert_eq!(draining.lifecycle, DaemonLifecycle::Draining);
    assert_eq!(draining.health, DaemonHealth::Healthy);
    assert_eq!(draining.readiness, DaemonReadiness::NotReady);
    assert_eq!(daemon.serve_next(), Err(DaemonServeError::NotReady));

    assert_eq!(daemon.complete_drain(), DrainCompletion::Complete);
    assert_eq!(daemon.begin_drain(), DrainTransition::AlreadyStopped);
    let stopped = daemon.probe_operational_status();
    assert_eq!(stopped.lifecycle, DaemonLifecycle::Stopped);
    assert_eq!(stopped.health, DaemonHealth::Healthy);
    assert_eq!(stopped.readiness, DaemonReadiness::NotReady);
    assert_eq!(daemon.complete_drain(), DrainCompletion::AlreadyComplete);
    assert_eq!(daemon.serve_next(), Err(DaemonServeError::NotReady));

    let usage = daemon.usage();
    assert_eq!(usage.drain_started, 1);
    assert_eq!(usage.drain_replayed, 3);
    assert_eq!(usage.operational_probes, 3);
}

#[test]
fn operational_snapshot_is_bounded_content_free_and_observationally_pure() {
    let fixture = Fixture::new("secret-source-content");
    let mut daemon =
        LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), limits()).unwrap();

    let service_before = daemon.service_usage();
    let usage_before = daemon.usage();
    let first = daemon.operational_snapshot();
    for _ in 0..64 {
        assert_eq!(daemon.operational_snapshot(), first);
    }

    assert_eq!(daemon.service_usage(), service_before);
    assert_eq!(daemon.usage(), usage_before);
    assert_eq!(first.status.lifecycle, DaemonLifecycle::Running);
    assert_eq!(first.status.readiness, DaemonReadiness::Ready);
    assert_eq!(first.utilization.in_flight_connections, 0);
    assert_eq!(first.utilization.in_flight_capacity, 1);
    assert_eq!(first.utilization.active_clients, 0);
    assert_eq!(
        first.utilization.client_capacity,
        limits().service.max_clients
    );
    assert_eq!(first.utilization.durable_receipts, 0);
    assert_eq!(
        first.utilization.durable_receipt_capacity,
        limits().request_journal.max_entries
    );
    assert!(!first.saturation.in_flight);
    assert!(!first.saturation.clients);
    assert!(!first.saturation.durable_receipts);
    assert!(!first.saturation.counters);
    assert!(!first.saturation.request_latency);
    assert_eq!(first.request_latency.unwrap().count, 0);

    let debug = format!("{first:?}");
    assert!(!debug.contains("secret-source-content"));
    assert!(!debug.contains(fixture.directory.to_str().unwrap()));
    assert!(!debug.contains(fixture.socket.to_str().unwrap()));

    // Arbitrarily many observations do not change the next service outcome.
    let socket = fixture.socket.clone();
    let client = std::thread::spawn(move || {
        let connector = UnixConnector::with_timeout(socket, Duration::from_secs(2)).unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let mut client = Client::connect(transport).unwrap();
        let ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot)) = client.snapshot().unwrap() else {
            panic!("expected snapshot")
        };
        assert_eq!(&*snapshot.text, "secret-source-content");
        if let Err((_, error)) = client.disconnect() {
            panic!("disconnect failed: {error:?}");
        }
    });
    for _ in 0..3 {
        daemon.serve_next().unwrap();
    }
    client.join().unwrap();

    // They also do not change the next lifecycle outcomes.
    assert_eq!(daemon.begin_drain(), DrainTransition::Started);
    assert_eq!(daemon.complete_drain(), DrainCompletion::Complete);
}

#[test]
fn request_latency_is_optional_and_excludes_time_blocked_in_accept() {
    let disabled_fixture = Fixture::new("disabled");
    let mut disabled_limits = limits();
    disabled_limits.request_latency = None;
    let disabled = LocalDaemon::bind(
        &disabled_fixture.socket,
        workspace(&disabled_fixture.source),
        disabled_limits,
    )
    .unwrap();
    assert_eq!(disabled.operational_snapshot().request_latency, None);
    assert!(!disabled.operational_snapshot().saturation.request_latency);
    drop(disabled);

    let fixture = Fixture::new("alpha");
    let mut daemon =
        LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), limits()).unwrap();
    let socket = fixture.socket.clone();
    let (delay_started_tx, delay_started_rx) = std::sync::mpsc::sync_channel(0);
    let client = std::thread::spawn(move || {
        delay_started_tx.send(()).unwrap();
        std::thread::sleep(Duration::from_millis(500));
        let connector = UnixConnector::with_timeout(socket, Duration::from_secs(2)).unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        drop(Client::connect(transport).unwrap());
    });
    delay_started_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    // The peer cannot connect until after its synchronized delay. Almost all
    // of that interval is therefore spent in the daemon's blocking accept;
    // the 100 ms assertion margin absorbs the bounded pre-accept checks.
    daemon.serve_next().unwrap();
    client.join().unwrap();
    let latency = daemon.operational_snapshot().request_latency.unwrap();
    assert_eq!(latency.count, 1);
    assert_eq!(
        latency.bucket_counts.into_iter().sum::<u64>() + latency.overflow_count,
        1
    );
    assert!(latency.sum < Duration::from_millis(400));
    assert!(!latency.fidelity_lost);
}

#[test]
fn given_an_accepted_silent_client_when_its_io_deadline_expires_then_timeout_evidence_is_closed() {
    let fixture = Fixture::new("alpha");
    let mut slow_limits = limits();
    slow_limits.io_timeout = Duration::from_millis(120);
    slow_limits.request_latency = Some(RequestLatencyProfile {
        upper_bounds: [
            Duration::from_millis(1),
            Duration::from_millis(5),
            Duration::from_millis(10),
            Duration::from_millis(25),
            Duration::from_millis(50),
        ],
    });
    let mut daemon =
        LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), slow_limits).unwrap();

    // Connecting before the server enters `accept` removes connection timing
    // from the assertion. The peer then deliberately sends no frame bytes.
    let silent = UnixStream::connect(&fixture.socket).unwrap();
    assert_eq!(
        daemon.serve_next(),
        Err(DaemonServeError::Request(
            vellum_transport::TransportError::Io
        ))
    );
    drop(silent);

    let snapshot = daemon.operational_snapshot();
    assert_eq!(snapshot.daemon.accepted_connections, 1);
    assert_eq!(snapshot.daemon.completed_connections, 0);
    assert_eq!(snapshot.daemon.request_failures, 1);
    assert_eq!(snapshot.daemon.request_deadline_exhausted, 1);
    assert_eq!(snapshot.status.in_flight, 0);
    assert_eq!(snapshot.status.readiness, DaemonReadiness::Ready);
    let latency = snapshot.request_latency.unwrap();
    assert_eq!(latency.count, 1);
    assert_eq!(latency.overflow_count, 1);

    // The timed-out obligation was discharged before the error was returned,
    // so a later drain has no unresolved work to conceal.
    assert_eq!(daemon.begin_drain(), DrainTransition::Started);
    assert_eq!(daemon.complete_drain(), DrainCompletion::Complete);
}

#[test]
fn invalid_latency_bounds_refuse_before_binding_the_socket() {
    let fixture = Fixture::new("alpha");
    let mut invalid = limits();
    invalid.request_latency = Some(RequestLatencyProfile {
        upper_bounds: [
            Duration::from_millis(1),
            Duration::from_millis(1),
            Duration::from_millis(2),
            Duration::from_millis(3),
            Duration::from_millis(4),
        ],
    });
    assert!(matches!(
        LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), invalid),
        Err(DaemonBuildError::InvalidLimits)
    ));
    assert!(!fixture.socket.exists());
}

#[test]
fn fixed_authority_lifetime_budget_is_checked_exactly_before_publication() {
    let fixture = Fixture::new("alpha");
    let mut exact = limits();
    let required = exact.service.max_requests * 2
        + (exact.request_journal.max_entries + exact.request_journal.max_retired_operations) * 3;
    exact.service.max_authority_decisions = u64::try_from(required).unwrap();
    let daemon = LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), exact).unwrap();
    drop(daemon);

    let mut insufficient = exact;
    insufficient.service.max_authority_decisions -= 1;
    assert_eq!(
        LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), insufficient)
            .err()
            .unwrap(),
        DaemonBuildError::InvalidLimits
    );
    assert!(!fixture.socket.exists());

    let mut overflow = exact;
    overflow.service.max_requests = usize::MAX;
    overflow.service.max_authority_decisions = u64::MAX;
    assert_eq!(
        LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), overflow)
            .err()
            .unwrap(),
        DaemonBuildError::InvalidLimits
    );
    assert!(!fixture.socket.exists());
}

#[test]
fn authentication_attempt_budget_is_independently_nonzero_before_publication() {
    let fixture = Fixture::new("alpha");
    let mut invalid = limits();
    invalid.max_authentication_attempts = 0;
    invalid.service.max_authority_decisions = u64::MAX;
    assert_eq!(
        LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), invalid)
            .err()
            .unwrap(),
        DaemonBuildError::InvalidLimits
    );
    assert!(!fixture.socket.exists());
}

#[test]
fn owner_only_shutdown_control_rejects_faults_wakes_accept_and_drains_truthfully() {
    let fixture = Fixture::new("alpha");
    let mut shutdown_limits = limits();
    shutdown_limits.io_timeout = Duration::from_millis(20);
    let mut daemon =
        LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), shutdown_limits).unwrap();
    let control = daemon.bind_shutdown_control(&fixture.control).unwrap();
    assert_eq!(
        fs::symlink_metadata(&fixture.control)
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o600
    );
    let control_thread = std::thread::spawn(move || control.wait());

    let silent = UnixStream::connect(&fixture.control).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    drop(silent);

    let mut malformed = UnixStream::connect(&fixture.control).unwrap();
    malformed.write_all(&[9]).unwrap();
    let mut refusal = [1_u8; 1];
    malformed.read_exact(&mut refusal).unwrap();
    assert_eq!(refusal, [0]);
    let mut extended = UnixStream::connect(&fixture.control).unwrap();
    extended.write_all(&[2, 1, 8]).unwrap();
    extended.shutdown(Shutdown::Write).unwrap();
    extended.read_exact(&mut refusal).unwrap();
    assert_eq!(refusal, [0]);
    assert_eq!(
        daemon.operational_snapshot().status.lifecycle,
        DaemonLifecycle::Running
    );

    let control_socket = fixture.control.clone();
    let requester =
        std::thread::spawn(move || request_shutdown(&control_socket, Duration::from_secs(2)));
    assert_eq!(
        daemon.serve_next(),
        Err(DaemonServeError::ShutdownRequested)
    );
    requester.join().unwrap().unwrap();
    control_thread.join().unwrap().unwrap();

    // The wake connection is not admitted as service traffic.
    assert_eq!(daemon.usage().accepted_connections, 0);
    assert_eq!(daemon.usage().completed_connections, 0);
    assert_eq!(daemon.begin_drain(), DrainTransition::Started);
    assert_eq!(daemon.complete_drain(), DrainCompletion::Complete);
    let stopped = daemon.operational_snapshot();
    assert_eq!(stopped.status.lifecycle, DaemonLifecycle::Stopped);
    assert_eq!(stopped.status.readiness, DaemonReadiness::NotReady);
    assert_eq!(stopped.daemon.drain_started, 1);
    assert_eq!(stopped.daemon.drain_completed, 1);
    assert!(!fixture.control.exists());
}

#[test]
fn private_control_status_is_passive_bounded_and_tracks_lifecycle() {
    let fixture = Fixture::new("alpha");
    let mut daemon =
        LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), limits()).unwrap();
    let control = daemon.bind_shutdown_control(&fixture.control).unwrap();
    let control_thread = std::thread::spawn(move || control.wait());

    let before = daemon.operational_snapshot();
    let running = request_status(&fixture.control, Duration::from_secs(2)).unwrap();
    assert_eq!(running.liveness, DaemonLiveness::Live);
    assert_eq!(running.lifecycle, DaemonLifecycle::Running);
    assert_eq!(running.health, DaemonHealth::Healthy);
    assert_eq!(running.readiness, DaemonReadiness::Ready);
    let mut v1_utilization = before.utilization;
    // The exact V1 status frame predates the additive tombstone gauge.
    v1_utilization.retired_durable_operations = 0;
    v1_utilization.retired_durable_operation_capacity = 0;
    assert_eq!(running.utilization, v1_utilization);
    assert_eq!(running.saturation, before.saturation);
    assert_eq!(daemon.operational_snapshot(), before);
    assert_eq!(daemon.usage().operational_probes, 0);

    let running_v2 = request_status_v2(&fixture.control, Duration::from_secs(2)).unwrap();
    assert_eq!(running_v2.liveness, DaemonLiveness::Live);
    assert_eq!(running_v2.lifecycle, DaemonLifecycle::Running);
    assert_eq!(running_v2.health, DaemonHealth::Healthy);
    assert_eq!(running_v2.readiness, DaemonReadiness::Ready);
    assert_eq!(running_v2.utilization, before.utilization);
    assert_eq!(running_v2.saturation, before.saturation);
    assert_eq!(daemon.operational_snapshot(), before);
    assert_eq!(daemon.usage().operational_probes, 0);

    assert_eq!(daemon.begin_drain(), DrainTransition::Started);
    let draining = request_status(&fixture.control, Duration::from_secs(2)).unwrap();
    assert_eq!(draining.lifecycle, DaemonLifecycle::Draining);
    assert_eq!(draining.health, DaemonHealth::Healthy);
    assert_eq!(draining.readiness, DaemonReadiness::NotReady);

    assert_eq!(daemon.complete_drain(), DrainCompletion::Complete);
    let stopped = request_status(&fixture.control, Duration::from_secs(2)).unwrap();
    assert_eq!(stopped.lifecycle, DaemonLifecycle::Stopped);
    assert_eq!(stopped.readiness, DaemonReadiness::NotReady);

    request_shutdown(&fixture.control, Duration::from_secs(2)).unwrap();
    control_thread.join().unwrap().unwrap();
}

#[test]
fn private_control_status_survives_slow_and_malformed_clients_and_reports_fencing() {
    let fixture = Fixture::new("alpha");
    let mut status_limits = limits();
    status_limits.io_timeout = Duration::from_millis(20);
    let mut daemon =
        LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), status_limits).unwrap();
    let control = daemon.bind_shutdown_control(&fixture.control).unwrap();
    let control_thread = std::thread::spawn(move || control.wait());

    let silent = UnixStream::connect(&fixture.control).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    drop(silent);

    let mut malformed = UnixStream::connect(&fixture.control).unwrap();
    malformed.write_all(&[2, 9]).unwrap();
    let mut refusal = [1_u8; 1];
    malformed.read_exact(&mut refusal).unwrap();
    assert_eq!(refusal, [0]);
    assert_eq!(
        request_status(&fixture.control, Duration::from_secs(2))
            .unwrap()
            .lifecycle,
        DaemonLifecycle::Running
    );

    fs::remove_file(&fixture.lock).unwrap();
    fs::write(&fixture.lock, b"replacement").unwrap();
    fs::set_permissions(&fixture.lock, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        daemon.probe_operational_status().lifecycle,
        DaemonLifecycle::Fenced
    );
    let fenced = request_status(&fixture.control, Duration::from_secs(2)).unwrap();
    assert_eq!(fenced.lifecycle, DaemonLifecycle::Fenced);
    assert_eq!(fenced.health, DaemonHealth::Fenced);
    assert_eq!(fenced.readiness, DaemonReadiness::NotReady);

    request_shutdown(&fixture.control, Duration::from_secs(2)).unwrap();
    control_thread.join().unwrap().unwrap();
}

#[test]
fn control_client_refuses_non_private_and_malformed_foreign_endpoints() {
    let fixture = Fixture::new("alpha");
    let foreign = fixture.directory.join("foreign.sock");
    let listener = UnixListener::bind(&foreign).unwrap();
    fs::set_permissions(&foreign, fs::Permissions::from_mode(0o666)).unwrap();
    assert_eq!(
        request_status(&foreign, Duration::from_millis(20)),
        Err(ShutdownControlError::SocketOwnership)
    );
    drop(listener);
    fs::remove_file(&foreign).unwrap();

    let malformed = fixture.directory.join("malformed.sock");
    let listener = UnixListener::bind(&malformed).unwrap();
    fs::set_permissions(&malformed, fs::Permissions::from_mode(0o600)).unwrap();
    let worker = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 2];
        stream.read_exact(&mut request).unwrap();
        assert_eq!(request, [2, 1]);
        stream.write_all(&[0]).unwrap();
    });
    assert_eq!(
        request_status(&malformed, Duration::from_secs(2)),
        Err(ShutdownControlError::Io)
    );
    worker.join().unwrap();
    fs::remove_file(&malformed).unwrap();
}

#[test]
fn shutdown_control_cleanup_never_removes_a_replacement_path() {
    let fixture = Fixture::new("alpha");
    let daemon = LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), limits()).unwrap();
    let control = daemon.bind_shutdown_control(&fixture.control).unwrap();
    fs::remove_file(&fixture.control).unwrap();
    fs::write(&fixture.control, "replacement").unwrap();
    drop(control);
    assert_eq!(fs::read_to_string(&fixture.control).unwrap(), "replacement");
}

#[test]
fn shutdown_control_refuses_symlink_and_stale_socket_without_unlinking() {
    let fixture = Fixture::new("alpha");
    let daemon = LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), limits()).unwrap();

    std::os::unix::fs::symlink(&fixture.source, &fixture.control).unwrap();
    assert!(matches!(
        daemon.bind_shutdown_control(&fixture.control),
        Err(ShutdownControlError::SocketExists)
    ));
    assert!(
        fs::symlink_metadata(&fixture.control)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    fs::remove_file(&fixture.control).unwrap();

    let stale = UnixListener::bind(&fixture.control).unwrap();
    assert!(matches!(
        daemon.bind_shutdown_control(&fixture.control),
        Err(ShutdownControlError::SocketExists)
    ));
    assert!(
        fs::symlink_metadata(&fixture.control)
            .unwrap()
            .file_type()
            .is_socket()
    );
    drop(stale);
}

#[test]
fn parent_symlink_and_replacement_socket_are_never_treated_as_owned() {
    let fixture = Fixture::new("alpha");
    let alias = std::env::temp_dir().join(format!(
        "vellum-daemon-alias-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::os::unix::fs::symlink(&fixture.directory, &alias).unwrap();
    assert!(matches!(
        LocalDaemon::bind(
            &alias.join("alternate.sock"),
            workspace(&fixture.source),
            limits()
        ),
        Err(DaemonBuildError::ParentNotDirectory)
    ));
    fs::remove_file(&alias).unwrap();

    let mut daemon =
        LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), limits()).unwrap();
    fs::remove_file(&fixture.socket).unwrap();
    let replacement = UnixListener::bind(&fixture.socket).unwrap();
    fs::set_permissions(&fixture.socket, fs::Permissions::from_mode(0o600)).unwrap();
    let status = daemon.probe_operational_status();
    assert_eq!(status.lifecycle, DaemonLifecycle::Fenced);
    drop(daemon);
    assert!(
        fs::symlink_metadata(&fixture.socket)
            .unwrap()
            .file_type()
            .is_socket()
    );
    drop(replacement);
}

#[test]
fn ownership_lock_is_private_persistent_and_exclusive() {
    let fixture = Fixture::new("alpha");
    let daemon = LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), limits()).unwrap();
    let lock_metadata = fs::symlink_metadata(&fixture.lock).unwrap();
    assert!(lock_metadata.file_type().is_file());
    assert_eq!(lock_metadata.permissions().mode() & 0o7777, 0o600);
    assert_eq!(lock_metadata.uid(), rustix::process::geteuid().as_raw());
    assert_eq!(lock_metadata.nlink(), 1);

    let alternate_socket = fixture.directory.join("alternate.sock");
    assert!(matches!(
        LocalDaemon::bind(&alternate_socket, workspace(&fixture.source), limits()),
        Err(DaemonBuildError::LockBusy)
    ));
    assert!(!alternate_socket.exists());

    let identity = (lock_metadata.dev(), lock_metadata.ino());
    drop(daemon);
    let persistent_metadata = fs::symlink_metadata(&fixture.lock).unwrap();
    assert_eq!(
        (persistent_metadata.dev(), persistent_metadata.ino()),
        identity
    );

    let rebound = LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), limits()).unwrap();
    let rebound_metadata = fs::symlink_metadata(&fixture.lock).unwrap();
    assert_eq!((rebound_metadata.dev(), rebound_metadata.ino()), identity);
    drop(rebound);
}

#[test]
fn replaced_lock_fences_service_and_is_never_deleted_by_the_daemon() {
    let fixture = Fixture::new("alpha");
    let mut daemon =
        LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), limits()).unwrap();
    let original = fs::symlink_metadata(&fixture.lock).unwrap();
    fs::remove_file(&fixture.lock).unwrap();
    fs::write(&fixture.lock, b"replacement").unwrap();
    fs::set_permissions(&fixture.lock, fs::Permissions::from_mode(0o600)).unwrap();
    let replacement = fs::symlink_metadata(&fixture.lock).unwrap();
    assert_ne!(
        (replacement.dev(), replacement.ino()),
        (original.dev(), original.ino())
    );

    assert_eq!(daemon.begin_drain(), DrainTransition::Fenced);
    assert_eq!(daemon.complete_drain(), DrainCompletion::Fenced);
    let status = daemon.probe_operational_status();
    assert_eq!(status.lifecycle, DaemonLifecycle::Fenced);
    assert_eq!(status.health, DaemonHealth::Fenced);
    assert_eq!(status.readiness, DaemonReadiness::NotReady);
    assert_eq!(daemon.probe_operational_status(), status);
    assert_eq!(daemon.usage().health_failures, 1);

    assert_eq!(
        daemon.serve_next(),
        Err(vellum_daemon::DaemonServeError::OwnershipLost)
    );
    drop(daemon);
    assert_eq!(fs::read(&fixture.lock).unwrap(), b"replacement");
    assert!(!fixture.socket.exists());
}

#[test]
fn daemon_refuses_symlink_and_multiply_linked_lock_files() {
    let symlink_fixture = Fixture::new("alpha");
    std::os::unix::fs::symlink(&symlink_fixture.source, &symlink_fixture.lock).unwrap();
    assert!(matches!(
        LocalDaemon::bind(
            &symlink_fixture.socket,
            workspace(&symlink_fixture.source),
            limits()
        ),
        Err(DaemonBuildError::LockUnsafe)
    ));
    assert!(!symlink_fixture.socket.exists());

    let linked_fixture = Fixture::new("alpha");
    fs::write(&linked_fixture.lock, b"").unwrap();
    fs::set_permissions(&linked_fixture.lock, fs::Permissions::from_mode(0o600)).unwrap();
    let alias = linked_fixture.directory.join("lock-alias");
    fs::hard_link(&linked_fixture.lock, &alias).unwrap();
    assert!(matches!(
        LocalDaemon::bind(
            &linked_fixture.socket,
            workspace(&linked_fixture.source),
            limits()
        ),
        Err(DaemonBuildError::LockUnsafe)
    ));
    assert!(!linked_fixture.socket.exists());
    fs::remove_file(alias).unwrap();
}

#[test]
fn cleanup_never_removes_a_replacement_path() {
    let fixture = Fixture::new("alpha");
    let daemon = LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), limits()).unwrap();
    fs::remove_file(&fixture.socket).unwrap();
    fs::write(&fixture.socket, "replacement").unwrap();
    drop(daemon);
    assert_eq!(fs::read_to_string(&fixture.socket).unwrap(), "replacement");
}

#[test]
fn daemon_refuses_public_parent_and_existing_socket_without_deleting_either() {
    let public = Fixture::new("alpha");
    fs::set_permissions(&public.directory, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(
        LocalDaemon::bind(&public.socket, workspace(&public.source), limits()),
        Err(DaemonBuildError::ParentPermissions)
    ));
    fs::set_permissions(&public.directory, fs::Permissions::from_mode(0o700)).unwrap();

    fs::write(&public.socket, "sentinel").unwrap();
    assert!(matches!(
        LocalDaemon::bind(&public.socket, workspace(&public.source), limits()),
        Err(DaemonBuildError::SocketExists)
    ));
    assert_eq!(fs::read_to_string(&public.socket).unwrap(), "sentinel");
}

#[test]
fn installed_binary_exposes_content_free_usage_without_starting_a_service() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vellum-daemon"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("vellum-daemon --socket"));
    assert!(stdout.contains("vellum-daemon --status"));
    assert!(stdout.contains("The default is read-only"));
    assert!(stdout.contains("--allow-add-file"));
    assert!(stdout.contains("--allow-add-directory"));
    assert!(stdout.contains("--allow-remove"));
    assert!(stdout.contains("--export-copy vdo1.OPERATION_HEX"));
    assert!(stdout.contains("ABSOLUTE_UTF8_DESTINATION"));
    assert!(stdout.contains("checkpointed before publication or reply"));
    assert!(stdout.contains("not principal authentication"));
}

#[test]
fn installed_export_cli_refuses_noncanonical_operation_and_non_utf8_destination_pre_effect() {
    let fixture = Fixture::new("alpha");
    let raw_operation = Command::new(env!("CARGO_BIN_EXE_vellum-daemon"))
        .arg("--socket")
        .arg(&fixture.socket)
        .arg("--control")
        .arg(fixture.directory.join("control.sock"))
        .arg("--export-copy")
        .arg("11111111111111111111111111111111")
        .arg("1")
        .arg("0")
        .arg(fixture.directory.join("export.txt"))
        .output()
        .unwrap();
    assert!(!raw_operation.status.success());
    assert!(!fixture.socket.exists());

    let non_utf8_destination = OsString::from_vec(vec![b'/', b't', b'm', b'p', b'/', 0xff]);
    assert!(
        DaemonExportPolicyV1::exact_create_exclusive(
            DurableOperationRefV1::new([0x11; 16]).unwrap(),
            1,
            0,
            PathBuf::from(non_utf8_destination.clone()),
            FilesystemLimits {
                max_resources: 1,
                max_bytes: 1,
                max_stage_attempts: 1,
                max_metadata_bytes: 1,
            },
            4,
        )
        .is_none()
    );
    let non_utf8 = Command::new(env!("CARGO_BIN_EXE_vellum-daemon"))
        .arg("--socket")
        .arg(&fixture.socket)
        .arg("--control")
        .arg(fixture.directory.join("control.sock"))
        .arg("--export-copy")
        .arg("vdo1.11111111111111111111111111111111")
        .arg("1")
        .arg("0")
        .arg(non_utf8_destination)
        .output()
        .unwrap();
    assert!(!non_utf8.status.success());
    assert!(!fixture.socket.exists());
}

#[test]
fn installed_credential_cli_authenticates_without_disclosing_its_path() {
    let fixture = Fixture::new("alpha");
    let credential = fixture.directory.join("private-credential-sentinel");
    let provisioned = Command::new(env!("CARGO_BIN_EXE_vellum-daemon"))
        .arg("--provision-credential")
        .arg(&credential)
        .output()
        .unwrap();
    assert!(provisioned.status.success());
    let provisioned_stdout = String::from_utf8(provisioned.stdout).unwrap();
    let provisioned_stderr = String::from_utf8(provisioned.stderr).unwrap();
    let credential_text = credential.to_string_lossy();
    assert!(!provisioned_stdout.contains(credential_text.as_ref()));
    assert!(!provisioned_stderr.contains(credential_text.as_ref()));

    let mut child = Command::new(env!("CARGO_BIN_EXE_vellum-daemon"))
        .arg("--socket")
        .arg(&fixture.socket)
        .arg("--control")
        .arg(&fixture.control)
        .arg("--credential")
        .arg(&credential)
        .arg("--allow-edit")
        .arg("--allow-save")
        .arg("--")
        .arg(&fixture.source)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut ready = String::new();
    stdout.read_line(&mut ready).unwrap();
    assert_eq!(ready, "vellum-daemon ready\n");

    let record = LocalCredentialStoreV1::default().load(&credential).unwrap();
    let connector = AuthenticatedUnixConnector::with_provider_and_timeout(
        fixture.socket.clone(),
        LocalCredentialClaimProviderV1::new(record),
        Duration::from_secs(2),
    )
    .unwrap();
    let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
    let mut client = Client::connect(transport).unwrap();
    assert!(matches!(
        client.workspace_inventory().unwrap(),
        ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(_))
    ));
    client.disconnect().map_err(|(_, error)| error).unwrap();

    let shutdown = Command::new(env!("CARGO_BIN_EXE_vellum-daemon"))
        .arg("--shutdown")
        .arg(&fixture.control)
        .output()
        .unwrap();
    assert!(shutdown.status.success());
    assert!(child.wait().unwrap().success());
    let mut remainder = String::new();
    stdout.read_to_string(&mut remainder).unwrap();
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(!remainder.contains(credential_text.as_ref()));
    assert!(!stderr.contains(credential_text.as_ref()));
    fs::remove_file(credential).unwrap();
}

#[test]
fn invalid_credential_refuses_before_workspace_and_socket_effects_without_path_echo() {
    let fixture = Fixture::new("alpha");
    let credential = fixture
        .directory
        .join("invalid-private-credential-sentinel");
    fs::write(&credential, b"not a credential").unwrap();
    fs::set_permissions(&credential, fs::Permissions::from_mode(0o600)).unwrap();
    let before = fs::read_dir(&fixture.directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<std::collections::BTreeSet<_>>();
    let output = Command::new(env!("CARGO_BIN_EXE_vellum-daemon"))
        .arg("--socket")
        .arg(&fixture.socket)
        .arg("--control")
        .arg(&fixture.control)
        .arg("--credential")
        .arg(&credential)
        .arg("--")
        .arg(&fixture.source)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    let credential_text = credential.to_string_lossy();
    assert!(!stdout.contains(credential_text.as_ref()));
    assert!(!stderr.contains(credential_text.as_ref()));
    assert!(stderr.contains("credential loading refused"));
    assert!(!fixture.socket.exists());
    assert!(!fixture.control.exists());
    let after = fs::read_dir(&fixture.directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(after, before);
    fs::remove_file(credential).unwrap();
}

#[test]
fn installed_status_cli_reads_real_daemon_without_disclosing_workspace_content() {
    let fixture = Fixture::new("private-source-sentinel");
    let (mut child, mut stdout) = spawn_daemon(&fixture);

    let status = Command::new(env!("CARGO_BIN_EXE_vellum-daemon"))
        .arg("--status")
        .arg(&fixture.control)
        .output()
        .unwrap();
    assert!(status.status.success());
    let body = String::from_utf8(status.stdout).unwrap();
    assert!(body.starts_with(
        "{\"version\":2,\"liveness\":\"live\",\"lifecycle\":\"running\",\"health\":\"healthy\",\"readiness\":\"ready\""
    ));
    assert!(body.contains("\"utilization\":"));
    assert!(body.contains("\"saturation\":"));
    assert!(body.contains("\"retired_durable_operations\":"));
    assert!(body.contains("\"retired_durable_operation_capacity\":"));
    assert!(!body.contains("private-source-sentinel"));
    assert!(!body.contains(fixture.directory.to_string_lossy().as_ref()));

    let shutdown = Command::new(env!("CARGO_BIN_EXE_vellum-daemon"))
        .arg("--shutdown")
        .arg(&fixture.control)
        .output()
        .unwrap();
    assert!(shutdown.status.success());
    assert!(child.wait().unwrap().success());
    let mut remainder = String::new();
    stdout.read_to_string(&mut remainder).unwrap();
    assert!(remainder.contains("vellum-daemon stopped"));
}

fn spawn_daemon(fixture: &Fixture) -> (Child, BufReader<std::process::ChildStdout>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vellum-daemon"))
        .arg("--socket")
        .arg(&fixture.socket)
        .arg("--control")
        .arg(&fixture.control)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut ready = String::new();
    stdout.read_line(&mut ready).unwrap();
    assert_eq!(ready, "vellum-daemon ready\n");
    (child, stdout)
}

#[test]
fn crash_residue_is_recovered_only_from_exact_evidence_and_sigterm_drains() {
    let fixture = Fixture::new("unused");
    let (mut crashed, _stdout) = spawn_daemon(&fixture);
    rustix::process::kill_process(
        rustix::process::Pid::from_child(&crashed),
        rustix::process::Signal::KILL,
    )
    .unwrap();
    assert!(!crashed.wait().unwrap().success());
    assert!(
        fs::symlink_metadata(&fixture.socket)
            .unwrap()
            .file_type()
            .is_socket()
    );
    assert!(
        fs::symlink_metadata(&fixture.control)
            .unwrap()
            .file_type()
            .is_socket()
    );

    let (mut restarted, mut stdout) = spawn_daemon(&fixture);
    rustix::process::kill_process(
        rustix::process::Pid::from_child(&restarted),
        rustix::process::Signal::TERM,
    )
    .unwrap();
    assert!(restarted.wait().unwrap().success());
    let mut remainder = String::new();
    stdout.read_to_string(&mut remainder).unwrap();
    assert!(remainder.contains("vellum-daemon stopped"));
    assert!(!fixture.socket.exists());
    assert!(!fixture.control.exists());
}
