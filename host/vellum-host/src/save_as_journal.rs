//! Durable ordering for one stable-binding Save As transaction.
//!
//! The journal intent is synchronized before any external write. A binding
//! change is then checkpointed as the exact successor workspace identity, and
//! only a terminal durable+activated receipt may complete the journal. A crash
//! at either intermediate cut therefore leaves an absorbing pending operation;
//! this type has no retry or effect-execution method.

use crate::{
    request_journal::{
        DurableOperationCompletionV1, DurableOperationIntentV1, DurableOperationOwnerV1,
        DurableOperationRecordV1, DurableOperationSortV1, RequestJournalLimitsV1,
        RequestJournalPortV1, RequestJournalRefusalV1, RequestJournalScopeV1,
        decode_durable_operation_envelope_v1,
    },
    save_as::{SaveAsLifecycleV1, SaveAsReceiptV1},
    workspace::{
        WorkspaceIdentityLimitsV1, WorkspaceIdentityMetadataPortV1, WorkspaceIdentityMetadataV1,
        WorkspaceIdentityRecoveryRefusalV1, WorkspaceRootBindingKindV1,
    },
};
use std::path::{Path, PathBuf};
pub use vellum_protocol::DurableOperationRefV1;
use vellum_protocol::{
    DurableSaveAsIntentV1, DurableSaveAsLifecycleV1 as PortableSaveAsLifecycleV1,
    DurableSaveAsReceiptV1,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DurableSaveAsStageV1 {
    IntentDurable,
    BindingCheckpointed,
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DurableSaveAsErrorV1 {
    Capacity,
    InvalidRequest,
    IdentityConflict,
    InvalidTransition,
    ReceiptNotTerminal,
    ReceiptDoesNotCommitBinding,
    Journal(RequestJournalRefusalV1),
    Checkpoint(WorkspaceIdentityRecoveryRefusalV1),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DurableSaveAsRecoveryStateV1 {
    Complete,
    OutcomeUnknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DurableSaveAsRecoveryRefusalV1 {
    OwnerConflict,
    CanonicalConflict,
    IdentityConflict,
    ReceiptConflict,
}

pub enum CanonicalDurableSaveAsOutcomeV1<'a> {
    Pending,
    Complete {
        receipt: &'a SaveAsReceiptV1,
        canonical_reply: &'a [u8],
    },
}

pub struct CanonicalDurableSaveAsV1<'a> {
    pub document: u64,
    pub expected_source_revision: u64,
    pub expected_destination: &'a Path,
    pub canonical_request: &'a [u8],
    pub outcome: CanonicalDurableSaveAsOutcomeV1<'a>,
}

pub enum DecodedCanonicalDurableSaveAsOutcomeV1<'a> {
    Pending,
    Complete(&'a DurableSaveAsReceiptV1),
}

/// Canonically decoded portable Save As facts. Byte-level canonicality and
/// intent/receipt congruence are established by the protocol service before
/// this host-owned identity and checkpoint validation.
pub struct DecodedCanonicalDurableSaveAsV1<'a> {
    pub intent: &'a DurableSaveAsIntentV1,
    pub outcome: DecodedCanonicalDurableSaveAsOutcomeV1<'a>,
}

impl std::fmt::Debug for DecodedCanonicalDurableSaveAsV1<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DecodedCanonicalDurableSaveAsV1")
            .field("intent", self.intent)
            .field("outcome", &"<redacted>")
            .finish()
    }
}

impl std::fmt::Debug for CanonicalDurableSaveAsV1<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CanonicalDurableSaveAsV1")
            .field("document", &"<opaque>")
            .field("expected_source_revision", &"<opaque>")
            .field("expected_destination", &"<redacted>")
            .field("canonical_request", &"<redacted>")
            .field("outcome", &"<redacted>")
            .finish()
    }
}

/// Validates one recovered Save As record without granting effect authority.
/// Pending intent is absorbing outcome-unknown; this function exposes no
/// admission, write, activation, or retry operation.
pub fn validate_recovered_durable_save_as_v1(
    record: &DurableOperationRecordV1,
    owner: DurableOperationOwnerV1,
    canonical: CanonicalDurableSaveAsV1<'_>,
    identity: &WorkspaceIdentityMetadataV1,
) -> Result<DurableSaveAsRecoveryStateV1, DurableSaveAsRecoveryRefusalV1> {
    if record.scope.workspace() != owner.scope.workspace()
        || record.scope.generation() > owner.scope.generation()
        || record.authenticated_subject != owner.authenticated_subject
    {
        return Err(DurableSaveAsRecoveryRefusalV1::OwnerConflict);
    }
    if owner.scope.workspace() != identity.workspace()
        || owner.scope.generation() != identity.generation()
        || canonical.document == 0
    {
        return Err(DurableSaveAsRecoveryRefusalV1::IdentityConflict);
    }
    let Some(current_document) = identity
        .document_bindings()
        .iter()
        .find(|binding| binding.ordinal() == canonical.document)
    else {
        return Err(DurableSaveAsRecoveryRefusalV1::IdentityConflict);
    };
    let binding_checkpoint_matches = current_document.locator() == canonical.expected_destination;
    if canonical.canonical_request.is_empty()
        || record.frozen_request != canonical.canonical_request
    {
        return Err(DurableSaveAsRecoveryRefusalV1::CanonicalConflict);
    }
    let envelope = decode_durable_operation_envelope_v1(
        canonical.canonical_request,
        canonical.canonical_request.len(),
    )
    .map_err(|_| DurableSaveAsRecoveryRefusalV1::CanonicalConflict)?;
    if envelope.sort() != DurableOperationSortV1::SaveAs {
        return Err(DurableSaveAsRecoveryRefusalV1::CanonicalConflict);
    }
    match (record.frozen_reply.as_deref(), canonical.outcome) {
        (None, CanonicalDurableSaveAsOutcomeV1::Pending) => {
            Ok(DurableSaveAsRecoveryStateV1::OutcomeUnknown)
        }
        (
            Some(stored),
            CanonicalDurableSaveAsOutcomeV1::Complete {
                receipt,
                canonical_reply,
            },
        ) if !canonical_reply.is_empty() && stored == canonical_reply => {
            if receipt.lifecycle() != SaveAsLifecycleV1::TerminalReplayable
                || receipt.evidence().source_revision() != canonical.expected_source_revision
                || receipt.changes_stable_binding() != binding_checkpoint_matches
                || (receipt.changes_stable_binding()
                    && record.scope.generation() >= owner.scope.generation())
            {
                Err(DurableSaveAsRecoveryRefusalV1::ReceiptConflict)
            } else {
                Ok(DurableSaveAsRecoveryStateV1::Complete)
            }
        }
        (Some(_), CanonicalDurableSaveAsOutcomeV1::Complete { .. }) => {
            Err(DurableSaveAsRecoveryRefusalV1::CanonicalConflict)
        }
        _ => Err(DurableSaveAsRecoveryRefusalV1::ReceiptConflict),
    }
}

/// Joins canonically decoded protocol facts to authenticated journal ownership
/// and the current durable workspace identity. This function has no admission,
/// write, binding, retry, or cleanup authority.
pub fn validate_decoded_recovered_durable_save_as_v1(
    record: &DurableOperationRecordV1,
    owner: DurableOperationOwnerV1,
    decoded: DecodedCanonicalDurableSaveAsV1<'_>,
    identity: &WorkspaceIdentityMetadataV1,
) -> Result<DurableSaveAsRecoveryStateV1, DurableSaveAsRecoveryRefusalV1> {
    if record.scope.workspace() != owner.scope.workspace()
        || record.scope.generation() > owner.scope.generation()
        || record.authenticated_subject != owner.authenticated_subject
    {
        return Err(DurableSaveAsRecoveryRefusalV1::OwnerConflict);
    }
    let intent = decoded.intent;
    if owner.scope.workspace() != identity.workspace()
        || owner.scope.generation() != identity.generation()
        || intent.document().workspace().get() != identity.workspace()
    {
        return Err(DurableSaveAsRecoveryRefusalV1::IdentityConflict);
    }
    let Some(current_document) = identity
        .document_bindings()
        .iter()
        .find(|binding| binding.ordinal() == intent.document().get())
    else {
        return Err(DurableSaveAsRecoveryRefusalV1::IdentityConflict);
    };
    let binding_checkpoint_matches = current_document.locator() == Path::new(intent.destination());
    match (record.frozen_reply.is_some(), decoded.outcome) {
        (false, DecodedCanonicalDurableSaveAsOutcomeV1::Pending) => {
            Ok(DurableSaveAsRecoveryStateV1::OutcomeUnknown)
        }
        (true, DecodedCanonicalDurableSaveAsOutcomeV1::Complete(receipt)) => {
            if receipt.document() != intent.document()
                || receipt.source_revision() != intent.expected_revision()
                || receipt.lifecycle() != PortableSaveAsLifecycleV1::TerminalReplayable
                || receipt.changes_stable_binding() != binding_checkpoint_matches
                || (receipt.changes_stable_binding()
                    && record.scope.generation() >= owner.scope.generation())
            {
                Err(DurableSaveAsRecoveryRefusalV1::ReceiptConflict)
            } else {
                Ok(DurableSaveAsRecoveryStateV1::Complete)
            }
        }
        _ => Err(DurableSaveAsRecoveryRefusalV1::ReceiptConflict),
    }
}

#[derive(Clone, Eq, PartialEq)]
struct DocumentBindingSnapshot {
    ordinal: u64,
    locator: PathBuf,
}

#[derive(Clone, Eq, PartialEq)]
struct RootBindingSnapshot {
    ordinal: u64,
    kind: WorkspaceRootBindingKindV1,
    locator: PathBuf,
}

/// Affine owner of one durable Save As ordering obligation.
#[must_use = "a durable Save As transaction may own an incomplete journal or checkpoint obligation"]
pub struct DurableSaveAsTransactionV1 {
    scope: RequestJournalScopeV1,
    operation: DurableOperationRefV1,
    authenticated_subject: [u8; 16],
    frozen_request: Vec<u8>,
    document: u64,
    document_issued_through: u64,
    document_reserved_through: u64,
    root_issued_through: u64,
    root_reserved_through: u64,
    documents: Vec<DocumentBindingSnapshot>,
    roots: Vec<RootBindingSnapshot>,
    stage: DurableSaveAsStageV1,
}

impl std::fmt::Debug for DurableSaveAsTransactionV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DurableSaveAsTransactionV1")
            .field("scope", &self.scope)
            .field("operation", &self.operation)
            .field("authenticated_subject", &"<opaque>")
            .field("frozen_request", &"<redacted>")
            .field("document", &"<opaque>")
            .field("stage", &self.stage)
            .finish()
    }
}

impl DurableSaveAsTransactionV1 {
    #[allow(
        clippy::too_many_arguments,
        reason = "the exact durable owner product is intentionally explicit"
    )]
    pub fn begin<J: RequestJournalPortV1 + ?Sized>(
        journal: &mut J,
        journal_directory: &Path,
        journal_limits: RequestJournalLimitsV1,
        scope: RequestJournalScopeV1,
        operation: DurableOperationRefV1,
        authenticated_subject: [u8; 16],
        frozen_request: &[u8],
        document: u64,
        identity: &WorkspaceIdentityMetadataV1,
    ) -> Result<Self, DurableSaveAsErrorV1> {
        let envelope =
            decode_durable_operation_envelope_v1(frozen_request, journal_limits.max_request_bytes)
                .map_err(|_| DurableSaveAsErrorV1::InvalidRequest)?;
        if envelope.sort() != DurableOperationSortV1::SaveAs
            || document == 0
            || scope.workspace() != identity.workspace()
            || scope.generation() != identity.generation()
            || !identity
                .document_bindings()
                .iter()
                .any(|binding| binding.ordinal() == document)
        {
            return Err(DurableSaveAsErrorV1::InvalidRequest);
        }
        let mut request = Vec::new();
        request
            .try_reserve_exact(frozen_request.len())
            .map_err(|_| DurableSaveAsErrorV1::Capacity)?;
        request.extend_from_slice(frozen_request);
        let documents = copy_documents(identity)?;
        let roots = copy_roots(identity)?;
        journal
            .begin_operation(
                journal_directory,
                DurableOperationIntentV1 {
                    scope,
                    operation,
                    authenticated_subject,
                    frozen_request: &request,
                },
                journal_limits,
            )
            .map_err(DurableSaveAsErrorV1::Journal)?;
        Ok(Self {
            scope,
            operation,
            authenticated_subject,
            frozen_request: request,
            document,
            document_issued_through: identity.document_issued_through(),
            document_reserved_through: identity.document_reserved_through(),
            root_issued_through: identity.root_issued_through(),
            root_reserved_through: identity.root_reserved_through(),
            documents,
            roots,
            stage: DurableSaveAsStageV1::IntentDurable,
        })
    }

    pub const fn stage(&self) -> DurableSaveAsStageV1 {
        self.stage
    }

    pub fn checkpoint_binding<M: WorkspaceIdentityMetadataPortV1 + ?Sized>(
        &mut self,
        metadata: &mut M,
        metadata_directory: &Path,
        metadata_limits: WorkspaceIdentityLimitsV1,
        candidate: &WorkspaceIdentityMetadataV1,
    ) -> Result<(), DurableSaveAsErrorV1> {
        if self.stage != DurableSaveAsStageV1::IntentDurable {
            return Err(DurableSaveAsErrorV1::InvalidTransition);
        }
        self.validate_successor(candidate)?;
        metadata
            .store(metadata_directory, candidate, metadata_limits)
            .map_err(DurableSaveAsErrorV1::Checkpoint)?;
        self.stage = DurableSaveAsStageV1::BindingCheckpointed;
        Ok(())
    }

    pub fn complete<J: RequestJournalPortV1 + ?Sized>(
        &mut self,
        journal: &mut J,
        journal_directory: &Path,
        journal_limits: RequestJournalLimitsV1,
        receipt: &SaveAsReceiptV1,
        frozen_reply: &[u8],
    ) -> Result<(), DurableSaveAsErrorV1> {
        if receipt.lifecycle() != SaveAsLifecycleV1::TerminalReplayable {
            return Err(DurableSaveAsErrorV1::ReceiptNotTerminal);
        }
        match (self.stage, receipt.changes_stable_binding()) {
            (DurableSaveAsStageV1::BindingCheckpointed, true)
            | (DurableSaveAsStageV1::IntentDurable, false) => {}
            (DurableSaveAsStageV1::BindingCheckpointed, false) => {
                return Err(DurableSaveAsErrorV1::ReceiptDoesNotCommitBinding);
            }
            (DurableSaveAsStageV1::IntentDurable, true) | (DurableSaveAsStageV1::Complete, _) => {
                return Err(DurableSaveAsErrorV1::InvalidTransition);
            }
        }
        if frozen_reply.is_empty() {
            return Err(DurableSaveAsErrorV1::InvalidRequest);
        }
        journal
            .complete_operation(
                journal_directory,
                DurableOperationCompletionV1 {
                    intent: DurableOperationIntentV1 {
                        scope: self.scope,
                        operation: self.operation,
                        authenticated_subject: self.authenticated_subject,
                        frozen_request: &self.frozen_request,
                    },
                    frozen_reply,
                },
                journal_limits,
            )
            .map_err(DurableSaveAsErrorV1::Journal)?;
        self.stage = DurableSaveAsStageV1::Complete;
        Ok(())
    }

    fn validate_successor(
        &self,
        candidate: &WorkspaceIdentityMetadataV1,
    ) -> Result<(), DurableSaveAsErrorV1> {
        if candidate.workspace() != self.scope.workspace()
            || self.scope.generation().checked_add(1) != Some(candidate.generation())
            || candidate.document_issued_through() != self.document_issued_through
            || candidate.document_reserved_through() != self.document_reserved_through
            || candidate.root_issued_through() != self.root_issued_through
            || candidate.root_reserved_through() != self.root_reserved_through
            || candidate.document_bindings().len() != self.documents.len()
            || candidate.root_bindings().len() != self.roots.len()
        {
            return Err(DurableSaveAsErrorV1::IdentityConflict);
        }
        let mut target_changed = false;
        for original in &self.documents {
            let Some(current) = candidate
                .document_bindings()
                .iter()
                .find(|binding| binding.ordinal() == original.ordinal)
            else {
                return Err(DurableSaveAsErrorV1::IdentityConflict);
            };
            if original.ordinal == self.document {
                if current.locator() == original.locator {
                    return Err(DurableSaveAsErrorV1::IdentityConflict);
                }
                target_changed = true;
            } else if current.locator() != original.locator {
                return Err(DurableSaveAsErrorV1::IdentityConflict);
            }
        }
        for original in &self.roots {
            let Some(current) = candidate
                .root_bindings()
                .iter()
                .find(|binding| binding.ordinal() == original.ordinal)
            else {
                return Err(DurableSaveAsErrorV1::IdentityConflict);
            };
            if current.kind() != original.kind || current.locator() != original.locator {
                return Err(DurableSaveAsErrorV1::IdentityConflict);
            }
        }
        if !target_changed {
            return Err(DurableSaveAsErrorV1::IdentityConflict);
        }
        Ok(())
    }
}

fn copy_path(path: &Path) -> Result<PathBuf, DurableSaveAsErrorV1> {
    let mut copy = PathBuf::new();
    copy.try_reserve(path.as_os_str().len())
        .map_err(|_| DurableSaveAsErrorV1::Capacity)?;
    copy.push(path);
    Ok(copy)
}

fn copy_documents(
    identity: &WorkspaceIdentityMetadataV1,
) -> Result<Vec<DocumentBindingSnapshot>, DurableSaveAsErrorV1> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(identity.document_bindings().len())
        .map_err(|_| DurableSaveAsErrorV1::Capacity)?;
    for binding in identity.document_bindings() {
        output.push(DocumentBindingSnapshot {
            ordinal: binding.ordinal(),
            locator: copy_path(binding.locator())?,
        });
    }
    Ok(output)
}

fn copy_roots(
    identity: &WorkspaceIdentityMetadataV1,
) -> Result<Vec<RootBindingSnapshot>, DurableSaveAsErrorV1> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(identity.root_bindings().len())
        .map_err(|_| DurableSaveAsErrorV1::Capacity)?;
    for binding in identity.root_bindings() {
        output.push(RootBindingSnapshot {
            ordinal: binding.ordinal(),
            kind: binding.kind(),
            locator: copy_path(binding.locator())?,
        });
    }
    Ok(output)
}
