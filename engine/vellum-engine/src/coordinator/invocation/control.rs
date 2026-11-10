//! Metadata-only protected intent; execution/outcome settlement remains separate.
use super::*;
pub(in crate::coordinator) mod arguments;
pub use arguments::*;
#[cfg(test)]
pub(in crate::coordinator) mod tests;
#[cfg(test)]
mod timing_tests;
pub struct TrustedInvocationControlV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub owner_control_allowed: Option<bool>,
    pub control_context_current: Option<bool>,
    pub control_scope_valid: Option<bool>,
    pub protected_control_ready: Option<bool>,
}
impl BoundIntent {
    pub(super) fn control_context(&self) -> Option<(&SettingsExecutionContextV1, &Arc<()>)> {
        match self {
            Self::Scratch(c) => Some((&c.context, &c.owner)),
            Self::LiveRelation(c) => Some((&c.context, &c.owner)),
            Self::RetainedRelease(c) => Some((&c.context, &c.owner)),
            Self::SourceRebind(c) => Some((&c.context, &c.owner)),
            Self::SourceLoad(c) => Some((&c.context, &c.owner)),
            Self::SourceSaveAs(c) => Some((&c.context, &c.owner)),
            Self::AnnotationLoad(c) => Some((&c.context, &c.owner)),
            Self::RetainedDisposal(c) => Some((&c.context, &c.owner)),
            Self::Annotation(c) => Some((&c.context, &c.owner)),
            Self::NativeSettings(c) => Some((&c.context, &c.owner)),
            Self::ResourceCatalog(c) => Some((&c.context, &c.owner)),
            Self::ManagedSearch(c) => Some((&c.scope.context, &c.owner)),
            Self::ReturnedSearchReplacement(c) => {
                Some((&c.consumer.scope.context, &c.consumer.owner))
            }
            Self::ScopedSettings(c) => Some((&c.context, &c.owner)),
            Self::ScopedDefinition(c) => Some((&c.context, &c.owner)),
            Self::ScopedLifecycle(c) => Some((&c.context, &c.owner)),
            Self::Serialization(c) => Some((&c.context, &c.owner)),
            Self::Composition(c) => Some((&c.actor, &c.owner)),
            Self::ExactBatch(c) => Some((&c.actor, &c.owner)),
            Self::AtomicAuthoring(c) => Some(c.control_context()),
            Self::RetiredRelease
            | Self::RetiredSearchReplacement
            | Self::RetiredAtomicAuthoring
            | Self::RetiredRebind
            | Self::RetiredLoad
            | Self::RetiredSaveAs
            | Self::RetiredAnnotationLoad
            | Self::RetiredScratch
            | Self::RetiredLiveRelation
            | Self::RetiredDisposal
            | Self::RetiredAnnotation
            | Self::SettingsLayers(_)
            | Self::SettingsDefinition(_)
            | Self::DependentSettings(_)
            | Self::ConfiguredLine(_)
            | Self::Settings(_)
            | Self::Relation { .. }
            | Self::Source(_)
            | Self::ScopedSource { .. }
            | Self::ScopedCompound { .. }
            | Self::DependentCompound { .. }
            | Self::Compound(_)
            | Self::LiteralEdit(_)
            | Self::LineEdit(_)
            | Self::Unicode(_)
            | Self::Selection { .. }
            | Self::SearchReplacement { .. }
            | Self::Reversal { .. }
            | Self::DependentReversal(_) => None,
        }
    }
}
impl Slot {
    pub(super) fn control_context(&self) -> Option<(&SettingsExecutionContextV1, &Arc<()>)> {
        self.compound_results
            .as_ref()
            .map(compound_results::Hold::control_context)
            .or_else(|| self.intent.as_ref().and_then(BoundIntent::control_context))
    }
}
fn pending_intact(
    slot: &Slot,
    owner: &Arc<()>,
    id: &InvocationId,
    ledger: &retention::RetentionState,
) -> bool {
    match (&slot.intent, &slot.phase) {
        (
            Some(BoundIntent::AtomicAuthoring(c)),
            Phase::Pending {
                prepared: Ok(p), ..
            },
        ) => c.matches_plan(owner, id, p, slot.compound_results.as_ref(), ledger),
        (
            Some(BoundIntent::ExactBatch(c)),
            Phase::Pending {
                prepared: Ok(p), ..
            },
        ) => Arc::ptr_eq(&c.owner, owner) && c.matches_plan(p),
        (
            Some(BoundIntent::Composition(c)),
            Phase::Pending {
                prepared: Ok(p), ..
            },
        ) => {
            Arc::ptr_eq(&c.owner, owner)
                && SourceCoordinator::composition_intent_matches(p, &c.view, &c.replacement)
        }
        (Some(BoundIntent::Scratch(c)), Phase::ScratchPending(p)) => {
            Arc::ptr_eq(c, p) && c.intact(owner, id, &slot.scratch_hold, ledger)
        }
        (Some(BoundIntent::LiveRelation(c)), Phase::LiveRelationPending(p)) => {
            Arc::ptr_eq(c, p) && c.intact(owner, id, &slot.live_relation_hold, ledger)
        }
        (Some(BoundIntent::Compound(intent)), Phase::Pending { prepared, .. }) => slot
            .compound_results
            .as_ref()
            .is_some_and(|h| h.pending_intact(owner, id, ledger, intent, prepared.as_ref())),
        (Some(BoundIntent::RetainedRelease(c)), Phase::RetainedReleasePending(p)) => {
            Arc::ptr_eq(c, p) && c.intact(owner, id, &slot.release_hold, ledger)
        }
        (Some(BoundIntent::SourceRebind(c)), Phase::SourceRebindPending(p)) => {
            Arc::ptr_eq(c, p) && c.intact(owner, id, &slot.rebind_hold, ledger)
        }
        (Some(BoundIntent::SourceLoad(c)), Phase::SourceLoadPending(p)) => {
            Arc::ptr_eq(c, p) && c.intact(owner, id, &slot.load_hold, ledger)
        }
        (Some(BoundIntent::SourceSaveAs(c)), Phase::SourceSaveAsPending(p)) => {
            Arc::ptr_eq(c, p) && c.intact(owner, id, &slot.saveas_hold, ledger)
        }
        (Some(BoundIntent::AnnotationLoad(c)), Phase::AnnotationLoadPending(p)) => {
            Arc::ptr_eq(c, p)
                && c.intact(owner, id, &slot.annotation_load_hold, ledger)
                && slot
                    .annotation_load_hold
                    .as_ref()
                    .is_some_and(|h| h.pending.is_some())
        }
        (Some(BoundIntent::RetainedDisposal(c)), Phase::RetainedDisposalPending(p)) => {
            Arc::ptr_eq(c, p) && c.intact(owner, id, &slot.disposal_hold, ledger)
        }
        (Some(BoundIntent::Annotation(c)), Phase::AnnotationPending(p)) => {
            p.intact(c, owner, id, &slot.annotation_hold, ledger)
        }
        (Some(BoundIntent::NativeSettings(c)), Phase::NativeSettingsPending(p)) => {
            p.intact(c, owner)
        }
        (Some(BoundIntent::ResourceCatalog(c)), Phase::ResourceCatalogPending(p)) => {
            Arc::ptr_eq(c, p)
                && Arc::ptr_eq(&c.owner, owner)
                && c.invocation == *id
                && c.scoped_shape_intact()
        }
        (Some(BoundIntent::ManagedSearch(c)), Phase::ManagedSearchPending(p)) => {
            Arc::ptr_eq(c, &p.capture) && Arc::ptr_eq(&c.owner, owner) && c.intact(&p.plan)
        }
        (Some(BoundIntent::ReturnedSearchReplacement(c)), Phase::ManagedSearchPending(p)) => {
            c.intact(owner, id, ledger)
                && Arc::ptr_eq(&c.consumer, &p.capture)
                && c.consumer.intact(&p.plan)
        }
        (Some(BoundIntent::ScopedSettings(c)), Phase::ScopedSettingsPending(p)) => {
            Arc::ptr_eq(c, &p.capture) && Arc::ptr_eq(&c.owner, owner)
        }
        (Some(BoundIntent::ScopedDefinition(c)), Phase::ScopedDefinitionPending(p)) => {
            Arc::ptr_eq(c, &p.capture) && Arc::ptr_eq(&c.owner, owner)
        }
        (Some(BoundIntent::ScopedLifecycle(c)), Phase::ScopedLifecyclePending(p)) => {
            Arc::ptr_eq(c, &p.capture) && Arc::ptr_eq(&c.owner, owner)
        }
        (Some(BoundIntent::Serialization(c)), Phase::SerializationPending(p)) => {
            Arc::ptr_eq(c, &p.capture) && Arc::ptr_eq(&c.owner, owner)
        }
        _ => false,
    }
}
impl SourceCoordinator {
    /// No content/right-to-edit grant, execution, settlement, allocation or refund.
    /// Context-free trusted-owner jobs are unsupported by this scoped port.
    pub fn request_invocation_stop(
        &mut self,
        id: &InvocationId,
        f: &TrustedInvocationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if [
                f.owner_control_allowed,
                f.control_context_current,
                f.control_scope_valid,
                f.protected_control_ready,
            ]
            .into_iter()
            .any(|v| v != Some(true))
            {
                return Err(InvocationError::Unavailable);
            }
            let slot = self.invocation_slot(id)?;
            let (context, owner) = slot.control_context().ok_or(InvocationError::Unavailable)?;
            if context != f.context || !Arc::ptr_eq(owner, &self.namespace) {
                return Err(InvocationError::Unavailable);
            }
            if matches!(slot.phase, Phase::Issued(_) | Phase::Terminal(_)) {
                return Err(InvocationError::Unavailable);
            }
            if matches!(slot.phase, Phase::Running) {
                return Err(InvocationError::Faulted);
            }
            if !pending_intact(slot, &self.namespace, id, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            self.invocations.slots[id.slot]
                .as_mut()
                .expect("validated controlled slot")
                .stop_requested = true;
            Ok(())
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(r, Boundary::InvocationStopRequest, &result, timing.finish());
        result
    }
    /// Only called by actual service after its normal healthy/context/kind gate.
    pub(super) fn service_requested_stop(
        &mut self,
        id: &InvocationId,
        r: &mut Recorder,
    ) -> Result<bool, InvocationError> {
        // This check is on every service path. Ordinary no-stop work must not
        // acquire two clocks or emit a fictitious stopped-settlement episode.
        if !self.faulted
            && self
                .invocation_slot(id)
                .is_ok_and(|slot| !slot.stop_requested || matches!(slot.phase, Phase::Terminal(_)))
        {
            return Ok(false);
        }
        let timing = r.begin_service_timing();
        let result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(id)?;
            if !slot.stop_requested || matches!(slot.phase, Phase::Terminal(_)) {
                return Ok(false);
            }
            if !pending_intact(slot, &self.namespace, id, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            self.install_invocation_terminal(id, InvocationOutcome::StoppedBeforePublication);
            Ok(true)
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        if !matches!(result, Ok(false)) {
            capture_timed(
                r,
                Boundary::InvocationStoppedSettlement,
                &result,
                timing.finish(),
            );
        }
        result
    }
}
