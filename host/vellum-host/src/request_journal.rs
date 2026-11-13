//! Adapter-neutral durable request/reply journal contract.
//!
//! This is an application durability port, not a second service protocol. The
//! owning composition supplies canonical bounded request and reply bytes. A
//! journal has fixed capacity, never evicts implicitly, and distinguishes an
//! intent without a receipt from a complete replay record.

use crate::workspace::WorkspaceIdentityMetadataV1;
use std::{num::NonZeroU64, path::Path};
use vellum_protocol::{
    DurableEffectIntentV1, DurableEffectReceiptV1, DurableExecuteOutcomeV1, DurableOperationRefV1,
    WorkspaceDocumentIdV1, WorkspaceRootIdV1,
};

const DURABLE_OPERATION_ENVELOPE_MAGIC_V1: &[u8; 4] = b"VOP1";
const DURABLE_OPERATION_ENVELOPE_HEADER_V1: usize = 9;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DurableOperationSortV1 {
    Effect = 1,
    SaveAs = 2,
}

impl DurableOperationSortV1 {
    const fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::Effect),
            2 => Some(Self::SaveAs),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DurableOperationEnvelopeRefusalV1 {
    Empty,
    Limit,
    Capacity,
    Malformed,
    UnsupportedSort,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct DurableOperationEnvelopeV1<'a> {
    sort: DurableOperationSortV1,
    payload: &'a [u8],
}

impl std::fmt::Debug for DurableOperationEnvelopeV1<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DurableOperationEnvelopeV1")
            .field("sort", &self.sort)
            .field("payload", &"<redacted>")
            .finish()
    }
}

impl<'a> DurableOperationEnvelopeV1<'a> {
    pub const fn sort(self) -> DurableOperationSortV1 {
        self.sort
    }

    pub const fn payload(self) -> &'a [u8] {
        self.payload
    }
}

/// Adds a stable operation-sort discriminator around opaque canonical bytes.
/// The payload codec remains owned by the corresponding adapter or protocol.
pub fn encode_durable_operation_envelope_v1(
    sort: DurableOperationSortV1,
    payload: &[u8],
    max_encoded_bytes: usize,
) -> Result<Vec<u8>, DurableOperationEnvelopeRefusalV1> {
    if payload.is_empty() {
        return Err(DurableOperationEnvelopeRefusalV1::Empty);
    }
    let payload_len =
        u32::try_from(payload.len()).map_err(|_| DurableOperationEnvelopeRefusalV1::Limit)?;
    let encoded_len = DURABLE_OPERATION_ENVELOPE_HEADER_V1
        .checked_add(payload.len())
        .ok_or(DurableOperationEnvelopeRefusalV1::Limit)?;
    if encoded_len > max_encoded_bytes {
        return Err(DurableOperationEnvelopeRefusalV1::Limit);
    }
    let mut encoded = Vec::new();
    encoded
        .try_reserve_exact(encoded_len)
        .map_err(|_| DurableOperationEnvelopeRefusalV1::Capacity)?;
    encoded.extend_from_slice(DURABLE_OPERATION_ENVELOPE_MAGIC_V1);
    encoded.push(sort as u8);
    encoded.extend_from_slice(&payload_len.to_be_bytes());
    encoded.extend_from_slice(payload);
    Ok(encoded)
}

pub fn decode_durable_operation_envelope_v1(
    encoded: &[u8],
    max_encoded_bytes: usize,
) -> Result<DurableOperationEnvelopeV1<'_>, DurableOperationEnvelopeRefusalV1> {
    if encoded.len() > max_encoded_bytes {
        return Err(DurableOperationEnvelopeRefusalV1::Limit);
    }
    if encoded.len() < DURABLE_OPERATION_ENVELOPE_HEADER_V1
        || &encoded[..4] != DURABLE_OPERATION_ENVELOPE_MAGIC_V1
    {
        return Err(DurableOperationEnvelopeRefusalV1::Malformed);
    }
    let sort = DurableOperationSortV1::from_tag(encoded[4])
        .ok_or(DurableOperationEnvelopeRefusalV1::UnsupportedSort)?;
    let payload_len = u32::from_be_bytes(encoded[5..9].try_into().expect("fixed header")) as usize;
    if payload_len == 0
        || DURABLE_OPERATION_ENVELOPE_HEADER_V1.checked_add(payload_len) != Some(encoded.len())
    {
        return Err(DurableOperationEnvelopeRefusalV1::Malformed);
    }
    Ok(DurableOperationEnvelopeV1 {
        sort,
        payload: &encoded[DURABLE_OPERATION_ENVELOPE_HEADER_V1..],
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestJournalLimitsV1 {
    pub max_entries: usize,
    /// Monotone durable-operation identities which have been retired. These
    /// tombstones are never reused and therefore have a separate lifetime
    /// bound from active receipt capacity.
    pub max_retired_operations: usize,
    pub max_request_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_encoded_bytes: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RequestJournalScopeV1 {
    workspace: NonZeroU64,
    generation: NonZeroU64,
}

impl std::fmt::Debug for RequestJournalScopeV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RequestJournalScopeV1")
            .field("workspace", &"<opaque>")
            .field("generation", &"<opaque>")
            .finish()
    }
}

impl RequestJournalScopeV1 {
    pub fn new(workspace: u64, generation: u64) -> Option<Self> {
        Some(Self {
            workspace: NonZeroU64::new(workspace)?,
            generation: NonZeroU64::new(generation)?,
        })
    }

    pub fn workspace(self) -> u64 {
        self.workspace.get()
    }

    pub fn generation(self) -> u64 {
        self.generation.get()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RequestJournalRecordV1 {
    pub scope: RequestJournalScopeV1,
    pub client: NonZeroU64,
    pub request: NonZeroU64,
    pub authenticated_subject: [u8; 16],
    pub frozen_request: Vec<u8>,
    pub frozen_reply: Vec<u8>,
}

impl std::fmt::Debug for RequestJournalRecordV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RequestJournalRecordV1")
            .field("scope", &self.scope)
            .field("client", &"<opaque>")
            .field("request", &"<opaque>")
            .field("authenticated_subject", &"<opaque>")
            .field("frozen_request_bytes", &self.frozen_request.len())
            .field("frozen_reply_bytes", &self.frozen_reply.len())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestJournalRefusalV1 {
    InvalidLimits,
    Missing,
    Full,
    Conflict,
    Pending,
    Malformed,
    UnsupportedResource,
    Io,
    OutcomeUnknown,
    Limit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequestJournalRecoveryV1 {
    Empty,
    Recovered(Vec<RequestJournalRecordV1>),
    Refused(RequestJournalRefusalV1),
}

/// Separately versioned durable-operation record. Unlike the legacy request
/// record, this identity contains no transport client/request correlation.
#[derive(Clone, PartialEq, Eq)]
pub struct DurableOperationRecordV1 {
    pub scope: RequestJournalScopeV1,
    pub operation: DurableOperationRefV1,
    pub authenticated_subject: [u8; 16],
    pub frozen_request: Vec<u8>,
    /// `None` is a synchronized intent whose outcome is not provably complete.
    pub frozen_reply: Option<Vec<u8>>,
}

impl std::fmt::Debug for DurableOperationRecordV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DurableOperationRecordV1")
            .field("scope", &self.scope)
            .field("operation", &self.operation)
            .field("authenticated_subject", &"<opaque>")
            .field("frozen_request", &"<redacted>")
            .field("frozen_reply", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DurableOperationRecoveryV1 {
    Empty,
    Recovered(Vec<DurableOperationRecordV1>),
    Refused(RequestJournalRefusalV1),
}

/// Absorbing identity evidence for a retired durable operation. The exact
/// canonical request is retained so a delayed changed-input attempt conflicts
/// while an exact delayed attempt can never become a new effect (ABA).
#[derive(Clone, PartialEq, Eq)]
pub struct DurableOperationRetirementRecordV1 {
    pub scope: RequestJournalScopeV1,
    pub operation: DurableOperationRefV1,
    pub authenticated_subject: [u8; 16],
    pub frozen_request: Vec<u8>,
}

impl std::fmt::Debug for DurableOperationRetirementRecordV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DurableOperationRetirementRecordV1")
            .field("scope", &self.scope)
            .field("operation", &self.operation)
            .field("authenticated_subject", &"<opaque>")
            .field("frozen_request", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DurableOperationRetirementRecoveryV1 {
    Empty,
    Recovered(Vec<DurableOperationRetirementRecordV1>),
    Refused(RequestJournalRefusalV1),
}

#[derive(Clone, Copy)]
pub struct DurableOperationRetirementV1<'a> {
    pub scope: RequestJournalScopeV1,
    pub operation: DurableOperationRefV1,
    pub authenticated_subject: [u8; 16],
    pub frozen_request: &'a [u8],
}

impl std::fmt::Debug for DurableOperationRetirementV1<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DurableOperationRetirementV1")
            .field("scope", &self.scope)
            .field("operation", &self.operation)
            .field("authenticated_subject", &"<opaque>")
            .field("frozen_request", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurableOperationRetirementResultV1 {
    Retired,
    AbsentOrForeign,
    RequestConflict,
    Pending,
}

#[derive(Clone, PartialEq, Eq)]
pub enum DurableOperationLookupV1 {
    Complete(Vec<u8>),
    RequestConflict,
    OutcomeUnknown,
    AbsentOrForeign,
    Unavailable,
}

impl std::fmt::Debug for DurableOperationLookupV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Complete(_) => formatter.write_str("Complete(<redacted>)"),
            Self::RequestConflict => formatter.write_str("RequestConflict"),
            Self::OutcomeUnknown => formatter.write_str("OutcomeUnknown"),
            Self::AbsentOrForeign => formatter.write_str("AbsentOrForeign"),
            Self::Unavailable => formatter.write_str("Unavailable"),
        }
    }
}

/// Exact journal owner expected by a trusted recovery composition.
///
/// The authenticated subject is intentionally restart-stable. A live session
/// is not durable ownership evidence and must be re-established separately
/// before disclosure or execution.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct DurableOperationOwnerV1 {
    pub scope: RequestJournalScopeV1,
    pub authenticated_subject: [u8; 16],
}

impl std::fmt::Debug for DurableOperationOwnerV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DurableOperationOwnerV1")
            .field("scope", &self.scope)
            .field("authenticated_subject", &"<opaque>")
            .finish()
    }
}

/// Inhabited canonical outcome supplied by the codec-owning composition.
/// Pending and complete cannot be represented simultaneously, and a complete
/// value always carries the exact canonical bytes to compare with storage.
pub enum CanonicalDurableOutcomeV1<'a> {
    Pending,
    Complete {
        outcome: &'a DurableExecuteOutcomeV1,
        canonical: &'a [u8],
    },
}

impl std::fmt::Debug for CanonicalDurableOutcomeV1<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => formatter.write_str("Pending"),
            Self::Complete { .. } => formatter.write_str("Complete(<redacted>)"),
        }
    }
}

/// Codec-decoded values paired with their exact canonical encodings.
///
/// This host value does not bless arbitrary bytes as canonical. The codec
/// owner must decode and re-encode before constructing it; recovery then
/// compares those bytes with the journal record before consulting semantics.
pub struct CanonicalDurableEffectV1<'a> {
    pub intent: &'a DurableEffectIntentV1,
    pub canonical_intent: &'a [u8],
    pub outcome: CanonicalDurableOutcomeV1<'a>,
}

impl std::fmt::Debug for CanonicalDurableEffectV1<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CanonicalDurableEffectV1")
            .field("intent", &self.intent)
            .field("canonical_intent", &"<redacted>")
            .field("outcome", &self.outcome)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurableEffectRecoveryStateV1 {
    /// A synchronized intent exists, but no canonical outcome is durable.
    OutcomeUnknown,
    Complete,
}

/// Content-free, non-enumerating recovery refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurableEffectRecoveryRefusalV1 {
    OwnerConflict,
    CanonicalConflict,
    IdentityConflict,
    IntentOutcomeConflict,
}

/// Validates one codec-decoded durable record before it enters replay state.
///
/// The function is pure: it cannot execute, retry, retire, disclose, or create
/// an operation. `OutcomeUnknown` is therefore absorbing until an explicit
/// higher-level recovery policy resolves it.
pub fn validate_recovered_durable_effect_v1(
    record: &DurableOperationRecordV1,
    owner: DurableOperationOwnerV1,
    canonical: CanonicalDurableEffectV1<'_>,
    identity: &WorkspaceIdentityMetadataV1,
) -> Result<DurableEffectRecoveryStateV1, DurableEffectRecoveryRefusalV1> {
    if record.scope.workspace() != owner.scope.workspace()
        || record.scope.generation() > owner.scope.generation()
        || record.authenticated_subject != owner.authenticated_subject
    {
        return Err(DurableEffectRecoveryRefusalV1::OwnerConflict);
    }
    if owner.scope.workspace() != identity.workspace()
        || owner.scope.generation() != identity.generation()
    {
        return Err(DurableEffectRecoveryRefusalV1::IdentityConflict);
    }
    if canonical.canonical_intent.is_empty() || record.frozen_request != canonical.canonical_intent
    {
        return Err(DurableEffectRecoveryRefusalV1::CanonicalConflict);
    }
    if !durable_intent_matches_identity(canonical.intent, identity) {
        return Err(DurableEffectRecoveryRefusalV1::IdentityConflict);
    }
    match (record.frozen_reply.as_deref(), canonical.outcome) {
        (None, CanonicalDurableOutcomeV1::Pending) => {
            Ok(DurableEffectRecoveryStateV1::OutcomeUnknown)
        }
        (
            Some(stored),
            CanonicalDurableOutcomeV1::Complete {
                outcome,
                canonical: canonical_outcome,
            },
        ) if !canonical_outcome.is_empty() && stored == canonical_outcome => {
            if durable_outcome_matches_intent(canonical.intent, outcome, identity) {
                Ok(DurableEffectRecoveryStateV1::Complete)
            } else {
                Err(DurableEffectRecoveryRefusalV1::IntentOutcomeConflict)
            }
        }
        (Some(_), CanonicalDurableOutcomeV1::Complete { .. }) => {
            Err(DurableEffectRecoveryRefusalV1::CanonicalConflict)
        }
        _ => Err(DurableEffectRecoveryRefusalV1::IntentOutcomeConflict),
    }
}

fn document_is_issued(
    document: WorkspaceDocumentIdV1,
    identity: &WorkspaceIdentityMetadataV1,
) -> bool {
    document.workspace().get() == identity.workspace()
        && document.get() <= identity.document_issued_through()
}

fn root_is_issued(root: WorkspaceRootIdV1, identity: &WorkspaceIdentityMetadataV1) -> bool {
    root.workspace().get() == identity.workspace() && root.get() <= identity.root_issued_through()
}

fn durable_intent_matches_identity(
    intent: &DurableEffectIntentV1,
    identity: &WorkspaceIdentityMetadataV1,
) -> bool {
    match intent {
        DurableEffectIntentV1::AddFile { .. } | DurableEffectIntentV1::AddDirectory { .. } => true,
        DurableEffectIntentV1::RemoveDocument { document }
        | DurableEffectIntentV1::SaveDocument { document, .. }
        | DurableEffectIntentV1::ReplaceDocumentAndSave { document, .. }
        | DurableEffectIntentV1::ExportDocumentCopyExclusive { document, .. } => {
            document_is_issued(*document, identity)
        }
        DurableEffectIntentV1::RemoveRoot { root } => root_is_issued(*root, identity),
    }
}

fn durable_outcome_matches_intent(
    intent: &DurableEffectIntentV1,
    outcome: &DurableExecuteOutcomeV1,
    identity: &WorkspaceIdentityMetadataV1,
) -> bool {
    let receipt = match outcome {
        DurableExecuteOutcomeV1::Completed(receipt) => receipt,
        DurableExecuteOutcomeV1::Refused(_) => return true,
        DurableExecuteOutcomeV1::Retired => return false,
    };
    match (intent, receipt) {
        (
            DurableEffectIntentV1::ExportDocumentCopyExclusive {
                document,
                expected_revision,
                ..
            },
            DurableEffectReceiptV1::DocumentCopyExportedExclusive(receipt),
        ) => {
            document_is_issued(*document, identity)
                && receipt.document() == *document
                && receipt.source_revision() == *expected_revision
                && vellum_protocol::DurableExportCopyReceiptV1::try_new(
                    receipt.document(),
                    receipt.source_revision(),
                    receipt.persistence(),
                    receipt.failure(),
                    receipt.cleanup_failed(),
                )
                .is_some()
        }
        _ => durable_non_export_outcome_matches_intent(intent, receipt, identity),
    }
}

fn durable_non_export_outcome_matches_intent(
    intent: &DurableEffectIntentV1,
    receipt: &DurableEffectReceiptV1,
    identity: &WorkspaceIdentityMetadataV1,
) -> bool {
    match (intent, receipt) {
        (DurableEffectIntentV1::AddFile { .. }, DurableEffectReceiptV1::FileAdmitted(receipt)) => {
            document_is_issued(receipt.document, identity)
                && match (receipt.status, receipt.root) {
                    (vellum_protocol::FileAdmissionStatusV1::Added, Some(root)) => {
                        root_is_issued(root, identity)
                    }
                    (vellum_protocol::FileAdmissionStatusV1::AlreadyPresent, None) => true,
                    _ => false,
                }
        }
        (
            DurableEffectIntentV1::AddDirectory { .. },
            DurableEffectReceiptV1::DirectoryAdmitted(receipt),
        ) => {
            root_is_issued(receipt.root, identity)
                && receipt
                    .added_documents
                    .iter()
                    .all(|document| document_is_issued(*document, identity))
                && u64::try_from(receipt.added_documents.len()).ok() == Some(receipt.added)
                && receipt
                    .added_documents
                    .iter()
                    .enumerate()
                    .all(|(index, value)| !receipt.added_documents[..index].contains(value))
        }
        (
            DurableEffectIntentV1::RemoveDocument { document },
            DurableEffectReceiptV1::DocumentRemoved(receipt),
        ) => receipt.document == *document,
        (
            DurableEffectIntentV1::RemoveRoot { root },
            DurableEffectReceiptV1::RootRemoved(receipt),
        ) => receipt.root == *root,
        (
            DurableEffectIntentV1::SaveDocument {
                document,
                expected_revision,
            },
            DurableEffectReceiptV1::DocumentSaved(receipt),
        ) => receipt.document == *document && receipt.source_revision == *expected_revision,
        (
            DurableEffectIntentV1::ReplaceDocumentAndSave {
                document,
                expected_revision,
                ..
            },
            DurableEffectReceiptV1::DocumentReplacedAndSaved(receipt),
        ) => {
            receipt.document == *document
                && expected_revision
                    .checked_add(1)
                    .is_some_and(|successor| receipt.source_revision == successor)
        }
        _ => false,
    }
}

/// Pure disclosure projection over already-recovered evidence. This function
/// has no service, authority, editor, filesystem, or journal port and therefore
/// cannot execute or create an intent on any outcome, including absence.
pub fn lookup_recovered_operation_v1(
    records: &[DurableOperationRecordV1],
    scope: RequestJournalScopeV1,
    authenticated_subject: [u8; 16],
    operation: DurableOperationRefV1,
    frozen_request: &[u8],
) -> DurableOperationLookupV1 {
    let mut matching = records.iter().filter(|record| {
        record.scope.workspace() == scope.workspace()
            && record.authenticated_subject == authenticated_subject
            && record.operation == operation
    });
    let Some(record) = matching.next() else {
        return DurableOperationLookupV1::AbsentOrForeign;
    };
    if matching.next().is_some() {
        return DurableOperationLookupV1::Unavailable;
    }
    if record.frozen_request != frozen_request {
        return DurableOperationLookupV1::RequestConflict;
    }
    match &record.frozen_reply {
        Some(reply) if !reply.is_empty() => DurableOperationLookupV1::Complete(reply.clone()),
        Some(_) => DurableOperationLookupV1::Unavailable,
        None => DurableOperationLookupV1::OutcomeUnknown,
    }
}

#[derive(Clone, Copy)]
pub struct DurableOperationIntentV1<'a> {
    pub scope: RequestJournalScopeV1,
    pub operation: DurableOperationRefV1,
    pub authenticated_subject: [u8; 16],
    pub frozen_request: &'a [u8],
}

impl std::fmt::Debug for DurableOperationIntentV1<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DurableOperationIntentV1")
            .field("scope", &self.scope)
            .field("operation", &self.operation)
            .field("authenticated_subject", &"<opaque>")
            .field("frozen_request", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Copy)]
pub struct DurableOperationCompletionV1<'a> {
    pub intent: DurableOperationIntentV1<'a>,
    pub frozen_reply: &'a [u8],
}

impl std::fmt::Debug for DurableOperationCompletionV1<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DurableOperationCompletionV1")
            .field("intent", &self.intent)
            .field("frozen_reply", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Copy)]
pub struct RequestJournalIntentV1<'a> {
    pub scope: RequestJournalScopeV1,
    pub client: NonZeroU64,
    pub request: NonZeroU64,
    pub authenticated_subject: [u8; 16],
    pub frozen_request: &'a [u8],
}

impl std::fmt::Debug for RequestJournalIntentV1<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RequestJournalIntentV1")
            .field("scope", &self.scope)
            .field("client", &"<opaque>")
            .field("request", &"<opaque>")
            .field("authenticated_subject", &"<opaque>")
            .field("frozen_request_bytes", &self.frozen_request.len())
            .finish()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RequestJournalCompletionV1<'a> {
    pub intent: RequestJournalIntentV1<'a>,
    pub frozen_reply: &'a [u8],
}

pub trait RequestJournalPortV1 {
    fn recover(
        &mut self,
        directory: &Path,
        scope: RequestJournalScopeV1,
        limits: RequestJournalLimitsV1,
    ) -> RequestJournalRecoveryV1;

    fn begin(
        &mut self,
        directory: &Path,
        intent: RequestJournalIntentV1<'_>,
        limits: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1>;

    fn complete(
        &mut self,
        directory: &Path,
        completion: RequestJournalCompletionV1<'_>,
        limits: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1>;

    /// Recovers the separately versioned operation-keyed format. The closed
    /// default lets legacy adapters refuse rather than reinterpret V1 request
    /// records as durable-operation records.
    fn recover_operations(
        &mut self,
        _directory: &Path,
        _scope: RequestJournalScopeV1,
        _limits: RequestJournalLimitsV1,
    ) -> DurableOperationRecoveryV1 {
        DurableOperationRecoveryV1::Refused(RequestJournalRefusalV1::UnsupportedResource)
    }

    fn begin_operation(
        &mut self,
        _directory: &Path,
        _intent: DurableOperationIntentV1<'_>,
        _limits: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1> {
        Err(RequestJournalRefusalV1::UnsupportedResource)
    }

    fn complete_operation(
        &mut self,
        _directory: &Path,
        _completion: DurableOperationCompletionV1<'_>,
        _limits: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1> {
        Err(RequestJournalRefusalV1::UnsupportedResource)
    }

    fn recover_operation_retirements(
        &mut self,
        _directory: &Path,
        _scope: RequestJournalScopeV1,
        _limits: RequestJournalLimitsV1,
    ) -> DurableOperationRetirementRecoveryV1 {
        DurableOperationRetirementRecoveryV1::Refused(RequestJournalRefusalV1::UnsupportedResource)
    }

    fn retire_operation(
        &mut self,
        _directory: &Path,
        _retirement: DurableOperationRetirementV1<'_>,
        _limits: RequestJournalLimitsV1,
    ) -> Result<DurableOperationRetirementResultV1, RequestJournalRefusalV1> {
        Err(RequestJournalRefusalV1::UnsupportedResource)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{WorkspaceIdentityAllocationV1, WorkspaceIdentityLimitsV1};
    use vellum_protocol::{
        DurableExportCopyPersistenceV1, DurableExportCopyReceiptV1, WorkspaceIdV1,
    };

    fn scope(workspace: u64) -> RequestJournalScopeV1 {
        RequestJournalScopeV1::new(workspace, 1).unwrap()
    }

    fn record(
        workspace: u64,
        subject: [u8; 16],
        request: &[u8],
        reply: Option<&[u8]>,
    ) -> DurableOperationRecordV1 {
        DurableOperationRecordV1 {
            scope: scope(workspace),
            operation: DurableOperationRefV1::new([7; 16]).unwrap(),
            authenticated_subject: subject,
            frozen_request: request.to_vec(),
            frozen_reply: reply.map(<[u8]>::to_vec),
        }
    }

    fn record_at_generation(
        workspace: u64,
        generation: u64,
        subject: [u8; 16],
        request: &[u8],
        reply: Option<&[u8]>,
    ) -> DurableOperationRecordV1 {
        DurableOperationRecordV1 {
            scope: RequestJournalScopeV1::new(workspace, generation).unwrap(),
            operation: DurableOperationRefV1::new([7; 16]).unwrap(),
            authenticated_subject: subject,
            frozen_request: request.to_vec(),
            frozen_reply: reply.map(<[u8]>::to_vec),
        }
    }

    fn identity(
        workspace: u64,
        generation: u64,
        issued_documents: u64,
    ) -> WorkspaceIdentityMetadataV1 {
        WorkspaceIdentityMetadataV1::try_new(
            workspace,
            generation,
            WorkspaceIdentityAllocationV1::try_new(issued_documents, 8, 2, 8).unwrap(),
            WorkspaceIdentityLimitsV1 {
                max_documents: 8,
                max_roots: 8,
                max_encoded_bytes: 4096,
            },
        )
        .unwrap()
    }

    fn export_intent(workspace: u64, document: u64, revision: u64) -> DurableEffectIntentV1 {
        DurableEffectIntentV1::ExportDocumentCopyExclusive {
            document: WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(workspace).unwrap(), document)
                .unwrap(),
            expected_revision: revision,
            destination: "/private/destination".to_owned(),
        }
    }

    fn export_outcome(workspace: u64, document: u64, revision: u64) -> DurableExecuteOutcomeV1 {
        DurableExecuteOutcomeV1::Completed(DurableEffectReceiptV1::DocumentCopyExportedExclusive(
            DurableExportCopyReceiptV1::try_new(
                WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(workspace).unwrap(), document)
                    .unwrap(),
                revision,
                DurableExportCopyPersistenceV1::Durable,
                None,
                false,
            )
            .unwrap(),
        ))
    }

    fn export_projection<'a>(
        intent: &'a DurableEffectIntentV1,
        outcome: Option<&'a DurableExecuteOutcomeV1>,
    ) -> CanonicalDurableEffectV1<'a> {
        CanonicalDurableEffectV1 {
            intent,
            canonical_intent: b"canonical-export-intent",
            outcome: match outcome {
                Some(outcome) => CanonicalDurableOutcomeV1::Complete {
                    outcome,
                    canonical: b"canonical-export-outcome",
                },
                None => CanonicalDurableOutcomeV1::Pending,
            },
        }
    }

    fn export_record(reply: bool) -> DurableOperationRecordV1 {
        record_at_generation(
            7,
            1,
            [3; 16],
            b"canonical-export-intent",
            reply.then_some(&b"canonical-export-outcome"[..]),
        )
    }

    fn export_owner() -> DurableOperationOwnerV1 {
        DurableOperationOwnerV1 {
            scope: RequestJournalScopeV1::new(7, 2).unwrap(),
            authenticated_subject: [3; 16],
        }
    }

    #[test]
    fn lookup_is_closed_over_complete_conflict_pending_and_nondisclosure() {
        let operation = DurableOperationRefV1::new([7; 16]).unwrap();
        assert_eq!(
            lookup_recovered_operation_v1(
                &[record(1, [1; 16], b"intent", Some(b"receipt"))],
                scope(1),
                [1; 16],
                operation,
                b"intent",
            ),
            DurableOperationLookupV1::Complete(b"receipt".to_vec())
        );
        assert_eq!(
            lookup_recovered_operation_v1(
                &[record(1, [1; 16], b"intent", Some(b"receipt"))],
                scope(1),
                [1; 16],
                operation,
                b"changed",
            ),
            DurableOperationLookupV1::RequestConflict
        );
        assert_eq!(
            lookup_recovered_operation_v1(
                &[record(1, [1; 16], b"intent", None)],
                scope(1),
                [1; 16],
                operation,
                b"intent",
            ),
            DurableOperationLookupV1::OutcomeUnknown
        );
        for records in [
            Vec::new(),
            vec![record(1, [2; 16], b"intent", Some(b"receipt"))],
            vec![record(2, [1; 16], b"intent", Some(b"receipt"))],
        ] {
            assert_eq!(
                lookup_recovered_operation_v1(&records, scope(1), [1; 16], operation, b"intent",),
                DurableOperationLookupV1::AbsentOrForeign
            );
        }
        assert_eq!(
            lookup_recovered_operation_v1(
                &[record_at_generation(
                    1,
                    1,
                    [1; 16],
                    b"intent",
                    Some(b"receipt"),
                )],
                RequestJournalScopeV1::new(1, 2).unwrap(),
                [1; 16],
                operation,
                b"intent",
            ),
            DurableOperationLookupV1::Complete(b"receipt".to_vec()),
            "a monotone membership generation advance must not hide a durable operation"
        );
        assert_eq!(
            lookup_recovered_operation_v1(
                &[
                    record(1, [1; 16], b"intent", Some(b"receipt")),
                    record(1, [1; 16], b"intent", Some(b"receipt")),
                ],
                scope(1),
                [1; 16],
                operation,
                b"intent",
            ),
            DurableOperationLookupV1::Unavailable,
            "a non-functional recovered relation must never become first-match replay"
        );
    }

    #[test]
    fn export_recovery_closes_complete_and_pending_states() {
        let intent = export_intent(7, 2, 11);
        let outcome = export_outcome(7, 2, 11);
        let identity = identity(7, 2, 2);
        assert_eq!(
            validate_recovered_durable_effect_v1(
                &export_record(true),
                export_owner(),
                export_projection(&intent, Some(&outcome)),
                &identity,
            ),
            Ok(DurableEffectRecoveryStateV1::Complete)
        );
        assert_eq!(
            validate_recovered_durable_effect_v1(
                &export_record(false),
                export_owner(),
                export_projection(&intent, None),
                &identity,
            ),
            Ok(DurableEffectRecoveryStateV1::OutcomeUnknown),
            "a crash cut after synchronized intent must not become a retryable absence"
        );
    }

    #[test]
    fn export_recovery_requires_exact_owner_canonical_bytes_and_issued_prefix() {
        let intent = export_intent(7, 2, 11);
        let outcome = export_outcome(7, 2, 11);
        let identity = identity(7, 2, 2);
        let mut foreign_owner = export_owner();
        foreign_owner.authenticated_subject = [4; 16];
        assert_eq!(
            validate_recovered_durable_effect_v1(
                &export_record(true),
                foreign_owner,
                export_projection(&intent, Some(&outcome)),
                &identity,
            ),
            Err(DurableEffectRecoveryRefusalV1::OwnerConflict)
        );
        let stale_owner = DurableOperationOwnerV1 {
            scope: RequestJournalScopeV1::new(7, 1).unwrap(),
            authenticated_subject: [3; 16],
        };
        assert_eq!(
            validate_recovered_durable_effect_v1(
                &export_record(true),
                stale_owner,
                export_projection(&intent, Some(&outcome)),
                &identity,
            ),
            Err(DurableEffectRecoveryRefusalV1::IdentityConflict)
        );

        let mut changed_request = export_record(true);
        changed_request.frozen_request = b"noncanonical".to_vec();
        assert_eq!(
            validate_recovered_durable_effect_v1(
                &changed_request,
                export_owner(),
                export_projection(&intent, Some(&outcome)),
                &identity,
            ),
            Err(DurableEffectRecoveryRefusalV1::CanonicalConflict)
        );

        let unissued = export_intent(7, 3, 11);
        let unissued_outcome = export_outcome(7, 3, 11);
        assert_eq!(
            validate_recovered_durable_effect_v1(
                &export_record(true),
                export_owner(),
                export_projection(&unissued, Some(&unissued_outcome)),
                &identity,
            ),
            Err(DurableEffectRecoveryRefusalV1::IdentityConflict)
        );
    }

    #[test]
    fn export_recovery_refuses_cross_sort_wrong_revision_and_retired_outcome() {
        let intent = export_intent(7, 2, 11);
        let identity = identity(7, 2, 2);
        for outcome in [
            export_outcome(7, 1, 11),
            export_outcome(7, 2, 12),
            DurableExecuteOutcomeV1::Retired,
        ] {
            assert_eq!(
                validate_recovered_durable_effect_v1(
                    &export_record(true),
                    export_owner(),
                    export_projection(&intent, Some(&outcome)),
                    &identity,
                ),
                Err(DurableEffectRecoveryRefusalV1::IntentOutcomeConflict)
            );
        }

        let wrong_sort = DurableExecuteOutcomeV1::Completed(DurableEffectReceiptV1::DocumentSaved(
            vellum_protocol::DurableSaveReceiptV1 {
                document: WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(7).unwrap(), 2).unwrap(),
                source_revision: 11,
                persistence: vellum_protocol::SaveReceiptV1 {
                    application: vellum_protocol::SaveApplicationV1::Written,
                    synchronization: vellum_protocol::SaveSynchronizationV1::Succeeded,
                    no_future_effects: true,
                    quiescent: true,
                    adapter_failure: false,
                    cleanup_failed: false,
                    baseline_published: true,
                    outcome_unknown: false,
                },
            },
        ));
        assert_eq!(
            validate_recovered_durable_effect_v1(
                &export_record(true),
                export_owner(),
                export_projection(&intent, Some(&wrong_sort)),
                &identity,
            ),
            Err(DurableEffectRecoveryRefusalV1::IntentOutcomeConflict)
        );
    }

    #[test]
    fn durable_operation_debug_is_content_free() {
        let request = b"private locator and inserted text";
        let reply = b"private historical receipt";
        let intent = DurableOperationIntentV1 {
            scope: scope(1),
            operation: DurableOperationRefV1::new([9; 16]).unwrap(),
            authenticated_subject: [8; 16],
            frozen_request: request,
        };
        let intent_debug = format!("{intent:?}");
        assert!(!intent_debug.contains(std::str::from_utf8(request).unwrap()));
        assert!(!intent_debug.contains(&format!("{:?}", [8_u8; 16])));
        assert!(!intent_debug.contains("33"));
        assert!(!intent_debug.contains("NonZero(1)"));

        let completion = DurableOperationCompletionV1 {
            intent,
            frozen_reply: reply,
        };
        let completion_debug = format!("{completion:?}");
        assert!(!completion_debug.contains(std::str::from_utf8(reply).unwrap()));
        assert!(!completion_debug.contains("26"));

        let lookup_debug = format!("{:?}", DurableOperationLookupV1::Complete(reply.to_vec()));
        assert!(!lookup_debug.contains(std::str::from_utf8(reply).unwrap()));
        assert!(!lookup_debug.contains("26"));

        let export = export_intent(7, 2, 11);
        let projection = export_projection(&export, None);
        let projection_debug = format!("{projection:?}");
        assert!(!projection_debug.contains("/private/destination"));
        assert!(!projection_debug.contains("canonical-export-intent"));
        assert!(!format!("{:?}", export_owner()).contains(&format!("{:?}", [3_u8; 16])));
    }

    #[test]
    fn durable_operation_envelope_is_bounded_canonical_and_content_safe() {
        let payload = b"private destination and canonical request";
        let encoded =
            encode_durable_operation_envelope_v1(DurableOperationSortV1::SaveAs, payload, 128)
                .unwrap();
        let decoded = decode_durable_operation_envelope_v1(&encoded, 128).unwrap();
        assert_eq!(decoded.sort(), DurableOperationSortV1::SaveAs);
        assert_eq!(decoded.payload(), payload);
        assert!(!format!("{decoded:?}").contains(std::str::from_utf8(payload).unwrap()));
        assert_eq!(
            encode_durable_operation_envelope_v1(
                DurableOperationSortV1::SaveAs,
                payload,
                encoded.len() - 1,
            ),
            Err(DurableOperationEnvelopeRefusalV1::Limit)
        );

        for mutant in [
            Vec::new(),
            b"VOP1\x02\0\0\0\0".to_vec(),
            b"VOP1\x7f\0\0\0\x01x".to_vec(),
            b"VOP1\x02\0\0\0\x02x".to_vec(),
            b"VOP1\x02\0\0\0\x01xy".to_vec(),
        ] {
            assert!(decode_durable_operation_envelope_v1(&mutant, 128).is_err());
        }
    }
}
