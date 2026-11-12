//! Adapter-neutral workspace resource discovery.
//!
//! Discovery grants no read, edit, or save authority. A refused operation may
//! report bounded progress, but never discloses a partial path set.

use std::{
    num::NonZeroU64,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectoryDiscoveryLimits {
    pub max_entries: usize,
    pub max_files: usize,
    pub max_depth: usize,
    pub max_path_bytes: usize,
    pub max_total_path_bytes: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DirectoryDiscoveryUsage {
    pub examined_entries: usize,
    pub directories: usize,
    pub regular_files: usize,
    pub retained_path_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectoryDiscoveryRefusal {
    InvalidLimits,
    Io,
    Limit,
    UnsupportedResource,
}

/// Returned paths are names observed through pinned directory descriptors, not
/// capabilities. They must be revalidated when subsequently opened because a
/// namespace entry may change after discovery completes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveredLocatorStatus {
    RevalidationRequired,
}

/// Closed discovery outcome. Refusal carries content-free progress evidence and
/// deliberately carries no partial paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DirectoryDiscoveryOutcome {
    Complete {
        files: Vec<PathBuf>,
        usage: DirectoryDiscoveryUsage,
        locator_status: DiscoveredLocatorStatus,
    },
    Refused {
        reason: DirectoryDiscoveryRefusal,
        usage: DirectoryDiscoveryUsage,
    },
}

pub trait DirectoryDiscoveryPort {
    fn discover(
        &mut self,
        root: &Path,
        limits: DirectoryDiscoveryLimits,
    ) -> DirectoryDiscoveryOutcome;
}

/// Bound on a durable, content-free workspace identity record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkspaceIdentityLimitsV1 {
    pub max_documents: usize,
    pub max_roots: usize,
    pub max_encoded_bytes: usize,
}

/// Restart-stable workspace scope, allocator reservation, and optional exact
/// locator bindings. The local adapter must validate persisted locator
/// provenance before returning a populated value; this host value alone is
/// not filesystem evidence and inventory order is never binding evidence.
#[derive(Clone, PartialEq, Eq)]
pub struct WorkspaceIdentityMetadataV1 {
    workspace: NonZeroU64,
    generation: NonZeroU64,
    document_issued_through: u64,
    document_reserved_through: NonZeroU64,
    root_issued_through: u64,
    root_reserved_through: NonZeroU64,
    document_bindings: Vec<WorkspaceDocumentBindingV1>,
    root_bindings: Vec<WorkspaceRootBindingV1>,
}

/// Exact issued prefixes and independently prepaid reservation ceilings for
/// the two workspace identity sorts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkspaceIdentityAllocationV1 {
    document_issued_through: u64,
    document_reserved_through: NonZeroU64,
    root_issued_through: u64,
    root_reserved_through: NonZeroU64,
}

impl WorkspaceIdentityAllocationV1 {
    pub fn try_new(
        document_issued_through: u64,
        document_reserved_through: u64,
        root_issued_through: u64,
        root_reserved_through: u64,
    ) -> Result<Self, WorkspaceIdentityValidationErrorV1> {
        let document_reserved_through = NonZeroU64::new(document_reserved_through)
            .ok_or(WorkspaceIdentityValidationErrorV1::Zero)?;
        let root_reserved_through = NonZeroU64::new(root_reserved_through)
            .ok_or(WorkspaceIdentityValidationErrorV1::Zero)?;
        if document_issued_through > document_reserved_through.get()
            || root_issued_through > root_reserved_through.get()
        {
            return Err(WorkspaceIdentityValidationErrorV1::Limit);
        }
        Ok(Self {
            document_issued_through,
            document_reserved_through,
            root_issued_through,
            root_reserved_through,
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct WorkspaceDocumentBindingV1 {
    ordinal: NonZeroU64,
    locator: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceRootBindingKindV1 {
    File,
    Directory,
}

#[derive(Clone, PartialEq, Eq)]
pub struct WorkspaceRootBindingV1 {
    ordinal: NonZeroU64,
    kind: WorkspaceRootBindingKindV1,
    locator: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceIdentityValidationErrorV1 {
    InvalidLimits,
    Zero,
    Limit,
    Duplicate,
}

impl WorkspaceDocumentBindingV1 {
    pub fn try_new(
        ordinal: u64,
        locator: PathBuf,
    ) -> Result<Self, WorkspaceIdentityValidationErrorV1> {
        let ordinal = NonZeroU64::new(ordinal).ok_or(WorkspaceIdentityValidationErrorV1::Zero)?;
        if locator.as_os_str().is_empty() {
            return Err(WorkspaceIdentityValidationErrorV1::Zero);
        }
        Ok(Self { ordinal, locator })
    }

    pub fn ordinal(&self) -> u64 {
        self.ordinal.get()
    }
    pub fn locator(&self) -> &Path {
        &self.locator
    }
}

impl std::fmt::Debug for WorkspaceDocumentBindingV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkspaceDocumentBindingV1")
            .field("ordinal", &"<opaque>")
            .field("locator", &"<redacted>")
            .finish()
    }
}

impl WorkspaceRootBindingV1 {
    pub fn try_new(
        ordinal: u64,
        kind: WorkspaceRootBindingKindV1,
        locator: PathBuf,
    ) -> Result<Self, WorkspaceIdentityValidationErrorV1> {
        let ordinal = NonZeroU64::new(ordinal).ok_or(WorkspaceIdentityValidationErrorV1::Zero)?;
        if locator.as_os_str().is_empty() {
            return Err(WorkspaceIdentityValidationErrorV1::Zero);
        }
        Ok(Self {
            ordinal,
            kind,
            locator,
        })
    }

    pub fn ordinal(&self) -> u64 {
        self.ordinal.get()
    }
    pub fn kind(&self) -> WorkspaceRootBindingKindV1 {
        self.kind
    }
    pub fn locator(&self) -> &Path {
        &self.locator
    }
}

impl std::fmt::Debug for WorkspaceRootBindingV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkspaceRootBindingV1")
            .field("ordinal", &"<opaque>")
            .field("kind", &self.kind)
            .field("locator", &"<redacted>")
            .finish()
    }
}

impl WorkspaceIdentityMetadataV1 {
    pub fn try_new(
        workspace: u64,
        generation: u64,
        allocation: WorkspaceIdentityAllocationV1,
        limits: WorkspaceIdentityLimitsV1,
    ) -> Result<Self, WorkspaceIdentityValidationErrorV1> {
        if limits.max_documents == 0 || limits.max_roots == 0 || limits.max_encoded_bytes == 0 {
            return Err(WorkspaceIdentityValidationErrorV1::InvalidLimits);
        }
        let workspace =
            NonZeroU64::new(workspace).ok_or(WorkspaceIdentityValidationErrorV1::Zero)?;
        let generation =
            NonZeroU64::new(generation).ok_or(WorkspaceIdentityValidationErrorV1::Zero)?;
        Ok(Self {
            workspace,
            generation,
            document_issued_through: allocation.document_issued_through,
            document_reserved_through: allocation.document_reserved_through,
            root_issued_through: allocation.root_issued_through,
            root_reserved_through: allocation.root_reserved_through,
            document_bindings: Vec::new(),
            root_bindings: Vec::new(),
        })
    }

    pub fn try_new_with_bindings(
        workspace: u64,
        generation: u64,
        allocation: WorkspaceIdentityAllocationV1,
        document_bindings: Vec<WorkspaceDocumentBindingV1>,
        root_bindings: Vec<WorkspaceRootBindingV1>,
        limits: WorkspaceIdentityLimitsV1,
    ) -> Result<Self, WorkspaceIdentityValidationErrorV1> {
        let mut value = Self::try_new(workspace, generation, allocation, limits)?;
        if document_bindings.len() > limits.max_documents || root_bindings.len() > limits.max_roots
        {
            return Err(WorkspaceIdentityValidationErrorV1::Limit);
        }
        let encoded_bytes = document_bindings
            .iter()
            .map(|binding| binding.locator().as_os_str().as_encoded_bytes().len())
            .chain(
                root_bindings
                    .iter()
                    .map(|binding| binding.locator().as_os_str().as_encoded_bytes().len()),
            )
            .try_fold(72usize, |total, locator_bytes| {
                total.checked_add(29)?.checked_add(locator_bytes)
            })
            .ok_or(WorkspaceIdentityValidationErrorV1::Limit)?;
        if encoded_bytes > limits.max_encoded_bytes {
            return Err(WorkspaceIdentityValidationErrorV1::Limit);
        }
        for (index, binding) in document_bindings.iter().enumerate() {
            if binding.ordinal() > value.document_issued_through
                || document_bindings[..index].iter().any(|prior| {
                    prior.ordinal() == binding.ordinal() || prior.locator() == binding.locator()
                })
            {
                return Err(WorkspaceIdentityValidationErrorV1::Duplicate);
            }
        }
        for (index, binding) in root_bindings.iter().enumerate() {
            if binding.ordinal() > value.root_issued_through
                || root_bindings[..index].iter().any(|prior| {
                    prior.ordinal() == binding.ordinal() || prior.locator() == binding.locator()
                })
            {
                return Err(WorkspaceIdentityValidationErrorV1::Duplicate);
            }
        }
        value.document_bindings = document_bindings;
        value.root_bindings = root_bindings;
        Ok(value)
    }

    pub fn workspace(&self) -> u64 {
        self.workspace.get()
    }

    pub fn generation(&self) -> u64 {
        self.generation.get()
    }

    pub fn document_reserved_through(&self) -> u64 {
        self.document_reserved_through.get()
    }

    pub fn document_issued_through(&self) -> u64 {
        self.document_issued_through
    }

    pub fn root_reserved_through(&self) -> u64 {
        self.root_reserved_through.get()
    }

    pub fn root_issued_through(&self) -> u64 {
        self.root_issued_through
    }

    pub fn document_bindings(&self) -> &[WorkspaceDocumentBindingV1] {
        &self.document_bindings
    }
    pub fn root_bindings(&self) -> &[WorkspaceRootBindingV1] {
        &self.root_bindings
    }
}

impl std::fmt::Debug for WorkspaceIdentityMetadataV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkspaceIdentityMetadataV1")
            .field("workspace", &"<opaque>")
            .field("generation", &"<opaque>")
            .field("document_issued_through", &"<opaque>")
            .field("document_reserved_through", &"<opaque>")
            .field("root_issued_through", &"<opaque>")
            .field("root_reserved_through", &"<opaque>")
            .field("document_bindings", &"<redacted>")
            .field("root_bindings", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceIdentityRecoveryRefusalV1 {
    InvalidLimits,
    Missing,
    Malformed,
    Limit,
    UnsupportedResource,
    Io,
    Conflict,
    /// The canonical rename completed but durability confirmation failed.
    /// Callers must recover before offering another generation.
    OutcomeUnknown,
}

/// Closed persistence result. Refusals never carry paths, identifiers, bytes,
/// or partially decoded metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkspaceIdentityRecoveryV1 {
    Recovered(WorkspaceIdentityMetadataV1),
    Refused(WorkspaceIdentityRecoveryRefusalV1),
}

pub trait WorkspaceIdentityMetadataPortV1 {
    fn load(
        &mut self,
        directory: &Path,
        limits: WorkspaceIdentityLimitsV1,
    ) -> WorkspaceIdentityRecoveryV1;

    fn store(
        &mut self,
        directory: &Path,
        metadata: &WorkspaceIdentityMetadataV1,
        limits: WorkspaceIdentityLimitsV1,
    ) -> Result<(), WorkspaceIdentityRecoveryRefusalV1>;
}
