//! Local Unix service composition for Vellum.
#![forbid(unsafe_code)]

#[cfg(unix)]
use std::{
    fs, io,
    os::unix::{
        fs::{FileTypeExt, MetadataExt, PermissionsExt},
        net::UnixListener,
    },
    path::{Path, PathBuf},
    time::Duration,
};
#[cfg(unix)]
use vellum_editor::Workspace;
#[cfg(unix)]
use vellum_service::{Service, ServiceBuildError, ServiceLimits, ServiceUsage};
#[cfg(unix)]
use vellum_transport::{TransportError, TransportLimits, serve_one};

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DaemonLimits {
    pub service: ServiceLimits,
    pub transport: TransportLimits,
    pub io_timeout: Duration,
}

#[cfg(unix)]
impl DaemonLimits {
    pub const fn reference() -> Self {
        Self {
            service: ServiceLimits::reference(),
            transport: TransportLimits::reference(),
            io_timeout: Duration::from_secs(5),
        }
    }
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaemonBuildError {
    InvalidLimits,
    ParentMissing,
    ParentNotDirectory,
    ParentPermissions,
    SocketExists,
    Inspection,
    Bind,
    SocketPermissions,
    Service(ServiceBuildError),
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaemonServeError {
    Accept,
    Request(TransportError),
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DaemonUsage {
    pub accepted_connections: u64,
    pub completed_connections: u64,
    pub accept_failures: u64,
    pub request_failures: u64,
    pub counter_saturated: bool,
}

#[cfg(unix)]
pub struct LocalDaemon {
    listener: UnixListener,
    service: Service,
    limits: DaemonLimits,
    usage: DaemonUsage,
    socket: SocketGuard,
}

#[cfg(unix)]
impl LocalDaemon {
    pub fn bind(
        socket: &Path,
        workspace: Workspace,
        limits: DaemonLimits,
    ) -> Result<Self, DaemonBuildError> {
        validate_limits(limits)?;
        let parent = socket.parent().ok_or(DaemonBuildError::ParentMissing)?;
        let parent_metadata = fs::symlink_metadata(parent).map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => DaemonBuildError::ParentMissing,
            _ => DaemonBuildError::Inspection,
        })?;
        if !parent_metadata.file_type().is_dir() || parent_metadata.file_type().is_symlink() {
            return Err(DaemonBuildError::ParentNotDirectory);
        }
        if parent_metadata.permissions().mode() & 0o077 != 0 {
            return Err(DaemonBuildError::ParentPermissions);
        }
        match fs::symlink_metadata(socket) {
            Ok(_) => return Err(DaemonBuildError::SocketExists),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return Err(DaemonBuildError::Inspection),
        }
        let service = Service::new(workspace, limits.service).map_err(DaemonBuildError::Service)?;
        let listener = UnixListener::bind(socket).map_err(|_| DaemonBuildError::Bind)?;
        let metadata = fs::symlink_metadata(socket).map_err(|_| DaemonBuildError::Inspection)?;
        if !metadata.file_type().is_socket() {
            return Err(DaemonBuildError::Inspection);
        }
        let socket = SocketGuard {
            path: socket.to_path_buf(),
            device: metadata.dev(),
            inode: metadata.ino(),
        };
        fs::set_permissions(&socket.path, fs::Permissions::from_mode(0o600))
            .map_err(|_| DaemonBuildError::SocketPermissions)?;
        Ok(Self {
            listener,
            service,
            limits,
            usage: DaemonUsage::default(),
            socket,
        })
    }

    pub fn serve_next(&mut self) -> Result<(), DaemonServeError> {
        let (mut stream, _) = match self.listener.accept() {
            Ok(connection) => connection,
            Err(_) => {
                bump(
                    &mut self.usage.accept_failures,
                    &mut self.usage.counter_saturated,
                );
                return Err(DaemonServeError::Accept);
            }
        };
        bump(
            &mut self.usage.accepted_connections,
            &mut self.usage.counter_saturated,
        );
        if stream
            .set_read_timeout(Some(self.limits.io_timeout))
            .and_then(|()| stream.set_write_timeout(Some(self.limits.io_timeout)))
            .is_err()
        {
            bump(
                &mut self.usage.request_failures,
                &mut self.usage.counter_saturated,
            );
            return Err(DaemonServeError::Request(TransportError::Io));
        }
        if let Err(error) = serve_one(&mut self.service, &mut stream, self.limits.transport) {
            bump(
                &mut self.usage.request_failures,
                &mut self.usage.counter_saturated,
            );
            return Err(DaemonServeError::Request(error));
        }
        bump(
            &mut self.usage.completed_connections,
            &mut self.usage.counter_saturated,
        );
        Ok(())
    }

    pub fn usage(&self) -> DaemonUsage {
        self.usage
    }

    pub fn service_usage(&self) -> ServiceUsage {
        self.service.usage()
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket.path
    }
}

#[cfg(unix)]
struct SocketGuard {
    path: PathBuf,
    device: u64,
    inode: u64,
}

#[cfg(unix)]
impl Drop for SocketGuard {
    fn drop(&mut self) {
        let Ok(metadata) = fs::symlink_metadata(&self.path) else {
            return;
        };
        if metadata.file_type().is_socket()
            && metadata.dev() == self.device
            && metadata.ino() == self.inode
        {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(unix)]
fn validate_limits(limits: DaemonLimits) -> Result<(), DaemonBuildError> {
    if limits.io_timeout.is_zero()
        || limits.transport.max_frame_bytes == 0
        || limits.transport.wire.max_frame_bytes == 0
        || limits.transport.wire.max_frame_bytes > limits.transport.max_frame_bytes
    {
        return Err(DaemonBuildError::InvalidLimits);
    }
    Ok(())
}

#[cfg(unix)]
fn bump(value: &mut u64, saturated: &mut bool) {
    let next = value.saturating_add(1);
    if next == *value {
        *saturated = true;
    }
    *value = next;
}
