//! Explicit source authoring: private companion stamps, current-service views.
use super::*;
use crate::capture::CaptureLimitsV1;
use crate::coordinator::settings::basis;
use std::mem::size_of;
pub(super) mod arguments;
#[cfg(test)]
mod tests;
pub use arguments::*;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ExactBatchIntent {
    pub(super) owner: Arc<()>,
    pub(super) actor: SettingsExecutionContextV1,
    source: SourceIntent,
    reads: read_dependency::ReadCapture,
    scopes: scope::ScopeCapture,
    limits: CaptureLimitsV1,
}
impl ExactBatchIntent {
    pub(super) fn matches_plan(&self, plan: &PreparedTransaction) -> bool {
        Arc::ptr_eq(&self.owner, &plan.namespace)
            && plan.scoped
            && plan.scopes.as_ref() == Some(&self.scopes)
            && plan.reads_required
            && plan.declared_reads.as_ref() == Some(&self.reads)
            && plan.read_dependencies.is_empty()
            && plan.annotations.is_empty()
            && plan.derived_kind.is_none()
            && plan.derived_edit.is_empty()
            && !plan.configured_required
            && plan.configured_line.is_none()
            && plan.search_dependency.is_none()
            && plan.view_dependencies.is_empty()
            && matches!(plan.history, history::PreparedHistory::Ordinary)
            && plan.candidates.len() == self.source.writes.len()
            && plan
                .candidates
                .iter()
                .zip(&self.source.writes)
                .all(|(c, w)| c.expected == w.expected && c.edits == w.edits)
    }
    pub(super) fn refers_to_document(&self, d: &DocumentId) -> bool {
        self.source.refers_to_document(d)
            || self.reads.refers_to_document(d)
            || self.scopes.scopes.iter().any(|s| s.expected.document == *d)
    }
}
fn coordinator_error(e: CoordinatorError) -> InvocationError {
    match e {
        CoordinatorError::Faulted => InvocationError::Faulted,
        CoordinatorError::Capacity => InvocationError::Capacity,
        CoordinatorError::Limit => InvocationError::Limit,
        _ => InvocationError::Unavailable,
    }
}
/// Native retained graph, not caller ingress, allocator capacity, stack or RSS.
/// Every optional field admitted by matches_plan is covered explicitly here.
fn envelope_bytes(
    intent: &ExactBatchIntent,
    plan: &PreparedTransaction,
) -> Result<usize, InvocationError> {
    let mut n =
        arguments::intent_bytes(&intent.source, &intent.reads, &intent.scopes, &intent.actor)?;
    let mut add = |bytes: usize| -> Result<(), InvocationError> {
        n = n.checked_add(bytes).ok_or(InvocationError::Limit)?;
        Ok(())
    };
    add(size_of::<PreparedTransaction>())?;
    for candidate in &plan.candidates {
        add(size_of::<Candidate>())?;
        add(candidate.source.text().len())?;
        for edit in &candidate.edits {
            add(size_of::<Replacement>())?;
            add(edit.expected.len())?;
            add(edit.inserted.len())?;
        }
    }
    add(intent
        .reads
        .logical_bytes()
        .ok_or(InvocationError::Limit)?
        .checked_sub(size_of::<read_dependency::ReadCapture>())
        .ok_or(InvocationError::Limit)?)?;
    add(scope::ScopeCapture::retained_bytes(&intent.scopes.scopes)
        .ok_or(InvocationError::Limit)?
        .checked_sub(size_of::<scope::ScopeCapture>())
        .ok_or(InvocationError::Limit)?)?;
    for view in &plan.view_dependencies {
        add(size_of::<views::ViewDependency>())?;
        add(view
            .selection_count()
            .checked_mul(size_of::<crate::selection::Selection>())
            .ok_or(InvocationError::Limit)?)?;
    }
    Ok(n)
}
pub(super) fn check_service_envelope(
    owner: &SourceCoordinator,
    id: &InvocationId,
    plan: &PreparedTransaction,
) -> Result<(), CoordinatorError> {
    let Some(BoundIntent::ExactBatch(intent)) = owner
        .invocation_slot(id)
        .map_err(|_| CoordinatorError::Faulted)?
        .intent
        .as_ref()
    else {
        return Err(CoordinatorError::Faulted);
    };
    let bytes = envelope_bytes(intent, plan).map_err(|_| CoordinatorError::Limit)?;
    if bytes > owner.invocations.limits.max_intent_bytes {
        return Err(CoordinatorError::Limit);
    }
    Ok(())
}
impl SourceCoordinator {
    /// Trusted embedding port, not a grant for arbitrary scripts. The embedding
    /// authorizes admission and service independently. No private annotation-set
    /// stamp or publishable candidate is returned to the caller.
    pub fn submit_exact_batch_arguments_v1(
        &mut self,
        q: &NormalizedExactBatchArgumentsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.guard_shutdown_invocation_admission(q.invocation, r)?;
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(q.invocation)?;
            if let Some(original) = &slot.intent {
                if !matches!(original, BoundIntent::ExactBatch(c) if c.source == q.source && c.reads == q.reads && c.scopes == q.scopes && c.actor == q.actor && c.limits == q.limits)
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            if arguments::intent_bytes(&q.source, &q.reads, &q.scopes, &q.actor)?
                > self
                    .invocations
                    .limits
                    .max_intent_bytes
                    .min(q.limits.max_bytes)
            {
                return Err(InvocationError::Limit);
            }
            let intent = ExactBatchIntent {
                owner: self.namespace.clone(),
                actor: basis::copy_context(&q.actor).map_err(|_| InvocationError::Capacity)?,
                source: self.normalize_intent_read_profile(&q.source, false)?,
                reads: q
                    .reads
                    .copy_owned()
                    .map_err(|_| InvocationError::Capacity)?,
                scopes: scope::ScopeCapture::copy(&q.scopes.scopes, q.scopes.budget).map_err(
                    |e| match e {
                        SourceScopeError::Capacity => InvocationError::Capacity,
                        _ => InvocationError::Limit,
                    },
                )?,
                limits: q.limits,
            };
            let preparation_timer = r.begin_service_timing();
            let prepared = (|| {
                self.check_owned_read_capture(&intent.reads, r)?;
                self.check_scoped_source_for_invocation(&intent.source, &intent.scopes, r)?;
                let mut plan = self.prepare_source_candidates(&intent.source.writes, &[])?;
                plan.scoped = true;
                plan.scopes = Some(
                    scope::ScopeCapture::copy(&intent.scopes.scopes, intent.scopes.budget)
                        .map_err(CoordinatorError::Scope)?,
                );
                plan.reads_required = true;
                plan.declared_reads = Some(intent.reads.copy_owned()?);
                Ok(plan)
            })();
            observe_service(
                r,
                Boundary::ExactBatchPreparation,
                &prepared,
                preparation_timer.finish(),
            );
            let prepared = prepared.map_err(coordinator_error)?;
            if !intent.matches_plan(&prepared) {
                return Err(InvocationError::Faulted);
            }
            if envelope_bytes(&intent, &prepared)? > self.invocations.limits.max_intent_bytes {
                return Err(InvocationError::Limit);
            }
            let slot = self.invocations.slots[q.invocation.slot]
                .as_mut()
                .expect("validated issued slot");
            let Phase::Issued(reserved) = std::mem::replace(&mut slot.phase, Phase::Running) else {
                unreachable!("unbound exact batch is issued")
            };
            self.install_invocation_admission(
                q.invocation,
                BoundIntent::ExactBatch(intent),
                Phase::Pending {
                    prepared: Ok(prepared),
                    reserved,
                },
            );
            Ok(InvocationState::Pending)
        })();
        if matches!(result, Err(InvocationError::Faulted)) {
            self.faulted = true;
        }
        capture_timed(r, Boundary::ExactBatchAdmission, &result, timer.finish());
        result
    }
}
