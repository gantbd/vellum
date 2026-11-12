//! Exact ordered bridge between the fresh semantic service and session actor.
//!
//! This mapper is intentionally separate from the legacy embedded runtime. It
//! owns the one service client and the sole provider epoch/projection binding.
//! Translation either closes as one ordered broker report or fails the actor;
//! it never fabricates an ordinary refusal for composition divergence.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    num::NonZeroU64,
    os::unix::ffi::OsStrExt as _,
};

use vellum_adapter_semantic_lsp::{
    LspAdapterLimitObservationV1, LspAdapterProfileV1, LspAdapterV1, LspFreshStartFailureV1,
    LspFreshStartRefusalV1, LspFreshStartedAdapterV1, LspIndexReadinessPortV1,
    LspInitializationProfileV1, LspLimitsV1, LspProviderProtocolProfileV1, LspTransportPortV1,
    LspTransportTerminalV1, LspVirtualDocumentIssuerV1, MAX_LSP_BODY_BYTES_V1,
    MAX_LSP_BUFFER_BYTES_V1, MAX_LSP_HEADER_BYTES_V1, MAX_LSP_HEADERS_V1,
    MAX_LSP_PENDING_REQUESTS_V1, MAX_LSP_URI_BYTES_V1, SemanticGenerationIssuerV1,
};
use vellum_host::semantic::{
    MAX_SEMANTIC_QUERY_BYTES_V1, SemanticCapabilitySetV1, SemanticInvalidationCauseV1,
    SemanticLimitsV1, SemanticRequestRefV1, SemanticRequestV1, SemanticTargetV1,
};
use vellum_semantic_protocol::{
    MAX_SEMANTIC_SESSION_LANGUAGE_BYTES_V1, MAX_SEMANTIC_SESSION_PENDING_V1,
    SemanticProviderObservationV1, SemanticProviderReadinessProjectionV1,
    SemanticSessionIncarnationV1, SemanticSessionLimitsV1, SemanticSessionRefusalV1,
    SemanticSessionSnapshotV1,
};
use vellum_semantic_service::{
    MAX_SEMANTIC_PENDING_V1, MAX_SEMANTIC_SNAPSHOT_BYTES_V1, SemanticBridgeCancelActionV1,
    SemanticBridgeChangeAdmissionV1, SemanticBridgeCloseAdmissionV1, SemanticBridgeExpiryActionV1,
    SemanticBridgeForcedEffectPolicyV1, SemanticBridgeForcedRetiringProviderV1,
    SemanticBridgeOpenAdmissionV1, SemanticBridgeOrderedReportV1, SemanticBridgePollDispositionV1,
    SemanticBridgePollFailureV1, SemanticBridgePollOutcomeV1,
    SemanticBridgeProviderExtractionBlockersV1, SemanticBridgeProviderPollV1,
    SemanticBridgeProviderTransitionV1, SemanticBridgeRetiringProviderV1, SemanticBridgeServiceV1,
    SemanticClientRefV1, SemanticClientRetirementReportV1, SemanticDeliveryOutcomeV1,
    SemanticProviderPortV1, SemanticProviderReadinessV1, SemanticProviderReplacementFailureV1,
    SemanticProviderRetirementV1, SemanticProviderStateV1, SemanticServiceLimitsV1,
    SemanticServiceRefusalV1, SemanticSnapshotV1, SemanticSubmitOutcomeV1,
};

const MAX_LSP_OUTGOING_TEXT_BYTES_V1: usize =
    if MAX_SEMANTIC_SESSION_LANGUAGE_BYTES_V1 > MAX_SEMANTIC_QUERY_BYTES_V1 {
        MAX_SEMANTIC_SESSION_LANGUAGE_BYTES_V1
    } else {
        MAX_SEMANTIC_QUERY_BYTES_V1
    };
const MAX_LSP_OUTGOING_BODY_REQUIRED_V1: usize = MAX_SEMANTIC_SNAPSHOT_BYTES_V1 * 6
    + MAX_LSP_OUTGOING_TEXT_BYTES_V1 * 6
    + MAX_LSP_URI_BYTES_V1
    + 4_096;
const _: () = assert!(MAX_SEMANTIC_SESSION_PENDING_V1 as usize <= MAX_LSP_PENDING_REQUESTS_V1);
const _: () = assert!(MAX_SEMANTIC_PENDING_V1 <= MAX_LSP_PENDING_REQUESTS_V1);
const _: () = assert!(MAX_LSP_OUTGOING_BODY_REQUIRED_V1 <= MAX_LSP_BODY_BYTES_V1);
const _: () = assert!(MAX_LSP_HEADER_BYTES_V1 >= 64);
const _: () = assert!(MAX_LSP_OUTGOING_BODY_REQUIRED_V1 + 64 <= MAX_LSP_BUFFER_BYTES_V1);
use vellum_semantic_session_actor::{
    SemanticBrokerEffectFailureV1, SemanticBrokerEffectObligationsV1, SemanticBrokerEffectPortV1,
    SemanticBrokerProviderDriveV1, SemanticSessionActorBuildFailureV1,
    SemanticSessionActorBuildRefusalV1, SemanticSessionActorControlCompletionV1,
    SemanticSessionActorControlRefusalV1, SemanticSessionActorControlTicketV1,
    SemanticSessionActorEventAdmissionV1, SemanticSessionActorExternalEventV1,
    SemanticSessionActorLimitsV1, SemanticSessionActorOwnerV1, SemanticSessionActorPhaseV1,
    SemanticSessionActorPollV1, SemanticSessionActorRecoveredV1, SemanticSessionActorRecoveryV1,
    SemanticSessionActorSenderV1, SemanticSessionActorStopAbandonmentV1,
    SemanticSessionActorStopDispositionV1, SemanticSessionActorStopReceiptV1,
    SemanticSessionActorStopResolutionV1, SemanticSessionActorStopTicketV1,
    SemanticSessionActorTerminationV1, SemanticSessionActorUsageV1,
    resume_semantic_session_actor_v1, spawn_semantic_session_actor_v1,
};
use vellum_semantic_session_broker::{
    SemanticBrokerAdmissionOutcomeV1, SemanticBrokerCancellationOutcomeV1,
    SemanticBrokerCancellationReportV1, SemanticBrokerDecisionV1, SemanticBrokerDeliveryOutcomeV1,
    SemanticBrokerEventV1, SemanticBrokerExpiryDriveV1, SemanticBrokerExpiryOutcomeV1,
    SemanticBrokerExpiryReportV1, SemanticBrokerOpenReportV1, SemanticBrokerProviderBindingV1,
    SemanticBrokerProviderDriveDispositionV1, SemanticBrokerProviderDriveFailureClassV1,
    SemanticBrokerProviderDriveFailureReportV1, SemanticBrokerProviderDriveReportV1,
    SemanticBrokerProviderTransitionV1, SemanticBrokerSourceAdmissionOutcomeV1,
    SemanticBrokerSourceReportV1, SemanticBrokerSubmissionOutcomeV1,
    SemanticBrokerSubmissionReportV1, SemanticBrokerUsageV1, SemanticSessionBrokerV1,
};

use super::{
    LocalLspFreshTransportFailureV1, LocalLspFreshTransportRefusalV1, LocalLspFreshTransportV1,
    LocalLspTransportV1, LocalSemanticRetirementReceiptV1, LocalSemanticRetirementStateV1,
    QualifiedProviderLaunchV1, SemanticProcessLaunchRefusalV1, SemanticProcessOwnedLaunchFailureV1,
    SemanticProcessUsageV1, SemanticRetirementDriverV1, launch_qualified_local_lsp_owned_v1,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalOrderedSemanticBuildRefusalV1 {
    InvalidLimits,
    InvalidAdapterLimits,
    InvalidProviderState,
    Service(SemanticServiceRefusalV1),
    BrokerGenesis,
}

#[must_use = "failed ordered construction retains provider cleanup authority"]
pub struct LocalOrderedSemanticBuildFailureV1<P> {
    reason: LocalOrderedSemanticBuildRefusalV1,
    residue: LocalOrderedSemanticBuildResidueV1<P>,
}

pub enum LocalOrderedSemanticBuildResidueV1<P> {
    UninstalledProvider(P),
    PreparedPort(Box<LocalOrderedSemanticPortV1<P>>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalSupervisedOrderedLspBuildRefusalV1 {
    InvalidCompositionLimits,
    InvalidInitializationProfile,
    InsufficientTransportFrame,
    Launch(SemanticProcessLaunchRefusalV1),
    TransportFreshness(LocalLspFreshTransportRefusalV1),
    FreshStart(LspFreshStartRefusalV1),
    Ordered(LocalOrderedSemanticBuildRefusalV1),
}

#[must_use = "failed supervised construction retains its remaining affine authority"]
pub enum LocalSupervisedOrderedLspBuildResidueV1<G, R, U> {
    PreLaunch {
        qualified: Box<QualifiedProviderLaunchV1>,
        generations: G,
        readiness: R,
        virtual_documents: U,
    },
    LaunchAttempted {
        failure: SemanticProcessOwnedLaunchFailureV1,
        generations: G,
        readiness: R,
        virtual_documents: U,
    },
    TransportFreshness {
        failure: LocalLspFreshTransportFailureV1,
        generations: G,
        readiness: R,
        virtual_documents: U,
    },
    FreshStart(Box<LspFreshStartFailureV1<LocalLspFreshTransportV1, G, R, U>>),
    Ordered(
        Box<
            LocalOrderedSemanticBuildFailureV1<
                LspFreshStartedAdapterV1<LocalLspFreshTransportV1, G, R, U>,
            >,
        >,
    ),
}

#[must_use = "failed supervised construction retains cleanup or retry authority"]
pub struct LocalSupervisedOrderedLspBuildFailureV1<G, R, U> {
    reason: LocalSupervisedOrderedLspBuildRefusalV1,
    residue: LocalSupervisedOrderedLspBuildResidueV1<G, R, U>,
}

impl<G, R, U> LocalSupervisedOrderedLspBuildFailureV1<G, R, U> {
    pub const fn reason(&self) -> LocalSupervisedOrderedLspBuildRefusalV1 {
        self.reason
    }

    pub fn into_residue(self) -> LocalSupervisedOrderedLspBuildResidueV1<G, R, U> {
        self.residue
    }

    pub fn into_cleanup(self) -> LocalSupervisedOrderedLspCleanupV1<G, R, U>
    where
        G: SemanticGenerationIssuerV1,
        R: LspIndexReadinessPortV1,
        U: LspVirtualDocumentIssuerV1,
    {
        let reason = self.reason;
        match self.residue {
            LocalSupervisedOrderedLspBuildResidueV1::FreshStart(failure) => {
                let origin = LocalOrderedCleanupOriginEvidenceV1 {
                    origin: LocalOrderedCleanupOriginV1::SupervisedBuildRefused(reason),
                    broker_usage: None,
                    actor_usage: None,
                    actor_limits: None,
                    actor_stop: None,
                    actor_recovery_join: LocalOrderedActorRecoveryJoinV1::NotApplicable,
                };
                LocalSupervisedOrderedLspCleanupV1::Provider(Box::new(
                    LocalOrderedProviderCleanupOwnerV1::from_uninstalled_adapter(
                        failure.into_adapter(),
                        origin,
                    ),
                ))
            }
            LocalSupervisedOrderedLspBuildResidueV1::Ordered(failure) => {
                LocalSupervisedOrderedLspCleanupV1::Ordered(failure.into_affine_cleanup_owner())
            }
            LocalSupervisedOrderedLspBuildResidueV1::TransportFreshness {
                failure,
                generations,
                readiness,
                virtual_documents,
            } => LocalSupervisedOrderedLspCleanupV1::Transport(
                LocalSupervisedTransportCleanupOwnerV1::new(
                    failure,
                    generations,
                    readiness,
                    virtual_documents,
                ),
            ),
            residue => LocalSupervisedOrderedLspCleanupV1::Residual(residue),
        }
    }
}

impl<G, R, U> fmt::Debug for LocalSupervisedOrderedLspBuildFailureV1<G, R, U> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalSupervisedOrderedLspBuildFailureV1")
            .field("reason", &self.reason)
            .field("residue", &"<owned>")
            .finish()
    }
}

/// Lossless cleanup routing for supervised construction. Only post-adapter
/// residues enter provider cleanup here; pre-launch retry authority,
/// transport-only freshness refusal, and an unresolved launch remain explicit
/// residual states for their own owners.
#[must_use = "supervised build cleanup routing retains every affine residue"]
pub enum LocalSupervisedOrderedLspCleanupV1<G, R, U> {
    Transport(LocalSupervisedTransportCleanupOwnerV1<G, R, U>),
    Provider(Box<LocalOrderedProviderCleanupOwnerV1<LocalLspFreshTransportV1, G, R, U>>),
    Ordered(LocalOrderedBuildCleanupOwnerV1<LocalLspFreshTransportV1, G, R, U>),
    Residual(LocalSupervisedOrderedLspBuildResidueV1<G, R, U>),
}

impl<G, R, U> fmt::Debug for LocalSupervisedOrderedLspCleanupV1<G, R, U> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Transport(_) => "Transport(<owned>)",
            Self::Provider(_) => "Provider(<owned>)",
            Self::Ordered(_) => "Ordered(<owned>)",
            Self::Residual(_) => "Residual(<owned>)",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalSupervisedTransportCleanupStateV1 {
    Pending,
    ForcedConfirmed,
    CleanupFailed,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LocalSupervisedTransportCleanupUsageV1 {
    pub drives: u64,
    pub abort_requests: u64,
    pub forced_confirmed: u64,
    pub cleanup_failed: u64,
    pub saturated: bool,
}

/// Forced-only owner for a process transport rejected before adapter
/// construction. No provider or bridge existed, so this type carries neither
/// provider retirement nor abandonment evidence.
#[must_use = "transport cleanup must reach terminal evidence"]
pub struct LocalSupervisedTransportCleanupOwnerV1<G, R, U> {
    transport: LocalLspTransportV1,
    generations: G,
    readiness: R,
    virtual_documents: U,
    state: LocalSupervisedTransportCleanupStateV1,
    usage: LocalSupervisedTransportCleanupUsageV1,
}

impl<G, R, U> LocalSupervisedTransportCleanupOwnerV1<G, R, U> {
    pub(crate) fn new(
        failure: LocalLspFreshTransportFailureV1,
        generations: G,
        readiness: R,
        virtual_documents: U,
    ) -> Self {
        let transport = failure.into_transport();
        transport.abort();
        Self {
            transport,
            generations,
            readiness,
            virtual_documents,
            state: LocalSupervisedTransportCleanupStateV1::Pending,
            usage: LocalSupervisedTransportCleanupUsageV1 {
                abort_requests: 1,
                ..LocalSupervisedTransportCleanupUsageV1::default()
            },
        }
    }

    pub const fn state(&self) -> LocalSupervisedTransportCleanupStateV1 {
        self.state
    }

    pub const fn usage(&self) -> LocalSupervisedTransportCleanupUsageV1 {
        self.usage
    }

    pub fn drive_once(&mut self) -> LocalSupervisedTransportCleanupStateV1 {
        if self.state != LocalSupervisedTransportCleanupStateV1::Pending {
            return self.state;
        }
        cleanup_bump(&mut self.usage.drives, &mut self.usage.saturated);
        self.state = match self.transport.terminal_state() {
            LspTransportTerminalV1::ForcedClosed => {
                cleanup_bump(&mut self.usage.forced_confirmed, &mut self.usage.saturated);
                LocalSupervisedTransportCleanupStateV1::ForcedConfirmed
            }
            LspTransportTerminalV1::GracefulClosed | LspTransportTerminalV1::Failed => {
                cleanup_bump(&mut self.usage.cleanup_failed, &mut self.usage.saturated);
                LocalSupervisedTransportCleanupStateV1::CleanupFailed
            }
            LspTransportTerminalV1::Active | LspTransportTerminalV1::Unobserved => {
                LocalSupervisedTransportCleanupStateV1::Pending
            }
        };
        self.state
    }

    pub fn try_into_terminal_receipt(
        mut self,
    ) -> Result<LocalSupervisedTransportCleanupReceiptV1<G, R, U>, Box<Self>> {
        if self.state == LocalSupervisedTransportCleanupStateV1::Pending {
            let _ = self.drive_once();
        }
        if self.state == LocalSupervisedTransportCleanupStateV1::Pending {
            return Err(Box::new(self));
        }
        Ok(LocalSupervisedTransportCleanupReceiptV1 {
            state: self.state,
            transport_terminal: self.transport.terminal_state(),
            process_usage: self.transport.usage(),
            cleanup_usage: self.usage,
            generations: self.generations,
            readiness: self.readiness,
            virtual_documents: self.virtual_documents,
        })
    }
}

impl<G, R, U> fmt::Debug for LocalSupervisedTransportCleanupOwnerV1<G, R, U> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalSupervisedTransportCleanupOwnerV1")
            .field("state", &self.state)
            .field("usage", &self.usage)
            .field("transport", &"<owned>")
            .field("collaborators", &"<owned>")
            .finish()
    }
}

#[must_use = "transport terminal evidence and collaborators must be retained"]
pub struct LocalSupervisedTransportCleanupReceiptV1<G, R, U> {
    state: LocalSupervisedTransportCleanupStateV1,
    transport_terminal: LspTransportTerminalV1,
    process_usage: SemanticProcessUsageV1,
    cleanup_usage: LocalSupervisedTransportCleanupUsageV1,
    generations: G,
    readiness: R,
    virtual_documents: U,
}

impl<G, R, U> LocalSupervisedTransportCleanupReceiptV1<G, R, U> {
    pub const fn state(&self) -> LocalSupervisedTransportCleanupStateV1 {
        self.state
    }

    pub const fn transport_terminal(&self) -> LspTransportTerminalV1 {
        self.transport_terminal
    }

    pub const fn process_usage(&self) -> SemanticProcessUsageV1 {
        self.process_usage
    }

    pub const fn cleanup_usage(&self) -> LocalSupervisedTransportCleanupUsageV1 {
        self.cleanup_usage
    }
}

impl<G, R, U> fmt::Debug for LocalSupervisedTransportCleanupReceiptV1<G, R, U> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _owned_collaborators = (&self.generations, &self.readiness, &self.virtual_documents);
        formatter
            .debug_struct("LocalSupervisedTransportCleanupReceiptV1")
            .field("state", &self.state)
            .field("transport_terminal", &self.transport_terminal)
            .field("process_usage", &self.process_usage)
            .field("cleanup_usage", &self.cleanup_usage)
            .field("collaborators", &"<owned>")
            .finish()
    }
}

fn cleanup_bump(counter: &mut u64, saturated: &mut bool) {
    if let Some(next) = counter.checked_add(1) {
        *counter = next;
    } else {
        *saturated = true;
    }
}

pub type LocalSupervisedOrderedLspPreparedV1<G, R, U> =
    LocalOrderedSemanticPreparedV1<LspFreshStartedAdapterV1<LocalLspFreshTransportV1, G, R, U>>;

pub type LocalSupervisedOrderedLspBuildResultV1<G, R, U> = Result<
    LocalSupervisedOrderedLspPreparedV1<G, R, U>,
    Box<LocalSupervisedOrderedLspBuildFailureV1<G, R, U>>,
>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalSupervisedOrderedTimedLspBuildRefusalV1 {
    Base(LocalSupervisedOrderedLspBuildRefusalV1),
    Deadlines(LocalOrderedDeadlineBuildRefusalV1),
}

#[must_use = "failed timed supervised construction retains cleanup and clock authority"]
pub enum LocalSupervisedOrderedTimedLspBuildResidueV1<G, R, U, C> {
    PreLaunch {
        qualified: Box<QualifiedProviderLaunchV1>,
        generations: G,
        readiness: R,
        virtual_documents: U,
        clock: C,
    },
    Base {
        failure: Box<LocalSupervisedOrderedLspBuildFailureV1<G, R, U>>,
        clock: C,
    },
    Deadlines(
        Box<
            LocalOrderedDeadlineBuildFailureV1<
                LspFreshStartedAdapterV1<LocalLspFreshTransportV1, G, R, U>,
                C,
            >,
        >,
    ),
}

#[must_use = "failed timed supervised construction retains cleanup and clock authority"]
pub struct LocalSupervisedOrderedTimedLspBuildFailureV1<G, R, U, C> {
    reason: LocalSupervisedOrderedTimedLspBuildRefusalV1,
    residue: LocalSupervisedOrderedTimedLspBuildResidueV1<G, R, U, C>,
}

impl<G, R, U, C> LocalSupervisedOrderedTimedLspBuildFailureV1<G, R, U, C> {
    pub const fn reason(&self) -> LocalSupervisedOrderedTimedLspBuildRefusalV1 {
        self.reason
    }

    pub fn into_residue(self) -> LocalSupervisedOrderedTimedLspBuildResidueV1<G, R, U, C> {
        self.residue
    }
}

impl<G, R, U, C> fmt::Debug for LocalSupervisedOrderedTimedLspBuildFailureV1<G, R, U, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalSupervisedOrderedTimedLspBuildFailureV1")
            .field("reason", &self.reason)
            .field("residue", &"<owned>")
            .finish()
    }
}

pub type LocalSupervisedOrderedTimedLspPreparedV1<G, R, U, C> = LocalOrderedTimedSemanticPreparedV1<
    LspFreshStartedAdapterV1<LocalLspFreshTransportV1, G, R, U>,
    C,
>;

pub type LocalSupervisedOrderedTimedLspBuildResultV1<G, R, U, C> = Result<
    LocalSupervisedOrderedTimedLspPreparedV1<G, R, U, C>,
    Box<LocalSupervisedOrderedTimedLspBuildFailureV1<G, R, U, C>>,
>;

impl<P> LocalOrderedSemanticBuildFailureV1<P> {
    pub const fn reason(&self) -> LocalOrderedSemanticBuildRefusalV1 {
        self.reason
    }

    pub fn into_residue(self) -> LocalOrderedSemanticBuildResidueV1<P> {
        self.residue
    }

    pub fn into_cleanup_origin(
        self,
    ) -> (
        LocalOrderedSemanticBuildCleanupV1<P>,
        LocalOrderedCleanupOriginEvidenceV1,
    ) {
        let evidence = LocalOrderedCleanupOriginEvidenceV1 {
            origin: LocalOrderedCleanupOriginV1::OrderedBuildRefused(self.reason),
            broker_usage: None,
            actor_usage: None,
            actor_limits: None,
            actor_stop: None,
            actor_recovery_join: LocalOrderedActorRecoveryJoinV1::NotApplicable,
        };
        let cleanup = match self.residue {
            LocalOrderedSemanticBuildResidueV1::UninstalledProvider(provider) => {
                LocalOrderedSemanticBuildCleanupV1::Uninstalled(provider)
            }
            LocalOrderedSemanticBuildResidueV1::PreparedPort(port) => {
                LocalOrderedSemanticBuildCleanupV1::Installed(Box::new(
                    LocalOrderedSemanticCleanupV1 { port: *port },
                ))
            }
        };
        (cleanup, evidence)
    }
}

impl<P> fmt::Debug for LocalOrderedSemanticBuildFailureV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedSemanticBuildFailureV1")
            .field("reason", &self.reason)
            .field("residue", &"<owned>")
            .finish()
    }
}

/// Fresh single-client bridge and the only allocator for its composition epoch.
#[must_use = "the ordered semantic bridge owns live provider and client obligations"]
pub struct LocalOrderedSemanticPortV1<P> {
    service: SemanticBridgeServiceV1<P>,
    client: SemanticClientRefV1,
    binding: SemanticBrokerProviderBindingV1,
    service_limits: SemanticServiceLimitsV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalOrderedProviderReplacementRefusalV1 {
    EpochExhausted,
    InvalidCandidateProjection,
    SchedulerPoisoned,
}

#[must_use = "failed ordered replacement retains the exact candidate"]
pub enum LocalOrderedProviderReplacementFailureV1<P> {
    Preflight {
        reason: LocalOrderedProviderReplacementRefusalV1,
        candidate: P,
    },
    Service(SemanticProviderReplacementFailureV1<P>),
}

impl<P> LocalOrderedProviderReplacementFailureV1<P> {
    pub const fn reason(&self) -> Option<LocalOrderedProviderReplacementRefusalV1> {
        match self {
            Self::Preflight { reason, .. } => Some(*reason),
            Self::Service(_) => None,
        }
    }

    pub fn into_candidate(self) -> P {
        match self {
            Self::Preflight { candidate, .. } => candidate,
            Self::Service(failure) => failure.into_candidate(),
        }
    }
}

impl<P> fmt::Debug for LocalOrderedProviderReplacementFailureV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Preflight { reason, .. } => formatter
                .debug_struct("Preflight")
                .field("reason", reason)
                .field("candidate", &"<owned>")
                .finish(),
            Self::Service(failure) => formatter.debug_tuple("Service").field(failure).finish(),
        }
    }
}

#[must_use = "replacement report and displaced-provider retirement must both be reconciled"]
pub struct LocalOrderedProviderReplacementV1<P> {
    pub retirement: SemanticProviderRetirementV1<P>,
    pub report: SemanticBrokerProviderDriveReportV1,
}

/// Pre-exposure broker/port product created from one genesis provider binding.
/// Neither half can be extracted, preventing a caller from pairing the port
/// with a broker constructed from a different projection or epoch.
#[must_use = "the prepared semantic actor runtime must be spawned or retired"]
pub struct LocalOrderedSemanticPreparedV1<P> {
    broker: SemanticSessionBrokerV1,
    port: LocalOrderedSemanticPortV1<P>,
}

pub type LocalOrderedLspAdapterV1<T, G, R, U> = LspFreshStartedAdapterV1<T, G, R, U>;
pub type LocalOrderedLspBuildResultV1<T, G, R, U> = Result<
    LocalOrderedSemanticPreparedV1<LocalOrderedLspAdapterV1<T, G, R, U>>,
    Box<LocalOrderedSemanticBuildFailureV1<LocalOrderedLspAdapterV1<T, G, R, U>>>,
>;

pub const MAX_LOCAL_ORDERED_DEADLINES_V1: usize = MAX_SEMANTIC_PENDING_V1;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LocalOrderedInstantV1(u64);

impl LocalOrderedInstantV1 {
    pub const fn from_composition_tick(tick: u64) -> Self {
        Self(tick)
    }

    const fn checked_add(self, delta: NonZeroU64) -> Option<Self> {
        match self.0.checked_add(delta.get()) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalOrderedClockFailureV1 {
    Unavailable,
}

pub trait LocalOrderedMonotonicClockV1: Send + 'static {
    fn sample(&mut self) -> Result<LocalOrderedInstantV1, LocalOrderedClockFailureV1>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalOrderedDeadlinePolicyV1 {
    pub relative_budget_ticks: NonZeroU64,
    pub max_deadlines: usize,
}

impl LocalOrderedDeadlinePolicyV1 {
    pub const fn is_valid(self) -> bool {
        self.max_deadlines > 0 && self.max_deadlines <= MAX_LOCAL_ORDERED_DEADLINES_V1
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LocalOrderedDeadlineUsageV1 {
    pub scheduled: u64,
    pub current: usize,
    pub high_water: usize,
    pub clock_samples: u64,
    pub clock_unavailable: u64,
    pub clock_regressions: u64,
    pub clock_overflow: u64,
    pub capacity_refusals: u64,
    pub expiry_drives: u64,
    pub no_due: u64,
    pub expired: u64,
    pub transition_satisfied: u64,
    pub not_pending: u64,
    pub clears_by_cause: [u64; 5],
    pub retirement_discards: u64,
    pub saturated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalOrderedDeadlineBuildRefusalV1 {
    InvalidPolicy,
    InsufficientCapacity,
}

#[must_use = "failed timed construction retains the prepared runtime and clock"]
pub struct LocalOrderedDeadlineBuildFailureV1<P, C> {
    reason: LocalOrderedDeadlineBuildRefusalV1,
    prepared: LocalOrderedSemanticPreparedV1<P>,
    clock: C,
}

impl<P, C> LocalOrderedDeadlineBuildFailureV1<P, C> {
    pub const fn reason(&self) -> LocalOrderedDeadlineBuildRefusalV1 {
        self.reason
    }

    pub fn into_parts(self) -> (LocalOrderedSemanticPreparedV1<P>, C) {
        (self.prepared, self.clock)
    }
}

impl<P, C> fmt::Debug for LocalOrderedDeadlineBuildFailureV1<P, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedDeadlineBuildFailureV1")
            .field("reason", &self.reason)
            .field("prepared", &"<owned>")
            .field("clock", &"<owned>")
            .finish()
    }
}

struct LocalOrderedDeadlineSchedulerV1<C> {
    clock: C,
    policy: LocalOrderedDeadlinePolicyV1,
    by_request: BTreeMap<SemanticRequestRefV1, LocalOrderedInstantV1>,
    by_due: BTreeSet<(LocalOrderedInstantV1, SemanticRequestRefV1)>,
    last_now: Option<LocalOrderedInstantV1>,
    usage: LocalOrderedDeadlineUsageV1,
    poisoned: bool,
}

#[derive(Debug)]
struct LocalOrderedDeadlineInstallV1 {
    request: SemanticRequestRefV1,
    due: LocalOrderedInstantV1,
}

#[must_use = "the timed ordered runtime must be spawned or retired"]
pub struct LocalOrderedTimedSemanticPreparedV1<P, C> {
    broker: SemanticSessionBrokerV1,
    port: LocalOrderedTimedSemanticPortV1<P, C>,
}

#[must_use = "the timed ordered port owns live provider and deadline obligations"]
pub struct LocalOrderedTimedSemanticPortV1<P, C> {
    base: LocalOrderedSemanticPortV1<P>,
    scheduler: LocalOrderedDeadlineSchedulerV1<C>,
}

#[must_use = "timed cleanup must retire scheduler obligations before provider extraction"]
pub struct LocalOrderedTimedSemanticCleanupV1<P, C> {
    port: LocalOrderedTimedSemanticPortV1<P, C>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalOrderedDeadlineDispositionV1 {
    Disabled,
    Retired,
    Abandoned,
    Unresolved,
}

/// Closed, content-free origin of one ordered cleanup obligation. The origin
/// is evidence about how ownership returned; it is not a successful cleanup
/// claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalOrderedCleanupOriginV1 {
    Unstarted,
    SupervisedBuildRefused(LocalSupervisedOrderedLspBuildRefusalV1),
    OrderedBuildRefused(LocalOrderedSemanticBuildRefusalV1),
    ActorBuildRefused(SemanticSessionActorBuildRefusalV1),
    ActorRecovered(SemanticSessionActorTerminationV1),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalOrderedActorTerminalStopV1 {
    Clean(SemanticSessionActorStopReceiptV1),
    OutcomeUnknown(SemanticSessionActorStopReceiptV1),
    Abandoned(SemanticSessionActorStopAbandonmentV1),
    FailedBeforeStop { usage: SemanticSessionActorUsageV1 },
}

/// Whether actor stop evidence and recovered termination were joined, and
/// whether their affirmative claims agreed. This disposition is retained all
/// the way to provider/process terminal evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalOrderedActorRecoveryJoinV1 {
    NotApplicable,
    Compatible,
    Mismatch,
}

/// Same-spawn stop evidence retained by the managed actor/recovery product.
/// A refusal is nonterminal: only its bounded history is retained while the
/// exact actor owner remains available for another retirement attempt.
#[must_use = "actor stop evidence must remain joined with terminal cleanup"]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalOrderedActorStopEvidenceV1 {
    terminal: LocalOrderedActorTerminalStopV1,
    refusal_attempts: u64,
    last_refusal: Option<SemanticSessionActorStopReceiptV1>,
    saturated: bool,
}

impl LocalOrderedActorStopEvidenceV1 {
    pub const fn terminal(&self) -> LocalOrderedActorTerminalStopV1 {
        self.terminal
    }

    pub const fn refusal_attempts(&self) -> u64 {
        self.refusal_attempts
    }

    pub const fn last_refusal(&self) -> Option<SemanticSessionActorStopReceiptV1> {
        self.last_refusal
    }

    pub const fn saturated(&self) -> bool {
        self.saturated
    }
}

const fn actor_stop_recovery_compatible(
    stop: LocalOrderedActorTerminalStopV1,
    termination: SemanticSessionActorTerminationV1,
) -> bool {
    match stop {
        LocalOrderedActorTerminalStopV1::Clean(_) => {
            matches!(termination, SemanticSessionActorTerminationV1::Clean)
        }
        LocalOrderedActorTerminalStopV1::FailedBeforeStop { .. } => {
            matches!(termination, SemanticSessionActorTerminationV1::Failed)
        }
        LocalOrderedActorTerminalStopV1::OutcomeUnknown(_)
        | LocalOrderedActorTerminalStopV1::Abandoned(_) => true,
    }
}

/// Fixed-cardinality evidence captured before the broker is consumed. This
/// prevents cleanup callers from having to split and silently discard the
/// broker merely to recover the effect port.
#[must_use = "ordered cleanup origin evidence must be joined with terminal retirement"]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalOrderedCleanupOriginEvidenceV1 {
    origin: LocalOrderedCleanupOriginV1,
    broker_usage: Option<SemanticBrokerUsageV1>,
    actor_usage: Option<SemanticSessionActorUsageV1>,
    actor_limits: Option<SemanticSessionActorLimitsV1>,
    actor_stop: Option<LocalOrderedActorStopEvidenceV1>,
    actor_recovery_join: LocalOrderedActorRecoveryJoinV1,
}

impl LocalOrderedCleanupOriginEvidenceV1 {
    pub const fn origin(&self) -> LocalOrderedCleanupOriginV1 {
        self.origin
    }

    pub const fn broker_usage(&self) -> Option<SemanticBrokerUsageV1> {
        self.broker_usage
    }

    pub const fn actor_usage(&self) -> Option<SemanticSessionActorUsageV1> {
        self.actor_usage
    }

    pub const fn actor_limits(&self) -> Option<SemanticSessionActorLimitsV1> {
        self.actor_limits
    }

    pub const fn actor_stop(&self) -> Option<LocalOrderedActorStopEvidenceV1> {
        self.actor_stop
    }

    pub const fn actor_recovery_join(&self) -> LocalOrderedActorRecoveryJoinV1 {
        self.actor_recovery_join
    }
}

/// Normalized ordered-construction residue. A pre-service provider remains
/// explicitly uninstalled; a post-service refusal advances only to cleanup.
#[must_use = "ordered build residue retains provider cleanup authority"]
pub enum LocalOrderedSemanticBuildCleanupV1<P> {
    Uninstalled(P),
    Installed(Box<LocalOrderedSemanticCleanupV1<P>>),
}

impl<P> fmt::Debug for LocalOrderedSemanticBuildCleanupV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Uninstalled(_) => "Uninstalled(<owned>)",
            Self::Installed(_) => "Installed(<owned>)",
        })
    }
}

#[must_use = "deadline retirement evidence must be reconciled"]
pub struct LocalOrderedDeadlineRetirementReceiptV1 {
    disposition: LocalOrderedDeadlineDispositionV1,
    terminal_clears: u64,
    discarded_live: Vec<SemanticRequestRefV1>,
    unresolved: Vec<SemanticRequestRefV1>,
    retirement_discards: u64,
    saturated: bool,
}

#[must_use = "unresolved deadline indexes retain the entire timed cleanup obligation"]
pub struct LocalOrderedDeadlineRetirementFailureV1<P, C> {
    cleanup: LocalOrderedTimedSemanticCleanupV1<P, C>,
    receipt: LocalOrderedDeadlineRetirementReceiptV1,
}

impl<P, C> LocalOrderedDeadlineRetirementFailureV1<P, C> {
    pub const fn receipt(&self) -> &LocalOrderedDeadlineRetirementReceiptV1 {
        &self.receipt
    }

    pub fn into_cleanup(self) -> LocalOrderedTimedSemanticCleanupV1<P, C> {
        self.cleanup
    }
}

impl<P, C> fmt::Debug for LocalOrderedDeadlineRetirementFailureV1<P, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedDeadlineRetirementFailureV1")
            .field("receipt", &self.receipt)
            .field("cleanup", &"<owned>")
            .finish()
    }
}

pub type LocalOrderedDeadlineRetirementSuccessV1<P> = (
    LocalOrderedSemanticCleanupV1<P>,
    LocalOrderedDeadlineRetirementReceiptV1,
);

pub type LocalOrderedDeadlineRetirementResultV1<P, C> = Result<
    LocalOrderedDeadlineRetirementSuccessV1<P>,
    Box<LocalOrderedDeadlineRetirementFailureV1<P, C>>,
>;

impl LocalOrderedDeadlineRetirementReceiptV1 {
    pub const fn disposition(&self) -> LocalOrderedDeadlineDispositionV1 {
        self.disposition
    }

    pub const fn terminal_clears(&self) -> u64 {
        self.terminal_clears
    }

    pub fn discarded_live(&self) -> impl ExactSizeIterator<Item = SemanticRequestRefV1> + '_ {
        self.discarded_live.iter().copied()
    }

    pub fn unresolved(&self) -> impl ExactSizeIterator<Item = SemanticRequestRefV1> + '_ {
        self.unresolved.iter().copied()
    }

    pub const fn saturated(&self) -> bool {
        self.saturated
    }

    pub const fn retirement_discards(&self) -> u64 {
        self.retirement_discards
    }
}

impl fmt::Debug for LocalOrderedDeadlineRetirementReceiptV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedDeadlineRetirementReceiptV1")
            .field("disposition", &self.disposition)
            .field("terminal_clears", &self.terminal_clears)
            .field("discarded_live", &self.discarded_live.len())
            .field("unresolved", &self.unresolved.len())
            .field("retirement_discards", &self.retirement_discards)
            .field("saturated", &self.saturated)
            .finish()
    }
}

#[must_use = "unstarted or recovered ordered runtime cleanup must reach provider retirement"]
pub struct LocalOrderedSemanticCleanupV1<P> {
    port: LocalOrderedSemanticPortV1<P>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LocalOrderedClientRetirementUsageV1 {
    pub passes: u64,
    pub cancellation_signals: u64,
    pub deliveries_discarded: u64,
    pub closes_accepted: u64,
    pub closes_unavailable: u64,
    pub closes_refused: u64,
    pub saturated: bool,
}

impl LocalOrderedClientRetirementUsageV1 {
    fn observe(&mut self, report: SemanticClientRetirementReportV1) {
        cleanup_add(&mut self.passes, 1, &mut self.saturated);
        cleanup_add(
            &mut self.cancellation_signals,
            report.cancellation_signals,
            &mut self.saturated,
        );
        cleanup_add(
            &mut self.deliveries_discarded,
            report.deliveries_discarded,
            &mut self.saturated,
        );
        cleanup_add(
            &mut self.closes_accepted,
            report.closes_accepted,
            &mut self.saturated,
        );
        cleanup_add(
            &mut self.closes_unavailable,
            report.closes_unavailable,
            &mut self.saturated,
        );
        cleanup_add(
            &mut self.closes_refused,
            report.closes_refused,
            &mut self.saturated,
        );
    }
}

fn cleanup_add(target: &mut u64, value: usize, saturated: &mut bool) {
    let value = u64::try_from(value).unwrap_or(u64::MAX);
    let before = *target;
    *target = before.saturating_add(value);
    *saturated |= before.checked_add(value).is_none();
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalOrderedProviderExtractionV1 {
    Checked,
    Forced,
    Uninstalled,
}

/// One affine owner joins origin, deadline and client-retirement evidence with
/// the still-live ordered bridge. It exposes no broker effect port or provider.
#[must_use = "ordered cleanup must advance to terminal provider evidence"]
pub struct LocalOrderedCleanupOwnerV1<P> {
    cleanup: LocalOrderedSemanticCleanupV1<P>,
    origin: LocalOrderedCleanupOriginEvidenceV1,
    deadlines: LocalOrderedDeadlineRetirementReceiptV1,
    client_usage: LocalOrderedClientRetirementUsageV1,
}

/// Timed ingress retains scheduler ownership and any unresolved observation
/// until it either retires lawfully or is explicitly abandoned.
#[must_use = "timed cleanup must reconcile or abandon its deadline graph"]
pub struct LocalOrderedTimedCleanupOwnerV1<P, C> {
    cleanup: LocalOrderedTimedSemanticCleanupV1<P, C>,
    origin: LocalOrderedCleanupOriginEvidenceV1,
    unresolved: Option<LocalOrderedDeadlineRetirementReceiptV1>,
}

impl<P, C> fmt::Debug for LocalOrderedTimedCleanupOwnerV1<P, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedTimedCleanupOwnerV1")
            .field("origin", &self.origin)
            .field("unresolved", &self.unresolved)
            .field("cleanup", &"<owned>")
            .finish()
    }
}

impl<P> fmt::Debug for LocalOrderedCleanupOwnerV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedCleanupOwnerV1")
            .field("origin", &self.origin)
            .field("deadlines", &self.deadlines)
            .field("client_usage", &self.client_usage)
            .field("cleanup", &"<owned>")
            .finish()
    }
}

#[must_use = "blocked checked extraction retains the complete cleanup owner"]
pub struct LocalOrderedCleanupBlockedV1<P> {
    owner: LocalOrderedCleanupOwnerV1<P>,
    blockers: SemanticBridgeProviderExtractionBlockersV1,
}

pub type LocalOrderedProviderCleanupResultV1<T, G, R, U> = Result<
    LocalOrderedProviderCleanupOwnerV1<T, G, R, U>,
    Box<LocalOrderedCleanupBlockedV1<LspFreshStartedAdapterV1<T, G, R, U>>>,
>;

impl<P> LocalOrderedCleanupBlockedV1<P> {
    pub const fn blockers(&self) -> SemanticBridgeProviderExtractionBlockersV1 {
        self.blockers
    }

    pub fn into_owner(self) -> LocalOrderedCleanupOwnerV1<P> {
        self.owner
    }
}

impl<P> fmt::Debug for LocalOrderedCleanupBlockedV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedCleanupBlockedV1")
            .field("blockers", &self.blockers)
            .field("owner", &"<owned>")
            .finish()
    }
}

/// Provider-terminal half of the same affine cleanup path.
#[must_use = "provider cleanup must reach a terminal receipt"]
pub struct LocalOrderedProviderCleanupOwnerV1<T: LspTransportPortV1, G, R, U> {
    origin: LocalOrderedCleanupOriginEvidenceV1,
    deadlines: LocalOrderedDeadlineRetirementReceiptV1,
    client_usage: LocalOrderedClientRetirementUsageV1,
    extraction: LocalOrderedProviderExtractionV1,
    driver: SemanticRetirementDriverV1<T, G, R, U>,
}

#[must_use = "ordered build cleanup must reach one terminal receipt"]
pub enum LocalOrderedBuildCleanupOwnerV1<T: LspTransportPortV1, G, R, U> {
    Bridge(Box<LocalOrderedCleanupOwnerV1<LspFreshStartedAdapterV1<T, G, R, U>>>),
    Uninstalled(Box<LocalOrderedProviderCleanupOwnerV1<T, G, R, U>>),
}

impl<T: LspTransportPortV1, G, R, U> fmt::Debug for LocalOrderedBuildCleanupOwnerV1<T, G, R, U> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Bridge(_) => "Bridge(<owned>)",
            Self::Uninstalled(_) => "Uninstalled(<owned>)",
        })
    }
}

impl<T: LspTransportPortV1, G, R, U> fmt::Debug for LocalOrderedProviderCleanupOwnerV1<T, G, R, U> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedProviderCleanupOwnerV1")
            .field("origin", &self.origin)
            .field("deadlines", &self.deadlines)
            .field("client_usage", &self.client_usage)
            .field("extraction", &self.extraction)
            .field("driver", &"<owned>")
            .finish()
    }
}

impl<T, G, R, U> LocalOrderedProviderCleanupOwnerV1<T, G, R, U>
where
    T: LspTransportPortV1,
    G: SemanticGenerationIssuerV1,
    R: LspIndexReadinessPortV1,
    U: LspVirtualDocumentIssuerV1,
{
    pub fn from_uninstalled_adapter(
        adapter: LspAdapterV1<T, G, R, U>,
        origin: LocalOrderedCleanupOriginEvidenceV1,
    ) -> Self {
        Self {
            origin,
            deadlines: disabled_deadline_receipt(),
            client_usage: LocalOrderedClientRetirementUsageV1::default(),
            extraction: LocalOrderedProviderExtractionV1::Uninstalled,
            driver: SemanticRetirementDriverV1::from_uninstalled_adapter(adapter),
        }
    }
}

#[must_use = "aggregate cleanup evidence must be retained"]
pub struct LocalOrderedTerminalCleanupReceiptV1 {
    origin: LocalOrderedCleanupOriginEvidenceV1,
    deadlines: LocalOrderedDeadlineRetirementReceiptV1,
    client_usage: LocalOrderedClientRetirementUsageV1,
    extraction: LocalOrderedProviderExtractionV1,
    provider: LocalSemanticRetirementReceiptV1,
}

impl LocalOrderedTerminalCleanupReceiptV1 {
    pub const fn origin(&self) -> LocalOrderedCleanupOriginEvidenceV1 {
        self.origin
    }

    pub const fn deadlines(&self) -> &LocalOrderedDeadlineRetirementReceiptV1 {
        &self.deadlines
    }

    pub const fn client_usage(&self) -> LocalOrderedClientRetirementUsageV1 {
        self.client_usage
    }

    pub const fn extraction(&self) -> LocalOrderedProviderExtractionV1 {
        self.extraction
    }

    pub const fn provider(&self) -> &LocalSemanticRetirementReceiptV1 {
        &self.provider
    }
}

impl fmt::Debug for LocalOrderedTerminalCleanupReceiptV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedTerminalCleanupReceiptV1")
            .field("origin", &self.origin)
            .field("deadlines", &self.deadlines)
            .field("client_usage", &self.client_usage)
            .field("extraction", &self.extraction)
            .field("provider", &self.provider)
            .finish()
    }
}

impl<P> fmt::Debug for LocalOrderedSemanticCleanupV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LocalOrderedSemanticCleanupV1(<opaque>)")
    }
}

impl<P: SemanticProviderPortV1> LocalOrderedSemanticCleanupV1<P> {
    /// Normalizes actor-construction refusal without exposing or discarding
    /// either half of the coordinated broker/port product.
    pub(crate) fn from_actor_build_failure(
        failure: SemanticSessionActorBuildFailureV1<LocalOrderedSemanticPortV1<P>>,
    ) -> (Self, LocalOrderedCleanupOriginEvidenceV1) {
        let (reason, limits, broker, port) = failure.into_parts();
        let evidence = LocalOrderedCleanupOriginEvidenceV1 {
            origin: LocalOrderedCleanupOriginV1::ActorBuildRefused(reason),
            broker_usage: Some(broker.usage()),
            actor_usage: None,
            actor_limits: Some(limits),
            actor_stop: None,
            actor_recovery_join: LocalOrderedActorRecoveryJoinV1::NotApplicable,
        };
        drop(broker);
        (Self { port }, evidence)
    }

    /// Normalizes terminal actor recovery while retaining the exact actor and
    /// broker observations that explain the cleanup path.
    pub(crate) fn from_actor_recovered(
        recovered: SemanticSessionActorRecoveredV1<LocalOrderedSemanticPortV1<P>>,
    ) -> (Self, LocalOrderedCleanupOriginEvidenceV1) {
        Self::from_actor_recovered_with_stop(
            recovered,
            None,
            LocalOrderedActorRecoveryJoinV1::NotApplicable,
        )
    }

    fn from_actor_recovered_with_stop(
        recovered: SemanticSessionActorRecoveredV1<LocalOrderedSemanticPortV1<P>>,
        actor_stop: Option<LocalOrderedActorStopEvidenceV1>,
        actor_recovery_join: LocalOrderedActorRecoveryJoinV1,
    ) -> (Self, LocalOrderedCleanupOriginEvidenceV1) {
        let (termination, actor_usage, broker, port) = recovered.into_parts();
        let evidence = LocalOrderedCleanupOriginEvidenceV1 {
            origin: LocalOrderedCleanupOriginV1::ActorRecovered(termination),
            broker_usage: Some(broker.usage()),
            actor_usage: Some(actor_usage),
            actor_limits: None,
            actor_stop,
            actor_recovery_join,
        };
        drop(broker);
        (Self { port }, evidence)
    }

    #[cfg(test)]
    pub(crate) fn from_recovered_port(port: LocalOrderedSemanticPortV1<P>) -> Self {
        Self { port }
    }

    pub(crate) fn retire_client(&mut self) -> SemanticClientRetirementReportV1 {
        self.port.retire_client()
    }

    pub(crate) fn try_into_retiring_provider(
        self,
    ) -> Result<SemanticBridgeRetiringProviderV1<P>, Box<LocalOrderedSemanticExtractionFailureV1<P>>>
    {
        self.port.try_into_retiring_provider()
    }

    /// Explicitly abandons any remaining bridge state and returns a
    /// forced-only provider capability. This transition can never be confused
    /// with checked extraction.
    pub(crate) fn force_into_retiring_provider(
        self,
        policy: SemanticBridgeForcedEffectPolicyV1,
    ) -> SemanticBridgeForcedRetiringProviderV1<P> {
        self.port.force_into_retiring_provider(policy)
    }

    pub(crate) fn confirm_deadlines_disabled(&self) -> LocalOrderedDeadlineRetirementReceiptV1 {
        disabled_deadline_receipt()
    }
}

impl<P: SemanticProviderPortV1> LocalOrderedCleanupOwnerV1<P> {
    pub fn from_unstarted(prepared: LocalOrderedSemanticPreparedV1<P>) -> Self {
        let (cleanup, origin) = prepared.into_cleanup_with_evidence();
        let deadlines = cleanup.confirm_deadlines_disabled();
        Self {
            cleanup,
            origin,
            deadlines,
            client_usage: LocalOrderedClientRetirementUsageV1::default(),
        }
    }

    pub fn from_actor_build_failure(
        failure: SemanticSessionActorBuildFailureV1<LocalOrderedSemanticPortV1<P>>,
    ) -> Self {
        let (cleanup, origin) = LocalOrderedSemanticCleanupV1::from_actor_build_failure(failure);
        let deadlines = cleanup.confirm_deadlines_disabled();
        Self {
            cleanup,
            origin,
            deadlines,
            client_usage: LocalOrderedClientRetirementUsageV1::default(),
        }
    }

    pub fn from_actor_recovered(
        recovered: SemanticSessionActorRecoveredV1<LocalOrderedSemanticPortV1<P>>,
    ) -> Self {
        let (cleanup, origin) = LocalOrderedSemanticCleanupV1::from_actor_recovered(recovered);
        let deadlines = cleanup.confirm_deadlines_disabled();
        Self {
            cleanup,
            origin,
            deadlines,
            client_usage: LocalOrderedClientRetirementUsageV1::default(),
        }
    }

    fn from_managed_actor_recovered(
        recovered: SemanticSessionActorRecoveredV1<LocalOrderedSemanticPortV1<P>>,
        stop: LocalOrderedActorStopEvidenceV1,
        join: LocalOrderedActorRecoveryJoinV1,
    ) -> Self {
        let (cleanup, origin) = LocalOrderedSemanticCleanupV1::from_actor_recovered_with_stop(
            recovered,
            Some(stop),
            join,
        );
        let deadlines = cleanup.confirm_deadlines_disabled();
        Self {
            cleanup,
            origin,
            deadlines,
            client_usage: LocalOrderedClientRetirementUsageV1::default(),
        }
    }

    pub const fn origin(&self) -> LocalOrderedCleanupOriginEvidenceV1 {
        self.origin
    }

    pub const fn deadlines(&self) -> &LocalOrderedDeadlineRetirementReceiptV1 {
        &self.deadlines
    }

    pub const fn client_usage(&self) -> LocalOrderedClientRetirementUsageV1 {
        self.client_usage
    }

    pub fn drive_client_retirement_once(&mut self) -> SemanticClientRetirementReportV1 {
        let report = self.cleanup.retire_client();
        self.client_usage.observe(report);
        report
    }
}

impl<T, G, R, U> LocalOrderedCleanupOwnerV1<LspFreshStartedAdapterV1<T, G, R, U>>
where
    T: LspTransportPortV1,
    G: SemanticGenerationIssuerV1,
    R: LspIndexReadinessPortV1,
    U: LspVirtualDocumentIssuerV1,
{
    pub fn try_into_provider_cleanup(self) -> LocalOrderedProviderCleanupResultV1<T, G, R, U> {
        let Self {
            cleanup,
            origin,
            deadlines,
            client_usage,
        } = self;
        match cleanup.try_into_retiring_provider() {
            Ok(provider) => Ok(LocalOrderedProviderCleanupOwnerV1 {
                origin,
                deadlines,
                client_usage,
                extraction: LocalOrderedProviderExtractionV1::Checked,
                driver: SemanticRetirementDriverV1::from_bridge_provider(provider),
            }),
            Err(failure) => {
                let blockers = failure.blockers();
                Err(Box::new(LocalOrderedCleanupBlockedV1 {
                    owner: LocalOrderedCleanupOwnerV1 {
                        cleanup: failure.into_cleanup(),
                        origin,
                        deadlines,
                        client_usage,
                    },
                    blockers,
                }))
            }
        }
    }

    pub fn force_into_provider_cleanup(
        self,
        policy: SemanticBridgeForcedEffectPolicyV1,
    ) -> LocalOrderedProviderCleanupOwnerV1<T, G, R, U> {
        let Self {
            cleanup,
            origin,
            deadlines,
            client_usage,
        } = self;
        LocalOrderedProviderCleanupOwnerV1 {
            origin,
            deadlines,
            client_usage,
            extraction: LocalOrderedProviderExtractionV1::Forced,
            driver: SemanticRetirementDriverV1::from_forced_bridge_provider(
                cleanup.force_into_retiring_provider(policy),
            ),
        }
    }
}

impl<T, G, R, U> LocalOrderedProviderCleanupOwnerV1<T, G, R, U>
where
    T: LspTransportPortV1 + Send,
    G: SemanticGenerationIssuerV1 + Send,
    R: LspIndexReadinessPortV1 + Send,
    U: LspVirtualDocumentIssuerV1 + Send,
{
    pub fn state(&self) -> LocalSemanticRetirementStateV1 {
        self.driver.state()
    }

    pub fn drive_once(&mut self) -> LocalSemanticRetirementStateV1 {
        self.driver.drive_once()
    }

    pub fn escalate(&mut self) -> LocalSemanticRetirementStateV1 {
        self.driver.escalate()
    }

    pub fn try_into_terminal_receipt(
        self,
    ) -> Result<LocalOrderedTerminalCleanupReceiptV1, Box<Self>> {
        let Self {
            origin,
            deadlines,
            client_usage,
            extraction,
            driver,
        } = self;
        match driver.try_into_terminal_receipt() {
            Ok(provider) => Ok(LocalOrderedTerminalCleanupReceiptV1 {
                origin,
                deadlines,
                client_usage,
                extraction,
                provider,
            }),
            Err(driver) => Err(Box::new(Self {
                origin,
                deadlines,
                client_usage,
                extraction,
                driver: *driver,
            })),
        }
    }
}

impl<T, G, R, U> LocalOrderedSemanticBuildFailureV1<LspFreshStartedAdapterV1<T, G, R, U>>
where
    T: LspTransportPortV1,
    G: SemanticGenerationIssuerV1,
    R: LspIndexReadinessPortV1,
    U: LspVirtualDocumentIssuerV1,
{
    pub fn into_affine_cleanup_owner(self) -> LocalOrderedBuildCleanupOwnerV1<T, G, R, U> {
        let (cleanup, origin) = self.into_cleanup_origin();
        let deadlines = disabled_deadline_receipt();
        match cleanup {
            LocalOrderedSemanticBuildCleanupV1::Uninstalled(adapter) => {
                LocalOrderedBuildCleanupOwnerV1::Uninstalled(Box::new(
                    LocalOrderedProviderCleanupOwnerV1 {
                        origin,
                        deadlines,
                        client_usage: LocalOrderedClientRetirementUsageV1::default(),
                        extraction: LocalOrderedProviderExtractionV1::Uninstalled,
                        driver: SemanticRetirementDriverV1::from_uninstalled_fresh_adapter(adapter),
                    },
                ))
            }
            LocalOrderedSemanticBuildCleanupV1::Installed(cleanup) => {
                LocalOrderedBuildCleanupOwnerV1::Bridge(Box::new(LocalOrderedCleanupOwnerV1 {
                    cleanup: *cleanup,
                    origin,
                    deadlines,
                    client_usage: LocalOrderedClientRetirementUsageV1::default(),
                }))
            }
        }
    }
}

fn disabled_deadline_receipt() -> LocalOrderedDeadlineRetirementReceiptV1 {
    LocalOrderedDeadlineRetirementReceiptV1 {
        disposition: LocalOrderedDeadlineDispositionV1::Disabled,
        terminal_clears: 0,
        discarded_live: Vec::new(),
        unresolved: Vec::new(),
        retirement_discards: 0,
        saturated: false,
    }
}

impl<C> LocalOrderedDeadlineSchedulerV1<C>
where
    C: LocalOrderedMonotonicClockV1,
{
    fn try_new(
        clock: C,
        policy: LocalOrderedDeadlinePolicyV1,
        required_capacity: usize,
    ) -> Result<Self, (LocalOrderedDeadlineBuildRefusalV1, C)> {
        if !policy.is_valid() {
            return Err((LocalOrderedDeadlineBuildRefusalV1::InvalidPolicy, clock));
        }
        if policy.max_deadlines < required_capacity {
            return Err((
                LocalOrderedDeadlineBuildRefusalV1::InsufficientCapacity,
                clock,
            ));
        }
        Ok(Self {
            clock,
            policy,
            by_request: BTreeMap::new(),
            by_due: BTreeSet::new(),
            last_now: None,
            usage: LocalOrderedDeadlineUsageV1::default(),
            poisoned: false,
        })
    }

    fn graph_is_consistent(&self) -> bool {
        self.by_request.len() == self.by_due.len()
            && self
                .by_request
                .iter()
                .all(|(request, due)| self.by_due.contains(&(*due, *request)))
            && self.by_due.iter().all(|(due, request)| {
                self.by_request
                    .get(request)
                    .is_some_and(|indexed_due| indexed_due == due)
            })
    }

    fn ensure_consistent(&mut self) -> Result<(), SemanticBrokerEffectFailureV1> {
        if self.graph_is_consistent() {
            Ok(())
        } else {
            self.poisoned = true;
            Err(SemanticBrokerEffectFailureV1::Poisoned)
        }
    }

    fn sample(&mut self) -> Result<LocalOrderedInstantV1, SemanticBrokerEffectFailureV1> {
        let now = match self.clock.sample() {
            Ok(now) => now,
            Err(LocalOrderedClockFailureV1::Unavailable) => {
                deadline_bump(&mut self.usage.clock_unavailable, &mut self.usage.saturated);
                return Err(SemanticBrokerEffectFailureV1::RuntimeUnavailable);
            }
        };
        deadline_bump(&mut self.usage.clock_samples, &mut self.usage.saturated);
        if self.last_now.is_some_and(|last| now < last) {
            deadline_bump(&mut self.usage.clock_regressions, &mut self.usage.saturated);
            return Err(SemanticBrokerEffectFailureV1::Poisoned);
        }
        self.last_now = Some(now);
        Ok(now)
    }

    fn reserve_due(&mut self) -> Result<LocalOrderedInstantV1, SemanticBrokerEffectFailureV1> {
        self.ensure_consistent()?;
        if self.by_request.len() == self.policy.max_deadlines {
            deadline_bump(&mut self.usage.capacity_refusals, &mut self.usage.saturated);
            return Err(SemanticBrokerEffectFailureV1::Poisoned);
        }
        let now = self.sample()?;
        let Some(due) = now.checked_add(self.policy.relative_budget_ticks) else {
            deadline_bump(&mut self.usage.clock_overflow, &mut self.usage.saturated);
            return Err(SemanticBrokerEffectFailureV1::Poisoned);
        };
        Ok(due)
    }

    fn prepare_install(
        &mut self,
        request: SemanticRequestRefV1,
        due: LocalOrderedInstantV1,
    ) -> Result<LocalOrderedDeadlineInstallV1, SemanticBrokerEffectFailureV1> {
        self.ensure_consistent()?;
        if self.by_request.contains_key(&request)
            || self.by_due.iter().any(|(_, indexed)| *indexed == request)
        {
            return Err(SemanticBrokerEffectFailureV1::Poisoned);
        }
        Ok(LocalOrderedDeadlineInstallV1 { request, due })
    }

    fn commit_install(&mut self, install: LocalOrderedDeadlineInstallV1) {
        // The affine token is issued only after a complete graph validation
        // and vacancy proof. Nothing can observe or mutate this scheduler
        // while the synchronous service effect runs, so commit cannot fail
        // after the broker report has been produced.
        let LocalOrderedDeadlineInstallV1 { request, due } = install;
        let request_absent = self.by_request.insert(request, due).is_none();
        let due_absent = self.by_due.insert((due, request));
        debug_assert!(request_absent && due_absent);
        deadline_bump(&mut self.usage.scheduled, &mut self.usage.saturated);
        self.usage.current = self.by_request.len();
        self.usage.high_water = self.usage.high_water.max(self.usage.current);
    }

    fn clear(&mut self, request: SemanticRequestRefV1, cause: usize) -> bool {
        // Every effect preflights the complete graph before entering the
        // service. Reconciliation is consequently total: exact duplicates are
        // absence, while a present request has exactly one matching due edge.
        let Some(due) = self.by_request.remove(&request) else {
            return false;
        };
        let removed = self.by_due.remove(&(due, request));
        debug_assert!(
            removed,
            "prevalidated deadline graph diverged without mutation"
        );
        let saturated = deadline_bump_slot(&mut self.usage.clears_by_cause, cause);
        self.usage.saturated |= saturated;
        self.usage.current = self.by_request.len();
        true
    }

    fn earliest_due(
        &mut self,
    ) -> Result<Option<SemanticRequestRefV1>, SemanticBrokerEffectFailureV1> {
        self.ensure_consistent()?;
        deadline_bump(&mut self.usage.expiry_drives, &mut self.usage.saturated);
        let now = self.sample()?;
        let request = self
            .by_due
            .first()
            .filter(|(due, _)| *due <= now)
            .map(|(_, request)| *request);
        if request.is_none() {
            deadline_bump(&mut self.usage.no_due, &mut self.usage.saturated);
        }
        Ok(request)
    }

    fn retirement_receipt(&mut self) -> LocalOrderedDeadlineRetirementReceiptV1 {
        let request_keys: Vec<_> = self.by_request.keys().copied().collect();
        let mut unresolved = BTreeSet::new();
        for (request, due) in &self.by_request {
            if !self.by_due.contains(&(*due, *request)) {
                unresolved.insert(*request);
            }
        }
        for (due, request) in &self.by_due {
            if self.by_request.get(request) != Some(due) {
                unresolved.insert(*request);
            }
        }
        let unresolved: Vec<_> = unresolved.into_iter().collect();
        let disposition = if unresolved.is_empty() {
            LocalOrderedDeadlineDispositionV1::Retired
        } else {
            LocalOrderedDeadlineDispositionV1::Unresolved
        };
        if disposition == LocalOrderedDeadlineDispositionV1::Retired {
            let discards = u64::try_from(request_keys.len()).unwrap_or(u64::MAX);
            let before = self.usage.retirement_discards;
            self.usage.retirement_discards = before.saturating_add(discards);
            self.usage.saturated |= before.checked_add(discards).is_none();
        }
        let mut terminal_clears = 0_u64;
        let mut saturated = self.usage.saturated;
        for clears in self.usage.clears_by_cause {
            let before = terminal_clears;
            terminal_clears = terminal_clears.saturating_add(clears);
            saturated |= before.checked_add(clears).is_none();
        }
        self.usage.saturated = saturated;
        LocalOrderedDeadlineRetirementReceiptV1 {
            disposition,
            terminal_clears,
            discarded_live: request_keys,
            unresolved,
            retirement_discards: self.usage.retirement_discards,
            saturated,
        }
    }
}

#[must_use = "failed ordered-provider extraction retains the mapper cleanup obligation"]
pub struct LocalOrderedSemanticExtractionFailureV1<P> {
    port: LocalOrderedSemanticPortV1<P>,
    blockers: SemanticBridgeProviderExtractionBlockersV1,
}

impl<P> LocalOrderedSemanticExtractionFailureV1<P> {
    pub const fn blockers(&self) -> SemanticBridgeProviderExtractionBlockersV1 {
        self.blockers
    }

    pub fn into_cleanup(self) -> LocalOrderedSemanticCleanupV1<P> {
        LocalOrderedSemanticCleanupV1 { port: self.port }
    }
}

impl<P> fmt::Debug for LocalOrderedSemanticExtractionFailureV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedSemanticExtractionFailureV1")
            .field("blockers", &self.blockers)
            .field("port", &"<owned>")
            .finish()
    }
}

impl<P> fmt::Debug for LocalOrderedSemanticPreparedV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _owned_pair = (&self.broker, &self.port);
        formatter.write_str("LocalOrderedSemanticPreparedV1(<opaque>)")
    }
}

impl<P, C> fmt::Debug for LocalOrderedTimedSemanticPreparedV1<P, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _owned_pair = (&self.broker, &self.port);
        formatter.write_str("LocalOrderedTimedSemanticPreparedV1(<opaque>)")
    }
}

impl<P, C> fmt::Debug for LocalOrderedTimedSemanticPortV1<P, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedTimedSemanticPortV1")
            .field("base", &"<owned>")
            .field("deadlines", &self.scheduler.by_request.len())
            .finish()
    }
}

impl<P, C> fmt::Debug for LocalOrderedTimedSemanticCleanupV1<P, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LocalOrderedTimedSemanticCleanupV1(<opaque>)")
    }
}

impl<P> fmt::Debug for LocalOrderedSemanticPortV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedSemanticPortV1")
            .field("provider_epoch", &self.binding.epoch)
            .field("provider_readiness", &self.binding.observation.readiness())
            .field("service", &"<owned>")
            .field("client", &"<opaque>")
            .finish()
    }
}

impl<P: SemanticProviderPortV1> LocalOrderedSemanticPortV1<P> {
    fn try_new(
        provider: P,
        client: SemanticClientRefV1,
        session_limits: SemanticSessionLimitsV1,
        service_limits: SemanticServiceLimitsV1,
    ) -> Result<Self, Box<LocalOrderedSemanticBuildFailureV1<P>>> {
        if !limits_align(session_limits, service_limits) {
            return Err(Box::new(LocalOrderedSemanticBuildFailureV1 {
                reason: LocalOrderedSemanticBuildRefusalV1::InvalidLimits,
                residue: LocalOrderedSemanticBuildResidueV1::UninstalledProvider(provider),
            }));
        }
        let binding = match project_state(provider.state(), 0) {
            Ok(binding) => binding,
            Err(_) => {
                return Err(Box::new(LocalOrderedSemanticBuildFailureV1 {
                    reason: LocalOrderedSemanticBuildRefusalV1::InvalidProviderState,
                    residue: LocalOrderedSemanticBuildResidueV1::UninstalledProvider(provider),
                }));
            }
        };
        // `limits_align` proves every bridge constructor predicate. The
        // service API predates lossless constructor refusal, so reaching this
        // branch would be an implementation invariant violation rather than a
        // caller-observable build outcome.
        let mut service = SemanticBridgeServiceV1::try_new(provider, service_limits)
            .unwrap_or_else(|_| unreachable!("validated bridge limits construct the service"));
        if let Err(reason) = service.admit_client(client) {
            return Err(Box::new(LocalOrderedSemanticBuildFailureV1 {
                reason: LocalOrderedSemanticBuildRefusalV1::Service(reason),
                residue: LocalOrderedSemanticBuildResidueV1::PreparedPort(Box::new(Self {
                    service,
                    client,
                    binding,
                    service_limits,
                })),
            }));
        }
        Ok(Self {
            service,
            client,
            binding,
            service_limits,
        })
    }

    /// Replaces the physical provider as one ordered broker event. Candidate
    /// projection and epoch headroom are proved before snapshot replay or swap,
    /// so every accepted mutation has a representable exact transition. The
    /// displaced provider remains affine in the returned retirement product.
    pub fn try_replace_provider(
        &mut self,
        candidate: P,
    ) -> Result<LocalOrderedProviderReplacementV1<P>, LocalOrderedProviderReplacementFailureV1<P>>
    {
        let Some(next_epoch) = self.binding.epoch.checked_add(1) else {
            return Err(LocalOrderedProviderReplacementFailureV1::Preflight {
                reason: LocalOrderedProviderReplacementRefusalV1::EpochExhausted,
                candidate,
            });
        };
        if project_state(candidate.state(), next_epoch).is_err() {
            return Err(LocalOrderedProviderReplacementFailureV1::Preflight {
                reason: LocalOrderedProviderReplacementRefusalV1::InvalidCandidateProjection,
                candidate,
            });
        }
        let (retirement, report) = self
            .service
            .try_replace_provider(candidate)
            .map_err(LocalOrderedProviderReplacementFailureV1::Service)?;
        let translated = self
            .translate(&report)
            .unwrap_or_else(|_| unreachable!("replacement projection was preflighted"));
        let broker_report = SemanticBrokerProviderDriveReportV1 {
            provider_before: translated.provider_before.clone(),
            pre_provider: translated.pre_provider.clone(),
            disposition: SemanticBrokerProviderDriveDispositionV1::NoEvent,
            post_provider: translated.post_provider.clone(),
        };
        self.commit(&translated);
        Ok(LocalOrderedProviderReplacementV1 {
            retirement,
            report: broker_report,
        })
    }

    fn translate<T>(
        &self,
        report: &SemanticBridgeOrderedReportV1<T>,
    ) -> Result<TranslatedTransitions, SemanticBrokerEffectFailureV1> {
        let provider_before = project_state(report.provider_before().clone(), self.binding.epoch)?;
        if provider_before != self.binding {
            return Err(SemanticBrokerEffectFailureV1::Poisoned);
        }
        let mut shadow = self.binding.clone();
        let pre_provider = report
            .pre_provider()
            .map(|transition| map_transition(transition, &mut shadow))
            .transpose()?;
        let post_provider = report
            .post_provider()
            .map(|transition| map_transition(transition, &mut shadow))
            .transpose()?;
        Ok(TranslatedTransitions {
            provider_before,
            pre_provider,
            post_provider,
            settled: shadow,
        })
    }

    fn snapshot(
        &self,
        snapshot: &SemanticSessionSnapshotV1,
    ) -> Result<SemanticSnapshotV1, SemanticBrokerEffectFailureV1> {
        SemanticSnapshotV1::try_new(
            snapshot.target(),
            snapshot.language().to_owned(),
            snapshot.source().to_owned(),
            self.service_limits,
        )
        .map_err(|_| SemanticBrokerEffectFailureV1::Poisoned)
    }

    fn commit(&mut self, translated: &TranslatedTransitions) {
        self.binding = translated.settled.clone();
    }

    fn expire(
        &mut self,
        request: SemanticRequestRefV1,
    ) -> Result<SemanticBrokerExpiryReportV1, SemanticBrokerEffectFailureV1> {
        let report = self.service.expire(self.client, request);
        let translated = self.translate(&report)?;
        let outcome = match report.outcome() {
            SemanticBridgeExpiryActionV1::Expired => SemanticBrokerExpiryOutcomeV1::Expired,
            SemanticBridgeExpiryActionV1::SatisfiedByProviderTransition => {
                SemanticBrokerExpiryOutcomeV1::SatisfiedByProviderTransition
            }
            SemanticBridgeExpiryActionV1::NotPending => SemanticBrokerExpiryOutcomeV1::NotPending,
            SemanticBridgeExpiryActionV1::Refused(_) => {
                return Err(SemanticBrokerEffectFailureV1::Poisoned);
            }
        };
        self.commit(&translated);
        Ok(SemanticBrokerExpiryReportV1 {
            provider_before: translated.provider_before,
            pre_provider: translated.pre_provider,
            internal_request: request,
            outcome,
            post_provider: translated.post_provider,
        })
    }

    fn retire_client(&mut self) -> SemanticClientRetirementReportV1 {
        self.service.retire_client(self.client)
    }

    fn try_into_retiring_provider(
        self,
    ) -> Result<SemanticBridgeRetiringProviderV1<P>, Box<LocalOrderedSemanticExtractionFailureV1<P>>>
    {
        let Self {
            service,
            client,
            binding,
            service_limits,
        } = self;
        match service.try_into_retiring_provider() {
            Ok(provider) => Ok(provider),
            Err(failure) => {
                let blockers = failure.blockers();
                Err(Box::new(LocalOrderedSemanticExtractionFailureV1 {
                    port: Self {
                        service: failure.into_service(),
                        client,
                        binding,
                        service_limits,
                    },
                    blockers,
                }))
            }
        }
    }

    fn force_into_retiring_provider(
        self,
        policy: SemanticBridgeForcedEffectPolicyV1,
    ) -> SemanticBridgeForcedRetiringProviderV1<P> {
        self.service.force_into_retiring_provider(policy)
    }
}

impl<P: SemanticProviderPortV1> LocalOrderedSemanticPreparedV1<P> {
    /// Coordinates an already-constructed injected provider with the fresh
    /// bridge and broker genesis in an explicitly deadline-disabled profile.
    /// This validates session/service bounds only; an LSP composition must
    /// additionally prove its adapter semantic, document and total-source
    /// limits before supplying the provider here. A timed profile requires a
    /// distinct scheduler-owning type and cannot be enabled after construction.
    pub(crate) fn try_new_injected_provider_without_deadlines(
        provider: P,
        client: SemanticClientRefV1,
        incarnation: SemanticSessionIncarnationV1,
        session_limits: SemanticSessionLimitsV1,
        service_limits: SemanticServiceLimitsV1,
    ) -> Result<Self, Box<LocalOrderedSemanticBuildFailureV1<P>>> {
        let port =
            LocalOrderedSemanticPortV1::try_new(provider, client, session_limits, service_limits)?;
        let Some(broker) = SemanticSessionBrokerV1::try_new_with_provider_binding(
            incarnation,
            session_limits,
            port.binding.clone(),
        ) else {
            return Err(Box::new(LocalOrderedSemanticBuildFailureV1 {
                reason: LocalOrderedSemanticBuildRefusalV1::BrokerGenesis,
                residue: LocalOrderedSemanticBuildResidueV1::PreparedPort(Box::new(port)),
            }));
        };
        Ok(Self { broker, port })
    }

    pub fn try_with_deadlines<C>(
        self,
        clock: C,
        policy: LocalOrderedDeadlinePolicyV1,
    ) -> Result<
        LocalOrderedTimedSemanticPreparedV1<P, C>,
        Box<LocalOrderedDeadlineBuildFailureV1<P, C>>,
    >
    where
        C: LocalOrderedMonotonicClockV1,
    {
        let required_capacity = self.port.service_limits.max_pending;
        let scheduler =
            match LocalOrderedDeadlineSchedulerV1::try_new(clock, policy, required_capacity) {
                Ok(scheduler) => scheduler,
                Err((reason, clock)) => {
                    return Err(Box::new(LocalOrderedDeadlineBuildFailureV1 {
                        reason,
                        prepared: self,
                        clock,
                    }));
                }
            };
        Ok(LocalOrderedTimedSemanticPreparedV1 {
            broker: self.broker,
            port: LocalOrderedTimedSemanticPortV1 {
                base: self.port,
                scheduler,
            },
        })
    }

    pub fn into_cleanup(self) -> LocalOrderedSemanticCleanupV1<P> {
        self.into_cleanup_with_evidence().0
    }

    pub fn into_cleanup_with_evidence(
        self,
    ) -> (
        LocalOrderedSemanticCleanupV1<P>,
        LocalOrderedCleanupOriginEvidenceV1,
    ) {
        // No command/effect surface is exposed before spawn, so this broker is
        // still its source-free, request-free genesis value.
        let evidence = LocalOrderedCleanupOriginEvidenceV1 {
            origin: LocalOrderedCleanupOriginV1::Unstarted,
            broker_usage: Some(self.broker.usage()),
            actor_usage: None,
            actor_limits: None,
            actor_stop: None,
            actor_recovery_join: LocalOrderedActorRecoveryJoinV1::NotApplicable,
        };
        drop(self.broker);
        (LocalOrderedSemanticCleanupV1 { port: self.port }, evidence)
    }
}

pub type LocalOrderedTimedSemanticActorV1<P, C> = (
    SemanticSessionActorOwnerV1,
    SemanticSessionActorSenderV1,
    SemanticSessionActorRecoveryV1<LocalOrderedTimedSemanticPortV1<P, C>>,
);

pub type LocalOrderedTimedSemanticSpawnResultV1<P, C> = Result<
    LocalOrderedTimedSemanticActorV1<P, C>,
    Box<SemanticSessionActorBuildFailureV1<LocalOrderedTimedSemanticPortV1<P, C>>>,
>;

/// Same-spawn control/recovery product for deadline-enabled actors. Scheduler
/// ownership stays inside the recovered timed port until an explicit deadline
/// retirement or abandonment transition.
#[must_use = "managed timed actor ownership must stop and recover into timed cleanup"]
pub struct LocalOrderedManagedTimedSemanticActorV1<P, C> {
    owner: SemanticSessionActorOwnerV1,
    recovery: SemanticSessionActorRecoveryV1<LocalOrderedTimedSemanticPortV1<P, C>>,
    refusal_attempts: u64,
    last_refusal: Option<SemanticSessionActorStopReceiptV1>,
    saturated: bool,
}

impl<P, C> fmt::Debug for LocalOrderedManagedTimedSemanticActorV1<P, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedManagedTimedSemanticActorV1")
            .field("refusal_attempts", &self.refusal_attempts)
            .field("last_refusal", &self.last_refusal)
            .field("saturated", &self.saturated)
            .field("owner", &"<owned>")
            .field("recovery", &"<owned>")
            .finish()
    }
}

impl<P, C> LocalOrderedManagedTimedSemanticActorV1<P, C> {
    pub fn try_event(
        &self,
        event: SemanticSessionActorExternalEventV1,
    ) -> SemanticSessionActorEventAdmissionV1 {
        self.owner.try_event(event)
    }

    pub fn try_drive_provider(
        &self,
    ) -> Result<SemanticSessionActorControlTicketV1, SemanticSessionActorControlRefusalV1> {
        self.owner.try_drive_provider()
    }

    pub fn try_drive_expiry(
        &self,
    ) -> Result<SemanticSessionActorControlTicketV1, SemanticSessionActorControlRefusalV1> {
        self.owner.try_drive_expiry()
    }

    pub fn try_drive_retirement(
        &self,
    ) -> Result<SemanticSessionActorControlTicketV1, SemanticSessionActorControlRefusalV1> {
        self.owner.try_drive_retirement()
    }

    pub fn try_rotate_incarnation(
        &self,
        next: SemanticSessionIncarnationV1,
    ) -> Result<SemanticSessionActorControlTicketV1, SemanticSessionActorControlRefusalV1> {
        self.owner.try_rotate_incarnation(next)
    }

    pub fn begin_drain(&self) -> bool {
        self.owner.begin_drain()
    }

    pub fn phase(&self) -> SemanticSessionActorPhaseV1 {
        self.owner.phase()
    }

    pub fn usage(&self) -> SemanticSessionActorUsageV1 {
        self.owner.usage()
    }

    pub fn try_begin_stop(
        self,
    ) -> Result<
        LocalOrderedJoinedTimedStopTicketV1<P, C>,
        Box<LocalOrderedManagedTimedStopStartFailureV1<P, C>>,
    > {
        let Self {
            owner,
            recovery,
            refusal_attempts,
            last_refusal,
            saturated,
        } = self;
        match owner.try_begin_stop() {
            Ok(ticket) => Ok(LocalOrderedJoinedTimedStopTicketV1 {
                ticket,
                recovery,
                refusal_attempts,
                last_refusal,
                saturated,
            }),
            Err(failure) => Err(Box::new(LocalOrderedManagedTimedStopStartFailureV1 {
                reason: failure.reason(),
                actor: Self {
                    owner: failure.into_owner(),
                    recovery,
                    refusal_attempts,
                    last_refusal,
                    saturated,
                },
            })),
        }
    }

    pub fn try_recover_failed(
        self,
    ) -> Result<LocalOrderedTimedStoppedRecoveryOwnerV1<P, C>, Box<Self>> {
        if self.owner.phase() != SemanticSessionActorPhaseV1::Failed {
            return Err(Box::new(self));
        }
        let Self {
            owner,
            recovery,
            refusal_attempts,
            last_refusal,
            saturated,
        } = self;
        let usage = owner.usage();
        drop(owner);
        Ok(LocalOrderedTimedStoppedRecoveryOwnerV1 {
            stop: LocalOrderedActorStopEvidenceV1 {
                terminal: LocalOrderedActorTerminalStopV1::FailedBeforeStop { usage },
                refusal_attempts,
                last_refusal,
                saturated,
            },
            recovery,
        })
    }

    /// Atomically fences new commands and preserves the complete live broker,
    /// snapshot, deadline, and provider product for a composition-owned
    /// replacement. The caller must discard every old sender before exposing a
    /// resumed sender; old senders remain permanently fenced by actor phase.
    pub fn try_begin_suspension(
        self,
    ) -> Result<LocalOrderedTimedSuspensionTicketV1<P, C>, Box<Self>> {
        let Self {
            owner,
            recovery,
            refusal_attempts,
            last_refusal,
            saturated,
        } = self;
        let limits = owner.limits();
        match owner.try_suspend() {
            Ok(control) => {
                drop(owner);
                Ok(LocalOrderedTimedSuspensionTicketV1 {
                    control: Some(control),
                    acknowledged: false,
                    recovery,
                    limits,
                    refusal_attempts,
                    last_refusal,
                    saturated,
                })
            }
            Err(_) => Err(Box::new(Self {
                owner,
                recovery,
                refusal_attempts,
                last_refusal,
                saturated,
            })),
        }
    }
}

#[must_use = "suspension must recover the exact live actor runtime"]
pub struct LocalOrderedTimedSuspensionTicketV1<P, C> {
    control: Option<SemanticSessionActorControlTicketV1>,
    acknowledged: bool,
    recovery: SemanticSessionActorRecoveryV1<LocalOrderedTimedSemanticPortV1<P, C>>,
    limits: SemanticSessionActorLimitsV1,
    refusal_attempts: u64,
    last_refusal: Option<SemanticSessionActorStopReceiptV1>,
    saturated: bool,
}

pub enum LocalOrderedTimedSuspensionAdvanceV1<P, C> {
    Pending(Box<LocalOrderedTimedSuspensionTicketV1<P, C>>),
    Ready(Box<LocalOrderedSuspendedTimedRuntimeV1<P, C>>),
    Failed(Box<SemanticSessionActorRecoveredV1<LocalOrderedTimedSemanticPortV1<P, C>>>),
}

impl<P, C> LocalOrderedTimedSuspensionTicketV1<P, C> {
    pub fn try_advance(mut self) -> LocalOrderedTimedSuspensionAdvanceV1<P, C> {
        if !self.acknowledged {
            let control = self
                .control
                .as_ref()
                .expect("unacknowledged control exists");
            match control.try_receive() {
                Ok(SemanticSessionActorControlCompletionV1::Applied) => {
                    self.acknowledged = true;
                    self.control = None;
                }
                Ok(
                    SemanticSessionActorControlCompletionV1::Quarantined
                    | SemanticSessionActorControlCompletionV1::OutcomeUnknown,
                ) => {
                    // The recovery product, not the acknowledgement channel,
                    // decides whether the exact runtime actually suspended.
                    self.acknowledged = true;
                    self.control = None;
                }
                Err(SemanticSessionActorPollV1::Pending) => {
                    return LocalOrderedTimedSuspensionAdvanceV1::Pending(Box::new(self));
                }
                Err(SemanticSessionActorPollV1::OutcomeUnknown) => {
                    self.acknowledged = true;
                    self.control = None;
                }
            }
        }
        let recovered = match self.recovery.try_recover() {
            Ok(recovered) => recovered,
            Err(recovery) => {
                self.recovery = recovery;
                return LocalOrderedTimedSuspensionAdvanceV1::Pending(Box::new(self));
            }
        };
        if recovered.termination() != SemanticSessionActorTerminationV1::Suspended {
            return LocalOrderedTimedSuspensionAdvanceV1::Failed(Box::new(recovered));
        }
        let (_, usage, broker, port) = recovered.into_parts();
        LocalOrderedTimedSuspensionAdvanceV1::Ready(Box::new(LocalOrderedSuspendedTimedRuntimeV1 {
            broker,
            port,
            limits: self.limits,
            prior_actor_usage: usage,
            refusal_attempts: self.refusal_attempts,
            last_refusal: self.last_refusal,
            saturated: self.saturated,
        }))
    }
}

#[must_use = "suspended runtime must be replaced, resumed, or retired explicitly"]
pub struct LocalOrderedSuspendedTimedRuntimeV1<P, C> {
    broker: SemanticSessionBrokerV1,
    port: LocalOrderedTimedSemanticPortV1<P, C>,
    limits: SemanticSessionActorLimitsV1,
    prior_actor_usage: SemanticSessionActorUsageV1,
    refusal_attempts: u64,
    last_refusal: Option<SemanticSessionActorStopReceiptV1>,
    saturated: bool,
}

pub type LocalOrderedSuspendedReplacementResultV1<P, C> = Result<
    (
        LocalOrderedSuspendedTimedRuntimeV1<P, C>,
        SemanticProviderRetirementV1<P>,
    ),
    Box<LocalOrderedSuspendedReplacementFailureV1<P, C>>,
>;

impl<P, C> LocalOrderedSuspendedTimedRuntimeV1<P, C>
where
    P: SemanticBridgeProviderPollV1 + Send + 'static,
    C: LocalOrderedMonotonicClockV1,
{
    pub const fn prior_actor_usage(&self) -> SemanticSessionActorUsageV1 {
        self.prior_actor_usage
    }

    pub fn try_replace_provider(
        mut self,
        candidate: P,
    ) -> LocalOrderedSuspendedReplacementResultV1<P, C> {
        let replacement = match self.port.try_replace_provider(candidate) {
            Ok(replacement) => replacement,
            Err(failure) => {
                return Err(Box::new(
                    LocalOrderedSuspendedReplacementFailureV1::Candidate {
                        runtime: self,
                        failure,
                    },
                ));
            }
        };
        let decision = self
            .broker
            .event(SemanticBrokerEventV1::ProviderDriveCompleted {
                report: Box::new(replacement.report),
            });
        if decision != SemanticBrokerDecisionV1::EventAccepted {
            return Err(Box::new(
                LocalOrderedSuspendedReplacementFailureV1::Reconciliation {
                    runtime: LocalOrderedFusedSuspendedRuntimeV1 { inner: self },
                    retirement: replacement.retirement,
                },
            ));
        }
        Ok((self, replacement.retirement))
    }

    pub fn resume(self) -> LocalOrderedManagedTimedSemanticSpawnResultV1<P, C> {
        let (owner, sender, recovery) = resume_semantic_session_actor_v1(
            self.broker,
            self.port,
            self.limits,
            self.prior_actor_usage,
        )?;
        Ok((
            LocalOrderedManagedTimedSemanticActorV1 {
                owner,
                recovery,
                refusal_attempts: self.refusal_attempts,
                last_refusal: self.last_refusal,
                saturated: self.saturated,
            },
            sender,
        ))
    }
}

#[must_use = "replacement failure retains both runtime and cleanup authority"]
pub enum LocalOrderedSuspendedReplacementFailureV1<P, C> {
    Candidate {
        runtime: LocalOrderedSuspendedTimedRuntimeV1<P, C>,
        failure: LocalOrderedProviderReplacementFailureV1<P>,
    },
    Reconciliation {
        runtime: LocalOrderedFusedSuspendedRuntimeV1<P, C>,
        retirement: SemanticProviderRetirementV1<P>,
    },
}

impl<P, C> fmt::Debug for LocalOrderedSuspendedReplacementFailureV1<P, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Candidate { failure, .. } => formatter
                .debug_struct("Candidate")
                .field("failure", failure)
                .field("runtime", &"<owned>")
                .finish(),
            Self::Reconciliation { .. } => formatter
                .debug_struct("Reconciliation")
                .field("runtime", &"<fused-owned>")
                .field("retirement", &"<owned>")
                .finish(),
        }
    }
}

/// A physical provider swap whose broker reconciliation did not close. There
/// is deliberately no resume operation: exposing this product would let the
/// physical and logical provider epochs diverge. It is retained solely with
/// the displaced-provider retirement owner for explicit cleanup.
#[must_use = "fused suspended runtime retains current and displaced provider cleanup authority"]
pub struct LocalOrderedFusedSuspendedRuntimeV1<P, C> {
    inner: LocalOrderedSuspendedTimedRuntimeV1<P, C>,
}

impl<P, C> fmt::Debug for LocalOrderedFusedSuspendedRuntimeV1<P, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _owned = &self.inner;
        formatter.write_str("LocalOrderedFusedSuspendedRuntimeV1(<owned>)")
    }
}

#[must_use = "failed timed stop admission retains the same-spawn actor product"]
pub struct LocalOrderedManagedTimedStopStartFailureV1<P, C> {
    reason: SemanticSessionActorControlRefusalV1,
    actor: LocalOrderedManagedTimedSemanticActorV1<P, C>,
}

impl<P, C> LocalOrderedManagedTimedStopStartFailureV1<P, C> {
    pub const fn reason(&self) -> SemanticSessionActorControlRefusalV1 {
        self.reason
    }

    pub fn into_actor(self) -> LocalOrderedManagedTimedSemanticActorV1<P, C> {
        self.actor
    }
}

impl<P, C> fmt::Debug for LocalOrderedManagedTimedStopStartFailureV1<P, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedManagedTimedStopStartFailureV1")
            .field("reason", &self.reason)
            .field("actor", &"<owned>")
            .finish()
    }
}

#[must_use = "joined timed stop ticket must resolve, retry, or explicitly abandon"]
pub struct LocalOrderedJoinedTimedStopTicketV1<P, C> {
    ticket: SemanticSessionActorStopTicketV1,
    recovery: SemanticSessionActorRecoveryV1<LocalOrderedTimedSemanticPortV1<P, C>>,
    refusal_attempts: u64,
    last_refusal: Option<SemanticSessionActorStopReceiptV1>,
    saturated: bool,
}

impl<P, C> fmt::Debug for LocalOrderedJoinedTimedStopTicketV1<P, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedJoinedTimedStopTicketV1")
            .field("refusal_attempts", &self.refusal_attempts)
            .field("last_refusal", &self.last_refusal)
            .field("saturated", &self.saturated)
            .field("ticket", &"<owned>")
            .field("recovery", &"<owned>")
            .finish()
    }
}

#[must_use = "resolved timed stop evidence must advance to recovery"]
pub enum LocalOrderedManagedTimedStopResolutionV1<P, C> {
    Terminal(LocalOrderedTimedStoppedRecoveryOwnerV1<P, C>),
    Refused(LocalOrderedManagedTimedStopRefusalV1<P, C>),
}

impl<P, C> fmt::Debug for LocalOrderedManagedTimedStopResolutionV1<P, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Terminal(_) => "Terminal(<owned>)",
            Self::Refused(_) => "Refused(<owned>)",
        })
    }
}

#[must_use = "timed stop refusal retains receipt and same-spawn actor ownership"]
pub struct LocalOrderedManagedTimedStopRefusalV1<P, C> {
    receipt: SemanticSessionActorStopReceiptV1,
    actor: LocalOrderedManagedTimedSemanticActorV1<P, C>,
}

impl<P, C> LocalOrderedManagedTimedStopRefusalV1<P, C> {
    pub const fn receipt(&self) -> SemanticSessionActorStopReceiptV1 {
        self.receipt
    }

    pub fn into_actor(self) -> LocalOrderedManagedTimedSemanticActorV1<P, C> {
        self.actor
    }
}

impl<P, C> fmt::Debug for LocalOrderedManagedTimedStopRefusalV1<P, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedManagedTimedStopRefusalV1")
            .field("receipt", &self.receipt)
            .field("actor", &"<owned>")
            .finish()
    }
}

impl<P, C> LocalOrderedJoinedTimedStopTicketV1<P, C> {
    pub fn try_complete(self) -> Result<LocalOrderedManagedTimedStopResolutionV1<P, C>, Box<Self>> {
        let Self {
            ticket,
            recovery,
            mut refusal_attempts,
            mut last_refusal,
            mut saturated,
        } = self;
        match ticket.try_complete() {
            Err(ticket) => Err(Box::new(Self {
                ticket,
                recovery,
                refusal_attempts,
                last_refusal,
                saturated,
            })),
            Ok(SemanticSessionActorStopResolutionV1::Clean(receipt)) => Ok(
                LocalOrderedManagedTimedStopResolutionV1::Terminal(Self::recovery_owner(
                    recovery,
                    refusal_attempts,
                    last_refusal,
                    saturated,
                    LocalOrderedActorTerminalStopV1::Clean(receipt),
                )),
            ),
            Ok(SemanticSessionActorStopResolutionV1::OutcomeUnknown(receipt)) => Ok(
                LocalOrderedManagedTimedStopResolutionV1::Terminal(Self::recovery_owner(
                    recovery,
                    refusal_attempts,
                    last_refusal,
                    saturated,
                    LocalOrderedActorTerminalStopV1::OutcomeUnknown(receipt),
                )),
            ),
            Ok(SemanticSessionActorStopResolutionV1::Refused { receipt, owner }) => {
                let before = refusal_attempts;
                refusal_attempts = before.saturating_add(1);
                saturated |= before == u64::MAX;
                last_refusal = Some(receipt);
                Ok(LocalOrderedManagedTimedStopResolutionV1::Refused(
                    LocalOrderedManagedTimedStopRefusalV1 {
                        receipt,
                        actor: LocalOrderedManagedTimedSemanticActorV1 {
                            owner,
                            recovery,
                            refusal_attempts,
                            last_refusal,
                            saturated,
                        },
                    },
                ))
            }
        }
    }

    pub fn abandon(self) -> LocalOrderedTimedStoppedRecoveryOwnerV1<P, C> {
        let Self {
            ticket,
            recovery,
            refusal_attempts,
            last_refusal,
            saturated,
        } = self;
        let abandonment = ticket.abandon();
        Self::recovery_owner(
            recovery,
            refusal_attempts,
            last_refusal,
            saturated,
            LocalOrderedActorTerminalStopV1::Abandoned(abandonment),
        )
    }

    fn recovery_owner(
        recovery: SemanticSessionActorRecoveryV1<LocalOrderedTimedSemanticPortV1<P, C>>,
        refusal_attempts: u64,
        last_refusal: Option<SemanticSessionActorStopReceiptV1>,
        saturated: bool,
        terminal: LocalOrderedActorTerminalStopV1,
    ) -> LocalOrderedTimedStoppedRecoveryOwnerV1<P, C> {
        LocalOrderedTimedStoppedRecoveryOwnerV1 {
            stop: LocalOrderedActorStopEvidenceV1 {
                terminal,
                refusal_attempts,
                last_refusal,
                saturated,
            },
            recovery,
        }
    }
}

#[must_use = "terminal timed stop evidence must recover the same-spawn runtime"]
pub struct LocalOrderedTimedStoppedRecoveryOwnerV1<P, C> {
    stop: LocalOrderedActorStopEvidenceV1,
    recovery: SemanticSessionActorRecoveryV1<LocalOrderedTimedSemanticPortV1<P, C>>,
}

impl<P, C> LocalOrderedTimedStoppedRecoveryOwnerV1<P, C> {
    pub const fn stop(&self) -> LocalOrderedActorStopEvidenceV1 {
        self.stop
    }
}

impl<P, C> fmt::Debug for LocalOrderedTimedStoppedRecoveryOwnerV1<P, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedTimedStoppedRecoveryOwnerV1")
            .field("stop", &self.stop)
            .field("recovery", &"<owned>")
            .finish()
    }
}

#[must_use = "timed recovery advance retains pending or mismatched cleanup authority"]
pub enum LocalOrderedTimedStoppedRecoveryAdvanceV1<P, C> {
    Pending(Box<LocalOrderedTimedStoppedRecoveryOwnerV1<P, C>>),
    Ready(Box<LocalOrderedTimedCleanupOwnerV1<P, C>>),
    Mismatch(Box<LocalOrderedTimedStopRecoveryMismatchV1<P, C>>),
}

impl<P, C> fmt::Debug for LocalOrderedTimedStoppedRecoveryAdvanceV1<P, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Pending(_) => "Pending(<owned>)",
            Self::Ready(_) => "Ready(<owned>)",
            Self::Mismatch(_) => "Mismatch(<owned>)",
        })
    }
}

#[must_use = "timed stop/recovery mismatch retains exact scheduler and cleanup authority"]
pub struct LocalOrderedTimedStopRecoveryMismatchV1<P, C> {
    stop: LocalOrderedActorStopEvidenceV1,
    recovered: SemanticSessionActorRecoveredV1<LocalOrderedTimedSemanticPortV1<P, C>>,
}

impl<P, C> LocalOrderedTimedStopRecoveryMismatchV1<P, C>
where
    P: SemanticBridgeProviderPollV1 + Send + 'static,
    C: LocalOrderedMonotonicClockV1,
{
    pub const fn stop(&self) -> LocalOrderedActorStopEvidenceV1 {
        self.stop
    }

    pub fn termination(&self) -> SemanticSessionActorTerminationV1 {
        self.recovered.termination()
    }

    pub fn into_cleanup(self) -> LocalOrderedTimedCleanupOwnerV1<P, C> {
        LocalOrderedTimedCleanupOwnerV1::from_managed_actor_recovered(
            self.recovered,
            self.stop,
            LocalOrderedActorRecoveryJoinV1::Mismatch,
        )
    }
}

impl<P, C> fmt::Debug for LocalOrderedTimedStopRecoveryMismatchV1<P, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedTimedStopRecoveryMismatchV1")
            .field("stop", &self.stop)
            .field("recovered", &"<owned>")
            .finish()
    }
}

impl<P, C> LocalOrderedTimedStoppedRecoveryOwnerV1<P, C>
where
    P: SemanticBridgeProviderPollV1 + Send + 'static,
    C: LocalOrderedMonotonicClockV1,
{
    pub fn try_advance(self) -> LocalOrderedTimedStoppedRecoveryAdvanceV1<P, C> {
        let Self { stop, recovery } = self;
        let recovered = match recovery.try_recover() {
            Ok(recovered) => recovered,
            Err(recovery) => {
                return LocalOrderedTimedStoppedRecoveryAdvanceV1::Pending(Box::new(Self {
                    stop,
                    recovery,
                }));
            }
        };
        if actor_stop_recovery_compatible(stop.terminal, recovered.termination()) {
            LocalOrderedTimedStoppedRecoveryAdvanceV1::Ready(Box::new(
                LocalOrderedTimedCleanupOwnerV1::from_managed_actor_recovered(
                    recovered,
                    stop,
                    LocalOrderedActorRecoveryJoinV1::Compatible,
                ),
            ))
        } else {
            LocalOrderedTimedStoppedRecoveryAdvanceV1::Mismatch(Box::new(
                LocalOrderedTimedStopRecoveryMismatchV1 { stop, recovered },
            ))
        }
    }
}

pub type LocalOrderedManagedTimedSemanticSpawnResultV1<P, C> = Result<
    (
        LocalOrderedManagedTimedSemanticActorV1<P, C>,
        SemanticSessionActorSenderV1,
    ),
    Box<SemanticSessionActorBuildFailureV1<LocalOrderedTimedSemanticPortV1<P, C>>>,
>;

impl<P, C> LocalOrderedTimedSemanticPreparedV1<P, C>
where
    P: SemanticBridgeProviderPollV1 + Send + 'static,
    C: LocalOrderedMonotonicClockV1,
{
    pub fn spawn(
        self,
        limits: SemanticSessionActorLimitsV1,
    ) -> LocalOrderedTimedSemanticSpawnResultV1<P, C> {
        spawn_semantic_session_actor_v1(self.broker, self.port, limits)
    }

    pub fn spawn_managed(
        self,
        limits: SemanticSessionActorLimitsV1,
    ) -> LocalOrderedManagedTimedSemanticSpawnResultV1<P, C> {
        let (owner, sender, recovery) =
            spawn_semantic_session_actor_v1(self.broker, self.port, limits)?;
        Ok((
            LocalOrderedManagedTimedSemanticActorV1 {
                owner,
                recovery,
                refusal_attempts: 0,
                last_refusal: None,
                saturated: false,
            },
            sender,
        ))
    }

    pub fn into_cleanup(self) -> LocalOrderedTimedSemanticCleanupV1<P, C> {
        self.into_cleanup_with_evidence().0
    }

    pub fn into_cleanup_with_evidence(
        self,
    ) -> (
        LocalOrderedTimedSemanticCleanupV1<P, C>,
        LocalOrderedCleanupOriginEvidenceV1,
    ) {
        let evidence = LocalOrderedCleanupOriginEvidenceV1 {
            origin: LocalOrderedCleanupOriginV1::Unstarted,
            broker_usage: Some(self.broker.usage()),
            actor_usage: None,
            actor_limits: None,
            actor_stop: None,
            actor_recovery_join: LocalOrderedActorRecoveryJoinV1::NotApplicable,
        };
        drop(self.broker);
        (
            LocalOrderedTimedSemanticCleanupV1 { port: self.port },
            evidence,
        )
    }
}

impl<P, C> LocalOrderedTimedSemanticCleanupV1<P, C>
where
    P: SemanticBridgeProviderPollV1 + Send + 'static,
    C: LocalOrderedMonotonicClockV1,
{
    pub fn from_actor_build_failure(
        failure: SemanticSessionActorBuildFailureV1<LocalOrderedTimedSemanticPortV1<P, C>>,
    ) -> (Self, LocalOrderedCleanupOriginEvidenceV1) {
        let (reason, limits, broker, port) = failure.into_parts();
        let evidence = LocalOrderedCleanupOriginEvidenceV1 {
            origin: LocalOrderedCleanupOriginV1::ActorBuildRefused(reason),
            broker_usage: Some(broker.usage()),
            actor_usage: None,
            actor_limits: Some(limits),
            actor_stop: None,
            actor_recovery_join: LocalOrderedActorRecoveryJoinV1::NotApplicable,
        };
        drop(broker);
        (Self { port }, evidence)
    }

    pub fn from_actor_recovered(
        recovered: SemanticSessionActorRecoveredV1<LocalOrderedTimedSemanticPortV1<P, C>>,
    ) -> (Self, LocalOrderedCleanupOriginEvidenceV1) {
        Self::from_actor_recovered_with_stop(
            recovered,
            None,
            LocalOrderedActorRecoveryJoinV1::NotApplicable,
        )
    }

    fn from_actor_recovered_with_stop(
        recovered: SemanticSessionActorRecoveredV1<LocalOrderedTimedSemanticPortV1<P, C>>,
        actor_stop: Option<LocalOrderedActorStopEvidenceV1>,
        actor_recovery_join: LocalOrderedActorRecoveryJoinV1,
    ) -> (Self, LocalOrderedCleanupOriginEvidenceV1) {
        let (termination, actor_usage, broker, port) = recovered.into_parts();
        let evidence = LocalOrderedCleanupOriginEvidenceV1 {
            origin: LocalOrderedCleanupOriginV1::ActorRecovered(termination),
            broker_usage: Some(broker.usage()),
            actor_usage: Some(actor_usage),
            actor_limits: None,
            actor_stop,
            actor_recovery_join,
        };
        drop(broker);
        (Self { port }, evidence)
    }

    #[cfg(test)]
    pub(crate) fn from_recovered_port(port: LocalOrderedTimedSemanticPortV1<P, C>) -> Self {
        Self { port }
    }

    pub(crate) fn retire_deadlines(mut self) -> LocalOrderedDeadlineRetirementResultV1<P, C> {
        let receipt = self.port.scheduler.retirement_receipt();
        if receipt.disposition == LocalOrderedDeadlineDispositionV1::Unresolved {
            return Err(Box::new(LocalOrderedDeadlineRetirementFailureV1 {
                cleanup: self,
                receipt,
            }));
        }
        let LocalOrderedTimedSemanticPortV1 { base, .. } = self.port;
        Ok((LocalOrderedSemanticCleanupV1 { port: base }, receipt))
    }

    /// Explicitly abandons an inconsistent deadline graph. Exact unresolved
    /// correlation remains in the receipt, while the bridge provider advances
    /// only to forced cleanup.
    pub(crate) fn abandon_deadlines(
        mut self,
    ) -> (
        LocalOrderedSemanticCleanupV1<P>,
        LocalOrderedDeadlineRetirementReceiptV1,
    ) {
        let mut receipt = self.port.scheduler.retirement_receipt();
        receipt.disposition = LocalOrderedDeadlineDispositionV1::Abandoned;
        let LocalOrderedTimedSemanticPortV1 { base, .. } = self.port;
        (LocalOrderedSemanticCleanupV1 { port: base }, receipt)
    }
}

impl<P, C> LocalOrderedTimedCleanupOwnerV1<P, C>
where
    P: SemanticBridgeProviderPollV1 + Send + 'static,
    C: LocalOrderedMonotonicClockV1,
{
    pub fn from_unstarted(prepared: LocalOrderedTimedSemanticPreparedV1<P, C>) -> Self {
        let (cleanup, origin) = prepared.into_cleanup_with_evidence();
        Self {
            cleanup,
            origin,
            unresolved: None,
        }
    }

    fn from_managed_actor_recovered(
        recovered: SemanticSessionActorRecoveredV1<LocalOrderedTimedSemanticPortV1<P, C>>,
        stop: LocalOrderedActorStopEvidenceV1,
        join: LocalOrderedActorRecoveryJoinV1,
    ) -> Self {
        let (cleanup, origin) = LocalOrderedTimedSemanticCleanupV1::from_actor_recovered_with_stop(
            recovered,
            Some(stop),
            join,
        );
        Self {
            cleanup,
            origin,
            unresolved: None,
        }
    }

    pub fn from_actor_build_failure(
        failure: SemanticSessionActorBuildFailureV1<LocalOrderedTimedSemanticPortV1<P, C>>,
    ) -> Self {
        let (cleanup, origin) =
            LocalOrderedTimedSemanticCleanupV1::from_actor_build_failure(failure);
        Self {
            cleanup,
            origin,
            unresolved: None,
        }
    }

    pub fn from_actor_recovered(
        recovered: SemanticSessionActorRecoveredV1<LocalOrderedTimedSemanticPortV1<P, C>>,
    ) -> Self {
        let (cleanup, origin) = LocalOrderedTimedSemanticCleanupV1::from_actor_recovered(recovered);
        Self {
            cleanup,
            origin,
            unresolved: None,
        }
    }

    pub const fn unresolved(&self) -> Option<&LocalOrderedDeadlineRetirementReceiptV1> {
        self.unresolved.as_ref()
    }

    pub fn try_retire_deadlines(self) -> Result<LocalOrderedCleanupOwnerV1<P>, Box<Self>> {
        let Self {
            cleanup, origin, ..
        } = self;
        match cleanup.retire_deadlines() {
            Ok((cleanup, deadlines)) => Ok(LocalOrderedCleanupOwnerV1 {
                cleanup,
                origin,
                deadlines,
                client_usage: LocalOrderedClientRetirementUsageV1::default(),
            }),
            Err(failure) => {
                let LocalOrderedDeadlineRetirementFailureV1 { cleanup, receipt } = *failure;
                Err(Box::new(Self {
                    cleanup,
                    origin,
                    unresolved: Some(receipt),
                }))
            }
        }
    }

    pub fn abandon_deadlines(self) -> LocalOrderedCleanupOwnerV1<P> {
        let (cleanup, deadlines) = self.cleanup.abandon_deadlines();
        LocalOrderedCleanupOwnerV1 {
            cleanup,
            origin: self.origin,
            deadlines,
            client_usage: LocalOrderedClientRetirementUsageV1::default(),
        }
    }
}

#[derive(Clone, Copy)]
enum QualifiedLspInitializationPolicyV1<'a> {
    Generic,
    Pinned(&'a LspProviderProtocolProfileV1),
    RustAnalyzerStandalone(&'a LspProviderProtocolProfileV1),
}

impl<'a> QualifiedLspInitializationPolicyV1<'a> {
    fn protocol(self) -> Option<&'a LspProviderProtocolProfileV1> {
        match self {
            Self::Generic => None,
            Self::Pinned(protocol) | Self::RustAnalyzerStandalone(protocol) => Some(protocol),
        }
    }
}

fn qualified_workspace_initialization(
    qualified: &QualifiedProviderLaunchV1,
    limits: LspLimitsV1,
    policy: QualifiedLspInitializationPolicyV1<'_>,
) -> Option<LspInitializationProfileV1> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let path = qualified.profile.workspace_root.as_os_str().as_bytes();
    let mut uri = String::with_capacity(path.len().saturating_mul(3).saturating_add(7));
    uri.push_str("file://");
    for byte in path.iter().copied() {
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
    match policy {
        QualifiedLspInitializationPolicyV1::Generic => LspInitializationProfileV1::try_workspace(
            uri,
            "vellum-workspace".to_owned(),
            None,
            limits,
        ),
        QualifiedLspInitializationPolicyV1::Pinned(protocol) => {
            LspInitializationProfileV1::try_workspace_with_protocol(
                uri,
                "vellum-workspace".to_owned(),
                protocol.clone(),
                limits,
            )
        }
        QualifiedLspInitializationPolicyV1::RustAnalyzerStandalone(protocol) => {
            LspInitializationProfileV1::try_rust_analyzer_standalone(
                uri,
                "vellum-workspace".to_owned(),
                protocol.clone(),
                limits,
            )
        }
    }
}

/// Consumes one qualified process launch into the only public production path
/// for a fresh, deadline-disabled ordered LSP runtime. Construction closes the
/// service/session/adapter/frame bounds before launch, linearizes Vellum's
/// first initialize admission against supervised stdout acceptance, starts the
/// adapter exactly once, and installs its exact genesis projection.
#[allow(clippy::too_many_arguments)]
pub fn launch_supervised_ordered_lsp_without_deadlines_v1<G, R, U>(
    qualified: QualifiedProviderLaunchV1,
    generations: G,
    readiness: R,
    virtual_documents: U,
    lsp_limits: LspLimitsV1,
    semantic_limits: SemanticLimitsV1,
    client: SemanticClientRefV1,
    incarnation: SemanticSessionIncarnationV1,
    session_limits: SemanticSessionLimitsV1,
    service_limits: SemanticServiceLimitsV1,
) -> LocalSupervisedOrderedLspBuildResultV1<G, R, U>
where
    G: SemanticGenerationIssuerV1,
    R: LspIndexReadinessPortV1,
    U: LspVirtualDocumentIssuerV1,
{
    launch_supervised_ordered_lsp_without_deadlines_with_protocol_v1(
        qualified,
        generations,
        readiness,
        virtual_documents,
        lsp_limits,
        semantic_limits,
        client,
        incarnation,
        session_limits,
        service_limits,
        QualifiedLspInitializationPolicyV1::Generic,
    )
}

/// Pinned-provider counterpart to
/// [`launch_supervised_ordered_lsp_without_deadlines_v1`]. The caller retains
/// the immutable protocol profile; the initialization root remains derived
/// exclusively from the qualified launch workspace.
#[allow(clippy::too_many_arguments)]
pub fn launch_supervised_ordered_lsp_without_deadlines_pinned_v1<G, R, U>(
    qualified: QualifiedProviderLaunchV1,
    generations: G,
    readiness: R,
    virtual_documents: U,
    lsp_limits: LspLimitsV1,
    semantic_limits: SemanticLimitsV1,
    client: SemanticClientRefV1,
    incarnation: SemanticSessionIncarnationV1,
    session_limits: SemanticSessionLimitsV1,
    service_limits: SemanticServiceLimitsV1,
    protocol: &LspProviderProtocolProfileV1,
) -> LocalSupervisedOrderedLspBuildResultV1<G, R, U>
where
    G: SemanticGenerationIssuerV1,
    R: LspIndexReadinessPortV1,
    U: LspVirtualDocumentIssuerV1,
{
    launch_supervised_ordered_lsp_without_deadlines_with_protocol_v1(
        qualified,
        generations,
        readiness,
        virtual_documents,
        lsp_limits,
        semantic_limits,
        client,
        incarnation,
        session_limits,
        service_limits,
        QualifiedLspInitializationPolicyV1::Pinned(protocol),
    )
}

#[allow(clippy::too_many_arguments)]
fn launch_supervised_ordered_lsp_without_deadlines_with_protocol_v1<G, R, U>(
    qualified: QualifiedProviderLaunchV1,
    generations: G,
    readiness: R,
    virtual_documents: U,
    lsp_limits: LspLimitsV1,
    semantic_limits: SemanticLimitsV1,
    client: SemanticClientRefV1,
    incarnation: SemanticSessionIncarnationV1,
    session_limits: SemanticSessionLimitsV1,
    service_limits: SemanticServiceLimitsV1,
    policy: QualifiedLspInitializationPolicyV1<'_>,
) -> LocalSupervisedOrderedLspBuildResultV1<G, R, U>
where
    G: SemanticGenerationIssuerV1,
    R: LspIndexReadinessPortV1,
    U: LspVirtualDocumentIssuerV1,
{
    let required_frame = required_lsp_frame_bytes(session_limits, service_limits);
    if !lsp_configuration_align(lsp_limits, semantic_limits, session_limits, service_limits) {
        return Err(Box::new(LocalSupervisedOrderedLspBuildFailureV1 {
            reason: LocalSupervisedOrderedLspBuildRefusalV1::InvalidCompositionLimits,
            residue: LocalSupervisedOrderedLspBuildResidueV1::PreLaunch {
                qualified: Box::new(qualified),
                generations,
                readiness,
                virtual_documents,
            },
        }));
    }
    if policy.protocol().is_some_and(|protocol| {
        !protocol.matches_launch_profile_digest(&qualified.profile_identity().digest())
    }) {
        return Err(Box::new(LocalSupervisedOrderedLspBuildFailureV1 {
            reason: LocalSupervisedOrderedLspBuildRefusalV1::InvalidInitializationProfile,
            residue: LocalSupervisedOrderedLspBuildResidueV1::PreLaunch {
                qualified: Box::new(qualified),
                generations,
                readiness,
                virtual_documents,
            },
        }));
    }
    let Some(initialization) = qualified_workspace_initialization(&qualified, lsp_limits, policy)
    else {
        return Err(Box::new(LocalSupervisedOrderedLspBuildFailureV1 {
            reason: LocalSupervisedOrderedLspBuildRefusalV1::InvalidInitializationProfile,
            residue: LocalSupervisedOrderedLspBuildResidueV1::PreLaunch {
                qualified: Box::new(qualified),
                generations,
                readiness,
                virtual_documents,
            },
        }));
    };
    if required_frame.is_none_or(|required| qualified.profile.limits.max_frame_bytes < required) {
        return Err(Box::new(LocalSupervisedOrderedLspBuildFailureV1 {
            reason: LocalSupervisedOrderedLspBuildRefusalV1::InsufficientTransportFrame,
            residue: LocalSupervisedOrderedLspBuildResidueV1::PreLaunch {
                qualified: Box::new(qualified),
                generations,
                readiness,
                virtual_documents,
            },
        }));
    }
    let provenance = qualified.profile.provenance.clone();
    #[cfg(test)]
    let fail_after_fresh_claim = qualified
        .profile
        .environment
        .contains_key("VELLUM_TEST_FAIL_AFTER_FRESH_CLAIM");
    #[cfg(test)]
    let taint_after_fresh_claim = qualified
        .profile
        .environment
        .contains_key("VELLUM_TEST_TAINT_AFTER_FRESH_CLAIM");
    #[cfg(test)]
    let delay_before_fresh_claim = qualified
        .profile
        .environment
        .get("VELLUM_TEST_DELAY_BEFORE_FRESH_CLAIM_MILLIS")
        .and_then(|value| value.parse::<u64>().ok());
    let transport = match launch_qualified_local_lsp_owned_v1(qualified) {
        Ok(transport) => transport,
        Err(failure) => {
            let reason = failure.reason();
            return Err(Box::new(LocalSupervisedOrderedLspBuildFailureV1 {
                reason: LocalSupervisedOrderedLspBuildRefusalV1::Launch(reason),
                residue: LocalSupervisedOrderedLspBuildResidueV1::LaunchAttempted {
                    failure,
                    generations,
                    readiness,
                    virtual_documents,
                },
            }));
        }
    };
    #[cfg(test)]
    if let Some(delay) = delay_before_fresh_claim {
        std::thread::sleep(std::time::Duration::from_millis(delay));
    }
    let transport = match transport.try_claim_fresh() {
        Ok(transport) => transport,
        Err(failure) => {
            let reason = failure.reason();
            return Err(Box::new(LocalSupervisedOrderedLspBuildFailureV1 {
                reason: LocalSupervisedOrderedLspBuildRefusalV1::TransportFreshness(reason),
                residue: LocalSupervisedOrderedLspBuildResidueV1::TransportFreshness {
                    failure,
                    generations,
                    readiness,
                    virtual_documents,
                },
            }));
        }
    };
    #[cfg(test)]
    if fail_after_fresh_claim {
        transport
            .inner
            .shared
            .live
            .store(false, std::sync::atomic::Ordering::Release);
    }
    #[cfg(test)]
    if taint_after_fresh_claim {
        let shared = std::sync::Arc::clone(&transport.inner.shared);
        let _admission = shared.admission.lock().expect("semantic admission mutex");
        shared
            .stdout_freshness
            .store(super::STDOUT_TAINTED, std::sync::atomic::Ordering::Release);
    }
    let fresh_start_witness = transport.fresh_start_witness();
    let adapter = LspAdapterV1::try_new_with_profile(
        transport,
        generations,
        readiness,
        virtual_documents,
        LspAdapterProfileV1::try_new(provenance, lsp_limits, semantic_limits, initialization)
            .unwrap_or_else(|| unreachable!("preflight proved adapter profile limits")),
    )
    .unwrap_or_else(|| unreachable!("preflight proved adapter constructor limits"));
    let adapter = match adapter.try_start_fresh() {
        Ok(adapter) => adapter,
        Err(failure) => {
            let reason = match fresh_start_witness.refusal() {
                Some(reason) => LocalSupervisedOrderedLspBuildRefusalV1::TransportFreshness(reason),
                None => LocalSupervisedOrderedLspBuildRefusalV1::FreshStart(failure.reason()),
            };
            return Err(Box::new(LocalSupervisedOrderedLspBuildFailureV1 {
                reason,
                residue: LocalSupervisedOrderedLspBuildResidueV1::FreshStart(failure),
            }));
        }
    };
    match LocalOrderedSemanticPreparedV1::try_new_lsp_without_deadlines(
        adapter,
        client,
        incarnation,
        session_limits,
        service_limits,
    ) {
        Ok(prepared) => Ok(prepared),
        Err(failure) => {
            let reason = failure.reason();
            Err(Box::new(LocalSupervisedOrderedLspBuildFailureV1 {
                reason: LocalSupervisedOrderedLspBuildRefusalV1::Ordered(reason),
                residue: LocalSupervisedOrderedLspBuildResidueV1::Ordered(failure),
            }))
        }
    }
}

/// Timed production composition. The deadline policy is selected before the
/// prepared runtime is returned, so callers never receive an intermediate
/// deadline-disabled actor capability. Every base failure retains the clock;
/// every scheduler refusal retains both the prepared provider and clock.
#[allow(clippy::too_many_arguments)]
pub fn launch_supervised_ordered_lsp_with_deadlines_v1<G, R, U, C>(
    qualified: QualifiedProviderLaunchV1,
    generations: G,
    readiness: R,
    virtual_documents: U,
    lsp_limits: LspLimitsV1,
    semantic_limits: SemanticLimitsV1,
    client: SemanticClientRefV1,
    incarnation: SemanticSessionIncarnationV1,
    session_limits: SemanticSessionLimitsV1,
    service_limits: SemanticServiceLimitsV1,
    clock: C,
    deadline_policy: LocalOrderedDeadlinePolicyV1,
) -> LocalSupervisedOrderedTimedLspBuildResultV1<G, R, U, C>
where
    G: SemanticGenerationIssuerV1,
    R: LspIndexReadinessPortV1,
    U: LspVirtualDocumentIssuerV1,
    C: LocalOrderedMonotonicClockV1,
{
    launch_supervised_ordered_lsp_with_deadlines_and_protocol_v1(
        qualified,
        generations,
        readiness,
        virtual_documents,
        lsp_limits,
        semantic_limits,
        client,
        incarnation,
        session_limits,
        service_limits,
        clock,
        deadline_policy,
        QualifiedLspInitializationPolicyV1::Generic,
    )
}

/// Timed pinned-provider composition. The protocol profile is borrowed, while
/// all effect authority and deadline obligations remain in the affine result.
#[allow(clippy::too_many_arguments)]
pub fn launch_supervised_ordered_lsp_with_deadlines_pinned_v1<G, R, U, C>(
    qualified: QualifiedProviderLaunchV1,
    generations: G,
    readiness: R,
    virtual_documents: U,
    lsp_limits: LspLimitsV1,
    semantic_limits: SemanticLimitsV1,
    client: SemanticClientRefV1,
    incarnation: SemanticSessionIncarnationV1,
    session_limits: SemanticSessionLimitsV1,
    service_limits: SemanticServiceLimitsV1,
    clock: C,
    deadline_policy: LocalOrderedDeadlinePolicyV1,
    protocol: &LspProviderProtocolProfileV1,
) -> LocalSupervisedOrderedTimedLspBuildResultV1<G, R, U, C>
where
    G: SemanticGenerationIssuerV1,
    R: LspIndexReadinessPortV1,
    U: LspVirtualDocumentIssuerV1,
    C: LocalOrderedMonotonicClockV1,
{
    launch_supervised_ordered_lsp_with_deadlines_and_protocol_v1(
        qualified,
        generations,
        readiness,
        virtual_documents,
        lsp_limits,
        semantic_limits,
        client,
        incarnation,
        session_limits,
        service_limits,
        clock,
        deadline_policy,
        QualifiedLspInitializationPolicyV1::Pinned(protocol),
    )
}

/// Timed pinned rust-analyzer standalone composition. This explicitly selects
/// the adapter-owned closed initialization-options policy; generic pinned
/// launch remains unchanged. The consumed qualified launch supplies the only
/// workspace root, and `src/lib.rs` is bound before process launch.
#[allow(clippy::too_many_arguments)]
pub fn launch_supervised_ordered_lsp_with_deadlines_rust_analyzer_standalone_v1<G, R, U, C>(
    qualified: QualifiedProviderLaunchV1,
    generations: G,
    readiness: R,
    virtual_documents: U,
    lsp_limits: LspLimitsV1,
    semantic_limits: SemanticLimitsV1,
    client: SemanticClientRefV1,
    incarnation: SemanticSessionIncarnationV1,
    session_limits: SemanticSessionLimitsV1,
    service_limits: SemanticServiceLimitsV1,
    clock: C,
    deadline_policy: LocalOrderedDeadlinePolicyV1,
    protocol: &LspProviderProtocolProfileV1,
) -> LocalSupervisedOrderedTimedLspBuildResultV1<G, R, U, C>
where
    G: SemanticGenerationIssuerV1,
    R: LspIndexReadinessPortV1,
    U: LspVirtualDocumentIssuerV1,
    C: LocalOrderedMonotonicClockV1,
{
    launch_supervised_ordered_lsp_with_deadlines_and_protocol_v1(
        qualified,
        generations,
        readiness,
        virtual_documents,
        lsp_limits,
        semantic_limits,
        client,
        incarnation,
        session_limits,
        service_limits,
        clock,
        deadline_policy,
        QualifiedLspInitializationPolicyV1::RustAnalyzerStandalone(protocol),
    )
}

#[allow(clippy::too_many_arguments)]
fn launch_supervised_ordered_lsp_with_deadlines_and_protocol_v1<G, R, U, C>(
    qualified: QualifiedProviderLaunchV1,
    generations: G,
    readiness: R,
    virtual_documents: U,
    lsp_limits: LspLimitsV1,
    semantic_limits: SemanticLimitsV1,
    client: SemanticClientRefV1,
    incarnation: SemanticSessionIncarnationV1,
    session_limits: SemanticSessionLimitsV1,
    service_limits: SemanticServiceLimitsV1,
    clock: C,
    deadline_policy: LocalOrderedDeadlinePolicyV1,
    policy: QualifiedLspInitializationPolicyV1<'_>,
) -> LocalSupervisedOrderedTimedLspBuildResultV1<G, R, U, C>
where
    G: SemanticGenerationIssuerV1,
    R: LspIndexReadinessPortV1,
    U: LspVirtualDocumentIssuerV1,
    C: LocalOrderedMonotonicClockV1,
{
    let deadline_refusal = if !deadline_policy.is_valid() {
        Some(LocalOrderedDeadlineBuildRefusalV1::InvalidPolicy)
    } else if deadline_policy.max_deadlines < service_limits.max_pending {
        Some(LocalOrderedDeadlineBuildRefusalV1::InsufficientCapacity)
    } else {
        None
    };
    if let Some(reason) = deadline_refusal {
        return Err(Box::new(LocalSupervisedOrderedTimedLspBuildFailureV1 {
            reason: LocalSupervisedOrderedTimedLspBuildRefusalV1::Deadlines(reason),
            residue: LocalSupervisedOrderedTimedLspBuildResidueV1::PreLaunch {
                qualified: Box::new(qualified),
                generations,
                readiness,
                virtual_documents,
                clock,
            },
        }));
    }
    let prepared = match launch_supervised_ordered_lsp_without_deadlines_with_protocol_v1(
        qualified,
        generations,
        readiness,
        virtual_documents,
        lsp_limits,
        semantic_limits,
        client,
        incarnation,
        session_limits,
        service_limits,
        policy,
    ) {
        Ok(prepared) => prepared,
        Err(failure) => {
            let reason = failure.reason();
            return Err(Box::new(LocalSupervisedOrderedTimedLspBuildFailureV1 {
                reason: LocalSupervisedOrderedTimedLspBuildRefusalV1::Base(reason),
                residue: LocalSupervisedOrderedTimedLspBuildResidueV1::Base { failure, clock },
            }));
        }
    };
    match prepared.try_with_deadlines(clock, deadline_policy) {
        Ok(prepared) => Ok(prepared),
        Err(failure) => {
            let reason = failure.reason();
            Err(Box::new(LocalSupervisedOrderedTimedLspBuildFailureV1 {
                reason: LocalSupervisedOrderedTimedLspBuildRefusalV1::Deadlines(reason),
                residue: LocalSupervisedOrderedTimedLspBuildResidueV1::Deadlines(failure),
            }))
        }
    }
}

impl<T, G, R, U> LocalOrderedSemanticPreparedV1<LspFreshStartedAdapterV1<T, G, R, U>>
where
    T: LspTransportPortV1,
    G: SemanticGenerationIssuerV1,
    R: LspIndexReadinessPortV1,
    U: LspVirtualDocumentIssuerV1,
{
    /// Closes the production LSP adapter's result, document, aggregate-source,
    /// URI and framing bounds before installing it in the ordered service.
    /// The adapter is returned intact on every refusal. This consumes an
    /// already-owned adapter and grants no process-launch authority.
    pub fn try_new_lsp_without_deadlines(
        adapter: LspFreshStartedAdapterV1<T, G, R, U>,
        client: SemanticClientRefV1,
        incarnation: SemanticSessionIncarnationV1,
        session_limits: SemanticSessionLimitsV1,
        service_limits: SemanticServiceLimitsV1,
    ) -> LocalOrderedLspBuildResultV1<T, G, R, U> {
        if !lsp_limits_align(adapter.limit_observation(), session_limits, service_limits) {
            return Err(Box::new(LocalOrderedSemanticBuildFailureV1 {
                reason: LocalOrderedSemanticBuildRefusalV1::InvalidAdapterLimits,
                residue: LocalOrderedSemanticBuildResidueV1::UninstalledProvider(adapter),
            }));
        }
        Self::try_new_injected_provider_without_deadlines(
            adapter,
            client,
            incarnation,
            session_limits,
            service_limits,
        )
    }
}

pub type LocalOrderedSemanticActorV1<P> = (
    SemanticSessionActorOwnerV1,
    SemanticSessionActorSenderV1,
    SemanticSessionActorRecoveryV1<LocalOrderedSemanticPortV1<P>>,
);

/// Same-spawn owner for the actor control capability and its independent
/// recovery capability. Neither half can be paired with another actor's stop
/// receipt because this product is created only by `spawn_managed`.
#[must_use = "managed actor ownership must stop and recover into cleanup"]
pub struct LocalOrderedManagedSemanticActorV1<P> {
    owner: SemanticSessionActorOwnerV1,
    recovery: SemanticSessionActorRecoveryV1<LocalOrderedSemanticPortV1<P>>,
    refusal_attempts: u64,
    last_refusal: Option<SemanticSessionActorStopReceiptV1>,
    saturated: bool,
}

impl<P> fmt::Debug for LocalOrderedManagedSemanticActorV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedManagedSemanticActorV1")
            .field("refusal_attempts", &self.refusal_attempts)
            .field("last_refusal", &self.last_refusal)
            .field("saturated", &self.saturated)
            .field("owner", &"<owned>")
            .field("recovery", &"<owned>")
            .finish()
    }
}

impl<P> LocalOrderedManagedSemanticActorV1<P> {
    pub fn try_event(
        &self,
        event: SemanticSessionActorExternalEventV1,
    ) -> SemanticSessionActorEventAdmissionV1 {
        self.owner.try_event(event)
    }

    pub fn try_drive_provider(
        &self,
    ) -> Result<SemanticSessionActorControlTicketV1, SemanticSessionActorControlRefusalV1> {
        self.owner.try_drive_provider()
    }

    pub fn try_drive_expiry(
        &self,
    ) -> Result<SemanticSessionActorControlTicketV1, SemanticSessionActorControlRefusalV1> {
        self.owner.try_drive_expiry()
    }

    pub fn try_drive_retirement(
        &self,
    ) -> Result<SemanticSessionActorControlTicketV1, SemanticSessionActorControlRefusalV1> {
        self.owner.try_drive_retirement()
    }

    pub fn try_rotate_incarnation(
        &self,
        next: SemanticSessionIncarnationV1,
    ) -> Result<SemanticSessionActorControlTicketV1, SemanticSessionActorControlRefusalV1> {
        self.owner.try_rotate_incarnation(next)
    }

    pub fn begin_drain(&self) -> bool {
        self.owner.begin_drain()
    }

    pub fn phase(&self) -> SemanticSessionActorPhaseV1 {
        self.owner.phase()
    }

    pub fn usage(&self) -> SemanticSessionActorUsageV1 {
        self.owner.usage()
    }

    pub fn try_begin_stop(
        self,
    ) -> Result<LocalOrderedJoinedStopTicketV1<P>, Box<LocalOrderedManagedStopStartFailureV1<P>>>
    {
        let Self {
            owner,
            recovery,
            refusal_attempts,
            last_refusal,
            saturated,
        } = self;
        match owner.try_begin_stop() {
            Ok(ticket) => Ok(LocalOrderedJoinedStopTicketV1 {
                ticket,
                recovery,
                refusal_attempts,
                last_refusal,
                saturated,
            }),
            Err(failure) => Err(Box::new(LocalOrderedManagedStopStartFailureV1 {
                reason: failure.reason(),
                actor: LocalOrderedManagedSemanticActorV1 {
                    owner: failure.into_owner(),
                    recovery,
                    refusal_attempts,
                    last_refusal,
                    saturated,
                },
            })),
        }
    }

    /// A worker may fail before stop admission (for example, after a provider
    /// poll failure). This is a distinct terminal witness, never a synthetic
    /// stop receipt. Recovery must later confirm `Failed` before cleanup can
    /// advance.
    pub fn try_recover_failed(self) -> Result<LocalOrderedStoppedRecoveryOwnerV1<P>, Box<Self>> {
        if self.owner.phase() != SemanticSessionActorPhaseV1::Failed {
            return Err(Box::new(self));
        }
        let Self {
            owner,
            recovery,
            refusal_attempts,
            last_refusal,
            saturated,
        } = self;
        let usage = owner.usage();
        drop(owner);
        Ok(LocalOrderedStoppedRecoveryOwnerV1 {
            stop: LocalOrderedActorStopEvidenceV1 {
                terminal: LocalOrderedActorTerminalStopV1::FailedBeforeStop { usage },
                refusal_attempts,
                last_refusal,
                saturated,
            },
            recovery,
        })
    }
}

#[must_use = "failed managed stop admission retains the same-spawn actor product"]
pub struct LocalOrderedManagedStopStartFailureV1<P> {
    reason: SemanticSessionActorControlRefusalV1,
    actor: LocalOrderedManagedSemanticActorV1<P>,
}

impl<P> LocalOrderedManagedStopStartFailureV1<P> {
    pub const fn reason(&self) -> SemanticSessionActorControlRefusalV1 {
        self.reason
    }

    pub fn into_actor(self) -> LocalOrderedManagedSemanticActorV1<P> {
        self.actor
    }
}

impl<P> fmt::Debug for LocalOrderedManagedStopStartFailureV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedManagedStopStartFailureV1")
            .field("reason", &self.reason)
            .field("actor", &"<owned>")
            .finish()
    }
}

#[must_use = "joined stop ticket must resolve, retry, or explicitly abandon"]
pub struct LocalOrderedJoinedStopTicketV1<P> {
    ticket: SemanticSessionActorStopTicketV1,
    recovery: SemanticSessionActorRecoveryV1<LocalOrderedSemanticPortV1<P>>,
    refusal_attempts: u64,
    last_refusal: Option<SemanticSessionActorStopReceiptV1>,
    saturated: bool,
}

impl<P> fmt::Debug for LocalOrderedJoinedStopTicketV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedJoinedStopTicketV1")
            .field("refusal_attempts", &self.refusal_attempts)
            .field("last_refusal", &self.last_refusal)
            .field("saturated", &self.saturated)
            .field("ticket", &"<owned>")
            .field("recovery", &"<owned>")
            .finish()
    }
}

#[must_use = "resolved managed stop evidence must advance to recovery"]
pub enum LocalOrderedManagedStopResolutionV1<P> {
    Terminal(LocalOrderedStoppedRecoveryOwnerV1<P>),
    Refused(LocalOrderedManagedStopRefusalV1<P>),
}

impl<P> fmt::Debug for LocalOrderedManagedStopResolutionV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Terminal(_) => "Terminal(<owned>)",
            Self::Refused(_) => "Refused(<owned>)",
        })
    }
}

#[must_use = "stop refusal retains exact receipt and same-spawn actor ownership"]
pub struct LocalOrderedManagedStopRefusalV1<P> {
    receipt: SemanticSessionActorStopReceiptV1,
    actor: LocalOrderedManagedSemanticActorV1<P>,
}

impl<P> LocalOrderedManagedStopRefusalV1<P> {
    pub const fn receipt(&self) -> SemanticSessionActorStopReceiptV1 {
        self.receipt
    }

    pub fn into_actor(self) -> LocalOrderedManagedSemanticActorV1<P> {
        self.actor
    }
}

impl<P> fmt::Debug for LocalOrderedManagedStopRefusalV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedManagedStopRefusalV1")
            .field("receipt", &self.receipt)
            .field("actor", &"<owned>")
            .finish()
    }
}

impl<P> LocalOrderedJoinedStopTicketV1<P> {
    pub fn try_complete(self) -> Result<LocalOrderedManagedStopResolutionV1<P>, Box<Self>> {
        let Self {
            ticket,
            recovery,
            mut refusal_attempts,
            mut last_refusal,
            mut saturated,
        } = self;
        match ticket.try_complete() {
            Err(ticket) => Err(Box::new(Self {
                ticket,
                recovery,
                refusal_attempts,
                last_refusal,
                saturated,
            })),
            Ok(SemanticSessionActorStopResolutionV1::Clean(receipt)) => {
                debug_assert_eq!(
                    receipt.disposition(),
                    SemanticSessionActorStopDispositionV1::Clean
                );
                Ok(LocalOrderedManagedStopResolutionV1::Terminal(
                    Self::recovery_owner(
                        recovery,
                        refusal_attempts,
                        last_refusal,
                        saturated,
                        LocalOrderedActorTerminalStopV1::Clean(receipt),
                    ),
                ))
            }
            Ok(SemanticSessionActorStopResolutionV1::OutcomeUnknown(receipt)) => {
                debug_assert_eq!(
                    receipt.disposition(),
                    SemanticSessionActorStopDispositionV1::OutcomeUnknown
                );
                Ok(LocalOrderedManagedStopResolutionV1::Terminal(
                    Self::recovery_owner(
                        recovery,
                        refusal_attempts,
                        last_refusal,
                        saturated,
                        LocalOrderedActorTerminalStopV1::OutcomeUnknown(receipt),
                    ),
                ))
            }
            Ok(SemanticSessionActorStopResolutionV1::Refused { receipt, owner }) => {
                let before = refusal_attempts;
                refusal_attempts = before.saturating_add(1);
                saturated |= before == u64::MAX;
                last_refusal = Some(receipt);
                Ok(LocalOrderedManagedStopResolutionV1::Refused(
                    LocalOrderedManagedStopRefusalV1 {
                        receipt,
                        actor: LocalOrderedManagedSemanticActorV1 {
                            owner,
                            recovery,
                            refusal_attempts,
                            last_refusal,
                            saturated,
                        },
                    },
                ))
            }
        }
    }

    pub fn abandon(self) -> LocalOrderedStoppedRecoveryOwnerV1<P> {
        let Self {
            ticket,
            recovery,
            refusal_attempts,
            last_refusal,
            saturated,
        } = self;
        let abandonment = ticket.abandon();
        Self::recovery_owner(
            recovery,
            refusal_attempts,
            last_refusal,
            saturated,
            LocalOrderedActorTerminalStopV1::Abandoned(abandonment),
        )
    }

    fn recovery_owner(
        recovery: SemanticSessionActorRecoveryV1<LocalOrderedSemanticPortV1<P>>,
        refusal_attempts: u64,
        last_refusal: Option<SemanticSessionActorStopReceiptV1>,
        saturated: bool,
        terminal: LocalOrderedActorTerminalStopV1,
    ) -> LocalOrderedStoppedRecoveryOwnerV1<P> {
        LocalOrderedStoppedRecoveryOwnerV1 {
            stop: LocalOrderedActorStopEvidenceV1 {
                terminal,
                refusal_attempts,
                last_refusal,
                saturated,
            },
            recovery,
        }
    }
}

#[must_use = "terminal stop evidence must recover the same-spawn runtime"]
pub struct LocalOrderedStoppedRecoveryOwnerV1<P> {
    stop: LocalOrderedActorStopEvidenceV1,
    recovery: SemanticSessionActorRecoveryV1<LocalOrderedSemanticPortV1<P>>,
}

impl<P> LocalOrderedStoppedRecoveryOwnerV1<P> {
    pub const fn stop(&self) -> LocalOrderedActorStopEvidenceV1 {
        self.stop
    }
}

impl<P> fmt::Debug for LocalOrderedStoppedRecoveryOwnerV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedStoppedRecoveryOwnerV1")
            .field("stop", &self.stop)
            .field("recovery", &"<owned>")
            .finish()
    }
}

#[must_use = "recovery advance retains pending or mismatched cleanup authority"]
pub enum LocalOrderedStoppedRecoveryAdvanceV1<P> {
    Pending(Box<LocalOrderedStoppedRecoveryOwnerV1<P>>),
    Ready(Box<LocalOrderedCleanupOwnerV1<P>>),
    Mismatch(Box<LocalOrderedStopRecoveryMismatchV1<P>>),
}

impl<P> fmt::Debug for LocalOrderedStoppedRecoveryAdvanceV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Pending(_) => "Pending(<owned>)",
            Self::Ready(_) => "Ready(<owned>)",
            Self::Mismatch(_) => "Mismatch(<owned>)",
        })
    }
}

/// Impossible under the managed actor contract, but still a cleanup-bearing
/// residue rather than an assertion or a false successful join.
#[must_use = "stop/recovery mismatch retains exact cleanup authority"]
pub struct LocalOrderedStopRecoveryMismatchV1<P> {
    stop: LocalOrderedActorStopEvidenceV1,
    recovered: SemanticSessionActorRecoveredV1<LocalOrderedSemanticPortV1<P>>,
}

impl<P> LocalOrderedStopRecoveryMismatchV1<P>
where
    P: SemanticProviderPortV1,
{
    pub const fn stop(&self) -> LocalOrderedActorStopEvidenceV1 {
        self.stop
    }

    pub fn termination(&self) -> SemanticSessionActorTerminationV1 {
        self.recovered.termination()
    }

    pub fn into_cleanup(self) -> LocalOrderedCleanupOwnerV1<P> {
        LocalOrderedCleanupOwnerV1::from_managed_actor_recovered(
            self.recovered,
            self.stop,
            LocalOrderedActorRecoveryJoinV1::Mismatch,
        )
    }
}

impl<P> fmt::Debug for LocalOrderedStopRecoveryMismatchV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalOrderedStopRecoveryMismatchV1")
            .field("stop", &self.stop)
            .field("recovered", &"<owned>")
            .finish()
    }
}

impl<P> LocalOrderedStoppedRecoveryOwnerV1<P>
where
    P: SemanticProviderPortV1,
{
    pub fn try_advance(self) -> LocalOrderedStoppedRecoveryAdvanceV1<P> {
        let Self { stop, recovery } = self;
        let recovered = match recovery.try_recover() {
            Ok(recovered) => recovered,
            Err(recovery) => {
                return LocalOrderedStoppedRecoveryAdvanceV1::Pending(Box::new(Self {
                    stop,
                    recovery,
                }));
            }
        };
        // Clean is affirmative termination evidence and therefore must agree
        // with recovery. OutcomeUnknown and Abandoned deliberately make no
        // termination claim, so either recovered terminal class is lawful.
        let compatible = actor_stop_recovery_compatible(stop.terminal, recovered.termination());
        if compatible {
            LocalOrderedStoppedRecoveryAdvanceV1::Ready(Box::new(
                LocalOrderedCleanupOwnerV1::from_managed_actor_recovered(
                    recovered,
                    stop,
                    LocalOrderedActorRecoveryJoinV1::Compatible,
                ),
            ))
        } else {
            LocalOrderedStoppedRecoveryAdvanceV1::Mismatch(Box::new(
                LocalOrderedStopRecoveryMismatchV1 { stop, recovered },
            ))
        }
    }
}

pub type LocalOrderedSemanticSpawnResultV1<P> = Result<
    LocalOrderedSemanticActorV1<P>,
    Box<SemanticSessionActorBuildFailureV1<LocalOrderedSemanticPortV1<P>>>,
>;

pub type LocalOrderedManagedSemanticSpawnResultV1<P> = Result<
    (
        LocalOrderedManagedSemanticActorV1<P>,
        SemanticSessionActorSenderV1,
    ),
    Box<SemanticSessionActorBuildFailureV1<LocalOrderedSemanticPortV1<P>>>,
>;

impl<P> LocalOrderedSemanticPreparedV1<P>
where
    P: SemanticBridgeProviderPollV1 + Send + 'static,
{
    pub fn spawn(
        self,
        limits: SemanticSessionActorLimitsV1,
    ) -> LocalOrderedSemanticSpawnResultV1<P> {
        spawn_semantic_session_actor_v1(self.broker, self.port, limits)
    }

    pub fn spawn_managed(
        self,
        limits: SemanticSessionActorLimitsV1,
    ) -> LocalOrderedManagedSemanticSpawnResultV1<P> {
        let (owner, sender, recovery) =
            spawn_semantic_session_actor_v1(self.broker, self.port, limits)?;
        Ok((
            LocalOrderedManagedSemanticActorV1 {
                owner,
                recovery,
                refusal_attempts: 0,
                last_refusal: None,
                saturated: false,
            },
            sender,
        ))
    }
}

impl<P> SemanticBrokerEffectPortV1 for LocalOrderedSemanticPortV1<P>
where
    P: SemanticBridgeProviderPollV1 + Send + 'static,
{
    fn obligations(&self) -> SemanticBrokerEffectObligationsV1 {
        SemanticBrokerEffectObligationsV1 {
            request_deadlines: 0,
            due_deadlines: 0,
            poisoned: false,
            consistent: true,
        }
    }

    fn open(
        &mut self,
        snapshot: &SemanticSessionSnapshotV1,
    ) -> Result<SemanticBrokerOpenReportV1, SemanticBrokerEffectFailureV1> {
        let snapshot = self.snapshot(snapshot)?;
        let report = self.service.open_snapshot(self.client, snapshot);
        let translated = self.translate(&report)?;
        let outcome = match report.outcome() {
            SemanticBridgeOpenAdmissionV1::Accepted => SemanticBrokerAdmissionOutcomeV1::Accepted,
            SemanticBridgeOpenAdmissionV1::Unavailable => {
                SemanticBrokerAdmissionOutcomeV1::Unavailable
            }
            SemanticBridgeOpenAdmissionV1::Refused(refusal) => {
                SemanticBrokerAdmissionOutcomeV1::Refused(map_provider_refusal(*refusal)?)
            }
        };
        self.commit(&translated);
        Ok(SemanticBrokerOpenReportV1 {
            provider_before: translated.provider_before,
            pre_provider: translated.pre_provider,
            outcome,
            post_provider: translated.post_provider,
        })
    }

    fn change(
        &mut self,
        snapshot: &SemanticSessionSnapshotV1,
    ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
        let snapshot = self.snapshot(snapshot)?;
        let report = self.service.change_snapshot(self.client, snapshot);
        let translated = self.translate(&report)?;
        let outcome = match report.outcome() {
            SemanticBridgeChangeAdmissionV1::Accepted {
                source_invalidations,
            } if source_invalidations.cause() == SemanticInvalidationCauseV1::SourceChanged => {
                SemanticBrokerSourceAdmissionOutcomeV1::Accepted(source_invalidations.clone())
            }
            SemanticBridgeChangeAdmissionV1::Accepted { .. } => {
                return Err(SemanticBrokerEffectFailureV1::Poisoned);
            }
            SemanticBridgeChangeAdmissionV1::Unavailable => {
                SemanticBrokerSourceAdmissionOutcomeV1::Unavailable
            }
            SemanticBridgeChangeAdmissionV1::Refused(refusal) => {
                SemanticBrokerSourceAdmissionOutcomeV1::Refused(map_provider_refusal(*refusal)?)
            }
        };
        self.commit(&translated);
        Ok(SemanticBrokerSourceReportV1 {
            provider_before: translated.provider_before,
            pre_provider: translated.pre_provider,
            outcome,
            post_provider: translated.post_provider,
        })
    }

    fn close(
        &mut self,
        target: SemanticTargetV1,
    ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
        let report = self.service.close_snapshot(self.client, target);
        let translated = self.translate(&report)?;
        let outcome = match report.outcome() {
            SemanticBridgeCloseAdmissionV1::Accepted {
                source_invalidations,
            } if source_invalidations.cause() == SemanticInvalidationCauseV1::SourceClosed => {
                SemanticBrokerSourceAdmissionOutcomeV1::Accepted(source_invalidations.clone())
            }
            SemanticBridgeCloseAdmissionV1::Accepted { .. } => {
                return Err(SemanticBrokerEffectFailureV1::Poisoned);
            }
            SemanticBridgeCloseAdmissionV1::Unavailable => {
                SemanticBrokerSourceAdmissionOutcomeV1::Unavailable
            }
            SemanticBridgeCloseAdmissionV1::Refused(refusal) => {
                SemanticBrokerSourceAdmissionOutcomeV1::Refused(map_provider_refusal(*refusal)?)
            }
        };
        self.commit(&translated);
        Ok(SemanticBrokerSourceReportV1 {
            provider_before: translated.provider_before,
            pre_provider: translated.pre_provider,
            outcome,
            post_provider: translated.post_provider,
        })
    }

    fn submit(
        &mut self,
        request: &SemanticRequestV1,
    ) -> Result<SemanticBrokerSubmissionReportV1, SemanticBrokerEffectFailureV1> {
        let report = self.service.submit(self.client, request.clone());
        let translated = self.translate(&report)?;
        let outcome = match report.outcome() {
            SemanticSubmitOutcomeV1::Accepted => SemanticBrokerSubmissionOutcomeV1::Accepted,
            SemanticSubmitOutcomeV1::OutcomeUnknown => {
                return Err(SemanticBrokerEffectFailureV1::Poisoned);
            }
            SemanticSubmitOutcomeV1::Unsupported(kind) => {
                SemanticBrokerSubmissionOutcomeV1::Unsupported(*kind)
            }
            SemanticSubmitOutcomeV1::Stale => SemanticBrokerSubmissionOutcomeV1::Stale,
            SemanticSubmitOutcomeV1::Unavailable => SemanticBrokerSubmissionOutcomeV1::Unavailable,
            SemanticSubmitOutcomeV1::Refused(refusal) => {
                SemanticBrokerSubmissionOutcomeV1::Refused(map_provider_refusal(*refusal)?)
            }
        };
        self.commit(&translated);
        Ok(SemanticBrokerSubmissionReportV1 {
            provider_before: translated.provider_before,
            pre_provider: translated.pre_provider,
            outcome,
            post_provider: translated.post_provider,
        })
    }

    fn cancel(
        &mut self,
        request: SemanticRequestRefV1,
    ) -> Result<SemanticBrokerCancellationReportV1, SemanticBrokerEffectFailureV1> {
        let report = self.service.cancel(self.client, request);
        let translated = self.translate(&report)?;
        let outcome = match report.outcome() {
            SemanticBridgeCancelActionV1::Cancelled
            | SemanticBridgeCancelActionV1::SatisfiedByProviderTransition => {
                SemanticBrokerCancellationOutcomeV1::Accepted
            }
            SemanticBridgeCancelActionV1::NotPending | SemanticBridgeCancelActionV1::Refused(_) => {
                return Err(SemanticBrokerEffectFailureV1::Poisoned);
            }
        };
        self.commit(&translated);
        Ok(SemanticBrokerCancellationReportV1 {
            provider_before: translated.provider_before,
            pre_provider: translated.pre_provider,
            outcome,
            post_provider: translated.post_provider,
        })
    }

    fn drive_provider(
        &mut self,
    ) -> Result<SemanticBrokerProviderDriveV1, SemanticBrokerEffectFailureV1> {
        let report = self.service.poll_once();
        if let SemanticBridgePollOutcomeV1::Failure(failure) = report.outcome() {
            if report.post_provider().is_some() {
                return Err(SemanticBrokerEffectFailureV1::Poisoned);
            }
            let translated = self.translate(&report)?;
            self.commit(&translated);
            return Ok(SemanticBrokerProviderDriveV1::Failed(Box::new(
                SemanticBrokerProviderDriveFailureReportV1 {
                    provider_before: translated.provider_before,
                    pre_provider: translated.pre_provider,
                    failure: map_poll_failure_class(*failure),
                },
            )));
        }
        let translated = self.translate(&report)?;
        let disposition = match report.outcome() {
            SemanticBridgePollOutcomeV1::NoEvent => {
                SemanticBrokerProviderDriveDispositionV1::NoEvent
            }
            SemanticBridgePollOutcomeV1::Completion(SemanticBridgePollDispositionV1::Delivery(
                delivery,
            )) => SemanticBrokerProviderDriveDispositionV1::Delivery {
                internal_request: delivery.request,
                outcome: map_physical_delivery(&delivery.outcome)?,
            },
            SemanticBridgePollOutcomeV1::Completion(SemanticBridgePollDispositionV1::Late {
                request,
            }) => SemanticBrokerProviderDriveDispositionV1::Late {
                internal_request: *request,
            },
            SemanticBridgePollOutcomeV1::Completion(SemanticBridgePollDispositionV1::Orphan {
                request,
            }) => SemanticBrokerProviderDriveDispositionV1::Orphan {
                internal_request: *request,
            },
            SemanticBridgePollOutcomeV1::Failure(_) => unreachable!("handled above"),
        };
        self.commit(&translated);
        Ok(SemanticBrokerProviderDriveV1::Completed(Box::new(
            SemanticBrokerProviderDriveReportV1 {
                provider_before: translated.provider_before,
                pre_provider: translated.pre_provider,
                disposition,
                post_provider: translated.post_provider,
            },
        )))
    }

    fn drive_expiry(
        &mut self,
    ) -> Result<SemanticBrokerExpiryDriveV1, SemanticBrokerEffectFailureV1> {
        // This constructor exposes no deadline-admission surface, so the
        // scheduler lattice is the singleton Disabled state. A future timed
        // composition must use a distinct type owning scheduler receipts.
        Ok(SemanticBrokerExpiryDriveV1::NoDue)
    }
}

impl<P, C> LocalOrderedTimedSemanticPortV1<P, C>
where
    P: SemanticBridgeProviderPollV1 + Send + 'static,
    C: LocalOrderedMonotonicClockV1,
{
    pub fn deadline_usage(&self) -> LocalOrderedDeadlineUsageV1 {
        self.scheduler.usage
    }

    pub fn try_replace_provider(
        &mut self,
        candidate: P,
    ) -> Result<LocalOrderedProviderReplacementV1<P>, LocalOrderedProviderReplacementFailureV1<P>>
    {
        if self.scheduler.ensure_consistent().is_err() {
            return Err(LocalOrderedProviderReplacementFailureV1::Preflight {
                reason: LocalOrderedProviderReplacementRefusalV1::SchedulerPoisoned,
                candidate,
            });
        }
        let replacement = self.base.try_replace_provider(candidate)?;
        self.clear_report_transitions(
            replacement.report.pre_provider.as_ref(),
            replacement.report.post_provider.as_ref(),
        );
        Ok(replacement)
    }

    fn clear_transition(&mut self, transition: Option<&SemanticBrokerProviderTransitionV1>) {
        if let Some(transition) = transition {
            let requests: Vec<_> = transition.invalidations.iter().copied().collect();
            for request in requests {
                self.scheduler.clear(request, 0);
            }
        }
    }

    fn clear_source_outcome(&mut self, outcome: &SemanticBrokerSourceAdmissionOutcomeV1) {
        if let SemanticBrokerSourceAdmissionOutcomeV1::Accepted(receipt) = outcome {
            let requests: Vec<_> = receipt.iter().copied().collect();
            for request in requests {
                self.scheduler.clear(request, 1);
            }
        }
    }

    fn clear_report_transitions(
        &mut self,
        pre: Option<&SemanticBrokerProviderTransitionV1>,
        post: Option<&SemanticBrokerProviderTransitionV1>,
    ) {
        self.clear_transition(pre);
        self.clear_transition(post);
    }
}

impl<P, C> SemanticBrokerEffectPortV1 for LocalOrderedTimedSemanticPortV1<P, C>
where
    P: SemanticBridgeProviderPollV1 + Send + 'static,
    C: LocalOrderedMonotonicClockV1,
{
    fn obligations(&self) -> SemanticBrokerEffectObligationsV1 {
        SemanticBrokerEffectObligationsV1 {
            request_deadlines: self.scheduler.by_request.len(),
            due_deadlines: self.scheduler.by_due.len(),
            poisoned: self.scheduler.poisoned,
            consistent: self.scheduler.graph_is_consistent(),
        }
    }

    fn open(
        &mut self,
        snapshot: &SemanticSessionSnapshotV1,
    ) -> Result<SemanticBrokerOpenReportV1, SemanticBrokerEffectFailureV1> {
        self.scheduler.ensure_consistent()?;
        let report = self.base.open(snapshot)?;
        self.clear_report_transitions(report.pre_provider.as_ref(), report.post_provider.as_ref());
        Ok(report)
    }

    fn change(
        &mut self,
        snapshot: &SemanticSessionSnapshotV1,
    ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
        self.scheduler.ensure_consistent()?;
        let report = self.base.change(snapshot)?;
        self.clear_report_transitions(report.pre_provider.as_ref(), report.post_provider.as_ref());
        self.clear_source_outcome(&report.outcome);
        Ok(report)
    }

    fn close(
        &mut self,
        target: SemanticTargetV1,
    ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
        self.scheduler.ensure_consistent()?;
        let report = self.base.close(target)?;
        self.clear_report_transitions(report.pre_provider.as_ref(), report.post_provider.as_ref());
        self.clear_source_outcome(&report.outcome);
        Ok(report)
    }

    fn submit(
        &mut self,
        request: &SemanticRequestV1,
    ) -> Result<SemanticBrokerSubmissionReportV1, SemanticBrokerEffectFailureV1> {
        if self.scheduler.by_request.contains_key(&request.request) {
            return Err(SemanticBrokerEffectFailureV1::Poisoned);
        }
        let due = self.scheduler.reserve_due()?;
        let install = self.scheduler.prepare_install(request.request, due)?;
        let report = self.base.submit(request)?;
        self.clear_report_transitions(report.pre_provider.as_ref(), report.post_provider.as_ref());
        if matches!(report.outcome, SemanticBrokerSubmissionOutcomeV1::Accepted)
            && !self.scheduler.by_request.contains_key(&request.request)
        {
            let invalidated_after_acceptance =
                report.post_provider.as_ref().is_some_and(|transition| {
                    transition
                        .invalidations
                        .iter()
                        .any(|id| *id == request.request)
                });
            if !invalidated_after_acceptance {
                self.scheduler.commit_install(install);
            }
        }
        Ok(report)
    }

    fn cancel(
        &mut self,
        request: SemanticRequestRefV1,
    ) -> Result<SemanticBrokerCancellationReportV1, SemanticBrokerEffectFailureV1> {
        self.scheduler.ensure_consistent()?;
        let report = self.base.cancel(request)?;
        self.clear_report_transitions(report.pre_provider.as_ref(), report.post_provider.as_ref());
        self.scheduler.clear(request, 2);
        Ok(report)
    }

    fn drive_provider(
        &mut self,
    ) -> Result<SemanticBrokerProviderDriveV1, SemanticBrokerEffectFailureV1> {
        self.scheduler.ensure_consistent()?;
        let drive = self.base.drive_provider()?;
        match &drive {
            SemanticBrokerProviderDriveV1::Completed(report) => {
                self.clear_report_transitions(
                    report.pre_provider.as_ref(),
                    report.post_provider.as_ref(),
                );
                if let SemanticBrokerProviderDriveDispositionV1::Delivery {
                    internal_request, ..
                } = report.disposition
                {
                    self.scheduler.clear(internal_request, 3);
                }
            }
            SemanticBrokerProviderDriveV1::Failed(report) => {
                self.clear_transition(report.pre_provider.as_ref());
            }
        }
        Ok(drive)
    }

    fn drive_expiry(
        &mut self,
    ) -> Result<SemanticBrokerExpiryDriveV1, SemanticBrokerEffectFailureV1> {
        let Some(request) = self.scheduler.earliest_due()? else {
            return Ok(SemanticBrokerExpiryDriveV1::NoDue);
        };
        let report = self.base.expire(request)?;
        self.clear_report_transitions(report.pre_provider.as_ref(), report.post_provider.as_ref());
        let cleared = self.scheduler.clear(request, 4);
        debug_assert!(cleared, "due request vanished after prevalidated expiry");
        match report.outcome {
            SemanticBrokerExpiryOutcomeV1::Expired => deadline_bump(
                &mut self.scheduler.usage.expired,
                &mut self.scheduler.usage.saturated,
            ),
            SemanticBrokerExpiryOutcomeV1::SatisfiedByProviderTransition => deadline_bump(
                &mut self.scheduler.usage.transition_satisfied,
                &mut self.scheduler.usage.saturated,
            ),
            SemanticBrokerExpiryOutcomeV1::NotPending => deadline_bump(
                &mut self.scheduler.usage.not_pending,
                &mut self.scheduler.usage.saturated,
            ),
        }
        Ok(SemanticBrokerExpiryDriveV1::Due(Box::new(report)))
    }
}

struct TranslatedTransitions {
    provider_before: SemanticBrokerProviderBindingV1,
    pre_provider: Option<SemanticBrokerProviderTransitionV1>,
    post_provider: Option<SemanticBrokerProviderTransitionV1>,
    settled: SemanticBrokerProviderBindingV1,
}

fn limits_align(session: SemanticSessionLimitsV1, service: SemanticServiceLimitsV1) -> bool {
    session.is_valid()
        && service.is_valid()
        && service.max_clients == 1
        && service.max_provider_polls_per_pump == 1
        && session.semantic == service.peer
        && session.max_snapshot_bytes <= service.max_snapshot_bytes
        && session.max_total_snapshot_bytes <= service.max_total_snapshot_bytes
        && session.max_language_bytes <= service.peer.max_label_bytes
        && usize::try_from(session.max_documents).is_ok_and(|value| value <= service.max_documents)
        && usize::try_from(session.max_pending)
            .is_ok_and(|value| value <= service.max_pending && value <= service.max_deliveries)
}

fn lsp_limits_align(
    adapter: LspAdapterLimitObservationV1,
    session: SemanticSessionLimitsV1,
    service: SemanticServiceLimitsV1,
) -> bool {
    let framing: LspLimitsV1 = adapter.framing();
    lsp_configuration_align(framing, adapter.semantic(), session, service)
}

fn required_lsp_body_bytes(
    session: SemanticSessionLimitsV1,
    service: SemanticServiceLimitsV1,
) -> Option<usize> {
    service
        .max_snapshot_bytes
        .checked_mul(6)
        .and_then(|source| {
            session
                .max_language_bytes
                .max(service.peer.max_query_bytes)
                .checked_mul(6)
                .and_then(|text| source.checked_add(text))
        })
        .and_then(|payload| payload.checked_add(MAX_LSP_URI_BYTES_V1))
        // Conservative JSON-RPC object/member/range/version envelope. JSON
        // string escaping is at most six output bytes per admitted input byte.
        .and_then(|payload| payload.checked_add(4_096))
}

fn required_lsp_frame_bytes(
    session: SemanticSessionLimitsV1,
    service: SemanticServiceLimitsV1,
) -> Option<usize> {
    required_lsp_body_bytes(session, service)?.checked_add(64)
}

fn lsp_configuration_align(
    framing: LspLimitsV1,
    semantic: SemanticLimitsV1,
    session: SemanticSessionLimitsV1,
    service: SemanticServiceLimitsV1,
) -> bool {
    let required_body = required_lsp_body_bytes(session, service);
    limits_align(session, service)
        && framing.is_valid()
        && semantic.is_valid()
        && semantic == session.semantic
        && semantic == service.peer
        && service.max_documents <= framing.max_documents
        && service.max_total_snapshot_bytes <= framing.max_total_source_bytes
        && usize::try_from(session.max_pending)
            .is_ok_and(|pending| pending <= MAX_LSP_PENDING_REQUESTS_V1)
        && service.max_pending <= MAX_LSP_PENDING_REQUESTS_V1
        // V1 deliberately selects the maximal closed framing/URI profile.
        // This prevents a production composition from accepting a service
        // contract and discovering only after launch that its adapter was
        // configured with a narrower body, buffer, header or URI ceiling.
        && framing.max_header_bytes == MAX_LSP_HEADER_BYTES_V1
        && framing.max_headers == MAX_LSP_HEADERS_V1
        && framing.max_body_bytes == MAX_LSP_BODY_BYTES_V1
        && framing.max_buffer_bytes == MAX_LSP_BUFFER_BYTES_V1
        && framing.max_uri_bytes == MAX_LSP_URI_BYTES_V1
        && required_body.is_some_and(|body| {
            body <= framing.max_body_bytes
                && framing.max_header_bytes >= 64
                && body
                    .checked_add(64)
                    .is_some_and(|frame| frame <= framing.max_buffer_bytes)
        })
}

fn project_state(
    state: SemanticProviderStateV1,
    epoch: u64,
) -> Result<SemanticBrokerProviderBindingV1, SemanticBrokerEffectFailureV1> {
    let (readiness, provenance) = match state.readiness {
        SemanticProviderReadinessV1::Unavailable => {
            if state.capabilities != SemanticCapabilitySetV1::bottom() {
                return Err(SemanticBrokerEffectFailureV1::Poisoned);
            }
            (SemanticProviderReadinessProjectionV1::Unavailable, None)
        }
        SemanticProviderReadinessV1::Indexing => {
            if state.capabilities != SemanticCapabilitySetV1::bottom() {
                return Err(SemanticBrokerEffectFailureV1::Poisoned);
            }
            (SemanticProviderReadinessProjectionV1::Indexing, None)
        }
        SemanticProviderReadinessV1::Ready { generation } => (
            SemanticProviderReadinessProjectionV1::Ready { generation },
            Some(state.provider),
        ),
    };
    Ok(SemanticBrokerProviderBindingV1 {
        epoch,
        observation: SemanticProviderObservationV1::new(readiness, state.capabilities),
        provenance,
    })
}

fn map_transition(
    transition: &SemanticBridgeProviderTransitionV1,
    shadow: &mut SemanticBrokerProviderBindingV1,
) -> Result<SemanticBrokerProviderTransitionV1, SemanticBrokerEffectFailureV1> {
    if transition.invalidations().cause() != SemanticInvalidationCauseV1::ProviderStateChanged {
        return Err(SemanticBrokerEffectFailureV1::Poisoned);
    }
    let epoch = shadow
        .epoch
        .checked_add(1)
        .ok_or(SemanticBrokerEffectFailureV1::Poisoned)?;
    let observed = project_state(transition.observed_state().clone(), epoch)?;
    if observed.observation == shadow.observation && observed.provenance == shadow.provenance {
        return Err(SemanticBrokerEffectFailureV1::Poisoned);
    }
    let settled = project_state(transition.settled_state().clone(), epoch)?;
    let mapped = SemanticBrokerProviderTransitionV1 {
        epoch,
        observed_observation: observed.observation,
        observed_provenance: observed.provenance,
        settled_observation: settled.observation,
        settled_provenance: settled.provenance.clone(),
        invalidations: transition.invalidations().clone(),
    };
    *shadow = settled;
    Ok(mapped)
}

fn map_provider_refusal(
    refusal: SemanticServiceRefusalV1,
) -> Result<SemanticSessionRefusalV1, SemanticBrokerEffectFailureV1> {
    match refusal {
        SemanticServiceRefusalV1::ProviderRefused => Ok(SemanticSessionRefusalV1::ProviderRefused),
        _ => Err(SemanticBrokerEffectFailureV1::Poisoned),
    }
}

fn map_physical_delivery(
    outcome: &SemanticDeliveryOutcomeV1,
) -> Result<SemanticBrokerDeliveryOutcomeV1, SemanticBrokerEffectFailureV1> {
    match outcome {
        SemanticDeliveryOutcomeV1::Peer(outcome) => {
            Ok(SemanticBrokerDeliveryOutcomeV1::Peer(outcome.clone()))
        }
        SemanticDeliveryOutcomeV1::Cancelled | SemanticDeliveryOutcomeV1::TimedOut => {
            Err(SemanticBrokerEffectFailureV1::Poisoned)
        }
    }
}

fn deadline_bump(counter: &mut u64, saturated: &mut bool) {
    if let Some(next) = counter.checked_add(1) {
        *counter = next;
    } else {
        *saturated = true;
    }
}

fn deadline_bump_slot(counters: &mut [u64], index: usize) -> bool {
    if let Some(next) = counters[index].checked_add(1) {
        counters[index] = next;
        false
    } else {
        true
    }
}

const fn map_poll_failure_class(
    failure: SemanticBridgePollFailureV1,
) -> SemanticBrokerProviderDriveFailureClassV1 {
    match failure {
        SemanticBridgePollFailureV1::Poisoned => {
            SemanticBrokerProviderDriveFailureClassV1::Poisoned
        }
        SemanticBridgePollFailureV1::RuntimeUnavailable => {
            SemanticBrokerProviderDriveFailureClassV1::RuntimeUnavailable
        }
        SemanticBridgePollFailureV1::DeliveryRefused => {
            SemanticBrokerProviderDriveFailureClassV1::DeliveryRefused
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::collections::VecDeque;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc::{Receiver, SyncSender},
    };
    use std::time::{Duration, Instant};

    use super::*;
    use vellum_adapter_semantic_lsp::LspAdapterV1;
    use vellum_host::semantic::{
        SemanticDocumentRefV1, SemanticIndexGenerationV1, SemanticProviderProvenanceV1,
        SemanticProviderRefV1, SemanticQueryV1, SemanticSourceRevisionV1, SemanticTextV1,
    };
    use vellum_semantic_service::{
        SemanticProviderAdmissionV1, SemanticProviderCompletionV1, SemanticProviderSubmissionV1,
        SemanticSnapshotUpdateV1,
    };

    struct ObservedTransport {
        drops: Arc<AtomicUsize>,
    }

    impl Drop for ObservedTransport {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl LspTransportPortV1 for ObservedTransport {
        fn try_send(&mut self, _frame: &[u8]) -> vellum_adapter_semantic_lsp::LspSendOutcomeV1 {
            vellum_adapter_semantic_lsp::LspSendOutcomeV1::Accepted
        }

        fn poll_receive(&mut self) -> vellum_adapter_semantic_lsp::LspReceiveOutcomeV1 {
            vellum_adapter_semantic_lsp::LspReceiveOutcomeV1::Pending
        }

        fn close(&mut self) {}
    }

    struct TerminalTransport {
        forced: bool,
    }

    impl LspTransportPortV1 for TerminalTransport {
        fn try_send(&mut self, _frame: &[u8]) -> vellum_adapter_semantic_lsp::LspSendOutcomeV1 {
            vellum_adapter_semantic_lsp::LspSendOutcomeV1::Accepted
        }

        fn poll_receive(&mut self) -> vellum_adapter_semantic_lsp::LspReceiveOutcomeV1 {
            if self.forced {
                vellum_adapter_semantic_lsp::LspReceiveOutcomeV1::Closed
            } else {
                vellum_adapter_semantic_lsp::LspReceiveOutcomeV1::Pending
            }
        }

        fn close(&mut self) {}

        fn abort(&mut self) {
            self.forced = true;
        }

        fn terminal_state(&self) -> vellum_adapter_semantic_lsp::LspTransportTerminalV1 {
            if self.forced {
                vellum_adapter_semantic_lsp::LspTransportTerminalV1::ForcedClosed
            } else {
                vellum_adapter_semantic_lsp::LspTransportTerminalV1::Active
            }
        }
    }

    struct LspGenerations(u64);

    impl SemanticGenerationIssuerV1 for LspGenerations {
        fn issue(&mut self) -> Option<SemanticIndexGenerationV1> {
            self.0 = self.0.checked_add(1)?;
            SemanticIndexGenerationV1::new(self.0)
        }
    }

    type ObservedAdapter = LspAdapterV1<ObservedTransport, LspGenerations>;

    fn observed_adapter(
        drops: Arc<AtomicUsize>,
        framing: LspLimitsV1,
        semantic: vellum_host::semantic::SemanticLimitsV1,
    ) -> ObservedAdapter {
        LspAdapterV1::try_new_with_semantic_limits(
            ObservedTransport { drops },
            LspGenerations(0),
            provenance(8),
            framing,
            semantic,
        )
        .unwrap()
    }

    struct ModelProvider {
        state: SemanticProviderStateV1,
        after_synchronize: Option<SemanticProviderStateV1>,
        after_cancel: Option<SemanticProviderStateV1>,
        after_poll: Option<SemanticProviderStateV1>,
        completions: VecDeque<SemanticProviderCompletionV1>,
        poll_failure: Option<SemanticBridgePollFailureV1>,
        poll_entered: Option<SyncSender<()>>,
        poll_release: Option<Receiver<()>>,
        panic_after_poll_release: bool,
        available: bool,
        refuse_synchronize: bool,
        submission: SemanticProviderSubmissionV1,
        state_reads: Cell<usize>,
        state_after_two_reads: Option<SemanticProviderStateV1>,
    }

    #[derive(Debug)]
    struct ScriptClock {
        samples: VecDeque<Result<LocalOrderedInstantV1, LocalOrderedClockFailureV1>>,
    }

    impl ScriptClock {
        fn ticks(ticks: impl IntoIterator<Item = u64>) -> Self {
            Self {
                samples: ticks
                    .into_iter()
                    .map(|tick| Ok(LocalOrderedInstantV1::from_composition_tick(tick)))
                    .collect(),
            }
        }
    }

    impl LocalOrderedMonotonicClockV1 for ScriptClock {
        fn sample(&mut self) -> Result<LocalOrderedInstantV1, LocalOrderedClockFailureV1> {
            self.samples
                .pop_front()
                .unwrap_or(Err(LocalOrderedClockFailureV1::Unavailable))
        }
    }

    fn deadline_policy(max_deadlines: usize, ticks: u64) -> LocalOrderedDeadlinePolicyV1 {
        LocalOrderedDeadlinePolicyV1 {
            relative_budget_ticks: NonZeroU64::new(ticks).unwrap(),
            max_deadlines,
        }
    }

    impl SemanticProviderPortV1 for ModelProvider {
        fn state(&self) -> SemanticProviderStateV1 {
            let reads = self.state_reads.get();
            self.state_reads.set(reads.saturating_add(1));
            if reads >= 2 {
                self.state_after_two_reads
                    .clone()
                    .unwrap_or_else(|| self.state.clone())
            } else {
                self.state.clone()
            }
        }

        fn synchronize(
            &mut self,
            _update: &SemanticSnapshotUpdateV1,
        ) -> SemanticProviderAdmissionV1 {
            if let Some(state) = self.after_synchronize.take() {
                self.state = state;
            }
            if self.refuse_synchronize {
                SemanticProviderAdmissionV1::Refused
            } else if self.available {
                SemanticProviderAdmissionV1::Accepted
            } else {
                SemanticProviderAdmissionV1::Unavailable
            }
        }

        fn submit(&mut self, _request: &SemanticRequestV1) -> SemanticProviderSubmissionV1 {
            if self.available {
                self.submission
            } else {
                SemanticProviderSubmissionV1::Unavailable
            }
        }

        fn cancel(&mut self, _request: SemanticRequestRefV1) {
            if let Some(state) = self.after_cancel.take() {
                self.state = state;
            }
        }

        fn poll(&mut self) -> Option<SemanticProviderCompletionV1> {
            if let Some(state) = self.after_poll.take() {
                self.state = state;
            }
            self.completions.pop_front()
        }
    }

    impl SemanticBridgeProviderPollV1 for ModelProvider {
        fn poll_bridge(
            &mut self,
        ) -> Result<Option<SemanticProviderCompletionV1>, SemanticBridgePollFailureV1> {
            if let Some(entered) = self.poll_entered.take() {
                let _ = entered.send(());
                if let Some(release) = self.poll_release.take() {
                    let _ = release.recv();
                }
                assert!(
                    !self.panic_after_poll_release,
                    "qualified managed provider poll panic"
                );
            }
            if let Some(failure) = self.poll_failure.take() {
                return Err(failure);
            }
            Ok(self.poll())
        }
    }

    fn provenance(tag: u8) -> SemanticProviderProvenanceV1 {
        SemanticProviderProvenanceV1::try_new(
            SemanticProviderRefV1::new([tag; 16]),
            format!("provider-{tag}"),
            "1.0.0".into(),
        )
        .unwrap()
    }

    fn unavailable(tag: u8) -> SemanticProviderStateV1 {
        SemanticProviderStateV1::unavailable(provenance(tag))
    }

    fn indexing(tag: u8) -> SemanticProviderStateV1 {
        SemanticProviderStateV1 {
            provider: provenance(tag),
            readiness: SemanticProviderReadinessV1::Indexing,
            capabilities: SemanticCapabilitySetV1::bottom(),
        }
    }

    fn ready(tag: u8, generation: u64) -> SemanticProviderStateV1 {
        SemanticProviderStateV1 {
            provider: provenance(tag),
            readiness: SemanticProviderReadinessV1::Ready {
                generation: SemanticIndexGenerationV1::new(generation).unwrap(),
            },
            capabilities: SemanticCapabilitySetV1::read_only(),
        }
    }

    fn provider(state: SemanticProviderStateV1) -> ModelProvider {
        ModelProvider {
            state,
            after_synchronize: None,
            after_cancel: None,
            after_poll: None,
            completions: VecDeque::new(),
            poll_failure: None,
            poll_entered: None,
            poll_release: None,
            panic_after_poll_release: false,
            available: true,
            refuse_synchronize: false,
            submission: SemanticProviderSubmissionV1::Accepted,
            state_reads: Cell::new(0),
            state_after_two_reads: None,
        }
    }

    fn limits(max_pending: usize) -> (SemanticSessionLimitsV1, SemanticServiceLimitsV1) {
        let session = SemanticSessionLimitsV1 {
            max_pending: u32::try_from(max_pending).unwrap(),
            ..SemanticSessionLimitsV1::reference()
        };
        let service = SemanticServiceLimitsV1 {
            max_clients: 1,
            max_pending,
            max_deliveries: max_pending,
            max_provider_polls_per_pump: 1,
            ..SemanticServiceLimitsV1::reference()
        };
        (session, service)
    }

    fn client() -> SemanticClientRefV1 {
        SemanticClientRefV1::new([9; 16])
    }

    fn incarnation() -> SemanticSessionIncarnationV1 {
        SemanticSessionIncarnationV1::new([7; 16]).unwrap()
    }

    fn target(revision: u64) -> SemanticTargetV1 {
        SemanticTargetV1 {
            document: SemanticDocumentRefV1::new([3; 16]),
            source_revision: SemanticSourceRevisionV1::new(revision),
        }
    }

    fn snapshot(revision: u64, limits: SemanticSessionLimitsV1) -> SemanticSessionSnapshotV1 {
        SemanticSessionSnapshotV1::try_new(
            target(revision),
            "rust".into(),
            "fn bounded() {}".into(),
            limits,
        )
        .unwrap()
    }

    fn request(revision: u64, tag: u8) -> SemanticRequestV1 {
        request_with_generation(revision, tag, 1)
    }

    fn request_with_generation(revision: u64, tag: u8, generation: u64) -> SemanticRequestV1 {
        SemanticRequestV1 {
            request: SemanticRequestRefV1::new([tag; 16]),
            target: target(revision),
            expected_index: SemanticIndexGenerationV1::new(generation),
            language: SemanticTextV1::try_new("rust".into(), 16).unwrap(),
            query: SemanticQueryV1::Diagnostics,
        }
    }

    fn prepared(
        provider: ModelProvider,
        max_pending: usize,
    ) -> (
        LocalOrderedSemanticPreparedV1<ModelProvider>,
        SemanticSessionLimitsV1,
    ) {
        let (session, service) = limits(max_pending);
        (
            LocalOrderedSemanticPreparedV1::try_new_injected_provider_without_deadlines(
                provider,
                client(),
                incarnation(),
                session,
                service,
            )
            .unwrap(),
            session,
        )
    }

    fn corrupted_timed() -> (
        LocalOrderedTimedSemanticPreparedV1<ModelProvider, ScriptClock>,
        SemanticSessionLimitsV1,
    ) {
        let (prepared, session) = prepared(provider(ready(1, 1)), 2);
        let mut timed = prepared
            .try_with_deadlines(ScriptClock::ticks([0, 1]), deadline_policy(2, 10))
            .unwrap();
        timed.port.scheduler.by_due.insert((
            LocalOrderedInstantV1::from_composition_tick(99),
            SemanticRequestRefV1::new([99; 16]),
        ));
        (timed, session)
    }

    #[test]
    fn managed_timed_actor_suspends_replaces_and_resumes_one_ordered_epoch() {
        let (prepared, _) = prepared(provider(ready(1, 1)), 2);
        let timed = prepared
            .try_with_deadlines(ScriptClock::ticks([0]), deadline_policy(2, 10))
            .unwrap();
        let (actor, old_sender) = timed
            .spawn_managed(SemanticSessionActorLimitsV1::reference())
            .unwrap();
        let mut ticket = actor.try_begin_suspension().unwrap();
        let mut suspended = loop {
            match ticket.try_advance() {
                LocalOrderedTimedSuspensionAdvanceV1::Pending(pending) => {
                    ticket = *pending;
                    std::thread::yield_now();
                }
                LocalOrderedTimedSuspensionAdvanceV1::Ready(runtime) => break *runtime,
                LocalOrderedTimedSuspensionAdvanceV1::Failed(_) => {
                    panic!("qualified suspension recovered a failed worker")
                }
            }
        };
        assert_eq!(suspended.prior_actor_usage().suspensions, 1);
        let (next, retirement) = suspended
            .try_replace_provider(provider(ready(2, 2)))
            .unwrap();
        suspended = next;
        assert_eq!(suspended.port.base.binding.epoch, 1);
        assert!(matches!(
            suspended.port.base.binding.observation.readiness(),
            SemanticProviderReadinessProjectionV1::Ready { generation }
                if generation == SemanticIndexGenerationV1::new(2).unwrap()
        ));
        assert!(retirement.into_provider_if_drained().is_ok());
        let (actor, sender) = suspended.resume().unwrap();
        assert_eq!(actor.phase(), SemanticSessionActorPhaseV1::Running);
        assert_eq!(actor.usage().suspensions, 1);
        assert_eq!(old_sender.phase(), SemanticSessionActorPhaseV1::Suspended);
        let old_admission = old_sender.try_exchange(
            vellum_semantic_session_broker::SemanticBrokerContextV1 {
                subject: vellum_host::authority::AuthoritySubjectRefV1::new([81; 16]),
                session: vellum_host::authority::AuthoritySessionRefV1::new([82; 16]),
                allowed_roles: vellum_semantic_protocol::SemanticSessionRolesV1::top(),
                policy_epoch: 1,
            },
            vellum_semantic_protocol::SemanticSessionCommandV1::Describe {
                exchange: vellum_semantic_protocol::SemanticExchangeRefV1::new([83; 16]).unwrap(),
            },
        );
        assert!(matches!(
            old_admission,
            vellum_semantic_session_actor::SemanticSessionActorAdmissionV1::NotSent {
                reason: vellum_semantic_session_actor::SemanticSessionActorNotSentReasonV1::Closed,
                ..
            }
        ));
        drop(old_sender);
        drop(sender);
        drop(actor);
    }

    #[test]
    fn ordered_replacement_replays_snapshot_and_invalidates_pending_deadline_without_query_replay()
    {
        let (prepared, session) = prepared(provider(ready(1, 1)), 2);
        let mut timed = prepared
            .try_with_deadlines(ScriptClock::ticks([10]), deadline_policy(2, 5))
            .unwrap();
        assert!(matches!(
            timed.port.open(&snapshot(1, session)).unwrap().outcome,
            SemanticBrokerAdmissionOutcomeV1::Accepted
        ));
        let pending = request(1, 4);
        assert_eq!(
            timed.port.submit(&pending).unwrap().outcome,
            SemanticBrokerSubmissionOutcomeV1::Accepted
        );
        assert_eq!(timed.port.scheduler.by_request.len(), 1);
        let replacement = timed
            .port
            .try_replace_provider(provider(ready(2, 2)))
            .unwrap();
        assert!(timed.port.scheduler.by_request.is_empty());
        assert!(timed.port.scheduler.by_due.is_empty());
        let invalidations = &replacement.report.pre_provider.unwrap().invalidations;
        assert_eq!(
            invalidations.iter().copied().collect::<Vec<_>>(),
            vec![pending.request]
        );
        assert!(replacement.retirement.into_provider_if_drained().is_ok());
        let usage = timed.port.base.service.usage();
        assert_eq!(usage.snapshots_replayed, 1);
        assert_eq!(usage.requests_submitted, 1);
        assert_eq!(usage.requests_invalidated, 1);
    }

    #[test]
    fn specialized_lsp_constructor_closes_every_limit_axis_and_retains_refusals() {
        let (session, service) = limits(2);
        let drops = Arc::new(AtomicUsize::new(0));
        let prepared = LocalOrderedSemanticPreparedV1::try_new_lsp_without_deadlines(
            observed_adapter(
                Arc::clone(&drops),
                LspLimitsV1::reference(),
                session.semantic,
            )
            .try_start_fresh()
            .unwrap(),
            client(),
            incarnation(),
            session,
            service,
        )
        .unwrap();
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        let mut cleanup = prepared.into_cleanup();
        let _ = cleanup.retire_client();
        let adapter = cleanup
            .try_into_retiring_provider()
            .unwrap()
            .into_provider_for_retirement();
        drop(adapter);
        assert_eq!(drops.load(Ordering::SeqCst), 1);

        let mut narrower = Vec::new();
        let reference = LspLimitsV1::reference();
        narrower.push(LspLimitsV1 {
            max_header_bytes: reference.max_header_bytes - 1,
            ..reference
        });
        narrower.push(LspLimitsV1 {
            max_headers: reference.max_headers - 1,
            ..reference
        });
        narrower.push(LspLimitsV1 {
            max_body_bytes: reference.max_body_bytes - 1,
            ..reference
        });
        narrower.push(LspLimitsV1 {
            max_buffer_bytes: reference.max_buffer_bytes - 1,
            ..reference
        });
        narrower.push(LspLimitsV1 {
            max_documents: service.max_documents - 1,
            ..reference
        });
        narrower.push(LspLimitsV1 {
            max_uri_bytes: reference.max_uri_bytes - 1,
            ..reference
        });
        narrower.push(LspLimitsV1 {
            max_total_source_bytes: service.max_total_snapshot_bytes - 1,
            ..reference
        });

        for framing in narrower {
            let drops = Arc::new(AtomicUsize::new(0));
            let failure = LocalOrderedSemanticPreparedV1::try_new_lsp_without_deadlines(
                observed_adapter(Arc::clone(&drops), framing, session.semantic)
                    .try_start_fresh()
                    .unwrap(),
                client(),
                incarnation(),
                session,
                service,
            )
            .unwrap_err();
            assert_eq!(
                failure.reason(),
                LocalOrderedSemanticBuildRefusalV1::InvalidAdapterLimits
            );
            assert_eq!(drops.load(Ordering::SeqCst), 0);
            let LocalOrderedSemanticBuildResidueV1::UninstalledProvider(adapter) =
                failure.into_residue()
            else {
                panic!("adapter-limit refusal must retain the uninstalled adapter")
            };
            assert_eq!(adapter.limit_observation().framing(), framing);
            drop(adapter);
            assert_eq!(drops.load(Ordering::SeqCst), 1);
        }

        let mut semantic = session.semantic;
        semantic.max_results -= 1;
        let drops = Arc::new(AtomicUsize::new(0));
        let failure = LocalOrderedSemanticPreparedV1::try_new_lsp_without_deadlines(
            observed_adapter(Arc::clone(&drops), LspLimitsV1::reference(), semantic)
                .try_start_fresh()
                .unwrap(),
            client(),
            incarnation(),
            session,
            service,
        )
        .unwrap_err();
        assert_eq!(
            failure.reason(),
            LocalOrderedSemanticBuildRefusalV1::InvalidAdapterLimits
        );
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        let LocalOrderedSemanticBuildResidueV1::UninstalledProvider(adapter) =
            failure.into_residue()
        else {
            panic!("semantic-limit refusal must retain the uninstalled adapter")
        };
        assert_eq!(adapter.limit_observation().semantic(), semantic);
        drop(adapter);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    fn recover_actor(
        mut recovery: SemanticSessionActorRecoveryV1<LocalOrderedSemanticPortV1<ModelProvider>>,
    ) -> vellum_semantic_session_actor::SemanticSessionActorRecoveredV1<
        LocalOrderedSemanticPortV1<ModelProvider>,
    > {
        loop {
            match recovery.try_recover() {
                Ok(recovered) => return recovered,
                Err(returned) => {
                    recovery = returned;
                    std::thread::yield_now();
                }
            }
        }
    }

    #[test]
    fn genesis_projection_is_exact_for_all_readiness_states() {
        for (state, readiness, has_provenance) in [
            (
                unavailable(1),
                SemanticProviderReadinessProjectionV1::Unavailable,
                false,
            ),
            (
                indexing(1),
                SemanticProviderReadinessProjectionV1::Indexing,
                false,
            ),
            (
                ready(1, 1),
                SemanticProviderReadinessProjectionV1::Ready {
                    generation: SemanticIndexGenerationV1::new(1).unwrap(),
                },
                true,
            ),
        ] {
            let (prepared, _) = prepared(provider(state), 2);
            assert_eq!(prepared.port.binding.epoch, 0);
            assert_eq!(prepared.port.binding.observation.readiness(), readiness);
            assert_eq!(prepared.port.binding.provenance.is_some(), has_provenance);
        }

        let invalid = SemanticProviderStateV1 {
            provider: provenance(1),
            readiness: SemanticProviderReadinessV1::Unavailable,
            capabilities: SemanticCapabilitySetV1::read_only(),
        };
        let (session, service) = limits(2);
        let failure = LocalOrderedSemanticPreparedV1::try_new_injected_provider_without_deadlines(
            provider(invalid),
            client(),
            incarnation(),
            session,
            service,
        )
        .unwrap_err();
        assert_eq!(
            failure.reason(),
            LocalOrderedSemanticBuildRefusalV1::InvalidProviderState
        );
        let LocalOrderedSemanticBuildResidueV1::UninstalledProvider(provider) =
            failure.into_residue()
        else {
            panic!("pre-service refusal must retain the raw provider")
        };
        assert_eq!(provider.state().provider, provenance(1));
    }

    #[test]
    fn prepared_broker_and_port_share_one_genesis_binding() {
        let (mut prepared, _) = prepared(provider(ready(1, 1)), 2);
        let binding = prepared.port.binding.clone();
        assert_eq!(
            prepared.broker.event(
                vellum_semantic_session_broker::SemanticBrokerEventV1::ProviderDriveCompleted {
                    report: Box::new(SemanticBrokerProviderDriveReportV1 {
                        provider_before: binding,
                        pre_provider: None,
                        disposition: SemanticBrokerProviderDriveDispositionV1::NoEvent,
                        post_provider: None,
                    }),
                }
            ),
            vellum_semantic_session_broker::SemanticBrokerDecisionV1::EventAccepted
        );
    }

    #[test]
    fn actor_build_refusal_retains_exact_cleanup_authority() {
        let (prepared, _) = prepared(provider(ready(1, 1)), 2);
        let failure = match prepared.spawn(SemanticSessionActorLimitsV1 {
            max_commands: 0,
            max_controls: 1,
        }) {
            Ok(_) => panic!("invalid actor limits must refuse"),
            Err(failure) => failure,
        };
        let (_, _, _broker, port) = failure.into_parts();
        let mut cleanup = LocalOrderedSemanticCleanupV1::from_recovered_port(port);
        assert!(matches!(
            cleanup.retire_client().state,
            vellum_semantic_service::SemanticClientRetirementStateV1::Retired { .. }
        ));
        let retiring = cleanup.try_into_retiring_provider().unwrap();
        let _provider = retiring.into_provider_for_retirement();
    }

    #[test]
    fn spawned_actor_uses_the_prepared_binding_and_recovers_cleanup_authority() {
        let (prepared, _) = prepared(provider(ready(1, 1)), 2);
        let (owner, _sender, recovery) = prepared
            .spawn(SemanticSessionActorLimitsV1::reference())
            .unwrap();
        assert_eq!(
            owner.try_drive_provider().unwrap().receive(),
            vellum_semantic_session_actor::SemanticSessionActorControlCompletionV1::Applied
        );
        assert!(owner.begin_drain());
        owner.try_stop().unwrap();
        let recovered = recover_actor(recovery);
        assert_eq!(
            recovered.termination(),
            vellum_semantic_session_actor::SemanticSessionActorTerminationV1::Clean
        );
        let (_, _, _broker, port) = recovered.into_parts();
        let mut cleanup = LocalOrderedSemanticCleanupV1::from_recovered_port(port);
        let _retired = cleanup.retire_client();
        let retiring = cleanup.try_into_retiring_provider().unwrap();
        let _provider = retiring.into_provider_for_retirement();
    }

    #[test]
    fn timed_actor_schedules_the_broker_internal_identity_not_the_external_request() {
        use vellum_host::authority::{AuthoritySessionRefV1, AuthoritySubjectRefV1};
        use vellum_semantic_protocol::{
            SemanticExchangeDispositionV1, SemanticExchangeRefV1, SemanticSessionCommandV1,
            SemanticSessionOperationRefV1, SemanticSessionOutcomeV1, SemanticSessionRolesV1,
            SemanticSynchronizationEpochV1, SemanticSynchronizationManifestV1,
        };
        use vellum_semantic_session_actor::SemanticSessionActorAdmissionV1;
        use vellum_semantic_session_broker::SemanticBrokerContextV1;

        let (prepared, session) = prepared(provider(ready(1, 1)), 1);
        let timed = prepared
            .try_with_deadlines(ScriptClock::ticks([0]), deadline_policy(1, 10))
            .unwrap();
        let (owner, sender, mut recovery) = timed
            .spawn(SemanticSessionActorLimitsV1::reference())
            .unwrap();
        let context = SemanticBrokerContextV1 {
            subject: AuthoritySubjectRefV1::new([1; 16]),
            session: AuthoritySessionRefV1::new([2; 16]),
            allowed_roles: SemanticSessionRolesV1::top(),
            policy_epoch: 1,
        };
        let exchange = |tag| SemanticExchangeRefV1::new([tag; 16]).unwrap();
        let operation = |tag| SemanticSessionOperationRefV1::new([tag; 16]).unwrap();
        let send = |command| {
            let SemanticSessionActorAdmissionV1::Submitted(ticket) =
                sender.try_exchange(context, command)
            else {
                panic!("actor command was not admitted")
            };
            let SemanticExchangeDispositionV1::Reply(reply) = ticket.receive() else {
                panic!("actor command became uncertain")
            };
            reply
        };
        let joined = send(SemanticSessionCommandV1::JoinSession {
            exchange: exchange(70),
            operation: operation(71),
            incarnation: incarnation(),
            requested_roles: SemanticSessionRolesV1::top(),
        });
        let SemanticSessionOutcomeV1::Joined { membership, .. } = joined.outcome() else {
            panic!("join was refused")
        };
        let membership = *membership;
        let snapshot = snapshot(1, session);
        let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
            std::slice::from_ref(&snapshot),
            session,
        )
        .unwrap();
        let epoch = SemanticSynchronizationEpochV1::new([72; 16]).unwrap();
        assert_eq!(
            send(SemanticSessionCommandV1::BeginSynchronization {
                exchange: exchange(73),
                operation: operation(74),
                incarnation: incarnation(),
                membership,
                epoch,
                manifest,
            })
            .outcome(),
            &SemanticSessionOutcomeV1::Accepted
        );
        assert_eq!(
            send(SemanticSessionCommandV1::SyncOpen {
                exchange: exchange(75),
                operation: operation(76),
                incarnation: incarnation(),
                membership,
                epoch,
                snapshot: snapshot.clone(),
            })
            .outcome(),
            &SemanticSessionOutcomeV1::Accepted
        );
        assert_eq!(
            send(SemanticSessionCommandV1::CompleteSynchronization {
                exchange: exchange(77),
                operation: operation(78),
                incarnation: incarnation(),
                membership,
                epoch,
                manifest,
            })
            .outcome(),
            &SemanticSessionOutcomeV1::Accepted
        );
        let external = SemanticRequestRefV1::new([79; 16]);
        let mut submitted = request(1, 79);
        submitted.request = external;
        assert_eq!(
            send(SemanticSessionCommandV1::Submit {
                exchange: exchange(80),
                operation: operation(81),
                incarnation: incarnation(),
                membership,
                request: submitted,
            })
            .outcome(),
            &SemanticSessionOutcomeV1::Accepted
        );
        drop(owner);
        drop(sender);
        let recovered = loop {
            match recovery.try_recover() {
                Ok(recovered) => break recovered,
                Err(returned) => {
                    recovery = returned;
                    std::thread::yield_now();
                }
            }
        };
        let (_, _, _, port) = recovered.into_parts();
        let scheduled = port
            .scheduler
            .by_request
            .keys()
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(scheduled.len(), 1);
        assert_ne!(scheduled[0], external);
        assert_eq!(
            scheduled[0],
            SemanticRequestRefV1::new(1_u128.to_be_bytes())
        );
        assert_eq!(port.scheduler.by_due.first().unwrap().1, scheduled[0]);
        let cleanup = LocalOrderedTimedSemanticCleanupV1::from_recovered_port(port);
        let (mut cleanup, retirement) = cleanup.retire_deadlines().unwrap();
        assert_eq!(retirement.discarded_live().collect::<Vec<_>>(), scheduled);
        let _ = cleanup.retire_client();
        let _provider = cleanup
            .try_into_retiring_provider()
            .unwrap()
            .into_provider_for_retirement();
    }

    #[test]
    fn fatal_mapper_projection_failure_recovers_the_exact_cleanup_port() {
        let mut model = provider(ready(1, 1));
        model.state_after_two_reads = Some(SemanticProviderStateV1 {
            provider: provenance(2),
            readiness: SemanticProviderReadinessV1::Unavailable,
            capabilities: SemanticCapabilitySetV1::read_only(),
        });
        let (prepared, _) = prepared(model, 1);
        let (owner, _sender, recovery) = prepared
            .spawn(SemanticSessionActorLimitsV1::reference())
            .unwrap();
        assert_eq!(
            owner.try_drive_provider().unwrap().receive(),
            vellum_semantic_session_actor::SemanticSessionActorControlCompletionV1::OutcomeUnknown
        );
        let recovered = recover_actor(recovery);
        assert_eq!(
            recovered.termination(),
            vellum_semantic_session_actor::SemanticSessionActorTerminationV1::Failed
        );
        let (_, _, _broker, port) = recovered.into_parts();
        let mut cleanup = LocalOrderedSemanticCleanupV1::from_recovered_port(port);
        let _retired = cleanup.retire_client();
        let retiring = cleanup.try_into_retiring_provider().unwrap();
        let _provider = retiring.into_provider_for_retirement();
    }

    #[test]
    fn unstarted_prepared_runtime_has_a_cleanup_only_exit() {
        let (prepared, _) = prepared(provider(ready(1, 1)), 2);
        let mut cleanup = prepared.into_cleanup();
        assert!(matches!(
            cleanup.retire_client().state,
            vellum_semantic_service::SemanticClientRetirementStateV1::Retired { .. }
        ));
        let retiring = cleanup.try_into_retiring_provider().unwrap();
        let _provider = retiring.into_provider_for_retirement();
    }

    #[test]
    fn mapper_debug_surfaces_are_content_safe() {
        let (prepared, _) = prepared(provider(ready(1, 1)), 2);
        assert_eq!(
            format!("{prepared:?}"),
            "LocalOrderedSemanticPreparedV1(<opaque>)"
        );
        let cleanup = prepared.into_cleanup();
        assert_eq!(
            format!("{cleanup:?}"),
            "LocalOrderedSemanticCleanupV1(<opaque>)"
        );
    }

    #[test]
    fn post_transition_advances_one_epoch_and_impossible_refusal_is_poison() {
        let mut model = provider(ready(1, 1));
        model.after_synchronize = Some(indexing(1));
        let (mut prepared, session) = prepared(model, 2);
        let report = prepared.port.open(&snapshot(1, session)).unwrap();
        assert_eq!(report.provider_before.epoch, 0);
        let transition = report.post_provider.unwrap();
        assert_eq!(transition.epoch, 1);
        assert_eq!(prepared.port.binding.epoch, 1);
        assert_eq!(
            transition.observed_observation.readiness(),
            SemanticProviderReadinessProjectionV1::Indexing
        );

        let before = prepared.port.binding.clone();
        assert_eq!(
            prepared.port.close(target(99)),
            Err(SemanticBrokerEffectFailureV1::Poisoned)
        );
        assert_eq!(prepared.port.binding, before);
    }

    #[test]
    fn epoch_overflow_and_projection_kernel_transition_fail_closed() {
        let mut overflow_model = provider(ready(1, 1));
        overflow_model.after_synchronize = Some(indexing(1));
        let (mut overflow, session) = prepared(overflow_model, 1);
        overflow.port.binding.epoch = u64::MAX;
        assert_eq!(
            overflow.port.open(&snapshot(1, session)),
            Err(SemanticBrokerEffectFailureV1::Poisoned)
        );
        assert_eq!(overflow.port.binding.epoch, u64::MAX);

        let mut hidden_model = provider(unavailable(1));
        hidden_model.after_synchronize = Some(unavailable(2));
        let (mut hidden, session) = prepared(hidden_model, 1);
        assert_eq!(
            hidden.port.open(&snapshot(1, session)),
            Err(SemanticBrokerEffectFailureV1::Poisoned)
        );
        assert_eq!(hidden.port.binding.epoch, 0);
    }

    #[test]
    fn operation_outcomes_and_source_receipt_causes_are_exact() {
        let (mut prepared, session) = prepared(provider(ready(1, 1)), 3);
        assert_eq!(
            prepared.port.open(&snapshot(1, session)).unwrap().outcome,
            SemanticBrokerAdmissionOutcomeV1::Accepted
        );
        let first = request(1, 11);
        assert_eq!(
            prepared.port.submit(&first).unwrap().outcome,
            SemanticBrokerSubmissionOutcomeV1::Accepted
        );
        let changed = prepared.port.change(&snapshot(2, session)).unwrap();
        let SemanticBrokerSourceAdmissionOutcomeV1::Accepted(receipt) = changed.outcome else {
            panic!("change must return exact source receipt")
        };
        assert_eq!(receipt.cause(), SemanticInvalidationCauseV1::SourceChanged);
        assert_eq!(
            receipt.iter().copied().collect::<Vec<_>>(),
            vec![first.request]
        );

        let second = request(2, 12);
        assert_eq!(
            prepared.port.submit(&second).unwrap().outcome,
            SemanticBrokerSubmissionOutcomeV1::Accepted
        );
        let closed = prepared.port.close(target(2)).unwrap();
        let SemanticBrokerSourceAdmissionOutcomeV1::Accepted(receipt) = closed.outcome else {
            panic!("close must return exact source receipt")
        };
        assert_eq!(receipt.cause(), SemanticInvalidationCauseV1::SourceClosed);
        assert_eq!(
            receipt.iter().copied().collect::<Vec<_>>(),
            vec![second.request]
        );
    }

    #[test]
    fn unavailable_refused_unsupported_and_stale_remain_distinct() {
        let mut unavailable_provider = provider(unavailable(1));
        unavailable_provider.available = false;
        let (mut unavailable_port, session) = prepared(unavailable_provider, 2);
        assert_eq!(
            unavailable_port
                .port
                .open(&snapshot(1, session))
                .unwrap()
                .outcome,
            SemanticBrokerAdmissionOutcomeV1::Unavailable
        );

        let mut refused_provider = provider(ready(1, 1));
        refused_provider.refuse_synchronize = true;
        let (mut refused_port, session) = prepared(refused_provider, 2);
        assert_eq!(
            refused_port
                .port
                .open(&snapshot(1, session))
                .unwrap()
                .outcome,
            SemanticBrokerAdmissionOutcomeV1::Refused(SemanticSessionRefusalV1::ProviderRefused)
        );

        let mut unsupported_provider = provider(ready(1, 1));
        unsupported_provider.submission = SemanticProviderSubmissionV1::Unsupported(
            vellum_host::semantic::SemanticQueryKindV1::Diagnostics,
        );
        let (mut unsupported_port, session) = prepared(unsupported_provider, 2);
        let _open = unsupported_port.port.open(&snapshot(1, session)).unwrap();
        assert_eq!(
            unsupported_port
                .port
                .submit(&request(1, 2))
                .unwrap()
                .outcome,
            SemanticBrokerSubmissionOutcomeV1::Unsupported(
                vellum_host::semantic::SemanticQueryKindV1::Diagnostics
            )
        );
        assert_eq!(
            unsupported_port
                .port
                .submit(&request_with_generation(1, 3, 2))
                .unwrap()
                .outcome,
            SemanticBrokerSubmissionOutcomeV1::Stale
        );
    }

    #[test]
    fn provider_drive_distinguishes_peer_delivery_and_orphan() {
        let peer_request = request(1, 14);
        let mut delivery_provider = provider(ready(1, 1));
        delivery_provider
            .completions
            .push_back(SemanticProviderCompletionV1 {
                request: peer_request.request,
                outcome: vellum_host::semantic::SemanticOutcomeV1::Unavailable,
            });
        let (mut delivery_port, session) = prepared(delivery_provider, 2);
        let _open = delivery_port.port.open(&snapshot(1, session)).unwrap();
        let _submit = delivery_port.port.submit(&peer_request).unwrap();
        let SemanticBrokerProviderDriveV1::Completed(delivery) =
            delivery_port.port.drive_provider().unwrap()
        else {
            panic!("peer completion must be a completed drive")
        };
        assert!(matches!(
            delivery.disposition,
            SemanticBrokerProviderDriveDispositionV1::Delivery { .. }
        ));

        let mut orphan_provider = provider(ready(1, 1));
        orphan_provider
            .completions
            .push_back(SemanticProviderCompletionV1 {
                request: SemanticRequestRefV1::new([99; 16]),
                outcome: vellum_host::semantic::SemanticOutcomeV1::Unavailable,
            });
        let (mut orphan_port, _) = prepared(orphan_provider, 1);
        let SemanticBrokerProviderDriveV1::Completed(orphan) =
            orphan_port.port.drive_provider().unwrap()
        else {
            panic!("orphan completion must be a completed drive")
        };
        assert!(matches!(
            orphan.disposition,
            SemanticBrokerProviderDriveDispositionV1::Orphan { .. }
        ));
    }

    #[test]
    fn provider_aba_stage_and_delivery_are_preserved_atomically() {
        let mut model = provider(ready(1, 1));
        model.after_poll = Some(indexing(1));
        model.after_cancel = Some(ready(1, 1));
        let completion_request = SemanticRequestRefV1::new([4; 16]);
        model.completions.push_back(SemanticProviderCompletionV1 {
            request: completion_request,
            outcome: vellum_host::semantic::SemanticOutcomeV1::Unavailable,
        });
        let (mut prepared, session) = prepared(model, 2);
        let _open = prepared.port.open(&snapshot(1, session)).unwrap();
        let submitted = request(1, 4);
        assert_eq!(
            prepared.port.submit(&submitted).unwrap().outcome,
            SemanticBrokerSubmissionOutcomeV1::Accepted
        );

        // The queued completion is still exactly classified; if the provider
        // transition invalidated it first, it is Late rather than Delivery.
        let SemanticBrokerProviderDriveV1::Completed(driven) =
            prepared.port.drive_provider().unwrap()
        else {
            panic!("successful poll must return completed evidence")
        };
        assert!(matches!(
            driven.disposition,
            SemanticBrokerProviderDriveDispositionV1::Late { .. }
        ));
        let transition = driven.post_provider.unwrap();
        assert_eq!(
            transition.observed_observation.readiness(),
            SemanticProviderReadinessProjectionV1::Indexing
        );
        assert_eq!(
            transition.settled_observation.readiness(),
            SemanticProviderReadinessProjectionV1::Ready {
                generation: SemanticIndexGenerationV1::new(1).unwrap()
            }
        );
    }

    #[test]
    fn cancellation_releases_live_capacity_without_rebinding_identity() {
        let (mut prepared, session) = prepared(provider(ready(1, 1)), 1);
        let _open = prepared.port.open(&snapshot(1, session)).unwrap();
        let first = request(1, 4);
        assert_eq!(
            prepared.port.submit(&first).unwrap().outcome,
            SemanticBrokerSubmissionOutcomeV1::Accepted
        );
        assert_eq!(
            prepared.port.cancel(first.request).unwrap().outcome,
            SemanticBrokerCancellationOutcomeV1::Accepted
        );
        assert_eq!(
            prepared.port.submit(&request(1, 5)).unwrap().outcome,
            SemanticBrokerSubmissionOutcomeV1::Accepted
        );
        assert_eq!(
            prepared.port.submit(&first),
            Err(SemanticBrokerEffectFailureV1::Poisoned)
        );
    }

    #[test]
    fn provider_poll_failures_remain_closed_and_do_not_advance_binding() {
        for (service_failure, broker_failure) in [
            (
                SemanticBridgePollFailureV1::Poisoned,
                SemanticBrokerProviderDriveFailureClassV1::Poisoned,
            ),
            (
                SemanticBridgePollFailureV1::RuntimeUnavailable,
                SemanticBrokerProviderDriveFailureClassV1::RuntimeUnavailable,
            ),
            (
                SemanticBridgePollFailureV1::DeliveryRefused,
                SemanticBrokerProviderDriveFailureClassV1::DeliveryRefused,
            ),
        ] {
            let mut model = provider(ready(1, 1));
            model.poll_failure = Some(service_failure);
            let (mut prepared, _) = prepared(model, 1);
            let before = prepared.port.binding.clone();
            let SemanticBrokerProviderDriveV1::Failed(report) =
                prepared.port.drive_provider().unwrap()
            else {
                panic!("physical poll failure must retain failure evidence")
            };
            assert_eq!(report.failure, broker_failure);
            assert_eq!(prepared.port.binding, before);
        }
    }

    #[test]
    fn provider_poll_failure_preserves_a_lawful_pre_transition() {
        let mut model = provider(ready(1, 1));
        model.state_after_two_reads = Some(indexing(1));
        model.poll_failure = Some(SemanticBridgePollFailureV1::RuntimeUnavailable);
        let (mut prepared, _) = prepared(model, 1);
        let SemanticBrokerProviderDriveV1::Failed(report) = prepared.port.drive_provider().unwrap()
        else {
            panic!("failure must retain its ordered evidence")
        };
        assert_eq!(report.provider_before.epoch, 0);
        assert_eq!(report.pre_provider.as_ref().unwrap().epoch, 1);
        assert_eq!(
            report.failure,
            SemanticBrokerProviderDriveFailureClassV1::RuntimeUnavailable
        );
        assert_eq!(prepared.port.binding.epoch, 1);
    }

    #[test]
    fn physical_provider_drive_cannot_fabricate_cancel_or_timeout() {
        assert_eq!(
            map_physical_delivery(&SemanticDeliveryOutcomeV1::Cancelled),
            Err(SemanticBrokerEffectFailureV1::Poisoned)
        );
        assert_eq!(
            map_physical_delivery(&SemanticDeliveryOutcomeV1::TimedOut),
            Err(SemanticBrokerEffectFailureV1::Poisoned)
        );
    }

    #[test]
    fn deadline_disabled_profile_has_a_truthful_singleton_no_due_state() {
        let (mut prepared, _) = prepared(provider(ready(1, 1)), 1);
        assert_eq!(
            prepared.port.drive_expiry(),
            Ok(SemanticBrokerExpiryDriveV1::NoDue)
        );
        let receipt = prepared.into_cleanup().confirm_deadlines_disabled();
        assert_eq!(
            receipt.disposition(),
            LocalOrderedDeadlineDispositionV1::Disabled
        );
        assert_eq!(receipt.discarded_live().len(), 0);
        assert_eq!(receipt.unresolved().len(), 0);
    }

    #[test]
    fn timed_construction_is_closed_and_retains_every_owned_input_on_refusal() {
        let (prepared, _) = prepared(provider(ready(1, 1)), 2);
        let failure = prepared
            .try_with_deadlines(ScriptClock::ticks([0]), deadline_policy(1, 10))
            .unwrap_err();
        assert_eq!(
            failure.reason(),
            LocalOrderedDeadlineBuildRefusalV1::InsufficientCapacity
        );
        assert!(!format!("{failure:?}").contains("provider-1"));
        let (prepared, _clock) = failure.into_parts();
        let mut cleanup = prepared.into_cleanup();
        let _ = cleanup.retire_client();
        let _provider = cleanup
            .try_into_retiring_provider()
            .unwrap()
            .into_provider_for_retirement();
    }

    #[test]
    fn timed_scheduler_drives_one_canonical_due_item_and_clears_exactly() {
        let (prepared, session) = prepared(provider(ready(1, 1)), 2);
        let mut timed = prepared
            .try_with_deadlines(
                ScriptClock::ticks([0, 0, 9, 10, 10]),
                deadline_policy(2, 10),
            )
            .unwrap();
        assert_eq!(
            timed.port.open(&snapshot(1, session)).unwrap().outcome,
            SemanticBrokerAdmissionOutcomeV1::Accepted
        );
        let first = request(1, 22);
        let second = request(1, 21);
        assert_eq!(
            timed.port.submit(&first).unwrap().outcome,
            SemanticBrokerSubmissionOutcomeV1::Accepted
        );
        assert_eq!(
            timed.port.submit(&second).unwrap().outcome,
            SemanticBrokerSubmissionOutcomeV1::Accepted
        );
        assert_eq!(timed.port.obligations().request_deadlines, 2);
        assert_eq!(
            timed.port.drive_expiry(),
            Ok(SemanticBrokerExpiryDriveV1::NoDue)
        );
        let SemanticBrokerExpiryDriveV1::Due(expired) = timed.port.drive_expiry().unwrap() else {
            panic!("the first canonical deadline must be due")
        };
        assert_eq!(expired.internal_request, second.request);
        assert_eq!(expired.outcome, SemanticBrokerExpiryOutcomeV1::Expired);
        assert_eq!(timed.port.obligations().request_deadlines, 1);
        let SemanticBrokerExpiryDriveV1::Due(expired) = timed.port.drive_expiry().unwrap() else {
            panic!("one activation must leave the second due item live")
        };
        assert_eq!(expired.internal_request, first.request);
        assert_eq!(timed.port.obligations().request_deadlines, 0);
        let usage = timed.port.deadline_usage();
        assert_eq!((usage.scheduled, usage.expired, usage.no_due), (2, 2, 1));
        assert_eq!(usage.high_water, 2);
    }

    #[test]
    fn timed_terminal_receipts_clear_and_retirement_is_exact_and_content_safe() {
        let (prepared, session) = prepared(provider(ready(1, 1)), 2);
        let mut timed = prepared
            .try_with_deadlines(ScriptClock::ticks([0, 1]), deadline_policy(2, 10))
            .unwrap();
        let _ = timed.port.open(&snapshot(1, session)).unwrap();
        let cancelled = request(1, 31);
        let discarded = request(1, 32);
        let _ = timed.port.submit(&cancelled).unwrap();
        let _ = timed.port.cancel(cancelled.request).unwrap();
        let _ = timed.port.submit(&discarded).unwrap();
        assert_eq!(timed.port.obligations().request_deadlines, 1);

        let cleanup = timed.into_cleanup();
        let (mut cleanup, receipt) = cleanup.retire_deadlines().unwrap();
        assert_eq!(
            receipt.disposition(),
            LocalOrderedDeadlineDispositionV1::Retired
        );
        assert_eq!(
            receipt.discarded_live().collect::<Vec<_>>(),
            vec![discarded.request]
        );
        assert_eq!(receipt.unresolved().len(), 0);
        assert_eq!(receipt.terminal_clears(), 1);
        let debug = format!("{receipt:?}");
        assert!(!debug.contains(&format!("{:?}", discarded.request)));
        let _ = cleanup.retire_client();
        let _provider = cleanup
            .try_into_retiring_provider()
            .unwrap()
            .into_provider_for_retirement();
    }

    #[test]
    fn timed_clock_failures_close_before_provider_submission() {
        let (prepared_runtime, session) = prepared(provider(ready(1, 1)), 1);
        let mut timed = prepared_runtime
            .try_with_deadlines(
                ScriptClock {
                    samples: VecDeque::from([Err(LocalOrderedClockFailureV1::Unavailable)]),
                },
                deadline_policy(1, 10),
            )
            .unwrap();
        let _ = timed.port.open(&snapshot(1, session)).unwrap();
        assert_eq!(
            timed.port.submit(&request(1, 41)),
            Err(SemanticBrokerEffectFailureV1::RuntimeUnavailable)
        );
        assert_eq!(timed.port.obligations().request_deadlines, 0);
        assert_eq!(timed.port.deadline_usage().clock_unavailable, 1);

        let (prepared, session) = prepared(provider(ready(1, 1)), 1);
        let mut regressing = prepared
            .try_with_deadlines(ScriptClock::ticks([5, 4]), deadline_policy(1, 10))
            .unwrap();
        let _ = regressing.port.open(&snapshot(1, session)).unwrap();
        let submitted = request(1, 42);
        let _ = regressing.port.submit(&submitted).unwrap();
        assert_eq!(
            regressing.port.drive_expiry(),
            Err(SemanticBrokerEffectFailureV1::Poisoned)
        );
        assert_eq!(regressing.port.obligations().request_deadlines, 1);
        assert_eq!(regressing.port.deadline_usage().clock_regressions, 1);
    }

    #[test]
    fn timed_capacity_and_tick_overflow_fail_closed_before_effects() {
        let mut scheduler = LocalOrderedDeadlineSchedulerV1::try_new(
            ScriptClock::ticks([0]),
            deadline_policy(1, 1),
            1,
        )
        .unwrap();
        let due = scheduler.reserve_due().unwrap();
        let install = scheduler
            .prepare_install(SemanticRequestRefV1::new([51; 16]), due)
            .unwrap();
        scheduler.commit_install(install);
        assert_eq!(
            scheduler.reserve_due(),
            Err(SemanticBrokerEffectFailureV1::Poisoned)
        );
        assert_eq!(scheduler.usage.capacity_refusals, 1);
        assert_eq!(scheduler.usage.clock_samples, 1);

        let mut overflow = LocalOrderedDeadlineSchedulerV1::try_new(
            ScriptClock::ticks([u64::MAX]),
            deadline_policy(1, 1),
            1,
        )
        .unwrap();
        assert_eq!(
            overflow.reserve_due(),
            Err(SemanticBrokerEffectFailureV1::Poisoned)
        );
        assert_eq!(overflow.usage.clock_overflow, 1);
        assert_eq!(overflow.usage.current, 0);
    }

    #[test]
    fn exact_graph_faults_preflight_every_effect_before_service_mutation() {
        let (mut timed, session) = corrupted_timed();
        let binding = timed.port.base.binding.clone();
        assert_eq!(
            timed.port.open(&snapshot(1, session)),
            Err(SemanticBrokerEffectFailureV1::Poisoned)
        );
        assert_eq!(timed.port.base.binding, binding);

        let (mut timed, session) = corrupted_timed();
        assert_eq!(
            timed.port.change(&snapshot(2, session)),
            Err(SemanticBrokerEffectFailureV1::Poisoned)
        );
        let (mut timed, _) = corrupted_timed();
        assert_eq!(
            timed.port.close(target(1)),
            Err(SemanticBrokerEffectFailureV1::Poisoned)
        );
        let (mut timed, _) = corrupted_timed();
        assert_eq!(
            timed.port.submit(&request(1, 61)),
            Err(SemanticBrokerEffectFailureV1::Poisoned)
        );
        let (mut timed, _) = corrupted_timed();
        assert_eq!(
            timed.port.cancel(SemanticRequestRefV1::new([61; 16])),
            Err(SemanticBrokerEffectFailureV1::Poisoned)
        );
        let (mut timed, _) = corrupted_timed();
        assert_eq!(
            timed.port.drive_provider(),
            Err(SemanticBrokerEffectFailureV1::Poisoned)
        );
        let (mut timed, _) = corrupted_timed();
        assert_eq!(
            timed.port.drive_expiry(),
            Err(SemanticBrokerEffectFailureV1::Poisoned)
        );

        let obligations = timed.port.obligations();
        assert_eq!(
            (obligations.request_deadlines, obligations.due_deadlines),
            (0, 1)
        );
        assert!(obligations.poisoned);
        assert!(!obligations.consistent);
    }

    #[test]
    fn install_is_transactional_and_retirement_detects_wrong_due_values() {
        let mut scheduler = LocalOrderedDeadlineSchedulerV1::try_new(
            ScriptClock::ticks([]),
            deadline_policy(2, 1),
            2,
        )
        .unwrap();
        let request = SemanticRequestRefV1::new([62; 16]);
        let first_due = LocalOrderedInstantV1::from_composition_tick(10);
        let second_due = LocalOrderedInstantV1::from_composition_tick(11);
        let install = scheduler.prepare_install(request, first_due).unwrap();
        scheduler.commit_install(install);
        assert_eq!(
            scheduler.prepare_install(request, second_due).unwrap_err(),
            SemanticBrokerEffectFailureV1::Poisoned
        );
        assert_eq!(scheduler.by_request.get(&request), Some(&first_due));
        assert!(scheduler.by_due.contains(&(first_due, request)));
        assert!(!scheduler.by_due.contains(&(second_due, request)));

        scheduler.by_due.remove(&(first_due, request));
        scheduler.by_due.insert((second_due, request));
        let receipt = scheduler.retirement_receipt();
        assert_eq!(
            receipt.disposition(),
            LocalOrderedDeadlineDispositionV1::Unresolved
        );
        assert_eq!(receipt.unresolved().collect::<Vec<_>>(), vec![request]);
        assert_eq!(receipt.retirement_discards(), 0);
    }

    #[test]
    fn multi_clear_is_atomic_after_preflight_and_retirement_saturates_truthfully() {
        let (prepared, session) = prepared(provider(ready(1, 1)), 2);
        let mut timed = prepared
            .try_with_deadlines(ScriptClock::ticks([0, 1]), deadline_policy(2, 10))
            .unwrap();
        let _ = timed.port.open(&snapshot(1, session)).unwrap();
        let first = request(1, 63);
        let second = request(1, 64);
        let _ = timed.port.submit(&first).unwrap();
        let _ = timed.port.submit(&second).unwrap();
        let changed = timed.port.change(&snapshot(2, session)).unwrap();
        let SemanticBrokerSourceAdmissionOutcomeV1::Accepted(receipt) = changed.outcome else {
            panic!("source change must carry exact multi-clear evidence")
        };
        assert_eq!(receipt.iter().count(), 2);
        assert!(timed.port.scheduler.graph_is_consistent());
        assert_eq!(timed.port.obligations().request_deadlines, 0);
        assert_eq!(timed.port.deadline_usage().clears_by_cause[1], 2);

        timed.port.scheduler.usage.clears_by_cause = [u64::MAX, 1, 0, 0, 0];
        timed.port.scheduler.usage.retirement_discards = u64::MAX;
        let (_, receipt) = timed.into_cleanup().retire_deadlines().unwrap();
        assert_eq!(receipt.terminal_clears(), u64::MAX);
        assert_eq!(receipt.retirement_discards(), u64::MAX);
        assert!(receipt.saturated());
    }

    #[test]
    fn affine_cleanup_owner_joins_origin_client_extraction_and_terminal_evidence() {
        let (session, service) = limits(2);
        let adapter = LspAdapterV1::try_new_with_semantic_limits(
            TerminalTransport { forced: false },
            LspGenerations(0),
            provenance(8),
            LspLimitsV1::reference(),
            session.semantic,
        )
        .unwrap()
        .try_start_fresh()
        .unwrap();
        let prepared = LocalOrderedSemanticPreparedV1::try_new_lsp_without_deadlines(
            adapter,
            client(),
            incarnation(),
            session,
            service,
        )
        .unwrap();
        let mut owner = LocalOrderedCleanupOwnerV1::from_unstarted(prepared);
        assert_eq!(
            owner.origin().origin(),
            LocalOrderedCleanupOriginV1::Unstarted
        );
        assert_eq!(
            owner.deadlines().disposition(),
            LocalOrderedDeadlineDispositionV1::Disabled
        );
        assert!(matches!(
            owner.drive_client_retirement_once().state,
            vellum_semantic_service::SemanticClientRetirementStateV1::Retired { .. }
        ));
        assert_eq!(owner.client_usage().passes, 1);
        let mut provider = owner.try_into_provider_cleanup().unwrap();
        assert_eq!(
            provider.escalate(),
            LocalSemanticRetirementStateV1::ForcedShutdownPending
        );
        assert_eq!(
            provider.drive_once(),
            LocalSemanticRetirementStateV1::ForcedConfirmed
        );
        let receipt = provider.try_into_terminal_receipt().unwrap();
        assert_eq!(
            receipt.extraction(),
            LocalOrderedProviderExtractionV1::Checked
        );
        assert_eq!(receipt.client_usage().passes, 1);
        assert_eq!(
            receipt.provider().terminal(),
            super::super::LocalSemanticRetirementTerminalV1::ForcedConfirmed
        );
        assert!(!format!("{receipt:?}").contains("provider-8"));
    }

    #[test]
    fn managed_clean_stop_joins_same_spawn_recovery_into_cleanup_origin() {
        let (prepared, _) = prepared(provider(ready(1, 1)), 2);
        let (actor, sender) = prepared
            .spawn_managed(SemanticSessionActorLimitsV1::reference())
            .unwrap();
        assert!(actor.begin_drain());
        let mut ticket = actor.try_begin_stop().unwrap();
        let stopped = loop {
            match ticket.try_complete() {
                Ok(LocalOrderedManagedStopResolutionV1::Terminal(stopped)) => break stopped,
                Ok(LocalOrderedManagedStopResolutionV1::Refused(_)) => {
                    panic!("quiescent managed actor refused stop")
                }
                Err(pending) => {
                    ticket = *pending;
                    std::thread::yield_now();
                }
            }
        };
        assert!(matches!(
            stopped.stop().terminal(),
            LocalOrderedActorTerminalStopV1::Clean(_)
        ));
        let mut stopped = stopped;
        let cleanup = loop {
            match stopped.try_advance() {
                LocalOrderedStoppedRecoveryAdvanceV1::Pending(pending) => {
                    stopped = *pending;
                    std::thread::yield_now();
                }
                LocalOrderedStoppedRecoveryAdvanceV1::Ready(cleanup) => break *cleanup,
                LocalOrderedStoppedRecoveryAdvanceV1::Mismatch(_) => {
                    panic!("same-spawn clean stop mismatched clean recovery")
                }
            }
        };
        assert_eq!(
            cleanup.origin().origin(),
            LocalOrderedCleanupOriginV1::ActorRecovered(SemanticSessionActorTerminationV1::Clean)
        );
        let stop = cleanup.origin().actor_stop().unwrap();
        assert_eq!(stop.refusal_attempts(), 0);
        assert!(stop.last_refusal().is_none());
        assert!(!stop.saturated());
        assert!(matches!(
            stop.terminal(),
            LocalOrderedActorTerminalStopV1::Clean(receipt)
                if receipt.observed_phase() == SemanticSessionActorPhaseV1::Stopped
        ));
        assert!(!format!("{cleanup:?}").contains("provider-1"));
        drop(sender);
    }

    #[test]
    fn managed_stop_refusal_restores_owner_and_joins_bounded_history_after_retirement() {
        use vellum_host::authority::{AuthoritySessionRefV1, AuthoritySubjectRefV1};
        use vellum_semantic_protocol::{
            SemanticExchangeDispositionV1, SemanticExchangeRefV1, SemanticSessionCommandV1,
            SemanticSessionOperationRefV1, SemanticSessionOutcomeV1, SemanticSessionRolesV1,
        };
        use vellum_semantic_session_actor::{
            SemanticSessionActorAdmissionV1, SemanticSessionActorControlCompletionV1,
        };
        use vellum_semantic_session_broker::SemanticBrokerContextV1;

        let (prepared, _) = prepared(provider(ready(3, 1)), 2);
        let (actor, sender) = prepared
            .spawn_managed(SemanticSessionActorLimitsV1::reference())
            .unwrap();
        let context = SemanticBrokerContextV1 {
            subject: AuthoritySubjectRefV1::new([31; 16]),
            session: AuthoritySessionRefV1::new([32; 16]),
            allowed_roles: SemanticSessionRolesV1::top(),
            policy_epoch: 1,
        };
        let send = |command| {
            let SemanticSessionActorAdmissionV1::Submitted(ticket) =
                sender.try_exchange(context, command)
            else {
                panic!("managed refusal command was not admitted")
            };
            let SemanticExchangeDispositionV1::Reply(reply) = ticket.receive() else {
                panic!("managed refusal command became uncertain")
            };
            reply
        };
        let joined = send(SemanticSessionCommandV1::JoinSession {
            exchange: SemanticExchangeRefV1::new([33; 16]).unwrap(),
            operation: SemanticSessionOperationRefV1::new([34; 16]).unwrap(),
            incarnation: incarnation(),
            requested_roles: SemanticSessionRolesV1::top(),
        });
        let SemanticSessionOutcomeV1::Joined { membership, .. } = joined.outcome() else {
            panic!("managed refusal join was refused")
        };
        let _membership = *membership;
        let SemanticSessionActorEventAdmissionV1::Submitted(policy) =
            actor.try_event(SemanticSessionActorExternalEventV1::PolicyLost {
                subject: context.subject,
                session: context.session,
                policy_epoch: context.policy_epoch,
            })
        else {
            panic!("managed refusal policy event was not admitted")
        };
        assert_eq!(
            policy.receive(),
            SemanticSessionActorControlCompletionV1::Applied
        );
        assert!(actor.begin_drain());
        let mut ticket = actor.try_begin_stop().unwrap();
        let refusal = loop {
            match ticket.try_complete() {
                Ok(LocalOrderedManagedStopResolutionV1::Refused(refusal)) => break refusal,
                Ok(LocalOrderedManagedStopResolutionV1::Terminal(_)) => {
                    panic!("live retirement obligation stopped cleanly")
                }
                Err(pending) => {
                    ticket = *pending;
                    std::thread::yield_now();
                }
            }
        };
        assert_eq!(
            refusal.receipt().disposition(),
            SemanticSessionActorStopDispositionV1::Refused(
                SemanticSessionActorControlRefusalV1::NotQuiescent
            )
        );
        let actor = refusal.into_actor();
        assert_eq!(actor.phase(), SemanticSessionActorPhaseV1::Draining);
        let mut actor = Some(actor);
        let mut stopped = None;
        'retirement: for _ in 0..8 {
            let current = actor.take().expect("refusal retains managed actor");
            assert_eq!(
                current.try_drive_retirement().unwrap().receive(),
                SemanticSessionActorControlCompletionV1::Applied
            );
            let mut ticket = current.try_begin_stop().unwrap();
            loop {
                match ticket.try_complete() {
                    Ok(LocalOrderedManagedStopResolutionV1::Terminal(terminal)) => {
                        stopped = Some(terminal);
                        break 'retirement;
                    }
                    Ok(LocalOrderedManagedStopResolutionV1::Refused(refusal)) => {
                        actor = Some(refusal.into_actor());
                        break;
                    }
                    Err(pending) => {
                        ticket = *pending;
                        std::thread::yield_now();
                    }
                }
            }
        }
        let mut stopped = stopped.expect("bounded retirement must make stop quiescent");
        let cleanup = loop {
            match stopped.try_advance() {
                LocalOrderedStoppedRecoveryAdvanceV1::Pending(pending) => {
                    stopped = *pending;
                    std::thread::yield_now();
                }
                LocalOrderedStoppedRecoveryAdvanceV1::Ready(cleanup) => break *cleanup,
                LocalOrderedStoppedRecoveryAdvanceV1::Mismatch(_) => {
                    panic!("clean retry mismatched recovery")
                }
            }
        };
        let evidence = cleanup.origin().actor_stop().unwrap();
        assert!(evidence.refusal_attempts() >= 1);
        assert!(matches!(
            evidence.last_refusal().map(|receipt| receipt.disposition()),
            Some(SemanticSessionActorStopDispositionV1::Refused(
                SemanticSessionActorControlRefusalV1::NotQuiescent
            ))
        ));
        assert!(matches!(
            evidence.terminal(),
            LocalOrderedActorTerminalStopV1::Clean(_)
        ));
        drop(sender);
    }

    #[test]
    fn managed_abandonment_is_retained_without_claiming_actor_termination() {
        let (prepared, _) = prepared(provider(ready(2, 1)), 2);
        let (actor, sender) = prepared
            .spawn_managed(SemanticSessionActorLimitsV1::reference())
            .unwrap();
        assert!(actor.begin_drain());
        let mut stopped = actor.try_begin_stop().unwrap().abandon();
        assert!(matches!(
            stopped.stop().terminal(),
            LocalOrderedActorTerminalStopV1::Abandoned(receipt)
                if receipt.phase_before_abandonment() == SemanticSessionActorPhaseV1::Stopping
        ));
        let cleanup = loop {
            match stopped.try_advance() {
                LocalOrderedStoppedRecoveryAdvanceV1::Pending(pending) => {
                    stopped = *pending;
                    std::thread::yield_now();
                }
                LocalOrderedStoppedRecoveryAdvanceV1::Ready(cleanup) => break *cleanup,
                LocalOrderedStoppedRecoveryAdvanceV1::Mismatch(_) => {
                    panic!("abandonment must not impose a termination class")
                }
            }
        };
        assert!(matches!(
            cleanup.origin().actor_stop().unwrap().terminal(),
            LocalOrderedActorTerminalStopV1::Abandoned(_)
        ));
        assert!(matches!(
            cleanup.origin().origin(),
            LocalOrderedCleanupOriginV1::ActorRecovered(
                SemanticSessionActorTerminationV1::Clean
                    | SemanticSessionActorTerminationV1::Failed
            )
        ));
        drop(sender);
    }

    #[test]
    fn managed_provider_failure_before_stop_requires_failed_recovery_witness() {
        let mut failed_provider = provider(ready(4, 1));
        failed_provider.poll_failure = Some(SemanticBridgePollFailureV1::RuntimeUnavailable);
        let (prepared, _) = prepared(failed_provider, 2);
        let (actor, sender) = prepared
            .spawn_managed(SemanticSessionActorLimitsV1::reference())
            .unwrap();
        assert_eq!(
            actor.try_drive_provider().unwrap().receive(),
            vellum_semantic_session_actor::SemanticSessionActorControlCompletionV1::OutcomeUnknown
        );
        for _ in 0..100_000 {
            if actor.phase() == SemanticSessionActorPhaseV1::Failed {
                break;
            }
            std::thread::yield_now();
        }
        assert_eq!(actor.phase(), SemanticSessionActorPhaseV1::Failed);
        let mut stopped = actor
            .try_recover_failed()
            .expect("failed managed actor must retain recovery authority");
        assert!(matches!(
            stopped.stop().terminal(),
            LocalOrderedActorTerminalStopV1::FailedBeforeStop { usage }
                if usage.provider_poll_failures == 1
        ));
        let cleanup = loop {
            match stopped.try_advance() {
                LocalOrderedStoppedRecoveryAdvanceV1::Pending(pending) => {
                    stopped = *pending;
                    std::thread::yield_now();
                }
                LocalOrderedStoppedRecoveryAdvanceV1::Ready(cleanup) => break *cleanup,
                LocalOrderedStoppedRecoveryAdvanceV1::Mismatch(_) => {
                    panic!("failed-before-stop witness mismatched failed recovery")
                }
            }
        };
        assert_eq!(
            cleanup.origin().origin(),
            LocalOrderedCleanupOriginV1::ActorRecovered(SemanticSessionActorTerminationV1::Failed)
        );
        assert!(matches!(
            cleanup.origin().actor_stop().unwrap().terminal(),
            LocalOrderedActorTerminalStopV1::FailedBeforeStop { .. }
        ));
        drop(sender);
    }

    #[test]
    fn managed_stop_outcome_unknown_joins_failed_same_spawn_recovery() {
        use std::sync::mpsc;
        use vellum_semantic_session_actor::{
            SemanticSessionActorControlCompletionV1, SemanticSessionActorPollV1,
        };

        let (entered_tx, entered_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let mut gated_provider = provider(ready(6, 1));
        gated_provider.poll_entered = Some(entered_tx);
        gated_provider.poll_release = Some(release_rx);
        gated_provider.panic_after_poll_release = true;
        let (prepared, _) = prepared(gated_provider, 2);
        let (actor, sender) = prepared
            .spawn_managed(SemanticSessionActorLimitsV1::reference())
            .unwrap();
        let drive = actor.try_drive_provider().unwrap();
        entered_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("provider poll did not enter");
        assert!(actor.begin_drain());
        let mut ticket = actor.try_begin_stop().unwrap();
        ticket = match ticket.try_complete() {
            Err(pending) => *pending,
            Ok(_) => panic!("blocked provider resolved an admitted stop before release"),
        };
        assert_eq!(sender.phase(), SemanticSessionActorPhaseV1::Stopping);
        release_tx.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            match drive.try_receive() {
                Ok(completion) => {
                    assert_eq!(
                        completion,
                        SemanticSessionActorControlCompletionV1::OutcomeUnknown
                    );
                    break;
                }
                Err(SemanticSessionActorPollV1::Pending) => {}
                Err(SemanticSessionActorPollV1::OutcomeUnknown) => break,
            }
            assert!(Instant::now() < deadline, "provider panic drive deadline");
            std::thread::yield_now();
        }
        let mut stopped = loop {
            match ticket.try_complete() {
                Ok(LocalOrderedManagedStopResolutionV1::Terminal(stopped)) => break stopped,
                Ok(LocalOrderedManagedStopResolutionV1::Refused(_)) => {
                    panic!("panicked worker returned a stop refusal")
                }
                Err(pending) => ticket = *pending,
            }
            assert!(Instant::now() < deadline, "outcome-unknown stop deadline");
            std::thread::yield_now();
        };
        assert!(matches!(
            stopped.stop().terminal(),
            LocalOrderedActorTerminalStopV1::OutcomeUnknown(receipt)
                if receipt.observed_phase() == SemanticSessionActorPhaseV1::Failed
                    && receipt.usage().worker_panics == 1
                    && receipt.usage().stop_outcome_unknown == 1
        ));
        let cleanup = loop {
            match stopped.try_advance() {
                LocalOrderedStoppedRecoveryAdvanceV1::Pending(pending) => stopped = *pending,
                LocalOrderedStoppedRecoveryAdvanceV1::Ready(cleanup) => break *cleanup,
                LocalOrderedStoppedRecoveryAdvanceV1::Mismatch(_) => {
                    panic!("outcome unknown made a false termination claim")
                }
            }
            assert!(
                Instant::now() < deadline,
                "outcome-unknown recovery deadline"
            );
            std::thread::yield_now();
        };
        assert_eq!(
            cleanup.origin().origin(),
            LocalOrderedCleanupOriginV1::ActorRecovered(SemanticSessionActorTerminationV1::Failed)
        );
        assert_eq!(
            cleanup.origin().actor_recovery_join(),
            LocalOrderedActorRecoveryJoinV1::Compatible
        );
        assert!(matches!(
            cleanup.origin().actor_stop().unwrap().terminal(),
            LocalOrderedActorTerminalStopV1::OutcomeUnknown(_)
        ));
        drop(sender);
    }

    #[test]
    fn defensive_stop_recovery_mismatch_remains_explicit_in_cleanup_origin() {
        use vellum_semantic_session_actor::{
            SemanticSessionActorControlCompletionV1, SemanticSessionActorPollV1,
        };

        let deadline = Instant::now() + Duration::from_secs(3);
        let (prepared_a, _) = prepared(provider(ready(7, 1)), 2);
        let (actor_a, sender_a) = prepared_a
            .spawn_managed(SemanticSessionActorLimitsV1::reference())
            .unwrap();
        assert!(actor_a.begin_drain());
        let mut ticket = actor_a.try_begin_stop().unwrap();
        let mut stopped_a = loop {
            match ticket.try_complete() {
                Ok(LocalOrderedManagedStopResolutionV1::Terminal(stopped)) => break stopped,
                Ok(LocalOrderedManagedStopResolutionV1::Refused(_)) => {
                    panic!("quiescent actor A refused stop")
                }
                Err(pending) => ticket = *pending,
            }
            assert!(Instant::now() < deadline, "actor A stop deadline");
            std::thread::yield_now();
        };
        let clean_stop = stopped_a.stop();
        assert!(matches!(
            clean_stop.terminal(),
            LocalOrderedActorTerminalStopV1::Clean(_)
        ));
        let cleanup_a = loop {
            match stopped_a.try_advance() {
                LocalOrderedStoppedRecoveryAdvanceV1::Pending(pending) => stopped_a = *pending,
                LocalOrderedStoppedRecoveryAdvanceV1::Ready(cleanup) => break *cleanup,
                LocalOrderedStoppedRecoveryAdvanceV1::Mismatch(_) => {
                    panic!("actor A clean stop mismatched its own recovery")
                }
            }
            assert!(Instant::now() < deadline, "actor A recovery deadline");
            std::thread::yield_now();
        };
        assert_eq!(
            cleanup_a.origin().actor_recovery_join(),
            LocalOrderedActorRecoveryJoinV1::Compatible
        );
        drop(sender_a);

        let mut failed_provider = provider(ready(8, 1));
        failed_provider.poll_failure = Some(SemanticBridgePollFailureV1::RuntimeUnavailable);
        let (prepared_b, _) = prepared(failed_provider, 2);
        let (owner_b, sender_b, recovery_b) = prepared_b
            .spawn(SemanticSessionActorLimitsV1::reference())
            .unwrap();
        let drive_b = owner_b.try_drive_provider().unwrap();
        let mismatch_deadline = Instant::now() + Duration::from_secs(3);
        loop {
            match drive_b.try_receive() {
                Ok(completion) => {
                    assert_eq!(
                        completion,
                        SemanticSessionActorControlCompletionV1::OutcomeUnknown
                    );
                    break;
                }
                Err(SemanticSessionActorPollV1::Pending) => {}
                Err(SemanticSessionActorPollV1::OutcomeUnknown) => break,
            }
            assert!(
                Instant::now() < mismatch_deadline,
                "actor B failure deadline"
            );
            std::thread::yield_now();
        }
        drop(owner_b);
        drop(sender_b);
        let mut forged = LocalOrderedStoppedRecoveryOwnerV1 {
            stop: clean_stop,
            recovery: recovery_b,
        };
        let mismatch = loop {
            match forged.try_advance() {
                LocalOrderedStoppedRecoveryAdvanceV1::Pending(pending) => forged = *pending,
                LocalOrderedStoppedRecoveryAdvanceV1::Ready(_) => {
                    panic!("clean stop was accepted with failed recovery")
                }
                LocalOrderedStoppedRecoveryAdvanceV1::Mismatch(mismatch) => break mismatch,
            }
            assert!(
                Instant::now() < mismatch_deadline,
                "mismatch recovery deadline"
            );
            std::thread::yield_now();
        };
        assert_eq!(
            mismatch.termination(),
            SemanticSessionActorTerminationV1::Failed
        );
        let cleanup_b = mismatch.into_cleanup();
        assert_eq!(
            cleanup_b.origin().actor_recovery_join(),
            LocalOrderedActorRecoveryJoinV1::Mismatch
        );
        assert!(matches!(
            cleanup_b.origin().actor_stop().unwrap().terminal(),
            LocalOrderedActorTerminalStopV1::Clean(_)
        ));
    }

    #[test]
    fn managed_timed_stop_joins_recovery_before_scheduler_retirement() {
        let (prepared, _) = prepared(provider(ready(5, 1)), 2);
        let timed = prepared
            .try_with_deadlines(ScriptClock::ticks([0]), deadline_policy(2, 10))
            .unwrap();
        let (actor, sender) = timed
            .spawn_managed(SemanticSessionActorLimitsV1::reference())
            .unwrap();
        assert!(actor.begin_drain());
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut ticket = actor.try_begin_stop().unwrap();
        let mut stopped = loop {
            match ticket.try_complete() {
                Ok(LocalOrderedManagedTimedStopResolutionV1::Terminal(stopped)) => break stopped,
                Ok(LocalOrderedManagedTimedStopResolutionV1::Refused(_)) => {
                    panic!("quiescent timed actor refused stop")
                }
                Err(pending) => ticket = *pending,
            }
            assert!(Instant::now() < deadline, "timed managed stop deadline");
            std::thread::yield_now();
        };
        let timed_cleanup = loop {
            match stopped.try_advance() {
                LocalOrderedTimedStoppedRecoveryAdvanceV1::Pending(pending) => {
                    stopped = *pending;
                }
                LocalOrderedTimedStoppedRecoveryAdvanceV1::Ready(cleanup) => break *cleanup,
                LocalOrderedTimedStoppedRecoveryAdvanceV1::Mismatch(_) => {
                    panic!("same-spawn timed stop mismatched recovery")
                }
            }
            assert!(Instant::now() < deadline, "timed managed recovery deadline");
            std::thread::yield_now();
        };
        assert_eq!(
            timed_cleanup.origin.actor_recovery_join(),
            LocalOrderedActorRecoveryJoinV1::Compatible
        );
        assert!(matches!(
            timed_cleanup.origin.actor_stop().unwrap().terminal(),
            LocalOrderedActorTerminalStopV1::Clean(_)
        ));
        let cleanup = timed_cleanup.try_retire_deadlines().unwrap();
        assert_eq!(
            cleanup.deadlines().disposition(),
            LocalOrderedDeadlineDispositionV1::Retired
        );
        assert_eq!(
            cleanup.origin().actor_recovery_join(),
            LocalOrderedActorRecoveryJoinV1::Compatible
        );
        drop(sender);
    }

    #[test]
    fn timed_owner_retains_unresolved_graph_until_explicit_abandonment() {
        let (timed, _) = corrupted_timed();
        let owner = LocalOrderedTimedCleanupOwnerV1::from_unstarted(timed);
        let owner = owner.try_retire_deadlines().unwrap_err();
        assert_eq!(
            owner.unresolved().unwrap().disposition(),
            LocalOrderedDeadlineDispositionV1::Unresolved
        );
        let owner = owner.abandon_deadlines();
        assert_eq!(
            owner.deadlines().disposition(),
            LocalOrderedDeadlineDispositionV1::Abandoned
        );
    }

    #[test]
    fn mismatched_limits_refuse_before_service_construction() {
        let (session, mut service) = limits(2);
        service.max_clients = 2;
        let failure = LocalOrderedSemanticPreparedV1::try_new_injected_provider_without_deadlines(
            provider(ready(1, 1)),
            client(),
            incarnation(),
            session,
            service,
        )
        .unwrap_err();
        assert_eq!(
            failure.reason(),
            LocalOrderedSemanticBuildRefusalV1::InvalidLimits
        );
        let LocalOrderedSemanticBuildResidueV1::UninstalledProvider(provider) =
            failure.into_residue()
        else {
            panic!("limit refusal must retain the raw provider")
        };
        assert_eq!(provider.state().provider, provenance(1));
    }
}
