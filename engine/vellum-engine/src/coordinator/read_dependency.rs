//! Closed original-root predicates, not semantic discovery or observation grants.
use super::*;
#[cfg(test)]
mod tests;
use crate::settings::{SettingDomainV1, SettingsInterpretationEpochsV1};
#[cfg(test)]
pub(in crate::coordinator) use tests::setup as configuration_read_test_fixture;
#[derive(Clone, PartialEq, Eq)]
pub enum ReadDependencyV3 {
    Base(ReadDependencyV2),
    ConfigurationDomain {
        target: SettingsTargetId,
        domain: SettingDomainV1,
        expected: u64,
        interpretation: SettingsInterpretationEpochsV1,
    },
    ConfigurationState {
        registry: SettingsRegistryId,
        expected: u64,
        interpretation: SettingsInterpretationEpochsV1,
    },
}
impl std::fmt::Debug for ReadDependencyV3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReadDependencyV3(<redacted>)")
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadDependencyRequestV3 {
    pub predicates: Vec<ReadDependencyV3>,
    pub budget: ReadDependencyBudget,
}
#[derive(Clone, PartialEq, Eq)]
pub enum ReadDependencyV1 {
    Source(DocumentVersion),
    AnnotationSet {
        document: DocumentId,
        expected: u64,
    },
    AnnotationBody {
        annotation: AnnotationId,
        expected: u64,
    },
    AnnotationTarget {
        annotation: AnnotationId,
        expected: u64,
    },
}
impl std::fmt::Debug for ReadDependencyV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReadDependencyV1(<redacted>)")
    }
}
#[derive(Clone, PartialEq, Eq)]
pub enum ReadDependencyV2 {
    Base(ReadDependencyV1),
    WorkspaceMembership {
        workspace: WorkspaceId,
        expected: u64,
    },
}
impl std::fmt::Debug for ReadDependencyV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReadDependencyV2(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadDependencyBudget {
    /// Includes source reads already present in the compound source intent.
    pub max_predicates: usize,
    /// Ordering and validation steps per pass; not elapsed CPU time.
    pub max_work: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadDependencyRequest {
    pub predicates: Vec<ReadDependencyV1>,
    pub budget: ReadDependencyBudget,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadDependencyRequestV2 {
    pub predicates: Vec<ReadDependencyV2>,
    pub budget: ReadDependencyBudget,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReadProfile {
    Configuration,
    SourceAnnotation,
    Workspace,
}
#[derive(Clone, Copy)]
pub(super) enum ReadInput<'a> {
    V3(&'a ReadDependencyRequestV3),
    V1(&'a ReadDependencyRequest),
    V2(&'a ReadDependencyRequestV2),
}
impl ReadInput<'_> {
    pub(super) fn copy(
        self,
        source: &[DocumentVersion],
    ) -> Result<ReadCapture, ReadDependencyError> {
        match self {
            Self::V1(request) => ReadCapture::copy(source, request),
            Self::V2(request) => ReadCapture::copy_parts(
                source,
                request
                    .predicates
                    .iter()
                    .cloned()
                    .map(ReadDependencyV3::Base),
                request.budget,
                ReadProfile::Workspace,
            ),
            Self::V3(request) => ReadCapture::copy_parts(
                source,
                request.predicates.iter().cloned(),
                request.budget,
                ReadProfile::Configuration,
            ),
        }
    }
    pub(super) fn retained_bytes(self, source: &[DocumentVersion]) -> Option<usize> {
        match self {
            Self::V1(request) => ReadCapture::retained_bytes(source, request),
            Self::V2(request) => ReadCapture::retained_size(source.len(), request.predicates.len()),
            Self::V3(request) => ReadCapture::retained_size(source.len(), request.predicates.len()),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadDependencyError {
    InvalidVersion,
    DuplicateSubject,
    Limit,
    Capacity,
}
impl From<ReadDependencyError> for CoordinatorError {
    fn from(value: ReadDependencyError) -> Self {
        Self::ReadDependency(value)
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct ReadCapture {
    profile: ReadProfile,
    pub(super) predicates: Vec<ReadDependencyV3>,
    pub(super) budget: ReadDependencyBudget,
}
impl ReadCapture {
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.predicates.iter().any(|p| match p {
            ReadDependencyV3::Base(ReadDependencyV2::Base(p)) => match p {
                ReadDependencyV1::Source(v) => v.document == *document,
                ReadDependencyV1::AnnotationSet { document: d, .. } => d == document,
                ReadDependencyV1::AnnotationBody { annotation, .. }
                | ReadDependencyV1::AnnotationTarget { annotation, .. } => {
                    annotation.document == *document
                }
            },
            ReadDependencyV3::Base(ReadDependencyV2::WorkspaceMembership { .. })
            | ReadDependencyV3::ConfigurationDomain { .. }
            | ReadDependencyV3::ConfigurationState { .. } => false,
        })
    }
}
impl ReadDependencyV2 {
    fn key(&self) -> (u8, usize, usize) {
        match self {
            Self::Base(predicate) => predicate.key(),
            Self::WorkspaceMembership { workspace, .. } => (4, workspace.slot, 0),
        }
    }
}
impl ReadDependencyV3 {
    fn key(&self) -> (u8, usize, usize) {
        match self {
            Self::Base(p) => p.key(),
            Self::ConfigurationDomain { target, domain, .. } => (
                5,
                target.native_identity(),
                match domain {
                    SettingDomainV1::Source => 0,
                    SettingDomainV1::Serialization => 1,
                    SettingDomainV1::Analysis => 2,
                },
            ),
            Self::ConfigurationState { .. } => (6, 0, 0),
        }
    }
    fn valid_versions(&self) -> bool {
        let (expected, epochs) = match self {
            Self::Base(_) => return true,
            Self::ConfigurationDomain {
                expected,
                interpretation,
                ..
            }
            | Self::ConfigurationState {
                expected,
                interpretation,
                ..
            } => (*expected, *interpretation),
        };
        [
            expected,
            epochs.schema,
            epochs.restriction,
            epochs.association,
        ]
        .into_iter()
        .all(|v| v > 0 && v <= 9_007_199_254_740_991)
    }
}
fn step(work: &mut usize) -> Result<(), ReadDependencyError> {
    *work = work.checked_sub(1).ok_or(ReadDependencyError::Limit)?;
    Ok(())
}
impl ReadDependencyV1 {
    // Stable native opaque-subject order, not a selected public wire identifier.
    fn key(&self) -> (u8, usize, usize) {
        match self {
            Self::Source(v) => (0, v.document.slot, 0),
            Self::AnnotationSet { document, .. } => (1, document.slot, 0),
            Self::AnnotationBody { annotation, .. } => {
                (2, annotation.document.slot, annotation.slot)
            }
            Self::AnnotationTarget { annotation, .. } => {
                (3, annotation.document.slot, annotation.slot)
            }
        }
    }
}
impl ReadCapture {
    pub(super) fn configuration_slice(
        input: &[ReadDependencyV3],
        budget: ReadDependencyBudget,
    ) -> Result<Self, ReadDependencyError> {
        Self::copy_parts(
            &[],
            input.iter().cloned(),
            budget,
            ReadProfile::Configuration,
        )
    }
    pub(super) fn size_for_predicates(count: usize) -> Option<usize> {
        Self::retained_size(0, count)
    }
    pub(super) fn conjoin(&self, other: &Self) -> Result<Self, ReadDependencyError> {
        if !self.configuration_profile()
            || !other.configuration_profile()
            || self.budget != other.budget
        {
            return Err(ReadDependencyError::InvalidVersion);
        }
        let count = self
            .predicates
            .len()
            .checked_add(other.predicates.len())
            .ok_or(ReadDependencyError::Limit)?
            .min(self.budget.max_predicates);
        let mut predicates = Vec::new();
        predicates
            .try_reserve_exact(count)
            .map_err(|_| ReadDependencyError::Capacity)?;
        let (mut a, mut b) = (0, 0);
        let mut work = self.budget.max_work;
        while a < self.predicates.len() || b < other.predicates.len() {
            step(&mut work)?;
            let value = match (self.predicates.get(a), other.predicates.get(b)) {
                (Some(x), Some(y)) if x.key() == y.key() => {
                    if x != y {
                        return Err(ReadDependencyError::DuplicateSubject);
                    }
                    a += 1;
                    b += 1;
                    x
                }
                (Some(x), Some(y)) if x.key() < y.key() => {
                    a += 1;
                    x
                }
                (Some(_), Some(y)) => {
                    b += 1;
                    y
                }
                (Some(x), None) => {
                    a += 1;
                    x
                }
                (None, Some(y)) => {
                    b += 1;
                    y
                }
                (None, None) => unreachable!("remaining read predicate"),
            };
            if predicates.len() >= self.budget.max_predicates {
                return Err(ReadDependencyError::Limit);
            }
            predicates.push(value.clone());
        }
        Ok(Self {
            predicates,
            profile: self.profile,
            budget: self.budget,
        })
    }
    pub(super) fn logical_bytes(&self) -> Option<usize> {
        Self::retained_size(0, self.predicates.len())
    }
    pub(super) fn configuration_profile(&self) -> bool {
        self.profile == ReadProfile::Configuration
    }
    pub(super) fn retained_bytes(
        source: &[DocumentVersion],
        request: &ReadDependencyRequest,
    ) -> Option<usize> {
        Self::retained_size(source.len(), request.predicates.len())
    }
    fn retained_size(source: usize, extra: usize) -> Option<usize> {
        std::mem::size_of::<Self>().checked_add(
            source
                .checked_add(extra)?
                .checked_mul(std::mem::size_of::<ReadDependencyV3>())?,
        )
    }
    pub(super) fn copy(
        source: &[DocumentVersion],
        request: &ReadDependencyRequest,
    ) -> Result<Self, ReadDependencyError> {
        Self::copy_parts(
            source,
            request
                .predicates
                .iter()
                .cloned()
                .map(|p| ReadDependencyV3::Base(ReadDependencyV2::Base(p))),
            request.budget,
            ReadProfile::SourceAnnotation,
        )
    }
    pub(super) fn copy_owned(&self) -> Result<Self, ReadDependencyError> {
        Self::copy_parts(
            &[],
            self.predicates.iter().cloned(),
            self.budget,
            self.profile,
        )
    }
    fn copy_parts(
        source: &[DocumentVersion],
        extra: impl ExactSizeIterator<Item = ReadDependencyV3>,
        budget: ReadDependencyBudget,
        profile: ReadProfile,
    ) -> Result<Self, ReadDependencyError> {
        let count = source
            .len()
            .checked_add(extra.len())
            .filter(|n| *n <= budget.max_predicates)
            .ok_or(ReadDependencyError::Limit)?;
        let mut work = budget.max_work;
        if count > work {
            return Err(ReadDependencyError::Limit);
        }
        let mut predicates = Vec::new();
        predicates
            .try_reserve_exact(count)
            .map_err(|_| ReadDependencyError::Capacity)?;
        predicates.extend(
            source.iter().cloned().map(|v| {
                ReadDependencyV3::Base(ReadDependencyV2::Base(ReadDependencyV1::Source(v)))
            }),
        );
        predicates.extend(extra);
        for i in 1..predicates.len() {
            let mut j = i;
            while j > 0 {
                step(&mut work)?;
                if predicates[j - 1].key() <= predicates[j].key() {
                    break;
                }
                predicates.swap(j - 1, j);
                j -= 1;
            }
        }
        for i in 0..predicates.len() {
            step(&mut work)?;
            if !predicates[i].valid_versions() {
                return Err(ReadDependencyError::InvalidVersion);
            }
            if i > 0 && predicates[i - 1].key() == predicates[i].key() {
                return Err(ReadDependencyError::DuplicateSubject);
            }
        }
        Ok(Self {
            predicates,
            budget,
            profile,
        })
    }
}
impl SourceCoordinator {
    pub fn check_read_dependencies_v3(
        &self,
        source: &[DocumentVersion],
        request: &ReadDependencyRequestV3,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let result = self
            .prepare_read_capture_profile(source, ReadInput::V3(request), r)
            .map(|_| ());
        observe(r, Boundary::ReadDependencyAdvice, &result);
        result
    }
    pub fn prepare_compound_with_reads_v3(
        &mut self,
        intent: &CompoundIntent,
        request: &ReadDependencyRequestV3,
        r: &mut Recorder,
    ) -> Result<PreparedTransaction, CoordinatorError> {
        self.prepare_compound_read_profile(intent, ReadInput::V3(request), None, r)
    }
    pub fn prepare_scoped_compound_with_reads_v3(
        &mut self,
        intent: &CompoundIntent,
        request: &ReadDependencyRequestV3,
        scopes: &[SourceOperationScope],
        budget: SourceScopeBudget,
        r: &mut Recorder,
    ) -> Result<PreparedTransaction, CoordinatorError> {
        self.prepare_compound_read_profile(
            intent,
            ReadInput::V3(request),
            Some((scopes, budget)),
            r,
        )
    }
    pub(super) fn check_owned_read_capture(
        &self,
        capture: &ReadCapture,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let result = self.validate_read_capture(capture);
        observe(r, Boundary::ReadDependencyPreparation, &result);
        result
    }
    pub(super) fn validate_read_capture(
        &self,
        capture: &ReadCapture,
    ) -> Result<(), CoordinatorError> {
        if capture.predicates.len() > capture.budget.max_predicates {
            return Err(ReadDependencyError::Limit.into());
        }
        let mut work = capture.budget.max_work;
        for (i, predicate) in capture.predicates.iter().enumerate() {
            step(&mut work)?;
            if !predicate.valid_versions() {
                return Err(ReadDependencyError::InvalidVersion.into());
            }
            if i > 0 && capture.predicates[i - 1].key() >= predicate.key() {
                return Err(ReadDependencyError::DuplicateSubject.into());
            }
            let equal = match predicate {
                ReadDependencyV3::ConfigurationDomain {
                    target,
                    domain,
                    expected,
                    interpretation,
                } => {
                    if capture.profile != ReadProfile::Configuration {
                        return Err(CoordinatorError::Faulted);
                    }
                    let state = self
                        .settings
                        .as_ref()
                        .ok_or(CoordinatorError::Unavailable)?;
                    let target = self.owned_settings_target(state, target).map_err(|e| {
                        if e == crate::settings::SettingsError::Faulted {
                            CoordinatorError::Faulted
                        } else {
                            CoordinatorError::Unavailable
                        }
                    })?;
                    let generations = state
                        .generations(target)
                        .map_err(|_| CoordinatorError::Faulted)?;
                    let generation = match domain {
                        SettingDomainV1::Source => generations.source,
                        SettingDomainV1::Serialization => generations.serialization,
                        SettingDomainV1::Analysis => generations.analysis,
                    };
                    state.basis().interpretation() == *interpretation && generation == *expected
                }
                ReadDependencyV3::ConfigurationState {
                    registry,
                    expected,
                    interpretation,
                } => {
                    if capture.profile != ReadProfile::Configuration {
                        return Err(CoordinatorError::Faulted);
                    }
                    self.owned_settings_registry(registry)
                        .map_err(|_| CoordinatorError::Unavailable)?;
                    let basis = self
                        .settings
                        .as_ref()
                        .ok_or(CoordinatorError::Unavailable)?
                        .basis();
                    basis.interpretation() == *interpretation
                        && basis.configuration_generation == *expected
                }
                ReadDependencyV3::Base(ReadDependencyV2::WorkspaceMembership {
                    workspace,
                    expected,
                }) => {
                    if capture.profile == ReadProfile::SourceAnnotation {
                        return Err(CoordinatorError::Faulted);
                    }
                    self.workspace_generation(workspace)? == *expected
                }
                ReadDependencyV3::Base(ReadDependencyV2::Base(predicate)) => match predicate {
                    ReadDependencyV1::Source(version) => {
                        self.validate_version(version)?;
                        true
                    }
                    ReadDependencyV1::AnnotationSet { document, expected } => {
                        self.document(document)?.annotation_generation == *expected
                    }
                    ReadDependencyV1::AnnotationBody {
                        annotation,
                        expected,
                    } => self.annotation_dependency_versions(annotation)?.0 == *expected,
                    ReadDependencyV1::AnnotationTarget {
                        annotation,
                        expected,
                    } => self.annotation_dependency_versions(annotation)?.1 == *expected,
                },
            };
            if !equal {
                return Err(CoordinatorError::Stale);
            }
        }
        Ok(())
    }
    fn prepare_read_capture(
        &self,
        source: &[DocumentVersion],
        request: &ReadDependencyRequest,
        r: &mut Recorder,
    ) -> Result<ReadCapture, CoordinatorError> {
        self.prepare_read_capture_profile(source, ReadInput::V1(request), r)
    }
    pub(super) fn prepare_read_capture_profile(
        &self,
        source: &[DocumentVersion],
        input: ReadInput<'_>,
        r: &mut Recorder,
    ) -> Result<ReadCapture, CoordinatorError> {
        let result = (|| {
            self.ensure_operational()?;
            let capture = input.copy(source)?;
            self.validate_read_capture(&capture)?;
            Ok(capture)
        })();
        observe(r, Boundary::ReadDependencyPreparation, &result);
        result
    }
    /// Current trusted-owner advice only; no reservation, mutation or new grant.
    pub fn check_read_dependencies(
        &self,
        source_reads: &[DocumentVersion],
        request: &ReadDependencyRequest,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let result = self
            .prepare_read_capture(source_reads, request, r)
            .map(|_| ());
        observe(r, Boundary::ReadDependencyAdvice, &result);
        result
    }
    pub fn prepare_compound_with_reads(
        &mut self,
        intent: &CompoundIntent,
        request: &ReadDependencyRequest,
        r: &mut Recorder,
    ) -> Result<PreparedTransaction, CoordinatorError> {
        self.prepare_compound_read_profile(intent, ReadInput::V1(request), None, r)
    }
    pub fn prepare_scoped_compound_with_reads(
        &mut self,
        intent: &CompoundIntent,
        request: &ReadDependencyRequest,
        scopes: &[SourceOperationScope],
        budget: SourceScopeBudget,
        r: &mut Recorder,
    ) -> Result<PreparedTransaction, CoordinatorError> {
        self.prepare_compound_read_profile(
            intent,
            ReadInput::V1(request),
            Some((scopes, budget)),
            r,
        )
    }
    pub fn check_read_dependencies_v2(
        &self,
        source: &[DocumentVersion],
        request: &ReadDependencyRequestV2,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let result = self
            .prepare_read_capture_profile(source, ReadInput::V2(request), r)
            .map(|_| ());
        observe(r, Boundary::ReadDependencyAdvice, &result);
        result
    }
    pub fn prepare_compound_with_reads_v2(
        &mut self,
        intent: &CompoundIntent,
        request: &ReadDependencyRequestV2,
        r: &mut Recorder,
    ) -> Result<PreparedTransaction, CoordinatorError> {
        self.prepare_compound_read_profile(intent, ReadInput::V2(request), None, r)
    }
    pub fn prepare_scoped_compound_with_reads_v2(
        &mut self,
        intent: &CompoundIntent,
        request: &ReadDependencyRequestV2,
        scopes: &[SourceOperationScope],
        budget: SourceScopeBudget,
        r: &mut Recorder,
    ) -> Result<PreparedTransaction, CoordinatorError> {
        self.prepare_compound_read_profile(
            intent,
            ReadInput::V2(request),
            Some((scopes, budget)),
            r,
        )
    }
    fn prepare_compound_read_profile(
        &mut self,
        intent: &CompoundIntent,
        input: ReadInput<'_>,
        scope: Option<(&[SourceOperationScope], SourceScopeBudget)>,
        r: &mut Recorder,
    ) -> Result<PreparedTransaction, CoordinatorError> {
        let capture = self.prepare_read_capture_profile(&intent.source.reads, input, r)?;
        let scope = if let Some((scopes, budget)) = scope {
            let result = self.capture_scopes(&intent.source.writes, scopes, budget);
            observe(r, Boundary::SourceScopePreparation, &result);
            Some(result?)
        } else {
            None
        };
        let mut p = self.prepare_compound(&intent.source.writes, &[], &intent.annotations, r)?;
        p.scoped = scope.is_some();
        p.scopes = scope;
        p.reads_required = true;
        p.declared_reads = Some(capture);
        Ok(p)
    }
    pub(super) fn validate_prepared_reads(
        &self,
        prepared: &PreparedTransaction,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let result = match (prepared.reads_required, &prepared.declared_reads) {
            (false, None) => return Ok(()),
            (true, Some(capture)) => self.validate_read_capture(capture),
            _ => Err(CoordinatorError::Faulted),
        };
        observe(r, Boundary::ReadDependencyValidation, &result);
        result
    }
}
