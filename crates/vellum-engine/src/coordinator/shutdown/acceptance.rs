//! Once-only accepted disposition. Acceptance starts controlled progress, not
//! whole-owner discharge, persistence, an implicit save or an elapsed-time deadline.
use super::*;
use crate::resource::{ArtifactKind, SaveId};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShutdownSaveBindingsV1 {
    pub document: DocumentId,
    pub source: Option<SaveId>,
    pub annotations: Option<SaveId>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShutdownSaveNeedV1 {
    pub document: DocumentId,
    pub source: bool,
    pub annotations: bool,
}
#[derive(Debug)]
pub enum ShutdownAcceptanceV1 {
    NeedsSave(Vec<ShutdownSaveNeedV1>),
    Accepted(ShutdownFenceIdV1),
}
struct Inputs {
    inventory: ShutdownInventoryV1,
    decisions: ShutdownDecisionsV1,
    bindings: Vec<ShutdownSaveBindingsV1>,
    maximum: usize,
}
pub(super) struct Accepted {
    inputs: Arc<Inputs>,
    reservation: retention::Reservation,
    charge: usize,
}
pub(super) enum State {
    Unrequested,
    Active(Accepted),
    Retired,
}
impl State {
    pub(in crate::coordinator::shutdown) fn active(&self) -> Option<&Accepted> {
        match self {
            Self::Active(accepted) => Some(accepted),
            Self::Unrequested | Self::Retired => None,
        }
    }
}
fn charge(inventory: &ShutdownInventoryV1, count: usize) -> Result<usize, CoordinatorError> {
    size_of::<Accepted>()
        .checked_add(size_of::<Inputs>())
        .and_then(|n| n.checked_add(inventory.capture_ceiling()))
        .and_then(|n| {
            n.checked_add(count.checked_mul(
                size_of::<ShutdownCapturedDecisionV1>() + size_of::<ShutdownSaveBindingsV1>(),
            )?)
        })
        .ok_or(CoordinatorError::Limit)
}
fn parent(e: RetentionErrorV1) -> CoordinatorError {
    match e {
        RetentionErrorV1::Faulted => CoordinatorError::Faulted,
        RetentionErrorV1::Limit => CoordinatorError::Limit,
        RetentionErrorV1::Unavailable => CoordinatorError::Unavailable,
    }
}
impl Accepted {
    pub(in crate::coordinator::shutdown) fn metadata_charge(
        &self,
        ledger: &retention::RetentionState,
    ) -> Result<usize, CoordinatorError> {
        if self.inputs.bindings.len() != self.inputs.decisions.decisions.len()
            || charge(&self.inputs.inventory, self.inputs.bindings.len())? != self.charge
        {
            return Err(CoordinatorError::Faulted);
        }
        ledger
            .validate_exact(&self.reservation, self.charge)
            .map_err(parent)?;
        Ok(self.charge)
    }
}
impl SourceCoordinator {
    pub(super) fn shutdown_cancellation_plan(&self) -> Result<(), CoordinatorError> {
        let fence = self
            .shutdown
            .fence
            .as_ref()
            .ok_or(CoordinatorError::Unavailable)?;
        match &fence.accepted {
            State::Active(plan) => plan.metadata_charge(&self.retention).map(|_| ()),
            State::Retired => Ok(()),
            State::Unrequested => Err(CoordinatorError::Unavailable),
        }
    }
    pub(super) fn shutdown_cancellation_required_save(
        &self,
        id: &SaveId,
        document: &DocumentId,
    ) -> Result<bool, CoordinatorError> {
        if !Arc::ptr_eq(&document.namespace, &self.namespace) {
            return Err(CoordinatorError::Faulted);
        }
        let fence = self
            .shutdown
            .fence
            .as_ref()
            .ok_or(CoordinatorError::Unavailable)?;
        match &fence.accepted {
            State::Active(plan) => {
                let bindings = &plan.inputs.bindings;
                Ok(bindings
                    .binary_search_by_key(&document.slot, |b| b.document.slot)
                    .ok()
                    .and_then(|i| bindings.get(i))
                    .is_some_and(|b| {
                        b.document == *document
                            && (b.source.as_ref() == Some(id) || b.annotations.as_ref() == Some(id))
                    }))
            }
            State::Retired => Ok(false),
            State::Unrequested => Err(CoordinatorError::Unavailable),
        }
    }
    /// One binding row per captured decision, in the same order. Save choices
    /// require original admitted allocations; discard choices must supply none.
    /// Missing/failed/conflicting saves return NeedsSave before installing any
    /// fence. The host can settle/replace them and recapture current consent.
    pub fn accept_shutdown_disposition_v1(
        &mut self,
        inventory: &ShutdownInventoryV1,
        decisions: &ShutdownDecisionsV1,
        bindings: &[ShutdownSaveBindingsV1],
        max_control_bytes: usize,
        f: &TrustedShutdownInventoryFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ShutdownAcceptanceV1, CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.shutdown_inventory_roles(&inventory.basis, f)?;
            if let Some(fence) = &self.shutdown.fence {
                self.validate_shutdown_owner_control(&fence.id, f.control)?;
                let accepted = fence
                    .accepted
                    .active()
                    .ok_or(CoordinatorError::Unavailable)?;
                accepted.metadata_charge(&self.retention)?;
                let original = &accepted.inputs;
                return if Arc::ptr_eq(&original.inventory.marker, &inventory.marker)
                    && Arc::ptr_eq(&original.decisions.marker, &decisions.marker)
                    && original.bindings == bindings
                    && original.maximum == max_control_bytes
                {
                    Ok(ShutdownAcceptanceV1::Accepted(fence.id.clone()))
                } else {
                    Err(CoordinatorError::Unavailable)
                };
            }
            self.validate_shutdown_decisions_current_v1(inventory, decisions, f, r)?;
            if bindings.len() != decisions.decisions.len() {
                return Err(CoordinatorError::Unavailable);
            }
            let charge = charge(inventory, bindings.len())?;
            if charge > max_control_bytes {
                return Err(CoordinatorError::Limit);
            }
            let cut = self.read_shutdown_inventory_v1(inventory, f, r)?;
            let mut needs = Vec::new();
            needs
                .try_reserve_exact(bindings.len())
                .map_err(|_| CoordinatorError::Capacity)?;
            for (choice, binding) in decisions.decisions.iter().zip(bindings) {
                if choice.expected.source.document != binding.document {
                    return Err(CoordinatorError::Unavailable);
                }
                let mut missing = [false; 2];
                for (index, (choice_kind, id, kind)) in [
                    (choice.source, binding.source.as_ref(), ArtifactKind::Source),
                    (
                        choice.annotations,
                        binding.annotations.as_ref(),
                        ArtifactKind::Annotations,
                    ),
                ]
                .into_iter()
                .enumerate()
                {
                    match (choice_kind, id) {
                        (ShutdownDocumentChoiceV1::Discard, None) => {}
                        (ShutdownDocumentChoiceV1::Discard, Some(_)) => {
                            return Err(CoordinatorError::Unavailable);
                        }
                        (ShutdownDocumentChoiceV1::Save, None) => missing[index] = true,
                        (ShutdownDocumentChoiceV1::Save, Some(id)) => {
                            if !cut.writes.iter().any(|w| {
                                w.write == *id
                                    && w.source == choice.expected.source
                                    && w.kind == kind
                            }) {
                                return Err(CoordinatorError::Unavailable);
                            }
                            missing[index] = !self
                                .shutdown_save_admissible(&choice.expected, kind, id, r)
                                .map_err(CoordinatorError::Resource)?;
                        }
                    }
                }
                if missing[0] || missing[1] {
                    needs.push(ShutdownSaveNeedV1 {
                        document: binding.document.clone(),
                        source: missing[0],
                        annotations: missing[1],
                    });
                }
            }
            if !needs.is_empty() {
                return Ok(ShutdownAcceptanceV1::NeedsSave(needs));
            }
            let mut rows = Vec::new();
            rows.try_reserve_exact(bindings.len())
                .map_err(|_| CoordinatorError::Capacity)?;
            rows.extend_from_slice(bindings);
            let mut choices = Vec::new();
            choices
                .try_reserve_exact(bindings.len())
                .map_err(|_| CoordinatorError::Capacity)?;
            choices.extend_from_slice(&decisions.decisions);
            let inputs = Arc::new(Inputs {
                inventory: inventory.retain_metadata(),
                decisions: ShutdownDecisionsV1 {
                    marker: decisions.marker.clone(),
                    original_inventory: decisions.original_inventory.clone(),
                    decisions: choices,
                },
                bindings: rows,
                maximum: max_control_bytes,
            });
            let total = charge
                .checked_add(inventory.basis.limits.required_capture_bytes()?)
                .ok_or(CoordinatorError::Limit)?;
            self.retention.check(total).map_err(parent)?;
            let mut reservation = self.retention.reserve(charge).map_err(parent)?;
            let basis = ShutdownFenceBasisV1(inventory.basis.clone());
            let fence = match self.install_shutdown_fence_v1(&basis, f.control, r) {
                Ok(fence) => fence,
                Err(error) => {
                    self.retention.release(&mut reservation).map_err(parent)?;
                    return Err(error);
                }
            };
            self.shutdown
                .fence
                .as_mut()
                .ok_or(CoordinatorError::Faulted)?
                .accepted = State::Active(Accepted {
                inputs,
                reservation,
                charge,
            });
            Ok(ShutdownAcceptanceV1::Accepted(fence))
        })();
        if matches!(
            result,
            Err(CoordinatorError::Faulted
                | CoordinatorError::Resource(crate::resource::ResourceError::Faulted))
        ) {
            self.faulted = true;
        }
        observe_service(
            r,
            Boundary::ShutdownDispositionAcceptance,
            &result,
            timer.finish(),
        );
        result
    }
    pub(super) fn validate_accepted_disposition(
        &self,
        inventory: &ShutdownInventoryV1,
        decisions: &ShutdownDecisionsV1,
        request: &ShutdownDocumentDispositionRequestV1<'_>,
    ) -> Result<(), CoordinatorError> {
        if matches!(
            self.shutdown.fence.as_ref().map(|f| &f.accepted),
            Some(State::Retired)
        ) {
            return Err(CoordinatorError::Unavailable);
        }
        if let Some(accepted) = self
            .shutdown
            .fence
            .as_ref()
            .and_then(|f| f.accepted.active())
        {
            accepted.metadata_charge(&self.retention)?;
            let input = &accepted.inputs;
            if !Arc::ptr_eq(&input.inventory.marker, &inventory.marker)
                || !Arc::ptr_eq(&input.decisions.marker, &decisions.marker)
            {
                return Err(CoordinatorError::Unavailable);
            }
            let b = input
                .bindings
                .iter()
                .find(|b| b.document == *request.document)
                .ok_or(CoordinatorError::Unavailable)?;
            if b.source.as_ref() != request.source_save
                || b.annotations.as_ref() != request.annotation_save
            {
                return Err(CoordinatorError::Unavailable);
            }
        }
        Ok(())
    }
    /// Continue from the engine-owned accepted plan even after the host drops
    /// its original inspection/decision handles. Fresh disposition rights apply.
    pub fn dispose_accepted_shutdown_document_v1(
        &mut self,
        fence: &ShutdownFenceIdV1,
        document: &DocumentId,
        f: &TrustedShutdownInventoryFactsV1<'_>,
        coverage: &TrustedDisposalCoverageV1,
        r: &mut Recorder,
    ) -> Result<LifecycleState, CoordinatorError> {
        let timer = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.validate_shutdown_owner_control(fence, f.control)?;
            let accepted = self
                .shutdown
                .fence
                .as_ref()
                .and_then(|f| f.accepted.active())
                .ok_or(CoordinatorError::Unavailable)?;
            accepted.metadata_charge(&self.retention)?;
            let input = accepted.inputs.clone();
            let binding = input
                .bindings
                .iter()
                .find(|b| b.document == *document)
                .ok_or(CoordinatorError::Unavailable)?;
            self.dispose_shutdown_document_v1(
                &input.inventory,
                &input.decisions,
                &ShutdownDocumentDispositionRequestV1 {
                    document,
                    source_save: binding.source.as_ref(),
                    annotation_save: binding.annotations.as_ref(),
                },
                f,
                coverage,
                r,
            )
        }));
        let result = match caught {
            Ok(result) => result,
            Err(panic) => {
                self.faulted = true;
                observe_service(
                    r,
                    Boundary::ShutdownAcceptedDocumentDisposition,
                    &Err::<LifecycleState, _>(CoordinatorError::Faulted),
                    timer.finish(),
                );
                std::panic::resume_unwind(panic)
            }
        };
        if matches!(
            result,
            Err(CoordinatorError::Faulted
                | CoordinatorError::Resource(crate::resource::ResourceError::Faulted))
        ) {
            self.faulted = true;
        }
        observe_service(
            r,
            Boundary::ShutdownAcceptedDocumentDisposition,
            &result,
            timer.finish(),
        );
        result
    }

    /// Release only the completed accepted plan's metadata. The admission fence,
    /// wait/report retention and external effect obligations remain independent.
    /// Retirement is absorbing: neither replay nor the raw disposition path can
    /// revive the original consent. Current owner control is still required.
    pub fn retire_shutdown_disposition_v1(
        &mut self,
        fence: &ShutdownFenceIdV1,
        facts: &TrustedShutdownFenceFactsV1<'_>,
        retention_released: Option<bool>,
        delivery_drained: Option<bool>,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.validate_shutdown_owner_control(fence, facts)?;
            let accepted = self
                .shutdown
                .fence
                .as_ref()
                .and_then(|f| f.accepted.active())
                .ok_or(CoordinatorError::Unavailable)?;
            accepted.metadata_charge(&self.retention)?;
            if retention_released != Some(true)
                || delivery_drained != Some(true)
                || Arc::strong_count(&accepted.inputs) != 1
            {
                return Err(CoordinatorError::Unavailable);
            }
            for binding in &accepted.inputs.bindings {
                let id = &binding.document;
                if !Arc::ptr_eq(&id.namespace, &self.namespace) {
                    return Err(CoordinatorError::Faulted);
                }
                // Issued slots never get reused. An absent previously issued
                // slot is final reclamation, not an unknown document or consent.
                let document = self
                    .documents
                    .get(id.slot)
                    .ok_or(CoordinatorError::Faulted)?;
                if document
                    .as_ref()
                    .is_some_and(|d| d.lifecycle.phase == LifecyclePhase::Live)
                {
                    return Err(CoordinatorError::Unavailable);
                }
            }
            let state = &mut self
                .shutdown
                .fence
                .as_mut()
                .ok_or(CoordinatorError::Faulted)?
                .accepted;
            let State::Active(accepted) = state else {
                return Err(CoordinatorError::Faulted);
            };
            self.retention
                .release(&mut accepted.reservation)
                .map_err(parent)?;
            *state = State::Retired;
            Ok(())
        })();
        if result == Err(CoordinatorError::Faulted) {
            self.faulted = true;
        }
        observe_service(
            r,
            Boundary::ShutdownDispositionRetirement,
            &result,
            timer.finish(),
        );
        result
    }
}
