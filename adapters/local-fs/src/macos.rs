use rustix::fs::{self, AtFlags, Mode, OFlags};
use std::ffi::{CString, OsString};
use std::fs::{File, Metadata};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::darwin::fs::MetadataExt as DarwinMetadataExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use vellum_engine::instrumentation::{Boundary, Disposition, Observation, Recorder};
use vellum_engine::resource::{
    AnnotationReadDispatch, AnnotationReadEvidence, AnnotationReadTarget, Application,
    OrdinaryPairCheck, OrdinaryPairEvidence, ProviderPort, ReadFailure, ReadStrength,
    ResourceError, ResourceRead, ResourceRelation, SaveDispatch, SaveEvidence, Synchronization,
};

/// No numeric deployment defaults are selected by the adapter.
#[derive(Clone, Copy, Debug)]
pub struct FilesystemLimits {
    pub max_resources: usize,
    pub max_bytes: usize,
    pub max_stage_attempts: u32,
    pub max_metadata_bytes: usize,
}
/// Content-free classifications; raw OS errors and paths are not diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilesystemError {
    Io,
    Limit,
    Capacity,
    Unavailable,
    Conflict,
    UnsupportedMetadata,
    UnsupportedFile,
    Resource(ResourceError),
}
impl From<std::io::Error> for FilesystemError {
    fn from(_: std::io::Error) -> Self {
        Self::Io
    }
}
impl From<rustix::io::Errno> for FilesystemError {
    fn from(_: rustix::io::Errno) -> Self {
        Self::Io
    }
}
impl From<ResourceError> for FilesystemError {
    fn from(error: ResourceError) -> Self {
        Self::Resource(error)
    }
}
#[derive(Clone)]
pub struct ResourceHandle {
    owner: Arc<()>,
    slot: usize,
}
impl std::fmt::Debug for ResourceHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResourceHandle(<opaque>)")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Stamp {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) size: u64,
    pub(crate) modified: i64,
    pub(crate) modified_ns: i64,
    pub(crate) changed: i64,
    pub(crate) changed_ns: i64,
    pub(crate) mode: u32,
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) links: u64,
    pub(crate) flags: u32,
}
impl Stamp {
    fn from(m: &Metadata) -> Self {
        Self {
            device: m.dev(),
            inode: m.ino(),
            size: m.len(),
            modified: m.mtime(),
            modified_ns: m.mtime_nsec(),
            changed: m.ctime(),
            changed_ns: m.ctime_nsec(),
            mode: m.mode(),
            uid: m.uid(),
            gid: m.gid(),
            links: m.nlink(),
            flags: m.st_flags(),
        }
    }
    // Best-effort stamp, deliberately not named a non-ABA or cryptographic token.
    fn validator(&self) -> [u8; 56] {
        let fields = [
            self.device,
            self.inode,
            self.size,
            self.modified as u64,
            self.modified_ns as u64,
            self.changed as u64,
            self.changed_ns as u64,
        ];
        let mut result = [0; 56];
        for (chunk, field) in result.chunks_exact_mut(8).zip(fields) {
            chunk.copy_from_slice(&field.to_le_bytes());
        }
        result
    }
    fn same_inode(&self, other: &Self) -> bool {
        self.device == other.device && self.inode == other.inode
    }
}
struct Entry {
    directory: File,
    name: OsString,
    directory_id: (u64, u64),
    // Holding the inode prevents its identity being reused during this binding.
    current: File,
}
pub(crate) struct Snapshot {
    pub(crate) file: File,
    pub(crate) stamp: Stamp,
    pub(crate) bytes: Vec<u8>,
}
/// One-use baseline observation paired with the engine probe; contains no callback.
pub struct WriteProbe {
    owner: Arc<()>,
    slot: usize,
    snapshot: Snapshot,
}
impl std::fmt::Debug for WriteProbe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WriteProbe(<redacted>)")
    }
}
/// Synchronous adapter completion. Intake of the three receipts is separate from
/// automatic baseline publication. An error here never hides application facts.
#[derive(Debug)]
pub struct WriteCompletion {
    /// Original written payload/version observation, NOT a subsequent fresh
    /// read. Missing after unknown/nonapplication or unavailable attribution.
    pub written_observation: Option<ResourceRead>,
    pub result: SaveEvidence,
    pub no_future_effects: SaveEvidence,
    pub quiescence: SaveEvidence,
    pub failure: Option<FilesystemError>,
    pub cleanup_failed: bool,
}
#[derive(Debug)]
pub struct AnnotationReadCompletion {
    pub receipts: Vec<AnnotationReadEvidence>,
    pub quiescence: AnnotationReadEvidence,
    pub failure: Option<FilesystemError>,
}
pub struct LocalFilesystem {
    // Absent only for the deliberately effect-only daemon export interpreter.
    // Ordinary resource methods fail closed through `provider()`.
    port: Option<ProviderPort>,
    pub(crate) limits: FilesystemLimits,
    owner: Arc<()>,
    entries: Vec<Entry>,
    pub(crate) stage_sequence: u64,
    pub(crate) export_copy_usage: crate::ExportCopyUsageV1,
    #[cfg(test)]
    pub(crate) fault: Option<Fault>,
}
struct ReplacementResult {
    file: File,
    synchronization: Synchronization,
    version: Option<[u8; 56]>,
    failure: Option<FilesystemError>,
}
#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fault {
    BeforeReplace,
    Replacement,
    AfterReplace,
    Synchronization,
    ExportBeforePublication,
    ExportPublication,
    ExportSynchronization,
}
impl LocalFilesystem {
    fn provider(&self) -> Result<&ProviderPort, FilesystemError> {
        self.port.as_ref().ok_or(FilesystemError::Unavailable)
    }
    /// Resolve an existing adapter entry; no read or permission is inferred from
    /// naming it. Admission and launch remain separate engine operations.
    pub fn annotation_read_target(
        &self,
        handle: &ResourceHandle,
    ) -> Result<AnnotationReadTarget, FilesystemError> {
        self.entry(handle)?;
        self.provider()?
            .annotation_read_target(&(handle.slot as u64).to_le_bytes())
            .map_err(Into::into)
    }
    /// Synchronous captured read, not an atomic-resource snapshot. Original bytes
    /// are attributed through the same closed start/chunk/end/failure vocabulary;
    /// separately returned quiescence is never inferred by the engine from EOF.
    pub fn read_annotation(
        &self,
        dispatch: &AnnotationReadDispatch,
        recorder: &mut Recorder,
    ) -> Result<AnnotationReadCompletion, FilesystemError> {
        let started = Instant::now();
        let result = (|| {
            let quiescence = self.provider()?.annotation_read_quiescence(dispatch)?;
            if dispatch.strength() != ReadStrength::CapturedRead {
                return Err(FilesystemError::Resource(ResourceError::Unsupported));
            }
            let mut receipts = Vec::new();
            if receipts.try_reserve_exact(3).is_err() {
                return Ok(AnnotationReadCompletion {
                    receipts,
                    quiescence,
                    failure: Some(FilesystemError::Capacity),
                });
            }
            let read = (|| {
                let key: [u8; 8] = dispatch
                    .resource_key()
                    .try_into()
                    .map_err(|_| FilesystemError::Unavailable)?;
                let slot = usize::try_from(u64::from_le_bytes(key))
                    .map_err(|_| FilesystemError::Unavailable)?;
                let entry = self.entries.get(slot).ok_or(FilesystemError::Unavailable)?;
                let snapshot = snapshot(&entry.directory, &entry.name, self.limits.max_bytes)?;
                let version = snapshot.stamp.validator();
                // Construct every bounded receipt before returning any of them;
                // a producer limit failure emits failure, never truncated EOF.
                let start = self.provider()?.annotation_read_start(dispatch, &version)?;
                let chunk = if snapshot.bytes.is_empty() {
                    None
                } else {
                    Some(
                        self.provider()?
                            .annotation_read_chunk(dispatch, 0, 0, &snapshot.bytes)?,
                    )
                };
                let end = self.provider()?.annotation_read_end(
                    dispatch,
                    &version,
                    usize::from(chunk.is_some()),
                    snapshot.bytes.len(),
                )?;
                receipts.push(start);
                if let Some(chunk) = chunk {
                    receipts.push(chunk);
                }
                receipts.push(end);
                Ok::<(), FilesystemError>(())
            })();
            let failure = read.err();
            if let Some(error) = failure {
                let reason = match error {
                    FilesystemError::Limit
                    | FilesystemError::Capacity
                    | FilesystemError::Resource(ResourceError::Limit) => ReadFailure::Limit,
                    FilesystemError::UnsupportedFile | FilesystemError::UnsupportedMetadata => {
                        ReadFailure::Unsupported
                    }
                    _ => ReadFailure::Unavailable,
                };
                receipts.push(self.provider()?.annotation_read_failure(dispatch, reason)?);
            }
            Ok(AnnotationReadCompletion {
                receipts,
                quiescence,
                failure,
            })
        })();
        recorder.record(Observation {
            boundary: Boundary::SubstrateAnnotationRead,
            disposition: match &result {
                Ok(c) if c.failure.is_none() => Disposition::Completed,
                Ok(_) => Disposition::Failed,
                Err(_) => Disposition::Refused,
            },
            service_time: Some(started.elapsed()),
        });
        result
    }
    /// Port provisioning is an explicit trusted-host choice, not document authority.
    pub fn new(port: ProviderPort, limits: FilesystemLimits) -> Self {
        Self {
            port: Some(port),
            limits,
            owner: Arc::new(()),
            entries: Vec::new(),
            stage_sequence: 0,
            export_copy_usage: crate::ExportCopyUsageV1::default(),
            #[cfg(test)]
            fault: None,
        }
    }

    pub(crate) fn new_export_only(limits: FilesystemLimits) -> Self {
        Self {
            port: None,
            limits,
            owner: Arc::new(()),
            entries: Vec::new(),
            stage_sequence: 0,
            export_copy_usage: crate::ExportCopyUsageV1::default(),
            #[cfg(test)]
            fault: None,
        }
    }
    fn entry(&self, handle: &ResourceHandle) -> Result<&Entry, FilesystemError> {
        if !Arc::ptr_eq(&self.owner, &handle.owner) {
            return Err(FilesystemError::Unavailable);
        }
        self.entries
            .get(handle.slot)
            .ok_or(FilesystemError::Unavailable)
    }
    pub fn open(
        &mut self,
        path: &Path,
        recorder: &mut Recorder,
    ) -> Result<(ResourceHandle, ResourceRead), FilesystemError> {
        let started = Instant::now();
        let result = self.open_inner(path);
        observe(recorder, Boundary::SubstrateRead, started, &result);
        result
    }
    fn open_inner(
        &mut self,
        path: &Path,
    ) -> Result<(ResourceHandle, ResourceRead), FilesystemError> {
        let name = path
            .file_name()
            .ok_or(FilesystemError::UnsupportedFile)?
            .to_owned();
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let directory = File::from(fs::open(
            parent,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )?);
        let m = directory.metadata()?;
        let directory_id = (m.dev(), m.ino());
        let snapshot = snapshot(&directory, &name, self.limits.max_bytes)?;
        let existing = self.entries.iter().position(|entry| {
            (entry.directory_id == directory_id && entry.name == name)
                || entry
                    .current
                    .metadata()
                    .is_ok_and(|m| Stamp::from(&m).same_inode(&snapshot.stamp))
        });
        let slot = existing.unwrap_or(self.entries.len());
        if existing.is_none() && slot >= self.limits.max_resources {
            return Err(FilesystemError::Limit);
        }
        let read = self.provider()?.read_result(
            &(slot as u64).to_le_bytes(),
            &snapshot.stamp.validator(),
            &snapshot.bytes,
        )?;
        if existing.is_none() {
            self.entries
                .try_reserve(1)
                .map_err(|_| FilesystemError::Capacity)?;
            self.entries.push(Entry {
                directory,
                name,
                directory_id,
                current: snapshot.file,
            });
        }
        Ok((
            ResourceHandle {
                owner: self.owner.clone(),
                slot,
            },
            read,
        ))
    }
    pub fn probe(
        &self,
        handle: &ResourceHandle,
        recorder: &mut Recorder,
    ) -> Result<(ResourceRead, WriteProbe), FilesystemError> {
        let started = Instant::now();
        let result = (|| {
            let entry = self.entry(handle)?;
            let snapshot = snapshot(&entry.directory, &entry.name, self.limits.max_bytes)?;
            let read = self.provider()?.read_result(
                &(handle.slot as u64).to_le_bytes(),
                &snapshot.stamp.validator(),
                &snapshot.bytes,
            )?;
            Ok((
                read,
                WriteProbe {
                    owner: self.owner.clone(),
                    slot: handle.slot,
                    snapshot,
                },
            ))
        })();
        observe(recorder, Boundary::SubstrateRead, started, &result);
        result
    }
    /// Reopen both exact anchored entries and compare their current native
    /// identities while retaining both descriptors. This is a best-effort host
    /// observation, not an atomic namespace snapshot or lasting alias guarantee.
    pub fn compare_ordinary_pair(
        &self,
        check: &OrdinaryPairCheck,
        recorder: &mut Recorder,
    ) -> Result<OrdinaryPairEvidence, FilesystemError> {
        let started = Instant::now();
        let result = (|| {
            let (source, annotations) = self.provider()?.ordinary_pair_keys(check)?;
            let read = |key: &[u8]| -> Result<Snapshot, FilesystemError> {
                let key: [u8; 8] = key.try_into().map_err(|_| FilesystemError::Unavailable)?;
                let slot = usize::try_from(u64::from_le_bytes(key))
                    .map_err(|_| FilesystemError::Unavailable)?;
                let entry = self.entries.get(slot).ok_or(FilesystemError::Unavailable)?;
                snapshot(&entry.directory, &entry.name, self.limits.max_bytes)
            };
            let annotations = read(annotations)?;
            let source = source.map(read).transpose()?;
            let relation = match source {
                Some(source) if source.stamp.same_inode(&annotations.stamp) => {
                    ResourceRelation::Equivalent
                }
                Some(_) => ResourceRelation::Distinct,
                None => ResourceRelation::Unknown,
            };
            self.provider()?
                .attest_ordinary_pair(check, relation)
                .map_err(Into::into)
        })();
        observe(
            recorder,
            Boundary::SubstrateIdentityComparison,
            started,
            &result,
        );
        result
    }
    /// Consumes the disclosed dispatch and one-use matching probe. All ordinary
    /// failures after attribution return explicit receipts, including nonapplication.
    pub fn write(
        &mut self,
        dispatch: SaveDispatch,
        probe: WriteProbe,
        recorder: &mut Recorder,
    ) -> Result<WriteCompletion, FilesystemError> {
        self.write_inner(dispatch, probe, false, recorder)
    }
    /// Opt-in original written observation for integration continuity guards.
    /// Ordinary source writes do not pay to copy this additional observation.
    pub fn write_observed(
        &mut self,
        dispatch: SaveDispatch,
        probe: WriteProbe,
        recorder: &mut Recorder,
    ) -> Result<WriteCompletion, FilesystemError> {
        self.write_inner(dispatch, probe, true, recorder)
    }
    fn write_inner(
        &mut self,
        dispatch: SaveDispatch,
        probe: WriteProbe,
        observe_written: bool,
        recorder: &mut Recorder,
    ) -> Result<WriteCompletion, FilesystemError> {
        if !Arc::ptr_eq(&self.owner, &probe.owner)
            || dispatch.resource_key() != (probe.slot as u64).to_le_bytes()
            || probe.slot >= self.entries.len()
        {
            return Err(FilesystemError::Unavailable);
        }
        // Validate original provider attribution BEFORE any physical I/O. These
        // receipts can be prebuilt because they are returned only after this
        // synchronous execution has no remaining write-producing continuation.
        let no_future_effects = self.provider()?.no_future_effects(&dispatch)?;
        let quiescence = self.provider()?.quiescence(&dispatch)?;
        let started = Instant::now();
        let mut cleanup_failed = false;
        let attempt = self.replace(&dispatch, &probe, recorder, &mut cleanup_failed);
        let (application, synchronization, version, failure) = match attempt {
            Ok(replacement) => {
                self.entries[probe.slot].current = replacement.file;
                (
                    Application::Written,
                    replacement.synchronization,
                    replacement.version,
                    replacement.failure,
                )
            }
            Err((application, error)) => (
                application,
                Synchronization::NotRequested,
                None,
                Some(error),
            ),
        };
        let result = self.provider()?.result(
            &dispatch,
            application,
            if application == Application::Written {
                dispatch.bytes().len()
            } else {
                0
            },
            synchronization,
            version.as_ref().map(|v| v.as_slice()),
        )?;
        recorder.record(Observation {
            boundary: Boundary::SubstrateWrite,
            disposition: if failure.is_some() {
                Disposition::Failed
            } else {
                Disposition::Completed
            },
            service_time: Some(started.elapsed()),
        });
        Ok(WriteCompletion {
            written_observation: if observe_written && application == Application::Written {
                version.as_ref().and_then(|version| {
                    self.provider()
                        .ok()?
                        .read_result(dispatch.resource_key(), version, dispatch.bytes())
                        .ok()
                })
            } else {
                None
            },
            result,
            no_future_effects,
            quiescence,
            failure,
            cleanup_failed,
        })
    }
    fn replace(
        &mut self,
        dispatch: &SaveDispatch,
        probe: &WriteProbe,
        recorder: &mut Recorder,
        cleanup_failed: &mut bool,
    ) -> Result<ReplacementResult, (Application, FilesystemError)> {
        let entry = &self.entries[probe.slot];
        let preflight = Instant::now();
        let check = (|| {
            if dispatch.protection() != vellum_engine::resource::Protection::BestEffort
                || dispatch.result_version_profile()
                    == vellum_engine::resource::SaveResultVersionProfileV1::EpochToken
                || dispatch.expected_bytes().is_none()
            {
                return Err(FilesystemError::Resource(ResourceError::Unsupported));
            }
            if Some(probe.snapshot.bytes.as_slice()) != dispatch.expected_bytes()
                || Some(probe.snapshot.stamp.validator().as_slice())
                    != dispatch.expected_validator()
            {
                return Err(FilesystemError::Conflict);
            }
            if dispatch.bytes().len() > self.limits.max_bytes {
                return Err(FilesystemError::Limit);
            }
            plain_metadata(&probe.snapshot.file)?;
            let current = snapshot(&entry.directory, &entry.name, self.limits.max_bytes)?;
            if current.stamp != probe.snapshot.stamp || current.bytes != probe.snapshot.bytes {
                return Err(FilesystemError::Conflict);
            }
            plain_metadata(&current.file)?;
            attributes(&current.file, self.limits.max_metadata_bytes)
        })();
        observe(recorder, Boundary::SubstrateValidation, preflight, &check);
        let expected_attributes = check.map_err(|e| (Application::NotWritten, e))?;
        let staging = Instant::now();
        let mut staged = stage(
            &entry.directory,
            &mut self.stage_sequence,
            self.limits.max_stage_attempts,
        )
        .map_err(|e| (Application::NotWritten, e))?;
        let prepare = (|| {
            staged.file.write_all(dispatch.bytes())?;
            native_acl::copy_attributes(&probe.snapshot.file, &staged.file)?;
            fs::fchmod(
                &staged.file,
                Mode::from_raw_mode((probe.snapshot.stamp.mode & 0o777) as u16),
            )?;
            plain_metadata(&staged.file)?;
            if attributes(&staged.file, self.limits.max_metadata_bytes)? != expected_attributes {
                return Err(FilesystemError::UnsupportedMetadata);
            }
            let meta = Stamp::from(&staged.file.metadata()?);
            if meta.uid != probe.snapshot.stamp.uid
                || meta.gid != probe.snapshot.stamp.gid
                || meta.mode != probe.snapshot.stamp.mode
            {
                return Err(FilesystemError::UnsupportedMetadata);
            }
            // Recheck content, identity, attributes and links immediately before
            // replacement. There remains a documented check/rename race.
            let current = snapshot(&entry.directory, &entry.name, self.limits.max_bytes)?;
            if current.stamp != probe.snapshot.stamp || current.bytes != probe.snapshot.bytes {
                return Err(FilesystemError::Conflict);
            }
            plain_metadata(&current.file)?;
            if attributes(&current.file, self.limits.max_metadata_bytes)? != expected_attributes {
                return Err(FilesystemError::Conflict);
            }
            #[cfg(test)]
            if self.fault == Some(Fault::BeforeReplace) {
                return Err(FilesystemError::Io);
            }
            // Reserve the retained identity handle before replacement, not after
            // an irreversible effect where descriptor exhaustion would hide it.
            staged.file.try_clone().map_err(FilesystemError::from)
        })();
        observe(recorder, Boundary::SubstrateStaging, staging, &prepare);
        let retained_file = match prepare {
            Ok(file) => file,
            Err(error) => {
                *cleanup_failed = staged.cleanup().is_err();
                return Err((Application::NotWritten, error));
            }
        };
        let replacing = Instant::now();
        #[cfg(test)]
        let injected = self.fault == Some(Fault::Replacement);
        #[cfg(not(test))]
        let injected = false;
        let replacement = if injected {
            Err(FilesystemError::Io)
        } else {
            fs::renameat(
                &entry.directory,
                &staged.name,
                &entry.directory,
                &entry.name,
            )
            .map_err(FilesystemError::from)
        };
        observe(
            recorder,
            Boundary::SubstrateReplacement,
            replacing,
            &replacement,
        );
        if let Err(error) = replacement {
            *cleanup_failed = staged.cleanup().is_err();
            // Even a failed rename is not promoted to known nonapplication.
            return Err((Application::Unknown, error));
        }
        staged.linked = false;
        // From here on, failure can NEVER be reported as NotWritten.
        let sync_start = Instant::now();
        let mut failure = None;
        let sync = if dispatch.synchronization_required() {
            #[cfg(test)]
            let injected = self.fault == Some(Fault::Synchronization);
            #[cfg(not(test))]
            let injected = false;
            let result = if injected {
                Err(std::io::Error::other("injected sync failure"))
            } else {
                staged
                    .file
                    .sync_all()
                    .and_then(|()| entry.directory.sync_all())
            };
            observe(
                recorder,
                Boundary::SubstrateSynchronization,
                sync_start,
                &result,
            );
            if result.is_ok() {
                Synchronization::Succeeded
            } else {
                failure = Some(FilesystemError::Io);
                Synchronization::Failed
            }
        } else {
            Synchronization::NotRequested
        };
        #[cfg(test)]
        let injected = self.fault == Some(Fault::AfterReplace);
        #[cfg(not(test))]
        let injected = false;
        let token = if injected {
            failure = Some(FilesystemError::Io);
            None
        } else {
            match staged.file.metadata() {
                Ok(m) => Some(Stamp::from(&m).validator()),
                Err(_) => {
                    failure = Some(FilesystemError::Io);
                    None
                }
            }
        };
        Ok(ReplacementResult {
            file: retained_file,
            synchronization: sync,
            version: token,
            failure,
        })
    }
}

#[cfg(test)]
mod tests;

pub(crate) fn observe<T, E>(
    recorder: &mut Recorder,
    boundary: Boundary,
    started: Instant,
    result: &Result<T, E>,
) {
    recorder.record(Observation {
        boundary,
        disposition: if result.is_ok() {
            Disposition::Completed
        } else {
            Disposition::Refused
        },
        service_time: Some(started.elapsed()),
    });
}
pub(crate) fn snapshot(
    directory: &File,
    name: &OsString,
    limit: usize,
) -> Result<Snapshot, FilesystemError> {
    let mut file = File::from(fs::openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )?);
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(FilesystemError::UnsupportedFile);
    }
    let stamp = Stamp::from(&metadata);
    if stamp.size > limit as u64 {
        return Err(FilesystemError::Limit);
    }
    let capacity = limit.checked_add(1).ok_or(FilesystemError::Limit)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| FilesystemError::Capacity)?;
    bytes.resize(capacity, 0);
    let mut used = 0;
    while used < bytes.len() {
        match file.read(&mut bytes[used..]) {
            Ok(0) => break,
            Ok(n) => used += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }
    if used > limit {
        return Err(FilesystemError::Limit);
    }
    bytes.truncate(used);
    if stamp != Stamp::from(&file.metadata()?) || used as u64 != stamp.size {
        return Err(FilesystemError::Conflict);
    }
    let after = fs::statat(directory, name, AtFlags::SYMLINK_NOFOLLOW)?;
    if after.st_dev as u64 != stamp.device || after.st_ino != stamp.inode {
        return Err(FilesystemError::Conflict);
    }
    file.seek(SeekFrom::Start(0))?;
    Ok(Snapshot { file, stamp, bytes })
}
pub(crate) fn plain_metadata(file: &File) -> Result<(), FilesystemError> {
    let m = file.metadata()?;
    if !m.is_file() || m.nlink() != 1 {
        return Err(FilesystemError::UnsupportedFile);
    }
    // Darwin SDK MNT_LOCAL. Remote filesystems need their own error/identity
    // profile and are not silently treated as the first local mechanism.
    if fs::fstatfs(file)?.f_flags & 0x00001000 == 0 {
        return Err(FilesystemError::UnsupportedFile);
    }
    if m.mode() & 0o7000 != 0 || m.mode() & 0o222 == 0 || m.st_flags() != 0 {
        return Err(FilesystemError::UnsupportedMetadata);
    }
    if !native_acl::is_empty(file)? {
        return Err(FilesystemError::UnsupportedMetadata);
    }
    Ok(())
}
// Read and compare every enumerable name/value, with one aggregate byte budget.
// No best-effort dropping, truncated list or allowlist of supposedly harmless attrs.
pub(crate) fn attributes(
    file: &File,
    limit: usize,
) -> Result<Vec<(CString, Vec<u8>)>, FilesystemError> {
    let size = limit.checked_add(1).ok_or(FilesystemError::Limit)?;
    let mut scratch = Vec::new();
    scratch
        .try_reserve_exact(size)
        .map_err(|_| FilesystemError::Capacity)?;
    scratch.resize(size, 0);
    let length =
        fs::flistxattr(file, &mut scratch[..]).map_err(|_| FilesystemError::UnsupportedMetadata)?;
    if length > limit {
        return Err(FilesystemError::Limit);
    }
    let mut names = Vec::new();
    let mut offset = 0;
    while offset < length {
        let end = scratch[offset..length]
            .iter()
            .position(|b| *b == 0)
            .map(|n| offset + n + 1)
            .ok_or(FilesystemError::UnsupportedMetadata)?;
        let name = CString::from_vec_with_nul(scratch[offset..end].to_vec())
            .map_err(|_| FilesystemError::UnsupportedMetadata)?;
        names.push(name);
        offset = end;
    }
    let mut used = length;
    let mut result = Vec::new();
    for name in names {
        let remaining = limit - used;
        let length = fs::fgetxattr(file, name.as_c_str(), &mut scratch[..remaining + 1])
            .map_err(|_| FilesystemError::UnsupportedMetadata)?;
        if length > remaining {
            return Err(FilesystemError::Limit);
        }
        used += length;
        result.push((name, scratch[..length].to_vec()));
    }
    result.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(result)
}
pub(crate) struct Staged<'a> {
    directory: &'a File,
    pub(crate) name: OsString,
    pub(crate) file: File,
    pub(crate) linked: bool,
}
impl Staged<'_> {
    pub(crate) fn cleanup(&mut self) -> Result<(), FilesystemError> {
        if self.linked {
            fs::unlinkat(self.directory, &self.name, AtFlags::empty())?;
            self.linked = false;
        }
        Ok(())
    }
}
impl Drop for Staged<'_> {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
pub(crate) fn stage<'a>(
    directory: &'a File,
    sequence: &mut u64,
    attempts: u32,
) -> Result<Staged<'a>, FilesystemError> {
    for _ in 0..attempts {
        *sequence = sequence.checked_add(1).ok_or(FilesystemError::Limit)?;
        let name = OsString::from(format!(".vellum-stage-{}-{sequence}", std::process::id()));
        match fs::openat(
            directory,
            &name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o600),
        ) {
            Ok(fd) => {
                return Ok(Staged {
                    directory,
                    name,
                    file: File::from(fd),
                    linked: true,
                });
            }
            Err(rustix::io::Errno::EXIST) => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err(FilesystemError::Limit)
}

// macOS ACLs are not enumerable xattrs. This isolated native seam checks absence
// and copies xattrs to our private stage. It never edits source metadata or ACLs.
#[allow(unsafe_code)]
pub(crate) mod native_acl {
    use super::FilesystemError;
    use std::ffi::{c_int, c_void};
    use std::fs::File;
    use std::os::fd::AsRawFd;
    unsafe extern "C" {
        fn acl_get_fd_np(fd: c_int, kind: c_int) -> *mut c_void;
        fn acl_valid(acl: *mut c_void) -> c_int;
        fn acl_get_entry(acl: *mut c_void, entry_id: c_int, entry: *mut *mut c_void) -> c_int;
        fn acl_free(object: *mut c_void) -> c_int;
        fn fcopyfile(from: c_int, to: c_int, state: *mut c_void, flags: u32) -> c_int;
    }
    pub(crate) fn copy_attributes(from: &File, to: &File) -> Result<(), FilesystemError> {
        // SAFETY: live borrowed descriptors; documented null optional state and
        // COPYFILE_XATTR only. Destination is an exclusively created private stage.
        let status = unsafe {
            fcopyfile(
                from.as_raw_fd(),
                to.as_raw_fd(),
                std::ptr::null_mut(),
                rustix::fs::CopyfileFlags::XATTR.bits(),
            )
        };
        if status == 0 {
            Ok(())
        } else {
            Err(FilesystemError::UnsupportedMetadata)
        }
    }
    pub(crate) fn is_empty(file: &File) -> Result<bool, FilesystemError> {
        // SAFETY: live borrowed descriptor, SDK ACL_TYPE_EXTENDED; returned
        // allocation is private, validated before reading, and freed exactly once.
        let acl = unsafe { acl_get_fd_np(file.as_raw_fd(), 0x100) };
        if acl.is_null() {
            // Descriptor remains valid: Darwin reports absent extended ACL as ENOENT.
            if std::io::Error::last_os_error().raw_os_error()
                == Some(rustix::io::Errno::NOENT.raw_os_error())
            {
                return Ok(true);
            }
            return Err(FilesystemError::UnsupportedMetadata);
        }
        // SAFETY: acl is an owned, live allocation from acl_get_fd_np.
        let valid = unsafe { acl_valid(acl) } == 0;
        let mut entry = std::ptr::null_mut();
        // SAFETY: validated live ACL, SDK ACL_FIRST_ENTRY, writable entry pointer.
        let status = if valid {
            // SAFETY: validated live ACL and writable output pointer, as above.
            unsafe { acl_get_entry(acl, 0, &mut entry) }
        } else {
            -1
        };
        let errno = std::io::Error::last_os_error().raw_os_error();
        // SAFETY: the original nonnull allocation has not been freed or escaped.
        let freed = unsafe { acl_free(acl) } == 0;
        if !valid || !freed {
            return Err(FilesystemError::UnsupportedMetadata);
        }
        if status == 0 {
            return Ok(false);
        }
        // On macOS valid empty ACL + FIRST_ENTRY yields -1/EINVAL (not Linux's 0).
        if status == -1 && errno == Some(rustix::io::Errno::INVAL.raw_os_error()) {
            Ok(true)
        } else {
            Err(FilesystemError::UnsupportedMetadata)
        }
    }
}
