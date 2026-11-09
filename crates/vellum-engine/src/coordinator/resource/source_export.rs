//! Explicit source destinations use the existing codec and single writer.
use super::*;
#[cfg(test)]
pub(super) mod tests;

#[derive(Clone, Copy)]
/// Supplied current host facts, not grants inferred from captured content or a
/// substitute for the owning adapter's authentication/policy evaluation.
pub struct TrustedSourceExportFactsV1 {
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub source_readable: Option<bool>,
    pub write_allowed: Option<bool>,
    pub binding_coverage_complete: Option<bool>,
    pub writer_coverage_complete: Option<bool>,
    pub provider_available: Option<bool>,
    pub resources_ready: Option<bool>,
    pub launch_allowed: Option<bool>,
    pub destination_allowed: Option<bool>,
    pub identity_allowed: Option<bool>,
    pub companions_ready: Option<bool>,
    pub versions_available: Option<bool>,
    pub reply_ready: Option<bool>,
    pub create_allowed: Option<bool>,
    pub overwrite_allowed: Option<bool>,
}
#[derive(Clone, Copy)]
pub struct TrustedSourceExportControlV1 {
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub binding_coverage_complete: Option<bool>,
    pub writer_coverage_complete: Option<bool>,
    pub binding_activation_allowed: Option<bool>,
    pub identity_allowed: Option<bool>,
    pub companions_ready: Option<bool>,
    pub versions_available: Option<bool>,
    pub reply_ready: Option<bool>,
    pub resources_ready: Option<bool>,
    pub disposition_readable: Option<bool>,
    pub reservation_release_allowed: Option<bool>,
}
#[derive(Debug)]
pub(super) struct ExportState {
    pub(super) marker: Arc<()>,
    pub(super) original_binding: Option<BindingKey>,
    pub(super) released: bool,
    pub(super) binding_applied: bool,
}
#[derive(Debug)]
pub(super) struct Reservation {
    pub(super) task: SaveId,
    pub(super) document: DocumentId,
    pub(super) identity: Identity,
}
fn current(f: &TrustedSourceExportFactsV1) -> bool {
    [
        f.authorized,
        f.scope,
        f.continuous,
        f.live,
        f.not_stopped,
        f.source_readable,
        f.write_allowed,
        f.binding_coverage_complete,
        f.writer_coverage_complete,
        f.provider_available,
        f.resources_ready,
        f.launch_allowed,
        f.destination_allowed,
        f.identity_allowed,
        f.companions_ready,
        f.versions_available,
        f.reply_ready,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn control_current(f: &TrustedSourceExportControlV1) -> bool {
    activation_current(f) && f.reservation_release_allowed == Some(true)
}
pub(super) fn activation_current(f: &TrustedSourceExportControlV1) -> bool {
    [
        f.authorized,
        f.scope,
        f.continuous,
        f.live,
        f.not_stopped,
        f.binding_coverage_complete,
        f.writer_coverage_complete,
        f.binding_activation_allowed,
        f.identity_allowed,
        f.companions_ready,
        f.versions_available,
        f.reply_ready,
        f.resources_ready,
        f.disposition_readable,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn synchronized(ack: Acknowledgment) -> bool {
    match ack {
        Acknowledgment::Ordinary => false,
        Acknowledgment::Synchronized => true,
    }
}
fn condition_allowed(
    c: &WriteCondition,
    reg: &Registration,
    requirements: SaveRequirements,
    f: &TrustedSourceExportFactsV1,
) -> Result<(), ResourceError> {
    if !current(f) {
        return Err(ResourceError::Unavailable);
    }
    if synchronized(requirements.acknowledgment) && !reg.profile.synchronization_supported {
        return Err(ResourceError::Unsupported);
    }
    let conditional = match requirements.protection {
        Protection::BestEffort => false,
        Protection::AtomicConditional => true,
    };
    match c {
        WriteCondition::Absent => {
            if f.create_allowed != Some(true) {
                return Err(ResourceError::Permission);
            }
            if !reg.destinations.exclusive_create {
                return Err(ResourceError::Unsupported);
            }
        }
        WriteCondition::Present(_) => {
            if f.overwrite_allowed != Some(true) {
                return Err(ResourceError::Permission);
            }
            if conditional && !reg.destinations.atomic_conditional_write {
                return Err(ResourceError::Unsupported);
            }
        }
    }
    Ok(())
}
impl SourceCoordinator {
    pub(super) fn export_original_binding(
        &self,
        document: &DocumentId,
    ) -> Result<Option<BindingKey>, ResourceError> {
        let mut rows = self
            .resources
            .bindings
            .iter()
            .filter(|b| b.document == *document);
        let first = rows.next();
        if rows.next().is_some() {
            return Err(ResourceError::Faulted);
        }
        if let Some(b) = first {
            self.provider_current(&b.registration)?;
        }
        Ok(first.map(Binding::key))
    }
    fn export_annotations(
        &self,
        document: &DocumentId,
    ) -> Result<Option<BindingKey>, ResourceError> {
        let mut rows = self
            .resources
            .annotation_bindings
            .iter()
            .filter(|b| b.document == *document);
        let first = rows.next();
        if rows.next().is_some() {
            return Err(ResourceError::Faulted);
        }
        if let Some(b) = first {
            self.provider_current(&b.key.registration)?;
        }
        Ok(first.map(|b| b.key.clone()))
    }
    pub(super) fn export_target_key(
        &self,
        document: &DocumentId,
        identity: &Identity,
        reg: &Arc<Registration>,
    ) -> Result<BindingKey, ResourceError> {
        let old = self.export_original_binding(document)?;
        let generation = match old {
            Some(b) if b.identity == *identity && Arc::ptr_eq(&b.registration, reg) => b.generation,
            Some(b) => b
                .generation
                .checked_add(1)
                .ok_or(ResourceError::VersionExhausted)?,
            None => 1,
        };
        Ok(BindingKey {
            identity: identity.clone(),
            registration: reg.clone(),
            generation,
        })
    }
    /// A current opaque host-comparison challenge, not proof from unequal keys.
    pub fn source_export_pair_check(
        &self,
        document: &DocumentId,
        target: &SourceExportDestinationV1,
        r: &mut Recorder,
    ) -> Result<Option<OrdinaryPairCheck>, ResourceError> {
        self.source_destination_pair_check(document, &target.identity, &target.registration, r)
    }
    pub(super) fn source_destination_pair_check(
        &self,
        document: &DocumentId,
        identity: &Identity,
        registration: &Arc<Registration>,
        r: &mut Recorder,
    ) -> Result<Option<OrdinaryPairCheck>, ResourceError> {
        let out = (|| {
            self.ensure_operational().map_err(owner_error)?;
            self.document(document).map_err(owner_error)?;
            self.provider_current(registration)?;
            let annotations = self.export_annotations(document)?;
            if annotations.is_none() {
                return Ok(None);
            }
            Ok(Some(OrdinaryPairCheck {
                owner: self.namespace.clone(),
                marker: Arc::new(()),
                document: document.clone(),
                source: Some(self.export_target_key(document, identity, registration)?),
                annotations,
                proposed: None,
            }))
        })();
        capture(r, Boundary::OrdinaryPairCheck, &out);
        out
    }
    pub(super) fn export_pair(
        &self,
        doc: &DocumentId,
        identity: &Identity,
        reg: &Arc<Registration>,
        purpose: SavePurpose,
        pair: Option<&OrdinaryPairEvidence>,
    ) -> Result<(), ResourceError> {
        match purpose {
            SavePurpose::SourceCopyExport => {
                return if pair.is_none() {
                    Ok(())
                } else {
                    Err(ResourceError::Malformed)
                };
            }
            SavePurpose::SourceSaveAs => {}
            SavePurpose::OrdinarySource
            | SavePurpose::OrdinaryAnnotations
            | SavePurpose::AnnotationExport => return Err(ResourceError::Faulted),
        }
        let Some(annotation) = self.export_annotations(doc)? else {
            return Ok(());
        };
        if annotation.identity == *identity {
            return Err(ResourceError::Conflict);
        }
        let e = pair.ok_or(ResourceError::MissingEvidence)?;
        if !Arc::ptr_eq(&e.check.owner, &self.namespace)
            || e.check.document != *doc
            || e.check.proposed.is_some()
            || e.check.annotations.as_ref() != Some(&annotation)
            || e.check.source.as_ref() != Some(&self.export_target_key(doc, identity, reg)?)
        {
            return Err(ResourceError::Stale);
        }
        match e.relation {
            ResourceRelation::Distinct => Ok(()),
            ResourceRelation::Equivalent => Err(ResourceError::Conflict),
            ResourceRelation::Unknown => Err(ResourceError::MissingEvidence),
        }
    }
    pub(super) fn export_owner(
        &self,
        doc: &DocumentId,
        identity: &Identity,
    ) -> Result<(), ResourceError> {
        let mut count = 0;
        for owner in self
            .resources
            .bindings
            .iter()
            .filter(|b| b.identity == *identity)
            .map(|b| &b.document)
            .chain(
                self.resources
                    .annotation_bindings
                    .iter()
                    .filter(|b| b.key.identity == *identity)
                    .map(|b| &b.document),
            )
        {
            if owner != doc {
                return Err(ResourceError::Conflict);
            }
            count += 1;
        }
        if count > 1 {
            return Err(ResourceError::Faulted);
        }
        if self
            .resources
            .source_export_reservations
            .iter()
            .any(|r| r.identity == *identity && r.document != *doc)
        {
            return Err(ResourceError::Conflict);
        }
        Ok(())
    }
    /// Consumes the original codec preparation on success or safe refusal.
    /// Copy export never binds; Save As activation remains a separate transition.
    pub fn queue_source_export(
        &mut self,
        prepared: &PreparedSourceArtifactIdV1,
        destination: SourceExportDestinationV1,
        options: SourceExportOptionsV1,
        facts: &TrustedSourceExportFactsV1,
        r: &mut Recorder,
    ) -> Result<SaveId, ResourceError> {
        self.queue_source_export_checked((prepared, None), destination, options, None, facts, r)
    }
    /// Consumes the preparation under the same lifecycle as v1, with an explicit
    /// after-result schema. V1 absence remains opaque even for causal providers.
    pub fn queue_source_export_v2(
        &mut self,
        prepared: &PreparedSourceArtifactIdV1,
        destination: SourceExportDestinationV1,
        options: SourceExportOptionsV2,
        facts: &TrustedSourceExportFactsV1,
        r: &mut Recorder,
    ) -> Result<SaveId, ResourceError> {
        self.queue_source_export_checked(
            (prepared, None),
            destination,
            options.intent,
            Some(options.result_version_profile),
            facts,
            r,
        )
    }
    pub(in crate::coordinator) fn queue_source_export_checked(
        &mut self,
        input: (
            &PreparedSourceArtifactIdV1,
            Option<Arc<WriteCatalogCaptureV1>>,
        ),
        destination: SourceExportDestinationV1,
        options: SourceExportOptionsV1,
        requested_profile: Option<SaveResultVersionProfileV1>,
        facts: &TrustedSourceExportFactsV1,
        r: &mut Recorder,
    ) -> Result<SaveId, ResourceError> {
        let (prepared, catalog) = input;
        let out = (|| {
            self.ensure_operational().map_err(owner_error)?;
            self.ensure_admission_unfenced().map_err(owner_error)?;
            if self.resources.source_save_admission_stopped {
                return Err(ResourceError::Unavailable);
            }
            let p = self.source_preparation(prepared)?;
            self.validate_version(&p.source).map_err(owner_error)?;
            let source = p.source.clone();
            let serialization = p.serialization;
            let payload = p.bytes.clone();
            self.provider_current(&destination.registration)?;
            self.writer_catalog_current(
                catalog.as_ref(),
                &destination.registration,
                &destination.identity,
                synchronized(options.requirements.acknowledgment),
                false,
                r,
            )?;
            let condition = match destination.condition {
                DispatchCondition::Absent => WriteCondition::Absent,
                DispatchCondition::Present {
                    bytes,
                    validator,
                    epoch,
                } => WriteCondition::Present(Baseline {
                    epoch,
                    bytes,
                    validator: Some(validator),
                    anchor: None,
                }),
            };
            let purpose = match options.purpose {
                SourceExportPurposeV1::CopyExport => SavePurpose::SourceCopyExport,
                SourceExportPurposeV1::SaveAs => SavePurpose::SourceSaveAs,
            };
            let inferred = match &condition {
                WriteCondition::Present(b) if b.epoch.is_some() => {
                    SaveResultVersionProfileV1::EpochToken
                }
                WriteCondition::Present(_) | WriteCondition::Absent => {
                    SaveResultVersionProfileV1::Opaque
                }
            };
            let result_version_profile = requested_profile.unwrap_or(inferred);
            if let Some(c) = &catalog {
                c.export_condition_allowed(
                    matches!(condition, WriteCondition::Absent),
                    options.requirements.protection,
                    result_version_profile,
                )?;
            }
            if !matches!(condition, WriteCondition::Absent) && result_version_profile != inferred {
                return Err(ResourceError::Unsupported);
            }
            if result_version_profile == SaveResultVersionProfileV1::EpochToken
                && !destination.registration.causal.causal_receipts
            {
                return Err(ResourceError::Unsupported);
            }
            condition_allowed(
                &condition,
                &destination.registration,
                options.requirements,
                facts,
            )?;
            self.export_owner(&source.document, &destination.identity)?;
            self.export_pair(
                &source.document,
                &destination.identity,
                &destination.registration,
                purpose,
                options.pair.as_ref(),
            )?;
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
            let original_binding = self.export_original_binding(&source.document)?;
            let binding_generation = match purpose {
                SavePurpose::SourceSaveAs => {
                    self.export_target_key(
                        &source.document,
                        &destination.identity,
                        &destination.registration,
                    )?
                    .generation
                }
                SavePurpose::SourceCopyExport => original_binding
                    .as_ref()
                    .filter(|b| {
                        b.identity == destination.identity
                            && Arc::ptr_eq(&b.registration, &destination.registration)
                    })
                    .map_or(0, |b| b.generation),
                SavePurpose::OrdinarySource
                | SavePurpose::OrdinaryAnnotations
                | SavePurpose::AnnotationExport => return Err(ResourceError::Faulted),
            };
            let growth = std::mem::size_of::<Reservation>()
                .checked_add(destination.identity.key.0.len())
                .ok_or(ResourceError::Limit)?;
            self.check_root_growth(growth, r).map_err(owner_error)?;
            let generation = self
                .next_publication_generation(None)
                .map_err(owner_error)?;
            self.resources
                .tasks
                .try_reserve(1)
                .map_err(|_| ResourceError::Capacity)?;
            self.resources
                .source_export_reservations
                .try_reserve(1)
                .map_err(|_| ResourceError::Capacity)?;
            let id = SaveId {
                owner: self.namespace.clone(),
                slot: self.resources.tasks.len(),
            };
            // All fallible/root/writer checks precede codec issuance. No callback
            // or effect occurs between this transfer and the two reserved pushes.
            let artifact =
                self.retain_prepared_source_artifact_catalog(prepared, catalog.clone(), r)?;
            let task = Task {
                queue_timing: queue_timing::Stamp::Off,
                catalog,
                operation: None,
                result_version_profile,
                continuity: None,
                launched_condition: None,
                causal: Default::default(),
                source_export: Some(ExportState {
                    marker: Arc::new(()),
                    original_binding,
                    released: false,
                    binding_applied: false,
                }),
                protection: options.requirements.protection,
                serialization: Some(serialization),
                baseline_claim: None,
                dispatch_serial: None,
                annotation_association: None,
                kind: ArtifactKind::Source,
                purpose,
                artifact,
                captured_annotation_set: None,
                encoding_context: None,
                captured: source.clone(),
                binding_generation,
                identity: destination.identity.clone(),
                registration: destination.registration,
                payload,
                condition,
                synchronize: synchronized(options.requirements.acknowledgment),
                phase: SavePhase::Queued,
                nonce: None,
                knowledge: Knowledge::default(),
                no_future: false,
                quiescent: false,
                stop_requested: false,
            };
            self.resources.tasks.push(Some(task));
            self.resources.source_export_reservations.push(Reservation {
                task: id.clone(),
                document: source.document,
                identity: destination.identity,
            });
            self.generation = generation;
            let stamp = self.resource_queue_admitted();
            self.resources.tasks[id.slot]
                .as_mut()
                .expect("installed export")
                .queue_timing = stamp;
            Ok(id)
        })();
        if out.is_err()
            && self.source_preparation(prepared).is_ok()
            && self
                .discard_source_artifact_preparation(prepared, r)
                .is_err()
        {
            self.faulted = true;
            let out = Err(ResourceError::Faulted);
            capture(r, Boundary::SourceExportAdmission, &out);
            return out;
        }
        if matches!(out, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture(r, Boundary::SourceExportAdmission, &out);
        out
    }

    pub fn dispatch_source_export(
        &mut self,
        id: &SaveId,
        probe: SourceExportDestinationV1,
        pair: Option<OrdinaryPairEvidence>,
        facts: &TrustedSourceExportFactsV1,
        r: &mut Recorder,
    ) -> Result<SaveDispatch, ResourceError> {
        self.dispatch_save_checked(id, probe, pair.as_ref(), Some(facts), None, r)
    }
    pub fn release_source_export_reservation(
        &mut self,
        id: &SaveId,
        facts: &TrustedSourceExportControlV1,
        r: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        let out = (|| {
            // Removing a destination owner is a semantic-root transition, unlike
            // private codec discard. A faulted root is not safe to republish.
            self.ensure_operational().map_err(owner_error)?;
            if !control_current(facts) {
                return Err(ResourceError::Unavailable);
            }
            let task = self.resource_task(id)?;
            let state = task
                .source_export
                .as_ref()
                .ok_or(ResourceError::Unsupported)?;
            let mut rows = self
                .resources
                .source_export_reservations
                .iter()
                .enumerate()
                .filter(|(_, r)| r.task == *id);
            let row = rows.next();
            if state.released {
                return if row.is_none() {
                    Ok(false)
                } else {
                    Err(ResourceError::Faulted)
                };
            }
            let (index, row) = row.ok_or(ResourceError::Faulted)?;
            if rows.next().is_some()
                || row.identity != task.identity
                || row.document != task.captured.document
            {
                return Err(ResourceError::Faulted);
            }
            if !task.safe() || self.resource_contested(&task.identity) {
                return Err(ResourceError::Retained);
            }
            let generation = self
                .next_retention_publication_generation(None)
                .map_err(owner_error)?;
            self.resources.source_export_reservations.swap_remove(index);
            self.resources.tasks[id.slot]
                .as_mut()
                .expect("validated original task")
                .source_export
                .as_mut()
                .expect("validated export")
                .released = true;
            self.generation = generation;
            Ok(true)
        })();
        if matches!(out, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture(r, Boundary::SourceExportReservationRelease, &out);
        out
    }

    pub(super) fn validate_source_export_dispatch(
        &self,
        id: &SaveId,
        task: &Task,
        facts: Option<&TrustedSourceExportFactsV1>,
        pair: Option<&OrdinaryPairEvidence>,
    ) -> Result<(), ResourceError> {
        let f = facts.ok_or(ResourceError::Unavailable)?;
        condition_allowed(
            &task.condition,
            &task.registration,
            SaveRequirements {
                protection: task.protection,
                acknowledgment: if task.synchronize {
                    Acknowledgment::Synchronized
                } else {
                    Acknowledgment::Ordinary
                },
            },
            f,
        )?;
        let state = task.source_export.as_ref().ok_or(ResourceError::Faulted)?;
        if state.released || task.stop_requested {
            return Err(ResourceError::Unavailable);
        }
        let mut rows = self
            .resources
            .source_export_reservations
            .iter()
            .filter(|r| r.task == *id);
        let row = rows.next().ok_or(ResourceError::Faulted)?;
        if rows.next().is_some()
            || row.document != task.captured.document
            || row.identity != task.identity
        {
            return Err(ResourceError::Faulted);
        }
        if self.export_original_binding(&task.captured.document)? != state.original_binding {
            return Err(ResourceError::Stale);
        }
        self.export_owner(&task.captured.document, &task.identity)?;
        self.export_pair(
            &task.captured.document,
            &task.identity,
            &task.registration,
            task.purpose,
            pair,
        )?;
        self.validate_source_capture(task)
    }
    pub(super) fn source_destination_reserved(&self, identity: &Identity) -> bool {
        self.resources
            .source_export_reservations
            .iter()
            .any(|r| r.identity == *identity)
    }
    pub(super) fn validate_export_retirement(
        &self,
        id: &SaveId,
        task: &Task,
    ) -> Result<(), ResourceError> {
        let mut rows = self
            .resources
            .source_export_reservations
            .iter()
            .filter(|r| r.task == *id);
        let row = rows.next();
        if rows.next().is_some()
            || row.is_some_and(|r| {
                r.document != task.captured.document || r.identity != task.identity
            })
            || matches!(
                task.purpose,
                SavePurpose::SourceCopyExport | SavePurpose::SourceSaveAs
            ) != task.source_export.is_some()
            || (task.source_export.is_some() && task.kind != ArtifactKind::Source)
        {
            return Err(ResourceError::Faulted);
        }
        match &task.source_export {
            Some(s) if s.released && row.is_none() => Ok(()),
            Some(s) if !s.released && row.is_some() => Err(ResourceError::Retained),
            Some(_) => Err(ResourceError::Faulted),
            None if row.is_some() => Err(ResourceError::Faulted),
            None => Ok(()),
        }
    }
}
