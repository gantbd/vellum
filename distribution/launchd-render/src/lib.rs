//! Inert, fail-closed rendering of Vellum's read-only LaunchAgent profile.

use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::{MetadataExt as _, PermissionsExt as _},
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};

const CONFIGURATION_FIELDS: [&str; 11] = [
    "binary",
    "controlSocket",
    "dataSocket",
    "label",
    "lockFile",
    "metadataFile",
    "ownerUid",
    "schema",
    "stateDirectory",
    "stderrLog",
    "stdoutLog",
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LaunchdConfigurationV1 {
    pub schema: u64,
    pub label: String,
    pub binary: PathBuf,
    pub state_directory: PathBuf,
    pub data_socket: PathBuf,
    pub control_socket: PathBuf,
    pub lock_file: PathBuf,
    pub metadata_file: PathBuf,
    pub stdout_log: PathBuf,
    pub stderr_log: PathBuf,
    pub owner_uid: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LaunchdProfileRefusalV1(&'static str);

impl std::fmt::Display for LaunchdProfileRefusalV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for LaunchdProfileRefusalV1 {}

fn refuse(message: &'static str) -> LaunchdProfileRefusalV1 {
    LaunchdProfileRefusalV1(message)
}

pub fn decode_configuration_v1(
    input: &[u8],
) -> Result<LaunchdConfigurationV1, LaunchdProfileRefusalV1> {
    let value: serde_json::Value =
        serde_json::from_slice(input).map_err(|_| refuse("configuration is not valid JSON"))?;
    let object = value
        .as_object()
        .ok_or_else(|| refuse("configuration is not an object"))?;
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = CONFIGURATION_FIELDS.into_iter().collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(refuse("configuration fields are incompatible"));
    }
    serde_json::from_value(value).map_err(|_| refuse("configuration values are incompatible"))
}

fn absolute(path: &Path) -> Result<(), LaunchdProfileRefusalV1> {
    if !path.is_absolute()
        || path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(refuse("path is not a normalized absolute path"));
    }
    let text = path
        .to_str()
        .ok_or_else(|| refuse("path is not valid UTF-8"))?;
    if text.contains('\0')
        || text.contains("${")
        || text.contains("{{")
        || text.contains("}}")
        || text.contains("@@")
        || (text.contains('<') && text.contains('>'))
    {
        return Err(refuse(
            "path is malformed or contains an unresolved placeholder",
        ));
    }
    Ok(())
}

fn canonical(path: &Path) -> Result<(), LaunchdProfileRefusalV1> {
    let resolved = fs::canonicalize(path).map_err(|_| refuse("path could not be resolved"))?;
    if resolved != path {
        return Err(refuse(
            "path contains a symbolic-link or noncanonical ancestor",
        ));
    }
    Ok(())
}

fn private_file(path: &Path, owner_uid: u32) -> Result<(), LaunchdProfileRefusalV1> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(refuse("private file could not be inspected")),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(refuse("private path is not a regular file"));
    }
    if metadata.uid() != owner_uid || metadata.mode() & 0o077 != 0 {
        return Err(refuse("private file ownership or mode is incompatible"));
    }
    Ok(())
}

fn must_be_absent(path: &Path) -> Result<(), LaunchdProfileRefusalV1> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(refuse("socket paths must be absent")),
        Err(_) => Err(refuse("socket path absence could not be proved")),
    }
}

fn exact_child(state: &Path, path: &Path, name: &str) -> Result<(), LaunchdProfileRefusalV1> {
    if path.parent() != Some(state)
        || path.file_name().and_then(|value| value.to_str()) != Some(name)
    {
        return Err(refuse("state path is not the required exact child"));
    }
    Ok(())
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn render_launchd_profile_v1(
    configuration: &LaunchdConfigurationV1,
) -> Result<String, LaunchdProfileRefusalV1> {
    if configuration.schema != 1 {
        return Err(refuse("configuration schema is incompatible"));
    }
    if configuration.label.is_empty()
        || configuration.label.len() > 128
        || !configuration
            .label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
    {
        return Err(refuse("label is not canonical"));
    }
    if rustix::process::geteuid().as_raw() != configuration.owner_uid {
        return Err(refuse("qualification must run as the declared owner"));
    }
    for path in [
        &configuration.binary,
        &configuration.state_directory,
        &configuration.data_socket,
        &configuration.control_socket,
        &configuration.lock_file,
        &configuration.metadata_file,
        &configuration.stdout_log,
        &configuration.stderr_log,
    ] {
        absolute(path)?;
    }

    let state = fs::symlink_metadata(&configuration.state_directory)
        .map_err(|_| refuse("state directory could not be inspected"))?;
    if state.file_type().is_symlink()
        || !state.is_dir()
        || state.uid() != configuration.owner_uid
        || state.permissions().mode() & 0o777 != 0o700
    {
        return Err(refuse("state directory ownership or mode is incompatible"));
    }
    canonical(&configuration.state_directory)?;

    let binary = fs::symlink_metadata(&configuration.binary)
        .map_err(|_| refuse("binary could not be inspected"))?;
    let binary_uid = binary.uid();
    if binary.file_type().is_symlink()
        || !binary.is_file()
        || (binary_uid != 0 && binary_uid != configuration.owner_uid)
        || binary.mode() & 0o111 == 0
        || binary.mode() & 0o022 != 0
    {
        return Err(refuse("binary ownership or mode is incompatible"));
    }
    canonical(&configuration.binary)?;

    for (path, name) in [
        (&configuration.data_socket, "vellum.sock"),
        (&configuration.control_socket, "vellum-control.sock"),
        (&configuration.lock_file, ".vellum-daemon.lock"),
        (&configuration.metadata_file, "workspace-identity-v1"),
        (&configuration.stdout_log, "stdout.log"),
        (&configuration.stderr_log, "stderr.log"),
    ] {
        exact_child(&configuration.state_directory, path, name)?;
    }
    let distinct = [
        &configuration.data_socket,
        &configuration.control_socket,
        &configuration.lock_file,
        &configuration.metadata_file,
        &configuration.stdout_log,
        &configuration.stderr_log,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if distinct.len() != 6 {
        return Err(refuse("state paths are not distinct"));
    }
    must_be_absent(&configuration.data_socket)?;
    must_be_absent(&configuration.control_socket)?;
    for path in [
        &configuration.lock_file,
        &configuration.metadata_file,
        &configuration.stdout_log,
        &configuration.stderr_log,
    ] {
        private_file(path, configuration.owner_uid)?;
    }

    let binary = escape_xml(configuration.binary.to_str().expect("validated UTF-8"));
    let data = escape_xml(configuration.data_socket.to_str().expect("validated UTF-8"));
    let control = escape_xml(
        configuration
            .control_socket
            .to_str()
            .expect("validated UTF-8"),
    );
    let label = escape_xml(&configuration.label);
    let stdout = escape_xml(configuration.stdout_log.to_str().expect("validated UTF-8"));
    let stderr = escape_xml(configuration.stderr_log.to_str().expect("validated UTF-8"));
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n  <dict>\n    <key>Label</key>\n    <string>{label}</string>\n    <key>ProgramArguments</key>\n    <array>\n      <string>{binary}</string>\n      <string>--socket</string>\n      <string>{data}</string>\n      <string>--control</string>\n      <string>{control}</string>\n    </array>\n    <key>RunAtLoad</key>\n    <true/>\n    <key>KeepAlive</key>\n    <false/>\n    <key>ProcessType</key>\n    <string>Background</string>\n    <key>Umask</key>\n    <integer>63</integer>\n    <key>StandardOutPath</key>\n    <string>{stdout}</string>\n    <key>StandardErrorPath</key>\n    <string>{stderr}</string>\n  </dict>\n</plist>\n"
    ))
}
