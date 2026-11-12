//! Explicit, approval-bound launchd activation with durable effect brackets.
//!
//! The transaction core owns policy and evidence validation. Platform effects
//! are injected; only [`MacOsLaunchdPortV1`] executes `launchctl` or the daemon
//! status command. A durable journal entry is synchronized before and after
//! every launchd mutation. Missing post-effect evidence therefore remains an
//! explicit unknown outcome and is never retried automatically.

use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::Write as _,
    os::unix::fs::{FileTypeExt as _, MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

const DIGEST_BYTES: usize = 32;
const TRANSACTION_BYTES: usize = 16;
const MAX_EFFECT_OUTPUT_BYTES: usize = 64 * 1024;
const MAX_HEALTH_ATTEMPTS: u32 = 256;
const MAX_EFFECT_TIMEOUT_MS: u64 = 120_000;
const JOURNAL_DIRECTORY: &str = "activation-journal";
const FIRST_INSTALL_JOURNAL_DIRECTORY: &str = "first-install-journal";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivationRefusalV1(&'static str);

impl std::fmt::Display for ActivationRefusalV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for ActivationRefusalV1 {}

const fn refuse(message: &'static str) -> ActivationRefusalV1 {
    ActivationRefusalV1(message)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ActivationConfigurationV1 {
    pub schema: u64,
    pub channel: String,
    pub qualification_evidence: PathBuf,
    pub approval: PathBuf,
    pub transaction_directory: PathBuf,
    pub label: String,
    pub owner_uid: u32,
    pub state_directory: PathBuf,
    pub data_socket: PathBuf,
    pub control_socket: PathBuf,
    pub stdout_log: PathBuf,
    pub stderr_log: PathBuf,
    pub candidate: ActivationVersionInputV1,
    pub prior: ActivationVersionInputV1,
    pub max_health_attempts: u32,
    pub health_interval_ms: u64,
    pub effect_timeout_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ActivationVersionInputV1 {
    pub directory: PathBuf,
    pub plist: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FirstInstallConfigurationV1 {
    pub schema: u64,
    pub channel: String,
    pub installation_receipt: PathBuf,
    pub approval: PathBuf,
    pub transaction_directory: PathBuf,
    pub label: String,
    pub owner_uid: u32,
    pub state_directory: PathBuf,
    pub data_socket: PathBuf,
    pub control_socket: PathBuf,
    pub stdout_log: PathBuf,
    pub stderr_log: PathBuf,
    pub candidate: ActivationVersionInputV1,
    pub max_health_attempts: u32,
    pub health_interval_ms: u64,
    pub effect_timeout_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ActivationApprovalV1 {
    schema: u64,
    channel: String,
    transaction_id: String,
    requested_action: String,
    candidate_manifest_sha256: String,
    prior_manifest_sha256: String,
    activation_authorized: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct FirstInstallApprovalV1 {
    schema: u64,
    channel: String,
    transaction_id: String,
    requested_action: String,
    candidate_manifest_sha256: String,
    installation_authorized: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct QualificationEvidenceV1 {
    schema: u64,
    channel: String,
    state: String,
    activation_authorized: bool,
    claims: Vec<String>,
    non_claims: Vec<String>,
    candidate: QualifiedCandidateV1,
    rollback: QualifiedRollbackV1,
    activation_plan: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct QualifiedCandidateV1 {
    manifest_sha256: String,
    vellum_revision: String,
    artifacts: Vec<QualifiedArtifactV1>,
    deployment: Vec<QualifiedArtifactV1>,
    smoke: serde_json::Value,
    launchd_smoke: serde_json::Value,
    activation_executor_smoke: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct QualifiedRollbackV1 {
    selected_prior_manifest_sha256: String,
    selected_prior_vellum_revision: String,
    artifacts: Vec<QualifiedArtifactV1>,
    deployment: Vec<QualifiedArtifactV1>,
    smoke: serde_json::Value,
    launchd_smoke: serde_json::Value,
    activation_executor_smoke: serde_json::Value,
    trigger: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QualifiedArtifactV1 {
    file: String,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct InstallationReceiptV1 {
    schema: u64,
    channel: String,
    manifest_sha256: String,
    claims: Vec<String>,
    non_claims: Vec<String>,
    products: Vec<InstalledProductV1>,
    activation_performed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct InstalledProductV1 {
    file: String,
    bytes: u64,
    sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LaunchdEffectDispositionV1 {
    Confirmed,
    Refused,
    OutcomeUnknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LaunchdJobIdentityV1 {
    pub pid: u32,
    pub program: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LaunchdObservationV1<T> {
    Observed(T),
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DaemonStatusV1 {
    Ready,
    NotReady,
}

pub trait LaunchdActivationPortV1 {
    fn inspect_job(&mut self, label: &str) -> LaunchdObservationV1<LaunchdJobIdentityV1>;

    fn unload_job(&mut self, label: &str, timeout: Duration) -> LaunchdEffectDispositionV1;

    fn load_job(
        &mut self,
        label: &str,
        plist: &Path,
        timeout: Duration,
    ) -> LaunchdEffectDispositionV1;

    fn daemon_status(
        &mut self,
        daemon: &Path,
        control_socket: &Path,
        timeout: Duration,
    ) -> LaunchdObservationV1<DaemonStatusV1>;

    fn wait_before_status_retry(&mut self, interval: Duration);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivationOutcomeV1 {
    CandidateActive,
    PriorRestored,
    ActivationRefused,
    RollbackRefused,
    OutcomeUnknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivationReceiptV1 {
    pub schema: u64,
    pub channel: &'static str,
    pub transaction_id: String,
    pub candidate_manifest_sha256: String,
    pub prior_manifest_sha256: String,
    pub selected_manifest_sha256: String,
    pub outcome: ActivationOutcomeV1,
    pub launchd_effect: &'static str,
    pub process_identity: &'static str,
    pub daemon_status: &'static str,
    pub content_disclosed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FirstInstallOutcomeV1 {
    CandidateActive,
    InstallationRefused,
    OutcomeUnknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstInstallReceiptV1 {
    pub schema: u64,
    pub channel: &'static str,
    pub transaction_id: String,
    pub candidate_manifest_sha256: String,
    pub selected_manifest_sha256: String,
    pub outcome: FirstInstallOutcomeV1,
    pub launchd_effect: &'static str,
    pub process_identity: &'static str,
    pub daemon_status: &'static str,
    pub content_disclosed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FirstInstallReconciliationStateV1 {
    CandidateActiveDurable,
    InstallationRefusedDurable,
    PreparedOnly,
    PendingEffect,
    OutcomeUnknown,
    ConfirmedEffectWithoutDurableTerminal,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstInstallReconciliationV1 {
    pub schema: u64,
    pub channel: &'static str,
    pub transaction_id: String,
    pub candidate_manifest_sha256: String,
    pub selected_manifest_sha256: String,
    pub journal_entries: u64,
    pub last_sequence: u64,
    pub last_phase: String,
    pub last_effect: String,
    pub last_disposition: String,
    pub state: FirstInstallReconciliationStateV1,
    pub required_action: ActivationReconciliationActionV1,
    pub automatic_retry_authorized: bool,
    pub launchd_effect: &'static str,
    pub content_disclosed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivationReconciliationStateV1 {
    CandidateActiveDurable,
    PriorRestoredDurable,
    ActivationRefusedDurable,
    RollbackRefusedDurable,
    PreparedOnly,
    PendingEffect,
    OutcomeUnknown,
    ConfirmedEffectWithoutDurableTerminal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivationReconciliationActionV1 {
    None,
    ManualReauthorizationRequired,
    InspectExactJobAndDaemonStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivationReconciliationV1 {
    pub schema: u64,
    pub channel: &'static str,
    pub transaction_id: String,
    pub candidate_manifest_sha256: String,
    pub prior_manifest_sha256: String,
    pub selected_manifest_sha256: String,
    pub journal_entries: u64,
    pub last_sequence: u64,
    pub last_phase: String,
    pub last_effect: String,
    pub last_disposition: String,
    pub state: ActivationReconciliationStateV1,
    pub required_action: ActivationReconciliationActionV1,
    pub automatic_retry_authorized: bool,
    pub launchd_effect: &'static str,
    pub content_disclosed: bool,
}

#[derive(Clone, Debug)]
struct CheckedVersionV1 {
    manifest: String,
    daemon: PathBuf,
    plist: PathBuf,
}

#[derive(Clone, Debug)]
struct CheckedActivationV1 {
    approval: ActivationApprovalV1,
    journal: PathBuf,
    candidate: CheckedVersionV1,
    prior: CheckedVersionV1,
    label: String,
    control_socket: PathBuf,
    health_attempts: u32,
    health_interval: Duration,
    effect_timeout: Duration,
    owner_uid: u32,
}

#[derive(Clone, Debug)]
struct CheckedFirstInstallV1 {
    approval: FirstInstallApprovalV1,
    journal: PathBuf,
    candidate: CheckedVersionV1,
    label: String,
    control_socket: PathBuf,
    health_attempts: u32,
    health_interval: Duration,
    effect_timeout: Duration,
    owner_uid: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct JournalEntryV1<'a> {
    schema: u64,
    channel: &'static str,
    sequence: u64,
    transaction_id: &'a str,
    phase: &'static str,
    effect: &'static str,
    disposition: &'static str,
    candidate_manifest_sha256: &'a str,
    prior_manifest_sha256: &'a str,
    selected_manifest_sha256: &'a str,
    content_disclosed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct JournalRecordV1 {
    schema: u64,
    channel: String,
    sequence: u64,
    transaction_id: String,
    phase: String,
    effect: String,
    disposition: String,
    candidate_manifest_sha256: String,
    prior_manifest_sha256: String,
    selected_manifest_sha256: String,
    content_disclosed: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FirstInstallJournalEntryV1<'a> {
    schema: u64,
    channel: &'static str,
    sequence: u64,
    transaction_id: &'a str,
    phase: &'static str,
    effect: &'static str,
    disposition: &'static str,
    candidate_manifest_sha256: &'a str,
    selected_manifest_sha256: &'a str,
    content_disclosed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct FirstInstallJournalRecordV1 {
    schema: u64,
    channel: String,
    sequence: u64,
    transaction_id: String,
    phase: String,
    effect: String,
    disposition: String,
    candidate_manifest_sha256: String,
    selected_manifest_sha256: String,
    content_disclosed: bool,
}

fn decode_hex<const N: usize>(value: &str) -> Option<[u8; N]> {
    if value.len() != N * 2 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut output = [0_u8; N];
    for (index, byte) in output.iter_mut().enumerate() {
        let pair = &value[index * 2..index * 2 + 2];
        *byte = u8::from_str_radix(pair, 16).ok()?;
    }
    Some(output)
}

fn sha256_file(path: &Path) -> Result<String, ActivationRefusalV1> {
    let bytes = fs::read(path).map_err(|_| refuse("artifact could not be read"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn normalized_absolute(path: &Path) -> bool {
    path.is_absolute()
        && !path.as_os_str().is_empty()
        && !path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
}

fn canonical_regular_file(
    path: &Path,
    owner: u32,
    exact_mode: Option<u32>,
    executable: bool,
) -> Result<(), ActivationRefusalV1> {
    if !normalized_absolute(path) || fs::canonicalize(path).ok().as_deref() != Some(path) {
        return Err(refuse("file path is not canonical"));
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| refuse("file could not be inspected"))?;
    let mode = metadata.mode() & 0o777;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.uid() != owner
        || mode & 0o022 != 0
        || exact_mode.is_some_and(|expected| mode != expected)
        || (executable && mode & 0o100 == 0)
    {
        return Err(refuse("file ownership, mode or type is incompatible"));
    }
    Ok(())
}

fn canonical_directory(
    path: &Path,
    owner: u32,
    exact_mode: u32,
) -> Result<(), ActivationRefusalV1> {
    if !normalized_absolute(path) || fs::canonicalize(path).ok().as_deref() != Some(path) {
        return Err(refuse("directory path is not canonical"));
    }
    let metadata =
        fs::symlink_metadata(path).map_err(|_| refuse("directory could not be inspected"))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || metadata.uid() != owner
        || metadata.mode() & 0o777 != exact_mode
    {
        return Err(refuse("directory ownership, mode or type is incompatible"));
    }
    Ok(())
}

fn optional_private_path(path: &Path, owner: u32, socket: bool) -> Result<(), ActivationRefusalV1> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(refuse("daemon state path could not be inspected")),
    };
    if metadata.file_type().is_symlink()
        || metadata.uid() != owner
        || metadata.mode() & 0o077 != 0
        || (socket && !metadata.file_type().is_socket())
        || (!socket && !metadata.is_file())
    {
        return Err(refuse(
            "daemon state path ownership, mode or type is incompatible",
        ));
    }
    Ok(())
}

fn read_json_file<T: for<'de> Deserialize<'de>>(
    path: &Path,
    owner: u32,
    mode: u32,
) -> Result<T, ActivationRefusalV1> {
    canonical_regular_file(path, owner, Some(mode), false)?;
    let bytes = fs::read(path).map_err(|_| refuse("evidence could not be read"))?;
    serde_json::from_slice(&bytes).map_err(|_| refuse("evidence JSON is incompatible"))
}

fn read_approval(path: &Path, owner: u32) -> Result<ActivationApprovalV1, ActivationRefusalV1> {
    canonical_regular_file(path, owner, Some(0o600), false)?;
    let bytes = fs::read(path).map_err(|_| refuse("approval could not be read"))?;
    let approval: ActivationApprovalV1 =
        serde_json::from_slice(&bytes).map_err(|_| refuse("approval JSON is incompatible"))?;
    let mut canonical =
        serde_json::to_vec_pretty(&approval).map_err(|_| refuse("approval encoding failed"))?;
    canonical.push(b'\n');
    if bytes != canonical {
        return Err(refuse("approval JSON is not canonical"));
    }
    Ok(approval)
}

fn read_first_install_approval(
    path: &Path,
    owner: u32,
) -> Result<FirstInstallApprovalV1, ActivationRefusalV1> {
    canonical_regular_file(path, owner, Some(0o600), false)?;
    let bytes = fs::read(path).map_err(|_| refuse("approval could not be read"))?;
    let approval: FirstInstallApprovalV1 =
        serde_json::from_slice(&bytes).map_err(|_| refuse("approval JSON is incompatible"))?;
    let mut canonical =
        serde_json::to_vec_pretty(&approval).map_err(|_| refuse("approval encoding failed"))?;
    canonical.push(b'\n');
    if bytes != canonical {
        return Err(refuse("approval JSON is not canonical"));
    }
    Ok(approval)
}

fn read_installation_receipt(
    path: &Path,
    owner: u32,
) -> Result<InstallationReceiptV1, ActivationRefusalV1> {
    canonical_regular_file(path, owner, Some(0o400), false)?;
    let bytes = fs::read(path).map_err(|_| refuse("installation receipt could not be read"))?;
    let receipt: InstallationReceiptV1 = serde_json::from_slice(&bytes)
        .map_err(|_| refuse("installation receipt JSON is incompatible"))?;
    let mut canonical = serde_json::to_vec_pretty(&receipt)
        .map_err(|_| refuse("installation receipt encoding failed"))?;
    canonical.push(b'\n');
    if bytes != canonical {
        return Err(refuse("installation receipt JSON is not canonical"));
    }
    Ok(receipt)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn expected_plist_fields(
    label: &str,
    data_socket: &Path,
    control_socket: &Path,
    stdout_log: &Path,
    stderr_log: &Path,
    daemon: &Path,
) -> Option<String> {
    let daemon = xml_escape(daemon.to_str()?);
    let data = xml_escape(data_socket.to_str()?);
    let control = xml_escape(control_socket.to_str()?);
    let stdout = xml_escape(stdout_log.to_str()?);
    let stderr = xml_escape(stderr_log.to_str()?);
    let label = xml_escape(label);
    Some(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n  <dict>\n    <key>Label</key>\n    <string>{label}</string>\n    <key>ProgramArguments</key>\n    <array>\n      <string>{daemon}</string>\n      <string>--socket</string>\n      <string>{data}</string>\n      <string>--control</string>\n      <string>{control}</string>\n    </array>\n    <key>RunAtLoad</key>\n    <true/>\n    <key>KeepAlive</key>\n    <false/>\n    <key>ProcessType</key>\n    <string>Background</string>\n    <key>Umask</key>\n    <integer>63</integer>\n    <key>StandardOutPath</key>\n    <string>{stdout}</string>\n    <key>StandardErrorPath</key>\n    <string>{stderr}</string>\n  </dict>\n</plist>\n"
    ))
}

fn expected_plist(configuration: &ActivationConfigurationV1, daemon: &Path) -> Option<String> {
    expected_plist_fields(
        &configuration.label,
        &configuration.data_socket,
        &configuration.control_socket,
        &configuration.stdout_log,
        &configuration.stderr_log,
        daemon,
    )
}

fn qualified_daemon(artifacts: &[QualifiedArtifactV1]) -> Option<&QualifiedArtifactV1> {
    (artifacts.len() == 2
        && artifacts
            .iter()
            .map(|artifact| artifact.file.as_str())
            .collect::<BTreeSet<_>>()
            == BTreeSet::from(["vellum-daemon", "vellum-mcp"]))
    .then(|| {
        artifacts
            .iter()
            .find(|artifact| artifact.file == "vellum-daemon")
    })
    .flatten()
}

fn validate_version(
    configuration: &ActivationConfigurationV1,
    input: &ActivationVersionInputV1,
    manifest: &str,
    artifacts: &[QualifiedArtifactV1],
) -> Result<CheckedVersionV1, ActivationRefusalV1> {
    decode_hex::<DIGEST_BYTES>(manifest).ok_or_else(|| refuse("manifest digest is invalid"))?;
    canonical_directory(&input.directory, configuration.owner_uid, 0o700)?;
    canonical_regular_file(&input.plist, configuration.owner_uid, Some(0o600), false)?;
    let daemon = input.directory.join("vellum-daemon");
    canonical_regular_file(&daemon, configuration.owner_uid, None, true)?;
    let expected =
        qualified_daemon(artifacts).ok_or_else(|| refuse("qualified artifact closure differs"))?;
    decode_hex::<DIGEST_BYTES>(&expected.sha256)
        .ok_or_else(|| refuse("qualified daemon digest is invalid"))?;
    if sha256_file(&daemon)? != expected.sha256 {
        return Err(refuse("qualified daemon digest differs"));
    }
    let plist = fs::read_to_string(&input.plist).map_err(|_| refuse("plist could not be read"))?;
    if Some(plist) != expected_plist(configuration, &daemon) {
        return Err(refuse("plist is not the exact inert activation profile"));
    }
    Ok(CheckedVersionV1 {
        manifest: manifest.to_owned(),
        daemon,
        plist: input.plist.clone(),
    })
}

fn prepare(
    configuration: &ActivationConfigurationV1,
    existing_journal: bool,
) -> Result<CheckedActivationV1, ActivationRefusalV1> {
    if configuration.schema != 1
        || configuration.channel != "vellum-launchd-activation"
        || configuration.max_health_attempts == 0
        || configuration.max_health_attempts > MAX_HEALTH_ATTEMPTS
        || configuration.effect_timeout_ms == 0
        || configuration.effect_timeout_ms > MAX_EFFECT_TIMEOUT_MS
        || configuration.health_interval_ms > MAX_EFFECT_TIMEOUT_MS
        || configuration.label.is_empty()
        || configuration.label.len() > 128
        || !configuration
            .label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
        || rustix::process::geteuid().as_raw() != configuration.owner_uid
    {
        return Err(refuse("activation configuration is incompatible"));
    }
    canonical_directory(
        &configuration.state_directory,
        configuration.owner_uid,
        0o700,
    )?;
    canonical_directory(
        &configuration.transaction_directory,
        configuration.owner_uid,
        0o700,
    )?;
    let transaction_entries = fs::read_dir(&configuration.transaction_directory)
        .map_err(|_| refuse("transaction directory could not be read"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| refuse("transaction directory could not be read"))?;
    if (!existing_journal && !transaction_entries.is_empty())
        || (existing_journal
            && (transaction_entries.len() != 1
                || transaction_entries[0].file_name() != JOURNAL_DIRECTORY
                || !transaction_entries[0]
                    .file_type()
                    .map_err(|_| refuse("journal directory could not be inspected"))?
                    .is_dir()))
    {
        return Err(refuse("transaction directory closure is incompatible"));
    }
    for (path, name) in [
        (&configuration.data_socket, "vellum.sock"),
        (&configuration.control_socket, "vellum-control.sock"),
        (&configuration.stdout_log, "stdout.log"),
        (&configuration.stderr_log, "stderr.log"),
    ] {
        if path.parent() != Some(configuration.state_directory.as_path())
            || path.file_name().and_then(|value| value.to_str()) != Some(name)
        {
            return Err(refuse("daemon state path is incompatible"));
        }
    }
    optional_private_path(&configuration.data_socket, configuration.owner_uid, true)?;
    optional_private_path(&configuration.control_socket, configuration.owner_uid, true)?;
    optional_private_path(&configuration.stdout_log, configuration.owner_uid, false)?;
    optional_private_path(&configuration.stderr_log, configuration.owner_uid, false)?;
    let evidence: QualificationEvidenceV1 = read_json_file(
        &configuration.qualification_evidence,
        configuration.owner_uid,
        0o600,
    )?;
    let approval = read_approval(&configuration.approval, configuration.owner_uid)?;
    if evidence.schema != 1
        || evidence.channel != "local-release-candidate-qualification"
        || evidence.state != "ready-for-manual-activation-review"
        || evidence.activation_authorized
        || !evidence
            .claims
            .iter()
            .any(|claim| claim == "candidate-integrity-verified")
        || !evidence
            .claims
            .iter()
            .any(|claim| claim == "candidate-launchd-render-smoke-passed")
        || !evidence
            .claims
            .iter()
            .any(|claim| claim == "rollback-version-selected")
        || evidence.activation_plan.is_empty()
        || evidence.non_claims.is_empty()
        || evidence.candidate.vellum_revision == evidence.rollback.selected_prior_vellum_revision
        || evidence
            .claims
            .iter()
            .all(|claim| claim != "candidate-activation-executor-help-smoke-passed-without-effect")
        || evidence.candidate.deployment.len() != 2
        || evidence.rollback.deployment.len() != 2
        || evidence
            .candidate
            .deployment
            .iter()
            .map(|artifact| artifact.file.as_str())
            .collect::<BTreeSet<_>>()
            != BTreeSet::from(["vellum-launchd-activate", "vellum-launchd-render"])
        || evidence
            .rollback
            .deployment
            .iter()
            .map(|artifact| artifact.file.as_str())
            .collect::<BTreeSet<_>>()
            != BTreeSet::from(["vellum-launchd-activate", "vellum-launchd-render"])
        || evidence.candidate.smoke.is_null()
        || evidence.rollback.smoke.is_null()
        || evidence.candidate.launchd_smoke.is_null()
        || evidence.rollback.launchd_smoke.is_null()
        || evidence.candidate.activation_executor_smoke.is_null()
        || evidence.rollback.activation_executor_smoke.is_null()
        || evidence.rollback.trigger != "post-activation readiness or health gate fails"
    {
        return Err(refuse("qualification evidence is incompatible"));
    }
    if approval.schema != 1
        || approval.channel != "local-activation-approval"
        || approval.requested_action != "select-candidate-for-external-activation"
        || !approval.activation_authorized
        || decode_hex::<TRANSACTION_BYTES>(&approval.transaction_id).is_none()
        || approval.candidate_manifest_sha256 != evidence.candidate.manifest_sha256
        || approval.prior_manifest_sha256 != evidence.rollback.selected_prior_manifest_sha256
        || approval.candidate_manifest_sha256 == approval.prior_manifest_sha256
    {
        return Err(refuse("activation approval is incompatible"));
    }
    let candidate = validate_version(
        configuration,
        &configuration.candidate,
        &evidence.candidate.manifest_sha256,
        &evidence.candidate.artifacts,
    )?;
    let prior = validate_version(
        configuration,
        &configuration.prior,
        &evidence.rollback.selected_prior_manifest_sha256,
        &evidence.rollback.artifacts,
    )?;
    if candidate.daemon == prior.daemon || candidate.plist == prior.plist {
        return Err(refuse("candidate and prior versions are not distinct"));
    }
    let journal = configuration.transaction_directory.join(JOURNAL_DIRECTORY);
    if existing_journal {
        canonical_directory(&journal, configuration.owner_uid, 0o700)?;
    } else {
        fs::create_dir(&journal).map_err(|_| refuse("journal directory could not be created"))?;
        fs::set_permissions(&journal, fs::Permissions::from_mode(0o700))
            .map_err(|_| refuse("journal directory mode could not be closed"))?;
        sync_directory(&configuration.transaction_directory)?;
    }
    Ok(CheckedActivationV1 {
        approval,
        journal,
        candidate,
        prior,
        label: configuration.label.clone(),
        control_socket: configuration.control_socket.clone(),
        health_attempts: configuration.max_health_attempts,
        health_interval: Duration::from_millis(configuration.health_interval_ms),
        effect_timeout: Duration::from_millis(configuration.effect_timeout_ms),
        owner_uid: configuration.owner_uid,
    })
}

fn validate_first_install_version(
    configuration: &FirstInstallConfigurationV1,
    input: &ActivationVersionInputV1,
    receipt_path: &Path,
    receipt: &InstallationReceiptV1,
) -> Result<CheckedVersionV1, ActivationRefusalV1> {
    decode_hex::<DIGEST_BYTES>(&receipt.manifest_sha256)
        .ok_or_else(|| refuse("installed manifest digest is invalid"))?;
    let installation_root = receipt_path
        .parent()
        .ok_or_else(|| refuse("installation receipt path is incompatible"))?;
    if receipt_path.file_name().and_then(|value| value.to_str()) != Some("install-receipt.json")
        || input.directory != installation_root.join("bin")
    {
        return Err(refuse("installed candidate path is incompatible"));
    }
    canonical_directory(installation_root, configuration.owner_uid, 0o700)?;
    let root_entries = fs::read_dir(installation_root)
        .map_err(|_| refuse("installation root could not be read"))?
        .map(|entry| {
            entry
                .map(|value| value.file_name())
                .map_err(|_| refuse("installation root could not be read"))
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if root_entries != BTreeSet::from(["bin".into(), "install-receipt.json".into()]) {
        return Err(refuse("installation root closure differs"));
    }
    canonical_directory(&input.directory, configuration.owner_uid, 0o700)?;
    let product_entries = fs::read_dir(&input.directory)
        .map_err(|_| refuse("installed product directory could not be read"))?
        .map(|entry| {
            entry
                .map(|value| value.file_name())
                .map_err(|_| refuse("installed product directory could not be read"))
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if product_entries != BTreeSet::from(["vellum-daemon".into(), "vellum-mcp".into()]) {
        return Err(refuse("installed product directory closure differs"));
    }
    canonical_regular_file(&input.plist, configuration.owner_uid, Some(0o600), false)?;
    let daemon = input.directory.join("vellum-daemon");
    let mcp = input.directory.join("vellum-mcp");
    if receipt.products.len() != 2
        || receipt
            .products
            .iter()
            .map(|product| product.file.as_str())
            .collect::<BTreeSet<_>>()
            != BTreeSet::from(["bin/vellum-daemon", "bin/vellum-mcp"])
    {
        return Err(refuse("installation receipt product closure differs"));
    }
    for (path, name) in [(&daemon, "bin/vellum-daemon"), (&mcp, "bin/vellum-mcp")] {
        canonical_regular_file(path, configuration.owner_uid, Some(0o500), true)?;
        let expected = receipt
            .products
            .iter()
            .find(|product| product.file == name)
            .ok_or_else(|| refuse("installation receipt product closure differs"))?;
        decode_hex::<DIGEST_BYTES>(&expected.sha256)
            .ok_or_else(|| refuse("installed product digest is invalid"))?;
        let metadata =
            fs::metadata(path).map_err(|_| refuse("installed product could not be inspected"))?;
        if metadata.len() != expected.bytes || sha256_file(path)? != expected.sha256 {
            return Err(refuse("installed product identity differs"));
        }
    }
    let plist = fs::read_to_string(&input.plist).map_err(|_| refuse("plist could not be read"))?;
    if Some(plist)
        != expected_plist_fields(
            &configuration.label,
            &configuration.data_socket,
            &configuration.control_socket,
            &configuration.stdout_log,
            &configuration.stderr_log,
            &daemon,
        )
    {
        return Err(refuse("plist is not the exact inert activation profile"));
    }
    Ok(CheckedVersionV1 {
        manifest: receipt.manifest_sha256.clone(),
        daemon,
        plist: input.plist.clone(),
    })
}

fn require_absent(path: &Path) -> Result<(), ActivationRefusalV1> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(refuse("first-install socket path is not absent")),
        Err(_) => Err(refuse("first-install socket absence could not be proved")),
    }
}

fn prepare_first_install(
    configuration: &FirstInstallConfigurationV1,
    existing_journal: bool,
) -> Result<CheckedFirstInstallV1, ActivationRefusalV1> {
    if configuration.schema != 1
        || configuration.channel != "vellum-launchd-first-install"
        || configuration.max_health_attempts == 0
        || configuration.max_health_attempts > MAX_HEALTH_ATTEMPTS
        || configuration.effect_timeout_ms == 0
        || configuration.effect_timeout_ms > MAX_EFFECT_TIMEOUT_MS
        || configuration.health_interval_ms > MAX_EFFECT_TIMEOUT_MS
        || configuration.label.is_empty()
        || configuration.label.len() > 128
        || !configuration
            .label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
        || rustix::process::geteuid().as_raw() != configuration.owner_uid
    {
        return Err(refuse("first-install configuration is incompatible"));
    }
    canonical_directory(
        &configuration.state_directory,
        configuration.owner_uid,
        0o700,
    )?;
    canonical_directory(
        &configuration.transaction_directory,
        configuration.owner_uid,
        0o700,
    )?;
    let transaction_entries = fs::read_dir(&configuration.transaction_directory)
        .map_err(|_| refuse("transaction directory could not be read"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| refuse("transaction directory could not be read"))?;
    if (!existing_journal && !transaction_entries.is_empty())
        || (existing_journal
            && (transaction_entries.len() != 1
                || transaction_entries[0].file_name() != FIRST_INSTALL_JOURNAL_DIRECTORY
                || !transaction_entries[0]
                    .file_type()
                    .map_err(|_| refuse("journal directory could not be inspected"))?
                    .is_dir()))
    {
        return Err(refuse("first-install transaction closure is incompatible"));
    }
    for (path, name) in [
        (&configuration.data_socket, "vellum.sock"),
        (&configuration.control_socket, "vellum-control.sock"),
        (&configuration.stdout_log, "stdout.log"),
        (&configuration.stderr_log, "stderr.log"),
    ] {
        if path.parent() != Some(configuration.state_directory.as_path())
            || path.file_name().and_then(|value| value.to_str()) != Some(name)
        {
            return Err(refuse("daemon state path is incompatible"));
        }
    }
    if existing_journal {
        optional_private_path(&configuration.data_socket, configuration.owner_uid, true)?;
        optional_private_path(&configuration.control_socket, configuration.owner_uid, true)?;
    } else {
        require_absent(&configuration.data_socket)?;
        require_absent(&configuration.control_socket)?;
    }
    optional_private_path(&configuration.stdout_log, configuration.owner_uid, false)?;
    optional_private_path(&configuration.stderr_log, configuration.owner_uid, false)?;
    let receipt =
        read_installation_receipt(&configuration.installation_receipt, configuration.owner_uid)?;
    let approval = read_first_install_approval(&configuration.approval, configuration.owner_uid)?;
    if receipt.schema != 1
        || receipt.channel != "local-release-candidate-installation"
        || receipt.claims != ["verified-bundle", "daemon-mcp-only", "durable-local-prefix"]
        || receipt.non_claims
            != [
                "path-configuration",
                "launchd-installation",
                "activation",
                "upgrade",
                "uninstall",
                "signature",
                "attestation",
            ]
        || receipt.activation_performed
    {
        return Err(refuse("first-install installation receipt is incompatible"));
    }
    if approval.schema != 1
        || approval.channel != "local-first-install-approval"
        || approval.requested_action != "install-candidate-as-new-launchagent"
        || !approval.installation_authorized
        || decode_hex::<TRANSACTION_BYTES>(&approval.transaction_id).is_none()
        || approval.candidate_manifest_sha256 != receipt.manifest_sha256
    {
        return Err(refuse("first-install approval is incompatible"));
    }
    let candidate = validate_first_install_version(
        configuration,
        &configuration.candidate,
        &configuration.installation_receipt,
        &receipt,
    )?;
    let journal = configuration
        .transaction_directory
        .join(FIRST_INSTALL_JOURNAL_DIRECTORY);
    if existing_journal {
        canonical_directory(&journal, configuration.owner_uid, 0o700)?;
    } else {
        fs::create_dir(&journal).map_err(|_| refuse("journal directory could not be created"))?;
        fs::set_permissions(&journal, fs::Permissions::from_mode(0o700))
            .map_err(|_| refuse("journal directory mode could not be closed"))?;
        sync_directory(&configuration.transaction_directory)?;
    }
    Ok(CheckedFirstInstallV1 {
        approval,
        journal,
        candidate,
        label: configuration.label.clone(),
        control_socket: configuration.control_socket.clone(),
        health_attempts: configuration.max_health_attempts,
        health_interval: Duration::from_millis(configuration.health_interval_ms),
        effect_timeout: Duration::from_millis(configuration.effect_timeout_ms),
        owner_uid: configuration.owner_uid,
    })
}

fn sync_directory(path: &Path) -> Result<(), ActivationRefusalV1> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|_| refuse("directory synchronization failed"))
}

fn write_journal(
    checked: &CheckedActivationV1,
    sequence: &mut u64,
    phase: &'static str,
    effect: &'static str,
    disposition: &'static str,
    selected: &str,
) -> Result<(), ActivationRefusalV1> {
    let entry = JournalEntryV1 {
        schema: 1,
        channel: "vellum-launchd-activation-journal",
        sequence: *sequence,
        transaction_id: &checked.approval.transaction_id,
        phase,
        effect,
        disposition,
        candidate_manifest_sha256: &checked.candidate.manifest,
        prior_manifest_sha256: &checked.prior.manifest,
        selected_manifest_sha256: selected,
        content_disclosed: false,
    };
    let file = checked.journal.join(format!("{:04}.json", *sequence));
    let bytes = serde_json::to_vec_pretty(&entry).map_err(|_| refuse("journal encoding failed"))?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o400)
        .open(&file)
        .map_err(|_| refuse("journal entry could not be created"))?;
    output
        .write_all(&bytes)
        .and_then(|()| output.write_all(b"\n"))
        .and_then(|()| output.sync_all())
        .map_err(|_| refuse("journal entry could not be synchronized"))?;
    fs::set_permissions(&file, fs::Permissions::from_mode(0o400))
        .map_err(|_| refuse("journal entry mode could not be closed"))?;
    sync_directory(&checked.journal)?;
    *sequence = sequence
        .checked_add(1)
        .ok_or_else(|| refuse("journal sequence exhausted"))?;
    Ok(())
}

fn write_first_install_journal(
    checked: &CheckedFirstInstallV1,
    sequence: &mut u64,
    phase: &'static str,
    effect: &'static str,
    disposition: &'static str,
) -> Result<(), ActivationRefusalV1> {
    let entry = FirstInstallJournalEntryV1 {
        schema: 1,
        channel: "vellum-launchd-first-install-journal",
        sequence: *sequence,
        transaction_id: &checked.approval.transaction_id,
        phase,
        effect,
        disposition,
        candidate_manifest_sha256: &checked.candidate.manifest,
        selected_manifest_sha256: &checked.candidate.manifest,
        content_disclosed: false,
    };
    let file = checked.journal.join(format!("{:04}.json", *sequence));
    let bytes = serde_json::to_vec_pretty(&entry).map_err(|_| refuse("journal encoding failed"))?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o400)
        .open(&file)
        .map_err(|_| refuse("journal entry could not be created"))?;
    output
        .write_all(&bytes)
        .and_then(|()| output.write_all(b"\n"))
        .and_then(|()| output.sync_all())
        .map_err(|_| refuse("journal entry could not be synchronized"))?;
    fs::set_permissions(&file, fs::Permissions::from_mode(0o400))
        .map_err(|_| refuse("journal entry mode could not be closed"))?;
    sync_directory(&checked.journal)?;
    *sequence = sequence
        .checked_add(1)
        .ok_or_else(|| refuse("journal sequence exhausted"))?;
    Ok(())
}

fn first_install_journal_shape_valid(record: &FirstInstallJournalRecordV1) -> bool {
    match (
        record.phase.as_str(),
        record.effect.as_str(),
        record.disposition.as_str(),
    ) {
        ("prepared", "none", "not_attempted") => true,
        ("candidate_load", "launchctl_bootstrap", disposition)
        | ("candidate_unload", "launchctl_bootout", disposition) => matches!(
            disposition,
            "pending" | "confirmed" | "refused" | "outcome_unknown"
        ),
        ("terminal", "candidate_health", "confirmed")
        | ("terminal", "candidate_cleanup", "confirmed")
        | ("terminal", "none", "existing_job" | "load_refused") => true,
        _ => false,
    }
}

fn first_install_journal_transition_valid(
    previous: &FirstInstallJournalRecordV1,
    next: &FirstInstallJournalRecordV1,
) -> bool {
    if previous.disposition == "pending" {
        return next.phase == previous.phase
            && next.effect == previous.effect
            && matches!(
                next.disposition.as_str(),
                "confirmed" | "refused" | "outcome_unknown"
            );
    }
    match (previous.phase.as_str(), previous.disposition.as_str()) {
        ("prepared", "not_attempted") => {
            (next.phase == "candidate_load" && next.disposition == "pending")
                || (next.phase == "terminal" && next.disposition == "existing_job")
        }
        ("candidate_load", "confirmed") => {
            (next.phase == "terminal"
                && next.effect == "candidate_health"
                && next.disposition == "confirmed")
                || (next.phase == "candidate_unload" && next.disposition == "pending")
        }
        ("candidate_load", "refused") => {
            next.phase == "terminal" && next.effect == "none" && next.disposition == "load_refused"
        }
        ("candidate_unload", "confirmed") => {
            next.phase == "terminal"
                && next.effect == "candidate_cleanup"
                && next.disposition == "confirmed"
        }
        _ => false,
    }
}

fn read_first_install_journal(
    checked: &CheckedFirstInstallV1,
) -> Result<Vec<FirstInstallJournalRecordV1>, ActivationRefusalV1> {
    let mut entries = fs::read_dir(&checked.journal)
        .map_err(|_| refuse("journal directory could not be read"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| refuse("journal directory could not be read"))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    if entries.is_empty() || entries.len() > 8 {
        return Err(refuse("journal entry count is incompatible"));
    }
    let mut records = Vec::with_capacity(entries.len());
    for (index, entry) in entries.into_iter().enumerate() {
        let sequence = u64::try_from(index).map_err(|_| refuse("journal sequence overflow"))?;
        let expected_name = format!("{sequence:04}.json");
        if entry.file_name().to_str() != Some(expected_name.as_str()) {
            return Err(refuse("journal filenames are not a contiguous sequence"));
        }
        let path = entry.path();
        canonical_regular_file(&path, checked.owner_uid, Some(0o400), false)?;
        let bytes = fs::read(&path).map_err(|_| refuse("journal entry could not be read"))?;
        let record: FirstInstallJournalRecordV1 = serde_json::from_slice(&bytes)
            .map_err(|_| refuse("journal entry JSON is incompatible"))?;
        let mut canonical =
            serde_json::to_vec_pretty(&record).map_err(|_| refuse("journal encoding failed"))?;
        canonical.push(b'\n');
        if bytes != canonical
            || record.schema != 1
            || record.channel != "vellum-launchd-first-install-journal"
            || record.sequence != sequence
            || record.transaction_id != checked.approval.transaction_id
            || record.candidate_manifest_sha256 != checked.candidate.manifest
            || record.selected_manifest_sha256 != checked.candidate.manifest
            || record.content_disclosed
            || !first_install_journal_shape_valid(&record)
            || records
                .last()
                .is_some_and(|previous| !first_install_journal_transition_valid(previous, &record))
        {
            return Err(refuse("journal evidence is incompatible"));
        }
        records.push(record);
    }
    if records[0].phase != "prepared" {
        return Err(refuse("journal genesis is incompatible"));
    }
    Ok(records)
}

fn journal_record_shape_valid(record: &JournalRecordV1, checked: &CheckedActivationV1) -> bool {
    let prior = record.selected_manifest_sha256 == checked.prior.manifest;
    let candidate = record.selected_manifest_sha256 == checked.candidate.manifest;
    match (
        record.phase.as_str(),
        record.effect.as_str(),
        record.disposition.as_str(),
    ) {
        ("prepared", "none", "not_attempted") => prior,
        ("prior_unload", "launchctl_bootout", disposition) => {
            prior
                && matches!(
                    disposition,
                    "pending" | "confirmed" | "refused" | "outcome_unknown"
                )
        }
        ("candidate_load", "launchctl_bootstrap", disposition) => {
            candidate
                && matches!(
                    disposition,
                    "pending" | "confirmed" | "refused" | "outcome_unknown"
                )
        }
        ("candidate_unload", "launchctl_bootout", disposition) => {
            candidate
                && matches!(
                    disposition,
                    "pending" | "confirmed" | "refused" | "outcome_unknown"
                )
        }
        ("prior_load", "launchctl_bootstrap", disposition) => {
            prior
                && matches!(
                    disposition,
                    "pending" | "confirmed" | "refused" | "outcome_unknown"
                )
        }
        ("terminal", "candidate_health", "confirmed") => candidate,
        ("terminal", "prior_health", "confirmed" | "refused") => prior,
        ("terminal", "none", "prior_not_ready") => prior,
        _ => false,
    }
}

fn journal_transition_valid(previous: &JournalRecordV1, next: &JournalRecordV1) -> bool {
    if previous.disposition == "pending" {
        return next.phase == previous.phase
            && next.effect == previous.effect
            && next.selected_manifest_sha256 == previous.selected_manifest_sha256
            && matches!(
                next.disposition.as_str(),
                "confirmed" | "refused" | "outcome_unknown"
            );
    }
    match (previous.phase.as_str(), previous.disposition.as_str()) {
        ("prepared", "not_attempted") => {
            (next.phase == "prior_unload" && next.disposition == "pending")
                || (next.phase == "terminal" && next.disposition == "prior_not_ready")
        }
        ("prior_unload", "confirmed") => {
            next.phase == "candidate_load" && next.disposition == "pending"
        }
        ("candidate_load", "confirmed") => {
            (next.phase == "terminal"
                && next.effect == "candidate_health"
                && next.disposition == "confirmed")
                || (next.phase == "candidate_unload" && next.disposition == "pending")
        }
        ("candidate_load", "refused") => {
            next.phase == "prior_load" && next.disposition == "pending"
        }
        ("candidate_unload", "confirmed") => {
            next.phase == "prior_load" && next.disposition == "pending"
        }
        ("prior_load", "confirmed" | "refused") => {
            next.phase == "terminal"
                && next.effect == "prior_health"
                && matches!(next.disposition.as_str(), "confirmed" | "refused")
        }
        _ => false,
    }
}

fn read_journal(
    checked: &CheckedActivationV1,
) -> Result<Vec<JournalRecordV1>, ActivationRefusalV1> {
    let mut entries = fs::read_dir(&checked.journal)
        .map_err(|_| refuse("journal directory could not be read"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| refuse("journal directory could not be read"))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    if entries.is_empty() || entries.len() > 16 {
        return Err(refuse("journal entry count is incompatible"));
    }
    let mut records = Vec::with_capacity(entries.len());
    for (index, entry) in entries.into_iter().enumerate() {
        let sequence = u64::try_from(index).map_err(|_| refuse("journal sequence overflow"))?;
        let expected_name = format!("{sequence:04}.json");
        if entry.file_name().to_str() != Some(expected_name.as_str()) {
            return Err(refuse("journal filenames are not a contiguous sequence"));
        }
        let path = entry.path();
        canonical_regular_file(&path, checked.owner_uid, Some(0o400), false)?;
        let bytes = fs::read(&path).map_err(|_| refuse("journal entry could not be read"))?;
        let record: JournalRecordV1 = serde_json::from_slice(&bytes)
            .map_err(|_| refuse("journal entry JSON is incompatible"))?;
        let mut canonical =
            serde_json::to_vec_pretty(&record).map_err(|_| refuse("journal encoding failed"))?;
        canonical.push(b'\n');
        if bytes != canonical
            || record.schema != 1
            || record.channel != "vellum-launchd-activation-journal"
            || record.sequence != sequence
            || record.transaction_id != checked.approval.transaction_id
            || record.candidate_manifest_sha256 != checked.candidate.manifest
            || record.prior_manifest_sha256 != checked.prior.manifest
            || record.content_disclosed
            || !journal_record_shape_valid(&record, checked)
            || records
                .last()
                .is_some_and(|previous| !journal_transition_valid(previous, &record))
        {
            return Err(refuse("journal evidence is incompatible"));
        }
        records.push(record);
    }
    if records[0].phase != "prepared" {
        return Err(refuse("journal genesis is incompatible"));
    }
    Ok(records)
}

/// Reads and validates one exact durable activation journal without invoking
/// launchd, the daemon, or any retry path. Nonterminal evidence always requires
/// explicit operator reconciliation before a separately authorized new action.
pub fn reconcile_activation_journal_v1(
    configuration: &ActivationConfigurationV1,
) -> Result<ActivationReconciliationV1, ActivationRefusalV1> {
    let checked = prepare(configuration, true)?;
    let records = read_journal(&checked)?;
    let last = records.last().expect("nonempty journal was validated");
    let (state, required_action) = match (
        last.phase.as_str(),
        last.effect.as_str(),
        last.disposition.as_str(),
    ) {
        ("terminal", "candidate_health", "confirmed") => (
            ActivationReconciliationStateV1::CandidateActiveDurable,
            ActivationReconciliationActionV1::None,
        ),
        ("terminal", "prior_health", "confirmed") => (
            ActivationReconciliationStateV1::PriorRestoredDurable,
            ActivationReconciliationActionV1::None,
        ),
        ("terminal", "none", "prior_not_ready") => (
            ActivationReconciliationStateV1::ActivationRefusedDurable,
            ActivationReconciliationActionV1::None,
        ),
        ("terminal", "prior_health", "refused") => (
            ActivationReconciliationStateV1::RollbackRefusedDurable,
            ActivationReconciliationActionV1::None,
        ),
        (_, _, "outcome_unknown") => (
            ActivationReconciliationStateV1::OutcomeUnknown,
            ActivationReconciliationActionV1::InspectExactJobAndDaemonStatus,
        ),
        (_, _, "pending") => (
            ActivationReconciliationStateV1::PendingEffect,
            ActivationReconciliationActionV1::InspectExactJobAndDaemonStatus,
        ),
        ("prepared", _, _) => (
            ActivationReconciliationStateV1::PreparedOnly,
            ActivationReconciliationActionV1::ManualReauthorizationRequired,
        ),
        _ => (
            ActivationReconciliationStateV1::ConfirmedEffectWithoutDurableTerminal,
            ActivationReconciliationActionV1::InspectExactJobAndDaemonStatus,
        ),
    };
    Ok(ActivationReconciliationV1 {
        schema: 1,
        channel: "vellum-launchd-activation-reconciliation",
        transaction_id: checked.approval.transaction_id,
        candidate_manifest_sha256: checked.candidate.manifest,
        prior_manifest_sha256: checked.prior.manifest,
        selected_manifest_sha256: last.selected_manifest_sha256.clone(),
        journal_entries: u64::try_from(records.len())
            .map_err(|_| refuse("journal entry count overflow"))?,
        last_sequence: last.sequence,
        last_phase: last.phase.clone(),
        last_effect: last.effect.clone(),
        last_disposition: last.disposition.clone(),
        state,
        required_action,
        automatic_retry_authorized: false,
        launchd_effect: "not_attempted",
        content_disclosed: false,
    })
}

/// Reconciles one first-install journal without invoking launchd, daemon
/// status, cleanup, or retry effects.
pub fn reconcile_first_install_journal_v1(
    configuration: &FirstInstallConfigurationV1,
) -> Result<FirstInstallReconciliationV1, ActivationRefusalV1> {
    let checked = prepare_first_install(configuration, true)?;
    let records = read_first_install_journal(&checked)?;
    let last = records.last().expect("nonempty journal was validated");
    let (state, required_action) = match (
        last.phase.as_str(),
        last.effect.as_str(),
        last.disposition.as_str(),
    ) {
        ("terminal", "candidate_health", "confirmed") => (
            FirstInstallReconciliationStateV1::CandidateActiveDurable,
            ActivationReconciliationActionV1::None,
        ),
        ("terminal", "candidate_cleanup", "confirmed")
        | ("terminal", "none", "existing_job" | "load_refused") => (
            FirstInstallReconciliationStateV1::InstallationRefusedDurable,
            ActivationReconciliationActionV1::None,
        ),
        (_, _, "outcome_unknown") => (
            FirstInstallReconciliationStateV1::OutcomeUnknown,
            ActivationReconciliationActionV1::InspectExactJobAndDaemonStatus,
        ),
        (_, _, "pending") => (
            FirstInstallReconciliationStateV1::PendingEffect,
            ActivationReconciliationActionV1::InspectExactJobAndDaemonStatus,
        ),
        ("prepared", _, _) => (
            FirstInstallReconciliationStateV1::PreparedOnly,
            ActivationReconciliationActionV1::ManualReauthorizationRequired,
        ),
        _ => (
            FirstInstallReconciliationStateV1::ConfirmedEffectWithoutDurableTerminal,
            ActivationReconciliationActionV1::InspectExactJobAndDaemonStatus,
        ),
    };
    Ok(FirstInstallReconciliationV1 {
        schema: 1,
        channel: "vellum-launchd-first-install-reconciliation",
        transaction_id: checked.approval.transaction_id,
        candidate_manifest_sha256: checked.candidate.manifest,
        selected_manifest_sha256: last.selected_manifest_sha256.clone(),
        journal_entries: u64::try_from(records.len())
            .map_err(|_| refuse("journal entry count overflow"))?,
        last_sequence: last.sequence,
        last_phase: last.phase.clone(),
        last_effect: last.effect.clone(),
        last_disposition: last.disposition.clone(),
        state,
        required_action,
        automatic_retry_authorized: false,
        launchd_effect: "not_attempted",
        content_disclosed: false,
    })
}

fn disposition_name(disposition: LaunchdEffectDispositionV1) -> &'static str {
    match disposition {
        LaunchdEffectDispositionV1::Confirmed => "confirmed",
        LaunchdEffectDispositionV1::Refused => "refused",
        LaunchdEffectDispositionV1::OutcomeUnknown => "outcome_unknown",
    }
}

fn receipt(
    checked: &CheckedActivationV1,
    selected: &str,
    outcome: ActivationOutcomeV1,
    identity: &'static str,
    status: &'static str,
) -> ActivationReceiptV1 {
    ActivationReceiptV1 {
        schema: 1,
        channel: "vellum-launchd-activation-receipt",
        transaction_id: checked.approval.transaction_id.clone(),
        candidate_manifest_sha256: checked.candidate.manifest.clone(),
        prior_manifest_sha256: checked.prior.manifest.clone(),
        selected_manifest_sha256: selected.to_owned(),
        outcome,
        launchd_effect: "durably_bracketed",
        process_identity: identity,
        daemon_status: status,
        content_disclosed: false,
    }
}

fn first_install_receipt(
    checked: &CheckedFirstInstallV1,
    outcome: FirstInstallOutcomeV1,
    identity: &'static str,
    status: &'static str,
) -> FirstInstallReceiptV1 {
    FirstInstallReceiptV1 {
        schema: 1,
        channel: "vellum-launchd-first-install-receipt",
        transaction_id: checked.approval.transaction_id.clone(),
        candidate_manifest_sha256: checked.candidate.manifest.clone(),
        selected_manifest_sha256: checked.candidate.manifest.clone(),
        outcome,
        launchd_effect: "durably_bracketed",
        process_identity: identity,
        daemon_status: status,
        content_disclosed: false,
    }
}

fn healthy<P: LaunchdActivationPortV1>(
    checked: &CheckedActivationV1,
    version: &CheckedVersionV1,
    port: &mut P,
) -> bool {
    for attempt in 0..checked.health_attempts {
        let identity = port.inspect_job(&checked.label);
        let status = port.daemon_status(
            &version.daemon,
            &checked.control_socket,
            checked.effect_timeout,
        );
        if matches!(
            identity,
            LaunchdObservationV1::Observed(LaunchdJobIdentityV1 { pid, ref program })
                if pid != 0 && program == &version.daemon
        ) && status == LaunchdObservationV1::Observed(DaemonStatusV1::Ready)
        {
            return true;
        }
        if attempt + 1 != checked.health_attempts {
            port.wait_before_status_retry(checked.health_interval);
        }
    }
    false
}

fn first_install_healthy<P: LaunchdActivationPortV1>(
    checked: &CheckedFirstInstallV1,
    port: &mut P,
) -> bool {
    for attempt in 0..checked.health_attempts {
        let identity = port.inspect_job(&checked.label);
        let status = port.daemon_status(
            &checked.candidate.daemon,
            &checked.control_socket,
            checked.effect_timeout,
        );
        if matches!(
            identity,
            LaunchdObservationV1::Observed(LaunchdJobIdentityV1 { pid, ref program })
                if pid != 0 && program == &checked.candidate.daemon
        ) && status == LaunchdObservationV1::Observed(DaemonStatusV1::Ready)
        {
            return true;
        }
        if attempt + 1 != checked.health_attempts {
            port.wait_before_status_retry(checked.health_interval);
        }
    }
    false
}

fn effect<P: LaunchdActivationPortV1>(
    checked: &CheckedActivationV1,
    sequence: &mut u64,
    port: &mut P,
    phase: &'static str,
    effect_name: &'static str,
    selected: &str,
    invoke: impl FnOnce(&mut P) -> LaunchdEffectDispositionV1,
) -> Result<LaunchdEffectDispositionV1, ActivationRefusalV1> {
    write_journal(checked, sequence, phase, effect_name, "pending", selected)?;
    let disposition = invoke(port);
    if write_journal(
        checked,
        sequence,
        phase,
        effect_name,
        disposition_name(disposition),
        selected,
    )
    .is_err()
    {
        return Ok(LaunchdEffectDispositionV1::OutcomeUnknown);
    }
    Ok(disposition)
}

fn first_install_effect<P: LaunchdActivationPortV1>(
    checked: &CheckedFirstInstallV1,
    sequence: &mut u64,
    port: &mut P,
    phase: &'static str,
    effect_name: &'static str,
    invoke: impl FnOnce(&mut P) -> LaunchdEffectDispositionV1,
) -> Result<LaunchdEffectDispositionV1, ActivationRefusalV1> {
    write_first_install_journal(checked, sequence, phase, effect_name, "pending")?;
    let disposition = invoke(port);
    if write_first_install_journal(
        checked,
        sequence,
        phase,
        effect_name,
        disposition_name(disposition),
    )
    .is_err()
    {
        return Ok(LaunchdEffectDispositionV1::OutcomeUnknown);
    }
    Ok(disposition)
}

pub fn first_install_v1<P: LaunchdActivationPortV1>(
    configuration: &FirstInstallConfigurationV1,
    port: &mut P,
) -> Result<FirstInstallReceiptV1, ActivationRefusalV1> {
    let checked = prepare_first_install(configuration, false)?;
    let mut sequence = 0;
    write_first_install_journal(&checked, &mut sequence, "prepared", "none", "not_attempted")?;

    if matches!(
        port.inspect_job(&checked.label),
        LaunchdObservationV1::Observed(_)
    ) {
        write_first_install_journal(&checked, &mut sequence, "terminal", "none", "existing_job")?;
        return Ok(first_install_receipt(
            &checked,
            FirstInstallOutcomeV1::InstallationRefused,
            "existing_job_observed",
            "not_checked",
        ));
    }

    let candidate_load = first_install_effect(
        &checked,
        &mut sequence,
        port,
        "candidate_load",
        "launchctl_bootstrap",
        |port| {
            port.load_job(
                &checked.label,
                &checked.candidate.plist,
                checked.effect_timeout,
            )
        },
    )?;
    match candidate_load {
        LaunchdEffectDispositionV1::OutcomeUnknown => {
            return Ok(first_install_receipt(
                &checked,
                FirstInstallOutcomeV1::OutcomeUnknown,
                "unconfirmed",
                "not_checked",
            ));
        }
        LaunchdEffectDispositionV1::Refused => {
            write_first_install_journal(
                &checked,
                &mut sequence,
                "terminal",
                "none",
                "load_refused",
            )?;
            return Ok(first_install_receipt(
                &checked,
                FirstInstallOutcomeV1::InstallationRefused,
                "not_started",
                "not_checked",
            ));
        }
        LaunchdEffectDispositionV1::Confirmed => {}
    }
    if first_install_healthy(&checked, port) {
        if write_first_install_journal(
            &checked,
            &mut sequence,
            "terminal",
            "candidate_health",
            "confirmed",
        )
        .is_err()
        {
            return Ok(first_install_receipt(
                &checked,
                FirstInstallOutcomeV1::OutcomeUnknown,
                "exact_program_and_pid",
                "running_healthy_ready_not_durable",
            ));
        }
        return Ok(first_install_receipt(
            &checked,
            FirstInstallOutcomeV1::CandidateActive,
            "exact_program_and_pid",
            "running_healthy_ready",
        ));
    }

    let candidate_unload = first_install_effect(
        &checked,
        &mut sequence,
        port,
        "candidate_unload",
        "launchctl_bootout",
        |port| port.unload_job(&checked.label, checked.effect_timeout),
    )?;
    if candidate_unload != LaunchdEffectDispositionV1::Confirmed {
        return Ok(first_install_receipt(
            &checked,
            FirstInstallOutcomeV1::OutcomeUnknown,
            "candidate_unconfirmed",
            "candidate_not_ready",
        ));
    }
    if write_first_install_journal(
        &checked,
        &mut sequence,
        "terminal",
        "candidate_cleanup",
        "confirmed",
    )
    .is_err()
    {
        return Ok(first_install_receipt(
            &checked,
            FirstInstallOutcomeV1::OutcomeUnknown,
            "candidate_absent_not_durable",
            "candidate_not_ready",
        ));
    }
    Ok(first_install_receipt(
        &checked,
        FirstInstallOutcomeV1::InstallationRefused,
        "candidate_absent",
        "candidate_not_ready",
    ))
}

pub fn activate_v1<P: LaunchdActivationPortV1>(
    configuration: &ActivationConfigurationV1,
    port: &mut P,
) -> Result<ActivationReceiptV1, ActivationRefusalV1> {
    let checked = prepare(configuration, false)?;
    let mut sequence = 0;
    write_journal(
        &checked,
        &mut sequence,
        "prepared",
        "none",
        "not_attempted",
        &checked.prior.manifest,
    )?;

    if !healthy(&checked, &checked.prior, port) {
        write_journal(
            &checked,
            &mut sequence,
            "terminal",
            "none",
            "prior_not_ready",
            &checked.prior.manifest,
        )?;
        return Ok(receipt(
            &checked,
            &checked.prior.manifest,
            ActivationOutcomeV1::ActivationRefused,
            "prior_unverified",
            "prior_not_ready",
        ));
    }

    let prior_unload = effect(
        &checked,
        &mut sequence,
        port,
        "prior_unload",
        "launchctl_bootout",
        &checked.prior.manifest,
        |port| port.unload_job(&checked.label, checked.effect_timeout),
    )?;
    if prior_unload != LaunchdEffectDispositionV1::Confirmed {
        let outcome = if prior_unload == LaunchdEffectDispositionV1::OutcomeUnknown {
            ActivationOutcomeV1::OutcomeUnknown
        } else {
            ActivationOutcomeV1::ActivationRefused
        };
        return Ok(receipt(
            &checked,
            &checked.prior.manifest,
            outcome,
            "prior_confirmed_before_effect",
            "not_checked",
        ));
    }

    let candidate_load = effect(
        &checked,
        &mut sequence,
        port,
        "candidate_load",
        "launchctl_bootstrap",
        &checked.candidate.manifest,
        |port| {
            port.load_job(
                &checked.label,
                &checked.candidate.plist,
                checked.effect_timeout,
            )
        },
    )?;
    if candidate_load == LaunchdEffectDispositionV1::OutcomeUnknown {
        return Ok(receipt(
            &checked,
            &checked.candidate.manifest,
            ActivationOutcomeV1::OutcomeUnknown,
            "unconfirmed",
            "not_checked",
        ));
    }
    let candidate_healthy = candidate_load == LaunchdEffectDispositionV1::Confirmed
        && healthy(&checked, &checked.candidate, port);
    if candidate_healthy {
        if write_journal(
            &checked,
            &mut sequence,
            "terminal",
            "candidate_health",
            "confirmed",
            &checked.candidate.manifest,
        )
        .is_err()
        {
            return Ok(receipt(
                &checked,
                &checked.candidate.manifest,
                ActivationOutcomeV1::OutcomeUnknown,
                "exact_program_and_pid",
                "running_healthy_ready_not_durable",
            ));
        }
        return Ok(receipt(
            &checked,
            &checked.candidate.manifest,
            ActivationOutcomeV1::CandidateActive,
            "exact_program_and_pid",
            "running_healthy_ready",
        ));
    }

    if candidate_load == LaunchdEffectDispositionV1::Confirmed {
        let candidate_unload = effect(
            &checked,
            &mut sequence,
            port,
            "candidate_unload",
            "launchctl_bootout",
            &checked.candidate.manifest,
            |port| port.unload_job(&checked.label, checked.effect_timeout),
        )?;
        if candidate_unload != LaunchdEffectDispositionV1::Confirmed {
            let outcome = if candidate_unload == LaunchdEffectDispositionV1::OutcomeUnknown {
                ActivationOutcomeV1::OutcomeUnknown
            } else {
                ActivationOutcomeV1::RollbackRefused
            };
            return Ok(receipt(
                &checked,
                &checked.candidate.manifest,
                outcome,
                "candidate_unconfirmed",
                "candidate_not_ready",
            ));
        }
    }

    let prior_load = effect(
        &checked,
        &mut sequence,
        port,
        "prior_load",
        "launchctl_bootstrap",
        &checked.prior.manifest,
        |port| port.load_job(&checked.label, &checked.prior.plist, checked.effect_timeout),
    )?;
    if prior_load == LaunchdEffectDispositionV1::OutcomeUnknown {
        return Ok(receipt(
            &checked,
            &checked.prior.manifest,
            ActivationOutcomeV1::OutcomeUnknown,
            "unconfirmed",
            "not_checked",
        ));
    }
    if prior_load != LaunchdEffectDispositionV1::Confirmed
        || !healthy(&checked, &checked.prior, port)
    {
        if write_journal(
            &checked,
            &mut sequence,
            "terminal",
            "prior_health",
            "refused",
            &checked.prior.manifest,
        )
        .is_err()
        {
            return Ok(receipt(
                &checked,
                &checked.prior.manifest,
                ActivationOutcomeV1::OutcomeUnknown,
                "prior_unconfirmed",
                "prior_not_ready_not_durable",
            ));
        }
        return Ok(receipt(
            &checked,
            &checked.prior.manifest,
            ActivationOutcomeV1::RollbackRefused,
            "prior_unconfirmed",
            "prior_not_ready",
        ));
    }
    if write_journal(
        &checked,
        &mut sequence,
        "terminal",
        "prior_health",
        "confirmed",
        &checked.prior.manifest,
    )
    .is_err()
    {
        return Ok(receipt(
            &checked,
            &checked.prior.manifest,
            ActivationOutcomeV1::OutcomeUnknown,
            "exact_program_and_pid",
            "running_healthy_ready_not_durable",
        ));
    }
    Ok(receipt(
        &checked,
        &checked.prior.manifest,
        ActivationOutcomeV1::PriorRestored,
        "exact_program_and_pid",
        "running_healthy_ready",
    ))
}

pub fn decode_configuration_v1(
    bytes: &[u8],
) -> Result<ActivationConfigurationV1, ActivationRefusalV1> {
    serde_json::from_slice(bytes)
        .map_err(|_| refuse("activation configuration JSON is incompatible"))
}

pub fn read_configuration_file_v1(
    path: &Path,
) -> Result<ActivationConfigurationV1, ActivationRefusalV1> {
    if !normalized_absolute(path) || fs::canonicalize(path).ok().as_deref() != Some(path) {
        return Err(refuse("activation configuration path is not canonical"));
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| refuse("activation configuration could not be inspected"))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o777 != 0o600
    {
        return Err(refuse(
            "activation configuration ownership, mode or type is incompatible",
        ));
    }
    let bytes = fs::read(path).map_err(|_| refuse("activation configuration could not be read"))?;
    let configuration = decode_configuration_v1(&bytes)?;
    if configuration.owner_uid != metadata.uid() {
        return Err(refuse("activation configuration owner differs"));
    }
    Ok(configuration)
}

pub fn decode_first_install_configuration_v1(
    bytes: &[u8],
) -> Result<FirstInstallConfigurationV1, ActivationRefusalV1> {
    serde_json::from_slice(bytes)
        .map_err(|_| refuse("first-install configuration JSON is incompatible"))
}

pub fn read_first_install_configuration_file_v1(
    path: &Path,
) -> Result<FirstInstallConfigurationV1, ActivationRefusalV1> {
    if !normalized_absolute(path) || fs::canonicalize(path).ok().as_deref() != Some(path) {
        return Err(refuse("first-install configuration path is not canonical"));
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| refuse("first-install configuration could not be inspected"))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o777 != 0o600
    {
        return Err(refuse(
            "first-install configuration ownership, mode or type is incompatible",
        ));
    }
    let bytes =
        fs::read(path).map_err(|_| refuse("first-install configuration could not be read"))?;
    let configuration = decode_first_install_configuration_v1(&bytes)?;
    if configuration.owner_uid != metadata.uid() {
        return Err(refuse("first-install configuration owner differs"));
    }
    Ok(configuration)
}

struct CommandResultV1 {
    status: std::process::ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

enum CommandDispositionV1 {
    Complete(CommandResultV1),
    NotStarted,
    OutcomeUnknown,
}

fn run_bounded(command: &mut Command, timeout: Duration) -> CommandDispositionV1 {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let Ok(mut child) = command.spawn() else {
        return CommandDispositionV1::NotStarted;
    };
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => match child.wait_with_output() {
                Ok(output)
                    if output.stdout.len() <= MAX_EFFECT_OUTPUT_BYTES
                        && output.stderr.len() <= MAX_EFFECT_OUTPUT_BYTES =>
                {
                    return CommandDispositionV1::Complete(CommandResultV1 {
                        status: output.status,
                        stdout: output.stdout,
                        stderr: output.stderr,
                    });
                }
                Ok(_) | Err(_) => return CommandDispositionV1::OutcomeUnknown,
            },
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return CommandDispositionV1::OutcomeUnknown;
            }
        }
    }
}

/// Real macOS command adapter. Constructing it has no effect; only trait calls
/// execute the fixed absolute launchctl path or an already-qualified daemon.
pub struct MacOsLaunchdPortV1 {
    owner_uid: u32,
}

impl MacOsLaunchdPortV1 {
    pub fn try_new(owner_uid: u32) -> Option<Self> {
        (cfg!(target_os = "macos") && rustix::process::geteuid().as_raw() == owner_uid)
            .then_some(Self { owner_uid })
    }

    fn domain(&self) -> String {
        format!("gui/{}", self.owner_uid)
    }

    fn target(&self, label: &str) -> String {
        format!("{}/{}", self.domain(), label)
    }
}

impl LaunchdActivationPortV1 for MacOsLaunchdPortV1 {
    fn inspect_job(&mut self, label: &str) -> LaunchdObservationV1<LaunchdJobIdentityV1> {
        let mut command = Command::new("/bin/launchctl");
        command.args(["print", &self.target(label)]);
        let CommandDispositionV1::Complete(output) =
            run_bounded(&mut command, Duration::from_secs(5))
        else {
            return LaunchdObservationV1::Unavailable;
        };
        if !output.status.success() || !output.stderr.is_empty() {
            return LaunchdObservationV1::Unavailable;
        }
        let Ok(text) = std::str::from_utf8(&output.stdout) else {
            return LaunchdObservationV1::Unavailable;
        };
        let mut program = None;
        let mut pid = None;
        for line in text.lines().map(str::trim) {
            if let Some(value) = line.strip_prefix("program = ") {
                program = Some(PathBuf::from(value));
            } else if let Some(value) = line.strip_prefix("pid = ") {
                pid = value.parse::<u32>().ok().filter(|pid| *pid != 0);
            }
        }
        match (program, pid) {
            (Some(program), Some(pid)) => {
                LaunchdObservationV1::Observed(LaunchdJobIdentityV1 { pid, program })
            }
            _ => LaunchdObservationV1::Unavailable,
        }
    }

    fn unload_job(&mut self, label: &str, timeout: Duration) -> LaunchdEffectDispositionV1 {
        let mut command = Command::new("/bin/launchctl");
        command.args(["bootout", &self.target(label)]);
        match run_bounded(&mut command, timeout) {
            CommandDispositionV1::Complete(output)
                if output.status.success() && output.stderr.is_empty() =>
            {
                LaunchdEffectDispositionV1::Confirmed
            }
            CommandDispositionV1::NotStarted => LaunchdEffectDispositionV1::Refused,
            CommandDispositionV1::Complete(_) | CommandDispositionV1::OutcomeUnknown => {
                LaunchdEffectDispositionV1::OutcomeUnknown
            }
        }
    }

    fn load_job(
        &mut self,
        _label: &str,
        plist: &Path,
        timeout: Duration,
    ) -> LaunchdEffectDispositionV1 {
        let mut command = Command::new("/bin/launchctl");
        command.arg("bootstrap").arg(self.domain()).arg(plist);
        match run_bounded(&mut command, timeout) {
            CommandDispositionV1::Complete(output)
                if output.status.success() && output.stderr.is_empty() =>
            {
                LaunchdEffectDispositionV1::Confirmed
            }
            CommandDispositionV1::NotStarted => LaunchdEffectDispositionV1::Refused,
            CommandDispositionV1::Complete(_) | CommandDispositionV1::OutcomeUnknown => {
                LaunchdEffectDispositionV1::OutcomeUnknown
            }
        }
    }

    fn daemon_status(
        &mut self,
        daemon: &Path,
        control_socket: &Path,
        timeout: Duration,
    ) -> LaunchdObservationV1<DaemonStatusV1> {
        let mut command = Command::new(daemon);
        command.arg("--status").arg(control_socket);
        let CommandDispositionV1::Complete(output) = run_bounded(&mut command, timeout) else {
            return LaunchdObservationV1::Unavailable;
        };
        if !output.status.success() || !output.stderr.is_empty() {
            return LaunchdObservationV1::Unavailable;
        }
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
            return LaunchdObservationV1::Unavailable;
        };
        if value.get("version").and_then(serde_json::Value::as_u64) == Some(2)
            && value.get("liveness").and_then(serde_json::Value::as_str) == Some("live")
            && value.get("lifecycle").and_then(serde_json::Value::as_str) == Some("running")
            && value.get("health").and_then(serde_json::Value::as_str) == Some("healthy")
            && value.get("readiness").and_then(serde_json::Value::as_str) == Some("ready")
        {
            LaunchdObservationV1::Observed(DaemonStatusV1::Ready)
        } else {
            LaunchdObservationV1::Observed(DaemonStatusV1::NotReady)
        }
    }

    fn wait_before_status_retry(&mut self, interval: Duration) {
        thread::sleep(interval);
    }
}
