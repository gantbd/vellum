#![forbid(unsafe_code)]

use std::path::PathBuf;
use vellum_semantic_lsp_provider_qualification::{load_profile, qualify_live};

enum Request {
    Help,
    Qualify {
        profile: PathBuf,
        executable: PathBuf,
    },
}

fn request(args: &[String]) -> Option<Request> {
    match args {
        [help] if help == "--help" => Some(Request::Help),
        [allow, profile_flag, profile, executable_flag, executable]
            if allow == "--qualify-pinned-provider"
                && profile_flag == "--profile"
                && executable_flag == "--executable" =>
        {
            Some(Request::Qualify {
                profile: PathBuf::from(profile),
                executable: PathBuf::from(executable),
            })
        }
        _ => None,
    }
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match request(&args) {
        Some(Request::Help) => println!(
            "vellum-semantic-lsp-provider-qualification --qualify-pinned-provider --profile <profile.json> --executable </absolute/canonical/provider>\nValidates the package-owned fixture and exact pinned artifact. Artifact preflight alone is not semantic-provider qualification evidence."
        ),
        Some(Request::Qualify {
            profile,
            executable,
        }) => match load_profile(&profile) {
            Ok(profile) => match qualify_live(&profile, &executable) {
                Ok(receipt) => match serde_json::to_string(&receipt) {
                    Ok(receipt) => println!("{receipt}"),
                    Err(_) => {
                        eprintln!("Refused: qualification receipt encoding failed");
                        std::process::exit(1);
                    }
                },
                Err(refusal) => {
                    eprintln!("FAIL: {refusal} ({refusal:?})");
                    std::process::exit(1);
                }
            },
            Err(refusal) => {
                eprintln!("Refused: {refusal}");
                std::process::exit(2);
            }
        },
        None => {
            eprintln!("Refused: exact --qualify-pinned-provider invocation required (see --help)");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_exact_opt_in_shape_is_accepted() {
        assert!(matches!(request(&["--help".into()]), Some(Request::Help)));
        assert!(matches!(
            request(&[
                "--qualify-pinned-provider".into(),
                "--profile".into(),
                "profile.json".into(),
                "--executable".into(),
                "/provider".into(),
            ]),
            Some(Request::Qualify { .. })
        ));
        for args in [
            vec![],
            vec!["--profile"],
            vec!["--qualify-pinned-provider", "--profile", "profile.json"],
            vec![
                "--qualify-pinned-provider",
                "--executable",
                "/provider",
                "--profile",
                "profile.json",
            ],
        ] {
            let owned: Vec<_> = args.into_iter().map(str::to_owned).collect();
            assert!(request(&owned).is_none());
        }
    }
}
