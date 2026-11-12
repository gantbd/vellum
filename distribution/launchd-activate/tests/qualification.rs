use std::{
    collections::VecDeque,
    fs,
    os::unix::fs::{MetadataExt as _, PermissionsExt as _},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use serde_json::json;
use sha2::{Digest as _, Sha256};
use vellum_launchd_activate::{
    ActivationConfigurationV1, ActivationOutcomeV1, ActivationReconciliationActionV1,
    ActivationReconciliationStateV1, ActivationVersionInputV1, DaemonStatusV1,
    LaunchdActivationPortV1, LaunchdEffectDispositionV1, LaunchdJobIdentityV1,
    LaunchdObservationV1, activate_v1, decode_configuration_v1, reconcile_activation_journal_v1,
};
use vellum_launchd_render::{LaunchdConfigurationV1, render_launchd_profile_v1};

static SCRATCH_ORDINAL: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "vellum-launchd-activate-{}-{}",
            std::process::id(),
            SCRATCH_ORDINAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self(path.canonicalize().unwrap())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn write(path: &Path, bytes: impl AsRef<[u8]>, mode: u32) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

fn digest(path: &Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}

struct Fixture {
    _scratch: Scratch,
    configuration: ActivationConfigurationV1,
    candidate_daemon: PathBuf,
    prior_daemon: PathBuf,
    candidate_plist: PathBuf,
    prior_plist: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let scratch = Scratch::new();
        let owner = fs::metadata(&scratch.0).unwrap().uid();
        let state = scratch.0.join("state");
        let transaction = scratch.0.join("transaction");
        fs::create_dir(&state).unwrap();
        fs::create_dir(&transaction).unwrap();
        fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&transaction, fs::Permissions::from_mode(0o700)).unwrap();
        let candidate_dir = scratch.0.join("candidate");
        let prior_dir = scratch.0.join("prior");
        fs::create_dir(&candidate_dir).unwrap();
        fs::create_dir(&prior_dir).unwrap();
        fs::set_permissions(&candidate_dir, fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&prior_dir, fs::Permissions::from_mode(0o700)).unwrap();
        let candidate_daemon = candidate_dir.join("vellum-daemon");
        let prior_daemon = prior_dir.join("vellum-daemon");
        write(&candidate_daemon, b"candidate daemon\n", 0o500);
        write(&prior_daemon, b"prior daemon\n", 0o500);
        write(&candidate_dir.join("vellum-mcp"), b"candidate mcp\n", 0o500);
        write(&prior_dir.join("vellum-mcp"), b"prior mcp\n", 0o500);

        let data_socket = state.join("vellum.sock");
        let control_socket = state.join("vellum-control.sock");
        let stdout_log = state.join("stdout.log");
        let stderr_log = state.join("stderr.log");
        let label = "com.example.vellum".to_owned();
        let render = |binary: &Path| {
            render_launchd_profile_v1(&LaunchdConfigurationV1 {
                schema: 1,
                label: label.clone(),
                binary: binary.to_path_buf(),
                state_directory: state.clone(),
                data_socket: data_socket.clone(),
                control_socket: control_socket.clone(),
                lock_file: state.join(".vellum-daemon.lock"),
                metadata_file: state.join("workspace-identity-v1"),
                stdout_log: stdout_log.clone(),
                stderr_log: stderr_log.clone(),
                owner_uid: owner,
            })
            .unwrap()
        };
        let candidate_plist = candidate_dir.join("vellum.plist");
        let prior_plist = prior_dir.join("vellum.plist");
        write(&candidate_plist, render(&candidate_daemon), 0o600);
        write(&prior_plist, render(&prior_daemon), 0o600);

        let candidate_manifest = "a".repeat(64);
        let prior_manifest = "b".repeat(64);
        let artifact = |directory: &Path| {
            vec![
                json!({"file":"vellum-daemon","sha256":digest(&directory.join("vellum-daemon"))}),
                json!({"file":"vellum-mcp","sha256":digest(&directory.join("vellum-mcp"))}),
            ]
        };
        let evidence = scratch.0.join("qualification.json");
        write(
            &evidence,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&json!({
                    "schema":1,
                    "channel":"local-release-candidate-qualification",
                    "state":"ready-for-manual-activation-review",
                    "activationAuthorized":false,
                    "claims":[
                        "candidate-integrity-verified",
                        "candidate-launchd-render-smoke-passed",
                        "candidate-activation-executor-help-smoke-passed-without-effect",
                        "rollback-version-selected"
                    ],
                    "nonClaims":["activation"],
                    "candidate":{
                        "manifestSha256":candidate_manifest,
                        "vellumRevision":"c".repeat(40),
                        "artifacts":artifact(&candidate_dir),
                        "deployment":[
                            {"file":"vellum-launchd-render","sha256":"d".repeat(64)},
                            {"file":"vellum-launchd-activate","sha256":"1".repeat(64)}
                        ],
                        "smoke":[],
                        "launchdSmoke":{"exactInertProfile":true},
                        "activationExecutorSmoke":{"launchdEffect":"not-attempted"}
                    },
                    "rollback":{
                        "selectedPriorManifestSha256":prior_manifest,
                        "selectedPriorVellumRevision":"e".repeat(40),
                        "artifacts":artifact(&prior_dir),
                        "deployment":[
                            {"file":"vellum-launchd-render","sha256":"f".repeat(64)},
                            {"file":"vellum-launchd-activate","sha256":"2".repeat(64)}
                        ],
                        "smoke":[],
                        "launchdSmoke":{"exactInertProfile":true},
                        "activationExecutorSmoke":{"launchdEffect":"not-attempted"},
                        "trigger":"post-activation readiness or health gate fails"
                    },
                    "activationPlan":["explicit approval"]
                }))
                .unwrap()
            ),
            0o600,
        );
        let approval = scratch.0.join("approval.json");
        write(
            &approval,
            format!(
                concat!(
                    "{{\n",
                    "  \"schema\": 1,\n",
                    "  \"channel\": \"local-activation-approval\",\n",
                    "  \"transactionId\": \"{}\",\n",
                    "  \"requestedAction\": \"select-candidate-for-external-activation\",\n",
                    "  \"candidateManifestSha256\": \"{}\",\n",
                    "  \"priorManifestSha256\": \"{}\",\n",
                    "  \"activationAuthorized\": true\n",
                    "}}\n"
                ),
                "1".repeat(32),
                "a".repeat(64),
                "b".repeat(64)
            ),
            0o600,
        );
        let configuration = ActivationConfigurationV1 {
            schema: 1,
            channel: "vellum-launchd-activation".into(),
            qualification_evidence: evidence,
            approval,
            transaction_directory: transaction,
            label,
            owner_uid: owner,
            state_directory: state,
            data_socket,
            control_socket,
            stdout_log,
            stderr_log,
            candidate: ActivationVersionInputV1 {
                directory: candidate_dir,
                plist: candidate_plist.clone(),
            },
            prior: ActivationVersionInputV1 {
                directory: prior_dir,
                plist: prior_plist.clone(),
            },
            max_health_attempts: 2,
            health_interval_ms: 0,
            effect_timeout_ms: 1_000,
        };
        Self {
            _scratch: scratch,
            configuration,
            candidate_daemon,
            prior_daemon,
            candidate_plist,
            prior_plist,
        }
    }
}

struct FakePort {
    current: Option<PathBuf>,
    candidate_daemon: PathBuf,
    prior_daemon: PathBuf,
    candidate_plist: PathBuf,
    prior_plist: PathBuf,
    candidate_ready: bool,
    candidate_identity_wrong: bool,
    effects: Vec<String>,
    dispositions: VecDeque<LaunchdEffectDispositionV1>,
}

impl FakePort {
    fn new(fixture: &Fixture) -> Self {
        Self {
            current: Some(fixture.prior_daemon.clone()),
            candidate_daemon: fixture.candidate_daemon.clone(),
            prior_daemon: fixture.prior_daemon.clone(),
            candidate_plist: fixture.candidate_plist.clone(),
            prior_plist: fixture.prior_plist.clone(),
            candidate_ready: true,
            candidate_identity_wrong: false,
            effects: Vec::new(),
            dispositions: VecDeque::new(),
        }
    }

    fn disposition(&mut self) -> LaunchdEffectDispositionV1 {
        self.dispositions
            .pop_front()
            .unwrap_or(LaunchdEffectDispositionV1::Confirmed)
    }
}

impl LaunchdActivationPortV1 for FakePort {
    fn inspect_job(&mut self, _label: &str) -> LaunchdObservationV1<LaunchdJobIdentityV1> {
        self.current
            .clone()
            .map_or(LaunchdObservationV1::Unavailable, |mut program| {
                if self.candidate_identity_wrong && program == self.candidate_daemon {
                    program = program.with_file_name("transplanted-daemon");
                }
                LaunchdObservationV1::Observed(LaunchdJobIdentityV1 { pid: 42, program })
            })
    }

    fn unload_job(&mut self, _label: &str, _timeout: Duration) -> LaunchdEffectDispositionV1 {
        self.effects.push("unload".into());
        let disposition = self.disposition();
        if disposition == LaunchdEffectDispositionV1::Confirmed {
            self.current = None;
        }
        disposition
    }

    fn load_job(
        &mut self,
        _label: &str,
        plist: &Path,
        _timeout: Duration,
    ) -> LaunchdEffectDispositionV1 {
        self.effects.push(
            if plist == self.candidate_plist {
                "load-candidate"
            } else {
                "load-prior"
            }
            .into(),
        );
        let disposition = self.disposition();
        if disposition == LaunchdEffectDispositionV1::Confirmed {
            self.current = if plist == self.candidate_plist {
                Some(self.candidate_daemon.clone())
            } else if plist == self.prior_plist {
                Some(self.prior_daemon.clone())
            } else {
                None
            };
        }
        disposition
    }

    fn daemon_status(
        &mut self,
        daemon: &Path,
        _control_socket: &Path,
        _timeout: Duration,
    ) -> LaunchdObservationV1<DaemonStatusV1> {
        if self.current.as_deref() != Some(daemon) {
            return LaunchdObservationV1::Unavailable;
        }
        let ready = daemon == self.prior_daemon || self.candidate_ready;
        LaunchdObservationV1::Observed(if ready {
            DaemonStatusV1::Ready
        } else {
            DaemonStatusV1::NotReady
        })
    }

    fn wait_before_status_retry(&mut self, _interval: Duration) {}
}

#[test]
fn exact_candidate_identity_and_readiness_complete_one_durably_bracketed_activation() {
    let fixture = Fixture::new();
    let mut port = FakePort::new(&fixture);
    let receipt = activate_v1(&fixture.configuration, &mut port).unwrap();
    assert_eq!(receipt.outcome, ActivationOutcomeV1::CandidateActive);
    assert_eq!(receipt.selected_manifest_sha256, "a".repeat(64));
    assert_eq!(port.effects, ["unload", "load-candidate"]);

    let journal = fixture
        .configuration
        .transaction_directory
        .join("activation-journal");
    let entries = fs::read_dir(journal).unwrap().count();
    assert_eq!(entries, 6);
    let encoded = serde_json::to_string(&receipt).unwrap();
    assert!(!encoded.contains(fixture.configuration.state_directory.to_str().unwrap()));
}

#[test]
fn failed_candidate_health_restores_and_requalifies_the_exact_prior_version() {
    let fixture = Fixture::new();
    let mut port = FakePort::new(&fixture);
    port.candidate_ready = false;
    let receipt = activate_v1(&fixture.configuration, &mut port).unwrap();
    assert_eq!(receipt.outcome, ActivationOutcomeV1::PriorRestored);
    assert_eq!(receipt.selected_manifest_sha256, "b".repeat(64));
    assert_eq!(
        port.effects,
        ["unload", "load-candidate", "unload", "load-prior"]
    );
    assert_eq!(port.current, Some(fixture.prior_daemon));
}

#[test]
fn uncertain_launchctl_result_stops_without_retry_or_rollback() {
    let fixture = Fixture::new();
    let mut port = FakePort::new(&fixture);
    port.dispositions
        .push_back(LaunchdEffectDispositionV1::Confirmed);
    port.dispositions
        .push_back(LaunchdEffectDispositionV1::OutcomeUnknown);
    let receipt = activate_v1(&fixture.configuration, &mut port).unwrap();
    assert_eq!(receipt.outcome, ActivationOutcomeV1::OutcomeUnknown);
    assert_eq!(port.effects, ["unload", "load-candidate"]);
}

#[test]
fn offline_reconciliation_preserves_unknown_without_retry_or_launchd_effect() {
    let fixture = Fixture::new();
    let mut port = FakePort::new(&fixture);
    port.dispositions
        .push_back(LaunchdEffectDispositionV1::Confirmed);
    port.dispositions
        .push_back(LaunchdEffectDispositionV1::OutcomeUnknown);
    let receipt = activate_v1(&fixture.configuration, &mut port).unwrap();
    assert_eq!(receipt.outcome, ActivationOutcomeV1::OutcomeUnknown);
    let effects_before = port.effects.clone();

    let reconciliation = reconcile_activation_journal_v1(&fixture.configuration).unwrap();
    assert_eq!(
        reconciliation.state,
        ActivationReconciliationStateV1::OutcomeUnknown
    );
    assert_eq!(
        reconciliation.required_action,
        ActivationReconciliationActionV1::InspectExactJobAndDaemonStatus
    );
    assert_eq!(reconciliation.selected_manifest_sha256, "a".repeat(64));
    assert!(!reconciliation.automatic_retry_authorized);
    assert_eq!(reconciliation.launchd_effect, "not_attempted");
    assert_eq!(port.effects, effects_before);
}

#[test]
fn offline_reconciliation_distinguishes_pending_and_durable_terminal_prefixes() {
    let fixture = Fixture::new();
    let mut port = FakePort::new(&fixture);
    port.dispositions
        .push_back(LaunchdEffectDispositionV1::Confirmed);
    port.dispositions
        .push_back(LaunchdEffectDispositionV1::OutcomeUnknown);
    activate_v1(&fixture.configuration, &mut port).unwrap();
    let journal = fixture
        .configuration
        .transaction_directory
        .join("activation-journal");
    fs::remove_file(journal.join("0004.json")).unwrap();
    let pending = reconcile_activation_journal_v1(&fixture.configuration).unwrap();
    assert_eq!(
        pending.state,
        ActivationReconciliationStateV1::PendingEffect
    );
    assert_eq!(pending.last_phase, "candidate_load");
    assert_eq!(pending.last_disposition, "pending");

    let terminal_fixture = Fixture::new();
    let mut terminal_port = FakePort::new(&terminal_fixture);
    activate_v1(&terminal_fixture.configuration, &mut terminal_port).unwrap();
    let terminal = reconcile_activation_journal_v1(&terminal_fixture.configuration).unwrap();
    assert_eq!(
        terminal.state,
        ActivationReconciliationStateV1::CandidateActiveDurable
    );
    assert_eq!(
        terminal.required_action,
        ActivationReconciliationActionV1::None
    );
    let configuration_file = terminal_fixture._scratch.0.join("reconcile-config.json");
    write(
        &configuration_file,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&terminal_fixture.configuration).unwrap()
        ),
        0o600,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_vellum-launchd-activate"))
        .arg("--reconcile-journal")
        .arg(format!("--config={}", configuration_file.display()))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let cli_report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(cli_report["state"], "candidate_active_durable");
    assert_eq!(cli_report["launchdEffect"], "not_attempted");
}

#[test]
fn offline_reconciliation_rejects_transplanted_or_noncanonical_journal_evidence() {
    let fixture = Fixture::new();
    let mut port = FakePort::new(&fixture);
    activate_v1(&fixture.configuration, &mut port).unwrap();
    let entry = fixture
        .configuration
        .transaction_directory
        .join("activation-journal/0005.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&entry).unwrap()).unwrap();
    value["candidateManifestSha256"] = serde_json::Value::String("9".repeat(64));
    fs::set_permissions(&entry, fs::Permissions::from_mode(0o600)).unwrap();
    write(
        &entry,
        format!("{}\n", serde_json::to_string_pretty(&value).unwrap()),
        0o400,
    );
    assert!(reconcile_activation_journal_v1(&fixture.configuration).is_err());
}

#[test]
fn transplanted_candidate_process_identity_triggers_exact_prior_rollback() {
    let fixture = Fixture::new();
    let mut port = FakePort::new(&fixture);
    port.candidate_identity_wrong = true;
    let receipt = activate_v1(&fixture.configuration, &mut port).unwrap();
    assert_eq!(receipt.outcome, ActivationOutcomeV1::PriorRestored);
    assert_eq!(
        port.effects,
        ["unload", "load-candidate", "unload", "load-prior"]
    );
}

#[test]
fn evidence_or_approval_substitution_refuses_before_any_launchd_effect() {
    let mut fixture = Fixture::new();
    fixture.configuration.owner_uid = fixture.configuration.owner_uid.saturating_add(1);
    let mut port = FakePort::new(&fixture);
    assert!(activate_v1(&fixture.configuration, &mut port).is_err());
    assert!(port.effects.is_empty());
    assert!(
        fs::read_dir(&fixture.configuration.transaction_directory)
            .unwrap()
            .next()
            .is_none()
    );
}

#[test]
fn decoder_is_closed_and_cli_requires_the_explicit_execution_switch() {
    let fixture = Fixture::new();
    let encoded = serde_json::to_vec(&fixture.configuration).unwrap();
    assert_eq!(
        decode_configuration_v1(&encoded).unwrap(),
        fixture.configuration
    );
    let mut value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("ambientAuthority".into(), true.into());
    assert!(decode_configuration_v1(&serde_json::to_vec(&value).unwrap()).is_err());

    let help = Command::new(env!("CARGO_BIN_EXE_vellum-launchd-activate"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains("vellum-launchd-activate")
    );
    let refused = Command::new(env!("CARGO_BIN_EXE_vellum-launchd-activate"))
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(
        String::from_utf8(refused.stderr)
            .unwrap()
            .contains("usage:")
    );
}
