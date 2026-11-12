//! Canonical bounded frames for the semantic isolation-helper protocol.
#![forbid(unsafe_code)]

use keystone_codec::{DecodeLimits, Decoder, encode_bytes};
use std::collections::BTreeMap;
use vellum_semantic_helper_protocol::{
    MAX_HELPER_RELATIVE_PATH_BYTES_V1, MAX_HELPER_SNAPSHOT_FILES_V1, SemanticHelperExchangeRefV1,
    SemanticHelperIsolationPolicyV1, SemanticHelperProviderQualificationDigestV1,
    SemanticHelperRefusalV1, SemanticHelperRelativePathV1, SemanticHelperReplyV1,
    SemanticHelperRequestV1, SemanticHelperResourceLimitsV1, SemanticHelperSessionPhaseV1,
    SemanticHelperSessionRefV1, SemanticHelperSnapshotDigestV1, SemanticHelperSnapshotEntryV1,
    SemanticHelperSnapshotManifestV1,
};

const MAGIC: &[u8; 4] = b"VSH1";
const VERSION: u8 = 1;
const MAX_FRAME_BYTES: usize = 24 * 1024 * 1024;
const MAX_FIELD_BYTES: usize = 20 * 1024 * 1024;
const MAX_ENVELOPE_FIELDS: usize = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticHelperWireEncodeErrorV1 {
    Limit,
    InvalidValue,
    Capacity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticHelperWireDecodeErrorV1 {
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

pub fn encode_helper_request_v1(
    request: &SemanticHelperRequestV1,
) -> Result<Vec<u8>, SemanticHelperWireEncodeErrorV1> {
    match request {
        SemanticHelperRequestV1::Start {
            exchange,
            session,
            snapshot,
            provider_qualification,
            isolation,
        } => encode_envelope(
            1,
            vec![
                field(1, exchange.as_bytes().to_vec()),
                field(2, session.as_bytes().to_vec()),
                field(3, encode_manifest(snapshot)?),
                field(4, provider_qualification.as_bytes().to_vec()),
                field(5, encode_resources(isolation.resources())),
            ],
        ),
        SemanticHelperRequestV1::Poll { exchange, session } => encode_envelope(
            2,
            vec![
                field(1, exchange.as_bytes().to_vec()),
                field(2, session.as_bytes().to_vec()),
            ],
        ),
        SemanticHelperRequestV1::Stop { exchange, session } => encode_envelope(
            3,
            vec![
                field(1, exchange.as_bytes().to_vec()),
                field(2, session.as_bytes().to_vec()),
            ],
        ),
    }
}

pub fn decode_helper_request_v1(
    input: &[u8],
) -> Result<SemanticHelperRequestV1, SemanticHelperWireDecodeErrorV1> {
    let (kind, fields) = decode_envelope(input)?;
    match kind {
        1 => {
            exact(&fields, &[1, 2, 3, 4, 5])?;
            Ok(SemanticHelperRequestV1::Start {
                exchange: exchange(required(&fields, 1)?)?,
                session: session(required(&fields, 2)?)?,
                snapshot: decode_manifest(required(&fields, 3)?)?,
                provider_qualification: provider_digest(required(&fields, 4)?)?,
                isolation: SemanticHelperIsolationPolicyV1::new(decode_resources(required(
                    &fields, 5,
                )?)?),
            })
        }
        2 => {
            exact(&fields, &[1, 2])?;
            Ok(SemanticHelperRequestV1::Poll {
                exchange: exchange(required(&fields, 1)?)?,
                session: session(required(&fields, 2)?)?,
            })
        }
        3 => {
            exact(&fields, &[1, 2])?;
            Ok(SemanticHelperRequestV1::Stop {
                exchange: exchange(required(&fields, 1)?)?,
                session: session(required(&fields, 2)?)?,
            })
        }
        _ => Err(SemanticHelperWireDecodeErrorV1::UnknownTag),
    }
}

pub fn encode_helper_reply_v1(
    reply: &SemanticHelperReplyV1,
) -> Result<Vec<u8>, SemanticHelperWireEncodeErrorV1> {
    match reply {
        SemanticHelperReplyV1::Status {
            exchange,
            session,
            snapshot,
            phase,
        } => encode_envelope(
            4,
            vec![
                field(1, exchange.as_bytes().to_vec()),
                field(2, session.as_bytes().to_vec()),
                field(3, snapshot.as_bytes().to_vec()),
                field(4, vec![encode_phase(*phase)]),
            ],
        ),
        SemanticHelperReplyV1::Refused {
            exchange,
            session,
            refusal,
        } => encode_envelope(
            5,
            vec![
                field(1, exchange.as_bytes().to_vec()),
                field(2, session.as_bytes().to_vec()),
                field(3, vec![encode_refusal(*refusal)]),
            ],
        ),
    }
}

pub fn decode_helper_reply_v1(
    input: &[u8],
) -> Result<SemanticHelperReplyV1, SemanticHelperWireDecodeErrorV1> {
    let (kind, fields) = decode_envelope(input)?;
    match kind {
        4 => {
            exact(&fields, &[1, 2, 3, 4])?;
            Ok(SemanticHelperReplyV1::Status {
                exchange: exchange(required(&fields, 1)?)?,
                session: session(required(&fields, 2)?)?,
                snapshot: snapshot_digest(required(&fields, 3)?)?,
                phase: decode_phase(one(required(&fields, 4)?)?)?,
            })
        }
        5 => {
            exact(&fields, &[1, 2, 3])?;
            Ok(SemanticHelperReplyV1::Refused {
                exchange: exchange(required(&fields, 1)?)?,
                session: session(required(&fields, 2)?)?,
                refusal: decode_refusal(one(required(&fields, 3)?)?)?,
            })
        }
        _ => Err(SemanticHelperWireDecodeErrorV1::UnknownTag),
    }
}

fn encode_envelope(
    kind: u8,
    fields: Vec<Field>,
) -> Result<Vec<u8>, SemanticHelperWireEncodeErrorV1> {
    if fields.len() > MAX_ENVELOPE_FIELDS {
        return Err(SemanticHelperWireEncodeErrorV1::Limit);
    }
    let mut previous = 0_u8;
    let mut length = 7_usize;
    for item in &fields {
        if item.tag <= previous {
            return Err(SemanticHelperWireEncodeErrorV1::InvalidValue);
        }
        if item.bytes.len() > MAX_FIELD_BYTES {
            return Err(SemanticHelperWireEncodeErrorV1::Limit);
        }
        previous = item.tag;
        length = length
            .checked_add(5)
            .and_then(|value| value.checked_add(item.bytes.len()))
            .ok_or(SemanticHelperWireEncodeErrorV1::Limit)?;
    }
    if length > MAX_FRAME_BYTES {
        return Err(SemanticHelperWireEncodeErrorV1::Limit);
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(length)
        .map_err(|_| SemanticHelperWireEncodeErrorV1::Capacity)?;
    output.extend_from_slice(MAGIC);
    output.push(VERSION);
    output.push(kind);
    output.push(fields.len() as u8);
    for item in fields {
        output.push(item.tag);
        output.extend_from_slice(&(item.bytes.len() as u32).to_be_bytes());
        output.extend_from_slice(&item.bytes);
    }
    Ok(output)
}

fn decode_envelope(
    input: &[u8],
) -> Result<(u8, BTreeMap<u8, Vec<u8>>), SemanticHelperWireDecodeErrorV1> {
    if input.len() > MAX_FRAME_BYTES {
        return Err(SemanticHelperWireDecodeErrorV1::Limit);
    }
    if input.len() < 7 {
        return Err(SemanticHelperWireDecodeErrorV1::Truncated);
    }
    if &input[..4] != MAGIC {
        return Err(SemanticHelperWireDecodeErrorV1::BadMagic);
    }
    if input[4] != VERSION {
        return Err(SemanticHelperWireDecodeErrorV1::UnsupportedVersion);
    }
    let kind = input[5];
    let count = usize::from(input[6]);
    if count > MAX_ENVELOPE_FIELDS {
        return Err(SemanticHelperWireDecodeErrorV1::Limit);
    }
    let mut cursor = 7_usize;
    let mut previous = 0_u8;
    let mut fields = BTreeMap::new();
    for _ in 0..count {
        let tag = *input
            .get(cursor)
            .ok_or(SemanticHelperWireDecodeErrorV1::Truncated)?;
        cursor += 1;
        if tag <= previous {
            return Err(SemanticHelperWireDecodeErrorV1::DuplicateOrReorderedField);
        }
        previous = tag;
        let length = u32::from_be_bytes(
            input
                .get(cursor..cursor + 4)
                .ok_or(SemanticHelperWireDecodeErrorV1::Truncated)?
                .try_into()
                .map_err(|_| SemanticHelperWireDecodeErrorV1::Truncated)?,
        ) as usize;
        cursor += 4;
        if length > MAX_FIELD_BYTES {
            return Err(SemanticHelperWireDecodeErrorV1::Limit);
        }
        let value = input
            .get(cursor..cursor + length)
            .ok_or(SemanticHelperWireDecodeErrorV1::Truncated)?;
        cursor += length;
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(length)
            .map_err(|_| SemanticHelperWireDecodeErrorV1::Capacity)?;
        owned.extend_from_slice(value);
        fields.insert(tag, owned);
    }
    if cursor != input.len() {
        return Err(SemanticHelperWireDecodeErrorV1::TrailingData);
    }
    Ok((kind, fields))
}

fn encode_manifest(
    manifest: &SemanticHelperSnapshotManifestV1,
) -> Result<Vec<u8>, SemanticHelperWireEncodeErrorV1> {
    let length = manifest.entries().iter().try_fold(8_usize, |total, entry| {
        total
            .checked_add(4)
            .and_then(|value| value.checked_add(entry.path().as_str().len()))
            .and_then(|value| value.checked_add(8 + 4 + 32))
    });
    let length = length.ok_or(SemanticHelperWireEncodeErrorV1::Limit)?;
    if length > MAX_FIELD_BYTES {
        return Err(SemanticHelperWireEncodeErrorV1::Limit);
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(length)
        .map_err(|_| SemanticHelperWireEncodeErrorV1::Capacity)?;
    output.extend_from_slice(&(manifest.entries().len() as u64).to_be_bytes());
    for entry in manifest.entries() {
        encode_bytes(&mut output, entry.path().as_str().as_bytes())
            .map_err(|_| SemanticHelperWireEncodeErrorV1::Limit)?;
        output.extend_from_slice(&entry.byte_len().to_be_bytes());
        encode_bytes(&mut output, &entry.digest().as_bytes())
            .map_err(|_| SemanticHelperWireEncodeErrorV1::Limit)?;
    }
    Ok(output)
}

fn decode_manifest(
    bytes: &[u8],
) -> Result<SemanticHelperSnapshotManifestV1, SemanticHelperWireDecodeErrorV1> {
    let max_fields = 1_usize
        .checked_add(MAX_HELPER_SNAPSHOT_FILES_V1.saturating_mul(3))
        .ok_or(SemanticHelperWireDecodeErrorV1::Limit)?;
    let mut decoder = Decoder::new(
        bytes,
        DecodeLimits {
            max_input_bytes: MAX_FIELD_BYTES,
            max_field_bytes: MAX_HELPER_RELATIVE_PATH_BYTES_V1,
            max_fields,
        },
    )
    .map_err(|_| SemanticHelperWireDecodeErrorV1::Limit)?;
    let count = usize::try_from(decoder.read_u64_be().map_err(decode_keystone)?)
        .map_err(|_| SemanticHelperWireDecodeErrorV1::Limit)?;
    if count == 0 || count > MAX_HELPER_SNAPSHOT_FILES_V1 {
        return Err(SemanticHelperWireDecodeErrorV1::Limit);
    }
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(count)
        .map_err(|_| SemanticHelperWireDecodeErrorV1::Capacity)?;
    let mut previous: Option<SemanticHelperRelativePathV1> = None;
    for _ in 0..count {
        let path = std::str::from_utf8(decoder.read_bytes().map_err(decode_keystone)?)
            .map_err(|_| SemanticHelperWireDecodeErrorV1::InvalidUtf8)?;
        let path = SemanticHelperRelativePathV1::new(path)
            .map_err(|_| SemanticHelperWireDecodeErrorV1::Malformed)?;
        if previous.as_ref().is_some_and(|prior| prior >= &path) {
            return Err(SemanticHelperWireDecodeErrorV1::DuplicateOrReorderedField);
        }
        let byte_len = decoder.read_u64_be().map_err(decode_keystone)?;
        let digest = fixed32(decoder.read_bytes().map_err(decode_keystone)?)?;
        let entry = SemanticHelperSnapshotEntryV1::new(path.clone(), byte_len, digest)
            .map_err(|_| SemanticHelperWireDecodeErrorV1::Malformed)?;
        previous = Some(path);
        entries.push(entry);
    }
    if !decoder.is_finished() {
        return Err(SemanticHelperWireDecodeErrorV1::TrailingData);
    }
    SemanticHelperSnapshotManifestV1::try_from_entries(entries)
        .map_err(|_| SemanticHelperWireDecodeErrorV1::Malformed)
}

fn encode_resources(resources: SemanticHelperResourceLimitsV1) -> Vec<u8> {
    let mut output = Vec::with_capacity(18);
    output.extend_from_slice(&resources.memory_bytes().to_be_bytes());
    output.extend_from_slice(&resources.process_count().to_be_bytes());
    output.extend_from_slice(&resources.cpu_millis().to_be_bytes());
    output
}

fn decode_resources(
    bytes: &[u8],
) -> Result<SemanticHelperResourceLimitsV1, SemanticHelperWireDecodeErrorV1> {
    if bytes.len() != 18 {
        return Err(SemanticHelperWireDecodeErrorV1::Malformed);
    }
    let memory = u64::from_be_bytes(bytes[..8].try_into().expect("exact length checked"));
    let processes = u16::from_be_bytes(bytes[8..10].try_into().expect("exact length checked"));
    let cpu = u64::from_be_bytes(bytes[10..].try_into().expect("exact length checked"));
    SemanticHelperResourceLimitsV1::new(memory, processes, cpu)
        .map_err(|_| SemanticHelperWireDecodeErrorV1::Malformed)
}

fn required(
    fields: &BTreeMap<u8, Vec<u8>>,
    tag: u8,
) -> Result<&[u8], SemanticHelperWireDecodeErrorV1> {
    fields
        .get(&tag)
        .map(Vec::as_slice)
        .ok_or(SemanticHelperWireDecodeErrorV1::MissingField)
}

fn exact(
    fields: &BTreeMap<u8, Vec<u8>>,
    tags: &[u8],
) -> Result<(), SemanticHelperWireDecodeErrorV1> {
    if fields.len() == tags.len() && fields.keys().copied().eq(tags.iter().copied()) {
        Ok(())
    } else {
        Err(SemanticHelperWireDecodeErrorV1::UnknownTag)
    }
}

fn fixed16(bytes: &[u8]) -> Result<[u8; 16], SemanticHelperWireDecodeErrorV1> {
    bytes
        .try_into()
        .map_err(|_| SemanticHelperWireDecodeErrorV1::InvalidIdentity)
}

fn fixed32(bytes: &[u8]) -> Result<[u8; 32], SemanticHelperWireDecodeErrorV1> {
    bytes
        .try_into()
        .map_err(|_| SemanticHelperWireDecodeErrorV1::InvalidIdentity)
}

fn exchange(bytes: &[u8]) -> Result<SemanticHelperExchangeRefV1, SemanticHelperWireDecodeErrorV1> {
    SemanticHelperExchangeRefV1::new(fixed16(bytes)?)
        .ok_or(SemanticHelperWireDecodeErrorV1::InvalidIdentity)
}

fn session(bytes: &[u8]) -> Result<SemanticHelperSessionRefV1, SemanticHelperWireDecodeErrorV1> {
    SemanticHelperSessionRefV1::new(fixed16(bytes)?)
        .ok_or(SemanticHelperWireDecodeErrorV1::InvalidIdentity)
}

fn snapshot_digest(
    bytes: &[u8],
) -> Result<SemanticHelperSnapshotDigestV1, SemanticHelperWireDecodeErrorV1> {
    SemanticHelperSnapshotDigestV1::new(fixed32(bytes)?)
        .ok_or(SemanticHelperWireDecodeErrorV1::InvalidIdentity)
}

fn provider_digest(
    bytes: &[u8],
) -> Result<SemanticHelperProviderQualificationDigestV1, SemanticHelperWireDecodeErrorV1> {
    SemanticHelperProviderQualificationDigestV1::new(fixed32(bytes)?)
        .ok_or(SemanticHelperWireDecodeErrorV1::InvalidIdentity)
}

fn one(bytes: &[u8]) -> Result<u8, SemanticHelperWireDecodeErrorV1> {
    if bytes.len() == 1 {
        Ok(bytes[0])
    } else {
        Err(SemanticHelperWireDecodeErrorV1::Malformed)
    }
}

fn encode_phase(phase: SemanticHelperSessionPhaseV1) -> u8 {
    match phase {
        SemanticHelperSessionPhaseV1::Starting => 1,
        SemanticHelperSessionPhaseV1::Ready => 2,
        SemanticHelperSessionPhaseV1::Stopping => 3,
        SemanticHelperSessionPhaseV1::Stopped => 4,
        SemanticHelperSessionPhaseV1::Failed => 5,
    }
}

fn decode_phase(
    value: u8,
) -> Result<SemanticHelperSessionPhaseV1, SemanticHelperWireDecodeErrorV1> {
    match value {
        1 => Ok(SemanticHelperSessionPhaseV1::Starting),
        2 => Ok(SemanticHelperSessionPhaseV1::Ready),
        3 => Ok(SemanticHelperSessionPhaseV1::Stopping),
        4 => Ok(SemanticHelperSessionPhaseV1::Stopped),
        5 => Ok(SemanticHelperSessionPhaseV1::Failed),
        _ => Err(SemanticHelperWireDecodeErrorV1::UnknownTag),
    }
}

fn encode_refusal(refusal: SemanticHelperRefusalV1) -> u8 {
    match refusal {
        SemanticHelperRefusalV1::AlreadyExists => 1,
        SemanticHelperRefusalV1::UnknownSession => 2,
        SemanticHelperRefusalV1::SnapshotMismatch => 3,
        SemanticHelperRefusalV1::IsolationUnavailable => 4,
        SemanticHelperRefusalV1::ResourceLimit => 5,
        SemanticHelperRefusalV1::ProviderFailed => 6,
    }
}

fn decode_refusal(value: u8) -> Result<SemanticHelperRefusalV1, SemanticHelperWireDecodeErrorV1> {
    match value {
        1 => Ok(SemanticHelperRefusalV1::AlreadyExists),
        2 => Ok(SemanticHelperRefusalV1::UnknownSession),
        3 => Ok(SemanticHelperRefusalV1::SnapshotMismatch),
        4 => Ok(SemanticHelperRefusalV1::IsolationUnavailable),
        5 => Ok(SemanticHelperRefusalV1::ResourceLimit),
        6 => Ok(SemanticHelperRefusalV1::ProviderFailed),
        _ => Err(SemanticHelperWireDecodeErrorV1::UnknownTag),
    }
}

fn decode_keystone(error: keystone_codec::DecodeError) -> SemanticHelperWireDecodeErrorV1 {
    match error {
        keystone_codec::DecodeError::Truncated => SemanticHelperWireDecodeErrorV1::Truncated,
        keystone_codec::DecodeError::InputLimit
        | keystone_codec::DecodeError::FieldLimit
        | keystone_codec::DecodeError::FieldCountLimit
        | keystone_codec::DecodeError::LengthOverflow => SemanticHelperWireDecodeErrorV1::Limit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> SemanticHelperSnapshotManifestV1 {
        SemanticHelperSnapshotManifestV1::try_from_entries(vec![
            SemanticHelperSnapshotEntryV1::new(
                SemanticHelperRelativePathV1::new("a/file.rs").unwrap(),
                5,
                [4; 32],
            )
            .unwrap(),
            SemanticHelperSnapshotEntryV1::new(
                SemanticHelperRelativePathV1::new("b/file.rs").unwrap(),
                3,
                [5; 32],
            )
            .unwrap(),
        ])
        .unwrap()
    }

    fn start() -> SemanticHelperRequestV1 {
        SemanticHelperRequestV1::Start {
            exchange: SemanticHelperExchangeRefV1::new([1; 16]).unwrap(),
            session: SemanticHelperSessionRefV1::new([2; 16]).unwrap(),
            snapshot: manifest(),
            provider_qualification: SemanticHelperProviderQualificationDigestV1::new([3; 32])
                .unwrap(),
            isolation: SemanticHelperIsolationPolicyV1::new(
                SemanticHelperResourceLimitsV1::new(1024, 2, 500).unwrap(),
            ),
        }
    }

    #[test]
    fn every_request_and_reply_round_trips_canonically() {
        let session = SemanticHelperSessionRefV1::new([2; 16]).unwrap();
        let exchange = SemanticHelperExchangeRefV1::new([1; 16]).unwrap();
        for request in [
            start(),
            SemanticHelperRequestV1::Poll { exchange, session },
            SemanticHelperRequestV1::Stop { exchange, session },
        ] {
            let encoded = encode_helper_request_v1(&request).unwrap();
            let decoded = decode_helper_request_v1(&encoded).unwrap();
            assert_eq!(decoded, request);
            assert_eq!(encode_helper_request_v1(&decoded).unwrap(), encoded);
        }
        for reply in [
            SemanticHelperReplyV1::Status {
                exchange,
                session,
                snapshot: manifest().digest(),
                phase: SemanticHelperSessionPhaseV1::Ready,
            },
            SemanticHelperReplyV1::Refused {
                exchange,
                session,
                refusal: SemanticHelperRefusalV1::IsolationUnavailable,
            },
        ] {
            let encoded = encode_helper_reply_v1(&reply).unwrap();
            assert_eq!(decode_helper_reply_v1(&encoded).unwrap(), reply);
        }
    }

    #[test]
    fn malformed_envelopes_and_noncanonical_manifests_fail_closed() {
        let encoded = encode_helper_request_v1(&start()).unwrap();
        for cut in 0..encoded.len() {
            assert!(decode_helper_request_v1(&encoded[..cut]).is_err());
        }
        let mut trailing = encoded.clone();
        trailing.push(0);
        assert_eq!(
            decode_helper_request_v1(&trailing),
            Err(SemanticHelperWireDecodeErrorV1::TrailingData)
        );

        let mut reordered = encode_manifest(&manifest()).unwrap();
        // Replace the second canonical path with the first, preserving length,
        // so duplicate/order rejection is exercised independently of framing.
        let first = b"a/file.rs";
        let second_offset = 8 + 4 + first.len() + 8 + 4 + 32;
        reordered[second_offset + 4..second_offset + 4 + first.len()].copy_from_slice(first);
        assert_eq!(
            decode_manifest(&reordered),
            Err(SemanticHelperWireDecodeErrorV1::DuplicateOrReorderedField)
        );
    }

    #[test]
    fn zero_identities_unknown_tags_and_oversize_frames_refuse() {
        let mut encoded = encode_helper_request_v1(&start()).unwrap();
        encoded[12..28].fill(0);
        assert_eq!(
            decode_helper_request_v1(&encoded),
            Err(SemanticHelperWireDecodeErrorV1::InvalidIdentity)
        );
        let mut unknown = encode_helper_reply_v1(&SemanticHelperReplyV1::Refused {
            exchange: SemanticHelperExchangeRefV1::new([1; 16]).unwrap(),
            session: SemanticHelperSessionRefV1::new([2; 16]).unwrap(),
            refusal: SemanticHelperRefusalV1::ProviderFailed,
        })
        .unwrap();
        *unknown.last_mut().unwrap() = 99;
        assert_eq!(
            decode_helper_reply_v1(&unknown),
            Err(SemanticHelperWireDecodeErrorV1::UnknownTag)
        );
        assert_eq!(
            decode_helper_request_v1(&vec![0; MAX_FRAME_BYTES + 1]),
            Err(SemanticHelperWireDecodeErrorV1::Limit)
        );
    }
}
