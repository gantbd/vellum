//! Single-resource convenience over the same multi-document host. Applications
//! use `LocalHost` directly rather than allocate an engine for every resource.
use crate::host::{LocalDocument, LocalHost};
pub use crate::host::{SaveReport, SessionConfig, SessionError};
use vellum_engine::coordinator::{
    CommitOutcome, DocumentVersion, HistoryDirection, SourceCoordinator, SourceRead,
};
use vellum_engine::instrumentation::{Observation, Recorder};
use vellum_engine::resource::{SaveId, SaveOutcome, SaveRequirements};
use vellum_engine::source::Replacement;
use vellum_sdk::resource::{ResourceAdapter, ResourceAdapterFactory};

pub struct LocalSession<A: ResourceAdapter> {
    host: LocalHost<A>,
    document: LocalDocument<A::Handle>,
}
impl<A: ResourceAdapter> LocalSession<A> {
    pub fn open<F>(locator: &A::Locator, config: SessionConfig<F>) -> Result<Self, SessionError>
    where
        F: ResourceAdapterFactory<Adapter = A>,
    {
        let mut host = LocalHost::new(config)?;
        let document = host.open(locator)?;
        Ok(Self { host, document })
    }
    pub fn read(&mut self) -> Result<SourceRead<'_>, SessionError> {
        self.host.read(&self.document)
    }
    pub fn edit(
        &mut self,
        expected: &DocumentVersion,
        edits: Vec<Replacement>,
    ) -> Result<CommitOutcome, SessionError> {
        self.check_document(expected)?;
        self.host.edit(expected, edits)
    }
    pub fn reverse(&mut self, direction: HistoryDirection) -> Result<bool, SessionError> {
        self.host.reverse(&self.document, direction)
    }
    pub fn queue_save(
        &mut self,
        expected: &DocumentVersion,
        requirements: SaveRequirements,
    ) -> Result<SaveId, SessionError> {
        self.check_document(expected)?;
        self.host.queue_save(expected, requirements)
    }
    pub fn service_save(&mut self, task: &SaveId) -> Result<SaveReport, SessionError> {
        self.host.service_save(&self.document, task)
    }
    pub fn publish_save_baseline(&mut self, task: &SaveId) -> Result<(), SessionError> {
        self.host.publish_save_baseline(task)
    }
    pub fn save_outcome(&mut self, task: &SaveId) -> Result<SaveOutcome, SessionError> {
        self.host.save_outcome(task)
    }
    pub fn stop_save(&mut self, task: &SaveId) -> Result<(), SessionError> {
        self.host.stop_save(task)
    }
    pub fn retire_save(&mut self, task: &SaveId) -> Result<(), SessionError> {
        self.host.retire_save(task)
    }
    pub fn pop_diagnostic(&mut self) -> Option<Observation> {
        self.host.pop_diagnostic()
    }
    pub fn dropped_diagnostics(&self) -> u64 {
        self.host.dropped_diagnostics()
    }
    /// The returned version is the original load capture, not a refreshed version.
    /// Refusal preserves the entire session, including any integration claims.
    #[allow(clippy::result_large_err)]
    pub fn into_parts(
        self,
    ) -> Result<(SourceCoordinator, A, A::Handle, DocumentVersion, Recorder), Self> {
        let (owner, filesystem, diagnostics) = match self.host.into_parts() {
            Ok(parts) => parts,
            Err(host) => {
                return Err(Self {
                    host,
                    document: self.document,
                });
            }
        };
        Ok((
            owner,
            filesystem,
            self.document.resource,
            self.document.loaded,
            diagnostics,
        ))
    }
    fn check_document(&self, expected: &DocumentVersion) -> Result<(), SessionError> {
        if expected.document != self.document.loaded.document {
            return Err(SessionError::WrongDocument);
        }
        Ok(())
    }
}
