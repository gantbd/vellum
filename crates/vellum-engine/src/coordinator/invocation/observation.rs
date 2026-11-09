//! Native F06 ownership and filtered local-command projections. No producer
//! action, public acquisition verb or wire codec.
use super::*;
use std::mem::size_of;
pub(super) mod projection;
pub use projection::{
    InvocationControlDispositionV1, InvocationLocalEffectV1, InvocationOutcomeProjectionV1,
    InvocationOutcomeReasonV1, InvocationPublicationV1, InvocationRecoveryV1,
};
mod ingress;
mod ordinary;
pub use ingress::{
    InvocationObservationEnvelopeValueV1, InvocationObservationReadPortV1,
    InvocationObservationReadReplyV1,
};
mod receipt_ingress;
mod receipts;
pub use receipt_ingress::InvocationReceiptEnvelopeValueV1;
mod receipts_v10;
mod receipts_v2;
mod receipts_v3;
mod receipts_v4;
mod receipts_v5;
mod receipts_v6;
mod receipts_v7;
mod receipts_v8;
mod receipts_v9;
pub(super) mod registry;
pub use receipts::{
    InvocationReceiptAdmissionV1, InvocationReceiptCapabilitiesV1, InvocationReceiptKindV1,
    InvocationReceiptRequestV1, InvocationReceiptV1, TrustedInvocationReceiptReleaseV1,
};
pub use receipts_v2::{InvocationReceiptRequestV2, InvocationReceiptV2};
pub use receipts_v3::{InvocationReceiptRequestV3, InvocationReceiptV3};
pub use receipts_v4::{InvocationReceiptRequestV4, InvocationReceiptV4};
pub use receipts_v5::{InvocationReceiptRequestV5, InvocationReceiptV5};
pub use receipts_v6::{InvocationReceiptRequestV6, InvocationReceiptV6};
pub use receipts_v7::{InvocationReceiptRequestV7, InvocationReceiptV7};
pub use receipts_v8::{InvocationReceiptRequestV8, InvocationReceiptV8};
pub use receipts_v9::{InvocationReceiptRequestV9, InvocationReceiptV9};
pub use receipts_v10::{InvocationReceiptRequestV10, InvocationReceiptV10};
mod wait;
pub use ordinary::{
    InvocationObservationQueryV1, InvocationObservationReferenceV1, InvocationObservationReplyV1,
    TrustedInvocationOrdinaryObservationReleaseV1,
};
pub use wait::{
    InvocationWaitClockV1, InvocationWaitEventV1, InvocationWaitIdV1, InvocationWaitReplyV1,
    InvocationWaitRequestV1, InvocationWaitUsageV1, TrustedInvocationWaitFactsV1,
};
#[cfg(test)]
pub(in crate::coordinator) mod ingress_tests;
#[cfg(test)]
pub(in crate::coordinator) mod tests;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReceiptProfile {
    Unrequested,
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    V8,
    V9,
    V10,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvocationObservationLimitsV1 {
    pub max_waiters: usize,
    pub max_issued: usize,
    pub max_wait_steps: usize,
    pub max_context_bytes: usize,
    pub max_wait_bytes: usize,
    pub max_reference_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_control_reply_bytes: usize,
    pub max_held_bytes: usize,
}
impl InvocationObservationLimitsV1 {
    /// Full declared local reply/context/wait/reference envelope.
    /// Logical native accounting, not allocator/RSS or a supported wire encoding.
    pub fn required_hold_bytes(self) -> Result<usize, InvocationError> {
        let add = |a: usize, b: usize| a.checked_add(b).ok_or(InvocationError::Limit);
        let mul = |a: usize, b: usize| a.checked_mul(b).ok_or(InvocationError::Limit);
        let n = add(mul(2, self.max_reply_bytes)?, self.max_control_reply_bytes)?;
        let n = add(n, self.max_context_bytes)?;
        let n = add(n, mul(add(self.max_waiters, 1)?, self.max_wait_bytes)?)?;
        let n = add(n, mul(add(self.max_issued, 2)?, self.max_reference_bytes)?)?;
        add(n, size_of::<Option<registry::Record>>())
    }
    pub fn required_context_bytes(
        c: &SettingsExecutionContextV1,
        p: &InvocationObservationPolicyV1,
    ) -> Result<usize, InvocationError> {
        super::super::settings::basis::labels(c)
            .into_iter()
            .try_fold(size_of::<InvocationObservationContextV1>(), |n, s| {
                n.checked_add(s.len()).ok_or(InvocationError::Limit)
            })?
            .checked_add(p.recipient.len())
            .ok_or(InvocationError::Limit)
    }
    pub fn required_wait_bytes() -> usize {
        size_of::<Option<wait::Waiter>>()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationObservationPhaseV1 {
    Pending,
    Terminal,
}

/// A separately typed protected reference cannot be used as an ordinary wait
/// selector. Its two identities are allocated at admission, never during reads.
#[derive(Clone)]
pub struct InvocationControlReferenceV1 {
    marker: Arc<()>,
}
impl PartialEq for InvocationControlReferenceV1 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.marker, &other.marker)
    }
}
impl Eq for InvocationControlReferenceV1 {}
impl std::fmt::Debug for InvocationControlReferenceV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationControlReferenceV1(<opaque>)")
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvocationControlObservationV1 {
    pub phase: InvocationObservationPhaseV1,
    pub reference: InvocationControlReferenceV1,
}
pub struct TrustedInvocationObservationReleaseV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub retained: Option<bool>,
    pub transfer_ready: Option<bool>,
    pub control_ready: Option<bool>,
}
/// Host-owned native context; dropping it never manufactures retention/drain
/// evidence or refunds its parent. Explicit protected retirement is required.
/// Receipt facets opt in at construction and retain their native release gates.
pub struct InvocationObservationContextV1 {
    registry_slot: usize,
    handle_life: Arc<()>,
    invocation: InvocationId,
    original: Option<Arc<observation_policy::Capture>>,
    context: SettingsExecutionContextV1,
    ceiling: InvocationObservationPolicyV1,
    limits: InvocationObservationLimitsV1,
    hold: retention::Reservation,
    live: bool,
    pending: InvocationControlReferenceV1,
    terminal: InvocationControlReferenceV1,
    ordinary_owner: Arc<()>,
    ordinary_issued: usize,
    ordinary_last: Option<ordinary::Signature>,
    waiters: Vec<Option<wait::Waiter>>,
    receipt_ceiling: Option<InvocationReceiptCapabilitiesV1>,
    receipt_faulted: bool,
    receipt_profile: ReceiptProfile,
}
/// Identity of one actual host-owned observation context, not a progress or control reference.
#[derive(Clone)]
pub struct InvocationObservationContextIdV1 {
    owner: Arc<()>,
}
impl PartialEq for InvocationObservationContextIdV1 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
    }
}
impl Eq for InvocationObservationContextIdV1 {}
impl std::fmt::Debug for InvocationObservationContextIdV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationObservationContextIdV1(<redacted>)")
    }
}
impl InvocationObservationContextIdV1 {
    pub(in crate::coordinator) fn matches_context(
        &self,
        context: &InvocationObservationContextV1,
    ) -> bool {
        Arc::ptr_eq(&self.owner, &context.ordinary_owner)
    }
}
impl InvocationObservationContextV1 {
    /// Borrows the existing unique identity; allocates no registry entry or lifetime unit.
    pub fn context_id_v1(&self) -> InvocationObservationContextIdV1 {
        InvocationObservationContextIdV1 {
            owner: Arc::clone(&self.ordinary_owner),
        }
    }
    fn close(&mut self) {
        self.live = false;
        for waiter in self.waiters.iter_mut().flatten() {
            waiter.waiting = false;
        }
    }
}
impl std::fmt::Debug for InvocationObservationContextV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationObservationContextV1(<opaque>)")
    }
}
fn parent(e: RetentionErrorV1) -> InvocationError {
    match e {
        RetentionErrorV1::Faulted => InvocationError::Faulted,
        RetentionErrorV1::Limit => InvocationError::Limit,
        RetentionErrorV1::Unavailable => InvocationError::Unavailable,
    }
}
fn admitted_phase(slot: &Slot) -> Result<InvocationObservationPhaseV1, InvocationError> {
    match slot.phase.state()? {
        InvocationState::Pending => Ok(InvocationObservationPhaseV1::Pending),
        InvocationState::Finished => Ok(InvocationObservationPhaseV1::Terminal),
        InvocationState::Issued => Err(InvocationError::Unavailable),
    }
}
impl SourceCoordinator {
    fn validate_invocation_observer_ownership(
        &self,
        observer: &InvocationObservationContextV1,
    ) -> Result<(), InvocationError> {
        let original = observer
            .original
            .as_ref()
            .ok_or(InvocationError::Unavailable)?;
        let slot = self.invocation_slot(&observer.invocation)?;
        let observation_policy::State::Held(binding) = &slot.observation_policy else {
            return Err(InvocationError::Faulted);
        };
        binding.validate(&observer.invocation, &self.retention)?;
        if observer.waiters.len() != observer.limits.max_waiters
            || observer.ordinary_issued > observer.limits.max_issued
        {
            return Err(InvocationError::Faulted);
        }
        if original.invocation != observer.invocation
            || original.context != observer.context
            || !Arc::ptr_eq(original, &binding.capture)
        {
            return Err(InvocationError::Faulted);
        }
        self.retention
            .validate_exact(&observer.hold, observer.limits.required_hold_bytes()?)
            .map_err(parent)?;
        self.invocations.observers.validate(observer)
    }
    fn original_observation_capture(
        &self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
    ) -> Result<&Arc<observation_policy::Capture>, InvocationError> {
        let slot = self.invocation_slot(id)?;
        admitted_phase(slot)?;
        let observation_policy::State::Held(b) = &slot.observation_policy else {
            return Err(InvocationError::Unavailable);
        };
        b.validate(id, &self.retention)?;
        if b.capture.context != *c {
            return Err(InvocationError::Unavailable);
        }
        if let Some((actual, owner)) = slot.control_context()
            && (actual != c || !Arc::ptr_eq(owner, &self.namespace))
        {
            return Err(InvocationError::Unavailable);
        }
        Ok(&b.capture)
    }
    /// Trusted-host native context construction, not a new caller acquisition
    /// verb. The complete declared envelope is charged before copying metadata.
    pub fn open_invocation_observation_v1(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        ceiling: &InvocationObservationPolicyV1,
        limits: InvocationObservationLimitsV1,
        facts: &TrustedInvocationPolicyFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationObservationContextV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_admission_unfenced().map_err(|e| match e {
                CoordinatorError::Faulted => InvocationError::Faulted,
                _ => InvocationError::Unavailable,
            })?;
            let charge = limits.required_hold_bytes()?;
            if charge > limits.max_held_bytes
                || InvocationObservationLimitsV1::required_context_bytes(c, ceiling)?
                    > limits.max_context_bytes
                || InvocationObservationLimitsV1::required_context_bytes(facts.context, ceiling)?
                    > limits.max_context_bytes
                || limits.max_reference_bytes < size_of::<InvocationControlReferenceV1>()
                || limits.max_reference_bytes < size_of::<InvocationObservationReferenceV1>()
                || limits.max_control_reply_bytes < size_of::<InvocationControlObservationV1>()
                || limits.max_reply_bytes < size_of::<InvocationObservationReplyV1>()
                || limits.max_reply_bytes < size_of::<InvocationWaitReplyV1>()
                || limits.max_wait_bytes < InvocationObservationLimitsV1::required_wait_bytes()
            {
                return Err(InvocationError::Limit);
            }
            if facts.context != c
                || [
                    facts.authorized,
                    facts.scope,
                    facts.continuous,
                    facts.live,
                    facts.resources_ready,
                    facts.retained,
                    facts.transfer_ready,
                    facts.preparation_reserved,
                    facts.registry_ready,
                    facts.retention_ready,
                ]
                .into_iter()
                .any(|v| v != Some(true))
                || !ceiling.grant
                || ceiling.recipient != c.recipient
            {
                return Err(InvocationError::Unavailable);
            }
            let original = self.original_observation_capture(id, c)?;
            if original.limits.max_result_bytes < size_of::<InvocationOutcomeProjectionV1>() {
                return Err(InvocationError::Limit);
            }
            if ceiling.epoch != original.policy.epoch
                || ceiling.recipient != original.policy.recipient
            {
                return Err(InvocationError::Unavailable);
            }
            // The native metadata-only projection has a fixed complete bound;
            // operation-specific receipts and wire envelopes remain separate.
            let reply_floor = original
                .limits
                .max_result_bytes
                .checked_add(size_of::<InvocationControlObservationV1>())
                .ok_or(InvocationError::Limit)?;
            if limits.max_reply_bytes < reply_floor {
                return Err(InvocationError::Limit);
            }
            let original = Arc::clone(original);
            self.retention.check(charge).map_err(parent)?;
            let (registry_slot, next_issued) = self.invocations.observers.prepare()?;
            let mut hold = self.retention.reserve(charge).map_err(parent)?;
            let copied = (|| {
                let mut waiters = Vec::new();
                waiters
                    .try_reserve_exact(limits.max_waiters)
                    .map_err(|_| InvocationError::Capacity)?;
                waiters.resize_with(limits.max_waiters, || None);
                let context = super::super::settings::basis::copy_context(c).map_err(|e| {
                    if e == crate::settings::SettingsError::Capacity {
                        InvocationError::Capacity
                    } else {
                        InvocationError::Limit
                    }
                })?;
                let mut recipient = String::new();
                recipient
                    .try_reserve_exact(ceiling.recipient.len())
                    .map_err(|_| InvocationError::Capacity)?;
                recipient.push_str(&ceiling.recipient);
                let ceiling = InvocationObservationPolicyV1 {
                    recipient,
                    epoch: ceiling.epoch,
                    grant: ceiling.grant,
                    publication: ceiling.publication,
                    control: ceiling.control,
                    effect: ceiling.effect,
                    reasons: ceiling.reasons,
                    details: ceiling.details,
                    recovery: ceiling.recovery,
                };
                Ok::<_, InvocationError>((context, ceiling, waiters))
            })();
            let (context, ceiling, waiters) = match copied {
                Ok(v) => v,
                Err(e) => {
                    self.retention.release(&mut hold).map_err(parent)?;
                    return Err(e);
                }
            };
            let observer = InvocationObservationContextV1 {
                registry_slot,
                handle_life: Arc::new(()),
                invocation: id.clone(),
                original: Some(original),
                context,
                ceiling,
                limits,
                hold,
                live: true,
                pending: InvocationControlReferenceV1 {
                    marker: Arc::new(()),
                },
                terminal: InvocationControlReferenceV1 {
                    marker: Arc::new(()),
                },
                ordinary_owner: Arc::new(()),
                ordinary_issued: 0,
                ordinary_last: None,
                waiters,
                receipt_ceiling: None,
                receipt_faulted: false,
                receipt_profile: ReceiptProfile::Unrequested,
            };
            self.invocations.observers.register(&observer, next_issued);
            Ok(observer)
        })();
        capture_timed(
            r,
            Boundary::InvocationObservationAdmission,
            &result,
            timing.finish(),
        );
        result
    }
    /// Protected metadata-only phase projection. Never borrows or clones the
    /// retained outcome, runs work, or consumes ordinary issuance capacity.
    pub fn minimal_invocation_control_status_v1(
        &self,
        observer: &mut InvocationObservationContextV1,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationObservationReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationControlObservationV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.validate_invocation_observer_ownership(observer)?;
            let Some(original) = observer.original.as_ref() else {
                return Err(InvocationError::Unavailable);
            };
            // A foreign coordinator cannot invalidate another owner's context.
            self.invocation_slot(&original.invocation)?;
            if !observer.live
                || observer.context != *facts.context
                || current.recipient != observer.ceiling.recipient
                || current.epoch != observer.ceiling.epoch
            {
                return Err(InvocationError::Unavailable);
            }
            if !current.grant
                || [
                    facts.authorized,
                    facts.scope,
                    facts.continuous,
                    facts.live,
                    facts.retained,
                ]
                .contains(&Some(false))
            {
                observer.close();
                return Err(InvocationError::Unavailable);
            }
            if [
                facts.authorized,
                facts.scope,
                facts.continuous,
                facts.live,
                facts.retained,
                facts.transfer_ready,
                facts.control_ready,
            ]
            .into_iter()
            .any(|v| v != Some(true))
                || !current.control
                || !observer.ceiling.control
                || !original.policy.control
            {
                return Err(InvocationError::Unavailable);
            }
            self.retention
                .validate_exact(&observer.hold, observer.limits.required_hold_bytes()?)
                .map_err(parent)?;
            let actual =
                self.original_observation_capture(&original.invocation, &observer.context)?;
            if !Arc::ptr_eq(original, actual) {
                observer.close();
                return Err(InvocationError::Faulted);
            }
            let phase = admitted_phase(self.invocation_slot(&original.invocation)?)?;
            let reference = match phase {
                InvocationObservationPhaseV1::Pending => observer.pending.clone(),
                InvocationObservationPhaseV1::Terminal => observer.terminal.clone(),
            };
            Ok(InvocationControlObservationV1 { phase, reference })
        })();
        if result == Err(InvocationError::Faulted) {
            observer.close();
        }
        capture_timed(
            r,
            Boundary::InvocationControlObservation,
            &result,
            timing.finish(),
        );
        result
    }
    /// Close only this observer. The pending producer remains pending.
    pub fn close_invocation_observation_v1(
        &self,
        observer: &mut InvocationObservationContextV1,
        control: &InvocationObservationPolicyControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.validate_invocation_observer_ownership(observer)?;
            let original = observer
                .original
                .as_ref()
                .ok_or(InvocationError::Unavailable)?;
            self.invocation_slot(&original.invocation)?;
            if control.context != &observer.context || control.owner_control_allowed != Some(true) {
                return Err(InvocationError::Unavailable);
            }
            observer.close();
            Ok(())
        })();
        capture_timed(
            r,
            Boundary::InvocationObservationClose,
            &result,
            timing.finish(),
        );
        result
    }
    /// Discharge only this context's actual hold; producer policy and issuance
    /// remain separately owned. No drop-triggered refund or quiescence inference.
    pub fn retire_invocation_observation_v1(
        &mut self,
        observer: &mut InvocationObservationContextV1,
        control: &InvocationObservationPolicyControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.validate_invocation_observer_ownership(observer)?;
            let original = observer
                .original
                .as_ref()
                .ok_or(InvocationError::Unavailable)?;
            self.invocation_slot(&original.invocation)?;
            if control.context != &observer.context
                || [
                    control.owner_control_allowed,
                    control.retention_released,
                    control.delivery_drained,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(InvocationError::Unavailable);
            }
            self.retention
                .validate_exact(&observer.hold, observer.limits.required_hold_bytes()?)
                .map_err(parent)?;
            if observer.waiters.iter().any(Option::is_some) {
                return Err(InvocationError::Unavailable);
            }
            self.invocations.observers.validate_retirement(observer)?;
            self.retention.release(&mut observer.hold).map_err(parent)?;
            self.invocations.observers.release(observer);
            observer.close();
            observer.waiters = Vec::new();
            observer.original = None;
            observer.ordinary_last = None;
            for label in [
                &mut observer.context.authority,
                &mut observer.context.engine,
                &mut observer.context.recipient,
                &mut observer.context.purpose,
                &mut observer.context.destination,
                &mut observer.context.profile,
                &mut observer.context.continuity,
                &mut observer.ceiling.recipient,
            ] {
                *label = String::new();
            }
            Ok(())
        })();
        capture_timed(
            r,
            Boundary::InvocationObservationRetirement,
            &result,
            timing.finish(),
        );
        result
    }
}
