//! Protected shutdown retirement of the shared pending-write/artifact owner.
use super::*;
#[cfg(test)]
mod tests;
impl ResourceState {
    pub(super) fn writer_lifecycle_intact(&self) -> Result<(), ResourceError> {
        self.annotation_reader_lifecycle_intact()?;
        if self.writer_closed
            && (self.limits.is_none()
                || !self.annotation_save_admission_stopped
                || !self.source_save_admission_stopped
                || !self.annotation_reader_closed
                || matches!(self.source_reader, source_read::State::Active(_))
                || matches!(self.observation, observation::State::Active(_))
                || !self.tasks.is_empty()
                || !self.artifacts.is_empty()
                || self.source_preparation.is_some()
                || !self.source_export_reservations.is_empty()
                || self.held_artifact_bytes != 0
                || !self.bindings.is_empty()
                || !self.annotation_bindings.is_empty()
                || self.continuity.charged != 0
                || self.continuity.limits.is_some()
                || !self.operations.retired_empty())
        {
            return Err(ResourceError::Faulted);
        }
        Ok(())
    }
}
impl SourceCoordinator {
    /// Retire the pending-write/artifact owner under its original absorbing
    /// shutdown fence. This neither discards documents nor cancels issued work.
    /// All actual producer/reader/observer/association obligations must already
    /// have discharged through their own controls. Provider/encoder/catalog
    /// metadata remains engine-owned; this is not whole-resource/domain closure.
    pub fn retire_shutdown_writer_v1(
        &mut self,
        fence: &ShutdownFenceIdV1,
        control: &TrustedShutdownFenceFactsV1<'_>,
        retained_released: Option<bool>,
        delivery_drained: Option<bool>,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.validate_shutdown_owner_control(fence, control)
                .map_err(owner_error)?;
            self.resources.writer_lifecycle_intact()?;
            if self.resources.limits.is_none() {
                return Err(ResourceError::Unsupported);
            }
            if self.resources.writer_closed {
                return Err(ResourceError::Unavailable);
            }
            self.artifact_charge_intact()?;
            self.validate_save_operations()?;
            self.validate_released_inventory()?;
            if [retained_released, delivery_drained]
                .into_iter()
                .any(|v| v != Some(true))
                || !self.resources.annotation_save_admission_stopped
                || !self.resources.source_save_admission_stopped
                || !self.resources.annotation_reader_closed
                || matches!(self.resources.source_reader, source_read::State::Active(_))
                || matches!(self.resources.observation, observation::State::Active(_))
                || self.resources.tasks.iter().any(Option::is_some)
                || self.resources.artifacts.iter().any(Option::is_some)
                || self.resources.source_preparation.is_some()
                || !self.resources.source_export_reservations.is_empty()
                || !self.resources.bindings.is_empty()
                || !self.resources.annotation_bindings.is_empty()
                || self.documents.iter().flatten().any(|d| {
                    d.lifecycle.phase != LifecyclePhase::ProvenanceReleased
                        && (d.retained_bindings.source.is_some()
                            || d.retained_bindings.annotations.is_some()
                            || !d.retained_bindings.source_history.is_empty())
                })
            {
                return Err(ResourceError::Retained);
            }
            if self.resources.held_artifact_bytes != 0 || self.resources.continuity.charged != 0 {
                return Err(ResourceError::Faulted);
            }
            // Native non-callback moves only. No ledger release and no authored
            // publication counter: all actual child credits returned separately.
            self.resources.tasks = Vec::new();
            self.resources.artifacts = Vec::new();
            self.resources.operations = operations::State::default();
            self.resources.continuity = continuity::State::default();
            self.resources.writer_closed = true;
            Ok(())
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(
            r,
            Boundary::ShutdownWriterRetirement,
            &result,
            timer.finish(),
        );
        result
    }
}
