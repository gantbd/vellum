use std::sync::Arc;
use vellum_protocol::{
    ClientId, CommandV1, OutcomeV1, RefusalV1, ReplyV1, RequestId, RequestV1, SnapshotV1,
    VersionWitness,
};
use vellum_wire::{
    WireDecodeError, WireEncodeError, WireLimits, WireReplyV1, WireRequestV1, decode_reply_v1,
    decode_request_v1, encode_reply_v1, encode_request_v1,
};

fn fixture(source: &str) -> Vec<u8> {
    source
        .split_ascii_whitespace()
        .map(|byte| u8::from_str_radix(byte, 16).unwrap())
        .collect()
}

fn limits() -> WireLimits {
    WireLimits {
        max_frame_bytes: 256,
        max_text_bytes: 64,
        max_fields: 16,
    }
}

#[test]
fn golden_connect_frames_are_canonical() {
    let request = WireRequestV1::Connect;
    let request_fixture = fixture(include_str!("fixtures/connect-request-v1.hex"));
    assert_eq!(
        encode_request_v1(&request, limits()).unwrap(),
        request_fixture
    );
    assert_eq!(
        decode_request_v1(&request_fixture, limits()).unwrap(),
        request
    );

    let reply = WireReplyV1::Connected(ClientId::new(7).unwrap());
    let reply_fixture = fixture(include_str!("fixtures/connect-reply-v1.hex"));
    assert_eq!(encode_reply_v1(&reply, limits()).unwrap(), reply_fixture);
    assert_eq!(decode_reply_v1(&reply_fixture, limits()).unwrap(), reply);
}

#[test]
fn golden_semantic_frames_preserve_utf8_and_redacted_protocol_values() {
    let request = WireRequestV1::Execute(RequestV1 {
        client: ClientId::new(7).unwrap(),
        request: RequestId::new(9).unwrap(),
        command: CommandV1::Replace {
            expected: VersionWitness::new(11).unwrap(),
            start: 1,
            end: 3,
            inserted: "é".into(),
        },
    });
    let request_fixture = fixture(include_str!("fixtures/replace-request-v1.hex"));
    assert_eq!(
        encode_request_v1(&request, limits()).unwrap(),
        request_fixture
    );
    assert_eq!(
        decode_request_v1(&request_fixture, limits()).unwrap(),
        request
    );

    let reply = WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::Snapshot(SnapshotV1 {
        witness: VersionWitness::new(13).unwrap(),
        revision: 5,
        text: Arc::from("alpha"),
    })));
    let reply_fixture = fixture(include_str!("fixtures/snapshot-reply-v1.hex"));
    assert_eq!(encode_reply_v1(&reply, limits()).unwrap(), reply_fixture);
    assert_eq!(decode_reply_v1(&reply_fixture, limits()).unwrap(), reply);
}

#[test]
fn every_truncation_and_any_trailing_byte_refuses() {
    let encoded = fixture(include_str!("fixtures/replace-request-v1.hex"));
    for end in 0..encoded.len() {
        assert!(decode_request_v1(&encoded[..end], limits()).is_err());
    }
    let mut trailing = encoded;
    trailing.push(0);
    assert_eq!(
        decode_request_v1(&trailing, limits()),
        Err(WireDecodeError::TrailingData)
    );
}

#[test]
fn bounds_versions_tags_identities_and_utf8_are_closed_refusals() {
    let encoded = fixture(include_str!("fixtures/replace-request-v1.hex"));
    let mut frame_limit = limits();
    frame_limit.max_frame_bytes = encoded.len() - 1;
    assert_eq!(
        decode_request_v1(&encoded, frame_limit),
        Err(WireDecodeError::Limit)
    );

    let mut text_limit = limits();
    text_limit.max_text_bytes = 1;
    assert_eq!(
        decode_request_v1(&encoded, text_limit),
        Err(WireDecodeError::Limit)
    );
    assert_eq!(
        encode_request_v1(
            &WireRequestV1::Execute(RequestV1 {
                client: ClientId::new(1).unwrap(),
                request: RequestId::new(1).unwrap(),
                command: CommandV1::Replace {
                    expected: VersionWitness::new(1).unwrap(),
                    start: 0,
                    end: 0,
                    inserted: "too long".into(),
                },
            }),
            WireLimits {
                max_text_bytes: 1,
                ..limits()
            },
        ),
        Err(WireEncodeError::Limit)
    );
    assert_eq!(
        encode_request_v1(
            &WireRequestV1::Connect,
            WireLimits {
                max_fields: 1,
                ..limits()
            },
        ),
        Err(WireEncodeError::Limit)
    );

    let mut unsupported = encoded.clone();
    unsupported[0] = 2;
    assert_eq!(
        decode_request_v1(&unsupported, limits()),
        Err(WireDecodeError::UnsupportedVersion)
    );
    let mut unknown_kind = encoded.clone();
    unknown_kind[1] = 127;
    assert_eq!(
        decode_request_v1(&unknown_kind, limits()),
        Err(WireDecodeError::UnknownTag)
    );
    let mut zero_client = encoded.clone();
    zero_client[2..10].fill(0);
    assert_eq!(
        decode_request_v1(&zero_client, limits()),
        Err(WireDecodeError::InvalidIdentity)
    );
    let mut invalid_utf8 = encoded;
    let last = invalid_utf8.len() - 1;
    invalid_utf8[last] = 0xff;
    assert_eq!(
        decode_request_v1(&invalid_utf8, limits()),
        Err(WireDecodeError::InvalidUtf8)
    );
}

#[test]
fn every_closed_reply_variant_round_trips_and_debug_redacts_content() {
    let refusals = [
        RefusalV1::UnknownClient,
        RefusalV1::ClientCapacity,
        RefusalV1::RequestCapacity,
        RefusalV1::RequestConflict,
        RefusalV1::WitnessCapacity,
        RefusalV1::UnknownWitness,
        RefusalV1::Stale,
        RefusalV1::InvalidRange,
        RefusalV1::InputLimit,
        RefusalV1::SnapshotLimit,
        RefusalV1::EditorClosed,
        RefusalV1::EditorCapacity,
        RefusalV1::EditorConflict,
        RefusalV1::EditorIncomplete,
        RefusalV1::EditorUnavailable,
        RefusalV1::CounterExhausted,
    ];
    for refusal in refusals {
        for reply in [
            WireReplyV1::ConnectRefused(refusal),
            WireReplyV1::Executed(ReplyV1::Refused(refusal)),
        ] {
            let encoded = encode_reply_v1(&reply, limits()).unwrap();
            assert_eq!(decode_reply_v1(&encoded, limits()).unwrap(), reply);
        }
    }

    for outcome in [
        OutcomeV1::Snapshot(SnapshotV1 {
            witness: VersionWitness::new(1).unwrap(),
            revision: 2,
            text: Arc::from("private snapshot"),
        }),
        OutcomeV1::Replaced(SnapshotV1 {
            witness: VersionWitness::new(3).unwrap(),
            revision: 4,
            text: Arc::from("private replacement"),
        }),
    ] {
        let reply = WireReplyV1::Executed(ReplyV1::Accepted(outcome));
        let debug = format!("{reply:?}");
        assert!(!debug.contains("private"));
        let encoded = encode_reply_v1(&reply, limits()).unwrap();
        assert_eq!(decode_reply_v1(&encoded, limits()).unwrap(), reply);
    }

    let request = WireRequestV1::Execute(RequestV1 {
        client: ClientId::new(1).unwrap(),
        request: RequestId::new(1).unwrap(),
        command: CommandV1::Snapshot,
    });
    let encoded = encode_request_v1(&request, limits()).unwrap();
    assert_eq!(decode_request_v1(&encoded, limits()).unwrap(), request);
}
