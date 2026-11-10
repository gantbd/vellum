//! Funded owner-local immutable interpretations. No recipient projection or UI state.
use super::*;
mod shutdown_inventory;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsRetentionLimitsV1 {
    pub max_issued: usize,
    pub max_retained: usize,
    /// Maximum logical envelope per interpretation or conditional basis, not physical bytes.
    pub max_snapshot_bytes: usize,
    /// Whole worst-case pool envelope, including lifetime tombstones.
    pub max_pool_bytes: usize,
}
impl SettingsRetentionLimitsV1 {
    /// Logical funding requirement, not a reservation or eligibility promise.
    pub fn required_pool_bytes(self) -> Result<usize, SettingsError> {
        cost(self)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SettingsRetentionUsageV1 {
    pub issued_lifetime: usize,
    pub retained: usize,
    /// Includes conditional-basis context/dependency envelopes as well as compiled roots.
    pub retained_snapshot_bytes: usize,
    pub reserved_pool_bytes: usize,
    pub open_preparations: usize,
    pub closed: bool,
}
#[derive(Clone)]
pub struct SettingsInterpretationId {
    owner: Arc<()>,
    slot: usize,
}
impl PartialEq for SettingsInterpretationId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for SettingsInterpretationId {}
impl std::fmt::Debug for SettingsInterpretationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SettingsInterpretationId(<opaque>)")
    }
}
#[derive(Debug)]
pub(in crate::coordinator) struct Pool {
    pub(in crate::coordinator) limits: SettingsRetentionLimitsV1,
    pub(in crate::coordinator) reservation: retention::Reservation,
    charge: usize,
    pub(in crate::coordinator) rows: Vec<Option<PoolRow>>,
}
#[derive(Debug)]
pub(in crate::coordinator) enum PoolRow {
    LifecycleBasis(Box<super::basis::lifecycle::LifecycleBasisCapture>),
    DefinitionBasis(Box<super::basis::definition::DefinitionBasisCapture>),
    Interpretation(Arc<CompiledSettingsV1>),
    WriteBasis(Box<super::basis::BasisCapture>),
}
impl PoolRow {
    fn logical_bytes(&self) -> usize {
        match self {
            Self::LifecycleBasis(row) => row.bytes,
            Self::DefinitionBasis(row) => row.bytes,
            Self::Interpretation(root) => root.usage().logical_bytes,
            Self::WriteBasis(row) => row.bytes,
        }
    }
}
#[derive(Debug, Default)]
pub(in crate::coordinator) enum RetentionPool {
    #[default]
    Unconfigured,
    Active(Pool),
    Closed {
        issued: usize,
    },
}
impl RetentionPool {
    pub(in crate::coordinator) fn closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }
}
fn cost(limits: SettingsRetentionLimitsV1) -> Result<usize, SettingsError> {
    std::mem::size_of::<Pool>()
        .checked_add(
            limits
                .max_issued
                .checked_mul(std::mem::size_of::<Option<PoolRow>>())
                .ok_or(SettingsError::Limit)?,
        )
        .and_then(|n| {
            limits
                .max_retained
                .checked_mul(limits.max_snapshot_bytes)
                .and_then(|x| n.checked_add(x))
        })
        .filter(|n| *n <= limits.max_pool_bytes)
        .ok_or(SettingsError::Limit)
}
pub(in crate::coordinator) fn parent_error(error: RetentionErrorV1) -> SettingsError {
    match error {
        RetentionErrorV1::Unavailable => SettingsError::Unavailable,
        RetentionErrorV1::Limit => SettingsError::Limit,
        RetentionErrorV1::Faulted => SettingsError::Faulted,
    }
}
pub(in crate::coordinator) fn live(pool: &Pool) -> usize {
    pool.rows.iter().filter(|row| row.is_some()).count()
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn retained_settings_result<T>(
        &mut self,
        r: &mut Recorder,
        boundary: Boundary,
        result: Result<T, SettingsError>,
    ) -> Result<T, SettingsError> {
        self.retained_settings_result_timed(r, boundary, result, None)
    }
    pub(in crate::coordinator) fn retained_settings_result_timed<T>(
        &mut self,
        r: &mut Recorder,
        boundary: Boundary,
        result: Result<T, SettingsError>,
        service_time: Option<std::time::Duration>,
    ) -> Result<T, SettingsError> {
        if matches!(result, Err(SettingsError::Faulted)) {
            self.faulted = true;
        }
        crate::settings::observe_service(r, boundary, &result, service_time);
        result
    }
    pub fn configure_settings_retention(
        &mut self,
        limits: SettingsRetentionLimitsV1,
        r: &mut Recorder,
    ) -> Result<bool, SettingsError> {
        let result = (|| {
            self.ensure_operational()
                .map_err(|_| SettingsError::Faulted)?;
            match &self.settings_retention {
                RetentionPool::Active(pool) => {
                    self.retention
                        .validate(&pool.reservation)
                        .map_err(parent_error)?;
                    return if pool.limits == limits {
                        Ok(false)
                    } else {
                        Err(SettingsError::Unavailable)
                    };
                }
                RetentionPool::Closed { .. } => return Err(SettingsError::Unavailable),
                RetentionPool::Unconfigured => {}
            }
            self.ensure_admission_unfenced()
                .map_err(super::admission_error)?;
            let charge = cost(limits)?;
            self.retention.check(charge).map_err(parent_error)?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(limits.max_issued)
                .map_err(|_| SettingsError::Capacity)?;
            let reservation = self.retention.reserve(charge).map_err(parent_error)?;
            self.settings_retention = RetentionPool::Active(Pool {
                limits,
                reservation,
                charge,
                rows,
            });
            Ok(true)
        })();
        self.retained_settings_result(r, Boundary::SettingsRetentionConfiguration, result)
    }
    pub fn settings_retention_usage(&self) -> SettingsRetentionUsageV1 {
        let mut usage = SettingsRetentionUsageV1 {
            open_preparations: Arc::strong_count(&self.settings_plan_life) - 1,
            ..SettingsRetentionUsageV1::default()
        };
        match &self.settings_retention {
            RetentionPool::Active(pool) => {
                usage.issued_lifetime = pool.rows.len();
                usage.retained = live(pool);
                usage.reserved_pool_bytes = pool.charge;
                usage.retained_snapshot_bytes = pool
                    .rows
                    .iter()
                    .flatten()
                    .fold(0usize, |n, row| n.saturating_add(row.logical_bytes()));
            }
            RetentionPool::Closed { issued } => {
                usage.issued_lifetime = *issued;
                usage.closed = true;
            }
            RetentionPool::Unconfigured => {}
        }
        usage
    }
    pub fn acquire_settings_interpretation(
        &mut self,
        r: &mut Recorder,
    ) -> Result<SettingsInterpretationId, SettingsError> {
        let result = (|| {
            self.ensure_ordinary_admission()
                .map_err(super::admission_error)?;
            let state = self.settings.as_ref().ok_or(SettingsError::Unavailable)?;
            let RetentionPool::Active(pool) = &mut self.settings_retention else {
                return Err(SettingsError::Unavailable);
            };
            self.retention
                .validate(&pool.reservation)
                .map_err(parent_error)?;
            if pool.rows.len() >= pool.limits.max_issued
                || live(pool) >= pool.limits.max_retained
                || state.usage().logical_bytes > pool.limits.max_snapshot_bytes
            {
                return Err(SettingsError::Limit);
            }
            let id = SettingsInterpretationId {
                owner: self.namespace.clone(),
                slot: pool.rows.len(),
            };
            pool.rows.push(Some(PoolRow::Interpretation(state.clone())));
            Ok(id)
        })();
        self.retained_settings_result(r, Boundary::SettingsInterpretationAcquisition, result)
    }
    fn interpretation(
        &self,
        id: &SettingsInterpretationId,
    ) -> Result<&CompiledSettingsV1, SettingsError> {
        self.ensure_operational()
            .map_err(|_| SettingsError::Faulted)?;
        self.retained_settings_interpretation(id)
    }
    pub(in crate::coordinator) fn retained_settings_interpretation(
        &self,
        id: &SettingsInterpretationId,
    ) -> Result<&CompiledSettingsV1, SettingsError> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return Err(SettingsError::Unavailable);
        }
        let RetentionPool::Active(pool) = &self.settings_retention else {
            return Err(SettingsError::Unavailable);
        };
        self.retention
            .validate(&pool.reservation)
            .map_err(parent_error)?;
        match pool.rows.get(id.slot).and_then(Option::as_ref) {
            Some(PoolRow::Interpretation(root)) => Ok(root),
            _ => Err(SettingsError::Unavailable),
        }
    }
    pub fn settings_interpretation_basis(
        &self,
        id: &SettingsInterpretationId,
        r: &mut Recorder,
    ) -> Result<SettingsBasisV1, SettingsError> {
        let result = self.interpretation(id).map(CompiledSettingsV1::basis);
        crate::settings::observe(r, Boundary::SettingsInterpretationQuery, &result);
        result
    }
    /// Privileged historical borrowed values, not an F07 recipient capability.
    pub fn settings_interpretation_read(
        &self,
        id: &SettingsInterpretationId,
        target: &SettingIdentityV1,
        key: &str,
        r: &mut Recorder,
    ) -> Result<SettingReadV1<'_>, SettingsError> {
        let result = self
            .interpretation(id)
            .and_then(|state| state.read(target, key));
        crate::settings::observe(r, Boundary::SettingsInterpretationQuery, &result);
        result
    }
    /// Protected discharge remains available after an unrelated engine fault.
    pub fn release_settings_interpretation(
        &mut self,
        id: &SettingsInterpretationId,
        r: &mut Recorder,
    ) -> Result<(), SettingsError> {
        let result = (|| {
            if !Arc::ptr_eq(&id.owner, &self.namespace) {
                return Err(SettingsError::Unavailable);
            }
            let RetentionPool::Active(pool) = &mut self.settings_retention else {
                return Err(SettingsError::Unavailable);
            };
            let row = pool
                .rows
                .get_mut(id.slot)
                .ok_or(SettingsError::Unavailable)?;
            if !matches!(row, Some(PoolRow::Interpretation(_))) {
                return Err(SettingsError::Unavailable);
            }
            *row = None;
            Ok(())
        })();
        self.retained_settings_result(r, Boundary::SettingsInterpretationRelease, result)
    }
    pub fn retire_settings_retention(&mut self, r: &mut Recorder) -> Result<bool, SettingsError> {
        let result = (|| {
            let pool = match &mut self.settings_retention {
                RetentionPool::Unconfigured => return Err(SettingsError::Unavailable),
                RetentionPool::Closed { .. } => return Ok(false),
                RetentionPool::Active(pool) => pool,
            };
            if live(pool) != 0 || Arc::strong_count(&self.settings_plan_life) != 1 {
                return Err(SettingsError::Unavailable);
            }
            let issued = pool.rows.len();
            self.retention
                .release(&mut pool.reservation)
                .map_err(parent_error)?;
            self.settings_retention = RetentionPool::Closed { issued };
            Ok(true)
        })();
        self.retained_settings_result(r, Boundary::SettingsPoolRetirement, result)
    }
}
