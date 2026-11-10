//! Optional historical metadata, separately funded from execution and fresh authority.
use super::*;
use crate::settings::{LayerCapture, SettingIdentityV1, SettingsError};
use std::sync::Weak;
#[cfg(test)]
mod tests;

pub struct TrustedCreatedLayerResultAdmissionV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    /// Native inline records plus actual payloads, not JSON or physical heap bytes.
    pub max_result_bytes: usize,
    pub references_allowed: Option<bool>,
    pub outcome_allowed: Option<bool>,
    /// Trusted embedding attests this complete context belongs to the actual owner.
    pub context_current: Option<bool>,
}
pub struct TrustedCreatedLayerResultReleaseV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub outcome_readable: Option<bool>,
    pub created_result_readable: Option<bool>,
    pub created_result_context_current: Option<bool>,
    pub historical_layers_readable: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreatedLayerResultStatusV1 {
    Applied,
    Unchanged,
}
#[derive(Clone)]
pub struct CreatedSettingLayerReferenceV1 {
    owner: Arc<()>,
    ticket: InvocationId,
    hold: Arc<()>,
    index: usize,
}
impl PartialEq for CreatedSettingLayerReferenceV1 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
            && self.ticket == other.ticket
            && Arc::ptr_eq(&self.hold, &other.hold)
            && self.index == other.index
    }
}
impl Eq for CreatedSettingLayerReferenceV1 {}
impl std::fmt::Debug for CreatedSettingLayerReferenceV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CreatedSettingLayerReferenceV1(<opaque>)")
    }
}
#[derive(Debug)]
pub struct CreatedSettingLayerResultV1 {
    pub index: usize,
    pub reference: CreatedSettingLayerReferenceV1,
}
pub struct CreatedLayerResultV1 {
    pub invocation: InvocationId,
    pub status: CreatedLayerResultStatusV1,
    pub created: Vec<CreatedSettingLayerResultV1>,
}
impl std::fmt::Debug for CreatedLayerResultV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CreatedLayerResultV1(<redacted>)")
    }
}
pub(super) enum State {
    Unrequested,
    Held(Box<Hold>),
    Spent,
}
impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CreatedResultsState(<redacted>)")
    }
}
impl State {
    pub(super) fn shutdown_reserved_bytes(
        &self,
        id: &InvocationId,
        ledger: &retention::RetentionState,
    ) -> Result<Option<usize>, CoordinatorError> {
        match self {
            Self::Held(hold) => {
                if hold.invocation != *id {
                    return Err(CoordinatorError::Faulted);
                }
                hold.validate(ledger)
                    .map_err(|_| CoordinatorError::Faulted)?;
                Ok(Some(hold.charge))
            }
            Self::Unrequested | Self::Spent => Ok(None),
        }
    }
    pub(super) fn held(&self) -> bool {
        matches!(self, Self::Held(_))
    }
}
pub(super) fn original_reply_maximum(slot: &Slot) -> Option<usize> {
    match &slot.created_results {
        State::Held(hold) => Some(hold.reply_bytes),
        _ => None,
    }
}
pub(super) struct Hold {
    invocation: InvocationId,
    context: SettingsExecutionContextV1,
    marker: Arc<()>,
    plan: Weak<LayerCapture>,
    identities: Vec<SettingIdentityV1>,
    reservation: Option<retention::Reservation>,
    max_result_bytes: usize,
    capture_bytes: usize,
    reply_bytes: usize,
    charge: usize,
    phase: ResultPhase,
}
enum ResultPhase {
    Prepared(Vec<CreatedSettingLayerResultV1>),
    Serving,
    Published(CreatedLayerResultV1),
}
impl Hold {
    fn validate(&self, parent: &retention::RetentionState) -> Result<(), RetentionErrorV1> {
        parent.validate_exact(
            self.reservation.as_ref().ok_or(RetentionErrorV1::Faulted)?,
            self.charge,
        )
    }
    fn release(&mut self, parent: &mut retention::RetentionState) -> Result<(), RetentionErrorV1> {
        parent.release(self.reservation.as_mut().ok_or(RetentionErrorV1::Faulted)?)
    }
}
pub(super) struct CreatedStage {
    marker: Arc<()>,
    result: CreatedLayerResultV1,
}
fn error(e: SettingsError) -> InvocationError {
    match e {
        SettingsError::Limit => InvocationError::Limit,
        SettingsError::Capacity => InvocationError::Capacity,
        SettingsError::Faulted => InvocationError::Faulted,
        _ => InvocationError::Unavailable,
    }
}
fn parent(e: RetentionErrorV1) -> InvocationError {
    match e {
        RetentionErrorV1::Faulted => InvocationError::Faulted,
        RetentionErrorV1::Limit => InvocationError::Limit,
        RetentionErrorV1::Unavailable => InvocationError::Unavailable,
    }
}
fn copied(s: &str) -> Result<String, InvocationError> {
    let mut v = String::new();
    v.try_reserve_exact(s.len())
        .map_err(|_| InvocationError::Capacity)?;
    v.push_str(s);
    Ok(v)
}
fn successful(phase: &Phase) -> bool {
    matches!(
        phase,
        Phase::Terminal(Some(
            InvocationOutcome::SettingsLayersCommitted(_)
                | InvocationOutcome::ScopedLifecycleCommitted(_)
        ))
    )
}
impl SourceCoordinator {
    pub(super) fn attach_native_created_hold(
        &mut self,
        id: &InvocationId,
        plan: &Arc<LayerCapture>,
        f: &TrustedCreatedLayerResultAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let (mut hold, charge) = self.prepare_created_hold(id, plan, f)?;
            hold.reservation = Some(self.retention.reserve(charge).map_err(parent)?);
            self.invocations.slots[id.slot]
                .as_mut()
                .expect("validated native result slot")
                .created_results = State::Held(hold);
            Ok(())
        })();
        capture_timed(
            r,
            Boundary::CreatedResultsAdmission,
            &result,
            timing.finish(),
        );
        result
    }
    fn prepare_created_hold(
        &self,
        id: &InvocationId,
        plan: &Arc<LayerCapture>,
        f: &TrustedCreatedLayerResultAdmissionV1<'_>,
    ) -> Result<(Box<Hold>, usize), InvocationError> {
        if self.faulted {
            return Err(InvocationError::Faulted);
        }
        let slot = self.invocation_slot(id)?;
        if !matches!(slot.phase, Phase::Issued(_))
            || slot.intent.is_some()
            || !matches!(slot.created_results, State::Unrequested)
        {
            return Err(InvocationError::IntentMismatch);
        }
        if [f.references_allowed, f.outcome_allowed, f.context_current]
            .into_iter()
            .any(|v| v != Some(true))
        {
            return Err(InvocationError::Unavailable);
        }
        let limits = plan.captured_limits();
        let current = self.settings.as_ref().ok_or(InvocationError::Unavailable)?;
        if f.context.authority != current.settings_authority()
            || super::super::settings::basis::labels(f.context)
                .into_iter()
                .any(|s| s.is_empty() || s.len() > limits.max_identifier_bytes)
        {
            return Err(InvocationError::Unavailable);
        }
        let n = plan.created_identities().len();
        let mut capture_bytes = std::mem::size_of::<Hold>()
            .checked_add(
                n.checked_mul(std::mem::size_of::<SettingIdentityV1>())
                    .ok_or(InvocationError::Limit)?,
            )
            .ok_or(InvocationError::Limit)?;
        for label in super::super::settings::basis::labels(f.context) {
            capture_bytes = capture_bytes
                .checked_add(label.len())
                .ok_or(InvocationError::Limit)?;
        }
        for identity in plan.created_identities() {
            capture_bytes = capture_bytes
                .checked_add(identity.authority.len())
                .and_then(|v| v.checked_add(identity.id.len()))
                .ok_or(InvocationError::Limit)?;
        }
        let reply_bytes = std::mem::size_of::<CreatedLayerResultV1>()
            .checked_add(
                n.checked_mul(std::mem::size_of::<CreatedSettingLayerResultV1>())
                    .ok_or(InvocationError::Limit)?,
            )
            .ok_or(InvocationError::Limit)?;
        if capture_bytes > f.max_result_bytes || reply_bytes > f.max_result_bytes {
            return Err(InvocationError::Limit);
        }
        let charge = capture_bytes
            .checked_add(reply_bytes)
            .and_then(|v| v.checked_add(limits.max_state_bytes))
            .ok_or(InvocationError::Limit)?;
        self.retention.check(charge).map_err(parent)?;
        let context = super::super::settings::basis::copy_context(f.context).map_err(error)?;
        let mut identities = Vec::new();
        identities
            .try_reserve_exact(n)
            .map_err(|_| InvocationError::Capacity)?;
        for id in plan.created_identities() {
            identities.push(SettingIdentityV1 {
                authority: copied(&id.authority)?,
                id: copied(&id.id)?,
                incarnation: id.incarnation,
            });
        }
        let mut reply = Vec::new();
        reply
            .try_reserve_exact(n)
            .map_err(|_| InvocationError::Capacity)?;
        // A reservation is attached only after every fallible validation/allocation.
        let reservation = None;
        Ok((
            Box::new(Hold {
                invocation: id.clone(),
                context,
                marker: Arc::new(()),
                plan: Arc::downgrade(plan),
                identities,
                reservation,
                max_result_bytes: f.max_result_bytes,
                capture_bytes,
                reply_bytes,
                charge,
                phase: ResultPhase::Prepared(reply),
            }),
            charge,
        ))
    }
    pub fn submit_prepared_settings_layers_with_created_results(
        &mut self,
        id: &InvocationId,
        prepared: PreparedSettingsLayersV1,
        f: &TrustedCreatedLayerResultAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        let timing = r.begin_service_timing();
        let result = (|| {
            if !Arc::ptr_eq(&prepared.owner, &self.namespace) {
                return Err(InvocationError::Unavailable);
            }
            if prepared.retained_bytes().map_err(error)? > self.settings_job_parent_limit() {
                return Err(InvocationError::Limit);
            }
            let (plan, expected) = prepared.plan.captured_intent();
            let capture = plan.clone();
            if prepared.read_count != prepared.reads.predicates.len()
                || !prepared.reads.configuration_profile()
            {
                return Err(InvocationError::Faulted);
            }
            let bytes = (std::mem::size_of::<settings_layers::LayerIntent>()
                - std::mem::size_of::<read_dependency::ReadCapture>())
            .checked_add(capture.bytes())
            .and_then(|n| n.checked_add(prepared.reads.logical_bytes()?))
            .ok_or(InvocationError::Limit)?;
            if bytes > self.invocations.limits.max_intent_bytes {
                return Err(InvocationError::Limit);
            }
            let reads = prepared.reads.copy_owned().map_err(|e| match e {
                ReadDependencyError::Capacity => InvocationError::Capacity,
                ReadDependencyError::Limit => InvocationError::Limit,
                _ => InvocationError::MalformedIntent,
            })?;
            let intent =
                settings_layers::LayerIntent::from_prepared(capture.clone(), expected, reads);
            let (mut hold, charge) = self.prepare_created_hold(id, &capture, f)?;
            hold.reservation = Some(self.retention.reserve(charge).map_err(parent)?);
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .expect("validated created result slot");
            slot.created_results = State::Held(hold);
            self.install_invocation_admission(
                id,
                BoundIntent::SettingsLayers(intent),
                Phase::SettingsLayersPending(Ok(prepared)),
            );
            Ok(InvocationState::Pending)
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(
            r,
            Boundary::CreatedResultsAdmission,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn submit_prepared_scoped_lifecycle_with_created_results(
        &mut self,
        id: &InvocationId,
        prepared: PreparedScopedSettingsLifecycleV1,
        f: &TrustedCreatedLayerResultAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        let timing = r.begin_service_timing();
        let result = (|| {
            if !Arc::ptr_eq(&prepared.capture.owner, &self.namespace)
                || prepared.capture.context != *f.context
            {
                return Err(InvocationError::Unavailable);
            }
            if prepared.retained_bytes().map_err(error)? > self.settings_job_parent_limit() {
                return Err(InvocationError::Limit);
            }
            let plan = prepared.created_result_capture().map_err(error)?;
            let (mut hold, charge) = self.prepare_created_hold(id, plan, f)?;
            hold.reservation = Some(self.retention.reserve(charge).map_err(parent)?);
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .expect("validated scoped result slot");
            slot.created_results = State::Held(hold);
            self.install_invocation_admission(
                id,
                BoundIntent::ScopedLifecycle(prepared.capture.clone()),
                Phase::ScopedLifecyclePending(prepared),
            );
            Ok(InvocationState::Pending)
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(
            r,
            Boundary::CreatedResultsAdmission,
            &result,
            timing.finish(),
        );
        result
    }
    pub(super) fn stage_created_layer_results(
        &mut self,
        id: &InvocationId,
        plan: &Arc<LayerCapture>,
        stage: &super::super::settings_layers::LayerStage,
        r: &mut Recorder,
    ) -> Result<Option<CreatedStage>, SettingsChangeErrorV2> {
        // Unrequested results emit no boundary and must not start a service clock.
        let timing = self
            .invocations
            .slots
            .get(id.slot)
            .and_then(Option::as_ref)
            .is_some_and(|s| s.created_results.held())
            .then(|| r.begin_service_timing());
        let result = (|| {
            let slot = self
                .invocation_slot(id)
                .map_err(|_| SettingsError::Faulted)?;
            let hold = match &slot.created_results {
                State::Unrequested => return Ok(None),
                State::Spent => return Err(SettingsError::Faulted),
                State::Held(hold) => hold,
            };
            hold.validate(&self.retention)
                .map_err(|_| SettingsError::Faulted)?;
            if !Weak::ptr_eq(&hold.plan, &Arc::downgrade(plan))
                || stage.outcome.created_layers != hold.identities.len()
                || hold.capture_bytes > hold.max_result_bytes
                || hold.reply_bytes > hold.max_result_bytes
            {
                return Err(SettingsError::Faulted);
            }
            let before = self.settings.as_ref().ok_or(SettingsError::Faulted)?;
            let candidate = stage.result_root(self)?;
            for identity in &hold.identities {
                if before.layer_generation(identity) != Err(SettingsError::Unavailable)
                    || candidate.layer_generation(identity) != Ok(1)
                {
                    return Err(SettingsError::Faulted);
                }
            }
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .ok_or(SettingsError::Faulted)?;
            let State::Held(hold) = &mut slot.created_results else {
                return Err(SettingsError::Faulted);
            };
            let ResultPhase::Prepared(mut rows) =
                std::mem::replace(&mut hold.phase, ResultPhase::Serving)
            else {
                return Err(SettingsError::Faulted);
            };
            if rows.capacity() < hold.identities.len() || !rows.is_empty() {
                return Err(SettingsError::Faulted);
            }
            #[cfg(test)]
            if self.created_result_fault_after == Some(0) {
                panic!("injected result stage interruption");
            }
            for index in 0..hold.identities.len() {
                rows.push(CreatedSettingLayerResultV1 {
                    index,
                    reference: CreatedSettingLayerReferenceV1 {
                        owner: self.namespace.clone(),
                        ticket: id.clone(),
                        hold: hold.marker.clone(),
                        index,
                    },
                });
                #[cfg(test)]
                if self.created_result_fault_after == Some(index + 1) {
                    panic!("injected private reference interruption");
                }
            }
            Ok(Some(CreatedStage {
                marker: hold.marker.clone(),
                result: CreatedLayerResultV1 {
                    invocation: id.clone(),
                    status: if stage.outcome.changed {
                        CreatedLayerResultStatusV1::Applied
                    } else {
                        CreatedLayerResultStatusV1::Unchanged
                    },
                    created: rows,
                },
            }))
        })()
        .map_err(SettingsChangeErrorV2::Settings);
        if self.invocations.slots[id.slot]
            .as_ref()
            .is_some_and(|s| s.created_results.held())
        {
            super::super::settings_reads::observe_service(
                r,
                Boundary::CreatedResultsStaging,
                &result,
                timing.and_then(|t| t.finish()),
            );
        }
        result
    }
    pub(super) fn publish_created_layer_results(
        &mut self,
        id: &InvocationId,
        stage: Option<CreatedStage>,
        r: &mut Recorder,
    ) {
        let Some(stage) = stage else {
            return;
        };
        let timing = r.begin_service_timing();
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("owned terminal result slot");
        let State::Held(hold) = &mut slot.created_results else {
            panic!("lost result hold");
        };
        assert!(
            Arc::ptr_eq(&hold.marker, &stage.marker)
                && matches!(hold.phase, ResultPhase::Serving)
                && successful(&slot.phase),
            "result publication contradiction"
        );
        hold.phase = ResultPhase::Published(stage.result);
        capture_timed(
            r,
            Boundary::CreatedResultsPublication,
            &Ok::<_, InvocationError>(()),
            timing.finish(),
        );
    }
    pub fn created_layer_results<'a>(
        &'a self,
        id: &InvocationId,
        f: &TrustedCreatedLayerResultReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<&'a CreatedLayerResultV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(id)?;
            let State::Held(hold) = &slot.created_results else {
                return Err(InvocationError::OutcomeUnavailable);
            };
            if hold.context != *f.context
                || [
                    f.outcome_readable,
                    f.created_result_readable,
                    f.created_result_context_current,
                    f.historical_layers_readable,
                ]
                .into_iter()
                .any(|v| v != Some(true))
                || !successful(&slot.phase)
            {
                return Err(InvocationError::OutcomeUnavailable);
            }
            hold.validate(&self.retention).map_err(parent)?;
            let ResultPhase::Published(result) = &hold.phase else {
                return Err(InvocationError::OutcomeUnavailable);
            };
            Ok(result)
        })();
        capture_timed(r, Boundary::CreatedResultsRelease, &result, timing.finish());
        result
    }
    pub fn forget_created_layer_results(
        &mut self,
        id: &InvocationId,
        f: &SettingsBasisControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<bool, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if [
                f.owner_control_allowed,
                f.retention_released,
                f.delivery_drained,
            ]
            .into_iter()
            .any(|v| v != Some(true))
            {
                return Err(InvocationError::Unavailable);
            }
            let slot = self.invocation_slot(id)?;
            if !matches!(slot.phase, Phase::Terminal(_) | Phase::Running) {
                return Err(InvocationError::Pending);
            }
            let State::Held(hold) = &slot.created_results else {
                return Ok(false);
            };
            if hold.context != *f.context {
                return Err(InvocationError::Unavailable);
            }
            hold.validate(&self.retention).map_err(parent)?;
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .expect("validated result cleanup slot");
            let State::Held(hold) = &mut slot.created_results else {
                unreachable!();
            };
            hold.release(&mut self.retention).map_err(parent)?;
            slot.created_results = State::Spent;
            Ok(true)
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(r, Boundary::CreatedResultsForget, &result, timing.finish());
        result
    }
    pub fn capture_settings_write_basis_from_created(
        &mut self,
        reference: &CreatedSettingLayerReferenceV1,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsBasisFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SettingsWriteBasisId, SettingsError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if self.faulted {
                return Err(SettingsError::Faulted);
            }
            if !Arc::ptr_eq(&reference.owner, &self.namespace) {
                return Err(SettingsError::Unavailable);
            }
            let slot = self
                .invocation_slot(&reference.ticket)
                .map_err(|_| SettingsError::Unavailable)?;
            let State::Held(hold) = &slot.created_results else {
                return Err(SettingsError::Unavailable);
            };
            if !successful(&slot.phase)
                || !matches!(hold.phase, ResultPhase::Published(_))
                || !Arc::ptr_eq(&hold.marker, &reference.hold)
                || hold.context != *c
            {
                return Err(SettingsError::Unavailable);
            }
            hold.validate(&self.retention)
                .map_err(|_| SettingsError::Faulted)?;
            let id = hold
                .identities
                .get(reference.index)
                .ok_or(SettingsError::Unavailable)?;
            let id = SettingIdentityV1 {
                authority: copied(&id.authority).map_err(|_| SettingsError::Capacity)?,
                id: copied(&id.id).map_err(|_| SettingsError::Capacity)?,
                incarnation: id.incarnation,
            };
            self.capture_settings_write_basis(&id, c, f, r)
        })();
        self.retained_settings_result_timed(
            r,
            Boundary::CreatedResultsBasisCapture,
            result,
            timing.finish(),
        )
    }
}
