//! Presentation-free workspace authority for services and headless embeddings.
//!
//! A `Workspace` owns documents and history publication. An `EditorView` is a
//! client-owned projection that selects a document but carries no authority to
//! mutate it. Cursors, selections, viewport, and other presentation state can
//! extend the view without entering the shared owner.

use std::path::PathBuf;
use vellum_engine::{
    coordinator::{DocumentVersion, HistoryGroupId},
    source::Replacement,
};
use vellum_host::resource::{ErasedPathResourceAdapter, ErasedResourceHandle};
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
    Session(SessionError),
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

/// One client's non-authoritative projection into a shared workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorView {
    active_document: usize,
}

impl EditorView {
    pub fn active_document(&self) -> usize {
        self.active_document
    }
}

/// Immutable source capture. It is disclosure, not write authority.
pub struct WorkspaceSnapshot {
    pub version: DocumentVersion,
    pub text: String,
}

impl std::fmt::Debug for WorkspaceSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WorkspaceSnapshot(<redacted>)")
    }
}

/// Shared document and history authority with no presentation state.
pub struct Workspace {
    host: LocalHost,
    documents: Vec<LocalDocument>,
    groups: Vec<HistoryGroupId>,
}

impl Workspace {
    pub fn open(paths: &[PathBuf], config: WorkspaceConfig) -> Result<Self, WorkspaceError> {
        if paths.is_empty() {
            return Err(WorkspaceError::Empty);
        }
        let session = config.into_session();
        if paths.len() > session.engine.limits.max_document_incarnations {
            return Err(WorkspaceError::Capacity);
        }
        let mut host = LocalHost::new(session)?;
        let mut documents = Vec::new();
        documents
            .try_reserve_exact(paths.len())
            .map_err(|_| WorkspaceError::Capacity)?;
        for path in paths {
            let document = host.open(path)?;
            let identity = host.read(&document)?.version.document;
            let mut duplicate = false;
            for retained in &documents {
                if host.read(retained)?.version.document == identity {
                    duplicate = true;
                    break;
                }
            }
            if !duplicate {
                documents.push(document);
            }
        }
        Ok(Self {
            host,
            documents,
            groups: Vec::new(),
        })
    }

    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    pub fn open_view(&self) -> Result<EditorView, WorkspaceError> {
        if self.documents.is_empty() {
            return Err(WorkspaceError::Empty);
        }
        Ok(EditorView { active_document: 0 })
    }

    pub fn select_document(
        &self,
        view: &mut EditorView,
        document: usize,
    ) -> Result<(), WorkspaceError> {
        if document >= self.documents.len() {
            return Err(WorkspaceError::InvalidView);
        }
        view.active_document = document;
        Ok(())
    }

    pub fn version(&mut self, view: &EditorView) -> Result<DocumentVersion, WorkspaceError> {
        let document = self.document(view)?.clone();
        Ok(self.host.read(&document)?.version)
    }

    pub fn snapshot(&mut self, view: &EditorView) -> Result<WorkspaceSnapshot, WorkspaceError> {
        let index = view.active_document;
        let document = self
            .documents
            .get(index)
            .ok_or(WorkspaceError::InvalidView)?;
        let read = self.host.read(document)?;
        let mut text = String::new();
        text.try_reserve_exact(read.source.text().len())
            .map_err(|_| WorkspaceError::Capacity)?;
        text.push_str(read.source.text());
        Ok(WorkspaceSnapshot {
            version: read.version,
            text,
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

    fn document(&self, view: &EditorView) -> Result<&LocalDocument, WorkspaceError> {
        self.documents
            .get(view.active_document)
            .ok_or(WorkspaceError::InvalidView)
    }
}
