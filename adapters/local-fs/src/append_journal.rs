//! Crash-conservative, descriptor-relative local append journal.

use rustix::fs::{AtFlags, Dir, FlockOperation, Mode, OFlags, flock, mkdirat, openat, statat};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::darwin::fs::MetadataExt as DarwinMetadataExt,
    os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
use vellum_host::append_journal::{
    AppendJournalCommitV1, AppendJournalLimitsV1, AppendJournalPortV1, AppendJournalReadRefusalV1,
};

const DIRECTORY_NAME: &str = ".vellum-append-journal-v1";
const NAMESPACE_FILE: &str = "namespace";
const NAMESPACE_MAGIC: &[u8; 8] = b"VELLAJ01";
const NAMESPACE_BYTES: usize = 8 + 16 + 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalAppendJournalNamespaceV1 {
    pub workspace: [u8; 16],
    pub owner: [u8; 16],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalAppendJournalOpenRefusalV1 {
    InvalidLimits,
    Missing,
    Unsafe,
    NamespaceMismatch,
    InUse,
    CapacityExceeded,
    OutcomeUnknown,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LocalAppendJournalUsageV1 {
    pub read_attempts: u64,
    pub reads: u64,
    pub append_attempts: u64,
    pub committed: u64,
    pub refused: u64,
    pub indeterminate: u64,
    pub counter_saturated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DirectoryIdentity {
    device: u64,
    inode: u64,
}

struct AnchoredDirectory {
    descriptor: File,
    namespace_path: PathBuf,
    identity: DirectoryIdentity,
}

/// A journal bound to one workspace/owner pair. The pair is used only for
/// equality checks and is never emitted through usage instrumentation.
pub struct LocalAppendJournalV1 {
    directory: AnchoredDirectory,
    namespace_bytes: [u8; NAMESPACE_BYTES],
    limits: AppendJournalLimitsV1,
    usage: LocalAppendJournalUsageV1,
    #[cfg(test)]
    crash_cut: Option<CrashCut>,
}

impl LocalAppendJournalV1 {
    pub fn try_open(
        parent: &Path,
        namespace: LocalAppendJournalNamespaceV1,
        limits: AppendJournalLimitsV1,
    ) -> Result<Self, LocalAppendJournalOpenRefusalV1> {
        if !limits.is_valid() {
            return Err(LocalAppendJournalOpenRefusalV1::InvalidLimits);
        }
        let directory = open_or_create_directory(parent)?;
        let namespace_bytes = encode_namespace(namespace);
        initialize_or_validate_namespace(&directory, &namespace_bytes)?;
        let journal = Self {
            directory,
            namespace_bytes,
            limits,
            usage: LocalAppendJournalUsageV1::default(),
            #[cfg(test)]
            crash_cut: None,
        };
        journal.snapshot(limits).map_err(|reason| match reason {
            AppendJournalReadRefusalV1::Limit => LocalAppendJournalOpenRefusalV1::CapacityExceeded,
            AppendJournalReadRefusalV1::Unavailable => LocalAppendJournalOpenRefusalV1::Unsafe,
        })?;
        Ok(journal)
    }

    pub const fn usage(&self) -> LocalAppendJournalUsageV1 {
        self.usage
    }

    fn snapshot(
        &self,
        limits: AppendJournalLimitsV1,
    ) -> Result<Vec<Vec<u8>>, AppendJournalReadRefusalV1> {
        if !limits.is_valid() {
            return Err(AppendJournalReadRefusalV1::Limit);
        }
        self.directory
            .revalidate()
            .map_err(|_| AppendJournalReadRefusalV1::Unavailable)?;
        let observed_namespace = self
            .directory
            .read_private_file(NAMESPACE_FILE, NAMESPACE_BYTES)
            .map_err(map_read_error)?;
        if observed_namespace.as_slice() != self.namespace_bytes {
            return Err(AppendJournalReadRefusalV1::Unavailable);
        }

        let mut names = self.directory.entry_names().map_err(map_read_error)?;
        names.sort_unstable();
        let mut records = Vec::new();
        let mut total = 0usize;
        for name in names {
            if name == NAMESPACE_FILE {
                continue;
            }
            let ordinal =
                parse_record_name(&name).ok_or(AppendJournalReadRefusalV1::Unavailable)?;
            if name != record_name(ordinal) {
                return Err(AppendJournalReadRefusalV1::Unavailable);
            }
            if ordinal != records.len() || records.len() == limits.max_records {
                return Err(AppendJournalReadRefusalV1::Limit);
            }
            let record = self
                .directory
                .read_private_file(&name, limits.max_record_bytes)
                .map_err(map_read_error)?;
            if record.is_empty() {
                return Err(AppendJournalReadRefusalV1::Unavailable);
            }
            total = total
                .checked_add(record.len())
                .ok_or(AppendJournalReadRefusalV1::Limit)?;
            if total > limits.max_total_bytes {
                return Err(AppendJournalReadRefusalV1::Limit);
            }
            records
                .try_reserve(1)
                .map_err(|_| AppendJournalReadRefusalV1::Limit)?;
            records.push(record);
        }
        let final_namespace = self
            .directory
            .read_private_file(NAMESPACE_FILE, NAMESPACE_BYTES)
            .map_err(map_read_error)?;
        if final_namespace.as_slice() != self.namespace_bytes {
            return Err(AppendJournalReadRefusalV1::Unavailable);
        }
        self.directory
            .revalidate()
            .map_err(|_| AppendJournalReadRefusalV1::Unavailable)?;
        Ok(records)
    }
}

impl AppendJournalPortV1 for LocalAppendJournalV1 {
    fn read_bounded(
        &mut self,
        limits: AppendJournalLimitsV1,
    ) -> Result<Vec<Vec<u8>>, AppendJournalReadRefusalV1> {
        bump(
            &mut self.usage.read_attempts,
            &mut self.usage.counter_saturated,
        );
        if !limits.is_valid() {
            return Err(AppendJournalReadRefusalV1::Limit);
        }
        let effective = AppendJournalLimitsV1 {
            max_records: limits.max_records.min(self.limits.max_records),
            max_record_bytes: limits.max_record_bytes.min(self.limits.max_record_bytes),
            max_total_bytes: limits.max_total_bytes.min(self.limits.max_total_bytes),
        };
        let result = self.snapshot(effective);
        if result.is_ok() {
            bump(&mut self.usage.reads, &mut self.usage.counter_saturated);
        }
        result
    }

    fn append_synchronized(&mut self, record: &[u8]) -> AppendJournalCommitV1 {
        bump(
            &mut self.usage.append_attempts,
            &mut self.usage.counter_saturated,
        );
        let before = match self.snapshot(self.limits) {
            Ok(records) => records,
            Err(_) => return self.observe_commit(AppendJournalCommitV1::Refused),
        };
        let total = match before
            .iter()
            .try_fold(0usize, |sum, value| sum.checked_add(value.len()))
            .and_then(|sum| sum.checked_add(record.len()))
        {
            Some(total) => total,
            None => return self.observe_commit(AppendJournalCommitV1::Refused),
        };
        if record.is_empty()
            || record.len() > self.limits.max_record_bytes
            || before.len() == self.limits.max_records
            || total > self.limits.max_total_bytes
        {
            return self.observe_commit(AppendJournalCommitV1::Refused);
        }
        #[cfg(test)]
        if self.crash_cut == Some(CrashCut::BeforeCreate) {
            self.crash_cut = None;
            return self.observe_commit(AppendJournalCommitV1::Refused);
        }

        let name = record_name(before.len());
        if self.directory.create_private_file(&name, record).is_err() {
            return self.observe_commit(AppendJournalCommitV1::Indeterminate);
        }
        #[cfg(test)]
        if self.crash_cut == Some(CrashCut::AfterFileSync) {
            self.crash_cut = None;
            return self.observe_commit(AppendJournalCommitV1::Indeterminate);
        }
        if self.directory.synchronize_and_revalidate().is_err() {
            return self.observe_commit(AppendJournalCommitV1::Indeterminate);
        }
        let verified = self
            .directory
            .read_private_file(&name, self.limits.max_record_bytes)
            .is_ok_and(|bytes| bytes == record);
        let namespace_valid = self
            .directory
            .read_private_file(NAMESPACE_FILE, NAMESPACE_BYTES)
            .is_ok_and(|bytes| bytes.as_slice() == self.namespace_bytes);
        if !verified || !namespace_valid || self.directory.revalidate().is_err() {
            return self.observe_commit(AppendJournalCommitV1::Indeterminate);
        }
        self.observe_commit(AppendJournalCommitV1::Committed)
    }
}

impl LocalAppendJournalV1 {
    fn observe_commit(&mut self, outcome: AppendJournalCommitV1) -> AppendJournalCommitV1 {
        let field = match outcome {
            AppendJournalCommitV1::Committed => &mut self.usage.committed,
            AppendJournalCommitV1::Refused => &mut self.usage.refused,
            AppendJournalCommitV1::Indeterminate => &mut self.usage.indeterminate,
        };
        bump(field, &mut self.usage.counter_saturated);
        outcome
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FsError {
    Missing,
    Unsafe,
    Limit,
    Conflict,
    Io,
    OutcomeUnknown,
}

fn open_or_create_directory(
    parent: &Path,
) -> Result<AnchoredDirectory, LocalAppendJournalOpenRefusalV1> {
    let parent_descriptor = open_directory_descriptor(parent).map_err(map_open_error)?;
    match mkdirat(
        &parent_descriptor,
        DIRECTORY_NAME,
        Mode::from_raw_mode(0o700),
    ) {
        Ok(()) | Err(rustix::io::Errno::EXIST) => {}
        Err(_) => return Err(LocalAppendJournalOpenRefusalV1::Unavailable),
    }
    synchronize(&parent_descriptor)
        .map_err(|()| LocalAppendJournalOpenRefusalV1::OutcomeUnknown)?;
    let directory = open_anchored_directory(
        &parent_descriptor,
        DIRECTORY_NAME,
        parent.join(DIRECTORY_NAME),
    )
    .map_err(map_open_error)?;
    flock(
        &directory.descriptor,
        FlockOperation::NonBlockingLockExclusive,
    )
    .map_err(|error| {
        if error == rustix::io::Errno::WOULDBLOCK {
            LocalAppendJournalOpenRefusalV1::InUse
        } else {
            LocalAppendJournalOpenRefusalV1::Unavailable
        }
    })?;
    Ok(directory)
}

fn open_directory_descriptor(path: &Path) -> Result<File, FsError> {
    let flags = i32::try_from((OFlags::NOFOLLOW | OFlags::DIRECTORY | OFlags::CLOEXEC).bits())
        .map_err(|_| FsError::Io)?;
    OpenOptions::new()
        .read(true)
        .custom_flags(flags)
        .open(path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                FsError::Missing
            } else {
                FsError::Io
            }
        })
}

fn open_anchored_directory(
    parent: &File,
    name: &str,
    namespace_path: PathBuf,
) -> Result<AnchoredDirectory, FsError> {
    let descriptor = File::from(
        openat(
            parent,
            name,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::DIRECTORY,
            Mode::empty(),
        )
        .map_err(|error| {
            if error == rustix::io::Errno::NOENT {
                FsError::Missing
            } else if error == rustix::io::Errno::LOOP || error == rustix::io::Errno::NOTDIR {
                FsError::Unsafe
            } else {
                FsError::Io
            }
        })?,
    );
    let metadata = descriptor.metadata().map_err(|_| FsError::Io)?;
    if !private_directory_security(&descriptor, &metadata) {
        return Err(FsError::Unsafe);
    }
    Ok(AnchoredDirectory {
        descriptor,
        namespace_path,
        identity: DirectoryIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
        },
    })
}

impl AnchoredDirectory {
    fn entry_names(&self) -> Result<Vec<String>, FsError> {
        let mut directory = Dir::read_from(&self.descriptor).map_err(|_| FsError::Io)?;
        let mut names = Vec::new();
        for entry in &mut directory {
            let entry = entry.map_err(|_| FsError::Io)?;
            let bytes = entry.file_name().to_bytes();
            if bytes == b"." || bytes == b".." {
                continue;
            }
            let name = std::str::from_utf8(bytes).map_err(|_| FsError::Unsafe)?;
            names.try_reserve(1).map_err(|_| FsError::Limit)?;
            names.push(name.to_owned());
        }
        Ok(names)
    }

    fn entry_exists(&self, name: &str) -> Result<bool, FsError> {
        match statat(&self.descriptor, name, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(_) => Ok(true),
            Err(error) if error == rustix::io::Errno::NOENT => Ok(false),
            Err(_) => Err(FsError::Io),
        }
    }

    fn create_private_file(&self, name: &str, bytes: &[u8]) -> Result<(), FsError> {
        let mut file = File::from(
            openat(
                &self.descriptor,
                name,
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC | OFlags::NOFOLLOW,
                Mode::from_raw_mode(0o600),
            )
            .map_err(|error| {
                if error == rustix::io::Errno::EXIST {
                    FsError::Conflict
                } else {
                    FsError::Io
                }
            })?,
        );
        let metadata = file.metadata().map_err(|_| FsError::OutcomeUnknown)?;
        if !private_file_security(&file, &metadata) {
            return Err(FsError::OutcomeUnknown);
        }
        file.write_all(bytes).map_err(|_| FsError::OutcomeUnknown)?;
        synchronize(&file).map_err(|()| FsError::OutcomeUnknown)
    }

    fn read_private_file(&self, name: &str, max: usize) -> Result<Vec<u8>, FsError> {
        let file = File::from(
            openat(
                &self.descriptor,
                name,
                OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
                Mode::empty(),
            )
            .map_err(|error| {
                if error == rustix::io::Errno::NOENT {
                    FsError::Missing
                } else if error == rustix::io::Errno::LOOP {
                    FsError::Unsafe
                } else {
                    FsError::Io
                }
            })?,
        );
        let metadata = file.metadata().map_err(|_| FsError::Io)?;
        if !private_file_security(&file, &metadata)
            || usize::try_from(metadata.len()).map_or(true, |length| length > max)
        {
            return Err(FsError::Unsafe);
        }
        let mut bytes = Vec::new();
        file.take(u64::try_from(max).map_err(|_| FsError::Limit)? + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| FsError::Io)?;
        if bytes.len() > max {
            return Err(FsError::Limit);
        }
        Ok(bytes)
    }

    fn revalidate(&self) -> Result<(), FsError> {
        let anchored_metadata = self.descriptor.metadata().map_err(|_| FsError::Io)?;
        if !private_directory_security(&self.descriptor, &anchored_metadata) {
            return Err(FsError::Unsafe);
        }
        let observed = open_directory_descriptor(&self.namespace_path).and_then(|directory| {
            let metadata = directory.metadata().map_err(|_| FsError::Io)?;
            if !private_directory_security(&directory, &metadata) {
                return Err(FsError::Unsafe);
            }
            Ok(DirectoryIdentity {
                device: metadata.dev(),
                inode: metadata.ino(),
            })
        });
        match observed {
            Ok(identity) if identity == self.identity => Ok(()),
            Ok(_) | Err(FsError::Missing) => Err(FsError::Conflict),
            Err(reason) => Err(reason),
        }
    }

    fn synchronize_and_revalidate(&self) -> Result<(), FsError> {
        synchronize(&self.descriptor).map_err(|()| FsError::OutcomeUnknown)?;
        self.revalidate().map_err(|_| FsError::OutcomeUnknown)
    }
}

fn private_directory_security(file: &File, metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_dir()
        && metadata.uid() == rustix::process::geteuid().as_raw()
        && metadata.mode() & 0o7777 == 0o700
        && metadata.nlink() >= 2
        && DarwinMetadataExt::st_flags(metadata) == 0
        && crate::macos::native_acl::is_empty(file).unwrap_or(false)
}

fn private_file_security(file: &File, metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_file()
        && !metadata.file_type().is_socket()
        && metadata.uid() == rustix::process::geteuid().as_raw()
        && metadata.mode() & 0o7777 == 0o600
        && metadata.nlink() == 1
        && DarwinMetadataExt::st_flags(metadata) == 0
        && crate::macos::native_acl::is_empty(file).unwrap_or(false)
}

fn initialize_or_validate_namespace(
    directory: &AnchoredDirectory,
    expected: &[u8; NAMESPACE_BYTES],
) -> Result<(), LocalAppendJournalOpenRefusalV1> {
    match directory
        .entry_exists(NAMESPACE_FILE)
        .map_err(map_open_error)?
    {
        true => {
            let observed = directory
                .read_private_file(NAMESPACE_FILE, NAMESPACE_BYTES)
                .map_err(map_open_error)?;
            if observed.as_slice() != expected {
                return Err(LocalAppendJournalOpenRefusalV1::NamespaceMismatch);
            }
        }
        false => {
            directory
                .create_private_file(NAMESPACE_FILE, expected)
                .map_err(|_| LocalAppendJournalOpenRefusalV1::OutcomeUnknown)?;
            directory
                .synchronize_and_revalidate()
                .map_err(|_| LocalAppendJournalOpenRefusalV1::OutcomeUnknown)?;
        }
    }
    Ok(())
}

fn encode_namespace(namespace: LocalAppendJournalNamespaceV1) -> [u8; NAMESPACE_BYTES] {
    let mut bytes = [0u8; NAMESPACE_BYTES];
    bytes[..8].copy_from_slice(NAMESPACE_MAGIC);
    bytes[8..24].copy_from_slice(&namespace.workspace);
    bytes[24..].copy_from_slice(&namespace.owner);
    bytes
}

fn record_name(ordinal: usize) -> String {
    format!("record-{ordinal:016x}.bin")
}

fn parse_record_name(name: &str) -> Option<usize> {
    let ordinal = name.strip_prefix("record-")?.strip_suffix(".bin")?;
    if ordinal.len() != 16 || !ordinal.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    usize::try_from(u64::from_str_radix(ordinal, 16).ok()?).ok()
}

fn map_read_error(error: FsError) -> AppendJournalReadRefusalV1 {
    match error {
        FsError::Limit => AppendJournalReadRefusalV1::Limit,
        _ => AppendJournalReadRefusalV1::Unavailable,
    }
}

fn map_open_error(error: FsError) -> LocalAppendJournalOpenRefusalV1 {
    match error {
        FsError::Missing => LocalAppendJournalOpenRefusalV1::Missing,
        FsError::Unsafe | FsError::Conflict => LocalAppendJournalOpenRefusalV1::Unsafe,
        FsError::OutcomeUnknown => LocalAppendJournalOpenRefusalV1::OutcomeUnknown,
        FsError::Io | FsError::Limit => LocalAppendJournalOpenRefusalV1::Unavailable,
    }
}

fn synchronize(file: &File) -> Result<(), ()> {
    rustix::fs::fcntl_fullfsync(file).map_err(|_| ())
}

fn bump(value: &mut u64, saturated: &mut bool) {
    if let Some(next) = value.checked_add(1) {
        *value = next;
    } else {
        *saturated = true;
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CrashCut {
    BeforeCreate,
    AfterFileSync,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::{PermissionsExt, symlink},
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT: AtomicU64 = AtomicU64::new(1);

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "vellum-append-journal-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn namespace() -> LocalAppendJournalNamespaceV1 {
        LocalAppendJournalNamespaceV1 {
            workspace: [7; 16],
            owner: [9; 16],
        }
    }

    #[test]
    fn append_is_durable_and_restart_recovers_in_order() {
        let fixture = Fixture::new();
        let limits = AppendJournalLimitsV1::reference();
        let mut first = LocalAppendJournalV1::try_open(&fixture.0, namespace(), limits).unwrap();
        assert_eq!(
            first.append_synchronized(b"one"),
            AppendJournalCommitV1::Committed
        );
        assert_eq!(
            first.append_synchronized(b"two"),
            AppendJournalCommitV1::Committed
        );
        drop(first);
        let mut recovered =
            LocalAppendJournalV1::try_open(&fixture.0, namespace(), limits).unwrap();
        assert_eq!(
            recovered.read_bounded(limits).unwrap(),
            vec![b"one".to_vec(), b"two".to_vec()]
        );
    }

    #[test]
    fn namespace_owner_binding_is_exact() {
        let fixture = Fixture::new();
        let limits = AppendJournalLimitsV1::reference();
        LocalAppendJournalV1::try_open(&fixture.0, namespace(), limits).unwrap();
        let mut other = namespace();
        other.owner = [8; 16];
        assert!(matches!(
            LocalAppendJournalV1::try_open(&fixture.0, other, limits),
            Err(LocalAppendJournalOpenRefusalV1::NamespaceMismatch)
        ));
    }

    #[test]
    fn one_open_handle_exclusively_owns_the_journal() {
        let fixture = Fixture::new();
        let limits = AppendJournalLimitsV1::reference();
        let first = LocalAppendJournalV1::try_open(&fixture.0, namespace(), limits).unwrap();
        assert!(matches!(
            LocalAppendJournalV1::try_open(&fixture.0, namespace(), limits),
            Err(LocalAppendJournalOpenRefusalV1::InUse)
        ));
        drop(first);
        LocalAppendJournalV1::try_open(&fixture.0, namespace(), limits).unwrap();
    }

    #[test]
    fn live_permission_drift_is_refused() {
        let fixture = Fixture::new();
        let limits = AppendJournalLimitsV1::reference();
        let mut journal = LocalAppendJournalV1::try_open(&fixture.0, namespace(), limits).unwrap();
        fs::set_permissions(
            fixture.0.join(DIRECTORY_NAME),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        assert_eq!(
            journal.read_bounded(limits),
            Err(AppendJournalReadRefusalV1::Unavailable)
        );
        assert_eq!(
            journal.append_synchronized(b"no"),
            AppendJournalCommitV1::Refused
        );
    }

    #[test]
    fn strict_bounds_refuse_without_eviction() {
        let fixture = Fixture::new();
        let limits = AppendJournalLimitsV1 {
            max_records: 1,
            max_record_bytes: 3,
            max_total_bytes: 3,
        };
        let mut journal = LocalAppendJournalV1::try_open(&fixture.0, namespace(), limits).unwrap();
        assert_eq!(
            journal.append_synchronized(b"one"),
            AppendJournalCommitV1::Committed
        );
        assert_eq!(
            journal.append_synchronized(b"two"),
            AppendJournalCommitV1::Refused
        );
        assert_eq!(journal.read_bounded(limits).unwrap(), vec![b"one".to_vec()]);
    }

    #[test]
    fn crash_cuts_report_only_provable_truth_and_restart_recovers() {
        let fixture = Fixture::new();
        let limits = AppendJournalLimitsV1::reference();
        let mut journal = LocalAppendJournalV1::try_open(&fixture.0, namespace(), limits).unwrap();
        journal.crash_cut = Some(CrashCut::BeforeCreate);
        assert_eq!(
            journal.append_synchronized(b"absent"),
            AppendJournalCommitV1::Refused
        );
        journal.crash_cut = Some(CrashCut::AfterFileSync);
        assert_eq!(
            journal.append_synchronized(b"maybe"),
            AppendJournalCommitV1::Indeterminate
        );
        drop(journal);
        let mut recovered =
            LocalAppendJournalV1::try_open(&fixture.0, namespace(), limits).unwrap();
        assert_eq!(
            recovered.read_bounded(limits).unwrap(),
            vec![b"maybe".to_vec()]
        );
    }

    #[test]
    fn symlink_hardlink_mode_and_rename_adversaries_fail_closed() {
        let limits = AppendJournalLimitsV1::reference();

        let symlink_fixture = Fixture::new();
        fs::create_dir(symlink_fixture.0.join("target")).unwrap();
        fs::set_permissions(
            symlink_fixture.0.join("target"),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        symlink("target", symlink_fixture.0.join(DIRECTORY_NAME)).unwrap();
        assert!(LocalAppendJournalV1::try_open(&symlink_fixture.0, namespace(), limits).is_err());

        let mode_fixture = Fixture::new();
        let journal = LocalAppendJournalV1::try_open(&mode_fixture.0, namespace(), limits).unwrap();
        fs::set_permissions(
            mode_fixture.0.join(DIRECTORY_NAME),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        drop(journal);
        assert!(LocalAppendJournalV1::try_open(&mode_fixture.0, namespace(), limits).is_err());

        let link_fixture = Fixture::new();
        let mut journal =
            LocalAppendJournalV1::try_open(&link_fixture.0, namespace(), limits).unwrap();
        assert_eq!(
            journal.append_synchronized(b"one"),
            AppendJournalCommitV1::Committed
        );
        fs::hard_link(
            link_fixture.0.join(DIRECTORY_NAME).join(record_name(0)),
            link_fixture.0.join("outside-link"),
        )
        .unwrap();
        assert_eq!(
            journal.read_bounded(limits),
            Err(AppendJournalReadRefusalV1::Unavailable)
        );

        let rename_fixture = Fixture::new();
        let mut journal =
            LocalAppendJournalV1::try_open(&rename_fixture.0, namespace(), limits).unwrap();
        fs::rename(
            rename_fixture.0.join(DIRECTORY_NAME),
            rename_fixture.0.join("moved"),
        )
        .unwrap();
        fs::create_dir(rename_fixture.0.join(DIRECTORY_NAME)).unwrap();
        fs::set_permissions(
            rename_fixture.0.join(DIRECTORY_NAME),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        assert_eq!(
            journal.append_synchronized(b"no"),
            AppendJournalCommitV1::Refused
        );
    }
}
