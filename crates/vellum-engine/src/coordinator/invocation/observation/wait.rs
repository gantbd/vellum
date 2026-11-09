//! Cooperative finite waits. No worker, callback, system clock, timer or producer
//! action. Logical clock truth and protected drain remain trusted host premises.
use super::*;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvocationWaitClockV1 {
    pub epoch: u64,
    pub tick: u64,
}
#[derive(Clone, Copy)]
pub struct InvocationWaitRequestV1<'a> {
    pub after: &'a InvocationObservationReferenceV1,
    pub budget: usize,
    pub clock: InvocationWaitClockV1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationWaitEventV1 {
    Poll,
    Tick(InvocationWaitClockV1),
    Expire,
    Stop,
}
#[derive(Clone)]
pub struct InvocationWaitIdV1 {
    owner: Arc<()>,
    marker: Arc<()>,
    slot: usize,
}
impl PartialEq for InvocationWaitIdV1 {
    fn eq(&self, other: &Self) -> bool {
        self.slot == other.slot
            && Arc::ptr_eq(&self.owner, &other.owner)
            && Arc::ptr_eq(&self.marker, &other.marker)
    }
}
impl Eq for InvocationWaitIdV1 {}
impl std::fmt::Debug for InvocationWaitIdV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationWaitIdV1(<opaque>)")
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InvocationWaitReplyV1 {
    Waiting(InvocationWaitIdV1),
    Observed(InvocationObservationReplyV1),
    Ended,
}
/// Host premises, not authentication or proof that an actual clock advanced.
/// Logical step counts select no elapsed-time, scheduling or fairness guarantee.
pub struct TrustedInvocationWaitFactsV1<'a> {
    pub observation: &'a TrustedInvocationOrdinaryObservationReleaseV1<'a>,
    pub wait_capacity: Option<bool>,
    pub wait_clock_valid: Option<bool>,
    pub retention_ready: Option<bool>,
    pub owner_control_allowed: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvocationWaitUsageV1 {
    pub retained: usize,
    pub waiting: usize,
    pub max_waiters: usize,
}
pub(super) struct Waiter {
    marker: Arc<()>,
    after: InvocationObservationReferenceV1,
    remaining: usize,
    pub(super) waiting: bool,
    clock: InvocationWaitClockV1,
}
impl InvocationObservationContextV1 {
    fn wait_slot(&self, id: &InvocationWaitIdV1) -> Result<usize, InvocationError> {
        if !Arc::ptr_eq(&id.owner, &self.ordinary_owner) {
            return Err(InvocationError::Unavailable);
        }
        let row = self
            .waiters
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(InvocationError::Unavailable)?;
        if !Arc::ptr_eq(&id.marker, &row.marker) {
            return Err(InvocationError::Unavailable);
        }
        if row.waiting && row.remaining == 0 {
            return Err(InvocationError::Faulted);
        }
        Ok(id.slot)
    }
}
impl SourceCoordinator {
    pub fn start_invocation_observation_wait_v1(
        &self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationWaitRequestV1<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationWaitFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationWaitIdV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if !self.current_invocation_observation(observer, current, facts.observation)?
                || [
                    facts.wait_capacity,
                    facts.wait_clock_valid,
                    facts.retention_ready,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(InvocationError::Unavailable);
            }
            if request.budget == 0 || request.budget > observer.limits.max_wait_steps {
                return Err(InvocationError::Limit);
            }
            if !request.after.belongs_to(observer) {
                return Err(InvocationError::Unavailable);
            }
            let slot = observer
                .waiters
                .iter()
                .position(Option::is_none)
                .ok_or(InvocationError::Limit)?;
            let marker = Arc::new(());
            let id = InvocationWaitIdV1 {
                owner: Arc::clone(&observer.ordinary_owner),
                marker: Arc::clone(&marker),
                slot,
            };
            observer.waiters[slot] = Some(Waiter {
                marker,
                after: request.after.clone(),
                remaining: request.budget,
                waiting: true,
                clock: request.clock,
            });
            Ok(id)
        })();
        if result == Err(InvocationError::Faulted) {
            observer.close();
        }
        capture_timed(
            r,
            Boundary::InvocationWaitAdmission,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn advance_invocation_observation_wait_v1(
        &self,
        observer: &mut InvocationObservationContextV1,
        wait: &InvocationWaitIdV1,
        event: InvocationWaitEventV1,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationWaitFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationWaitReplyV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.validate_invocation_observer_ownership(observer)?;
            let f = facts.observation.current;
            if observer.context != *f.context {
                return Err(InvocationError::Unavailable);
            }
            let slot = observer.wait_slot(wait)?;
            let row = observer.waiters[slot]
                .as_ref()
                .ok_or(InvocationError::Faulted)?;
            if !row.waiting {
                return Err(InvocationError::Unavailable);
            }
            match event {
                InvocationWaitEventV1::Stop | InvocationWaitEventV1::Expire => {
                    if facts.owner_control_allowed != Some(true) {
                        return Err(InvocationError::Unavailable);
                    }
                    observer.waiters[slot].as_mut().expect("owned wait").waiting = false;
                    Ok(InvocationWaitReplyV1::Ended)
                }
                InvocationWaitEventV1::Tick(clock) => {
                    // Clock service is independent of ordinary result capacity.
                    // Only a matching policy can explicitly revoke this context.
                    if current.recipient == observer.ceiling.recipient
                        && current.epoch == observer.ceiling.epoch
                        && (!current.grant
                            || [f.authorized, f.scope, f.continuous, f.live, f.retained]
                                .contains(&Some(false)))
                    {
                        observer.close();
                        return Err(InvocationError::Unavailable);
                    }
                    let invalid = facts.wait_clock_valid != Some(true)
                        || clock.epoch != row.clock.epoch
                        || clock.tick <= row.clock.tick;
                    let row = observer.waiters[slot].as_mut().expect("owned wait");
                    if invalid {
                        row.waiting = false;
                        return Err(InvocationError::Unavailable);
                    }
                    row.clock = clock;
                    if row.remaining <= 1 {
                        row.remaining = 0;
                        row.waiting = false;
                        Ok(InvocationWaitReplyV1::Ended)
                    } else {
                        row.remaining -= 1;
                        Ok(InvocationWaitReplyV1::Waiting(wait.clone()))
                    }
                }
                InvocationWaitEventV1::Poll => {
                    let after = row.after.clone();
                    match self.query_invocation_observation_inner(
                        observer,
                        InvocationObservationQueryV1::Status,
                        current,
                        facts.observation,
                    ) {
                        Err(e) => {
                            observer.waiters[slot].as_mut().expect("owned wait").waiting = false;
                            Err(e)
                        }
                        Ok(reply) => {
                            if reply.reference == after
                                && reply.phase != Some(InvocationObservationPhaseV1::Terminal)
                            {
                                return Ok(InvocationWaitReplyV1::Waiting(wait.clone()));
                            }
                            observer.waiters[slot].as_mut().expect("owned wait").waiting = false;
                            Ok(InvocationWaitReplyV1::Observed(reply))
                        }
                    }
                }
            }
        })();
        if result == Err(InvocationError::Faulted) {
            observer.close();
        }
        capture_timed(r, Boundary::InvocationWaitService, &result, timing.finish());
        result
    }
    /// Separate protected discharge frees one retained row, not parent funding.
    /// Live or ended waits both require actual owner/drain/release evidence.
    pub fn retire_invocation_observation_wait_v1(
        &self,
        observer: &mut InvocationObservationContextV1,
        wait: &InvocationWaitIdV1,
        control: &InvocationObservationPolicyControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.validate_invocation_observer_ownership(observer)?;
            if observer.context != *control.context
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
            let slot = observer.wait_slot(wait)?;
            observer.waiters[slot] = None;
            Ok(())
        })();
        if result == Err(InvocationError::Faulted) {
            observer.close();
        }
        capture_timed(
            r,
            Boundary::InvocationWaitRetirement,
            &result,
            timing.finish(),
        );
        result
    }
    /// Trusted bounded inventory scan, not a recipient observation product.
    /// Poll/tick use an opaque slot and do not scan this inventory.
    pub fn invocation_observation_wait_usage_v1(
        &self,
        observer: &InvocationObservationContextV1,
        r: &mut Recorder,
    ) -> Result<InvocationWaitUsageV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.validate_invocation_observer_ownership(observer)?;
            let mut retained = 0;
            let mut waiting = 0;
            for row in observer.waiters.iter().flatten() {
                retained += 1;
                waiting += usize::from(row.waiting);
            }
            Ok(InvocationWaitUsageV1 {
                retained,
                waiting,
                max_waiters: observer.limits.max_waiters,
            })
        })();
        capture_timed(r, Boundary::InvocationWaitUsage, &result, timing.finish());
        result
    }
}
