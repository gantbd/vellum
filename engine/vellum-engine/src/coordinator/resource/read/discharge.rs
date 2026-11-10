//! Protected native cleanup. A trusted owner authorizes historical-result loss;
//! actual owned dependencies and delivery handles cannot be overridden by flags.
use super::*;

impl SourceCoordinator {
    fn safe_annotation_destination(&self, identity: &Identity) -> Result<(), ResourceError> {
        if self.resource_contested(identity) {
            return Err(ResourceError::Contested);
        }
        if self
            .resources
            .tasks
            .iter()
            .flatten()
            .any(|task| task.identity == *identity && (!task.safe() || !task.quiescent))
        {
            return Err(ResourceError::Retained);
        }
        Ok(())
    }

    fn loaded_read_referenced(&self, id: &AnnotationReadId) -> Result<bool, ResourceError> {
        self.validate_released_inventory()?;
        if self.resources.annotation_bindings.iter().any(|binding| {
            matches!(&binding.baseline, Some(AnnotationBaselineSlot::Loaded(b)) if b.fact.origin == *id)
        }) { return Ok(true); }
        if self.documents.iter().flatten().any(|document| {
            document.lifecycle.phase != LifecyclePhase::ProvenanceReleased
                && document.retained_bindings.annotations.as_ref().is_some_and(|binding|
                    matches!(&binding.baseline, Some(AnnotationBaselineSlot::Loaded(b)) if b.fact.origin == *id))
        }) { return Ok(true); }
        for task in self.resources.tasks.iter().flatten() {
            let artifact = self
                .resource_artifact(&task.artifact)
                .map_err(|_| ResourceError::Faulted)?;
            if artifact.loaded_condition.as_ref().is_some_and(|baseline| {
                baseline.fact.origin == *id && (!task.safe() || !task.quiescent)
            }) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn producing_read_release_remaining(
        &self,
        id: &AnnotationReadId,
    ) -> Result<usize, ResourceError> {
        let task = self.read_task(id)?;
        let provenance = task
            .restoration
            .as_ref()
            .ok_or(ResourceError::MissingEvidence)?;
        let capture = task.capture.as_ref().ok_or(ResourceError::Faulted)?;
        if !Arc::ptr_eq(capture, &provenance.package.capture.data)
            || provenance.package.capture.origin() != id
            || provenance.package.capture.source() != &task.source
            || provenance.outcome.source != task.source
            || provenance.outcome.context != provenance.package.registration.context
            || task.phase != ReadPhase::Submitted
            || !matches!(task.terminal, Some(ReadAtom::End { .. }))
        {
            return Err(ResourceError::Faulted);
        }
        if !task.quiescent
            || task
                .nonce
                .as_ref()
                .is_none_or(|nonce| Arc::strong_count(nonce) != 1)
            || Arc::strong_count(capture) != 2
        {
            return Err(ResourceError::Retained);
        }
        self.resources
            .held_read_bytes
            .checked_sub(task.charge)
            .ok_or(ResourceError::Faulted)
    }

    /// Protected producer retirement after explicit native provenance release.
    /// Neither disposal nor key release alone satisfies this root dependency.
    pub fn retire_disposed_annotation_read(
        &mut self,
        id: &AnnotationReadId,
        recorder: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let result = (|| {
            let task = self.read_task(id)?;
            let operation = self.annotation_read_operation_retirement(id)?;
            let provenance = task
                .restoration
                .as_ref()
                .ok_or(ResourceError::MissingEvidence)?;
            if !self.disposed_provenance_released(&task.source.document)? {
                return Err(ResourceError::Retained);
            }
            if let Some(attachment) = &self.resources.read_baseline_attachment {
                attachment.current(&self.namespace)?;
            } else if provenance.outcome.loaded_baseline_adopted {
                return Err(ResourceError::Faulted);
            }
            let owner = self
                .retained_document(&task.source.document)
                .map_err(owner_error)?;
            let archived = owner
                .retained_bindings
                .annotations
                .as_ref()
                .ok_or(ResourceError::Faulted)?;
            if archived.document != task.source.document
                || archived.key != task.association.binding
                || owner.retained_bindings.source.as_ref().map(Binding::key)
                    != task.association.source_binding
            {
                return Err(ResourceError::Faulted);
            }
            if self.loaded_read_referenced(id)? {
                return Err(ResourceError::Retained);
            }
            self.safe_annotation_destination(&task.identity)?;
            let remaining = self.producing_read_release_remaining(id)?;
            if let Some(index) = operation {
                self.spend_annotation_read_operation(index);
            }
            self.resources.reads[id.slot] = None;
            self.resources.held_read_bytes = remaining;
            if operation.is_some() {
                super::super::capture(
                    recorder,
                    Boundary::AnnotationReadOperationRetirement,
                    &Ok::<(), ResourceError>(()),
                );
            }
            Ok(())
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture(recorder, Boundary::DisposedReadRetirement, &result);
        result
    }

    /// Explicit trusted-owner retention discharge after a current independent
    /// acknowledged annotation checkpoint. Does not require a healthy mutation
    /// domain, spend a publication counter or release writer/artifact evidence.
    pub fn retire_superseded_annotation_read(
        &mut self,
        id: &AnnotationReadId,
        recorder: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let result = (|| {
            let task = self.read_task(id)?;
            let operation = self.annotation_read_operation_retirement(id)?;
            let provenance = task
                .restoration
                .as_ref()
                .ok_or(ResourceError::MissingEvidence)?;
            self.resources
                .read_baseline_attachment
                .as_ref()
                .ok_or(ResourceError::MissingEvidence)?
                .current(&self.namespace)?;
            if self.resource_contested(&task.identity) {
                return Err(ResourceError::Contested);
            }
            if self.loaded_read_referenced(id)? {
                return Err(ResourceError::Retained);
            }
            let document = self.document(&task.source.document).map_err(owner_error)?;
            let binding = self
                .resources
                .annotation_bindings
                .iter()
                .find(|b| b.document == task.source.document)
                .ok_or(ResourceError::Unbound)?;
            let baseline = binding
                .baseline
                .as_ref()
                .and_then(AnnotationBaselineSlot::written)
                .ok_or(ResourceError::MissingEvidence)?;
            self.validate_annotation_baseline(binding, baseline)?;
            if self
                .current_annotation_association(&task.source.document)
                .as_ref()
                != Some(&task.association)
                || baseline.association != task.association
                || baseline.fact.source.revision != document.revision
                || baseline.fact.set_version != document.annotation_generation
            {
                return Err(ResourceError::Stale);
            }
            let producer = self.resource_task(&baseline.fact.producer)?;
            if !producer.safe() || !producer.quiescent {
                return Err(ResourceError::Retained);
            }
            self.safe_annotation_destination(&task.identity)?;
            // The task and its retained restoration package are the only native
            // owners of this exact capture. Copies or receipts remain real holds.
            let capture = task.capture.as_ref().ok_or(ResourceError::Faulted)?;
            if !Arc::ptr_eq(capture, &provenance.package.capture.data) {
                return Err(ResourceError::Faulted);
            }
            if !task.quiescent
                || task
                    .nonce
                    .as_ref()
                    .is_none_or(|n| Arc::strong_count(n) != 1)
                || Arc::strong_count(capture) != 2
            {
                return Err(ResourceError::Retained);
            }
            let remaining = self
                .resources
                .held_read_bytes
                .checked_sub(task.charge)
                .ok_or(ResourceError::Faulted)?;
            // Current native history, snapshots and invocations contain authored
            // values/identities, not read-origin references. Source witnesses are
            // separately owned immutable captures; removing this package neither
            // retires nor edits them. New consumer families must join this guard.
            if let Some(index) = operation {
                self.spend_annotation_read_operation(index);
            }
            self.resources.reads[id.slot] = None;
            self.resources.held_read_bytes = remaining;
            if operation.is_some() {
                super::super::capture(
                    recorder,
                    Boundary::AnnotationReadOperationRetirement,
                    &Ok::<(), ResourceError>(()),
                );
            }
            Ok(())
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture(recorder, Boundary::AnnotationReadDischarge, &result);
        result
    }

    /// Remove both same-owner relation endpoints only after producing reads and
    /// live loaded launch dependencies have discharged. Unconsumed reads survive.
    pub fn detach_annotation_read_baselines(
        &mut self,
        recorder: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        let result = (|| {
            let Some(attachment) = &self.resources.read_baseline_attachment else {
                return Ok(false);
            };
            attachment.current(&self.namespace)?;
            let generation = self
                .resources
                .read_baseline_generation
                .checked_add(1)
                .ok_or(ResourceError::Faulted)?;
            if self
                .resources
                .reads
                .iter()
                .flatten()
                .any(|task| task.restoration.is_some())
                || self.documents.iter().flatten().any(|document| {
                    document.lifecycle.phase != LifecyclePhase::ProvenanceReleased
                        && document
                            .retained_bindings
                            .annotations
                            .as_ref()
                            .is_some_and(|binding| {
                                matches!(binding.baseline, Some(AnnotationBaselineSlot::Loaded(_)))
                            })
                })
                || self.resources.annotation_bindings.iter().any(|binding| {
                    matches!(binding.baseline, Some(AnnotationBaselineSlot::Loaded(_)))
                })
            {
                return Err(ResourceError::Retained);
            }
            for task in self.resources.tasks.iter().flatten() {
                let artifact = self
                    .resource_artifact(&task.artifact)
                    .map_err(|_| ResourceError::Faulted)?;
                if artifact.loaded_condition.is_some() {
                    self.safe_annotation_destination(&task.identity)?;
                }
            }
            let charge = std::mem::size_of::<ReadBaselineEndpoint>();
            let reader = self
                .resources
                .held_read_bytes
                .checked_sub(charge)
                .ok_or(ResourceError::Faulted)?;
            let writer = self
                .resources
                .held_artifact_bytes
                .checked_sub(charge)
                .ok_or(ResourceError::Faulted)?;
            self.resources.read_baseline_attachment = None;
            self.resources.read_baseline_generation = generation;
            self.resources.held_read_bytes = reader;
            self.resources.held_artifact_bytes = writer;
            Ok(true)
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture(recorder, Boundary::AnnotationReadDetach, &result);
        result
    }
}
