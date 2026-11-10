//! Bounded request/reply framing and remote Vellum service transport.
#![forbid(unsafe_code)]

use std::io::{self, Read, Write};
use vellum_protocol::{
    ClientId, ReplyV1, RequestV1, ServiceAccessErrorV1, ServiceControlErrorV1, ServicePort,
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
    Io,
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
    pub counter_saturated: bool,
}

pub trait FrameExchange {
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
            .map_err(map_decode)
            .map_err(|error| self.failed(error))?;
        bump(&mut self.usage.completed, &mut self.usage.counter_saturated);
        Ok(decoded)
    }

    fn unexpected<T>(&mut self) -> Result<T, ServiceAccessErrorV1> {
        Err(self.failed(TransportError::UnexpectedReply))
    }

    fn failed(&mut self, error: TransportError) -> ServiceAccessErrorV1 {
        let category = match error {
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
    writer.write_all(&length.to_be_bytes()).map_err(map_io)?;
    writer.write_all(frame).map_err(map_io)?;
    writer.flush().map_err(map_io)
}

#[cfg(unix)]
pub struct UnixConnector {
    path: std::path::PathBuf,
    timeout: std::time::Duration,
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
        let mut stream = std::os::unix::net::UnixStream::connect(&self.path).map_err(map_io)?;
        stream
            .set_read_timeout(Some(self.timeout))
            .map_err(map_io)?;
        stream
            .set_write_timeout(Some(self.timeout))
            .map_err(map_io)?;
        write_frame(&mut stream, request, max_reply_bytes)?;
        read_frame(&mut stream, max_reply_bytes)
    }
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
