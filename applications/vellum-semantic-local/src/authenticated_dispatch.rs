//! Application-owned authenticated dispatch for one local semantic session.
//!
//! Authentication establishes the subject/session pair only. An independent
//! role port derives the semantic role ceiling and policy epoch; authentication
//! success alone therefore grants no synchronization or query authority. This
//! module owns no socket, credential, provider, retry, or background loop.

use std::fmt;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use vellum_host::{authentication::AuthenticatedSessionV1, authority::AuthorityContextV1};
use vellum_semantic_protocol::{
    SemanticExchangeDispositionV1, SemanticSessionCommandV1, SemanticSessionReplyV1,
    SemanticSessionRolesV1,
};
use vellum_semantic_session_actor::{
    SemanticSessionActorAdmissionV1, SemanticSessionActorExternalEventV1,
    SemanticSessionActorNotSentReasonV1, SemanticSessionActorPollV1,
    SemanticSessionActorReplyTicketV1, SemanticSessionActorSenderV1,
};
use vellum_semantic_session_broker::SemanticBrokerContextV1;
use vellum_semantic_session_client::SemanticSessionExchangePortV1;
use vellum_semantic_wire::{
    SemanticWireDecodeErrorV1, SemanticWireEncodeErrorV1, SemanticWireLimitsV1, decode_command_v1,
    encode_reply_v1,
};

pub const MAX_AUTHENTICATED_SEMANTIC_EXCHANGES_V1: u64 = 1_048_576;
pub const MAX_AUTHENTICATED_SEMANTIC_COMPLETION_POLLS_V1: usize = 1_048_576;

/// Application-owned zero-policy bridge from the transport-neutral session
/// client to one actor sender. Composition supplies the already-authorized
/// broker context; this bridge derives no identity, role, policy epoch, or
/// retry behavior. It lives here so the service actor never depends on a
/// client crate.
#[derive(Clone)]
pub struct SemanticSessionActorExchangePortV1 {
    sender: SemanticSessionActorSenderV1,
    context: SemanticBrokerContextV1,
}

impl SemanticSessionActorExchangePortV1 {
    pub const fn new(
        sender: SemanticSessionActorSenderV1,
        context: SemanticBrokerContextV1,
    ) -> Self {
        Self { sender, context }
    }
}

impl fmt::Debug for SemanticSessionActorExchangePortV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SemanticSessionActorExchangePortV1(<redacted>)")
    }
}

impl SemanticSessionExchangePortV1 for SemanticSessionActorExchangePortV1 {
    fn exchange(
        &mut self,
        command: &SemanticSessionCommandV1,
    ) -> SemanticExchangeDispositionV1<SemanticSessionReplyV1> {
        match self.sender.try_exchange(self.context, command.clone()) {
            SemanticSessionActorAdmissionV1::Submitted(ticket) => ticket.receive(),
            SemanticSessionActorAdmissionV1::NotSent { .. } => {
                SemanticExchangeDispositionV1::NotSent
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthenticatedSemanticDispatchLimitsV1 {
    pub max_exchanges: u64,
    pub wire: SemanticWireLimitsV1,
}

impl AuthenticatedSemanticDispatchLimitsV1 {
    pub const fn reference() -> Self {
        Self {
            max_exchanges: 4_096,
            wire: SemanticWireLimitsV1::reference(),
        }
    }

    pub const fn is_valid(self) -> bool {
        self.max_exchanges > 0
            && self.max_exchanges <= MAX_AUTHENTICATED_SEMANTIC_EXCHANGES_V1
            && self.wire.is_valid()
    }
}

/// Independent semantic policy result for one authenticated subject/session.
/// A grant is a role ceiling, not evidence that the subject requested or joined
/// those roles. The broker still intersects it with each Join request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthenticatedSemanticRoleDecisionV1 {
    Grant {
        roles: SemanticSessionRolesV1,
        policy_epoch: u64,
    },
    Denied,
    Unavailable,
}

/// Application-owned authority port. It receives attribution established by
/// authentication but remains the sole source of semantic role policy.
pub trait AuthenticatedSemanticRolePortV1 {
    type Context: ?Sized;

    fn derive_roles(
        &mut self,
        authenticated: AuthorityContextV1,
        context: &Self::Context,
    ) -> AuthenticatedSemanticRoleDecisionV1;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AuthenticatedSemanticDispatchUsageV1 {
    pub role_derivations: u64,
    pub frames_attempted: u64,
    pub frames_decoded: u64,
    pub decode_refusals: u64,
    pub budget_refusals: u64,
    pub actor_submissions: u64,
    pub actor_full_refusals: u64,
    pub actor_draining_refusals: u64,
    pub actor_closed_refusals: u64,
    pub completion_polls: u64,
    pub completion_pending: u64,
    pub replies: u64,
    pub not_sent_completions: u64,
    pub outcome_unknown: u64,
    pub encode_refusals: u64,
    pub connection_exchanges: u64,
    pub connection_io_before_dispatch: u64,
    pub connection_io_after_dispatch: u64,
    pub completion_budget_exhausted: u64,
    pub saturated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthenticatedSemanticDispatchBuildRefusalV1 {
    InvalidLimits,
    RoleDenied,
    RoleUnavailable,
    InvalidAuthenticatedSession,
}

/// Build refusal retains the affine authenticated session and exact actor
/// sender. No credential, subject, session, or role-policy data enters Debug.
#[must_use = "dispatcher build refusal retains authenticated session ownership"]
pub struct AuthenticatedSemanticDispatchBuildFailureV1 {
    reason: AuthenticatedSemanticDispatchBuildRefusalV1,
    authenticated: AuthenticatedSessionV1,
    sender: SemanticSessionActorSenderV1,
    usage: AuthenticatedSemanticDispatchUsageV1,
}

impl AuthenticatedSemanticDispatchBuildFailureV1 {
    pub const fn reason(&self) -> AuthenticatedSemanticDispatchBuildRefusalV1 {
        self.reason
    }

    pub const fn usage(&self) -> AuthenticatedSemanticDispatchUsageV1 {
        self.usage
    }

    pub fn into_parts(self) -> (AuthenticatedSessionV1, SemanticSessionActorSenderV1) {
        (self.authenticated, self.sender)
    }
}

impl fmt::Debug for AuthenticatedSemanticDispatchBuildFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthenticatedSemanticDispatchBuildFailureV1")
            .field("reason", &self.reason)
            .field("usage", &self.usage)
            .field("authenticated", &"<owned>")
            .field("sender", &"<owned>")
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthenticatedSemanticDispatchRefusalV1 {
    BudgetExhausted,
    Decode(SemanticWireDecodeErrorV1),
    Actor(SemanticSessionActorNotSentReasonV1),
}

#[derive(Debug, Eq, PartialEq)]
pub enum AuthenticatedSemanticDispatchCompletionV1 {
    Reply(Vec<u8>),
    NotSent,
    OutcomeUnknown,
    EncodeRefused(SemanticWireEncodeErrorV1),
}

#[must_use = "an admitted authenticated semantic exchange must be observed"]
pub struct AuthenticatedSemanticDispatchTicketV1 {
    inner: SemanticSessionActorReplyTicketV1,
    wire: SemanticWireLimitsV1,
    usage: Arc<Mutex<AuthenticatedSemanticDispatchUsageV1>>,
}

impl fmt::Debug for AuthenticatedSemanticDispatchTicketV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AuthenticatedSemanticDispatchTicketV1(<opaque>)")
    }
}

#[must_use = "pending dispatch retains the exact reply ticket"]
pub enum AuthenticatedSemanticDispatchPollV1 {
    Pending(AuthenticatedSemanticDispatchTicketV1),
    Complete(AuthenticatedSemanticDispatchCompletionV1),
}

impl fmt::Debug for AuthenticatedSemanticDispatchPollV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Pending(_) => "Pending(<owned>)",
            Self::Complete(_) => "Complete(<content-safe-disposition>)",
        })
    }
}

impl AuthenticatedSemanticDispatchTicketV1 {
    pub fn try_complete(self) -> AuthenticatedSemanticDispatchPollV1 {
        update_usage(&self.usage, |usage| {
            increment(&mut usage.completion_polls, &mut usage.saturated)
        });
        let disposition = match self.inner.try_receive() {
            Ok(disposition) => disposition,
            Err(SemanticSessionActorPollV1::Pending) => {
                update_usage(&self.usage, |usage| {
                    increment(&mut usage.completion_pending, &mut usage.saturated)
                });
                return AuthenticatedSemanticDispatchPollV1::Pending(self);
            }
            Err(SemanticSessionActorPollV1::OutcomeUnknown) => {
                update_usage(&self.usage, |usage| {
                    increment(&mut usage.outcome_unknown, &mut usage.saturated)
                });
                return AuthenticatedSemanticDispatchPollV1::Complete(
                    AuthenticatedSemanticDispatchCompletionV1::OutcomeUnknown,
                );
            }
        };
        match disposition {
            SemanticExchangeDispositionV1::Reply(reply) => match encode_reply_v1(&reply, self.wire)
            {
                Ok(bytes) => {
                    update_usage(&self.usage, |usage| {
                        increment(&mut usage.replies, &mut usage.saturated)
                    });
                    AuthenticatedSemanticDispatchPollV1::Complete(
                        AuthenticatedSemanticDispatchCompletionV1::Reply(bytes),
                    )
                }
                Err(error) => {
                    update_usage(&self.usage, |usage| {
                        increment(&mut usage.encode_refusals, &mut usage.saturated)
                    });
                    AuthenticatedSemanticDispatchPollV1::Complete(
                        AuthenticatedSemanticDispatchCompletionV1::EncodeRefused(error),
                    )
                }
            },
            SemanticExchangeDispositionV1::NotSent => {
                update_usage(&self.usage, |usage| {
                    increment(&mut usage.not_sent_completions, &mut usage.saturated)
                });
                AuthenticatedSemanticDispatchPollV1::Complete(
                    AuthenticatedSemanticDispatchCompletionV1::NotSent,
                )
            }
            SemanticExchangeDispositionV1::OutcomeUnknown => {
                update_usage(&self.usage, |usage| {
                    increment(&mut usage.outcome_unknown, &mut usage.saturated)
                });
                AuthenticatedSemanticDispatchPollV1::Complete(
                    AuthenticatedSemanticDispatchCompletionV1::OutcomeUnknown,
                )
            }
        }
    }

    fn abandon_outcome_unknown(self) {
        update_usage(&self.usage, |usage| {
            increment(&mut usage.outcome_unknown, &mut usage.saturated)
        });
    }
}

/// One authenticated local connection bound to one actor sender and one
/// independently derived semantic role ceiling.
pub struct AuthenticatedSemanticDispatcherV1 {
    _authenticated: AuthenticatedSessionV1,
    sender: SemanticSessionActorSenderV1,
    broker_context: SemanticBrokerContextV1,
    limits: AuthenticatedSemanticDispatchLimitsV1,
    remaining_exchanges: u64,
    usage: Arc<Mutex<AuthenticatedSemanticDispatchUsageV1>>,
}

impl fmt::Debug for AuthenticatedSemanticDispatcherV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthenticatedSemanticDispatcherV1")
            .field("limits", &self.limits)
            .field("remaining_exchanges", &self.remaining_exchanges)
            .field("usage", &self.usage())
            .field("authenticated", &"<owned>")
            .field("sender", &"<owned>")
            .finish()
    }
}

impl AuthenticatedSemanticDispatcherV1 {
    pub fn try_new<R: AuthenticatedSemanticRolePortV1>(
        authenticated: AuthenticatedSessionV1,
        role_port: &mut R,
        role_context: &R::Context,
        sender: SemanticSessionActorSenderV1,
        limits: AuthenticatedSemanticDispatchLimitsV1,
    ) -> Result<Self, Box<AuthenticatedSemanticDispatchBuildFailureV1>> {
        let mut usage = AuthenticatedSemanticDispatchUsageV1::default();
        if !limits.is_valid() {
            return Err(Box::new(AuthenticatedSemanticDispatchBuildFailureV1 {
                reason: AuthenticatedSemanticDispatchBuildRefusalV1::InvalidLimits,
                authenticated,
                sender,
                usage,
            }));
        }
        increment(&mut usage.role_derivations, &mut usage.saturated);
        let authority = authenticated.authority_context();
        let Some(session) = authority.session else {
            return Err(Box::new(AuthenticatedSemanticDispatchBuildFailureV1 {
                reason: AuthenticatedSemanticDispatchBuildRefusalV1::InvalidAuthenticatedSession,
                authenticated,
                sender,
                usage,
            }));
        };
        let (allowed_roles, policy_epoch) = match role_port.derive_roles(authority, role_context) {
            AuthenticatedSemanticRoleDecisionV1::Grant {
                roles,
                policy_epoch,
            } => (roles, policy_epoch),
            AuthenticatedSemanticRoleDecisionV1::Denied => {
                return Err(Box::new(AuthenticatedSemanticDispatchBuildFailureV1 {
                    reason: AuthenticatedSemanticDispatchBuildRefusalV1::RoleDenied,
                    authenticated,
                    sender,
                    usage,
                }));
            }
            AuthenticatedSemanticRoleDecisionV1::Unavailable => {
                return Err(Box::new(AuthenticatedSemanticDispatchBuildFailureV1 {
                    reason: AuthenticatedSemanticDispatchBuildRefusalV1::RoleUnavailable,
                    authenticated,
                    sender,
                    usage,
                }));
            }
        };
        Ok(Self {
            _authenticated: authenticated,
            sender,
            broker_context: SemanticBrokerContextV1 {
                subject: authority.subject,
                session,
                allowed_roles,
                policy_epoch,
            },
            limits,
            remaining_exchanges: limits.max_exchanges,
            usage: Arc::new(Mutex::new(usage)),
        })
    }

    pub fn usage(&self) -> AuthenticatedSemanticDispatchUsageV1 {
        *self
            .usage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Exact authority-loss event for this authenticated connection. The
    /// composition root decides when transport loss terminates the authority
    /// lifetime; the dispatcher cannot submit owner-only actor events itself.
    pub const fn authority_lost_event(&self) -> SemanticSessionActorExternalEventV1 {
        SemanticSessionActorExternalEventV1::PolicyLost {
            subject: self.broker_context.subject,
            session: self.broker_context.session,
            policy_epoch: self.broker_context.policy_epoch,
        }
    }

    pub fn try_dispatch(
        &mut self,
        frame: &[u8],
    ) -> Result<AuthenticatedSemanticDispatchTicketV1, AuthenticatedSemanticDispatchRefusalV1> {
        update_usage(&self.usage, |usage| {
            increment(&mut usage.frames_attempted, &mut usage.saturated)
        });
        if self.remaining_exchanges == 0 {
            update_usage(&self.usage, |usage| {
                increment(&mut usage.budget_refusals, &mut usage.saturated)
            });
            return Err(AuthenticatedSemanticDispatchRefusalV1::BudgetExhausted);
        }
        self.remaining_exchanges -= 1;
        let command = decode_command_v1(frame, self.limits.wire).map_err(|error| {
            update_usage(&self.usage, |usage| {
                increment(&mut usage.decode_refusals, &mut usage.saturated)
            });
            AuthenticatedSemanticDispatchRefusalV1::Decode(error)
        })?;
        update_usage(&self.usage, |usage| {
            increment(&mut usage.frames_decoded, &mut usage.saturated)
        });
        match self.sender.try_exchange(self.broker_context, command) {
            SemanticSessionActorAdmissionV1::Submitted(inner) => {
                update_usage(&self.usage, |usage| {
                    increment(&mut usage.actor_submissions, &mut usage.saturated)
                });
                Ok(AuthenticatedSemanticDispatchTicketV1 {
                    inner,
                    wire: self.limits.wire,
                    usage: Arc::clone(&self.usage),
                })
            }
            SemanticSessionActorAdmissionV1::NotSent { reason, .. } => {
                update_usage(&self.usage, |usage| match reason {
                    SemanticSessionActorNotSentReasonV1::Full => {
                        increment(&mut usage.actor_full_refusals, &mut usage.saturated)
                    }
                    SemanticSessionActorNotSentReasonV1::Draining => {
                        increment(&mut usage.actor_draining_refusals, &mut usage.saturated)
                    }
                    SemanticSessionActorNotSentReasonV1::Closed => {
                        increment(&mut usage.actor_closed_refusals, &mut usage.saturated)
                    }
                });
                Err(AuthenticatedSemanticDispatchRefusalV1::Actor(reason))
            }
        }
    }
}

/// Bounded driver policy for one already-authenticated stream exchange.
/// Authentication handshake and stream deadlines remain owned by the caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthenticatedSemanticConnectionLimitsV1 {
    pub max_completion_polls: usize,
}

impl AuthenticatedSemanticConnectionLimitsV1 {
    pub const fn reference() -> Self {
        Self {
            max_completion_polls: 4_096,
        }
    }

    pub const fn is_valid(self) -> bool {
        self.max_completion_polls > 0
            && self.max_completion_polls <= MAX_AUTHENTICATED_SEMANTIC_COMPLETION_POLLS_V1
    }
}

/// Phase-aware terminal result for one already-authenticated local connection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthenticatedSemanticConnectionRefusalV1 {
    InvalidLimits,
    FrameLimit,
    IoBeforeDispatch,
    Dispatch(AuthenticatedSemanticDispatchRefusalV1),
    CompletionNotSent,
    CompletionOutcomeUnknown,
    CompletionEncodeRefused(SemanticWireEncodeErrorV1),
    CompletionBudgetExhausted,
    IoAfterDispatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthenticatedSemanticConnectionReceiptV1 {
    pub request_bytes: usize,
    pub reply_bytes: usize,
    pub completion_polls: usize,
    pub usage: AuthenticatedSemanticDispatchUsageV1,
}

/// Drives exactly one length-prefixed semantic command/reply exchange over an
/// already-authenticated stream. The dispatcher itself owns the affine
/// authenticated session; this function performs no handshake and cannot be
/// called with identity bytes alone. Callers must install read/write deadlines.
///
/// The completion loop is bounded. Exhaustion is classified as outcome unknown
/// because actor admission already occurred; it is never reported as NotSent.
pub fn serve_one_authenticated_semantic_exchange_v1<T: Read + Write>(
    dispatcher: &mut AuthenticatedSemanticDispatcherV1,
    stream: &mut T,
    limits: AuthenticatedSemanticConnectionLimitsV1,
) -> Result<AuthenticatedSemanticConnectionReceiptV1, AuthenticatedSemanticConnectionRefusalV1> {
    if !limits.is_valid() {
        return Err(AuthenticatedSemanticConnectionRefusalV1::InvalidLimits);
    }
    let frame = read_connection_frame(stream, dispatcher.limits.wire.max_frame_bytes).inspect_err(
        |reason| {
            if *reason == AuthenticatedSemanticConnectionRefusalV1::IoBeforeDispatch {
                update_usage(&dispatcher.usage, |usage| {
                    increment(
                        &mut usage.connection_io_before_dispatch,
                        &mut usage.saturated,
                    )
                });
            }
        },
    )?;
    let request_bytes = frame.len();
    let ticket = dispatcher
        .try_dispatch(&frame)
        .map_err(AuthenticatedSemanticConnectionRefusalV1::Dispatch)?;
    let mut ticket = ticket;
    for completion_polls in 1..=limits.max_completion_polls {
        match ticket.try_complete() {
            AuthenticatedSemanticDispatchPollV1::Pending(pending) => {
                ticket = pending;
                std::thread::yield_now();
            }
            AuthenticatedSemanticDispatchPollV1::Complete(
                AuthenticatedSemanticDispatchCompletionV1::Reply(reply),
            ) => {
                let reply_bytes = reply.len();
                write_connection_frame(stream, &reply, dispatcher.limits.wire.max_frame_bytes)
                    .map_err(|()| {
                        update_usage(&dispatcher.usage, |usage| {
                            increment(
                                &mut usage.connection_io_after_dispatch,
                                &mut usage.saturated,
                            )
                        });
                        AuthenticatedSemanticConnectionRefusalV1::IoAfterDispatch
                    })?;
                update_usage(&dispatcher.usage, |usage| {
                    increment(&mut usage.connection_exchanges, &mut usage.saturated)
                });
                return Ok(AuthenticatedSemanticConnectionReceiptV1 {
                    request_bytes,
                    reply_bytes,
                    completion_polls,
                    usage: dispatcher.usage(),
                });
            }
            AuthenticatedSemanticDispatchPollV1::Complete(
                AuthenticatedSemanticDispatchCompletionV1::NotSent,
            ) => {
                return Err(AuthenticatedSemanticConnectionRefusalV1::CompletionNotSent);
            }
            AuthenticatedSemanticDispatchPollV1::Complete(
                AuthenticatedSemanticDispatchCompletionV1::OutcomeUnknown,
            ) => {
                return Err(AuthenticatedSemanticConnectionRefusalV1::CompletionOutcomeUnknown);
            }
            AuthenticatedSemanticDispatchPollV1::Complete(
                AuthenticatedSemanticDispatchCompletionV1::EncodeRefused(reason),
            ) => {
                return Err(
                    AuthenticatedSemanticConnectionRefusalV1::CompletionEncodeRefused(reason),
                );
            }
        }
    }
    ticket.abandon_outcome_unknown();
    update_usage(&dispatcher.usage, |usage| {
        increment(&mut usage.completion_budget_exhausted, &mut usage.saturated)
    });
    Err(AuthenticatedSemanticConnectionRefusalV1::CompletionBudgetExhausted)
}

fn read_connection_frame<T: Read>(
    stream: &mut T,
    max_bytes: usize,
) -> Result<Vec<u8>, AuthenticatedSemanticConnectionRefusalV1> {
    let mut length = [0_u8; 4];
    stream
        .read_exact(&mut length)
        .map_err(|_| AuthenticatedSemanticConnectionRefusalV1::IoBeforeDispatch)?;
    let length = u32::from_be_bytes(length) as usize;
    if length > max_bytes {
        return Err(AuthenticatedSemanticConnectionRefusalV1::FrameLimit);
    }
    let mut frame = Vec::new();
    frame
        .try_reserve_exact(length)
        .map_err(|_| AuthenticatedSemanticConnectionRefusalV1::FrameLimit)?;
    frame.resize(length, 0);
    stream
        .read_exact(&mut frame)
        .map_err(|_| AuthenticatedSemanticConnectionRefusalV1::IoBeforeDispatch)?;
    Ok(frame)
}

fn write_connection_frame<T: Write>(
    stream: &mut T,
    frame: &[u8],
    max_bytes: usize,
) -> Result<(), ()> {
    if frame.len() > max_bytes {
        return Err(());
    }
    let length = u32::try_from(frame.len()).map_err(|_| ())?;
    stream.write_all(&length.to_be_bytes()).map_err(|_| ())?;
    stream.write_all(frame).map_err(|_| ())?;
    stream.flush().map_err(|_| ())
}

fn update_usage(
    usage: &Arc<Mutex<AuthenticatedSemanticDispatchUsageV1>>,
    update: impl FnOnce(&mut AuthenticatedSemanticDispatchUsageV1),
) {
    update(
        &mut usage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    );
}

fn increment(value: &mut u64, saturated: &mut bool) {
    let prior = *value;
    *value = value.saturating_add(1);
    *saturated |= prior == u64::MAX;
}

#[cfg(test)]
mod tests;
