//! Transport-neutral uncertainty and sovereign-validation ledger for one
//! semantic session.
//!
//! This crate selects no transport, identity, grant, provider, or retry policy.
//! It performs one exchange only when explicitly requested and retains one
//! outcome-unknown mutation for explicit exact replay.
//! Provider provenance remains structurally validated by its closed host type,
//! but is not preselected here; composition must supply a separate expectation
//! when provider identity is policy. This crate cannot mint the private
//! sovereign-client seal owned by the embedded semantic client.
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use vellum_host::semantic::{
    SemanticDocumentRefV1, SemanticLocationV1, SemanticQueryV1, SemanticReplyV1,
    SemanticRequestRefV1, SemanticRequestV1, SemanticResultV1,
};
use vellum_semantic_protocol::{
    MAX_SEMANTIC_SESSION_OPERATIONS_PER_MEMBERSHIP_V1, MAX_SEMANTIC_SESSION_PENDING_V1,
    SemanticExchangeDispositionV1, SemanticExchangeRefV1, SemanticMembershipRefV1,
    SemanticProviderReadinessProjectionV1, SemanticSessionCommandKindV1, SemanticSessionCommandV1,
    SemanticSessionDescriptionV1, SemanticSessionIncarnationV1, SemanticSessionOperationRefV1,
    SemanticSessionOutcomeV1, SemanticSessionPendingV1, SemanticSessionReadinessV1,
    SemanticSessionReplyV1, SemanticSessionRolesV1, SemanticSessionSnapshotV1,
    SemanticSynchronizationDocumentSealV1, SemanticSynchronizationEpochV1,
    SemanticSynchronizationManifestV1, semantic_session_result_reservation_bytes_v1,
};

mod native_snapshot;
pub use native_snapshot::*;

pub const MAX_REPLAY_INTENT_BYTES_V1: usize = 8 * 1024 * 1024;
pub const MAX_WITNESS_BYTES_V1: usize = 32 * 1024 * 1024;
pub const MAX_RESULT_RESERVATION_V1: u64 = 16 * 1024 * 1024;
const DOCUMENT_WITNESS_FIXED_BYTES_V1: usize = 80;

pub trait SemanticSessionExchangePortV1 {
    fn exchange(
        &mut self,
        command: &SemanticSessionCommandV1,
    ) -> SemanticExchangeDispositionV1<SemanticSessionReplyV1>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticSessionClientLimitsV1 {
    pub max_replay_intent_bytes: usize,
    pub max_witness_bytes: usize,
    pub max_tracked_requests: u32,
    pub max_result_reservation: u64,
    pub max_seen_epochs: u32,
}

impl SemanticSessionClientLimitsV1 {
    pub const fn reference() -> Self {
        Self {
            max_replay_intent_bytes: MAX_REPLAY_INTENT_BYTES_V1,
            max_witness_bytes: MAX_WITNESS_BYTES_V1,
            max_tracked_requests: MAX_SEMANTIC_SESSION_PENDING_V1,
            max_result_reservation: MAX_RESULT_RESERVATION_V1,
            max_seen_epochs: MAX_SEMANTIC_SESSION_OPERATIONS_PER_MEMBERSHIP_V1,
        }
    }

    pub const fn is_valid(self) -> bool {
        self.max_replay_intent_bytes > 0
            && self.max_replay_intent_bytes <= MAX_REPLAY_INTENT_BYTES_V1
            && self.max_witness_bytes > 0
            && self.max_witness_bytes <= MAX_WITNESS_BYTES_V1
            && self.max_tracked_requests > 0
            && self.max_tracked_requests <= MAX_SEMANTIC_SESSION_PENDING_V1
            && self.max_result_reservation > 0
            && self.max_result_reservation <= MAX_RESULT_RESERVATION_V1
            && self.max_seen_epochs > 0
            && self.max_seen_epochs <= MAX_SEMANTIC_SESSION_OPERATIONS_PER_MEMBERSHIP_V1
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticSessionClientBuildErrorV1 {
    InvalidLimits,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticSessionClientRefusalV1 {
    PendingMutation,
    NoPendingMutation,
    SessionNotObserved,
    SessionBinding,
    RoleDenied,
    SynchronizationState,
    UnsealedValidation,
    RequestState,
    Capacity,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SemanticSessionClientUsageV1 {
    pub attempts: u64,
    pub explicit_replays: u64,
    pub replies: u64,
    pub not_sent: u64,
    pub outcome_unknown: u64,
    pub binding_refusals: u64,
    pub fenced_attempts: u64,
    pub capacity_refusals: u64,
    pub tracked_requests: u64,
    pub result_reservation: u64,
    pub witness_bytes: u64,
    pub replay_intent_bytes: u64,
    pub remaining_operations: u64,
    pub counter_saturated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticReplyValidationV1 {
    Unsealed,
    WitnessValidated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PendingSemanticMutationV1 {
    pub kind: SemanticSessionCommandKindV1,
    pub exchange: SemanticExchangeRefV1,
    pub operation: SemanticSessionOperationRefV1,
}

#[derive(Clone)]
struct DocumentWitnessV1 {
    seal: SemanticSynchronizationDocumentSealV1,
    language: String,
    utf8_boundaries: Vec<u8>,
}

impl DocumentWitnessV1 {
    fn try_from_snapshot(
        snapshot: &SemanticSessionSnapshotV1,
        limits: vellum_semantic_protocol::SemanticSessionLimitsV1,
    ) -> Option<Self> {
        let seal = SemanticSynchronizationDocumentSealV1::try_from_snapshot(snapshot, limits)?;
        let slots = snapshot.source().len().checked_add(1)?;
        let mut boundaries = vec![0u8; slots.checked_add(7)? / 8];
        for offset in snapshot
            .source()
            .char_indices()
            .map(|(offset, _)| offset)
            .chain(std::iter::once(snapshot.source().len()))
        {
            boundaries[offset / 8] |= 1 << (offset % 8);
        }
        Some(Self {
            seal,
            language: snapshot.language().to_owned(),
            utf8_boundaries: boundaries,
        })
    }

    fn retained_bytes(&self) -> usize {
        DOCUMENT_WITNESS_FIXED_BYTES_V1
            .saturating_add(self.utf8_boundaries.len())
            .saturating_add(self.language.len())
    }

    fn contains_span(&self, start: u64, end: u64) -> bool {
        let (Ok(start), Ok(end), Ok(source_bytes)) = (
            usize::try_from(start),
            usize::try_from(end),
            usize::try_from(self.seal.source_bytes()),
        ) else {
            return false;
        };
        start <= end
            && end <= source_bytes
            && boundary(&self.utf8_boundaries, start)
            && boundary(&self.utf8_boundaries, end)
    }
}

/// Source-free validation product exported only after an accepted
/// synchronization completion. The document seals are collision-resistant
/// commitments, not categorical source equality or authority. Debug reveals no
/// languages, documents, revisions, sizes, boundaries, or manifest digest.
#[derive(Clone)]
pub struct SemanticSynchronizationWitnessStoreV1 {
    incarnation: SemanticSessionIncarnationV1,
    epoch: SemanticSynchronizationEpochV1,
    documents: BTreeMap<SemanticDocumentRefV1, DocumentWitnessV1>,
    retained_bytes: usize,
}

impl std::fmt::Debug for SemanticSynchronizationWitnessStoreV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SemanticSynchronizationWitnessStoreV1")
            .field("incarnation", &"<redacted>")
            .field("epoch", &"<redacted>")
            .field("documents", &"<redacted>")
            .field("retained_bytes", &"<redacted>")
            .finish()
    }
}

#[derive(Clone)]
struct StagedSynchronizationV1 {
    epoch: SemanticSynchronizationEpochV1,
    manifest: SemanticSynchronizationManifestV1,
    documents: BTreeMap<SemanticDocumentRefV1, DocumentWitnessV1>,
    retained_bytes: usize,
    source_bytes: u64,
}

#[derive(Clone)]
struct MembershipProjectionV1 {
    incarnation: SemanticSessionIncarnationV1,
    membership: SemanticMembershipRefV1,
    roles: SemanticSessionRolesV1,
    validation: SemanticReplyValidationV1,
    epoch: Option<SemanticSynchronizationEpochV1>,
    documents: BTreeMap<SemanticDocumentRefV1, DocumentWitnessV1>,
    witness_bytes: usize,
    staged: Option<StagedSynchronizationV1>,
    requests: BTreeMap<SemanticRequestRefV1, SemanticRequestV1>,
    result_reservation: u64,
    remaining_operations: u32,
    definitive_operations: BTreeSet<SemanticSessionOperationRefV1>,
}

pub struct SemanticSessionClientV1<P> {
    port: P,
    limits: SemanticSessionClientLimitsV1,
    description: Option<SemanticSessionDescriptionV1>,
    membership: Option<MembershipProjectionV1>,
    pending: Option<SemanticSessionCommandV1>,
    pending_bytes: usize,
    seen_epochs: BTreeSet<SemanticSynchronizationEpochV1>,
    usage: SemanticSessionClientUsageV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticSessionClientBindingV1 {
    pub incarnation: SemanticSessionIncarnationV1,
    pub membership: SemanticMembershipRefV1,
    pub roles: SemanticSessionRolesV1,
}

impl<P> SemanticSessionClientV1<P> {
    pub fn try_new(
        port: P,
        limits: SemanticSessionClientLimitsV1,
    ) -> Result<Self, SemanticSessionClientBuildErrorV1> {
        if !limits.is_valid() {
            return Err(SemanticSessionClientBuildErrorV1::InvalidLimits);
        }
        Ok(Self {
            port,
            limits,
            description: None,
            membership: None,
            pending: None,
            pending_bytes: 0,
            seen_epochs: BTreeSet::new(),
            usage: SemanticSessionClientUsageV1::default(),
        })
    }

    pub const fn usage(&self) -> SemanticSessionClientUsageV1 {
        self.usage
    }

    pub fn validation_state(&self) -> SemanticReplyValidationV1 {
        self.membership
            .as_ref()
            .map_or(SemanticReplyValidationV1::Unsealed, |state| {
                state.validation
            })
    }

    /// Content-safe session observation needed by trusted composition adapters.
    pub const fn description(&self) -> Option<SemanticSessionDescriptionV1> {
        self.description
    }

    /// Exact active binding. Correlation values remain neither credentials nor
    /// authority; the granted role product is included so adapters cannot infer
    /// query authority from membership possession.
    pub fn binding(&self) -> Option<SemanticSessionClientBindingV1> {
        self.membership
            .as_ref()
            .map(|state| SemanticSessionClientBindingV1 {
                incarnation: state.incarnation,
                membership: state.membership,
                roles: state.roles,
            })
    }

    pub fn pending_mutation(&self) -> Option<PendingSemanticMutationV1> {
        let command = self.pending.as_ref()?;
        Some(PendingSemanticMutationV1 {
            kind: command.kind(),
            exchange: command.exchange(),
            operation: command
                .operation()
                .expect("only mutations enter the uncertainty ledger"),
        })
    }

    pub fn export_witness_store(&self) -> Option<SemanticSynchronizationWitnessStoreV1> {
        let state = self.membership.as_ref()?;
        (state.validation == SemanticReplyValidationV1::WitnessValidated).then(|| {
            SemanticSynchronizationWitnessStoreV1 {
                incarnation: state.incarnation,
                epoch: state.epoch.expect("sealed projection has epoch"),
                documents: state.documents.clone(),
                retained_bytes: state.witness_bytes,
            }
        })
    }

    /// Trusted composition may share an accepted synchronization projection
    /// with a query-only membership. Canonical decoding alone cannot mint it.
    pub fn install_witness_store(
        &mut self,
        store: SemanticSynchronizationWitnessStoreV1,
    ) -> Result<(), SemanticSessionClientRefusalV1> {
        if !self.seen_epochs.contains(&store.epoch)
            && self.seen_epochs.len() >= self.limits.max_seen_epochs as usize
        {
            return Err(SemanticSessionClientRefusalV1::Capacity);
        }
        let state = self
            .membership
            .as_mut()
            .ok_or(SemanticSessionClientRefusalV1::SessionBinding)?;
        if self.pending.is_some()
            || state.incarnation != store.incarnation
            || store.retained_bytes > self.limits.max_witness_bytes
            || !state.roles.contains(SemanticSessionRolesV1::query())
            || !matches!(
                self.description.map(SemanticSessionDescriptionV1::readiness),
                Some(SemanticSessionReadinessV1::Ready { epoch }) if epoch == store.epoch
            )
            || !matches!(
                self.description.map(SemanticSessionDescriptionV1::provider),
                Some(provider)
                    if matches!(
                        provider.readiness(),
                        SemanticProviderReadinessProjectionV1::Ready { .. }
                    )
            )
        {
            return Err(SemanticSessionClientRefusalV1::SessionBinding);
        }
        state.documents = store.documents;
        state.witness_bytes = store.retained_bytes;
        state.epoch = Some(store.epoch);
        self.seen_epochs.insert(store.epoch);
        state.validation = SemanticReplyValidationV1::WitnessValidated;
        self.update_gauges();
        Ok(())
    }

    pub fn try_into_port(self) -> Result<P, Box<Self>> {
        if self.pending.is_some() {
            Err(Box::new(self))
        } else {
            Ok(self.port)
        }
    }

    fn update_gauges(&mut self) {
        self.usage.replay_intent_bytes = u64::try_from(self.pending_bytes).unwrap_or(u64::MAX);
        let (tracked, reservation, witnesses) =
            self.membership.as_ref().map_or((0, 0, 0), |state| {
                (
                    u64::try_from(state.requests.len()).unwrap_or(u64::MAX),
                    state.result_reservation,
                    u64::try_from(
                        state.witness_bytes.saturating_add(
                            state
                                .staged
                                .as_ref()
                                .map_or(0, |stage| stage.retained_bytes),
                        ),
                    )
                    .unwrap_or(u64::MAX),
                )
            });
        self.usage.tracked_requests = tracked;
        self.usage.result_reservation = reservation;
        self.usage.witness_bytes = witnesses;
        self.usage.remaining_operations = self
            .membership
            .as_ref()
            .map_or(0, |state| u64::from(state.remaining_operations));
    }
}

impl<P: SemanticSessionExchangePortV1> SemanticSessionClientV1<P> {
    pub fn attempt(
        &mut self,
        command: SemanticSessionCommandV1,
    ) -> Result<SemanticExchangeDispositionV1<SemanticSessionReplyV1>, SemanticSessionClientRefusalV1>
    {
        if self.pending.is_some() && command.operation().is_some() {
            bump(
                &mut self.usage.fenced_attempts,
                &mut self.usage.counter_saturated,
            );
            return Err(SemanticSessionClientRefusalV1::PendingMutation);
        }
        self.preflight(&command)?;
        self.exchange_once(command, false)
    }

    pub fn replay_pending(
        &mut self,
        exchange: SemanticExchangeRefV1,
    ) -> Result<SemanticExchangeDispositionV1<SemanticSessionReplyV1>, SemanticSessionClientRefusalV1>
    {
        let retained = self
            .pending
            .clone()
            .ok_or(SemanticSessionClientRefusalV1::NoPendingMutation)?;
        if retained.exchange() == exchange {
            return Err(SemanticSessionClientRefusalV1::RequestState);
        }
        let command = command_with_exchange(retained, exchange);
        self.preflight(&command)?;
        self.exchange_once(command, true)
    }

    fn exchange_once(
        &mut self,
        command: SemanticSessionCommandV1,
        replay: bool,
    ) -> Result<SemanticExchangeDispositionV1<SemanticSessionReplyV1>, SemanticSessionClientRefusalV1>
    {
        bump(&mut self.usage.attempts, &mut self.usage.counter_saturated);
        if replay {
            bump(
                &mut self.usage.explicit_replays,
                &mut self.usage.counter_saturated,
            );
        }
        match self.port.exchange(&command) {
            SemanticExchangeDispositionV1::NotSent => {
                bump(&mut self.usage.not_sent, &mut self.usage.counter_saturated);
                Ok(SemanticExchangeDispositionV1::NotSent)
            }
            SemanticExchangeDispositionV1::OutcomeUnknown => {
                if replay {
                    self.pending_bytes = command
                        .retained_intent_bytes_v1()
                        .expect("validated mutation has checked logical size");
                    self.pending = Some(command.clone());
                    self.update_gauges();
                } else {
                    self.retain_uncertainty(&command);
                }
                bump(
                    &mut self.usage.outcome_unknown,
                    &mut self.usage.counter_saturated,
                );
                Ok(SemanticExchangeDispositionV1::OutcomeUnknown)
            }
            SemanticExchangeDispositionV1::Reply(reply) => {
                if !self.reply_matches(&command, &reply) {
                    if replay {
                        self.pending_bytes = command
                            .retained_intent_bytes_v1()
                            .expect("validated mutation has checked logical size");
                        self.pending = Some(command.clone());
                        self.update_gauges();
                    } else {
                        self.retain_uncertainty(&command);
                    }
                    bump(
                        &mut self.usage.binding_refusals,
                        &mut self.usage.counter_saturated,
                    );
                    bump(
                        &mut self.usage.outcome_unknown,
                        &mut self.usage.counter_saturated,
                    );
                    return Ok(SemanticExchangeDispositionV1::OutcomeUnknown);
                }
                let incarnation_changed = matches!(
                    (&command, reply.outcome(), self.description),
                    (
                        SemanticSessionCommandV1::Describe { .. },
                        SemanticSessionOutcomeV1::Description(observed),
                        Some(prior)
                    ) if prior.incarnation() != observed.incarnation()
                );
                self.consume_definitive_operation(&command, &reply);
                self.apply_reply(&command, &reply)?;
                if incarnation_changed || replay {
                    self.pending = None;
                    self.pending_bytes = 0;
                }
                self.update_gauges();
                bump(&mut self.usage.replies, &mut self.usage.counter_saturated);
                Ok(SemanticExchangeDispositionV1::Reply(reply))
            }
        }
    }

    fn retain_uncertainty(&mut self, command: &SemanticSessionCommandV1) {
        if command.operation().is_some() && self.pending.is_none() {
            self.pending_bytes = command
                .retained_intent_bytes_v1()
                .expect("validated mutation has checked logical size");
            self.pending = Some(command.clone());
            self.update_gauges();
        }
    }

    fn preflight(
        &mut self,
        command: &SemanticSessionCommandV1,
    ) -> Result<(), SemanticSessionClientRefusalV1> {
        if command.operation().is_some() {
            let Some(bytes) = command.retained_intent_bytes_v1() else {
                return self.capacity_refusal();
            };
            if bytes > self.limits.max_replay_intent_bytes {
                return self.capacity_refusal();
            }
        }
        match command {
            SemanticSessionCommandV1::Describe { .. } => Ok(()),
            SemanticSessionCommandV1::JoinSession { incarnation, .. } => {
                let description = self
                    .description
                    .ok_or(SemanticSessionClientRefusalV1::SessionNotObserved)?;
                if self.membership.is_some() || description.incarnation() != *incarnation {
                    return self.binding_refusal();
                }
                Ok(())
            }
            _ => {
                let (incarnation, membership) = command
                    .binding()
                    .ok_or(SemanticSessionClientRefusalV1::SessionBinding)?;
                let state = self
                    .membership
                    .as_ref()
                    .ok_or(SemanticSessionClientRefusalV1::SessionBinding)?;
                if state.incarnation != incarnation || state.membership != membership {
                    return self.binding_refusal();
                }
                if !state.roles.contains(command.required_roles()) {
                    return Err(SemanticSessionClientRefusalV1::RoleDenied);
                }
                self.preflight_bound(command)
            }
        }
    }

    fn preflight_bound(
        &mut self,
        command: &SemanticSessionCommandV1,
    ) -> Result<(), SemanticSessionClientRefusalV1> {
        let state = self.membership.as_ref().expect("binding checked");
        if let Some(operation) = command.operation()
            && !state.definitive_operations.contains(&operation)
            && state.remaining_operations == 0
        {
            return Err(SemanticSessionClientRefusalV1::Capacity);
        }
        match command {
            SemanticSessionCommandV1::BeginSynchronization {
                epoch, manifest, ..
            } => {
                let protocol_limits = self.protocol_limits();
                let aggregate_fits = state
                    .witness_bytes
                    .checked_mul(2)
                    .is_some_and(|bytes| bytes <= self.limits.max_witness_bytes);
                if state.staged.is_none()
                    && !self.seen_epochs.contains(epoch)
                    && self.seen_epochs.len() < self.limits.max_seen_epochs as usize
                    && aggregate_fits
                    && manifest.documents() <= protocol_limits.max_documents
                    && manifest.source_bytes() <= protocol_limits.max_total_snapshot_bytes as u64
                {
                    Ok(())
                } else {
                    Err(SemanticSessionClientRefusalV1::SynchronizationState)
                }
            }
            SemanticSessionCommandV1::SyncOpen {
                epoch, snapshot, ..
            } => {
                let staged = exact_stage(state, *epoch)?;
                if staged.documents.contains_key(&snapshot.target().document) {
                    return Err(SemanticSessionClientRefusalV1::SynchronizationState);
                }
                self.preflight_protocol_projection(staged, snapshot, None, true)?;
                self.preflight_witness(state.witness_bytes, staged, snapshot, None)
            }
            SemanticSessionCommandV1::SyncChange {
                epoch, snapshot, ..
            } => {
                let staged = exact_stage(state, *epoch)?;
                let previous = staged
                    .documents
                    .get(&snapshot.target().document)
                    .ok_or(SemanticSessionClientRefusalV1::SynchronizationState)?;
                if snapshot.target().document != previous.seal.document()
                    || snapshot.target().source_revision <= previous.seal.target().source_revision
                {
                    return Err(SemanticSessionClientRefusalV1::SynchronizationState);
                }
                self.preflight_protocol_projection(staged, snapshot, Some(previous), false)?;
                self.preflight_witness(state.witness_bytes, staged, snapshot, Some(previous))
            }
            SemanticSessionCommandV1::SyncClose { epoch, target, .. } => {
                let staged = exact_stage(state, *epoch)?;
                let limits = self.protocol_limits();
                if staged.documents.len() > limits.max_documents as usize
                    || staged.source_bytes > limits.max_total_snapshot_bytes as u64
                {
                    return Err(SemanticSessionClientRefusalV1::Capacity);
                }
                staged
                    .documents
                    .get(&target.document)
                    .is_some_and(|witness| witness.seal.target() == *target)
                    .then_some(())
                    .ok_or(SemanticSessionClientRefusalV1::SynchronizationState)
            }
            SemanticSessionCommandV1::CompleteSynchronization {
                epoch, manifest, ..
            } => {
                let staged = exact_stage(state, *epoch)?;
                let seals: Vec<_> = staged.documents.values().map(|value| value.seal).collect();
                let protocol_limits = self.protocol_limits();
                if staged.manifest == *manifest
                    && SemanticSynchronizationManifestV1::try_from_seals(&seals, protocol_limits)
                        == Some(*manifest)
                {
                    Ok(())
                } else {
                    Err(SemanticSessionClientRefusalV1::SynchronizationState)
                }
            }
            SemanticSessionCommandV1::Submit { request, .. } => {
                if state.validation != SemanticReplyValidationV1::WitnessValidated {
                    return Err(SemanticSessionClientRefusalV1::UnsealedValidation);
                }
                if !self.readiness_allows_queries(state) {
                    return Err(SemanticSessionClientRefusalV1::UnsealedValidation);
                }
                if !request.is_within_limits(self.protocol_limits().semantic)
                    || !state
                        .documents
                        .get(&request.target.document)
                        .is_some_and(|witness| {
                            witness.seal.target() == request.target
                                && witness.language == request.language.as_str()
                                && query_offset_valid(&request.query, witness)
                        })
                {
                    return Err(SemanticSessionClientRefusalV1::RequestState);
                }
                if state
                    .requests
                    .get(&request.request)
                    .is_some_and(|retained| retained != request)
                {
                    return Err(SemanticSessionClientRefusalV1::RequestState);
                }
                let Some(reservation) = semantic_session_result_reservation_bytes_v1(
                    request.query.kind(),
                    self.protocol_limits().semantic,
                )
                .and_then(|bytes| u64::try_from(bytes).ok()) else {
                    return self.capacity_refusal();
                };
                if (!state.requests.contains_key(&request.request)
                    && state.requests.len() >= self.limits.max_tracked_requests as usize)
                    || (!state.requests.contains_key(&request.request)
                        && state
                            .result_reservation
                            .checked_add(reservation)
                            .is_none_or(|total| total > self.limits.max_result_reservation))
                {
                    return self.capacity_refusal();
                }
                Ok(())
            }
            SemanticSessionCommandV1::Cancel { request, .. }
            | SemanticSessionCommandV1::Collect { request, .. }
            | SemanticSessionCommandV1::ReleaseRequest { request, .. } => state
                .requests
                .contains_key(request)
                .then_some(())
                .ok_or(SemanticSessionClientRefusalV1::RequestState),
            SemanticSessionCommandV1::LeaveSession { .. } => Ok(()),
            SemanticSessionCommandV1::Describe { .. }
            | SemanticSessionCommandV1::JoinSession { .. } => {
                Err(SemanticSessionClientRefusalV1::SessionBinding)
            }
        }
    }

    fn preflight_witness(
        &self,
        committed_bytes: usize,
        staged: &StagedSynchronizationV1,
        snapshot: &SemanticSessionSnapshotV1,
        previous: Option<&DocumentWitnessV1>,
    ) -> Result<(), SemanticSessionClientRefusalV1> {
        let witness = DocumentWitnessV1::try_from_snapshot(snapshot, self.protocol_limits())
            .ok_or(SemanticSessionClientRefusalV1::SynchronizationState)?;
        let retained = staged
            .retained_bytes
            .saturating_sub(previous.map_or(0, DocumentWitnessV1::retained_bytes))
            .checked_add(witness.retained_bytes())
            .ok_or(SemanticSessionClientRefusalV1::Capacity)?;
        if committed_bytes
            .checked_add(retained)
            .is_none_or(|aggregate| aggregate > self.limits.max_witness_bytes)
        {
            Err(SemanticSessionClientRefusalV1::Capacity)
        } else {
            Ok(())
        }
    }

    fn preflight_protocol_projection(
        &self,
        staged: &StagedSynchronizationV1,
        snapshot: &SemanticSessionSnapshotV1,
        previous: Option<&DocumentWitnessV1>,
        adds_document: bool,
    ) -> Result<(), SemanticSessionClientRefusalV1> {
        let limits = self.protocol_limits();
        let documents = staged
            .documents
            .len()
            .checked_add(usize::from(adds_document))
            .ok_or(SemanticSessionClientRefusalV1::Capacity)?;
        let source_bytes = staged
            .source_bytes
            .checked_sub(previous.map_or(0, |value| value.seal.source_bytes()))
            .and_then(|bytes| bytes.checked_add(u64::try_from(snapshot.source().len()).ok()?))
            .ok_or(SemanticSessionClientRefusalV1::Capacity)?;
        if documents > limits.max_documents as usize
            || source_bytes > limits.max_total_snapshot_bytes as u64
        {
            Err(SemanticSessionClientRefusalV1::Capacity)
        } else {
            Ok(())
        }
    }

    fn reply_matches(
        &self,
        command: &SemanticSessionCommandV1,
        reply: &SemanticSessionReplyV1,
    ) -> bool {
        if reply.kind() != command.kind()
            || reply.exchange() != command.exchange()
            || reply.operation() != command.operation()
        {
            return false;
        }
        match command {
            SemanticSessionCommandV1::Describe { .. } => {}
            SemanticSessionCommandV1::JoinSession { incarnation, .. } => {
                if reply.incarnation() != *incarnation {
                    return false;
                }
            }
            _ => {
                let Some((incarnation, membership)) = command.binding() else {
                    return false;
                };
                if reply.incarnation() != incarnation || reply.membership() != Some(membership) {
                    return false;
                }
            }
        }
        match (command, reply.outcome()) {
            (
                SemanticSessionCommandV1::Describe { .. },
                SemanticSessionOutcomeV1::Description(observed),
            ) => {
                let prior = self.description;
                if prior.is_some_and(|value| value.incarnation() == observed.incarnation())
                    && let SemanticSessionReadinessV1::Ready { epoch } = observed.readiness()
                {
                    let continuously_ready = matches!(
                        prior.map(SemanticSessionDescriptionV1::readiness),
                        Some(SemanticSessionReadinessV1::Ready { epoch: prior_epoch })
                            if prior_epoch == epoch
                    );
                    let locally_completed = self.membership.as_ref().is_some_and(|state| {
                        state.validation == SemanticReplyValidationV1::WitnessValidated
                            && state.epoch == Some(epoch)
                    });
                    if self.seen_epochs.contains(&epoch)
                        && !continuously_ready
                        && !locally_completed
                    {
                        return false;
                    }
                    if !self.seen_epochs.contains(&epoch)
                        && self.seen_epochs.len() >= self.limits.max_seen_epochs as usize
                    {
                        return false;
                    }
                }
                true
            }
            (
                SemanticSessionCommandV1::JoinSession {
                    requested_roles, ..
                },
                SemanticSessionOutcomeV1::Joined {
                    granted_roles,
                    remaining_operations,
                    ..
                },
            ) => self.description.is_some_and(|description| {
                granted_roles.bits() != 0
                    && requested_roles.contains(*granted_roles)
                    && description.role_ceiling().contains(*granted_roles)
                    && *remaining_operations > 0
                    && *remaining_operations <= description.limits().max_operations_per_membership
            }),
            (
                SemanticSessionCommandV1::Submit { request, .. },
                SemanticSessionOutcomeV1::Unsupported(kind),
            ) => *kind == request.query.kind(),
            (
                SemanticSessionCommandV1::Collect { request, .. },
                SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Request(observed)),
            ) => observed == request,
            (
                SemanticSessionCommandV1::Collect { request, .. },
                SemanticSessionOutcomeV1::Complete(result),
            ) => self
                .membership
                .as_ref()
                .and_then(|state| {
                    state
                        .requests
                        .get(request)
                        .map(|expected| (state, expected))
                })
                .is_some_and(|(state, expected)| {
                    state.validation == SemanticReplyValidationV1::WitnessValidated
                        && self.readiness_allows_queries(state)
                        && complete_reply_valid(
                            result,
                            expected,
                            &state.documents,
                            self.protocol_limits().semantic,
                        )
                }),
            (
                SemanticSessionCommandV1::Collect { request, .. },
                SemanticSessionOutcomeV1::Unsupported(kind),
            ) => self
                .membership
                .as_ref()
                .and_then(|state| state.requests.get(request))
                .is_some_and(|expected| *kind == expected.query.kind()),
            _ => true,
        }
    }

    fn apply_reply(
        &mut self,
        command: &SemanticSessionCommandV1,
        reply: &SemanticSessionReplyV1,
    ) -> Result<(), SemanticSessionClientRefusalV1> {
        match (command, reply.outcome()) {
            (
                SemanticSessionCommandV1::Describe { .. },
                SemanticSessionOutcomeV1::Description(description),
            ) => {
                if self
                    .description
                    .is_some_and(|prior| prior.incarnation() != description.incarnation())
                {
                    self.membership = None;
                    self.seen_epochs.clear();
                } else if let Some(state) = self.membership.as_mut() {
                    let ready_epoch = match description.readiness() {
                        SemanticSessionReadinessV1::Ready { epoch } => Some(epoch),
                        _ => None,
                    };
                    let provider_ready = matches!(
                        description.provider().readiness(),
                        SemanticProviderReadinessProjectionV1::Ready { .. }
                    );
                    if ready_epoch != state.epoch || !provider_ready {
                        state.validation = SemanticReplyValidationV1::Unsealed;
                        if !matches!(
                            description.readiness(),
                            SemanticSessionReadinessV1::Synchronizing { epoch }
                                if state.staged.as_ref().is_some_and(|value| value.epoch == epoch)
                        ) {
                            state.staged = None;
                        }
                    }
                }
                if let SemanticSessionReadinessV1::Ready { epoch } = description.readiness() {
                    self.seen_epochs.insert(epoch);
                }
                self.description = Some(*description);
            }
            (
                SemanticSessionCommandV1::JoinSession { .. },
                SemanticSessionOutcomeV1::Joined {
                    membership,
                    granted_roles,
                    remaining_operations,
                },
            ) => {
                self.membership = Some(MembershipProjectionV1 {
                    incarnation: reply.incarnation(),
                    membership: *membership,
                    roles: *granted_roles,
                    validation: SemanticReplyValidationV1::Unsealed,
                    epoch: None,
                    documents: BTreeMap::new(),
                    witness_bytes: 0,
                    staged: None,
                    requests: BTreeMap::new(),
                    result_reservation: 0,
                    remaining_operations: *remaining_operations,
                    definitive_operations: BTreeSet::new(),
                });
            }
            (
                SemanticSessionCommandV1::BeginSynchronization {
                    epoch, manifest, ..
                },
                SemanticSessionOutcomeV1::Accepted,
            ) => {
                assert!(
                    self.seen_epochs.insert(*epoch),
                    "accepted synchronization epoch was preflighted as fresh"
                );
                let state = self.membership.as_mut().expect("binding checked");
                let (documents, retained_bytes) =
                    if state.validation == SemanticReplyValidationV1::WitnessValidated {
                        (state.documents.clone(), state.witness_bytes)
                    } else {
                        (BTreeMap::new(), 0)
                    };
                let source_bytes = documents
                    .values()
                    .try_fold(0u64, |total, witness| {
                        total.checked_add(witness.seal.source_bytes())
                    })
                    .expect("committed witness bytes were previously bounded");
                state.validation = SemanticReplyValidationV1::Unsealed;
                state.staged = Some(StagedSynchronizationV1 {
                    epoch: *epoch,
                    manifest: *manifest,
                    documents,
                    retained_bytes,
                    source_bytes,
                });
            }
            (
                SemanticSessionCommandV1::SyncOpen {
                    epoch, snapshot, ..
                }
                | SemanticSessionCommandV1::SyncChange {
                    epoch, snapshot, ..
                },
                SemanticSessionOutcomeV1::Accepted,
            ) => {
                let witness =
                    DocumentWitnessV1::try_from_snapshot(snapshot, self.protocol_limits())
                        .ok_or(SemanticSessionClientRefusalV1::SynchronizationState)?;
                let staged =
                    exact_stage_mut(self.membership.as_mut().expect("binding checked"), *epoch)?;
                if let Some(previous) = staged
                    .documents
                    .insert(snapshot.target().document, witness.clone())
                {
                    staged.retained_bytes = staged
                        .retained_bytes
                        .checked_sub(previous.retained_bytes())
                        .expect("staged witness accounting invariant");
                    staged.source_bytes = staged
                        .source_bytes
                        .checked_sub(previous.seal.source_bytes())
                        .expect("staged source accounting invariant");
                }
                staged.retained_bytes = staged
                    .retained_bytes
                    .checked_add(witness.retained_bytes())
                    .expect("witness capacity was preflighted");
                staged.source_bytes = staged
                    .source_bytes
                    .checked_add(witness.seal.source_bytes())
                    .expect("source capacity was preflighted");
            }
            (
                SemanticSessionCommandV1::SyncClose { epoch, target, .. },
                SemanticSessionOutcomeV1::Accepted,
            ) => {
                let staged =
                    exact_stage_mut(self.membership.as_mut().expect("binding checked"), *epoch)?;
                if let Some(removed) = staged.documents.remove(&target.document) {
                    staged.retained_bytes = staged
                        .retained_bytes
                        .checked_sub(removed.retained_bytes())
                        .expect("staged witness accounting invariant");
                    staged.source_bytes = staged
                        .source_bytes
                        .checked_sub(removed.seal.source_bytes())
                        .expect("staged source accounting invariant");
                }
            }
            (
                SemanticSessionCommandV1::CompleteSynchronization { epoch, .. },
                SemanticSessionOutcomeV1::Accepted,
            ) => {
                let state = self.membership.as_mut().expect("binding checked");
                let staged = state
                    .staged
                    .take()
                    .filter(|value| value.epoch == *epoch)
                    .ok_or(SemanticSessionClientRefusalV1::SynchronizationState)?;
                state.documents = staged.documents;
                state.witness_bytes = staged.retained_bytes;
                state.epoch = Some(*epoch);
                state.validation = SemanticReplyValidationV1::WitnessValidated;
            }
            (
                SemanticSessionCommandV1::Submit { request, .. },
                SemanticSessionOutcomeV1::Accepted,
            ) => {
                let reservation = semantic_session_result_reservation_bytes_v1(
                    request.query.kind(),
                    self.protocol_limits().semantic,
                )
                .and_then(|bytes| u64::try_from(bytes).ok())
                .ok_or(SemanticSessionClientRefusalV1::Capacity)?;
                let state = self.membership.as_mut().expect("binding checked");
                if state
                    .requests
                    .insert(request.request, request.clone())
                    .is_none()
                {
                    state.result_reservation = state
                        .result_reservation
                        .checked_add(reservation)
                        .expect("result reservation was preflighted");
                }
            }
            (
                SemanticSessionCommandV1::ReleaseRequest { request, .. },
                SemanticSessionOutcomeV1::Accepted,
            ) => {
                let retained = self
                    .membership
                    .as_ref()
                    .and_then(|state| state.requests.get(request))
                    .ok_or(SemanticSessionClientRefusalV1::RequestState)?;
                let reservation = semantic_session_result_reservation_bytes_v1(
                    retained.query.kind(),
                    self.protocol_limits().semantic,
                )
                .and_then(|bytes| u64::try_from(bytes).ok())
                .ok_or(SemanticSessionClientRefusalV1::Capacity)?;
                let state = self.membership.as_mut().expect("binding checked");
                if state.requests.remove(request).is_some() {
                    state.result_reservation = state
                        .result_reservation
                        .checked_sub(reservation)
                        .expect("tracked request reservation invariant");
                }
            }
            (SemanticSessionCommandV1::LeaveSession { .. }, SemanticSessionOutcomeV1::Accepted) => {
                self.membership = None
            }
            (
                SemanticSessionCommandV1::BeginSynchronization { .. }
                | SemanticSessionCommandV1::SyncOpen { .. }
                | SemanticSessionCommandV1::SyncChange { .. }
                | SemanticSessionCommandV1::SyncClose { .. }
                | SemanticSessionCommandV1::CompleteSynchronization { .. },
                _,
            ) => {
                let state = self.membership.as_mut().expect("binding checked");
                if state.staged.is_some() {
                    state.staged = None;
                    state.validation = SemanticReplyValidationV1::Unsealed;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn protocol_limits(&self) -> vellum_semantic_protocol::SemanticSessionLimitsV1 {
        self.description
            .expect("bound session follows description")
            .limits()
    }

    fn readiness_allows_queries(&self, state: &MembershipProjectionV1) -> bool {
        self.description.is_some_and(|description| {
            matches!(
                description.readiness(),
                SemanticSessionReadinessV1::Ready { epoch } if Some(epoch) == state.epoch
            ) && matches!(
                description.provider().readiness(),
                SemanticProviderReadinessProjectionV1::Ready { .. }
            )
        })
    }

    fn consume_definitive_operation(
        &mut self,
        command: &SemanticSessionCommandV1,
        reply: &SemanticSessionReplyV1,
    ) {
        if matches!(command, SemanticSessionCommandV1::JoinSession { .. }) {
            return;
        }
        let Some(operation) = command.operation() else {
            return;
        };
        let Some(state) = self.membership.as_mut() else {
            return;
        };
        if state.definitive_operations.insert(operation) {
            state.remaining_operations =
                if matches!(reply.outcome(), SemanticSessionOutcomeV1::ReplayCapacity) {
                    0
                } else {
                    state
                        .remaining_operations
                        .checked_sub(1)
                        .expect("fresh definitive operation was preflighted")
                };
        }
    }

    fn binding_refusal<T>(&mut self) -> Result<T, SemanticSessionClientRefusalV1> {
        bump(
            &mut self.usage.binding_refusals,
            &mut self.usage.counter_saturated,
        );
        Err(SemanticSessionClientRefusalV1::SessionBinding)
    }

    fn capacity_refusal<T>(&mut self) -> Result<T, SemanticSessionClientRefusalV1> {
        bump(
            &mut self.usage.capacity_refusals,
            &mut self.usage.counter_saturated,
        );
        Err(SemanticSessionClientRefusalV1::Capacity)
    }
}

fn exact_stage(
    state: &MembershipProjectionV1,
    epoch: SemanticSynchronizationEpochV1,
) -> Result<&StagedSynchronizationV1, SemanticSessionClientRefusalV1> {
    state
        .staged
        .as_ref()
        .filter(|value| value.epoch == epoch)
        .ok_or(SemanticSessionClientRefusalV1::SynchronizationState)
}

fn exact_stage_mut(
    state: &mut MembershipProjectionV1,
    epoch: SemanticSynchronizationEpochV1,
) -> Result<&mut StagedSynchronizationV1, SemanticSessionClientRefusalV1> {
    state
        .staged
        .as_mut()
        .filter(|value| value.epoch == epoch)
        .ok_or(SemanticSessionClientRefusalV1::SynchronizationState)
}

fn complete_reply_valid(
    reply: &SemanticReplyV1,
    request: &SemanticRequestV1,
    documents: &BTreeMap<SemanticDocumentRefV1, DocumentWitnessV1>,
    limits: vellum_host::semantic::SemanticLimitsV1,
) -> bool {
    reply.request == request.request
        && reply.target == request.target
        && request
            .expected_index
            .is_none_or(|expected| expected == reply.index_generation)
        && reply.result.matches_query(request.query.kind())
        && reply.result.is_within_limits(limits)
        && result_locations_valid(&reply.result, request, documents)
}

fn result_locations_valid(
    result: &SemanticResultV1,
    request: &SemanticRequestV1,
    documents: &BTreeMap<SemanticDocumentRefV1, DocumentWitnessV1>,
) -> bool {
    let valid = |location: &SemanticLocationV1| {
        documents
            .get(&location.target.document)
            .is_some_and(|witness| {
                witness.seal.target() == location.target
                    && witness.contains_span(location.span.start, location.span.end)
            })
    };
    let individually_valid = result
        .symbols()
        .is_none_or(|values| values.iter().all(|value| valid(&value.location)))
        && result
            .locations()
            .is_none_or(|values| values.iter().all(&valid))
        && result
            .diagnostics()
            .is_none_or(|values| values.iter().all(|value| valid(&value.location)))
        && result
            .rename_plan()
            .is_none_or(|values| values.iter().all(|value| valid(&value.location)));
    let diagnostics_local = result.diagnostics().is_none_or(|values| {
        values
            .iter()
            .all(|value| value.location.target == request.target)
    });
    let rename_nonoverlapping = result.rename_plan().is_none_or(|values| {
        let mut locations: Vec<_> = values.iter().map(|value| value.location).collect();
        locations.sort_by_key(|location| {
            (
                location.target.document,
                location.span.start,
                location.span.end,
            )
        });
        locations.windows(2).all(|pair| {
            pair[0].target.document != pair[1].target.document
                || pair[0].span.end <= pair[1].span.start
        })
    });
    individually_valid && diagnostics_local && rename_nonoverlapping
}

fn query_offset_valid(query: &SemanticQueryV1, witness: &DocumentWitnessV1) -> bool {
    match query {
        SemanticQueryV1::Definition { offset }
        | SemanticQueryV1::References { offset }
        | SemanticQueryV1::Implementations { offset }
        | SemanticQueryV1::TypeAtPoint { offset }
        | SemanticQueryV1::PlanRename { offset, .. } => witness.contains_span(*offset, *offset),
        SemanticQueryV1::Symbols { .. } | SemanticQueryV1::Diagnostics => true,
    }
}

fn command_with_exchange(
    mut command: SemanticSessionCommandV1,
    fresh: SemanticExchangeRefV1,
) -> SemanticSessionCommandV1 {
    let exchange = match &mut command {
        SemanticSessionCommandV1::Describe { exchange }
        | SemanticSessionCommandV1::JoinSession { exchange, .. }
        | SemanticSessionCommandV1::BeginSynchronization { exchange, .. }
        | SemanticSessionCommandV1::SyncOpen { exchange, .. }
        | SemanticSessionCommandV1::SyncChange { exchange, .. }
        | SemanticSessionCommandV1::SyncClose { exchange, .. }
        | SemanticSessionCommandV1::CompleteSynchronization { exchange, .. }
        | SemanticSessionCommandV1::Submit { exchange, .. }
        | SemanticSessionCommandV1::Cancel { exchange, .. }
        | SemanticSessionCommandV1::Collect { exchange, .. }
        | SemanticSessionCommandV1::ReleaseRequest { exchange, .. }
        | SemanticSessionCommandV1::LeaveSession { exchange, .. } => exchange,
    };
    *exchange = fresh;
    command
}

fn boundary(bitmap: &[u8], offset: usize) -> bool {
    bitmap
        .get(offset / 8)
        .is_some_and(|byte| byte & (1 << (offset % 8)) != 0)
}

fn bump(value: &mut u64, saturated: &mut bool) {
    let Some(next) = value.checked_add(1) else {
        *value = u64::MAX;
        *saturated = true;
        return;
    };
    *value = next;
}

#[cfg(test)]
mod tests;
