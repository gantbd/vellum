//! Phase-aware, transport-neutral semantic-session exchange adapter.
//!
//! The portable core owns no socket, authentication, retry, identity allocation,
//! provider, clock, or server authority. It converts one canonical semantic
//! command into one bounded frame exchange and admits a reply only after exact
//! canonical and command-binding validation. Once an exchange is called, an
//! untrustworthy response is conservatively `OutcomeUnknown`, never `NotSent`.
#![forbid(unsafe_code)]

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;

use vellum_semantic_protocol::{
    SemanticExchangeDispositionV1, SemanticSessionCommandV1, SemanticSessionReplyV1,
};
use vellum_semantic_session_client::SemanticSessionExchangePortV1;
use vellum_semantic_wire::{
    SemanticWireLimitsV1, decode_reply_v1, encode_command_v1, encode_reply_v1,
};

pub const MAX_SEMANTIC_TRANSPORT_FRAME_BYTES_V1: usize = 16 * 1024 * 1024;

/// One complete semantic request/reply frame exchange.
///
/// An implementation returns `NotSent` only when no request byte reached the
/// peer. Once transmission begins, lack of one trustworthy complete reply is
/// `OutcomeUnknown`. This port supplies no retry policy.
pub trait SemanticFrameExchangeV1 {
    fn exchange(
        &mut self,
        request: &[u8],
        max_reply_bytes: usize,
    ) -> SemanticExchangeDispositionV1<Vec<u8>>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticTransportLimitsV1 {
    pub max_request_bytes: usize,
    pub max_reply_bytes: usize,
    pub wire: SemanticWireLimitsV1,
}

impl SemanticTransportLimitsV1 {
    pub const fn reference() -> Self {
        Self {
            max_request_bytes: MAX_SEMANTIC_TRANSPORT_FRAME_BYTES_V1,
            max_reply_bytes: MAX_SEMANTIC_TRANSPORT_FRAME_BYTES_V1,
            wire: SemanticWireLimitsV1::reference(),
        }
    }

    pub const fn is_valid(self) -> bool {
        self.max_request_bytes > 0
            && self.max_request_bytes <= MAX_SEMANTIC_TRANSPORT_FRAME_BYTES_V1
            && self.max_reply_bytes > 0
            && self.max_reply_bytes <= MAX_SEMANTIC_TRANSPORT_FRAME_BYTES_V1
            && self.wire.is_valid()
            && self.wire.max_frame_bytes <= MAX_SEMANTIC_TRANSPORT_FRAME_BYTES_V1
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticTransportBuildErrorV1 {
    InvalidLimits,
}

/// Fixed-cardinality, content-safe transport observations.
///
/// `request_bytes` counts canonical command-frame bytes handed to the exchange
/// port. `received_reply_bytes` counts bytes returned in a `Reply` disposition
/// before semantic validation, including refused frames. They are logical
/// frame-content accounting, not heap, allocation, kernel-buffer, physical I/O,
/// or successfully transmitted/received byte claims.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SemanticTransportUsageV1 {
    pub attempts: u64,
    pub exchange_calls: u64,
    pub request_bytes: u64,
    pub received_reply_bytes: u64,
    pub replies: u64,
    pub not_sent: u64,
    pub outcome_unknown: u64,
    pub local_encode_refusals: u64,
    pub local_limit_refusals: u64,
    pub oversized_replies: u64,
    pub malformed_replies: u64,
    pub noncanonical_replies: u64,
    pub mismatched_replies: u64,
    pub counter_saturated: bool,
}

/// Zero-authority adapter from canonical bytes to the semantic client port.
pub struct SemanticRemoteSessionV1<X> {
    exchange: X,
    limits: SemanticTransportLimitsV1,
    usage: SemanticTransportUsageV1,
}

impl<X> std::fmt::Debug for SemanticRemoteSessionV1<X> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SemanticRemoteSessionV1(<redacted>)")
    }
}

impl<X> SemanticRemoteSessionV1<X> {
    pub fn try_new(
        exchange: X,
        limits: SemanticTransportLimitsV1,
    ) -> Result<Self, SemanticTransportBuildErrorV1> {
        if !limits.is_valid() {
            return Err(SemanticTransportBuildErrorV1::InvalidLimits);
        }
        Ok(Self {
            exchange,
            limits,
            usage: SemanticTransportUsageV1::default(),
        })
    }

    pub const fn limits(&self) -> SemanticTransportLimitsV1 {
        self.limits
    }

    pub const fn usage(&self) -> SemanticTransportUsageV1 {
        self.usage
    }

    pub fn into_exchange(self) -> X {
        self.exchange
    }

    fn not_sent(&mut self) -> SemanticExchangeDispositionV1<SemanticSessionReplyV1> {
        bump(&mut self.usage.not_sent, &mut self.usage.counter_saturated);
        SemanticExchangeDispositionV1::NotSent
    }

    fn outcome_unknown(&mut self) -> SemanticExchangeDispositionV1<SemanticSessionReplyV1> {
        bump(
            &mut self.usage.outcome_unknown,
            &mut self.usage.counter_saturated,
        );
        SemanticExchangeDispositionV1::OutcomeUnknown
    }
}

impl<X: SemanticFrameExchangeV1> SemanticSessionExchangePortV1 for SemanticRemoteSessionV1<X> {
    fn exchange(
        &mut self,
        command: &SemanticSessionCommandV1,
    ) -> SemanticExchangeDispositionV1<SemanticSessionReplyV1> {
        bump(&mut self.usage.attempts, &mut self.usage.counter_saturated);
        let request = match encode_command_v1(command, self.limits.wire) {
            Ok(request) => request,
            Err(_) => {
                bump(
                    &mut self.usage.local_encode_refusals,
                    &mut self.usage.counter_saturated,
                );
                return self.not_sent();
            }
        };
        if request.len() > self.limits.max_request_bytes {
            bump(
                &mut self.usage.local_limit_refusals,
                &mut self.usage.counter_saturated,
            );
            return self.not_sent();
        }
        bump(
            &mut self.usage.exchange_calls,
            &mut self.usage.counter_saturated,
        );
        add(
            &mut self.usage.request_bytes,
            request.len(),
            &mut self.usage.counter_saturated,
        );
        let bytes = match self
            .exchange
            .exchange(&request, self.limits.max_reply_bytes)
        {
            SemanticExchangeDispositionV1::NotSent => return self.not_sent(),
            SemanticExchangeDispositionV1::OutcomeUnknown => return self.outcome_unknown(),
            SemanticExchangeDispositionV1::Reply(bytes) => bytes,
        };
        add(
            &mut self.usage.received_reply_bytes,
            bytes.len(),
            &mut self.usage.counter_saturated,
        );
        if bytes.len() > self.limits.max_reply_bytes {
            bump(
                &mut self.usage.oversized_replies,
                &mut self.usage.counter_saturated,
            );
            return self.outcome_unknown();
        }
        let reply = match decode_reply_v1(&bytes, self.limits.wire) {
            Ok(reply) => reply,
            Err(_) => {
                bump(
                    &mut self.usage.malformed_replies,
                    &mut self.usage.counter_saturated,
                );
                return self.outcome_unknown();
            }
        };
        // The current strict decoder already rejects every known noncanonical
        // representation. Keep this independent equality gate and observation
        // as defense against future decoder relaxation; do not infer current
        // branch reachability or fabricate a noncanonical accepted fixture.
        if encode_reply_v1(&reply, self.limits.wire).as_deref() != Ok(bytes.as_slice()) {
            bump(
                &mut self.usage.noncanonical_replies,
                &mut self.usage.counter_saturated,
            );
            return self.outcome_unknown();
        }
        if !reply_matches(command, &reply) {
            bump(
                &mut self.usage.mismatched_replies,
                &mut self.usage.counter_saturated,
            );
            return self.outcome_unknown();
        }
        bump(&mut self.usage.replies, &mut self.usage.counter_saturated);
        SemanticExchangeDispositionV1::Reply(reply)
    }
}

fn reply_matches(command: &SemanticSessionCommandV1, reply: &SemanticSessionReplyV1) -> bool {
    if reply.kind() != command.kind()
        || reply.exchange() != command.exchange()
        || reply.operation() != command.operation()
    {
        return false;
    }
    match command {
        SemanticSessionCommandV1::Describe { .. } => true,
        SemanticSessionCommandV1::JoinSession { incarnation, .. } => {
            reply.incarnation() == *incarnation
        }
        _ => command.binding().is_some_and(|(incarnation, membership)| {
            reply.incarnation() == incarnation && reply.membership() == Some(membership)
        }),
    }
}

fn bump(value: &mut u64, saturated: &mut bool) {
    let previous = *value;
    *value = value.saturating_add(1);
    *saturated |= previous == u64::MAX;
}

fn add(value: &mut u64, amount: usize, saturated: &mut bool) {
    let amount = u64::try_from(amount).unwrap_or(u64::MAX);
    match value.checked_add(amount) {
        Some(total) => *value = total,
        None => {
            *value = u64::MAX;
            *saturated = true;
        }
    }
}

#[cfg(test)]
mod tests;
