//! Qualified local copy export with no binding effect.

use crate::{
    FilesystemError, LocalFilesystem, Snapshot, attributes, native_acl, observe, plain_metadata,
    snapshot, stage,
};
use rustix::fs::{self, AtFlags, Mode, OFlags, RenameFlags};
use std::fs::File;
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path};
use std::time::Instant;
use vellum_engine::instrumentation::{Boundary, Disposition, Observation, Recorder};
use vellum_host::authority::{SaveAsDestinationObjectV1, SaveAsDestinationRequestV1};
use vellum_host::export_copy::{
    AuthorizedExportCopyV1, ExportCopyDispositionV1, ExportCopyEffectPortV1, ExportCopyEvidenceV1,
    ExportCopyFailureV1, ExportCopyPersistenceV1, ExportCopyReceiptV1,
    FailedExportCopyPersistenceV1,
};

/// Content-free USE/RED counters for the bounded local export substrate.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExportCopyUsageV1 {
    pub attempts: u64,
    pub prepared: u64,
    pub durable: u64,
    pub not_durable: u64,
    pub unknown: u64,
    pub not_attempted: u64,
    pub cleanup_failed: u64,
    pub counter_saturated: bool,
}

/// Standalone local export interpreter for daemon composition.
///
/// It owns only the bounded staging state required by the copy effect; unlike
/// `LocalFilesystem`, it does not manufacture a source-resource provider or
/// inherit any ordinary read/write authority.
pub struct LocalExportCopyEffectV1 {
    filesystem: LocalFilesystem,
}

impl LocalExportCopyEffectV1 {
    pub fn new(limits: crate::FilesystemLimits) -> Self {
        Self {
            filesystem: LocalFilesystem::new_export_only(limits),
        }
    }

    pub fn usage(&self) -> ExportCopyUsageV1 {
        self.filesystem.export_copy_usage()
    }
}

/// Qualified destination state. It is deliberately non-cloneable and redacted.
/// Namespace and content are revalidated immediately before publication.
pub(crate) struct PreparedExportCopyV1 {
    evidence: ExportCopyEvidenceV1,
    bytes: Vec<u8>,
    directory: File,
    name: std::ffi::OsString,
    original: Option<Snapshot>,
}

/// Descriptor-anchored destination installed by the local composition root.
/// The descriptor, rather than the display locator, is the object capability.
struct LocalExportDestinationV1 {
    request: SaveAsDestinationRequestV1,
    directory: File,
    name: std::ffi::OsString,
}

/// Pin the exact parent directory before policy authorization. The returned
/// opaque object is one-use and only this adapter can interpret it.
pub fn install_local_export_destination(
    request: &SaveAsDestinationRequestV1,
) -> Result<SaveAsDestinationObjectV1, FilesystemError> {
    let path = request.destination();
    let name = path
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or(FilesystemError::UnsupportedFile)?
        .to_owned();
    let parent = path.parent().ok_or(FilesystemError::UnsupportedFile)?;
    let directory = open_parent_anchored(parent)?;
    qualify_directory(&directory)?;
    Ok(SaveAsDestinationObjectV1::install(
        request.clone(),
        LocalExportDestinationV1 {
            request: request.clone(),
            directory,
            name,
        },
    ))
}

struct QualifiedDestinationRef<'a> {
    directory: &'a File,
    name: &'a std::ffi::OsString,
    original: Option<&'a Snapshot>,
}

impl std::fmt::Debug for PreparedExportCopyV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PreparedExportCopyV1(<redacted>)")
    }
}

impl LocalFilesystem {
    pub fn export_copy_usage(&self) -> ExportCopyUsageV1 {
        self.export_copy_usage
    }

    /// Qualify the exact policy-authorized destination through an anchored
    /// parent descriptor. This performs no write, stage creation, or rebind.
    pub(crate) fn prepare_export_copy(
        &mut self,
        authorization: AuthorizedExportCopyV1,
        recorder: &mut Recorder,
    ) -> Result<PreparedExportCopyV1, ExportCopyReceiptV1> {
        bump(
            &mut self.export_copy_usage.attempts,
            &mut self.export_copy_usage.counter_saturated,
        );
        let started = Instant::now();
        let result = qualify(authorization, self.limits.max_bytes);
        recorder.record(Observation {
            boundary: Boundary::SubstrateValidation,
            disposition: if result.is_ok() {
                Disposition::Completed
            } else {
                Disposition::Refused
            },
            service_time: Some(started.elapsed()),
        });
        match result {
            Ok(prepared) => {
                bump(
                    &mut self.export_copy_usage.prepared,
                    &mut self.export_copy_usage.counter_saturated,
                );
                Ok(prepared)
            }
            Err((evidence, failure)) => {
                bump(
                    &mut self.export_copy_usage.not_attempted,
                    &mut self.export_copy_usage.counter_saturated,
                );
                Err(ExportCopyReceiptV1::failed(
                    evidence,
                    FailedExportCopyPersistenceV1::NotAttempted,
                    failure,
                    false,
                ))
            }
        }
    }

    /// Publish an already-qualified copy. Exclusive creation uses the kernel's
    /// no-replace rename primitive; overwrite repeats the ordinary writer's
    /// content, identity, ownership, link, flag, ACL, and xattr checks.
    pub(crate) fn service_export_copy(
        &mut self,
        prepared: PreparedExportCopyV1,
        recorder: &mut Recorder,
    ) -> ExportCopyReceiptV1 {
        let PreparedExportCopyV1 {
            evidence,
            bytes,
            directory,
            name,
            original,
        } = prepared;
        let mut cleanup_failed = false;
        let outcome = self.export_copy_inner(
            evidence.disposition(),
            &bytes,
            QualifiedDestinationRef {
                directory: &directory,
                name: &name,
                original: original.as_ref(),
            },
            recorder,
            &mut cleanup_failed,
        );
        let (persistence, failure) = match outcome {
            Ok(()) => (ExportCopyPersistenceV1::Durable, None),
            Err((persistence, failure)) => (persistence, Some(failure)),
        };
        let counter = match persistence {
            ExportCopyPersistenceV1::Durable => &mut self.export_copy_usage.durable,
            ExportCopyPersistenceV1::NotDurable => &mut self.export_copy_usage.not_durable,
            ExportCopyPersistenceV1::Unknown => &mut self.export_copy_usage.unknown,
            ExportCopyPersistenceV1::NotAttempted => &mut self.export_copy_usage.not_attempted,
        };
        bump(counter, &mut self.export_copy_usage.counter_saturated);
        if cleanup_failed {
            bump(
                &mut self.export_copy_usage.cleanup_failed,
                &mut self.export_copy_usage.counter_saturated,
            );
        }
        match failure {
            None => ExportCopyReceiptV1::durable(evidence),
            Some(failure) => ExportCopyReceiptV1::failed(
                evidence,
                match persistence {
                    ExportCopyPersistenceV1::NotDurable => {
                        FailedExportCopyPersistenceV1::NotDurable
                    }
                    ExportCopyPersistenceV1::Unknown => FailedExportCopyPersistenceV1::Unknown,
                    ExportCopyPersistenceV1::NotAttempted => {
                        FailedExportCopyPersistenceV1::NotAttempted
                    }
                    ExportCopyPersistenceV1::Durable => unreachable!("failure cannot be durable"),
                },
                failure,
                cleanup_failed,
            ),
        }
    }

    fn export_copy_inner(
        &mut self,
        disposition: ExportCopyDispositionV1,
        bytes: &[u8],
        destination: QualifiedDestinationRef<'_>,
        recorder: &mut Recorder,
        cleanup_failed: &mut bool,
    ) -> Result<(), (ExportCopyPersistenceV1, ExportCopyFailureV1)> {
        let QualifiedDestinationRef {
            directory,
            name,
            original,
        } = destination;
        if bytes.len() > self.limits.max_bytes {
            return Err((
                ExportCopyPersistenceV1::NotAttempted,
                ExportCopyFailureV1::Limit,
            ));
        }
        let staging_started = Instant::now();
        let mut staged = stage(
            directory,
            &mut self.stage_sequence,
            self.limits.max_stage_attempts,
        )
        .map_err(|error| (ExportCopyPersistenceV1::NotAttempted, classify(error)))?;
        let staging = (|| {
            staged.file.write_all(bytes)?;
            match (disposition, original) {
                (ExportCopyDispositionV1::OverwriteExisting, Some(original)) => {
                    let expected_attributes =
                        attributes(&original.file, self.limits.max_metadata_bytes)?;
                    native_acl::copy_attributes(&original.file, &staged.file)?;
                    fs::fchmod(
                        &staged.file,
                        Mode::from_raw_mode((original.stamp.mode & 0o777) as u16),
                    )?;
                    plain_metadata(&staged.file)?;
                    let metadata = staged.file.metadata()?;
                    if metadata.uid() != original.stamp.uid
                        || metadata.gid() != original.stamp.gid
                        || metadata.mode() != original.stamp.mode
                        || attributes(&staged.file, self.limits.max_metadata_bytes)?
                            != expected_attributes
                    {
                        return Err(FilesystemError::UnsupportedMetadata);
                    }
                }
                (ExportCopyDispositionV1::CreateExclusive, None) => {
                    plain_new_file(&staged.file)?;
                }
                _ => return Err(FilesystemError::Conflict),
            }
            synchronize(&staged.file)?;
            revalidate(
                directory,
                name,
                disposition,
                original,
                self.limits.max_bytes,
                self.limits.max_metadata_bytes,
            )?;
            #[cfg(test)]
            if self.fault == Some(crate::Fault::ExportBeforePublication) {
                return Err(FilesystemError::Io);
            }
            Ok::<(), FilesystemError>(())
        })();
        observe(
            recorder,
            Boundary::SubstrateStaging,
            staging_started,
            &staging,
        );
        if let Err(error) = staging {
            *cleanup_failed = staged.cleanup().is_err();
            return Err((ExportCopyPersistenceV1::NotAttempted, classify(error)));
        }

        let publication_started = Instant::now();
        #[cfg(test)]
        let injected = self.fault == Some(crate::Fault::ExportPublication);
        #[cfg(not(test))]
        let injected = false;
        let mut exclusive_collision = false;
        let publication = if injected {
            Err(FilesystemError::Io)
        } else {
            match disposition {
                ExportCopyDispositionV1::OverwriteExisting => {
                    fs::renameat(directory, &staged.name, directory, name)
                        .map_err(FilesystemError::from)
                }
                ExportCopyDispositionV1::CreateExclusive => {
                    match fs::renameat_with(
                        directory,
                        &staged.name,
                        directory,
                        name,
                        RenameFlags::NOREPLACE,
                    ) {
                        Ok(()) => Ok(()),
                        Err(rustix::io::Errno::EXIST) => {
                            exclusive_collision = true;
                            Err(FilesystemError::Conflict)
                        }
                        Err(error) => Err(error.into()),
                    }
                }
            }
        };
        observe(
            recorder,
            Boundary::SubstrateReplacement,
            publication_started,
            &publication,
        );
        if let Err(error) = publication {
            *cleanup_failed = staged.cleanup().is_err();
            // Except for a known no-replace collision, failed namespace
            // publication is conservatively unknown, matching ordinary writes.
            let persistence = if exclusive_collision {
                ExportCopyPersistenceV1::NotAttempted
            } else {
                ExportCopyPersistenceV1::Unknown
            };
            return Err((persistence, classify(error)));
        }
        staged.linked = false;

        let synchronization_started = Instant::now();
        #[cfg(test)]
        let injected = self.fault == Some(crate::Fault::ExportSynchronization);
        #[cfg(not(test))]
        let injected = false;
        let synchronization = if injected {
            Err(FilesystemError::Io)
        } else {
            synchronize(directory)
        };
        recorder.record(Observation {
            boundary: Boundary::SubstrateSynchronization,
            disposition: if synchronization.is_ok() {
                Disposition::Completed
            } else {
                Disposition::Failed
            },
            service_time: Some(synchronization_started.elapsed()),
        });
        synchronization.map_err(|error| (ExportCopyPersistenceV1::Unknown, classify(error)))
    }
}

impl ExportCopyEffectPortV1 for LocalFilesystem {
    fn execute_export_copy(
        &mut self,
        authorization: AuthorizedExportCopyV1,
        recorder: &mut Recorder,
    ) -> ExportCopyReceiptV1 {
        match self.prepare_export_copy(authorization, recorder) {
            Ok(prepared) => self.service_export_copy(prepared, recorder),
            Err(receipt) => receipt,
        }
    }
}

impl ExportCopyEffectPortV1 for LocalExportCopyEffectV1 {
    fn execute_export_copy(
        &mut self,
        authorization: AuthorizedExportCopyV1,
        recorder: &mut Recorder,
    ) -> ExportCopyReceiptV1 {
        self.filesystem.execute_export_copy(authorization, recorder)
    }
}

fn qualify(
    authorization: AuthorizedExportCopyV1,
    max_bytes: usize,
) -> Result<PreparedExportCopyV1, (ExportCopyEvidenceV1, ExportCopyFailureV1)> {
    let (evidence, destination, bytes, object) = authorization.into_effect_parts();
    let result = (|| {
        let path = destination.destination();
        let name = path
            .file_name()
            .filter(|name| !name.is_empty())
            .ok_or(FilesystemError::UnsupportedFile)?
            .to_owned();
        let object = object.ok_or(FilesystemError::UnsupportedFile)?;
        let LocalExportDestinationV1 {
            request: pinned_request,
            directory,
            name: pinned_name,
        } = object
            .downcast::<LocalExportDestinationV1>()
            .map_err(|_| FilesystemError::UnsupportedFile)?;
        if destination != pinned_request || name != pinned_name {
            return Err(FilesystemError::Conflict);
        }
        qualify_directory(&directory)?;
        let original = match evidence.disposition() {
            ExportCopyDispositionV1::OverwriteExisting => {
                let snapshot = snapshot(&directory, &name, max_bytes)?;
                qualify_existing(&snapshot.file)?;
                Some(snapshot)
            }
            ExportCopyDispositionV1::CreateExclusive => {
                match fs::statat(&directory, &name, AtFlags::SYMLINK_NOFOLLOW) {
                    Err(rustix::io::Errno::NOENT) => None,
                    Ok(_) => return Err(FilesystemError::Conflict),
                    Err(error) => return Err(error.into()),
                }
            }
        };
        Ok((directory, name, original))
    })();
    match result {
        Ok((directory, name, original)) => Ok(PreparedExportCopyV1 {
            evidence,
            bytes,
            directory,
            name,
            original,
        }),
        Err(error) => Err((evidence, classify(error))),
    }
}

fn qualify_directory(directory: &File) -> Result<(), FilesystemError> {
    let metadata = directory.metadata()?;
    if !metadata.is_dir()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || fs::fstatfs(directory)?.f_flags & 0x00001000 == 0
    {
        return Err(FilesystemError::UnsupportedFile);
    }
    Ok(())
}

/// Resolve every parent component under a descriptor with NOFOLLOW. Parent
/// traversal and platform prefixes are outside this local profile.
fn open_parent_anchored(parent: &Path) -> Result<File, FilesystemError> {
    let mut components = parent.components();
    let mut directory = match components.clone().next() {
        Some(Component::RootDir) => {
            components.next();
            File::from(fs::open(
                Path::new("/"),
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )?)
        }
        Some(Component::Prefix(_)) | Some(Component::ParentDir) => {
            return Err(FilesystemError::UnsupportedFile);
        }
        _ => File::from(fs::open(
            Path::new("."),
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )?),
    };
    for component in components {
        match component {
            Component::CurDir | Component::RootDir => {}
            Component::Normal(name) => {
                directory = File::from(fs::openat(
                    &directory,
                    name,
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )?);
            }
            Component::ParentDir | Component::Prefix(_) => {
                return Err(FilesystemError::UnsupportedFile);
            }
        }
    }
    Ok(directory)
}

fn qualify_existing(file: &File) -> Result<(), FilesystemError> {
    plain_metadata(file)?;
    if file.metadata()?.uid() != rustix::process::geteuid().as_raw() {
        return Err(FilesystemError::UnsupportedFile);
    }
    Ok(())
}

fn plain_new_file(file: &File) -> Result<(), FilesystemError> {
    plain_metadata(file)?;
    let metadata = file.metadata()?;
    if metadata.uid() != rustix::process::geteuid().as_raw() || metadata.mode() & 0o777 != 0o600 {
        return Err(FilesystemError::UnsupportedMetadata);
    }
    Ok(())
}

fn revalidate(
    directory: &File,
    name: &std::ffi::OsString,
    disposition: ExportCopyDispositionV1,
    original: Option<&Snapshot>,
    max_bytes: usize,
    max_metadata_bytes: usize,
) -> Result<(), FilesystemError> {
    qualify_directory(directory)?;
    match (disposition, original) {
        (ExportCopyDispositionV1::OverwriteExisting, Some(original)) => {
            let current = snapshot(directory, name, max_bytes)?;
            qualify_existing(&current.file)?;
            if current.stamp != original.stamp || current.bytes != original.bytes {
                return Err(FilesystemError::Conflict);
            }
            if attributes(&current.file, max_metadata_bytes)?
                != attributes(&original.file, max_metadata_bytes)?
            {
                return Err(FilesystemError::Conflict);
            }
            Ok(())
        }
        (ExportCopyDispositionV1::CreateExclusive, None) => {
            match fs::statat(directory, name, AtFlags::SYMLINK_NOFOLLOW) {
                Err(rustix::io::Errno::NOENT) => Ok(()),
                Ok(_) => Err(FilesystemError::Conflict),
                Err(error) => Err(error.into()),
            }
        }
        _ => Err(FilesystemError::Conflict),
    }
}

fn synchronize(file: &File) -> Result<(), FilesystemError> {
    fs::fcntl_fullfsync(file).map_err(FilesystemError::from)
}

fn classify(error: FilesystemError) -> ExportCopyFailureV1 {
    match error {
        FilesystemError::Conflict => ExportCopyFailureV1::Conflict,
        FilesystemError::Limit
        | FilesystemError::Resource(vellum_engine::resource::ResourceError::Limit) => {
            ExportCopyFailureV1::Limit
        }
        FilesystemError::Capacity => ExportCopyFailureV1::Capacity,
        FilesystemError::UnsupportedMetadata
        | FilesystemError::UnsupportedFile
        | FilesystemError::Resource(_) => ExportCopyFailureV1::Unsupported,
        FilesystemError::Io | FilesystemError::Unavailable => ExportCopyFailureV1::Io,
    }
}

fn bump(counter: &mut u64, saturated: &mut bool) {
    match counter.checked_add(1) {
        Some(value) => *counter = value,
        None => *saturated = true,
    }
}
