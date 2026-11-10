//! Active, disposed and archived binding obligations preserve original producers.
//! This inspection never reinterprets an installed contested baseline as absent.
use super::*;
#[cfg(test)]
pub(super) mod tests;
use crate::coordinator::shutdown::inventory::{
    Builder, ShutdownBaselineOriginV1 as Origin, ShutdownBindingIdV1 as Id,
    ShutdownBindingLocationV1 as Location, ShutdownBindingRecordV1 as Row,
    ShutdownDestinationIdV1 as Destination, ShutdownInventoryTargetV1 as Target,
};
impl SourceCoordinator {
    fn shutdown_binding_row(
        &self,
        document: &DocumentId,
        key: &BindingKey,
        kind: ArtifactKind,
        location: Location,
    ) -> Result<Row, CoordinatorError> {
        self.retained_document(document)?;
        self.provider_current(&key.registration)
            .map_err(|_| CoordinatorError::Faulted)?;
        if key.identity.domain != key.registration.domain {
            return Err(CoordinatorError::Faulted);
        }
        Ok(Row {
            binding: Id {
                document: document.clone(),
                kind,
                generation: key.generation,
            },
            document: document.clone(),
            kind,
            generation: key.generation,
            location,
            destination: Destination(key.identity.clone()),
            provider: ReadProviderContext {
                registration: key.registration.clone(),
            },
            baseline: Origin::Absent,
            baseline_bytes: 0,
            version_available: false,
            epoch_available: false,
            contested_destination: self.resource_contested(&key.identity),
        })
    }
    fn shutdown_source_binding(
        &self,
        binding: &Binding,
        location: Location,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        b.step()?;
        let mut row = self.shutdown_binding_row(
            &binding.document,
            &binding.key(),
            ArtifactKind::Source,
            location,
        )?;
        if let Some(baseline) = &binding.baseline {
            row.baseline_bytes = baseline.bytes.0.len();
            row.version_available = baseline.validator.is_some();
            row.epoch_available = baseline.epoch.is_some();
            row.baseline = if let Some(id) = &baseline.anchor {
                let task = self
                    .resource_task(id)
                    .map_err(|_| CoordinatorError::Faulted)?;
                if !task.source_baseline_candidate()
                    || task.captured.document != binding.document
                    || task.identity != binding.identity
                    || task.binding_generation != binding.generation
                    || !Arc::ptr_eq(&task.registration, &binding.registration)
                    || task.payload != baseline.bytes
                {
                    return Err(CoordinatorError::Faulted);
                }
                self.validate_source_capture(task)
                    .map_err(|_| CoordinatorError::Faulted)?;
                Origin::Write(id.clone())
            } else {
                Origin::Unanchored
            };
        }
        self.shutdown_emit_binding(row, binding.identity.key.0.len(), b)
    }
    fn shutdown_annotation_binding(
        &self,
        binding: &AnnotationBinding,
        location: Location,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        b.step()?;
        let mut row = self.shutdown_binding_row(
            &binding.document,
            &binding.key,
            ArtifactKind::Annotations,
            location,
        )?;
        if let Some(baseline) = &binding.baseline {
            row.baseline_bytes = baseline.bytes().0.len();
            row.baseline = match baseline {
                AnnotationBaselineSlot::Written(w) => {
                    let task = self
                        .resource_task(&w.fact.producer)
                        .map_err(|_| CoordinatorError::Faulted)?;
                    if task.kind != ArtifactKind::Annotations
                        || task.artifact != w.fact.artifact
                        || task.captured != w.fact.source
                        || task.captured.document != binding.document
                        || task.identity != binding.key.identity
                        || !Arc::ptr_eq(&task.registration, &binding.key.registration)
                        || task.payload != w.bytes
                        || task.annotation_association.as_ref() != Some(&w.association)
                        || w.association.binding != binding.key
                    {
                        return Err(CoordinatorError::Faulted);
                    }
                    row.version_available = w.external_version.is_some();
                    Origin::Write(w.fact.producer.clone())
                }
                AnnotationBaselineSlot::Loaded(l) => {
                    if l.fact.source.document != binding.document
                        || l.association.binding != binding.key
                    {
                        return Err(CoordinatorError::Faulted);
                    }
                    // Original loaded-evidence identity is checked independently
                    // of present write eligibility or a contested destination.
                    self.shutdown_loaded_baseline_intact(binding, l)
                        .map_err(|_| CoordinatorError::Faulted)?;
                    row.version_available = true;
                    Origin::AnnotationRead(l.fact.origin.clone())
                }
            };
        }
        self.shutdown_emit_binding(row, binding.key.identity.key.0.len(), b)
    }
    fn shutdown_emit_binding(
        &self,
        row: Row,
        key_bytes: usize,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let node = Target::Binding(row.binding.clone());
        b.dependency(node.clone(), Target::Document(row.document.clone()))?;
        match &row.baseline {
            Origin::Write(id) => b.dependency(node, Target::Write(id.clone()))?,
            Origin::AnnotationRead(id) => b.dependency(node, Target::AnnotationRead(id.clone()))?,
            Origin::Absent | Origin::Unanchored => {}
        }
        b.binding(row, key_bytes)
    }
    pub(in crate::coordinator) fn capture_shutdown_binding_inventory(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        for binding in &self.resources.bindings {
            self.shutdown_source_binding(binding, Location::Active, b)?;
        }
        for binding in &self.resources.annotation_bindings {
            self.shutdown_annotation_binding(binding, Location::Active, b)?;
        }
        for (slot, document) in self.documents.iter().enumerate() {
            b.step()?;
            let Some(document) = document else {
                continue;
            };
            if document.lifecycle.phase == LifecyclePhase::ProvenanceReleased {
                // These retained descriptors are historical document metadata,
                // not active producer pins. Validate actual release before
                // excluding them; a retired producer must not be looked up and
                // resurrected as an outstanding binding obligation.
                self.disposed_provenance_released(&DocumentId {
                    namespace: self.namespace.clone(),
                    slot,
                })
                .map_err(CoordinatorError::Resource)?;
                continue;
            }
            let retained = &document.retained_bindings;
            if let Some(binding) = &retained.source {
                self.shutdown_source_binding(binding, Location::DisposedRetained, b)?;
            }
            if let Some(binding) = &retained.annotations {
                self.shutdown_annotation_binding(binding, Location::DisposedRetained, b)?;
            }
            for binding in &retained.source_history {
                self.shutdown_source_binding(binding, Location::AssociationHistory, b)?;
            }
        }
        Ok(())
    }
}
