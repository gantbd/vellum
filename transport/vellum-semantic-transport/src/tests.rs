use std::collections::VecDeque;

use super::*;
use vellum_semantic_protocol::{
    SemanticExchangeRefV1, SemanticMembershipRefV1, SemanticSessionCommandKindV1,
    SemanticSessionIncarnationV1, SemanticSessionOperationRefV1, SemanticSessionOutcomeV1,
    SemanticSessionRefusalV1, SemanticSessionRolesV1,
};

struct FakeExchange {
    outcomes: VecDeque<SemanticExchangeDispositionV1<Vec<u8>>>,
    calls: usize,
    requests: Vec<Vec<u8>>,
    reply_limits: Vec<usize>,
}

impl FakeExchange {
    fn one(outcome: SemanticExchangeDispositionV1<Vec<u8>>) -> Self {
        Self {
            outcomes: VecDeque::from([outcome]),
            calls: 0,
            requests: Vec::new(),
            reply_limits: Vec::new(),
        }
    }
}

impl SemanticFrameExchangeV1 for FakeExchange {
    fn exchange(
        &mut self,
        request: &[u8],
        max_reply_bytes: usize,
    ) -> SemanticExchangeDispositionV1<Vec<u8>> {
        self.calls += 1;
        self.requests.push(request.to_vec());
        self.reply_limits.push(max_reply_bytes);
        self.outcomes
            .pop_front()
            .expect("the adapter must never retry implicitly")
    }
}

fn exchange(tag: u8) -> SemanticExchangeRefV1 {
    SemanticExchangeRefV1::new([tag; 16]).unwrap()
}

fn operation(tag: u8) -> SemanticSessionOperationRefV1 {
    SemanticSessionOperationRefV1::new([tag; 16]).unwrap()
}

fn incarnation(tag: u8) -> SemanticSessionIncarnationV1 {
    SemanticSessionIncarnationV1::new([tag; 16]).unwrap()
}

fn membership(tag: u8) -> SemanticMembershipRefV1 {
    SemanticMembershipRefV1::new([tag; 16]).unwrap()
}

fn describe_command(tag: u8) -> SemanticSessionCommandV1 {
    SemanticSessionCommandV1::Describe {
        exchange: exchange(tag),
    }
}

fn describe_reply(tag: u8) -> SemanticSessionReplyV1 {
    SemanticSessionReplyV1::try_new(
        SemanticSessionCommandKindV1::Describe,
        exchange(tag),
        incarnation(2),
        None,
        None,
        SemanticSessionOutcomeV1::Unavailable,
    )
    .unwrap()
}

fn join_command(
    exchange_tag: u8,
    operation_tag: u8,
    incarnation_tag: u8,
) -> SemanticSessionCommandV1 {
    SemanticSessionCommandV1::JoinSession {
        exchange: exchange(exchange_tag),
        operation: operation(operation_tag),
        incarnation: incarnation(incarnation_tag),
        requested_roles: SemanticSessionRolesV1::query(),
    }
}

fn join_refusal(
    exchange_tag: u8,
    operation_tag: u8,
    incarnation_tag: u8,
) -> SemanticSessionReplyV1 {
    SemanticSessionReplyV1::try_new(
        SemanticSessionCommandKindV1::JoinSession,
        exchange(exchange_tag),
        incarnation(incarnation_tag),
        None,
        Some(operation(operation_tag)),
        SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::InvalidSession),
    )
    .unwrap()
}

fn joined_reply() -> SemanticSessionReplyV1 {
    SemanticSessionReplyV1::try_new(
        SemanticSessionCommandKindV1::JoinSession,
        exchange(1),
        incarnation(2),
        Some(membership(4)),
        Some(operation(3)),
        SemanticSessionOutcomeV1::Joined {
            membership: membership(4),
            granted_roles: SemanticSessionRolesV1::query(),
            remaining_operations: 32,
        },
    )
    .unwrap()
}

fn leave_command() -> SemanticSessionCommandV1 {
    SemanticSessionCommandV1::LeaveSession {
        exchange: exchange(5),
        operation: operation(6),
        incarnation: incarnation(7),
        membership: membership(8),
    }
}

fn leave_reply(membership_tag: u8) -> SemanticSessionReplyV1 {
    SemanticSessionReplyV1::try_new(
        SemanticSessionCommandKindV1::LeaveSession,
        exchange(5),
        incarnation(7),
        Some(membership(membership_tag)),
        Some(operation(6)),
        SemanticSessionOutcomeV1::Accepted,
    )
    .unwrap()
}

fn encoded(reply: &SemanticSessionReplyV1) -> Vec<u8> {
    encode_reply_v1(reply, SemanticWireLimitsV1::reference()).unwrap()
}

#[test]
fn canonical_exact_bound_reply_is_admitted_once() {
    let command = describe_command(1);
    let reply = describe_reply(1);
    let bytes = encoded(&reply);
    let reply_bytes = bytes.len() as u64;
    let request = encode_command_v1(&command, SemanticWireLimitsV1::reference()).unwrap();
    let fake = FakeExchange::one(SemanticExchangeDispositionV1::Reply(bytes));
    let mut remote =
        SemanticRemoteSessionV1::try_new(fake, SemanticTransportLimitsV1::reference()).unwrap();

    assert_eq!(
        SemanticSessionExchangePortV1::exchange(&mut remote, &command),
        SemanticExchangeDispositionV1::Reply(reply)
    );
    assert_eq!(remote.usage().attempts, 1);
    assert_eq!(remote.usage().exchange_calls, 1);
    assert_eq!(remote.usage().request_bytes, request.len() as u64);
    assert_eq!(remote.usage().received_reply_bytes, reply_bytes);
    assert_eq!(remote.usage().replies, 1);
    let fake = remote.into_exchange();
    assert_eq!(fake.calls, 1);
    assert_eq!(fake.requests, [request]);
    assert_eq!(
        fake.reply_limits,
        [SemanticTransportLimitsV1::reference().max_reply_bytes]
    );
}

#[test]
fn canonical_join_and_bound_leave_successes_are_admitted() {
    let join = join_command(1, 3, 2);
    let joined = joined_reply();
    let mut remote = SemanticRemoteSessionV1::try_new(
        FakeExchange::one(SemanticExchangeDispositionV1::Reply(encoded(&joined))),
        SemanticTransportLimitsV1::reference(),
    )
    .unwrap();
    assert_eq!(
        remote.exchange(&join),
        SemanticExchangeDispositionV1::Reply(joined)
    );
    assert_eq!(remote.usage().replies, 1);
    assert_eq!(remote.into_exchange().calls, 1);

    let leave = leave_command();
    let left = leave_reply(8);
    let mut remote = SemanticRemoteSessionV1::try_new(
        FakeExchange::one(SemanticExchangeDispositionV1::Reply(encoded(&left))),
        SemanticTransportLimitsV1::reference(),
    )
    .unwrap();
    assert_eq!(
        remote.exchange(&leave),
        SemanticExchangeDispositionV1::Reply(left)
    );
    assert_eq!(remote.usage().replies, 1);
    assert_eq!(remote.into_exchange().calls, 1);
}

#[test]
fn underlying_phase_truth_is_preserved_without_retry() {
    for (underlying, expected_not_sent, expected_unknown) in [
        (SemanticExchangeDispositionV1::NotSent, 1, 0),
        (SemanticExchangeDispositionV1::OutcomeUnknown, 0, 1),
    ] {
        let fake = FakeExchange::one(underlying);
        let mut remote =
            SemanticRemoteSessionV1::try_new(fake, SemanticTransportLimitsV1::reference()).unwrap();
        let outcome = remote.exchange(&describe_command(1));
        assert_eq!(
            outcome,
            if expected_not_sent == 1 {
                SemanticExchangeDispositionV1::NotSent
            } else {
                SemanticExchangeDispositionV1::OutcomeUnknown
            }
        );
        assert_eq!(remote.usage().not_sent, expected_not_sent);
        assert_eq!(remote.usage().outcome_unknown, expected_unknown);
        assert_eq!(remote.into_exchange().calls, 1);
    }
}

#[test]
fn local_encode_and_transport_limits_refuse_before_exchange() {
    let tiny_wire = SemanticWireLimitsV1 {
        max_frame_bytes: 16,
        max_field_bytes: 16,
        max_fields: 16,
        session: vellum_semantic_protocol::SemanticSessionLimitsV1::reference(),
    };
    let encode_limits = SemanticTransportLimitsV1 {
        max_request_bytes: 16,
        max_reply_bytes: 16,
        wire: tiny_wire,
    };
    let mut encode_refusal = SemanticRemoteSessionV1::try_new(
        FakeExchange::one(SemanticExchangeDispositionV1::OutcomeUnknown),
        encode_limits,
    )
    .unwrap();
    assert_eq!(
        encode_refusal.exchange(&describe_command(1)),
        SemanticExchangeDispositionV1::NotSent
    );
    assert_eq!(encode_refusal.usage().local_encode_refusals, 1);
    assert_eq!(encode_refusal.into_exchange().calls, 0);

    let request_limits = SemanticTransportLimitsV1 {
        max_request_bytes: 8,
        ..SemanticTransportLimitsV1::reference()
    };
    let mut limit_refusal = SemanticRemoteSessionV1::try_new(
        FakeExchange::one(SemanticExchangeDispositionV1::OutcomeUnknown),
        request_limits,
    )
    .unwrap();
    assert_eq!(
        limit_refusal.exchange(&describe_command(1)),
        SemanticExchangeDispositionV1::NotSent
    );
    assert_eq!(limit_refusal.usage().local_limit_refusals, 1);
    assert_eq!(limit_refusal.into_exchange().calls, 0);
}

#[test]
fn oversized_and_malformed_replies_are_unknown_after_one_send() {
    let cases = [
        vec![0; 33],
        Vec::new(),
        vec![b'V', b'S', b'S', b'1', 1, 129, 0],
    ];
    for (index, bytes) in cases.into_iter().enumerate() {
        let limits = SemanticTransportLimitsV1 {
            max_reply_bytes: 32,
            ..SemanticTransportLimitsV1::reference()
        };
        let fake = FakeExchange::one(SemanticExchangeDispositionV1::Reply(bytes));
        let mut remote = SemanticRemoteSessionV1::try_new(fake, limits).unwrap();
        assert_eq!(
            remote.exchange(&describe_command(1)),
            SemanticExchangeDispositionV1::OutcomeUnknown
        );
        if index == 0 {
            assert_eq!(remote.usage().oversized_replies, 1);
        } else {
            assert_eq!(remote.usage().malformed_replies, 1);
        }
        assert_eq!(remote.into_exchange().calls, 1);
    }
}

#[test]
fn every_reply_binding_dimension_is_checked_after_send() {
    let cases = [
        (describe_command(1), describe_reply(9)),
        (describe_command(1), join_refusal(1, 3, 2)),
        (join_command(1, 3, 2), join_refusal(1, 9, 2)),
        (join_command(1, 3, 2), join_refusal(1, 3, 9)),
        (leave_command(), leave_reply(9)),
    ];
    for (command, reply) in cases {
        let fake = FakeExchange::one(SemanticExchangeDispositionV1::Reply(encoded(&reply)));
        let mut remote =
            SemanticRemoteSessionV1::try_new(fake, SemanticTransportLimitsV1::reference()).unwrap();
        assert_eq!(
            remote.exchange(&command),
            SemanticExchangeDispositionV1::OutcomeUnknown
        );
        assert_eq!(remote.usage().mismatched_replies, 1);
        assert_eq!(remote.into_exchange().calls, 1);
    }
}

#[test]
fn limits_are_closed_debug_is_redacted_and_counters_saturate() {
    let invalid = [
        SemanticTransportLimitsV1 {
            max_request_bytes: 0,
            ..SemanticTransportLimitsV1::reference()
        },
        SemanticTransportLimitsV1 {
            max_reply_bytes: 0,
            ..SemanticTransportLimitsV1::reference()
        },
        SemanticTransportLimitsV1 {
            max_request_bytes: MAX_SEMANTIC_TRANSPORT_FRAME_BYTES_V1 + 1,
            ..SemanticTransportLimitsV1::reference()
        },
        SemanticTransportLimitsV1 {
            max_reply_bytes: MAX_SEMANTIC_TRANSPORT_FRAME_BYTES_V1 + 1,
            ..SemanticTransportLimitsV1::reference()
        },
        SemanticTransportLimitsV1 {
            wire: SemanticWireLimitsV1 {
                max_frame_bytes: 0,
                ..SemanticWireLimitsV1::reference()
            },
            ..SemanticTransportLimitsV1::reference()
        },
    ];
    for limits in invalid {
        assert_eq!(
            SemanticRemoteSessionV1::try_new(
                FakeExchange::one(SemanticExchangeDispositionV1::NotSent),
                limits,
            )
            .unwrap_err(),
            SemanticTransportBuildErrorV1::InvalidLimits
        );
    }

    let reply = describe_reply(1);
    let fake = FakeExchange::one(SemanticExchangeDispositionV1::Reply(encoded(&reply)));
    let mut remote =
        SemanticRemoteSessionV1::try_new(fake, SemanticTransportLimitsV1::reference()).unwrap();
    assert_eq!(format!("{remote:?}"), "SemanticRemoteSessionV1(<redacted>)");
    remote.usage.attempts = u64::MAX;
    remote.usage.request_bytes = u64::MAX;
    remote.usage.received_reply_bytes = u64::MAX;
    assert_eq!(
        remote.exchange(&describe_command(1)),
        SemanticExchangeDispositionV1::Reply(reply)
    );
    assert_eq!(remote.usage().attempts, u64::MAX);
    assert_eq!(remote.usage().request_bytes, u64::MAX);
    assert_eq!(remote.usage().received_reply_bytes, u64::MAX);
    assert!(remote.usage().counter_saturated);
}
