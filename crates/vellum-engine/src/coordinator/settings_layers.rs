//! Privileged authored-layer registry changes; no recipient grants are inferred.
use super::*;
use crate::settings::{
    LayerCapture, LayerPlan, SettingsError, SettingsLayerBasisV1, SettingsLayerChangeV1,
    SettingsLayerLimitsV1, SettingsLayerOutcomeV1,
};
use settings_reads::observe;
type Error = SettingsChangeErrorV2;
#[derive(Debug)]
pub struct PreparedSettingsLayersV1 {
    pub(super) owner: Arc<()>,
    retention: Arc<()>,
    pub(super) plan: LayerPlan,
    pub(super) reads: read_dependency::ReadCapture,
    pub(super) read_count: usize,
}
impl PreparedSettingsLayersV1 {
    pub(super) fn retained_bytes(&self) -> Result<usize, SettingsError> {
        let header = std::mem::size_of::<Self>()
            - std::mem::size_of::<LayerPlan>()
            - std::mem::size_of::<read_dependency::ReadCapture>();
        self.plan
            .retained_bytes()?
            .checked_add(self.reads.logical_bytes().ok_or(SettingsError::Limit)?)
            .and_then(|n| n.checked_add(header))
            .ok_or(SettingsError::Limit)
    }
}
pub(super) struct LayerStage {
    candidate: Option<Arc<crate::settings::CompiledSettingsV1>>,
    generation: u64,
    pub(super) outcome: SettingsLayerOutcomeV1,
}
impl LayerStage {
    pub(super) fn result_root<'a>(
        &'a self,
        owner: &'a SourceCoordinator,
    ) -> Result<&'a Arc<crate::settings::CompiledSettingsV1>, SettingsError> {
        self.candidate
            .as_ref()
            .or(owner.settings.as_ref())
            .ok_or(SettingsError::Faulted)
    }
    pub(super) fn install(self, owner: &mut SourceCoordinator) {
        if let Some(candidate) = self.candidate {
            owner.settings = Some(candidate);
            owner.generation = self.generation;
        }
    }
}
fn publication(e: CoordinatorError) -> Error {
    Error::Settings(match e {
        CoordinatorError::VersionExhausted => SettingsError::VersionExhausted,
        _ => SettingsError::Faulted,
    })
}
impl SourceCoordinator {
    pub fn configure_settings_layers(
        &mut self,
        limits: SettingsLayerLimitsV1,
        r: &mut Recorder,
    ) -> Result<bool, SettingsError> {
        let result = (|| {
            self.ensure_operational()
                .map_err(|_| SettingsError::Faulted)?;
            if let Some(old) = self.settings_layer_limits {
                return if old == limits {
                    Ok(false)
                } else {
                    Err(SettingsError::Unavailable)
                };
            }
            self.ensure_admission_unfenced()
                .map_err(settings::admission_error)?;
            self.settings_layer_limits = Some(limits);
            Ok(true)
        })();
        crate::settings::observe(r, Boundary::SettingsLayerConfiguration, &result);
        result
    }
    pub fn settings_layer_basis(
        &self,
        r: &mut Recorder,
    ) -> Result<SettingsLayerBasisV1, SettingsError> {
        let result = self
            .ensure_operational()
            .map_err(|_| SettingsError::Faulted)
            .and_then(|()| {
                self.settings
                    .as_ref()
                    .map(|s| s.layer_basis())
                    .ok_or(SettingsError::Unavailable)
            });
        crate::settings::observe(r, Boundary::SettingsQuery, &result);
        result
    }
    pub fn prepare_settings_layers(
        &self,
        expected: SettingsLayerBasisV1,
        input: &SettingsLayerChangeV1,
        request: &ReadDependencyRequestV3,
        r: &mut Recorder,
    ) -> Result<PreparedSettingsLayersV1, Error> {
        let result = (|| {
            self.ensure_ordinary_admission().map_err(Error::Reads)?;
            let limits = self
                .settings_layer_limits
                .ok_or(Error::Settings(SettingsError::Unavailable))?;
            let reads = read_dependency::ReadInput::V3(request);
            let size = reads
                .retained_bytes(&[])
                .ok_or(Error::Settings(SettingsError::Limit))?;
            let ceiling = limits
                .max_plan_bytes
                .checked_sub(size)
                .ok_or(Error::Settings(SettingsError::Limit))?;
            let current = self
                .settings
                .as_ref()
                .ok_or(Error::Settings(SettingsError::Unavailable))?;
            let capture = Arc::new(
                LayerCapture::copy(input, current, limits, ceiling).map_err(Error::Settings)?,
            );
            let reads = reads.copy(&[]).map_err(|e| Error::Reads(e.into()))?;
            self.prepare_layer_capture(expected, capture, reads, r)
        })();
        observe(r, Boundary::SettingsLayerPreparation, &result);
        result
    }
    pub(super) fn prepare_layer_capture(
        &self,
        expected: SettingsLayerBasisV1,
        capture: Arc<LayerCapture>,
        reads: read_dependency::ReadCapture,
        r: &mut Recorder,
    ) -> Result<PreparedSettingsLayersV1, Error> {
        self.ensure_ordinary_admission().map_err(Error::Reads)?;
        if self.settings_retention.closed() {
            return Err(Error::Settings(SettingsError::Unavailable));
        }
        if self.settings_layer_limits != Some(capture.limits) || !reads.configuration_profile() {
            return Err(Error::Settings(SettingsError::Faulted));
        }
        self.check_owned_read_capture(&reads, r)
            .map_err(Error::Reads)?;
        let current = self
            .settings
            .as_ref()
            .ok_or(Error::Settings(SettingsError::Unavailable))?
            .clone();
        let plan = LayerPlan::prepare(current, capture, expected).map_err(Error::Settings)?;
        self.check_settings_root_bytes(plan.candidate_logical_bytes, r)
            .map_err(Error::Settings)?;
        let prepared = PreparedSettingsLayersV1 {
            owner: self.namespace.clone(),
            retention: self.settings_plan_life.clone(),
            read_count: reads.predicates.len(),
            reads,
            plan,
        };
        if prepared.retained_bytes().map_err(Error::Settings)? > prepared.plan.limits.max_plan_bytes
        {
            return Err(Error::Settings(SettingsError::Limit));
        }
        if prepared.plan.would_change {
            self.next_publication_generation(None)
                .map_err(publication)?;
        }
        Ok(prepared)
    }
    pub(super) fn stage_settings_layers(
        &self,
        prepared: PreparedSettingsLayersV1,
        r: &mut Recorder,
    ) -> Result<LayerStage, Error> {
        let result = (|| {
            self.ensure_admission_unfenced().map_err(Error::Reads)?;
            if !Arc::ptr_eq(&prepared.owner, &self.namespace) {
                return Err(Error::Settings(SettingsError::Unavailable));
            }
            if !Arc::ptr_eq(&prepared.retention, &self.settings_plan_life)
                || prepared.read_count != prepared.reads.predicates.len()
                || !prepared.reads.configuration_profile()
            {
                return Err(Error::Settings(SettingsError::Faulted));
            }
            if self.settings_layer_limits != Some(prepared.plan.limits) {
                return Err(Error::Settings(SettingsError::Stale));
            }
            let read = self.validate_read_capture(&prepared.reads);
            super::observe(r, Boundary::ReadDependencyValidation, &read);
            read.map_err(Error::Reads)?;
            if prepared.retained_bytes().map_err(Error::Settings)?
                > prepared.plan.limits.max_plan_bytes
            {
                return Err(Error::Settings(SettingsError::Limit));
            }
            let current = self
                .settings
                .as_ref()
                .ok_or(Error::Settings(SettingsError::Unavailable))?;
            let (candidate, outcome) = prepared.plan.decide(current).map_err(Error::Settings)?;
            self.check_settings_root_bytes(
                candidate
                    .as_ref()
                    .map_or(current.usage().logical_bytes, |c| c.usage().logical_bytes),
                r,
            )
            .map_err(Error::Settings)?;
            let generation = if candidate.is_some() {
                self.next_publication_generation(None)
                    .map_err(publication)?
            } else {
                self.generation
            };
            Ok(LayerStage {
                candidate: candidate.map(Arc::new),
                outcome,
                generation,
            })
        })();
        observe(r, Boundary::SettingsLayerValidation, &result);
        result
    }
    pub fn commit_settings_layers(
        &mut self,
        prepared: PreparedSettingsLayersV1,
        r: &mut Recorder,
    ) -> Result<SettingsLayerOutcomeV1, Error> {
        if self.faulted {
            let result = Err(Error::Settings(SettingsError::Faulted));
            observe(r, Boundary::SettingsLayerPublication, &result);
            return result;
        }
        self.faulted = true;
        let result = self.stage_settings_layers(prepared, r).map(|stage| {
            let outcome = stage.outcome;
            stage.install(self);
            outcome
        });
        if !result.as_ref().err().is_some_and(|e| e.faulted()) {
            self.faulted = false;
        }
        observe(r, Boundary::SettingsLayerPublication, &result);
        result
    }
}
