//! Recipient-bound conditional bases. Trusted facts are not authenticated policy proof.
use super::retained::{PoolRow, live, parent_error};
use super::*;
use crate::settings::{SettingsActivationLimitsV1, WriteFootprint};
pub(in crate::coordinator) mod definition;
pub(in crate::coordinator) mod lifecycle;
pub use definition::{SettingsDefinitionBasisId, TrustedSettingsDefinitionBasisFactsV1};
pub use lifecycle::{SettingsLifecycleBasisId, TrustedSettingsLifecycleBasisFactsV1};
#[cfg(test)]
mod tests;
#[derive(Clone, PartialEq, Eq)]
pub struct SettingsExecutionContextV1 {
    pub authority: String,
    pub engine: String,
    pub recipient: String,
    pub purpose: String,
    pub destination: String,
    pub profile: String,
    pub continuity: String,
}
impl std::fmt::Debug for SettingsExecutionContextV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SettingsExecutionContextV1(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsBasisLimitsV1 {
    pub max_label_bytes: usize,
    pub max_context_bytes: usize,
    pub max_grants: usize,
    pub max_work: usize,
}
/// Populated only by the trusted embedding after authenticating/authorizing its caller.
/// `None` is unknown, never a denial or a grant. This is not a request payload.
pub struct TrustedSettingsBasisFactsV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub activation: SettingsActivationLimitsV1,
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub interpretation_current: Option<bool>,
    pub resources_ready: Option<bool>,
    pub owner_control_allowed: Option<bool>,
    pub settings_writable: Option<bool>,
    pub basis_allowed: Option<bool>,
    pub writable_layers: Option<&'a [SettingIdentityV1]>,
}
/// Trusted embedding discharge facts, not requester attestations. Protected controls
/// do not require current disclosure/mutation rights or a healthy owner.
pub struct SettingsBasisControlV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub owner_control_allowed: Option<bool>,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsBasisAvailabilityV1 {
    Current,
    Unavailable,
}
#[derive(Clone)]
pub struct SettingsWriteBasisId {
    owner: Arc<()>,
    slot: usize,
}
impl PartialEq for SettingsWriteBasisId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for SettingsWriteBasisId {}
impl std::fmt::Debug for SettingsWriteBasisId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SettingsWriteBasisId(<opaque>)")
    }
}
pub(in crate::coordinator) struct BasisProfile {
    engine: String,
    limits: SettingsBasisLimitsV1,
}
impl std::fmt::Debug for BasisProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BasisProfile(<redacted>)")
    }
}
impl BasisProfile {
    pub(in crate::coordinator) fn grant_work_limit(&self) -> usize {
        self.limits.max_work
    }
}
pub(in crate::coordinator) struct BasisCapture {
    context: SettingsExecutionContextV1,
    footprint: WriteFootprint,
    activation: SettingsActivationLimitsV1,
    closed: bool,
    pub(in crate::coordinator) bytes: usize,
}
impl std::fmt::Debug for BasisCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BasisCapture(<redacted>)")
    }
}
pub(in crate::coordinator) fn labels(c: &SettingsExecutionContextV1) -> [&str; 7] {
    [
        &c.authority,
        &c.engine,
        &c.recipient,
        &c.purpose,
        &c.destination,
        &c.profile,
        &c.continuity,
    ]
}
fn valid_context(c: &SettingsExecutionContextV1, p: SettingsBasisLimitsV1) -> bool {
    let mut bytes = std::mem::size_of::<SettingsExecutionContextV1>();
    for value in labels(c) {
        if value.is_empty() || value.len() > p.max_label_bytes {
            return false;
        }
        let Some(n) = bytes.checked_add(value.len()) else {
            return false;
        };
        bytes = n;
    }
    bytes <= p.max_context_bytes && p.max_work >= 7
}
fn owned_label(value: &str) -> Result<String, SettingsError> {
    let mut out = String::new();
    out.try_reserve_exact(value.len())
        .map_err(|_| SettingsError::Capacity)?;
    out.push_str(value);
    Ok(out)
}
pub(in crate::coordinator) fn copy_context(
    c: &SettingsExecutionContextV1,
) -> Result<SettingsExecutionContextV1, SettingsError> {
    Ok(SettingsExecutionContextV1 {
        authority: owned_label(&c.authority)?,
        engine: owned_label(&c.engine)?,
        recipient: owned_label(&c.recipient)?,
        purpose: owned_label(&c.purpose)?,
        destination: owned_label(&c.destination)?,
        profile: owned_label(&c.profile)?,
        continuity: owned_label(&c.continuity)?,
    })
}
fn grant_packet_valid(f: &TrustedSettingsBasisFactsV1<'_>, p: SettingsBasisLimitsV1) -> bool {
    let Some(rows) = f.writable_layers else {
        return true;
    };
    if rows.len() > p.max_grants || rows.len() > p.max_work.saturating_sub(7) {
        return false;
    }
    rows.iter().all(|row| {
        row.authority == f.context.authority
            && !row.id.is_empty()
            && row.id.len() <= p.max_label_bytes
            && row.incarnation > 0
            && row.incarnation <= 9_007_199_254_740_991
    })
}
pub(in crate::coordinator) fn all_guards(f: &TrustedSettingsBasisFactsV1<'_>) -> bool {
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
        f.basis_allowed,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn observed_loss(f: &TrustedSettingsBasisFactsV1<'_>) -> bool {
    [
        f.authorized,
        f.scope,
        f.continuous,
        f.live,
        f.interpretation_current,
        f.settings_writable,
        f.basis_allowed,
    ]
    .contains(&Some(false))
}
impl SourceCoordinator {
    pub fn configure_settings_bases(
        &mut self,
        engine: &str,
        limits: SettingsBasisLimitsV1,
        r: &mut Recorder,
    ) -> Result<bool, SettingsError> {
        let result = (|| {
            self.ensure_operational()
                .map_err(|_| SettingsError::Faulted)?;
            if engine.is_empty() || engine.len() > limits.max_label_bytes {
                return Err(SettingsError::InvalidIdentity);
            }
            if let Some(old) = &self.settings_basis_profile {
                return if old.engine == engine && old.limits == limits {
                    Ok(false)
                } else {
                    Err(SettingsError::Unavailable)
                };
            }
            self.ensure_admission_unfenced()
                .map_err(super::admission_error)?;
            if self.settings_retention.closed() {
                return Err(SettingsError::Unavailable);
            }
            self.settings_basis_profile = Some(BasisProfile {
                engine: owned_label(engine)?,
                limits,
            });
            Ok(true)
        })();
        crate::settings::observe(r, Boundary::SettingsBasisConfiguration, &result);
        result
    }
    pub(in crate::coordinator) fn basis_context_current(
        &self,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsBasisFactsV1<'_>,
    ) -> bool {
        self.basis_binding_current(c, f)
            && self
                .settings_basis_profile
                .as_ref()
                .is_some_and(|p| grant_packet_valid(f, p.limits))
    }
    pub(in crate::coordinator) fn basis_binding_current(
        &self,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsBasisFactsV1<'_>,
    ) -> bool {
        self.settings_basis_profile.as_ref().is_some_and(|p| {
            valid_context(c, p.limits)
                && valid_context(f.context, p.limits)
                && p.engine == c.engine
                && c == f.context
                && self.settings_activation == Some(f.activation)
                && self
                    .settings
                    .as_ref()
                    .is_some_and(|s| s.settings_authority() == c.authority)
        })
    }
    pub fn capture_settings_write_basis(
        &mut self,
        layer: &SettingIdentityV1,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsBasisFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SettingsWriteBasisId, SettingsError> {
        let result = (|| {
            self.ensure_ordinary_admission()
                .map_err(super::admission_error)?;
            if !self.basis_context_current(c, f)
                || !all_guards(f)
                || !f.writable_layers.is_some_and(|rows| rows.contains(layer))
            {
                return Err(SettingsError::Unavailable);
            }
            let p = self
                .settings_basis_profile
                .as_ref()
                .ok_or(SettingsError::Unavailable)?;
            let root = self
                .settings
                .as_ref()
                .ok_or(SettingsError::Unavailable)?
                .clone();
            let footprint = WriteFootprint::capture(root, layer, p.limits.max_work)?;
            let bytes = footprint
                .retained_bytes()?
                .checked_add(
                    std::mem::size_of::<BasisCapture>() - std::mem::size_of::<WriteFootprint>(),
                )
                .and_then(|n| {
                    labels(c)
                        .into_iter()
                        .try_fold(n, |n, s| n.checked_add(s.len()))
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
            let capture = BasisCapture {
                context: copy_context(c)?,
                footprint,
                activation: f.activation,
                closed: false,
                bytes,
            };
            let id = SettingsWriteBasisId {
                owner: self.namespace.clone(),
                slot: pool.rows.len(),
            };
            pool.rows.push(Some(PoolRow::WriteBasis(Box::new(capture))));
            Ok(id)
        })();
        self.retained_settings_result(r, Boundary::SettingsBasisCapture, result)
    }
    fn held_write_basis(
        &self,
        id: &SettingsWriteBasisId,
        c: &SettingsExecutionContextV1,
    ) -> Option<&BasisCapture> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return None;
        }
        let RetentionPool::Active(pool) = &self.settings_retention else {
            return None;
        };
        match pool.rows.get(id.slot)?.as_ref()? {
            PoolRow::WriteBasis(row) if row.context == *c => Some(row),
            _ => None,
        }
    }
    fn close_write_basis(&mut self, id: &SettingsWriteBasisId) {
        if let RetentionPool::Active(pool) = &mut self.settings_retention
            && let Some(Some(PoolRow::WriteBasis(row))) = pool.rows.get_mut(id.slot)
        {
            row.closed = true;
        }
    }
    pub fn inspect_settings_write_basis(
        &mut self,
        id: &SettingsWriteBasisId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsBasisFactsV1<'_>,
        r: &mut Recorder,
    ) -> SettingsBasisAvailabilityV1 {
        let result = if self.faulted {
            Err(SettingsError::Faulted)
        } else {
            self.validate_settings_write_basis(id, c, f)
        };
        match self.retained_settings_result(r, Boundary::SettingsBasisInspection, result) {
            Ok(()) => SettingsBasisAvailabilityV1::Current,
            Err(_) => SettingsBasisAvailabilityV1::Unavailable,
        }
    }
    /// Caller owns the outer healthy check/publication fence. This does not clear it.
    pub(in crate::coordinator) fn validate_settings_write_basis(
        &mut self,
        id: &SettingsWriteBasisId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsBasisFactsV1<'_>,
    ) -> Result<(), SettingsError> {
        let row = self
            .held_write_basis(id, c)
            .ok_or(SettingsError::Unavailable)?;
        if row.closed || row.activation != f.activation || !self.basis_context_current(c, f) {
            return Err(SettingsError::Unavailable);
        }
        if observed_loss(f) {
            self.close_write_basis(id);
            return Err(SettingsError::Unavailable);
        }
        if !all_guards(f) {
            return Err(SettingsError::Unavailable);
        }
        let Some(layers) = f.writable_layers else {
            return Err(SettingsError::Unavailable);
        };
        if !layers.contains(row.footprint.layer()) {
            self.close_write_basis(id);
            return Err(SettingsError::Unavailable);
        }
        let RetentionPool::Active(pool) = &self.settings_retention else {
            return Err(SettingsError::Unavailable);
        };
        self.retention
            .validate(&pool.reservation)
            .map_err(parent_error)?;
        let p = self
            .settings_basis_profile
            .as_ref()
            .ok_or(SettingsError::Unavailable)?;
        let current = self.settings.as_ref().ok_or(SettingsError::Unavailable)?;
        if !row.footprint.current(current, p.limits.max_work)? {
            self.close_write_basis(id);
            return Err(SettingsError::Unavailable);
        }
        Ok(())
    }
    pub(in crate::coordinator) fn settings_write_condition<'a>(
        &'a self,
        id: &SettingsWriteBasisId,
        c: &SettingsExecutionContextV1,
    ) -> Result<(&'a SettingIdentityV1, u64), SettingsError> {
        let row = self
            .held_write_basis(id, c)
            .ok_or(SettingsError::Unavailable)?;
        if row.closed || self.settings_activation != Some(row.activation) {
            return Err(SettingsError::Unavailable);
        }
        Ok((row.footprint.layer(), row.footprint.generation()))
    }
    /// Pure structural check before any conditional permission-loss observer runs.
    pub(in crate::coordinator) fn settings_write_condition_current(
        &self,
        id: &SettingsWriteBasisId,
        c: &SettingsExecutionContextV1,
    ) -> Result<(), SettingsError> {
        self.settings_write_condition(id, c)?;
        let row = self
            .held_write_basis(id, c)
            .ok_or(SettingsError::Unavailable)?;
        let p = self
            .settings_basis_profile
            .as_ref()
            .ok_or(SettingsError::Unavailable)?;
        let root = self.settings.as_ref().ok_or(SettingsError::Unavailable)?;
        if !row.footprint.current(root, p.limits.max_work)? {
            return Err(SettingsError::Stale);
        }
        Ok(())
    }
    fn basis_control_matches(
        &self,
        id: &SettingsWriteBasisId,
        c: &SettingsExecutionContextV1,
        f: &SettingsBasisControlV1<'_>,
    ) -> bool {
        f.owner_control_allowed == Some(true)
            && f.context == c
            && self.held_write_basis(id, c).is_some()
    }
    pub fn stop_settings_write_basis(
        &mut self,
        id: &SettingsWriteBasisId,
        c: &SettingsExecutionContextV1,
        f: &SettingsBasisControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let result = if self.basis_control_matches(id, c, f) {
            self.close_write_basis(id);
            Ok(())
        } else {
            Err(SettingsError::Unavailable)
        };
        self.retained_settings_result(r, Boundary::SettingsBasisStop, result)
            .is_ok()
    }
    pub fn retire_settings_write_basis(
        &mut self,
        id: &SettingsWriteBasisId,
        c: &SettingsExecutionContextV1,
        f: &SettingsBasisControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let result = (|| {
            if !self.basis_control_matches(id, c, f)
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
        self.retained_settings_result(r, Boundary::SettingsBasisRetirement, result)
            .is_ok()
    }
}
