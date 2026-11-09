use super::*;
#[cfg(test)]
mod tests;
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_annotation_reader(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        self.validate_read_operations()
            .map_err(CoordinatorError::Resource)?;
        let owner = self.shutdown_owner_id(ShutdownOwnerRoleV1::AnnotationReader);
        let limits = self.resources.limits;
        let closed = self.resources.annotation_reader_closed;
        let mut held = if self.resources.read_baseline_attachment.is_some() {
            std::mem::size_of::<ReadBaselineEndpoint>()
        } else {
            0
        };
        b.owner(ShutdownOwnerRecordV1 {
            owner: owner.clone(),
            state: if closed {
                ShutdownOwnerStateV1::Closed
            } else if limits.is_some() {
                ShutdownOwnerStateV1::Active
            } else {
                ShutdownOwnerStateV1::Unconfigured
            },
            admission_stopped: self.resources.annotation_read_admission_stopped,
            authority_fenced: self.shutdown_fence_marker().is_some(),
            issued_slots: (!closed).then_some(self.resources.reads.len()),
            retained_records: (!closed).then_some(self.resources.reads.iter().flatten().count()),
            reserved_bytes: None,
            additional_reserved_bytes: None,
            local_reserved_bytes: None,
            local_byte_limit: limits.filter(|_| !closed).map(|l| l.max_held_read_bytes),
            operation_registry_state: limits.filter(|_| !closed).map(|_| {
                match self.resources.read_operations {
                    operations::State::Unconfigured => ShutdownOwnerStateV1::Unconfigured,
                    operations::State::Active(_) => ShutdownOwnerStateV1::Active,
                    operations::State::Closed => ShutdownOwnerStateV1::Closed,
                }
            }),
            held_bytes: (!closed).then_some(self.resources.held_read_bytes),
            retirement_supported: true,
        })?;
        if limits.is_some() && !closed {
            b.dependency(
                ShutdownInventoryTargetV1::Owner(owner.clone()),
                ShutdownInventoryTargetV1::Owner(
                    self.shutdown_owner_id(ShutdownOwnerRoleV1::SharedWriter),
                ),
            )?;
        }
        for (slot, task) in self.resources.reads.iter().enumerate() {
            b.step()?;
            let Some(_) = task else {
                continue;
            };
            let id = AnnotationReadId {
                owner: self.namespace.clone(),
                slot,
            };
            let t = self.read_task(&id).map_err(CoordinatorError::Resource)?;
            self.retained_document(&t.source.document)?;
            held = held
                .checked_add(t.charge)
                .ok_or(CoordinatorError::Faulted)?;
            let node = ShutdownInventoryTargetV1::AnnotationRead(id.clone());
            b.dependency(
                node.clone(),
                ShutdownInventoryTargetV1::Owner(owner.clone()),
            )?;
            b.dependency(
                node.clone(),
                ShutdownInventoryTargetV1::Document(t.source.document.clone()),
            )?;
            let terminal = match &t.terminal {
                None => ShutdownReadTerminalV1::Pending,
                Some(ReadAtom::End { .. }) => ShutdownReadTerminalV1::End,
                Some(ReadAtom::Failure(_)) => ShutdownReadTerminalV1::Failure,
                _ => return Err(CoordinatorError::Faulted),
            };
            if let Some(p) = &t.restoration {
                if p.package.logical_size() != Some(p.package.charge)
                    || p.package.charge > t.registration.limits.max_restoration_bytes
                    || p.package.capture.origin() != &id
                    || p.outcome.source != t.source
                    || p.outcome.context != p.package.registration.context
                    || t.capture
                        .as_ref()
                        .is_none_or(|c| !Arc::ptr_eq(c, &p.package.capture.data))
                {
                    return Err(CoordinatorError::Faulted);
                }
                for source in &p.package.sources {
                    b.step()?;
                    self.retained_document(&source.source().document)?;
                    if source.source().document != t.source.document {
                        b.dependency(
                            node.clone(),
                            ShutdownInventoryTargetV1::Document(source.source().document.clone()),
                        )?;
                    }
                }
                if let Some(a) = &self.resources.read_baseline_attachment {
                    a.current(&self.namespace)
                        .map_err(CoordinatorError::Resource)?;
                    b.dependency(
                        node,
                        ShutdownInventoryTargetV1::ReadWriterAttachment(
                            ShutdownReadWriterAttachmentIdV1 {
                                owner: self.namespace.clone(),
                                generation: self.resources.read_baseline_generation,
                            },
                        ),
                    )?;
                } else if p.outcome.loaded_baseline_adopted {
                    return Err(CoordinatorError::Faulted);
                }
            }
            b.read(
                ShutdownReadRecordV1 {
                    read: ShutdownReadIdV1::Annotation(id),
                    destination: ShutdownDestinationIdV1(t.identity.clone()),
                    provider: ReadProviderContext {
                        registration: t.registration.clone(),
                    },
                    source: Some(t.source.clone()),
                    annotation_generation: Some(t.set),
                    phase: t.phase,
                    binding_generation: Some(t.association.binding.generation),
                    source_binding_generation: t
                        .association
                        .source_binding
                        .as_ref()
                        .map(|key| key.generation),
                    terminal,
                    stopped: t.stopped,
                    quiescent: t.quiescent,
                    contested: t.contested,
                    refused: t.refused,
                    candidate_ready: t.capture.is_some(),
                    received_bytes: t.data().len(),
                    received_chunks: t.chunks.len(),
                    witness_count: t.witnesses.iter().flatten().count(),
                    dispatch_evidence_references: t.nonce.as_ref().map_or(0, Arc::strong_count),
                    capture_references: t.capture.as_ref().map_or(0, Arc::strong_count),
                    held_bytes: t.charge,
                    restoration_bytes: t.restoration.as_ref().map_or(0, |p| p.package.charge),
                    restored_annotation_generation: t
                        .restoration
                        .as_ref()
                        .map(|p| p.outcome.set_version),
                    loaded_baseline_adopted: t
                        .restoration
                        .as_ref()
                        .is_some_and(|p| p.outcome.loaded_baseline_adopted),
                },
                t.identity.key.0.len(),
            )?;
        }
        if held != self.resources.held_read_bytes
            || limits.is_some_and(|l| held > l.max_held_read_bytes)
        {
            return Err(CoordinatorError::Faulted);
        }
        Ok(())
    }
}
