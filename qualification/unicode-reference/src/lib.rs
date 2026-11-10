//! Explicit external reference provider, never an engine publication dependency.
mod process;
mod protocol;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use vellum_engine::coordinates::BytePosition;
use vellum_engine::coordinator::BoundaryTableV1;
use vellum_engine::instrumentation::{Boundary, Disposition, Observation, Recorder};
pub const ICU_JAR_SHA256: &str = "28b292e60d0d00c496bae88a15b2615f89f781be13670aeac9dc90f189570e07";
pub const TOOL_SHA256: &str = "72b157ed3b33259d44169e934bd5e996444e3a6ef0245db5f7db86a085f00fbb";
const TOOL: &[u8] = include_bytes!("../../../formal/UnicodeBoundary.java");
const MAX_JAR_BYTES: usize = 64 * 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceError {
    ArtifactUnavailable,
    PinMismatch,
    InputLimit,
    OutputLimit,
    ProcessFailed,
    Deadline,
    Protocol,
    Capacity,
    Io,
}
pub struct ReferenceRow {
    pub table: BoundaryTableV1,
    /// Supplementary diagnostic grid only. Never substituted for default words.
    pub dictionary: Vec<BytePosition>,
}
impl std::fmt::Debug for ReferenceRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReferenceRow(<redacted>)")
    }
}
pub struct ReferenceProvider {
    java: PathBuf,
    jar: Vec<u8>,
}
impl std::fmt::Debug for ReferenceProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReferenceProvider(<redacted>)")
    }
}
fn capture<T>(
    r: &mut Recorder,
    boundary: Boundary,
    start: Instant,
    result: &Result<T, ReferenceError>,
) {
    r.record(Observation {
        boundary,
        disposition: if result.is_ok() {
            Disposition::Completed
        } else {
            Disposition::Refused
        },
        service_time: Some(start.elapsed()),
    });
}
impl ReferenceProvider {
    /// Read/hash a bounded jar snapshot. Java executable/OS are trusted host
    /// prerequisites. This selects a reference tool, not a production dependency.
    pub fn qualify(java: &Path, jar: &Path, r: &mut Recorder) -> Result<Self, ReferenceError> {
        let start = Instant::now();
        let result = (|| {
            let file = File::open(jar).map_err(|_| ReferenceError::ArtifactUnavailable)?;
            if !file
                .metadata()
                .map_err(|_| ReferenceError::ArtifactUnavailable)?
                .is_file()
            {
                return Err(ReferenceError::ArtifactUnavailable);
            }
            let mut bytes = Vec::new();
            file.take((MAX_JAR_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(|_| ReferenceError::Io)?;
            if bytes.len() > MAX_JAR_BYTES
                || format!("{:x}", Sha256::digest(&bytes)) != ICU_JAR_SHA256
                || format!("{:x}", Sha256::digest(TOOL)) != TOOL_SHA256
            {
                return Err(ReferenceError::PinMismatch);
            }
            Ok(Self {
                java: java.to_path_buf(),
                jar: bytes,
            })
        })();
        capture(r, Boundary::UnicodeProviderQualification, start, &result);
        result
    }
    /// Snapshot staging and process execution are entirely outside engine commit.
    /// Fixed reference limits; no streaming/incremental/provider fairness claim.
    pub fn run_batch(
        &self,
        texts: &[&str],
        r: &mut Recorder,
    ) -> Result<Vec<ReferenceRow>, ReferenceError> {
        let start = Instant::now();
        let input = protocol::encode(texts);
        capture(r, Boundary::UnicodeProviderInput, start, &input);
        let input = input?;
        let start = Instant::now();
        let output = (|| {
            let directory = Staging::create()?;
            let jar = directory.write("icu4j.jar", &self.jar)?;
            let source = directory.write("UnicodeBoundary.java", TOOL)?;
            let mut command = Command::new(&self.java);
            command.arg("--class-path").arg(jar).arg(source);
            process::execute(command, input, Duration::from_secs(30), MAX_OUTPUT_BYTES)
        })();
        capture(r, Boundary::UnicodeProviderExecution, start, &output);
        let output = output?;
        let start = Instant::now();
        let result = protocol::decode(texts, &output);
        capture(r, Boundary::UnicodeProviderDecode, start, &result);
        result
    }
}
static SERIAL: AtomicU64 = AtomicU64::new(0);
struct Staging(PathBuf);
impl Staging {
    fn create() -> Result<Self, ReferenceError> {
        for _ in 0..8 {
            let ticks = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| ReferenceError::Io)?
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "vellum-unicode-ref-{}-{ticks}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(ReferenceError::Io),
            }
        }
        Err(ReferenceError::Io)
    }
    fn write(&self, name: &str, bytes: &[u8]) -> Result<PathBuf, ReferenceError> {
        let path = self.0.join(name);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options
            .open(&path)
            .and_then(|mut f| f.write_all(bytes))
            .map_err(|_| ReferenceError::Io)?;
        Ok(path)
    }
}
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compiled_reference_source_matches_its_pin() {
        assert_eq!(format!("{:x}", Sha256::digest(TOOL)), TOOL_SHA256);
    }
    #[test]
    fn wrong_artifact_refuses_before_any_java_execution_and_debug_is_redacted() {
        let dir = Staging::create().unwrap();
        let jar = dir.write("wrong.jar", b"not a jar").unwrap();
        assert_eq!(
            ReferenceProvider::qualify(Path::new("must-not-run"), &jar, &mut Recorder::disabled())
                .err(),
            Some(ReferenceError::PinMismatch)
        );
        let provider = ReferenceProvider {
            java: PathBuf::from("PRIVATE_PATH"),
            jar: vec![],
        };
        assert!(!format!("{provider:?}").contains("PRIVATE_PATH"));
    }
}
