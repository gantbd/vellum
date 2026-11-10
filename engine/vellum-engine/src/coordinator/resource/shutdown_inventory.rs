//! Actual shared writer records and dependency edges, not usage-count inference.
use super::*;
use crate::coordinator::shutdown::inventory::{
    Builder, ShutdownArtifactRecordV1, ShutdownInventoryTargetV1 as Target, ShutdownWriteRecordV1,
};
#[cfg(test)]
mod tests;
impl SourceCoordinator {
    /// Provider/encoder/restorer and catalog declarations are inert runtime
    /// configuration, not worker leases. A prepared source or export reservation
    /// is different: it is retained work and must block final root release.
    /// Keep this classification paired with capture_shutdown_resource_metadata.
    pub(in crate::coordinator) fn shutdown_resource_metadata_idle(&self) -> bool {
        self.resources.source_preparation.is_none()
            && self.resources.source_export_reservations.is_empty()
    }
    pub(in crate::coordinator) fn shutdown_cancellation_save_end(&self) -> usize {
        self.resources.tasks.len()
    }
    pub(in crate::coordinator) fn shutdown_cancellation_save_at(
        &self,
        index: usize,
    ) -> Option<(SaveId, DocumentId)> {
        let task = self.resources.tasks.get(index)?.as_ref()?;
        (task.phase == SavePhase::Queued).then(|| {
            (
                SaveId {
                    owner: self.namespace.clone(),
                    slot: index,
                },
                task.captured.document.clone(),
            )
        })
    }
    pub(in crate::coordinator) fn shutdown_save_admissible(
        &self,
        expected: &crate::coordinator::ShutdownDocumentRecordV1,
        kind: ArtifactKind,
        id: &SaveId,
        r: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        let t = self.resource_task(id)?;
        if t.kind != kind || t.captured != expected.source {
            return Err(ResourceError::Unavailable);
        }
        self.provider_current(&t.registration)?;
        self.writer_catalog_current(
            t.catalog.as_ref(),
            &t.registration,
            &t.identity,
            t.synchronize,
            t.continuity.is_some(),
            r,
        )?;
        if t.contested() || self.resource_contested(&t.identity) {
            return Ok(false);
        }
        for other in self.resources.tasks.iter().flatten() {
            if other.identity != t.identity {
                continue;
            }
            let harmless = other.phase == SavePhase::Queued && other.stop_requested
                || other.phase == SavePhase::Dispatched
                    && other.knowledge.data.application == Application::NotWritten
                    && other.no_future
                    && other.quiescent;
            if !harmless
                && (other.contested()
                    || other.captured != t.captured
                    || other.kind != t.kind
                    || other.serialization != t.serialization
                    || other.captured_annotation_set != t.captured_annotation_set)
            {
                return Ok(false);
            }
        }
        let ordinary = match kind {
            ArtifactKind::Source => {
                t.purpose == SavePurpose::OrdinarySource
                    && t.serialization == Some(expected.serialization)
                    && Some(t.binding_generation) == expected.source_binding_generation
            }
            ArtifactKind::Annotations => {
                t.purpose == SavePurpose::OrdinaryAnnotations
                    && t.captured_annotation_set == Some(expected.annotation_generation)
                    && t.annotation_association
                        .as_ref()
                        .map(|a| a.binding.generation)
                        == expected.annotation_binding_generation
            }
        };
        if !ordinary || t.stop_requested {
            return match self.validate_shutdown_save_readiness(expected, kind, id, r) {
                Ok(()) => Ok(true),
                Err(ResourceError::Faulted) => Err(ResourceError::Faulted),
                Err(_) => Ok(false),
            };
        }
        if t.baseline_claim.is_none() {
            return Err(ResourceError::Faulted);
        }
        Ok(t.knowledge.data.application != Application::NotWritten
            && t.knowledge.data.synchronization != Synchronization::Failed)
    }
    pub(in crate::coordinator) fn capture_shutdown_resource_metadata(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let s = &self.resources;
        let role = ShutdownOwnerRoleV1::ResourceMetadata;
        let (catalog, mut retained) = s.catalog.shutdown_records(b)?;
        for _ in &s.providers {
            b.step()?;
            retained = retained.checked_add(1).ok_or(CoordinatorError::Faulted)?;
        }
        for _ in &s.encoders {
            b.step()?;
            retained = retained.checked_add(1).ok_or(CoordinatorError::Faulted)?;
        }
        for _ in &s.restorers {
            b.step()?;
            retained = retained.checked_add(1).ok_or(CoordinatorError::Faulted)?;
        }
        if let Some(prepared) = &s.source_preparation {
            b.step()?;
            retained = retained.checked_add(1).ok_or(CoordinatorError::Faulted)?;
            self.record_shutdown_pool_document(b, role, &prepared.source.document)?;
        }
        for reservation in &s.source_export_reservations {
            b.step()?;
            retained = retained.checked_add(1).ok_or(CoordinatorError::Faulted)?;
            self.resource_task(&reservation.task)
                .map_err(|_| CoordinatorError::Faulted)?;
            self.record_shutdown_pool_document(b, role, &reservation.document)?;
            b.dependency(
                Target::Owner(self.shutdown_owner_id(role)),
                Target::Write(reservation.task.clone()),
            )?;
        }
        if s.limits.is_none()
            && (!s.providers.is_empty()
                || !s.encoders.is_empty()
                || !s.restorers.is_empty()
                || s.source_preparation.is_some()
                || !s.source_export_reservations.is_empty())
        {
            return Err(CoordinatorError::Faulted);
        }
        self.record_shutdown_local_owner(
            b,
            role,
            crate::coordinator::shutdown::inventory::LocalOwnerSnapshot {
                additional_reserved: None,
                configured: s.limits.is_some() || catalog,
                issued: None,
                retained,
                held: None,
                reserved: None,
                limit: None,
            },
        )
    }
    pub(in crate::coordinator) fn validate_shutdown_save_readiness(
        &self,
        expected: &crate::coordinator::ShutdownDocumentRecordV1,
        kind: ArtifactKind,
        id: &SaveId,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let task = self.resource_task(id)?;
            if task.kind != kind || task.captured != expected.source {
                return Err(ResourceError::Unavailable);
            }
            let candidate = match kind {
                ArtifactKind::Source => task.source_baseline_candidate(),
                ArtifactKind::Annotations => matches!(
                    task.purpose,
                    SavePurpose::OrdinaryAnnotations | SavePurpose::AnnotationExport
                ),
            };
            if !candidate {
                return Err(ResourceError::Unsupported);
            }
            self.provider_current(&task.registration)?;
            self.writer_catalog_current(
                task.catalog.as_ref(),
                &task.registration,
                &task.identity,
                task.synchronize,
                task.continuity.is_some(),
                r,
            )?;
            if self.resource_contested(&task.identity) {
                return Err(ResourceError::Contested);
            }
            if !task.qualified() {
                return Err(ResourceError::AcknowledgmentUnsatisfied);
            }
            match task.baseline_claim.as_ref() {
                Some(claim) if claim.state != BaselineCapacityState::Installed => {
                    return Err(ResourceError::Retained);
                }
                None if matches!(
                    task.purpose,
                    SavePurpose::OrdinarySource | SavePurpose::OrdinaryAnnotations
                ) =>
                {
                    return Err(ResourceError::Faulted);
                }
                // Activated Save As and explicitly adopted annotation exports
                // have no ordinary-save capacity claim. The exact installed
                // native baseline below, not export success, establishes them.
                _ => {}
            }
            match kind {
                ArtifactKind::Source => {
                    if task.serialization != Some(expected.serialization)
                        || Some(task.binding_generation) != expected.source_binding_generation
                    {
                        return Err(ResourceError::Stale);
                    }
                    self.validate_source_capture(task)?;
                    let binding = self
                        .resources
                        .bindings
                        .iter()
                        .find(|b| b.document == expected.source.document)
                        .ok_or(ResourceError::Unbound)?;
                    if binding.generation != task.binding_generation
                        || binding.identity != task.identity
                        || !Arc::ptr_eq(&binding.registration, &task.registration)
                    {
                        return Err(ResourceError::Stale);
                    }
                    let baseline = binding
                        .baseline
                        .as_ref()
                        .ok_or(ResourceError::Unavailable)?;
                    if baseline.anchor.as_ref() != Some(id) {
                        return Err(ResourceError::Unavailable);
                    }
                    // Later nonconflicting token enrichment does not erase the
                    // installed acknowledgment; any retained token must agree.
                    if baseline.bytes != task.payload
                        || baseline
                            .validator
                            .as_ref()
                            .is_some_and(|v| task.knowledge.data.version.as_ref() != Some(v))
                        || baseline
                            .epoch
                            .as_ref()
                            .is_some_and(|v| task.knowledge.data.epoch.as_ref() != Some(v))
                    {
                        return Err(ResourceError::Faulted);
                    }
                }
                ArtifactKind::Annotations => {
                    if task.captured_annotation_set != Some(expected.annotation_generation)
                        || task
                            .annotation_association
                            .as_ref()
                            .map(|a| a.binding.generation)
                            != expected.annotation_binding_generation
                        || self.current_annotation_association(&expected.source.document)
                            != task.annotation_association
                    {
                        return Err(ResourceError::Stale);
                    }
                    let binding = self
                        .resources
                        .annotation_bindings
                        .iter()
                        .find(|b| b.document == expected.source.document)
                        .ok_or(ResourceError::Unbound)?;
                    let baseline = binding
                        .baseline
                        .as_ref()
                        .and_then(AnnotationBaselineSlot::written)
                        .filter(|b| b.fact.producer == *id)
                        .ok_or(ResourceError::Unavailable)?;
                    self.validate_annotation_baseline(binding, baseline)?;
                }
            }
            Ok(())
        })();
        capture_service(r, Boundary::ShutdownSaveReadiness, &result, timer.finish());
        result
    }
    pub(in crate::coordinator) fn shutdown_annotation_binding_generation(
        &self,
        document: &DocumentId,
    ) -> Result<Option<u64>, CoordinatorError> {
        let mut active = self
            .resources
            .annotation_bindings
            .iter()
            .filter(|b| b.document == *document);
        let binding = active.next();
        let retained = self
            .retained_document(document)?
            .retained_bindings
            .annotations
            .as_ref();
        if active.next().is_some() || (binding.is_some() && retained.is_some()) {
            return Err(CoordinatorError::Faulted);
        }
        Ok(binding.map(|b| b.key.generation))
    }
    pub(in crate::coordinator) fn capture_shutdown_writer_inventory(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        self.resources
            .writer_lifecycle_intact()
            .map_err(CoordinatorError::Resource)?;
        self.artifact_charge_intact().map_err(owner_error_inverse)?;
        self.validate_save_operations()
            .map_err(owner_error_inverse)?;
        for (slot, task) in self.resources.tasks.iter().enumerate() {
            b.step()?;
            let Some(t) = task else {
                continue;
            };
            let id = SaveId {
                owner: self.namespace.clone(),
                slot,
            };
            let artifact = self
                .resource_artifact(&t.artifact)
                .map_err(|_| CoordinatorError::Faulted)?;
            self.retained_document(&t.captured.document)?;
            if t.baseline_claim
                .as_ref()
                .is_some_and(|c| c.state == BaselineCapacityState::Pending)
            {
                self.validate_root_promise(t)
                    .map_err(|_| CoordinatorError::Faulted)?;
            }
            if artifact.basis.source() != &t.captured || artifact.bytes != t.payload {
                return Err(CoordinatorError::Faulted);
            }
            match (&artifact.basis, t.kind) {
                (ArtifactBasis::Source { serialization, .. }, ArtifactKind::Source)
                    if Some(*serialization) == t.serialization => {}
                (ArtifactBasis::Annotations { snapshot, encoder }, ArtifactKind::Annotations)
                    if Some(snapshot.set_version()) == t.captured_annotation_set
                        && t.encoding_context.as_ref() == Some(&encoder.context)
                        && self.knows_annotation_snapshot(snapshot) => {}
                _ => return Err(CoordinatorError::Faulted),
            }
            b.write(ShutdownWriteRecordV1 {
                write: id.clone(),
                artifact: t.artifact.clone(),
                source: t.captured.clone(),
                kind: t.kind,
                purpose: t.purpose,
                binding_generation: t.binding_generation,
                annotation_generation: t.captured_annotation_set,
                serialization: t.serialization,
                phase: t.phase,
                application: t.knowledge.data.application,
                observed_bytes: t.knowledge.data.observed,
                version_available: t.knowledge.data.version.is_some(),
                epoch_available: t.knowledge.data.epoch.is_some(),
                causal_edge_available: t.causal.edge_available(),
                retained_evidence_witnesses: t.knowledge.witnesses.iter().flatten().count()
                    + usize::from(t.knowledge.conflict.is_some())
                    + t.causal.conflicts(),
                dispatch_evidence_references: t.nonce.as_ref().map_or(0, Arc::strong_count),
                synchronization: t.knowledge.data.synchronization,
                synchronization_required: t.synchronize,
                no_future_effects: t.no_future,
                quiescent: t.quiescent,
                contested: t.contested(),
                stop_requested: t.stop_requested,
                baseline_capacity: t.baseline_claim.as_ref().map(|c| c.state),
                baseline_envelope_bytes: t.baseline_claim.as_ref().map(|c| c.charge),
                dispatch_serial: t.dispatch_serial,
            })?;
            b.dependency(
                Target::Write(id.clone()),
                Target::Artifact(t.artifact.clone()),
            )?;
            b.dependency(
                Target::Write(id),
                Target::Document(t.captured.document.clone()),
            )?;
        }
        for (slot, row) in self.resources.artifacts.iter().enumerate() {
            b.step()?;
            let Some(row) = row else {
                continue;
            };
            let id = ResourceArtifactId {
                owner: self.namespace.clone(),
                slot,
            };
            let (source, kind) = match &row.basis {
                ArtifactBasis::Source { source, .. } => (source.clone(), ArtifactKind::Source),
                ArtifactBasis::Annotations { snapshot, .. } => {
                    (snapshot.source().clone(), ArtifactKind::Annotations)
                }
            };
            self.retained_document(&source.document)?;
            b.artifact(ShutdownArtifactRecordV1 {
                artifact: id.clone(),
                source: source.clone(),
                kind,
                stopped: row.stopped,
            })?;
            b.dependency(Target::Artifact(id), Target::Document(source.document))?;
        }
        Ok(())
    }
}
fn owner_error_inverse(error: ResourceError) -> CoordinatorError {
    CoordinatorError::Resource(error)
}
