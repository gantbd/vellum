//! Manual OS qualification, never part of ordinary tests or application startup.
#![forbid(unsafe_code)]

#[derive(Debug, Eq, PartialEq)]
enum Request {
    Help,
    Run,
}

fn request(args: &[String]) -> Option<Request> {
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["--help"] => Some(Request::Help),
        ["--allow-private-pasteboard"] => Some(Request::Run),
        _ => None,
    }
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let request = request(&args);
    if request == Some(Request::Help) {
        println!(
            "vellum-clipboard-macos-qualification --allow-private-pasteboard\nAccesses only new unique AppKit pasteboards and temporary documents. Never reads or writes the general clipboard. Requires explicit operator approval; not desktop, cross-app delivery or durability qualification."
        );
        return;
    }
    if request != Some(Request::Run) {
        eprintln!("Refused: explicit --allow-private-pasteboard is required (see --help)");
        std::process::exit(2);
    }
    #[cfg(target_os = "macos")]
    match macos::run() {
        Ok(()) => println!(
            "PASS: private AppKit raw-text and native cut/paste/replay/cleanup checks across three capture modes. No general clipboard access. Not desktop or cross-app delivery qualification."
        ),
        Err(error) => {
            eprintln!("FAIL: {error}");
            std::process::exit(1);
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("Unsupported: AppKit qualification requires macOS");
        std::process::exit(2);
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSPasteboard;
    use objc2_foundation::{NSData, NSString};
    use std::panic::AssertUnwindSafe;
    use vellum_adapter_clipboard_macos::{
        Error, QualificationBackend, QualificationWriteError, qualify_backend,
    };

    struct PrivatePasteboard {
        _thread: MainThreadMarker,
        board: Option<objc2::rc::Retained<NSPasteboard>>,
    }

    impl PrivatePasteboard {
        fn new() -> Result<Self, Error> {
            let thread = MainThreadMarker::new().ok_or(Error::Unavailable)?;
            let board = objc2::exception::catch(NSPasteboard::pasteboardWithUniqueName)
                .map_err(|_| Error::Unavailable)?;
            Ok(Self {
                _thread: thread,
                board: Some(board),
            })
        }

        fn board(&self) -> Result<&NSPasteboard, Error> {
            self.board.as_deref().ok_or(Error::Unavailable)
        }
    }

    impl QualificationBackend for PrivatePasteboard {
        fn write(&mut self, text: &str) -> Result<bool, QualificationWriteError> {
            objc2::exception::catch(AssertUnwindSafe(|| {
                let board = self.board().map_err(|_| QualificationWriteError)?;
                let data = NSData::with_bytes(text.as_bytes());
                let kind = NSString::from_str("public.utf8-plain-text");
                board.clearContents();
                Ok(board.setData_forType(Some(&data), &kind))
            }))
            .map_err(|_| QualificationWriteError)?
        }

        fn read(&mut self, max_bytes: usize) -> Result<Option<String>, Error> {
            objc2::exception::catch(AssertUnwindSafe(|| {
                let board = self.board()?;
                let before = board.changeCount();
                let kind = NSString::from_str("public.utf8-plain-text");
                let value = board.dataForType(&kind);
                let text = if let Some(value) = value {
                    if value.len() > max_bytes {
                        return Err(Error::Limit);
                    }
                    Some(String::from_utf8(value.to_vec()).map_err(|_| Error::InvalidText)?)
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

        fn clear(&mut self) -> Result<(), Error> {
            let board = self.board()?;
            objc2::exception::catch(AssertUnwindSafe(|| board.clearContents()))
                .map_err(|_| Error::Unavailable)?;
            Ok(())
        }

        fn release(&mut self) -> Result<(), Error> {
            // Spend before platform dispatch: exceptions cannot cause a retry.
            let board = self.board.take().ok_or(Error::Unavailable)?;
            objc2::exception::catch(AssertUnwindSafe(|| board.releaseGlobally()))
                .map_err(|_| Error::Unavailable)
        }
    }

    impl Drop for PrivatePasteboard {
        fn drop(&mut self) {
            let _ = self.release();
        }
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        objc2::rc::autoreleasepool(|_| qualify_backend(PrivatePasteboard::new, reference_profile))
    }

    fn reference_profile() -> vellum_model::ReferenceSessionConfig {
        use vellum_adapter_local_fs::{FilesystemLimits, LocalFilesystemFactory};
        vellum_model::reference_profile(
            LocalFilesystemFactory::new(FilesystemLimits {
                max_resources: 16,
                max_bytes: 65536,
                max_stage_attempts: 16,
                max_metadata_bytes: 65536,
            })
            .erase(),
            Box::new(vellum_adapter_annotation_json::JsonAnnotationCodec),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_exact_opt_in_can_request_os_access() {
        for args in [
            vec![],
            vec!["--allow-general-pasteboard"],
            vec!["--allow-private-pasteboard", "extra"],
            vec!["--help", "--allow-private-pasteboard"],
        ] {
            let args: Vec<_> = args.into_iter().map(str::to_owned).collect();
            assert_eq!(request(&args), None);
        }
        assert_eq!(request(&["--help".into()]), Some(Request::Help));
        assert_eq!(
            request(&["--allow-private-pasteboard".into()]),
            Some(Request::Run)
        );
    }
}
