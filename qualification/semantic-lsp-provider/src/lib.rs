//! Pinned-provider qualification preflight.
//!
//! This excluded package validates only package-owned fixtures and an exact
//! provider artifact. A preflight receipt is deliberately not evidence that a
//! provider completed Vellum's semantic qualification scenario.
#![forbid(unsafe_code)]

mod scenario;
mod strict_json;

pub use scenario::{LiveQualificationReceiptV1, LiveQualificationRefusalV1, qualify_live};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File, Metadata};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

const PROFILE_LIMIT_BYTES: u64 = 65_536;
const MANIFEST_LIMIT_BYTES: u64 = 65_536;
const MAX_FIXTURE_FILES: usize = 32;
const MAX_FIXTURE_BYTES: u64 = 1_048_576;
const MAX_ARTIFACT_BYTES: u64 = 536_870_912;

#[derive(Clone, Copy, Default, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct DigestV1(#[serde(serialize_with = "serialize_digest")] [u8; 32]);

impl DigestV1 {
    pub fn from_hex(value: &str) -> Option<Self> {
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let mut bytes = [0_u8; 32];
        for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
            let high = hex_digit(pair[0])?;
            let low = hex_digit(pair[1])?;
            bytes[index] = (high << 4) | low;
        }
        Some(Self(bytes))
    }

    pub fn to_hex(self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut output = String::with_capacity(64);
        for byte in self.0 {
            output.push(char::from(HEX[usize::from(byte >> 4)]));
            output.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        output
    }
}

impl fmt::Debug for DigestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DigestV1(..)")
    }
}

fn serialize_digest<S>(digest: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&DigestV1(*digest).to_hex())
}

const fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderProfileV1 {
    qualification_profile_json_sha256: DigestV1,
    document: ProviderProfileDocumentV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct ProviderProfileDocumentV1 {
    schema_version: u32,
    profile_id: String,
    fixture: FixtureExpectationV1,
    qualification: QualificationExpectationV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct FixtureExpectationV1 {
    manifest: String,
    sha256: String,
    file_count: usize,
    total_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum QualificationExpectationV1 {
    Pending {
        reason: PendingReasonV1,
    },
    Pinned {
        platform: PlatformExpectationV1,
        resource_policy: ResourcePolicyExpectationV1,
        artifact: ArtifactExpectationV1,
        provider: Box<ProviderExpectationV1>,
        ceilings: TimeCeilingsV1,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ResourcePolicyExpectationV1 {
    Unbounded,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum PendingReasonV1 {
    ArtifactNotPinned,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct PlatformExpectationV1 {
    os: String,
    arch: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct ArtifactExpectationV1 {
    source_url: String,
    archive_sha256: String,
    compression: ArtifactCompressionV1,
    sha256: String,
    bytes: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ArtifactCompressionV1 {
    Gzip,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct ProviderExpectationV1 {
    server_name: String,
    server_version: String,
    argv: Vec<String>,
    environment: BTreeMap<String, String>,
    readiness: ReadinessExpectationV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct ReadinessExpectationV1 {
    method: String,
    schema_version: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct TimeCeilingsV1 {
    launch_ms: u64,
    readiness_ms: u64,
    operation_ms: u64,
    graceful_stop_ms: u64,
    forced_stop_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct FixtureManifestV1 {
    schema_version: u32,
    fixture_id: String,
    files: Vec<FixtureFileV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct FixtureFileV1 {
    path: String,
    sha256: String,
    bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreflightRefusalV1 {
    ProfileIo,
    ProfilePath,
    ProfileLimit,
    ProfileProtocol,
    ProfileInvalid,
    FixtureIo,
    FixturePath,
    FixtureLimit,
    FixtureProtocol,
    FixtureMismatch,
    QualificationPending,
    PlatformMismatch,
    ArtifactPath,
    ArtifactIo,
    ArtifactMismatch,
}

impl fmt::Display for PreflightRefusalV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ProfileIo => "profile unavailable",
            Self::ProfilePath => "profile path is not a stable regular file",
            Self::ProfileLimit => "profile exceeds its bound",
            Self::ProfileProtocol => "profile is not strict qualification JSON",
            Self::ProfileInvalid => "profile violates the qualification contract",
            Self::FixtureIo => "fixture unavailable",
            Self::FixturePath => "fixture path is not package-owned and regular",
            Self::FixtureLimit => "fixture exceeds its bound",
            Self::FixtureProtocol => "fixture manifest is not strict qualification JSON",
            Self::FixtureMismatch => "fixture identity mismatch",
            Self::QualificationPending => "provider artifact has not been pinned",
            Self::PlatformMismatch => "provider platform mismatch",
            Self::ArtifactPath => "provider executable path is not absolute and canonical",
            Self::ArtifactIo => "provider executable unavailable",
            Self::ArtifactMismatch => "provider artifact identity mismatch",
        })
    }
}

impl std::error::Error for PreflightRefusalV1 {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflightStatusV1 {
    ExactArtifact,
}

/// Content-safe artifact and fixture evidence. This is not a semantic-provider
/// qualification receipt.
#[derive(Clone, Eq, PartialEq, Serialize)]
pub struct ProviderPreflightReceiptV1 {
    schema_version: u32,
    profile_id: String,
    qualification_profile_json_sha256: DigestV1,
    status: PreflightStatusV1,
    artifact_sha256: DigestV1,
    artifact_bytes: u64,
    fixture_manifest_sha256: DigestV1,
    fixture_files: usize,
    fixture_bytes: u64,
}

impl fmt::Debug for ProviderPreflightReceiptV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderPreflightReceiptV1")
            .field("schema_version", &self.schema_version)
            .field("profile_id", &self.profile_id)
            .field(
                "qualification_profile_json_sha256",
                &self.qualification_profile_json_sha256,
            )
            .field("status", &self.status)
            .field("artifact_sha256", &self.artifact_sha256)
            .field("artifact_bytes", &self.artifact_bytes)
            .field("fixture_manifest_sha256", &self.fixture_manifest_sha256)
            .field("fixture_files", &self.fixture_files)
            .field("fixture_bytes", &self.fixture_bytes)
            .finish()
    }
}

pub fn load_profile(path: &Path) -> Result<ProviderProfileV1, PreflightRefusalV1> {
    let bytes =
        read_regular_bounded(path, PROFILE_LIMIT_BYTES, None).map_err(|error| match error {
            BoundedReadError::Io => PreflightRefusalV1::ProfileIo,
            BoundedReadError::Limit => PreflightRefusalV1::ProfileLimit,
            BoundedReadError::Substitution => PreflightRefusalV1::ProfilePath,
        })?;
    let document: ProviderProfileDocumentV1 =
        strict_json::from_slice(&bytes).map_err(|_| PreflightRefusalV1::ProfileProtocol)?;
    let profile = ProviderProfileV1 {
        qualification_profile_json_sha256: digest(&bytes),
        document,
    };
    validate_profile(&profile)?;
    Ok(profile)
}

pub fn preflight(
    profile: &ProviderProfileV1,
    executable: &Path,
) -> Result<ProviderPreflightReceiptV1, PreflightRefusalV1> {
    validate_profile(profile)?;
    let fixture = preflight_fixture(&profile.document.fixture)?;
    let QualificationExpectationV1::Pinned {
        platform, artifact, ..
    } = &profile.document.qualification
    else {
        return Err(PreflightRefusalV1::QualificationPending);
    };
    if platform.os != std::env::consts::OS || platform.arch != std::env::consts::ARCH {
        return Err(PreflightRefusalV1::PlatformMismatch);
    }
    if !executable.is_absolute() {
        return Err(PreflightRefusalV1::ArtifactPath);
    }
    let canonical = fs::canonicalize(executable).map_err(|_| PreflightRefusalV1::ArtifactIo)?;
    if canonical != executable {
        return Err(PreflightRefusalV1::ArtifactPath);
    }
    let bytes =
        read_regular_bounded(executable, artifact.bytes, None).map_err(|error| match error {
            BoundedReadError::Io => PreflightRefusalV1::ArtifactIo,
            BoundedReadError::Limit => PreflightRefusalV1::ArtifactMismatch,
            BoundedReadError::Substitution => PreflightRefusalV1::ArtifactPath,
        })?;
    let digest = digest(&bytes);
    let expected =
        DigestV1::from_hex(&artifact.sha256).ok_or(PreflightRefusalV1::ProfileInvalid)?;
    if u64::try_from(bytes.len()).ok() != Some(artifact.bytes) || digest != expected {
        return Err(PreflightRefusalV1::ArtifactMismatch);
    }
    Ok(ProviderPreflightReceiptV1 {
        schema_version: 1,
        profile_id: profile.document.profile_id.clone(),
        qualification_profile_json_sha256: profile.qualification_profile_json_sha256,
        status: PreflightStatusV1::ExactArtifact,
        artifact_sha256: digest,
        artifact_bytes: artifact.bytes,
        fixture_manifest_sha256: fixture.manifest_digest,
        fixture_files: fixture.files,
        fixture_bytes: fixture.bytes,
    })
}

pub fn receipt_json(receipt: &ProviderPreflightReceiptV1) -> Result<String, PreflightRefusalV1> {
    serde_json::to_string(receipt).map_err(|_| PreflightRefusalV1::ProfileProtocol)
}

struct FixtureEvidenceV1 {
    manifest_digest: DigestV1,
    files: usize,
    bytes: u64,
}

fn preflight_fixture(
    expected: &FixtureExpectationV1,
) -> Result<FixtureEvidenceV1, PreflightRefusalV1> {
    let relative = safe_relative_path(&expected.manifest)?;
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let manifest_bytes = read_package_file(&fixture_root, &relative, MANIFEST_LIMIT_BYTES)
        .map_err(|error| match error {
            BoundedReadError::Io => PreflightRefusalV1::FixtureIo,
            BoundedReadError::Limit => PreflightRefusalV1::FixtureLimit,
            BoundedReadError::Substitution => PreflightRefusalV1::FixturePath,
        })?;
    let manifest_digest = digest(&manifest_bytes);
    if DigestV1::from_hex(&expected.sha256) != Some(manifest_digest) {
        return Err(PreflightRefusalV1::FixtureMismatch);
    }
    let manifest: FixtureManifestV1 = strict_json::from_slice(&manifest_bytes)
        .map_err(|_| PreflightRefusalV1::FixtureProtocol)?;
    if manifest.schema_version != 1
        || !valid_token(&manifest.fixture_id, 64)
        || manifest.files.is_empty()
        || manifest.files.len() > MAX_FIXTURE_FILES
        || manifest.files.len() != expected.file_count
    {
        return Err(PreflightRefusalV1::FixtureProtocol);
    }
    let base = relative
        .parent()
        .ok_or(PreflightRefusalV1::FixtureProtocol)?;
    let mut seen = BTreeSet::new();
    let mut total = 0_u64;
    for file in &manifest.files {
        let relative = safe_relative_path(&file.path)?;
        if !seen.insert(relative.clone()) || file.bytes > MAX_FIXTURE_BYTES {
            return Err(PreflightRefusalV1::FixtureProtocol);
        }
        let expected_digest =
            DigestV1::from_hex(&file.sha256).ok_or(PreflightRefusalV1::FixtureProtocol)?;
        let fixture_relative = base.join(relative);
        let bytes =
            read_package_file(&fixture_root, &fixture_relative, file.bytes).map_err(|error| {
                match error {
                    BoundedReadError::Io => PreflightRefusalV1::FixtureIo,
                    BoundedReadError::Limit => PreflightRefusalV1::FixtureMismatch,
                    BoundedReadError::Substitution => PreflightRefusalV1::FixturePath,
                }
            })?;
        if u64::try_from(bytes.len()).ok() != Some(file.bytes) || digest(&bytes) != expected_digest
        {
            return Err(PreflightRefusalV1::FixtureMismatch);
        }
        total = total
            .checked_add(file.bytes)
            .ok_or(PreflightRefusalV1::FixtureLimit)?;
        if total > MAX_FIXTURE_BYTES {
            return Err(PreflightRefusalV1::FixtureLimit);
        }
    }
    if total != expected.total_bytes {
        return Err(PreflightRefusalV1::FixtureMismatch);
    }
    Ok(FixtureEvidenceV1 {
        manifest_digest,
        files: manifest.files.len(),
        bytes: total,
    })
}

fn validate_profile(profile: &ProviderProfileV1) -> Result<(), PreflightRefusalV1> {
    if profile.document.schema_version != 1
        || !valid_token(&profile.document.profile_id, 64)
        || safe_relative_path(&profile.document.fixture.manifest).is_err()
        || DigestV1::from_hex(&profile.document.fixture.sha256).is_none()
        || profile.document.fixture.file_count == 0
        || profile.document.fixture.file_count > MAX_FIXTURE_FILES
        || profile.document.fixture.total_bytes > MAX_FIXTURE_BYTES
    {
        return Err(PreflightRefusalV1::ProfileInvalid);
    }
    if let QualificationExpectationV1::Pinned {
        platform,
        resource_policy,
        artifact,
        provider,
        ceilings,
    } = &profile.document.qualification
    {
        let compression_valid = matches!(artifact.compression, ArtifactCompressionV1::Gzip);
        let environment_valid = provider.environment.len() <= 16
            && provider.environment.iter().all(|(key, value)| {
                valid_environment_key(key) && !value.is_empty() && value.len() <= 1024
            });
        if !matches!(resource_policy, ResourcePolicyExpectationV1::Unbounded)
            || !valid_token(&platform.os, 32)
            || !valid_token(&platform.arch, 32)
            || !artifact
                .source_url
                .starts_with("https://github.com/rust-lang/rust-analyzer/releases/download/")
            || artifact.source_url.len() > 512
            || DigestV1::from_hex(&artifact.archive_sha256).is_none()
            || !compression_valid
            || artifact.bytes == 0
            || artifact.bytes > MAX_ARTIFACT_BYTES
            || DigestV1::from_hex(&artifact.sha256).is_none()
            || !valid_bounded_text(&provider.server_name, 128)
            || !valid_bounded_text(&provider.server_version, 128)
            || provider.argv.len() > 32
            || provider
                .argv
                .iter()
                .any(|argument| argument.is_empty() || argument.len() > 1024)
            || !environment_valid
            || !valid_bounded_text(&provider.readiness.method, 128)
            || provider.readiness.schema_version == 0
            || [
                ceilings.launch_ms,
                ceilings.readiness_ms,
                ceilings.operation_ms,
                ceilings.graceful_stop_ms,
                ceilings.forced_stop_ms,
            ]
            .contains(&0)
        {
            return Err(PreflightRefusalV1::ProfileInvalid);
        }
    }
    Ok(())
}

fn valid_token(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn valid_bounded_text(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}

fn valid_environment_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

fn safe_relative_path(value: &str) -> Result<PathBuf, PreflightRefusalV1> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(PreflightRefusalV1::ProfileInvalid);
    }
    Ok(path.to_owned())
}

fn digest(bytes: &[u8]) -> DigestV1 {
    DigestV1(Sha256::digest(bytes).into())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BoundedReadError {
    Io,
    Limit,
    Substitution,
}

fn read_regular_bounded(
    path: &Path,
    max_bytes: u64,
    required_root: Option<&Path>,
) -> Result<Vec<u8>, BoundedReadError> {
    let before = fs::symlink_metadata(path).map_err(|_| BoundedReadError::Io)?;
    if before.file_type().is_symlink() || !before.is_file() {
        return Err(BoundedReadError::Substitution);
    }
    if before.len() > max_bytes {
        return Err(BoundedReadError::Limit);
    }
    let canonical_before = fs::canonicalize(path).map_err(|_| BoundedReadError::Io)?;
    if required_root.is_some_and(|root| !canonical_before.starts_with(root)) {
        return Err(BoundedReadError::Substitution);
    }
    let mut file = File::open(path).map_err(|_| BoundedReadError::Io)?;
    let opened = file.metadata().map_err(|_| BoundedReadError::Io)?;
    if !opened.is_file() || !same_file(&before, &opened) {
        return Err(BoundedReadError::Substitution);
    }
    let capacity = usize::try_from(opened.len().min(max_bytes)).unwrap_or(usize::MAX);
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| BoundedReadError::Limit)?;
    file.by_ref()
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| BoundedReadError::Io)?;
    if u64::try_from(bytes.len()).map_err(|_| BoundedReadError::Limit)? > max_bytes {
        return Err(BoundedReadError::Limit);
    }
    let opened_after = file.metadata().map_err(|_| BoundedReadError::Io)?;
    let after = fs::symlink_metadata(path).map_err(|_| BoundedReadError::Substitution)?;
    let canonical_after = fs::canonicalize(path).map_err(|_| BoundedReadError::Substitution)?;
    if after.file_type().is_symlink()
        || !after.is_file()
        || canonical_before != canonical_after
        || !same_file(&before, &opened_after)
        || !same_file(&opened_after, &after)
        || u64::try_from(bytes.len()).ok() != Some(opened_after.len())
        || required_root.is_some_and(|root| !canonical_after.starts_with(root))
    {
        return Err(BoundedReadError::Substitution);
    }
    Ok(bytes)
}

fn read_package_file(
    root: &Path,
    relative: &Path,
    max_bytes: u64,
) -> Result<Vec<u8>, BoundedReadError> {
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(BoundedReadError::Substitution);
    }
    let root_metadata = fs::symlink_metadata(root).map_err(|_| BoundedReadError::Io)?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(BoundedReadError::Substitution);
    }
    let canonical_root = fs::canonicalize(root).map_err(|_| BoundedReadError::Io)?;
    let components: Vec<_> = relative.components().collect();
    let mut candidate = root.to_owned();
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(component) = component else {
            return Err(BoundedReadError::Substitution);
        };
        candidate.push(component);
        let metadata = fs::symlink_metadata(&candidate).map_err(|_| BoundedReadError::Io)?;
        if metadata.file_type().is_symlink()
            || (index + 1 == components.len() && !metadata.is_file())
            || (index + 1 != components.len() && !metadata.is_dir())
        {
            return Err(BoundedReadError::Substitution);
        }
    }
    read_regular_bounded(&candidate, max_bytes, Some(&canonical_root))
}

fn same_file(left: &Metadata, right: &Metadata) -> bool {
    left.is_file()
        && right.is_file()
        && left.len() == right.len()
        && left.modified().ok() == right.modified().ok()
        && platform_file_identity_matches(left, right)
}

#[cfg(unix)]
fn platform_file_identity_matches(left: &Metadata, right: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.dev() == right.dev()
        && left.ino() == right.ino()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
}

#[cfg(not(unix))]
fn platform_file_identity_matches(_left: &Metadata, _right: &Metadata) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "vellum-semantic-provider-qualification-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(name);
            fs::write(&path, bytes).unwrap();
            path
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn pending_profile() -> ProviderProfileV1 {
        load_profile(&Path::new(env!("CARGO_MANIFEST_DIR")).join("profiles/rust-analyzer-v1.json"))
            .unwrap()
    }

    fn pin(profile: &mut ProviderProfileV1, bytes: &[u8]) {
        profile.document.qualification = QualificationExpectationV1::Pinned {
            platform: PlatformExpectationV1 {
                os: std::env::consts::OS.to_owned(),
                arch: std::env::consts::ARCH.to_owned(),
            },
            resource_policy: ResourcePolicyExpectationV1::Unbounded,
            artifact: ArtifactExpectationV1 {
                source_url:
                    "https://github.com/rust-lang/rust-analyzer/releases/download/test/provider.gz"
                        .to_owned(),
                archive_sha256: digest(b"archive").to_hex(),
                compression: ArtifactCompressionV1::Gzip,
                sha256: digest(bytes).to_hex(),
                bytes: u64::try_from(bytes.len()).unwrap(),
            },
            provider: Box::new(ProviderExpectationV1 {
                server_name: "pinned-provider".to_owned(),
                server_version: "1.0.0".to_owned(),
                argv: vec!["--stdio".to_owned()],
                environment: BTreeMap::new(),
                readiness: ReadinessExpectationV1 {
                    method: "provider/status".to_owned(),
                    schema_version: 1,
                },
            }),
            ceilings: TimeCeilingsV1 {
                launch_ms: 1_000,
                readiness_ms: 5_000,
                operation_ms: 1_000,
                graceful_stop_ms: 1_000,
                forced_stop_ms: 1_000,
            },
        };
    }

    #[test]
    fn checked_in_profile_and_fixture_are_valid_and_require_the_exact_artifact() {
        let mut profile = pending_profile();
        assert_eq!(
            preflight(&profile, Path::new("/provider")).unwrap_err(),
            PreflightRefusalV1::ArtifactIo
        );
        profile.document.fixture.sha256 = "00".repeat(32);
        assert_eq!(
            preflight(&profile, Path::new("/provider")).unwrap_err(),
            PreflightRefusalV1::FixtureMismatch
        );
    }

    #[test]
    fn exact_canonical_artifact_produces_only_a_content_safe_preflight_receipt() {
        let scratch = Scratch::new();
        let artifact = scratch.write("provider", b"deterministic provider artifact");
        let canonical = fs::canonicalize(artifact).unwrap();
        let mut profile = pending_profile();
        pin(&mut profile, b"deterministic provider artifact");
        let receipt = preflight(&profile, &canonical).unwrap();
        let json = receipt_json(&receipt).unwrap();
        assert!(json.contains("exact_artifact"));
        assert!(!json.contains("deterministic provider artifact"));
        assert!(!json.contains(canonical.to_str().unwrap()));
        assert!(!format!("{receipt:?}").contains(&receipt.artifact_sha256.to_hex()));
    }

    #[test]
    fn artifact_path_platform_size_and_digest_mismatches_refuse() {
        let scratch = Scratch::new();
        let artifact = scratch.write("provider", b"exact");
        let canonical = fs::canonicalize(&artifact).unwrap();
        let mut profile = pending_profile();
        pin(&mut profile, b"exact");
        assert_eq!(
            preflight(&profile, Path::new("provider")).unwrap_err(),
            PreflightRefusalV1::ArtifactPath
        );
        if let QualificationExpectationV1::Pinned { platform, .. } =
            &mut profile.document.qualification
        {
            platform.os = "definitely-not-this-os".to_owned();
        }
        assert_eq!(
            preflight(&profile, &canonical).unwrap_err(),
            PreflightRefusalV1::PlatformMismatch
        );
        pin(&mut profile, b"other");
        assert_eq!(
            preflight(&profile, &canonical).unwrap_err(),
            PreflightRefusalV1::ArtifactMismatch
        );
    }

    #[test]
    fn unknown_duplicate_and_trailing_profile_json_refuse() {
        let scratch = Scratch::new();
        let valid =
            fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("profiles/rust-analyzer-v1.json"))
                .unwrap();
        for bytes in [
            br#"{"schema_version":1,"schema_version":1,"profile_id":"x","fixture":{},"qualification":{}}"#.as_slice(),
            br#"{"schema_version":1,"profile_id":"x","fixture":{},"qualification":{},"unknown":true}"#.as_slice(),
            [valid.as_slice(), b" true"].concat().as_slice(),
        ] {
            let path = scratch.write("profile.json", bytes);
            assert!(matches!(
                load_profile(&path),
                Err(PreflightRefusalV1::ProfileProtocol)
            ));
        }
    }

    #[test]
    fn pinned_profile_requires_an_explicit_closed_resource_policy() {
        let scratch = Scratch::new();
        let valid = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("profiles/rust-analyzer-v1.json"),
        )
        .unwrap();
        let missing = valid.replace(
            "    \"resource_policy\": {\n      \"kind\": \"unbounded\"\n    },\n",
            "",
        );
        assert_ne!(missing, valid);
        let unknown = valid.replace("\"kind\": \"unbounded\"", "\"kind\": \"ambient\"");
        for bytes in [missing.as_bytes(), unknown.as_bytes()] {
            let path = scratch.write("resource-policy.json", bytes);
            assert_eq!(
                load_profile(&path).unwrap_err(),
                PreflightRefusalV1::ProfileProtocol
            );
        }
    }

    #[test]
    fn profile_bounds_and_closed_pending_shape_refuse() {
        let scratch = Scratch::new();
        let oversized = scratch.write("oversized.json", &vec![b' '; 65_537]);
        assert_eq!(
            load_profile(&oversized).unwrap_err(),
            PreflightRefusalV1::ProfileLimit
        );
        let extra = scratch.write(
            "extra.json",
            br#"{"schema_version":1,"profile_id":"x","fixture":{"manifest":"x","sha256":"0000000000000000000000000000000000000000000000000000000000000000","file_count":1,"total_bytes":1},"qualification":{"kind":"pending","reason":"artifact_not_pinned","artifact":{}}}"#,
        );
        assert_eq!(
            load_profile(&extra).unwrap_err(),
            PreflightRefusalV1::ProfileProtocol
        );
        assert_eq!(
            load_profile(&scratch.0).unwrap_err(),
            PreflightRefusalV1::ProfilePath
        );
    }

    #[test]
    fn fixture_paths_cannot_traverse_or_name_non_regular_files() {
        let scratch = Scratch::new();
        let root = scratch.0.join("fixtures");
        fs::create_dir(&root).unwrap();
        let outside = scratch.write("outside", b"outside");
        assert_eq!(
            read_package_file(&root, Path::new("../outside"), 16).unwrap_err(),
            BoundedReadError::Substitution
        );
        fs::create_dir(root.join("directory")).unwrap();
        assert_eq!(
            read_package_file(&root, Path::new("directory"), 16).unwrap_err(),
            BoundedReadError::Substitution
        );
        let mut profile = pending_profile();
        profile.document.fixture.manifest = "../outside".to_owned();
        assert_eq!(
            preflight(&profile, &outside).unwrap_err(),
            PreflightRefusalV1::ProfileInvalid
        );
    }

    #[cfg(unix)]
    #[test]
    fn profile_and_fixture_symlinks_are_refused() {
        use std::os::unix::fs::symlink;

        let scratch = Scratch::new();
        let profile_target =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("profiles/rust-analyzer-v1.json");
        let profile_link = scratch.0.join("profile-link.json");
        symlink(profile_target, &profile_link).unwrap();
        assert_eq!(
            load_profile(&profile_link).unwrap_err(),
            PreflightRefusalV1::ProfilePath
        );

        let root = scratch.0.join("fixtures");
        fs::create_dir(&root).unwrap();
        let outside = scratch.write("outside-fixture", b"outside");
        symlink(&outside, root.join("file-link")).unwrap();
        assert_eq!(
            read_package_file(&root, Path::new("file-link"), 16).unwrap_err(),
            BoundedReadError::Substitution
        );

        let outside_directory = scratch.0.join("outside-directory");
        fs::create_dir(&outside_directory).unwrap();
        fs::write(outside_directory.join("fixture"), b"outside").unwrap();
        symlink(&outside_directory, root.join("directory-link")).unwrap();
        assert_eq!(
            read_package_file(&root, Path::new("directory-link/fixture"), 16).unwrap_err(),
            BoundedReadError::Substitution
        );

        let linked_root = scratch.0.join("linked-root");
        symlink(&root, &linked_root).unwrap();
        assert_eq!(
            read_package_file(&linked_root, Path::new("anything"), 16).unwrap_err(),
            BoundedReadError::Substitution
        );
    }
}
