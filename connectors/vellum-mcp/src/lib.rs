//! Bounded MCP projection of the native Vellum client contract.
//!
//! This adapter implements only MCP `2026-07-28` over newline-delimited JSON-RPC.
//! MCP remains a projection: native commands, witnesses, refusals, and transport
//! failures retain their meaning at the `vellum-client` boundary.
#![forbid(unsafe_code)]

pub mod cli;

use serde_json::{Map, Value, json};
use std::io::{self, BufRead};
use vellum_sdk_client::{
    client::Client,
    protocol::{
        CommandV1, DurableEffectIntentV1, DurableEffectReceiptV1, DurableExecuteOutcomeV1,
        DurableExportCopyFailureV1, DurableExportCopyPersistenceV1, DurableOperationRefV1,
        DurableReceiptLookupOutcomeV1, DurableReceiptRetirementOutcomeV1, DurableReceiptStatusV1,
        EditPersistenceV1, FileAdmissionStatusV1, MAX_WORKSPACE_INVENTORY_PAGE_SIZE_V1,
        MembershipRemovalStatusV1, OutcomeV1, RefusalV1, ReplyV1, SaveReceiptV1,
        ServiceAccessErrorV1, ServicePort, ViewCloseStatusV1, ViewOpenStatusV1,
        WorkspaceChangeCursorV1, WorkspaceChangeKindV1, WorkspaceChangesV1,
        WorkspaceDocumentEditV1, WorkspaceDocumentIdV1, WorkspaceIdV1, WorkspaceInventoryCursorV1,
        WorkspaceReplacementV1, WorkspaceRootIdV1, WorkspaceRootKindV1,
    },
};

#[cfg(unix)]
use vellum_sdk_client::transport::{AuthenticatedUnixConnector, RemoteTransport};

pub const PROTOCOL_VERSION: &str = "2026-07-28";
pub const SNAPSHOT_TOOL: &str = "vellum.snapshot";
pub const REPLACE_TOOL: &str = "vellum.replace";
pub const REPLACE_AND_SAVE_TOOL: &str = "vellum.replace_and_save";
pub const WORKSPACE_EDIT_TOOL: &str = "vellum.workspace_edit";
pub const CHANGES_TOOL: &str = "vellum.poll_changes";
pub const RESOLVE_LOCATOR_TOOL: &str = "vellum.resolve_locator";
pub const MEMBERSHIP_TOOL: &str = "vellum.membership";
pub const DURABLE_EXECUTE_TOOL: &str = "vellum.durable_execute";
pub const DURABLE_RECEIPT_TOOL: &str = "vellum.durable_receipt";
pub const DURABLE_RETIRE_TOOL: &str = "vellum.durable_retire";
pub const DURABLE_STATUS_TOOL: &str = "vellum.durable_status";
const MAX_ROOT_LOOKUP_PAGES: usize = 256;
const MAX_WORKSPACE_EDIT_DOCUMENTS: usize = 16;
const MAX_WORKSPACE_EDIT_REPLACEMENTS: usize = 64;
const TOOL_ERROR_CODES: &[&str] = &[
    "authority_denied",
    "authority_unavailable",
    "unknown_client",
    "client_capacity",
    "client_lifetime_capacity",
    "request_capacity",
    "request_conflict",
    "witness_capacity",
    "unknown_witness",
    "unknown_document",
    "unknown_root",
    "document_in_use",
    "foreign_inventory_cursor",
    "stale_inventory_cursor",
    "invalid_inventory_cursor",
    "foreign_change_cursor",
    "invalid_change_cursor",
    "no_active_document",
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
    "service_not_sent",
    "service_outcome_unknown",
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
    pub resource_lists: u64,
    pub resource_reads: u64,
    pub resource_errors: u64,
    pub tool_call_micros_total: u64,
    pub tool_call_micros_max: u64,
    pub protocol_errors: u64,
    pub tool_errors: u64,
    pub durable_executes: u64,
    pub durable_lookups: u64,
    pub durable_retirements: u64,
    pub durable_statuses: u64,
    pub durable_complete: u64,
    pub durable_retired: u64,
    pub durable_conflict: u64,
    pub durable_outcome_unknown: u64,
    pub durable_pending: u64,
    pub durable_absent_or_foreign: u64,
    pub durable_unavailable: u64,
    pub retirement_retired: u64,
    pub retirement_conflict: u64,
    pub retirement_outcome_unknown: u64,
    pub retirement_absent_or_foreign: u64,
    pub retirement_unavailable: u64,
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
    pending_durable_edit: Option<PendingDurableEdit>,
    next_operation: u64,
    durable_projection: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingDurableEdit {
    operation: u64,
    document: WorkspaceDocumentIdV1,
    expected_revision: u64,
    start: u64,
    end: u64,
    inserted: String,
    stage: DurableEditStage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DurableEditStage {
    Select,
    Snapshot,
    Apply(CommandV1),
}

impl<T: ServicePort> McpServer<T> {
    pub fn new(client: Client<T>, limits: McpLimits) -> Result<Self, McpBuildError> {
        if limits.max_request_bytes == 0 || limits.max_response_bytes < 256 {
            return Err(McpBuildError::InvalidLimits);
        }
        Self::build(client, limits, false)
    }

    fn build(
        client: Client<T>,
        limits: McpLimits,
        durable_projection: bool,
    ) -> Result<Self, McpBuildError> {
        if limits.max_request_bytes == 0 || limits.max_response_bytes < 256 {
            return Err(McpBuildError::InvalidLimits);
        }
        Ok(Self {
            client,
            limits,
            usage: McpUsage::default(),
            pending_durable_edit: None,
            next_operation: 1,
            durable_projection,
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
                success(id, discover_result(self.durable_projection))
            }
            "tools/list" => {
                if !only_keys(params, &["_meta"]) {
                    self.protocol_error_value(id, -32602, "cursor is not supported", None)
                } else {
                    bump(
                        &mut self.usage.tool_lists,
                        &mut self.usage.counter_saturated,
                    );
                    success(id, tools_result(self.durable_projection))
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
            "resources/list" => {
                if !params
                    .keys()
                    .all(|key| matches!(key.as_str(), "_meta" | "cursor"))
                {
                    self.protocol_error_value(id, -32602, "invalid resource params", None)
                } else {
                    bump(
                        &mut self.usage.resource_lists,
                        &mut self.usage.counter_saturated,
                    );
                    self.list_resources(id, params)
                }
            }
            "resources/read" => {
                if !only_keys(params, &["_meta", "uri"]) {
                    self.protocol_error_value(id, -32602, "invalid resource params", None)
                } else {
                    bump(
                        &mut self.usage.resource_reads,
                        &mut self.usage.counter_saturated,
                    );
                    self.read_resource(id, params)
                }
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
                if params.get("arguments").is_some_and(|value| {
                    value.as_object().and_then(document_arguments).is_some()
                }) =>
            {
                let document = document_arguments(
                    params
                        .get("arguments")
                        .and_then(Value::as_object)
                        .expect("validated object"),
                )
                .expect("validated document");
                self.snapshot_document(document)
            }
            SNAPSHOT_TOOL => {
                return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
            }
            REPLACE_TOOL => {
                let Some(arguments) = params.get("arguments").and_then(Value::as_object) else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                let Some((document, expected_revision, start, end, inserted, operation)) =
                    replace_arguments(arguments)
                else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                if operation.is_some() {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                }
                self.replace_at_revision(document, expected_revision, start, end, inserted)
            }
            REPLACE_AND_SAVE_TOOL => {
                let Some(arguments) = params.get("arguments").and_then(Value::as_object) else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                let Some((document, expected_revision, start, end, inserted, operation)) =
                    replace_arguments(arguments)
                else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                self.replace_and_save_at_revision(
                    document,
                    expected_revision,
                    start,
                    end,
                    inserted,
                    operation,
                )
            }
            WORKSPACE_EDIT_TOOL => {
                let Some(arguments) = params.get("arguments").and_then(Value::as_object) else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                let Some(documents) = workspace_edit_arguments(arguments) else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                self.client.workspace_edit(documents)
            }
            CHANGES_TOOL => {
                let Some(arguments) = params.get("arguments").and_then(Value::as_object) else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                let Some((after, page_size)) = change_arguments(arguments) else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                self.client.workspace_changes(after, page_size)
            }
            RESOLVE_LOCATOR_TOOL => {
                let Some(arguments) = params.get("arguments").and_then(Value::as_object) else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                let Some(locator) = locator_argument(arguments) else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                self.client.resolve_locator(locator)
            }
            MEMBERSHIP_TOOL => {
                let Some(arguments) = params.get("arguments").and_then(Value::as_object) else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                let Some(command) = membership_arguments(arguments) else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                match command {
                    MembershipCommand::AddFile(locator) => self.client.add_file(locator),
                    MembershipCommand::AddDirectory(locator) => self.client.add_directory(locator),
                    MembershipCommand::RemoveDocument(document) => {
                        self.client.remove_document(document)
                    }
                    MembershipCommand::RemoveRoot(root) => self.client.remove_root(root),
                }
            }
            DURABLE_EXECUTE_TOOL if self.durable_projection => {
                let Some(arguments) = params.get("arguments").and_then(Value::as_object) else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                let Some((operation, intent)) = durable_arguments(arguments) else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                bump(
                    &mut self.usage.durable_executes,
                    &mut self.usage.counter_saturated,
                );
                return self.durable_execute_result(id, operation, intent);
            }
            DURABLE_RECEIPT_TOOL if self.durable_projection => {
                let Some(arguments) = params.get("arguments").and_then(Value::as_object) else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                let Some((operation, intent)) = durable_arguments(arguments) else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                bump(
                    &mut self.usage.durable_lookups,
                    &mut self.usage.counter_saturated,
                );
                return self.durable_lookup_result(id, operation, intent);
            }
            DURABLE_RETIRE_TOOL if self.durable_projection => {
                let Some(arguments) = params.get("arguments").and_then(Value::as_object) else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                let Some((operation, intent)) = durable_arguments(arguments) else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                bump(
                    &mut self.usage.durable_retirements,
                    &mut self.usage.counter_saturated,
                );
                return self.durable_retirement_result(id, operation, intent);
            }
            DURABLE_STATUS_TOOL if self.durable_projection => {
                let Some(arguments) = params.get("arguments").and_then(Value::as_object) else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                let Some(operation) = durable_status_argument(arguments) else {
                    return self.protocol_error_value(id, -32602, "invalid tool arguments", None);
                };
                bump(
                    &mut self.usage.durable_statuses,
                    &mut self.usage.counter_saturated,
                );
                return self.durable_status_result(id, operation);
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
                if let Some(pending) = &self.pending_durable_edit {
                    success(
                        id,
                        tool_error_with_operation(access_code(error), pending.operation),
                    )
                } else {
                    success(id, tool_error(access_code(error)))
                }
            }
        }
    }

    fn durable_execute_result(
        &mut self,
        id: Value,
        operation: DurableOperationRefV1,
        intent: DurableEffectIntentV1,
    ) -> Value {
        match self.client.execute_durable(operation, intent.clone()) {
            Ok(outcome) => {
                bump(
                    &mut self.usage.durable_complete,
                    &mut self.usage.counter_saturated,
                );
                if matches!(
                    &outcome,
                    DurableExecuteOutcomeV1::Refused(RefusalV1::RequestConflict)
                ) {
                    bump(
                        &mut self.usage.durable_conflict,
                        &mut self.usage.counter_saturated,
                    );
                }
                if matches!(&outcome, DurableExecuteOutcomeV1::Retired) {
                    bump(
                        &mut self.usage.durable_retired,
                        &mut self.usage.counter_saturated,
                    );
                }
                if matches!(&outcome, DurableExecuteOutcomeV1::Refused(_)) {
                    bump(
                        &mut self.usage.tool_errors,
                        &mut self.usage.counter_saturated,
                    );
                }
                success(id, durable_execute_json(outcome, &intent))
            }
            Err(error) => {
                match error {
                    ServiceAccessErrorV1::OutcomeUnknown => bump(
                        &mut self.usage.durable_outcome_unknown,
                        &mut self.usage.counter_saturated,
                    ),
                    ServiceAccessErrorV1::Unavailable => bump(
                        &mut self.usage.durable_unavailable,
                        &mut self.usage.counter_saturated,
                    ),
                    ServiceAccessErrorV1::NotSent
                    | ServiceAccessErrorV1::Malformed
                    | ServiceAccessErrorV1::Limit => {}
                }
                bump(
                    &mut self.usage.tool_errors,
                    &mut self.usage.counter_saturated,
                );
                success(id, tool_error(access_code(error)))
            }
        }
    }

    fn durable_lookup_result(
        &mut self,
        id: Value,
        operation: DurableOperationRefV1,
        intent: DurableEffectIntentV1,
    ) -> Value {
        match self
            .client
            .lookup_durable_receipt(operation, intent.clone())
        {
            Ok(outcome) => {
                let counter = match &outcome {
                    DurableReceiptLookupOutcomeV1::Complete(_) => &mut self.usage.durable_complete,
                    DurableReceiptLookupOutcomeV1::RequestConflict => {
                        &mut self.usage.durable_conflict
                    }
                    DurableReceiptLookupOutcomeV1::OutcomeUnknown => {
                        &mut self.usage.durable_outcome_unknown
                    }
                    DurableReceiptLookupOutcomeV1::AbsentOrForeign => {
                        &mut self.usage.durable_absent_or_foreign
                    }
                    DurableReceiptLookupOutcomeV1::Unavailable => {
                        &mut self.usage.durable_unavailable
                    }
                };
                bump(counter, &mut self.usage.counter_saturated);
                if matches!(
                    &outcome,
                    DurableReceiptLookupOutcomeV1::Complete(DurableExecuteOutcomeV1::Refused(
                        RefusalV1::RequestConflict
                    ))
                ) {
                    bump(
                        &mut self.usage.durable_conflict,
                        &mut self.usage.counter_saturated,
                    );
                }
                if matches!(
                    &outcome,
                    DurableReceiptLookupOutcomeV1::Complete(DurableExecuteOutcomeV1::Refused(_))
                        | DurableReceiptLookupOutcomeV1::RequestConflict
                ) {
                    bump(
                        &mut self.usage.tool_errors,
                        &mut self.usage.counter_saturated,
                    );
                }
                success(id, durable_lookup_json(outcome, &intent))
            }
            Err(error) => {
                match error {
                    ServiceAccessErrorV1::OutcomeUnknown => bump(
                        &mut self.usage.durable_outcome_unknown,
                        &mut self.usage.counter_saturated,
                    ),
                    ServiceAccessErrorV1::Unavailable => bump(
                        &mut self.usage.durable_unavailable,
                        &mut self.usage.counter_saturated,
                    ),
                    ServiceAccessErrorV1::NotSent
                    | ServiceAccessErrorV1::Malformed
                    | ServiceAccessErrorV1::Limit => {}
                }
                bump(
                    &mut self.usage.tool_errors,
                    &mut self.usage.counter_saturated,
                );
                success(id, tool_error(access_code(error)))
            }
        }
    }

    fn durable_retirement_result(
        &mut self,
        id: Value,
        operation: DurableOperationRefV1,
        intent: DurableEffectIntentV1,
    ) -> Value {
        match self.client.retire_durable_receipt(operation, intent) {
            Ok(outcome) => {
                let counter = match outcome {
                    DurableReceiptRetirementOutcomeV1::Retired => {
                        &mut self.usage.retirement_retired
                    }
                    DurableReceiptRetirementOutcomeV1::AbsentOrForeign => {
                        &mut self.usage.retirement_absent_or_foreign
                    }
                    DurableReceiptRetirementOutcomeV1::RequestConflict => {
                        &mut self.usage.retirement_conflict
                    }
                    DurableReceiptRetirementOutcomeV1::OutcomeUnknown => {
                        &mut self.usage.retirement_outcome_unknown
                    }
                    DurableReceiptRetirementOutcomeV1::Unavailable => {
                        &mut self.usage.retirement_unavailable
                    }
                };
                bump(counter, &mut self.usage.counter_saturated);
                if outcome == DurableReceiptRetirementOutcomeV1::RequestConflict {
                    bump(
                        &mut self.usage.tool_errors,
                        &mut self.usage.counter_saturated,
                    );
                }
                success(id, durable_retirement_json(outcome))
            }
            Err(error) => {
                match error {
                    ServiceAccessErrorV1::OutcomeUnknown => bump(
                        &mut self.usage.retirement_outcome_unknown,
                        &mut self.usage.counter_saturated,
                    ),
                    ServiceAccessErrorV1::Unavailable => bump(
                        &mut self.usage.retirement_unavailable,
                        &mut self.usage.counter_saturated,
                    ),
                    ServiceAccessErrorV1::NotSent
                    | ServiceAccessErrorV1::Malformed
                    | ServiceAccessErrorV1::Limit => {}
                }
                bump(
                    &mut self.usage.tool_errors,
                    &mut self.usage.counter_saturated,
                );
                success(id, tool_error(access_code(error)))
            }
        }
    }

    fn durable_status_result(&mut self, id: Value, operation: DurableOperationRefV1) -> Value {
        match self.client.durable_receipt_status(operation) {
            Ok(status) => {
                let counter = match status {
                    DurableReceiptStatusV1::Pending => &mut self.usage.durable_pending,
                    DurableReceiptStatusV1::Complete => &mut self.usage.durable_complete,
                    DurableReceiptStatusV1::Retired => &mut self.usage.durable_retired,
                    DurableReceiptStatusV1::AbsentOrForeign => {
                        &mut self.usage.durable_absent_or_foreign
                    }
                    DurableReceiptStatusV1::Unavailable => &mut self.usage.durable_unavailable,
                };
                bump(counter, &mut self.usage.counter_saturated);
                success(id, durable_status_json(status))
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
        document: WorkspaceDocumentIdV1,
        expected_revision: u64,
        start: u64,
        end: u64,
        inserted: String,
    ) -> Result<ReplyV1, ServiceAccessErrorV1> {
        match self.client.select_document(document)? {
            ReplyV1::Accepted(OutcomeV1::DocumentSelected(selected)) if selected == document => {}
            ReplyV1::Refused(refusal) => return Ok(ReplyV1::Refused(refusal)),
            _ => return Err(ServiceAccessErrorV1::Malformed),
        }
        match self.client.snapshot()? {
            ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot))
                if snapshot.revision == expected_revision =>
            {
                self.client.replace(snapshot.witness, start, end, inserted)
            }
            ReplyV1::Accepted(OutcomeV1::Snapshot(_)) => Ok(ReplyV1::Refused(RefusalV1::Stale)),
            ReplyV1::Accepted(
                OutcomeV1::WorkspaceInventory(_)
                | OutcomeV1::LocatorResolved(_)
                | OutcomeV1::WorkspaceChanges(_)
                | OutcomeV1::WorkspaceInventoryPage(_)
                | OutcomeV1::ViewOpened(_)
                | OutcomeV1::ViewClosed(_)
                | OutcomeV1::FileAdded(_)
                | OutcomeV1::DirectoryAdded(_)
                | OutcomeV1::FileAdmitted(_)
                | OutcomeV1::DirectoryAdmitted(_)
                | OutcomeV1::DocumentRemoved(_)
                | OutcomeV1::RootRemoved(_)
                | OutcomeV1::DocumentSelected(_)
                | OutcomeV1::Saved(_)
                | OutcomeV1::ReplacedAndSaved(_)
                | OutcomeV1::Replaced(_)
                | OutcomeV1::WorkspaceEdited(_),
            ) => Err(ServiceAccessErrorV1::Malformed),
            ReplyV1::Refused(refusal) => Ok(ReplyV1::Refused(refusal)),
        }
    }

    fn replace_and_save_at_revision(
        &mut self,
        document: WorkspaceDocumentIdV1,
        expected_revision: u64,
        start: u64,
        end: u64,
        inserted: String,
        operation: Option<u64>,
    ) -> Result<ReplyV1, ServiceAccessErrorV1> {
        let (operation_id, mut stage) = if let Some(pending) = self.pending_durable_edit.clone() {
            if operation != Some(pending.operation)
                || pending.document != document
                || pending.expected_revision != expected_revision
                || pending.start != start
                || pending.end != end
                || pending.inserted != inserted
            {
                return Err(ServiceAccessErrorV1::OutcomeUnknown);
            }
            (pending.operation, pending.stage)
        } else {
            if operation.is_some() {
                return Err(ServiceAccessErrorV1::Malformed);
            }
            if self.next_operation == 0 {
                return Ok(ReplyV1::Refused(RefusalV1::CounterExhausted));
            }
            (self.next_operation, DurableEditStage::Select)
        };

        loop {
            let result = match stage.clone() {
                DurableEditStage::Select => self.client.select_document(document),
                DurableEditStage::Snapshot => self.client.snapshot(),
                DurableEditStage::Apply(command) => self.client.send(command),
            };
            let reply = match result {
                Ok(reply) => reply,
                Err(error) => {
                    if self.client.pending_request().is_some() {
                        if operation_id == self.next_operation {
                            self.next_operation = self.next_operation.checked_add(1).unwrap_or(0);
                        }
                        self.pending_durable_edit = Some(PendingDurableEdit {
                            operation: operation_id,
                            document,
                            expected_revision,
                            start,
                            end,
                            inserted,
                            stage,
                        });
                    } else {
                        self.pending_durable_edit = None;
                    }
                    return Err(error);
                }
            };
            self.pending_durable_edit = None;
            match (stage, reply) {
                (
                    DurableEditStage::Select,
                    ReplyV1::Accepted(OutcomeV1::DocumentSelected(selected)),
                ) if selected == document => stage = DurableEditStage::Snapshot,
                (DurableEditStage::Select, ReplyV1::Refused(refusal))
                | (DurableEditStage::Snapshot, ReplyV1::Refused(refusal))
                | (DurableEditStage::Apply(_), ReplyV1::Refused(refusal)) => {
                    return Ok(ReplyV1::Refused(refusal));
                }
                (DurableEditStage::Snapshot, ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot)))
                    if snapshot.revision == expected_revision =>
                {
                    stage = DurableEditStage::Apply(CommandV1::ReplaceAndSave {
                        expected: snapshot.witness,
                        start,
                        end,
                        inserted: inserted.clone(),
                    });
                }
                (DurableEditStage::Snapshot, ReplyV1::Accepted(OutcomeV1::Snapshot(_))) => {
                    return Ok(ReplyV1::Refused(RefusalV1::Stale));
                }
                (DurableEditStage::Apply(_), reply) => return Ok(reply),
                _ => return Err(ServiceAccessErrorV1::Malformed),
            }
        }
    }

    fn snapshot_document(
        &mut self,
        document: WorkspaceDocumentIdV1,
    ) -> Result<ReplyV1, ServiceAccessErrorV1> {
        match self.client.select_document(document)? {
            ReplyV1::Accepted(OutcomeV1::DocumentSelected(selected)) if selected == document => {
                self.client.snapshot()
            }
            ReplyV1::Refused(refusal) => Ok(ReplyV1::Refused(refusal)),
            _ => Err(ServiceAccessErrorV1::Malformed),
        }
    }

    fn list_resources(&mut self, id: Value, params: &Map<String, Value>) -> Value {
        let cursor = match params.get("cursor") {
            None => None,
            Some(Value::String(value)) => match decode_inventory_cursor(value) {
                Some(cursor) => Some(cursor),
                None => {
                    return self.protocol_error_value(id, -32602, "invalid resource cursor", None);
                }
            },
            Some(_) => {
                return self.protocol_error_value(id, -32602, "invalid resource cursor", None);
            }
        };
        match self
            .client
            .workspace_inventory_page(cursor, MAX_WORKSPACE_INVENTORY_PAGE_SIZE_V1)
        {
            Ok(ReplyV1::Accepted(OutcomeV1::WorkspaceInventoryPage(page))) => {
                let workspace = page.workspace.get();
                let mut resources = Vec::with_capacity(page.documents.len() + page.roots.len());
                for document in page.documents {
                    resources.push(json!({
                        "uri": document_uri(document),
                        "name": format!("document-{}", document.get()),
                        "title": "Vellum workspace document",
                        "mimeType": "text/plain",
                        "description": "Revisioned Vellum document; select it explicitly before reading."
                    }));
                }
                for root in page.roots {
                    resources.push(json!({
                        "uri": root_uri(root.id),
                        "name": format!("root-{}", root.id.get()),
                        "title": "Vellum workspace root",
                        "mimeType": "application/vnd.vellum.root+json",
                        "description": match root.kind {
                            WorkspaceRootKindV1::File => "Content-free Vellum file-root descriptor.",
                            WorkspaceRootKindV1::Directory => "Content-free Vellum directory-root descriptor.",
                        }
                    }));
                }
                let mut result = json!({
                    "resultType": "complete",
                    "resources": resources,
                    "workspace": workspace,
                    "ttlMs": 0,
                    "cacheScope": "private"
                });
                if let Some(next) = page.next {
                    result.as_object_mut().expect("object").insert(
                        "nextCursor".into(),
                        Value::String(encode_inventory_cursor(next)),
                    );
                }
                success(id, with_server_meta(result))
            }
            Ok(ReplyV1::Refused(refusal)) => self.resource_error(id, refusal_code(refusal)),
            Ok(_) => self.resource_error(id, "service_malformed"),
            Err(error) => self.resource_error(id, access_code(error)),
        }
    }

    fn read_resource(&mut self, id: Value, params: &Map<String, Value>) -> Value {
        let Some(uri) = params.get("uri").and_then(Value::as_str) else {
            return self.protocol_error_value(id, -32602, "invalid resource uri", None);
        };
        let Some(resource) = parse_resource_uri(uri) else {
            return self.protocol_error_value(id, -32602, "invalid resource uri", None);
        };
        match resource {
            ResourceIdentity::Document(document) => match self.snapshot_document(document) {
                Ok(ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot))) => success(
                    id,
                    with_server_meta(json!({
                        "resultType": "complete",
                        "contents": [{
                            "uri": uri,
                            "mimeType": "text/plain",
                            "text": &*snapshot.text,
                            "_meta": {"vellum/revision": snapshot.revision}
                        }]
                    })),
                ),
                Ok(ReplyV1::Refused(refusal)) => self.resource_error(id, refusal_code(refusal)),
                Ok(_) => self.resource_error(id, "service_malformed"),
                Err(error) => self.resource_error(id, access_code(error)),
            },
            ResourceIdentity::Root(root) => match self.root_kind(root) {
                Ok(kind) => success(
                    id,
                    with_server_meta(json!({
                        "resultType": "complete",
                        "contents": [{
                            "uri": uri,
                            "mimeType": "application/vnd.vellum.root+json",
                            "text": match kind {
                                WorkspaceRootKindV1::File => "{\"kind\":\"file\"}",
                                WorkspaceRootKindV1::Directory => "{\"kind\":\"directory\"}",
                            },
                            "_meta": {"vellum/contentFreeDescriptor": true}
                        }]
                    })),
                ),
                Err(code) => self.resource_error(id, code),
            },
        }
    }

    fn root_kind(&mut self, root: WorkspaceRootIdV1) -> Result<WorkspaceRootKindV1, &'static str> {
        let mut cursor = None;
        for _ in 0..MAX_ROOT_LOOKUP_PAGES {
            match self
                .client
                .workspace_inventory_page(cursor, MAX_WORKSPACE_INVENTORY_PAGE_SIZE_V1)
            {
                Ok(ReplyV1::Accepted(OutcomeV1::WorkspaceInventoryPage(page))) => {
                    if page.workspace != root.workspace() {
                        return Err("unknown_root");
                    }
                    if let Some(entry) = page.roots.into_iter().find(|entry| entry.id == root) {
                        return Ok(entry.kind);
                    }
                    let Some(next) = page.next else {
                        return Err("unknown_root");
                    };
                    cursor = Some(next);
                }
                Ok(ReplyV1::Refused(refusal)) => return Err(refusal_code(refusal)),
                Ok(_) => return Err("service_malformed"),
                Err(error) => return Err(access_code(error)),
            }
        }
        Err("service_limit")
    }

    fn resource_error(&mut self, id: Value, code: &'static str) -> Value {
        bump(
            &mut self.usage.resource_errors,
            &mut self.usage.counter_saturated,
        );
        let (jsonrpc_code, message) = if matches!(code, "unknown_document" | "unknown_root") {
            (-32602, "resource not found")
        } else {
            (-32020, "resource request refused")
        };
        error_value(id, jsonrpc_code, message, Some(json!({"code": code})))
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

#[cfg(unix)]
impl McpServer<RemoteTransport<AuthenticatedUnixConnector>> {
    /// Enables the restart-stable operation tools only for the transport type
    /// that performs the daemon's required authentication handshake.
    pub fn new_authenticated(
        client: Client<RemoteTransport<AuthenticatedUnixConnector>>,
        limits: McpLimits,
    ) -> Result<Self, McpBuildError> {
        Self::build(client, limits, true)
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

fn discover_result(durable_projection: bool) -> Value {
    let instructions = if durable_projection {
        "Enumerate content-free resources. Persist a canonical nonzero vdo1 operation reference before the first durable execute; recover only through durable receipt lookup with the identical stable intent. An absent receipt never executes. Retire only a completed receipt with the same exact operation reference and intent; discovery of the retirement tool does not grant RetireReceipt authority."
    } else {
        "Enumerate content-free resources. Every document read/edit carries its scoped identity. Retry an outcome-unknown durable edit only with its returned operation handle and identical arguments on this stdio session."
    };
    with_server_meta(json!({
        "resultType": "complete",
        "supportedVersions": [PROTOCOL_VERSION],
        "capabilities": {
            "tools": {"listChanged": false},
            "resources": {"listChanged": false, "subscribe": false}
        },
        "instructions": instructions,
        "ttlMs": 0,
        "cacheScope": "private"
    }))
}

fn tools_result(durable_projection: bool) -> Value {
    let mut tools = vec![
        json!({
            "name": SNAPSHOT_TOOL,
            "title": "Read Vellum snapshot",
            "description": "Select and return one explicitly identified document and its revision.",
            "inputSchema": document_input_schema(),
            "outputSchema": tool_output_schema("snapshot"),
            "annotations": {"readOnlyHint": true, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false}
        }),
        json!({
            "name": REPLACE_TOOL,
            "title": "Replace Vellum text range",
            "description": "Replace a byte range only when the supplied workspace revision is still current.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expected_revision": {"type": "integer", "minimum": 0},
                    "workspace": {"type": "integer", "minimum": 1},
                    "document": {"type": "integer", "minimum": 1},
                    "start": {"type": "integer", "minimum": 0},
                    "end": {"type": "integer", "minimum": 0},
                    "inserted": {"type": "string"}
                },
                "required": ["workspace", "document", "expected_revision", "start", "end", "inserted"],
                "additionalProperties": false
            },
            "outputSchema": tool_output_schema("replaced"),
            "annotations": {"readOnlyHint": false, "destructiveHint": true, "idempotentHint": false, "openWorldHint": false}
        }),
        json!({
            "name": REPLACE_AND_SAVE_TOOL,
            "title": "Replace and durably save Vellum text range",
            "description": "Replace a byte range at the exact revision and return explicit durable, non-durable, unknown, or post-edit save-admission evidence.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expected_revision": {"type": "integer", "minimum": 0},
                    "workspace": {"type": "integer", "minimum": 1},
                    "document": {"type": "integer", "minimum": 1},
                    "start": {"type": "integer", "minimum": 0},
                    "end": {"type": "integer", "minimum": 0},
                    "inserted": {"type": "string"},
                    "operation": {"type": "string", "maxLength": 32}
                },
                "required": ["workspace", "document", "expected_revision", "start", "end", "inserted"],
                "additionalProperties": false
            },
            "outputSchema": durable_tool_output_schema(),
            "annotations": {"readOnlyHint": false, "destructiveHint": true, "idempotentHint": false, "openWorldHint": false}
        }),
        json!({
            "name": WORKSPACE_EDIT_TOOL,
            "title": "Atomically edit Vellum workspace documents",
            "description": "Publish a bounded set of byte-range replacements through one revision-witnessed in-memory transaction. This does not persist files.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "documents": {
                        "type": "array",
                        "minItems": 1,
                        "maxItems": MAX_WORKSPACE_EDIT_DOCUMENTS,
                        "items": {
                            "type": "object",
                            "properties": {
                                "workspace": {"type": "integer", "minimum": 1},
                                "document": {"type": "integer", "minimum": 1},
                                "expected_revision": {"type": "integer", "minimum": 0},
                                "replacements": {
                                    "type": "array",
                                    "minItems": 1,
                                    "maxItems": MAX_WORKSPACE_EDIT_REPLACEMENTS,
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "start": {"type": "integer", "minimum": 0},
                                            "end": {"type": "integer", "minimum": 0},
                                            "inserted": {"type": "string"}
                                        },
                                        "required": ["start", "end", "inserted"],
                                        "additionalProperties": false
                                    }
                                }
                            },
                            "required": ["workspace", "document", "expected_revision", "replacements"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["documents"],
                "additionalProperties": false
            },
            "outputSchema": workspace_edit_output_schema(),
            "annotations": {"readOnlyHint": false, "destructiveHint": true, "idempotentHint": false, "openWorldHint": false}
        }),
        json!({
            "name": CHANGES_TOOL,
            "title": "Poll bounded Vellum workspace changes",
            "description": "Return at most one content-free workspace change, an opaque continuation cursor, or an explicit retention gap.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cursor": {"type": "string", "maxLength": 96},
                    "page_size": {"type": "integer", "minimum": 1, "maximum": 1}
                },
                "required": ["page_size"],
                "additionalProperties": false
            },
            "outputSchema": generic_tool_output_schema(),
            "annotations": {"readOnlyHint": true, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false}
        }),
        json!({
            "name": RESOLVE_LOCATOR_TOOL,
            "title": "Resolve a Vellum workspace locator",
            "description": "Resolve one exact absolute locator to its existing opaque workspace identities without returning the locator, reading the filesystem, or changing membership.",
            "inputSchema": closed_schema(
                &["locator"],
                json!({"locator": {"type": "string", "minLength": 1, "maxLength": 4096}}),
            ),
            "outputSchema": locator_resolution_output_schema(),
            "annotations": {"readOnlyHint": true, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false}
        }),
        json!({
            "name": MEMBERSHIP_TOOL,
            "title": "Change Vellum workspace membership",
            "description": "Add or retire workspace membership. The daemon's independently configured authority policy must permit the requested operation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "operation": {"type": "string", "enum": ["add_file", "add_directory", "remove_document", "remove_root"]},
                    "locator": {"type": "string"},
                    "workspace": {"type": "integer", "minimum": 1},
                    "document": {"type": "integer", "minimum": 1},
                    "root": {"type": "integer", "minimum": 1}
                },
                "required": ["operation"],
                "additionalProperties": false
            },
            "outputSchema": generic_tool_output_schema(),
            "annotations": {"readOnlyHint": false, "destructiveHint": true, "idempotentHint": false, "openWorldHint": true}
        }),
    ];
    if durable_projection {
        tools.extend([
            json!({
                "name": DURABLE_EXECUTE_TOOL,
                "title": "Execute a restart-stable Vellum effect",
                "description": "Execute one stable intent under the nonzero 128-bit operation reference the harness persisted before this first attempt.",
                "inputSchema": durable_input_schema(),
                "outputSchema": durable_output_schema(false),
                "annotations": {"readOnlyHint": false, "destructiveHint": true, "idempotentHint": true, "openWorldHint": true}
            }),
            json!({
                "name": DURABLE_RECEIPT_TOOL,
                "title": "Look up a durable Vellum receipt",
                "description": "Read historical evidence for an exact operation reference and identical stable intent; absence never executes the intent.",
                "inputSchema": durable_input_schema(),
                "outputSchema": durable_output_schema(true),
                "annotations": {"readOnlyHint": true, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false}
            }),
            json!({
                "name": DURABLE_RETIRE_TOOL,
                "title": "Retire a completed durable Vellum receipt",
                "description": "Request retirement of exact completed historical evidence under the identical operation reference and stable intent. The daemon must independently grant RetireReceipt authority.",
                "inputSchema": durable_input_schema(),
                "outputSchema": durable_retirement_output_schema(),
                "annotations": {"readOnlyHint": false, "destructiveHint": true, "idempotentHint": true, "openWorldHint": false}
            }),
            json!({
                "name": DURABLE_STATUS_TOOL,
                "title": "Read durable Vellum operation status",
                "description": "Project only the authenticated subject- and workspace-scoped current state of one stable operation reference. This grants no execute or retirement authority.",
                "inputSchema": durable_status_input_schema(),
                "outputSchema": durable_status_output_schema(),
                "annotations": {"readOnlyHint": true, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false}
            })
        ]);
    }
    with_server_meta(json!({
        "resultType": "complete",
        "tools": tools,
        "ttlMs": 0,
        "cacheScope": "private"
    }))
}

fn document_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "workspace": {"type": "integer", "minimum": 1},
            "document": {"type": "integer", "minimum": 1}
        },
        "required": ["workspace", "document"],
        "additionalProperties": false
    })
}

fn durable_input_schema() -> Value {
    json!({
        "oneOf": [
            durable_intent_schema("add_file", &["locator"]),
            durable_intent_schema("add_directory", &["locator"]),
            durable_intent_schema("remove_document", &["workspace", "document"]),
            durable_intent_schema("remove_root", &["workspace", "root"]),
            durable_intent_schema("save_document", &["workspace", "document", "expected_revision"]),
            durable_intent_schema("replace_document_and_save", &["workspace", "document", "expected_revision", "start", "end", "inserted"]),
            durable_intent_schema("export_document_copy_exclusive", &["workspace", "document", "expected_revision", "destination"])
        ]
    })
}

fn durable_status_input_schema() -> Value {
    closed_schema(
        &["operation_ref"],
        json!({
            "operation_ref": {
                "type": "string",
                "pattern": "^vdo1[.](?!0{32}$)[0-9a-f]{32}$",
                "minLength": 37,
                "maxLength": 37
            }
        }),
    )
}

fn durable_intent_schema(intent: &'static str, fields: &[&str]) -> Value {
    let mut properties = Map::new();
    properties.insert(
        "operation_ref".into(),
        json!({
            "type": "string",
            "pattern": "^vdo1[.](?!0{32}$)[0-9a-f]{32}$",
            "minLength": 37,
            "maxLength": 37
        }),
    );
    properties.insert("intent".into(), json!({"const": intent}));
    for field in fields {
        let schema = match *field {
            "locator" | "inserted" => json!({"type": "string"}),
            "destination" => json!({"type": "string", "minLength": 1}),
            "workspace" | "document" | "root" => {
                json!({"type": "integer", "minimum": 1})
            }
            "expected_revision" | "start" | "end" => {
                json!({"type": "integer", "minimum": 0})
            }
            _ => unreachable!("closed durable intent field"),
        };
        properties.insert((*field).into(), schema);
    }
    let mut required = vec![json!("operation_ref"), json!("intent")];
    required.extend(fields.iter().map(|field| json!(field)));
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

fn durable_output_schema(lookup: bool) -> Value {
    let persistence = json!({
        "type": "object",
        "properties": {
            "status": {"type": "string", "enum": ["durable", "not_durable", "unknown", "not_attempted"]},
            "application": {"type": ["string", "null"]},
            "synchronization": {"type": ["string", "null"]},
            "no_future_effects": {"type": ["boolean", "null"]},
            "quiescent": {"type": ["boolean", "null"]},
            "adapter_failure": {"type": ["boolean", "null"]},
            "cleanup_failed": {"type": ["boolean", "null"]},
            "baseline_published": {"type": ["boolean", "null"]},
            "outcome_unknown": {"type": ["boolean", "null"]},
            "refusal": {"type": ["string", "null"]}
        },
        "required": ["status", "application", "synchronization", "no_future_effects", "quiescent", "adapter_failure", "cleanup_failed", "baseline_published", "outcome_unknown", "refusal"],
        "additionalProperties": false
    });
    let mut variants = vec![
        closed_schema(
            &["kind", "workspace", "document", "root", "status"],
            json!({
                "kind": {"const": "file_admission"}, "workspace": nonzero_integer_schema(),
                "document": nonzero_integer_schema(), "root": {"type": ["integer", "null"], "minimum": 1},
                "status": {"enum": ["added", "already_present"]}
            }),
        ),
        closed_schema(
            &[
                "kind",
                "workspace",
                "root",
                "added",
                "joined",
                "added_documents",
            ],
            json!({
                "kind": {"const": "directory_admission"}, "workspace": nonzero_integer_schema(),
                "root": nonzero_integer_schema(), "added": {"type": "integer", "minimum": 0},
                "joined": {"type": "integer", "minimum": 0},
                "added_documents": {"type": "array", "items": nonzero_integer_schema()}
            }),
        ),
        closed_schema(
            &["kind", "workspace", "document", "status"],
            json!({
                "kind": {"const": "document_removed"}, "workspace": nonzero_integer_schema(),
                "document": nonzero_integer_schema(), "status": {"enum": ["removed", "already_removed"]}
            }),
        ),
        closed_schema(
            &["kind", "workspace", "root", "status"],
            json!({
                "kind": {"const": "root_removed"}, "workspace": nonzero_integer_schema(),
                "root": nonzero_integer_schema(), "status": {"enum": ["removed", "already_removed"]}
            }),
        ),
        closed_schema(
            &[
                "kind",
                "workspace",
                "document",
                "source_revision",
                "persistence",
            ],
            json!({
                "kind": {"const": "document_saved"}, "workspace": nonzero_integer_schema(),
                "document": nonzero_integer_schema(), "source_revision": {"type": "integer", "minimum": 0},
                "persistence": persistence.clone()
            }),
        ),
        closed_schema(
            &[
                "kind",
                "workspace",
                "document",
                "source_revision",
                "text",
                "persistence",
            ],
            json!({
                "kind": {"const": "document_replaced_and_saved"}, "workspace": nonzero_integer_schema(),
                "document": nonzero_integer_schema(), "source_revision": {"type": "integer", "minimum": 0},
                "text": {"type": "string"}, "persistence": persistence
            }),
        ),
        closed_schema(
            &[
                "kind",
                "workspace",
                "document",
                "source_revision",
                "persistence",
            ],
            json!({
                "kind": {"const": "document_copy_exported_exclusive"},
                "workspace": nonzero_integer_schema(),
                "document": nonzero_integer_schema(),
                "source_revision": {"type": "integer", "minimum": 0},
                "persistence": export_persistence_schema()
            }),
        ),
        closed_schema(
            &["code"],
            json!({"code": {"type": "string", "enum": TOOL_ERROR_CODES}}),
        ),
    ];
    if lookup {
        for kind in ["outcome_unknown", "absent_or_foreign", "unavailable"] {
            variants.push(closed_schema(&["kind"], json!({"kind": {"const": kind}})));
        }
    } else {
        variants.push(closed_schema(
            &["kind"],
            json!({"kind": {"const": "retired"}}),
        ));
    }
    json!({"oneOf": variants})
}

fn durable_retirement_output_schema() -> Value {
    json!({
        "oneOf": [
            closed_schema(&["kind"], json!({"kind": {"const": "retired"}})),
            closed_schema(
                &["kind"],
                json!({"kind": {"const": "absent_or_foreign"}}),
            ),
            closed_schema(
                &["kind"],
                json!({"kind": {"const": "outcome_unknown"}}),
            ),
            closed_schema(&["kind"], json!({"kind": {"const": "unavailable"}})),
            closed_schema(
                &["code"],
                json!({"code": {"type": "string", "enum": TOOL_ERROR_CODES}}),
            ),
        ]
    })
}

fn durable_status_output_schema() -> Value {
    json!({
        "oneOf": [
            closed_schema(&["kind"], json!({"kind": {"enum": ["pending", "complete", "retired", "absent_or_foreign", "unavailable"]}})),
            closed_schema(&["code"], json!({"code": {"type": "string", "enum": TOOL_ERROR_CODES}}))
        ]
    })
}

fn export_persistence_schema() -> Value {
    json!({
        "oneOf": [
            closed_schema(
                &["status", "failure", "cleanup_failed"],
                json!({
                    "status": {"const": "durable"},
                    "failure": {"type": "null"},
                    "cleanup_failed": {"const": false}
                }),
            ),
            closed_schema(
                &["status", "failure", "cleanup_failed"],
                json!({
                    "status": {"enum": ["not_durable", "unknown", "not_attempted"]},
                    "failure": {"enum": ["conflict", "unsupported", "limit", "capacity", "io"]},
                    "cleanup_failed": {"type": "boolean"}
                }),
            )
        ]
    })
}

fn nonzero_integer_schema() -> Value {
    json!({"type": "integer", "minimum": 1})
}

fn closed_schema(required: &[&str], properties: Value) -> Value {
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

fn generic_tool_output_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": true
    })
}

fn locator_resolution_output_schema() -> Value {
    let root = closed_schema(
        &["root", "kind"],
        json!({
            "root": nonzero_integer_schema(),
            "kind": {"enum": ["file", "directory"]}
        }),
    );
    json!({
        "oneOf": [
            closed_schema(
                &["kind", "workspace", "document", "root"],
                json!({
                    "kind": {"const": "locator_resolved"},
                    "workspace": nonzero_integer_schema(),
                    "document": {"oneOf": [nonzero_integer_schema(), {"type": "null"}]},
                    "root": {"oneOf": [root, {"type": "null"}]}
                }),
            ),
            closed_schema(
                &["kind", "workspace"],
                json!({
                    "kind": {"const": "locator_absent_or_foreign"},
                    "workspace": nonzero_integer_schema()
                }),
            ),
            closed_schema(
                &["code"],
                json!({"code": {"type": "string", "enum": TOOL_ERROR_CODES}}),
            )
        ]
    })
}

fn workspace_edit_output_schema() -> Value {
    json!({
        "oneOf": [
            closed_schema(
                &["kind", "workspace", "generation", "persistence", "documents"],
                json!({
                    "kind": {"const": "workspace_edited"},
                    "workspace": {"type": "integer", "minimum": 1},
                    "generation": {"type": "integer", "minimum": 0},
                    "persistence": {"const": "not_attempted"},
                    "documents": {
                        "type": "array",
                        "maxItems": MAX_WORKSPACE_EDIT_DOCUMENTS,
                        "items": closed_schema(
                            &["workspace", "document", "revision", "text"],
                            json!({
                                "workspace": {"type": "integer", "minimum": 1},
                                "document": {"type": "integer", "minimum": 1},
                                "revision": {"type": "integer", "minimum": 0},
                                "text": {"type": "string"}
                            }),
                        )
                    }
                }),
            ),
            closed_schema(
                &["code"],
                json!({"code": {"type": "string", "enum": TOOL_ERROR_CODES}}),
            )
        ]
    })
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

fn durable_tool_output_schema() -> Value {
    json!({
        "oneOf": [
            {
                "type": "object",
                "properties": {
                    "kind": {"type": "string", "enum": ["replaced_and_saved"]},
                    "revision": {"type": "integer", "minimum": 0},
                    "text": {"type": "string"},
                    "persistence": {
                        "type": "object",
                        "properties": {
                            "status": {"type": "string", "enum": ["durable", "not_durable", "unknown", "not_attempted"]},
                            "application": {"type": ["string", "null"]},
                            "synchronization": {"type": ["string", "null"]},
                            "no_future_effects": {"type": ["boolean", "null"]},
                            "quiescent": {"type": ["boolean", "null"]},
                            "adapter_failure": {"type": ["boolean", "null"]},
                            "cleanup_failed": {"type": ["boolean", "null"]},
                            "baseline_published": {"type": ["boolean", "null"]},
                            "outcome_unknown": {"type": ["boolean", "null"]},
                            "refusal": {"type": ["string", "null"]}
                        },
                        "required": ["status", "application", "synchronization", "no_future_effects", "quiescent", "adapter_failure", "cleanup_failed", "baseline_published", "outcome_unknown", "refusal"],
                        "additionalProperties": false
                    }
                },
                "required": ["kind", "revision", "text", "persistence"],
                "additionalProperties": false
            },
            {
                "type": "object",
                "properties": {
                    "code": {"type": "string", "enum": TOOL_ERROR_CODES},
                    "operation": {"type": "string", "maxLength": 32}
                },
                "required": ["code"],
                "additionalProperties": false
            }
        ]
    })
}

fn document_arguments(arguments: &Map<String, Value>) -> Option<WorkspaceDocumentIdV1> {
    if !only_keys(arguments, &["workspace", "document"]) {
        return None;
    }
    let workspace = WorkspaceIdV1::new(arguments.get("workspace")?.as_u64()?)?;
    WorkspaceDocumentIdV1::new(workspace, arguments.get("document")?.as_u64()?)
}

fn locator_argument(arguments: &Map<String, Value>) -> Option<String> {
    if !only_keys(arguments, &["locator"]) {
        return None;
    }
    let locator = arguments.get("locator")?.as_str()?;
    if locator.is_empty() || locator.len() > 4096 {
        return None;
    }
    Some(locator.to_owned())
}

fn replace_arguments(
    arguments: &Map<String, Value>,
) -> Option<(WorkspaceDocumentIdV1, u64, u64, u64, String, Option<u64>)> {
    if !arguments.keys().all(|key| {
        matches!(
            key.as_str(),
            "workspace"
                | "document"
                | "expected_revision"
                | "start"
                | "end"
                | "inserted"
                | "operation"
        )
    }) || !matches!(arguments.len(), 6 | 7)
    {
        return None;
    }
    let workspace = WorkspaceIdV1::new(arguments.get("workspace")?.as_u64()?)?;
    let document = WorkspaceDocumentIdV1::new(workspace, arguments.get("document")?.as_u64()?)?;
    let expected = arguments.get("expected_revision")?.as_u64()?;
    let start = arguments.get("start")?.as_u64()?;
    let end = arguments.get("end")?.as_u64()?;
    let inserted = arguments.get("inserted")?.as_str()?.to_owned();
    let operation = match arguments.get("operation") {
        None => None,
        Some(value) => Some(decode_operation(value.as_str()?)?),
    };
    Some((document, expected, start, end, inserted, operation))
}

fn workspace_edit_arguments(
    arguments: &Map<String, Value>,
) -> Option<Vec<WorkspaceDocumentEditV1>> {
    if !only_keys(arguments, &["documents"]) {
        return None;
    }
    let values = arguments.get("documents")?.as_array()?;
    if values.is_empty() || values.len() > MAX_WORKSPACE_EDIT_DOCUMENTS {
        return None;
    }
    let mut documents = Vec::with_capacity(values.len());
    for value in values {
        let document = value.as_object()?;
        if !only_keys(
            document,
            &["workspace", "document", "expected_revision", "replacements"],
        ) {
            return None;
        }
        let workspace = WorkspaceIdV1::new(document.get("workspace")?.as_u64()?)?;
        let id = WorkspaceDocumentIdV1::new(workspace, document.get("document")?.as_u64()?)?;
        if documents
            .iter()
            .any(|prior: &WorkspaceDocumentEditV1| prior.document == id)
        {
            return None;
        }
        let values = document.get("replacements")?.as_array()?;
        if values.is_empty() || values.len() > MAX_WORKSPACE_EDIT_REPLACEMENTS {
            return None;
        }
        let mut replacements = Vec::with_capacity(values.len());
        for value in values {
            let replacement = value.as_object()?;
            if !only_keys(replacement, &["start", "end", "inserted"]) {
                return None;
            }
            replacements.push(WorkspaceReplacementV1 {
                start: replacement.get("start")?.as_u64()?,
                end: replacement.get("end")?.as_u64()?,
                inserted: replacement.get("inserted")?.as_str()?.to_owned(),
            });
        }
        documents.push(WorkspaceDocumentEditV1 {
            document: id,
            expected_revision: document.get("expected_revision")?.as_u64()?,
            replacements,
        });
    }
    Some(documents)
}

fn change_arguments(
    arguments: &Map<String, Value>,
) -> Option<(Option<WorkspaceChangeCursorV1>, u16)> {
    if !arguments
        .keys()
        .all(|key| matches!(key.as_str(), "cursor" | "page_size"))
        || arguments.len() > 2
    {
        return None;
    }
    let page_size = u16::try_from(arguments.get("page_size")?.as_u64()?).ok()?;
    if page_size != 1 {
        return None;
    }
    let after = match arguments.get("cursor") {
        None => None,
        Some(value) => Some(decode_change_cursor(value.as_str()?)?),
    };
    Some((after, page_size))
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MembershipCommand {
    AddFile(String),
    AddDirectory(String),
    RemoveDocument(WorkspaceDocumentIdV1),
    RemoveRoot(WorkspaceRootIdV1),
}

fn membership_arguments(arguments: &Map<String, Value>) -> Option<MembershipCommand> {
    let operation = arguments.get("operation")?.as_str()?;
    match operation {
        "add_file" if only_keys(arguments, &["operation", "locator"]) => Some(
            MembershipCommand::AddFile(arguments.get("locator")?.as_str()?.to_owned()),
        ),
        "add_directory" if only_keys(arguments, &["operation", "locator"]) => Some(
            MembershipCommand::AddDirectory(arguments.get("locator")?.as_str()?.to_owned()),
        ),
        "remove_document" if only_keys(arguments, &["operation", "workspace", "document"]) => {
            let workspace = WorkspaceIdV1::new(arguments.get("workspace")?.as_u64()?)?;
            Some(MembershipCommand::RemoveDocument(
                WorkspaceDocumentIdV1::new(workspace, arguments.get("document")?.as_u64()?)?,
            ))
        }
        "remove_root" if only_keys(arguments, &["operation", "workspace", "root"]) => {
            let workspace = WorkspaceIdV1::new(arguments.get("workspace")?.as_u64()?)?;
            Some(MembershipCommand::RemoveRoot(WorkspaceRootIdV1::new(
                workspace,
                arguments.get("root")?.as_u64()?,
            )?))
        }
        _ => None,
    }
}

fn durable_arguments(
    arguments: &Map<String, Value>,
) -> Option<(DurableOperationRefV1, DurableEffectIntentV1)> {
    let operation = decode_durable_operation(arguments.get("operation_ref")?.as_str()?)?;
    let intent = match arguments.get("intent")?.as_str()? {
        "add_file" if only_keys(arguments, &["operation_ref", "intent", "locator"]) => {
            DurableEffectIntentV1::AddFile {
                locator: arguments.get("locator")?.as_str()?.to_owned(),
            }
        }
        "add_directory" if only_keys(arguments, &["operation_ref", "intent", "locator"]) => {
            DurableEffectIntentV1::AddDirectory {
                locator: arguments.get("locator")?.as_str()?.to_owned(),
            }
        }
        "remove_document"
            if only_keys(
                arguments,
                &["operation_ref", "intent", "workspace", "document"],
            ) =>
        {
            let workspace = WorkspaceIdV1::new(arguments.get("workspace")?.as_u64()?)?;
            DurableEffectIntentV1::RemoveDocument {
                document: WorkspaceDocumentIdV1::new(
                    workspace,
                    arguments.get("document")?.as_u64()?,
                )?,
            }
        }
        "remove_root"
            if only_keys(arguments, &["operation_ref", "intent", "workspace", "root"]) =>
        {
            let workspace = WorkspaceIdV1::new(arguments.get("workspace")?.as_u64()?)?;
            DurableEffectIntentV1::RemoveRoot {
                root: WorkspaceRootIdV1::new(workspace, arguments.get("root")?.as_u64()?)?,
            }
        }
        "save_document"
            if only_keys(
                arguments,
                &[
                    "operation_ref",
                    "intent",
                    "workspace",
                    "document",
                    "expected_revision",
                ],
            ) =>
        {
            let workspace = WorkspaceIdV1::new(arguments.get("workspace")?.as_u64()?)?;
            DurableEffectIntentV1::SaveDocument {
                document: WorkspaceDocumentIdV1::new(
                    workspace,
                    arguments.get("document")?.as_u64()?,
                )?,
                expected_revision: arguments.get("expected_revision")?.as_u64()?,
            }
        }
        "replace_document_and_save"
            if only_keys(
                arguments,
                &[
                    "operation_ref",
                    "intent",
                    "workspace",
                    "document",
                    "expected_revision",
                    "start",
                    "end",
                    "inserted",
                ],
            ) =>
        {
            let workspace = WorkspaceIdV1::new(arguments.get("workspace")?.as_u64()?)?;
            DurableEffectIntentV1::ReplaceDocumentAndSave {
                document: WorkspaceDocumentIdV1::new(
                    workspace,
                    arguments.get("document")?.as_u64()?,
                )?,
                expected_revision: arguments.get("expected_revision")?.as_u64()?,
                start: arguments.get("start")?.as_u64()?,
                end: arguments.get("end")?.as_u64()?,
                inserted: arguments.get("inserted")?.as_str()?.to_owned(),
            }
        }
        "export_document_copy_exclusive"
            if only_keys(
                arguments,
                &[
                    "operation_ref",
                    "intent",
                    "workspace",
                    "document",
                    "expected_revision",
                    "destination",
                ],
            ) =>
        {
            let workspace = WorkspaceIdV1::new(arguments.get("workspace")?.as_u64()?)?;
            let destination = arguments.get("destination")?.as_str()?;
            if destination.is_empty() {
                return None;
            }
            DurableEffectIntentV1::ExportDocumentCopyExclusive {
                document: WorkspaceDocumentIdV1::new(
                    workspace,
                    arguments.get("document")?.as_u64()?,
                )?,
                expected_revision: arguments.get("expected_revision")?.as_u64()?,
                destination: destination.to_owned(),
            }
        }
        _ => return None,
    };
    Some((operation, intent))
}

fn durable_status_argument(arguments: &Map<String, Value>) -> Option<DurableOperationRefV1> {
    only_keys(arguments, &["operation_ref"])
        .then(|| decode_durable_operation(arguments.get("operation_ref")?.as_str()?))?
}

fn decode_durable_operation(value: &str) -> Option<DurableOperationRefV1> {
    let hex = value.strip_prefix("vdo1.")?;
    if hex.len() != 32
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    let mut bytes = [0_u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).ok()?;
    }
    DurableOperationRefV1::new(bytes)
}

fn encode_inventory_cursor(cursor: WorkspaceInventoryCursorV1) -> String {
    format!(
        "vi1.{:x}.{:x}.{:x}",
        cursor.workspace().get(),
        cursor.generation(),
        cursor.offset()
    )
}

fn decode_inventory_cursor(value: &str) -> Option<WorkspaceInventoryCursorV1> {
    let mut fields = value.split('.');
    if fields.next()? != "vi1" {
        return None;
    }
    let workspace = parse_hex_nonzero(fields.next()?)?;
    let generation = parse_hex_nonzero(fields.next()?)?;
    let offset = parse_hex_nonzero(fields.next()?)?;
    if fields.next().is_some() || value.len() > 72 {
        return None;
    }
    WorkspaceInventoryCursorV1::new(WorkspaceIdV1::new(workspace)?, generation, offset)
}

fn encode_change_cursor(cursor: WorkspaceChangeCursorV1) -> String {
    format!(
        "vc1.{:x}.{:x}.{:x}",
        cursor.workspace().get(),
        cursor.epoch(),
        cursor.revision()
    )
}

fn decode_change_cursor(value: &str) -> Option<WorkspaceChangeCursorV1> {
    let mut fields = value.split('.');
    if fields.next()? != "vc1" {
        return None;
    }
    let workspace = parse_hex_nonzero(fields.next()?)?;
    let epoch = parse_hex_nonzero(fields.next()?)?;
    let revision = parse_hex(fields.next()?)?;
    if fields.next().is_some() || value.len() > 72 {
        return None;
    }
    WorkspaceChangeCursorV1::new(WorkspaceIdV1::new(workspace)?, epoch, revision)
}

fn parse_hex(value: &str) -> Option<u64> {
    if value.is_empty() || value.len() > 16 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    u64::from_str_radix(value, 16).ok()
}

fn parse_hex_nonzero(value: &str) -> Option<u64> {
    let parsed = parse_hex(value)?;
    (parsed != 0).then_some(parsed)
}

fn encode_operation(operation: u64) -> String {
    format!("vo1.{operation:016x}")
}

fn decode_operation(value: &str) -> Option<u64> {
    let encoded = value.strip_prefix("vo1.")?;
    if encoded.len() != 16 {
        return None;
    }
    parse_hex_nonzero(encoded)
}

fn document_uri(document: WorkspaceDocumentIdV1) -> String {
    format!(
        "vellum://workspace/{}/document/{}",
        document.workspace().get(),
        document.get()
    )
}

fn root_uri(root: WorkspaceRootIdV1) -> String {
    format!(
        "vellum://workspace/{}/root/{}",
        root.workspace().get(),
        root.get()
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResourceIdentity {
    Document(WorkspaceDocumentIdV1),
    Root(WorkspaceRootIdV1),
}

fn parse_resource_uri(uri: &str) -> Option<ResourceIdentity> {
    if uri.len() > 128 {
        return None;
    }
    let fields: Vec<_> = uri.split('/').collect();
    if fields.len() != 6
        || fields[0] != "vellum:"
        || !fields[1].is_empty()
        || fields[2] != "workspace"
    {
        return None;
    }
    let workspace = WorkspaceIdV1::new(fields[3].parse().ok()?)?;
    match fields[4] {
        "document" if fields.len() == 6 => Some(ResourceIdentity::Document(
            WorkspaceDocumentIdV1::new(workspace, fields[5].parse().ok()?)?,
        )),
        "root" => Some(ResourceIdentity::Root(WorkspaceRootIdV1::new(
            workspace,
            fields[5].parse().ok()?,
        )?)),
        _ => None,
    }
}

fn only_keys(arguments: &Map<String, Value>, allowed: &[&str]) -> bool {
    arguments.len() == allowed.len() && arguments.keys().all(|key| allowed.contains(&key.as_str()))
}

fn tool_success(outcome: OutcomeV1) -> Value {
    let (kind, snapshot) = match outcome {
        OutcomeV1::WorkspaceInventory(inventory)
        | OutcomeV1::FileAdded(inventory)
        | OutcomeV1::DirectoryAdded(inventory) => {
            let structured = inventory_json(inventory);
            return tool_complete("workspace_inventory", structured);
        }
        OutcomeV1::LocatorResolved(resolution) => {
            if resolution.document.is_none() && resolution.root.is_none() {
                return tool_complete(
                    "locator_absent_or_foreign",
                    json!({
                        "kind": "locator_absent_or_foreign",
                        "workspace": resolution.workspace.get()
                    }),
                );
            }
            let root = resolution.root.map(|root| {
                json!({
                    "root": root.id.get(),
                    "kind": match root.kind {
                        WorkspaceRootKindV1::File => "file",
                        WorkspaceRootKindV1::Directory => "directory",
                    }
                })
            });
            return tool_complete(
                "locator_resolved",
                json!({
                    "kind": "locator_resolved",
                    "workspace": resolution.workspace.get(),
                    "document": resolution.document.map(|document| document.get()),
                    "root": root
                }),
            );
        }
        OutcomeV1::FileAdmitted(receipt) => {
            let root = receipt.root.map(|root| {
                json!({
                    "workspace": root.workspace().get(),
                    "root": root.get(),
                    "uri": root_uri(root)
                })
            });
            return tool_complete(
                "file_admission",
                json!({
                    "kind": "file_admission",
                    "workspace": receipt.document.workspace().get(),
                    "document": receipt.document.get(),
                    "document_uri": document_uri(receipt.document),
                    "root": root,
                    "status": match receipt.status {
                        FileAdmissionStatusV1::Added => "added",
                        FileAdmissionStatusV1::AlreadyPresent => "already_present",
                    }
                }),
            );
        }
        OutcomeV1::DirectoryAdmitted(receipt) => {
            return tool_complete(
                "directory_admission",
                json!({
                    "kind": "directory_admission",
                    "workspace": receipt.root.workspace().get(),
                    "root": receipt.root.get(),
                    "root_uri": root_uri(receipt.root),
                    "added": receipt.added,
                    "joined": receipt.joined,
                    "added_documents": receipt.added_documents.into_iter().map(|document| json!({
                        "workspace": document.workspace().get(),
                        "document": document.get(),
                        "uri": document_uri(document)
                    })).collect::<Vec<_>>()
                }),
            );
        }
        OutcomeV1::WorkspaceChanges(changes) => {
            return tool_complete("workspace_changes", changes_json(changes));
        }
        OutcomeV1::WorkspaceInventoryPage(_) => return tool_error("service_malformed"),
        OutcomeV1::ViewOpened(receipt) => {
            return tool_complete(
                "view_opened",
                json!({
                    "kind": "view_opened",
                    "workspace": receipt.document.workspace().get(),
                    "document": receipt.document.get(),
                    "status": match receipt.status {
                        ViewOpenStatusV1::Opened => "opened",
                        ViewOpenStatusV1::AlreadyOpen => "already_open",
                    }
                }),
            );
        }
        OutcomeV1::ViewClosed(status) => {
            return tool_complete(
                "view_closed",
                json!({
                    "kind": "view_closed",
                    "status": match status {
                        ViewCloseStatusV1::Closed => "closed",
                        ViewCloseStatusV1::AlreadyClosed => "already_closed",
                    }
                }),
            );
        }
        OutcomeV1::DocumentRemoved(receipt) => {
            return tool_complete(
                "document_removed",
                json!({
                    "kind": "document_removed",
                    "workspace": receipt.document.workspace().get(),
                    "document": receipt.document.get(),
                    "status": membership_status(receipt.status)
                }),
            );
        }
        OutcomeV1::RootRemoved(receipt) => {
            return tool_complete(
                "root_removed",
                json!({
                    "kind": "root_removed",
                    "workspace": receipt.root.workspace().get(),
                    "root": receipt.root.get(),
                    "status": membership_status(receipt.status)
                }),
            );
        }
        OutcomeV1::DocumentSelected(document) => {
            return tool_complete(
                "document_selected",
                json!({
                    "kind": "document_selected",
                    "workspace": document.workspace().get(),
                    "document": document.get()
                }),
            );
        }
        OutcomeV1::Saved(_) => return tool_error("service_malformed"),
        OutcomeV1::ReplacedAndSaved(receipt) => {
            let structured = json!({
                "kind": "replaced_and_saved",
                "revision": receipt.snapshot.revision,
                "text": &*receipt.snapshot.text,
                "persistence": edit_persistence_json(receipt.persistence),
            });
            return with_server_meta(json!({
                "resultType": "complete",
                "content": [{"type": "text", "text": "replaced_and_saved"}],
                "structuredContent": structured,
                "isError": false
            }));
        }
        OutcomeV1::WorkspaceEdited(receipt) => {
            return tool_complete(
                "workspace_edited",
                json!({
                    "kind": "workspace_edited",
                    "workspace": receipt.workspace.get(),
                    "generation": receipt.generation,
                    "persistence": "not_attempted",
                    "documents": receipt.documents.into_iter().map(|document| json!({
                        "workspace": document.document.workspace().get(),
                        "document": document.document.get(),
                        "revision": document.snapshot.revision,
                        "text": &*document.snapshot.text,
                    })).collect::<Vec<_>>()
                }),
            );
        }
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

fn durable_execute_json(outcome: DurableExecuteOutcomeV1, intent: &DurableEffectIntentV1) -> Value {
    match outcome {
        DurableExecuteOutcomeV1::Completed(receipt) => {
            match durable_receipt_json(receipt, intent) {
                Some(receipt) => tool_complete("durable_complete", receipt),
                None => tool_error("service_malformed"),
            }
        }
        DurableExecuteOutcomeV1::Refused(refusal) => tool_error(refusal_code(refusal)),
        DurableExecuteOutcomeV1::Retired => tool_complete("retired", json!({"kind": "retired"})),
    }
}

fn durable_lookup_json(
    outcome: DurableReceiptLookupOutcomeV1,
    intent: &DurableEffectIntentV1,
) -> Value {
    match outcome {
        DurableReceiptLookupOutcomeV1::Complete(DurableExecuteOutcomeV1::Retired) => {
            tool_error("service_malformed")
        }
        DurableReceiptLookupOutcomeV1::Complete(outcome) => durable_execute_json(outcome, intent),
        DurableReceiptLookupOutcomeV1::RequestConflict => tool_error("request_conflict"),
        DurableReceiptLookupOutcomeV1::OutcomeUnknown => {
            tool_complete("outcome_unknown", json!({"kind": "outcome_unknown"}))
        }
        DurableReceiptLookupOutcomeV1::AbsentOrForeign => {
            tool_complete("absent_or_foreign", json!({"kind": "absent_or_foreign"}))
        }
        DurableReceiptLookupOutcomeV1::Unavailable => {
            tool_complete("unavailable", json!({"kind": "unavailable"}))
        }
    }
}

fn durable_retirement_json(outcome: DurableReceiptRetirementOutcomeV1) -> Value {
    match outcome {
        DurableReceiptRetirementOutcomeV1::Retired => {
            tool_complete("retired", json!({"kind": "retired"}))
        }
        DurableReceiptRetirementOutcomeV1::AbsentOrForeign => {
            tool_complete("absent_or_foreign", json!({"kind": "absent_or_foreign"}))
        }
        DurableReceiptRetirementOutcomeV1::RequestConflict => tool_error("request_conflict"),
        DurableReceiptRetirementOutcomeV1::OutcomeUnknown => {
            tool_complete("outcome_unknown", json!({"kind": "outcome_unknown"}))
        }
        DurableReceiptRetirementOutcomeV1::Unavailable => {
            tool_complete("unavailable", json!({"kind": "unavailable"}))
        }
    }
}

fn durable_status_json(status: DurableReceiptStatusV1) -> Value {
    let kind = match status {
        DurableReceiptStatusV1::Pending => "pending",
        DurableReceiptStatusV1::Complete => "complete",
        DurableReceiptStatusV1::Retired => "retired",
        DurableReceiptStatusV1::AbsentOrForeign => "absent_or_foreign",
        DurableReceiptStatusV1::Unavailable => "unavailable",
    };
    tool_complete(kind, json!({"kind": kind}))
}

fn durable_receipt_json(
    receipt: DurableEffectReceiptV1,
    intent: &DurableEffectIntentV1,
) -> Option<Value> {
    match (receipt, intent) {
        (DurableEffectReceiptV1::FileAdmitted(receipt), DurableEffectIntentV1::AddFile { .. }) => {
            Some(json!({
                "kind": "file_admission",
                "workspace": receipt.document.workspace().get(),
                "document": receipt.document.get(),
                "root": receipt.root.map(WorkspaceRootIdV1::get),
                "status": match receipt.status {
                    FileAdmissionStatusV1::Added => "added",
                    FileAdmissionStatusV1::AlreadyPresent => "already_present",
                }
            }))
        }
        (
            DurableEffectReceiptV1::DirectoryAdmitted(receipt),
            DurableEffectIntentV1::AddDirectory { .. },
        ) => Some(json!({
            "kind": "directory_admission",
            "workspace": receipt.root.workspace().get(),
            "root": receipt.root.get(),
            "added": receipt.added,
            "joined": receipt.joined,
            "added_documents": receipt.added_documents.into_iter().map(WorkspaceDocumentIdV1::get).collect::<Vec<_>>()
        })),
        (
            DurableEffectReceiptV1::DocumentRemoved(receipt),
            DurableEffectIntentV1::RemoveDocument { document },
        ) if receipt.document == *document => Some(json!({
            "kind": "document_removed",
            "workspace": receipt.document.workspace().get(),
            "document": receipt.document.get(),
            "status": membership_status(receipt.status),
        })),
        (
            DurableEffectReceiptV1::RootRemoved(receipt),
            DurableEffectIntentV1::RemoveRoot { root },
        ) if receipt.root == *root => Some(json!({
            "kind": "root_removed",
            "workspace": receipt.root.workspace().get(),
            "root": receipt.root.get(),
            "status": membership_status(receipt.status),
        })),
        (
            DurableEffectReceiptV1::DocumentSaved(receipt),
            DurableEffectIntentV1::SaveDocument {
                document,
                expected_revision,
            },
        ) if receipt.document == *document && receipt.source_revision == *expected_revision => {
            Some(json!({
                "kind": "document_saved",
                "workspace": receipt.document.workspace().get(),
                "document": receipt.document.get(),
                "source_revision": receipt.source_revision,
                "persistence": save_receipt_json("durable", receipt.persistence),
            }))
        }
        (
            DurableEffectReceiptV1::DocumentReplacedAndSaved(receipt),
            DurableEffectIntentV1::ReplaceDocumentAndSave {
                document,
                expected_revision,
                ..
            },
        ) if receipt.document == *document
            && expected_revision
                .checked_add(1)
                .is_some_and(|revision| receipt.source_revision == revision) =>
        {
            Some(json!({
                "kind": "document_replaced_and_saved",
                "workspace": receipt.document.workspace().get(),
                "document": receipt.document.get(),
                "source_revision": receipt.source_revision,
                "text": &*receipt.text,
                "persistence": edit_persistence_json(receipt.persistence),
            }))
        }
        (
            DurableEffectReceiptV1::DocumentCopyExportedExclusive(receipt),
            DurableEffectIntentV1::ExportDocumentCopyExclusive {
                document,
                expected_revision,
                ..
            },
        ) if receipt.document() == *document && receipt.source_revision() == *expected_revision => {
            Some(json!({
                "kind": "document_copy_exported_exclusive",
                "workspace": receipt.document().workspace().get(),
                "document": receipt.document().get(),
                "source_revision": receipt.source_revision(),
                "persistence": export_persistence_json(
                    receipt.persistence(),
                    receipt.failure(),
                    receipt.cleanup_failed(),
                ),
            }))
        }
        _ => None,
    }
}

fn export_persistence_json(
    persistence: DurableExportCopyPersistenceV1,
    failure: Option<DurableExportCopyFailureV1>,
    cleanup_failed: bool,
) -> Value {
    json!({
        "status": match persistence {
            DurableExportCopyPersistenceV1::Durable => "durable",
            DurableExportCopyPersistenceV1::NotDurable => "not_durable",
            DurableExportCopyPersistenceV1::Unknown => "unknown",
            DurableExportCopyPersistenceV1::NotAttempted => "not_attempted",
        },
        "failure": failure.map(|failure| match failure {
            DurableExportCopyFailureV1::Conflict => "conflict",
            DurableExportCopyFailureV1::Unsupported => "unsupported",
            DurableExportCopyFailureV1::Limit => "limit",
            DurableExportCopyFailureV1::Capacity => "capacity",
            DurableExportCopyFailureV1::Io => "io",
        }),
        "cleanup_failed": cleanup_failed,
    })
}

fn tool_complete(label: &'static str, structured: Value) -> Value {
    with_server_meta(json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": label}],
        "structuredContent": structured,
        "isError": false
    }))
}

fn membership_status(status: MembershipRemovalStatusV1) -> &'static str {
    match status {
        MembershipRemovalStatusV1::Removed => "removed",
        MembershipRemovalStatusV1::AlreadyRemoved => "already_removed",
    }
}

fn inventory_json(inventory: vellum_sdk_client::protocol::WorkspaceInventoryV1) -> Value {
    json!({
        "kind": "workspace_inventory",
        "workspace": inventory.workspace.get(),
        "documents": inventory.documents.into_iter().map(|document| json!({
            "workspace": document.workspace().get(),
            "document": document.get(),
            "uri": document_uri(document)
        })).collect::<Vec<_>>(),
        "roots": inventory.roots.into_iter().map(|root| json!({
            "workspace": root.id.workspace().get(),
            "root": root.id.get(),
            "kind": match root.kind {
                WorkspaceRootKindV1::File => "file",
                WorkspaceRootKindV1::Directory => "directory",
            },
            "uri": root_uri(root.id)
        })).collect::<Vec<_>>()
    })
}

fn changes_json(changes: WorkspaceChangesV1) -> Value {
    match changes {
        WorkspaceChangesV1::Page(page) => json!({
            "kind": "change_page",
            "workspace": page.workspace.get(),
            "observed_head": page.observed_head,
            "changes": page.changes.into_iter().map(|change| {
                let (kind, workspace, identity, source_revision) = match change.kind {
                    WorkspaceChangeKindV1::DocumentAdded(document) =>
                        ("document_added", document.workspace().get(), document.get(), None),
                    WorkspaceChangeKindV1::DocumentRemoved(document) =>
                        ("document_removed", document.workspace().get(), document.get(), None),
                    WorkspaceChangeKindV1::RootAdded(root) =>
                        ("root_added", root.workspace().get(), root.get(), None),
                    WorkspaceChangeKindV1::RootRemoved(root) =>
                        ("root_removed", root.workspace().get(), root.get(), None),
                    WorkspaceChangeKindV1::DocumentChanged { document, source_revision } =>
                        ("document_changed", document.workspace().get(), document.get(), Some(source_revision)),
                };
                json!({
                    "revision": change.revision(),
                    "kind": kind,
                    "workspace": workspace,
                    "identity": identity,
                    "source_revision": source_revision
                })
            }).collect::<Vec<_>>(),
            "next_cursor": encode_change_cursor(page.next)
        }),
        WorkspaceChangesV1::Gap(gap) => json!({
            "kind": "change_gap",
            "workspace": gap.workspace.get(),
            "requested_after": gap.requested_after,
            "oldest_available": gap.oldest_available,
            "observed_head": gap.observed_head,
            "recovery": "relist_resources_then_poll_without_cursor"
        }),
    }
}

fn edit_persistence_json(persistence: EditPersistenceV1) -> Value {
    match persistence {
        EditPersistenceV1::Durable(receipt) => save_receipt_json("durable", receipt),
        EditPersistenceV1::NotDurable(receipt) => save_receipt_json("not_durable", receipt),
        EditPersistenceV1::Unknown(receipt) => save_receipt_json("unknown", receipt),
        EditPersistenceV1::NotAttempted(refusal) => json!({
            "status": "not_attempted",
            "application": null,
            "synchronization": null,
            "no_future_effects": null,
            "quiescent": null,
            "adapter_failure": null,
            "cleanup_failed": null,
            "baseline_published": null,
            "outcome_unknown": null,
            "refusal": refusal_code(refusal),
        }),
    }
}

fn save_receipt_json(status: &'static str, receipt: SaveReceiptV1) -> Value {
    json!({
        "status": status,
        "application": match receipt.application {
            vellum_sdk_client::protocol::SaveApplicationV1::Unknown => "unknown",
            vellum_sdk_client::protocol::SaveApplicationV1::NotWritten => "not_written",
            vellum_sdk_client::protocol::SaveApplicationV1::Written => "written",
        },
        "synchronization": match receipt.synchronization {
            vellum_sdk_client::protocol::SaveSynchronizationV1::Unknown => "unknown",
            vellum_sdk_client::protocol::SaveSynchronizationV1::NotRequested => "not_requested",
            vellum_sdk_client::protocol::SaveSynchronizationV1::Succeeded => "succeeded",
            vellum_sdk_client::protocol::SaveSynchronizationV1::Failed => "failed",
        },
        "no_future_effects": receipt.no_future_effects,
        "quiescent": receipt.quiescent,
        "adapter_failure": receipt.adapter_failure,
        "cleanup_failed": receipt.cleanup_failed,
        "baseline_published": receipt.baseline_published,
        "outcome_unknown": receipt.outcome_unknown,
        "refusal": null,
    })
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

fn tool_error_with_operation(code: &'static str, operation: u64) -> Value {
    let structured = json!({"code": code, "operation": encode_operation(operation)});
    with_server_meta(json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": code}],
        "structuredContent": structured,
        "isError": true
    }))
}

fn refusal_code(refusal: RefusalV1) -> &'static str {
    match refusal {
        RefusalV1::AuthorityDenied => "authority_denied",
        RefusalV1::AuthorityUnavailable => "authority_unavailable",
        RefusalV1::UnknownClient => "unknown_client",
        RefusalV1::ClientCapacity => "client_capacity",
        RefusalV1::ClientLifetimeCapacity => "client_lifetime_capacity",
        RefusalV1::RequestCapacity => "request_capacity",
        RefusalV1::RequestConflict => "request_conflict",
        RefusalV1::WitnessCapacity => "witness_capacity",
        RefusalV1::UnknownWitness => "unknown_witness",
        RefusalV1::UnknownDocument => "unknown_document",
        RefusalV1::UnknownRoot => "unknown_root",
        RefusalV1::DocumentInUse => "document_in_use",
        RefusalV1::ForeignInventoryCursor => "foreign_inventory_cursor",
        RefusalV1::StaleInventoryCursor => "stale_inventory_cursor",
        RefusalV1::InvalidInventoryCursor => "invalid_inventory_cursor",
        RefusalV1::ForeignChangeCursor => "foreign_change_cursor",
        RefusalV1::InvalidChangeCursor => "invalid_change_cursor",
        RefusalV1::NoActiveDocument => "no_active_document",
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
        ServiceAccessErrorV1::NotSent => "service_not_sent",
        ServiceAccessErrorV1::OutcomeUnknown => "service_outcome_unknown",
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

#[cfg(test)]
mod durable_projection_tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use vellum_sdk_client::protocol::{
        ClientId, DurableExecuteRequestV1, DurableReceiptLookupV1, DurableReceiptRetirementV1,
        DurableReceiptStatusRequestV1, RequestV1, ServiceControlErrorV1,
    };

    #[derive(Default)]
    struct Calls {
        executes: usize,
        lookups: usize,
        retirements: usize,
        statuses: usize,
        execute_intents: Vec<DurableEffectIntentV1>,
        lookup_intents: Vec<DurableEffectIntentV1>,
        retirement_operations: Vec<DurableOperationRefV1>,
        retirement_intents: Vec<DurableEffectIntentV1>,
    }

    struct Port {
        calls: Arc<Mutex<Calls>>,
        execute: Result<DurableExecuteOutcomeV1, ServiceAccessErrorV1>,
        lookup: Result<DurableReceiptLookupOutcomeV1, ServiceAccessErrorV1>,
        retirement: Result<DurableReceiptRetirementOutcomeV1, ServiceAccessErrorV1>,
        status: Result<DurableReceiptStatusV1, ServiceAccessErrorV1>,
    }

    impl ServicePort for Port {
        fn connect(&mut self) -> Result<ClientId, ServiceControlErrorV1> {
            Ok(ClientId::new(1).unwrap())
        }

        fn execute(&mut self, _request: RequestV1) -> Result<ReplyV1, ServiceAccessErrorV1> {
            Err(ServiceAccessErrorV1::Unavailable)
        }

        fn execute_durable(
            &mut self,
            request: DurableExecuteRequestV1,
        ) -> Result<DurableExecuteOutcomeV1, ServiceAccessErrorV1> {
            let mut calls = self.calls.lock().unwrap();
            calls.executes += 1;
            calls.execute_intents.push(request.intent);
            self.execute.clone()
        }

        fn lookup_durable_receipt(
            &mut self,
            lookup: DurableReceiptLookupV1,
        ) -> Result<DurableReceiptLookupOutcomeV1, ServiceAccessErrorV1> {
            let mut calls = self.calls.lock().unwrap();
            calls.lookups += 1;
            calls.lookup_intents.push(lookup.intent);
            self.lookup.clone()
        }

        fn retire_durable_receipt(
            &mut self,
            retirement: DurableReceiptRetirementV1,
        ) -> Result<DurableReceiptRetirementOutcomeV1, ServiceAccessErrorV1> {
            let mut calls = self.calls.lock().unwrap();
            calls.retirements += 1;
            calls.retirement_operations.push(retirement.operation);
            calls.retirement_intents.push(retirement.intent);
            self.retirement
        }

        fn durable_receipt_status(
            &mut self,
            _request: DurableReceiptStatusRequestV1,
        ) -> Result<DurableReceiptStatusV1, ServiceAccessErrorV1> {
            self.calls.lock().unwrap().statuses += 1;
            self.status
        }

        fn disconnect(&mut self, _client: ClientId) -> Result<(), ServiceControlErrorV1> {
            Ok(())
        }
    }

    fn meta() -> Value {
        json!({
            "io.modelcontextprotocol/protocolVersion": PROTOCOL_VERSION,
            "io.modelcontextprotocol/clientCapabilities": {}
        })
    }

    fn request(name: &str, arguments: Value) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {"_meta": meta(), "name": name, "arguments": arguments}
        }))
        .unwrap()
    }

    fn server(
        execute: Result<DurableExecuteOutcomeV1, ServiceAccessErrorV1>,
        lookup: Result<DurableReceiptLookupOutcomeV1, ServiceAccessErrorV1>,
    ) -> (McpServer<Port>, Arc<Mutex<Calls>>) {
        let calls = Arc::new(Mutex::new(Calls::default()));
        let client = Client::connect(Port {
            calls: calls.clone(),
            execute,
            lookup,
            retirement: Ok(DurableReceiptRetirementOutcomeV1::Unavailable),
            status: Ok(DurableReceiptStatusV1::Unavailable),
        })
        .unwrap();
        (
            McpServer::build(client, McpLimits::reference(), true).unwrap(),
            calls,
        )
    }

    fn server_with_retirement(
        retirement: Result<DurableReceiptRetirementOutcomeV1, ServiceAccessErrorV1>,
    ) -> (McpServer<Port>, Arc<Mutex<Calls>>) {
        let calls = Arc::new(Mutex::new(Calls::default()));
        let client = Client::connect(Port {
            calls: calls.clone(),
            execute: Err(ServiceAccessErrorV1::Unavailable),
            lookup: Ok(DurableReceiptLookupOutcomeV1::Unavailable),
            retirement,
            status: Ok(DurableReceiptStatusV1::Unavailable),
        })
        .unwrap();
        (
            McpServer::build(client, McpLimits::reference(), true).unwrap(),
            calls,
        )
    }

    fn server_with_status(
        status: Result<DurableReceiptStatusV1, ServiceAccessErrorV1>,
    ) -> (McpServer<Port>, Arc<Mutex<Calls>>) {
        let calls = Arc::new(Mutex::new(Calls::default()));
        let client = Client::connect(Port {
            calls: calls.clone(),
            execute: Err(ServiceAccessErrorV1::Unavailable),
            lookup: Ok(DurableReceiptLookupOutcomeV1::Unavailable),
            retirement: Ok(DurableReceiptRetirementOutcomeV1::Unavailable),
            status,
        })
        .unwrap();
        (
            McpServer::build(client, McpLimits::reference(), true).unwrap(),
            calls,
        )
    }

    fn add_file(operation_ref: &str, locator: &str) -> Value {
        json!({
            "operation_ref": operation_ref,
            "intent": "add_file",
            "locator": locator
        })
    }

    fn export_copy(operation_ref: &str, destination: &str) -> Value {
        json!({
            "operation_ref": operation_ref,
            "intent": "export_document_copy_exclusive",
            "workspace": 1,
            "document": 1,
            "expected_revision": 7,
            "destination": destination,
        })
    }

    #[test]
    fn durable_status_projects_only_closed_state_without_intent_or_authority() {
        let operation = "vdo1.11111111111111111111111111111111";
        for (status, expected) in [
            (DurableReceiptStatusV1::Pending, "pending"),
            (DurableReceiptStatusV1::Complete, "complete"),
            (DurableReceiptStatusV1::Retired, "retired"),
            (DurableReceiptStatusV1::AbsentOrForeign, "absent_or_foreign"),
            (DurableReceiptStatusV1::Unavailable, "unavailable"),
        ] {
            let (mut server, calls) = server_with_status(Ok(status));
            let reply: Value = serde_json::from_slice(
                &server
                    .handle(&request(
                        DURABLE_STATUS_TOOL,
                        json!({"operation_ref": operation}),
                    ))
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(reply["result"]["structuredContent"]["kind"], expected);
            assert_eq!(calls.lock().unwrap().statuses, 1);
            assert_eq!(calls.lock().unwrap().executes, 0);
            assert_eq!(calls.lock().unwrap().lookups, 0);
            assert_eq!(calls.lock().unwrap().retirements, 0);
        }
        let (mut server, calls) = server_with_status(Ok(DurableReceiptStatusV1::Complete));
        let rejected: Value = serde_json::from_slice(
            &server
                .handle(&request(
                    DURABLE_STATUS_TOOL,
                    json!({"operation_ref": operation, "intent": "add_file"}),
                ))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(rejected["error"]["code"], -32602);
        assert_eq!(calls.lock().unwrap().statuses, 0);
    }

    #[test]
    fn ordinary_composition_neither_advertises_nor_accepts_durable_tools() {
        let calls = Arc::new(Mutex::new(Calls::default()));
        let client = Client::connect(Port {
            calls: calls.clone(),
            execute: Err(ServiceAccessErrorV1::Unavailable),
            lookup: Ok(DurableReceiptLookupOutcomeV1::AbsentOrForeign),
            retirement: Ok(DurableReceiptRetirementOutcomeV1::Unavailable),
            status: Ok(DurableReceiptStatusV1::Unavailable),
        })
        .unwrap();
        let mut server = McpServer::new(client, McpLimits::reference()).unwrap();
        let list = serde_json::to_vec(&json!({
            "jsonrpc":"2.0", "id":1, "method":"tools/list", "params":{"_meta":meta()}
        }))
        .unwrap();
        let reply: Value = serde_json::from_slice(&server.handle(&list).unwrap()).unwrap();
        assert_eq!(reply["result"]["tools"].as_array().unwrap().len(), 7);
        for tool in [
            DURABLE_EXECUTE_TOOL,
            DURABLE_RECEIPT_TOOL,
            DURABLE_RETIRE_TOOL,
            DURABLE_STATUS_TOOL,
        ] {
            let rejected: Value = serde_json::from_slice(
                &server
                    .handle(&request(
                        tool,
                        add_file("vdo1.11111111111111111111111111111111", "/a"),
                    ))
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(rejected["error"]["code"], -32602);
        }
        assert_eq!(calls.lock().unwrap().executes, 0);
        assert_eq!(calls.lock().unwrap().lookups, 0);
        assert_eq!(calls.lock().unwrap().retirements, 0);
    }

    #[test]
    fn lookup_absence_is_nondisclosing_and_never_falls_through_to_execute() {
        let (mut server, calls) = server(
            Err(ServiceAccessErrorV1::Unavailable),
            Ok(DurableReceiptLookupOutcomeV1::AbsentOrForeign),
        );
        let reply: Value = serde_json::from_slice(
            &server
                .handle(&request(
                    DURABLE_RECEIPT_TOOL,
                    add_file("vdo1.11111111111111111111111111111111", "/a"),
                ))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            reply["result"]["structuredContent"]["kind"],
            "absent_or_foreign"
        );
        assert_eq!(calls.lock().unwrap().lookups, 1);
        assert_eq!(calls.lock().unwrap().executes, 0);
        assert_eq!(server.usage().durable_absent_or_foreign, 1);
    }

    #[test]
    fn malformed_or_zero_operation_reference_reaches_no_native_port() {
        let (mut server, calls) = server(
            Err(ServiceAccessErrorV1::Unavailable),
            Ok(DurableReceiptLookupOutcomeV1::AbsentOrForeign),
        );
        for operation in [
            "vdo1.00000000000000000000000000000000",
            "vdo1.1111111111111111111111111111111g",
            "vo1.1111111111111111",
        ] {
            for tool in [
                DURABLE_EXECUTE_TOOL,
                DURABLE_RECEIPT_TOOL,
                DURABLE_RETIRE_TOOL,
                DURABLE_STATUS_TOOL,
            ] {
                let reply: Value = serde_json::from_slice(
                    &server
                        .handle(&request(tool, add_file(operation, "/a")))
                        .unwrap(),
                )
                .unwrap();
                assert_eq!(reply["error"]["code"], -32602);
            }
        }
        assert_eq!(calls.lock().unwrap().executes, 0);
        assert_eq!(calls.lock().unwrap().lookups, 0);
        assert_eq!(calls.lock().unwrap().retirements, 0);
    }

    #[test]
    fn durable_discovery_describes_the_exact_closed_algebra() {
        let catalog = tools_result(true);
        let tools = catalog["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 11);
        let execute = tools
            .iter()
            .find(|tool| tool["name"] == DURABLE_EXECUTE_TOOL)
            .unwrap();
        let lookup = tools
            .iter()
            .find(|tool| tool["name"] == DURABLE_RECEIPT_TOOL)
            .unwrap();
        let retire = tools
            .iter()
            .find(|tool| tool["name"] == DURABLE_RETIRE_TOOL)
            .unwrap();
        let status = tools
            .iter()
            .find(|tool| tool["name"] == DURABLE_STATUS_TOOL)
            .unwrap();
        assert_eq!(status["annotations"]["readOnlyHint"], true);
        assert_eq!(status["annotations"]["destructiveHint"], false);

        let intents = execute["inputSchema"]["oneOf"].as_array().unwrap();
        assert_eq!(intents.len(), 7);
        for intent in intents {
            assert_eq!(intent["additionalProperties"], false);
            assert!(intent["required"].as_array().unwrap().len() >= 3);
            assert_eq!(
                intent["properties"]["operation_ref"]["pattern"],
                "^vdo1[.](?!0{32}$)[0-9a-f]{32}$"
            );
        }
        assert_eq!(
            intents[0]["required"],
            json!(["operation_ref", "intent", "locator"])
        );
        assert_eq!(intents[0]["properties"]["intent"]["const"], "add_file");
        assert_eq!(
            intents[5]["required"],
            json!([
                "operation_ref",
                "intent",
                "workspace",
                "document",
                "expected_revision",
                "start",
                "end",
                "inserted"
            ])
        );
        assert_eq!(
            intents[6]["required"],
            json!([
                "operation_ref",
                "intent",
                "workspace",
                "document",
                "expected_revision",
                "destination"
            ])
        );
        assert_eq!(
            intents[6]["properties"]["intent"]["const"],
            "export_document_copy_exclusive"
        );
        assert_eq!(intents[6]["properties"]["destination"]["minLength"], 1);
        assert_eq!(retire["inputSchema"], execute["inputSchema"]);
        assert_eq!(retire["annotations"]["readOnlyHint"], false);
        assert_eq!(retire["annotations"]["destructiveHint"], true);
        assert_eq!(retire["annotations"]["idempotentHint"], true);
        let retirement_outputs = retire["outputSchema"]["oneOf"].as_array().unwrap();
        assert_eq!(retirement_outputs.len(), 5);
        assert_eq!(
            retirement_outputs
                .iter()
                .filter_map(|variant| variant["properties"]["kind"]["const"].as_str())
                .collect::<Vec<_>>(),
            vec![
                "retired",
                "absent_or_foreign",
                "outcome_unknown",
                "unavailable"
            ]
        );
        assert!(retirement_outputs.iter().any(|variant| {
            variant["properties"]["code"]["enum"]
                .as_array()
                .is_some_and(|codes| {
                    codes
                        .iter()
                        .any(|code| code.as_str() == Some("request_conflict"))
                })
        }));
        assert!(
            retirement_outputs
                .iter()
                .all(|variant| variant["additionalProperties"] == false)
        );

        let execute_outputs = execute["outputSchema"]["oneOf"].as_array().unwrap();
        let lookup_outputs = lookup["outputSchema"]["oneOf"].as_array().unwrap();
        assert_eq!(execute_outputs.len(), 9);
        assert_eq!(lookup_outputs.len(), 11);
        assert!(execute_outputs.iter().any(|variant| {
            variant["required"] == json!(["kind"])
                && variant["properties"]["kind"]["const"] == "retired"
        }));
        assert!(
            !lookup_outputs
                .iter()
                .any(|variant| { variant["properties"]["kind"]["const"] == "retired" })
        );
        assert!(
            execute_outputs
                .iter()
                .all(|variant| variant["additionalProperties"] == false)
        );
        assert!(
            lookup_outputs
                .iter()
                .all(|variant| variant["additionalProperties"] == false)
        );
        assert_eq!(
            execute_outputs[4]["properties"]["persistence"]["additionalProperties"],
            false
        );
        let export = execute_outputs
            .iter()
            .find(|variant| {
                variant["properties"]["kind"]["const"] == "document_copy_exported_exclusive"
            })
            .unwrap();
        let export_persistence = export["properties"]["persistence"]["oneOf"]
            .as_array()
            .unwrap();
        assert_eq!(export_persistence.len(), 2);
        assert_eq!(
            export_persistence[0]["properties"]["status"]["const"],
            "durable"
        );
        assert_eq!(
            export_persistence[0]["properties"]["cleanup_failed"]["const"],
            false
        );
        assert_eq!(
            export_persistence[1]["properties"]["status"]["enum"],
            json!(["not_durable", "unknown", "not_attempted"])
        );
        assert_eq!(
            export_persistence[1]["properties"]["failure"]["enum"],
            json!(["conflict", "unsupported", "limit", "capacity", "io"])
        );
        assert!(
            export_persistence
                .iter()
                .all(|variant| variant["additionalProperties"] == false)
        );
    }

    #[test]
    fn export_receipt_projection_is_exact_bound_and_locator_free() {
        use vellum_sdk_client::protocol::{
            DurableExportCopyReceiptV1, WorkspaceDocumentIdV1, WorkspaceIdV1,
        };

        let workspace = WorkspaceIdV1::new(1).unwrap();
        let document = WorkspaceDocumentIdV1::new(workspace, 1).unwrap();
        let intent = DurableEffectIntentV1::ExportDocumentCopyExclusive {
            document,
            expected_revision: 7,
            destination: "/private/export.txt".to_owned(),
        };
        let receipt = || {
            DurableExportCopyReceiptV1::try_new(
                document,
                7,
                DurableExportCopyPersistenceV1::Durable,
                None,
                false,
            )
            .unwrap()
        };
        let projected = durable_execute_json(
            DurableExecuteOutcomeV1::Completed(
                DurableEffectReceiptV1::DocumentCopyExportedExclusive(receipt()),
            ),
            &intent,
        );

        assert_eq!(
            projected["structuredContent"]["kind"],
            "document_copy_exported_exclusive"
        );
        assert_eq!(projected["structuredContent"]["workspace"], 1);
        assert_eq!(projected["structuredContent"]["document"], 1);
        assert_eq!(projected["structuredContent"]["source_revision"], 7);
        assert_eq!(
            projected["structuredContent"]["persistence"],
            json!({"status":"durable", "failure":null, "cleanup_failed":false})
        );
        assert!(!projected.to_string().contains("/private/export.txt"));

        for mismatched in [
            DurableEffectIntentV1::ExportDocumentCopyExclusive {
                document,
                expected_revision: 8,
                destination: "/private/export.txt".to_owned(),
            },
            DurableEffectIntentV1::ExportDocumentCopyExclusive {
                document: WorkspaceDocumentIdV1::new(workspace, 2).unwrap(),
                expected_revision: 7,
                destination: "/private/export.txt".to_owned(),
            },
            DurableEffectIntentV1::AddFile {
                locator: "/private/export.txt".to_owned(),
            },
        ] {
            let malformed = durable_execute_json(
                DurableExecuteOutcomeV1::Completed(
                    DurableEffectReceiptV1::DocumentCopyExportedExclusive(receipt()),
                ),
                &mismatched,
            );
            assert_eq!(malformed["structuredContent"]["code"], "service_malformed");
            assert_eq!(malformed["isError"], true);
            assert!(!malformed.to_string().contains("/private/export.txt"));
        }
    }

    #[test]
    fn export_execute_and_lookup_preserve_the_exact_destination_only_in_native_intent() {
        use vellum_sdk_client::protocol::{
            DurableExportCopyReceiptV1, WorkspaceDocumentIdV1, WorkspaceIdV1,
        };

        let document = WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(1).unwrap(), 1).unwrap();
        let receipt = DurableEffectReceiptV1::DocumentCopyExportedExclusive(
            DurableExportCopyReceiptV1::try_new(
                document,
                7,
                DurableExportCopyPersistenceV1::Durable,
                None,
                false,
            )
            .unwrap(),
        );
        let completed = DurableExecuteOutcomeV1::Completed(receipt);
        let (mut server, calls) = server(
            Ok(completed.clone()),
            Ok(DurableReceiptLookupOutcomeV1::Complete(completed)),
        );
        let arguments = export_copy(
            "vdo1.11111111111111111111111111111111",
            "/private/exact-export.txt",
        );
        let execute: Value = serde_json::from_slice(
            &server
                .handle(&request(DURABLE_EXECUTE_TOOL, arguments.clone()))
                .unwrap(),
        )
        .unwrap();
        let lookup: Value = serde_json::from_slice(
            &server
                .handle(&request(DURABLE_RECEIPT_TOOL, arguments))
                .unwrap(),
        )
        .unwrap();
        for reply in [execute, lookup] {
            assert_eq!(
                reply["result"]["structuredContent"]["kind"],
                "document_copy_exported_exclusive"
            );
            assert!(!reply.to_string().contains("/private/exact-export.txt"));
        }
        let calls = calls.lock().unwrap();
        let expected = DurableEffectIntentV1::ExportDocumentCopyExclusive {
            document,
            expected_revision: 7,
            destination: "/private/exact-export.txt".to_owned(),
        };
        assert_eq!(calls.execute_intents, vec![expected.clone()]);
        assert_eq!(calls.lookup_intents, vec![expected]);
    }

    #[test]
    fn malformed_export_fields_reach_neither_durable_native_port() {
        let (mut server, calls) = server(
            Err(ServiceAccessErrorV1::Unavailable),
            Ok(DurableReceiptLookupOutcomeV1::AbsentOrForeign),
        );
        let valid = export_copy(
            "vdo1.11111111111111111111111111111111",
            "/private/exact-export.txt",
        );
        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove("destination");
        let mut empty = valid.clone();
        empty["destination"] = json!("");
        let mut extra = valid;
        extra["unexpected"] = json!(true);
        for arguments in [missing, empty, extra] {
            for tool in [
                DURABLE_EXECUTE_TOOL,
                DURABLE_RECEIPT_TOOL,
                DURABLE_RETIRE_TOOL,
            ] {
                let reply: Value = serde_json::from_slice(
                    &server.handle(&request(tool, arguments.clone())).unwrap(),
                )
                .unwrap();
                assert_eq!(reply["error"]["code"], -32602);
            }
        }
        let calls = calls.lock().unwrap();
        assert_eq!(calls.executes, 0);
        assert_eq!(calls.lookups, 0);
        assert_eq!(calls.retirements, 0);
    }

    #[test]
    fn retirement_outcomes_are_closed_content_free_and_never_execute_or_lookup() {
        let cases = [
            (DurableReceiptRetirementOutcomeV1::Retired, "retired", false),
            (
                DurableReceiptRetirementOutcomeV1::AbsentOrForeign,
                "absent_or_foreign",
                false,
            ),
            (
                DurableReceiptRetirementOutcomeV1::RequestConflict,
                "request_conflict",
                true,
            ),
            (
                DurableReceiptRetirementOutcomeV1::OutcomeUnknown,
                "outcome_unknown",
                false,
            ),
            (
                DurableReceiptRetirementOutcomeV1::Unavailable,
                "unavailable",
                false,
            ),
        ];
        for (native, projected, is_error) in cases {
            let (mut server, calls) = server_with_retirement(Ok(native));
            let reply: Value = serde_json::from_slice(
                &server
                    .handle(&request(
                        DURABLE_RETIRE_TOOL,
                        add_file(
                            "vdo1.11111111111111111111111111111111",
                            "/private/retire-me",
                        ),
                    ))
                    .unwrap(),
            )
            .unwrap();
            let structured = &reply["result"]["structuredContent"];
            if is_error {
                assert_eq!(structured["code"], projected);
            } else {
                assert_eq!(structured["kind"], projected);
            }
            assert_eq!(reply["result"]["isError"], is_error);
            assert!(!reply.to_string().contains("/private/retire-me"));
            assert!(
                !reply
                    .to_string()
                    .contains("vdo1.11111111111111111111111111111111")
            );
            let calls = calls.lock().unwrap();
            assert_eq!(calls.retirements, 1);
            assert_eq!(calls.executes, 0);
            assert_eq!(calls.lookups, 0);
            assert_eq!(
                calls.retirement_operations,
                vec![DurableOperationRefV1::new([0x11; 16]).unwrap()]
            );
            assert_eq!(
                calls.retirement_intents,
                vec![DurableEffectIntentV1::AddFile {
                    locator: "/private/retire-me".to_owned(),
                }]
            );
        }
    }

    #[test]
    fn retirement_metrics_classify_every_native_and_access_outcome() {
        for (native, projection) in [
            (
                DurableReceiptRetirementOutcomeV1::Retired,
                [1, 0, 0, 0, 0, 0],
            ),
            (
                DurableReceiptRetirementOutcomeV1::RequestConflict,
                [0, 1, 0, 0, 0, 1],
            ),
            (
                DurableReceiptRetirementOutcomeV1::OutcomeUnknown,
                [0, 0, 1, 0, 0, 0],
            ),
            (
                DurableReceiptRetirementOutcomeV1::AbsentOrForeign,
                [0, 0, 0, 1, 0, 0],
            ),
            (
                DurableReceiptRetirementOutcomeV1::Unavailable,
                [0, 0, 0, 0, 1, 0],
            ),
        ] {
            let (mut server, _) = server_with_retirement(Ok(native));
            let _ = server
                .handle(&request(
                    DURABLE_RETIRE_TOOL,
                    add_file("vdo1.11111111111111111111111111111111", "/a"),
                ))
                .unwrap();
            let usage = server.usage();
            assert_eq!(usage.durable_retirements, 1);
            assert_eq!(
                [
                    usage.retirement_retired,
                    usage.retirement_conflict,
                    usage.retirement_outcome_unknown,
                    usage.retirement_absent_or_foreign,
                    usage.retirement_unavailable,
                    usage.tool_errors,
                ],
                projection
            );
        }

        for (error, unknown, unavailable) in [
            (ServiceAccessErrorV1::NotSent, 0, 0),
            (ServiceAccessErrorV1::OutcomeUnknown, 1, 0),
            (ServiceAccessErrorV1::Unavailable, 0, 1),
            (ServiceAccessErrorV1::Malformed, 0, 0),
            (ServiceAccessErrorV1::Limit, 0, 0),
        ] {
            let (mut server, _) = server_with_retirement(Err(error));
            let _ = server
                .handle(&request(
                    DURABLE_RETIRE_TOOL,
                    add_file("vdo1.22222222222222222222222222222222", "/a"),
                ))
                .unwrap();
            let usage = server.usage();
            assert_eq!(usage.durable_retirements, 1);
            assert_eq!(usage.retirement_outcome_unknown, unknown);
            assert_eq!(usage.retirement_unavailable, unavailable);
            assert_eq!(usage.tool_errors, 1);
        }
    }

    #[test]
    fn durable_metrics_classify_execute_and_lookup_errors_exhaustively() {
        let arguments = add_file("vdo1.11111111111111111111111111111111", "/a");

        let (mut uncertain, _) = server(
            Err(ServiceAccessErrorV1::OutcomeUnknown),
            Ok(DurableReceiptLookupOutcomeV1::AbsentOrForeign),
        );
        let _ = uncertain
            .handle(&request(DURABLE_EXECUTE_TOOL, arguments.clone()))
            .unwrap();
        assert_eq!(uncertain.usage().durable_executes, 1);
        assert_eq!(uncertain.usage().durable_outcome_unknown, 1);
        assert_eq!(uncertain.usage().durable_complete, 0);
        assert_eq!(uncertain.usage().tool_errors, 1);

        let (mut refused, _) = server(
            Ok(DurableExecuteOutcomeV1::Refused(RefusalV1::Stale)),
            Ok(DurableReceiptLookupOutcomeV1::RequestConflict),
        );
        let _ = refused
            .handle(&request(DURABLE_EXECUTE_TOOL, arguments.clone()))
            .unwrap();
        let _ = refused
            .handle(&request(DURABLE_RECEIPT_TOOL, arguments))
            .unwrap();
        assert_eq!(refused.usage().durable_complete, 1);
        assert_eq!(refused.usage().durable_conflict, 1);
        assert_eq!(refused.usage().tool_errors, 2);

        let (mut execute_conflict, _) = server(
            Ok(DurableExecuteOutcomeV1::Refused(RefusalV1::RequestConflict)),
            Ok(DurableReceiptLookupOutcomeV1::AbsentOrForeign),
        );
        let _ = execute_conflict
            .handle(&request(
                DURABLE_EXECUTE_TOOL,
                add_file("vdo1.33333333333333333333333333333333", "/c"),
            ))
            .unwrap();
        assert_eq!(execute_conflict.usage().durable_complete, 1);
        assert_eq!(execute_conflict.usage().durable_conflict, 1);
        assert_eq!(execute_conflict.usage().tool_errors, 1);

        let (mut unavailable, _) = server(
            Err(ServiceAccessErrorV1::Unavailable),
            Err(ServiceAccessErrorV1::Unavailable),
        );
        let _ = unavailable
            .handle(&request(
                DURABLE_RECEIPT_TOOL,
                add_file("vdo1.22222222222222222222222222222222", "/b"),
            ))
            .unwrap();
        assert_eq!(unavailable.usage().durable_unavailable, 1);
        assert_eq!(unavailable.usage().tool_errors, 1);
    }

    #[test]
    fn lookup_conflict_and_pending_are_closed_and_content_free() {
        for (native, code, counter) in [
            (
                DurableReceiptLookupOutcomeV1::RequestConflict,
                "request_conflict",
                0,
            ),
            (
                DurableReceiptLookupOutcomeV1::OutcomeUnknown,
                "outcome_unknown",
                1,
            ),
        ] {
            let (mut server, _) = server(Err(ServiceAccessErrorV1::Unavailable), Ok(native));
            let reply: Value = serde_json::from_slice(
                &server
                    .handle(&request(
                        DURABLE_RECEIPT_TOOL,
                        add_file("vdo1.11111111111111111111111111111111", "/private/a"),
                    ))
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(
                reply["result"]["structuredContent"]["code"]
                    .as_str()
                    .unwrap_or(code),
                code
            );
            assert!(!reply.to_string().contains("/private/a"));
            assert_eq!(server.usage().durable_outcome_unknown, counter);
        }
    }
}
