//! Bounded MCP projection of the native Vellum client contract.
//!
//! This adapter implements only MCP `2026-07-28` over newline-delimited JSON-RPC.
//! MCP remains a projection: native commands, witnesses, refusals, and transport
//! failures retain their meaning at the `vellum-client` boundary.
#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};
use std::io::{self, BufRead};
use vellum_client::Client;
use vellum_protocol::{OutcomeV1, RefusalV1, ReplyV1, ServiceAccessErrorV1, ServicePort};

pub const PROTOCOL_VERSION: &str = "2026-07-28";
pub const SNAPSHOT_TOOL: &str = "vellum.snapshot";
pub const REPLACE_TOOL: &str = "vellum.replace";
const TOOL_ERROR_CODES: &[&str] = &[
    "unknown_client",
    "client_capacity",
    "client_lifetime_capacity",
    "request_capacity",
    "request_conflict",
    "witness_capacity",
    "unknown_witness",
    "stale",
    "invalid_range",
    "input_limit",
    "snapshot_limit",
    "editor_closed",
    "editor_capacity",
    "editor_conflict",
    "editor_incomplete",
    "editor_unavailable",
    "counter_exhausted",
    "service_unavailable",
    "service_malformed",
    "service_limit",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct McpLimits {
    pub max_request_bytes: usize,
    pub max_response_bytes: usize,
}

impl McpLimits {
    pub const fn reference() -> Self {
        Self {
            max_request_bytes: 262_144,
            max_response_bytes: 524_288,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpBuildError {
    InvalidLimits,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct McpUsage {
    pub requests: u64,
    pub request_bytes: u64,
    pub response_bytes: u64,
    pub discoveries: u64,
    pub tool_lists: u64,
    pub tool_calls: u64,
    pub tool_call_micros_total: u64,
    pub tool_call_micros_max: u64,
    pub protocol_errors: u64,
    pub tool_errors: u64,
    pub counter_saturated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundedLine {
    Eof,
    Line(Vec<u8>),
    Limit(usize),
    Truncated(usize),
}

pub struct McpServer<T> {
    client: Client<T>,
    limits: McpLimits,
    usage: McpUsage,
}

impl<T: ServicePort> McpServer<T> {
    pub fn new(client: Client<T>, limits: McpLimits) -> Result<Self, McpBuildError> {
        if limits.max_request_bytes == 0 || limits.max_response_bytes < 256 {
            return Err(McpBuildError::InvalidLimits);
        }
        Ok(Self {
            client,
            limits,
            usage: McpUsage::default(),
        })
    }

    pub fn usage(&self) -> McpUsage {
        self.usage
    }

    pub fn into_client(self) -> Client<T> {
        self.client
    }

    /// Handles one newline-free MCP message. Notifications intentionally yield
    /// no response; requests always yield exactly one bounded JSON-RPC response.
    pub fn handle(&mut self, line: &[u8]) -> Option<Vec<u8>> {
        bump(&mut self.usage.requests, &mut self.usage.counter_saturated);
        add(
            &mut self.usage.request_bytes,
            line.len(),
            &mut self.usage.counter_saturated,
        );
        if line.len() > self.limits.max_request_bytes {
            return Some(self.protocol_error(Value::Null, -32600, "request exceeds limit", None));
        }
        let value: Value = match serde_json::from_slice(line) {
            Ok(value) => value,
            Err(_) => return Some(self.protocol_error(Value::Null, -32700, "parse error", None)),
        };
        let Some(object) = value.as_object() else {
            return Some(self.protocol_error(Value::Null, -32600, "invalid request", None));
        };
        let id = match object.get("id") {
            Some(id @ (Value::String(_) | Value::Number(_))) => Some(id.clone()),
            Some(_) => {
                return Some(self.protocol_error(Value::Null, -32600, "invalid request id", None));
            }
            None => None,
        };
        if object.get("jsonrpc") != Some(&Value::String("2.0".into())) {
            return id.map(|id| self.protocol_error(id, -32600, "invalid request", None));
        }
        let Some(method) = object.get("method").and_then(Value::as_str) else {
            return id.map(|id| self.protocol_error(id, -32600, "invalid request", None));
        };
        if method == "initialize" {
            let id = id?;
            return Some(self.protocol_error(
                id,
                -32022,
                "unsupported protocol version",
                Some(json!({"supported": [PROTOCOL_VERSION]})),
            ));
        }
        // Modern stdio cancellation is advisory. Native Vellum operations are
        // synchronous and bounded, so there is no detached worker to cancel.
        let id = id?;
        let Some(params) = object.get("params").and_then(Value::as_object) else {
            return Some(self.protocol_error(id, -32602, "invalid params", None));
        };
        if let Err(requested) = validate_meta(params) {
            let data = requested
                .map(|requested| json!({"supported": [PROTOCOL_VERSION], "requested": requested}));
            let (code, message) = if data.is_some() {
                (-32022, "unsupported protocol version")
            } else {
                (-32602, "invalid request metadata")
            };
            return Some(self.protocol_error(id, code, message, data));
        }

        let response = match method {
            "server/discover" => {
                if !only_keys(params, &["_meta"]) {
                    return Some(self.protocol_error(id, -32602, "invalid discover params", None));
                }
                bump(
                    &mut self.usage.discoveries,
                    &mut self.usage.counter_saturated,
                );
                success(id, discover_result())
            }
            "tools/list" => {
                if !only_keys(params, &["_meta"]) {
                    self.protocol_error_value(id, -32602, "cursor is not supported", None)
                } else {
                    bump(
                        &mut self.usage.tool_lists,
                        &mut self.usage.counter_saturated,
                    );
                    success(id, tools_result())
                }
            }
            "tools/call" => {
                if !params
                    .keys()
                    .all(|key| matches!(key.as_str(), "_meta" | "name" | "arguments"))
                {
                    return Some(self.protocol_error(id, -32602, "invalid tool params", None));
                }
                bump(
                    &mut self.usage.tool_calls,
                    &mut self.usage.counter_saturated,
                );
                let started = std::time::Instant::now();
                let response = self.call_tool(id, params);
                record_duration(&mut self.usage, started.elapsed());
                response
            }
            _ => self.protocol_error_value(id, -32601, "method not found", None),
        };
        Some(self.encode_bounded(response))
    }

    pub fn limit_response(&mut self, observed_bytes: usize) -> Vec<u8> {
        self.record_framed_request(observed_bytes);
        self.protocol_error(Value::Null, -32600, "request exceeds limit", None)
    }

    pub fn truncated_response(&mut self, observed_bytes: usize) -> Vec<u8> {
        self.record_framed_request(observed_bytes);
        self.protocol_error(Value::Null, -32600, "truncated request", None)
    }

    fn record_framed_request(&mut self, observed_bytes: usize) {
        bump(&mut self.usage.requests, &mut self.usage.counter_saturated);
        add(
            &mut self.usage.request_bytes,
            observed_bytes,
            &mut self.usage.counter_saturated,
        );
    }

    fn call_tool(&mut self, id: Value, params: &Map<String, Value>) -> Value {
        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return self.protocol_error_value(id, -32602, "invalid tool name", None);
        };
        let result = match name {
            SNAPSHOT_TOOL
                if params
                    .get("arguments")
                    .is_none_or(|value| value.as_object().is_some_and(Map::is_empty)) =>
            {
                self.client.snapshot()
            }
            SNAPSHOT_TOOL => {
                return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
            }
            REPLACE_TOOL => {
                let Some(arguments) = params.get("arguments").and_then(Value::as_object) else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                let Some((expected_revision, start, end, inserted)) = replace_arguments(arguments)
                else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                self.replace_at_revision(expected_revision, start, end, inserted)
            }
            _ => return self.protocol_error_value(id, -32602, "unknown tool", None),
        };
        match result {
            Ok(ReplyV1::Accepted(outcome)) => success(id, tool_success(outcome)),
            Ok(ReplyV1::Refused(refusal)) => {
                bump(
                    &mut self.usage.tool_errors,
                    &mut self.usage.counter_saturated,
                );
                success(id, tool_error(refusal_code(refusal)))
            }
            Err(error) => {
                bump(
                    &mut self.usage.tool_errors,
                    &mut self.usage.counter_saturated,
                );
                success(id, tool_error(access_code(error)))
            }
        }
    }

    fn replace_at_revision(
        &mut self,
        expected_revision: u64,
        start: u64,
        end: u64,
        inserted: String,
    ) -> Result<ReplyV1, ServiceAccessErrorV1> {
        match self.client.snapshot()? {
            ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot))
                if snapshot.revision == expected_revision =>
            {
                self.client.replace(snapshot.witness, start, end, inserted)
            }
            ReplyV1::Accepted(OutcomeV1::Snapshot(_)) => Ok(ReplyV1::Refused(RefusalV1::Stale)),
            ReplyV1::Accepted(OutcomeV1::Replaced(_)) => Err(ServiceAccessErrorV1::Malformed),
            ReplyV1::Refused(refusal) => Ok(ReplyV1::Refused(refusal)),
        }
    }

    fn protocol_error(
        &mut self,
        id: Value,
        code: i64,
        message: &'static str,
        data: Option<Value>,
    ) -> Vec<u8> {
        let value = self.protocol_error_value(id, code, message, data);
        self.encode_bounded(value)
    }

    fn protocol_error_value(
        &mut self,
        id: Value,
        code: i64,
        message: &'static str,
        data: Option<Value>,
    ) -> Value {
        bump(
            &mut self.usage.protocol_errors,
            &mut self.usage.counter_saturated,
        );
        error_value(id, code, message, data)
    }

    fn encode_bounded(&mut self, value: Value) -> Vec<u8> {
        let response_id = value.get("id").cloned().unwrap_or(Value::Null);
        let encoded = serde_json::to_vec(&value).expect("JSON values always serialize");
        if encoded.len() <= self.limits.max_response_bytes {
            add(
                &mut self.usage.response_bytes,
                encoded.len(),
                &mut self.usage.counter_saturated,
            );
            encoded
        } else {
            bump(
                &mut self.usage.protocol_errors,
                &mut self.usage.counter_saturated,
            );
            let fallback = internal_error(response_id);
            add(
                &mut self.usage.response_bytes,
                fallback.len(),
                &mut self.usage.counter_saturated,
            );
            fallback
        }
    }
}

pub fn read_bounded_line<R: BufRead>(reader: &mut R, max_bytes: usize) -> io::Result<BoundedLine> {
    let mut line = Vec::new();
    let mut limited = false;
    let mut observed = 0_usize;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return if observed == 0 && !limited {
                Ok(BoundedLine::Eof)
            } else if limited {
                Ok(BoundedLine::Limit(observed))
            } else {
                Ok(BoundedLine::Truncated(observed))
            };
        }
        let end = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |position| position + 1);
        let payload_end = if available.get(end.saturating_sub(1)) == Some(&b'\n') {
            end - 1
        } else {
            end
        };
        observed = observed.saturating_add(payload_end);
        if !limited {
            if line.len().saturating_add(payload_end) > max_bytes {
                limited = true;
                line.clear();
            } else {
                line.extend_from_slice(&available[..payload_end]);
            }
        }
        let found_newline = available.get(end.saturating_sub(1)) == Some(&b'\n');
        reader.consume(end);
        if found_newline {
            return if limited {
                Ok(BoundedLine::Limit(observed))
            } else {
                Ok(BoundedLine::Line(line))
            };
        }
    }
}

fn validate_meta(params: &Map<String, Value>) -> Result<(), Option<String>> {
    let Some(meta) = params.get("_meta").and_then(Value::as_object) else {
        return Err(None);
    };
    let Some(version) = meta
        .get("io.modelcontextprotocol/protocolVersion")
        .and_then(Value::as_str)
    else {
        return Err(None);
    };
    if version != PROTOCOL_VERSION {
        return Err(Some(version.to_owned()));
    }
    if !meta
        .get("io.modelcontextprotocol/clientCapabilities")
        .is_some_and(Value::is_object)
    {
        return Err(None);
    }
    if let Some(info) = meta.get("io.modelcontextprotocol/clientInfo")
        && !valid_implementation(info)
    {
        return Err(None);
    }
    Ok(())
}

fn valid_implementation(value: &Value) -> bool {
    value.as_object().is_some_and(|object| {
        object
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|v| !v.is_empty())
            && object
                .get("version")
                .and_then(Value::as_str)
                .is_some_and(|v| !v.is_empty())
    })
}

fn discover_result() -> Value {
    with_server_meta(json!({
        "resultType": "complete",
        "supportedVersions": [PROTOCOL_VERSION],
        "capabilities": {"tools": {"listChanged": false}},
        "instructions": "Read a snapshot before replacing text. Pass its revision unchanged; stale edits are refused.",
        "ttlMs": 0,
        "cacheScope": "private"
    }))
}

fn tools_result() -> Value {
    with_server_meta(json!({
        "resultType": "complete",
        "tools": [
            {
                "name": SNAPSHOT_TOOL,
                "title": "Read Vellum snapshot",
                "description": "Return the active document text and workspace revision.",
                "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false},
                "outputSchema": tool_output_schema("snapshot"),
                "annotations": {"readOnlyHint": true, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false}
            },
            {
                "name": REPLACE_TOOL,
                "title": "Replace Vellum text range",
                "description": "Replace a byte range only when the supplied workspace revision is still current.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "expected_revision": {"type": "integer", "minimum": 0},
                        "start": {"type": "integer", "minimum": 0},
                        "end": {"type": "integer", "minimum": 0},
                        "inserted": {"type": "string"}
                    },
                    "required": ["expected_revision", "start", "end", "inserted"],
                    "additionalProperties": false
                },
                "outputSchema": tool_output_schema("replaced"),
                "annotations": {"readOnlyHint": false, "destructiveHint": true, "idempotentHint": false, "openWorldHint": false}
            }
        ],
        "ttlMs": 0,
        "cacheScope": "private"
    }))
}

fn snapshot_schema(kind: &'static str) -> Value {
    json!({
        "type": "object",
        "properties": {
            "kind": {"type": "string", "enum": [kind]},
            "revision": {"type": "integer", "minimum": 0},
            "text": {"type": "string"}
        },
        "required": ["kind", "revision", "text"],
        "additionalProperties": false
    })
}

fn tool_output_schema(kind: &'static str) -> Value {
    json!({
        "oneOf": [
            snapshot_schema(kind),
            {
                "type": "object",
                "properties": {"code": {"type": "string", "enum": TOOL_ERROR_CODES}},
                "required": ["code"],
                "additionalProperties": false
            }
        ]
    })
}

fn replace_arguments(arguments: &Map<String, Value>) -> Option<(u64, u64, u64, String)> {
    if !only_keys(
        arguments,
        &["expected_revision", "start", "end", "inserted"],
    ) {
        return None;
    }
    let expected = arguments.get("expected_revision")?.as_u64()?;
    let start = arguments.get("start")?.as_u64()?;
    let end = arguments.get("end")?.as_u64()?;
    let inserted = arguments.get("inserted")?.as_str()?.to_owned();
    Some((expected, start, end, inserted))
}

fn only_keys(arguments: &Map<String, Value>, allowed: &[&str]) -> bool {
    arguments.len() == allowed.len() && arguments.keys().all(|key| allowed.contains(&key.as_str()))
}

fn tool_success(outcome: OutcomeV1) -> Value {
    let (kind, snapshot) = match outcome {
        OutcomeV1::Snapshot(snapshot) => ("snapshot", snapshot),
        OutcomeV1::Replaced(snapshot) => ("replaced", snapshot),
    };
    let structured = json!({
        "kind": kind,
        "revision": snapshot.revision,
        "text": &*snapshot.text
    });
    with_server_meta(json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": kind}],
        "structuredContent": structured,
        "isError": false
    }))
}

fn tool_error(code: &'static str) -> Value {
    let structured = json!({"code": code});
    with_server_meta(json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": code}],
        "structuredContent": structured,
        "isError": true
    }))
}

fn refusal_code(refusal: RefusalV1) -> &'static str {
    match refusal {
        RefusalV1::UnknownClient => "unknown_client",
        RefusalV1::ClientCapacity => "client_capacity",
        RefusalV1::ClientLifetimeCapacity => "client_lifetime_capacity",
        RefusalV1::RequestCapacity => "request_capacity",
        RefusalV1::RequestConflict => "request_conflict",
        RefusalV1::WitnessCapacity => "witness_capacity",
        RefusalV1::UnknownWitness => "unknown_witness",
        RefusalV1::Stale => "stale",
        RefusalV1::InvalidRange => "invalid_range",
        RefusalV1::InputLimit => "input_limit",
        RefusalV1::SnapshotLimit => "snapshot_limit",
        RefusalV1::EditorClosed => "editor_closed",
        RefusalV1::EditorCapacity => "editor_capacity",
        RefusalV1::EditorConflict => "editor_conflict",
        RefusalV1::EditorIncomplete => "editor_incomplete",
        RefusalV1::EditorUnavailable => "editor_unavailable",
        RefusalV1::CounterExhausted => "counter_exhausted",
    }
}

fn access_code(error: ServiceAccessErrorV1) -> &'static str {
    match error {
        ServiceAccessErrorV1::Unavailable => "service_unavailable",
        ServiceAccessErrorV1::Malformed => "service_malformed",
        ServiceAccessErrorV1::Limit => "service_limit",
    }
}

fn with_server_meta(mut result: Value) -> Value {
    result.as_object_mut().expect("object result").insert(
        "_meta".into(),
        json!({"io.modelcontextprotocol/serverInfo": {
            "name": "vellum-mcp",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Bounded MCP projection of a Vellum editor service"
        }}),
    );
    result
}

fn success(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn error_value(id: Value, code: i64, message: &'static str, data: Option<Value>) -> Value {
    let mut error = json!({"code": code, "message": message});
    if let Some(data) = data {
        error
            .as_object_mut()
            .expect("object error")
            .insert("data".into(), data);
    }
    json!({"jsonrpc": "2.0", "id": id, "error": error})
}

fn internal_error(id: Value) -> Vec<u8> {
    serde_json::to_vec(&error_value(id, -32603, "response exceeds limit", None))
        .expect("JSON values always serialize")
}

fn bump(counter: &mut u64, saturated: &mut bool) {
    match counter.checked_add(1) {
        Some(value) => *counter = value,
        None => *saturated = true,
    }
}

fn add(counter: &mut u64, amount: usize, saturated: &mut bool) {
    let Ok(amount) = u64::try_from(amount) else {
        *counter = u64::MAX;
        *saturated = true;
        return;
    };
    match counter.checked_add(amount) {
        Some(value) => *counter = value,
        None => {
            *counter = u64::MAX;
            *saturated = true;
        }
    }
}

fn record_duration(usage: &mut McpUsage, duration: std::time::Duration) {
    let micros = u64::try_from(duration.as_micros()).unwrap_or(u64::MAX);
    match usage.tool_call_micros_total.checked_add(micros) {
        Some(value) => usage.tool_call_micros_total = value,
        None => {
            usage.tool_call_micros_total = u64::MAX;
            usage.counter_saturated = true;
        }
    }
    usage.tool_call_micros_max = usage.tool_call_micros_max.max(micros);
}
