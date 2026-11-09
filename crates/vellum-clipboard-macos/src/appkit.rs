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

pub(crate) struct AppKit {
    _thread: MainThreadMarker,
    target: Target,
}

enum Target {
    General,
    #[cfg(feature = "private-pasteboard-qualification")]
    Private(Option<objc2::rc::Retained<NSPasteboard>>),
}

impl Target {
    fn board(&self) -> Result<objc2::rc::Retained<NSPasteboard>, Error> {
        match self {
            Self::General => Ok(NSPasteboard::generalPasteboard()),
            #[cfg(feature = "private-pasteboard-qualification")]
            Self::Private(board) => board.clone().ok_or(Error::Unavailable),
        }
    }

    #[cfg(feature = "private-pasteboard-qualification")]
    fn take_private(&mut self) -> Result<objc2::rc::Retained<NSPasteboard>, Error> {
        match self {
            Self::General => Err(Error::Unavailable),
            Self::Private(board) => board.take().ok_or(Error::Unavailable),
        }
    }
}

impl AppKit {
    pub(crate) fn new() -> Result<Self, Error> {
        Ok(Self {
            _thread: MainThreadMarker::new().ok_or(Error::Unavailable)?,
            target: Target::General,
        })
    }

    // Only the explicitly gated qualification runner can select this target.
    // Public production constructors still stay lazy and select General.
    #[cfg(feature = "private-pasteboard-qualification")]
    pub(crate) fn unique_for_qualification() -> Result<Self, Error> {
        let thread = MainThreadMarker::new().ok_or(Error::Unavailable)?;
        let board = objc2::exception::catch(NSPasteboard::pasteboardWithUniqueName)
            .map_err(|_| Error::Unavailable)?;
        Ok(Self {
            _thread: thread,
            target: Target::Private(Some(board)),
        })
    }

    #[cfg(feature = "private-pasteboard-qualification")]
    pub(crate) fn clear_private_for_qualification(&mut self) -> Result<(), Error> {
        let Target::Private(Some(board)) = &self.target else {
            return Err(Error::Unavailable);
        };
        objc2::exception::catch(AssertUnwindSafe(|| board.clearContents()))
            .map_err(|_| Error::Unavailable)?;
        Ok(())
    }

    #[cfg(feature = "private-pasteboard-qualification")]
    pub(crate) fn release_private_for_qualification(&mut self) -> Result<(), Error> {
        // Spend before platform dispatch: exceptions cannot cause an automatic
        // second release. General is never eligible, and Closed never falls back.
        let board = self.target.take_private()?;
        objc2::exception::catch(AssertUnwindSafe(|| board.releaseGlobally()))
            .map_err(|_| Error::Unavailable)
    }
}

#[cfg(feature = "private-pasteboard-qualification")]
impl Drop for AppKit {
    fn drop(&mut self) {
        // Best effort on early errors/unwind. Success uses explicit release and
        // checks its result. Never retry a release that was already dispatched.
        let _ = self.release_private_for_qualification();
    }
}

impl Backend for AppKit {
    fn write(&mut self, text: &str) -> Result<bool, ()> {
        objc2::exception::catch(AssertUnwindSafe(|| {
            let board = self.target.board().map_err(|_| ())?;
            // Publish the standard UTF-8 plain-text representation as exact
            // bytes, without a decoder treating leading U+FEFF as metadata.
            let data = NSData::with_bytes(text.as_bytes());
            let kind = NSString::from_str("public.utf8-plain-text");
            board.clearContents();
            Ok(board.setData_forType(Some(&data), &kind))
        }))
        .map_err(|_| ())?
    }

    fn read(&mut self, max_bytes: usize) -> Result<Option<String>, Error> {
        objc2::exception::catch(AssertUnwindSafe(|| {
            let board = self.target.board()?;
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

    #[cfg(feature = "private-pasteboard-qualification")]
    #[test]
    fn released_private_target_refuses_without_falling_back_to_general() {
        let mut target = Target::Private(None);
        assert_eq!(target.board(), Err(Error::Unavailable));
        assert_eq!(target.take_private(), Err(Error::Unavailable));
    }

    #[cfg(feature = "private-pasteboard-qualification")]
    #[test]
    fn general_target_can_never_be_taken_for_global_release() {
        let mut target = Target::General;
        assert_eq!(target.take_private(), Err(Error::Unavailable));
    }

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
