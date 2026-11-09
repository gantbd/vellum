//! Initial retained disposal/binding release. No live rebind or implicit purge.
use super::*;
#[cfg(test)]
mod tests;
impl SourceCoordinator {
    pub(super) fn validate_released_inventory(&self) -> Result<(), ResourceError> {
        for (slot, owner) in self.documents.iter().enumerate() {
            if owner
                .as_ref()
                .is_some_and(|owner| owner.lifecycle.phase == LifecyclePhase::ProvenanceReleased)
            {
                self.disposed_provenance_released(&DocumentId {
                    namespace: self.namespace.clone(),
                    slot,
                })?;
            }
        }
        Ok(())
    }
    pub(in crate::coordinator) fn disposed_provenance_released(
        &self,
        document: &DocumentId,
    ) -> Result<bool, ResourceError> {
        let owner = self.retained_document(document).map_err(owner_error)?;
        if owner.lifecycle.phase != LifecyclePhase::ProvenanceReleased {
            return Ok(false);
        }
        if owner.lifecycle.generation == 0
            || self
                .resources
                .bindings
                .iter()
                .any(|binding| binding.document == *document)
            || self
                .resources
                .annotation_bindings
                .iter()
                .any(|binding| binding.document == *document)
            || self.history_retains_document(document)
            || self
                .resources
                .source_export_reservations
                .iter()
                .any(|r| r.document == *document)
        {
            return Err(ResourceError::Faulted);
        }
        Ok(true)
    }
    /// Explicit privileged release of this disposed root's provenance pins.
    /// Immutable captures/outcomes remain separately owned; this is not payload
    /// erasure, producer retirement, worker quiescence or recipient authorization.
    pub fn release_disposed_provenance(
        &mut self,
        document: &DocumentId,
        expected: LifecycleState,
        recorder: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        self.release_disposed_provenance_for_invocation(document, expected, None, recorder)
    }
    fn release_disposed_provenance_for_invocation(
        &mut self,
        document: &DocumentId,
        expected: LifecycleState,
        invocation: Option<&InvocationId>,
        recorder: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        let timing = recorder.begin_service_timing();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.ensure_operational().map_err(owner_error)?;
            let owner = self.retained_document(document).map_err(owner_error)?;
            if owner.lifecycle != expected {
                return Err(ResourceError::Stale);
            }
            if expected.phase == LifecyclePhase::Live {
                return Err(ResourceError::Unsupported);
            }
            if self
                .resources
                .bindings
                .iter()
                .any(|binding| binding.document == *document)
                || self
                    .resources
                    .annotation_bindings
                    .iter()
                    .any(|binding| binding.document == *document)
                || self.history_retains_document(document)
                || self
                    .resources
                    .source_export_reservations
                    .iter()
                    .any(|r| r.document == *document)
            {
                return Err(ResourceError::Retained);
            }
            if expected.phase == LifecyclePhase::ProvenanceReleased {
                self.disposed_provenance_released(document)?;
                if let Some(id) = invocation {
                    self.stage_retained_release_candidate(
                        id,
                        document,
                        expected,
                        RetainedReleaseOperationV1::Provenance,
                        false,
                    )
                    .map_err(|_| ResourceError::Faulted)?;
                }
                return Ok(false);
            }
            self.validate_disposal_obligations(document)?;
            for identity in owner
                .retained_bindings
                .source
                .iter()
                .chain(owner.retained_bindings.source_history.iter())
                .map(|binding| &binding.identity)
                .chain(
                    owner
                        .retained_bindings
                        .annotations
                        .iter()
                        .map(|binding| &binding.key.identity),
                )
            {
                if self.resource_contested(identity) {
                    return Err(ResourceError::Contested);
                }
            }
            if self
                .resources
                .tasks
                .iter()
                .flatten()
                .any(|task| task.captured.document == *document && !task.safe())
            {
                return Err(ResourceError::Retained);
            }
            let next = LifecycleState {
                phase: LifecyclePhase::ProvenanceReleased,
                generation: expected
                    .generation
                    .checked_add(1)
                    .ok_or(ResourceError::VersionExhausted)?,
                ..expected
            };
            let publication = self
                .next_retention_publication_generation(None)
                .map_err(owner_error)?;
            if let Some(id) = invocation {
                self.stage_retained_release_candidate(
                    id,
                    document,
                    expected,
                    RetainedReleaseOperationV1::Provenance,
                    true,
                )
                .map_err(|_| ResourceError::Faulted)?;
            }
            self.faulted = true;
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(0),
                "injected lifecycle publication interruption"
            );
            self.documents[document.slot]
                .as_mut()
                .expect("checked retained owner")
                .lifecycle = next;
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(1),
                "injected lifecycle publication interruption"
            );
            self.generation = publication;
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(2),
                "injected lifecycle publication interruption"
            );
            self.faulted = false;
            Ok(true)
        }));
        let result = match result {
            Ok(result) => result,
            Err(panic) => {
                self.faulted = true;
                capture_service(
                    recorder,
                    Boundary::DisposedProvenanceRelease,
                    &Err::<bool, _>(ResourceError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(panic)
            }
        };
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(
            recorder,
            Boundary::DisposedProvenanceRelease,
            &result,
            timing.finish(),
        );
        result
    }
    pub(in crate::coordinator) fn validate_disposal_obligations(
        &self,
        document: &DocumentId,
    ) -> Result<(), ResourceError> {
        if self
            .resources
            .bindings
            .iter()
            .filter(|binding| binding.document == *document)
            .count()
            > 1
            || self
                .resources
                .annotation_bindings
                .iter()
                .filter(|binding| binding.document == *document)
                .count()
                > 1
        {
            return Err(ResourceError::Faulted);
        }
        if self.resources.tasks.iter().flatten().any(|task| {
            task.captured.document == *document
                && task
                    .baseline_claim
                    .as_ref()
                    .is_some_and(|claim| claim.state == BaselineCapacityState::Pending)
        }) {
            return Err(ResourceError::Retained);
        }
        Ok(())
    }
    pub fn release_disposed_source_binding(
        &mut self,
        document: &DocumentId,
        expected: LifecycleState,
        recorder: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        self.release_disposed_binding(document, expected, ArtifactKind::Source, None, recorder)
    }
    pub fn release_disposed_annotation_binding(
        &mut self,
        document: &DocumentId,
        expected: LifecycleState,
        recorder: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        self.release_disposed_binding(
            document,
            expected,
            ArtifactKind::Annotations,
            None,
            recorder,
        )
    }
    pub(in crate::coordinator) fn release_retained_for_invocation(
        &mut self,
        id: &InvocationId,
        request: &RetainedReleaseRequestV1,
        recorder: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        match request.operation {
            RetainedReleaseOperationV1::SourceBinding => self.release_disposed_binding(
                &request.document,
                request.expected,
                ArtifactKind::Source,
                Some(id),
                recorder,
            ),
            RetainedReleaseOperationV1::AnnotationBinding => self.release_disposed_binding(
                &request.document,
                request.expected,
                ArtifactKind::Annotations,
                Some(id),
                recorder,
            ),
            RetainedReleaseOperationV1::Provenance => self
                .release_disposed_provenance_for_invocation(
                    &request.document,
                    request.expected,
                    Some(id),
                    recorder,
                ),
        }
    }
    fn release_disposed_binding(
        &mut self,
        document: &DocumentId,
        expected: LifecycleState,
        kind: ArtifactKind,
        invocation: Option<&InvocationId>,
        recorder: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        let timing = recorder.begin_service_timing();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.ensure_operational().map_err(owner_error)?;
            let owner = self.retained_document(document).map_err(owner_error)?;
            if owner.lifecycle != expected {
                return Err(ResourceError::Stale);
            }
            if expected.phase != LifecyclePhase::DisposedRetained {
                return Err(ResourceError::Unsupported);
            }
            self.validate_disposal_obligations(document)?;
            let (index, identity, already_retained) = match kind {
                ArtifactKind::Source => {
                    let binding = self
                        .resources
                        .bindings
                        .iter()
                        .enumerate()
                        .find(|(_, b)| b.document == *document);
                    (
                        binding.map(|(i, _)| i),
                        binding.map(|(_, b)| &b.identity),
                        owner.retained_bindings.source.is_some(),
                    )
                }
                ArtifactKind::Annotations => {
                    let binding = self
                        .resources
                        .annotation_bindings
                        .iter()
                        .enumerate()
                        .find(|(_, b)| b.document == *document);
                    (
                        binding.map(|(i, _)| i),
                        binding.map(|(_, b)| &b.key.identity),
                        owner.retained_bindings.annotations.is_some(),
                    )
                }
            };
            let Some(index) = index else {
                if let Some(id) = invocation {
                    self.stage_retained_release_candidate(
                        id,
                        document,
                        expected,
                        match kind {
                            ArtifactKind::Source => RetainedReleaseOperationV1::SourceBinding,
                            ArtifactKind::Annotations => {
                                RetainedReleaseOperationV1::AnnotationBinding
                            }
                        },
                        false,
                    )
                    .map_err(|_| ResourceError::Faulted)?;
                }
                return Ok(false);
            };
            if already_retained {
                return Err(ResourceError::Faulted);
            }
            let identity = identity.ok_or(ResourceError::Faulted)?;
            // The active owner reservation must be exact and unique across roles.
            let owners = self
                .resources
                .bindings
                .iter()
                .filter(|b| b.identity == *identity)
                .count()
                + self
                    .resources
                    .annotation_bindings
                    .iter()
                    .filter(|b| b.key.identity == *identity)
                    .count();
            if owners != 1 {
                return Err(ResourceError::Faulted);
            }
            if self.resource_contested(identity) {
                return Err(ResourceError::Contested);
            }
            if self
                .resources
                .tasks
                .iter()
                .flatten()
                .any(|task| task.identity == *identity && !task.safe())
            {
                return Err(ResourceError::Retained);
            }
            let next = LifecycleState {
                generation: expected
                    .generation
                    .checked_add(1)
                    .ok_or(ResourceError::VersionExhausted)?,
                pair_generation: expected
                    .pair_generation
                    .checked_add(1)
                    .ok_or(ResourceError::VersionExhausted)?,
                ..expected
            };
            let publication = self
                .next_retention_publication_generation(None)
                .map_err(owner_error)?;
            if let Some(id) = invocation {
                self.stage_retained_release_candidate(
                    id,
                    document,
                    expected,
                    match kind {
                        ArtifactKind::Source => RetainedReleaseOperationV1::SourceBinding,
                        ArtifactKind::Annotations => RetainedReleaseOperationV1::AnnotationBinding,
                    },
                    true,
                )
                .map_err(|_| ResourceError::Faulted)?;
            }
            self.faulted = true;
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(0),
                "injected lifecycle publication interruption"
            );
            let owner = self.documents[document.slot]
                .as_mut()
                .expect("retained owner");
            match kind {
                ArtifactKind::Source => {
                    owner.retained_bindings.source = Some(self.resources.bindings.remove(index))
                }
                ArtifactKind::Annotations => {
                    owner.retained_bindings.annotations =
                        Some(self.resources.annotation_bindings.remove(index))
                }
            }
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(1),
                "injected lifecycle publication interruption"
            );
            owner.lifecycle = next;
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(2),
                "injected lifecycle publication interruption"
            );
            self.generation = publication;
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(3),
                "injected lifecycle publication interruption"
            );
            self.faulted = false;
            Ok(true)
        }));
        let result = match result {
            Ok(result) => result,
            Err(panic) => {
                self.faulted = true;
                capture_service(
                    recorder,
                    Boundary::DisposedBindingRelease,
                    &Err::<bool, _>(ResourceError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(panic)
            }
        };
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(
            recorder,
            Boundary::DisposedBindingRelease,
            &result,
            timing.finish(),
        );
        result
    }
}
