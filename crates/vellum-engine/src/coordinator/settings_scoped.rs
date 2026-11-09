//! First-party conditional lowering. No requester-supplied native layer versions.
use super::*;
use crate::settings::{
    SettingChangeInput, SettingOperationV1, SettingsError, SettingsRequestCapture,
};
use settings::basis::{all_guards, copy_context, labels};
use settings_reads::observe;
type Error = SettingsChangeErrorV2;
pub struct ScopedSettingChangeV1 {
    pub basis: usize,
    pub key: String,
    pub operation: SettingOperationV1,
}
pub struct ScopedSettingsRequestV1<'a> {
    pub bases: &'a [SettingsWriteBasisId],
    pub changes: &'a [ScopedSettingChangeV1],
    pub dependencies: &'a ReadDependencyRequestV3,
}
/// Current first-party host facts, never accepted as requester assertions.
pub struct TrustedSettingsMutationFactsV1<'a> {
    pub basis: &'a TrustedSettingsBasisFactsV1<'a>,
    pub host_basis_allowed: Option<bool>,
    pub required_reads: &'a [ReadDependencyV3],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopedSettingsUpdateV1 {
    pub changed: bool,
}
pub(super) struct ScopedCapture {
    pub(super) owner: Arc<()>,
    pub(super) context: SettingsExecutionContextV1,
    pub(super) native: SettingsRequestCapture,
    pub(super) bases: Vec<SettingsWriteBasisId>,
    indices: Vec<usize>,
    caller: read_dependency::ReadCapture,
    required: read_dependency::ReadCapture,
    pub(super) reads: read_dependency::ReadCapture,
    bytes: usize,
}
impl std::fmt::Debug for ScopedCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ScopedCapture(<redacted>)")
    }
}
impl ScopedCapture {
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.caller.refers_to_document(document)
            || self.required.refers_to_document(document)
            || self.reads.refers_to_document(document)
    }
}
#[derive(Debug)]
pub struct PreparedScopedSettingsV1 {
    pub(super) capture: Arc<ScopedCapture>,
    seal: Arc<ScopedCapture>,
    pub(super) native: PreparedSettingsChangeV2,
}
impl PreparedScopedSettingsV1 {
    pub(super) fn retained_bytes(&self) -> Result<usize, SettingsError> {
        self.native
            .retained_bytes()?
            .checked_add(self.capture.bytes)
            .and_then(|n| {
                n.checked_add(
                    std::mem::size_of::<Self>() - std::mem::size_of::<PreparedSettingsChangeV2>(),
                )
            })
            .and_then(|n| n.checked_add(std::mem::size_of::<Arc<ScopedCapture>>()))
            .ok_or(SettingsError::Limit)
    }
}
fn settings(error: SettingsError) -> Error {
    Error::Settings(error)
}
fn ordinary_guards(f: &TrustedSettingsBasisFactsV1<'_>) -> bool {
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
    .all(|fact| fact == Some(true))
}
fn reads(error: ReadDependencyError) -> Error {
    Error::Reads(error.into())
}
fn reserve<T>(out: &mut Vec<T>, count: usize) -> Result<(), Error> {
    out.try_reserve_exact(count)
        .map_err(|_| settings(SettingsError::Capacity))
}
fn step(work: &mut usize) -> Result<(), Error> {
    *work = work.checked_sub(1).ok_or(settings(SettingsError::Limit))?;
    Ok(())
}
impl ScopedCapture {
    pub(super) fn replay_matches(
        &self,
        request: &ScopedSettingsRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        parent: usize,
    ) -> Result<bool, Error> {
        if self.context != *c
            || self.bases != request.bases
            || self.indices.len() != request.changes.len()
        {
            return Ok(false);
        }
        let overhead = self
            .bytes
            .checked_sub(self.native.retained_bytes())
            .and_then(|n| n.checked_add(std::mem::size_of::<Arc<Self>>()))
            .ok_or(settings(SettingsError::Faulted))?;
        let ceiling = parent
            .checked_sub(overhead)
            .ok_or(settings(SettingsError::Limit))?;
        let caller_size = read_dependency::ReadInput::V3(request.dependencies)
            .retained_bytes(&[])
            .ok_or(settings(SettingsError::Limit))?;
        if caller_size
            > self
                .caller
                .logical_bytes()
                .ok_or(settings(SettingsError::Faulted))?
        {
            return Ok(false);
        }
        let caller = read_dependency::ReadInput::V3(request.dependencies)
            .copy(&[])
            .map_err(reads)?;
        if caller != self.caller {
            return Ok(false);
        }
        let mut inputs = Vec::new();
        reserve(&mut inputs, request.changes.len())?;
        for (i, change) in request.changes.iter().enumerate() {
            if change.basis != self.indices[i] {
                return Ok(false);
            }
            let (layer, expected_generation) = self
                .native
                .condition_at(i)
                .ok_or(settings(SettingsError::Faulted))?;
            inputs.push(SettingChangeInput {
                layer,
                expected_generation,
                key: &change.key,
                operation: &change.operation,
            });
        }
        let native = SettingsRequestCapture::copy_inputs(&inputs, self.native.profile, ceiling)
            .map_err(settings)?;
        self.native.same(&native).map_err(settings)
    }
}
impl SourceCoordinator {
    fn scope_inputs<'a>(
        &'a self,
        request: &'a ScopedSettingsRequestV1<'_>,
        c: &SettingsExecutionContextV1,
    ) -> Result<Vec<SettingChangeInput<'a>>, Error> {
        let profile = self
            .settings_activation
            .ok_or(settings(SettingsError::Unavailable))?;
        if request.changes.len() > profile.max_changes || request.bases.len() > profile.max_changes
        {
            return Err(settings(SettingsError::Limit));
        }
        let mut work = profile.max_work;
        for (i, basis) in request.bases.iter().enumerate() {
            self.settings_write_condition(basis, c).map_err(settings)?;
            for other in &request.bases[..i] {
                step(&mut work)?;
                if basis == other {
                    return Err(settings(SettingsError::Duplicate));
                }
            }
            let mut used = false;
            for change in request.changes {
                step(&mut work)?;
                used |= change.basis == i;
            }
            if !used {
                return Err(settings(SettingsError::InvalidValue));
            }
        }
        let mut inputs = Vec::new();
        reserve(&mut inputs, request.changes.len())?;
        for change in request.changes {
            step(&mut work)?;
            let basis = request
                .bases
                .get(change.basis)
                .ok_or(settings(SettingsError::InvalidValue))?;
            let (layer, expected_generation) =
                self.settings_write_condition(basis, c).map_err(settings)?;
            inputs.push(SettingChangeInput {
                layer,
                expected_generation,
                key: &change.key,
                operation: &change.operation,
            });
        }
        Ok(inputs)
    }
    pub(super) fn capture_scoped_settings(
        &self,
        request: &ScopedSettingsRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        g: &TrustedSettingsMutationFactsV1<'_>,
    ) -> Result<Arc<ScopedCapture>, Error> {
        self.ensure_ordinary_admission().map_err(Error::Reads)?;
        if g.host_basis_allowed != Some(true)
            || !self.basis_context_current(c, g.basis)
            || !ordinary_guards(g.basis)
        {
            return Err(settings(SettingsError::Unavailable));
        }
        let profile = self
            .settings_activation
            .ok_or(settings(SettingsError::Unavailable))?;
        if request.bases.len() > profile.max_changes || request.changes.len() > profile.max_changes
        {
            return Err(settings(SettingsError::Limit));
        }
        let parent = self.settings_job_parent_limit();
        let input_rows = request
            .dependencies
            .predicates
            .len()
            .checked_add(g.required_reads.len())
            .ok_or(settings(SettingsError::Limit))?;
        let rows = read_dependency::ReadCapture::size_for_predicates(input_rows)
            .ok_or(settings(SettingsError::Limit))?;
        if rows > parent {
            return Err(settings(SettingsError::Limit));
        }
        let caller = read_dependency::ReadInput::V3(request.dependencies)
            .copy(&[])
            .map_err(reads)?;
        let required = read_dependency::ReadCapture::configuration_slice(
            g.required_reads,
            request.dependencies.budget,
        )
        .map_err(reads)?;
        let combined = caller.conjoin(&required).map_err(reads)?;
        let mut floor =
            std::mem::size_of::<ScopedCapture>() - std::mem::size_of::<SettingsRequestCapture>();
        for capture in [&caller, &required, &combined] {
            floor = floor
                .checked_add(
                    capture
                        .logical_bytes()
                        .ok_or(settings(SettingsError::Limit))?
                        - std::mem::size_of::<read_dependency::ReadCapture>(),
                )
                .ok_or(settings(SettingsError::Limit))?;
        }
        for label in labels(c) {
            floor = floor
                .checked_add(label.len())
                .ok_or(settings(SettingsError::Limit))?;
        }
        floor = request
            .bases
            .len()
            .checked_mul(std::mem::size_of::<SettingsWriteBasisId>())
            .and_then(|n| floor.checked_add(n))
            .and_then(|n| {
                request
                    .changes
                    .len()
                    .checked_mul(std::mem::size_of::<usize>())
                    .and_then(|m| n.checked_add(m))
            })
            .ok_or(settings(SettingsError::Limit))?;
        let ceiling = parent
            .checked_sub(floor)
            .and_then(|n| n.checked_sub(std::mem::size_of::<Arc<ScopedCapture>>()))
            .ok_or(settings(SettingsError::Limit))?;
        let inputs = self.scope_inputs(request, c)?;
        let mut permission_work = profile.max_work;
        for input in &inputs {
            let layers = g
                .basis
                .writable_layers
                .ok_or(settings(SettingsError::Unavailable))?;
            let mut granted = false;
            for layer in layers {
                step(&mut permission_work)?;
                if layer == input.layer {
                    granted = true;
                    break;
                }
            }
            if !granted {
                return Err(settings(SettingsError::Unavailable));
            }
        }
        let native =
            SettingsRequestCapture::copy_inputs(&inputs, profile, ceiling).map_err(settings)?;
        let bytes = floor
            .checked_add(native.retained_bytes())
            .ok_or(settings(SettingsError::Limit))?;
        let mut bases = Vec::new();
        reserve(&mut bases, request.bases.len())?;
        bases.extend_from_slice(request.bases);
        let mut indices = Vec::new();
        reserve(&mut indices, request.changes.len())?;
        indices.extend(request.changes.iter().map(|c| c.basis));
        Ok(Arc::new(ScopedCapture {
            owner: self.namespace.clone(),
            context: copy_context(c).map_err(settings)?,
            native,
            bases,
            indices,
            caller,
            required,
            reads: combined,
            bytes,
        }))
    }
    pub(super) fn prepare_scoped_capture(
        &mut self,
        capture: Arc<ScopedCapture>,
        request: &ScopedSettingsRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        g: &TrustedSettingsMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<PreparedScopedSettingsV1, Error> {
        let inputs = self.scope_inputs(request, c)?;
        // Pure whole-proposal semantic preflight precedes every stateful loss observer.
        self.check_owned_read_capture(&capture.reads, r)
            .map_err(Error::Reads)?;
        let prepared_settings = self
            .prepare_settings_input_changes(&inputs, r)
            .map_err(settings)?;
        let reads = capture.reads.copy_owned().map_err(reads)?;
        let native = PreparedSettingsChangeV2 {
            read_count: reads.predicates.len(),
            settings: prepared_settings,
            reads,
            reads_required: true,
        };
        let prepared = PreparedScopedSettingsV1 {
            seal: capture.clone(),
            capture,
            native,
        };
        if prepared.retained_bytes().map_err(settings)?
            > prepared.capture.native.profile.max_plan_bytes
        {
            return Err(settings(SettingsError::Limit));
        }
        if prepared.retained_bytes().map_err(settings)? > self.settings_job_parent_limit() {
            return Err(settings(SettingsError::Limit));
        }
        self.validate_scoped_authority(&prepared.capture, c, g, r)?;
        Ok(prepared)
    }
    fn validate_scoped_authority(
        &mut self,
        capture: &ScopedCapture,
        c: &SettingsExecutionContextV1,
        g: &TrustedSettingsMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        if !Arc::ptr_eq(&capture.owner, &self.namespace)
            || capture.context != *c
            || g.host_basis_allowed != Some(true)
            || !self.basis_context_current(c, g.basis)
        {
            return Err(settings(SettingsError::Unavailable));
        }
        if g.required_reads.len() != capture.required.predicates.len() {
            return Err(settings(SettingsError::Stale));
        }
        let mandatory = read_dependency::ReadCapture::configuration_slice(
            g.required_reads,
            capture.required.budget,
        )
        .map_err(reads)?;
        if mandatory != capture.required {
            return Err(settings(SettingsError::Stale));
        }
        // Bind every original handle before any observer can close a preceding one.
        for basis in &capture.bases {
            self.settings_write_condition(basis, c).map_err(settings)?;
        }
        for (i, basis) in capture.indices.iter().enumerate() {
            let handle = capture
                .bases
                .get(*basis)
                .ok_or(settings(SettingsError::Faulted))?;
            let condition = self.settings_write_condition(handle, c).map_err(settings)?;
            if capture.native.condition_at(i) != Some(condition) {
                return Err(settings(SettingsError::Faulted));
            }
        }
        for basis in &capture.bases {
            let check = self.validate_settings_write_basis(basis, c, g.basis);
            crate::settings::observe(r, Boundary::SettingsBasisInspection, &check);
            check.map_err(settings)?;
        }
        if !all_guards(g.basis) {
            return Err(settings(SettingsError::Unavailable));
        }
        Ok(())
    }
    pub fn prepare_scoped_settings_change(
        &mut self,
        request: &ScopedSettingsRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        g: &TrustedSettingsMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<PreparedScopedSettingsV1, Error> {
        let result = self
            .capture_scoped_settings(request, c, g)
            .and_then(|capture| self.prepare_scoped_capture(capture, request, c, g, r));
        if result.as_ref().err().is_some_and(|e| e.faulted()) {
            self.faulted = true;
        }
        observe(r, Boundary::ScopedSettingsPreparation, &result);
        result
    }
    pub(super) fn stage_scoped_settings(
        &mut self,
        p: PreparedScopedSettingsV1,
        c: &SettingsExecutionContextV1,
        g: &TrustedSettingsMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<settings::SettingsStage, Error> {
        let result = (|| {
            if !Arc::ptr_eq(&p.capture, &p.seal) {
                return Err(settings(SettingsError::Faulted));
            }
            if !Arc::ptr_eq(&p.native.settings.owner, &self.namespace) {
                return Err(settings(SettingsError::Unavailable));
            }
            if !p.native.reads_required
                || p.native.read_count != p.capture.reads.predicates.len()
                || p.native.reads != p.capture.reads
                || !p
                    .native
                    .settings
                    .plan
                    .matches(&p.capture.native)
                    .map_err(settings)?
            {
                return Err(settings(SettingsError::Faulted));
            }
            if p.retained_bytes().map_err(settings)? > p.capture.native.profile.max_plan_bytes
                || p.retained_bytes().map_err(settings)? > self.settings_job_parent_limit()
            {
                return Err(settings(SettingsError::Limit));
            }
            self.validate_scoped_authority(&p.capture, c, g, r)?;
            self.stage_settings_with_reads(p.native, r)
        })();
        observe(r, Boundary::ScopedSettingsValidation, &result);
        result
    }
    pub fn commit_scoped_settings_change(
        &mut self,
        p: PreparedScopedSettingsV1,
        c: &SettingsExecutionContextV1,
        g: &TrustedSettingsMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ScopedSettingsUpdateV1, Error> {
        if self.faulted {
            let result = Err(settings(SettingsError::Faulted));
            observe(r, Boundary::ScopedSettingsPublication, &result);
            return result;
        }
        self.faulted = true;
        let result = self.stage_scoped_settings(p, c, g, r).map(|stage| {
            let outcome = ScopedSettingsUpdateV1 {
                changed: stage.outcome.changed,
            };
            stage.install(self);
            outcome
        });
        if !result.as_ref().err().is_some_and(|e| e.faulted()) {
            self.faulted = false;
        }
        observe(r, Boundary::ScopedSettingsPublication, &result);
        result
    }
}
