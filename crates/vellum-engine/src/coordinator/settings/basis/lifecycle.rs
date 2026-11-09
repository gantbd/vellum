//! Conditional authored-layer issuer state, not registry-write authority.
use super::definition::ordinary;
use super::*;
use crate::settings::{SettingsLayerBasisV1, SettingsLayerLimitsV1};
#[cfg(test)]
mod tests;
#[derive(Clone)]
pub struct SettingsLifecycleBasisId {
    owner: Arc<()>,
    slot: usize,
}
impl PartialEq for SettingsLifecycleBasisId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for SettingsLifecycleBasisId {}
impl std::fmt::Debug for SettingsLifecycleBasisId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SettingsLifecycleBasisId(<opaque>)")
    }
}
/// Observation premises only: native create/remove/association grants remain separate.
pub struct TrustedSettingsLifecycleBasisFactsV1<'a> {
    pub common: &'a TrustedSettingsBasisFactsV1<'a>,
    pub lifecycle: SettingsLayerLimitsV1,
    pub basis_allowed: Option<bool>,
}
pub(in crate::coordinator) struct LifecycleBasisCapture {
    context: SettingsExecutionContextV1,
    expected: SettingsLayerBasisV1,
    activation: SettingsActivationLimitsV1,
    lifecycle: SettingsLayerLimitsV1,
    closed: bool,
    pub(in crate::coordinator) bytes: usize,
}
impl std::fmt::Debug for LifecycleBasisCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LifecycleBasisCapture(<redacted>)")
    }
}
fn lost(f: &TrustedSettingsLifecycleBasisFactsV1<'_>) -> bool {
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
    pub(in crate::coordinator) fn lifecycle_basis_binding_current(
        &self,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleBasisFactsV1<'_>,
    ) -> bool {
        self.basis_binding_current(c, f.common) && self.settings_layer_limits == Some(f.lifecycle)
    }
    pub fn capture_settings_lifecycle_basis(
        &mut self,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleBasisFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SettingsLifecycleBasisId, SettingsError> {
        let result = (|| {
            self.ensure_ordinary_admission()
                .map_err(settings::admission_error)?;
            if !self.lifecycle_basis_binding_current(c, f)
                || !ordinary(f.common)
                || f.basis_allowed != Some(true)
            {
                return Err(SettingsError::Unavailable);
            }
            let expected = self
                .settings
                .as_ref()
                .ok_or(SettingsError::Unavailable)?
                .layer_basis();
            expected.validate().map_err(|_| SettingsError::Faulted)?;
            let bytes = labels(c)
                .into_iter()
                .try_fold(std::mem::size_of::<LifecycleBasisCapture>(), |n, s| {
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
            let capture = LifecycleBasisCapture {
                context: copy_context(c)?,
                expected,
                activation: f.common.activation,
                lifecycle: f.lifecycle,
                closed: false,
                bytes,
            };
            let id = SettingsLifecycleBasisId {
                owner: self.namespace.clone(),
                slot: pool.rows.len(),
            };
            pool.rows
                .push(Some(PoolRow::LifecycleBasis(Box::new(capture))));
            Ok(id)
        })();
        self.retained_settings_result(r, Boundary::SettingsLifecycleBasisCapture, result)
    }
    fn held_lifecycle_basis(
        &self,
        id: &SettingsLifecycleBasisId,
        c: &SettingsExecutionContextV1,
    ) -> Option<&LifecycleBasisCapture> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return None;
        }
        let RetentionPool::Active(pool) = &self.settings_retention else {
            return None;
        };
        match pool.rows.get(id.slot)?.as_ref()? {
            PoolRow::LifecycleBasis(row) if row.context == *c => Some(row),
            _ => None,
        }
    }
    fn close_lifecycle_basis(&mut self, id: &SettingsLifecycleBasisId) {
        if let RetentionPool::Active(pool) = &mut self.settings_retention
            && let Some(Some(PoolRow::LifecycleBasis(row))) = pool.rows.get_mut(id.slot)
        {
            row.closed = true;
        }
    }
    pub(in crate::coordinator) fn validate_lifecycle_basis(
        &mut self,
        id: &SettingsLifecycleBasisId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleBasisFactsV1<'_>,
    ) -> Result<(), SettingsError> {
        let row = self
            .held_lifecycle_basis(id, c)
            .ok_or(SettingsError::Unavailable)?;
        if row.closed
            || row.activation != f.common.activation
            || row.lifecycle != f.lifecycle
            || !self.lifecycle_basis_binding_current(c, f)
        {
            return Err(SettingsError::Unavailable);
        }
        if lost(f) {
            self.close_lifecycle_basis(id);
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
            .layer_basis();
        current.validate().map_err(|_| SettingsError::Faulted)?;
        if current != row.expected {
            self.close_lifecycle_basis(id);
            return Err(SettingsError::Unavailable);
        }
        Ok(())
    }
    pub(in crate::coordinator) fn settings_lifecycle_condition(
        &self,
        id: &SettingsLifecycleBasisId,
        c: &SettingsExecutionContextV1,
    ) -> Result<SettingsLayerBasisV1, SettingsError> {
        let row = self
            .held_lifecycle_basis(id, c)
            .ok_or(SettingsError::Unavailable)?;
        if row.closed
            || self.settings_activation != Some(row.activation)
            || self.settings_layer_limits != Some(row.lifecycle)
        {
            return Err(SettingsError::Unavailable);
        }
        row.expected
            .validate()
            .map_err(|_| SettingsError::Faulted)?;
        Ok(row.expected)
    }
    pub fn inspect_settings_lifecycle_basis(
        &mut self,
        id: &SettingsLifecycleBasisId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleBasisFactsV1<'_>,
        r: &mut Recorder,
    ) -> SettingsBasisAvailabilityV1 {
        let result = if self.faulted {
            Err(SettingsError::Faulted)
        } else {
            self.validate_lifecycle_basis(id, c, f)
        };
        match self.retained_settings_result(r, Boundary::SettingsLifecycleBasisInspection, result) {
            Ok(()) => SettingsBasisAvailabilityV1::Current,
            Err(_) => SettingsBasisAvailabilityV1::Unavailable,
        }
    }
    fn lifecycle_basis_control(
        &self,
        id: &SettingsLifecycleBasisId,
        c: &SettingsExecutionContextV1,
        f: &SettingsBasisControlV1<'_>,
    ) -> bool {
        f.context == c
            && f.owner_control_allowed == Some(true)
            && self.held_lifecycle_basis(id, c).is_some()
    }
    pub fn stop_settings_lifecycle_basis(
        &mut self,
        id: &SettingsLifecycleBasisId,
        c: &SettingsExecutionContextV1,
        f: &SettingsBasisControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let result = if self.lifecycle_basis_control(id, c, f) {
            self.close_lifecycle_basis(id);
            Ok(())
        } else {
            Err(SettingsError::Unavailable)
        };
        self.retained_settings_result(r, Boundary::SettingsLifecycleBasisStop, result)
            .is_ok()
    }
    pub fn retire_settings_lifecycle_basis(
        &mut self,
        id: &SettingsLifecycleBasisId,
        c: &SettingsExecutionContextV1,
        f: &SettingsBasisControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let result = (|| {
            if !self.lifecycle_basis_control(id, c, f)
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
        self.retained_settings_result(r, Boundary::SettingsLifecycleBasisRetirement, result)
            .is_ok()
    }
}
