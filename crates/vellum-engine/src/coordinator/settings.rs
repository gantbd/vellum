//! Compiler-owned settings and bounded direct activation. Recipient projections are separate.
use super::*;
#[derive(Clone)]
pub struct SettingsRegistryId {
    owner: Arc<()>,
}
impl PartialEq for SettingsRegistryId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
    }
}
impl Eq for SettingsRegistryId {}
impl std::fmt::Debug for SettingsRegistryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SettingsRegistryId(<opaque>)")
    }
}
pub(super) mod basis;
pub(in crate::coordinator) mod reader;
pub(in crate::coordinator) fn admission_error(error: CoordinatorError) -> SettingsError {
    match error {
        CoordinatorError::Unavailable => SettingsError::Unavailable,
        _ => SettingsError::Faulted,
    }
}
mod retained;
use crate::settings::{
    CompiledSettingsV1, SettingIdentityV1, SettingReadV1, SettingsError, SettingsUsageV1,
};
pub(super) use basis::BasisProfile;
pub use basis::{
    SettingsBasisAvailabilityV1, SettingsBasisControlV1, SettingsBasisLimitsV1,
    SettingsExecutionContextV1, SettingsWriteBasisId, TrustedSettingsBasisFactsV1,
};
pub(super) use retained::RetentionPool;
pub use retained::{SettingsInterpretationId, SettingsRetentionLimitsV1, SettingsRetentionUsageV1};
#[derive(Clone)]
pub struct SettingsTargetId {
    owner: Arc<()>,
    identity: usize,
}

#[cfg(test)]
mod tests;
#[cfg(test)]
pub(in crate::coordinator) use tests::compiled as settings_test_fixture;
impl PartialEq for SettingsTargetId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.identity == other.identity
    }
}
impl Eq for SettingsTargetId {}
impl SettingsTargetId {
    pub(super) fn native_identity(&self) -> usize {
        self.identity
    }
}
impl std::fmt::Debug for SettingsTargetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SettingsTargetId(<opaque>)")
    }
}
pub use crate::settings::SettingsBasisV1;
use crate::settings::{
    MutationPlan, SettingChangeV1, SettingsActivationLimitsV1, SettingsGenerationsV1,
    SettingsUpdateV1,
};
#[derive(Debug)]
pub struct PreparedSettingsChange {
    _retention: Arc<()>,
    pub(super) owner: Arc<()>,
    pub(super) plan: MutationPlan,
}
pub(super) struct SettingsStage {
    candidate: Option<Arc<CompiledSettingsV1>>,
    generation: u64,
    pub(super) outcome: SettingsUpdateV1,
}
impl SettingsStage {
    pub(super) fn install(self, owner: &mut SourceCoordinator) {
        if let Some(candidate) = self.candidate {
            owner.settings = Some(candidate);
            owner.generation = self.generation;
        }
    }
}
impl SourceCoordinator {
    pub(super) fn prepare_settings_input_changes(
        &self,
        changes: &[crate::settings::SettingChangeInput<'_>],
        r: &mut Recorder,
    ) -> Result<PreparedSettingsChange, SettingsError> {
        let result = (|| {
            self.ensure_ordinary_admission().map_err(admission_error)?;
            let limits = self.settings_activation.ok_or(SettingsError::Unavailable)?;
            if self.settings_retention.closed() {
                return Err(SettingsError::Unavailable);
            }
            let state = self
                .settings
                .as_ref()
                .ok_or(SettingsError::Unavailable)?
                .clone();
            let plan = MutationPlan::prepare_inputs(state, changes, limits)?;
            self.check_settings_root_bytes(plan.candidate_logical_bytes, r)?;
            if plan.would_change {
                self.next_publication_generation(None)
                    .map_err(|e| match e {
                        CoordinatorError::VersionExhausted => SettingsError::VersionExhausted,
                        _ => SettingsError::Faulted,
                    })?;
            }
            Ok(PreparedSettingsChange {
                _retention: self.settings_plan_life.clone(),
                owner: self.namespace.clone(),
                plan,
            })
        })();
        crate::settings::observe(r, Boundary::SettingsChangePreparation, &result);
        result
    }
    pub fn settings_registry(&self, r: &mut Recorder) -> Result<SettingsRegistryId, SettingsError> {
        let result = (|| {
            self.ensure_operational()
                .map_err(|_| SettingsError::Faulted)?;
            self.settings.as_ref().ok_or(SettingsError::Unavailable)?;
            Ok(SettingsRegistryId {
                owner: self.namespace.clone(),
            })
        })();
        crate::settings::observe(r, Boundary::SettingsQuery, &result);
        result
    }
    pub(super) fn owned_settings_registry(
        &self,
        id: &SettingsRegistryId,
    ) -> Result<(), SettingsError> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return Err(SettingsError::Unavailable);
        }
        Ok(())
    }
    pub(super) fn owned_settings_target(
        &self,
        state: &CompiledSettingsV1,
        target: &SettingsTargetId,
    ) -> Result<usize, SettingsError> {
        if !Arc::ptr_eq(&target.owner, &self.namespace) {
            return Err(SettingsError::Unavailable);
        }
        state.native_target_index(target.identity)
    }
    pub(super) fn stage_settings_change(
        &self,
        prepared: PreparedSettingsChange,
        r: &mut Recorder,
    ) -> Result<SettingsStage, SettingsError> {
        let decision = (|| {
            // Final commit owns the integrity latch; admission fencing remains
            // independent and also applies to unchanged plans.
            self.ensure_admission_unfenced().map_err(admission_error)?;
            if !Arc::ptr_eq(&prepared.owner, &self.namespace) {
                return Err(SettingsError::Unavailable);
            }
            if !Arc::ptr_eq(&prepared._retention, &self.settings_plan_life) {
                return Err(SettingsError::Faulted);
            }
            if self.settings_activation != Some(prepared.plan.limits) {
                return Err(SettingsError::Stale);
            }
            let current = self.settings.as_ref().ok_or(SettingsError::Unavailable)?;
            let decision = prepared.plan.decide(current)?;
            self.check_settings_root_bytes(
                decision
                    .0
                    .as_ref()
                    .map_or(current.usage().logical_bytes, |c| c.usage().logical_bytes),
                r,
            )?;
            Ok(decision)
        })();
        crate::settings::observe(r, Boundary::SettingsChangeValidation, &decision);
        let (candidate, outcome) = decision?;
        let generation = if candidate.is_some() {
            self.next_publication_generation(None)
                .map_err(|e| match e {
                    CoordinatorError::VersionExhausted => SettingsError::VersionExhausted,
                    _ => SettingsError::Faulted,
                })?
        } else {
            self.generation
        };
        Ok(SettingsStage {
            candidate: candidate.map(Arc::new),
            generation,
            outcome,
        })
    }
    pub fn configure_settings_activation(
        &mut self,
        limits: SettingsActivationLimitsV1,
        r: &mut Recorder,
    ) -> Result<bool, SettingsError> {
        let result = (|| {
            self.ensure_operational()
                .map_err(|_| SettingsError::Faulted)?;
            if let Some(old) = self.settings_activation {
                return if old == limits {
                    Ok(false)
                } else {
                    Err(SettingsError::Unavailable)
                };
            }
            self.ensure_admission_unfenced().map_err(admission_error)?;
            self.settings_activation = Some(limits);
            Ok(true)
        })();
        crate::settings::observe(r, Boundary::SettingsActivationConfiguration, &result);
        result
    }
    pub fn settings_basis(&self, r: &mut Recorder) -> Result<SettingsBasisV1, SettingsError> {
        let result = self
            .ensure_operational()
            .map_err(|_| SettingsError::Faulted)
            .and_then(|()| {
                self.settings
                    .as_ref()
                    .map(|s| s.basis())
                    .ok_or(SettingsError::Unavailable)
            });
        crate::settings::observe(r, Boundary::SettingsQuery, &result);
        result
    }
    pub fn settings_generations(
        &self,
        target: &SettingsTargetId,
        r: &mut Recorder,
    ) -> Result<SettingsGenerationsV1, SettingsError> {
        let result = (|| {
            self.ensure_operational()
                .map_err(|_| SettingsError::Faulted)?;
            if !Arc::ptr_eq(&target.owner, &self.namespace) {
                return Err(SettingsError::Unavailable);
            }
            let state = self.settings.as_ref().ok_or(SettingsError::Unavailable)?;
            state.generations(self.owned_settings_target(state, target)?)
        })();
        crate::settings::observe(r, Boundary::SettingsQuery, &result);
        result
    }
    pub fn settings_layer_generation(
        &self,
        layer: &SettingIdentityV1,
        r: &mut Recorder,
    ) -> Result<u64, SettingsError> {
        let result = self
            .ensure_operational()
            .map_err(|_| SettingsError::Faulted)
            .and_then(|()| {
                self.settings
                    .as_ref()
                    .ok_or(SettingsError::Unavailable)?
                    .layer_generation(layer)
            });
        crate::settings::observe(r, Boundary::SettingsQuery, &result);
        result
    }
    pub fn prepare_settings_change(
        &self,
        changes: &[SettingChangeV1],
        r: &mut Recorder,
    ) -> Result<PreparedSettingsChange, SettingsError> {
        let result = (|| {
            self.ensure_ordinary_admission().map_err(admission_error)?;
            let limits = self.settings_activation.ok_or(SettingsError::Unavailable)?;
            if self.settings_retention.closed() {
                return Err(SettingsError::Unavailable);
            }
            let state = self
                .settings
                .as_ref()
                .ok_or(SettingsError::Unavailable)?
                .clone();
            let plan = MutationPlan::prepare(state, changes, limits)?;
            self.check_settings_root_bytes(plan.candidate_logical_bytes, r)?;
            if plan.would_change {
                self.next_publication_generation(None)
                    .map_err(|e| match e {
                        CoordinatorError::VersionExhausted => SettingsError::VersionExhausted,
                        _ => SettingsError::Faulted,
                    })?;
            }
            Ok(PreparedSettingsChange {
                _retention: self.settings_plan_life.clone(),
                owner: self.namespace.clone(),
                plan,
            })
        })();
        crate::settings::observe(r, Boundary::SettingsChangePreparation, &result);
        result
    }
    pub fn commit_settings_change(
        &mut self,
        prepared: PreparedSettingsChange,
        r: &mut Recorder,
    ) -> Result<SettingsUpdateV1, SettingsError> {
        if self.faulted {
            let result = Err(SettingsError::Faulted);
            crate::settings::observe(r, Boundary::SettingsChangePublication, &result);
            return result;
        }
        self.faulted = true;
        let result = self.stage_settings_change(prepared, r).map(|stage| {
            let outcome = stage.outcome;
            stage.install(self);
            outcome
        });
        if !matches!(result, Err(SettingsError::Faulted)) {
            self.faulted = false;
        }
        crate::settings::observe(r, Boundary::SettingsChangePublication, &result);
        result
    }
    pub fn install_initial_settings(
        &mut self,
        compiled: CompiledSettingsV1,
        r: &mut Recorder,
    ) -> Result<SettingsBasisV1, SettingsError> {
        let result = (|| {
            self.ensure_ordinary_admission().map_err(admission_error)?;
            if self.settings.is_some() {
                return Err(SettingsError::Unavailable);
            }
            if self.root_byte_ceiling.is_some() {
                self.check_settings_root_bytes(compiled.usage().logical_bytes, r)?;
            }
            let generation = self
                .next_publication_generation(None)
                .map_err(|e| match e {
                    CoordinatorError::VersionExhausted => SettingsError::VersionExhausted,
                    _ => SettingsError::Faulted,
                })?;
            let basis = compiled.basis();
            let compiled = Arc::new(compiled);
            self.settings = Some(compiled);
            self.generation = generation;
            Ok(basis)
        })();
        if matches!(result, Err(SettingsError::Faulted)) {
            self.faulted = true;
        }
        crate::settings::observe(r, Boundary::SettingsInstallation, &result);
        result
    }
    pub fn settings_usage(&self) -> Option<SettingsUsageV1> {
        self.settings.as_ref().map(|s| s.usage())
    }
    pub fn settings_target(
        &self,
        identity: &SettingIdentityV1,
        r: &mut Recorder,
    ) -> Result<SettingsTargetId, SettingsError> {
        let result = (|| {
            self.ensure_operational()
                .map_err(|_| SettingsError::Faulted)?;
            let state = self.settings.as_ref().ok_or(SettingsError::Unavailable)?;
            let index = state.target_index(identity)?;
            Ok(SettingsTargetId {
                owner: self.namespace.clone(),
                identity: state.native_target_identity(index)?,
            })
        })();
        crate::settings::observe(r, Boundary::SettingsQuery, &result);
        result
    }
    /// Privileged borrowed data, not an F07 recipient projection or interpretation lease.
    pub fn settings_read(
        &self,
        target: &SettingsTargetId,
        key: &str,
        r: &mut Recorder,
    ) -> Result<SettingReadV1<'_>, SettingsError> {
        let result = (|| {
            self.ensure_operational()
                .map_err(|_| SettingsError::Faulted)?;
            if !Arc::ptr_eq(&self.namespace, &target.owner) {
                return Err(SettingsError::Unavailable);
            }
            let state = self.settings.as_ref().ok_or(SettingsError::Unavailable)?;
            state.read_index(self.owned_settings_target(state, target)?, key)
        })();
        crate::settings::observe(r, Boundary::SettingsQuery, &result);
        result
    }
}
