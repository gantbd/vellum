//! Manual OS qualification, never part of ordinary tests or reference startup.
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
            "private-pasteboard --allow-private-pasteboard\nAccesses only a new unique AppKit pasteboard and temporary documents. Never reads or writes the general clipboard. Requires explicit operator approval; not desktop, cross-app delivery or durability qualification."
        );
        return;
    }
    if request != Some(Request::Run) {
        eprintln!("Refused: explicit --allow-private-pasteboard is required (see --help)");
        std::process::exit(2);
    }
    #[cfg(target_os = "macos")]
    match vellum_adapter_clipboard_macos::qualify_private_pasteboard(reference_profile) {
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
