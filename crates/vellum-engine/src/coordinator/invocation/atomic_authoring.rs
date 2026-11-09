//! One funded native mixed command, not sequential standalone F13 commands.
use super::*;
use crate::capture::CaptureLimitsV1;
use crate::coordinator::settings::basis;
use std::mem::size_of;
mod arguments;
#[cfg(test)]
mod tests;
pub use arguments::*;

pub struct TrustedAtomicAnnotationBindingV1<'a> {
    pub component: usize,
    pub invocation: &'a InvocationId,
    pub document: &'a DocumentId,
    pub operation: AnnotationResultOperationV1,
    pub native: &'a TrustedAnnotationInvocationV1<'a>,
}
/// Trusted current premises, not requester data or retained authoring grants.
pub struct TrustedAtomicAuthoringV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub components: &'a [TrustedAtomicAnnotationBindingV1<'a>],
    pub source_readable: Option<bool>,
    pub source_writable: Option<bool>,
    pub all_targets_writable: Option<bool>,
}
pub(super) struct Component {
    document: DocumentId,
    original: AnnotationMutation,
    caller: read_dependency::ReadCapture,
    required: read_dependency::ReadCapture,
    combined: read_dependency::ReadCapture,
    limits: AnnotationInvocationLimitsV1,
    allocation: Option<AnnotationId>,
}
pub(super) struct Capture {
    owner: Arc<()>,
    invocation: InvocationId,
    actor: SettingsExecutionContextV1,
    intent: CompoundIntent,
    reads: read_dependency::ReadCapture,
    scopes: scope::ScopeCapture,
    components: Vec<Component>,
    limits: CaptureLimitsV1,
    native: annotation_scoped::NativeProfile,
    base_charge: usize,
}
impl std::fmt::Debug for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AtomicAuthoring(<owned:redacted>)")
    }
}
fn operation(m: &AnnotationMutation) -> AnnotationResultOperationV1 {
    annotation_scoped::receipt::Operation::of(m).into()
}
fn parent(e: RetentionErrorV1) -> InvocationError {
    match e {
        RetentionErrorV1::Faulted => InvocationError::Faulted,
        RetentionErrorV1::Limit => InvocationError::Limit,
        RetentionErrorV1::Unavailable => InvocationError::Unavailable,
    }
}
fn read_error(e: ReadDependencyError) -> InvocationError {
    match e {
        ReadDependencyError::Capacity => InvocationError::Capacity,
        ReadDependencyError::Limit => InvocationError::Limit,
        _ => InvocationError::MalformedIntent,
    }
}
fn add(a: usize, b: usize) -> Result<usize, InvocationError> {
    a.checked_add(b).ok_or(InvocationError::Limit)
}
fn body_bytes(m: &AnnotationMutation) -> usize {
    match m {
        AnnotationMutation::Create { body, .. } | AnnotationMutation::UpdateBody { body, .. } => {
            body.len()
        }
        _ => 0,
    }
}
fn binding<'a>(
    id: &InvocationId,
    c: &SettingsExecutionContextV1,
    count: usize,
    index: usize,
    document: &DocumentId,
    mutation: &AnnotationMutation,
    f: &'a TrustedAtomicAuthoringV1<'a>,
) -> Result<&'a TrustedAnnotationInvocationV1<'a>, InvocationError> {
    if f.context != c || f.components.len() != count {
        return Err(InvocationError::Unavailable);
    }
    let b = f
        .components
        .get(index)
        .ok_or(InvocationError::Unavailable)?;
    if b.component != index
        || b.invocation != id
        || b.document != document
        || b.operation != operation(mutation)
        || b.native.context != c
    {
        return Err(InvocationError::Unavailable);
    }
    Ok(b.native)
}
impl BoundIntent {
    pub(super) fn compound_result_intent(&self) -> Option<&CompoundIntent> {
        match self {
            Self::Compound(c) => Some(c),
            Self::AtomicAuthoring(c) => Some(&c.intent),
            _ => None,
        }
    }
}
impl Capture {
    pub(super) fn expected_base_charge(&self) -> usize {
        self.base_charge
    }
    pub(super) fn control_context(&self) -> (&SettingsExecutionContextV1, &Arc<()>) {
        (&self.actor, &self.owner)
    }
    pub(super) fn refers_to_document(&self, d: &DocumentId) -> bool {
        self.intent.source.refers_to_document(d)
            || self.reads.refers_to_document(d)
            || self.scopes.scopes.iter().any(|s| s.expected.document == *d)
            || self.components.iter().any(|c| {
                c.document == *d
                    || c.caller.refers_to_document(d)
                    || c.required.refers_to_document(d)
                    || c.combined.refers_to_document(d)
            })
    }
    pub(super) fn matches_plan(
        &self,
        owner: &Arc<()>,
        id: &InvocationId,
        p: &PreparedTransaction,
        hold: Option<&compound_results::Hold>,
        ledger: &retention::RetentionState,
    ) -> bool {
        Arc::ptr_eq(&self.owner, owner)
            && self.invocation == *id
            && Arc::ptr_eq(&p.namespace, owner)
            && p.reads_required
            && p.declared_reads.as_ref() == Some(&self.reads)
            && p.scoped
            && p.scopes.as_ref() == Some(&self.scopes)
            && p.derived_kind.is_none()
            && p.derived_edit.is_empty()
            && p.view_dependencies.is_empty()
            && p.configured_line.is_none()
            && !p.configured_required
            && p.search_dependency.is_none()
            && matches!(p.history, history::PreparedHistory::Ordinary)
            && self.components.len() == p.annotations.len()
            && self
                .components
                .iter()
                .zip(&p.annotations)
                .all(|(c, p)| c.allocation.as_ref() == Some(&p.id))
            && hold.is_some_and(|h| {
                h.base_charge() == Some(self.base_charge)
                    && h.pending_intact(owner, id, ledger, &self.intent, Ok(p))
            })
    }
    fn same_request(&self, q: &NormalizedAtomicAuthoringArgumentsV1<'_>) -> bool {
        self.invocation == *q.source.invocation
            && self.actor == q.source.actor
            && self.intent.source == q.source.source
            && self.reads == q.source.reads
            && self.scopes == q.source.scopes
            && self.limits == q.source.limits
            && self.components.len() == q.components.len()
            && self.components.iter().zip(&q.components).all(|(c, q)| {
                c.document == q.document && c.original == q.mutation && c.caller == q.reads
            })
    }
    fn current(
        &self,
        owner: &SourceCoordinator,
        f: &TrustedAtomicAuthoringV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        if !self.intent.source.writes.is_empty()
            && [f.source_readable, f.source_writable, f.all_targets_writable]
                .into_iter()
                .any(|v| v != Some(true))
        {
            return Err(CoordinatorError::Unavailable);
        }
        if owner
            .annotation_profile()
            .map_err(|_| CoordinatorError::Stale)?
            != self.native
        {
            return Err(CoordinatorError::Stale);
        }
        for (i, c) in self.components.iter().enumerate() {
            let native = binding(
                &self.invocation,
                &self.actor,
                self.components.len(),
                i,
                &c.document,
                &c.original,
                f,
            )
            .map_err(|_| CoordinatorError::Unavailable)?;
            if !native.allowed(&self.actor) {
                return Err(CoordinatorError::Unavailable);
            }
            if native.limits != c.limits {
                return Err(CoordinatorError::Stale);
            }
            if read_dependency::ReadCapture::size_for_predicates(native.required_reads.len())
                .is_none_or(|n| {
                    n > c
                        .limits
                        .max_capture_bytes
                        .min(owner.invocations.limits.max_intent_bytes)
                })
            {
                return Err(CoordinatorError::Limit);
            }
            let required = read_dependency::ReadCapture::configuration_slice(
                native.required_reads,
                c.required.budget,
            )
            .map_err(CoordinatorError::ReadDependency)?;
            if required != c.required {
                return Err(CoordinatorError::Stale);
            }
            owner.check_owned_read_capture(&c.combined, r)?;
        }
        Ok(())
    }
    fn bytes(&self) -> Result<usize, InvocationError> {
        let mut n = size_of::<Self>();
        for label in basis::labels(&self.actor) {
            n = add(n, label.len())?;
        }
        for w in &self.intent.source.writes {
            n = add(n, size_of::<DocumentEdits>())?;
            for e in &w.edits {
                n = add(
                    n,
                    add(
                        size_of::<Replacement>(),
                        add(e.expected.len(), e.inserted.len())?,
                    )?,
                )?;
            }
        }
        n = add(n, self.reads.logical_bytes().ok_or(InvocationError::Limit)?)?;
        n = add(
            n,
            scope::ScopeCapture::retained_bytes(&self.scopes.scopes)
                .ok_or(InvocationError::Limit)?,
        )?;
        for (c, lowered) in self.components.iter().zip(&self.intent.annotations) {
            n = add(
                n,
                add(size_of::<Component>(), size_of::<AnnotationMutation>())?,
            )?;
            n = add(n, add(body_bytes(&c.original), body_bytes(lowered))?)?;
            for read in [&c.caller, &c.required, &c.combined] {
                n = add(n, read.logical_bytes().ok_or(InvocationError::Limit)?)?;
            }
        }
        Ok(n)
    }
}
pub(super) fn check_service_envelope(
    o: &SourceCoordinator,
    id: &InvocationId,
    p: &PreparedTransaction,
) -> Result<(), CoordinatorError> {
    let Some(BoundIntent::AtomicAuthoring(c)) = o
        .invocation_slot(id)
        .map_err(|_| CoordinatorError::Faulted)?
        .intent
        .as_ref()
    else {
        return Err(CoordinatorError::Faulted);
    };
    let n = envelope_bytes(c, p)?;
    if n > o.invocations.limits.max_intent_bytes {
        return Err(CoordinatorError::Limit);
    }
    Ok(())
}
fn envelope_bytes(c: &Capture, p: &PreparedTransaction) -> Result<usize, CoordinatorError> {
    let mut n = add(
        c.bytes().map_err(|_| CoordinatorError::Limit)?,
        size_of::<PreparedTransaction>(),
    )
    .map_err(|_| CoordinatorError::Limit)?;
    for candidate in &p.candidates {
        n = n
            .checked_add(size_of::<Candidate>())
            .and_then(|n| n.checked_add(candidate.source.text().len()))
            .ok_or(CoordinatorError::Limit)?;
        for e in &candidate.edits {
            n = n
                .checked_add(size_of::<Replacement>())
                .and_then(|n| n.checked_add(e.expected.len()))
                .and_then(|n| n.checked_add(e.inserted.len()))
                .ok_or(CoordinatorError::Limit)?;
        }
    }
    for (plan, mutation) in p.annotations.iter().zip(&c.intent.annotations) {
        let _ = plan;
        n = n
            .checked_add(size_of::<annotation::AnnotationPlan>())
            .and_then(|n| n.checked_add(body_bytes(mutation)))
            .ok_or(CoordinatorError::Limit)?;
    }
    for v in &p.view_dependencies {
        n = n
            .checked_add(size_of::<views::ViewDependency>())
            .and_then(|n| {
                n.checked_add(
                    v.selection_count()
                        .checked_mul(size_of::<crate::selection::Selection>())?,
                )
            })
            .ok_or(CoordinatorError::Limit)?;
    }
    n = n
        .checked_add(c.reads.logical_bytes().ok_or(CoordinatorError::Limit)?)
        .and_then(|n| n.checked_add(scope::ScopeCapture::retained_bytes(&c.scopes.scopes)?))
        .ok_or(CoordinatorError::Limit)?;
    Ok(n)
}
impl SourceCoordinator {
    pub fn submit_atomic_authoring_arguments_v1(
        &mut self,
        q: &NormalizedAtomicAuthoringArgumentsV1<'_>,
        f: &TrustedAtomicAuthoringV1<'_>,
        a: &TrustedCompoundAnnotationResultAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timer = r.begin_service_timing();
        let mut base = None;
        let mut hold: Option<compound_results::Hold> = None;
        let result = (|| {
            let id = q.source.invocation;
            self.guard_shutdown_invocation_admission(id, r)?;
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(id)?;
            if let Some(original) = &slot.intent {
                let BoundIntent::AtomicAuthoring(c) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                if !c.same_request(q) {
                    return Err(InvocationError::IntentMismatch);
                }
                for (i, cpt) in c.components.iter().enumerate() {
                    let native = binding(
                        id,
                        &c.actor,
                        c.components.len(),
                        i,
                        &cpt.document,
                        &cpt.original,
                        f,
                    )?;
                    if read_dependency::ReadCapture::size_for_predicates(
                        native.required_reads.len(),
                    )
                    .is_none_or(|n| {
                        n > cpt
                            .limits
                            .max_capture_bytes
                            .min(self.invocations.limits.max_intent_bytes)
                    }) {
                        return Err(InvocationError::Limit);
                    }
                    let required = read_dependency::ReadCapture::configuration_slice(
                        native.required_reads,
                        cpt.required.budget,
                    )
                    .map_err(read_error)?;
                    if native.limits != cpt.limits || required != cpt.required {
                        return Err(InvocationError::IntentMismatch);
                    }
                }
                let h = slot
                    .compound_results
                    .as_ref()
                    .ok_or(InvocationError::Unavailable)?;
                h.matches_admission(self, id, &c.intent, a)?;
                return slot.phase.state();
            }
            if f.context != &q.source.actor || a.context != f.context {
                return Err(InvocationError::Unavailable);
            }
            if !q.source.source.writes.is_empty()
                && [f.source_readable, f.source_writable, f.all_targets_writable]
                    .into_iter()
                    .any(|v| v != Some(true))
            {
                return Err(InvocationError::Unavailable);
            }
            let native_profile = self.annotation_profile()?;
            let base_charge = q
                .bytes
                .checked_mul(3)
                .and_then(|n| {
                    n.checked_add(self.invocations.limits.max_intent_bytes.checked_mul(2)?)
                })
                .and_then(|n| n.checked_add(native_profile.root.checked_mul(2)?))
                .ok_or(InvocationError::Limit)?;
            base = Some(self.retention.reserve(base_charge).map_err(parent)?);
            let actor = basis::copy_context(f.context).map_err(|_| InvocationError::Capacity)?;
            let mut components = Vec::new();
            let mut mutations = Vec::new();
            components
                .try_reserve_exact(q.components.len())
                .map_err(|_| InvocationError::Capacity)?;
            mutations
                .try_reserve_exact(q.components.len())
                .map_err(|_| InvocationError::Capacity)?;
            for (i, raw) in q.components.iter().enumerate() {
                let native = binding(
                    id,
                    &actor,
                    q.components.len(),
                    i,
                    &raw.document,
                    &raw.mutation,
                    f,
                )?;
                if !native.allowed(&actor) {
                    return Err(InvocationError::Unavailable);
                }
                let context_bytes = basis::labels(&actor)
                    .into_iter()
                    .try_fold(size_of::<SettingsExecutionContextV1>(), |n, s| {
                        add(n, s.len())
                    })?;
                if context_bytes > native.limits.max_context_bytes
                    || read_dependency::ReadCapture::size_for_predicates(
                        native.required_reads.len(),
                    )
                    .is_none_or(|n| {
                        n > native
                            .limits
                            .max_capture_bytes
                            .min(self.invocations.limits.max_intent_bytes)
                    })
                {
                    return Err(InvocationError::Limit);
                }
                let caller = raw.reads.copy_owned().map_err(read_error)?;
                let required = read_dependency::ReadCapture::configuration_slice(
                    native.required_reads,
                    caller.budget,
                )
                .map_err(read_error)?;
                let combined = caller.conjoin(&required).map_err(read_error)?;
                self.check_owned_read_capture(&combined, r)
                    .map_err(|_| InvocationError::Unavailable)?;
                let mut captured = add(
                    size_of::<Component>(),
                    add(context_bytes, body_bytes(&raw.mutation))?,
                )?;
                for read in [&caller, &required, &combined] {
                    captured = add(
                        captured,
                        read.logical_bytes().ok_or(InvocationError::Limit)?,
                    )?;
                }
                let planned = add(
                    captured,
                    add(
                        size_of::<PreparedTransaction>(),
                        add(
                            size_of::<annotation::AnnotationPlan>(),
                            body_bytes(&raw.mutation),
                        )?,
                    )?,
                )?;
                if captured > native.limits.max_capture_bytes
                    || planned > native.limits.max_plan_bytes
                    || planned > native.limits.max_construction_bytes
                {
                    return Err(InvocationError::Limit);
                }
                let mut lowered = annotation_scoped::copy_mutation(&raw.mutation)?;
                if let AnnotationMutation::Create { target, .. }
                | AnnotationMutation::Reattach { target, .. } = &mut lowered
                    && target.basis == TargetBasis::PreparedSource
                    && !q
                        .source
                        .source
                        .writes
                        .iter()
                        .any(|w| w.expected.document == target.source.document)
                {
                    target.basis = TargetBasis::Before;
                }
                mutations.push(lowered);
                components.push(Component {
                    document: raw.document.clone(),
                    original: annotation_scoped::copy_mutation(&raw.mutation)?,
                    caller,
                    required,
                    combined,
                    limits: native.limits,
                    allocation: None,
                });
            }
            let mut c = Capture {
                owner: self.namespace.clone(),
                invocation: id.clone(),
                actor,
                intent: CompoundIntent {
                    source: self.normalize_intent_read_profile(&q.source.source, false)?,
                    annotations: mutations,
                },
                reads: q.source.reads.copy_owned().map_err(read_error)?,
                scopes: scope::ScopeCapture::copy(&q.source.scopes.scopes, q.source.scopes.budget)
                    .map_err(|_| InvocationError::Limit)?,
                components,
                limits: q.source.limits,
                native: native_profile,
                base_charge,
            };
            if c.bytes()? > self.invocations.limits.max_intent_bytes {
                return Err(InvocationError::Limit);
            }
            self.check_owned_read_capture(&c.reads, r)
                .map_err(|_| InvocationError::Unavailable)?;
            self.check_scoped_source_for_invocation(&c.intent.source, &c.scopes, r)
                .map_err(|_| InvocationError::Unavailable)?;
            hold = Some(self.prepare_atomic_result_hold(id, &c.intent, a)?);
            let mut plan = self
                .prepare_atomic_authoring_candidates(
                    &c.intent.source.writes,
                    &c.intent.annotations,
                    r,
                )
                .map_err(|e| match e {
                    CoordinatorError::Faulted => InvocationError::Faulted,
                    CoordinatorError::Capacity => InvocationError::Capacity,
                    CoordinatorError::Limit => InvocationError::Limit,
                    _ => InvocationError::Unavailable,
                })?;
            plan.scoped = true;
            plan.scopes = Some(
                scope::ScopeCapture::copy(&c.scopes.scopes, c.scopes.budget)
                    .map_err(|_| InvocationError::Capacity)?,
            );
            plan.reads_required = true;
            plan.declared_reads = Some(c.reads.copy_owned().map_err(read_error)?);
            for (component, plan) in c.components.iter_mut().zip(&plan.annotations) {
                component.allocation = Some(plan.id.clone());
            }
            if envelope_bytes(&c, &plan).map_err(|_| InvocationError::Limit)?
                > self.invocations.limits.max_intent_bytes
            {
                return Err(InvocationError::Limit);
            }
            let h = hold.as_mut().expect("funded receipts");
            h.bind_plan(Some(&plan))?;
            h.bind_base(base.take().expect("funded mixed preparation"), base_charge);
            if !c.matches_plan(&self.namespace, id, &plan, Some(h), &self.retention) {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .expect("validated issued slot");
            let Phase::Issued(reserved) = std::mem::replace(&mut slot.phase, Phase::Running) else {
                unreachable!("unbound mixed slot")
            };
            slot.compound_results = hold.take();
            self.install_invocation_admission(
                id,
                BoundIntent::AtomicAuthoring(Arc::new(c)),
                Phase::Pending {
                    prepared: Ok(plan),
                    reserved,
                },
            );
            Ok(InvocationState::Pending)
        })();
        let mut result = result;
        if let Some(mut h) = hold
            && h.release_funding(&mut self.retention).is_err()
        {
            result = Err(InvocationError::Faulted);
        }
        if let Some(mut b) = base
            && self.retention.release(&mut b).is_err()
        {
            result = Err(InvocationError::Faulted);
        }
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(
            r,
            Boundary::AtomicAuthoringAdmission,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn run_atomic_authoring_v1(
        &mut self,
        id: &InvocationId,
        f: &TrustedAtomicAuthoringV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(id)?;
            let Some(BoundIntent::AtomicAuthoring(c)) = &slot.intent else {
                return Err(InvocationError::Unavailable);
            };
            if f.context != &c.actor {
                return Err(InvocationError::Unavailable);
            }
            if matches!(slot.phase, Phase::Terminal(_)) {
                return Ok(InvocationState::Finished);
            }
            if self.service_requested_stop(id, r)? {
                return Ok(InvocationState::Finished);
            }
            let Some(BoundIntent::AtomicAuthoring(c)) = self.invocation_slot(id)?.intent.as_ref()
            else {
                return Err(InvocationError::Faulted);
            };
            let validation = c.current(self, f, r);
            self.run_with_atomic_validation(id, Some(validation), r)
        })();
        capture_timed(r, Boundary::AtomicAuthoringService, &result, timer.finish());
        result
    }
}
