//! Bounded, no-process-authority Language Server Protocol substrate.
//!
//! This crate owns LSP framing, session state, virtual-document synchronization
//! and position conversion. It never discovers or launches an executable,
//! reads a workspace path, opens a network connection, applies an edit, or
//! grants semantic authority. A composition root supplies an already-owned
//! byte transport and non-reusing semantic generation issuer.
#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
};

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value, json};
use vellum_host::semantic::{
    SemanticCapabilitySetV1, SemanticCapabilityV1, SemanticDiagnosticSeverityV1,
    SemanticDiagnosticV1, SemanticDocumentRefV1, SemanticIndexGenerationV1, SemanticLimitsV1,
    SemanticLocationV1, SemanticOutcomeV1, SemanticProviderProvenanceV1, SemanticRefusalV1,
    SemanticReplyV1, SemanticRequestRefV1, SemanticRequestV1, SemanticResultV1, SemanticSpanV1,
    SemanticSymbolKindV1, SemanticSymbolV1, SemanticTargetV1, SemanticTextEditV1, SemanticTextV1,
};
use vellum_semantic_service::{
    SemanticBridgePollFailureV1, SemanticBridgeProviderPollV1, SemanticProviderAdmissionV1,
    SemanticProviderCompletionV1, SemanticProviderPortV1, SemanticProviderReadinessV1,
    SemanticProviderStateV1, SemanticProviderSubmissionV1, SemanticSnapshotUpdateV1,
};

/// Maximum canonical JSON bytes retained while carrying one provider-owned
/// call-hierarchy item from prepare into the direction query.
pub const MAX_LSP_CALL_HIERARCHY_ITEM_BYTES_V1: usize = 64 * 1024;

/// JSON value constructed only after recursively proving that every object key
/// occurs exactly once. `serde_json::Value` alone uses last-key-wins semantics,
/// which would erase ambiguity before the protocol's exact-member checks.
struct StrictJsonValue(Value);

impl<'de> Deserialize<'de> for StrictJsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictJsonVisitor)
    }
}

struct StrictJsonVisitor;

impl<'de> Visitor<'de> for StrictJsonVisitor {
    type Value = StrictJsonValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
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

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .map(StrictJsonValue)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
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

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(StrictJsonValue(value)) = sequence.next_element::<StrictJsonValue>()? {
            values.push(value);
        }
        Ok(StrictJsonValue(Value::Array(values)))
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
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

pub const MAX_LSP_HEADER_BYTES_V1: usize = 8 * 1024;
pub const MAX_LSP_HEADERS_V1: usize = 16;
pub const MAX_LSP_BODY_BYTES_V1: usize = 32 * 1024 * 1024;
pub const MAX_LSP_BUFFER_BYTES_V1: usize = MAX_LSP_HEADER_BYTES_V1 + MAX_LSP_BODY_BYTES_V1;
pub const MAX_LSP_DOCUMENTS_V1: usize = 4_096;
pub const MAX_LSP_URI_BYTES_V1: usize = 512;
pub const MAX_LSP_WORKSPACE_NAME_BYTES_V1: usize = 128;
pub const MAX_LSP_METHOD_BYTES_V1: usize = 128;
pub const MAX_LSP_SERVER_INFO_NAME_BYTES_V1: usize = 128;
pub const MAX_LSP_SERVER_INFO_VERSION_BYTES_V1: usize = 128;
pub const MAX_LSP_READINESS_MESSAGE_BYTES_V1: usize = 1_024;
pub const MAX_LSP_PENDING_REQUESTS_V1: usize = 4_096;
pub const MAX_LSP_TOTAL_SOURCE_BYTES_V1: usize = 64 * 1024 * 1024;

/// Closed initialization facts. Client capabilities remain adapter-owned;
/// callers may bind only one syntactically canonical file root/workspace and
/// one audited readiness-notification method. This is not filesystem authority.
#[derive(Clone, Eq, PartialEq)]
pub struct LspInitializationProfileV1 {
    workspace: Option<LspWorkspaceFolderV1>,
    readiness_notification_method: Option<String>,
    protocol: Option<LspProviderProtocolProfileV1>,
    provider_options: Option<LspProviderInitializationOptionsV1>,
}

#[derive(Clone, Eq, PartialEq)]
struct LspWorkspaceFolderV1 {
    uri: String,
    name: String,
}

/// Closed provider-specific initialization policies. No variant accepts raw
/// JSON; every payload is adapter-owned and serialized from bounded facts.
#[derive(Clone, Eq, PartialEq)]
pub enum LspProviderInitializationOptionsV1 {
    RustAnalyzerStandalone(LspRustAnalyzerStandaloneInitializationV1),
}

/// Pre-effect binding between the qualified workspace root and rust-analyzer's
/// single standalone package document (`src/lib.rs`).
#[derive(Clone, Eq, PartialEq)]
pub struct LspRustAnalyzerStandaloneInitializationV1 {
    linked_project_path: String,
}

impl fmt::Debug for LspProviderInitializationOptionsV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RustAnalyzerStandalone(_) => formatter
                .write_str("LspProviderInitializationOptionsV1::RustAnalyzerStandalone(<bound>)"),
        }
    }
}

impl fmt::Debug for LspRustAnalyzerStandaloneInitializationV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LspRustAnalyzerStandaloneInitializationV1(<bound>)")
    }
}

impl LspInitializationProfileV1 {
    pub const fn disabled() -> Self {
        Self {
            workspace: None,
            readiness_notification_method: None,
            protocol: None,
            provider_options: None,
        }
    }

    pub fn try_workspace(
        root_uri: String,
        workspace_name: String,
        readiness_notification_method: Option<String>,
        limits: LspLimitsV1,
    ) -> Option<Self> {
        let root = root_uri.strip_prefix("file://")?;
        let readiness_valid = readiness_notification_method.as_ref().is_none_or(|method| {
            !method.is_empty()
                && method.len() <= MAX_LSP_METHOD_BYTES_V1
                && method.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'$' | b'_' | b'-' | b'.')
                })
        });
        (limits.is_valid()
            && !root.is_empty()
            && root_uri.len() <= limits.max_uri_bytes
            && root_uri
                .bytes()
                .all(|byte| !byte.is_ascii_control() && byte != b' ')
            && !workspace_name.is_empty()
            && workspace_name.len() <= MAX_LSP_WORKSPACE_NAME_BYTES_V1
            && workspace_name.bytes().all(|byte| !byte.is_ascii_control())
            && readiness_valid)
            .then_some(Self {
                workspace: Some(LspWorkspaceFolderV1 {
                    uri: root_uri,
                    name: workspace_name,
                }),
                readiness_notification_method,
                protocol: None,
                provider_options: None,
            })
    }

    pub fn try_workspace_with_protocol(
        root_uri: String,
        workspace_name: String,
        protocol: LspProviderProtocolProfileV1,
        limits: LspLimitsV1,
    ) -> Option<Self> {
        let method = protocol.readiness_notification_method.clone();
        let mut profile = Self::try_workspace(root_uri, workspace_name, Some(method), limits)?;
        profile.protocol = Some(protocol);
        Some(profile)
    }

    /// Constructs the exact standalone rust-analyzer policy. The package root
    /// is fixed to `src/lib.rs`, resolved against this profile's file root and
    /// length-checked before any initialize send can occur.
    pub fn try_rust_analyzer_standalone(
        root_uri: String,
        workspace_name: String,
        protocol: LspProviderProtocolProfileV1,
        limits: LspLimitsV1,
    ) -> Option<Self> {
        let mut profile =
            Self::try_workspace_with_protocol(root_uri, workspace_name, protocol, limits)?;
        let root = &profile.workspace.as_ref()?.uri;
        let root_path = canonical_absolute_path_from_file_uri(root, limits)?;
        let linked_project_path = if root_path == "/" {
            "/src/lib.rs".to_owned()
        } else {
            format!("{root_path}/src/lib.rs")
        };
        if linked_project_path.len() > limits.max_uri_bytes {
            return None;
        }
        let _linked_project = resolve_workspace_relative_document_uri(root, "src/lib.rs", limits)?;
        profile.provider_options =
            Some(LspProviderInitializationOptionsV1::RustAnalyzerStandalone(
                LspRustAnalyzerStandaloneInitializationV1 {
                    linked_project_path,
                },
            ));
        Some(profile)
    }

    pub const fn has_workspace(&self) -> bool {
        self.workspace.is_some()
    }

    pub const fn observes_readiness_notifications(&self) -> bool {
        self.readiness_notification_method.is_some()
    }

    pub const fn has_provider_initialization_options(&self) -> bool {
        self.provider_options.is_some()
    }
}

impl Default for LspInitializationProfileV1 {
    fn default() -> Self {
        Self::disabled()
    }
}

impl fmt::Debug for LspInitializationProfileV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LspInitializationProfileV1")
            .field("workspace", &self.workspace.as_ref().map(|_| "<bound>"))
            .field(
                "readiness_notification",
                &self
                    .readiness_notification_method
                    .as_ref()
                    .map(|_| "<bound>"),
            )
            .field("protocol", &self.protocol.as_ref().map(|_| "<pinned>"))
            .field(
                "provider_options",
                &self.provider_options.as_ref().map(|_| "<bound>"),
            )
            .finish()
    }
}

/// Pinned provider protocol identity. The digest is supplied by the owning
/// qualification profile; expected strings and method never appear in Debug
/// or metric labels.
#[derive(Clone, Eq, PartialEq)]
pub struct LspProviderProtocolProfileV1 {
    profile_digest: [u8; 32],
    expected_server_name: String,
    expected_server_version: Option<String>,
    readiness_notification_method: String,
    readiness_notification_schema: LspReadinessNotificationSchemaV1,
}

/// Closed payload shapes accepted from a pinned provider's readiness
/// notification. Message text is validated against a fixed bound and then
/// discarded; it never enters evidence, Debug output, or metric labels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LspReadinessNotificationSchemaV1 {
    HealthAndQuiescent,
    HealthAndQuiescentWithOptionalMessage,
}

impl LspProviderProtocolProfileV1 {
    pub fn try_new(
        profile_digest: [u8; 32],
        expected_server_name: String,
        expected_server_version: Option<String>,
        readiness_notification_method: String,
    ) -> Option<Self> {
        Self::try_new_with_readiness_schema(
            profile_digest,
            expected_server_name,
            expected_server_version,
            readiness_notification_method,
            LspReadinessNotificationSchemaV1::HealthAndQuiescent,
        )
    }

    pub fn try_new_with_readiness_schema(
        profile_digest: [u8; 32],
        expected_server_name: String,
        expected_server_version: Option<String>,
        readiness_notification_method: String,
        readiness_notification_schema: LspReadinessNotificationSchemaV1,
    ) -> Option<Self> {
        let version_valid = expected_server_version.as_ref().is_none_or(|version| {
            !version.is_empty()
                && version.len() <= MAX_LSP_SERVER_INFO_VERSION_BYTES_V1
                && version.bytes().all(|byte| !byte.is_ascii_control())
        });
        ((!profile_digest.iter().all(|byte| *byte == 0))
            && !expected_server_name.is_empty()
            && expected_server_name.len() <= MAX_LSP_SERVER_INFO_NAME_BYTES_V1
            && expected_server_name
                .bytes()
                .all(|byte| !byte.is_ascii_control())
            && version_valid
            && !readiness_notification_method.is_empty()
            && readiness_notification_method.len() <= MAX_LSP_METHOD_BYTES_V1
            && readiness_notification_method.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'$' | b'_' | b'-' | b'.')
            }))
        .then_some(Self {
            profile_digest,
            expected_server_name,
            expected_server_version,
            readiness_notification_method,
            readiness_notification_schema,
        })
    }

    /// Proves that this protocol policy was constructed for the exact
    /// qualified launch profile consumed by the composition root.
    pub fn matches_launch_profile_digest(&self, digest: &[u8; 32]) -> bool {
        &self.profile_digest == digest
    }
}

impl fmt::Debug for LspProviderProtocolProfileV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LspProviderProtocolProfileV1(<opaque>)")
    }
}

/// Closed construction product for one adapter instance. Grouping provider
/// identity, framing, semantic bounds and initialization policy prevents
/// callers from assembling a long positional argument list with mismatched
/// policies.
#[derive(Clone, Eq, PartialEq)]
pub struct LspAdapterProfileV1 {
    provenance: SemanticProviderProvenanceV1,
    framing: LspLimitsV1,
    semantic: SemanticLimitsV1,
    initialization: LspInitializationProfileV1,
}

impl LspAdapterProfileV1 {
    pub fn try_new(
        provenance: SemanticProviderProvenanceV1,
        framing: LspLimitsV1,
        semantic: SemanticLimitsV1,
        initialization: LspInitializationProfileV1,
    ) -> Option<Self> {
        (framing.is_valid() && semantic.is_valid()).then_some(Self {
            provenance,
            framing,
            semantic,
            initialization,
        })
    }
}

impl fmt::Debug for LspAdapterProfileV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LspAdapterProfileV1")
            .field("provenance", &"<opaque>")
            .field("framing", &self.framing)
            .field("semantic", &self.semantic)
            .field("initialization", &self.initialization)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LspServerHealthV1 {
    Ok,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LspReadinessNotificationV1 {
    pub health: LspServerHealthV1,
    pub quiescent: bool,
}

/// Bounded server identity captured from InitializeResult. Values remain
/// opaque; qualification code compares exact expected identity without Debug
/// or metric-label disclosure.
#[derive(Clone, Eq, PartialEq)]
pub struct LspServerInfoV1 {
    name: String,
    version: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LspServerIdentityDispositionV1 {
    UnpinnedAbsent,
    UnpinnedObserved,
    PinnedUnobserved,
    PinnedExact,
    PinnedRefused,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LspReadinessEvidenceSourceV1 {
    Polling,
    NotificationBound,
}

/// Content-safe observation from one live adapter. It is not cleanup evidence;
/// the retirement driver must capture it from the same owned adapter when it
/// creates a terminal receipt.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct LspProviderQualificationEvidenceV1 {
    profile_digest: Option<[u8; 32]>,
    workspace_bound: bool,
    server_identity: LspServerIdentityDispositionV1,
    readiness_source: LspReadinessEvidenceSourceV1,
    last_notification_readiness: Option<LspIndexReadinessV1>,
    notifications_observed: u64,
    readiness_promotions: u64,
    readiness_demotions: u64,
    readiness_unavailable: u64,
    read_index: bool,
    rename_plan: bool,
    saturated: bool,
}

impl LspProviderQualificationEvidenceV1 {
    pub fn profile_matches(&self, digest: &[u8; 32]) -> bool {
        self.profile_digest.as_ref() == Some(digest)
    }

    pub const fn workspace_bound(&self) -> bool {
        self.workspace_bound
    }

    pub const fn server_identity(&self) -> LspServerIdentityDispositionV1 {
        self.server_identity
    }

    pub const fn readiness_source(&self) -> LspReadinessEvidenceSourceV1 {
        self.readiness_source
    }

    pub const fn last_notification_readiness(&self) -> Option<LspIndexReadinessV1> {
        self.last_notification_readiness
    }

    pub const fn notifications_observed(&self) -> u64 {
        self.notifications_observed
    }

    pub const fn readiness_transitions(&self) -> (u64, u64, u64) {
        (
            self.readiness_promotions,
            self.readiness_demotions,
            self.readiness_unavailable,
        )
    }

    pub const fn capabilities(&self) -> (bool, bool) {
        (self.read_index, self.rename_plan)
    }

    pub const fn saturated(&self) -> bool {
        self.saturated
    }
}

impl fmt::Debug for LspProviderQualificationEvidenceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LspProviderQualificationEvidenceV1")
            .field("profile", &self.profile_digest.map(|_| "<bound>"))
            .field("workspace_bound", &self.workspace_bound)
            .field("server_identity", &self.server_identity)
            .field("readiness_source", &self.readiness_source)
            .field(
                "last_notification_readiness",
                &self.last_notification_readiness,
            )
            .field("notifications_observed", &self.notifications_observed)
            .field("readiness_promotions", &self.readiness_promotions)
            .field("readiness_demotions", &self.readiness_demotions)
            .field("readiness_unavailable", &self.readiness_unavailable)
            .field("read_index", &self.read_index)
            .field("rename_plan", &self.rename_plan)
            .field("saturated", &self.saturated)
            .finish()
    }
}

impl LspServerInfoV1 {
    pub fn matches(&self, name: &str, version: Option<&str>) -> bool {
        self.name == name && self.version.as_deref() == version
    }
}

impl fmt::Debug for LspServerInfoV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LspServerInfoV1(<opaque>)")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LspLimitsV1 {
    pub max_header_bytes: usize,
    pub max_headers: usize,
    pub max_body_bytes: usize,
    pub max_buffer_bytes: usize,
    pub max_documents: usize,
    pub max_uri_bytes: usize,
    pub max_total_source_bytes: usize,
}

impl LspLimitsV1 {
    pub const fn reference() -> Self {
        Self {
            max_header_bytes: MAX_LSP_HEADER_BYTES_V1,
            max_headers: MAX_LSP_HEADERS_V1,
            max_body_bytes: MAX_LSP_BODY_BYTES_V1,
            max_buffer_bytes: MAX_LSP_BUFFER_BYTES_V1,
            max_documents: MAX_LSP_DOCUMENTS_V1,
            max_uri_bytes: MAX_LSP_URI_BYTES_V1,
            max_total_source_bytes: MAX_LSP_TOTAL_SOURCE_BYTES_V1,
        }
    }

    pub const fn is_valid(self) -> bool {
        self.max_header_bytes >= 4
            && self.max_header_bytes <= MAX_LSP_HEADER_BYTES_V1
            && self.max_headers > 0
            && self.max_headers <= MAX_LSP_HEADERS_V1
            && self.max_body_bytes > 0
            && self.max_body_bytes <= MAX_LSP_BODY_BYTES_V1
            && self.max_buffer_bytes >= self.max_header_bytes
            && self.max_buffer_bytes
                <= MAX_LSP_HEADER_BYTES_V1.saturating_add(MAX_LSP_BODY_BYTES_V1)
            && self.max_documents > 0
            && self.max_documents <= MAX_LSP_DOCUMENTS_V1
            && self.max_uri_bytes > 0
            && self.max_uri_bytes <= MAX_LSP_URI_BYTES_V1
            && self.max_total_source_bytes > 0
            && self.max_total_source_bytes <= MAX_LSP_TOTAL_SOURCE_BYTES_V1
    }
}

/// Content-free construction facts for an already-owned LSP adapter. This is
/// an observation, not provider authority: a composition must consume the
/// adapter itself when it closes these limits against its service/session
/// policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LspAdapterLimitObservationV1 {
    framing: LspLimitsV1,
    semantic: SemanticLimitsV1,
}

impl LspAdapterLimitObservationV1 {
    pub const fn framing(self) -> LspLimitsV1 {
        self.framing
    }

    pub const fn semantic(self) -> SemanticLimitsV1 {
        self.semantic
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LspFrameRefusalV1 {
    InvalidLimits,
    BufferLimit,
    HeaderLimit,
    HeaderCount,
    MalformedHeader,
    MissingContentLength,
    DuplicateContentLength,
    BodyLimit,
}

/// Incremental bounded `Content-Length` frame decoder.
pub struct LspFrameDecoderV1 {
    limits: LspLimitsV1,
    buffer: Vec<u8>,
}

impl LspFrameDecoderV1 {
    pub fn try_new(limits: LspLimitsV1) -> Result<Self, LspFrameRefusalV1> {
        if !limits.is_valid() {
            return Err(LspFrameRefusalV1::InvalidLimits);
        }
        Ok(Self {
            limits,
            buffer: Vec::new(),
        })
    }

    fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn push(&mut self, chunk: &[u8]) -> Result<(), LspFrameRefusalV1> {
        if self
            .buffer
            .len()
            .checked_add(chunk.len())
            .is_none_or(|length| length > self.limits.max_buffer_bytes)
        {
            return Err(LspFrameRefusalV1::BufferLimit);
        }
        self.buffer.extend_from_slice(chunk);
        Ok(())
    }

    pub fn next_frame(&mut self) -> Result<Option<Vec<u8>>, LspFrameRefusalV1> {
        let Some(header_end) = find_header_end(&self.buffer) else {
            if self.buffer.len() > self.limits.max_header_bytes {
                return Err(LspFrameRefusalV1::HeaderLimit);
            }
            return Ok(None);
        };
        if header_end + 4 > self.limits.max_header_bytes {
            return Err(LspFrameRefusalV1::HeaderLimit);
        }
        let header = &self.buffer[..header_end];
        if !header.is_ascii() {
            return Err(LspFrameRefusalV1::MalformedHeader);
        }
        for (index, byte) in header.iter().copied().enumerate() {
            if (byte == b'\n' && (index == 0 || header[index - 1] != b'\r'))
                || (byte == b'\r' && header.get(index + 1) != Some(&b'\n'))
            {
                return Err(LspFrameRefusalV1::MalformedHeader);
            }
        }
        let mut content_length = None;
        let mut count = 0_usize;
        for line in header.split(|byte| *byte == b'\n') {
            count = count.saturating_add(1);
            if count > self.limits.max_headers {
                return Err(LspFrameRefusalV1::HeaderCount);
            }
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            let Some((name, value)) = split_once(line, b':') else {
                return Err(LspFrameRefusalV1::MalformedHeader);
            };
            if name.eq_ignore_ascii_case(b"Content-Length") {
                if content_length.is_some() {
                    return Err(LspFrameRefusalV1::DuplicateContentLength);
                }
                let value = trim_ascii(value);
                if value.is_empty() || value.iter().any(|byte| !byte.is_ascii_digit()) {
                    return Err(LspFrameRefusalV1::MalformedHeader);
                }
                let text =
                    std::str::from_utf8(value).map_err(|_| LspFrameRefusalV1::MalformedHeader)?;
                content_length = Some(
                    text.parse::<usize>()
                        .map_err(|_| LspFrameRefusalV1::MalformedHeader)?,
                );
            } else if !name.eq_ignore_ascii_case(b"Content-Type") {
                return Err(LspFrameRefusalV1::MalformedHeader);
            }
        }
        let length = content_length.ok_or(LspFrameRefusalV1::MissingContentLength)?;
        if length == 0 || length > self.limits.max_body_bytes {
            return Err(LspFrameRefusalV1::BodyLimit);
        }
        let body_start = header_end + 4;
        let Some(frame_end) = body_start.checked_add(length) else {
            return Err(LspFrameRefusalV1::BodyLimit);
        };
        if frame_end > self.limits.max_buffer_bytes {
            return Err(LspFrameRefusalV1::BufferLimit);
        }
        if self.buffer.len() < frame_end {
            return Ok(None);
        }
        let body = self.buffer[body_start..frame_end].to_vec();
        self.buffer.drain(..frame_end);
        Ok(Some(body))
    }
}

pub fn encode_lsp_frame_v1(body: &[u8], limits: LspLimitsV1) -> Result<Vec<u8>, LspFrameRefusalV1> {
    if !limits.is_valid() {
        return Err(LspFrameRefusalV1::InvalidLimits);
    }
    if body.is_empty() || body.len() > limits.max_body_bytes {
        return Err(LspFrameRefusalV1::BodyLimit);
    }
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    if header.len() > limits.max_header_bytes
        || header
            .len()
            .checked_add(body.len())
            .is_none_or(|length| length > limits.max_buffer_bytes)
    {
        return Err(LspFrameRefusalV1::BufferLimit);
    }
    let mut frame = Vec::with_capacity(header.len() + body.len());
    frame.extend_from_slice(header.as_bytes());
    frame.extend_from_slice(body);
    Ok(frame)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LspPositionEncodingV1 {
    Utf8,
    Utf16,
    Utf32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LspPositionV1 {
    pub line: u32,
    pub character: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LspPositionRefusalV1 {
    OffsetLimit,
    LineLimit,
    CharacterLimit,
    SplitEncoding,
    CrlfInterior,
}

pub fn byte_offset_to_lsp_position_v1(
    source: &str,
    offset: usize,
    encoding: LspPositionEncodingV1,
) -> Result<LspPositionV1, LspPositionRefusalV1> {
    if offset > source.len() || !source.is_char_boundary(offset) {
        return Err(LspPositionRefusalV1::OffsetLimit);
    }
    let (line, start, end) = line_for_offset(source, offset)?;
    if offset > end {
        return Err(LspPositionRefusalV1::CrlfInterior);
    }
    let prefix = &source[start..offset];
    let character = units(prefix, encoding)?;
    Ok(LspPositionV1 {
        line: u32::try_from(line).map_err(|_| LspPositionRefusalV1::LineLimit)?,
        character,
    })
}

pub fn lsp_position_to_byte_offset_v1(
    source: &str,
    position: LspPositionV1,
    encoding: LspPositionEncodingV1,
) -> Result<usize, LspPositionRefusalV1> {
    let (start, end) =
        line_bounds(source, position.line as usize).ok_or(LspPositionRefusalV1::LineLimit)?;
    let line = &source[start..end];
    let wanted = position.character;
    let mut current = 0_u32;
    if wanted == 0 {
        return Ok(start);
    }
    for (relative, scalar) in line.char_indices() {
        let next = current
            .checked_add(scalar_units(scalar, encoding))
            .ok_or(LspPositionRefusalV1::CharacterLimit)?;
        if wanted > current && wanted < next {
            return Err(LspPositionRefusalV1::SplitEncoding);
        }
        current = next;
        if current == wanted {
            return Ok(start + relative + scalar.len_utf8());
        }
    }
    Err(LspPositionRefusalV1::CharacterLimit)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LspSendOutcomeV1 {
    Accepted,
    Unavailable,
    Refused,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LspReceiveOutcomeV1 {
    Pending,
    Chunk(Vec<u8>),
    Closed,
    Failed,
}

/// Content-safe lifecycle projection supplied by an already-owned transport.
/// Closure variants prove only the transport's qualified terminal product;
/// provider protocol shutdown requires separate adapter-session evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LspTransportTerminalV1 {
    Active,
    GracefulClosed,
    ForcedClosed,
    /// Provider/session failed, but the owning transport separately confirmed
    /// whole-group cleanup and leader reap. Protocol success is not implied.
    FailedCleanupConfirmed,
    Failed,
    Unobserved,
}

/// An already-owned, nonblocking byte transport. This grants no process or
/// network construction authority.
pub trait LspTransportPortV1 {
    fn try_send(&mut self, frame: &[u8]) -> LspSendOutcomeV1;
    fn poll_receive(&mut self) -> LspReceiveOutcomeV1;
    fn close(&mut self);

    /// Composition-owned escalation. Generic transports may only have their
    /// ordinary close primitive, so the default makes no forced-cleanup claim.
    fn abort(&mut self) {
        self.close();
    }

    fn terminal_state(&self) -> LspTransportTerminalV1 {
        LspTransportTerminalV1::Unobserved
    }
}

pub trait SemanticGenerationIssuerV1 {
    fn issue(&mut self) -> Option<SemanticIndexGenerationV1>;
}

/// Composition-owned authority for assigning an opaque or workspace-relative
/// locator to a semantic document. Implementations must be deterministic and
/// injective for the document population they admit. The adapter independently
/// resolves relative locators against its bound root, enforces its document
/// bound, and rejects resolved-URI collisions.
pub trait LspVirtualDocumentIssuerV1 {
    fn issue(&mut self, document: SemanticDocumentRefV1) -> Option<LspVirtualDocumentV1>;
}

/// Retains the explicit-registration surface for compositions that provision
/// the complete document population before synchronization.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LspExplicitVirtualDocumentsV1;

impl LspVirtualDocumentIssuerV1 for LspExplicitVirtualDocumentsV1 {
    fn issue(&mut self, _document: SemanticDocumentRefV1) -> Option<LspVirtualDocumentV1> {
        None
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LspIndexReadinessV1 {
    Indexing,
    Current,
    Unavailable,
}

/// Privately owned, server-profile-specific index readiness evidence.
/// Generic LSP initialization alone never proves semantic currency.
pub trait LspIndexReadinessPortV1 {
    fn poll_readiness(&mut self) -> LspIndexReadinessV1;

    /// Receives only the adapter-validated closed status shape configured by
    /// the initialization profile. Raw provider JSON never crosses this seam.
    fn observe_notification(
        &mut self,
        _notification: LspReadinessNotificationV1,
    ) -> Option<LspIndexReadinessV1> {
        None
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LspNeverReadyV1;

impl LspIndexReadinessPortV1 for LspNeverReadyV1 {
    fn poll_readiness(&mut self) -> LspIndexReadinessV1 {
        LspIndexReadinessV1::Indexing
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LspSessionStateV1 {
    Cold,
    Initializing,
    Indexing {
        encoding: LspPositionEncodingV1,
        read_index: bool,
    },
    Ready {
        generation: SemanticIndexGenerationV1,
        encoding: LspPositionEncodingV1,
        read_index: bool,
    },
    Unavailable {
        encoding: LspPositionEncodingV1,
        read_index: bool,
    },
    ShuttingDown {
        request_id: u64,
    },
    Failed,
    Closed,
    Aborted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LspStartOutcomeV1 {
    Started,
    AlreadyStarted,
    Unavailable,
    Refused,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LspShutdownOutcomeV1 {
    Started,
    Busy,
    AlreadyTerminal,
    Unavailable,
    Refused,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LspAbortOutcomeV1 {
    Aborted,
    AlreadyTerminal,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LspUsageV1 {
    pub frames_sent: u64,
    pub frames_received: u64,
    pub snapshots_opened: u64,
    pub snapshots_changed: u64,
    pub snapshots_closed: u64,
    pub unsupported_queries: u64,
    pub requests_submitted: u64,
    pub requests_completed: u64,
    pub output_refused: u64,
    pub source_bytes_current: u64,
    pub source_bytes_high_water: u64,
    pub cancellations: u64,
    pub cancelled_correlations_current: u64,
    pub cancelled_correlations_high_water: u64,
    pub cancelled_responses_correlated: u64,
    pub orphan_messages: u64,
    pub malformed_messages: u64,
    pub transport_unavailable: u64,
    pub transport_refused: u64,
    pub transport_closed: u64,
    pub readiness_promotions: u64,
    pub readiness_demotions: u64,
    pub readiness_unavailable: u64,
    pub shutdown_started: u64,
    pub shutdown_completed: u64,
    pub shutdown_refused: u64,
    pub shutdown_aborted: u64,
    pub rename_requests_submitted: u64,
    pub rename_plans_completed: u64,
    pub rename_plans_refused: u64,
    pub server_info_captured: u64,
    pub server_info_refused: u64,
    pub notifications_observed: u64,
    pub notifications_ignored: u64,
    pub notifications_refused: u64,
    pub inbound_server_requests: u64,
    pub saturated: bool,
}

#[derive(Clone, Eq, PartialEq)]
enum LspDocumentLocatorKindV1 {
    OpaqueUri(String),
    WorkspaceRelative(String),
}

/// Closed document locator. Callers may retain opaque `vellum-lsp://` behavior
/// or provide a canonical relative component path. Only the adapter may turn a
/// relative path into a file URI, using its already-bound initialization root.
#[derive(Clone, Eq, PartialEq)]
pub struct LspVirtualDocumentV1(LspDocumentLocatorKindV1);

/// Taxonomic name for the closed locator surface. The legacy name remains the
/// trait currency so existing opaque issuers stay source-compatible.
pub type LspDocumentLocatorV1 = LspVirtualDocumentV1;

#[derive(Clone, Eq, PartialEq)]
struct LspResolvedDocumentUriV1(String);

impl fmt::Debug for LspResolvedDocumentUriV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LspVirtualDocumentV1(<opaque>)")
    }
}

fn canonical_absolute_path_from_file_uri(uri: &str, limits: LspLimitsV1) -> Option<String> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let encoded = uri.strip_prefix("file://")?;
    if !encoded.starts_with('/') || encoded.contains('?') || encoded.contains('#') {
        return None;
    }
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] == b'%' {
            let high = *bytes.get(cursor.checked_add(1)?)?;
            let low = *bytes.get(cursor.checked_add(2)?)?;
            let decode_nibble = |byte| match byte {
                b'0'..=b'9' => Some(byte - b'0'),
                b'A'..=b'F' => Some(byte - b'A' + 10),
                _ => None,
            };
            decoded.push(decode_nibble(high)? << 4 | decode_nibble(low)?);
            cursor = cursor.checked_add(3)?;
        } else {
            decoded.push(bytes[cursor]);
            cursor = cursor.checked_add(1)?;
        }
    }
    let path = String::from_utf8(decoded).ok()?;
    if path.len() > limits.max_uri_bytes
        || path.bytes().any(|byte| byte.is_ascii_control())
        || (path.len() > 1 && path.ends_with('/'))
        || (path != "/"
            && path
                .split('/')
                .skip(1)
                .any(|component| component.is_empty() || matches!(component, "." | "..")))
    {
        return None;
    }
    let mut canonical = String::with_capacity(uri.len());
    canonical.push_str("file://");
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'.' | b'_' | b'~') {
            canonical.push(char::from(byte));
        } else {
            canonical.push('%');
            canonical.push(char::from(HEX[usize::from(byte >> 4)]));
            canonical.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    (canonical == uri).then_some(path)
}

fn resolve_workspace_relative_document_uri(
    root: &str,
    relative: &str,
    limits: LspLimitsV1,
) -> Option<LspResolvedDocumentUriV1> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let separator = usize::from(!root.ends_with('/'));
    let capacity = root
        .len()
        .checked_add(separator)?
        .checked_add(relative.len().checked_mul(3)?)?;
    if root.len().checked_add(separator)? > limits.max_uri_bytes {
        return None;
    }
    let mut uri = String::with_capacity(capacity);
    uri.push_str(root);
    if separator == 1 {
        uri.push('/');
    }
    for byte in relative.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'.' | b'_' | b'~') {
            uri.push(char::from(byte));
        } else {
            uri.push('%');
            uri.push(char::from(HEX[usize::from(byte >> 4)]));
            uri.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        if uri.len() > limits.max_uri_bytes {
            return None;
        }
    }
    Some(LspResolvedDocumentUriV1(uri))
}

impl LspVirtualDocumentV1 {
    pub fn try_new(value: String, limits: LspLimitsV1) -> Option<Self> {
        let token = value.strip_prefix("vellum-lsp://")?;
        (limits.is_valid()
            && value.len() <= limits.max_uri_bytes
            && token
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphanumeric)
            && token
                .as_bytes()
                .last()
                .is_some_and(u8::is_ascii_alphanumeric)
            && token
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')))
        .then_some(Self(LspDocumentLocatorKindV1::OpaqueUri(value)))
    }

    pub fn try_workspace_relative(value: String, limits: LspLimitsV1) -> Option<Self> {
        let canonical = limits.is_valid()
            && !value.is_empty()
            && value.len() <= limits.max_uri_bytes
            && !value.starts_with('/')
            && !value.ends_with('/')
            && !value
                .bytes()
                .any(|byte| byte.is_ascii_control() || matches!(byte, b'\\' | b'%' | b'?' | b'#'))
            && value
                .split('/')
                .all(|component| !component.is_empty() && !matches!(component, "." | ".."));
        canonical.then_some(Self(LspDocumentLocatorKindV1::WorkspaceRelative(value)))
    }
}

impl std::fmt::Debug for LspVirtualDocumentV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            LspDocumentLocatorKindV1::OpaqueUri(_) => {
                formatter.write_str("LspVirtualDocumentV1(<opaque>)")
            }
            LspDocumentLocatorKindV1::WorkspaceRelative(_) => {
                formatter.write_str("LspVirtualDocumentV1(<workspace-relative>)")
            }
        }
    }
}

struct DocumentState {
    uri: LspResolvedDocumentUriV1,
    target: SemanticTargetV1,
    language: String,
    source: String,
    version: u32,
    opened: bool,
}

#[derive(Clone)]
struct PendingRequest {
    request: SemanticRequestV1,
    generation: SemanticIndexGenerationV1,
    stage: PendingRequestStage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PendingRequestStage {
    Direct,
    PreparingCallHierarchy,
    ResolvingCallHierarchy,
}

pub struct LspAdapterV1<
    T: LspTransportPortV1,
    G,
    R = LspNeverReadyV1,
    U = LspExplicitVirtualDocumentsV1,
> {
    transport: T,
    generations: G,
    readiness: R,
    virtual_documents: U,
    provenance: SemanticProviderProvenanceV1,
    limits: LspLimitsV1,
    semantic_limits: SemanticLimitsV1,
    initialization: LspInitializationProfileV1,
    server_info: Option<LspServerInfoV1>,
    server_identity: LspServerIdentityDispositionV1,
    last_notification_readiness: Option<LspIndexReadinessV1>,
    decoder: LspFrameDecoderV1,
    state: LspSessionStateV1,
    documents: BTreeMap<SemanticDocumentRefV1, DocumentState>,
    pending: BTreeMap<u64, PendingRequest>,
    cancelled: BTreeMap<u64, SemanticRequestRefV1>,
    completions: VecDeque<SemanticProviderCompletionV1>,
    bridge_failure: Option<SemanticBridgePollFailureV1>,
    next_request_id: u64,
    read_index: bool,
    diagnostics: bool,
    symbols: bool,
    definition: bool,
    references: bool,
    implementations: bool,
    call_hierarchy: bool,
    type_at_point: bool,
    rename_plan: bool,
    usage: LspUsageV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LspFreshStartRefusalV1 {
    NotFresh,
    RuntimeUnavailable,
    DeliveryRefused,
}

/// Failed fresh start retains the exact adapter and its transport cleanup
/// authority. Debug exposes only the closed refusal class.
#[must_use = "failed LSP start retains adapter cleanup authority"]
pub struct LspFreshStartFailureV1<
    T: LspTransportPortV1,
    G,
    R = LspNeverReadyV1,
    U = LspExplicitVirtualDocumentsV1,
> {
    reason: LspFreshStartRefusalV1,
    adapter: LspAdapterV1<T, G, R, U>,
}

impl<T: LspTransportPortV1, G, R, U> LspFreshStartFailureV1<T, G, R, U> {
    pub const fn reason(&self) -> LspFreshStartRefusalV1 {
        self.reason
    }

    pub fn into_adapter(self) -> LspAdapterV1<T, G, R, U> {
        self.adapter
    }
}

impl<T: LspTransportPortV1, G, R, U> fmt::Debug for LspFreshStartFailureV1<T, G, R, U> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LspFreshStartFailureV1")
            .field("reason", &self.reason)
            .field("adapter", &"<owned>")
            .finish()
    }
}

/// Fresh, empty adapter after its initialize request was accepted. This
/// typestate is the only LSP provider accepted by the ordered local
/// composition; it cannot be constructed from an already-used adapter.
#[must_use = "started LSP adapter owns live provider cleanup authority"]
pub struct LspFreshStartedAdapterV1<
    T: LspTransportPortV1,
    G,
    R = LspNeverReadyV1,
    U = LspExplicitVirtualDocumentsV1,
> {
    inner: LspAdapterV1<T, G, R, U>,
}

pub type LspFreshStartResultV1<T, G, R, U> =
    Result<LspFreshStartedAdapterV1<T, G, R, U>, Box<LspFreshStartFailureV1<T, G, R, U>>>;

impl<T: LspTransportPortV1, G, R, U> fmt::Debug for LspFreshStartedAdapterV1<T, G, R, U> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LspFreshStartedAdapterV1(<opaque>)")
    }
}

impl<T: LspTransportPortV1, G: SemanticGenerationIssuerV1> LspAdapterV1<T, G, LspNeverReadyV1> {
    pub fn try_new(
        transport: T,
        generations: G,
        provenance: SemanticProviderProvenanceV1,
        limits: LspLimitsV1,
    ) -> Option<Self> {
        Self::try_new_with_semantic_limits(
            transport,
            generations,
            provenance,
            limits,
            SemanticLimitsV1::reference(),
        )
    }

    pub fn try_new_with_semantic_limits(
        transport: T,
        generations: G,
        provenance: SemanticProviderProvenanceV1,
        limits: LspLimitsV1,
        semantic_limits: SemanticLimitsV1,
    ) -> Option<Self> {
        Self::try_new_with_readiness(
            transport,
            generations,
            LspNeverReadyV1,
            provenance,
            limits,
            semantic_limits,
        )
    }
}

impl<T: LspTransportPortV1, G: SemanticGenerationIssuerV1, R: LspIndexReadinessPortV1>
    LspAdapterV1<T, G, R>
{
    pub fn try_new_with_readiness(
        transport: T,
        generations: G,
        readiness: R,
        provenance: SemanticProviderProvenanceV1,
        limits: LspLimitsV1,
        semantic_limits: SemanticLimitsV1,
    ) -> Option<Self> {
        Self::try_new_with_readiness_and_documents(
            transport,
            generations,
            readiness,
            LspExplicitVirtualDocumentsV1,
            provenance,
            limits,
            semantic_limits,
        )
    }
}

impl<
    T: LspTransportPortV1,
    G: SemanticGenerationIssuerV1,
    R: LspIndexReadinessPortV1,
    U: LspVirtualDocumentIssuerV1,
> LspAdapterV1<T, G, R, U>
{
    pub fn try_new_with_readiness_and_documents(
        transport: T,
        generations: G,
        readiness: R,
        virtual_documents: U,
        provenance: SemanticProviderProvenanceV1,
        limits: LspLimitsV1,
        semantic_limits: SemanticLimitsV1,
    ) -> Option<Self> {
        Self::try_new_with_profile(
            transport,
            generations,
            readiness,
            virtual_documents,
            LspAdapterProfileV1::try_new(
                provenance,
                limits,
                semantic_limits,
                LspInitializationProfileV1::disabled(),
            )?,
        )
    }

    pub fn try_new_with_profile(
        transport: T,
        generations: G,
        readiness: R,
        virtual_documents: U,
        profile: LspAdapterProfileV1,
    ) -> Option<Self> {
        let LspAdapterProfileV1 {
            provenance,
            framing: limits,
            semantic: semantic_limits,
            initialization,
        } = profile;
        let server_identity = if initialization.protocol.is_some() {
            LspServerIdentityDispositionV1::PinnedUnobserved
        } else {
            LspServerIdentityDispositionV1::UnpinnedAbsent
        };
        Some(Self {
            transport,
            generations,
            readiness,
            virtual_documents,
            provenance,
            limits,
            semantic_limits,
            initialization,
            server_info: None,
            server_identity,
            last_notification_readiness: None,
            decoder: LspFrameDecoderV1::try_new(limits).ok()?,
            state: LspSessionStateV1::Cold,
            documents: BTreeMap::new(),
            pending: BTreeMap::new(),
            cancelled: BTreeMap::new(),
            completions: VecDeque::new(),
            bridge_failure: None,
            next_request_id: 2,
            read_index: false,
            diagnostics: false,
            symbols: false,
            definition: false,
            references: false,
            implementations: false,
            call_hierarchy: false,
            type_at_point: false,
            rename_plan: false,
            usage: LspUsageV1::default(),
        })
    }

    pub const fn session_state(&self) -> LspSessionStateV1 {
        self.state
    }

    pub const fn usage(&self) -> LspUsageV1 {
        self.usage
    }

    pub fn server_info(&self) -> Option<&LspServerInfoV1> {
        self.server_info.as_ref()
    }

    pub fn qualification_evidence(&self) -> LspProviderQualificationEvidenceV1 {
        LspProviderQualificationEvidenceV1 {
            profile_digest: self
                .initialization
                .protocol
                .as_ref()
                .map(|profile| profile.profile_digest),
            workspace_bound: self.initialization.workspace.is_some(),
            server_identity: self.server_identity,
            readiness_source: if self.initialization.readiness_notification_method.is_some() {
                LspReadinessEvidenceSourceV1::NotificationBound
            } else {
                LspReadinessEvidenceSourceV1::Polling
            },
            last_notification_readiness: self.last_notification_readiness,
            notifications_observed: self.usage.notifications_observed,
            readiness_promotions: self.usage.readiness_promotions,
            readiness_demotions: self.usage.readiness_demotions,
            readiness_unavailable: self.usage.readiness_unavailable,
            read_index: self.read_index,
            rename_plan: self.rename_plan,
            saturated: self.usage.saturated,
        }
    }

    /// Projects only bounded numeric construction facts. Transport, provider,
    /// source, URI assignments and process identity remain private.
    pub const fn limit_observation(&self) -> LspAdapterLimitObservationV1 {
        LspAdapterLimitObservationV1 {
            framing: self.limits,
            semantic: self.semantic_limits,
        }
    }

    /// Consumes a never-used, empty adapter and accepts exactly one initialize
    /// send. Refusal retains the complete adapter. This proves adapter-local
    /// freshness; the injected transport remains responsible for its own
    /// pre-construction byte-buffer contract.
    pub fn try_start_fresh(mut self) -> LspFreshStartResultV1<T, G, R, U> {
        let terminal = self.transport.terminal_state();
        let fresh = self.state == LspSessionStateV1::Cold
            && self.documents.is_empty()
            && self.pending.is_empty()
            && self.cancelled.is_empty()
            && self.completions.is_empty()
            && self.decoder.is_empty()
            && self.bridge_failure.is_none()
            && self.server_info.is_none()
            && self.next_request_id == 2
            && !self.read_index
            && !self.diagnostics
            && !self.symbols
            && !self.definition
            && !self.references
            && !self.implementations
            && !self.call_hierarchy
            && !self.type_at_point
            && !self.rename_plan
            && self.usage == LspUsageV1::default()
            && matches!(
                terminal,
                LspTransportTerminalV1::Active | LspTransportTerminalV1::Unobserved
            );
        if !fresh {
            return Err(Box::new(LspFreshStartFailureV1 {
                reason: LspFreshStartRefusalV1::NotFresh,
                adapter: self,
            }));
        }
        match self.start() {
            LspStartOutcomeV1::Started => Ok(LspFreshStartedAdapterV1 { inner: self }),
            LspStartOutcomeV1::Unavailable => Err(Box::new(LspFreshStartFailureV1 {
                reason: LspFreshStartRefusalV1::RuntimeUnavailable,
                adapter: self,
            })),
            LspStartOutcomeV1::Refused => Err(Box::new(LspFreshStartFailureV1 {
                reason: LspFreshStartRefusalV1::DeliveryRefused,
                adapter: self,
            })),
            LspStartOutcomeV1::AlreadyStarted => unreachable!("fresh adapter starts once"),
        }
    }

    pub fn transport_terminal_state(&self) -> LspTransportTerminalV1 {
        self.transport.terminal_state()
    }

    pub fn register_virtual_document(
        &mut self,
        document: SemanticDocumentRefV1,
        locator: LspVirtualDocumentV1,
    ) -> bool {
        let Some(uri) = self.resolve_document_locator(&locator) else {
            return false;
        };
        if self.documents.contains_key(&document)
            || self.documents.values().any(|state| state.uri == uri)
            || self.documents.len() == self.limits.max_documents
        {
            return false;
        }
        self.documents.insert(
            document,
            DocumentState {
                uri,
                target: SemanticTargetV1 {
                    document,
                    source_revision: vellum_host::semantic::SemanticSourceRevisionV1::new(0),
                },
                language: String::new(),
                source: String::new(),
                version: 0,
                opened: false,
            },
        );
        true
    }

    fn resolve_document_locator(
        &self,
        locator: &LspVirtualDocumentV1,
    ) -> Option<LspResolvedDocumentUriV1> {
        let resolved = match &locator.0 {
            LspDocumentLocatorKindV1::OpaqueUri(uri) => {
                if matches!(
                    self.initialization.provider_options.as_ref(),
                    Some(LspProviderInitializationOptionsV1::RustAnalyzerStandalone(
                        _
                    ))
                ) {
                    return None;
                }
                Some(LspResolvedDocumentUriV1(uri.clone()))
            }
            LspDocumentLocatorKindV1::WorkspaceRelative(relative) => {
                let root = &self.initialization.workspace.as_ref()?.uri;
                resolve_workspace_relative_document_uri(root, relative, self.limits)
            }
        }?;
        Some(resolved)
    }

    pub fn start(&mut self) -> LspStartOutcomeV1 {
        if self.state != LspSessionStateV1::Cold {
            return LspStartOutcomeV1::AlreadyStarted;
        }
        let (root_uri, workspace_folders) = self.initialization.workspace.as_ref().map_or(
            (Value::Null, Value::Null),
            |workspace| {
                (
                    Value::String(workspace.uri.clone()),
                    json!([{"uri": workspace.uri.as_str(), "name": workspace.name.as_str()}]),
                )
            },
        );
        let mut capabilities = json!({
            "general": {"positionEncodings": ["utf-8", "utf-16", "utf-32"]},
            "workspace": {
                "workspaceFolders": true,
                "workspaceEdit": {"documentChanges": true}
            },
            "textDocument": {
                "synchronization": {"didSave": false},
                "diagnostic": {
                    "dynamicRegistration": false,
                    "relatedDocumentSupport": false
                },
                "documentSymbol": {
                    "dynamicRegistration": false,
                    "hierarchicalDocumentSymbolSupport": true
                },
                "definition": {
                    "dynamicRegistration": false,
                    "linkSupport": false
                },
                "references": {"dynamicRegistration": false},
                "implementation": {
                    "dynamicRegistration": false,
                    "linkSupport": false
                },
                "callHierarchy": {"dynamicRegistration": false},
                "hover": {
                    "dynamicRegistration": false,
                    "contentFormat": ["plaintext", "markdown"]
                },
                "rename": {
                    "dynamicRegistration": false,
                    "prepareSupport": false,
                    "honorsChangeAnnotations": false
                }
            }
        });
        if self.initialization.protocol.is_some() {
            capabilities
                .as_object_mut()
                .expect("closed client capabilities are an object")
                .insert(
                    "experimental".to_owned(),
                    json!({"serverStatusNotification": true}),
                );
        }
        let mut params = json!({
            "processId": Value::Null,
            "rootUri": root_uri,
            "capabilities": capabilities,
            "workspaceFolders": workspace_folders
        });
        if let Some(LspProviderInitializationOptionsV1::RustAnalyzerStandalone(options)) =
            self.initialization.provider_options.as_ref()
        {
            params
                .as_object_mut()
                .expect("closed initialize params are an object")
                .insert(
                    "initializationOptions".to_owned(),
                    json!({
                        "linkedProjects": [options.linked_project_path.as_str()],
                        "cargo": {
                            "autoreload": true,
                            "buildScripts": {"enable": false},
                            "noDeps": true,
                            "sysroot": Value::Null
                        },
                        "checkOnSave": false,
                        "procMacro": {"enable": false},
                        "notifications": {"cargoTomlNotFound": false}
                    }),
                );
        }
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": params
        });
        match self.send_json(&body) {
            LspSendOutcomeV1::Accepted => {
                self.state = LspSessionStateV1::Initializing;
                LspStartOutcomeV1::Started
            }
            LspSendOutcomeV1::Unavailable => LspStartOutcomeV1::Unavailable,
            LspSendOutcomeV1::Refused => LspStartOutcomeV1::Refused,
        }
    }

    pub fn begin_shutdown(&mut self) -> LspShutdownOutcomeV1 {
        match self.state {
            LspSessionStateV1::Closed | LspSessionStateV1::Aborted => {
                return LspShutdownOutcomeV1::AlreadyTerminal;
            }
            LspSessionStateV1::Initializing | LspSessionStateV1::ShuttingDown { .. } => {
                return LspShutdownOutcomeV1::Busy;
            }
            LspSessionStateV1::Cold | LspSessionStateV1::Failed => {
                return LspShutdownOutcomeV1::Refused;
            }
            LspSessionStateV1::Indexing { .. }
            | LspSessionStateV1::Ready { .. }
            | LspSessionStateV1::Unavailable { .. } => {}
        }
        if self.documents.values().any(|document| document.opened)
            || !self.pending.is_empty()
            || !self.cancelled.is_empty()
            || !self.completions.is_empty()
        {
            return LspShutdownOutcomeV1::Busy;
        }
        let request_id = self.next_request_id;
        let Some(next_request_id) = request_id.checked_add(1) else {
            return LspShutdownOutcomeV1::Refused;
        };
        let message = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "shutdown",
            "params": Value::Null
        });
        match self.send_json(&message) {
            LspSendOutcomeV1::Accepted => {
                self.next_request_id = next_request_id;
                self.state = LspSessionStateV1::ShuttingDown { request_id };
                bump(&mut self.usage.shutdown_started, &mut self.usage.saturated);
                LspShutdownOutcomeV1::Started
            }
            LspSendOutcomeV1::Unavailable => LspShutdownOutcomeV1::Unavailable,
            LspSendOutcomeV1::Refused => LspShutdownOutcomeV1::Refused,
        }
    }

    pub fn abort(&mut self) -> LspAbortOutcomeV1 {
        if matches!(
            self.state,
            LspSessionStateV1::Closed | LspSessionStateV1::Aborted
        ) {
            return LspAbortOutcomeV1::AlreadyTerminal;
        }
        self.transport.abort();
        self.pending.clear();
        self.cancelled.clear();
        self.completions.clear();
        self.usage.cancelled_correlations_current = 0;
        for document in self.documents.values_mut() {
            document.opened = false;
            document.source.clear();
        }
        self.usage.source_bytes_current = 0;
        self.state = LspSessionStateV1::Aborted;
        bump(&mut self.usage.shutdown_aborted, &mut self.usage.saturated);
        LspAbortOutcomeV1::Aborted
    }

    fn send_json(&mut self, value: &Value) -> LspSendOutcomeV1 {
        let Ok(body) = serde_json::to_vec(value) else {
            bump(&mut self.usage.transport_refused, &mut self.usage.saturated);
            return LspSendOutcomeV1::Refused;
        };
        let Ok(frame) = encode_lsp_frame_v1(&body, self.limits) else {
            bump(&mut self.usage.transport_refused, &mut self.usage.saturated);
            return LspSendOutcomeV1::Refused;
        };
        let outcome = self.transport.try_send(&frame);
        match outcome {
            LspSendOutcomeV1::Accepted => {
                bump(&mut self.usage.frames_sent, &mut self.usage.saturated)
            }
            LspSendOutcomeV1::Unavailable => bump(
                &mut self.usage.transport_unavailable,
                &mut self.usage.saturated,
            ),
            LspSendOutcomeV1::Refused => {
                bump(&mut self.usage.transport_refused, &mut self.usage.saturated)
            }
        }
        outcome
    }

    fn poll_transport(&mut self) {
        match self.transport.poll_receive() {
            LspReceiveOutcomeV1::Pending => {}
            LspReceiveOutcomeV1::Chunk(chunk) => {
                if self.decoder.push(&chunk).is_err() {
                    self.fail_malformed();
                    return;
                }
                loop {
                    match self.decoder.next_frame() {
                        Ok(Some(body)) => self.handle_message(&body),
                        Ok(None) => break,
                        Err(_) => {
                            self.fail_malformed();
                            break;
                        }
                    }
                }
            }
            LspReceiveOutcomeV1::Closed => {
                bump(&mut self.usage.transport_closed, &mut self.usage.saturated);
                if !matches!(
                    self.state,
                    LspSessionStateV1::Closed | LspSessionStateV1::Aborted
                ) {
                    self.mark_failed(SemanticBridgePollFailureV1::RuntimeUnavailable);
                }
            }
            LspReceiveOutcomeV1::Failed => {
                bump(
                    &mut self.usage.transport_unavailable,
                    &mut self.usage.saturated,
                );
                self.mark_failed(SemanticBridgePollFailureV1::RuntimeUnavailable);
            }
        }
    }

    fn poll_index_readiness(&mut self) {
        let (encoding, read_index, was_ready) = match self.state {
            LspSessionStateV1::Indexing {
                encoding,
                read_index,
            }
            | LspSessionStateV1::Unavailable {
                encoding,
                read_index,
            } => (encoding, read_index, false),
            LspSessionStateV1::Ready {
                encoding,
                read_index,
                ..
            } => (encoding, read_index, true),
            _ => return,
        };
        let observed = if self.initialization.readiness_notification_method.is_some() {
            self.last_notification_readiness
                .unwrap_or(LspIndexReadinessV1::Indexing)
        } else {
            self.readiness.poll_readiness()
        };
        match observed {
            LspIndexReadinessV1::Current if !was_ready => {
                let Some(generation) = self.generations.issue() else {
                    self.mark_failed(SemanticBridgePollFailureV1::Poisoned);
                    return;
                };
                self.state = LspSessionStateV1::Ready {
                    generation,
                    encoding,
                    read_index,
                };
                bump(
                    &mut self.usage.readiness_promotions,
                    &mut self.usage.saturated,
                );
            }
            LspIndexReadinessV1::Current => {}
            LspIndexReadinessV1::Indexing => {
                if was_ready {
                    bump(
                        &mut self.usage.readiness_demotions,
                        &mut self.usage.saturated,
                    );
                }
                self.state = LspSessionStateV1::Indexing {
                    encoding,
                    read_index,
                };
            }
            LspIndexReadinessV1::Unavailable => {
                if !matches!(self.state, LspSessionStateV1::Unavailable { .. }) {
                    bump(
                        &mut self.usage.readiness_unavailable,
                        &mut self.usage.saturated,
                    );
                    if was_ready {
                        bump(
                            &mut self.usage.readiness_demotions,
                            &mut self.usage.saturated,
                        );
                    }
                }
                self.state = LspSessionStateV1::Unavailable {
                    encoding,
                    read_index,
                };
            }
        }
    }

    fn handle_message(&mut self, body: &[u8]) {
        bump(&mut self.usage.frames_received, &mut self.usage.saturated);
        let Ok(StrictJsonValue(value)) = serde_json::from_slice::<StrictJsonValue>(body) else {
            self.fail_malformed();
            return;
        };
        if value.get("jsonrpc").and_then(Value::as_str) != Some("2.0") || !value.is_object() {
            self.fail_malformed();
            return;
        }
        if let LspSessionStateV1::ShuttingDown { request_id } = self.state {
            match (value.get("id"), value.get("method")) {
                (None, Some(Value::String(method))) => {
                    self.handle_notification(method, value.get("params"));
                    return;
                }
                (Some(_), Some(_)) => {
                    bump(
                        &mut self.usage.inbound_server_requests,
                        &mut self.usage.saturated,
                    );
                    self.fail_malformed();
                    return;
                }
                (Some(id), None) if id.as_u64() == Some(request_id) => {}
                _ => {
                    self.fail_shutdown(SemanticBridgePollFailureV1::DeliveryRefused);
                    return;
                }
            }
            if value.get("error").is_some() || value.get("result") != Some(&Value::Null) {
                self.fail_shutdown(SemanticBridgePollFailureV1::DeliveryRefused);
                return;
            }
            match self.send_json(&json!({"jsonrpc": "2.0", "method": "exit"})) {
                LspSendOutcomeV1::Accepted => {}
                LspSendOutcomeV1::Unavailable => {
                    self.fail_shutdown(SemanticBridgePollFailureV1::RuntimeUnavailable);
                    return;
                }
                LspSendOutcomeV1::Refused => {
                    self.fail_shutdown(SemanticBridgePollFailureV1::DeliveryRefused);
                    return;
                }
            }
            self.transport.close();
            self.state = LspSessionStateV1::Closed;
            bump(
                &mut self.usage.shutdown_completed,
                &mut self.usage.saturated,
            );
            return;
        }
        if self.state == LspSessionStateV1::Initializing
            && value.get("id") == Some(&json!(1))
            && value.get("method").is_none()
        {
            if value.get("error").is_some() || !value.get("result").is_some_and(Value::is_object) {
                if self.initialization.protocol.is_some() {
                    self.server_identity = LspServerIdentityDispositionV1::PinnedRefused;
                }
                self.mark_failed(SemanticBridgePollFailureV1::DeliveryRefused);
                return;
            }
            if !self.capture_server_info(&value) {
                if self.initialization.protocol.is_some() {
                    self.server_identity = LspServerIdentityDispositionV1::PinnedRefused;
                }
                self.mark_failed(SemanticBridgePollFailureV1::DeliveryRefused);
                return;
            }
            let encoding = match value
                .pointer("/result/capabilities/positionEncoding")
                .and_then(Value::as_str)
                .unwrap_or("utf-16")
            {
                "utf-8" => LspPositionEncodingV1::Utf8,
                "utf-16" => LspPositionEncodingV1::Utf16,
                "utf-32" => LspPositionEncodingV1::Utf32,
                _ => {
                    self.mark_failed(SemanticBridgePollFailureV1::DeliveryRefused);
                    return;
                }
            };
            let capabilities = value
                .pointer("/result/capabilities")
                .unwrap_or(&Value::Null);
            self.diagnostics = matches!(
                capabilities.get("diagnosticProvider"),
                Some(Value::Object(_))
            );
            self.symbols = capability_enabled(capabilities.get("documentSymbolProvider"));
            self.definition = capability_enabled(capabilities.get("definitionProvider"));
            self.references = capability_enabled(capabilities.get("referencesProvider"));
            self.implementations = capability_enabled(capabilities.get("implementationProvider"));
            self.call_hierarchy = capability_enabled(capabilities.get("callHierarchyProvider"));
            self.type_at_point = capability_enabled(capabilities.get("hoverProvider"));
            let read_index = self.diagnostics
                || self.symbols
                || self.definition
                || self.references
                || self.implementations
                || self.call_hierarchy
                || self.type_at_point;
            self.read_index = read_index;
            self.rename_plan = capability_enabled(capabilities.get("renameProvider"));
            match self.send_json(&json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}))
            {
                LspSendOutcomeV1::Accepted => {}
                LspSendOutcomeV1::Unavailable => {
                    self.mark_failed(SemanticBridgePollFailureV1::RuntimeUnavailable);
                    return;
                }
                LspSendOutcomeV1::Refused => {
                    self.mark_failed(SemanticBridgePollFailureV1::DeliveryRefused);
                    return;
                }
            }
            self.state = LspSessionStateV1::Indexing {
                encoding,
                read_index,
            };
        } else {
            match (value.get("id"), value.get("method")) {
                (Some(_), Some(_)) => {
                    bump(
                        &mut self.usage.inbound_server_requests,
                        &mut self.usage.saturated,
                    );
                    self.fail_malformed();
                }
                (Some(id), None) => match id.as_u64() {
                    Some(id) => self.handle_response(id, &value),
                    None => self.fail_malformed(),
                },
                (None, Some(Value::String(method))) => {
                    self.handle_notification(method, value.get("params"));
                }
                (None, Some(_)) => self.fail_malformed(),
                (None, None) => {
                    bump(&mut self.usage.orphan_messages, &mut self.usage.saturated);
                }
            }
        }
    }

    fn capture_server_info(&mut self, value: &Value) -> bool {
        let Some(server_info) = value.pointer("/result/serverInfo") else {
            if self.initialization.protocol.is_some() {
                bump(
                    &mut self.usage.server_info_refused,
                    &mut self.usage.saturated,
                );
                return false;
            }
            self.server_identity = LspServerIdentityDispositionV1::UnpinnedAbsent;
            return true;
        };
        let Some(object) = server_info.as_object() else {
            bump(
                &mut self.usage.server_info_refused,
                &mut self.usage.saturated,
            );
            return false;
        };
        let valid_keys = object.len() <= 2
            && object
                .keys()
                .all(|key| matches!(key.as_str(), "name" | "version"));
        let name = object.get("name").and_then(Value::as_str);
        let version = match object.get("version") {
            None => None,
            Some(Value::String(version)) => Some(version.as_str()),
            Some(_) => {
                bump(
                    &mut self.usage.server_info_refused,
                    &mut self.usage.saturated,
                );
                return false;
            }
        };
        let Some(name) = name else {
            bump(
                &mut self.usage.server_info_refused,
                &mut self.usage.saturated,
            );
            return false;
        };
        if !valid_keys
            || name.is_empty()
            || name.len() > MAX_LSP_SERVER_INFO_NAME_BYTES_V1
            || name.bytes().any(|byte| byte.is_ascii_control())
            || version.is_some_and(|version| {
                version.is_empty()
                    || version.len() > MAX_LSP_SERVER_INFO_VERSION_BYTES_V1
                    || version.bytes().any(|byte| byte.is_ascii_control())
            })
        {
            bump(
                &mut self.usage.server_info_refused,
                &mut self.usage.saturated,
            );
            return false;
        }
        let observed = LspServerInfoV1 {
            name: name.to_owned(),
            version: version.map(str::to_owned),
        };
        if let Some(protocol) = self.initialization.protocol.as_ref() {
            if !observed.matches(
                &protocol.expected_server_name,
                protocol.expected_server_version.as_deref(),
            ) {
                bump(
                    &mut self.usage.server_info_refused,
                    &mut self.usage.saturated,
                );
                return false;
            }
            self.server_identity = LspServerIdentityDispositionV1::PinnedExact;
        } else {
            self.server_identity = LspServerIdentityDispositionV1::UnpinnedObserved;
        }
        self.server_info = Some(observed);
        bump(
            &mut self.usage.server_info_captured,
            &mut self.usage.saturated,
        );
        true
    }

    fn handle_notification(&mut self, method: &str, params: Option<&Value>) {
        let Some(expected) = self.initialization.readiness_notification_method.as_deref() else {
            bump(
                &mut self.usage.notifications_ignored,
                &mut self.usage.saturated,
            );
            return;
        };
        if method != expected {
            bump(
                &mut self.usage.notifications_ignored,
                &mut self.usage.saturated,
            );
            return;
        }
        let schema = self.initialization.protocol.as_ref().map_or(
            LspReadinessNotificationSchemaV1::HealthAndQuiescent,
            |protocol| protocol.readiness_notification_schema,
        );
        let parsed = params.and_then(Value::as_object).and_then(|object| {
            let required = object.contains_key("health") && object.contains_key("quiescent");
            let schema_valid = match schema {
                LspReadinessNotificationSchemaV1::HealthAndQuiescent => object.len() == 2,
                LspReadinessNotificationSchemaV1::HealthAndQuiescentWithOptionalMessage => {
                    (object.len() == 2 || (object.len() == 3 && object.contains_key("message")))
                        && object.get("message").is_none_or(|message| match message {
                            Value::Null => true,
                            Value::String(message) => {
                                message.len() <= MAX_LSP_READINESS_MESSAGE_BYTES_V1
                            }
                            _ => false,
                        })
                }
            };
            (required && schema_valid).then_some(object)
        });
        let health = parsed
            .and_then(|object| object.get("health"))
            .and_then(Value::as_str)
            .and_then(|health| match health {
                "ok" => Some(LspServerHealthV1::Ok),
                "warning" => Some(LspServerHealthV1::Warning),
                "error" => Some(LspServerHealthV1::Error),
                _ => None,
            });
        let quiescent = parsed
            .and_then(|object| object.get("quiescent"))
            .and_then(Value::as_bool);
        let (Some(health), Some(quiescent)) = (health, quiescent) else {
            bump(
                &mut self.usage.notifications_refused,
                &mut self.usage.saturated,
            );
            self.mark_failed(SemanticBridgePollFailureV1::DeliveryRefused);
            return;
        };
        let Some(readiness) = self
            .readiness
            .observe_notification(LspReadinessNotificationV1 { health, quiescent })
        else {
            bump(
                &mut self.usage.notifications_refused,
                &mut self.usage.saturated,
            );
            self.mark_failed(SemanticBridgePollFailureV1::DeliveryRefused);
            return;
        };
        self.last_notification_readiness = Some(readiness);
        bump(
            &mut self.usage.notifications_observed,
            &mut self.usage.saturated,
        );
    }

    fn handle_response(&mut self, id: u64, value: &Value) {
        if let Some(request) = self.cancelled.get(&id).copied() {
            if self.completions.len() == MAX_LSP_PENDING_REQUESTS_V1 {
                self.mark_failed(SemanticBridgePollFailureV1::DeliveryRefused);
                bump(&mut self.usage.output_refused, &mut self.usage.saturated);
                return;
            }
            self.cancelled.remove(&id);
            self.usage.cancelled_correlations_current -= 1;
            self.completions.push_back(SemanticProviderCompletionV1 {
                request,
                // The bridge service owns the cancellation quarantine and
                // classifies this exact correlation as Late before inspecting
                // the payload. No peer result is granted semantic authority.
                outcome: SemanticOutcomeV1::Unavailable,
            });
            bump(
                &mut self.usage.cancelled_responses_correlated,
                &mut self.usage.saturated,
            );
            return;
        }
        let Some(pending) = self.pending.remove(&id) else {
            bump(&mut self.usage.orphan_messages, &mut self.usage.saturated);
            return;
        };
        let has_error = value.get("error").is_some();
        let has_result = value.get("result").is_some();
        let outcome = if has_error == has_result {
            SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply)
        } else if has_error {
            convert_error(&pending.request, value.get("error"))
        } else if let Some(result) = value.get("result") {
            if pending.stage == PendingRequestStage::PreparingCallHierarchy {
                match self.continue_call_hierarchy(pending, result) {
                    Ok(()) => return,
                    Err(failure) => {
                        let (pending, outcome) = *failure;
                        return self.finish_pending(pending, outcome);
                    }
                }
            }
            self.convert_response(&pending, result)
        } else {
            SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply)
        };
        self.finish_pending(pending, outcome);
    }

    fn finish_pending(&mut self, pending: PendingRequest, outcome: SemanticOutcomeV1) {
        if self.completions.len() == MAX_LSP_PENDING_REQUESTS_V1 {
            self.mark_failed(SemanticBridgePollFailureV1::DeliveryRefused);
            bump(&mut self.usage.output_refused, &mut self.usage.saturated);
            return;
        }
        self.completions.push_back(SemanticProviderCompletionV1 {
            request: pending.request.request,
            outcome,
        });
        bump(
            &mut self.usage.requests_completed,
            &mut self.usage.saturated,
        );
    }

    fn continue_call_hierarchy(
        &mut self,
        mut pending: PendingRequest,
        result: &Value,
    ) -> Result<(), Box<(PendingRequest, SemanticOutcomeV1)>> {
        let LspSessionStateV1::Ready {
            generation,
            encoding,
            read_index: true,
        } = self.state
        else {
            return Err(Box::new((pending, SemanticOutcomeV1::Unavailable)));
        };
        if generation != pending.generation
            || self
                .documents
                .get(&pending.request.target.document)
                .is_none_or(|document| {
                    !document.opened || document.target != pending.request.target
                })
        {
            return Err(Box::new((pending, SemanticOutcomeV1::Stale)));
        }
        let items = if result.is_null() {
            &[][..]
        } else if let Some(items) = result.as_array() {
            items.as_slice()
        } else {
            return Err(Box::new((
                pending,
                SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply),
            )));
        };
        if items.is_empty() {
            let result = SemanticResultV1::try_locations(Vec::new(), self.semantic_limits)
                .map_err(|_| {
                    Box::new((
                        pending.clone(),
                        SemanticOutcomeV1::Refused(SemanticRefusalV1::OutputLimit),
                    ))
                })?;
            return Err(Box::new((
                pending.clone(),
                SemanticOutcomeV1::Complete(SemanticReplyV1 {
                    request: pending.request.request,
                    target: pending.request.target,
                    index_generation: pending.generation,
                    provider: self.provenance.clone(),
                    result,
                }),
            )));
        }
        // One point must identify one exact provider-owned item. Selecting from
        // multiple items would silently turn provider identity into text search.
        if items.len() != 1
            || !valid_call_hierarchy_item(
                &items[0],
                &self.documents,
                encoding,
                self.semantic_limits,
            )
        {
            return Err(Box::new((
                pending,
                SemanticOutcomeV1::Refused(if items.len() > 1 {
                    SemanticRefusalV1::OutputLimit
                } else {
                    SemanticRefusalV1::MalformedProviderReply
                }),
            )));
        }
        let id = self.next_request_id;
        let Some(next_id) = id.checked_add(1) else {
            return Err(Box::new((pending, SemanticOutcomeV1::Unavailable)));
        };
        let method = match pending.request.query {
            vellum_host::semantic::SemanticQueryV1::IncomingCalls { .. } => {
                "callHierarchy/incomingCalls"
            }
            vellum_host::semantic::SemanticQueryV1::OutgoingCalls { .. } => {
                "callHierarchy/outgoingCalls"
            }
            _ => unreachable!("only call hierarchy requests enter the prepare stage"),
        };
        let message = json!({
            "jsonrpc":"2.0", "id":id, "method":method,
            "params":{"item":items[0].clone()}
        });
        match self.send_json(&message) {
            LspSendOutcomeV1::Accepted => {
                self.next_request_id = next_id;
                pending.stage = PendingRequestStage::ResolvingCallHierarchy;
                self.pending.insert(id, pending);
                Ok(())
            }
            LspSendOutcomeV1::Unavailable => {
                Err(Box::new((pending, SemanticOutcomeV1::Unavailable)))
            }
            LspSendOutcomeV1::Refused => Err(Box::new((
                pending,
                SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply),
            ))),
        }
    }

    fn convert_response(&mut self, pending: &PendingRequest, result: &Value) -> SemanticOutcomeV1 {
        let request = &pending.request;
        let LspSessionStateV1::Ready {
            generation,
            encoding,
            read_index: true,
        } = self.state
        else {
            return SemanticOutcomeV1::Unavailable;
        };
        if generation != pending.generation {
            return SemanticOutcomeV1::Stale;
        }
        let Some(document) = self.documents.get(&request.target.document) else {
            return SemanticOutcomeV1::Stale;
        };
        if !document.opened || document.target != request.target {
            return SemanticOutcomeV1::Stale;
        }
        if matches!(
            request.query,
            vellum_host::semantic::SemanticQueryV1::TypeAtPoint { .. }
        ) && result.is_null()
        {
            return SemanticOutcomeV1::Unavailable;
        }
        let converted = match &request.query {
            vellum_host::semantic::SemanticQueryV1::Diagnostics => {
                convert_diagnostics(result, document, encoding, self.semantic_limits)
            }
            vellum_host::semantic::SemanticQueryV1::Symbols { needle } => {
                convert_symbols(result, document, encoding, needle, self.semantic_limits)
            }
            vellum_host::semantic::SemanticQueryV1::Definition { .. }
            | vellum_host::semantic::SemanticQueryV1::Implementations { .. } => convert_locations(
                result,
                &self.documents,
                encoding,
                self.semantic_limits,
                true,
            ),
            vellum_host::semantic::SemanticQueryV1::References { .. } => convert_locations(
                result,
                &self.documents,
                encoding,
                self.semantic_limits,
                false,
            ),
            vellum_host::semantic::SemanticQueryV1::IncomingCalls { .. } => convert_call_hierarchy(
                result,
                &self.documents,
                document,
                encoding,
                self.semantic_limits,
                true,
            ),
            vellum_host::semantic::SemanticQueryV1::OutgoingCalls { .. } => convert_call_hierarchy(
                result,
                &self.documents,
                document,
                encoding,
                self.semantic_limits,
                false,
            ),
            vellum_host::semantic::SemanticQueryV1::TypeAtPoint { .. } => {
                convert_hover(result, self.semantic_limits)
            }
            vellum_host::semantic::SemanticQueryV1::PlanRename { .. } => {
                convert_rename_plan(result, &self.documents, encoding, self.semantic_limits)
            }
        };
        match converted {
            Ok(result) => {
                if matches!(
                    request.query,
                    vellum_host::semantic::SemanticQueryV1::PlanRename { .. }
                ) {
                    bump(
                        &mut self.usage.rename_plans_completed,
                        &mut self.usage.saturated,
                    );
                }
                SemanticOutcomeV1::Complete(SemanticReplyV1 {
                    request: request.request,
                    target: request.target,
                    index_generation: pending.generation,
                    provider: self.provenance.clone(),
                    result,
                })
            }
            Err(refusal) => {
                if matches!(
                    request.query,
                    vellum_host::semantic::SemanticQueryV1::PlanRename { .. }
                ) {
                    bump(
                        &mut self.usage.rename_plans_refused,
                        &mut self.usage.saturated,
                    );
                }
                if refusal == SemanticRefusalV1::OutputLimit {
                    bump(&mut self.usage.output_refused, &mut self.usage.saturated);
                } else {
                    bump(
                        &mut self.usage.malformed_messages,
                        &mut self.usage.saturated,
                    );
                }
                SemanticOutcomeV1::Refused(refusal)
            }
        }
    }

    fn fail_malformed(&mut self) {
        bump(
            &mut self.usage.malformed_messages,
            &mut self.usage.saturated,
        );
        self.mark_failed(SemanticBridgePollFailureV1::DeliveryRefused);
    }

    fn fail_shutdown(&mut self, failure: SemanticBridgePollFailureV1) {
        bump(&mut self.usage.shutdown_refused, &mut self.usage.saturated);
        self.mark_failed(failure);
    }

    fn mark_failed(&mut self, failure: SemanticBridgePollFailureV1) {
        if self.bridge_failure.is_none() {
            self.bridge_failure = Some(failure);
        }
        self.state = LspSessionStateV1::Failed;
    }

    fn synchronize_inner(
        &mut self,
        update: &SemanticSnapshotUpdateV1,
    ) -> SemanticProviderAdmissionV1 {
        if !matches!(
            self.state,
            LspSessionStateV1::Indexing { .. } | LspSessionStateV1::Ready { .. }
        ) {
            return SemanticProviderAdmissionV1::Unavailable;
        }
        match update {
            SemanticSnapshotUpdateV1::Admit(snapshot) => {
                let document = snapshot.target().document;
                let (uri, registered) = if let Some(current) = self.documents.get(&document) {
                    (current.uri.clone(), true)
                } else {
                    if self.documents.len() == self.limits.max_documents {
                        return SemanticProviderAdmissionV1::Refused;
                    }
                    let Some(locator) = self.virtual_documents.issue(document) else {
                        return SemanticProviderAdmissionV1::Refused;
                    };
                    let Some(uri) = self.resolve_document_locator(&locator) else {
                        return SemanticProviderAdmissionV1::Refused;
                    };
                    if self.documents.values().any(|current| current.uri == uri) {
                        return SemanticProviderAdmissionV1::Refused;
                    }
                    (uri, false)
                };
                let current = self.documents.get(&document);
                let replaced_bytes = current.map_or(0, |current| current.source.len());
                let current_total = usize::try_from(self.usage.source_bytes_current)
                    .expect("source byte ceiling fits usize");
                let Some(next_total) = current_total
                    .checked_sub(replaced_bytes)
                    .and_then(|value| value.checked_add(snapshot.source().len()))
                    .filter(|value| *value <= self.limits.max_total_source_bytes)
                else {
                    return SemanticProviderAdmissionV1::Refused;
                };
                let opened = current.is_some_and(|current| current.opened);
                let (method, version) = if opened {
                    let current = current.expect("opened document is registered");
                    if snapshot.target().source_revision <= current.target.source_revision {
                        return SemanticProviderAdmissionV1::Refused;
                    }
                    let Some(version) = current
                        .version
                        .checked_add(1)
                        .filter(|value| *value <= i32::MAX as u32)
                    else {
                        return SemanticProviderAdmissionV1::Refused;
                    };
                    ("textDocument/didChange", version)
                } else {
                    ("textDocument/didOpen", 1)
                };
                let params = if opened {
                    json!({
                        "textDocument": {"uri": uri.0, "version": version},
                        "contentChanges": [{"text": snapshot.source()}]
                    })
                } else {
                    json!({
                        "textDocument": {
                            "uri": uri.0,
                            "languageId": snapshot.language(),
                            "version": version,
                            "text": snapshot.source()
                        }
                    })
                };
                let message = json!({"jsonrpc": "2.0", "method": method, "params": params});
                match self.send_json(&message) {
                    LspSendOutcomeV1::Accepted => {
                        if registered {
                            let current = self
                                .documents
                                .get_mut(&document)
                                .expect("registered document remains present");
                            current.target = snapshot.target();
                            current.language = snapshot.language().to_owned();
                            current.source = snapshot.source().to_owned();
                            current.version = version;
                            current.opened = true;
                        } else {
                            self.documents.insert(
                                document,
                                DocumentState {
                                    uri,
                                    target: snapshot.target(),
                                    language: snapshot.language().to_owned(),
                                    source: snapshot.source().to_owned(),
                                    version,
                                    opened: true,
                                },
                            );
                        }
                        self.usage.source_bytes_current = next_total as u64;
                        self.usage.source_bytes_high_water = self
                            .usage
                            .source_bytes_high_water
                            .max(self.usage.source_bytes_current);
                        if version == 1 {
                            bump(&mut self.usage.snapshots_opened, &mut self.usage.saturated);
                        } else {
                            bump(&mut self.usage.snapshots_changed, &mut self.usage.saturated);
                        }
                        SemanticProviderAdmissionV1::Accepted
                    }
                    LspSendOutcomeV1::Unavailable => SemanticProviderAdmissionV1::Unavailable,
                    LspSendOutcomeV1::Refused => SemanticProviderAdmissionV1::Refused,
                }
            }
            SemanticSnapshotUpdateV1::Close { target } => {
                let Some(current) = self.documents.get(&target.document) else {
                    return SemanticProviderAdmissionV1::Refused;
                };
                if !current.opened || current.target != *target {
                    return SemanticProviderAdmissionV1::Refused;
                }
                let message = json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/didClose",
                    "params": {"textDocument": {"uri": current.uri.0}}
                });
                match self.send_json(&message) {
                    LspSendOutcomeV1::Accepted => {
                        let current = self
                            .documents
                            .get_mut(&target.document)
                            .expect("document remains registered");
                        current.opened = false;
                        self.usage.source_bytes_current = self
                            .usage
                            .source_bytes_current
                            .saturating_sub(current.source.len() as u64);
                        current.source.clear();
                        bump(&mut self.usage.snapshots_closed, &mut self.usage.saturated);
                        SemanticProviderAdmissionV1::Accepted
                    }
                    LspSendOutcomeV1::Unavailable => SemanticProviderAdmissionV1::Unavailable,
                    LspSendOutcomeV1::Refused => SemanticProviderAdmissionV1::Refused,
                }
            }
        }
    }
}

impl<
    T: LspTransportPortV1,
    G: SemanticGenerationIssuerV1,
    R: LspIndexReadinessPortV1,
    U: LspVirtualDocumentIssuerV1,
> SemanticProviderPortV1 for LspAdapterV1<T, G, R, U>
{
    fn state(&self) -> SemanticProviderStateV1 {
        match self.state {
            LspSessionStateV1::Initializing | LspSessionStateV1::Indexing { .. } => {
                SemanticProviderStateV1 {
                    provider: self.provenance.clone(),
                    readiness: SemanticProviderReadinessV1::Indexing,
                    capabilities: SemanticCapabilitySetV1::bottom(),
                }
            }
            LspSessionStateV1::Ready {
                generation,
                read_index,
                ..
            } => SemanticProviderStateV1 {
                provider: self.provenance.clone(),
                readiness: SemanticProviderReadinessV1::Ready { generation },
                capabilities: if read_index {
                    let read = SemanticCapabilitySetV1::read_only();
                    if self.rename_plan {
                        read.join(SemanticCapabilitySetV1::singleton(
                            SemanticCapabilityV1::PlanRename,
                        ))
                    } else {
                        read
                    }
                } else {
                    SemanticCapabilitySetV1::bottom()
                },
            },
            LspSessionStateV1::Cold
            | LspSessionStateV1::Unavailable { .. }
            | LspSessionStateV1::ShuttingDown { .. }
            | LspSessionStateV1::Failed
            | LspSessionStateV1::Closed
            | LspSessionStateV1::Aborted => {
                SemanticProviderStateV1::unavailable(self.provenance.clone())
            }
        }
    }

    fn synchronize(&mut self, update: &SemanticSnapshotUpdateV1) -> SemanticProviderAdmissionV1 {
        self.synchronize_inner(update)
    }

    fn submit(&mut self, request: &SemanticRequestV1) -> SemanticProviderSubmissionV1 {
        let LspSessionStateV1::Ready {
            generation,
            read_index,
            ..
        } = self.state
        else {
            return SemanticProviderSubmissionV1::Unavailable;
        };
        let supported = read_index
            && match request.query {
                vellum_host::semantic::SemanticQueryV1::Diagnostics => self.diagnostics,
                vellum_host::semantic::SemanticQueryV1::Symbols { .. } => self.symbols,
                vellum_host::semantic::SemanticQueryV1::Definition { .. } => self.definition,
                vellum_host::semantic::SemanticQueryV1::References { .. } => self.references,
                vellum_host::semantic::SemanticQueryV1::Implementations { .. } => {
                    self.implementations
                }
                vellum_host::semantic::SemanticQueryV1::IncomingCalls { .. }
                | vellum_host::semantic::SemanticQueryV1::OutgoingCalls { .. } => {
                    self.call_hierarchy
                }
                vellum_host::semantic::SemanticQueryV1::TypeAtPoint { .. } => self.type_at_point,
                vellum_host::semantic::SemanticQueryV1::PlanRename { .. } => self.rename_plan,
            };
        if !supported {
            bump(
                &mut self.usage.unsupported_queries,
                &mut self.usage.saturated,
            );
            return SemanticProviderSubmissionV1::Unsupported(request.query.kind());
        }
        if request
            .expected_index
            .is_some_and(|expected| expected != generation)
        {
            return SemanticProviderSubmissionV1::Refused;
        }
        let Some(document) = self.documents.get(&request.target.document) else {
            return SemanticProviderSubmissionV1::Refused;
        };
        if !document.opened
            || document.target != request.target
            || document.language != request.language.as_str()
            || self.pending.len() == MAX_LSP_PENDING_REQUESTS_V1
            || self
                .pending
                .values()
                .any(|pending| pending.request.request == request.request)
        {
            return SemanticProviderSubmissionV1::Refused;
        }
        let id = self.next_request_id;
        let Some(next_id) = id.checked_add(1) else {
            return SemanticProviderSubmissionV1::Refused;
        };
        let method = match request.query {
            vellum_host::semantic::SemanticQueryV1::Diagnostics => "textDocument/diagnostic",
            vellum_host::semantic::SemanticQueryV1::Symbols { .. } => "textDocument/documentSymbol",
            vellum_host::semantic::SemanticQueryV1::Definition { .. } => "textDocument/definition",
            vellum_host::semantic::SemanticQueryV1::References { .. } => "textDocument/references",
            vellum_host::semantic::SemanticQueryV1::Implementations { .. } => {
                "textDocument/implementation"
            }
            vellum_host::semantic::SemanticQueryV1::IncomingCalls { .. }
            | vellum_host::semantic::SemanticQueryV1::OutgoingCalls { .. } => {
                "textDocument/prepareCallHierarchy"
            }
            vellum_host::semantic::SemanticQueryV1::TypeAtPoint { .. } => "textDocument/hover",
            vellum_host::semantic::SemanticQueryV1::PlanRename { .. } => "textDocument/rename",
        };
        let params = match &request.query {
            vellum_host::semantic::SemanticQueryV1::Definition { offset }
            | vellum_host::semantic::SemanticQueryV1::Implementations { offset }
            | vellum_host::semantic::SemanticQueryV1::IncomingCalls { offset }
            | vellum_host::semantic::SemanticQueryV1::OutgoingCalls { offset }
            | vellum_host::semantic::SemanticQueryV1::TypeAtPoint { offset } => {
                let Ok(offset) = usize::try_from(*offset) else {
                    return SemanticProviderSubmissionV1::Refused;
                };
                let Ok(position) = byte_offset_to_lsp_position_v1(
                    &document.source,
                    offset,
                    match self.state {
                        LspSessionStateV1::Ready { encoding, .. } => encoding,
                        _ => unreachable!("ready state checked above"),
                    },
                ) else {
                    return SemanticProviderSubmissionV1::Refused;
                };
                json!({
                    "textDocument": {"uri": document.uri.0},
                    "position": {"line": position.line, "character": position.character}
                })
            }
            vellum_host::semantic::SemanticQueryV1::References { offset } => {
                let Ok(offset) = usize::try_from(*offset) else {
                    return SemanticProviderSubmissionV1::Refused;
                };
                let Ok(position) = byte_offset_to_lsp_position_v1(
                    &document.source,
                    offset,
                    match self.state {
                        LspSessionStateV1::Ready { encoding, .. } => encoding,
                        _ => unreachable!("ready state checked above"),
                    },
                ) else {
                    return SemanticProviderSubmissionV1::Refused;
                };
                json!({
                    "textDocument": {"uri": document.uri.0},
                    "position": {"line": position.line, "character": position.character},
                    "context": {"includeDeclaration": true}
                })
            }
            vellum_host::semantic::SemanticQueryV1::PlanRename { offset, new_name } => {
                let Ok(offset) = usize::try_from(*offset) else {
                    return SemanticProviderSubmissionV1::Refused;
                };
                let Ok(position) = byte_offset_to_lsp_position_v1(
                    &document.source,
                    offset,
                    match self.state {
                        LspSessionStateV1::Ready { encoding, .. } => encoding,
                        _ => unreachable!("ready state checked above"),
                    },
                ) else {
                    return SemanticProviderSubmissionV1::Refused;
                };
                json!({
                    "textDocument": {"uri": document.uri.0},
                    "position": {"line": position.line, "character": position.character},
                    "newName": new_name.as_str()
                })
            }
            _ => json!({"textDocument": {"uri": document.uri.0}}),
        };
        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        match self.send_json(&message) {
            LspSendOutcomeV1::Accepted => {
                self.next_request_id = next_id;
                self.pending.insert(
                    id,
                    PendingRequest {
                        request: request.clone(),
                        generation,
                        stage: if matches!(
                            request.query,
                            vellum_host::semantic::SemanticQueryV1::IncomingCalls { .. }
                                | vellum_host::semantic::SemanticQueryV1::OutgoingCalls { .. }
                        ) {
                            PendingRequestStage::PreparingCallHierarchy
                        } else {
                            PendingRequestStage::Direct
                        },
                    },
                );
                bump(
                    &mut self.usage.requests_submitted,
                    &mut self.usage.saturated,
                );
                if matches!(
                    request.query,
                    vellum_host::semantic::SemanticQueryV1::PlanRename { .. }
                ) {
                    bump(
                        &mut self.usage.rename_requests_submitted,
                        &mut self.usage.saturated,
                    );
                }
                SemanticProviderSubmissionV1::Accepted
            }
            LspSendOutcomeV1::Unavailable => SemanticProviderSubmissionV1::Unavailable,
            LspSendOutcomeV1::Refused => SemanticProviderSubmissionV1::Refused,
        }
    }

    fn cancel(&mut self, request: SemanticRequestRefV1) {
        let id = self
            .pending
            .iter()
            .find_map(|(id, pending)| (pending.request.request == request).then_some(*id));
        let Some(id) = id else {
            return;
        };
        if self.cancelled.len() >= MAX_LSP_PENDING_REQUESTS_V1 {
            bump(&mut self.usage.output_refused, &mut self.usage.saturated);
            self.mark_failed(SemanticBridgePollFailureV1::DeliveryRefused);
            return;
        }
        let Some(pending) = self.pending.remove(&id) else {
            return;
        };
        self.cancelled.insert(id, pending.request.request);
        self.usage.cancelled_correlations_current += 1;
        self.usage.cancelled_correlations_high_water = self
            .usage
            .cancelled_correlations_high_water
            .max(self.usage.cancelled_correlations_current);
        match self.send_json(&json!({
            "jsonrpc": "2.0",
            "method": "$/cancelRequest",
            "params": {"id": id}
        })) {
            LspSendOutcomeV1::Accepted => {}
            LspSendOutcomeV1::Unavailable => {
                self.mark_failed(SemanticBridgePollFailureV1::RuntimeUnavailable);
            }
            LspSendOutcomeV1::Refused => {
                self.mark_failed(SemanticBridgePollFailureV1::DeliveryRefused);
            }
        }
        bump(&mut self.usage.cancellations, &mut self.usage.saturated);
    }

    fn poll(&mut self) -> Option<SemanticProviderCompletionV1> {
        if let Some(completion) = self.completions.pop_front() {
            return Some(completion);
        }
        self.poll_transport();
        self.poll_index_readiness();
        self.completions.pop_front()
    }
}

impl<
    T: LspTransportPortV1,
    G: SemanticGenerationIssuerV1,
    R: LspIndexReadinessPortV1,
    U: LspVirtualDocumentIssuerV1,
> SemanticBridgeProviderPollV1 for LspAdapterV1<T, G, R, U>
{
    fn poll_bridge(
        &mut self,
    ) -> Result<Option<SemanticProviderCompletionV1>, SemanticBridgePollFailureV1> {
        if let Some(failure) = self.bridge_failure {
            return Err(failure);
        }
        if matches!(
            self.state,
            LspSessionStateV1::Closed | LspSessionStateV1::Aborted
        ) {
            return Err(SemanticBridgePollFailureV1::RuntimeUnavailable);
        }
        if self.state == LspSessionStateV1::Failed {
            return Err(SemanticBridgePollFailureV1::Poisoned);
        }
        if let Some(completion) = self.completions.pop_front() {
            return Ok(Some(completion));
        }

        self.poll_transport();
        if let Some(failure) = self.bridge_failure {
            return Err(failure);
        }
        self.poll_index_readiness();
        if let Some(failure) = self.bridge_failure {
            return Err(failure);
        }
        Ok(self.completions.pop_front())
    }
}

impl<
    T: LspTransportPortV1,
    G: SemanticGenerationIssuerV1,
    R: LspIndexReadinessPortV1,
    U: LspVirtualDocumentIssuerV1,
> LspFreshStartedAdapterV1<T, G, R, U>
{
    pub const fn limit_observation(&self) -> LspAdapterLimitObservationV1 {
        self.inner.limit_observation()
    }

    pub const fn session_state(&self) -> LspSessionStateV1 {
        self.inner.session_state()
    }

    pub const fn usage(&self) -> LspUsageV1 {
        self.inner.usage()
    }

    pub fn transport_terminal_state(&self) -> LspTransportTerminalV1 {
        self.inner.transport_terminal_state()
    }

    pub fn begin_shutdown(&mut self) -> LspShutdownOutcomeV1 {
        self.inner.begin_shutdown()
    }

    pub fn abort(&mut self) -> LspAbortOutcomeV1 {
        self.inner.abort()
    }

    /// Transfers the adapter only to a terminal retirement driver. It does not
    /// prove graceful protocol or transport shutdown.
    pub fn into_adapter_for_retirement(self) -> LspAdapterV1<T, G, R, U> {
        self.inner
    }
}

impl<
    T: LspTransportPortV1,
    G: SemanticGenerationIssuerV1,
    R: LspIndexReadinessPortV1,
    U: LspVirtualDocumentIssuerV1,
> SemanticProviderPortV1 for LspFreshStartedAdapterV1<T, G, R, U>
{
    fn state(&self) -> SemanticProviderStateV1 {
        self.inner.state()
    }

    fn synchronize(&mut self, update: &SemanticSnapshotUpdateV1) -> SemanticProviderAdmissionV1 {
        self.inner.synchronize(update)
    }

    fn submit(&mut self, request: &SemanticRequestV1) -> SemanticProviderSubmissionV1 {
        self.inner.submit(request)
    }

    fn cancel(&mut self, request: SemanticRequestRefV1) {
        self.inner.cancel(request);
    }

    fn poll(&mut self) -> Option<SemanticProviderCompletionV1> {
        self.inner.poll()
    }
}

impl<
    T: LspTransportPortV1,
    G: SemanticGenerationIssuerV1,
    R: LspIndexReadinessPortV1,
    U: LspVirtualDocumentIssuerV1,
> SemanticBridgeProviderPollV1 for LspFreshStartedAdapterV1<T, G, R, U>
{
    fn poll_bridge(
        &mut self,
    ) -> Result<Option<SemanticProviderCompletionV1>, SemanticBridgePollFailureV1> {
        self.inner.poll_bridge()
    }
}

impl<T: LspTransportPortV1, G, R, U> Drop for LspAdapterV1<T, G, R, U> {
    fn drop(&mut self) {
        self.transport.close();
    }
}

fn capability_enabled(value: Option<&Value>) -> bool {
    matches!(value, Some(Value::Object(_)) | Some(Value::Bool(true)))
}

fn convert_error(request: &SemanticRequestV1, value: Option<&Value>) -> SemanticOutcomeV1 {
    let Some(error) = value.filter(|value| value.is_object()) else {
        return SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply);
    };
    if error.get("message").and_then(Value::as_str).is_none() {
        return SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply);
    }
    match error.get("code").and_then(Value::as_i64) {
        Some(-32801) => SemanticOutcomeV1::Stale,
        Some(-32601) => SemanticOutcomeV1::Unsupported(request.query.kind()),
        Some(-32600 | -32602) => SemanticOutcomeV1::Refused(SemanticRefusalV1::InvalidRequest),
        Some(_) => SemanticOutcomeV1::Unavailable,
        None => SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply),
    }
}

fn convert_diagnostics(
    result: &Value,
    document: &DocumentState,
    encoding: LspPositionEncodingV1,
    limits: SemanticLimitsV1,
) -> Result<SemanticResultV1, SemanticRefusalV1> {
    if result.get("kind").and_then(Value::as_str) != Some("full") {
        return Err(SemanticRefusalV1::MalformedProviderReply);
    }
    let items = result
        .get("items")
        .and_then(Value::as_array)
        .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
    if items.len() > limits.max_results {
        return Err(SemanticRefusalV1::OutputLimit);
    }
    let mut diagnostics = Vec::with_capacity(items.len());
    for item in items {
        let span = parse_range(item.get("range"), &document.source, encoding)?;
        let message = bounded_text(
            item.get("message").and_then(Value::as_str),
            limits.max_detail_bytes,
        )?;
        let severity = match item.get("severity").and_then(Value::as_u64) {
            Some(1) => SemanticDiagnosticSeverityV1::Error,
            Some(2) => SemanticDiagnosticSeverityV1::Warning,
            Some(3) => SemanticDiagnosticSeverityV1::Information,
            Some(4) => SemanticDiagnosticSeverityV1::Hint,
            _ => return Err(SemanticRefusalV1::MalformedProviderReply),
        };
        let code = match item.get("code") {
            None | Some(Value::Null) => None,
            Some(Value::String(value)) => {
                Some(text_from_owned(value.clone(), limits.max_label_bytes)?)
            }
            Some(Value::Number(value)) => {
                Some(text_from_owned(value.to_string(), limits.max_label_bytes)?)
            }
            _ => return Err(SemanticRefusalV1::MalformedProviderReply),
        };
        diagnostics.push(SemanticDiagnosticV1 {
            severity,
            location: SemanticLocationV1 {
                target: document.target,
                span,
            },
            message,
            code,
        });
    }
    SemanticResultV1::try_diagnostics(diagnostics, limits)
        .map_err(|_| SemanticRefusalV1::OutputLimit)
}

fn convert_symbols(
    result: &Value,
    document: &DocumentState,
    encoding: LspPositionEncodingV1,
    needle: &SemanticTextV1,
    limits: SemanticLimitsV1,
) -> Result<SemanticResultV1, SemanticRefusalV1> {
    let values = result
        .as_array()
        .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
    let mut symbols = Vec::new();
    let mut visited = 0_usize;
    for value in values {
        visit_symbol(
            value,
            document,
            encoding,
            needle.as_str(),
            limits,
            0,
            &mut visited,
            &mut symbols,
        )?;
    }
    SemanticResultV1::try_symbols(symbols, limits).map_err(|_| SemanticRefusalV1::OutputLimit)
}

fn convert_locations(
    result: &Value,
    documents: &BTreeMap<SemanticDocumentRefV1, DocumentState>,
    encoding: LspPositionEncodingV1,
    limits: SemanticLimitsV1,
    allow_single: bool,
) -> Result<SemanticResultV1, SemanticRefusalV1> {
    let values: &[Value] = if result.is_null() {
        &[]
    } else if let Some(values) = result.as_array() {
        values
    } else if allow_single && result.is_object() {
        std::slice::from_ref(result)
    } else {
        return Err(SemanticRefusalV1::MalformedProviderReply);
    };
    if values.len() > limits.max_results {
        return Err(SemanticRefusalV1::OutputLimit);
    }
    let mut locations = Vec::with_capacity(values.len());
    for value in values {
        let location = value
            .as_object()
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
        let uri = location
            .get("uri")
            .and_then(Value::as_str)
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
        let document = documents
            .values()
            .find(|document| document.opened && document.uri.0 == uri)
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
        locations.push(SemanticLocationV1 {
            target: document.target,
            span: parse_range(location.get("range"), &document.source, encoding)?,
        });
    }
    SemanticResultV1::try_locations(locations, limits).map_err(|_| SemanticRefusalV1::OutputLimit)
}

fn valid_call_hierarchy_item(
    value: &Value,
    documents: &BTreeMap<SemanticDocumentRefV1, DocumentState>,
    encoding: LspPositionEncodingV1,
    limits: SemanticLimitsV1,
) -> bool {
    let Some(item) = value.as_object() else {
        return false;
    };
    let bounded = serde_json::to_vec(value)
        .ok()
        .is_some_and(|bytes| bytes.len() <= MAX_LSP_CALL_HIERARCHY_ITEM_BYTES_V1);
    let name = item.get("name").and_then(Value::as_str);
    let kind = item.get("kind").and_then(Value::as_u64);
    let uri = item.get("uri").and_then(Value::as_str);
    let detail_valid = item.get("detail").is_none_or(|detail| {
        detail
            .as_str()
            .is_some_and(|value| value.len() <= limits.max_detail_bytes)
    });
    let Some(document) = uri.and_then(|uri| {
        documents
            .values()
            .find(|document| document.opened && document.uri.0 == uri)
    }) else {
        return false;
    };
    bounded
        && name.is_some_and(|value| !value.is_empty() && value.len() <= limits.max_label_bytes)
        && kind.is_some_and(|value| (1..=26).contains(&value))
        && detail_valid
        && parse_range(item.get("range"), &document.source, encoding).is_ok()
        && parse_range(item.get("selectionRange"), &document.source, encoding).is_ok()
}

fn convert_call_hierarchy(
    result: &Value,
    documents: &BTreeMap<SemanticDocumentRefV1, DocumentState>,
    origin: &DocumentState,
    encoding: LspPositionEncodingV1,
    limits: SemanticLimitsV1,
    incoming: bool,
) -> Result<SemanticResultV1, SemanticRefusalV1> {
    let values: &[Value] = if result.is_null() {
        &[]
    } else {
        result
            .as_array()
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?
    };
    if values.len() > limits.max_results {
        return Err(SemanticRefusalV1::OutputLimit);
    }
    let mut locations = Vec::with_capacity(values.len());
    let mut total_from_ranges = 0usize;
    for value in values {
        let call = value
            .as_object()
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
        let item = call
            .get(if incoming { "from" } else { "to" })
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
        if !valid_call_hierarchy_item(item, documents, encoding, limits) {
            return Err(SemanticRefusalV1::MalformedProviderReply);
        }
        let item = item
            .as_object()
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
        let uri = item
            .get("uri")
            .and_then(Value::as_str)
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
        let document = documents
            .values()
            .find(|document| document.opened && document.uri.0 == uri)
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
        let from_ranges = call
            .get("fromRanges")
            .and_then(Value::as_array)
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
        total_from_ranges = total_from_ranges
            .checked_add(from_ranges.len())
            .ok_or(SemanticRefusalV1::OutputLimit)?;
        if total_from_ranges > limits.max_results {
            return Err(SemanticRefusalV1::OutputLimit);
        }
        let range_document = if incoming { document } else { origin };
        if from_ranges
            .iter()
            .any(|range| parse_range(Some(range), &range_document.source, encoding).is_err())
        {
            return Err(SemanticRefusalV1::MalformedProviderReply);
        }
        locations.push(SemanticLocationV1 {
            target: document.target,
            span: parse_range(item.get("selectionRange"), &document.source, encoding)?,
        });
    }
    SemanticResultV1::try_locations(locations, limits).map_err(|_| SemanticRefusalV1::OutputLimit)
}

fn convert_hover(
    result: &Value,
    limits: SemanticLimitsV1,
) -> Result<SemanticResultV1, SemanticRefusalV1> {
    let hover = result
        .as_object()
        .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
    let contents = hover
        .get("contents")
        .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
    let value = if let Some(value) = contents.as_str() {
        value
    } else {
        let markup = contents
            .as_object()
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
        if !matches!(
            markup.get("kind").and_then(Value::as_str),
            Some("plaintext" | "markdown")
        ) {
            return Err(SemanticRefusalV1::MalformedProviderReply);
        }
        markup
            .get("value")
            .and_then(Value::as_str)
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?
    };
    let text = SemanticTextV1::try_new(value.to_owned(), limits.max_detail_bytes)
        .map_err(|_| SemanticRefusalV1::OutputLimit)?;
    SemanticResultV1::try_type_at_point(text, limits).map_err(|_| SemanticRefusalV1::OutputLimit)
}

fn convert_rename_plan(
    result: &Value,
    documents: &BTreeMap<SemanticDocumentRefV1, DocumentState>,
    encoding: LspPositionEncodingV1,
    limits: SemanticLimitsV1,
) -> Result<SemanticResultV1, SemanticRefusalV1> {
    // LSP permits `null` when no rename can be produced. Preserve that exact
    // meaning as a reviewable empty plan rather than manufacturing an error or
    // edit authority.
    if result.is_null() {
        return SemanticResultV1::try_rename_plan(Vec::new(), limits)
            .map_err(|_| SemanticRefusalV1::OutputLimit);
    }
    let workspace_edit = result
        .as_object()
        .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
    if workspace_edit.len() != 1 || !workspace_edit.contains_key("documentChanges") {
        return Err(SemanticRefusalV1::MalformedProviderReply);
    }
    let document_changes = workspace_edit
        .get("documentChanges")
        .and_then(Value::as_array)
        .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
    let mut plan = Vec::new();
    let mut spans_by_document: BTreeMap<SemanticDocumentRefV1, Vec<SemanticSpanV1>> =
        BTreeMap::new();
    for change in document_changes {
        let change = change
            .as_object()
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
        // Resource operations, annotated edits and extension members are not
        // in the accepted subalgebra.
        if change.len() != 2
            || !change.contains_key("textDocument")
            || !change.contains_key("edits")
        {
            return Err(SemanticRefusalV1::MalformedProviderReply);
        }
        let text_document = change
            .get("textDocument")
            .and_then(Value::as_object)
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
        if text_document.len() != 2
            || !text_document.contains_key("uri")
            || !text_document.contains_key("version")
        {
            return Err(SemanticRefusalV1::MalformedProviderReply);
        }
        let uri = text_document
            .get("uri")
            .and_then(Value::as_str)
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
        let version = text_document
            .get("version")
            .and_then(Value::as_u64)
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
        let (document_ref, document) = documents
            .iter()
            .find(|(_, document)| document.uri.0 == uri)
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
        if !document.opened || version != u64::from(document.version) {
            return Err(SemanticRefusalV1::MalformedProviderReply);
        }
        let edits = change
            .get("edits")
            .and_then(Value::as_array)
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
        if plan
            .len()
            .checked_add(edits.len())
            .is_none_or(|count| count > limits.max_rename_edits)
        {
            return Err(SemanticRefusalV1::OutputLimit);
        }
        for edit in edits {
            let edit = edit
                .as_object()
                .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
            if edit.len() != 2 || !edit.contains_key("range") || !edit.contains_key("newText") {
                return Err(SemanticRefusalV1::MalformedProviderReply);
            }
            let span = parse_strict_range(edit.get("range"), &document.source, encoding)?;
            let replacement = SemanticTextV1::try_new(
                edit.get("newText")
                    .and_then(Value::as_str)
                    .ok_or(SemanticRefusalV1::MalformedProviderReply)?
                    .to_owned(),
                limits.max_label_bytes,
            )
            .map_err(|_| SemanticRefusalV1::OutputLimit)?;
            spans_by_document
                .entry(*document_ref)
                .or_default()
                .push(span);
            plan.push(SemanticTextEditV1 {
                location: SemanticLocationV1 {
                    target: document.target,
                    span,
                },
                replacement,
            });
        }
    }
    for spans in spans_by_document.values_mut() {
        spans.sort_unstable_by_key(|span| (span.start, span.end));
        if spans
            .windows(2)
            .any(|pair| pair[1].start < pair[0].end || pair[1].start == pair[0].start)
        {
            return Err(SemanticRefusalV1::MalformedProviderReply);
        }
    }
    SemanticResultV1::try_rename_plan(plan, limits).map_err(|_| SemanticRefusalV1::OutputLimit)
}

fn parse_strict_range(
    range: Option<&Value>,
    source: &str,
    encoding: LspPositionEncodingV1,
) -> Result<SemanticSpanV1, SemanticRefusalV1> {
    let range = range
        .and_then(Value::as_object)
        .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
    if range.len() != 2 || !range.contains_key("start") || !range.contains_key("end") {
        return Err(SemanticRefusalV1::MalformedProviderReply);
    }
    for position in [range.get("start"), range.get("end")] {
        let position = position
            .and_then(Value::as_object)
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
        if position.len() != 2
            || !position.contains_key("line")
            || !position.contains_key("character")
        {
            return Err(SemanticRefusalV1::MalformedProviderReply);
        }
    }
    parse_range(Some(&Value::Object(range.clone())), source, encoding)
}

#[allow(clippy::too_many_arguments)]
fn visit_symbol(
    value: &Value,
    document: &DocumentState,
    encoding: LspPositionEncodingV1,
    needle: &str,
    limits: SemanticLimitsV1,
    depth: usize,
    visited: &mut usize,
    output: &mut Vec<SemanticSymbolV1>,
) -> Result<(), SemanticRefusalV1> {
    const MAX_DEPTH: usize = 64;
    const MAX_VISITED: usize = 4_096;
    if depth > MAX_DEPTH || *visited == MAX_VISITED {
        return Err(SemanticRefusalV1::OutputLimit);
    }
    *visited += 1;
    let name_value = value
        .get("name")
        .and_then(Value::as_str)
        .ok_or(SemanticRefusalV1::MalformedProviderReply)?;
    let full_span = parse_range(value.get("range"), &document.source, encoding)?;
    let selection = parse_range(value.get("selectionRange"), &document.source, encoding)?;
    if selection.start < full_span.start || selection.end > full_span.end {
        return Err(SemanticRefusalV1::MalformedProviderReply);
    }
    if name_value.contains(needle) {
        if output.len() == limits.max_results {
            return Err(SemanticRefusalV1::OutputLimit);
        }
        output.push(SemanticSymbolV1 {
            name: text_from_owned(name_value.to_owned(), limits.max_label_bytes)?,
            kind: symbol_kind(
                value
                    .get("kind")
                    .and_then(Value::as_u64)
                    .ok_or(SemanticRefusalV1::MalformedProviderReply)?,
            )
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?,
            location: SemanticLocationV1 {
                target: document.target,
                span: selection,
            },
        });
    }
    if let Some(children) = value.get("children") {
        for child in children
            .as_array()
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?
        {
            visit_symbol(
                child,
                document,
                encoding,
                needle,
                limits,
                depth + 1,
                visited,
                output,
            )?;
        }
    }
    Ok(())
}

fn parse_range(
    range: Option<&Value>,
    source: &str,
    encoding: LspPositionEncodingV1,
) -> Result<SemanticSpanV1, SemanticRefusalV1> {
    let range = range.ok_or(SemanticRefusalV1::MalformedProviderReply)?;
    let start = parse_position(range.get("start"))?;
    let end = parse_position(range.get("end"))?;
    let start = lsp_position_to_byte_offset_v1(source, start, encoding)
        .map_err(|_| SemanticRefusalV1::MalformedProviderReply)?;
    let end = lsp_position_to_byte_offset_v1(source, end, encoding)
        .map_err(|_| SemanticRefusalV1::MalformedProviderReply)?;
    SemanticSpanV1::try_new(start as u64, end as u64)
        .map_err(|_| SemanticRefusalV1::MalformedProviderReply)
}

fn parse_position(value: Option<&Value>) -> Result<LspPositionV1, SemanticRefusalV1> {
    let value = value.ok_or(SemanticRefusalV1::MalformedProviderReply)?;
    Ok(LspPositionV1 {
        line: u32::try_from(
            value
                .get("line")
                .and_then(Value::as_u64)
                .ok_or(SemanticRefusalV1::MalformedProviderReply)?,
        )
        .map_err(|_| SemanticRefusalV1::MalformedProviderReply)?,
        character: u32::try_from(
            value
                .get("character")
                .and_then(Value::as_u64)
                .ok_or(SemanticRefusalV1::MalformedProviderReply)?,
        )
        .map_err(|_| SemanticRefusalV1::MalformedProviderReply)?,
    })
}

fn bounded_text(value: Option<&str>, max: usize) -> Result<SemanticTextV1, SemanticRefusalV1> {
    text_from_owned(
        value
            .ok_or(SemanticRefusalV1::MalformedProviderReply)?
            .to_owned(),
        max,
    )
}

fn text_from_owned(value: String, max: usize) -> Result<SemanticTextV1, SemanticRefusalV1> {
    if value.is_empty() {
        return Err(SemanticRefusalV1::MalformedProviderReply);
    }
    SemanticTextV1::try_new(value, max).map_err(|_| SemanticRefusalV1::OutputLimit)
}

const fn symbol_kind(kind: u64) -> Option<SemanticSymbolKindV1> {
    Some(match kind {
        1..=4 => SemanticSymbolKindV1::Module,
        5 | 10 | 11 | 23 | 26 => SemanticSymbolKindV1::Type,
        9 | 12 => SemanticSymbolKindV1::Function,
        6 => SemanticSymbolKindV1::Method,
        7 | 8 => SemanticSymbolKindV1::Field,
        13..=22 => SemanticSymbolKindV1::Value,
        24 | 25 => SemanticSymbolKindV1::Unknown,
        _ => return None,
    })
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn split_once(bytes: &[u8], separator: u8) -> Option<(&[u8], &[u8])> {
    let index = bytes.iter().position(|byte| *byte == separator)?;
    Some((&bytes[..index], &bytes[index + 1..]))
}

fn trim_ascii(mut bytes: &[u8]) -> &[u8] {
    while bytes.first().is_some_and(u8::is_ascii_whitespace) {
        bytes = &bytes[1..];
    }
    while bytes.last().is_some_and(u8::is_ascii_whitespace) {
        bytes = &bytes[..bytes.len() - 1];
    }
    bytes
}

fn line_for_offset(
    source: &str,
    offset: usize,
) -> Result<(usize, usize, usize), LspPositionRefusalV1> {
    let mut start = 0_usize;
    let mut line = 0_usize;
    for (index, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            let end = if index > start && source.as_bytes()[index - 1] == b'\r' {
                index - 1
            } else {
                index
            };
            if offset <= end {
                return Ok((line, start, end));
            }
            if offset == index && end != index {
                return Err(LspPositionRefusalV1::CrlfInterior);
            }
            start = index + 1;
            line = line.saturating_add(1);
        }
    }
    Ok((line, start, source.len()))
}

fn line_bounds(source: &str, wanted: usize) -> Option<(usize, usize)> {
    let mut start = 0_usize;
    let mut line = 0_usize;
    for (index, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            let end = if index > start && source.as_bytes()[index - 1] == b'\r' {
                index - 1
            } else {
                index
            };
            if line == wanted {
                return Some((start, end));
            }
            start = index + 1;
            line = line.saturating_add(1);
        }
    }
    (line == wanted).then_some((start, source.len()))
}

fn units(prefix: &str, encoding: LspPositionEncodingV1) -> Result<u32, LspPositionRefusalV1> {
    match encoding {
        LspPositionEncodingV1::Utf8 => {
            u32::try_from(prefix.len()).map_err(|_| LspPositionRefusalV1::CharacterLimit)
        }
        LspPositionEncodingV1::Utf16 => u32::try_from(prefix.encode_utf16().count())
            .map_err(|_| LspPositionRefusalV1::CharacterLimit),
        LspPositionEncodingV1::Utf32 => {
            u32::try_from(prefix.chars().count()).map_err(|_| LspPositionRefusalV1::CharacterLimit)
        }
    }
}

const fn scalar_units(value: char, encoding: LspPositionEncodingV1) -> u32 {
    match encoding {
        LspPositionEncodingV1::Utf8 => value.len_utf8() as u32,
        LspPositionEncodingV1::Utf16 => value.len_utf16() as u32,
        LspPositionEncodingV1::Utf32 => 1,
    }
}

fn bump(value: &mut u64, saturated: &mut bool) {
    let prior = *value;
    *value = value.saturating_add(1);
    *saturated |= prior == u64::MAX;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use vellum_host::semantic::{SemanticProviderRefV1, SemanticSourceRevisionV1, SemanticTextV1};
    use vellum_semantic_service::SemanticSnapshotV1;

    #[derive(Default)]
    struct MemoryTransport {
        sends: Vec<Vec<u8>>,
        receives: VecDeque<LspReceiveOutcomeV1>,
        next_send: Option<LspSendOutcomeV1>,
        receive_polls: u64,
        closed: bool,
    }

    impl LspTransportPortV1 for MemoryTransport {
        fn try_send(&mut self, frame: &[u8]) -> LspSendOutcomeV1 {
            let outcome = self.next_send.take().unwrap_or(LspSendOutcomeV1::Accepted);
            if outcome == LspSendOutcomeV1::Accepted {
                self.sends.push(frame.to_vec());
            }
            outcome
        }

        fn poll_receive(&mut self) -> LspReceiveOutcomeV1 {
            self.receive_polls += 1;
            self.receives
                .pop_front()
                .unwrap_or(LspReceiveOutcomeV1::Pending)
        }

        fn close(&mut self) {
            self.closed = true;
        }
    }

    struct Issuer(u64);

    impl SemanticGenerationIssuerV1 for Issuer {
        fn issue(&mut self) -> Option<SemanticIndexGenerationV1> {
            self.0 = self.0.checked_add(1)?;
            SemanticIndexGenerationV1::new(self.0)
        }
    }

    struct VirtualIssuer {
        issued: usize,
        uris: VecDeque<LspVirtualDocumentV1>,
        assignments: BTreeMap<SemanticDocumentRefV1, LspVirtualDocumentV1>,
    }

    impl VirtualIssuer {
        fn new(values: &[&str]) -> Self {
            Self {
                issued: 0,
                uris: values
                    .iter()
                    .map(|value| {
                        LspVirtualDocumentV1::try_new((*value).to_owned(), LspLimitsV1::reference())
                            .unwrap()
                    })
                    .collect(),
                assignments: BTreeMap::new(),
            }
        }
    }

    impl LspVirtualDocumentIssuerV1 for VirtualIssuer {
        fn issue(&mut self, document: SemanticDocumentRefV1) -> Option<LspVirtualDocumentV1> {
            if let Some(uri) = self.assignments.get(&document) {
                return Some(uri.clone());
            }
            let uri = self.uris.pop_front()?;
            self.issued += 1;
            self.assignments.insert(document, uri.clone());
            Some(uri)
        }
    }

    struct ScriptedReadiness {
        outcomes: VecDeque<LspIndexReadinessV1>,
        last: LspIndexReadinessV1,
        polls: u64,
        notifications: Vec<LspReadinessNotificationV1>,
    }

    impl ScriptedReadiness {
        fn current() -> Self {
            Self {
                outcomes: VecDeque::from([LspIndexReadinessV1::Current]),
                last: LspIndexReadinessV1::Current,
                polls: 0,
                notifications: Vec::new(),
            }
        }
    }

    impl LspIndexReadinessPortV1 for ScriptedReadiness {
        fn poll_readiness(&mut self) -> LspIndexReadinessV1 {
            self.polls += 1;
            if let Some(outcome) = self.outcomes.pop_front() {
                self.last = outcome;
            }
            self.last
        }

        fn observe_notification(
            &mut self,
            notification: LspReadinessNotificationV1,
        ) -> Option<LspIndexReadinessV1> {
            self.last = match (notification.health, notification.quiescent) {
                (LspServerHealthV1::Ok, true) => LspIndexReadinessV1::Current,
                (LspServerHealthV1::Error, _) => LspIndexReadinessV1::Unavailable,
                _ => LspIndexReadinessV1::Indexing,
            };
            self.notifications.push(notification);
            Some(self.last)
        }
    }

    fn framed(value: Value) -> Vec<u8> {
        encode_lsp_frame_v1(
            &serde_json::to_vec(&value).unwrap(),
            LspLimitsV1::reference(),
        )
        .unwrap()
    }

    fn framed_raw(value: &str) -> Vec<u8> {
        encode_lsp_frame_v1(value.as_bytes(), LspLimitsV1::reference()).unwrap()
    }

    fn framed_json(frame: &[u8]) -> Value {
        let body = find_header_end(frame).expect("complete frame") + 4;
        serde_json::from_slice(&frame[body..]).unwrap()
    }

    fn target(revision: u64) -> SemanticTargetV1 {
        SemanticTargetV1 {
            document: SemanticDocumentRefV1::new([7; 16]),
            source_revision: SemanticSourceRevisionV1::new(revision),
        }
    }

    fn provenance() -> SemanticProviderProvenanceV1 {
        provenance_with(9)
    }

    fn provenance_with(provider: u8) -> SemanticProviderProvenanceV1 {
        SemanticProviderProvenanceV1::try_new(
            SemanticProviderRefV1::new([provider; 16]),
            "rust-test".to_owned(),
            "1.0.0".to_owned(),
        )
        .unwrap()
    }

    fn ready_adapter() -> LspAdapterV1<MemoryTransport, Issuer, ScriptedReadiness> {
        let mut transport = MemoryTransport::default();
        transport.receives.push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"capabilities": {
                "positionEncoding": "utf-16",
                "diagnosticProvider": {"interFileDependencies": false, "workspaceDiagnostics": false},
                "documentSymbolProvider": true,
                "definitionProvider": true,
                "referencesProvider": true,
                "implementationProvider": true,
                "callHierarchyProvider": true,
                "hoverProvider": true,
                "renameProvider": true
            }}
        }))));
        let mut adapter = LspAdapterV1::try_new_with_readiness(
            transport,
            Issuer(0),
            ScriptedReadiness::current(),
            provenance(),
            LspLimitsV1::reference(),
            SemanticLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(adapter.start(), LspStartOutcomeV1::Started);
        assert_eq!(adapter.poll(), None);
        adapter
    }

    fn ready_dynamic_adapter(
        issuer: VirtualIssuer,
        limits: LspLimitsV1,
    ) -> LspAdapterV1<MemoryTransport, Issuer, ScriptedReadiness, VirtualIssuer> {
        ready_dynamic_adapter_with(issuer, limits, 9, 0)
    }

    fn ready_dynamic_adapter_with(
        issuer: VirtualIssuer,
        limits: LspLimitsV1,
        provider: u8,
        prior_generation: u64,
    ) -> LspAdapterV1<MemoryTransport, Issuer, ScriptedReadiness, VirtualIssuer> {
        let mut transport = MemoryTransport::default();
        transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {"capabilities": {"positionEncoding": "utf-16"}}
            }))));
        let mut adapter = LspAdapterV1::try_new_with_readiness_and_documents(
            transport,
            Issuer(prior_generation),
            ScriptedReadiness::current(),
            issuer,
            provenance_with(provider),
            limits,
            SemanticLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(adapter.start(), LspStartOutcomeV1::Started);
        assert_eq!(adapter.poll(), None);
        adapter
    }

    fn open_document(adapter: &mut LspAdapterV1<MemoryTransport, Issuer, ScriptedReadiness>) {
        let uri = LspVirtualDocumentV1::try_new(
            "vellum-lsp://seven".to_owned(),
            LspLimitsV1::reference(),
        )
        .unwrap();
        assert!(adapter.register_virtual_document(target(0).document, uri));
        let snapshot = SemanticSnapshotV1::try_new(
            target(0),
            "rust".to_owned(),
            "fn alpha() {}\n".to_owned(),
            vellum_semantic_service::SemanticServiceLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(
            adapter.synchronize(&SemanticSnapshotUpdateV1::Admit(snapshot)),
            SemanticProviderAdmissionV1::Accepted
        );
    }

    fn rename_request(tag: u8, offset: u64) -> SemanticRequestV1 {
        SemanticRequestV1 {
            request: SemanticRequestRefV1::new([tag; 16]),
            target: target(0),
            expected_index: SemanticIndexGenerationV1::new(1),
            language: SemanticTextV1::try_new("rust".to_owned(), 16).unwrap(),
            query: vellum_host::semantic::SemanticQueryV1::PlanRename {
                offset,
                new_name: SemanticTextV1::try_new("beta".to_owned(), 16).unwrap(),
            },
        }
    }

    fn diagnostics_request(tag: u8) -> SemanticRequestV1 {
        SemanticRequestV1 {
            request: SemanticRequestRefV1::new([tag; 16]),
            target: target(0),
            expected_index: SemanticIndexGenerationV1::new(1),
            language: SemanticTextV1::try_new("rust".to_owned(), 16).unwrap(),
            query: vellum_host::semantic::SemanticQueryV1::Diagnostics,
        }
    }

    fn diagnostics_request_numbered(number: u64) -> SemanticRequestV1 {
        let mut request = [0_u8; 16];
        request[..8].copy_from_slice(&number.to_be_bytes());
        SemanticRequestV1 {
            request: SemanticRequestRefV1::new(request),
            ..diagnostics_request(0)
        }
    }

    fn point_request(tag: u8, query: vellum_host::semantic::SemanticQueryV1) -> SemanticRequestV1 {
        SemanticRequestV1 {
            request: SemanticRequestRefV1::new([tag; 16]),
            target: target(0),
            expected_index: SemanticIndexGenerationV1::new(1),
            language: SemanticTextV1::try_new("rust".to_owned(), 16).unwrap(),
            query,
        }
    }

    #[test]
    fn limit_observation_projects_only_exact_content_free_construction_facts() {
        let semantic = SemanticLimitsV1 {
            max_results: 7,
            ..SemanticLimitsV1::reference()
        };
        let framing = LspLimitsV1 {
            max_documents: 9,
            ..LspLimitsV1::reference()
        };
        let adapter = LspAdapterV1::try_new_with_semantic_limits(
            MemoryTransport::default(),
            Issuer(0),
            provenance(),
            framing,
            semantic,
        )
        .unwrap();
        let observation = adapter.limit_observation();
        assert_eq!(observation.framing(), framing);
        assert_eq!(observation.semantic(), semantic);
        let debug = format!("{observation:?}");
        assert!(!debug.contains("rust-test"));
        assert!(!debug.contains("1.0.0"));
    }

    #[test]
    fn fresh_start_typestate_rejects_used_buffered_terminal_and_refused_adapters() {
        let fresh = LspAdapterV1::try_new(
            MemoryTransport::default(),
            Issuer(0),
            provenance(),
            LspLimitsV1::reference(),
        )
        .unwrap()
        .try_start_fresh()
        .unwrap();
        assert_eq!(fresh.session_state(), LspSessionStateV1::Initializing);
        assert_eq!(fresh.usage().frames_sent, 1);
        assert_eq!(format!("{fresh:?}"), "LspFreshStartedAdapterV1(<opaque>)");

        let mut used = LspAdapterV1::try_new(
            MemoryTransport::default(),
            Issuer(0),
            provenance(),
            LspLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(used.start(), LspStartOutcomeV1::Started);
        let failure = used.try_start_fresh().unwrap_err();
        assert_eq!(failure.reason(), LspFreshStartRefusalV1::NotFresh);
        assert_eq!(
            (*failure).into_adapter().session_state(),
            LspSessionStateV1::Initializing
        );

        let mut registered = LspAdapterV1::try_new(
            MemoryTransport::default(),
            Issuer(0),
            provenance(),
            LspLimitsV1::reference(),
        )
        .unwrap();
        assert!(
            registered.register_virtual_document(
                SemanticDocumentRefV1::new([4; 16]),
                LspVirtualDocumentV1::try_new(
                    "vellum-lsp://registered".into(),
                    LspLimitsV1::reference(),
                )
                .unwrap(),
            )
        );
        assert_eq!(
            registered.try_start_fresh().unwrap_err().reason(),
            LspFreshStartRefusalV1::NotFresh
        );

        let mut transport = MemoryTransport::default();
        transport.receives.push_back(LspReceiveOutcomeV1::Chunk(
            b"Content-Length: 5\r\n\r\n{".to_vec(),
        ));
        let mut buffered =
            LspAdapterV1::try_new(transport, Issuer(0), provenance(), LspLimitsV1::reference())
                .unwrap();
        assert_eq!(buffered.poll(), None);
        assert_eq!(
            buffered.try_start_fresh().unwrap_err().reason(),
            LspFreshStartRefusalV1::NotFresh
        );

        let mut terminal = LspAdapterV1::try_new(
            MemoryTransport::default(),
            Issuer(0),
            provenance(),
            LspLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(terminal.abort(), LspAbortOutcomeV1::Aborted);
        assert_eq!(
            terminal.try_start_fresh().unwrap_err().reason(),
            LspFreshStartRefusalV1::NotFresh
        );

        for (send, expected) in [
            (
                LspSendOutcomeV1::Unavailable,
                LspFreshStartRefusalV1::RuntimeUnavailable,
            ),
            (
                LspSendOutcomeV1::Refused,
                LspFreshStartRefusalV1::DeliveryRefused,
            ),
        ] {
            let transport = MemoryTransport {
                next_send: Some(send),
                ..MemoryTransport::default()
            };
            let failure =
                LspAdapterV1::try_new(transport, Issuer(0), provenance(), LspLimitsV1::reference())
                    .unwrap()
                    .try_start_fresh()
                    .unwrap_err();
            assert_eq!(failure.reason(), expected);
            let debug = format!("{failure:?}");
            assert!(!debug.contains("rust-test"));
            let _retained = (*failure).into_adapter();
        }
    }

    #[test]
    fn bridge_poll_distinguishes_absence_completion_and_delivery_obligation() {
        let mut adapter = ready_adapter();
        let before_absence = adapter.transport.receive_polls;
        assert_eq!(adapter.poll_bridge(), Ok(None));
        assert_eq!(adapter.transport.receive_polls, before_absence + 1);

        open_document(&mut adapter);
        let request = diagnostics_request(51);
        assert_eq!(
            adapter.submit(&request),
            SemanticProviderSubmissionV1::Accepted
        );
        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0",
                "id": 2,
                "result": {"kind": "full", "items": []}
            }))));
        let before_completion = adapter.transport.receive_polls;
        let completion = adapter.poll_bridge().unwrap().unwrap();
        assert_eq!(completion.request, request.request);
        assert!(matches!(completion.outcome, SemanticOutcomeV1::Complete(_)));
        assert_eq!(adapter.transport.receive_polls, before_completion + 1);

        let queued = SemanticProviderCompletionV1 {
            request: SemanticRequestRefV1::new([52; 16]),
            outcome: SemanticOutcomeV1::Unavailable,
        };
        adapter.completions.push_back(queued.clone());
        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Closed);
        let before_queued = adapter.transport.receive_polls;
        assert_eq!(adapter.poll_bridge(), Ok(Some(queued)));
        assert_eq!(adapter.transport.receive_polls, before_queued);
        assert_eq!(
            adapter.poll_bridge(),
            Err(SemanticBridgePollFailureV1::RuntimeUnavailable)
        );
        assert_eq!(adapter.transport.receive_polls, before_queued + 1);
    }

    #[test]
    fn bridge_poll_classifies_and_latches_transport_and_delivery_failures() {
        for receive in [LspReceiveOutcomeV1::Closed, LspReceiveOutcomeV1::Failed] {
            let mut adapter = ready_adapter();
            adapter.transport.receives.push_back(receive);
            let readiness_polls = adapter.readiness.polls;
            assert_eq!(
                adapter.poll_bridge(),
                Err(SemanticBridgePollFailureV1::RuntimeUnavailable)
            );
            assert_eq!(adapter.readiness.polls, readiness_polls);
            let polls = adapter.transport.receive_polls;
            assert_eq!(
                adapter.poll_bridge(),
                Err(SemanticBridgePollFailureV1::RuntimeUnavailable)
            );
            assert_eq!(adapter.transport.receive_polls, polls);
            assert_eq!(adapter.session_state(), LspSessionStateV1::Failed);
        }

        let mut malformed = ready_adapter();
        malformed
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed_raw("{")));
        assert_eq!(
            malformed.poll_bridge(),
            Err(SemanticBridgePollFailureV1::DeliveryRefused)
        );
        let polls = malformed.transport.receive_polls;
        assert_eq!(
            malformed.poll_bridge(),
            Err(SemanticBridgePollFailureV1::DeliveryRefused)
        );
        assert_eq!(malformed.transport.receive_polls, polls);
        assert_eq!(malformed.usage().malformed_messages, 1);
    }

    #[test]
    fn bridge_poll_classifies_local_generation_failure_as_poisoned() {
        let mut transport = MemoryTransport::default();
        transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {"capabilities": {"positionEncoding": "utf-16"}}
            }))));
        let mut adapter = LspAdapterV1::try_new_with_readiness(
            transport,
            Issuer(u64::MAX),
            ScriptedReadiness::current(),
            provenance(),
            LspLimitsV1::reference(),
            SemanticLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(adapter.start(), LspStartOutcomeV1::Started);
        assert_eq!(
            adapter.poll_bridge(),
            Err(SemanticBridgePollFailureV1::Poisoned)
        );
        assert_eq!(adapter.session_state(), LspSessionStateV1::Failed);
    }

    #[test]
    fn cancelled_response_retains_exact_correlation_then_duplicate_is_orphaned() {
        let mut adapter = ready_adapter();
        open_document(&mut adapter);
        let request = diagnostics_request(61);
        assert_eq!(
            adapter.submit(&request),
            SemanticProviderSubmissionV1::Accepted
        );
        adapter.cancel(request.request);
        assert_eq!(adapter.cancelled.get(&2), Some(&request.request));
        assert_eq!(adapter.usage().cancelled_correlations_current, 1);

        let late = framed(json!({"jsonrpc": "2.0", "id": 2, "result": null}));
        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(late.clone()));
        let completion = adapter.poll_bridge().unwrap().unwrap();
        assert_eq!(completion.request, request.request);
        assert_eq!(completion.outcome, SemanticOutcomeV1::Unavailable);
        assert!(adapter.cancelled.is_empty());
        assert_eq!(adapter.usage().cancelled_correlations_current, 0);
        assert_eq!(adapter.usage().cancelled_responses_correlated, 1);

        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(late));
        assert_eq!(adapter.poll_bridge(), Ok(None));
        assert_eq!(adapter.usage().orphan_messages, 1);
        assert_eq!(adapter.usage().cancelled_responses_correlated, 1);
    }

    #[test]
    fn cancelled_correlation_is_bounded_and_released_only_by_response_or_abort() {
        let mut adapter = ready_adapter();
        open_document(&mut adapter);
        for number in 0..MAX_LSP_PENDING_REQUESTS_V1 as u64 {
            let request = diagnostics_request_numbered(number);
            assert_eq!(
                adapter.submit(&request),
                SemanticProviderSubmissionV1::Accepted
            );
            adapter.cancel(request.request);
        }
        assert_eq!(adapter.cancelled.len(), MAX_LSP_PENDING_REQUESTS_V1);
        assert_eq!(
            adapter.usage().cancelled_correlations_current,
            MAX_LSP_PENDING_REQUESTS_V1 as u64
        );
        assert_eq!(
            adapter.usage().cancelled_correlations_high_water,
            MAX_LSP_PENDING_REQUESTS_V1 as u64
        );
        assert_eq!(adapter.begin_shutdown(), LspShutdownOutcomeV1::Busy);

        let overflow = diagnostics_request_numbered(MAX_LSP_PENDING_REQUESTS_V1 as u64);
        assert_eq!(
            adapter.submit(&overflow),
            SemanticProviderSubmissionV1::Accepted
        );
        adapter.cancel(overflow.request);
        assert_eq!(adapter.cancelled.len(), MAX_LSP_PENDING_REQUESTS_V1);
        assert!(
            adapter
                .pending
                .values()
                .any(|pending| pending.request == overflow)
        );
        assert_eq!(
            adapter.poll_bridge(),
            Err(SemanticBridgePollFailureV1::DeliveryRefused)
        );

        assert_eq!(adapter.abort(), LspAbortOutcomeV1::Aborted);
        assert!(adapter.cancelled.is_empty());
        assert!(adapter.pending.is_empty());
        assert_eq!(adapter.usage().cancelled_correlations_current, 0);
    }

    #[test]
    fn cancel_notification_send_failures_are_latched_for_bridge_poll() {
        for (send, expected) in [
            (
                LspSendOutcomeV1::Unavailable,
                SemanticBridgePollFailureV1::RuntimeUnavailable,
            ),
            (
                LspSendOutcomeV1::Refused,
                SemanticBridgePollFailureV1::DeliveryRefused,
            ),
        ] {
            let mut adapter = ready_adapter();
            open_document(&mut adapter);
            let request = diagnostics_request(71);
            assert_eq!(
                adapter.submit(&request),
                SemanticProviderSubmissionV1::Accepted
            );
            adapter.transport.next_send = Some(send);
            adapter.cancel(request.request);
            assert_eq!(adapter.cancelled.get(&2), Some(&request.request));
            assert_eq!(adapter.poll_bridge(), Err(expected));
            assert_eq!(adapter.poll_bridge(), Err(expected));
        }
    }

    #[test]
    fn framing_is_incremental_coalesced_and_rejects_ambiguous_lengths() {
        let limits = LspLimitsV1::reference();
        let first = encode_lsp_frame_v1(br#"{"a":1}"#, limits).unwrap();
        let second = encode_lsp_frame_v1(br#"{"b":2}"#, limits).unwrap();
        let mut decoder = LspFrameDecoderV1::try_new(limits).unwrap();
        decoder.push(&first[..5]).unwrap();
        assert_eq!(decoder.next_frame().unwrap(), None);
        let mut rest = first[5..].to_vec();
        rest.extend_from_slice(&second);
        decoder.push(&rest).unwrap();
        assert_eq!(decoder.next_frame().unwrap(), Some(br#"{"a":1}"#.to_vec()));
        assert_eq!(decoder.next_frame().unwrap(), Some(br#"{"b":2}"#.to_vec()));
        assert_eq!(decoder.next_frame().unwrap(), None);

        for invalid in [
            b"X: 1\r\n\r\n{}".as_slice(),
            b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}".as_slice(),
            b"Content-Length: -1\r\n\r\n".as_slice(),
        ] {
            let mut decoder = LspFrameDecoderV1::try_new(limits).unwrap();
            decoder.push(invalid).unwrap();
            assert!(decoder.next_frame().is_err());
        }
    }

    #[test]
    fn utf_position_conversion_is_exact_and_refuses_surrogate_and_crlf_interiors() {
        let source = "a😀b\r\nç";
        assert_eq!(
            byte_offset_to_lsp_position_v1(source, 5, LspPositionEncodingV1::Utf16).unwrap(),
            LspPositionV1 {
                line: 0,
                character: 3
            }
        );
        assert_eq!(
            lsp_position_to_byte_offset_v1(
                source,
                LspPositionV1 {
                    line: 0,
                    character: 3
                },
                LspPositionEncodingV1::Utf16,
            )
            .unwrap(),
            5
        );
        assert_eq!(
            lsp_position_to_byte_offset_v1(
                source,
                LspPositionV1 {
                    line: 0,
                    character: 2
                },
                LspPositionEncodingV1::Utf16,
            ),
            Err(LspPositionRefusalV1::SplitEncoding)
        );
        assert_eq!(
            byte_offset_to_lsp_position_v1(source, 7, LspPositionEncodingV1::Utf8),
            Err(LspPositionRefusalV1::CrlfInterior)
        );
    }

    #[test]
    fn generic_initialization_remains_indexing_without_private_readiness_evidence() {
        let mut transport = MemoryTransport::default();
        transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {"capabilities": {
                    "diagnosticProvider": {},
                    "documentSymbolProvider": true
                }}
            }))));
        let mut adapter =
            LspAdapterV1::try_new(transport, Issuer(0), provenance(), LspLimitsV1::reference())
                .unwrap();
        assert_eq!(adapter.start(), LspStartOutcomeV1::Started);
        assert_eq!(adapter.poll(), None);
        assert!(matches!(
            adapter.session_state(),
            LspSessionStateV1::Indexing { .. }
        ));
        assert_eq!(
            adapter.state().readiness,
            SemanticProviderReadinessV1::Indexing
        );
        assert_eq!(adapter.usage().readiness_promotions, 0);
        let uri = LspVirtualDocumentV1::try_new(
            "vellum-lsp://seven".to_owned(),
            LspLimitsV1::reference(),
        )
        .unwrap();
        assert!(adapter.register_virtual_document(target(0).document, uri));
        let snapshot = SemanticSnapshotV1::try_new(
            target(0),
            "rust".to_owned(),
            "fn warming() {}".to_owned(),
            vellum_semantic_service::SemanticServiceLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(
            adapter.synchronize(&SemanticSnapshotUpdateV1::Admit(snapshot)),
            SemanticProviderAdmissionV1::Accepted
        );
    }

    #[test]
    fn initialization_profile_binds_one_workspace_and_fixed_client_capabilities() {
        let limits = LspLimitsV1::reference();
        let profile = LspInitializationProfileV1::try_workspace(
            "file:///private/workspace".to_owned(),
            "workspace".to_owned(),
            Some("experimental/serverStatus".to_owned()),
            limits,
        )
        .unwrap();
        assert!(profile.has_workspace());
        assert!(profile.observes_readiness_notifications());
        let debug = format!("{profile:?}");
        assert!(!debug.contains("private"));
        assert!(!debug.contains("serverStatus"));
        let mut adapter = LspAdapterV1::try_new_with_profile(
            MemoryTransport::default(),
            Issuer(0),
            ScriptedReadiness::current(),
            LspExplicitVirtualDocumentsV1,
            LspAdapterProfileV1::try_new(
                provenance(),
                limits,
                SemanticLimitsV1::reference(),
                profile,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            adapter.qualification_evidence().server_identity(),
            LspServerIdentityDispositionV1::UnpinnedAbsent
        );
        assert_eq!(adapter.start(), LspStartOutcomeV1::Started);
        let initialize = framed_json(adapter.transport.sends.first().unwrap());
        assert_eq!(
            initialize
                .pointer("/params/rootUri")
                .and_then(Value::as_str),
            Some("file:///private/workspace")
        );
        assert_eq!(
            initialize
                .pointer("/params/workspaceFolders/0/name")
                .and_then(Value::as_str),
            Some("workspace")
        );
        assert_eq!(
            initialize
                .pointer("/params/capabilities/workspace/workspaceEdit/documentChanges")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            initialize
                .pointer("/params/capabilities/workspace/workspaceFolders")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            initialize.pointer("/params/capabilities/experimental"),
            None
        );
        assert_eq!(initialize.pointer("/params/initializationOptions"), None);
        assert!(
            LspInitializationProfileV1::try_workspace(
                "https://example.invalid".to_owned(),
                "workspace".to_owned(),
                None,
                limits,
            )
            .is_none()
        );
    }

    #[test]
    fn rust_analyzer_standalone_pins_project_root_and_confines_distinct_documents() {
        let limits = LspLimitsV1::reference();
        let protocol = LspProviderProtocolProfileV1::try_new_with_readiness_schema(
            [11; 32],
            "rust-analyzer".to_owned(),
            Some("1.0.0".to_owned()),
            "experimental/serverStatus".to_owned(),
            LspReadinessNotificationSchemaV1::HealthAndQuiescentWithOptionalMessage,
        )
        .unwrap();
        let initialization = LspInitializationProfileV1::try_rust_analyzer_standalone(
            "file:///private/qualified%20package".to_owned(),
            "workspace".to_owned(),
            protocol,
            limits,
        )
        .unwrap();
        assert!(initialization.has_provider_initialization_options());
        let debug = format!("{initialization:?}");
        assert!(!debug.contains("private"));
        assert!(!debug.contains("src/lib.rs"));
        assert!(!debug.contains("rust-analyzer"));
        let options_debug = format!("{:?}", initialization.provider_options.as_ref().unwrap());
        assert!(!options_debug.contains("private"));
        assert!(!options_debug.contains("src/lib.rs"));

        let mut adapter = LspAdapterV1::try_new_with_profile(
            MemoryTransport::default(),
            Issuer(0),
            ScriptedReadiness::current(),
            LspExplicitVirtualDocumentsV1,
            LspAdapterProfileV1::try_new(
                provenance(),
                limits,
                SemanticLimitsV1::reference(),
                initialization,
            )
            .unwrap(),
        )
        .unwrap();
        let main =
            LspDocumentLocatorV1::try_workspace_relative("src/main.rs".to_owned(), limits).unwrap();
        assert!(adapter.register_virtual_document(SemanticDocumentRefV1::new([2; 16]), main));
        let opaque =
            LspDocumentLocatorV1::try_new("vellum-lsp://other".to_owned(), limits).unwrap();
        assert!(!adapter.register_virtual_document(SemanticDocumentRefV1::new([3; 16]), opaque));
        let lib =
            LspDocumentLocatorV1::try_workspace_relative("src/lib.rs".to_owned(), limits).unwrap();
        assert!(adapter.register_virtual_document(target(0).document, lib));

        assert_eq!(adapter.start(), LspStartOutcomeV1::Started);
        let initialize = framed_json(adapter.transport.sends.first().unwrap());
        assert_eq!(
            initialize.pointer("/params/initializationOptions"),
            Some(&json!({
                "linkedProjects": ["/private/qualified package/src/lib.rs"],
                "cargo": {
                    "autoreload": true,
                    "buildScripts": {"enable": false},
                    "noDeps": true,
                    "sysroot": Value::Null
                },
                "checkOnSave": false,
                "procMacro": {"enable": false},
                "notifications": {"cargoTomlNotFound": false}
            }))
        );
        assert_eq!(
            initialize
                .pointer("/params/initializationOptions/linkedProjects/0")
                .and_then(Value::as_str),
            Some("/private/qualified package/src/lib.rs")
        );
        assert_eq!(
            adapter.documents[&target(0).document].uri.0,
            "file:///private/qualified%20package/src/lib.rs"
        );
        assert_eq!(
            adapter.documents[&SemanticDocumentRefV1::new([2; 16])]
                .uri
                .0,
            "file:///private/qualified%20package/src/main.rs"
        );
        assert_eq!(
            initialize
                .pointer("/params/capabilities/experimental/serverStatusNotification")
                .and_then(Value::as_bool),
            Some(true)
        );
        for absent in [
            "/params/capabilities/workspace/configuration",
            "/params/capabilities/workspace/didChangeWatchedFiles",
            "/params/capabilities/window/workDoneProgress",
            "/params/capabilities/workspace/diagnostics",
            "/params/capabilities/workspace/semanticTokens",
            "/params/capabilities/workspace/inlayHint",
            "/params/capabilities/workspace/codeLens",
        ] {
            assert_eq!(
                initialize.pointer(absent),
                None,
                "advertised forbidden capability"
            );
        }
    }

    #[test]
    fn rust_analyzer_standalone_refuses_a_root_that_cannot_bind_src_lib() {
        let mut limits = LspLimitsV1::reference();
        limits.max_uri_bytes = 25;
        let protocol = LspProviderProtocolProfileV1::try_new(
            [12; 32],
            "rust-analyzer".to_owned(),
            None,
            "experimental/serverStatus".to_owned(),
        )
        .unwrap();
        assert!(
            LspInitializationProfileV1::try_workspace_with_protocol(
                "file:///workspace".to_owned(),
                "workspace".to_owned(),
                protocol.clone(),
                limits,
            )
            .is_some()
        );
        assert!(
            LspInitializationProfileV1::try_rust_analyzer_standalone(
                "file:///workspace".to_owned(),
                "workspace".to_owned(),
                protocol.clone(),
                limits,
            )
            .is_none()
        );
        for noncanonical_root in [
            "file://host/workspace",
            "file:///workspace/",
            "file:///workspace/../escape",
            "file:///workspace%2fescape",
            "file:///workspace/%2E%2E",
        ] {
            assert!(
                LspInitializationProfileV1::try_rust_analyzer_standalone(
                    noncanonical_root.to_owned(),
                    "workspace".to_owned(),
                    protocol.clone(),
                    LspLimitsV1::reference(),
                )
                .is_none(),
                "accepted non-canonical standalone root"
            );
        }
    }

    #[test]
    fn bounded_server_identity_and_closed_status_notification_drive_readiness() {
        let limits = LspLimitsV1::reference();
        let profile = LspInitializationProfileV1::try_workspace(
            "file:///workspace".to_owned(),
            "workspace".to_owned(),
            Some("experimental/serverStatus".to_owned()),
            limits,
        )
        .unwrap();
        let mut readiness = ScriptedReadiness::current();
        readiness.last = LspIndexReadinessV1::Indexing;
        readiness.outcomes.clear();
        let mut transport = MemoryTransport::default();
        transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "capabilities": {"positionEncoding": "utf-8"},
                    "serverInfo": {"name": "pinned-provider", "version": "1.2.3"}
                }
            }))));
        transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0",
                "method": "experimental/serverStatus",
                "params": {"health": "ok", "quiescent": true}
            }))));
        let mut adapter = LspAdapterV1::try_new_with_profile(
            transport,
            Issuer(0),
            readiness,
            LspExplicitVirtualDocumentsV1,
            LspAdapterProfileV1::try_new(
                provenance(),
                limits,
                SemanticLimitsV1::reference(),
                profile,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(adapter.start(), LspStartOutcomeV1::Started);
        assert_eq!(adapter.poll(), None);
        assert!(matches!(
            adapter.session_state(),
            LspSessionStateV1::Indexing { .. }
        ));
        assert!(
            adapter
                .server_info()
                .is_some_and(|info| info.matches("pinned-provider", Some("1.2.3")))
        );
        assert_eq!(
            format!("{:?}", adapter.server_info().unwrap()),
            "LspServerInfoV1(<opaque>)"
        );
        assert_eq!(adapter.poll(), None);
        assert!(matches!(
            adapter.session_state(),
            LspSessionStateV1::Ready { .. }
        ));
        assert_eq!(adapter.readiness.notifications.len(), 1);
        assert_eq!(adapter.usage().server_info_captured, 1);
        assert_eq!(adapter.usage().notifications_observed, 1);
    }

    #[test]
    fn pinned_protocol_identity_and_notification_readiness_are_exact_and_content_safe() {
        let limits = LspLimitsV1::reference();
        let digest = [7_u8; 32];
        assert!(
            LspProviderProtocolProfileV1::try_new_with_readiness_schema(
                [0; 32],
                "pinned-provider".to_owned(),
                None,
                "experimental/serverStatus".to_owned(),
                LspReadinessNotificationSchemaV1::HealthAndQuiescent,
            )
            .is_none()
        );
        let protocol = LspProviderProtocolProfileV1::try_new_with_readiness_schema(
            digest,
            "pinned-provider".to_owned(),
            Some("1.2.3".to_owned()),
            "experimental/serverStatus".to_owned(),
            LspReadinessNotificationSchemaV1::HealthAndQuiescentWithOptionalMessage,
        )
        .unwrap();
        assert_eq!(
            format!("{protocol:?}"),
            "LspProviderProtocolProfileV1(<opaque>)"
        );
        let profile = LspInitializationProfileV1::try_workspace_with_protocol(
            "file:///private/pinned-workspace".to_owned(),
            "pinned-workspace".to_owned(),
            protocol,
            limits,
        )
        .unwrap();
        let mut transport = MemoryTransport::default();
        transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "id": 1,
                "result": {
                    "capabilities": {
                        "diagnosticProvider": {},
                        "documentSymbolProvider": true,
                        "renameProvider": true
                    },
                    "serverInfo": {"name": "pinned-provider", "version": "1.2.3"}
                }
            }))));
        transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "method": "experimental/serverStatus",
                "params": {
                    "health": "ok", "quiescent": true,
                    "message": "private provider progress"
                }
            }))));
        let mut adapter = LspAdapterV1::try_new_with_profile(
            transport,
            Issuer(0),
            ScriptedReadiness::current(),
            LspExplicitVirtualDocumentsV1,
            LspAdapterProfileV1::try_new(
                provenance(),
                limits,
                SemanticLimitsV1::reference(),
                profile,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(adapter.start(), LspStartOutcomeV1::Started);
        let initialize = framed_json(adapter.transport.sends.first().unwrap());
        assert_eq!(
            initialize.pointer("/params/capabilities/experimental/serverStatusNotification"),
            Some(&Value::Bool(true))
        );
        assert_eq!(adapter.poll(), None);
        assert!(matches!(
            adapter.session_state(),
            LspSessionStateV1::Indexing { .. }
        ));
        let before = adapter.qualification_evidence();
        assert!(before.profile_matches(&digest));
        assert_eq!(
            before.server_identity(),
            LspServerIdentityDispositionV1::PinnedExact
        );
        assert_eq!(
            before.readiness_source(),
            LspReadinessEvidenceSourceV1::NotificationBound
        );
        assert_eq!(before.last_notification_readiness(), None);
        assert_eq!(before.capabilities(), (true, true));
        let debug = format!("{before:?}");
        assert!(!debug.contains("private"));
        assert!(!debug.contains("pinned-provider"));
        assert!(!debug.contains("serverStatus"));
        assert!(!debug.contains("private provider progress"));

        assert_eq!(adapter.poll(), None);
        assert!(matches!(
            adapter.session_state(),
            LspSessionStateV1::Ready { .. }
        ));
        let after = adapter.qualification_evidence();
        assert_eq!(
            after.last_notification_readiness(),
            Some(LspIndexReadinessV1::Current)
        );
        assert_eq!(after.notifications_observed(), 1);
        assert_eq!(after.readiness_transitions(), (1, 0, 0));
    }

    #[test]
    fn pinned_protocol_rejects_missing_or_mismatched_server_identity() {
        let limits = LspLimitsV1::reference();
        for server_info in [
            None,
            Some(json!({"name": "other-provider", "version": "1.2.3"})),
            Some(json!({"name": "pinned-provider", "version": "9.9.9"})),
        ] {
            let protocol = LspProviderProtocolProfileV1::try_new_with_readiness_schema(
                [8; 32],
                "pinned-provider".to_owned(),
                Some("1.2.3".to_owned()),
                "experimental/serverStatus".to_owned(),
                LspReadinessNotificationSchemaV1::HealthAndQuiescent,
            )
            .unwrap();
            let profile = LspInitializationProfileV1::try_workspace_with_protocol(
                "file:///workspace".to_owned(),
                "workspace".to_owned(),
                protocol,
                limits,
            )
            .unwrap();
            let mut result = json!({"capabilities": {}});
            if let Some(server_info) = server_info {
                result
                    .as_object_mut()
                    .unwrap()
                    .insert("serverInfo".to_owned(), server_info);
            }
            let mut transport = MemoryTransport::default();
            transport
                .receives
                .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                    "jsonrpc": "2.0", "id": 1, "result": result
                }))));
            let mut adapter = LspAdapterV1::try_new_with_profile(
                transport,
                Issuer(0),
                ScriptedReadiness::current(),
                LspExplicitVirtualDocumentsV1,
                LspAdapterProfileV1::try_new(
                    provenance(),
                    limits,
                    SemanticLimitsV1::reference(),
                    profile,
                )
                .unwrap(),
            )
            .unwrap();
            assert_eq!(adapter.start(), LspStartOutcomeV1::Started);
            assert_eq!(adapter.poll(), None);
            assert_eq!(adapter.session_state(), LspSessionStateV1::Failed);
            assert_eq!(adapter.usage().server_info_refused, 1);
            assert_eq!(
                adapter.qualification_evidence().server_identity(),
                LspServerIdentityDispositionV1::PinnedRefused
            );
        }
    }

    #[test]
    fn pinned_initialize_error_is_refused_without_claiming_unpinned_identity() {
        let limits = LspLimitsV1::reference();
        let protocol = LspProviderProtocolProfileV1::try_new_with_readiness_schema(
            [9; 32],
            "pinned-provider".to_owned(),
            Some("1.2.3".to_owned()),
            "experimental/serverStatus".to_owned(),
            LspReadinessNotificationSchemaV1::HealthAndQuiescent,
        )
        .unwrap();
        let initialization = LspInitializationProfileV1::try_workspace_with_protocol(
            "file:///workspace".to_owned(),
            "workspace".to_owned(),
            protocol,
            limits,
        )
        .unwrap();
        let mut transport = MemoryTransport::default();
        transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "id": 1,
                "error": {"code": -32002, "message": "initialization refused"}
            }))));
        let mut adapter = LspAdapterV1::try_new_with_profile(
            transport,
            Issuer(0),
            ScriptedReadiness::current(),
            LspExplicitVirtualDocumentsV1,
            LspAdapterProfileV1::try_new(
                provenance(),
                limits,
                SemanticLimitsV1::reference(),
                initialization,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            adapter.qualification_evidence().server_identity(),
            LspServerIdentityDispositionV1::PinnedUnobserved
        );
        assert_eq!(adapter.start(), LspStartOutcomeV1::Started);
        assert_eq!(adapter.poll(), None);
        assert_eq!(adapter.session_state(), LspSessionStateV1::Failed);
        assert_eq!(
            adapter.qualification_evidence().server_identity(),
            LspServerIdentityDispositionV1::PinnedRefused
        );
    }

    #[test]
    fn readiness_notification_schema_closes_optional_message_shape_and_bound() {
        let limits = LspLimitsV1::reference();
        let initialization = |schema| {
            let protocol = LspProviderProtocolProfileV1::try_new_with_readiness_schema(
                [10; 32],
                "pinned-provider".to_owned(),
                None,
                "experimental/serverStatus".to_owned(),
                schema,
            )
            .unwrap();
            LspInitializationProfileV1::try_workspace_with_protocol(
                "file:///workspace".to_owned(),
                "workspace".to_owned(),
                protocol,
                limits,
            )
            .unwrap()
        };

        let mut optional_message = ready_adapter();
        optional_message.initialization =
            initialization(LspReadinessNotificationSchemaV1::HealthAndQuiescentWithOptionalMessage);
        optional_message
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "method": "experimental/serverStatus",
                "params": {"health": "ok", "quiescent": true, "message": "discard me"}
            }))));
        assert_eq!(optional_message.poll(), None);
        assert!(matches!(
            optional_message.session_state(),
            LspSessionStateV1::Ready { .. }
        ));
        assert_eq!(optional_message.usage().notifications_observed, 1);
        assert!(!format!("{:?}", optional_message.qualification_evidence()).contains("discard me"));

        let mut null_message = ready_adapter();
        null_message.initialization =
            initialization(LspReadinessNotificationSchemaV1::HealthAndQuiescentWithOptionalMessage);
        null_message
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "method": "experimental/serverStatus",
                "params": {"health": "ok", "quiescent": true, "message": Value::Null}
            }))));
        assert_eq!(null_message.poll(), None);
        assert!(matches!(
            null_message.session_state(),
            LspSessionStateV1::Ready { .. }
        ));
        let null_evidence = null_message.qualification_evidence();
        assert_eq!(
            null_evidence.last_notification_readiness(),
            Some(LspIndexReadinessV1::Current)
        );
        assert_eq!(null_evidence.notifications_observed(), 1);
        assert!(!format!("{null_evidence:?}").contains("message"));

        for (schema, message) in [
            (
                LspReadinessNotificationSchemaV1::HealthAndQuiescent,
                json!("extra"),
            ),
            (
                LspReadinessNotificationSchemaV1::HealthAndQuiescentWithOptionalMessage,
                json!("x".repeat(MAX_LSP_READINESS_MESSAGE_BYTES_V1 + 1)),
            ),
            (
                LspReadinessNotificationSchemaV1::HealthAndQuiescentWithOptionalMessage,
                json!(true),
            ),
            (
                LspReadinessNotificationSchemaV1::HealthAndQuiescentWithOptionalMessage,
                json!(1),
            ),
            (
                LspReadinessNotificationSchemaV1::HealthAndQuiescentWithOptionalMessage,
                json!([]),
            ),
            (
                LspReadinessNotificationSchemaV1::HealthAndQuiescentWithOptionalMessage,
                json!({}),
            ),
        ] {
            let mut refused = ready_adapter();
            refused.initialization = initialization(schema);
            refused
                .transport
                .receives
                .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                    "jsonrpc": "2.0", "method": "experimental/serverStatus",
                    "params": {"health": "ok", "quiescent": true, "message": message}
                }))));
            assert_eq!(refused.poll(), None);
            assert_eq!(refused.session_state(), LspSessionStateV1::Failed);
            assert_eq!(refused.usage().notifications_refused, 1);
        }
    }

    #[test]
    fn malformed_identity_notification_and_inbound_request_fail_closed() {
        let limits = LspLimitsV1::reference();
        let profile = || {
            LspInitializationProfileV1::try_workspace(
                "file:///workspace".to_owned(),
                "workspace".to_owned(),
                Some("experimental/serverStatus".to_owned()),
                limits,
            )
            .unwrap()
        };
        let mut transport = MemoryTransport::default();
        transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "id": 1,
                "result": {"capabilities": {}, "serverInfo": {"name": "x", "extra": true}}
            }))));
        let mut malformed_identity = LspAdapterV1::try_new_with_profile(
            transport,
            Issuer(0),
            ScriptedReadiness::current(),
            LspExplicitVirtualDocumentsV1,
            LspAdapterProfileV1::try_new(
                provenance(),
                limits,
                SemanticLimitsV1::reference(),
                profile(),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(malformed_identity.start(), LspStartOutcomeV1::Started);
        assert_eq!(malformed_identity.poll(), None);
        assert_eq!(
            malformed_identity.session_state(),
            LspSessionStateV1::Failed
        );
        assert_eq!(malformed_identity.usage().server_info_refused, 1);

        let mut malformed_notification = ready_adapter();
        malformed_notification.initialization = profile();
        malformed_notification
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "method": "experimental/serverStatus",
                "params": {"health": "ok", "quiescent": "yes"}
            }))));
        assert_eq!(malformed_notification.poll(), None);
        assert_eq!(
            malformed_notification.session_state(),
            LspSessionStateV1::Failed
        );
        assert_eq!(malformed_notification.usage().notifications_refused, 1);

        let mut inbound_request = ready_adapter();
        inbound_request
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "id": 44, "method": "window/showMessageRequest",
                "params": {}
            }))));
        assert_eq!(inbound_request.poll(), None);
        assert_eq!(inbound_request.session_state(), LspSessionStateV1::Failed);
        assert_eq!(inbound_request.usage().inbound_server_requests, 1);
    }

    #[test]
    fn readiness_demotes_and_repromotes_with_a_fresh_generation() {
        let mut adapter = ready_adapter();
        assert!(matches!(
            adapter.session_state(),
            LspSessionStateV1::Ready { generation, .. }
                if generation == SemanticIndexGenerationV1::new(1).unwrap()
        ));
        adapter
            .readiness
            .outcomes
            .push_back(LspIndexReadinessV1::Indexing);
        assert_eq!(adapter.poll(), None);
        assert!(matches!(
            adapter.session_state(),
            LspSessionStateV1::Indexing { .. }
        ));
        adapter
            .readiness
            .outcomes
            .push_back(LspIndexReadinessV1::Current);
        assert_eq!(adapter.poll(), None);
        assert!(matches!(
            adapter.session_state(),
            LspSessionStateV1::Ready { generation, .. }
                if generation == SemanticIndexGenerationV1::new(2).unwrap()
        ));
        adapter
            .readiness
            .outcomes
            .push_back(LspIndexReadinessV1::Unavailable);
        assert_eq!(adapter.poll(), None);
        assert!(matches!(
            adapter.session_state(),
            LspSessionStateV1::Unavailable { .. }
        ));
        adapter
            .readiness
            .outcomes
            .push_back(LspIndexReadinessV1::Current);
        assert_eq!(adapter.poll(), None);
        assert!(matches!(
            adapter.session_state(),
            LspSessionStateV1::Ready { generation, .. }
                if generation == SemanticIndexGenerationV1::new(3).unwrap()
        ));
        assert_eq!(adapter.usage().readiness_promotions, 3);
        assert_eq!(adapter.usage().readiness_demotions, 2);
        assert_eq!(adapter.usage().readiness_unavailable, 1);
    }

    #[test]
    fn graceful_shutdown_is_busy_until_drained_and_closes_only_after_exit() {
        let mut adapter = ready_adapter();
        open_document(&mut adapter);
        assert_eq!(adapter.begin_shutdown(), LspShutdownOutcomeV1::Busy);
        assert_eq!(
            adapter.synchronize(&SemanticSnapshotUpdateV1::Close { target: target(0) }),
            SemanticProviderAdmissionV1::Accepted
        );
        assert_eq!(adapter.begin_shutdown(), LspShutdownOutcomeV1::Started);
        assert_eq!(
            adapter.session_state(),
            LspSessionStateV1::ShuttingDown { request_id: 2 }
        );
        assert!(!adapter.transport.closed);
        let shutdown = framed_json(adapter.transport.sends.last().unwrap());
        assert_eq!(
            shutdown.get("method").and_then(Value::as_str),
            Some("shutdown")
        );
        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "id": 2, "result": Value::Null
            }))));
        assert_eq!(adapter.poll(), None);
        assert_eq!(adapter.session_state(), LspSessionStateV1::Closed);
        assert!(adapter.transport.closed);
        let exit = framed_json(adapter.transport.sends.last().unwrap());
        assert_eq!(exit.get("method").and_then(Value::as_str), Some("exit"));
        assert_eq!(adapter.usage().shutdown_started, 1);
        assert_eq!(adapter.usage().shutdown_completed, 1);
        assert_eq!(adapter.abort(), LspAbortOutcomeV1::AlreadyTerminal);
    }

    #[test]
    fn shutdown_classifies_late_notifications_before_exact_correlated_response() {
        let limits = LspLimitsV1::reference();
        let initialization = || {
            LspInitializationProfileV1::try_workspace(
                "file:///workspace".to_owned(),
                "workspace".to_owned(),
                Some("experimental/serverStatus".to_owned()),
                limits,
            )
            .unwrap()
        };

        let mut adapter = ready_adapter();
        adapter.initialization = initialization();
        assert_eq!(adapter.begin_shutdown(), LspShutdownOutcomeV1::Started);
        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "method": "experimental/serverStatus",
                "params": {"health": "ok", "quiescent": true}
            }))));
        assert_eq!(adapter.poll(), None);
        assert_eq!(
            adapter.session_state(),
            LspSessionStateV1::ShuttingDown { request_id: 2 }
        );
        assert!(!adapter.transport.closed);
        assert_eq!(adapter.usage().notifications_observed, 1);
        assert_eq!(adapter.usage().shutdown_refused, 0);
        assert_eq!(
            adapter
                .qualification_evidence()
                .last_notification_readiness(),
            Some(LspIndexReadinessV1::Current)
        );

        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "id": 2, "result": Value::Null
            }))));
        assert_eq!(adapter.poll(), None);
        assert_eq!(adapter.session_state(), LspSessionStateV1::Closed);
        assert!(adapter.transport.closed);
        assert_eq!(
            framed_json(adapter.transport.sends.last().unwrap())
                .get("method")
                .and_then(Value::as_str),
            Some("exit")
        );
        assert_eq!(adapter.usage().shutdown_completed, 1);

        let mut malformed_notification = ready_adapter();
        malformed_notification.initialization = initialization();
        assert_eq!(
            malformed_notification.begin_shutdown(),
            LspShutdownOutcomeV1::Started
        );
        malformed_notification
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "method": "experimental/serverStatus",
                "params": {"health": "ok", "quiescent": true, "extra": true}
            }))));
        assert_eq!(malformed_notification.poll(), None);
        assert_eq!(
            malformed_notification.session_state(),
            LspSessionStateV1::Failed
        );
        assert_eq!(malformed_notification.usage().notifications_refused, 1);

        let mut inbound_request = ready_adapter();
        inbound_request.initialization = initialization();
        assert_eq!(
            inbound_request.begin_shutdown(),
            LspShutdownOutcomeV1::Started
        );
        inbound_request
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "id": 44, "method": "window/showMessageRequest",
                "params": {}
            }))));
        assert_eq!(inbound_request.poll(), None);
        assert_eq!(inbound_request.session_state(), LspSessionStateV1::Failed);
        assert_eq!(inbound_request.usage().inbound_server_requests, 1);
    }

    #[test]
    fn malformed_shutdown_never_claims_graceful_close_and_abort_is_explicit() {
        let mut adapter = ready_adapter();
        adapter.transport.next_send = Some(LspSendOutcomeV1::Unavailable);
        assert_eq!(adapter.begin_shutdown(), LspShutdownOutcomeV1::Unavailable);
        assert!(matches!(
            adapter.session_state(),
            LspSessionStateV1::Ready { .. }
        ));
        assert_eq!(adapter.begin_shutdown(), LspShutdownOutcomeV1::Started);
        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "id": 99, "result": Value::Null
            }))));
        assert_eq!(adapter.poll(), None);
        assert_eq!(adapter.session_state(), LspSessionStateV1::Failed);
        assert!(!adapter.transport.closed);
        assert_eq!(adapter.usage().shutdown_refused, 1);
        assert_eq!(adapter.abort(), LspAbortOutcomeV1::Aborted);
        assert_eq!(adapter.session_state(), LspSessionStateV1::Aborted);
        assert!(adapter.transport.closed);
        assert_eq!(adapter.usage().shutdown_aborted, 1);
    }

    #[test]
    fn initialize_and_snapshot_publication_are_send_before_state() {
        let mut transport = MemoryTransport::default();
        let response = framed(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"capabilities": {"positionEncoding": "utf-16"}}
        }));
        transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(response));
        let mut adapter = LspAdapterV1::try_new_with_readiness(
            transport,
            Issuer(0),
            ScriptedReadiness::current(),
            provenance(),
            LspLimitsV1::reference(),
            SemanticLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(adapter.start(), LspStartOutcomeV1::Started);
        assert_eq!(adapter.poll(), None);
        assert!(matches!(
            adapter.session_state(),
            LspSessionStateV1::Ready { .. }
        ));
        let uri = LspVirtualDocumentV1::try_new(
            "vellum-lsp://seven".to_owned(),
            LspLimitsV1::reference(),
        )
        .unwrap();
        assert!(adapter.register_virtual_document(target(0).document, uri));
        let snapshot = SemanticSnapshotV1::try_new(
            target(0),
            "rust".to_owned(),
            "fn main() {}".to_owned(),
            vellum_semantic_service::SemanticServiceLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(
            adapter.synchronize(&SemanticSnapshotUpdateV1::Admit(snapshot)),
            SemanticProviderAdmissionV1::Accepted
        );
        assert_eq!(adapter.usage().snapshots_opened, 1);

        adapter.transport.next_send = Some(LspSendOutcomeV1::Unavailable);
        let changed = SemanticSnapshotV1::try_new(
            target(1),
            "rust".to_owned(),
            "fn changed() {}".to_owned(),
            vellum_semantic_service::SemanticServiceLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(
            adapter.synchronize(&SemanticSnapshotUpdateV1::Admit(changed)),
            SemanticProviderAdmissionV1::Unavailable
        );
        assert_eq!(adapter.usage().snapshots_changed, 0);
    }

    #[test]
    fn queries_are_unsupported_when_server_capabilities_are_absent() {
        let mut transport = MemoryTransport::default();
        transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "id": 1, "result": {"capabilities": {}}
            }))));
        let mut adapter = LspAdapterV1::try_new_with_readiness(
            transport,
            Issuer(0),
            ScriptedReadiness::current(),
            provenance(),
            LspLimitsV1::reference(),
            SemanticLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(adapter.start(), LspStartOutcomeV1::Started);
        assert_eq!(adapter.poll(), None);
        let request = SemanticRequestV1 {
            request: SemanticRequestRefV1::new([1; 16]),
            target: target(0),
            expected_index: None,
            language: SemanticTextV1::try_new("rust".to_owned(), 16).unwrap(),
            query: vellum_host::semantic::SemanticQueryV1::Diagnostics,
        };
        assert_eq!(
            adapter.submit(&request),
            SemanticProviderSubmissionV1::Unsupported(
                vellum_host::semantic::SemanticQueryKindV1::Diagnostics,
            )
        );
        assert_eq!(adapter.usage().unsupported_queries, 1);
        let rename = rename_request(6, 0);
        assert_eq!(
            adapter.submit(&rename),
            SemanticProviderSubmissionV1::Unsupported(
                vellum_host::semantic::SemanticQueryKindV1::PlanRename,
            )
        );
        assert_eq!(adapter.usage().unsupported_queries, 2);
    }

    #[test]
    fn virtual_document_uris_are_injective() {
        let mut adapter = ready_adapter();
        let uri = LspVirtualDocumentV1::try_new(
            "vellum-lsp://shared".to_owned(),
            LspLimitsV1::reference(),
        )
        .unwrap();
        assert!(
            adapter.register_virtual_document(SemanticDocumentRefV1::new([1; 16]), uri.clone())
        );
        assert!(!adapter.register_virtual_document(SemanticDocumentRefV1::new([2; 16]), uri));
    }

    #[test]
    fn virtual_document_construction_proves_a_path_free_opaque_grammar() {
        let limits = LspLimitsV1::reference();
        let valid =
            LspVirtualDocumentV1::try_new("vellum-lsp://A7._opaque-token9".to_owned(), limits)
                .unwrap();
        assert_eq!(format!("{valid:?}"), "LspVirtualDocumentV1(<opaque>)");
        for hostile in [
            "vellum-lsp://",
            "vellum-lsp:///absolute",
            "vellum-lsp://a/plain/path",
            "vellum-lsp://a\\windows",
            "vellum-lsp://a%2fpath",
            "vellum-lsp://a%2Fpath",
            "vellum-lsp://a%5cpath",
            "vellum-lsp://a?query",
            "vellum-lsp://a#fragment",
            "vellum-lsp://../escape",
            "vellum-lsp://user@host",
            "vellum-lsp://host:99",
            "file:///tmp/source.rs",
        ] {
            assert!(
                LspVirtualDocumentV1::try_new(hostile.to_owned(), limits).is_none(),
                "accepted hostile virtual URI shape"
            );
        }
    }

    #[test]
    fn workspace_relative_document_construction_is_canonical_and_content_safe() {
        let limits = LspLimitsV1::reference();
        let relative =
            LspDocumentLocatorV1::try_workspace_relative("src/my file-ß.rs".to_owned(), limits)
                .unwrap();
        assert_eq!(
            format!("{relative:?}"),
            "LspVirtualDocumentV1(<workspace-relative>)"
        );
        assert!(!format!("{relative:?}").contains("my file"));
        for hostile in [
            "",
            "/absolute.rs",
            ".",
            "..",
            "./source.rs",
            "src/../escape.rs",
            "src/./source.rs",
            "src//source.rs",
            "src/",
            "src\\source.rs",
            "src%2fsource.rs",
            "src?query.rs",
            "src#fragment.rs",
            "src/line\nbreak.rs",
        ] {
            assert!(
                LspDocumentLocatorV1::try_workspace_relative(hostile.to_owned(), limits).is_none(),
                "accepted non-canonical relative document locator"
            );
        }
    }

    #[test]
    fn workspace_relative_document_is_root_bound_and_publishes_exact_file_uri() {
        let limits = LspLimitsV1::reference();
        let initialization = LspInitializationProfileV1::try_workspace(
            "file:///qualified-root".to_owned(),
            "workspace".to_owned(),
            None,
            limits,
        )
        .unwrap();
        let mut transport = MemoryTransport::default();
        transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "id": 1,
                "result": {"capabilities": {"positionEncoding": "utf-8"}}
            }))));
        let mut adapter = LspAdapterV1::try_new_with_profile(
            transport,
            Issuer(0),
            ScriptedReadiness::current(),
            LspExplicitVirtualDocumentsV1,
            LspAdapterProfileV1::try_new(
                provenance(),
                limits,
                SemanticLimitsV1::reference(),
                initialization,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(adapter.start(), LspStartOutcomeV1::Started);
        assert_eq!(
            framed_json(adapter.transport.sends.first().unwrap())
                .pointer("/params/rootUri")
                .and_then(Value::as_str),
            Some("file:///qualified-root")
        );
        assert_eq!(adapter.poll(), None);
        assert!(matches!(
            adapter.session_state(),
            LspSessionStateV1::Ready { .. }
        ));

        let relative =
            LspDocumentLocatorV1::try_workspace_relative("src/my file-ß.rs".to_owned(), limits)
                .unwrap();
        assert!(adapter.register_virtual_document(target(0).document, relative));
        let snapshot = SemanticSnapshotV1::try_new(
            target(0),
            "rust".to_owned(),
            "fn main() {}".to_owned(),
            vellum_semantic_service::SemanticServiceLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(
            adapter.synchronize(&SemanticSnapshotUpdateV1::Admit(snapshot)),
            SemanticProviderAdmissionV1::Accepted
        );
        let initialized = framed_json(adapter.transport.sends.get(1).unwrap());
        assert_eq!(
            initialized.get("method").and_then(Value::as_str),
            Some("initialized")
        );
        assert_eq!(initialized.get("params"), Some(&json!({})));
        let did_open = framed_json(adapter.transport.sends.get(2).unwrap());
        assert_eq!(
            did_open.get("method").and_then(Value::as_str),
            Some("textDocument/didOpen")
        );
        assert_eq!(
            did_open
                .pointer("/params/textDocument/uri")
                .and_then(Value::as_str),
            Some("file:///qualified-root/src/my%20file-%C3%9F.rs")
        );

        let mut unbound = ready_adapter();
        let relative =
            LspDocumentLocatorV1::try_workspace_relative("src/lib.rs".to_owned(), limits).unwrap();
        assert!(!unbound.register_virtual_document(target(0).document, relative));
    }

    #[test]
    fn dynamic_virtual_document_admit_change_close_and_reopen_reuse_one_uri() {
        let uri = "vellum-lsp://opaque-seven";
        let mut adapter =
            ready_dynamic_adapter(VirtualIssuer::new(&[uri]), LspLimitsV1::reference());
        for (revision, source) in [(0, "zero"), (1, "one")] {
            let snapshot = SemanticSnapshotV1::try_new(
                target(revision),
                "rust".to_owned(),
                source.to_owned(),
                vellum_semantic_service::SemanticServiceLimitsV1::reference(),
            )
            .unwrap();
            assert_eq!(
                adapter.synchronize(&SemanticSnapshotUpdateV1::Admit(snapshot)),
                SemanticProviderAdmissionV1::Accepted
            );
        }
        assert_eq!(
            adapter.synchronize(&SemanticSnapshotUpdateV1::Close { target: target(1) }),
            SemanticProviderAdmissionV1::Accepted
        );
        let reopened = SemanticSnapshotV1::try_new(
            target(2),
            "rust".to_owned(),
            "two".to_owned(),
            vellum_semantic_service::SemanticServiceLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(
            adapter.synchronize(&SemanticSnapshotUpdateV1::Admit(reopened)),
            SemanticProviderAdmissionV1::Accepted
        );
        assert_eq!(adapter.virtual_documents.issued, 1);
        let publications: Vec<_> = adapter.transport.sends[2..]
            .iter()
            .map(|frame| framed_json(frame))
            .collect();
        assert_eq!(
            publications
                .iter()
                .map(|message| {
                    message
                        .pointer("/params/textDocument/uri")
                        .and_then(Value::as_str)
                        .unwrap()
                })
                .collect::<Vec<_>>(),
            vec![uri, uri, uri, uri]
        );
        assert_eq!(adapter.usage().snapshots_opened, 2);
        assert_eq!(adapter.usage().snapshots_changed, 1);
        assert_eq!(adapter.usage().snapshots_closed, 1);
    }

    #[test]
    fn dynamic_virtual_document_is_deterministic_without_pre_send_state_and_refuses_bounds() {
        let shared = "vellum-lsp://shared";
        let mut adapter = ready_dynamic_adapter(
            VirtualIssuer::new(&[shared, shared]),
            LspLimitsV1::reference(),
        );
        adapter.transport.next_send = Some(LspSendOutcomeV1::Unavailable);
        let first = SemanticSnapshotV1::try_new(
            target(0),
            "rust".to_owned(),
            "first".to_owned(),
            vellum_semantic_service::SemanticServiceLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(
            adapter.synchronize(&SemanticSnapshotUpdateV1::Admit(first.clone())),
            SemanticProviderAdmissionV1::Unavailable
        );
        assert_eq!(adapter.virtual_documents.issued, 1);
        assert!(!adapter.documents.contains_key(&target(0).document));
        assert_eq!(adapter.usage().snapshots_opened, 0);
        assert_eq!(
            adapter.synchronize(&SemanticSnapshotUpdateV1::Admit(first)),
            SemanticProviderAdmissionV1::Accepted
        );
        assert_eq!(adapter.virtual_documents.issued, 1);

        let other_target = SemanticTargetV1 {
            document: SemanticDocumentRefV1::new([8; 16]),
            source_revision: SemanticSourceRevisionV1::new(0),
        };
        let collision = SemanticSnapshotV1::try_new(
            other_target,
            "rust".to_owned(),
            "other".to_owned(),
            vellum_semantic_service::SemanticServiceLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(
            adapter.synchronize(&SemanticSnapshotUpdateV1::Admit(collision)),
            SemanticProviderAdmissionV1::Refused
        );
        assert_eq!(adapter.virtual_documents.issued, 2);

        let mut limits = LspLimitsV1::reference();
        limits.max_documents = 1;
        let mut bounded = ready_dynamic_adapter(
            VirtualIssuer::new(&["vellum-lsp://one", "vellum-lsp://two"]),
            limits,
        );
        let one = SemanticSnapshotV1::try_new(
            target(0),
            "rust".to_owned(),
            "one".to_owned(),
            vellum_semantic_service::SemanticServiceLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(
            bounded.synchronize(&SemanticSnapshotUpdateV1::Admit(one)),
            SemanticProviderAdmissionV1::Accepted
        );
        let two = SemanticSnapshotV1::try_new(
            other_target,
            "rust".to_owned(),
            "two".to_owned(),
            vellum_semantic_service::SemanticServiceLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(
            bounded.synchronize(&SemanticSnapshotUpdateV1::Admit(two)),
            SemanticProviderAdmissionV1::Refused
        );
        assert_eq!(bounded.virtual_documents.issued, 1);
        assert_eq!(
            format!("{:?}", bounded.documents[&target(0).document].uri),
            "LspVirtualDocumentV1(<opaque>)"
        );
    }

    #[test]
    fn dynamic_documents_preserve_client_partitions_and_replay_into_a_fresh_adapter() {
        use vellum_semantic_service::{
            SemanticAdmissionOutcomeV1, SemanticClientRefV1, SemanticServiceRefusalV1,
            SemanticServiceV1,
        };

        let old = ready_dynamic_adapter_with(
            VirtualIssuer::new(&["vellum-lsp://old-seven", "vellum-lsp://old-eight"]),
            LspLimitsV1::reference(),
            9,
            0,
        );
        let mut service = SemanticServiceV1::try_new(
            old,
            vellum_semantic_service::SemanticServiceLimitsV1::reference(),
        )
        .unwrap();
        let first_client = SemanticClientRefV1::new([1; 16]);
        let second_client = SemanticClientRefV1::new([2; 16]);
        service.admit_client(first_client).unwrap();
        service.admit_client(second_client).unwrap();
        let second_target = SemanticTargetV1 {
            document: SemanticDocumentRefV1::new([8; 16]),
            source_revision: SemanticSourceRevisionV1::new(0),
        };
        let snapshot = |target, source: &str| {
            SemanticSnapshotV1::try_new(
                target,
                "rust".to_owned(),
                source.to_owned(),
                vellum_semantic_service::SemanticServiceLimitsV1::reference(),
            )
            .unwrap()
        };
        assert_eq!(
            service.open_snapshot(first_client, snapshot(target(0), "seven")),
            SemanticAdmissionOutcomeV1::Accepted
        );
        assert_eq!(
            service.open_snapshot(second_client, snapshot(second_target, "eight")),
            SemanticAdmissionOutcomeV1::Accepted
        );
        assert_eq!(
            service.change_snapshot(
                first_client,
                snapshot(
                    SemanticTargetV1 {
                        source_revision: SemanticSourceRevisionV1::new(1),
                        ..second_target
                    },
                    "foreign",
                ),
            ),
            SemanticAdmissionOutcomeV1::Refused(SemanticServiceRefusalV1::ClientIsolation)
        );

        let replacement = ready_dynamic_adapter_with(
            VirtualIssuer::new(&["vellum-lsp://new-seven", "vellum-lsp://new-eight"]),
            LspLimitsV1::reference(),
            10,
            1,
        );
        let retirement = service.try_replace_provider(replacement).unwrap();
        assert_eq!(service.usage().provider_replacements, 1);
        assert_eq!(service.usage().snapshots_replayed, 2);
        assert!(retirement.into_provider_if_drained().is_ok());
    }

    #[test]
    fn diagnostics_symbols_and_cancellation_are_correlated_and_bounded() {
        let mut adapter = ready_adapter();
        open_document(&mut adapter);
        assert_eq!(adapter.usage().source_bytes_current, 14);
        assert_eq!(adapter.usage().source_bytes_high_water, 14);
        assert_eq!(adapter.state().capabilities, SemanticCapabilitySetV1::top());
        let diagnostic_request = SemanticRequestV1 {
            request: SemanticRequestRefV1::new([1; 16]),
            target: target(0),
            expected_index: SemanticIndexGenerationV1::new(1),
            language: SemanticTextV1::try_new("rust".to_owned(), 16).unwrap(),
            query: vellum_host::semantic::SemanticQueryV1::Diagnostics,
        };
        assert_eq!(
            adapter.submit(&diagnostic_request),
            SemanticProviderSubmissionV1::Accepted
        );
        adapter.transport.receives.push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {"kind": "full", "items": [{
                "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 2}},
                "severity": 2,
                "message": "warning",
                "code": "W1"
            }]}
        }))));
        let completion = adapter.poll().unwrap();
        let SemanticOutcomeV1::Complete(reply) = completion.outcome else {
            panic!("expected completed diagnostics")
        };
        assert_eq!(reply.request, diagnostic_request.request);
        assert_eq!(reply.result.diagnostics().unwrap().len(), 1);

        let symbol_request = SemanticRequestV1 {
            request: SemanticRequestRefV1::new([2; 16]),
            target: target(0),
            expected_index: SemanticIndexGenerationV1::new(1),
            language: SemanticTextV1::try_new("rust".to_owned(), 16).unwrap(),
            query: vellum_host::semantic::SemanticQueryV1::Symbols {
                needle: SemanticTextV1::try_new("alpha".to_owned(), 16).unwrap(),
            },
        };
        assert_eq!(
            adapter.submit(&symbol_request),
            SemanticProviderSubmissionV1::Accepted
        );
        adapter.transport.receives.push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
            "jsonrpc": "2.0",
            "id": 3,
            "result": [{
                "name": "alpha",
                "kind": 12,
                "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 13}},
                "selectionRange": {"start": {"line": 0, "character": 3}, "end": {"line": 0, "character": 8}}
            }]
        }))));
        let completion = adapter.poll().unwrap();
        let SemanticOutcomeV1::Complete(reply) = completion.outcome else {
            panic!("expected completed symbols")
        };
        assert_eq!(reply.result.symbols().unwrap().len(), 1);

        let cancelled = SemanticRequestV1 {
            request: SemanticRequestRefV1::new([3; 16]),
            ..diagnostic_request
        };
        assert_eq!(
            adapter.submit(&cancelled),
            SemanticProviderSubmissionV1::Accepted
        );
        adapter.cancel(cancelled.request);
        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "id": 4, "result": {"kind": "full", "items": []}
            }))));
        assert_eq!(
            adapter.poll(),
            Some(SemanticProviderCompletionV1 {
                request: cancelled.request,
                outcome: SemanticOutcomeV1::Unavailable,
            })
        );
        assert_eq!(adapter.usage().cancellations, 1);
        assert_eq!(adapter.usage().orphan_messages, 0);
        assert_eq!(adapter.usage().cancelled_responses_correlated, 1);
    }

    #[test]
    fn navigation_references_implementations_and_hover_use_exact_open_snapshots() {
        let mut adapter = ready_adapter();
        open_document(&mut adapter);

        let definition = point_request(
            20,
            vellum_host::semantic::SemanticQueryV1::Definition { offset: 3 },
        );
        assert_eq!(
            adapter.submit(&definition),
            SemanticProviderSubmissionV1::Accepted
        );
        let emitted = framed_json(adapter.transport.sends.last().unwrap());
        assert_eq!(emitted["method"], "textDocument/definition");
        assert_eq!(
            emitted["params"]["position"],
            json!({"line": 0, "character": 3})
        );
        adapter.transport.receives.push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
            "jsonrpc": "2.0", "id": 2,
            "result": {
                "uri": "vellum-lsp://seven",
                "range": {"start": {"line": 0, "character": 3}, "end": {"line": 0, "character": 8}}
            }
        }))));
        let SemanticOutcomeV1::Complete(reply) = adapter.poll().unwrap().outcome else {
            panic!("definition must complete")
        };
        assert_eq!(reply.result.locations().unwrap()[0].target, target(0));
        assert_eq!(
            reply.result.locations().unwrap()[0].span,
            SemanticSpanV1::try_new(3, 8).unwrap()
        );

        let references = point_request(
            21,
            vellum_host::semantic::SemanticQueryV1::References { offset: 3 },
        );
        assert_eq!(
            adapter.submit(&references),
            SemanticProviderSubmissionV1::Accepted
        );
        let emitted = framed_json(adapter.transport.sends.last().unwrap());
        assert_eq!(emitted["method"], "textDocument/references");
        assert_eq!(emitted["params"]["context"]["includeDeclaration"], true);
        adapter.transport.receives.push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
            "jsonrpc": "2.0", "id": 3,
            "result": [{
                "uri": "vellum-lsp://seven",
                "range": {"start": {"line": 0, "character": 3}, "end": {"line": 0, "character": 8}}
            }]
        }))));
        let SemanticOutcomeV1::Complete(reply) = adapter.poll().unwrap().outcome else {
            panic!("references must complete")
        };
        assert_eq!(reply.result.locations().unwrap().len(), 1);

        let implementations = point_request(
            22,
            vellum_host::semantic::SemanticQueryV1::Implementations { offset: 3 },
        );
        assert_eq!(
            adapter.submit(&implementations),
            SemanticProviderSubmissionV1::Accepted
        );
        assert_eq!(
            framed_json(adapter.transport.sends.last().unwrap())["method"],
            "textDocument/implementation"
        );
        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "id": 4, "result": null
            }))));
        let SemanticOutcomeV1::Complete(reply) = adapter.poll().unwrap().outcome else {
            panic!("empty implementations must complete")
        };
        assert!(reply.result.locations().unwrap().is_empty());

        let hover = point_request(
            23,
            vellum_host::semantic::SemanticQueryV1::TypeAtPoint { offset: 3 },
        );
        assert_eq!(
            adapter.submit(&hover),
            SemanticProviderSubmissionV1::Accepted
        );
        assert_eq!(
            framed_json(adapter.transport.sends.last().unwrap())["method"],
            "textDocument/hover"
        );
        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "id": 5,
                "result": {"contents": {"kind": "markdown", "value": "```rust\nfn alpha()\n```"}}
            }))));
        let SemanticOutcomeV1::Complete(reply) = adapter.poll().unwrap().outcome else {
            panic!("hover must complete")
        };
        assert_eq!(
            reply.result.type_at_point().unwrap().as_str(),
            "```rust\nfn alpha()\n```"
        );
    }

    #[test]
    fn call_hierarchy_preserves_prepared_item_and_exact_snapshot_provenance() {
        let mut adapter = ready_adapter();
        open_document(&mut adapter);
        let request = point_request(
            30,
            vellum_host::semantic::SemanticQueryV1::IncomingCalls { offset: 3 },
        );
        assert_eq!(
            adapter.submit(&request),
            SemanticProviderSubmissionV1::Accepted
        );
        let prepare = framed_json(adapter.transport.sends.last().unwrap());
        assert_eq!(prepare["method"], "textDocument/prepareCallHierarchy");
        assert_eq!(
            prepare["params"]["position"],
            json!({"line":0,"character":3})
        );

        let prepared = json!({
            "name":"alpha", "kind":12, "uri":"vellum-lsp://seven",
            "range":{"start":{"line":0,"character":0},"end":{"line":0,"character":13}},
            "selectionRange":{"start":{"line":0,"character":3},"end":{"line":0,"character":8}},
            "data":{"providerToken":"opaque-7"}
        });
        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc":"2.0", "id":2, "result":[prepared.clone()]
            }))));
        assert!(adapter.poll().is_none());
        let incoming = framed_json(adapter.transport.sends.last().unwrap());
        assert_eq!(incoming["method"], "callHierarchy/incomingCalls");
        assert_eq!(incoming["params"]["item"], prepared);

        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc":"2.0", "id":3, "result":[{
                "from": prepared.clone(),
                    "fromRanges":[{"start":{"line":0,"character":3},"end":{"line":0,"character":8}}]
                }]
            }))));
        let SemanticOutcomeV1::Complete(reply) = adapter.poll().unwrap().outcome else {
            panic!("incoming calls must complete")
        };
        assert_eq!(reply.target, target(0));
        assert_eq!(
            reply.index_generation,
            SemanticIndexGenerationV1::new(1).unwrap()
        );
        assert_eq!(reply.provider, provenance());
        assert_eq!(reply.result.locations().unwrap()[0].target, target(0));
        assert_eq!(
            reply.result.locations().unwrap()[0].span,
            SemanticSpanV1::try_new(3, 8).unwrap()
        );

        let outgoing_request = point_request(
            32,
            vellum_host::semantic::SemanticQueryV1::OutgoingCalls { offset: 3 },
        );
        assert_eq!(
            adapter.submit(&outgoing_request),
            SemanticProviderSubmissionV1::Accepted
        );
        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc":"2.0", "id":4, "result":[prepared.clone()]
            }))));
        assert!(adapter.poll().is_none());
        let outgoing = framed_json(adapter.transport.sends.last().unwrap());
        assert_eq!(outgoing["method"], "callHierarchy/outgoingCalls");
        assert_eq!(outgoing["params"]["item"], prepared);
        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc":"2.0", "id":5, "result":[{
                    "to": prepared,
                    "fromRanges":[{"start":{"line":0,"character":3},"end":{"line":0,"character":8}}]
                }]
            }))));
        let SemanticOutcomeV1::Complete(reply) = adapter.poll().unwrap().outcome else {
            panic!("outgoing calls must complete")
        };
        assert_eq!(reply.result.locations().unwrap()[0].target, target(0));
    }

    #[test]
    fn call_hierarchy_refuses_ambiguous_prepare_items_without_followup() {
        let mut adapter = ready_adapter();
        open_document(&mut adapter);
        let request = point_request(
            31,
            vellum_host::semantic::SemanticQueryV1::OutgoingCalls { offset: 3 },
        );
        assert_eq!(
            adapter.submit(&request),
            SemanticProviderSubmissionV1::Accepted
        );
        let item = json!({
            "name":"alpha", "kind":12, "uri":"vellum-lsp://seven",
            "range":{"start":{"line":0,"character":0},"end":{"line":0,"character":13}},
            "selectionRange":{"start":{"line":0,"character":3},"end":{"line":0,"character":8}}
        });
        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc":"2.0", "id":2, "result":[item.clone(), item]
            }))));
        let completion = adapter.poll().unwrap();
        assert_eq!(
            completion.outcome,
            SemanticOutcomeV1::Refused(SemanticRefusalV1::OutputLimit)
        );
        assert_eq!(
            framed_json(adapter.transport.sends.last().unwrap())["method"],
            "textDocument/prepareCallHierarchy"
        );
    }

    #[test]
    fn navigation_rejects_unbound_provider_locations() {
        let mut adapter = ready_adapter();
        open_document(&mut adapter);
        let definition = point_request(
            24,
            vellum_host::semantic::SemanticQueryV1::Definition { offset: 3 },
        );
        assert_eq!(
            adapter.submit(&definition),
            SemanticProviderSubmissionV1::Accepted
        );
        adapter.transport.receives.push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
            "jsonrpc": "2.0", "id": 2,
            "result": {
                "uri": "file:///untrusted/foreign.rs",
                "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 1}}
            }
        }))));
        assert_eq!(
            adapter.poll().unwrap().outcome,
            SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply)
        );
        assert_eq!(adapter.usage().malformed_messages, 1);
    }

    #[test]
    fn static_rename_capability_emits_exact_request_and_versioned_review_plan() {
        let mut adapter = ready_adapter();
        open_document(&mut adapter);
        assert!(
            adapter
                .state()
                .capabilities
                .contains(SemanticCapabilityV1::PlanRename)
        );
        let request = rename_request(8, 3);
        assert_eq!(
            adapter.submit(&request),
            SemanticProviderSubmissionV1::Accepted
        );
        let emitted = framed_json(adapter.transport.sends.last().unwrap());
        assert_eq!(
            emitted.get("method").and_then(Value::as_str),
            Some("textDocument/rename")
        );
        assert_eq!(emitted.pointer("/params/position/line"), Some(&json!(0)));
        assert_eq!(
            emitted.pointer("/params/position/character"),
            Some(&json!(3))
        );
        assert_eq!(emitted.pointer("/params/newName"), Some(&json!("beta")));

        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0",
                "id": 2,
                "result": {"documentChanges": [{
                    "textDocument": {
                        "uri": "vellum-lsp://seven",
                        "version": 1
                    },
                    "edits": [{
                        "range": {
                            "start": {"line": 0, "character": 3},
                            "end": {"line": 0, "character": 8}
                        },
                        "newText": "beta"
                    }]
                }]}
            }))));
        let completion = adapter.poll().unwrap();
        let SemanticOutcomeV1::Complete(reply) = completion.outcome else {
            panic!("expected a completed rename plan")
        };
        let edits = reply.result.rename_plan().unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].location.target, target(0));
        assert_eq!(
            edits[0].location.span,
            SemanticSpanV1::try_new(3, 8).unwrap()
        );
        assert_eq!(edits[0].replacement.as_str(), "beta");
        assert_eq!(adapter.usage().rename_requests_submitted, 1);
        assert_eq!(adapter.usage().rename_plans_completed, 1);
        assert_eq!(adapter.usage().rename_plans_refused, 0);
    }

    #[test]
    fn duplicate_json_keys_are_rejected_before_protocol_interpretation() {
        let mut transport = MemoryTransport::default();
        transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed_raw(
                r#"{"jsonrpc":"2.0","jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}"#,
            )));
        let mut initializing =
            LspAdapterV1::try_new(transport, Issuer(0), provenance(), LspLimitsV1::reference())
                .unwrap();
        assert_eq!(initializing.start(), LspStartOutcomeV1::Started);
        assert_eq!(initializing.poll(), None);
        assert_eq!(initializing.session_state(), LspSessionStateV1::Failed);
        assert_eq!(initializing.usage().malformed_messages, 1);

        let duplicate_rename_replies = [
            r#"{"jsonrpc":"2.0","id":2,"result":null,"result":{"documentChanges":[]}}"#,
            r#"{"jsonrpc":"2.0","id":2,"result":{"documentChanges":[],"documentChanges":[]}}"#,
            r#"{"jsonrpc":"2.0","id":2,"result":{"documentChanges":[{"textDocument":{"uri":"vellum-lsp://seven","uri":"vellum-lsp://seven","version":1},"edits":[]}]}}"#,
            r#"{"jsonrpc":"2.0","id":2,"result":{"documentChanges":[{"textDocument":{"uri":"vellum-lsp://seven","version":1,"version":1},"edits":[]}]}}"#,
            r#"{"jsonrpc":"2.0","id":2,"result":{"documentChanges":[{"textDocument":{"uri":"vellum-lsp://seven","version":1},"edits":[{"range":{"start":{"line":0,"character":3},"end":{"line":0,"character":8}},"range":{"start":{"line":0,"character":3},"end":{"line":0,"character":8}},"newText":"beta"}]}]}}"#,
            r#"{"jsonrpc":"2.0","id":2,"result":{"documentChanges":[{"textDocument":{"uri":"vellum-lsp://seven","version":1},"edits":[{"range":{"start":{"line":0,"character":3},"end":{"line":0,"character":8}},"newText":"beta","newText":"beta"}]}]}}"#,
            r#"{"jsonrpc":"2.0","id":2,"result":{"documentChanges":[{"textDocument":{"uri":"vellum-lsp://seven","version":1},"edits":[{"range":{"start":{"line":0,"line":0,"character":3},"end":{"line":0,"character":8}},"newText":"beta"}]}]}}"#,
        ];
        for (index, reply) in duplicate_rename_replies.into_iter().enumerate() {
            let mut adapter = ready_adapter();
            open_document(&mut adapter);
            assert_eq!(
                adapter.submit(&rename_request(index as u8 + 70, 3)),
                SemanticProviderSubmissionV1::Accepted
            );
            adapter
                .transport
                .receives
                .push_back(LspReceiveOutcomeV1::Chunk(framed_raw(reply)));
            assert_eq!(adapter.poll(), None);
            assert_eq!(adapter.session_state(), LspSessionStateV1::Failed);
            assert_eq!(adapter.usage().malformed_messages, 1);
            assert_eq!(adapter.usage().rename_plans_completed, 0);
        }

        let mut diagnostics = ready_adapter();
        open_document(&mut diagnostics);
        let request = SemanticRequestV1 {
            request: SemanticRequestRefV1::new([99; 16]),
            target: target(0),
            expected_index: SemanticIndexGenerationV1::new(1),
            language: SemanticTextV1::try_new("rust".to_owned(), 16).unwrap(),
            query: vellum_host::semantic::SemanticQueryV1::Diagnostics,
        };
        assert_eq!(
            diagnostics.submit(&request),
            SemanticProviderSubmissionV1::Accepted
        );
        diagnostics
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed_raw(
                r#"{"jsonrpc":"2.0","id":2,"result":{"kind":"full","items":[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":2}},"message":"first","message":"second"}]}}"#,
            )));
        assert_eq!(diagnostics.poll(), None);
        assert_eq!(diagnostics.session_state(), LspSessionStateV1::Failed);
        assert_eq!(diagnostics.usage().malformed_messages, 1);
    }

    #[test]
    fn rename_refuses_unversioned_foreign_annotated_resource_and_overlapping_edits() {
        let invalid_results = [
            json!({"changes": {"vellum-lsp://seven": []}}),
            json!({"documentChanges": [{
                "kind": "create", "uri": "vellum-lsp://seven"
            }]}),
            json!({"documentChanges": [{
                "textDocument": {
                    "uri": "vellum-lsp://foreign", "version": 1
                },
                "edits": []
            }]}),
            json!({"documentChanges": [{
                "textDocument": {
                    "uri": "vellum-lsp://seven", "version": 99
                },
                "edits": []
            }]}),
            json!({"documentChanges": [{
                "textDocument": {"uri": "vellum-lsp://seven"},
                "edits": []
            }]}),
            json!({"documentChanges": [{
                "textDocument": {
                    "uri": "vellum-lsp://seven", "version": 1
                },
                "edits": [{
                    "range": {
                        "start": {"line": 0, "character": 3},
                        "end": {"line": 0, "character": 8}
                    },
                    "newText": "beta",
                    "annotationId": "approval"
                }]
            }]}),
            json!({"documentChanges": [{
                "textDocument": {
                    "uri": "vellum-lsp://seven", "version": 1
                },
                "edits": [
                    {
                        "range": {
                            "start": {"line": 0, "character": 3},
                            "end": {"line": 0, "character": 8}
                        },
                        "newText": "beta"
                    },
                    {
                        "range": {
                            "start": {"line": 0, "character": 7},
                            "end": {"line": 0, "character": 9}
                        },
                        "newText": "beta"
                    }
                ]
            }]}),
        ];
        for (index, result) in invalid_results.into_iter().enumerate() {
            let mut adapter = ready_adapter();
            open_document(&mut adapter);
            assert_eq!(
                adapter.submit(&rename_request(index as u8 + 20, 3)),
                SemanticProviderSubmissionV1::Accepted
            );
            adapter
                .transport
                .receives
                .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                    "jsonrpc": "2.0", "id": 2, "result": result
                }))));
            assert_eq!(
                adapter.poll().unwrap().outcome,
                SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply)
            );
            assert_eq!(adapter.usage().rename_plans_refused, 1);
        }
    }

    #[test]
    fn rename_refuses_split_utf16_ranges_and_accepts_spec_null_as_empty_plan() {
        let mut adapter = ready_adapter();
        open_document(&mut adapter);
        let changed = SemanticSnapshotV1::try_new(
            target(1),
            "rust".to_owned(),
            "fn 😀alpha() {}\n".to_owned(),
            vellum_semantic_service::SemanticServiceLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(
            adapter.synchronize(&SemanticSnapshotUpdateV1::Admit(changed)),
            SemanticProviderAdmissionV1::Accepted
        );
        let mut request = rename_request(40, 7);
        request.target = target(1);
        assert_eq!(
            adapter.submit(&request),
            SemanticProviderSubmissionV1::Accepted
        );
        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "id": 2,
                "result": {"documentChanges": [{
                    "textDocument": {
                        "uri": "vellum-lsp://seven", "version": 2
                    },
                    "edits": [{
                        "range": {
                            "start": {"line": 0, "character": 4},
                            "end": {"line": 0, "character": 5}
                        },
                        "newText": "beta"
                    }]
                }]}
            }))));
        assert_eq!(
            adapter.poll().unwrap().outcome,
            SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply)
        );

        let mut request = rename_request(41, 7);
        request.target = target(1);
        assert_eq!(
            adapter.submit(&request),
            SemanticProviderSubmissionV1::Accepted
        );
        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "id": 3, "result": Value::Null
            }))));
        let SemanticOutcomeV1::Complete(reply) = adapter.poll().unwrap().outcome else {
            panic!("LSP null is an exact empty plan")
        };
        assert!(reply.result.rename_plan().unwrap().is_empty());
    }

    #[test]
    fn diagnostic_output_overflow_refuses_without_truncation() {
        let mut adapter = ready_adapter();
        open_document(&mut adapter);
        let request = SemanticRequestV1 {
            request: SemanticRequestRefV1::new([9; 16]),
            target: target(0),
            expected_index: SemanticIndexGenerationV1::new(1),
            language: SemanticTextV1::try_new("rust".to_owned(), 16).unwrap(),
            query: vellum_host::semantic::SemanticQueryV1::Diagnostics,
        };
        assert_eq!(
            adapter.submit(&request),
            SemanticProviderSubmissionV1::Accepted
        );
        let item = json!({
            "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 2}},
            "severity": 3,
            "message": "overflow"
        });
        let items = vec![item; SemanticLimitsV1::reference().max_results + 1];
        adapter
            .transport
            .receives
            .push_back(LspReceiveOutcomeV1::Chunk(framed(json!({
                "jsonrpc": "2.0", "id": 2, "result": {"kind": "full", "items": items}
            }))));
        let completion = adapter.poll().unwrap();
        assert_eq!(
            completion.outcome,
            SemanticOutcomeV1::Refused(SemanticRefusalV1::OutputLimit)
        );
        assert_eq!(adapter.usage().output_refused, 1);
    }
}
