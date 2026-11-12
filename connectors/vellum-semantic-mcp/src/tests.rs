use std::{
    collections::VecDeque,
    future::Future,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

use super::*;
use vellum_sdk_semantic_client::client as vellum_semantic_client;
use vellum_semantic_client::{
    SemanticAdmissionOutcomeV1, SemanticCapabilitySetV1, SemanticDeliveryV1,
    SemanticProviderObservationV1, SemanticProviderProvenanceV1, SemanticProviderRefV1,
    SemanticReleaseOutcomeV1, SemanticResultV1, SemanticServiceRefusalV1, SemanticSnapshotV1,
    SemanticSpanV1, SemanticSymbolV1,
};

fn run<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("model future unexpectedly pending"),
    }
}

struct Model {
    observation: SemanticProviderObservationV1,
    submissions: Vec<SemanticRequestV1>,
    submit_outcomes: VecDeque<SemanticSubmitOutcomeV1>,
    deliveries: VecDeque<SemanticDeliveryV1>,
    releases: Vec<SemanticRequestRefV1>,
}

#[derive(Clone)]
struct ModelPort(Arc<Mutex<Model>>);

impl ModelPort {
    fn ready() -> Self {
        Self(Arc::new(Mutex::new(Model {
            observation: SemanticProviderObservationV1::from_parts(
                SemanticProviderReadinessV1::Ready {
                    generation: SemanticIndexGenerationV1::new(7).unwrap(),
                },
                SemanticCapabilitySetV1::top(),
            ),
            submissions: Vec::new(),
            submit_outcomes: VecDeque::new(),
            deliveries: VecDeque::new(),
            releases: Vec::new(),
        })))
    }
}

impl SemanticServicePortV1 for ModelPort {
    fn observe_provider(
        &self,
    ) -> vellum_semantic_client::SemanticFutureV1<'_, SemanticProviderObservationV1> {
        Box::pin(std::future::ready(self.0.lock().unwrap().observation))
    }

    fn open_snapshot(
        &mut self,
        _snapshot: SemanticSnapshotV1,
    ) -> vellum_semantic_client::SemanticFutureV1<'_, SemanticAdmissionOutcomeV1> {
        Box::pin(std::future::ready(SemanticAdmissionOutcomeV1::Refused(
            SemanticServiceRefusalV1::UnknownClient,
        )))
    }

    fn change_snapshot(
        &mut self,
        _snapshot: SemanticSnapshotV1,
    ) -> vellum_semantic_client::SemanticFutureV1<'_, SemanticAdmissionOutcomeV1> {
        Box::pin(std::future::ready(SemanticAdmissionOutcomeV1::Refused(
            SemanticServiceRefusalV1::UnknownClient,
        )))
    }

    fn close_snapshot(
        &mut self,
        _target: SemanticTargetV1,
    ) -> vellum_semantic_client::SemanticFutureV1<'_, SemanticAdmissionOutcomeV1> {
        Box::pin(std::future::ready(SemanticAdmissionOutcomeV1::Refused(
            SemanticServiceRefusalV1::UnknownClient,
        )))
    }

    fn submit(
        &mut self,
        request: SemanticRequestV1,
    ) -> vellum_semantic_client::SemanticFutureV1<'_, SemanticSubmitOutcomeV1> {
        let outcome = {
            let mut model = self.0.lock().unwrap();
            model.submissions.push(request);
            model
                .submit_outcomes
                .pop_front()
                .unwrap_or(SemanticSubmitOutcomeV1::Accepted)
        };
        Box::pin(std::future::ready(outcome))
    }

    fn cancel(
        &mut self,
        _request: SemanticRequestRefV1,
    ) -> vellum_semantic_client::SemanticFutureV1<'_, Result<bool, SemanticServiceRefusalV1>> {
        Box::pin(std::future::ready(Ok(false)))
    }

    fn release_request(
        &mut self,
        request: SemanticRequestRefV1,
    ) -> vellum_semantic_client::SemanticFutureV1<'_, SemanticReleaseOutcomeV1> {
        self.0.lock().unwrap().releases.push(request);
        Box::pin(std::future::ready(SemanticReleaseOutcomeV1::Released))
    }

    fn take_delivery(
        &mut self,
    ) -> vellum_semantic_client::SemanticFutureV1<
        '_,
        Result<Option<SemanticDeliveryV1>, SemanticServiceRefusalV1>,
    > {
        let delivery = self.0.lock().unwrap().deliveries.pop_front();
        Box::pin(std::future::ready(Ok(delivery)))
    }
}

fn server() -> (SemanticMcpServerV1<ModelPort>, ModelPort) {
    let port = ModelPort::ready();
    let client = SemanticClientV1::new(port.clone());
    (
        SemanticMcpServerV1::try_new_test(client, SemanticMcpLimitsV1::reference()).unwrap(),
        port,
    )
}

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion":PROTOCOL_VERSION,
        "io.modelcontextprotocol/clientCapabilities":{},
        "io.modelcontextprotocol/clientInfo":{"name":"semantic-test","version":"1"}
    })
}

fn server_meta() -> Value {
    json!({"io.modelcontextprotocol/serverInfo":{
        "name":"vellum-semantic-mcp",
        "version":env!("CARGO_PKG_VERSION"),
        "description":"Bounded MCP projection of Vellum semantic queries"
    }})
}

fn common_arguments() -> Value {
    json!({
        "request":"01010101010101010101010101010101",
        "document":"02020202020202020202020202020202",
        "source_revision":3,
        "index_generation":7,
        "language":"rust"
    })
}

fn call(name: &str, mut arguments: Value) -> Vec<u8> {
    if name == SYMBOLS_TOOL {
        arguments
            .as_object_mut()
            .unwrap()
            .insert("needle".into(), json!("item"));
    } else if matches!(
        name,
        DEFINITION_TOOL
            | REFERENCES_TOOL
            | IMPLEMENTATIONS_TOOL
            | INCOMING_CALLS_TOOL
            | OUTGOING_CALLS_TOOL
            | TYPE_AT_POINT_TOOL
    ) {
        arguments
            .as_object_mut()
            .unwrap()
            .insert("offset".into(), json!(4));
    } else if name == PLAN_RENAME_TOOL {
        let arguments = arguments.as_object_mut().unwrap();
        arguments.insert("offset".into(), json!(4));
        arguments.insert("new_name".into(), json!("renamed"));
    }
    serde_json::to_vec(&json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"_meta":meta(),"name":name,"arguments":arguments}
    }))
    .unwrap()
}

fn release_call(request: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"_meta":meta(),"name":RELEASE_REQUEST_TOOL,"arguments":{"request":request}}
    }))
    .unwrap()
}

fn value(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}

fn status(bytes: &[u8]) -> String {
    value(bytes)
        .pointer("/result/structuredContent/status")
        .and_then(Value::as_str)
        .unwrap()
        .to_owned()
}

fn provenance() -> SemanticProviderProvenanceV1 {
    SemanticProviderProvenanceV1::try_new(
        SemanticProviderRefV1::new([9; 16]),
        "rust-analyzer".into(),
        "1.0.0".into(),
    )
    .unwrap()
}

#[test]
fn outcome_unknown_requires_identical_caller_replay_before_resubmission() {
    let (mut server, port) = server();
    port.0.lock().unwrap().submit_outcomes.extend([
        SemanticSubmitOutcomeV1::OutcomeUnknown,
        SemanticSubmitOutcomeV1::Accepted,
    ]);
    let request = call(DIAGNOSTICS_TOOL, common_arguments());
    let first = run(server.handle(&request)).unwrap();
    assert_eq!(status(&first), "outcome_unknown");

    let mut changed = common_arguments();
    changed["language"] = json!("other");
    let conflict = run(server.handle(&call(DIAGNOSTICS_TOOL, changed))).unwrap();
    assert_eq!(status(&conflict), "refused");
    assert_eq!(
        value(&conflict)["result"]["structuredContent"]["reason"],
        "request_conflict"
    );
    assert_eq!(port.0.lock().unwrap().submissions.len(), 1);

    let replay = run(server.handle(&request)).unwrap();
    assert_eq!(status(&replay), "pending");
    assert_eq!(port.0.lock().unwrap().submissions.len(), 2);
    assert_eq!(server.usage().query_replays, 1);
}

fn symbols_result(target: SemanticTargetV1, count: usize) -> SemanticResultV1 {
    SemanticResultV1::try_symbols(
        (0..count)
            .map(|_| SemanticSymbolV1 {
                name: SemanticTextV1::try_new("symbol".into(), 64).unwrap(),
                kind: SemanticSymbolKindV1::Function,
                location: SemanticLocationV1 {
                    target,
                    span: SemanticSpanV1::try_new(4, 8).unwrap(),
                },
            })
            .collect(),
        SemanticLimitsV1::reference(),
    )
    .unwrap()
}

#[test]
fn catalog_is_fixed_read_only_and_has_no_identity_or_apply_arguments() {
    let (mut server, port) = server();
    let request = serde_json::to_vec(&json!({
        "jsonrpc":"2.0","id":1,"method":"tools/list","params":{"_meta":meta()}
    }))
    .unwrap();
    let response = value(&run(server.handle(&request)).unwrap());
    assert_eq!(response["result"]["resultType"], "complete");
    assert_eq!(response["result"]["_meta"], server_meta());
    let tools = response
        .pointer("/result/tools")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(response["result"]["ttlMs"], 0);
    assert_eq!(response["result"]["cacheScope"], "private");
    assert_eq!(tools.len(), 10);
    assert_eq!(
        tools
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        TOOLS
    );
    let encoded = serde_json::to_string(&tools).unwrap();
    assert!(!encoded.contains("rename_apply"));
    assert!(!encoded.contains("client_id"));
    assert!(!encoded.contains("auth"));
    assert!(tools.iter().all(|tool| {
        tool["annotations"]["readOnlyHint"]
            == json!(tool["name"].as_str() != Some(RELEASE_REQUEST_TOOL))
    }));
    assert!(port.0.lock().unwrap().submissions.is_empty());
}

#[test]
fn discovery_and_resource_catalog_match_the_canonical_golden_values() {
    let (mut server, _) = server();
    let discover = serde_json::to_vec(&json!({
        "jsonrpc":"2.0","id":"golden","method":"server/discover","params":{"_meta":meta()}
    }))
    .unwrap();
    let response = run(server.handle(&discover)).unwrap();
    let expected = json!({
        "jsonrpc":"2.0",
        "id":"golden",
        "result":{
            "resultType":"complete",
            "supportedVersions":[PROTOCOL_VERSION],
            "capabilities":{"tools":{"listChanged":false},"resources":{"listChanged":false,"subscribe":false}},
            "instructions":"Use exact request, document, source revision and index generation. Pending work is retrieved only by replaying identical arguments. Rename returns a plan and never applies it. Complete results disclose the provider fingerprint for semantic correlation.",
            "ttlMs":0,
            "cacheScope":"private",
            "_meta":server_meta()
        }
    });
    assert_eq!(response, serde_json::to_vec(&expected).unwrap());
    let resources = serde_json::to_vec(&json!({
        "jsonrpc":"2.0","id":2,"method":"resources/list","params":{"_meta":meta()}
    }))
    .unwrap();
    let expected = json!({"jsonrpc":"2.0","id":2,"result":{"resultType":"complete","resources":[
        {
            "uri":CAPABILITIES_RESOURCE,
            "name":"Vellum semantic capabilities",
            "description":"Content-safe readiness and capability lattice projection.",
            "mimeType":"application/json"
        },
        {
            "uri":DOCUMENTS_RESOURCE,
            "name":"Vellum semantic documents",
            "description":"Bounded locator-free native-to-semantic document identity and revision mapping.",
            "mimeType":"application/json"
        }
    ],"ttlMs":0,"cacheScope":"private","_meta":server_meta()}});
    assert_eq!(
        run(server.handle(&resources)).unwrap(),
        serde_json::to_vec(&expected).unwrap()
    );
}

#[test]
fn document_resource_exposes_exact_locator_free_query_coordinates() {
    let port = ModelPort::ready();
    let limits = SemanticMcpLimitsV1::reference();
    let document = SemanticMcpDocumentV1::try_new(
        7,
        11,
        SemanticDocumentRefV1::new([0x42; 16]),
        SemanticSourceRevisionV1::new(13),
        "rust".to_owned(),
        limits.semantic,
    )
    .unwrap();
    let mut server = SemanticMcpServerV1::try_new_test_with_documents(
        SemanticClientV1::new(port),
        limits,
        vec![document],
    )
    .unwrap();
    let request = serde_json::to_vec(&json!({
        "jsonrpc":"2.0","id":1,"method":"resources/read",
        "params":{"_meta":meta(),"uri":DOCUMENTS_RESOURCE}
    }))
    .unwrap();
    let response = value(&run(server.handle(&request)).unwrap());
    let text = response
        .pointer("/result/contents/0/text")
        .and_then(Value::as_str)
        .unwrap();
    let catalog: Value = serde_json::from_str(text).unwrap();
    assert_eq!(catalog["documents"][0]["native_workspace"], 7);
    assert_eq!(catalog["documents"][0]["native_document"], 11);
    assert_eq!(
        catalog["documents"][0]["document"],
        "42424242424242424242424242424242"
    );
    assert_eq!(catalog["documents"][0]["source_revision"], 13);
    assert_eq!(catalog["documents"][0]["language"], "rust");
    assert!(!text.contains("path"));
    assert!(!text.contains("locator"));
    assert!(!text.contains("contents"));
}

#[test]
fn modern_metadata_is_required_and_legacy_initialize_fails_forward() {
    let (mut server, _) = server();
    let missing = run(
        server.handle(br#"{"jsonrpc":"2.0","id":"missing","method":"tools/list","params":{}}"#)
    )
    .unwrap();
    assert_eq!(value(&missing)["error"]["code"], -32602);

    let unsupported = run(server.handle(
        &serde_json::to_vec(&json!({
            "jsonrpc":"2.0","id":"version","method":"tools/list","params":{"_meta":{
                "io.modelcontextprotocol/protocolVersion":"1900-01-01",
                "io.modelcontextprotocol/clientCapabilities":{}
            }}
        }))
        .unwrap(),
    ))
    .unwrap();
    let unsupported = value(&unsupported);
    assert_eq!(unsupported["error"]["code"], -32022);
    assert_eq!(
        unsupported["error"]["data"]["supported"],
        json!([PROTOCOL_VERSION])
    );
    assert_eq!(unsupported["error"]["data"]["requested"], "1900-01-01");

    let legacy = run(server.handle(
        br#"{"jsonrpc":"2.0","id":"legacy","method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"old","version":"1"}}}"#,
    ))
    .unwrap();
    let legacy = value(&legacy);
    assert_eq!(legacy["error"]["code"], -32022);
    assert_eq!(legacy["error"]["data"]["requested"], "2025-11-25");

    let oversized_id = "x".repeat(MAX_JSON_RPC_ID_BYTES + 1);
    let response = run(server.handle(
        &serde_json::to_vec(&json!({
            "jsonrpc":"2.0","id":oversized_id,"method":"tools/list","params":{"_meta":meta()}
        }))
        .unwrap(),
    ))
    .unwrap();
    let response = value(&response);
    assert_eq!(response["error"]["code"], -32600);
    assert!(response["id"].is_null());

    let conforming = json!({
        "io.modelcontextprotocol/protocolVersion":PROTOCOL_VERSION,
        "io.modelcontextprotocol/clientCapabilities":{
            "sampling":{},
            "extensions":{"com.example/feature":{},"com.example/":{}}
        },
        "io.modelcontextprotocol/clientInfo":{
            "name":"semantic-test","version":"1","title":"Semantic test",
            "description":"bounded test client","websiteUrl":"https://example.invalid",
            "icons":[{"src":"data:image/png;base64,AA==","mimeType":"image/png","sizes":["16x16"],"theme":"dark"}]
        },
        "io.modelcontextprotocol/logLevel":"notice",
        "traceparent":"00-0af7651916cd43dd8448eb211c80319c-00f067aa0ba902b7-01",
        "com.example/feature":{"enabled":true}
    });
    let accepted = run(server.handle(
        &serde_json::to_vec(&json!({
            "jsonrpc":"2.0","id":"metadata","method":"tools/list","params":{"_meta":conforming}
        }))
        .unwrap(),
    ))
    .unwrap();
    assert_eq!(value(&accepted)["result"]["resultType"], "complete");

    for extensions in [
        json!(true),
        json!({"unprefixed":{}}),
        json!({"com.example/feature":true}),
    ] {
        let mut invalid = meta();
        invalid["io.modelcontextprotocol/clientCapabilities"]["extensions"] = extensions;
        let response = run(server.handle(
            &serde_json::to_vec(&json!({
                "jsonrpc":"2.0","id":"extensions","method":"tools/list","params":{"_meta":invalid}
            }))
            .unwrap(),
        ))
        .unwrap();
        assert_eq!(value(&response)["error"]["code"], -32602);
    }

    for reserved in [
        "io.modelcontextprotocol/unknown",
        "dev.mcp/feature",
        "org.modelcontextprotocol.api/feature",
        "com.mcp.tools/feature",
    ] {
        let mut invalid = meta();
        invalid
            .as_object_mut()
            .unwrap()
            .insert(reserved.into(), json!(true));
        let response = run(server.handle(
            &serde_json::to_vec(&json!({
                "jsonrpc":"2.0","id":"reserved","method":"tools/list","params":{"_meta":invalid}
            }))
            .unwrap(),
        ))
        .unwrap();
        assert_eq!(value(&response)["error"]["code"], -32602);
    }

    let mut too_deep = json!(true);
    for _ in 0..=MAX_METADATA_DEPTH {
        too_deep = json!({"nested":too_deep});
    }
    for (key, extension) in [
        ("com.example/deep".to_owned(), too_deep),
        (
            "com.example/large".to_owned(),
            json!("x".repeat(MAX_METADATA_VALUE_BYTES + 1)),
        ),
        ("x".repeat(MAX_METADATA_KEY_BYTES + 1), json!(true)),
    ] {
        let mut invalid = meta();
        invalid.as_object_mut().unwrap().insert(key, extension);
        let response = run(server.handle(
            &serde_json::to_vec(&json!({
                "jsonrpc":"2.0","id":"bounded","method":"tools/list","params":{"_meta":invalid}
            }))
            .unwrap(),
        ))
        .unwrap();
        assert_eq!(value(&response)["error"]["code"], -32602);
    }

    let cursor = run(server.handle(
        &serde_json::to_vec(&json!({
            "jsonrpc":"2.0","id":"cursor","method":"resources/list",
            "params":{"_meta":meta(),"cursor":"opaque-but-unissued"}
        }))
        .unwrap(),
    ))
    .unwrap();
    assert_eq!(value(&cursor)["error"]["code"], -32602);
}

#[test]
fn drained_frame_remains_unexecuted_until_composition_serves_it() {
    let (mut server, _) = server();
    let request = serde_json::to_vec(&json!({
        "jsonrpc":"2.0","id":1,"method":"resources/list","params":{"_meta":meta()}
    }))
    .unwrap();
    let mut framed = request;
    framed.push(b'\n');
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let mut reader = tokio::io::BufReader::new(framed.as_slice());
    let mut usage = SemanticMcpStdioUsageV1::default();
    let frame = runtime
        .block_on(read_stdio_frame_v1(
            &mut reader,
            SemanticMcpStdioLimitsV1::reference(),
            server.limits().max_request_bytes,
            &mut usage,
        ))
        .unwrap();

    assert_eq!(server.usage().requests, 0);
    assert_eq!(usage.frames, 0);
    let mut writer = Vec::new();
    assert_eq!(
        runtime
            .block_on(serve_stdio_read_frame_v1(
                &mut server,
                frame,
                &mut writer,
                &mut usage,
            ))
            .unwrap(),
        SemanticMcpStdioStepV1::Served
    );
    assert_eq!(server.usage().requests, 1);
    assert_eq!(usage.frames, 1);
    assert!(writer.ends_with(b"\n"));
}

#[test]
fn notification_and_recursive_duplicate_keys_never_submit_work() {
    let (mut server, port) = server();
    let notification = serde_json::to_vec(&json!({
        "jsonrpc":"2.0","method":"tools/call",
        "params":{"_meta":meta(),"name":DIAGNOSTICS_TOOL,"arguments":common_arguments()}
    }))
    .unwrap();
    assert!(run(server.handle(&notification)).is_none());
    let duplicate = br#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}},"name":"vellum.semantic.diagnostics","arguments":{"request":"01010101010101010101010101010101","document":"02020202020202020202020202020202","source_revision":3,"index_generation":7,"language":"rust","language":"go"}}}"#;
    let response = run(server.handle(duplicate)).unwrap();
    assert_eq!(
        value(&response)
            .pointer("/error/code")
            .and_then(Value::as_i64),
        Some(-32700)
    );
    assert!(port.0.lock().unwrap().submissions.is_empty());
}

#[test]
fn all_nine_query_tools_build_exact_requests_and_changed_replay_conflicts() {
    for name in [
        SYMBOLS_TOOL,
        DEFINITION_TOOL,
        REFERENCES_TOOL,
        IMPLEMENTATIONS_TOOL,
        INCOMING_CALLS_TOOL,
        OUTGOING_CALLS_TOOL,
        DIAGNOSTICS_TOOL,
        TYPE_AT_POINT_TOOL,
        PLAN_RENAME_TOOL,
    ] {
        let (mut server, port) = server();
        let response = run(server.handle(&call(name, common_arguments()))).unwrap();
        assert_eq!(status(&response), "pending");
        let response_value = value(&response);
        assert_eq!(response_value["result"]["resultType"], "complete");
        assert_eq!(response_value["result"]["_meta"], server_meta());
        let model = port.0.lock().unwrap();
        assert_eq!(model.submissions.len(), 1);
        assert_eq!(
            model.submissions[0].query.kind(),
            match name {
                SYMBOLS_TOOL => SemanticQueryKindV1::Symbols,
                DEFINITION_TOOL => SemanticQueryKindV1::Definition,
                REFERENCES_TOOL => SemanticQueryKindV1::References,
                IMPLEMENTATIONS_TOOL => SemanticQueryKindV1::Implementations,
                INCOMING_CALLS_TOOL => SemanticQueryKindV1::IncomingCalls,
                OUTGOING_CALLS_TOOL => SemanticQueryKindV1::OutgoingCalls,
                DIAGNOSTICS_TOOL => SemanticQueryKindV1::Diagnostics,
                TYPE_AT_POINT_TOOL => SemanticQueryKindV1::TypeAtPoint,
                PLAN_RENAME_TOOL => SemanticQueryKindV1::PlanRename,
                _ => unreachable!(),
            }
        );
    }

    let (mut server, port) = server();
    assert_eq!(
        status(&run(server.handle(&call(DIAGNOSTICS_TOOL, common_arguments()))).unwrap()),
        "pending"
    );
    let mut changed = common_arguments();
    changed["source_revision"] = json!(4);
    let response = run(server.handle(&call(DIAGNOSTICS_TOOL, changed))).unwrap();
    assert_eq!(status(&response), "refused");
    assert_eq!(
        value(&response)
            .pointer("/result/structuredContent/reason")
            .and_then(Value::as_str),
        Some("request_conflict")
    );
    assert_eq!(port.0.lock().unwrap().submissions.len(), 1);
}

#[test]
fn retained_request_capacity_never_evicts_or_reuses_an_identity() {
    let port = ModelPort::ready();
    let mut limits = SemanticMcpLimitsV1::reference();
    limits.max_retained_requests = 1;
    let mut server =
        SemanticMcpServerV1::try_new_test(SemanticClientV1::new(port.clone()), limits).unwrap();
    let first = call(DIAGNOSTICS_TOOL, common_arguments());
    assert_eq!(status(&run(server.handle(&first)).unwrap()), "pending");
    let mut second_arguments = common_arguments();
    second_arguments["request"] = json!("03030303030303030303030303030303");
    let second = run(server.handle(&call(DIAGNOSTICS_TOOL, second_arguments))).unwrap();
    assert_eq!(status(&second), "refused");
    assert_eq!(
        value(&second)
            .pointer("/result/structuredContent/reason")
            .and_then(Value::as_str),
        Some("capacity")
    );
    assert_eq!(status(&run(server.handle(&first)).unwrap()), "pending");
    assert_eq!(port.0.lock().unwrap().submissions.len(), 1);
}

#[test]
fn pending_exact_replay_delivers_once_and_replays_terminal_without_resubmit() {
    let (mut server, port) = server();
    let request = call(SYMBOLS_TOOL, common_arguments());
    assert_eq!(status(&run(server.handle(&request)).unwrap()), "pending");
    let target = SemanticTargetV1 {
        document: SemanticDocumentRefV1::new([2; 16]),
        source_revision: SemanticSourceRevisionV1::new(3),
    };
    let location = SemanticLocationV1 {
        target,
        span: SemanticSpanV1::try_new(4, 8).unwrap(),
    };
    let result = SemanticResultV1::try_symbols(
        vec![SemanticSymbolV1 {
            name: SemanticTextV1::try_new("secret_symbol".into(), 64).unwrap(),
            kind: SemanticSymbolKindV1::Function,
            location,
        }],
        SemanticLimitsV1::reference(),
    )
    .unwrap();
    port.0
        .lock()
        .unwrap()
        .deliveries
        .push_back(SemanticDeliveryV1 {
            request: SemanticRequestRefV1::new([1; 16]),
            outcome: SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Complete(
                SemanticReplyV1 {
                    request: SemanticRequestRefV1::new([1; 16]),
                    target,
                    index_generation: SemanticIndexGenerationV1::new(7).unwrap(),
                    provider: provenance(),
                    result,
                },
            )),
        });
    let complete = run(server.handle(&request)).unwrap();
    assert_eq!(status(&complete), "complete");
    assert_eq!(
        value(&complete)
            .pointer("/result/structuredContent/result/items/0/name")
            .and_then(Value::as_str),
        Some("secret_symbol")
    );
    let replay = run(server.handle(&request)).unwrap();
    assert_eq!(complete, replay);
    assert_eq!(port.0.lock().unwrap().submissions.len(), 1);
}

#[test]
fn terminal_result_is_replayable_until_explicit_release_then_capacity_is_reclaimed() {
    let port = ModelPort::ready();
    let mut limits = SemanticMcpLimitsV1::reference();
    limits.max_retained_requests = 1;
    let mut server =
        SemanticMcpServerV1::try_new_test(SemanticClientV1::new(port.clone()), limits).unwrap();
    let request = call(DIAGNOSTICS_TOOL, common_arguments());
    assert_eq!(status(&run(server.handle(&request)).unwrap()), "pending");
    port.0
        .lock()
        .unwrap()
        .deliveries
        .push_back(SemanticDeliveryV1 {
            request: SemanticRequestRefV1::new([1; 16]),
            outcome: SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Complete(
                SemanticReplyV1 {
                    request: SemanticRequestRefV1::new([1; 16]),
                    target: SemanticTargetV1 {
                        document: SemanticDocumentRefV1::new([2; 16]),
                        source_revision: SemanticSourceRevisionV1::new(3),
                    },
                    index_generation: SemanticIndexGenerationV1::new(7).unwrap(),
                    provider: provenance(),
                    result: SemanticResultV1::try_diagnostics(
                        Vec::new(),
                        SemanticLimitsV1::reference(),
                    )
                    .unwrap(),
                },
            )),
        });
    assert_eq!(status(&run(server.handle(&request)).unwrap()), "complete");
    let released = run(server.handle(&release_call("01010101010101010101010101010101"))).unwrap();
    assert_eq!(status(&released), "released");
    assert_eq!(
        port.0.lock().unwrap().releases,
        vec![SemanticRequestRefV1::new([1; 16])]
    );
    assert_eq!(server.usage().released, 1);

    let mut next = common_arguments();
    next["request"] = json!("03030303030303030303030303030303");
    assert_eq!(
        status(&run(server.handle(&call(DIAGNOSTICS_TOOL, next))).unwrap()),
        "pending"
    );
}

#[test]
fn hostile_transplanted_deliveries_are_refused_without_disclosure() {
    enum Mutation {
        Request,
        Target,
        Index,
        Family,
        Limit,
        UnsupportedKind,
    }
    for mutation in [
        Mutation::Request,
        Mutation::Target,
        Mutation::Index,
        Mutation::Family,
        Mutation::Limit,
        Mutation::UnsupportedKind,
    ] {
        let port = ModelPort::ready();
        let mut limits = SemanticMcpLimitsV1::reference();
        if matches!(mutation, Mutation::Limit) {
            limits.semantic.max_results = 1;
        }
        let mut server =
            SemanticMcpServerV1::try_new_test(SemanticClientV1::new(port.clone()), limits).unwrap();
        let request = call(SYMBOLS_TOOL, common_arguments());
        assert_eq!(status(&run(server.handle(&request)).unwrap()), "pending");
        let target = SemanticTargetV1 {
            document: SemanticDocumentRefV1::new([2; 16]),
            source_revision: SemanticSourceRevisionV1::new(3),
        };
        let outcome = match mutation {
            Mutation::UnsupportedKind => {
                SemanticOutcomeV1::Unsupported(SemanticQueryKindV1::Diagnostics)
            }
            Mutation::Family => SemanticOutcomeV1::Complete(SemanticReplyV1 {
                request: SemanticRequestRefV1::new([1; 16]),
                target,
                index_generation: SemanticIndexGenerationV1::new(7).unwrap(),
                provider: provenance(),
                result: SemanticResultV1::try_locations(
                    vec![SemanticLocationV1 {
                        target,
                        span: SemanticSpanV1::try_new(4, 8).unwrap(),
                    }],
                    SemanticLimitsV1::reference(),
                )
                .unwrap(),
            }),
            _ => SemanticOutcomeV1::Complete(SemanticReplyV1 {
                request: SemanticRequestRefV1::new(if matches!(mutation, Mutation::Request) {
                    [8; 16]
                } else {
                    [1; 16]
                }),
                target: if matches!(mutation, Mutation::Target) {
                    SemanticTargetV1 {
                        document: SemanticDocumentRefV1::new([7; 16]),
                        source_revision: SemanticSourceRevisionV1::new(3),
                    }
                } else {
                    target
                },
                index_generation: SemanticIndexGenerationV1::new(
                    if matches!(mutation, Mutation::Index) {
                        8
                    } else {
                        7
                    },
                )
                .unwrap(),
                provider: provenance(),
                result: symbols_result(
                    target,
                    if matches!(mutation, Mutation::Limit) {
                        2
                    } else {
                        1
                    },
                ),
            }),
        };
        port.0
            .lock()
            .unwrap()
            .deliveries
            .push_back(SemanticDeliveryV1 {
                request: SemanticRequestRefV1::new([1; 16]),
                outcome: SemanticDeliveryOutcomeV1::Peer(outcome),
            });
        let response = run(server.handle(&request)).unwrap();
        assert_eq!(status(&response), "refused");
        let encoded = String::from_utf8(response).unwrap();
        assert!(encoded.contains("malformed_delivery"));
        assert!(!encoded.contains("rust-analyzer"));
        assert!(!encoded.contains("symbol"));
    }
}

#[test]
fn unavailable_stale_unsupported_refused_cancelled_and_timeout_are_distinct() {
    let immediate = [
        (SemanticSubmitOutcomeV1::Unavailable, "unavailable"),
        (SemanticSubmitOutcomeV1::Stale, "stale"),
        (
            SemanticSubmitOutcomeV1::Unsupported(SemanticQueryKindV1::Diagnostics),
            "unsupported",
        ),
        (
            SemanticSubmitOutcomeV1::Refused(SemanticServiceRefusalV1::ProviderRefused),
            "refused",
        ),
    ];
    for (outcome, expected) in immediate {
        let (mut server, port) = server();
        port.0.lock().unwrap().submit_outcomes.push_back(outcome);
        assert_eq!(
            status(&run(server.handle(&call(DIAGNOSTICS_TOOL, common_arguments()))).unwrap()),
            expected
        );
    }
    for (outcome, expected) in [
        (SemanticDeliveryOutcomeV1::Cancelled, "cancelled"),
        (SemanticDeliveryOutcomeV1::TimedOut, "timed_out"),
        (
            SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Stale),
            "stale",
        ),
    ] {
        let (mut server, port) = server();
        let request = call(DIAGNOSTICS_TOOL, common_arguments());
        assert_eq!(status(&run(server.handle(&request)).unwrap()), "pending");
        port.0
            .lock()
            .unwrap()
            .deliveries
            .push_back(SemanticDeliveryV1 {
                request: SemanticRequestRefV1::new([1; 16]),
                outcome,
            });
        assert_eq!(status(&run(server.handle(&request)).unwrap()), expected);
    }
}

#[test]
fn capability_resource_is_content_safe_and_response_overflow_is_one_bounded_error() {
    let (mut server, _) = server();
    let request = serde_json::to_vec(&json!({
        "jsonrpc":"2.0","id":1,"method":"resources/read",
        "params":{"_meta":meta(),"uri":CAPABILITIES_RESOURCE}
    }))
    .unwrap();
    let response = run(server.handle(&request)).unwrap();
    let response_value = value(&response);
    assert_eq!(response_value["result"]["resultType"], "complete");
    assert_eq!(response_value["result"]["_meta"], server_meta());
    assert_eq!(response_value["result"]["ttlMs"], 0);
    assert_eq!(response_value["result"]["cacheScope"], "private");
    let text = response_value
        .pointer("/result/contents/0/text")
        .and_then(Value::as_str)
        .unwrap();
    assert!(text.contains("read_index"));
    assert!(!text.contains("provider"));
    assert!(!text.contains("profile"));

    let port = ModelPort::ready();
    let mut limits = SemanticMcpLimitsV1::reference();
    limits.max_response_bytes = 256;
    let mut bounded =
        SemanticMcpServerV1::try_new_test(SemanticClientV1::new(port), limits).unwrap();
    let response = run(bounded.handle(
        &serde_json::to_vec(&json!({
            "jsonrpc":"2.0","id":"correlation-7","method":"tools/list","params":{"_meta":meta()}
        }))
        .unwrap(),
    ))
    .unwrap();
    assert!(response.len() <= 256);
    assert_eq!(
        value(&response)
            .pointer("/error/code")
            .and_then(Value::as_i64),
        Some(-32603)
    );
    assert_eq!(value(&response)["id"], "correlation-7");
}

#[test]
fn debug_and_errors_do_not_disclose_query_or_result_text() {
    let request = SemanticRequestV1 {
        request: SemanticRequestRefV1::new([1; 16]),
        target: SemanticTargetV1 {
            document: SemanticDocumentRefV1::new([2; 16]),
            source_revision: SemanticSourceRevisionV1::new(3),
        },
        expected_index: SemanticIndexGenerationV1::new(7),
        language: SemanticTextV1::try_new("secret-language".into(), 64).unwrap(),
        query: SemanticQueryV1::Symbols {
            needle: SemanticTextV1::try_new("secret-needle".into(), 64).unwrap(),
        },
    };
    let debug = format!("{request:?}");
    assert!(!debug.contains("secret-language"));
    assert!(!debug.contains("secret-needle"));
    let (mut server, _) = server();
    let response = run(server.handle(b"not secret valid json")).unwrap();
    assert!(
        !String::from_utf8(response)
            .unwrap()
            .contains("not secret valid json")
    );
}
