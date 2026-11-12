//! Presentation-free workspace authority for services and headless embeddings.
//!
//! A `Workspace` owns documents and history publication. An `EditorView` is a
//! client-owned projection that selects a document but carries no authority to
//! mutate it. Cursors, selections, viewport, and other presentation state can
//! extend the view without entering the shared owner.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use vellum_engine::{
    coordinator::{DocumentEdits, DocumentVersion, HistoryGroupId},
    resource::{
        Acknowledgment, Application, Protection, SaveId, SaveOutcome, SaveRequirements,
        Synchronization,
    },
    source::Replacement,
};
use vellum_host::resource::{ErasedPathResourceAdapter, ErasedResourceHandle};
use vellum_host::workspace::DirectoryDiscoveryOutcome;
pub use vellum_host::workspace::{
    DirectoryDiscoveryLimits, DirectoryDiscoveryPort, DirectoryDiscoveryRefusal,
};
use vellum_host::{
    authority::{AuthorityRequestV1, SaveAsDestinationRequestV1},
    export_copy::{
        AuthorizedExportCopyV1, ExportCopyAuthorityPortV1, ExportCopyAuthorizationRefusalV1,
    },
};
use vellum_session::host::{
    LocalDocument as SessionDocument, LocalHost as SessionHost, SessionConfig, SessionError,
};

type LocalDocument = SessionDocument<ErasedResourceHandle>;
type LocalHost = SessionHost<ErasedPathResourceAdapter>;

/// Capabilities and bounds selected by the workspace composition root.
pub struct WorkspaceConfig(SessionConfig<vellum_host::resource::ErasedPathResourceAdapterFactory>);

impl WorkspaceConfig {
    pub fn new(
        session: SessionConfig<vellum_host::resource::ErasedPathResourceAdapterFactory>,
    ) -> Self {
        Self(session)
    }

    pub(crate) fn into_session(
        self,
    ) -> SessionConfig<vellum_host::resource::ErasedPathResourceAdapterFactory> {
        self.0
    }
}

impl std::ops::Deref for WorkspaceConfig {
    type Target = SessionConfig<vellum_host::resource::ErasedPathResourceAdapterFactory>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for WorkspaceConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub enum WorkspaceError {
    Empty,
    InvalidView,
    Capacity,
    Discovery(DirectoryDiscoveryRefusal),
    Session(SessionError),
}

/// Closed pre-effect failures for an exact workspace copy export.
///
/// No variant implies that destination I/O was attempted. In particular, a
/// stale protocol revision is rejected before authority is consulted, while
/// an authority refusal is discharged before it is returned to the caller.
#[derive(Debug)]
pub enum WorkspaceExportCopyErrorV1 {
    Workspace(WorkspaceError),
    Stale,
    Authority(ExportCopyAuthorizationRefusalV1),
    Preparation(vellum_session::host::SourceExportCopyPreparationErrorV1),
}

impl From<WorkspaceError> for WorkspaceExportCopyErrorV1 {
    fn from(value: WorkspaceError) -> Self {
        Self::Workspace(value)
    }
}

impl From<SessionError> for WorkspaceExportCopyErrorV1 {
    fn from(value: SessionError) -> Self {
        Self::Workspace(WorkspaceError::Session(value))
    }
}

impl From<SessionError> for WorkspaceError {
    fn from(value: SessionError) -> Self {
        Self::Session(value)
    }
}

impl std::fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "workspace: {self:?}")
    }
}

impl std::error::Error for WorkspaceError {}

/// Opaque identity for one live workspace. It is stable only for the
/// workspace's current process lifetime and cannot be recreated from a path or
/// caller-selected integer. Durable/public protocol identity is a separate host
/// concern.
#[derive(Clone)]
pub struct WorkspaceId(Arc<()>);

impl PartialEq for WorkspaceId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for WorkspaceId {}

impl std::fmt::Debug for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WorkspaceId(<opaque>)")
    }
}

/// Opaque, non-reused live-process identity for a document admitted to one
/// workspace. This is not a restart-stable protocol identifier.
#[derive(Clone)]
pub struct WorkspaceDocumentId {
    owner: Arc<()>,
    slot: usize,
}

impl PartialEq for WorkspaceDocumentId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}

impl Eq for WorkspaceDocumentId {}

impl std::fmt::Debug for WorkspaceDocumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WorkspaceDocumentId(<opaque>)")
    }
}

/// Opaque, non-reused live-process identity for an explicitly added workspace
/// root. This is not a restart-stable protocol identifier.
#[derive(Clone)]
pub struct WorkspaceRootId {
    owner: Arc<()>,
    slot: usize,
}

impl PartialEq for WorkspaceRootId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}

impl Eq for WorkspaceRootId {}

impl std::fmt::Debug for WorkspaceRootId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WorkspaceRootId(<opaque>)")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceRootKind {
    File,
    Directory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceRoot {
    pub id: WorkspaceRootId,
    pub kind: WorkspaceRootKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceDocumentBinding {
    pub document: WorkspaceDocumentId,
    pub locator: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceRootBinding {
    pub root: WorkspaceRootId,
    pub kind: WorkspaceRootKind,
    pub locator: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceBindingInventory {
    pub documents: Vec<WorkspaceDocumentBinding>,
    pub roots: Vec<WorkspaceRootBinding>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddFile {
    pub document: WorkspaceDocumentId,
    pub root: Option<WorkspaceRootId>,
    pub added: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddDirectory {
    pub root: WorkspaceRootId,
    pub added: usize,
    pub joined: usize,
    /// Complete bounded identities introduced by this serialized admission,
    /// in discovery admission order.
    pub documents: Vec<WorkspaceDocumentId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MembershipRemovalStatus {
    Removed,
    AlreadyRemoved,
}

/// Bounded copy of the workspace's content-free identity inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceInventory {
    pub workspace: WorkspaceId,
    pub documents: Vec<WorkspaceDocumentId>,
    pub roots: Vec<WorkspaceRoot>,
}

/// Bounded page over the canonical document-then-root inventory order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceInventoryPage {
    pub generation: u64,
    pub total_entries: usize,
    pub documents: Vec<WorkspaceDocumentId>,
    pub roots: Vec<WorkspaceRoot>,
}

struct WorkspaceDocument {
    id: WorkspaceDocumentId,
    locator: PathBuf,
    document: LocalDocument,
    member: bool,
}

struct WorkspaceRootEntry {
    root: WorkspaceRoot,
    locator: PathBuf,
    member: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SaveRetention {
    /// Outcome or contention is not closed; never attempt retirement.
    Protected,
    /// Current published baseline. A newer published baseline must supersede it
    /// before retirement is attempted.
    Baseline,
    /// Terminal evidence or a superseded baseline permits a retirement attempt.
    Eligible,
}

struct RetainedSave {
    document: WorkspaceDocumentId,
    task: SaveId,
    retention: SaveRetention,
}

/// One client's non-authoritative projection into a shared workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorView {
    active_document: WorkspaceDocumentId,
}

impl EditorView {
    pub fn active_document(&self) -> usize {
        self.active_document.slot
    }

    pub fn active_document_id(&self) -> &WorkspaceDocumentId {
        &self.active_document
    }
}

/// Immutable source capture. It is disclosure, not write authority.
pub struct WorkspaceSnapshot {
    pub version: DocumentVersion,
    pub text: String,
}

/// One exact workspace participant in an atomic in-memory source publication.
/// The workspace identity and native version must name the same retained
/// document; neither value is authority by possession.
pub struct WorkspaceDocumentEdits {
    pub document: WorkspaceDocumentId,
    pub expected: DocumentVersion,
    pub edits: Vec<Replacement>,
}

impl std::fmt::Debug for WorkspaceDocumentEdits {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkspaceDocumentEdits")
            .field("document", &self.document)
            .field("revision", &self.expected.revision)
            .field("edits", &self.edits.len())
            .finish()
    }
}

/// Evidence from one engine prepare/commit. This is atomic memory publication
/// only; it makes no filesystem write, synchronization, or durability claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceBatchEditReceipt {
    pub generation: u64,
    pub changed_documents: usize,
    pub history_group: Option<HistoryGroupId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceSaveReceipt {
    pub application: Application,
    pub synchronization: Synchronization,
    pub no_future_effects: bool,
    pub quiescent: bool,
    pub adapter_failure: bool,
    pub cleanup_failed: bool,
    pub baseline_published: bool,
    pub outcome_unknown: bool,
}

impl std::fmt::Debug for WorkspaceSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WorkspaceSnapshot(<redacted>)")
    }
}

/// Shared document and history authority with no presentation state.
pub struct Workspace {
    id: WorkspaceId,
    host: LocalHost,
    documents: Vec<WorkspaceDocument>,
    roots: Vec<WorkspaceRootEntry>,
    max_documents: usize,
    groups: Vec<HistoryGroupId>,
    save_tasks: Vec<RetainedSave>,
    max_save_tasks: usize,
    inventory_generation: u64,
}

impl Workspace {
    pub fn open(paths: &[PathBuf], config: WorkspaceConfig) -> Result<Self, WorkspaceError> {
        let mut workspace = Self::empty(config)?;
        if paths.len() > workspace.max_documents {
            return Err(WorkspaceError::Capacity);
        }
        for path in paths {
            workspace.add_file(path)?;
        }
        Ok(workspace)
    }

    pub fn empty(config: WorkspaceConfig) -> Result<Self, WorkspaceError> {
        let session = config.into_session();
        let max_documents = session.engine.limits.max_document_incarnations;
        let max_save_tasks = session
            .engine
            .resources
            .as_ref()
            .map_or(0, |limits| limits.max_live_tasks);
        let mut save_tasks = Vec::new();
        save_tasks
            .try_reserve_exact(max_save_tasks)
            .map_err(|_| WorkspaceError::Capacity)?;
        Ok(Self {
            id: WorkspaceId(Arc::new(())),
            host: LocalHost::new(session)?,
            documents: Vec::new(),
            roots: Vec::new(),
            max_documents,
            groups: Vec::new(),
            save_tasks,
            max_save_tasks,
            inventory_generation: 1,
        })
    }

    pub fn id(&self) -> &WorkspaceId {
        &self.id
    }

    pub fn inventory(&self) -> Result<WorkspaceInventory, WorkspaceError> {
        let mut documents = Vec::new();
        documents
            .try_reserve_exact(self.documents.len())
            .map_err(|_| WorkspaceError::Capacity)?;
        documents.extend(
            self.documents
                .iter()
                .filter(|entry| entry.member)
                .map(|entry| entry.id.clone()),
        );
        let mut roots = Vec::new();
        roots
            .try_reserve_exact(self.roots.len())
            .map_err(|_| WorkspaceError::Capacity)?;
        roots.extend(
            self.roots
                .iter()
                .filter(|entry| entry.member)
                .map(|entry| entry.root.clone()),
        );
        Ok(WorkspaceInventory {
            workspace: self.id.clone(),
            documents,
            roots,
        })
    }

    /// Exact locator-to-live-identity associations for trusted startup
    /// composition. Locators are not protocol inventory and must not be
    /// disclosed to clients by projection code.
    pub fn binding_inventory(&self) -> Result<WorkspaceBindingInventory, WorkspaceError> {
        let mut documents = Vec::new();
        documents
            .try_reserve_exact(self.documents.len())
            .map_err(|_| WorkspaceError::Capacity)?;
        documents.extend(
            self.documents
                .iter()
                .filter(|entry| entry.member)
                .map(|entry| WorkspaceDocumentBinding {
                    document: entry.id.clone(),
                    locator: entry.locator.clone(),
                }),
        );
        let mut roots = Vec::new();
        roots
            .try_reserve_exact(self.roots.len())
            .map_err(|_| WorkspaceError::Capacity)?;
        roots.extend(self.roots.iter().filter(|entry| entry.member).map(|entry| {
            WorkspaceRootBinding {
                root: entry.root.id.clone(),
                kind: entry.root.kind,
                locator: entry.locator.clone(),
            }
        }));
        Ok(WorkspaceBindingInventory { documents, roots })
    }

    /// Pure structural witness for inventory membership/order. This is not a
    /// source revision and does not change for document edits or saves.
    pub fn inventory_generation(&self) -> u64 {
        self.inventory_generation
    }

    /// Content-free count of linked history groups published by this
    /// workspace. This is observation evidence only: it exposes neither group
    /// identities nor undo authority.
    pub fn linked_history_group_count(&self) -> usize {
        self.groups.len()
    }

    /// Copies at most `limit` identities from one exact inventory generation.
    /// Offset zero begins the document-then-root order. Callers must bind a
    /// continuation to `generation`; this method never silently resynchronizes.
    pub fn inventory_page(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<WorkspaceInventoryPage, WorkspaceError> {
        if limit == 0 {
            return Err(WorkspaceError::Capacity);
        }
        let document_count = self.documents.iter().filter(|entry| entry.member).count();
        let root_count = self.roots.iter().filter(|entry| entry.member).count();
        let total_entries = document_count
            .checked_add(root_count)
            .ok_or(WorkspaceError::Capacity)?;
        if offset > total_entries {
            return Err(WorkspaceError::Capacity);
        }
        let end = offset.saturating_add(limit).min(total_entries);
        let document_start = offset.min(document_count);
        let document_end = end.min(document_count);
        let root_start = offset.saturating_sub(document_count);
        let root_end = end.saturating_sub(document_count);
        let mut documents = Vec::new();
        documents
            .try_reserve_exact(document_end.saturating_sub(document_start))
            .map_err(|_| WorkspaceError::Capacity)?;
        documents.extend(
            self.documents
                .iter()
                .filter(|entry| entry.member)
                .skip(document_start)
                .take(document_end.saturating_sub(document_start))
                .map(|entry| entry.id.clone()),
        );
        let mut roots = Vec::new();
        roots
            .try_reserve_exact(root_end.saturating_sub(root_start))
            .map_err(|_| WorkspaceError::Capacity)?;
        roots.extend(
            self.roots
                .iter()
                .filter(|entry| entry.member)
                .skip(root_start)
                .take(root_end.saturating_sub(root_start))
                .map(|entry| entry.root.clone()),
        );
        Ok(WorkspaceInventoryPage {
            generation: self.inventory_generation,
            total_entries,
            documents,
            roots,
        })
    }

    pub fn add_file(&mut self, path: &Path) -> Result<AddFile, WorkspaceError> {
        self.add_file_inner(path, true)
    }

    fn add_file_inner(
        &mut self,
        path: &Path,
        explicit_root: bool,
    ) -> Result<AddFile, WorkspaceError> {
        if let Some(retained) = self
            .documents
            .iter()
            .find(|retained| retained.member && retained.locator == path)
        {
            return Ok(AddFile {
                document: retained.id.clone(),
                root: None,
                added: false,
            });
        }
        if self.documents.len() >= self.max_documents {
            return Err(WorkspaceError::Capacity);
        }
        if explicit_root && self.roots.len() >= self.root_capacity() {
            return Err(WorkspaceError::Capacity);
        }
        self.documents
            .try_reserve(1)
            .map_err(|_| WorkspaceError::Capacity)?;
        if explicit_root {
            self.roots
                .try_reserve(1)
                .map_err(|_| WorkspaceError::Capacity)?;
        }
        let document = self.host.open(path)?;
        let identity = self.host.read(&document)?.version.document;
        for retained in self.documents.iter().filter(|retained| retained.member) {
            if self.host.read(&retained.document)?.version.document == identity {
                return Ok(AddFile {
                    document: retained.id.clone(),
                    root: None,
                    added: false,
                });
            }
        }
        let next_inventory_generation = self
            .inventory_generation
            .checked_add(1)
            .ok_or(WorkspaceError::Capacity)?;
        let id = WorkspaceDocumentId {
            owner: self.id.0.clone(),
            slot: self.documents.len(),
        };
        let root = explicit_root.then(|| WorkspaceRootId {
            owner: self.id.0.clone(),
            slot: self.roots.len(),
        });
        self.documents.push(WorkspaceDocument {
            id: id.clone(),
            locator: path.to_path_buf(),
            document,
            member: true,
        });
        if let Some(root) = &root {
            self.roots.push(WorkspaceRootEntry {
                root: WorkspaceRoot {
                    id: root.clone(),
                    kind: WorkspaceRootKind::File,
                },
                locator: path.to_path_buf(),
                member: true,
            });
        }
        self.inventory_generation = next_inventory_generation;
        Ok(AddFile {
            document: id,
            root,
            added: true,
        })
    }

    pub fn add_directory<D: DirectoryDiscoveryPort + ?Sized>(
        &mut self,
        root: &Path,
        limits: DirectoryDiscoveryLimits,
        discovery: &mut D,
    ) -> Result<AddDirectory, WorkspaceError> {
        let existing_root = self
            .roots
            .iter()
            .find(|entry| {
                entry.member
                    && entry.locator == root
                    && entry.root.kind == WorkspaceRootKind::Directory
            })
            .map(|entry| entry.root.id.clone());
        if existing_root.is_none() && self.roots.len() >= self.root_capacity() {
            return Err(WorkspaceError::Capacity);
        }
        let files = match discovery.discover(root, limits) {
            DirectoryDiscoveryOutcome::Complete { files, .. } => files,
            DirectoryDiscoveryOutcome::Refused { reason, .. } => {
                return Err(WorkspaceError::Discovery(reason));
            }
        };
        if files.len() > limits.max_files {
            return Err(WorkspaceError::Discovery(DirectoryDiscoveryRefusal::Limit));
        }
        let mut total_path_bytes = 0usize;
        for file in &files {
            let path_bytes = file.as_os_str().as_encoded_bytes().len();
            if path_bytes > limits.max_path_bytes {
                return Err(WorkspaceError::Discovery(DirectoryDiscoveryRefusal::Limit));
            }
            total_path_bytes = total_path_bytes
                .checked_add(path_bytes)
                .filter(|total| *total <= limits.max_total_path_bytes)
                .ok_or(WorkspaceError::Discovery(DirectoryDiscoveryRefusal::Limit))?;
        }
        let mut unique_candidates = 0usize;
        for (index, file) in files.iter().enumerate() {
            if self
                .documents
                .iter()
                .any(|retained| retained.member && retained.locator == *file)
                || files[..index].iter().any(|earlier| earlier == file)
            {
                continue;
            }
            unique_candidates = unique_candidates
                .checked_add(1)
                .ok_or(WorkspaceError::Capacity)?;
        }
        if unique_candidates > self.max_documents.saturating_sub(self.documents.len()) {
            return Err(WorkspaceError::Capacity);
        }
        self.documents
            .try_reserve(unique_candidates)
            .map_err(|_| WorkspaceError::Capacity)?;
        if existing_root.is_none() {
            self.roots
                .try_reserve(1)
                .map_err(|_| WorkspaceError::Capacity)?;
        }
        let root_id = existing_root.clone().unwrap_or_else(|| WorkspaceRootId {
            owner: self.id.0.clone(),
            slot: self.roots.len(),
        });
        let mut staged = Vec::new();
        staged
            .try_reserve_exact(files.len())
            .map_err(|_| WorkspaceError::Capacity)?;
        let mut added = 0usize;
        let mut joined = 0usize;
        for file in files {
            if self
                .documents
                .iter()
                .chain(staged.iter())
                .any(|retained: &WorkspaceDocument| retained.member && retained.locator == file)
            {
                joined = joined.checked_add(1).ok_or(WorkspaceError::Capacity)?;
                continue;
            }
            let document = self.host.open(&file)?;
            let identity = self.host.read(&document)?.version.document;
            let mut duplicate = false;
            for retained in self
                .documents
                .iter()
                .filter(|retained| retained.member)
                .chain(staged.iter())
            {
                if self.host.read(&retained.document)?.version.document == identity {
                    duplicate = true;
                    break;
                }
            }
            if duplicate {
                joined = joined.checked_add(1).ok_or(WorkspaceError::Capacity)?;
                continue;
            }
            let slot = self
                .documents
                .len()
                .checked_add(staged.len())
                .ok_or(WorkspaceError::Capacity)?;
            staged.push(WorkspaceDocument {
                id: WorkspaceDocumentId {
                    owner: self.id.0.clone(),
                    slot,
                },
                locator: file,
                document,
                member: true,
            });
            added = added.checked_add(1).ok_or(WorkspaceError::Capacity)?;
        }
        // The inventory publishes only after every external open completed.
        // Failed discovery/open may have affected the adapter, but never exposes
        // a partial root or partial membership set through this workspace.
        let mut documents = Vec::new();
        documents
            .try_reserve_exact(staged.len())
            .map_err(|_| WorkspaceError::Capacity)?;
        documents.extend(staged.iter().map(|document| document.id.clone()));
        let changed = !staged.is_empty() || existing_root.is_none();
        let next_inventory_generation = if changed {
            Some(
                self.inventory_generation
                    .checked_add(1)
                    .ok_or(WorkspaceError::Capacity)?,
            )
        } else {
            None
        };
        self.documents.extend(staged);
        if existing_root.is_none() {
            self.roots.push(WorkspaceRootEntry {
                root: WorkspaceRoot {
                    id: root_id.clone(),
                    kind: WorkspaceRootKind::Directory,
                },
                locator: root.to_path_buf(),
                member: true,
            });
        }
        if let Some(generation) = next_inventory_generation {
            self.inventory_generation = generation;
        }
        Ok(AddDirectory {
            root: root_id,
            added,
            joined,
            documents,
        })
    }

    pub fn document_count(&self) -> usize {
        self.documents.iter().filter(|entry| entry.member).count()
    }

    pub fn document_capacity(&self) -> usize {
        self.max_documents
    }

    pub fn root_capacity(&self) -> usize {
        self.max_documents
    }

    pub fn open_view(&self) -> Result<EditorView, WorkspaceError> {
        let document = self
            .documents
            .iter()
            .find(|entry| entry.member)
            .ok_or(WorkspaceError::Empty)?;
        Ok(EditorView {
            active_document: document.id.clone(),
        })
    }

    pub fn select_document(
        &self,
        view: &mut EditorView,
        document: usize,
    ) -> Result<(), WorkspaceError> {
        if document >= self.documents.len() || !self.documents[document].member {
            return Err(WorkspaceError::InvalidView);
        }
        view.active_document = self.documents[document].id.clone();
        Ok(())
    }

    pub fn select_document_id(
        &self,
        view: &mut EditorView,
        document: &WorkspaceDocumentId,
    ) -> Result<(), WorkspaceError> {
        self.entry(document)?;
        view.active_document = document.clone();
        Ok(())
    }

    /// Retires only workspace membership. The native source, history, save
    /// tasks, provenance and filesystem content remain retained by their
    /// existing owners.
    pub fn remove_document(
        &mut self,
        document: &WorkspaceDocumentId,
    ) -> Result<MembershipRemovalStatus, WorkspaceError> {
        if !Arc::ptr_eq(&self.id.0, &document.owner) {
            return Err(WorkspaceError::InvalidView);
        }
        let entry = self
            .documents
            .get_mut(document.slot)
            .filter(|entry| entry.id == *document)
            .ok_or(WorkspaceError::InvalidView)?;
        if !entry.member {
            return Ok(MembershipRemovalStatus::AlreadyRemoved);
        }
        let generation = self
            .inventory_generation
            .checked_add(1)
            .ok_or(WorkspaceError::Capacity)?;
        entry.member = false;
        self.inventory_generation = generation;
        Ok(MembershipRemovalStatus::Removed)
    }

    /// Retires one explicit root membership without cascading to documents.
    /// This workspace does not currently own root-to-document provenance.
    pub fn remove_root(
        &mut self,
        root: &WorkspaceRootId,
    ) -> Result<MembershipRemovalStatus, WorkspaceError> {
        if !Arc::ptr_eq(&self.id.0, &root.owner) {
            return Err(WorkspaceError::InvalidView);
        }
        let entry = self
            .roots
            .get_mut(root.slot)
            .filter(|entry| entry.root.id == *root)
            .ok_or(WorkspaceError::InvalidView)?;
        if !entry.member {
            return Ok(MembershipRemovalStatus::AlreadyRemoved);
        }
        let generation = self
            .inventory_generation
            .checked_add(1)
            .ok_or(WorkspaceError::Capacity)?;
        entry.member = false;
        self.inventory_generation = generation;
        Ok(MembershipRemovalStatus::Removed)
    }

    pub fn version(&mut self, view: &EditorView) -> Result<DocumentVersion, WorkspaceError> {
        let document = self.document(view)?.clone();
        Ok(self.host.read(&document)?.version)
    }

    pub fn document_version(
        &mut self,
        document: &WorkspaceDocumentId,
    ) -> Result<DocumentVersion, WorkspaceError> {
        let retained = self.entry(document)?.document.clone();
        Ok(self.host.read(&retained)?.version)
    }

    /// Resolve one workspace-owned document to its exact native version, then
    /// acquire the atomic source/destination grant before the codec discloses
    /// bytes. This does not mutate membership, selection, source text, or the
    /// document's persistence binding.
    pub fn authorize_document_export_copy<A: ExportCopyAuthorityPortV1 + ?Sized>(
        &mut self,
        document: &WorkspaceDocumentId,
        expected_revision: u64,
        authority: &mut A,
        source_request: AuthorityRequestV1,
        destination_request: SaveAsDestinationRequestV1,
    ) -> Result<AuthorizedExportCopyV1, WorkspaceExportCopyErrorV1> {
        let retained = self.entry(document)?.document.clone();
        let version = self.host.read(&retained)?.version;
        if version.revision != expected_revision {
            return Err(WorkspaceExportCopyErrorV1::Stale);
        }
        match self.host.authorize_source_export_copy(
            &version,
            authority,
            source_request,
            destination_request,
        ) {
            Ok(authorized) => Ok(authorized),
            Err(vellum_session::host::SourceExportCopyPreparationErrorV1::Authority(refusal)) => {
                Err(WorkspaceExportCopyErrorV1::Authority(refusal))
            }
            Err(error) => {
                // Capture/attachment errors do not establish that the affine
                // preparation remains live. Fail closed without guessing its
                // state or fabricating a reusable preparation.
                Err(WorkspaceExportCopyErrorV1::Preparation(error))
            }
        }
    }

    pub fn snapshot(&mut self, view: &EditorView) -> Result<WorkspaceSnapshot, WorkspaceError> {
        self.document_snapshot(&view.active_document)
    }

    /// Captures one exact workspace document without manufacturing presentation
    /// state. The returned source is disclosure, not edit authority.
    pub fn document_snapshot(
        &mut self,
        document: &WorkspaceDocumentId,
    ) -> Result<WorkspaceSnapshot, WorkspaceError> {
        let document = self.entry(document)?.document.clone();
        let read = self.host.read(&document)?;
        let mut text = String::new();
        text.try_reserve_exact(read.source.text().len())
            .map_err(|_| WorkspaceError::Capacity)?;
        text.push_str(read.source.text());
        Ok(WorkspaceSnapshot {
            version: read.version,
            text,
        })
    }

    /// Publishes all exact source changes through one engine prepare/commit and
    /// one linked history group. Every workspace/native identity pair is
    /// checked before preparation. No filesystem persistence is attempted.
    pub fn edit_document_batch(
        &mut self,
        writes: &[WorkspaceDocumentEdits],
    ) -> Result<WorkspaceBatchEditReceipt, WorkspaceError> {
        let mut native = Vec::new();
        native
            .try_reserve_exact(writes.len())
            .map_err(|_| WorkspaceError::Capacity)?;
        for write in writes {
            let retained = self.entry(&write.document)?.document.clone();
            if self.host.read(&retained)?.version.document != write.expected.document {
                return Err(SessionError::WrongDocument.into());
            }
            let mut edits = Vec::new();
            edits
                .try_reserve_exact(write.edits.len())
                .map_err(|_| WorkspaceError::Capacity)?;
            edits.extend(write.edits.iter().cloned());
            native.push(DocumentEdits {
                expected: write.expected.clone(),
                edits,
            });
        }
        self.groups
            .try_reserve(1)
            .map_err(|_| WorkspaceError::Capacity)?;
        let outcome = self.host.edit_batch(&native, &[])?;
        if let Some(group) = &outcome.history_group {
            self.groups.push(group.clone());
        }
        Ok(WorkspaceBatchEditReceipt {
            generation: outcome.generation,
            changed_documents: outcome.changes.len(),
            history_group: outcome.history_group,
        })
    }

    pub fn edit_snapshot(
        &mut self,
        view: &EditorView,
        expected: &DocumentVersion,
        edits: Vec<Replacement>,
    ) -> Result<(), WorkspaceError> {
        let document = self.document(view)?.clone();
        if self.host.read(&document)?.version.document != expected.document {
            return Err(SessionError::WrongDocument.into());
        }
        self.groups
            .try_reserve(1)
            .map_err(|_| WorkspaceError::Capacity)?;
        let outcome = self.host.edit(expected, edits)?;
        if let Some(group) = outcome.history_group {
            self.groups.push(group);
        }
        Ok(())
    }

    pub fn save_snapshot(
        &mut self,
        view: &EditorView,
        expected: &DocumentVersion,
    ) -> Result<WorkspaceSaveReceipt, WorkspaceError> {
        let document_id = view.active_document.clone();
        let document = self.document(view)?.clone();
        if self.host.read(&document)?.version != *expected {
            return Err(SessionError::WrongDocument.into());
        }
        self.reclaim_eligible_save_tasks();
        if self.save_tasks.len() >= self.max_save_tasks {
            return Err(WorkspaceError::Capacity);
        }
        let task = self.host.queue_save(
            expected,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Synchronized,
            },
        )?;
        self.save_tasks.push(RetainedSave {
            document: document_id.clone(),
            task: task.clone(),
            retention: SaveRetention::Protected,
        });
        let report = match self.host.service_save(&document, &task) {
            Ok(report) => report,
            Err(_) => {
                // Service errors do not prove non-application. Ask the retained
                // task to stop, then report only the evidence the engine can
                // actually close. In particular, a pre-dispatch external-change
                // conflict becomes terminal NotWritten rather than an invented
                // transport-style unknown outcome.
                let stop_failed = self.host.stop_save(&task).is_err();
                return Ok(match self.host.save_outcome(&task) {
                    Ok(outcome) => {
                        let outcome_unknown = stop_failed
                            || outcome.application == Application::Unknown
                            || !outcome.no_future_effects
                            || !outcome.quiescent;
                        if !outcome_unknown {
                            self.mark_save_retention(&task, SaveRetention::Eligible);
                            self.reclaim_eligible_save_tasks();
                        }
                        save_receipt(&outcome, outcome_unknown, false, false, false)
                    }
                    Err(_) => WorkspaceSaveReceipt {
                        application: Application::Unknown,
                        synchronization: Synchronization::Unknown,
                        no_future_effects: false,
                        quiescent: false,
                        adapter_failure: false,
                        cleanup_failed: false,
                        baseline_published: false,
                        outcome_unknown: true,
                    },
                });
            }
        };
        let complete = report.failure.is_none()
            && !report.cleanup_failed
            && report.outcome.application == Application::Written
            && report.outcome.synchronization == Synchronization::Succeeded
            && report.outcome.no_future_effects
            && report.outcome.quiescent;
        let baseline_published = complete && self.host.publish_save_baseline(&task).is_ok();
        let terminal = report.outcome.application != Application::Unknown
            && report.outcome.no_future_effects
            && report.outcome.quiescent;
        if baseline_published {
            for retained in &mut self.save_tasks {
                if retained.document == document_id && retained.retention == SaveRetention::Baseline
                {
                    retained.retention = SaveRetention::Eligible;
                }
            }
            self.mark_save_retention(&task, SaveRetention::Baseline);
        } else if terminal {
            self.mark_save_retention(&task, SaveRetention::Eligible);
        }
        self.reclaim_eligible_save_tasks();
        Ok(save_receipt(
            &report.outcome,
            false,
            report.failure.is_some(),
            report.cleanup_failed,
            baseline_published,
        ))
    }

    fn document(&self, view: &EditorView) -> Result<&LocalDocument, WorkspaceError> {
        Ok(&self.entry(&view.active_document)?.document)
    }

    fn mark_save_retention(&mut self, task: &SaveId, retention: SaveRetention) {
        if let Some(retained) = self
            .save_tasks
            .iter_mut()
            .find(|retained| retained.task == *task)
        {
            retained.retention = retention;
        }
    }

    fn reclaim_eligible_save_tasks(&mut self) {
        let mut index = 0;
        while index < self.save_tasks.len() {
            if self.save_tasks[index].retention == SaveRetention::Eligible
                && self.host.retire_save(&self.save_tasks[index].task).is_ok()
            {
                self.save_tasks.swap_remove(index);
            } else {
                index += 1;
            }
        }
    }

    fn entry(&self, id: &WorkspaceDocumentId) -> Result<&WorkspaceDocument, WorkspaceError> {
        if !Arc::ptr_eq(&self.id.0, &id.owner) {
            return Err(WorkspaceError::InvalidView);
        }
        self.documents
            .get(id.slot)
            .filter(|entry| entry.member && entry.id == *id)
            .ok_or(WorkspaceError::InvalidView)
    }
}

fn save_receipt(
    outcome: &SaveOutcome,
    outcome_unknown: bool,
    adapter_failure: bool,
    cleanup_failed: bool,
    baseline_published: bool,
) -> WorkspaceSaveReceipt {
    WorkspaceSaveReceipt {
        application: outcome.application,
        synchronization: outcome.synchronization,
        no_future_effects: outcome.no_future_effects,
        quiescent: outcome.quiescent,
        adapter_failure,
        cleanup_failed,
        baseline_published,
        outcome_unknown,
    }
}
