//! Source-owner birth only: no view, workspace, persistence, history edit or I/O.
use super::*;
#[cfg(test)]
mod tests;
pub(super) struct PreparedDocumentBirth {
    document: Document,
    pub(super) version: DocumentVersion,
    generation: u64,
    growth: usize,
}
impl PreparedDocumentBirth {
    pub(super) fn validate(
        &self,
        owner: &SourceCoordinator,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        owner.ensure_operational()?;
        if self.version.document.slot != owner.documents.len()
            || self.generation != owner.next_publication_generation(None)?
        {
            return Err(CoordinatorError::Stale);
        }
        owner.check_root_growth(self.growth, r)?;
        Ok(())
    }
    /// Caller holds the fault fence through the entire composite publication.
    pub(super) fn install_fenced(self, owner: &mut SourceCoordinator) {
        debug_assert!(owner.faulted);
        #[cfg(test)]
        assert_ne!(
            owner.publication_fault_after,
            Some(0),
            "injected document birth interruption"
        );
        owner.documents.push(Some(self.document));
        #[cfg(test)]
        assert_ne!(
            owner.publication_fault_after,
            Some(1),
            "injected document birth interruption"
        );
        owner.generation = self.generation;
        #[cfg(test)]
        assert_ne!(
            owner.publication_fault_after,
            Some(2),
            "injected document birth interruption"
        );
    }
}
pub struct ScratchDocumentRequestV1<'a> {
    /// Strict logical scalar UTF-8. No file-signature interpretation or normalization.
    pub source: &'a [u8],
    pub serialization: SourceSerializationV1,
    pub insertion_policy: GeneratedNewlineV1,
}
impl SourceCoordinator {
    /// Privileged native owner constructor, not recipient-authorized ingress.
    /// Output fidelity may require refusing a later encoding; never rewrite text
    /// to make the selected serialization possible.
    pub fn create_scratch_document_v1(
        &mut self,
        q: &ScratchDocumentRequestV1<'_>,
        r: &mut Recorder,
    ) -> Result<DocumentVersion, CoordinatorError> {
        self.create_scratch_document_for_invocation(q, None, r)
    }
    pub(super) fn preflight_scratch_document(
        &self,
        q: &ScratchDocumentRequestV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        self.ensure_operational()?;
        if self.documents.len() >= self.limits.max_document_incarnations {
            return Err(CoordinatorError::Limit);
        }
        match q.serialization.profile {
            SourceSerializationProfileV1::StrictUtf8OneBom => {}
        }
        if q.source.len() > self.limits.source.max_source_bytes {
            return Err(CoordinatorError::Source(SourceError::SourceLimit));
        }
        let growth = std::mem::size_of::<Option<Document>>()
            .checked_add(q.source.len())
            .ok_or(CoordinatorError::Limit)?;
        self.check_root_growth(growth, r)?;
        self.next_publication_generation(None)?;
        Ok(())
    }
    pub(super) fn create_scratch_document_for_invocation(
        &mut self,
        q: &ScratchDocumentRequestV1<'_>,
        invocation: Option<&InvocationId>,
        r: &mut Recorder,
    ) -> Result<DocumentVersion, CoordinatorError> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.preflight_scratch_document(q, r)?;
            let source = Utf8Source::from_logical_utf8_observed(
                q.source,
                self.limits.source.max_source_bytes,
                r,
            )
            .map_err(CoordinatorError::Source)?;
            // Scratch's specified generation-one logical birth is distinct
            // from the retained revision/set/lifecycle-zero import baseline.
            let document = Document {
                source,
                serialization: SourceSerializationStateV1 {
                    generation: 1,
                    serialization: q.serialization,
                },
                insertion_policy: q.insertion_policy,
                lifecycle: LifecycleState {
                    phase: LifecyclePhase::Live,
                    generation: 1,
                    pair_generation: 1,
                },
                retained_bindings: resource::RetainedBindings::default(),
                revision: 1,
                read_only: false,
                policy_generation: 0,
                history: history::Frontiers::default(),
                annotation_generation: 1,
                annotation_policy: 0,
                annotation_read_only: false,
            };
            self.install_document_birth_for_invocation(document, 0, invocation, r)
        }));
        let result = match caught {
            Ok(v) => v,
            Err(p) => {
                self.faulted = true;
                observe_service(
                    r,
                    Boundary::ScratchCreation,
                    &Err::<DocumentVersion, _>(CoordinatorError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(p)
            }
        };
        if matches!(result, Err(CoordinatorError::Faulted)) {
            self.faulted = true
        }
        observe_service(r, Boundary::ScratchCreation, &result, timing.finish());
        result
    }
    /// Shared allocator/root installation for logical scratch and byte import.
    /// All fallible checks/capacity precede the callback-free installation.
    pub(super) fn install_document_birth(
        &mut self,
        document: Document,
        additional_root_bytes: usize,
        r: &mut Recorder,
    ) -> Result<DocumentVersion, CoordinatorError> {
        self.install_document_birth_for_invocation(document, additional_root_bytes, None, r)
    }
    fn install_document_birth_for_invocation(
        &mut self,
        document: Document,
        additional_root_bytes: usize,
        invocation: Option<&InvocationId>,
        r: &mut Recorder,
    ) -> Result<DocumentVersion, CoordinatorError> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let prepared = self.prepare_document_birth(document, additional_root_bytes, r)?;
            let version = prepared.version.clone();
            if let Some(id) = invocation {
                self.stage_scratch_candidate(id, &version, &prepared.document)
                    .map_err(|_| CoordinatorError::Faulted)?;
            }
            self.faulted = true;
            prepared.install_fenced(self);
            self.faulted = false;
            Ok(version)
        }));
        let result = match caught {
            Ok(v) => v,
            Err(p) => {
                self.faulted = true;
                observe_service(
                    r,
                    Boundary::DocumentBirthPublication,
                    &Err::<DocumentVersion, _>(CoordinatorError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(p)
            }
        };
        if matches!(result, Err(CoordinatorError::Faulted)) {
            self.faulted = true
        }
        observe_service(
            r,
            Boundary::DocumentBirthPublication,
            &result,
            timing.finish(),
        );
        result
    }
    pub(super) fn prepare_document_birth(
        &mut self,
        document: Document,
        additional_root_bytes: usize,
        r: &mut Recorder,
    ) -> Result<PreparedDocumentBirth, CoordinatorError> {
        self.ensure_operational()?;
        if self.documents.len() >= self.limits.max_document_incarnations
            || document.source.text().len() > self.limits.source.max_source_bytes
        {
            return Err(CoordinatorError::Limit);
        }
        let growth = std::mem::size_of::<Option<Document>>()
            .checked_add(document.source.text().len())
            .and_then(|n| n.checked_add(additional_root_bytes))
            .ok_or(CoordinatorError::Limit)?;
        self.check_root_growth(growth, r)?;
        let generation = self.next_publication_generation(None)?;
        self.documents
            .try_reserve(1)
            .map_err(|_| CoordinatorError::Capacity)?;
        let version = DocumentVersion {
            document: DocumentId {
                namespace: self.namespace.clone(),
                slot: self.documents.len(),
            },
            revision: document.revision,
        };
        Ok(PreparedDocumentBirth {
            document,
            version,
            generation,
            growth,
        })
    }
}
