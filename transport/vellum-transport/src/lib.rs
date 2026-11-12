//! Bounded request/reply framing and remote Vellum service transport.
#![forbid(unsafe_code)]

use std::io::{self, Read, Write};
use vellum_protocol::{
    ClientId, DurableExecuteOutcomeV1, DurableExecuteRequestV1, DurableReceiptLookupOutcomeV1,
    DurableReceiptLookupV1, DurableReceiptRetirementOutcomeV1, DurableReceiptRetirementV1,
    DurableReceiptStatusRequestV1, DurableReceiptStatusV1, ReplyV1, RequestV1,
    ServiceAccessErrorV1, ServiceControlErrorV1, ServicePort,
};
use vellum_wire::{
    WireDecodeError, WireEncodeError, WireLimits, WireReplyV1, WireRequestV1, decode_reply_v1,
    decode_request_v1, encode_reply_v1, encode_request_v1,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransportLimits {
    pub max_frame_bytes: usize,
    pub wire: WireLimits,
}

impl TransportLimits {
    pub const fn reference() -> Self {
        Self {
            max_frame_bytes: 65_600,
            wire: WireLimits::reference(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportBuildError {
    InvalidLimits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportError {
    /// No request byte reached the peer.
    NotSent,
    /// Transmission began, but no trustworthy reply established the outcome.
    OutcomeUnknown,
    /// A raw framing I/O failure without an exchange-phase claim.
    Io,
    /// A raw frame ended before its declared length.
    Truncated,
    FrameLimit,
    LengthOverflow,
    Capacity,
    Malformed,
    UnexpectedReply,
    ServiceUnavailable,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TransportUsage {
    pub exchanges: u64,
    pub completed: u64,
    pub io_failures: u64,
    pub limit_failures: u64,
    pub malformed_failures: u64,
    pub service_failures: u64,
    pub not_sent_failures: u64,
    pub outcome_unknown_failures: u64,
    pub counter_saturated: bool,
}

pub trait FrameExchange {
    /// Exchanges one complete request for one complete reply.
    ///
    /// Implementations crossing an effect boundary must return [`NotSent`]
    /// only when no request byte was transmitted and [`OutcomeUnknown`] once
    /// transmission has begun. Raw `Io` and `Truncated` errors do not carry
    /// enough evidence for a caller to retry a state-changing request safely.
    ///
    /// [`NotSent`]: TransportError::NotSent
    /// [`OutcomeUnknown`]: TransportError::OutcomeUnknown
    fn exchange(
        &mut self,
        request: &[u8],
        max_reply_bytes: usize,
    ) -> Result<Vec<u8>, TransportError>;
}

pub struct RemoteTransport<X> {
    exchange: X,
    limits: TransportLimits,
    usage: TransportUsage,
}

impl<X> RemoteTransport<X> {
    pub fn new(exchange: X, limits: TransportLimits) -> Result<Self, TransportBuildError> {
        if limits.max_frame_bytes == 0
            || limits.wire.max_frame_bytes == 0
            || limits.wire.max_frame_bytes > limits.max_frame_bytes
        {
            return Err(TransportBuildError::InvalidLimits);
        }
        Ok(Self {
            exchange,
            limits,
            usage: TransportUsage::default(),
        })
    }

    pub fn usage(&self) -> TransportUsage {
        self.usage
    }

    pub fn into_exchange(self) -> X {
        self.exchange
    }
}

impl<X: FrameExchange> RemoteTransport<X> {
    fn call(&mut self, request: &WireRequestV1) -> Result<WireReplyV1, ServiceAccessErrorV1> {
        bump(&mut self.usage.exchanges, &mut self.usage.counter_saturated);
        let encoded = encode_request_v1(request, self.limits.wire)
            .map_err(map_encode)
            .map_err(|error| self.failed(error))?;
        let reply = self
            .exchange
            .exchange(&encoded, self.limits.max_frame_bytes)
            .map_err(|error| self.failed(error))?;
        let decoded = decode_reply_v1(&reply, self.limits.wire)
            .map_err(|_| self.failed(TransportError::OutcomeUnknown))?;
        bump(&mut self.usage.completed, &mut self.usage.counter_saturated);
        Ok(decoded)
    }

    fn unexpected<T>(&mut self) -> Result<T, ServiceAccessErrorV1> {
        Err(self.failed(TransportError::OutcomeUnknown))
    }

    fn failed(&mut self, error: TransportError) -> ServiceAccessErrorV1 {
        let category = match error {
            TransportError::NotSent => &mut self.usage.not_sent_failures,
            TransportError::OutcomeUnknown => &mut self.usage.outcome_unknown_failures,
            TransportError::Io | TransportError::Truncated => &mut self.usage.io_failures,
            TransportError::FrameLimit
            | TransportError::LengthOverflow
            | TransportError::Capacity => &mut self.usage.limit_failures,
            TransportError::Malformed | TransportError::UnexpectedReply => {
                &mut self.usage.malformed_failures
            }
            TransportError::ServiceUnavailable => &mut self.usage.service_failures,
        };
        bump(category, &mut self.usage.counter_saturated);
        access_error(error)
    }
}

impl<X: FrameExchange> ServicePort for RemoteTransport<X> {
    fn connect(&mut self) -> Result<ClientId, ServiceControlErrorV1> {
        match self.call(&WireRequestV1::Connect) {
            Ok(WireReplyV1::Connected(client)) => Ok(client),
            Ok(WireReplyV1::ConnectRefused(refusal)) => {
                Err(ServiceControlErrorV1::Refused(refusal))
            }
            Ok(_) => self.unexpected().map_err(ServiceControlErrorV1::Access),
            Err(error) => Err(ServiceControlErrorV1::Access(error)),
        }
    }

    fn execute(&mut self, request: RequestV1) -> Result<ReplyV1, ServiceAccessErrorV1> {
        match self.call(&WireRequestV1::Execute(request))? {
            WireReplyV1::Executed(reply) => Ok(reply),
            _ => self.unexpected(),
        }
    }

    fn execute_durable(
        &mut self,
        request: DurableExecuteRequestV1,
    ) -> Result<DurableExecuteOutcomeV1, ServiceAccessErrorV1> {
        match self.call(&WireRequestV1::DurableExecute(request))? {
            WireReplyV1::DurableExecuted(outcome) => Ok(outcome),
            _ => self.unexpected(),
        }
    }

    fn lookup_durable_receipt(
        &mut self,
        lookup: DurableReceiptLookupV1,
    ) -> Result<DurableReceiptLookupOutcomeV1, ServiceAccessErrorV1> {
        match self.call(&WireRequestV1::DurableReceiptLookup(lookup))? {
            WireReplyV1::DurableReceiptLookedUp(outcome) => Ok(outcome),
            _ => self.unexpected(),
        }
    }

    fn retire_durable_receipt(
        &mut self,
        retirement: DurableReceiptRetirementV1,
    ) -> Result<DurableReceiptRetirementOutcomeV1, ServiceAccessErrorV1> {
        match self.call(&WireRequestV1::DurableReceiptRetirement(retirement))? {
            WireReplyV1::DurableReceiptRetired(outcome) => Ok(outcome),
            _ => self.unexpected(),
        }
    }

    fn durable_receipt_status(
        &mut self,
        request: DurableReceiptStatusRequestV1,
    ) -> Result<DurableReceiptStatusV1, ServiceAccessErrorV1> {
        match self.call(&WireRequestV1::DurableReceiptStatus(request))? {
            WireReplyV1::DurableReceiptStatus(status) => Ok(status),
            _ => self.unexpected(),
        }
    }

    fn disconnect(&mut self, client: ClientId) -> Result<(), ServiceControlErrorV1> {
        match self.call(&WireRequestV1::Disconnect(client)) {
            Ok(WireReplyV1::Disconnected) => Ok(()),
            Ok(WireReplyV1::DisconnectRefused(refusal)) => {
                Err(ServiceControlErrorV1::Refused(refusal))
            }
            Ok(_) => self.unexpected().map_err(ServiceControlErrorV1::Access),
            Err(error) => Err(ServiceControlErrorV1::Access(error)),
        }
    }
}

pub fn dispatch_v1<S: ServicePort>(
    service: &mut S,
    request: &[u8],
    limits: WireLimits,
) -> Result<Vec<u8>, TransportError> {
    let request = decode_request_v1(request, limits).map_err(map_decode)?;
    let reply = match request {
        WireRequestV1::Connect => match service.connect() {
            Ok(client) => WireReplyV1::Connected(client),
            Err(ServiceControlErrorV1::Refused(refusal)) => WireReplyV1::ConnectRefused(refusal),
            Err(ServiceControlErrorV1::Access(_)) => {
                return Err(TransportError::ServiceUnavailable);
            }
        },
        WireRequestV1::Execute(request) => match service.execute(request) {
            Ok(reply) => WireReplyV1::Executed(reply),
            Err(_) => return Err(TransportError::ServiceUnavailable),
        },
        WireRequestV1::DurableExecute(request) => match service.execute_durable(request) {
            Ok(outcome) => WireReplyV1::DurableExecuted(outcome),
            Err(_) => return Err(TransportError::ServiceUnavailable),
        },
        WireRequestV1::DurableReceiptLookup(lookup) => {
            match service.lookup_durable_receipt(lookup) {
                Ok(outcome) => WireReplyV1::DurableReceiptLookedUp(outcome),
                Err(_) => return Err(TransportError::ServiceUnavailable),
            }
        }
        WireRequestV1::DurableReceiptRetirement(retirement) => {
            match service.retire_durable_receipt(retirement) {
                Ok(outcome) => WireReplyV1::DurableReceiptRetired(outcome),
                Err(_) => return Err(TransportError::ServiceUnavailable),
            }
        }
        WireRequestV1::DurableReceiptStatus(request) => {
            match service.durable_receipt_status(request) {
                Ok(status) => WireReplyV1::DurableReceiptStatus(status),
                Err(_) => return Err(TransportError::ServiceUnavailable),
            }
        }
        WireRequestV1::Disconnect(client) => match service.disconnect(client) {
            Ok(()) => WireReplyV1::Disconnected,
            Err(ServiceControlErrorV1::Refused(refusal)) => WireReplyV1::DisconnectRefused(refusal),
            Err(ServiceControlErrorV1::Access(_)) => {
                return Err(TransportError::ServiceUnavailable);
            }
        },
    };
    encode_reply_v1(&reply, limits).map_err(map_encode)
}

pub fn serve_one<S: ServicePort, T: Read + Write>(
    service: &mut S,
    stream: &mut T,
    limits: TransportLimits,
) -> Result<(), TransportError> {
    let request = read_frame(stream, limits.max_frame_bytes)?;
    let reply = dispatch_v1(service, &request, limits.wire)?;
    write_frame(stream, &reply, limits.max_frame_bytes)
}

pub fn read_frame<R: Read>(reader: &mut R, max_bytes: usize) -> Result<Vec<u8>, TransportError> {
    let mut length = [0_u8; 4];
    reader.read_exact(&mut length).map_err(map_io)?;
    let length =
        usize::try_from(u32::from_be_bytes(length)).map_err(|_| TransportError::LengthOverflow)?;
    if length > max_bytes {
        return Err(TransportError::FrameLimit);
    }
    let mut frame = Vec::new();
    frame
        .try_reserve_exact(length)
        .map_err(|_| TransportError::Capacity)?;
    frame.resize(length, 0);
    reader.read_exact(&mut frame).map_err(map_io)?;
    Ok(frame)
}

pub fn write_frame<W: Write>(
    writer: &mut W,
    frame: &[u8],
    max_bytes: usize,
) -> Result<(), TransportError> {
    if frame.len() > max_bytes {
        return Err(TransportError::FrameLimit);
    }
    let length = u32::try_from(frame.len()).map_err(|_| TransportError::LengthOverflow)?;
    let mut sent = 0_usize;
    write_all_phase(writer, &length.to_be_bytes(), &mut sent)?;
    write_all_phase(writer, frame, &mut sent)?;
    writer.flush().map_err(|_| TransportError::OutcomeUnknown)
}

#[cfg(unix)]
pub struct UnixConnector {
    path: std::path::PathBuf,
    timeout: std::time::Duration,
}

/// Opaque proof bytes offered by a local authenticated connector.
///
/// The bytes have no authority semantics. Concrete credential adapters derive
/// them from the exact server challenge and session; this transport only
/// carries the result before the ordinary Vellum frame.
#[cfg(unix)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct LocalAuthenticationClaimV1([u8; 16]);

#[cfg(unix)]
impl LocalAuthenticationClaimV1 {
    pub const fn new(value: [u8; 16]) -> Self {
        Self(value)
    }

    /// Returns the opaque proof bytes for a transport handshake. They remain
    /// neither credential material nor authority evidence.
    pub const fn to_bytes(self) -> [u8; 16] {
        self.0
    }
}

#[cfg(unix)]
impl std::fmt::Debug for LocalAuthenticationClaimV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("LocalAuthenticationClaimV1(<opaque>)")
    }
}

/// Content-free refusal to derive a local authentication claim.
#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalAuthenticationClaimError {
    Unavailable,
}

/// Adapter-owned derivation of one connection-local authentication claim.
/// Refusal is deliberately content-free and occurs before any Vellum request
/// byte is sent.
#[cfg(unix)]
pub trait LocalAuthenticationClaimProvider {
    fn claim(
        &mut self,
        challenge: [u8; 16],
        session: [u8; 16],
    ) -> Result<LocalAuthenticationClaimV1, LocalAuthenticationClaimError>;
}

#[cfg(unix)]
impl LocalAuthenticationClaimProvider for LocalAuthenticationClaimV1 {
    fn claim(
        &mut self,
        _challenge: [u8; 16],
        _session: [u8; 16],
    ) -> Result<LocalAuthenticationClaimV1, LocalAuthenticationClaimError> {
        Ok(*self)
    }
}

#[cfg(unix)]
const AUTHENTICATION_HANDSHAKE_VERSION_V1: u8 = 1;
#[cfg(unix)]
const AUTHENTICATION_HELLO_BYTES_V1: usize = 33;
#[cfg(unix)]
const AUTHENTICATION_RESPONSE_BYTES_V1: usize = 49;
#[cfg(unix)]
const AUTHENTICATION_RESULT_BYTES_V1: usize = 2;

/// Unix connector for the daemon's required local-authentication profile.
///
/// Credential verification remains an adapter responsibility; this connector
/// owns only the fixed-size transport prelude and request/reply phase truth.
#[cfg(unix)]
pub struct AuthenticatedUnixConnector {
    path: std::path::PathBuf,
    timeout: std::time::Duration,
    provider: Box<dyn LocalAuthenticationClaimProvider>,
}

#[cfg(unix)]
impl AuthenticatedUnixConnector {
    pub fn new(path: std::path::PathBuf, claim: LocalAuthenticationClaimV1) -> Self {
        Self::with_provider(path, claim)
    }

    pub fn with_timeout(
        path: std::path::PathBuf,
        claim: LocalAuthenticationClaimV1,
        timeout: std::time::Duration,
    ) -> Result<Self, TransportError> {
        Self::with_provider_and_timeout(path, claim, timeout).map_err(|_| TransportError::NotSent)
    }

    pub fn with_provider(
        path: std::path::PathBuf,
        provider: impl LocalAuthenticationClaimProvider + 'static,
    ) -> Self {
        Self {
            path,
            timeout: std::time::Duration::from_secs(5),
            provider: Box::new(provider),
        }
    }

    pub fn with_provider_and_timeout(
        path: std::path::PathBuf,
        provider: impl LocalAuthenticationClaimProvider + 'static,
        timeout: std::time::Duration,
    ) -> Result<Self, TransportBuildError> {
        if timeout.is_zero() {
            return Err(TransportBuildError::InvalidLimits);
        }
        Ok(Self {
            path,
            timeout,
            provider: Box::new(provider),
        })
    }
}

#[cfg(unix)]
impl FrameExchange for AuthenticatedUnixConnector {
    fn exchange(
        &mut self,
        request: &[u8],
        max_reply_bytes: usize,
    ) -> Result<Vec<u8>, TransportError> {
        use std::os::unix::net::UnixStream;

        let mut stream = UnixStream::connect(&self.path).map_err(|_| TransportError::NotSent)?;
        stream
            .set_read_timeout(Some(self.timeout))
            .and_then(|()| stream.set_write_timeout(Some(self.timeout)))
            .map_err(|_| TransportError::NotSent)?;
        let mut hello = [0_u8; AUTHENTICATION_HELLO_BYTES_V1];
        stream
            .read_exact(&mut hello)
            .map_err(|_| TransportError::NotSent)?;
        if hello[0] != AUTHENTICATION_HANDSHAKE_VERSION_V1 {
            return Err(TransportError::NotSent);
        }
        let mut challenge = [0_u8; 16];
        challenge.copy_from_slice(&hello[1..17]);
        let mut session = [0_u8; 16];
        session.copy_from_slice(&hello[17..33]);
        let claim = self
            .provider
            .claim(challenge, session)
            .map_err(|_| TransportError::NotSent)?;
        let mut response = [0_u8; AUTHENTICATION_RESPONSE_BYTES_V1];
        response[0] = AUTHENTICATION_HANDSHAKE_VERSION_V1;
        response[1..33].copy_from_slice(&hello[1..33]);
        response[33..49].copy_from_slice(&claim.0);
        stream
            .write_all(&response)
            .and_then(|()| stream.flush())
            .map_err(|_| TransportError::NotSent)?;
        let mut result = [0_u8; AUTHENTICATION_RESULT_BYTES_V1];
        stream
            .read_exact(&mut result)
            .map_err(|_| TransportError::NotSent)?;
        if result != [AUTHENTICATION_HANDSHAKE_VERSION_V1, 1] {
            return Err(TransportError::NotSent);
        }
        write_frame(&mut stream, request, max_reply_bytes)?;
        read_frame(&mut stream, max_reply_bytes).map_err(|_| TransportError::OutcomeUnknown)
    }
}

#[cfg(unix)]
impl UnixConnector {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self {
            path,
            timeout: std::time::Duration::from_secs(5),
        }
    }

    pub fn with_timeout(
        path: std::path::PathBuf,
        timeout: std::time::Duration,
    ) -> Result<Self, TransportBuildError> {
        if timeout.is_zero() {
            return Err(TransportBuildError::InvalidLimits);
        }
        Ok(Self { path, timeout })
    }
}

#[cfg(unix)]
impl FrameExchange for UnixConnector {
    fn exchange(
        &mut self,
        request: &[u8],
        max_reply_bytes: usize,
    ) -> Result<Vec<u8>, TransportError> {
        let mut stream = std::os::unix::net::UnixStream::connect(&self.path)
            .map_err(|_| TransportError::NotSent)?;
        stream
            .set_read_timeout(Some(self.timeout))
            .map_err(|_| TransportError::NotSent)?;
        stream
            .set_write_timeout(Some(self.timeout))
            .map_err(|_| TransportError::NotSent)?;
        write_frame(&mut stream, request, max_reply_bytes)?;
        read_frame(&mut stream, max_reply_bytes).map_err(|_| TransportError::OutcomeUnknown)
    }
}

fn write_all_phase<W: Write>(
    writer: &mut W,
    mut bytes: &[u8],
    sent: &mut usize,
) -> Result<(), TransportError> {
    while !bytes.is_empty() {
        match writer.write(bytes) {
            Ok(0) => {
                return Err(if *sent == 0 {
                    TransportError::NotSent
                } else {
                    TransportError::OutcomeUnknown
                });
            }
            Ok(written) => {
                if written > bytes.len() {
                    return Err(TransportError::OutcomeUnknown);
                }
                *sent = sent.saturating_add(written);
                bytes = &bytes[written..];
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => {
                return Err(if *sent == 0 {
                    TransportError::NotSent
                } else {
                    TransportError::OutcomeUnknown
                });
            }
        }
    }
    Ok(())
}

fn map_io(error: io::Error) -> TransportError {
    if error.kind() == io::ErrorKind::UnexpectedEof {
        TransportError::Truncated
    } else {
        TransportError::Io
    }
}

fn map_encode(error: WireEncodeError) -> TransportError {
    match error {
        WireEncodeError::Limit => TransportError::FrameLimit,
        WireEncodeError::Capacity => TransportError::Capacity,
        WireEncodeError::LengthOverflow => TransportError::LengthOverflow,
        WireEncodeError::IdentityMismatch => TransportError::Malformed,
    }
}

fn map_decode(error: WireDecodeError) -> TransportError {
    match error {
        WireDecodeError::Limit => TransportError::FrameLimit,
        WireDecodeError::Capacity => TransportError::Capacity,
        WireDecodeError::Malformed
        | WireDecodeError::UnsupportedVersion
        | WireDecodeError::UnknownTag
        | WireDecodeError::InvalidIdentity
        | WireDecodeError::InvalidUtf8
        | WireDecodeError::TrailingData => TransportError::Malformed,
    }
}

fn access_error(error: TransportError) -> ServiceAccessErrorV1 {
    match error {
        TransportError::NotSent => ServiceAccessErrorV1::NotSent,
        TransportError::OutcomeUnknown => ServiceAccessErrorV1::OutcomeUnknown,
        TransportError::Io | TransportError::Truncated | TransportError::ServiceUnavailable => {
            ServiceAccessErrorV1::Unavailable
        }
        TransportError::FrameLimit | TransportError::LengthOverflow | TransportError::Capacity => {
            ServiceAccessErrorV1::Limit
        }
        TransportError::Malformed | TransportError::UnexpectedReply => {
            ServiceAccessErrorV1::Malformed
        }
    }
}

fn bump(value: &mut u64, saturated: &mut bool) {
    let next = value.saturating_add(1);
    if next == *value {
        *saturated = true;
    }
    *value = next;
}
