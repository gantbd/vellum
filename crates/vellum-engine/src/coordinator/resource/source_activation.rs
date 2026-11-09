//! Local acknowledged Save As publication; no provider I/O or compensating write.
use super::*;
mod authority;
pub use authority::{SourceSaveAsAuthorityFactsV1, SourceSaveAsAuthorityV1};
#[cfg(test)]
mod tests;

/// Current host assertions, not document-derived authority. A present probe can
/// carry an explicitly unavailable external version; it never invents a token.
pub struct TrustedSourceSaveAsFactsV1 {
    pub control: TrustedSourceExportControlV1,
    pub probe_attested: Option<bool>,
    pub resource_continuity_attested: Option<bool>,
    pub create_allowed: Option<bool>,
    pub overwrite_allowed: Option<bool>,
}
pub(in crate::coordinator) fn source_save_as_authorized(
    facts: &TrustedSourceSaveAsFactsV1,
) -> bool {
    source_export::activation_current(&facts.control)
        && facts.probe_attested == Some(true)
        && facts.resource_continuity_attested == Some(true)
}
use crate::coordinator::root_identity::NativeRootIdentity;
use std::sync::Weak;
pub(in crate::coordinator) struct PreparedSourceSaveAs {
    before: NativeRootIdentity,
    id: SaveId,
    record: Weak<()>,
    probe: SourceSaveAsProbeV1,
    pair: Option<OrdinaryPairEvidence>,
    maximum: usize,
    decision: Decision,
}
enum Decision {
    Current,
    Install {
        binding: Binding,
        index: Option<usize>,
        archive: bool,
        reservation: usize,
        publication: u64,
    },
}
impl Decision {
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Current, Self::Current) => true,
            (
                Self::Install {
                    binding: a,
                    index: ai,
                    archive: aa,
                    reservation: ar,
                    publication: ap,
                },
                Self::Install {
                    binding: b,
                    index: bi,
                    archive: ba,
                    reservation: br,
                    publication: bp,
                },
            ) => {
                ai == bi
                    && aa == ba
                    && ar == br
                    && ap == bp
                    && a.document == b.document
                    && a.identity == b.identity
                    && Arc::ptr_eq(&a.registration, &b.registration)
                    && a.generation == b.generation
                    && match (&a.baseline, &b.baseline) {
                        (Some(a), Some(b)) => {
                            a.epoch == b.epoch
                                && a.bytes == b.bytes
                                && a.validator == b.validator
                                && a.anchor == b.anchor
                        }
                        _ => false,
                    }
            }
            _ => false,
        }
    }
}
impl SourceCoordinator {
    /// Local acknowledged Save As only; no write, compensation or provider callback.
    pub fn activate_source_save_as(
        &mut self,
        id: &SaveId,
        probe: SourceSaveAsProbeV1,
        pair: Option<OrdinaryPairEvidence>,
        max_binding_history: usize,
        facts: &TrustedSourceSaveAsFactsV1,
        r: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let plan =
                self.prepare_source_save_as_native(id, probe, pair, max_binding_history, facts, r)?;
            self.commit_source_save_as_native(plan, facts, r)
        }));
        let out = match caught {
            Ok(v) => v,
            Err(p) => {
                self.faulted = true;
                capture_service(
                    r,
                    Boundary::SourceSaveAsActivation,
                    &Err::<bool, _>(ResourceError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(p)
            }
        };
        if matches!(out, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture_service(r, Boundary::SourceSaveAsActivation, &out, timing.finish());
        out
    }
    fn validate_source_save_as_native(
        &self,
        id: &SaveId,
        probe: &SourceSaveAsProbeV1,
        pair: Option<&OrdinaryPairEvidence>,
        max_binding_history: usize,
        facts: &TrustedSourceSaveAsFactsV1,
        r: &mut Recorder,
    ) -> Result<Decision, ResourceError> {
        self.ensure_operational().map_err(owner_error)?;
        if !source_save_as_authorized(facts) {
            return Err(ResourceError::Unavailable);
        }
        let task = self.resource_task(id)?;
        if task.purpose != SavePurpose::SourceSaveAs {
            return Err(ResourceError::Unsupported);
        }
        self.writer_catalog_current(
            task.catalog.as_ref(),
            &task.registration,
            &task.identity,
            task.synchronize,
            false,
            r,
        )?;
        let state = task.source_export.as_ref().ok_or(ResourceError::Faulted)?;
        self.document(&task.captured.document)
            .map_err(owner_error)?;
        self.provider_current(&task.registration)?;
        self.validate_source_capture(task)?;
        if task.stop_requested || !task.qualified() || !task.safe() {
            return Err(ResourceError::AcknowledgmentUnsatisfied);
        }
        if self.resource_contested(&task.identity) {
            return Err(ResourceError::Contested);
        }
        let current = self.export_original_binding(&task.captured.document)?;
        let target = BindingKey {
            identity: task.identity.clone(),
            registration: task.registration.clone(),
            generation: task.binding_generation,
        };
        if state.binding_applied {
            if !state.released
                || self
                    .resources
                    .source_export_reservations
                    .iter()
                    .any(|r| r.task == *id)
            {
                return Err(ResourceError::Faulted);
            }
            return if current.as_ref() == Some(&target) {
                Ok(Decision::Current)
            } else {
                Err(ResourceError::Stale)
            };
        }
        if state.released || current != state.original_binding {
            return Err(ResourceError::Stale);
        }
        if self.export_target_key(&task.captured.document, &task.identity, &task.registration)?
            != target
        {
            return Err(ResourceError::Faulted);
        }
        match task.condition {
            WriteCondition::Absent => {
                if facts.create_allowed != Some(true) {
                    return Err(ResourceError::Permission);
                }
            }
            WriteCondition::Present(_) => {
                if facts.overwrite_allowed != Some(true) {
                    return Err(ResourceError::Permission);
                }
            }
        }
        self.provider_current(&probe.registration)?;
        if probe.identity != task.identity
            || !Arc::ptr_eq(&probe.registration, &task.registration)
            || probe.version != task.knowledge.data.version
            || probe.epoch != task.knowledge.data.epoch
        {
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
        let mut reservations = self
            .resources
            .source_export_reservations
            .iter()
            .enumerate()
            .filter(|(_, r)| r.task == *id);
        let (reservation_index, reservation) = reservations.next().ok_or(ResourceError::Faulted)?;
        if reservations.next().is_some()
            || reservation.document != task.captured.document
            || reservation.identity != task.identity
        {
            return Err(ResourceError::Faulted);
        }

        // Both physical effect domains must be resolved, regardless of
        // producer quiescence. Unknown writes never become "not written".
        for identity in std::iter::once(&task.identity).chain(current.iter().map(|b| &b.identity)) {
            if self.resource_contested(identity) {
                return Err(ResourceError::Contested);
            }
            if self
                .resources
                .tasks
                .iter()
                .flatten()
                .any(|t| t.identity == *identity && !t.safe())
            {
                return Err(ResourceError::Retained);
            }
        }
        if current.as_ref().is_some_and(|old| {
            old.identity != task.identity && self.source_destination_reserved(&old.identity)
        }) || self.resources.tasks.iter().flatten().any(|t| {
            t.captured.document == task.captured.document
                && t.baseline_claim
                    .as_ref()
                    .is_some_and(|c| c.state == BaselineCapacityState::Pending)
        }) {
            return Err(ResourceError::Retained);
        }
        let changed = current.as_ref() != Some(&target);
        let archive = changed && current.is_some();
        let history =
            self.validate_source_binding_history(&task.captured.document, max_binding_history)?;
        if history
            .checked_add(usize::from(archive))
            .is_none_or(|n| n > max_binding_history)
        {
            return Err(ResourceError::Limit);
        }
        let index = self
            .resources
            .bindings
            .iter()
            .position(|b| b.document == task.captured.document);
        let baseline = Baseline {
            epoch: task.knowledge.data.epoch.clone(),
            bytes: task.payload.clone(),
            validator: task.knowledge.data.version.clone(),
            anchor: Some(id.clone()),
        };
        let reservation_bytes = std::mem::size_of::<source_export::Reservation>()
            .checked_add(task.identity.key.0.len())
            .ok_or(ResourceError::Limit)?;
        let mut removed = reservation_bytes;
        let mut added = baseline
            .root_payload_bytes()
            .map_err(|_| ResourceError::Limit)?;
        if changed {
            // The old row moves intact into history, retaining its old
            // baseline producer. Only the new current row adds occupancy.
            added = added
                .checked_add(std::mem::size_of::<Binding>())
                .and_then(|n| n.checked_add(task.identity.key.0.len()))
                .ok_or(ResourceError::Limit)?;
        } else {
            let old = &self.resources.bindings[index.ok_or(ResourceError::Faulted)?].baseline;
            removed = removed
                .checked_add(
                    old.as_ref()
                        .map_or(Ok(0), Baseline::root_payload_bytes)
                        .map_err(|_| ResourceError::Limit)?,
                )
                .ok_or(ResourceError::Limit)?;
        }
        self.check_root_replacement(removed, added, r)
            .map_err(owner_error)?;
        let publication = self
            .next_publication_generation(None)
            .map_err(owner_error)?;
        let document_id = task.captured.document.clone();
        let binding = Binding {
            document: document_id.clone(),
            identity: target.identity,
            registration: target.registration,
            generation: target.generation,
            baseline: Some(baseline),
        };

        Ok(Decision::Install {
            binding,
            index,
            archive,
            reservation: reservation_index,
            publication,
        })
    }
    pub(in crate::coordinator) fn prepare_source_save_as_native(
        &mut self,
        id: &SaveId,
        probe: SourceSaveAsProbeV1,
        pair: Option<OrdinaryPairEvidence>,
        maximum: usize,
        facts: &TrustedSourceSaveAsFactsV1,
        r: &mut Recorder,
    ) -> Result<PreparedSourceSaveAs, ResourceError> {
        let decision =
            self.validate_source_save_as_native(id, &probe, pair.as_ref(), maximum, facts, r)?;
        if let Decision::Install {
            binding,
            index,
            archive,
            ..
        } = &decision
        {
            if *archive {
                self.documents[binding.document.slot]
                    .as_mut()
                    .expect("validated live owner")
                    .retained_bindings
                    .source_history
                    .try_reserve(1)
                    .map_err(|_| ResourceError::Capacity)?;
            }
            if index.is_none() {
                self.resources
                    .bindings
                    .try_reserve(1)
                    .map_err(|_| ResourceError::Capacity)?;
            }
        }
        let state = self
            .resource_task(id)?
            .source_export
            .as_ref()
            .ok_or(ResourceError::Faulted)?;
        let record = Arc::downgrade(&state.marker);
        let before = self
            .native_root_identity()
            .map_err(|_| ResourceError::Faulted)?;
        Ok(PreparedSourceSaveAs {
            before,
            id: id.clone(),
            record,
            probe,
            pair,
            maximum,
            decision,
        })
    }
    pub(in crate::coordinator) fn commit_source_save_as_native(
        &mut self,
        plan: PreparedSourceSaveAs,
        facts: &TrustedSourceSaveAsFactsV1,
        r: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        self.commit_source_save_as_for_invocation(plan, facts, None, r)
    }
    pub(in crate::coordinator) fn commit_source_save_as_for_invocation(
        &mut self,
        plan: PreparedSourceSaveAs,
        facts: &TrustedSourceSaveAsFactsV1,
        invocation: Option<&InvocationId>,
        r: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        self.ensure_operational().map_err(owner_error)?;
        if self
            .native_root_identity()
            .map_err(|_| ResourceError::Faulted)?
            != plan.before
        {
            return Err(ResourceError::Stale);
        }
        let state = self
            .resource_task(&plan.id)?
            .source_export
            .as_ref()
            .ok_or(ResourceError::Faulted)?;
        if !Weak::ptr_eq(&plan.record, &Arc::downgrade(&state.marker)) {
            return Err(ResourceError::Stale);
        }
        let fresh = self.validate_source_save_as_native(
            &plan.id,
            &plan.probe,
            plan.pair.as_ref(),
            plan.maximum,
            facts,
            r,
        )?;
        if !plan.decision.same(&fresh) {
            return Err(ResourceError::Stale);
        }
        let changed = matches!(&plan.decision, Decision::Install { .. });
        let document = self.resource_task(&plan.id)?.captured.document.clone();
        let terminal = invocation
            .map(|id| {
                self.stage_source_save_as_candidate(
                    id,
                    &plan.id,
                    &document,
                    (&plan.probe, &plan.pair, plan.maximum),
                    changed,
                )
            })
            .transpose()?;
        let Decision::Install {
            binding,
            index,
            archive,
            reservation,
            publication,
        } = plan.decision
        else {
            if let (Some(id), Some(terminal)) = (invocation, terminal) {
                self.faulted = true;
                self.install_invocation_terminal(id, terminal);
                self.faulted = false;
            }
            return Ok(false);
        };
        self.faulted = true;
        #[cfg(test)]
        assert_ne!(
            self.publication_fault_after,
            Some(0),
            "injected Save As activation interruption"
        );
        match index {
            Some(index) => {
                let old = std::mem::replace(&mut self.resources.bindings[index], binding);
                if archive {
                    self.documents[document.slot]
                        .as_mut()
                        .expect("validated live owner")
                        .retained_bindings
                        .source_history
                        .push(old);
                }
            }
            None => self.resources.bindings.push(binding),
        }
        #[cfg(test)]
        assert_ne!(
            self.publication_fault_after,
            Some(1),
            "injected Save As activation interruption"
        );
        self.resources
            .source_export_reservations
            .swap_remove(reservation);
        #[cfg(test)]
        assert_ne!(
            self.publication_fault_after,
            Some(2),
            "injected Save As activation interruption"
        );
        let state = self.resources.tasks[plan.id.slot]
            .as_mut()
            .expect("validated task")
            .source_export
            .as_mut()
            .expect("validated Save As");
        state.released = true;
        state.binding_applied = true;
        #[cfg(test)]
        assert_ne!(
            self.publication_fault_after,
            Some(3),
            "injected Save As activation interruption"
        );
        self.generation = publication;
        #[cfg(test)]
        assert_ne!(
            self.publication_fault_after,
            Some(4),
            "injected Save As activation interruption"
        );
        if let (Some(id), Some(terminal)) = (invocation, terminal) {
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(5),
                "injected Save As terminal interruption"
            );
            self.install_invocation_terminal(id, terminal);
        }
        self.faulted = false;
        Ok(true)
    }
}
