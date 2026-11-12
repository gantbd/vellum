//! Canonical bounded wire encoding for Vellum semantic sessions.
#![forbid(unsafe_code)]

use keystone_codec::encode_bytes;
use std::collections::BTreeMap;
use vellum_host::semantic::{
    SemanticBuildErrorV1, SemanticCapabilitySetV1, SemanticCapabilityV1,
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

const MAGIC: &[u8; 4] = b"VSS1";
const VERSION: u8 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticWireLimitsV1 {
    pub max_frame_bytes: usize,
    pub max_field_bytes: usize,
    pub max_fields: usize,
    pub session: SemanticSessionLimitsV1,
}

impl SemanticWireLimitsV1 {
    pub const fn reference() -> Self {
        Self {
            max_frame_bytes: 16 * 1024 * 1024,
            max_field_bytes: 8 * 1024 * 1024,
            max_fields: 16,
            session: SemanticSessionLimitsV1::reference(),
        }
    }
    pub const fn is_valid(self) -> bool {
        self.max_frame_bytes > 0
            && self.max_frame_bytes <= 16 * 1024 * 1024
            && self.max_field_bytes > 0
            && self.max_field_bytes <= 8 * 1024 * 1024
            && self.max_field_bytes <= self.max_frame_bytes
            && self.max_fields > 0
            && self.max_fields <= 16
            && self.session.is_valid()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticWireEncodeErrorV1 {
    InvalidLimits,
    Limit,
    InvalidValue,
    Capacity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticWireDecodeErrorV1 {
    InvalidLimits,
    Limit,
    BadMagic,
    UnsupportedVersion,
    UnknownTag,
    DuplicateOrReorderedField,
    MissingField,
    Truncated,
    TrailingData,
    InvalidUtf8,
    InvalidIdentity,
    Malformed,
    Capacity,
}

#[derive(Clone)]
struct Field {
    tag: u8,
    bytes: Vec<u8>,
}
fn field(tag: u8, bytes: Vec<u8>) -> Field {
    Field { tag, bytes }
}

fn encode_envelope(
    kind: u8,
    fields: Vec<Field>,
    limits: SemanticWireLimitsV1,
) -> Result<Vec<u8>, SemanticWireEncodeErrorV1> {
    if !limits.is_valid() {
        return Err(SemanticWireEncodeErrorV1::InvalidLimits);
    }
    if fields.len() > limits.max_fields || fields.len() > u8::MAX as usize {
        return Err(SemanticWireEncodeErrorV1::Limit);
    }
    let mut previous = 0;
    let mut length = 7usize;
    for item in &fields {
        if item.tag <= previous {
            return Err(SemanticWireEncodeErrorV1::InvalidValue);
        }
        if item.bytes.len() > limits.max_field_bytes {
            return Err(SemanticWireEncodeErrorV1::Limit);
        }
        previous = item.tag;
        length = length
            .checked_add(5)
            .and_then(|v| v.checked_add(item.bytes.len()))
            .ok_or(SemanticWireEncodeErrorV1::Limit)?;
    }
    if length > limits.max_frame_bytes {
        return Err(SemanticWireEncodeErrorV1::Limit);
    }
    let mut out = Vec::new();
    out.try_reserve_exact(length)
        .map_err(|_| SemanticWireEncodeErrorV1::Capacity)?;
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.push(kind);
    out.push(fields.len() as u8);
    for item in fields {
        out.push(item.tag);
        out.extend_from_slice(&(item.bytes.len() as u32).to_be_bytes());
        out.extend_from_slice(&item.bytes);
    }
    Ok(out)
}

fn decode_envelope(
    input: &[u8],
    limits: SemanticWireLimitsV1,
) -> Result<(u8, BTreeMap<u8, Vec<u8>>), SemanticWireDecodeErrorV1> {
    if !limits.is_valid() {
        return Err(SemanticWireDecodeErrorV1::InvalidLimits);
    }
    if input.len() > limits.max_frame_bytes {
        return Err(SemanticWireDecodeErrorV1::Limit);
    }
    if input.len() < 7 {
        return Err(SemanticWireDecodeErrorV1::Truncated);
    }
    if &input[..4] != MAGIC {
        return Err(SemanticWireDecodeErrorV1::BadMagic);
    }
    if input[4] != VERSION {
        return Err(SemanticWireDecodeErrorV1::UnsupportedVersion);
    }
    let kind = input[5];
    let count = input[6] as usize;
    if count > limits.max_fields {
        return Err(SemanticWireDecodeErrorV1::Limit);
    }
    let mut cursor = 7usize;
    let mut previous = 0u8;
    let mut fields = BTreeMap::new();
    for _ in 0..count {
        let tag = *input
            .get(cursor)
            .ok_or(SemanticWireDecodeErrorV1::Truncated)?;
        cursor += 1;
        if tag <= previous {
            return Err(SemanticWireDecodeErrorV1::DuplicateOrReorderedField);
        }
        previous = tag;
        let raw: [u8; 4] = input
            .get(cursor..cursor + 4)
            .ok_or(SemanticWireDecodeErrorV1::Truncated)?
            .try_into()
            .map_err(|_| SemanticWireDecodeErrorV1::Truncated)?;
        cursor += 4;
        let length = u32::from_be_bytes(raw) as usize;
        if length > limits.max_field_bytes {
            return Err(SemanticWireDecodeErrorV1::Limit);
        }
        let value = input
            .get(cursor..cursor + length)
            .ok_or(SemanticWireDecodeErrorV1::Truncated)?;
        cursor += length;
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(length)
            .map_err(|_| SemanticWireDecodeErrorV1::Capacity)?;
        owned.extend_from_slice(value);
        fields.insert(tag, owned);
    }
    if cursor != input.len() {
        return Err(SemanticWireDecodeErrorV1::TrailingData);
    }
    Ok((kind, fields))
}

fn required(fields: &BTreeMap<u8, Vec<u8>>, tag: u8) -> Result<&[u8], SemanticWireDecodeErrorV1> {
    fields
        .get(&tag)
        .map(Vec::as_slice)
        .ok_or(SemanticWireDecodeErrorV1::MissingField)
}
fn exact(fields: &BTreeMap<u8, Vec<u8>>, tags: &[u8]) -> Result<(), SemanticWireDecodeErrorV1> {
    if fields.len() == tags.len() && fields.keys().copied().eq(tags.iter().copied()) {
        Ok(())
    } else {
        Err(SemanticWireDecodeErrorV1::UnknownTag)
    }
}
fn fixed16(bytes: &[u8]) -> Result<[u8; 16], SemanticWireDecodeErrorV1> {
    bytes
        .try_into()
        .map_err(|_| SemanticWireDecodeErrorV1::InvalidIdentity)
}
fn exchange(bytes: &[u8]) -> Result<SemanticExchangeRefV1, SemanticWireDecodeErrorV1> {
    SemanticExchangeRefV1::new(fixed16(bytes)?).ok_or(SemanticWireDecodeErrorV1::InvalidIdentity)
}
fn operation(bytes: &[u8]) -> Result<SemanticSessionOperationRefV1, SemanticWireDecodeErrorV1> {
    SemanticSessionOperationRefV1::new(fixed16(bytes)?)
        .ok_or(SemanticWireDecodeErrorV1::InvalidIdentity)
}
fn incarnation(bytes: &[u8]) -> Result<SemanticSessionIncarnationV1, SemanticWireDecodeErrorV1> {
    SemanticSessionIncarnationV1::new(fixed16(bytes)?)
        .ok_or(SemanticWireDecodeErrorV1::InvalidIdentity)
}
fn membership(bytes: &[u8]) -> Result<SemanticMembershipRefV1, SemanticWireDecodeErrorV1> {
    SemanticMembershipRefV1::new(fixed16(bytes)?).ok_or(SemanticWireDecodeErrorV1::InvalidIdentity)
}
fn epoch(bytes: &[u8]) -> Result<SemanticSynchronizationEpochV1, SemanticWireDecodeErrorV1> {
    SemanticSynchronizationEpochV1::new(fixed16(bytes)?)
        .ok_or(SemanticWireDecodeErrorV1::InvalidIdentity)
}
fn request_ref(bytes: &[u8]) -> Result<SemanticRequestRefV1, SemanticWireDecodeErrorV1> {
    Ok(SemanticRequestRefV1::new(fixed16(bytes)?))
}
fn one(bytes: &[u8]) -> Result<u8, SemanticWireDecodeErrorV1> {
    if bytes.len() == 1 {
        Ok(bytes[0])
    } else {
        Err(SemanticWireDecodeErrorV1::Malformed)
    }
}

pub fn encode_command_v1(
    value: &SemanticSessionCommandV1,
    limits: SemanticWireLimitsV1,
) -> Result<Vec<u8>, SemanticWireEncodeErrorV1> {
    use SemanticSessionCommandV1 as C;
    let exchange = field(1, value.exchange().as_bytes().to_vec());
    let (kind, fields) = match value {
        C::Describe { .. } => (1, vec![exchange]),
        C::JoinSession {
            operation,
            incarnation,
            requested_roles,
            ..
        } => (
            2,
            vec![
                exchange,
                field(2, operation.as_bytes().to_vec()),
                field(3, incarnation.as_bytes().to_vec()),
                field(5, vec![requested_roles.bits()]),
            ],
        ),
        C::BeginSynchronization {
            operation,
            incarnation,
            membership,
            epoch,
            manifest,
            ..
        } => (
            3,
            bound_mutation(
                exchange,
                *operation,
                *incarnation,
                *membership,
                vec![
                    field(6, epoch.as_bytes().to_vec()),
                    field(7, encode_manifest(*manifest)),
                ],
            ),
        ),
        C::SyncOpen {
            operation,
            incarnation,
            membership,
            epoch,
            snapshot,
            ..
        } => (
            4,
            bound_mutation(
                exchange,
                *operation,
                *incarnation,
                *membership,
                vec![
                    field(6, epoch.as_bytes().to_vec()),
                    field(8, encode_snapshot(snapshot, limits)?),
                ],
            ),
        ),
        C::SyncChange {
            operation,
            incarnation,
            membership,
            epoch,
            snapshot,
            ..
        } => (
            5,
            bound_mutation(
                exchange,
                *operation,
                *incarnation,
                *membership,
                vec![
                    field(6, epoch.as_bytes().to_vec()),
                    field(8, encode_snapshot(snapshot, limits)?),
                ],
            ),
        ),
        C::SyncClose {
            operation,
            incarnation,
            membership,
            epoch,
            target,
            ..
        } => (
            6,
            bound_mutation(
                exchange,
                *operation,
                *incarnation,
                *membership,
                vec![
                    field(6, epoch.as_bytes().to_vec()),
                    field(9, encode_target(*target)),
                ],
            ),
        ),
        C::CompleteSynchronization {
            operation,
            incarnation,
            membership,
            epoch,
            manifest,
            ..
        } => (
            7,
            bound_mutation(
                exchange,
                *operation,
                *incarnation,
                *membership,
                vec![
                    field(6, epoch.as_bytes().to_vec()),
                    field(7, encode_manifest(*manifest)),
                ],
            ),
        ),
        C::Submit {
            operation,
            incarnation,
            membership,
            request,
            ..
        } => (
            8,
            bound_mutation(
                exchange,
                *operation,
                *incarnation,
                *membership,
                vec![field(10, encode_request(request, limits)?)],
            ),
        ),
        C::Cancel {
            operation,
            incarnation,
            membership,
            request,
            ..
        } => (
            9,
            bound_mutation(
                exchange,
                *operation,
                *incarnation,
                *membership,
                vec![field(11, request.as_bytes().to_vec())],
            ),
        ),
        C::Collect {
            incarnation,
            membership,
            request,
            ..
        } => (
            10,
            vec![
                exchange,
                field(3, incarnation.as_bytes().to_vec()),
                field(4, membership.as_bytes().to_vec()),
                field(11, request.as_bytes().to_vec()),
            ],
        ),
        C::ReleaseRequest {
            operation,
            incarnation,
            membership,
            request,
            ..
        } => (
            11,
            bound_mutation(
                exchange,
                *operation,
                *incarnation,
                *membership,
                vec![field(11, request.as_bytes().to_vec())],
            ),
        ),
        C::LeaveSession {
            operation,
            incarnation,
            membership,
            ..
        } => (
            12,
            bound_mutation(exchange, *operation, *incarnation, *membership, vec![]),
        ),
    };
    encode_envelope(kind, fields, limits)
}

fn bound_mutation(
    exchange: Field,
    operation: SemanticSessionOperationRefV1,
    incarnation: SemanticSessionIncarnationV1,
    membership: SemanticMembershipRefV1,
    mut rest: Vec<Field>,
) -> Vec<Field> {
    let mut fields = vec![
        exchange,
        field(2, operation.as_bytes().to_vec()),
        field(3, incarnation.as_bytes().to_vec()),
        field(4, membership.as_bytes().to_vec()),
    ];
    fields.append(&mut rest);
    fields
}

pub fn decode_command_v1(
    input: &[u8],
    limits: SemanticWireLimitsV1,
) -> Result<SemanticSessionCommandV1, SemanticWireDecodeErrorV1> {
    use SemanticSessionCommandV1 as C;
    let (kind, fields) = decode_envelope(input, limits)?;
    let exchange = exchange(required(&fields, 1)?)?;
    Ok(match kind {
        1 => {
            exact(&fields, &[1])?;
            C::Describe { exchange }
        }
        2 => {
            exact(&fields, &[1, 2, 3, 5])?;
            C::JoinSession {
                exchange,
                operation: operation(required(&fields, 2)?)?,
                incarnation: incarnation(required(&fields, 3)?)?,
                requested_roles: decode_roles(required(&fields, 5)?)?,
            }
        }
        3 => {
            exact(&fields, &[1, 2, 3, 4, 6, 7])?;
            C::BeginSynchronization {
                exchange,
                operation: operation(required(&fields, 2)?)?,
                incarnation: incarnation(required(&fields, 3)?)?,
                membership: membership(required(&fields, 4)?)?,
                epoch: epoch(required(&fields, 6)?)?,
                manifest: decode_manifest(required(&fields, 7)?, limits)?,
            }
        }
        4 | 5 => {
            exact(&fields, &[1, 2, 3, 4, 6, 8])?;
            let operation = operation(required(&fields, 2)?)?;
            let incarnation = incarnation(required(&fields, 3)?)?;
            let membership = membership(required(&fields, 4)?)?;
            let epoch = epoch(required(&fields, 6)?)?;
            let snapshot = decode_snapshot(required(&fields, 8)?, limits)?;
            if kind == 4 {
                C::SyncOpen {
                    exchange,
                    operation,
                    incarnation,
                    membership,
                    epoch,
                    snapshot,
                }
            } else {
                C::SyncChange {
                    exchange,
                    operation,
                    incarnation,
                    membership,
                    epoch,
                    snapshot,
                }
            }
        }
        6 => {
            exact(&fields, &[1, 2, 3, 4, 6, 9])?;
            C::SyncClose {
                exchange,
                operation: operation(required(&fields, 2)?)?,
                incarnation: incarnation(required(&fields, 3)?)?,
                membership: membership(required(&fields, 4)?)?,
                epoch: epoch(required(&fields, 6)?)?,
                target: decode_target(required(&fields, 9)?)?,
            }
        }
        7 => {
            exact(&fields, &[1, 2, 3, 4, 6, 7])?;
            C::CompleteSynchronization {
                exchange,
                operation: operation(required(&fields, 2)?)?,
                incarnation: incarnation(required(&fields, 3)?)?,
                membership: membership(required(&fields, 4)?)?,
                epoch: epoch(required(&fields, 6)?)?,
                manifest: decode_manifest(required(&fields, 7)?, limits)?,
            }
        }
        8 => {
            exact(&fields, &[1, 2, 3, 4, 10])?;
            C::Submit {
                exchange,
                operation: operation(required(&fields, 2)?)?,
                incarnation: incarnation(required(&fields, 3)?)?,
                membership: membership(required(&fields, 4)?)?,
                request: decode_request(required(&fields, 10)?, limits)?,
            }
        }
        9 | 11 => {
            exact(&fields, &[1, 2, 3, 4, 11])?;
            let operation = operation(required(&fields, 2)?)?;
            let incarnation = incarnation(required(&fields, 3)?)?;
            let membership = membership(required(&fields, 4)?)?;
            let request = request_ref(required(&fields, 11)?)?;
            if kind == 9 {
                C::Cancel {
                    exchange,
                    operation,
                    incarnation,
                    membership,
                    request,
                }
            } else {
                C::ReleaseRequest {
                    exchange,
                    operation,
                    incarnation,
                    membership,
                    request,
                }
            }
        }
        10 => {
            exact(&fields, &[1, 3, 4, 11])?;
            C::Collect {
                exchange,
                incarnation: incarnation(required(&fields, 3)?)?,
                membership: membership(required(&fields, 4)?)?,
                request: request_ref(required(&fields, 11)?)?,
            }
        }
        12 => {
            exact(&fields, &[1, 2, 3, 4])?;
            C::LeaveSession {
                exchange,
                operation: operation(required(&fields, 2)?)?,
                incarnation: incarnation(required(&fields, 3)?)?,
                membership: membership(required(&fields, 4)?)?,
            }
        }
        _ => return Err(SemanticWireDecodeErrorV1::UnknownTag),
    })
}

fn decode_roles(bytes: &[u8]) -> Result<SemanticSessionRolesV1, SemanticWireDecodeErrorV1> {
    SemanticSessionRolesV1::from_bits(one(bytes)?).ok_or(SemanticWireDecodeErrorV1::Malformed)
}
fn encode_manifest(value: SemanticSynchronizationManifestV1) -> Vec<u8> {
    let mut out = Vec::with_capacity(44);
    out.extend_from_slice(&value.documents().to_be_bytes());
    out.extend_from_slice(&value.source_bytes().to_be_bytes());
    out.extend_from_slice(&value.digest());
    out
}
fn decode_manifest(
    bytes: &[u8],
    limits: SemanticWireLimitsV1,
) -> Result<SemanticSynchronizationManifestV1, SemanticWireDecodeErrorV1> {
    if bytes.len() != 44 {
        return Err(SemanticWireDecodeErrorV1::Malformed);
    };
    let documents = u32::from_be_bytes(bytes[..4].try_into().unwrap());
    let source_bytes = u64::from_be_bytes(bytes[4..12].try_into().unwrap());
    let digest = bytes[12..].try_into().unwrap();
    SemanticSynchronizationManifestV1::new(documents, source_bytes, digest, limits.session)
        .ok_or(SemanticWireDecodeErrorV1::Limit)
}
fn encode_target(value: SemanticTargetV1) -> Vec<u8> {
    let mut out = Vec::with_capacity(24);
    out.extend_from_slice(&value.document.as_bytes());
    out.extend_from_slice(&value.source_revision.get().to_be_bytes());
    out
}
fn decode_target(bytes: &[u8]) -> Result<SemanticTargetV1, SemanticWireDecodeErrorV1> {
    if bytes.len() != 24 {
        return Err(SemanticWireDecodeErrorV1::Malformed);
    };
    Ok(SemanticTargetV1 {
        document: SemanticDocumentRefV1::new(bytes[..16].try_into().unwrap()),
        source_revision: SemanticSourceRevisionV1::new(u64::from_be_bytes(
            bytes[16..].try_into().unwrap(),
        )),
    })
}

fn put_text(out: &mut Vec<u8>, value: &str) -> Result<(), SemanticWireEncodeErrorV1> {
    encode_bytes(out, value.as_bytes()).map_err(|_| SemanticWireEncodeErrorV1::Limit)
}
fn encode_snapshot(
    value: &SemanticSessionSnapshotV1,
    limits: SemanticWireLimitsV1,
) -> Result<Vec<u8>, SemanticWireEncodeErrorV1> {
    if value.source().len() > limits.session.max_snapshot_bytes
        || value.language().len() > limits.session.max_language_bytes
    {
        return Err(SemanticWireEncodeErrorV1::Limit);
    }
    let mut out = encode_target(value.target());
    put_text(&mut out, value.language())?;
    put_text(&mut out, value.source())?;
    Ok(out)
}
fn decode_snapshot(
    bytes: &[u8],
    limits: SemanticWireLimitsV1,
) -> Result<SemanticSessionSnapshotV1, SemanticWireDecodeErrorV1> {
    let mut c = Cursor::new(bytes);
    let target = decode_target(c.take(24)?)?;
    let language = c.text(limits.session.max_language_bytes)?;
    let source = c.text(limits.session.max_snapshot_bytes)?;
    c.finish()?;
    SemanticSessionSnapshotV1::try_new(target, language, source, limits.session)
        .ok_or(SemanticWireDecodeErrorV1::Limit)
}

fn encode_request(
    value: &SemanticRequestV1,
    limits: SemanticWireLimitsV1,
) -> Result<Vec<u8>, SemanticWireEncodeErrorV1> {
    if !value.is_within_limits(limits.session.semantic) {
        return Err(SemanticWireEncodeErrorV1::Limit);
    }
    let mut out = Vec::new();
    out.extend_from_slice(&value.request.as_bytes());
    out.extend_from_slice(&encode_target(value.target));
    match value.expected_index {
        Some(index) => {
            out.push(1);
            out.extend_from_slice(&index.get().to_be_bytes())
        }
        None => out.push(0),
    }
    put_text(&mut out, value.language.as_str())?;
    encode_query(&mut out, &value.query)?;
    Ok(out)
}
fn decode_request(
    bytes: &[u8],
    limits: SemanticWireLimitsV1,
) -> Result<SemanticRequestV1, SemanticWireDecodeErrorV1> {
    let mut c = Cursor::new(bytes);
    let request = SemanticRequestRefV1::new(c.array16()?);
    let target = decode_target(c.take(24)?)?;
    let expected_index = match c.u8()? {
        0 => None,
        1 => Some(
            SemanticIndexGenerationV1::new(c.u64()?).ok_or(SemanticWireDecodeErrorV1::Malformed)?,
        ),
        _ => return Err(SemanticWireDecodeErrorV1::Malformed),
    };
    let language = SemanticTextV1::try_new(
        c.text(limits.session.semantic.max_label_bytes)?,
        limits.session.semantic.max_label_bytes,
    )
    .map_err(map_build)?;
    let query = decode_query(&mut c, limits)?;
    c.finish()?;
    let request = SemanticRequestV1 {
        request,
        target,
        expected_index,
        language,
        query,
    };
    if request.is_within_limits(limits.session.semantic) {
        Ok(request)
    } else {
        Err(SemanticWireDecodeErrorV1::Limit)
    }
}
fn encode_query(
    out: &mut Vec<u8>,
    query: &SemanticQueryV1,
) -> Result<(), SemanticWireEncodeErrorV1> {
    match query {
        SemanticQueryV1::Symbols { needle } => {
            out.push(1);
            put_text(out, needle.as_str())?
        }
        SemanticQueryV1::Definition { offset } => {
            out.push(2);
            out.extend_from_slice(&offset.to_be_bytes())
        }
        SemanticQueryV1::References { offset } => {
            out.push(3);
            out.extend_from_slice(&offset.to_be_bytes())
        }
        SemanticQueryV1::Implementations { offset } => {
            out.push(4);
            out.extend_from_slice(&offset.to_be_bytes())
        }
        SemanticQueryV1::Diagnostics => out.push(5),
        SemanticQueryV1::TypeAtPoint { offset } => {
            out.push(6);
            out.extend_from_slice(&offset.to_be_bytes())
        }
        SemanticQueryV1::PlanRename { offset, new_name } => {
            out.push(7);
            out.extend_from_slice(&offset.to_be_bytes());
            put_text(out, new_name.as_str())?
        }
        SemanticQueryV1::IncomingCalls { offset } => {
            out.push(8);
            out.extend_from_slice(&offset.to_be_bytes())
        }
        SemanticQueryV1::OutgoingCalls { offset } => {
            out.push(9);
            out.extend_from_slice(&offset.to_be_bytes())
        }
    }
    Ok(())
}
fn decode_query(
    c: &mut Cursor<'_>,
    limits: SemanticWireLimitsV1,
) -> Result<SemanticQueryV1, SemanticWireDecodeErrorV1> {
    Ok(match c.u8()? {
        1 => SemanticQueryV1::Symbols {
            needle: SemanticTextV1::try_new(
                c.text(limits.session.semantic.max_query_bytes)?,
                limits.session.semantic.max_query_bytes,
            )
            .map_err(map_build)?,
        },
        2 => SemanticQueryV1::Definition { offset: c.u64()? },
        3 => SemanticQueryV1::References { offset: c.u64()? },
        4 => SemanticQueryV1::Implementations { offset: c.u64()? },
        5 => SemanticQueryV1::Diagnostics,
        6 => SemanticQueryV1::TypeAtPoint { offset: c.u64()? },
        7 => SemanticQueryV1::PlanRename {
            offset: c.u64()?,
            new_name: SemanticTextV1::try_new(
                c.text(limits.session.semantic.max_label_bytes)?,
                limits.session.semantic.max_label_bytes,
            )
            .map_err(map_build)?,
        },
        8 => SemanticQueryV1::IncomingCalls { offset: c.u64()? },
        9 => SemanticQueryV1::OutgoingCalls { offset: c.u64()? },
        _ => return Err(SemanticWireDecodeErrorV1::UnknownTag),
    })
}

fn map_build(_: SemanticBuildErrorV1) -> SemanticWireDecodeErrorV1 {
    SemanticWireDecodeErrorV1::Malformed
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}
impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }
    fn take(&mut self, length: usize) -> Result<&'a [u8], SemanticWireDecodeErrorV1> {
        let end = self
            .position
            .checked_add(length)
            .ok_or(SemanticWireDecodeErrorV1::Limit)?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(SemanticWireDecodeErrorV1::Truncated)?;
        self.position = end;
        Ok(value)
    }
    fn u8(&mut self) -> Result<u8, SemanticWireDecodeErrorV1> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, SemanticWireDecodeErrorV1> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, SemanticWireDecodeErrorV1> {
        Ok(u64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn array16(&mut self) -> Result<[u8; 16], SemanticWireDecodeErrorV1> {
        Ok(self.take(16)?.try_into().unwrap())
    }
    fn text(&mut self, max: usize) -> Result<String, SemanticWireDecodeErrorV1> {
        let length = self.u32()? as usize;
        if length > max {
            return Err(SemanticWireDecodeErrorV1::Limit);
        }
        let bytes = self.take(length)?;
        let value =
            std::str::from_utf8(bytes).map_err(|_| SemanticWireDecodeErrorV1::InvalidUtf8)?;
        let mut text = String::new();
        text.try_reserve_exact(length)
            .map_err(|_| SemanticWireDecodeErrorV1::Capacity)?;
        text.push_str(value);
        Ok(text)
    }
    fn count(&mut self, max: usize) -> Result<usize, SemanticWireDecodeErrorV1> {
        let value = self.u32()? as usize;
        if value > max {
            Err(SemanticWireDecodeErrorV1::Limit)
        } else {
            Ok(value)
        }
    }
    fn finish(&self) -> Result<(), SemanticWireDecodeErrorV1> {
        if self.position == self.bytes.len() {
            Ok(())
        } else {
            Err(SemanticWireDecodeErrorV1::TrailingData)
        }
    }
}

pub fn encode_reply_v1(
    value: &SemanticSessionReplyV1,
    limits: SemanticWireLimitsV1,
) -> Result<Vec<u8>, SemanticWireEncodeErrorV1> {
    let mut fields = vec![field(1, value.exchange().as_bytes().to_vec())];
    if let Some(operation) = value.operation() {
        fields.push(field(2, operation.as_bytes().to_vec()))
    }
    fields.push(field(3, value.incarnation().as_bytes().to_vec()));
    if let Some(membership) = value.membership() {
        fields.push(field(4, membership.as_bytes().to_vec()))
    }
    fields.push(field(12, encode_outcome(value.outcome(), limits)?));
    encode_envelope(128 + value.kind() as u8, fields, limits)
}

pub fn decode_reply_v1(
    input: &[u8],
    limits: SemanticWireLimitsV1,
) -> Result<SemanticSessionReplyV1, SemanticWireDecodeErrorV1> {
    let (kind, fields) = decode_envelope(input, limits)?;
    let raw_kind = kind
        .checked_sub(128)
        .ok_or(SemanticWireDecodeErrorV1::UnknownTag)?;
    let kind = decode_command_kind(raw_kind)?;
    let expected = match kind {
        SemanticSessionCommandKindV1::Describe => vec![1, 3, 12],
        SemanticSessionCommandKindV1::JoinSession => {
            if fields.contains_key(&4) {
                vec![1, 2, 3, 4, 12]
            } else {
                vec![1, 2, 3, 12]
            }
        }
        SemanticSessionCommandKindV1::Collect => vec![1, 3, 4, 12],
        _ => vec![1, 2, 3, 4, 12],
    };
    exact(&fields, &expected)?;
    let exchange = exchange(required(&fields, 1)?)?;
    let operation = fields.get(&2).map(|bytes| operation(bytes)).transpose()?;
    let incarnation = incarnation(required(&fields, 3)?)?;
    let membership = fields.get(&4).map(|bytes| membership(bytes)).transpose()?;
    let outcome = decode_outcome(required(&fields, 12)?, limits)?;
    SemanticSessionReplyV1::try_new(kind, exchange, incarnation, membership, operation, outcome)
        .ok_or(SemanticWireDecodeErrorV1::Malformed)
}

fn encode_outcome(
    value: &SemanticSessionOutcomeV1,
    limits: SemanticWireLimitsV1,
) -> Result<Vec<u8>, SemanticWireEncodeErrorV1> {
    if !outcome_within_wire_limits(value, limits) {
        return Err(SemanticWireEncodeErrorV1::Limit);
    }
    let mut out = Vec::new();
    match value {
        SemanticSessionOutcomeV1::Description(value) => {
            out.push(1);
            encode_description(&mut out, *value, limits)?
        }
        SemanticSessionOutcomeV1::Joined {
            membership,
            granted_roles,
            remaining_operations,
        } => {
            out.push(2);
            out.extend_from_slice(&membership.as_bytes());
            out.push(granted_roles.bits());
            out.extend_from_slice(&remaining_operations.to_be_bytes())
        }
        SemanticSessionOutcomeV1::Accepted => out.push(3),
        SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Request(request)) => {
            out.push(4);
            out.extend_from_slice(&request.as_bytes())
        }
        SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Retirement {
            remaining_documents,
            quarantined_requests,
        }) => {
            out.push(5);
            out.extend_from_slice(&remaining_documents.to_be_bytes());
            out.extend_from_slice(&quarantined_requests.to_be_bytes())
        }
        SemanticSessionOutcomeV1::Complete(reply) => {
            out.push(6);
            encode_semantic_reply(&mut out, reply, limits)?
        }
        SemanticSessionOutcomeV1::Unsupported(kind) => {
            out.push(7);
            out.push(query_kind_tag(*kind))
        }
        SemanticSessionOutcomeV1::Stale => out.push(8),
        SemanticSessionOutcomeV1::Unavailable => out.push(9),
        SemanticSessionOutcomeV1::Cancelled => out.push(10),
        SemanticSessionOutcomeV1::TimedOut => out.push(11),
        SemanticSessionOutcomeV1::PeerRefused(reason) => {
            out.push(12);
            out.push(peer_refusal_tag(*reason))
        }
        SemanticSessionOutcomeV1::Refused(reason) => {
            out.push(13);
            out.push(session_refusal_tag(*reason))
        }
        SemanticSessionOutcomeV1::RequestConflict => out.push(14),
        SemanticSessionOutcomeV1::ReplayCapacity => out.push(15),
    }
    Ok(out)
}
fn decode_outcome(
    bytes: &[u8],
    limits: SemanticWireLimitsV1,
) -> Result<SemanticSessionOutcomeV1, SemanticWireDecodeErrorV1> {
    let mut c = Cursor::new(bytes);
    let value = match c.u8()? {
        1 => SemanticSessionOutcomeV1::Description(decode_description(&mut c, limits)?),
        2 => SemanticSessionOutcomeV1::Joined {
            membership: SemanticMembershipRefV1::new(c.array16()?)
                .ok_or(SemanticWireDecodeErrorV1::InvalidIdentity)?,
            granted_roles: SemanticSessionRolesV1::from_bits(c.u8()?)
                .ok_or(SemanticWireDecodeErrorV1::Malformed)?,
            remaining_operations: c.u32()?,
        },
        3 => SemanticSessionOutcomeV1::Accepted,
        4 => SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Request(
            SemanticRequestRefV1::new(c.array16()?),
        )),
        5 => SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Retirement {
            remaining_documents: c.u32()?,
            quarantined_requests: c.u32()?,
        }),
        6 => SemanticSessionOutcomeV1::Complete(decode_semantic_reply(&mut c, limits)?),
        7 => SemanticSessionOutcomeV1::Unsupported(decode_query_kind(c.u8()?)?),
        8 => SemanticSessionOutcomeV1::Stale,
        9 => SemanticSessionOutcomeV1::Unavailable,
        10 => SemanticSessionOutcomeV1::Cancelled,
        11 => SemanticSessionOutcomeV1::TimedOut,
        12 => SemanticSessionOutcomeV1::PeerRefused(decode_peer_refusal(c.u8()?)?),
        13 => SemanticSessionOutcomeV1::Refused(decode_session_refusal(c.u8()?)?),
        14 => SemanticSessionOutcomeV1::RequestConflict,
        15 => SemanticSessionOutcomeV1::ReplayCapacity,
        _ => return Err(SemanticWireDecodeErrorV1::UnknownTag),
    };
    c.finish()?;
    if !outcome_within_wire_limits(&value, limits) {
        return Err(SemanticWireDecodeErrorV1::Limit);
    }
    Ok(value)
}

fn outcome_within_wire_limits(
    value: &SemanticSessionOutcomeV1,
    wire: SemanticWireLimitsV1,
) -> bool {
    match value {
        SemanticSessionOutcomeV1::Description(value) => limits_narrow(value.limits(), wire.session),
        SemanticSessionOutcomeV1::Joined {
            remaining_operations,
            ..
        } => {
            *remaining_operations > 0
                && *remaining_operations <= wire.session.max_operations_per_membership
        }
        SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Retirement {
            remaining_documents,
            quarantined_requests,
        }) => {
            (*remaining_documents != 0 || *quarantined_requests != 0)
                && *remaining_documents <= wire.session.max_documents
                && *quarantined_requests <= wire.session.max_pending
        }
        SemanticSessionOutcomeV1::Complete(reply) => {
            reply.result.is_within_limits(wire.session.semantic)
        }
        _ => true,
    }
}

fn encode_description(
    out: &mut Vec<u8>,
    value: SemanticSessionDescriptionV1,
    wire: SemanticWireLimitsV1,
) -> Result<(), SemanticWireEncodeErrorV1> {
    if !limits_narrow(value.limits(), wire.session) {
        return Err(SemanticWireEncodeErrorV1::Limit);
    }
    out.extend_from_slice(&value.incarnation().as_bytes());
    out.push(value.role_ceiling().bits());
    match value.readiness() {
        SemanticSessionReadinessV1::Unavailable => out.push(0),
        SemanticSessionReadinessV1::Synchronizing { epoch } => {
            out.push(1);
            out.extend_from_slice(&epoch.as_bytes())
        }
        SemanticSessionReadinessV1::Ready { epoch } => {
            out.push(2);
            out.extend_from_slice(&epoch.as_bytes())
        }
    }
    match value.provider().readiness() {
        SemanticProviderReadinessProjectionV1::Unavailable => out.push(0),
        SemanticProviderReadinessProjectionV1::Indexing => out.push(1),
        SemanticProviderReadinessProjectionV1::Ready { generation } => {
            out.push(2);
            out.extend_from_slice(&generation.get().to_be_bytes())
        }
    }
    out.push(capability_bits(value.provider().capabilities()));
    encode_limits(out, value.limits());
    Ok(())
}
fn decode_description(
    c: &mut Cursor<'_>,
    wire: SemanticWireLimitsV1,
) -> Result<SemanticSessionDescriptionV1, SemanticWireDecodeErrorV1> {
    let incarnation = SemanticSessionIncarnationV1::new(c.array16()?)
        .ok_or(SemanticWireDecodeErrorV1::InvalidIdentity)?;
    let role_ceiling =
        SemanticSessionRolesV1::from_bits(c.u8()?).ok_or(SemanticWireDecodeErrorV1::Malformed)?;
    let readiness = match c.u8()? {
        0 => SemanticSessionReadinessV1::Unavailable,
        1 => SemanticSessionReadinessV1::Synchronizing {
            epoch: SemanticSynchronizationEpochV1::new(c.array16()?)
                .ok_or(SemanticWireDecodeErrorV1::InvalidIdentity)?,
        },
        2 => SemanticSessionReadinessV1::Ready {
            epoch: SemanticSynchronizationEpochV1::new(c.array16()?)
                .ok_or(SemanticWireDecodeErrorV1::InvalidIdentity)?,
        },
        _ => return Err(SemanticWireDecodeErrorV1::Malformed),
    };
    let provider_readiness = match c.u8()? {
        0 => SemanticProviderReadinessProjectionV1::Unavailable,
        1 => SemanticProviderReadinessProjectionV1::Indexing,
        2 => SemanticProviderReadinessProjectionV1::Ready {
            generation: SemanticIndexGenerationV1::new(c.u64()?)
                .ok_or(SemanticWireDecodeErrorV1::Malformed)?,
        },
        _ => return Err(SemanticWireDecodeErrorV1::Malformed),
    };
    let capabilities = decode_capabilities(c.u8()?)?;
    let limits = decode_limits(c)?;
    if !limits_narrow(limits, wire.session) {
        return Err(SemanticWireDecodeErrorV1::Limit);
    }
    SemanticSessionDescriptionV1::try_new(
        incarnation,
        role_ceiling,
        readiness,
        SemanticProviderObservationV1::new(provider_readiness, capabilities),
        limits,
    )
    .ok_or(SemanticWireDecodeErrorV1::Malformed)
}

fn encode_limits(out: &mut Vec<u8>, value: SemanticSessionLimitsV1) {
    out.extend_from_slice(&(value.max_snapshot_bytes as u64).to_be_bytes());
    out.extend_from_slice(&(value.max_total_snapshot_bytes as u64).to_be_bytes());
    out.extend_from_slice(&(value.max_language_bytes as u64).to_be_bytes());
    out.extend_from_slice(&value.max_documents.to_be_bytes());
    out.extend_from_slice(&value.max_pending.to_be_bytes());
    out.extend_from_slice(&value.max_memberships.to_be_bytes());
    out.extend_from_slice(&(value.max_replay_intent_bytes as u64).to_be_bytes());
    out.extend_from_slice(&(value.max_retained_result_bytes as u64).to_be_bytes());
    out.extend_from_slice(&value.max_requests_per_membership.to_be_bytes());
    out.extend_from_slice(&value.max_operations_per_membership.to_be_bytes());
    out.extend_from_slice(&(value.semantic.max_query_bytes as u64).to_be_bytes());
    out.extend_from_slice(&(value.semantic.max_label_bytes as u64).to_be_bytes());
    out.extend_from_slice(&(value.semantic.max_detail_bytes as u64).to_be_bytes());
    out.extend_from_slice(&(value.semantic.max_results as u64).to_be_bytes());
    out.extend_from_slice(&(value.semantic.max_rename_edits as u64).to_be_bytes());
    out.extend_from_slice(&value.semantic.max_requests.to_be_bytes())
}
fn decode_limits(c: &mut Cursor<'_>) -> Result<SemanticSessionLimitsV1, SemanticWireDecodeErrorV1> {
    Ok(SemanticSessionLimitsV1 {
        max_snapshot_bytes: usize::try_from(c.u64()?)
            .map_err(|_| SemanticWireDecodeErrorV1::Limit)?,
        max_total_snapshot_bytes: usize::try_from(c.u64()?)
            .map_err(|_| SemanticWireDecodeErrorV1::Limit)?,
        max_language_bytes: usize::try_from(c.u64()?)
            .map_err(|_| SemanticWireDecodeErrorV1::Limit)?,
        max_documents: c.u32()?,
        max_pending: c.u32()?,
        max_memberships: c.u32()?,
        max_replay_intent_bytes: usize::try_from(c.u64()?)
            .map_err(|_| SemanticWireDecodeErrorV1::Limit)?,
        max_retained_result_bytes: usize::try_from(c.u64()?)
            .map_err(|_| SemanticWireDecodeErrorV1::Limit)?,
        max_requests_per_membership: c.u32()?,
        max_operations_per_membership: c.u32()?,
        semantic: SemanticLimitsV1 {
            max_query_bytes: usize::try_from(c.u64()?)
                .map_err(|_| SemanticWireDecodeErrorV1::Limit)?,
            max_label_bytes: usize::try_from(c.u64()?)
                .map_err(|_| SemanticWireDecodeErrorV1::Limit)?,
            max_detail_bytes: usize::try_from(c.u64()?)
                .map_err(|_| SemanticWireDecodeErrorV1::Limit)?,
            max_results: usize::try_from(c.u64()?).map_err(|_| SemanticWireDecodeErrorV1::Limit)?,
            max_rename_edits: usize::try_from(c.u64()?)
                .map_err(|_| SemanticWireDecodeErrorV1::Limit)?,
            max_requests: c.u64()?,
        },
    })
}
fn limits_narrow(v: SemanticSessionLimitsV1, ceiling: SemanticSessionLimitsV1) -> bool {
    v.is_valid()
        && v.max_snapshot_bytes <= ceiling.max_snapshot_bytes
        && v.max_total_snapshot_bytes <= ceiling.max_total_snapshot_bytes
        && v.max_language_bytes <= ceiling.max_language_bytes
        && v.max_documents <= ceiling.max_documents
        && v.max_pending <= ceiling.max_pending
        && v.max_memberships <= ceiling.max_memberships
        && v.max_replay_intent_bytes <= ceiling.max_replay_intent_bytes
        && v.max_retained_result_bytes <= ceiling.max_retained_result_bytes
        && v.max_requests_per_membership <= ceiling.max_requests_per_membership
        && v.max_operations_per_membership <= ceiling.max_operations_per_membership
        && v.semantic.max_query_bytes <= ceiling.semantic.max_query_bytes
        && v.semantic.max_label_bytes <= ceiling.semantic.max_label_bytes
        && v.semantic.max_detail_bytes <= ceiling.semantic.max_detail_bytes
        && v.semantic.max_results <= ceiling.semantic.max_results
        && v.semantic.max_rename_edits <= ceiling.semantic.max_rename_edits
        && v.semantic.max_requests <= ceiling.semantic.max_requests
}
fn capability_bits(v: SemanticCapabilitySetV1) -> u8 {
    u8::from(v.contains(SemanticCapabilityV1::ReadIndex))
        | (u8::from(v.contains(SemanticCapabilityV1::PlanRename)) << 1)
}
fn decode_capabilities(v: u8) -> Result<SemanticCapabilitySetV1, SemanticWireDecodeErrorV1> {
    if v & !3 != 0 {
        return Err(SemanticWireDecodeErrorV1::Malformed);
    }
    let mut out = SemanticCapabilitySetV1::bottom();
    if v & 1 != 0 {
        out = out.join(SemanticCapabilitySetV1::singleton(
            SemanticCapabilityV1::ReadIndex,
        ))
    }
    if v & 2 != 0 {
        out = out.join(SemanticCapabilitySetV1::singleton(
            SemanticCapabilityV1::PlanRename,
        ))
    }
    Ok(out)
}

fn encode_semantic_reply(
    out: &mut Vec<u8>,
    value: &SemanticReplyV1,
    limits: SemanticWireLimitsV1,
) -> Result<(), SemanticWireEncodeErrorV1> {
    out.extend_from_slice(&value.request.as_bytes());
    out.extend_from_slice(&encode_target(value.target));
    out.extend_from_slice(&value.index_generation.get().to_be_bytes());
    out.extend_from_slice(&value.provider.provider().as_bytes());
    put_text(out, value.provider.profile())?;
    put_text(out, value.provider.version())?;
    encode_result(out, &value.result, limits)
}
fn decode_semantic_reply(
    c: &mut Cursor<'_>,
    limits: SemanticWireLimitsV1,
) -> Result<SemanticReplyV1, SemanticWireDecodeErrorV1> {
    let request = SemanticRequestRefV1::new(c.array16()?);
    let target = decode_target(c.take(24)?)?;
    let index_generation =
        SemanticIndexGenerationV1::new(c.u64()?).ok_or(SemanticWireDecodeErrorV1::Malformed)?;
    let provider_ref = SemanticProviderRefV1::new(c.array16()?);
    let profile = c.text(128)?;
    let version = c.text(128)?;
    let provider =
        SemanticProviderProvenanceV1::try_new(provider_ref, profile, version).map_err(map_build)?;
    let result = decode_result(c, limits)?;
    Ok(SemanticReplyV1 {
        request,
        target,
        index_generation,
        provider,
        result,
    })
}
fn encode_location(out: &mut Vec<u8>, value: SemanticLocationV1) {
    out.extend_from_slice(&encode_target(value.target));
    out.extend_from_slice(&value.span.start.to_be_bytes());
    out.extend_from_slice(&value.span.end.to_be_bytes())
}
fn decode_location(c: &mut Cursor<'_>) -> Result<SemanticLocationV1, SemanticWireDecodeErrorV1> {
    let target = decode_target(c.take(24)?)?;
    let span = SemanticSpanV1::try_new(c.u64()?, c.u64()?).map_err(map_build)?;
    Ok(SemanticLocationV1 { target, span })
}
fn encode_result(
    out: &mut Vec<u8>,
    result: &SemanticResultV1,
    limits: SemanticWireLimitsV1,
) -> Result<(), SemanticWireEncodeErrorV1> {
    if !result.is_within_limits(limits.session.semantic) {
        return Err(SemanticWireEncodeErrorV1::Limit);
    }
    if let Some(values) = result.symbols() {
        out.push(1);
        out.extend_from_slice(&(values.len() as u32).to_be_bytes());
        for value in values {
            put_text(out, value.name.as_str())?;
            out.push(symbol_kind_tag(value.kind));
            encode_location(out, value.location)
        }
    } else if let Some(values) = result.locations() {
        out.push(2);
        out.extend_from_slice(&(values.len() as u32).to_be_bytes());
        for value in values {
            encode_location(out, *value)
        }
    } else if let Some(values) = result.diagnostics() {
        out.push(3);
        out.extend_from_slice(&(values.len() as u32).to_be_bytes());
        for value in values {
            out.push(severity_tag(value.severity));
            encode_location(out, value.location);
            put_text(out, value.message.as_str())?;
            match &value.code {
                Some(code) => {
                    out.push(1);
                    put_text(out, code.as_str())?
                }
                None => out.push(0),
            }
        }
    } else if let Some(value) = result.type_at_point() {
        out.push(4);
        put_text(out, value.as_str())?
    } else if let Some(values) = result.rename_plan() {
        out.push(5);
        out.extend_from_slice(&(values.len() as u32).to_be_bytes());
        for value in values {
            encode_location(out, value.location);
            put_text(out, value.replacement.as_str())?
        }
    } else {
        return Err(SemanticWireEncodeErrorV1::InvalidValue);
    }
    Ok(())
}
fn decode_result(
    c: &mut Cursor<'_>,
    limits: SemanticWireLimitsV1,
) -> Result<SemanticResultV1, SemanticWireDecodeErrorV1> {
    let semantic = limits.session.semantic;
    match c.u8()? {
        1 => {
            let count = c.count(semantic.max_results)?;
            let mut values = Vec::new();
            values
                .try_reserve_exact(count)
                .map_err(|_| SemanticWireDecodeErrorV1::Capacity)?;
            for _ in 0..count {
                values.push(SemanticSymbolV1 {
                    name: SemanticTextV1::try_new(
                        c.text(semantic.max_label_bytes)?,
                        semantic.max_label_bytes,
                    )
                    .map_err(map_build)?,
                    kind: decode_symbol_kind(c.u8()?)?,
                    location: decode_location(c)?,
                })
            }
            SemanticResultV1::try_symbols(values, semantic).map_err(map_build)
        }
        2 => {
            let count = c.count(semantic.max_results)?;
            let mut values = Vec::new();
            values
                .try_reserve_exact(count)
                .map_err(|_| SemanticWireDecodeErrorV1::Capacity)?;
            for _ in 0..count {
                values.push(decode_location(c)?)
            }
            SemanticResultV1::try_locations(values, semantic).map_err(map_build)
        }
        3 => {
            let count = c.count(semantic.max_results)?;
            let mut values = Vec::new();
            values
                .try_reserve_exact(count)
                .map_err(|_| SemanticWireDecodeErrorV1::Capacity)?;
            for _ in 0..count {
                let severity = decode_severity(c.u8()?)?;
                let location = decode_location(c)?;
                let message = SemanticTextV1::try_new(
                    c.text(semantic.max_detail_bytes)?,
                    semantic.max_detail_bytes,
                )
                .map_err(map_build)?;
                let code = match c.u8()? {
                    0 => None,
                    1 => Some(
                        SemanticTextV1::try_new(
                            c.text(semantic.max_label_bytes)?,
                            semantic.max_label_bytes,
                        )
                        .map_err(map_build)?,
                    ),
                    _ => return Err(SemanticWireDecodeErrorV1::Malformed),
                };
                values.push(SemanticDiagnosticV1 {
                    severity,
                    location,
                    message,
                    code,
                })
            }
            SemanticResultV1::try_diagnostics(values, semantic).map_err(map_build)
        }
        4 => SemanticResultV1::try_type_at_point(
            SemanticTextV1::try_new(
                c.text(semantic.max_detail_bytes)?,
                semantic.max_detail_bytes,
            )
            .map_err(map_build)?,
            semantic,
        )
        .map_err(map_build),
        5 => {
            let count = c.count(semantic.max_rename_edits)?;
            let mut values = Vec::new();
            values
                .try_reserve_exact(count)
                .map_err(|_| SemanticWireDecodeErrorV1::Capacity)?;
            for _ in 0..count {
                values.push(SemanticTextEditV1 {
                    location: decode_location(c)?,
                    replacement: SemanticTextV1::try_new(
                        c.text(semantic.max_label_bytes)?,
                        semantic.max_label_bytes,
                    )
                    .map_err(map_build)?,
                })
            }
            SemanticResultV1::try_rename_plan(values, semantic).map_err(map_build)
        }
        _ => Err(SemanticWireDecodeErrorV1::UnknownTag),
    }
}

fn decode_command_kind(
    value: u8,
) -> Result<SemanticSessionCommandKindV1, SemanticWireDecodeErrorV1> {
    match value {
        1 => Ok(SemanticSessionCommandKindV1::Describe),
        2 => Ok(SemanticSessionCommandKindV1::JoinSession),
        3 => Ok(SemanticSessionCommandKindV1::BeginSynchronization),
        4 => Ok(SemanticSessionCommandKindV1::SyncOpen),
        5 => Ok(SemanticSessionCommandKindV1::SyncChange),
        6 => Ok(SemanticSessionCommandKindV1::SyncClose),
        7 => Ok(SemanticSessionCommandKindV1::CompleteSynchronization),
        8 => Ok(SemanticSessionCommandKindV1::Submit),
        9 => Ok(SemanticSessionCommandKindV1::Cancel),
        10 => Ok(SemanticSessionCommandKindV1::Collect),
        11 => Ok(SemanticSessionCommandKindV1::ReleaseRequest),
        12 => Ok(SemanticSessionCommandKindV1::LeaveSession),
        _ => Err(SemanticWireDecodeErrorV1::UnknownTag),
    }
}

const fn query_kind_tag(value: SemanticQueryKindV1) -> u8 {
    match value {
        SemanticQueryKindV1::Symbols => 1,
        SemanticQueryKindV1::Definition => 2,
        SemanticQueryKindV1::References => 3,
        SemanticQueryKindV1::Implementations => 4,
        SemanticQueryKindV1::Diagnostics => 5,
        SemanticQueryKindV1::TypeAtPoint => 6,
        SemanticQueryKindV1::PlanRename => 7,
        SemanticQueryKindV1::IncomingCalls => 8,
        SemanticQueryKindV1::OutgoingCalls => 9,
    }
}

fn decode_query_kind(value: u8) -> Result<SemanticQueryKindV1, SemanticWireDecodeErrorV1> {
    match value {
        1 => Ok(SemanticQueryKindV1::Symbols),
        2 => Ok(SemanticQueryKindV1::Definition),
        3 => Ok(SemanticQueryKindV1::References),
        4 => Ok(SemanticQueryKindV1::Implementations),
        5 => Ok(SemanticQueryKindV1::Diagnostics),
        6 => Ok(SemanticQueryKindV1::TypeAtPoint),
        7 => Ok(SemanticQueryKindV1::PlanRename),
        8 => Ok(SemanticQueryKindV1::IncomingCalls),
        9 => Ok(SemanticQueryKindV1::OutgoingCalls),
        _ => Err(SemanticWireDecodeErrorV1::UnknownTag),
    }
}

const fn peer_refusal_tag(value: SemanticRefusalV1) -> u8 {
    match value {
        SemanticRefusalV1::AuthorityDenied => 1,
        SemanticRefusalV1::InputLimit => 2,
        SemanticRefusalV1::OutputLimit => 3,
        SemanticRefusalV1::InvalidRequest => 4,
        SemanticRefusalV1::MalformedProviderReply => 5,
    }
}

fn decode_peer_refusal(value: u8) -> Result<SemanticRefusalV1, SemanticWireDecodeErrorV1> {
    match value {
        1 => Ok(SemanticRefusalV1::AuthorityDenied),
        2 => Ok(SemanticRefusalV1::InputLimit),
        3 => Ok(SemanticRefusalV1::OutputLimit),
        4 => Ok(SemanticRefusalV1::InvalidRequest),
        5 => Ok(SemanticRefusalV1::MalformedProviderReply),
        _ => Err(SemanticWireDecodeErrorV1::UnknownTag),
    }
}

const fn session_refusal_tag(value: SemanticSessionRefusalV1) -> u8 {
    match value {
        SemanticSessionRefusalV1::InvalidSession => 1,
        SemanticSessionRefusalV1::UnknownMembership => 2,
        SemanticSessionRefusalV1::RoleDenied => 3,
        SemanticSessionRefusalV1::InputLimit => 4,
        SemanticSessionRefusalV1::Capacity => 5,
        SemanticSessionRefusalV1::SynchronizationRequired => 6,
        SemanticSessionRefusalV1::SynchronizationConflict => 7,
        SemanticSessionRefusalV1::ProviderRefused => 8,
        SemanticSessionRefusalV1::ClientRetiring => 9,
        SemanticSessionRefusalV1::AbsentOrForeign => 10,
        SemanticSessionRefusalV1::ProtocolViolation => 11,
    }
}

fn decode_session_refusal(
    value: u8,
) -> Result<SemanticSessionRefusalV1, SemanticWireDecodeErrorV1> {
    match value {
        1 => Ok(SemanticSessionRefusalV1::InvalidSession),
        2 => Ok(SemanticSessionRefusalV1::UnknownMembership),
        3 => Ok(SemanticSessionRefusalV1::RoleDenied),
        4 => Ok(SemanticSessionRefusalV1::InputLimit),
        5 => Ok(SemanticSessionRefusalV1::Capacity),
        6 => Ok(SemanticSessionRefusalV1::SynchronizationRequired),
        7 => Ok(SemanticSessionRefusalV1::SynchronizationConflict),
        8 => Ok(SemanticSessionRefusalV1::ProviderRefused),
        9 => Ok(SemanticSessionRefusalV1::ClientRetiring),
        10 => Ok(SemanticSessionRefusalV1::AbsentOrForeign),
        11 => Ok(SemanticSessionRefusalV1::ProtocolViolation),
        _ => Err(SemanticWireDecodeErrorV1::UnknownTag),
    }
}

const fn symbol_kind_tag(value: SemanticSymbolKindV1) -> u8 {
    match value {
        SemanticSymbolKindV1::Module => 1,
        SemanticSymbolKindV1::Type => 2,
        SemanticSymbolKindV1::Function => 3,
        SemanticSymbolKindV1::Method => 4,
        SemanticSymbolKindV1::Value => 5,
        SemanticSymbolKindV1::Field => 6,
        SemanticSymbolKindV1::Unknown => 7,
    }
}

fn decode_symbol_kind(value: u8) -> Result<SemanticSymbolKindV1, SemanticWireDecodeErrorV1> {
    match value {
        1 => Ok(SemanticSymbolKindV1::Module),
        2 => Ok(SemanticSymbolKindV1::Type),
        3 => Ok(SemanticSymbolKindV1::Function),
        4 => Ok(SemanticSymbolKindV1::Method),
        5 => Ok(SemanticSymbolKindV1::Value),
        6 => Ok(SemanticSymbolKindV1::Field),
        7 => Ok(SemanticSymbolKindV1::Unknown),
        _ => Err(SemanticWireDecodeErrorV1::UnknownTag),
    }
}

const fn severity_tag(value: SemanticDiagnosticSeverityV1) -> u8 {
    match value {
        SemanticDiagnosticSeverityV1::Error => 1,
        SemanticDiagnosticSeverityV1::Warning => 2,
        SemanticDiagnosticSeverityV1::Information => 3,
        SemanticDiagnosticSeverityV1::Hint => 4,
    }
}

fn decode_severity(value: u8) -> Result<SemanticDiagnosticSeverityV1, SemanticWireDecodeErrorV1> {
    match value {
        1 => Ok(SemanticDiagnosticSeverityV1::Error),
        2 => Ok(SemanticDiagnosticSeverityV1::Warning),
        3 => Ok(SemanticDiagnosticSeverityV1::Information),
        4 => Ok(SemanticDiagnosticSeverityV1::Hint),
        _ => Err(SemanticWireDecodeErrorV1::UnknownTag),
    }
}
