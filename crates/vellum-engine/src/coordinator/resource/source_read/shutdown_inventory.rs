use super::*;
#[cfg(test)]
mod tests;
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_source_reader(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let owner = self.shutdown_owner_id(ShutdownOwnerRoleV1::SourceReader);
        let mut record = ShutdownOwnerRecordV1 {
            owner: owner.clone(),
            state: ShutdownOwnerStateV1::Unconfigured,
            admission_stopped: false,
            authority_fenced: self.shutdown_fence_marker().is_some(),
            issued_slots: None,
            retained_records: None,
            reserved_bytes: None,
            additional_reserved_bytes: None,
            local_reserved_bytes: None,
            local_byte_limit: None,
            operation_registry_state: None,
            held_bytes: None,
            retirement_supported: true,
        };
        let s = match &self.resources.source_reader {
            State::Unconfigured => {
                b.owner(record)?;
                return Ok(());
            }
            State::Closed => {
                record.state = ShutdownOwnerStateV1::Closed;
                record.admission_stopped = true;
                b.owner(record)?;
                return Ok(());
            }
            State::Active(s) => s,
        };
        self.validate_source_reader()
            .map_err(CoordinatorError::Resource)?;
        if self.resources.limits.is_none() {
            return Err(CoordinatorError::Faulted);
        }
        record.state = ShutdownOwnerStateV1::Active;
        record.admission_stopped = s.stopped;
        record.issued_slots = Some(s.rows.len());
        record.retained_records = Some(s.rows.iter().flatten().count());
        record.reserved_bytes = Some(
            s.limits
                .reserved_bytes()
                .map_err(CoordinatorError::Resource)?,
        );
        record.local_byte_limit = Some(s.limits.max_held_bytes);
        record.operation_registry_state = Some(match s.operations {
            operations::State::Unconfigured => ShutdownOwnerStateV1::Unconfigured,
            operations::State::Active(_) => ShutdownOwnerStateV1::Active,
            operations::State::Closed => ShutdownOwnerStateV1::Closed,
        });
        record.held_bytes = Some(s.held);
        b.owner(record)?;
        b.dependency(
            ShutdownInventoryTargetV1::Owner(owner.clone()),
            ShutdownInventoryTargetV1::Owner(
                self.shutdown_owner_id(ShutdownOwnerRoleV1::SharedWriter),
            ),
        )?;
        for (slot, task) in s.rows.iter().enumerate() {
            b.step()?;
            let Some(t) = task else {
                continue;
            };
            let id = SourceReadIdV1 {
                owner: self.namespace.clone(),
                slot,
            };
            let node = ShutdownInventoryTargetV1::SourceRead(id.clone());
            let terminal = match &t.terminal {
                None => ShutdownReadTerminalV1::Pending,
                Some(SourceReadAtom::End { .. }) => ShutdownReadTerminalV1::End,
                Some(SourceReadAtom::Failure(_)) => ShutdownReadTerminalV1::Failure,
                _ => return Err(CoordinatorError::Faulted),
            };
            let source = t.activated.as_ref().map(|(a, _)| a.document.clone());
            if let Some(source) = &source {
                self.retained_document(&source.document)?;
                b.dependency(
                    node.clone(),
                    ShutdownInventoryTargetV1::Document(source.document.clone()),
                )?;
            }
            b.dependency(node, ShutdownInventoryTargetV1::Owner(owner.clone()))?;
            b.read(
                ShutdownReadRecordV1 {
                    read: ShutdownReadIdV1::Source(id),
                    destination: ShutdownDestinationIdV1(t.identity.clone()),
                    provider: ReadProviderContext {
                        registration: t.registration.clone(),
                    },
                    source,
                    annotation_generation: None,
                    phase: t.phase,
                    binding_generation: t.activated.as_ref().map(|(_, key)| key.generation),
                    source_binding_generation: None,
                    terminal,
                    stopped: t.stopped,
                    quiescent: t.quiescent,
                    contested: t.contested,
                    refused: t.refused,
                    candidate_ready: t.candidate.is_some(),
                    received_bytes: t.bytes.len(),
                    received_chunks: t.chunks.len(),
                    witness_count: t.witnesses.iter().flatten().count(),
                    dispatch_evidence_references: t.nonce.as_ref().map_or(0, Arc::strong_count),
                    capture_references: t.candidate_marker.as_ref().map_or(0, Arc::strong_count),
                    held_bytes: t.charge,
                    restoration_bytes: 0,
                    restored_annotation_generation: None,
                    loaded_baseline_adopted: false,
                },
                t.identity.key.0.len(),
            )?;
        }
        Ok(())
    }
}
