//! Reader owners and their shared attachment remain distinct from child tasks.
use super::*;
impl SourceCoordinator {
    pub(in crate::coordinator) fn shutdown_owner_id(
        &self,
        role: ShutdownOwnerRoleV1,
    ) -> ShutdownOwnerIdV1 {
        ShutdownOwnerIdV1 {
            owner: self.namespace.clone(),
            role,
        }
    }
    pub(in crate::coordinator) fn capture_shutdown_reader_inventory(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let writer = self.shutdown_owner_id(ShutdownOwnerRoleV1::SharedWriter);
        let closed = self.resources.writer_closed;
        b.owner(ShutdownOwnerRecordV1 {
            owner: writer.clone(),
            state: if closed {
                ShutdownOwnerStateV1::Closed
            } else if self.resources.limits.is_some() {
                ShutdownOwnerStateV1::Active
            } else {
                ShutdownOwnerStateV1::Unconfigured
            },
            admission_stopped: self.resources.annotation_save_admission_stopped
                && self.resources.source_save_admission_stopped,
            authority_fenced: self.shutdown_fence_marker().is_some(),
            issued_slots: (!closed).then_some(self.resources.tasks.len()),
            retained_records: (!closed).then_some(self.resources.tasks.iter().flatten().count()),
            reserved_bytes: None,
            additional_reserved_bytes: None,
            local_reserved_bytes: None,
            local_byte_limit: self
                .resources
                .limits
                .filter(|_| !closed)
                .map(|l| l.max_held_artifact_bytes),
            operation_registry_state: self.resources.limits.filter(|_| !closed).map(|_| {
                if self.resources.operations.root_profile().is_some() {
                    ShutdownOwnerStateV1::Active
                } else {
                    ShutdownOwnerStateV1::Unconfigured
                }
            }),
            held_bytes: (!closed).then_some(self.resources.held_artifact_bytes),
            retirement_supported: true,
        })?;
        self.capture_shutdown_source_reader(b)?;
        self.capture_shutdown_annotation_reader(b)?;
        if let Some(a) = &self.resources.read_baseline_attachment {
            a.current(&self.namespace)
                .map_err(|_| CoordinatorError::Faulted)?;
            if self.resources.read_baseline_generation == 0
                || self.resources.read_baseline_generation == u64::MAX
            {
                return Err(CoordinatorError::Faulted);
            }
            let reader = self.shutdown_owner_id(ShutdownOwnerRoleV1::AnnotationReader);
            let id = ShutdownReadWriterAttachmentIdV1 {
                owner: self.namespace.clone(),
                generation: self.resources.read_baseline_generation,
            };
            b.dependency(
                ShutdownInventoryTargetV1::ReadWriterAttachment(id.clone()),
                ShutdownInventoryTargetV1::Owner(reader.clone()),
            )?;
            b.dependency(
                ShutdownInventoryTargetV1::ReadWriterAttachment(id.clone()),
                ShutdownInventoryTargetV1::Owner(writer.clone()),
            )?;
            b.read_writer_attachment(ShutdownReadWriterAttachmentV1 {
                attachment: id,
                reader,
                writer,
                reader_endpoint_bytes: std::mem::size_of::<ReadBaselineEndpoint>(),
                writer_endpoint_bytes: std::mem::size_of::<ReadBaselineEndpoint>(),
            })?;
        }
        Ok(())
    }
}
