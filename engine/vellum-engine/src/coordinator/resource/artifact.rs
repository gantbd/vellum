//! Shared retained-artifact accounting and annotation export admission. Codec
//! work stays outside this owner; issuer attestations are trusted host premises.
use super::*;
impl SourceCoordinator {
    pub(super) fn encoder_current(
        &self,
        encoder: &Arc<EncoderRegistration>,
    ) -> Result<(), ResourceError> {
        if !Arc::ptr_eq(&self.namespace, &encoder.context.owner)
            || !self
                .resources
                .encoders
                .iter()
                .any(|e| Arc::ptr_eq(e, encoder))
        {
            return Err(ResourceError::Unavailable);
        }
        Ok(())
    }
    pub fn register_annotation_encoder(
        &mut self,
        recorder: &mut Recorder,
    ) -> Result<AnnotationEncoderPort, ResourceError> {
        let result = (|| {
            self.ensure_ordinary_admission().map_err(owner_error)?;
            let limits = self.resources.limits.ok_or(ResourceError::Unsupported)?;
            if self.resources.encoders.len() >= limits.max_encoders {
                return Err(ResourceError::Limit);
            }
            let growth = super::root_accounting::registration_bytes::<EncoderRegistration>()
                .map_err(|_| ResourceError::Limit)?;
            self.check_root_growth(growth, recorder)
                .map_err(owner_error)?;
            self.resources
                .encoders
                .try_reserve(1)
                .map_err(|_| ResourceError::Capacity)?;
            let registration = Arc::new(EncoderRegistration {
                context: AnnotationEncodingContext {
                    owner: self.namespace.clone(),
                    marker: Arc::new(()),
                },
                limits,
            });
            self.resources.encoders.push(registration.clone());
            Ok(AnnotationEncoderPort { registration })
        })();
        if matches!(result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture(recorder, Boundary::ArtifactAdmission, &result);
        result
    }
    pub(super) fn preflight_artifact(&mut self, bytes: usize) -> Result<(), ResourceError> {
        self.preflight_artifact_capture(bytes, 0)
    }
    pub(super) fn preflight_artifact_capture(
        &mut self,
        bytes: usize,
        condition: usize,
    ) -> Result<(), ResourceError> {
        let limits = self.resources.limits.ok_or(ResourceError::Unsupported)?;
        let charge = bytes.checked_add(condition).ok_or(ResourceError::Limit)?;
        let pending = usize::from(self.resources.source_preparation.is_some());
        if bytes > limits.max_blob_bytes
            || self
                .resources
                .artifacts
                .len()
                .checked_add(pending)
                .is_none_or(|n| n >= limits.max_issued_artifacts)
            || self
                .resources
                .artifacts
                .iter()
                .flatten()
                .count()
                .checked_add(pending)
                .is_none_or(|n| n >= limits.max_live_artifacts)
            || self
                .resources
                .held_artifact_bytes
                .checked_add(charge)
                .is_none_or(|n| n > limits.max_held_artifact_bytes)
        {
            return Err(ResourceError::Limit);
        }
        self.resources
            .artifacts
            .try_reserve(1 + pending)
            .map_err(|_| ResourceError::Capacity)
    }
    pub(super) fn install_artifact(&mut self, artifact: ArtifactRow) -> ResourceArtifactId {
        let id = ResourceArtifactId {
            owner: self.namespace.clone(),
            slot: self.resources.artifacts.len(),
        };
        self.resources.held_artifact_bytes +=
            artifact.held_size().expect("preflight capture charge");
        self.resources.artifacts.push(Some(artifact));
        id
    }
    pub(super) fn resource_artifact(
        &self,
        id: &ResourceArtifactId,
    ) -> Result<&ArtifactRow, ResourceError> {
        if !Arc::ptr_eq(&self.namespace, &id.owner) {
            return Err(ResourceError::Unavailable);
        }
        self.resources
            .artifacts
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(ResourceError::Unavailable)
    }
    pub fn retain_annotation_artifact(
        &mut self,
        encoded: EncodedAnnotations,
        recorder: &mut Recorder,
    ) -> Result<ResourceArtifactId, ResourceError> {
        let result = (|| {
            self.ensure_ordinary_admission().map_err(owner_error)?;
            self.encoder_current(&encoded.registration)?;
            if !self.knows_annotation_snapshot(&encoded.snapshot) {
                return Err(ResourceError::Unavailable);
            }
            self.document(&encoded.snapshot.source().document)
                .map_err(owner_error)?;
            self.preflight_artifact(encoded.bytes.0.len())?;
            Ok(self.install_artifact(ArtifactRow {
                catalog: None,
                loaded_condition: None,
                basis: ArtifactBasis::Annotations {
                    snapshot: encoded.snapshot,
                    encoder: encoded.registration,
                },
                bytes: encoded.bytes,
                stopped: false,
            }))
        })();
        capture(recorder, Boundary::ArtifactAdmission, &result);
        result
    }
    /// Trusted-holder copies, not revocable leases or current recipient grants.
    /// Raw byte chunks may split Unicode/signatures; they are not edit positions.
    pub fn resource_artifact_bytes(
        &self,
        id: &ResourceArtifactId,
        start: usize,
        length: usize,
        recorder: &mut Recorder,
    ) -> Result<Vec<u8>, ResourceError> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            let row = self.resource_artifact(id)?;
            if row.stopped {
                return Err(ResourceError::Unavailable);
            }
            self.retained_document(&row.basis.source().document)
                .map_err(owner_error)?;
            let bytes = start
                .checked_add(length)
                .and_then(|end| row.bytes.0.get(start..end))
                .ok_or(ResourceError::Malformed)?;
            let mut copy = Vec::new();
            copy.try_reserve_exact(bytes.len())
                .map_err(|_| ResourceError::Capacity)?;
            copy.extend_from_slice(bytes);
            Ok(copy)
        })();
        capture(recorder, Boundary::ArtifactDelivery, &result);
        result
    }
    /// Protected stop affects future artifact delivery/admission, not effects
    /// that already own a capture, previous copies or retained byte charges.
    pub fn stop_resource_artifact(
        &mut self,
        id: &ResourceArtifactId,
        recorder: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let result = (|| {
            self.resource_artifact(id)?;
            self.resources.artifacts[id.slot]
                .as_mut()
                .expect("live artifact")
                .stopped = true;
            Ok(())
        })();
        capture(recorder, Boundary::ArtifactControl, &result);
        result
    }
    pub(super) fn remove_artifact(&mut self, id: &ResourceArtifactId) -> Result<(), ResourceError> {
        let row = self.resource_artifact(id)?;
        if self
            .resources
            .tasks
            .iter()
            .flatten()
            .any(|t| t.artifact == *id)
        {
            return Err(ResourceError::Retained);
        }
        let bytes = row.held_size().ok_or(ResourceError::Faulted)?;
        let Some(remaining) = self.resources.held_artifact_bytes.checked_sub(bytes) else {
            self.faulted = true;
            return Err(ResourceError::Faulted);
        };
        self.resources.artifacts[id.slot] = None;
        self.resources.held_artifact_bytes = remaining;
        Ok(())
    }
    pub fn retire_resource_artifact(
        &mut self,
        id: &ResourceArtifactId,
        recorder: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let result = self.remove_artifact(id);
        capture(recorder, Boundary::ArtifactControl, &result);
        result
    }
    /// Explicit export is a trusted-host read/export + destination-write command,
    /// not annotation authoring. It does not activate an ordinary binding, inherit
    /// source-save baseline rights, or infer permission from possession of a read.
    pub fn queue_annotation_export(
        &mut self,
        id: &ResourceArtifactId,
        destination: ResourceRead,
        requirements: SaveRequirements,
        recorder: &mut Recorder,
    ) -> Result<SaveId, ResourceError> {
        self.queue_annotation_export_checked(id, destination, requirements, None, None, recorder)
    }
    /// Explicit export with a captured ordinary association, not an ordinary save
    /// promising baseline installation. Publication still needs fresh evidence.
    pub fn queue_bound_annotation_export(
        &mut self,
        id: &ResourceArtifactId,
        destination: ResourceRead,
        requirements: SaveRequirements,
        pair: Option<OrdinaryPairEvidence>,
        recorder: &mut Recorder,
    ) -> Result<SaveId, ResourceError> {
        self.queue_annotation_export_checked(
            id,
            destination,
            requirements,
            Some(pair.as_ref()),
            None,
            recorder,
        )
    }
    pub(in crate::coordinator) fn queue_annotation_export_checked(
        &mut self,
        id: &ResourceArtifactId,
        destination: ResourceRead,
        requirements: SaveRequirements,
        association: Option<Option<&OrdinaryPairEvidence>>,
        catalog: Option<Arc<WriteCatalogCaptureV1>>,
        recorder: &mut Recorder,
    ) -> Result<SaveId, ResourceError> {
        let result = (|| {
            self.ensure_ordinary_admission().map_err(owner_error)?;
            self.provider_current(&destination.registration)?;
            self.writer_catalog_current(
                catalog.as_ref(),
                &destination.registration,
                &destination.identity,
                requirements.acknowledgment == Acknowledgment::Synchronized,
                false,
                recorder,
            )?;
            if destination.epoch.is_some() {
                return Err(ResourceError::Unsupported);
            }
            let artifact = self.resource_artifact(id)?;
            if artifact.stopped
                || (artifact.catalog.is_some()
                    && !WriteCatalogCaptureV1::same_intent(
                        artifact.catalog.as_ref(),
                        catalog.as_ref(),
                    ))
            {
                return Err(ResourceError::Unavailable);
            }
            let ArtifactBasis::Annotations { snapshot, encoder } = &artifact.basis else {
                return Err(ResourceError::Unsupported);
            };
            self.encoder_current(encoder)?;
            self.document(&snapshot.source().document)
                .map_err(owner_error)?;
            let annotation_association = if let Some(pair) = association {
                self.validate_ordinary_pair(&snapshot.source().document, pair)?;
                let binding = self
                    .resources
                    .annotation_bindings
                    .iter()
                    .find(|b| b.document == snapshot.source().document)
                    .ok_or(ResourceError::Unbound)?;
                if binding.key.identity != destination.identity
                    || !Arc::ptr_eq(&binding.key.registration, &destination.registration)
                {
                    return Err(ResourceError::Unavailable);
                }
                Some(AnnotationAssociation {
                    binding: binding.key.clone(),
                    source_binding: self
                        .resources
                        .bindings
                        .iter()
                        .find(|b| b.document == snapshot.source().document)
                        .map(Binding::key),
                })
            } else {
                None
            };
            if requirements.protection != Protection::BestEffort
                || (requirements.acknowledgment == Acknowledgment::Synchronized
                    && !destination.registration.profile.synchronization_supported)
            {
                return Err(ResourceError::Unsupported);
            }
            if self.resource_contested(&destination.identity) {
                return Err(ResourceError::Contested);
            }
            let limits = self.resources.limits.ok_or(ResourceError::Unsupported)?;
            if self.resources.tasks.len() >= limits.max_issued_tasks
                || self.resources.tasks.iter().flatten().count() >= limits.max_live_tasks
                || self
                    .resources
                    .tasks
                    .iter()
                    .flatten()
                    .filter(|t| t.identity == destination.identity && t.phase == SavePhase::Queued)
                    .count()
                    >= limits.max_queued_per_destination
            {
                return Err(ResourceError::Limit);
            }
            // A catalog export owns a fresh bounded artifact envelope. The
            // reusable caller artifact and its other destinations are unchanged.
            let copy = catalog.as_ref().map(|c| ArtifactRow {
                catalog: Some(c.clone()),
                loaded_condition: artifact.loaded_condition.clone(),
                basis: ArtifactBasis::Annotations {
                    snapshot: snapshot.clone(),
                    encoder: encoder.clone(),
                },
                bytes: artifact.bytes.clone(),
                stopped: false,
            });
            let export_artifact = if copy.is_some() {
                ResourceArtifactId {
                    owner: self.namespace.clone(),
                    slot: self.resources.artifacts.len(),
                }
            } else {
                id.clone()
            };
            let task = Task {
                queue_timing: queue_timing::Stamp::Off,
                catalog,
                operation: None,
                result_version_profile: SaveResultVersionProfileV1::Opaque,
                continuity: None,
                launched_condition: None,
                causal: Default::default(),
                source_export: None,
                protection: requirements.protection,
                serialization: None,
                baseline_claim: None,
                dispatch_serial: None,
                annotation_association,
                kind: ArtifactKind::Annotations,
                purpose: SavePurpose::AnnotationExport,
                artifact: export_artifact,
                captured_annotation_set: Some(snapshot.set_version()),
                encoding_context: Some(encoder.context.clone()),
                captured: snapshot.source().clone(),
                binding_generation: 0,
                identity: destination.identity,
                registration: destination.registration.clone(),
                payload: artifact.bytes.clone(),
                condition: WriteCondition::Present(Baseline {
                    epoch: None,
                    bytes: destination.bytes,
                    validator: Some(destination.validator),
                    anchor: None,
                }),
                synchronize: requirements.acknowledgment == Acknowledgment::Synchronized,
                phase: SavePhase::Queued,
                nonce: None,
                knowledge: Knowledge::default(),
                no_future: false,
                quiescent: false,
                stop_requested: false,
            };
            self.resources
                .tasks
                .try_reserve(1)
                .map_err(|_| ResourceError::Capacity)?;
            let id = SaveId {
                owner: self.namespace.clone(),
                slot: self.resources.tasks.len(),
            };
            if let Some(copy) = copy {
                let extra = copy
                    .held_size()
                    .and_then(|n| n.checked_sub(copy.bytes.0.len()))
                    .ok_or(ResourceError::Limit)?;
                self.preflight_artifact_capture(copy.bytes.0.len(), extra)?;
                self.install_artifact(copy);
            }
            self.resources.tasks.push(Some(task));
            let stamp = self.resource_queue_admitted();
            self.resources.tasks[id.slot]
                .as_mut()
                .expect("installed annotation save")
                .queue_timing = stamp;
            Ok(id)
        })();
        capture(recorder, Boundary::AnnotationExport, &result);
        result
    }
}
