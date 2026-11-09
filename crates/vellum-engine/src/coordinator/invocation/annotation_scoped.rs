//! Owned, funded native F13 preparation over the existing ledger and publisher.
use super::super::settings::basis::{copy_context, labels};
use super::*;
use std::mem::size_of;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnnotationInvocationLimitsV1 {
    pub max_context_bytes: usize,
    pub max_capture_bytes: usize,
    pub max_plan_bytes: usize,
    pub max_construction_bytes: usize,
}
impl AnnotationInvocationLimitsV1 {
    /// Selected logical frame ceilings, not physical allocation or numeric defaults.
    pub fn required_hold_bytes(
        self,
        parent_intent: usize,
        root: usize,
    ) -> Result<usize, InvocationError> {
        if self.max_context_bytes == 0
            || self.max_capture_bytes < self.max_context_bytes
            || self.max_plan_bytes == 0
            || self.max_construction_bytes < self.max_plan_bytes
            || parent_intent == 0
            || root == 0
        {
            return Err(InvocationError::Limit);
        }
        self.max_capture_bytes
            .checked_mul(3)
            .and_then(|n| n.checked_add(parent_intent.checked_mul(2)?))
            .and_then(|n| n.checked_add(root.checked_mul(2)?))
            .and_then(|n| {
                n.checked_add(
                    self.max_construction_bytes
                        .saturating_sub(parent_intent)
                        .checked_mul(2)?,
                )
            })
            .ok_or(InvocationError::Limit)
    }
}
/// Trusted host premises after caller authentication. No source-write right is
/// requested; source/annotation currency and native policy still come from core.
pub struct TrustedAnnotationInvocationV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub limits: AnnotationInvocationLimitsV1,
    pub required_reads: &'a [ReadDependencyV3],
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub interpretation_current: Option<bool>,
    pub companions_ready: Option<bool>,
    pub history_ready: Option<bool>,
    pub annotation_writable: Option<bool>,
    pub resources_ready: Option<bool>,
}
pub struct AnnotationInvocationRetirementV1 {
    pub owner_control_allowed: Option<bool>,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
}
impl TrustedAnnotationInvocationV1<'_> {
    pub(super) fn allowed(&self, c: &SettingsExecutionContextV1) -> bool {
        self.context == c
            && [
                self.authorized,
                self.scope,
                self.continuous,
                self.live,
                self.not_stopped,
                self.interpretation_current,
                self.companions_ready,
                self.history_ready,
                self.annotation_writable,
                self.resources_ready,
            ]
            .into_iter()
            .all(|v| v == Some(true))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::coordinator) struct NativeProfile {
    parent_intent: usize,
    coordinator: CoordinatorLimits,
    annotations: AnnotationLimits,
    history: HistoryLimits,
    pub(super) root: usize,
}
pub(super) struct Capture {
    pub(super) owner: Arc<()>,
    pub(super) context: SettingsExecutionContextV1,
    invocation: InvocationId,
    marker: Arc<()>,
    document: DocumentId,
    pub(super) mutation: AnnotationMutation,
    input: AnnotationAuthoringArgumentLimitsV1,
    limits: AnnotationInvocationLimitsV1,
    native: NativeProfile,
    caller: read_dependency::ReadCapture,
    required: read_dependency::ReadCapture,
    combined: read_dependency::ReadCapture,
    plan: Option<annotation::AnnotationPlan>,
    result: Option<results::Capture>,
}
impl Capture {
    pub(super) fn shutdown_reserved_bytes(
        &self,
        owner: &Arc<()>,
        id: &InvocationId,
        hold: &Option<Hold>,
        ledger: &retention::RetentionState,
    ) -> Result<usize, CoordinatorError> {
        let hold = hold.as_ref().ok_or(CoordinatorError::Faulted)?;
        if !Arc::ptr_eq(&self.owner, owner)
            || self.invocation != *id
            || !Arc::ptr_eq(&self.marker, &hold.marker)
        {
            return Err(CoordinatorError::Faulted);
        }
        let charge = self
            .limits
            .required_hold_bytes(self.native.parent_intent, self.native.root)
            .map_err(|_| CoordinatorError::Faulted)?;
        ledger
            .validate_exact(&hold.reservation, charge)
            .map_err(|_| CoordinatorError::Faulted)?;
        Ok(charge)
    }
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.document == *document
            || self.caller.refers_to_document(document)
            || self.required.refers_to_document(document)
            || self.combined.refers_to_document(document)
    }
}
pub(super) struct Hold {
    marker: Arc<()>,
    reservation: retention::Reservation,
    pub(super) receipt: Option<receipt::Receipt>,
}
pub(super) struct Prepared {
    capture: Arc<Capture>,
    seal: Arc<Capture>,
    plan: Result<PreparedTransaction, CoordinatorError>,
}
impl Prepared {
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.capture.refers_to_document(document)
            || self
                .plan
                .as_ref()
                .is_ok_and(|p| p.refers_to_document(document))
    }
}
macro_rules! redacted { ($($t:ty),+) => { $(impl std::fmt::Debug for $t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("ScopedAnnotation(<owned:redacted>)") }
})+ }; }
redacted!(Capture, Hold, Prepared);
pub(in crate::coordinator) mod receipt;
mod results;
pub use results::{
    AnnotationResultCapabilitiesV1, AnnotationResultDecisionV1, AnnotationResultGrantTargetV1,
    AnnotationResultOperationV1, AnnotationResultRecordV1, AnnotationResultV1,
    TrustedAnnotationResultAdmissionV1, TrustedAnnotationResultGrantV1,
    TrustedAnnotationResultReleaseV1,
};
fn reads(e: ReadDependencyError) -> InvocationError {
    match e {
        ReadDependencyError::Capacity => InvocationError::Capacity,
        ReadDependencyError::Limit => InvocationError::Limit,
        _ => InvocationError::MalformedIntent,
    }
}
fn parent(e: RetentionErrorV1) -> InvocationError {
    match e {
        RetentionErrorV1::Faulted => InvocationError::Faulted,
        RetentionErrorV1::Limit => InvocationError::Limit,
        RetentionErrorV1::Unavailable => InvocationError::Unavailable,
    }
}
fn add(a: usize, b: usize) -> Result<usize, InvocationError> {
    a.checked_add(b).ok_or(InvocationError::Limit)
}
pub(super) fn copy_mutation(q: &AnnotationMutation) -> Result<AnnotationMutation, InvocationError> {
    let copy = |s: &str| {
        let mut out = String::new();
        out.try_reserve_exact(s.len())
            .map_err(|_| InvocationError::Capacity)?;
        out.push_str(s);
        Ok::<_, InvocationError>(out)
    };
    Ok(match q {
        AnnotationMutation::Create { body, target } => AnnotationMutation::Create {
            body: copy(body)?,
            target: target.clone(),
        },
        AnnotationMutation::UpdateBody {
            annotation,
            expected_content,
            body,
        } => AnnotationMutation::UpdateBody {
            annotation: annotation.clone(),
            expected_content: *expected_content,
            body: copy(body)?,
        },
        AnnotationMutation::Reattach {
            annotation,
            expected_target,
            target,
        } => AnnotationMutation::Reattach {
            annotation: annotation.clone(),
            expected_target: *expected_target,
            target: target.clone(),
        },
        AnnotationMutation::Delete {
            annotation,
            expected_content,
            expected_target,
        } => AnnotationMutation::Delete {
            annotation: annotation.clone(),
            expected_content: *expected_content,
            expected_target: *expected_target,
        },
    })
}
impl Prepared {
    pub(super) fn intact(
        &self,
        c: &Arc<Capture>,
        owner: &Arc<()>,
        id: &InvocationId,
        hold: &Option<Hold>,
        ledger: &retention::RetentionState,
    ) -> bool {
        Arc::ptr_eq(c, &self.capture)
            && Arc::ptr_eq(c, &self.seal)
            && Arc::ptr_eq(&c.owner, owner)
            && c.invocation == *id
            && hold.as_ref().is_some_and(|h| {
                Arc::ptr_eq(&h.marker, &c.marker)
                    && h.receipt.is_none()
                    && c.limits
                        .required_hold_bytes(c.native.parent_intent, c.native.root)
                        .is_ok_and(|n| ledger.validate_exact(&h.reservation, n).is_ok())
            })
            && match &self.plan {
                Ok(p) => {
                    Arc::ptr_eq(&p.namespace, owner)
                        && p.standalone_annotation_seal() == c.plan
                        && p.reads_required
                        && p.declared_reads.as_ref() == Some(&c.combined)
                }
                Err(e) => {
                    !matches!(
                        e,
                        CoordinatorError::Faulted
                            | CoordinatorError::History(HistoryError::Faulted)
                    ) && c.plan.is_none()
                }
            }
    }
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn annotation_profile(
        &self,
    ) -> Result<NativeProfile, InvocationError> {
        Ok(NativeProfile {
            parent_intent: self.invocations.limits.max_intent_bytes,
            coordinator: self.limits,
            annotations: self
                .annotations
                .limits
                .ok_or(InvocationError::Unavailable)?,
            history: self
                .history
                .captured_limits()
                .ok_or(InvocationError::Unavailable)?,
            root: self.root_byte_ceiling.ok_or(InvocationError::Unavailable)?,
        })
    }
    pub fn submit_scoped_annotation_v1(
        &mut self,
        q: &NormalizedAnnotationAuthoringArgumentsV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedAnnotationInvocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.submit_scoped_annotation_result_input(q, c, f, None, r)
    }
    pub fn submit_scoped_annotation_with_result_v1(
        &mut self,
        q: &NormalizedAnnotationAuthoringArgumentsV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedAnnotationInvocationV1<'_>,
        result: &TrustedAnnotationResultAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let value = self.submit_scoped_annotation_result_input(q, c, f, Some(result), r);
        capture_timed(
            r,
            Boundary::AnnotationResultAdmission,
            &value,
            timing.finish(),
        );
        value
    }
    fn submit_scoped_annotation_result_input(
        &mut self,
        q: &NormalizedAnnotationAuthoringArgumentsV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedAnnotationInvocationV1<'_>,
        result: Option<&TrustedAnnotationResultAdmissionV1<'_>>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let mut provisional = None;
        let mut result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(q.invocation())?;
            if f.context != c
                || q.execution_profile() != c.profile
                || labels(c).iter().any(|s| s.is_empty())
            {
                return Err(InvocationError::Unavailable);
            }
            let result_capture =
                results::capture_admission(q, c, result, self.invocations.limits.max_intent_bytes)?;
            let native = if slot.intent.is_none() {
                if !f.allowed(c) {
                    return Err(InvocationError::Unavailable);
                }
                let native = self.annotation_profile()?;
                self.document(q.document())
                    .map_err(|_| InvocationError::Unavailable)?;
                let charge = f
                    .limits
                    .required_hold_bytes(self.invocations.limits.max_intent_bytes, native.root)?;
                provisional = Some(self.retention.reserve(charge).map_err(parent)?);
                Some(native)
            } else {
                let Some(BoundIntent::Annotation(original)) = &slot.intent else {
                    return Err(InvocationError::IntentMismatch);
                };
                let hold = slot
                    .annotation_hold
                    .as_ref()
                    .ok_or(InvocationError::Faulted)?;
                if original.invocation != *q.invocation()
                    || !Arc::ptr_eq(&original.owner, &self.namespace)
                    || !Arc::ptr_eq(&original.marker, &hold.marker)
                {
                    return Err(InvocationError::Faulted);
                }
                let expected = original
                    .limits
                    .required_hold_bytes(original.native.parent_intent, original.native.root)
                    .map_err(|_| InvocationError::Faulted)?;
                self.retention
                    .validate_exact(&hold.reservation, expected)
                    .map_err(parent)?;
                None
            };
            let input = read_dependency::ReadInput::V3(q.dependencies());
            let bound = f
                .limits
                .max_capture_bytes
                .min(self.invocations.limits.max_intent_bytes);
            if input.retained_bytes(&[]).is_none_or(|n| n > bound)
                || read_dependency::ReadCapture::size_for_predicates(f.required_reads.len())
                    .is_none_or(|n| n > bound)
            {
                return Err(InvocationError::Limit);
            }
            let caller = input.copy(&[]).map_err(reads)?;
            let required = read_dependency::ReadCapture::configuration_slice(
                f.required_reads,
                q.dependencies().budget,
            )
            .map_err(reads)?;
            let combined = caller.conjoin(&required).map_err(reads)?;
            if let Some(original) = &self.invocation_slot(q.invocation())?.intent {
                let BoundIntent::Annotation(original) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                if original.context != *c
                    || original.document != *q.document()
                    || original.mutation != *q.mutation()
                    || original.input != q.limits()
                    || original.limits != f.limits
                    || original.caller != caller
                    || original.required != required
                    || original.result != result_capture
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return self.invocation_slot(q.invocation())?.phase.state();
            }
            let native = native.ok_or(InvocationError::Faulted)?;
            let parent_intent = self.invocations.limits.max_intent_bytes;
            let context_bytes = labels(c)
                .iter()
                .try_fold(size_of::<SettingsExecutionContextV1>(), |n, s| {
                    add(n, s.len())
                })?;
            if context_bytes > f.limits.max_context_bytes {
                return Err(InvocationError::Limit);
            }
            let mut captured = add(size_of::<Capture>(), size_of::<Hold>())?;
            if result_capture.is_some() {
                captured = add(captured, AnnotationResultV1::required_bytes())?;
            }
            captured = add(captured, context_bytes)?;
            captured = add(captured, q.owned_bytes())?;
            for read in [&caller, &required, &combined] {
                captured = add(
                    captured,
                    read.logical_bytes().ok_or(InvocationError::Limit)?,
                )?;
            }
            if captured > parent_intent || captured > f.limits.max_capture_bytes {
                return Err(InvocationError::Limit);
            }
            let mutation = copy_mutation(q.mutation())?;
            let context = copy_context(c).map_err(|e| {
                if e == crate::settings::SettingsError::Capacity {
                    InvocationError::Capacity
                } else {
                    InvocationError::Limit
                }
            })?;
            let mut plan = self
                .check_owned_read_capture(&combined, r)
                .and_then(|()| self.prepare_compound(&[], &[], std::slice::from_ref(&mutation), r));
            if plan.as_ref().err().is_some_and(|e| {
                matches!(
                    e,
                    CoordinatorError::Faulted | CoordinatorError::History(HistoryError::Faulted)
                )
            }) {
                return Err(InvocationError::Faulted);
            }
            if let Ok(p) = &mut plan {
                p.reads_required = true;
                p.declared_reads = Some(combined.copy_owned().map_err(reads)?);
                let actual = add(
                    captured,
                    p.standalone_annotation_bytes()
                        .ok_or(InvocationError::Faulted)?,
                )?;
                if actual > parent_intent
                    || actual > f.limits.max_plan_bytes
                    || actual > f.limits.max_construction_bytes
                {
                    return Err(InvocationError::Limit);
                }
            }
            let marker = Arc::new(());
            let capture = Arc::new(Capture {
                owner: self.namespace.clone(),
                context,
                invocation: q.invocation().clone(),
                marker: marker.clone(),
                document: q.document().clone(),
                mutation,
                input: q.limits(),
                limits: f.limits,
                native,
                caller,
                required,
                combined,
                plan: plan
                    .as_ref()
                    .ok()
                    .and_then(PreparedTransaction::standalone_annotation_seal),
                result: result_capture,
            });
            let prepared = Prepared {
                capture: capture.clone(),
                seal: capture.clone(),
                plan,
            };
            self.invocations.slots[q.invocation().slot]
                .as_mut()
                .expect("issued slot")
                .annotation_hold = Some(Hold {
                marker,
                reservation: provisional.take().expect("funded preparation"),
                receipt: None,
            });
            self.install_invocation_admission(
                q.invocation(),
                BoundIntent::Annotation(capture),
                Phase::AnnotationPending(prepared),
            );
            Ok(InvocationState::Pending)
        })();
        if let Some(mut reservation) = provisional
            && self.retention.release(&mut reservation).is_err()
        {
            result = Err(InvocationError::Faulted);
        }
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(
            r,
            Boundary::AnnotationInvocationAdmission,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn run_scoped_annotation_v1(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        f: &TrustedAnnotationInvocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let result = self.run_scoped_annotation_inner(id, c, f, r);
        capture_timed(
            r,
            Boundary::AnnotationInvocationService,
            &result,
            timing.finish(),
        );
        result
    }
    fn run_scoped_annotation_inner(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        f: &TrustedAnnotationInvocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        if self.faulted {
            return Err(InvocationError::Faulted);
        }
        let slot = self.invocation_slot(id)?;
        let Some(BoundIntent::Annotation(original)) = &slot.intent else {
            return Err(InvocationError::IntentMismatch);
        };
        if original.context != *c || f.context != c {
            return Err(InvocationError::Unavailable);
        }
        match slot.phase {
            Phase::Terminal(_) => return Ok(InvocationState::Finished),
            Phase::Running => return Err(InvocationError::Faulted),
            Phase::AnnotationPending(_) => {}
            _ => return Err(InvocationError::IntentMismatch),
        }
        if self.service_requested_stop(id, r)? {
            return Ok(InvocationState::Finished);
        }
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated slot");
        let Phase::AnnotationPending(p) = std::mem::replace(&mut slot.phase, Phase::Running) else {
            unreachable!("annotation pending")
        };
        self.faulted = true;
        let Some(BoundIntent::Annotation(original)) = &slot.intent else {
            return Err(InvocationError::Faulted);
        };
        if !p.intact(
            original,
            &self.namespace,
            id,
            &slot.annotation_hold,
            &self.retention,
        ) {
            return Err(InvocationError::Faulted);
        }
        self.retention
            .validate(
                &slot
                    .annotation_hold
                    .as_ref()
                    .ok_or(InvocationError::Faulted)?
                    .reservation,
            )
            .map_err(parent)?;
        let original = original.clone();
        let mut receipt = None;
        let decision = (|| {
            if !f.allowed(c) {
                return Err(CoordinatorError::Unavailable);
            }
            if original.limits != f.limits
                || self
                    .annotation_profile()
                    .map_err(|_| CoordinatorError::Stale)?
                    != original.native
            {
                return Err(CoordinatorError::Stale);
            }
            let current = read_dependency::ReadCapture::configuration_slice(
                f.required_reads,
                original.required.budget,
            )
            .map_err(|_| CoordinatorError::Unavailable)?;
            if current != original.required {
                return Err(CoordinatorError::Stale);
            }
            self.check_owned_read_capture(&original.combined, r)?;
            self.commit_with_annotation_storage(p.plan?, id, &mut receipt, r)
        })();
        if decision.as_ref().err().is_some_and(|e| {
            matches!(
                e,
                CoordinatorError::Faulted | CoordinatorError::History(HistoryError::Faulted)
            )
        }) {
            return Err(InvocationError::Faulted);
        }
        let outcome = match decision {
            Ok(result) => {
                let Some(receipt) = receipt else {
                    return Err(InvocationError::Faulted);
                };
                self.invocations.slots[id.slot]
                    .as_mut()
                    .expect("original running slot")
                    .annotation_hold
                    .as_mut()
                    .expect("validated hold")
                    .receipt = Some(receipt);
                InvocationOutcome::Committed(result)
            }
            Err(e) => InvocationOutcome::Rejected(e),
        };
        self.install_invocation_terminal(id, outcome);
        self.faulted = false;
        Ok(InvocationState::Finished)
    }
    /// Protected preparation retirement; independently attested drain/discharge.
    /// Clears this capsule/coarse result, never refunds issuance or other holds.
    pub fn retire_scoped_annotation_v1(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        f: &AnnotationInvocationRetirementV1,
        r: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if [
                f.owner_control_allowed,
                f.retention_released,
                f.delivery_drained,
            ]
            .into_iter()
            .any(|v| v != Some(true))
            {
                return Err(InvocationError::Unavailable);
            }
            let slot = self.invocation_slot(id)?;
            let Some(BoundIntent::Annotation(capture)) = &slot.intent else {
                return Err(InvocationError::Unavailable);
            };
            if capture.context != *c || !Arc::ptr_eq(&capture.owner, &self.namespace) {
                return Err(InvocationError::Unavailable);
            }
            if !(matches!(slot.phase, Phase::Terminal(_))
                || self.faulted
                    && matches!(slot.phase, Phase::Running | Phase::AnnotationPending(_)))
            {
                return Err(InvocationError::Pending);
            }
            let hold = slot
                .annotation_hold
                .as_ref()
                .ok_or(InvocationError::Unavailable)?;
            if !Arc::ptr_eq(&hold.marker, &capture.marker) {
                return Err(InvocationError::Faulted);
            }
            let expected = capture
                .limits
                .required_hold_bytes(capture.native.parent_intent, capture.native.root)
                .map_err(|_| InvocationError::Faulted)?;
            self.retention
                .validate_exact(&hold.reservation, expected)
                .map_err(parent)?;
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .expect("validated slot");
            self.retention
                .release(
                    &mut slot
                        .annotation_hold
                        .as_mut()
                        .expect("funded hold")
                        .reservation,
                )
                .map_err(parent)?;
            slot.annotation_hold = None;
            slot.intent = Some(BoundIntent::RetiredAnnotation);
            if matches!(slot.phase, Phase::Terminal(_)) {
                slot.phase = Phase::Terminal(None);
            } else if matches!(slot.phase, Phase::AnnotationPending(_)) {
                slot.phase = Phase::AnnotationFaulted;
            }
            Ok(())
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(
            r,
            Boundary::AnnotationInvocationRetirement,
            &result,
            timing.finish(),
        );
        result
    }
}
