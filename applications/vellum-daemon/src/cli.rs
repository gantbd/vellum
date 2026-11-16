#[cfg(target_os = "macos")]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    use crate::{
        DaemonExportPolicyV1, DaemonHealth, DaemonLifecycle, DaemonLimits, DaemonReadiness,
        DaemonServeError, DrainCompletion, DrainTransition, LocalDaemon,
        StderrOperationalEventSink, compose_startup_workspace, request_shutdown, request_status_v2,
    };
    use std::{ffi::OsString, os::unix::ffi::OsStrExt, path::PathBuf, time::Duration};
    use vellum_adapter_local_authentication::{LocalCredentialStoreV1, LocalCredentialVerifierV1};
    use vellum_adapter_local_fs::{FilesystemLimits, LocalFilesystemFactory};
    use vellum_host::authority::{AuthorityActionSetV1, AuthorityActionV1};

    #[derive(Debug)]
    struct CliError(&'static str);
    impl std::fmt::Display for CliError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.0)
        }
    }
    impl std::error::Error for CliError {}

    fn parse_operation(value: &str) -> Result<vellum_protocol::DurableOperationRefV1, CliError> {
        let Some(hex) = value.strip_prefix("vdo1.") else {
            return Err(CliError("export operation identity refused"));
        };
        if hex.len() != 32
            || !hex
                .as_bytes()
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        {
            return Err(CliError("export operation identity refused"));
        }
        let mut bytes = [0_u8; 16];
        for (index, pair) in hex.as_bytes().chunks_exact(2).enumerate() {
            let text = std::str::from_utf8(pair)
                .map_err(|_| CliError("export operation identity refused"))?;
            bytes[index] = u8::from_str_radix(text, 16)
                .map_err(|_| CliError("export operation identity refused"))?;
        }
        vellum_protocol::DurableOperationRefV1::new(bytes)
            .ok_or(CliError("export operation identity refused"))
    }

    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == "--help" {
        println!(
            "vellum-daemon --socket PRIVATE_SOCKET --control PRIVATE_CONTROL_SOCKET [--credential PRIVATE_CREDENTIAL] [ACTION_CEILING_FLAGS...] [--] [EXISTING_FILE_OR_DIRECTORY...]"
        );
        println!("vellum-daemon --shutdown PRIVATE_CONTROL_SOCKET");
        println!("vellum-daemon --status PRIVATE_CONTROL_SOCKET");
        println!("vellum-daemon --provision-credential PRIVATE_CREDENTIAL");
        println!(
            "vellum-daemon --stage-credential-rotation CURRENT_PRIVATE_CREDENTIAL NEXT_PRIVATE_CREDENTIAL"
        );
        println!(
            "Action ceilings: --allow-add-file --allow-add-directory --allow-remove --allow-edit --allow-save --allow-retire-receipt. Exact authenticated export: --export-copy vdo1.OPERATION_HEX DOCUMENT_ORDINAL EXPECTED_REVISION ABSOLUTE_UTF8_DESTINATION. The default is read-only."
        );
        println!(
            "Startup and accepted runtime membership are checkpointed before publication or reply. An indeterminate runtime checkpoint fences the daemon and yields an outcome-unknown exchange."
        );
        println!(
            "Without --credential the daemon remains owner-only offline. A configured credential authenticates one stable local subject but grants no action; the independent action ceiling still applies. Credential material is accepted only through a private file, never argv or environment."
        );
        println!(
            "Private socket ownership is endpoint access control, not principal authentication."
        );
        println!(
            "Workspace scope and allocator high-water marks are committed in that private directory before the service socket is published. Missing identity after first installation refuses startup."
        );
        return Ok(());
    }
    if args.len() == 2 && args[0] == "--provision-credential" {
        LocalCredentialStoreV1::default()
            .provision(PathBuf::from(&args[1]).as_path())
            .map_err(|_| CliError("credential provisioning refused"))?;
        println!("vellum-daemon credential provisioned");
        return Ok(());
    }
    if args.len() == 3 && args[0] == "--stage-credential-rotation" {
        LocalCredentialStoreV1::default()
            .stage_rotation(
                PathBuf::from(&args[1]).as_path(),
                PathBuf::from(&args[2]).as_path(),
            )
            .map_err(|_| CliError("credential rotation staging refused"))?;
        println!("vellum-daemon credential rotation staged");
        return Ok(());
    }
    if args.len() == 2 && args[0] == "--shutdown" {
        request_shutdown(PathBuf::from(&args[1]).as_path(), Duration::from_secs(5))
            .map_err(|_| CliError("shutdown request refused"))?;
        println!("vellum-daemon shutdown requested");
        return Ok(());
    }
    if args.len() == 2 && args[0] == "--status" {
        let status = request_status_v2(PathBuf::from(&args[1]).as_path(), Duration::from_secs(5))
            .map_err(|_| CliError("status request refused"))?;
        let lifecycle = match status.lifecycle {
            DaemonLifecycle::Running => "running",
            DaemonLifecycle::Draining => "draining",
            DaemonLifecycle::Stopped => "stopped",
            DaemonLifecycle::Fenced => "fenced",
        };
        let health = match status.health {
            DaemonHealth::Healthy => "healthy",
            DaemonHealth::Fenced => "fenced",
        };
        let readiness = match status.readiness {
            DaemonReadiness::Ready => "ready",
            DaemonReadiness::NotReady => "not_ready",
        };
        let u = status.utilization;
        let s = status.saturation;
        println!(
            concat!(
                "{{\"version\":2,\"liveness\":\"live\",\"lifecycle\":\"{}\",",
                "\"health\":\"{}\",\"readiness\":\"{}\",\"utilization\":{{",
                "\"in_flight_connections\":{},\"in_flight_capacity\":{},",
                "\"active_clients\":{},\"client_capacity\":{},",
                "\"issued_clients\":{},\"issued_client_capacity\":{},",
                "\"retired_clients\":{},\"retained_requests\":{},",
                "\"request_capacity\":{},\"retained_witnesses\":{},",
                "\"witness_capacity\":{},\"durable_receipts\":{},",
                "\"durable_receipt_capacity\":{},",
                "\"retired_durable_operations\":{},",
                "\"retired_durable_operation_capacity\":{}}},\"saturation\":{{",
                "\"in_flight\":{},\"clients\":{},\"client_lifetime\":{},",
                "\"requests\":{},\"witnesses\":{},\"durable_receipts\":{},",
                "\"retired_durable_operations\":{},",
                "\"daemon_counters\":{},\"service_counters\":{},\"counters\":{},",
                "\"request_latency\":{},\"workspace_identity\":{},\"events\":{},",
                "\"authentication\":{}}}}}"
            ),
            lifecycle,
            health,
            readiness,
            u.in_flight_connections,
            u.in_flight_capacity,
            u.active_clients,
            u.client_capacity,
            u.issued_clients,
            u.issued_client_capacity,
            u.retired_clients,
            u.retained_requests,
            u.request_capacity,
            u.retained_witnesses,
            u.witness_capacity,
            u.durable_receipts,
            u.durable_receipt_capacity,
            u.retired_durable_operations,
            u.retired_durable_operation_capacity,
            s.in_flight,
            s.clients,
            s.client_lifetime,
            s.requests,
            s.witnesses,
            s.durable_receipts,
            s.retired_durable_operations,
            s.daemon_counters,
            s.service_counters,
            s.counters,
            s.request_latency,
            s.workspace_identity,
            s.events,
            s.authentication,
        );
        return Ok(());
    }
    if args.len() < 4 || args[0] != "--socket" || args[2] != "--control" {
        return Err(Box::new(CliError(
            "usage: vellum-daemon --socket PRIVATE_SOCKET --control PRIVATE_CONTROL_SOCKET [EXISTING_FILE_OR_DIRECTORY...]",
        )));
    }
    let socket = PathBuf::from(&args[1]);
    let control_socket = PathBuf::from(&args[3]);
    let mut actions = AuthorityActionSetV1::read_only();
    let mut credential = None;
    let mut export_copy = None;
    let mut roots = Vec::new();
    let mut options = true;
    let mut index = 4;
    while index < args.len() {
        let argument = &args[index];
        if options && argument == "--" {
            options = false;
        } else if options && argument == "--allow-edit" {
            actions = actions.join(AuthorityActionSetV1::singleton(
                AuthorityActionV1::EditSource,
            ));
        } else if options && argument == "--allow-save" {
            actions = actions.join(AuthorityActionSetV1::singleton(
                AuthorityActionV1::PersistSource,
            ));
        } else if options && argument == "--allow-add-file" {
            actions = actions.join(AuthorityActionSetV1::singleton(
                AuthorityActionV1::AdmitFile,
            ));
        } else if options && argument == "--allow-add-directory" {
            actions = actions.join(AuthorityActionSetV1::singleton(
                AuthorityActionV1::DiscoverDirectory,
            ));
        } else if options && argument == "--allow-remove" {
            actions = actions.join(AuthorityActionSetV1::singleton(
                AuthorityActionV1::RetireMembership,
            ));
        } else if options && argument == "--allow-retire-receipt" {
            actions = actions.join(AuthorityActionSetV1::singleton(
                AuthorityActionV1::RetireReceipt,
            ));
        } else if options && argument == "--credential" {
            index += 1;
            let Some(path) = args.get(index) else {
                return Err(Box::new(CliError(
                    "credential option requires a private file",
                )));
            };
            if path.as_bytes().starts_with(b"--") || credential.is_some() {
                return Err(Box::new(CliError("credential option refused")));
            }
            credential = Some(PathBuf::from(path));
        } else if options && argument == "--export-copy" {
            let Some(operation) = args.get(index + 1).and_then(|value| value.to_str()) else {
                return Err(Box::new(CliError("export operation identity refused")));
            };
            let Some(document) = args.get(index + 2).and_then(|value| value.to_str()) else {
                return Err(Box::new(CliError("export document ordinal refused")));
            };
            let Some(revision) = args.get(index + 3).and_then(|value| value.to_str()) else {
                return Err(Box::new(CliError("export revision refused")));
            };
            let Some(destination) = args.get(index + 4).and_then(|value| value.to_str()) else {
                return Err(Box::new(CliError("export destination refused")));
            };
            if export_copy.is_some() {
                return Err(Box::new(CliError("duplicate export policy refused")));
            }
            export_copy = Some((
                parse_operation(operation)?,
                document
                    .parse::<u64>()
                    .map_err(|_| CliError("export document ordinal refused"))?,
                revision
                    .parse::<u64>()
                    .map_err(|_| CliError("export revision refused"))?,
                PathBuf::from(destination),
            ));
            index += 4;
        } else if options && argument.as_bytes().starts_with(b"--credential=") {
            return Err(Box::new(CliError(
                "credential option requires a separate path",
            )));
        } else if options && argument.as_bytes().starts_with(b"--") {
            return Err(Box::new(CliError("unknown action-ceiling option")));
        } else {
            options = false;
            roots.push(PathBuf::from(argument));
        }
        index += 1;
    }
    if socket.as_os_str().as_bytes().len() > 100
        || control_socket.as_os_str().as_bytes().len() > 100
    {
        return Err(Box::new(CliError(
            "the socket path exceeds the reference limit",
        )));
    }

    let limits = DaemonLimits::reference();
    if export_copy.is_some() && credential.is_none() {
        return Err(Box::new(CliError(
            "export policy requires principal authentication",
        )));
    }
    // Authentication configuration is validated before workspace discovery,
    // identity reservation, or socket publication can produce an effect.
    let verifier = credential
        .as_deref()
        .map(|path| {
            LocalCredentialStoreV1::default()
                .load(path)
                .map(LocalCredentialVerifierV1::new)
        })
        .transpose()
        .map_err(|_| CliError("credential loading refused"))?;
    let filesystem_limits = FilesystemLimits {
        max_resources: 16,
        max_bytes: 65_536,
        max_stage_attempts: 16,
        max_metadata_bytes: 65_536,
    };
    let workspace = compose_startup_workspace(
        &roots,
        vellum_workspace::reference_workspace_profile(
            LocalFilesystemFactory::new(filesystem_limits).erase(),
        ),
        limits,
    )
    .map_err(|_| CliError("workspace construction refused"))?;
    let mut daemon = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &socket, workspace, limits, actions,
    )
    .map_err(|error| {
        eprintln!(
            "vellum-daemon: binding_refused class={}",
            error.operational_class()
        );
        CliError("daemon binding refused")
    })?;
    if let Some((operation, document, revision, destination)) = export_copy {
        let policy = DaemonExportPolicyV1::exact_create_exclusive(
            operation,
            document,
            revision,
            destination,
            filesystem_limits,
            8,
        )
        .ok_or(CliError("export policy refused"))?;
        daemon = daemon
            .with_exact_export_policy(policy)
            .map_err(|_| CliError("export policy installation refused"))?;
    }
    if let Some(verifier) = verifier {
        daemon = daemon.require_authentication(verifier, (), limits.max_authentication_attempts);
    }
    let mut daemon = daemon.with_event_sink(StderrOperationalEventSink);
    let control = daemon
        .bind_shutdown_control(&control_socket)
        .map_err(|_| CliError("shutdown control binding refused"))?;
    let signal_supervisor = daemon
        .install_signal_supervisor(&control_socket)
        .map_err(|_| CliError("signal supervision installation refused"))?;
    let control_thread = std::thread::spawn(move || control.wait());
    println!("vellum-daemon ready");
    loop {
        match daemon.serve_next() {
            Ok(()) | Err(DaemonServeError::Request(_)) | Err(DaemonServeError::Authentication) => {}
            Err(DaemonServeError::ShutdownRequested) => {
                if daemon.begin_drain() != DrainTransition::Started {
                    return Err(Box::new(CliError("daemon drain could not start")));
                }
                if daemon.complete_drain() != DrainCompletion::Complete {
                    return Err(Box::new(CliError("daemon drain did not quiesce")));
                }
                drop(signal_supervisor);
                match control_thread.join() {
                    Ok(Ok(())) => {
                        println!("vellum-daemon stopped");
                        return Ok(());
                    }
                    Ok(Err(_)) | Err(_) => {
                        return Err(Box::new(CliError("shutdown control failed")));
                    }
                }
            }
            Err(DaemonServeError::OwnershipLost) => {
                return Err(Box::new(CliError("daemon ownership was lost")));
            }
            Err(
                DaemonServeError::NotReady
                | DaemonServeError::Capacity
                | DaemonServeError::LifecycleInvariant,
            ) => {
                return Err(Box::new(CliError("daemon is not accepting work")));
            }
            Err(DaemonServeError::Accept) => {
                return Err(Box::new(CliError("socket acceptance failed")));
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn run() {
    eprintln!("vellum-daemon: the qualified local-filesystem profile currently requires macOS");
    std::process::exit(2);
}
