//! Metadata-only conditional references, separately funded from settings snapshots.
use super::*;
mod shutdown_inventory;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SerializationBasisLimitsV1 {
    pub serialization: SourceSerializationLimitsV1,
    pub max_contexts: usize,
    pub max_issued_contexts: usize,
    pub max_context_bytes: usize,
    pub max_held_bytes: usize,
}
impl SerializationBasisLimitsV1 {
    /// Native logical envelopes including lifetime tombstones; not JSON or physical heap bytes.
    pub fn required_pool_bytes(self) -> Result<usize, Error> {
        std::mem::size_of::<Pool>()
            .checked_add(
                self.max_issued_contexts
                    .checked_mul(std::mem::size_of::<Option<Box<Record>>>())
                    .ok_or(Error::Limit)?,
            )
            .and_then(|n| {
                self.max_contexts
                    .checked_mul(self.max_context_bytes)
                    .and_then(|m| n.checked_add(m))
            })
            .filter(|n| *n <= self.max_held_bytes)
            .ok_or(Error::Limit)
    }
}
/// Host control premises, not requester data or authenticated policy evidence.
pub struct SerializationPoolControlV1 {
    pub owner_control_allowed: Option<bool>,
    pub resources_ready: Option<bool>,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
}
pub struct TrustedSerializationBasisFactsV1<'a> {
    pub native: &'a TrustedSerializationFactsV1<'a>,
    pub owner_control_allowed: Option<bool>,
    pub basis_allowed: Option<bool>,
    pub host_basis_allowed: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SerializationBasisAvailabilityV1 {
    Current,
    Unavailable,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SerializationBasisUsageV1 {
    pub issued_lifetime: usize,
    pub retained: usize,
    pub retained_context_bytes: usize,
    pub reserved_pool_bytes: usize,
    pub closed: bool,
}
/// Clones alias the same opaque reference; they do not create another condition or allowance.
#[derive(Clone)]
pub struct SerializationBasisId {
    owner: Arc<()>,
    slot: usize,
}
impl PartialEq for SerializationBasisId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for SerializationBasisId {}
impl std::fmt::Debug for SerializationBasisId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SerializationBasisId(<opaque>)")
    }
}
struct Record {
    context: SettingsExecutionContextV1,
    document: DocumentId,
    generation: u64,
    bytes: usize,
    closed: bool,
}
impl std::fmt::Debug for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SerializationBasisRecord(<redacted>)")
    }
}
#[derive(Debug)]
pub(in crate::coordinator) struct Pool {
    limits: SerializationBasisLimitsV1,
    reservation: retention::Reservation,
    charge: usize,
    rows: Vec<Option<Box<Record>>>,
}
#[derive(Debug, Default)]
pub(in crate::coordinator) enum BasisPool {
    #[default]
    Unconfigured,
    Active(Pool),
    Closed {
        issued: usize,
    },
}
impl BasisPool {
    pub(in crate::coordinator) fn document_pins(&self, document: &DocumentId) -> usize {
        match self {
            Self::Unconfigured | Self::Closed { .. } => 0,
            Self::Active(pool) => pool
                .rows
                .iter()
                .flatten()
                .filter(|row| row.document == *document)
                .count(),
        }
    }
}
fn parent_error(e: RetentionErrorV1) -> Error {
    match e {
        RetentionErrorV1::Unavailable => Error::Unavailable,
        RetentionErrorV1::Limit => Error::Limit,
        RetentionErrorV1::Faulted => Error::Faulted,
    }
}
fn ordinary(f: &TrustedSerializationBasisFactsV1<'_>) -> bool {
    let n = f.native;
    [
        n.authorized,
        n.scope,
        n.continuous,
        n.live,
        n.not_stopped,
        n.interpretation_current,
        n.resources_ready,
        n.settings_writable,
        n.write_allowed,
        f.owner_control_allowed,
        f.basis_allowed,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn loss(f: &TrustedSerializationBasisFactsV1<'_>) -> bool {
    let n = f.native;
    [
        n.authorized,
        n.scope,
        n.continuous,
        n.live,
        n.interpretation_current,
        n.settings_writable,
        n.write_allowed,
        f.basis_allowed,
    ]
    .contains(&Some(false))
}
impl SourceCoordinator {
    fn serialization_basis_result<T>(
        &mut self,
        r: &mut Recorder,
        b: Boundary,
        result: Result<T, Error>,
    ) -> Result<T, Error> {
        if result.as_ref().err().is_some_and(|e| e.faulted()) {
            self.faulted = true;
        }
        observe(r, b, &result);
        result
    }
    pub fn configure_serialization_bases(
        &mut self,
        limits: SerializationBasisLimitsV1,
        f: &SerializationPoolControlV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = (|| {
            self.ensure_ordinary_admission()?;
            if f.owner_control_allowed != Some(true)
                || f.resources_ready != Some(true)
                || self
                    .serialization_profile
                    .as_ref()
                    .is_none_or(|p| p.limits != limits.serialization)
                || !matches!(self.serialization_bases, BasisPool::Unconfigured)
            {
                return Err(Error::Unavailable);
            }
            let charge = limits.required_pool_bytes()?;
            if std::mem::size_of::<Record>() > limits.max_context_bytes {
                return Err(Error::Limit);
            }
            self.retention.check(charge).map_err(parent_error)?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(limits.max_issued_contexts)
                .map_err(|_| Error::Capacity)?;
            let reservation = self.retention.reserve(charge).map_err(parent_error)?;
            self.serialization_bases = BasisPool::Active(Pool {
                limits,
                reservation,
                charge,
                rows,
            });
            Ok(())
        })();
        self.serialization_basis_result(r, Boundary::SerializationBasisConfiguration, result)
    }
    pub fn serialization_basis_usage(&self) -> SerializationBasisUsageV1 {
        match &self.serialization_bases {
            BasisPool::Unconfigured => SerializationBasisUsageV1::default(),
            BasisPool::Closed { issued } => SerializationBasisUsageV1 {
                issued_lifetime: *issued,
                closed: true,
                ..SerializationBasisUsageV1::default()
            },
            BasisPool::Active(p) => SerializationBasisUsageV1 {
                issued_lifetime: p.rows.len(),
                retained: p.rows.iter().flatten().count(),
                retained_context_bytes: p
                    .rows
                    .iter()
                    .flatten()
                    .fold(0usize, |n, row| n.saturating_add(row.bytes)),
                reserved_pool_bytes: p.charge,
                closed: false,
            },
        }
    }
    pub fn capture_serialization_basis(
        &mut self,
        document: &DocumentId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationBasisFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SerializationBasisId, Error> {
        let result = (|| {
            self.ensure_ordinary_admission()?;
            if !self.serialization_facts_bound(document, c, f.native) || !ordinary(f) {
                return Err(Error::Unavailable);
            }
            let state = self.document(document)?.serialization;
            if !state.valid() {
                return Err(Error::Faulted);
            }
            let bytes = labels(c)
                .into_iter()
                .try_fold(std::mem::size_of::<Record>(), |n, s| n.checked_add(s.len()))
                .ok_or(Error::Limit)?;
            let BasisPool::Active(p) = &mut self.serialization_bases else {
                return Err(Error::Unavailable);
            };
            self.retention
                .validate(&p.reservation)
                .map_err(parent_error)?;
            if p.limits.serialization != f.native.profile {
                return Err(Error::Unavailable);
            }
            if bytes > p.limits.max_context_bytes
                || p.rows.len() >= p.limits.max_issued_contexts
                || p.rows.iter().flatten().count() >= p.limits.max_contexts
            {
                return Err(Error::Limit);
            }
            let row = Record {
                context: copy_context(c).map_err(|_| Error::Capacity)?,
                document: document.clone(),
                generation: state.generation,
                bytes,
                closed: false,
            };
            let id = SerializationBasisId {
                owner: self.namespace.clone(),
                slot: p.rows.len(),
            };
            p.rows.push(Some(Box::new(row)));
            Ok(id)
        })();
        self.serialization_basis_result(r, Boundary::SerializationBasisCapture, result)
    }
    fn serialization_basis_record(
        &self,
        id: &SerializationBasisId,
        c: &SettingsExecutionContextV1,
    ) -> Option<&Record> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return None;
        }
        let BasisPool::Active(p) = &self.serialization_bases else {
            return None;
        };
        p.rows
            .get(id.slot)?
            .as_deref()
            .filter(|row| row.context == *c)
    }
    fn close_serialization_basis(&mut self, id: &SerializationBasisId) {
        if let BasisPool::Active(p) = &mut self.serialization_bases
            && let Some(Some(row)) = p.rows.get_mut(id.slot)
        {
            row.closed = true;
        }
    }
    // Outer caller owns healthy admission/publication fence. No native condition is released.
    pub(super) fn current_serialization_basis(
        &mut self,
        id: &SerializationBasisId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationBasisFactsV1<'_>,
    ) -> Result<(), Error> {
        let row = self
            .serialization_basis_record(id, c)
            .ok_or(Error::Unavailable)?;
        if row.closed
            || !self.serialization_facts_bound(&row.document, c, f.native)
            || f.native.write_allowed.is_none()
        {
            return Err(Error::Unavailable);
        }
        let BasisPool::Active(p) = &self.serialization_bases else {
            return Err(Error::Unavailable);
        };
        self.retention
            .validate(&p.reservation)
            .map_err(parent_error)?;
        if p.limits.serialization != f.native.profile {
            return Err(Error::Unavailable);
        }
        if loss(f) {
            self.close_serialization_basis(id);
            return Err(Error::Unavailable);
        }
        if !ordinary(f) {
            return Err(Error::Unavailable);
        }
        match self.document(&row.document) {
            Ok(d) if !d.serialization.valid() => return Err(Error::Faulted),
            Ok(d) if d.serialization.generation == row.generation => return Ok(()),
            Err(e) if e != CoordinatorError::Unavailable => return Err(e.into()),
            _ => {}
        }
        self.close_serialization_basis(id);
        Err(Error::Unavailable)
    }
    pub(super) fn serialization_basis_condition(
        &self,
        id: &SerializationBasisId,
        c: &SettingsExecutionContextV1,
    ) -> Result<(DocumentId, u64), Error> {
        let row = self
            .serialization_basis_record(id, c)
            .ok_or(Error::Unavailable)?;
        if row.closed {
            return Err(Error::Unavailable);
        }
        Ok((row.document.clone(), row.generation))
    }
    pub fn inspect_serialization_basis(
        &mut self,
        id: &SerializationBasisId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationBasisFactsV1<'_>,
        r: &mut Recorder,
    ) -> SerializationBasisAvailabilityV1 {
        let result = if self.faulted {
            Err(Error::Faulted)
        } else {
            self.current_serialization_basis(id, c, f)
        };
        match self.serialization_basis_result(r, Boundary::SerializationBasisInspection, result) {
            Ok(()) => SerializationBasisAvailabilityV1::Current,
            Err(_) => SerializationBasisAvailabilityV1::Unavailable,
        }
    }
    fn serialization_basis_control(
        &self,
        id: &SerializationBasisId,
        c: &SettingsExecutionContextV1,
        f: &SettingsBasisControlV1<'_>,
    ) -> bool {
        f.owner_control_allowed == Some(true)
            && f.context == c
            && self.serialization_basis_record(id, c).is_some()
    }
    pub fn stop_serialization_basis(
        &mut self,
        id: &SerializationBasisId,
        c: &SettingsExecutionContextV1,
        f: &SettingsBasisControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let result = if self.serialization_basis_control(id, c, f) {
            self.close_serialization_basis(id);
            Ok(())
        } else {
            Err(Error::Unavailable)
        };
        self.serialization_basis_result(r, Boundary::SerializationBasisStop, result)
            .is_ok()
    }
    pub fn retire_serialization_basis(
        &mut self,
        id: &SerializationBasisId,
        c: &SettingsExecutionContextV1,
        f: &SettingsBasisControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let result = (|| {
            if !self.serialization_basis_control(id, c, f)
                || f.retention_released != Some(true)
                || f.delivery_drained != Some(true)
            {
                return Err(Error::Unavailable);
            }
            let BasisPool::Active(p) = &mut self.serialization_bases else {
                return Err(Error::Unavailable);
            };
            self.retention
                .validate(&p.reservation)
                .map_err(parent_error)?;
            p.rows[id.slot] = None;
            Ok(())
        })();
        self.serialization_basis_result(r, Boundary::SerializationBasisRetirement, result)
            .is_ok()
    }
    pub fn retire_serialization_bases(
        &mut self,
        f: &SerializationPoolControlV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = (|| {
            if f.owner_control_allowed != Some(true)
                || f.retention_released != Some(true)
                || f.delivery_drained != Some(true)
            {
                return Err(Error::Unavailable);
            }
            let BasisPool::Active(p) = &mut self.serialization_bases else {
                return Err(Error::Unavailable);
            };
            if p.rows.iter().any(Option::is_some) {
                return Err(Error::Unavailable);
            }
            let issued = p.rows.len();
            self.retention
                .release(&mut p.reservation)
                .map_err(parent_error)?;
            self.serialization_bases = BasisPool::Closed { issued };
            Ok(())
        })();
        self.serialization_basis_result(r, Boundary::SerializationPoolRetirement, result)
    }
}
