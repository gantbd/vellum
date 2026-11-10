//! Conditional layer-registry lowering, using the native layer participant.
use super::*;
use crate::settings::{
    LayerCapture, LayerCreateInput, SettingContributionV1, SettingIdentityV1, SettingLayerRemoveV1,
    SettingScopeV1, SettingTargetV1, SettingsError, SettingsLayerBasisV1, SettingsLayerLimitsV1,
    SettingsLayerOutcomeV1,
};
use settings::basis::definition::ordinary;
use settings::basis::{copy_context, labels};
use settings_reads::observe;
mod lower;
mod selectors;
pub use selectors::{
    BasisSettingLayerReferenceV1, BasisSettingTargetV1, BasisSettingsLifecycleRequestV1,
};
#[cfg(test)]
mod tests;
type Error = SettingsChangeErrorV2;
pub struct ScopedSettingLayerCreateV1 {
    pub id: String,
    pub scope: SettingScopeV1,
    pub values: Vec<(String, SettingContributionV1)>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum ScopedSettingLayerReferenceV1 {
    Existing(SettingIdentityV1),
    Create { index: usize },
}
pub struct ScopedSettingTargetV1 {
    pub identity: SettingIdentityV1,
    pub layers: Vec<ScopedSettingLayerReferenceV1>,
}
pub struct ScopedSettingsLifecycleRequestV1<'a> {
    pub basis: &'a SettingsLifecycleBasisId,
    pub create: &'a [ScopedSettingLayerCreateV1],
    pub remove: &'a [SettingIdentityV1],
    pub targets: &'a [ScopedSettingTargetV1],
    pub dependencies: &'a ReadDependencyRequestV3,
}
pub struct TrustedSettingsLifecycleMutationFactsV1<'a> {
    pub basis: &'a TrustedSettingsLifecycleBasisFactsV1<'a>,
    pub host_basis_allowed: Option<bool>,
    pub layer_registry_writable: Option<bool>,
    pub associations_writable: Option<bool>,
    pub writable_layers: Option<&'a [SettingIdentityV1]>,
    pub required_reads: &'a [ReadDependencyV3],
    /// Independent current facts for selected ordinary write bases; unused without selectors.
    pub selected_basis_facts: Option<&'a TrustedSettingsBasisFactsV1<'a>>,
}
pub(super) struct ScopedLifecycleCapture {
    pub(super) owner: Arc<()>,
    pub(super) context: SettingsExecutionContextV1,
    basis: SettingsLifecycleBasisId,
    expected: SettingsLayerBasisV1,
    layers: Arc<LayerCapture>,
    recipe: Vec<Option<usize>>,
    caller: read_dependency::ReadCapture,
    required: read_dependency::ReadCapture,
    reads: read_dependency::ReadCapture,
    requested_budget: ReadDependencyBudget,
    bytes: usize,
    selectors: Option<selectors::SelectorCapture>,
}
impl std::fmt::Debug for ScopedLifecycleCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ScopedLifecycleCapture(<redacted>)")
    }
}
impl ScopedLifecycleCapture {
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.caller.refers_to_document(document)
            || self.required.refers_to_document(document)
            || self.reads.refers_to_document(document)
    }
}
pub struct PreparedScopedSettingsLifecycleV1 {
    pub(super) capture: Arc<ScopedLifecycleCapture>,
    seal: Arc<ScopedLifecycleCapture>,
    native: PreparedSettingsLayersV1,
}
impl std::fmt::Debug for PreparedScopedSettingsLifecycleV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreparedScopedSettingsLifecycleV1(<redacted>)")
    }
}
impl PreparedScopedSettingsLifecycleV1 {
    pub(super) fn created_result_capture(&self) -> Result<&Arc<LayerCapture>, SettingsError> {
        if !Arc::ptr_eq(&self.capture, &self.seal)
            || !self
                .native
                .plan
                .matches(&self.capture.layers, self.capture.expected)
        {
            return Err(SettingsError::Faulted);
        }
        Ok(&self.capture.layers)
    }
    pub(super) fn retained_bytes(&self) -> Result<usize, SettingsError> {
        self.native
            .retained_bytes()?
            .checked_add(self.capture.bytes)
            .and_then(|n| {
                n.checked_add(
                    std::mem::size_of::<Self>() - std::mem::size_of::<PreparedSettingsLayersV1>(),
                )
            })
            .and_then(|n| n.checked_add(std::mem::size_of::<Arc<ScopedLifecycleCapture>>()))
            .ok_or(SettingsError::Limit)
    }
}
fn s(e: SettingsError) -> Error {
    Error::Settings(e)
}
fn read(e: ReadDependencyError) -> Error {
    Error::Reads(e.into())
}
impl ScopedLifecycleCapture {
    pub(super) fn layer_capture(&self) -> &Arc<LayerCapture> {
        &self.layers
    }
    pub(super) fn replay_matches(
        &self,
        q: &ScopedSettingsLifecycleRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        parent: usize,
    ) -> Result<bool, Error> {
        if self.selectors.is_some() {
            return Ok(false);
        }
        self.native_replay_matches(q, c, parent)
    }
    fn native_replay_matches(
        &self,
        q: &ScopedSettingsLifecycleRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        parent: usize,
    ) -> Result<bool, Error> {
        if self.basis != *q.basis
            || self.context != *c
            || self.requested_budget != q.dependencies.budget
            || self.caller.predicates.len() != q.dependencies.predicates.len()
        {
            return Ok(false);
        }
        let overhead = self
            .bytes
            .checked_sub(self.layers.bytes())
            .ok_or(s(SettingsError::Faulted))?;
        let ceiling = parent
            .checked_sub(overhead)
            .ok_or(s(SettingsError::Limit))?;
        let caller = read_dependency::ReadCapture::configuration_slice(
            &q.dependencies.predicates,
            self.caller.budget,
        )
        .map_err(read)?;
        Ok(caller == self.caller && lower::replay(self, q, ceiling)?)
    }
}
impl SourceCoordinator {
    fn scoped_lifecycle_gate(
        &self,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleMutationFactsV1<'_>,
    ) -> bool {
        f.host_basis_allowed == Some(true)
            && self.lifecycle_basis_binding_current(c, f.basis)
            && ordinary(f.basis.common)
    }
    fn lifecycle_grants(
        &self,
        capture: &ScopedLifecycleCapture,
        o: SettingsLayerOutcomeV1,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleMutationFactsV1<'_>,
    ) -> Result<(), Error> {
        if o.associations_changed && f.associations_writable != Some(true) {
            return Err(s(SettingsError::Unavailable));
        }
        if o.created_layers == 0 && o.removed_layers == 0 {
            return Ok(());
        }
        if f.layer_registry_writable != Some(true) {
            return Err(s(SettingsError::Unavailable));
        }
        let packet = TrustedSettingsBasisFactsV1 {
            writable_layers: f.writable_layers,
            ..*f.basis.common
        };
        if !self.basis_context_current(c, &packet) {
            return Err(s(SettingsError::Unavailable));
        }
        let grants = f.writable_layers.ok_or(s(SettingsError::Unavailable))?;
        let mut work = capture
            .layers
            .limits
            .max_work
            .min(capture.layers.captured_limits().max_work);
        for required in capture
            .layers
            .created_identities()
            .chain(capture.layers.removals().iter().map(|r| &r.identity))
        {
            let mut found = false;
            for grant in grants {
                work = work.checked_sub(1).ok_or(s(SettingsError::Limit))?;
                if grant == required {
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(s(SettingsError::Unavailable));
            }
        }
        Ok(())
    }
    pub(super) fn capture_scoped_lifecycle(
        &self,
        q: &ScopedSettingsLifecycleRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleMutationFactsV1<'_>,
    ) -> Result<Arc<ScopedLifecycleCapture>, Error> {
        self.ensure_ordinary_admission().map_err(Error::Reads)?;
        if !self.scoped_lifecycle_gate(c, f) {
            return Err(s(SettingsError::Unavailable));
        }
        let expected = self.settings_lifecycle_condition(q.basis, c).map_err(s)?;
        let parent = self.settings_job_parent_limit();
        let p = f.basis.lifecycle;
        let count = q
            .dependencies
            .predicates
            .len()
            .checked_add(f.required_reads.len())
            .ok_or(s(SettingsError::Limit))?;
        let floor = read_dependency::ReadCapture::size_for_predicates(count)
            .ok_or(s(SettingsError::Limit))?;
        if floor > parent || floor > p.max_request_bytes {
            return Err(s(SettingsError::Limit));
        }
        let budget = ReadDependencyBudget {
            max_work: q.dependencies.budget.max_work.min(p.max_work),
            ..q.dependencies.budget
        };
        let caller =
            read_dependency::ReadCapture::configuration_slice(&q.dependencies.predicates, budget)
                .map_err(read)?;
        let required = read_dependency::ReadCapture::configuration_slice(f.required_reads, budget)
            .map_err(read)?;
        let reads = caller.conjoin(&required).map_err(read)?;
        let root = self
            .settings
            .as_ref()
            .ok_or(s(SettingsError::Unavailable))?;
        if q.targets.len() > root.limits().max_targets || q.targets.len() > p.max_work {
            return Err(s(SettingsError::Limit));
        }
        let refs = q
            .targets
            .iter()
            .try_fold(0usize, |n, t| n.checked_add(t.layers.len()))
            .ok_or(s(SettingsError::Limit))?;
        if refs > p.max_work {
            return Err(s(SettingsError::Limit));
        }
        let mut header = refs
            .checked_mul(std::mem::size_of::<Option<usize>>())
            .and_then(|n| n.checked_add(std::mem::size_of::<ScopedLifecycleCapture>()))
            .ok_or(s(SettingsError::Limit))?;
        for label in labels(c) {
            header = header
                .checked_add(label.len())
                .ok_or(s(SettingsError::Limit))?;
        }
        for r in [&caller, &required, &reads] {
            header = header
                .checked_add(
                    r.logical_bytes().ok_or(s(SettingsError::Limit))?
                        - std::mem::size_of::<read_dependency::ReadCapture>(),
                )
                .ok_or(s(SettingsError::Limit))?;
        }
        let ceiling = parent
            .min(p.max_request_bytes)
            .checked_sub(header)
            .ok_or(s(SettingsError::Limit))?;
        let (layers, recipe) = lower::initial(q, root, expected, p, ceiling)?;
        let bytes = header
            .checked_add(layers.bytes())
            .ok_or(s(SettingsError::Limit))?;
        if bytes
            .checked_add(std::mem::size_of::<Arc<ScopedLifecycleCapture>>())
            .is_none_or(|n| n > parent)
        {
            return Err(s(SettingsError::Limit));
        }
        Ok(Arc::new(ScopedLifecycleCapture {
            owner: self.namespace.clone(),
            context: copy_context(c).map_err(s)?,
            basis: q.basis.clone(),
            expected,
            layers: Arc::new(layers),
            recipe,
            caller,
            required,
            reads,
            requested_budget: q.dependencies.budget,
            bytes,
            selectors: None,
        }))
    }
    fn scoped_lifecycle_bound(&self, p: &PreparedScopedSettingsLifecycleV1) -> Result<(), Error> {
        let bytes = p.retained_bytes().map_err(s)?;
        if bytes > p.capture.layers.limits.max_plan_bytes
            || bytes > self.settings_job_parent_limit()
        {
            return Err(s(SettingsError::Limit));
        }
        Ok(())
    }
    pub(super) fn prepare_scoped_lifecycle_capture(
        &mut self,
        capture: Arc<ScopedLifecycleCapture>,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<PreparedScopedSettingsLifecycleV1, Error> {
        let native = self.prepare_layer_capture(
            capture.expected,
            capture.layers.clone(),
            capture.reads.copy_owned().map_err(read)?,
            r,
        )?;
        let p = PreparedScopedSettingsLifecycleV1 {
            seal: capture.clone(),
            capture,
            native,
        };
        self.scoped_lifecycle_bound(&p)?;
        let root = self
            .settings
            .as_ref()
            .ok_or(s(SettingsError::Unavailable))?;
        let (_, outcome) = p.native.plan.decide(root).map_err(s)?;
        self.lifecycle_grants(&p.capture, outcome, c, f)?;
        self.validate_lifecycle_selectors(&p.capture, c, f, r)?;
        self.validate_lifecycle_basis(&p.capture.basis, c, f.basis)
            .map_err(s)?;
        Ok(p)
    }
    pub fn prepare_scoped_settings_lifecycle(
        &mut self,
        q: &ScopedSettingsLifecycleRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<PreparedScopedSettingsLifecycleV1, Error> {
        let capture = self.capture_scoped_lifecycle(q, c, f);
        observe(r, Boundary::ScopedLifecycleLowering, &capture);
        let result =
            capture.and_then(|capture| self.prepare_scoped_lifecycle_capture(capture, c, f, r));
        if result.as_ref().err().is_some_and(|e| e.faulted()) {
            self.faulted = true;
        }
        observe(r, Boundary::ScopedLifecyclePreparation, &result);
        result
    }
    pub(super) fn stage_scoped_lifecycle(
        &mut self,
        p: PreparedScopedSettingsLifecycleV1,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<settings_layers::LayerStage, Error> {
        let result = (|| {
            if !Arc::ptr_eq(&p.capture, &p.seal)
                || !p.native.plan.matches(&p.capture.layers, p.capture.expected)
                || p.native.reads != p.capture.reads
                || p.native.read_count != p.capture.reads.predicates.len()
            {
                return Err(s(SettingsError::Faulted));
            }
            if !Arc::ptr_eq(&p.capture.owner, &self.namespace)
                || p.capture.context != *c
                || !self.scoped_lifecycle_gate(c, f)
            {
                return Err(s(SettingsError::Unavailable));
            }
            self.scoped_lifecycle_bound(&p)?;
            if f.required_reads.len() != p.capture.required.predicates.len() {
                return Err(s(SettingsError::Stale));
            }
            let required = read_dependency::ReadCapture::configuration_slice(
                f.required_reads,
                p.capture.required.budget,
            )
            .map_err(read)?;
            if required != p.capture.required {
                return Err(s(SettingsError::Stale));
            }
            let stage = self.stage_settings_layers(p.native, r)?;
            self.lifecycle_grants(&p.capture, stage.outcome, c, f)?;
            self.validate_lifecycle_selectors(&p.capture, c, f, r)?;
            self.validate_lifecycle_basis(&p.capture.basis, c, f.basis)
                .map_err(s)?;
            if self
                .settings_lifecycle_condition(&p.capture.basis, c)
                .map_err(s)?
                != p.capture.expected
            {
                return Err(s(SettingsError::Faulted));
            }
            Ok(stage)
        })();
        observe(r, Boundary::ScopedLifecycleValidation, &result);
        result
    }
    pub fn commit_scoped_settings_lifecycle(
        &mut self,
        p: PreparedScopedSettingsLifecycleV1,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ScopedSettingsUpdateV1, Error> {
        if self.faulted {
            let result = Err(s(SettingsError::Faulted));
            observe(r, Boundary::ScopedLifecyclePublication, &result);
            return result;
        }
        self.faulted = true;
        let result = self.stage_scoped_lifecycle(p, c, f, r).map(|stage| {
            let outcome = ScopedSettingsUpdateV1 {
                changed: stage.outcome.changed,
            };
            stage.install(self);
            outcome
        });
        if !result.as_ref().err().is_some_and(|e| e.faulted()) {
            self.faulted = false;
        }
        observe(r, Boundary::ScopedLifecyclePublication, &result);
        result
    }
}
