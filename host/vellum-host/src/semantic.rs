//! Bounded outbound contract for a peer language-semantic service.
//!
//! Vellum owns the source revision to which a question is bound. The peer owns
//! its index generation and language-specific meaning. This contract does not
//! implement an LSP, PSI, nREPL, editor, process launcher, or transport. In
//! particular, rename produces a reviewable plan and never edit authority.

use std::{fmt, num::NonZeroU64};

pub const MAX_SEMANTIC_QUERY_BYTES_V1: usize = 256;
pub const MAX_SEMANTIC_LABEL_BYTES_V1: usize = 512;
pub const MAX_SEMANTIC_DETAIL_BYTES_V1: usize = 4096;
pub const MAX_SEMANTIC_PROVIDER_PROFILE_BYTES_V1: usize = 128;
pub const MAX_SEMANTIC_PROVIDER_VERSION_BYTES_V1: usize = 128;
pub const MAX_SEMANTIC_RESULTS_V1: usize = 256;
pub const MAX_SEMANTIC_RENAME_EDITS_V1: usize = 1024;
pub const MAX_SEMANTIC_REQUESTS_V1: u64 = 65_536;

macro_rules! opaque_ref {
    ($name:ident) => {
        #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; 16]);

        impl $name {
            pub const fn new(value: [u8; 16]) -> Self {
                Self(value)
            }

            /// Canonical opaque bytes for a protocol projection. Possession is
            /// correlation evidence only; it is never authentication or authority.
            pub const fn as_bytes(self) -> [u8; 16] {
                self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!(stringify!($name), "(<opaque>)"))
            }
        }
    };
}

opaque_ref!(SemanticRequestRefV1);
opaque_ref!(SemanticDocumentRefV1);
opaque_ref!(SemanticProviderRefV1);

/// Closed reason why a composition-owned semantic request ceased to be
/// eligible for an ordinary provider completion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticInvalidationCauseV1 {
    SourceChanged,
    SourceClosed,
    ProviderStateChanged,
}

/// Construction failures for an exact invalidation receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticInvalidationReceiptErrorV1 {
    InvalidLimit,
    CollectionLimit,
    NonCanonical,
}

/// Exact, bounded evidence that one closed cause invalidated a canonical set
/// of semantic requests. Request identities are strictly ascending so the
/// value has one representation and cannot carry duplicates.
#[must_use = "invalidation evidence must be reconciled or explicitly discarded"]
#[derive(Clone, Eq, PartialEq)]
pub struct SemanticInvalidationReceiptV1 {
    cause: SemanticInvalidationCauseV1,
    requests: Vec<SemanticRequestRefV1>,
}

impl SemanticInvalidationReceiptV1 {
    pub fn try_new(
        cause: SemanticInvalidationCauseV1,
        requests: Vec<SemanticRequestRefV1>,
        max_requests: u64,
    ) -> Result<Self, SemanticInvalidationReceiptErrorV1> {
        if max_requests == 0 || max_requests > MAX_SEMANTIC_REQUESTS_V1 {
            return Err(SemanticInvalidationReceiptErrorV1::InvalidLimit);
        }
        if u64::try_from(requests.len()).unwrap_or(u64::MAX) > max_requests {
            return Err(SemanticInvalidationReceiptErrorV1::CollectionLimit);
        }
        if requests.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(SemanticInvalidationReceiptErrorV1::NonCanonical);
        }
        Ok(Self { cause, requests })
    }

    pub const fn cause(&self) -> SemanticInvalidationCauseV1 {
        self.cause
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &SemanticRequestRefV1> {
        self.requests.iter()
    }

    pub fn len(&self) -> usize {
        self.requests.len()
    }

    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }

    pub fn into_requests(self) -> Vec<SemanticRequestRefV1> {
        self.requests
    }
}

impl fmt::Debug for SemanticInvalidationReceiptV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticInvalidationReceiptV1")
            .field("cause", &self.cause)
            .field("request_count", &self.requests.len())
            .finish()
    }
}

/// Composition-installed provider provenance. Neither field is learned from a
/// provider response; the restricted token alphabet excludes paths and control
/// syntax from the semantic result domain.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticProviderProvenanceV1 {
    provider: SemanticProviderRefV1,
    profile: String,
    version: String,
}

impl SemanticProviderProvenanceV1 {
    pub fn try_new(
        provider: SemanticProviderRefV1,
        profile: String,
        version: String,
    ) -> Result<Self, SemanticBuildErrorV1> {
        if !valid_provider_token(&profile, MAX_SEMANTIC_PROVIDER_PROFILE_BYTES_V1)
            || !valid_provider_token(&version, MAX_SEMANTIC_PROVIDER_VERSION_BYTES_V1)
        {
            return Err(SemanticBuildErrorV1::TextLimit);
        }
        Ok(Self {
            provider,
            profile,
            version,
        })
    }

    pub const fn provider(&self) -> SemanticProviderRefV1 {
        self.provider
    }

    pub fn profile(&self) -> &str {
        &self.profile
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}

impl fmt::Debug for SemanticProviderProvenanceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticProviderProvenanceV1")
            .field("provider", &self.provider)
            .field("profile_bytes", &self.profile.len())
            .field("version_bytes", &self.version.len())
            .finish()
    }
}

/// Peer-owned index generation. It is not a Vellum source revision.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticIndexGenerationV1(NonZeroU64);

impl SemanticIndexGenerationV1 {
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Debug for SemanticIndexGenerationV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SemanticIndexGenerationV1(<opaque>)")
    }
}

/// A non-empty UTF-8 value bounded at construction.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticTextV1(String);

impl SemanticTextV1 {
    pub fn try_new(value: String, max_bytes: usize) -> Result<Self, SemanticBuildErrorV1> {
        if value.is_empty()
            || max_bytes == 0
            || max_bytes > MAX_SEMANTIC_DETAIL_BYTES_V1
            || value.len() > max_bytes
        {
            return Err(SemanticBuildErrorV1::TextLimit);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for SemanticTextV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SemanticTextV1(<redacted>)")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticBuildErrorV1 {
    InvalidLimits,
    TextLimit,
    InvalidSpan,
    CollectionLimit,
    IncompatibleResult,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticLimitsV1 {
    pub max_query_bytes: usize,
    pub max_label_bytes: usize,
    pub max_detail_bytes: usize,
    pub max_results: usize,
    pub max_rename_edits: usize,
    pub max_requests: u64,
}

impl SemanticLimitsV1 {
    pub const fn reference() -> Self {
        Self {
            max_query_bytes: MAX_SEMANTIC_QUERY_BYTES_V1,
            max_label_bytes: MAX_SEMANTIC_LABEL_BYTES_V1,
            max_detail_bytes: MAX_SEMANTIC_DETAIL_BYTES_V1,
            max_results: MAX_SEMANTIC_RESULTS_V1,
            max_rename_edits: MAX_SEMANTIC_RENAME_EDITS_V1,
            max_requests: MAX_SEMANTIC_REQUESTS_V1,
        }
    }

    pub const fn is_valid(self) -> bool {
        self.max_query_bytes > 0
            && self.max_label_bytes > 0
            && self.max_detail_bytes > 0
            && self.max_results > 0
            && self.max_rename_edits > 0
            && self.max_requests > 0
            && self.max_query_bytes <= MAX_SEMANTIC_QUERY_BYTES_V1
            && self.max_label_bytes <= MAX_SEMANTIC_LABEL_BYTES_V1
            && self.max_detail_bytes <= MAX_SEMANTIC_DETAIL_BYTES_V1
            && self.max_results <= MAX_SEMANTIC_RESULTS_V1
            && self.max_rename_edits <= MAX_SEMANTIC_RENAME_EDITS_V1
            && self.max_requests <= MAX_SEMANTIC_REQUESTS_V1
    }
}

/// Byte offsets into one exact source revision.
///
/// Spans are half-open. Empty spans are valid point locations, including an
/// insertion point or type query position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticSpanV1 {
    pub start: u64,
    pub end: u64,
}

impl SemanticSpanV1 {
    pub const fn try_new(start: u64, end: u64) -> Result<Self, SemanticBuildErrorV1> {
        if start > end {
            return Err(SemanticBuildErrorV1::InvalidSpan);
        }
        Ok(Self { start, end })
    }
}

/// Vellum's logical source revision. Zero is the valid initial revision; this
/// wrapper keeps that domain distinct from the peer's non-zero index generation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticSourceRevisionV1(u64);

impl SemanticSourceRevisionV1 {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Vellum-owned identity and source revision. A peer must never retarget it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticTargetV1 {
    pub document: SemanticDocumentRefV1,
    pub source_revision: SemanticSourceRevisionV1,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum SemanticCapabilityV1 {
    ReadIndex = 0,
    PlanRename = 1,
}

/// Finite capability lattice. Meet attenuates; join is trusted composition.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticCapabilitySetV1(u8);

impl SemanticCapabilitySetV1 {
    const ALL: u8 = 0b11;

    pub const fn bottom() -> Self {
        Self(0)
    }

    pub const fn top() -> Self {
        Self(Self::ALL)
    }

    pub const fn read_only() -> Self {
        Self::singleton(SemanticCapabilityV1::ReadIndex)
    }

    pub const fn singleton(capability: SemanticCapabilityV1) -> Self {
        Self(1 << capability as u8)
    }

    pub const fn meet(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    pub const fn join(self, other: Self) -> Self {
        Self((self.0 | other.0) & Self::ALL)
    }

    pub const fn contains(self, capability: SemanticCapabilityV1) -> bool {
        self.0 & (1 << capability as u8) != 0
    }

    pub const fn is_subset_of(self, other: Self) -> bool {
        self.0 & !other.0 == 0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SemanticQueryKindV1 {
    Symbols,
    Definition,
    References,
    Implementations,
    IncomingCalls,
    OutgoingCalls,
    Diagnostics,
    TypeAtPoint,
    PlanRename,
}

#[derive(Clone, Eq, PartialEq)]
pub enum SemanticQueryV1 {
    Symbols {
        needle: SemanticTextV1,
    },
    Definition {
        offset: u64,
    },
    References {
        offset: u64,
    },
    Implementations {
        offset: u64,
    },
    /// Calls into the exact symbol prepared at this byte offset.
    IncomingCalls {
        offset: u64,
    },
    /// Calls made by the exact symbol prepared at this byte offset.
    OutgoingCalls {
        offset: u64,
    },
    Diagnostics,
    TypeAtPoint {
        offset: u64,
    },
    PlanRename {
        offset: u64,
        new_name: SemanticTextV1,
    },
}

impl SemanticQueryV1 {
    pub const fn kind(&self) -> SemanticQueryKindV1 {
        match self {
            Self::Symbols { .. } => SemanticQueryKindV1::Symbols,
            Self::Definition { .. } => SemanticQueryKindV1::Definition,
            Self::References { .. } => SemanticQueryKindV1::References,
            Self::Implementations { .. } => SemanticQueryKindV1::Implementations,
            Self::IncomingCalls { .. } => SemanticQueryKindV1::IncomingCalls,
            Self::OutgoingCalls { .. } => SemanticQueryKindV1::OutgoingCalls,
            Self::Diagnostics => SemanticQueryKindV1::Diagnostics,
            Self::TypeAtPoint { .. } => SemanticQueryKindV1::TypeAtPoint,
            Self::PlanRename { .. } => SemanticQueryKindV1::PlanRename,
        }
    }

    pub const fn required_capabilities(&self) -> SemanticCapabilitySetV1 {
        match self {
            Self::PlanRename { .. } => SemanticCapabilitySetV1::read_only().join(
                SemanticCapabilitySetV1::singleton(SemanticCapabilityV1::PlanRename),
            ),
            _ => SemanticCapabilitySetV1::read_only(),
        }
    }
}

impl fmt::Debug for SemanticQueryV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticQueryV1")
            .field("kind", &self.kind())
            .finish_non_exhaustive()
    }
}

/// One exact, bounded peer query. `expected_index` is a peer freshness
/// precondition; `target.source_revision` is the Vellum source precondition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticRequestV1 {
    pub request: SemanticRequestRefV1,
    pub target: SemanticTargetV1,
    pub expected_index: Option<SemanticIndexGenerationV1>,
    pub language: SemanticTextV1,
    pub query: SemanticQueryV1,
}

impl SemanticRequestV1 {
    /// Recheck a constructed request against a deployment ceiling narrower
    /// than the reference limits.
    pub fn is_within_limits(&self, limits: SemanticLimitsV1) -> bool {
        limits.is_valid() && request_within_limits(self, limits)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticLocationV1 {
    pub target: SemanticTargetV1,
    pub span: SemanticSpanV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticSymbolKindV1 {
    Module,
    Type,
    Function,
    Method,
    Value,
    Field,
    Unknown,
}

#[derive(Clone, Eq, PartialEq)]
pub struct SemanticSymbolV1 {
    pub name: SemanticTextV1,
    pub kind: SemanticSymbolKindV1,
    pub location: SemanticLocationV1,
}

impl fmt::Debug for SemanticSymbolV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticSymbolV1")
            .field("name", &"<redacted>")
            .field("kind", &self.kind)
            .field("location", &self.location)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticDiagnosticSeverityV1 {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Clone, Eq, PartialEq)]
pub struct SemanticDiagnosticV1 {
    pub severity: SemanticDiagnosticSeverityV1,
    pub location: SemanticLocationV1,
    pub message: SemanticTextV1,
    pub code: Option<SemanticTextV1>,
}

impl fmt::Debug for SemanticDiagnosticV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticDiagnosticV1")
            .field("severity", &self.severity)
            .field("location", &self.location)
            .field("message", &"<redacted>")
            .field("code", &self.code.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct SemanticTextEditV1 {
    pub location: SemanticLocationV1,
    pub replacement: SemanticTextV1,
}

impl fmt::Debug for SemanticTextEditV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticTextEditV1")
            .field("location", &self.location)
            .field("replacement", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq)]
enum SemanticResultKindV1 {
    Symbols(Vec<SemanticSymbolV1>),
    Locations(Vec<SemanticLocationV1>),
    Diagnostics(Vec<SemanticDiagnosticV1>),
    TypeAtPoint(SemanticTextV1),
    /// A proposal only. Applying any edit requires independent Vellum edit
    /// authority, fresh witnesses, and caller-selected atomicity policy.
    RenamePlan(Vec<SemanticTextEditV1>),
}

/// Result whose text and collections can only be created through bounded
/// constructors. Borrowing accessors do not transfer mutation authority.
#[derive(Clone, Eq, PartialEq)]
pub struct SemanticResultV1(SemanticResultKindV1);

impl SemanticResultV1 {
    pub fn try_symbols(
        values: Vec<SemanticSymbolV1>,
        limits: SemanticLimitsV1,
    ) -> Result<Self, SemanticBuildErrorV1> {
        valid_limits(limits)?;
        bounded(values.len(), limits.max_results)?;
        validate_symbol_text(&values, limits)?;
        validate_spans(values.iter().map(|value| value.location.span))?;
        Ok(Self(SemanticResultKindV1::Symbols(values)))
    }

    pub fn try_locations(
        values: Vec<SemanticLocationV1>,
        limits: SemanticLimitsV1,
    ) -> Result<Self, SemanticBuildErrorV1> {
        valid_limits(limits)?;
        bounded(values.len(), limits.max_results)?;
        validate_spans(values.iter().map(|value| value.span))?;
        Ok(Self(SemanticResultKindV1::Locations(values)))
    }

    pub fn try_diagnostics(
        values: Vec<SemanticDiagnosticV1>,
        limits: SemanticLimitsV1,
    ) -> Result<Self, SemanticBuildErrorV1> {
        valid_limits(limits)?;
        bounded(values.len(), limits.max_results)?;
        validate_spans(values.iter().map(|value| value.location.span))?;
        for value in &values {
            if value.message.len() > limits.max_detail_bytes
                || value
                    .code
                    .as_ref()
                    .is_some_and(|code| code.len() > limits.max_label_bytes)
            {
                return Err(SemanticBuildErrorV1::TextLimit);
            }
        }
        Ok(Self(SemanticResultKindV1::Diagnostics(values)))
    }

    pub fn try_type_at_point(
        value: SemanticTextV1,
        limits: SemanticLimitsV1,
    ) -> Result<Self, SemanticBuildErrorV1> {
        valid_limits(limits)?;
        if value.len() > limits.max_detail_bytes {
            return Err(SemanticBuildErrorV1::TextLimit);
        }
        Ok(Self(SemanticResultKindV1::TypeAtPoint(value)))
    }

    pub fn try_rename_plan(
        values: Vec<SemanticTextEditV1>,
        limits: SemanticLimitsV1,
    ) -> Result<Self, SemanticBuildErrorV1> {
        valid_limits(limits)?;
        bounded(values.len(), limits.max_rename_edits)?;
        validate_spans(values.iter().map(|value| value.location.span))?;
        if values
            .iter()
            .any(|value| value.replacement.len() > limits.max_label_bytes)
        {
            return Err(SemanticBuildErrorV1::TextLimit);
        }
        Ok(Self(SemanticResultKindV1::RenamePlan(values)))
    }

    pub const fn item_count(&self) -> usize {
        match &self.0 {
            SemanticResultKindV1::Symbols(values) => values.len(),
            SemanticResultKindV1::Locations(values) => values.len(),
            SemanticResultKindV1::Diagnostics(values) => values.len(),
            SemanticResultKindV1::TypeAtPoint(_) => 1,
            SemanticResultKindV1::RenamePlan(values) => values.len(),
        }
    }

    pub fn symbols(&self) -> Option<&[SemanticSymbolV1]> {
        match &self.0 {
            SemanticResultKindV1::Symbols(values) => Some(values),
            _ => None,
        }
    }

    pub fn locations(&self) -> Option<&[SemanticLocationV1]> {
        match &self.0 {
            SemanticResultKindV1::Locations(values) => Some(values),
            _ => None,
        }
    }

    pub fn diagnostics(&self) -> Option<&[SemanticDiagnosticV1]> {
        match &self.0 {
            SemanticResultKindV1::Diagnostics(values) => Some(values),
            _ => None,
        }
    }

    pub fn type_at_point(&self) -> Option<&SemanticTextV1> {
        match &self.0 {
            SemanticResultKindV1::TypeAtPoint(value) => Some(value),
            _ => None,
        }
    }

    pub fn rename_plan(&self) -> Option<&[SemanticTextEditV1]> {
        match &self.0 {
            SemanticResultKindV1::RenamePlan(values) => Some(values),
            _ => None,
        }
    }

    const fn matches(&self, kind: SemanticQueryKindV1) -> bool {
        matches!(
            (kind, &self.0),
            (
                SemanticQueryKindV1::Symbols,
                SemanticResultKindV1::Symbols(_)
            ) | (
                SemanticQueryKindV1::Definition,
                SemanticResultKindV1::Locations(_)
            ) | (
                SemanticQueryKindV1::References,
                SemanticResultKindV1::Locations(_)
            ) | (
                SemanticQueryKindV1::Implementations,
                SemanticResultKindV1::Locations(_)
            ) | (
                SemanticQueryKindV1::IncomingCalls,
                SemanticResultKindV1::Locations(_)
            ) | (
                SemanticQueryKindV1::OutgoingCalls,
                SemanticResultKindV1::Locations(_)
            ) | (
                SemanticQueryKindV1::Diagnostics,
                SemanticResultKindV1::Diagnostics(_)
            ) | (
                SemanticQueryKindV1::TypeAtPoint,
                SemanticResultKindV1::TypeAtPoint(_)
            ) | (
                SemanticQueryKindV1::PlanRename,
                SemanticResultKindV1::RenamePlan(_)
            )
        )
    }

    /// Whether this result belongs to the closed result family for `kind`.
    /// This proves shape compatibility only, not language-semantic correctness.
    pub const fn matches_query(&self, kind: SemanticQueryKindV1) -> bool {
        self.matches(kind)
    }

    /// Recheck a constructed result against a deployment ceiling narrower than
    /// the reference limits.
    pub fn is_within_limits(&self, limits: SemanticLimitsV1) -> bool {
        limits.is_valid() && result_within_limits(self, limits)
    }
}

impl fmt::Debug for SemanticResultV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticResultV1")
            .field("items", &self.item_count())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct SemanticReplyV1 {
    pub request: SemanticRequestRefV1,
    pub target: SemanticTargetV1,
    pub index_generation: SemanticIndexGenerationV1,
    pub provider: SemanticProviderProvenanceV1,
    pub result: SemanticResultV1,
}

impl fmt::Debug for SemanticReplyV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticReplyV1")
            .field("request", &self.request)
            .field("target", &self.target)
            .field("index_generation", &self.index_generation)
            .field("provider", &self.provider)
            .field("result", &self.result)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticRefusalV1 {
    AuthorityDenied,
    InputLimit,
    OutputLimit,
    InvalidRequest,
    MalformedProviderReply,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticOutcomeV1 {
    Complete(SemanticReplyV1),
    Unsupported(SemanticQueryKindV1),
    /// The peer cannot prove that its index matches both requested freshness
    /// preconditions. No current revision or generation is disclosed.
    Stale,
    Unavailable,
    Refused(SemanticRefusalV1),
}

/// Outbound peer port. Implementations may be in-process or remote but cannot
/// mutate Vellum through this interface.
pub trait SemanticPeerPortV1 {
    fn query(&mut self, request: SemanticRequestV1) -> SemanticOutcomeV1;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SemanticUsageV1 {
    pub requests: u64,
    pub completed: u64,
    pub unsupported: u64,
    pub stale: u64,
    pub unavailable: u64,
    pub refused: u64,
    pub result_items: u64,
    pub saturated: bool,
}

/// Host-side guard and content-safe observer. The trusted composition root
/// supplies an already-authorized capability ceiling; this type does not
/// authenticate a principal or evaluate policy.
pub struct SemanticPeerGateV1<P> {
    port: P,
    provider: SemanticProviderProvenanceV1,
    limits: SemanticLimitsV1,
    capabilities: SemanticCapabilitySetV1,
    remaining_requests: u64,
    usage: SemanticUsageV1,
}

impl<P: SemanticPeerPortV1> SemanticPeerGateV1<P> {
    pub fn try_new(
        port: P,
        provider: SemanticProviderProvenanceV1,
        limits: SemanticLimitsV1,
        capabilities: SemanticCapabilitySetV1,
    ) -> Result<Self, SemanticBuildErrorV1> {
        if !limits.is_valid() {
            return Err(SemanticBuildErrorV1::InvalidLimits);
        }
        Ok(Self {
            port,
            provider,
            remaining_requests: limits.max_requests,
            limits,
            capabilities,
            usage: SemanticUsageV1 {
                requests: 0,
                completed: 0,
                unsupported: 0,
                stale: 0,
                unavailable: 0,
                refused: 0,
                result_items: 0,
                saturated: false,
            },
        })
    }

    pub const fn usage(&self) -> SemanticUsageV1 {
        self.usage
    }

    pub fn attenuate(&mut self, ceiling: SemanticCapabilitySetV1) {
        self.capabilities = self.capabilities.meet(ceiling);
    }

    pub fn query(&mut self, request: SemanticRequestV1) -> SemanticOutcomeV1 {
        if self.remaining_requests == 0 {
            increment(&mut self.usage.unavailable, &mut self.usage.saturated);
            return SemanticOutcomeV1::Unavailable;
        }
        self.remaining_requests -= 1;
        increment(&mut self.usage.requests, &mut self.usage.saturated);

        if !request
            .query
            .required_capabilities()
            .is_subset_of(self.capabilities)
        {
            increment(&mut self.usage.refused, &mut self.usage.saturated);
            return SemanticOutcomeV1::Refused(SemanticRefusalV1::AuthorityDenied);
        }
        if !request_within_limits(&request, self.limits) {
            increment(&mut self.usage.refused, &mut self.usage.saturated);
            return SemanticOutcomeV1::Refused(SemanticRefusalV1::InputLimit);
        }

        let expected = request.clone();
        let outcome = self.port.query(request);
        let outcome = match outcome {
            SemanticOutcomeV1::Complete(reply)
                if reply.request != expected.request
                    || reply.target != expected.target
                    || reply.provider != self.provider
                    || expected
                        .expected_index
                        .is_some_and(|generation| generation != reply.index_generation)
                    || !reply.result.matches(expected.query.kind()) =>
            {
                SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply)
            }
            SemanticOutcomeV1::Complete(reply)
                if !result_within_limits(&reply.result, self.limits) =>
            {
                SemanticOutcomeV1::Refused(SemanticRefusalV1::OutputLimit)
            }
            SemanticOutcomeV1::Unsupported(kind) if kind != expected.query.kind() => {
                SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply)
            }
            other => other,
        };
        self.observe(&outcome);
        outcome
    }

    fn observe(&mut self, outcome: &SemanticOutcomeV1) {
        match outcome {
            SemanticOutcomeV1::Complete(reply) => {
                increment(&mut self.usage.completed, &mut self.usage.saturated);
                add(
                    &mut self.usage.result_items,
                    reply.result.item_count() as u64,
                    &mut self.usage.saturated,
                );
            }
            SemanticOutcomeV1::Unsupported(_) => {
                increment(&mut self.usage.unsupported, &mut self.usage.saturated);
            }
            SemanticOutcomeV1::Stale => {
                increment(&mut self.usage.stale, &mut self.usage.saturated);
            }
            SemanticOutcomeV1::Unavailable => {
                increment(&mut self.usage.unavailable, &mut self.usage.saturated);
            }
            SemanticOutcomeV1::Refused(_) => {
                increment(&mut self.usage.refused, &mut self.usage.saturated);
            }
        }
    }
}

fn bounded(actual: usize, maximum: usize) -> Result<(), SemanticBuildErrorV1> {
    if actual > maximum {
        Err(SemanticBuildErrorV1::CollectionLimit)
    } else {
        Ok(())
    }
}

fn valid_provider_token(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-'))
}

fn valid_limits(limits: SemanticLimitsV1) -> Result<(), SemanticBuildErrorV1> {
    if limits.is_valid() {
        Ok(())
    } else {
        Err(SemanticBuildErrorV1::InvalidLimits)
    }
}

fn validate_symbol_text(
    values: &[SemanticSymbolV1],
    limits: SemanticLimitsV1,
) -> Result<(), SemanticBuildErrorV1> {
    if values
        .iter()
        .any(|value| value.name.len() > limits.max_label_bytes)
    {
        return Err(SemanticBuildErrorV1::TextLimit);
    }
    Ok(())
}

fn validate_spans(spans: impl Iterator<Item = SemanticSpanV1>) -> Result<(), SemanticBuildErrorV1> {
    if spans.into_iter().any(|span| span.start > span.end) {
        Err(SemanticBuildErrorV1::InvalidSpan)
    } else {
        Ok(())
    }
}

fn request_within_limits(request: &SemanticRequestV1, limits: SemanticLimitsV1) -> bool {
    if request.language.len() > limits.max_label_bytes {
        return false;
    }
    match &request.query {
        SemanticQueryV1::Symbols { needle } => needle.len() <= limits.max_query_bytes,
        SemanticQueryV1::PlanRename { new_name, .. } => new_name.len() <= limits.max_label_bytes,
        _ => true,
    }
}

fn result_within_limits(result: &SemanticResultV1, limits: SemanticLimitsV1) -> bool {
    match &result.0 {
        SemanticResultKindV1::Symbols(values) => {
            values.len() <= limits.max_results
                && values
                    .iter()
                    .all(|value| value.name.len() <= limits.max_label_bytes)
        }
        SemanticResultKindV1::Locations(values) => values.len() <= limits.max_results,
        SemanticResultKindV1::Diagnostics(values) => {
            values.len() <= limits.max_results
                && values.iter().all(|value| {
                    value.message.len() <= limits.max_detail_bytes
                        && value
                            .code
                            .as_ref()
                            .is_none_or(|code| code.len() <= limits.max_label_bytes)
                })
        }
        SemanticResultKindV1::TypeAtPoint(value) => value.len() <= limits.max_detail_bytes,
        SemanticResultKindV1::RenamePlan(values) => {
            values.len() <= limits.max_rename_edits
                && values
                    .iter()
                    .all(|value| value.replacement.len() <= limits.max_label_bytes)
        }
    }
}

fn increment(value: &mut u64, saturated: &mut bool) {
    add(value, 1, saturated);
}

fn add(value: &mut u64, amount: u64, saturated: &mut bool) {
    let prior = *value;
    *value = value.saturating_add(amount);
    *saturated |= *value == u64::MAX && prior > u64::MAX.saturating_sub(amount);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedPort(SemanticOutcomeV1);

    impl SemanticPeerPortV1 for FixedPort {
        fn query(&mut self, _request: SemanticRequestV1) -> SemanticOutcomeV1 {
            self.0.clone()
        }
    }

    fn text(value: &str, maximum: usize) -> SemanticTextV1 {
        SemanticTextV1::try_new(value.to_owned(), maximum).unwrap()
    }

    fn provenance() -> SemanticProviderProvenanceV1 {
        SemanticProviderProvenanceV1::try_new(
            SemanticProviderRefV1::new([9; 16]),
            "rust-analyzer".into(),
            "1.0.0".into(),
        )
        .unwrap()
    }

    fn target(tag: u8, source_revision: u64) -> SemanticTargetV1 {
        SemanticTargetV1 {
            document: SemanticDocumentRefV1::new([tag; 16]),
            source_revision: SemanticSourceRevisionV1::new(source_revision),
        }
    }

    fn request(query: SemanticQueryV1) -> SemanticRequestV1 {
        SemanticRequestV1 {
            request: SemanticRequestRefV1::new([3; 16]),
            target: target(5, 8),
            expected_index: SemanticIndexGenerationV1::new(13),
            language: text("rust", 8),
            query,
        }
    }

    fn complete(for_request: &SemanticRequestV1, result: SemanticResultV1) -> SemanticOutcomeV1 {
        SemanticOutcomeV1::Complete(SemanticReplyV1 {
            request: for_request.request,
            target: for_request.target,
            index_generation: for_request.expected_index.unwrap(),
            provider: provenance(),
            result,
        })
    }

    #[test]
    fn invalidation_receipt_accepts_exact_canonical_and_empty_sets() {
        let first = SemanticRequestRefV1::new([1; 16]);
        let second = SemanticRequestRefV1::new([2; 16]);
        let receipt = SemanticInvalidationReceiptV1::try_new(
            SemanticInvalidationCauseV1::SourceChanged,
            vec![first, second],
            2,
        )
        .unwrap();
        assert_eq!(receipt.cause(), SemanticInvalidationCauseV1::SourceChanged);
        assert_eq!(receipt.len(), 2);
        assert!(!receipt.is_empty());
        assert_eq!(
            receipt.iter().copied().collect::<Vec<_>>(),
            vec![first, second]
        );
        assert_eq!(
            format!("{receipt:?}"),
            "SemanticInvalidationReceiptV1 { cause: SourceChanged, request_count: 2 }"
        );
        assert_eq!(receipt.into_requests(), vec![first, second]);

        let empty = SemanticInvalidationReceiptV1::try_new(
            SemanticInvalidationCauseV1::ProviderStateChanged,
            Vec::new(),
            1,
        )
        .unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert_eq!(empty.iter().len(), 0);
    }

    #[test]
    fn invalidation_receipt_rejects_duplicates_unordered_sets_and_excess() {
        let first = SemanticRequestRefV1::new([1; 16]);
        let second = SemanticRequestRefV1::new([2; 16]);
        assert_eq!(
            SemanticInvalidationReceiptV1::try_new(
                SemanticInvalidationCauseV1::SourceClosed,
                vec![first, first],
                2,
            ),
            Err(SemanticInvalidationReceiptErrorV1::NonCanonical)
        );
        assert_eq!(
            SemanticInvalidationReceiptV1::try_new(
                SemanticInvalidationCauseV1::SourceClosed,
                vec![second, first],
                2,
            ),
            Err(SemanticInvalidationReceiptErrorV1::NonCanonical)
        );
        assert_eq!(
            SemanticInvalidationReceiptV1::try_new(
                SemanticInvalidationCauseV1::SourceClosed,
                vec![first, second],
                1,
            ),
            Err(SemanticInvalidationReceiptErrorV1::CollectionLimit)
        );
    }

    #[test]
    fn invalidation_receipt_rejects_zero_and_global_excess_limits() {
        assert_eq!(
            SemanticInvalidationReceiptV1::try_new(
                SemanticInvalidationCauseV1::ProviderStateChanged,
                Vec::new(),
                0,
            ),
            Err(SemanticInvalidationReceiptErrorV1::InvalidLimit)
        );
        assert_eq!(
            SemanticInvalidationReceiptV1::try_new(
                SemanticInvalidationCauseV1::ProviderStateChanged,
                Vec::new(),
                MAX_SEMANTIC_REQUESTS_V1 + 1,
            ),
            Err(SemanticInvalidationReceiptErrorV1::InvalidLimit)
        );
    }

    #[test]
    fn capability_lattice_attenuates_without_accidental_rename_authority() {
        let read = SemanticCapabilitySetV1::read_only();
        let rename = SemanticCapabilitySetV1::singleton(SemanticCapabilityV1::PlanRename);
        assert_eq!(read.meet(rename), SemanticCapabilitySetV1::bottom());
        assert_eq!(read.join(rename), SemanticCapabilitySetV1::top());
        assert!(read.is_subset_of(SemanticCapabilitySetV1::top()));
        assert!(!read.contains(SemanticCapabilityV1::PlanRename));
    }

    #[test]
    fn construction_refuses_over_bound_text_collections_and_spans() {
        assert_eq!(
            SemanticProviderProvenanceV1::try_new(
                SemanticProviderRefV1::new([1; 16]),
                "../workspace/server".into(),
                "1.0.0".into(),
            ),
            Err(SemanticBuildErrorV1::TextLimit)
        );
        assert_eq!(
            SemanticProviderProvenanceV1::try_new(
                SemanticProviderRefV1::new([1; 16]),
                "provider".into(),
                String::new(),
            ),
            Err(SemanticBuildErrorV1::TextLimit)
        );
        let provider = provenance();
        let rendered = format!("{provider:?}");
        assert!(!rendered.contains(provider.profile()));
        assert!(!rendered.contains(provider.version()));
        assert_eq!(
            SemanticTextV1::try_new("private".into(), 3),
            Err(SemanticBuildErrorV1::TextLimit)
        );
        assert_eq!(
            SemanticTextV1::try_new(
                "private".into(),
                MAX_SEMANTIC_DETAIL_BYTES_V1.saturating_add(1)
            ),
            Err(SemanticBuildErrorV1::TextLimit)
        );
        assert_eq!(
            SemanticSpanV1::try_new(9, 2),
            Err(SemanticBuildErrorV1::InvalidSpan)
        );
        assert_eq!(SemanticSpanV1::try_new(2, 2).unwrap().start, 2);
        let inverted = SemanticSpanV1 { start: 2, end: 1 };
        let location = SemanticLocationV1 {
            target: target(1, 1),
            span: inverted,
        };
        assert_eq!(
            SemanticResultV1::try_symbols(
                vec![SemanticSymbolV1 {
                    name: text("symbol", 16),
                    kind: SemanticSymbolKindV1::Function,
                    location,
                }],
                SemanticLimitsV1::reference(),
            ),
            Err(SemanticBuildErrorV1::InvalidSpan)
        );
        assert_eq!(
            SemanticResultV1::try_locations(vec![location], SemanticLimitsV1::reference(),),
            Err(SemanticBuildErrorV1::InvalidSpan)
        );
        assert_eq!(
            SemanticResultV1::try_diagnostics(
                vec![SemanticDiagnosticV1 {
                    severity: SemanticDiagnosticSeverityV1::Error,
                    location,
                    message: text("diagnostic", 32),
                    code: None,
                }],
                SemanticLimitsV1::reference(),
            ),
            Err(SemanticBuildErrorV1::InvalidSpan)
        );
        assert_eq!(
            SemanticResultV1::try_rename_plan(
                vec![SemanticTextEditV1 {
                    location,
                    replacement: text("replacement", 32),
                }],
                SemanticLimitsV1::reference(),
            ),
            Err(SemanticBuildErrorV1::InvalidSpan)
        );
        assert_eq!(SemanticSourceRevisionV1::new(0).get(), 0);
        let limits = SemanticLimitsV1 {
            max_results: 1,
            ..SemanticLimitsV1::reference()
        };
        assert_eq!(
            SemanticResultV1::try_locations(
                vec![
                    SemanticLocationV1 {
                        target: target(1, 1),
                        span: SemanticSpanV1::try_new(0, 1).unwrap(),
                    },
                    SemanticLocationV1 {
                        target: target(1, 1),
                        span: SemanticSpanV1::try_new(1, 2).unwrap(),
                    },
                ],
                limits,
            ),
            Err(SemanticBuildErrorV1::CollectionLimit)
        );
    }

    #[test]
    fn provider_output_is_rechecked_against_the_composition_ceiling() {
        let offered = request(SemanticQueryV1::References { offset: 2 });
        let result = SemanticResultV1::try_locations(
            vec![
                SemanticLocationV1 {
                    target: offered.target,
                    span: SemanticSpanV1::try_new(0, 1).unwrap(),
                },
                SemanticLocationV1 {
                    target: offered.target,
                    span: SemanticSpanV1::try_new(1, 2).unwrap(),
                },
            ],
            SemanticLimitsV1::reference(),
        )
        .unwrap();
        let mut gate = SemanticPeerGateV1::try_new(
            FixedPort(complete(&offered, result)),
            provenance(),
            SemanticLimitsV1 {
                max_results: 1,
                ..SemanticLimitsV1::reference()
            },
            SemanticCapabilitySetV1::read_only(),
        )
        .unwrap();
        assert_eq!(
            gate.query(offered),
            SemanticOutcomeV1::Refused(SemanticRefusalV1::OutputLimit)
        );
        assert_eq!(gate.usage().refused, 1);
    }

    #[test]
    fn exact_request_source_revision_and_index_generation_prevent_transplant() {
        let offered = request(SemanticQueryV1::Definition { offset: 2 });
        let mut transplanted = offered.clone();
        transplanted.target = target(5, 9);
        let outcome = complete(
            &transplanted,
            SemanticResultV1::try_locations(vec![], SemanticLimitsV1::reference()).unwrap(),
        );
        let mut gate = SemanticPeerGateV1::try_new(
            FixedPort(outcome),
            provenance(),
            SemanticLimitsV1::reference(),
            SemanticCapabilitySetV1::read_only(),
        )
        .unwrap();
        assert_eq!(
            gate.query(offered),
            SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply)
        );

        let offered = request(SemanticQueryV1::Definition { offset: 2 });
        let outcome = SemanticOutcomeV1::Complete(SemanticReplyV1 {
            request: offered.request,
            target: offered.target,
            index_generation: offered.expected_index.unwrap(),
            provider: SemanticProviderProvenanceV1::try_new(
                SemanticProviderRefV1::new([8; 16]),
                "other-provider".into(),
                "1.0.0".into(),
            )
            .unwrap(),
            result: SemanticResultV1::try_locations(vec![], SemanticLimitsV1::reference()).unwrap(),
        });
        let mut gate = SemanticPeerGateV1::try_new(
            FixedPort(outcome),
            provenance(),
            SemanticLimitsV1::reference(),
            SemanticCapabilitySetV1::read_only(),
        )
        .unwrap();
        assert_eq!(
            gate.query(offered),
            SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply)
        );

        let offered = request(SemanticQueryV1::Definition { offset: 2 });
        let mut wrong_generation = SemanticIndexGenerationV1::new(21).unwrap();
        assert_ne!(offered.expected_index, Some(wrong_generation));
        let outcome = SemanticOutcomeV1::Complete(SemanticReplyV1 {
            request: offered.request,
            target: offered.target,
            index_generation: wrong_generation,
            provider: provenance(),
            result: SemanticResultV1::try_locations(vec![], SemanticLimitsV1::reference()).unwrap(),
        });
        let mut gate = SemanticPeerGateV1::try_new(
            FixedPort(outcome),
            provenance(),
            SemanticLimitsV1::reference(),
            SemanticCapabilitySetV1::read_only(),
        )
        .unwrap();
        assert_eq!(
            gate.query(offered),
            SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply)
        );
        wrong_generation = SemanticIndexGenerationV1::new(21).unwrap();
        assert_eq!(wrong_generation.get(), 21);
    }

    #[test]
    fn stale_is_closed_and_does_not_disclose_peer_freshness() {
        let offered = request(SemanticQueryV1::Diagnostics);
        let mut gate = SemanticPeerGateV1::try_new(
            FixedPort(SemanticOutcomeV1::Stale),
            provenance(),
            SemanticLimitsV1::reference(),
            SemanticCapabilitySetV1::read_only(),
        )
        .unwrap();
        assert_eq!(gate.query(offered), SemanticOutcomeV1::Stale);
        assert_eq!(gate.usage().stale, 1);
    }

    #[test]
    fn unsupported_outcome_must_name_the_exact_query() {
        let offered = request(SemanticQueryV1::Diagnostics);
        let mut gate = SemanticPeerGateV1::try_new(
            FixedPort(SemanticOutcomeV1::Unsupported(
                SemanticQueryKindV1::Definition,
            )),
            provenance(),
            SemanticLimitsV1::reference(),
            SemanticCapabilitySetV1::read_only(),
        )
        .unwrap();
        assert_eq!(
            gate.query(offered),
            SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply)
        );
    }

    #[test]
    fn rename_is_plan_only_and_requires_distinct_capability() {
        let offered = request(SemanticQueryV1::PlanRename {
            offset: 4,
            new_name: text("renamed", 16),
        });
        let plan = SemanticResultV1::try_rename_plan(
            vec![SemanticTextEditV1 {
                location: SemanticLocationV1 {
                    target: offered.target,
                    span: SemanticSpanV1::try_new(4, 7).unwrap(),
                },
                replacement: text("renamed", 16),
            }],
            SemanticLimitsV1::reference(),
        )
        .unwrap();
        let outcome = complete(&offered, plan);
        let mut gate = SemanticPeerGateV1::try_new(
            FixedPort(outcome),
            provenance(),
            SemanticLimitsV1::reference(),
            SemanticCapabilitySetV1::read_only(),
        )
        .unwrap();
        assert_eq!(
            gate.query(offered),
            SemanticOutcomeV1::Refused(SemanticRefusalV1::AuthorityDenied)
        );
        assert_eq!(gate.usage().requests, 1);
        assert_eq!(gate.usage().refused, 1);
    }

    #[test]
    fn debug_and_usage_are_content_safe() {
        let private = "secret_symbol_and_message";
        let offered = request(SemanticQueryV1::Symbols {
            needle: text(private, 64),
        });
        let symbol = SemanticSymbolV1 {
            name: text(private, 64),
            kind: SemanticSymbolKindV1::Function,
            location: SemanticLocationV1 {
                target: offered.target,
                span: SemanticSpanV1::try_new(0, 2).unwrap(),
            },
        };
        let result =
            SemanticResultV1::try_symbols(vec![symbol], SemanticLimitsV1::reference()).unwrap();
        let outcome = complete(&offered, result);
        assert!(!format!("{offered:?}{outcome:?}").contains(private));

        let mut gate = SemanticPeerGateV1::try_new(
            FixedPort(outcome),
            provenance(),
            SemanticLimitsV1::reference(),
            SemanticCapabilitySetV1::read_only(),
        )
        .unwrap();
        assert!(matches!(
            gate.query(offered),
            SemanticOutcomeV1::Complete(_)
        ));
        assert_eq!(
            gate.usage(),
            SemanticUsageV1 {
                requests: 1,
                completed: 1,
                result_items: 1,
                ..SemanticUsageV1::default()
            }
        );
    }
}
