//! Bounded, provider-neutral semantic snapshot and query service.
//!
//! This crate synchronizes exact immutable source snapshots and routes bounded
//! semantic requests. It does not launch or select a provider, implement LSP,
//! authenticate a caller, authorize workspace disclosure, or apply edits.
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};

use vellum_host::semantic::{
    MAX_SEMANTIC_REQUESTS_V1, SemanticCapabilitySetV1, SemanticDocumentRefV1,
    SemanticIndexGenerationV1, SemanticInvalidationCauseV1, SemanticInvalidationReceiptV1,
    SemanticLimitsV1, SemanticOutcomeV1, SemanticProviderProvenanceV1, SemanticQueryKindV1,
    SemanticRefusalV1, SemanticRequestRefV1, SemanticRequestV1, SemanticTargetV1,
};

pub const MAX_SEMANTIC_SNAPSHOT_BYTES_V1: usize = 4 * 1024 * 1024;
pub const MAX_SEMANTIC_TOTAL_SNAPSHOT_BYTES_V1: usize = 64 * 1024 * 1024;
pub const MAX_SEMANTIC_CLIENTS_V1: usize = 64;
pub const MAX_SEMANTIC_DOCUMENTS_V1: usize = 4_096;
pub const MAX_SEMANTIC_PENDING_V1: usize = 1_024;
pub const MAX_SEMANTIC_DELIVERIES_V1: usize = 1_024;
pub const MAX_PROVIDER_POLLS_PER_PUMP_V1: usize = 256;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticClientRefV1([u8; 16]);

impl SemanticClientRefV1 {
    pub const fn new(value: [u8; 16]) -> Self {
        Self(value)
    }

    /// Canonical correlation bytes for trusted persistence adapters. These
    /// bytes are neither authentication nor authority by possession.
    pub const fn as_bytes(self) -> [u8; 16] {
        self.0
    }
}

impl fmt::Debug for SemanticClientRefV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SemanticClientRefV1(<opaque>)")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticServiceLimitsV1 {
    pub max_snapshot_bytes: usize,
    pub max_total_snapshot_bytes: usize,
    pub max_clients: usize,
    pub max_documents: usize,
    pub max_pending: usize,
    pub max_deliveries: usize,
    pub max_provider_polls_per_pump: usize,
    pub peer: SemanticLimitsV1,
}

impl SemanticServiceLimitsV1 {
    pub const fn reference() -> Self {
        Self {
            max_snapshot_bytes: MAX_SEMANTIC_SNAPSHOT_BYTES_V1,
            max_total_snapshot_bytes: MAX_SEMANTIC_TOTAL_SNAPSHOT_BYTES_V1,
            max_clients: MAX_SEMANTIC_CLIENTS_V1,
            max_documents: MAX_SEMANTIC_DOCUMENTS_V1,
            max_pending: MAX_SEMANTIC_PENDING_V1,
            max_deliveries: MAX_SEMANTIC_DELIVERIES_V1,
            max_provider_polls_per_pump: MAX_PROVIDER_POLLS_PER_PUMP_V1,
            peer: SemanticLimitsV1::reference(),
        }
    }

    pub const fn is_valid(self) -> bool {
        self.max_snapshot_bytes > 0
            && self.max_snapshot_bytes <= MAX_SEMANTIC_SNAPSHOT_BYTES_V1
            && self.max_total_snapshot_bytes >= self.max_snapshot_bytes
            && self.max_total_snapshot_bytes <= MAX_SEMANTIC_TOTAL_SNAPSHOT_BYTES_V1
            && self.max_clients > 0
            && self.max_clients <= MAX_SEMANTIC_CLIENTS_V1
            && self.max_documents > 0
            && self.max_documents <= MAX_SEMANTIC_DOCUMENTS_V1
            && self.max_pending > 0
            && self.max_pending <= MAX_SEMANTIC_PENDING_V1
            && self.max_deliveries > 0
            && self.max_deliveries <= MAX_SEMANTIC_DELIVERIES_V1
            && self.max_provider_polls_per_pump > 0
            && self.max_provider_polls_per_pump <= MAX_PROVIDER_POLLS_PER_PUMP_V1
            && self.peer.is_valid()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct SemanticSnapshotV1 {
    target: SemanticTargetV1,
    language: String,
    source: String,
}

impl SemanticSnapshotV1 {
    pub fn try_new(
        target: SemanticTargetV1,
        language: String,
        source: String,
        limits: SemanticServiceLimitsV1,
    ) -> Result<Self, SemanticServiceRefusalV1> {
        if !limits.is_valid()
            || language.is_empty()
            || language.len() > limits.peer.max_label_bytes
            || source.len() > limits.max_snapshot_bytes
        {
            return Err(SemanticServiceRefusalV1::InputLimit);
        }
        Ok(Self {
            target,
            language,
            source,
        })
    }

    pub const fn target(&self) -> SemanticTargetV1 {
        self.target
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn is_within_limits(&self, limits: SemanticServiceLimitsV1) -> bool {
        limits.is_valid()
            && !self.language.is_empty()
            && self.language.len() <= limits.peer.max_label_bytes
            && self.source.len() <= limits.max_snapshot_bytes
    }
}

impl fmt::Debug for SemanticSnapshotV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticSnapshotV1")
            .field("target", &self.target)
            .field("language_bytes", &self.language.len())
            .field("source_bytes", &self.source.len())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticProviderReadinessV1 {
    Unavailable,
    Indexing,
    Ready {
        generation: SemanticIndexGenerationV1,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticProviderStateV1 {
    pub provider: SemanticProviderProvenanceV1,
    pub readiness: SemanticProviderReadinessV1,
    pub capabilities: SemanticCapabilitySetV1,
}

impl SemanticProviderStateV1 {
    pub fn unavailable(provider: SemanticProviderProvenanceV1) -> Self {
        Self {
            provider,
            readiness: SemanticProviderReadinessV1::Unavailable,
            capabilities: SemanticCapabilitySetV1::bottom(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticSnapshotUpdateV1 {
    Admit(SemanticSnapshotV1),
    Close { target: SemanticTargetV1 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticProviderAdmissionV1 {
    Accepted,
    Unavailable,
    Refused,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticProviderSubmissionV1 {
    Accepted,
    Unsupported(SemanticQueryKindV1),
    Unavailable,
    Refused,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticProviderCompletionV1 {
    pub request: SemanticRequestRefV1,
    pub outcome: SemanticOutcomeV1,
}

/// Injected provider port. Construction and process lifecycle remain outside
/// this interface and under the operator-owned workspace-trust boundary.
pub trait SemanticProviderPortV1 {
    /// Provider-owned readiness and capability state. The service caches this
    /// projection only to detect transitions; callers cannot publish a state
    /// beside the privately owned provider.
    fn state(&self) -> SemanticProviderStateV1;
    fn synchronize(&mut self, update: &SemanticSnapshotUpdateV1) -> SemanticProviderAdmissionV1;
    fn submit(&mut self, request: &SemanticRequestV1) -> SemanticProviderSubmissionV1;
    fn cancel(&mut self, request: SemanticRequestRefV1);
    fn poll(&mut self) -> Option<SemanticProviderCompletionV1>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticServiceRefusalV1 {
    InvalidLimits,
    UnknownClient,
    ClientRetiring,
    ClientIsolation,
    UnknownDocument,
    DocumentAlreadyOpen,
    OutOfOrderRevision,
    SourceTargetMismatch,
    InputLimit,
    Capacity,
    DuplicateRequest,
    ProviderRefused,
    MalformedProviderReply,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticAdmissionOutcomeV1 {
    Accepted,
    Unavailable,
    Refused(SemanticServiceRefusalV1),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticSubmitOutcomeV1 {
    Accepted,
    Unsupported(SemanticQueryKindV1),
    Stale,
    Unavailable,
    Refused(SemanticServiceRefusalV1),
}

#[must_use = "provider transition evidence must be reconciled"]
#[derive(Clone, Eq, PartialEq)]
/// One atomic composition observation stage. `observed_state` triggered the
/// stage; `settled_state` is the final state after its bounded cancellation
/// signals. Composition may map the whole stage to one monotone epoch, but
/// must not publish either state independently or erase an ABA stage.
pub struct SemanticBridgeProviderTransitionV1 {
    observed_state: SemanticProviderStateV1,
    settled_state: SemanticProviderStateV1,
    invalidations: SemanticInvalidationReceiptV1,
}

impl SemanticBridgeProviderTransitionV1 {
    pub const fn observed_state(&self) -> &SemanticProviderStateV1 {
        &self.observed_state
    }
    /// Final state after all cancellation signals caused by this atomic stage.
    /// Pending work was already drained, so settling does not invalidate twice.
    pub const fn settled_state(&self) -> &SemanticProviderStateV1 {
        &self.settled_state
    }
    pub const fn invalidations(&self) -> &SemanticInvalidationReceiptV1 {
        &self.invalidations
    }
}

impl fmt::Debug for SemanticBridgeProviderTransitionV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SemanticBridgeProviderTransitionV1")
            .field(
                "settled_differs",
                &(self.observed_state != self.settled_state),
            )
            .field("invalidations", &self.invalidations)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticBridgeOpenAdmissionV1 {
    Accepted,
    Unavailable,
    Refused(SemanticServiceRefusalV1),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticBridgeChangeAdmissionV1 {
    Accepted {
        source_invalidations: SemanticInvalidationReceiptV1,
    },
    Unavailable,
    Refused(SemanticServiceRefusalV1),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticBridgeCloseAdmissionV1 {
    Accepted {
        source_invalidations: SemanticInvalidationReceiptV1,
    },
    Unavailable,
    Refused(SemanticServiceRefusalV1),
}

#[must_use = "bridge reports must be validated atomically"]
#[derive(Clone, Eq, PartialEq)]
pub struct SemanticBridgeOrderedReportV1<T> {
    provider_before: SemanticProviderStateV1,
    pre_provider: Option<SemanticBridgeProviderTransitionV1>,
    outcome: T,
    post_provider: Option<SemanticBridgeProviderTransitionV1>,
}

impl<T> SemanticBridgeOrderedReportV1<T> {
    pub const fn provider_before(&self) -> &SemanticProviderStateV1 {
        &self.provider_before
    }
    pub const fn pre_provider(&self) -> Option<&SemanticBridgeProviderTransitionV1> {
        self.pre_provider.as_ref()
    }
    pub const fn outcome(&self) -> &T {
        &self.outcome
    }
    pub const fn post_provider(&self) -> Option<&SemanticBridgeProviderTransitionV1> {
        self.post_provider.as_ref()
    }
    pub fn provider_after(&self) -> &SemanticProviderStateV1 {
        self.post_provider
            .as_ref()
            .or(self.pre_provider.as_ref())
            .map_or(
                &self.provider_before,
                SemanticBridgeProviderTransitionV1::settled_state,
            )
    }
}

impl<T: fmt::Debug> fmt::Debug for SemanticBridgeOrderedReportV1<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SemanticBridgeOrderedReportV1")
            .field(
                "pre_provider_invalidations",
                &self.pre_provider.as_ref().map(|v| v.invalidations.len()),
            )
            .field("outcome", &self.outcome)
            .field(
                "post_provider_invalidations",
                &self.post_provider.as_ref().map(|v| v.invalidations.len()),
            )
            .finish()
    }
}

pub type SemanticBridgeOpenReportV1 = SemanticBridgeOrderedReportV1<SemanticBridgeOpenAdmissionV1>;
pub type SemanticBridgeChangeReportV1 =
    SemanticBridgeOrderedReportV1<SemanticBridgeChangeAdmissionV1>;
pub type SemanticBridgeCloseReportV1 =
    SemanticBridgeOrderedReportV1<SemanticBridgeCloseAdmissionV1>;
pub type SemanticBridgeSubmissionReportV1 = SemanticBridgeOrderedReportV1<SemanticSubmitOutcomeV1>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticBridgeCancelActionV1 {
    Cancelled,
    SatisfiedByProviderTransition,
    NotPending,
    Refused(SemanticServiceRefusalV1),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticBridgeExpiryActionV1 {
    Expired,
    SatisfiedByProviderTransition,
    NotPending,
    Refused(SemanticServiceRefusalV1),
}

pub type SemanticBridgeCancelReportV1 = SemanticBridgeOrderedReportV1<SemanticBridgeCancelActionV1>;
pub type SemanticBridgeExpiryReportV1 = SemanticBridgeOrderedReportV1<SemanticBridgeExpiryActionV1>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticBridgePollFailureV1 {
    Poisoned,
    RuntimeUnavailable,
    DeliveryRefused,
}

/// Bridge-only fallible one-event polling. Legacy providers retain their
/// infallible `poll`; a bridge composition must implement this stronger port.
pub trait SemanticBridgeProviderPollV1: SemanticProviderPortV1 {
    fn poll_bridge(
        &mut self,
    ) -> Result<Option<SemanticProviderCompletionV1>, SemanticBridgePollFailureV1>;
}

#[derive(Clone, Eq, PartialEq)]
pub enum SemanticBridgePollDispositionV1 {
    Delivery(SemanticDeliveryV1),
    Late { request: SemanticRequestRefV1 },
    Orphan { request: SemanticRequestRefV1 },
}

impl fmt::Debug for SemanticBridgePollDispositionV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Delivery(_) => "Delivery",
            Self::Late { .. } => "Late",
            Self::Orphan { .. } => "Orphan",
        })
    }
}

#[derive(Clone, Eq, PartialEq)]
pub enum SemanticBridgePollOutcomeV1 {
    NoEvent,
    Failure(SemanticBridgePollFailureV1),
    Completion(SemanticBridgePollDispositionV1),
}

impl fmt::Debug for SemanticBridgePollOutcomeV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoEvent => formatter.write_str("NoEvent"),
            Self::Failure(failure) => formatter.debug_tuple("Failure").field(failure).finish(),
            Self::Completion(disposition) => formatter
                .debug_tuple("Completion")
                .field(disposition)
                .finish(),
        }
    }
}

pub type SemanticBridgePollReportV1 = SemanticBridgeOrderedReportV1<SemanticBridgePollOutcomeV1>;

#[must_use = "provider refresh evidence must be reconciled"]
#[derive(Clone, Eq, PartialEq)]
pub struct SemanticBridgeProviderRefreshV1 {
    provider_before: SemanticProviderStateV1,
    transition: Option<SemanticBridgeProviderTransitionV1>,
}

impl SemanticBridgeProviderRefreshV1 {
    pub const fn provider_before(&self) -> &SemanticProviderStateV1 {
        &self.provider_before
    }
    pub const fn transition(&self) -> Option<&SemanticBridgeProviderTransitionV1> {
        self.transition.as_ref()
    }
    pub fn provider_after(&self) -> &SemanticProviderStateV1 {
        self.transition.as_ref().map_or(
            &self.provider_before,
            SemanticBridgeProviderTransitionV1::settled_state,
        )
    }
}

impl fmt::Debug for SemanticBridgeProviderRefreshV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SemanticBridgeProviderRefreshV1")
            .field(
                "provider_invalidations",
                &self.transition.as_ref().map(|v| v.invalidations.len()),
            )
            .finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticDeliveryOutcomeV1 {
    Peer(SemanticOutcomeV1),
    Cancelled,
    TimedOut,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticDeliveryV1 {
    pub request: SemanticRequestRefV1,
    pub outcome: SemanticDeliveryOutcomeV1,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SemanticServiceUsageV1 {
    pub clients_admitted: u64,
    pub clients_retired: u64,
    pub client_retirement_attempts: u64,
    pub client_retirement_pending: u64,
    pub retirement_cancellation_signals: u64,
    pub retirement_deliveries_discarded: u64,
    pub retirement_closes_accepted: u64,
    pub retirement_closes_unavailable: u64,
    pub retirement_closes_refused: u64,
    pub snapshots_admitted: u64,
    pub snapshots_closed: u64,
    pub requests_submitted: u64,
    pub requests_completed: u64,
    pub requests_cancelled: u64,
    pub requests_timed_out: u64,
    pub requests_invalidated: u64,
    pub provider_replacements: u64,
    pub provider_replacement_refusals: u64,
    pub snapshots_replayed: u64,
    pub provider_unavailable: u64,
    pub provider_unsupported: u64,
    pub refusals: u64,
    pub late_completions: u64,
    pub orphan_completions: u64,
    pub current_clients: usize,
    pub high_water_clients: usize,
    pub current_quarantined_requests: usize,
    pub high_water_quarantined_requests: usize,
    pub current_snapshot_bytes: usize,
    pub high_water_snapshot_bytes: usize,
    pub saturated: bool,
}

/// Exact state after one bounded retirement pass. `Retired` means the client
/// partition and all source-bearing snapshots are gone. Quarantined request
/// references remain correlation-only residuals until provider replacement;
/// they prevent late completions from binding to a later client lifetime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticClientRetirementStateV1 {
    Absent,
    Pending {
        remaining_documents: usize,
        quarantined_requests: usize,
    },
    Retired {
        quarantined_requests: usize,
    },
}

/// Per-pass, content-safe retirement evidence. Cancellation is deliberately
/// reported as a signal: the provider port has no cancellation acknowledgement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticClientRetirementReportV1 {
    pub state: SemanticClientRetirementStateV1,
    pub cancellation_signals: usize,
    pub deliveries_discarded: usize,
    pub closes_accepted: usize,
    pub closes_unavailable: usize,
    pub closes_refused: usize,
}

/// Exact source-bearing or routable state that prevents a bridge provider
/// from leaving its service lifetime. Correlation quarantine is intentionally
/// absent: it may cross this boundary only with the retiring provider that can
/// still emit the corresponding completions.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SemanticBridgeProviderExtractionBlockersV1 {
    pub clients: usize,
    pub retiring_clients: usize,
    pub documents: usize,
    pub pending_requests: usize,
    pub queued_deliveries: usize,
}

impl SemanticBridgeProviderExtractionBlockersV1 {
    pub const fn is_empty(self) -> bool {
        self.clients == 0
            && self.retiring_clients == 0
            && self.documents == 0
            && self.pending_requests == 0
            && self.queued_deliveries == 0
    }
}

/// Failed checked extraction retains the complete bridge service so cleanup
/// can continue; no provider capability is duplicated or silently dropped.
#[must_use = "failed provider extraction retains the bridge cleanup obligation"]
pub struct SemanticBridgeProviderExtractionFailureV1<P> {
    service: SemanticBridgeServiceV1<P>,
    blockers: SemanticBridgeProviderExtractionBlockersV1,
}

impl<P> SemanticBridgeProviderExtractionFailureV1<P> {
    pub const fn blockers(&self) -> SemanticBridgeProviderExtractionBlockersV1 {
        self.blockers
    }

    pub fn into_service(self) -> SemanticBridgeServiceV1<P> {
        self.service
    }
}

impl<P> fmt::Debug for SemanticBridgeProviderExtractionFailureV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticBridgeProviderExtractionFailureV1")
            .field("blockers", &self.blockers)
            .finish()
    }
}

/// Provider capability after every source-bearing/routable service partition
/// has closed. Quarantined request identities remain exact, counted
/// correlation residue and require this provider to be retired rather than
/// returned to service.
#[must_use = "the extracted provider still requires confirmed retirement"]
pub struct SemanticBridgeRetiringProviderV1<P> {
    provider: P,
    quarantined_requests: usize,
}

impl<P> SemanticBridgeRetiringProviderV1<P> {
    pub const fn quarantined_requests(&self) -> usize {
        self.quarantined_requests
    }

    /// Transfers the provider only to an external retirement driver. This is
    /// not evidence that shutdown was graceful and the provider must never be
    /// installed into another semantic service lifetime.
    pub fn into_provider_for_retirement(self) -> P {
        self.provider
    }
}

impl<P> fmt::Debug for SemanticBridgeRetiringProviderV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticBridgeRetiringProviderV1")
            .field("provider", &"<owned>")
            .field("quarantined_requests", &self.quarantined_requests)
            .finish()
    }
}

/// Whether forced extraction may issue a final bounded cancellation/close
/// pass. A caller recovering from a provider panic can conservatively skip all
/// provider calls while still retaining exact abandonment evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticBridgeForcedEffectPolicyV1 {
    AttemptBounded,
    SkipUnsafeProvider,
}

/// Exact content-free state abandoned by a forced bridge extraction. Counts
/// describe the state at extraction entry; attempted plus skipped effects close
/// over the corresponding pending/document population.
#[must_use = "forced abandonment evidence must be joined with provider retirement"]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SemanticBridgeForcedAbandonmentReceiptV1 {
    pub clients_abandoned: usize,
    pub retiring_clients_abandoned: usize,
    pub documents_abandoned: usize,
    pub pending_requests_abandoned: usize,
    pub queued_deliveries_abandoned: usize,
    pub quarantined_requests: usize,
    pub cancellation_signals_attempted: usize,
    pub cancellation_signals_skipped: usize,
    pub close_attempts: usize,
    pub closes_accepted: usize,
    pub closes_unavailable: usize,
    pub closes_refused: usize,
    pub close_outcomes_unknown: usize,
    pub closes_skipped: usize,
    pub provider_panicked: bool,
}

impl SemanticBridgeForcedAbandonmentReceiptV1 {
    pub const fn is_closed(self) -> bool {
        self.cancellation_signals_attempted
            .saturating_add(self.cancellation_signals_skipped)
            == self.pending_requests_abandoned
            && self.close_attempts.saturating_add(self.closes_skipped) == self.documents_abandoned
            && self
                .closes_accepted
                .saturating_add(self.closes_unavailable)
                .saturating_add(self.closes_refused)
                .saturating_add(self.close_outcomes_unknown)
                == self.close_attempts
    }
}

/// Provider coupled to exact abandoned bridge state. This type can never be
/// converted into the graceful checked-extraction capability; it may only be
/// transferred to a forced retirement driver.
#[must_use = "forced provider extraction still requires confirmed forced retirement"]
pub struct SemanticBridgeForcedRetiringProviderV1<P> {
    provider: P,
    abandonment: SemanticBridgeForcedAbandonmentReceiptV1,
}

/// Linear hand-off capability for the forced retirement driver. Keeping the
/// provider coupled to its abandonment evidence prevents callers from
/// extracting a reusable provider directly from the bridge wrapper.
#[must_use = "forced retirement parts must be consumed by a terminal driver"]
pub struct SemanticBridgeForcedRetirementPartsV1<P> {
    provider: P,
    abandonment: SemanticBridgeForcedAbandonmentReceiptV1,
}

impl<P> SemanticBridgeForcedRetiringProviderV1<P> {
    pub const fn abandonment(&self) -> SemanticBridgeForcedAbandonmentReceiptV1 {
        self.abandonment
    }

    pub const fn is_forced_only(&self) -> bool {
        true
    }

    /// Transfers the provider and exact abandonment evidence only to a forced
    /// terminal driver. Neither extraction nor this transfer is graceful
    /// shutdown evidence, and the provider must never return to service.
    pub fn into_forced_retirement_parts(self) -> SemanticBridgeForcedRetirementPartsV1<P> {
        SemanticBridgeForcedRetirementPartsV1 {
            provider: self.provider,
            abandonment: self.abandonment,
        }
    }
}

impl<P> SemanticBridgeForcedRetirementPartsV1<P> {
    pub const fn abandonment(&self) -> SemanticBridgeForcedAbandonmentReceiptV1 {
        self.abandonment
    }

    pub const fn is_forced_only(&self) -> bool {
        true
    }

    /// Consumes the linear hand-off at the forced terminal driver boundary.
    /// The provider must never be returned to a live service.
    pub fn into_provider_and_abandonment(self) -> (P, SemanticBridgeForcedAbandonmentReceiptV1) {
        (self.provider, self.abandonment)
    }
}

impl<P> fmt::Debug for SemanticBridgeForcedRetiringProviderV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticBridgeForcedRetiringProviderV1")
            .field("provider", &"<owned>")
            .field("abandonment", &self.abandonment)
            .field("retirement", &"<forced-only>")
            .finish()
    }
}

impl<P> fmt::Debug for SemanticBridgeForcedRetirementPartsV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticBridgeForcedRetirementPartsV1")
            .field("provider", &"<owned>")
            .field("abandonment", &self.abandonment)
            .field("retirement", &"<forced-only>")
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticProviderReplacementRefusalV1 {
    CandidateNotReady,
    CandidateNotFresh,
    DeliveryCapacity,
    SynchronizationUnavailable,
    SynchronizationRefused,
    CandidateStateChanged,
}

/// Failed replacement retains ownership of the candidate so composition may
/// shut it down or inspect its content-safe operational state. The installed
/// provider and every service-owned snapshot, request, and delivery are
/// unchanged on this path.
pub struct SemanticProviderReplacementFailureV1<P> {
    candidate: P,
    refusal: SemanticProviderReplacementRefusalV1,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SemanticProviderRetirementUsageV1 {
    pub close_attempts: u64,
    pub closes_accepted: u64,
    pub closes_unavailable: u64,
    pub closes_refused: u64,
    pub saturated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticProviderRetirementStateV1 {
    Drained,
    Pending { remaining: usize },
}

/// Old provider plus the exact bounded close plan that remains after an
/// authoritative replacement. One `retry_drain` call attempts each remaining
/// close at most once. Composition may keep retrying under its own deadline or
/// extract the provider and escalate; neither path changes the installed
/// provider or claims that an unaccepted close occurred.
pub struct SemanticProviderRetirementV1<P> {
    provider: P,
    remaining: VecDeque<SemanticTargetV1>,
    usage: SemanticProviderRetirementUsageV1,
}

impl<P> fmt::Debug for SemanticProviderRetirementV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticProviderRetirementV1")
            .field("provider", &"<owned>")
            .field("remaining", &self.remaining.len())
            .field("usage", &self.usage)
            .finish()
    }
}

impl<P> SemanticProviderRetirementV1<P> {
    pub fn state(&self) -> SemanticProviderRetirementStateV1 {
        if self.remaining.is_empty() {
            SemanticProviderRetirementStateV1::Drained
        } else {
            SemanticProviderRetirementStateV1::Pending {
                remaining: self.remaining.len(),
            }
        }
    }

    pub const fn usage(&self) -> SemanticProviderRetirementUsageV1 {
        self.usage
    }

    /// Explicit escalation escape hatch. Extraction does not imply that the
    /// remaining close plan was accepted or that shutdown was graceful.
    pub fn into_provider(self) -> P {
        self.provider
    }

    pub fn into_provider_if_drained(self) -> Result<P, Self> {
        if self.remaining.is_empty() {
            Ok(self.provider)
        } else {
            Err(self)
        }
    }
}

impl<P: SemanticProviderPortV1> SemanticProviderRetirementV1<P> {
    pub fn provider_state(&self) -> SemanticProviderStateV1 {
        self.provider.state()
    }

    pub fn retry_drain(&mut self) -> SemanticProviderRetirementStateV1 {
        let attempts = self.remaining.len();
        for _ in 0..attempts {
            let target = self
                .remaining
                .pop_front()
                .expect("attempt count equals retained close plan");
            inc(&mut self.usage.close_attempts, &mut self.usage.saturated);
            match self
                .provider
                .synchronize(&SemanticSnapshotUpdateV1::Close { target })
            {
                SemanticProviderAdmissionV1::Accepted => {
                    inc(&mut self.usage.closes_accepted, &mut self.usage.saturated);
                }
                SemanticProviderAdmissionV1::Unavailable => {
                    inc(
                        &mut self.usage.closes_unavailable,
                        &mut self.usage.saturated,
                    );
                    self.remaining.push_back(target);
                }
                SemanticProviderAdmissionV1::Refused => {
                    inc(&mut self.usage.closes_refused, &mut self.usage.saturated);
                    self.remaining.push_back(target);
                }
            }
        }
        self.state()
    }
}

impl<P> fmt::Debug for SemanticProviderReplacementFailureV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticProviderReplacementFailureV1")
            .field("refusal", &self.refusal)
            .field("candidate", &"<owned>")
            .finish()
    }
}

impl<P> SemanticProviderReplacementFailureV1<P> {
    pub const fn refusal(&self) -> SemanticProviderReplacementRefusalV1 {
        self.refusal
    }

    pub fn into_candidate(self) -> P {
        self.candidate
    }
}

#[derive(Clone)]
struct DocumentState {
    owner: SemanticClientRefV1,
    snapshot: SemanticSnapshotV1,
}

#[derive(Clone)]
struct PendingRequest {
    owner: SemanticClientRefV1,
    request: SemanticRequestV1,
    index_generation: SemanticIndexGenerationV1,
    provider: SemanticProviderProvenanceV1,
}

/// Sovereign routing state. Client references are correlation/partition keys,
/// not authentication facts or authority grants; only `admit_client` installs
/// an operator-selected client partition.
pub struct SemanticServiceV1<P> {
    provider: P,
    limits: SemanticServiceLimitsV1,
    provider_state: SemanticProviderStateV1,
    clients: BTreeSet<SemanticClientRefV1>,
    retiring_clients: BTreeSet<SemanticClientRefV1>,
    documents: BTreeMap<SemanticDocumentRefV1, DocumentState>,
    pending: BTreeMap<SemanticRequestRefV1, PendingRequest>,
    quarantined_requests: BTreeMap<SemanticRequestRefV1, SemanticClientRefV1>,
    deliveries: BTreeMap<SemanticClientRefV1, VecDeque<SemanticDeliveryV1>>,
    snapshot_bytes: usize,
    usage: SemanticServiceUsageV1,
}

/// Newly constructed, empty single-client semantic service whose provider
/// effects return ordered invalidation evidence instead of synthetic
/// deliveries. This does not prove provider freshness or an empty provider
/// completion buffer. The legacy service is private so the two
/// terminalization algebras cannot be mixed.
pub struct SemanticBridgeServiceV1<P> {
    inner: SemanticServiceV1<P>,
}

impl<P: SemanticProviderPortV1> SemanticBridgeServiceV1<P> {
    pub fn try_new(
        provider: P,
        limits: SemanticServiceLimitsV1,
    ) -> Result<Self, SemanticServiceRefusalV1> {
        if limits.max_clients != 1 || limits.max_provider_polls_per_pump != 1 {
            return Err(SemanticServiceRefusalV1::InvalidLimits);
        }
        Ok(Self {
            inner: SemanticServiceV1::try_new(provider, limits)?,
        })
    }

    pub fn admit_client(
        &mut self,
        client: SemanticClientRefV1,
    ) -> Result<(), SemanticServiceRefusalV1> {
        self.inner.admit_client(client)
    }

    pub const fn usage(&self) -> SemanticServiceUsageV1 {
        self.inner.usage()
    }

    pub fn provider_state(&self) -> SemanticProviderStateV1 {
        self.inner.provider_state()
    }

    /// Advances bounded client cleanup without exposing the legacy service or
    /// its delivery algebra. A retired client may leave exact quarantine
    /// residue behind; that residue remains coupled to this provider until
    /// checked provider extraction.
    pub fn retire_client(
        &mut self,
        client: SemanticClientRefV1,
    ) -> SemanticClientRetirementReportV1 {
        self.inner.retire_client(client)
    }

    /// Consumes a fully source-free and route-free bridge service into a
    /// retirement-only provider capability. Quarantine is allowed because it
    /// remains coupled to the only provider that can emit those completions;
    /// all other live state is reported as an exact extraction blocker.
    pub fn try_into_retiring_provider(
        self,
    ) -> Result<
        SemanticBridgeRetiringProviderV1<P>,
        Box<SemanticBridgeProviderExtractionFailureV1<P>>,
    > {
        let blockers = SemanticBridgeProviderExtractionBlockersV1 {
            clients: self.inner.clients.len(),
            retiring_clients: self.inner.retiring_clients.len(),
            documents: self.inner.documents.len(),
            pending_requests: self.inner.pending.len(),
            queued_deliveries: self.inner.deliveries.values().map(VecDeque::len).sum(),
        };
        if !blockers.is_empty() {
            return Err(Box::new(SemanticBridgeProviderExtractionFailureV1 {
                service: self,
                blockers,
            }));
        }
        let SemanticBridgeServiceV1 { inner } = self;
        let quarantined_requests = inner.quarantined_requests.len();
        Ok(SemanticBridgeRetiringProviderV1 {
            provider: inner.provider,
            quarantined_requests,
        })
    }

    /// Consumes the entire bridge lifetime when ordinary client retirement can
    /// no longer close. Entry state is counted exactly. Under `AttemptBounded`,
    /// each still-pending request receives one cancellation signal and each
    /// still-open document receives one close attempt. Under
    /// `SkipUnsafeProvider`, no provider method is called. Either path returns
    /// a forced-only provider capability and can never claim graceful cleanup.
    pub fn force_into_retiring_provider(
        mut self,
        policy: SemanticBridgeForcedEffectPolicyV1,
    ) -> SemanticBridgeForcedRetiringProviderV1<P> {
        let clients_abandoned = self.inner.clients.len();
        let retiring_clients_abandoned = self.inner.retiring_clients.len();
        let documents_abandoned = self.inner.documents.len();
        let pending_requests_abandoned = self.inner.pending.len();
        let queued_deliveries_abandoned = self.inner.deliveries.values().map(VecDeque::len).sum();
        let quarantined_requests = self.inner.quarantined_requests.len();

        let mut cancellation_signals_attempted = 0usize;
        let mut cancellation_signals_skipped = pending_requests_abandoned;
        let mut close_attempts = 0usize;
        let mut closes_accepted = 0usize;
        let mut closes_unavailable = 0usize;
        let mut closes_refused = 0usize;
        let mut close_outcomes_unknown = 0usize;
        let mut closes_skipped = documents_abandoned;
        let mut provider_panicked = false;

        if policy == SemanticBridgeForcedEffectPolicyV1::AttemptBounded {
            let requests: Vec<_> = self.inner.pending.keys().copied().collect();
            for request in requests {
                cancellation_signals_attempted += 1;
                if catch_unwind(AssertUnwindSafe(|| self.inner.provider.cancel(request))).is_err() {
                    provider_panicked = true;
                    break;
                }
            }
            cancellation_signals_skipped =
                pending_requests_abandoned.saturating_sub(cancellation_signals_attempted);

            if !provider_panicked {
                let targets: Vec<_> = self
                    .inner
                    .documents
                    .values()
                    .map(|document| document.snapshot.target())
                    .collect();
                for target in targets {
                    close_attempts += 1;
                    match catch_unwind(AssertUnwindSafe(|| {
                        self.inner
                            .provider
                            .synchronize(&SemanticSnapshotUpdateV1::Close { target })
                    })) {
                        Ok(SemanticProviderAdmissionV1::Accepted) => closes_accepted += 1,
                        Ok(SemanticProviderAdmissionV1::Unavailable) => closes_unavailable += 1,
                        Ok(SemanticProviderAdmissionV1::Refused) => closes_refused += 1,
                        Err(_) => {
                            close_outcomes_unknown += 1;
                            provider_panicked = true;
                            break;
                        }
                    }
                }
            }
            closes_skipped = documents_abandoned.saturating_sub(close_attempts);
        }

        let abandonment = SemanticBridgeForcedAbandonmentReceiptV1 {
            clients_abandoned,
            retiring_clients_abandoned,
            documents_abandoned,
            pending_requests_abandoned,
            queued_deliveries_abandoned,
            quarantined_requests,
            cancellation_signals_attempted,
            cancellation_signals_skipped,
            close_attempts,
            closes_accepted,
            closes_unavailable,
            closes_refused,
            close_outcomes_unknown,
            closes_skipped,
            provider_panicked,
        };
        debug_assert!(abandonment.is_closed());
        SemanticBridgeForcedRetiringProviderV1 {
            provider: self.inner.provider,
            abandonment,
        }
    }

    pub fn refresh_provider_state(&mut self) -> SemanticBridgeProviderRefreshV1 {
        let provider_before = self.inner.provider_state.clone();
        let transition = self.inner.bridge_observe_provider();
        SemanticBridgeProviderRefreshV1 {
            provider_before,
            transition,
        }
    }

    pub fn open_snapshot(
        &mut self,
        client: SemanticClientRefV1,
        snapshot: SemanticSnapshotV1,
    ) -> SemanticBridgeOpenReportV1 {
        let refusal = self
            .inner
            .check_client(client)
            .err()
            .or_else(|| {
                (!snapshot.is_within_limits(self.inner.limits))
                    .then_some(SemanticServiceRefusalV1::InputLimit)
            })
            .or_else(|| {
                self.inner
                    .documents
                    .contains_key(&snapshot.target.document)
                    .then_some(SemanticServiceRefusalV1::DocumentAlreadyOpen)
            })
            .or_else(|| {
                (self.inner.documents.len() == self.inner.limits.max_documents)
                    .then_some(SemanticServiceRefusalV1::Capacity)
            });
        let candidate_bytes = self.inner.snapshot_bytes.checked_add(snapshot.source.len());
        let refusal = refusal.or_else(|| {
            candidate_bytes
                .filter(|bytes| *bytes <= self.inner.limits.max_total_snapshot_bytes)
                .is_none()
                .then_some(SemanticServiceRefusalV1::Capacity)
        });
        if let Some(reason) = refusal {
            return self.open_refused_report(reason);
        }
        let candidate_bytes = candidate_bytes.expect("validated snapshot capacity");
        let provider_before = self.inner.provider_state.clone();
        let pre_provider = self.inner.bridge_observe_provider();
        let result = self
            .inner
            .provider
            .synchronize(&SemanticSnapshotUpdateV1::Admit(snapshot.clone()));
        let outcome = match result {
            SemanticProviderAdmissionV1::Accepted => {
                let document = snapshot.target.document;
                self.inner.documents.insert(
                    document,
                    DocumentState {
                        owner: client,
                        snapshot,
                    },
                );
                self.inner.snapshot_bytes = candidate_bytes;
                self.inner.usage.current_snapshot_bytes = candidate_bytes;
                self.inner.usage.high_water_snapshot_bytes = self
                    .inner
                    .usage
                    .high_water_snapshot_bytes
                    .max(candidate_bytes);
                inc(
                    &mut self.inner.usage.snapshots_admitted,
                    &mut self.inner.usage.saturated,
                );
                SemanticBridgeOpenAdmissionV1::Accepted
            }
            SemanticProviderAdmissionV1::Unavailable => {
                inc(
                    &mut self.inner.usage.provider_unavailable,
                    &mut self.inner.usage.saturated,
                );
                SemanticBridgeOpenAdmissionV1::Unavailable
            }
            SemanticProviderAdmissionV1::Refused => {
                self.record_refusal();
                SemanticBridgeOpenAdmissionV1::Refused(SemanticServiceRefusalV1::ProviderRefused)
            }
        };
        let post_provider = self.inner.bridge_observe_provider();
        SemanticBridgeOrderedReportV1 {
            provider_before,
            pre_provider,
            outcome,
            post_provider,
        }
    }

    pub fn cancel(
        &mut self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> SemanticBridgeCancelReportV1 {
        if let Err(reason) = self.inner.check_client(client) {
            self.record_refusal();
            return self.terminal_report(SemanticBridgeCancelActionV1::Refused(reason));
        }
        if self
            .inner
            .pending
            .get(&request)
            .is_some_and(|value| value.owner != client)
        {
            self.record_refusal();
            return self.terminal_report(SemanticBridgeCancelActionV1::Refused(
                SemanticServiceRefusalV1::ClientIsolation,
            ));
        }
        let provider_before = self.inner.provider_state.clone();
        let pre_provider = self.inner.bridge_observe_provider();
        let satisfied = pre_provider.as_ref().is_some_and(|transition| {
            transition
                .invalidations()
                .iter()
                .any(|value| *value == request)
        });
        let outcome = if satisfied {
            SemanticBridgeCancelActionV1::SatisfiedByProviderTransition
        } else if self.inner.pending.remove(&request).is_some() {
            self.inner.quarantine(client, request);
            self.inner.provider.cancel(request);
            inc(
                &mut self.inner.usage.requests_cancelled,
                &mut self.inner.usage.saturated,
            );
            SemanticBridgeCancelActionV1::Cancelled
        } else {
            SemanticBridgeCancelActionV1::NotPending
        };
        let post_provider = self.inner.bridge_observe_provider();
        SemanticBridgeOrderedReportV1 {
            provider_before,
            pre_provider,
            outcome,
            post_provider,
        }
    }

    pub fn expire(
        &mut self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> SemanticBridgeExpiryReportV1 {
        if let Err(reason) = self.inner.check_client(client) {
            self.record_refusal();
            return self.terminal_report(SemanticBridgeExpiryActionV1::Refused(reason));
        }
        if self
            .inner
            .pending
            .get(&request)
            .is_some_and(|value| value.owner != client)
        {
            self.record_refusal();
            return self.terminal_report(SemanticBridgeExpiryActionV1::Refused(
                SemanticServiceRefusalV1::ClientIsolation,
            ));
        }
        let provider_before = self.inner.provider_state.clone();
        let pre_provider = self.inner.bridge_observe_provider();
        let satisfied = pre_provider.as_ref().is_some_and(|transition| {
            transition
                .invalidations()
                .iter()
                .any(|value| *value == request)
        });
        let outcome = if satisfied {
            SemanticBridgeExpiryActionV1::SatisfiedByProviderTransition
        } else if self.inner.pending.remove(&request).is_some() {
            self.inner.quarantine(client, request);
            self.inner.provider.cancel(request);
            inc(
                &mut self.inner.usage.requests_timed_out,
                &mut self.inner.usage.saturated,
            );
            SemanticBridgeExpiryActionV1::Expired
        } else {
            SemanticBridgeExpiryActionV1::NotPending
        };
        let post_provider = self.inner.bridge_observe_provider();
        SemanticBridgeOrderedReportV1 {
            provider_before,
            pre_provider,
            outcome,
            post_provider,
        }
    }

    pub fn change_snapshot(
        &mut self,
        client: SemanticClientRefV1,
        snapshot: SemanticSnapshotV1,
    ) -> SemanticBridgeChangeReportV1 {
        if let Err(reason) = self.inner.check_client(client) {
            return self.change_refused_report(reason);
        }
        if !snapshot.is_within_limits(self.inner.limits) {
            return self.change_refused_report(SemanticServiceRefusalV1::InputLimit);
        }
        let Some(current) = self.inner.documents.get(&snapshot.target.document) else {
            return self.change_refused_report(SemanticServiceRefusalV1::UnknownDocument);
        };
        if current.owner != client {
            return self.change_refused_report(SemanticServiceRefusalV1::ClientIsolation);
        }
        if snapshot.target.source_revision <= current.snapshot.target.source_revision {
            return self.change_refused_report(SemanticServiceRefusalV1::OutOfOrderRevision);
        }
        let replaced_bytes = current.snapshot.source.len();
        let Some(candidate_bytes) = self
            .inner
            .snapshot_bytes
            .checked_sub(replaced_bytes)
            .and_then(|bytes| bytes.checked_add(snapshot.source.len()))
            .filter(|bytes| *bytes <= self.inner.limits.max_total_snapshot_bytes)
        else {
            return self.change_refused_report(SemanticServiceRefusalV1::Capacity);
        };
        let provider_before = self.inner.provider_state.clone();
        let pre_provider = self.inner.bridge_observe_provider();
        let result = self
            .inner
            .provider
            .synchronize(&SemanticSnapshotUpdateV1::Admit(snapshot.clone()));
        let outcome = match result {
            SemanticProviderAdmissionV1::Accepted => {
                let document = snapshot.target.document;
                let requests = self.inner.bridge_invalidate_document(document);
                self.inner.documents.insert(
                    document,
                    DocumentState {
                        owner: client,
                        snapshot,
                    },
                );
                self.inner.snapshot_bytes = candidate_bytes;
                self.inner.usage.current_snapshot_bytes = candidate_bytes;
                self.inner.usage.high_water_snapshot_bytes = self
                    .inner
                    .usage
                    .high_water_snapshot_bytes
                    .max(candidate_bytes);
                inc(
                    &mut self.inner.usage.snapshots_admitted,
                    &mut self.inner.usage.saturated,
                );
                SemanticBridgeChangeAdmissionV1::Accepted {
                    source_invalidations: semantic_invalidation_receipt(
                        SemanticInvalidationCauseV1::SourceChanged,
                        requests,
                        self.inner.limits.max_pending,
                    ),
                }
            }
            SemanticProviderAdmissionV1::Unavailable => {
                inc(
                    &mut self.inner.usage.provider_unavailable,
                    &mut self.inner.usage.saturated,
                );
                SemanticBridgeChangeAdmissionV1::Unavailable
            }
            SemanticProviderAdmissionV1::Refused => {
                self.record_refusal();
                SemanticBridgeChangeAdmissionV1::Refused(SemanticServiceRefusalV1::ProviderRefused)
            }
        };
        let post_provider = self.inner.bridge_observe_provider();
        SemanticBridgeOrderedReportV1 {
            provider_before,
            pre_provider,
            outcome,
            post_provider,
        }
    }

    pub fn close_snapshot(
        &mut self,
        client: SemanticClientRefV1,
        target: SemanticTargetV1,
    ) -> SemanticBridgeCloseReportV1 {
        if let Err(reason) = self.inner.check_client(client) {
            return self.close_refused_report(reason);
        }
        let Some(current) = self.inner.documents.get(&target.document) else {
            return self.close_refused_report(SemanticServiceRefusalV1::UnknownDocument);
        };
        if current.owner != client {
            return self.close_refused_report(SemanticServiceRefusalV1::ClientIsolation);
        }
        if current.snapshot.target != target {
            return self.close_refused_report(SemanticServiceRefusalV1::SourceTargetMismatch);
        }
        let provider_before = self.inner.provider_state.clone();
        let pre_provider = self.inner.bridge_observe_provider();
        let result = self
            .inner
            .provider
            .synchronize(&SemanticSnapshotUpdateV1::Close { target });
        let outcome = match result {
            SemanticProviderAdmissionV1::Accepted => {
                let requests = self.inner.bridge_invalidate_document(target.document);
                let removed = self
                    .inner
                    .documents
                    .remove(&target.document)
                    .expect("validated document remains open");
                self.inner.snapshot_bytes = self
                    .inner
                    .snapshot_bytes
                    .saturating_sub(removed.snapshot.source.len());
                self.inner.usage.current_snapshot_bytes = self.inner.snapshot_bytes;
                inc(
                    &mut self.inner.usage.snapshots_closed,
                    &mut self.inner.usage.saturated,
                );
                SemanticBridgeCloseAdmissionV1::Accepted {
                    source_invalidations: semantic_invalidation_receipt(
                        SemanticInvalidationCauseV1::SourceClosed,
                        requests,
                        self.inner.limits.max_pending,
                    ),
                }
            }
            SemanticProviderAdmissionV1::Unavailable => {
                inc(
                    &mut self.inner.usage.provider_unavailable,
                    &mut self.inner.usage.saturated,
                );
                SemanticBridgeCloseAdmissionV1::Unavailable
            }
            SemanticProviderAdmissionV1::Refused => {
                self.record_refusal();
                SemanticBridgeCloseAdmissionV1::Refused(SemanticServiceRefusalV1::ProviderRefused)
            }
        };
        let post_provider = self.inner.bridge_observe_provider();
        SemanticBridgeOrderedReportV1 {
            provider_before,
            pre_provider,
            outcome,
            post_provider,
        }
    }

    pub fn submit(
        &mut self,
        client: SemanticClientRefV1,
        request: SemanticRequestV1,
    ) -> SemanticBridgeSubmissionReportV1 {
        if let Err(reason) = self.inner.check_client(client) {
            return self.submission_without_effect(SemanticSubmitOutcomeV1::Refused(reason), true);
        }
        let Some(document) = self.inner.documents.get(&request.target.document) else {
            return self.submission_without_effect(
                SemanticSubmitOutcomeV1::Refused(SemanticServiceRefusalV1::UnknownDocument),
                true,
            );
        };
        if document.owner != client {
            return self.submission_without_effect(
                SemanticSubmitOutcomeV1::Refused(SemanticServiceRefusalV1::ClientIsolation),
                true,
            );
        }
        if document.snapshot.target != request.target
            || document.snapshot.language != request.language.as_str()
        {
            return self.submission_without_effect(SemanticSubmitOutcomeV1::Stale, false);
        }
        if !request.is_within_limits(self.inner.limits.peer)
            || !request_offsets_within_snapshot(&request, &document.snapshot.source)
        {
            return self.submission_without_effect(
                SemanticSubmitOutcomeV1::Refused(SemanticServiceRefusalV1::InputLimit),
                true,
            );
        }
        if self.inner.pending.contains_key(&request.request)
            || self
                .inner
                .quarantined_requests
                .contains_key(&request.request)
        {
            return self.submission_without_effect(
                SemanticSubmitOutcomeV1::Refused(SemanticServiceRefusalV1::DuplicateRequest),
                true,
            );
        }
        // Pending work and retired correlation have different lifetimes.
        // `max_pending` bounds live provider obligations; exact quarantine is
        // separately bounded by the protocol's per-session request ceiling
        // and remains non-reusable until this provider is retired.
        if self.inner.pending.len() >= self.inner.limits.max_pending
            || self
                .inner
                .pending
                .len()
                .saturating_add(self.inner.quarantined_requests.len())
                >= usize::try_from(MAX_SEMANTIC_REQUESTS_V1).unwrap_or(usize::MAX)
        {
            return self.submission_without_effect(
                SemanticSubmitOutcomeV1::Refused(SemanticServiceRefusalV1::Capacity),
                true,
            );
        }
        let owned_pending = self
            .inner
            .pending
            .values()
            .filter(|pending| pending.owner == client)
            .count();
        if owned_pending >= self.inner.limits.max_deliveries {
            return self.submission_without_effect(
                SemanticSubmitOutcomeV1::Refused(SemanticServiceRefusalV1::Capacity),
                true,
            );
        }

        let provider_before = self.inner.provider_state.clone();
        let pre_provider = self.inner.bridge_observe_provider();
        let outcome = if let SemanticProviderReadinessV1::Ready { generation } =
            self.inner.provider_state.readiness
        {
            if request
                .expected_index
                .is_some_and(|expected| expected != generation)
            {
                SemanticSubmitOutcomeV1::Stale
            } else if !request
                .query
                .required_capabilities()
                .is_subset_of(self.inner.provider_state.capabilities)
            {
                inc(
                    &mut self.inner.usage.provider_unsupported,
                    &mut self.inner.usage.saturated,
                );
                SemanticSubmitOutcomeV1::Unsupported(request.query.kind())
            } else {
                let submission = self.inner.provider.submit(&request);
                match submission {
                    SemanticProviderSubmissionV1::Accepted => {
                        let request_ref = request.request;
                        let provider = self.inner.provider_state.provider.clone();
                        self.inner.pending.insert(
                            request_ref,
                            PendingRequest {
                                owner: client,
                                request,
                                index_generation: generation,
                                provider,
                            },
                        );
                        inc(
                            &mut self.inner.usage.requests_submitted,
                            &mut self.inner.usage.saturated,
                        );
                        SemanticSubmitOutcomeV1::Accepted
                    }
                    SemanticProviderSubmissionV1::Unsupported(kind)
                        if kind == request.query.kind() =>
                    {
                        inc(
                            &mut self.inner.usage.provider_unsupported,
                            &mut self.inner.usage.saturated,
                        );
                        SemanticSubmitOutcomeV1::Unsupported(kind)
                    }
                    SemanticProviderSubmissionV1::Unavailable => {
                        inc(
                            &mut self.inner.usage.provider_unavailable,
                            &mut self.inner.usage.saturated,
                        );
                        SemanticSubmitOutcomeV1::Unavailable
                    }
                    SemanticProviderSubmissionV1::Unsupported(_) => self
                        .inner
                        .submit_refusal(SemanticServiceRefusalV1::MalformedProviderReply),
                    SemanticProviderSubmissionV1::Refused => self
                        .inner
                        .submit_refusal(SemanticServiceRefusalV1::ProviderRefused),
                }
            }
        } else {
            inc(
                &mut self.inner.usage.provider_unavailable,
                &mut self.inner.usage.saturated,
            );
            SemanticSubmitOutcomeV1::Unavailable
        };
        let post_provider = self.inner.bridge_observe_provider();
        SemanticBridgeOrderedReportV1 {
            provider_before,
            pre_provider,
            outcome,
            post_provider,
        }
    }

    fn open_refused_report(
        &mut self,
        reason: SemanticServiceRefusalV1,
    ) -> SemanticBridgeOpenReportV1 {
        let provider_before = self.inner.provider_state.clone();
        self.record_refusal();
        SemanticBridgeOrderedReportV1 {
            provider_before,
            pre_provider: None,
            outcome: SemanticBridgeOpenAdmissionV1::Refused(reason),
            post_provider: None,
        }
    }

    fn change_refused_report(
        &mut self,
        reason: SemanticServiceRefusalV1,
    ) -> SemanticBridgeChangeReportV1 {
        let provider_before = self.inner.provider_state.clone();
        self.record_refusal();
        SemanticBridgeOrderedReportV1 {
            provider_before,
            pre_provider: None,
            outcome: SemanticBridgeChangeAdmissionV1::Refused(reason),
            post_provider: None,
        }
    }

    fn close_refused_report(
        &mut self,
        reason: SemanticServiceRefusalV1,
    ) -> SemanticBridgeCloseReportV1 {
        let provider_before = self.inner.provider_state.clone();
        self.record_refusal();
        SemanticBridgeOrderedReportV1 {
            provider_before,
            pre_provider: None,
            outcome: SemanticBridgeCloseAdmissionV1::Refused(reason),
            post_provider: None,
        }
    }

    fn submission_without_effect(
        &mut self,
        outcome: SemanticSubmitOutcomeV1,
        refused: bool,
    ) -> SemanticBridgeSubmissionReportV1 {
        if refused {
            self.record_refusal();
        }
        SemanticBridgeOrderedReportV1 {
            provider_before: self.inner.provider_state.clone(),
            pre_provider: None,
            outcome,
            post_provider: None,
        }
    }

    fn terminal_report<T>(&self, outcome: T) -> SemanticBridgeOrderedReportV1<T> {
        SemanticBridgeOrderedReportV1 {
            provider_before: self.inner.provider_state.clone(),
            pre_provider: None,
            outcome,
            post_provider: None,
        }
    }

    fn record_refusal(&mut self) {
        inc(
            &mut self.inner.usage.refusals,
            &mut self.inner.usage.saturated,
        );
    }
}

impl<P: SemanticBridgeProviderPollV1> SemanticBridgeServiceV1<P> {
    pub fn poll_once(&mut self) -> SemanticBridgePollReportV1 {
        let provider_before = self.inner.provider_state.clone();
        let pre_provider = self.inner.bridge_observe_provider();
        let polled = self.inner.provider.poll_bridge();
        let (outcome, post_provider) = match polled {
            Err(failure) => (SemanticBridgePollOutcomeV1::Failure(failure), None),
            Ok(None) => (
                SemanticBridgePollOutcomeV1::NoEvent,
                self.inner.bridge_observe_provider(),
            ),
            Ok(Some(completion)) => {
                let post_provider = self.inner.bridge_observe_provider();
                let request = completion.request;
                let outcome = if self.inner.quarantined_requests.contains_key(&request) {
                    inc(
                        &mut self.inner.usage.late_completions,
                        &mut self.inner.usage.saturated,
                    );
                    SemanticBridgePollOutcomeV1::Completion(SemanticBridgePollDispositionV1::Late {
                        request,
                    })
                } else if let Some(pending) = self.inner.pending.remove(&request) {
                    // A successfully correlated completion terminalizes the
                    // identity but does not make it reusable. Providers may
                    // duplicate or delay replies; retaining the exact owner
                    // keeps every later copy late rather than allowing it to
                    // bind to a new request with the same opaque reference.
                    self.inner.quarantine(pending.owner, request);
                    let outcome = validate_completion(
                        &pending.request,
                        pending.index_generation,
                        &pending.provider,
                        completion.outcome,
                        self.inner.limits.peer,
                    );
                    let outcome = validate_partitioned_result(
                        &pending.request,
                        pending.owner,
                        outcome,
                        &self.inner.documents,
                    );
                    inc(
                        &mut self.inner.usage.requests_completed,
                        &mut self.inner.usage.saturated,
                    );
                    SemanticBridgePollOutcomeV1::Completion(
                        SemanticBridgePollDispositionV1::Delivery(SemanticDeliveryV1 {
                            request,
                            outcome: SemanticDeliveryOutcomeV1::Peer(outcome),
                        }),
                    )
                } else {
                    inc(
                        &mut self.inner.usage.orphan_completions,
                        &mut self.inner.usage.saturated,
                    );
                    SemanticBridgePollOutcomeV1::Completion(
                        SemanticBridgePollDispositionV1::Orphan { request },
                    )
                };
                (outcome, post_provider)
            }
        };
        SemanticBridgeOrderedReportV1 {
            provider_before,
            pre_provider,
            outcome,
            post_provider,
        }
    }
}

impl<P: SemanticProviderPortV1> SemanticServiceV1<P> {
    pub fn try_new(
        provider: P,
        limits: SemanticServiceLimitsV1,
    ) -> Result<Self, SemanticServiceRefusalV1> {
        if !limits.is_valid() {
            return Err(SemanticServiceRefusalV1::InvalidLimits);
        }
        let provider_state = provider.state();
        Ok(Self {
            provider,
            limits,
            provider_state,
            clients: BTreeSet::new(),
            retiring_clients: BTreeSet::new(),
            documents: BTreeMap::new(),
            pending: BTreeMap::new(),
            quarantined_requests: BTreeMap::new(),
            deliveries: BTreeMap::new(),
            snapshot_bytes: 0,
            usage: SemanticServiceUsageV1::default(),
        })
    }

    pub const fn usage(&self) -> SemanticServiceUsageV1 {
        self.usage
    }

    pub fn provider_state(&self) -> SemanticProviderStateV1 {
        self.provider_state.clone()
    }

    /// Replaces the privately owned provider only after a ready, fresh
    /// candidate has accepted a deterministic replay of every retained exact
    /// snapshot without changing its projected state. Failure returns the
    /// candidate and leaves the installed product untouched. Success returns
    /// an owned retirement product whose exact bounded close plan can be
    /// driven to completion before composition-owned graceful shutdown.
    pub fn try_replace_provider(
        &mut self,
        mut candidate: P,
    ) -> Result<SemanticProviderRetirementV1<P>, SemanticProviderReplacementFailureV1<P>> {
        let candidate_state = candidate.state();
        let SemanticProviderReadinessV1::Ready {
            generation: candidate_generation,
        } = candidate_state.readiness
        else {
            return self.replacement_failure(
                candidate,
                SemanticProviderReplacementRefusalV1::CandidateNotReady,
            );
        };
        let current_generation = match self.provider_state.readiness {
            SemanticProviderReadinessV1::Ready { generation } => Some(generation),
            SemanticProviderReadinessV1::Unavailable | SemanticProviderReadinessV1::Indexing => {
                None
            }
        };
        if candidate_state.provider == self.provider_state.provider
            || current_generation == Some(candidate_generation)
        {
            return self.replacement_failure(
                candidate,
                SemanticProviderReplacementRefusalV1::CandidateNotFresh,
            );
        }
        if self.clients.iter().any(|client| {
            let queued = self.deliveries.get(client).map_or(0, VecDeque::len);
            let pending = self
                .pending
                .values()
                .filter(|request| request.owner == *client)
                .count();
            queued.saturating_add(pending) > self.limits.max_deliveries
        }) {
            return self.replacement_failure(
                candidate,
                SemanticProviderReplacementRefusalV1::DeliveryCapacity,
            );
        }

        let snapshots: Vec<_> = self
            .documents
            .values()
            .map(|document| document.snapshot.clone())
            .collect();
        for snapshot in &snapshots {
            let admission =
                candidate.synchronize(&SemanticSnapshotUpdateV1::Admit(snapshot.clone()));
            if candidate.state() != candidate_state {
                return self.replacement_failure(
                    candidate,
                    SemanticProviderReplacementRefusalV1::CandidateStateChanged,
                );
            }
            match admission {
                SemanticProviderAdmissionV1::Accepted => {}
                SemanticProviderAdmissionV1::Unavailable => {
                    return self.replacement_failure(
                        candidate,
                        SemanticProviderReplacementRefusalV1::SynchronizationUnavailable,
                    );
                }
                SemanticProviderAdmissionV1::Refused => {
                    return self.replacement_failure(
                        candidate,
                        SemanticProviderReplacementRefusalV1::SynchronizationRefused,
                    );
                }
            }
        }

        let mut old_provider = std::mem::replace(&mut self.provider, candidate);
        self.provider_state = candidate_state;
        self.invalidate_all_pending_with(&mut old_provider);
        // The installed candidate is fresh and cannot emit completions from
        // the extracted provider. Correlation quarantine is therefore closed.
        self.quarantined_requests.clear();
        self.usage.current_quarantined_requests = 0;
        for _ in &snapshots {
            inc(
                &mut self.usage.snapshots_replayed,
                &mut self.usage.saturated,
            );
        }
        inc(
            &mut self.usage.provider_replacements,
            &mut self.usage.saturated,
        );
        let mut retirement = SemanticProviderRetirementV1 {
            provider: old_provider,
            remaining: snapshots
                .into_iter()
                .map(|snapshot| snapshot.target())
                .collect(),
            usage: SemanticProviderRetirementUsageV1::default(),
        };
        retirement.retry_drain();
        Ok(retirement)
    }

    /// Refreshes the cached projection from the privately owned provider.
    /// A real transition invalidates every request admitted under the prior
    /// generation/capability product; repeating the same state is inert.
    pub fn refresh_provider_state(&mut self) {
        let state = self.provider.state();
        if self.provider_state != state {
            self.provider_state = state;
            self.invalidate_all_pending();
        }
    }

    fn bridge_observe_provider(&mut self) -> Option<SemanticBridgeProviderTransitionV1> {
        let observed_state = self.provider.state();
        if self.provider_state == observed_state {
            return None;
        }
        let requests = self.bridge_invalidate_all_pending();
        let settled_state = self.provider.state();
        self.provider_state = settled_state.clone();
        Some(SemanticBridgeProviderTransitionV1 {
            observed_state,
            settled_state,
            invalidations: semantic_invalidation_receipt(
                SemanticInvalidationCauseV1::ProviderStateChanged,
                requests,
                self.limits.max_pending,
            ),
        })
    }

    /// Trusted composition input. Possession of the opaque value alone does not
    /// admit another client or broaden this set.
    pub fn admit_client(
        &mut self,
        client: SemanticClientRefV1,
    ) -> Result<(), SemanticServiceRefusalV1> {
        if self.retiring_clients.contains(&client) {
            return Err(SemanticServiceRefusalV1::ClientRetiring);
        }
        if self.clients.contains(&client) {
            return Ok(());
        }
        if self.clients.len() == self.limits.max_clients {
            return Err(SemanticServiceRefusalV1::Capacity);
        }
        self.clients.insert(client);
        self.deliveries.insert(client, VecDeque::new());
        inc(&mut self.usage.clients_admitted, &mut self.usage.saturated);
        self.usage.current_clients = self.clients.len();
        self.usage.high_water_clients = self.usage.high_water_clients.max(self.clients.len());
        Ok(())
    }

    /// Begins or advances one trusted-composition retirement. The pass is
    /// bounded by service limits: it signals cancellation for each owned
    /// pending request once, explicitly discards the client's queued
    /// deliveries, and attempts each remaining provider close once. Refused or
    /// unavailable closes retain the fenced client and exact snapshots for a
    /// later retry. No operation may enter a fenced partition.
    pub fn retire_client(
        &mut self,
        client: SemanticClientRefV1,
    ) -> SemanticClientRetirementReportV1 {
        inc(
            &mut self.usage.client_retirement_attempts,
            &mut self.usage.saturated,
        );
        if !self.clients.contains(&client) {
            return SemanticClientRetirementReportV1 {
                state: SemanticClientRetirementStateV1::Absent,
                cancellation_signals: 0,
                deliveries_discarded: 0,
                closes_accepted: 0,
                closes_unavailable: 0,
                closes_refused: 0,
            };
        }

        let first_pass = self.retiring_clients.insert(client);
        let mut cancellation_signals = 0usize;
        let mut deliveries_discarded = 0usize;
        if first_pass {
            let requests: Vec<_> = self
                .pending
                .iter()
                .filter_map(|(request, pending)| (pending.owner == client).then_some(*request))
                .collect();
            for request in requests {
                self.pending.remove(&request);
                self.quarantine(client, request);
                self.provider.cancel(request);
                cancellation_signals += 1;
                inc(
                    &mut self.usage.retirement_cancellation_signals,
                    &mut self.usage.saturated,
                );
            }
            if let Some(deliveries) = self.deliveries.get_mut(&client) {
                deliveries_discarded = deliveries.len();
                deliveries.clear();
            }
            add_usize_to_counter(
                &mut self.usage.retirement_deliveries_discarded,
                deliveries_discarded,
                &mut self.usage.saturated,
            );
        }

        let targets: Vec<_> = self
            .documents
            .values()
            .filter_map(|document| (document.owner == client).then_some(document.snapshot.target()))
            .collect();
        let mut closes_accepted = 0usize;
        let mut closes_unavailable = 0usize;
        let mut closes_refused = 0usize;
        for target in targets {
            self.refresh_provider_state();
            let admission = self
                .provider
                .synchronize(&SemanticSnapshotUpdateV1::Close { target });
            self.refresh_provider_state();
            match admission {
                SemanticProviderAdmissionV1::Accepted => {
                    self.invalidate_document(target.document, false);
                    if let Some(removed) = self.documents.remove(&target.document) {
                        self.snapshot_bytes = self
                            .snapshot_bytes
                            .saturating_sub(removed.snapshot.source.len());
                        self.usage.current_snapshot_bytes = self.snapshot_bytes;
                        inc(&mut self.usage.snapshots_closed, &mut self.usage.saturated);
                    }
                    closes_accepted += 1;
                    inc(
                        &mut self.usage.retirement_closes_accepted,
                        &mut self.usage.saturated,
                    );
                }
                SemanticProviderAdmissionV1::Unavailable => {
                    closes_unavailable += 1;
                    inc(
                        &mut self.usage.retirement_closes_unavailable,
                        &mut self.usage.saturated,
                    );
                }
                SemanticProviderAdmissionV1::Refused => {
                    closes_refused += 1;
                    inc(
                        &mut self.usage.retirement_closes_refused,
                        &mut self.usage.saturated,
                    );
                }
            }
        }

        let remaining_documents = self
            .documents
            .values()
            .filter(|document| document.owner == client)
            .count();
        let quarantined_requests = self
            .quarantined_requests
            .values()
            .filter(|owner| **owner == client)
            .count();
        let state = if remaining_documents == 0 {
            self.retiring_clients.remove(&client);
            self.clients.remove(&client);
            self.deliveries.remove(&client);
            self.usage.current_clients = self.clients.len();
            inc(&mut self.usage.clients_retired, &mut self.usage.saturated);
            SemanticClientRetirementStateV1::Retired {
                quarantined_requests,
            }
        } else {
            inc(
                &mut self.usage.client_retirement_pending,
                &mut self.usage.saturated,
            );
            SemanticClientRetirementStateV1::Pending {
                remaining_documents,
                quarantined_requests,
            }
        };
        SemanticClientRetirementReportV1 {
            state,
            cancellation_signals,
            deliveries_discarded,
            closes_accepted,
            closes_unavailable,
            closes_refused,
        }
    }

    pub fn open_snapshot(
        &mut self,
        client: SemanticClientRefV1,
        snapshot: SemanticSnapshotV1,
    ) -> SemanticAdmissionOutcomeV1 {
        if let Err(refusal) = self.check_client(client) {
            return self.admission_refusal(refusal);
        }
        if !snapshot.is_within_limits(self.limits) {
            return self.admission_refusal(SemanticServiceRefusalV1::InputLimit);
        }
        if self.documents.contains_key(&snapshot.target.document) {
            return self.admission_refusal(SemanticServiceRefusalV1::DocumentAlreadyOpen);
        }
        if self.documents.len() == self.limits.max_documents {
            return self.admission_refusal(SemanticServiceRefusalV1::Capacity);
        }
        self.commit_snapshot(client, snapshot, false)
    }

    pub fn change_snapshot(
        &mut self,
        client: SemanticClientRefV1,
        snapshot: SemanticSnapshotV1,
    ) -> SemanticAdmissionOutcomeV1 {
        if let Err(refusal) = self.check_client(client) {
            return self.admission_refusal(refusal);
        }
        if !snapshot.is_within_limits(self.limits) {
            return self.admission_refusal(SemanticServiceRefusalV1::InputLimit);
        }
        let Some(current) = self.documents.get(&snapshot.target.document) else {
            return self.admission_refusal(SemanticServiceRefusalV1::UnknownDocument);
        };
        if current.owner != client {
            return self.admission_refusal(SemanticServiceRefusalV1::ClientIsolation);
        }
        if snapshot.target.source_revision <= current.snapshot.target.source_revision {
            return self.admission_refusal(SemanticServiceRefusalV1::OutOfOrderRevision);
        }
        self.commit_snapshot(client, snapshot, true)
    }

    pub fn close_snapshot(
        &mut self,
        client: SemanticClientRefV1,
        target: SemanticTargetV1,
    ) -> SemanticAdmissionOutcomeV1 {
        if let Err(refusal) = self.check_client(client) {
            return self.admission_refusal(refusal);
        }
        let Some(current) = self.documents.get(&target.document) else {
            return self.admission_refusal(SemanticServiceRefusalV1::UnknownDocument);
        };
        if current.owner != client {
            return self.admission_refusal(SemanticServiceRefusalV1::ClientIsolation);
        }
        if current.snapshot.target != target {
            return self.admission_refusal(SemanticServiceRefusalV1::SourceTargetMismatch);
        }
        self.refresh_provider_state();
        let admission = self
            .provider
            .synchronize(&SemanticSnapshotUpdateV1::Close { target });
        self.refresh_provider_state();
        match admission {
            SemanticProviderAdmissionV1::Accepted => {
                self.invalidate_document(target.document, false);
                let removed = self
                    .documents
                    .remove(&target.document)
                    .expect("validated document remains open");
                self.snapshot_bytes = self
                    .snapshot_bytes
                    .saturating_sub(removed.snapshot.source.len());
                self.usage.current_snapshot_bytes = self.snapshot_bytes;
                inc(&mut self.usage.snapshots_closed, &mut self.usage.saturated);
                SemanticAdmissionOutcomeV1::Accepted
            }
            SemanticProviderAdmissionV1::Unavailable => {
                inc(
                    &mut self.usage.provider_unavailable,
                    &mut self.usage.saturated,
                );
                SemanticAdmissionOutcomeV1::Unavailable
            }
            SemanticProviderAdmissionV1::Refused => {
                self.admission_refusal(SemanticServiceRefusalV1::ProviderRefused)
            }
        }
    }

    pub fn submit(
        &mut self,
        client: SemanticClientRefV1,
        request: SemanticRequestV1,
    ) -> SemanticSubmitOutcomeV1 {
        if let Err(refusal) = self.check_client(client) {
            return self.submit_refusal(refusal);
        }
        let Some(document) = self.documents.get(&request.target.document) else {
            return self.submit_refusal(SemanticServiceRefusalV1::UnknownDocument);
        };
        if document.owner != client {
            return self.submit_refusal(SemanticServiceRefusalV1::ClientIsolation);
        }
        if document.snapshot.target != request.target
            || document.snapshot.language != request.language.as_str()
        {
            return SemanticSubmitOutcomeV1::Stale;
        }
        if !request.is_within_limits(self.limits.peer)
            || !request_offsets_within_snapshot(&request, &document.snapshot.source)
        {
            return self.submit_refusal(SemanticServiceRefusalV1::InputLimit);
        }
        if self.pending.contains_key(&request.request) {
            return self.submit_refusal(SemanticServiceRefusalV1::DuplicateRequest);
        }
        if self.quarantined_requests.contains_key(&request.request) {
            return self.submit_refusal(SemanticServiceRefusalV1::DuplicateRequest);
        }
        if self
            .pending
            .len()
            .saturating_add(self.quarantined_requests.len())
            == self.limits.max_pending
        {
            return self.submit_refusal(SemanticServiceRefusalV1::Capacity);
        }
        self.refresh_provider_state();
        let SemanticProviderReadinessV1::Ready { generation } = self.provider_state.readiness
        else {
            inc(
                &mut self.usage.provider_unavailable,
                &mut self.usage.saturated,
            );
            return SemanticSubmitOutcomeV1::Unavailable;
        };
        if request
            .expected_index
            .is_some_and(|expected| expected != generation)
        {
            return SemanticSubmitOutcomeV1::Stale;
        }
        if !request
            .query
            .required_capabilities()
            .is_subset_of(self.provider_state.capabilities)
        {
            inc(
                &mut self.usage.provider_unsupported,
                &mut self.usage.saturated,
            );
            return SemanticSubmitOutcomeV1::Unsupported(request.query.kind());
        }
        let reserved_deliveries = self
            .pending
            .values()
            .filter(|pending| pending.owner == client)
            .count()
            .saturating_add(self.deliveries.get(&client).map_or(0, VecDeque::len));
        if reserved_deliveries >= self.limits.max_deliveries {
            return self.submit_refusal(SemanticServiceRefusalV1::Capacity);
        }
        let submission = self.provider.submit(&request);
        match submission {
            SemanticProviderSubmissionV1::Accepted => {
                let request_ref = request.request;
                let provider = self.provider_state.provider.clone();
                self.pending.insert(
                    request_ref,
                    PendingRequest {
                        owner: client,
                        request,
                        index_generation: generation,
                        provider,
                    },
                );
                inc(
                    &mut self.usage.requests_submitted,
                    &mut self.usage.saturated,
                );
                self.refresh_provider_state();
                SemanticSubmitOutcomeV1::Accepted
            }
            SemanticProviderSubmissionV1::Unsupported(kind) if kind == request.query.kind() => {
                self.refresh_provider_state();
                inc(
                    &mut self.usage.provider_unsupported,
                    &mut self.usage.saturated,
                );
                SemanticSubmitOutcomeV1::Unsupported(kind)
            }
            SemanticProviderSubmissionV1::Unavailable => {
                self.refresh_provider_state();
                inc(
                    &mut self.usage.provider_unavailable,
                    &mut self.usage.saturated,
                );
                SemanticSubmitOutcomeV1::Unavailable
            }
            SemanticProviderSubmissionV1::Unsupported(_) => {
                self.refresh_provider_state();
                self.submit_refusal(SemanticServiceRefusalV1::MalformedProviderReply)
            }
            SemanticProviderSubmissionV1::Refused => {
                self.refresh_provider_state();
                self.submit_refusal(SemanticServiceRefusalV1::ProviderRefused)
            }
        }
    }

    pub fn cancel(
        &mut self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> Result<bool, SemanticServiceRefusalV1> {
        self.check_client(client)?;
        let Some(pending) = self.pending.get(&request) else {
            return Ok(false);
        };
        if pending.owner != client {
            return Err(SemanticServiceRefusalV1::ClientIsolation);
        }
        if self
            .deliveries
            .get(&client)
            .is_some_and(|queue| queue.len() >= self.limits.max_deliveries)
        {
            inc(&mut self.usage.refusals, &mut self.usage.saturated);
            return Err(SemanticServiceRefusalV1::Capacity);
        }
        self.pending.remove(&request);
        self.quarantine(client, request);
        self.provider.cancel(request);
        self.refresh_provider_state();
        self.enqueue(
            client,
            SemanticDeliveryV1 {
                request,
                outcome: SemanticDeliveryOutcomeV1::Cancelled,
            },
        )?;
        inc(
            &mut self.usage.requests_cancelled,
            &mut self.usage.saturated,
        );
        Ok(true)
    }

    /// Returns whether this exact admitted client owns the pending request.
    /// A request owned by another client is never reported as merely absent.
    pub fn is_pending_for(
        &self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> Result<bool, SemanticServiceRefusalV1> {
        self.check_client(client)?;
        let Some(pending) = self.pending.get(&request) else {
            return Ok(false);
        };
        if pending.owner != client {
            return Err(SemanticServiceRefusalV1::ClientIsolation);
        }
        Ok(true)
    }

    /// Composition-driven deadline evidence. The service owns no clock and
    /// therefore cannot infer a timeout from polling delay or provider state.
    pub fn expire(
        &mut self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> Result<bool, SemanticServiceRefusalV1> {
        self.check_client(client)?;
        let Some(pending) = self.pending.get(&request) else {
            return Ok(false);
        };
        if pending.owner != client {
            return Err(SemanticServiceRefusalV1::ClientIsolation);
        }
        if self
            .deliveries
            .get(&client)
            .is_some_and(|queue| queue.len() >= self.limits.max_deliveries)
        {
            inc(&mut self.usage.refusals, &mut self.usage.saturated);
            return Err(SemanticServiceRefusalV1::Capacity);
        }
        self.pending.remove(&request);
        self.quarantine(client, request);
        self.provider.cancel(request);
        self.refresh_provider_state();
        self.enqueue(
            client,
            SemanticDeliveryV1 {
                request,
                outcome: SemanticDeliveryOutcomeV1::TimedOut,
            },
        )?;
        inc(
            &mut self.usage.requests_timed_out,
            &mut self.usage.saturated,
        );
        Ok(true)
    }

    /// Poll at most the configured finite budget. Provider completions may be
    /// out of order; each is routed only to the client that submitted it.
    pub fn pump(&mut self) {
        self.refresh_provider_state();
        for _ in 0..self.limits.max_provider_polls_per_pump {
            let outcome = self.provider.poll();
            self.refresh_provider_state();
            let Some(outcome) = outcome else {
                break;
            };
            self.route(outcome);
        }
    }

    pub fn take_delivery(
        &mut self,
        client: SemanticClientRefV1,
    ) -> Result<Option<SemanticDeliveryV1>, SemanticServiceRefusalV1> {
        self.check_client(client)?;
        Ok(self
            .deliveries
            .get_mut(&client)
            .and_then(VecDeque::pop_front))
    }

    fn commit_snapshot(
        &mut self,
        client: SemanticClientRefV1,
        snapshot: SemanticSnapshotV1,
        replacing: bool,
    ) -> SemanticAdmissionOutcomeV1 {
        let replaced_bytes = if replacing {
            self.documents
                .get(&snapshot.target.document)
                .map_or(0, |document| document.snapshot.source.len())
        } else {
            0
        };
        let Some(candidate_bytes) = self
            .snapshot_bytes
            .checked_sub(replaced_bytes)
            .and_then(|bytes| bytes.checked_add(snapshot.source.len()))
        else {
            return self.admission_refusal(SemanticServiceRefusalV1::Capacity);
        };
        if candidate_bytes > self.limits.max_total_snapshot_bytes {
            return self.admission_refusal(SemanticServiceRefusalV1::Capacity);
        }
        self.refresh_provider_state();
        let admission = self
            .provider
            .synchronize(&SemanticSnapshotUpdateV1::Admit(snapshot.clone()));
        self.refresh_provider_state();
        match admission {
            SemanticProviderAdmissionV1::Accepted => {
                let document = snapshot.target.document;
                if replacing {
                    self.invalidate_document(document, true);
                }
                self.documents.insert(
                    document,
                    DocumentState {
                        owner: client,
                        snapshot,
                    },
                );
                self.snapshot_bytes = candidate_bytes;
                self.usage.current_snapshot_bytes = candidate_bytes;
                self.usage.high_water_snapshot_bytes =
                    self.usage.high_water_snapshot_bytes.max(candidate_bytes);
                inc(
                    &mut self.usage.snapshots_admitted,
                    &mut self.usage.saturated,
                );
                SemanticAdmissionOutcomeV1::Accepted
            }
            SemanticProviderAdmissionV1::Unavailable => {
                inc(
                    &mut self.usage.provider_unavailable,
                    &mut self.usage.saturated,
                );
                SemanticAdmissionOutcomeV1::Unavailable
            }
            SemanticProviderAdmissionV1::Refused => {
                self.admission_refusal(SemanticServiceRefusalV1::ProviderRefused)
            }
        }
    }

    fn invalidate_document(&mut self, document: SemanticDocumentRefV1, stale: bool) {
        let requests: Vec<_> = self
            .pending
            .iter()
            .filter_map(|(request, pending)| {
                (pending.request.target.document == document).then_some((*request, pending.owner))
            })
            .collect();
        for (request, owner) in requests {
            self.pending.remove(&request);
            self.quarantine(owner, request);
            self.provider.cancel(request);
            let outcome = if stale {
                SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Stale)
            } else {
                SemanticDeliveryOutcomeV1::Cancelled
            };
            let _ = self.enqueue(owner, SemanticDeliveryV1 { request, outcome });
            inc(
                &mut self.usage.requests_invalidated,
                &mut self.usage.saturated,
            );
        }
    }

    fn bridge_invalidate_document(
        &mut self,
        document: SemanticDocumentRefV1,
    ) -> Vec<SemanticRequestRefV1> {
        let requests: Vec<_> = self
            .pending
            .iter()
            .filter_map(|(request, pending)| {
                (pending.request.target.document == document).then_some(*request)
            })
            .collect();
        for request in &requests {
            let pending = self
                .pending
                .remove(request)
                .expect("collected pending request");
            self.quarantine(pending.owner, *request);
            self.provider.cancel(*request);
            inc(
                &mut self.usage.requests_invalidated,
                &mut self.usage.saturated,
            );
        }
        requests
    }

    fn bridge_invalidate_all_pending(&mut self) -> Vec<SemanticRequestRefV1> {
        let requests: Vec<_> = self.pending.keys().copied().collect();
        for request in &requests {
            let pending = self
                .pending
                .remove(request)
                .expect("collected pending request");
            self.quarantine(pending.owner, *request);
            self.provider.cancel(*request);
            inc(
                &mut self.usage.requests_invalidated,
                &mut self.usage.saturated,
            );
        }
        requests
    }

    fn invalidate_all_pending(&mut self) {
        let requests = std::mem::take(&mut self.pending);
        for (request, pending) in requests {
            self.quarantine(pending.owner, request);
            self.provider.cancel(request);
            let _ = self.enqueue(
                pending.owner,
                SemanticDeliveryV1 {
                    request,
                    outcome: SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Stale),
                },
            );
            inc(
                &mut self.usage.requests_invalidated,
                &mut self.usage.saturated,
            );
        }
    }

    fn invalidate_all_pending_with(&mut self, provider: &mut P) {
        let requests: Vec<_> = self
            .pending
            .iter()
            .map(|(request, pending)| (*request, pending.owner))
            .collect();
        for (request, owner) in requests {
            self.pending.remove(&request);
            self.quarantine(owner, request);
            provider.cancel(request);
            let _ = self.enqueue(
                owner,
                SemanticDeliveryV1 {
                    request,
                    outcome: SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Stale),
                },
            );
            inc(
                &mut self.usage.requests_invalidated,
                &mut self.usage.saturated,
            );
        }
    }

    fn replacement_failure<T>(
        &mut self,
        candidate: P,
        refusal: SemanticProviderReplacementRefusalV1,
    ) -> Result<T, SemanticProviderReplacementFailureV1<P>> {
        inc(
            &mut self.usage.provider_replacement_refusals,
            &mut self.usage.saturated,
        );
        Err(SemanticProviderReplacementFailureV1 { candidate, refusal })
    }

    fn route(&mut self, completion: SemanticProviderCompletionV1) {
        let request_ref = completion.request;
        if self.quarantined_requests.contains_key(&request_ref) {
            inc(
                &mut self.usage.orphan_completions,
                &mut self.usage.saturated,
            );
            return;
        }
        let Some(pending) = self.pending.remove(&request_ref) else {
            inc(
                &mut self.usage.orphan_completions,
                &mut self.usage.saturated,
            );
            return;
        };
        let outcome = validate_completion(
            &pending.request,
            pending.index_generation,
            &pending.provider,
            completion.outcome,
            self.limits.peer,
        );
        let outcome =
            validate_partitioned_result(&pending.request, pending.owner, outcome, &self.documents);
        if self
            .enqueue(
                pending.owner,
                SemanticDeliveryV1 {
                    request: request_ref,
                    outcome: SemanticDeliveryOutcomeV1::Peer(outcome),
                },
            )
            .is_err()
        {
            inc(&mut self.usage.refusals, &mut self.usage.saturated);
        }
        inc(
            &mut self.usage.requests_completed,
            &mut self.usage.saturated,
        );
    }

    fn enqueue(
        &mut self,
        client: SemanticClientRefV1,
        delivery: SemanticDeliveryV1,
    ) -> Result<(), SemanticServiceRefusalV1> {
        let queue = self
            .deliveries
            .get_mut(&client)
            .ok_or(SemanticServiceRefusalV1::UnknownClient)?;
        if queue.len() == self.limits.max_deliveries {
            return Err(SemanticServiceRefusalV1::Capacity);
        }
        queue.push_back(delivery);
        Ok(())
    }

    fn check_client(&self, client: SemanticClientRefV1) -> Result<(), SemanticServiceRefusalV1> {
        if self.retiring_clients.contains(&client) {
            Err(SemanticServiceRefusalV1::ClientRetiring)
        } else if self.clients.contains(&client) {
            Ok(())
        } else {
            Err(SemanticServiceRefusalV1::UnknownClient)
        }
    }

    fn quarantine(&mut self, owner: SemanticClientRefV1, request: SemanticRequestRefV1) {
        self.quarantined_requests.insert(request, owner);
        self.usage.current_quarantined_requests = self.quarantined_requests.len();
        self.usage.high_water_quarantined_requests = self
            .usage
            .high_water_quarantined_requests
            .max(self.quarantined_requests.len());
    }

    fn admission_refusal(
        &mut self,
        refusal: SemanticServiceRefusalV1,
    ) -> SemanticAdmissionOutcomeV1 {
        inc(&mut self.usage.refusals, &mut self.usage.saturated);
        SemanticAdmissionOutcomeV1::Refused(refusal)
    }

    fn submit_refusal(&mut self, refusal: SemanticServiceRefusalV1) -> SemanticSubmitOutcomeV1 {
        inc(&mut self.usage.refusals, &mut self.usage.saturated);
        SemanticSubmitOutcomeV1::Refused(refusal)
    }
}

fn validate_completion(
    request: &SemanticRequestV1,
    expected_generation: SemanticIndexGenerationV1,
    expected_provider: &SemanticProviderProvenanceV1,
    outcome: SemanticOutcomeV1,
    limits: SemanticLimitsV1,
) -> SemanticOutcomeV1 {
    match outcome {
        SemanticOutcomeV1::Complete(reply)
            if reply.request == request.request
                && reply.target == request.target
                && reply.index_generation == expected_generation
                && &reply.provider == expected_provider
                && reply.result.matches_query(request.query.kind())
                && reply.result.is_within_limits(limits) =>
        {
            SemanticOutcomeV1::Complete(reply)
        }
        SemanticOutcomeV1::Complete(_) => {
            SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply)
        }
        SemanticOutcomeV1::Unsupported(kind) if kind != request.query.kind() => {
            SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply)
        }
        other => other,
    }
}

fn semantic_invalidation_receipt(
    cause: SemanticInvalidationCauseV1,
    requests: Vec<SemanticRequestRefV1>,
    max_pending: usize,
) -> SemanticInvalidationReceiptV1 {
    SemanticInvalidationReceiptV1::try_new(
        cause,
        requests,
        u64::try_from(max_pending).expect("semantic pending limit fits u64"),
    )
    .expect("service-owned request set is canonical and bounded")
}

fn validate_partitioned_result(
    request: &SemanticRequestV1,
    owner: SemanticClientRefV1,
    outcome: SemanticOutcomeV1,
    documents: &BTreeMap<SemanticDocumentRefV1, DocumentState>,
) -> SemanticOutcomeV1 {
    let SemanticOutcomeV1::Complete(reply) = &outcome else {
        return outcome;
    };
    let valid_location = |location: &vellum_host::semantic::SemanticLocationV1| {
        documents
            .get(&location.target.document)
            .is_some_and(|document| {
                let Ok(start) = usize::try_from(location.span.start) else {
                    return false;
                };
                let Ok(end) = usize::try_from(location.span.end) else {
                    return false;
                };
                document.owner == owner
                    && document.snapshot.target == location.target
                    && start <= end
                    && end <= document.snapshot.source.len()
                    && document.snapshot.source.is_char_boundary(start)
                    && document.snapshot.source.is_char_boundary(end)
            })
    };
    let valid = if let Some(symbols) = reply.result.symbols() {
        symbols
            .iter()
            .all(|symbol| valid_location(&symbol.location))
    } else if let Some(locations) = reply.result.locations() {
        locations.iter().all(valid_location)
    } else if let Some(diagnostics) = reply.result.diagnostics() {
        diagnostics.iter().all(|diagnostic| {
            diagnostic.location.target == request.target && valid_location(&diagnostic.location)
        })
    } else if let Some(edits) = reply.result.rename_plan() {
        let mut ranges = Vec::with_capacity(edits.len());
        for edit in edits {
            if !valid_location(&edit.location) {
                return SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply);
            }
            ranges.push((
                edit.location.target.document,
                edit.location.span.start,
                edit.location.span.end,
            ));
        }
        ranges.sort_unstable();
        ranges.windows(2).all(|pair| {
            let [
                (left_document, left_start, left_end),
                (right_document, right_start, _),
            ] = pair
            else {
                return true;
            };
            left_document != right_document
                || (*right_start >= *left_end && *right_start != *left_start)
        })
    } else {
        reply.result.type_at_point().is_some()
    };
    if valid {
        outcome
    } else {
        SemanticOutcomeV1::Refused(SemanticRefusalV1::MalformedProviderReply)
    }
}

fn request_offsets_within_snapshot(request: &SemanticRequestV1, source: &str) -> bool {
    let offset = match &request.query {
        vellum_host::semantic::SemanticQueryV1::Definition { offset }
        | vellum_host::semantic::SemanticQueryV1::References { offset }
        | vellum_host::semantic::SemanticQueryV1::Implementations { offset }
        | vellum_host::semantic::SemanticQueryV1::TypeAtPoint { offset }
        | vellum_host::semantic::SemanticQueryV1::PlanRename { offset, .. } => Some(*offset),
        vellum_host::semantic::SemanticQueryV1::Symbols { .. }
        | vellum_host::semantic::SemanticQueryV1::Diagnostics => None,
    };
    offset.is_none_or(|offset| {
        usize::try_from(offset)
            .ok()
            .is_some_and(|offset| offset <= source.len() && source.is_char_boundary(offset))
    })
}

fn inc(value: &mut u64, saturated: &mut bool) {
    let before = *value;
    *value = value.saturating_add(1);
    *saturated |= before == u64::MAX;
}

fn add_usize_to_counter(value: &mut u64, amount: usize, saturated: &mut bool) {
    let Ok(amount) = u64::try_from(amount) else {
        *value = u64::MAX;
        *saturated = true;
        return;
    };
    if let Some(next) = value.checked_add(amount) {
        *value = next;
    } else {
        *value = u64::MAX;
        *saturated = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vellum_host::semantic::{
        SemanticCapabilityV1, SemanticLocationV1, SemanticProviderRefV1, SemanticQueryV1,
        SemanticReplyV1, SemanticResultV1, SemanticSourceRevisionV1, SemanticSpanV1,
        SemanticTextEditV1, SemanticTextV1,
    };

    struct ModelProvider {
        synchronized: Vec<SemanticSnapshotUpdateV1>,
        submitted: Vec<SemanticRequestV1>,
        cancelled: Vec<SemanticRequestRefV1>,
        completions: VecDeque<SemanticProviderCompletionV1>,
        state: SemanticProviderStateV1,
        state_after_poll: Option<SemanticProviderStateV1>,
        state_after_synchronize: Option<SemanticProviderStateV1>,
        state_after_submit: Option<SemanticProviderStateV1>,
        state_after_cancel: Option<SemanticProviderStateV1>,
        bridge_poll_failure: Option<SemanticBridgePollFailureV1>,
        available: bool,
        refuse_synchronization: bool,
        close_refusals_remaining: usize,
        panic_on_cancel: bool,
        panic_on_close: bool,
    }

    impl Default for ModelProvider {
        fn default() -> Self {
            Self {
                synchronized: Vec::new(),
                submitted: Vec::new(),
                cancelled: Vec::new(),
                completions: VecDeque::new(),
                state: SemanticProviderStateV1::unavailable(provenance()),
                state_after_poll: None,
                state_after_synchronize: None,
                state_after_submit: None,
                state_after_cancel: None,
                bridge_poll_failure: None,
                available: false,
                refuse_synchronization: false,
                close_refusals_remaining: 0,
                panic_on_cancel: false,
                panic_on_close: false,
            }
        }
    }

    impl SemanticProviderPortV1 for ModelProvider {
        fn state(&self) -> SemanticProviderStateV1 {
            self.state.clone()
        }

        fn synchronize(
            &mut self,
            update: &SemanticSnapshotUpdateV1,
        ) -> SemanticProviderAdmissionV1 {
            if !self.available {
                return SemanticProviderAdmissionV1::Unavailable;
            }
            self.synchronized.push(update.clone());
            if self.panic_on_close && matches!(update, SemanticSnapshotUpdateV1::Close { .. }) {
                panic!("injected provider close panic");
            }
            if let Some(state) = self.state_after_synchronize.take() {
                self.state = state;
            }
            if matches!(update, SemanticSnapshotUpdateV1::Close { .. })
                && self.close_refusals_remaining > 0
            {
                self.close_refusals_remaining -= 1;
                return SemanticProviderAdmissionV1::Refused;
            }
            if self.refuse_synchronization {
                return SemanticProviderAdmissionV1::Refused;
            }
            SemanticProviderAdmissionV1::Accepted
        }

        fn submit(&mut self, request: &SemanticRequestV1) -> SemanticProviderSubmissionV1 {
            if !self.available {
                return SemanticProviderSubmissionV1::Unavailable;
            }
            self.submitted.push(request.clone());
            if let Some(state) = self.state_after_submit.take() {
                self.state = state;
            }
            SemanticProviderSubmissionV1::Accepted
        }

        fn cancel(&mut self, request: SemanticRequestRefV1) {
            self.cancelled.push(request);
            if self.panic_on_cancel {
                panic!("injected provider cancel panic");
            }
            if let Some(state) = self.state_after_cancel.take() {
                self.state = state;
            }
        }

        fn poll(&mut self) -> Option<SemanticProviderCompletionV1> {
            if let Some(state) = self.state_after_poll.take() {
                self.state = state;
            }
            self.completions.pop_front()
        }
    }

    impl SemanticBridgeProviderPollV1 for ModelProvider {
        fn poll_bridge(
            &mut self,
        ) -> Result<Option<SemanticProviderCompletionV1>, SemanticBridgePollFailureV1> {
            if let Some(failure) = self.bridge_poll_failure.take() {
                return Err(failure);
            }
            Ok(self.poll())
        }
    }

    fn client(tag: u8) -> SemanticClientRefV1 {
        SemanticClientRefV1::new([tag; 16])
    }

    fn target(tag: u8, revision: u64) -> SemanticTargetV1 {
        SemanticTargetV1 {
            document: SemanticDocumentRefV1::new([tag; 16]),
            source_revision: SemanticSourceRevisionV1::new(revision),
        }
    }

    fn snapshot(tag: u8, revision: u64) -> SemanticSnapshotV1 {
        SemanticSnapshotV1::try_new(
            target(tag, revision),
            "rust".into(),
            "fn bounded() {}".into(),
            SemanticServiceLimitsV1::reference(),
        )
        .unwrap()
    }

    fn request(tag: u8, revision: u64, request_tag: u8) -> SemanticRequestV1 {
        SemanticRequestV1 {
            request: SemanticRequestRefV1::new([request_tag; 16]),
            target: target(tag, revision),
            expected_index: SemanticIndexGenerationV1::new(1),
            language: SemanticTextV1::try_new("rust".into(), 16).unwrap(),
            query: SemanticQueryV1::Diagnostics,
        }
    }

    fn provenance() -> SemanticProviderProvenanceV1 {
        SemanticProviderProvenanceV1::try_new(
            SemanticProviderRefV1::new([19; 16]),
            "model-provider".into(),
            "1.0.0".into(),
        )
        .unwrap()
    }

    fn alternate_provenance() -> SemanticProviderProvenanceV1 {
        SemanticProviderProvenanceV1::try_new(
            SemanticProviderRefV1::new([20; 16]),
            "alternate-provider".into(),
            "2.0.0".into(),
        )
        .unwrap()
    }

    fn ready_state(generation: u64) -> SemanticProviderStateV1 {
        SemanticProviderStateV1 {
            provider: provenance(),
            readiness: SemanticProviderReadinessV1::Ready {
                generation: SemanticIndexGenerationV1::new(generation).unwrap(),
            },
            capabilities: SemanticCapabilitySetV1::read_only(),
        }
    }

    fn alternate_ready_state(generation: u64) -> SemanticProviderStateV1 {
        SemanticProviderStateV1 {
            provider: alternate_provenance(),
            readiness: SemanticProviderReadinessV1::Ready {
                generation: SemanticIndexGenerationV1::new(generation).unwrap(),
            },
            capabilities: SemanticCapabilitySetV1::read_only(),
        }
    }

    fn service() -> SemanticServiceV1<ModelProvider> {
        SemanticServiceV1::try_new(
            ModelProvider {
                state: ready_state(1),
                available: true,
                ..ModelProvider::default()
            },
            SemanticServiceLimitsV1::reference(),
        )
        .unwrap()
    }

    fn bridge_limits() -> SemanticServiceLimitsV1 {
        SemanticServiceLimitsV1 {
            max_clients: 1,
            max_pending: 2,
            max_deliveries: 2,
            max_provider_polls_per_pump: 1,
            ..SemanticServiceLimitsV1::reference()
        }
    }

    fn bridge_service() -> SemanticBridgeServiceV1<ModelProvider> {
        SemanticBridgeServiceV1::try_new(
            ModelProvider {
                state: ready_state(1),
                available: true,
                ..ModelProvider::default()
            },
            bridge_limits(),
        )
        .unwrap()
    }

    #[test]
    fn bridge_requires_single_client_single_poll_empty_construction() {
        for limits in [
            SemanticServiceLimitsV1 {
                max_clients: 2,
                ..bridge_limits()
            },
            SemanticServiceLimitsV1 {
                max_provider_polls_per_pump: 2,
                ..bridge_limits()
            },
        ] {
            assert!(matches!(
                SemanticBridgeServiceV1::try_new(ModelProvider::default(), limits),
                Err(SemanticServiceRefusalV1::InvalidLimits)
            ));
        }
    }

    #[test]
    fn bridge_change_returns_exact_max_bound_without_synthetic_delivery() {
        let mut bridge = bridge_service();
        bridge.admit_client(client(1)).unwrap();
        assert_eq!(
            bridge.open_snapshot(client(1), snapshot(3, 1)).outcome(),
            &SemanticBridgeOpenAdmissionV1::Accepted
        );
        let first = request(3, 1, 7);
        let second = request(3, 1, 8);
        assert_eq!(
            bridge.submit(client(1), first.clone()).outcome(),
            &SemanticSubmitOutcomeV1::Accepted
        );
        assert_eq!(
            bridge.submit(client(1), second.clone()).outcome(),
            &SemanticSubmitOutcomeV1::Accepted
        );
        let report = bridge.change_snapshot(client(1), snapshot(3, 2));
        let SemanticBridgeChangeAdmissionV1::Accepted {
            source_invalidations,
        } = report.outcome()
        else {
            panic!()
        };
        assert_eq!(
            source_invalidations.cause(),
            SemanticInvalidationCauseV1::SourceChanged
        );
        assert_eq!(
            source_invalidations.iter().copied().collect::<Vec<_>>(),
            vec![first.request, second.request]
        );
        assert!(report.pre_provider().is_none());
        assert!(report.post_provider().is_none());
        assert_eq!(bridge.inner.take_delivery(client(1)).unwrap(), None);
        assert_eq!(
            bridge.inner.provider.cancelled,
            vec![first.request, second.request]
        );
    }

    #[test]
    fn bridge_close_returns_exact_closed_receipt_without_delivery() {
        let mut bridge = bridge_service();
        bridge.admit_client(client(1)).unwrap();
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));
        let offered = request(3, 1, 7);
        let _ = bridge.submit(client(1), offered.clone());
        let report = bridge.close_snapshot(client(1), target(3, 1));
        let SemanticBridgeCloseAdmissionV1::Accepted {
            source_invalidations,
        } = report.outcome()
        else {
            panic!()
        };
        assert_eq!(
            source_invalidations.cause(),
            SemanticInvalidationCauseV1::SourceClosed
        );
        assert_eq!(
            source_invalidations.iter().copied().collect::<Vec<_>>(),
            vec![offered.request]
        );
        assert_eq!(bridge.inner.take_delivery(client(1)).unwrap(), None);
    }

    #[test]
    fn bridge_orders_disjoint_source_then_post_provider_invalidations() {
        let mut bridge = bridge_service();
        bridge.admit_client(client(1)).unwrap();
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));
        let _ = bridge.open_snapshot(client(1), snapshot(4, 1));
        let changed = request(3, 1, 7);
        let other = request(4, 1, 8);
        let _ = bridge.submit(client(1), changed.clone());
        let _ = bridge.submit(client(1), other.clone());
        bridge.inner.provider.state_after_synchronize = Some(alternate_ready_state(2));
        let report = bridge.change_snapshot(client(1), snapshot(3, 2));
        let SemanticBridgeChangeAdmissionV1::Accepted {
            source_invalidations,
        } = report.outcome()
        else {
            panic!()
        };
        assert_eq!(
            source_invalidations.iter().copied().collect::<Vec<_>>(),
            vec![changed.request]
        );
        let post = report.post_provider().unwrap();
        assert_eq!(
            post.invalidations().cause(),
            SemanticInvalidationCauseV1::ProviderStateChanged
        );
        assert_eq!(
            post.invalidations().iter().copied().collect::<Vec<_>>(),
            vec![other.request]
        );
        assert_eq!(report.provider_after(), &alternate_ready_state(2));
        assert_eq!(bridge.inner.take_delivery(client(1)).unwrap(), None);
    }

    #[test]
    fn bridge_preserves_aba_pre_and_post_provider_stages_on_refusal() {
        let mut bridge = bridge_service();
        bridge.admit_client(client(1)).unwrap();
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));
        let offered = request(3, 1, 7);
        let _ = bridge.submit(client(1), offered.clone());
        bridge.inner.provider.state = alternate_ready_state(2);
        bridge.inner.provider.state_after_synchronize = Some(ready_state(1));
        bridge.inner.provider.refuse_synchronization = true;
        let report = bridge.change_snapshot(client(1), snapshot(3, 2));
        assert_eq!(
            report.outcome(),
            &SemanticBridgeChangeAdmissionV1::Refused(SemanticServiceRefusalV1::ProviderRefused)
        );
        assert_eq!(
            report
                .pre_provider()
                .unwrap()
                .invalidations()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![offered.request]
        );
        assert_eq!(report.post_provider().unwrap().invalidations().len(), 0);
        assert_eq!(report.provider_before(), &ready_state(1));
        assert_eq!(report.provider_after(), &ready_state(1));
    }

    #[test]
    fn bridge_stable_unavailable_is_receipt_free_and_repeated_refresh_is_inert() {
        let mut bridge = bridge_service();
        bridge.admit_client(client(1)).unwrap();
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));
        let offered = request(3, 1, 7);
        let _ = bridge.submit(client(1), offered.clone());
        bridge.inner.provider.available = false;
        let unavailable = bridge.change_snapshot(client(1), snapshot(3, 2));
        assert_eq!(
            unavailable.outcome(),
            &SemanticBridgeChangeAdmissionV1::Unavailable
        );
        assert!(unavailable.pre_provider().is_none());
        assert!(unavailable.post_provider().is_none());
        assert!(
            bridge
                .inner
                .is_pending_for(client(1), offered.request)
                .unwrap()
        );

        bridge.inner.provider.state = alternate_ready_state(2);
        let first = bridge.refresh_provider_state();
        assert_eq!(
            first
                .transition()
                .unwrap()
                .invalidations()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![offered.request]
        );
        assert!(bridge.refresh_provider_state().transition().is_none());
        assert_eq!(bridge.inner.take_delivery(client(1)).unwrap(), None);
        assert!(!format!("{first:?}").contains("alternate-provider"));
    }

    #[test]
    fn bridge_submit_post_transition_invalidates_prior_and_newly_accepted_requests() {
        let mut bridge = bridge_service();
        bridge.admit_client(client(1)).unwrap();
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));
        let first = request(3, 1, 7);
        let second = request(3, 1, 8);
        assert_eq!(
            bridge.submit(client(1), first.clone()).outcome(),
            &SemanticSubmitOutcomeV1::Accepted
        );
        bridge.inner.provider.state_after_submit = Some(alternate_ready_state(2));
        let report = bridge.submit(client(1), second.clone());
        assert_eq!(report.outcome(), &SemanticSubmitOutcomeV1::Accepted);
        assert!(report.pre_provider().is_none());
        assert_eq!(
            report
                .post_provider()
                .unwrap()
                .invalidations()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![first.request, second.request]
        );
        assert_eq!(bridge.inner.take_delivery(client(1)).unwrap(), None);
    }

    #[test]
    fn bridge_submit_reserves_delivery_obligation_before_provider_effect() {
        let limits = SemanticServiceLimitsV1 {
            max_clients: 1,
            max_pending: 2,
            max_deliveries: 1,
            max_provider_polls_per_pump: 1,
            ..SemanticServiceLimitsV1::reference()
        };
        let provider = ModelProvider {
            state: ready_state(1),
            available: true,
            ..ModelProvider::default()
        };
        let mut bridge = SemanticBridgeServiceV1::try_new(provider, limits).unwrap();
        bridge.admit_client(client(1)).unwrap();
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));
        assert_eq!(
            bridge.submit(client(1), request(3, 1, 7)).outcome(),
            &SemanticSubmitOutcomeV1::Accepted
        );
        assert_eq!(
            bridge.submit(client(1), request(3, 1, 8)).outcome(),
            &SemanticSubmitOutcomeV1::Refused(SemanticServiceRefusalV1::Capacity)
        );
        assert_eq!(bridge.inner.provider.submitted.len(), 1);
    }

    #[test]
    fn bridge_cancel_and_expiry_terminalize_without_synthetic_delivery() {
        let mut cancelled = bridge_service();
        cancelled.admit_client(client(1)).unwrap();
        let _ = cancelled.open_snapshot(client(1), snapshot(3, 1));
        let cancel_request = request(3, 1, 7);
        let _ = cancelled.submit(client(1), cancel_request.clone());
        let report = cancelled.cancel(client(1), cancel_request.request);
        assert_eq!(report.outcome(), &SemanticBridgeCancelActionV1::Cancelled);
        assert_eq!(
            cancelled.inner.provider.cancelled,
            vec![cancel_request.request]
        );
        assert_eq!(cancelled.inner.take_delivery(client(1)).unwrap(), None);

        let mut expired = bridge_service();
        expired.admit_client(client(1)).unwrap();
        let _ = expired.open_snapshot(client(1), snapshot(3, 1));
        let expire_request = request(3, 1, 8);
        let _ = expired.submit(client(1), expire_request.clone());
        assert_eq!(
            expired.expire(client(1), expire_request.request).outcome(),
            &SemanticBridgeExpiryActionV1::Expired
        );
        assert_eq!(
            expired.inner.provider.cancelled,
            vec![expire_request.request]
        );
        assert_eq!(expired.inner.take_delivery(client(1)).unwrap(), None);
    }

    #[test]
    fn bridge_quarantine_does_not_consume_live_pending_capacity() {
        let limits = SemanticServiceLimitsV1 {
            max_clients: 1,
            max_pending: 1,
            max_deliveries: 1,
            max_provider_polls_per_pump: 1,
            ..SemanticServiceLimitsV1::reference()
        };
        let provider = ModelProvider {
            state: ready_state(1),
            available: true,
            ..ModelProvider::default()
        };
        let mut bridge = SemanticBridgeServiceV1::try_new(provider, limits).unwrap();
        bridge.admit_client(client(1)).unwrap();
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));

        for id in [7, 8, 9] {
            let offered = request(3, 1, id);
            assert_eq!(
                bridge.submit(client(1), offered.clone()).outcome(),
                &SemanticSubmitOutcomeV1::Accepted
            );
            assert_eq!(
                bridge.cancel(client(1), offered.request).outcome(),
                &SemanticBridgeCancelActionV1::Cancelled
            );
        }
        assert_eq!(bridge.usage().current_quarantined_requests, 3);
        assert_eq!(bridge.inner.pending.len(), 0);
        assert_eq!(bridge.inner.provider.submitted.len(), 3);
    }

    #[test]
    fn bridge_provider_invalidation_releases_live_capacity_but_not_identity() {
        let limits = SemanticServiceLimitsV1 {
            max_clients: 1,
            max_pending: 1,
            max_deliveries: 1,
            max_provider_polls_per_pump: 1,
            ..SemanticServiceLimitsV1::reference()
        };
        let provider = ModelProvider {
            state: ready_state(1),
            available: true,
            ..ModelProvider::default()
        };
        let mut bridge = SemanticBridgeServiceV1::try_new(provider, limits).unwrap();
        bridge.admit_client(client(1)).unwrap();
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));
        let first = request(3, 1, 7);
        assert_eq!(
            bridge.submit(client(1), first.clone()).outcome(),
            &SemanticSubmitOutcomeV1::Accepted
        );
        bridge.inner.provider.state = alternate_ready_state(2);
        let transition = bridge.refresh_provider_state();
        assert_eq!(
            transition
                .transition()
                .unwrap()
                .invalidations()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![first.request]
        );

        let mut second = request(3, 1, 8);
        second.expected_index = SemanticIndexGenerationV1::new(2);
        assert_eq!(
            bridge.submit(client(1), second).outcome(),
            &SemanticSubmitOutcomeV1::Accepted
        );
        assert_eq!(
            bridge.submit(client(1), first).outcome(),
            &SemanticSubmitOutcomeV1::Refused(SemanticServiceRefusalV1::DuplicateRequest)
        );
    }

    #[test]
    fn bridge_duplicate_late_completion_remains_exact_until_provider_retirement() {
        let mut bridge = bridge_service();
        bridge.admit_client(client(1)).unwrap();
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));
        let offered = request(3, 1, 7);
        let _ = bridge.submit(client(1), offered.clone());
        let _ = bridge.cancel(client(1), offered.request);
        for _ in 0..2 {
            bridge
                .inner
                .provider
                .completions
                .push_back(SemanticProviderCompletionV1 {
                    request: offered.request,
                    outcome: SemanticOutcomeV1::Stale,
                });
            assert_eq!(
                bridge.poll_once().outcome(),
                &SemanticBridgePollOutcomeV1::Completion(SemanticBridgePollDispositionV1::Late {
                    request: offered.request
                })
            );
        }
        assert_eq!(bridge.usage().current_quarantined_requests, 1);
        assert_eq!(bridge.usage().late_completions, 2);
    }

    #[test]
    fn bridge_delivered_identity_is_terminal_and_duplicate_completion_is_late() {
        let mut bridge = bridge_service();
        bridge.admit_client(client(1)).unwrap();
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));
        let offered = request(3, 1, 7);
        let _ = bridge.submit(client(1), offered.clone());
        for _ in 0..2 {
            bridge
                .inner
                .provider
                .completions
                .push_back(SemanticProviderCompletionV1 {
                    request: offered.request,
                    outcome: SemanticOutcomeV1::Stale,
                });
        }
        assert!(matches!(
            bridge.poll_once().outcome(),
            SemanticBridgePollOutcomeV1::Completion(SemanticBridgePollDispositionV1::Delivery(_))
        ));
        assert_eq!(
            bridge.poll_once().outcome(),
            &SemanticBridgePollOutcomeV1::Completion(SemanticBridgePollDispositionV1::Late {
                request: offered.request
            })
        );
        assert_eq!(
            bridge.submit(client(1), offered).outcome(),
            &SemanticSubmitOutcomeV1::Refused(SemanticServiceRefusalV1::DuplicateRequest)
        );
        assert_eq!(bridge.usage().current_quarantined_requests, 1);
    }

    #[test]
    fn bridge_checked_extraction_retains_service_until_cleanup_is_closed() {
        let mut bridge = bridge_service();
        bridge.admit_client(client(1)).unwrap();
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));
        let offered = request(3, 1, 7);
        let _ = bridge.submit(client(1), offered.clone());

        let failure = bridge.try_into_retiring_provider().unwrap_err();
        assert_eq!(
            failure.blockers(),
            SemanticBridgeProviderExtractionBlockersV1 {
                clients: 1,
                retiring_clients: 0,
                documents: 1,
                pending_requests: 1,
                queued_deliveries: 0,
            }
        );
        let mut bridge = failure.into_service();
        let _ = bridge.cancel(client(1), offered.request);
        let report = bridge.retire_client(client(1));
        assert_eq!(
            report.state,
            SemanticClientRetirementStateV1::Retired {
                quarantined_requests: 1
            }
        );

        let retiring = bridge.try_into_retiring_provider().unwrap();
        assert_eq!(retiring.quarantined_requests(), 1);
        let debug = format!("{retiring:?}");
        assert!(debug.contains("quarantined_requests: 1"));
        assert!(!debug.contains("model-provider"));
        let provider = retiring.into_provider_for_retirement();
        assert_eq!(provider.cancelled, vec![offered.request]);
    }

    #[test]
    fn bridge_extraction_refuses_a_partially_retired_client() {
        let provider = ModelProvider {
            state: ready_state(1),
            available: true,
            close_refusals_remaining: 1,
            ..ModelProvider::default()
        };
        let mut bridge = SemanticBridgeServiceV1::try_new(provider, bridge_limits()).unwrap();
        bridge.admit_client(client(1)).unwrap();
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));
        assert!(matches!(
            bridge.retire_client(client(1)).state,
            SemanticClientRetirementStateV1::Pending {
                remaining_documents: 1,
                ..
            }
        ));
        let failure = bridge.try_into_retiring_provider().unwrap_err();
        assert_eq!(failure.blockers().clients, 1);
        assert_eq!(failure.blockers().retiring_clients, 1);
        assert_eq!(failure.blockers().documents, 1);
        let mut bridge = failure.into_service();
        assert!(matches!(
            bridge.retire_client(client(1)).state,
            SemanticClientRetirementStateV1::Retired { .. }
        ));
        assert_eq!(
            bridge
                .try_into_retiring_provider()
                .unwrap()
                .quarantined_requests(),
            0
        );
    }

    #[test]
    fn bridge_forced_extraction_counts_live_state_and_attempts_each_effect_once() {
        let provider = ModelProvider {
            state: ready_state(1),
            available: true,
            close_refusals_remaining: 1,
            ..ModelProvider::default()
        };
        let mut bridge = SemanticBridgeServiceV1::try_new(provider, bridge_limits()).unwrap();
        bridge.admit_client(client(1)).unwrap();
        assert_eq!(
            bridge.open_snapshot(client(1), snapshot(3, 1)).outcome(),
            &SemanticBridgeOpenAdmissionV1::Accepted
        );
        assert_eq!(
            bridge.open_snapshot(client(1), snapshot(4, 1)).outcome(),
            &SemanticBridgeOpenAdmissionV1::Accepted
        );
        let quarantined = request(3, 1, 7);
        let pending = request(4, 1, 8);
        assert_eq!(
            bridge.submit(client(1), quarantined.clone()).outcome(),
            &SemanticSubmitOutcomeV1::Accepted
        );
        assert_eq!(
            bridge.cancel(client(1), quarantined.request).outcome(),
            &SemanticBridgeCancelActionV1::Cancelled
        );
        assert_eq!(
            bridge.submit(client(1), pending.clone()).outcome(),
            &SemanticSubmitOutcomeV1::Accepted
        );
        bridge
            .inner
            .deliveries
            .get_mut(&client(1))
            .unwrap()
            .push_back(SemanticDeliveryV1 {
                request: quarantined.request,
                outcome: SemanticDeliveryOutcomeV1::Cancelled,
            });

        let forced =
            bridge.force_into_retiring_provider(SemanticBridgeForcedEffectPolicyV1::AttemptBounded);
        assert!(forced.is_forced_only());
        assert_eq!(
            forced.abandonment(),
            SemanticBridgeForcedAbandonmentReceiptV1 {
                clients_abandoned: 1,
                retiring_clients_abandoned: 0,
                documents_abandoned: 2,
                pending_requests_abandoned: 1,
                queued_deliveries_abandoned: 1,
                quarantined_requests: 1,
                cancellation_signals_attempted: 1,
                cancellation_signals_skipped: 0,
                close_attempts: 2,
                closes_accepted: 1,
                closes_unavailable: 0,
                closes_refused: 1,
                close_outcomes_unknown: 0,
                closes_skipped: 0,
                provider_panicked: false,
            }
        );
        assert!(forced.abandonment().is_closed());
        let debug = format!("{forced:?}");
        assert!(debug.contains("<forced-only>"));
        assert!(!debug.contains("model-provider"));
        let parts = forced.into_forced_retirement_parts();
        assert!(parts.is_forced_only());
        let (provider, receipt) = parts.into_provider_and_abandonment();
        assert_eq!(receipt.close_attempts, 2);
        assert_eq!(
            provider.cancelled,
            vec![quarantined.request, pending.request]
        );
        assert_eq!(
            provider
                .synchronized
                .iter()
                .filter(|update| matches!(update, SemanticSnapshotUpdateV1::Close { .. }))
                .count(),
            2
        );
    }

    #[test]
    fn bridge_forced_extraction_preserves_unavailable_and_retiring_evidence() {
        let provider = ModelProvider {
            state: ready_state(1),
            available: true,
            refuse_synchronization: true,
            ..ModelProvider::default()
        };
        let mut bridge = SemanticBridgeServiceV1::try_new(provider, bridge_limits()).unwrap();
        bridge.admit_client(client(1)).unwrap();
        // Temporarily allow admission, then make both ordinary and forced close
        // attempts refuse without erasing the live source.
        bridge.inner.provider.refuse_synchronization = false;
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));
        let offered = request(3, 1, 7);
        let _ = bridge.submit(client(1), offered.clone());
        bridge.inner.provider.refuse_synchronization = true;
        assert!(matches!(
            bridge.retire_client(client(1)).state,
            SemanticClientRetirementStateV1::Pending { .. }
        ));
        let forced =
            bridge.force_into_retiring_provider(SemanticBridgeForcedEffectPolicyV1::AttemptBounded);
        let receipt = forced.abandonment();
        assert_eq!(receipt.clients_abandoned, 1);
        assert_eq!(receipt.retiring_clients_abandoned, 1);
        assert_eq!(receipt.documents_abandoned, 1);
        assert_eq!(receipt.pending_requests_abandoned, 0);
        assert_eq!(receipt.quarantined_requests, 1);
        assert_eq!(receipt.cancellation_signals_attempted, 0);
        assert_eq!(receipt.closes_refused, 1);
        let (provider, _) = forced
            .into_forced_retirement_parts()
            .into_provider_and_abandonment();
        assert_eq!(provider.cancelled, vec![offered.request]);

        let provider = ModelProvider {
            state: ready_state(1),
            available: true,
            ..ModelProvider::default()
        };
        let mut unavailable = SemanticBridgeServiceV1::try_new(provider, bridge_limits()).unwrap();
        unavailable.admit_client(client(1)).unwrap();
        let _ = unavailable.open_snapshot(client(1), snapshot(3, 1));
        unavailable.inner.provider.available = false;
        let forced = unavailable
            .force_into_retiring_provider(SemanticBridgeForcedEffectPolicyV1::AttemptBounded);
        assert_eq!(forced.abandonment().closes_unavailable, 1);
        assert_eq!(forced.abandonment().closes_accepted, 0);
    }

    #[test]
    fn bridge_forced_extraction_can_skip_a_provider_that_is_unsafe_to_call() {
        let mut bridge = bridge_service();
        bridge.admit_client(client(1)).unwrap();
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));
        let offered = request(3, 1, 7);
        let _ = bridge.submit(client(1), offered);

        let forced = bridge
            .force_into_retiring_provider(SemanticBridgeForcedEffectPolicyV1::SkipUnsafeProvider);
        assert_eq!(
            forced.abandonment(),
            SemanticBridgeForcedAbandonmentReceiptV1 {
                clients_abandoned: 1,
                retiring_clients_abandoned: 0,
                documents_abandoned: 1,
                pending_requests_abandoned: 1,
                queued_deliveries_abandoned: 0,
                quarantined_requests: 0,
                cancellation_signals_attempted: 0,
                cancellation_signals_skipped: 1,
                close_attempts: 0,
                closes_accepted: 0,
                closes_unavailable: 0,
                closes_refused: 0,
                close_outcomes_unknown: 0,
                closes_skipped: 1,
                provider_panicked: false,
            }
        );
        let (provider, _) = forced
            .into_forced_retirement_parts()
            .into_provider_and_abandonment();
        assert!(provider.cancelled.is_empty());
        assert_eq!(
            provider
                .synchronized
                .iter()
                .filter(|update| matches!(update, SemanticSnapshotUpdateV1::Close { .. }))
                .count(),
            0
        );
    }

    #[test]
    fn bridge_forced_extraction_closes_over_a_cancel_panic_without_later_effects() {
        let provider = ModelProvider {
            state: ready_state(1),
            available: true,
            ..ModelProvider::default()
        };
        let mut bridge = SemanticBridgeServiceV1::try_new(provider, bridge_limits()).unwrap();
        bridge.admit_client(client(1)).unwrap();
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));
        let _ = bridge.submit(client(1), request(3, 1, 7));
        let _ = bridge.submit(client(1), request(3, 1, 8));
        bridge.inner.provider.panic_on_cancel = true;

        let forced =
            bridge.force_into_retiring_provider(SemanticBridgeForcedEffectPolicyV1::AttemptBounded);
        assert!(forced.is_forced_only());
        assert_eq!(
            forced.abandonment(),
            SemanticBridgeForcedAbandonmentReceiptV1 {
                clients_abandoned: 1,
                retiring_clients_abandoned: 0,
                documents_abandoned: 1,
                pending_requests_abandoned: 2,
                queued_deliveries_abandoned: 0,
                quarantined_requests: 0,
                cancellation_signals_attempted: 1,
                cancellation_signals_skipped: 1,
                close_attempts: 0,
                closes_accepted: 0,
                closes_unavailable: 0,
                closes_refused: 0,
                close_outcomes_unknown: 0,
                closes_skipped: 1,
                provider_panicked: true,
            }
        );
        assert!(forced.abandonment().is_closed());

        let parts = forced.into_forced_retirement_parts();
        assert!(parts.is_forced_only());
        assert!(format!("{parts:?}").contains("<forced-only>"));
        let (provider, abandonment) = parts.into_provider_and_abandonment();
        assert!(abandonment.provider_panicked);
        assert_eq!(provider.cancelled.len(), 1);
        assert_eq!(
            provider
                .synchronized
                .iter()
                .filter(|update| matches!(update, SemanticSnapshotUpdateV1::Close { .. }))
                .count(),
            0
        );
    }

    #[test]
    fn bridge_forced_extraction_closes_over_a_close_panic_without_later_effects() {
        let provider = ModelProvider {
            state: ready_state(1),
            available: true,
            ..ModelProvider::default()
        };
        let mut bridge = SemanticBridgeServiceV1::try_new(provider, bridge_limits()).unwrap();
        bridge.admit_client(client(1)).unwrap();
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));
        let _ = bridge.open_snapshot(client(1), snapshot(4, 1));
        bridge.inner.provider.panic_on_close = true;

        let forced =
            bridge.force_into_retiring_provider(SemanticBridgeForcedEffectPolicyV1::AttemptBounded);
        assert_eq!(
            forced.abandonment(),
            SemanticBridgeForcedAbandonmentReceiptV1 {
                clients_abandoned: 1,
                retiring_clients_abandoned: 0,
                documents_abandoned: 2,
                pending_requests_abandoned: 0,
                queued_deliveries_abandoned: 0,
                quarantined_requests: 0,
                cancellation_signals_attempted: 0,
                cancellation_signals_skipped: 0,
                close_attempts: 1,
                closes_accepted: 0,
                closes_unavailable: 0,
                closes_refused: 0,
                close_outcomes_unknown: 1,
                closes_skipped: 1,
                provider_panicked: true,
            }
        );
        assert!(forced.abandonment().is_closed());

        let parts = forced.into_forced_retirement_parts();
        assert_eq!(parts.abandonment().close_outcomes_unknown, 1);
        let (provider, abandonment) = parts.into_provider_and_abandonment();
        assert!(abandonment.provider_panicked);
        assert!(provider.cancelled.is_empty());
        assert_eq!(
            provider
                .synchronized
                .iter()
                .filter(|update| matches!(update, SemanticSnapshotUpdateV1::Close { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn bridge_cancel_pre_transition_satisfies_without_second_signal() {
        let mut bridge = bridge_service();
        bridge.admit_client(client(1)).unwrap();
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));
        let offered = request(3, 1, 7);
        let _ = bridge.submit(client(1), offered.clone());
        bridge.inner.provider.state = alternate_ready_state(2);
        let report = bridge.cancel(client(1), offered.request);
        assert_eq!(
            report.outcome(),
            &SemanticBridgeCancelActionV1::SatisfiedByProviderTransition
        );
        assert_eq!(
            report
                .pre_provider()
                .unwrap()
                .invalidations()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![offered.request]
        );
        assert_eq!(bridge.inner.provider.cancelled, vec![offered.request]);
    }

    #[test]
    fn bridge_provider_stage_settles_cancel_induced_aba_before_middle_operation() {
        let mut bridge = bridge_service();
        bridge.admit_client(client(1)).unwrap();
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));
        let prior = request(3, 1, 7);
        let _ = bridge.submit(client(1), prior.clone());
        bridge.inner.provider.state = alternate_ready_state(2);
        bridge.inner.provider.state_after_cancel = Some(ready_state(1));

        let next = request(3, 1, 8);
        let report = bridge.submit(client(1), next.clone());
        let transition = report.pre_provider().unwrap();
        assert_eq!(transition.observed_state(), &alternate_ready_state(2));
        assert_eq!(transition.settled_state(), &ready_state(1));
        assert_eq!(report.provider_before(), &ready_state(1));
        assert_eq!(report.provider_after(), &ready_state(1));
        assert_eq!(report.outcome(), &SemanticSubmitOutcomeV1::Accepted);
        assert_eq!(bridge.inner.provider.submitted.last(), Some(&next));
        assert_eq!(bridge.inner.provider.cancelled, vec![prior.request]);
    }

    #[test]
    fn bridge_poll_distinguishes_absence_failure_delivery_late_and_orphan() {
        let mut absent = bridge_service();
        absent.admit_client(client(1)).unwrap();
        assert_eq!(
            absent.poll_once().outcome(),
            &SemanticBridgePollOutcomeV1::NoEvent
        );
        absent.inner.provider.bridge_poll_failure = Some(SemanticBridgePollFailureV1::Poisoned);
        let failure = absent.poll_once();
        assert_eq!(
            failure.outcome(),
            &SemanticBridgePollOutcomeV1::Failure(SemanticBridgePollFailureV1::Poisoned)
        );
        assert!(failure.post_provider().is_none());

        let mut delivered = bridge_service();
        delivered.admit_client(client(1)).unwrap();
        let _ = delivered.open_snapshot(client(1), snapshot(3, 1));
        let offered = request(3, 1, 7);
        let _ = delivered.submit(client(1), offered.clone());
        delivered
            .inner
            .provider
            .completions
            .push_back(SemanticProviderCompletionV1 {
                request: offered.request,
                outcome: SemanticOutcomeV1::Stale,
            });
        assert!(matches!(
            delivered.poll_once().outcome(),
            SemanticBridgePollOutcomeV1::Completion(
                SemanticBridgePollDispositionV1::Delivery(SemanticDeliveryV1 {
                    request,
                    outcome: SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Stale),
                })
            ) if *request == offered.request
        ));

        let mut late = bridge_service();
        late.admit_client(client(1)).unwrap();
        let _ = late.open_snapshot(client(1), snapshot(3, 1));
        let late_request = request(3, 1, 8);
        let _ = late.submit(client(1), late_request.clone());
        late.inner
            .provider
            .completions
            .push_back(SemanticProviderCompletionV1 {
                request: late_request.request,
                outcome: SemanticOutcomeV1::Stale,
            });
        late.inner.provider.state_after_poll = Some(alternate_ready_state(2));
        assert_eq!(
            late.poll_once().outcome(),
            &SemanticBridgePollOutcomeV1::Completion(SemanticBridgePollDispositionV1::Late {
                request: late_request.request
            })
        );

        let orphan = SemanticRequestRefV1::new([99; 16]);
        late.inner
            .provider
            .completions
            .push_back(SemanticProviderCompletionV1 {
                request: orphan,
                outcome: SemanticOutcomeV1::Stale,
            });
        assert_eq!(
            late.poll_once().outcome(),
            &SemanticBridgePollOutcomeV1::Completion(SemanticBridgePollDispositionV1::Orphan {
                request: orphan
            })
        );
    }

    #[test]
    fn bridge_poll_preserves_aba_pre_and_post_transitions() {
        let mut bridge = bridge_service();
        bridge.admit_client(client(1)).unwrap();
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));
        let offered = request(3, 1, 7);
        let _ = bridge.submit(client(1), offered.clone());
        bridge.inner.provider.state = alternate_ready_state(2);
        bridge.inner.provider.state_after_poll = Some(ready_state(1));
        bridge
            .inner
            .provider
            .completions
            .push_back(SemanticProviderCompletionV1 {
                request: offered.request,
                outcome: SemanticOutcomeV1::Stale,
            });
        let report = bridge.poll_once();
        assert!(report.pre_provider().is_some());
        assert!(report.post_provider().is_some());
        assert_eq!(report.provider_before(), &ready_state(1));
        assert_eq!(report.provider_after(), &ready_state(1));
        assert_eq!(
            report.outcome(),
            &SemanticBridgePollOutcomeV1::Completion(SemanticBridgePollDispositionV1::Late {
                request: offered.request
            })
        );
        assert!(!format!("{report:?}").contains("model-provider"));
    }

    #[test]
    fn bridge_complete_delivery_debug_reveals_no_semantic_payload_metadata() {
        let mut bridge = bridge_service();
        bridge.admit_client(client(1)).unwrap();
        let _ = bridge.open_snapshot(client(1), snapshot(3, 1));
        let offered = request(3, 1, 7);
        let _ = bridge.submit(client(1), offered.clone());
        let result =
            SemanticResultV1::try_diagnostics(vec![], SemanticLimitsV1::reference()).unwrap();
        bridge
            .inner
            .provider
            .completions
            .push_back(SemanticProviderCompletionV1 {
                request: offered.request,
                outcome: SemanticOutcomeV1::Complete(SemanticReplyV1 {
                    request: offered.request,
                    target: offered.target,
                    index_generation: SemanticIndexGenerationV1::new(1).unwrap(),
                    provider: provenance(),
                    result,
                }),
            });
        let report = bridge.poll_once();
        assert!(matches!(
            report.outcome(),
            SemanticBridgePollOutcomeV1::Completion(SemanticBridgePollDispositionV1::Delivery(_))
        ));
        let debug = format!("{report:?}");
        for forbidden in [
            "model-provider",
            "1.0.0",
            "SemanticReplyV1",
            "source_revision",
            "items",
            "SemanticRequestRefV1",
        ] {
            assert!(!debug.contains(forbidden), "leaked {forbidden}: {debug}");
        }
    }

    #[test]
    fn exact_snapshots_reject_out_of_order_and_invalidate_older_requests() {
        let mut service = service();
        service.admit_client(client(1)).unwrap();
        assert_eq!(
            service.open_snapshot(client(1), snapshot(3, 4)),
            SemanticAdmissionOutcomeV1::Accepted
        );
        assert_eq!(
            service.submit(client(1), request(3, 4, 7)),
            SemanticSubmitOutcomeV1::Accepted
        );
        assert_eq!(
            service.change_snapshot(client(1), snapshot(3, 3)),
            SemanticAdmissionOutcomeV1::Refused(SemanticServiceRefusalV1::OutOfOrderRevision)
        );
        assert_eq!(
            service.change_snapshot(client(1), snapshot(3, 5)),
            SemanticAdmissionOutcomeV1::Accepted
        );
        assert_eq!(
            service.take_delivery(client(1)).unwrap().unwrap().outcome,
            SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Stale)
        );
        assert_eq!(service.usage().requests_invalidated, 1);
    }

    #[test]
    fn point_queries_refuse_split_utf8_offsets_before_provider_effect() {
        let mut service = service();
        service.provider.state.capabilities = SemanticCapabilitySetV1::top();
        service.refresh_provider_state();
        service.admit_client(client(1)).unwrap();
        let unicode_snapshot = SemanticSnapshotV1::try_new(
            target(4, 1),
            "rust".into(),
            "é".into(),
            SemanticServiceLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(
            service.open_snapshot(client(1), unicode_snapshot),
            SemanticAdmissionOutcomeV1::Accepted
        );
        let queries = [
            SemanticQueryV1::Definition { offset: 1 },
            SemanticQueryV1::References { offset: 1 },
            SemanticQueryV1::Implementations { offset: 1 },
            SemanticQueryV1::TypeAtPoint { offset: 1 },
            SemanticQueryV1::PlanRename {
                offset: 1,
                new_name: SemanticTextV1::try_new("renamed".into(), 16).unwrap(),
            },
        ];
        for (index, query) in queries.into_iter().enumerate() {
            let request = SemanticRequestV1 {
                request: SemanticRequestRefV1::new([40 + index as u8; 16]),
                target: target(4, 1),
                expected_index: SemanticIndexGenerationV1::new(1),
                language: SemanticTextV1::try_new("rust".into(), 16).unwrap(),
                query,
            };
            assert_eq!(
                service.submit(client(1), request),
                SemanticSubmitOutcomeV1::Refused(SemanticServiceRefusalV1::InputLimit)
            );
        }
        assert!(service.provider.submitted.is_empty());

        let eof = SemanticRequestV1 {
            request: SemanticRequestRefV1::new([50; 16]),
            target: target(4, 1),
            expected_index: SemanticIndexGenerationV1::new(1),
            language: SemanticTextV1::try_new("rust".into(), 16).unwrap(),
            query: SemanticQueryV1::Definition { offset: 2 },
        };
        assert_eq!(
            service.submit(client(1), eof),
            SemanticSubmitOutcomeV1::Accepted
        );
        assert_eq!(service.provider.submitted.len(), 1);
    }

    #[test]
    fn unavailable_stale_and_unsupported_are_distinct() {
        let mut service = service();
        service.admit_client(client(1)).unwrap();
        service.open_snapshot(client(1), snapshot(2, 1));
        let mut stale = request(2, 2, 1);
        assert_eq!(
            service.submit(client(1), stale.clone()),
            SemanticSubmitOutcomeV1::Stale
        );
        service.provider.state = SemanticProviderStateV1::unavailable(provenance());
        service.refresh_provider_state();
        stale.target.source_revision = SemanticSourceRevisionV1::new(1);
        assert_eq!(
            service.submit(client(1), stale.clone()),
            SemanticSubmitOutcomeV1::Unavailable
        );
        service.provider.state = SemanticProviderStateV1 {
            provider: provenance(),
            readiness: SemanticProviderReadinessV1::Ready {
                generation: SemanticIndexGenerationV1::new(1).unwrap(),
            },
            capabilities: SemanticCapabilitySetV1::bottom(),
        };
        service.refresh_provider_state();
        assert_eq!(
            service.submit(client(1), stale.clone()),
            SemanticSubmitOutcomeV1::Unsupported(SemanticQueryKindV1::Diagnostics)
        );
        service.provider.state = SemanticProviderStateV1 {
            provider: provenance(),
            readiness: SemanticProviderReadinessV1::Indexing,
            capabilities: SemanticCapabilitySetV1::read_only(),
        };
        service.refresh_provider_state();
        assert_eq!(
            service.submit(client(1), stale),
            SemanticSubmitOutcomeV1::Unavailable
        );
    }

    #[test]
    fn newer_index_generation_cancels_requests_bound_to_the_old_generation() {
        let mut service = service();
        service.admit_client(client(1)).unwrap();
        service.open_snapshot(client(1), snapshot(2, 1));
        service.submit(client(1), request(2, 1, 9));
        service.provider.state = ready_state(2);
        service.refresh_provider_state();
        assert_eq!(
            service.take_delivery(client(1)).unwrap().unwrap().outcome,
            SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Stale)
        );
        assert_eq!(service.usage().requests_invalidated, 1);
    }

    #[test]
    fn provider_ready_to_indexing_during_poll_invalidates_once() {
        let mut service = service();
        service.admit_client(client(1)).unwrap();
        assert_eq!(
            service.open_snapshot(client(1), snapshot(2, 1)),
            SemanticAdmissionOutcomeV1::Accepted
        );
        let offered = request(2, 1, 9);
        assert_eq!(
            service.submit(client(1), offered.clone()),
            SemanticSubmitOutcomeV1::Accepted
        );
        service.provider.state_after_poll = Some(SemanticProviderStateV1 {
            provider: provenance(),
            readiness: SemanticProviderReadinessV1::Indexing,
            capabilities: SemanticCapabilitySetV1::read_only(),
        });

        service.pump();

        assert_eq!(
            service.take_delivery(client(1)).unwrap().unwrap().outcome,
            SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Stale)
        );
        assert_eq!(service.provider.cancelled, vec![offered.request]);
        assert_eq!(service.usage().requests_invalidated, 1);
        assert_eq!(
            service.provider_state().readiness,
            SemanticProviderReadinessV1::Indexing
        );

        service.pump();
        assert_eq!(service.take_delivery(client(1)).unwrap(), None);
        assert_eq!(service.usage().requests_invalidated, 1);
    }

    #[test]
    fn provider_ready_to_unavailable_during_poll_fails_closed_once() {
        let mut service = service();
        service.admit_client(client(1)).unwrap();
        service.open_snapshot(client(1), snapshot(2, 1));
        let offered = request(2, 1, 9);
        service.submit(client(1), offered.clone());
        service.provider.state_after_poll =
            Some(SemanticProviderStateV1::unavailable(provenance()));

        service.pump();

        assert_eq!(
            service.take_delivery(client(1)).unwrap().unwrap().outcome,
            SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Stale)
        );
        assert_eq!(service.provider.cancelled, vec![offered.request]);
        assert_eq!(service.usage().requests_invalidated, 1);
        assert_eq!(
            service.submit(client(1), request(2, 1, 10)),
            SemanticSubmitOutcomeV1::Unavailable
        );
        service.refresh_provider_state();
        assert_eq!(service.take_delivery(client(1)).unwrap(), None);
        assert_eq!(service.usage().requests_invalidated, 1);
    }

    #[test]
    fn provider_provenance_is_exact_and_transition_invalidates_pending_once() {
        let mut service = service();
        service.admit_client(client(1)).unwrap();
        service.open_snapshot(client(1), snapshot(2, 1));
        let mismatched = request(2, 1, 41);
        service.submit(client(1), mismatched.clone());
        let result =
            SemanticResultV1::try_diagnostics(vec![], SemanticLimitsV1::reference()).unwrap();
        service
            .provider
            .completions
            .push_back(SemanticProviderCompletionV1 {
                request: mismatched.request,
                outcome: SemanticOutcomeV1::Complete(SemanticReplyV1 {
                    request: mismatched.request,
                    target: mismatched.target,
                    index_generation: mismatched.expected_index.unwrap(),
                    provider: alternate_provenance(),
                    result,
                }),
            });
        service.pump();
        assert_eq!(
            service.take_delivery(client(1)).unwrap().unwrap().outcome,
            SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Refused(
                SemanticRefusalV1::MalformedProviderReply,
            ))
        );

        let pending = request(2, 1, 42);
        service.submit(client(1), pending.clone());
        service.provider.state.provider = alternate_provenance();
        service.refresh_provider_state();
        assert_eq!(
            service.take_delivery(client(1)).unwrap().unwrap().outcome,
            SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Stale)
        );
        assert_eq!(service.provider.cancelled.last(), Some(&pending.request));
        assert_eq!(service.usage().requests_invalidated, 1);
        service.refresh_provider_state();
        assert_eq!(service.take_delivery(client(1)).unwrap(), None);
        assert_eq!(service.usage().requests_invalidated, 1);
    }

    #[test]
    fn cancellation_and_client_isolation_are_fail_closed() {
        let mut service = service();
        service.admit_client(client(1)).unwrap();
        service.admit_client(client(2)).unwrap();
        service.open_snapshot(client(1), snapshot(2, 1));
        let offered = request(2, 1, 9);
        service.submit(client(1), offered.clone());
        assert_eq!(
            service.cancel(client(2), offered.request),
            Err(SemanticServiceRefusalV1::ClientIsolation)
        );
        assert_eq!(service.take_delivery(client(2)).unwrap(), None);
        assert_eq!(service.cancel(client(1), offered.request), Ok(true));
        assert_eq!(
            service.take_delivery(client(1)).unwrap().unwrap().outcome,
            SemanticDeliveryOutcomeV1::Cancelled
        );
    }

    #[test]
    fn composition_expiry_is_distinct_isolated_and_late_completion_is_orphaned() {
        let mut service = service();
        service.admit_client(client(1)).unwrap();
        service.admit_client(client(2)).unwrap();
        service.open_snapshot(client(1), snapshot(2, 1));
        let offered = request(2, 1, 9);
        assert_eq!(
            service.submit(client(1), offered.clone()),
            SemanticSubmitOutcomeV1::Accepted
        );
        assert_eq!(
            service.expire(client(2), offered.request),
            Err(SemanticServiceRefusalV1::ClientIsolation)
        );
        assert_eq!(service.expire(client(1), offered.request), Ok(true));
        assert_eq!(service.expire(client(1), offered.request), Ok(false));
        assert_eq!(
            service.take_delivery(client(1)).unwrap().unwrap().outcome,
            SemanticDeliveryOutcomeV1::TimedOut
        );
        assert_eq!(service.take_delivery(client(2)).unwrap(), None);
        assert_eq!(service.provider.cancelled, vec![offered.request]);
        assert_eq!(service.usage().requests_timed_out, 1);

        service
            .provider
            .completions
            .push_back(SemanticProviderCompletionV1 {
                request: offered.request,
                outcome: SemanticOutcomeV1::Stale,
            });
        service.pump();
        assert_eq!(service.take_delivery(client(1)).unwrap(), None);
        assert_eq!(service.usage().orphan_completions, 1);
    }

    #[test]
    fn full_delivery_destination_preserves_pending_and_blocks_replacement_pre_effect() {
        let limits = SemanticServiceLimitsV1 {
            max_deliveries: 1,
            ..SemanticServiceLimitsV1::reference()
        };
        let provider = ModelProvider {
            state: ready_state(1),
            available: true,
            ..ModelProvider::default()
        };
        let mut service = SemanticServiceV1::try_new(provider, limits).unwrap();
        service.admit_client(client(1)).unwrap();
        service.open_snapshot(client(1), snapshot(3, 1));
        let offered = request(3, 1, 47);
        assert_eq!(
            service.submit(client(1), offered.clone()),
            SemanticSubmitOutcomeV1::Accepted
        );
        service
            .deliveries
            .get_mut(&client(1))
            .unwrap()
            .push_back(SemanticDeliveryV1 {
                request: SemanticRequestRefV1::new([99; 16]),
                outcome: SemanticDeliveryOutcomeV1::Cancelled,
            });

        assert_eq!(
            service.expire(client(1), offered.request),
            Err(SemanticServiceRefusalV1::Capacity)
        );
        assert_eq!(service.is_pending_for(client(1), offered.request), Ok(true));
        assert!(service.provider.cancelled.is_empty());
        let candidate = ModelProvider {
            state: alternate_ready_state(2),
            available: true,
            ..ModelProvider::default()
        };
        let failure = service.try_replace_provider(candidate).err().unwrap();
        assert_eq!(
            failure.refusal(),
            SemanticProviderReplacementRefusalV1::DeliveryCapacity
        );
        assert!(failure.into_candidate().synchronized.is_empty());
        assert_eq!(service.provider_state(), ready_state(1));
        assert!(service.provider.cancelled.is_empty());
        assert_eq!(
            service.submit(client(1), offered),
            SemanticSubmitOutcomeV1::Refused(SemanticServiceRefusalV1::DuplicateRequest)
        );
    }

    #[test]
    fn provider_replacement_replays_deterministically_then_swaps_once() {
        let mut service = service();
        service.admit_client(client(1)).unwrap();
        service.open_snapshot(client(1), snapshot(7, 1));
        service.open_snapshot(client(1), snapshot(3, 1));
        let offered = request(3, 1, 44);
        service.submit(client(1), offered.clone());
        let candidate = ModelProvider {
            state: alternate_ready_state(2),
            available: true,
            ..ModelProvider::default()
        };

        let retirement = service.try_replace_provider(candidate).unwrap();

        let replayed_targets: Vec<_> = service
            .provider
            .synchronized
            .iter()
            .map(|update| match update {
                SemanticSnapshotUpdateV1::Admit(snapshot) => snapshot.target(),
                SemanticSnapshotUpdateV1::Close { .. } => panic!("replacement only replays admits"),
            })
            .collect();
        assert_eq!(replayed_targets, vec![target(3, 1), target(7, 1)]);
        assert_eq!(service.provider_state(), alternate_ready_state(2));
        assert_eq!(
            retirement.state(),
            SemanticProviderRetirementStateV1::Drained
        );
        assert_eq!(retirement.usage().close_attempts, 2);
        assert_eq!(retirement.usage().closes_accepted, 2);
        let old = retirement.into_provider_if_drained().unwrap();
        assert_eq!(old.cancelled, vec![offered.request]);
        assert_eq!(
            &old.synchronized[old.synchronized.len() - 2..],
            [
                SemanticSnapshotUpdateV1::Close {
                    target: target(3, 1),
                },
                SemanticSnapshotUpdateV1::Close {
                    target: target(7, 1),
                },
            ]
        );
        assert_eq!(
            service.take_delivery(client(1)).unwrap().unwrap().outcome,
            SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Stale)
        );
        assert_eq!(service.usage().provider_replacements, 1);
        assert_eq!(service.usage().snapshots_replayed, 2);
        assert_eq!(service.usage().requests_invalidated, 1);
    }

    #[test]
    fn replacement_retirement_preserves_failed_close_for_one_bounded_retry() {
        let mut service = service();
        service.admit_client(client(1)).unwrap();
        service.open_snapshot(client(1), snapshot(7, 1));
        service.open_snapshot(client(1), snapshot(3, 1));
        service.provider.close_refusals_remaining = 1;
        let candidate = ModelProvider {
            state: alternate_ready_state(2),
            available: true,
            ..ModelProvider::default()
        };

        let retirement = service.try_replace_provider(candidate).unwrap();

        assert_eq!(
            retirement.state(),
            SemanticProviderRetirementStateV1::Pending { remaining: 1 }
        );
        assert_eq!(retirement.usage().close_attempts, 2);
        assert_eq!(retirement.usage().closes_refused, 1);
        let mut retirement = match retirement.into_provider_if_drained() {
            Ok(_) => panic!("failed close cannot produce a drained provider"),
            Err(retirement) => retirement,
        };
        assert_eq!(
            retirement.retry_drain(),
            SemanticProviderRetirementStateV1::Drained
        );
        assert_eq!(retirement.usage().close_attempts, 3);
        assert_eq!(retirement.usage().closes_accepted, 2);
    }

    #[test]
    fn retirement_retry_and_provider_state_do_not_invent_graceful_shutdown() {
        let mut service = service();
        service.admit_client(client(1)).unwrap();
        service.open_snapshot(client(1), snapshot(3, 1));
        service.provider.close_refusals_remaining = 1;
        service.provider.state_after_synchronize =
            Some(SemanticProviderStateV1::unavailable(provenance()));
        let candidate = ModelProvider {
            state: alternate_ready_state(2),
            available: true,
            ..ModelProvider::default()
        };

        let mut retirement = service.try_replace_provider(candidate).unwrap();
        assert_eq!(
            retirement.state(),
            SemanticProviderRetirementStateV1::Pending { remaining: 1 }
        );
        assert_eq!(
            retirement.provider_state().readiness,
            SemanticProviderReadinessV1::Unavailable
        );
        assert_eq!(
            retirement.retry_drain(),
            SemanticProviderRetirementStateV1::Drained
        );
        assert_eq!(retirement.usage().close_attempts, 2);
        assert_eq!(retirement.usage().closes_refused, 1);
        assert_eq!(retirement.usage().closes_accepted, 1);
    }

    #[test]
    fn replacing_without_documents_returns_an_empty_retirement_plan() {
        let mut service = service();
        let candidate = ModelProvider {
            state: alternate_ready_state(2),
            available: true,
            ..ModelProvider::default()
        };

        let retirement = service.try_replace_provider(candidate).unwrap();

        assert_eq!(
            retirement.state(),
            SemanticProviderRetirementStateV1::Drained
        );
        assert_eq!(retirement.usage().close_attempts, 0);
    }

    #[test]
    fn replacement_failure_returns_candidate_without_touching_authoritative_state() {
        let mut service = service();
        service.admit_client(client(1)).unwrap();
        service.open_snapshot(client(1), snapshot(3, 1));
        let offered = request(3, 1, 45);
        service.submit(client(1), offered.clone());
        let candidate = ModelProvider {
            state: alternate_ready_state(2),
            available: true,
            refuse_synchronization: true,
            ..ModelProvider::default()
        };

        let failure = service.try_replace_provider(candidate).err().unwrap();

        assert_eq!(
            failure.refusal(),
            SemanticProviderReplacementRefusalV1::SynchronizationRefused
        );
        assert_eq!(failure.into_candidate().synchronized.len(), 1);
        assert_eq!(service.provider_state(), ready_state(1));
        assert!(service.provider.cancelled.is_empty());
        assert_eq!(service.take_delivery(client(1)).unwrap(), None);
        assert_eq!(
            service.submit(client(1), offered),
            SemanticSubmitOutcomeV1::Refused(SemanticServiceRefusalV1::DuplicateRequest)
        );
        assert_eq!(service.usage().provider_replacements, 0);
        assert_eq!(service.usage().provider_replacement_refusals, 1);
        assert_eq!(service.usage().snapshots_replayed, 0);
    }

    #[test]
    fn replacement_requires_ready_fresh_and_stable_candidate_evidence() {
        let mut service = service();
        service.admit_client(client(1)).unwrap();
        service.open_snapshot(client(1), snapshot(3, 1));

        let not_ready = ModelProvider {
            state: SemanticProviderStateV1::unavailable(alternate_provenance()),
            available: true,
            ..ModelProvider::default()
        };
        assert_eq!(
            service
                .try_replace_provider(not_ready)
                .err()
                .unwrap()
                .refusal(),
            SemanticProviderReplacementRefusalV1::CandidateNotReady
        );

        let not_fresh = ModelProvider {
            state: ready_state(2),
            available: true,
            ..ModelProvider::default()
        };
        assert_eq!(
            service
                .try_replace_provider(not_fresh)
                .err()
                .unwrap()
                .refusal(),
            SemanticProviderReplacementRefusalV1::CandidateNotFresh
        );

        let unstable = ModelProvider {
            state: alternate_ready_state(2),
            state_after_synchronize: Some(SemanticProviderStateV1 {
                provider: alternate_provenance(),
                readiness: SemanticProviderReadinessV1::Indexing,
                capabilities: SemanticCapabilitySetV1::read_only(),
            }),
            available: true,
            ..ModelProvider::default()
        };
        assert_eq!(
            service
                .try_replace_provider(unstable)
                .err()
                .unwrap()
                .refusal(),
            SemanticProviderReplacementRefusalV1::CandidateStateChanged
        );
        assert_eq!(service.provider_state(), ready_state(1));
        assert_eq!(service.usage().provider_replacement_refusals, 3);
    }

    #[test]
    fn out_of_order_completions_route_without_cross_client_disclosure() {
        let mut service = service();
        for tag in [1, 2] {
            service.admit_client(client(tag)).unwrap();
            service.open_snapshot(client(tag), snapshot(tag, 1));
            service.submit(client(tag), request(tag, 1, tag + 10));
        }
        let result =
            SemanticResultV1::try_diagnostics(vec![], SemanticLimitsV1::reference()).unwrap();
        for tag in [2, 1] {
            let offered = request(tag, 1, tag + 10);
            service
                .provider
                .completions
                .push_back(SemanticProviderCompletionV1 {
                    request: offered.request,
                    outcome: SemanticOutcomeV1::Complete(SemanticReplyV1 {
                        request: offered.request,
                        target: offered.target,
                        index_generation: offered.expected_index.unwrap(),
                        provider: provenance(),
                        result: result.clone(),
                    }),
                });
        }
        service.pump();
        assert_eq!(
            service.take_delivery(client(1)).unwrap().unwrap().request,
            request(1, 1, 11).request
        );
        assert_eq!(
            service.take_delivery(client(2)).unwrap().unwrap().request,
            request(2, 1, 12).request
        );
    }

    #[test]
    fn provider_plans_cannot_target_foreign_snapshots_or_ambiguous_ranges() {
        let mut service = service();
        service.provider.state.capabilities = SemanticCapabilitySetV1::top();
        service.refresh_provider_state();
        for tag in [1, 2] {
            service.admit_client(client(tag)).unwrap();
            assert_eq!(
                service.open_snapshot(client(tag), snapshot(tag, 1)),
                SemanticAdmissionOutcomeV1::Accepted
            );
        }
        let unicode_snapshot = SemanticSnapshotV1::try_new(
            target(3, 1),
            "rust".into(),
            "é".into(),
            SemanticServiceLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(
            service.open_snapshot(client(1), unicode_snapshot),
            SemanticAdmissionOutcomeV1::Accepted
        );
        let rename_request = |request_tag| SemanticRequestV1 {
            request: SemanticRequestRefV1::new([request_tag; 16]),
            target: target(1, 1),
            expected_index: SemanticIndexGenerationV1::new(1),
            language: SemanticTextV1::try_new("rust".into(), 16).unwrap(),
            query: SemanticQueryV1::PlanRename {
                offset: 1,
                new_name: SemanticTextV1::try_new("renamed".into(), 16).unwrap(),
            },
        };
        let replacement = || SemanticTextV1::try_new("x".into(), 16).unwrap();

        let foreign = rename_request(31);
        assert_eq!(
            service.submit(client(1), foreign.clone()),
            SemanticSubmitOutcomeV1::Accepted
        );
        let foreign_result = SemanticResultV1::try_rename_plan(
            vec![SemanticTextEditV1 {
                location: SemanticLocationV1 {
                    target: target(2, 1),
                    span: SemanticSpanV1::try_new(0, 1).unwrap(),
                },
                replacement: replacement(),
            }],
            SemanticLimitsV1::reference(),
        )
        .unwrap();
        service
            .provider
            .completions
            .push_back(SemanticProviderCompletionV1 {
                request: foreign.request,
                outcome: SemanticOutcomeV1::Complete(SemanticReplyV1 {
                    request: foreign.request,
                    target: foreign.target,
                    index_generation: foreign.expected_index.unwrap(),
                    provider: provenance(),
                    result: foreign_result,
                }),
            });
        service.pump();
        assert_eq!(
            service.take_delivery(client(1)).unwrap().unwrap().outcome,
            SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Refused(
                SemanticRefusalV1::MalformedProviderReply,
            ))
        );

        let split_scalar = rename_request(33);
        assert_eq!(
            service.submit(client(1), split_scalar.clone()),
            SemanticSubmitOutcomeV1::Accepted
        );
        let split_result = SemanticResultV1::try_rename_plan(
            vec![SemanticTextEditV1 {
                location: SemanticLocationV1 {
                    target: target(3, 1),
                    span: SemanticSpanV1::try_new(1, 2).unwrap(),
                },
                replacement: replacement(),
            }],
            SemanticLimitsV1::reference(),
        )
        .unwrap();
        service
            .provider
            .completions
            .push_back(SemanticProviderCompletionV1 {
                request: split_scalar.request,
                outcome: SemanticOutcomeV1::Complete(SemanticReplyV1 {
                    request: split_scalar.request,
                    target: split_scalar.target,
                    index_generation: split_scalar.expected_index.unwrap(),
                    provider: provenance(),
                    result: split_result,
                }),
            });
        service.pump();
        assert_eq!(
            service.take_delivery(client(1)).unwrap().unwrap().outcome,
            SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Refused(
                SemanticRefusalV1::MalformedProviderReply,
            ))
        );

        let overlapping = rename_request(32);
        assert_eq!(
            service.submit(client(1), overlapping.clone()),
            SemanticSubmitOutcomeV1::Accepted
        );
        let overlapping_result = SemanticResultV1::try_rename_plan(
            vec![
                SemanticTextEditV1 {
                    location: SemanticLocationV1 {
                        target: target(1, 1),
                        span: SemanticSpanV1::try_new(0, 2).unwrap(),
                    },
                    replacement: replacement(),
                },
                SemanticTextEditV1 {
                    location: SemanticLocationV1 {
                        target: target(1, 1),
                        span: SemanticSpanV1::try_new(1, 3).unwrap(),
                    },
                    replacement: replacement(),
                },
            ],
            SemanticLimitsV1::reference(),
        )
        .unwrap();
        service
            .provider
            .completions
            .push_back(SemanticProviderCompletionV1 {
                request: overlapping.request,
                outcome: SemanticOutcomeV1::Complete(SemanticReplyV1 {
                    request: overlapping.request,
                    target: overlapping.target,
                    index_generation: overlapping.expected_index.unwrap(),
                    provider: provenance(),
                    result: overlapping_result,
                }),
            });
        service.pump();
        assert_eq!(
            service.take_delivery(client(1)).unwrap().unwrap().outcome,
            SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Refused(
                SemanticRefusalV1::MalformedProviderReply,
            ))
        );
    }

    #[test]
    fn limits_and_observations_are_content_safe() {
        let limits = SemanticServiceLimitsV1 {
            max_snapshot_bytes: 3,
            max_total_snapshot_bytes: 3,
            max_clients: 1,
            max_documents: 1,
            max_pending: 1,
            max_deliveries: 1,
            max_provider_polls_per_pump: 1,
            peer: SemanticLimitsV1::reference(),
        };
        assert_eq!(
            SemanticSnapshotV1::try_new(target(1, 1), "rust".into(), "secret".into(), limits),
            Err(SemanticServiceRefusalV1::InputLimit)
        );
        let rendered = format!("{:?}", snapshot(1, 1));
        assert!(!rendered.contains("bounded"));
        assert!(!rendered.contains("rust"));
        let usage = service().usage();
        let rendered = format!("{usage:?}");
        assert!(!rendered.contains("source"));
        assert!(!SemanticCapabilitySetV1::read_only().contains(SemanticCapabilityV1::PlanRename));
    }

    #[test]
    fn aggregate_snapshot_budget_refuses_before_provider_effect_and_recovers_on_close() {
        let limits = SemanticServiceLimitsV1 {
            max_snapshot_bytes: 16,
            max_total_snapshot_bytes: 16,
            max_clients: 1,
            max_documents: 2,
            ..SemanticServiceLimitsV1::reference()
        };
        let provider = ModelProvider {
            state: ready_state(1),
            available: true,
            ..ModelProvider::default()
        };
        let mut service = SemanticServiceV1::try_new(provider, limits).unwrap();
        service.admit_client(client(1)).unwrap();
        let first =
            SemanticSnapshotV1::try_new(target(1, 1), "rust".into(), "0123456789".into(), limits)
                .unwrap();
        let second =
            SemanticSnapshotV1::try_new(target(2, 1), "rust".into(), "abcdefghij".into(), limits)
                .unwrap();
        assert_eq!(
            service.open_snapshot(client(1), first),
            SemanticAdmissionOutcomeV1::Accepted
        );
        assert_eq!(service.usage().current_snapshot_bytes, 10);
        assert_eq!(service.usage().high_water_snapshot_bytes, 10);
        assert_eq!(
            service.open_snapshot(client(1), second.clone()),
            SemanticAdmissionOutcomeV1::Refused(SemanticServiceRefusalV1::Capacity)
        );
        assert_eq!(service.provider.synchronized.len(), 1);
        assert_eq!(
            service.close_snapshot(client(1), target(1, 1)),
            SemanticAdmissionOutcomeV1::Accepted
        );
        assert_eq!(service.usage().current_snapshot_bytes, 0);
        assert_eq!(
            service.open_snapshot(client(1), second),
            SemanticAdmissionOutcomeV1::Accepted
        );
        assert_eq!(service.usage().high_water_snapshot_bytes, 10);
    }

    #[test]
    fn retirement_reclaims_partition_and_quarantines_late_completion_correlation() {
        let mut service = service();
        service.admit_client(client(1)).unwrap();
        service.admit_client(client(2)).unwrap();
        assert_eq!(
            service.open_snapshot(client(1), snapshot(1, 1)),
            SemanticAdmissionOutcomeV1::Accepted
        );

        let delivered = request(1, 1, 61);
        assert_eq!(
            service.submit(client(1), delivered.clone()),
            SemanticSubmitOutcomeV1::Accepted
        );
        assert_eq!(service.cancel(client(1), delivered.request), Ok(true));
        let pending = request(1, 1, 62);
        assert_eq!(
            service.submit(client(1), pending.clone()),
            SemanticSubmitOutcomeV1::Accepted
        );

        let report = service.retire_client(client(1));
        assert_eq!(
            report,
            SemanticClientRetirementReportV1 {
                state: SemanticClientRetirementStateV1::Retired {
                    quarantined_requests: 2,
                },
                cancellation_signals: 1,
                deliveries_discarded: 1,
                closes_accepted: 1,
                closes_unavailable: 0,
                closes_refused: 0,
            }
        );
        assert_eq!(
            service.take_delivery(client(1)),
            Err(SemanticServiceRefusalV1::UnknownClient)
        );
        assert_eq!(service.take_delivery(client(2)).unwrap(), None);
        assert_eq!(
            service.provider.cancelled,
            vec![delivered.request, pending.request]
        );

        service.admit_client(client(1)).unwrap();
        assert_eq!(
            service.open_snapshot(client(1), snapshot(1, 2)),
            SemanticAdmissionOutcomeV1::Accepted
        );
        let mut reused = pending.clone();
        reused.target = target(1, 2);
        assert_eq!(
            service.submit(client(1), reused),
            SemanticSubmitOutcomeV1::Refused(SemanticServiceRefusalV1::DuplicateRequest)
        );
        service
            .provider
            .completions
            .push_back(SemanticProviderCompletionV1 {
                request: pending.request,
                outcome: SemanticOutcomeV1::Stale,
            });
        service.pump();
        assert_eq!(service.take_delivery(client(1)).unwrap(), None);
        assert_eq!(service.take_delivery(client(2)).unwrap(), None);
        assert_eq!(service.usage().orphan_completions, 1);
        assert_eq!(service.usage().clients_retired, 1);
        assert_eq!(service.usage().current_clients, 2);
        assert_eq!(service.usage().high_water_clients, 2);
        assert_eq!(service.usage().current_quarantined_requests, 2);
    }

    #[test]
    fn refused_close_fences_client_and_retry_finishes_exact_residual() {
        let mut service = service();
        service.admit_client(client(1)).unwrap();
        service.open_snapshot(client(1), snapshot(1, 1));
        service.provider.close_refusals_remaining = 1;

        assert_eq!(
            service.retire_client(client(1)),
            SemanticClientRetirementReportV1 {
                state: SemanticClientRetirementStateV1::Pending {
                    remaining_documents: 1,
                    quarantined_requests: 0,
                },
                cancellation_signals: 0,
                deliveries_discarded: 0,
                closes_accepted: 0,
                closes_unavailable: 0,
                closes_refused: 1,
            }
        );
        assert_eq!(
            service.admit_client(client(1)),
            Err(SemanticServiceRefusalV1::ClientRetiring)
        );
        assert_eq!(
            service.submit(client(1), request(1, 1, 70)),
            SemanticSubmitOutcomeV1::Refused(SemanticServiceRefusalV1::ClientRetiring)
        );
        assert_eq!(
            service.retire_client(client(1)),
            SemanticClientRetirementReportV1 {
                state: SemanticClientRetirementStateV1::Retired {
                    quarantined_requests: 0,
                },
                cancellation_signals: 0,
                deliveries_discarded: 0,
                closes_accepted: 1,
                closes_unavailable: 0,
                closes_refused: 0,
            }
        );
        assert_eq!(
            service.retire_client(client(1)).state,
            SemanticClientRetirementStateV1::Absent
        );
        assert_eq!(service.usage().client_retirement_attempts, 3);
        assert_eq!(service.usage().client_retirement_pending, 1);
        assert_eq!(service.usage().retirement_closes_refused, 1);
        assert_eq!(service.usage().retirement_closes_accepted, 1);
    }

    #[test]
    fn unavailable_close_retains_source_until_provider_can_confirm_close() {
        let mut service = service();
        service.admit_client(client(1)).unwrap();
        service.open_snapshot(client(1), snapshot(1, 1));
        service.provider.available = false;
        assert_eq!(
            service.retire_client(client(1)).state,
            SemanticClientRetirementStateV1::Pending {
                remaining_documents: 1,
                quarantined_requests: 0,
            }
        );
        assert!(service.snapshot_bytes > 0);
        service.provider.available = true;
        assert!(matches!(
            service.retire_client(client(1)).state,
            SemanticClientRetirementStateV1::Retired { .. }
        ));
        assert_eq!(service.snapshot_bytes, 0);
        assert_eq!(service.usage().retirement_closes_unavailable, 1);
    }

    #[test]
    fn retirement_discards_a_full_delivery_queue_and_reclaims_client_capacity() {
        let limits = SemanticServiceLimitsV1 {
            max_clients: 1,
            max_deliveries: 1,
            ..SemanticServiceLimitsV1::reference()
        };
        let provider = ModelProvider {
            state: ready_state(1),
            available: true,
            ..ModelProvider::default()
        };
        let mut service = SemanticServiceV1::try_new(provider, limits).unwrap();
        service.admit_client(client(1)).unwrap();
        service.open_snapshot(client(1), snapshot(1, 1));
        let offered = request(1, 1, 80);
        service.submit(client(1), offered.clone());
        assert_eq!(service.cancel(client(1), offered.request), Ok(true));
        assert_eq!(
            service.admit_client(client(2)),
            Err(SemanticServiceRefusalV1::Capacity)
        );

        let report = service.retire_client(client(1));
        assert_eq!(report.deliveries_discarded, 1);
        assert_eq!(report.closes_accepted, 1);
        assert!(matches!(
            report.state,
            SemanticClientRetirementStateV1::Retired { .. }
        ));
        service.admit_client(client(2)).unwrap();
        assert_eq!(service.usage().current_clients, 1);
        assert_eq!(service.usage().high_water_clients, 1);
    }
}
