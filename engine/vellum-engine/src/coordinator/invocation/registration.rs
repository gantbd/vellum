//! Once-only catalog metadata jobs, using the existing native stage/publisher.
use super::*;
use crate::coordinator::resource::catalog::CatalogRequestCapture;
use crate::resource::ResourceError;
use std::mem::size_of;
use std::panic::{AssertUnwindSafe, catch_unwind};
mod arguments;
mod scoped;
pub use arguments::*;
#[cfg(test)]
mod tests;
pub use scoped::*;
pub const NATIVE_RESOURCE_REGISTRATION_PROFILE_V1: &str = "native-resource-registration/1";
pub struct TrustedResourceRegistrationInvocationV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub profile: ResourceCatalogLimitsV1,
    pub max_result_bytes: usize,
    pub interpretation_current: Option<bool>,
    /// Original exact-true ceiling, not permission to execute or a wire grant.
    pub host_result_allowed: Option<bool>,
    pub native: &'a TrustedResourceCatalogFactsV1,
}
pub struct TrustedResourceRegistrationResultV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub max_result_bytes: usize,
    pub outcome_readable: Option<bool>,
    pub registration_result_readable: Option<bool>,
    pub registration_result_context_current: Option<bool>,
    pub host_result_allowed: Option<bool>,
    pub reply_ready: Option<bool>,
    pub resources_ready: Option<bool>,
}
#[derive(Debug)]
pub struct ResourceRegistrationResultV1 {
    pub invocation: InvocationId,
    /// Native owner-wide stamps; trusted-host delivery only, not public encoding.
    pub result: ResourceCatalogOutcomeV1,
}
/// Unforgeable native publication product retained by the existing job outcome.
pub struct RetainedResourceRegistrationOutcomeV1 {
    origin: Arc<Capture>,
    value: Arc<ResourceCatalogOutcomeV1>,
    seal: Arc<ResourceCatalogOutcomeV1>,
}
impl PartialEq for RetainedResourceRegistrationOutcomeV1 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.origin, &other.origin)
            && self.value == other.value
            && Arc::ptr_eq(&self.seal, &other.seal)
    }
}
impl Eq for RetainedResourceRegistrationOutcomeV1 {}
impl std::fmt::Debug for RetainedResourceRegistrationOutcomeV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RetainedRegistrationOutcome(<sealed>)")
    }
}
impl RetainedResourceRegistrationOutcomeV1 {
    /// Trusted-owner inspection only, like `invocation_outcome` itself.
    pub fn result(&self) -> &ResourceCatalogOutcomeV1 {
        &self.value
    }
    fn intact(&self, original: &Arc<Capture>) -> bool {
        Arc::ptr_eq(original, &self.origin)
            && Arc::ptr_eq(&self.value, &self.seal)
            && original.result_intact(&self.value)
    }
}
pub(super) struct Capture {
    pub(super) owner: Arc<()>,
    pub(super) context: SettingsExecutionContextV1,
    pub(super) invocation: InvocationId,
    native: CatalogRequestCapture,
    max_result_bytes: usize,
    result_ceiling: bool,
    scoped: Option<Arc<scoped::Capture>>,
}
impl std::fmt::Debug for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RegistrationCapture(<owned>)")
    }
}
fn error(e: ResourceError) -> InvocationError {
    match e {
        ResourceError::Limit => InvocationError::Limit,
        ResourceError::Capacity => InvocationError::Capacity,
        ResourceError::Malformed => InvocationError::MalformedIntent,
        ResourceError::Faulted => InvocationError::Faulted,
        _ => InvocationError::Unavailable,
    }
}
fn context_payload(c: &SettingsExecutionContextV1) -> Result<usize, InvocationError> {
    let labels = super::super::settings::basis::labels(c);
    if c.profile != NATIVE_RESOURCE_REGISTRATION_PROFILE_V1 || labels.iter().any(|s| s.is_empty()) {
        return Err(InvocationError::MalformedIntent);
    }
    labels.into_iter().map(str::len).try_fold(0usize, |n, m| {
        n.checked_add(m).ok_or(InvocationError::Limit)
    })
}
fn receipt_bytes(result: &ResourceCatalogOutcomeV1) -> Result<usize, InvocationError> {
    result
        .native_bytes()
        .map_err(error)?
        .checked_sub(size_of::<ResourceCatalogOutcomeV1>())
        .and_then(|n| n.checked_add(size_of::<ResourceRegistrationResultV1>()))
        .ok_or(InvocationError::Limit)
}
pub(super) fn original_receipt_maximum(slot: &Slot) -> Option<(bool, usize)> {
    match &slot.intent {
        Some(BoundIntent::ResourceCatalog(c)) => Some((c.scoped.is_some(), c.max_result_bytes)),
        _ => None,
    }
}
impl Capture {
    pub(super) fn scoped_shape_intact(&self) -> bool {
        self.scoped
            .as_ref()
            .is_none_or(|s| s.shape_intact(self.native.changes.len()))
    }
    fn receipt_bytes(&self, result: &ResourceCatalogOutcomeV1) -> Result<usize, InvocationError> {
        if let Some(s) = &self.scoped {
            scoped::receipt_bytes(s.indices.len())
        } else {
            receipt_bytes(result)
        }
    }
    fn result_intact(&self, result: &ResourceCatalogOutcomeV1) -> bool {
        if !self.scoped_shape_intact() {
            return false;
        }
        if result.registrations.len() != self.native.changes.len() {
            return false;
        }
        let mut changed_stamp = None;
        let mut changed = false;
        for (row, change) in result.registrations.iter().zip(&self.native.changes) {
            if row.address != change.address
                || row
                    .generation
                    .is_some_and(|g| g > crate::coordinator::resource::catalog::MAX_STAMP)
            {
                return false;
            }
            match (&change.action, change.expected, row.generation) {
                (ResourceCatalogActionV1::Remove, CatalogExpectationV1::Absent, None) => {}
                (ResourceCatalogActionV1::Remove, CatalogExpectationV1::Present { .. }, None) => {
                    changed = true
                }
                (ResourceCatalogActionV1::Install(_), expected, Some(g)) => {
                    let old = match expected {
                        CatalogExpectationV1::Absent => 0,
                        CatalogExpectationV1::Present { generation } => generation,
                    };
                    if g < old || (g == 0 && expected == CatalogExpectationV1::Absent) {
                        return false;
                    }
                    if expected == CatalogExpectationV1::Absent || g != old {
                        changed = true;
                        if changed_stamp.is_some_and(|stamp| stamp != g) {
                            return false;
                        }
                        changed_stamp = Some(g);
                    }
                }
                _ => return false,
            }
        }
        changed == (result.status == ResourceCatalogStatusV1::Applied)
    }
}
impl SourceCoordinator {
    pub fn submit_resource_registration_v1(
        &mut self,
        id: &InvocationId,
        changes: &[ResourceCatalogChangeV1],
        c: &SettingsExecutionContextV1,
        f: &TrustedResourceRegistrationInvocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        let repeated = self.invocation_slot(id).is_ok_and(|s| s.intent.is_some());
        let result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            if f.context != c {
                return Err(InvocationError::Unavailable);
            }
            let slot = self.invocation_slot(id)?;
            let payload = context_payload(c)?;
            let budget = self
                .invocations
                .limits
                .max_intent_bytes
                .checked_sub(size_of::<Capture>() - size_of::<CatalogRequestCapture>())
                .and_then(|n| n.checked_sub(payload))
                .ok_or(InvocationError::Limit)?;
            if let Some(original) = &slot.intent {
                let BoundIntent::ResourceCatalog(original) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                if original.invocation != *id || !Arc::ptr_eq(&original.owner, &self.namespace) {
                    return Err(InvocationError::Faulted);
                }
                if original.scoped.is_some() {
                    return Err(InvocationError::IntentMismatch);
                }
                let native =
                    CatalogRequestCapture::copy(changes, f.profile, budget).map_err(error)?;
                if original.context != *c
                    || original.native.profile != f.profile
                    || original.native.changes != native.changes
                    || original.max_result_bytes != f.max_result_bytes
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            if f.interpretation_current != Some(true)
                || self.resource_catalog_profile().map_err(error)? != f.profile
            {
                return Err(InvocationError::Unavailable);
            }
            let native = CatalogRequestCapture::copy(changes, f.profile, budget).map_err(error)?;
            let context = super::super::settings::basis::copy_context(c)
                .map_err(|_| InvocationError::Capacity)?;
            let p = self
                .prepare_resource_catalog_v1(&native.changes, f.native, r)
                .map_err(error)?;
            let cost = receipt_bytes(p.outcome())?;
            if cost > f.max_result_bytes || cost > self.invocations.limits.max_intent_bytes {
                return Err(InvocationError::Limit);
            }
            // Drop first-admission prediction. Service must reconstruct against
            // the then-current root; neither a first stamp nor root is promised.
            drop(p);
            let capture = Arc::new(Capture {
                owner: self.namespace.clone(),
                context,
                invocation: id.clone(),
                native,
                max_result_bytes: f.max_result_bytes,
                result_ceiling: f.host_result_allowed == Some(true),
                scoped: None,
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
        capture(
            r,
            if repeated {
                Boundary::InvocationReplay
            } else {
                Boundary::InvocationAdmission
            },
            &result,
        );
        result
    }
    pub fn run_resource_registration_v1(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        f: &TrustedResourceRegistrationInvocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let terminal = self
            .invocation_slot(id)
            .is_ok_and(|s| matches!(s.phase, Phase::Terminal(_)));
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.run_registration_inner(id, c, f, None, r)
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
    fn run_registration_inner(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        f: &TrustedResourceRegistrationInvocationV1<'_>,
        scoped_facts: Option<&TrustedScopedResourceRegistrationInvocationV1<'_>>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        if self.faulted {
            return Err(InvocationError::Faulted);
        }
        let slot = self.invocation_slot(id)?;
        let Some(BoundIntent::ResourceCatalog(original)) = &slot.intent else {
            return Err(InvocationError::IntentMismatch);
        };
        if original.invocation != *id || !Arc::ptr_eq(&original.owner, &self.namespace) {
            self.faulted = true;
            return Err(InvocationError::Faulted);
        }
        if original.context != *c || f.context != c {
            return Err(InvocationError::Unavailable);
        }
        if original.scoped.is_some() != scoped_facts.is_some() {
            return Err(InvocationError::IntentMismatch);
        }
        if let (Some(scoped), Some(facts)) = (&original.scoped, scoped_facts)
            && scoped.scope != *facts.scope
        {
            return Err(InvocationError::Unavailable);
        }
        match &slot.phase {
            Phase::Terminal(_) => return Ok(InvocationState::Finished),
            Phase::Running => return Err(InvocationError::Faulted),
            Phase::ResourceCatalogPending(_) => {}
            _ => return Err(InvocationError::IntentMismatch),
        }
        if self.service_requested_stop(id, r)? {
            return Ok(InvocationState::Finished);
        }
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated registration slot");
        let Phase::ResourceCatalogPending(pending) =
            std::mem::replace(&mut slot.phase, Phase::Running)
        else {
            unreachable!("registration pending")
        };
        if !matches!(&slot.intent,Some(BoundIntent::ResourceCatalog(original)) if Arc::ptr_eq(original,&pending) && Arc::ptr_eq(&original.owner,&self.namespace))
        {
            self.faulted = true;
            return Err(InvocationError::Faulted);
        }
        // No producer mutation occurs while staging. An unexpected native unwind
        // is nevertheless unknown/faulted, never a fabricated known rejection.
        let staged = catch_unwind(AssertUnwindSafe(|| {
            #[cfg(test)]
            if tests::panic_before_stage() {
                panic!("injected registration staging fault");
            }
            if f.interpretation_current != Some(true)
                || f.profile != pending.native.profile
                || f.max_result_bytes != pending.max_result_bytes
            {
                return Err(ResourceError::Unavailable);
            }
            if self.resource_catalog_profile()? != pending.native.profile {
                return Err(ResourceError::Unavailable);
            }
            if let Some(sf) = scoped_facts {
                self.validate_scoped_registration_capture(&pending, c, sf, r)?;
            }
            let p = self.prepare_resource_catalog_v1(&pending.native.changes, f.native, r)?;
            let stage = self.stage_resource_catalog_v1(p, f.native, r)?;
            let cost = pending
                .receipt_bytes(&stage.outcome)
                .map_err(|_| ResourceError::Limit)?;
            if cost > pending.max_result_bytes || cost > self.invocations.limits.max_intent_bytes {
                return Err(ResourceError::Limit);
            }
            if !pending.result_intact(&stage.outcome) {
                return Err(ResourceError::Faulted);
            }
            if let Some(sf) = scoped_facts {
                self.validate_scoped_registration_capture(&pending, c, sf, r)?;
            }
            let value = Arc::new(stage.outcome);
            let retained = RetainedResourceRegistrationOutcomeV1 {
                origin: pending.clone(),
                seal: value.clone(),
                value,
            };
            Ok((stage.publication, retained))
        }))
        .unwrap_or(Err(ResourceError::Faulted));
        if matches!(staged, Err(ResourceError::Faulted)) {
            self.faulted = true;
            return Err(InvocationError::Faulted);
        }
        self.faulted = true;
        let outcome = match staged {
            Ok((publication, retained)) => {
                let outcome = InvocationOutcome::ResourceCatalogCommitted(retained);
                publication.install(self, r);
                #[cfg(test)]
                if tests::panic_after_publication() {
                    panic!("injected registration publication interruption");
                }
                outcome
            }
            Err(e) => InvocationOutcome::ResourceCatalogRejected(e),
        };
        self.install_invocation_terminal(id, outcome);
        self.faulted = false;
        Ok(InvocationState::Finished)
    }
    /// Typed historical retained-outcome facet. Receipt delivery neither executes
    /// the job nor promotes native owner-wide stamps to safe public identifiers.
    pub fn retained_resource_registration_result_v1(
        &self,
        id: &InvocationId,
        f: &TrustedResourceRegistrationResultV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResourceRegistrationResultV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if self.faulted
                || [
                    f.outcome_readable,
                    f.registration_result_readable,
                    f.registration_result_context_current,
                    f.host_result_allowed,
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
            if original.invocation != *id {
                return Err(InvocationError::Faulted);
            }
            if !Arc::ptr_eq(&original.owner, &self.namespace)
                || original.scoped.is_some()
                || original.context != *f.context
                || !original.result_ceiling
            {
                return Err(InvocationError::Unavailable);
            }
            let Phase::Terminal(Some(InvocationOutcome::ResourceCatalogCommitted(retained))) =
                &slot.phase
            else {
                return Err(InvocationError::OutcomeUnavailable);
            };
            if !retained.intact(original) {
                return Err(InvocationError::Faulted);
            }
            let outcome = retained.result();
            let cost = receipt_bytes(outcome)?;
            if cost > original.max_result_bytes
                || cost > f.max_result_bytes
                || cost > self.invocations.limits.max_intent_bytes
            {
                return Err(InvocationError::Limit);
            }
            Ok(ResourceRegistrationResultV1 {
                invocation: id.clone(),
                result: outcome
                    .copy_bounded(original.native.profile.max_result_bytes)
                    .map_err(error)?,
            })
        })();
        capture_timed(
            r,
            Boundary::ResourceRegistrationResult,
            &result,
            timing.finish(),
        );
        result
    }
}
