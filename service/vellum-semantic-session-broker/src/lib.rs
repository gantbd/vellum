//! Pure serialized semantic-session broker reducer.
//!
//! Authentication and provider effects are composition inputs. The reducer
//! owns one sovereign semantic partition, multiplexes attenuated memberships,
//! retains exact logical mutation intent, and emits affine typed effects. A
//! trusted `DriveRetirement` event only reduces memberships already fenced by
//! policy/session loss; it creates no exchange, operation, reply or authority.
//! Observations are fixed-cardinality and content-free. Compact witnesses retain
//! language/target/source-length/UTF-8-boundary metadata, never source bytes.
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use vellum_host::authority::{AuthoritySessionRefV1, AuthoritySubjectRefV1};
use vellum_host::semantic::{
    SemanticCapabilitySetV1, SemanticIndexGenerationV1, SemanticInvalidationCauseV1,
    SemanticInvalidationReceiptV1, SemanticOutcomeV1, SemanticProviderProvenanceV1,
    SemanticRequestRefV1, SemanticRequestV1, SemanticTargetV1,
};
use vellum_semantic_protocol::{
    SemanticExchangeRefV1, SemanticMembershipRefV1, SemanticProviderObservationV1,
    SemanticProviderReadinessProjectionV1, SemanticSessionCommandKindV1, SemanticSessionCommandV1,
    SemanticSessionDescriptionV1, SemanticSessionIncarnationV1, SemanticSessionLimitsV1,
    SemanticSessionOperationRefV1, SemanticSessionOutcomeV1, SemanticSessionPendingV1,
    SemanticSessionReadinessV1, SemanticSessionRefusalV1, SemanticSessionReplyV1,
    SemanticSessionRolesV1, SemanticSessionSnapshotV1, SemanticSynchronizationDocumentSealV1,
    SemanticSynchronizationEpochV1, SemanticSynchronizationManifestV1,
    semantic_session_result_reservation_bytes_v1,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticBrokerContextV1 {
    pub subject: AuthoritySubjectRefV1,
    pub session: AuthoritySessionRefV1,
    pub allowed_roles: SemanticSessionRolesV1,
    pub policy_epoch: u64,
}

#[derive(Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticBrokerAdmissionContinuationV1(u64);
#[derive(Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticBrokerSubmissionContinuationV1(u64);
#[derive(Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticBrokerCancellationContinuationV1(u64);
macro_rules! redacted_continuation_debug {
    ($type:ty, $name:literal) => {
        impl fmt::Debug for $type {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str($name)
            }
        }
    };
}
redacted_continuation_debug!(
    SemanticBrokerAdmissionContinuationV1,
    "SemanticBrokerAdmissionContinuationV1(<opaque>)"
);
redacted_continuation_debug!(
    SemanticBrokerSubmissionContinuationV1,
    "SemanticBrokerSubmissionContinuationV1(<opaque>)"
);
redacted_continuation_debug!(
    SemanticBrokerCancellationContinuationV1,
    "SemanticBrokerCancellationContinuationV1(<opaque>)"
);

#[derive(Debug, Eq, PartialEq)]
pub enum SemanticBrokerEffectV1 {
    Open {
        continuation: SemanticBrokerAdmissionContinuationV1,
        snapshot: SemanticSessionSnapshotV1,
    },
    Change {
        continuation: SemanticBrokerAdmissionContinuationV1,
        snapshot: SemanticSessionSnapshotV1,
    },
    Close {
        continuation: SemanticBrokerAdmissionContinuationV1,
        target: SemanticTargetV1,
    },
    Submit {
        continuation: SemanticBrokerSubmissionContinuationV1,
        request: SemanticRequestV1,
    },
    Cancel {
        continuation: SemanticBrokerCancellationContinuationV1,
        request: SemanticRequestRefV1,
    },
}

#[derive(Eq, PartialEq)]
pub enum SemanticBrokerEventV1 {
    OpenCompleted {
        continuation: SemanticBrokerAdmissionContinuationV1,
        report: SemanticBrokerOpenReportV1,
    },
    ChangeCompleted {
        continuation: SemanticBrokerAdmissionContinuationV1,
        report: SemanticBrokerSourceReportV1,
    },
    CloseCompleted {
        continuation: SemanticBrokerAdmissionContinuationV1,
        report: SemanticBrokerSourceReportV1,
    },
    SubmissionCompleted {
        continuation: SemanticBrokerSubmissionContinuationV1,
        report: SemanticBrokerSubmissionReportV1,
    },
    CancellationCompleted {
        continuation: SemanticBrokerCancellationContinuationV1,
        report: SemanticBrokerCancellationReportV1,
    },
    ProviderDriveCompleted {
        report: Box<SemanticBrokerProviderDriveReportV1>,
    },
    ProviderDriveFailed {
        report: Box<SemanticBrokerProviderDriveFailureReportV1>,
    },
    ExpiryDriveCompleted {
        report: Box<SemanticBrokerExpiryReportV1>,
    },
    PolicyLost {
        subject: AuthoritySubjectRefV1,
        session: AuthoritySessionRefV1,
        policy_epoch: u64,
    },
    /// Trusted composition drive for cleanup after authority/session loss.
    /// It can only reduce already-retiring state and mints no peer authority.
    DriveRetirement,
}

impl fmt::Debug for SemanticBrokerEventV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::OpenCompleted { .. } => "OpenCompleted",
            Self::ChangeCompleted { .. } => "ChangeCompleted",
            Self::CloseCompleted { .. } => "CloseCompleted",
            Self::SubmissionCompleted { .. } => "SubmissionCompleted",
            Self::CancellationCompleted { .. } => "CancellationCompleted",
            Self::ProviderDriveCompleted { .. } => "ProviderDriveCompleted",
            Self::ProviderDriveFailed { .. } => "ProviderDriveFailed",
            Self::ExpiryDriveCompleted { .. } => "ExpiryDriveCompleted",
            Self::PolicyLost { .. } => "PolicyLost",
            Self::DriveRetirement => "DriveRetirement",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticBrokerAdmissionOutcomeV1 {
    Accepted,
    Unavailable,
    Refused(SemanticSessionRefusalV1),
}

#[must_use = "provider transition evidence must be reconciled"]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticBrokerProviderTransitionV1 {
    pub epoch: u64,
    pub observed_observation: SemanticProviderObservationV1,
    pub observed_provenance: Option<SemanticProviderProvenanceV1>,
    pub settled_observation: SemanticProviderObservationV1,
    pub settled_provenance: Option<SemanticProviderProvenanceV1>,
    pub invalidations: SemanticInvalidationReceiptV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticBrokerProviderBindingV1 {
    pub epoch: u64,
    pub observation: SemanticProviderObservationV1,
    pub provenance: Option<SemanticProviderProvenanceV1>,
}

#[must_use = "open transition evidence must be reconciled"]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticBrokerOpenReportV1 {
    pub provider_before: SemanticBrokerProviderBindingV1,
    pub pre_provider: Option<SemanticBrokerProviderTransitionV1>,
    pub outcome: SemanticBrokerAdmissionOutcomeV1,
    pub post_provider: Option<SemanticBrokerProviderTransitionV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticBrokerSourceAdmissionOutcomeV1 {
    Accepted(SemanticInvalidationReceiptV1),
    Unavailable,
    Refused(SemanticSessionRefusalV1),
}

#[must_use = "source transition evidence must be reconciled"]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticBrokerSourceReportV1 {
    pub provider_before: SemanticBrokerProviderBindingV1,
    pub pre_provider: Option<SemanticBrokerProviderTransitionV1>,
    pub outcome: SemanticBrokerSourceAdmissionOutcomeV1,
    pub post_provider: Option<SemanticBrokerProviderTransitionV1>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticBrokerSubmissionOutcomeV1 {
    Accepted,
    Unsupported(vellum_host::semantic::SemanticQueryKindV1),
    Stale,
    Unavailable,
    Refused(SemanticSessionRefusalV1),
}

#[must_use = "submission transition evidence must be reconciled"]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticBrokerSubmissionReportV1 {
    pub provider_before: SemanticBrokerProviderBindingV1,
    pub pre_provider: Option<SemanticBrokerProviderTransitionV1>,
    pub outcome: SemanticBrokerSubmissionOutcomeV1,
    pub post_provider: Option<SemanticBrokerProviderTransitionV1>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticBrokerCancellationOutcomeV1 {
    Accepted,
    Refused(SemanticSessionRefusalV1),
}

#[must_use = "cancellation transition evidence must be reconciled"]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticBrokerCancellationReportV1 {
    pub provider_before: SemanticBrokerProviderBindingV1,
    pub pre_provider: Option<SemanticBrokerProviderTransitionV1>,
    pub outcome: SemanticBrokerCancellationOutcomeV1,
    pub post_provider: Option<SemanticBrokerProviderTransitionV1>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticBrokerDeliveryOutcomeV1 {
    Peer(SemanticOutcomeV1),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticBrokerExpiryOutcomeV1 {
    Expired,
    SatisfiedByProviderTransition,
    NotPending,
}

/// One due scheduler activation and every provider transition observed around
/// its exact service expiry operation. Request correlation remains opaque and
/// Debug exposes no identity.
#[must_use = "expiry evidence must be reconciled atomically"]
#[derive(Clone, Eq, PartialEq)]
pub struct SemanticBrokerExpiryReportV1 {
    pub provider_before: SemanticBrokerProviderBindingV1,
    pub pre_provider: Option<SemanticBrokerProviderTransitionV1>,
    pub internal_request: SemanticRequestRefV1,
    pub outcome: SemanticBrokerExpiryOutcomeV1,
    pub post_provider: Option<SemanticBrokerProviderTransitionV1>,
}

impl fmt::Debug for SemanticBrokerExpiryReportV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticBrokerExpiryReportV1")
            .field("pre_provider", &self.pre_provider.is_some())
            .field("outcome", &self.outcome)
            .field("post_provider", &self.post_provider.is_some())
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub enum SemanticBrokerExpiryDriveV1 {
    NoDue,
    Due(Box<SemanticBrokerExpiryReportV1>),
}

impl fmt::Debug for SemanticBrokerExpiryDriveV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NoDue => "NoDue",
            Self::Due(_) => "Due",
        })
    }
}

/// Exact disposition of the one completion position in a successful provider
/// drive. Request identities remain opaque correlation, never authority.
#[derive(Clone, Eq, PartialEq)]
pub enum SemanticBrokerProviderDriveDispositionV1 {
    NoEvent,
    Delivery {
        internal_request: SemanticRequestRefV1,
        outcome: SemanticBrokerDeliveryOutcomeV1,
    },
    Late {
        internal_request: SemanticRequestRefV1,
    },
    Orphan {
        internal_request: SemanticRequestRefV1,
    },
}

impl fmt::Debug for SemanticBrokerProviderDriveDispositionV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NoEvent => "NoEvent",
            Self::Delivery { .. } => "Delivery",
            Self::Late { .. } => "Late",
            Self::Orphan { .. } => "Orphan",
        })
    }
}

/// One successful physical provider poll and every composition transition it
/// observed. The broker validates this complete product before mutating live
/// request/provider state.
#[must_use = "provider drive evidence must be reconciled atomically"]
#[derive(Clone, Eq, PartialEq)]
pub struct SemanticBrokerProviderDriveReportV1 {
    pub provider_before: SemanticBrokerProviderBindingV1,
    pub pre_provider: Option<SemanticBrokerProviderTransitionV1>,
    pub disposition: SemanticBrokerProviderDriveDispositionV1,
    pub post_provider: Option<SemanticBrokerProviderTransitionV1>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticBrokerProviderDriveFailureClassV1 {
    Poisoned,
    RuntimeUnavailable,
    DeliveryRefused,
}

/// A failed physical poll may still have observed and terminalized one exact
/// pre-poll provider transition. Failure has neither disposition nor post
/// observation, but its pre-transition evidence must still reach the broker.
#[must_use = "provider failure evidence must be reconciled before actor failure"]
#[derive(Clone, Eq, PartialEq)]
pub struct SemanticBrokerProviderDriveFailureReportV1 {
    pub provider_before: SemanticBrokerProviderBindingV1,
    pub pre_provider: Option<SemanticBrokerProviderTransitionV1>,
    pub failure: SemanticBrokerProviderDriveFailureClassV1,
}

impl fmt::Debug for SemanticBrokerProviderDriveFailureReportV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticBrokerProviderDriveFailureReportV1")
            .field("pre_provider", &self.pre_provider.is_some())
            .field("failure", &self.failure)
            .finish()
    }
}

impl fmt::Debug for SemanticBrokerProviderDriveReportV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticBrokerProviderDriveReportV1")
            .field("pre_provider", &self.pre_provider.is_some())
            .field("disposition", &self.disposition)
            .field("post_provider", &self.post_provider.is_some())
            .finish()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum SemanticBrokerDecisionV1 {
    Reply(SemanticSessionReplyV1),
    Effect(SemanticBrokerEffectV1),
    AwaitingExisting,
    EventAccepted,
    Quarantined,
    Diverged(SemanticBrokerDivergenceV1),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticBrokerDivergenceV1 {
    TransitionEvidence,
    TransitionOrdering,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SemanticBrokerUsageV1 {
    pub commands: u64,
    pub commands_by_kind: [u64; 12],
    pub replies_by_outcome: [u64; 14],
    pub refusals_by_reason: [u64; 11],
    pub effects: u64,
    pub synchronization_effects: u64,
    pub cleanup_effects: u64,
    pub cleanup_refusals: u64,
    pub replay_hits: u64,
    pub conflicts: u64,
    pub quarantined: u64,
    pub transition_divergences: u64,
    pub transition_ordering_divergences: u64,
    pub deliveries_routed: u64,
    pub deliveries_orphaned: u64,
    pub deliveries_malformed: u64,
    pub provider_drive_failures: u64,
    pub provider_drive_failures_by_class: [u64; 3],
    pub expiry_reports: u64,
    pub requests_expired: u64,
    pub expiries_satisfied_by_transition: u64,
    pub expiries_not_pending: u64,
    pub memberships_joined: u64,
    pub membership_high_water: u32,
    pub requests_issued: u64,
    pub current_operation_entries: u32,
    pub operation_entry_high_water: u32,
    pub request_high_water: u32,
    pub replay_intent_bytes: usize,
    pub replay_intent_bytes_high_water: usize,
    pub retained_result_bytes: usize,
    pub retained_result_bytes_high_water: usize,
    pub current_memberships: u32,
    pub retiring_memberships: u32,
    pub current_requests: u32,
    pub submitting_requests: u32,
    pub pending_requests: u32,
    pub terminal_requests: u32,
    pub released_requests: u32,
    pub authoritative_documents: u32,
    pub cleanup_documents: u32,
    pub compact_witness_bytes: usize,
    pub compact_witness_bytes_high_water: usize,
    pub pending_effects: u32,
    pub saturated: bool,
}

pub struct SemanticSessionBrokerV1 {
    incarnation: SemanticSessionIncarnationV1,
    limits: SemanticSessionLimitsV1,
    memberships: BTreeMap<SemanticMembershipRefV1, Membership>,
    joins: BTreeMap<
        (
            AuthoritySubjectRefV1,
            AuthoritySessionRefV1,
            u64,
            SemanticSessionOperationRefV1,
        ),
        ReplayRecord,
    >,
    seen_epochs: BTreeSet<SemanticSynchronizationEpochV1>,
    pending: BTreeMap<u64, PendingEffect>,
    reverse_requests:
        BTreeMap<SemanticRequestRefV1, (SemanticMembershipRefV1, SemanticRequestRefV1)>,
    seals: BTreeMap<
        vellum_host::semantic::SemanticDocumentRefV1,
        SemanticSynchronizationDocumentSealV1,
    >,
    witnesses: BTreeMap<vellum_host::semantic::SemanticDocumentRefV1, DocumentWitness>,
    cleanup_seals: BTreeMap<
        vellum_host::semantic::SemanticDocumentRefV1,
        SemanticSynchronizationDocumentSealV1,
    >,
    synchronization: SynchronizationState,
    next_identity: u128,
    next_request: u128,
    next_continuation: u64,
    usage: SemanticBrokerUsageV1,
    provider: ProviderState,
    divergence: Option<SemanticBrokerDivergenceV1>,
}

impl SemanticSessionBrokerV1 {
    pub fn try_new(
        incarnation: SemanticSessionIncarnationV1,
        limits: SemanticSessionLimitsV1,
    ) -> Option<Self> {
        Self::try_new_with_provider_binding(
            incarnation,
            limits,
            SemanticBrokerProviderBindingV1 {
                epoch: 0,
                observation: SemanticProviderObservationV1::new(
                    SemanticProviderReadinessProjectionV1::Unavailable,
                    SemanticCapabilitySetV1::bottom(),
                ),
                provenance: None,
            },
        )
    }

    /// Constructs the broker and its provider projection from one genesis
    /// binding owned by the surrounding composition. Epoch zero names the
    /// initial observation; later observed provider changes alone advance it.
    pub fn try_new_with_provider_binding(
        incarnation: SemanticSessionIncarnationV1,
        limits: SemanticSessionLimitsV1,
        provider: SemanticBrokerProviderBindingV1,
    ) -> Option<Self> {
        if !limits.is_valid()
            || provider.epoch != 0
            || !provider_state_is_valid(provider.observation, provider.provenance.as_ref())
        {
            return None;
        }
        Some(Self {
            incarnation,
            limits,
            memberships: BTreeMap::new(),
            joins: BTreeMap::new(),
            seen_epochs: BTreeSet::new(),
            pending: BTreeMap::new(),
            reverse_requests: BTreeMap::new(),
            seals: BTreeMap::new(),
            witnesses: BTreeMap::new(),
            cleanup_seals: BTreeMap::new(),
            synchronization: SynchronizationState::Unavailable,
            next_identity: 1,
            next_request: 1,
            next_continuation: 1,
            usage: SemanticBrokerUsageV1::default(),
            provider: ProviderState {
                epoch: provider.epoch,
                observation: provider.observation,
                provenance: provider.provenance,
            },
            divergence: None,
        })
    }
    pub fn usage(&self) -> SemanticBrokerUsageV1 {
        let mut usage = self.usage;
        usage.retiring_memberships = usize_to_u32(
            self.memberships
                .values()
                .filter(|member| member.retiring && !member.retired)
                .count(),
        );
        usage.submitting_requests = usize_to_u32(
            self.request_state_count(|state| matches!(state, RequestState::Submitting)),
        );
        usage.pending_requests =
            usize_to_u32(self.request_state_count(|state| matches!(state, RequestState::Pending)));
        usage.terminal_requests = usize_to_u32(
            self.request_state_count(|state| matches!(state, RequestState::Terminal(_))),
        );
        usage.released_requests =
            usize_to_u32(self.request_state_count(|state| matches!(state, RequestState::Released)));
        usage.authoritative_documents = usize_to_u32(self.seals.len());
        usage.cleanup_documents = usize_to_u32(self.cleanup_seals.len());
        usage.pending_effects = usize_to_u32(self.pending.len());
        usage.current_operation_entries = usize_to_u32(
            self.joins.len()
                + self
                    .memberships
                    .values()
                    .map(|member| member.operations.len())
                    .sum::<usize>(),
        );
        usage.compact_witness_bytes = self.compact_witness_bytes();
        usage
    }
    pub const fn incarnation(&self) -> SemanticSessionIncarnationV1 {
        self.incarnation
    }
    pub const fn limits(&self) -> SemanticSessionLimitsV1 {
        self.limits
    }
    pub fn membership_count(&self) -> usize {
        self.memberships.len()
    }

    fn request_state_count(&self, predicate: impl Fn(&RequestState) -> bool) -> usize {
        self.memberships
            .values()
            .flat_map(|member| member.requests.values())
            .filter(|request| predicate(&request.state))
            .count()
    }

    fn has_incompatible_effect_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    fn capture_transition_expectation(
        &self,
        source: ExpectedSourceEffect,
    ) -> TransitionExpectation {
        let mut requests = Vec::new();
        for member in self.memberships.values() {
            for record in member.requests.values() {
                if matches!(record.state, RequestState::Pending) {
                    requests.push(ExpectedRequest {
                        internal: record.internal,
                        provider_epoch: record.provider_epoch,
                        source_affected: source.affects(record),
                    });
                }
            }
        }
        requests.sort_by_key(|request| request.internal);
        TransitionExpectation {
            provider: self.provider.clone(),
            requests,
            source,
        }
    }

    fn compact_witness_bytes(&self) -> usize {
        self.witnesses.values().fold(0usize, |total, witness| {
            total
                .saturating_add(16 + 8 + 8 + 32)
                .saturating_add(witness.language.len())
                .saturating_add(witness.boundaries.len())
        })
    }

    pub fn command(
        &mut self,
        context: SemanticBrokerContextV1,
        command: SemanticSessionCommandV1,
    ) -> SemanticBrokerDecisionV1 {
        if let Some(divergence) = self.divergence {
            return SemanticBrokerDecisionV1::Diverged(divergence);
        }
        bump(&mut self.usage.commands, &mut self.usage.saturated);
        let command_index = command.kind() as usize - 1;
        bump(
            &mut self.usage.commands_by_kind[command_index],
            &mut self.usage.saturated,
        );
        match command.clone() {
            SemanticSessionCommandV1::Describe { exchange } => {
                let readiness = match self.synchronization {
                    SynchronizationState::Unavailable => SemanticSessionReadinessV1::Unavailable,
                    SynchronizationState::Cleaning { owner } => {
                        let _ = owner;
                        SemanticSessionReadinessV1::Unavailable
                    }
                    SynchronizationState::Active { epoch, .. } => {
                        SemanticSessionReadinessV1::Synchronizing { epoch }
                    }
                    SynchronizationState::Ready { owner, epoch } => {
                        let _ = owner;
                        SemanticSessionReadinessV1::Ready { epoch }
                    }
                };
                let description = SemanticSessionDescriptionV1::try_new(
                    self.incarnation,
                    context.allowed_roles,
                    readiness,
                    self.provider.observation,
                    self.limits,
                )
                .expect("valid broker limits");
                self.make_reply(
                    command.kind(),
                    exchange,
                    self.incarnation,
                    None,
                    None,
                    SemanticSessionOutcomeV1::Description(description),
                )
            }
            SemanticSessionCommandV1::JoinSession { operation, .. } => {
                self.join(context, command, operation)
            }
            _ => self.bound(context, command),
        }
    }

    pub fn event(&mut self, event: SemanticBrokerEventV1) -> SemanticBrokerDecisionV1 {
        if let Some(divergence) = self.divergence {
            return SemanticBrokerDecisionV1::Diverged(divergence);
        }
        if !self.pending.is_empty() && !self.event_matches_pending(&event) {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionOrdering);
        }
        match event {
            SemanticBrokerEventV1::OpenCompleted {
                continuation,
                report,
            } => self.complete_open(continuation, report),
            SemanticBrokerEventV1::ChangeCompleted {
                continuation,
                report,
            } => self.complete_source(
                continuation,
                report,
                SemanticInvalidationCauseV1::SourceChanged,
            ),
            SemanticBrokerEventV1::CloseCompleted {
                continuation,
                report,
            } => self.complete_source(
                continuation,
                report,
                SemanticInvalidationCauseV1::SourceClosed,
            ),
            SemanticBrokerEventV1::SubmissionCompleted {
                continuation,
                report,
            } => self.complete_submission(continuation, report),
            SemanticBrokerEventV1::CancellationCompleted {
                continuation,
                report,
            } => self.complete_cancellation(continuation, report),
            SemanticBrokerEventV1::PolicyLost {
                subject,
                session,
                policy_epoch,
            } => {
                for membership in self.memberships.values_mut() {
                    if membership.context.subject == subject
                        && membership.context.session == session
                        && membership.context.policy_epoch <= policy_epoch
                    {
                        membership.retiring = true;
                    }
                }
                if let SynchronizationState::Active { owner, .. }
                | SynchronizationState::Ready { owner, .. } = self.synchronization
                    && self
                        .memberships
                        .get(&owner)
                        .is_some_and(|member| member.retiring)
                {
                    self.synchronization = SynchronizationState::Cleaning { owner };
                }
                SemanticBrokerDecisionV1::EventAccepted
            }
            SemanticBrokerEventV1::DriveRetirement => self.drive_retirement(),
            SemanticBrokerEventV1::ProviderDriveCompleted { report } => {
                self.complete_provider_drive(*report)
            }
            SemanticBrokerEventV1::ProviderDriveFailed { report } => {
                self.complete_provider_drive_failure(*report)
            }
            SemanticBrokerEventV1::ExpiryDriveCompleted { report } => self.complete_expiry(*report),
        }
    }

    fn event_matches_pending(&self, event: &SemanticBrokerEventV1) -> bool {
        if self.pending.len() != 1 {
            return false;
        }
        let (key, kind) = match event {
            SemanticBrokerEventV1::OpenCompleted { continuation, .. } => {
                (continuation.0, PendingCompletionKind::Open)
            }
            SemanticBrokerEventV1::ChangeCompleted { continuation, .. } => {
                (continuation.0, PendingCompletionKind::Change)
            }
            SemanticBrokerEventV1::CloseCompleted { continuation, .. } => {
                (continuation.0, PendingCompletionKind::Close)
            }
            SemanticBrokerEventV1::SubmissionCompleted { continuation, .. } => {
                (continuation.0, PendingCompletionKind::Submit)
            }
            SemanticBrokerEventV1::CancellationCompleted { continuation, .. } => {
                (continuation.0, PendingCompletionKind::Cancel)
            }
            _ => return false,
        };
        let Some(pending) = self.pending.get(&key) else {
            return false;
        };
        match kind {
            PendingCompletionKind::Open => pending
                .transition
                .as_ref()
                .is_some_and(|value| value.source.kind() == ExpectedSourceKind::Open),
            PendingCompletionKind::Change => pending
                .transition
                .as_ref()
                .is_some_and(|value| value.source.kind() == ExpectedSourceKind::Change),
            PendingCompletionKind::Close => pending
                .transition
                .as_ref()
                .is_some_and(|value| value.source.kind() == ExpectedSourceKind::Close),
            PendingCompletionKind::Submit => matches!(
                pending.command,
                Some(SemanticSessionCommandV1::Submit { .. })
            ),
            PendingCompletionKind::Cancel => {
                pending.cleanup_internal.is_some()
                    || matches!(
                        pending.command,
                        Some(SemanticSessionCommandV1::Cancel { .. })
                    )
            }
        }
    }

    fn join(
        &mut self,
        context: SemanticBrokerContextV1,
        command: SemanticSessionCommandV1,
        operation: SemanticSessionOperationRefV1,
    ) -> SemanticBrokerDecisionV1 {
        let key = (
            context.subject,
            context.session,
            context.policy_epoch,
            operation,
        );
        if let Some(record) = self.joins.get(&key).cloned() {
            return self.replay(&command, record, None);
        }
        let SemanticSessionCommandV1::JoinSession {
            exchange,
            incarnation,
            requested_roles,
            ..
        } = command.clone()
        else {
            unreachable!()
        };
        let bytes = intent_bytes(&command);
        if !self.can_charge(bytes) {
            return self.make_reply(
                command.kind(),
                exchange,
                incarnation,
                None,
                Some(operation),
                SemanticSessionOutcomeV1::ReplayCapacity,
            );
        }
        if incarnation != self.incarnation {
            return self.complete_join(
                key,
                command,
                bytes,
                None,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::InvalidSession),
            );
        }
        if requested_roles == SemanticSessionRolesV1::bottom() {
            return self.complete_join(
                key,
                command,
                bytes,
                None,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::RoleDenied),
            );
        }
        if self.usage.current_memberships >= self.limits.max_memberships {
            return self.complete_join(
                key,
                command,
                bytes,
                None,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::Capacity),
            );
        }
        let roles = requested_roles.meet(context.allowed_roles);
        if roles == SemanticSessionRolesV1::bottom() {
            return self.complete_join(
                key,
                command,
                bytes,
                None,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::RoleDenied),
            );
        }
        let Some(membership) = self.allocate_membership() else {
            return self.complete_join(
                key,
                command,
                bytes,
                None,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::Capacity),
            );
        };
        let outcome = SemanticSessionOutcomeV1::Joined {
            membership,
            granted_roles: roles,
            remaining_operations: self.limits.max_operations_per_membership,
        };
        self.memberships.insert(
            membership,
            Membership {
                context,
                roles,
                operations: BTreeMap::new(),
                requests: BTreeMap::new(),
                retiring: false,
                retired: false,
            },
        );
        self.usage.current_memberships += 1;
        bump(
            &mut self.usage.memberships_joined,
            &mut self.usage.saturated,
        );
        self.usage.membership_high_water = self
            .usage
            .membership_high_water
            .max(self.usage.current_memberships);
        self.complete_join(key, command, bytes, Some(membership), outcome)
    }

    fn drive_retirement(&mut self) -> SemanticBrokerDecisionV1 {
        let Some(membership) = self.memberships.iter().find_map(|(membership, member)| {
            (member.retiring && !member.retired).then_some(*membership)
        }) else {
            return SemanticBrokerDecisionV1::EventAccepted;
        };
        if self.has_incompatible_effect_pending() {
            return SemanticBrokerDecisionV1::AwaitingExisting;
        }
        let terminal: Vec<_> = self.memberships[&membership]
            .requests
            .iter()
            .filter_map(|(external, record)| {
                matches!(record.state, RequestState::Terminal(_)).then_some(*external)
            })
            .collect();
        for external in terminal {
            let record = self
                .memberships
                .get_mut(&membership)
                .and_then(|member| member.requests.get_mut(&external))
                .expect("selected request exists");
            self.usage.retained_result_bytes -= record.reservation;
            record.state = RequestState::Released;
        }
        if let Some(internal) = self.memberships[&membership]
            .requests
            .values()
            .find_map(|record| {
                matches!(record.state, RequestState::Pending).then_some(record.internal)
            })
        {
            return self.emit_system_cleanup(
                membership,
                None,
                Some(internal),
                SemanticBrokerEffectV1::Cancel {
                    continuation: SemanticBrokerCancellationContinuationV1(0),
                    request: internal,
                },
            );
        }
        if self.memberships[&membership]
            .requests
            .values()
            .any(|record| matches!(record.state, RequestState::Submitting))
        {
            return SemanticBrokerDecisionV1::AwaitingExisting;
        }
        if matches!(self.synchronization, SynchronizationState::Active { owner, .. } | SynchronizationState::Ready { owner, .. } | SynchronizationState::Cleaning { owner } if owner == membership)
        {
            self.synchronization = SynchronizationState::Cleaning { owner: membership };
            if let Some(seal) = self
                .cleanup_seals
                .values()
                .next()
                .or_else(|| self.seals.values().next())
                .copied()
            {
                return self.emit_system_cleanup(
                    membership,
                    Some(seal),
                    None,
                    SemanticBrokerEffectV1::Close {
                        continuation: SemanticBrokerAdmissionContinuationV1(0),
                        target: seal.target(),
                    },
                );
            }
        }
        if self.retirement_obligations(membership) != (0, 0) {
            return SemanticBrokerDecisionV1::AwaitingExisting;
        }
        if matches!(self.synchronization, SynchronizationState::Cleaning { owner } if owner == membership)
        {
            self.synchronization = SynchronizationState::Unavailable;
        }
        self.mark_retired(membership);
        SemanticBrokerDecisionV1::EventAccepted
    }

    fn complete_join(
        &mut self,
        key: (
            AuthoritySubjectRefV1,
            AuthoritySessionRefV1,
            u64,
            SemanticSessionOperationRefV1,
        ),
        command: SemanticSessionCommandV1,
        bytes: usize,
        membership: Option<SemanticMembershipRefV1>,
        outcome: SemanticSessionOutcomeV1,
    ) -> SemanticBrokerDecisionV1 {
        let exchange = command.exchange();
        let operation = command.operation();
        let incarnation = match command {
            SemanticSessionCommandV1::JoinSession { incarnation, .. } => incarnation,
            _ => unreachable!(),
        };
        self.joins.insert(
            key,
            ReplayRecord {
                intent: command,
                bytes,
                result: ReplayResult::Complete(outcome.clone()),
            },
        );
        self.charge(bytes);
        self.make_reply(
            SemanticSessionCommandKindV1::JoinSession,
            exchange,
            incarnation,
            membership,
            operation,
            outcome,
        )
    }

    fn bound(
        &mut self,
        context: SemanticBrokerContextV1,
        command: SemanticSessionCommandV1,
    ) -> SemanticBrokerDecisionV1 {
        let (incarnation, membership) = command.binding().expect("bound command");
        let Some(member) = self.memberships.get(&membership) else {
            return self.bound_reply(
                &command,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::UnknownMembership),
            );
        };
        if member.context != context {
            return self.bound_reply(
                &command,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::UnknownMembership),
            );
        }
        if let Some(operation) = command.operation() {
            if let Some(record) = member.operations.get(&operation).cloned() {
                return self.replay(&command, record, Some(membership));
            }
            if member.retired {
                return self.bound_reply(
                    &command,
                    SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::UnknownMembership),
                );
            }
            let bytes = intent_bytes(&command);
            if member.operations.len() >= self.limits.max_operations_per_membership as usize
                || !self.can_charge(bytes)
            {
                return self.bound_reply(&command, SemanticSessionOutcomeV1::ReplayCapacity);
            }
        }
        if self
            .memberships
            .get(&membership)
            .is_some_and(|member| member.retired)
        {
            return self.bound_reply(
                &command,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::UnknownMembership),
            );
        }
        if incarnation != self.incarnation {
            return self.finish(
                command,
                membership,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::InvalidSession),
            );
        }
        let member = self.memberships.get(&membership).expect("validated");
        if member.retiring && command.kind() != SemanticSessionCommandKindV1::LeaveSession {
            return self.finish(
                command,
                membership,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::ClientRetiring),
            );
        }
        if !member.roles.contains(command.required_roles()) {
            return self.finish(
                command,
                membership,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::RoleDenied),
            );
        }
        self.execute(membership, command)
    }

    fn execute(
        &mut self,
        membership: SemanticMembershipRefV1,
        command: SemanticSessionCommandV1,
    ) -> SemanticBrokerDecisionV1 {
        match command.clone() {
            SemanticSessionCommandV1::BeginSynchronization {
                epoch, manifest, ..
            } => {
                if self.seen_epochs.contains(&epoch) {
                    return self.finish(
                        command,
                        membership,
                        SemanticSessionOutcomeV1::Refused(
                            SemanticSessionRefusalV1::ProtocolViolation,
                        ),
                    );
                }
                let can_start_empty =
                    matches!(self.synchronization, SynchronizationState::Unavailable)
                        && self.seals.is_empty()
                        && self.cleanup_seals.is_empty();
                let can_start_ready = match self.synchronization {
                    SynchronizationState::Ready { owner, epoch: _ } => {
                        owner == membership && self.cleanup_seals.is_empty()
                    }
                    _ => false,
                };
                let outcome = if can_start_empty || can_start_ready {
                    self.seen_epochs.insert(epoch);
                    self.synchronization = SynchronizationState::Active {
                        owner: membership,
                        epoch,
                        manifest,
                    };
                    SemanticSessionOutcomeV1::Accepted
                } else {
                    SemanticSessionOutcomeV1::Refused(
                        SemanticSessionRefusalV1::SynchronizationConflict,
                    )
                };
                self.finish(command, membership, outcome)
            }
            SemanticSessionCommandV1::SyncOpen {
                epoch, snapshot, ..
            } => {
                if self.has_incompatible_effect_pending() {
                    return self.finish(
                        command,
                        membership,
                        SemanticSessionOutcomeV1::Refused(
                            SemanticSessionRefusalV1::SynchronizationConflict,
                        ),
                    );
                }
                if !self.active(membership, epoch) {
                    let outcome = self.sync_refusal();
                    return self.finish(command, membership, outcome);
                }
                let Some(seal) = SemanticSynchronizationDocumentSealV1::try_from_snapshot(
                    &snapshot,
                    self.limits,
                ) else {
                    return self.finish(
                        command,
                        membership,
                        SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::InputLimit),
                    );
                };
                if self.seals.contains_key(&seal.document()) {
                    return self.finish(
                        command,
                        membership,
                        SemanticSessionOutcomeV1::Refused(
                            SemanticSessionRefusalV1::ProtocolViolation,
                        ),
                    );
                }
                if !self.stage_within_limits(None, seal) {
                    return self.finish(
                        command,
                        membership,
                        SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::InputLimit),
                    );
                }
                self.emit(
                    command,
                    membership,
                    Some(seal),
                    SemanticBrokerEffectV1::Open {
                        continuation: SemanticBrokerAdmissionContinuationV1(0),
                        snapshot,
                    },
                )
            }
            SemanticSessionCommandV1::SyncChange {
                epoch, snapshot, ..
            } => {
                if self.has_incompatible_effect_pending() {
                    return self.finish(
                        command,
                        membership,
                        SemanticSessionOutcomeV1::Refused(
                            SemanticSessionRefusalV1::SynchronizationConflict,
                        ),
                    );
                }
                if !self.active(membership, epoch) {
                    let outcome = self.sync_refusal();
                    return self.finish(command, membership, outcome);
                }
                let Some(seal) = SemanticSynchronizationDocumentSealV1::try_from_snapshot(
                    &snapshot,
                    self.limits,
                ) else {
                    return self.finish(
                        command,
                        membership,
                        SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::InputLimit),
                    );
                };
                let Some(current) = self.seals.get(&seal.document()).copied() else {
                    return self.finish(
                        command,
                        membership,
                        SemanticSessionOutcomeV1::Refused(
                            SemanticSessionRefusalV1::ProtocolViolation,
                        ),
                    );
                };
                if snapshot.target().source_revision <= current.target().source_revision {
                    return self.finish(
                        command,
                        membership,
                        SemanticSessionOutcomeV1::Refused(
                            SemanticSessionRefusalV1::ProtocolViolation,
                        ),
                    );
                }
                if !self.stage_within_limits(Some(current), seal) {
                    return self.finish(
                        command,
                        membership,
                        SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::InputLimit),
                    );
                }
                self.emit(
                    command,
                    membership,
                    Some(seal),
                    SemanticBrokerEffectV1::Change {
                        continuation: SemanticBrokerAdmissionContinuationV1(0),
                        snapshot,
                    },
                )
            }
            SemanticSessionCommandV1::SyncClose { epoch, target, .. } => {
                if self.has_incompatible_effect_pending() {
                    return self.finish(
                        command,
                        membership,
                        SemanticSessionOutcomeV1::Refused(
                            SemanticSessionRefusalV1::SynchronizationConflict,
                        ),
                    );
                }
                if !self.active(membership, epoch)
                    || self.seals.get(&target.document).map(|seal| seal.target()) != Some(target)
                {
                    return self.finish(
                        command,
                        membership,
                        SemanticSessionOutcomeV1::Refused(
                            SemanticSessionRefusalV1::ProtocolViolation,
                        ),
                    );
                }
                self.emit(
                    command,
                    membership,
                    None,
                    SemanticBrokerEffectV1::Close {
                        continuation: SemanticBrokerAdmissionContinuationV1(0),
                        target,
                    },
                )
            }
            SemanticSessionCommandV1::CompleteSynchronization {
                epoch, manifest, ..
            } => {
                if self.sync_effect_pending() {
                    return self.finish(
                        command,
                        membership,
                        SemanticSessionOutcomeV1::Refused(
                            SemanticSessionRefusalV1::SynchronizationConflict,
                        ),
                    );
                }
                let valid_owner = matches!(self.synchronization, SynchronizationState::Active { owner, epoch: active, manifest: declared } if owner == membership && active == epoch && declared == manifest);
                let seals: Vec<_> = self.seals.values().copied().collect();
                let root_matches =
                    SemanticSynchronizationManifestV1::try_from_seals(&seals, self.limits)
                        == Some(manifest);
                let outcome = if !valid_owner {
                    self.sync_refusal()
                } else if root_matches && self.cleanup_seals.is_empty() {
                    self.synchronization = SynchronizationState::Ready {
                        owner: membership,
                        epoch,
                    };
                    SemanticSessionOutcomeV1::Accepted
                } else {
                    self.synchronization = SynchronizationState::Cleaning { owner: membership };
                    SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::ProtocolViolation)
                };
                self.finish(command, membership, outcome)
            }
            SemanticSessionCommandV1::Submit { request, .. } => {
                self.submit(membership, command, request)
            }
            SemanticSessionCommandV1::Cancel { request, .. } => {
                self.cancel(membership, command, request)
            }
            SemanticSessionCommandV1::Collect { request, .. } => {
                let outcome = self.collect(membership, request);
                self.bound_reply(&command, outcome)
            }
            SemanticSessionCommandV1::ReleaseRequest { request, .. } => {
                self.release(membership, command, request)
            }
            SemanticSessionCommandV1::LeaveSession { .. } => {
                if self.has_incompatible_effect_pending() {
                    return self.finish(
                        command,
                        membership,
                        SemanticSessionOutcomeV1::Refused(
                            SemanticSessionRefusalV1::ProtocolViolation,
                        ),
                    );
                }
                let member = self.memberships.get_mut(&membership).expect("validated");
                member.retiring = true;
                if let Some(internal) = member.requests.values().find_map(|record| {
                    matches!(record.state, RequestState::Pending).then_some(record.internal)
                }) {
                    return self.emit_leave_cancel(command, membership, internal);
                }
                let submitting = member
                    .requests
                    .values()
                    .filter(|record| matches!(record.state, RequestState::Submitting))
                    .count() as u32;
                for record in member.requests.values_mut() {
                    if matches!(record.state, RequestState::Terminal(_)) {
                        self.usage.retained_result_bytes -= record.reservation;
                        record.state = RequestState::Released;
                    }
                }
                if submitting != 0 {
                    return self.finish(
                        command,
                        membership,
                        SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Retirement {
                            remaining_documents: 0,
                            quarantined_requests: submitting,
                        }),
                    );
                }
                if matches!(self.synchronization, SynchronizationState::Active { owner, .. } | SynchronizationState::Ready { owner, .. } | SynchronizationState::Cleaning { owner } if owner == membership)
                {
                    self.synchronization = SynchronizationState::Cleaning { owner: membership };
                    if let Some(seal) = self
                        .cleanup_seals
                        .values()
                        .next()
                        .or_else(|| self.seals.values().next())
                        .copied()
                    {
                        return self.emit(
                            command,
                            membership,
                            Some(seal),
                            SemanticBrokerEffectV1::Close {
                                continuation: SemanticBrokerAdmissionContinuationV1(0),
                                target: seal.target(),
                            },
                        );
                    }
                }
                let obligations = self.retirement_obligations(membership);
                if obligations != (0, 0) {
                    return self.finish(
                        command,
                        membership,
                        SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Retirement {
                            remaining_documents: obligations.0,
                            quarantined_requests: obligations.1,
                        }),
                    );
                }
                if matches!(self.synchronization, SynchronizationState::Cleaning { owner } if owner == membership)
                {
                    self.synchronization = SynchronizationState::Unavailable;
                }
                self.finish(command, membership, SemanticSessionOutcomeV1::Accepted)
            }
            SemanticSessionCommandV1::Describe { .. }
            | SemanticSessionCommandV1::JoinSession { .. } => unreachable!(),
        }
    }

    fn submit(
        &mut self,
        membership: SemanticMembershipRefV1,
        command: SemanticSessionCommandV1,
        external: SemanticRequestV1,
    ) -> SemanticBrokerDecisionV1 {
        if self.has_incompatible_effect_pending() {
            return self.finish(
                command,
                membership,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::ProtocolViolation),
            );
        }
        if !matches!(self.synchronization, SynchronizationState::Ready { .. }) {
            return self.finish(
                command,
                membership,
                SemanticSessionOutcomeV1::Refused(
                    SemanticSessionRefusalV1::SynchronizationRequired,
                ),
            );
        }
        let (
            Some(expected_provider),
            SemanticProviderReadinessProjectionV1::Ready {
                generation: expected_generation,
            },
        ) = (
            self.provider.provenance.clone(),
            self.provider.observation.readiness(),
        )
        else {
            return self.finish(command, membership, SemanticSessionOutcomeV1::Unavailable);
        };
        if external
            .expected_index
            .is_some_and(|expected| expected != expected_generation)
        {
            return self.finish(command, membership, SemanticSessionOutcomeV1::Stale);
        }
        let Some(witness) = self.witnesses.get(&external.target.document) else {
            return self.finish(
                command,
                membership,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::AbsentOrForeign),
            );
        };
        if witness.seal.target() != external.target
            || witness.language != external.language.as_str()
        {
            return self.finish(command, membership, SemanticSessionOutcomeV1::Stale);
        }
        if !query_offset_valid(&external.query, witness) {
            return self.finish(
                command,
                membership,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::InputLimit),
            );
        }
        let member = self.memberships.get(&membership).expect("validated");
        if member.requests.contains_key(&external.request) {
            return self.finish(
                command,
                membership,
                SemanticSessionOutcomeV1::RequestConflict,
            );
        }
        if member.requests.len() >= self.limits.max_requests_per_membership as usize {
            return self.finish(
                command,
                membership,
                SemanticSessionOutcomeV1::ReplayCapacity,
            );
        }
        let globally_pending = self.request_state_count(|state| {
            matches!(state, RequestState::Submitting | RequestState::Pending)
        });
        if globally_pending >= self.limits.max_pending as usize {
            return self.finish(
                command,
                membership,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::Capacity),
            );
        }
        let Some(reservation) = semantic_session_result_reservation_bytes_v1(
            external.query.kind(),
            self.limits.semantic,
        ) else {
            return self.finish(
                command,
                membership,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::Capacity),
            );
        };
        if self
            .usage
            .retained_result_bytes
            .checked_add(reservation)
            .is_none_or(|total| total > self.limits.max_retained_result_bytes)
        {
            return self.finish(
                command,
                membership,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::Capacity),
            );
        }
        let Some(internal) = self.allocate_request() else {
            return self.finish(
                command,
                membership,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::Capacity),
            );
        };
        let mut request = external.clone();
        request.request = internal;
        request.expected_index = Some(expected_generation);
        self.memberships
            .get_mut(&membership)
            .expect("validated")
            .requests
            .insert(
                external.request,
                RequestRecord {
                    internal,
                    external: external.clone(),
                    reservation,
                    provider_epoch: self.provider.epoch,
                    expected_provider,
                    expected_generation,
                    state: RequestState::Submitting,
                },
            );
        self.reverse_requests
            .insert(internal, (membership, external.request));
        self.usage.retained_result_bytes += reservation;
        self.usage.current_requests += 1;
        self.usage.request_high_water = self
            .usage
            .request_high_water
            .max(self.usage.current_requests);
        self.usage.retained_result_bytes_high_water = self
            .usage
            .retained_result_bytes_high_water
            .max(self.usage.retained_result_bytes);
        bump(&mut self.usage.requests_issued, &mut self.usage.saturated);
        self.emit(
            command,
            membership,
            None,
            SemanticBrokerEffectV1::Submit {
                continuation: SemanticBrokerSubmissionContinuationV1(0),
                request,
            },
        )
    }
    fn cancel(
        &mut self,
        membership: SemanticMembershipRefV1,
        command: SemanticSessionCommandV1,
        external: SemanticRequestRefV1,
    ) -> SemanticBrokerDecisionV1 {
        if self.has_incompatible_effect_pending() {
            return self.finish(
                command,
                membership,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::ProtocolViolation),
            );
        }
        let Some(record) = self
            .memberships
            .get(&membership)
            .and_then(|member| member.requests.get(&external))
        else {
            return self.finish(
                command,
                membership,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::AbsentOrForeign),
            );
        };
        if !matches!(record.state, RequestState::Pending) {
            return self.finish(command, membership, SemanticSessionOutcomeV1::Accepted);
        }
        self.emit(
            command,
            membership,
            None,
            SemanticBrokerEffectV1::Cancel {
                continuation: SemanticBrokerCancellationContinuationV1(0),
                request: record.internal,
            },
        )
    }
    fn collect(
        &self,
        membership: SemanticMembershipRefV1,
        external: SemanticRequestRefV1,
    ) -> SemanticSessionOutcomeV1 {
        match self
            .memberships
            .get(&membership)
            .and_then(|member| member.requests.get(&external))
            .map(|record| &record.state)
        {
            Some(RequestState::Submitting | RequestState::Pending) => {
                SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Request(external))
            }
            Some(RequestState::Terminal(outcome)) => outcome.clone(),
            Some(RequestState::Released) | None => {
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::AbsentOrForeign)
            }
        }
    }
    fn release(
        &mut self,
        membership: SemanticMembershipRefV1,
        command: SemanticSessionCommandV1,
        external: SemanticRequestRefV1,
    ) -> SemanticBrokerDecisionV1 {
        let Some(record) = self
            .memberships
            .get_mut(&membership)
            .and_then(|member| member.requests.get_mut(&external))
        else {
            return self.finish(
                command,
                membership,
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::AbsentOrForeign),
            );
        };
        let outcome = match record.state {
            RequestState::Terminal(_) => {
                record.state = RequestState::Released;
                self.usage.retained_result_bytes -= record.reservation;
                SemanticSessionOutcomeV1::Accepted
            }
            RequestState::Released => SemanticSessionOutcomeV1::Accepted,
            RequestState::Submitting | RequestState::Pending => {
                SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::ProtocolViolation)
            }
        };
        self.finish(command, membership, outcome)
    }

    fn emit(
        &mut self,
        command: SemanticSessionCommandV1,
        membership: SemanticMembershipRefV1,
        seal: Option<SemanticSynchronizationDocumentSealV1>,
        effect: SemanticBrokerEffectV1,
    ) -> SemanticBrokerDecisionV1 {
        debug_assert!(self.pending.is_empty(), "global affine effect barrier");
        let operation = command.operation().expect("effects are mutations");
        let bytes = intent_bytes(&command);
        let key = self.next_continuation;
        let Some(next) = self.next_continuation.checked_add(1) else {
            return self.finish(
                command,
                membership,
                SemanticSessionOutcomeV1::ReplayCapacity,
            );
        };
        self.next_continuation = next;
        let effect = match effect {
            SemanticBrokerEffectV1::Open { snapshot, .. } => SemanticBrokerEffectV1::Open {
                continuation: SemanticBrokerAdmissionContinuationV1(key),
                snapshot,
            },
            SemanticBrokerEffectV1::Change { snapshot, .. } => SemanticBrokerEffectV1::Change {
                continuation: SemanticBrokerAdmissionContinuationV1(key),
                snapshot,
            },
            SemanticBrokerEffectV1::Close { target, .. } => SemanticBrokerEffectV1::Close {
                continuation: SemanticBrokerAdmissionContinuationV1(key),
                target,
            },
            SemanticBrokerEffectV1::Submit { request, .. } => SemanticBrokerEffectV1::Submit {
                continuation: SemanticBrokerSubmissionContinuationV1(key),
                request,
            },
            SemanticBrokerEffectV1::Cancel { request, .. } => SemanticBrokerEffectV1::Cancel {
                continuation: SemanticBrokerCancellationContinuationV1(key),
                request,
            },
        };
        self.memberships
            .get_mut(&membership)
            .expect("validated")
            .operations
            .insert(
                operation,
                ReplayRecord {
                    intent: command.clone(),
                    bytes,
                    result: ReplayResult::Pending(key),
                },
            );
        let transition = match &command {
            SemanticSessionCommandV1::SyncOpen { .. } => {
                Some(self.capture_transition_expectation(ExpectedSourceEffect::Open))
            }
            SemanticSessionCommandV1::SyncChange { snapshot, .. } => Some(
                self.capture_transition_expectation(ExpectedSourceEffect::Change {
                    document: snapshot.target().document,
                    target: snapshot.target(),
                }),
            ),
            SemanticSessionCommandV1::SyncClose { target, .. } => Some(
                self.capture_transition_expectation(ExpectedSourceEffect::Close {
                    document: target.document,
                }),
            ),
            SemanticSessionCommandV1::Submit { .. } => {
                Some(self.capture_transition_expectation(ExpectedSourceEffect::None))
            }
            SemanticSessionCommandV1::Cancel { .. } => {
                Some(self.capture_transition_expectation(ExpectedSourceEffect::None))
            }
            _ => match &effect {
                SemanticBrokerEffectV1::Close { target, .. } => Some(
                    self.capture_transition_expectation(ExpectedSourceEffect::Close {
                        document: target.document,
                    }),
                ),
                SemanticBrokerEffectV1::Cancel { .. } => {
                    Some(self.capture_transition_expectation(ExpectedSourceEffect::None))
                }
                _ => None,
            },
        };
        self.pending.insert(
            key,
            PendingEffect {
                command: Some(command),
                membership,
                seal,
                cleanup_internal: None,
                transition,
            },
        );
        self.charge(bytes);
        bump(&mut self.usage.effects, &mut self.usage.saturated);
        if matches!(
            self.pending[&key].command.as_ref(),
            Some(
                SemanticSessionCommandV1::SyncOpen { .. }
                    | SemanticSessionCommandV1::SyncChange { .. }
                    | SemanticSessionCommandV1::SyncClose { .. }
            )
        ) {
            bump(
                &mut self.usage.synchronization_effects,
                &mut self.usage.saturated,
            );
        }
        if matches!(
            self.pending[&key].command.as_ref(),
            Some(SemanticSessionCommandV1::LeaveSession { .. })
        ) {
            bump(&mut self.usage.cleanup_effects, &mut self.usage.saturated);
        }
        SemanticBrokerDecisionV1::Effect(effect)
    }

    fn emit_leave_cancel(
        &mut self,
        command: SemanticSessionCommandV1,
        membership: SemanticMembershipRefV1,
        internal: SemanticRequestRefV1,
    ) -> SemanticBrokerDecisionV1 {
        debug_assert!(self.pending.is_empty(), "global affine effect barrier");
        let operation = command.operation().expect("leave is mutation");
        let bytes = intent_bytes(&command);
        let key = self.next_continuation;
        let Some(next) = key.checked_add(1) else {
            return self.finish(
                command,
                membership,
                SemanticSessionOutcomeV1::ReplayCapacity,
            );
        };
        self.next_continuation = next;
        self.memberships
            .get_mut(&membership)
            .expect("validated")
            .operations
            .insert(
                operation,
                ReplayRecord {
                    intent: command.clone(),
                    bytes,
                    result: ReplayResult::Pending(key),
                },
            );
        let transition = self.capture_transition_expectation(ExpectedSourceEffect::None);
        self.pending.insert(
            key,
            PendingEffect {
                command: Some(command),
                membership,
                seal: None,
                cleanup_internal: Some(internal),
                transition: Some(transition),
            },
        );
        self.charge(bytes);
        bump(&mut self.usage.effects, &mut self.usage.saturated);
        bump(&mut self.usage.cleanup_effects, &mut self.usage.saturated);
        SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Cancel {
            continuation: SemanticBrokerCancellationContinuationV1(key),
            request: internal,
        })
    }

    fn emit_system_cleanup(
        &mut self,
        membership: SemanticMembershipRefV1,
        seal: Option<SemanticSynchronizationDocumentSealV1>,
        cleanup_internal: Option<SemanticRequestRefV1>,
        effect: SemanticBrokerEffectV1,
    ) -> SemanticBrokerDecisionV1 {
        debug_assert!(self.pending.is_empty(), "global affine effect barrier");
        let key = self.next_continuation;
        let Some(next) = key.checked_add(1) else {
            return SemanticBrokerDecisionV1::Quarantined;
        };
        self.next_continuation = next;
        let effect = match effect {
            SemanticBrokerEffectV1::Close { target, .. } => SemanticBrokerEffectV1::Close {
                continuation: SemanticBrokerAdmissionContinuationV1(key),
                target,
            },
            SemanticBrokerEffectV1::Cancel { request, .. } => SemanticBrokerEffectV1::Cancel {
                continuation: SemanticBrokerCancellationContinuationV1(key),
                request,
            },
            _ => return SemanticBrokerDecisionV1::Quarantined,
        };
        let transition = match &effect {
            SemanticBrokerEffectV1::Close { target, .. } => Some(
                self.capture_transition_expectation(ExpectedSourceEffect::Close {
                    document: target.document,
                }),
            ),
            SemanticBrokerEffectV1::Cancel { .. } => {
                Some(self.capture_transition_expectation(ExpectedSourceEffect::None))
            }
            _ => None,
        };
        self.pending.insert(
            key,
            PendingEffect {
                command: None,
                membership,
                seal,
                cleanup_internal,
                transition,
            },
        );
        bump(&mut self.usage.effects, &mut self.usage.saturated);
        bump(&mut self.usage.cleanup_effects, &mut self.usage.saturated);
        SemanticBrokerDecisionV1::Effect(effect)
    }

    fn complete_open(
        &mut self,
        continuation: SemanticBrokerAdmissionContinuationV1,
        report: SemanticBrokerOpenReportV1,
    ) -> SemanticBrokerDecisionV1 {
        let Some(expectation) =
            self.transition_expectation(continuation.0, ExpectedSourceKind::Open)
        else {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionOrdering);
        };
        let Ok(plan) = self.validate_ordered_transition(
            &expectation,
            &report.provider_before,
            report.pre_provider.as_ref(),
            None,
            report.post_provider.as_ref(),
        ) else {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
        };
        let outcome = match report.outcome {
            SemanticBrokerAdmissionOutcomeV1::Accepted => Completion::Accepted,
            SemanticBrokerAdmissionOutcomeV1::Unavailable => Completion::Unavailable,
            SemanticBrokerAdmissionOutcomeV1::Refused(reason) => Completion::Refused(reason),
        };
        self.commit_transition(plan.provider, plan.terminal);
        self.complete_effect(continuation.0, outcome)
    }

    fn complete_source(
        &mut self,
        continuation: SemanticBrokerAdmissionContinuationV1,
        report: SemanticBrokerSourceReportV1,
        cause: SemanticInvalidationCauseV1,
    ) -> SemanticBrokerDecisionV1 {
        let kind = match cause {
            SemanticInvalidationCauseV1::SourceChanged => ExpectedSourceKind::Change,
            SemanticInvalidationCauseV1::SourceClosed => ExpectedSourceKind::Close,
            SemanticInvalidationCauseV1::ProviderStateChanged => unreachable!(),
        };
        let Some(expectation) = self.transition_expectation(continuation.0, kind) else {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionOrdering);
        };
        let source = match &report.outcome {
            SemanticBrokerSourceAdmissionOutcomeV1::Accepted(receipt) => Some((cause, receipt)),
            SemanticBrokerSourceAdmissionOutcomeV1::Unavailable
            | SemanticBrokerSourceAdmissionOutcomeV1::Refused(_) => None,
        };
        let Ok(plan) = self.validate_ordered_transition(
            &expectation,
            &report.provider_before,
            report.pre_provider.as_ref(),
            source,
            report.post_provider.as_ref(),
        ) else {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
        };
        let outcome = match report.outcome {
            SemanticBrokerSourceAdmissionOutcomeV1::Accepted(_) => Completion::Accepted,
            SemanticBrokerSourceAdmissionOutcomeV1::Unavailable => Completion::Unavailable,
            SemanticBrokerSourceAdmissionOutcomeV1::Refused(reason) => Completion::Refused(reason),
        };
        self.commit_transition(plan.provider, plan.terminal);
        self.complete_effect(continuation.0, outcome)
    }

    fn transition_expectation(
        &self,
        continuation: u64,
        kind: ExpectedSourceKind,
    ) -> Option<TransitionExpectation> {
        let expectation = self.pending.get(&continuation)?.transition.as_ref()?;
        (expectation.source.kind() == kind).then(|| expectation.clone())
    }

    fn validate_ordered_transition(
        &self,
        expectation: &TransitionExpectation,
        provider_before: &SemanticBrokerProviderBindingV1,
        pre: Option<&SemanticBrokerProviderTransitionV1>,
        source: Option<(SemanticInvalidationCauseV1, &SemanticInvalidationReceiptV1)>,
        post: Option<&SemanticBrokerProviderTransitionV1>,
    ) -> Result<TransitionPlan, ()> {
        if self.capture_transition_expectation(expectation.source) != *expectation
            || !provider_binding_matches(provider_before, &expectation.provider)
        {
            return Err(());
        }
        let mut shadow = expectation.live_ids();
        let mut terminal = BTreeSet::new();
        let mut provider = self.provider.clone();
        if let Some(transition) = pre {
            validate_provider_transition(
                transition,
                expectation,
                &mut shadow,
                &mut terminal,
                &mut provider,
            )?;
        }
        if let Some((cause, receipt)) = source {
            let expected: BTreeSet<_> = expectation
                .requests
                .iter()
                .filter(|request| request.source_affected && shadow.contains(&request.internal))
                .map(|request| request.internal)
                .collect();
            validate_receipt(receipt, cause, &expected)?;
            for request in expected {
                shadow.remove(&request);
                terminal.insert(request);
            }
        }
        if let Some(transition) = post {
            validate_provider_transition(
                transition,
                expectation,
                &mut shadow,
                &mut terminal,
                &mut provider,
            )?;
        }
        Ok(TransitionPlan {
            provider,
            terminal,
            remaining: shadow,
        })
    }

    fn complete_provider_drive(
        &mut self,
        report: SemanticBrokerProviderDriveReportV1,
    ) -> SemanticBrokerDecisionV1 {
        if self.has_incompatible_effect_pending() {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionOrdering);
        }
        let expectation = self.capture_transition_expectation(ExpectedSourceEffect::None);
        let Ok(plan) = self.validate_ordered_transition(
            &expectation,
            &report.provider_before,
            report.pre_provider.as_ref(),
            None,
            report.post_provider.as_ref(),
        ) else {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
        };

        match &report.disposition {
            SemanticBrokerProviderDriveDispositionV1::NoEvent => {}
            SemanticBrokerProviderDriveDispositionV1::Delivery {
                internal_request, ..
            } => {
                if !plan.remaining.contains(internal_request) {
                    return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
                }
            }
            SemanticBrokerProviderDriveDispositionV1::Late { internal_request } => {
                let terminal_now = plan.terminal.contains(internal_request)
                    || self.request_is_non_pending(*internal_request);
                if !terminal_now || plan.remaining.contains(internal_request) {
                    return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
                }
            }
            SemanticBrokerProviderDriveDispositionV1::Orphan { internal_request } => {
                if self.reverse_requests.contains_key(internal_request) {
                    return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
                }
            }
        }

        self.commit_transition(plan.provider, plan.terminal);
        match report.disposition {
            SemanticBrokerProviderDriveDispositionV1::NoEvent => {
                SemanticBrokerDecisionV1::EventAccepted
            }
            SemanticBrokerProviderDriveDispositionV1::Delivery {
                internal_request,
                outcome,
            } => self.delivery(internal_request, outcome),
            SemanticBrokerProviderDriveDispositionV1::Late { .. } => {
                bump(&mut self.usage.quarantined, &mut self.usage.saturated);
                bump(
                    &mut self.usage.deliveries_orphaned,
                    &mut self.usage.saturated,
                );
                SemanticBrokerDecisionV1::EventAccepted
            }
            SemanticBrokerProviderDriveDispositionV1::Orphan { .. } => {
                bump(&mut self.usage.quarantined, &mut self.usage.saturated);
                bump(
                    &mut self.usage.deliveries_orphaned,
                    &mut self.usage.saturated,
                );
                SemanticBrokerDecisionV1::EventAccepted
            }
        }
    }

    fn complete_provider_drive_failure(
        &mut self,
        report: SemanticBrokerProviderDriveFailureReportV1,
    ) -> SemanticBrokerDecisionV1 {
        if self.has_incompatible_effect_pending() {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionOrdering);
        }
        let expectation = self.capture_transition_expectation(ExpectedSourceEffect::None);
        let Ok(plan) = self.validate_ordered_transition(
            &expectation,
            &report.provider_before,
            report.pre_provider.as_ref(),
            None,
            None,
        ) else {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
        };
        self.commit_transition(plan.provider, plan.terminal);
        bump(
            &mut self.usage.provider_drive_failures,
            &mut self.usage.saturated,
        );
        let class = match report.failure {
            SemanticBrokerProviderDriveFailureClassV1::Poisoned => 0,
            SemanticBrokerProviderDriveFailureClassV1::RuntimeUnavailable => 1,
            SemanticBrokerProviderDriveFailureClassV1::DeliveryRefused => 2,
        };
        let saturated = bump_slot(&mut self.usage.provider_drive_failures_by_class, class);
        self.usage.saturated |= saturated;
        SemanticBrokerDecisionV1::EventAccepted
    }

    fn complete_expiry(
        &mut self,
        report: SemanticBrokerExpiryReportV1,
    ) -> SemanticBrokerDecisionV1 {
        if self.has_incompatible_effect_pending() {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionOrdering);
        }
        let expectation = self.capture_transition_expectation(ExpectedSourceEffect::None);
        if !provider_binding_matches(&report.provider_before, &expectation.provider) {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
        }
        let mut shadow = expectation.live_ids();
        let mut terminal = BTreeSet::new();
        let mut provider = self.provider.clone();
        if let Some(pre) = report.pre_provider.as_ref()
            && validate_provider_transition(
                pre,
                &expectation,
                &mut shadow,
                &mut terminal,
                &mut provider,
            )
            .is_err()
        {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
        }

        let target = report.internal_request;
        let explicitly_expired = match report.outcome {
            SemanticBrokerExpiryOutcomeV1::Expired => {
                if !shadow.remove(&target) || terminal.contains(&target) {
                    return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
                }
                true
            }
            SemanticBrokerExpiryOutcomeV1::SatisfiedByProviderTransition => {
                if !terminal.contains(&target) || shadow.contains(&target) {
                    return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
                }
                false
            }
            SemanticBrokerExpiryOutcomeV1::NotPending => {
                if terminal.contains(&target)
                    || shadow.contains(&target)
                    || !self.request_is_non_pending(target)
                {
                    return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
                }
                false
            }
        };
        if let Some(post) = report.post_provider.as_ref()
            && validate_provider_transition(
                post,
                &expectation,
                &mut shadow,
                &mut terminal,
                &mut provider,
            )
            .is_err()
        {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
        }

        let explicit_record = if explicitly_expired {
            let Some((membership, external)) = self.reverse_requests.get(&target).copied() else {
                return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
            };
            let valid = self
                .memberships
                .get(&membership)
                .and_then(|member| member.requests.get(&external))
                .is_some_and(|record| matches!(record.state, RequestState::Pending));
            if !valid {
                return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
            }
            Some((membership, external))
        } else {
            None
        };

        self.commit_transition(provider, terminal);
        if let Some((membership, external)) = explicit_record {
            let record = self
                .memberships
                .get_mut(&membership)
                .and_then(|member| member.requests.get_mut(&external))
                .expect("validated expiry record remains present");
            assert!(matches!(record.state, RequestState::Pending));
            record.state = RequestState::Terminal(SemanticSessionOutcomeV1::TimedOut);
        }
        bump(&mut self.usage.expiry_reports, &mut self.usage.saturated);
        match report.outcome {
            SemanticBrokerExpiryOutcomeV1::Expired => {
                bump(&mut self.usage.requests_expired, &mut self.usage.saturated)
            }
            SemanticBrokerExpiryOutcomeV1::SatisfiedByProviderTransition => bump(
                &mut self.usage.expiries_satisfied_by_transition,
                &mut self.usage.saturated,
            ),
            SemanticBrokerExpiryOutcomeV1::NotPending => bump(
                &mut self.usage.expiries_not_pending,
                &mut self.usage.saturated,
            ),
        }
        SemanticBrokerDecisionV1::EventAccepted
    }

    fn request_is_non_pending(&self, internal: SemanticRequestRefV1) -> bool {
        let Some((membership, external)) = self.reverse_requests.get(&internal).copied() else {
            return false;
        };
        self.memberships
            .get(&membership)
            .and_then(|member| member.requests.get(&external))
            .is_some_and(|record| !matches!(record.state, RequestState::Pending))
    }

    fn commit_transition(
        &mut self,
        provider: ProviderState,
        terminal: BTreeSet<SemanticRequestRefV1>,
    ) {
        for internal in terminal {
            if let Some((membership, external)) = self.reverse_requests.get(&internal).copied()
                && let Some(record) = self
                    .memberships
                    .get_mut(&membership)
                    .and_then(|member| member.requests.get_mut(&external))
                && matches!(
                    record.state,
                    RequestState::Submitting | RequestState::Pending
                )
            {
                record.state = RequestState::Terminal(SemanticSessionOutcomeV1::Stale);
            }
        }
        self.provider = provider;
    }

    fn diverge(&mut self, reason: SemanticBrokerDivergenceV1) -> SemanticBrokerDecisionV1 {
        if self.divergence.is_none() {
            self.divergence = Some(reason);
            bump(
                &mut self.usage.transition_divergences,
                &mut self.usage.saturated,
            );
            if matches!(reason, SemanticBrokerDivergenceV1::TransitionOrdering) {
                bump(
                    &mut self.usage.transition_ordering_divergences,
                    &mut self.usage.saturated,
                );
            }
        }
        SemanticBrokerDecisionV1::Diverged(reason)
    }
    fn complete_submission(
        &mut self,
        continuation: SemanticBrokerSubmissionContinuationV1,
        report: SemanticBrokerSubmissionReportV1,
    ) -> SemanticBrokerDecisionV1 {
        let Some(mut expectation) =
            self.transition_expectation(continuation.0, ExpectedSourceKind::None)
        else {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionOrdering);
        };
        if self.capture_transition_expectation(ExpectedSourceEffect::None) != expectation {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionOrdering);
        }
        if !provider_binding_matches(&report.provider_before, &expectation.provider) {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
        }
        let submitted = self.pending.get(&continuation.0).and_then(|pending| {
            let external = match pending.command.as_ref()? {
                SemanticSessionCommandV1::Submit { request, .. } => request.request,
                _ => return None,
            };
            self.memberships
                .get(&pending.membership)?
                .requests
                .get(&external)
                .map(|record| record.internal)
        });
        let submitted_kind = self
            .pending
            .get(&continuation.0)
            .and_then(|pending| pending.command.as_ref())
            .and_then(|command| match command {
                SemanticSessionCommandV1::Submit { request, .. } => Some(request.query.kind()),
                _ => None,
            });
        if let SemanticBrokerSubmissionOutcomeV1::Unsupported(kind) = &report.outcome
            && submitted_kind != Some(*kind)
        {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
        }
        let accepted = matches!(report.outcome, SemanticBrokerSubmissionOutcomeV1::Accepted);
        let mut shadow = expectation.live_ids();
        let mut terminal = BTreeSet::new();
        let mut provider = self.provider.clone();
        if let Some(pre) = report.pre_provider.as_ref()
            && validate_provider_transition(
                pre,
                &expectation,
                &mut shadow,
                &mut terminal,
                &mut provider,
            )
            .is_err()
        {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
        }
        let mut accepted_binding = None;
        if accepted {
            let Some(internal) = submitted else {
                return self.diverge(SemanticBrokerDivergenceV1::TransitionOrdering);
            };
            let Some((membership, external)) = self.reverse_requests.get(&internal).copied() else {
                return self.diverge(SemanticBrokerDivergenceV1::TransitionOrdering);
            };
            let Some(record) = self
                .memberships
                .get(&membership)
                .and_then(|member| member.requests.get(&external))
            else {
                return self.diverge(SemanticBrokerDivergenceV1::TransitionOrdering);
            };
            let (Some(provenance), SemanticProviderReadinessProjectionV1::Ready { generation }) = (
                provider.provenance.clone(),
                provider.observation.readiness(),
            ) else {
                return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
            };
            if record.expected_generation != generation
                || !record
                    .external
                    .query
                    .required_capabilities()
                    .is_subset_of(provider.observation.capabilities())
            {
                return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
            }
            expectation.requests.push(ExpectedRequest {
                internal,
                provider_epoch: provider.epoch,
                source_affected: false,
            });
            expectation.requests.sort_by_key(|request| request.internal);
            shadow.insert(internal);
            accepted_binding = Some((internal, provider.epoch, provenance, generation));
        }
        if let Some(post) = report.post_provider.as_ref()
            && validate_provider_transition(
                post,
                &expectation,
                &mut shadow,
                &mut terminal,
                &mut provider,
            )
            .is_err()
        {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
        }
        let outcome = match report.outcome {
            SemanticBrokerSubmissionOutcomeV1::Accepted => Completion::Accepted,
            SemanticBrokerSubmissionOutcomeV1::Unsupported(kind) => Completion::Unsupported(kind),
            SemanticBrokerSubmissionOutcomeV1::Stale => Completion::Stale,
            SemanticBrokerSubmissionOutcomeV1::Unavailable => Completion::Unavailable,
            SemanticBrokerSubmissionOutcomeV1::Refused(reason) => Completion::Refused(reason),
        };
        if let Some((internal, epoch, provenance, generation)) = accepted_binding {
            self.commit_submission_binding(internal, epoch, provenance, generation);
        }
        self.commit_transition(provider, terminal);
        self.complete_effect(continuation.0, outcome)
    }

    fn commit_submission_binding(
        &mut self,
        internal: SemanticRequestRefV1,
        provider_epoch: u64,
        expected_provider: SemanticProviderProvenanceV1,
        expected_generation: SemanticIndexGenerationV1,
    ) {
        let (membership, external) = self.reverse_requests[&internal];
        let record = self
            .memberships
            .get_mut(&membership)
            .and_then(|member| member.requests.get_mut(&external))
            .expect("validated submission record remains present");
        assert!(matches!(record.state, RequestState::Submitting));
        record.provider_epoch = provider_epoch;
        record.expected_provider = expected_provider;
        record.expected_generation = expected_generation;
    }
    fn complete_cancellation(
        &mut self,
        continuation: SemanticBrokerCancellationContinuationV1,
        report: SemanticBrokerCancellationReportV1,
    ) -> SemanticBrokerDecisionV1 {
        let Some(expectation) =
            self.transition_expectation(continuation.0, ExpectedSourceKind::None)
        else {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionOrdering);
        };
        if self.capture_transition_expectation(ExpectedSourceEffect::None) != expectation {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionOrdering);
        }
        if !provider_binding_matches(&report.provider_before, &expectation.provider) {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
        }
        if matches!(
            report.outcome,
            SemanticBrokerCancellationOutcomeV1::Refused(_)
        ) {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
        }
        let mut shadow = expectation.live_ids();
        let mut terminal = BTreeSet::new();
        let mut provider = self.provider.clone();
        if let Some(pre) = report.pre_provider.as_ref()
            && validate_provider_transition(
                pre,
                &expectation,
                &mut shadow,
                &mut terminal,
                &mut provider,
            )
            .is_err()
        {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
        }
        let cancelled = self.pending.get(&continuation.0).and_then(|pending| {
            pending.cleanup_internal.or_else(|| {
                let external = match pending.command.as_ref()? {
                    SemanticSessionCommandV1::Cancel { request, .. } => *request,
                    _ => return None,
                };
                self.memberships
                    .get(&pending.membership)?
                    .requests
                    .get(&external)
                    .map(|record| record.internal)
            })
        });
        let accepted = matches!(
            report.outcome,
            SemanticBrokerCancellationOutcomeV1::Accepted
        );
        if accepted {
            let Some(cancelled) = cancelled else {
                return self.diverge(SemanticBrokerDivergenceV1::TransitionOrdering);
            };
            if !shadow.remove(&cancelled) && !terminal.contains(&cancelled) {
                return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
            }
        }
        if let Some(post) = report.post_provider.as_ref()
            && validate_provider_transition(
                post,
                &expectation,
                &mut shadow,
                &mut terminal,
                &mut provider,
            )
            .is_err()
        {
            return self.diverge(SemanticBrokerDivergenceV1::TransitionEvidence);
        }
        let outcome = match report.outcome {
            SemanticBrokerCancellationOutcomeV1::Accepted => Completion::Accepted,
            SemanticBrokerCancellationOutcomeV1::Refused(reason) => Completion::Refused(reason),
        };
        let cancelled_was_staled = cancelled.is_some_and(|request| terminal.contains(&request));
        self.commit_transition(provider, terminal);
        if accepted && !cancelled_was_staled {
            self.commit_explicit_cancellation(continuation.0, cancelled);
        }
        self.complete_effect(continuation.0, outcome)
    }

    fn commit_explicit_cancellation(
        &mut self,
        continuation: u64,
        cancelled: Option<SemanticRequestRefV1>,
    ) {
        let Some(internal) = cancelled else { return };
        let explicit = self.pending.get(&continuation).is_some_and(|pending| {
            matches!(
                pending.command,
                Some(SemanticSessionCommandV1::Cancel { .. })
            )
        });
        if !explicit {
            return;
        }
        let (membership, external) = self.reverse_requests[&internal];
        let record = self
            .memberships
            .get_mut(&membership)
            .and_then(|member| member.requests.get_mut(&external))
            .expect("validated cancellation record remains present");
        assert!(matches!(record.state, RequestState::Pending));
        record.state = RequestState::Terminal(SemanticSessionOutcomeV1::Cancelled);
    }
    fn complete_effect(
        &mut self,
        continuation: u64,
        effect: Completion,
    ) -> SemanticBrokerDecisionV1 {
        let Some(pending) = self.pending.remove(&continuation) else {
            bump(&mut self.usage.quarantined, &mut self.usage.saturated);
            return SemanticBrokerDecisionV1::Quarantined;
        };
        let system_retirement = pending.command.is_none();
        let retirement_effect = system_retirement
            || matches!(
                pending.command.as_ref(),
                Some(SemanticSessionCommandV1::LeaveSession { .. })
            );
        if retirement_effect && !matches!(effect, Completion::Accepted) {
            bump(&mut self.usage.cleanup_refusals, &mut self.usage.saturated);
        }
        if system_retirement && !matches!(effect, Completion::Accepted) {
            return SemanticBrokerDecisionV1::EventAccepted;
        }
        if !matches!(effect, Completion::Accepted)
            && matches!(
                pending.command.as_ref(),
                Some(
                    SemanticSessionCommandV1::SyncOpen { .. }
                        | SemanticSessionCommandV1::SyncChange { .. }
                        | SemanticSessionCommandV1::SyncClose { .. }
                )
            )
        {
            self.synchronization = SynchronizationState::Cleaning {
                owner: pending.membership,
            };
        }
        let leave_cleanup_pending = matches!(
            pending.command.as_ref(),
            Some(SemanticSessionCommandV1::LeaveSession { .. })
        ) && !matches!(effect, Completion::Accepted);
        let outcome = if leave_cleanup_pending {
            let (remaining_documents, quarantined_requests) =
                self.retirement_obligations(pending.membership);
            if remaining_documents == 0 && quarantined_requests == 0 {
                SemanticSessionOutcomeV1::Accepted
            } else {
                SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Retirement {
                    remaining_documents,
                    quarantined_requests,
                })
            }
        } else {
            match effect {
                Completion::Accepted => {
                    match pending.command.as_ref() {
                        None => {
                            self.apply_retirement_cleanup(&pending);
                        }
                        Some(
                            SemanticSessionCommandV1::SyncOpen { .. }
                            | SemanticSessionCommandV1::SyncChange { .. },
                        ) => {
                            if let Some(seal) = pending.seal {
                                let changed_document = matches!(
                                    pending.command.as_ref(),
                                    Some(SemanticSessionCommandV1::SyncChange { .. })
                                )
                                .then_some((seal.document(), seal.target()));
                                let authoritative = sync_effect_is_current(
                                    pending.command.as_ref().expect("client effect"),
                                    pending.membership,
                                    self.synchronization,
                                );
                                if authoritative {
                                    let snapshot = match pending.command.as_ref() {
                                        Some(SemanticSessionCommandV1::SyncOpen {
                                            snapshot,
                                            ..
                                        })
                                        | Some(SemanticSessionCommandV1::SyncChange {
                                            snapshot,
                                            ..
                                        }) => snapshot,
                                        _ => unreachable!(),
                                    };
                                    if let Some(witness) =
                                        DocumentWitness::from_snapshot(snapshot, self.limits)
                                    {
                                        self.witnesses.insert(seal.document(), witness);
                                        self.seals.insert(seal.document(), seal);
                                        self.usage.compact_witness_bytes_high_water = self
                                            .usage
                                            .compact_witness_bytes_high_water
                                            .max(self.compact_witness_bytes());
                                    }
                                } else {
                                    // The sovereign effect happened after its epoch was fenced.
                                    // It is real cleanup debt, never authoritative readiness.
                                    self.seals.remove(&seal.document());
                                    self.witnesses.remove(&seal.document());
                                    self.cleanup_seals.insert(seal.document(), seal);
                                    if self
                                        .memberships
                                        .get(&pending.membership)
                                        .is_some_and(|member| member.retiring)
                                    {
                                        self.synchronization = SynchronizationState::Cleaning {
                                            owner: pending.membership,
                                        };
                                    }
                                }
                                let _ = changed_document;
                            }
                        }
                        Some(SemanticSessionCommandV1::SyncClose { target, .. }) => {
                            self.seals.remove(&target.document);
                            self.witnesses.remove(&target.document);
                            self.cleanup_seals.remove(&target.document);
                        }
                        Some(SemanticSessionCommandV1::LeaveSession { .. }) => {
                            self.apply_retirement_cleanup(&pending);
                        }
                        Some(SemanticSessionCommandV1::Submit { request, .. }) => {
                            if let Some(record) = self
                                .memberships
                                .get_mut(&pending.membership)
                                .and_then(|member| member.requests.get_mut(&request.request))
                                && matches!(record.state, RequestState::Submitting)
                                && record.provider_epoch == self.provider.epoch
                            {
                                record.state = RequestState::Pending;
                            }
                        }
                        Some(_) => {}
                    }
                    let obligations = self.retirement_obligations(pending.membership);
                    if (system_retirement
                        || matches!(
                            pending.command.as_ref(),
                            Some(SemanticSessionCommandV1::LeaveSession { .. })
                        ))
                        && obligations != (0, 0)
                    {
                        SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Retirement {
                            remaining_documents: obligations.0,
                            quarantined_requests: obligations.1,
                        })
                    } else {
                        if matches!(
                            pending.command.as_ref(),
                            Some(SemanticSessionCommandV1::LeaveSession { .. })
                        ) && matches!(self.synchronization, SynchronizationState::Cleaning { owner } if owner == pending.membership)
                        {
                            self.synchronization = SynchronizationState::Unavailable;
                        }
                        SemanticSessionOutcomeV1::Accepted
                    }
                }
                Completion::Unsupported(kind) => SemanticSessionOutcomeV1::Unsupported(kind),
                Completion::Stale => SemanticSessionOutcomeV1::Stale,
                Completion::Unavailable => SemanticSessionOutcomeV1::Unavailable,
                Completion::Refused(reason) => SemanticSessionOutcomeV1::Refused(reason),
            }
        };
        if !matches!(outcome, SemanticSessionOutcomeV1::Accepted)
            && let Some(SemanticSessionCommandV1::Submit { request, .. }) = pending.command.as_ref()
            && let Some(record) = self
                .memberships
                .get_mut(&pending.membership)
                .and_then(|member| member.requests.get_mut(&request.request))
        {
            self.usage.retained_result_bytes -= record.reservation;
            record.state = RequestState::Released;
        }
        if let Some(operation) = pending
            .command
            .as_ref()
            .and_then(SemanticSessionCommandV1::operation)
            && let Some(record) = self
                .memberships
                .get_mut(&pending.membership)
                .and_then(|member| member.operations.get_mut(&operation))
        {
            record.result = ReplayResult::Complete(outcome.clone());
        }
        if (system_retirement
            || matches!(
                pending.command.as_ref(),
                Some(SemanticSessionCommandV1::LeaveSession { .. })
            ))
            && matches!(outcome, SemanticSessionOutcomeV1::Accepted)
        {
            if matches!(self.synchronization, SynchronizationState::Cleaning { owner } if owner == pending.membership)
            {
                self.synchronization = SynchronizationState::Unavailable;
            }
            self.mark_retired(pending.membership);
        }
        match pending.command.as_ref() {
            Some(command) => self.bound_reply(command, outcome),
            None => SemanticBrokerDecisionV1::EventAccepted,
        }
    }

    fn apply_retirement_cleanup(&mut self, pending: &PendingEffect) {
        if let Some(seal) = pending.seal {
            self.seals.remove(&seal.document());
            self.witnesses.remove(&seal.document());
            self.cleanup_seals.remove(&seal.document());
        }
        if let Some(internal) = pending.cleanup_internal
            && let Some((member, external)) = self.reverse_requests.get(&internal).copied()
            && let Some(record) = self
                .memberships
                .get_mut(&member)
                .and_then(|state| state.requests.get_mut(&external))
            && !matches!(record.state, RequestState::Released)
        {
            self.usage.retained_result_bytes -= record.reservation;
            record.state = RequestState::Released;
        }
    }

    fn delivery(
        &mut self,
        internal: SemanticRequestRefV1,
        delivery: SemanticBrokerDeliveryOutcomeV1,
    ) -> SemanticBrokerDecisionV1 {
        let Some((membership, external)) = self.reverse_requests.get(&internal).copied() else {
            bump(&mut self.usage.quarantined, &mut self.usage.saturated);
            bump(
                &mut self.usage.deliveries_orphaned,
                &mut self.usage.saturated,
            );
            return SemanticBrokerDecisionV1::Quarantined;
        };
        let Some(record) = self
            .memberships
            .get_mut(&membership)
            .and_then(|member| member.requests.get_mut(&external))
        else {
            bump(&mut self.usage.quarantined, &mut self.usage.saturated);
            bump(
                &mut self.usage.deliveries_orphaned,
                &mut self.usage.saturated,
            );
            return SemanticBrokerDecisionV1::Quarantined;
        };
        if !matches!(record.state, RequestState::Pending) {
            bump(&mut self.usage.quarantined, &mut self.usage.saturated);
            bump(
                &mut self.usage.deliveries_orphaned,
                &mut self.usage.saturated,
            );
            return SemanticBrokerDecisionV1::Quarantined;
        }
        let outcome = match delivery {
            SemanticBrokerDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Complete(mut reply)) => {
                if reply.request != internal
                    || reply.target != record.external.target
                    || reply.provider != record.expected_provider
                    || reply.index_generation != record.expected_generation
                    || record
                        .external
                        .expected_index
                        .is_some_and(|expected| expected != record.expected_generation)
                    || !result_matches_query(&reply.result, record.external.query.kind())
                    || !reply.result.is_within_limits(self.limits.semantic)
                    || !result_locations_valid(&reply.result, &record.external, &self.witnesses)
                {
                    bump(&mut self.usage.quarantined, &mut self.usage.saturated);
                    bump(
                        &mut self.usage.deliveries_malformed,
                        &mut self.usage.saturated,
                    );
                    SemanticSessionOutcomeV1::PeerRefused(
                        vellum_host::semantic::SemanticRefusalV1::MalformedProviderReply,
                    )
                } else {
                    reply.request = external;
                    SemanticSessionOutcomeV1::Complete(reply)
                }
            }
            SemanticBrokerDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Unsupported(kind)) => {
                if kind != record.external.query.kind() {
                    bump(&mut self.usage.quarantined, &mut self.usage.saturated);
                    bump(
                        &mut self.usage.deliveries_malformed,
                        &mut self.usage.saturated,
                    );
                    SemanticSessionOutcomeV1::PeerRefused(
                        vellum_host::semantic::SemanticRefusalV1::MalformedProviderReply,
                    )
                } else {
                    SemanticSessionOutcomeV1::Unsupported(kind)
                }
            }
            SemanticBrokerDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Stale) => {
                SemanticSessionOutcomeV1::Stale
            }
            SemanticBrokerDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Unavailable) => {
                SemanticSessionOutcomeV1::Unavailable
            }
            SemanticBrokerDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Refused(reason)) => {
                SemanticSessionOutcomeV1::PeerRefused(reason)
            }
        };
        record.state = RequestState::Terminal(outcome);
        bump(&mut self.usage.deliveries_routed, &mut self.usage.saturated);
        SemanticBrokerDecisionV1::EventAccepted
    }

    fn finish(
        &mut self,
        command: SemanticSessionCommandV1,
        membership: SemanticMembershipRefV1,
        outcome: SemanticSessionOutcomeV1,
    ) -> SemanticBrokerDecisionV1 {
        if let Some(operation) = command.operation() {
            let bytes = intent_bytes(&command);
            self.memberships
                .get_mut(&membership)
                .expect("validated")
                .operations
                .insert(
                    operation,
                    ReplayRecord {
                        intent: command.clone(),
                        bytes,
                        result: ReplayResult::Complete(outcome.clone()),
                    },
                );
            self.charge(bytes);
        }
        if matches!(command, SemanticSessionCommandV1::LeaveSession { .. })
            && matches!(outcome, SemanticSessionOutcomeV1::Accepted)
        {
            self.mark_retired(membership);
        }
        self.bound_reply(&command, outcome)
    }

    fn replay(
        &mut self,
        command: &SemanticSessionCommandV1,
        record: ReplayRecord,
        membership: Option<SemanticMembershipRefV1>,
    ) -> SemanticBrokerDecisionV1 {
        let _charged_bytes = record.bytes;
        if !same_intent(command, &record.intent) {
            bump(&mut self.usage.conflicts, &mut self.usage.saturated);
            return match membership {
                Some(_) => self.bound_reply(command, SemanticSessionOutcomeV1::RequestConflict),
                None => self.make_reply(
                    command.kind(),
                    command.exchange(),
                    self.incarnation,
                    None,
                    command.operation(),
                    SemanticSessionOutcomeV1::RequestConflict,
                ),
            };
        }
        bump(&mut self.usage.replay_hits, &mut self.usage.saturated);
        match record.result {
            ReplayResult::Pending(continuation) => {
                let _ = continuation;
                SemanticBrokerDecisionV1::AwaitingExisting
            }
            ReplayResult::Complete(outcome) => match membership {
                Some(_) => self.bound_reply(command, outcome),
                None => {
                    let issued = match &outcome {
                        SemanticSessionOutcomeV1::Joined { membership, .. } => Some(*membership),
                        _ => None,
                    };
                    self.make_reply(
                        command.kind(),
                        command.exchange(),
                        self.incarnation,
                        issued,
                        command.operation(),
                        outcome,
                    )
                }
            },
        }
    }

    fn bound_reply(
        &mut self,
        command: &SemanticSessionCommandV1,
        outcome: SemanticSessionOutcomeV1,
    ) -> SemanticBrokerDecisionV1 {
        let (incarnation, membership) = command.binding().expect("bound");
        self.make_reply(
            command.kind(),
            command.exchange(),
            incarnation,
            Some(membership),
            command.operation(),
            outcome,
        )
    }
    fn make_reply(
        &mut self,
        kind: SemanticSessionCommandKindV1,
        exchange: SemanticExchangeRefV1,
        incarnation: SemanticSessionIncarnationV1,
        membership: Option<SemanticMembershipRefV1>,
        operation: Option<SemanticSessionOperationRefV1>,
        outcome: SemanticSessionOutcomeV1,
    ) -> SemanticBrokerDecisionV1 {
        let outcome_index = semantic_outcome_index(&outcome);
        bump(
            &mut self.usage.replies_by_outcome[outcome_index],
            &mut self.usage.saturated,
        );
        if let SemanticSessionOutcomeV1::Refused(reason) = &outcome {
            bump(
                &mut self.usage.refusals_by_reason[*reason as usize],
                &mut self.usage.saturated,
            );
        }
        SemanticBrokerDecisionV1::Reply(
            SemanticSessionReplyV1::try_new(
                kind,
                exchange,
                incarnation,
                membership,
                operation,
                outcome,
            )
            .expect("broker constructs closed reply"),
        )
    }
    fn active(
        &self,
        membership: SemanticMembershipRefV1,
        epoch: SemanticSynchronizationEpochV1,
    ) -> bool {
        matches!(self.synchronization, SynchronizationState::Active { owner, epoch: active, .. } if owner == membership && active == epoch)
    }
    fn stage_within_limits(
        &self,
        replacing: Option<SemanticSynchronizationDocumentSealV1>,
        candidate: SemanticSynchronizationDocumentSealV1,
    ) -> bool {
        let projected_count = self.seals.len() + usize::from(replacing.is_none());
        if projected_count > self.limits.max_documents as usize {
            return false;
        }
        let total = self
            .seals
            .values()
            .try_fold(0u64, |sum, seal| sum.checked_add(seal.source_bytes()));
        total
            .and_then(|sum| sum.checked_sub(replacing.map_or(0, |seal| seal.source_bytes())))
            .and_then(|sum| sum.checked_add(candidate.source_bytes()))
            .is_some_and(|sum| sum <= self.limits.max_total_snapshot_bytes as u64)
    }
    fn sync_effect_pending(&self) -> bool {
        self.pending.values().any(|effect| {
            matches!(
                effect.command.as_ref(),
                Some(
                    SemanticSessionCommandV1::SyncOpen { .. }
                        | SemanticSessionCommandV1::SyncChange { .. }
                        | SemanticSessionCommandV1::SyncClose { .. }
                )
            )
        })
    }
    fn sync_refusal(&self) -> SemanticSessionOutcomeV1 {
        SemanticSessionOutcomeV1::Refused(
            if matches!(self.synchronization, SynchronizationState::Unavailable) {
                SemanticSessionRefusalV1::SynchronizationRequired
            } else {
                SemanticSessionRefusalV1::SynchronizationConflict
            },
        )
    }
    fn can_charge(&self, bytes: usize) -> bool {
        self.usage
            .replay_intent_bytes
            .checked_add(bytes)
            .is_some_and(|total| total <= self.limits.max_replay_intent_bytes)
    }
    fn charge(&mut self, bytes: usize) {
        self.usage.replay_intent_bytes = self.usage.replay_intent_bytes.saturating_add(bytes);
        self.usage.replay_intent_bytes_high_water = self
            .usage
            .replay_intent_bytes_high_water
            .max(self.usage.replay_intent_bytes);
        let entries = self.joins.len()
            + self
                .memberships
                .values()
                .map(|member| member.operations.len())
                .sum::<usize>();
        self.usage.operation_entry_high_water = self
            .usage
            .operation_entry_high_water
            .max(usize_to_u32(entries));
    }
    fn allocate_membership(&mut self) -> Option<SemanticMembershipRefV1> {
        while self.next_identity != 0 {
            let current = self.next_identity;
            self.next_identity = self.next_identity.checked_add(1).unwrap_or(0);
            if let Some(value) = SemanticMembershipRefV1::new(current.to_be_bytes())
                && !self.memberships.contains_key(&value)
            {
                return Some(value);
            }
        }
        None
    }
    fn allocate_request(&mut self) -> Option<SemanticRequestRefV1> {
        if self.next_request == 0 {
            return None;
        }
        let current = self.next_request;
        self.next_request = self.next_request.checked_add(1).unwrap_or(0);
        Some(SemanticRequestRefV1::new(current.to_be_bytes()))
    }
    fn mark_retired(&mut self, membership: SemanticMembershipRefV1) {
        if let Some(member) = self.memberships.get_mut(&membership)
            && !member.retired
        {
            member.retired = true;
            self.usage.current_memberships = self.usage.current_memberships.saturating_sub(1);
        }
    }
    fn retirement_obligations(&self, membership: SemanticMembershipRefV1) -> (u32, u32) {
        let mut requests = self.memberships.get(&membership).map_or(0, |member| {
            member
                .requests
                .values()
                .filter(|record| {
                    matches!(
                        record.state,
                        RequestState::Submitting | RequestState::Pending
                    )
                })
                .count() as u32
        });
        let mut documents = if matches!(self.synchronization, SynchronizationState::Active { owner, .. } | SynchronizationState::Ready { owner, .. } | SynchronizationState::Cleaning { owner } if owner == membership)
        {
            let cleanup_only = self
                .cleanup_seals
                .keys()
                .filter(|document| !self.seals.contains_key(document))
                .count();
            let in_flight = self
                .pending
                .values()
                .filter(|effect| {
                    effect.membership == membership
                        && matches!(
                            effect.command.as_ref(),
                            Some(
                                SemanticSessionCommandV1::SyncOpen { .. }
                                    | SemanticSessionCommandV1::SyncChange { .. }
                                    | SemanticSessionCommandV1::SyncClose { .. }
                            )
                        )
                })
                .count();
            u32::try_from(self.seals.len() + cleanup_only + in_flight).unwrap_or(u32::MAX)
        } else {
            0
        };
        for effect in self.pending.values().filter(|effect| {
            effect.membership == membership
                && effect.command.as_ref().is_none_or(|command| {
                    matches!(command, SemanticSessionCommandV1::LeaveSession { .. })
                })
        }) {
            if effect.cleanup_internal.is_some() {
                requests = requests.max(1);
            }
            if effect.seal.is_some() {
                documents = documents.max(1);
            }
        }
        (documents, requests)
    }
}

fn intent_bytes(command: &SemanticSessionCommandV1) -> usize {
    command
        .retained_intent_bytes_v1()
        .expect("bounded protocol command has a finite logical size")
}
fn result_matches_query(
    result: &vellum_host::semantic::SemanticResultV1,
    kind: vellum_host::semantic::SemanticQueryKindV1,
) -> bool {
    use vellum_host::semantic::SemanticQueryKindV1 as K;
    match kind {
        K::Symbols => result.symbols().is_some(),
        K::Definition | K::References | K::Implementations => result.locations().is_some(),
        K::Diagnostics => result.diagnostics().is_some(),
        K::TypeAtPoint => result.type_at_point().is_some(),
        K::PlanRename => result.rename_plan().is_some(),
    }
}
fn result_locations_valid(
    result: &vellum_host::semantic::SemanticResultV1,
    request: &SemanticRequestV1,
    witnesses: &BTreeMap<vellum_host::semantic::SemanticDocumentRefV1, DocumentWitness>,
) -> bool {
    let valid = |location: &vellum_host::semantic::SemanticLocationV1| {
        witnesses
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
fn query_offset_valid(
    query: &vellum_host::semantic::SemanticQueryV1,
    witness: &DocumentWitness,
) -> bool {
    use vellum_host::semantic::SemanticQueryV1 as Q;
    match query {
        Q::Definition { offset }
        | Q::References { offset }
        | Q::Implementations { offset }
        | Q::TypeAtPoint { offset }
        | Q::PlanRename { offset, .. } => witness.contains_span(*offset, *offset),
        Q::Symbols { .. } | Q::Diagnostics => true,
    }
}
fn bitmap_boundary(bitmap: &[u8], offset: usize) -> bool {
    bitmap
        .get(offset / 8)
        .is_some_and(|byte| byte & (1 << (offset % 8)) != 0)
}
fn same_intent(left: &SemanticSessionCommandV1, right: &SemanticSessionCommandV1) -> bool {
    use SemanticSessionCommandV1 as C;
    match (left, right) {
        (
            C::JoinSession {
                operation: a,
                incarnation: b,
                requested_roles: c,
                ..
            },
            C::JoinSession {
                operation: x,
                incarnation: y,
                requested_roles: z,
                ..
            },
        ) => (a, b, c) == (x, y, z),
        (
            C::BeginSynchronization {
                operation: a,
                incarnation: b,
                membership: c,
                epoch: d,
                manifest: e,
                ..
            },
            C::BeginSynchronization {
                operation: x,
                incarnation: y,
                membership: z,
                epoch: q,
                manifest: r,
                ..
            },
        ) => (a, b, c, d, e) == (x, y, z, q, r),
        (
            C::SyncOpen {
                operation: a,
                incarnation: b,
                membership: c,
                epoch: d,
                snapshot: e,
                ..
            },
            C::SyncOpen {
                operation: x,
                incarnation: y,
                membership: z,
                epoch: q,
                snapshot: r,
                ..
            },
        )
        | (
            C::SyncChange {
                operation: a,
                incarnation: b,
                membership: c,
                epoch: d,
                snapshot: e,
                ..
            },
            C::SyncChange {
                operation: x,
                incarnation: y,
                membership: z,
                epoch: q,
                snapshot: r,
                ..
            },
        ) => (a, b, c, d, e) == (x, y, z, q, r),
        (
            C::SyncClose {
                operation: a,
                incarnation: b,
                membership: c,
                epoch: d,
                target: e,
                ..
            },
            C::SyncClose {
                operation: x,
                incarnation: y,
                membership: z,
                epoch: q,
                target: r,
                ..
            },
        ) => (a, b, c, d, e) == (x, y, z, q, r),
        (
            C::CompleteSynchronization {
                operation: a,
                incarnation: b,
                membership: c,
                epoch: d,
                manifest: e,
                ..
            },
            C::CompleteSynchronization {
                operation: x,
                incarnation: y,
                membership: z,
                epoch: q,
                manifest: r,
                ..
            },
        ) => (a, b, c, d, e) == (x, y, z, q, r),
        (
            C::Submit {
                operation: a,
                incarnation: b,
                membership: c,
                request: d,
                ..
            },
            C::Submit {
                operation: x,
                incarnation: y,
                membership: z,
                request: q,
                ..
            },
        ) => (a, b, c, d) == (x, y, z, q),
        (
            C::Cancel {
                operation: a,
                incarnation: b,
                membership: c,
                request: d,
                ..
            },
            C::Cancel {
                operation: x,
                incarnation: y,
                membership: z,
                request: q,
                ..
            },
        )
        | (
            C::ReleaseRequest {
                operation: a,
                incarnation: b,
                membership: c,
                request: d,
                ..
            },
            C::ReleaseRequest {
                operation: x,
                incarnation: y,
                membership: z,
                request: q,
                ..
            },
        ) => (a, b, c, d) == (x, y, z, q),
        (
            C::LeaveSession {
                operation: a,
                incarnation: b,
                membership: c,
                ..
            },
            C::LeaveSession {
                operation: x,
                incarnation: y,
                membership: z,
                ..
            },
        ) => (a, b, c) == (x, y, z),
        _ => false,
    }
}
fn bump(value: &mut u64, saturated: &mut bool) {
    if let Some(next) = value.checked_add(1) {
        *value = next
    } else {
        *saturated = true
    }
}
fn bump_slot(values: &mut [u64], index: usize) -> bool {
    let prior = values[index];
    values[index] = prior.saturating_add(1);
    values[index] == u64::MAX && prior == u64::MAX
}
fn usize_to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
fn semantic_outcome_index(outcome: &SemanticSessionOutcomeV1) -> usize {
    match outcome {
        SemanticSessionOutcomeV1::Description(_) => 0,
        SemanticSessionOutcomeV1::Joined { .. } => 1,
        SemanticSessionOutcomeV1::Accepted => 2,
        SemanticSessionOutcomeV1::Pending(_) => 3,
        SemanticSessionOutcomeV1::Complete(_) => 4,
        SemanticSessionOutcomeV1::Unsupported(_) => 5,
        SemanticSessionOutcomeV1::Stale => 6,
        SemanticSessionOutcomeV1::Unavailable => 7,
        SemanticSessionOutcomeV1::Cancelled => 8,
        SemanticSessionOutcomeV1::TimedOut => 9,
        SemanticSessionOutcomeV1::PeerRefused(_) => 10,
        SemanticSessionOutcomeV1::Refused(_) => 11,
        SemanticSessionOutcomeV1::RequestConflict => 12,
        SemanticSessionOutcomeV1::ReplayCapacity => 13,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use vellum_host::semantic::{
        SemanticDocumentRefV1, SemanticProviderRefV1, SemanticSourceRevisionV1,
    };

    fn id<T>(make: impl FnOnce([u8; 16]) -> T, tag: u8) -> T {
        make([tag; 16])
    }
    fn context(roles: SemanticSessionRolesV1) -> SemanticBrokerContextV1 {
        SemanticBrokerContextV1 {
            subject: AuthoritySubjectRefV1::new([1; 16]),
            session: AuthoritySessionRefV1::new([2; 16]),
            allowed_roles: roles,
            policy_epoch: 1,
        }
    }
    fn broker() -> SemanticSessionBrokerV1 {
        SemanticSessionBrokerV1::try_new(
            id(|v| SemanticSessionIncarnationV1::new(v).unwrap(), 3),
            SemanticSessionLimitsV1::reference(),
        )
        .unwrap()
    }
    fn join(exchange: u8) -> SemanticSessionCommandV1 {
        SemanticSessionCommandV1::JoinSession {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), exchange),
            operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 5),
            incarnation: id(|v| SemanticSessionIncarnationV1::new(v).unwrap(), 3),
            requested_roles: SemanticSessionRolesV1::top(),
        }
    }
    fn joined(decision: SemanticBrokerDecisionV1) -> SemanticMembershipRefV1 {
        let SemanticBrokerDecisionV1::Reply(reply) = decision else {
            panic!()
        };
        let SemanticSessionOutcomeV1::Joined { membership, .. } = reply.outcome() else {
            panic!()
        };
        *membership
    }
    fn provider() -> SemanticProviderProvenanceV1 {
        SemanticProviderProvenanceV1::try_new(
            SemanticProviderRefV1::new([42; 16]),
            "test".into(),
            "1".into(),
        )
        .unwrap()
    }
    fn invalidations(
        broker: &SemanticSessionBrokerV1,
        cause: SemanticInvalidationCauseV1,
        requests: impl IntoIterator<Item = SemanticRequestRefV1>,
    ) -> SemanticInvalidationReceiptV1 {
        let mut requests: Vec<_> = requests.into_iter().collect();
        requests.sort();
        SemanticInvalidationReceiptV1::try_new(
            cause,
            requests,
            u64::from(broker.limits.max_pending),
        )
        .unwrap()
    }
    fn provider_binding(broker: &SemanticSessionBrokerV1) -> SemanticBrokerProviderBindingV1 {
        SemanticBrokerProviderBindingV1 {
            epoch: broker.provider.epoch,
            observation: broker.provider.observation,
            provenance: broker.provider.provenance.clone(),
        }
    }
    fn complete_admission(
        broker: &mut SemanticSessionBrokerV1,
        continuation: SemanticBrokerAdmissionContinuationV1,
        outcome: SemanticBrokerAdmissionOutcomeV1,
    ) -> SemanticBrokerDecisionV1 {
        let expectation = broker.pending[&continuation.0]
            .transition
            .as_ref()
            .expect("admission effects capture transition evidence")
            .clone();
        match expectation.source.kind() {
            ExpectedSourceKind::Open => broker.event(SemanticBrokerEventV1::OpenCompleted {
                continuation,
                report: SemanticBrokerOpenReportV1 {
                    provider_before: provider_binding(broker),
                    pre_provider: None,
                    outcome,
                    post_provider: None,
                },
            }),
            ExpectedSourceKind::Change | ExpectedSourceKind::Close => {
                let outcome = match outcome {
                    SemanticBrokerAdmissionOutcomeV1::Accepted => {
                        let cause = if expectation.source.kind() == ExpectedSourceKind::Change {
                            SemanticInvalidationCauseV1::SourceChanged
                        } else {
                            SemanticInvalidationCauseV1::SourceClosed
                        };
                        let requests = expectation
                            .requests
                            .iter()
                            .filter(|request| request.source_affected)
                            .map(|request| request.internal);
                        SemanticBrokerSourceAdmissionOutcomeV1::Accepted(invalidations(
                            broker, cause, requests,
                        ))
                    }
                    SemanticBrokerAdmissionOutcomeV1::Unavailable => {
                        SemanticBrokerSourceAdmissionOutcomeV1::Unavailable
                    }
                    SemanticBrokerAdmissionOutcomeV1::Refused(reason) => {
                        SemanticBrokerSourceAdmissionOutcomeV1::Refused(reason)
                    }
                };
                let event = if expectation.source.kind() == ExpectedSourceKind::Change {
                    SemanticBrokerEventV1::ChangeCompleted {
                        continuation,
                        report: SemanticBrokerSourceReportV1 {
                            provider_before: provider_binding(broker),
                            pre_provider: None,
                            outcome,
                            post_provider: None,
                        },
                    }
                } else {
                    SemanticBrokerEventV1::CloseCompleted {
                        continuation,
                        report: SemanticBrokerSourceReportV1 {
                            provider_before: provider_binding(broker),
                            pre_provider: None,
                            outcome,
                            post_provider: None,
                        },
                    }
                };
                broker.event(event)
            }
            ExpectedSourceKind::None => panic!("not an admission effect"),
        }
    }
    fn complete_submission(
        broker: &mut SemanticSessionBrokerV1,
        continuation: SemanticBrokerSubmissionContinuationV1,
        outcome: SemanticBrokerSubmissionOutcomeV1,
    ) -> SemanticBrokerDecisionV1 {
        let decision = broker.event(SemanticBrokerEventV1::SubmissionCompleted {
            continuation,
            report: SemanticBrokerSubmissionReportV1 {
                provider_before: provider_binding(broker),
                pre_provider: None,
                outcome,
                post_provider: None,
            },
        });
        assert!(
            !matches!(decision, SemanticBrokerDecisionV1::Diverged(_)),
            "unexpected submission divergence: {decision:?}"
        );
        decision
    }
    fn complete_cancellation(
        broker: &mut SemanticSessionBrokerV1,
        continuation: SemanticBrokerCancellationContinuationV1,
        outcome: SemanticBrokerCancellationOutcomeV1,
    ) -> SemanticBrokerDecisionV1 {
        broker.event(SemanticBrokerEventV1::CancellationCompleted {
            continuation,
            report: SemanticBrokerCancellationReportV1 {
                provider_before: provider_binding(broker),
                pre_provider: None,
                outcome,
                post_provider: None,
            },
        })
    }
    fn complete_delivery(
        broker: &mut SemanticSessionBrokerV1,
        internal_request: SemanticRequestRefV1,
        outcome: SemanticBrokerDeliveryOutcomeV1,
    ) -> SemanticBrokerDecisionV1 {
        broker.event(SemanticBrokerEventV1::ProviderDriveCompleted {
            report: Box::new(SemanticBrokerProviderDriveReportV1 {
                provider_before: provider_binding(broker),
                pre_provider: None,
                disposition: SemanticBrokerProviderDriveDispositionV1::Delivery {
                    internal_request,
                    outcome,
                },
                post_provider: None,
            }),
        })
    }
    fn complete_expiry(
        broker: &mut SemanticSessionBrokerV1,
        internal_request: SemanticRequestRefV1,
        outcome: SemanticBrokerExpiryOutcomeV1,
    ) -> SemanticBrokerDecisionV1 {
        broker.event(SemanticBrokerEventV1::ExpiryDriveCompleted {
            report: Box::new(SemanticBrokerExpiryReportV1 {
                provider_before: provider_binding(broker),
                pre_provider: None,
                internal_request,
                outcome,
                post_provider: None,
            }),
        })
    }
    fn complete_provider_transition(
        broker: &mut SemanticSessionBrokerV1,
        transition: SemanticBrokerProviderTransitionV1,
    ) -> SemanticBrokerDecisionV1 {
        broker.event(SemanticBrokerEventV1::ProviderDriveCompleted {
            report: Box::new(SemanticBrokerProviderDriveReportV1 {
                provider_before: provider_binding(broker),
                pre_provider: Some(transition),
                disposition: SemanticBrokerProviderDriveDispositionV1::NoEvent,
                post_provider: None,
            }),
        })
    }
    fn complete_late(
        broker: &mut SemanticSessionBrokerV1,
        internal_request: SemanticRequestRefV1,
    ) -> SemanticBrokerDecisionV1 {
        broker.event(SemanticBrokerEventV1::ProviderDriveCompleted {
            report: Box::new(SemanticBrokerProviderDriveReportV1 {
                provider_before: provider_binding(broker),
                pre_provider: None,
                disposition: SemanticBrokerProviderDriveDispositionV1::Late { internal_request },
                post_provider: None,
            }),
        })
    }
    fn install_ready_provider(broker: &mut SemanticSessionBrokerV1, epoch: u64) {
        let provider_before = provider_binding(broker);
        let requests = broker
            .capture_transition_expectation(ExpectedSourceEffect::None)
            .requests
            .into_iter()
            .filter(|request| request.provider_epoch < epoch)
            .map(|request| request.internal);
        let pre_invalidations = invalidations(
            broker,
            SemanticInvalidationCauseV1::ProviderStateChanged,
            requests,
        );
        assert_eq!(
            broker.event(SemanticBrokerEventV1::ProviderDriveCompleted {
                report: Box::new(SemanticBrokerProviderDriveReportV1 {
                    provider_before,
                    pre_provider: Some(SemanticBrokerProviderTransitionV1 {
                        epoch,
                        observed_observation: SemanticProviderObservationV1::new(
                            SemanticProviderReadinessProjectionV1::Ready {
                                generation: SemanticIndexGenerationV1::new(epoch).unwrap(),
                            },
                            SemanticCapabilitySetV1::top(),
                        ),
                        observed_provenance: Some(provider()),
                        settled_observation: SemanticProviderObservationV1::new(
                            SemanticProviderReadinessProjectionV1::Ready {
                                generation: SemanticIndexGenerationV1::new(epoch).unwrap(),
                            },
                            SemanticCapabilitySetV1::top(),
                        ),
                        settled_provenance: Some(provider()),
                        invalidations: pre_invalidations,
                    }),
                    disposition: SemanticBrokerProviderDriveDispositionV1::NoEvent,
                    post_provider: None,
                }),
            }),
            SemanticBrokerDecisionV1::EventAccepted
        );
    }
    fn pending_request(
        query: vellum_host::semantic::SemanticQueryV1,
        source: &str,
    ) -> (
        SemanticSessionBrokerV1,
        SemanticBrokerContextV1,
        SemanticMembershipRefV1,
        SemanticRequestRefV1,
        SemanticRequestRefV1,
        SemanticTargetV1,
    ) {
        use vellum_host::semantic::SemanticTextV1;
        let mut broker = broker();
        install_ready_provider(&mut broker, 1);
        let ctx = context(SemanticSessionRolesV1::top());
        let membership = joined(broker.command(ctx, join(100)));
        let epoch = id(|v| SemanticSynchronizationEpochV1::new(v).unwrap(), 101);
        let target = SemanticTargetV1 {
            document: SemanticDocumentRefV1::new([102; 16]),
            source_revision: SemanticSourceRevisionV1::new(1),
        };
        let snapshot = SemanticSessionSnapshotV1::try_new(
            target,
            "rust".into(),
            source.into(),
            broker.limits(),
        )
        .unwrap();
        let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
            std::slice::from_ref(&snapshot),
            broker.limits(),
        )
        .unwrap();
        broker.command(
            ctx,
            SemanticSessionCommandV1::BeginSynchronization {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 103),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 104),
                incarnation: broker.incarnation(),
                membership,
                epoch,
                manifest,
            },
        );
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Open { continuation, .. }) =
            broker.command(
                ctx,
                SemanticSessionCommandV1::SyncOpen {
                    exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 105),
                    operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 106),
                    incarnation: broker.incarnation(),
                    membership,
                    epoch,
                    snapshot,
                },
            )
        else {
            panic!()
        };
        complete_admission(
            &mut broker,
            continuation,
            SemanticBrokerAdmissionOutcomeV1::Accepted,
        );
        broker.command(
            ctx,
            SemanticSessionCommandV1::CompleteSynchronization {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 107),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 108),
                incarnation: broker.incarnation(),
                membership,
                epoch,
                manifest,
            },
        );
        let external = SemanticRequestRefV1::new([109; 16]);
        let request = SemanticRequestV1 {
            request: external,
            target,
            expected_index: None,
            language: SemanticTextV1::try_new("rust".into(), 32).unwrap(),
            query,
        };
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Submit {
            continuation,
            request,
        }) = broker.command(
            ctx,
            SemanticSessionCommandV1::Submit {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 110),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 111),
                incarnation: broker.incarnation(),
                membership,
                request,
            },
        )
        else {
            panic!()
        };
        let internal = request.request;
        assert_eq!(
            request.expected_index,
            Some(SemanticIndexGenerationV1::new(1).unwrap())
        );
        complete_submission(
            &mut broker,
            continuation,
            SemanticBrokerSubmissionOutcomeV1::Accepted,
        );
        (broker, ctx, membership, external, internal, target)
    }

    fn staged_change_with_pending() -> (
        SemanticSessionBrokerV1,
        SemanticBrokerContextV1,
        SemanticBrokerAdmissionContinuationV1,
        SemanticRequestRefV1,
    ) {
        use vellum_host::semantic::SemanticQueryV1;
        let (mut broker, ctx, membership, _, internal, old_target) =
            pending_request(SemanticQueryV1::Diagnostics, "abc");
        let epoch = id(|v| SemanticSynchronizationEpochV1::new(v).unwrap(), 210);
        let changed = SemanticSessionSnapshotV1::try_new(
            SemanticTargetV1 {
                document: old_target.document,
                source_revision: SemanticSourceRevisionV1::new(2),
            },
            "rust".into(),
            "abcd".into(),
            broker.limits(),
        )
        .unwrap();
        let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
            std::slice::from_ref(&changed),
            broker.limits(),
        )
        .unwrap();
        let _ = broker.command(
            ctx,
            SemanticSessionCommandV1::BeginSynchronization {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 211),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 212),
                incarnation: broker.incarnation(),
                membership,
                epoch,
                manifest,
            },
        );
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Change {
            continuation, ..
        }) = broker.command(
            ctx,
            SemanticSessionCommandV1::SyncChange {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 213),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 214),
                incarnation: broker.incarnation(),
                membership,
                epoch,
                snapshot: changed,
            },
        )
        else {
            panic!()
        };
        (broker, ctx, continuation, internal)
    }

    #[test]
    fn genesis_provider_binding_accepts_exact_closed_projection_states() {
        let states = [
            (
                SemanticProviderObservationV1::new(
                    SemanticProviderReadinessProjectionV1::Unavailable,
                    SemanticCapabilitySetV1::bottom(),
                ),
                None,
            ),
            (
                SemanticProviderObservationV1::new(
                    SemanticProviderReadinessProjectionV1::Indexing,
                    SemanticCapabilitySetV1::bottom(),
                ),
                None,
            ),
            (
                SemanticProviderObservationV1::new(
                    SemanticProviderReadinessProjectionV1::Ready {
                        generation: SemanticIndexGenerationV1::new(1).unwrap(),
                    },
                    SemanticCapabilitySetV1::top(),
                ),
                Some(provider()),
            ),
        ];

        for (index, (observation, provenance)) in states.into_iter().enumerate() {
            let mut broker = SemanticSessionBrokerV1::try_new_with_provider_binding(
                id(|value| SemanticSessionIncarnationV1::new(value).unwrap(), 3),
                SemanticSessionLimitsV1::reference(),
                SemanticBrokerProviderBindingV1 {
                    epoch: 0,
                    observation,
                    provenance: provenance.clone(),
                },
            )
            .unwrap();
            assert_eq!(broker.provider.epoch, 0);
            assert_eq!(broker.provider.observation, observation);
            assert_eq!(broker.provider.provenance, provenance);

            let SemanticBrokerDecisionV1::Reply(reply) = broker.command(
                context(SemanticSessionRolesV1::top()),
                SemanticSessionCommandV1::Describe {
                    exchange: id(
                        |value| SemanticExchangeRefV1::new(value).unwrap(),
                        10 + u8::try_from(index).unwrap(),
                    ),
                },
            ) else {
                panic!("genesis describe must reply")
            };
            let SemanticSessionOutcomeV1::Description(description) = reply.outcome() else {
                panic!("describe must return a description")
            };
            assert_eq!(description.provider(), observation);
        }
    }

    #[test]
    fn genesis_provider_binding_rejects_nonzero_or_unclosed_states() {
        let unavailable = SemanticProviderObservationV1::new(
            SemanticProviderReadinessProjectionV1::Unavailable,
            SemanticCapabilitySetV1::bottom(),
        );
        let indexing = SemanticProviderObservationV1::new(
            SemanticProviderReadinessProjectionV1::Indexing,
            SemanticCapabilitySetV1::bottom(),
        );
        let ready = SemanticProviderObservationV1::new(
            SemanticProviderReadinessProjectionV1::Ready {
                generation: SemanticIndexGenerationV1::new(1).unwrap(),
            },
            SemanticCapabilitySetV1::top(),
        );
        let invalid = [
            SemanticBrokerProviderBindingV1 {
                epoch: 1,
                observation: unavailable,
                provenance: None,
            },
            SemanticBrokerProviderBindingV1 {
                epoch: 0,
                observation: unavailable,
                provenance: Some(provider()),
            },
            SemanticBrokerProviderBindingV1 {
                epoch: 0,
                observation: indexing,
                provenance: Some(provider()),
            },
            SemanticBrokerProviderBindingV1 {
                epoch: 0,
                observation: SemanticProviderObservationV1::new(
                    SemanticProviderReadinessProjectionV1::Unavailable,
                    SemanticCapabilitySetV1::top(),
                ),
                provenance: None,
            },
            SemanticBrokerProviderBindingV1 {
                epoch: 0,
                observation: SemanticProviderObservationV1::new(
                    SemanticProviderReadinessProjectionV1::Indexing,
                    SemanticCapabilitySetV1::top(),
                ),
                provenance: None,
            },
            SemanticBrokerProviderBindingV1 {
                epoch: 0,
                observation: ready,
                provenance: None,
            },
        ];
        for binding in invalid {
            assert!(
                SemanticSessionBrokerV1::try_new_with_provider_binding(
                    id(|value| SemanticSessionIncarnationV1::new(value).unwrap(), 3),
                    SemanticSessionLimitsV1::reference(),
                    binding,
                )
                .is_none()
            );
        }
    }

    #[test]
    fn genesis_ready_binding_accepts_only_a_valid_exact_successor() {
        let ready = SemanticProviderObservationV1::new(
            SemanticProviderReadinessProjectionV1::Ready {
                generation: SemanticIndexGenerationV1::new(1).unwrap(),
            },
            SemanticCapabilitySetV1::top(),
        );
        let mut broker = SemanticSessionBrokerV1::try_new_with_provider_binding(
            id(|value| SemanticSessionIncarnationV1::new(value).unwrap(), 3),
            SemanticSessionLimitsV1::reference(),
            SemanticBrokerProviderBindingV1 {
                epoch: 0,
                observation: ready,
                provenance: Some(provider()),
            },
        )
        .unwrap();
        let indexing = SemanticProviderObservationV1::new(
            SemanticProviderReadinessProjectionV1::Indexing,
            SemanticCapabilitySetV1::bottom(),
        );
        let receipt = invalidations(
            &broker,
            SemanticInvalidationCauseV1::ProviderStateChanged,
            [],
        );
        assert_eq!(
            complete_provider_transition(
                &mut broker,
                SemanticBrokerProviderTransitionV1 {
                    epoch: 1,
                    observed_observation: indexing,
                    observed_provenance: None,
                    settled_observation: indexing,
                    settled_provenance: None,
                    invalidations: receipt,
                },
            ),
            SemanticBrokerDecisionV1::EventAccepted
        );
        assert_eq!(broker.provider.epoch, 1);
        assert_eq!(broker.provider.observation, indexing);
        assert_eq!(broker.provider.provenance, None);

        let poison = SemanticProviderObservationV1::new(
            SemanticProviderReadinessProjectionV1::Indexing,
            SemanticCapabilitySetV1::top(),
        );
        let receipt = invalidations(
            &broker,
            SemanticInvalidationCauseV1::ProviderStateChanged,
            [],
        );
        assert_eq!(
            complete_provider_transition(
                &mut broker,
                SemanticBrokerProviderTransitionV1 {
                    epoch: 2,
                    observed_observation: poison,
                    observed_provenance: None,
                    settled_observation: poison,
                    settled_provenance: None,
                    invalidations: receipt,
                },
            ),
            SemanticBrokerDecisionV1::Diverged(SemanticBrokerDivergenceV1::TransitionEvidence)
        );
        assert_eq!(broker.provider.epoch, 1);
        assert_eq!(broker.provider.observation, indexing);
    }

    #[test]
    fn join_is_policy_meet_and_replay_ignores_only_exchange() {
        let mut broker = broker();
        let membership = joined(broker.command(context(SemanticSessionRolesV1::query()), join(6)));
        let SemanticBrokerDecisionV1::Reply(replay) =
            broker.command(context(SemanticSessionRolesV1::query()), join(7))
        else {
            panic!()
        };
        assert_eq!(
            replay.exchange(),
            id(|v| SemanticExchangeRefV1::new(v).unwrap(), 7)
        );
        assert_eq!(replay.membership(), Some(membership));
        assert_eq!(broker.membership_count(), 1);
        assert_eq!(broker.usage().replay_hits, 1);
    }

    #[test]
    fn join_operation_reuse_with_changed_intent_conflicts_without_capacity_growth() {
        let mut broker = broker();
        joined(broker.command(context(SemanticSessionRolesV1::top()), join(6)));
        let mut changed = join(7);
        if let SemanticSessionCommandV1::JoinSession {
            requested_roles, ..
        } = &mut changed
        {
            *requested_roles = SemanticSessionRolesV1::query();
        }
        let SemanticBrokerDecisionV1::Reply(reply) =
            broker.command(context(SemanticSessionRolesV1::top()), changed)
        else {
            panic!()
        };
        assert_eq!(reply.outcome(), &SemanticSessionOutcomeV1::RequestConflict);
        assert_eq!(broker.membership_count(), 1);
    }

    #[test]
    fn replay_bytes_are_exactly_bounded_and_early_refusals_are_immutable() {
        let mut limits = SemanticSessionLimitsV1::reference();
        limits.max_replay_intent_bytes = 143;
        let mut bounded = SemanticSessionBrokerV1::try_new(
            id(|v| SemanticSessionIncarnationV1::new(v).unwrap(), 3),
            limits,
        )
        .unwrap();
        let ctx = context(SemanticSessionRolesV1::top());
        let membership = joined(bounded.command(ctx, join(6)));
        let epoch = id(|v| SemanticSynchronizationEpochV1::new(v).unwrap(), 7);
        let manifest =
            SemanticSynchronizationManifestV1::try_from_seals(&[], bounded.limits()).unwrap();
        let begin = SemanticSessionCommandV1::BeginSynchronization {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 8),
            operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 9),
            incarnation: bounded.incarnation(),
            membership,
            epoch,
            manifest,
        };
        let SemanticBrokerDecisionV1::Reply(accepted) = bounded.command(ctx, begin.clone()) else {
            panic!()
        };
        assert_eq!(accepted.outcome(), &SemanticSessionOutcomeV1::Accepted);
        assert_eq!(bounded.usage().replay_intent_bytes, 143);
        let mut replay = begin;
        if let SemanticSessionCommandV1::BeginSynchronization { exchange, .. } = &mut replay {
            *exchange = id(|v| SemanticExchangeRefV1::new(v).unwrap(), 10);
        }
        bounded.command(ctx, replay);
        assert_eq!(bounded.usage().replay_intent_bytes, 143);
        let SemanticBrokerDecisionV1::Reply(full) = bounded.command(
            ctx,
            SemanticSessionCommandV1::CompleteSynchronization {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 11),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 12),
                incarnation: bounded.incarnation(),
                membership,
                epoch,
                manifest,
            },
        ) else {
            panic!()
        };
        assert_eq!(full.outcome(), &SemanticSessionOutcomeV1::ReplayCapacity);

        let mut refused_broker = broker();
        let mut denied_context = context(SemanticSessionRolesV1::bottom());
        let mut requested_query = join(13);
        if let SemanticSessionCommandV1::JoinSession {
            requested_roles, ..
        } = &mut requested_query
        {
            *requested_roles = SemanticSessionRolesV1::query();
        }
        let SemanticBrokerDecisionV1::Reply(denied) =
            refused_broker.command(denied_context, requested_query.clone())
        else {
            panic!()
        };
        assert_eq!(
            denied.outcome(),
            &SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::RoleDenied)
        );
        denied_context.allowed_roles = SemanticSessionRolesV1::top();
        let SemanticBrokerDecisionV1::Reply(replayed) =
            refused_broker.command(denied_context, requested_query)
        else {
            panic!()
        };
        assert_eq!(replayed.outcome(), denied.outcome());
        assert_eq!(refused_broker.membership_count(), 0);
    }

    #[test]
    fn foreign_authenticated_context_coalesces_with_unknown_membership() {
        let mut broker = broker();
        let membership = joined(broker.command(context(SemanticSessionRolesV1::query()), join(6)));
        let mut foreign = context(SemanticSessionRolesV1::query());
        foreign.subject = AuthoritySubjectRefV1::new([9; 16]);
        let command = SemanticSessionCommandV1::LeaveSession {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 8),
            operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 9),
            incarnation: broker.incarnation(),
            membership,
        };
        let SemanticBrokerDecisionV1::Reply(reply) = broker.command(foreign, command) else {
            panic!()
        };
        assert_eq!(
            reply.outcome(),
            &SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::UnknownMembership)
        );
    }

    #[test]
    fn seal_commits_only_after_accepted_effect() {
        let mut broker = broker();
        let ctx = context(SemanticSessionRolesV1::synchronize());
        let membership = joined(broker.command(ctx, join(6)));
        let epoch = id(|v| SemanticSynchronizationEpochV1::new(v).unwrap(), 8);
        let target = SemanticTargetV1 {
            document: SemanticDocumentRefV1::new([10; 16]),
            source_revision: SemanticSourceRevisionV1::new(1),
        };
        let snapshot =
            SemanticSessionSnapshotV1::try_new(target, "rust".into(), "x".into(), broker.limits())
                .unwrap();
        let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
            std::slice::from_ref(&snapshot),
            broker.limits(),
        )
        .unwrap();
        let begin = SemanticSessionCommandV1::BeginSynchronization {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 11),
            operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 12),
            incarnation: broker.incarnation(),
            membership,
            epoch,
            manifest,
        };
        assert!(matches!(
            broker.command(ctx, begin),
            SemanticBrokerDecisionV1::Reply(_)
        ));
        let open = SemanticSessionCommandV1::SyncOpen {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 13),
            operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 14),
            incarnation: broker.incarnation(),
            membership,
            epoch,
            snapshot,
        };
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Open { continuation, .. }) =
            broker.command(ctx, open)
        else {
            panic!()
        };
        assert!(broker.seals.is_empty());
        assert!(matches!(
            complete_admission(
                &mut broker,
                continuation,
                SemanticBrokerAdmissionOutcomeV1::Unavailable
            ),
            SemanticBrokerDecisionV1::Reply(_)
        ));
        assert!(broker.seals.is_empty());
    }

    #[test]
    fn provider_state_requires_strict_epoch_and_real_state_change() {
        let mut first = broker();
        install_ready_provider(&mut first, 1);
        let unchanged = SemanticBrokerProviderTransitionV1 {
            epoch: 2,
            observed_observation: first.provider.observation,
            observed_provenance: first.provider.provenance.clone(),
            settled_observation: first.provider.observation,
            settled_provenance: first.provider.provenance.clone(),
            invalidations: invalidations(
                &first,
                SemanticInvalidationCauseV1::ProviderStateChanged,
                [],
            ),
        };
        assert_eq!(
            complete_provider_transition(&mut first, unchanged),
            SemanticBrokerDecisionV1::Diverged(SemanticBrokerDivergenceV1::TransitionEvidence)
        );

        let mut second = broker();
        install_ready_provider(&mut second, 1);
        let invalidations = invalidations(
            &second,
            SemanticInvalidationCauseV1::ProviderStateChanged,
            [],
        );
        assert_eq!(
            complete_provider_transition(
                &mut second,
                SemanticBrokerProviderTransitionV1 {
                    epoch: 3,
                    observed_observation: SemanticProviderObservationV1::new(
                        SemanticProviderReadinessProjectionV1::Unavailable,
                        SemanticCapabilitySetV1::bottom(),
                    ),
                    observed_provenance: None,
                    settled_observation: SemanticProviderObservationV1::new(
                        SemanticProviderReadinessProjectionV1::Unavailable,
                        SemanticCapabilitySetV1::bottom(),
                    ),
                    settled_provenance: None,
                    invalidations,
                },
            ),
            SemanticBrokerDecisionV1::Diverged(SemanticBrokerDivergenceV1::TransitionEvidence)
        );
    }

    #[test]
    fn provider_transition_accepts_observed_then_settled_aba_as_one_stage() {
        let mut broker = broker();
        install_ready_provider(&mut broker, 1);
        let settled_observation = broker.provider.observation;
        let settled_provenance = broker.provider.provenance.clone();
        let transition = SemanticBrokerProviderTransitionV1 {
            epoch: 2,
            observed_observation: SemanticProviderObservationV1::new(
                SemanticProviderReadinessProjectionV1::Unavailable,
                SemanticCapabilitySetV1::bottom(),
            ),
            observed_provenance: None,
            settled_observation,
            settled_provenance: settled_provenance.clone(),
            invalidations: invalidations(
                &broker,
                SemanticInvalidationCauseV1::ProviderStateChanged,
                [],
            ),
        };
        assert_eq!(
            complete_provider_transition(&mut broker, transition),
            SemanticBrokerDecisionV1::EventAccepted
        );
        assert_eq!(broker.provider.epoch, 2);
        assert_eq!(broker.provider.observation, settled_observation);
        assert_eq!(broker.provider.provenance, settled_provenance);
    }

    #[test]
    fn composite_provider_drive_routes_delivery_after_exact_transition_validation() {
        use vellum_host::semantic::SemanticQueryV1;

        let (mut broker, _, membership, external, internal, _) =
            pending_request(SemanticQueryV1::Diagnostics, "abc");
        assert_eq!(
            broker.event(SemanticBrokerEventV1::ProviderDriveCompleted {
                report: Box::new(SemanticBrokerProviderDriveReportV1 {
                    provider_before: provider_binding(&broker),
                    pre_provider: None,
                    disposition: SemanticBrokerProviderDriveDispositionV1::NoEvent,
                    post_provider: None,
                }),
            }),
            SemanticBrokerDecisionV1::EventAccepted
        );
        assert_eq!(
            broker.event(SemanticBrokerEventV1::ProviderDriveCompleted {
                report: Box::new(SemanticBrokerProviderDriveReportV1 {
                    provider_before: provider_binding(&broker),
                    pre_provider: None,
                    disposition: SemanticBrokerProviderDriveDispositionV1::Delivery {
                        internal_request: internal,
                        outcome: SemanticBrokerDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Stale),
                    },
                    post_provider: None,
                }),
            }),
            SemanticBrokerDecisionV1::EventAccepted
        );
        assert!(matches!(
            broker.memberships[&membership].requests[&external].state,
            RequestState::Terminal(SemanticSessionOutcomeV1::Stale)
        ));
    }

    #[test]
    fn composite_provider_drive_accepts_exact_late_and_orphan_but_poison_mismatch() {
        use vellum_host::semantic::SemanticQueryV1;

        let (mut broker, _, membership, external, internal, _) =
            pending_request(SemanticQueryV1::Diagnostics, "abc");
        let late_invalidations = invalidations(
            &broker,
            SemanticInvalidationCauseV1::ProviderStateChanged,
            [internal],
        );
        let report = SemanticBrokerProviderDriveReportV1 {
            provider_before: provider_binding(&broker),
            pre_provider: None,
            disposition: SemanticBrokerProviderDriveDispositionV1::Late {
                internal_request: internal,
            },
            post_provider: Some(SemanticBrokerProviderTransitionV1 {
                epoch: 2,
                observed_observation: SemanticProviderObservationV1::new(
                    SemanticProviderReadinessProjectionV1::Unavailable,
                    SemanticCapabilitySetV1::bottom(),
                ),
                observed_provenance: None,
                settled_observation: SemanticProviderObservationV1::new(
                    SemanticProviderReadinessProjectionV1::Unavailable,
                    SemanticCapabilitySetV1::bottom(),
                ),
                settled_provenance: None,
                invalidations: late_invalidations,
            }),
        };
        assert_eq!(
            broker.event(SemanticBrokerEventV1::ProviderDriveCompleted {
                report: Box::new(report),
            }),
            SemanticBrokerDecisionV1::EventAccepted
        );
        assert!(matches!(
            broker.memberships[&membership].requests[&external].state,
            RequestState::Terminal(SemanticSessionOutcomeV1::Stale)
        ));

        let orphan = SemanticRequestRefV1::new([249; 16]);
        assert_eq!(
            broker.event(SemanticBrokerEventV1::ProviderDriveCompleted {
                report: Box::new(SemanticBrokerProviderDriveReportV1 {
                    provider_before: provider_binding(&broker),
                    pre_provider: None,
                    disposition: SemanticBrokerProviderDriveDispositionV1::Orphan {
                        internal_request: orphan,
                    },
                    post_provider: None,
                }),
            }),
            SemanticBrokerDecisionV1::EventAccepted
        );

        let (mut poisoned, ctx, poisoned_membership, _, known, _) =
            pending_request(SemanticQueryV1::Diagnostics, "abc");
        let before = provider_binding(&poisoned);
        let poison_invalidations = invalidations(
            &poisoned,
            SemanticInvalidationCauseV1::ProviderStateChanged,
            [known],
        );
        assert_eq!(
            poisoned.event(SemanticBrokerEventV1::ProviderDriveCompleted {
                report: Box::new(SemanticBrokerProviderDriveReportV1 {
                    provider_before: before,
                    pre_provider: None,
                    disposition: SemanticBrokerProviderDriveDispositionV1::Orphan {
                        internal_request: known,
                    },
                    post_provider: Some(SemanticBrokerProviderTransitionV1 {
                        epoch: 2,
                        observed_observation: SemanticProviderObservationV1::new(
                            SemanticProviderReadinessProjectionV1::Unavailable,
                            SemanticCapabilitySetV1::bottom(),
                        ),
                        observed_provenance: None,
                        settled_observation: SemanticProviderObservationV1::new(
                            SemanticProviderReadinessProjectionV1::Unavailable,
                            SemanticCapabilitySetV1::bottom(),
                        ),
                        settled_provenance: None,
                        invalidations: poison_invalidations,
                    }),
                }),
            }),
            SemanticBrokerDecisionV1::Diverged(SemanticBrokerDivergenceV1::TransitionEvidence)
        );
        assert_eq!(poisoned.provider.epoch, 1);
        assert!(matches!(
            poisoned.memberships[&poisoned_membership]
                .requests
                .values()
                .next()
                .unwrap()
                .state,
            RequestState::Pending
        ));
        assert_eq!(
            poisoned.command(
                ctx,
                SemanticSessionCommandV1::Describe {
                    exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 248),
                }
            ),
            SemanticBrokerDecisionV1::Diverged(SemanticBrokerDivergenceV1::TransitionEvidence)
        );
    }

    #[test]
    fn ordered_expiry_closes_timeout_transition_races_and_atomic_poison() {
        use vellum_host::semantic::SemanticQueryV1;

        let (mut expired, _, membership, external, internal, _) =
            pending_request(SemanticQueryV1::Diagnostics, "abc");
        assert_eq!(
            complete_expiry(
                &mut expired,
                internal,
                SemanticBrokerExpiryOutcomeV1::Expired,
            ),
            SemanticBrokerDecisionV1::EventAccepted
        );
        assert!(matches!(
            expired.memberships[&membership].requests[&external].state,
            RequestState::Terminal(SemanticSessionOutcomeV1::TimedOut)
        ));
        assert_eq!(expired.usage().requests_expired, 1);
        assert_eq!(
            complete_expiry(
                &mut expired,
                internal,
                SemanticBrokerExpiryOutcomeV1::NotPending,
            ),
            SemanticBrokerDecisionV1::EventAccepted
        );
        assert_eq!(expired.usage().expiries_not_pending, 1);

        let (mut satisfied, _, membership, external, internal, _) =
            pending_request(SemanticQueryV1::Diagnostics, "abc");
        let pre_invalidations = invalidations(
            &satisfied,
            SemanticInvalidationCauseV1::ProviderStateChanged,
            [internal],
        );
        assert_eq!(
            satisfied.event(SemanticBrokerEventV1::ExpiryDriveCompleted {
                report: Box::new(SemanticBrokerExpiryReportV1 {
                    provider_before: provider_binding(&satisfied),
                    pre_provider: Some(SemanticBrokerProviderTransitionV1 {
                        epoch: 2,
                        observed_observation: SemanticProviderObservationV1::new(
                            SemanticProviderReadinessProjectionV1::Unavailable,
                            SemanticCapabilitySetV1::bottom(),
                        ),
                        observed_provenance: None,
                        settled_observation: SemanticProviderObservationV1::new(
                            SemanticProviderReadinessProjectionV1::Unavailable,
                            SemanticCapabilitySetV1::bottom(),
                        ),
                        settled_provenance: None,
                        invalidations: pre_invalidations,
                    }),
                    internal_request: internal,
                    outcome: SemanticBrokerExpiryOutcomeV1::SatisfiedByProviderTransition,
                    post_provider: None,
                }),
            }),
            SemanticBrokerDecisionV1::EventAccepted
        );
        assert!(matches!(
            satisfied.memberships[&membership].requests[&external].state,
            RequestState::Terminal(SemanticSessionOutcomeV1::Stale)
        ));
        assert_eq!(satisfied.usage().expiries_satisfied_by_transition, 1);

        let (mut poisoned, _, membership, external, internal, _) =
            pending_request(SemanticQueryV1::Diagnostics, "abc");
        let before = provider_binding(&poisoned);
        let wrong_post = invalidations(
            &poisoned,
            SemanticInvalidationCauseV1::ProviderStateChanged,
            [internal],
        );
        assert_eq!(
            poisoned.event(SemanticBrokerEventV1::ExpiryDriveCompleted {
                report: Box::new(SemanticBrokerExpiryReportV1 {
                    provider_before: before.clone(),
                    pre_provider: None,
                    internal_request: internal,
                    outcome: SemanticBrokerExpiryOutcomeV1::Expired,
                    post_provider: Some(SemanticBrokerProviderTransitionV1 {
                        epoch: 2,
                        observed_observation: SemanticProviderObservationV1::new(
                            SemanticProviderReadinessProjectionV1::Unavailable,
                            SemanticCapabilitySetV1::bottom(),
                        ),
                        observed_provenance: None,
                        settled_observation: SemanticProviderObservationV1::new(
                            SemanticProviderReadinessProjectionV1::Unavailable,
                            SemanticCapabilitySetV1::bottom(),
                        ),
                        settled_provenance: None,
                        invalidations: wrong_post,
                    }),
                }),
            }),
            SemanticBrokerDecisionV1::Diverged(SemanticBrokerDivergenceV1::TransitionEvidence)
        );
        assert_eq!(provider_binding(&poisoned), before);
        assert!(matches!(
            poisoned.memberships[&membership].requests[&external].state,
            RequestState::Pending
        ));
    }

    #[test]
    fn provider_failure_commits_exact_pre_transition_and_mismatch_is_atomic() {
        use vellum_host::semantic::SemanticQueryV1;

        let (mut broker, _, membership, external, internal, _) =
            pending_request(SemanticQueryV1::Diagnostics, "abc");
        let before = provider_binding(&broker);
        let pre_invalidations = invalidations(
            &broker,
            SemanticInvalidationCauseV1::ProviderStateChanged,
            [internal],
        );
        assert_eq!(
            broker.event(SemanticBrokerEventV1::ProviderDriveFailed {
                report: Box::new(SemanticBrokerProviderDriveFailureReportV1 {
                    provider_before: before,
                    pre_provider: Some(SemanticBrokerProviderTransitionV1 {
                        epoch: 2,
                        observed_observation: SemanticProviderObservationV1::new(
                            SemanticProviderReadinessProjectionV1::Unavailable,
                            SemanticCapabilitySetV1::bottom(),
                        ),
                        observed_provenance: None,
                        settled_observation: SemanticProviderObservationV1::new(
                            SemanticProviderReadinessProjectionV1::Unavailable,
                            SemanticCapabilitySetV1::bottom(),
                        ),
                        settled_provenance: None,
                        invalidations: pre_invalidations,
                    }),
                    failure: SemanticBrokerProviderDriveFailureClassV1::RuntimeUnavailable,
                }),
            }),
            SemanticBrokerDecisionV1::EventAccepted
        );
        assert_eq!(broker.provider.epoch, 2);
        assert!(matches!(
            broker.memberships[&membership].requests[&external].state,
            RequestState::Terminal(SemanticSessionOutcomeV1::Stale)
        ));
        assert_eq!(broker.usage().provider_drive_failures, 1);
        assert_eq!(broker.usage().provider_drive_failures_by_class, [0, 1, 0]);

        let (mut poisoned, _, membership, external, _, _) =
            pending_request(SemanticQueryV1::Diagnostics, "abc");
        let before = provider_binding(&poisoned);
        let empty = invalidations(
            &poisoned,
            SemanticInvalidationCauseV1::ProviderStateChanged,
            [],
        );
        assert_eq!(
            poisoned.event(SemanticBrokerEventV1::ProviderDriveFailed {
                report: Box::new(SemanticBrokerProviderDriveFailureReportV1 {
                    provider_before: before.clone(),
                    pre_provider: Some(SemanticBrokerProviderTransitionV1 {
                        epoch: 2,
                        observed_observation: SemanticProviderObservationV1::new(
                            SemanticProviderReadinessProjectionV1::Unavailable,
                            SemanticCapabilitySetV1::bottom(),
                        ),
                        observed_provenance: None,
                        settled_observation: SemanticProviderObservationV1::new(
                            SemanticProviderReadinessProjectionV1::Unavailable,
                            SemanticCapabilitySetV1::bottom(),
                        ),
                        settled_provenance: None,
                        invalidations: empty,
                    }),
                    failure: SemanticBrokerProviderDriveFailureClassV1::DeliveryRefused,
                }),
            }),
            SemanticBrokerDecisionV1::Diverged(SemanticBrokerDivergenceV1::TransitionEvidence)
        );
        assert_eq!(provider_binding(&poisoned), before);
        assert!(matches!(
            poisoned.memberships[&membership].requests[&external].state,
            RequestState::Pending
        ));
        assert_eq!(poisoned.usage().provider_drive_failures, 0);
    }

    #[test]
    fn change_receipt_mismatch_poison_is_atomic_and_fences_later_work() {
        for case in 0..3 {
            let (mut broker, ctx, continuation, internal) = staged_change_with_pending();
            let provider_before = provider_binding(&broker);
            let (cause, mut requests) = match case {
                0 => (SemanticInvalidationCauseV1::SourceChanged, vec![]),
                1 => (
                    SemanticInvalidationCauseV1::SourceChanged,
                    vec![internal, SemanticRequestRefV1::new([250; 16])],
                ),
                _ => (SemanticInvalidationCauseV1::SourceClosed, vec![internal]),
            };
            requests.sort();
            let receipt = SemanticInvalidationReceiptV1::try_new(
                cause,
                requests,
                u64::from(broker.limits.max_pending),
            )
            .unwrap();
            assert_eq!(
                broker.event(SemanticBrokerEventV1::ChangeCompleted {
                    continuation,
                    report: SemanticBrokerSourceReportV1 {
                        provider_before,
                        pre_provider: None,
                        outcome: SemanticBrokerSourceAdmissionOutcomeV1::Accepted(receipt),
                        post_provider: None,
                    },
                }),
                SemanticBrokerDecisionV1::Diverged(SemanticBrokerDivergenceV1::TransitionEvidence)
            );
            assert_eq!(broker.usage().pending_effects, 1);
            let (membership, external) = broker.reverse_requests[&internal];
            assert!(matches!(
                broker.memberships[&membership].requests[&external].state,
                RequestState::Pending
            ));
            assert_eq!(
                broker.command(
                    ctx,
                    SemanticSessionCommandV1::Describe {
                        exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 215),
                    },
                ),
                SemanticBrokerDecisionV1::Diverged(SemanticBrokerDivergenceV1::TransitionEvidence)
            );
        }
    }

    #[test]
    fn non_owner_complete_does_not_seize_or_clean_active_synchronization() {
        let mut broker = broker();
        let ctx = context(SemanticSessionRolesV1::synchronize());
        let owner = joined(broker.command(ctx, join(61)));
        let mut second_join = join(62);
        if let SemanticSessionCommandV1::JoinSession { operation, .. } = &mut second_join {
            *operation = id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 63);
        }
        let other = joined(broker.command(ctx, second_join));
        let epoch = id(|v| SemanticSynchronizationEpochV1::new(v).unwrap(), 64);
        let manifest =
            SemanticSynchronizationManifestV1::try_from_seals(&[], broker.limits()).unwrap();
        let begin = SemanticSessionCommandV1::BeginSynchronization {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 65),
            operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 66),
            incarnation: broker.incarnation(),
            membership: owner,
            epoch,
            manifest,
        };
        broker.command(ctx, begin);
        let complete = SemanticSessionCommandV1::CompleteSynchronization {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 67),
            operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 68),
            incarnation: broker.incarnation(),
            membership: other,
            epoch,
            manifest,
        };
        let SemanticBrokerDecisionV1::Reply(reply) = broker.command(ctx, complete) else {
            panic!()
        };
        assert_eq!(
            reply.outcome(),
            &SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::SynchronizationConflict)
        );
        assert!(matches!(
            broker.synchronization,
            SynchronizationState::Active { owner: active, .. } if active == owner
        ));
    }

    #[test]
    fn leave_refuses_while_open_effect_is_pending_then_retries_after_completion() {
        let mut broker = broker();
        let ctx = context(SemanticSessionRolesV1::synchronize());
        let membership = joined(broker.command(ctx, join(69)));
        let epoch = id(|v| SemanticSynchronizationEpochV1::new(v).unwrap(), 70);
        let target = SemanticTargetV1 {
            document: SemanticDocumentRefV1::new([71; 16]),
            source_revision: SemanticSourceRevisionV1::new(1),
        };
        let snapshot = SemanticSessionSnapshotV1::try_new(
            target,
            "rust".into(),
            "late".into(),
            broker.limits(),
        )
        .unwrap();
        let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
            std::slice::from_ref(&snapshot),
            broker.limits(),
        )
        .unwrap();
        broker.command(
            ctx,
            SemanticSessionCommandV1::BeginSynchronization {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 72),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 73),
                incarnation: broker.incarnation(),
                membership,
                epoch,
                manifest,
            },
        );
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Open { continuation, .. }) =
            broker.command(
                ctx,
                SemanticSessionCommandV1::SyncOpen {
                    exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 74),
                    operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 75),
                    incarnation: broker.incarnation(),
                    membership,
                    epoch,
                    snapshot,
                },
            )
        else {
            panic!()
        };
        let SemanticBrokerDecisionV1::Reply(pending) = broker.command(
            ctx,
            SemanticSessionCommandV1::LeaveSession {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 76),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 77),
                incarnation: broker.incarnation(),
                membership,
            },
        ) else {
            panic!()
        };
        assert!(matches!(
            pending.outcome(),
            SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::ProtocolViolation)
        ));
        complete_admission(
            &mut broker,
            continuation,
            SemanticBrokerAdmissionOutcomeV1::Accepted,
        );
        assert_eq!(
            broker.seals.get(&target.document).map(|seal| seal.target()),
            Some(target)
        );
        assert!(broker.cleanup_seals.is_empty());
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Close {
            continuation,
            target: cleanup_target,
        }) = broker.command(
            ctx,
            SemanticSessionCommandV1::LeaveSession {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 78),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 79),
                incarnation: broker.incarnation(),
                membership,
            },
        )
        else {
            panic!()
        };
        assert_eq!(cleanup_target, target);
        let SemanticBrokerDecisionV1::Reply(retired) = complete_admission(
            &mut broker,
            continuation,
            SemanticBrokerAdmissionOutcomeV1::Accepted,
        ) else {
            panic!()
        };
        assert_eq!(retired.outcome(), &SemanticSessionOutcomeV1::Accepted);
        assert!(matches!(
            broker.synchronization,
            SynchronizationState::Unavailable
        ));
        let mut next_context = ctx;
        next_context.policy_epoch = 2;
        let mut rejoin = join(88);
        if let SemanticSessionCommandV1::JoinSession {
            operation,
            requested_roles,
            ..
        } = &mut rejoin
        {
            *operation = id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 89);
            *requested_roles = SemanticSessionRolesV1::synchronize();
        }
        let next_membership = joined(broker.command(next_context, rejoin));
        let SemanticBrokerDecisionV1::Reply(reused) = broker.command(
            next_context,
            SemanticSessionCommandV1::BeginSynchronization {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 90),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 91),
                incarnation: broker.incarnation(),
                membership: next_membership,
                epoch,
                manifest,
            },
        ) else {
            panic!()
        };
        assert_eq!(
            reused.outcome(),
            &SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::ProtocolViolation)
        );
    }

    #[test]
    fn empty_manifest_can_publish_ready_and_policy_loss_fences_it() {
        let mut broker = broker();
        let ctx = context(SemanticSessionRolesV1::synchronize());
        let membership = joined(broker.command(ctx, join(80)));
        let epoch = id(|v| SemanticSynchronizationEpochV1::new(v).unwrap(), 81);
        let manifest =
            SemanticSynchronizationManifestV1::try_from_seals(&[], broker.limits()).unwrap();
        broker.command(
            ctx,
            SemanticSessionCommandV1::BeginSynchronization {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 82),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 83),
                incarnation: broker.incarnation(),
                membership,
                epoch,
                manifest,
            },
        );
        let SemanticBrokerDecisionV1::Reply(complete) = broker.command(
            ctx,
            SemanticSessionCommandV1::CompleteSynchronization {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 84),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 85),
                incarnation: broker.incarnation(),
                membership,
                epoch,
                manifest,
            },
        ) else {
            panic!()
        };
        assert_eq!(complete.outcome(), &SemanticSessionOutcomeV1::Accepted);
        broker.event(SemanticBrokerEventV1::PolicyLost {
            subject: ctx.subject,
            session: ctx.session,
            policy_epoch: ctx.policy_epoch,
        });
        assert!(matches!(
            broker.synchronization,
            SynchronizationState::Cleaning { owner } if owner == membership
        ));
        let SemanticBrokerDecisionV1::Reply(left) = broker.command(
            ctx,
            SemanticSessionCommandV1::LeaveSession {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 86),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 87),
                incarnation: broker.incarnation(),
                membership,
            },
        ) else {
            panic!()
        };
        assert_eq!(left.outcome(), &SemanticSessionOutcomeV1::Accepted);
        assert!(matches!(
            broker.synchronization,
            SynchronizationState::Unavailable
        ));
    }

    #[test]
    fn owner_manifest_mismatch_enters_cleanup_without_publishing_ready() {
        let mut broker = broker();
        let ctx = context(SemanticSessionRolesV1::synchronize());
        let membership = joined(broker.command(ctx, join(88)));
        let epoch = id(|v| SemanticSynchronizationEpochV1::new(v).unwrap(), 89);
        let target = SemanticTargetV1 {
            document: SemanticDocumentRefV1::new([90; 16]),
            source_revision: SemanticSourceRevisionV1::new(1),
        };
        let snapshot = SemanticSessionSnapshotV1::try_new(
            target,
            "rust".into(),
            "declared-but-not-opened".into(),
            broker.limits(),
        )
        .unwrap();
        let manifest =
            SemanticSynchronizationManifestV1::try_from_snapshots(&[snapshot], broker.limits())
                .unwrap();
        broker.command(
            ctx,
            SemanticSessionCommandV1::BeginSynchronization {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 91),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 92),
                incarnation: broker.incarnation(),
                membership,
                epoch,
                manifest,
            },
        );
        let SemanticBrokerDecisionV1::Reply(reply) = broker.command(
            ctx,
            SemanticSessionCommandV1::CompleteSynchronization {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 93),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 94),
                incarnation: broker.incarnation(),
                membership,
                epoch,
                manifest,
            },
        ) else {
            panic!()
        };
        assert_eq!(
            reply.outcome(),
            &SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::ProtocolViolation)
        );
        assert!(matches!(
            broker.synchronization,
            SynchronizationState::Cleaning { owner } if owner == membership
        ));
    }

    #[test]
    fn ready_projection_supports_transactional_change_open_close_epochs_and_rejects_aba() {
        use vellum_host::semantic::SemanticQueryV1;
        let (mut broker, ctx, membership, _, _, first_target) =
            pending_request(SemanticQueryV1::Diagnostics, "abc");
        let second_epoch = id(|v| SemanticSynchronizationEpochV1::new(v).unwrap(), 130);
        let changed_target = SemanticTargetV1 {
            document: first_target.document,
            source_revision: SemanticSourceRevisionV1::new(2),
        };
        let changed = SemanticSessionSnapshotV1::try_new(
            changed_target,
            "rust".into(),
            "abcd".into(),
            broker.limits(),
        )
        .unwrap();
        let added_target = SemanticTargetV1 {
            document: SemanticDocumentRefV1::new([131; 16]),
            source_revision: SemanticSourceRevisionV1::new(1),
        };
        let added = SemanticSessionSnapshotV1::try_new(
            added_target,
            "rust".into(),
            "new".into(),
            broker.limits(),
        )
        .unwrap();
        let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
            &[changed.clone(), added.clone()],
            broker.limits(),
        )
        .unwrap();
        let begin = SemanticSessionCommandV1::BeginSynchronization {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 132),
            operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 133),
            incarnation: broker.incarnation(),
            membership,
            epoch: second_epoch,
            manifest,
        };
        let SemanticBrokerDecisionV1::Reply(started) = broker.command(ctx, begin.clone()) else {
            panic!()
        };
        assert_eq!(started.outcome(), &SemanticSessionOutcomeV1::Accepted);
        for (exchange, operation, command) in [
            (
                134,
                135,
                SemanticSessionCommandV1::SyncChange {
                    exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 134),
                    operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 135),
                    incarnation: broker.incarnation(),
                    membership,
                    epoch: second_epoch,
                    snapshot: changed,
                },
            ),
            (
                136,
                137,
                SemanticSessionCommandV1::SyncOpen {
                    exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 136),
                    operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 137),
                    incarnation: broker.incarnation(),
                    membership,
                    epoch: second_epoch,
                    snapshot: added.clone(),
                },
            ),
        ] {
            let _ = (exchange, operation);
            let SemanticBrokerDecisionV1::Effect(effect) = broker.command(ctx, command) else {
                panic!()
            };
            let continuation = match effect {
                SemanticBrokerEffectV1::Open { continuation, .. }
                | SemanticBrokerEffectV1::Change { continuation, .. } => continuation,
                _ => panic!(),
            };
            complete_admission(
                &mut broker,
                continuation,
                SemanticBrokerAdmissionOutcomeV1::Accepted,
            );
        }
        let SemanticBrokerDecisionV1::Reply(published) = broker.command(
            ctx,
            SemanticSessionCommandV1::CompleteSynchronization {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 138),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 139),
                incarnation: broker.incarnation(),
                membership,
                epoch: second_epoch,
                manifest,
            },
        ) else {
            panic!()
        };
        assert_eq!(published.outcome(), &SemanticSessionOutcomeV1::Accepted);
        assert_eq!(broker.seals.len(), 2);

        let third_epoch = id(|v| SemanticSynchronizationEpochV1::new(v).unwrap(), 140);
        let final_manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
            std::slice::from_ref(&added),
            broker.limits(),
        )
        .unwrap();
        broker.command(
            ctx,
            SemanticSessionCommandV1::BeginSynchronization {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 141),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 142),
                incarnation: broker.incarnation(),
                membership,
                epoch: third_epoch,
                manifest: final_manifest,
            },
        );
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Close {
            continuation, ..
        }) = broker.command(
            ctx,
            SemanticSessionCommandV1::SyncClose {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 143),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 144),
                incarnation: broker.incarnation(),
                membership,
                epoch: third_epoch,
                target: changed_target,
            },
        )
        else {
            panic!()
        };
        complete_admission(
            &mut broker,
            continuation,
            SemanticBrokerAdmissionOutcomeV1::Accepted,
        );
        broker.command(
            ctx,
            SemanticSessionCommandV1::CompleteSynchronization {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 145),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 146),
                incarnation: broker.incarnation(),
                membership,
                epoch: third_epoch,
                manifest: final_manifest,
            },
        );
        assert_eq!(broker.seals.len(), 1);
        let mut aba = begin;
        if let SemanticSessionCommandV1::BeginSynchronization {
            exchange,
            operation,
            ..
        } = &mut aba
        {
            *exchange = id(|v| SemanticExchangeRefV1::new(v).unwrap(), 147);
            *operation = id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 148);
        }
        let SemanticBrokerDecisionV1::Reply(reused) = broker.command(ctx, aba) else {
            panic!()
        };
        assert_eq!(
            reused.outcome(),
            &SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::ProtocolViolation)
        );
    }

    #[test]
    fn refused_next_epoch_effect_aborts_to_truthful_cleanup() {
        use vellum_host::semantic::SemanticQueryV1;
        let (mut broker, ctx, membership, _, _, target) =
            pending_request(SemanticQueryV1::Diagnostics, "abc");
        let epoch = id(|v| SemanticSynchronizationEpochV1::new(v).unwrap(), 149);
        let changed = SemanticSessionSnapshotV1::try_new(
            SemanticTargetV1 {
                document: target.document,
                source_revision: SemanticSourceRevisionV1::new(2),
            },
            "rust".into(),
            "abcd".into(),
            broker.limits(),
        )
        .unwrap();
        let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
            std::slice::from_ref(&changed),
            broker.limits(),
        )
        .unwrap();
        broker.command(
            ctx,
            SemanticSessionCommandV1::BeginSynchronization {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 150),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 151),
                incarnation: broker.incarnation(),
                membership,
                epoch,
                manifest,
            },
        );
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Change {
            continuation, ..
        }) = broker.command(
            ctx,
            SemanticSessionCommandV1::SyncChange {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 152),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 153),
                incarnation: broker.incarnation(),
                membership,
                epoch,
                snapshot: changed,
            },
        )
        else {
            panic!()
        };
        complete_admission(
            &mut broker,
            continuation,
            SemanticBrokerAdmissionOutcomeV1::Unavailable,
        );
        assert!(matches!(
            broker.synchronization,
            SynchronizationState::Cleaning { owner } if owner == membership
        ));
        assert_eq!(broker.seals[&target.document].target(), target);
    }

    #[test]
    fn synchronization_lane_rejects_concurrent_change_and_early_complete() {
        use vellum_host::semantic::SemanticQueryV1;
        let (mut broker, ctx, membership, _, _, target) =
            pending_request(SemanticQueryV1::Diagnostics, "abc");
        let epoch = id(|v| SemanticSynchronizationEpochV1::new(v).unwrap(), 154);
        let changed = |revision, source: &str| {
            SemanticSessionSnapshotV1::try_new(
                SemanticTargetV1 {
                    document: target.document,
                    source_revision: SemanticSourceRevisionV1::new(revision),
                },
                "rust".into(),
                source.into(),
                broker.limits(),
            )
            .unwrap()
        };
        let rev2 = changed(2, "abcd");
        let rev3 = changed(3, "abcde");
        let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
            std::slice::from_ref(&rev2),
            broker.limits(),
        )
        .unwrap();
        broker.command(
            ctx,
            SemanticSessionCommandV1::BeginSynchronization {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 155),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 156),
                incarnation: broker.incarnation(),
                membership,
                epoch,
                manifest,
            },
        );
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Change {
            continuation, ..
        }) = broker.command(
            ctx,
            SemanticSessionCommandV1::SyncChange {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 157),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 158),
                incarnation: broker.incarnation(),
                membership,
                epoch,
                snapshot: rev2,
            },
        )
        else {
            panic!()
        };
        for command in [
            SemanticSessionCommandV1::SyncChange {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 159),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 160),
                incarnation: broker.incarnation(),
                membership,
                epoch,
                snapshot: rev3,
            },
            SemanticSessionCommandV1::CompleteSynchronization {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 161),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 162),
                incarnation: broker.incarnation(),
                membership,
                epoch,
                manifest,
            },
        ] {
            let SemanticBrokerDecisionV1::Reply(reply) = broker.command(ctx, command) else {
                panic!()
            };
            assert_eq!(
                reply.outcome(),
                &SemanticSessionOutcomeV1::Refused(
                    SemanticSessionRefusalV1::SynchronizationConflict
                )
            );
        }
        complete_admission(
            &mut broker,
            continuation,
            SemanticBrokerAdmissionOutcomeV1::Accepted,
        );
        assert_eq!(
            broker.seals[&target.document]
                .target()
                .source_revision
                .get(),
            2
        );
    }

    #[test]
    fn staged_open_refuses_count_and_aggregate_overflow_before_effect() {
        for (max_documents, max_total_snapshot_bytes) in [(1, 8), (2, 4)] {
            let mut limits = SemanticSessionLimitsV1::reference();
            limits.max_snapshot_bytes = 4;
            limits.max_total_snapshot_bytes = max_total_snapshot_bytes;
            limits.max_documents = max_documents;
            let mut broker = SemanticSessionBrokerV1::try_new(
                id(|v| SemanticSessionIncarnationV1::new(v).unwrap(), 3),
                limits,
            )
            .unwrap();
            let ctx = context(SemanticSessionRolesV1::synchronize());
            let mut command = join(164);
            if let SemanticSessionCommandV1::JoinSession {
                requested_roles, ..
            } = &mut command
            {
                *requested_roles = SemanticSessionRolesV1::synchronize();
            }
            let membership = joined(broker.command(ctx, command));
            let old = SemanticSessionSnapshotV1::try_new(
                SemanticTargetV1 {
                    document: SemanticDocumentRefV1::new([165; 16]),
                    source_revision: SemanticSourceRevisionV1::new(1),
                },
                "rust".into(),
                "abc".into(),
                limits,
            )
            .unwrap();
            let old_witness = DocumentWitness::from_snapshot(&old, limits).unwrap();
            broker.seals.insert(old.target().document, old_witness.seal);
            broker.witnesses.insert(old.target().document, old_witness);
            broker.synchronization = SynchronizationState::Ready {
                owner: membership,
                epoch: id(|v| SemanticSynchronizationEpochV1::new(v).unwrap(), 166),
            };
            let next = SemanticSessionSnapshotV1::try_new(
                SemanticTargetV1 {
                    document: SemanticDocumentRefV1::new([167; 16]),
                    source_revision: SemanticSourceRevisionV1::new(1),
                },
                "rust".into(),
                "xy".into(),
                limits,
            )
            .unwrap();
            let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
                std::slice::from_ref(&next),
                limits,
            )
            .unwrap();
            let epoch = id(|v| SemanticSynchronizationEpochV1::new(v).unwrap(), 168);
            broker.command(
                ctx,
                SemanticSessionCommandV1::BeginSynchronization {
                    exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 169),
                    operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 170),
                    incarnation: broker.incarnation(),
                    membership,
                    epoch,
                    manifest,
                },
            );
            let effects = broker.usage().effects;
            let SemanticBrokerDecisionV1::Reply(refused) = broker.command(
                ctx,
                SemanticSessionCommandV1::SyncOpen {
                    exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 171),
                    operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 172),
                    incarnation: broker.incarnation(),
                    membership,
                    epoch,
                    snapshot: next,
                },
            ) else {
                panic!()
            };
            assert_eq!(
                refused.outcome(),
                &SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::InputLimit)
            );
            assert_eq!(broker.usage().effects, effects);
        }
    }

    #[test]
    fn unsupported_delivery_must_match_retained_query_kind() {
        use vellum_host::semantic::{SemanticQueryKindV1, SemanticQueryV1};
        let (mut broker, ctx, membership, external, internal, _) =
            pending_request(SemanticQueryV1::Diagnostics, "abc");
        let incarnation = broker.incarnation();
        assert_eq!(
            complete_delivery(
                &mut broker,
                internal,
                SemanticBrokerDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Unsupported(
                    SemanticQueryKindV1::Symbols
                ),),
            ),
            SemanticBrokerDecisionV1::EventAccepted
        );
        let collect = |exchange| SemanticSessionCommandV1::Collect {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), exchange),
            incarnation,
            membership,
            request: external,
        };
        let SemanticBrokerDecisionV1::Reply(pending) = broker.command(ctx, collect(112)) else {
            panic!()
        };
        assert_eq!(
            pending.outcome(),
            &SemanticSessionOutcomeV1::PeerRefused(
                vellum_host::semantic::SemanticRefusalV1::MalformedProviderReply
            )
        );
        let (mut broker, ctx, membership, external, internal, _) =
            pending_request(SemanticQueryV1::Diagnostics, "abc");
        let incarnation = broker.incarnation();
        let collect = |exchange| SemanticSessionCommandV1::Collect {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), exchange),
            incarnation,
            membership,
            request: external,
        };
        assert_eq!(
            complete_delivery(
                &mut broker,
                internal,
                SemanticBrokerDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Unsupported(
                    SemanticQueryKindV1::Diagnostics
                ),),
            ),
            SemanticBrokerDecisionV1::EventAccepted
        );
        let SemanticBrokerDecisionV1::Reply(terminal) = broker.command(ctx, collect(113)) else {
            panic!()
        };
        assert_eq!(
            terminal.outcome(),
            &SemanticSessionOutcomeV1::Unsupported(SemanticQueryKindV1::Diagnostics)
        );
    }

    #[test]
    fn ordered_expiry_is_stable_until_explicit_release() {
        use vellum_host::semantic::SemanticQueryV1;
        let (mut broker, ctx, membership, external, internal, _) =
            pending_request(SemanticQueryV1::Diagnostics, "abc");
        assert_eq!(
            complete_expiry(
                &mut broker,
                internal,
                SemanticBrokerExpiryOutcomeV1::Expired,
            ),
            SemanticBrokerDecisionV1::EventAccepted
        );
        for exchange in [114, 115] {
            let SemanticBrokerDecisionV1::Reply(reply) = broker.command(
                ctx,
                SemanticSessionCommandV1::Collect {
                    exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), exchange),
                    incarnation: broker.incarnation(),
                    membership,
                    request: external,
                },
            ) else {
                panic!()
            };
            assert_eq!(reply.outcome(), &SemanticSessionOutcomeV1::TimedOut);
        }
        let SemanticBrokerDecisionV1::Reply(released) = broker.command(
            ctx,
            SemanticSessionCommandV1::ReleaseRequest {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 116),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 117),
                incarnation: broker.incarnation(),
                membership,
                request: external,
            },
        ) else {
            panic!()
        };
        assert_eq!(released.outcome(), &SemanticSessionOutcomeV1::Accepted);
    }

    #[test]
    fn interleaved_delivery_during_leave_cancel_poison_fences_later_commands() {
        use vellum_host::semantic::SemanticQueryV1;
        let (mut broker, ctx, membership, _, internal, _) =
            pending_request(SemanticQueryV1::Diagnostics, "abc");
        let incarnation = broker.incarnation();
        let leave = |exchange, operation| SemanticSessionCommandV1::LeaveSession {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), exchange),
            operation: id(
                |v| SemanticSessionOperationRefV1::new(v).unwrap(),
                operation,
            ),
            incarnation,
            membership,
        };
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Cancel {
            continuation, ..
        }) = broker.command(ctx, leave(124, 125))
        else {
            panic!()
        };
        assert_eq!(
            broker.event(SemanticBrokerEventV1::ProviderDriveCompleted {
                report: Box::new(SemanticBrokerProviderDriveReportV1 {
                    provider_before: provider_binding(&broker),
                    pre_provider: None,
                    disposition: SemanticBrokerProviderDriveDispositionV1::Delivery {
                        internal_request: internal,
                        outcome: SemanticBrokerDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Stale),
                    },
                    post_provider: None,
                }),
            }),
            SemanticBrokerDecisionV1::Diverged(SemanticBrokerDivergenceV1::TransitionOrdering)
        );
        assert_eq!(broker.usage().pending_effects, 1);
        assert_eq!(
            broker.command(ctx, leave(126, 127)),
            SemanticBrokerDecisionV1::Diverged(SemanticBrokerDivergenceV1::TransitionOrdering)
        );
        let _ = continuation;
    }

    #[test]
    fn policy_loss_system_drive_cancels_then_closes_without_protocol_identity() {
        use vellum_host::semantic::SemanticQueryV1;
        let (mut broker, ctx, membership, _, _, target) =
            pending_request(SemanticQueryV1::Diagnostics, "abc");
        let operations_before = broker.memberships[&membership].operations.len();
        broker.event(SemanticBrokerEventV1::PolicyLost {
            subject: ctx.subject,
            session: ctx.session,
            policy_epoch: ctx.policy_epoch,
        });
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Cancel {
            continuation, ..
        }) = broker.event(SemanticBrokerEventV1::DriveRetirement)
        else {
            panic!()
        };
        assert_eq!(
            complete_cancellation(
                &mut broker,
                continuation,
                SemanticBrokerCancellationOutcomeV1::Accepted
            ),
            SemanticBrokerDecisionV1::EventAccepted
        );
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Close {
            continuation,
            target: closing,
        }) = broker.event(SemanticBrokerEventV1::DriveRetirement)
        else {
            panic!()
        };
        assert_eq!(closing, target);
        assert_eq!(
            complete_admission(
                &mut broker,
                continuation,
                SemanticBrokerAdmissionOutcomeV1::Accepted
            ),
            SemanticBrokerDecisionV1::EventAccepted
        );
        assert_eq!(broker.usage().current_memberships, 0);
        assert_eq!(
            broker.memberships[&membership].operations.len(),
            operations_before
        );
        assert!(matches!(
            broker.synchronization,
            SynchronizationState::Unavailable
        ));
    }

    #[test]
    fn complete_from_replaced_provider_epoch_is_quarantined() {
        use vellum_host::semantic::SemanticQueryV1;
        let (mut broker, _, _, _, internal, _) =
            pending_request(SemanticQueryV1::Diagnostics, "abc");
        install_ready_provider(&mut broker, 2);
        let (membership, external) = broker.reverse_requests[&internal];
        assert!(matches!(
            broker.memberships[&membership].requests[&external].state,
            RequestState::Terminal(SemanticSessionOutcomeV1::Stale)
        ));
        assert_eq!(
            complete_late(&mut broker, internal),
            SemanticBrokerDecisionV1::EventAccepted
        );
        assert_eq!(broker.usage().quarantined, 1);
    }

    #[test]
    fn delivery_rejects_foreign_out_of_bounds_and_split_utf8_locations() {
        use vellum_host::semantic::{
            SemanticLocationV1, SemanticQueryV1, SemanticReplyV1, SemanticResultV1, SemanticSpanV1,
        };
        for (target_mutation, start, end) in [(false, 1, 1), (false, 0, 4), (true, 0, 2)] {
            let (mut broker, _, _, _, internal, target) =
                pending_request(SemanticQueryV1::References { offset: 0 }, "éx");
            let location_target = if target_mutation {
                SemanticTargetV1 {
                    document: SemanticDocumentRefV1::new([123; 16]),
                    source_revision: target.source_revision,
                }
            } else {
                target
            };
            let result = SemanticResultV1::try_locations(
                vec![SemanticLocationV1 {
                    target: location_target,
                    span: SemanticSpanV1::try_new(start, end).unwrap(),
                }],
                broker.limits.semantic,
            )
            .unwrap();
            assert_eq!(
                complete_delivery(
                    &mut broker,
                    internal,
                    SemanticBrokerDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Complete(
                        SemanticReplyV1 {
                            request: internal,
                            target,
                            index_generation: SemanticIndexGenerationV1::new(1).unwrap(),
                            provider: provider(),
                            result,
                        }
                    ),),
                ),
                SemanticBrokerDecisionV1::EventAccepted
            );
        }
    }

    #[test]
    fn delivery_rejects_overlapping_rename_edits() {
        use vellum_host::semantic::{
            SemanticLocationV1, SemanticQueryV1, SemanticReplyV1, SemanticResultV1, SemanticSpanV1,
            SemanticTextEditV1, SemanticTextV1,
        };
        let text = || SemanticTextV1::try_new("z".into(), 8).unwrap();
        let (mut broker, _, _, _, internal, target) = pending_request(
            SemanticQueryV1::PlanRename {
                offset: 0,
                new_name: text(),
            },
            "abc",
        );
        let result = SemanticResultV1::try_rename_plan(
            vec![
                SemanticTextEditV1 {
                    location: SemanticLocationV1 {
                        target,
                        span: SemanticSpanV1::try_new(0, 2).unwrap(),
                    },
                    replacement: text(),
                },
                SemanticTextEditV1 {
                    location: SemanticLocationV1 {
                        target,
                        span: SemanticSpanV1::try_new(1, 3).unwrap(),
                    },
                    replacement: text(),
                },
            ],
            broker.limits.semantic,
        )
        .unwrap();
        assert_eq!(
            complete_delivery(
                &mut broker,
                internal,
                SemanticBrokerDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Complete(
                    SemanticReplyV1 {
                        request: internal,
                        target,
                        index_generation: SemanticIndexGenerationV1::new(1).unwrap(),
                        provider: provider(),
                        result,
                    },
                )),
            ),
            SemanticBrokerDecisionV1::EventAccepted
        );
    }

    #[test]
    fn accepted_change_terminalizes_old_type_request_before_late_delivery() {
        use vellum_host::semantic::SemanticQueryV1;
        let (mut broker, ctx, membership, external, internal, old_target) =
            pending_request(SemanticQueryV1::TypeAtPoint { offset: 0 }, "abc");
        let reserved = broker.usage().retained_result_bytes;
        let epoch = id(|v| SemanticSynchronizationEpochV1::new(v).unwrap(), 180);
        let changed = SemanticSessionSnapshotV1::try_new(
            SemanticTargetV1 {
                document: old_target.document,
                source_revision: SemanticSourceRevisionV1::new(2),
            },
            "rust".into(),
            "abcd".into(),
            broker.limits(),
        )
        .unwrap();
        let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
            std::slice::from_ref(&changed),
            broker.limits(),
        )
        .unwrap();
        broker.command(
            ctx,
            SemanticSessionCommandV1::BeginSynchronization {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 181),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 182),
                incarnation: broker.incarnation(),
                membership,
                epoch,
                manifest,
            },
        );
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Change {
            continuation, ..
        }) = broker.command(
            ctx,
            SemanticSessionCommandV1::SyncChange {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 183),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 184),
                incarnation: broker.incarnation(),
                membership,
                epoch,
                snapshot: changed,
            },
        )
        else {
            panic!()
        };
        complete_admission(
            &mut broker,
            continuation,
            SemanticBrokerAdmissionOutcomeV1::Accepted,
        );
        assert_eq!(broker.usage().retained_result_bytes, reserved);
        let SemanticBrokerDecisionV1::Reply(collected) = broker.command(
            ctx,
            SemanticSessionCommandV1::Collect {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 185),
                incarnation: broker.incarnation(),
                membership,
                request: external,
            },
        ) else {
            panic!()
        };
        assert_eq!(collected.outcome(), &SemanticSessionOutcomeV1::Stale);
        assert_eq!(
            complete_late(&mut broker, internal),
            SemanticBrokerDecisionV1::EventAccepted
        );
        let SemanticBrokerDecisionV1::Reply(released) = broker.command(
            ctx,
            SemanticSessionCommandV1::ReleaseRequest {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 194),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 195),
                incarnation: broker.incarnation(),
                membership,
                request: external,
            },
        ) else {
            panic!()
        };
        assert_eq!(released.outcome(), &SemanticSessionOutcomeV1::Accepted);
        assert_eq!(broker.usage().retained_result_bytes, 0);
    }

    #[test]
    fn accepted_close_terminalizes_old_location_request_before_empty_late_delivery() {
        use vellum_host::semantic::SemanticQueryV1;
        let (mut broker, ctx, membership, external, internal, old_target) =
            pending_request(SemanticQueryV1::References { offset: 0 }, "abc");
        let reserved = broker.usage().retained_result_bytes;
        let epoch = id(|v| SemanticSynchronizationEpochV1::new(v).unwrap(), 186);
        let manifest =
            SemanticSynchronizationManifestV1::try_from_seals(&[], broker.limits()).unwrap();
        broker.command(
            ctx,
            SemanticSessionCommandV1::BeginSynchronization {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 187),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 188),
                incarnation: broker.incarnation(),
                membership,
                epoch,
                manifest,
            },
        );
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Close {
            continuation, ..
        }) = broker.command(
            ctx,
            SemanticSessionCommandV1::SyncClose {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 189),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 190),
                incarnation: broker.incarnation(),
                membership,
                epoch,
                target: old_target,
            },
        )
        else {
            panic!()
        };
        complete_admission(
            &mut broker,
            continuation,
            SemanticBrokerAdmissionOutcomeV1::Accepted,
        );
        assert_eq!(broker.usage().retained_result_bytes, reserved);
        let SemanticBrokerDecisionV1::Reply(collected) = broker.command(
            ctx,
            SemanticSessionCommandV1::Collect {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 191),
                incarnation: broker.incarnation(),
                membership,
                request: external,
            },
        ) else {
            panic!()
        };
        assert_eq!(collected.outcome(), &SemanticSessionOutcomeV1::Stale);
        assert_eq!(
            complete_late(&mut broker, internal),
            SemanticBrokerDecisionV1::EventAccepted
        );
        let SemanticBrokerDecisionV1::Reply(released) = broker.command(
            ctx,
            SemanticSessionCommandV1::ReleaseRequest {
                exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 196),
                operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 197),
                incarnation: broker.incarnation(),
                membership,
                request: external,
            },
        ) else {
            panic!()
        };
        assert_eq!(released.outcome(), &SemanticSessionOutcomeV1::Accepted);
        assert_eq!(broker.usage().retained_result_bytes, 0);
    }

    #[test]
    fn submit_reserves_and_namespaces_before_effect_then_collects_pending() {
        use vellum_host::semantic::SemanticTextV1;
        let mut broker = broker();
        install_ready_provider(&mut broker, 1);
        let ctx = context(SemanticSessionRolesV1::top());
        let membership = joined(broker.command(ctx, join(6)));
        let epoch = id(|v| SemanticSynchronizationEpochV1::new(v).unwrap(), 20);
        let synchronized_target = SemanticTargetV1 {
            document: SemanticDocumentRefV1::new([26; 16]),
            source_revision: SemanticSourceRevisionV1::new(1),
        };
        let synchronized_snapshot = SemanticSessionSnapshotV1::try_new(
            synchronized_target,
            "rust".into(),
            "abc".into(),
            broker.limits(),
        )
        .unwrap();
        let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
            std::slice::from_ref(&synchronized_snapshot),
            broker.limits(),
        )
        .unwrap();
        let begin = SemanticSessionCommandV1::BeginSynchronization {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 21),
            operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 22),
            incarnation: broker.incarnation(),
            membership,
            epoch,
            manifest,
        };
        broker.command(ctx, begin);
        let open = SemanticSessionCommandV1::SyncOpen {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 38),
            operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 39),
            incarnation: broker.incarnation(),
            membership,
            epoch,
            snapshot: synchronized_snapshot,
        };
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Open {
            continuation: open_continuation,
            ..
        }) = broker.command(ctx, open)
        else {
            panic!()
        };
        complete_admission(
            &mut broker,
            open_continuation,
            SemanticBrokerAdmissionOutcomeV1::Accepted,
        );
        let complete = SemanticSessionCommandV1::CompleteSynchronization {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 23),
            operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 24),
            incarnation: broker.incarnation(),
            membership,
            epoch,
            manifest,
        };
        broker.command(ctx, complete);
        let external = SemanticRequestRefV1::new([25; 16]);
        let request = SemanticRequestV1 {
            request: external,
            target: synchronized_target,
            expected_index: None,
            language: SemanticTextV1::try_new("rust".into(), 32).unwrap(),
            query: vellum_host::semantic::SemanticQueryV1::Diagnostics,
        };
        let submit = SemanticSessionCommandV1::Submit {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 27),
            operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 28),
            incarnation: broker.incarnation(),
            membership,
            request,
        };
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Submit {
            continuation,
            request,
        }) = broker.command(ctx, submit)
        else {
            panic!()
        };
        assert_ne!(request.request, external);
        assert!(broker.usage().retained_result_bytes > 0);
        complete_submission(
            &mut broker,
            continuation,
            SemanticBrokerSubmissionOutcomeV1::Accepted,
        );
        let external_two = SemanticRequestRefV1::new([32; 16]);
        let request_two = SemanticRequestV1 {
            request: external_two,
            target: synchronized_target,
            expected_index: None,
            language: SemanticTextV1::try_new("rust".into(), 32).unwrap(),
            query: vellum_host::semantic::SemanticQueryV1::Diagnostics,
        };
        let submit_two = SemanticSessionCommandV1::Submit {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 33),
            operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 34),
            incarnation: broker.incarnation(),
            membership,
            request: request_two,
        };
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Submit {
            continuation: continuation_two,
            ..
        }) = broker.command(ctx, submit_two)
        else {
            panic!()
        };
        complete_submission(
            &mut broker,
            continuation_two,
            SemanticBrokerSubmissionOutcomeV1::Accepted,
        );
        let collect = SemanticSessionCommandV1::Collect {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 29),
            incarnation: broker.incarnation(),
            membership,
            request: external,
        };
        let SemanticBrokerDecisionV1::Reply(reply) = broker.command(ctx, collect) else {
            panic!()
        };
        assert_eq!(
            reply.outcome(),
            &SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Request(external))
        );
        let leave = SemanticSessionCommandV1::LeaveSession {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 30),
            operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 31),
            incarnation: broker.incarnation(),
            membership,
        };
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Cancel {
            continuation,
            request,
        }) = broker.command(ctx, leave.clone())
        else {
            panic!()
        };
        assert_ne!(request, external);
        let SemanticBrokerDecisionV1::Reply(retired) = complete_cancellation(
            &mut broker,
            continuation,
            SemanticBrokerCancellationOutcomeV1::Accepted,
        ) else {
            panic!()
        };
        assert_eq!(
            retired.outcome(),
            &SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Retirement {
                remaining_documents: 1,
                quarantined_requests: 1
            })
        );
        let mut replay_leave = leave.clone();
        if let SemanticSessionCommandV1::LeaveSession { exchange, .. } = &mut replay_leave {
            *exchange = id(|v| SemanticExchangeRefV1::new(v).unwrap(), 35);
        }
        let SemanticBrokerDecisionV1::Reply(old_replay) = broker.command(ctx, replay_leave) else {
            panic!()
        };
        assert_eq!(old_replay.outcome(), retired.outcome());
        let fresh_leave = SemanticSessionCommandV1::LeaveSession {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 36),
            operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 37),
            incarnation: broker.incarnation(),
            membership,
        };
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Cancel {
            continuation: final_cancel,
            ..
        }) = broker.command(ctx, fresh_leave)
        else {
            panic!()
        };
        let SemanticBrokerDecisionV1::Reply(still_retiring) = complete_cancellation(
            &mut broker,
            final_cancel,
            SemanticBrokerCancellationOutcomeV1::Accepted,
        ) else {
            panic!()
        };
        assert_eq!(
            still_retiring.outcome(),
            &SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Retirement {
                remaining_documents: 1,
                quarantined_requests: 0
            })
        );
        let close_leave = SemanticSessionCommandV1::LeaveSession {
            exchange: id(|v| SemanticExchangeRefV1::new(v).unwrap(), 40),
            operation: id(|v| SemanticSessionOperationRefV1::new(v).unwrap(), 41),
            incarnation: broker.incarnation(),
            membership,
        };
        let SemanticBrokerDecisionV1::Effect(SemanticBrokerEffectV1::Close {
            continuation: close,
            target,
        }) = broker.command(ctx, close_leave)
        else {
            panic!()
        };
        assert_eq!(target, synchronized_target);
        let SemanticBrokerDecisionV1::Reply(retired) = complete_admission(
            &mut broker,
            close,
            SemanticBrokerAdmissionOutcomeV1::Accepted,
        ) else {
            panic!()
        };
        assert_eq!(retired.outcome(), &SemanticSessionOutcomeV1::Accepted);
        assert_eq!(broker.usage().current_memberships, 0);
    }
}

#[derive(Clone)]
enum ReplayResult {
    Pending(u64),
    Complete(SemanticSessionOutcomeV1),
}
#[derive(Clone)]
struct ReplayRecord {
    intent: SemanticSessionCommandV1,
    bytes: usize,
    result: ReplayResult,
}
struct Membership {
    context: SemanticBrokerContextV1,
    roles: SemanticSessionRolesV1,
    operations: BTreeMap<SemanticSessionOperationRefV1, ReplayRecord>,
    requests: BTreeMap<SemanticRequestRefV1, RequestRecord>,
    retiring: bool,
    retired: bool,
}
#[derive(Clone)]
enum RequestState {
    Submitting,
    Pending,
    Terminal(SemanticSessionOutcomeV1),
    Released,
}
#[derive(Clone)]
struct RequestRecord {
    internal: SemanticRequestRefV1,
    external: SemanticRequestV1,
    reservation: usize,
    provider_epoch: u64,
    expected_provider: SemanticProviderProvenanceV1,
    expected_generation: SemanticIndexGenerationV1,
    state: RequestState,
}
#[derive(Clone, Eq, PartialEq)]
struct ProviderState {
    epoch: u64,
    observation: SemanticProviderObservationV1,
    provenance: Option<SemanticProviderProvenanceV1>,
}
#[derive(Clone, Copy)]
enum SynchronizationState {
    Unavailable,
    Active {
        owner: SemanticMembershipRefV1,
        epoch: SemanticSynchronizationEpochV1,
        manifest: SemanticSynchronizationManifestV1,
    },
    Ready {
        owner: SemanticMembershipRefV1,
        epoch: SemanticSynchronizationEpochV1,
    },
    Cleaning {
        owner: SemanticMembershipRefV1,
    },
}
struct PendingEffect {
    command: Option<SemanticSessionCommandV1>,
    membership: SemanticMembershipRefV1,
    seal: Option<SemanticSynchronizationDocumentSealV1>,
    cleanup_internal: Option<SemanticRequestRefV1>,
    transition: Option<TransitionExpectation>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExpectedSourceKind {
    None,
    Open,
    Change,
    Close,
}

#[derive(Clone, Copy)]
enum PendingCompletionKind {
    Open,
    Change,
    Close,
    Submit,
    Cancel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExpectedSourceEffect {
    None,
    Open,
    Change {
        document: vellum_host::semantic::SemanticDocumentRefV1,
        target: SemanticTargetV1,
    },
    Close {
        document: vellum_host::semantic::SemanticDocumentRefV1,
    },
}

impl ExpectedSourceEffect {
    const fn kind(self) -> ExpectedSourceKind {
        match self {
            Self::None => ExpectedSourceKind::None,
            Self::Open => ExpectedSourceKind::Open,
            Self::Change { .. } => ExpectedSourceKind::Change,
            Self::Close { .. } => ExpectedSourceKind::Close,
        }
    }

    fn affects(self, record: &RequestRecord) -> bool {
        match self {
            Self::None | Self::Open => false,
            Self::Change { document, target } => {
                record.external.target.document == document && record.external.target != target
            }
            Self::Close { document } => record.external.target.document == document,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExpectedRequest {
    internal: SemanticRequestRefV1,
    provider_epoch: u64,
    source_affected: bool,
}

#[derive(Clone, Eq, PartialEq)]
struct TransitionExpectation {
    provider: ProviderState,
    requests: Vec<ExpectedRequest>,
    source: ExpectedSourceEffect,
}

impl TransitionExpectation {
    fn live_ids(&self) -> BTreeSet<SemanticRequestRefV1> {
        self.requests
            .iter()
            .map(|request| request.internal)
            .collect()
    }
}

struct TransitionPlan {
    provider: ProviderState,
    terminal: BTreeSet<SemanticRequestRefV1>,
    remaining: BTreeSet<SemanticRequestRefV1>,
}
#[derive(Clone)]
struct DocumentWitness {
    seal: SemanticSynchronizationDocumentSealV1,
    language: String,
    boundaries: Vec<u8>,
}
impl DocumentWitness {
    fn from_snapshot(
        snapshot: &SemanticSessionSnapshotV1,
        limits: SemanticSessionLimitsV1,
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
            boundaries,
        })
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
            && bitmap_boundary(&self.boundaries, start)
            && bitmap_boundary(&self.boundaries, end)
    }
}
enum Completion {
    Accepted,
    Unsupported(vellum_host::semantic::SemanticQueryKindV1),
    Stale,
    Unavailable,
    Refused(SemanticSessionRefusalV1),
}

fn provider_state_is_valid(
    observation: SemanticProviderObservationV1,
    provenance: Option<&SemanticProviderProvenanceV1>,
) -> bool {
    match (observation.readiness(), provenance) {
        (SemanticProviderReadinessProjectionV1::Ready { .. }, Some(_)) => true,
        (
            SemanticProviderReadinessProjectionV1::Unavailable
            | SemanticProviderReadinessProjectionV1::Indexing,
            None,
        ) => observation.capabilities() == SemanticCapabilitySetV1::bottom(),
        _ => false,
    }
}

fn validate_receipt(
    receipt: &SemanticInvalidationReceiptV1,
    cause: SemanticInvalidationCauseV1,
    expected: &BTreeSet<SemanticRequestRefV1>,
) -> Result<(), ()> {
    if receipt.cause() != cause || !receipt.iter().copied().eq(expected.iter().copied()) {
        return Err(());
    }
    Ok(())
}

fn provider_binding_matches(
    binding: &SemanticBrokerProviderBindingV1,
    provider: &ProviderState,
) -> bool {
    binding.epoch == provider.epoch
        && binding.observation == provider.observation
        && binding.provenance == provider.provenance
}

fn validate_provider_transition(
    transition: &SemanticBrokerProviderTransitionV1,
    expectation: &TransitionExpectation,
    shadow: &mut BTreeSet<SemanticRequestRefV1>,
    terminal: &mut BTreeSet<SemanticRequestRefV1>,
    provider: &mut ProviderState,
) -> Result<(), ()> {
    if transition.epoch == 0
        || provider.epoch.checked_add(1) != Some(transition.epoch)
        || (transition.observed_observation == provider.observation
            && transition.observed_provenance == provider.provenance)
        || !provider_state_is_valid(
            transition.observed_observation,
            transition.observed_provenance.as_ref(),
        )
        || !provider_state_is_valid(
            transition.settled_observation,
            transition.settled_provenance.as_ref(),
        )
    {
        return Err(());
    }
    let expected: BTreeSet<_> = expectation
        .requests
        .iter()
        .filter(|request| {
            shadow.contains(&request.internal) && request.provider_epoch < transition.epoch
        })
        .map(|request| request.internal)
        .collect();
    validate_receipt(
        &transition.invalidations,
        SemanticInvalidationCauseV1::ProviderStateChanged,
        &expected,
    )?;
    for request in expected {
        shadow.remove(&request);
        terminal.insert(request);
    }
    *provider = ProviderState {
        epoch: transition.epoch,
        observation: transition.settled_observation,
        provenance: transition.settled_provenance.clone(),
    };
    Ok(())
}

fn sync_effect_is_current(
    command: &SemanticSessionCommandV1,
    membership: SemanticMembershipRefV1,
    state: SynchronizationState,
) -> bool {
    let epoch = match command {
        SemanticSessionCommandV1::SyncOpen { epoch, .. }
        | SemanticSessionCommandV1::SyncChange { epoch, .. }
        | SemanticSessionCommandV1::SyncClose { epoch, .. } => *epoch,
        _ => return false,
    };
    matches!(state, SynchronizationState::Active { owner, epoch: active, .. } if owner == membership && active == epoch)
}
