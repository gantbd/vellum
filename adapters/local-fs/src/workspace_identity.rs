//! Private, bounded persistence for restart-stable workspace protocol identity.
//!
//! The record contains exact locator bytes and protocol identity metadata but
//! no source text. The caller supplies a private (0700), same-user directory. Updates use a pinned
//! directory descriptor, an exclusive 0600 stage, file synchronization,
//! atomic rename, and directory synchronization. A corrupt or unsafe record is
//! never treated as absence and no repair policy is inferred.

use rustix::fs::{self, AtFlags, FileType, Mode, OFlags};
use std::{
    fs::File,
    io::{Read, Write},
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};
use vellum_host::workspace::{
    WorkspaceDocumentBindingV1, WorkspaceIdentityAllocationV1, WorkspaceIdentityLimitsV1,
    WorkspaceIdentityMetadataPortV1, WorkspaceIdentityMetadataV1,
    WorkspaceIdentityRecoveryRefusalV1, WorkspaceIdentityRecoveryV1, WorkspaceRootBindingKindV1,
    WorkspaceRootBindingV1,
};

const RECORD: &str = "workspace-identity-v1";
const STAGE: &str = ".workspace-identity-v1.stage";
const LEGACY_V1_MAGIC: &[u8; 8] = b"VLMID01\0";
const LEGACY_V2_MAGIC: &[u8; 8] = b"VLMID02\0";
const MAGIC: &[u8; 8] = b"VLMID03\0";
const HEADER_BYTES: usize = 72;
const BINDING_FIXED_BYTES: usize = 29;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkspaceIdentityMetadataUsageV1 {
    pub load_attempts: u64,
    pub recovered: u64,
    pub load_refused: u64,
    pub store_attempts: u64,
    pub committed: u64,
    pub store_refused: u64,
    pub outcome_unknown: u64,
    pub counter_saturated: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocalWorkspaceIdentityMetadataV1 {
    usage: WorkspaceIdentityMetadataUsageV1,
}

impl LocalWorkspaceIdentityMetadataV1 {
    pub fn usage(&self) -> WorkspaceIdentityMetadataUsageV1 {
        self.usage
    }
}

impl WorkspaceIdentityMetadataPortV1 for LocalWorkspaceIdentityMetadataV1 {
    fn load(
        &mut self,
        directory: &Path,
        limits: WorkspaceIdentityLimitsV1,
    ) -> WorkspaceIdentityRecoveryV1 {
        bump(
            &mut self.usage.load_attempts,
            &mut self.usage.counter_saturated,
        );
        match load(directory, limits) {
            Ok(metadata) => {
                bump(&mut self.usage.recovered, &mut self.usage.counter_saturated);
                WorkspaceIdentityRecoveryV1::Recovered(metadata)
            }
            Err(reason) => {
                bump(
                    &mut self.usage.load_refused,
                    &mut self.usage.counter_saturated,
                );
                WorkspaceIdentityRecoveryV1::Refused(reason)
            }
        }
    }

    fn store(
        &mut self,
        directory: &Path,
        metadata: &WorkspaceIdentityMetadataV1,
        limits: WorkspaceIdentityLimitsV1,
    ) -> Result<(), WorkspaceIdentityRecoveryRefusalV1> {
        bump(
            &mut self.usage.store_attempts,
            &mut self.usage.counter_saturated,
        );
        let result = store(directory, metadata, limits);
        match result {
            Ok(()) => bump(&mut self.usage.committed, &mut self.usage.counter_saturated),
            Err(WorkspaceIdentityRecoveryRefusalV1::OutcomeUnknown) => bump(
                &mut self.usage.outcome_unknown,
                &mut self.usage.counter_saturated,
            ),
            Err(_) => bump(
                &mut self.usage.store_refused,
                &mut self.usage.counter_saturated,
            ),
        }
        result
    }
}

fn load(
    directory: &Path,
    limits: WorkspaceIdentityLimitsV1,
) -> Result<WorkspaceIdentityMetadataV1, WorkspaceIdentityRecoveryRefusalV1> {
    validate_limits(limits)?;
    let (directory_descriptor, directory_identity) = open_private_directory(directory)?;
    let descriptor = match fs::openat(
        &directory_descriptor,
        RECORD,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::empty(),
    ) {
        Ok(descriptor) => descriptor,
        Err(rustix::io::Errno::NOENT) => {
            revalidate_directory(directory, directory_identity)
                .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Conflict)?;
            return Err(WorkspaceIdentityRecoveryRefusalV1::Missing);
        }
        Err(_) => return Err(WorkspaceIdentityRecoveryRefusalV1::Io),
    };
    let file = File::from(descriptor);
    validate_private_file(&file)?;
    let length = usize::try_from(file.metadata().map_err(io)?.len())
        .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Limit)?;
    if length > limits.max_encoded_bytes {
        return Err(WorkspaceIdentityRecoveryRefusalV1::Limit);
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Limit)?;
    file.take((limits.max_encoded_bytes as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() != length {
        return Err(WorkspaceIdentityRecoveryRefusalV1::Conflict);
    }
    let metadata = decode(&bytes, limits)?;
    revalidate_directory(directory, directory_identity)
        .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Conflict)?;
    Ok(metadata)
}

fn store(
    directory: &Path,
    metadata: &WorkspaceIdentityMetadataV1,
    limits: WorkspaceIdentityLimitsV1,
) -> Result<(), WorkspaceIdentityRecoveryRefusalV1> {
    validate_limits(limits)?;
    let bytes = encode(metadata, limits)?;
    let directory_path = directory;
    let (directory, directory_identity) = open_private_directory(directory_path)?;

    let prior = match read_from_directory(&directory, limits) {
        Ok(existing) => {
            let existing_bytes = encode(&existing, limits)?;
            if existing.generation() == metadata.generation() && existing_bytes == bytes {
                revalidate_directory(directory_path, directory_identity)
                    .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Conflict)?;
                return Ok(());
            }
            if existing.workspace() != metadata.workspace()
                || existing
                    .generation()
                    .checked_add(1)
                    .filter(|next| *next == metadata.generation())
                    .is_none()
            {
                return Err(WorkspaceIdentityRecoveryRefusalV1::Conflict);
            }
            validate_transition(&existing, metadata)?;
            Some(existing_bytes)
        }
        Err(WorkspaceIdentityRecoveryRefusalV1::Missing) if metadata.generation() == 1 => {
            validate_initial(metadata)?;
            None
        }
        Err(reason) => return Err(reason),
    };

    let descriptor = fs::openat(
        &directory,
        STAGE,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::from_raw_mode(0o600),
    )
    .map_err(|error| match error {
        rustix::io::Errno::EXIST => WorkspaceIdentityRecoveryRefusalV1::Conflict,
        _ => WorkspaceIdentityRecoveryRefusalV1::Io,
    })?;
    let mut stage = Stage {
        directory: &directory,
        file: File::from(descriptor),
        linked: true,
    };
    validate_private_file(&stage.file)?;
    stage.file.write_all(&bytes).map_err(io)?;
    synchronize(&stage.file)?;
    let current = match read_from_directory(&directory, limits) {
        Ok(metadata) => Some(encode(&metadata, limits)?),
        Err(WorkspaceIdentityRecoveryRefusalV1::Missing) => None,
        Err(reason) => return Err(reason),
    };
    if current != prior {
        return Err(WorkspaceIdentityRecoveryRefusalV1::Conflict);
    }
    fs::renameat(&directory, STAGE, &directory, RECORD).map_err(io)?;
    stage.linked = false;
    directory
        .sync_all()
        .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::OutcomeUnknown)?;
    revalidate_directory(directory_path, directory_identity)
        .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::OutcomeUnknown)?;
    Ok(())
}

fn read_from_directory(
    directory: &File,
    limits: WorkspaceIdentityLimitsV1,
) -> Result<WorkspaceIdentityMetadataV1, WorkspaceIdentityRecoveryRefusalV1> {
    let descriptor = match fs::openat(
        directory,
        RECORD,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::empty(),
    ) {
        Ok(descriptor) => descriptor,
        Err(rustix::io::Errno::NOENT) => {
            return Err(WorkspaceIdentityRecoveryRefusalV1::Missing);
        }
        Err(_) => return Err(WorkspaceIdentityRecoveryRefusalV1::Io),
    };
    let mut file = File::from(descriptor);
    validate_private_file(&file)?;
    let length = usize::try_from(file.metadata().map_err(io)?.len())
        .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Limit)?;
    if length > limits.max_encoded_bytes {
        return Err(WorkspaceIdentityRecoveryRefusalV1::Limit);
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Limit)?;
    file.read_to_end(&mut bytes).map_err(io)?;
    if bytes.len() != length {
        return Err(WorkspaceIdentityRecoveryRefusalV1::Conflict);
    }
    decode(&bytes, limits)
}

fn validate_limits(
    limits: WorkspaceIdentityLimitsV1,
) -> Result<(), WorkspaceIdentityRecoveryRefusalV1> {
    if limits.max_documents == 0 || limits.max_roots == 0 || limits.max_encoded_bytes < HEADER_BYTES
    {
        return Err(WorkspaceIdentityRecoveryRefusalV1::InvalidLimits);
    }
    Ok(())
}

fn validate_initial(
    candidate: &WorkspaceIdentityMetadataV1,
) -> Result<(), WorkspaceIdentityRecoveryRefusalV1> {
    validate_issued_tail(
        0,
        candidate.document_issued_through(),
        candidate
            .document_bindings()
            .iter()
            .map(WorkspaceDocumentBindingV1::ordinal),
    )?;
    validate_issued_tail(
        0,
        candidate.root_issued_through(),
        candidate
            .root_bindings()
            .iter()
            .map(WorkspaceRootBindingV1::ordinal),
    )
}

fn validate_transition(
    prior: &WorkspaceIdentityMetadataV1,
    candidate: &WorkspaceIdentityMetadataV1,
) -> Result<(), WorkspaceIdentityRecoveryRefusalV1> {
    if candidate.document_issued_through() < prior.document_issued_through()
        || candidate.root_issued_through() < prior.root_issued_through()
        || candidate.document_reserved_through() < prior.document_reserved_through()
        || candidate.root_reserved_through() < prior.root_reserved_through()
    {
        return Err(WorkspaceIdentityRecoveryRefusalV1::Conflict);
    }
    if candidate.document_bindings().iter().any(|binding| {
        binding.ordinal() <= prior.document_issued_through()
            && !prior
                .document_bindings()
                .iter()
                .any(|old| old.ordinal() == binding.ordinal() && old.locator() == binding.locator())
    }) || candidate.root_bindings().iter().any(|binding| {
        binding.ordinal() <= prior.root_issued_through()
            && !prior.root_bindings().iter().any(|old| {
                old.ordinal() == binding.ordinal()
                    && old.kind() == binding.kind()
                    && old.locator() == binding.locator()
            })
    }) {
        return Err(WorkspaceIdentityRecoveryRefusalV1::Conflict);
    }
    validate_issued_tail(
        prior.document_issued_through(),
        candidate.document_issued_through(),
        candidate
            .document_bindings()
            .iter()
            .map(WorkspaceDocumentBindingV1::ordinal),
    )?;
    validate_issued_tail(
        prior.root_issued_through(),
        candidate.root_issued_through(),
        candidate
            .root_bindings()
            .iter()
            .map(WorkspaceRootBindingV1::ordinal),
    )
}

fn validate_issued_tail(
    prior: u64,
    candidate: u64,
    ordinals: impl Iterator<Item = u64>,
) -> Result<(), WorkspaceIdentityRecoveryRefusalV1> {
    let advanced = candidate
        .checked_sub(prior)
        .ok_or(WorkspaceIdentityRecoveryRefusalV1::Conflict)?;
    let present = ordinals
        .filter(|ordinal| *ordinal > prior && *ordinal <= candidate)
        .count();
    if u64::try_from(present).ok() == Some(advanced) {
        Ok(())
    } else {
        Err(WorkspaceIdentityRecoveryRefusalV1::Conflict)
    }
}

fn encode(
    metadata: &WorkspaceIdentityMetadataV1,
    limits: WorkspaceIdentityLimitsV1,
) -> Result<Vec<u8>, WorkspaceIdentityRecoveryRefusalV1> {
    validate_limits(limits)?;
    let mut encoded_len = HEADER_BYTES;
    for locator in metadata
        .document_bindings()
        .iter()
        .map(|value| value.locator())
        .chain(metadata.root_bindings().iter().map(|value| value.locator()))
    {
        encoded_len = encoded_len
            .checked_add(BINDING_FIXED_BYTES)
            .and_then(|value| value.checked_add(locator.as_os_str().as_bytes().len()))
            .ok_or(WorkspaceIdentityRecoveryRefusalV1::Limit)?;
    }
    if encoded_len > limits.max_encoded_bytes {
        return Err(WorkspaceIdentityRecoveryRefusalV1::Limit);
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(encoded_len)
        .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Limit)?;
    bytes.extend_from_slice(MAGIC);
    for value in [
        metadata.workspace(),
        metadata.generation(),
        metadata.document_issued_through(),
        metadata.document_reserved_through(),
        metadata.root_issued_through(),
        metadata.root_reserved_through(),
    ] {
        bytes.extend_from_slice(&value.to_be_bytes());
    }
    bytes.extend_from_slice(&(metadata.document_bindings().len() as u64).to_be_bytes());
    bytes.extend_from_slice(&(metadata.root_bindings().len() as u64).to_be_bytes());
    for binding in metadata.document_bindings() {
        encode_binding(&mut bytes, 0, binding.ordinal(), binding.locator())?;
    }
    for binding in metadata.root_bindings() {
        let tag = match binding.kind() {
            WorkspaceRootBindingKindV1::File => 1,
            WorkspaceRootBindingKindV1::Directory => 2,
        };
        encode_binding(&mut bytes, tag, binding.ordinal(), binding.locator())?;
    }
    debug_assert_eq!(bytes.len(), encoded_len);
    Ok(bytes)
}

fn encode_binding(
    bytes: &mut Vec<u8>,
    tag: u8,
    ordinal: u64,
    locator: &Path,
) -> Result<(), WorkspaceIdentityRecoveryRefusalV1> {
    let locator_bytes = locator.as_os_str().as_bytes();
    let locator_len = u32::try_from(locator_bytes.len())
        .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Limit)?;
    let provenance = provenance(locator, tag)?;
    bytes.push(tag);
    bytes.extend_from_slice(&ordinal.to_be_bytes());
    bytes.extend_from_slice(&locator_len.to_be_bytes());
    bytes.extend_from_slice(&provenance.0.to_be_bytes());
    bytes.extend_from_slice(&provenance.1.to_be_bytes());
    bytes.extend_from_slice(locator_bytes);
    Ok(())
}

fn provenance(locator: &Path, tag: u8) -> Result<(u64, u64), WorkspaceIdentityRecoveryRefusalV1> {
    let parent = locator
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = locator
        .file_name()
        .filter(|value| !value.is_empty())
        .ok_or(WorkspaceIdentityRecoveryRefusalV1::UnsupportedResource)?;
    let (directory, parent_identity) = open_directory_identity(parent)?;
    let stat = fs::statat(&directory, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
        if error == rustix::io::Errno::NOENT {
            WorkspaceIdentityRecoveryRefusalV1::Conflict
        } else {
            WorkspaceIdentityRecoveryRefusalV1::Io
        }
    })?;
    let expected = if tag == 2 {
        FileType::from_raw_mode(stat.st_mode) == FileType::Directory
    } else {
        FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile
    };
    if !expected || stat.st_nlink == 0 {
        return Err(WorkspaceIdentityRecoveryRefusalV1::UnsupportedResource);
    }
    // Ordinary save deliberately replaces a file through an atomic rename, so
    // the file inode is not durable identity. The leaf observation is relative
    // to a pinned parent descriptor, then the parent namespace is revalidated.
    let (_, revalidated) = open_directory_identity(parent)?;
    if revalidated != parent_identity {
        return Err(WorkspaceIdentityRecoveryRefusalV1::Conflict);
    }
    Ok((parent_identity.device, parent_identity.inode))
}

fn open_directory_identity(
    path: &Path,
) -> Result<(File, DirectoryIdentity), WorkspaceIdentityRecoveryRefusalV1> {
    let directory = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags((OFlags::NOFOLLOW | OFlags::DIRECTORY | OFlags::CLOEXEC).bits() as i32)
        .open(path)
        .map_err(io)?;
    let metadata = directory.metadata().map_err(io)?;
    if !metadata.is_dir() {
        return Err(WorkspaceIdentityRecoveryRefusalV1::UnsupportedResource);
    }
    Ok((
        directory,
        DirectoryIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
        },
    ))
}

fn decode(
    bytes: &[u8],
    limits: WorkspaceIdentityLimitsV1,
) -> Result<WorkspaceIdentityMetadataV1, WorkspaceIdentityRecoveryRefusalV1> {
    validate_limits(limits)?;
    if bytes.get(..8) == Some(LEGACY_V1_MAGIC) || bytes.get(..8) == Some(LEGACY_V2_MAGIC) {
        return Err(WorkspaceIdentityRecoveryRefusalV1::Malformed);
    }
    if bytes.len() < HEADER_BYTES || &bytes[..8] != MAGIC {
        return Err(WorkspaceIdentityRecoveryRefusalV1::Malformed);
    }
    let workspace = integer(bytes, 8)?;
    let generation = integer(bytes, 16)?;
    let document_issued_through = integer(bytes, 24)?;
    let document_reserved_through = integer(bytes, 32)?;
    let root_issued_through = integer(bytes, 40)?;
    let root_reserved_through = integer(bytes, 48)?;
    let document_count = usize::try_from(integer(bytes, 56)?)
        .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Limit)?;
    let root_count = usize::try_from(integer(bytes, 64)?)
        .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Limit)?;
    if document_count > limits.max_documents || root_count > limits.max_roots {
        return Err(WorkspaceIdentityRecoveryRefusalV1::Limit);
    }
    let mut offset = HEADER_BYTES;
    let mut documents = Vec::new();
    documents
        .try_reserve_exact(document_count)
        .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Limit)?;
    let mut roots = Vec::new();
    roots
        .try_reserve_exact(root_count)
        .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Limit)?;
    for _ in 0..document_count {
        let (tag, ordinal, locator, next) = decode_binding(bytes, offset)?;
        if tag != 0 {
            return Err(WorkspaceIdentityRecoveryRefusalV1::Malformed);
        }
        documents.push(
            WorkspaceDocumentBindingV1::try_new(ordinal, locator)
                .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Malformed)?,
        );
        offset = next;
    }
    for _ in 0..root_count {
        let (tag, ordinal, locator, next) = decode_binding(bytes, offset)?;
        let kind = match tag {
            1 => WorkspaceRootBindingKindV1::File,
            2 => WorkspaceRootBindingKindV1::Directory,
            _ => return Err(WorkspaceIdentityRecoveryRefusalV1::Malformed),
        };
        roots.push(
            WorkspaceRootBindingV1::try_new(ordinal, kind, locator)
                .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Malformed)?,
        );
        offset = next;
    }
    if offset != bytes.len() {
        return Err(WorkspaceIdentityRecoveryRefusalV1::Malformed);
    }
    WorkspaceIdentityMetadataV1::try_new_with_bindings(
        workspace,
        generation,
        WorkspaceIdentityAllocationV1::try_new(
            document_issued_through,
            document_reserved_through,
            root_issued_through,
            root_reserved_through,
        )
        .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Malformed)?,
        documents,
        roots,
        limits,
    )
    .map_err(|error| match error {
        vellum_host::workspace::WorkspaceIdentityValidationErrorV1::InvalidLimits => {
            WorkspaceIdentityRecoveryRefusalV1::InvalidLimits
        }
        _ => WorkspaceIdentityRecoveryRefusalV1::Malformed,
    })
}

fn decode_binding(
    bytes: &[u8],
    offset: usize,
) -> Result<(u8, u64, PathBuf, usize), WorkspaceIdentityRecoveryRefusalV1> {
    let fixed_end = offset
        .checked_add(BINDING_FIXED_BYTES)
        .ok_or(WorkspaceIdentityRecoveryRefusalV1::Limit)?;
    let fixed = bytes
        .get(offset..fixed_end)
        .ok_or(WorkspaceIdentityRecoveryRefusalV1::Malformed)?;
    let tag = fixed[0];
    let ordinal = u64::from_be_bytes(
        fixed[1..9]
            .try_into()
            .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Malformed)?,
    );
    let locator_len = usize::try_from(u32::from_be_bytes(
        fixed[9..13]
            .try_into()
            .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Malformed)?,
    ))
    .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Limit)?;
    let expected_dev = u64::from_be_bytes(
        fixed[13..21]
            .try_into()
            .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Malformed)?,
    );
    let expected_ino = u64::from_be_bytes(
        fixed[21..29]
            .try_into()
            .map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Malformed)?,
    );
    let end = fixed_end
        .checked_add(locator_len)
        .ok_or(WorkspaceIdentityRecoveryRefusalV1::Limit)?;
    let locator = PathBuf::from(std::ffi::OsString::from_vec(
        bytes
            .get(fixed_end..end)
            .ok_or(WorkspaceIdentityRecoveryRefusalV1::Malformed)?
            .to_vec(),
    ));
    if locator.as_os_str().is_empty() {
        return Err(WorkspaceIdentityRecoveryRefusalV1::Malformed);
    }
    let observed = provenance(&locator, tag)?;
    if observed != (expected_dev, expected_ino) {
        return Err(WorkspaceIdentityRecoveryRefusalV1::Conflict);
    }
    Ok((tag, ordinal, locator, end))
}

fn integer(bytes: &[u8], offset: usize) -> Result<u64, WorkspaceIdentityRecoveryRefusalV1> {
    bytes
        .get(offset..offset + 8)
        .and_then(|slice| slice.try_into().ok())
        .map(u64::from_be_bytes)
        .ok_or(WorkspaceIdentityRecoveryRefusalV1::Malformed)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DirectoryIdentity {
    device: u64,
    inode: u64,
}

fn open_private_directory(
    path: &Path,
) -> Result<(File, DirectoryIdentity), WorkspaceIdentityRecoveryRefusalV1> {
    let directory = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags((OFlags::NOFOLLOW | OFlags::DIRECTORY | OFlags::CLOEXEC).bits() as i32)
        .open(path)
        .map_err(io)?;
    let metadata = directory.metadata().map_err(io)?;
    if !metadata.is_dir()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o7777 != 0o700
    {
        return Err(WorkspaceIdentityRecoveryRefusalV1::UnsupportedResource);
    }
    Ok((
        directory,
        DirectoryIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
        },
    ))
}

fn revalidate_directory(
    path: &Path,
    expected: DirectoryIdentity,
) -> Result<(), WorkspaceIdentityRecoveryRefusalV1> {
    let (directory, observed) = open_private_directory(path)?;
    drop(directory);
    if observed == expected {
        Ok(())
    } else {
        Err(WorkspaceIdentityRecoveryRefusalV1::Conflict)
    }
}

fn validate_private_file(file: &File) -> Result<(), WorkspaceIdentityRecoveryRefusalV1> {
    let metadata = file.metadata().map_err(io)?;
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o7777 != 0o600
    {
        return Err(WorkspaceIdentityRecoveryRefusalV1::UnsupportedResource);
    }
    Ok(())
}

fn synchronize(file: &File) -> Result<(), WorkspaceIdentityRecoveryRefusalV1> {
    file.sync_all().map_err(io)?;
    rustix::fs::fcntl_fullfsync(file).map_err(|_| WorkspaceIdentityRecoveryRefusalV1::Io)
}

fn io(_: impl Sized) -> WorkspaceIdentityRecoveryRefusalV1 {
    WorkspaceIdentityRecoveryRefusalV1::Io
}

fn bump(value: &mut u64, saturated: &mut bool) {
    if *value == u64::MAX {
        *saturated = true;
    } else {
        *value += 1;
    }
}

struct Stage<'a> {
    directory: &'a File,
    file: File,
    linked: bool,
}

impl Drop for Stage<'_> {
    fn drop(&mut self) {
        if self.linked {
            let _ = fs::unlinkat(self.directory, STAGE, AtFlags::empty());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "vellum-identity-{}-{}",
                std::process::id(),
                std::thread::current().name().unwrap_or("case")
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn limits() -> WorkspaceIdentityLimitsV1 {
        WorkspaceIdentityLimitsV1 {
            max_documents: 4,
            max_roots: 4,
            max_encoded_bytes: 1024,
        }
    }

    fn allocation(
        document_issued: u64,
        document_reserved: u64,
        root_issued: u64,
        root_reserved: u64,
    ) -> WorkspaceIdentityAllocationV1 {
        WorkspaceIdentityAllocationV1::try_new(
            document_issued,
            document_reserved,
            root_issued,
            root_reserved,
        )
        .unwrap()
    }

    fn metadata(generation: u64) -> WorkspaceIdentityMetadataV1 {
        WorkspaceIdentityMetadataV1::try_new(7, generation, allocation(0, 13, 0, 17), limits())
            .unwrap()
    }

    fn bound_metadata(generation: u64, locator: PathBuf) -> WorkspaceIdentityMetadataV1 {
        WorkspaceIdentityMetadataV1::try_new_with_bindings(
            7,
            generation,
            allocation(1, 13, 1, 17),
            vec![WorkspaceDocumentBindingV1::try_new(1, locator.clone()).unwrap()],
            vec![
                WorkspaceRootBindingV1::try_new(1, WorkspaceRootBindingKindV1::File, locator)
                    .unwrap(),
            ],
            limits(),
        )
        .unwrap()
    }

    #[test]
    fn given_missing_then_store_and_restart_load_preserve_exact_identity() {
        let fixture = Fixture::new();
        let mut adapter = LocalWorkspaceIdentityMetadataV1::default();
        assert_eq!(
            adapter.load(&fixture.0, limits()),
            WorkspaceIdentityRecoveryV1::Refused(WorkspaceIdentityRecoveryRefusalV1::Missing)
        );
        adapter.store(&fixture.0, &metadata(1), limits()).unwrap();
        let mut restarted = LocalWorkspaceIdentityMetadataV1::default();
        assert_eq!(
            restarted.load(&fixture.0, limits()),
            WorkspaceIdentityRecoveryV1::Recovered(metadata(1))
        );
        assert_eq!(restarted.usage().recovered, 1);
    }

    #[test]
    fn bound_resource_requires_exact_locator_provenance_after_restart() {
        let fixture = Fixture::new();
        let locator = fixture.0.join("document.txt");
        fs::write(&locator, b"one").unwrap();
        let mut adapter = LocalWorkspaceIdentityMetadataV1::default();
        let metadata = bound_metadata(1, locator.clone());
        adapter.store(&fixture.0, &metadata, limits()).unwrap();
        assert_eq!(
            adapter.load(&fixture.0, limits()),
            WorkspaceIdentityRecoveryV1::Recovered(metadata.clone())
        );

        fs::rename(&locator, fixture.0.join("old-document.txt")).unwrap();
        fs::write(&locator, b"replacement").unwrap();
        assert_eq!(
            adapter.load(&fixture.0, limits()),
            WorkspaceIdentityRecoveryV1::Recovered(metadata.clone())
        );
        fs::rename(&locator, fixture.0.join("renamed-document.txt")).unwrap();
        assert_eq!(
            adapter.load(&fixture.0, limits()),
            WorkspaceIdentityRecoveryV1::Refused(WorkspaceIdentityRecoveryRefusalV1::Conflict)
        );
    }

    #[test]
    fn stale_generation_and_corruption_fail_closed_without_identity_evidence() {
        let fixture = Fixture::new();
        let mut adapter = LocalWorkspaceIdentityMetadataV1::default();
        adapter.store(&fixture.0, &metadata(1), limits()).unwrap();
        assert_eq!(
            adapter.store(&fixture.0, &metadata(3), limits()),
            Err(WorkspaceIdentityRecoveryRefusalV1::Conflict)
        );
        fs::write(fixture.0.join(RECORD), b"corrupt").unwrap();
        fs::set_permissions(fixture.0.join(RECORD), fs::Permissions::from_mode(0o600)).unwrap();
        let refusal = adapter.load(&fixture.0, limits());
        assert_eq!(
            refusal,
            WorkspaceIdentityRecoveryV1::Refused(WorkspaceIdentityRecoveryRefusalV1::Malformed)
        );
        assert!(!format!("{refusal:?}").contains(fixture.0.to_string_lossy().as_ref()));
    }

    #[test]
    fn symlink_record_and_unsafe_directory_are_not_treated_as_missing() {
        let fixture = Fixture::new();
        std::os::unix::fs::symlink("outside", fixture.0.join(RECORD)).unwrap();
        let mut adapter = LocalWorkspaceIdentityMetadataV1::default();
        assert_eq!(
            adapter.load(&fixture.0, limits()),
            WorkspaceIdentityRecoveryV1::Refused(WorkspaceIdentityRecoveryRefusalV1::Io)
        );
        fs::remove_file(fixture.0.join(RECORD)).unwrap();
        fs::set_permissions(&fixture.0, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            adapter.load(&fixture.0, limits()),
            WorkspaceIdentityRecoveryV1::Refused(
                WorkspaceIdentityRecoveryRefusalV1::UnsupportedResource
            )
        );
    }

    #[test]
    fn pinned_directory_identity_refuses_detached_namespace_completion() {
        let fixture = Fixture::new();
        let moved = fixture.0.with_extension("moved");
        let _ = fs::remove_dir_all(&moved);
        let (descriptor, identity) = open_private_directory(&fixture.0).unwrap();
        fs::rename(&fixture.0, &moved).unwrap();
        fs::create_dir(&fixture.0).unwrap();
        fs::set_permissions(&fixture.0, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            revalidate_directory(&fixture.0, identity),
            Err(WorkspaceIdentityRecoveryRefusalV1::Conflict)
        );
        drop(descriptor);
        fs::remove_dir_all(&moved).unwrap();
    }

    #[test]
    fn v3_round_trip_encodes_separate_issued_and_reserved_marks_and_refuses_v1_v2() {
        let fixture = Fixture::new();
        let mut adapter = LocalWorkspaceIdentityMetadataV1::default();
        let value =
            WorkspaceIdentityMetadataV1::try_new(7, 1, allocation(0, 13, 0, 17), limits()).unwrap();
        adapter.store(&fixture.0, &value, limits()).unwrap();
        let bytes = fs::read(fixture.0.join(RECORD)).unwrap();
        assert_eq!(&bytes[..8], MAGIC);
        assert_eq!(integer(&bytes, 24).unwrap(), 0);
        assert_eq!(integer(&bytes, 32).unwrap(), 13);
        assert_eq!(integer(&bytes, 40).unwrap(), 0);
        assert_eq!(integer(&bytes, 48).unwrap(), 17);
        assert_eq!(
            adapter.load(&fixture.0, limits()),
            WorkspaceIdentityRecoveryV1::Recovered(value)
        );

        for magic in [LEGACY_V1_MAGIC, LEGACY_V2_MAGIC] {
            let mut legacy = vec![0_u8; HEADER_BYTES];
            legacy[..8].copy_from_slice(magic);
            fs::write(fixture.0.join(RECORD), legacy).unwrap();
            fs::set_permissions(fixture.0.join(RECORD), fs::Permissions::from_mode(0o600)).unwrap();
            assert_eq!(
                adapter.load(&fixture.0, limits()),
                WorkspaceIdentityRecoveryV1::Refused(WorkspaceIdentityRecoveryRefusalV1::Malformed)
            );
        }
    }

    #[test]
    fn issued_prefix_transition_requires_the_complete_new_tail_and_preserves_marks_on_removal() {
        let fixture = Fixture::new();
        let one = fixture.0.join("one.txt");
        let two = fixture.0.join("two.txt");
        let three = fixture.0.join("three.txt");
        fs::write(&one, b"one").unwrap();
        fs::write(&two, b"two").unwrap();
        fs::write(&three, b"three").unwrap();
        let mut adapter = LocalWorkspaceIdentityMetadataV1::default();
        let initial =
            WorkspaceIdentityMetadataV1::try_new(7, 1, allocation(0, 4, 0, 4), limits()).unwrap();
        adapter.store(&fixture.0, &initial, limits()).unwrap();
        let admitted = WorkspaceIdentityMetadataV1::try_new_with_bindings(
            7,
            2,
            allocation(1, 4, 0, 4),
            vec![WorkspaceDocumentBindingV1::try_new(1, one.clone()).unwrap()],
            vec![],
            limits(),
        )
        .unwrap();
        adapter.store(&fixture.0, &admitted, limits()).unwrap();
        let rebound = WorkspaceIdentityMetadataV1::try_new_with_bindings(
            7,
            3,
            allocation(1, 4, 0, 4),
            vec![WorkspaceDocumentBindingV1::try_new(1, two.clone()).unwrap()],
            vec![],
            limits(),
        )
        .unwrap();
        assert_eq!(
            adapter.store(&fixture.0, &rebound, limits()),
            Err(WorkspaceIdentityRecoveryRefusalV1::Conflict)
        );
        let removed =
            WorkspaceIdentityMetadataV1::try_new(7, 3, allocation(1, 4, 0, 4), limits()).unwrap();
        adapter.store(&fixture.0, &removed, limits()).unwrap();

        let revived = WorkspaceIdentityMetadataV1::try_new_with_bindings(
            7,
            4,
            allocation(1, 4, 0, 4),
            vec![WorkspaceDocumentBindingV1::try_new(1, one).unwrap()],
            vec![],
            limits(),
        )
        .unwrap();
        assert_eq!(
            adapter.store(&fixture.0, &revived, limits()),
            Err(WorkspaceIdentityRecoveryRefusalV1::Conflict)
        );

        let skipped = WorkspaceIdentityMetadataV1::try_new_with_bindings(
            7,
            4,
            allocation(3, 4, 0, 4),
            vec![WorkspaceDocumentBindingV1::try_new(3, three).unwrap()],
            vec![],
            limits(),
        )
        .unwrap();
        assert_eq!(
            adapter.store(&fixture.0, &skipped, limits()),
            Err(WorkspaceIdentityRecoveryRefusalV1::Conflict)
        );

        let contiguous = WorkspaceIdentityMetadataV1::try_new_with_bindings(
            7,
            4,
            allocation(3, 5, 0, 4),
            vec![
                WorkspaceDocumentBindingV1::try_new(2, two).unwrap(),
                WorkspaceDocumentBindingV1::try_new(3, fixture.0.join("three.txt")).unwrap(),
            ],
            vec![],
            limits(),
        )
        .unwrap();
        adapter.store(&fixture.0, &contiguous, limits()).unwrap();
        let regressed =
            WorkspaceIdentityMetadataV1::try_new(7, 5, allocation(2, 5, 0, 4), limits()).unwrap();
        assert_eq!(
            adapter.store(&fixture.0, &regressed, limits()),
            Err(WorkspaceIdentityRecoveryRefusalV1::Conflict)
        );
        let reserve_regressed =
            WorkspaceIdentityMetadataV1::try_new(7, 5, allocation(3, 4, 0, 4), limits()).unwrap();
        assert_eq!(
            adapter.store(&fixture.0, &reserve_regressed, limits()),
            Err(WorkspaceIdentityRecoveryRefusalV1::Conflict)
        );
    }

    #[test]
    fn initial_issued_prefix_cannot_contain_a_hole() {
        let fixture = Fixture::new();
        let locator = fixture.0.join("two.txt");
        fs::write(&locator, b"two").unwrap();
        let hole = WorkspaceIdentityMetadataV1::try_new_with_bindings(
            7,
            1,
            allocation(2, 4, 0, 4),
            vec![WorkspaceDocumentBindingV1::try_new(2, locator).unwrap()],
            vec![],
            limits(),
        )
        .unwrap();
        let mut adapter = LocalWorkspaceIdentityMetadataV1::default();
        assert_eq!(
            adapter.store(&fixture.0, &hole, limits()),
            Err(WorkspaceIdentityRecoveryRefusalV1::Conflict)
        );
    }

    #[test]
    fn root_transition_refuses_locator_or_kind_rebinding_and_retired_identity_revival() {
        let fixture = Fixture::new();
        let original = fixture.0.join("root.txt");
        let replacement = fixture.0.join("replacement.txt");
        fs::write(&original, b"root").unwrap();
        fs::write(&replacement, b"replacement").unwrap();
        let initial = WorkspaceIdentityMetadataV1::try_new_with_bindings(
            7,
            1,
            allocation(0, 4, 1, 4),
            vec![],
            vec![
                WorkspaceRootBindingV1::try_new(
                    1,
                    WorkspaceRootBindingKindV1::File,
                    original.clone(),
                )
                .unwrap(),
            ],
            limits(),
        )
        .unwrap();
        let rebound_locator = WorkspaceIdentityMetadataV1::try_new_with_bindings(
            7,
            2,
            allocation(0, 4, 1, 4),
            vec![],
            vec![
                WorkspaceRootBindingV1::try_new(1, WorkspaceRootBindingKindV1::File, replacement)
                    .unwrap(),
            ],
            limits(),
        )
        .unwrap();
        assert_eq!(
            validate_transition(&initial, &rebound_locator),
            Err(WorkspaceIdentityRecoveryRefusalV1::Conflict)
        );
        let rebound_kind = WorkspaceIdentityMetadataV1::try_new_with_bindings(
            7,
            2,
            allocation(0, 4, 1, 4),
            vec![],
            vec![
                WorkspaceRootBindingV1::try_new(
                    1,
                    WorkspaceRootBindingKindV1::Directory,
                    original.clone(),
                )
                .unwrap(),
            ],
            limits(),
        )
        .unwrap();
        assert_eq!(
            validate_transition(&initial, &rebound_kind),
            Err(WorkspaceIdentityRecoveryRefusalV1::Conflict)
        );
        let removed =
            WorkspaceIdentityMetadataV1::try_new(7, 2, allocation(0, 4, 1, 4), limits()).unwrap();
        let revived = WorkspaceIdentityMetadataV1::try_new_with_bindings(
            7,
            3,
            allocation(0, 4, 1, 4),
            vec![],
            vec![
                WorkspaceRootBindingV1::try_new(1, WorkspaceRootBindingKindV1::File, original)
                    .unwrap(),
            ],
            limits(),
        )
        .unwrap();
        assert_eq!(
            validate_transition(&removed, &revived),
            Err(WorkspaceIdentityRecoveryRefusalV1::Conflict)
        );
    }
}
