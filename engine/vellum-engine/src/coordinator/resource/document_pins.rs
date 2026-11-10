//! Actual resource-owned document incidences. This is one owner contribution,
//! not a whole-engine reclamation certificate, recipient grant or RSS measure.
use super::*;
#[cfg(test)]
mod tests;

/// Trusted-owner metadata, like ResourceUsage. Counts are retained row incidences
/// (not unique allocations); stopped or quiescent records still count until
/// actual retirement. Document/history/view/invocation and other owners are not
/// included. Zero here must never be interpreted as permission to reclaim.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DocumentResourcePinsV1 {
    pub source_bindings: usize,
    pub annotation_bindings: usize,
    pub write_tasks: usize,
    pub write_parents: usize,
    pub artifacts: usize,
    pub source_preparations: usize,
    pub export_reservations: usize,
    pub annotation_reads: usize,
    pub annotation_read_parents: usize,
    pub activated_source_reads: usize,
}
impl DocumentResourcePinsV1 {
    pub fn total(self) -> Result<usize, ResourceError> {
        [
            self.source_bindings,
            self.annotation_bindings,
            self.write_tasks,
            self.write_parents,
            self.artifacts,
            self.source_preparations,
            self.export_reservations,
            self.annotation_reads,
            self.annotation_read_parents,
            self.activated_source_reads,
        ]
        .into_iter()
        .try_fold(0usize, |n, value| {
            n.checked_add(value).ok_or(ResourceError::Faulted)
        })
    }
}
impl SourceCoordinator {
    /// Bounded, allocation-free resource projection for an exact retained native
    /// document (live or disposed). No body, path, identity key or token release.
    /// A fault/invalid owner is an error, never an invented empty inventory.
    pub fn document_resource_pins_v1(
        &self,
        document: &DocumentId,
        r: &mut Recorder,
    ) -> Result<DocumentResourcePinsV1, ResourceError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            self.retained_document(document).map_err(owner_error)?;
            self.artifact_charge_intact()?;
            // Exhaustive owner census: additions must be classified here. Pure
            // provider/catalog/domain metadata has no document incarnation edge.
            // Observer target IDs and continuity refer through existing tasks;
            // they do not manufacture extra direct document retention after a
            // producer's independently permitted retirement.
            let ResourceState {
                catalog: _,
                queue_timing: _, // Numeric diagnostics retain no document identity.
                source_reader,
                read_operations: _,
                annotation_read_admission_stopped: _,
                annotation_reader_closed: _,
                writer_closed: _,
                observation: _,
                operations: _,
                continuity: _,
                limits: _,
                providers: _,
                bindings,
                annotation_bindings,
                tasks,
                encoders: _,
                artifacts,
                source_preparation,
                source_export_reservations,
                held_artifact_bytes: _,
                last_dispatch_serial: _,
                annotation_save_admission_stopped: _,
                source_save_admission_stopped: _,
                reads,
                held_read_bytes: _,
                restorers: _,
                read_baseline_attachment: _,
                read_baseline_generation: _,
            } = &self.resources;
            let pins = DocumentResourcePinsV1 {
                source_bindings: bindings.iter().filter(|v| v.document == *document).count(),
                annotation_bindings: annotation_bindings
                    .iter()
                    .filter(|v| v.document == *document)
                    .count(),
                write_tasks: tasks
                    .iter()
                    .flatten()
                    .filter(|v| v.captured.document == *document)
                    .count(),
                write_parents: self.save_operation_document_pins(document)?,
                artifacts: artifacts
                    .iter()
                    .flatten()
                    .filter(|v| v.basis.source().document == *document)
                    .count(),
                source_preparations: usize::from(
                    source_preparation
                        .as_ref()
                        .is_some_and(|v| v.source.document == *document),
                ),
                export_reservations: source_export_reservations
                    .iter()
                    .filter(|v| v.document == *document)
                    .count(),
                annotation_reads: reads
                    .iter()
                    .flatten()
                    .filter(|v| v.retains_document(document))
                    .count(),
                annotation_read_parents: self.annotation_read_operation_document_pins(document)?,
                activated_source_reads: source_reader.document_pins(document),
            };
            pins.total()?;
            Ok(pins)
        })();
        capture_service(r, Boundary::DocumentResourcePins, &result, timing.finish());
        result
    }
}
