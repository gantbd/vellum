//! Scoped adapter over the one existing registration job and native publisher.
use super::*;
pub(in crate::coordinator::invocation::registration) mod arguments;
pub use arguments::*;
#[cfg(test)]
mod tests;
pub const SCOPED_RESOURCE_REGISTRATION_PROFILE_V1: &str = "scoped-resource-registration/1";
#[derive(Debug, PartialEq, Eq)]
pub struct ScopedResourceRegistrationChangeV1 {
    pub basis: usize,
    pub action: ResourceCatalogActionV1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopedResourceRegistrationRowV1 {
    pub basis: usize,
    pub state: ResourceRegistrationConditionV1,
}
#[derive(Debug, PartialEq, Eq)]
pub struct ScopedResourceRegistrationResultV1 {
    pub invocation: InvocationId,
    pub status: ResourceCatalogStatusV1,
    pub registrations: Vec<ScopedResourceRegistrationRowV1>,
}
pub struct TrustedScopedResourceRegistrationInvocationV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub scope: &'a ResourceRegistrationBasisScopeV1,
    pub basis: &'a TrustedResourceRegistrationBasisFactsV1<'a>,
    pub native: &'a TrustedResourceCatalogFactsV1,
    pub max_result_bytes: usize,
    pub interpretation_current: Option<bool>,
    pub basis_scope_current: Option<bool>,
    pub scoped_result_allowed: Option<bool>,
}
pub struct TrustedScopedResourceRegistrationResultV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub scope: &'a ResourceRegistrationBasisScopeV1,
    pub basis: &'a TrustedResourceRegistrationBasisFactsV1<'a>,
    pub max_result_bytes: usize,
    pub interpretation_current: Option<bool>,
    pub basis_scope_current: Option<bool>,
    pub outcome_readable: Option<bool>,
    pub scoped_result_readable: Option<bool>,
    pub scoped_result_context_current: Option<bool>,
    pub scoped_result_allowed: Option<bool>,
    pub reply_ready: Option<bool>,
    pub resources_ready: Option<bool>,
}
pub(super) struct Capture {
    issuer: Arc<()>,
    pub(super) scope: ResourceRegistrationBasisScopeV1,
    bases: Vec<ResourceRegistrationBasisIdV1>,
    // The original actions/configurations are shared losslessly with the enclosing
    // native request, not copied into a second contradictory request tree.
    pub(super) indices: Vec<usize>,
}
impl Capture {
    pub(super) fn shape_intact(&self, count: usize) -> bool {
        self.indices.len() == count
            && self.bases.len() == count
            && self
                .indices
                .iter()
                .enumerate()
                .all(|(i, n)| *n < count && !self.indices[..i].contains(n))
    }
}
pub(super) fn receipt_bytes(count: usize) -> Result<usize, InvocationError> {
    count
        .checked_mul(size_of::<ScopedResourceRegistrationRowV1>())
        .and_then(|n| n.checked_add(size_of::<ScopedResourceRegistrationResultV1>()))
        .ok_or(InvocationError::Limit)
}
fn add(a: usize, b: usize) -> Result<usize, ResourceError> {
    a.checked_add(b).ok_or(ResourceError::Limit)
}
fn bound(n: usize, max: usize) -> Result<usize, ResourceError> {
    if n <= max {
        Ok(n)
    } else {
        Err(ResourceError::Limit)
    }
}
fn binding(c: &SettingsExecutionContextV1, s: &ResourceRegistrationBasisScopeV1) -> bool {
    c.profile == SCOPED_RESOURCE_REGISTRATION_PROFILE_V1
        && c.authority == s.authority
        && c.engine == s.engine
        && c.recipient == s.recipient
        && c.purpose == s.purpose
        && c.destination == s.destination
        && c.continuity == s.continuity
}
fn context_bytes(c: &SettingsExecutionContextV1) -> Result<usize, ResourceError> {
    super::super::super::settings::basis::labels(c)
        .into_iter()
        .try_fold(0usize, |n, label| {
            if label.is_empty() {
                return Err(ResourceError::Malformed);
            }
            add(n, label.len())
        })
}
fn retained_prefix(
    c: &SettingsExecutionContextV1,
    scope: &ResourceRegistrationBasisScopeV1,
    count: usize,
) -> Result<usize, ResourceError> {
    let extra = add(
        add(
            size_of::<Capture>(),
            scope
                .native_bytes()?
                .checked_sub(size_of::<ResourceRegistrationBasisScopeV1>())
                .ok_or(ResourceError::Faulted)?,
        )?,
        count
            .checked_mul(size_of::<ResourceRegistrationBasisIdV1>() + size_of::<usize>())
            .ok_or(ResourceError::Limit)?,
    )?;
    add(
        add(
            size_of::<super::Capture>() - size_of::<CatalogRequestCapture>(),
            context_bytes(c)?,
        )?,
        extra,
    )
}
fn input_bytes(
    c: &SettingsExecutionContextV1,
    scope: &ResourceRegistrationBasisScopeV1,
    changes: &[ScopedResourceRegistrationChangeV1],
) -> Result<usize, ResourceError> {
    let mut n = add(
        add(
            add(
                size_of::<Vec<ScopedResourceRegistrationChangeV1>>(),
                size_of::<Vec<ResourceRegistrationBasisIdV1>>(),
            )?,
            scope.native_bytes()?,
        )?,
        add(size_of::<SettingsExecutionContextV1>(), context_bytes(c)?)?,
    )?;
    n = add(
        n,
        add(
            size_of::<InvocationId>(),
            size_of::<ResourceCatalogLimitsV1>(),
        )?,
    )?;
    n = add(
        n,
        changes
            .len()
            .checked_mul(
                size_of::<ScopedResourceRegistrationChangeV1>()
                    + size_of::<ResourceRegistrationBasisIdV1>(),
            )
            .ok_or(ResourceError::Limit)?,
    )?;
    for v in changes {
        if let ResourceCatalogActionV1::Install(config) = &v.action {
            n = add(n, config.payload()?)?;
        }
    }
    Ok(n)
}
// Closed borrowed views. No caller callback or executable input participates.
enum Input<'a> {
    Submitted(&'a [ScopedResourceRegistrationChangeV1]),
    Retained(&'a Capture, &'a [ResourceCatalogChangeV1]),
}
impl Input<'_> {
    fn len(&self) -> usize {
        match self {
            Self::Submitted(v) => v.len(),
            Self::Retained(v, _) => v.indices.len(),
        }
    }
    fn at(&self, i: usize) -> (usize, &ResourceCatalogActionV1) {
        match self {
            Self::Submitted(v) => (v[i].basis, &v[i].action),
            Self::Retained(c, v) => (c.indices[i], &v[i].action),
        }
    }
}
impl SourceCoordinator {
    fn scoped_registration_issuer(
        &self,
        c: &SettingsExecutionContextV1,
        f: &TrustedScopedResourceRegistrationInvocationV1<'_>,
    ) -> Result<Arc<()>, ResourceError> {
        if f.context != c
            || !binding(c, f.scope)
            || f.basis_scope_current != Some(true)
            || f.interpretation_current != Some(true)
        {
            return Err(ResourceError::Unavailable);
        }
        self.registration_basis_issuer_v1(f.scope, f.basis)
    }
    fn lower_scoped_registration(
        &self,
        input: Input<'_>,
        bases: &[ResourceRegistrationBasisIdV1],
        f: &TrustedScopedResourceRegistrationInvocationV1<'_>,
        budget: usize,
    ) -> Result<CatalogRequestCapture, ResourceError> {
        let count = input.len();
        let p = f.basis.native_profile;
        if count != bases.len() || count > p.max_relations {
            return Err(ResourceError::Malformed);
        }
        let mut native_bytes = size_of::<CatalogRequestCapture>();
        // Preflight every reference, permutation, table/configuration and complete
        // native payload before copying any caller-sized data.
        for i in 0..count {
            let (index, action) = input.at(i);
            if index >= count || (0..i).any(|j| input.at(j).0 == index) {
                return Err(ResourceError::Malformed);
            }
            let (slot, _) = self.registration_basis_slot_v1(&bases[index], f.scope, f.basis)?;
            // Different handles for the same typed slot must not hide a duplicate.
            for j in 0..i {
                let (other, _) =
                    self.registration_basis_slot_v1(&bases[input.at(j).0], f.scope, f.basis)?;
                if slot == other {
                    return Err(ResourceError::Malformed);
                }
            }
            native_bytes = add(
                add(native_bytes, size_of::<ResourceCatalogChangeV1>())?,
                slot.payload()?,
            )?;
            if let ResourceCatalogActionV1::Install(config) = action {
                if !config.valid(slot.table, p) {
                    return Err(ResourceError::Malformed);
                }
                native_bytes = add(native_bytes, config.payload()?)?;
            }
        }
        bound(native_bytes, budget)?;
        bound(
            native_bytes
                .checked_sub(
                    size_of::<CatalogRequestCapture>() - size_of::<Vec<ResourceCatalogChangeV1>>(),
                )
                .ok_or(ResourceError::Faulted)?,
            p.max_request_bytes,
        )?;
        let mut changes = Vec::new();
        changes
            .try_reserve_exact(count)
            .map_err(|_| ResourceError::Capacity)?;
        for i in 0..count {
            let (index, action) = input.at(i);
            let (address, expected) =
                self.registration_basis_slot_v1(&bases[index], f.scope, f.basis)?;
            changes.push(ResourceCatalogChangeV1 {
                address: address.copy()?,
                expected,
                action: match action {
                    ResourceCatalogActionV1::Remove => ResourceCatalogActionV1::Remove,
                    ResourceCatalogActionV1::Install(config) => {
                        ResourceCatalogActionV1::Install(config.copy()?)
                    }
                },
            });
        }
        if CatalogRequestCapture::bytes(&changes, p)? != native_bytes {
            return Err(ResourceError::Faulted);
        }
        Ok(CatalogRequestCapture {
            profile: p,
            changes,
        })
    }
    pub(super) fn validate_scoped_registration_capture(
        &self,
        original: &super::Capture,
        c: &SettingsExecutionContextV1,
        f: &TrustedScopedResourceRegistrationInvocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let Some(scoped) = &original.scoped else {
                return Err(ResourceError::Faulted);
            };
            if !scoped.shape_intact(original.native.changes.len()) {
                return Err(ResourceError::Faulted);
            }
            if scoped.scope != *f.scope {
                return Err(ResourceError::Unavailable);
            }
            let issuer = self.scoped_registration_issuer(c, f)?;
            if !Arc::ptr_eq(&issuer, &scoped.issuer) {
                return Err(ResourceError::Unavailable);
            }
            let current = self.lower_scoped_registration(
                Input::Retained(scoped, &original.native.changes),
                &scoped.bases,
                f,
                self.invocations.limits.max_intent_bytes,
            )?;
            if current.profile != original.native.profile
                || current.changes != original.native.changes
            {
                return Err(ResourceError::Unavailable);
            }
            Ok(())
        })();
        record(
            r,
            Boundary::ScopedRegistrationValidation,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn submit_scoped_resource_registration_v1(
        &mut self,
        id: &InvocationId,
        changes: &[ScopedResourceRegistrationChangeV1],
        bases: &[ResourceRegistrationBasisIdV1],
        c: &SettingsExecutionContextV1,
        f: &TrustedScopedResourceRegistrationInvocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        let timing = r.begin_service_timing();
        let repeated = self.invocation_slot(id).is_ok_and(|s| s.intent.is_some());
        let result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            self.invocation_slot(id)?;
            let issuer = self.scoped_registration_issuer(c, f).map_err(error)?;
            let count = changes.len();
            if count != bases.len() || count > f.basis.native_profile.max_relations {
                return Err(InvocationError::MalformedIntent);
            }
            let max = self.invocations.limits.max_intent_bytes;
            let prefix = retained_prefix(c, f.scope, count).map_err(error)?;
            let budget = max.checked_sub(prefix).ok_or(InvocationError::Limit)?;
            // A separately checked ingress envelope includes the original scoped
            // descriptors, scope and actual reference array before normalization.
            if input_bytes(c, f.scope, changes).map_err(error)? > max {
                return Err(InvocationError::Limit);
            }
            let native = self
                .lower_scoped_registration(Input::Submitted(changes), bases, f, budget)
                .map_err(error)?;
            // Scoped re-preparation is current service, unlike an inert native
            // replay check: all native current prerequisites still apply.
            let p = self
                .prepare_resource_catalog_v1(&native.changes, f.native, r)
                .map_err(error)?;
            drop(p);
            let slot = self.invocation_slot(id)?;
            if let Some(intent) = &slot.intent {
                let BoundIntent::ResourceCatalog(original) = intent else {
                    return Err(InvocationError::IntentMismatch);
                };
                let Some(scoped) = &original.scoped else {
                    return Err(InvocationError::IntentMismatch);
                };
                if original.invocation != *id || !Arc::ptr_eq(&original.owner, &self.namespace) {
                    return Err(InvocationError::Faulted);
                }
                if original.context != *c
                    || original.native.profile != native.profile
                    || original.native.changes != native.changes
                    || original.max_result_bytes != f.max_result_bytes
                    || scoped.scope != *f.scope
                    || !Arc::ptr_eq(&scoped.issuer, &issuer)
                    || scoped.bases != bases
                    || scoped
                        .indices
                        .iter()
                        .copied()
                        .ne(changes.iter().map(|v| v.basis))
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let cost = receipt_bytes(count)?;
            if cost > f.max_result_bytes || cost > max {
                return Err(InvocationError::Limit);
            }
            let mut references = Vec::new();
            references
                .try_reserve_exact(count)
                .map_err(|_| InvocationError::Capacity)?;
            references.extend_from_slice(bases);
            let mut indices = Vec::new();
            indices
                .try_reserve_exact(count)
                .map_err(|_| InvocationError::Capacity)?;
            indices.extend(changes.iter().map(|v| v.basis));
            let scoped = Arc::new(Capture {
                issuer,
                scope: f.scope.copy().map_err(error)?,
                bases: references,
                indices,
            });
            let capture = Arc::new(super::Capture {
                owner: self.namespace.clone(),
                context: super::super::super::settings::basis::copy_context(c)
                    .map_err(|_| InvocationError::Capacity)?,
                invocation: id.clone(),
                native,
                max_result_bytes: f.max_result_bytes,
                result_ceiling: f.scoped_result_allowed == Some(true),
                scoped: Some(scoped),
            });
            self.install_invocation_admission(
                id,
                BoundIntent::ResourceCatalog(capture.clone()),
                Phase::ResourceCatalogPending(capture),
            );
            Ok(InvocationState::Pending)
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        let elapsed = timing.finish();
        capture_timed(
            r,
            if repeated {
                Boundary::InvocationReplay
            } else {
                Boundary::InvocationAdmission
            },
            &result,
            elapsed,
        );
        capture_timed(r, Boundary::ScopedRegistrationPreparation, &result, elapsed);
        result
    }
    pub fn run_scoped_resource_registration_v1(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        f: &TrustedScopedResourceRegistrationInvocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let terminal = self
            .invocation_slot(id)
            .is_ok_and(|s| matches!(s.phase, Phase::Terminal(_)));
        let n = TrustedResourceRegistrationInvocationV1 {
            context: f.context,
            profile: f.basis.native_profile,
            max_result_bytes: f.max_result_bytes,
            interpretation_current: f.interpretation_current,
            host_result_allowed: None,
            native: f.native,
        };
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.run_registration_inner(id, c, &n, Some(f), r)
        }))
        .unwrap_or_else(|_| {
            self.faulted = true;
            Err(InvocationError::Faulted)
        });
        capture_timed(
            r,
            if terminal {
                Boundary::InvocationReplay
            } else {
                Boundary::ResourceRegistrationService
            },
            &result,
            timing.finish(),
        );
        result
    }
    pub fn retained_scoped_resource_registration_result_v1(
        &self,
        id: &InvocationId,
        f: &TrustedScopedResourceRegistrationResultV1<'_>,
        r: &mut Recorder,
    ) -> Result<ScopedResourceRegistrationResultV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if self.faulted
                || [
                    f.interpretation_current,
                    f.basis_scope_current,
                    f.outcome_readable,
                    f.scoped_result_readable,
                    f.scoped_result_context_current,
                    f.scoped_result_allowed,
                    f.reply_ready,
                    f.resources_ready,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(InvocationError::Unavailable);
            }
            let slot = self.invocation_slot(id)?;
            let Some(BoundIntent::ResourceCatalog(original)) = &slot.intent else {
                return Err(InvocationError::Unavailable);
            };
            let Some(scoped) = &original.scoped else {
                return Err(InvocationError::Unavailable);
            };
            if original.invocation != *id {
                return Err(InvocationError::Faulted);
            }
            if !Arc::ptr_eq(&original.owner, &self.namespace)
                || original.context != *f.context
                || !original.result_ceiling
                || scoped.scope != *f.scope
                || !binding(f.context, f.scope)
                || original.native.profile != f.basis.native_profile
            {
                return Err(InvocationError::Unavailable);
            }
            // Reauthorize current issuer/scope/root, NOT the historical row
            // condition: a successful mutation may have staled its own bases.
            let issuer = self
                .registration_basis_issuer_v1(f.scope, f.basis)
                .map_err(error)?;
            if !Arc::ptr_eq(&issuer, &scoped.issuer) {
                return Err(InvocationError::Unavailable);
            }
            let Phase::Terminal(Some(InvocationOutcome::ResourceCatalogCommitted(retained))) =
                &slot.phase
            else {
                return Err(InvocationError::OutcomeUnavailable);
            };
            if !retained.intact(original)
                || scoped.indices.len() != retained.value.registrations.len()
            {
                return Err(InvocationError::Faulted);
            }
            let cost = receipt_bytes(scoped.indices.len())?;
            if cost > original.max_result_bytes
                || cost > f.max_result_bytes
                || cost > self.invocations.limits.max_intent_bytes
            {
                return Err(InvocationError::Limit);
            }
            let mut registrations = Vec::new();
            registrations
                .try_reserve_exact(scoped.indices.len())
                .map_err(|_| InvocationError::Capacity)?;
            for (basis, row) in scoped.indices.iter().zip(&retained.value.registrations) {
                registrations.push(ScopedResourceRegistrationRowV1 {
                    basis: *basis,
                    state: if row.generation.is_some() {
                        ResourceRegistrationConditionV1::Present
                    } else {
                        ResourceRegistrationConditionV1::Absent
                    },
                });
            }
            Ok(ScopedResourceRegistrationResultV1 {
                invocation: id.clone(),
                status: retained.value.status,
                registrations,
            })
        })();
        capture_timed(
            r,
            Boundary::ScopedRegistrationResult,
            &result,
            timing.finish(),
        );
        result
    }
}
fn record<T>(
    r: &mut Recorder,
    boundary: Boundary,
    result: &Result<T, ResourceError>,
    elapsed: Option<std::time::Duration>,
) {
    r.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(ResourceError::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time: elapsed,
    });
}
