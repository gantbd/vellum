use std::{
    fs,
    os::unix::fs::{MetadataExt as _, PermissionsExt as _, symlink},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

use vellum_launchd_render::{
    LaunchdConfigurationV1, decode_configuration_v1, render_launchd_profile_v1,
};

static SCRATCH_ORDINAL: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "vellum-launchd-render-{}-{}",
            std::process::id(),
            SCRATCH_ORDINAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create isolated scratch");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("close scratch mode");
        Self(path.canonicalize().expect("canonical scratch"))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove isolated scratch");
    }
}

fn fixture(root: &Path) -> LaunchdConfigurationV1 {
    let state = root.join("state & logs");
    fs::create_dir(&state).expect("create state");
    fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).expect("close state mode");
    let binary = root.join("vellum-daemon");
    fs::write(&binary, b"#!/bin/sh\nexit 0\n").expect("write binary");
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).expect("make executable");
    LaunchdConfigurationV1 {
        schema: 1,
        label: "com.example.vellum".into(),
        binary,
        data_socket: state.join("vellum.sock"),
        control_socket: state.join("vellum-control.sock"),
        lock_file: state.join(".vellum-daemon.lock"),
        metadata_file: state.join("workspace-identity-v1"),
        stdout_log: state.join("stdout.log"),
        stderr_log: state.join("stderr.log"),
        state_directory: state,
        owner_uid: fs::metadata(root).expect("scratch metadata").uid(),
    }
}

#[test]
fn renders_the_exact_inert_read_only_profile_and_refuses_authority_drift() {
    let scratch = Scratch::new();
    let configuration = fixture(&scratch.0);
    let profile = render_launchd_profile_v1(&configuration).expect("render profile");
    assert!(profile.contains("state &amp; logs/vellum.sock"));
    assert!(profile.contains("<integer>63</integer>"));
    assert!(!profile.contains("--allow-"));
    assert!(!profile.contains("<key>Sockets</key>"));
    assert!(!profile.contains("Health"));

    let mut changed = configuration.clone();
    changed.owner_uid = changed.owner_uid.saturating_add(1);
    assert!(render_launchd_profile_v1(&changed).is_err());
    let mut changed = configuration.clone();
    changed.data_socket = changed.state_directory.join("../other/vellum.sock");
    assert!(render_launchd_profile_v1(&changed).is_err());
    let linked = scratch.0.join("linked-daemon");
    symlink(&configuration.binary, &linked).expect("link binary");
    let mut changed = configuration.clone();
    changed.binary = linked;
    assert!(render_launchd_profile_v1(&changed).is_err());
    fs::write(&configuration.data_socket, b"not a socket").expect("occupy socket path");
    assert!(render_launchd_profile_v1(&configuration).is_err());
}

#[test]
fn decoder_requires_the_exact_schema_and_cli_rejects_aliased_configuration() {
    let scratch = Scratch::new();
    let configuration = fixture(&scratch.0);
    let encoded = serde_json::to_vec(&configuration).expect("encode configuration");
    assert_eq!(
        decode_configuration_v1(&encoded).expect("decode configuration"),
        configuration
    );
    let mut value: serde_json::Value = serde_json::from_slice(&encoded).expect("decode value");
    value
        .as_object_mut()
        .expect("configuration object")
        .insert("ambientAuthority".into(), true.into());
    assert!(decode_configuration_v1(&serde_json::to_vec(&value).unwrap()).is_err());

    let config = scratch.0.join("deployment.json");
    fs::write(&config, encoded).expect("write configuration");
    fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).expect("close config mode");
    let alias = scratch.0.join("deployment-link.json");
    symlink(&config, &alias).expect("link configuration");
    let refused = Command::new(env!("CARGO_BIN_EXE_vellum-launchd-render"))
        .arg(format!("--config={}", alias.display()))
        .output()
        .expect("run renderer");
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("symbolic link"));

    let rendered = Command::new(env!("CARGO_BIN_EXE_vellum-launchd-render"))
        .arg(format!("--config={}", config.display()))
        .output()
        .expect("run renderer");
    assert!(rendered.status.success());
    assert_eq!(
        String::from_utf8(rendered.stdout).unwrap(),
        render_launchd_profile_v1(&configuration).unwrap()
    );
    assert!(rendered.stderr.is_empty());
}
