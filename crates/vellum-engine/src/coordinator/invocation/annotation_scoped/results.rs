//! Admission-bound native F13 disclosure; host policy truth remains a premise.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationResultCapabilitiesV1 {
    None,
    Identity,
    IdentityContent,
    IdentityTarget,
    IdentityContentTarget,
}
impl AnnotationResultCapabilitiesV1 {
    pub(in crate::coordinator) const fn bits(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Identity => 1,
            Self::IdentityContent => 3,
            Self::IdentityTarget => 5,
            Self::IdentityContentTarget => 7,
        }
    }
    const fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::None,
            1 => Self::Identity,
            3 => Self::IdentityContent,
            5 => Self::IdentityTarget,
            7 => Self::IdentityContentTarget,
            _ => panic!("closed annotation result capability invariant"),
        }
    }
    pub const fn meet(self, other: Self) -> Self {
        Self::from_bits(self.bits() & other.bits())
    }
    pub const fn join(self, other: Self) -> Self {
        Self::from_bits(self.bits() | other.bits())
    }
}
#[derive(Clone, Copy)]
pub enum AnnotationResultGrantTargetV1<'a> {
    /// Only the one original allocation of the bound create invocation.
    CreatedByInvocation(&'a DocumentId),
    Annotation(&'a AnnotationId),
}
#[derive(Clone, Copy)]
pub struct TrustedAnnotationResultGrantV1<'a> {
    pub invocation: &'a InvocationId,
    pub authority: &'a str,
    pub target: AnnotationResultGrantTargetV1<'a>,
    pub capabilities: AnnotationResultCapabilitiesV1,
}
pub struct TrustedAnnotationResultAdmissionV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    /// Actual host policy epoch, not an authenticated F06 policy object.
    pub publication_epoch: u64,
    pub publication_allowed: Option<bool>,
    pub context_current: Option<bool>,
    pub result_admission_allowed: Option<bool>,
    pub max_result_bytes: usize,
    pub grant: Option<TrustedAnnotationResultGrantV1<'a>>,
}
pub struct TrustedAnnotationResultReleaseV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub publication_epoch: u64,
    pub publication_allowed: Option<bool>,
    pub outcome_readable: Option<bool>,
    pub result_context_current: Option<bool>,
    pub reply_ready: Option<bool>,
    pub resources_ready: Option<bool>,
    pub max_result_bytes: usize,
    pub grant: Option<TrustedAnnotationResultGrantV1<'a>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationResultOperationV1 {
    Create,
    UpdateBody,
    Reattach,
    Delete,
}
impl From<receipt::Operation> for AnnotationResultOperationV1 {
    fn from(op: receipt::Operation) -> Self {
        match op {
            receipt::Operation::Create => Self::Create,
            receipt::Operation::Body => Self::UpdateBody,
            receipt::Operation::Target => Self::Reattach,
            receipt::Operation::Delete => Self::Delete,
        }
    }
}
#[derive(PartialEq, Eq)]
pub struct AnnotationResultRecordV1 {
    pub annotation: AnnotationId,
    /// Historical membership, not current liveness or mutation authority.
    pub present: bool,
    pub content_version: Option<u64>,
    pub target_generation: Option<u64>,
}
#[derive(PartialEq, Eq)]
pub enum AnnotationResultDecisionV1 {
    Applied {
        publication: InvocationId,
        record: AnnotationResultRecordV1,
    },
    Unchanged {
        record: AnnotationResultRecordV1,
    },
    NotPublished,
}
#[derive(PartialEq, Eq)]
pub struct AnnotationResultV1 {
    pub invocation: InvocationId,
    pub operation: AnnotationResultOperationV1,
    pub decision: AnnotationResultDecisionV1,
}
redacted!(
    AnnotationResultRecordV1,
    AnnotationResultDecisionV1,
    AnnotationResultV1
);
impl AnnotationResultV1 {
    /// Complete native inline frame, not JSON bytes or physical allocator peak.
    pub const fn required_bytes() -> usize {
        size_of::<Self>()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Capture {
    epoch: u64,
    maximum: usize,
    publication: bool,
    ceiling: AnnotationResultCapabilitiesV1,
}
fn capabilities(
    id: &InvocationId,
    c: &SettingsExecutionContextV1,
    document: &DocumentId,
    mutation: &AnnotationMutation,
    grant: Option<TrustedAnnotationResultGrantV1<'_>>,
    allocated: Option<&AnnotationId>,
) -> Result<AnnotationResultCapabilitiesV1, InvocationError> {
    let Some(g) = grant else {
        return Ok(AnnotationResultCapabilitiesV1::None);
    };
    if g.invocation != id || g.authority != c.authority {
        return Err(InvocationError::Unavailable);
    }
    let valid = match (mutation, g.target) {
        (
            AnnotationMutation::Create { .. },
            AnnotationResultGrantTargetV1::CreatedByInvocation(d),
        ) => d == document,
        (AnnotationMutation::Create { .. }, AnnotationResultGrantTargetV1::Annotation(a)) => {
            allocated == Some(a)
        }
        (
            AnnotationMutation::UpdateBody { annotation, .. }
            | AnnotationMutation::Reattach { annotation, .. }
            | AnnotationMutation::Delete { annotation, .. },
            AnnotationResultGrantTargetV1::Annotation(a),
        ) => a == annotation,
        _ => false,
    };
    if !valid {
        return Err(InvocationError::Unavailable);
    }
    Ok(g.capabilities)
}
pub(super) fn capture_admission(
    q: &NormalizedAnnotationAuthoringArgumentsV1<'_>,
    c: &SettingsExecutionContextV1,
    f: Option<&TrustedAnnotationResultAdmissionV1<'_>>,
    parent: usize,
) -> Result<Option<Capture>, InvocationError> {
    let Some(f) = f else { return Ok(None) };
    if f.context != c || f.context_current != Some(true) || f.result_admission_allowed != Some(true)
    {
        return Err(InvocationError::Unavailable);
    }
    let publication = f.publication_allowed.ok_or(InvocationError::Unavailable)?;
    if f.max_result_bytes < AnnotationResultV1::required_bytes() || f.max_result_bytes > parent {
        return Err(InvocationError::Limit);
    }
    let ceiling = capabilities(q.invocation(), c, q.document(), q.mutation(), f.grant, None)?;
    Ok(Some(Capture {
        epoch: f.publication_epoch,
        maximum: f.max_result_bytes,
        publication,
        ceiling,
    }))
}
impl SourceCoordinator {
    /// Native retained-outcome facet. Does not execute, mint a reference or
    /// supply the separate F06 policy owner/decoder/observation context.
    pub fn retained_annotation_result_v1(
        &self,
        id: &InvocationId,
        f: &TrustedAnnotationResultReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<AnnotationResultV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if [
                f.publication_allowed,
                f.outcome_readable,
                f.result_context_current,
                f.reply_ready,
                f.resources_ready,
            ]
            .into_iter()
            .any(|v| v != Some(true))
            {
                return Err(InvocationError::Unavailable);
            }
            let slot = self.invocation_slot(id)?;
            let Some(BoundIntent::Annotation(original)) = &slot.intent else {
                return Err(InvocationError::OutcomeUnavailable);
            };
            let c = original
                .result
                .as_ref()
                .ok_or(InvocationError::OutcomeUnavailable)?;
            if original.context != *f.context || c.epoch != f.publication_epoch || !c.publication {
                return Err(InvocationError::Unavailable);
            }
            if original.invocation != *id || !Arc::ptr_eq(&original.owner, &self.namespace) {
                return Err(InvocationError::Faulted);
            }
            let Phase::Terminal(Some(outcome)) = &slot.phase else {
                return Err(InvocationError::OutcomeUnavailable);
            };
            let hold = slot
                .annotation_hold
                .as_ref()
                .ok_or(InvocationError::Faulted)?;
            if !Arc::ptr_eq(&hold.marker, &original.marker) {
                return Err(InvocationError::Faulted);
            }
            let charge = original
                .limits
                .required_hold_bytes(original.native.parent_intent, original.native.root)
                .map_err(|_| InvocationError::Faulted)?;
            self.retention
                .validate_exact(&hold.reservation, charge)
                .map_err(parent)?;
            if c.maximum < AnnotationResultV1::required_bytes() {
                return Err(InvocationError::Faulted);
            }
            if f.max_result_bytes < AnnotationResultV1::required_bytes() {
                return Err(InvocationError::Limit);
            }
            let receipt = hold.receipt.as_ref().ok_or(InvocationError::Faulted)?;
            if receipt.invocation != *id
                || receipt.operation != receipt::Operation::of(&original.mutation)
            {
                return Err(InvocationError::Faulted);
            }
            let project =
                |record: &receipt::Record| -> Result<AnnotationResultRecordV1, InvocationError> {
                    if original.plan.as_ref().map(|p| &p.id) != Some(&record.annotation)
                        || record.content_version == 0
                        || record.target_generation == 0
                        || record.present == (receipt.operation == receipt::Operation::Delete)
                    {
                        return Err(InvocationError::Faulted);
                    }
                    let current = capabilities(
                        id,
                        &original.context,
                        &original.document,
                        &original.mutation,
                        f.grant,
                        Some(&record.annotation),
                    )?;
                    let allowed = c.ceiling.meet(current).bits();
                    if allowed & 1 == 0 {
                        return Err(InvocationError::Unavailable);
                    }
                    Ok(AnnotationResultRecordV1 {
                        annotation: record.annotation.clone(),
                        present: record.present,
                        content_version: (allowed & 2 != 0).then_some(record.content_version),
                        target_generation: (allowed & 4 != 0).then_some(record.target_generation),
                    })
                };
            let decision = match (&receipt.decision, outcome) {
                (
                    receipt::Decision::NotPublished,
                    InvocationOutcome::Rejected(_) | InvocationOutcome::StoppedBeforePublication,
                ) => AnnotationResultDecisionV1::NotPublished,
                (
                    receipt::Decision::Applied {
                        publication,
                        record,
                    },
                    InvocationOutcome::Committed(coarse),
                ) if publication == id
                    && coarse.operation.is_some()
                    && coarse.changes.is_empty()
                    && coarse.annotation_changes.len() == 1
                    && coarse.annotation_changes.iter().all(|f| {
                        f.annotation == record.annotation
                            && f.present == record.present
                            && f.content_version == record.content_version
                            && f.target_generation == record.target_generation
                    }) =>
                {
                    AnnotationResultDecisionV1::Applied {
                        publication: publication.clone(),
                        record: project(record)?,
                    }
                }
                (receipt::Decision::Unchanged { record }, InvocationOutcome::Committed(coarse))
                    if coarse.operation.is_none()
                        && coarse.changes.is_empty()
                        && coarse.annotation_changes.is_empty()
                        && matches!(
                            receipt.operation,
                            receipt::Operation::Body | receipt::Operation::Target
                        ) =>
                {
                    AnnotationResultDecisionV1::Unchanged {
                        record: project(record)?,
                    }
                }
                _ => return Err(InvocationError::Faulted),
            };
            Ok(AnnotationResultV1 {
                invocation: id.clone(),
                operation: receipt.operation.into(),
                decision,
            })
        })();
        capture_timed(
            r,
            Boundary::AnnotationResultRelease,
            &result,
            timing.finish(),
        );
        result
    }
}
