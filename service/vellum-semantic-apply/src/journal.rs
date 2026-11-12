//! Injected, bounded journal algebra for semantic plan handles.
//!
//! This module defines ordering and recovery semantics. It does not provide a
//! durable filesystem adapter: `InMemorySemanticPlanJournalV1` is a fault model
//! and a convenient composition/test adapter only.

use std::collections::VecDeque;

use sha2::{Digest as _, Sha256};
use vellum_host::{
    authority::{AuthoritySessionRefV1, AuthoritySubjectRefV1},
    semantic::{
        SemanticCapabilitySetV1, SemanticCapabilityV1, SemanticDocumentRefV1,
        SemanticIndexGenerationV1, SemanticLimitsV1, SemanticLocationV1,
        SemanticProviderProvenanceV1, SemanticProviderRefV1, SemanticRequestRefV1,
        SemanticResultV1, SemanticSourceRevisionV1, SemanticSpanV1, SemanticTextEditV1,
        SemanticTextV1,
    },
};

use super::*;

const MAGIC: &[u8; 4] = b"VSJP";
const VERSION: u8 = 1;
const PLAN: u8 = 1;
const EXPIRE: u8 = 2;
const PENDING: u8 = 3;
const COMPLETE: u8 = 4;

pub use vellum_host::append_journal::{
    AppendJournalCommitV1 as SemanticPlanJournalCommitV1,
    AppendJournalLimitsV1 as SemanticPlanJournalLimitsV1,
    AppendJournalPortV1 as SemanticPlanJournalPortV1,
    AppendJournalReadRefusalV1 as SemanticPlanJournalReadRefusalV1,
    MAX_APPEND_JOURNAL_BYTES_V1 as MAX_SEMANTIC_PLAN_JOURNAL_BYTES_V1,
    MAX_APPEND_JOURNAL_RECORD_BYTES_V1 as MAX_SEMANTIC_PLAN_JOURNAL_RECORD_BYTES_V1,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticPlanRecoveryRefusalV1 {
    InvalidLimits,
    Unavailable,
    Limit,
    Corrupt,
    FutureVersion,
    Conflict,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JournaledSemanticPlanIssueOutcomeV1 {
    Issued(SemanticPlanIssuedV1),
    Refused(SemanticApplyRefusalV1),
    JournalRefused,
    OutcomeUnknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JournaledSemanticPlanOperationAdmissionV1 {
    Prepared,
    Complete(SemanticApplyOutcomeV1),
    OutcomeUnknown,
    JournalRefused,
    Conflict,
    Capacity,
    AbsentOrForeign,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JournaledSemanticPlanExecutionOutcomeV1 {
    Complete(SemanticApplyOutcomeV1),
    OutcomeUnknown,
    JournalRefused,
    Conflict,
    Capacity,
    AbsentOrForeign,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JournaledSemanticPlanExpirationOutcomeV1 {
    Expired,
    AlreadyExpired,
    Refused(SemanticApplyRefusalV1),
    JournalRefused,
    OutcomeUnknown,
}

/// Durable-by-contract composition wrapper. Durability is supplied solely by
/// `J`; this crate ships no disk-backed implementation.
pub struct JournaledSemanticPlanRegistryV1<A, I, J> {
    volatile: VolatileSemanticPlanRegistryV1<A, I>,
    journal: J,
    journal_limits: SemanticPlanJournalLimitsV1,
    committed_records: usize,
    committed_bytes: usize,
    fenced: bool,
}

impl<A, I, J> JournaledSemanticPlanRegistryV1<A, I, J>
where
    A: AuthorityPortV1,
    I: SemanticPlanHandleIssuerV1,
    J: SemanticPlanJournalPortV1,
{
    pub fn try_new_empty(
        apply: SemanticApplyServiceV1<A>,
        issuer: I,
        mut journal: J,
        registry_limits: SemanticPlanRegistryLimitsV1,
        journal_limits: SemanticPlanJournalLimitsV1,
    ) -> Result<Self, SemanticPlanRecoveryRefusalV1> {
        if !registry_limits.is_valid() || !journal_limits.is_valid() {
            return Err(SemanticPlanRecoveryRefusalV1::InvalidLimits);
        }
        let records = journal
            .read_bounded(journal_limits)
            .map_err(map_read_refusal)?;
        validate_record_collection(&records, journal_limits)?;
        if !records.is_empty() {
            return Err(SemanticPlanRecoveryRefusalV1::Conflict);
        }
        Self::construct(apply, issuer, journal, registry_limits, journal_limits)
    }

    fn construct(
        apply: SemanticApplyServiceV1<A>,
        issuer: I,
        journal: J,
        registry_limits: SemanticPlanRegistryLimitsV1,
        journal_limits: SemanticPlanJournalLimitsV1,
    ) -> Result<Self, SemanticPlanRecoveryRefusalV1> {
        if !registry_limits.is_valid() || !journal_limits.is_valid() {
            return Err(SemanticPlanRecoveryRefusalV1::InvalidLimits);
        }
        let volatile = VolatileSemanticPlanRegistryV1::try_new(apply, issuer, registry_limits)
            .map_err(|_| SemanticPlanRecoveryRefusalV1::InvalidLimits)?;
        Ok(Self {
            volatile,
            journal,
            journal_limits,
            committed_records: 0,
            committed_bytes: 0,
            fenced: false,
        })
    }

    /// Reconstructs retained state without applying any workspace effect.
    /// Recovered pending operations remain outcome-unknown forever.
    pub fn try_recover(
        apply: SemanticApplyServiceV1<A>,
        issuer: I,
        journal: J,
        registry_limits: SemanticPlanRegistryLimitsV1,
        journal_limits: SemanticPlanJournalLimitsV1,
    ) -> Result<Self, SemanticPlanRecoveryRefusalV1> {
        let mut registry =
            Self::construct(apply, issuer, journal, registry_limits, journal_limits)?;
        let records = registry
            .journal
            .read_bounded(journal_limits)
            .map_err(map_read_refusal)?;
        validate_record_collection(&records, journal_limits)?;
        registry.committed_records = records.len();
        registry.committed_bytes = records.iter().map(Vec::len).sum();
        for bytes in records {
            let record = decode(&bytes, registry_limits)?;
            registry.recover_record(record)?;
        }
        registry.recompute_usage()?;
        Ok(registry)
    }

    pub const fn is_fenced(&self) -> bool {
        self.fenced
    }

    pub const fn usage(&self) -> SemanticPlanRegistryUsageV1 {
        self.volatile.usage()
    }

    pub const fn apply_usage(&self) -> SemanticApplyUsageV1 {
        self.volatile.apply_usage()
    }

    pub fn bind_snapshot(
        &mut self,
        client: SemanticClientRefV1,
        snapshot: &SemanticSnapshotV1,
        workspace: &mut Workspace,
        document: &WorkspaceDocumentId,
    ) -> Result<(), SemanticApplyRefusalV1> {
        self.volatile
            .bind_snapshot(client, snapshot, workspace, document)
    }

    pub fn unbind_snapshot(
        &mut self,
        client: SemanticClientRefV1,
        target: SemanticTargetV1,
        document: &WorkspaceDocumentId,
    ) -> Result<(), SemanticApplyRefusalV1> {
        self.volatile.unbind_snapshot(client, target, document)
    }

    pub fn issue_plan(
        &mut self,
        workspace: &mut Workspace,
        client: SemanticClientRefV1,
        authority_context: AuthorityContextV1,
        current_provider: &SemanticProviderStateV1,
        reply: &SemanticReplyV1,
    ) -> JournaledSemanticPlanIssueOutcomeV1 {
        if self.fenced {
            return JournaledSemanticPlanIssueOutcomeV1::OutcomeUnknown;
        }
        let before = self.volatile.usage;
        let issued = match self.volatile.issue_plan(
            workspace,
            client,
            authority_context,
            current_provider,
            reply,
        ) {
            Ok(value) => value,
            Err(error) => return JournaledSemanticPlanIssueOutcomeV1::Refused(error),
        };
        let retained = self
            .volatile
            .plans
            .get(&issued.handle)
            .expect("issued plan is retained");
        let bytes = encode_plan(issued.handle, retained);
        match self.commit(&bytes) {
            SemanticPlanJournalCommitV1::Committed => {
                JournaledSemanticPlanIssueOutcomeV1::Issued(issued)
            }
            SemanticPlanJournalCommitV1::Refused => {
                self.volatile.plans.remove(&issued.handle);
                self.volatile.usage = before;
                JournaledSemanticPlanIssueOutcomeV1::JournalRefused
            }
            SemanticPlanJournalCommitV1::Indeterminate => {
                self.fenced = true;
                JournaledSemanticPlanIssueOutcomeV1::OutcomeUnknown
            }
        }
    }

    pub fn expire_plan(
        &mut self,
        client: SemanticClientRefV1,
        handle: SemanticPlanHandleV1,
    ) -> JournaledSemanticPlanExpirationOutcomeV1 {
        if self.fenced {
            return JournaledSemanticPlanExpirationOutcomeV1::OutcomeUnknown;
        }
        let Some(plan) = self.volatile.plans.get(&handle) else {
            return JournaledSemanticPlanExpirationOutcomeV1::Refused(
                SemanticApplyRefusalV1::UnknownPlan,
            );
        };
        if plan.client != client {
            return JournaledSemanticPlanExpirationOutcomeV1::Refused(
                SemanticApplyRefusalV1::UnknownPlan,
            );
        }
        if plan.expired {
            return JournaledSemanticPlanExpirationOutcomeV1::AlreadyExpired;
        }
        match self.commit(&encode_expire(client, handle)) {
            SemanticPlanJournalCommitV1::Committed => {
                let result = self.volatile.expire_plan(client, handle);
                debug_assert_eq!(result, Ok(true));
                JournaledSemanticPlanExpirationOutcomeV1::Expired
            }
            SemanticPlanJournalCommitV1::Refused => {
                JournaledSemanticPlanExpirationOutcomeV1::JournalRefused
            }
            SemanticPlanJournalCommitV1::Indeterminate => {
                self.fenced = true;
                JournaledSemanticPlanExpirationOutcomeV1::OutcomeUnknown
            }
        }
    }

    pub fn prepare_operation(
        &mut self,
        client: SemanticClientRefV1,
        operation: SemanticApplyOperationRefV1,
        handle: SemanticPlanHandleV1,
    ) -> JournaledSemanticPlanOperationAdmissionV1 {
        if self.fenced {
            return JournaledSemanticPlanOperationAdmissionV1::OutcomeUnknown;
        }
        let before = self.volatile.usage;
        match self.volatile.prepare_operation(client, operation, handle) {
            VolatileSemanticPlanOperationAdmissionV1::Prepared => {}
            VolatileSemanticPlanOperationAdmissionV1::Complete(outcome) => {
                return JournaledSemanticPlanOperationAdmissionV1::Complete(outcome);
            }
            VolatileSemanticPlanOperationAdmissionV1::OutcomeUnknown => {
                return JournaledSemanticPlanOperationAdmissionV1::OutcomeUnknown;
            }
            VolatileSemanticPlanOperationAdmissionV1::Conflict => {
                return JournaledSemanticPlanOperationAdmissionV1::Conflict;
            }
            VolatileSemanticPlanOperationAdmissionV1::Capacity => {
                return JournaledSemanticPlanOperationAdmissionV1::Capacity;
            }
            VolatileSemanticPlanOperationAdmissionV1::AbsentOrForeign => {
                return JournaledSemanticPlanOperationAdmissionV1::AbsentOrForeign;
            }
        }
        match self.commit(&encode_pending(client, operation, handle)) {
            SemanticPlanJournalCommitV1::Committed => {
                JournaledSemanticPlanOperationAdmissionV1::Prepared
            }
            SemanticPlanJournalCommitV1::Refused => {
                self.volatile.operations.remove(&operation);
                self.volatile.usage = before;
                JournaledSemanticPlanOperationAdmissionV1::JournalRefused
            }
            SemanticPlanJournalCommitV1::Indeterminate => {
                self.fenced = true;
                JournaledSemanticPlanOperationAdmissionV1::OutcomeUnknown
            }
        }
    }

    pub fn execute_prepared(
        &mut self,
        workspace: &mut Workspace,
        client: SemanticClientRefV1,
        operation: SemanticApplyOperationRefV1,
        handle: SemanticPlanHandleV1,
        current_provider: &SemanticProviderStateV1,
    ) -> JournaledSemanticPlanExecutionOutcomeV1 {
        if self.fenced {
            return JournaledSemanticPlanExecutionOutcomeV1::OutcomeUnknown;
        }
        if let Some(RetainedOperation::Complete {
            client: owner,
            handle: retained_handle,
            outcome,
        }) = self.volatile.operations.get(&operation)
        {
            return if *owner == client && *retained_handle == handle {
                JournaledSemanticPlanExecutionOutcomeV1::Complete(outcome.clone())
            } else if *owner != client {
                JournaledSemanticPlanExecutionOutcomeV1::AbsentOrForeign
            } else {
                JournaledSemanticPlanExecutionOutcomeV1::Conflict
            };
        }
        let result =
            self.volatile
                .execute_prepared(workspace, client, operation, handle, current_provider);
        let outcome = match result {
            VolatileSemanticPlanExecutionOutcomeV1::Complete(value) => value,
            VolatileSemanticPlanExecutionOutcomeV1::OutcomeUnknown => {
                return JournaledSemanticPlanExecutionOutcomeV1::OutcomeUnknown;
            }
            VolatileSemanticPlanExecutionOutcomeV1::Conflict => {
                return JournaledSemanticPlanExecutionOutcomeV1::Conflict;
            }
            VolatileSemanticPlanExecutionOutcomeV1::Capacity => {
                return JournaledSemanticPlanExecutionOutcomeV1::Capacity;
            }
            VolatileSemanticPlanExecutionOutcomeV1::AbsentOrForeign => {
                return JournaledSemanticPlanExecutionOutcomeV1::AbsentOrForeign;
            }
        };
        let commit = self.commit(&encode_complete(client, operation, handle, &outcome));
        match commit {
            SemanticPlanJournalCommitV1::Committed => {
                JournaledSemanticPlanExecutionOutcomeV1::Complete(outcome)
            }
            SemanticPlanJournalCommitV1::Refused => {
                // The workspace effect may already have occurred. Retain only
                // pending evidence and never permit automatic reapplication.
                self.volatile
                    .operations
                    .insert(operation, RetainedOperation::Pending { client, handle });
                JournaledSemanticPlanExecutionOutcomeV1::OutcomeUnknown
            }
            SemanticPlanJournalCommitV1::Indeterminate => {
                self.volatile
                    .operations
                    .insert(operation, RetainedOperation::Pending { client, handle });
                self.fenced = true;
                JournaledSemanticPlanExecutionOutcomeV1::OutcomeUnknown
            }
        }
    }

    pub fn execute(
        &mut self,
        workspace: &mut Workspace,
        client: SemanticClientRefV1,
        operation: SemanticApplyOperationRefV1,
        handle: SemanticPlanHandleV1,
        current_provider: &SemanticProviderStateV1,
    ) -> JournaledSemanticPlanExecutionOutcomeV1 {
        match self.prepare_operation(client, operation, handle) {
            JournaledSemanticPlanOperationAdmissionV1::Prepared => {
                self.execute_prepared(workspace, client, operation, handle, current_provider)
            }
            JournaledSemanticPlanOperationAdmissionV1::Complete(value) => {
                JournaledSemanticPlanExecutionOutcomeV1::Complete(value)
            }
            JournaledSemanticPlanOperationAdmissionV1::OutcomeUnknown => {
                JournaledSemanticPlanExecutionOutcomeV1::OutcomeUnknown
            }
            JournaledSemanticPlanOperationAdmissionV1::JournalRefused => {
                JournaledSemanticPlanExecutionOutcomeV1::JournalRefused
            }
            JournaledSemanticPlanOperationAdmissionV1::Conflict => {
                JournaledSemanticPlanExecutionOutcomeV1::Conflict
            }
            JournaledSemanticPlanOperationAdmissionV1::Capacity => {
                JournaledSemanticPlanExecutionOutcomeV1::Capacity
            }
            JournaledSemanticPlanOperationAdmissionV1::AbsentOrForeign => {
                JournaledSemanticPlanExecutionOutcomeV1::AbsentOrForeign
            }
        }
    }

    pub fn plan_digest(
        &self,
        client: SemanticClientRefV1,
        handle: SemanticPlanHandleV1,
    ) -> Option<SemanticPlanDigestV1> {
        self.volatile.plan_digest(client, handle)
    }

    pub fn into_parts(self) -> (VolatileSemanticPlanRegistryV1<A, I>, J) {
        (self.volatile, self.journal)
    }

    fn commit(&mut self, record: &[u8]) -> SemanticPlanJournalCommitV1 {
        let Some(next_records) = self.committed_records.checked_add(1) else {
            return SemanticPlanJournalCommitV1::Refused;
        };
        let Some(next_bytes) = self.committed_bytes.checked_add(record.len()) else {
            return SemanticPlanJournalCommitV1::Refused;
        };
        if record.is_empty()
            || record.len() > self.journal_limits.max_record_bytes
            || next_records > self.journal_limits.max_records
            || next_bytes > self.journal_limits.max_total_bytes
        {
            return SemanticPlanJournalCommitV1::Refused;
        }
        let outcome = self.journal.append_synchronized(record);
        if outcome == SemanticPlanJournalCommitV1::Committed {
            self.committed_records = next_records;
            self.committed_bytes = next_bytes;
        }
        outcome
    }

    fn recover_record(&mut self, record: Record) -> Result<(), SemanticPlanRecoveryRefusalV1> {
        match record {
            Record::Plan { handle, retained } => {
                if self.volatile.plans.contains_key(&handle)
                    || !provider_matches(&retained.provider, &retained.reply)
                    || digest_plan(&retained.reply) != retained.digest
                    || !plan_targets_are_snapshot_congruent(&retained.reply)
                {
                    return Err(SemanticPlanRecoveryRefusalV1::Conflict);
                }
                self.volatile.plans.insert(handle, *retained);
            }
            Record::Expire { client, handle } => {
                let plan = self
                    .volatile
                    .plans
                    .get_mut(&handle)
                    .ok_or(SemanticPlanRecoveryRefusalV1::Conflict)?;
                if plan.client != client || plan.expired {
                    return Err(SemanticPlanRecoveryRefusalV1::Conflict);
                }
                plan.expired = true;
            }
            Record::Pending {
                client,
                operation,
                handle,
            } => {
                let plan = self
                    .volatile
                    .plans
                    .get(&handle)
                    .ok_or(SemanticPlanRecoveryRefusalV1::Conflict)?;
                if plan.client != client || self.volatile.operations.contains_key(&operation) {
                    return Err(SemanticPlanRecoveryRefusalV1::Conflict);
                }
                self.volatile
                    .operations
                    .insert(operation, RetainedOperation::Pending { client, handle });
            }
            Record::Complete {
                client,
                operation,
                handle,
                outcome,
            } => {
                let plan = self
                    .volatile
                    .plans
                    .get(&handle)
                    .ok_or(SemanticPlanRecoveryRefusalV1::Conflict)?;
                if plan.client != client || !outcome_matches_plan(&outcome, &plan.reply) {
                    return Err(SemanticPlanRecoveryRefusalV1::Conflict);
                }
                match self.volatile.operations.get_mut(&operation) {
                    Some(slot @ RetainedOperation::Pending { .. }) => {
                        match slot {
                            RetainedOperation::Pending {
                                client: owner,
                                handle: retained_handle,
                            } if *owner == client && *retained_handle == handle => {}
                            _ => return Err(SemanticPlanRecoveryRefusalV1::Conflict),
                        }
                        *slot = RetainedOperation::Complete {
                            client,
                            handle,
                            outcome,
                        };
                    }
                    _ => return Err(SemanticPlanRecoveryRefusalV1::Conflict),
                }
            }
        }
        Ok(())
    }

    fn recompute_usage(&mut self) -> Result<(), SemanticPlanRecoveryRefusalV1> {
        let mut edits = 0usize;
        let mut payload = 0usize;
        for plan in self.volatile.plans.values() {
            let rename = plan
                .reply
                .result
                .rename_plan()
                .ok_or(SemanticPlanRecoveryRefusalV1::Corrupt)?;
            if rename.len() > self.volatile.apply.limits.max_edits {
                return Err(SemanticPlanRecoveryRefusalV1::Limit);
            }
            let mut plan_payload = 0usize;
            let mut documents = BTreeMap::<SemanticDocumentRefV1, ()>::new();
            edits = edits
                .checked_add(rename.len())
                .filter(|value| *value <= self.volatile.limits.max_retained_edits)
                .ok_or(SemanticPlanRecoveryRefusalV1::Limit)?;
            for edit in rename {
                documents.insert(edit.location.target.document, ());
                plan_payload = plan_payload
                    .checked_add(edit.replacement.len())
                    .filter(|value| *value <= self.volatile.apply.limits.max_payload_bytes)
                    .ok_or(SemanticPlanRecoveryRefusalV1::Limit)?;
                payload = payload
                    .checked_add(edit.replacement.len())
                    .filter(|value| *value <= self.volatile.limits.max_retained_payload_bytes)
                    .ok_or(SemanticPlanRecoveryRefusalV1::Limit)?;
            }
            if documents.len() > self.volatile.apply.limits.max_documents {
                return Err(SemanticPlanRecoveryRefusalV1::Limit);
            }
        }
        if self.volatile.plans.len() > self.volatile.limits.max_plans
            || self.volatile.operations.len() > self.volatile.limits.max_operations
        {
            return Err(SemanticPlanRecoveryRefusalV1::Limit);
        }
        let expired = self
            .volatile
            .plans
            .values()
            .filter(|plan| plan.expired)
            .count();
        self.volatile.usage = SemanticPlanRegistryUsageV1 {
            plans: self.volatile.plans.len(),
            plans_high_water: self.volatile.plans.len(),
            plans_issued: u64::try_from(self.volatile.plans.len()).unwrap_or(u64::MAX),
            plans_expired: u64::try_from(expired).unwrap_or(u64::MAX),
            retained_edits: edits,
            retained_edits_high_water: edits,
            retained_payload_bytes: payload,
            retained_payload_bytes_high_water: payload,
            operations: self.volatile.operations.len(),
            operations_high_water: self.volatile.operations.len(),
            saturated: self.volatile.plans.len() > u64::MAX as usize,
            ..SemanticPlanRegistryUsageV1::default()
        };
        Ok(())
    }
}

fn outcome_matches_plan(outcome: &SemanticApplyOutcomeV1, reply: &SemanticReplyV1) -> bool {
    let Some(plan) = reply.result.rename_plan() else {
        return false;
    };
    let changed_ceiling = plan
        .iter()
        .map(|edit| edit.location.target.document)
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    match outcome {
        SemanticApplyOutcomeV1::Applied(receipt) => {
            receipt.persistence == SemanticPersistenceV1::NotAttempted
                && receipt.changed_documents > 0
                && receipt.changed_documents <= changed_ceiling
                && receipt.applied_edits == plan.len()
        }
        SemanticApplyOutcomeV1::Unchanged(receipt) => {
            receipt.persistence == SemanticPersistenceV1::NotAttempted
                && receipt.changed_documents == 0
                && (receipt.applied_edits == 0 || receipt.applied_edits == plan.len())
        }
        SemanticApplyOutcomeV1::Refused(_) => true,
    }
}

fn plan_targets_are_snapshot_congruent(reply: &SemanticReplyV1) -> bool {
    let Some(plan) = reply.result.rename_plan() else {
        return false;
    };
    let mut revisions = BTreeMap::<SemanticDocumentRefV1, SemanticSourceRevisionV1>::new();
    revisions.insert(reply.target.document, reply.target.source_revision);
    for edit in plan {
        match revisions.insert(
            edit.location.target.document,
            edit.location.target.source_revision,
        ) {
            Some(existing) if existing != edit.location.target.source_revision => return false,
            Some(_) | None => {}
        }
    }
    true
}

#[derive(Clone)]
enum Record {
    Plan {
        handle: SemanticPlanHandleV1,
        retained: Box<RetainedPlan>,
    },
    Expire {
        client: SemanticClientRefV1,
        handle: SemanticPlanHandleV1,
    },
    Pending {
        client: SemanticClientRefV1,
        operation: SemanticApplyOperationRefV1,
        handle: SemanticPlanHandleV1,
    },
    Complete {
        client: SemanticClientRefV1,
        operation: SemanticApplyOperationRefV1,
        handle: SemanticPlanHandleV1,
        outcome: SemanticApplyOutcomeV1,
    },
}

fn prefix(kind: u8) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(256);
    bytes.extend_from_slice(MAGIC);
    bytes.push(VERSION);
    bytes.push(kind);
    bytes
}

fn encode_plan(handle: SemanticPlanHandleV1, plan: &RetainedPlan) -> Vec<u8> {
    let mut out = prefix(PLAN);
    put_16(&mut out, handle.as_bytes());
    put_16(&mut out, plan.client.as_bytes());
    put_context(&mut out, plan.authority_context);
    put_provider(&mut out, &plan.provider);
    put_reply(&mut out, &plan.reply);
    out.extend_from_slice(&plan.digest.as_bytes());
    seal(out)
}

fn encode_expire(client: SemanticClientRefV1, handle: SemanticPlanHandleV1) -> Vec<u8> {
    let mut out = prefix(EXPIRE);
    put_16(&mut out, client.as_bytes());
    put_16(&mut out, handle.as_bytes());
    seal(out)
}

fn encode_pending(
    client: SemanticClientRefV1,
    operation: SemanticApplyOperationRefV1,
    handle: SemanticPlanHandleV1,
) -> Vec<u8> {
    let mut out = prefix(PENDING);
    put_16(&mut out, client.as_bytes());
    put_16(&mut out, operation.as_bytes());
    put_16(&mut out, handle.as_bytes());
    seal(out)
}

fn encode_complete(
    client: SemanticClientRefV1,
    operation: SemanticApplyOperationRefV1,
    handle: SemanticPlanHandleV1,
    outcome: &SemanticApplyOutcomeV1,
) -> Vec<u8> {
    let mut out = prefix(COMPLETE);
    put_16(&mut out, client.as_bytes());
    put_16(&mut out, operation.as_bytes());
    put_16(&mut out, handle.as_bytes());
    put_outcome(&mut out, outcome);
    seal(out)
}

fn seal(mut body: Vec<u8>) -> Vec<u8> {
    let mut digest = Sha256::new();
    digest.update(b"vellum-semantic-plan-journal-record-v1\0");
    digest.update(&body);
    body.extend_from_slice(&digest.finalize());
    body
}

fn decode(
    bytes: &[u8],
    registry_limits: SemanticPlanRegistryLimitsV1,
) -> Result<Record, SemanticPlanRecoveryRefusalV1> {
    if bytes.len() < 38 {
        return Err(SemanticPlanRecoveryRefusalV1::Corrupt);
    }
    let body_len = bytes.len() - 32;
    let (body, checksum) = bytes.split_at(body_len);
    let mut digest = Sha256::new();
    digest.update(b"vellum-semantic-plan-journal-record-v1\0");
    digest.update(body);
    if digest.finalize().as_slice() != checksum {
        return Err(SemanticPlanRecoveryRefusalV1::Corrupt);
    }
    let mut cursor = Cursor::new(body);
    if cursor.take(4)? != MAGIC {
        return Err(SemanticPlanRecoveryRefusalV1::Corrupt);
    }
    let version = cursor.u8()?;
    if version > VERSION {
        return Err(SemanticPlanRecoveryRefusalV1::FutureVersion);
    }
    if version != VERSION {
        return Err(SemanticPlanRecoveryRefusalV1::Corrupt);
    }
    let kind = cursor.u8()?;
    let record = match kind {
        PLAN => {
            let handle = SemanticPlanHandleV1::try_new(cursor.array16()?)
                .ok_or(SemanticPlanRecoveryRefusalV1::Corrupt)?;
            let client = SemanticClientRefV1::new(cursor.array16()?);
            let authority_context = cursor.context()?;
            let provider = cursor.provider()?;
            let reply = cursor.reply(registry_limits)?;
            let digest = SemanticPlanDigestV1(cursor.array32()?);
            Record::Plan {
                handle,
                retained: Box::new(RetainedPlan {
                    client,
                    authority_context,
                    provider,
                    reply,
                    digest,
                    expired: false,
                }),
            }
        }
        EXPIRE => Record::Expire {
            client: SemanticClientRefV1::new(cursor.array16()?),
            handle: SemanticPlanHandleV1::try_new(cursor.array16()?)
                .ok_or(SemanticPlanRecoveryRefusalV1::Corrupt)?,
        },
        PENDING => Record::Pending {
            client: SemanticClientRefV1::new(cursor.array16()?),
            operation: SemanticApplyOperationRefV1::try_new(cursor.array16()?)
                .ok_or(SemanticPlanRecoveryRefusalV1::Corrupt)?,
            handle: SemanticPlanHandleV1::try_new(cursor.array16()?)
                .ok_or(SemanticPlanRecoveryRefusalV1::Corrupt)?,
        },
        COMPLETE => Record::Complete {
            client: SemanticClientRefV1::new(cursor.array16()?),
            operation: SemanticApplyOperationRefV1::try_new(cursor.array16()?)
                .ok_or(SemanticPlanRecoveryRefusalV1::Corrupt)?,
            handle: SemanticPlanHandleV1::try_new(cursor.array16()?)
                .ok_or(SemanticPlanRecoveryRefusalV1::Corrupt)?,
            outcome: cursor.outcome()?,
        },
        _ => return Err(SemanticPlanRecoveryRefusalV1::Corrupt),
    };
    if !cursor.done() {
        return Err(SemanticPlanRecoveryRefusalV1::Corrupt);
    }
    Ok(record)
}

fn validate_record_collection(
    records: &[Vec<u8>],
    limits: SemanticPlanJournalLimitsV1,
) -> Result<(), SemanticPlanRecoveryRefusalV1> {
    if records.len() > limits.max_records {
        return Err(SemanticPlanRecoveryRefusalV1::Limit);
    }
    let mut total = 0usize;
    for record in records {
        if record.is_empty() || record.len() > limits.max_record_bytes {
            return Err(SemanticPlanRecoveryRefusalV1::Limit);
        }
        total = total
            .checked_add(record.len())
            .filter(|value| *value <= limits.max_total_bytes)
            .ok_or(SemanticPlanRecoveryRefusalV1::Limit)?;
    }
    Ok(())
}

const fn map_read_refusal(
    error: SemanticPlanJournalReadRefusalV1,
) -> SemanticPlanRecoveryRefusalV1 {
    match error {
        SemanticPlanJournalReadRefusalV1::Unavailable => SemanticPlanRecoveryRefusalV1::Unavailable,
        SemanticPlanJournalReadRefusalV1::Limit => SemanticPlanRecoveryRefusalV1::Limit,
    }
}

fn put_16(out: &mut Vec<u8>, value: [u8; 16]) {
    out.extend_from_slice(&value);
}

fn put_u32(out: &mut Vec<u8>, value: usize) {
    out.extend_from_slice(&u32::try_from(value).unwrap_or(u32::MAX).to_be_bytes());
}

fn put_string(out: &mut Vec<u8>, value: &str) {
    put_u32(out, value.len());
    out.extend_from_slice(value.as_bytes());
}

fn put_context(out: &mut Vec<u8>, context: AuthorityContextV1) {
    put_16(out, context.subject.to_bytes());
    match context.session {
        Some(session) => {
            out.push(1);
            put_16(out, session.to_bytes());
        }
        None => out.push(0),
    }
}

fn put_provenance(out: &mut Vec<u8>, value: &SemanticProviderProvenanceV1) {
    put_16(out, value.provider().as_bytes());
    put_string(out, value.profile());
    put_string(out, value.version());
}

fn put_provider(out: &mut Vec<u8>, value: &SemanticProviderStateV1) {
    put_provenance(out, &value.provider);
    match value.readiness {
        SemanticProviderReadinessV1::Unavailable => out.push(0),
        SemanticProviderReadinessV1::Indexing => out.push(1),
        SemanticProviderReadinessV1::Ready { generation } => {
            out.push(2);
            out.extend_from_slice(&generation.get().to_be_bytes());
        }
    }
    let mut capabilities = 0u8;
    if value.capabilities.contains(SemanticCapabilityV1::ReadIndex) {
        capabilities |= 1;
    }
    if value
        .capabilities
        .contains(SemanticCapabilityV1::PlanRename)
    {
        capabilities |= 2;
    }
    out.push(capabilities);
}

fn put_target(out: &mut Vec<u8>, value: SemanticTargetV1) {
    put_16(out, value.document.as_bytes());
    out.extend_from_slice(&value.source_revision.get().to_be_bytes());
}

fn put_reply(out: &mut Vec<u8>, value: &SemanticReplyV1) {
    put_16(out, value.request.as_bytes());
    put_target(out, value.target);
    out.extend_from_slice(&value.index_generation.get().to_be_bytes());
    put_provenance(out, &value.provider);
    let plan = value
        .result
        .rename_plan()
        .expect("only rename plans are retained");
    put_u32(out, plan.len());
    for edit in plan {
        put_target(out, edit.location.target);
        out.extend_from_slice(&edit.location.span.start.to_be_bytes());
        out.extend_from_slice(&edit.location.span.end.to_be_bytes());
        put_string(out, edit.replacement.as_str());
    }
}

fn put_outcome(out: &mut Vec<u8>, outcome: &SemanticApplyOutcomeV1) {
    match outcome {
        SemanticApplyOutcomeV1::Applied(receipt) => {
            out.push(0);
            put_receipt(out, receipt);
        }
        SemanticApplyOutcomeV1::Unchanged(receipt) => {
            out.push(1);
            put_receipt(out, receipt);
        }
        SemanticApplyOutcomeV1::Refused(refusal) => {
            out.push(2);
            out.push(refusal_tag(*refusal));
        }
    }
}

fn put_receipt(out: &mut Vec<u8>, receipt: &SemanticApplyReceiptV1) {
    out.extend_from_slice(
        &u64::try_from(receipt.changed_documents)
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    out.extend_from_slice(
        &u64::try_from(receipt.applied_edits)
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    out.push(0); // SemanticPersistenceV1::NotAttempted
}

fn refusal_tag(value: SemanticApplyRefusalV1) -> u8 {
    match value {
        SemanticApplyRefusalV1::InvalidLimits => 0,
        SemanticApplyRefusalV1::AuthorityDenied => 1,
        SemanticApplyRefusalV1::AuthorityUnavailable => 2,
        SemanticApplyRefusalV1::AuthorityMalformed => 3,
        SemanticApplyRefusalV1::NotRenamePlan => 4,
        SemanticApplyRefusalV1::ProviderStale => 5,
        SemanticApplyRefusalV1::UnknownBinding => 6,
        SemanticApplyRefusalV1::ClientIsolation => 7,
        SemanticApplyRefusalV1::SourceStale => 8,
        SemanticApplyRefusalV1::MalformedPlan => 9,
        SemanticApplyRefusalV1::InputLimit => 10,
        SemanticApplyRefusalV1::Capacity => 11,
        SemanticApplyRefusalV1::WorkspaceUnavailable => 12,
        SemanticApplyRefusalV1::UnknownPlan => 13,
        SemanticApplyRefusalV1::PlanExpired => 14,
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], SemanticPlanRecoveryRefusalV1> {
        let end = self
            .at
            .checked_add(count)
            .filter(|end| *end <= self.bytes.len())
            .ok_or(SemanticPlanRecoveryRefusalV1::Corrupt)?;
        let value = &self.bytes[self.at..end];
        self.at = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, SemanticPlanRecoveryRefusalV1> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, SemanticPlanRecoveryRefusalV1> {
        Ok(u32::from_be_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| SemanticPlanRecoveryRefusalV1::Corrupt)?,
        ))
    }

    fn u64(&mut self) -> Result<u64, SemanticPlanRecoveryRefusalV1> {
        Ok(u64::from_be_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| SemanticPlanRecoveryRefusalV1::Corrupt)?,
        ))
    }

    fn array16(&mut self) -> Result<[u8; 16], SemanticPlanRecoveryRefusalV1> {
        self.take(16)?
            .try_into()
            .map_err(|_| SemanticPlanRecoveryRefusalV1::Corrupt)
    }

    fn array32(&mut self) -> Result<[u8; 32], SemanticPlanRecoveryRefusalV1> {
        self.take(32)?
            .try_into()
            .map_err(|_| SemanticPlanRecoveryRefusalV1::Corrupt)
    }

    fn string(&mut self, max: usize) -> Result<String, SemanticPlanRecoveryRefusalV1> {
        let len = usize::try_from(self.u32()?).map_err(|_| SemanticPlanRecoveryRefusalV1::Limit)?;
        if len > max {
            return Err(SemanticPlanRecoveryRefusalV1::Limit);
        }
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| SemanticPlanRecoveryRefusalV1::Corrupt)
    }

    fn context(&mut self) -> Result<AuthorityContextV1, SemanticPlanRecoveryRefusalV1> {
        let subject = AuthoritySubjectRefV1::new(self.array16()?);
        let session = match self.u8()? {
            0 => None,
            1 => Some(AuthoritySessionRefV1::new(self.array16()?)),
            _ => return Err(SemanticPlanRecoveryRefusalV1::Corrupt),
        };
        Ok(AuthorityContextV1 { subject, session })
    }

    fn provenance(
        &mut self,
    ) -> Result<SemanticProviderProvenanceV1, SemanticPlanRecoveryRefusalV1> {
        SemanticProviderProvenanceV1::try_new(
            SemanticProviderRefV1::new(self.array16()?),
            self.string(vellum_host::semantic::MAX_SEMANTIC_PROVIDER_PROFILE_BYTES_V1)?,
            self.string(vellum_host::semantic::MAX_SEMANTIC_PROVIDER_VERSION_BYTES_V1)?,
        )
        .map_err(|_| SemanticPlanRecoveryRefusalV1::Corrupt)
    }

    fn provider(&mut self) -> Result<SemanticProviderStateV1, SemanticPlanRecoveryRefusalV1> {
        let provider = self.provenance()?;
        let readiness = match self.u8()? {
            0 => SemanticProviderReadinessV1::Unavailable,
            1 => SemanticProviderReadinessV1::Indexing,
            2 => SemanticProviderReadinessV1::Ready {
                generation: SemanticIndexGenerationV1::new(self.u64()?)
                    .ok_or(SemanticPlanRecoveryRefusalV1::Corrupt)?,
            },
            _ => return Err(SemanticPlanRecoveryRefusalV1::Corrupt),
        };
        let capability_bits = self.u8()?;
        if capability_bits & !0b11 != 0 {
            return Err(SemanticPlanRecoveryRefusalV1::Corrupt);
        }
        let mut capabilities = SemanticCapabilitySetV1::bottom();
        if capability_bits & 1 != 0 {
            capabilities = capabilities.join(SemanticCapabilitySetV1::singleton(
                SemanticCapabilityV1::ReadIndex,
            ));
        }
        if capability_bits & 2 != 0 {
            capabilities = capabilities.join(SemanticCapabilitySetV1::singleton(
                SemanticCapabilityV1::PlanRename,
            ));
        }
        Ok(SemanticProviderStateV1 {
            provider,
            readiness,
            capabilities,
        })
    }

    fn target(&mut self) -> Result<SemanticTargetV1, SemanticPlanRecoveryRefusalV1> {
        Ok(SemanticTargetV1 {
            document: SemanticDocumentRefV1::new(self.array16()?),
            source_revision: SemanticSourceRevisionV1::new(self.u64()?),
        })
    }

    fn reply(
        &mut self,
        registry_limits: SemanticPlanRegistryLimitsV1,
    ) -> Result<SemanticReplyV1, SemanticPlanRecoveryRefusalV1> {
        let request = SemanticRequestRefV1::new(self.array16()?);
        let target = self.target()?;
        let index_generation = SemanticIndexGenerationV1::new(self.u64()?)
            .ok_or(SemanticPlanRecoveryRefusalV1::Corrupt)?;
        let provider = self.provenance()?;
        let count =
            usize::try_from(self.u32()?).map_err(|_| SemanticPlanRecoveryRefusalV1::Limit)?;
        if count > registry_limits.max_retained_edits
            || count > SemanticLimitsV1::reference().max_rename_edits
        {
            return Err(SemanticPlanRecoveryRefusalV1::Limit);
        }
        let mut edits = Vec::with_capacity(count);
        for _ in 0..count {
            let edit_target = self.target()?;
            let span = SemanticSpanV1::try_new(self.u64()?, self.u64()?)
                .map_err(|_| SemanticPlanRecoveryRefusalV1::Corrupt)?;
            let replacement = SemanticTextV1::try_new(
                self.string(SemanticLimitsV1::reference().max_label_bytes)?,
                SemanticLimitsV1::reference().max_label_bytes,
            )
            .map_err(|_| SemanticPlanRecoveryRefusalV1::Corrupt)?;
            edits.push(SemanticTextEditV1 {
                location: SemanticLocationV1 {
                    target: edit_target,
                    span,
                },
                replacement,
            });
        }
        let result = SemanticResultV1::try_rename_plan(edits, SemanticLimitsV1::reference())
            .map_err(|_| SemanticPlanRecoveryRefusalV1::Corrupt)?;
        Ok(SemanticReplyV1 {
            request,
            target,
            index_generation,
            provider,
            result,
        })
    }

    fn outcome(&mut self) -> Result<SemanticApplyOutcomeV1, SemanticPlanRecoveryRefusalV1> {
        match self.u8()? {
            0 => Ok(SemanticApplyOutcomeV1::Applied(self.receipt()?)),
            1 => Ok(SemanticApplyOutcomeV1::Unchanged(self.receipt()?)),
            2 => Ok(SemanticApplyOutcomeV1::Refused(self.refusal()?)),
            _ => Err(SemanticPlanRecoveryRefusalV1::Corrupt),
        }
    }

    fn receipt(&mut self) -> Result<SemanticApplyReceiptV1, SemanticPlanRecoveryRefusalV1> {
        let changed_documents =
            usize::try_from(self.u64()?).map_err(|_| SemanticPlanRecoveryRefusalV1::Limit)?;
        let applied_edits =
            usize::try_from(self.u64()?).map_err(|_| SemanticPlanRecoveryRefusalV1::Limit)?;
        if self.u8()? != 0
            || changed_documents > MAX_APPLY_DOCUMENTS_V1
            || applied_edits > MAX_APPLY_EDITS_V1
        {
            return Err(SemanticPlanRecoveryRefusalV1::Corrupt);
        }
        Ok(SemanticApplyReceiptV1 {
            changed_documents,
            applied_edits,
            persistence: SemanticPersistenceV1::NotAttempted,
        })
    }

    fn refusal(&mut self) -> Result<SemanticApplyRefusalV1, SemanticPlanRecoveryRefusalV1> {
        Ok(match self.u8()? {
            0 => SemanticApplyRefusalV1::InvalidLimits,
            1 => SemanticApplyRefusalV1::AuthorityDenied,
            2 => SemanticApplyRefusalV1::AuthorityUnavailable,
            3 => SemanticApplyRefusalV1::AuthorityMalformed,
            4 => SemanticApplyRefusalV1::NotRenamePlan,
            5 => SemanticApplyRefusalV1::ProviderStale,
            6 => SemanticApplyRefusalV1::UnknownBinding,
            7 => SemanticApplyRefusalV1::ClientIsolation,
            8 => SemanticApplyRefusalV1::SourceStale,
            9 => SemanticApplyRefusalV1::MalformedPlan,
            10 => SemanticApplyRefusalV1::InputLimit,
            11 => SemanticApplyRefusalV1::Capacity,
            12 => SemanticApplyRefusalV1::WorkspaceUnavailable,
            13 => SemanticApplyRefusalV1::UnknownPlan,
            14 => SemanticApplyRefusalV1::PlanExpired,
            _ => return Err(SemanticPlanRecoveryRefusalV1::Corrupt),
        })
    }

    fn done(&self) -> bool {
        self.at == self.bytes.len()
    }
}

/// Non-durable bounded journal with explicit fault scripting. An indeterminate
/// scripted append may either retain or omit the record, modeling both crash
/// cuts without pretending to be a disk adapter.
#[derive(Clone, Debug)]
pub struct InMemorySemanticPlanJournalV1 {
    records: Vec<Vec<u8>>,
    bytes: usize,
    limits: SemanticPlanJournalLimitsV1,
    faults: VecDeque<(SemanticPlanJournalCommitV1, bool)>,
    read_refusal: Option<SemanticPlanJournalReadRefusalV1>,
}

impl InMemorySemanticPlanJournalV1 {
    pub fn try_new(
        limits: SemanticPlanJournalLimitsV1,
    ) -> Result<Self, SemanticPlanRecoveryRefusalV1> {
        if !limits.is_valid() {
            return Err(SemanticPlanRecoveryRefusalV1::InvalidLimits);
        }
        Ok(Self {
            records: Vec::new(),
            bytes: 0,
            limits,
            faults: VecDeque::new(),
            read_refusal: None,
        })
    }

    pub fn script_append(&mut self, outcome: SemanticPlanJournalCommitV1, retained: bool) {
        self.faults.push_back((outcome, retained));
    }

    pub fn refuse_reads(&mut self, refusal: SemanticPlanJournalReadRefusalV1) {
        self.read_refusal = Some(refusal);
    }

    pub fn records(&self) -> &[Vec<u8>] {
        &self.records
    }
}

impl SemanticPlanJournalPortV1 for InMemorySemanticPlanJournalV1 {
    fn read_bounded(
        &mut self,
        limits: SemanticPlanJournalLimitsV1,
    ) -> Result<Vec<Vec<u8>>, SemanticPlanJournalReadRefusalV1> {
        if let Some(refusal) = self.read_refusal {
            return Err(refusal);
        }
        if !limits.is_valid()
            || self.records.len() > limits.max_records
            || self.bytes > limits.max_total_bytes
            || self
                .records
                .iter()
                .any(|record| record.len() > limits.max_record_bytes)
        {
            return Err(SemanticPlanJournalReadRefusalV1::Limit);
        }
        Ok(self.records.clone())
    }

    fn append_synchronized(&mut self, record: &[u8]) -> SemanticPlanJournalCommitV1 {
        let scripted = self
            .faults
            .pop_front()
            .unwrap_or((SemanticPlanJournalCommitV1::Committed, true));
        let retain = match scripted.0 {
            SemanticPlanJournalCommitV1::Committed => true,
            SemanticPlanJournalCommitV1::Refused => false,
            SemanticPlanJournalCommitV1::Indeterminate => scripted.1,
        };
        if retain {
            let Some(bytes) = self.bytes.checked_add(record.len()) else {
                return SemanticPlanJournalCommitV1::Refused;
            };
            if record.is_empty()
                || record.len() > self.limits.max_record_bytes
                || self.records.len() == self.limits.max_records
                || bytes > self.limits.max_total_bytes
            {
                return SemanticPlanJournalCommitV1::Refused;
            }
            self.records.push(record.to_vec());
            self.bytes = bytes;
        }
        scripted.0
    }
}

#[cfg(test)]
mod codec_tests {
    use super::*;

    #[test]
    fn pending_codec_is_canonical_and_rejects_trailing_bytes_and_future_versions() {
        let bytes = encode_pending(
            SemanticClientRefV1::new([1; 16]),
            SemanticApplyOperationRefV1::try_new([2; 16]).unwrap(),
            SemanticPlanHandleV1::try_new([3; 16]).unwrap(),
        );
        assert_eq!(bytes.len(), 86);
        assert_eq!(
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            concat!(
                "56534a50010301010101010101010101010101010101",
                "02020202020202020202020202020202",
                "03030303030303030303030303030303",
                "5963ec7dfd9d08fb2f86c059a5a278216b1e2b7883bf300469a897da8bcb5aa7"
            )
        );
        assert!(matches!(
            decode(&bytes, SemanticPlanRegistryLimitsV1::reference()),
            Ok(Record::Pending { .. })
        ));
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(matches!(
            decode(&trailing, SemanticPlanRegistryLimitsV1::reference()),
            Err(SemanticPlanRecoveryRefusalV1::Corrupt)
        ));
        let mut future_body = bytes[..bytes.len() - 32].to_vec();
        future_body[4] = VERSION + 1;
        let future = seal(future_body);
        assert!(matches!(
            decode(&future, SemanticPlanRegistryLimitsV1::reference()),
            Err(SemanticPlanRecoveryRefusalV1::FutureVersion)
        ));
    }

    #[test]
    fn in_memory_fault_journal_distinguishes_all_three_commit_outcomes() {
        let mut journal =
            InMemorySemanticPlanJournalV1::try_new(SemanticPlanJournalLimitsV1::reference())
                .unwrap();
        journal.script_append(SemanticPlanJournalCommitV1::Refused, true);
        assert_eq!(
            journal.append_synchronized(b"not-written"),
            SemanticPlanJournalCommitV1::Refused
        );
        assert!(journal.records().is_empty());
        journal.script_append(SemanticPlanJournalCommitV1::Indeterminate, true);
        assert_eq!(
            journal.append_synchronized(b"maybe-written"),
            SemanticPlanJournalCommitV1::Indeterminate
        );
        assert_eq!(journal.records(), &[b"maybe-written".to_vec()]);
    }
}
