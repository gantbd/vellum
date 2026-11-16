use super::*;
use objc2::MainThreadMarker;
use objc2_app_kit::NSPasteboard;
use objc2_foundation::{NSData, NSString};
use std::panic::AssertUnwindSafe;

/// Main-thread-confined, lazy provider. Construction never accesses a pasteboard.
pub struct AppKitClipboard {
    provider: Provider<AppKit>,
}

impl std::fmt::Debug for AppKitClipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AppKitClipboard(<redacted>)")
    }
}

impl AppKitClipboard {
    pub fn new(limits: Limits, timed: bool) -> Result<Self, Error> {
        Ok(Self {
            provider: Provider::new(AppKit::new()?, limits, timed)?,
        })
    }

    pub fn export(&mut self, text: &str) -> ExportOutcome {
        objc2::rc::autoreleasepool(|_| self.provider.export(text))
    }

    /// `None` means AppKit returned no UTF-8 plain data, not proof of an empty
    /// clipboard. A present empty payload remains `Some(PlainText)`.
    pub fn read(&mut self) -> Result<Option<PlainText>, Error> {
        objc2::rc::autoreleasepool(|_| self.provider.read().map(|value| value.map(PlainText)))
    }

    /// Content/identity-free counters; querying never probes the pasteboard.
    pub fn usage(&self) -> Usage {
        self.provider.usage
    }
}

#[doc(hidden)]
pub struct AppKit {
    _thread: MainThreadMarker,
}

impl AppKit {
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            _thread: MainThreadMarker::new().ok_or(Error::Unavailable)?,
        })
    }
}

impl Backend for AppKit {
    fn write(&mut self, text: &str) -> Result<bool, BackendWriteError> {
        objc2::exception::catch(AssertUnwindSafe(|| {
            let board = NSPasteboard::generalPasteboard();
            // Publish the standard UTF-8 plain-text representation as exact
            // bytes, without a decoder treating leading U+FEFF as metadata.
            let data = NSData::with_bytes(text.as_bytes());
            let kind = NSString::from_str("public.utf8-plain-text");
            board.clearContents();
            Ok(board.setData_forType(Some(&data), &kind))
        }))
        .map_err(|_| BackendWriteError)?
    }

    fn read(&mut self, max_bytes: usize) -> Result<Option<String>, Error> {
        objc2::exception::catch(AssertUnwindSafe(|| {
            let board = NSPasteboard::generalPasteboard();
            let before = board.changeCount();
            let kind = NSString::from_str("public.utf8-plain-text");
            let value = board.dataForType(&kind);
            let text = if let Some(value) = value {
                Some(bounded_plain_text(&value, max_bytes)?)
            } else {
                None
            };
            if board.changeCount() != before {
                return Err(Error::ChangedDuringRead);
            }
            Ok(text)
        }))
        .map_err(|_| Error::Unavailable)?
    }
}

fn bounded_plain_text(data: &NSData, max_bytes: usize) -> Result<String, Error> {
    // Check before making a Rust-owned copy. OS-side materialization, allocator
    // overhead and IPC time are outside this logical byte bound.
    if data.len() > max_bytes {
        return Err(Error::Limit);
    }
    String::from_utf8(data.to_vec()).map_err(|_| Error::InvalidText)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Foundation values only: neither test obtains any NSPasteboard instance.
    #[test]
    fn foundation_utf8_round_trip_preserves_raw_plain_text_without_lossy_conversion() {
        for raw in [
            "",
            "\u{feff}Grüße\r\n\0\x1b$(literal)",
            "\u{feff}\u{feff}",
            "e\u{301}👩‍💻",
        ] {
            let native = NSData::with_bytes(raw.as_bytes());
            assert_eq!(bounded_plain_text(&native, raw.len()).unwrap(), raw);
        }
    }

    #[test]
    fn utf8_limits_and_invalid_data_refuse_without_truncation_or_replacement() {
        let ascii = NSData::with_bytes(b"AB");
        assert_eq!(bounded_plain_text(&ascii, 1), Err(Error::Limit));
        let native = NSData::with_bytes("界".as_bytes());
        assert_eq!(bounded_plain_text(&native, 2), Err(Error::Limit));
        assert_eq!(bounded_plain_text(&native, 3).unwrap(), "界");
        assert_eq!(
            bounded_plain_text(&NSData::with_bytes(&[0xff]), 1),
            Err(Error::InvalidText)
        );
    }
}
