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
struct ActivationApprovalV1 {
    schema: u64,
    channel: String,
    transaction_id: String,
    requested_action: String,
    candidate_manifest_sha256: String,
    prior_manifest_sha256: String,
    activation_authorized: bool,
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
    trigger: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QualifiedArtifactV1 {
    file: String,
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

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn expected_plist(configuration: &ActivationConfigurationV1, daemon: &Path) -> Option<String> {
    let daemon = xml_escape(daemon.to_str()?);
    let data = xml_escape(configuration.data_socket.to_str()?);
    let control = xml_escape(configuration.control_socket.to_str()?);
    let stdout = xml_escape(configuration.stdout_log.to_str()?);
    let stderr = xml_escape(configuration.stderr_log.to_str()?);
    let label = xml_escape(&configuration.label);
    Some(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n  <dict>\n    <key>Label</key>\n    <string>{label}</string>\n    <key>ProgramArguments</key>\n    <array>\n      <string>{daemon}</string>\n      <string>--socket</string>\n      <string>{data}</string>\n      <string>--control</string>\n      <string>{control}</string>\n    </array>\n    <key>RunAtLoad</key>\n    <true/>\n    <key>KeepAlive</key>\n    <false/>\n    <key>ProcessType</key>\n    <string>Background</string>\n    <key>Umask</key>\n    <integer>63</integer>\n    <key>StandardOutPath</key>\n    <string>{stdout}</string>\n    <key>StandardErrorPath</key>\n    <string>{stderr}</string>\n  </dict>\n</plist>\n"
    ))
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
    if fs::read_dir(&configuration.transaction_directory)
        .map_err(|_| refuse("transaction directory could not be read"))?
        .next()
        .is_some()
    {
        return Err(refuse("transaction directory is not empty"));
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
        || evidence.candidate.deployment.len() != 1
        || evidence.rollback.deployment.len() != 1
        || evidence.candidate.smoke.is_null()
        || evidence.rollback.smoke.is_null()
        || evidence.candidate.launchd_smoke.is_null()
        || evidence.rollback.launchd_smoke.is_null()
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
    fs::create_dir(&journal).map_err(|_| refuse("journal directory could not be created"))?;
    fs::set_permissions(&journal, fs::Permissions::from_mode(0o700))
        .map_err(|_| refuse("journal directory mode could not be closed"))?;
    sync_directory(&configuration.transaction_directory)?;
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

pub fn activate_v1<P: LaunchdActivationPortV1>(
    configuration: &ActivationConfigurationV1,
    port: &mut P,
) -> Result<ActivationReceiptV1, ActivationRefusalV1> {
    let checked = prepare(configuration)?;
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
