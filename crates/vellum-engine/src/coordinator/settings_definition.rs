//! Privileged declaration replacement, preserving authored layers and root ownership.
use super::*;
use crate::settings::{
    DefinitionCapture, DefinitionPlan, SettingsDeclarationV1, SettingsDefinitionLimitsV1,
    SettingsDefinitionOutcomeV1, SettingsError, SettingsInterpretationEpochsV1,
};
use settings_reads::observe;
type Error = SettingsChangeErrorV2;
#[derive(Debug)]
pub struct PreparedSettingsDefinitionV1 {
    pub(super) owner: Arc<()>,
    retention: Arc<()>,
    pub(super) plan: DefinitionPlan,
    pub(super) reads: read_dependency::ReadCapture,
    pub(super) read_count: usize,
}
impl PreparedSettingsDefinitionV1 {
    #[cfg(test)]
    pub(in crate::coordinator) fn retained_bytes_for_test(&self) -> usize {
        self.retained_bytes().unwrap()
    }
    pub(super) fn retained_bytes(&self) -> Result<usize, SettingsError> {
        let header = std::mem::size_of::<Self>()
            - std::mem::size_of::<DefinitionPlan>()
            - std::mem::size_of::<read_dependency::ReadCapture>();
        self.plan
            .retained_bytes()?
            .checked_add(self.reads.logical_bytes().ok_or(SettingsError::Limit)?)
            .and_then(|n| n.checked_add(header))
            .ok_or(SettingsError::Limit)
    }
}
pub(super) struct DefinitionStage {
    candidate: Option<Arc<crate::settings::CompiledSettingsV1>>,
    generation: u64,
    pub(super) outcome: SettingsDefinitionOutcomeV1,
}
impl DefinitionStage {
    pub(super) fn install(self, owner: &mut SourceCoordinator) {
        if let Some(candidate) = self.candidate {
            owner.settings = Some(candidate);
            owner.generation = self.generation;
        }
    }
}
fn publication(error: CoordinatorError) -> Error {
    Error::Settings(match error {
        CoordinatorError::VersionExhausted => SettingsError::VersionExhausted,
        _ => SettingsError::Faulted,
    })
}
impl SourceCoordinator {
    pub fn configure_settings_definitions(
        &mut self,
        limits: SettingsDefinitionLimitsV1,
        r: &mut Recorder,
    ) -> Result<bool, SettingsError> {
        let result = (|| {
            self.ensure_operational()
                .map_err(|_| SettingsError::Faulted)?;
            if let Some(old) = self.settings_definition_limits {
                return if old == limits {
                    Ok(false)
                } else {
                    Err(SettingsError::Unavailable)
                };
            }
            self.ensure_admission_unfenced()
                .map_err(settings::admission_error)?;
            self.settings_definition_limits = Some(limits);
            Ok(true)
        })();
        crate::settings::observe(r, Boundary::SettingsDefinitionConfiguration, &result);
        result
    }
    pub fn prepare_settings_definition(
        &self,
        expected: SettingsInterpretationEpochsV1,
        declaration: &SettingsDeclarationV1,
        reads: &ReadDependencyRequestV3,
        r: &mut Recorder,
    ) -> Result<PreparedSettingsDefinitionV1, Error> {
        let result = (|| {
            self.ensure_ordinary_admission().map_err(Error::Reads)?;
            let limits = self
                .settings_definition_limits
                .ok_or(Error::Settings(SettingsError::Unavailable))?;
            let input = read_dependency::ReadInput::V3(reads);
            let bytes = input
                .retained_bytes(&[])
                .ok_or(Error::Settings(SettingsError::Limit))?;
            let ceiling = limits
                .max_plan_bytes
                .checked_sub(bytes)
                .ok_or(Error::Settings(SettingsError::Limit))?;
            let current = self
                .settings
                .as_ref()
                .ok_or(Error::Settings(SettingsError::Unavailable))?;
            let capture = Arc::new(
                DefinitionCapture::from_state(declaration, current, limits, ceiling)
                    .map_err(Error::Settings)?,
            );
            let reads = input.copy(&[]).map_err(|e| Error::Reads(e.into()))?;
            self.prepare_definition_capture(expected, capture, reads, r)
        })();
        observe(r, Boundary::SettingsDefinitionPreparation, &result);
        result
    }
    pub(super) fn prepare_definition_capture(
        &self,
        expected: SettingsInterpretationEpochsV1,
        capture: Arc<DefinitionCapture>,
        reads: read_dependency::ReadCapture,
        r: &mut Recorder,
    ) -> Result<PreparedSettingsDefinitionV1, Error> {
        self.ensure_ordinary_admission().map_err(Error::Reads)?;
        if self.settings_retention.closed() {
            return Err(Error::Settings(SettingsError::Unavailable));
        }
        if self.settings_definition_limits != Some(capture.limits) || !reads.configuration_profile()
        {
            return Err(Error::Settings(SettingsError::Faulted));
        }
        self.check_owned_read_capture(&reads, r)
            .map_err(Error::Reads)?;
        let current = self
            .settings
            .as_ref()
            .ok_or(Error::Settings(SettingsError::Unavailable))?
            .clone();
        let plan = DefinitionPlan::prepare(current, capture, expected).map_err(Error::Settings)?;
        self.check_settings_root_bytes(plan.candidate_logical_bytes, r)
            .map_err(Error::Settings)?;
        let prepared = PreparedSettingsDefinitionV1 {
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
    pub(super) fn stage_settings_definition(
        &self,
        prepared: PreparedSettingsDefinitionV1,
        r: &mut Recorder,
    ) -> Result<DefinitionStage, Error> {
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
            if self.settings_definition_limits != Some(prepared.plan.limits) {
                return Err(Error::Settings(SettingsError::Stale));
            }
            let check = self.validate_read_capture(&prepared.reads);
            super::observe(r, Boundary::ReadDependencyValidation, &check);
            check.map_err(Error::Reads)?;
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
            Ok(DefinitionStage {
                candidate: candidate.map(Arc::new),
                outcome,
                generation,
            })
        })();
        observe(r, Boundary::SettingsDefinitionValidation, &result);
        result
    }
    pub fn commit_settings_definition(
        &mut self,
        prepared: PreparedSettingsDefinitionV1,
        r: &mut Recorder,
    ) -> Result<SettingsDefinitionOutcomeV1, Error> {
        if self.faulted {
            let result = Err(Error::Settings(SettingsError::Faulted));
            observe(r, Boundary::SettingsDefinitionPublication, &result);
            return result;
        }
        self.faulted = true;
        let result = self.stage_settings_definition(prepared, r).map(|stage| {
            let outcome = stage.outcome;
            stage.install(self);
            outcome
        });
        if !result.as_ref().err().is_some_and(|e| e.faulted()) {
            self.faulted = false;
        }
        observe(r, Boundary::SettingsDefinitionPublication, &result);
        result
    }
}
