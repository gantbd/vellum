use std::panic::{AssertUnwindSafe, catch_unwind};
use vellum_host::semantic::{
    SemanticDiagnosticSeverityV1, SemanticDiagnosticV1, SemanticDocumentRefV1,
    SemanticIndexGenerationV1, SemanticLimitsV1, SemanticLocationV1, SemanticProviderProvenanceV1,
    SemanticProviderRefV1, SemanticQueryKindV1, SemanticQueryV1, SemanticRefusalV1,
    SemanticReplyV1, SemanticRequestRefV1, SemanticRequestV1, SemanticResultV1,
    SemanticSourceRevisionV1, SemanticSpanV1, SemanticSymbolKindV1, SemanticSymbolV1,
    SemanticTargetV1, SemanticTextEditV1, SemanticTextV1,
};
use vellum_semantic_protocol::{
    SemanticExchangeRefV1, SemanticMembershipRefV1, SemanticProviderObservationV1,
    SemanticProviderReadinessProjectionV1, SemanticSessionCommandKindV1, SemanticSessionCommandV1,
    SemanticSessionDescriptionV1, SemanticSessionIncarnationV1, SemanticSessionLimitsV1,
    SemanticSessionOperationRefV1, SemanticSessionOutcomeV1, SemanticSessionPendingV1,
    SemanticSessionReadinessV1, SemanticSessionRefusalV1, SemanticSessionReplyV1,
    SemanticSessionRolesV1, SemanticSessionSnapshotV1, SemanticSynchronizationEpochV1,
    SemanticSynchronizationManifestV1,
};
use vellum_semantic_wire::{
    SemanticWireDecodeErrorV1, SemanticWireLimitsV1, decode_command_v1, decode_reply_v1,
    encode_command_v1, encode_reply_v1,
};

fn exchange() -> SemanticExchangeRefV1 {
    SemanticExchangeRefV1::new([1; 16]).unwrap()
}
fn operation() -> SemanticSessionOperationRefV1 {
    SemanticSessionOperationRefV1::new([2; 16]).unwrap()
}
fn incarnation() -> SemanticSessionIncarnationV1 {
    SemanticSessionIncarnationV1::new([3; 16]).unwrap()
}
fn membership() -> SemanticMembershipRefV1 {
    SemanticMembershipRefV1::new([4; 16]).unwrap()
}
fn epoch() -> SemanticSynchronizationEpochV1 {
    SemanticSynchronizationEpochV1::new([5; 16]).unwrap()
}
fn request_ref() -> SemanticRequestRefV1 {
    SemanticRequestRefV1::new([6; 16])
}
fn target() -> SemanticTargetV1 {
    SemanticTargetV1 {
        document: SemanticDocumentRefV1::new([7; 16]),
        source_revision: SemanticSourceRevisionV1::new(8),
    }
}
fn manifest() -> SemanticSynchronizationManifestV1 {
    SemanticSynchronizationManifestV1::new(1, 2, [9; 32], SemanticSessionLimitsV1::reference())
        .unwrap()
}
fn snapshot() -> SemanticSessionSnapshotV1 {
    SemanticSessionSnapshotV1::try_new(
        target(),
        "rust".into(),
        "fn main() {}".into(),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap()
}
fn request(query: SemanticQueryV1) -> SemanticRequestV1 {
    SemanticRequestV1 {
        request: request_ref(),
        target: target(),
        expected_index: Some(SemanticIndexGenerationV1::new(10).unwrap()),
        language: SemanticTextV1::try_new("rust".into(), 32).unwrap(),
        query,
    }
}

fn commands() -> Vec<SemanticSessionCommandV1> {
    let e = exchange();
    let o = operation();
    let i = incarnation();
    let m = membership();
    let s = epoch();
    vec![
        SemanticSessionCommandV1::Describe { exchange: e },
        SemanticSessionCommandV1::JoinSession {
            exchange: e,
            operation: o,
            incarnation: i,
            requested_roles: SemanticSessionRolesV1::top(),
        },
        SemanticSessionCommandV1::BeginSynchronization {
            exchange: e,
            operation: o,
            incarnation: i,
            membership: m,
            epoch: s,
            manifest: manifest(),
        },
        SemanticSessionCommandV1::SyncOpen {
            exchange: e,
            operation: o,
            incarnation: i,
            membership: m,
            epoch: s,
            snapshot: snapshot(),
        },
        SemanticSessionCommandV1::SyncChange {
            exchange: e,
            operation: o,
            incarnation: i,
            membership: m,
            epoch: s,
            snapshot: snapshot(),
        },
        SemanticSessionCommandV1::SyncClose {
            exchange: e,
            operation: o,
            incarnation: i,
            membership: m,
            epoch: s,
            target: target(),
        },
        SemanticSessionCommandV1::CompleteSynchronization {
            exchange: e,
            operation: o,
            incarnation: i,
            membership: m,
            epoch: s,
            manifest: manifest(),
        },
        SemanticSessionCommandV1::Submit {
            exchange: e,
            operation: o,
            incarnation: i,
            membership: m,
            request: request(SemanticQueryV1::Diagnostics),
        },
        SemanticSessionCommandV1::Cancel {
            exchange: e,
            operation: o,
            incarnation: i,
            membership: m,
            request: request_ref(),
        },
        SemanticSessionCommandV1::Collect {
            exchange: e,
            incarnation: i,
            membership: m,
            request: request_ref(),
        },
        SemanticSessionCommandV1::ReleaseRequest {
            exchange: e,
            operation: o,
            incarnation: i,
            membership: m,
            request: request_ref(),
        },
        SemanticSessionCommandV1::LeaveSession {
            exchange: e,
            operation: o,
            incarnation: i,
            membership: m,
        },
    ]
}

#[test]
fn all_commands_roundtrip_canonically() {
    let limits = SemanticWireLimitsV1::reference();
    for command in commands() {
        let bytes = encode_command_v1(&command, limits).unwrap();
        let decoded = decode_command_v1(&bytes, limits).unwrap();
        assert_eq!(decoded, command);
        assert_eq!(encode_command_v1(&decoded, limits).unwrap(), bytes);
        assert!(decode_reply_v1(&bytes, limits).is_err());
    }
}

#[test]
fn every_query_tag_roundtrips() {
    let text = || SemanticTextV1::try_new("x".into(), 32).unwrap();
    let queries = vec![
        SemanticQueryV1::Symbols { needle: text() },
        SemanticQueryV1::Definition { offset: 1 },
        SemanticQueryV1::References { offset: 2 },
        SemanticQueryV1::Implementations { offset: 3 },
        SemanticQueryV1::IncomingCalls { offset: 6 },
        SemanticQueryV1::OutgoingCalls { offset: 7 },
        SemanticQueryV1::Diagnostics,
        SemanticQueryV1::TypeAtPoint { offset: 4 },
        SemanticQueryV1::PlanRename {
            offset: 5,
            new_name: text(),
        },
    ];
    for query in queries {
        let command = SemanticSessionCommandV1::Submit {
            exchange: exchange(),
            operation: operation(),
            incarnation: incarnation(),
            membership: membership(),
            request: request(query),
        };
        let bytes = encode_command_v1(&command, SemanticWireLimitsV1::reference()).unwrap();
        assert_eq!(
            decode_command_v1(&bytes, SemanticWireLimitsV1::reference()).unwrap(),
            command
        );
    }
}

fn reply(
    kind: SemanticSessionCommandKindV1,
    outcome: SemanticSessionOutcomeV1,
) -> SemanticSessionReplyV1 {
    let (member, op) = match kind {
        SemanticSessionCommandKindV1::Describe => (None, None),
        SemanticSessionCommandKindV1::JoinSession => (
            match &outcome {
                SemanticSessionOutcomeV1::Joined { membership, .. } => Some(*membership),
                _ => None,
            },
            Some(operation()),
        ),
        SemanticSessionCommandKindV1::Collect => (Some(membership()), None),
        _ => (Some(membership()), Some(operation())),
    };
    SemanticSessionReplyV1::try_new(kind, exchange(), incarnation(), member, op, outcome).unwrap()
}

#[test]
fn representative_reply_for_every_command_roundtrips_and_is_domain_separated() {
    let description = SemanticSessionDescriptionV1::try_new(
        incarnation(),
        SemanticSessionRolesV1::top(),
        SemanticSessionReadinessV1::Unavailable,
        SemanticProviderObservationV1::new(
            SemanticProviderReadinessProjectionV1::Unavailable,
            vellum_host::semantic::SemanticCapabilitySetV1::bottom(),
        ),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    let replies = vec![
        reply(
            SemanticSessionCommandKindV1::Describe,
            SemanticSessionOutcomeV1::Description(description),
        ),
        reply(
            SemanticSessionCommandKindV1::JoinSession,
            SemanticSessionOutcomeV1::Joined {
                membership: membership(),
                granted_roles: SemanticSessionRolesV1::query(),
                remaining_operations: 1,
            },
        ),
        reply(
            SemanticSessionCommandKindV1::BeginSynchronization,
            SemanticSessionOutcomeV1::Accepted,
        ),
        reply(
            SemanticSessionCommandKindV1::SyncOpen,
            SemanticSessionOutcomeV1::Accepted,
        ),
        reply(
            SemanticSessionCommandKindV1::SyncChange,
            SemanticSessionOutcomeV1::Accepted,
        ),
        reply(
            SemanticSessionCommandKindV1::SyncClose,
            SemanticSessionOutcomeV1::Accepted,
        ),
        reply(
            SemanticSessionCommandKindV1::CompleteSynchronization,
            SemanticSessionOutcomeV1::Accepted,
        ),
        reply(
            SemanticSessionCommandKindV1::Submit,
            SemanticSessionOutcomeV1::Accepted,
        ),
        reply(
            SemanticSessionCommandKindV1::Cancel,
            SemanticSessionOutcomeV1::Accepted,
        ),
        reply(
            SemanticSessionCommandKindV1::Collect,
            SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Request(request_ref())),
        ),
        reply(
            SemanticSessionCommandKindV1::ReleaseRequest,
            SemanticSessionOutcomeV1::Accepted,
        ),
        reply(
            SemanticSessionCommandKindV1::LeaveSession,
            SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Retirement {
                remaining_documents: 1,
                quarantined_requests: 1,
            }),
        ),
    ];
    let limits = SemanticWireLimitsV1::reference();
    for value in replies {
        let bytes = encode_reply_v1(&value, limits).unwrap();
        let decoded = decode_reply_v1(&bytes, limits).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(encode_reply_v1(&decoded, limits).unwrap(), bytes);
        assert!(decode_command_v1(&bytes, limits).is_err());
    }
}

#[test]
fn adversarial_envelopes_are_rejected_and_never_panic() {
    let limits = SemanticWireLimitsV1::reference();
    let canonical = encode_command_v1(&commands()[0], limits).unwrap();
    let mut cases = vec![vec![], canonical[..canonical.len() - 1].to_vec()];
    let mut future = canonical.clone();
    future[4] = 2;
    cases.push(future);
    let mut unknown = canonical.clone();
    unknown[5] = 127;
    cases.push(unknown);
    let mut trailing = canonical.clone();
    trailing.push(0);
    cases.push(trailing);
    let mut zero_identity = canonical.clone();
    zero_identity[12..28].fill(0);
    cases.push(zero_identity);
    let mut duplicate = canonical.clone();
    duplicate[6] = 2;
    duplicate.extend_from_slice(&canonical[7..]);
    cases.push(duplicate);
    let mut oversized = canonical.clone();
    oversized[8..12].copy_from_slice(&u32::MAX.to_be_bytes());
    cases.push(oversized);
    for bytes in cases {
        let result = catch_unwind(AssertUnwindSafe(|| decode_command_v1(&bytes, limits)));
        assert!(result.is_ok());
        assert!(result.unwrap().is_err());
    }
    let mut tiny = limits;
    tiny.max_frame_bytes = canonical.len() - 1;
    tiny.max_field_bytes = tiny.max_frame_bytes;
    assert_eq!(
        decode_command_v1(&canonical, tiny),
        Err(SemanticWireDecodeErrorV1::Limit)
    );
}

fn field_payload(frame: &[u8], wanted: u8) -> std::ops::Range<usize> {
    let mut cursor = 7;
    for _ in 0..frame[6] {
        let tag = frame[cursor];
        let len = u32::from_be_bytes(frame[cursor + 1..cursor + 5].try_into().unwrap()) as usize;
        let range = cursor + 5..cursor + 5 + len;
        if tag == wanted {
            return range;
        }
        cursor = range.end;
    }
    panic!("missing field")
}

#[test]
fn nested_malformed_values_and_reordered_fields_refuse() {
    let limits = SemanticWireLimitsV1::reference();
    let mut join = encode_command_v1(&commands()[1], limits).unwrap();
    let roles = field_payload(&join, 5);
    join[roles.start] = 4;
    assert!(decode_command_v1(&join, limits).is_err());

    let mut open = encode_command_v1(&commands()[3], limits).unwrap();
    let snapshot = field_payload(&open, 8);
    open[snapshot.start + 28] = 0xff;
    assert_eq!(
        decode_command_v1(&open, limits),
        Err(SemanticWireDecodeErrorV1::InvalidUtf8)
    );

    let mut submit = encode_command_v1(
        &SemanticSessionCommandV1::Submit {
            exchange: exchange(),
            operation: operation(),
            incarnation: incarnation(),
            membership: membership(),
            request: request(SemanticQueryV1::Diagnostics),
        },
        limits,
    )
    .unwrap();
    let request = field_payload(&submit, 10);
    submit[request.start + 41..request.start + 49].fill(0);
    assert!(decode_command_v1(&submit, limits).is_err());

    let original = encode_command_v1(&commands()[1], limits).unwrap();
    let first_end = field_payload(&original, 1).end;
    let second_end = field_payload(&original, 2).end;
    let mut reordered = original[..7].to_vec();
    reordered.extend_from_slice(&original[first_end..second_end]);
    reordered.extend_from_slice(&original[7..first_end]);
    reordered.extend_from_slice(&original[second_end..]);
    assert_eq!(
        decode_command_v1(&reordered, limits),
        Err(SemanticWireDecodeErrorV1::DuplicateOrReorderedField)
    );
}

#[test]
fn reference_limits_are_internally_consistent() {
    assert!(SemanticSessionLimitsV1::reference().is_valid());
    assert!(
        SemanticLimitsV1::reference().max_label_bytes
            >= SemanticSessionLimitsV1::reference().max_language_bytes
    );
}

fn location() -> SemanticLocationV1 {
    SemanticLocationV1 {
        target: target(),
        span: SemanticSpanV1::try_new(1, 2).unwrap(),
    }
}

fn semantic_reply(result: SemanticResultV1) -> SemanticReplyV1 {
    SemanticReplyV1 {
        request: request_ref(),
        target: target(),
        index_generation: SemanticIndexGenerationV1::new(10).unwrap(),
        provider: SemanticProviderProvenanceV1::try_new(
            SemanticProviderRefV1::new([11; 16]),
            "rust-analyzer".into(),
            "1.0".into(),
        )
        .unwrap(),
        result,
    }
}

#[test]
fn nested_results_and_all_refusal_tags_roundtrip() {
    let limits = SemanticWireLimitsV1::reference();
    let semantic = limits.session.semantic;
    let text = || SemanticTextV1::try_new("x".into(), semantic.max_label_bytes).unwrap();
    let detail = || SemanticTextV1::try_new("detail".into(), semantic.max_detail_bytes).unwrap();
    let results = vec![
        SemanticResultV1::try_symbols(
            vec![SemanticSymbolV1 {
                name: text(),
                kind: SemanticSymbolKindV1::Method,
                location: location(),
            }],
            semantic,
        )
        .unwrap(),
        SemanticResultV1::try_locations(vec![location()], semantic).unwrap(),
        SemanticResultV1::try_diagnostics(
            vec![SemanticDiagnosticV1 {
                severity: SemanticDiagnosticSeverityV1::Warning,
                location: location(),
                message: detail(),
                code: Some(text()),
            }],
            semantic,
        )
        .unwrap(),
        SemanticResultV1::try_type_at_point(detail(), semantic).unwrap(),
        SemanticResultV1::try_rename_plan(
            vec![SemanticTextEditV1 {
                location: location(),
                replacement: text(),
            }],
            semantic,
        )
        .unwrap(),
    ];
    for result in results {
        let value = reply(
            SemanticSessionCommandKindV1::Collect,
            SemanticSessionOutcomeV1::Complete(semantic_reply(result)),
        );
        let bytes = encode_reply_v1(&value, limits).unwrap();
        assert_eq!(decode_reply_v1(&bytes, limits).unwrap(), value);
    }
    let peer = [
        SemanticRefusalV1::AuthorityDenied,
        SemanticRefusalV1::InputLimit,
        SemanticRefusalV1::OutputLimit,
        SemanticRefusalV1::InvalidRequest,
        SemanticRefusalV1::MalformedProviderReply,
    ];
    for reason in peer {
        let value = reply(
            SemanticSessionCommandKindV1::Collect,
            SemanticSessionOutcomeV1::PeerRefused(reason),
        );
        assert_eq!(
            decode_reply_v1(&encode_reply_v1(&value, limits).unwrap(), limits).unwrap(),
            value
        );
    }
    let session = [
        (
            SemanticSessionCommandKindV1::Submit,
            SemanticSessionRefusalV1::InvalidSession,
        ),
        (
            SemanticSessionCommandKindV1::Submit,
            SemanticSessionRefusalV1::UnknownMembership,
        ),
        (
            SemanticSessionCommandKindV1::Submit,
            SemanticSessionRefusalV1::RoleDenied,
        ),
        (
            SemanticSessionCommandKindV1::Submit,
            SemanticSessionRefusalV1::InputLimit,
        ),
        (
            SemanticSessionCommandKindV1::Submit,
            SemanticSessionRefusalV1::Capacity,
        ),
        (
            SemanticSessionCommandKindV1::Submit,
            SemanticSessionRefusalV1::SynchronizationRequired,
        ),
        (
            SemanticSessionCommandKindV1::BeginSynchronization,
            SemanticSessionRefusalV1::SynchronizationConflict,
        ),
        (
            SemanticSessionCommandKindV1::Submit,
            SemanticSessionRefusalV1::ProviderRefused,
        ),
        (
            SemanticSessionCommandKindV1::Submit,
            SemanticSessionRefusalV1::ClientRetiring,
        ),
        (
            SemanticSessionCommandKindV1::Collect,
            SemanticSessionRefusalV1::AbsentOrForeign,
        ),
        (
            SemanticSessionCommandKindV1::Submit,
            SemanticSessionRefusalV1::ProtocolViolation,
        ),
    ];
    for (kind, reason) in session {
        let value = reply(kind, SemanticSessionOutcomeV1::Refused(reason));
        assert_eq!(
            decode_reply_v1(&encode_reply_v1(&value, limits).unwrap(), limits).unwrap(),
            value
        );
    }
    let kinds = [
        SemanticQueryKindV1::Symbols,
        SemanticQueryKindV1::Definition,
        SemanticQueryKindV1::References,
        SemanticQueryKindV1::Implementations,
        SemanticQueryKindV1::IncomingCalls,
        SemanticQueryKindV1::OutgoingCalls,
        SemanticQueryKindV1::Diagnostics,
        SemanticQueryKindV1::TypeAtPoint,
        SemanticQueryKindV1::PlanRename,
    ];
    for kind in kinds {
        let value = reply(
            SemanticSessionCommandKindV1::Submit,
            SemanticSessionOutcomeV1::Unsupported(kind),
        );
        assert_eq!(
            decode_reply_v1(&encode_reply_v1(&value, limits).unwrap(), limits).unwrap(),
            value
        );
    }
}

fn decode_hex(value: &str) -> Vec<u8> {
    value
        .split_ascii_whitespace()
        .map(|byte| u8::from_str_radix(byte, 16).unwrap())
        .collect()
}

#[test]
fn frozen_fixtures_are_canonical_release_tripwires() {
    let limits = SemanticWireLimitsV1::reference();
    let command = decode_hex(include_str!("fixtures/describe-command-v1.hex"));
    let decoded = decode_command_v1(&command, limits).unwrap();
    assert_eq!(
        decoded,
        SemanticSessionCommandV1::Describe {
            exchange: exchange()
        }
    );
    assert_eq!(encode_command_v1(&decoded, limits).unwrap(), command);

    let reply_bytes = decode_hex(include_str!("fixtures/describe-unavailable-reply-v1.hex"));
    let decoded = decode_reply_v1(&reply_bytes, limits).unwrap();
    assert_eq!(
        decoded,
        reply(
            SemanticSessionCommandKindV1::Describe,
            SemanticSessionOutcomeV1::Unavailable
        )
    );
    assert_eq!(encode_reply_v1(&decoded, limits).unwrap(), reply_bytes);
}

#[test]
fn reply_mutations_and_invalid_products_are_rejected() {
    let limits = SemanticWireLimitsV1::reference();
    let valid = reply(
        SemanticSessionCommandKindV1::Collect,
        SemanticSessionOutcomeV1::TimedOut,
    );
    let canonical = encode_reply_v1(&valid, limits).unwrap();
    let mut unknown_outcome = canonical.clone();
    *unknown_outcome.last_mut().unwrap() = 255;
    assert!(decode_reply_v1(&unknown_outcome, limits).is_err());
    assert!(decode_reply_v1(&canonical[..canonical.len() - 1], limits).is_err());
    let mut unknown_field = canonical.clone();
    unknown_field[6] += 1;
    unknown_field.extend_from_slice(&[13, 0, 0, 0, 0]);
    assert!(decode_reply_v1(&unknown_field, limits).is_err());
    let mut wrong_class = canonical.clone();
    wrong_class[5] = 128;
    assert!(decode_reply_v1(&wrong_class, limits).is_err());
    let mut zero_member = canonical.clone();
    let member_start = 7 + (5 + 16) + (5 + 16) + 5;
    zero_member[member_start..member_start + 16].fill(0);
    assert!(decode_reply_v1(&zero_member, limits).is_err());
    assert!(
        SemanticSessionReplyV1::try_new(
            SemanticSessionCommandKindV1::Cancel,
            exchange(),
            incarnation(),
            Some(membership()),
            Some(operation()),
            SemanticSessionOutcomeV1::Cancelled,
        )
        .is_none()
    );
}
