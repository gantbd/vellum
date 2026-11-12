//! Composition-owned projection from native Vellum snapshot products into a
//! semantic-session synchronization plan.
//!
//! This module opens no transport and infers no workspace, locator, language,
//! identity, or authority. Callers supply one complete native inventory, one
//! exact public-SDK snapshot per member, and an injected language registration.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

use vellum_host::semantic::{SemanticDocumentRefV1, SemanticSourceRevisionV1, SemanticTargetV1};
use vellum_sdk_client::protocol::{
    OutcomeV1, ReplyV1, ServiceAccessErrorV1, ServicePort, SnapshotV1, VersionWitness,
    WorkspaceDocumentIdV1, WorkspaceIdV1, WorkspaceInventoryV1,
};
use vellum_semantic_protocol::{
    SemanticExchangeDispositionV1, SemanticExchangeRefV1, SemanticSessionCommandV1,
    SemanticSessionLimitsV1, SemanticSessionOperationRefV1, SemanticSessionOutcomeV1,
    SemanticSessionSnapshotV1, SemanticSynchronizationEpochV1, SemanticSynchronizationManifestV1,
};

use crate::{
    SemanticSessionClientBindingV1, SemanticSessionClientRefusalV1, SemanticSessionClientV1,
    SemanticSessionExchangePortV1,
};

#[derive(Clone)]
pub struct VellumSemanticSnapshotProductV1 {
    document: WorkspaceDocumentIdV1,
    language: String,
    snapshot: SnapshotV1,
}

impl VellumSemanticSnapshotProductV1 {
    /// The language is a trusted composition registration. It must not be
    /// inferred from a locator, extension, workspace file, or source content.
    pub fn try_new(
        inventory: &WorkspaceInventoryV1,
        document: WorkspaceDocumentIdV1,
        language: String,
        snapshot: SnapshotV1,
    ) -> Option<Self> {
        (document.workspace() == inventory.workspace
            && inventory.documents.contains(&document)
            && !language.is_empty())
        .then_some(Self {
            document,
            language,
            snapshot,
        })
    }

    pub const fn document(&self) -> WorkspaceDocumentIdV1 {
        self.document
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    pub const fn revision(&self) -> u64 {
        self.snapshot.revision
    }

    pub const fn version_witness(&self) -> VersionWitness {
        self.snapshot.witness
    }

    pub fn utf8_bytes(&self) -> usize {
        self.snapshot.text.len()
    }
}

impl fmt::Debug for VellumSemanticSnapshotProductV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VellumSemanticSnapshotProductV1")
            .field("document", &"<redacted>")
            .field("language", &"<redacted>")
            .field("snapshot", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticSynchronizationCommandIdentityV1 {
    pub exchange: SemanticExchangeRefV1,
    pub operation: SemanticSessionOperationRefV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VellumSnapshotSynchronizationRefusalV1 {
    WorkspaceChanged,
    IncompleteMembership,
    DuplicateDocument,
    StaleRevision,
    RevisionConflict,
    InvalidSnapshot,
    IdentityCardinality,
    Capacity,
    StalePlan,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VellumSnapshotSynchronizationUsageV1 {
    pub plans: u64,
    pub commits: u64,
    pub stale_refusals: u64,
    pub conflict_refusals: u64,
    pub membership_refusals: u64,
    pub capacity_refusals: u64,
    pub planned_commands: u64,
    pub planned_documents: u64,
    pub planned_utf8_bytes: u64,
    pub synchronized_documents: u64,
    pub synchronized_utf8_bytes: u64,
    pub counter_saturated: bool,
}

#[derive(Clone)]
struct RetainedNativeSnapshotV1 {
    workspace_document: WorkspaceDocumentIdV1,
    target: SemanticTargetV1,
    witness: VersionWitness,
    language: String,
    source: Arc<str>,
    utf8_boundaries: u64,
}

impl RetainedNativeSnapshotV1 {
    fn from_product(product: &VellumSemanticSnapshotProductV1) -> Self {
        let target = SemanticTargetV1 {
            document: semantic_document_ref(product.document),
            source_revision: SemanticSourceRevisionV1::new(product.snapshot.revision),
        };
        Self {
            workspace_document: product.document,
            target,
            witness: product.snapshot.witness,
            language: product.language.clone(),
            source: Arc::clone(&product.snapshot.text),
            utf8_boundaries: product
                .snapshot
                .text
                .char_indices()
                .count()
                .saturating_add(1)
                .try_into()
                .unwrap_or(u64::MAX),
        }
    }

    fn semantic_snapshot(
        &self,
        limits: SemanticSessionLimitsV1,
    ) -> Option<SemanticSessionSnapshotV1> {
        SemanticSessionSnapshotV1::try_new(
            self.target,
            self.language.clone(),
            self.source.to_string(),
            limits,
        )
    }

    fn exact_same_revision(&self, other: &Self) -> bool {
        self.workspace_document == other.workspace_document
            && self.target == other.target
            && self.witness == other.witness
            && self.language == other.language
            && self.source == other.source
            && self.utf8_boundaries == other.utf8_boundaries
    }
}

pub struct VellumSnapshotSynchronizationPlanV1 {
    base_generation: u64,
    workspace: WorkspaceIdV1,
    commands: Vec<SemanticSessionCommandV1>,
    next: BTreeMap<SemanticDocumentRefV1, RetainedNativeSnapshotV1>,
    utf8_bytes: u64,
}

impl VellumSnapshotSynchronizationPlanV1 {
    pub fn commands(&self) -> &[SemanticSessionCommandV1] {
        &self.commands
    }

    pub const fn workspace(&self) -> WorkspaceIdV1 {
        self.workspace
    }
}

impl fmt::Debug for VellumSnapshotSynchronizationPlanV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VellumSnapshotSynchronizationPlanV1")
            .field("commands", &self.commands.len())
            .field("documents", &self.next.len())
            .field("content", &"<redacted>")
            .finish()
    }
}

#[derive(Default)]
pub struct VellumSnapshotSynchronizationCoordinatorV1 {
    generation: u64,
    workspace: Option<WorkspaceIdV1>,
    documents: BTreeMap<SemanticDocumentRefV1, RetainedNativeSnapshotV1>,
    usage: VellumSnapshotSynchronizationUsageV1,
}

impl VellumSnapshotSynchronizationCoordinatorV1 {
    pub const fn usage(&self) -> VellumSnapshotSynchronizationUsageV1 {
        self.usage
    }

    pub fn plan(
        &mut self,
        binding: SemanticSessionClientBindingV1,
        epoch: SemanticSynchronizationEpochV1,
        inventory: &WorkspaceInventoryV1,
        products: &[VellumSemanticSnapshotProductV1],
        identities: &[SemanticSynchronizationCommandIdentityV1],
        limits: SemanticSessionLimitsV1,
    ) -> Result<VellumSnapshotSynchronizationPlanV1, VellumSnapshotSynchronizationRefusalV1> {
        if self
            .workspace
            .is_some_and(|workspace| workspace != inventory.workspace)
        {
            return self.refuse(VellumSnapshotSynchronizationRefusalV1::WorkspaceChanged);
        }
        if products.len() != inventory.documents.len() {
            return self.refuse(VellumSnapshotSynchronizationRefusalV1::IncompleteMembership);
        }
        let member_set: BTreeSet<_> = inventory
            .documents
            .iter()
            .map(|document| (document.workspace().get(), document.get()))
            .collect();
        let mut observed_members = BTreeSet::new();
        let mut next = BTreeMap::new();
        for product in products {
            let member = (product.document.workspace().get(), product.document.get());
            if product.document.workspace() != inventory.workspace || !member_set.contains(&member)
            {
                return self.refuse(VellumSnapshotSynchronizationRefusalV1::IncompleteMembership);
            }
            if !observed_members.insert(member) {
                return self.refuse(VellumSnapshotSynchronizationRefusalV1::DuplicateDocument);
            }
            let retained = RetainedNativeSnapshotV1::from_product(product);
            if retained.semantic_snapshot(limits).is_none() {
                return self.refuse(VellumSnapshotSynchronizationRefusalV1::InvalidSnapshot);
            }
            if next.insert(retained.target.document, retained).is_some() {
                return self.refuse(VellumSnapshotSynchronizationRefusalV1::DuplicateDocument);
            }
        }

        let mut changes = Vec::new();
        for (document, retained) in &next {
            match self.documents.get(document) {
                None => changes.push(SynchronizationDelta::Open(retained.clone())),
                Some(previous)
                    if retained.target.source_revision.get()
                        < previous.target.source_revision.get() =>
                {
                    return self.refuse(VellumSnapshotSynchronizationRefusalV1::StaleRevision);
                }
                Some(previous)
                    if retained.target.source_revision == previous.target.source_revision =>
                {
                    if !previous.exact_same_revision(retained) {
                        return self
                            .refuse(VellumSnapshotSynchronizationRefusalV1::RevisionConflict);
                    }
                }
                Some(_) => changes.push(SynchronizationDelta::Update(retained.clone())),
            }
        }
        for (document, retained) in &self.documents {
            if !next.contains_key(document) {
                changes.push(SynchronizationDelta::Close(retained.target));
            }
        }

        let required = changes.len().saturating_add(2);
        if identities.len() != required {
            return self.refuse(VellumSnapshotSynchronizationRefusalV1::IdentityCardinality);
        }
        let snapshots: Option<Vec<_>> = next
            .values()
            .map(|retained| retained.semantic_snapshot(limits))
            .collect();
        let snapshots = snapshots.ok_or(VellumSnapshotSynchronizationRefusalV1::InvalidSnapshot)?;
        let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(&snapshots, limits)
            .ok_or(VellumSnapshotSynchronizationRefusalV1::Capacity)?;
        let mut identity = identities.iter();
        let begin = *identity.next().expect("cardinality checked");
        let mut commands = vec![SemanticSessionCommandV1::BeginSynchronization {
            exchange: begin.exchange,
            operation: begin.operation,
            incarnation: binding.incarnation,
            membership: binding.membership,
            epoch,
            manifest,
        }];
        for change in changes {
            let correlation = *identity.next().expect("cardinality checked");
            commands.push(match change {
                SynchronizationDelta::Open(retained) => SemanticSessionCommandV1::SyncOpen {
                    exchange: correlation.exchange,
                    operation: correlation.operation,
                    incarnation: binding.incarnation,
                    membership: binding.membership,
                    epoch,
                    snapshot: retained.semantic_snapshot(limits).expect("validated"),
                },
                SynchronizationDelta::Update(retained) => SemanticSessionCommandV1::SyncChange {
                    exchange: correlation.exchange,
                    operation: correlation.operation,
                    incarnation: binding.incarnation,
                    membership: binding.membership,
                    epoch,
                    snapshot: retained.semantic_snapshot(limits).expect("validated"),
                },
                SynchronizationDelta::Close(target) => SemanticSessionCommandV1::SyncClose {
                    exchange: correlation.exchange,
                    operation: correlation.operation,
                    incarnation: binding.incarnation,
                    membership: binding.membership,
                    epoch,
                    target,
                },
            });
        }
        let complete = *identity.next().expect("cardinality checked");
        commands.push(SemanticSessionCommandV1::CompleteSynchronization {
            exchange: complete.exchange,
            operation: complete.operation,
            incarnation: binding.incarnation,
            membership: binding.membership,
            epoch,
            manifest,
        });
        let utf8_bytes = next.values().fold(0u64, |total, retained| {
            total.saturating_add(retained.source.len().try_into().unwrap_or(u64::MAX))
        });
        bump(&mut self.usage.plans, &mut self.usage.counter_saturated);
        add(
            &mut self.usage.planned_commands,
            commands.len(),
            &mut self.usage.counter_saturated,
        );
        add(
            &mut self.usage.planned_documents,
            next.len(),
            &mut self.usage.counter_saturated,
        );
        self.usage.planned_utf8_bytes = self.usage.planned_utf8_bytes.saturating_add(utf8_bytes);
        Ok(VellumSnapshotSynchronizationPlanV1 {
            base_generation: self.generation,
            workspace: inventory.workspace,
            commands,
            next,
            utf8_bytes,
        })
    }

    /// Commit only after the plan's complete-synchronization reply was
    /// definitive and accepted by `SemanticSessionClientV1`.
    pub fn commit(
        &mut self,
        plan: VellumSnapshotSynchronizationPlanV1,
    ) -> Result<(), VellumSnapshotSynchronizationRefusalV1> {
        if plan.base_generation != self.generation {
            return self.refuse(VellumSnapshotSynchronizationRefusalV1::StalePlan);
        }
        self.generation = self.generation.saturating_add(1);
        self.workspace = Some(plan.workspace);
        self.documents = plan.next;
        bump(&mut self.usage.commits, &mut self.usage.counter_saturated);
        self.usage.synchronized_documents = self.documents.len().try_into().unwrap_or(u64::MAX);
        self.usage.synchronized_utf8_bytes = plan.utf8_bytes;
        Ok(())
    }

    fn refuse<T>(
        &mut self,
        reason: VellumSnapshotSynchronizationRefusalV1,
    ) -> Result<T, VellumSnapshotSynchronizationRefusalV1> {
        match reason {
            VellumSnapshotSynchronizationRefusalV1::StaleRevision => bump(
                &mut self.usage.stale_refusals,
                &mut self.usage.counter_saturated,
            ),
            VellumSnapshotSynchronizationRefusalV1::RevisionConflict
            | VellumSnapshotSynchronizationRefusalV1::DuplicateDocument
            | VellumSnapshotSynchronizationRefusalV1::StalePlan => bump(
                &mut self.usage.conflict_refusals,
                &mut self.usage.counter_saturated,
            ),
            VellumSnapshotSynchronizationRefusalV1::WorkspaceChanged
            | VellumSnapshotSynchronizationRefusalV1::IncompleteMembership => bump(
                &mut self.usage.membership_refusals,
                &mut self.usage.counter_saturated,
            ),
            VellumSnapshotSynchronizationRefusalV1::InvalidSnapshot
            | VellumSnapshotSynchronizationRefusalV1::IdentityCardinality
            | VellumSnapshotSynchronizationRefusalV1::Capacity => bump(
                &mut self.usage.capacity_refusals,
                &mut self.usage.counter_saturated,
            ),
        }
        Err(reason)
    }
}

/// Composition supplies language registrations and fresh correlation values.
/// Neither source has authority and neither may inspect locators or source.
pub trait VellumSnapshotSynchronizationCompositionV1 {
    fn language(&mut self, document: WorkspaceDocumentIdV1) -> Option<String>;
    fn next_epoch(&mut self) -> Option<SemanticSynchronizationEpochV1>;
    fn next_identity(&mut self) -> Option<SemanticSynchronizationCommandIdentityV1>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VellumSnapshotSynchronizationExecutionRefusalV1 {
    NativeAccess(ServiceAccessErrorV1),
    NativeRefused,
    NativeReplyMismatch,
    MissingLanguage,
    MissingCorrelation,
    SessionBinding,
    Planning(VellumSnapshotSynchronizationRefusalV1),
    SessionClient(SemanticSessionClientRefusalV1),
    SemanticRefused,
}

#[must_use = "suspended synchronization owns its exact remaining plan"]
pub struct VellumSnapshotSynchronizationResumeV1 {
    plan: VellumSnapshotSynchronizationPlanV1,
    next: usize,
    pending: bool,
}

impl fmt::Debug for VellumSnapshotSynchronizationResumeV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VellumSnapshotSynchronizationResumeV1")
            .field("next", &self.next)
            .field("pending", &self.pending)
            .field("plan", &"<redacted>")
            .finish()
    }
}

#[must_use = "execution completion or affine resume ownership must be reconciled"]
pub enum VellumSnapshotSynchronizationExecutionV1 {
    Synchronized,
    Suspended(VellumSnapshotSynchronizationResumeV1),
    Refused(VellumSnapshotSynchronizationExecutionRefusalV1),
}

impl fmt::Debug for VellumSnapshotSynchronizationExecutionV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Synchronized => "Synchronized",
            Self::Suspended(_) => "Suspended(<resume-owner>)",
            Self::Refused(_) => "Refused",
        })
    }
}

/// Reads only public native products: exact inventory, explicit selection and
/// selected-document snapshot. Selection is client-local state, not workspace
/// membership or edit authority.
pub fn synchronize_vellum_snapshots<T, P, C>(
    native: &mut vellum_sdk_client::client::Client<T>,
    semantic: &mut SemanticSessionClientV1<P>,
    coordinator: &mut VellumSnapshotSynchronizationCoordinatorV1,
    composition: &mut C,
    limits: SemanticSessionLimitsV1,
) -> VellumSnapshotSynchronizationExecutionV1
where
    T: ServicePort,
    P: SemanticSessionExchangePortV1,
    C: VellumSnapshotSynchronizationCompositionV1,
{
    let inventory = match native.workspace_inventory() {
        Ok(ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(inventory))) => inventory,
        Ok(ReplyV1::Refused(_)) => {
            return VellumSnapshotSynchronizationExecutionV1::Refused(
                VellumSnapshotSynchronizationExecutionRefusalV1::NativeRefused,
            );
        }
        Ok(_) => {
            return VellumSnapshotSynchronizationExecutionV1::Refused(
                VellumSnapshotSynchronizationExecutionRefusalV1::NativeReplyMismatch,
            );
        }
        Err(error) => {
            return VellumSnapshotSynchronizationExecutionV1::Refused(
                VellumSnapshotSynchronizationExecutionRefusalV1::NativeAccess(error),
            );
        }
    };
    let mut products = Vec::with_capacity(inventory.documents.len());
    for document in &inventory.documents {
        match native.select_document(*document) {
            Ok(ReplyV1::Accepted(OutcomeV1::DocumentSelected(selected)))
                if selected == *document => {}
            Ok(ReplyV1::Refused(_)) => {
                return VellumSnapshotSynchronizationExecutionV1::Refused(
                    VellumSnapshotSynchronizationExecutionRefusalV1::NativeRefused,
                );
            }
            Ok(_) => {
                return VellumSnapshotSynchronizationExecutionV1::Refused(
                    VellumSnapshotSynchronizationExecutionRefusalV1::NativeReplyMismatch,
                );
            }
            Err(error) => {
                return VellumSnapshotSynchronizationExecutionV1::Refused(
                    VellumSnapshotSynchronizationExecutionRefusalV1::NativeAccess(error),
                );
            }
        }
        let snapshot = match native.snapshot() {
            Ok(ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot))) => snapshot,
            Ok(ReplyV1::Refused(_)) => {
                return VellumSnapshotSynchronizationExecutionV1::Refused(
                    VellumSnapshotSynchronizationExecutionRefusalV1::NativeRefused,
                );
            }
            Ok(_) => {
                return VellumSnapshotSynchronizationExecutionV1::Refused(
                    VellumSnapshotSynchronizationExecutionRefusalV1::NativeReplyMismatch,
                );
            }
            Err(error) => {
                return VellumSnapshotSynchronizationExecutionV1::Refused(
                    VellumSnapshotSynchronizationExecutionRefusalV1::NativeAccess(error),
                );
            }
        };
        let Some(language) = composition.language(*document) else {
            return VellumSnapshotSynchronizationExecutionV1::Refused(
                VellumSnapshotSynchronizationExecutionRefusalV1::MissingLanguage,
            );
        };
        let Some(product) =
            VellumSemanticSnapshotProductV1::try_new(&inventory, *document, language, snapshot)
        else {
            return VellumSnapshotSynchronizationExecutionV1::Refused(
                VellumSnapshotSynchronizationExecutionRefusalV1::NativeReplyMismatch,
            );
        };
        products.push(product);
    }
    let Some(binding) = semantic.binding() else {
        return VellumSnapshotSynchronizationExecutionV1::Refused(
            VellumSnapshotSynchronizationExecutionRefusalV1::SessionBinding,
        );
    };
    let Some(epoch) = composition.next_epoch() else {
        return VellumSnapshotSynchronizationExecutionV1::Refused(
            VellumSnapshotSynchronizationExecutionRefusalV1::MissingCorrelation,
        );
    };
    let projected: BTreeMap<_, _> = products
        .iter()
        .map(|product| {
            let retained = RetainedNativeSnapshotV1::from_product(product);
            (retained.target.document, retained)
        })
        .collect();
    let changed = projected
        .iter()
        .filter(|(document, retained)| {
            coordinator
                .documents
                .get(document)
                .is_none_or(|prior| !prior.exact_same_revision(retained))
        })
        .count();
    let closed = coordinator
        .documents
        .keys()
        .filter(|document| !projected.contains_key(document))
        .count();
    let identity_count = changed.saturating_add(closed).saturating_add(2);
    let identities: Option<Vec<_>> = (0..identity_count)
        .map(|_| composition.next_identity())
        .collect();
    let Some(identities) = identities else {
        return VellumSnapshotSynchronizationExecutionV1::Refused(
            VellumSnapshotSynchronizationExecutionRefusalV1::MissingCorrelation,
        );
    };
    let plan = match coordinator.plan(binding, epoch, &inventory, &products, &identities, limits) {
        Ok(plan) => plan,
        Err(reason) => {
            return VellumSnapshotSynchronizationExecutionV1::Refused(
                VellumSnapshotSynchronizationExecutionRefusalV1::Planning(reason),
            );
        }
    };
    drive_plan(semantic, coordinator, plan, 0, false, None)
}

impl VellumSnapshotSynchronizationResumeV1 {
    /// `replay_exchange` is mandatory only after outcome-unknown. A not-sent
    /// command is retried under its unchanged correlation.
    pub fn resume<P: SemanticSessionExchangePortV1>(
        self,
        semantic: &mut SemanticSessionClientV1<P>,
        coordinator: &mut VellumSnapshotSynchronizationCoordinatorV1,
        replay_exchange: Option<SemanticExchangeRefV1>,
    ) -> VellumSnapshotSynchronizationExecutionV1 {
        drive_plan(
            semantic,
            coordinator,
            self.plan,
            self.next,
            self.pending,
            replay_exchange,
        )
    }
}

fn drive_plan<P: SemanticSessionExchangePortV1>(
    semantic: &mut SemanticSessionClientV1<P>,
    coordinator: &mut VellumSnapshotSynchronizationCoordinatorV1,
    plan: VellumSnapshotSynchronizationPlanV1,
    mut next: usize,
    mut pending: bool,
    replay_exchange: Option<SemanticExchangeRefV1>,
) -> VellumSnapshotSynchronizationExecutionV1 {
    while next < plan.commands.len() {
        let disposition = if pending {
            let Some(exchange) = replay_exchange else {
                return VellumSnapshotSynchronizationExecutionV1::Suspended(
                    VellumSnapshotSynchronizationResumeV1 {
                        plan,
                        next,
                        pending,
                    },
                );
            };
            semantic.replay_pending(exchange)
        } else {
            semantic.attempt(plan.commands[next].clone())
        };
        match disposition {
            Err(_reason) if pending => {
                return VellumSnapshotSynchronizationExecutionV1::Suspended(
                    VellumSnapshotSynchronizationResumeV1 {
                        plan,
                        next,
                        pending: true,
                    },
                );
            }
            Err(reason) => {
                return VellumSnapshotSynchronizationExecutionV1::Refused(
                    VellumSnapshotSynchronizationExecutionRefusalV1::SessionClient(reason),
                );
            }
            Ok(SemanticExchangeDispositionV1::NotSent) => {
                return VellumSnapshotSynchronizationExecutionV1::Suspended(
                    VellumSnapshotSynchronizationResumeV1 {
                        plan,
                        next,
                        pending,
                    },
                );
            }
            Ok(SemanticExchangeDispositionV1::OutcomeUnknown) => {
                return VellumSnapshotSynchronizationExecutionV1::Suspended(
                    VellumSnapshotSynchronizationResumeV1 {
                        plan,
                        next,
                        pending: true,
                    },
                );
            }
            Ok(SemanticExchangeDispositionV1::Reply(reply)) => {
                if matches!(reply.outcome(), SemanticSessionOutcomeV1::Refused(_)) {
                    return VellumSnapshotSynchronizationExecutionV1::Refused(
                        VellumSnapshotSynchronizationExecutionRefusalV1::SemanticRefused,
                    );
                }
                next += 1;
                pending = false;
            }
        }
    }
    match coordinator.commit(plan) {
        Ok(()) => VellumSnapshotSynchronizationExecutionV1::Synchronized,
        Err(reason) => VellumSnapshotSynchronizationExecutionV1::Refused(
            VellumSnapshotSynchronizationExecutionRefusalV1::Planning(reason),
        ),
    }
}

enum SynchronizationDelta {
    Open(RetainedNativeSnapshotV1),
    Update(RetainedNativeSnapshotV1),
    Close(SemanticTargetV1),
}

pub fn semantic_document_ref(document: WorkspaceDocumentIdV1) -> SemanticDocumentRefV1 {
    let mut bytes = [0; 16];
    bytes[..8].copy_from_slice(&document.workspace().get().to_be_bytes());
    bytes[8..].copy_from_slice(&document.get().to_be_bytes());
    SemanticDocumentRefV1::new(bytes)
}

fn bump(counter: &mut u64, saturated: &mut bool) {
    let next = counter.saturating_add(1);
    *saturated |= next == u64::MAX;
    *counter = next;
}

fn add(counter: &mut u64, value: usize, saturated: &mut bool) {
    let value = value.try_into().unwrap_or(u64::MAX);
    let next = counter.saturating_add(value);
    *saturated |= next == u64::MAX;
    *counter = next;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use vellum_host::semantic::SemanticCapabilitySetV1;
    use vellum_sdk_client::protocol::{
        ClientId, CommandV1, RefusalV1, RequestV1, ServiceControlErrorV1, WorkspaceIdV1,
    };
    use vellum_semantic_protocol::{
        SemanticProviderObservationV1, SemanticProviderReadinessProjectionV1,
        SemanticSessionDescriptionV1, SemanticSessionReadinessV1, SemanticSessionReplyV1,
        SemanticSessionRolesV1,
    };

    struct NativePort {
        inventory: WorkspaceInventoryV1,
        document: WorkspaceDocumentIdV1,
        snapshot: SnapshotV1,
    }

    impl ServicePort for NativePort {
        fn connect(&mut self) -> Result<ClientId, ServiceControlErrorV1> {
            Ok(ClientId::new(1).unwrap())
        }

        fn execute(&mut self, request: RequestV1) -> Result<ReplyV1, ServiceAccessErrorV1> {
            Ok(match request.command {
                CommandV1::WorkspaceInventory => {
                    ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(self.inventory.clone()))
                }
                CommandV1::SelectDocument { document } if document == self.document => {
                    ReplyV1::Accepted(OutcomeV1::DocumentSelected(document))
                }
                CommandV1::Snapshot => {
                    ReplyV1::Accepted(OutcomeV1::Snapshot(self.snapshot.clone()))
                }
                _ => ReplyV1::Refused(RefusalV1::UnknownDocument),
            })
        }

        fn disconnect(&mut self, _client: ClientId) -> Result<(), ServiceControlErrorV1> {
            Ok(())
        }
    }

    struct AcceptSemanticPort {
        incarnation: vellum_semantic_protocol::SemanticSessionIncarnationV1,
        membership: vellum_semantic_protocol::SemanticMembershipRefV1,
    }

    impl SemanticSessionExchangePortV1 for AcceptSemanticPort {
        fn exchange(
            &mut self,
            command: &SemanticSessionCommandV1,
        ) -> SemanticExchangeDispositionV1<SemanticSessionReplyV1> {
            SemanticExchangeDispositionV1::Reply(
                SemanticSessionReplyV1::try_new(
                    command.kind(),
                    command.exchange(),
                    self.incarnation,
                    Some(self.membership),
                    command.operation(),
                    SemanticSessionOutcomeV1::Accepted,
                )
                .unwrap(),
            )
        }
    }

    struct UnknownSemanticPort;

    impl SemanticSessionExchangePortV1 for UnknownSemanticPort {
        fn exchange(
            &mut self,
            _command: &SemanticSessionCommandV1,
        ) -> SemanticExchangeDispositionV1<SemanticSessionReplyV1> {
            SemanticExchangeDispositionV1::OutcomeUnknown
        }
    }

    struct Composition {
        next: u8,
    }

    impl VellumSnapshotSynchronizationCompositionV1 for Composition {
        fn language(&mut self, _document: WorkspaceDocumentIdV1) -> Option<String> {
            Some("rust".into())
        }

        fn next_epoch(&mut self) -> Option<SemanticSynchronizationEpochV1> {
            self.next = self.next.saturating_add(1);
            SemanticSynchronizationEpochV1::new([self.next; 16])
        }

        fn next_identity(&mut self) -> Option<SemanticSynchronizationCommandIdentityV1> {
            self.next = self.next.saturating_add(1);
            Some(SemanticSynchronizationCommandIdentityV1 {
                exchange: SemanticExchangeRefV1::new([self.next; 16])?,
                operation: SemanticSessionOperationRefV1::new([self.next; 16])?,
            })
        }
    }

    fn semantic_client() -> SemanticSessionClientV1<AcceptSemanticPort> {
        let incarnation =
            vellum_semantic_protocol::SemanticSessionIncarnationV1::new([1; 16]).unwrap();
        let membership = vellum_semantic_protocol::SemanticMembershipRefV1::new([2; 16]).unwrap();
        let mut client = SemanticSessionClientV1::try_new(
            AcceptSemanticPort {
                incarnation,
                membership,
            },
            crate::SemanticSessionClientLimitsV1::reference(),
        )
        .unwrap();
        client.membership = Some(crate::MembershipProjectionV1 {
            incarnation,
            membership,
            roles: SemanticSessionRolesV1::synchronize(),
            validation: crate::SemanticReplyValidationV1::Unsealed,
            epoch: None,
            documents: BTreeMap::new(),
            witness_bytes: 0,
            staged: None,
            requests: BTreeMap::new(),
            result_reservation: 0,
            remaining_operations: 32,
            definitive_operations: BTreeSet::new(),
        });
        client.description = Some(
            SemanticSessionDescriptionV1::try_new(
                incarnation,
                SemanticSessionRolesV1::top(),
                SemanticSessionReadinessV1::Unavailable,
                SemanticProviderObservationV1::new(
                    SemanticProviderReadinessProjectionV1::Unavailable,
                    SemanticCapabilitySetV1::top(),
                ),
                SemanticSessionLimitsV1::reference(),
            )
            .unwrap(),
        );
        client
    }

    fn unknown_semantic_client() -> SemanticSessionClientV1<UnknownSemanticPort> {
        let incarnation =
            vellum_semantic_protocol::SemanticSessionIncarnationV1::new([1; 16]).unwrap();
        let membership = vellum_semantic_protocol::SemanticMembershipRefV1::new([2; 16]).unwrap();
        let mut client = SemanticSessionClientV1::try_new(
            UnknownSemanticPort,
            crate::SemanticSessionClientLimitsV1::reference(),
        )
        .unwrap();
        client.membership = Some(crate::MembershipProjectionV1 {
            incarnation,
            membership,
            roles: SemanticSessionRolesV1::synchronize(),
            validation: crate::SemanticReplyValidationV1::Unsealed,
            epoch: None,
            documents: BTreeMap::new(),
            witness_bytes: 0,
            staged: None,
            requests: BTreeMap::new(),
            result_reservation: 0,
            remaining_operations: 32,
            definitive_operations: BTreeSet::new(),
        });
        client.description = Some(
            SemanticSessionDescriptionV1::try_new(
                incarnation,
                SemanticSessionRolesV1::top(),
                SemanticSessionReadinessV1::Unavailable,
                SemanticProviderObservationV1::new(
                    SemanticProviderReadinessProjectionV1::Unavailable,
                    SemanticCapabilitySetV1::top(),
                ),
                SemanticSessionLimitsV1::reference(),
            )
            .unwrap(),
        );
        client
    }

    #[test]
    fn public_native_client_snapshot_drives_complete_semantic_synchronization() {
        let workspace = WorkspaceIdV1::new(7).unwrap();
        let document = WorkspaceDocumentIdV1::new(workspace, 9).unwrap();
        let mut native = vellum_sdk_client::client::Client::connect(NativePort {
            inventory: WorkspaceInventoryV1 {
                workspace,
                documents: vec![document],
                roots: Vec::new(),
            },
            document,
            snapshot: SnapshotV1 {
                witness: VersionWitness::new(11).unwrap(),
                revision: 3,
                text: Arc::from("fn café() {}\n"),
            },
        })
        .unwrap();
        let mut semantic = semantic_client();
        let mut coordinator = VellumSnapshotSynchronizationCoordinatorV1::default();
        let mut composition = Composition { next: 20 };

        assert!(matches!(
            synchronize_vellum_snapshots(
                &mut native,
                &mut semantic,
                &mut coordinator,
                &mut composition,
                SemanticSessionLimitsV1::reference(),
            ),
            VellumSnapshotSynchronizationExecutionV1::Synchronized
        ));
        assert!(matches!(
            synchronize_vellum_snapshots(
                &mut native,
                &mut semantic,
                &mut coordinator,
                &mut composition,
                SemanticSessionLimitsV1::reference(),
            ),
            VellumSnapshotSynchronizationExecutionV1::Synchronized
        ));
        assert_eq!(
            semantic.validation_state(),
            crate::SemanticReplyValidationV1::WitnessValidated
        );
        assert_eq!(coordinator.usage().commits, 2);
        assert_eq!(coordinator.usage().planned_commands, 5);
        assert_eq!(coordinator.usage().synchronized_documents, 1);
        assert_eq!(
            coordinator.usage().synchronized_utf8_bytes,
            u64::try_from("fn café() {}\n".len()).unwrap()
        );
    }

    #[test]
    fn stale_and_same_revision_conflict_are_refused_before_commands() {
        let workspace = WorkspaceIdV1::new(7).unwrap();
        let document = WorkspaceDocumentIdV1::new(workspace, 9).unwrap();
        let inventory = WorkspaceInventoryV1 {
            workspace,
            documents: vec![document],
            roots: Vec::new(),
        };
        let product = |revision, witness, text: &'static str| {
            VellumSemanticSnapshotProductV1::try_new(
                &inventory,
                document,
                "rust".into(),
                SnapshotV1 {
                    witness: VersionWitness::new(witness).unwrap(),
                    revision,
                    text: Arc::from(text),
                },
            )
            .unwrap()
        };
        let mut coordinator = VellumSnapshotSynchronizationCoordinatorV1::default();
        let binding = semantic_client().binding().unwrap();
        let identities: Vec<_> = (30..33)
            .map(|tag| SemanticSynchronizationCommandIdentityV1 {
                exchange: SemanticExchangeRefV1::new([tag; 16]).unwrap(),
                operation: SemanticSessionOperationRefV1::new([tag; 16]).unwrap(),
            })
            .collect();
        let plan = coordinator
            .plan(
                binding,
                SemanticSynchronizationEpochV1::new([3; 16]).unwrap(),
                &inventory,
                &[product(4, 4, "one")],
                &identities,
                SemanticSessionLimitsV1::reference(),
            )
            .unwrap();
        coordinator.commit(plan).unwrap();
        assert!(matches!(
            coordinator.plan(
                binding,
                SemanticSynchronizationEpochV1::new([4; 16]).unwrap(),
                &inventory,
                &[product(3, 3, "old")],
                &identities,
                SemanticSessionLimitsV1::reference(),
            ),
            Err(VellumSnapshotSynchronizationRefusalV1::StaleRevision)
        ));
        assert!(matches!(
            coordinator.plan(
                binding,
                SemanticSynchronizationEpochV1::new([5; 16]).unwrap(),
                &inventory,
                &[product(4, 5, "other")],
                &identities,
                SemanticSessionLimitsV1::reference(),
            ),
            Err(VellumSnapshotSynchronizationRefusalV1::RevisionConflict)
        ));
    }

    #[test]
    fn replay_local_refusal_preserves_affine_plan_owner() {
        let workspace = WorkspaceIdV1::new(7).unwrap();
        let document = WorkspaceDocumentIdV1::new(workspace, 9).unwrap();
        let mut native = vellum_sdk_client::client::Client::connect(NativePort {
            inventory: WorkspaceInventoryV1 {
                workspace,
                documents: vec![document],
                roots: Vec::new(),
            },
            document,
            snapshot: SnapshotV1 {
                witness: VersionWitness::new(11).unwrap(),
                revision: 3,
                text: Arc::from("source"),
            },
        })
        .unwrap();
        let mut semantic = unknown_semantic_client();
        let mut coordinator = VellumSnapshotSynchronizationCoordinatorV1::default();
        let mut composition = Composition { next: 20 };
        let VellumSnapshotSynchronizationExecutionV1::Suspended(owner) =
            synchronize_vellum_snapshots(
                &mut native,
                &mut semantic,
                &mut coordinator,
                &mut composition,
                SemanticSessionLimitsV1::reference(),
            )
        else {
            panic!("outcome unknown must return resume ownership");
        };
        let same_exchange = owner.plan.commands[owner.next].exchange();
        assert!(owner.pending);
        assert!(matches!(
            owner.resume(&mut semantic, &mut coordinator, Some(same_exchange)),
            VellumSnapshotSynchronizationExecutionV1::Suspended(
                VellumSnapshotSynchronizationResumeV1 { pending: true, .. }
            )
        ));
        assert!(semantic.pending_mutation().is_some());
        assert_eq!(coordinator.usage().commits, 0);
    }
}
