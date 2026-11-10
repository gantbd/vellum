//! Local source association/provenance controls, never substrate operations.
use super::*;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy)]
pub struct TrustedSourceRebindFactsV1 {
    pub control: TrustedSourceExportControlV1,
    pub destination_allowed: Option<bool>,
    pub rebind_allowed: Option<bool>,
    pub owner_control_allowed: Option<bool>,
    pub preparation_reserved: Option<bool>,
    pub ordinary_bindings_current: Option<bool>,
    pub ordinary_identity_current: Option<bool>,
}

/// Current privileged host coverage/discharge assertions. These are not content
/// disclosure, create/overwrite or provider-launch rights.
#[derive(Clone, Copy)]
pub struct TrustedSourceBindingHistoryReleaseV1 {
    pub control: TrustedSourceExportControlV1,
    pub owner_control_allowed: Option<bool>,
    pub preparation_reserved: Option<bool>,
    pub ordinary_bindings_current: Option<bool>,
    pub ordinary_identity_current: Option<bool>,
    pub binding_history_release_allowed: Option<bool>,
    pub history_evidence_released: Option<bool>,
    pub snapshot_evidence_released: Option<bool>,
    pub read_evidence_released: Option<bool>,
    pub write_evidence_released: Option<bool>,
    pub provider_evidence_released: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RebindDecision {
    Unchanged,
    Changed {
        index: Option<usize>,
        binding_generation: u64,
        publication: u64,
    },
}
/// Short-lived native certificate. Owned jobs must construct this at service,
/// not retain a replacement root across unrelated editing.
pub(in crate::coordinator) struct PreparedSourceRebind {
    owner: Arc<()>,
    before: super::super::root_identity::NativeRootIdentity,
    document: DocumentId,
    target: SourceBindingTargetV1,
    options: SourceRebindOptionsV1,
    decision: RebindDecision,
}
pub(in crate::coordinator) fn rebind_authorized(facts: &TrustedSourceRebindFactsV1) -> bool {
    source_export::activation_current(&facts.control)
        && [
            facts.destination_allowed,
            facts.rebind_allowed,
            facts.owner_control_allowed,
            facts.preparation_reserved,
            facts.ordinary_bindings_current,
            facts.ordinary_identity_current,
        ]
        .into_iter()
        .all(|v| v == Some(true))
}
impl SourceCoordinator {
    pub fn source_rebind_pair_check(
        &self,
        document: &DocumentId,
        target: &SourceBindingTargetV1,
        r: &mut Recorder,
    ) -> Result<Option<OrdinaryPairCheck>, ResourceError> {
        self.source_destination_pair_check(document, &target.identity, &target.registration, r)
    }
    pub(super) fn validate_source_binding_history(
        &self,
        document: &DocumentId,
        max: usize,
    ) -> Result<usize, ResourceError> {
        let owner = self.document(document).map_err(owner_error)?;
        let history = &owner.retained_bindings.source_history;
        if history.len() > max {
            return Err(ResourceError::Limit);
        }
        for row in history {
            if row.document != *document
                || row.identity.domain != row.registration.domain
                || !Arc::ptr_eq(&row.registration.owner, &self.namespace)
            {
                return Err(ResourceError::Faulted);
            }
            if row.baseline.as_ref().is_some_and(|b| {
                b.epoch.is_some()
                    && (b.validator.is_none() || !row.registration.causal.causal_receipts)
            }) {
                return Err(ResourceError::Faulted);
            }
            if let Some(baseline) = &row.baseline
                && let Some(id) = &baseline.anchor
            {
                let task = self.resource_task(id).map_err(|_| ResourceError::Faulted)?;
                if !task.source_baseline_candidate()
                    || task.captured.document != *document
                    || task.identity != row.identity
                    || task.binding_generation != row.generation
                    || !Arc::ptr_eq(&task.registration, &row.registration)
                    || task.payload != baseline.bytes
                {
                    return Err(ResourceError::Faulted);
                }
                self.validate_source_capture(task)?;
            }
        }
        Ok(history.len())
    }
    pub fn rebind_source_document(
        &mut self,
        document: &DocumentId,
        target: SourceBindingTargetV1,
        options: SourceRebindOptionsV1,
        facts: &TrustedSourceRebindFactsV1,
        r: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let plan = self.prepare_source_rebind_native(document, target, options, facts, r)?;
            self.publish_source_rebind_native(plan, facts, r)
        }));
        let out = match caught {
            Ok(out) => out,
            Err(panic) => {
                self.faulted = true;
                capture_service(
                    r,
                    Boundary::SourceRebindPublication,
                    &Err::<bool, _>(ResourceError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(panic)
            }
        };
        capture_service(r, Boundary::SourceRebindPublication, &out, timing.finish());
        out
    }
    fn validate_source_rebind_native(
        &mut self,
        document: &DocumentId,
        target: &SourceBindingTargetV1,
        options: &SourceRebindOptionsV1,
        facts: &TrustedSourceRebindFactsV1,
        r: &mut Recorder,
    ) -> Result<RebindDecision, ResourceError> {
        let out = (|| {
            self.ensure_operational().map_err(owner_error)?;
            if !rebind_authorized(facts) {
                return Err(ResourceError::Unavailable);
            }
            self.document(document).map_err(owner_error)?;
            self.provider_current(&target.registration)?;
            let current = self.export_original_binding(document)?;
            if current.as_ref().map(|b| b.generation) != options.expected_binding {
                return Err(ResourceError::Stale);
            }
            let proposed =
                self.export_target_key(document, &target.identity, &target.registration)?;
            self.export_owner(document, &target.identity)?;
            self.export_pair(
                document,
                &target.identity,
                &target.registration,
                SavePurpose::SourceSaveAs,
                options.pair.as_ref(),
            )?;
            let history =
                self.validate_source_binding_history(document, options.max_binding_history)?;
            if self
                .resources
                .source_export_reservations
                .iter()
                .any(|r| r.document == *document)
            {
                return Err(ResourceError::Retained);
            }
            for identity in
                std::iter::once(&target.identity).chain(current.iter().map(|b| &b.identity))
            {
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
            if current.as_ref() == Some(&proposed) {
                self.check_root_growth(0, r).map_err(owner_error)?;
                return Ok(RebindDecision::Unchanged);
            }
            if self.resources.tasks.iter().flatten().any(|t| {
                t.captured.document == *document
                    && t.baseline_claim
                        .as_ref()
                        .is_some_and(|c| c.state == BaselineCapacityState::Pending)
            }) {
                return Err(ResourceError::Retained);
            }
            let archive = current.is_some();
            if history
                .checked_add(usize::from(archive))
                .is_none_or(|n| n > options.max_binding_history)
            {
                return Err(ResourceError::Limit);
            }
            let growth = std::mem::size_of::<Binding>()
                .checked_add(target.identity.key.0.len())
                .ok_or(ResourceError::Limit)?;
            self.check_root_growth(growth, r).map_err(owner_error)?;
            let publication = self
                .next_publication_generation(None)
                .map_err(owner_error)?;
            let index = self
                .resources
                .bindings
                .iter()
                .position(|b| b.document == *document);
            Ok(RebindDecision::Changed {
                index,
                binding_generation: proposed.generation,
                publication,
            })
        })();
        if matches!(out, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        out
    }
    pub(in crate::coordinator) fn prepare_source_rebind_native(
        &mut self,
        document: &DocumentId,
        target: SourceBindingTargetV1,
        options: SourceRebindOptionsV1,
        facts: &TrustedSourceRebindFactsV1,
        r: &mut Recorder,
    ) -> Result<PreparedSourceRebind, ResourceError> {
        let timing = r.begin_service_timing();
        let out = (|| {
            let decision =
                self.validate_source_rebind_native(document, &target, &options, facts, r)?;
            if let RebindDecision::Changed { index, .. } = decision {
                if index.is_some() {
                    self.documents[document.slot]
                        .as_mut()
                        .expect("validated live owner")
                        .retained_bindings
                        .source_history
                        .try_reserve(1)
                        .map_err(|_| ResourceError::Capacity)?;
                } else {
                    self.resources
                        .bindings
                        .try_reserve(1)
                        .map_err(|_| ResourceError::Capacity)?;
                }
            }
            let before = self
                .native_root_identity()
                .map_err(|_| ResourceError::Faulted)?;
            Ok(PreparedSourceRebind {
                owner: self.namespace.clone(),
                before,
                document: document.clone(),
                target,
                options,
                decision,
            })
        })();
        if matches!(out, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture_service(r, Boundary::SourceRebindPreparation, &out, timing.finish());
        out
    }
    pub(in crate::coordinator) fn publish_source_rebind_native(
        &mut self,
        plan: PreparedSourceRebind,
        facts: &TrustedSourceRebindFactsV1,
        r: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        self.publish_source_rebind_native_for_invocation(plan, facts, None, r)
    }
    pub(in crate::coordinator) fn publish_source_rebind_native_for_invocation(
        &mut self,
        plan: PreparedSourceRebind,
        facts: &TrustedSourceRebindFactsV1,
        invocation: Option<&InvocationId>,
        r: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if !Arc::ptr_eq(&plan.owner, &self.namespace) || !rebind_authorized(facts) {
                return Err(ResourceError::Unavailable);
            }
            self.ensure_operational().map_err(owner_error)?;
            if self
                .native_root_identity()
                .map_err(|_| ResourceError::Faulted)?
                != plan.before
            {
                return Err(ResourceError::Stale);
            }
            // Writer admission/reservations are not all semantic-root changes.
            // Revalidate the actual current ledgers, not just the root stamp.
            let fresh = self.validate_source_rebind_native(
                &plan.document,
                &plan.target,
                &plan.options,
                facts,
                r,
            )?;
            if fresh != plan.decision {
                return Err(ResourceError::Faulted);
            }
            let RebindDecision::Changed {
                index,
                binding_generation,
                publication,
            } = fresh
            else {
                if let Some(id) = invocation {
                    self.stage_source_rebind_candidate(
                        id,
                        &plan.document,
                        &plan.target,
                        &plan.options,
                        false,
                    )
                    .map_err(|_| ResourceError::Faulted)?;
                }
                return Ok(false);
            };
            if index.is_some() {
                let history = &self
                    .document(&plan.document)
                    .map_err(owner_error)?
                    .retained_bindings
                    .source_history;
                if history.len() == history.capacity() {
                    return Err(ResourceError::Faulted);
                }
            } else if self.resources.bindings.len() == self.resources.bindings.capacity() {
                return Err(ResourceError::Faulted);
            }
            if let Some(id) = invocation {
                self.stage_source_rebind_candidate(
                    id,
                    &plan.document,
                    &plan.target,
                    &plan.options,
                    true,
                )
                .map_err(|_| ResourceError::Faulted)?;
            }
            let binding = Binding {
                document: plan.document.clone(),
                identity: plan.target.identity,
                registration: plan.target.registration,
                generation: binding_generation,
                baseline: None,
            };
            self.faulted = true;
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(0),
                "injected rebind publication interruption"
            );
            // All storage is reserved and checks complete. These ownership moves
            // have no callback, I/O, suspension or fallible allocation.
            match index {
                Some(index) => {
                    let old = std::mem::replace(&mut self.resources.bindings[index], binding);
                    self.documents[plan.document.slot]
                        .as_mut()
                        .expect("validated live owner")
                        .retained_bindings
                        .source_history
                        .push(old);
                }
                None => self.resources.bindings.push(binding),
            }
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(1),
                "injected rebind publication interruption"
            );
            self.generation = publication;
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(2),
                "injected rebind publication interruption"
            );
            self.faulted = false;
            Ok(true)
        }));
        let out = match caught {
            Ok(out) => out,
            Err(panic) => {
                self.faulted = true;
                capture_service(
                    r,
                    Boundary::SourceRebindCommit,
                    &Err::<bool, _>(ResourceError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(panic)
            }
        };
        if out == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(r, Boundary::SourceRebindCommit, &out, timing.finish());
        out
    }
    /// Drops only this live owner's source association archive. `None` means an
    /// expected unbound owner, not binding generation zero. Bounds are explicit
    /// host profile envelopes, not automatic eviction or producer retirement.
    pub fn release_source_binding_history(
        &mut self,
        document: &DocumentId,
        expected_binding: Option<u64>,
        pair: Option<OrdinaryPairEvidence>,
        max_binding_history: usize,
        facts: &TrustedSourceBindingHistoryReleaseV1,
        r: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        let out = (|| {
            self.ensure_operational().map_err(owner_error)?;
            if !source_export::activation_current(&facts.control)
                || ![
                    facts.owner_control_allowed,
                    facts.preparation_reserved,
                    facts.ordinary_bindings_current,
                    facts.ordinary_identity_current,
                    facts.binding_history_release_allowed,
                    facts.history_evidence_released,
                    facts.snapshot_evidence_released,
                    facts.read_evidence_released,
                    facts.write_evidence_released,
                    facts.provider_evidence_released,
                ]
                .into_iter()
                .all(|v| v == Some(true))
            {
                return Err(ResourceError::Unavailable);
            }
            let owner = self.document(document).map_err(owner_error)?;
            let current = self.export_original_binding(document)?;
            if current.as_ref().map(|b| b.generation) != expected_binding {
                return Err(ResourceError::Stale);
            }
            self.validate_ordinary_pair(document, pair.as_ref())?;
            self.validate_source_binding_history(document, max_binding_history)?;
            let unchanged = owner.retained_bindings.source_history.is_empty();
            let removed = owner
                .retained_bindings
                .source_history_bytes()
                .map_err(|_| ResourceError::Limit)?;
            self.check_root_replacement(removed, 0, r)
                .map_err(owner_error)?;
            if unchanged {
                return Ok(false);
            }
            let publication = self
                .next_retention_publication_generation(None)
                .map_err(owner_error)?;
            // Only the known root consumer disappears. Tasks, captures, worker
            // disposition/quiescence and current baselines are untouched.
            self.documents[document.slot]
                .as_mut()
                .expect("validated live owner")
                .retained_bindings
                .source_history
                .clear();
            self.generation = publication;
            Ok(true)
        })();
        if matches!(out, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture(r, Boundary::SourceBindingHistoryRelease, &out);
        out
    }
}
