#![cfg(target_os = "macos")]

use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc,
    time::Duration,
};
use vellum_adapter_local_authentication::{
    LocalCredentialClaimProviderV1, LocalCredentialStoreV1, LocalCredentialVerifierV1,
};
use vellum_adapter_local_fs::{FilesystemLimits, LocalFilesystemFactory};
use vellum_daemon::{AuthenticatedUnixConnector, DaemonExportPolicyV1, DaemonLimits, LocalDaemon};
use vellum_editor::Workspace;
use vellum_host::authority::{AuthorityActionSetV1, AuthorityActionV1};
use vellum_mcp::{
    CHANGES_TOOL, DURABLE_EXECUTE_TOOL, DURABLE_RECEIPT_TOOL, DURABLE_RETIRE_TOOL,
    DURABLE_STATUS_TOOL, MEMBERSHIP_TOOL, McpLimits, McpServer, PROTOCOL_VERSION,
    REPLACE_AND_SAVE_TOOL, REPLACE_TOOL, RESOLVE_LOCATOR_TOOL, SNAPSHOT_TOOL, WORKSPACE_EDIT_TOOL,
};
use vellum_sdk_client::{
    client::Client,
    protocol::DurableOperationRefV1,
    transport::{RemoteTransport, TransportLimits},
};
use vellum_service::ServiceLimits;

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": PROTOCOL_VERSION,
        "io.modelcontextprotocol/clientCapabilities": {},
        "io.modelcontextprotocol/clientInfo": {"name": "test", "version": "1"}
    })
}

fn durable_call(name: &str, arguments: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "jsonrpc":"2.0", "id":name, "method":"tools/call",
        "params":{"_meta":meta(), "name":name, "arguments":arguments}
    }))
    .unwrap()
}

fn built_daemon_binary() -> PathBuf {
    let executable = std::env::current_exe().unwrap();
    let profile = executable
        .parent()
        .and_then(std::path::Path::parent)
        .expect("integration test executable must be below PROFILE/deps");
    let daemon = profile.join("vellum-daemon");
    assert!(
        daemon.is_file(),
        "build both product binaries before process qualification"
    );
    daemon
}

fn extraction_catalog() -> (Vec<String>, Vec<String>) {
    let contract =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../release/vellum-mcp-extraction.json");
    let contract: Value = serde_json::from_slice(&fs::read(contract).unwrap()).unwrap();
    let names = |field: &str| {
        contract["protocol"][field]
            .as_array()
            .unwrap_or_else(|| panic!("protocol.{field} must be an array"))
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .unwrap_or_else(|| panic!("protocol.{field} entries must be strings"))
                    .to_owned()
            })
            .collect::<Vec<_>>()
    };
    (names("ordinaryTools"), names("authenticatedAdditions"))
}

#[test]
#[ignore = "run through make check-shipped-mcp so both product executables are current"]
fn shipped_daemon_and_mcp_binaries_share_the_authenticated_durable_catalog() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::path::PathBuf::from("/private/tmp").join(format!(
        "vellum-mcp-process-auth-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    let source = directory.join("source.txt");
    let socket = directory.join("service.sock");
    let control = directory.join("control.sock");
    let credential = directory.join("private-credential-sentinel");
    fs::write(&source, "alpha").unwrap();

    let daemon_binary = built_daemon_binary();
    let provisioned = Command::new(&daemon_binary)
        .arg("--provision-credential")
        .arg(&credential)
        .output()
        .unwrap();
    assert!(provisioned.status.success());
    let credential_text = credential.to_string_lossy();
    assert!(!String::from_utf8_lossy(&provisioned.stdout).contains(credential_text.as_ref()));
    assert!(!String::from_utf8_lossy(&provisioned.stderr).contains(credential_text.as_ref()));

    let mut daemon = Command::new(&daemon_binary)
        .arg("--socket")
        .arg(&socket)
        .arg("--control")
        .arg(&control)
        .arg("--credential")
        .arg(&credential)
        .arg("--allow-edit")
        .arg("--allow-save")
        .arg("--")
        .arg(&source)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut daemon_stdout = BufReader::new(daemon.stdout.take().unwrap());
    let mut ready = String::new();
    daemon_stdout.read_line(&mut ready).unwrap();
    assert_eq!(ready, "vellum-daemon ready\n");

    let mut mcp = Command::new(env!("CARGO_BIN_EXE_vellum-mcp"))
        .arg("--socket")
        .arg(&socket)
        .arg("--credential")
        .arg(&credential)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let request = json!({
        "jsonrpc":"2.0", "id":"tools", "method":"tools/list",
        "params":{"_meta":meta()}
    });
    let mut mcp_stdin = mcp.stdin.take().unwrap();
    writeln!(mcp_stdin, "{request}").unwrap();
    let resolve = json!({
        "jsonrpc":"2.0", "id":"resolve", "method":"tools/call",
        "params":{"_meta":meta(), "name":RESOLVE_LOCATOR_TOOL,
            "arguments":{"locator":source.to_str().unwrap()}}
    });
    writeln!(mcp_stdin, "{resolve}").unwrap();
    mcp_stdin.flush().unwrap();
    let mut mcp_stdout = BufReader::new(mcp.stdout.take().unwrap());
    let mut listed = String::new();
    mcp_stdout.read_line(&mut listed).unwrap();
    let mut resolved = String::new();
    mcp_stdout.read_line(&mut resolved).unwrap();
    drop(mcp_stdin);
    let status = mcp.wait().unwrap();
    let mut mcp_stderr = String::new();
    mcp.stderr
        .take()
        .unwrap()
        .read_to_string(&mut mcp_stderr)
        .unwrap();
    if !status.success() {
        let shutdown = Command::new(&daemon_binary)
            .arg("--shutdown")
            .arg(&control)
            .output()
            .unwrap();
        let daemon_status = daemon.wait().unwrap();
        let mut daemon_remainder = String::new();
        daemon_stdout.read_to_string(&mut daemon_remainder).unwrap();
        let mut daemon_stderr = String::new();
        daemon
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut daemon_stderr)
            .unwrap();
        panic!(
            "MCP failed: {mcp_stderr}; resolved={resolved}; shutdown={:?}; daemon={daemon_status:?}; daemon stdout={daemon_remainder}; daemon stderr={daemon_stderr}",
            shutdown.status
        );
    }
    let listed: Value = serde_json::from_str(&listed).unwrap();
    let resolved: Value = serde_json::from_str(&resolved).unwrap();
    assert_eq!(
        resolved["result"]["structuredContent"]["kind"],
        "locator_resolved"
    );
    assert!(resolved["result"]["structuredContent"]["document"].is_number());
    assert_eq!(
        resolved["result"]["structuredContent"]["root"]["kind"],
        "file"
    );
    assert!(!resolved.to_string().contains(source.to_str().unwrap()));
    let names = listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();
    let (ordinary_contract, authenticated_contract) = extraction_catalog();
    let ordinary_runtime = [
        SNAPSHOT_TOOL,
        REPLACE_TOOL,
        REPLACE_AND_SAVE_TOOL,
        WORKSPACE_EDIT_TOOL,
        CHANGES_TOOL,
        RESOLVE_LOCATOR_TOOL,
        MEMBERSHIP_TOOL,
    ];
    let authenticated_runtime = [
        DURABLE_EXECUTE_TOOL,
        DURABLE_RECEIPT_TOOL,
        DURABLE_RETIRE_TOOL,
        DURABLE_STATUS_TOOL,
    ];
    assert_eq!(
        ordinary_contract,
        ordinary_runtime.map(str::to_owned),
        "extraction ordinary catalog drifted from runtime"
    );
    assert_eq!(
        authenticated_contract,
        authenticated_runtime.map(str::to_owned),
        "extraction authenticated catalog drifted from runtime"
    );
    assert_eq!(
        names,
        ordinary_runtime
            .into_iter()
            .chain(authenticated_runtime)
            .collect::<Vec<_>>()
    );
    assert!(!mcp_stderr.contains(credential_text.as_ref()));

    let shutdown = Command::new(&daemon_binary)
        .arg("--shutdown")
        .arg(&control)
        .output()
        .unwrap();
    assert!(shutdown.status.success());
    assert!(daemon.wait().unwrap().success());
    let mut daemon_remainder = String::new();
    daemon_stdout.read_to_string(&mut daemon_remainder).unwrap();
    let mut daemon_stderr = String::new();
    daemon
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut daemon_stderr)
        .unwrap();
    assert!(!daemon_remainder.contains(credential_text.as_ref()));
    assert!(!daemon_stderr.contains(credential_text.as_ref()));

    // The same shipped peers retain the categorical profile distinction: an
    // owner-only daemon has no authenticated subject and therefore cannot
    // advertise durable operation tools.
    let mut owner_daemon = Command::new(&daemon_binary)
        .arg("--socket")
        .arg(&socket)
        .arg("--control")
        .arg(&control)
        .arg("--")
        .arg(&source)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut owner_stdout = BufReader::new(owner_daemon.stdout.take().unwrap());
    let mut owner_ready = String::new();
    owner_stdout.read_line(&mut owner_ready).unwrap();
    assert_eq!(owner_ready, "vellum-daemon ready\n");
    let mut owner_mcp = Command::new(env!("CARGO_BIN_EXE_vellum-mcp"))
        .arg("--socket")
        .arg(&socket)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut owner_stdin = owner_mcp.stdin.take().unwrap();
    writeln!(owner_stdin, "{request}").unwrap();
    owner_stdin.flush().unwrap();
    let mut owner_mcp_stdout = BufReader::new(owner_mcp.stdout.take().unwrap());
    let mut owner_listed = String::new();
    owner_mcp_stdout.read_line(&mut owner_listed).unwrap();
    drop(owner_stdin);
    assert!(owner_mcp.wait().unwrap().success());
    let owner_listed: Value = serde_json::from_str(&owner_listed).unwrap();
    let owner_names = owner_listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(owner_names, ordinary_runtime);
    let owner_shutdown = Command::new(&daemon_binary)
        .arg("--shutdown")
        .arg(&control)
        .output()
        .unwrap();
    assert!(owner_shutdown.status.success());
    assert!(owner_daemon.wait().unwrap().success());

    let _ = fs::remove_file(&credential);
    let _ = fs::remove_file(&source);
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&control);
    let _ = fs::remove_file(directory.join(".vellum-daemon.lock"));
    let _ = fs::remove_file(directory.join("workspace-identity-v1"));
    let _ = fs::remove_dir_all(directory.join(".vellum-workspace-v1"));
    let _ = fs::remove_dir_all(directory.join(".vellum-request-journal-v1"));
    let _ = fs::remove_dir_all(directory.join(".vellum-durable-operation-v1"));
    let _ = fs::remove_dir_all(directory.join(".vellum-durable-operation-v2"));
    let _ = fs::remove_dir(&directory);
}

#[test]
fn shipped_processes_recover_a_lost_durable_reply_after_both_restart() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::path::PathBuf::from("/private/tmp").join(format!(
        "vellum-mcp-process-restart-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    let source = directory.join("source.txt");
    let socket = directory.join("service.sock");
    let control = directory.join("control.sock");
    let credential = directory.join("credential");
    fs::write(&source, "alpha").unwrap();

    let daemon_binary = built_daemon_binary();
    assert!(
        Command::new(&daemon_binary)
            .arg("--provision-credential")
            .arg(&credential)
            .output()
            .unwrap()
            .status
            .success()
    );

    let spawn_daemon = || {
        let mut daemon = Command::new(&daemon_binary)
            .arg("--socket")
            .arg(&socket)
            .arg("--control")
            .arg(&control)
            .arg("--credential")
            .arg(&credential)
            .arg("--allow-edit")
            .arg("--allow-save")
            .arg("--")
            .arg(&source)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdout = BufReader::new(daemon.stdout.take().unwrap());
        let mut ready = String::new();
        stdout.read_line(&mut ready).unwrap();
        if ready != "vellum-daemon ready\n" {
            let mut stderr = String::new();
            daemon
                .stderr
                .take()
                .unwrap()
                .read_to_string(&mut stderr)
                .unwrap();
            let status = daemon.wait().unwrap();
            panic!("daemon startup failed ({status}): {stderr}");
        }
        (daemon, stdout)
    };
    let shutdown_daemon = |daemon: &mut std::process::Child| {
        let shutdown = Command::new(&daemon_binary)
            .arg("--shutdown")
            .arg(&control)
            .output()
            .unwrap();
        assert!(shutdown.status.success());
        assert!(daemon.wait().unwrap().success());
    };
    let spawn_mcp = || {
        Command::new(env!("CARGO_BIN_EXE_vellum-mcp"))
            .arg("--socket")
            .arg(&socket)
            .arg("--credential")
            .arg(&credential)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    };

    let (mut first_daemon, _first_stdout) = spawn_daemon();
    let mut first_mcp = spawn_mcp();
    let mut first_stdin = first_mcp.stdin.take().unwrap();
    let mut first_stdout = BufReader::new(first_mcp.stdout.take().unwrap());
    let list = json!({
        "jsonrpc":"2.0", "id":"resources", "method":"resources/list",
        "params":{"_meta":meta()}
    });
    writeln!(first_stdin, "{list}").unwrap();
    first_stdin.flush().unwrap();
    let mut listed = String::new();
    first_stdout.read_line(&mut listed).unwrap();
    let listed: Value = serde_json::from_str(&listed).unwrap();
    let document_uri = listed["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|resource| resource["uri"].as_str())
        .find(|uri| uri.contains("/document/"))
        .unwrap();
    let fields: Vec<_> = document_uri.split('/').collect();
    let intent = json!({
        "operation_ref": "vdo1.73737373737373737373737373737373",
        "intent": "replace_document_and_save",
        "workspace": fields[3].parse::<u64>().unwrap(),
        "document": fields[5].parse::<u64>().unwrap(),
        "expected_revision": 0,
        "start": 0,
        "end": 5,
        "inserted": "omega"
    });
    let execute = String::from_utf8(durable_call(DURABLE_EXECUTE_TOOL, intent.clone())).unwrap();
    writeln!(first_stdin, "{execute}").unwrap();
    first_stdin.flush().unwrap();

    // The effect is observed only through its declared filesystem product.
    // The harness never consumes the MCP reply and terminates the connector,
    // so recovery must use the persisted operation reference after both
    // shipped processes have restarted.
    let mut applied = false;
    for _ in 0..200 {
        if fs::read_to_string(&source).unwrap() == "omega" {
            applied = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(applied, "durable effect did not reach its declared product");
    first_mcp.kill().unwrap();
    assert!(!first_mcp.wait().unwrap().success());
    drop(first_stdin);
    drop(first_stdout);
    shutdown_daemon(&mut first_daemon);

    fs::write(&source, "external").unwrap();
    let (mut restarted_daemon, _restarted_stdout) = spawn_daemon();
    let mut restarted_mcp = spawn_mcp();
    let mut restarted_stdin = restarted_mcp.stdin.take().unwrap();
    let mut restarted_stdout = BufReader::new(restarted_mcp.stdout.take().unwrap());
    let lookup = String::from_utf8(durable_call(DURABLE_RECEIPT_TOOL, intent)).unwrap();
    writeln!(restarted_stdin, "{lookup}").unwrap();
    restarted_stdin.flush().unwrap();
    let mut receipt = String::new();
    restarted_stdout.read_line(&mut receipt).unwrap();
    let receipt: Value = serde_json::from_str(&receipt).unwrap();
    assert_eq!(
        receipt["result"]["structuredContent"]["kind"],
        "document_replaced_and_saved"
    );
    assert_eq!(receipt["result"]["structuredContent"]["text"], "omega");
    assert_eq!(fs::read_to_string(&source).unwrap(), "external");

    drop(restarted_stdin);
    assert!(restarted_mcp.wait().unwrap().success());
    shutdown_daemon(&mut restarted_daemon);

    let _ = fs::remove_file(&credential);
    let _ = fs::remove_file(&source);
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&control);
    let _ = fs::remove_file(directory.join(".vellum-daemon.lock"));
    let _ = fs::remove_file(directory.join("workspace-identity-v1"));
    let _ = fs::remove_dir_all(directory.join(".vellum-workspace-v1"));
    let _ = fs::remove_dir_all(directory.join(".vellum-request-journal-v1"));
    let _ = fs::remove_dir_all(directory.join(".vellum-durable-operation-v1"));
    let _ = fs::remove_dir_all(directory.join(".vellum-durable-operation-v2"));
    let _ = fs::remove_dir(&directory);
}

#[test]
fn stdio_adapter_reaches_the_real_daemon_and_disconnects_on_eof() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::path::PathBuf::from("/private/tmp")
        .join(format!("vellum-mcp-live-{}-{nonce}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    let source = directory.join("source.txt");
    let socket = directory.join("service.sock");
    fs::write(&source, "alpha").unwrap();
    let (ready_tx, ready_rx) = mpsc::sync_channel(0);
    let daemon_socket = socket.clone();
    let daemon_source = source.clone();
    let daemon = std::thread::spawn(move || {
        let workspace = Workspace::open(
            std::slice::from_ref(&daemon_source),
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
        .unwrap();
        let mut daemon = LocalDaemon::bind(
            &daemon_socket,
            workspace,
            DaemonLimits {
                service: ServiceLimits::reference(),
                transport: TransportLimits::reference(),
                io_timeout: Duration::from_secs(2),
                ..DaemonLimits::reference()
            },
        )
        .unwrap();
        ready_tx.send(()).unwrap();
        for _ in 0..5 {
            daemon.serve_next().unwrap();
        }
        assert_eq!(daemon.service_usage().active_clients, 0);
    });
    ready_rx.recv().unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_vellum-mcp"))
        .arg("--socket")
        .arg(&socket)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let list = json!({
        "jsonrpc":"2.0", "id":"list", "method":"resources/list",
        "params":{"_meta":meta()}
    });
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    writeln!(stdin, "{list}").unwrap();
    stdin.flush().unwrap();
    let mut listed = String::new();
    stdout.read_line(&mut listed).unwrap();
    let listed: Value = serde_json::from_str(&listed).unwrap();
    let uri = listed["result"]["resources"][0]["uri"]
        .as_str()
        .unwrap()
        .to_owned();
    let read = json!({
        "jsonrpc":"2.0", "id":"read", "method":"resources/read",
        "params":{"_meta":meta(), "uri":uri}
    });
    writeln!(stdin, "{read}").unwrap();
    drop(stdin);
    let mut remaining = String::new();
    stdout.read_to_string(&mut remaining).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    let read_reply: Value = serde_json::from_str(remaining.trim()).unwrap();
    assert!(uri.starts_with("vellum://workspace/"));
    assert_eq!(read_reply["result"]["contents"][0]["text"], "alpha");
    daemon.join().unwrap();

    let _ = fs::remove_file(&source);
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_dir(&directory);
}

#[test]
fn authenticated_mcp_receipt_lookup_and_retirement_survive_restart_without_reexecution() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::path::PathBuf::from("/private/tmp")
        .join(format!("vellum-mcp-durable-{}-{nonce}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    let source = directory.join("source.txt");
    let socket = directory.join("service.sock");
    let credential = directory.join("credential");
    fs::write(&source, "alpha").unwrap();
    LocalCredentialStoreV1::default()
        .provision(&credential)
        .unwrap();
    let actions = AuthorityActionSetV1::read_only()
        .join(AuthorityActionSetV1::singleton(
            AuthorityActionV1::EditSource,
        ))
        .join(AuthorityActionSetV1::singleton(
            AuthorityActionV1::PersistSource,
        ))
        .join(AuthorityActionSetV1::singleton(
            AuthorityActionV1::RetireReceipt,
        ));
    let workspace = || {
        Workspace::open(
            std::slice::from_ref(&source),
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
    };
    let daemon_limits = DaemonLimits {
        service: ServiceLimits::reference(),
        transport: TransportLimits::reference(),
        io_timeout: Duration::from_secs(2),
        ..DaemonLimits::reference()
    };
    let operation_ref = "vdo1.53535353535353535353535353535353";
    let mut first = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &socket,
        workspace(),
        daemon_limits,
        actions,
    )
    .unwrap()
    .require_authentication(
        LocalCredentialVerifierV1::new(
            LocalCredentialStoreV1::default().load(&credential).unwrap(),
        ),
        (),
        4,
    );
    let first_socket = socket.clone();
    let first_credential = credential.clone();
    let first_client = std::thread::spawn(move || {
        let connector = AuthenticatedUnixConnector::with_provider_and_timeout(
            first_socket,
            LocalCredentialClaimProviderV1::new(
                LocalCredentialStoreV1::default()
                    .load(&first_credential)
                    .unwrap(),
            ),
            Duration::from_secs(2),
        )
        .unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let client = Client::connect(transport).unwrap();
        let mut mcp = McpServer::new_authenticated(client, McpLimits::reference()).unwrap();
        let list = serde_json::to_vec(&json!({
            "jsonrpc":"2.0", "id":"resources", "method":"resources/list",
            "params":{"_meta":meta()}
        }))
        .unwrap();
        let listed: Value = serde_json::from_slice(&mcp.handle(&list).unwrap()).unwrap();
        let document_uri = listed["result"]["resources"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|resource| resource["uri"].as_str())
            .find(|uri| uri.contains("/document/"))
            .unwrap();
        let fields: Vec<_> = document_uri.split('/').collect();
        let intent = json!({
            "operation_ref": operation_ref,
            "intent": "replace_document_and_save",
            "workspace": fields[3].parse::<u64>().unwrap(),
            "document": fields[5].parse::<u64>().unwrap(),
            "expected_revision": 0,
            "start": 0,
            "end": 5,
            "inserted": "omega"
        });
        // The harness persisted `operation_ref` before this first effect. It
        // intentionally discards the valid MCP response to model loss above
        // the authenticated native exchange.
        let _discarded = mcp
            .handle(&durable_call(DURABLE_EXECUTE_TOOL, intent.clone()))
            .unwrap();
        assert!(mcp.into_client().disconnect().is_ok());
        intent
    });
    for _ in 0..4 {
        first.serve_next().unwrap();
    }
    let intent = first_client.join().unwrap();
    assert_eq!(fs::read_to_string(&source).unwrap(), "omega");
    assert_eq!(first.service_usage().durable_effect_requests, 1);
    drop(first);

    fs::write(&source, "external").unwrap();
    let mut restarted = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &socket,
        workspace(),
        daemon_limits,
        actions,
    )
    .unwrap()
    .require_authentication(
        LocalCredentialVerifierV1::new(
            LocalCredentialStoreV1::default().load(&credential).unwrap(),
        ),
        (),
        7,
    );
    let restarted_socket = socket.clone();
    let restarted_credential = credential.clone();
    let lookup_intent = intent.clone();
    let lookup = std::thread::spawn(move || {
        let connector = AuthenticatedUnixConnector::with_provider_and_timeout(
            restarted_socket,
            LocalCredentialClaimProviderV1::new(
                LocalCredentialStoreV1::default()
                    .load(&restarted_credential)
                    .unwrap(),
            ),
            Duration::from_secs(2),
        )
        .unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let client = Client::connect(transport).unwrap();
        let mut mcp = McpServer::new_authenticated(client, McpLimits::reference()).unwrap();
        let complete: Value = serde_json::from_slice(
            &mcp.handle(&durable_call(DURABLE_RECEIPT_TOOL, lookup_intent.clone()))
                .unwrap(),
        )
        .unwrap();
        let mut changed = lookup_intent.clone();
        changed["inserted"] = json!("changed");
        let conflict: Value = serde_json::from_slice(
            &mcp.handle(&durable_call(DURABLE_RECEIPT_TOOL, changed))
                .unwrap(),
        )
        .unwrap();
        let absent: Value = serde_json::from_slice(
            &mcp.handle(&durable_call(
                DURABLE_RECEIPT_TOOL,
                json!({
                    "operation_ref": "vdo1.54545454545454545454545454545454",
                    "intent": "add_file",
                    "locator": "/absent"
                }),
            ))
            .unwrap(),
        )
        .unwrap();
        let retired: Value = serde_json::from_slice(
            &mcp.handle(&durable_call(DURABLE_RETIRE_TOOL, lookup_intent.clone()))
                .unwrap(),
        )
        .unwrap();
        let after_retirement: Value = serde_json::from_slice(
            &mcp.handle(&durable_call(DURABLE_RECEIPT_TOOL, lookup_intent))
                .unwrap(),
        )
        .unwrap();
        assert!(mcp.into_client().disconnect().is_ok());
        (complete, conflict, absent, retired, after_retirement)
    });
    // Authenticated native connect, five MCP/native operations, and native
    // disconnect are seven daemon exchanges; authentication is part of connect.
    for _ in 0..7 {
        restarted.serve_next().unwrap();
    }
    let (complete, conflict, absent, retired, after_retirement) = lookup.join().unwrap();
    assert_eq!(
        complete["result"]["structuredContent"]["kind"],
        "document_replaced_and_saved"
    );
    assert_eq!(complete["result"]["structuredContent"]["text"], "omega");
    assert_eq!(
        conflict["result"]["structuredContent"]["code"],
        "request_conflict"
    );
    assert_eq!(
        absent["result"]["structuredContent"]["kind"],
        "absent_or_foreign"
    );
    assert_eq!(retired["result"]["structuredContent"]["kind"], "retired");
    assert_eq!(
        after_retirement["result"]["structuredContent"]["kind"],
        "absent_or_foreign"
    );
    assert_eq!(fs::read_to_string(&source).unwrap(), "external");
    assert_eq!(restarted.service_usage().durable_effect_requests, 0);
    assert_eq!(restarted.usage().durable_operation_lookups, 4);
    assert_eq!(restarted.usage().durable_retirement_requests, 1);
    assert_eq!(restarted.usage().durable_retirement_retired, 1);
    drop(restarted);

    // A separately provisioned credential has another random subject. A
    // retired operation owned by the first subject and a genuinely unknown
    // operation must have the same externally visible absence shape.
    let foreign_credential = directory.join("foreign-credential");
    LocalCredentialStoreV1::default()
        .provision(&foreign_credential)
        .unwrap();
    let mut foreign = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &socket,
        workspace(),
        daemon_limits,
        actions,
    )
    .unwrap()
    .require_authentication(
        LocalCredentialVerifierV1::new(
            LocalCredentialStoreV1::default()
                .load(&foreign_credential)
                .unwrap(),
        ),
        (),
        4,
    );
    let foreign_socket = socket.clone();
    let foreign_path = foreign_credential.clone();
    let retained_intent = intent;
    let foreign_lookup = std::thread::spawn(move || {
        let connector = AuthenticatedUnixConnector::with_provider_and_timeout(
            foreign_socket,
            LocalCredentialClaimProviderV1::new(
                LocalCredentialStoreV1::default()
                    .load(&foreign_path)
                    .unwrap(),
            ),
            Duration::from_secs(2),
        )
        .unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let client = Client::connect(transport).unwrap();
        let mut mcp = McpServer::new_authenticated(client, McpLimits::reference()).unwrap();
        let retained: Value = serde_json::from_slice(
            &mcp.handle(&durable_call(DURABLE_RECEIPT_TOOL, retained_intent))
                .unwrap(),
        )
        .unwrap();
        let unknown: Value = serde_json::from_slice(
            &mcp.handle(&durable_call(
                DURABLE_RECEIPT_TOOL,
                json!({
                    "operation_ref": "vdo1.55555555555555555555555555555555",
                    "intent": "add_file",
                    "locator": "/unknown"
                }),
            ))
            .unwrap(),
        )
        .unwrap();
        assert!(mcp.into_client().disconnect().is_ok());
        (retained, unknown)
    });
    // Authenticated native connect, two lookups, and native disconnect are four
    // daemon exchanges; authentication is part of connect.
    for _ in 0..4 {
        foreign.serve_next().unwrap();
    }
    let (retained, unknown) = foreign_lookup.join().unwrap();
    assert_eq!(
        retained["result"]["structuredContent"],
        unknown["result"]["structuredContent"]
    );
    assert_eq!(
        retained["result"]["structuredContent"]["kind"],
        "absent_or_foreign"
    );

    let _ = fs::remove_file(&source);
    let _ = fs::remove_file(&credential);
    let _ = fs::remove_file(&foreign_credential);
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_dir_all(directory.join(".vellum-workspace-v1"));
    let _ = fs::remove_dir_all(directory.join(".vellum-request-journal-v1"));
    let _ = fs::remove_dir_all(directory.join(".vellum-durable-operation-v1"));
    let _ = fs::remove_dir_all(directory.join(".vellum-durable-operation-v2"));
    let _ = fs::remove_file(directory.join(".vellum-daemon.lock"));
    let _ = fs::remove_dir(&directory);
}

#[test]
fn authenticated_mcp_export_lost_reply_recovers_without_republication_or_state_change() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::path::PathBuf::from("/private/tmp").join(format!(
        "vellum-mcp-export-durable-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    let source = directory.join("source.txt");
    let destination = directory.join("exported-copy.txt");
    let changed_destination = directory.join("changed-export.txt");
    let socket = directory.join("service.sock");
    let credential = directory.join("credential");
    fs::write(&source, "alpha").unwrap();
    LocalCredentialStoreV1::default()
        .provision(&credential)
        .unwrap();

    let workspace = || {
        Workspace::open(
            std::slice::from_ref(&source),
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
    };
    let daemon_limits = DaemonLimits {
        service: ServiceLimits::reference(),
        transport: TransportLimits::reference(),
        io_timeout: Duration::from_secs(2),
        ..DaemonLimits::reference()
    };
    let operation = DurableOperationRefV1::new([0x63; 16]).unwrap();
    let operation_ref = "vdo1.63636363636363636363636363636363";
    let export_policy = || {
        DaemonExportPolicyV1::exact_create_exclusive(
            operation,
            1,
            0,
            destination.clone(),
            FilesystemLimits {
                max_resources: 1,
                max_bytes: 4096,
                max_stage_attempts: 8,
                max_metadata_bytes: 4096,
            },
            8,
        )
        .unwrap()
    };
    let verifier = || {
        LocalCredentialVerifierV1::new(LocalCredentialStoreV1::default().load(&credential).unwrap())
    };
    let provider = |path: &std::path::Path| {
        LocalCredentialClaimProviderV1::new(LocalCredentialStoreV1::default().load(path).unwrap())
    };

    let mut first = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &socket,
        workspace(),
        daemon_limits,
        AuthorityActionSetV1::read_only(),
    )
    .unwrap()
    .with_exact_export_policy(export_policy())
    .unwrap()
    .require_authentication(verifier(), (), 16);
    let first_socket = socket.clone();
    let first_credential = credential.clone();
    let first_destination = destination.clone();
    let first_client = std::thread::spawn(move || {
        let connector = AuthenticatedUnixConnector::with_provider_and_timeout(
            first_socket,
            provider(&first_credential),
            Duration::from_secs(2),
        )
        .unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let client = Client::connect(transport).unwrap();
        let mut mcp = McpServer::new_authenticated(client, McpLimits::reference()).unwrap();
        let list = serde_json::to_vec(&json!({
            "jsonrpc":"2.0", "id":"resources", "method":"resources/list",
            "params":{"_meta":meta()}
        }))
        .unwrap();
        let listed: Value = serde_json::from_slice(&mcp.handle(&list).unwrap()).unwrap();
        let resources = listed["result"]["resources"].clone();
        let document_uri = resources
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|resource| resource["uri"].as_str())
            .find(|uri| uri.contains("/document/"))
            .unwrap()
            .to_owned();
        let fields: Vec<_> = document_uri.split('/').collect();
        let read = serde_json::to_vec(&json!({
            "jsonrpc":"2.0", "id":"read", "method":"resources/read",
            "params":{"_meta":meta(), "uri":document_uri}
        }))
        .unwrap();
        let read: Value = serde_json::from_slice(&mcp.handle(&read).unwrap()).unwrap();
        let content = read["result"]["contents"][0].clone();
        let intent = json!({
            "operation_ref": operation_ref,
            "intent": "export_document_copy_exclusive",
            "workspace": fields[3].parse::<u64>().unwrap(),
            "document": fields[5].parse::<u64>().unwrap(),
            "expected_revision": 0,
            "destination": first_destination.to_str().unwrap()
        });
        // The harness persisted the operation reference before execution and
        // deliberately discards this complete response above the native
        // exchange. Recovery must therefore use only the durable lookup path.
        let _discarded = mcp
            .handle(&durable_call(DURABLE_EXECUTE_TOOL, intent.clone()))
            .unwrap();
        assert!(mcp.into_client().disconnect().is_ok());
        (resources, content, intent)
    });
    for _ in 0..6 {
        first.serve_next().unwrap();
    }
    let (resources_before, content_before, intent) = first_client.join().unwrap();
    assert_eq!(content_before["text"], "alpha");
    assert_eq!(content_before["_meta"]["vellum/revision"], 0);
    assert_eq!(fs::read_to_string(&source).unwrap(), "alpha");
    assert_eq!(fs::read_to_string(&destination).unwrap(), "alpha");
    assert_eq!(first.service_usage().durable_effect_requests, 1);
    drop(first);

    let mut restarted = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &socket,
        workspace(),
        daemon_limits,
        AuthorityActionSetV1::read_only(),
    )
    .unwrap()
    .with_exact_export_policy(export_policy())
    .unwrap()
    .require_authentication(verifier(), (), 16);
    let restarted_socket = socket.clone();
    let restarted_credential = credential.clone();
    let lookup_intent = intent.clone();
    let changed_path = changed_destination.clone();
    let lookup = std::thread::spawn(move || {
        let connector = AuthenticatedUnixConnector::with_provider_and_timeout(
            restarted_socket,
            provider(&restarted_credential),
            Duration::from_secs(2),
        )
        .unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let client = Client::connect(transport).unwrap();
        let mut mcp = McpServer::new_authenticated(client, McpLimits::reference()).unwrap();
        let list = serde_json::to_vec(&json!({
            "jsonrpc":"2.0", "id":"resources", "method":"resources/list",
            "params":{"_meta":meta()}
        }))
        .unwrap();
        let listed: Value = serde_json::from_slice(&mcp.handle(&list).unwrap()).unwrap();
        let resources = listed["result"]["resources"].clone();
        let document_uri = resources
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|resource| resource["uri"].as_str())
            .find(|uri| uri.contains("/document/"))
            .unwrap()
            .to_owned();
        let read = serde_json::to_vec(&json!({
            "jsonrpc":"2.0", "id":"read", "method":"resources/read",
            "params":{"_meta":meta(), "uri":document_uri}
        }))
        .unwrap();
        let read: Value = serde_json::from_slice(&mcp.handle(&read).unwrap()).unwrap();
        let content = read["result"]["contents"][0].clone();
        let complete: Value = serde_json::from_slice(
            &mcp.handle(&durable_call(DURABLE_RECEIPT_TOOL, lookup_intent.clone()))
                .unwrap(),
        )
        .unwrap();
        let mut changed = lookup_intent;
        changed["destination"] = json!(changed_path.to_str().unwrap());
        let conflict: Value = serde_json::from_slice(
            &mcp.handle(&durable_call(DURABLE_RECEIPT_TOOL, changed))
                .unwrap(),
        )
        .unwrap();
        assert!(mcp.into_client().disconnect().is_ok());
        (resources, content, complete, conflict)
    });
    for _ in 0..7 {
        restarted.serve_next().unwrap();
    }
    let (resources_after, content_after, complete, conflict) = lookup.join().unwrap();
    assert_eq!(resources_after, resources_before);
    assert_eq!(content_after, content_before);
    assert_eq!(
        complete["result"]["structuredContent"],
        json!({
            "kind": "document_copy_exported_exclusive",
            "workspace": intent["workspace"].clone(),
            "document": intent["document"].clone(),
            "source_revision": 0,
            "persistence": {
                "status": "durable",
                "failure": null,
                "cleanup_failed": false
            }
        })
    );
    assert!(!complete.to_string().contains(destination.to_str().unwrap()));
    assert_eq!(
        conflict["result"]["structuredContent"]["code"],
        "request_conflict"
    );
    assert_eq!(fs::read_to_string(&source).unwrap(), "alpha");
    assert_eq!(fs::read_to_string(&destination).unwrap(), "alpha");
    assert!(!changed_destination.exists());
    assert_eq!(restarted.service_usage().durable_effect_requests, 0);
    assert_eq!(restarted.usage().durable_operation_lookups, 2);
    drop(restarted);

    let foreign_credential = directory.join("foreign-credential");
    LocalCredentialStoreV1::default()
        .provision(&foreign_credential)
        .unwrap();
    let mut foreign = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &socket,
        workspace(),
        daemon_limits,
        AuthorityActionSetV1::read_only(),
    )
    .unwrap()
    .with_exact_export_policy(export_policy())
    .unwrap()
    .require_authentication(
        LocalCredentialVerifierV1::new(
            LocalCredentialStoreV1::default()
                .load(&foreign_credential)
                .unwrap(),
        ),
        (),
        8,
    );
    let foreign_socket = socket.clone();
    let foreign_path = foreign_credential.clone();
    let retained_intent = intent;
    let foreign_lookup = std::thread::spawn(move || {
        let connector = AuthenticatedUnixConnector::with_provider_and_timeout(
            foreign_socket,
            provider(&foreign_path),
            Duration::from_secs(2),
        )
        .unwrap();
        let transport = RemoteTransport::new(connector, TransportLimits::reference()).unwrap();
        let client = Client::connect(transport).unwrap();
        let mut mcp = McpServer::new_authenticated(client, McpLimits::reference()).unwrap();
        let retained: Value = serde_json::from_slice(
            &mcp.handle(&durable_call(DURABLE_RECEIPT_TOOL, retained_intent.clone()))
                .unwrap(),
        )
        .unwrap();
        let mut unknown_intent = retained_intent;
        unknown_intent["operation_ref"] = json!("vdo1.64646464646464646464646464646464");
        let unknown: Value = serde_json::from_slice(
            &mcp.handle(&durable_call(DURABLE_RECEIPT_TOOL, unknown_intent))
                .unwrap(),
        )
        .unwrap();
        assert!(mcp.into_client().disconnect().is_ok());
        (retained, unknown)
    });
    for _ in 0..4 {
        foreign.serve_next().unwrap();
    }
    let (retained, unknown) = foreign_lookup.join().unwrap();
    assert_eq!(
        retained["result"]["structuredContent"],
        unknown["result"]["structuredContent"]
    );
    assert_eq!(
        retained["result"]["structuredContent"]["kind"],
        "absent_or_foreign"
    );
    assert_eq!(foreign.service_usage().durable_effect_requests, 0);
    drop(foreign);

    let _ = fs::remove_file(&source);
    let _ = fs::remove_file(&destination);
    let _ = fs::remove_file(&credential);
    let _ = fs::remove_file(&foreign_credential);
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_dir_all(directory.join(".vellum-workspace-v1"));
    let _ = fs::remove_dir_all(directory.join(".vellum-request-journal-v1"));
    let _ = fs::remove_dir_all(directory.join(".vellum-durable-operation-v1"));
    let _ = fs::remove_dir_all(directory.join(".vellum-durable-operation-v2"));
    let _ = fs::remove_file(directory.join(".vellum-daemon.lock"));
    let _ = fs::remove_dir(&directory);
}
