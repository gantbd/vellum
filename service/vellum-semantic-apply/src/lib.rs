//! Authorized, stale-safe lowering of semantic rename proposals into one
//! Vellum workspace memory transaction.
//!
//! Semantic replies are advice. This service independently binds them to exact
//! workspace snapshots, rechecks provider/source freshness, asks for edit
//! authority once, and publishes through the existing multi-document engine
//! transaction. Durable storage remains an injected composition concern.
//!
//! The optional `VolatileSemanticPlanRegistryV1` prevents clients from
//! resubmitting provider plan bytes and gives one live service process bounded
//! prepare/pending/complete replay semantics. It is intentionally not a durable
//! journal: process loss erases handles and receipts. The separate
//! `JournaledSemanticPlanRegistryV1` defines a bounded journal/recovery algebra,
//! but this crate ships no disk adapter and does not authorize advertising an
//! MCP `rename_apply` tool.
#![forbid(unsafe_code)]

use std::{collections::BTreeMap, fmt};

use sha2::{Digest as _, Sha256};
use vellum_editor::{Workspace, WorkspaceDocumentEdits, WorkspaceDocumentId, WorkspaceError};
use vellum_engine::source::Replacement;
use vellum_host::{
    authority::{
        AuthorityActionV1, AuthorityContextV1, AuthorityDecisionV1, AuthorityPortV1,
        AuthorityRequestV1, AuthorityResourceRefV1,
    },
    semantic::{SemanticDocumentRefV1, SemanticReplyV1, SemanticTargetV1, SemanticTextEditV1},
};
use vellum_semantic_service::{
    SemanticClientRefV1, SemanticProviderReadinessV1, SemanticProviderStateV1, SemanticSnapshotV1,
};

mod journal;
#[cfg(all(test, target_os = "macos"))]
mod tests;
pub use journal::*;

pub const MAX_APPLY_BINDINGS_V1: usize = 4_096;
pub const MAX_APPLY_DOCUMENTS_V1: usize = 256;
pub const MAX_APPLY_EDITS_V1: usize = 4_096;
pub const MAX_APPLY_PAYLOAD_BYTES_V1: usize = 4 * 1024 * 1024;
pub const MAX_RETAINED_SEMANTIC_PLANS_V1: usize = 1_024;
pub const MAX_RETAINED_SEMANTIC_OPERATIONS_V1: usize = 4_096;
pub const MAX_RETAINED_SEMANTIC_PLAN_EDITS_V1: usize = 65_536;
pub const MAX_RETAINED_SEMANTIC_PLAN_PAYLOAD_BYTES_V1: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticApplyLimitsV1 {
    pub max_bindings: usize,
    pub max_documents: usize,
    pub max_edits: usize,
    pub max_payload_bytes: usize,
}

impl SemanticApplyLimitsV1 {
    pub const fn reference() -> Self {
        Self {
            max_bindings: MAX_APPLY_BINDINGS_V1,
            max_documents: MAX_APPLY_DOCUMENTS_V1,
            max_edits: MAX_APPLY_EDITS_V1,
            max_payload_bytes: MAX_APPLY_PAYLOAD_BYTES_V1,
        }
    }

    pub const fn is_valid(self) -> bool {
        self.max_bindings > 0
            && self.max_bindings <= MAX_APPLY_BINDINGS_V1
            && self.max_documents > 0
            && self.max_documents <= MAX_APPLY_DOCUMENTS_V1
            && self.max_edits > 0
            && self.max_edits <= MAX_APPLY_EDITS_V1
            && self.max_payload_bytes > 0
            && self.max_payload_bytes <= MAX_APPLY_PAYLOAD_BYTES_V1
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticApplyRefusalV1 {
    InvalidLimits,
    AuthorityDenied,
    AuthorityUnavailable,
    AuthorityMalformed,
    NotRenamePlan,
    ProviderStale,
    UnknownBinding,
    ClientIsolation,
    SourceStale,
    MalformedPlan,
    InputLimit,
    Capacity,
    WorkspaceUnavailable,
    UnknownPlan,
    PlanExpired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticPersistenceV1 {
    NotAttempted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticApplyReceiptV1 {
    pub changed_documents: usize,
    pub applied_edits: usize,
    pub persistence: SemanticPersistenceV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticApplyOutcomeV1 {
    Applied(SemanticApplyReceiptV1),
    Unchanged(SemanticApplyReceiptV1),
    Refused(SemanticApplyRefusalV1),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SemanticApplyUsageV1 {
    pub bindings: usize,
    pub bindings_high_water: usize,
    pub bindings_installed: u64,
    pub bindings_rebound: u64,
    pub bindings_unbound: u64,
    pub binding_refusals: u64,
    pub attempts: u64,
    pub applied: u64,
    pub unchanged: u64,
    pub refused: u64,
    pub stale: u64,
    pub authority_refused: u64,
    pub documents_high_water: usize,
    pub edits_high_water: usize,
    pub payload_bytes_high_water: usize,
    pub saturated: bool,
}

macro_rules! opaque_plan_ref {
    ($name:ident) => {
        #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; 16]);

        impl $name {
            pub const fn try_new(value: [u8; 16]) -> Option<Self> {
                let mut index = 0;
                while index < value.len() {
                    if value[index] != 0 {
                        return Some(Self(value));
                    }
                    index += 1;
                }
                None
            }

            /// Canonical correlation bytes for trusted journal adapters.
            /// Possession is not authority and cannot recreate retained state.
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

opaque_plan_ref!(SemanticPlanHandleV1);
opaque_plan_ref!(SemanticApplyOperationRefV1);

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct SemanticPlanDigestV1([u8; 32]);

impl fmt::Debug for SemanticPlanDigestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SemanticPlanDigestV1(<opaque>)")
    }
}

impl SemanticPlanDigestV1 {
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Composition-owned non-reusing issuer. A refusal to issue is explicit
/// capacity/exhaustion evidence and never falls back to a client-selected key.
pub trait SemanticPlanHandleIssuerV1 {
    fn issue(&mut self) -> Option<SemanticPlanHandleV1>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticPlanRegistryLimitsV1 {
    pub max_plans: usize,
    pub max_operations: usize,
    pub max_retained_edits: usize,
    pub max_retained_payload_bytes: usize,
}

impl SemanticPlanRegistryLimitsV1 {
    pub const fn reference() -> Self {
        Self {
            max_plans: MAX_RETAINED_SEMANTIC_PLANS_V1,
            max_operations: MAX_RETAINED_SEMANTIC_OPERATIONS_V1,
            max_retained_edits: MAX_RETAINED_SEMANTIC_PLAN_EDITS_V1,
            max_retained_payload_bytes: MAX_RETAINED_SEMANTIC_PLAN_PAYLOAD_BYTES_V1,
        }
    }

    pub const fn is_valid(self) -> bool {
        self.max_plans > 0
            && self.max_plans <= MAX_RETAINED_SEMANTIC_PLANS_V1
            && self.max_operations > 0
            && self.max_operations <= MAX_RETAINED_SEMANTIC_OPERATIONS_V1
            && self.max_retained_edits > 0
            && self.max_retained_edits <= MAX_RETAINED_SEMANTIC_PLAN_EDITS_V1
            && self.max_retained_payload_bytes > 0
            && self.max_retained_payload_bytes <= MAX_RETAINED_SEMANTIC_PLAN_PAYLOAD_BYTES_V1
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticPlanIssuedV1 {
    pub handle: SemanticPlanHandleV1,
    pub digest: SemanticPlanDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VolatileSemanticPlanExecutionOutcomeV1 {
    Complete(SemanticApplyOutcomeV1),
    OutcomeUnknown,
    Conflict,
    Capacity,
    AbsentOrForeign,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SemanticPlanRegistryUsageV1 {
    pub plans: usize,
    pub plans_high_water: usize,
    pub plans_issued: u64,
    pub plans_expired: u64,
    pub retained_edits: usize,
    pub retained_edits_high_water: usize,
    pub retained_payload_bytes: usize,
    pub retained_payload_bytes_high_water: usize,
    pub operations: usize,
    pub operations_high_water: usize,
    pub operation_replays: u64,
    pub operation_conflicts: u64,
    pub operation_unknown: u64,
    pub saturated: bool,
}

#[derive(Clone)]
struct Binding {
    client: SemanticClientRefV1,
    target: SemanticTargetV1,
    workspace: WorkspaceDocumentId,
}

impl fmt::Debug for Binding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Binding")
            .field("client", &self.client)
            .field("target", &self.target)
            .field("workspace", &self.workspace)
            .finish()
    }
}

pub struct SemanticApplyRequestV1<'a> {
    pub client: SemanticClientRefV1,
    pub authority_context: AuthorityContextV1,
    pub current_provider: &'a SemanticProviderStateV1,
    pub reply: &'a SemanticReplyV1,
}

/// Trusted composition service. The authority interpreter remains privately
/// owned; bindings are correlation/freshness facts rather than grants.
pub struct SemanticApplyServiceV1<A> {
    authority: A,
    authority_resource: AuthorityResourceRefV1,
    bindings: BTreeMap<SemanticDocumentRefV1, Binding>,
    limits: SemanticApplyLimitsV1,
    usage: SemanticApplyUsageV1,
}

impl<A: AuthorityPortV1> SemanticApplyServiceV1<A> {
    pub fn try_new(
        authority: A,
        authority_resource: AuthorityResourceRefV1,
        limits: SemanticApplyLimitsV1,
    ) -> Result<Self, SemanticApplyRefusalV1> {
        if !limits.is_valid() {
            return Err(SemanticApplyRefusalV1::InvalidLimits);
        }
        Ok(Self {
            authority,
            authority_resource,
            bindings: BTreeMap::new(),
            limits,
            usage: SemanticApplyUsageV1::default(),
        })
    }

    pub const fn usage(&self) -> SemanticApplyUsageV1 {
        self.usage
    }

    /// Installs an exact client/semantic/workspace product only when revision
    /// and source bytes match the already-admitted semantic snapshot.
    pub fn bind_snapshot(
        &mut self,
        client: SemanticClientRefV1,
        snapshot: &SemanticSnapshotV1,
        workspace: &mut Workspace,
        document: &WorkspaceDocumentId,
    ) -> Result<(), SemanticApplyRefusalV1> {
        let result = self.bind_snapshot_inner(client, snapshot, workspace, document);
        if result.is_err() {
            inc(&mut self.usage.binding_refusals, &mut self.usage.saturated);
        }
        result
    }

    fn bind_snapshot_inner(
        &mut self,
        client: SemanticClientRefV1,
        snapshot: &SemanticSnapshotV1,
        workspace: &mut Workspace,
        document: &WorkspaceDocumentId,
    ) -> Result<(), SemanticApplyRefusalV1> {
        let captured = workspace
            .document_snapshot(document)
            .map_err(map_workspace_error)?;
        if captured.version.revision != snapshot.target().source_revision.get()
            || captured.text != snapshot.source()
        {
            return Err(SemanticApplyRefusalV1::SourceStale);
        }
        let target = snapshot.target();
        if let Some(existing) = self.bindings.get_mut(&target.document) {
            if existing.client != client {
                return Err(SemanticApplyRefusalV1::ClientIsolation);
            }
            if existing.workspace != *document {
                return Err(SemanticApplyRefusalV1::UnknownBinding);
            }
            let previous_revision = existing.target.source_revision.get();
            let candidate_revision = target.source_revision.get();
            if candidate_revision < previous_revision {
                return Err(SemanticApplyRefusalV1::SourceStale);
            }
            if candidate_revision == previous_revision {
                return if existing.target == target {
                    Ok(())
                } else {
                    Err(SemanticApplyRefusalV1::UnknownBinding)
                };
            }
            existing.target = target;
            inc(&mut self.usage.bindings_rebound, &mut self.usage.saturated);
            return Ok(());
        }
        if self.bindings.len() == self.limits.max_bindings {
            return Err(SemanticApplyRefusalV1::Capacity);
        }
        if self
            .bindings
            .values()
            .any(|binding| binding.workspace == *document)
        {
            return Err(SemanticApplyRefusalV1::MalformedPlan);
        }
        self.bindings.insert(
            target.document,
            Binding {
                client,
                target,
                workspace: document.clone(),
            },
        );
        self.usage.bindings = self.bindings.len();
        self.usage.bindings_high_water = self.usage.bindings_high_water.max(self.bindings.len());
        inc(
            &mut self.usage.bindings_installed,
            &mut self.usage.saturated,
        );
        Ok(())
    }

    /// Retires one exact binding after the owning composition has closed the
    /// matching semantic snapshot or workspace membership. Client, semantic
    /// target and workspace identity are all required; none is authority by
    /// itself and a stale or transplanted close leaves the binding intact.
    pub fn unbind_snapshot(
        &mut self,
        client: SemanticClientRefV1,
        target: SemanticTargetV1,
        document: &WorkspaceDocumentId,
    ) -> Result<(), SemanticApplyRefusalV1> {
        let result = self.unbind_snapshot_inner(client, target, document);
        if result.is_err() {
            inc(&mut self.usage.binding_refusals, &mut self.usage.saturated);
        }
        result
    }

    fn unbind_snapshot_inner(
        &mut self,
        client: SemanticClientRefV1,
        target: SemanticTargetV1,
        document: &WorkspaceDocumentId,
    ) -> Result<(), SemanticApplyRefusalV1> {
        let binding = self
            .bindings
            .get(&target.document)
            .ok_or(SemanticApplyRefusalV1::UnknownBinding)?;
        if binding.client != client {
            return Err(SemanticApplyRefusalV1::ClientIsolation);
        }
        if binding.target != target || binding.workspace != *document {
            return Err(SemanticApplyRefusalV1::UnknownBinding);
        }
        self.bindings.remove(&target.document);
        self.usage.bindings = self.bindings.len();
        inc(&mut self.usage.bindings_unbound, &mut self.usage.saturated);
        Ok(())
    }

    pub fn apply(
        &mut self,
        workspace: &mut Workspace,
        request: SemanticApplyRequestV1<'_>,
    ) -> SemanticApplyOutcomeV1 {
        inc(&mut self.usage.attempts, &mut self.usage.saturated);
        let Some(plan) = request.reply.result.rename_plan() else {
            return self.refusal(SemanticApplyRefusalV1::NotRenamePlan);
        };
        if !provider_matches(request.current_provider, request.reply) {
            inc(&mut self.usage.stale, &mut self.usage.saturated);
            return self.refusal(SemanticApplyRefusalV1::ProviderStale);
        }
        if let Err(refusal) = self.validate_primary(workspace, request.client, request.reply.target)
        {
            return self.refusal(refusal);
        }
        if plan.is_empty() {
            inc(&mut self.usage.unchanged, &mut self.usage.saturated);
            return SemanticApplyOutcomeV1::Unchanged(receipt(0, 0));
        }
        if plan.len() > self.limits.max_edits {
            return self.refusal(SemanticApplyRefusalV1::InputLimit);
        }

        let mut document_counts = BTreeMap::<SemanticDocumentRefV1, usize>::new();
        let mut payload_bytes = 0_usize;
        for edit in plan {
            let Some(binding) = self.bindings.get(&edit.location.target.document) else {
                return self.refusal(SemanticApplyRefusalV1::UnknownBinding);
            };
            if binding.client != request.client {
                return self.refusal(SemanticApplyRefusalV1::ClientIsolation);
            }
            if binding.target != edit.location.target {
                inc(&mut self.usage.stale, &mut self.usage.saturated);
                return self.refusal(SemanticApplyRefusalV1::SourceStale);
            }
            payload_bytes = match payload_bytes.checked_add(edit.replacement.len()) {
                Some(bytes) if bytes <= self.limits.max_payload_bytes => bytes,
                _ => return self.refusal(SemanticApplyRefusalV1::InputLimit),
            };
            *document_counts
                .entry(edit.location.target.document)
                .or_default() += 1;
        }
        if document_counts.len() > self.limits.max_documents {
            return self.refusal(SemanticApplyRefusalV1::InputLimit);
        }

        let authority_request = AuthorityRequestV1 {
            context: request.authority_context,
            resource: self.authority_resource,
            action: AuthorityActionV1::EditSource,
        };
        match self.authority.authorize(authority_request) {
            AuthorityDecisionV1::Granted(evidence) if evidence.request() == authority_request => {}
            AuthorityDecisionV1::Granted(_) => {
                inc(&mut self.usage.authority_refused, &mut self.usage.saturated);
                return self.refusal(SemanticApplyRefusalV1::AuthorityMalformed);
            }
            AuthorityDecisionV1::Denied => {
                inc(&mut self.usage.authority_refused, &mut self.usage.saturated);
                return self.refusal(SemanticApplyRefusalV1::AuthorityDenied);
            }
            AuthorityDecisionV1::Unavailable => {
                inc(&mut self.usage.authority_refused, &mut self.usage.saturated);
                return self.refusal(SemanticApplyRefusalV1::AuthorityUnavailable);
            }
        }

        let writes = match self.lower(workspace, plan) {
            Ok(writes) => writes,
            Err(refusal) => return self.refusal(refusal),
        };
        self.usage.documents_high_water = self.usage.documents_high_water.max(writes.len());
        self.usage.edits_high_water = self.usage.edits_high_water.max(plan.len());
        self.usage.payload_bytes_high_water =
            self.usage.payload_bytes_high_water.max(payload_bytes);
        match workspace.edit_document_batch(&writes) {
            Ok(result) => {
                if result.changed_documents == 0 {
                    inc(&mut self.usage.unchanged, &mut self.usage.saturated);
                    SemanticApplyOutcomeV1::Unchanged(receipt(0, plan.len()))
                } else {
                    inc(&mut self.usage.applied, &mut self.usage.saturated);
                    SemanticApplyOutcomeV1::Applied(receipt(result.changed_documents, plan.len()))
                }
            }
            Err(error) => self.refusal(map_workspace_error(error)),
        }
    }

    fn lower(
        &self,
        workspace: &mut Workspace,
        plan: &[SemanticTextEditV1],
    ) -> Result<Vec<WorkspaceDocumentEdits>, SemanticApplyRefusalV1> {
        let mut by_document = BTreeMap::<SemanticDocumentRefV1, Vec<&SemanticTextEditV1>>::new();
        for edit in plan {
            by_document
                .entry(edit.location.target.document)
                .or_default()
                .push(edit);
        }
        let mut writes = Vec::new();
        writes
            .try_reserve_exact(by_document.len())
            .map_err(|_| SemanticApplyRefusalV1::Capacity)?;
        for (semantic_document, mut edits) in by_document {
            let binding = self
                .bindings
                .get(&semantic_document)
                .ok_or(SemanticApplyRefusalV1::UnknownBinding)?;
            let captured = workspace
                .document_snapshot(&binding.workspace)
                .map_err(map_workspace_error)?;
            if captured.version.revision != binding.target.source_revision.get() {
                return Err(SemanticApplyRefusalV1::SourceStale);
            }
            edits.sort_by_key(|edit| (edit.location.span.start, edit.location.span.end));
            let mut replacements = Vec::new();
            replacements
                .try_reserve_exact(edits.len())
                .map_err(|_| SemanticApplyRefusalV1::Capacity)?;
            let mut previous = None;
            for edit in edits {
                let start = usize::try_from(edit.location.span.start)
                    .map_err(|_| SemanticApplyRefusalV1::MalformedPlan)?;
                let end = usize::try_from(edit.location.span.end)
                    .map_err(|_| SemanticApplyRefusalV1::MalformedPlan)?;
                if start > end
                    || end > captured.text.len()
                    || !captured.text.is_char_boundary(start)
                    || !captured.text.is_char_boundary(end)
                    || previous.is_some_and(|(prior_start, prior_end)| {
                        start == prior_start || start < prior_end
                    })
                {
                    return Err(SemanticApplyRefusalV1::MalformedPlan);
                }
                previous = Some((start, end));
                let mut expected = String::new();
                expected
                    .try_reserve_exact(end - start)
                    .map_err(|_| SemanticApplyRefusalV1::Capacity)?;
                expected.push_str(&captured.text[start..end]);
                let mut inserted = String::new();
                inserted
                    .try_reserve_exact(edit.replacement.len())
                    .map_err(|_| SemanticApplyRefusalV1::Capacity)?;
                inserted.push_str(edit.replacement.as_str());
                replacements.push(Replacement {
                    start,
                    end,
                    expected,
                    inserted,
                });
            }
            writes.push(WorkspaceDocumentEdits {
                document: binding.workspace.clone(),
                expected: captured.version,
                edits: replacements,
            });
        }
        Ok(writes)
    }

    fn validate_primary(
        &self,
        workspace: &mut Workspace,
        client: SemanticClientRefV1,
        target: SemanticTargetV1,
    ) -> Result<(), SemanticApplyRefusalV1> {
        let binding = self
            .bindings
            .get(&target.document)
            .ok_or(SemanticApplyRefusalV1::UnknownBinding)?;
        if binding.client != client {
            return Err(SemanticApplyRefusalV1::ClientIsolation);
        }
        if binding.target != target {
            return Err(SemanticApplyRefusalV1::SourceStale);
        }
        let captured = workspace
            .document_snapshot(&binding.workspace)
            .map_err(map_workspace_error)?;
        if captured.version.revision != target.source_revision.get() {
            return Err(SemanticApplyRefusalV1::SourceStale);
        }
        Ok(())
    }

    fn refusal(&mut self, refusal: SemanticApplyRefusalV1) -> SemanticApplyOutcomeV1 {
        inc(&mut self.usage.refused, &mut self.usage.saturated);
        SemanticApplyOutcomeV1::Refused(refusal)
    }
}

#[derive(Clone)]
struct RetainedPlan {
    client: SemanticClientRefV1,
    authority_context: AuthorityContextV1,
    provider: SemanticProviderStateV1,
    reply: SemanticReplyV1,
    digest: SemanticPlanDigestV1,
    expired: bool,
}

#[derive(Clone)]
enum RetainedOperation {
    Pending {
        client: SemanticClientRefV1,
        handle: SemanticPlanHandleV1,
    },
    Complete {
        client: SemanticClientRefV1,
        handle: SemanticPlanHandleV1,
        outcome: SemanticApplyOutcomeV1,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VolatileSemanticPlanOperationAdmissionV1 {
    Prepared,
    Complete(SemanticApplyOutcomeV1),
    OutcomeUnknown,
    Conflict,
    Capacity,
    AbsentOrForeign,
}

/// Volatile server-owned rename-plan and operation registry.
///
/// The registry deliberately has no restart-durability claim. Use
/// `JournaledSemanticPlanRegistryV1` with an independently qualified durable
/// adapter when restart recovery is required. Within one service lifetime,
/// clients can present only opaque plan/operation references: plan content,
/// provider evidence, authority context and exact replay receipts remain owned
/// by this service.
pub struct VolatileSemanticPlanRegistryV1<A, I> {
    apply: SemanticApplyServiceV1<A>,
    issuer: I,
    plans: BTreeMap<SemanticPlanHandleV1, RetainedPlan>,
    operations: BTreeMap<SemanticApplyOperationRefV1, RetainedOperation>,
    limits: SemanticPlanRegistryLimitsV1,
    usage: SemanticPlanRegistryUsageV1,
}

impl<A: AuthorityPortV1, I: SemanticPlanHandleIssuerV1> VolatileSemanticPlanRegistryV1<A, I> {
    pub fn try_new(
        apply: SemanticApplyServiceV1<A>,
        issuer: I,
        limits: SemanticPlanRegistryLimitsV1,
    ) -> Result<Self, SemanticApplyRefusalV1> {
        if !limits.is_valid() {
            return Err(SemanticApplyRefusalV1::InvalidLimits);
        }
        Ok(Self {
            apply,
            issuer,
            plans: BTreeMap::new(),
            operations: BTreeMap::new(),
            limits,
            usage: SemanticPlanRegistryUsageV1::default(),
        })
    }

    pub const fn usage(&self) -> SemanticPlanRegistryUsageV1 {
        self.usage
    }

    pub const fn apply_usage(&self) -> SemanticApplyUsageV1 {
        self.apply.usage()
    }

    pub fn bind_snapshot(
        &mut self,
        client: SemanticClientRefV1,
        snapshot: &SemanticSnapshotV1,
        workspace: &mut Workspace,
        document: &WorkspaceDocumentId,
    ) -> Result<(), SemanticApplyRefusalV1> {
        self.apply
            .bind_snapshot(client, snapshot, workspace, document)
    }

    pub fn unbind_snapshot(
        &mut self,
        client: SemanticClientRefV1,
        target: SemanticTargetV1,
        document: &WorkspaceDocumentId,
    ) -> Result<(), SemanticApplyRefusalV1> {
        self.apply.unbind_snapshot(client, target, document)
    }

    /// Retains a trusted provider reply; the caller cannot supply a handle.
    pub fn issue_plan(
        &mut self,
        workspace: &mut Workspace,
        client: SemanticClientRefV1,
        authority_context: AuthorityContextV1,
        current_provider: &SemanticProviderStateV1,
        reply: &SemanticReplyV1,
    ) -> Result<SemanticPlanIssuedV1, SemanticApplyRefusalV1> {
        let plan = reply
            .result
            .rename_plan()
            .ok_or(SemanticApplyRefusalV1::NotRenamePlan)?;
        if !provider_matches(current_provider, reply) {
            return Err(SemanticApplyRefusalV1::ProviderStale);
        }
        self.apply
            .validate_primary(workspace, client, reply.target)?;
        if self.plans.len() == self.limits.max_plans {
            return Err(SemanticApplyRefusalV1::Capacity);
        }
        if plan.len() > self.apply.limits.max_edits {
            return Err(SemanticApplyRefusalV1::InputLimit);
        }
        let mut documents = BTreeMap::<SemanticDocumentRefV1, ()>::new();
        let mut payload_bytes = 0_usize;
        for edit in plan {
            let binding = self
                .apply
                .bindings
                .get(&edit.location.target.document)
                .ok_or(SemanticApplyRefusalV1::UnknownBinding)?;
            if binding.client != client {
                return Err(SemanticApplyRefusalV1::ClientIsolation);
            }
            if binding.target != edit.location.target {
                return Err(SemanticApplyRefusalV1::SourceStale);
            }
            documents.insert(edit.location.target.document, ());
            payload_bytes = payload_bytes
                .checked_add(edit.replacement.len())
                .filter(|bytes| *bytes <= self.apply.limits.max_payload_bytes)
                .ok_or(SemanticApplyRefusalV1::InputLimit)?;
        }
        if documents.len() > self.apply.limits.max_documents {
            return Err(SemanticApplyRefusalV1::InputLimit);
        }
        let retained_edits = self
            .usage
            .retained_edits
            .checked_add(plan.len())
            .filter(|count| *count <= self.limits.max_retained_edits)
            .ok_or(SemanticApplyRefusalV1::Capacity)?;
        let retained_payload_bytes = self
            .usage
            .retained_payload_bytes
            .checked_add(payload_bytes)
            .filter(|bytes| *bytes <= self.limits.max_retained_payload_bytes)
            .ok_or(SemanticApplyRefusalV1::Capacity)?;
        // Lowering is effect-free and proves every retained target/range against
        // the exact current workspace snapshots before a handle is published.
        self.apply.lower(workspace, plan)?;
        let handle = self
            .issuer
            .issue()
            .ok_or(SemanticApplyRefusalV1::Capacity)?;
        if self.plans.contains_key(&handle) {
            return Err(SemanticApplyRefusalV1::Capacity);
        }
        let digest = digest_plan(reply);
        self.plans.insert(
            handle,
            RetainedPlan {
                client,
                authority_context,
                provider: current_provider.clone(),
                reply: reply.clone(),
                digest,
                expired: false,
            },
        );
        self.usage.plans = self.plans.len();
        self.usage.plans_high_water = self.usage.plans_high_water.max(self.plans.len());
        self.usage.retained_edits = retained_edits;
        self.usage.retained_edits_high_water =
            self.usage.retained_edits_high_water.max(retained_edits);
        self.usage.retained_payload_bytes = retained_payload_bytes;
        self.usage.retained_payload_bytes_high_water = self
            .usage
            .retained_payload_bytes_high_water
            .max(retained_payload_bytes);
        inc(&mut self.usage.plans_issued, &mut self.usage.saturated);
        Ok(SemanticPlanIssuedV1 { handle, digest })
    }

    /// Composition supplies deadline evidence. No clock is hidden here and an
    /// expired handle remains as a non-reusable tombstone until registry drop.
    pub fn expire_plan(
        &mut self,
        client: SemanticClientRefV1,
        handle: SemanticPlanHandleV1,
    ) -> Result<bool, SemanticApplyRefusalV1> {
        let plan = self
            .plans
            .get_mut(&handle)
            .ok_or(SemanticApplyRefusalV1::UnknownPlan)?;
        if plan.client != client {
            return Err(SemanticApplyRefusalV1::UnknownPlan);
        }
        if plan.expired {
            return Ok(false);
        }
        plan.expired = true;
        inc(&mut self.usage.plans_expired, &mut self.usage.saturated);
        Ok(true)
    }

    /// Reserves an operation before any workspace effect. A retained pending
    /// record is outcome-unknown and fences both exact and changed retries.
    pub fn prepare_operation(
        &mut self,
        client: SemanticClientRefV1,
        operation: SemanticApplyOperationRefV1,
        handle: SemanticPlanHandleV1,
    ) -> VolatileSemanticPlanOperationAdmissionV1 {
        if let Some(retained) = self.operations.get(&operation) {
            return match retained {
                RetainedOperation::Pending {
                    client: owner,
                    handle: retained_handle,
                } if *owner == client && *retained_handle == handle => {
                    inc(&mut self.usage.operation_unknown, &mut self.usage.saturated);
                    VolatileSemanticPlanOperationAdmissionV1::OutcomeUnknown
                }
                RetainedOperation::Complete {
                    client: owner,
                    handle: retained_handle,
                    outcome,
                } if *owner == client && *retained_handle == handle => {
                    inc(&mut self.usage.operation_replays, &mut self.usage.saturated);
                    VolatileSemanticPlanOperationAdmissionV1::Complete(outcome.clone())
                }
                RetainedOperation::Pending { client: owner, .. }
                | RetainedOperation::Complete { client: owner, .. }
                    if *owner != client =>
                {
                    VolatileSemanticPlanOperationAdmissionV1::AbsentOrForeign
                }
                RetainedOperation::Pending { .. } | RetainedOperation::Complete { .. } => {
                    inc(
                        &mut self.usage.operation_conflicts,
                        &mut self.usage.saturated,
                    );
                    VolatileSemanticPlanOperationAdmissionV1::Conflict
                }
            };
        }
        let Some(plan) = self.plans.get(&handle) else {
            return VolatileSemanticPlanOperationAdmissionV1::AbsentOrForeign;
        };
        if plan.client != client {
            return VolatileSemanticPlanOperationAdmissionV1::AbsentOrForeign;
        }
        if self.operations.len() == self.limits.max_operations {
            return VolatileSemanticPlanOperationAdmissionV1::Capacity;
        }
        self.operations
            .insert(operation, RetainedOperation::Pending { client, handle });
        self.usage.operations = self.operations.len();
        self.usage.operations_high_water =
            self.usage.operations_high_water.max(self.operations.len());
        VolatileSemanticPlanOperationAdmissionV1::Prepared
    }

    /// Executes one previously prepared exact operation. If the process is lost
    /// after preparation, replay remains `OutcomeUnknown`; this volatile registry
    /// cannot claim crash recovery.
    pub fn execute_prepared(
        &mut self,
        workspace: &mut Workspace,
        client: SemanticClientRefV1,
        operation: SemanticApplyOperationRefV1,
        handle: SemanticPlanHandleV1,
        current_provider: &SemanticProviderStateV1,
    ) -> VolatileSemanticPlanExecutionOutcomeV1 {
        match self.operations.get(&operation) {
            Some(RetainedOperation::Pending {
                client: owner,
                handle: retained_handle,
            }) if *owner == client && *retained_handle == handle => {}
            Some(RetainedOperation::Complete {
                client: owner,
                handle: retained_handle,
                outcome,
            }) if *owner == client && *retained_handle == handle => {
                return VolatileSemanticPlanExecutionOutcomeV1::Complete(outcome.clone());
            }
            Some(RetainedOperation::Pending { client: owner, .. })
            | Some(RetainedOperation::Complete { client: owner, .. })
                if *owner != client =>
            {
                return VolatileSemanticPlanExecutionOutcomeV1::AbsentOrForeign;
            }
            Some(_) => return VolatileSemanticPlanExecutionOutcomeV1::Conflict,
            None => return VolatileSemanticPlanExecutionOutcomeV1::AbsentOrForeign,
        }
        let Some(plan) = self.plans.get(&handle).cloned() else {
            return VolatileSemanticPlanExecutionOutcomeV1::AbsentOrForeign;
        };
        if plan.client != client {
            return VolatileSemanticPlanExecutionOutcomeV1::AbsentOrForeign;
        }
        let outcome = if plan.expired {
            SemanticApplyOutcomeV1::Refused(SemanticApplyRefusalV1::PlanExpired)
        } else if plan.provider != *current_provider {
            SemanticApplyOutcomeV1::Refused(SemanticApplyRefusalV1::ProviderStale)
        } else {
            self.apply.apply(
                workspace,
                SemanticApplyRequestV1 {
                    client,
                    authority_context: plan.authority_context,
                    current_provider,
                    reply: &plan.reply,
                },
            )
        };
        self.operations.insert(
            operation,
            RetainedOperation::Complete {
                client,
                handle,
                outcome: outcome.clone(),
            },
        );
        VolatileSemanticPlanExecutionOutcomeV1::Complete(outcome)
    }

    pub fn execute(
        &mut self,
        workspace: &mut Workspace,
        client: SemanticClientRefV1,
        operation: SemanticApplyOperationRefV1,
        handle: SemanticPlanHandleV1,
        current_provider: &SemanticProviderStateV1,
    ) -> VolatileSemanticPlanExecutionOutcomeV1 {
        match self.prepare_operation(client, operation, handle) {
            VolatileSemanticPlanOperationAdmissionV1::Prepared => {
                self.execute_prepared(workspace, client, operation, handle, current_provider)
            }
            VolatileSemanticPlanOperationAdmissionV1::Complete(outcome) => {
                VolatileSemanticPlanExecutionOutcomeV1::Complete(outcome)
            }
            VolatileSemanticPlanOperationAdmissionV1::OutcomeUnknown => {
                VolatileSemanticPlanExecutionOutcomeV1::OutcomeUnknown
            }
            VolatileSemanticPlanOperationAdmissionV1::Conflict => {
                VolatileSemanticPlanExecutionOutcomeV1::Conflict
            }
            VolatileSemanticPlanOperationAdmissionV1::Capacity => {
                VolatileSemanticPlanExecutionOutcomeV1::Capacity
            }
            VolatileSemanticPlanOperationAdmissionV1::AbsentOrForeign => {
                VolatileSemanticPlanExecutionOutcomeV1::AbsentOrForeign
            }
        }
    }

    pub fn plan_digest(
        &self,
        client: SemanticClientRefV1,
        handle: SemanticPlanHandleV1,
    ) -> Option<SemanticPlanDigestV1> {
        self.plans
            .get(&handle)
            .filter(|plan| plan.client == client)
            .map(|plan| plan.digest)
    }
}

fn digest_plan(reply: &SemanticReplyV1) -> SemanticPlanDigestV1 {
    let mut digest = Sha256::new();
    digest.update(b"vellum-semantic-rename-plan-v1\0");
    digest.update(reply.request.as_bytes());
    digest.update(reply.target.document.as_bytes());
    digest.update(reply.target.source_revision.get().to_be_bytes());
    digest.update(reply.index_generation.get().to_be_bytes());
    digest.update(reply.provider.provider().as_bytes());
    digest.update((reply.provider.profile().len() as u64).to_be_bytes());
    digest.update(reply.provider.profile().as_bytes());
    digest.update((reply.provider.version().len() as u64).to_be_bytes());
    digest.update(reply.provider.version().as_bytes());
    let plan = reply
        .result
        .rename_plan()
        .expect("digest is called only for a rename plan");
    digest.update((plan.len() as u64).to_be_bytes());
    for edit in plan {
        digest.update(edit.location.target.document.as_bytes());
        digest.update(edit.location.target.source_revision.get().to_be_bytes());
        digest.update(edit.location.span.start.to_be_bytes());
        digest.update(edit.location.span.end.to_be_bytes());
        digest.update((edit.replacement.len() as u64).to_be_bytes());
        digest.update(edit.replacement.as_str().as_bytes());
    }
    SemanticPlanDigestV1(digest.finalize().into())
}

fn provider_matches(state: &SemanticProviderStateV1, reply: &SemanticReplyV1) -> bool {
    matches!(
        state.readiness,
        SemanticProviderReadinessV1::Ready { generation }
            if generation == reply.index_generation && state.provider == reply.provider
    )
}

fn receipt(changed_documents: usize, applied_edits: usize) -> SemanticApplyReceiptV1 {
    SemanticApplyReceiptV1 {
        changed_documents,
        applied_edits,
        persistence: SemanticPersistenceV1::NotAttempted,
    }
}

fn map_workspace_error(error: WorkspaceError) -> SemanticApplyRefusalV1 {
    match error {
        WorkspaceError::Capacity => SemanticApplyRefusalV1::Capacity,
        WorkspaceError::Empty
        | WorkspaceError::InvalidView
        | WorkspaceError::SaveAsInProgress
        | WorkspaceError::Discovery(_)
        | WorkspaceError::Session(_) => SemanticApplyRefusalV1::WorkspaceUnavailable,
    }
}

fn inc(value: &mut u64, saturated: &mut bool) {
    let next = value.saturating_add(1);
    *saturated |= next == u64::MAX;
    *value = next;
}

#[cfg(all(test, target_os = "macos"))]
mod qualification {
    use super::*;
    use crate::tests::{Fixture, JournalFixture, workspace};
    use vellum_adapter_local_fs::{
        LocalAppendJournalNamespaceV1, LocalAppendJournalOpenRefusalV1, LocalAppendJournalV1,
    };
    use vellum_host::{
        authority::{
            AuthorityActionSetV1, AuthorityDecisionEvidenceV1, AuthorityDecisionRefV1,
            AuthoritySubjectRefV1, FixedAuthorityV1,
        },
        semantic::{
            SemanticCapabilitySetV1, SemanticDocumentRefV1, SemanticIndexGenerationV1,
            SemanticLimitsV1, SemanticLocationV1, SemanticProviderProvenanceV1,
            SemanticProviderRefV1, SemanticRequestRefV1, SemanticResultV1,
            SemanticSourceRevisionV1, SemanticSpanV1, SemanticTextV1,
        },
    };
    use vellum_semantic_service::SemanticServiceLimitsV1;

    fn context() -> AuthorityContextV1 {
        AuthorityContextV1 {
            subject: AuthoritySubjectRefV1::new([1; 16]),
            session: None,
        }
    }

    fn resource() -> AuthorityResourceRefV1 {
        AuthorityResourceRefV1::new([2; 16])
    }

    fn authority(max: u64) -> FixedAuthorityV1 {
        FixedAuthorityV1::try_new(
            context(),
            resource(),
            AuthorityActionSetV1::singleton(AuthorityActionV1::EditSource),
            max,
        )
        .unwrap()
    }

    struct TransplantAuthority;

    impl AuthorityPortV1 for TransplantAuthority {
        fn authorize(&mut self, request: AuthorityRequestV1) -> AuthorityDecisionV1 {
            let foreign = AuthorityRequestV1 {
                resource: AuthorityResourceRefV1::new([8; 16]),
                ..request
            };
            AuthorityDecisionV1::Granted(AuthorityDecisionEvidenceV1::new(
                AuthorityDecisionRefV1::new([7; 16]),
                foreign,
            ))
        }
    }

    struct PanicAuthority;

    impl AuthorityPortV1 for PanicAuthority {
        fn authorize(&mut self, _request: AuthorityRequestV1) -> AuthorityDecisionV1 {
            panic!("hostile authority unwind")
        }
    }

    fn provenance() -> SemanticProviderProvenanceV1 {
        SemanticProviderProvenanceV1::try_new(
            SemanticProviderRefV1::new([3; 16]),
            "test-profile".into(),
            "1.0".into(),
        )
        .unwrap()
    }

    fn provider() -> SemanticProviderStateV1 {
        SemanticProviderStateV1 {
            provider: provenance(),
            readiness: SemanticProviderReadinessV1::Ready {
                generation: SemanticIndexGenerationV1::new(1).unwrap(),
            },
            capabilities: SemanticCapabilitySetV1::top(),
        }
    }

    fn target(tag: u8, revision: u64) -> SemanticTargetV1 {
        SemanticTargetV1 {
            document: SemanticDocumentRefV1::new([tag; 16]),
            source_revision: SemanticSourceRevisionV1::new(revision),
        }
    }

    fn snapshot(target: SemanticTargetV1, source: &str) -> SemanticSnapshotV1 {
        SemanticSnapshotV1::try_new(
            target,
            "rust".into(),
            source.into(),
            SemanticServiceLimitsV1::reference(),
        )
        .unwrap()
    }

    fn edit(target: SemanticTargetV1, start: u64, end: u64, inserted: &str) -> SemanticTextEditV1 {
        SemanticTextEditV1 {
            location: SemanticLocationV1 {
                target,
                span: SemanticSpanV1::try_new(start, end).unwrap(),
            },
            replacement: SemanticTextV1::try_new(inserted.into(), 256).unwrap(),
        }
    }

    fn reply(primary: SemanticTargetV1, edits: Vec<SemanticTextEditV1>) -> SemanticReplyV1 {
        SemanticReplyV1 {
            request: SemanticRequestRefV1::new([9; 16]),
            target: primary,
            index_generation: SemanticIndexGenerationV1::new(1).unwrap(),
            provider: provenance(),
            result: SemanticResultV1::try_rename_plan(edits, SemanticLimitsV1::reference())
                .unwrap(),
        }
    }

    #[test]
    fn two_document_plan_publishes_once_and_never_persists() {
        let first = Fixture::new("alpha");
        let second = Fixture::new("beta");
        let mut workspace = workspace(&[&first, &second]);
        let ids = workspace.inventory().unwrap().documents;
        let a = workspace.document_snapshot(&ids[0]).unwrap();
        let b = workspace.document_snapshot(&ids[1]).unwrap();
        let ta = target(1, a.version.revision);
        let tb = target(2, b.version.revision);
        let client = SemanticClientRefV1::new([4; 16]);
        let mut service = SemanticApplyServiceV1::try_new(
            authority(1),
            resource(),
            SemanticApplyLimitsV1::reference(),
        )
        .unwrap();
        service
            .bind_snapshot(client, &snapshot(ta, "alpha"), &mut workspace, &ids[0])
            .unwrap();
        service
            .bind_snapshot(client, &snapshot(tb, "beta"), &mut workspace, &ids[1])
            .unwrap();
        let provider = provider();
        let reply = reply(ta, vec![edit(ta, 0, 5, "one"), edit(tb, 0, 4, "two")]);
        assert_eq!(
            service.apply(
                &mut workspace,
                SemanticApplyRequestV1 {
                    client,
                    authority_context: context(),
                    current_provider: &provider,
                    reply: &reply,
                },
            ),
            SemanticApplyOutcomeV1::Applied(SemanticApplyReceiptV1 {
                changed_documents: 2,
                applied_edits: 2,
                persistence: SemanticPersistenceV1::NotAttempted,
            })
        );
        assert_eq!(workspace.document_snapshot(&ids[0]).unwrap().text, "one");
        assert_eq!(workspace.document_snapshot(&ids[1]).unwrap().text, "two");
        assert_eq!(first.source_on_disk(), "alpha");
        assert_eq!(second.source_on_disk(), "beta");
    }

    #[test]
    fn stale_companion_refuses_the_whole_plan_before_publication() {
        let first = Fixture::new("alpha");
        let second = Fixture::new("beta");
        let mut workspace = workspace(&[&first, &second]);
        let ids = workspace.inventory().unwrap().documents;
        let a = workspace.document_snapshot(&ids[0]).unwrap();
        let b = workspace.document_snapshot(&ids[1]).unwrap();
        let ta = target(1, a.version.revision);
        let tb = target(2, b.version.revision);
        let client = SemanticClientRefV1::new([4; 16]);
        let mut service = SemanticApplyServiceV1::try_new(
            authority(1),
            resource(),
            SemanticApplyLimitsV1::reference(),
        )
        .unwrap();
        service
            .bind_snapshot(client, &snapshot(ta, "alpha"), &mut workspace, &ids[0])
            .unwrap();
        service
            .bind_snapshot(client, &snapshot(tb, "beta"), &mut workspace, &ids[1])
            .unwrap();
        let stale = target(2, b.version.revision + 1);
        let provider = provider();
        let reply = reply(ta, vec![edit(ta, 0, 5, "one"), edit(stale, 0, 4, "two")]);
        assert_eq!(
            service.apply(
                &mut workspace,
                SemanticApplyRequestV1 {
                    client,
                    authority_context: context(),
                    current_provider: &provider,
                    reply: &reply,
                },
            ),
            SemanticApplyOutcomeV1::Refused(SemanticApplyRefusalV1::SourceStale)
        );
        assert_eq!(workspace.document_snapshot(&ids[0]).unwrap().text, "alpha");
        assert_eq!(workspace.document_snapshot(&ids[1]).unwrap().text, "beta");
    }

    #[test]
    fn empty_plan_checks_primary_and_provider_but_consumes_no_authority() {
        let file = Fixture::new("alpha");
        let mut workspace = workspace(&[&file]);
        let id = workspace.inventory().unwrap().documents.remove(0);
        let capture = workspace.document_snapshot(&id).unwrap();
        let target = target(1, capture.version.revision);
        let client = SemanticClientRefV1::new([4; 16]);
        let foreign = SemanticClientRefV1::new([5; 16]);
        let mut service = SemanticApplyServiceV1::try_new(
            authority(1),
            resource(),
            SemanticApplyLimitsV1::reference(),
        )
        .unwrap();
        service
            .bind_snapshot(client, &snapshot(target, "alpha"), &mut workspace, &id)
            .unwrap();
        let provider = provider();
        let empty = reply(target, vec![]);
        assert_eq!(
            service.apply(
                &mut workspace,
                SemanticApplyRequestV1 {
                    client: foreign,
                    authority_context: context(),
                    current_provider: &provider,
                    reply: &empty,
                },
            ),
            SemanticApplyOutcomeV1::Refused(SemanticApplyRefusalV1::ClientIsolation)
        );
        assert_eq!(
            service.apply(
                &mut workspace,
                SemanticApplyRequestV1 {
                    client,
                    authority_context: context(),
                    current_provider: &provider,
                    reply: &empty,
                },
            ),
            SemanticApplyOutcomeV1::Unchanged(receipt(0, 0))
        );
        let changing = reply(target, vec![edit(target, 0, 5, "one")]);
        assert!(matches!(
            service.apply(
                &mut workspace,
                SemanticApplyRequestV1 {
                    client,
                    authority_context: context(),
                    current_provider: &provider,
                    reply: &changing,
                },
            ),
            SemanticApplyOutcomeV1::Applied(_)
        ));
    }

    #[test]
    fn split_scalar_overlap_and_same_start_plans_never_publish() {
        let file = Fixture::new("aéz");
        let mut workspace = workspace(&[&file]);
        let id = workspace.inventory().unwrap().documents.remove(0);
        let capture = workspace.document_snapshot(&id).unwrap();
        let target = target(1, capture.version.revision);
        let client = SemanticClientRefV1::new([4; 16]);
        let mut service = SemanticApplyServiceV1::try_new(
            authority(3),
            resource(),
            SemanticApplyLimitsV1::reference(),
        )
        .unwrap();
        service
            .bind_snapshot(client, &snapshot(target, "aéz"), &mut workspace, &id)
            .unwrap();
        let provider = provider();
        for plan in [
            vec![edit(target, 2, 3, "x")],
            vec![edit(target, 0, 2, "x"), edit(target, 1, 3, "y")],
            vec![edit(target, 0, 0, "x"), edit(target, 0, 1, "y")],
        ] {
            let reply = reply(target, plan);
            assert_eq!(
                service.apply(
                    &mut workspace,
                    SemanticApplyRequestV1 {
                        client,
                        authority_context: context(),
                        current_provider: &provider,
                        reply: &reply,
                    },
                ),
                SemanticApplyOutcomeV1::Refused(SemanticApplyRefusalV1::MalformedPlan)
            );
            assert_eq!(workspace.document_snapshot(&id).unwrap().text, "aéz");
        }
    }

    #[test]
    fn stale_provider_and_transplanted_authority_are_closed_prepublication() {
        let file = Fixture::new("alpha");
        let mut workspace = workspace(&[&file]);
        let id = workspace.inventory().unwrap().documents.remove(0);
        let capture = workspace.document_snapshot(&id).unwrap();
        let target = target(1, capture.version.revision);
        let client = SemanticClientRefV1::new([4; 16]);
        let provider = provider();
        let reply = reply(target, vec![edit(target, 0, 5, "one")]);

        let mut stale_service = SemanticApplyServiceV1::try_new(
            authority(1),
            resource(),
            SemanticApplyLimitsV1::reference(),
        )
        .unwrap();
        stale_service
            .bind_snapshot(client, &snapshot(target, "alpha"), &mut workspace, &id)
            .unwrap();
        let stale_provider = SemanticProviderStateV1 {
            readiness: SemanticProviderReadinessV1::Ready {
                generation: SemanticIndexGenerationV1::new(2).unwrap(),
            },
            ..provider.clone()
        };
        assert_eq!(
            stale_service.apply(
                &mut workspace,
                SemanticApplyRequestV1 {
                    client,
                    authority_context: context(),
                    current_provider: &stale_provider,
                    reply: &reply,
                },
            ),
            SemanticApplyOutcomeV1::Refused(SemanticApplyRefusalV1::ProviderStale)
        );

        let mut transplant_service = SemanticApplyServiceV1::try_new(
            TransplantAuthority,
            resource(),
            SemanticApplyLimitsV1::reference(),
        )
        .unwrap();
        transplant_service
            .bind_snapshot(client, &snapshot(target, "alpha"), &mut workspace, &id)
            .unwrap();
        assert_eq!(
            transplant_service.apply(
                &mut workspace,
                SemanticApplyRequestV1 {
                    client,
                    authority_context: context(),
                    current_provider: &provider,
                    reply: &reply,
                },
            ),
            SemanticApplyOutcomeV1::Refused(SemanticApplyRefusalV1::AuthorityMalformed)
        );
        assert_eq!(workspace.document_snapshot(&id).unwrap().text, "alpha");
        assert_eq!(file.source_on_disk(), "alpha");
    }

    #[test]
    fn exact_resynchronization_rebinds_monotonically_for_a_second_rename() {
        let file = Fixture::new("alpha");
        let mut workspace = workspace(&[&file]);
        let id = workspace.inventory().unwrap().documents.remove(0);
        let first = workspace.document_snapshot(&id).unwrap();
        let first_target = target(1, first.version.revision);
        let client = SemanticClientRefV1::new([4; 16]);
        let mut service = SemanticApplyServiceV1::try_new(
            authority(2),
            resource(),
            SemanticApplyLimitsV1::reference(),
        )
        .unwrap();
        service
            .bind_snapshot(
                client,
                &snapshot(first_target, "alpha"),
                &mut workspace,
                &id,
            )
            .unwrap();
        let provider = provider();
        let first_reply = reply(first_target, vec![edit(first_target, 0, 5, "one")]);
        assert!(matches!(
            service.apply(
                &mut workspace,
                SemanticApplyRequestV1 {
                    client,
                    authority_context: context(),
                    current_provider: &provider,
                    reply: &first_reply,
                },
            ),
            SemanticApplyOutcomeV1::Applied(_)
        ));

        let second = workspace.document_snapshot(&id).unwrap();
        let second_target = target(1, second.version.revision);
        assert!(second_target.source_revision.get() > first_target.source_revision.get());
        service
            .bind_snapshot(client, &snapshot(second_target, "one"), &mut workspace, &id)
            .unwrap();
        let second_reply = reply(second_target, vec![edit(second_target, 0, 3, "two")]);
        assert!(matches!(
            service.apply(
                &mut workspace,
                SemanticApplyRequestV1 {
                    client,
                    authority_context: context(),
                    current_provider: &provider,
                    reply: &second_reply,
                },
            ),
            SemanticApplyOutcomeV1::Applied(_)
        ));
        assert_eq!(workspace.document_snapshot(&id).unwrap().text, "two");
        assert_eq!(file.source_on_disk(), "alpha");
        assert_eq!(service.usage().bindings, 1);
        assert_eq!(service.usage().bindings_installed, 1);
        assert_eq!(service.usage().bindings_rebound, 1);
    }

    #[test]
    fn failed_rebind_preserves_the_exact_old_binding() {
        let file = Fixture::new("alpha");
        let mut workspace = workspace(&[&file]);
        let id = workspace.inventory().unwrap().documents.remove(0);
        let capture = workspace.document_snapshot(&id).unwrap();
        let current = target(1, capture.version.revision);
        let client = SemanticClientRefV1::new([4; 16]);
        let mut service = SemanticApplyServiceV1::try_new(
            authority(1),
            resource(),
            SemanticApplyLimitsV1::reference(),
        )
        .unwrap();
        service
            .bind_snapshot(client, &snapshot(current, "alpha"), &mut workspace, &id)
            .unwrap();
        let future = target(1, capture.version.revision + 1);
        assert_eq!(
            service.bind_snapshot(client, &snapshot(future, "alpha"), &mut workspace, &id),
            Err(SemanticApplyRefusalV1::SourceStale)
        );
        let provider = provider();
        let empty = reply(current, vec![]);
        assert_eq!(
            service.apply(
                &mut workspace,
                SemanticApplyRequestV1 {
                    client,
                    authority_context: context(),
                    current_provider: &provider,
                    reply: &empty,
                },
            ),
            SemanticApplyOutcomeV1::Unchanged(receipt(0, 0))
        );
        assert_eq!(service.usage().bindings_rebound, 0);
        assert_eq!(service.usage().binding_refusals, 1);
    }

    #[test]
    fn exact_unbind_releases_capacity_but_foreign_or_stale_close_does_not() {
        let first = Fixture::new("alpha");
        let second = Fixture::new("beta");
        let mut workspace = workspace(&[&first, &second]);
        let ids = workspace.inventory().unwrap().documents;
        let a = workspace.document_snapshot(&ids[0]).unwrap();
        let b = workspace.document_snapshot(&ids[1]).unwrap();
        let ta = target(1, a.version.revision);
        let tb = target(2, b.version.revision);
        let client = SemanticClientRefV1::new([4; 16]);
        let foreign = SemanticClientRefV1::new([5; 16]);
        let mut limits = SemanticApplyLimitsV1::reference();
        limits.max_bindings = 1;
        let mut service =
            SemanticApplyServiceV1::try_new(authority(1), resource(), limits).unwrap();
        service
            .bind_snapshot(client, &snapshot(ta, "alpha"), &mut workspace, &ids[0])
            .unwrap();
        assert_eq!(
            service.unbind_snapshot(foreign, ta, &ids[0]),
            Err(SemanticApplyRefusalV1::ClientIsolation)
        );
        assert_eq!(
            service.unbind_snapshot(client, target(1, ta.source_revision.get() + 1), &ids[0]),
            Err(SemanticApplyRefusalV1::UnknownBinding)
        );
        assert_eq!(
            service.bind_snapshot(client, &snapshot(tb, "beta"), &mut workspace, &ids[1]),
            Err(SemanticApplyRefusalV1::Capacity)
        );
        service.unbind_snapshot(client, ta, &ids[0]).unwrap();
        service
            .bind_snapshot(client, &snapshot(tb, "beta"), &mut workspace, &ids[1])
            .unwrap();
        let usage = service.usage();
        assert_eq!(usage.bindings, 1);
        assert_eq!(usage.bindings_high_water, 1);
        assert_eq!(usage.bindings_installed, 2);
        assert_eq!(usage.bindings_unbound, 1);
        assert_eq!(usage.binding_refusals, 3);
    }

    struct HandleIssuer(u8);

    impl SemanticPlanHandleIssuerV1 for HandleIssuer {
        fn issue(&mut self) -> Option<SemanticPlanHandleV1> {
            let value = self.0;
            self.0 = self.0.checked_add(1)?;
            SemanticPlanHandleV1::try_new([value; 16])
        }
    }

    fn plan_registry(
        authority_budget: u64,
        limits: SemanticPlanRegistryLimitsV1,
    ) -> VolatileSemanticPlanRegistryV1<FixedAuthorityV1, HandleIssuer> {
        let apply = SemanticApplyServiceV1::try_new(
            authority(authority_budget),
            resource(),
            SemanticApplyLimitsV1::reference(),
        )
        .unwrap();
        VolatileSemanticPlanRegistryV1::try_new(apply, HandleIssuer(1), limits).unwrap()
    }

    #[test]
    fn retained_plan_is_server_owned_client_bound_and_exactly_replayed() {
        let file = Fixture::new("alpha");
        let mut workspace = workspace(&[&file]);
        let id = workspace.inventory().unwrap().documents.remove(0);
        let capture = workspace.document_snapshot(&id).unwrap();
        let target = target(1, capture.version.revision);
        let client = SemanticClientRefV1::new([4; 16]);
        let foreign = SemanticClientRefV1::new([5; 16]);
        let provider = provider();
        let reply = reply(target, vec![edit(target, 0, 5, "one")]);
        let mut registry = plan_registry(1, SemanticPlanRegistryLimitsV1::reference());
        registry
            .bind_snapshot(client, &snapshot(target, "alpha"), &mut workspace, &id)
            .unwrap();
        let issued = registry
            .issue_plan(&mut workspace, client, context(), &provider, &reply)
            .unwrap();
        assert_eq!(
            format!("{:?}", issued.digest),
            "SemanticPlanDigestV1(<opaque>)"
        );
        assert_eq!(registry.plan_digest(foreign, issued.handle), None);
        assert_eq!(
            registry.expire_plan(foreign, issued.handle),
            Err(SemanticApplyRefusalV1::UnknownPlan)
        );

        let operation = SemanticApplyOperationRefV1::try_new([1; 16]).unwrap();
        assert_eq!(
            registry.execute(&mut workspace, foreign, operation, issued.handle, &provider,),
            VolatileSemanticPlanExecutionOutcomeV1::AbsentOrForeign
        );
        let expected = VolatileSemanticPlanExecutionOutcomeV1::Complete(
            SemanticApplyOutcomeV1::Applied(SemanticApplyReceiptV1 {
                changed_documents: 1,
                applied_edits: 1,
                persistence: SemanticPersistenceV1::NotAttempted,
            }),
        );
        assert_eq!(
            registry.execute(&mut workspace, client, operation, issued.handle, &provider,),
            expected
        );
        assert_eq!(workspace.document_snapshot(&id).unwrap().text, "one");
        let revision = workspace.document_snapshot(&id).unwrap().version.revision;
        assert_eq!(
            registry.execute(&mut workspace, client, operation, issued.handle, &provider,),
            expected
        );
        assert_eq!(
            workspace.document_snapshot(&id).unwrap().version.revision,
            revision
        );
        assert_eq!(registry.usage().operation_replays, 1);
        assert_eq!(
            registry.prepare_operation(foreign, operation, issued.handle),
            VolatileSemanticPlanOperationAdmissionV1::AbsentOrForeign
        );
        assert_eq!(
            registry
                .execute_prepared(&mut workspace, foreign, operation, issued.handle, &provider,),
            VolatileSemanticPlanExecutionOutcomeV1::AbsentOrForeign
        );
        assert_eq!(registry.usage().operation_conflicts, 0);
    }

    #[test]
    fn aggregate_plan_payload_and_edit_bounds_are_exact_and_never_evict() {
        let file = Fixture::new("alpha");
        let mut workspace = workspace(&[&file]);
        let id = workspace.inventory().unwrap().documents.remove(0);
        let capture = workspace.document_snapshot(&id).unwrap();
        let target = target(1, capture.version.revision);
        let client = SemanticClientRefV1::new([4; 16]);
        let provider = provider();
        let reply = reply(target, vec![edit(target, 0, 5, "one")]);

        let mut payload_limits = SemanticPlanRegistryLimitsV1::reference();
        payload_limits.max_plans = 2;
        payload_limits.max_retained_edits = 2;
        payload_limits.max_retained_payload_bytes = 3;
        let mut payload_registry = plan_registry(1, payload_limits);
        payload_registry
            .bind_snapshot(client, &snapshot(target, "alpha"), &mut workspace, &id)
            .unwrap();
        payload_registry
            .issue_plan(&mut workspace, client, context(), &provider, &reply)
            .unwrap();
        assert_eq!(
            payload_registry.issue_plan(&mut workspace, client, context(), &provider, &reply,),
            Err(SemanticApplyRefusalV1::Capacity)
        );
        let usage = payload_registry.usage();
        assert_eq!(usage.plans, 1);
        assert_eq!(usage.retained_edits, 1);
        assert_eq!(usage.retained_edits_high_water, 1);
        assert_eq!(usage.retained_payload_bytes, 3);
        assert_eq!(usage.retained_payload_bytes_high_water, 3);

        let mut edit_limits = SemanticPlanRegistryLimitsV1::reference();
        edit_limits.max_plans = 2;
        edit_limits.max_retained_edits = 1;
        edit_limits.max_retained_payload_bytes = 6;
        let mut edit_registry = plan_registry(1, edit_limits);
        edit_registry
            .bind_snapshot(client, &snapshot(target, "alpha"), &mut workspace, &id)
            .unwrap();
        edit_registry
            .issue_plan(&mut workspace, client, context(), &provider, &reply)
            .unwrap();
        assert_eq!(
            edit_registry.issue_plan(&mut workspace, client, context(), &provider, &reply),
            Err(SemanticApplyRefusalV1::Capacity)
        );
        assert_eq!(edit_registry.usage().plans, 1);
        assert_eq!(edit_registry.usage().retained_edits, 1);
        assert_eq!(edit_registry.usage().retained_payload_bytes, 3);
    }

    #[test]
    fn pending_operation_is_unknown_and_changed_retry_conflicts_without_effect() {
        let file = Fixture::new("alpha");
        let mut workspace = workspace(&[&file]);
        let id = workspace.inventory().unwrap().documents.remove(0);
        let capture = workspace.document_snapshot(&id).unwrap();
        let target = target(1, capture.version.revision);
        let client = SemanticClientRefV1::new([4; 16]);
        let provider = provider();
        let reply = reply(target, vec![edit(target, 0, 5, "one")]);
        let mut registry = plan_registry(1, SemanticPlanRegistryLimitsV1::reference());
        registry
            .bind_snapshot(client, &snapshot(target, "alpha"), &mut workspace, &id)
            .unwrap();
        let first = registry
            .issue_plan(&mut workspace, client, context(), &provider, &reply)
            .unwrap();
        let second = registry
            .issue_plan(&mut workspace, client, context(), &provider, &reply)
            .unwrap();
        let operation = SemanticApplyOperationRefV1::try_new([2; 16]).unwrap();
        assert_eq!(
            registry.prepare_operation(client, operation, first.handle),
            VolatileSemanticPlanOperationAdmissionV1::Prepared
        );
        assert_eq!(
            registry.execute(&mut workspace, client, operation, first.handle, &provider,),
            VolatileSemanticPlanExecutionOutcomeV1::OutcomeUnknown
        );
        assert_eq!(
            registry.prepare_operation(client, operation, second.handle),
            VolatileSemanticPlanOperationAdmissionV1::Conflict
        );
        assert_eq!(workspace.document_snapshot(&id).unwrap().text, "alpha");
    }

    #[test]
    fn expiry_stale_provider_and_capacity_are_closed_without_eviction() {
        let file = Fixture::new("alpha");
        let mut workspace = workspace(&[&file]);
        let id = workspace.inventory().unwrap().documents.remove(0);
        let capture = workspace.document_snapshot(&id).unwrap();
        let target = target(1, capture.version.revision);
        let client = SemanticClientRefV1::new([4; 16]);
        let provider = provider();
        let reply = reply(target, vec![edit(target, 0, 5, "one")]);
        let mut limits = SemanticPlanRegistryLimitsV1::reference();
        limits.max_plans = 1;
        limits.max_operations = 1;
        let mut registry = plan_registry(1, limits);
        registry
            .bind_snapshot(client, &snapshot(target, "alpha"), &mut workspace, &id)
            .unwrap();
        let issued = registry
            .issue_plan(&mut workspace, client, context(), &provider, &reply)
            .unwrap();
        assert!(registry.expire_plan(client, issued.handle).unwrap());
        assert!(!registry.expire_plan(client, issued.handle).unwrap());
        assert_eq!(
            registry.issue_plan(&mut workspace, client, context(), &provider, &reply),
            Err(SemanticApplyRefusalV1::Capacity)
        );
        let operation = SemanticApplyOperationRefV1::try_new([3; 16]).unwrap();
        assert_eq!(
            registry.execute(&mut workspace, client, operation, issued.handle, &provider,),
            VolatileSemanticPlanExecutionOutcomeV1::Complete(SemanticApplyOutcomeV1::Refused(
                SemanticApplyRefusalV1::PlanExpired
            ))
        );
        assert_eq!(
            registry.prepare_operation(
                client,
                SemanticApplyOperationRefV1::try_new([4; 16]).unwrap(),
                issued.handle,
            ),
            VolatileSemanticPlanOperationAdmissionV1::Capacity
        );
        assert_eq!(workspace.document_snapshot(&id).unwrap().text, "alpha");

        // A fresh registry proves provider replacement after issuance is stale.
        let mut stale_registry = plan_registry(1, SemanticPlanRegistryLimitsV1::reference());
        stale_registry
            .bind_snapshot(client, &snapshot(target, "alpha"), &mut workspace, &id)
            .unwrap();
        let stale = stale_registry
            .issue_plan(&mut workspace, client, context(), &provider, &reply)
            .unwrap();
        let mut changed_provider = provider.clone();
        changed_provider.readiness = SemanticProviderReadinessV1::Ready {
            generation: SemanticIndexGenerationV1::new(2).unwrap(),
        };
        assert_eq!(
            stale_registry.execute(
                &mut workspace,
                client,
                SemanticApplyOperationRefV1::try_new([5; 16]).unwrap(),
                stale.handle,
                &changed_provider,
            ),
            VolatileSemanticPlanExecutionOutcomeV1::Complete(SemanticApplyOutcomeV1::Refused(
                SemanticApplyRefusalV1::ProviderStale
            ))
        );
    }

    #[test]
    fn unwind_after_pending_retains_unknown_and_never_reenters_authority() {
        let file = Fixture::new("alpha");
        let mut workspace = workspace(&[&file]);
        let id = workspace.inventory().unwrap().documents.remove(0);
        let capture = workspace.document_snapshot(&id).unwrap();
        let target = target(1, capture.version.revision);
        let client = SemanticClientRefV1::new([4; 16]);
        let provider = provider();
        let reply = reply(target, vec![edit(target, 0, 5, "one")]);
        let apply = SemanticApplyServiceV1::try_new(
            PanicAuthority,
            resource(),
            SemanticApplyLimitsV1::reference(),
        )
        .unwrap();
        let mut registry = VolatileSemanticPlanRegistryV1::try_new(
            apply,
            HandleIssuer(1),
            SemanticPlanRegistryLimitsV1::reference(),
        )
        .unwrap();
        registry
            .bind_snapshot(client, &snapshot(target, "alpha"), &mut workspace, &id)
            .unwrap();
        let issued = registry
            .issue_plan(&mut workspace, client, context(), &provider, &reply)
            .unwrap();
        let operation = SemanticApplyOperationRefV1::try_new([6; 16]).unwrap();
        let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            registry.execute(&mut workspace, client, operation, issued.handle, &provider)
        }));
        assert!(unwind.is_err());
        assert_eq!(workspace.document_snapshot(&id).unwrap().text, "alpha");
        assert_eq!(
            registry.execute(&mut workspace, client, operation, issued.handle, &provider,),
            VolatileSemanticPlanExecutionOutcomeV1::OutcomeUnknown
        );
    }

    fn journaled_registry(
        authority_budget: u64,
        journal: InMemorySemanticPlanJournalV1,
    ) -> JournaledSemanticPlanRegistryV1<
        FixedAuthorityV1,
        HandleIssuer,
        InMemorySemanticPlanJournalV1,
    > {
        journaled_registry_with_limits(
            authority_budget,
            journal,
            SemanticPlanJournalLimitsV1::reference(),
        )
    }

    fn journaled_registry_with_limits(
        authority_budget: u64,
        journal: InMemorySemanticPlanJournalV1,
        journal_limits: SemanticPlanJournalLimitsV1,
    ) -> JournaledSemanticPlanRegistryV1<
        FixedAuthorityV1,
        HandleIssuer,
        InMemorySemanticPlanJournalV1,
    > {
        let apply = SemanticApplyServiceV1::try_new(
            authority(authority_budget),
            resource(),
            SemanticApplyLimitsV1::reference(),
        )
        .unwrap();
        JournaledSemanticPlanRegistryV1::try_new_empty(
            apply,
            HandleIssuer(1),
            journal,
            SemanticPlanRegistryLimitsV1::reference(),
            journal_limits,
        )
        .unwrap()
    }

    #[test]
    fn crash_after_effect_before_complete_sync_recovers_pending_without_reapply() {
        let file = Fixture::new("alpha");
        let mut workspace = workspace(&[&file]);
        let id = workspace.inventory().unwrap().documents.remove(0);
        let capture = workspace.document_snapshot(&id).unwrap();
        let target = target(1, capture.version.revision);
        let client = SemanticClientRefV1::new([4; 16]);
        let provider = provider();
        let reply = reply(target, vec![edit(target, 0, 5, "one")]);
        let operation = SemanticApplyOperationRefV1::try_new([7; 16]).unwrap();
        let mut journal =
            InMemorySemanticPlanJournalV1::try_new(SemanticPlanJournalLimitsV1::reference())
                .unwrap();
        journal.script_append(SemanticPlanJournalCommitV1::Committed, true);
        journal.script_append(SemanticPlanJournalCommitV1::Committed, true);
        journal.script_append(SemanticPlanJournalCommitV1::Indeterminate, false);
        let mut registry = journaled_registry(1, journal);
        registry
            .bind_snapshot(client, &snapshot(target, "alpha"), &mut workspace, &id)
            .unwrap();
        let JournaledSemanticPlanIssueOutcomeV1::Issued(issued) =
            registry.issue_plan(&mut workspace, client, context(), &provider, &reply)
        else {
            panic!("plan must be issued");
        };
        assert_eq!(
            registry.execute(&mut workspace, client, operation, issued.handle, &provider),
            JournaledSemanticPlanExecutionOutcomeV1::OutcomeUnknown
        );
        assert!(registry.is_fenced());
        assert_eq!(workspace.document_snapshot(&id).unwrap().text, "one");
        let revision_after_effect = workspace.document_snapshot(&id).unwrap().version.revision;
        let (_, journal) = registry.into_parts();

        let apply = SemanticApplyServiceV1::try_new(
            authority(1),
            resource(),
            SemanticApplyLimitsV1::reference(),
        )
        .unwrap();
        let mut recovered = JournaledSemanticPlanRegistryV1::try_recover(
            apply,
            HandleIssuer(2),
            journal,
            SemanticPlanRegistryLimitsV1::reference(),
            SemanticPlanJournalLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(
            recovered.execute(&mut workspace, client, operation, issued.handle, &provider),
            JournaledSemanticPlanExecutionOutcomeV1::OutcomeUnknown
        );
        assert_eq!(
            workspace.document_snapshot(&id).unwrap().version.revision,
            revision_after_effect
        );
    }

    #[test]
    fn committed_receipt_recovers_and_replays_without_workspace_or_authority_entry() {
        let file = Fixture::new("alpha");
        let mut workspace = workspace(&[&file]);
        let id = workspace.inventory().unwrap().documents.remove(0);
        let capture = workspace.document_snapshot(&id).unwrap();
        let target = target(1, capture.version.revision);
        let client = SemanticClientRefV1::new([4; 16]);
        let provider = provider();
        let reply = reply(target, vec![edit(target, 0, 5, "one")]);
        let operation = SemanticApplyOperationRefV1::try_new([8; 16]).unwrap();
        let journal =
            InMemorySemanticPlanJournalV1::try_new(SemanticPlanJournalLimitsV1::reference())
                .unwrap();
        let mut registry = journaled_registry(1, journal);
        registry
            .bind_snapshot(client, &snapshot(target, "alpha"), &mut workspace, &id)
            .unwrap();
        let JournaledSemanticPlanIssueOutcomeV1::Issued(issued) =
            registry.issue_plan(&mut workspace, client, context(), &provider, &reply)
        else {
            panic!("plan must be issued");
        };
        let expected = JournaledSemanticPlanExecutionOutcomeV1::Complete(
            SemanticApplyOutcomeV1::Applied(receipt(1, 1)),
        );
        assert_eq!(
            registry.execute(&mut workspace, client, operation, issued.handle, &provider),
            expected
        );
        let revision = workspace.document_snapshot(&id).unwrap().version.revision;
        let (_, journal) = registry.into_parts();
        let apply = SemanticApplyServiceV1::try_new(
            PanicAuthority,
            resource(),
            SemanticApplyLimitsV1::reference(),
        )
        .unwrap();
        let mut recovered = JournaledSemanticPlanRegistryV1::try_recover(
            apply,
            HandleIssuer(2),
            journal,
            SemanticPlanRegistryLimitsV1::reference(),
            SemanticPlanJournalLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(
            recovered.execute(&mut workspace, client, operation, issued.handle, &provider),
            expected
        );
        assert_eq!(
            workspace.document_snapshot(&id).unwrap().version.revision,
            revision
        );
    }

    #[test]
    fn disk_journal_recovers_complete_and_pending_without_reapplying_effects() {
        let file = Fixture::new("alpha");
        let journal_root = JournalFixture::new();
        let mut workspace = workspace(&[&file]);
        let id = workspace.inventory().unwrap().documents.remove(0);
        let capture = workspace.document_snapshot(&id).unwrap();
        let target = target(1, capture.version.revision);
        let client = SemanticClientRefV1::new([4; 16]);
        let provider = provider();
        let reply = reply(target, vec![edit(target, 0, 5, "one")]);
        let complete_operation = SemanticApplyOperationRefV1::try_new([12; 16]).unwrap();
        let pending_operation = SemanticApplyOperationRefV1::try_new([13; 16]).unwrap();
        let namespace = LocalAppendJournalNamespaceV1 {
            workspace: [21; 16],
            owner: [22; 16],
        };
        let mut journal_limits = SemanticPlanJournalLimitsV1::reference();
        journal_limits.max_records = 4;
        let journal =
            LocalAppendJournalV1::try_open(journal_root.path(), namespace, journal_limits).unwrap();
        let apply = SemanticApplyServiceV1::try_new(
            authority(1),
            resource(),
            SemanticApplyLimitsV1::reference(),
        )
        .unwrap();
        let mut registry = JournaledSemanticPlanRegistryV1::try_new_empty(
            apply,
            HandleIssuer(1),
            journal,
            SemanticPlanRegistryLimitsV1::reference(),
            journal_limits,
        )
        .unwrap();
        registry
            .bind_snapshot(client, &snapshot(target, "alpha"), &mut workspace, &id)
            .unwrap();
        let JournaledSemanticPlanIssueOutcomeV1::Issued(issued) =
            registry.issue_plan(&mut workspace, client, context(), &provider, &reply)
        else {
            panic!("disk-backed plan must be issued");
        };
        let expected = JournaledSemanticPlanExecutionOutcomeV1::Complete(
            SemanticApplyOutcomeV1::Applied(receipt(1, 1)),
        );
        assert_eq!(
            registry.execute(
                &mut workspace,
                client,
                complete_operation,
                issued.handle,
                &provider,
            ),
            expected
        );
        assert_eq!(
            registry.prepare_operation(client, pending_operation, issued.handle),
            JournaledSemanticPlanOperationAdmissionV1::Prepared
        );
        assert_eq!(workspace.document_snapshot(&id).unwrap().text, "one");
        let revision_after_effect = workspace.document_snapshot(&id).unwrap().version.revision;
        assert_eq!(
            registry.expire_plan(client, issued.handle),
            JournaledSemanticPlanExpirationOutcomeV1::JournalRefused
        );
        drop(registry);

        let mut foreign_namespace = namespace;
        foreign_namespace.owner = [23; 16];
        assert!(matches!(
            LocalAppendJournalV1::try_open(journal_root.path(), foreign_namespace, journal_limits,),
            Err(LocalAppendJournalOpenRefusalV1::NamespaceMismatch)
        ));
        let recovered_journal =
            LocalAppendJournalV1::try_open(journal_root.path(), namespace, journal_limits).unwrap();
        let apply = SemanticApplyServiceV1::try_new(
            PanicAuthority,
            resource(),
            SemanticApplyLimitsV1::reference(),
        )
        .unwrap();
        let mut recovered = JournaledSemanticPlanRegistryV1::try_recover(
            apply,
            HandleIssuer(2),
            recovered_journal,
            SemanticPlanRegistryLimitsV1::reference(),
            journal_limits,
        )
        .unwrap();
        assert_eq!(
            recovered.execute(
                &mut workspace,
                client,
                complete_operation,
                issued.handle,
                &provider,
            ),
            expected
        );
        assert_eq!(
            recovered.execute(
                &mut workspace,
                client,
                pending_operation,
                issued.handle,
                &provider,
            ),
            JournaledSemanticPlanExecutionOutcomeV1::OutcomeUnknown
        );
        assert_eq!(workspace.document_snapshot(&id).unwrap().text, "one");
        assert_eq!(
            workspace.document_snapshot(&id).unwrap().version.revision,
            revision_after_effect
        );
    }

    #[test]
    fn wrapper_record_limit_preflights_a_more_permissive_adapter_and_remains_recoverable() {
        let file = Fixture::new("alpha");
        let mut workspace = workspace(&[&file]);
        let id = workspace.inventory().unwrap().documents.remove(0);
        let capture = workspace.document_snapshot(&id).unwrap();
        let target = target(1, capture.version.revision);
        let client = SemanticClientRefV1::new([4; 16]);
        let provider = provider();
        let reply = reply(target, vec![edit(target, 0, 5, "one")]);
        let operation = SemanticApplyOperationRefV1::try_new([10; 16]).unwrap();
        let adapter =
            InMemorySemanticPlanJournalV1::try_new(SemanticPlanJournalLimitsV1::reference())
                .unwrap();
        let mut wrapper_limits = SemanticPlanJournalLimitsV1::reference();
        wrapper_limits.max_records = 2;
        let mut registry = journaled_registry_with_limits(1, adapter, wrapper_limits);
        registry
            .bind_snapshot(client, &snapshot(target, "alpha"), &mut workspace, &id)
            .unwrap();
        let JournaledSemanticPlanIssueOutcomeV1::Issued(issued) =
            registry.issue_plan(&mut workspace, client, context(), &provider, &reply)
        else {
            panic!("plan must be issued");
        };
        assert_eq!(
            registry.execute(&mut workspace, client, operation, issued.handle, &provider),
            JournaledSemanticPlanExecutionOutcomeV1::OutcomeUnknown
        );
        assert!(!registry.is_fenced());
        let revision = workspace.document_snapshot(&id).unwrap().version.revision;
        let (_, journal) = registry.into_parts();
        assert_eq!(journal.records().len(), 2);

        let apply = SemanticApplyServiceV1::try_new(
            PanicAuthority,
            resource(),
            SemanticApplyLimitsV1::reference(),
        )
        .unwrap();
        let mut recovered = JournaledSemanticPlanRegistryV1::try_recover(
            apply,
            HandleIssuer(2),
            journal,
            SemanticPlanRegistryLimitsV1::reference(),
            wrapper_limits,
        )
        .unwrap();
        assert_eq!(
            recovered.execute(&mut workspace, client, operation, issued.handle, &provider),
            JournaledSemanticPlanExecutionOutcomeV1::OutcomeUnknown
        );
        assert_eq!(
            workspace.document_snapshot(&id).unwrap().version.revision,
            revision
        );
    }

    #[test]
    fn wrapper_byte_limit_preflights_a_more_permissive_adapter_and_remains_recoverable() {
        let file = Fixture::new("alpha");
        let mut workspace = workspace(&[&file]);
        let id = workspace.inventory().unwrap().documents.remove(0);
        let capture = workspace.document_snapshot(&id).unwrap();
        let target = target(1, capture.version.revision);
        let client = SemanticClientRefV1::new([4; 16]);
        let provider = provider();
        let reply = reply(target, vec![edit(target, 0, 5, "one")]);
        let operation = SemanticApplyOperationRefV1::try_new([11; 16]).unwrap();

        let probe_adapter =
            InMemorySemanticPlanJournalV1::try_new(SemanticPlanJournalLimitsV1::reference())
                .unwrap();
        let mut probe = journaled_registry(1, probe_adapter);
        probe
            .bind_snapshot(client, &snapshot(target, "alpha"), &mut workspace, &id)
            .unwrap();
        let JournaledSemanticPlanIssueOutcomeV1::Issued(probe_issued) =
            probe.issue_plan(&mut workspace, client, context(), &provider, &reply)
        else {
            panic!("probe plan must be issued");
        };
        assert_eq!(
            probe.prepare_operation(client, operation, probe_issued.handle),
            JournaledSemanticPlanOperationAdmissionV1::Prepared
        );
        let (_, probe_journal) = probe.into_parts();
        let retained_bytes = probe_journal.records().iter().map(Vec::len).sum::<usize>();
        let largest_record = probe_journal.records().iter().map(Vec::len).max().unwrap();

        let mut wrapper_limits = SemanticPlanJournalLimitsV1::reference();
        wrapper_limits.max_record_bytes = largest_record;
        wrapper_limits.max_total_bytes = retained_bytes;
        let adapter =
            InMemorySemanticPlanJournalV1::try_new(SemanticPlanJournalLimitsV1::reference())
                .unwrap();
        let mut registry = journaled_registry_with_limits(1, adapter, wrapper_limits);
        registry
            .bind_snapshot(client, &snapshot(target, "alpha"), &mut workspace, &id)
            .unwrap();
        let JournaledSemanticPlanIssueOutcomeV1::Issued(issued) =
            registry.issue_plan(&mut workspace, client, context(), &provider, &reply)
        else {
            panic!("plan must be issued");
        };
        assert_eq!(
            registry.execute(&mut workspace, client, operation, issued.handle, &provider),
            JournaledSemanticPlanExecutionOutcomeV1::OutcomeUnknown
        );
        let revision = workspace.document_snapshot(&id).unwrap().version.revision;
        let (_, journal) = registry.into_parts();
        assert_eq!(
            journal.records().iter().map(Vec::len).sum::<usize>(),
            retained_bytes
        );
        let apply = SemanticApplyServiceV1::try_new(
            PanicAuthority,
            resource(),
            SemanticApplyLimitsV1::reference(),
        )
        .unwrap();
        let mut recovered = JournaledSemanticPlanRegistryV1::try_recover(
            apply,
            HandleIssuer(2),
            journal,
            SemanticPlanRegistryLimitsV1::reference(),
            wrapper_limits,
        )
        .unwrap();
        assert_eq!(
            recovered.execute(&mut workspace, client, operation, issued.handle, &provider),
            JournaledSemanticPlanExecutionOutcomeV1::OutcomeUnknown
        );
        assert_eq!(
            workspace.document_snapshot(&id).unwrap().version.revision,
            revision
        );
    }
}
