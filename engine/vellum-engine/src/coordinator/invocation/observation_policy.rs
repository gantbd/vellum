//! Original invocation policy metadata, not an F06 observer or mutation grant.
use super::super::settings::basis::{copy_context, labels};
use super::*;
use std::mem::size_of;
#[cfg(test)]
mod tests;

#[derive(Clone, PartialEq, Eq)]
pub struct InvocationObservationPolicyV1 {
    pub recipient: String,
    pub epoch: u64,
    pub grant: bool,
    pub publication: bool,
    pub control: bool,
    pub effect: bool,
    pub reasons: bool,
    pub details: bool,
    pub recovery: bool,
}
impl std::fmt::Debug for InvocationObservationPolicyV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationObservationPolicyV1(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvocationObservationPolicyLimitsV1 {
    pub max_context_bytes: usize,
    /// Declared future native projection ceiling, not sponsorship or support of
    /// an arbitrary coarse outcome payload. Observer admission must validate it.
    pub max_result_bytes: usize,
}
impl InvocationObservationPolicyLimitsV1 {
    /// Native metadata headers and copied label bytes, not JSON/RSS.
    pub fn required_capture_bytes(
        c: &SettingsExecutionContextV1,
        p: &InvocationObservationPolicyV1,
    ) -> Result<usize, InvocationError> {
        labels(c)
            .into_iter()
            .try_fold(
                size_of::<State>() + size_of::<Binding>() + size_of::<Capture>(),
                |n, v| n.checked_add(v.len()).ok_or(InvocationError::Limit),
            )?
            .checked_add(p.recipient.len())
            .ok_or(InvocationError::Limit)
    }
}
pub struct TrustedInvocationPolicyFactsV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub resources_ready: Option<bool>,
    pub retained: Option<bool>,
    pub transfer_ready: Option<bool>,
    pub preparation_reserved: Option<bool>,
    pub registry_ready: Option<bool>,
    pub retention_ready: Option<bool>,
}
pub struct InvocationObservationPolicyControlV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub owner_control_allowed: Option<bool>,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvocationObservationPolicyUsageV1 {
    pub bound: bool,
    pub spent: bool,
    pub declared_held_bytes: usize,
    /// Private capture users, not provider work or global quiescence evidence.
    pub attached_capture_users: usize,
}
pub(super) enum State {
    Unrequested,
    Held(Box<Binding>),
    Spent,
}
impl State {
    pub(super) fn shutdown_reserved_bytes(
        &self,
        id: &InvocationId,
        ledger: &retention::RetentionState,
    ) -> Result<Option<usize>, CoordinatorError> {
        match self {
            Self::Held(binding) => {
                binding
                    .validate(id, ledger)
                    .map_err(|_| CoordinatorError::Faulted)?;
                Ok(Some(binding.capture.limits.max_context_bytes))
            }
            Self::Unrequested | Self::Spent => Ok(None),
        }
    }
    pub(super) fn held(&self) -> bool {
        matches!(self, Self::Held(_))
    }
}
pub(super) struct Capture {
    pub(super) invocation: InvocationId,
    pub(super) context: SettingsExecutionContextV1,
    pub(super) policy: InvocationObservationPolicyV1,
    pub(super) limits: InvocationObservationPolicyLimitsV1,
}
pub(super) struct Binding {
    pub(super) capture: Arc<Capture>,
    reservation: retention::Reservation,
}
macro_rules! redacted {($($t:ty),+) => {$(impl std::fmt::Debug for $t {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {f.write_str("InvocationPolicy(<owned:redacted>)")}
})+};}
redacted!(State, Capture, Binding);
fn parent(e: RetentionErrorV1) -> InvocationError {
    match e {
        RetentionErrorV1::Faulted => InvocationError::Faulted,
        RetentionErrorV1::Limit => InvocationError::Limit,
        RetentionErrorV1::Unavailable => InvocationError::Unavailable,
    }
}
fn copy_text(s: &str) -> Result<String, InvocationError> {
    let mut out = String::new();
    out.try_reserve_exact(s.len())
        .map_err(|_| InvocationError::Capacity)?;
    out.push_str(s);
    Ok(out)
}
impl Binding {
    pub(super) fn validate(
        &self,
        id: &InvocationId,
        ledger: &retention::RetentionState,
    ) -> Result<(), InvocationError> {
        if self.capture.invocation != *id {
            return Err(InvocationError::Faulted);
        }
        ledger
            .validate_exact(&self.reservation, self.capture.limits.max_context_bytes)
            .map_err(parent)
    }
}
impl SourceCoordinator {
    /// Trusted-host preparation on the original issued ticket. No policy may be
    /// attached after command admission or replaced after protected discharge.
    pub fn bind_invocation_observation_policy_v1(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        p: &InvocationObservationPolicyV1,
        limits: InvocationObservationPolicyLimitsV1,
        f: &TrustedInvocationPolicyFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<bool, InvocationError> {
        let timing = r.begin_service_timing();
        let mut provisional = None;
        let mut result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(id)?;
            // Preflight lengths before comparing caller-owned strings. Both
            // contexts must fit the declared native capture envelope.
            if limits.max_context_bytes == 0
                || limits.max_context_bytes > self.invocations.limits.max_intent_bytes
                || limits.max_result_bytes == 0
                || limits.max_result_bytes > self.invocations.limits.max_intent_bytes
                || InvocationObservationPolicyLimitsV1::required_capture_bytes(c, p)?
                    > limits.max_context_bytes
                || InvocationObservationPolicyLimitsV1::required_capture_bytes(f.context, p)?
                    > limits.max_context_bytes
            {
                return Err(InvocationError::Limit);
            }
            if f.context != c
                || [
                    f.authorized,
                    f.scope,
                    f.continuous,
                    f.live,
                    f.resources_ready,
                    f.retained,
                    f.transfer_ready,
                    f.preparation_reserved,
                    f.registry_ready,
                    f.retention_ready,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(InvocationError::Unavailable);
            }
            // The accepted original local-command projection is metadata-only.
            // Reject unsupported details instead of silently downgrading them.
            if labels(c).iter().any(|v| v.is_empty())
                || !p.grant
                || p.details
                || p.recipient != c.recipient
            {
                return Err(InvocationError::Unavailable);
            }
            match &slot.observation_policy {
                State::Held(binding) => {
                    binding.validate(id, &self.retention)?;
                    let original = &binding.capture;
                    return if original.context == *c
                        && original.policy == *p
                        && original.limits == limits
                    {
                        Ok(false)
                    } else {
                        Err(InvocationError::IntentMismatch)
                    };
                }
                State::Spent => return Err(InvocationError::Unavailable),
                State::Unrequested => {}
            }
            if !matches!(slot.phase, Phase::Issued(_)) || slot.intent.is_some() {
                return Err(InvocationError::Unavailable);
            }
            provisional = Some(
                self.retention
                    .reserve(limits.max_context_bytes)
                    .map_err(parent)?,
            );
            let context = copy_context(c).map_err(|e| {
                if e == crate::settings::SettingsError::Capacity {
                    InvocationError::Capacity
                } else {
                    InvocationError::Limit
                }
            })?;
            let policy = InvocationObservationPolicyV1 {
                recipient: copy_text(&p.recipient)?,
                epoch: p.epoch,
                grant: p.grant,
                publication: p.publication,
                control: p.control,
                effect: p.effect,
                reasons: p.reasons,
                details: p.details,
                recovery: p.recovery,
            };
            let capture = Arc::new(Capture {
                invocation: id.clone(),
                context,
                policy,
                limits,
            });
            let binding = Box::new(Binding {
                capture,
                reservation: provisional.take().expect("funded original policy"),
            });
            self.invocations.slots[id.slot]
                .as_mut()
                .expect("original issued slot")
                .observation_policy = State::Held(binding);
            Ok(true)
        })();
        if let Some(mut hold) = provisional
            && self.retention.release(&mut hold).is_err()
        {
            result = Err(InvocationError::Faulted);
        }
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(
            r,
            Boundary::InvocationPolicyBinding,
            &result,
            timing.finish(),
        );
        result
    }
    /// Trusted metadata gauges only; not a recipient phase or outcome product.
    pub fn invocation_observation_policy_usage_v1(
        &self,
        id: &InvocationId,
        r: &mut Recorder,
    ) -> Result<InvocationObservationPolicyUsageV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let slot = self.invocation_slot(id)?;
            let (bound, spent, bytes, users) = match &slot.observation_policy {
                State::Unrequested => (false, false, 0, 0),
                State::Spent => (false, true, 0, 0),
                State::Held(b) => {
                    b.validate(id, &self.retention)?;
                    (
                        true,
                        false,
                        b.capture.limits.max_context_bytes,
                        Arc::strong_count(&b.capture) - 1,
                    )
                }
            };
            Ok(InvocationObservationPolicyUsageV1 {
                bound,
                spent,
                declared_held_bytes: bytes,
                attached_capture_users: users,
            })
        })();
        capture_timed(r, Boundary::InvocationPolicyUsage, &result, timing.finish());
        result
    }
    /// Releases only this policy capture after owner-controlled drain. It never
    /// stops/runs/forgets the producer, refunds issuance or asserts quiescence.
    pub fn retire_invocation_observation_policy_v1(
        &mut self,
        id: &InvocationId,
        f: &InvocationObservationPolicyControlV1<'_>,
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
            let State::Held(b) = &slot.observation_policy else {
                return Err(InvocationError::Unavailable);
            };
            if b.capture.context != *f.context {
                return Err(InvocationError::Unavailable);
            }
            b.validate(id, &self.retention)?;
            if Arc::strong_count(&b.capture) != 1 {
                return Err(InvocationError::Unavailable);
            }
            match slot.phase {
                Phase::Issued(_) | Phase::Terminal(_) => {}
                Phase::Running | Phase::AnnotationFaulted if self.faulted => {}
                Phase::Running | Phase::AnnotationFaulted => return Err(InvocationError::Faulted),
                _ => return Err(InvocationError::Pending),
            }
            let State::Held(b) = &mut self.invocations.slots[id.slot]
                .as_mut()
                .expect("original policy slot")
                .observation_policy
            else {
                unreachable!()
            };
            self.retention.release(&mut b.reservation).map_err(parent)?;
            self.invocations.slots[id.slot]
                .as_mut()
                .expect("released policy slot")
                .observation_policy = State::Spent;
            Ok(())
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(
            r,
            Boundary::InvocationPolicyRetirement,
            &result,
            timing.finish(),
        );
        result
    }
}
