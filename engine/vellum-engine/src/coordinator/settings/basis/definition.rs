//! Declaration-role condition only. No layer/read authority or retained source/root.
use super::*;
use crate::settings::{
    DefinitionCapture, SettingsDefinitionLimitsV1, SettingsInterpretationEpochsV1,
};
#[derive(Clone)]
pub struct SettingsDefinitionBasisId {
    owner: Arc<()>,
    slot: usize,
}
impl PartialEq for SettingsDefinitionBasisId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for SettingsDefinitionBasisId {}
impl std::fmt::Debug for SettingsDefinitionBasisId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SettingsDefinitionBasisId(<opaque>)")
    }
}
/// Trusted embedding premises. Ordinary layer basis permissions/grants are not this role.
pub struct TrustedSettingsDefinitionBasisFactsV1<'a> {
    pub common: &'a TrustedSettingsBasisFactsV1<'a>,
    pub definition: SettingsDefinitionLimitsV1,
    pub basis_allowed: Option<bool>,
}
pub(in crate::coordinator) struct DefinitionBasisCapture {
    context: SettingsExecutionContextV1,
    expected: SettingsInterpretationEpochsV1,
    activation: SettingsActivationLimitsV1,
    definition: SettingsDefinitionLimitsV1,
    closed: bool,
    pub(in crate::coordinator) bytes: usize,
}
impl std::fmt::Debug for DefinitionBasisCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DefinitionBasisCapture(<redacted>)")
    }
}
pub(in crate::coordinator) fn ordinary(f: &TrustedSettingsBasisFactsV1<'_>) -> bool {
    [
        f.authorized,
        f.scope,
        f.continuous,
        f.live,
        f.not_stopped,
        f.interpretation_current,
        f.resources_ready,
        f.owner_control_allowed,
        f.settings_writable,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn lost(f: &TrustedSettingsDefinitionBasisFactsV1<'_>) -> bool {
    let c = f.common;
    [
        c.authorized,
        c.scope,
        c.continuous,
        c.live,
        c.interpretation_current,
        c.settings_writable,
        f.basis_allowed,
    ]
    .contains(&Some(false))
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn definition_basis_binding_current(
        &self,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsDefinitionBasisFactsV1<'_>,
    ) -> bool {
        self.basis_binding_current(c, f.common)
            && self.settings_definition_limits == Some(f.definition)
    }
    pub fn capture_settings_definition_basis(
        &mut self,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsDefinitionBasisFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SettingsDefinitionBasisId, SettingsError> {
        let result = (|| {
            self.ensure_ordinary_admission()
                .map_err(settings::admission_error)?;
            if !self.definition_basis_binding_current(c, f)
                || !ordinary(f.common)
                || f.basis_allowed != Some(true)
            {
                return Err(SettingsError::Unavailable);
            }
            let expected = self
                .settings
                .as_ref()
                .ok_or(SettingsError::Unavailable)?
                .basis()
                .interpretation();
            DefinitionCapture::validate_expected(expected).map_err(|_| SettingsError::Faulted)?;
            let bytes = labels(c)
                .into_iter()
                .try_fold(std::mem::size_of::<DefinitionBasisCapture>(), |n, s| {
                    n.checked_add(s.len())
                })
                .ok_or(SettingsError::Limit)?;
            let RetentionPool::Active(pool) = &mut self.settings_retention else {
                return Err(SettingsError::Unavailable);
            };
            self.retention
                .validate(&pool.reservation)
                .map_err(parent_error)?;
            if pool.rows.len() >= pool.limits.max_issued
                || live(pool) >= pool.limits.max_retained
                || bytes > pool.limits.max_snapshot_bytes
            {
                return Err(SettingsError::Limit);
            }
            let row = DefinitionBasisCapture {
                context: copy_context(c)?,
                expected,
                activation: f.common.activation,
                definition: f.definition,
                closed: false,
                bytes,
            };
            let id = SettingsDefinitionBasisId {
                owner: self.namespace.clone(),
                slot: pool.rows.len(),
            };
            pool.rows
                .push(Some(PoolRow::DefinitionBasis(Box::new(row))));
            Ok(id)
        })();
        self.retained_settings_result(r, Boundary::DefinitionBasisCapture, result)
    }
    fn held_definition_basis(
        &self,
        id: &SettingsDefinitionBasisId,
        c: &SettingsExecutionContextV1,
    ) -> Option<&DefinitionBasisCapture> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return None;
        }
        let RetentionPool::Active(pool) = &self.settings_retention else {
            return None;
        };
        match pool.rows.get(id.slot)?.as_ref()? {
            PoolRow::DefinitionBasis(row) if row.context == *c => Some(row),
            _ => None,
        }
    }
    fn close_definition_basis(&mut self, id: &SettingsDefinitionBasisId) {
        if let RetentionPool::Active(pool) = &mut self.settings_retention
            && let Some(Some(PoolRow::DefinitionBasis(row))) = pool.rows.get_mut(id.slot)
        {
            row.closed = true;
        }
    }
    pub(in crate::coordinator) fn settings_definition_condition(
        &self,
        id: &SettingsDefinitionBasisId,
        c: &SettingsExecutionContextV1,
    ) -> Result<SettingsInterpretationEpochsV1, SettingsError> {
        let row = self
            .held_definition_basis(id, c)
            .ok_or(SettingsError::Unavailable)?;
        if row.closed
            || self.settings_definition_limits != Some(row.definition)
            || self.settings_activation != Some(row.activation)
        {
            return Err(SettingsError::Unavailable);
        }
        Ok(row.expected)
    }
    pub(in crate::coordinator) fn validate_settings_definition_basis(
        &mut self,
        id: &SettingsDefinitionBasisId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsDefinitionBasisFactsV1<'_>,
    ) -> Result<(), SettingsError> {
        let row = self
            .held_definition_basis(id, c)
            .ok_or(SettingsError::Unavailable)?;
        if row.closed
            || row.definition != f.definition
            || row.activation != f.common.activation
            || !self.definition_basis_binding_current(c, f)
        {
            return Err(SettingsError::Unavailable);
        }
        if lost(f) {
            self.close_definition_basis(id);
            return Err(SettingsError::Unavailable);
        }
        if !ordinary(f.common) || f.basis_allowed != Some(true) {
            return Err(SettingsError::Unavailable);
        }
        let RetentionPool::Active(pool) = &self.settings_retention else {
            return Err(SettingsError::Unavailable);
        };
        self.retention
            .validate(&pool.reservation)
            .map_err(parent_error)?;
        let current = self
            .settings
            .as_ref()
            .ok_or(SettingsError::Unavailable)?
            .basis()
            .interpretation();
        DefinitionCapture::validate_expected(current).map_err(|_| SettingsError::Faulted)?;
        if current != row.expected {
            self.close_definition_basis(id);
            return Err(SettingsError::Unavailable);
        }
        Ok(())
    }
    pub fn inspect_settings_definition_basis(
        &mut self,
        id: &SettingsDefinitionBasisId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsDefinitionBasisFactsV1<'_>,
        r: &mut Recorder,
    ) -> SettingsBasisAvailabilityV1 {
        let result = if self.faulted {
            Err(SettingsError::Faulted)
        } else {
            self.validate_settings_definition_basis(id, c, f)
        };
        match self.retained_settings_result(r, Boundary::DefinitionBasisInspection, result) {
            Ok(()) => SettingsBasisAvailabilityV1::Current,
            Err(_) => SettingsBasisAvailabilityV1::Unavailable,
        }
    }
    fn definition_basis_control(
        &self,
        id: &SettingsDefinitionBasisId,
        c: &SettingsExecutionContextV1,
        f: &SettingsBasisControlV1<'_>,
    ) -> bool {
        f.context == c
            && f.owner_control_allowed == Some(true)
            && self.held_definition_basis(id, c).is_some()
    }
    pub fn stop_settings_definition_basis(
        &mut self,
        id: &SettingsDefinitionBasisId,
        c: &SettingsExecutionContextV1,
        f: &SettingsBasisControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let result = if self.definition_basis_control(id, c, f) {
            self.close_definition_basis(id);
            Ok(())
        } else {
            Err(SettingsError::Unavailable)
        };
        self.retained_settings_result(r, Boundary::DefinitionBasisStop, result)
            .is_ok()
    }
    pub fn retire_settings_definition_basis(
        &mut self,
        id: &SettingsDefinitionBasisId,
        c: &SettingsExecutionContextV1,
        f: &SettingsBasisControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let result = (|| {
            if !self.definition_basis_control(id, c, f)
                || f.retention_released != Some(true)
                || f.delivery_drained != Some(true)
            {
                return Err(SettingsError::Unavailable);
            }
            let RetentionPool::Active(pool) = &mut self.settings_retention else {
                return Err(SettingsError::Unavailable);
            };
            self.retention
                .validate(&pool.reservation)
                .map_err(parent_error)?;
            pool.rows[id.slot] = None;
            Ok(())
        })();
        self.retained_settings_result(r, Boundary::DefinitionBasisRetirement, result)
            .is_ok()
    }
}
