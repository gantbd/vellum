//! Protected whole-reader retirement, separate from document/writer lifetimes.
use super::*;
#[cfg(test)]
mod tests;
impl ResourceState {
    pub(in crate::coordinator::resource) fn annotation_reader_lifecycle_intact(
        &self,
    ) -> Result<(), ResourceError> {
        if self.annotation_reader_closed
            && (self.limits.is_none()
                || !self.annotation_read_admission_stopped
                || !matches!(self.read_operations, operations::State::Closed)
                || !self.reads.is_empty()
                || self.held_read_bytes != 0
                || !self.restorers.is_empty()
                || self.read_baseline_attachment.is_some())
        {
            return Err(ResourceError::Faulted);
        }
        Ok(())
    }
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn ensure_annotation_reader_open(
        &self,
    ) -> Result<(), ResourceError> {
        self.resources.annotation_reader_lifecycle_intact()?;
        if self.resources.annotation_reader_closed {
            return Err(ResourceError::Unavailable);
        }
        Ok(())
    }
    /// Protected owner control, not read/discard authority. Original tasks,
    /// captures, restoration provenance, registry reservations and both attachment
    /// endpoints must already have discharged through their existing controls.
    /// Inert host-held restorer ports are not workers; removing their registry
    /// entries and closing ingress prevents them from acquiring new engine work.
    pub fn retire_annotation_reader_v1(
        &mut self,
        owner_control: Option<bool>,
        retained_released: Option<bool>,
        delivery_drained: Option<bool>,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.resources.annotation_reader_lifecycle_intact()?;
            if self.resources.limits.is_none() {
                return Err(ResourceError::Unsupported);
            }
            if self.resources.annotation_reader_closed {
                return Err(ResourceError::Unavailable);
            }
            self.validate_read_operations()?;
            if !self.resources.annotation_read_admission_stopped
                || [owner_control, retained_released, delivery_drained]
                    .into_iter()
                    .any(|v| v != Some(true))
                || self.resources.reads.iter().any(Option::is_some)
                || self.resources.read_baseline_attachment.is_some()
                || matches!(self.resources.read_operations, operations::State::Active(_))
            {
                return Err(ResourceError::Retained);
            }
            // Empty child slots do not authorize dropping an unexplained charge.
            if self.resources.held_read_bytes != 0 {
                return Err(ResourceError::Faulted);
            }
            // All fallible validation precedes these non-callback native moves.
            // There is no parent debit to refund here: an active owned-operation
            // store must have retired separately, with its actual reservation.
            self.resources.read_operations = operations::State::Closed;
            self.resources.reads = Vec::new();
            self.resources.restorers = Vec::new();
            self.resources.annotation_reader_closed = true;
            Ok(())
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(
            r,
            Boundary::AnnotationReaderRetirement,
            &result,
            timer.finish(),
        );
        result
    }
}
