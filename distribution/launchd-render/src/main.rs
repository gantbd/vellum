use std::{fs, os::unix::fs::MetadataExt as _, path::PathBuf};

use vellum_launchd_render::{decode_configuration_v1, render_launchd_profile_v1};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let Some(argument) = arguments.next() else {
        return Err("usage: vellum-launchd-render --config=DEPLOYMENT.json".into());
    };
    if arguments.next().is_some() {
        return Err("usage: vellum-launchd-render --config=DEPLOYMENT.json".into());
    }
    let text = argument
        .to_str()
        .ok_or("configuration argument is not valid UTF-8")?;
    if text == "--help" {
        println!("vellum-launchd-render --config=ABSOLUTE_PRIVATE_DEPLOYMENT.json");
        return Ok(());
    }
    let path = PathBuf::from(
        text.strip_prefix("--config=")
            .ok_or("usage: vellum-launchd-render --config=DEPLOYMENT.json")?,
    );
    if !path.is_absolute() {
        return Err("configuration path is not an absolute path".into());
    }
    let metadata = fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink() {
        return Err("configuration is a symbolic link".into());
    }
    if !metadata.is_file() || metadata.mode() & 0o022 != 0 {
        return Err("configuration type or mode is incompatible".into());
    }
    if fs::canonicalize(&path)? != path {
        return Err("configuration has a noncanonical ancestor".into());
    }
    let bytes = fs::read(&path)?;
    let configuration = decode_configuration_v1(&bytes)?;
    if metadata.uid() != configuration.owner_uid {
        return Err("configuration has the wrong owner".into());
    }
    print!("{}", render_launchd_profile_v1(&configuration)?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("launchd profile: {error}");
        std::process::exit(1);
    }
}
