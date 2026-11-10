//! Closed trusted-native behavior facade; not the reference data/wire envelope.
//! No new job, owner, identity, authority, preparation, result or executor.
use super::*;
mod arguments;
pub use arguments::{
    NATIVE_DOCUMENT_LIFECYCLE_PROFILE_V1, NativeDocumentLifecycleArgumentLimitsV1,
    NativeDocumentLifecycleEnvelopeValueV1, NormalizedDocumentLifecycleArgumentsV1,
    normalize_native_document_lifecycle_arguments_v1,
};
#[cfg(test)]
mod tests;

/// Borrowed native requests. The existing owner captures immutable intent.
/// `LiveRelation` covers exactly attach, detach and membership, not UI state.
#[derive(Clone, Copy)]
pub enum NativeDocumentLifecycleCommandV1<'a> {
    Scratch(&'a ScratchDocumentRequestV1<'a>),
    LiveRelation(&'a LiveRelationRequestV1),
    PreparedDisposal(&'a PreparedDisposal),
}
/// Original independent ceilings and trusted premises remain operation-specific.
#[derive(Clone, Copy)]
pub enum NativeDocumentLifecycleAdmissionV1<'a> {
    Scratch(&'a ScratchInvocationAdmissionV1<'a>),
    LiveRelation(&'a LiveRelationAdmissionV1<'a>),
    PreparedDisposal(&'a RetainedDisposalAdmissionV1<'a>),
}
/// Fresh service facts, not captured permission or an implicit authority upgrade.
#[derive(Clone, Copy)]
pub enum NativeDocumentLifecycleServiceV1<'a> {
    Scratch(&'a TrustedScratchFactsV1<'a>),
    LiveRelation(&'a TrustedLiveRelationFactsV1<'a>),
    PreparedDisposal(&'a TrustedRetainedDisposalFactsV1<'a>),
}
/// Explicit profile inclusion; the exhaustive V1 vocabulary is unchanged.
#[derive(Clone, Copy)]
pub enum NativeDocumentLifecycleCommandV2<'a> {
    Existing(NativeDocumentLifecycleCommandV1<'a>),
    ExplicitDisposal(&'a PreparedExplicitDisposalV1),
}
#[derive(Clone, Copy)]
pub enum NativeDocumentLifecycleAdmissionV2<'a> {
    Existing(NativeDocumentLifecycleAdmissionV1<'a>),
    ExplicitDisposal {
        native: &'a RetainedDisposalAdmissionV1<'a>,
        coverage: &'a TrustedDisposalCoverageV1,
    },
}
#[derive(Clone, Copy)]
pub enum NativeDocumentLifecycleServiceV2<'a> {
    Existing(NativeDocumentLifecycleServiceV1<'a>),
    ExplicitDisposal {
        native: &'a TrustedRetainedDisposalFactsV1<'a>,
        coverage: &'a TrustedDisposalCoverageV1,
    },
}
/// Semantic requests for the supported native operations. Callers do not need
/// to retain a prepared disposal artifact to retry the same original intent.
/// Opaque native targets/expectations remain native roles, not decoded strings.
#[derive(Clone, Copy)]
pub enum NativeDocumentLifecycleRequestV1<'a> {
    Scratch(&'a ScratchDocumentRequestV1<'a>),
    LiveRelation(&'a LiveRelationRequestV1),
    ExplicitDisposal(&'a ExplicitDisposalRequestV1),
}
impl SourceCoordinator {
    /// One data-first entry over scratch, attach/detach/membership and explicit
    /// disposal. Existing owners capture intent and enforce independent rights.
    /// Fresh disposal preparation is funded; replay never prepares a later root.
    pub fn submit_native_document_lifecycle_request_v1(
        &mut self,
        id: &InvocationId,
        request: NativeDocumentLifecycleRequestV1<'_>,
        admission: NativeDocumentLifecycleAdmissionV2<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        let timing = r.begin_service_timing();
        let result = match (request, admission) {
            (
                NativeDocumentLifecycleRequestV1::Scratch(q),
                NativeDocumentLifecycleAdmissionV2::Existing(
                    NativeDocumentLifecycleAdmissionV1::Scratch(a),
                ),
            ) => self.submit_scratch_v1(id, q, a, r),
            (
                NativeDocumentLifecycleRequestV1::LiveRelation(q),
                NativeDocumentLifecycleAdmissionV2::Existing(
                    NativeDocumentLifecycleAdmissionV1::LiveRelation(a),
                ),
            ) => self.submit_live_relation_v1(id, q, a, r),
            (
                NativeDocumentLifecycleRequestV1::ExplicitDisposal(q),
                NativeDocumentLifecycleAdmissionV2::ExplicitDisposal { native, coverage },
            ) => self.submit_explicit_disposal_request_v1(id, q, native, coverage, r),
            _ => Err(InvocationError::Unavailable),
        };
        capture_timed(
            r,
            Boundary::DocumentLifecycleFacadeAdmission,
            &result,
            timing.finish(),
        );
        result
    }
    /// Delegates admission to the same original native invocation owner.
    /// Mismatched command/premise families refuse before any native preparation.
    /// Prepared disposal deliberately does not decode retention/discard data.
    pub fn submit_native_document_lifecycle_v1(
        &mut self,
        id: &InvocationId,
        command: NativeDocumentLifecycleCommandV1<'_>,
        admission: NativeDocumentLifecycleAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_native_document_lifecycle_v2(
            id,
            NativeDocumentLifecycleCommandV2::Existing(command),
            NativeDocumentLifecycleAdmissionV2::Existing(admission),
            r,
        )
    }
    /// Adds explicit disposal without changing V1 or installing another owner.
    /// Admission retains all original applicable coverage and native checks.
    pub fn submit_native_document_lifecycle_v2(
        &mut self,
        id: &InvocationId,
        command: NativeDocumentLifecycleCommandV2<'_>,
        admission: NativeDocumentLifecycleAdmissionV2<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        let timing = r.begin_service_timing();
        let result = match (command, admission) {
            (
                NativeDocumentLifecycleCommandV2::Existing(q),
                NativeDocumentLifecycleAdmissionV2::Existing(a),
            ) => self.submit_native_document_lifecycle_existing(id, q, a, r),
            (
                NativeDocumentLifecycleCommandV2::ExplicitDisposal(q),
                NativeDocumentLifecycleAdmissionV2::ExplicitDisposal { native, coverage },
            ) => self.submit_explicit_disposal_v1(id, q, native, coverage, r),
            _ => Err(InvocationError::Unavailable),
        };
        capture_timed(
            r,
            Boundary::DocumentLifecycleFacadeAdmission,
            &result,
            timing.finish(),
        );
        result
    }
    fn submit_native_document_lifecycle_existing(
        &mut self,
        id: &InvocationId,
        command: NativeDocumentLifecycleCommandV1<'_>,
        admission: NativeDocumentLifecycleAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        match (command, admission) {
            (
                NativeDocumentLifecycleCommandV1::Scratch(q),
                NativeDocumentLifecycleAdmissionV1::Scratch(a),
            ) => self.submit_scratch_v1(id, q, a, r),
            (
                NativeDocumentLifecycleCommandV1::LiveRelation(q),
                NativeDocumentLifecycleAdmissionV1::LiveRelation(a),
            ) => self.submit_live_relation_v1(id, q, a, r),
            (
                NativeDocumentLifecycleCommandV1::PreparedDisposal(q),
                NativeDocumentLifecycleAdmissionV1::PreparedDisposal(a),
            ) => self.submit_retained_disposal_v1(id, q, a, r),
            _ => Err(InvocationError::Unavailable),
        }
    }
    /// Execute through the existing native service. It rechecks original owner,
    /// intent, fresh applicable rights, stop and actual publication guards.
    pub fn run_native_document_lifecycle_v1(
        &mut self,
        id: &InvocationId,
        facts: NativeDocumentLifecycleServiceV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.run_native_document_lifecycle_v2(
            id,
            NativeDocumentLifecycleServiceV2::Existing(facts),
            r,
        )
    }
    /// Current facts select an existing owner, not a profile or authority upgrade.
    /// One outer timing/fault boundary covers both versions and never settles Unknown.
    pub fn run_native_document_lifecycle_v2(
        &mut self,
        id: &InvocationId,
        facts: NativeDocumentLifecycleServiceV2<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        // The inner native service fences and propagates publisher interruption.
        // Preserve that panic and report Unknown at the facade too, not a refusal.
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match facts {
            NativeDocumentLifecycleServiceV2::Existing(f) => match f {
                NativeDocumentLifecycleServiceV1::Scratch(f) => self.run_scratch_v1(id, f, r),
                NativeDocumentLifecycleServiceV1::LiveRelation(f) => {
                    self.run_live_relation_v1(id, f, r)
                }
                NativeDocumentLifecycleServiceV1::PreparedDisposal(f) => {
                    self.run_retained_disposal_v1(id, f, r)
                }
            },
            NativeDocumentLifecycleServiceV2::ExplicitDisposal { native, coverage } => {
                self.run_explicit_disposal_v1(id, native, coverage, r)
            }
        }));
        let result = match caught {
            Ok(v) => v,
            Err(p) => {
                capture_timed(
                    r,
                    Boundary::DocumentLifecycleFacadeService,
                    &Err::<InvocationState, _>(InvocationError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(p)
            }
        };
        capture_timed(
            r,
            Boundary::DocumentLifecycleFacadeService,
            &result,
            timing.finish(),
        );
        result
    }
}
