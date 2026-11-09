//! Atomic native ordinary annotation save admission and capacity obligations.
//! Codec work is outside this owner. Transient producer cost remains host-funded;
//! no JSON wire-envelope or complete physical/global allocation proof is implied.
use super::*;
impl SourceCoordinator {
    /// Fence new ordinary source and source-export work, not existing dispatch,
    /// result intake, baseline settlement or protected retirement.
    pub fn stop_source_save_admission_v1(&mut self, recorder: &mut Recorder) {
        let timing = recorder.begin_service_timing();
        self.resources.source_save_admission_stopped = true;
        capture_service(
            recorder,
            Boundary::BaselineCapacityControl,
            &Ok::<(), ResourceError>(()),
            timing.finish(),
        );
    }
    pub fn source_save_admission_stopped_v1(&self) -> bool {
        self.resources.source_save_admission_stopped
    }
    /// Stops new ordinary annotation admissions, never existing obligations.
    pub fn stop_annotation_save_admission(&mut self, recorder: &mut Recorder) {
        self.resources.annotation_save_admission_stopped = true;
        capture(
            recorder,
            Boundary::BaselineCapacityControl,
            &Ok::<(), ResourceError>(()),
        );
    }
    /// Use the admitted native budget rather than asking the caller to fund a
    /// second baseline. A later token enrichment still needs fresh publication
    /// headroom once the original promise has been installed.
    pub fn install_annotation_save_baseline(
        &mut self,
        id: &SaveId,
        pair: Option<OrdinaryPairEvidence>,
        recorder: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        let budget = self.resource_task(id).and_then(|task| {
            let claim = task
                .baseline_claim
                .as_ref()
                .ok_or(ResourceError::Unsupported)?;
            match claim.state {
                BaselineCapacityState::Pending | BaselineCapacityState::Installed => {
                    Ok(claim.charge)
                }
                BaselineCapacityState::Superseded => Err(ResourceError::Superseded),
                _ => Err(ResourceError::AcknowledgmentUnsatisfied),
            }
        });
        match budget {
            Ok(budget) => self.publish_annotation_baseline(id, pair, budget, recorder),
            Err(error) => {
                let result = Err(error);
                capture(recorder, Boundary::AnnotationBaselinePublication, &result);
                result
            }
        }
    }
    pub(super) fn pending_baseline_units(&self, consume: Option<&SaveId>) -> usize {
        self.resources
            .tasks
            .iter()
            .enumerate()
            .filter(|(slot, t)| {
                consume.is_none_or(|id| id.slot != *slot)
                    && t.as_ref().is_some_and(|t| {
                        t.baseline_claim
                            .as_ref()
                            .is_some_and(|c| c.state == BaselineCapacityState::Pending)
                    })
            })
            .count()
    }
    pub(in crate::coordinator) fn next_publication_generation(
        &self,
        consume: Option<&SaveId>,
    ) -> Result<u64, CoordinatorError> {
        if consume.is_none() {
            self.ensure_admission_unfenced()?;
        }
        self.next_retention_publication_generation(consume)
    }
    /// Internal counter calculation for actual protected retirement/bookkeeping.
    /// The calling owner still validates its exact operation and discharge facts;
    /// this is not a public mutation bypass or an admission capability.
    pub(in crate::coordinator) fn next_retention_publication_generation(
        &self,
        consume: Option<&SaveId>,
    ) -> Result<u64, CoordinatorError> {
        let next = self
            .generation
            .checked_add(1)
            .ok_or(CoordinatorError::VersionExhausted)?;
        let reserved = u64::try_from(self.pending_baseline_units(consume))
            .map_err(|_| CoordinatorError::VersionExhausted)?;
        next.checked_add(reserved)
            .ok_or(CoordinatorError::VersionExhausted)?;
        Ok(next)
    }
    /// Consumes a current, issuer-validated transient candidate, not an issued
    /// artifact. All expected refusal occurs before either persistent issuance.
    /// The explicit present-destination read is the original pinned condition;
    /// final dispatch never refreshes it from a newer read or baseline.
    pub fn queue_annotation_save(
        &mut self,
        encoded: EncodedAnnotations,
        destination: ResourceRead,
        requirements: SaveRequirements,
        pair: Option<OrdinaryPairEvidence>,
        recorder: &mut Recorder,
    ) -> Result<SaveId, ResourceError> {
        self.queue_annotation_save_catalog(encoded, destination, requirements, pair, None, recorder)
    }
    pub(in crate::coordinator) fn queue_annotation_save_catalog(
        &mut self,
        encoded: EncodedAnnotations,
        destination: ResourceRead,
        requirements: SaveRequirements,
        pair: Option<OrdinaryPairEvidence>,
        catalog: Option<Arc<WriteCatalogCaptureV1>>,
        recorder: &mut Recorder,
    ) -> Result<SaveId, ResourceError> {
        self.queue_annotation_save_with_parent(
            encoded,
            destination,
            requirements,
            pair,
            (catalog, None),
            recorder,
        )
    }
    pub(super) fn queue_annotation_save_with_parent(
        &mut self,
        encoded: EncodedAnnotations,
        destination: ResourceRead,
        requirements: SaveRequirements,
        pair: Option<OrdinaryPairEvidence>,
        installation: (Option<Arc<WriteCatalogCaptureV1>>, Option<operations::Row>),
        recorder: &mut Recorder,
    ) -> Result<SaveId, ResourceError> {
        let timing = recorder.begin_service_timing();
        let (catalog, parent) = installation;
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.ensure_operational().map_err(owner_error)?;
            self.ensure_admission_unfenced().map_err(owner_error)?;
            if self.resources.annotation_save_admission_stopped {
                return Err(ResourceError::Unavailable);
            }
            let limits = self.resources.limits.ok_or(ResourceError::Unsupported)?;
            self.encoder_current(&encoded.registration)?;
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
            if !self.knows_annotation_snapshot(&encoded.snapshot) {
                return Err(ResourceError::Unavailable);
            }
            let source = encoded.snapshot.source();
            let document = self.validate_version(source).map_err(owner_error)?;
            if encoded.snapshot.set_version() != document.annotation_generation
                || encoded.snapshot.publication_generation() != self.generation
            {
                return Err(ResourceError::Stale);
            }
            if document.annotation_read_only {
                return Err(ResourceError::Permission);
            }
            self.validate_ordinary_pair(&source.document, pair.as_ref())?;
            let association = self
                .current_annotation_association(&source.document)
                .ok_or(ResourceError::Unbound)?;
            if association.binding.identity != destination.identity
                || !Arc::ptr_eq(&association.binding.registration, &destination.registration)
            {
                return Err(ResourceError::Unavailable);
            }
            // Capture the actual loaded producer, not a caller-supplied token
            // or whatever baseline happens to occupy the slot at dispatch.
            let loaded_condition = match self
                .resources
                .annotation_bindings
                .iter()
                .find(|b| b.document == source.document)
            {
                Some(binding) => match &binding.baseline {
                    Some(AnnotationBaselineSlot::Loaded(baseline)) => {
                        self.validate_loaded_baseline(binding, baseline)?;
                        if destination.bytes != baseline.bytes
                            || destination.validator != baseline.version
                        {
                            return Err(ResourceError::Conflict);
                        }
                        if encoded.registration.context != baseline.fact.encoding_context {
                            return Err(ResourceError::Stale);
                        }
                        Some(baseline.clone())
                    }
                    _ => None,
                },
                None => return Err(ResourceError::Unbound),
            };
            let condition_charge = match &loaded_condition {
                Some(condition) => condition.retained_size().ok_or(ResourceError::Limit)?,
                None => 0,
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
            let units = self
                .pending_baseline_units(None)
                .checked_add(1)
                .and_then(|n| u64::try_from(n).ok())
                .ok_or(ResourceError::VersionExhausted)?;
            self.generation
                .checked_add(units)
                .ok_or(ResourceError::VersionExhausted)?;
            // Native descriptor fields are fixed-size except already-retained
            // payload/keys and the provider's bounded optional version bytes.
            let future = super::root_accounting::future_baseline_payload(
                &encoded.bytes,
                &association,
                &destination.registration,
            )
            .map_err(|_| ResourceError::Limit)?;
            let charge = super::root_accounting::baseline_claim_charge(future)
                .map_err(|_| ResourceError::Limit)?;
            let reserved = self
                .resources
                .tasks
                .iter()
                .flatten()
                .filter_map(|t| t.baseline_claim.as_ref())
                .filter(|c| c.state == BaselineCapacityState::Pending)
                .try_fold(0usize, |n, c| n.checked_add(c.charge))
                .ok_or(ResourceError::Faulted)?;
            if reserved
                .checked_add(charge)
                .is_none_or(|n| n > limits.max_future_baseline_bytes)
            {
                return Err(ResourceError::Limit);
            }
            let id = SaveId {
                owner: self.namespace.clone(),
                slot: self.resources.tasks.len(),
            };
            let artifact_id = ResourceArtifactId {
                owner: self.namespace.clone(),
                slot: self.resources.artifacts.len(),
            };
            let binding_generation = association.binding.generation;
            let mut task = Task {
                queue_timing: queue_timing::Stamp::Off,
                catalog: catalog.clone(),
                operation: None,
                result_version_profile: SaveResultVersionProfileV1::Opaque,
                continuity: None,
                launched_condition: None,
                causal: Default::default(),
                source_export: None,
                protection: requirements.protection,
                serialization: None,
                baseline_claim: Some(BaselineClaim {
                    state: BaselineCapacityState::Pending,
                    charge,
                }),
                dispatch_serial: None,
                annotation_association: Some(association),
                kind: ArtifactKind::Annotations,
                purpose: SavePurpose::OrdinaryAnnotations,
                artifact: artifact_id,
                captured_annotation_set: Some(encoded.snapshot.set_version()),
                encoding_context: Some(encoded.registration.context.clone()),
                captured: source.clone(),
                binding_generation,
                identity: destination.identity,
                registration: destination.registration,
                payload: encoded.bytes.clone(),
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
            self.prepare_save_parent(&mut task, parent.as_ref())?;
            self.check_root_replacement_projected(
                0,
                0,
                BaselineRootProjection {
                    admission: Some(&task),
                    ..BaselineRootProjection::default()
                },
                recorder,
            )
            .map_err(owner_error)?;
            self.resources
                .tasks
                .try_reserve(1)
                .map_err(|_| ResourceError::Capacity)?;
            let catalog_bytes = catalog
                .as_ref()
                .map(|c| c.native_bytes())
                .transpose()?
                .unwrap_or(0);
            self.preflight_artifact_capture(
                encoded.bytes.0.len(),
                condition_charge
                    .checked_add(catalog_bytes)
                    .ok_or(ResourceError::Limit)?,
            )?;
            // Infallible final installation under exclusive owner access. No
            // callback, await, receipt or externally visible intermediate handle.
            let artifact = ArtifactRow {
                catalog,
                loaded_condition,
                basis: ArtifactBasis::Annotations {
                    snapshot: encoded.snapshot,
                    encoder: encoded.registration,
                },
                bytes: encoded.bytes,
                stopped: false,
            };
            self.install_save_admission(task, artifact, parent, false);
            Ok(id)
        }));
        let result = match caught {
            Ok(v) => v,
            Err(p) => {
                self.faulted = true;
                capture_service(
                    recorder,
                    Boundary::AnnotationSaveAdmission,
                    &Err::<SaveId, _>(ResourceError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(p)
            }
        };
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(
            recorder,
            Boundary::AnnotationSaveAdmission,
            &result,
            timing.finish(),
        );
        result
    }
    /// Preflight has reserved both vectors and (where present) the parent pool.
    /// No callback, provider I/O, allocation or fallible decision is permitted
    /// between the first install and clearing this fault fence.
    pub(super) fn install_save_admission(
        &mut self,
        task: Task,
        artifact: ArtifactRow,
        parent: Option<operations::Row>,
        own_chain: bool,
    ) {
        self.faulted = true;
        #[cfg(test)]
        assert_ne!(
            self.publication_fault_after,
            Some(0),
            "injected save admission interruption"
        );
        self.install_artifact(artifact);
        #[cfg(test)]
        assert_ne!(
            self.publication_fault_after,
            Some(1),
            "injected save admission interruption"
        );
        self.resources.tasks.push(Some(task));
        #[cfg(test)]
        assert_ne!(
            self.publication_fault_after,
            Some(2),
            "injected save admission interruption"
        );
        if own_chain {
            self.install_continuity_credit();
        }
        #[cfg(test)]
        assert_ne!(
            self.publication_fault_after,
            Some(3),
            "injected save admission interruption"
        );
        self.install_save_parent(parent);
        #[cfg(test)]
        assert_ne!(
            self.publication_fault_after,
            Some(4),
            "injected save admission interruption"
        );
        self.faulted = false;
        let stamp = self.resource_queue_admitted();
        self.resources
            .tasks
            .last_mut()
            .and_then(Option::as_mut)
            .expect("installed save")
            .queue_timing = stamp;
    }
    /// Protected cleanup: release the promise, never erase write evidence,
    /// cancel an applied effect, recycle IDs or discharge baseline/producer pins.
    pub fn release_annotation_save_capacity(
        &mut self,
        id: &SaveId,
        recorder: &mut Recorder,
    ) -> Result<BaselineCapacityState, ResourceError> {
        self.release_save_capacity(id, ArtifactKind::Annotations, recorder)
    }
    /// Protected release of an impossible/superseded ordinary source promise.
    /// Does not erase evidence, retire the producer or discharge writer leases.
    pub fn release_source_save_capacity_v1(
        &mut self,
        id: &SaveId,
        recorder: &mut Recorder,
    ) -> Result<BaselineCapacityState, ResourceError> {
        self.release_save_capacity(id, ArtifactKind::Source, recorder)
    }
    fn release_save_capacity(
        &mut self,
        id: &SaveId,
        kind: ArtifactKind,
        recorder: &mut Recorder,
    ) -> Result<BaselineCapacityState, ResourceError> {
        let timing = recorder.begin_service_timing();
        let result = (|| {
            let task = self.resource_task(id)?;
            if task.kind != kind {
                return Err(ResourceError::Unsupported);
            }
            let claim = task
                .baseline_claim
                .as_ref()
                .ok_or(ResourceError::Unsupported)?;
            if self.resource_contested(&task.identity) {
                return Err(ResourceError::Contested);
            }
            if claim.state != BaselineCapacityState::Pending {
                return Ok(claim.state);
            }
            // This claim is private inline state with no exported hold lease.
            // Writer receipt/dispatch leases have a different lifetime and still
            // block task retirement, not release of an impossible capacity promise.
            if !task.safe() || !task.quiescent {
                return Err(ResourceError::Retained);
            }
            let state = self.releasable_baseline_capacity(id, task)?;
            self.resources.tasks[id.slot]
                .as_mut()
                .expect("retained task")
                .baseline_claim
                .as_mut()
                .expect("checked claim")
                .state = state;
            Ok(state)
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(
            recorder,
            Boundary::BaselineCapacityControl,
            &result,
            timing.finish(),
        );
        result
    }
    pub(super) fn releasable_baseline_capacity(
        &self,
        id: &SaveId,
        task: &Task,
    ) -> Result<BaselineCapacityState, ResourceError> {
        if self.resource_contested(&task.identity) {
            return Err(ResourceError::Contested);
        }
        if !task.safe() || !task.quiescent {
            return Err(ResourceError::Retained);
        }
        let state = if task.knowledge.data.application == Application::NotWritten {
            BaselineCapacityState::NotWritten
        } else if task.knowledge.data.application == Application::Written
            && task.synchronize
            && matches!(
                task.knowledge.data.synchronization,
                Synchronization::Failed | Synchronization::NotRequested
            )
        {
            BaselineCapacityState::TerminalIneligible
        } else if task.qualified() && self.actual_superseding_baseline(id, task)? {
            BaselineCapacityState::Superseded
        } else {
            return Err(ResourceError::Retained);
        };
        Ok(state)
    }
    /// Control-only proof from actual retained producer metadata, not a caller
    /// label, newer set number, provider lookup or copied content/byte envelope.
    fn actual_superseding_baseline(&self, id: &SaveId, task: &Task) -> Result<bool, ResourceError> {
        if task.kind == ArtifactKind::Source {
            let Some(binding) = self.resources.bindings.iter().find(|b| {
                b.document == task.captured.document
                    && b.identity == task.identity
                    && b.generation == task.binding_generation
                    && Arc::ptr_eq(&b.registration, &task.registration)
            }) else {
                return Ok(false);
            };
            let Some(anchor) = binding.baseline_anchor() else {
                return Ok(false);
            };
            if anchor == id {
                return Ok(false);
            }
            let producer = self
                .resource_task(anchor)
                .map_err(|_| ResourceError::Faulted)?;
            let baseline = binding.baseline.as_ref().ok_or(ResourceError::Faulted)?;
            if !producer.source_baseline_candidate()
                || producer.captured.document != task.captured.document
                || producer.identity != task.identity
                || producer.binding_generation != task.binding_generation
                || !Arc::ptr_eq(&producer.registration, &task.registration)
                || !producer.qualified()
                || baseline.bytes != producer.payload
                || baseline
                    .validator
                    .as_ref()
                    .is_some_and(|v| producer.knowledge.data.version.as_ref() != Some(v))
                || baseline
                    .epoch
                    .as_ref()
                    .is_some_and(|v| producer.knowledge.data.epoch.as_ref() != Some(v))
            {
                return Err(ResourceError::Faulted);
            }
            self.validate_source_capture(producer)
                .map_err(|_| ResourceError::Faulted)?;
            return Ok(producer.dispatch_serial > task.dispatch_serial);
        }
        let Some(binding) = self
            .resources
            .annotation_bindings
            .iter()
            .find(|b| b.document == task.captured.document)
        else {
            return Ok(false);
        };
        let Some(baseline) = binding
            .baseline
            .as_ref()
            .and_then(AnnotationBaselineSlot::written)
        else {
            return Ok(false);
        };
        if baseline.fact.producer == *id
            || Some(baseline.fact.dispatch_serial) <= task.dispatch_serial
        {
            return Ok(false);
        }
        let producer = self
            .resource_task(&baseline.fact.producer)
            .map_err(|_| ResourceError::Faulted)?;
        if task.annotation_association.as_ref() != Some(&baseline.association)
            || baseline.association.binding != binding.key
            || producer.annotation_association != task.annotation_association
            || !producer.qualified()
            || producer.artifact != baseline.fact.artifact
            || producer.captured != baseline.fact.source
            || producer.captured_annotation_set != Some(baseline.fact.set_version)
            || producer.encoding_context.as_ref() != Some(&baseline.fact.encoding_context)
            || producer.dispatch_serial != Some(baseline.fact.dispatch_serial)
            || producer.identity != binding.key.identity
            || !Arc::ptr_eq(&producer.registration, &binding.key.registration)
        {
            return Err(ResourceError::Faulted);
        }
        Ok(true)
    }
}
