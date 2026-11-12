//! Bounded, OS-neutral product shared with a semantic isolation helper.
//!
//! The protocol transfers descriptions of immutable workspace snapshots, not
//! ambient filesystem authority. Platform compositions remain responsible for
//! materializing the committed snapshot in a private container and for proving
//! that the helper actually enforces this crate's fixed isolation requirements.
#![forbid(unsafe_code)]

use sha2::{Digest as _, Sha256};
use std::fmt;
use std::sync::Arc;

pub const MAX_HELPER_SNAPSHOT_FILES_V1: usize = 4_096;
pub const MAX_HELPER_SNAPSHOT_BYTES_V1: u64 = 128 * 1024 * 1024;
pub const MAX_HELPER_RELATIVE_PATH_BYTES_V1: usize = 4_096;
pub const MAX_HELPER_PROCESS_MEMORY_BYTES_V1: u64 = 4 * 1024 * 1024 * 1024;
pub const MAX_HELPER_PROCESSES_V1: u16 = 64;
pub const MAX_HELPER_CPU_MILLIS_V1: u64 = 60 * 60 * 1_000;

macro_rules! opaque_ref {
    ($name:ident, $size:literal) => {
        #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; $size]);

        impl $name {
            pub fn new(bytes: [u8; $size]) -> Option<Self> {
                (bytes != [0; $size]).then_some(Self(bytes))
            }

            /// Canonical correlation bytes. Possession conveys no authority.
            pub const fn as_bytes(self) -> [u8; $size] {
                self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!(stringify!($name), "(<opaque>)"))
            }
        }
    };
}

opaque_ref!(SemanticHelperExchangeRefV1, 16);
opaque_ref!(SemanticHelperSessionRefV1, 16);
opaque_ref!(SemanticHelperSnapshotDigestV1, 32);
opaque_ref!(SemanticHelperProviderQualificationDigestV1, 32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticHelperSnapshotErrorV1 {
    EmptyPath,
    PathTooLong,
    AbsolutePath,
    NonCanonicalPath,
    DuplicatePath,
    EmptySnapshot,
    TooManyFiles,
    SnapshotTooLarge,
    InvalidDigest,
}

/// Canonical UTF-8 path relative to a private immutable snapshot root.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticHelperRelativePathV1(Arc<str>);

impl SemanticHelperRelativePathV1 {
    pub fn new(path: impl AsRef<str>) -> Result<Self, SemanticHelperSnapshotErrorV1> {
        let path = path.as_ref();
        if path.is_empty() {
            return Err(SemanticHelperSnapshotErrorV1::EmptyPath);
        }
        if path.len() > MAX_HELPER_RELATIVE_PATH_BYTES_V1 {
            return Err(SemanticHelperSnapshotErrorV1::PathTooLong);
        }
        if path.starts_with('/') {
            return Err(SemanticHelperSnapshotErrorV1::AbsolutePath);
        }
        if path.contains('\0')
            || path.contains('\\')
            || path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(SemanticHelperSnapshotErrorV1::NonCanonicalPath);
        }
        Ok(Self(Arc::from(path)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SemanticHelperRelativePathV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SemanticHelperRelativePathV1(<redacted>)")
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct SemanticHelperSnapshotEntryV1 {
    path: SemanticHelperRelativePathV1,
    byte_len: u64,
    digest: SemanticHelperSnapshotDigestV1,
}

impl SemanticHelperSnapshotEntryV1 {
    pub fn new(
        path: SemanticHelperRelativePathV1,
        byte_len: u64,
        digest: [u8; 32],
    ) -> Result<Self, SemanticHelperSnapshotErrorV1> {
        let digest = SemanticHelperSnapshotDigestV1::new(digest)
            .ok_or(SemanticHelperSnapshotErrorV1::InvalidDigest)?;
        Ok(Self {
            path,
            byte_len,
            digest,
        })
    }

    pub fn path(&self) -> &SemanticHelperRelativePathV1 {
        &self.path
    }

    pub const fn byte_len(&self) -> u64 {
        self.byte_len
    }

    pub const fn digest(&self) -> SemanticHelperSnapshotDigestV1 {
        self.digest
    }
}

impl fmt::Debug for SemanticHelperSnapshotEntryV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticHelperSnapshotEntryV1")
            .field("path", &"<redacted>")
            .field("byte_len", &self.byte_len)
            .field("digest", &"<opaque>")
            .finish()
    }
}

/// Sorted, duplicate-free, size-bounded commitment to a complete snapshot.
#[derive(Clone, Eq, PartialEq)]
pub struct SemanticHelperSnapshotManifestV1 {
    entries: Arc<[SemanticHelperSnapshotEntryV1]>,
    total_bytes: u64,
    digest: SemanticHelperSnapshotDigestV1,
}

impl SemanticHelperSnapshotManifestV1 {
    pub fn try_from_entries(
        mut entries: Vec<SemanticHelperSnapshotEntryV1>,
    ) -> Result<Self, SemanticHelperSnapshotErrorV1> {
        if entries.is_empty() {
            return Err(SemanticHelperSnapshotErrorV1::EmptySnapshot);
        }
        if entries.len() > MAX_HELPER_SNAPSHOT_FILES_V1 {
            return Err(SemanticHelperSnapshotErrorV1::TooManyFiles);
        }
        entries.sort_unstable_by(|left, right| left.path.cmp(&right.path));
        if entries.windows(2).any(|pair| pair[0].path == pair[1].path) {
            return Err(SemanticHelperSnapshotErrorV1::DuplicatePath);
        }
        let total_bytes = entries
            .iter()
            .try_fold(0_u64, |total, entry| total.checked_add(entry.byte_len))
            .ok_or(SemanticHelperSnapshotErrorV1::SnapshotTooLarge)?;
        if total_bytes > MAX_HELPER_SNAPSHOT_BYTES_V1 {
            return Err(SemanticHelperSnapshotErrorV1::SnapshotTooLarge);
        }

        let mut hasher = Sha256::new();
        hasher.update(b"vellum-semantic-helper-snapshot-v1\0");
        hasher.update((entries.len() as u64).to_be_bytes());
        hasher.update(total_bytes.to_be_bytes());
        for entry in &entries {
            let path = entry.path.as_str().as_bytes();
            hasher.update((path.len() as u64).to_be_bytes());
            hasher.update(path);
            hasher.update(entry.byte_len.to_be_bytes());
            hasher.update(entry.digest.as_bytes());
        }
        let digest = SemanticHelperSnapshotDigestV1::new(hasher.finalize().into())
            .expect("SHA-256 domain commitment is nonzero");
        Ok(Self {
            entries: entries.into(),
            total_bytes,
            digest,
        })
    }

    pub fn entries(&self) -> &[SemanticHelperSnapshotEntryV1] {
        &self.entries
    }

    pub const fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    pub const fn digest(&self) -> SemanticHelperSnapshotDigestV1 {
        self.digest
    }
}

impl fmt::Debug for SemanticHelperSnapshotManifestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticHelperSnapshotManifestV1")
            .field("file_count", &self.entries.len())
            .field("total_bytes", &self.total_bytes)
            .field("digest", &"<opaque>")
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticHelperResourceErrorV1 {
    ZeroLimit,
    LimitTooLarge,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticHelperResourceLimitsV1 {
    memory_bytes: u64,
    process_count: u16,
    cpu_millis: u64,
}

impl SemanticHelperResourceLimitsV1 {
    pub fn new(
        memory_bytes: u64,
        process_count: u16,
        cpu_millis: u64,
    ) -> Result<Self, SemanticHelperResourceErrorV1> {
        if memory_bytes == 0 || process_count == 0 || cpu_millis == 0 {
            return Err(SemanticHelperResourceErrorV1::ZeroLimit);
        }
        if memory_bytes > MAX_HELPER_PROCESS_MEMORY_BYTES_V1
            || process_count > MAX_HELPER_PROCESSES_V1
            || cpu_millis > MAX_HELPER_CPU_MILLIS_V1
        {
            return Err(SemanticHelperResourceErrorV1::LimitTooLarge);
        }
        Ok(Self {
            memory_bytes,
            process_count,
            cpu_millis,
        })
    }

    pub const fn memory_bytes(self) -> u64 {
        self.memory_bytes
    }
    pub const fn process_count(self) -> u16 {
        self.process_count
    }
    pub const fn cpu_millis(self) -> u64 {
        self.cpu_millis
    }
}

/// Fixed required isolation product. Only quantitative bounds are selectable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticHelperIsolationPolicyV1 {
    resources: SemanticHelperResourceLimitsV1,
}

impl SemanticHelperIsolationPolicyV1 {
    pub const fn new(resources: SemanticHelperResourceLimitsV1) -> Self {
        Self { resources }
    }
    pub const fn resources(self) -> SemanticHelperResourceLimitsV1 {
        self.resources
    }
    pub const fn network_allowed(self) -> bool {
        false
    }
    pub const fn snapshot_read_only(self) -> bool {
        true
    }
    pub const fn private_temporary_directory(self) -> bool {
        true
    }
    pub const fn terminate_on_client_disconnect(self) -> bool {
        true
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticHelperRequestV1 {
    Start {
        exchange: SemanticHelperExchangeRefV1,
        session: SemanticHelperSessionRefV1,
        snapshot: SemanticHelperSnapshotManifestV1,
        /// Digest of the separately qualified provider artifact and profile.
        provider_qualification: SemanticHelperProviderQualificationDigestV1,
        isolation: SemanticHelperIsolationPolicyV1,
    },
    Poll {
        exchange: SemanticHelperExchangeRefV1,
        session: SemanticHelperSessionRefV1,
    },
    Stop {
        exchange: SemanticHelperExchangeRefV1,
        session: SemanticHelperSessionRefV1,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticHelperSessionPhaseV1 {
    Starting,
    Ready,
    Stopping,
    Stopped,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticHelperRefusalV1 {
    AlreadyExists,
    UnknownSession,
    SnapshotMismatch,
    IsolationUnavailable,
    ResourceLimit,
    ProviderFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticHelperReplyV1 {
    Status {
        exchange: SemanticHelperExchangeRefV1,
        session: SemanticHelperSessionRefV1,
        snapshot: SemanticHelperSnapshotDigestV1,
        phase: SemanticHelperSessionPhaseV1,
    },
    Refused {
        exchange: SemanticHelperExchangeRefV1,
        session: SemanticHelperSessionRefV1,
        refusal: SemanticHelperRefusalV1,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, len: u64, marker: u8) -> SemanticHelperSnapshotEntryV1 {
        SemanticHelperSnapshotEntryV1::new(
            SemanticHelperRelativePathV1::new(path).unwrap(),
            len,
            [marker; 32],
        )
        .unwrap()
    }

    #[test]
    fn paths_are_relative_and_canonical() {
        for path in ["", "/tmp/a", "a//b", "a/./b", "a/../b", "a\\b", "a\0b"] {
            assert!(SemanticHelperRelativePathV1::new(path).is_err(), "{path:?}");
        }
        assert_eq!(
            SemanticHelperRelativePathV1::new("src/lib.rs")
                .unwrap()
                .as_str(),
            "src/lib.rs"
        );
    }

    #[test]
    fn manifests_sort_reject_duplicates_and_bind_content() {
        let first = SemanticHelperSnapshotManifestV1::try_from_entries(vec![
            entry("z.rs", 2, 2),
            entry("a.rs", 1, 1),
        ])
        .unwrap();
        let reordered = SemanticHelperSnapshotManifestV1::try_from_entries(vec![
            entry("a.rs", 1, 1),
            entry("z.rs", 2, 2),
        ])
        .unwrap();
        let changed = SemanticHelperSnapshotManifestV1::try_from_entries(vec![
            entry("a.rs", 1, 3),
            entry("z.rs", 2, 2),
        ])
        .unwrap();
        assert_eq!(first.digest(), reordered.digest());
        assert_ne!(first.digest(), changed.digest());
        assert_eq!(first.entries()[0].path().as_str(), "a.rs");
        assert_eq!(first.total_bytes(), 3);
        assert_eq!(
            SemanticHelperSnapshotManifestV1::try_from_entries(vec![
                entry("a", 1, 1),
                entry("a", 1, 1)
            ]),
            Err(SemanticHelperSnapshotErrorV1::DuplicatePath),
        );
    }

    #[test]
    fn snapshots_are_bounded_without_partial_success() {
        assert_eq!(
            SemanticHelperSnapshotManifestV1::try_from_entries(Vec::new()),
            Err(SemanticHelperSnapshotErrorV1::EmptySnapshot),
        );
        assert_eq!(
            SemanticHelperSnapshotManifestV1::try_from_entries(vec![entry(
                "large",
                MAX_HELPER_SNAPSHOT_BYTES_V1 + 1,
                1
            )]),
            Err(SemanticHelperSnapshotErrorV1::SnapshotTooLarge),
        );
    }

    #[test]
    fn debug_redacts_workspace_evidence() {
        let manifest = SemanticHelperSnapshotManifestV1::try_from_entries(vec![entry(
            "secret/customer.rs",
            7,
            9,
        )])
        .unwrap();
        let debug = format!("{manifest:?} {:?}", manifest.entries()[0]);
        assert!(!debug.contains("secret"));
        assert!(!debug.contains("0909"));
    }

    #[test]
    fn required_isolation_cannot_be_disabled() {
        let resources = SemanticHelperResourceLimitsV1::new(1024, 2, 500).unwrap();
        let policy = SemanticHelperIsolationPolicyV1::new(resources);
        assert!(!policy.network_allowed());
        assert!(policy.snapshot_read_only());
        assert!(policy.private_temporary_directory());
        assert!(policy.terminate_on_client_disconnect());
        assert_eq!(policy.resources(), resources);
        assert_eq!(
            SemanticHelperResourceLimitsV1::new(0, 1, 1),
            Err(SemanticHelperResourceErrorV1::ZeroLimit)
        );
    }

    #[test]
    fn references_are_nonzero_and_debug_opaque() {
        assert!(SemanticHelperSessionRefV1::new([0; 16]).is_none());
        let reference = SemanticHelperSessionRefV1::new([7; 16]).unwrap();
        assert_eq!(
            format!("{reference:?}"),
            "SemanticHelperSessionRefV1(<opaque>)"
        );
    }

    #[test]
    fn start_request_binds_snapshot_provider_and_fixed_isolation_without_disclosure() {
        let snapshot = SemanticHelperSnapshotManifestV1::try_from_entries(vec![entry(
            "private/source.rs",
            7,
            9,
        )])
        .unwrap();
        let request = SemanticHelperRequestV1::Start {
            exchange: SemanticHelperExchangeRefV1::new([1; 16]).unwrap(),
            session: SemanticHelperSessionRefV1::new([2; 16]).unwrap(),
            snapshot,
            provider_qualification: SemanticHelperProviderQualificationDigestV1::new([3; 32])
                .unwrap(),
            isolation: SemanticHelperIsolationPolicyV1::new(
                SemanticHelperResourceLimitsV1::new(1024, 2, 500).unwrap(),
            ),
        };
        let SemanticHelperRequestV1::Start {
            snapshot,
            provider_qualification,
            isolation,
            ..
        } = &request
        else {
            unreachable!()
        };
        assert_eq!(provider_qualification.as_bytes(), [3; 32]);
        assert_ne!(snapshot.digest().as_bytes(), [0; 32]);
        assert!(!isolation.network_allowed());
        assert!(isolation.snapshot_read_only());
        let debug = format!("{request:?}");
        assert!(!debug.contains("private"));
        assert!(!debug.contains("0303"));
    }
}
