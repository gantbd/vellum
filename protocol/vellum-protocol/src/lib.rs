//! Closed native protocol between Vellum clients and a Vellum service.
//!
//! This first profile is deliberately in-process and single-workspace. It is
//! not itself a wire-format, authentication, or remote-transport claim.
//! `vellum-wire` owns the canonical encoding of these values. MCP and other
//! transports must project this vocabulary rather than exposing editor or
//! engine internals.
#![forbid(unsafe_code)]

use std::{fmt, num::NonZeroU64, sync::Arc};

macro_rules! opaque_id {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(NonZeroU64);

        impl $name {
            pub fn new(value: u64) -> Option<Self> {
                NonZeroU64::new(value).map(Self)
            }

            pub fn get(self) -> u64 {
                self.0.get()
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "(<opaque>)"))
            }
        }
    };
}

opaque_id!(ClientId);
opaque_id!(RequestId);
opaque_id!(VersionWitness);
opaque_id!(WorkspaceIdV1);

/// Fixed-width, caller-chosen correlation for one durable effect.
///
/// This value is neither authority nor authentication evidence. Trusted
/// compositions generate it from an OS CSPRNG before the first effect attempt.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DurableOperationRefV1([u8; 16]);

impl DurableOperationRefV1 {
    pub fn new(bytes: [u8; 16]) -> Option<Self> {
        if bytes == [0; 16] {
            None
        } else {
            Some(Self(bytes))
        }
    }

    pub const fn to_bytes(self) -> [u8; 16] {
        self.0
    }
}

impl fmt::Debug for DurableOperationRefV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DurableOperationRefV1(<opaque>)")
    }
}

/// Opaque continuation for one exact workspace inventory generation.
///
/// The fields are exposed only to canonical codecs and interpreters. They are
/// not filesystem locators, source revisions, or durable resume tokens.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkspaceInventoryCursorV1 {
    workspace: WorkspaceIdV1,
    generation: NonZeroU64,
    offset: NonZeroU64,
}

impl WorkspaceInventoryCursorV1 {
    pub fn new(workspace: WorkspaceIdV1, generation: u64, offset: u64) -> Option<Self> {
        Some(Self {
            workspace,
            generation: NonZeroU64::new(generation)?,
            offset: NonZeroU64::new(offset)?,
        })
    }

    pub fn workspace(self) -> WorkspaceIdV1 {
        self.workspace
    }

    pub fn generation(self) -> u64 {
        self.generation.get()
    }

    pub fn offset(self) -> u64 {
        self.offset.get()
    }
}

impl fmt::Debug for WorkspaceInventoryCursorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WorkspaceInventoryCursorV1(<opaque>)")
    }
}

macro_rules! scoped_workspace_id {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name {
            workspace: WorkspaceIdV1,
            local: NonZeroU64,
        }

        impl $name {
            pub fn new(workspace: WorkspaceIdV1, local: u64) -> Option<Self> {
                NonZeroU64::new(local).map(|local| Self { workspace, local })
            }

            pub fn workspace(self) -> WorkspaceIdV1 {
                self.workspace
            }

            pub fn get(self) -> u64 {
                self.local.get()
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "(<opaque>)"))
            }
        }
    };
}

scoped_workspace_id!(WorkspaceDocumentIdV1);
scoped_workspace_id!(WorkspaceRootIdV1);

/// Cross-transport V1 bound. Two roots plus continuation metadata fit the
/// canonical reference wire profile's sixteen-field ceiling. Roots cost two
/// wire fields, so this deliberately uses the worst-case entry shape.
pub const MAX_WORKSPACE_INVENTORY_PAGE_SIZE_V1: u16 = 2;

/// Conservative cross-transport V1 change-feed page bound. One worst-case
/// source-change event plus its continuation fits the canonical sixteen-field
/// wire profile.
pub const MAX_WORKSPACE_CHANGE_PAGE_SIZE_V1: u16 = 1;

/// Opaque position after one exact workspace change revision. Revision zero
/// denotes the point before the first change and is valid only when scoped to
/// the exact workspace.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkspaceChangeCursorV1 {
    workspace: WorkspaceIdV1,
    epoch: NonZeroU64,
    revision: u64,
}

impl WorkspaceChangeCursorV1 {
    pub fn new(workspace: WorkspaceIdV1, epoch: u64, revision: u64) -> Option<Self> {
        Some(Self {
            workspace,
            epoch: NonZeroU64::new(epoch)?,
            revision,
        })
    }

    pub const fn workspace(self) -> WorkspaceIdV1 {
        self.workspace
    }
    pub const fn epoch(self) -> u64 {
        self.epoch.get()
    }
    pub const fn revision(self) -> u64 {
        self.revision
    }
}

impl fmt::Debug for WorkspaceChangeCursorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WorkspaceChangeCursorV1(<opaque>)")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceChangeKindV1 {
    DocumentAdded(WorkspaceDocumentIdV1),
    DocumentRemoved(WorkspaceDocumentIdV1),
    RootAdded(WorkspaceRootIdV1),
    RootRemoved(WorkspaceRootIdV1),
    /// Content-free notification carrying only the editor's source revision.
    DocumentChanged {
        document: WorkspaceDocumentIdV1,
        source_revision: u64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkspaceChangeV1 {
    pub revision: NonZeroU64,
    pub kind: WorkspaceChangeKindV1,
}

impl WorkspaceChangeV1 {
    pub fn new(revision: u64, kind: WorkspaceChangeKindV1) -> Option<Self> {
        NonZeroU64::new(revision).map(|revision| Self { revision, kind })
    }

    pub fn revision(self) -> u64 {
        self.revision.get()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceChangePageV1 {
    pub workspace: WorkspaceIdV1,
    /// Head observed atomically with this page. Later changes may exist by the
    /// time a caller continues.
    pub observed_head: u64,
    pub changes: Vec<WorkspaceChangeV1>,
    /// Position immediately after the returned events (or the requested
    /// position for an empty page).
    pub next: WorkspaceChangeCursorV1,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceChangeGapV1 {
    pub workspace: WorkspaceIdV1,
    epoch: NonZeroU64,
    pub requested_after: u64,
    pub oldest_available: u64,
    pub observed_head: u64,
}

impl fmt::Debug for WorkspaceChangeGapV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorkspaceChangeGapV1")
            .field("workspace", &self.workspace)
            .field("epoch", &"<opaque>")
            .field("requested_after", &self.requested_after)
            .field("oldest_available", &self.oldest_available)
            .field("observed_head", &self.observed_head)
            .finish()
    }
}

impl WorkspaceChangeGapV1 {
    pub fn new(
        workspace: WorkspaceIdV1,
        epoch: u64,
        requested_after: u64,
        oldest_available: u64,
        observed_head: u64,
    ) -> Option<Self> {
        Some(Self {
            workspace,
            epoch: NonZeroU64::new(epoch)?,
            requested_after,
            oldest_available,
            observed_head,
        })
    }
    pub fn epoch(self) -> u64 {
        self.epoch.get()
    }
}

/// Polling is total over valid workspace-scoped positions: retention loss is
/// an explicit accepted gap, never a fabricated complete page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkspaceChangesV1 {
    Page(WorkspaceChangePageV1),
    Gap(WorkspaceChangeGapV1),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceRootKindV1 {
    File,
    Directory,
}

/// Exact result of opening the caller's workspace view.
///
/// Opening is idempotent: an already-open view retains its selected document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewOpenStatusV1 {
    Opened,
    AlreadyOpen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewOpenReceiptV1 {
    pub document: WorkspaceDocumentIdV1,
    pub status: ViewOpenStatusV1,
}

/// Exact result of closing the caller's workspace view.
///
/// A close releases only per-client selection state. It does not claim to
/// remove a document, root, native resource, history, witness, or save task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewCloseStatusV1 {
    Closed,
    AlreadyClosed,
}

/// Exact membership transition. `AlreadyRemoved` is distinct from an unknown
/// or foreign identity and is returned only for an identity retained by this
/// live service instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MembershipRemovalStatusV1 {
    Removed,
    AlreadyRemoved,
}

/// Exact result of admitting one file locator into workspace membership.
///
/// `AlreadyPresent` means the returned document identity was already a live
/// member. In that case no root was created. These identities are content-free
/// service projections, not filesystem locators or durable object names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileAdmissionStatusV1 {
    Added,
    AlreadyPresent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileAdmissionReceiptV1 {
    pub document: WorkspaceDocumentIdV1,
    pub root: Option<WorkspaceRootIdV1>,
    pub status: FileAdmissionStatusV1,
}

/// Exact bounded result of one serialized directory discovery and admission.
///
/// `added_documents` is the complete identity set introduced by this command,
/// in editor admission order. Its length must equal `added`; `joined` counts
/// discovered entries which resolved to membership that already existed or
/// was already staged by this admission. The receipt is historical evidence:
/// it does not claim those documents remain members after later commands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryAdmissionReceiptV1 {
    pub root: WorkspaceRootIdV1,
    pub added: u64,
    pub joined: u64,
    pub added_documents: Vec<WorkspaceDocumentIdV1>,
}

/// Workspace membership retirement, not native source disposal or file
/// deletion. The receipt deliberately omits revoked-witness cardinality so one
/// client cannot infer another client's retained activity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DocumentRemovalReceiptV1 {
    pub document: WorkspaceDocumentIdV1,
    pub status: MembershipRemovalStatusV1,
}

/// Root membership retirement never cascades to document membership.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RootRemovalReceiptV1 {
    pub root: WorkspaceRootIdV1,
    pub status: MembershipRemovalStatusV1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceRootV1 {
    pub id: WorkspaceRootIdV1,
    pub kind: WorkspaceRootKindV1,
}

/// Content-free result of resolving one caller-supplied exact locator against
/// current workspace membership. The locator is never retained in or projected
/// by this product. Both identities may be present for a file root; neither is
/// the single closed absent-or-foreign result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceLocatorResolutionV1 {
    pub workspace: WorkspaceIdV1,
    pub document: Option<WorkspaceDocumentIdV1>,
    pub root: Option<WorkspaceRootV1>,
}

/// Content-free inventory scoped to the connected live service instance.
///
/// These opaque identities are projections of live workspace identities. They
/// are neither filesystem locators nor restart-stable identifiers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceInventoryV1 {
    pub workspace: WorkspaceIdV1,
    pub documents: Vec<WorkspaceDocumentIdV1>,
    pub roots: Vec<WorkspaceRootV1>,
}

/// One bounded, content-free page from an exact inventory generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceInventoryPageV1 {
    pub workspace: WorkspaceIdV1,
    pub documents: Vec<WorkspaceDocumentIdV1>,
    pub roots: Vec<WorkspaceRootV1>,
    /// `None` is the canonical end outcome. A present cursor resumes at the
    /// first entry not included in this page.
    pub next: Option<WorkspaceInventoryCursorV1>,
}

/// One bounded byte-range replacement within a revision-witnessed document.
/// The expected source fragment is recovered from the retained witness by the
/// service; callers cannot weaken conflict detection by supplying it.
#[derive(Clone, PartialEq, Eq)]
pub struct WorkspaceReplacementV1 {
    pub start: u64,
    pub end: u64,
    pub inserted: String,
}

pub const MAX_WORKSPACE_EDIT_DOCUMENTS_V1: usize = 16;
pub const MAX_WORKSPACE_EDIT_REPLACEMENTS_V1: usize = 64;
pub const MAX_WORKSPACE_EDIT_PAYLOAD_BYTES_V1: usize = 65_536;

impl fmt::Debug for WorkspaceReplacementV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorkspaceReplacementV1")
            .field("start", &self.start)
            .field("end", &self.end)
            .field("inserted", &"<redacted>")
            .finish()
    }
}

/// One exact participant in an atomic in-memory workspace edit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceDocumentEditV1 {
    pub document: WorkspaceDocumentIdV1,
    pub expected_revision: u64,
    pub replacements: Vec<WorkspaceReplacementV1>,
}

/// The new exact snapshot for one participant in a committed workspace edit.
#[derive(Clone, PartialEq, Eq)]
pub struct WorkspaceEditedDocumentV1 {
    pub document: WorkspaceDocumentIdV1,
    pub snapshot: SnapshotV1,
}

impl fmt::Debug for WorkspaceEditedDocumentV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorkspaceEditedDocumentV1")
            .field("document", &self.document)
            .field("snapshot", &self.snapshot)
            .finish()
    }
}

/// Evidence from one atomic memory publication. This receipt makes no
/// filesystem persistence or cross-document durability claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceEditReceiptV1 {
    pub workspace: WorkspaceIdV1,
    pub generation: u64,
    pub documents: Vec<WorkspaceEditedDocumentV1>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum CommandV1 {
    WorkspaceInventory,
    ResolveLocator {
        locator: String,
    },
    WorkspaceChanges {
        after: Option<WorkspaceChangeCursorV1>,
        page_size: u16,
    },
    WorkspaceInventoryPage {
        cursor: Option<WorkspaceInventoryCursorV1>,
        page_size: u16,
    },
    OpenView,
    CloseView,
    AddFile {
        locator: String,
    },
    AddDirectory {
        locator: String,
    },
    RemoveDocument {
        document: WorkspaceDocumentIdV1,
    },
    RemoveRoot {
        root: WorkspaceRootIdV1,
    },
    SelectDocument {
        document: WorkspaceDocumentIdV1,
    },
    Save {
        expected: VersionWitness,
    },
    /// Applies one exact revision-witnessed replacement and then attempts an
    /// ordinary synchronized save. Edit and persistence authority are
    /// independent; the accepted receipt never erases an edit merely because
    /// persistence was refused, incomplete, or indeterminate.
    ReplaceAndSave {
        expected: VersionWitness,
        start: u64,
        end: u64,
        inserted: String,
    },
    Snapshot,
    Replace {
        expected: VersionWitness,
        start: u64,
        end: u64,
        inserted: String,
    },
    /// Atomically publishes all revision-witnessed replacements in memory.
    /// An empty batch, duplicate document, empty document edit, stale witness,
    /// or invalid range is refused before publication.
    WorkspaceEdit {
        documents: Vec<WorkspaceDocumentEditV1>,
    },
}

impl fmt::Debug for CommandV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkspaceInventory => f.write_str("WorkspaceInventory"),
            Self::ResolveLocator { locator: _ } => f
                .debug_struct("ResolveLocator")
                .field("locator", &"<redacted>")
                .finish(),
            Self::WorkspaceChanges { after, page_size } => f
                .debug_struct("WorkspaceChanges")
                .field("after", after)
                .field("page_size", page_size)
                .finish(),
            Self::WorkspaceInventoryPage { cursor, page_size } => f
                .debug_struct("WorkspaceInventoryPage")
                .field("cursor", cursor)
                .field("page_size", page_size)
                .finish(),
            Self::OpenView => f.write_str("OpenView"),
            Self::CloseView => f.write_str("CloseView"),
            Self::AddFile { locator: _ } => f
                .debug_struct("AddFile")
                .field("locator", &"<redacted>")
                .finish(),
            Self::AddDirectory { locator: _ } => f
                .debug_struct("AddDirectory")
                .field("locator", &"<redacted>")
                .finish(),
            Self::RemoveDocument { document } => f
                .debug_struct("RemoveDocument")
                .field("document", document)
                .finish(),
            Self::RemoveRoot { root } => f.debug_struct("RemoveRoot").field("root", root).finish(),
            Self::SelectDocument { document } => f
                .debug_struct("SelectDocument")
                .field("document", document)
                .finish(),
            Self::Save { expected } => f.debug_struct("Save").field("expected", expected).finish(),
            Self::ReplaceAndSave {
                expected,
                start,
                end,
                inserted: _,
            } => f
                .debug_struct("ReplaceAndSave")
                .field("expected", expected)
                .field("start", start)
                .field("end", end)
                .field("inserted", &"<redacted>")
                .finish(),
            Self::Snapshot => f.write_str("Snapshot"),
            Self::Replace {
                expected,
                start,
                end,
                inserted: _,
            } => f
                .debug_struct("Replace")
                .field("expected", expected)
                .field("start", start)
                .field("end", end)
                .field("inserted", &"<redacted>")
                .finish(),
            Self::WorkspaceEdit { documents } => f
                .debug_struct("WorkspaceEdit")
                .field("documents", &documents)
                .finish(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestV1 {
    pub client: ClientId,
    pub request: RequestId,
    pub command: CommandV1,
}

/// Restart-stable, explicit target and arguments for the closed durable-effect
/// profile. Session witnesses and implicit selected documents are deliberately
/// absent.
#[derive(Clone, PartialEq, Eq)]
pub enum DurableEffectIntentV1 {
    AddFile {
        locator: String,
    },
    AddDirectory {
        locator: String,
    },
    RemoveDocument {
        document: WorkspaceDocumentIdV1,
    },
    RemoveRoot {
        root: WorkspaceRootIdV1,
    },
    SaveDocument {
        document: WorkspaceDocumentIdV1,
        expected_revision: u64,
    },
    ReplaceDocumentAndSave {
        document: WorkspaceDocumentIdV1,
        expected_revision: u64,
        start: u64,
        end: u64,
        inserted: String,
    },
    /// Writes the exact source revision to a policy-qualified destination that
    /// must not already exist. This is a copy effect only: it cannot rebind the
    /// source document or admit the destination into the workspace.
    ExportDocumentCopyExclusive {
        document: WorkspaceDocumentIdV1,
        expected_revision: u64,
        destination: String,
    },
}

impl fmt::Debug for DurableEffectIntentV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AddFile { .. } => f
                .debug_struct("AddFile")
                .field("locator", &"<redacted>")
                .finish(),
            Self::AddDirectory { .. } => f
                .debug_struct("AddDirectory")
                .field("locator", &"<redacted>")
                .finish(),
            Self::RemoveDocument { document } => f
                .debug_struct("RemoveDocument")
                .field("document", document)
                .finish(),
            Self::RemoveRoot { root } => f.debug_struct("RemoveRoot").field("root", root).finish(),
            Self::SaveDocument {
                document,
                expected_revision,
            } => f
                .debug_struct("SaveDocument")
                .field("document", document)
                .field("expected_revision", expected_revision)
                .finish(),
            Self::ReplaceDocumentAndSave {
                document,
                expected_revision,
                start,
                end,
                ..
            } => f
                .debug_struct("ReplaceDocumentAndSave")
                .field("document", document)
                .field("expected_revision", expected_revision)
                .field("start", start)
                .field("end", end)
                .field("inserted", &"<redacted>")
                .finish(),
            Self::ExportDocumentCopyExclusive {
                document,
                expected_revision,
                ..
            } => f
                .debug_struct("ExportDocumentCopyExclusive")
                .field("document", document)
                .field("expected_revision", expected_revision)
                .field("destination", &"<redacted>")
                .finish(),
        }
    }
}

/// A fresh, session-correlated execution carrying a restart-stable operation
/// reference and explicit durable intent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DurableExecuteRequestV1 {
    pub operation: DurableOperationRefV1,
    pub client: ClientId,
    pub request: RequestId,
    pub intent: DurableEffectIntentV1,
}

/// Read-only durable receipt query. The command is repeated so the journal can
/// compare the exact canonical intent; no historical transport identity is
/// accepted by this algebra.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DurableReceiptLookupV1 {
    pub operation: DurableOperationRefV1,
    pub client: ClientId,
    pub request: RequestId,
    pub intent: DurableEffectIntentV1,
}

/// Read-only, content-free projection of one restart-stable operation's
/// current receipt state. The daemon binds workspace and authenticated subject;
/// the operation reference is correlation only and grants no mutation or
/// retirement authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DurableReceiptStatusRequestV1 {
    pub operation: DurableOperationRefV1,
    pub client: ClientId,
    pub request: RequestId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurableReceiptStatusV1 {
    Pending,
    Complete,
    Retired,
    AbsentOrForeign,
    Unavailable,
}

/// Exact, authenticated request to retire one completed durable receipt.
/// The stable intent is part of the target so changed-input reuse cannot erase
/// or alias an operation identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DurableReceiptRetirementV1 {
    pub operation: DurableOperationRefV1,
    pub client: ClientId,
    pub request: RequestId,
    pub intent: DurableEffectIntentV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurableReceiptRetirementOutcomeV1 {
    Retired,
    AbsentOrForeign,
    RequestConflict,
    OutcomeUnknown,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DurableSaveReceiptV1 {
    pub document: WorkspaceDocumentIdV1,
    pub source_revision: u64,
    pub persistence: SaveReceiptV1,
}

/// Restart-stable edit evidence. It deliberately carries no live
/// `VersionWitness`; `source_revision` and content describe the historical
/// result and cannot be presented as authority for a later edit.
#[derive(Clone, PartialEq, Eq)]
pub struct DurableReplaceAndSaveReceiptV1 {
    pub document: WorkspaceDocumentIdV1,
    pub source_revision: u64,
    pub text: Arc<str>,
    pub persistence: EditPersistenceV1,
}

/// Truthful crash-persistence classification for a create-exclusive export.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurableExportCopyPersistenceV1 {
    Durable,
    NotDurable,
    Unknown,
    NotAttempted,
}

/// Content- and locator-free physical failure classification for an export.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurableExportCopyFailureV1 {
    Conflict,
    Unsupported,
    Limit,
    Capacity,
    Io,
}

/// Restart-stable evidence for a create-exclusive copy export. The destination
/// and serialized source bytes are deliberately absent. Construction closes
/// the persistence/failure algebra: a durable result has no failure, while
/// every non-durable result names exactly one failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DurableExportCopyReceiptV1 {
    document: WorkspaceDocumentIdV1,
    source_revision: u64,
    persistence: DurableExportCopyPersistenceV1,
    failure: Option<DurableExportCopyFailureV1>,
    cleanup_failed: bool,
}

impl DurableExportCopyReceiptV1 {
    pub fn try_new(
        document: WorkspaceDocumentIdV1,
        source_revision: u64,
        persistence: DurableExportCopyPersistenceV1,
        failure: Option<DurableExportCopyFailureV1>,
        cleanup_failed: bool,
    ) -> Option<Self> {
        let valid = match persistence {
            DurableExportCopyPersistenceV1::Durable => failure.is_none() && !cleanup_failed,
            DurableExportCopyPersistenceV1::NotDurable
            | DurableExportCopyPersistenceV1::Unknown
            | DurableExportCopyPersistenceV1::NotAttempted => failure.is_some(),
        };
        valid.then_some(Self {
            document,
            source_revision,
            persistence,
            failure,
            cleanup_failed,
        })
    }

    pub const fn document(&self) -> WorkspaceDocumentIdV1 {
        self.document
    }

    pub const fn source_revision(&self) -> u64 {
        self.source_revision
    }

    pub const fn persistence(&self) -> DurableExportCopyPersistenceV1 {
        self.persistence
    }

    pub const fn failure(&self) -> Option<DurableExportCopyFailureV1> {
        self.failure
    }

    pub const fn cleanup_failed(&self) -> bool {
        self.cleanup_failed
    }
}

impl fmt::Debug for DurableReplaceAndSaveReceiptV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DurableReplaceAndSaveReceiptV1")
            .field("document", &self.document)
            .field("source_revision", &self.source_revision)
            .field("text", &"<redacted>")
            .field("persistence", &self.persistence)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DurableEffectReceiptV1 {
    FileAdmitted(FileAdmissionReceiptV1),
    DirectoryAdmitted(DirectoryAdmissionReceiptV1),
    DocumentRemoved(DocumentRemovalReceiptV1),
    RootRemoved(RootRemovalReceiptV1),
    DocumentSaved(DurableSaveReceiptV1),
    DocumentReplacedAndSaved(DurableReplaceAndSaveReceiptV1),
    DocumentCopyExportedExclusive(DurableExportCopyReceiptV1),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DurableExecuteOutcomeV1 {
    Completed(DurableEffectReceiptV1),
    Refused(RefusalV1),
    /// Exact operation identity was previously retired. This is absorbing and
    /// proves that no new effect occurred under the delayed attempt.
    Retired,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DurableReceiptLookupOutcomeV1 {
    Complete(DurableExecuteOutcomeV1),
    RequestConflict,
    OutcomeUnknown,
    AbsentOrForeign,
    Unavailable,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SnapshotV1 {
    pub witness: VersionWitness,
    pub revision: u64,
    pub text: Arc<str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveApplicationV1 {
    Unknown,
    NotWritten,
    Written,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveSynchronizationV1 {
    Unknown,
    NotRequested,
    Succeeded,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveReceiptV1 {
    pub application: SaveApplicationV1,
    pub synchronization: SaveSynchronizationV1,
    pub no_future_effects: bool,
    pub quiescent: bool,
    pub adapter_failure: bool,
    pub cleanup_failed: bool,
    pub baseline_published: bool,
    pub outcome_unknown: bool,
}

/// Closed persistence evidence for a replacement that has already been
/// applied to the live workspace.
///
/// `Durable` is intentionally conservative: the synchronized write and the
/// engine's exact baseline publication both completed. `NotDurable` preserves
/// definitive partial facts without claiming the bytes cannot exist.
/// `Unknown` means the adapter cannot close whether a durable effect occurred.
/// `NotAttempted` records the exact post-edit save-admission refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditPersistenceV1 {
    Durable(SaveReceiptV1),
    NotDurable(SaveReceiptV1),
    Unknown(SaveReceiptV1),
    NotAttempted(RefusalV1),
}

impl EditPersistenceV1 {
    /// Classifies one completed save receipt into the closed persistence
    /// evidence algebra shared by services and protocol projections.
    pub fn classify(receipt: SaveReceiptV1) -> Self {
        if receipt.outcome_unknown
            || receipt.application == SaveApplicationV1::Unknown
            || !receipt.no_future_effects
            || !receipt.quiescent
        {
            Self::Unknown(receipt)
        } else if receipt.application == SaveApplicationV1::Written
            && receipt.synchronization == SaveSynchronizationV1::Succeeded
            && !receipt.adapter_failure
            && !receipt.cleanup_failed
            && receipt.baseline_published
        {
            Self::Durable(receipt)
        } else {
            Self::NotDurable(receipt)
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ReplaceAndSaveReceiptV1 {
    pub snapshot: SnapshotV1,
    pub persistence: EditPersistenceV1,
}

impl fmt::Debug for ReplaceAndSaveReceiptV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReplaceAndSaveReceiptV1")
            .field("snapshot", &self.snapshot)
            .field("persistence", &self.persistence)
            .finish()
    }
}

impl fmt::Debug for SnapshotV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SnapshotV1")
            .field("witness", &self.witness)
            .field("revision", &self.revision)
            .field("text", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutcomeV1 {
    WorkspaceInventory(WorkspaceInventoryV1),
    LocatorResolved(WorkspaceLocatorResolutionV1),
    WorkspaceChanges(WorkspaceChangesV1),
    WorkspaceInventoryPage(WorkspaceInventoryPageV1),
    ViewOpened(ViewOpenReceiptV1),
    ViewClosed(ViewCloseStatusV1),
    FileAdded(WorkspaceInventoryV1),
    DirectoryAdded(WorkspaceInventoryV1),
    /// Exact successor to the legacy whole-inventory `FileAdded` projection.
    FileAdmitted(FileAdmissionReceiptV1),
    /// Exact successor to the legacy whole-inventory `DirectoryAdded`
    /// projection. The legacy variants remain decodable for V1 peers.
    DirectoryAdmitted(DirectoryAdmissionReceiptV1),
    DocumentRemoved(DocumentRemovalReceiptV1),
    RootRemoved(RootRemovalReceiptV1),
    DocumentSelected(WorkspaceDocumentIdV1),
    Saved(SaveReceiptV1),
    ReplacedAndSaved(ReplaceAndSaveReceiptV1),
    Snapshot(SnapshotV1),
    Replaced(SnapshotV1),
    WorkspaceEdited(WorkspaceEditReceiptV1),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefusalV1 {
    AuthorityDenied,
    AuthorityUnavailable,
    UnknownClient,
    ClientCapacity,
    ClientLifetimeCapacity,
    RequestCapacity,
    RequestConflict,
    WitnessCapacity,
    UnknownWitness,
    UnknownDocument,
    UnknownRoot,
    DocumentInUse,
    ForeignInventoryCursor,
    StaleInventoryCursor,
    InvalidInventoryCursor,
    ForeignChangeCursor,
    InvalidChangeCursor,
    NoActiveDocument,
    Stale,
    InvalidRange,
    InputLimit,
    SnapshotLimit,
    EditorClosed,
    EditorCapacity,
    EditorConflict,
    EditorIncomplete,
    EditorUnavailable,
    CounterExhausted,
}

/// Failures of access to a service interpreter, distinct from a semantic
/// refusal returned by that interpreter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceAccessErrorV1 {
    /// The interpreter did not receive any request bytes. A retry is safe
    /// with respect to this attempt, subject to the command's normal policy.
    NotSent,
    /// Some or all request bytes were transmitted, but no trustworthy reply
    /// was obtained. The operation may have happened and must not be retried
    /// under a new identity.
    OutcomeUnknown,
    Unavailable,
    Malformed,
    Limit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceControlErrorV1 {
    Refused(RefusalV1),
    Access(ServiceAccessErrorV1),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplyV1 {
    Accepted(OutcomeV1),
    Refused(RefusalV1),
}

/// Transport-neutral client port. Implementations may call an in-process
/// service or carry the same request/reply vocabulary over a qualified wire.
pub trait ServicePort {
    fn connect(&mut self) -> Result<ClientId, ServiceControlErrorV1>;
    fn execute(&mut self, request: RequestV1) -> Result<ReplyV1, ServiceAccessErrorV1>;
    fn disconnect(&mut self, client: ClientId) -> Result<(), ServiceControlErrorV1>;

    /// Executes a fresh durable effect. The default is closed so an older or
    /// in-process service cannot accidentally degrade to ordinary execution.
    fn execute_durable(
        &mut self,
        _request: DurableExecuteRequestV1,
    ) -> Result<DurableExecuteOutcomeV1, ServiceAccessErrorV1> {
        Err(ServiceAccessErrorV1::Unavailable)
    }

    /// Performs only historical evidence lookup. The default cannot call
    /// `execute`, create an intent, authorize, or mutate application state.
    fn lookup_durable_receipt(
        &mut self,
        _lookup: DurableReceiptLookupV1,
    ) -> Result<DurableReceiptLookupOutcomeV1, ServiceAccessErrorV1> {
        Ok(DurableReceiptLookupOutcomeV1::Unavailable)
    }

    /// Projects only current operation state. The default is closed and never
    /// falls through to lookup, execution, or retirement.
    fn durable_receipt_status(
        &mut self,
        _request: DurableReceiptStatusRequestV1,
    ) -> Result<DurableReceiptStatusV1, ServiceAccessErrorV1> {
        Ok(DurableReceiptStatusV1::Unavailable)
    }

    /// Retires only exact completed historical evidence. The default is
    /// closed; lookup acknowledgement or transport ownership is insufficient.
    fn retire_durable_receipt(
        &mut self,
        _retirement: DurableReceiptRetirementV1,
    ) -> Result<DurableReceiptRetirementOutcomeV1, ServiceAccessErrorV1> {
        Ok(DurableReceiptRetirementOutcomeV1::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_evidence_redacts_content_and_opaque_ids() {
        let snapshot = SnapshotV1 {
            witness: VersionWitness::new(9).unwrap(),
            revision: 4,
            text: Arc::from("private source"),
        };
        let debug = format!("{snapshot:?}");
        assert!(debug.contains("<redacted>"));
        assert!(debug.contains("<opaque>"));
        assert!(!debug.contains("private source"));
        assert!(!debug.contains('9'));

        let request = RequestV1 {
            client: ClientId::new(1).unwrap(),
            request: RequestId::new(2).unwrap(),
            command: CommandV1::Replace {
                expected: VersionWitness::new(3).unwrap(),
                start: 0,
                end: 0,
                inserted: "private insertion".into(),
            },
        };
        let debug = format!("{request:?}");
        assert!(!debug.contains("private insertion"));
        assert!(debug.contains("<redacted>"));

        let inventory = WorkspaceInventoryV1 {
            workspace: WorkspaceIdV1::new(17).unwrap(),
            documents: vec![
                WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(17).unwrap(), 19).unwrap(),
            ],
            roots: vec![WorkspaceRootV1 {
                id: WorkspaceRootIdV1::new(WorkspaceIdV1::new(17).unwrap(), 23).unwrap(),
                kind: WorkspaceRootKindV1::Directory,
            }],
        };
        let debug = format!("{inventory:?}");
        assert!(debug.contains("<opaque>"));
        assert!(!debug.contains("17"));
        assert!(!debug.contains("19"));
        assert!(!debug.contains("23"));

        let cursor = WorkspaceInventoryCursorV1::new(inventory.workspace, 29, 31).unwrap();
        let debug = format!("{cursor:?}");
        assert!(debug.contains("<opaque>"));
        assert!(!debug.contains("17"));
        assert!(!debug.contains("29"));
        assert!(!debug.contains("31"));

        for command in [
            CommandV1::ResolveLocator {
                locator: "/private/resolved".into(),
            },
            CommandV1::AddFile {
                locator: "/private/file".into(),
            },
            CommandV1::AddDirectory {
                locator: "/private/directory".into(),
            },
            CommandV1::WorkspaceEdit {
                documents: vec![WorkspaceDocumentEditV1 {
                    document: WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(17).unwrap(), 19)
                        .unwrap(),
                    expected_revision: 3,
                    replacements: vec![WorkspaceReplacementV1 {
                        start: 0,
                        end: 0,
                        inserted: "private workspace insertion".into(),
                    }],
                }],
            },
        ] {
            let debug = format!("{command:?}");
            assert!(debug.contains("<redacted>"));
            assert!(!debug.contains("/private"));
        }
    }
}
