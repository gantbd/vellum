//! First-party declaration-role lowering into the existing definition participant.
use super::*;
#[cfg(test)]
pub(super) mod tests;
use crate::settings::{
    DefinitionCapture, SettingsDeclarationV1, SettingsDefinitionOutcomeV1, SettingsError,
    SettingsInterpretationEpochsV1,
};
use settings::basis::definition::ordinary;
use settings::basis::{copy_context, labels};
use settings_reads::observe;
type Error = SettingsChangeErrorV2;
pub struct ScopedSettingsDefinitionRequestV1<'a> {
    pub basis: &'a SettingsDefinitionBasisId,
    pub declaration: &'a SettingsDeclarationV1,
    pub dependencies: &'a ReadDependencyRequestV3,
}
/// Current trusted premises; basis observation does not authorize changed facets.
pub struct TrustedSettingsDefinitionMutationFactsV1<'a> {
    pub basis: &'a TrustedSettingsDefinitionBasisFactsV1<'a>,
    pub host_basis_allowed: Option<bool>,
    pub schema_writable: Option<bool>,
    pub restrictions_writable: Option<bool>,
    pub associations_writable: Option<bool>,
    pub required_reads: &'a [ReadDependencyV3],
}
pub(super) struct ScopedDefinitionCapture {
    pub(super) owner: Arc<()>,
    pub(super) context: SettingsExecutionContextV1,
    basis: SettingsDefinitionBasisId,
    expected: SettingsInterpretationEpochsV1,
    declaration: Arc<DefinitionCapture>,
    caller: read_dependency::ReadCapture,
    required: read_dependency::ReadCapture,
    reads: read_dependency::ReadCapture,
    requested_budget: ReadDependencyBudget,
    bytes: usize,
}
impl std::fmt::Debug for ScopedDefinitionCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ScopedDefinitionCapture(<redacted>)")
    }
}
impl ScopedDefinitionCapture {
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.caller.refers_to_document(document)
            || self.required.refers_to_document(document)
            || self.reads.refers_to_document(document)
    }
}
pub struct PreparedScopedSettingsDefinitionV1 {
    pub(super) capture: Arc<ScopedDefinitionCapture>,
    seal: Arc<ScopedDefinitionCapture>,
    native: PreparedSettingsDefinitionV1,
}
impl std::fmt::Debug for PreparedScopedSettingsDefinitionV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreparedScopedSettingsDefinitionV1(<redacted>)")
    }
}
impl PreparedScopedSettingsDefinitionV1 {
    pub(super) fn retained_bytes(&self) -> Result<usize, SettingsError> {
        self.native
            .retained_bytes()?
            .checked_add(self.capture.bytes)
            .and_then(|n| {
                n.checked_add(
                    std::mem::size_of::<Self>()
                        - std::mem::size_of::<PreparedSettingsDefinitionV1>(),
                )
            })
            .and_then(|n| n.checked_add(std::mem::size_of::<Arc<ScopedDefinitionCapture>>()))
            .ok_or(SettingsError::Limit)
    }
}
fn s(e: SettingsError) -> Error {
    Error::Settings(e)
}
fn read(e: ReadDependencyError) -> Error {
    Error::Reads(e.into())
}
fn grants(
    o: SettingsDefinitionOutcomeV1,
    f: &TrustedSettingsDefinitionMutationFactsV1<'_>,
) -> bool {
    (!o.schema_changed || f.schema_writable == Some(true))
        && (!o.restrictions_changed || f.restrictions_writable == Some(true))
        && (!o.associations_changed || f.associations_writable == Some(true))
}
impl ScopedDefinitionCapture {
    pub(super) fn replay_matches(
        &self,
        request: &ScopedSettingsDefinitionRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        parent: usize,
    ) -> Result<bool, Error> {
        if self.basis != *request.basis
            || self.context != *c
            || self.requested_budget != request.dependencies.budget
            || self.caller.predicates.len() != request.dependencies.predicates.len()
        {
            return Ok(false);
        }
        let overhead = self
            .bytes
            .checked_sub(self.declaration.bytes())
            .ok_or(s(SettingsError::Faulted))?;
        let ceiling = parent
            .checked_sub(overhead)
            .ok_or(s(SettingsError::Limit))?;
        let declaration = self
            .declaration
            .recopy(request.declaration, ceiling)
            .map_err(s)?;
        let input = read_dependency::ReadCapture::configuration_slice(
            &request.dependencies.predicates,
            self.caller.budget,
        )
        .map_err(read)?;
        Ok(self.declaration.same(&declaration).map_err(s)? && input == self.caller)
    }
}
impl SourceCoordinator {
    fn scoped_definition_gate(
        &self,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsDefinitionMutationFactsV1<'_>,
    ) -> bool {
        f.host_basis_allowed == Some(true)
            && self.definition_basis_binding_current(c, f.basis)
            && ordinary(f.basis.common)
    }
    pub(super) fn capture_scoped_definition(
        &self,
        request: &ScopedSettingsDefinitionRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsDefinitionMutationFactsV1<'_>,
    ) -> Result<Arc<ScopedDefinitionCapture>, Error> {
        self.ensure_ordinary_admission().map_err(Error::Reads)?;
        if !self.scoped_definition_gate(c, f) {
            return Err(s(SettingsError::Unavailable));
        }
        let expected = self
            .settings_definition_condition(request.basis, c)
            .map_err(s)?;
        let parent = self.settings_job_parent_limit();
        let limits = f.basis.definition;
        let count = request
            .dependencies
            .predicates
            .len()
            .checked_add(f.required_reads.len())
            .ok_or(s(SettingsError::Limit))?;
        let floor = read_dependency::ReadCapture::size_for_predicates(count)
            .ok_or(s(SettingsError::Limit))?;
        if floor > parent || floor > limits.max_request_bytes {
            return Err(s(SettingsError::Limit));
        }
        let budget = ReadDependencyBudget {
            max_work: request.dependencies.budget.max_work.min(limits.max_work),
            ..request.dependencies.budget
        };
        let caller = read_dependency::ReadCapture::configuration_slice(
            &request.dependencies.predicates,
            budget,
        )
        .map_err(read)?;
        let required = read_dependency::ReadCapture::configuration_slice(f.required_reads, budget)
            .map_err(read)?;
        let reads = caller.conjoin(&required).map_err(read)?;
        let mut header = std::mem::size_of::<ScopedDefinitionCapture>();
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
            .min(limits.max_request_bytes)
            .checked_sub(header)
            .ok_or(s(SettingsError::Limit))?;
        let root = self
            .settings
            .as_ref()
            .ok_or(s(SettingsError::Unavailable))?;
        let declaration =
            DefinitionCapture::from_state(request.declaration, root, limits, ceiling).map_err(s)?;
        let bytes = header
            .checked_add(declaration.bytes())
            .ok_or(s(SettingsError::Limit))?;
        if bytes
            .checked_add(std::mem::size_of::<Arc<ScopedDefinitionCapture>>())
            .is_none_or(|n| n > parent)
        {
            return Err(s(SettingsError::Limit));
        }
        Ok(Arc::new(ScopedDefinitionCapture {
            owner: self.namespace.clone(),
            context: copy_context(c).map_err(s)?,
            basis: request.basis.clone(),
            expected,
            declaration: Arc::new(declaration),
            caller,
            required,
            reads,
            requested_budget: request.dependencies.budget,
            bytes,
        }))
    }
    fn scoped_definition_bound(&self, p: &PreparedScopedSettingsDefinitionV1) -> Result<(), Error> {
        let bytes = p.retained_bytes().map_err(s)?;
        if bytes > p.capture.declaration.limits.max_plan_bytes
            || bytes > self.settings_job_parent_limit()
        {
            return Err(s(SettingsError::Limit));
        }
        Ok(())
    }
    pub(super) fn prepare_scoped_definition_capture(
        &mut self,
        capture: Arc<ScopedDefinitionCapture>,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsDefinitionMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<PreparedScopedSettingsDefinitionV1, Error> {
        let native = self.prepare_definition_capture(
            capture.expected,
            capture.declaration.clone(),
            capture.reads.copy_owned().map_err(read)?,
            r,
        )?;
        let p = PreparedScopedSettingsDefinitionV1 {
            seal: capture.clone(),
            capture,
            native,
        };
        self.scoped_definition_bound(&p)?;
        let root = self
            .settings
            .as_ref()
            .ok_or(s(SettingsError::Unavailable))?;
        let (_, outcome) = p.native.plan.decide(root).map_err(s)?;
        if !grants(outcome, f) {
            return Err(s(SettingsError::Unavailable));
        }
        self.validate_settings_definition_basis(&p.capture.basis, c, f.basis)
            .map_err(s)?;
        Ok(p)
    }
    pub fn prepare_scoped_settings_definition(
        &mut self,
        request: &ScopedSettingsDefinitionRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsDefinitionMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<PreparedScopedSettingsDefinitionV1, Error> {
        let result = self
            .capture_scoped_definition(request, c, f)
            .and_then(|capture| self.prepare_scoped_definition_capture(capture, c, f, r));
        if result.as_ref().err().is_some_and(|e| e.faulted()) {
            self.faulted = true;
        }
        observe(r, Boundary::ScopedDefinitionPreparation, &result);
        result
    }
    pub(super) fn stage_scoped_definition(
        &mut self,
        p: PreparedScopedSettingsDefinitionV1,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsDefinitionMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<settings_definition::DefinitionStage, Error> {
        let result = (|| {
            if !Arc::ptr_eq(&p.capture, &p.seal)
                || !p
                    .native
                    .plan
                    .matches(&p.capture.declaration, p.capture.expected)
                || p.native.reads != p.capture.reads
                || p.native.read_count != p.capture.reads.predicates.len()
            {
                return Err(s(SettingsError::Faulted));
            }
            if !Arc::ptr_eq(&p.capture.owner, &self.namespace)
                || p.capture.context != *c
                || !self.scoped_definition_gate(c, f)
            {
                return Err(s(SettingsError::Unavailable));
            }
            self.scoped_definition_bound(&p)?;
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
            let stage = self.stage_settings_definition(p.native, r)?;
            if !grants(stage.outcome, f) {
                return Err(s(SettingsError::Unavailable));
            }
            self.validate_settings_definition_basis(&p.capture.basis, c, f.basis)
                .map_err(s)?;
            if self
                .settings_definition_condition(&p.capture.basis, c)
                .map_err(s)?
                != p.capture.expected
            {
                return Err(s(SettingsError::Faulted));
            }
            Ok(stage)
        })();
        observe(r, Boundary::ScopedDefinitionValidation, &result);
        result
    }
    pub fn commit_scoped_settings_definition(
        &mut self,
        p: PreparedScopedSettingsDefinitionV1,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsDefinitionMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ScopedSettingsUpdateV1, Error> {
        if self.faulted {
            let result = Err(s(SettingsError::Faulted));
            observe(r, Boundary::ScopedDefinitionPublication, &result);
            return result;
        }
        self.faulted = true;
        let result = self.stage_scoped_definition(p, c, f, r).map(|stage| {
            let outcome = ScopedSettingsUpdateV1 {
                changed: stage.outcome.changed,
            };
            stage.install(self);
            outcome
        });
        if !result.as_ref().err().is_some_and(|e| e.faulted()) {
            self.faulted = false;
        }
        observe(r, Boundary::ScopedDefinitionPublication, &result);
        result
    }
}
