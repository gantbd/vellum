//! Conditional catalog authority. No provider capability, execution or wire codec.
use super::*;
mod shutdown_inventory;
#[cfg(test)]
mod tests;
pub const NATIVE_REGISTRATION_BASIS_PROFILE_V1: &str = "native-registration-basis/1";

#[derive(Clone, PartialEq, Eq)]
pub struct ResourceRegistrationBasisScopeV1 {
    pub authority: String,
    pub engine: String,
    pub recipient: String,
    pub purpose: String,
    pub destination: String,
    pub policy: String,
    pub continuity: String,
    pub profile: String,
}
impl std::fmt::Debug for ResourceRegistrationBasisScopeV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RegistrationBasisScope(<redacted>)")
    }
}
impl ResourceRegistrationBasisScopeV1 {
    fn labels(&self) -> [&str; 8] {
        [
            &self.authority,
            &self.engine,
            &self.recipient,
            &self.purpose,
            &self.destination,
            &self.policy,
            &self.continuity,
            &self.profile,
        ]
    }
    fn bytes(&self, l: ResourceRegistrationBasisLimitsV1) -> Result<usize> {
        if self.profile != NATIVE_REGISTRATION_BASIS_PROFILE_V1 {
            return Err(ResourceError::Unavailable);
        }
        self.labels()
            .into_iter()
            .try_fold(size_of::<Self>(), |n, s| {
                if s.is_empty() || s.len() > l.max_tag_bytes {
                    return Err(ResourceError::Malformed);
                }
                add(n, s.len())
            })
            .and_then(|n| bounded(n, l.max_scope_bytes))
    }
    pub(in crate::coordinator) fn copy(&self) -> Result<Self> {
        Ok(Self {
            authority: text(&self.authority)?,
            engine: text(&self.engine)?,
            recipient: text(&self.recipient)?,
            purpose: text(&self.purpose)?,
            destination: text(&self.destination)?,
            policy: text(&self.policy)?,
            continuity: text(&self.continuity)?,
            profile: text(&self.profile)?,
        })
    }
    pub(in crate::coordinator) fn native_bytes(&self) -> Result<usize> {
        self.labels()
            .into_iter()
            .try_fold(size_of::<Self>(), |n, v| add(n, v.len()))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceRegistrationBasisLimitsV1 {
    pub max_bases: usize,
    pub max_issued: usize,
    /// Actual current native semantic-root logical bytes, not a held root copy.
    pub max_root_bytes: usize,
    pub max_profile_bytes: usize,
    pub max_scope_bytes: usize,
    pub max_tag_bytes: usize,
    pub max_slot_bytes: usize,
    pub max_basis_bytes: usize,
    pub max_use_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_held_bytes: usize,
}
impl ResourceRegistrationBasisLimitsV1 {
    /// Conservative logical envelope; not allocator/RSS or wire accounting.
    pub fn required_pool_bytes(self) -> Result<usize> {
        if self.max_bases > 0 && self.max_basis_bytes < size_of::<Record>() {
            return Err(ResourceError::Limit);
        }
        let per = add(
            add(
                self.max_basis_bytes
                    .checked_mul(2)
                    .ok_or(ResourceError::Limit)?,
                self.max_use_bytes,
            )?,
            self.max_reply_bytes
                .checked_mul(2)
                .ok_or(ResourceError::Limit)?,
        )?;
        let n = add(
            self.max_profile_bytes,
            self.max_bases
                .checked_mul(per)
                .ok_or(ResourceError::Limit)?,
        )?;
        bounded(n, self.max_held_bytes)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceRegistrationConditionV1 {
    Absent,
    Present,
}
#[derive(Clone)]
pub struct ResourceRegistrationBasisIdV1 {
    owner: Arc<()>,
    marker: Arc<()>,
}
impl PartialEq for ResourceRegistrationBasisIdV1 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && Arc::ptr_eq(&self.marker, &other.marker)
    }
}
impl Eq for ResourceRegistrationBasisIdV1 {}
impl std::fmt::Debug for ResourceRegistrationBasisIdV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RegistrationBasis(<opaque>)")
    }
}
#[derive(Debug)]
pub struct ResourceRegistrationBasisCaptureV1 {
    pub basis: ResourceRegistrationBasisIdV1,
    pub condition: ResourceRegistrationConditionV1,
}
/// Privileged host-only product. Not an ordinary recipient disclosure endpoint,
/// grant, mutation plan or certificate; final publication must re-resolve.
pub struct ResolvedResourceRegistrationBasisV1 {
    pub address: ResourceCatalogAddressV1,
    pub expected: CatalogExpectationV1,
    pub row: Option<ResourceCatalogRowV1>,
}
impl std::fmt::Debug for ResolvedResourceRegistrationBasisV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResolvedRegistrationBasis(<host-only>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceRegistrationBasisUsageV1 {
    pub live: usize,
    pub issued_lifetime: usize,
    pub reserved_bytes: usize,
    pub declared_bytes: usize,
}
#[derive(Clone, Copy, Default)]
pub struct TrustedResourceRegistrationBasisSetupV1 {
    pub owner_control_allowed: Option<bool>,
    pub resources_ready: Option<bool>,
    pub preparation_reserved: Option<bool>,
}
/// Authenticated host facts, never requester attestations. Every current use
/// needs exact true, independently of captured permission or raw host reads.
#[derive(Clone, Copy)]
pub struct TrustedResourceRegistrationBasisFactsV1<'a> {
    pub scope: &'a ResourceRegistrationBasisScopeV1,
    pub native_profile: ResourceCatalogLimitsV1,
    pub authorized: Option<bool>,
    pub scope_current: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub registration_allowed: Option<bool>,
    pub identity_allowed: Option<bool>,
    pub registry_coverage_complete: Option<bool>,
    pub resources_ready: Option<bool>,
    pub retained: Option<bool>,
    pub reply_ready: Option<bool>,
    pub basis_allowed: Option<bool>,
    pub presence_observable: Option<bool>,
    pub equality_observable: Option<bool>,
    pub slot_allowed: Option<bool>,
    pub owner_control_allowed: Option<bool>,
    pub preparation_reserved: Option<bool>,
    pub interpretation_current: Option<bool>,
    pub host_basis_allowed: Option<bool>,
}
impl TrustedResourceRegistrationBasisFactsV1<'_> {
    fn current(&self) -> Result<()> {
        if [
            self.authorized,
            self.scope_current,
            self.continuous,
            self.live,
            self.not_stopped,
            self.registration_allowed,
            self.identity_allowed,
            self.registry_coverage_complete,
            self.resources_ready,
            self.retained,
            self.reply_ready,
            self.basis_allowed,
            self.presence_observable,
            self.equality_observable,
            self.slot_allowed,
            self.owner_control_allowed,
            self.preparation_reserved,
            self.interpretation_current,
        ]
        .into_iter()
        .all(|v| v == Some(true))
        {
            Ok(())
        } else {
            Err(ResourceError::Unavailable)
        }
    }
}
pub struct TrustedResourceRegistrationBasisControlV1 {
    pub owner_control_allowed: Option<bool>,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
}
impl TrustedResourceRegistrationBasisControlV1 {
    fn current(&self) -> Result<()> {
        if [
            self.owner_control_allowed,
            self.retention_released,
            self.delivery_drained,
        ]
        .into_iter()
        .all(|v| v == Some(true))
        {
            Ok(())
        } else {
            Err(ResourceError::Unavailable)
        }
    }
}
#[derive(Debug, Default)]
pub(in crate::coordinator) enum State {
    #[default]
    Unconfigured,
    Active(Box<Store>),
    Closed,
}
pub(in crate::coordinator) struct Store {
    issuer: Arc<()>,
    authority: String,
    engine: String,
    native: ResourceCatalogLimitsV1,
    limits: ResourceRegistrationBasisLimitsV1,
    records: Vec<Record>,
    issued: usize,
    reservation: retention::Reservation,
}
impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RegistrationBasisStore(<redacted>)")
    }
}
struct Record {
    marker: Arc<()>,
    scope: ResourceRegistrationBasisScopeV1,
    address: ResourceCatalogAddressV1,
    row: Option<ResourceCatalogRowV1>,
}
impl Record {
    fn condition(&self) -> ResourceRegistrationConditionV1 {
        if self.row.is_some() {
            ResourceRegistrationConditionV1::Present
        } else {
            ResourceRegistrationConditionV1::Absent
        }
    }
    fn use_bytes(
        address: &ResourceCatalogAddressV1,
        row: Option<&ResourceCatalogRowV1>,
    ) -> Result<usize> {
        let n = add(
            size_of::<ResolvedResourceRegistrationBasisV1>(),
            address.payload()?,
        )?;
        match row {
            Some(row) => add(n, row.payload()?),
            None => Ok(n),
        }
    }
    fn bytes(
        scope_bytes: usize,
        address: &ResourceCatalogAddressV1,
        row: Option<&ResourceCatalogRowV1>,
    ) -> Result<usize> {
        let n = add(add(size_of::<Record>(), scope_bytes)?, address.payload()?)?;
        match row {
            Some(row) => add(n, row.payload()?),
            None => Ok(n),
        }
    }
}
fn text(s: &str) -> Result<String> {
    let mut out = String::new();
    out.try_reserve_exact(s.len())
        .map_err(|_| ResourceError::Capacity)?;
    out.push_str(s);
    Ok(out)
}
fn bounded(n: usize, limit: usize) -> Result<usize> {
    if n <= limit {
        Ok(n)
    } else {
        Err(ResourceError::Limit)
    }
}
fn parent(e: RetentionErrorV1) -> ResourceError {
    match e {
        RetentionErrorV1::Faulted => ResourceError::Faulted,
        RetentionErrorV1::Limit => ResourceError::Limit,
        RetentionErrorV1::Unavailable => ResourceError::Unavailable,
    }
}
fn observe<T>(
    r: &mut Recorder,
    boundary: Boundary,
    value: &Result<T>,
    elapsed: Option<std::time::Duration>,
) {
    r.record(Observation {
        boundary,
        disposition: match value {
            Ok(_) => Disposition::Completed,
            Err(ResourceError::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time: elapsed,
    });
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn registration_basis_issuer_v1(
        &self,
        c: &ResourceRegistrationBasisScopeV1,
        f: &TrustedResourceRegistrationBasisFactsV1<'_>,
    ) -> Result<Arc<()>> {
        if f.host_basis_allowed != Some(true) {
            return Err(ResourceError::Unavailable);
        }
        let (s, _) = self.registration_basis_current(c, f)?;
        Ok(s.issuer.clone())
    }
    /// Borrowed first-party lowering only, under the exclusive owner's lifetime.
    /// No authority escapes through an ordinary recipient reply.
    pub(in crate::coordinator) fn registration_basis_slot_v1<'a>(
        &'a self,
        id: &ResourceRegistrationBasisIdV1,
        c: &ResourceRegistrationBasisScopeV1,
        f: &TrustedResourceRegistrationBasisFactsV1<'_>,
    ) -> Result<(&'a ResourceCatalogAddressV1, CatalogExpectationV1)> {
        if f.host_basis_allowed != Some(true) {
            return Err(ResourceError::Unavailable);
        }
        let (s, p) = self.registration_basis_record(id, c, f)?;
        bounded(
            Record::use_bytes(&p.address, p.row.as_ref())?,
            s.limits.max_use_bytes,
        )?;
        Ok((
            &p.address,
            match &p.row {
                Some(row) => CatalogExpectationV1::Present {
                    generation: row.generation,
                },
                None => CatalogExpectationV1::Absent,
            },
        ))
    }
    fn registration_basis_store(&self) -> Result<&Store> {
        match &self.registration_bases {
            State::Active(s) => {
                self.retention.validate(&s.reservation).map_err(parent)?;
                if s.records.len() > s.limits.max_bases
                    || s.records.len() > s.issued
                    || s.issued > s.limits.max_issued
                {
                    return Err(ResourceError::Faulted);
                }
                Ok(s)
            }
            _ => Err(ResourceError::Unavailable),
        }
    }
    fn registration_basis_current<'a>(
        &'a self,
        c: &ResourceRegistrationBasisScopeV1,
        f: &TrustedResourceRegistrationBasisFactsV1<'_>,
    ) -> Result<(&'a Store, &'a Arc<Root>)> {
        self.ensure_operational()
            .map_err(|_| ResourceError::Faulted)?;
        let s = self.registration_basis_store()?;
        c.bytes(s.limits)?;
        if c != f.scope
            || c.authority != s.authority
            || c.engine != s.engine
            || f.native_profile != s.native
        {
            return Err(ResourceError::Unavailable);
        }
        f.current()?;
        let (root, native) = self.resources.catalog.current()?;
        if native != s.native {
            return Err(ResourceError::Unavailable);
        }
        let bytes = self
            .measure_semantic_root()
            .map_err(|e| match e {
                RootAccountingErrorV1::Faulted => ResourceError::Faulted,
                _ => ResourceError::Limit,
            })?
            .total_bytes;
        bounded(bytes, s.limits.max_root_bytes)?;
        Ok((s, root))
    }
    fn registration_basis_record<'a>(
        &'a self,
        id: &ResourceRegistrationBasisIdV1,
        c: &ResourceRegistrationBasisScopeV1,
        f: &TrustedResourceRegistrationBasisFactsV1<'_>,
    ) -> Result<(&'a Store, &'a Record)> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return Err(ResourceError::Unavailable);
        }
        let (s, root) = self.registration_basis_current(c, f)?;
        let p = s
            .records
            .iter()
            .find(|p| Arc::ptr_eq(&p.marker, &id.marker))
            .ok_or(ResourceError::Unavailable)?;
        if p.scope != *c {
            return Err(ResourceError::Unavailable);
        }
        let row = root.rows.iter().find(|row| row.address == p.address);
        if row != p.row.as_ref() {
            return Err(ResourceError::Unavailable);
        }
        Ok((s, p))
    }
    pub fn configure_registration_bases_v1(
        &mut self,
        c: &ResourceRegistrationBasisScopeV1,
        native: ResourceCatalogLimitsV1,
        l: ResourceRegistrationBasisLimitsV1,
        f: &TrustedResourceRegistrationBasisSetupV1,
        r: &mut Recorder,
    ) -> Result<bool> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational()
                .map_err(|_| ResourceError::Faulted)?;
            if [
                f.owner_control_allowed,
                f.resources_ready,
                f.preparation_reserved,
            ]
            .into_iter()
            .any(|v| v != Some(true))
            {
                return Err(ResourceError::Unavailable);
            }
            c.bytes(l)?;
            l.required_pool_bytes()?;
            let (_, actual) = self.resources.catalog.current()?;
            if actual != native {
                return Err(ResourceError::Unavailable);
            }
            match &self.registration_bases {
                State::Active(_) => {
                    let s = self.registration_basis_store()?;
                    return if s.authority == c.authority
                        && s.engine == c.engine
                        && s.native == native
                        && s.limits == l
                    {
                        Ok(false)
                    } else {
                        Err(ResourceError::Unavailable)
                    };
                }
                State::Closed => return Err(ResourceError::Unavailable),
                State::Unconfigured => (),
            }
            self.ensure_admission_unfenced().map_err(owner_error)?;
            let profile_bytes = add(
                add(
                    add(size_of::<State>(), size_of::<Store>())?,
                    c.authority.len(),
                )?,
                c.engine.len(),
            )?;
            bounded(profile_bytes, l.max_profile_bytes)?;
            self.retention.check(l.max_held_bytes).map_err(parent)?;
            let authority = text(&c.authority)?;
            let engine = text(&c.engine)?;
            let mut records = Vec::new();
            records
                .try_reserve_exact(l.max_bases)
                .map_err(|_| ResourceError::Capacity)?;
            let reservation = self.retention.reserve(l.max_held_bytes).map_err(parent)?;
            self.registration_bases = State::Active(Box::new(Store {
                issuer: Arc::new(()),
                authority,
                engine,
                native,
                limits: l,
                records,
                issued: 0,
                reservation,
            }));
            Ok(true)
        })();
        observe(
            r,
            Boundary::RegistrationBasisConfiguration,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn capture_registration_basis_v1(
        &mut self,
        c: &ResourceRegistrationBasisScopeV1,
        address: &ResourceCatalogAddressV1,
        f: &TrustedResourceRegistrationBasisFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResourceRegistrationBasisCaptureV1> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let (s, root) = self.registration_basis_current(c, f)?;
            let l = s.limits;
            if !address.valid(ResourceCatalogLimitsV1 {
                max_tag_bytes: l.max_tag_bytes.min(s.native.max_tag_bytes),
                ..s.native
            }) {
                return Err(ResourceError::Malformed);
            }
            bounded(
                add(size_of::<ResourceCatalogAddressV1>(), address.payload()?)?,
                l.max_slot_bytes,
            )?;
            let row = root.rows.iter().find(|row| row.address == *address);
            bounded(Record::bytes(c.bytes(l)?, address, row)?, l.max_basis_bytes)?;
            bounded(Record::use_bytes(address, row)?, l.max_use_bytes)?;
            bounded(
                size_of::<ResourceRegistrationBasisCaptureV1>(),
                l.max_reply_bytes,
            )?;
            if let Some(p) = s
                .records
                .iter()
                .find(|p| p.scope == *c && p.address == *address && p.row.as_ref() == row)
            {
                return Ok(ResourceRegistrationBasisCaptureV1 {
                    basis: ResourceRegistrationBasisIdV1 {
                        owner: self.namespace.clone(),
                        marker: p.marker.clone(),
                    },
                    condition: p.condition(),
                });
            }
            // Matching original records above are service, not renewed
            // issuance. A retired record cannot be recreated across the fence.
            self.ensure_admission_unfenced().map_err(owner_error)?;
            if s.records.len() >= l.max_bases || s.issued >= l.max_issued {
                return Err(ResourceError::Limit);
            }
            let p = Record {
                marker: Arc::new(()),
                scope: c.copy()?,
                address: address.copy()?,
                row: row.map(ResourceCatalogRowV1::copy).transpose()?,
            };
            let reply = ResourceRegistrationBasisCaptureV1 {
                basis: ResourceRegistrationBasisIdV1 {
                    owner: self.namespace.clone(),
                    marker: p.marker.clone(),
                },
                condition: p.condition(),
            };
            let State::Active(s) = &mut self.registration_bases else {
                return Err(ResourceError::Faulted);
            };
            s.records.push(p);
            s.issued += 1;
            Ok(reply)
        })();
        observe(
            r,
            Boundary::RegistrationBasisCapture,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn inspect_registration_basis_v1(
        &self,
        id: &ResourceRegistrationBasisIdV1,
        c: &ResourceRegistrationBasisScopeV1,
        f: &TrustedResourceRegistrationBasisFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResourceRegistrationConditionV1> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let (s, p) = self.registration_basis_record(id, c, f)?;
            bounded(
                size_of::<ResourceRegistrationConditionV1>(),
                s.limits.max_reply_bytes,
            )?;
            Ok(p.condition())
        })();
        observe(
            r,
            Boundary::RegistrationBasisInspection,
            &result,
            timing.finish(),
        );
        result
    }
    /// Native first-party resolver only. These raw fields must never be forwarded
    /// through ordinary recipient routes. This product cannot commit a mutation.
    pub fn resolve_registration_basis_for_host_v1(
        &self,
        id: &ResourceRegistrationBasisIdV1,
        c: &ResourceRegistrationBasisScopeV1,
        f: &TrustedResourceRegistrationBasisFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResolvedResourceRegistrationBasisV1> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if f.host_basis_allowed != Some(true) {
                return Err(ResourceError::Unavailable);
            }
            let (s, p) = self.registration_basis_record(id, c, f)?;
            bounded(
                Record::use_bytes(&p.address, p.row.as_ref())?,
                s.limits.max_use_bytes,
            )?;
            Ok(ResolvedResourceRegistrationBasisV1 {
                address: p.address.copy()?,
                expected: match &p.row {
                    Some(row) => CatalogExpectationV1::Present {
                        generation: row.generation,
                    },
                    None => CatalogExpectationV1::Absent,
                },
                row: p.row.as_ref().map(ResourceCatalogRowV1::copy).transpose()?,
            })
        })();
        observe(
            r,
            Boundary::RegistrationBasisHostResolution,
            &result,
            timing.finish(),
        );
        result
    }
    /// Protected owner control: no content grants, current profile or healthy
    /// domain needed. The original opaque handle still must belong to this owner.
    pub fn retire_registration_basis_v1(
        &mut self,
        id: &ResourceRegistrationBasisIdV1,
        f: &TrustedResourceRegistrationBasisControlV1,
        r: &mut Recorder,
    ) -> Result<()> {
        let timing = r.begin_service_timing();
        let result = (|| {
            f.current()?;
            if !Arc::ptr_eq(&id.owner, &self.namespace) {
                return Err(ResourceError::Unavailable);
            }
            let s = self.registration_basis_store()?;
            let index = s
                .records
                .iter()
                .position(|p| Arc::ptr_eq(&p.marker, &id.marker))
                .ok_or(ResourceError::Unavailable)?;
            let State::Active(s) = &mut self.registration_bases else {
                return Err(ResourceError::Faulted);
            };
            s.records.remove(index);
            Ok(())
        })();
        observe(
            r,
            Boundary::RegistrationBasisRetirement,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn retire_registration_basis_issuer_v1(
        &mut self,
        f: &TrustedResourceRegistrationBasisControlV1,
        r: &mut Recorder,
    ) -> Result<()> {
        let timing = r.begin_service_timing();
        let result = (|| {
            f.current()?;
            if !self.registration_basis_store()?.records.is_empty() {
                return Err(ResourceError::Unavailable);
            }
            let State::Active(s) = &mut self.registration_bases else {
                return Err(ResourceError::Faulted);
            };
            self.retention.release(&mut s.reservation).map_err(parent)?;
            self.registration_bases = State::Closed;
            Ok(())
        })();
        observe(
            r,
            Boundary::RegistrationBasisRetirement,
            &result,
            timing.finish(),
        );
        result
    }
    /// Trusted-owner retention gauges, never public metric label values.
    pub fn registration_basis_usage_v1(&self) -> Result<ResourceRegistrationBasisUsageV1> {
        let s = self.registration_basis_store()?;
        Ok(ResourceRegistrationBasisUsageV1 {
            live: s.records.len(),
            issued_lifetime: s.issued,
            reserved_bytes: s.limits.max_held_bytes,
            declared_bytes: s.limits.required_pool_bytes()?,
        })
    }
}
