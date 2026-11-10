//! Canonical bounded wire encoding for the Vellum native protocol.
#![forbid(unsafe_code)]

use keystone_codec::{DecodeError, DecodeLimits, Decoder, encode_bytes};
use std::{fmt, sync::Arc};
use vellum_protocol::{
    ClientId, CommandV1, OutcomeV1, RefusalV1, ReplyV1, RequestId, RequestV1, SnapshotV1,
    VersionWitness,
};

const VERSION_V1: u8 = 1;
const CONNECT_REQUEST: u8 = 0x01;
const EXECUTE_REQUEST: u8 = 0x02;
const CONNECTED_REPLY: u8 = 0x81;
const CONNECT_REFUSED_REPLY: u8 = 0x82;
const EXECUTED_REPLY: u8 = 0x83;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WireLimits {
    pub max_frame_bytes: usize,
    pub max_text_bytes: usize,
    pub max_fields: usize,
}

impl WireLimits {
    pub const fn reference() -> Self {
        Self {
            max_frame_bytes: 65_600,
            max_text_bytes: 65_536,
            max_fields: 16,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireEncodeError {
    Limit,
    Capacity,
    LengthOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireDecodeError {
    Limit,
    Malformed,
    UnsupportedVersion,
    UnknownTag,
    InvalidIdentity,
    InvalidUtf8,
    TrailingData,
    Capacity,
}

#[derive(Clone, PartialEq, Eq)]
pub enum WireRequestV1 {
    Connect,
    Execute(RequestV1),
}

impl fmt::Debug for WireRequestV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect => f.write_str("Connect"),
            Self::Execute(request) => f.debug_tuple("Execute").field(request).finish(),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum WireReplyV1 {
    Connected(ClientId),
    ConnectRefused(RefusalV1),
    Executed(ReplyV1),
}

impl fmt::Debug for WireReplyV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connected(client) => f.debug_tuple("Connected").field(client).finish(),
            Self::ConnectRefused(refusal) => {
                f.debug_tuple("ConnectRefused").field(refusal).finish()
            }
            Self::Executed(reply) => f.debug_tuple("Executed").field(reply).finish(),
        }
    }
}

pub fn encode_request_v1(
    request: &WireRequestV1,
    limits: WireLimits,
) -> Result<Vec<u8>, WireEncodeError> {
    validate_request(request, limits)?;
    let length = request_length(request)?;
    let mut encoder = Encoder::new(length, limits)?;
    encoder.u8(VERSION_V1)?;
    match request {
        WireRequestV1::Connect => encoder.u8(CONNECT_REQUEST)?,
        WireRequestV1::Execute(request) => {
            encoder.u8(EXECUTE_REQUEST)?;
            encoder.u64(request.client.get())?;
            encoder.u64(request.request.get())?;
            encode_command(&mut encoder, &request.command)?;
        }
    }
    encoder.finish(length)
}

pub fn decode_request_v1(
    input: &[u8],
    limits: WireLimits,
) -> Result<WireRequestV1, WireDecodeError> {
    let mut decoder = decoder(input, limits)?;
    read_version(&mut decoder)?;
    let request = match read_u8(&mut decoder)? {
        CONNECT_REQUEST => WireRequestV1::Connect,
        EXECUTE_REQUEST => {
            let client = client_id(read_u64(&mut decoder)?)?;
            let request = request_id(read_u64(&mut decoder)?)?;
            let command = decode_command(&mut decoder)?;
            WireRequestV1::Execute(RequestV1 {
                client,
                request,
                command,
            })
        }
        _ => return Err(WireDecodeError::UnknownTag),
    };
    finish_decode(&decoder)?;
    Ok(request)
}

pub fn encode_reply_v1(
    reply: &WireReplyV1,
    limits: WireLimits,
) -> Result<Vec<u8>, WireEncodeError> {
    validate_reply(reply, limits)?;
    let length = reply_length(reply)?;
    let mut encoder = Encoder::new(length, limits)?;
    encoder.u8(VERSION_V1)?;
    match reply {
        WireReplyV1::Connected(client) => {
            encoder.u8(CONNECTED_REPLY)?;
            encoder.u64(client.get())?;
        }
        WireReplyV1::ConnectRefused(refusal) => {
            encoder.u8(CONNECT_REFUSED_REPLY)?;
            encoder.u8(refusal_tag(*refusal))?;
        }
        WireReplyV1::Executed(reply) => {
            encoder.u8(EXECUTED_REPLY)?;
            encode_protocol_reply(&mut encoder, reply)?;
        }
    }
    encoder.finish(length)
}

pub fn decode_reply_v1(input: &[u8], limits: WireLimits) -> Result<WireReplyV1, WireDecodeError> {
    let mut decoder = decoder(input, limits)?;
    read_version(&mut decoder)?;
    let reply = match read_u8(&mut decoder)? {
        CONNECTED_REPLY => WireReplyV1::Connected(client_id(read_u64(&mut decoder)?)?),
        CONNECT_REFUSED_REPLY => {
            WireReplyV1::ConnectRefused(decode_refusal(read_u8(&mut decoder)?)?)
        }
        EXECUTED_REPLY => WireReplyV1::Executed(decode_protocol_reply(&mut decoder)?),
        _ => return Err(WireDecodeError::UnknownTag),
    };
    finish_decode(&decoder)?;
    Ok(reply)
}

fn encode_command(encoder: &mut Encoder, command: &CommandV1) -> Result<(), WireEncodeError> {
    match command {
        CommandV1::Snapshot => encoder.u8(1),
        CommandV1::Replace {
            expected,
            start,
            end,
            inserted,
        } => {
            encoder.u8(2)?;
            encoder.u64(expected.get())?;
            encoder.u64(*start)?;
            encoder.u64(*end)?;
            encoder.text(inserted)
        }
    }
}

fn decode_command(decoder: &mut Decoder<'_>) -> Result<CommandV1, WireDecodeError> {
    match read_u8(decoder)? {
        1 => Ok(CommandV1::Snapshot),
        2 => Ok(CommandV1::Replace {
            expected: witness(read_u64(decoder)?)?,
            start: read_u64(decoder)?,
            end: read_u64(decoder)?,
            inserted: read_text(decoder)?,
        }),
        _ => Err(WireDecodeError::UnknownTag),
    }
}

fn encode_protocol_reply(encoder: &mut Encoder, reply: &ReplyV1) -> Result<(), WireEncodeError> {
    match reply {
        ReplyV1::Accepted(outcome) => {
            encoder.u8(1)?;
            match outcome {
                OutcomeV1::Snapshot(snapshot) => {
                    encoder.u8(1)?;
                    encode_snapshot(encoder, snapshot)
                }
                OutcomeV1::Replaced(snapshot) => {
                    encoder.u8(2)?;
                    encode_snapshot(encoder, snapshot)
                }
            }
        }
        ReplyV1::Refused(refusal) => {
            encoder.u8(2)?;
            encoder.u8(refusal_tag(*refusal))
        }
    }
}

fn decode_protocol_reply(decoder: &mut Decoder<'_>) -> Result<ReplyV1, WireDecodeError> {
    match read_u8(decoder)? {
        1 => {
            let outcome = match read_u8(decoder)? {
                1 => OutcomeV1::Snapshot(decode_snapshot(decoder)?),
                2 => OutcomeV1::Replaced(decode_snapshot(decoder)?),
                _ => return Err(WireDecodeError::UnknownTag),
            };
            Ok(ReplyV1::Accepted(outcome))
        }
        2 => Ok(ReplyV1::Refused(decode_refusal(read_u8(decoder)?)?)),
        _ => Err(WireDecodeError::UnknownTag),
    }
}

fn encode_snapshot(encoder: &mut Encoder, snapshot: &SnapshotV1) -> Result<(), WireEncodeError> {
    encoder.u64(snapshot.witness.get())?;
    encoder.u64(snapshot.revision)?;
    encoder.text(&snapshot.text)
}

fn decode_snapshot(decoder: &mut Decoder<'_>) -> Result<SnapshotV1, WireDecodeError> {
    Ok(SnapshotV1 {
        witness: witness(read_u64(decoder)?)?,
        revision: read_u64(decoder)?,
        text: Arc::from(read_text(decoder)?),
    })
}

fn request_length(request: &WireRequestV1) -> Result<usize, WireEncodeError> {
    match request {
        WireRequestV1::Connect => Ok(2),
        WireRequestV1::Execute(request) => match &request.command {
            CommandV1::Snapshot => Ok(19),
            CommandV1::Replace { inserted, .. } => {
                check_wire_text_length(inserted.len())?;
                checked_add(47, inserted.len())
            }
        },
    }
}

fn reply_length(reply: &WireReplyV1) -> Result<usize, WireEncodeError> {
    match reply {
        WireReplyV1::Connected(_) => Ok(10),
        WireReplyV1::ConnectRefused(_) => Ok(3),
        WireReplyV1::Executed(ReplyV1::Refused(_)) => Ok(4),
        WireReplyV1::Executed(ReplyV1::Accepted(outcome)) => {
            let text = match outcome {
                OutcomeV1::Snapshot(snapshot) | OutcomeV1::Replaced(snapshot) => &snapshot.text,
            };
            check_wire_text_length(text.len())?;
            checked_add(24, text.len())
        }
    }
}

fn validate_request(request: &WireRequestV1, limits: WireLimits) -> Result<(), WireEncodeError> {
    let fields = match request {
        WireRequestV1::Connect => 2,
        WireRequestV1::Execute(RequestV1 {
            command: CommandV1::Snapshot,
            ..
        }) => 5,
        WireRequestV1::Execute(RequestV1 {
            command: CommandV1::Replace { inserted, .. },
            ..
        }) => {
            check_text_limit(inserted.len(), limits)?;
            9
        }
    };
    check_field_limit(fields, limits)
}

fn validate_reply(reply: &WireReplyV1, limits: WireLimits) -> Result<(), WireEncodeError> {
    let fields = match reply {
        WireReplyV1::Connected(_) | WireReplyV1::ConnectRefused(_) => 3,
        WireReplyV1::Executed(ReplyV1::Refused(_)) => 4,
        WireReplyV1::Executed(ReplyV1::Accepted(outcome)) => {
            let text = match outcome {
                OutcomeV1::Snapshot(snapshot) | OutcomeV1::Replaced(snapshot) => &snapshot.text,
            };
            check_text_limit(text.len(), limits)?;
            7
        }
    };
    check_field_limit(fields, limits)
}

fn check_text_limit(length: usize, limits: WireLimits) -> Result<(), WireEncodeError> {
    check_wire_text_length(length)?;
    if length > limits.max_text_bytes {
        return Err(WireEncodeError::Limit);
    }
    Ok(())
}

fn check_wire_text_length(length: usize) -> Result<(), WireEncodeError> {
    u32::try_from(length)
        .map(|_| ())
        .map_err(|_| WireEncodeError::LengthOverflow)
}

fn check_field_limit(fields: usize, limits: WireLimits) -> Result<(), WireEncodeError> {
    if fields > limits.max_fields {
        return Err(WireEncodeError::Limit);
    }
    Ok(())
}

fn checked_add(base: usize, length: usize) -> Result<usize, WireEncodeError> {
    base.checked_add(length)
        .ok_or(WireEncodeError::LengthOverflow)
}

struct Encoder {
    output: Vec<u8>,
    fields: usize,
    limits: WireLimits,
}

impl Encoder {
    fn new(length: usize, limits: WireLimits) -> Result<Self, WireEncodeError> {
        if length > limits.max_frame_bytes {
            return Err(WireEncodeError::Limit);
        }
        let mut output = Vec::new();
        output
            .try_reserve_exact(length)
            .map_err(|_| WireEncodeError::Capacity)?;
        Ok(Self {
            output,
            fields: 0,
            limits,
        })
    }

    fn u8(&mut self, value: u8) -> Result<(), WireEncodeError> {
        self.field()?;
        self.output.push(value);
        Ok(())
    }

    fn u64(&mut self, value: u64) -> Result<(), WireEncodeError> {
        self.field()?;
        self.output.extend_from_slice(&value.to_be_bytes());
        Ok(())
    }

    fn text(&mut self, value: &str) -> Result<(), WireEncodeError> {
        self.field()?;
        if value.len() > self.limits.max_text_bytes {
            return Err(WireEncodeError::Limit);
        }
        encode_bytes(&mut self.output, value.as_bytes())
            .map_err(|_| WireEncodeError::LengthOverflow)
    }

    fn field(&mut self) -> Result<(), WireEncodeError> {
        if self.fields >= self.limits.max_fields {
            return Err(WireEncodeError::Limit);
        }
        self.fields += 1;
        Ok(())
    }

    fn finish(self, expected: usize) -> Result<Vec<u8>, WireEncodeError> {
        if self.output.len() != expected || self.output.len() > self.limits.max_frame_bytes {
            return Err(WireEncodeError::LengthOverflow);
        }
        Ok(self.output)
    }
}

fn decoder(input: &[u8], limits: WireLimits) -> Result<Decoder<'_>, WireDecodeError> {
    Decoder::new(
        input,
        DecodeLimits {
            max_input_bytes: limits.max_frame_bytes,
            max_field_bytes: limits.max_text_bytes,
            max_fields: limits.max_fields,
        },
    )
    .map_err(map_decode)
}

fn read_version(decoder: &mut Decoder<'_>) -> Result<(), WireDecodeError> {
    if read_u8(decoder)? != VERSION_V1 {
        return Err(WireDecodeError::UnsupportedVersion);
    }
    Ok(())
}

fn read_u8(decoder: &mut Decoder<'_>) -> Result<u8, WireDecodeError> {
    decoder.read_u8().map_err(map_decode)
}

fn read_u64(decoder: &mut Decoder<'_>) -> Result<u64, WireDecodeError> {
    decoder.read_u64_be().map_err(map_decode)
}

fn read_text(decoder: &mut Decoder<'_>) -> Result<String, WireDecodeError> {
    let bytes = decoder.read_bytes().map_err(map_decode)?;
    let value = std::str::from_utf8(bytes).map_err(|_| WireDecodeError::InvalidUtf8)?;
    let mut text = String::new();
    text.try_reserve_exact(value.len())
        .map_err(|_| WireDecodeError::Capacity)?;
    text.push_str(value);
    Ok(text)
}

fn finish_decode(decoder: &Decoder<'_>) -> Result<(), WireDecodeError> {
    if !decoder.is_finished() {
        return Err(WireDecodeError::TrailingData);
    }
    Ok(())
}

fn map_decode(error: DecodeError) -> WireDecodeError {
    match error {
        DecodeError::InputLimit | DecodeError::FieldLimit | DecodeError::FieldCountLimit => {
            WireDecodeError::Limit
        }
        DecodeError::Truncated | DecodeError::LengthOverflow => WireDecodeError::Malformed,
    }
}

fn client_id(value: u64) -> Result<ClientId, WireDecodeError> {
    ClientId::new(value).ok_or(WireDecodeError::InvalidIdentity)
}

fn request_id(value: u64) -> Result<RequestId, WireDecodeError> {
    RequestId::new(value).ok_or(WireDecodeError::InvalidIdentity)
}

fn witness(value: u64) -> Result<VersionWitness, WireDecodeError> {
    VersionWitness::new(value).ok_or(WireDecodeError::InvalidIdentity)
}

fn refusal_tag(refusal: RefusalV1) -> u8 {
    match refusal {
        RefusalV1::UnknownClient => 1,
        RefusalV1::ClientCapacity => 2,
        RefusalV1::RequestCapacity => 3,
        RefusalV1::RequestConflict => 4,
        RefusalV1::WitnessCapacity => 5,
        RefusalV1::UnknownWitness => 6,
        RefusalV1::Stale => 7,
        RefusalV1::InvalidRange => 8,
        RefusalV1::InputLimit => 9,
        RefusalV1::SnapshotLimit => 10,
        RefusalV1::EditorClosed => 11,
        RefusalV1::EditorCapacity => 12,
        RefusalV1::EditorConflict => 13,
        RefusalV1::EditorIncomplete => 14,
        RefusalV1::EditorUnavailable => 15,
        RefusalV1::CounterExhausted => 16,
    }
}

fn decode_refusal(tag: u8) -> Result<RefusalV1, WireDecodeError> {
    match tag {
        1 => Ok(RefusalV1::UnknownClient),
        2 => Ok(RefusalV1::ClientCapacity),
        3 => Ok(RefusalV1::RequestCapacity),
        4 => Ok(RefusalV1::RequestConflict),
        5 => Ok(RefusalV1::WitnessCapacity),
        6 => Ok(RefusalV1::UnknownWitness),
        7 => Ok(RefusalV1::Stale),
        8 => Ok(RefusalV1::InvalidRange),
        9 => Ok(RefusalV1::InputLimit),
        10 => Ok(RefusalV1::SnapshotLimit),
        11 => Ok(RefusalV1::EditorClosed),
        12 => Ok(RefusalV1::EditorCapacity),
        13 => Ok(RefusalV1::EditorConflict),
        14 => Ok(RefusalV1::EditorIncomplete),
        15 => Ok(RefusalV1::EditorUnavailable),
        16 => Ok(RefusalV1::CounterExhausted),
        _ => Err(WireDecodeError::UnknownTag),
    }
}
