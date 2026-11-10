//! One accepted replacement in the original invocation ledger, not an IME owner.
use super::*;
use crate::capture::CaptureLimitsV1;
use crate::coordinator::settings::basis;
use crate::settings::SettingsError;
use std::mem::size_of;
mod arguments;
#[cfg(test)]
mod tests;
pub use arguments::*;

#[derive(PartialEq, Eq)]
pub(super) struct CompositionIntent {
    pub(super) view: LogicalViewContext,
    pub(super) replacement: Replacement,
    pub(super) actor: SettingsExecutionContextV1,
    pub(super) owner: Arc<()>,
    limits: CaptureLimitsV1,
}
impl std::fmt::Debug for CompositionIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CompositionIntent(<redacted>)")
    }
}

/// Native logical retained envelope, not physical allocation, ingress, stack or
/// RSS. The issued slot and reserved outcome vector are prepaid by the ledger.
/// Preparation is restricted to ordinary, single-document composition plans;
/// additional dynamic fields must not silently escape this accounting.
fn envelope_bytes(
    intent: &CompositionIntent,
    plan: &PreparedTransaction,
) -> Result<usize, InvocationError> {
    let mut bytes = size_of::<CompositionIntent>()
        .checked_add(size_of::<PreparedTransaction>())
        .ok_or(InvocationError::Limit)?;
    let mut add = |n: usize| -> Result<(), InvocationError> {
        bytes = bytes.checked_add(n).ok_or(InvocationError::Limit)?;
        Ok(())
    };
    add(intent.replacement.expected.len())?;
    add(intent.replacement.inserted.len())?;
    for label in arguments::context_labels(&intent.actor) {
        add(label.len())?;
    }
    for candidate in &plan.candidates {
        add(size_of::<Candidate>())?;
        add(candidate.source.text().len())?;
        for edit in &candidate.edits {
            add(size_of::<Replacement>())?;
            add(edit.expected.len())?;
            add(edit.inserted.len())?;
        }
    }
    add(size_of::<views::DerivedEdit>())?;
    add(intent.replacement.expected.len())?;
    add(intent.replacement.inserted.len())?;
    for before in &plan.view_dependencies {
        add(size_of::<views::ViewDependency>())?;
        add(before
            .selection_count()
            .checked_mul(size_of::<crate::selection::Selection>())
            .ok_or(InvocationError::Limit)?)?;
    }
    Ok(bytes)
}
impl SourceCoordinator {
    /// Trusted embedding admission. Original actor labels are correlation and
    /// protected-control bindings, not authenticated mutation permission. The
    /// embedding authorizes admission and current disclosure independently.
    /// Failed preparation or envelope admission leaves the ticket issued/unbound.
    pub fn submit_composition_replacement_v1(
        &mut self,
        id: &InvocationId,
        view: &LogicalViewContext,
        replacement: &Replacement,
        actor: &SettingsExecutionContextV1,
        limits: CaptureLimitsV1,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.guard_shutdown_invocation_admission(id, r)?;
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(id)?;
            if let Some(original) = &slot.intent {
                if !matches!(original, BoundIntent::Composition(c) if c.view == *view && c.replacement == *replacement && c.actor == *actor && c.limits == limits)
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            arguments::validate_capture(
                view,
                replacement.start,
                replacement.end,
                &replacement.expected,
                &replacement.inserted,
                actor,
                CaptureLimitsV1 {
                    max_bytes: limits
                        .max_bytes
                        .min(self.invocations.limits.max_intent_bytes),
                    ..limits
                },
                limits,
                r,
            )?;
            let intent = CompositionIntent {
                view: view.clone(),
                replacement: views::composition::copy_replacement(replacement)
                    .map_err(preparation_error)?,
                actor: basis::copy_context(actor).map_err(|e| match e {
                    SettingsError::Capacity => InvocationError::Capacity,
                    _ => InvocationError::MalformedIntent,
                })?,
                owner: self.namespace.clone(),
                limits,
            };
            // Drop both capture and native preparation on refusal. Never retain
            // an error as an admitted composition job or claim a spent ticket.
            let prepared = self
                .prepare_composition_replacement_v1(view, replacement, r)
                .map_err(preparation_error)?;
            if envelope_bytes(&intent, &prepared)? > self.invocations.limits.max_intent_bytes {
                return Err(InvocationError::Limit);
            }
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .expect("validated issued slot");
            let Phase::Issued(reserved) = std::mem::replace(&mut slot.phase, Phase::Running) else {
                unreachable!("unbound composition slot is issued")
            };
            self.install_invocation_admission(
                id,
                BoundIntent::Composition(intent),
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
        capture_timed(r, Boundary::CompositionAdmission, &result, timer.finish());
        result
    }
    pub fn submit_composition_arguments_v1(
        &mut self,
        q: &NormalizedCompositionArgumentsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.submit_composition_replacement_v1(
            q.invocation,
            q.view,
            &q.replacement,
            &q.actor,
            q.limits,
            r,
        )
    }
}
fn preparation_error(error: crate::selection::SelectionError) -> InvocationError {
    use crate::selection::SelectionError as E;
    match error {
        E::Faulted => InvocationError::Faulted,
        E::Capacity => InvocationError::Capacity,
        E::Limit => InvocationError::Limit,
        _ => InvocationError::Unavailable,
    }
}
