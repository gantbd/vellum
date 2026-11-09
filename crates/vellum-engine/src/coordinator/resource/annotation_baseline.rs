//! Explicit historical adoption and current exact-output comparison. No I/O or
//! recipient authorization is inferred. This is not ordinary-save reservation.
use super::*;
impl Task {
    fn annotation_baseline(&self, producer: &SaveId) -> Result<AnnotationBaseline, ResourceError> {
        let association = self
            .annotation_association
            .clone()
            .ok_or(ResourceError::Unbound)?;
        Ok(AnnotationBaseline {
            fact: AnnotationBaselineFact {
                producer: producer.clone(),
                artifact: self.artifact.clone(),
                source: self.captured.clone(),
                set_version: self.captured_annotation_set.ok_or(ResourceError::Faulted)?,
                encoding_context: self
                    .encoding_context
                    .clone()
                    .ok_or(ResourceError::Faulted)?,
                ordinary_binding_generation: association.binding.generation,
                source_binding_generation: association
                    .source_binding
                    .as_ref()
                    .map(|b| b.generation),
                dispatch_serial: self.dispatch_serial.ok_or(ResourceError::Faulted)?,
                required_acknowledgment: if self.synchronize {
                    Acknowledgment::Synchronized
                } else {
                    Acknowledgment::Ordinary
                },
                qualified_acknowledgment: if self.knowledge.data.synchronization
                    == Synchronization::Succeeded
                {
                    Acknowledgment::Synchronized
                } else {
                    Acknowledgment::Ordinary
                },
                external_version_available: self.knowledge.data.version.is_some(),
            },
            association,
            bytes: self.payload.clone(),
            external_version: self.knowledge.data.version.clone(),
        })
    }
}

#[cfg(test)]
mod tests;
impl AnnotationBaseline {
    /// Complete fixed native descriptor plus variable referenced payload/key/token
    /// bytes, not a JSON wire-envelope or global allocator accounting claim.
    pub(super) fn retained_size(&self) -> Option<usize> {
        std::mem::size_of::<AnnotationBaselineSlot>()
            .checked_add(self.bytes.0.len())?
            .checked_add(self.external_version.as_ref().map_or(0, |b| b.0.len()))?
            .checked_add(self.association.binding.identity.key.0.len())?
            .checked_add(
                self.association
                    .source_binding
                    .as_ref()
                    .map_or(0, |b| b.identity.key.0.len()),
            )
    }
}
impl SourceCoordinator {
    pub(super) fn current_annotation_association(
        &self,
        document: &DocumentId,
    ) -> Option<AnnotationAssociation> {
        self.resources
            .annotation_bindings
            .iter()
            .find(|b| b.document == *document)
            .map(|b| AnnotationAssociation {
                binding: b.key.clone(),
                source_binding: self
                    .resources
                    .bindings
                    .iter()
                    .find(|s| s.document == *document)
                    .map(Binding::key),
            })
    }
    fn newer_annotation_producer(&self, task: &Task) -> bool {
        self.resources.tasks.iter().flatten().any(|other| {
            other.captured.document == task.captured.document
                && other.annotation_association == task.annotation_association
                && other.annotation_association.is_some()
                && other.dispatch_serial > task.dispatch_serial
                && other.qualified()
        })
    }
    pub(super) fn annotation_producer_retained(&self, id: &SaveId, task: &Task) -> bool {
        self.documents.iter().flatten().any(|document| {
            document.lifecycle.phase != LifecyclePhase::ProvenanceReleased
                && document
                    .retained_bindings
                    .annotations
                    .as_ref()
                    .and_then(|binding| binding.baseline.as_ref())
                    .and_then(AnnotationBaselineSlot::written)
                    .is_some_and(|baseline| baseline.fact.producer == *id)
        }) || self.resources.annotation_bindings.iter().any(|b| {
            b.baseline
                .as_ref()
                .and_then(AnnotationBaselineSlot::written)
                .is_some_and(|baseline| baseline.fact.producer == *id)
        }) || (task.annotation_association.is_some()
            && task.qualified()
            && self.current_annotation_association(&task.captured.document)
                == task.annotation_association
            && !self.newer_annotation_producer(task))
    }
    pub(super) fn validate_annotation_baseline(
        &self,
        binding: &AnnotationBinding,
        baseline: &AnnotationBaseline,
    ) -> Result<(), ResourceError> {
        let task = self
            .resource_task(&baseline.fact.producer)
            .map_err(|_| ResourceError::Faulted)?;
        let artifact = self
            .resource_artifact(&baseline.fact.artifact)
            .map_err(|_| ResourceError::Faulted)?;
        if self.resource_contested(&binding.key.identity) {
            return Err(ResourceError::Contested);
        }
        let ArtifactBasis::Annotations { snapshot, encoder } = &artifact.basis else {
            return Err(ResourceError::Faulted);
        };
        self.encoder_current(encoder)?;
        self.provider_current(&binding.key.registration)?;
        if task.kind != ArtifactKind::Annotations
            || !task.qualified()
            || task.artifact != baseline.fact.artifact
            || task.captured != baseline.fact.source
            || task.captured.document != binding.document
            || baseline.association.binding != binding.key
            || task.identity != binding.key.identity
            || !Arc::ptr_eq(&task.registration, &binding.key.registration)
            || baseline.fact.ordinary_binding_generation != baseline.association.binding.generation
            || baseline.fact.source_binding_generation
                != baseline
                    .association
                    .source_binding
                    .as_ref()
                    .map(|b| b.generation)
            || task.annotation_association.as_ref() != Some(&baseline.association)
            || task.dispatch_serial != Some(baseline.fact.dispatch_serial)
            || task.captured_annotation_set != Some(baseline.fact.set_version)
            || task.encoding_context.as_ref() != Some(&baseline.fact.encoding_context)
            || snapshot.source() != &baseline.fact.source
            || snapshot.set_version() != baseline.fact.set_version
            || encoder.context != baseline.fact.encoding_context
            || task.payload != baseline.bytes
            || artifact.bytes != baseline.bytes
            || baseline.fact.external_version_available != baseline.external_version.is_some()
            || baseline
                .external_version
                .as_ref()
                .is_some_and(|v| task.knowledge.data.version.as_ref() != Some(v))
            || (baseline.fact.required_acknowledgment == Acknowledgment::Synchronized)
                != task.synchronize
            || (baseline.fact.qualified_acknowledgment == Acknowledgment::Synchronized
                && task.knowledge.data.synchronization != Synchronization::Succeeded)
            || (baseline.fact.required_acknowledgment == Acknowledgment::Synchronized
                && baseline.fact.qualified_acknowledgment != Acknowledgment::Synchronized)
        {
            return Err(ResourceError::Faulted);
        }
        Ok(())
    }
    /// Explicit export may succeed even when this independent adoption has no
    /// capacity. The existing binding preallocates the fixed native baseline slot.
    pub fn publish_annotation_baseline(
        &mut self,
        id: &SaveId,
        pair: Option<OrdinaryPairEvidence>,
        max_baseline_bytes: usize,
        recorder: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        let timing = recorder.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            let task = self.resource_task(id)?;
            self.writer_catalog_current(
                task.catalog.as_ref(),
                &task.registration,
                &task.identity,
                task.synchronize,
                false,
                recorder,
            )?;
            if task.kind != ArtifactKind::Annotations {
                return Err(ResourceError::Unsupported);
            }
            let association = task
                .annotation_association
                .as_ref()
                .ok_or(ResourceError::Unbound)?;
            self.document(&task.captured.document)
                .map_err(owner_error)?;
            self.validate_ordinary_pair(&task.captured.document, pair.as_ref())?;
            if self
                .current_annotation_association(&task.captured.document)
                .as_ref()
                != Some(association)
            {
                return Err(ResourceError::Stale);
            }
            if self.resource_contested(&task.identity) {
                return Err(ResourceError::Contested);
            }
            if !task.qualified() {
                return Err(ResourceError::AcknowledgmentUnsatisfied);
            }
            if self.newer_annotation_producer(task) {
                return Err(ResourceError::Superseded);
            }
            let index = self
                .resources
                .annotation_bindings
                .iter()
                .position(|b| b.document == task.captured.document)
                .ok_or(ResourceError::Unbound)?;
            if let Some(old) = &self.resources.annotation_bindings[index].baseline {
                self.validate_annotation_baseline_slot(
                    &self.resources.annotation_bindings[index],
                    old,
                )?;
            }
            let baseline = task.annotation_baseline(id)?;
            self.validate_annotation_baseline(
                &self.resources.annotation_bindings[index],
                &baseline,
            )?;
            if baseline
                .retained_size()
                .is_none_or(|n| n > max_baseline_bytes)
            {
                return Err(ResourceError::Limit);
            }
            if self.resources.annotation_bindings[index]
                .baseline
                .as_ref()
                .and_then(AnnotationBaselineSlot::written)
                .is_some_and(|old| {
                    old.fact == baseline.fact
                        && old.external_version == baseline.external_version
                        && old.bytes == baseline.bytes
                })
            {
                return Ok(false);
            }
            let pending = task
                .baseline_claim
                .as_ref()
                .is_some_and(|c| c.state == BaselineCapacityState::Pending);
            if pending
                && baseline
                    .retained_size()
                    .is_none_or(|n| n > task.baseline_claim.as_ref().expect("checked claim").charge)
            {
                return Err(ResourceError::Faulted);
            }
            let generation = self
                .next_publication_generation(pending.then_some(id))
                .map_err(owner_error)?;
            let slot = AnnotationBaselineSlot::Written(baseline);
            let removed = self.resources.annotation_bindings[index]
                .baseline
                .as_ref()
                .map_or(Ok(0), AnnotationBaselineSlot::root_payload_bytes)
                .map_err(|_| ResourceError::Limit)?;
            let added = slot
                .root_payload_bytes()
                .map_err(|_| ResourceError::Limit)?;
            self.check_root_replacement_projected(
                removed,
                added,
                BaselineRootProjection {
                    changed: Some((&task.captured.document, added)),
                    source_changed: None,
                    consume: pending.then_some(id),
                    admission: None,
                },
                recorder,
            )
            .map_err(owner_error)?;
            self.resources.annotation_bindings[index].baseline = Some(slot);
            if pending {
                self.resources.tasks[id.slot]
                    .as_mut()
                    .expect("retained producer")
                    .baseline_claim
                    .as_mut()
                    .expect("checked claim")
                    .state = BaselineCapacityState::Installed;
            }
            self.generation = generation;
            Ok(true)
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(
            recorder,
            Boundary::AnnotationBaselinePublication,
            &result,
            timing.finish(),
        );
        result
    }
    /// Written-only historical metadata; no raw provider version is released.
    /// A Loaded slot returns Unsupported, not an apparent absence. Use the typed
    /// annotation_baseline_evidence facet when both producer kinds are needed.
    pub fn annotation_baseline(
        &mut self,
        document: &DocumentId,
        pair: Option<OrdinaryPairEvidence>,
        recorder: &mut Recorder,
    ) -> Result<Option<AnnotationBaselineFact>, ResourceError> {
        let timing = recorder.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            self.document(document).map_err(owner_error)?;
            self.validate_ordinary_pair(document, pair.as_ref())?;
            let Some(binding) = self
                .resources
                .annotation_bindings
                .iter()
                .find(|b| b.document == *document)
            else {
                return Ok(None);
            };
            let Some(baseline) = &binding.baseline else {
                return Ok(None);
            };
            self.validate_annotation_baseline_slot(binding, baseline)?;
            Ok(Some(
                baseline
                    .written()
                    .ok_or(ResourceError::Unsupported)?
                    .fact
                    .clone(),
            ))
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(
            recorder,
            Boundary::AnnotationBaselineQuery,
            &result,
            timing.finish(),
        );
        result
    }
    /// Exact historical output equality, external freshness not established. The
    /// current candidate stays transient: no writer/artifact issuance or root edit.
    /// As with retained captures, complete producer validation is a host premise.
    pub fn compare_annotation_baseline(
        &mut self,
        document: &DocumentId,
        current: &EncodedAnnotations,
        pair: Option<OrdinaryPairEvidence>,
        max_combined_bytes: usize,
        recorder: &mut Recorder,
    ) -> Result<ContentComparison, ResourceError> {
        let timing = recorder.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            let doc = self.document(document).map_err(owner_error)?;
            self.validate_ordinary_pair(document, pair.as_ref())?;
            let Some(binding) = self
                .resources
                .annotation_bindings
                .iter()
                .find(|b| b.document == *document)
            else {
                return Ok(ContentComparison::Unbound);
            };
            let baseline = binding
                .baseline
                .as_ref()
                .ok_or(ResourceError::MissingEvidence)?;
            self.validate_annotation_baseline_slot(binding, baseline)?;
            self.encoder_current(&current.registration)?;
            if !self.knows_annotation_snapshot(&current.snapshot) {
                return Err(ResourceError::Unavailable);
            }
            if current.snapshot.source().document != *document
                || current.snapshot.source().revision != doc.revision
                || current.snapshot.set_version() != doc.annotation_generation
                || &current.registration.context != baseline.encoding_context()
            {
                return Err(ResourceError::Stale);
            }
            if current
                .bytes
                .0
                .len()
                .checked_add(baseline.bytes().0.len())
                .is_none_or(|n| n > max_combined_bytes)
            {
                return Err(ResourceError::Limit);
            }
            Ok(if &current.bytes == baseline.bytes() {
                ContentComparison::Unchanged
            } else {
                ContentComparison::Modified
            })
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(
            recorder,
            Boundary::AnnotationComparison,
            &result,
            timing.finish(),
        );
        result
    }
}
