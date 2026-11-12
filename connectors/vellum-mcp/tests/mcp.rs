use serde_json::{Value, json};
use std::{collections::VecDeque, io::BufReader};
use vellum_mcp::{
    BoundedLine, CHANGES_TOOL, MEMBERSHIP_TOOL, McpLimits, McpServer, PROTOCOL_VERSION,
    REPLACE_AND_SAVE_TOOL, REPLACE_TOOL, RESOLVE_LOCATOR_TOOL, SNAPSHOT_TOOL, WORKSPACE_EDIT_TOOL,
    read_bounded_line,
};
use vellum_sdk_client::{
    client::Client,
    protocol::{
        ClientId, CommandV1, DirectoryAdmissionReceiptV1, EditPersistenceV1,
        FileAdmissionReceiptV1, FileAdmissionStatusV1, OutcomeV1, RefusalV1,
        ReplaceAndSaveReceiptV1, ReplyV1, RequestV1, SaveApplicationV1, SaveReceiptV1,
        SaveSynchronizationV1, ServiceAccessErrorV1, ServiceControlErrorV1, ServicePort,
        SnapshotV1, VersionWitness, WorkspaceChangeCursorV1, WorkspaceChangeGapV1,
        WorkspaceChangeKindV1, WorkspaceChangePageV1, WorkspaceChangeV1, WorkspaceChangesV1,
        WorkspaceDocumentIdV1, WorkspaceEditReceiptV1, WorkspaceEditedDocumentV1, WorkspaceIdV1,
        WorkspaceInventoryCursorV1, WorkspaceInventoryPageV1, WorkspaceLocatorResolutionV1,
        WorkspaceRootIdV1, WorkspaceRootKindV1, WorkspaceRootV1,
    },
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

fn document() -> WorkspaceDocumentIdV1 {
    WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(7).unwrap(), 11).unwrap()
}

fn document_arguments() -> Value {
    json!({"workspace": 7, "document": 11})
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
    assert_eq!(list["result"]["tools"][2]["name"], REPLACE_AND_SAVE_TOOL);
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
    let mut server = server(vec![
        Ok(ReplyV1::Accepted(OutcomeV1::DocumentSelected(document()))),
        Ok(ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot))),
    ]);
    let reply = send(
        &mut server,
        json!({
            "jsonrpc":"2.0", "id":2, "method":"tools/call",
            "params":{"_meta":meta(), "name":SNAPSHOT_TOOL, "arguments":document_arguments()}
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
fn workspace_edit_submits_one_revision_bound_atomic_native_command() {
    let workspace = WorkspaceIdV1::new(7).unwrap();
    let first = WorkspaceDocumentIdV1::new(workspace, 11).unwrap();
    let second = WorkspaceDocumentIdV1::new(workspace, 12).unwrap();
    let mut server = server(vec![Ok(ReplyV1::Accepted(OutcomeV1::WorkspaceEdited(
        WorkspaceEditReceiptV1 {
            workspace,
            generation: 6,
            documents: vec![
                WorkspaceEditedDocumentV1 {
                    document: first,
                    snapshot: SnapshotV1 {
                        witness: VersionWitness::new(11).unwrap(),
                        revision: 7,
                        text: "aQQd".into(),
                    },
                },
                WorkspaceEditedDocumentV1 {
                    document: second,
                    snapshot: SnapshotV1 {
                        witness: VersionWitness::new(12).unwrap(),
                        revision: 8,
                        text: "wRRz".into(),
                    },
                },
            ],
        },
    )))]);
    let reply = send(
        &mut server,
        json!({
            "jsonrpc":"2.0", "id":"batch", "method":"tools/call",
            "params":{"_meta":meta(), "name":WORKSPACE_EDIT_TOOL, "arguments":{"documents":[
                {"workspace":7, "document":11, "expected_revision":4, "replacements":[{"start":1, "end":3, "inserted":"QQ"}]},
                {"workspace":7, "document":12, "expected_revision":5, "replacements":[{"start":1, "end":3, "inserted":"RR"}]}
            ]}}
        }),
    );
    assert_eq!(reply["result"]["isError"], false);
    assert_eq!(
        reply["result"]["structuredContent"]["kind"],
        "workspace_edited"
    );
    assert_eq!(
        reply["result"]["structuredContent"]["persistence"],
        "not_attempted"
    );
    assert_eq!(
        reply["result"]["structuredContent"]["documents"][1]["text"],
        "wRRz"
    );

    let transport = server.into_client().into_transport();
    assert_eq!(transport.requests.len(), 1);
    let CommandV1::WorkspaceEdit { documents } = &transport.requests[0].command else {
        panic!("the command must be one atomic workspace edit")
    };
    assert_eq!(documents.len(), 2);
    assert_eq!(documents[0].expected_revision, 4);
    assert_eq!(documents[1].expected_revision, 5);
}

#[test]
fn semantic_refusal_is_a_closed_tool_error_while_bad_input_is_protocol_error() {
    let snapshot = SnapshotV1 {
        witness: VersionWitness::new(7).unwrap(),
        revision: 4,
        text: "alpha".into(),
    };
    let mut server = server(vec![
        Ok(ReplyV1::Accepted(OutcomeV1::DocumentSelected(document()))),
        Ok(ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot))),
        Ok(ReplyV1::Refused(RefusalV1::Stale)),
    ]);
    let stale = send(
        &mut server,
        json!({
            "jsonrpc":"2.0", "id":3, "method":"tools/call",
            "params":{"_meta":meta(), "name":REPLACE_TOOL, "arguments":{
                "workspace":7, "document":11, "expected_revision":4, "start":0, "end":0, "inserted":"x"
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
    let foreign_operation = send(
        &mut server,
        json!({
            "jsonrpc":"2.0", "id":5, "method":"tools/call",
            "params":{"_meta":meta(), "name":REPLACE_TOOL, "arguments":{
                "workspace":7, "document":11, "expected_revision":4,
                "start":0, "end":0, "inserted":"x", "operation":"vo1.0000000000000001"
            }}
        }),
    );
    assert_eq!(foreign_operation["error"]["code"], -32602);
    assert_eq!(server.usage().tool_errors, 1);
}

#[test]
fn authority_refusals_remain_closed_tool_errors() {
    for (refusal, expected) in [
        (RefusalV1::AuthorityDenied, "authority_denied"),
        (RefusalV1::AuthorityUnavailable, "authority_unavailable"),
    ] {
        let mut server = server(vec![Ok(ReplyV1::Refused(refusal))]);
        let reply = send(
            &mut server,
            json!({
                "jsonrpc":"2.0", "id":"authority", "method":"tools/call",
                "params":{"_meta":meta(), "name":SNAPSHOT_TOOL, "arguments":document_arguments()}
            }),
        );
        assert_eq!(reply["result"]["isError"], true);
        assert_eq!(reply["result"]["structuredContent"]["code"], expected);
    }
}

#[test]
fn transport_phase_failures_are_closed_tool_errors() {
    for (failure, expected) in [
        (ServiceAccessErrorV1::NotSent, "service_not_sent"),
        (
            ServiceAccessErrorV1::OutcomeUnknown,
            "service_outcome_unknown",
        ),
    ] {
        let mut server = server(vec![Err(failure)]);
        let reply = send(
            &mut server,
            json!({
                "jsonrpc":"2.0", "id":"phase", "method":"tools/call",
                "params":{"_meta":meta(), "name":SNAPSHOT_TOOL, "arguments":document_arguments()}
            }),
        );
        assert_eq!(reply["result"]["isError"], true);
        assert_eq!(reply["result"]["structuredContent"]["code"], expected);
    }
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
        Ok(ReplyV1::Accepted(OutcomeV1::DocumentSelected(document()))),
        Ok(ReplyV1::Accepted(OutcomeV1::Snapshot(before))),
        Ok(ReplyV1::Accepted(OutcomeV1::Replaced(after))),
    ]);
    let reply = send(
        &mut server,
        json!({
            "jsonrpc":"2.0", "id":"replace", "method":"tools/call",
            "params":{"_meta":meta(), "name":REPLACE_TOOL, "arguments":{
                "workspace":7, "document":11, "expected_revision":4, "start":0, "end":0, "inserted":"x"
            }}
        }),
    );
    assert_eq!(reply["result"]["structuredContent"]["kind"], "replaced");
    assert_eq!(reply["result"]["structuredContent"]["revision"], 5);

    let transport = server.into_client().into_transport();
    assert_eq!(transport.requests.len(), 3);
    assert!(matches!(transport.requests[1].command, CommandV1::Snapshot));
    assert!(matches!(
        transport.requests[2].command,
        CommandV1::Replace { expected, .. } if expected == VersionWitness::new(7).unwrap()
    ));
}

#[test]
fn durable_replacement_projects_closed_persistence_evidence_without_native_witness() {
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
    let save = SaveReceiptV1 {
        application: SaveApplicationV1::Written,
        synchronization: SaveSynchronizationV1::Succeeded,
        no_future_effects: true,
        quiescent: true,
        adapter_failure: false,
        cleanup_failed: false,
        baseline_published: true,
        outcome_unknown: false,
    };
    let mut server = server(vec![
        Ok(ReplyV1::Accepted(OutcomeV1::DocumentSelected(document()))),
        Ok(ReplyV1::Accepted(OutcomeV1::Snapshot(before))),
        Ok(ReplyV1::Accepted(OutcomeV1::ReplacedAndSaved(
            ReplaceAndSaveReceiptV1 {
                snapshot: after,
                persistence: EditPersistenceV1::Durable(save),
            },
        ))),
    ]);
    let reply = send(
        &mut server,
        json!({
            "jsonrpc":"2.0", "id":"durable", "method":"tools/call",
            "params":{"_meta":meta(), "name":REPLACE_AND_SAVE_TOOL, "arguments":{
                "workspace":7, "document":11, "expected_revision":4, "start":0, "end":0, "inserted":"x"
            }}
        }),
    );
    let structured = &reply["result"]["structuredContent"];
    assert_eq!(structured["kind"], "replaced_and_saved");
    assert_eq!(structured["revision"], 5);
    assert_eq!(structured["persistence"]["status"], "durable");
    assert_eq!(structured["persistence"]["application"], "written");
    assert!(structured["witness"].is_null());

    let transport = server.into_client().into_transport();
    assert!(matches!(
        transport.requests[2].command,
        CommandV1::ReplaceAndSave { expected, .. }
            if expected == VersionWitness::new(7).unwrap()
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
fn snapshot_requires_explicit_document_identity_and_rejects_undeclared_params() {
    let snapshot = SnapshotV1 {
        witness: VersionWitness::new(1).unwrap(),
        revision: 0,
        text: "".into(),
    };
    let mut server = server(vec![
        Ok(ReplyV1::Accepted(OutcomeV1::DocumentSelected(document()))),
        Ok(ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot))),
    ]);
    let accepted = send(
        &mut server,
        json!({
            "jsonrpc":"2.0", "id":8, "method":"tools/call",
            "params":{"_meta":meta(), "name":SNAPSHOT_TOOL, "arguments":document_arguments()}
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
        replies: vec![
            Ok(ReplyV1::Accepted(OutcomeV1::DocumentSelected(document()))),
            Ok(ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot))),
        ]
        .into(),
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
            "params":{"_meta":meta(), "name":SNAPSHOT_TOOL, "arguments":document_arguments()}
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
    assert!(stdout.contains("grants no Vellum action"));
    assert!(stdout.contains("never accepted through argv or environment"));
}

#[test]
fn modern_discovery_is_per_request_and_initialize_is_always_legacy() {
    let mut server = server(Vec::new());
    let reply = send(
        &mut server,
        json!({
            "jsonrpc":"2.0", "id":"init", "method":"initialize", "params":{
                "protocolVersion":PROTOCOL_VERSION,
                "capabilities":{"resources":{}},
                "clientInfo":{"name":"test", "version":"1"}
            }
        }),
    );
    assert_eq!(reply["error"]["code"], -32022);
    assert_eq!(
        reply["error"]["data"]["supported"],
        json!([PROTOCOL_VERSION])
    );

    let listed = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":"tools", "method":"tools/list", "params":{"_meta":meta()}}),
    );
    let names = listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            SNAPSHOT_TOOL,
            REPLACE_TOOL,
            REPLACE_AND_SAVE_TOOL,
            WORKSPACE_EDIT_TOOL,
            CHANGES_TOOL,
            RESOLVE_LOCATOR_TOOL,
            MEMBERSHIP_TOOL
        ]
    );
    let workspace_edit = listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == WORKSPACE_EDIT_TOOL)
        .unwrap();
    let success = &workspace_edit["outputSchema"]["oneOf"][0];
    assert_eq!(success["additionalProperties"], false);
    assert_eq!(
        success["properties"]["persistence"]["const"],
        "not_attempted"
    );
    assert_eq!(success["properties"]["documents"]["maxItems"], 16);
    assert_eq!(
        success["properties"]["documents"]["items"]["additionalProperties"],
        false
    );
}

#[test]
fn resources_are_content_free_paginated_and_cursor_bound() {
    let workspace = WorkspaceIdV1::new(7).unwrap();
    let document = WorkspaceDocumentIdV1::new(workspace, 11).unwrap();
    let root = WorkspaceRootIdV1::new(workspace, 13).unwrap();
    let cursor = WorkspaceInventoryCursorV1::new(workspace, 3, 2).unwrap();
    let mut server = server(vec![
        Ok(ReplyV1::Accepted(OutcomeV1::WorkspaceInventoryPage(
            WorkspaceInventoryPageV1 {
                workspace,
                documents: vec![document],
                roots: vec![WorkspaceRootV1 {
                    id: root,
                    kind: WorkspaceRootKindV1::Directory,
                }],
                next: Some(cursor),
            },
        ))),
        Ok(ReplyV1::Accepted(OutcomeV1::WorkspaceInventoryPage(
            WorkspaceInventoryPageV1 {
                workspace,
                documents: Vec::new(),
                roots: Vec::new(),
                next: None,
            },
        ))),
    ]);
    let reply = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":"resources", "method":"resources/list", "params":{"_meta":meta()}}),
    );
    assert_eq!(reply["result"]["workspace"], 7);
    assert_eq!(reply["result"]["resources"].as_array().unwrap().len(), 2);
    assert_eq!(
        reply["result"]["resources"][0]["uri"],
        "vellum://workspace/7/document/11"
    );
    assert_eq!(
        reply["result"]["resources"][1]["uri"],
        "vellum://workspace/7/root/13"
    );
    let encoded = reply["result"]["nextCursor"].as_str().unwrap();
    assert!(encoded.starts_with("vi1."));
    let serialized = reply.to_string();
    assert!(!serialized.contains('/') || serialized.contains("vellum://"));
    assert!(!serialized.contains("private/tmp"));

    let continued = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":"continued", "method":"resources/list", "params":{"_meta":meta(), "cursor":encoded}}),
    );
    assert!(
        continued["result"]["resources"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(continued["result"]["nextCursor"].is_null());

    let invalid = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":"bad", "method":"resources/list", "params":{"_meta":meta(), "cursor":"vi1.0.1.1"}}),
    );
    assert_eq!(invalid["error"]["code"], -32602);
    let transport = server.into_client().into_transport();
    assert!(matches!(
        transport.requests[1].command,
        CommandV1::WorkspaceInventoryPage { cursor: Some(actual), page_size: 2 }
            if actual == cursor
    ));
}

#[test]
fn resource_read_is_self_contained_and_never_accepts_path_uris() {
    let workspace = WorkspaceIdV1::new(7).unwrap();
    let document = WorkspaceDocumentIdV1::new(workspace, 11).unwrap();
    let snapshot = SnapshotV1 {
        witness: VersionWitness::new(4).unwrap(),
        revision: 9,
        text: "alpha".into(),
    };
    let mut mcp = server(vec![
        Ok(ReplyV1::Accepted(OutcomeV1::DocumentSelected(document))),
        Ok(ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot))),
    ]);
    let read = send(
        &mut mcp,
        json!({"jsonrpc":"2.0", "id":1, "method":"resources/read", "params":{"_meta":meta(), "uri":"vellum://workspace/7/document/11"}}),
    );
    assert_eq!(read["result"]["ttlMs"], 0);
    assert_eq!(read["result"]["cacheScope"], "private");
    assert_eq!(read["result"]["contents"][0]["text"], "alpha");
    assert_eq!(read["result"]["contents"][0]["_meta"]["vellum/revision"], 9);

    let path = send(
        &mut mcp,
        json!({"jsonrpc":"2.0", "id":2, "method":"resources/read", "params":{"_meta":meta(), "uri":"file:///private/tmp/secret"}}),
    );
    assert_eq!(path["error"]["code"], -32602);

    let mut missing = server(vec![Ok(ReplyV1::Refused(RefusalV1::UnknownDocument))]);
    let missing = send(
        &mut missing,
        json!({"jsonrpc":"2.0", "id":3, "method":"resources/read", "params":{"_meta":meta(), "uri":"vellum://workspace/7/document/99"}}),
    );
    assert_eq!(missing["error"]["code"], -32602);
    assert_eq!(missing["error"]["data"]["code"], "unknown_document");
}

#[test]
fn root_resource_read_revalidates_kind_and_rejects_unknown_or_tampered_uris() {
    let workspace = WorkspaceIdV1::new(7).unwrap();
    let root = WorkspaceRootIdV1::new(workspace, 13).unwrap();
    let mut server = server(vec![
        Ok(ReplyV1::Accepted(OutcomeV1::WorkspaceInventoryPage(
            WorkspaceInventoryPageV1 {
                workspace,
                documents: Vec::new(),
                roots: vec![WorkspaceRootV1 {
                    id: root,
                    kind: WorkspaceRootKindV1::Directory,
                }],
                next: None,
            },
        ))),
        Ok(ReplyV1::Accepted(OutcomeV1::WorkspaceInventoryPage(
            WorkspaceInventoryPageV1 {
                workspace,
                documents: Vec::new(),
                roots: Vec::new(),
                next: None,
            },
        ))),
    ]);
    let read = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":1, "method":"resources/read", "params":{"_meta":meta(), "uri":"vellum://workspace/7/root/13"}}),
    );
    assert_eq!(read["result"]["ttlMs"], 0);
    assert_eq!(read["result"]["cacheScope"], "private");
    assert_eq!(
        read["result"]["contents"][0]["text"],
        "{\"kind\":\"directory\"}"
    );
    let unknown = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":2, "method":"resources/read", "params":{"_meta":meta(), "uri":"vellum://workspace/7/root/99"}}),
    );
    assert_eq!(unknown["error"]["code"], -32602);
    assert_eq!(unknown["error"]["data"]["code"], "unknown_root");
    let tampered = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":3, "method":"resources/read", "params":{"_meta":meta(), "uri":"vellum://workspace/7/root/file/13"}}),
    );
    assert_eq!(tampered["error"]["code"], -32602);
}

#[test]
fn change_poll_projects_opaque_continuations_and_explicit_gaps() {
    let workspace = WorkspaceIdV1::new(7).unwrap();
    let document = WorkspaceDocumentIdV1::new(workspace, 11).unwrap();
    let next = WorkspaceChangeCursorV1::new(workspace, 5, 4).unwrap();
    let page = WorkspaceChangesV1::Page(WorkspaceChangePageV1 {
        workspace,
        observed_head: 4,
        changes: vec![
            WorkspaceChangeV1::new(
                4,
                WorkspaceChangeKindV1::DocumentChanged {
                    document,
                    source_revision: 9,
                },
            )
            .unwrap(),
        ],
        next,
    });
    let gap = WorkspaceChangesV1::Gap(WorkspaceChangeGapV1::new(workspace, 5, 4, 6, 8).unwrap());
    let mut server = server(vec![
        Ok(ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(page))),
        Ok(ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(gap))),
    ]);
    let reply = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{"_meta":meta(), "name":CHANGES_TOOL, "arguments":{"page_size":1}}}),
    );
    assert_eq!(reply["result"]["structuredContent"]["kind"], "change_page");
    assert_eq!(
        reply["result"]["structuredContent"]["changes"][0]["source_revision"],
        9
    );
    assert!(
        reply["result"]["structuredContent"]["next_cursor"]
            .as_str()
            .unwrap()
            .starts_with("vc1.")
    );
    let cursor = reply["result"]["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let gap = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":2, "method":"tools/call", "params":{"_meta":meta(), "name":CHANGES_TOOL, "arguments":{"page_size":1, "cursor":cursor}}}),
    );
    assert_eq!(gap["result"]["structuredContent"]["kind"], "change_gap");
    assert_eq!(gap["result"]["structuredContent"]["oldest_available"], 6);

    let malformed = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":3, "method":"tools/call", "params":{"_meta":meta(), "name":CHANGES_TOOL, "arguments":{"page_size":2, "cursor":"x"}}}),
    );
    assert_eq!(malformed["error"]["code"], -32602);
    let transport = server.into_client().into_transport();
    assert!(matches!(
        transport.requests[1].command,
        CommandV1::WorkspaceChanges { after: Some(actual), page_size: 1 }
            if actual == next
    ));
}

#[test]
fn outcome_unknown_durable_edit_replays_the_exact_native_request() {
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
    let receipt = ReplaceAndSaveReceiptV1 {
        snapshot: after,
        persistence: EditPersistenceV1::Durable(SaveReceiptV1 {
            application: SaveApplicationV1::Written,
            synchronization: SaveSynchronizationV1::Succeeded,
            no_future_effects: true,
            quiescent: true,
            adapter_failure: false,
            cleanup_failed: false,
            baseline_published: true,
            outcome_unknown: false,
        }),
    };
    let mut server = server(vec![
        Ok(ReplyV1::Accepted(OutcomeV1::DocumentSelected(document()))),
        Ok(ReplyV1::Accepted(OutcomeV1::Snapshot(before))),
        Err(ServiceAccessErrorV1::OutcomeUnknown),
        Ok(ReplyV1::Accepted(OutcomeV1::ReplacedAndSaved(receipt))),
    ]);
    let request = json!({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{"_meta":meta(), "name":REPLACE_AND_SAVE_TOOL, "arguments":{"workspace":7, "document":11, "expected_revision":4, "start":0, "end":0, "inserted":"x"}}});
    let unknown = send(&mut server, request.clone());
    assert_eq!(
        unknown["result"]["structuredContent"]["code"],
        "service_outcome_unknown"
    );
    let operation = unknown["result"]["structuredContent"]["operation"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut retry = request;
    retry["id"] = json!(2);
    retry["params"]["arguments"]["operation"] = json!(operation);
    let completed = send(&mut server, retry);
    assert_eq!(
        completed["result"]["structuredContent"]["persistence"]["status"],
        "durable"
    );
    let transport = server.into_client().into_transport();
    assert_eq!(transport.requests.len(), 4);
    assert_eq!(transport.requests[2], transport.requests[3]);
}

#[test]
fn durable_workflow_handle_resumes_an_uncertain_snapshot_stage() {
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
        Ok(ReplyV1::Accepted(OutcomeV1::DocumentSelected(document()))),
        Err(ServiceAccessErrorV1::OutcomeUnknown),
        Ok(ReplyV1::Accepted(OutcomeV1::Snapshot(before))),
        Ok(ReplyV1::Accepted(OutcomeV1::ReplacedAndSaved(
            ReplaceAndSaveReceiptV1 {
                snapshot: after,
                persistence: EditPersistenceV1::NotAttempted(RefusalV1::AuthorityDenied),
            },
        ))),
    ]);
    let mut request = json!({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{"_meta":meta(), "name":REPLACE_AND_SAVE_TOOL, "arguments":{"workspace":7, "document":11, "expected_revision":4, "start":0, "end":0, "inserted":"x"}}});
    let unknown = send(&mut server, request.clone());
    request["id"] = json!(2);
    request["params"]["arguments"]["operation"] =
        unknown["result"]["structuredContent"]["operation"].clone();
    let completed = send(&mut server, request);
    assert_eq!(
        completed["result"]["structuredContent"]["persistence"]["status"],
        "not_attempted"
    );
    let transport = server.into_client().into_transport();
    assert_eq!(transport.requests[1], transport.requests[2]);
    assert!(matches!(
        transport.requests[3].command,
        CommandV1::ReplaceAndSave { .. }
    ));
}

#[test]
fn membership_is_strictly_shaped_and_native_authority_remains_decisive() {
    let mut server = server(vec![Ok(ReplyV1::Refused(RefusalV1::AuthorityDenied))]);
    let denied = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{"_meta":meta(), "name":MEMBERSHIP_TOOL, "arguments":{"operation":"add_file", "locator":"/private/tmp/source"}}}),
    );
    assert_eq!(
        denied["result"]["structuredContent"]["code"],
        "authority_denied"
    );
    assert!(!denied.to_string().contains("/private/tmp/source"));

    let malformed = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":2, "method":"tools/call", "params":{"_meta":meta(), "name":MEMBERSHIP_TOOL, "arguments":{"operation":"add_file", "locator":"x", "document":1}}}),
    );
    assert_eq!(malformed["error"]["code"], -32602);
}

#[test]
fn locator_resolution_is_closed_and_never_echoes_private_input() {
    let workspace = WorkspaceIdV1::new(7).unwrap();
    let document = WorkspaceDocumentIdV1::new(workspace, 11).unwrap();
    let root = WorkspaceRootV1 {
        id: WorkspaceRootIdV1::new(workspace, 13).unwrap(),
        kind: WorkspaceRootKindV1::File,
    };
    let mut server = server(vec![
        Ok(ReplyV1::Accepted(OutcomeV1::LocatorResolved(
            WorkspaceLocatorResolutionV1 {
                workspace,
                document: Some(document),
                root: Some(root),
            },
        ))),
        Ok(ReplyV1::Accepted(OutcomeV1::LocatorResolved(
            WorkspaceLocatorResolutionV1 {
                workspace,
                document: None,
                root: None,
            },
        ))),
    ]);
    let private = "/private/agent/source.rs";
    let resolved = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{"_meta":meta(), "name":RESOLVE_LOCATOR_TOOL, "arguments":{"locator":private}}}),
    );
    assert_eq!(
        resolved["result"]["structuredContent"],
        json!({
            "kind": "locator_resolved",
            "workspace": 7,
            "document": 11,
            "root": {"root": 13, "kind": "file"}
        })
    );
    assert!(!resolved.to_string().contains(private));

    let absent = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":2, "method":"tools/call", "params":{"_meta":meta(), "name":RESOLVE_LOCATOR_TOOL, "arguments":{"locator":"/foreign/member"}}}),
    );
    assert_eq!(
        absent["result"]["structuredContent"],
        json!({"kind": "locator_absent_or_foreign", "workspace": 7})
    );
    assert!(!absent.to_string().contains("/foreign/member"));

    let malformed = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":3, "method":"tools/call", "params":{"_meta":meta(), "name":RESOLVE_LOCATOR_TOOL, "arguments":{"locator":"/x", "extra":true}}}),
    );
    assert_eq!(malformed["error"]["code"], -32602);

    let transport = server.into_client().into_transport();
    assert!(matches!(
        &transport.requests[0].command,
        CommandV1::ResolveLocator { locator } if locator == private
    ));
}

#[test]
fn membership_projects_exact_content_free_admission_receipts() {
    let workspace = WorkspaceIdV1::new(7).unwrap();
    let document = WorkspaceDocumentIdV1::new(workspace, 11).unwrap();
    let file_root = WorkspaceRootIdV1::new(workspace, 13).unwrap();
    let directory_root = WorkspaceRootIdV1::new(workspace, 14).unwrap();
    let directory_documents = vec![
        WorkspaceDocumentIdV1::new(workspace, 15).unwrap(),
        WorkspaceDocumentIdV1::new(workspace, 16).unwrap(),
    ];
    let mut server = server(vec![
        Ok(ReplyV1::Accepted(OutcomeV1::FileAdmitted(
            FileAdmissionReceiptV1 {
                document,
                root: Some(file_root),
                status: FileAdmissionStatusV1::Added,
            },
        ))),
        Ok(ReplyV1::Accepted(OutcomeV1::DirectoryAdmitted(
            DirectoryAdmissionReceiptV1 {
                root: directory_root,
                added: 2,
                joined: 1,
                added_documents: directory_documents,
            },
        ))),
    ]);
    let file = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{"_meta":meta(), "name":MEMBERSHIP_TOOL, "arguments":{"operation":"add_file", "locator":"private-file"}}}),
    );
    let file_result = &file["result"]["structuredContent"];
    assert_eq!(file_result["kind"], "file_admission");
    assert_eq!(file_result["document"], 11);
    assert_eq!(file_result["root"]["root"], 13);
    assert_eq!(file_result["status"], "added");
    assert!(!file.to_string().contains("private-file"));

    let directory = send(
        &mut server,
        json!({"jsonrpc":"2.0", "id":2, "method":"tools/call", "params":{"_meta":meta(), "name":MEMBERSHIP_TOOL, "arguments":{"operation":"add_directory", "locator":"private-directory"}}}),
    );
    let directory_result = &directory["result"]["structuredContent"];
    assert_eq!(directory_result["kind"], "directory_admission");
    assert_eq!(directory_result["root"], 14);
    assert_eq!(directory_result["added"], 2);
    assert_eq!(directory_result["joined"], 1);
    assert_eq!(directory_result["added_documents"][0]["document"], 15);
    assert_eq!(directory_result["added_documents"][1]["document"], 16);
    assert!(!directory.to_string().contains("private-directory"));
}
