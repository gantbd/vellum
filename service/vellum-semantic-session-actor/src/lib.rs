//! Bounded serialized actor shell for the pure semantic-session broker.
//!
//! Authentication, sockets, clocks, persistence, and provider lifecycle remain
//! composition concerns. An authenticated composition supplies broker contexts
//! and trusted events. Effects are interpreted synchronously so an affine
//! continuation never crosses a mailbox turn. Mailbox admission is the only
//! `NotSent` boundary; loss after admission is reported as `OutcomeUnknown`.
#![forbid(unsafe_code)]

use std::fmt;
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use vellum_host::authority::{AuthoritySessionRefV1, AuthoritySubjectRefV1};
use vellum_host::semantic::{SemanticRequestRefV1, SemanticRequestV1, SemanticTargetV1};
use vellum_semantic_protocol::{
    SemanticExchangeDispositionV1, SemanticSessionCommandV1, SemanticSessionReplyV1,
    SemanticSessionSnapshotV1,
};
use vellum_semantic_session_broker::{
    SemanticBrokerCancellationReportV1, SemanticBrokerContextV1, SemanticBrokerDecisionV1,
    SemanticBrokerEffectV1, SemanticBrokerEventV1, SemanticBrokerExpiryDriveV1,
    SemanticBrokerOpenReportV1, SemanticBrokerProviderDriveDispositionV1,
    SemanticBrokerProviderDriveFailureClassV1, SemanticBrokerProviderDriveFailureReportV1,
    SemanticBrokerProviderDriveReportV1, SemanticBrokerSourceReportV1,
    SemanticBrokerSubmissionReportV1, SemanticBrokerUsageV1, SemanticSessionBrokerV1,
};

pub const MAX_SEMANTIC_ACTOR_COMMANDS_V1: usize = 4_096;
pub const MAX_SEMANTIC_ACTOR_CONTROLS_V1: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticSessionActorLimitsV1 {
    pub max_commands: usize,
    pub max_controls: usize,
}

impl SemanticSessionActorLimitsV1 {
    pub const fn reference() -> Self {
        Self {
            max_commands: 256,
            max_controls: 32,
        }
    }

    pub const fn is_valid(self) -> bool {
        self.max_commands > 0
            && self.max_commands <= MAX_SEMANTIC_ACTOR_COMMANDS_V1
            && self.max_controls > 0
            && self.max_controls <= MAX_SEMANTIC_ACTOR_CONTROLS_V1
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticSessionActorPhaseV1 {
    Running,
    Draining,
    Stopping,
    Stopped,
    Failed,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SemanticSessionActorUsageV1 {
    pub commands_admitted: u64,
    pub commands_full: u64,
    pub commands_draining: u64,
    pub commands_closed: u64,
    pub controls_admitted: u64,
    pub controls_full: u64,
    pub replies: u64,
    pub replies_dropped: u64,
    pub effects: u64,
    pub effects_by_kind: [u64; 5],
    pub effect_failures: u64,
    pub effect_failures_by_kind: [u64; 5],
    pub effect_failures_by_reason: [u64; 3],
    pub provider_drives: u64,
    pub provider_poll_failures: u64,
    pub provider_poll_failures_by_reason: [u64; 3],
    pub expiry_drives: u64,
    pub expiry_no_due: u64,
    pub expiry_events: u64,
    pub expiry_failures: u64,
    pub expiry_failures_by_reason: [u64; 3],
    pub broker_divergences: u64,
    pub provider_events: u64,
    pub policy_events: u64,
    pub retirement_drives: u64,
    pub outcome_unknown: u64,
    pub quarantined: u64,
    pub worker_panics: u64,
    pub drain_requests: u64,
    pub stop_tickets: u64,
    pub stop_ticket_polls: u64,
    pub stop_ticket_pending: u64,
    pub stop_abandonments: u64,
    pub stop_outcome_unknown: u64,
    pub stop_refusals: u64,
    pub clean_stops: u64,
    pub queued_commands: usize,
    pub in_flight_commands: usize,
    pub command_high_water: usize,
    pub queued_controls: usize,
    pub control_high_water: usize,
    pub saturated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticSessionActorBuildRefusalV1 {
    InvalidLimits,
    ThreadUnavailable,
}

/// Failed actor construction retains the exact broker and effect port. A
/// thread-launch refusal must not destroy the only client/provider cleanup
/// authority merely because no worker was created.
#[must_use = "failed actor construction retains runtime cleanup authority"]
pub struct SemanticSessionActorBuildFailureV1<P> {
    reason: SemanticSessionActorBuildRefusalV1,
    limits: SemanticSessionActorLimitsV1,
    broker: SemanticSessionBrokerV1,
    port: P,
}

impl<P> SemanticSessionActorBuildFailureV1<P> {
    pub const fn reason(&self) -> SemanticSessionActorBuildRefusalV1 {
        self.reason
    }

    pub const fn limits(&self) -> SemanticSessionActorLimitsV1 {
        self.limits
    }

    pub fn into_parts(
        self,
    ) -> (
        SemanticSessionActorBuildRefusalV1,
        SemanticSessionActorLimitsV1,
        SemanticSessionBrokerV1,
        P,
    ) {
        (self.reason, self.limits, self.broker, self.port)
    }
}

impl<P> fmt::Debug for SemanticSessionActorBuildFailureV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticSessionActorBuildFailureV1")
            .field("reason", &self.reason)
            .field("limits", &self.limits)
            .field("broker", &"<opaque>")
            .field("port", &"<opaque>")
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticSessionActorNotSentReasonV1 {
    Full,
    Draining,
    Closed,
}

#[must_use = "admission determines whether the exact command was sent"]
pub enum SemanticSessionActorAdmissionV1 {
    Submitted(SemanticSessionActorReplyTicketV1),
    NotSent {
        reason: SemanticSessionActorNotSentReasonV1,
        command: SemanticSessionCommandV1,
    },
}

impl fmt::Debug for SemanticSessionActorAdmissionV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Submitted(_) => formatter.write_str("Submitted(<reply-ticket>)"),
            Self::NotSent { reason, .. } => formatter
                .debug_struct("NotSent")
                .field("reason", reason)
                .field("command", &"<redacted>")
                .finish(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticSessionActorPollV1 {
    Pending,
    OutcomeUnknown,
}

#[must_use = "an admitted exchange has transport-phase truth that must be observed"]
pub struct SemanticSessionActorReplyTicketV1 {
    receiver: Receiver<SemanticExchangeDispositionV1<SemanticSessionReplyV1>>,
}

impl fmt::Debug for SemanticSessionActorReplyTicketV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SemanticSessionActorReplyTicketV1(<opaque>)")
    }
}

impl SemanticSessionActorReplyTicketV1 {
    pub fn try_receive(
        &self,
    ) -> Result<SemanticExchangeDispositionV1<SemanticSessionReplyV1>, SemanticSessionActorPollV1>
    {
        match self.receiver.try_recv() {
            Ok(disposition) => Ok(disposition),
            Err(TryRecvError::Empty) => Err(SemanticSessionActorPollV1::Pending),
            Err(TryRecvError::Disconnected) => Err(SemanticSessionActorPollV1::OutcomeUnknown),
        }
    }

    pub fn receive(self) -> SemanticExchangeDispositionV1<SemanticSessionReplyV1> {
        self.receiver
            .recv()
            .unwrap_or(SemanticExchangeDispositionV1::OutcomeUnknown)
    }
}

/// Trusted synchronous interpreter. Implementations receive only domain
/// payloads and return closed outcomes or a closed effect failure. A failure
/// carries no ordered report and therefore makes the admitted exchange outcome
/// unknown; the actor fails without consuming the affine broker continuation
/// and preserves the exact port for recovery. Affine broker continuations
/// remain private to the actor, which constructs the exact matching completion
/// event. Implementations must not block or retain source-bearing payloads.
pub trait SemanticBrokerEffectPortV1: Send + 'static {
    /// Fixed-cardinality cleanup obligations owned outside the broker. The
    /// observation is mandatory so a newly introduced obligation-owning port
    /// cannot silently inherit an optimistic empty state.
    fn obligations(&self) -> SemanticBrokerEffectObligationsV1;

    fn open(
        &mut self,
        snapshot: &SemanticSessionSnapshotV1,
    ) -> Result<SemanticBrokerOpenReportV1, SemanticBrokerEffectFailureV1>;
    fn change(
        &mut self,
        snapshot: &SemanticSessionSnapshotV1,
    ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1>;
    fn close(
        &mut self,
        target: SemanticTargetV1,
    ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1>;
    fn submit(
        &mut self,
        request: &SemanticRequestV1,
    ) -> Result<SemanticBrokerSubmissionReportV1, SemanticBrokerEffectFailureV1>;
    fn cancel(
        &mut self,
        request: SemanticRequestRefV1,
    ) -> Result<SemanticBrokerCancellationReportV1, SemanticBrokerEffectFailureV1>;

    /// Performs exactly one provider poll and returns its complete ordered
    /// transition/delivery product. Failures are never collapsed into absence;
    /// a failure has no post-provider observation. No clock, retry, or loop is
    /// implied.
    fn drive_provider(
        &mut self,
    ) -> Result<SemanticBrokerProviderDriveV1, SemanticBrokerEffectFailureV1>;

    /// Examines at most one due composition-owned deadline. Timeout is never a
    /// provider delivery; a due activation carries its complete ordered expiry
    /// report and `NoDue` means the scheduler check itself succeeded.
    fn drive_expiry(
        &mut self,
    ) -> Result<SemanticBrokerExpiryDriveV1, SemanticBrokerEffectFailureV1>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticBrokerEffectObligationsV1 {
    pub request_deadlines: usize,
    pub due_deadlines: usize,
    pub poisoned: bool,
    pub consistent: bool,
}

impl SemanticBrokerEffectObligationsV1 {
    pub const fn is_empty(self) -> bool {
        self.request_deadlines == 0 && self.due_deadlines == 0 && !self.poisoned && self.consistent
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticBrokerEffectFailureV1 {
    Poisoned,
    RuntimeUnavailable,
    DeliveryRefused,
}

#[must_use = "provider drive completion or failure evidence must be reconciled"]
#[derive(Clone, Eq, PartialEq)]
pub enum SemanticBrokerProviderDriveV1 {
    Completed(Box<SemanticBrokerProviderDriveReportV1>),
    Failed(Box<SemanticBrokerProviderDriveFailureReportV1>),
}

impl fmt::Debug for SemanticBrokerProviderDriveV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Completed(_) => "Completed",
            Self::Failed(_) => "Failed",
        })
    }
}

#[derive(Clone)]
pub struct SemanticSessionActorSenderV1 {
    commands: SyncSender<CommandEnvelope>,
    shared: Arc<Mutex<Shared>>,
    limits: SemanticSessionActorLimitsV1,
}

impl fmt::Debug for SemanticSessionActorSenderV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SemanticSessionActorSenderV1(<opaque>)")
    }
}

impl SemanticSessionActorSenderV1 {
    pub fn try_exchange(
        &self,
        context: SemanticBrokerContextV1,
        command: SemanticSessionCommandV1,
    ) -> SemanticSessionActorAdmissionV1 {
        let (reply, receiver) = mpsc::sync_channel(1);
        let mut shared = lock_shared(&self.shared);
        let reason = match shared.phase {
            SemanticSessionActorPhaseV1::Running => {
                if shared.usage.queued_commands == self.limits.max_commands {
                    bump_usage(&mut shared.usage, Counter::CommandsFull);
                    Some(SemanticSessionActorNotSentReasonV1::Full)
                } else {
                    shared.usage.queued_commands += 1;
                    shared.usage.command_high_water = shared
                        .usage
                        .command_high_water
                        .max(shared.usage.queued_commands);
                    None
                }
            }
            SemanticSessionActorPhaseV1::Draining | SemanticSessionActorPhaseV1::Stopping => {
                bump_usage(&mut shared.usage, Counter::CommandsDraining);
                Some(SemanticSessionActorNotSentReasonV1::Draining)
            }
            SemanticSessionActorPhaseV1::Stopped | SemanticSessionActorPhaseV1::Failed => {
                bump_usage(&mut shared.usage, Counter::CommandsClosed);
                Some(SemanticSessionActorNotSentReasonV1::Closed)
            }
        };
        if let Some(reason) = reason {
            return SemanticSessionActorAdmissionV1::NotSent { reason, command };
        }
        match self.commands.try_send(CommandEnvelope {
            context,
            command,
            reply,
        }) {
            Ok(()) => {
                bump_usage(&mut shared.usage, Counter::CommandsAdmitted);
                SemanticSessionActorAdmissionV1::Submitted(SemanticSessionActorReplyTicketV1 {
                    receiver,
                })
            }
            Err(TrySendError::Full(envelope)) => {
                shared.usage.queued_commands -= 1;
                bump_usage(&mut shared.usage, Counter::CommandsFull);
                SemanticSessionActorAdmissionV1::NotSent {
                    reason: SemanticSessionActorNotSentReasonV1::Full,
                    command: envelope.command,
                }
            }
            Err(TrySendError::Disconnected(envelope)) => {
                shared.usage.queued_commands -= 1;
                bump_usage(&mut shared.usage, Counter::CommandsClosed);
                SemanticSessionActorAdmissionV1::NotSent {
                    reason: SemanticSessionActorNotSentReasonV1::Closed,
                    command: envelope.command,
                }
            }
        }
    }

    pub fn usage(&self) -> SemanticSessionActorUsageV1 {
        lock_shared(&self.shared).usage
    }

    pub fn phase(&self) -> SemanticSessionActorPhaseV1 {
        lock_shared(&self.shared).phase
    }
}

pub struct SemanticSessionActorOwnerV1 {
    controls: SyncSender<ControlEnvelope>,
    shared: Arc<Mutex<Shared>>,
    limits: SemanticSessionActorLimitsV1,
}

impl fmt::Debug for SemanticSessionActorOwnerV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SemanticSessionActorOwnerV1(<opaque>)")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticSessionActorControlRefusalV1 {
    Full,
    Closed,
    NotDraining,
    NotQuiescent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticSessionActorControlCompletionV1 {
    Applied,
    Quarantined,
    OutcomeUnknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticSessionActorStopDispositionV1 {
    Clean,
    Refused(SemanticSessionActorControlRefusalV1),
    OutcomeUnknown,
}

#[must_use = "actor stop evidence must be joined with recovery evidence"]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticSessionActorStopReceiptV1 {
    disposition: SemanticSessionActorStopDispositionV1,
    observed_phase: SemanticSessionActorPhaseV1,
    usage: SemanticSessionActorUsageV1,
}

impl SemanticSessionActorStopReceiptV1 {
    pub const fn disposition(&self) -> SemanticSessionActorStopDispositionV1 {
        self.disposition
    }

    pub const fn observed_phase(&self) -> SemanticSessionActorPhaseV1 {
        self.observed_phase
    }

    pub const fn usage(&self) -> SemanticSessionActorUsageV1 {
        self.usage
    }
}

#[must_use = "failed stop admission retains the exact actor owner"]
pub struct SemanticSessionActorStopStartFailureV1 {
    reason: SemanticSessionActorControlRefusalV1,
    owner: SemanticSessionActorOwnerV1,
}

impl SemanticSessionActorStopStartFailureV1 {
    pub const fn reason(&self) -> SemanticSessionActorControlRefusalV1 {
        self.reason
    }

    pub fn into_owner(self) -> SemanticSessionActorOwnerV1 {
        self.owner
    }
}

impl fmt::Debug for SemanticSessionActorStopStartFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticSessionActorStopStartFailureV1")
            .field("reason", &self.reason)
            .field("owner", &"<owned>")
            .finish()
    }
}

#[must_use = "an admitted actor stop must resolve, retry, or be explicitly abandoned"]
pub struct SemanticSessionActorStopTicketV1 {
    owner: Option<SemanticSessionActorOwnerV1>,
    receiver: Receiver<Result<(), SemanticSessionActorControlRefusalV1>>,
}

impl fmt::Debug for SemanticSessionActorStopTicketV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SemanticSessionActorStopTicketV1(<opaque>)")
    }
}

#[must_use = "resolved stop evidence and any retry owner must be retained"]
pub enum SemanticSessionActorStopResolutionV1 {
    Clean(SemanticSessionActorStopReceiptV1),
    Refused {
        receipt: SemanticSessionActorStopReceiptV1,
        owner: SemanticSessionActorOwnerV1,
    },
    OutcomeUnknown(SemanticSessionActorStopReceiptV1),
}

impl fmt::Debug for SemanticSessionActorStopResolutionV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Clean(receipt) => formatter.debug_tuple("Clean").field(receipt).finish(),
            Self::Refused { receipt, .. } => formatter
                .debug_struct("Refused")
                .field("receipt", receipt)
                .field("owner", &"<owned>")
                .finish(),
            Self::OutcomeUnknown(receipt) => formatter
                .debug_tuple("OutcomeUnknown")
                .field(receipt)
                .finish(),
        }
    }
}

#[must_use = "actor stop abandonment is not actor termination evidence"]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticSessionActorStopAbandonmentV1 {
    phase_before_abandonment: SemanticSessionActorPhaseV1,
    usage: SemanticSessionActorUsageV1,
}

impl SemanticSessionActorStopAbandonmentV1 {
    pub const fn phase_before_abandonment(&self) -> SemanticSessionActorPhaseV1 {
        self.phase_before_abandonment
    }

    pub const fn usage(&self) -> SemanticSessionActorUsageV1 {
        self.usage
    }
}

impl SemanticSessionActorStopTicketV1 {
    /// Observes one stop completion attempt without blocking. Pending returns
    /// the exact ticket unchanged; a quiescence refusal returns the owner so
    /// retirement can make progress before another stop attempt.
    pub fn try_complete(mut self) -> Result<SemanticSessionActorStopResolutionV1, Self> {
        {
            let owner = self.owner.as_ref().expect("stop ticket owns actor");
            let mut shared = lock_shared(&owner.shared);
            bump_usage(&mut shared.usage, Counter::StopTicketPolls);
        }
        match self.receiver.try_recv() {
            Ok(Ok(())) => {
                let owner = self.owner.take().expect("stop ticket owns actor");
                let receipt = SemanticSessionActorStopReceiptV1 {
                    disposition: SemanticSessionActorStopDispositionV1::Clean,
                    observed_phase: owner.phase(),
                    usage: owner.usage(),
                };
                drop(owner);
                Ok(SemanticSessionActorStopResolutionV1::Clean(receipt))
            }
            Ok(Err(reason)) => {
                let owner = self.owner.take().expect("stop ticket owns actor");
                let receipt = SemanticSessionActorStopReceiptV1 {
                    disposition: SemanticSessionActorStopDispositionV1::Refused(reason),
                    observed_phase: owner.phase(),
                    usage: owner.usage(),
                };
                Ok(SemanticSessionActorStopResolutionV1::Refused { receipt, owner })
            }
            Err(TryRecvError::Empty) => {
                let owner = self.owner.as_ref().expect("stop ticket owns actor");
                let mut shared = lock_shared(&owner.shared);
                bump_usage(&mut shared.usage, Counter::StopTicketPending);
                drop(shared);
                Err(self)
            }
            Err(TryRecvError::Disconnected) => {
                let owner = self.owner.take().expect("stop ticket owns actor");
                {
                    let mut shared = lock_shared(&owner.shared);
                    bump_usage(&mut shared.usage, Counter::StopOutcomeUnknown);
                }
                let receipt = SemanticSessionActorStopReceiptV1 {
                    disposition: SemanticSessionActorStopDispositionV1::OutcomeUnknown,
                    observed_phase: owner.phase(),
                    usage: owner.usage(),
                };
                drop(owner);
                Ok(SemanticSessionActorStopResolutionV1::OutcomeUnknown(
                    receipt,
                ))
            }
        }
    }

    /// Explicitly loses the control owner. Recovery remains a separate affine
    /// capability; this receipt records only abandonment, never termination.
    pub fn abandon(mut self) -> SemanticSessionActorStopAbandonmentV1 {
        let owner = self.owner.take().expect("stop ticket owns actor");
        let (phase_before_abandonment, usage) = {
            let mut shared = lock_shared(&owner.shared);
            bump_usage(&mut shared.usage, Counter::StopAbandonments);
            (shared.phase, shared.usage)
        };
        drop(owner);
        SemanticSessionActorStopAbandonmentV1 {
            phase_before_abandonment,
            usage,
        }
    }
}

impl Drop for SemanticSessionActorStopTicketV1 {
    fn drop(&mut self) {
        let Some(owner) = self.owner.take() else {
            return;
        };
        let mut shared = lock_shared(&owner.shared);
        bump_usage(&mut shared.usage, Counter::StopAbandonments);
    }
}

#[must_use = "an admitted control has completion truth that must be observed"]
pub struct SemanticSessionActorControlTicketV1 {
    receiver: Receiver<SemanticSessionActorControlCompletionV1>,
}

impl fmt::Debug for SemanticSessionActorControlTicketV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SemanticSessionActorControlTicketV1(<opaque>)")
    }
}

impl SemanticSessionActorControlTicketV1 {
    pub fn try_receive(
        &self,
    ) -> Result<SemanticSessionActorControlCompletionV1, SemanticSessionActorPollV1> {
        match self.receiver.try_recv() {
            Ok(completion) => Ok(completion),
            Err(TryRecvError::Empty) => Err(SemanticSessionActorPollV1::Pending),
            Err(TryRecvError::Disconnected) => Err(SemanticSessionActorPollV1::OutcomeUnknown),
        }
    }

    pub fn receive(self) -> SemanticSessionActorControlCompletionV1 {
        self.receiver
            .recv()
            .unwrap_or(SemanticSessionActorControlCompletionV1::OutcomeUnknown)
    }
}

#[derive(Eq, PartialEq)]
pub enum SemanticSessionActorExternalEventV1 {
    PolicyLost {
        subject: AuthoritySubjectRefV1,
        session: AuthoritySessionRefV1,
        policy_epoch: u64,
    },
}

impl fmt::Debug for SemanticSessionActorExternalEventV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self {
            Self::PolicyLost { .. } => "PolicyLost",
        };
        formatter
            .debug_struct("SemanticSessionActorExternalEventV1")
            .field("kind", &kind)
            .field("payload", &"<redacted>")
            .finish()
    }
}

impl SemanticSessionActorExternalEventV1 {
    fn into_broker_event(self) -> SemanticBrokerEventV1 {
        match self {
            Self::PolicyLost {
                subject,
                session,
                policy_epoch,
            } => SemanticBrokerEventV1::PolicyLost {
                subject,
                session,
                policy_epoch,
            },
        }
    }
}

#[must_use = "control admission returns exact event ownership when not sent"]
pub enum SemanticSessionActorEventAdmissionV1 {
    Submitted(SemanticSessionActorControlTicketV1),
    NotSent {
        reason: SemanticSessionActorControlRefusalV1,
        event: SemanticSessionActorExternalEventV1,
    },
}

impl fmt::Debug for SemanticSessionActorEventAdmissionV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Submitted(_) => formatter.write_str("Submitted(<control-ticket>)"),
            Self::NotSent { reason, .. } => formatter
                .debug_struct("NotSent")
                .field("reason", reason)
                .field("event", &"<redacted>")
                .finish(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticSessionActorTerminationV1 {
    Clean,
    Failed,
}

/// Affine authority to recover the exact broker and effect port after the
/// worker has terminated. This capability is deliberately separate from the
/// control owner so losing the owner cannot also lose cleanup authority.
#[must_use = "actor recovery authority must be retained until the runtime is reclaimed"]
pub struct SemanticSessionActorRecoveryV1<P> {
    worker: JoinHandle<ActorWorkerRecoveredV1<P>>,
    shared: Arc<Mutex<Shared>>,
}

impl<P> fmt::Debug for SemanticSessionActorRecoveryV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SemanticSessionActorRecoveryV1(<opaque>)")
    }
}

/// Terminal actor state returned by the recovery capability. Clean actor
/// termination does not imply provider or client retirement; the returned
/// broker and port remain the caller's exact cleanup evidence.
#[must_use = "recovered broker and effect-port ownership must be retired or reused explicitly"]
pub struct SemanticSessionActorRecoveredV1<P> {
    termination: SemanticSessionActorTerminationV1,
    usage: SemanticSessionActorUsageV1,
    broker: SemanticSessionBrokerV1,
    port: P,
}

impl<P> fmt::Debug for SemanticSessionActorRecoveredV1<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticSessionActorRecoveredV1")
            .field("termination", &self.termination)
            .field("usage", &self.usage)
            .field("broker", &"<opaque>")
            .field("port", &"<opaque>")
            .finish()
    }
}

impl<P> SemanticSessionActorRecoveredV1<P> {
    pub const fn termination(&self) -> SemanticSessionActorTerminationV1 {
        self.termination
    }

    pub const fn usage(&self) -> SemanticSessionActorUsageV1 {
        self.usage
    }

    pub const fn broker(&self) -> &SemanticSessionBrokerV1 {
        &self.broker
    }

    pub const fn port(&self) -> &P {
        &self.port
    }

    pub fn into_parts(
        self,
    ) -> (
        SemanticSessionActorTerminationV1,
        SemanticSessionActorUsageV1,
        SemanticSessionBrokerV1,
        P,
    ) {
        (self.termination, self.usage, self.broker, self.port)
    }
}

impl<P> SemanticSessionActorRecoveryV1<P> {
    /// Refuses without blocking while the worker still owns the runtime. On
    /// refusal, the exact affine recovery capability is returned unchanged.
    pub fn try_recover(self) -> Result<SemanticSessionActorRecoveredV1<P>, Self> {
        if !self.worker.is_finished() {
            return Err(self);
        }
        let usage = lock_shared(&self.shared).usage;
        let recovered = self
            .worker
            .join()
            .unwrap_or_else(|_| panic!("actor worker escaped its unwind boundary"));
        Ok(SemanticSessionActorRecoveredV1 {
            termination: recovered.termination,
            usage,
            broker: recovered.broker,
            port: recovered.port,
        })
    }
}

impl SemanticSessionActorOwnerV1 {
    pub fn try_event(
        &self,
        event: SemanticSessionActorExternalEventV1,
    ) -> SemanticSessionActorEventAdmissionV1 {
        let (ack, receiver) = mpsc::sync_channel(1);
        match self.try_control(Control::Event { event, ack }, receiver) {
            Ok(ticket) => SemanticSessionActorEventAdmissionV1::Submitted(ticket),
            Err(failure) => match *failure {
                (reason, Control::Event { event, .. }) => {
                    SemanticSessionActorEventAdmissionV1::NotSent { reason, event }
                }
                _ => unreachable!("event admission returned a different control"),
            },
        }
    }

    pub fn try_drive_provider(
        &self,
    ) -> Result<SemanticSessionActorControlTicketV1, SemanticSessionActorControlRefusalV1> {
        let (ack, receiver) = mpsc::sync_channel(1);
        self.try_control(Control::DriveProvider { ack }, receiver)
            .map_err(|failure| failure.0)
    }

    pub fn try_drive_expiry(
        &self,
    ) -> Result<SemanticSessionActorControlTicketV1, SemanticSessionActorControlRefusalV1> {
        let (ack, receiver) = mpsc::sync_channel(1);
        self.try_control(Control::DriveExpiry { ack }, receiver)
            .map_err(|failure| failure.0)
    }

    pub fn try_drive_retirement(
        &self,
    ) -> Result<SemanticSessionActorControlTicketV1, SemanticSessionActorControlRefusalV1> {
        let (ack, receiver) = mpsc::sync_channel(1);
        self.try_control(Control::DriveRetirement { ack }, receiver)
            .map_err(|failure| failure.0)
    }

    pub fn begin_drain(&self) -> bool {
        let mut shared = lock_shared(&self.shared);
        if shared.phase != SemanticSessionActorPhaseV1::Running {
            return false;
        }
        shared.phase = SemanticSessionActorPhaseV1::Draining;
        bump_usage(&mut shared.usage, Counter::DrainRequests);
        true
    }

    /// Compatibility-only blocking stop. New cleanup owners must use
    /// [`Self::try_begin_stop`] and drive its affine ticket without blocking.
    #[doc(hidden)]
    pub fn try_stop(&self) -> Result<(), SemanticSessionActorControlRefusalV1> {
        self.try_admit_stop()?
            .recv()
            .unwrap_or(Err(SemanticSessionActorControlRefusalV1::Closed))
    }

    /// Atomically fences command admission and returns the sole owner of the
    /// in-flight stop observation. Admission failure returns this owner intact.
    pub fn try_begin_stop(
        self,
    ) -> Result<SemanticSessionActorStopTicketV1, SemanticSessionActorStopStartFailureV1> {
        match self.try_admit_stop() {
            Ok(receiver) => {
                {
                    let mut shared = lock_shared(&self.shared);
                    bump_usage(&mut shared.usage, Counter::StopTickets);
                }
                Ok(SemanticSessionActorStopTicketV1 {
                    owner: Some(self),
                    receiver,
                })
            }
            Err(reason) => Err(SemanticSessionActorStopStartFailureV1 {
                reason,
                owner: self,
            }),
        }
    }

    fn try_admit_stop(
        &self,
    ) -> Result<
        Receiver<Result<(), SemanticSessionActorControlRefusalV1>>,
        SemanticSessionActorControlRefusalV1,
    > {
        let (reply, receiver) = mpsc::sync_channel(1);
        {
            let mut shared = lock_shared(&self.shared);
            if shared.phase != SemanticSessionActorPhaseV1::Draining {
                return Err(SemanticSessionActorControlRefusalV1::NotDraining);
            }
            if shared.usage.queued_controls == self.limits.max_controls {
                bump_usage(&mut shared.usage, Counter::ControlsFull);
                return Err(SemanticSessionActorControlRefusalV1::Full);
            }
            shared.phase = SemanticSessionActorPhaseV1::Stopping;
            shared.usage.queued_controls += 1;
            shared.usage.control_high_water = shared
                .usage
                .control_high_water
                .max(shared.usage.queued_controls);
            match self
                .controls
                .try_send(ControlEnvelope(Control::Stop(reply)))
            {
                Ok(()) => bump_usage(&mut shared.usage, Counter::ControlsAdmitted),
                Err(TrySendError::Full(_)) => {
                    shared.phase = SemanticSessionActorPhaseV1::Draining;
                    shared.usage.queued_controls -= 1;
                    bump_usage(&mut shared.usage, Counter::ControlsFull);
                    return Err(SemanticSessionActorControlRefusalV1::Full);
                }
                Err(TrySendError::Disconnected(_)) => {
                    shared.phase = SemanticSessionActorPhaseV1::Failed;
                    shared.usage.queued_controls -= 1;
                    return Err(SemanticSessionActorControlRefusalV1::Closed);
                }
            }
        }
        Ok(receiver)
    }

    pub fn usage(&self) -> SemanticSessionActorUsageV1 {
        lock_shared(&self.shared).usage
    }

    pub fn phase(&self) -> SemanticSessionActorPhaseV1 {
        lock_shared(&self.shared).phase
    }

    fn try_control(
        &self,
        control: Control,
        receiver: Receiver<SemanticSessionActorControlCompletionV1>,
    ) -> Result<
        SemanticSessionActorControlTicketV1,
        Box<(SemanticSessionActorControlRefusalV1, Control)>,
    > {
        let mut shared = lock_shared(&self.shared);
        if matches!(
            shared.phase,
            SemanticSessionActorPhaseV1::Stopping
                | SemanticSessionActorPhaseV1::Stopped
                | SemanticSessionActorPhaseV1::Failed
        ) {
            return Err(Box::new((
                SemanticSessionActorControlRefusalV1::Closed,
                control,
            )));
        }
        if shared.usage.queued_controls == self.limits.max_controls {
            bump_usage(&mut shared.usage, Counter::ControlsFull);
            return Err(Box::new((
                SemanticSessionActorControlRefusalV1::Full,
                control,
            )));
        }
        shared.usage.queued_controls += 1;
        shared.usage.control_high_water = shared
            .usage
            .control_high_water
            .max(shared.usage.queued_controls);
        match self.controls.try_send(ControlEnvelope(control)) {
            Ok(()) => {
                bump_usage(&mut shared.usage, Counter::ControlsAdmitted);
                Ok(SemanticSessionActorControlTicketV1 { receiver })
            }
            Err(TrySendError::Full(ControlEnvelope(control))) => {
                shared.usage.queued_controls -= 1;
                bump_usage(&mut shared.usage, Counter::ControlsFull);
                Err(Box::new((
                    SemanticSessionActorControlRefusalV1::Full,
                    control,
                )))
            }
            Err(TrySendError::Disconnected(ControlEnvelope(control))) => {
                shared.usage.queued_controls -= 1;
                Err(Box::new((
                    SemanticSessionActorControlRefusalV1::Closed,
                    control,
                )))
            }
        }
    }
}

pub fn spawn_semantic_session_actor_v1<P: SemanticBrokerEffectPortV1>(
    broker: SemanticSessionBrokerV1,
    port: P,
    limits: SemanticSessionActorLimitsV1,
) -> Result<
    (
        SemanticSessionActorOwnerV1,
        SemanticSessionActorSenderV1,
        SemanticSessionActorRecoveryV1<P>,
    ),
    Box<SemanticSessionActorBuildFailureV1<P>>,
> {
    spawn_semantic_session_actor_with_v1(broker, port, limits, |worker| {
        thread::Builder::new()
            .name("vellum-semantic-session-actor".into())
            .spawn(worker)
    })
}

type ActorWorkerV1<P> = Box<dyn FnOnce() -> ActorWorkerRecoveredV1<P> + Send + 'static>;

fn spawn_semantic_session_actor_with_v1<P, L>(
    broker: SemanticSessionBrokerV1,
    port: P,
    limits: SemanticSessionActorLimitsV1,
    launch: L,
) -> Result<
    (
        SemanticSessionActorOwnerV1,
        SemanticSessionActorSenderV1,
        SemanticSessionActorRecoveryV1<P>,
    ),
    Box<SemanticSessionActorBuildFailureV1<P>>,
>
where
    P: SemanticBrokerEffectPortV1,
    L: FnOnce(ActorWorkerV1<P>) -> std::io::Result<JoinHandle<ActorWorkerRecoveredV1<P>>>,
{
    if !limits.is_valid() {
        return Err(Box::new(SemanticSessionActorBuildFailureV1 {
            reason: SemanticSessionActorBuildRefusalV1::InvalidLimits,
            limits,
            broker,
            port,
        }));
    }
    let (commands, command_rx) = mpsc::sync_channel(limits.max_commands);
    let (controls, control_rx) = mpsc::sync_channel(limits.max_controls);
    let shared = Arc::new(Mutex::new(Shared::default()));
    let worker_shared = Arc::clone(&shared);
    let runtime = Arc::new(Mutex::new(Some((broker, port))));
    let worker_runtime = Arc::clone(&runtime);
    let worker_fn: ActorWorkerV1<P> = Box::new(move || {
        let (mut broker, mut port) = worker_runtime
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
            .expect("actor runtime is taken exactly once by its worker");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_actor(
                &mut broker,
                &mut port,
                command_rx,
                control_rx,
                &worker_shared,
            );
        }));
        if result.is_err() {
            let mut shared = lock_shared(&worker_shared);
            shared.phase = SemanticSessionActorPhaseV1::Failed;
            bump_usage(&mut shared.usage, Counter::WorkerPanics);
            reconcile_abandoned_queues(&mut shared.usage);
        }
        let termination =
            if lock_shared(&worker_shared).phase == SemanticSessionActorPhaseV1::Stopped {
                SemanticSessionActorTerminationV1::Clean
            } else {
                SemanticSessionActorTerminationV1::Failed
            };
        ActorWorkerRecoveredV1 {
            broker,
            port,
            termination,
        }
    });
    let worker = match launch(worker_fn) {
        Ok(worker) => worker,
        Err(_) => {
            let (broker, port) = runtime
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take()
                .expect("failed launch cannot consume actor runtime");
            return Err(Box::new(SemanticSessionActorBuildFailureV1 {
                reason: SemanticSessionActorBuildRefusalV1::ThreadUnavailable,
                limits,
                broker,
                port,
            }));
        }
    };
    Ok((
        SemanticSessionActorOwnerV1 {
            controls,
            shared: Arc::clone(&shared),
            limits,
        },
        SemanticSessionActorSenderV1 {
            commands,
            shared: Arc::clone(&shared),
            limits,
        },
        SemanticSessionActorRecoveryV1 { worker, shared },
    ))
}

struct ActorWorkerRecoveredV1<P> {
    broker: SemanticSessionBrokerV1,
    port: P,
    termination: SemanticSessionActorTerminationV1,
}

struct CommandEnvelope {
    context: SemanticBrokerContextV1,
    command: SemanticSessionCommandV1,
    reply: SyncSender<SemanticExchangeDispositionV1<SemanticSessionReplyV1>>,
}

struct ControlEnvelope(Control);

enum Control {
    Event {
        event: SemanticSessionActorExternalEventV1,
        ack: SyncSender<SemanticSessionActorControlCompletionV1>,
    },
    DriveProvider {
        ack: SyncSender<SemanticSessionActorControlCompletionV1>,
    },
    DriveExpiry {
        ack: SyncSender<SemanticSessionActorControlCompletionV1>,
    },
    DriveRetirement {
        ack: SyncSender<SemanticSessionActorControlCompletionV1>,
    },
    Stop(SyncSender<Result<(), SemanticSessionActorControlRefusalV1>>),
}

struct Shared {
    phase: SemanticSessionActorPhaseV1,
    usage: SemanticSessionActorUsageV1,
}

impl Default for Shared {
    fn default() -> Self {
        Self {
            phase: SemanticSessionActorPhaseV1::Running,
            usage: SemanticSessionActorUsageV1::default(),
        }
    }
}

fn run_actor<P: SemanticBrokerEffectPortV1>(
    broker: &mut SemanticSessionBrokerV1,
    port: &mut P,
    commands: Receiver<CommandEnvelope>,
    controls: Receiver<ControlEnvelope>,
    shared: &Arc<Mutex<Shared>>,
) {
    let mut commands_disconnected = false;
    let mut controls_disconnected = false;
    loop {
        let mut processed_control = false;
        if !controls_disconnected {
            match controls.try_recv() {
                Ok(ControlEnvelope(control)) => {
                    processed_control = true;
                    release_control(shared);
                    if apply_control(control, broker, port, shared) {
                        return;
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => controls_disconnected = true,
            }
        }
        if controls_disconnected {
            let mut state = lock_shared(shared);
            state.phase = SemanticSessionActorPhaseV1::Failed;
            reconcile_abandoned_queues(&mut state.usage);
            return;
        }
        if !commands_disconnected {
            match commands.recv_timeout(std::time::Duration::from_millis(5)) {
                Ok(envelope) => {
                    release_command(shared);
                    lock_shared(shared).usage.in_flight_commands += 1;
                    let decision = broker.command(envelope.context, envelope.command);
                    if finish_command(decision, envelope.reply, broker, port, shared) {
                        return;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => commands_disconnected = true,
            }
        } else if !controls_disconnected && !processed_control {
            match controls.recv_timeout(std::time::Duration::from_millis(5)) {
                Ok(ControlEnvelope(control)) => {
                    release_control(shared);
                    if apply_control(control, broker, port, shared) {
                        return;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => controls_disconnected = true,
            }
        }
    }
}

fn apply_control<P: SemanticBrokerEffectPortV1>(
    control: Control,
    broker: &mut SemanticSessionBrokerV1,
    port: &mut P,
    shared: &Arc<Mutex<Shared>>,
) -> bool {
    match control {
        Control::Event { event, ack } => {
            let mut state = lock_shared(shared);
            bump_usage(&mut state.usage, Counter::PolicyEvents);
            drop(state);
            let (completion, fatal) = resolve_control(
                broker.event(event.into_broker_event()),
                broker,
                port,
                shared,
            );
            let _ = ack.send(completion);
            if fatal {
                return true;
            }
        }
        Control::DriveProvider { ack } => {
            let mut state = lock_shared(shared);
            bump_usage(&mut state.usage, Counter::ProviderDrives);
            drop(state);
            match port.drive_provider() {
                Ok(SemanticBrokerProviderDriveV1::Completed(report)) => {
                    let has_event = report.pre_provider.is_some()
                        || report.post_provider.is_some()
                        || !matches!(
                            report.disposition,
                            SemanticBrokerProviderDriveDispositionV1::NoEvent
                        );
                    if has_event {
                        let mut state = lock_shared(shared);
                        bump_usage(&mut state.usage, Counter::ProviderEvents);
                    }
                    let (completion, fatal) = resolve_control(
                        broker.event(SemanticBrokerEventV1::ProviderDriveCompleted { report }),
                        broker,
                        port,
                        shared,
                    );
                    let _ = ack.send(completion);
                    if fatal {
                        return true;
                    }
                }
                Ok(SemanticBrokerProviderDriveV1::Failed(report)) => {
                    let failure = report.failure;
                    let decision =
                        broker.event(SemanticBrokerEventV1::ProviderDriveFailed { report });
                    let mut state = lock_shared(shared);
                    state.phase = SemanticSessionActorPhaseV1::Failed;
                    bump_usage(&mut state.usage, Counter::ProviderPollFailures);
                    bump_usage(&mut state.usage, Counter::OutcomeUnknown);
                    if matches!(decision, SemanticBrokerDecisionV1::Diverged(_)) {
                        bump_usage(&mut state.usage, Counter::BrokerDivergences);
                    }
                    let reason = match failure {
                        SemanticBrokerProviderDriveFailureClassV1::Poisoned => 0,
                        SemanticBrokerProviderDriveFailureClassV1::RuntimeUnavailable => 1,
                        SemanticBrokerProviderDriveFailureClassV1::DeliveryRefused => 2,
                    };
                    let saturated =
                        bump_slot(&mut state.usage.provider_poll_failures_by_reason, reason);
                    state.usage.saturated |= saturated;
                    reconcile_abandoned_queues(&mut state.usage);
                    drop(state);
                    let _ = ack.send(SemanticSessionActorControlCompletionV1::OutcomeUnknown);
                    return true;
                }
                Err(failure) => {
                    let mut state = lock_shared(shared);
                    state.phase = SemanticSessionActorPhaseV1::Failed;
                    bump_usage(&mut state.usage, Counter::ProviderPollFailures);
                    bump_usage(&mut state.usage, Counter::OutcomeUnknown);
                    let reason = match failure {
                        SemanticBrokerEffectFailureV1::Poisoned => 0,
                        SemanticBrokerEffectFailureV1::RuntimeUnavailable => 1,
                        SemanticBrokerEffectFailureV1::DeliveryRefused => 2,
                    };
                    let saturated =
                        bump_slot(&mut state.usage.provider_poll_failures_by_reason, reason);
                    state.usage.saturated |= saturated;
                    reconcile_abandoned_queues(&mut state.usage);
                    drop(state);
                    let _ = ack.send(SemanticSessionActorControlCompletionV1::OutcomeUnknown);
                    return true;
                }
            }
        }
        Control::DriveExpiry { ack } => {
            {
                let mut state = lock_shared(shared);
                bump_usage(&mut state.usage, Counter::ExpiryDrives);
            }
            match port.drive_expiry() {
                Ok(SemanticBrokerExpiryDriveV1::NoDue) => {
                    let mut state = lock_shared(shared);
                    bump_usage(&mut state.usage, Counter::ExpiryNoDue);
                    drop(state);
                    let _ = ack.send(SemanticSessionActorControlCompletionV1::Applied);
                }
                Ok(SemanticBrokerExpiryDriveV1::Due(report)) => {
                    let mut state = lock_shared(shared);
                    bump_usage(&mut state.usage, Counter::ExpiryEvents);
                    drop(state);
                    let (completion, fatal) = resolve_control(
                        broker.event(SemanticBrokerEventV1::ExpiryDriveCompleted { report }),
                        broker,
                        port,
                        shared,
                    );
                    let _ = ack.send(completion);
                    if fatal {
                        return true;
                    }
                }
                Err(failure) => {
                    fail_expiry(failure, shared);
                    let _ = ack.send(SemanticSessionActorControlCompletionV1::OutcomeUnknown);
                    return true;
                }
            }
        }
        Control::DriveRetirement { ack } => {
            let mut state = lock_shared(shared);
            bump_usage(&mut state.usage, Counter::RetirementDrives);
            drop(state);
            let (completion, fatal) = resolve_control(
                broker.event(SemanticBrokerEventV1::DriveRetirement),
                broker,
                port,
                shared,
            );
            let _ = ack.send(completion);
            if fatal {
                return true;
            }
        }
        Control::Stop(reply) => {
            let queued = lock_shared(shared).usage.queued_commands;
            if queued == 0 && broker_is_quiescent(broker.usage()) && port.obligations().is_empty() {
                {
                    let mut state = lock_shared(shared);
                    state.phase = SemanticSessionActorPhaseV1::Stopped;
                    bump_usage(&mut state.usage, Counter::CleanStops);
                }
                let _ = reply.send(Ok(()));
                return true;
            }
            let mut state = lock_shared(shared);
            state.phase = SemanticSessionActorPhaseV1::Draining;
            bump_usage(&mut state.usage, Counter::StopRefusals);
            let _ = reply.send(Err(SemanticSessionActorControlRefusalV1::NotQuiescent));
        }
    }
    false
}

fn finish_command<P: SemanticBrokerEffectPortV1>(
    mut decision: SemanticBrokerDecisionV1,
    reply: SyncSender<SemanticExchangeDispositionV1<SemanticSessionReplyV1>>,
    broker: &mut SemanticSessionBrokerV1,
    port: &mut P,
    shared: &Arc<Mutex<Shared>>,
) -> bool {
    if let SemanticBrokerDecisionV1::Effect(effect) = decision {
        let effect_kind = record_effect(&effect, shared);
        match complete_effect(port, effect) {
            Ok(event) => decision = broker.event(event),
            Err(failure) => {
                fail_effect(failure, effect_kind, shared);
                if reply
                    .send(SemanticExchangeDispositionV1::OutcomeUnknown)
                    .is_err()
                {
                    bump_usage(&mut lock_shared(shared).usage, Counter::RepliesDropped);
                }
                return true;
            }
        }
    }
    let fatal = matches!(decision, SemanticBrokerDecisionV1::Diverged(_));
    let disposition = match decision {
        SemanticBrokerDecisionV1::Reply(reply) => {
            let mut state = lock_shared(shared);
            bump_usage(&mut state.usage, Counter::Replies);
            SemanticExchangeDispositionV1::Reply(reply)
        }
        SemanticBrokerDecisionV1::Quarantined => {
            let mut state = lock_shared(shared);
            bump_usage(&mut state.usage, Counter::Quarantined);
            bump_usage(&mut state.usage, Counter::OutcomeUnknown);
            SemanticExchangeDispositionV1::OutcomeUnknown
        }
        SemanticBrokerDecisionV1::Diverged(_) => {
            let mut state = lock_shared(shared);
            state.phase = SemanticSessionActorPhaseV1::Failed;
            bump_usage(&mut state.usage, Counter::BrokerDivergences);
            bump_usage(&mut state.usage, Counter::OutcomeUnknown);
            state.usage.in_flight_commands = state.usage.in_flight_commands.saturating_sub(1);
            reconcile_abandoned_queues(&mut state.usage);
            SemanticExchangeDispositionV1::OutcomeUnknown
        }
        SemanticBrokerDecisionV1::AwaitingExisting
        | SemanticBrokerDecisionV1::EventAccepted
        | SemanticBrokerDecisionV1::Effect(_) => {
            let mut state = lock_shared(shared);
            bump_usage(&mut state.usage, Counter::OutcomeUnknown);
            SemanticExchangeDispositionV1::OutcomeUnknown
        }
    };
    if reply.send(disposition).is_err() {
        let mut state = lock_shared(shared);
        bump_usage(&mut state.usage, Counter::RepliesDropped);
    }
    if !fatal {
        let mut state = lock_shared(shared);
        state.usage.in_flight_commands = state.usage.in_flight_commands.saturating_sub(1);
    }
    fatal
}

fn resolve_control<P: SemanticBrokerEffectPortV1>(
    decision: SemanticBrokerDecisionV1,
    broker: &mut SemanticSessionBrokerV1,
    port: &mut P,
    shared: &Arc<Mutex<Shared>>,
) -> (SemanticSessionActorControlCompletionV1, bool) {
    let decision = if let SemanticBrokerDecisionV1::Effect(effect) = decision {
        let effect_kind = record_effect(&effect, shared);
        match complete_effect(port, effect) {
            Ok(event) => broker.event(event),
            Err(failure) => {
                fail_effect(failure, effect_kind, shared);
                return (
                    SemanticSessionActorControlCompletionV1::OutcomeUnknown,
                    true,
                );
            }
        }
    } else {
        decision
    };
    match decision {
        SemanticBrokerDecisionV1::Diverged(_) => {
            let mut state = lock_shared(shared);
            state.phase = SemanticSessionActorPhaseV1::Failed;
            bump_usage(&mut state.usage, Counter::BrokerDivergences);
            bump_usage(&mut state.usage, Counter::OutcomeUnknown);
            reconcile_abandoned_queues(&mut state.usage);
            (
                SemanticSessionActorControlCompletionV1::OutcomeUnknown,
                true,
            )
        }
        SemanticBrokerDecisionV1::Quarantined => {
            let mut state = lock_shared(shared);
            bump_usage(&mut state.usage, Counter::Quarantined);
            (SemanticSessionActorControlCompletionV1::Quarantined, false)
        }
        SemanticBrokerDecisionV1::AwaitingExisting => (
            SemanticSessionActorControlCompletionV1::OutcomeUnknown,
            false,
        ),
        SemanticBrokerDecisionV1::Reply(_)
        | SemanticBrokerDecisionV1::EventAccepted
        | SemanticBrokerDecisionV1::Effect(_) => {
            (SemanticSessionActorControlCompletionV1::Applied, false)
        }
    }
}

fn effect_kind(effect: &SemanticBrokerEffectV1) -> usize {
    match effect {
        SemanticBrokerEffectV1::Open { .. } => 0,
        SemanticBrokerEffectV1::Change { .. } => 1,
        SemanticBrokerEffectV1::Close { .. } => 2,
        SemanticBrokerEffectV1::Submit { .. } => 3,
        SemanticBrokerEffectV1::Cancel { .. } => 4,
    }
}

fn record_effect(effect: &SemanticBrokerEffectV1, shared: &Arc<Mutex<Shared>>) -> usize {
    let index = effect_kind(effect);
    let mut state = lock_shared(shared);
    bump_usage(&mut state.usage, Counter::Effects);
    let mut value = state.usage.effects_by_kind[index];
    let prior = value;
    value = value.saturating_add(1);
    state.usage.saturated |= value == u64::MAX && prior == u64::MAX;
    state.usage.effects_by_kind[index] = value;
    index
}

fn fail_effect(
    failure: SemanticBrokerEffectFailureV1,
    effect_kind: usize,
    shared: &Arc<Mutex<Shared>>,
) {
    let reason = match failure {
        SemanticBrokerEffectFailureV1::Poisoned => 0,
        SemanticBrokerEffectFailureV1::RuntimeUnavailable => 1,
        SemanticBrokerEffectFailureV1::DeliveryRefused => 2,
    };
    let mut state = lock_shared(shared);
    state.phase = SemanticSessionActorPhaseV1::Failed;
    bump_usage(&mut state.usage, Counter::EffectFailures);
    let kind_saturated = bump_slot(&mut state.usage.effect_failures_by_kind, effect_kind);
    let reason_saturated = bump_slot(&mut state.usage.effect_failures_by_reason, reason);
    state.usage.saturated |= kind_saturated || reason_saturated;
    reconcile_abandoned_queues(&mut state.usage);
}

fn fail_expiry(failure: SemanticBrokerEffectFailureV1, shared: &Arc<Mutex<Shared>>) {
    let reason = match failure {
        SemanticBrokerEffectFailureV1::Poisoned => 0,
        SemanticBrokerEffectFailureV1::RuntimeUnavailable => 1,
        SemanticBrokerEffectFailureV1::DeliveryRefused => 2,
    };
    let mut state = lock_shared(shared);
    state.phase = SemanticSessionActorPhaseV1::Failed;
    bump_usage(&mut state.usage, Counter::ExpiryFailures);
    bump_usage(&mut state.usage, Counter::OutcomeUnknown);
    let saturated = bump_slot(&mut state.usage.expiry_failures_by_reason, reason);
    state.usage.saturated |= saturated;
    reconcile_abandoned_queues(&mut state.usage);
}

fn complete_effect<P: SemanticBrokerEffectPortV1>(
    port: &mut P,
    effect: SemanticBrokerEffectV1,
) -> Result<SemanticBrokerEventV1, SemanticBrokerEffectFailureV1> {
    match effect {
        SemanticBrokerEffectV1::Open {
            continuation,
            snapshot,
        } => Ok(SemanticBrokerEventV1::OpenCompleted {
            continuation,
            report: port.open(&snapshot)?,
        }),
        SemanticBrokerEffectV1::Change {
            continuation,
            snapshot,
        } => Ok(SemanticBrokerEventV1::ChangeCompleted {
            continuation,
            report: port.change(&snapshot)?,
        }),
        SemanticBrokerEffectV1::Close {
            continuation,
            target,
        } => Ok(SemanticBrokerEventV1::CloseCompleted {
            continuation,
            report: port.close(target)?,
        }),
        SemanticBrokerEffectV1::Submit {
            continuation,
            request,
        } => Ok(SemanticBrokerEventV1::SubmissionCompleted {
            continuation,
            report: port.submit(&request)?,
        }),
        SemanticBrokerEffectV1::Cancel {
            continuation,
            request,
        } => Ok(SemanticBrokerEventV1::CancellationCompleted {
            continuation,
            report: port.cancel(request)?,
        }),
    }
}

fn broker_is_quiescent(usage: SemanticBrokerUsageV1) -> bool {
    usage.current_memberships == 0
        && usage.retiring_memberships == 0
        && usage.pending_effects == 0
        && usage.authoritative_documents == 0
        && usage.cleanup_documents == 0
}

fn release_command(shared: &Arc<Mutex<Shared>>) {
    let mut shared = lock_shared(shared);
    shared.usage.queued_commands = shared.usage.queued_commands.saturating_sub(1);
}

fn release_control(shared: &Arc<Mutex<Shared>>) {
    let mut shared = lock_shared(shared);
    shared.usage.queued_controls = shared.usage.queued_controls.saturating_sub(1);
}

fn reconcile_abandoned_queues(usage: &mut SemanticSessionActorUsageV1) {
    let abandoned_commands = usage
        .queued_commands
        .saturating_add(usage.in_flight_commands);
    let abandoned = u64::try_from(abandoned_commands).unwrap_or(u64::MAX);
    let prior = usage.outcome_unknown;
    usage.outcome_unknown = prior.saturating_add(abandoned);
    usage.saturated |= prior.checked_add(abandoned).is_none();
    usage.queued_commands = 0;
    usage.in_flight_commands = 0;
    usage.queued_controls = 0;
}

fn lock_shared(shared: &Arc<Mutex<Shared>>) -> std::sync::MutexGuard<'_, Shared> {
    shared
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Clone, Copy)]
enum Counter {
    CommandsAdmitted,
    CommandsFull,
    CommandsDraining,
    CommandsClosed,
    ControlsAdmitted,
    ControlsFull,
    Replies,
    RepliesDropped,
    Effects,
    EffectFailures,
    ProviderDrives,
    ProviderPollFailures,
    ExpiryDrives,
    ExpiryNoDue,
    ExpiryEvents,
    ExpiryFailures,
    BrokerDivergences,
    ProviderEvents,
    PolicyEvents,
    RetirementDrives,
    OutcomeUnknown,
    Quarantined,
    WorkerPanics,
    DrainRequests,
    StopTickets,
    StopTicketPolls,
    StopTicketPending,
    StopAbandonments,
    StopOutcomeUnknown,
    StopRefusals,
    CleanStops,
}

fn bump_usage(usage: &mut SemanticSessionActorUsageV1, counter: Counter) {
    let value = match counter {
        Counter::CommandsAdmitted => &mut usage.commands_admitted,
        Counter::CommandsFull => &mut usage.commands_full,
        Counter::CommandsDraining => &mut usage.commands_draining,
        Counter::CommandsClosed => &mut usage.commands_closed,
        Counter::ControlsAdmitted => &mut usage.controls_admitted,
        Counter::ControlsFull => &mut usage.controls_full,
        Counter::Replies => &mut usage.replies,
        Counter::RepliesDropped => &mut usage.replies_dropped,
        Counter::Effects => &mut usage.effects,
        Counter::EffectFailures => &mut usage.effect_failures,
        Counter::ProviderDrives => &mut usage.provider_drives,
        Counter::ProviderPollFailures => &mut usage.provider_poll_failures,
        Counter::ExpiryDrives => &mut usage.expiry_drives,
        Counter::ExpiryNoDue => &mut usage.expiry_no_due,
        Counter::ExpiryEvents => &mut usage.expiry_events,
        Counter::ExpiryFailures => &mut usage.expiry_failures,
        Counter::BrokerDivergences => &mut usage.broker_divergences,
        Counter::ProviderEvents => &mut usage.provider_events,
        Counter::PolicyEvents => &mut usage.policy_events,
        Counter::RetirementDrives => &mut usage.retirement_drives,
        Counter::OutcomeUnknown => &mut usage.outcome_unknown,
        Counter::Quarantined => &mut usage.quarantined,
        Counter::WorkerPanics => &mut usage.worker_panics,
        Counter::DrainRequests => &mut usage.drain_requests,
        Counter::StopTickets => &mut usage.stop_tickets,
        Counter::StopTicketPolls => &mut usage.stop_ticket_polls,
        Counter::StopTicketPending => &mut usage.stop_ticket_pending,
        Counter::StopAbandonments => &mut usage.stop_abandonments,
        Counter::StopOutcomeUnknown => &mut usage.stop_outcome_unknown,
        Counter::StopRefusals => &mut usage.stop_refusals,
        Counter::CleanStops => &mut usage.clean_stops,
    };
    let prior = *value;
    *value = value.saturating_add(1);
    usage.saturated |= *value == u64::MAX && prior == u64::MAX;
}

fn bump_slot(values: &mut [u64], index: usize) -> bool {
    let prior = values[index];
    values[index] = prior.saturating_add(1);
    values[index] == u64::MAX && prior == u64::MAX
}

#[cfg(test)]
mod tests {
    use super::*;
    use vellum_host::semantic::{
        SemanticCapabilitySetV1, SemanticDocumentRefV1, SemanticIndexGenerationV1,
        SemanticInvalidationCauseV1, SemanticInvalidationReceiptV1, SemanticProviderProvenanceV1,
        SemanticProviderRefV1, SemanticQueryV1, SemanticSourceRevisionV1, SemanticTargetV1,
        SemanticTextV1,
    };
    use vellum_semantic_session_broker::{
        SemanticBrokerAdmissionOutcomeV1, SemanticBrokerCancellationOutcomeV1,
        SemanticBrokerProviderBindingV1, SemanticBrokerProviderTransitionV1,
        SemanticBrokerSourceAdmissionOutcomeV1, SemanticBrokerSubmissionOutcomeV1,
    };

    fn open_accepted() -> SemanticBrokerOpenReportV1 {
        SemanticBrokerOpenReportV1 {
            provider_before: active_provider_binding(),
            pre_provider: None,
            outcome: SemanticBrokerAdmissionOutcomeV1::Accepted,
            post_provider: None,
        }
    }

    fn source_accepted(cause: SemanticInvalidationCauseV1) -> SemanticBrokerSourceReportV1 {
        SemanticBrokerSourceReportV1 {
            provider_before: active_provider_binding(),
            pre_provider: None,
            outcome: SemanticBrokerSourceAdmissionOutcomeV1::Accepted(
                SemanticInvalidationReceiptV1::try_new(cause, vec![], 1).unwrap(),
            ),
            post_provider: None,
        }
    }

    fn submission_accepted() -> SemanticBrokerSubmissionReportV1 {
        SemanticBrokerSubmissionReportV1 {
            provider_before: active_provider_binding(),
            pre_provider: None,
            outcome: SemanticBrokerSubmissionOutcomeV1::Accepted,
            post_provider: None,
        }
    }

    fn cancellation_accepted() -> SemanticBrokerCancellationReportV1 {
        SemanticBrokerCancellationReportV1 {
            provider_before: active_provider_binding(),
            pre_provider: None,
            outcome: SemanticBrokerCancellationOutcomeV1::Accepted,
            post_provider: None,
        }
    }

    fn active_provider_binding() -> SemanticBrokerProviderBindingV1 {
        SemanticBrokerProviderBindingV1 {
            epoch: 1,
            observation: SemanticProviderObservationV1::new(
                SemanticProviderReadinessProjectionV1::Ready {
                    generation: SemanticIndexGenerationV1::new(1).unwrap(),
                },
                SemanticCapabilitySetV1::top(),
            ),
            provenance: Some(provider_provenance()),
        }
    }

    fn unavailable_provider_binding() -> SemanticBrokerProviderBindingV1 {
        SemanticBrokerProviderBindingV1 {
            epoch: 0,
            observation: SemanticProviderObservationV1::new(
                SemanticProviderReadinessProjectionV1::Unavailable,
                SemanticCapabilitySetV1::bottom(),
            ),
            provenance: None,
        }
    }

    fn no_event_drive() -> SemanticBrokerProviderDriveReportV1 {
        SemanticBrokerProviderDriveReportV1 {
            provider_before: unavailable_provider_binding(),
            pre_provider: None,
            disposition: SemanticBrokerProviderDriveDispositionV1::NoEvent,
            post_provider: None,
        }
    }

    fn provider_provenance() -> SemanticProviderProvenanceV1 {
        SemanticProviderProvenanceV1::try_new(
            SemanticProviderRefV1::new([40; 16]),
            "qualification".into(),
            "1".into(),
        )
        .unwrap()
    }
    use vellum_semantic_protocol::{
        SemanticExchangeRefV1, SemanticProviderObservationV1,
        SemanticProviderReadinessProjectionV1, SemanticSessionIncarnationV1,
        SemanticSessionLimitsV1, SemanticSessionOperationRefV1, SemanticSessionOutcomeV1,
        SemanticSessionRolesV1, SemanticSessionSnapshotV1, SemanticSynchronizationEpochV1,
        SemanticSynchronizationManifestV1,
    };

    struct AcceptingPort;

    fn no_effect_obligations() -> SemanticBrokerEffectObligationsV1 {
        SemanticBrokerEffectObligationsV1 {
            request_deadlines: 0,
            due_deadlines: 0,
            poisoned: false,
            consistent: true,
        }
    }

    impl SemanticBrokerEffectPortV1 for AcceptingPort {
        fn obligations(&self) -> SemanticBrokerEffectObligationsV1 {
            no_effect_obligations()
        }
        fn open(
            &mut self,
            _snapshot: &SemanticSessionSnapshotV1,
        ) -> Result<SemanticBrokerOpenReportV1, SemanticBrokerEffectFailureV1> {
            Ok(open_accepted())
        }

        fn change(
            &mut self,
            _snapshot: &SemanticSessionSnapshotV1,
        ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
            Ok(source_accepted(SemanticInvalidationCauseV1::SourceChanged))
        }

        fn close(
            &mut self,
            _target: SemanticTargetV1,
        ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
            Ok(source_accepted(SemanticInvalidationCauseV1::SourceClosed))
        }

        fn submit(
            &mut self,
            _request: &SemanticRequestV1,
        ) -> Result<SemanticBrokerSubmissionReportV1, SemanticBrokerEffectFailureV1> {
            Ok(submission_accepted())
        }

        fn cancel(
            &mut self,
            _request: SemanticRequestRefV1,
        ) -> Result<SemanticBrokerCancellationReportV1, SemanticBrokerEffectFailureV1> {
            Ok(cancellation_accepted())
        }

        fn drive_provider(
            &mut self,
        ) -> Result<SemanticBrokerProviderDriveV1, SemanticBrokerEffectFailureV1> {
            Ok(SemanticBrokerProviderDriveV1::Completed(Box::new(
                no_event_drive(),
            )))
        }
        fn drive_expiry(
            &mut self,
        ) -> Result<SemanticBrokerExpiryDriveV1, SemanticBrokerEffectFailureV1> {
            Ok(SemanticBrokerExpiryDriveV1::NoDue)
        }
    }

    struct ObligatedPort {
        inner: AcceptingPort,
        obligations: SemanticBrokerEffectObligationsV1,
    }

    impl SemanticBrokerEffectPortV1 for ObligatedPort {
        fn obligations(&self) -> SemanticBrokerEffectObligationsV1 {
            self.obligations
        }

        fn open(
            &mut self,
            snapshot: &SemanticSessionSnapshotV1,
        ) -> Result<SemanticBrokerOpenReportV1, SemanticBrokerEffectFailureV1> {
            self.inner.open(snapshot)
        }

        fn change(
            &mut self,
            snapshot: &SemanticSessionSnapshotV1,
        ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
            self.inner.change(snapshot)
        }

        fn close(
            &mut self,
            target: SemanticTargetV1,
        ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
            self.inner.close(target)
        }

        fn submit(
            &mut self,
            request: &SemanticRequestV1,
        ) -> Result<SemanticBrokerSubmissionReportV1, SemanticBrokerEffectFailureV1> {
            self.inner.submit(request)
        }

        fn cancel(
            &mut self,
            request: SemanticRequestRefV1,
        ) -> Result<SemanticBrokerCancellationReportV1, SemanticBrokerEffectFailureV1> {
            self.inner.cancel(request)
        }

        fn drive_provider(
            &mut self,
        ) -> Result<SemanticBrokerProviderDriveV1, SemanticBrokerEffectFailureV1> {
            self.inner.drive_provider()
        }

        fn drive_expiry(
            &mut self,
        ) -> Result<SemanticBrokerExpiryDriveV1, SemanticBrokerEffectFailureV1> {
            self.inner.drive_expiry()
        }
    }

    struct BlockingPollPort {
        entered: SyncSender<()>,
        release: Receiver<()>,
        tag: u64,
        panic_after_release: bool,
    }

    impl SemanticBrokerEffectPortV1 for BlockingPollPort {
        fn obligations(&self) -> SemanticBrokerEffectObligationsV1 {
            no_effect_obligations()
        }
        fn open(
            &mut self,
            _snapshot: &SemanticSessionSnapshotV1,
        ) -> Result<SemanticBrokerOpenReportV1, SemanticBrokerEffectFailureV1> {
            Ok(open_accepted())
        }
        fn change(
            &mut self,
            _snapshot: &SemanticSessionSnapshotV1,
        ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
            Ok(source_accepted(SemanticInvalidationCauseV1::SourceChanged))
        }
        fn close(
            &mut self,
            _target: SemanticTargetV1,
        ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
            Ok(source_accepted(SemanticInvalidationCauseV1::SourceClosed))
        }
        fn submit(
            &mut self,
            _request: &SemanticRequestV1,
        ) -> Result<SemanticBrokerSubmissionReportV1, SemanticBrokerEffectFailureV1> {
            Ok(submission_accepted())
        }
        fn cancel(
            &mut self,
            _request: SemanticRequestRefV1,
        ) -> Result<SemanticBrokerCancellationReportV1, SemanticBrokerEffectFailureV1> {
            Ok(cancellation_accepted())
        }
        fn drive_provider(
            &mut self,
        ) -> Result<SemanticBrokerProviderDriveV1, SemanticBrokerEffectFailureV1> {
            let _ = self.entered.send(());
            let _ = self.release.recv();
            assert!(!self.panic_after_release, "qualified blocking poll panic");
            Ok(SemanticBrokerProviderDriveV1::Completed(Box::new(
                no_event_drive(),
            )))
        }
        fn drive_expiry(
            &mut self,
        ) -> Result<SemanticBrokerExpiryDriveV1, SemanticBrokerEffectFailureV1> {
            Ok(SemanticBrokerExpiryDriveV1::NoDue)
        }
    }

    struct FailingPollPort(u64);

    impl SemanticBrokerEffectPortV1 for FailingPollPort {
        fn obligations(&self) -> SemanticBrokerEffectObligationsV1 {
            no_effect_obligations()
        }
        fn open(
            &mut self,
            _snapshot: &SemanticSessionSnapshotV1,
        ) -> Result<SemanticBrokerOpenReportV1, SemanticBrokerEffectFailureV1> {
            Ok(open_accepted())
        }
        fn change(
            &mut self,
            _snapshot: &SemanticSessionSnapshotV1,
        ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
            Ok(source_accepted(SemanticInvalidationCauseV1::SourceChanged))
        }
        fn close(
            &mut self,
            _target: SemanticTargetV1,
        ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
            Ok(source_accepted(SemanticInvalidationCauseV1::SourceClosed))
        }
        fn submit(
            &mut self,
            _request: &SemanticRequestV1,
        ) -> Result<SemanticBrokerSubmissionReportV1, SemanticBrokerEffectFailureV1> {
            Ok(submission_accepted())
        }
        fn cancel(
            &mut self,
            _request: SemanticRequestRefV1,
        ) -> Result<SemanticBrokerCancellationReportV1, SemanticBrokerEffectFailureV1> {
            Ok(cancellation_accepted())
        }
        fn drive_provider(
            &mut self,
        ) -> Result<SemanticBrokerProviderDriveV1, SemanticBrokerEffectFailureV1> {
            Ok(SemanticBrokerProviderDriveV1::Failed(Box::new(
                SemanticBrokerProviderDriveFailureReportV1 {
                    provider_before: unavailable_provider_binding(),
                    pre_provider: Some(SemanticBrokerProviderTransitionV1 {
                        epoch: 1,
                        observed_observation: SemanticProviderObservationV1::new(
                            SemanticProviderReadinessProjectionV1::Indexing,
                            SemanticCapabilitySetV1::bottom(),
                        ),
                        observed_provenance: None,
                        settled_observation: SemanticProviderObservationV1::new(
                            SemanticProviderReadinessProjectionV1::Indexing,
                            SemanticCapabilitySetV1::bottom(),
                        ),
                        settled_provenance: None,
                        invalidations: SemanticInvalidationReceiptV1::try_new(
                            SemanticInvalidationCauseV1::ProviderStateChanged,
                            vec![],
                            1,
                        )
                        .unwrap(),
                    }),
                    failure: SemanticBrokerProviderDriveFailureClassV1::RuntimeUnavailable,
                },
            )))
        }
        fn drive_expiry(
            &mut self,
        ) -> Result<SemanticBrokerExpiryDriveV1, SemanticBrokerEffectFailureV1> {
            Ok(SemanticBrokerExpiryDriveV1::NoDue)
        }
    }

    struct FailingExpiryPort(u64);

    impl SemanticBrokerEffectPortV1 for FailingExpiryPort {
        fn obligations(&self) -> SemanticBrokerEffectObligationsV1 {
            no_effect_obligations()
        }
        fn open(
            &mut self,
            _snapshot: &SemanticSessionSnapshotV1,
        ) -> Result<SemanticBrokerOpenReportV1, SemanticBrokerEffectFailureV1> {
            Ok(open_accepted())
        }
        fn change(
            &mut self,
            _snapshot: &SemanticSessionSnapshotV1,
        ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
            Ok(source_accepted(SemanticInvalidationCauseV1::SourceChanged))
        }
        fn close(
            &mut self,
            _target: SemanticTargetV1,
        ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
            Ok(source_accepted(SemanticInvalidationCauseV1::SourceClosed))
        }
        fn submit(
            &mut self,
            _request: &SemanticRequestV1,
        ) -> Result<SemanticBrokerSubmissionReportV1, SemanticBrokerEffectFailureV1> {
            Ok(submission_accepted())
        }
        fn cancel(
            &mut self,
            _request: SemanticRequestRefV1,
        ) -> Result<SemanticBrokerCancellationReportV1, SemanticBrokerEffectFailureV1> {
            Ok(cancellation_accepted())
        }
        fn drive_provider(
            &mut self,
        ) -> Result<SemanticBrokerProviderDriveV1, SemanticBrokerEffectFailureV1> {
            Ok(SemanticBrokerProviderDriveV1::Completed(Box::new(
                no_event_drive(),
            )))
        }
        fn drive_expiry(
            &mut self,
        ) -> Result<SemanticBrokerExpiryDriveV1, SemanticBrokerEffectFailureV1> {
            Err(SemanticBrokerEffectFailureV1::DeliveryRefused)
        }
    }

    struct FailingEffectPort {
        kind: usize,
        tag: u64,
    }

    impl FailingEffectPort {
        fn outcome<T>(&self, kind: usize, value: T) -> Result<T, SemanticBrokerEffectFailureV1> {
            if self.kind == kind {
                Err(match kind % 3 {
                    0 => SemanticBrokerEffectFailureV1::Poisoned,
                    1 => SemanticBrokerEffectFailureV1::RuntimeUnavailable,
                    _ => SemanticBrokerEffectFailureV1::DeliveryRefused,
                })
            } else {
                Ok(value)
            }
        }
    }

    impl SemanticBrokerEffectPortV1 for FailingEffectPort {
        fn obligations(&self) -> SemanticBrokerEffectObligationsV1 {
            no_effect_obligations()
        }
        fn open(
            &mut self,
            _snapshot: &SemanticSessionSnapshotV1,
        ) -> Result<SemanticBrokerOpenReportV1, SemanticBrokerEffectFailureV1> {
            self.outcome(0, open_accepted())
        }

        fn change(
            &mut self,
            _snapshot: &SemanticSessionSnapshotV1,
        ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
            self.outcome(
                1,
                source_accepted(SemanticInvalidationCauseV1::SourceChanged),
            )
        }

        fn close(
            &mut self,
            _target: SemanticTargetV1,
        ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
            self.outcome(
                2,
                source_accepted(SemanticInvalidationCauseV1::SourceClosed),
            )
        }

        fn submit(
            &mut self,
            _request: &SemanticRequestV1,
        ) -> Result<SemanticBrokerSubmissionReportV1, SemanticBrokerEffectFailureV1> {
            self.outcome(3, submission_accepted())
        }

        fn cancel(
            &mut self,
            _request: SemanticRequestRefV1,
        ) -> Result<SemanticBrokerCancellationReportV1, SemanticBrokerEffectFailureV1> {
            self.outcome(4, cancellation_accepted())
        }

        fn drive_provider(
            &mut self,
        ) -> Result<SemanticBrokerProviderDriveV1, SemanticBrokerEffectFailureV1> {
            Ok(SemanticBrokerProviderDriveV1::Completed(Box::new(
                no_event_drive(),
            )))
        }
        fn drive_expiry(
            &mut self,
        ) -> Result<SemanticBrokerExpiryDriveV1, SemanticBrokerEffectFailureV1> {
            Ok(SemanticBrokerExpiryDriveV1::NoDue)
        }
    }

    struct MismatchedDrivePort;

    impl SemanticBrokerEffectPortV1 for MismatchedDrivePort {
        fn obligations(&self) -> SemanticBrokerEffectObligationsV1 {
            no_effect_obligations()
        }
        fn open(
            &mut self,
            _snapshot: &SemanticSessionSnapshotV1,
        ) -> Result<SemanticBrokerOpenReportV1, SemanticBrokerEffectFailureV1> {
            Ok(open_accepted())
        }
        fn change(
            &mut self,
            _snapshot: &SemanticSessionSnapshotV1,
        ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
            Ok(source_accepted(SemanticInvalidationCauseV1::SourceChanged))
        }
        fn close(
            &mut self,
            _target: SemanticTargetV1,
        ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
            Ok(source_accepted(SemanticInvalidationCauseV1::SourceClosed))
        }
        fn submit(
            &mut self,
            _request: &SemanticRequestV1,
        ) -> Result<SemanticBrokerSubmissionReportV1, SemanticBrokerEffectFailureV1> {
            Ok(submission_accepted())
        }
        fn cancel(
            &mut self,
            _request: SemanticRequestRefV1,
        ) -> Result<SemanticBrokerCancellationReportV1, SemanticBrokerEffectFailureV1> {
            Ok(cancellation_accepted())
        }
        fn drive_provider(
            &mut self,
        ) -> Result<SemanticBrokerProviderDriveV1, SemanticBrokerEffectFailureV1> {
            Ok(SemanticBrokerProviderDriveV1::Completed(Box::new(
                SemanticBrokerProviderDriveReportV1 {
                    provider_before: active_provider_binding(),
                    pre_provider: None,
                    disposition: SemanticBrokerProviderDriveDispositionV1::NoEvent,
                    post_provider: None,
                },
            )))
        }
        fn drive_expiry(
            &mut self,
        ) -> Result<SemanticBrokerExpiryDriveV1, SemanticBrokerEffectFailureV1> {
            Ok(SemanticBrokerExpiryDriveV1::NoDue)
        }
    }

    struct PanickingOpenPort(u64);

    impl SemanticBrokerEffectPortV1 for PanickingOpenPort {
        fn obligations(&self) -> SemanticBrokerEffectObligationsV1 {
            no_effect_obligations()
        }
        fn open(
            &mut self,
            _snapshot: &SemanticSessionSnapshotV1,
        ) -> Result<SemanticBrokerOpenReportV1, SemanticBrokerEffectFailureV1> {
            panic!("qualified provider panic")
        }
        fn change(
            &mut self,
            _snapshot: &SemanticSessionSnapshotV1,
        ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
            Ok(source_accepted(SemanticInvalidationCauseV1::SourceChanged))
        }
        fn close(
            &mut self,
            _target: SemanticTargetV1,
        ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
            Ok(source_accepted(SemanticInvalidationCauseV1::SourceClosed))
        }
        fn submit(
            &mut self,
            _request: &SemanticRequestV1,
        ) -> Result<SemanticBrokerSubmissionReportV1, SemanticBrokerEffectFailureV1> {
            Ok(submission_accepted())
        }
        fn cancel(
            &mut self,
            _request: SemanticRequestRefV1,
        ) -> Result<SemanticBrokerCancellationReportV1, SemanticBrokerEffectFailureV1> {
            Ok(cancellation_accepted())
        }

        fn drive_provider(
            &mut self,
        ) -> Result<SemanticBrokerProviderDriveV1, SemanticBrokerEffectFailureV1> {
            Ok(SemanticBrokerProviderDriveV1::Completed(Box::new(
                no_event_drive(),
            )))
        }
        fn drive_expiry(
            &mut self,
        ) -> Result<SemanticBrokerExpiryDriveV1, SemanticBrokerEffectFailureV1> {
            Ok(SemanticBrokerExpiryDriveV1::NoDue)
        }
    }

    struct MismatchedOpenPort;

    impl SemanticBrokerEffectPortV1 for MismatchedOpenPort {
        fn obligations(&self) -> SemanticBrokerEffectObligationsV1 {
            no_effect_obligations()
        }
        fn open(
            &mut self,
            _snapshot: &SemanticSessionSnapshotV1,
        ) -> Result<SemanticBrokerOpenReportV1, SemanticBrokerEffectFailureV1> {
            let mut report = open_accepted();
            report.provider_before = SemanticBrokerProviderBindingV1 {
                epoch: 0,
                observation: SemanticProviderObservationV1::new(
                    SemanticProviderReadinessProjectionV1::Unavailable,
                    SemanticCapabilitySetV1::bottom(),
                ),
                provenance: None,
            };
            Ok(report)
        }

        fn change(
            &mut self,
            _snapshot: &SemanticSessionSnapshotV1,
        ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
            Ok(source_accepted(SemanticInvalidationCauseV1::SourceChanged))
        }

        fn close(
            &mut self,
            _target: SemanticTargetV1,
        ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
            Ok(source_accepted(SemanticInvalidationCauseV1::SourceClosed))
        }

        fn submit(
            &mut self,
            _request: &SemanticRequestV1,
        ) -> Result<SemanticBrokerSubmissionReportV1, SemanticBrokerEffectFailureV1> {
            Ok(submission_accepted())
        }

        fn cancel(
            &mut self,
            _request: SemanticRequestRefV1,
        ) -> Result<SemanticBrokerCancellationReportV1, SemanticBrokerEffectFailureV1> {
            Ok(cancellation_accepted())
        }

        fn drive_provider(
            &mut self,
        ) -> Result<SemanticBrokerProviderDriveV1, SemanticBrokerEffectFailureV1> {
            Ok(SemanticBrokerProviderDriveV1::Completed(Box::new(
                no_event_drive(),
            )))
        }
        fn drive_expiry(
            &mut self,
        ) -> Result<SemanticBrokerExpiryDriveV1, SemanticBrokerEffectFailureV1> {
            Ok(SemanticBrokerExpiryDriveV1::NoDue)
        }
    }

    fn actor() -> (
        SemanticSessionActorOwnerV1,
        SemanticSessionActorSenderV1,
        SemanticSessionActorRecoveryV1<AcceptingPort>,
    ) {
        let broker = SemanticSessionBrokerV1::try_new(
            SemanticSessionIncarnationV1::new([3; 16]).unwrap(),
            SemanticSessionLimitsV1::reference(),
        )
        .unwrap();
        spawn_semantic_session_actor_v1(
            broker,
            AcceptingPort,
            SemanticSessionActorLimitsV1::reference(),
        )
        .unwrap()
    }

    fn ready_broker() -> SemanticSessionBrokerV1 {
        let mut broker = SemanticSessionBrokerV1::try_new(
            SemanticSessionIncarnationV1::new([3; 16]).unwrap(),
            SemanticSessionLimitsV1::reference(),
        )
        .unwrap();
        let observation = active_provider_binding().observation;
        assert_eq!(
            broker.event(SemanticBrokerEventV1::ProviderDriveCompleted {
                report: Box::new(SemanticBrokerProviderDriveReportV1 {
                    provider_before: unavailable_provider_binding(),
                    pre_provider: Some(SemanticBrokerProviderTransitionV1 {
                        epoch: 1,
                        observed_observation: observation,
                        observed_provenance: Some(provider_provenance()),
                        settled_observation: observation,
                        settled_provenance: Some(provider_provenance()),
                        invalidations: SemanticInvalidationReceiptV1::try_new(
                            SemanticInvalidationCauseV1::ProviderStateChanged,
                            vec![],
                            1,
                        )
                        .unwrap(),
                    }),
                    disposition: SemanticBrokerProviderDriveDispositionV1::NoEvent,
                    post_provider: None,
                }),
            }),
            SemanticBrokerDecisionV1::EventAccepted
        );
        broker
    }

    fn ready_actor() -> (
        SemanticSessionActorOwnerV1,
        SemanticSessionActorSenderV1,
        SemanticSessionActorRecoveryV1<AcceptingPort>,
    ) {
        spawn_semantic_session_actor_v1(
            ready_broker(),
            AcceptingPort,
            SemanticSessionActorLimitsV1::reference(),
        )
        .unwrap()
    }

    fn recover<P>(
        mut recovery: SemanticSessionActorRecoveryV1<P>,
    ) -> SemanticSessionActorRecoveredV1<P> {
        loop {
            match recovery.try_recover() {
                Ok(recovered) => return recovered,
                Err(returned) => {
                    recovery = returned;
                    thread::yield_now();
                }
            }
        }
    }

    fn context() -> SemanticBrokerContextV1 {
        use vellum_host::authority::{AuthoritySessionRefV1, AuthoritySubjectRefV1};
        use vellum_semantic_protocol::SemanticSessionRolesV1;
        SemanticBrokerContextV1 {
            subject: AuthoritySubjectRefV1::new([1; 16]),
            session: AuthoritySessionRefV1::new([2; 16]),
            allowed_roles: SemanticSessionRolesV1::top(),
            policy_epoch: 1,
        }
    }

    fn describe(tag: u8) -> SemanticSessionCommandV1 {
        SemanticSessionCommandV1::Describe {
            exchange: SemanticExchangeRefV1::new([tag; 16]).unwrap(),
        }
    }

    fn reference<T>(tag: u8, make: impl FnOnce([u8; 16]) -> Option<T>) -> T {
        make([tag; 16]).unwrap()
    }

    fn exchange(
        sender: &SemanticSessionActorSenderV1,
        command: SemanticSessionCommandV1,
    ) -> SemanticSessionReplyV1 {
        let SemanticSessionActorAdmissionV1::Submitted(ticket) =
            sender.try_exchange(context(), command)
        else {
            panic!("command was not admitted")
        };
        let SemanticExchangeDispositionV1::Reply(reply) = ticket.receive() else {
            panic!("command became uncertain")
        };
        reply
    }

    fn broker_exchange(
        broker: &mut SemanticSessionBrokerV1,
        port: &mut AcceptingPort,
        command: SemanticSessionCommandV1,
    ) -> SemanticSessionReplyV1 {
        let decision = match broker.command(context(), command) {
            SemanticBrokerDecisionV1::Effect(effect) => {
                broker.event(complete_effect(port, effect).unwrap())
            }
            decision => decision,
        };
        let SemanticBrokerDecisionV1::Reply(reply) = decision else {
            panic!("broker preparation did not close with a reply")
        };
        reply
    }

    fn effect_failure_case(kind: usize) -> (SemanticSessionBrokerV1, SemanticSessionCommandV1) {
        let mut broker = ready_broker();
        let mut port = AcceptingPort;
        let joined = broker_exchange(
            &mut broker,
            &mut port,
            SemanticSessionCommandV1::JoinSession {
                exchange: reference(61, SemanticExchangeRefV1::new),
                operation: reference(62, SemanticSessionOperationRefV1::new),
                incarnation: reference(3, SemanticSessionIncarnationV1::new),
                requested_roles: SemanticSessionRolesV1::top(),
            },
        );
        let SemanticSessionOutcomeV1::Joined { membership, .. } = joined.outcome() else {
            panic!("preparation join was refused")
        };
        let membership = *membership;
        let limits = SemanticSessionLimitsV1::reference();
        let snapshot = SemanticSessionSnapshotV1::try_new(
            SemanticTargetV1 {
                document: SemanticDocumentRefV1::new([63; 16]),
                source_revision: SemanticSourceRevisionV1::new(1),
            },
            "rust".into(),
            "fn prepared() {}".into(),
            limits,
        )
        .unwrap();
        let epoch = reference(64, SemanticSynchronizationEpochV1::new);
        let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
            std::slice::from_ref(&snapshot),
            limits,
        )
        .unwrap();
        assert_eq!(
            broker_exchange(
                &mut broker,
                &mut port,
                SemanticSessionCommandV1::BeginSynchronization {
                    exchange: reference(65, SemanticExchangeRefV1::new),
                    operation: reference(66, SemanticSessionOperationRefV1::new),
                    incarnation: reference(3, SemanticSessionIncarnationV1::new),
                    membership,
                    epoch,
                    manifest,
                },
            )
            .outcome(),
            &SemanticSessionOutcomeV1::Accepted
        );
        let open = SemanticSessionCommandV1::SyncOpen {
            exchange: reference(67, SemanticExchangeRefV1::new),
            operation: reference(68, SemanticSessionOperationRefV1::new),
            incarnation: reference(3, SemanticSessionIncarnationV1::new),
            membership,
            epoch,
            snapshot: snapshot.clone(),
        };
        if kind == 0 {
            return (broker, open);
        }
        assert_eq!(
            broker_exchange(&mut broker, &mut port, open).outcome(),
            &SemanticSessionOutcomeV1::Accepted
        );
        assert_eq!(
            broker_exchange(
                &mut broker,
                &mut port,
                SemanticSessionCommandV1::CompleteSynchronization {
                    exchange: reference(69, SemanticExchangeRefV1::new),
                    operation: reference(70, SemanticSessionOperationRefV1::new),
                    incarnation: reference(3, SemanticSessionIncarnationV1::new),
                    membership,
                    epoch,
                    manifest,
                },
            )
            .outcome(),
            &SemanticSessionOutcomeV1::Accepted
        );
        let request = SemanticRequestV1 {
            request: SemanticRequestRefV1::new([71; 16]),
            target: snapshot.target(),
            expected_index: Some(SemanticIndexGenerationV1::new(1).unwrap()),
            language: SemanticTextV1::try_new("rust".into(), 16).unwrap(),
            query: SemanticQueryV1::Diagnostics,
        };
        let submit = SemanticSessionCommandV1::Submit {
            exchange: reference(72, SemanticExchangeRefV1::new),
            operation: reference(73, SemanticSessionOperationRefV1::new),
            incarnation: reference(3, SemanticSessionIncarnationV1::new),
            membership,
            request: request.clone(),
        };
        if kind == 3 {
            return (broker, submit);
        }
        if kind == 4 {
            assert_eq!(
                broker_exchange(&mut broker, &mut port, submit).outcome(),
                &SemanticSessionOutcomeV1::Accepted
            );
            return (
                broker,
                SemanticSessionCommandV1::Cancel {
                    exchange: reference(74, SemanticExchangeRefV1::new),
                    operation: reference(75, SemanticSessionOperationRefV1::new),
                    incarnation: reference(3, SemanticSessionIncarnationV1::new),
                    membership,
                    request: request.request,
                },
            );
        }
        let next_epoch = reference(76, SemanticSynchronizationEpochV1::new);
        let changed = SemanticSessionSnapshotV1::try_new(
            SemanticTargetV1 {
                document: snapshot.target().document,
                source_revision: SemanticSourceRevisionV1::new(2),
            },
            "rust".into(),
            "fn prepared_changed() {}".into(),
            limits,
        )
        .unwrap();
        let next_manifest = if kind == 1 {
            SemanticSynchronizationManifestV1::try_from_snapshots(
                std::slice::from_ref(&changed),
                limits,
            )
            .unwrap()
        } else {
            SemanticSynchronizationManifestV1::try_from_snapshots(&[], limits).unwrap()
        };
        assert_eq!(
            broker_exchange(
                &mut broker,
                &mut port,
                SemanticSessionCommandV1::BeginSynchronization {
                    exchange: reference(77, SemanticExchangeRefV1::new),
                    operation: reference(78, SemanticSessionOperationRefV1::new),
                    incarnation: reference(3, SemanticSessionIncarnationV1::new),
                    membership,
                    epoch: next_epoch,
                    manifest: next_manifest,
                },
            )
            .outcome(),
            &SemanticSessionOutcomeV1::Accepted
        );
        if kind == 1 {
            (
                broker,
                SemanticSessionCommandV1::SyncChange {
                    exchange: reference(79, SemanticExchangeRefV1::new),
                    operation: reference(80, SemanticSessionOperationRefV1::new),
                    incarnation: reference(3, SemanticSessionIncarnationV1::new),
                    membership,
                    epoch: next_epoch,
                    snapshot: changed,
                },
            )
        } else {
            (
                broker,
                SemanticSessionCommandV1::SyncClose {
                    exchange: reference(81, SemanticExchangeRefV1::new),
                    operation: reference(82, SemanticSessionOperationRefV1::new),
                    incarnation: reference(3, SemanticSessionIncarnationV1::new),
                    membership,
                    epoch: next_epoch,
                    target: snapshot.target(),
                },
            )
        }
    }

    fn policy_event(policy_epoch: u64) -> SemanticSessionActorExternalEventV1 {
        SemanticSessionActorExternalEventV1::PolicyLost {
            subject: AuthoritySubjectRefV1::new([91; 16]),
            session: AuthoritySessionRefV1::new([92; 16]),
            policy_epoch,
        }
    }

    #[test]
    fn describe_round_trips_as_an_exact_reply() {
        let (owner, sender, recovery) = actor();
        let SemanticSessionActorAdmissionV1::Submitted(ticket) =
            sender.try_exchange(context(), describe(9))
        else {
            panic!("describe was not admitted")
        };
        let SemanticExchangeDispositionV1::Reply(reply) = ticket.receive() else {
            panic!("describe became uncertain")
        };
        assert!(matches!(
            reply.outcome(),
            SemanticSessionOutcomeV1::Description(_)
        ));
        assert_eq!(sender.usage().commands_admitted, 1);
        assert!(owner.begin_drain());
        owner.try_stop().unwrap();
        assert_eq!(
            recover(recovery).termination(),
            SemanticSessionActorTerminationV1::Clean
        );
    }

    #[test]
    fn drain_is_an_immediate_not_sent_fence() {
        let (owner, sender, recovery) = actor();
        assert!(owner.begin_drain());
        let SemanticSessionActorAdmissionV1::NotSent { reason, command } =
            sender.try_exchange(context(), describe(8))
        else {
            panic!("draining actor admitted a command")
        };
        assert_eq!(reason, SemanticSessionActorNotSentReasonV1::Draining);
        assert_eq!(
            command.exchange(),
            SemanticExchangeRefV1::new([8; 16]).unwrap()
        );
        owner.try_stop().unwrap();
        assert_eq!(
            recover(recovery).termination(),
            SemanticSessionActorTerminationV1::Clean
        );
    }

    #[test]
    fn recovery_refuses_without_blocking_and_returns_exact_runtime() {
        let broker = SemanticSessionBrokerV1::try_new(
            reference(3, SemanticSessionIncarnationV1::new),
            SemanticSessionLimitsV1::reference(),
        )
        .unwrap();
        let (entered_tx, entered_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let (owner, _sender, recovery) = spawn_semantic_session_actor_v1(
            broker,
            BlockingPollPort {
                entered: entered_tx,
                release: release_rx,
                tag: 77,
                panic_after_release: false,
            },
            SemanticSessionActorLimitsV1::reference(),
        )
        .unwrap();
        let drive = owner.try_drive_provider().unwrap();
        entered_rx.recv().unwrap();
        assert_eq!(
            format!("{recovery:?}"),
            "SemanticSessionActorRecoveryV1(<opaque>)"
        );
        let recovery = recovery
            .try_recover()
            .expect_err("a running worker must return its recovery token");
        release_tx.send(()).unwrap();
        assert_eq!(
            drive.receive(),
            SemanticSessionActorControlCompletionV1::Applied
        );
        assert!(owner.begin_drain());
        owner.try_stop().unwrap();
        let recovered = recover(recovery);
        assert_eq!(
            recovered.termination(),
            SemanticSessionActorTerminationV1::Clean
        );
        assert_eq!(recovered.port().tag, 77);
        let debug = format!("{recovered:?}");
        assert!(!debug.contains("77"));
        assert!(debug.contains("broker: \"<opaque>\""));
        assert!(debug.contains("port: \"<opaque>\""));
    }

    #[test]
    fn exhausted_admission_credit_is_exactly_not_sent() {
        let broker = SemanticSessionBrokerV1::try_new(
            reference(3, SemanticSessionIncarnationV1::new),
            SemanticSessionLimitsV1::reference(),
        )
        .unwrap();
        let (entered_tx, entered_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let (owner, sender, recovery) = spawn_semantic_session_actor_v1(
            broker,
            BlockingPollPort {
                entered: entered_tx,
                release: release_rx,
                tag: 11,
                panic_after_release: false,
            },
            SemanticSessionActorLimitsV1 {
                max_commands: 1,
                max_controls: 1,
            },
        )
        .unwrap();
        let drive = owner.try_drive_provider().unwrap();
        entered_rx.recv().unwrap();
        let SemanticSessionActorAdmissionV1::Submitted(first) =
            sender.try_exchange(context(), describe(11))
        else {
            panic!("first queued command was not admitted")
        };
        let SemanticSessionActorAdmissionV1::NotSent { reason, command } =
            sender.try_exchange(context(), describe(12))
        else {
            panic!("exhausted mailbox credit admitted a command")
        };
        assert_eq!(reason, SemanticSessionActorNotSentReasonV1::Full);
        assert_eq!(
            command.exchange(),
            reference(12, SemanticExchangeRefV1::new)
        );
        release_tx.send(()).unwrap();
        assert_eq!(
            drive.receive(),
            SemanticSessionActorControlCompletionV1::Applied
        );
        assert!(matches!(
            first.receive(),
            SemanticExchangeDispositionV1::Reply(_)
        ));
        assert_eq!(sender.usage().commands_admitted, 1);
        assert!(owner.begin_drain());
        owner.try_stop().unwrap();
        assert_eq!(
            recover(recovery).termination(),
            SemanticSessionActorTerminationV1::Clean
        );
    }

    #[test]
    fn invalid_limits_refuse_before_thread_launch() {
        let broker = SemanticSessionBrokerV1::try_new(
            SemanticSessionIncarnationV1::new([3; 16]).unwrap(),
            SemanticSessionLimitsV1::reference(),
        )
        .unwrap();
        let result = spawn_semantic_session_actor_v1(
            broker,
            AcceptingPort,
            SemanticSessionActorLimitsV1 {
                max_commands: 0,
                max_controls: 1,
            },
        );
        let Err(failure) = result else {
            panic!("invalid limits unexpectedly launched an actor")
        };
        assert_eq!(
            failure.reason(),
            SemanticSessionActorBuildRefusalV1::InvalidLimits
        );
        let debug = format!("{failure:?}");
        assert!(debug.contains("broker: \"<opaque>\""));
        assert!(debug.contains("port: \"<opaque>\""));
        let (reason, limits, broker, _port) = failure.into_parts();
        assert_eq!(reason, SemanticSessionActorBuildRefusalV1::InvalidLimits);
        assert_eq!(limits.max_commands, 0);
        assert_eq!(broker.usage().pending_effects, 0);
    }

    #[test]
    fn thread_launch_failure_returns_exact_runtime_without_starting_it() {
        let broker = SemanticSessionBrokerV1::try_new(
            SemanticSessionIncarnationV1::new([3; 16]).unwrap(),
            SemanticSessionLimitsV1::reference(),
        )
        .unwrap();
        let result = spawn_semantic_session_actor_with_v1(
            broker,
            FailingPollPort(91),
            SemanticSessionActorLimitsV1::reference(),
            |_worker| Err(std::io::Error::other("injected launch refusal")),
        );
        let Err(failure) = result else {
            panic!("injected launcher unexpectedly launched an actor")
        };
        assert_eq!(
            failure.reason(),
            SemanticSessionActorBuildRefusalV1::ThreadUnavailable
        );
        let (reason, limits, broker, port) = failure.into_parts();
        assert_eq!(
            reason,
            SemanticSessionActorBuildRefusalV1::ThreadUnavailable
        );
        assert_eq!(limits, SemanticSessionActorLimitsV1::reference());
        assert_eq!(broker.usage().pending_effects, 0);
        assert_eq!(port.0, 91);
    }

    #[test]
    fn owner_loss_fences_surviving_command_senders_and_terminates() {
        let (owner, sender, recovery) = actor();
        drop(owner);
        for _ in 0..100_000 {
            if sender.phase() == SemanticSessionActorPhaseV1::Failed {
                break;
            }
            thread::yield_now();
        }
        assert_eq!(sender.phase(), SemanticSessionActorPhaseV1::Failed);
        let SemanticSessionActorAdmissionV1::NotSent { reason, command } =
            sender.try_exchange(context(), describe(14))
        else {
            panic!("ownerless actor admitted a command")
        };
        assert_eq!(reason, SemanticSessionActorNotSentReasonV1::Closed);
        assert_eq!(
            command.exchange(),
            reference(14, SemanticExchangeRefV1::new)
        );
        let recovered = recover(recovery);
        assert_eq!(
            recovered.termination(),
            SemanticSessionActorTerminationV1::Failed
        );
    }

    #[test]
    fn full_control_mailbox_returns_exact_event_ownership() {
        let broker = SemanticSessionBrokerV1::try_new(
            reference(3, SemanticSessionIncarnationV1::new),
            SemanticSessionLimitsV1::reference(),
        )
        .unwrap();
        let (entered_tx, entered_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let (owner, _sender, recovery) = spawn_semantic_session_actor_v1(
            broker,
            BlockingPollPort {
                entered: entered_tx,
                release: release_rx,
                tag: 12,
                panic_after_release: false,
            },
            SemanticSessionActorLimitsV1 {
                max_commands: 1,
                max_controls: 1,
            },
        )
        .unwrap();
        let drive = owner.try_drive_provider().unwrap();
        entered_rx.recv().unwrap();
        let SemanticSessionActorEventAdmissionV1::Submitted(first) =
            owner.try_event(policy_event(1))
        else {
            panic!("first event was not admitted")
        };
        let SemanticSessionActorEventAdmissionV1::NotSent { reason, event } =
            owner.try_event(policy_event(2))
        else {
            panic!("full control mailbox admitted a second event")
        };
        assert_eq!(reason, SemanticSessionActorControlRefusalV1::Full);
        let SemanticSessionActorExternalEventV1::PolicyLost { policy_epoch, .. } = event;
        assert_eq!(policy_epoch, 2);
        release_tx.send(()).unwrap();
        assert_eq!(
            drive.receive(),
            SemanticSessionActorControlCompletionV1::Applied
        );
        assert_eq!(
            first.receive(),
            SemanticSessionActorControlCompletionV1::Applied
        );
        assert!(owner.begin_drain());
        owner.try_stop().unwrap();
        assert_eq!(
            recover(recovery).termination(),
            SemanticSessionActorTerminationV1::Clean
        );
    }

    #[test]
    fn stop_fences_late_controls_without_discarding_admitted_control() {
        let broker = SemanticSessionBrokerV1::try_new(
            reference(3, SemanticSessionIncarnationV1::new),
            SemanticSessionLimitsV1::reference(),
        )
        .unwrap();
        let (entered_tx, entered_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let (owner, sender, recovery) = spawn_semantic_session_actor_v1(
            broker,
            BlockingPollPort {
                entered: entered_tx,
                release: release_rx,
                tag: 13,
                panic_after_release: false,
            },
            SemanticSessionActorLimitsV1 {
                max_commands: 1,
                max_controls: 2,
            },
        )
        .unwrap();
        let drive = owner.try_drive_provider().unwrap();
        entered_rx.recv().unwrap();
        let SemanticSessionActorEventAdmissionV1::Submitted(trailing) =
            owner.try_event(policy_event(1))
        else {
            panic!("trailing control was not admitted")
        };
        assert!(owner.begin_drain());
        thread::scope(|scope| {
            let stop = scope.spawn(|| owner.try_stop());
            for _ in 0..100_000 {
                if sender.phase() == SemanticSessionActorPhaseV1::Stopping {
                    break;
                }
                thread::yield_now();
            }
            assert_eq!(sender.phase(), SemanticSessionActorPhaseV1::Stopping);
            let SemanticSessionActorEventAdmissionV1::NotSent { reason, event } =
                owner.try_event(policy_event(2))
            else {
                panic!("stopping actor admitted a late control")
            };
            assert_eq!(reason, SemanticSessionActorControlRefusalV1::Closed);
            let SemanticSessionActorExternalEventV1::PolicyLost { policy_epoch, .. } = event;
            assert_eq!(policy_epoch, 2);
            release_tx.send(()).unwrap();
            assert_eq!(stop.join().unwrap(), Ok(()));
        });
        assert_eq!(
            drive.receive(),
            SemanticSessionActorControlCompletionV1::Applied
        );
        assert_eq!(
            trailing.receive(),
            SemanticSessionActorControlCompletionV1::Applied
        );
        assert_eq!(
            recover(recovery).termination(),
            SemanticSessionActorTerminationV1::Clean
        );
    }

    #[test]
    fn fatal_provider_poll_failure_is_not_false_absence() {
        let broker = SemanticSessionBrokerV1::try_new(
            reference(3, SemanticSessionIncarnationV1::new),
            SemanticSessionLimitsV1::reference(),
        )
        .unwrap();
        let (owner, sender, recovery) = spawn_semantic_session_actor_v1(
            broker,
            FailingPollPort(31),
            SemanticSessionActorLimitsV1::reference(),
        )
        .unwrap();
        let drive = owner.try_drive_provider().unwrap();
        assert_eq!(
            drive.receive(),
            SemanticSessionActorControlCompletionV1::OutcomeUnknown
        );
        assert_eq!(sender.phase(), SemanticSessionActorPhaseV1::Failed);
        assert_eq!(sender.usage().provider_poll_failures, 1);
        let SemanticSessionActorAdmissionV1::NotSent { reason, command } =
            sender.try_exchange(context(), describe(30))
        else {
            panic!("failed actor admitted a command")
        };
        assert_eq!(reason, SemanticSessionActorNotSentReasonV1::Closed);
        assert_eq!(
            command.exchange(),
            reference(30, SemanticExchangeRefV1::new)
        );
        let recovered = recover(recovery);
        assert_eq!(
            recovered.termination(),
            SemanticSessionActorTerminationV1::Failed
        );
        assert_eq!(recovered.port().0, 31);
        assert_eq!(recovered.broker().usage().provider_drive_failures, 1);
        assert_eq!(
            recovered.broker().usage().provider_drive_failures_by_class,
            [0, 1, 0]
        );
    }

    #[test]
    fn expiry_no_due_is_a_truthful_serialized_control() {
        let (owner, sender, recovery) = actor();
        let drive = owner.try_drive_expiry().unwrap();
        assert_eq!(
            drive.receive(),
            SemanticSessionActorControlCompletionV1::Applied
        );
        assert_eq!(sender.usage().expiry_drives, 1);
        assert_eq!(sender.usage().expiry_no_due, 1);
        assert_eq!(sender.usage().expiry_events, 0);
        assert!(owner.begin_drain());
        assert_eq!(owner.try_stop(), Ok(()));
        assert_eq!(
            recover(recovery).termination(),
            SemanticSessionActorTerminationV1::Clean
        );
    }

    #[test]
    fn expiry_failure_is_unknown_fatal_and_recovers_exact_port() {
        let broker = SemanticSessionBrokerV1::try_new(
            reference(3, SemanticSessionIncarnationV1::new),
            SemanticSessionLimitsV1::reference(),
        )
        .unwrap();
        let (owner, sender, recovery) = spawn_semantic_session_actor_v1(
            broker,
            FailingExpiryPort(37),
            SemanticSessionActorLimitsV1::reference(),
        )
        .unwrap();
        assert_eq!(
            owner.try_drive_expiry().unwrap().receive(),
            SemanticSessionActorControlCompletionV1::OutcomeUnknown
        );
        assert_eq!(sender.phase(), SemanticSessionActorPhaseV1::Failed);
        let usage = sender.usage();
        assert_eq!(usage.expiry_drives, 1);
        assert_eq!(usage.expiry_failures, 1);
        assert_eq!(usage.expiry_failures_by_reason, [0, 0, 1]);
        assert_eq!(usage.outcome_unknown, 1);
        let recovered = recover(recovery);
        assert_eq!(recovered.port().0, 37);
        assert_eq!(
            recovered.termination(),
            SemanticSessionActorTerminationV1::Failed
        );
    }

    #[test]
    fn composite_provider_drive_divergence_is_fatal_not_partial_success() {
        let broker = SemanticSessionBrokerV1::try_new(
            reference(3, SemanticSessionIncarnationV1::new),
            SemanticSessionLimitsV1::reference(),
        )
        .unwrap();
        let (owner, sender, recovery) = spawn_semantic_session_actor_v1(
            broker,
            MismatchedDrivePort,
            SemanticSessionActorLimitsV1::reference(),
        )
        .unwrap();
        let drive = owner.try_drive_provider().unwrap();
        assert_eq!(
            drive.receive(),
            SemanticSessionActorControlCompletionV1::OutcomeUnknown
        );
        assert_eq!(sender.phase(), SemanticSessionActorPhaseV1::Failed);
        assert_eq!(sender.usage().broker_divergences, 1);
        assert_eq!(sender.usage().outcome_unknown, 1);
        let recovered = recover(recovery);
        assert_eq!(
            recovered.termination(),
            SemanticSessionActorTerminationV1::Failed
        );
    }

    #[test]
    fn effect_report_divergence_makes_command_unknown_and_terminates() {
        let broker = ready_broker();
        let (_owner, sender, recovery) = spawn_semantic_session_actor_v1(
            broker,
            MismatchedOpenPort,
            SemanticSessionActorLimitsV1::reference(),
        )
        .unwrap();
        let joined = exchange(
            &sender,
            SemanticSessionCommandV1::JoinSession {
                exchange: reference(41, SemanticExchangeRefV1::new),
                operation: reference(42, SemanticSessionOperationRefV1::new),
                incarnation: reference(3, SemanticSessionIncarnationV1::new),
                requested_roles: SemanticSessionRolesV1::top(),
            },
        );
        let SemanticSessionOutcomeV1::Joined { membership, .. } = joined.outcome() else {
            panic!("join was refused")
        };
        let membership = *membership;
        let epoch = reference(43, SemanticSynchronizationEpochV1::new);
        let limits = SemanticSessionLimitsV1::reference();
        let snapshot = SemanticSessionSnapshotV1::try_new(
            SemanticTargetV1 {
                document: SemanticDocumentRefV1::new([44; 16]),
                source_revision: SemanticSourceRevisionV1::new(1),
            },
            "rust".into(),
            "fn divergent() {}".into(),
            limits,
        )
        .unwrap();
        let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
            std::slice::from_ref(&snapshot),
            limits,
        )
        .unwrap();
        assert_eq!(
            exchange(
                &sender,
                SemanticSessionCommandV1::BeginSynchronization {
                    exchange: reference(45, SemanticExchangeRefV1::new),
                    operation: reference(46, SemanticSessionOperationRefV1::new),
                    incarnation: reference(3, SemanticSessionIncarnationV1::new),
                    membership,
                    epoch,
                    manifest,
                }
            )
            .outcome(),
            &SemanticSessionOutcomeV1::Accepted
        );
        let SemanticSessionActorAdmissionV1::Submitted(ticket) = sender.try_exchange(
            context(),
            SemanticSessionCommandV1::SyncOpen {
                exchange: reference(47, SemanticExchangeRefV1::new),
                operation: reference(48, SemanticSessionOperationRefV1::new),
                incarnation: reference(3, SemanticSessionIncarnationV1::new),
                membership,
                epoch,
                snapshot,
            },
        ) else {
            panic!("open was not admitted")
        };
        assert_eq!(
            ticket.receive(),
            SemanticExchangeDispositionV1::OutcomeUnknown
        );
        assert_eq!(sender.phase(), SemanticSessionActorPhaseV1::Failed);
        let usage = sender.usage();
        assert_eq!(usage.broker_divergences, 1);
        assert_eq!(usage.outcome_unknown, 1);
        assert_eq!(usage.in_flight_commands, 0);
        assert_eq!(
            recover(recovery).termination(),
            SemanticSessionActorTerminationV1::Failed
        );
    }

    #[test]
    fn every_admitted_effect_failure_is_unknown_fatal_and_recoverable() {
        for kind in 0..5 {
            let (broker, command) = effect_failure_case(kind);
            let (_owner, sender, recovery) = spawn_semantic_session_actor_v1(
                broker,
                FailingEffectPort {
                    kind,
                    tag: 100 + u64::try_from(kind).unwrap(),
                },
                SemanticSessionActorLimitsV1::reference(),
            )
            .unwrap();
            let SemanticSessionActorAdmissionV1::Submitted(ticket) =
                sender.try_exchange(context(), command)
            else {
                panic!("effect failure case {kind} was not admitted")
            };
            assert_eq!(
                ticket.receive(),
                SemanticExchangeDispositionV1::OutcomeUnknown,
                "effect kind {kind}"
            );
            assert_eq!(sender.phase(), SemanticSessionActorPhaseV1::Failed);
            let usage = sender.usage();
            assert_eq!(usage.effect_failures, 1);
            assert_eq!(usage.effect_failures_by_kind[kind], 1);
            assert_eq!(usage.effect_failures_by_reason[kind % 3], 1);
            assert_eq!(usage.outcome_unknown, 1);
            assert_eq!(usage.in_flight_commands, 0);
            let SemanticSessionActorAdmissionV1::NotSent { reason, .. } =
                sender.try_exchange(context(), describe(90))
            else {
                panic!("failed effect kind {kind} continued serving")
            };
            assert_eq!(reason, SemanticSessionActorNotSentReasonV1::Closed);
            let recovered = recover(recovery);
            assert_eq!(
                recovered.termination(),
                SemanticSessionActorTerminationV1::Failed
            );
            assert_eq!(recovered.port().tag, 100 + u64::try_from(kind).unwrap());
            assert_eq!(recovered.broker().usage().pending_effects, 1);
        }
    }

    #[test]
    fn affine_effect_is_completed_in_the_same_serial_turn() {
        let (owner, sender, recovery) = ready_actor();
        let joined = exchange(
            &sender,
            SemanticSessionCommandV1::JoinSession {
                exchange: reference(4, SemanticExchangeRefV1::new),
                operation: reference(5, SemanticSessionOperationRefV1::new),
                incarnation: reference(3, SemanticSessionIncarnationV1::new),
                requested_roles: SemanticSessionRolesV1::top(),
            },
        );
        let SemanticSessionOutcomeV1::Joined { membership, .. } = joined.outcome() else {
            panic!("join was refused")
        };
        let membership = *membership;
        let epoch = reference(6, SemanticSynchronizationEpochV1::new);
        let limits = SemanticSessionLimitsV1::reference();
        let snapshot = SemanticSessionSnapshotV1::try_new(
            SemanticTargetV1 {
                document: SemanticDocumentRefV1::new([7; 16]),
                source_revision: SemanticSourceRevisionV1::new(1),
            },
            "rust".into(),
            "fn main() {}".into(),
            limits,
        )
        .unwrap();
        let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
            std::slice::from_ref(&snapshot),
            limits,
        )
        .unwrap();
        let begun = exchange(
            &sender,
            SemanticSessionCommandV1::BeginSynchronization {
                exchange: reference(8, SemanticExchangeRefV1::new),
                operation: reference(9, SemanticSessionOperationRefV1::new),
                incarnation: reference(3, SemanticSessionIncarnationV1::new),
                membership,
                epoch,
                manifest,
            },
        );
        assert_eq!(begun.outcome(), &SemanticSessionOutcomeV1::Accepted);
        let opened = exchange(
            &sender,
            SemanticSessionCommandV1::SyncOpen {
                exchange: reference(10, SemanticExchangeRefV1::new),
                operation: reference(11, SemanticSessionOperationRefV1::new),
                incarnation: reference(3, SemanticSessionIncarnationV1::new),
                membership,
                epoch,
                snapshot,
            },
        );
        assert_eq!(opened.outcome(), &SemanticSessionOutcomeV1::Accepted);
        let usage = sender.usage();
        assert_eq!((usage.effects, usage.effects_by_kind[0]), (1, 1));
        drop(owner);
        drop(sender);
        assert_eq!(
            recover(recovery).termination(),
            SemanticSessionActorTerminationV1::Failed
        );
    }

    #[test]
    fn provider_panic_accounts_for_in_flight_outcome_unknown() {
        let broker = ready_broker();
        let (_owner, sender, recovery) = spawn_semantic_session_actor_v1(
            broker,
            PanickingOpenPort(32),
            SemanticSessionActorLimitsV1::reference(),
        )
        .unwrap();
        let joined = exchange(
            &sender,
            SemanticSessionCommandV1::JoinSession {
                exchange: reference(21, SemanticExchangeRefV1::new),
                operation: reference(22, SemanticSessionOperationRefV1::new),
                incarnation: reference(3, SemanticSessionIncarnationV1::new),
                requested_roles: SemanticSessionRolesV1::top(),
            },
        );
        let SemanticSessionOutcomeV1::Joined { membership, .. } = joined.outcome() else {
            panic!("join was refused")
        };
        let membership = *membership;
        let epoch = reference(23, SemanticSynchronizationEpochV1::new);
        let limits = SemanticSessionLimitsV1::reference();
        let snapshot = SemanticSessionSnapshotV1::try_new(
            SemanticTargetV1 {
                document: SemanticDocumentRefV1::new([24; 16]),
                source_revision: SemanticSourceRevisionV1::new(1),
            },
            "rust".into(),
            "fn panic_path() {}".into(),
            limits,
        )
        .unwrap();
        let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
            std::slice::from_ref(&snapshot),
            limits,
        )
        .unwrap();
        assert_eq!(
            exchange(
                &sender,
                SemanticSessionCommandV1::BeginSynchronization {
                    exchange: reference(25, SemanticExchangeRefV1::new),
                    operation: reference(26, SemanticSessionOperationRefV1::new),
                    incarnation: reference(3, SemanticSessionIncarnationV1::new),
                    membership,
                    epoch,
                    manifest,
                },
            )
            .outcome(),
            &SemanticSessionOutcomeV1::Accepted
        );
        let SemanticSessionActorAdmissionV1::Submitted(open) = sender.try_exchange(
            context(),
            SemanticSessionCommandV1::SyncOpen {
                exchange: reference(27, SemanticExchangeRefV1::new),
                operation: reference(28, SemanticSessionOperationRefV1::new),
                incarnation: reference(3, SemanticSessionIncarnationV1::new),
                membership,
                epoch,
                snapshot,
            },
        ) else {
            panic!("open was not admitted")
        };
        assert_eq!(
            open.receive(),
            SemanticExchangeDispositionV1::OutcomeUnknown
        );
        for _ in 0..100_000 {
            if sender.phase() == SemanticSessionActorPhaseV1::Failed {
                break;
            }
            thread::yield_now();
        }
        let usage = sender.usage();
        assert_eq!(sender.phase(), SemanticSessionActorPhaseV1::Failed);
        assert_eq!(usage.in_flight_commands, 0);
        assert_eq!(usage.outcome_unknown, 1);
        assert_eq!(usage.worker_panics, 1);
        let recovered = recover(recovery);
        assert_eq!(
            recovered.termination(),
            SemanticSessionActorTerminationV1::Failed
        );
        assert_eq!(recovered.port().0, 32);
    }

    #[test]
    fn clean_stop_refuses_fixed_cardinality_port_obligations() {
        for obligations in [
            SemanticBrokerEffectObligationsV1 {
                request_deadlines: 1,
                due_deadlines: 1,
                poisoned: false,
                consistent: true,
            },
            SemanticBrokerEffectObligationsV1 {
                request_deadlines: 0,
                due_deadlines: 1,
                poisoned: false,
                consistent: false,
            },
            SemanticBrokerEffectObligationsV1 {
                request_deadlines: 0,
                due_deadlines: 0,
                poisoned: true,
                consistent: true,
            },
        ] {
            let (owner, sender, recovery) = spawn_semantic_session_actor_v1(
                ready_broker(),
                ObligatedPort {
                    inner: AcceptingPort,
                    obligations,
                },
                SemanticSessionActorLimitsV1::reference(),
            )
            .unwrap();
            assert!(owner.begin_drain());
            assert_eq!(
                owner.try_stop(),
                Err(SemanticSessionActorControlRefusalV1::NotQuiescent)
            );
            assert_eq!(sender.phase(), SemanticSessionActorPhaseV1::Draining);
            let usage = sender.usage();
            assert_eq!(usage.stop_refusals, 1);
            assert_eq!(usage.clean_stops, 0);
            drop(owner);
            drop(sender);
            assert_eq!(
                recover(recovery).termination(),
                SemanticSessionActorTerminationV1::Failed
            );
        }
    }

    #[test]
    fn stop_ticket_fences_commands_and_observes_clean_completion_without_blocking() {
        let broker = SemanticSessionBrokerV1::try_new(
            reference(3, SemanticSessionIncarnationV1::new),
            SemanticSessionLimitsV1::reference(),
        )
        .unwrap();
        let (entered_tx, entered_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let (owner, sender, recovery) = spawn_semantic_session_actor_v1(
            broker,
            BlockingPollPort {
                entered: entered_tx,
                release: release_rx,
                tag: 93,
                panic_after_release: false,
            },
            SemanticSessionActorLimitsV1::reference(),
        )
        .unwrap();
        let drive = owner.try_drive_provider().unwrap();
        entered_rx.recv().unwrap();
        assert!(owner.begin_drain());
        let ticket = owner.try_begin_stop().unwrap();
        assert_eq!(sender.phase(), SemanticSessionActorPhaseV1::Stopping);
        let SemanticSessionActorAdmissionV1::NotSent { reason, .. } =
            sender.try_exchange(context(), describe(94))
        else {
            panic!("stopping actor admitted a command")
        };
        assert_eq!(reason, SemanticSessionActorNotSentReasonV1::Draining);

        let mut ticket = ticket
            .try_complete()
            .expect_err("a blocked worker cannot have observed stop");
        assert_eq!(sender.usage().stop_tickets, 1);
        assert_eq!(sender.usage().stop_ticket_polls, 1);
        assert_eq!(sender.usage().stop_ticket_pending, 1);

        release_tx.send(()).unwrap();
        assert_eq!(
            drive.receive(),
            SemanticSessionActorControlCompletionV1::Applied
        );
        let receipt = loop {
            match ticket.try_complete() {
                Ok(SemanticSessionActorStopResolutionV1::Clean(receipt)) => break receipt,
                Ok(other) => panic!("clean stop resolved unexpectedly: {other:?}"),
                Err(pending) => {
                    ticket = pending;
                    thread::yield_now();
                }
            }
        };
        assert_eq!(
            receipt.disposition(),
            SemanticSessionActorStopDispositionV1::Clean
        );
        assert_eq!(
            receipt.observed_phase(),
            SemanticSessionActorPhaseV1::Stopped
        );
        assert_eq!(receipt.usage().stop_tickets, 1);
        assert!(receipt.usage().stop_ticket_polls >= 2);
        assert_eq!(receipt.usage().clean_stops, 1);
        let recovered = recover(recovery);
        assert_eq!(
            recovered.termination(),
            SemanticSessionActorTerminationV1::Clean
        );
        assert_eq!(recovered.port().tag, 93);
    }

    #[test]
    fn refused_stop_ticket_returns_exact_owner_and_draining_evidence() {
        let obligations = SemanticBrokerEffectObligationsV1 {
            request_deadlines: 1,
            due_deadlines: 1,
            poisoned: false,
            consistent: true,
        };
        let (owner, sender, recovery) = spawn_semantic_session_actor_v1(
            ready_broker(),
            ObligatedPort {
                inner: AcceptingPort,
                obligations,
            },
            SemanticSessionActorLimitsV1::reference(),
        )
        .unwrap();
        assert!(owner.begin_drain());
        let mut ticket = owner.try_begin_stop().unwrap();
        let (receipt, owner) = loop {
            match ticket.try_complete() {
                Ok(SemanticSessionActorStopResolutionV1::Refused { receipt, owner }) => {
                    break (receipt, owner);
                }
                Ok(other) => panic!("obligated stop resolved unexpectedly: {other:?}"),
                Err(pending) => {
                    ticket = pending;
                    thread::yield_now();
                }
            }
        };
        assert_eq!(
            receipt.disposition(),
            SemanticSessionActorStopDispositionV1::Refused(
                SemanticSessionActorControlRefusalV1::NotQuiescent
            )
        );
        assert_eq!(
            receipt.observed_phase(),
            SemanticSessionActorPhaseV1::Draining
        );
        assert_eq!(owner.phase(), SemanticSessionActorPhaseV1::Draining);
        assert_eq!(receipt.usage().stop_refusals, 1);
        assert_eq!(receipt.usage().stop_tickets, 1);
        drop(owner);
        drop(sender);
        assert_eq!(
            recover(recovery).termination(),
            SemanticSessionActorTerminationV1::Failed
        );
    }

    #[test]
    fn failed_stop_admission_returns_owner_and_abandonment_is_not_termination() {
        let (owner, sender, recovery) = actor();
        let failure = owner
            .try_begin_stop()
            .expect_err("running actor must refuse stop admission");
        assert_eq!(
            failure.reason(),
            SemanticSessionActorControlRefusalV1::NotDraining
        );
        assert_eq!(
            format!("{failure:?}"),
            "SemanticSessionActorStopStartFailureV1 { reason: NotDraining, owner: \"<owned>\" }"
        );
        let owner = failure.into_owner();
        assert_eq!(owner.phase(), SemanticSessionActorPhaseV1::Running);
        assert!(owner.begin_drain());
        let abandonment = owner.try_begin_stop().unwrap().abandon();
        assert_eq!(
            abandonment.phase_before_abandonment(),
            SemanticSessionActorPhaseV1::Stopping
        );
        assert_eq!(abandonment.usage().stop_tickets, 1);
        assert_eq!(abandonment.usage().stop_abandonments, 1);
        assert_eq!(
            format!("{sender:?}"),
            "SemanticSessionActorSenderV1(<opaque>)"
        );
        drop(sender);
        let recovered = recover(recovery);
        assert!(matches!(
            recovered.termination(),
            SemanticSessionActorTerminationV1::Clean | SemanticSessionActorTerminationV1::Failed
        ));
    }

    #[test]
    fn dropping_a_live_stop_ticket_records_implicit_abandonment() {
        let (owner, sender, recovery) = actor();
        assert!(owner.begin_drain());
        let ticket = owner.try_begin_stop().unwrap();
        drop(ticket);
        assert_eq!(sender.usage().stop_abandonments, 1);
        drop(sender);
        let recovered = recover(recovery);
        assert!(matches!(
            recovered.termination(),
            SemanticSessionActorTerminationV1::Clean | SemanticSessionActorTerminationV1::Failed
        ));
    }

    #[test]
    fn stop_ticket_disconnect_is_outcome_unknown_and_recovery_remains_independent() {
        let (entered_tx, entered_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let (owner, sender, recovery) = spawn_semantic_session_actor_v1(
            ready_broker(),
            BlockingPollPort {
                entered: entered_tx,
                release: release_rx,
                tag: 95,
                panic_after_release: true,
            },
            SemanticSessionActorLimitsV1::reference(),
        )
        .unwrap();
        let drive = owner.try_drive_provider().unwrap();
        entered_rx.recv().unwrap();
        assert!(owner.begin_drain());
        let mut ticket = owner.try_begin_stop().unwrap();
        release_tx.send(()).unwrap();
        assert_eq!(
            drive.receive(),
            SemanticSessionActorControlCompletionV1::OutcomeUnknown
        );
        let receipt = loop {
            match ticket.try_complete() {
                Ok(SemanticSessionActorStopResolutionV1::OutcomeUnknown(receipt)) => break receipt,
                Ok(other) => panic!("disconnected stop resolved unexpectedly: {other:?}"),
                Err(pending) => {
                    ticket = pending;
                    thread::yield_now();
                }
            }
        };
        assert_eq!(
            receipt.disposition(),
            SemanticSessionActorStopDispositionV1::OutcomeUnknown
        );
        assert_eq!(
            receipt.observed_phase(),
            SemanticSessionActorPhaseV1::Stopping
        );
        assert_eq!(receipt.usage().stop_outcome_unknown, 1);
        drop(sender);
        let recovered = recover(recovery);
        assert_eq!(
            recovered.termination(),
            SemanticSessionActorTerminationV1::Failed
        );
        assert_eq!(recovered.port().tag, 95);
    }
}
