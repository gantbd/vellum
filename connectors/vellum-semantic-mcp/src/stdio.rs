use std::io;

use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};
use vellum_sdk_semantic_client::client as vellum_semantic_client;
use vellum_semantic_client::SemanticServicePortV1;

use crate::SemanticMcpServerV1;

/// Physical newline-frame limits for one stdio peer.
///
/// This narrows the projection's JSON request ceiling; it does not bound peer
/// lifetime or claim that an operating-system pipe wrote or read atomically.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticMcpStdioLimitsV1 {
    pub max_frame_bytes: usize,
}

impl SemanticMcpStdioLimitsV1 {
    pub const fn reference() -> Self {
        Self {
            max_frame_bytes: 256 * 1024,
        }
    }

    const fn is_valid(self, request_limit: usize) -> bool {
        self.max_frame_bytes > 0 && self.max_frame_bytes <= request_limit
    }
}

/// Fixed-cardinality, content-free observations from one stdio lifetime.
///
/// Byte counters describe newline-frame content presented to or accepted from
/// the MCP projection. They are not kernel-I/O, allocation, latency, health,
/// confidentiality, or durable-delivery evidence.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SemanticMcpStdioUsageV1 {
    pub frames: u64,
    pub frame_bytes: u64,
    pub responses: u64,
    pub response_bytes: u64,
    pub notifications: u64,
    pub oversized_frames: u64,
    pub truncated_frames: u64,
    pub io_failures: u64,
    pub saturated: bool,
}

#[derive(Debug)]
pub enum SemanticMcpStdioFailureV1 {
    InvalidLimits,
    Io {
        error: io::Error,
        usage: SemanticMcpStdioUsageV1,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticMcpStdioStepV1 {
    Served,
    Eof,
}

impl SemanticMcpStdioFailureV1 {
    pub const fn usage(&self) -> Option<SemanticMcpStdioUsageV1> {
        match self {
            Self::InvalidLimits => None,
            Self::Io { usage, .. } => Some(*usage),
        }
    }

    pub const fn io_error(&self) -> Option<&io::Error> {
        match self {
            Self::InvalidLimits => None,
            Self::Io { error, .. } => Some(error),
        }
    }
}

/// One fully drained physical frame. A composition may inspect only the shape
/// and regain synchronization authority before executing `Complete` bytes.
#[derive(Debug)]
pub enum SemanticMcpStdioFrameV1 {
    Eof,
    Complete { bytes: Vec<u8>, observed: usize },
    Oversized { observed: usize },
    Truncated { observed: usize },
}

/// Serves newline-delimited MCP JSON-RPC until clean EOF.
///
/// Requests are processed sequentially. A response is flushed before the next
/// frame is read. Oversized frames are drained with bounded retained memory and
/// rejected by the ordinary MCP projection; a nonempty EOF fragment is never
/// executed, even when it happens to contain valid JSON.
pub async fn serve_stdio_v1<P, R, W>(
    server: &mut SemanticMcpServerV1<P>,
    reader: &mut R,
    writer: &mut W,
    limits: SemanticMcpStdioLimitsV1,
) -> Result<SemanticMcpStdioUsageV1, SemanticMcpStdioFailureV1>
where
    P: SemanticServicePortV1,
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    if !limits.is_valid(server.limits().max_request_bytes) {
        return Err(SemanticMcpStdioFailureV1::InvalidLimits);
    }
    let mut usage = SemanticMcpStdioUsageV1::default();
    loop {
        if serve_stdio_frame_v1(server, reader, writer, limits, &mut usage).await?
            == SemanticMcpStdioStepV1::Eof
        {
            return Ok(usage);
        }
    }
}

/// Serves at most one complete frame and returns control to the composition
/// root before the next read. This is the request-bound synchronization seam;
/// it owns no polling, provider, workspace, or retry policy.
pub async fn serve_stdio_frame_v1<P, R, W>(
    server: &mut SemanticMcpServerV1<P>,
    reader: &mut R,
    writer: &mut W,
    limits: SemanticMcpStdioLimitsV1,
    usage: &mut SemanticMcpStdioUsageV1,
) -> Result<SemanticMcpStdioStepV1, SemanticMcpStdioFailureV1>
where
    P: SemanticServicePortV1,
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    if !limits.is_valid(server.limits().max_request_bytes) {
        return Err(SemanticMcpStdioFailureV1::InvalidLimits);
    }
    let frame =
        read_stdio_frame_v1(reader, limits, server.limits().max_request_bytes, usage).await?;
    serve_stdio_read_frame_v1(server, frame, writer, usage).await
}

/// Reads and drains exactly one frame without executing it. This locates the
/// synchronization seam after request arrival and before semantic effects.
pub async fn read_stdio_frame_v1<R>(
    reader: &mut R,
    limits: SemanticMcpStdioLimitsV1,
    max_request_bytes: usize,
    usage: &mut SemanticMcpStdioUsageV1,
) -> Result<SemanticMcpStdioFrameV1, SemanticMcpStdioFailureV1>
where
    R: AsyncBufRead + Unpin,
{
    if !limits.is_valid(max_request_bytes) {
        return Err(SemanticMcpStdioFailureV1::InvalidLimits);
    }
    match read_frame(reader, limits.max_frame_bytes).await {
        Ok(frame) => Ok(frame),
        Err(error) => Err(io_failure(error, *usage)),
    }
}

/// Executes one already-drained frame against the current exact MCP owner.
pub async fn serve_stdio_read_frame_v1<P, W>(
    server: &mut SemanticMcpServerV1<P>,
    frame: SemanticMcpStdioFrameV1,
    writer: &mut W,
    usage: &mut SemanticMcpStdioUsageV1,
) -> Result<SemanticMcpStdioStepV1, SemanticMcpStdioFailureV1>
where
    P: SemanticServicePortV1,
    W: AsyncWrite + Unpin,
{
    let response = match frame {
        SemanticMcpStdioFrameV1::Eof => return Ok(SemanticMcpStdioStepV1::Eof),
        SemanticMcpStdioFrameV1::Complete { bytes, observed } => {
            account_frame(usage, observed);
            server.handle(&bytes).await
        }
        SemanticMcpStdioFrameV1::Oversized { observed } => {
            account_frame(usage, observed);
            bump(&mut usage.oversized_frames, &mut usage.saturated);
            Some(server.reject_oversized_frame(observed))
        }
        SemanticMcpStdioFrameV1::Truncated { observed } => {
            account_frame(usage, observed);
            bump(&mut usage.truncated_frames, &mut usage.saturated);
            Some(server.reject_truncated_frame(observed))
        }
    };
    let Some(response) = response else {
        bump(&mut usage.notifications, &mut usage.saturated);
        return Ok(SemanticMcpStdioStepV1::Served);
    };
    if let Err(error) = write_response(writer, &response).await {
        bump(&mut usage.io_failures, &mut usage.saturated);
        return Err(SemanticMcpStdioFailureV1::Io {
            error,
            usage: *usage,
        });
    }
    bump(&mut usage.responses, &mut usage.saturated);
    add(
        &mut usage.response_bytes,
        response.len(),
        &mut usage.saturated,
    );
    Ok(SemanticMcpStdioStepV1::Served)
}

async fn read_frame<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    limit: usize,
) -> io::Result<SemanticMcpStdioFrameV1> {
    let retained_limit = limit.saturating_add(1);
    let mut retained = Vec::new();
    let mut observed = 0usize;
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            return Ok(if observed == 0 {
                SemanticMcpStdioFrameV1::Eof
            } else {
                SemanticMcpStdioFrameV1::Truncated { observed }
            });
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let content = newline.unwrap_or(available.len());
        observed = observed.saturating_add(content);
        if retained.len() < retained_limit {
            let take = content.min(retained_limit - retained.len());
            retained.extend_from_slice(&available[..take]);
        }
        let consumed = newline.map_or(available.len(), |index| index + 1);
        reader.consume(consumed);
        if newline.is_some() {
            return Ok(if observed > limit {
                SemanticMcpStdioFrameV1::Oversized { observed }
            } else {
                SemanticMcpStdioFrameV1::Complete {
                    bytes: retained,
                    observed,
                }
            });
        }
    }
}

async fn write_response<W: AsyncWrite + Unpin>(writer: &mut W, response: &[u8]) -> io::Result<()> {
    writer.write_all(response).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await
}

fn account_frame(usage: &mut SemanticMcpStdioUsageV1, bytes: usize) {
    bump(&mut usage.frames, &mut usage.saturated);
    add(&mut usage.frame_bytes, bytes, &mut usage.saturated);
}

fn io_failure(error: io::Error, mut usage: SemanticMcpStdioUsageV1) -> SemanticMcpStdioFailureV1 {
    bump(&mut usage.io_failures, &mut usage.saturated);
    SemanticMcpStdioFailureV1::Io { error, usage }
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
