//! One synchronous engine authority and substrate module for multiple files.
//!
//! Calls disclose source/results to the embedding host. This facade is not safe
//! for untrusted callers without a separately enforced host authority boundary.
//! Engine versions, history groups and save tasks remain original opaque handles;
//! there is no string-ID registry, automatic retry or implicit conflict repair.
use vellum_engine::coordinator::{
    CommitOutcome, CoordinatorConfig, CoordinatorError, DocumentEdits, DocumentId, DocumentVersion,
    HistoryAvailability, HistoryDirection, HistoryError, HistoryGroupId, LifecycleState,
    SearchError, SearchFound, SearchNavigation, SearchRequest, SearchResult, SearchSelection,
    SearchSession, SearchUsage, SourceCoordinator, SourceRead, TrustedDocumentReclamationV1,
};
use vellum_engine::instrumentation::{NativeTimingProfileV1, Observation, Recorder};
use vellum_engine::resource::{
    OrdinaryPairEvidence, ProviderProfile, ResourceError, SaveId, SaveOutcome, SaveRequirements,
    SavedState,
};
use vellum_engine::source::Replacement;
use vellum_host::resource::{
    AdapterFailure, ClassifiedAdapterError, ResourceAdapter, ResourceAdapterFactory,
    WriteCompletion,
};
mod annotations;
pub use annotations::AnnotationCommit;
mod annotation_storage;
pub use annotation_storage::{AnnotationReadAttempt, AnnotationReadReport, AnnotationResource};
mod source_export;
pub use source_export::{
    AuthorizedSourceSaveAsPreparationV1, PreparationDischargeV1, RetainedSourceSaveAsErrorV1,
    RetainedSourceSaveAsStageV1, RetainedSourceSaveAsV1, SaveAsActivation, SourceExportAttempt,
    SourceExportCopyPreparationErrorV1, SourceExportPreparation, SourceSaveAsActivationPortV1,
    SourceSaveAsAdmissionPortV1, SourceSaveAsCompletionPortV1, SourceSaveAsObservationV1,
    SourceSaveAsPreparationDischargePortV1, SourceSaveAsPreparationErrorV1,
    SourceSaveAsPreparePortV1, SourceSaveAsServicePortV1, SourceSaveAsTokenSetV1,
};
mod invocation_claims;
mod invocations;
pub use invocation_claims::{
    InvocationClaimErrorV1, InvocationClaimKindV1, InvocationClaimUsageV1, InvocationClaimV1,
};
pub mod copy_transfer;
mod text_transfer;
pub use text_transfer::{
    TextTransferCaptureErrorV1, TextTransferCaptureLimitsV1, TextTransferCaptureUsageV1,
    TextTransferCaptureV1,
};
mod unicode;
mod views;

/// Caller-selected host limits; no implicit deployment budgets or SLOs.
pub struct SessionConfig<F> {
    pub engine: CoordinatorConfig,
    /// Optional explicit native view profile, not a UI or Unicode provider.
    pub logical_views: Option<vellum_engine::coordinator::LogicalViewLimits>,
    pub line_edits: Option<vellum_engine::coordinator::LineEditBudget>,
    /// Concrete resource construction selected by the composition root.
    pub resource_adapter: F,
    pub capture_capacity: usize,
    pub timing: NativeTimingProfileV1,
}

/// Content/path-free classifications. Errors never assert nonapplication of a
/// dispatched save; query its original task for the actual retained facts.
#[derive(Debug)]
pub enum SessionError {
    InvalidConfig,
    Capacity,
    WrongDocument,
    Engine(CoordinatorError),
    History(HistoryError),
    Search(SearchError),
    Resource(ResourceError),
    Adapter(AdapterFailure),
    AnnotationCodec(vellum_host::annotation::AnnotationCodecFailure),
    Serialization(vellum_engine::coordinator::SourceSerializationErrorV1),
    Selection(vellum_engine::selection::SelectionError),
    LineEdit(vellum_engine::coordinator::LineEditError),
    Invocation(vellum_engine::coordinator::InvocationError),
    InvocationClaim(InvocationClaimErrorV1),
    TextTransfer(TextTransferCaptureErrorV1),
    CopyTransfer(copy_transfer::CopyErrorV1),
    Boundary(vellum_engine::coordinator::BoundaryError),
    Unicode(vellum_engine::coordinator::UnicodeCommandError),
}
impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "local session: {self:?}")
    }
}
impl std::error::Error for SessionError {}
impl From<CoordinatorError> for SessionError {
    fn from(value: CoordinatorError) -> Self {
        Self::Engine(value)
    }
}
impl From<HistoryError> for SessionError {
    fn from(value: HistoryError) -> Self {
        Self::History(value)
    }
}
impl From<SearchError> for SessionError {
    fn from(value: SearchError) -> Self {
        Self::Search(value)
    }
}
impl From<ResourceError> for SessionError {
    fn from(value: ResourceError) -> Self {
        Self::Resource(value)
    }
}
impl From<vellum_engine::selection::SelectionError> for SessionError {
    fn from(value: vellum_engine::selection::SelectionError) -> Self {
        Self::Selection(value)
    }
}
impl From<vellum_engine::coordinator::LineEditError> for SessionError {
    fn from(value: vellum_engine::coordinator::LineEditError) -> Self {
        Self::LineEdit(value)
    }
}
impl From<vellum_engine::coordinator::InvocationError> for SessionError {
    fn from(value: vellum_engine::coordinator::InvocationError) -> Self {
        Self::Invocation(value)
    }
}

/// Adapter execution and engine evidence stay distinct. Even a failed
/// synchronization may accompany Written; cleanup failure must remain visible.
#[derive(Debug)]
pub struct SaveReport {
    pub outcome: SaveOutcome,
    pub failure: Option<AdapterFailure>,
    pub cleanup_failed: bool,
}

pub struct LocalHost<A: ResourceAdapter> {
    owner: SourceCoordinator,
    resource_adapter: A,
    diagnostics: Recorder,
    invocation_claims: invocation_claims::Claims,
    copy_transfers: Option<copy_transfer::CopyTransfers>,
}
/// Original native identities, not independent authority or a string-ID registry.
/// Clones cannot revive a disposed/reclaimed incarnation.
#[derive(Clone)]
pub struct LocalDocument<H> {
    pub(crate) resource: H,
    pub(crate) loaded: DocumentVersion,
}
impl<A: ResourceAdapter> LocalHost<A> {
    /// Configure one host for many files. Provider ownership stays exclusive to
    /// this trusted local embedding; paths never enter the semantic engine.
    pub fn new<F>(config: SessionConfig<F>) -> Result<Self, SessionError>
    where
        F: ResourceAdapterFactory<Adapter = A>,
    {
        if config.engine.resources.is_none() {
            return Err(SessionError::InvalidConfig);
        }
        let claim_limit = config
            .engine
            .invocation
            .map_or(0, |limits| limits.max_retained);
        let mut diagnostics = Recorder::try_with_capacity(config.capture_capacity)
            .map_err(|_| SessionError::Capacity)?
            .with_native_timing(config.timing);
        let mut owner = SourceCoordinator::configured(config.engine);
        if let Some(limits) = config.logical_views {
            owner.configure_logical_views(limits, &mut diagnostics)?;
        }
        if let Some(budget) = config.line_edits {
            owner.configure_line_edits(budget, &mut diagnostics)?;
        }
        let port = owner.register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            None,
            &mut diagnostics,
        )?;
        let resource_adapter = config
            .resource_adapter
            .build(port)
            .map_err(|error| SessionError::Adapter(error.classification()))?;
        Ok(Self {
            owner,
            resource_adapter,
            diagnostics,
            invocation_claims: invocation_claims::Claims::new(claim_limit),
            copy_transfers: None,
        })
    }

    /// Existing supported files only. Native adapter bindings are bounded by
    /// caller limits; disposal does not reset lifetime engine/adapter budgets.
    pub fn open(&mut self, locator: &A::Locator) -> Result<LocalDocument<A::Handle>, SessionError> {
        let (resource, read) = self
            .resource_adapter
            .open(locator, &mut self.diagnostics)
            .map_err(|error| SessionError::Adapter(error.classification()))?;
        let loaded = self.owner.load_resource(read, &mut self.diagnostics)?;
        Ok(LocalDocument { resource, loaded })
    }

    pub fn read(
        &mut self,
        document: &LocalDocument<A::Handle>,
    ) -> Result<SourceRead<'_>, SessionError> {
        Ok(self
            .owner
            .read(&document.loaded.document, &mut self.diagnostics)?)
    }

    /// Trusted host source-editing policy, not OS permissions or a principal
    /// grant inferred from handle possession. Native publishers enforce it.
    pub fn set_editing_locked(
        &mut self,
        document: &LocalDocument<A::Handle>,
        locked: bool,
    ) -> Result<(), SessionError> {
        Ok(self
            .owner
            .set_read_only(&document.loaded.document, locked, &mut self.diagnostics)?)
    }

    /// Exact original-snapshot byte coordinates. No UI selection policy or
    /// normalization is introduced; stale and foreign versions refuse.
    pub fn edit(
        &mut self,
        expected: &DocumentVersion,
        edits: Vec<Replacement>,
    ) -> Result<CommitOutcome, SessionError> {
        self.edit_batch(
            &[DocumentEdits {
                expected: expected.clone(),
                edits,
            }],
            &[],
        )
    }

    /// One existing shared publisher, including linked history and declared
    /// read dependencies. This is atomic memory publication, not multi-file I/O.
    pub fn edit_batch(
        &mut self,
        writes: &[DocumentEdits],
        reads: &[DocumentVersion],
    ) -> Result<CommitOutcome, SessionError> {
        let prepared = self.owner.prepare(writes, reads, &mut self.diagnostics)?;
        Ok(self.owner.commit(prepared, &mut self.diagnostics)?)
    }

    /// Reverses the source frontier, never a wrapper-owned stack. Annotation
    /// frontiers are selected explicitly with `reverse_annotation`.
    pub fn reverse(
        &mut self,
        document: &LocalDocument<A::Handle>,
        direction: HistoryDirection,
    ) -> Result<bool, SessionError> {
        let status = self.owner.history_status(
            &document.loaded.document,
            direction,
            &mut self.diagnostics,
        )?;
        self.reverse_available(status, direction)
    }

    fn reverse_available(
        &mut self,
        status: HistoryAvailability,
        direction: HistoryDirection,
    ) -> Result<bool, SessionError> {
        let group = match status {
            HistoryAvailability::Empty => return Ok(false),
            HistoryAvailability::Eligible(group) => group,
            HistoryAvailability::Blocked(_) => return Err(HistoryError::Blocked.into()),
            HistoryAvailability::Unavailable => return Err(HistoryError::Unavailable.into()),
        };
        let prepared = self
            .owner
            .prepare_reversal(&group, direction, &mut self.diagnostics)?;
        self.owner.commit(prepared, &mut self.diagnostics)?;
        Ok(true)
    }

    /// Original engine search owner. Requires caller-configured search limits;
    /// no fallback scan or independent result registry is created by this SDK.
    pub fn open_search(
        &mut self,
        document: &LocalDocument<A::Handle>,
    ) -> Result<SearchSession, SessionError> {
        Ok(self
            .owner
            .open_search(&document.loaded.document, &mut self.diagnostics)?)
    }

    /// Exact UTF-8 literal search against the supplied original source version.
    /// Results disclose content-derived ranges to this trusted embedding, not
    /// to an implicitly authorized principal. Partial status is preserved.
    pub fn search(
        &mut self,
        session: &SearchSession,
        expected: &DocumentVersion,
        request: &SearchRequest,
    ) -> Result<SearchResult, SessionError> {
        Ok(self
            .owner
            .search(session, expected, request, &mut self.diagnostics)?)
    }

    /// Revalidates the original native result; never refreshes stale matches.
    pub fn navigate_search(
        &mut self,
        result: &SearchResult,
        navigation: SearchNavigation<'_>,
    ) -> Result<Option<SearchFound>, SessionError> {
        Ok(self
            .owner
            .navigate_search(result, navigation, &mut self.diagnostics)?)
    }

    /// One native prepare/commit and linked history group. Replacement is
    /// literal data, not regex/template syntax. Incomplete all-result selection,
    /// stale results and policy refusal retain the engine's existing meaning.
    pub fn replace_search(
        &mut self,
        selected: &SearchSelection,
        replacement: &str,
    ) -> Result<CommitOutcome, SessionError> {
        let prepared =
            self.owner
                .prepare_search_replace(selected, replacement, &mut self.diagnostics)?;
        Ok(self.owner.commit(prepared, &mut self.diagnostics)?)
    }

    /// Invalidates action on the original session. Caller-held result/occurrence
    /// captures must also be dropped before their retention can be discharged.
    pub fn close_search(&mut self, session: &SearchSession) -> Result<(), SessionError> {
        Ok(self.owner.close_search(session, &mut self.diagnostics)?)
    }

    pub fn search_usage(&self) -> SearchUsage {
        self.owner.search_usage()
    }

    /// Admission only. Retain this handle even if service later fails: an error
    /// cannot erase the attempt, prove nonapplication or authorize a retry.
    pub fn queue_save(
        &mut self,
        expected: &DocumentVersion,
        requirements: SaveRequirements,
    ) -> Result<SaveId, SessionError> {
        let pair = self.ordinary_pair(&expected.document)?;
        Ok(match pair {
            Some(pair) => self.owner.queue_pinned_save_with_pair(
                expected,
                requirements,
                pair,
                &mut self.diagnostics,
            )?,
            None => self
                .owner
                .queue_pinned_save(expected, requirements, &mut self.diagnostics)?,
        })
    }

    /// Probe, dispatch and synchronous native execution once, then ingest each
    /// separately attributed receipt. Repeated service is not a second write.
    /// The original task remains queryable/control-capable after any error.
    pub fn service_save(
        &mut self,
        document: &LocalDocument<A::Handle>,
        task: &SaveId,
    ) -> Result<SaveReport, SessionError> {
        self.service_resource(&document.loaded.document, &document.resource, task)
    }

    fn ordinary_pair(
        &mut self,
        document: &DocumentId,
    ) -> Result<Option<OrdinaryPairEvidence>, SessionError> {
        self.owner
            .ordinary_pair_check(document, &mut self.diagnostics)?
            .map(|check| {
                self.resource_adapter
                    .compare_ordinary_pair(&check, &mut self.diagnostics)
                    .map_err(|error| SessionError::Adapter(error.classification()))
            })
            .transpose()
    }

    fn service_resource(
        &mut self,
        document: &DocumentId,
        resource: &A::Handle,
        task: &SaveId,
    ) -> Result<SaveReport, SessionError> {
        Ok(self
            .service_resource_observed(document, resource, task, false)?
            .0)
    }
    fn service_resource_observed(
        &mut self,
        document: &DocumentId,
        resource: &A::Handle,
        task: &SaveId,
        observe_written: bool,
    ) -> Result<(SaveReport, Option<vellum_engine::resource::ResourceRead>), SessionError> {
        let outcome = self.owner.save_outcome(task, &mut self.diagnostics)?;
        if outcome.captured.document != *document {
            return Err(SessionError::WrongDocument);
        }
        let (read, probe) = self
            .resource_adapter
            .probe(resource, &mut self.diagnostics)
            .map_err(|error| SessionError::Adapter(error.classification()))?;
        let pair = self.ordinary_pair(document)?;
        let dispatch = match pair {
            Some(pair) => {
                self.owner
                    .dispatch_save_with_pair(task, read, pair, &mut self.diagnostics)?
            }
            None => self
                .owner
                .dispatch_save(task, read, &mut self.diagnostics)?,
        };
        let mut completion = if observe_written {
            self.resource_adapter
                .write_observed(dispatch, probe, &mut self.diagnostics)
                .map_err(|error| SessionError::Adapter(error.classification()))?
        } else {
            self.resource_adapter
                .write(dispatch, probe, &mut self.diagnostics)
                .map_err(|error| SessionError::Adapter(error.classification()))?
        };
        let written = completion.written_observation.take();
        Ok((self.finish_write(task, completion)?, written))
    }

    fn finish_write(
        &mut self,
        task: &SaveId,
        completion: WriteCompletion<A::Error>,
    ) -> Result<SaveReport, SessionError> {
        let failure = completion
            .failure
            .as_ref()
            .map(ClassifiedAdapterError::classification);
        let cleanup_failed = completion.cleanup_failed;
        self.owner
            .record_save_evidence(completion.result, &mut self.diagnostics)?;
        self.owner
            .record_save_evidence(completion.no_future_effects, &mut self.diagnostics)?;
        self.owner
            .record_save_evidence(completion.quiescence, &mut self.diagnostics)?;
        Ok(SaveReport {
            outcome: self.owner.save_outcome(task, &mut self.diagnostics)?,
            failure,
            cleanup_failed,
        })
    }

    /// Local baseline publication, not an additional filesystem acknowledgment.
    pub fn publish_save_baseline(&mut self, task: &SaveId) -> Result<(), SessionError> {
        let outcome = self.owner.save_outcome(task, &mut self.diagnostics)?;
        let pair = self.ordinary_pair(&outcome.captured.document)?;
        match pair {
            Some(pair) => {
                self.owner
                    .publish_save_baseline_with_pair(task, pair, &mut self.diagnostics)?
            }
            None => self
                .owner
                .publish_save_baseline(task, &mut self.diagnostics)?,
        };
        Ok(())
    }
    pub fn save_outcome(&mut self, task: &SaveId) -> Result<SaveOutcome, SessionError> {
        Ok(self.owner.save_outcome(task, &mut self.diagnostics)?)
    }
    pub fn saved_state(
        &mut self,
        document: &LocalDocument<A::Handle>,
    ) -> Result<SavedState, SessionError> {
        Ok(self
            .owner
            .saved_state(&document.loaded.document, &mut self.diagnostics)?)
    }
    pub fn stop_save(&mut self, task: &SaveId) -> Result<(), SessionError> {
        Ok(self.owner.stop_save(task, &mut self.diagnostics)?)
    }
    /// Can refuse Retained: the current baseline or unresolved obligations still
    /// own the task. Never treat Drop or stop intent as successful retirement.
    pub fn retire_save(&mut self, task: &SaveId) -> Result<(), SessionError> {
        Ok(self.owner.retire_save(task, &mut self.diagnostics)?)
    }
    /// Separate explicit artifact discharge. Superseded annotation task
    /// retirement alone need not release its original capture/payload owner.
    pub fn retire_resource_artifact(
        &mut self,
        artifact: &vellum_engine::resource::ResourceArtifactId,
    ) -> Result<(), SessionError> {
        Ok(self
            .owner
            .retire_resource_artifact(artifact, &mut self.diagnostics)?)
    }
    pub fn pop_diagnostic(&mut self) -> Option<Observation> {
        self.diagnostics.pop_front()
    }
    pub fn dropped_diagnostics(&self) -> u64 {
        self.diagnostics.dropped()
    }
    /// Native logical ownership/queue gauges, not physical RSS, authorization or
    /// a cleanup receipt. Keep read, writer and baseline populations distinct.
    pub fn resource_usage(&self) -> vellum_engine::resource::ResourceUsage {
        self.owner.resource_usage()
    }

    pub fn lifecycle(
        &mut self,
        document: &LocalDocument<A::Handle>,
    ) -> Result<LifecycleState, SessionError> {
        Ok(self
            .owner
            .lifecycle_state(&document.loaded.document, &mut self.diagnostics)?)
    }

    /// Explicit discard of editing authority, not a save, erasure or worker stop.
    /// Exact current source and annotation-set revisions are checked by the owner.
    pub fn dispose(&mut self, expected: &DocumentVersion) -> Result<LifecycleState, SessionError> {
        let set = self
            .owner
            .annotation_set_version(&expected.document, &mut self.diagnostics)?;
        let plan = self
            .owner
            .prepare_retained_disposal(expected, set, &mut self.diagnostics)?;
        Ok(self
            .owner
            .commit_retained_disposal(plan, &mut self.diagnostics)?)
    }

    /// Provenance release after bindings/history have been explicitly discharged.
    /// Retained outcomes and actual producer obligations can still block cleanup.
    pub fn release_provenance(
        &mut self,
        document: &LocalDocument<A::Handle>,
        expected: LifecycleState,
    ) -> Result<LifecycleState, SessionError> {
        self.owner.release_disposed_provenance(
            &document.loaded.document,
            expected,
            &mut self.diagnostics,
        )?;
        self.lifecycle(document)
    }

    /// Explicitly release the disposed document's source binding. This does not
    /// release history or retained results and is not file deletion.
    pub fn release_source_binding(
        &mut self,
        document: &LocalDocument<A::Handle>,
        expected: LifecycleState,
    ) -> Result<LifecycleState, SessionError> {
        self.owner.release_disposed_source_binding(
            &document.loaded.document,
            expected,
            &mut self.diagnostics,
        )?;
        self.lifecycle(document)
    }

    /// Discards the identified linked history group at the actual owner. A group
    /// can span files; callers must deliberately authorize that loss of history.
    pub fn retire_history(&mut self, group: &HistoryGroupId) -> Result<usize, SessionError> {
        Ok(self
            .owner
            .retire_history_group(group, &mut self.diagnostics)?)
    }
    pub fn history_usage(&self) -> vellum_engine::coordinator::HistoryUsage {
        self.owner.history_usage()
    }

    /// Host premises are supplied, never invented here. The actual native owner
    /// still checks pins, lifetimes and obligations before final reclamation.
    pub fn reclaim(
        &mut self,
        document: &LocalDocument<A::Handle>,
        expected: LifecycleState,
        facts: &TrustedDocumentReclamationV1,
    ) -> Result<(), SessionError> {
        Ok(self.owner.reclaim_disposed_document_v1(
            &document.loaded.document,
            expected,
            facts,
            &mut self.diagnostics,
        )?)
    }

    /// Transfer all owners to the embedding for advanced engine operations,
    /// explicit disposition/shutdown and protected retirement. This is not a
    /// completion receipt; the returned original owners retain all obligations.
    /// Outstanding integration claims or copy-provider registrations refuse
    /// unpacking and return this intact host. Discharge them explicitly before
    /// retrying; dropping is not discharge.
    #[allow(clippy::result_large_err)] // Preserve owners without allocating on refusal.
    pub fn into_parts(self) -> Result<(SourceCoordinator, A, Recorder), Self> {
        if self.invocation_claims.retained() != 0
            || self
                .copy_transfers
                .as_ref()
                .is_some_and(|copies| copies.has_obligations())
        {
            return Err(self);
        }
        Ok((self.owner, self.resource_adapter, self.diagnostics))
    }
}
