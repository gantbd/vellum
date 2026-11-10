//! Native F10 current checks. Advice is neither preparation nor authority.
use super::*;
use std::mem::size_of;
use std::panic::{AssertUnwindSafe, catch_unwind};
pub(in crate::coordinator) mod ingress;
pub use ingress::CompoundAdviceEnvelopeValueV1;
#[cfg(test)]
pub(in crate::coordinator) mod tests;
pub const NATIVE_COMPOUND_ADVICE_PROFILE_V1: &str = "native-compound-current-checks/1";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompoundAdviceLimitsV1 {
    /// Native logical request units, not JSON bytes or total allocator/RSS.
    pub max_request_bytes: usize,
    pub max_label_bytes: usize,
    pub max_reply_bytes: usize,
}
pub struct CompoundAdviceRequestV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub policy: &'a str,
    pub profile: &'a str,
    pub plan: &'a CompoundIntent,
    pub reads: Option<&'a ReadDependencyRequestV3>,
    pub scopes: Option<(&'a [SourceOperationScope], SourceScopeBudget)>,
    pub limits: CompoundAdviceLimitsV1,
}
/// Fixed metadata only; borrows the exact immutable request. No root payload pin,
/// registration, claim, invocation or capacity reservation. Host attestation aid.
pub struct CompoundAdviceBasisV1<'a> {
    request: &'a CompoundAdviceRequestV1<'a>,
    root: root_identity::NativeRootIdentity,
    annotations: AnnotationUsage,
    history: HistoryUsage,
}
/// Separately authenticated host premises, not requester assertions. Complete
/// observability includes private companion/capacity influences on the Boolean.
pub struct TrustedCompoundAdviceFactsV1<'a> {
    pub request: &'a CompoundAdviceRequestV1<'a>,
    pub basis: &'a CompoundAdviceBasisV1<'a>,
    pub context: &'a SettingsExecutionContextV1,
    pub policy: &'a str,
    pub domain_bound: Option<bool>,
    pub authorized: Option<bool>,
    pub scope_allowed: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub interpretation_current: Option<bool>,
    pub proposal_readable: Option<bool>,
    pub validation_observable: Option<bool>,
    pub resources_ready: Option<bool>,
    pub reply_ready: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompoundAdviceReplyV1 {
    Advice {
        profile: &'static str,
        checks_satisfied: bool,
    },
    Unavailable,
}
macro_rules! redact {($($t:ty),+) => {$(impl std::fmt::Debug for $t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("<compound-advice:redacted>") }
})+};}
redact!(
    CompoundAdviceRequestV1<'_>,
    CompoundAdviceBasisV1<'_>,
    TrustedCompoundAdviceFactsV1<'_>
);
fn parent(error: RetentionErrorV1) -> CoordinatorError {
    match error {
        RetentionErrorV1::Faulted => CoordinatorError::Faulted,
        _ => CoordinatorError::Capacity,
    }
}
fn root(error: RootAccountingErrorV1) -> CoordinatorError {
    match error {
        RootAccountingErrorV1::Faulted | RootAccountingErrorV1::Overflow => {
            CoordinatorError::Faulted
        }
        _ => CoordinatorError::Unavailable,
    }
}
fn input_bytes(q: &CompoundAdviceRequestV1<'_>, maximum: usize) -> Result<usize, CoordinatorError> {
    let mut bytes = size_of::<CompoundAdviceRequestV1<'_>>() + size_of::<CompoundIntent>();
    let mut charge = |n: usize| {
        bytes = bytes
            .checked_add(n)
            .filter(|n| *n <= maximum)
            .ok_or(CoordinatorError::Capacity)?;
        Ok::<_, CoordinatorError>(())
    };
    let rows = |n: usize, size: usize| n.checked_mul(size).ok_or(CoordinatorError::Capacity);
    charge(0)?;
    charge(rows(
        q.plan.source.writes.len(),
        size_of::<DocumentEdits>(),
    )?)?;
    charge(rows(
        q.plan.source.reads.len(),
        size_of::<DocumentVersion>(),
    )?)?;
    charge(rows(
        q.plan.annotations.len(),
        size_of::<AnnotationMutation>(),
    )?)?;
    for value in settings::basis::labels(q.context) {
        charge(value.len())?;
    }
    charge(q.policy.len())?;
    for write in &q.plan.source.writes {
        charge(rows(write.edits.len(), size_of::<Replacement>())?)?;
        for edit in &write.edits {
            charge(edit.expected.len())?;
            charge(edit.inserted.len())?;
        }
    }
    for mutation in &q.plan.annotations {
        if let AnnotationMutation::Create { body, .. }
        | AnnotationMutation::UpdateBody { body, .. } = mutation
        {
            charge(body.len())?;
        }
    }
    if let Some(reads) = q.reads {
        charge(size_of::<ReadDependencyRequestV3>())?;
        charge(rows(reads.predicates.len(), size_of::<ReadDependencyV3>())?)?;
    }
    if let Some((scopes, _)) = q.scopes {
        charge(size_of::<SourceScopeBudget>())?;
        charge(rows(scopes.len(), size_of::<SourceOperationScope>())?)?;
        for scope in scopes {
            charge(rows(
                scope.replacement_ranges.len(),
                size_of::<ByteRange>(),
            )?)?;
            charge(rows(
                scope.insertion_sites.len(),
                size_of::<InsertionSiteRange>(),
            )?)?;
        }
    }
    Ok(bytes)
}
fn private_failure(error: &CoordinatorError) -> bool {
    matches!(
        error,
        CoordinatorError::Faulted
            | CoordinatorError::History(HistoryError::Faulted)
            | CoordinatorError::Annotation(AnnotationError::Faulted)
            | CoordinatorError::Search(SearchError::Faulted)
            | CoordinatorError::Resource(crate::resource::ResourceError::Faulted)
    )
}
fn capacity_failure(error: &CoordinatorError) -> bool {
    matches!(
        error,
        CoordinatorError::Capacity
            | CoordinatorError::Source(SourceError::Capacity)
            | CoordinatorError::History(HistoryError::Capacity)
            | CoordinatorError::Annotation(AnnotationError::Capacity)
            | CoordinatorError::Search(SearchError::Capacity)
            | CoordinatorError::Scope(SourceScopeError::Capacity)
            | CoordinatorError::ReadDependency(ReadDependencyError::Capacity)
    )
}
impl SourceCoordinator {
    /// Trusted host basis capture only; possession grants no proposal disclosure.
    pub fn capture_compound_advice_basis_v1<'a>(
        &self,
        request: &'a CompoundAdviceRequestV1<'a>,
        r: &mut Recorder,
    ) -> Option<CompoundAdviceBasisV1<'a>> {
        let out = self
            .native_root_identity()
            .ok()
            .map(|root| CompoundAdviceBasisV1 {
                request,
                root,
                annotations: self.annotation_usage(),
                history: self.history_usage(),
            });
        r.record(Observation {
            boundary: Boundary::CompoundAdviceBasis,
            disposition: if out.is_some() {
                Disposition::Completed
            } else {
                Disposition::Refused
            },
            service_time: None,
        });
        out
    }
    /// Native, in-process recipient-scoped advice; host authentication and complete
    /// predicate observability are explicit premises. No wire/JSON codec implied.
    pub fn query_compound_advice_v1(
        &mut self,
        q: &CompoundAdviceRequestV1<'_>,
        f: &TrustedCompoundAdviceFactsV1<'_>,
        r: &mut Recorder,
    ) -> CompoundAdviceReplyV1 {
        let timing = r.begin_service_timing();
        let mut classification = None;
        let mut workspace = None;
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.ensure_operational()?;
            if self.annotations.limits.is_none() {
                return Err(CoordinatorError::Unavailable);
            }
            if !std::ptr::eq(q, f.request)
                || !std::ptr::eq(q, f.basis.request)
                || q.context != f.context
                || q.policy != f.policy
                || q.profile != NATIVE_COMPOUND_ADVICE_PROFILE_V1
                || f.basis.root != self.native_root_identity().map_err(root)?
                || f.basis.annotations != self.annotation_usage()
                || f.basis.history != self.history_usage()
                || [
                    f.domain_bound,
                    f.authorized,
                    f.scope_allowed,
                    f.continuous,
                    f.live,
                    f.not_stopped,
                    f.interpretation_current,
                    f.proposal_readable,
                    f.validation_observable,
                    f.resources_ready,
                    f.reply_ready,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(CoordinatorError::Unavailable);
            }
            let limits = q.limits;
            if limits.max_reply_bytes < size_of::<CompoundAdviceReplyV1>()
                || q.policy.is_empty()
                || q.policy.len() > limits.max_label_bytes
                || settings::basis::labels(q.context)
                    .iter()
                    .any(|v| v.is_empty() || v.len() > limits.max_label_bytes)
            {
                return Err(CoordinatorError::Unavailable);
            }
            classification = Some(
                self.retention
                    .reserve(size_of::<CompoundAdviceLimitsV1>())
                    .map_err(parent)?,
            );
            let maximum = limits
                .max_request_bytes
                .min(self.compound_advice_request_ceiling());
            let frames = maximum.checked_mul(8).ok_or(CoordinatorError::Capacity)?;
            workspace = Some(self.retention.reserve(frames).map_err(parent)?);
            input_bytes(q, maximum)?;
            r.record(Observation {
                boundary: Boundary::CompoundAdviceAdmission,
                disposition: Disposition::Completed,
                service_time: None,
            });
            let verdict = self.decide_compound_advice(q, r);
            match verdict {
                Ok(()) => Ok(CompoundAdviceReplyV1::Advice {
                    profile: NATIVE_COMPOUND_ADVICE_PROFILE_V1,
                    checks_satisfied: true,
                }),
                Err(error) if private_failure(&error) => {
                    self.faulted = true;
                    Err(error)
                }
                Err(error) if capacity_failure(&error) => Err(error),
                Err(_) => Ok(CompoundAdviceReplyV1::Advice {
                    profile: NATIVE_COMPOUND_ADVICE_PROFILE_V1,
                    checks_satisfied: false,
                }),
            }
        }));
        let mut out = match result {
            Ok(Ok(out)) => out,
            Ok(Err(error)) => {
                if private_failure(&error) {
                    self.faulted = true;
                }
                CompoundAdviceReplyV1::Unavailable
            }
            Err(_) => {
                self.faulted = true;
                CompoundAdviceReplyV1::Unavailable
            }
        };
        for lease in [workspace.as_mut(), classification.as_mut()]
            .into_iter()
            .flatten()
        {
            if self.retention.release(lease).is_err() {
                self.faulted = true;
                out = CompoundAdviceReplyV1::Unavailable;
            }
        }
        r.record(Observation {
            boundary: Boundary::CompoundAdviceRelease,
            disposition: if self.faulted {
                Disposition::Unknown
            } else if matches!(out, CompoundAdviceReplyV1::Advice { .. }) {
                Disposition::Completed
            } else {
                Disposition::Refused
            },
            service_time: timing.finish(),
        });
        out
    }
    fn decide_compound_advice(
        &mut self,
        q: &CompoundAdviceRequestV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let result = (|| {
            let source = self
                .normalize_intent_read_profile(&q.plan.source, q.reads.is_none())
                .map_err(|error| match error {
                    InvocationError::Capacity => CoordinatorError::Capacity,
                    InvocationError::Faulted => CoordinatorError::Faulted,
                    _ => CoordinatorError::Unavailable,
                })?;
            let reads = q
                .reads
                .map(|reads| {
                    self.prepare_read_capture_profile(
                        &q.plan.source.reads,
                        read_dependency::ReadInput::V3(reads),
                        r,
                    )
                })
                .transpose()?;
            let scopes = q
                .scopes
                .map(|(scopes, budget)| self.capture_scopes(&source.writes, scopes, budget))
                .transpose()?;
            let private = self.prepare_compound_advice_candidate(
                &source.writes,
                &source.reads,
                &q.plan.annotations,
                scopes,
                reads,
                r,
            )?;
            let prepared = private.prepared();
            let decision = self.decide_transaction_candidate(prepared, true, r)?;
            if decision.changed == 0 && !decision.annotations.changed() && !decision.views.changed()
            {
                return self.check_root_growth(0, r);
            }
            let generation = self.next_publication_generation(None)?;
            let history = self
                .decide_history(prepared, generation, &decision.annotations.journals)
                .map_err(CoordinatorError::History)?;
            self.check_compound_advice_root(
                prepared,
                &decision.annotations,
                &decision.views,
                &history,
                r,
            )
        })();
        observe(r, Boundary::CompoundAdviceDecision, &result);
        result
    }
}
