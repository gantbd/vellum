//! Eager plain-text clipboard I/O and original-operation integration, not a
//! semantic publisher, authority evaluator or second native task registry.
//!
//! `EditorClipboard` orchestrates admission, one dispatch, retained receipts and
//! separately authorized publication through the existing Editor facade. Call
//! raw `AppKitClipboard` only after that facade releases the original payload;
//! feed results into the same native owner. Do not
//! infer cut deletion or retry permission from these provider values. Acceptance
//! is AppKit acceptance, not persistence or delivery to another application.
//! This module does not read the clipboard during construction or observation.
#![forbid(unsafe_code)]
#![cfg(target_os = "macos")]

use std::time::{Duration, Instant};
mod integration;
#[cfg(feature = "private-pasteboard-qualification")]
pub use integration::qualify_private_pasteboard;
pub use integration::{
    Admission, ClipboardStage, DesktopAccess, DesktopResult, EditorClipboard, IntegrationError,
    ProviderReceiptSummary, Publication, ReferenceClipboard, reference_session_config,
};
pub use vellum_session::host::copy_transfer::TextTransferKindV1 as Operation;

#[cfg(target_os = "macos")]
mod appkit;
#[cfg(target_os = "macos")]
pub use appkit::AppKitClipboard;

pub const ACKNOWLEDGMENT_PROFILE: &str = "macos.appkit.eager-plain/1";

/// Owned insertion data. Debug is redacted; payload access is explicit.
pub struct PlainText(String);

impl std::fmt::Debug for PlainText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PlainText(<redacted>)")
    }
}

impl PlainText {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    /// Maximum Rust-owned payload bytes, not OS materialization or IPC time.
    pub max_plain_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Limit,
    Unavailable,
    InvalidText,
    ChangedDuringRead,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportOutcome {
    /// The eager data setter returned true. This is not durability evidence.
    Accepted,
    /// Validation failed before clipboard I/O began.
    Refused(Error),
    /// The setter returned false after clearing prior contents. No cut is allowed.
    RejectedAfterClear,
    /// A platform exception leaves the effect uncertain. Never automatically retry.
    Unknown,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Usage {
    pub export_offers: u64,
    pub export_accepted: u64,
    pub export_refused: u64,
    pub export_rejected: u64,
    pub export_unknown: u64,
    pub read_offers: u64,
    pub read_accepted: u64,
    /// AppKit returned no plain-text value; not proof of an empty clipboard.
    pub read_no_data: u64,
    pub read_refused: u64,
    pub export_timed_samples: u64,
    pub export_untimed_samples: u64,
    pub read_timed_samples: u64,
    pub read_untimed_samples: u64,
    /// Measured samples only; counters above disclose missing timing coverage.
    pub export_service_total: Option<Duration>,
    pub read_service_total: Option<Duration>,
    pub max_export_service: Option<Duration>,
    pub max_read_service: Option<Duration>,
}

// Private platform port permits deterministic fault tests without a general
// pasteboard, a hidden session clipboard, or a production fake provider profile.
trait Backend {
    fn write(&mut self, text: &str) -> Result<bool, ()>;
    fn read(&mut self, max_bytes: usize) -> Result<Option<String>, Error>;
}

struct Provider<B> {
    backend: B,
    limits: Limits,
    timed: bool,
    usage: Usage,
}

impl<B: Backend> Provider<B> {
    fn new(backend: B, limits: Limits, timed: bool) -> Result<Self, Error> {
        if limits.max_plain_bytes == 0 || limits.max_plain_bytes > isize::MAX as usize {
            return Err(Error::Limit);
        }
        Ok(Self {
            backend,
            limits,
            timed,
            usage: Usage::default(),
        })
    }

    fn export(&mut self, text: &str) -> ExportOutcome {
        self.usage.export_offers = self.usage.export_offers.saturating_add(1);
        let start = self.timed.then(clock).flatten();
        let outcome = if text.len() > self.limits.max_plain_bytes {
            ExportOutcome::Refused(Error::Limit)
        } else {
            match self.backend.write(text) {
                Ok(true) => ExportOutcome::Accepted,
                Ok(false) => ExportOutcome::RejectedAfterClear,
                Err(()) => ExportOutcome::Unknown,
            }
        };
        let counter = match outcome {
            ExportOutcome::Accepted => &mut self.usage.export_accepted,
            ExportOutcome::Refused(_) => &mut self.usage.export_refused,
            ExportOutcome::RejectedAfterClear => &mut self.usage.export_rejected,
            ExportOutcome::Unknown => &mut self.usage.export_unknown,
        };
        *counter = counter.saturating_add(1);
        let measured = elapsed(
            start,
            &mut self.usage.export_service_total,
            &mut self.usage.max_export_service,
        );
        let samples = if measured {
            &mut self.usage.export_timed_samples
        } else {
            &mut self.usage.export_untimed_samples
        };
        *samples = samples.saturating_add(1);
        outcome
    }

    fn read(&mut self) -> Result<Option<String>, Error> {
        self.usage.read_offers = self.usage.read_offers.saturating_add(1);
        let start = self.timed.then(clock).flatten();
        let result = self
            .backend
            .read(self.limits.max_plain_bytes)
            .and_then(|value| {
                if value
                    .as_ref()
                    .is_some_and(|s| s.len() > self.limits.max_plain_bytes)
                {
                    Err(Error::Limit)
                } else {
                    Ok(value)
                }
            });
        let counter = match &result {
            Ok(Some(_)) => &mut self.usage.read_accepted,
            Ok(None) => &mut self.usage.read_no_data,
            Err(_) => &mut self.usage.read_refused,
        };
        *counter = counter.saturating_add(1);
        let measured = elapsed(
            start,
            &mut self.usage.read_service_total,
            &mut self.usage.max_read_service,
        );
        let samples = if measured {
            &mut self.usage.read_timed_samples
        } else {
            &mut self.usage.read_untimed_samples
        };
        *samples = samples.saturating_add(1);
        result
    }
}

fn clock() -> Option<Instant> {
    std::panic::catch_unwind(Instant::now).ok()
}

fn elapsed(
    start: Option<Instant>,
    total: &mut Option<Duration>,
    maximum: &mut Option<Duration>,
) -> bool {
    if let Some(duration) = start.and_then(|s| clock()?.checked_duration_since(s)) {
        *total = Some(total.unwrap_or_default().saturating_add(duration));
        *maximum = Some(maximum.unwrap_or_default().max(duration));
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests;
