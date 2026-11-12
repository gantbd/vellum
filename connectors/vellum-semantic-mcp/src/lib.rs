//! Bounded MCP projection and stdio driver for a sovereign-validated,
//! partition-bound semantic client.
//!
//! This crate owns no provider, process, workspace, authentication, clock, edit,
//! or transport authority. Seven query tools submit revision/index-bound read
//! or rename-plan requests through an already-bound client port; one lifecycle
//! tool explicitly releases a retained terminal result. Rename application is
//! deliberately absent. The library is prepared for a later
//! transport/extraction boundary; it is not itself a transport or deployable
//! peer. Complete query results deliberately disclose the bounded provider
//! fingerprint (opaque id, profile, and version) needed to correlate semantic
//! behavior. Capability observation remains content-safe and omits it.
//! Construction is closed to the semantic client's sealed validation port, so
//! an arbitrary public alternate port cannot bypass nested document, revision,
//! span, or rename-range validation.
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value, json};
use vellum_sdk_semantic_client::{
    client as vellum_semantic_client, session as vellum_semantic_session_client,
};
use vellum_semantic_client::{
    MAX_SEMANTIC_LABEL_BYTES_V1, MAX_SEMANTIC_QUERY_BYTES_V1, SemanticCapabilityV1,
    SemanticClientV1, SemanticDeliveryOutcomeV1, SemanticDiagnosticSeverityV1,
    SemanticDocumentRefV1, SemanticIndexGenerationV1, SemanticLimitsV1, SemanticLocationV1,
    SemanticOutcomeV1, SemanticProviderReadinessV1, SemanticQueryKindV1, SemanticQueryV1,
    SemanticRefusalV1, SemanticReleaseOutcomeV1, SemanticReplyV1, SemanticRequestRefV1,
    SemanticRequestV1, SemanticServicePortV1, SemanticServiceRefusalV1, SemanticSessionQueryPortV1,
    SemanticSessionQueryReconnectV1, SemanticSessionQueryResumeRefusalV1,
    SemanticSessionQuerySuspensionV1, SemanticSourceRevisionV1, SemanticSubmitOutcomeV1,
    SemanticSymbolKindV1, SemanticTargetV1, SemanticTextV1,
    SovereignValidatedSemanticServicePortV1,
};
use vellum_semantic_session_client::{SemanticSessionClientV1, SemanticSessionExchangePortV1};

mod stdio;
pub use stdio::{
    SemanticMcpStdioFailureV1, SemanticMcpStdioFrameV1, SemanticMcpStdioLimitsV1,
    SemanticMcpStdioStepV1, SemanticMcpStdioUsageV1, read_stdio_frame_v1, serve_stdio_frame_v1,
    serve_stdio_read_frame_v1, serve_stdio_v1,
};

struct StrictJsonValue(Value);

impl<'de> Deserialize<'de> for StrictJsonValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(StrictJsonVisitor)
    }
}

struct StrictJsonVisitor;

impl<'de> Visitor<'de> for StrictJsonVisitor {
    type Value = StrictJsonValue;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON value without duplicate object keys")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(StrictJsonValue(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(StrictJsonValue(Value::Number(Number::from(value))))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(StrictJsonValue(Value::Number(Number::from(value))))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        Number::from_f64(value)
            .map(Value::Number)
            .map(StrictJsonValue)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        self.visit_string(value.to_owned())
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(StrictJsonValue(Value::String(value)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(StrictJsonValue(Value::Null))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(StrictJsonValue(Value::Null))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(StrictJsonValue(value)) = sequence.next_element::<StrictJsonValue>()? {
            values.push(value);
        }
        Ok(StrictJsonValue(Value::Array(values)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Self::Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = object.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom("duplicate JSON object key"));
            }
            let StrictJsonValue(value) = object.next_value::<StrictJsonValue>()?;
            values.insert(key, value);
        }
        Ok(StrictJsonValue(Value::Object(values)))
    }
}

pub const PROTOCOL_VERSION: &str = "2026-07-28";
pub const SYMBOLS_TOOL: &str = "vellum.semantic.symbols";
pub const DEFINITION_TOOL: &str = "vellum.semantic.definition";
pub const REFERENCES_TOOL: &str = "vellum.semantic.references";
pub const IMPLEMENTATIONS_TOOL: &str = "vellum.semantic.implementations";
pub const INCOMING_CALLS_TOOL: &str = "vellum.semantic.incoming_calls";
pub const OUTGOING_CALLS_TOOL: &str = "vellum.semantic.outgoing_calls";
pub const DIAGNOSTICS_TOOL: &str = "vellum.semantic.diagnostics";
pub const TYPE_AT_POINT_TOOL: &str = "vellum.semantic.type_at_point";
pub const PLAN_RENAME_TOOL: &str = "vellum.semantic.plan_rename";
pub const RELEASE_REQUEST_TOOL: &str = "vellum.semantic.release_request";
pub const CAPABILITIES_RESOURCE: &str = "vellum-semantic://capabilities";
pub const DOCUMENTS_RESOURCE: &str = "vellum-semantic://documents";
pub const MAX_SEMANTIC_MCP_DOCUMENTS_V1: usize = 4_096;
const MAX_JSON_RPC_ID_BYTES: usize = 128;
const MAX_IMPLEMENTATION_FIELD_BYTES: usize = 128;
const MAX_CLIENT_INFO_BYTES: usize = 8 * 1024;
const MAX_METADATA_VALUE_BYTES: usize = 8 * 1024;
const MAX_CLIENT_CAPABILITIES_BYTES: usize = 8 * 1024;
const MAX_METADATA_BYTES: usize = 24 * 1024;
const MAX_METADATA_KEY_BYTES: usize = 256;
const MAX_METADATA_DEPTH: usize = 16;
const MAX_CURSOR_BYTES: usize = 256;

const TOOLS: [&str; 10] = [
    SYMBOLS_TOOL,
    DEFINITION_TOOL,
    REFERENCES_TOOL,
    IMPLEMENTATIONS_TOOL,
    INCOMING_CALLS_TOOL,
    OUTGOING_CALLS_TOOL,
    DIAGNOSTICS_TOOL,
    TYPE_AT_POINT_TOOL,
    PLAN_RENAME_TOOL,
    RELEASE_REQUEST_TOOL,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticMcpLimitsV1 {
    pub max_request_bytes: usize,
    pub max_response_bytes: usize,
    pub max_retained_requests: usize,
    pub semantic: SemanticLimitsV1,
}

impl SemanticMcpLimitsV1 {
    pub const fn reference() -> Self {
        Self {
            max_request_bytes: 256 * 1024,
            max_response_bytes: 2 * 1024 * 1024,
            max_retained_requests: 1_024,
            semantic: SemanticLimitsV1::reference(),
        }
    }

    const fn is_valid(self) -> bool {
        self.max_request_bytes > 0
            && self.max_request_bytes <= 256 * 1024
            && self.max_response_bytes >= 256
            && self.max_response_bytes <= 2 * 1024 * 1024
            && self.max_retained_requests > 0
            && self.max_retained_requests <= 1_024
            && self.semantic.is_valid()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticMcpBuildRefusalV1 {
    InvalidLimits,
    InvalidDocuments,
}

/// Content-free mapping needed to formulate exact semantic queries. Native
/// ordinals are scoped by the workspace and reveal neither locators nor text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticMcpDocumentV1 {
    native_workspace: u64,
    native_document: u64,
    document: SemanticDocumentRefV1,
    source_revision: SemanticSourceRevisionV1,
    language: String,
}

impl SemanticMcpDocumentV1 {
    pub fn try_new(
        native_workspace: u64,
        native_document: u64,
        document: SemanticDocumentRefV1,
        source_revision: SemanticSourceRevisionV1,
        language: String,
        limits: SemanticLimitsV1,
    ) -> Option<Self> {
        (native_workspace > 0
            && native_document > 0
            && !language.is_empty()
            && language.len() <= limits.max_label_bytes)
            .then_some(Self {
                native_workspace,
                native_document,
                document,
                source_revision,
                language,
            })
    }

    pub const fn native_workspace(&self) -> u64 {
        self.native_workspace
    }

    pub const fn native_document(&self) -> u64 {
        self.native_document
    }

    pub const fn document(&self) -> SemanticDocumentRefV1 {
        self.document
    }

    pub const fn source_revision(&self) -> SemanticSourceRevisionV1 {
        self.source_revision
    }

    pub fn language(&self) -> &str {
        &self.language
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SemanticMcpUsageV1 {
    pub requests: u64,
    pub request_bytes: u64,
    pub response_bytes: u64,
    pub tool_calls: u64,
    pub queries_submitted: u64,
    pub query_replays: u64,
    pub pending: u64,
    pub completed: u64,
    pub cancelled: u64,
    pub timed_out: u64,
    pub released: u64,
    pub refused: u64,
    pub protocol_errors: u64,
    pub malformed_deliveries: u64,
    pub saturated: bool,
}

struct QueryRecord {
    request: SemanticRequestV1,
    outcome: Option<Value>,
    outcome_unknown: bool,
}

/// One MCP session over one already-bound semantic client partition.
pub struct SemanticMcpServerV1<P> {
    client: SemanticClientV1<P>,
    limits: SemanticMcpLimitsV1,
    documents: Vec<SemanticMcpDocumentV1>,
    records: BTreeMap<SemanticRequestRefV1, QueryRecord>,
    usage: SemanticMcpUsageV1,
}

/// Affine MCP pause between stdio frames. Request records, usage, the current
/// catalog, query correlations, and collection obligations all remain owned.
#[must_use = "a suspended MCP server must be resumed or explicitly retired"]
pub struct SemanticMcpServerSuspensionV1<P> {
    client: SemanticSessionQuerySuspensionV1<P>,
    limits: SemanticMcpLimitsV1,
    documents: Vec<SemanticMcpDocumentV1>,
    records: BTreeMap<SemanticRequestRefV1, QueryRecord>,
    usage: SemanticMcpUsageV1,
}

/// Sessionless MCP owner used only across a definitive semantic transport
/// reconnect. MCP request records, metrics, catalog and query correlations are
/// retained without carrying the retired session or transport.
#[must_use = "an MCP reconnect owner must bind a fresh session or be explicitly abandoned"]
pub struct SemanticMcpServerReconnectV1 {
    client: SemanticSessionQueryReconnectV1,
    limits: SemanticMcpLimitsV1,
    documents: Vec<SemanticMcpDocumentV1>,
    records: BTreeMap<SemanticRequestRefV1, QueryRecord>,
    usage: SemanticMcpUsageV1,
}

#[must_use = "reconnect refusal retains the complete MCP owner and candidate session"]
pub struct SemanticMcpServerReconnectFailureV1<P> {
    reason: SemanticSessionQueryResumeRefusalV1,
    reconnect: SemanticMcpServerReconnectV1,
    session: SemanticSessionClientV1<P>,
}

impl<P> SemanticMcpServerReconnectFailureV1<P> {
    pub const fn reason(&self) -> SemanticSessionQueryResumeRefusalV1 {
        self.reason
    }

    pub fn into_parts(self) -> (SemanticMcpServerReconnectV1, SemanticSessionClientV1<P>) {
        (self.reconnect, self.session)
    }
}

impl<P> std::fmt::Debug for SemanticMcpServerReconnectFailureV1<P> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SemanticMcpServerReconnectFailureV1")
            .field("reason", &self.reason)
            .field("reconnect", &"<owned>")
            .field("session", &"<owned>")
            .finish()
    }
}

impl std::fmt::Debug for SemanticMcpServerReconnectV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SemanticMcpServerReconnectV1")
            .field("documents", &self.documents.len())
            .field("records", &self.records.len())
            .field("usage", &self.usage)
            .field("client", &"<owned>")
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticMcpServerResumeRefusalV1 {
    InvalidDocuments,
    Query(SemanticSessionQueryResumeRefusalV1),
}

#[must_use = "resume refusal retains the exact suspended MCP owner"]
pub struct SemanticMcpServerResumeFailureV1<P> {
    reason: SemanticMcpServerResumeRefusalV1,
    suspension: SemanticMcpServerSuspensionV1<P>,
}

impl<P> SemanticMcpServerResumeFailureV1<P> {
    pub const fn reason(&self) -> SemanticMcpServerResumeRefusalV1 {
        self.reason
    }

    pub fn into_suspension(self) -> SemanticMcpServerSuspensionV1<P> {
        self.suspension
    }
}

impl<P> std::fmt::Debug for SemanticMcpServerResumeFailureV1<P> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SemanticMcpServerResumeFailureV1")
            .field("reason", &self.reason)
            .field("suspension", &"<owned>")
            .finish()
    }
}

fn validate_documents(
    limits: SemanticMcpLimitsV1,
    documents: &mut [SemanticMcpDocumentV1],
) -> Result<(), SemanticMcpBuildRefusalV1> {
    documents.sort_by_key(|value| (value.native_workspace, value.native_document));
    let semantic_documents: BTreeSet<_> = documents.iter().map(|value| value.document).collect();
    if documents.len() > MAX_SEMANTIC_MCP_DOCUMENTS_V1
        || semantic_documents.len() != documents.len()
        || documents.windows(2).any(|pair| {
            pair[0].native_workspace == pair[1].native_workspace
                && pair[0].native_document == pair[1].native_document
        })
        || documents
            .iter()
            .any(|value| value.language.len() > limits.semantic.max_label_bytes)
    {
        Err(SemanticMcpBuildRefusalV1::InvalidDocuments)
    } else {
        Ok(())
    }
}

impl<P: SovereignValidatedSemanticServicePortV1> SemanticMcpServerV1<P> {
    pub fn try_new(
        client: SemanticClientV1<P>,
        limits: SemanticMcpLimitsV1,
    ) -> Result<Self, SemanticMcpBuildRefusalV1> {
        Self::build(client, limits, Vec::new())
    }

    pub fn try_new_with_documents(
        client: SemanticClientV1<P>,
        limits: SemanticMcpLimitsV1,
        documents: Vec<SemanticMcpDocumentV1>,
    ) -> Result<Self, SemanticMcpBuildRefusalV1> {
        Self::build(client, limits, documents)
    }
}

impl<P: SemanticServicePortV1> SemanticMcpServerV1<P> {
    fn build(
        client: SemanticClientV1<P>,
        limits: SemanticMcpLimitsV1,
        mut documents: Vec<SemanticMcpDocumentV1>,
    ) -> Result<Self, SemanticMcpBuildRefusalV1> {
        if !limits.is_valid() {
            return Err(SemanticMcpBuildRefusalV1::InvalidLimits);
        }
        validate_documents(limits, &mut documents)?;
        Ok(Self {
            client,
            limits,
            documents,
            records: BTreeMap::new(),
            usage: SemanticMcpUsageV1::default(),
        })
    }

    #[cfg(test)]
    fn try_new_test(
        client: SemanticClientV1<P>,
        limits: SemanticMcpLimitsV1,
    ) -> Result<Self, SemanticMcpBuildRefusalV1> {
        Self::build(client, limits, Vec::new())
    }

    #[cfg(test)]
    fn try_new_test_with_documents(
        client: SemanticClientV1<P>,
        limits: SemanticMcpLimitsV1,
        documents: Vec<SemanticMcpDocumentV1>,
    ) -> Result<Self, SemanticMcpBuildRefusalV1> {
        Self::build(client, limits, documents)
    }

    pub const fn usage(&self) -> SemanticMcpUsageV1 {
        self.usage
    }

    pub const fn limits(&self) -> SemanticMcpLimitsV1 {
        self.limits
    }

    pub fn into_client(self) -> SemanticClientV1<P> {
        self.client
    }

    /// Handles one newline-free MCP JSON-RPC message. Notifications have no
    /// response. Accepted provider work is returned as explicit `pending`; an
    /// exact replay retrieves the retained terminal outcome without resubmission.
    pub async fn handle(&mut self, bytes: &[u8]) -> Option<Vec<u8>> {
        bump(&mut self.usage.requests, &mut self.usage.saturated);
        add(
            &mut self.usage.request_bytes,
            bytes.len(),
            &mut self.usage.saturated,
        );
        if bytes.len() > self.limits.max_request_bytes {
            return Some(self.protocol_error_bytes(Value::Null, -32600, "request exceeds limit"));
        }
        let value: Value = match serde_json::from_slice::<StrictJsonValue>(bytes) {
            Ok(StrictJsonValue(value)) => value,
            Err(_) => return Some(self.protocol_error_bytes(Value::Null, -32700, "parse error")),
        };
        let Some(object) = value.as_object() else {
            return Some(self.protocol_error_bytes(Value::Null, -32600, "invalid request"));
        };
        if !only_keys(object, &["jsonrpc", "id", "method", "params"])
            || object.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
        {
            return object
                .get("id")
                .cloned()
                .map(|id| self.protocol_error_bytes(id, -32600, "invalid request"));
        }
        let id = match object.get("id") {
            Some(id @ (Value::String(_) | Value::Number(_))) if valid_id(id) => Some(id.clone()),
            Some(_) => {
                return Some(self.protocol_error_bytes(Value::Null, -32600, "invalid request id"));
            }
            None => None,
        };
        let Some(method) = object.get("method").and_then(Value::as_str) else {
            return id.map(|id| self.protocol_error_bytes(id, -32600, "invalid request"));
        };
        let id = id?;
        if method == "initialize" {
            let requested = object
                .get("params")
                .and_then(Value::as_object)
                .and_then(|params| params.get("protocolVersion"))
                .and_then(Value::as_str)
                .filter(|value| value.len() <= MAX_IMPLEMENTATION_FIELD_BYTES);
            return Some(self.protocol_error_data_bytes(
                id,
                -32022,
                "unsupported protocol version",
                unsupported_version_data(requested),
            ));
        }
        let Some(params) = object.get("params").and_then(Value::as_object) else {
            return Some(self.protocol_error_bytes(id, -32602, "invalid params"));
        };
        if let Err(requested) = validate_meta(params.get("_meta")) {
            return Some(match requested {
                Some(requested) => self.protocol_error_data_bytes(
                    id,
                    -32022,
                    "unsupported protocol version",
                    unsupported_version_data(Some(&requested)),
                ),
                None => self.protocol_error_bytes(id, -32602, "invalid request metadata"),
            });
        }

        let response = match method {
            "server/discover" if only_keys(params, &["_meta"]) => success(id, discover_result()),
            "tools/list" if list_params(params).is_ok() => {
                if params.contains_key("cursor") {
                    self.protocol_error(id, -32602, "cursor is not supported")
                } else {
                    success(id, tools_result())
                }
            }
            "tools/list" => self.protocol_error(id, -32602, "invalid list params"),
            "tools/call" => self.call_tool(id, params).await,
            "resources/list" if list_params(params).is_ok() => {
                if params.contains_key("cursor") {
                    self.protocol_error(id, -32602, "cursor is not supported")
                } else {
                    success(id, resources_result())
                }
            }
            "resources/list" => self.protocol_error(id, -32602, "invalid list params"),
            "resources/read" if only_keys(params, &["_meta", "uri"]) => {
                self.read_resource(id, params).await
            }
            _ => self.protocol_error(id, -32601, "method not found"),
        };
        Some(self.encode(response))
    }

    fn reject_truncated_frame(&mut self, bytes: usize) -> Vec<u8> {
        bump(&mut self.usage.requests, &mut self.usage.saturated);
        add(
            &mut self.usage.request_bytes,
            bytes,
            &mut self.usage.saturated,
        );
        self.protocol_error_bytes(Value::Null, -32700, "truncated request")
    }

    fn reject_oversized_frame(&mut self, bytes: usize) -> Vec<u8> {
        bump(&mut self.usage.requests, &mut self.usage.saturated);
        add(
            &mut self.usage.request_bytes,
            bytes,
            &mut self.usage.saturated,
        );
        self.protocol_error_bytes(Value::Null, -32600, "request exceeds limit")
    }

    async fn call_tool(&mut self, id: Value, params: &Map<String, Value>) -> Value {
        if !only_keys(params, &["_meta", "name", "arguments"]) {
            return self.protocol_error(id, -32602, "invalid tool params");
        }
        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return self.protocol_error(id, -32602, "invalid tool name");
        };
        if !TOOLS.contains(&name) {
            return self.protocol_error(id, -32602, "unknown tool");
        }
        let Some(arguments) = params.get("arguments").and_then(Value::as_object) else {
            return self.protocol_error(id, -32602, "invalid tool arguments");
        };
        bump(&mut self.usage.tool_calls, &mut self.usage.saturated);
        if name == RELEASE_REQUEST_TOOL {
            return self.release_request(id, arguments).await;
        }
        if let Err(refusal) = self.drain_deliveries().await {
            bump(&mut self.usage.refused, &mut self.usage.saturated);
            return success(id, tool_outcome(refusal, true));
        }
        let Some(request) = parse_request(name, arguments, self.limits.semantic) else {
            bump(&mut self.usage.refused, &mut self.usage.saturated);
            return self.protocol_error(id, -32602, "invalid tool arguments");
        };
        let request_ref = request.request;
        if let Some(record) = self.records.get(&request_ref) {
            if record.request != request {
                bump(&mut self.usage.refused, &mut self.usage.saturated);
                return success(
                    id,
                    tool_outcome(
                        json!({"status":"refused","reason":"request_conflict"}),
                        true,
                    ),
                );
            }
            if !record.outcome_unknown {
                bump(&mut self.usage.query_replays, &mut self.usage.saturated);
                let outcome = record
                    .outcome
                    .clone()
                    .unwrap_or_else(|| json!({"status":"pending"}));
                return success(
                    id,
                    tool_outcome(outcome.clone(), outcome_is_error(&outcome)),
                );
            }
        }
        if !self.records.contains_key(&request_ref)
            && self.records.len() == self.limits.max_retained_requests
        {
            bump(&mut self.usage.refused, &mut self.usage.saturated);
            return success(
                id,
                tool_outcome(json!({"status":"refused","reason":"capacity"}), true),
            );
        }

        let replaying_unknown = self.records.contains_key(&request_ref);
        if replaying_unknown {
            bump(&mut self.usage.query_replays, &mut self.usage.saturated);
        }
        let submission = self.client.submit(request.clone()).await;
        bump(&mut self.usage.queries_submitted, &mut self.usage.saturated);
        let (outcome, outcome_unknown) = match submission {
            SemanticSubmitOutcomeV1::Accepted => (None, false),
            SemanticSubmitOutcomeV1::OutcomeUnknown => {
                (Some(json!({"status":"outcome_unknown"})), true)
            }
            SemanticSubmitOutcomeV1::Unsupported(kind) => (
                Some(json!({"status":"unsupported","query":query_kind(kind)})),
                false,
            ),
            SemanticSubmitOutcomeV1::Stale => (Some(json!({"status":"stale"})), false),
            SemanticSubmitOutcomeV1::Unavailable => (Some(json!({"status":"unavailable"})), false),
            SemanticSubmitOutcomeV1::Refused(refusal) => (Some(service_refusal(refusal)), false),
        };
        self.records.insert(
            request_ref,
            QueryRecord {
                request,
                outcome,
                outcome_unknown,
            },
        );
        if let Err(refusal) = self.drain_deliveries().await {
            bump(&mut self.usage.refused, &mut self.usage.saturated);
            return success(id, tool_outcome(refusal, true));
        }
        let outcome = self.records[&request_ref]
            .outcome
            .clone()
            .unwrap_or_else(|| {
                bump(&mut self.usage.pending, &mut self.usage.saturated);
                json!({"status":"pending"})
            });
        let is_error = outcome_is_error(&outcome);
        success(id, tool_outcome(outcome, is_error))
    }

    async fn release_request(&mut self, id: Value, arguments: &Map<String, Value>) -> Value {
        if !only_keys(arguments, &["request"]) {
            bump(&mut self.usage.refused, &mut self.usage.saturated);
            return self.protocol_error(id, -32602, "invalid tool arguments");
        }
        let Some(request) = arguments
            .get("request")
            .and_then(Value::as_str)
            .and_then(parse_hex_ref)
            .map(SemanticRequestRefV1::new)
        else {
            bump(&mut self.usage.refused, &mut self.usage.saturated);
            return self.protocol_error(id, -32602, "invalid tool arguments");
        };
        let releasable = self
            .records
            .get(&request)
            .is_some_and(|record| record.outcome.is_some() && !record.outcome_unknown);
        if !releasable {
            bump(&mut self.usage.refused, &mut self.usage.saturated);
            let reason = if self.records.contains_key(&request) {
                "request_not_terminal"
            } else {
                "unknown_request"
            };
            return success(
                id,
                tool_outcome(json!({"status":"refused","reason":reason}), true),
            );
        }
        let outcome = match self.client.release_request(request).await {
            SemanticReleaseOutcomeV1::Released => {
                self.records.remove(&request);
                bump(&mut self.usage.released, &mut self.usage.saturated);
                json!({"status":"released","request":hex_ref(request.as_bytes())})
            }
            SemanticReleaseOutcomeV1::OutcomeUnknown => {
                json!({"status":"outcome_unknown"})
            }
            SemanticReleaseOutcomeV1::Unavailable => json!({"status":"unavailable"}),
            SemanticReleaseOutcomeV1::Refused(refusal) => service_refusal(refusal),
        };
        let is_error = outcome_is_error(&outcome);
        if is_error {
            bump(&mut self.usage.refused, &mut self.usage.saturated);
        }
        success(id, tool_outcome(outcome, is_error))
    }

    async fn drain_deliveries(&mut self) -> Result<(), Value> {
        loop {
            let delivery = self.client.take_delivery().await.map_err(service_refusal)?;
            let Some(delivery) = delivery else {
                return Ok(());
            };
            let Some(record) = self.records.get_mut(&delivery.request) else {
                bump(
                    &mut self.usage.malformed_deliveries,
                    &mut self.usage.saturated,
                );
                return Err(json!({"status":"refused","reason":"malformed_delivery"}));
            };
            if record.outcome.is_some() {
                bump(
                    &mut self.usage.malformed_deliveries,
                    &mut self.usage.saturated,
                );
                return Err(json!({"status":"refused","reason":"duplicate_delivery"}));
            }
            record.outcome = Some(match delivery.outcome {
                SemanticDeliveryOutcomeV1::Peer(outcome) => {
                    let Some(outcome) =
                        validated_peer_outcome(&record.request, outcome, self.limits.semantic)
                    else {
                        bump(
                            &mut self.usage.malformed_deliveries,
                            &mut self.usage.saturated,
                        );
                        let refusal = json!({"status":"refused","reason":"malformed_delivery"});
                        record.outcome = Some(refusal.clone());
                        return Err(refusal);
                    };
                    outcome
                }
                SemanticDeliveryOutcomeV1::Cancelled => {
                    bump(&mut self.usage.cancelled, &mut self.usage.saturated);
                    json!({"status":"cancelled"})
                }
                SemanticDeliveryOutcomeV1::TimedOut => {
                    bump(&mut self.usage.timed_out, &mut self.usage.saturated);
                    json!({"status":"timed_out"})
                }
            });
            record.outcome_unknown = false;
            bump(&mut self.usage.completed, &mut self.usage.saturated);
        }
    }

    async fn read_resource(&mut self, id: Value, params: &Map<String, Value>) -> Value {
        let Some(uri) = params.get("uri").and_then(Value::as_str) else {
            return self.protocol_error(id, -32602, "unknown resource");
        };
        if uri == DOCUMENTS_RESOURCE {
            let documents: Vec<_> = self
                .documents
                .iter()
                .map(|document| {
                    json!({
                        "native_workspace":document.native_workspace,
                        "native_document":document.native_document,
                        "document":hex_ref(document.document.as_bytes()),
                        "source_revision":document.source_revision.get(),
                        "language":document.language
                    })
                })
                .collect();
            return success(
                id,
                json!({
                    "resultType":"complete",
                    "contents":[{
                        "uri":DOCUMENTS_RESOURCE,
                        "mimeType":"application/json",
                        "text":serde_json::to_string(&json!({"documents":documents}))
                            .expect("JSON value serializes")
                    }],
                    "ttlMs":0,
                    "cacheScope":"private"
                }),
            );
        }
        if uri != CAPABILITIES_RESOURCE {
            return self.protocol_error(id, -32602, "unknown resource");
        }
        let observed = self.client.observe_provider().await;
        let readiness = match observed.readiness() {
            SemanticProviderReadinessV1::Unavailable => json!({"state":"unavailable"}),
            SemanticProviderReadinessV1::Indexing => json!({"state":"indexing"}),
            SemanticProviderReadinessV1::Ready { generation } => {
                json!({"state":"ready","index_generation":generation.get()})
            }
        };
        let capabilities = observed.capabilities();
        success(
            id,
            json!({
                "resultType":"complete",
                "contents":[{
                    "uri":CAPABILITIES_RESOURCE,
                    "mimeType":"application/json",
                    "text":serde_json::to_string(&json!({
                        "readiness":readiness,
                        "read_index":capabilities.contains(SemanticCapabilityV1::ReadIndex),
                        "plan_rename":capabilities.contains(SemanticCapabilityV1::PlanRename)
                    })).expect("JSON value serializes")
                }],
                "ttlMs":0,
                "cacheScope":"private"
            }),
        )
    }

    fn encode(&mut self, value: Value) -> Vec<u8> {
        let id = value.get("id").cloned().unwrap_or(Value::Null);
        let encoded = serde_json::to_vec(&value).expect("JSON value serializes");
        let result = if encoded.len() <= self.limits.max_response_bytes {
            encoded
        } else {
            bump(&mut self.usage.protocol_errors, &mut self.usage.saturated);
            serde_json::to_vec(&error(id, -32603, "response exceeds limit", None))
                .expect("JSON value serializes")
        };
        add(
            &mut self.usage.response_bytes,
            result.len(),
            &mut self.usage.saturated,
        );
        result
    }

    fn protocol_error(&mut self, id: Value, code: i64, message: &'static str) -> Value {
        bump(&mut self.usage.protocol_errors, &mut self.usage.saturated);
        error(id, code, message, None)
    }

    fn protocol_error_bytes(&mut self, id: Value, code: i64, message: &'static str) -> Vec<u8> {
        let value = self.protocol_error(id, code, message);
        self.encode(value)
    }

    fn protocol_error_data_bytes(
        &mut self,
        id: Value,
        code: i64,
        message: &'static str,
        data: Value,
    ) -> Vec<u8> {
        bump(&mut self.usage.protocol_errors, &mut self.usage.saturated);
        self.encode(error(id, code, message, Some(data)))
    }
}

impl<P: SemanticSessionExchangePortV1 + Send> SemanticMcpServerV1<SemanticSessionQueryPortV1<P>> {
    /// Yields synchronization authority only when no query mutation has an
    /// indeterminate transport outcome. Failure returns the complete server.
    pub fn try_suspend_client(self) -> Result<SemanticMcpServerSuspensionV1<P>, Box<Self>> {
        let Self {
            client,
            limits,
            documents,
            records,
            usage,
        } = self;
        match client.into_port().try_suspend() {
            Ok(client) => Ok(SemanticMcpServerSuspensionV1 {
                client,
                limits,
                documents,
                records,
                usage,
            }),
            Err(client) => Err(Box::new(Self {
                client: SemanticClientV1::new(*client),
                limits,
                documents,
                records,
                usage,
            })),
        }
    }
}

impl<P: SemanticSessionExchangePortV1 + Send> SemanticMcpServerSuspensionV1<P> {
    pub fn session_mut(
        &mut self,
    ) -> &mut vellum_semantic_session_client::SemanticSessionClientV1<P> {
        self.client.session_mut()
    }

    /// Reconstitutes the query peer with one exact post-synchronization
    /// catalog. Any refusal retains the entire suspended peer.
    pub fn try_resume(
        mut self,
        mut documents: Vec<SemanticMcpDocumentV1>,
    ) -> Result<
        SemanticMcpServerV1<SemanticSessionQueryPortV1<P>>,
        Box<SemanticMcpServerResumeFailureV1<P>>,
    > {
        if validate_documents(self.limits, &mut documents).is_err() {
            return Err(Box::new(SemanticMcpServerResumeFailureV1 {
                reason: SemanticMcpServerResumeRefusalV1::InvalidDocuments,
                suspension: self,
            }));
        }
        let prior_documents = std::mem::replace(&mut self.documents, documents);
        match self.client.try_resume() {
            Ok(client) => Ok(SemanticMcpServerV1 {
                client: SemanticClientV1::new(client),
                limits: self.limits,
                documents: self.documents,
                records: self.records,
                usage: self.usage,
            }),
            Err(failure) => {
                self.documents = prior_documents;
                let reason = SemanticMcpServerResumeRefusalV1::Query(failure.reason());
                self.client = failure.into_suspension();
                Err(Box::new(SemanticMcpServerResumeFailureV1 {
                    reason,
                    suspension: self,
                }))
            }
        }
    }

    pub fn into_query_suspension(self) -> SemanticSessionQuerySuspensionV1<P> {
        self.client
    }

    /// Separates the complete MCP lifetime from a definitively retired
    /// session and returns the old transport so composition can close it
    /// before attempting a fresh authenticated connection.
    pub fn try_into_reconnect(self) -> Result<(SemanticMcpServerReconnectV1, P), Box<Self>> {
        if self.client.collecting_requests().any(|request| {
            self.records
                .get(&request)
                .is_none_or(|record| record.outcome.is_some() || record.outcome_unknown)
        }) {
            return Err(Box::new(self));
        }
        let Self {
            client,
            limits,
            documents,
            mut records,
            mut usage,
        } = self;
        match client.try_into_reconnect() {
            Ok((mut client, port)) => {
                for request in client.take_retired_collections() {
                    let record = records
                        .get_mut(&request)
                        .expect("preflight retained every collected request");
                    record.outcome = Some(json!({"status":"cancelled"}));
                    record.outcome_unknown = false;
                    bump(&mut usage.cancelled, &mut usage.saturated);
                    bump(&mut usage.completed, &mut usage.saturated);
                }
                Ok((
                    SemanticMcpServerReconnectV1 {
                        client,
                        limits,
                        documents,
                        records,
                        usage,
                    },
                    port,
                ))
            }
            Err(client) => Err(Box::new(Self {
                client: *client,
                limits,
                documents,
                records,
                usage,
            })),
        }
    }
}

impl SemanticMcpServerReconnectV1 {
    pub fn try_bind<P: SemanticSessionExchangePortV1 + Send>(
        self,
        session: SemanticSessionClientV1<P>,
    ) -> Result<SemanticMcpServerSuspensionV1<P>, Box<SemanticMcpServerReconnectFailureV1<P>>> {
        let Self {
            client,
            limits,
            documents,
            records,
            usage,
        } = self;
        match client.try_bind(session) {
            Ok(client) => Ok(SemanticMcpServerSuspensionV1 {
                client,
                limits,
                documents,
                records,
                usage,
            }),
            Err(failure) => {
                let reason = failure.reason();
                let (client, session) = failure.into_parts();
                Err(Box::new(SemanticMcpServerReconnectFailureV1 {
                    reason,
                    reconnect: Self {
                        client,
                        limits,
                        documents,
                        records,
                        usage,
                    },
                    session,
                }))
            }
        }
    }
}

fn parse_request(
    tool: &str,
    arguments: &Map<String, Value>,
    limits: SemanticLimitsV1,
) -> Option<SemanticRequestV1> {
    let common = [
        "request",
        "document",
        "source_revision",
        "index_generation",
        "language",
    ];
    let extra: &[&str] = match tool {
        SYMBOLS_TOOL => &["needle"],
        DEFINITION_TOOL | REFERENCES_TOOL | IMPLEMENTATIONS_TOOL | INCOMING_CALLS_TOOL
        | OUTGOING_CALLS_TOOL | TYPE_AT_POINT_TOOL => &["offset"],
        DIAGNOSTICS_TOOL => &[],
        PLAN_RENAME_TOOL => &["offset", "new_name"],
        _ => return None,
    };
    if !arguments
        .keys()
        .all(|key| common.contains(&key.as_str()) || extra.contains(&key.as_str()))
        || common.iter().any(|key| !arguments.contains_key(*key))
        || extra.iter().any(|key| !arguments.contains_key(*key))
    {
        return None;
    }
    let request = SemanticRequestRefV1::new(parse_hex_ref(arguments.get("request")?.as_str()?)?);
    let document = SemanticDocumentRefV1::new(parse_hex_ref(arguments.get("document")?.as_str()?)?);
    let source_revision =
        SemanticSourceRevisionV1::new(arguments.get("source_revision")?.as_u64()?);
    let expected_index =
        SemanticIndexGenerationV1::new(arguments.get("index_generation")?.as_u64()?);
    let language = SemanticTextV1::try_new(
        arguments.get("language")?.as_str()?.to_owned(),
        limits.max_label_bytes,
    )
    .ok()?;
    let offset = || arguments.get("offset").and_then(Value::as_u64);
    let query = match tool {
        SYMBOLS_TOOL => SemanticQueryV1::Symbols {
            needle: SemanticTextV1::try_new(
                arguments.get("needle")?.as_str()?.to_owned(),
                limits.max_query_bytes,
            )
            .ok()?,
        },
        DEFINITION_TOOL => SemanticQueryV1::Definition { offset: offset()? },
        REFERENCES_TOOL => SemanticQueryV1::References { offset: offset()? },
        IMPLEMENTATIONS_TOOL => SemanticQueryV1::Implementations { offset: offset()? },
        INCOMING_CALLS_TOOL => SemanticQueryV1::IncomingCalls { offset: offset()? },
        OUTGOING_CALLS_TOOL => SemanticQueryV1::OutgoingCalls { offset: offset()? },
        DIAGNOSTICS_TOOL => SemanticQueryV1::Diagnostics,
        TYPE_AT_POINT_TOOL => SemanticQueryV1::TypeAtPoint { offset: offset()? },
        PLAN_RENAME_TOOL => SemanticQueryV1::PlanRename {
            offset: offset()?,
            new_name: SemanticTextV1::try_new(
                arguments.get("new_name")?.as_str()?.to_owned(),
                limits.max_label_bytes,
            )
            .ok()?,
        },
        _ => return None,
    };
    let request = SemanticRequestV1 {
        request,
        target: SemanticTargetV1 {
            document,
            source_revision,
        },
        expected_index,
        language,
        query,
    };
    request.is_within_limits(limits).then_some(request)
}

fn parse_hex_ref(value: &str) -> Option<[u8; 16]> {
    if value.len() != 32
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_hexdigit() || byte.is_ascii_uppercase())
    {
        return None;
    }
    let mut result = [0_u8; 16];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_nibble(pair[0])?;
        let low = hex_nibble(pair[1])?;
        result[index] = (high << 4) | low;
    }
    Some(result)
}

const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn hex_ref(value: [u8; 16]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(32);
    for byte in value {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn validated_peer_outcome(
    request: &SemanticRequestV1,
    outcome: SemanticOutcomeV1,
    limits: SemanticLimitsV1,
) -> Option<Value> {
    match &outcome {
        SemanticOutcomeV1::Complete(reply)
            if reply.request == request.request
                && reply.target == request.target
                && request.expected_index == Some(reply.index_generation)
                && reply.result.matches_query(request.query.kind())
                && reply.result.is_within_limits(limits) => {}
        SemanticOutcomeV1::Unsupported(kind) if *kind == request.query.kind() => {}
        SemanticOutcomeV1::Stale
        | SemanticOutcomeV1::Unavailable
        | SemanticOutcomeV1::Refused(_) => {}
        SemanticOutcomeV1::Complete(_) | SemanticOutcomeV1::Unsupported(_) => return None,
    }
    Some(peer_outcome(outcome))
}

fn peer_outcome(outcome: SemanticOutcomeV1) -> Value {
    match outcome {
        SemanticOutcomeV1::Complete(reply) => complete_reply(reply),
        SemanticOutcomeV1::Unsupported(kind) => {
            json!({"status":"unsupported","query":query_kind(kind)})
        }
        SemanticOutcomeV1::Stale => json!({"status":"stale"}),
        SemanticOutcomeV1::Unavailable => json!({"status":"unavailable"}),
        SemanticOutcomeV1::Refused(refusal) => {
            json!({"status":"refused","reason":peer_refusal(refusal)})
        }
    }
}

fn complete_reply(reply: SemanticReplyV1) -> Value {
    let result = if let Some(values) = reply.result.symbols() {
        json!({"kind":"symbols","items":values.iter().map(|value| json!({
            "name":value.name.as_str(), "kind":symbol_kind(value.kind), "location":location(value.location)
        })).collect::<Vec<_>>()})
    } else if let Some(values) = reply.result.locations() {
        json!({"kind":"locations","items":values.iter().copied().map(location).collect::<Vec<_>>()})
    } else if let Some(values) = reply.result.diagnostics() {
        json!({"kind":"diagnostics","items":values.iter().map(|value| json!({
            "severity":severity(value.severity), "location":location(value.location),
            "message":value.message.as_str(), "code":value.code.as_ref().map(SemanticTextV1::as_str)
        })).collect::<Vec<_>>()})
    } else if let Some(value) = reply.result.type_at_point() {
        json!({"kind":"type_at_point","value":value.as_str()})
    } else if let Some(values) = reply.result.rename_plan() {
        json!({"kind":"rename_plan","edits":values.iter().map(|value| json!({
            "location":location(value.location), "replacement":value.replacement.as_str()
        })).collect::<Vec<_>>()})
    } else {
        json!({"kind":"invalid"})
    };
    json!({
        "status":"complete",
        "request":hex_ref(reply.request.as_bytes()),
        "document":hex_ref(reply.target.document.as_bytes()),
        "source_revision":reply.target.source_revision.get(),
        "index_generation":reply.index_generation.get(),
        "provider":{
            "id":hex_ref(reply.provider.provider().as_bytes()),
            "profile":reply.provider.profile(),
            "version":reply.provider.version()
        },
        "result":result
    })
}

fn location(value: SemanticLocationV1) -> Value {
    json!({
        "document":hex_ref(value.target.document.as_bytes()),
        "source_revision":value.target.source_revision.get(),
        "start":value.span.start,
        "end":value.span.end
    })
}

fn service_refusal(refusal: SemanticServiceRefusalV1) -> Value {
    let reason = match refusal {
        SemanticServiceRefusalV1::InvalidLimits => "invalid_limits",
        SemanticServiceRefusalV1::AuthorityDenied => "authority_denied",
        SemanticServiceRefusalV1::SynchronizationRequired => "synchronization_required",
        SemanticServiceRefusalV1::UnknownClient => "unknown_client",
        SemanticServiceRefusalV1::ClientRetiring => "client_retiring",
        SemanticServiceRefusalV1::ClientIsolation => "client_isolation",
        SemanticServiceRefusalV1::UnknownDocument => "unknown_document",
        SemanticServiceRefusalV1::DocumentAlreadyOpen => "document_already_open",
        SemanticServiceRefusalV1::OutOfOrderRevision => "out_of_order_revision",
        SemanticServiceRefusalV1::SourceTargetMismatch => "source_target_mismatch",
        SemanticServiceRefusalV1::InputLimit => "input_limit",
        SemanticServiceRefusalV1::Capacity => "capacity",
        SemanticServiceRefusalV1::DuplicateRequest => "duplicate_request",
        SemanticServiceRefusalV1::ProviderRefused => "provider_refused",
        SemanticServiceRefusalV1::MalformedProviderReply => "malformed_provider_reply",
    };
    json!({"status":"refused","reason":reason})
}

const fn peer_refusal(refusal: SemanticRefusalV1) -> &'static str {
    match refusal {
        SemanticRefusalV1::AuthorityDenied => "authority_denied",
        SemanticRefusalV1::InputLimit => "input_limit",
        SemanticRefusalV1::OutputLimit => "output_limit",
        SemanticRefusalV1::InvalidRequest => "invalid_request",
        SemanticRefusalV1::MalformedProviderReply => "malformed_provider_reply",
    }
}

const fn query_kind(kind: SemanticQueryKindV1) -> &'static str {
    match kind {
        SemanticQueryKindV1::Symbols => "symbols",
        SemanticQueryKindV1::Definition => "definition",
        SemanticQueryKindV1::References => "references",
        SemanticQueryKindV1::Implementations => "implementations",
        SemanticQueryKindV1::IncomingCalls => "incoming_calls",
        SemanticQueryKindV1::OutgoingCalls => "outgoing_calls",
        SemanticQueryKindV1::Diagnostics => "diagnostics",
        SemanticQueryKindV1::TypeAtPoint => "type_at_point",
        SemanticQueryKindV1::PlanRename => "plan_rename",
    }
}

const fn symbol_kind(kind: SemanticSymbolKindV1) -> &'static str {
    match kind {
        SemanticSymbolKindV1::Module => "module",
        SemanticSymbolKindV1::Type => "type",
        SemanticSymbolKindV1::Function => "function",
        SemanticSymbolKindV1::Method => "method",
        SemanticSymbolKindV1::Value => "value",
        SemanticSymbolKindV1::Field => "field",
        SemanticSymbolKindV1::Unknown => "unknown",
    }
}

const fn severity(value: SemanticDiagnosticSeverityV1) -> &'static str {
    match value {
        SemanticDiagnosticSeverityV1::Error => "error",
        SemanticDiagnosticSeverityV1::Warning => "warning",
        SemanticDiagnosticSeverityV1::Information => "information",
        SemanticDiagnosticSeverityV1::Hint => "hint",
    }
}

fn discover_result() -> Value {
    json!({
        "resultType":"complete",
        "supportedVersions":[PROTOCOL_VERSION],
        "capabilities":{"tools":{"listChanged":false},"resources":{"listChanged":false,"subscribe":false}},
        "instructions":"Use exact request, document, source revision and index generation. Pending work is retrieved only by replaying identical arguments. Rename returns a plan and never applies it. Complete results disclose the provider fingerprint for semantic correlation.",
        "ttlMs":0,
        "cacheScope":"private"
    })
}

fn tools_result() -> Value {
    success_tools(TOOLS.iter().map(|name| tool_schema(name)).collect())
}

fn success_tools(tools: Vec<Value>) -> Value {
    json!({
        "resultType":"complete","tools":tools,
        "ttlMs":0,"cacheScope":"private"
    })
}

fn tool_schema(name: &str) -> Value {
    if name == RELEASE_REQUEST_TOOL {
        return json!({
            "name":name,
            "title":"Release semantic result",
            "description":"Explicitly retire one terminal retained query result after the caller no longer needs replay.",
            "inputSchema":{
                "type":"object",
                "properties":{"request":{"type":"string","pattern":"^[0-9a-f]{32}$"}},
                "required":["request"],
                "additionalProperties":false
            },
            "outputSchema":{"type":"object","properties":{"status":{"type":"string"}},"required":["status"]},
            "annotations":{"readOnlyHint":false,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}
        });
    }
    let (title, description, extras) = match name {
        SYMBOLS_TOOL => (
            "Search document symbols",
            "Search symbols in one exact document snapshot.",
            json!({"needle":{"type":"string","minLength":1,"maxLength":MAX_SEMANTIC_QUERY_BYTES_V1}}),
        ),
        DEFINITION_TOOL => (
            "Find definition",
            "Find definitions at an exact byte offset.",
            offset_schema(),
        ),
        REFERENCES_TOOL => (
            "Find references",
            "Find references at an exact byte offset.",
            offset_schema(),
        ),
        IMPLEMENTATIONS_TOOL => (
            "Find implementations",
            "Find implementations at an exact byte offset.",
            offset_schema(),
        ),
        INCOMING_CALLS_TOOL => (
            "Find incoming calls",
            "Find callers of the exact symbol prepared at a byte offset.",
            offset_schema(),
        ),
        OUTGOING_CALLS_TOOL => (
            "Find outgoing calls",
            "Find callees of the exact symbol prepared at a byte offset.",
            offset_schema(),
        ),
        DIAGNOSTICS_TOOL => (
            "Read diagnostics",
            "Read diagnostics for one exact document snapshot.",
            json!({}),
        ),
        TYPE_AT_POINT_TOOL => (
            "Read type",
            "Read type information at an exact byte offset.",
            offset_schema(),
        ),
        PLAN_RENAME_TOOL => (
            "Plan rename",
            "Prepare a review-only rename plan; this tool never applies edits.",
            json!({"offset":{"type":"integer","minimum":0},"new_name":{"type":"string","minLength":1,"maxLength":MAX_SEMANTIC_LABEL_BYTES_V1}}),
        ),
        _ => unreachable!("closed tool catalog"),
    };
    let mut properties = json!({
        "request":{"type":"string","pattern":"^[0-9a-f]{32}$"},
        "document":{"type":"string","pattern":"^[0-9a-f]{32}$"},
        "source_revision":{"type":"integer","minimum":0},
        "index_generation":{"type":"integer","minimum":1},
        "language":{"type":"string","minLength":1,"maxLength":MAX_SEMANTIC_LABEL_BYTES_V1}
    });
    properties
        .as_object_mut()
        .expect("object")
        .extend(extras.as_object().expect("object").clone());
    let mut required = vec![
        "request",
        "document",
        "source_revision",
        "index_generation",
        "language",
    ];
    if name == SYMBOLS_TOOL {
        required.push("needle");
    }
    if matches!(
        name,
        DEFINITION_TOOL
            | REFERENCES_TOOL
            | IMPLEMENTATIONS_TOOL
            | INCOMING_CALLS_TOOL
            | OUTGOING_CALLS_TOOL
            | TYPE_AT_POINT_TOOL
            | PLAN_RENAME_TOOL
    ) {
        required.push("offset");
    }
    if name == PLAN_RENAME_TOOL {
        required.push("new_name");
    }
    json!({
        "name":name,
        "title":title,
        "description":description,
        "inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},
        "outputSchema":{"type":"object","properties":{"status":{"type":"string"}},"required":["status"]},
        "annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}
    })
}

fn offset_schema() -> Value {
    json!({"offset":{"type":"integer","minimum":0}})
}

fn resources_result() -> Value {
    json!({"resultType":"complete","resources":[
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
    ],"ttlMs":0,"cacheScope":"private"})
}

fn tool_outcome(outcome: Value, is_error: bool) -> Value {
    let summary = outcome
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("refused");
    json!({
        "resultType":"complete",
        "content":[{"type":"text","text":format!("semantic query {summary}")}],
        "structuredContent":outcome,
        "isError":is_error
    })
}

fn outcome_is_error(outcome: &Value) -> bool {
    !matches!(
        outcome.get("status").and_then(Value::as_str),
        Some("complete" | "pending" | "released")
    )
}

fn validate_meta(value: Option<&Value>) -> Result<(), Option<String>> {
    let Some(meta_value) = value else {
        return Err(None);
    };
    let Some(meta) = meta_value.as_object() else {
        return Err(None);
    };
    if !bounded_metadata_value(meta_value, MAX_METADATA_BYTES)
        || meta
            .keys()
            .any(|key| !valid_metadata_key(key) || key.len() > MAX_METADATA_KEY_BYTES)
    {
        return Err(None);
    }
    let known = [
        "io.modelcontextprotocol/protocolVersion",
        "io.modelcontextprotocol/clientCapabilities",
        "io.modelcontextprotocol/clientInfo",
        "io.modelcontextprotocol/logLevel",
    ];
    if meta.iter().any(|(key, value)| {
        !known.contains(&key.as_str())
            && (reserved_prefix(key)
                || !bounded_metadata_value(value, MAX_METADATA_VALUE_BYTES)
                || !valid_standard_metadata_value(key, value))
    }) {
        return Err(None);
    }
    let Some(version) = meta
        .get("io.modelcontextprotocol/protocolVersion")
        .and_then(Value::as_str)
    else {
        return Err(None);
    };
    if version != PROTOCOL_VERSION {
        return Err(Some(bounded_copy(version)));
    }
    if !meta
        .get("io.modelcontextprotocol/clientCapabilities")
        .is_some_and(valid_client_capabilities)
    {
        return Err(None);
    }
    if let Some(info) = meta.get("io.modelcontextprotocol/clientInfo")
        && !valid_implementation(info)
    {
        return Err(None);
    }
    if let Some(level) = meta.get("io.modelcontextprotocol/logLevel")
        && !level.as_str().is_some_and(|level| {
            matches!(
                level,
                "debug"
                    | "info"
                    | "notice"
                    | "warning"
                    | "error"
                    | "critical"
                    | "alert"
                    | "emergency"
            )
        })
    {
        return Err(None);
    }
    Ok(())
}

fn valid_client_capabilities(value: &Value) -> bool {
    value.as_object().is_some_and(|capabilities| {
        bounded_metadata_value(value, MAX_CLIENT_CAPABILITIES_BYTES)
            && capabilities.get("extensions").is_none_or(|extensions| {
                extensions.as_object().is_some_and(|extensions| {
                    extensions.iter().all(|(identifier, declaration)| {
                        valid_extension_identifier(identifier)
                            && identifier.len() <= MAX_METADATA_KEY_BYTES
                            && declaration.is_object()
                            && bounded_metadata_value(declaration, MAX_METADATA_VALUE_BYTES)
                    })
                })
            })
    })
}

fn valid_extension_identifier(identifier: &str) -> bool {
    identifier
        .split_once('/')
        .is_some_and(|(prefix, _)| !prefix.is_empty())
        && valid_metadata_key(identifier)
}

fn valid_standard_metadata_value(key: &str, value: &Value) -> bool {
    match key {
        "progressToken" => matches!(value, Value::String(_) | Value::Number(_)),
        "traceparent" | "tracestate" | "baggage" => value.is_string(),
        _ => true,
    }
}

fn valid_metadata_key(key: &str) -> bool {
    let (prefix, name) = match key.split_once('/') {
        Some((prefix, name)) if !prefix.is_empty() && !name.contains('/') => (Some(prefix), name),
        Some(_) => return false,
        None => (None, key),
    };
    if let Some(prefix) = prefix
        && !prefix.split('.').all(valid_prefix_label)
    {
        return false;
    }
    valid_metadata_name(name)
}

fn valid_prefix_label(label: &str) -> bool {
    let bytes = label.as_bytes();
    bytes.first().is_some_and(u8::is_ascii_alphabetic)
        && bytes
            .last()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
}

fn valid_metadata_name(name: &str) -> bool {
    if name.is_empty() {
        return true;
    }
    let bytes = name.as_bytes();
    bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'_' | b'.'))
}

fn reserved_prefix(key: &str) -> bool {
    let Some((prefix, _)) = key.split_once('/') else {
        return false;
    };
    prefix
        .split('.')
        .nth(1)
        .is_some_and(|label| matches!(label, "modelcontextprotocol" | "mcp"))
}

fn valid_implementation(value: &Value) -> bool {
    value.as_object().is_some_and(|object| {
        bounded_json(value, MAX_CLIENT_INFO_BYTES)
            && only_keys(
                object,
                &[
                    "name",
                    "version",
                    "title",
                    "description",
                    "websiteUrl",
                    "icons",
                ],
            )
            && ["name", "version"].iter().all(|key| {
                object
                    .get(*key)
                    .and_then(Value::as_str)
                    .is_some_and(|value| {
                        !value.is_empty() && value.len() <= MAX_IMPLEMENTATION_FIELD_BYTES
                    })
            })
            && ["title", "description", "websiteUrl"].iter().all(|key| {
                object.get(*key).is_none_or(|value| {
                    value
                        .as_str()
                        .is_some_and(|value| !value.is_empty() && value.len() <= 2_048)
                })
            })
            && object.get("icons").is_none_or(valid_icons)
    })
}

fn valid_icons(value: &Value) -> bool {
    value.as_array().is_some_and(|icons| {
        icons.len() <= 8
            && icons.iter().all(|icon| {
                icon.as_object().is_some_and(|icon| {
                    only_keys(icon, &["src", "mimeType", "sizes", "theme"])
                        && icon
                            .get("src")
                            .and_then(Value::as_str)
                            .is_some_and(|src| !src.is_empty() && src.len() <= 2_048)
                        && icon.get("mimeType").is_none_or(|value| {
                            value
                                .as_str()
                                .is_some_and(|value| !value.is_empty() && value.len() <= 128)
                        })
                        && icon.get("theme").is_none_or(|value| {
                            value
                                .as_str()
                                .is_some_and(|value| matches!(value, "light" | "dark"))
                        })
                        && icon.get("sizes").is_none_or(|value| {
                            value.as_array().is_some_and(|sizes| {
                                sizes.len() <= 8
                                    && sizes.iter().all(|size| {
                                        size.as_str().is_some_and(|size| {
                                            !size.is_empty() && size.len() <= 32
                                        })
                                    })
                            })
                        })
                })
            })
    })
}

fn bounded_json(value: &Value, max_bytes: usize) -> bool {
    serde_json::to_vec(value).is_ok_and(|encoded| encoded.len() <= max_bytes)
}

fn bounded_metadata_value(value: &Value, max_bytes: usize) -> bool {
    if !bounded_json(value, max_bytes) {
        return false;
    }
    let mut pending = vec![(value, 1_usize)];
    while let Some((value, depth)) = pending.pop() {
        if depth > MAX_METADATA_DEPTH {
            return false;
        }
        match value {
            Value::Array(values) => pending.extend(values.iter().map(|value| (value, depth + 1))),
            Value::Object(values) => {
                if values.keys().any(|key| key.len() > MAX_METADATA_KEY_BYTES) {
                    return false;
                }
                pending.extend(values.values().map(|value| (value, depth + 1)));
            }
            _ => {}
        }
    }
    true
}

fn list_params(params: &Map<String, Value>) -> Result<(), ()> {
    if !only_keys(params, &["_meta", "cursor"]) {
        return Err(());
    }
    if let Some(cursor) = params.get("cursor")
        && !cursor
            .as_str()
            .is_some_and(|cursor| !cursor.is_empty() && cursor.len() <= MAX_CURSOR_BYTES)
    {
        return Err(());
    }
    Ok(())
}

fn valid_id(value: &Value) -> bool {
    serde_json::to_vec(value).is_ok_and(|encoded| encoded.len() <= MAX_JSON_RPC_ID_BYTES)
}

fn bounded_copy(value: &str) -> String {
    let end = value
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index <= MAX_IMPLEMENTATION_FIELD_BYTES)
        .last()
        .unwrap_or(0);
    if value.len() <= MAX_IMPLEMENTATION_FIELD_BYTES {
        value.to_owned()
    } else {
        value[..end].to_owned()
    }
}

fn unsupported_version_data(requested: Option<&str>) -> Value {
    let mut data = json!({"supported":[PROTOCOL_VERSION]});
    if let Some(requested) = requested {
        data["requested"] = json!(bounded_copy(requested));
    }
    data
}

fn only_keys(object: &Map<String, Value>, allowed: &[&str]) -> bool {
    object.keys().all(|key| allowed.contains(&key.as_str()))
}

fn success(id: Value, result: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"result":with_server_meta(result)})
}
fn with_server_meta(mut result: Value) -> Value {
    let object = result.as_object_mut().expect("success result is an object");
    object
        .entry("resultType")
        .or_insert_with(|| Value::String("complete".into()));
    object.insert(
        "_meta".into(),
        json!({"io.modelcontextprotocol/serverInfo":{
            "name":"vellum-semantic-mcp",
            "version":env!("CARGO_PKG_VERSION"),
            "description":"Bounded MCP projection of Vellum semantic queries"
        }}),
    );
    result
}
fn error(id: Value, code: i64, message: &'static str, data: Option<Value>) -> Value {
    let mut error = json!({"code":code,"message":message});
    if let Some(data) = data {
        error["data"] = data;
    }
    json!({"jsonrpc":"2.0","id":id,"error":error})
}

fn bump(counter: &mut u64, saturated: &mut bool) {
    let next = counter.saturating_add(1);
    *saturated |= next == u64::MAX;
    *counter = next;
}

fn add(counter: &mut u64, amount: usize, saturated: &mut bool) {
    let amount = u64::try_from(amount).unwrap_or(u64::MAX);
    let next = counter.saturating_add(amount);
    *saturated |= next == u64::MAX;
    *counter = next;
}

#[cfg(test)]
mod tests;
