//! Trusted first-party compound result ownership, not public mixed-plan authority.
use super::super::settings::basis::{copy_context, labels};
use super::*;
use annotation_scoped::receipt::{Decision, Operation, Receipt, Record};
use std::mem::size_of;
#[cfg(test)]
mod tests;

/// Separate closed profile: standalone CreatedByInvocation is not widened.
#[derive(Clone, Copy)]
pub enum CompoundAnnotationResultGrantTargetV1<'a> {
    /// Original annotation-component ordinal, never execution/result ordering.
    /// The ordinal locates a binding; it is not itself a capability.
    CreatedByComponent {
        document: &'a DocumentId,
        component: usize,
    },
    Annotation(&'a AnnotationId),
}
#[derive(Clone, Copy)]
pub struct TrustedCompoundAnnotationResultGrantV1<'a> {
    pub invocation: &'a InvocationId,
    pub authority: &'a str,
    pub target: CompoundAnnotationResultGrantTargetV1<'a>,
    pub capabilities: AnnotationResultCapabilitiesV1,
}
pub struct TrustedCompoundAnnotationResultAdmissionV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub publication_epoch: u64,
    pub publication_allowed: Option<bool>,
    pub context_current: Option<bool>,
    pub result_admission_allowed: Option<bool>,
    pub max_result_bytes: usize,
    /// Exactly one optional grant per original annotation component. No targets
    /// are taken from this vector; native preparation allocates/binds them.
    pub grants: &'a [Option<TrustedCompoundAnnotationResultGrantV1<'a>>],
}
pub struct TrustedCompoundAnnotationResultReleaseV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub publication_epoch: u64,
    pub publication_allowed: Option<bool>,
    pub outcome_readable: Option<bool>,
    pub result_context_current: Option<bool>,
    pub reply_ready: Option<bool>,
    pub resources_ready: Option<bool>,
    pub max_result_bytes: usize,
    pub grant: Option<TrustedCompoundAnnotationResultGrantV1<'a>>,
}
pub struct CompoundAnnotationResultEvictionV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
}
#[derive(Debug)]
pub(super) struct Hold {
    owner: Arc<()>,
    invocation: InvocationId,
    context: SettingsExecutionContextV1,
    epoch: u64,
    publication: bool,
    maximum: usize,
    charge: usize,
    reservation: retention::Reservation,
    base: Option<(retention::Reservation, usize)>,
    ceilings: Vec<AnnotationResultCapabilitiesV1>,
    plan: Option<Vec<annotation::AnnotationPlan>>,
    pub(super) receipts: Option<Vec<Receipt>>,
}
pub(in crate::coordinator) struct Storage<'a> {
    pub(in crate::coordinator) invocation: &'a InvocationId,
    pub(in crate::coordinator) receipts: &'a mut Option<Vec<Receipt>>,
}
fn parent(e: RetentionErrorV1) -> InvocationError {
    match e {
        RetentionErrorV1::Faulted => InvocationError::Faulted,
        RetentionErrorV1::Limit => InvocationError::Limit,
        RetentionErrorV1::Unavailable => InvocationError::Unavailable,
    }
}
fn ceiling(
    id: &InvocationId,
    c: &SettingsExecutionContextV1,
    index: usize,
    m: &AnnotationMutation,
    g: Option<TrustedCompoundAnnotationResultGrantV1<'_>>,
    allocated: Option<&AnnotationId>,
) -> Result<AnnotationResultCapabilitiesV1, InvocationError> {
    let Some(g) = g else {
        return Ok(AnnotationResultCapabilitiesV1::None);
    };
    if g.invocation != id || g.authority != c.authority {
        return Err(InvocationError::Unavailable);
    }
    let valid = match (m, g.target) {
        (
            AnnotationMutation::Create { target, .. },
            CompoundAnnotationResultGrantTargetV1::CreatedByComponent {
                document,
                component,
            },
        ) => component == index && *document == target.source.document,
        (
            AnnotationMutation::Create { .. },
            CompoundAnnotationResultGrantTargetV1::Annotation(a),
        ) => allocated == Some(a),
        (
            AnnotationMutation::UpdateBody { annotation, .. }
            | AnnotationMutation::Reattach { annotation, .. }
            | AnnotationMutation::Delete { annotation, .. },
            CompoundAnnotationResultGrantTargetV1::Annotation(a),
        ) => annotation == a,
        _ => false,
    };
    if !valid {
        return Err(InvocationError::Unavailable);
    }
    Ok(g.capabilities)
}
/// Logical complete original frame, including the plan seal, descriptors and
/// ceilings. Bounded before normalization/copy; not allocator/RSS accounting.
fn bytes(
    intent: &CompoundIntent,
    a: &TrustedCompoundAnnotationResultAdmissionV1<'_>,
    maximum: usize,
) -> Result<usize, InvocationError> {
    let mut b = size_of::<Hold>()
        .checked_add(size_of::<CompoundIntent>())
        .ok_or(InvocationError::Limit)?;
    let mut add = |n: usize| {
        b = b
            .checked_add(n)
            .filter(|v| *v <= maximum)
            .ok_or(InvocationError::Limit)?;
        Ok::<(), InvocationError>(())
    };
    add(0)?;
    for s in labels(a.context) {
        if s.is_empty() {
            return Err(InvocationError::Unavailable);
        }
        add(s.len())?;
    }
    for (i, w) in intent.source.writes.iter().enumerate() {
        if !intent.source.writes[..i]
            .iter()
            .any(|v| v.expected.document == w.expected.document)
        {
            add(size_of::<DocumentEdits>())?;
        }
        for e in &w.edits {
            add(size_of::<Replacement>())?;
            add(e.expected.len())?;
            add(e.inserted.len())?;
        }
    }
    for _ in &intent.source.reads {
        add(size_of::<DocumentVersion>())?;
    }
    for m in &intent.annotations {
        add(size_of::<AnnotationMutation>())?;
        add(size_of::<annotation::AnnotationPlan>())?;
        add(size_of::<AnnotationResultCapabilitiesV1>())?;
        add(size_of::<Receipt>())?;
        if let AnnotationMutation::Create { body, .. }
        | AnnotationMutation::UpdateBody { body, .. } = m
        {
            // Original intent and its native plan each retain body bytes.
            add(body.len())?;
            add(body.len())?;
        }
    }
    Ok(b)
}
impl Hold {
    fn funding_intact(&self, ledger: &retention::RetentionState) -> bool {
        ledger
            .validate_exact(&self.reservation, self.charge)
            .is_ok()
            && self
                .base
                .as_ref()
                .is_none_or(|(r, n)| ledger.validate_exact(r, *n).is_ok())
            && self
                .charge
                .checked_add(self.base_charge().unwrap_or(0))
                .is_some_and(|n| ledger.check(0).is_ok_and(|held| held >= n))
    }
    pub(super) fn base_charge(&self) -> Option<usize> {
        self.base.as_ref().map(|(_, n)| *n)
    }
    pub(super) fn bind_base(&mut self, reservation: retention::Reservation, charge: usize) {
        assert!(self.base.is_none(), "one original base reservation");
        self.base = Some((reservation, charge));
    }
    pub(super) fn matches_admission(
        &self,
        owner: &SourceCoordinator,
        id: &InvocationId,
        intent: &CompoundIntent,
        a: &TrustedCompoundAnnotationResultAdmissionV1<'_>,
    ) -> Result<(), InvocationError> {
        if !self.intact(owner, id) {
            return Err(InvocationError::Faulted);
        }
        let b = bytes(intent, a, owner.invocations.limits.max_intent_bytes)?;
        let charge = result_charge(intent.annotations.len(), b, a.max_result_bytes)?;
        if a.context_current != Some(true) || a.result_admission_allowed != Some(true) {
            return Err(InvocationError::Unavailable);
        }
        if self.context != *a.context
            || self.epoch != a.publication_epoch
            || Some(self.publication) != a.publication_allowed
            || self.maximum != a.max_result_bytes
            || self.charge != charge
            || a.grants.len() != intent.annotations.len()
        {
            return Err(InvocationError::IntentMismatch);
        }
        for (i, (m, g)) in intent.annotations.iter().zip(a.grants).enumerate() {
            if self.ceilings.get(i) != Some(&ceiling(id, a.context, i, m, *g, None)?) {
                return Err(InvocationError::IntentMismatch);
            }
        }
        Ok(())
    }
    pub(super) fn release_funding(
        &mut self,
        ledger: &mut retention::RetentionState,
    ) -> Result<(), InvocationError> {
        if !self.funding_intact(ledger) {
            return Err(InvocationError::Faulted);
        }
        ledger.release(&mut self.reservation).map_err(parent)?;
        if let Some((r, _)) = &mut self.base {
            ledger.release(r).map_err(parent)?;
        }
        Ok(())
    }
    pub(super) fn shutdown_reserved_bytes(
        &self,
        owner: &Arc<()>,
        id: &InvocationId,
        ledger: &retention::RetentionState,
    ) -> Result<usize, CoordinatorError> {
        if !Arc::ptr_eq(&self.owner, owner) || self.invocation != *id {
            return Err(CoordinatorError::Faulted);
        }
        if !self.funding_intact(ledger) {
            return Err(CoordinatorError::Faulted);
        }
        self.charge
            .checked_add(self.base_charge().unwrap_or(0))
            .ok_or(CoordinatorError::Faulted)
    }
    pub(super) fn control_context(&self) -> (&SettingsExecutionContextV1, &Arc<()>) {
        (&self.context, &self.owner)
    }
    pub(super) fn pending_intact(
        &self,
        owner: &Arc<()>,
        id: &InvocationId,
        ledger: &retention::RetentionState,
        intent: &CompoundIntent,
        prepared: Result<&PreparedTransaction, &NativePreparationError>,
    ) -> bool {
        Arc::ptr_eq(&self.owner, owner)
            && self.invocation == *id
            && self.funding_intact(ledger)
            && self.ceilings.len() == intent.annotations.len()
            && match prepared {
                Ok(p) => self.plan_matches(intent, p),
                Err(_) => self.plan.is_none(),
            }
    }
    fn plan_matches(&self, intent: &CompoundIntent, p: &PreparedTransaction) -> bool {
        self.plan.as_deref() == Some(p.annotations.as_slice())
            && p.annotations.len() == intent.annotations.len()
            && p.annotations
                .iter()
                .zip(&intent.annotations)
                .all(|(p, m)| p.matches_mutation(m))
            && p.read_dependencies == intent.source.reads
            && p.candidates.len() == intent.source.writes.len()
            && p.candidates
                .iter()
                .zip(&intent.source.writes)
                .all(|(c, w)| c.expected == w.expected && c.edits == w.edits)
    }
    pub(super) fn bind_plan(
        &mut self,
        p: Option<&PreparedTransaction>,
    ) -> Result<(), InvocationError> {
        if let Some(p) = p {
            if p.annotations.len() != self.ceilings.len() {
                return Err(InvocationError::Faulted);
            }
            let mut plans = Vec::new();
            plans
                .try_reserve_exact(p.annotations.len())
                .map_err(|_| InvocationError::Capacity)?;
            plans.extend(p.annotations.iter().cloned());
            self.plan = Some(plans);
        }
        Ok(())
    }
    fn intact(&self, owner: &SourceCoordinator, id: &InvocationId) -> bool {
        Arc::ptr_eq(&self.owner, &owner.namespace)
            && self.invocation == *id
            && self.funding_intact(&owner.retention)
            && owner
                .invocation_slot(id)
                .is_ok_and(|slot| match slot.intent.as_ref() {
                    Some(BoundIntent::AtomicAuthoring(c)) => {
                        self.base_charge() == Some(c.expected_base_charge())
                    }
                    Some(BoundIntent::Compound(_)) => self.base.is_none(),
                    _ => false,
                })
    }
}
fn result_charge(count: usize, bytes: usize, maximum: usize) -> Result<usize, InvocationError> {
    bytes
        .checked_mul(2)
        .and_then(|n| n.checked_add(count.checked_mul(maximum)?.checked_mul(3)?))
        .ok_or(InvocationError::Limit)
}
impl SourceCoordinator {
    /// First-party mixed preparation: retain existing per-component receipt
    /// ceilings without installing a second command or allocating identities.
    pub(super) fn prepare_atomic_result_hold(
        &mut self,
        id: &InvocationId,
        intent: &CompoundIntent,
        a: &TrustedCompoundAnnotationResultAdmissionV1<'_>,
    ) -> Result<Hold, InvocationError> {
        if a.context_current != Some(true) || a.result_admission_allowed != Some(true) {
            return Err(InvocationError::Unavailable);
        }
        let publication = a.publication_allowed.ok_or(InvocationError::Unavailable)?;
        if intent.annotations.is_empty()
            || intent.annotations.len() > self.limits.max_participants
            || a.grants.len() != intent.annotations.len()
            || a.max_result_bytes < AnnotationResultV1::required_bytes()
        {
            return Err(InvocationError::Limit);
        }
        let b = bytes(intent, a, self.invocations.limits.max_intent_bytes)?;
        let charge = result_charge(intent.annotations.len(), b, a.max_result_bytes)?;
        for (i, (m, g)) in intent.annotations.iter().zip(a.grants).enumerate() {
            ceiling(id, a.context, i, m, *g, None)?;
        }
        let mut reservation = Some(self.retention.reserve(charge).map_err(parent)?);
        let result = (|| {
            let context = copy_context(a.context).map_err(|_| InvocationError::Capacity)?;
            let mut ceilings = Vec::new();
            let mut receipts = Vec::new();
            ceilings
                .try_reserve_exact(intent.annotations.len())
                .map_err(|_| InvocationError::Capacity)?;
            receipts
                .try_reserve_exact(intent.annotations.len())
                .map_err(|_| InvocationError::Capacity)?;
            for (i, (m, g)) in intent.annotations.iter().zip(a.grants).enumerate() {
                ceilings.push(ceiling(id, a.context, i, m, *g, None)?);
                receipts.push(Receipt::not_published(id, m));
            }
            Ok(Hold {
                owner: self.namespace.clone(),
                invocation: id.clone(),
                context,
                epoch: a.publication_epoch,
                publication,
                maximum: a.max_result_bytes,
                charge,
                reservation: reservation.take().expect("funded result capture"),
                base: None,
                ceilings,
                plan: None,
                receipts: Some(receipts),
            })
        })();
        if let Some(mut p) = reservation {
            self.retention.release(&mut p).map_err(parent)?;
        }
        result
    }
    /// Trusted embedding only. Result authority does not supply authoring
    /// authority. Uses the existing native compound preparer and publisher.
    pub fn submit_compound_with_annotation_results_v1(
        &mut self,
        id: &InvocationId,
        intent: &CompoundIntent,
        a: &TrustedCompoundAnnotationResultAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        let timing = r.begin_service_timing();
        let mut reservation = None;
        let mut hold = None;
        let mut result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(id)?;
            if a.context_current != Some(true) || a.result_admission_allowed != Some(true) {
                return Err(InvocationError::Unavailable);
            }
            let publication = a.publication_allowed.ok_or(InvocationError::Unavailable)?;
            if intent.annotations.is_empty()
                || intent.annotations.len() > self.limits.max_participants
                || a.grants.len() != intent.annotations.len()
            {
                return Err(InvocationError::Limit);
            }
            if a.max_result_bytes < AnnotationResultV1::required_bytes() {
                return Err(InvocationError::Limit);
            }
            self.source_intent_bytes(&intent.source)?;
            let b = bytes(intent, a, self.invocations.limits.max_intent_bytes)?;
            let charge = b
                .checked_mul(2)
                .and_then(|n| {
                    n.checked_add(
                        intent
                            .annotations
                            .len()
                            .checked_mul(a.max_result_bytes)?
                            .checked_mul(3)?,
                    )
                })
                .ok_or(InvocationError::Limit)?;
            if slot.intent.is_some() {
                let old = slot
                    .compound_results
                    .as_ref()
                    .ok_or(InvocationError::IntentMismatch)?;
                if !old.intact(self, id) {
                    return Err(InvocationError::Faulted);
                }
                if old.context != *a.context
                    || old.epoch != a.publication_epoch
                    || old.publication != publication
                    || old.maximum != a.max_result_bytes
                    || old.charge != charge
                {
                    return Err(InvocationError::IntentMismatch);
                }
                for (i, (m, g)) in intent.annotations.iter().zip(a.grants).enumerate() {
                    if old.ceilings.get(i) != Some(&ceiling(id, a.context, i, m, *g, None)?) {
                        return Err(InvocationError::IntentMismatch);
                    }
                }
                return self.submit_compound(id, intent, r);
            }
            for (i, (m, g)) in intent.annotations.iter().zip(a.grants).enumerate() {
                ceiling(id, a.context, i, m, *g, None)?;
            }
            reservation = Some(self.retention.reserve(charge).map_err(parent)?);
            let context = copy_context(a.context).map_err(|_| InvocationError::Capacity)?;
            let mut ceilings = Vec::new();
            let mut receipts = Vec::new();
            ceilings
                .try_reserve_exact(intent.annotations.len())
                .map_err(|_| InvocationError::Capacity)?;
            receipts
                .try_reserve_exact(intent.annotations.len())
                .map_err(|_| InvocationError::Capacity)?;
            for (i, (m, g)) in intent.annotations.iter().zip(a.grants).enumerate() {
                ceilings.push(ceiling(id, a.context, i, m, *g, None)?);
                receipts.push(Receipt::not_published(id, m));
            }
            hold = Some(Hold {
                owner: self.namespace.clone(),
                invocation: id.clone(),
                context,
                epoch: a.publication_epoch,
                publication,
                maximum: a.max_result_bytes,
                charge,
                reservation: reservation.take().expect("funded compound"),
                base: None,
                ceilings,
                plan: None,
                receipts: Some(receipts),
            });
            self.submit_compound_owned(id, intent, None, None, &mut hold, r)
        })();
        if let Some(h) = hold {
            reservation = Some(h.reservation)
        }
        if let Some(mut p) = reservation
            && self.retention.release(&mut p).is_err()
        {
            result = Err(InvocationError::Faulted)
        }
        if result == Err(InvocationError::Faulted) {
            self.faulted = true
        }
        capture_timed(
            r,
            Boundary::AnnotationResultAdmission,
            &result,
            timing.finish(),
        );
        result
    }

    pub(in crate::coordinator) fn stage_compound_annotation_receipts(
        &self,
        id: &InvocationId,
        p: &PreparedTransaction,
        staged: &annotation::AnnotationDecision,
        published: bool,
        r: &mut Recorder,
    ) -> Result<Vec<Receipt>, CoordinatorError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let s = self
                .invocation_slot(id)
                .map_err(|_| CoordinatorError::Faulted)?;
            let h = s
                .compound_results
                .as_ref()
                .ok_or(CoordinatorError::Faulted)?;
            let Some(intent) = s
                .intent
                .as_ref()
                .and_then(BoundIntent::compound_result_intent)
            else {
                return Err(CoordinatorError::Faulted);
            };
            if !h.intact(self, id)
                || !matches!(s.phase, Phase::Running)
                || h.ceilings.len() != intent.annotations.len()
                || !h.plan_matches(intent, p)
            {
                return Err(CoordinatorError::Faulted);
            }
            let mut out = Vec::new();
            out.try_reserve_exact(p.annotations.len())
                .map_err(|_| CoordinatorError::Capacity)?;
            for (plan, m) in p.annotations.iter().zip(&intent.annotations) {
                let f = staged.component_fact(self, &plan.id)?;
                let operation = Operation::of(m);
                if f.content_version == 0
                    || f.target_generation == 0
                    || f.present == (operation == Operation::Delete)
                    || (!published && matches!(operation, Operation::Create | Operation::Delete))
                {
                    return Err(CoordinatorError::Faulted);
                }
                let record = Record {
                    annotation: f.annotation,
                    present: f.present,
                    content_version: f.content_version,
                    target_generation: f.target_generation,
                };
                // Applied means this whole compound was published, not that
                // every field changed. All rows describe the final candidate.
                let decision = if published {
                    Decision::Applied {
                        publication: id.clone(),
                        record,
                    }
                } else {
                    Decision::Unchanged { record }
                };
                out.push(Receipt {
                    invocation: id.clone(),
                    operation,
                    decision,
                });
            }
            Ok(out)
        })();
        observe_service(
            r,
            Boundary::AnnotationDecisionStaging,
            &result,
            timing.finish(),
        );
        result
    }

    pub fn retained_compound_annotation_result_v1(
        &self,
        id: &InvocationId,
        index: usize,
        f: &TrustedCompoundAnnotationResultReleaseV1<'_>,
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
            let s = self.invocation_slot(id)?;
            let h = s
                .compound_results
                .as_ref()
                .ok_or(InvocationError::OutcomeUnavailable)?;
            if h.context != *f.context || h.epoch != f.publication_epoch || !h.publication {
                return Err(InvocationError::Unavailable);
            }
            if !h.intact(self, id) {
                return Err(InvocationError::Faulted);
            }
            if f.max_result_bytes < AnnotationResultV1::required_bytes() {
                return Err(InvocationError::Limit);
            }
            let Some(intent) = s
                .intent
                .as_ref()
                .and_then(BoundIntent::compound_result_intent)
            else {
                return Err(InvocationError::Faulted);
            };
            let m = intent
                .annotations
                .get(index)
                .ok_or(InvocationError::Unavailable)?;
            let Phase::Terminal(Some(outcome)) = &s.phase else {
                return Err(InvocationError::OutcomeUnavailable);
            };
            let receipts = h
                .receipts
                .as_ref()
                .ok_or(InvocationError::OutcomeUnavailable)?;
            if receipts.len() != intent.annotations.len() || h.ceilings.len() != receipts.len() {
                return Err(InvocationError::Faulted);
            }
            let receipt = &receipts[index];
            if receipt.invocation != *id || receipt.operation != Operation::of(m) {
                return Err(InvocationError::Faulted);
            }
            let project = |record: &Record| -> Result<AnnotationResultRecordV1, InvocationError> {
                if h.plan.as_ref().and_then(|p| p.get(index)).map(|p| &p.id)
                    != Some(&record.annotation)
                    || record.content_version == 0
                    || record.target_generation == 0
                    || record.present == (receipt.operation == Operation::Delete)
                {
                    return Err(InvocationError::Faulted);
                }
                let bits = h.ceilings[index]
                    .meet(ceiling(
                        id,
                        &h.context,
                        index,
                        m,
                        f.grant,
                        Some(&record.annotation),
                    )?)
                    .bits();
                if bits & 1 == 0 {
                    return Err(InvocationError::Unavailable);
                }
                Ok(AnnotationResultRecordV1 {
                    annotation: record.annotation.clone(),
                    present: record.present,
                    content_version: (bits & 2 != 0).then_some(record.content_version),
                    target_generation: (bits & 4 != 0).then_some(record.target_generation),
                })
            };
            let decision = match (&receipt.decision, outcome) {
                (
                    Decision::NotPublished,
                    InvocationOutcome::StoppedBeforePublication | InvocationOutcome::Rejected(_),
                ) => AnnotationResultDecisionV1::NotPublished,
                (
                    Decision::Applied {
                        publication,
                        record,
                    },
                    InvocationOutcome::Committed(coarse),
                ) if publication == id
                    && coarse.operation.is_some()
                    && coarse
                        .annotation_changes
                        .iter()
                        .filter(|f| f.annotation == record.annotation)
                        .all(|f| {
                            f.present == record.present
                                && f.content_version == record.content_version
                                && f.target_generation == record.target_generation
                        }) =>
                {
                    AnnotationResultDecisionV1::Applied {
                        publication: publication.clone(),
                        record: project(record)?,
                    }
                }
                (Decision::Unchanged { record }, InvocationOutcome::Committed(coarse))
                    if coarse.operation.is_none()
                        && coarse.changes.is_empty()
                        && coarse.annotation_changes.is_empty()
                        && matches!(receipt.operation, Operation::Body | Operation::Target) =>
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

    pub fn forget_compound_annotation_results_v1(
        &mut self,
        id: &InvocationId,
        f: &CompoundAnnotationResultEvictionV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let s = self.invocation_slot(id)?;
            let h = s
                .compound_results
                .as_ref()
                .ok_or(InvocationError::Unavailable)?;
            if h.context != *f.context
                || f.retention_released != Some(true)
                || f.delivery_drained != Some(true)
                || s.created_results.held()
            {
                return Err(InvocationError::Unavailable);
            }
            if !h.intact(self, id) {
                return Err(InvocationError::Faulted);
            }
            if !matches!(s.phase, Phase::Terminal(_)) {
                return Err(InvocationError::Pending);
            }
            let s = self.invocations.slots[id.slot]
                .as_mut()
                .expect("terminal compound");
            s.compound_results
                .as_mut()
                .expect("funded compound")
                .receipts = None;
            s.phase = Phase::Terminal(None);
            Ok(())
        })();
        capture_timed(r, Boundary::Control, &result, timing.finish());
        result
    }
    pub fn retire_compound_annotation_results_v1(
        &mut self,
        id: &InvocationId,
        f: &InvocationObservationPolicyControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let s = self.invocation_slot(id)?;
            let h = s
                .compound_results
                .as_ref()
                .ok_or(InvocationError::Unavailable)?;
            if h.context != *f.context
                || [
                    f.owner_control_allowed,
                    f.retention_released,
                    f.delivery_drained,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(InvocationError::Unavailable);
            }
            if !h.intact(self, id) {
                return Err(InvocationError::Faulted);
            }
            if !(matches!(s.phase, Phase::Terminal(_))
                || self.faulted && matches!(s.phase, Phase::Running))
            {
                return Err(InvocationError::Pending);
            }
            let s = self.invocations.slots[id.slot]
                .as_mut()
                .expect("controlled compound");
            s.compound_results
                .as_mut()
                .expect("funded compound")
                .release_funding(&mut self.retention)?;
            if matches!(s.intent, Some(BoundIntent::AtomicAuthoring(_))) {
                s.intent = Some(BoundIntent::RetiredAtomicAuthoring);
                if matches!(s.phase, Phase::Terminal(_)) {
                    s.phase = Phase::Terminal(None);
                }
            }
            s.compound_results = None;
            Ok(())
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true
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
