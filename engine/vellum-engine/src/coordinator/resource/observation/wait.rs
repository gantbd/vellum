use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeOperationWaitClockV1 {
    pub epoch: u64,
    pub tick: u64,
}
pub struct NativeOperationWaitRequestV1<'a> {
    pub after: &'a NativeOperationProgressIdV1,
    pub budget: usize,
    pub clock: NativeOperationWaitClockV1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeOperationWaitEventV1 {
    Poll,
    Tick(NativeOperationWaitClockV1),
    Expire,
    Stop,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeOperationWaitReplyV1 {
    Waiting(NativeOperationWaitIdV1),
    Observed(NativeOperationObservationReplyV1),
    Ended,
}
pub(super) struct Waiter {
    pub(super) marker: Arc<()>,
    pub(super) after: NativeOperationProgressIdV1,
    pub(super) remaining: usize,
    pub(super) waiting: bool,
    pub(super) clock: NativeOperationWaitClockV1,
}
impl SourceCoordinator {
    pub fn start_native_operation_observation_wait_v1(
        &mut self,
        id: &NativeOperationObservationIdV1,
        request: NativeOperationWaitRequestV1<'_>,
        p: &NativeOperationObservationPolicyV1,
        f: &NativeOperationObservationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<NativeOperationWaitIdV1, Error> {
        let result = self.observation_service(id, r, |o, _| {
            let (allowed, _, _) = o.observation_current(id, p, f, false)?;
            let row = o.observation_row(id)?;
            if !allowed
                || [f.wait_capacity, f.wait_clock_valid, f.retention_ready]
                    .into_iter()
                    .any(|v| v != Some(true))
            {
                return Err(Error::Unavailable);
            }
            if request.budget == 0 || request.budget > row.limits.max_wait_steps {
                return Err(Error::InvalidRequest);
            }
            if !row
                .refs
                .iter()
                .any(|v| !v.control && v.id == *request.after)
            {
                return Err(Error::Unavailable);
            }
            let slot = row
                .waiters
                .iter()
                .position(Option::is_none)
                .ok_or(Error::Limit)?;
            let marker = Arc::new(());
            let wait = NativeOperationWaitIdV1 {
                owner: o.namespace.clone(),
                context: row.marker.clone(),
                marker: marker.clone(),
            };
            o.observation_row_mut(id)?.waiters[slot] = Some(Waiter {
                marker,
                after: request.after.clone(),
                remaining: request.budget,
                waiting: true,
                clock: request.clock,
            });
            Ok(wait)
        });
        observe(r, Boundary::NativeOperationObservationWait, &result);
        result
    }
    fn waiter_slot(
        &self,
        id: &NativeOperationObservationIdV1,
        wait: &NativeOperationWaitIdV1,
    ) -> Result<usize, Error> {
        let row = self.observation_row(id)?;
        if !Arc::ptr_eq(&self.namespace, &wait.owner) || !Arc::ptr_eq(&row.marker, &wait.context) {
            return Err(Error::Unavailable);
        }
        row.waiters
            .iter()
            .position(|w| {
                w.as_ref()
                    .is_some_and(|w| Arc::ptr_eq(&w.marker, &wait.marker))
            })
            .ok_or(Error::Unavailable)
    }
    pub fn advance_native_operation_observation_wait_v1(
        &mut self,
        id: &NativeOperationObservationIdV1,
        wait: &NativeOperationWaitIdV1,
        event: NativeOperationWaitEventV1,
        p: &NativeOperationObservationPolicyV1,
        f: &NativeOperationObservationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<NativeOperationWaitReplyV1, Error> {
        let result = self.observation_service(id, r, |o, r| {
            let slot = o.waiter_slot(id, wait)?;
            let row = o.observation_row(id)?;
            if row.context != *f.context {
                return Err(Error::Unavailable);
            }
            if input_bytes(f.context, p)? > row.limits.max_context_bytes {
                return Err(Error::InvalidRequest);
            }
            let w = row.waiters[slot].as_ref().ok_or(Error::Faulted)?;
            if !w.waiting {
                return Err(Error::Unavailable);
            }
            match event {
                NativeOperationWaitEventV1::Stop | NativeOperationWaitEventV1::Expire => {
                    if f.owner_control_allowed != Some(true) {
                        return Err(Error::Unavailable);
                    }
                    o.observation_row_mut(id)?.waiters[slot]
                        .as_mut()
                        .expect("owned waiter")
                        .waiting = false;
                    Ok(NativeOperationWaitReplyV1::Ended)
                }
                NativeOperationWaitEventV1::Tick(clock) => {
                    if row.ceiling.recipient == p.recipient
                        && row.ceiling.epoch == p.epoch
                        && (!p.grant
                            || [f.authorized, f.scope, f.continuous, f.live, f.retained]
                                .into_iter()
                                .any(|v| v == Some(false)))
                    {
                        o.observation_row_mut(id)?.close();
                        return Err(Error::Unavailable);
                    }
                    let invalid = f.wait_clock_valid != Some(true)
                        || clock.epoch != w.clock.epoch
                        || clock.tick <= w.clock.tick;
                    let w = o.observation_row_mut(id)?.waiters[slot]
                        .as_mut()
                        .expect("owned waiter");
                    if invalid {
                        w.waiting = false;
                        return Err(Error::Unavailable);
                    }
                    w.clock = clock;
                    if w.remaining <= 1 {
                        w.remaining = 0;
                        w.waiting = false;
                        Ok(NativeOperationWaitReplyV1::Ended)
                    } else {
                        w.remaining -= 1;
                        Ok(NativeOperationWaitReplyV1::Waiting(wait.clone()))
                    }
                }
                NativeOperationWaitEventV1::Poll => {
                    let after = w.after.clone();
                    let next = o.query_observation_inner(
                        id,
                        NativeOperationObservationQueryV1::Status,
                        p,
                        f,
                        r,
                    );
                    let row = o.observation_row_mut(id)?;
                    match next {
                        Err(e) => {
                            row.waiters[slot].as_mut().expect("owned waiter").waiting = false;
                            Err(e)
                        }
                        Ok(next) => {
                            let terminal = matches!(
                                next,
                                NativeOperationObservationReplyV1::Status {
                                    phase: NativeOperationObservationPhaseV1::Terminal,
                                    ..
                                }
                            );
                            if next.observation() == &after && !terminal {
                                return Ok(NativeOperationWaitReplyV1::Waiting(wait.clone()));
                            }
                            row.waiters[slot].as_mut().expect("owned waiter").waiting = false;
                            let tail = reply_bytes(&next)?
                                .checked_sub(size_of::<NativeOperationObservationReplyV1>())
                                .ok_or(Error::Faulted)?;
                            if add(size_of::<NativeOperationWaitReplyV1>(), tail)?
                                > row.limits.max_reply_bytes
                            {
                                return Err(Error::Faulted);
                            }
                            Ok(NativeOperationWaitReplyV1::Observed(next))
                        }
                    }
                }
            }
        });
        if result == Err(Error::Faulted)
            && let Ok(slot) = self.waiter_slot(id, wait)
            && let Ok(row) = self.observation_row_mut(id)
        {
            row.waiters[slot].as_mut().expect("owned waiter").waiting = false;
        }
        observe(r, Boundary::NativeOperationObservationWait, &result);
        result
    }
    pub fn retire_native_operation_observation_wait_v1(
        &mut self,
        id: &NativeOperationObservationIdV1,
        wait: &NativeOperationWaitIdV1,
        f: &NativeOperationObservationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = (|| {
            let slot = self.waiter_slot(id, wait)?;
            let row = self.observation_row(id)?;
            if row.busy || row.context != *f.context || !discharged(f) {
                return Err(Error::Unavailable);
            }
            self.observation_row_mut(id)?.waiters[slot] = None;
            Ok(())
        })();
        observe(r, Boundary::NativeOperationObservationRetirement, &result);
        result
    }
}
