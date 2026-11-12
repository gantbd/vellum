//! Opt-in, bounded Unix connection/frame qualification for the daemon composition.
#![cfg(target_os = "macos")]
#![forbid(unsafe_code)]

use std::{
    env, fs,
    io::Write,
    net::Shutdown,
    os::unix::{fs::PermissionsExt, net::UnixStream},
    path::PathBuf,
    sync::mpsc,
    thread,
    time::Duration,
};
use vellum_adapter_local_fs::{FilesystemLimits, LocalFilesystemFactory};
use vellum_daemon::{DaemonLimits, DaemonServeError, LocalDaemon};
use vellum_editor::Workspace;
use vellum_host::workspace::WorkspaceIdentityLimitsV1;
use vellum_service::ServiceLimits;
use vellum_transport::{TransportError, TransportLimits, read_frame, write_frame};

const DEFAULT_CYCLES: usize = 16;

fn setting(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

struct Fixture {
    directory: PathBuf,
    source: PathBuf,
    socket: PathBuf,
}

impl Fixture {
    fn new(cycle: usize) -> Result<Self, &'static str> {
        let directory = env::temp_dir().join(format!(
            "vellum-qualification-daemon-{}-{cycle}",
            std::process::id()
        ));
        fs::create_dir(&directory).map_err(|_| "fixture directory creation failed")?;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
            .map_err(|_| "fixture permission setup failed")?;
        let source = directory.join("source.txt");
        fs::write(&source, "qualification").map_err(|_| "fixture source creation failed")?;
        let socket = directory.join("service.sock");
        Ok(Self {
            directory,
            source,
            socket,
        })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket);
        let _ = fs::remove_file(self.directory.join(".vellum-daemon.lock"));
        let _ = fs::remove_file(&self.source);
        let _ = fs::remove_dir(&self.directory);
    }
}

fn workspace(path: &PathBuf) -> Result<Workspace, &'static str> {
    Workspace::open(
        std::slice::from_ref(path),
        vellum_editor::reference_workspace_profile(
            LocalFilesystemFactory::new(FilesystemLimits {
                max_resources: 8,
                max_bytes: 4_096,
                max_stage_attempts: 8,
                max_metadata_bytes: 4_096,
            })
            .erase(),
        ),
    )
    .map_err(|_| "workspace construction failed")
}

fn limits() -> DaemonLimits {
    DaemonLimits {
        service: ServiceLimits::reference(),
        transport: TransportLimits::reference(),
        io_timeout: Duration::from_secs(2),
        max_in_flight_connections: 1,
        max_startup_roots: 8,
        max_authentication_attempts: 32,
        workspace_identity: WorkspaceIdentityLimitsV1 {
            max_documents: 256,
            max_roots: 256,
            max_encoded_bytes: 1_024,
        },
        request_journal: vellum_host::request_journal::RequestJournalLimitsV1 {
            max_entries: 256,
            max_retired_operations: 4096,
            max_request_bytes: 65_600,
            max_reply_bytes: 65_600,
            max_encoded_bytes: 131_264,
        },
        request_latency: None,
    }
}

fn valid_exchange(daemon: &mut LocalDaemon, socket: PathBuf, request: Vec<u8>) -> Vec<u8> {
    let transport = limits().transport;
    let worker = thread::spawn(move || -> Result<Vec<u8>, &'static str> {
        let mut stream = UnixStream::connect(socket).map_err(|_| "connection failed")?;
        write_frame(&mut stream, &request, transport.max_frame_bytes)
            .map_err(|_| "request write failed")?;
        read_frame(&mut stream, transport.max_frame_bytes).map_err(|_| "reply read failed")
    });
    daemon
        .serve_next()
        .unwrap_or_else(|_| panic!("valid daemon exchange failed"));
    worker
        .join()
        .unwrap_or_else(|_| panic!("valid exchange worker panicked"))
        .unwrap_or_else(|error| panic!("{error}"))
}

fn invalid_exchange(
    daemon: &mut LocalDaemon,
    socket: PathBuf,
    bytes: Vec<u8>,
    expected: DaemonServeError,
) {
    let (ready_sender, ready_receiver) = mpsc::sync_channel(0);
    let (done_sender, done_receiver) = mpsc::sync_channel(0);
    let worker = thread::spawn(move || -> Result<(), &'static str> {
        let mut stream = UnixStream::connect(socket).map_err(|_| "connection failed")?;
        stream
            .write_all(&bytes)
            .map_err(|_| "invalid frame write failed")?;
        stream
            .shutdown(Shutdown::Write)
            .map_err(|_| "write shutdown failed")?;
        ready_sender
            .send(())
            .map_err(|_| "exchange readiness failed")?;
        done_receiver
            .recv()
            .map_err(|_| "exchange completion failed")
    });
    ready_receiver
        .recv()
        .unwrap_or_else(|_| panic!("invalid exchange did not become ready"));
    assert_eq!(daemon.serve_next(), Err(expected));
    done_sender
        .send(())
        .unwrap_or_else(|_| panic!("invalid exchange completion failed"));
    worker
        .join()
        .unwrap_or_else(|_| panic!("invalid exchange worker panicked"))
        .unwrap_or_else(|error| panic!("{error}"));
}

#[test]
#[ignore = "opt-in deterministic qualification; run make check-fuzz-soak"]
fn bounded_daemon_soak_closes_valid_malformed_oversize_and_truncated_connections() {
    let cycles = usize::try_from(setting("VELLUM_DAEMON_CYCLES", DEFAULT_CYCLES as u64)).unwrap();
    assert!(
        (1..=1_000).contains(&cycles),
        "daemon cycle budget is invalid"
    );

    for cycle in 0..cycles {
        let fixture = Fixture::new(cycle).unwrap_or_else(|error| panic!("cycle {cycle}: {error}"));
        let mut daemon = LocalDaemon::bind(
            &fixture.socket,
            workspace(&fixture.source).unwrap_or_else(|error| panic!("cycle {cycle}: {error}")),
            limits(),
        )
        .unwrap_or_else(|error| panic!("cycle {cycle}: daemon bind failed: {error:?}"));

        let connected = valid_exchange(&mut daemon, fixture.socket.clone(), vec![1, 1]);
        assert_eq!(
            connected.len(),
            10,
            "cycle {cycle}: connect reply length drift"
        );
        assert_eq!(
            &connected[..2],
            &[1, 0x81],
            "cycle {cycle}: connect reply tag drift"
        );
        let client = u64::from_be_bytes(
            connected[2..]
                .try_into()
                .unwrap_or_else(|_| panic!("cycle {cycle}: connect identity width drift")),
        );
        assert_ne!(client, 0, "cycle {cycle}: zero client identity");
        let mut disconnect = vec![1, 3];
        disconnect.extend_from_slice(&client.to_be_bytes());
        assert_eq!(
            valid_exchange(&mut daemon, fixture.socket.clone(), disconnect),
            vec![1, 0x84],
            "cycle {cycle}: disconnect reply drift"
        );

        let mut malformed = Vec::new();
        write_frame(&mut malformed, &[0xff], limits().transport.max_frame_bytes).unwrap();
        invalid_exchange(
            &mut daemon,
            fixture.socket.clone(),
            malformed,
            DaemonServeError::Request(TransportError::Malformed),
        );

        let oversize = u32::try_from(limits().transport.max_frame_bytes + 1)
            .unwrap()
            .to_be_bytes()
            .to_vec();
        invalid_exchange(
            &mut daemon,
            fixture.socket.clone(),
            oversize,
            DaemonServeError::Request(TransportError::FrameLimit),
        );

        let mut truncated = 2_u32.to_be_bytes().to_vec();
        truncated.push(1);
        invalid_exchange(
            &mut daemon,
            fixture.socket.clone(),
            truncated,
            DaemonServeError::Request(TransportError::Truncated),
        );

        let usage = daemon.usage();
        assert_eq!(
            usage.accepted_connections, 5,
            "cycle {cycle}: accept count drift"
        );
        assert_eq!(
            usage.completed_connections, 2,
            "cycle {cycle}: completion count drift"
        );
        assert_eq!(
            usage.request_failures, 3,
            "cycle {cycle}: failure count drift"
        );
        assert_eq!(
            usage.request_deadline_exhausted, 0,
            "cycle {cycle}: false deadline evidence"
        );
        assert_eq!(
            daemon.service_usage().active_clients,
            0,
            "cycle {cycle}: client leaked"
        );
        drop(daemon);
        drop(fixture);
    }
}
