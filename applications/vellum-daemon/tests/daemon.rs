#![cfg(target_os = "macos")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use vellum_adapter_local_fs::{FilesystemLimits, LocalFilesystemFactory};
use vellum_client::Client;
use vellum_daemon::{DaemonBuildError, DaemonLimits, LocalDaemon};
use vellum_editor::Workspace;
use vellum_protocol::{OutcomeV1, ReplyV1};
use vellum_service::ServiceLimits;
use vellum_transport::{RemoteTransport, TransportLimits, UnixConnector};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    directory: PathBuf,
    source: PathBuf,
    socket: PathBuf,
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
        Self {
            directory,
            source: source_path,
            socket,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket);
        let _ = fs::remove_file(&self.source);
        let _ = fs::remove_dir(&self.directory);
    }
}

fn workspace(path: &PathBuf) -> Workspace {
    Workspace::open(
        std::slice::from_ref(path),
        vellum_editor::reference_workspace_profile(
            LocalFilesystemFactory::new(FilesystemLimits {
                max_resources: 8,
                max_bytes: 4096,
                max_stage_attempts: 8,
                max_metadata_bytes: 4096,
            })
            .erase(),
        ),
    )
    .unwrap()
}

fn limits() -> DaemonLimits {
    DaemonLimits {
        service: ServiceLimits::reference(),
        transport: TransportLimits::reference(),
        io_timeout: Duration::from_secs(2),
    }
}

#[test]
fn unix_daemon_serves_remote_snapshot_and_reclaims_the_client() {
    let fixture = Fixture::new("alpha");
    let mut daemon =
        LocalDaemon::bind(&fixture.socket, workspace(&fixture.source), limits()).unwrap();
    assert_eq!(
        fs::symlink_metadata(&fixture.socket)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
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
    drop(daemon);
    assert!(!fixture.socket.exists());
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
    assert!(stdout.contains("not principal authentication"));
}
