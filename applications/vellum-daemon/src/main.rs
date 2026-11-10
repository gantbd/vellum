#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{ffi::OsString, os::unix::ffi::OsStrExt, path::PathBuf};
    use vellum_adapter_local_fs::{FilesystemLimits, LocalFilesystemFactory};
    use vellum_daemon::{DaemonLimits, DaemonServeError, LocalDaemon};
    use vellum_editor::Workspace;

    #[derive(Debug)]
    struct CliError(&'static str);
    impl std::fmt::Display for CliError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.0)
        }
    }
    impl std::error::Error for CliError {}

    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == "--help" {
        println!("vellum-daemon --socket PRIVATE_SOCKET EXISTING_FILE...");
        println!(
            "The socket parent must already exist with no group/other permissions. Existing socket paths are never removed. The local-user socket boundary is not principal authentication or Aegis authorization."
        );
        return Ok(());
    }
    if args.len() < 3 || args[0] != "--socket" {
        return Err(Box::new(CliError(
            "usage: vellum-daemon --socket PRIVATE_SOCKET EXISTING_FILE...",
        )));
    }
    let socket = PathBuf::from(&args[1]);
    let files: Vec<PathBuf> = args[2..].iter().map(PathBuf::from).collect();
    if files.len() > 16 {
        return Err(Box::new(CliError("at most 16 source files are supported")));
    }
    if socket.as_os_str().as_bytes().len() > 100
        || files
            .iter()
            .any(|path| path.as_os_str().as_bytes().len() > 4096)
    {
        return Err(Box::new(CliError("a path exceeds the reference limit")));
    }

    let workspace = Workspace::open(
        &files,
        vellum_editor::reference_workspace_profile(
            LocalFilesystemFactory::new(FilesystemLimits {
                max_resources: 16,
                max_bytes: 65_536,
                max_stage_attempts: 16,
                max_metadata_bytes: 65_536,
            })
            .erase(),
        ),
    )
    .map_err(|_| CliError("workspace construction refused"))?;
    let mut daemon = LocalDaemon::bind(&socket, workspace, DaemonLimits::reference())
        .map_err(|_| CliError("daemon binding refused"))?;
    println!("vellum-daemon ready");
    loop {
        match daemon.serve_next() {
            Ok(()) | Err(DaemonServeError::Request(_)) => {}
            Err(DaemonServeError::Accept) => {
                return Err(Box::new(CliError("socket acceptance failed")));
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("vellum-daemon: the qualified local-filesystem profile currently requires macOS");
    std::process::exit(2);
}
