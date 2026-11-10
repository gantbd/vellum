//! Intrinsic output intent. Metadata mutation runs no codec/provider/schema or source edit.
//! The private encoding helper is invoked explicitly at resource capture/comparison edges.
use super::*;
use settings::basis::{copy_context, labels};
mod arguments;
mod basis;
pub use arguments::*;
pub(super) use basis::BasisPool;
pub use basis::{
    SerializationBasisAvailabilityV1, SerializationBasisId, SerializationBasisLimitsV1,
    SerializationBasisUsageV1, SerializationPoolControlV1, TrustedSerializationBasisFactsV1,
};
#[cfg(test)]
pub(super) mod tests;
const MAX_SAFE: u64 = 9_007_199_254_740_991;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceSerializationProfileV1 {
    StrictUtf8OneBom,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceSerializationV1 {
    pub profile: SourceSerializationProfileV1,
    pub bom: bool,
}
impl SourceSerializationV1 {
    pub(in crate::coordinator) fn encode(
        self,
        source: &Utf8Source,
        r: &mut Recorder,
    ) -> Result<Vec<u8>, SourceError> {
        match self.profile {
            SourceSerializationProfileV1::StrictUtf8OneBom => {
                source.encode_with_bom_observed(self.bom, r)
            }
        }
    }
    pub(in crate::coordinator) fn encoded_len(self, source: &Utf8Source) -> Option<usize> {
        match self.profile {
            SourceSerializationProfileV1::StrictUtf8OneBom => source.encoded_len_with_bom(self.bom),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceSerializationStateV1 {
    pub generation: u64,
    pub serialization: SourceSerializationV1,
}
impl SourceSerializationStateV1 {
    pub(super) fn initial(bom: bool) -> Self {
        Self {
            generation: 1,
            serialization: SourceSerializationV1 {
                profile: SourceSerializationProfileV1::StrictUtf8OneBom,
                bom,
            },
        }
    }
    pub(in crate::coordinator) fn valid(self) -> bool {
        self.generation > 0 && self.generation <= MAX_SAFE
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceSerializationLimitsV1 {
    pub max_dependencies: usize,
    pub max_label_bytes: usize,
    /// Native logical descriptor/plan bytes, not the reference JSON/wire measure.
    pub max_request_bytes: usize,
    pub max_plan_bytes: usize,
    pub max_work: usize,
}
pub struct SourceSerializationRequestV1<'a> {
    pub document: &'a DocumentId,
    pub expected: u64,
    pub serialization: SourceSerializationV1,
    pub dependencies: &'a ReadDependencyRequestV3,
}
/// No caller-supplied target or generation: the opaque basis supplies both privately.
pub struct ScopedSourceSerializationRequestV1<'a> {
    pub basis: &'a SerializationBasisId,
    pub serialization: SourceSerializationV1,
    pub dependencies: &'a ReadDependencyRequestV3,
}
/// Normalized trusted host premises, not requester-supplied permission claims.
pub struct TrustedSerializationFactsV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub document: &'a DocumentId,
    pub profile: SourceSerializationLimitsV1,
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub interpretation_current: Option<bool>,
    pub companions_ready: Option<bool>,
    pub settings_writable: Option<bool>,
    pub resources_ready: Option<bool>,
    pub write_allowed: Option<bool>,
    pub required_reads: &'a [ReadDependencyV3],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceSerializationErrorV1 {
    InvalidVersion,
    Unavailable,
    Stale,
    Limit,
    Capacity,
    VersionExhausted,
    Faulted,
    Dependency(CoordinatorError),
}
impl SourceSerializationErrorV1 {
    pub(super) fn faulted(self) -> bool {
        matches!(
            self,
            Self::Faulted
                | Self::Dependency(
                    CoordinatorError::Faulted
                        | CoordinatorError::History(HistoryError::Faulted)
                        | CoordinatorError::Annotation(AnnotationError::Faulted)
                )
        )
    }
}
impl From<CoordinatorError> for SourceSerializationErrorV1 {
    fn from(e: CoordinatorError) -> Self {
        match e {
            CoordinatorError::Faulted => Self::Faulted,
            CoordinatorError::Unavailable => Self::Unavailable,
            CoordinatorError::Stale => Self::Stale,
            CoordinatorError::Limit => Self::Limit,
            CoordinatorError::Capacity => Self::Capacity,
            CoordinatorError::VersionExhausted => Self::VersionExhausted,
            other => Self::Dependency(other),
        }
    }
}
type Error = SourceSerializationErrorV1;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceSerializationUpdateV1 {
    pub changed: bool,
}
pub(super) struct SerializationRegistration {
    authority: String,
    engine: String,
    limits: SourceSerializationLimitsV1,
}
impl std::fmt::Debug for SerializationRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SerializationRegistration(<redacted>)")
    }
}
pub(super) struct SerializationCapture {
    pub(super) basis: Option<SerializationBasisId>,
    pub(super) owner: Arc<()>,
    pub(super) context: SettingsExecutionContextV1,
    pub(super) document: DocumentId,
    expected: u64,
    serialization: SourceSerializationV1,
    limits: SourceSerializationLimitsV1,
    requested_budget: ReadDependencyBudget,
    caller: read_dependency::ReadCapture,
    required: read_dependency::ReadCapture,
    reads: read_dependency::ReadCapture,
    bytes: usize,
}
impl std::fmt::Debug for SerializationCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SerializationCapture(<redacted>)")
    }
}
impl SerializationCapture {
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.document == *document
            || self.caller.refers_to_document(document)
            || self.required.refers_to_document(document)
            || self.reads.refers_to_document(document)
    }
}
pub struct PreparedSourceSerializationV1 {
    pub(super) capture: Arc<SerializationCapture>,
    seal: Arc<SerializationCapture>,
    before: SourceSerializationStateV1,
    reads: read_dependency::ReadCapture,
    read_count: usize,
}
impl std::fmt::Debug for PreparedSourceSerializationV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreparedSourceSerializationV1(<redacted>)")
    }
}
impl PreparedSourceSerializationV1 {
    pub(super) fn retained_bytes(&self) -> Result<usize, Error> {
        self.capture
            .bytes
            .checked_add(std::mem::size_of::<Self>())
            .and_then(|n| n.checked_add(std::mem::size_of::<Arc<SerializationCapture>>()))
            .and_then(|n| {
                self.reads.logical_bytes().and_then(|m| {
                    n.checked_add(m - std::mem::size_of::<read_dependency::ReadCapture>())
                })
            })
            .ok_or(Error::Limit)
    }
}
pub(super) struct SerializationStage {
    document: DocumentId,
    candidate: Option<SourceSerializationStateV1>,
    generation: u64,
    pub(super) outcome: SourceSerializationUpdateV1,
}
impl SerializationStage {
    pub(super) fn install(self, owner: &mut SourceCoordinator) {
        if let Some(state) = self.candidate {
            owner.documents[self.document.slot]
                .as_mut()
                .expect("validated serialization target")
                .serialization = state;
            owner.generation = self.generation;
        }
    }
}
pub(super) fn observe<T>(r: &mut Recorder, boundary: Boundary, result: &Result<T, Error>) {
    r.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(e) if e.faulted() => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time: None,
    });
}
fn copy_label(value: &str) -> Result<String, Error> {
    let mut text = String::new();
    text.try_reserve_exact(value.len())
        .map_err(|_| Error::Capacity)?;
    text.push_str(value);
    Ok(text)
}
fn read_error(error: ReadDependencyError) -> Error {
    Error::Dependency(error.into())
}
fn valid_context(c: &SettingsExecutionContextV1, limits: SourceSerializationLimitsV1) -> bool {
    limits.max_work >= 7
        && labels(c)
            .iter()
            .all(|s| !s.is_empty() && s.len() <= limits.max_label_bytes)
}
impl SerializationCapture {
    pub(super) fn replay_matches(
        &self,
        request: &SourceSerializationRequestV1<'_>,
        c: &SettingsExecutionContextV1,
    ) -> Result<bool, Error> {
        if self.basis.is_some()
            || self.document != *request.document
            || self.expected != request.expected
            || self.serialization != request.serialization
            || self.context != *c
            || self.requested_budget != request.dependencies.budget
            || self.caller.predicates.len() != request.dependencies.predicates.len()
        {
            return Ok(false);
        }
        let caller = read_dependency::ReadCapture::configuration_slice(
            &request.dependencies.predicates,
            self.caller.budget,
        )
        .map_err(read_error)?;
        Ok(caller == self.caller)
    }
    pub(super) fn scoped_replay_matches(
        &self,
        request: &ScopedSourceSerializationRequestV1<'_>,
        c: &SettingsExecutionContextV1,
    ) -> Result<bool, Error> {
        if self.basis.as_ref() != Some(request.basis)
            || self.serialization != request.serialization
            || self.context != *c
            || self.requested_budget != request.dependencies.budget
            || self.caller.predicates.len() != request.dependencies.predicates.len()
        {
            return Ok(false);
        }
        let reads = read_dependency::ReadCapture::configuration_slice(
            &request.dependencies.predicates,
            self.caller.budget,
        )
        .map_err(read_error)?;
        Ok(reads == self.caller)
    }
}
impl SourceCoordinator {
    pub fn configure_source_serialization(
        &mut self,
        authority: &str,
        engine: &str,
        limits: SourceSerializationLimitsV1,
        r: &mut Recorder,
    ) -> Result<bool, Error> {
        let result = (|| {
            self.ensure_operational()?;
            if [authority, engine]
                .iter()
                .any(|s| s.is_empty() || s.len() > limits.max_label_bytes)
            {
                return Err(Error::Unavailable);
            }
            if let Some(old) = &self.serialization_profile {
                return if old.authority == authority && old.engine == engine && old.limits == limits
                {
                    Ok(false)
                } else {
                    Err(Error::Unavailable)
                };
            }
            self.ensure_admission_unfenced()?;
            self.serialization_profile = Some(SerializationRegistration {
                authority: copy_label(authority)?,
                engine: copy_label(engine)?,
                limits,
            });
            Ok(true)
        })();
        observe(r, Boundary::SerializationConfiguration, &result);
        result
    }
    /// Privileged current intrinsic metadata, not an ordinary recipient format basis.
    pub fn source_serialization(
        &self,
        document: &DocumentId,
        r: &mut Recorder,
    ) -> Result<SourceSerializationStateV1, Error> {
        let result = (|| {
            self.ensure_operational()?;
            let state = self.document(document)?.serialization;
            if !state.valid() {
                return Err(Error::Faulted);
            }
            Ok(state)
        })();
        observe(r, Boundary::SerializationQuery, &result);
        result
    }
    fn serialization_facts_bound(
        &self,
        document: &DocumentId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationFactsV1<'_>,
    ) -> bool {
        self.serialization_profile.as_ref().is_some_and(|p| {
            p.limits == f.profile
                && p.authority == c.authority
                && p.engine == c.engine
                && valid_context(c, p.limits)
                && f.context == c
                && f.document == document
        })
    }
    fn serialization_facts_current(
        &self,
        document: &DocumentId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationFactsV1<'_>,
    ) -> bool {
        self.serialization_facts_bound(document, c, f)
            && [
                f.authorized,
                f.scope,
                f.continuous,
                f.live,
                f.not_stopped,
                f.interpretation_current,
                f.companions_ready,
                f.settings_writable,
                f.resources_ready,
                f.write_allowed,
            ]
            .into_iter()
            .all(|value| value == Some(true))
    }
    pub(super) fn capture_source_serialization(
        &self,
        request: &SourceSerializationRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationFactsV1<'_>,
    ) -> Result<Arc<SerializationCapture>, Error> {
        self.ensure_ordinary_admission()?;
        if !self.serialization_facts_current(request.document, c, f) {
            return Err(Error::Unavailable);
        }
        if request.expected == 0 || request.expected > MAX_SAFE {
            return Err(Error::InvalidVersion);
        }
        let limits = f.profile;
        if request.dependencies.predicates.len() > limits.max_dependencies
            || f.required_reads.len() > limits.max_dependencies
        {
            return Err(Error::Limit);
        }
        let parent = self.settings_job_parent_limit();
        let count = request
            .dependencies
            .predicates
            .len()
            .checked_add(f.required_reads.len())
            .ok_or(Error::Limit)?;
        let early = read_dependency::ReadCapture::size_for_predicates(count).ok_or(Error::Limit)?;
        if early > limits.max_request_bytes || early > parent {
            return Err(Error::Limit);
        }
        let budget = ReadDependencyBudget {
            max_predicates: request
                .dependencies
                .budget
                .max_predicates
                .min(limits.max_dependencies),
            max_work: request.dependencies.budget.max_work.min(limits.max_work),
        };
        let caller = read_dependency::ReadCapture::configuration_slice(
            &request.dependencies.predicates,
            budget,
        )
        .map_err(read_error)?;
        let required = read_dependency::ReadCapture::configuration_slice(f.required_reads, budget)
            .map_err(read_error)?;
        let reads = caller.conjoin(&required).map_err(read_error)?;
        let mut bytes = std::mem::size_of::<SerializationCapture>();
        for label in labels(c) {
            bytes = bytes.checked_add(label.len()).ok_or(Error::Limit)?;
        }
        for read in [&caller, &required, &reads] {
            bytes = bytes
                .checked_add(
                    read.logical_bytes().ok_or(Error::Limit)?
                        - std::mem::size_of::<read_dependency::ReadCapture>(),
                )
                .ok_or(Error::Limit)?;
        }
        if bytes > limits.max_request_bytes
            || bytes
                .checked_add(std::mem::size_of::<Arc<SerializationCapture>>())
                .is_none_or(|n| n > parent)
        {
            return Err(Error::Limit);
        }
        let context = copy_context(c).map_err(|e| {
            if e == crate::settings::SettingsError::Capacity {
                Error::Capacity
            } else {
                Error::Faulted
            }
        })?;
        Ok(Arc::new(SerializationCapture {
            basis: None,
            owner: self.namespace.clone(),
            context,
            document: request.document.clone(),
            expected: request.expected,
            serialization: request.serialization,
            limits,
            requested_budget: request.dependencies.budget,
            caller,
            required,
            reads,
            bytes,
        }))
    }
    pub(super) fn prepare_serialization_capture(
        &self,
        capture: Arc<SerializationCapture>,
        r: &mut Recorder,
    ) -> Result<PreparedSourceSerializationV1, Error> {
        self.ensure_ordinary_admission()?;
        let before = self.document(&capture.document)?.serialization;
        if !before.valid() {
            return Err(Error::Faulted);
        }
        if before.generation != capture.expected {
            return Err(Error::Stale);
        }
        self.check_owned_read_capture(&capture.reads, r)
            .map_err(Error::Dependency)?;
        let reads = capture.reads.copy_owned().map_err(read_error)?;
        let p = PreparedSourceSerializationV1 {
            seal: capture.clone(),
            capture,
            before,
            read_count: reads.predicates.len(),
            reads,
        };
        self.serialization_plan_bound(&p)?;
        if before.serialization != p.capture.serialization {
            self.next_serialization_stage(&p)?;
        }
        Ok(p)
    }
    fn serialization_plan_bound(&self, p: &PreparedSourceSerializationV1) -> Result<(), Error> {
        let bytes = p.retained_bytes()?;
        if bytes > p.capture.limits.max_plan_bytes || bytes > self.settings_job_parent_limit() {
            return Err(Error::Limit);
        }
        Ok(())
    }
    fn next_serialization_stage(
        &self,
        p: &PreparedSourceSerializationV1,
    ) -> Result<SerializationStage, Error> {
        let changed = p.before.serialization != p.capture.serialization;
        let generation = if changed {
            self.next_publication_generation(None)?
        } else {
            self.generation
        };
        let candidate = if changed {
            Some(SourceSerializationStateV1 {
                generation: p
                    .before
                    .generation
                    .checked_add(1)
                    .filter(|n| *n <= MAX_SAFE)
                    .ok_or(Error::VersionExhausted)?,
                serialization: p.capture.serialization,
            })
        } else {
            None
        };
        Ok(SerializationStage {
            document: p.capture.document.clone(),
            candidate,
            generation,
            outcome: SourceSerializationUpdateV1 { changed },
        })
    }
    pub fn prepare_source_serialization(
        &mut self,
        request: &SourceSerializationRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<PreparedSourceSerializationV1, Error> {
        let result = self
            .capture_source_serialization(request, c, f)
            .and_then(|capture| self.prepare_serialization_capture(capture, r));
        if result.as_ref().err().is_some_and(|e| e.faulted()) {
            self.faulted = true;
        }
        observe(r, Boundary::SerializationPreparation, &result);
        result
    }
    pub(super) fn stage_source_serialization(
        &self,
        p: PreparedSourceSerializationV1,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SerializationStage, Error> {
        if p.capture.basis.is_some() {
            let result = Err(Error::Unavailable);
            observe(r, Boundary::SerializationValidation, &result);
            return result;
        }
        self.stage_serialization_native(p, c, f, r)
    }
    fn stage_serialization_native(
        &self,
        p: PreparedSourceSerializationV1,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SerializationStage, Error> {
        let result = (|| {
            // Commit already owns the integrity latch. The independent shutdown
            // fence must also reject unchanged plans, not only changed roots.
            self.ensure_admission_unfenced()?;
            if !Arc::ptr_eq(&p.capture, &p.seal)
                || p.read_count != p.capture.reads.predicates.len()
                || p.reads != p.capture.reads
            {
                return Err(Error::Faulted);
            }
            if !Arc::ptr_eq(&p.capture.owner, &self.namespace)
                || p.capture.context != *c
                || !self.serialization_facts_current(&p.capture.document, c, f)
            {
                return Err(Error::Unavailable);
            }
            self.serialization_plan_bound(&p)?;
            if f.required_reads.len() != p.capture.required.predicates.len() {
                return Err(Error::Stale);
            }
            let required = read_dependency::ReadCapture::configuration_slice(
                f.required_reads,
                p.capture.required.budget,
            )
            .map_err(read_error)?;
            if required != p.capture.required {
                return Err(Error::Stale);
            }
            let current = self.document(&p.capture.document)?.serialization;
            if !current.valid() {
                return Err(Error::Faulted);
            }
            if current.generation != p.capture.expected {
                return Err(Error::Stale);
            }
            if current != p.before {
                return Err(Error::Faulted);
            }
            let check = self.validate_read_capture(&p.reads);
            super::observe(r, Boundary::ReadDependencyValidation, &check);
            check.map_err(Error::Dependency)?;
            self.next_serialization_stage(&p)
        })();
        observe(r, Boundary::SerializationValidation, &result);
        result
    }
    pub fn commit_source_serialization(
        &mut self,
        p: PreparedSourceSerializationV1,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourceSerializationUpdateV1, Error> {
        if self.faulted {
            let result = Err(Error::Faulted);
            observe(r, Boundary::SerializationPublication, &result);
            return result;
        }
        self.faulted = true;
        let result = self.stage_source_serialization(p, c, f, r).map(|stage| {
            let outcome = stage.outcome;
            stage.install(self);
            outcome
        });
        if !result.as_ref().err().is_some_and(|e| e.faulted()) {
            self.faulted = false;
        }
        observe(r, Boundary::SerializationPublication, &result);
        result
    }
    pub(super) fn prepare_scoped_serialization_inner(
        &mut self,
        request: &ScopedSourceSerializationRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationBasisFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<PreparedSourceSerializationV1, Error> {
        self.ensure_operational()?;
        if f.host_basis_allowed != Some(true) {
            return Err(Error::Unavailable);
        }
        let (document, expected) = self.serialization_basis_condition(request.basis, c)?;
        let native = SourceSerializationRequestV1 {
            document: &document,
            expected,
            serialization: request.serialization,
            dependencies: request.dependencies,
        };
        let mut capture = self.capture_source_serialization(&native, c, f.native)?;
        Arc::get_mut(&mut capture)
            .expect("unshared scoped capture")
            .basis = Some(request.basis.clone());
        // Complete native semantic/read/whole-wrapper preflight precedes stateful loss.
        let p = self.prepare_serialization_capture(capture, r)?;
        self.current_serialization_basis(request.basis, c, f)?;
        Ok(p)
    }
    pub fn prepare_scoped_source_serialization(
        &mut self,
        request: &ScopedSourceSerializationRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationBasisFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<PreparedSourceSerializationV1, Error> {
        let result = self.prepare_scoped_serialization_inner(request, c, f, r);
        if result.as_ref().err().is_some_and(|e| e.faulted()) {
            self.faulted = true;
        }
        observe(r, Boundary::ScopedSerializationPreparation, &result);
        result
    }
    pub(super) fn stage_scoped_source_serialization(
        &mut self,
        p: PreparedSourceSerializationV1,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationBasisFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SerializationStage, Error> {
        let result = (|| {
            if f.host_basis_allowed != Some(true) {
                return Err(Error::Unavailable);
            }
            let basis = p.capture.basis.clone().ok_or(Error::Unavailable)?;
            let document = p.capture.document.clone();
            let expected = p.capture.expected;
            let stage = self.stage_serialization_native(p, c, f.native, r)?;
            self.current_serialization_basis(&basis, c, f)?;
            if self.serialization_basis_condition(&basis, c)? != (document, expected) {
                return Err(Error::Faulted);
            }
            Ok(stage)
        })();
        observe(r, Boundary::ScopedSerializationValidation, &result);
        result
    }
    pub fn commit_scoped_source_serialization(
        &mut self,
        p: PreparedSourceSerializationV1,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationBasisFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourceSerializationUpdateV1, Error> {
        if self.faulted {
            let result = Err(Error::Faulted);
            observe(r, Boundary::ScopedSerializationPublication, &result);
            return result;
        }
        self.faulted = true;
        let result = self
            .stage_scoped_source_serialization(p, c, f, r)
            .map(|stage| {
                let outcome = stage.outcome;
                stage.install(self);
                outcome
            });
        if !result.as_ref().err().is_some_and(|e| e.faulted()) {
            self.faulted = false;
        }
        observe(r, Boundary::ScopedSerializationPublication, &result);
        result
    }
}
