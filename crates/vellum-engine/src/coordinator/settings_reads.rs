//! Additional settings dependencies conjunct with, never replace, kernel captures.
use super::*;
use crate::settings::{SettingChangeV1, SettingsError, SettingsUpdateV1};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsChangeErrorV2 {
    Settings(SettingsError),
    Reads(CoordinatorError),
}
impl SettingsChangeErrorV2 {
    pub(super) fn faulted(self) -> bool {
        matches!(
            self,
            Self::Settings(SettingsError::Faulted)
                | Self::Reads(
                    CoordinatorError::Faulted
                        | CoordinatorError::History(HistoryError::Faulted)
                        | CoordinatorError::Annotation(AnnotationError::Faulted)
                )
        )
    }
}
#[derive(Debug)]
pub struct PreparedSettingsChangeV2 {
    pub(super) settings: PreparedSettingsChange,
    pub(super) reads: read_dependency::ReadCapture,
    pub(super) reads_required: bool,
    pub(super) read_count: usize,
}
impl PreparedSettingsChangeV2 {
    pub(super) fn retained_bytes(&self) -> Result<usize, SettingsError> {
        let header = std::mem::size_of::<Self>()
            - std::mem::size_of::<crate::settings::MutationPlan>()
            - std::mem::size_of::<read_dependency::ReadCapture>();
        self.settings
            .plan
            .retained_bytes()?
            .checked_add(self.reads.logical_bytes().ok_or(SettingsError::Limit)?)
            .and_then(|n| n.checked_add(header))
            .ok_or(SettingsError::Limit)
    }
}
pub(super) fn observe<T>(
    r: &mut Recorder,
    boundary: Boundary,
    result: &Result<T, SettingsChangeErrorV2>,
) {
    observe_service(r, boundary, result, None);
}
pub(super) fn observe_service<T>(
    r: &mut Recorder,
    boundary: Boundary,
    result: &Result<T, SettingsChangeErrorV2>,
    service_time: Option<std::time::Duration>,
) {
    r.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(error) if error.faulted() => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time,
    });
}
impl SourceCoordinator {
    pub fn prepare_settings_change_with_reads(
        &self,
        changes: &[SettingChangeV1],
        request: &ReadDependencyRequestV3,
        r: &mut Recorder,
    ) -> Result<PreparedSettingsChangeV2, SettingsChangeErrorV2> {
        let result = (|| {
            self.ensure_ordinary_admission()
                .map_err(SettingsChangeErrorV2::Reads)?;
            let limit = self
                .settings_activation
                .ok_or(SettingsChangeErrorV2::Settings(SettingsError::Unavailable))?
                .max_plan_bytes;
            if read_dependency::ReadInput::V3(request)
                .retained_bytes(&[])
                .is_none_or(|bytes| bytes > limit)
            {
                return Err(SettingsChangeErrorV2::Settings(SettingsError::Limit));
            }
            let reads = read_dependency::ReadInput::V3(request)
                .copy(&[])
                .map_err(|e| SettingsChangeErrorV2::Reads(e.into()))?;
            self.prepare_settings_read_capture(changes, reads, r)
        })();
        observe(r, Boundary::SettingsDependencyPreparation, &result);
        result
    }
    pub(super) fn prepare_settings_read_capture(
        &self,
        changes: &[SettingChangeV1],
        reads: read_dependency::ReadCapture,
        r: &mut Recorder,
    ) -> Result<PreparedSettingsChangeV2, SettingsChangeErrorV2> {
        self.ensure_ordinary_admission()
            .map_err(SettingsChangeErrorV2::Reads)?;
        if !reads.configuration_profile() {
            return Err(SettingsChangeErrorV2::Settings(SettingsError::Faulted));
        }
        self.check_owned_read_capture(&reads, r)
            .map_err(SettingsChangeErrorV2::Reads)?;
        let settings = self
            .prepare_settings_change(changes, r)
            .map_err(SettingsChangeErrorV2::Settings)?;
        let prepared = PreparedSettingsChangeV2 {
            read_count: reads.predicates.len(),
            settings,
            reads,
            reads_required: true,
        };
        if prepared
            .retained_bytes()
            .map_err(SettingsChangeErrorV2::Settings)?
            > prepared.settings.plan.limits.max_plan_bytes
        {
            return Err(SettingsChangeErrorV2::Settings(SettingsError::Limit));
        }
        Ok(prepared)
    }
    pub(super) fn stage_settings_with_reads(
        &self,
        prepared: PreparedSettingsChangeV2,
        r: &mut Recorder,
    ) -> Result<settings::SettingsStage, SettingsChangeErrorV2> {
        let result = (|| {
            if !Arc::ptr_eq(&prepared.settings.owner, &self.namespace) {
                return Err(SettingsChangeErrorV2::Settings(SettingsError::Unavailable));
            }
            if !prepared.reads_required
                || prepared.reads.predicates.len() != prepared.read_count
                || !prepared.reads.configuration_profile()
            {
                return Err(SettingsChangeErrorV2::Settings(SettingsError::Faulted));
            }
            let read_result = self.validate_read_capture(&prepared.reads);
            super::observe(r, Boundary::ReadDependencyValidation, &read_result);
            read_result.map_err(SettingsChangeErrorV2::Reads)?;
            if prepared
                .retained_bytes()
                .map_err(SettingsChangeErrorV2::Settings)?
                > prepared.settings.plan.limits.max_plan_bytes
            {
                return Err(SettingsChangeErrorV2::Settings(SettingsError::Limit));
            }
            self.stage_settings_change(prepared.settings, r)
                .map_err(SettingsChangeErrorV2::Settings)
        })();
        observe(r, Boundary::SettingsDependencyValidation, &result);
        result
    }
    pub fn commit_settings_change_with_reads(
        &mut self,
        prepared: PreparedSettingsChangeV2,
        r: &mut Recorder,
    ) -> Result<SettingsUpdateV1, SettingsChangeErrorV2> {
        if self.faulted {
            let result = Err(SettingsChangeErrorV2::Settings(SettingsError::Faulted));
            observe(r, Boundary::SettingsDependencyPublication, &result);
            return result;
        }
        self.faulted = true;
        let result = self.stage_settings_with_reads(prepared, r).map(|stage| {
            let outcome = stage.outcome;
            stage.install(self);
            outcome
        });
        if !result.as_ref().err().is_some_and(|error| error.faulted()) {
            self.faulted = false;
        }
        observe(r, Boundary::SettingsDependencyPublication, &result);
        result
    }
}
