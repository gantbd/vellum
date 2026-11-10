use serde_json::{Value, json};
use std::{collections::VecDeque, io::BufReader};
use vellum_client::Client;
use vellum_mcp::{
    BoundedLine, McpLimits, McpServer, PROTOCOL_VERSION, REPLACE_TOOL, SNAPSHOT_TOOL,
    read_bounded_line,
};
use vellum_protocol::{
    ClientId, CommandV1, OutcomeV1, RefusalV1, ReplyV1, RequestV1, ServiceAccessErrorV1,
    ServiceControlErrorV1, ServicePort, SnapshotV1, VersionWitness,
};

#[derive(Debug)]
struct ScriptedPort {
    replies: VecDeque<Result<ReplyV1, ServiceAccessErrorV1>>,
    requests: Vec<RequestV1>,
}

impl ServicePort for ScriptedPort {
    fn connect(&mut self) -> Result<ClientId, ServiceControlErrorV1> {
        Ok(ClientId::new(41).unwrap())
    }

    fn execute(&mut self, request: RequestV1) -> Result<ReplyV1, ServiceAccessErrorV1> {
        self.requests.push(request);
        self.replies
            .pop_front()
            .unwrap_or(Err(ServiceAccessErrorV1::Unavailable))
    }

    fn disconnect(&mut self, _client: ClientId) -> Result<(), ServiceControlErrorV1> {
        Ok(())
    }
}

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": PROTOCOL_VERSION,
        "io.modelcontextprotocol/clientCapabilities": {},
        "io.modelcontextprotocol/clientInfo": {"name": "test", "version": "1"}
    })
}

fn server(replies: Vec<Result<ReplyV1, ServiceAccessErrorV1>>) -> McpServer<ScriptedPort> {
    let client = Client::connect(ScriptedPort {
        replies: replies.into(),
        requests: Vec::new(),
    })
    .unwrap();
    McpServer::new(client, McpLimits::reference()).unwrap()
}

fn send(server: &mut McpServer<ScriptedPort>, request: Value) -> Value {
    serde_json::from_slice(&server.handle(request.to_string().as_bytes()).unwrap()).unwrap()
}

#[test]
fn current_protocol_discovery_and_catalog_are_complete_deterministic_and_private() {
    let mut server = server(Vec::new());
    let discover = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":1, "method":"server/discover", "params":{"_meta":meta()}}),
    );
    assert_eq!(
        discover["result"]["supportedVersions"],
        json!([PROTOCOL_VERSION])
    );
    assert_eq!(
        discover["result"]["capabilities"]["tools"]["listChanged"],
        false
    );
    assert_eq!(discover["result"]["cacheScope"], "private");

    let list = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":"list", "method":"tools/list", "params":{"_meta":meta()}}),
    );
    assert_eq!(list["result"]["tools"][0]["name"], SNAPSHOT_TOOL);
    assert_eq!(list["result"]["tools"][1]["name"], REPLACE_TOOL);
    assert_eq!(
        list["result"]["tools"][1]["inputSchema"]["additionalProperties"],
        false
    );
    assert_eq!(server.usage().discoveries, 1);
    assert_eq!(server.usage().tool_lists, 1);
}

#[test]
fn snapshot_projects_workspace_revision_and_text_without_leaking_client_witness() {
    let snapshot = SnapshotV1 {
        witness: VersionWitness::new(9).unwrap(),
        revision: 3,
        text: "alpha".into(),
    };
    let mut server = server(vec![Ok(ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot)))]);
    let reply = send(
        &mut server,
        json!({
            "jsonrpc":"2.0", "id":2, "method":"tools/call",
            "params":{"_meta":meta(), "name":SNAPSHOT_TOOL, "arguments":{}}
        }),
    );
    assert_eq!(reply["result"]["isError"], false);
    assert!(reply["result"]["structuredContent"]["witness"].is_null());
    assert_eq!(reply["result"]["structuredContent"]["revision"], 3);
    assert_eq!(reply["result"]["structuredContent"]["text"], "alpha");
    assert_eq!(server.usage().tool_calls, 1);
    assert!(server.usage().request_bytes > 0);
    assert!(server.usage().response_bytes > 0);
}

#[test]
fn semantic_refusal_is_a_closed_tool_error_while_bad_input_is_protocol_error() {
    let snapshot = SnapshotV1 {
        witness: VersionWitness::new(7).unwrap(),
        revision: 4,
        text: "alpha".into(),
    };
    let mut server = server(vec![
        Ok(ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot))),
        Ok(ReplyV1::Refused(RefusalV1::Stale)),
    ]);
    let stale = send(
        &mut server,
        json!({
            "jsonrpc":"2.0", "id":3, "method":"tools/call",
            "params":{"_meta":meta(), "name":REPLACE_TOOL, "arguments":{
                "expected_revision":4, "start":0, "end":0, "inserted":"x"
            }}
        }),
    );
    assert_eq!(stale["result"]["isError"], true);
    assert_eq!(stale["result"]["structuredContent"]["code"], "stale");

    let invalid = send(
        &mut server,
        json!({
            "jsonrpc":"2.0", "id":4, "method":"tools/call",
            "params":{"_meta":meta(), "name":REPLACE_TOOL, "arguments":{"start":0}}
        }),
    );
    assert_eq!(invalid["error"]["code"], -32602);
    assert_eq!(server.usage().tool_errors, 1);
}

#[test]
fn replacement_derives_a_fresh_native_witness_inside_the_same_mcp_request() {
    let before = SnapshotV1 {
        witness: VersionWitness::new(7).unwrap(),
        revision: 4,
        text: "alpha".into(),
    };
    let after = SnapshotV1 {
        witness: VersionWitness::new(8).unwrap(),
        revision: 5,
        text: "xalpha".into(),
    };
    let mut server = server(vec![
        Ok(ReplyV1::Accepted(OutcomeV1::Snapshot(before))),
        Ok(ReplyV1::Accepted(OutcomeV1::Replaced(after))),
    ]);
    let reply = send(
        &mut server,
        json!({
            "jsonrpc":"2.0", "id":"replace", "method":"tools/call",
            "params":{"_meta":meta(), "name":REPLACE_TOOL, "arguments":{
                "expected_revision":4, "start":0, "end":0, "inserted":"x"
            }}
        }),
    );
    assert_eq!(reply["result"]["structuredContent"]["kind"], "replaced");
    assert_eq!(reply["result"]["structuredContent"]["revision"], 5);

    let transport = server.into_client().into_transport();
    assert_eq!(transport.requests.len(), 2);
    assert!(matches!(transport.requests[0].command, CommandV1::Snapshot));
    assert!(matches!(
        transport.requests[1].command,
        CommandV1::Replace { expected, .. } if expected == VersionWitness::new(7).unwrap()
    ));
}

#[test]
fn every_request_must_carry_supported_version_and_capabilities() {
    let mut server = server(Vec::new());
    let unsupported = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":5, "method":"tools/list", "params":{"_meta":{
            "io.modelcontextprotocol/protocolVersion":"1900-01-01",
            "io.modelcontextprotocol/clientCapabilities":{}
        }}}),
    );
    assert_eq!(unsupported["error"]["code"], -32022);
    assert_eq!(
        unsupported["error"]["data"]["supported"],
        json!([PROTOCOL_VERSION])
    );

    let missing = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":6, "method":"tools/list", "params":{}}),
    );
    assert_eq!(missing["error"]["code"], -32602);
    assert_eq!(server.usage().protocol_errors, 2);
}

#[test]
fn legacy_initialization_fails_forward_with_the_supported_modern_version() {
    let mut server = server(Vec::new());
    let reply = send(
        &mut server,
        json!({
            "jsonrpc":"2.0", "id":7, "method":"initialize",
            "params":{"protocolVersion":"2025-11-25", "capabilities":{}, "clientInfo":{"name":"old", "version":"1"}}
        }),
    );
    assert_eq!(reply["error"]["code"], -32022);
    assert_eq!(
        reply["error"]["data"]["supported"],
        json!([PROTOCOL_VERSION])
    );
}

#[test]
fn snapshot_accepts_spec_optional_arguments_but_rejects_undeclared_params() {
    let snapshot = SnapshotV1 {
        witness: VersionWitness::new(1).unwrap(),
        revision: 0,
        text: "".into(),
    };
    let mut server = server(vec![Ok(ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot)))]);
    let accepted = send(
        &mut server,
        json!({
            "jsonrpc":"2.0", "id":8, "method":"tools/call",
            "params":{"_meta":meta(), "name":SNAPSHOT_TOOL}
        }),
    );
    assert_eq!(accepted["result"]["isError"], false);

    let rejected = send(
        &mut server,
        json!({
            "jsonrpc":"2.0", "id":9, "method":"server/discover",
            "params":{"_meta":meta(), "extra":true}
        }),
    );
    assert_eq!(rejected["error"]["code"], -32602);
}

#[test]
fn bounded_reader_drains_an_oversized_message_before_returning_to_closure() {
    let bytes = b"123456789\n{}\n";
    let mut reader = BufReader::new(&bytes[..]);
    assert_eq!(
        read_bounded_line(&mut reader, 8).unwrap(),
        BoundedLine::Limit(9)
    );
    assert_eq!(
        read_bounded_line(&mut reader, 8).unwrap(),
        BoundedLine::Line(b"{}".to_vec())
    );
    assert_eq!(read_bounded_line(&mut reader, 8).unwrap(), BoundedLine::Eof);
}

#[test]
fn eof_without_a_delimiter_is_a_counted_truncated_request() {
    let mut reader = BufReader::new(&b"{}"[..]);
    assert_eq!(
        read_bounded_line(&mut reader, 8).unwrap(),
        BoundedLine::Truncated(2)
    );
    let mut server = server(Vec::new());
    let reply: Value = serde_json::from_slice(&server.truncated_response(2)).unwrap();
    assert_eq!(reply["error"]["code"], -32600);
    assert_eq!(server.usage().requests, 1);
    assert_eq!(server.usage().request_bytes, 2);
}

#[test]
fn notifications_never_receive_a_compatibility_error() {
    let mut server = server(Vec::new());
    let initialize = json!({
        "jsonrpc":"2.0", "method":"initialize",
        "params":{"protocolVersion":"2025-11-25"}
    });
    assert!(server.handle(initialize.to_string().as_bytes()).is_none());
}

#[test]
fn response_limit_error_preserves_the_original_request_identity() {
    let snapshot = SnapshotV1 {
        witness: VersionWitness::new(1).unwrap(),
        revision: 0,
        text: "x".repeat(1024).into(),
    };
    let client = Client::connect(ScriptedPort {
        replies: vec![Ok(ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot)))].into(),
        requests: Vec::new(),
    })
    .unwrap();
    let mut server = McpServer::new(
        client,
        McpLimits {
            max_request_bytes: 4096,
            max_response_bytes: 256,
        },
    )
    .unwrap();
    let reply = send(
        &mut server,
        json!({
            "jsonrpc":"2.0", "id":"bounded", "method":"tools/call",
            "params":{"_meta":meta(), "name":SNAPSHOT_TOOL}
        }),
    );
    assert_eq!(reply["id"], "bounded");
    assert_eq!(reply["error"]["code"], -32603);
}

#[test]
fn binary_help_is_content_free_and_names_the_trust_boundary() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vellum-mcp"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("vellum-mcp --socket"));
    assert!(stdout.contains("not authentication"));
}

#[cfg(target_os = "macos")]
#[test]
fn stdio_adapter_reaches_the_real_daemon_and_disconnects_on_eof() {
    use std::{
        fs,
        io::{Read, Write},
        os::unix::fs::PermissionsExt,
        process::{Command, Stdio},
        sync::mpsc,
        time::Duration,
    };
    use vellum_adapter_local_fs::{FilesystemLimits, LocalFilesystemFactory};
    use vellum_daemon::{DaemonLimits, LocalDaemon};
    use vellum_editor::Workspace;
    use vellum_service::ServiceLimits;
    use vellum_transport::TransportLimits;

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::path::PathBuf::from("/tmp")
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
            },
        )
        .unwrap();
        ready_tx.send(()).unwrap();
        for _ in 0..3 {
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
    let request = json!({
        "jsonrpc":"2.0", "id":"live", "method":"tools/call",
        "params":{"_meta":meta(), "name":SNAPSHOT_TOOL, "arguments":{}}
    });
    writeln!(child.stdin.take().unwrap(), "{request}").unwrap();
    let mut stdout = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    let reply: Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(reply["result"]["structuredContent"]["text"], "alpha");
    daemon.join().unwrap();

    let _ = fs::remove_file(&source);
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_dir(&directory);
}
