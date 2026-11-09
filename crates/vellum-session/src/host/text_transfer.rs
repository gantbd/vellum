//! Source-neutral trusted-host capture for the adopted plain transfer contract.
//! This is NOT a clipboard task, provider grant, structured handle or receipt.
use super::*;
use vellum_engine::coordinator::{InvocationId, InvocationState, LogicalViewContext};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextTransferCaptureLimitsV1 {
    pub max_fragments: usize,
    pub max_plain_bytes: usize,
    /// Owned value headers and copied UTF-8 payloads, not allocator/RSS peak.
    pub max_logical_bytes: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextTransferCaptureErrorV1 {
    Unavailable,
    Stale,
    Limit,
    Capacity,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextTransferCaptureUsageV1 {
    pub fragments: usize,
    pub fragment_bytes: usize,
    pub plain_bytes: usize,
    pub logical_bytes: usize,
}
/// Owned inert data disclosed to the trusted embedding host. Holding this is
/// neither provider-release authority nor session-structured clipboard identity.
/// Per-capture bounds do not prove a bound on all captures retained by a host.
pub struct TextTransferCaptureV1 {
    origin: InvocationId,
    context: LogicalViewContext,
    selections: vellum_engine::selection::SelectionSet,
    fragments: Vec<String>,
    plain: Option<String>,
    usage: TextTransferCaptureUsageV1,
}
impl std::fmt::Debug for TextTransferCaptureV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TextTransferCaptureV1(<redacted>)")
    }
}
impl TextTransferCaptureV1 {
    pub fn origin(&self) -> &InvocationId {
        &self.origin
    }
    pub fn context(&self) -> &LogicalViewContext {
        &self.context
    }
    pub fn fragments(&self) -> &[String] {
        &self.fragments
    }
    pub fn selections(&self) -> &vellum_engine::selection::SelectionSet {
        &self.selections
    }
    /// None means all captured ranges were empty: do not clear a clipboard,
    /// expand whole lines or mistake separator bytes for authored content.
    pub fn plain(&self) -> Option<&str> {
        self.plain.as_deref()
    }
    pub fn usage(&self) -> TextTransferCaptureUsageV1 {
        self.usage
    }
}
fn failure(error: TextTransferCaptureErrorV1) -> SessionError {
    SessionError::TextTransfer(error)
}
fn add(a: usize, b: usize) -> Result<usize, SessionError> {
    a.checked_add(b)
        .ok_or(failure(TextTransferCaptureErrorV1::Limit))
}
impl LocalHost {
    /// Capture only. No request/task issuance, provider I/O, source/history
    /// publication or invocation terminalization follows. The actual native ID
    /// must have been issued by this owner before the future transfer request.
    /// Rights here are the same privileged trusted-host disclosure as `read`;
    /// a mediated provider/recipient boundary must reauthorize release separately.
    pub fn capture_text_transfer(
        &mut self,
        origin: &InvocationId,
        context: &LogicalViewContext,
        limits: TextTransferCaptureLimitsV1,
    ) -> Result<TextTransferCaptureV1, SessionError> {
        if self.owner.invocation_state(origin, &mut self.diagnostics)? != InvocationState::Issued {
            return Err(failure(TextTransferCaptureErrorV1::Unavailable));
        }
        let snapshot = self
            .owner
            .logical_view(&context.view, &mut self.diagnostics)?;
        if snapshot.context().as_ref() != Some(context) {
            return Err(failure(TextTransferCaptureErrorV1::Stale));
        }
        let count = snapshot.selections.selections().len();
        if count > limits.max_fragments {
            return Err(failure(TextTransferCaptureErrorV1::Limit));
        }
        let read = self
            .owner
            .read(&context.source.document, &mut self.diagnostics)?;
        if read.version != context.source {
            return Err(failure(TextTransferCaptureErrorV1::Stale));
        }
        let fragment_bytes = snapshot
            .selections
            .selections()
            .iter()
            .try_fold(0, |n, s| add(n, s.anchor.0.abs_diff(s.head.0)))?;
        let plain_bytes = if fragment_bytes == 0 {
            0
        } else {
            add(fragment_bytes, count.saturating_sub(1))?
        };
        let headers = count
            .checked_mul(add(
                std::mem::size_of::<String>(),
                std::mem::size_of::<vellum_engine::selection::Selection>(),
            )?)
            .ok_or(failure(TextTransferCaptureErrorV1::Limit))?;
        let logical_bytes = add(
            add(
                add(std::mem::size_of::<TextTransferCaptureV1>(), headers)?,
                fragment_bytes,
            )?,
            plain_bytes,
        )?;
        if plain_bytes > limits.max_plain_bytes || logical_bytes > limits.max_logical_bytes {
            return Err(failure(TextTransferCaptureErrorV1::Limit));
        }
        let mut fragments = Vec::new();
        fragments
            .try_reserve_exact(count)
            .map_err(|_| failure(TextTransferCaptureErrorV1::Capacity))?;
        let mut plain = String::new();
        plain
            .try_reserve_exact(plain_bytes)
            .map_err(|_| failure(TextTransferCaptureErrorV1::Capacity))?;
        for (index, selection) in snapshot.selections.selections().iter().enumerate() {
            let start = selection.anchor.0.min(selection.head.0);
            let end = selection.anchor.0.max(selection.head.0);
            // Native view validation has already checked these UTF-8 endpoints.
            let text = &read.source.text()[start..end];
            let mut fragment = String::new();
            fragment
                .try_reserve_exact(text.len())
                .map_err(|_| failure(TextTransferCaptureErrorV1::Capacity))?;
            fragment.push_str(text);
            fragments.push(fragment);
            if fragment_bytes > 0 {
                if index > 0 {
                    plain.push('\n');
                }
                plain.push_str(text);
            }
        }
        Ok(TextTransferCaptureV1 {
            origin: origin.clone(),
            context: context.clone(),
            selections: snapshot.selections,
            fragments,
            plain: (fragment_bytes > 0).then_some(plain),
            usage: TextTransferCaptureUsageV1 {
                fragments: count,
                fragment_bytes,
                plain_bytes,
                logical_bytes,
            },
        })
    }
}
