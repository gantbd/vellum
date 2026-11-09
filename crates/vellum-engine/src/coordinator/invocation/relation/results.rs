//! Admission-bound historical receipts. No execution grant, reference issuer or F06 observer.
use super::*;
use std::mem::size_of;
#[cfg(test)]
mod tests;
pub struct TrustedRelationResultAdmissionV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub max_result_bytes: usize,
    pub result_admission_allowed: Option<bool>,
    /// Trusted host attests this complete frame belongs to this actual owner/ticket.
    pub context_current: Option<bool>,
    /// Immutable disclosure ceiling; false permits execution but withholds identifiers.
    pub relation_result_identifiers_allowed: Option<bool>,
}
pub struct TrustedRelationResultReleaseV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub max_result_bytes: usize,
    pub outcome_readable: Option<bool>,
    pub relation_result_readable: Option<bool>,
    pub relation_result_context_current: Option<bool>,
    pub historical_target_readable: Option<bool>,
    pub reply_ready: Option<bool>,
    pub resources_ready: Option<bool>,
}
#[derive(PartialEq, Eq)]
pub struct RelationResultV1 {
    pub invocation: InvocationId,
    /// Status/kind/target are encoded by the closed native receipt enum.
    /// Opaque historical identity is not current existence or mutation authority.
    pub receipt: RelationReceiptV1,
}
impl std::fmt::Debug for RelationResultV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RelationResultV1(<redacted>)")
    }
}
impl RelationResultV1 {
    /// Fixed native inline wrapper and Arc-backed opaque handles; not a wire/RSS bound.
    pub const fn required_bytes() -> usize {
        size_of::<Self>()
    }
}
pub(in crate::coordinator::invocation) struct Capture {
    invocation: InvocationId,
    context: SettingsExecutionContextV1,
    maximum: usize,
    ceiling: bool,
    pub(in crate::coordinator::invocation) receipt: Option<RelationReceiptV1>,
}
impl std::fmt::Debug for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RelationResultCapture(<redacted>)")
    }
}
pub(in crate::coordinator::invocation) fn base_bytes() -> usize {
    size_of::<RelationCommandV1>()
        + size_of::<Option<super::super::super::relation::RelationProfile>>()
        + size_of::<Option<Box<Capture>>>()
}
fn labels(c: &SettingsExecutionContextV1) -> [&str; 7] {
    [
        &c.authority,
        &c.engine,
        &c.recipient,
        &c.purpose,
        &c.destination,
        &c.profile,
        &c.continuity,
    ]
}
impl TrustedRelationResultAdmissionV1<'_> {
    /// Complete native relation interpretation plus owned result capture/seal.
    /// Callers still select explicit invocation/result limits; no default is installed.
    pub fn required_intent_bytes(
        context: &SettingsExecutionContextV1,
    ) -> Result<usize, InvocationError> {
        labels(context)
            .into_iter()
            .try_fold(base_bytes() + size_of::<Capture>(), |n, v| {
                n.checked_add(v.len()).ok_or(InvocationError::Limit)
            })
    }
}
fn receipt_matches(command: &RelationCommandV1, receipt: &RelationReceiptV1) -> bool {
    match (command, receipt) {
        (RelationCommandV1::CreateView, RelationReceiptV1::ViewCreated(_))
        | (RelationCommandV1::CreateWorkspace, RelationReceiptV1::WorkspaceCreated(_)) => true,
        (RelationCommandV1::RetireView { view, .. }, RelationReceiptV1::ViewRetired(target)) => {
            view == target
        }
        (
            RelationCommandV1::RetireWorkspace { expected },
            RelationReceiptV1::WorkspaceRetired(target),
        ) => expected.workspace == *target,
        _ => false,
    }
}
fn native_error(receipt: &RelationReceiptV1, error: InvocationError) -> RelationErrorV1 {
    if matches!(
        receipt,
        RelationReceiptV1::ViewCreated(_) | RelationReceiptV1::ViewRetired(_)
    ) {
        RelationErrorV1::View(if error == InvocationError::Limit {
            crate::selection::SelectionError::Limit
        } else {
            crate::selection::SelectionError::Faulted
        })
    } else {
        RelationErrorV1::Workspace(if error == InvocationError::Limit {
            CoordinatorError::Limit
        } else {
            CoordinatorError::Faulted
        })
    }
}
impl SourceCoordinator {
    /// Optional result admission for an issued native relation job. This only
    /// scopes later disclosure; it does not authorize execution on a recipient's behalf.
    pub fn submit_relation_with_result_v1(
        &mut self,
        id: &InvocationId,
        command: &RelationCommandV1,
        f: &TrustedRelationResultAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        let timing = r.begin_service_timing();
        let result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(id)?;
            if f.result_admission_allowed != Some(true) || f.context_current != Some(true) {
                return Err(InvocationError::Unavailable);
            }
            let ceiling = f
                .relation_result_identifiers_allowed
                .ok_or(InvocationError::Unavailable)?;
            if labels(f.context).iter().any(|v| v.is_empty()) {
                return Err(InvocationError::MalformedIntent);
            }
            if f.max_result_bytes > self.invocations.limits.max_intent_bytes
                || (ceiling && f.max_result_bytes < RelationResultV1::required_bytes())
                || TrustedRelationResultAdmissionV1::required_intent_bytes(f.context)?
                    > self.invocations.limits.max_intent_bytes
            {
                return Err(InvocationError::Limit);
            }
            if let Some(intent) = &slot.intent {
                let BoundIntent::Relation {
                    command: original,
                    result: Some(original_result),
                    ..
                } = intent
                else {
                    return Err(InvocationError::Unavailable);
                };
                if original_result.invocation != *id {
                    return Err(InvocationError::Faulted);
                }
                if original != command
                    || original_result.context != *f.context
                    || original_result.maximum != f.max_result_bytes
                    || original_result.ceiling != ceiling
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return self.submit_relation(id, command, r);
            }
            if !matches!(slot.phase, Phase::Issued(_)) {
                return Err(InvocationError::Unavailable);
            }
            let context =
                crate::coordinator::settings::basis::copy_context(f.context).map_err(|e| {
                    if e == crate::settings::SettingsError::Capacity {
                        InvocationError::Capacity
                    } else {
                        InvocationError::Limit
                    }
                })?;
            let capture = Box::new(Capture {
                invocation: id.clone(),
                context,
                maximum: f.max_result_bytes,
                ceiling,
                receipt: None,
            });
            let state = self.submit_relation(id, command, r)?;
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .ok_or(InvocationError::Faulted)?;
            let Some(BoundIntent::Relation { result, .. }) = &mut slot.intent else {
                return Err(InvocationError::Faulted);
            };
            *result = Some(capture);
            Ok(state)
        })();
        capture_timed(
            r,
            Boundary::RelationResultAdmission,
            &result,
            timing.finish(),
        );
        result
    }
    pub(in crate::coordinator::invocation) fn stage_relation_result_v1(
        &self,
        id: &InvocationId,
        receipt: &RelationReceiptV1,
    ) -> Result<Option<RelationReceiptV1>, RelationErrorV1> {
        let result = (|| {
            let slot = self.invocation_slot(id)?;
            let Some(BoundIntent::Relation {
                command, result, ..
            }) = &slot.intent
            else {
                return Err(InvocationError::Faulted);
            };
            if !receipt_matches(command, receipt) {
                return Err(InvocationError::Faulted);
            }
            let Some(capture) = result else {
                return Ok(None);
            };
            if capture.invocation != *id || capture.receipt.is_some() {
                return Err(InvocationError::Faulted);
            }
            if TrustedRelationResultAdmissionV1::required_intent_bytes(&capture.context)?
                > self.invocations.limits.max_intent_bytes
                || (capture.ceiling
                    && RelationResultV1::required_bytes()
                        > capture
                            .maximum
                            .min(self.invocations.limits.max_intent_bytes))
            {
                return Err(InvocationError::Limit);
            }
            Ok(capture.ceiling.then(|| receipt.clone()))
        })();
        result.map_err(|e| native_error(receipt, e))
    }
    /// Current permission-filtered historical facet. No execution, target lookup,
    /// reference issuance, implicit result hold or promotion of later grants.
    pub fn retained_relation_result_v1(
        &self,
        id: &InvocationId,
        f: &TrustedRelationResultReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<RelationResultV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if self.faulted
                || [
                    f.outcome_readable,
                    f.relation_result_readable,
                    f.relation_result_context_current,
                    f.historical_target_readable,
                    f.reply_ready,
                    f.resources_ready,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(InvocationError::Unavailable);
            }
            let slot = self.invocation_slot(id)?;
            let Some(BoundIntent::Relation {
                command,
                result: Some(capture),
                ..
            }) = &slot.intent
            else {
                return Err(InvocationError::Unavailable);
            };
            if capture.invocation != *id {
                return Err(InvocationError::Faulted);
            }
            if !capture.ceiling || capture.context != *f.context {
                return Err(InvocationError::Unavailable);
            }
            let Phase::Terminal(Some(InvocationOutcome::RelationCommitted(receipt))) = &slot.phase
            else {
                return Err(InvocationError::OutcomeUnavailable);
            };
            if capture.receipt.as_ref() != Some(receipt) || !receipt_matches(command, receipt) {
                return Err(InvocationError::Faulted);
            }
            if RelationResultV1::required_bytes()
                > f.max_result_bytes
                    .min(capture.maximum)
                    .min(self.invocations.limits.max_intent_bytes)
            {
                return Err(InvocationError::Limit);
            }
            Ok(RelationResultV1 {
                invocation: id.clone(),
                receipt: receipt.clone(),
            })
        })();
        capture_timed(r, Boundary::RelationResultRelease, &result, timing.finish());
        result
    }
}
