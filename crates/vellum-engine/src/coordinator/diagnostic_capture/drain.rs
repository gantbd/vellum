//! Local diagnostic bridge. No native invocation kind, scheduler, callback or I/O.
use super::*;
mod types;
pub use types::*;
mod arguments;
pub use arguments::*;
#[cfg(test)]
mod tests;
type Result<T> = std::result::Result<T, DiagnosticDrainErrorV1>;
#[derive(Debug)]
struct Row {
    serial: usize,
    generation: DiagnosticGenerationV1,
    phase: DiagnosticDrainPhaseV1,
    intent: Option<usize>,
    disposition: Option<DiagnosticDrainDispositionV1>,
    batch: Option<DiagnosticBatchV1>,
}
#[derive(Debug)]
pub(super) struct Gateway {
    scope: DiagnosticScopeV1,
    limits: DiagnosticDrainLimitsV1,
    binding: DiagnosticDrainBindingV1,
    hold: retention::Reservation,
    charge: usize,
    queue_reply_bytes: usize,
    rows: Vec<Option<Row>>,
    issued: usize,
    live: usize,
    stopped: bool,
    faulted: bool,
    released: usize,
    #[cfg(test)]
    fault: Option<Fault>,
}
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
enum Fault {
    BeforeDrain,
    AfterDrain,
    WrongGeneration,
    UnwindAfterDrain,
}
fn queue_error(e: CoordinatorError) -> DiagnosticDrainErrorV1 {
    match e {
        CoordinatorError::Limit => DiagnosticDrainErrorV1::Limit,
        CoordinatorError::Capacity => DiagnosticDrainErrorV1::Capacity,
        CoordinatorError::Faulted => DiagnosticDrainErrorV1::Faulted,
        _ => DiagnosticDrainErrorV1::Unavailable,
    }
}
fn funding(e: RetentionErrorV1) -> DiagnosticDrainErrorV1 {
    queue_error(parent(e))
}
fn record<T>(
    r: &mut Recorder,
    b: Boundary,
    result: &Result<T>,
    elapsed: Option<std::time::Duration>,
) {
    r.record(Observation {
        boundary: b,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(DiagnosticDrainErrorV1::Faulted) => Disposition::Unknown,
            _ => Disposition::Refused,
        },
        service_time: elapsed,
    });
}
fn scope_bytes(s: &DiagnosticScopeV1) -> Result<usize> {
    settings::basis::labels(&s.context)
        .into_iter()
        .try_fold(size_of::<DiagnosticScopeV1>(), |n, s| {
            n.checked_add(s.len()).ok_or(DiagnosticDrainErrorV1::Limit)
        })?
        .checked_add(s.policy.len())
        .ok_or(DiagnosticDrainErrorV1::Limit)
}
fn same_scope(q: &DiagnosticScopeV1, g: &DiagnosticScopeV1) -> bool {
    q.policy == g.policy
        && q.context.authority == g.context.authority
        && q.context.engine == g.context.engine
        && q.context.recipient == g.context.recipient
        && q.context.purpose == g.context.purpose
        && q.context.destination == g.context.destination
        && q.context.continuity == g.context.continuity
        && q.context.profile == BOUNDED_OPTIONAL_CAPTURE_PROFILE_V1
        && g.context.profile == OWNED_DIAGNOSTIC_DRAIN_PROFILE_V1
}
fn batch_bytes(b: &DiagnosticBatchV1) -> Result<usize> {
    b.records
        .len()
        .checked_mul(size_of::<DiagnosticMarkerV1>())
        .and_then(|n| n.checked_add(size_of::<DiagnosticBatchV1>()))
        .ok_or(DiagnosticDrainErrorV1::Limit)
}
impl Gateway {
    pub(super) fn shutdown_obligations(
        &self,
        pool: &Pool,
        ledger: &retention::RetentionState,
        b: &mut crate::coordinator::shutdown::inventory::Builder,
    ) -> std::result::Result<(usize, usize, usize), CoordinatorError> {
        let fail = |_| CoordinatorError::Faulted;
        if self.charge != self.limits.required_hold_bytes().map_err(fail)?
            || self.live > self.limits.max_tasks
            || self.issued > self.limits.max_issued
            || self.released > self.issued
            || self.queue_reply_bytes != pool.limits.max_reply_bytes
            || !same_scope(&pool.scope, &self.scope)
            || pool
                .binding
                .as_ref()
                .is_none_or(|m| !Arc::ptr_eq(m, &self.binding.marker))
            || !Arc::ptr_eq(&self.binding.owner, &pool.generation.owner)
        {
            return Err(CoordinatorError::Faulted);
        }
        ledger
            .validate_exact(&self.hold, self.charge)
            .map_err(|_| CoordinatorError::Faulted)?;
        if b.slots(&self.rows)? != self.live {
            return Err(CoordinatorError::Faulted);
        }
        let mut retained = 1usize
            .checked_add(self.live)
            .ok_or(CoordinatorError::Limit)?;
        for (slot, row) in self
            .rows
            .iter()
            .enumerate()
            .filter_map(|(i, r)| r.as_ref().map(|r| (i, r)))
        {
            let task = DiagnosticDrainTaskV1 {
                owner: self.binding.marker.clone(),
                slot,
                serial: row.serial,
            };
            self.row(&task).map_err(fail)?;
            if !Arc::ptr_eq(&row.generation.owner, &pool.generation.owner)
                || row.generation.ordinal == 0
                || row.generation.ordinal > pool.limits.max_generations
            {
                return Err(CoordinatorError::Faulted);
            }
            if let Some(batch) = &row.batch {
                b.step()?;
                if batch.generation != row.generation
                    || batch.records.len() > row.intent.unwrap_or(0)
                    || batch_bytes(batch).map_err(fail)? > self.limits.max_batch_bytes
                {
                    return Err(CoordinatorError::Faulted);
                }
                retained = retained.checked_add(1).ok_or(CoordinatorError::Limit)?;
            }
        }
        Ok((self.charge, retained, self.issued))
    }
    fn discard(&mut self, slot: usize) {
        if self.rows[slot]
            .as_mut()
            .is_some_and(|r| r.batch.take().is_some())
        {
            self.released += 1;
        }
    }
    fn fence(&mut self, faulted: bool) {
        self.stopped = true;
        self.faulted |= faulted;
        for slot in 0..self.rows.len() {
            if let Some(r) = &mut self.rows[slot] {
                if r.phase == DiagnosticDrainPhaseV1::Pending {
                    r.phase = DiagnosticDrainPhaseV1::Terminal;
                    r.disposition = Some(DiagnosticDrainDispositionV1::NotPerformed);
                }
                self.discard(slot);
            }
        }
    }
    fn current(
        &mut self,
        scope: &DiagnosticScopeV1,
        f: &TrustedDiagnosticDrainFactsV1<'_>,
    ) -> Result<()> {
        if self.scope != *scope || f.scope != scope || f.outside_publication_guard != Some(true) {
            return Err(DiagnosticDrainErrorV1::Unavailable);
        }
        if !all(&[
            f.authorized,
            f.scope_allowed,
            f.continuous,
            f.live,
            f.not_stopped,
        ]) {
            self.fence(false);
            return Err(DiagnosticDrainErrorV1::Unavailable);
        }
        if self.faulted {
            self.fence(true);
        }
        if self.stopped {
            return Err(DiagnosticDrainErrorV1::Unavailable);
        }
        Ok(())
    }
    fn row(&self, task: &DiagnosticDrainTaskV1) -> Result<&Row> {
        if !Arc::ptr_eq(&task.owner, &self.binding.marker) {
            return Err(DiagnosticDrainErrorV1::Unavailable);
        }
        let row = self
            .rows
            .get(task.slot)
            .and_then(Option::as_ref)
            .filter(|r| r.serial == task.serial)
            .ok_or(DiagnosticDrainErrorV1::Unavailable)?;
        if row.serial >= self.issued
            || match row.phase {
                DiagnosticDrainPhaseV1::Issued => {
                    row.intent.is_some() || row.disposition.is_some() || row.batch.is_some()
                }
                DiagnosticDrainPhaseV1::Pending => {
                    row.intent.is_none() || row.disposition.is_some() || row.batch.is_some()
                }
                DiagnosticDrainPhaseV1::Terminal => {
                    row.disposition.is_none()
                        || row.batch.is_some()
                            && row.disposition != Some(DiagnosticDrainDispositionV1::Performed)
                        || row.disposition == Some(DiagnosticDrainDispositionV1::Performed)
                            && row.intent.is_none()
                }
            }
        {
            return Err(DiagnosticDrainErrorV1::Faulted);
        }
        Ok(row)
    }
    fn controlled(&self, f: &DiagnosticDrainControlV1<'_>, retirement: bool) -> bool {
        self.scope == *f.scope
            && all(&[f.owner_control_allowed, f.outside_publication_guard])
            && if retirement {
                all(&[f.retention_released, f.delivery_drained])
            } else {
                f.control_allowed == Some(true)
            }
    }
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn native_diagnostic_drain_profile_present(&self) -> bool {
        self.diagnostic_gateway().is_ok()
    }
    fn diagnostic_gateway(&self) -> Result<&Gateway> {
        let p = self.diagnostic_pool().map_err(queue_error)?;
        let g = p
            .gateway
            .as_deref()
            .ok_or(DiagnosticDrainErrorV1::Unavailable)?;
        if p.binding
            .as_ref()
            .is_none_or(|m| !Arc::ptr_eq(m, &g.binding.marker))
            || !Arc::ptr_eq(&g.binding.owner, &p.generation.owner)
            || g.charge != g.limits.required_hold_bytes()?
            || g.released > g.issued
            || g.live > g.limits.max_tasks
            || g.issued > g.limits.max_issued
        {
            return Err(DiagnosticDrainErrorV1::Faulted);
        }
        self.retention
            .validate_exact(&g.hold, g.charge)
            .map_err(funding)?;
        Ok(g)
    }
    fn diagnostic_gateway_mut(&mut self) -> Result<&mut Gateway> {
        self.diagnostic_gateway()?;
        let State::Active(p) = &mut self.diagnostic_capture else {
            unreachable!()
        };
        Ok(p.gateway.as_deref_mut().expect("validated original bridge"))
    }
    pub fn open_diagnostic_drain_v1(
        &mut self,
        scope: &DiagnosticScopeV1,
        limits: DiagnosticDrainLimitsV1,
        f: &DiagnosticDrainControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<()> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(queue_error)?;
            let p = self.diagnostic_pool().map_err(queue_error)?;
            if p.gateway.is_some()
                || p.binding_spent
                || f.scope != scope
                || !same_scope(&p.scope, scope)
                || !all(&[
                    f.owner_control_allowed,
                    f.outside_publication_guard,
                    f.resources_ready,
                    f.preparation_reserved,
                    f.queue_interpretation_current,
                    f.drain_interpretation_current,
                    f.binding_ready,
                ])
            {
                return Err(DiagnosticDrainErrorV1::Unavailable);
            }
            let queue_reply_bytes = p.limits.max_reply_bytes;
            let binding_bytes = size_of::<DiagnosticDrainBindingV1>()
                .checked_add(scope_bytes(&p.scope)?)
                .ok_or(DiagnosticDrainErrorV1::Limit)?;
            if scope_bytes(scope)? > limits.max_context_bytes
                || scope_bytes(&p.scope)? > limits.max_context_bytes
                || binding_bytes > limits.max_binding_bytes
                || limits.max_request_bytes
                    < size_of::<DiagnosticDrainIssuedV1>() + size_of::<usize>()
                || limits.max_batch_bytes < queue_reply_bytes
                || limits.max_result_bytes
                    < queue_reply_bytes
                        .checked_add(size_of::<DiagnosticDrainResultV1>())
                        .ok_or(DiagnosticDrainErrorV1::Limit)?
                || limits.max_control_reply_bytes < size_of::<DiagnosticDrainControlReplyV1>()
            {
                return Err(DiagnosticDrainErrorV1::Limit);
            }
            let charge = limits.required_hold_bytes()?;
            let mut hold = self.retention.reserve(charge).map_err(funding)?;
            let staged = (|| {
                let mut rows = Vec::new();
                rows.try_reserve_exact(limits.max_tasks)
                    .map_err(|_| DiagnosticDrainErrorV1::Capacity)?;
                rows.resize_with(limits.max_tasks, || None);
                let context = settings::basis::copy_context(&scope.context)
                    .map_err(|_| DiagnosticDrainErrorV1::Capacity)?;
                let mut policy = String::new();
                policy
                    .try_reserve_exact(scope.policy.len())
                    .map_err(|_| DiagnosticDrainErrorV1::Capacity)?;
                policy.push_str(&scope.policy);
                Ok((rows, DiagnosticScopeV1 { context, policy }))
            })();
            let (rows, scope) = match staged {
                Ok(v) => v,
                Err(e) => {
                    self.retention.release(&mut hold).map_err(funding)?;
                    return Err(e);
                }
            };
            // All fallible staging precedes spending the queue's one binding.
            let State::Active(p) = &mut self.diagnostic_capture else {
                unreachable!()
            };
            let marker = Arc::new(());
            p.binding = Some(marker.clone());
            p.binding_spent = true;
            let binding = DiagnosticDrainBindingV1 {
                owner: p.generation.owner.clone(),
                marker,
            };
            p.gateway = Some(Box::new(Gateway {
                scope,
                limits,
                binding,
                hold,
                charge,
                queue_reply_bytes,
                rows,
                issued: 0,
                live: 0,
                stopped: false,
                faulted: false,
                released: 0,
                #[cfg(test)]
                fault: None,
            }));
            Ok(())
        })();
        record(r, Boundary::DiagnosticDrainOpen, &result, timer.finish());
        result
    }
    pub fn issue_diagnostic_drain_v1(
        &mut self,
        scope: &DiagnosticScopeV1,
        f: &TrustedDiagnosticDrainFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<DiagnosticDrainIssuedV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(queue_error)?;
            self.diagnostic_gateway_mut()?.current(scope, f)?;
            let p = self.diagnostic_pool().map_err(queue_error)?;
            if p.mode != DiagnosticModeV1::Enabled {
                return Err(DiagnosticDrainErrorV1::Unavailable);
            }
            let generation = p.generation.clone();
            let g = self.diagnostic_gateway_mut()?;
            if !all(&[
                f.drain_allowed,
                f.resources_ready,
                f.bookkeeping_reserved,
                f.completion_reserved,
                f.queue_interpretation_current,
                f.drain_interpretation_current,
            ]) {
                return Err(DiagnosticDrainErrorV1::Unavailable);
            }
            if g.live >= g.limits.max_tasks || g.issued >= g.limits.max_issued {
                return Err(DiagnosticDrainErrorV1::Limit);
            }
            let slot = g
                .rows
                .iter()
                .position(Option::is_none)
                .ok_or(DiagnosticDrainErrorV1::Faulted)?;
            let task = DiagnosticDrainTaskV1 {
                owner: g.binding.marker.clone(),
                slot,
                serial: g.issued,
            };
            g.rows[slot] = Some(Row {
                serial: g.issued,
                generation: generation.clone(),
                phase: DiagnosticDrainPhaseV1::Issued,
                intent: None,
                disposition: None,
                batch: None,
            });
            g.issued += 1;
            g.live += 1;
            Ok(DiagnosticDrainIssuedV1 { task, generation })
        })();
        record(r, Boundary::DiagnosticDrainIssue, &result, timer.finish());
        result
    }
    #[allow(clippy::too_many_arguments)]
    pub fn submit_diagnostic_drain_v1(
        &mut self,
        task: &DiagnosticDrainTaskV1,
        generation: &DiagnosticGenerationV1,
        limit: usize,
        scope: &DiagnosticScopeV1,
        f: &TrustedDiagnosticDrainFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<DiagnosticDrainPhaseV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let g = self.diagnostic_gateway_mut()?;
            g.current(scope, f)?;
            if !all(&[
                f.drain_allowed,
                f.resources_ready,
                f.bookkeeping_reserved,
                f.completion_reserved,
                f.queue_interpretation_current,
                f.drain_interpretation_current,
            ]) {
                return Err(DiagnosticDrainErrorV1::Unavailable);
            }
            let row = g.row(task)?;
            if limit == 0 || limit as u128 > 9_007_199_254_740_991 {
                return Err(DiagnosticDrainErrorV1::Unavailable);
            }
            if *generation != row.generation {
                return Err(DiagnosticDrainErrorV1::IntentMismatch);
            }
            if let Some(original) = row.intent {
                return if original == limit {
                    Ok(row.phase)
                } else {
                    Err(DiagnosticDrainErrorV1::IntentMismatch)
                };
            }
            if row.phase != DiagnosticDrainPhaseV1::Issued {
                return Err(DiagnosticDrainErrorV1::IntentMismatch);
            }
            self.ensure_ordinary_admission().map_err(queue_error)?;
            let p = self.diagnostic_pool().map_err(queue_error)?;
            if p.mode != DiagnosticModeV1::Enabled
                || p.generation != *generation
                || limit > p.limits.max_drain_records
            {
                return Err(DiagnosticDrainErrorV1::Unavailable);
            }
            let g = self.diagnostic_gateway_mut()?;
            let row = g.rows[task.slot].as_mut().expect("original issued task");
            row.intent = Some(limit);
            row.phase = DiagnosticDrainPhaseV1::Pending;
            Ok(DiagnosticDrainPhaseV1::Pending)
        })();
        record(
            r,
            Boundary::DiagnosticDrainSubmission,
            &result,
            timer.finish(),
        );
        result
    }
    /// One serialized eligible turn. Missing readiness leaves pending work; a
    /// known refusal settles notPerformed. Faulted means this optional bridge,
    /// never an assertion about native document publication or external delivery.
    pub fn run_diagnostic_drain_v1(
        &mut self,
        task: &DiagnosticDrainTaskV1,
        scope: &DiagnosticScopeV1,
        f: &TrustedDiagnosticDrainFactsV1<'_>,
        queue_facts: &TrustedDiagnosticFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<DiagnosticDrainPhaseV1> {
        let timer = r.begin_service_timing();
        let was_pending = self.diagnostic_gateway().is_ok_and(|g| {
            g.scope == *scope
                && g.row(task)
                    .is_ok_and(|r| r.phase == DiagnosticDrainPhaseV1::Pending)
        });
        let attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let g = self.diagnostic_gateway_mut()?;
            g.current(scope, f)?;
            if !all(&[
                f.scheduler_owned,
                f.resources_ready,
                f.completion_reserved,
                f.queue_interpretation_current,
                f.drain_interpretation_current,
            ]) {
                return Err(DiagnosticDrainErrorV1::Unavailable);
            }
            let row = g.row(task)?;
            if row.phase != DiagnosticDrainPhaseV1::Pending {
                return Err(DiagnosticDrainErrorV1::Unavailable);
            }
            let generation = row.generation.clone();
            let limit = row.intent.ok_or(DiagnosticDrainErrorV1::Faulted)?;
            let p = self.diagnostic_pool().map_err(queue_error)?;
            if p.scope != *queue_facts.scope {
                return Err(DiagnosticDrainErrorV1::Unavailable);
            }
            let eligible = p.mode == DiagnosticModeV1::Enabled
                && p.generation == generation
                && f.drain_allowed == Some(true);
            // Mark the one attempted service before calling the queue. No public
            // fourth phase or retry is introduced; unwind cannot leave it pending.
            let g = self.diagnostic_gateway_mut()?;
            g.faulted = true;
            let row = g.rows[task.slot].as_mut().expect("original pending task");
            row.phase = DiagnosticDrainPhaseV1::Terminal;
            row.disposition = Some(DiagnosticDrainDispositionV1::Unknown);
            #[cfg(test)]
            if matches!(g.fault, Some(Fault::BeforeDrain)) {
                return Err(DiagnosticDrainErrorV1::Faulted);
            }
            #[allow(unused_mut)]
            let mut decision = if !eligible {
                None
            } else {
                let mut actual = *queue_facts;
                if f.queue_release_allowed != Some(true) {
                    actual.release_allowed = Some(false);
                }
                match self.drain_diagnostic_markers_v1(
                    queue_facts.scope,
                    &generation,
                    limit,
                    &actual,
                    r,
                ) {
                    Ok(batch) => Some(batch),
                    Err(CoordinatorError::Faulted) => return Err(DiagnosticDrainErrorV1::Faulted),
                    Err(_) => None,
                }
            };
            #[cfg(test)]
            match self.diagnostic_gateway()?.fault {
                Some(Fault::AfterDrain) => return Err(DiagnosticDrainErrorV1::Faulted),
                Some(Fault::WrongGeneration) => {
                    if let Some(batch) = &mut decision {
                        batch.generation.ordinal = batch.generation.ordinal.saturating_add(1);
                    }
                }
                Some(Fault::UnwindAfterDrain) => {
                    panic!("controlled diagnostic service interruption")
                }
                _ => {}
            }
            let g = self.diagnostic_gateway_mut()?;
            if let Some(batch) = &decision {
                let bytes = batch_bytes(batch).map_err(|_| DiagnosticDrainErrorV1::Faulted)?;
                if batch.generation != generation
                    || batch.records.len() > limit
                    || bytes > g.limits.max_batch_bytes
                    || bytes > g.queue_reply_bytes
                {
                    return Err(DiagnosticDrainErrorV1::Faulted);
                }
            }
            let row = g.rows[task.slot].as_mut().expect("original pending task");
            row.phase = DiagnosticDrainPhaseV1::Terminal;
            row.disposition = Some(if decision.is_some() {
                DiagnosticDrainDispositionV1::Performed
            } else {
                DiagnosticDrainDispositionV1::NotPerformed
            });
            row.batch = decision;
            g.faulted = false;
            Ok(DiagnosticDrainPhaseV1::Terminal)
        }));
        let result = attempt.unwrap_or(Err(DiagnosticDrainErrorV1::Faulted));
        if result == Err(DiagnosticDrainErrorV1::Faulted)
            && let State::Active(p) = &mut self.diagnostic_capture
            && let Some(g) = p.gateway.as_deref_mut()
        {
            if Arc::ptr_eq(&task.owner, &g.binding.marker)
                && let Some(row) = g.rows.get_mut(task.slot).and_then(Option::as_mut)
                && row.serial == task.serial
                && row.phase == DiagnosticDrainPhaseV1::Pending
            {
                row.phase = DiagnosticDrainPhaseV1::Terminal;
                row.disposition = Some(DiagnosticDrainDispositionV1::Unknown);
            }
            g.fence(true);
        }
        record(r, Boundary::DiagnosticDrainService, &result, timer.finish());
        if was_pending
            && f.scope == scope
            && let Ok(g) = self.diagnostic_gateway()
            && let Ok(row) = g.row(task)
        {
            let boundary = match row.disposition {
                Some(DiagnosticDrainDispositionV1::Performed) => Boundary::DiagnosticDrainPerformed,
                Some(DiagnosticDrainDispositionV1::NotPerformed) => {
                    Boundary::DiagnosticDrainNotPerformed
                }
                Some(DiagnosticDrainDispositionV1::Unknown) => Boundary::DiagnosticDrainUnknown,
                None => Boundary::DiagnosticDrainDeferred,
            };
            record(r, boundary, &Ok(()), None);
        }
        result
    }
    pub fn query_diagnostic_drain_result_v1(
        &mut self,
        task: &DiagnosticDrainTaskV1,
        scope: &DiagnosticScopeV1,
        f: &TrustedDiagnosticDrainFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<DiagnosticDrainResultV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let generation = self
                .diagnostic_pool()
                .map_err(queue_error)?
                .generation
                .clone();
            let g = self.diagnostic_gateway_mut()?;
            g.current(scope, f)?;
            if !all(&[
                f.result_allowed,
                f.queue_release_allowed,
                f.retained,
                f.transfer_ready,
                f.reply_ready,
                f.resources_ready,
                f.queue_interpretation_current,
                f.drain_interpretation_current,
            ]) {
                return Err(DiagnosticDrainErrorV1::Unavailable);
            }
            let row = g.row(task)?;
            if row.phase != DiagnosticDrainPhaseV1::Terminal {
                return Err(DiagnosticDrainErrorV1::Unavailable);
            }
            if row.generation != generation {
                g.discard(task.slot);
            }
            let row = g.row(task)?;
            let batch = if let Some(b) = &row.batch {
                let bytes = batch_bytes(b).map_err(|_| DiagnosticDrainErrorV1::Faulted)?;
                if b.generation != row.generation
                    || bytes > g.limits.max_batch_bytes
                    || bytes
                        .checked_add(size_of::<DiagnosticDrainResultV1>())
                        .is_none_or(|n| n > g.limits.max_result_bytes)
                {
                    return Err(DiagnosticDrainErrorV1::Faulted);
                }
                let mut records = Vec::new();
                records
                    .try_reserve_exact(b.records.len())
                    .map_err(|_| DiagnosticDrainErrorV1::Capacity)?;
                records.extend_from_slice(&b.records);
                Some(DiagnosticBatchV1 {
                    generation: b.generation.clone(),
                    records,
                    remaining: b.remaining,
                })
            } else {
                None
            };
            Ok(DiagnosticDrainResultV1 {
                task: task.clone(),
                admitted: row.intent.is_some(),
                drain: row.disposition.ok_or(DiagnosticDrainErrorV1::Faulted)?,
                batch,
            })
        })();
        record(r, Boundary::DiagnosticDrainResult, &result, timer.finish());
        result
    }
    pub fn query_diagnostic_drain_control_v1(
        &self,
        task: &DiagnosticDrainTaskV1,
        f: &DiagnosticDrainControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<DiagnosticDrainControlReplyV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let g = self.diagnostic_gateway()?;
            if !g.controlled(f, false) || !all(&[f.control_ready, f.transfer_ready]) {
                return Err(DiagnosticDrainErrorV1::Unavailable);
            }
            let row = g.row(task)?;
            Ok(DiagnosticDrainControlReplyV1 {
                task: task.clone(),
                phase: row.phase,
                admitted: row.intent.is_some(),
                drain: row.disposition,
            })
        })();
        record(r, Boundary::DiagnosticDrainControl, &result, timer.finish());
        result
    }
    pub fn stop_diagnostic_drain_task_v1(
        &mut self,
        task: &DiagnosticDrainTaskV1,
        f: &DiagnosticDrainControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<()> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let g = self.diagnostic_gateway_mut()?;
            if !g.controlled(f, false) {
                return Err(DiagnosticDrainErrorV1::Unavailable);
            }
            g.row(task)?;
            let row = g.rows[task.slot].as_mut().expect("original task");
            if row.phase != DiagnosticDrainPhaseV1::Terminal {
                row.phase = DiagnosticDrainPhaseV1::Terminal;
                row.disposition = Some(DiagnosticDrainDispositionV1::NotPerformed);
            }
            g.discard(task.slot);
            Ok(())
        })();
        record(
            r,
            Boundary::DiagnosticDrainTaskStop,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn retire_diagnostic_drain_task_v1(
        &mut self,
        task: &DiagnosticDrainTaskV1,
        f: &DiagnosticDrainControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<()> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let g = self.diagnostic_gateway_mut()?;
            if !g.controlled(f, true) || g.row(task)?.phase == DiagnosticDrainPhaseV1::Pending {
                return Err(DiagnosticDrainErrorV1::Unavailable);
            }
            g.discard(task.slot);
            g.rows[task.slot] = None;
            g.live -= 1;
            Ok(())
        })();
        record(
            r,
            Boundary::DiagnosticDrainTaskRetirement,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn stop_diagnostic_drain_v1(
        &mut self,
        f: &DiagnosticDrainControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<()> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let g = self.diagnostic_gateway_mut()?;
            if !g.controlled(f, false) {
                return Err(DiagnosticDrainErrorV1::Unavailable);
            }
            g.fence(false);
            Ok(())
        })();
        record(r, Boundary::DiagnosticDrainStop, &result, timer.finish());
        result
    }
    pub fn retire_diagnostic_drain_v1(
        &mut self,
        f: &DiagnosticDrainControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<()> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let g = self.diagnostic_gateway()?;
            if g.live != g.rows.iter().flatten().count() {
                return Err(DiagnosticDrainErrorV1::Faulted);
            }
            if !g.controlled(f, true) || !g.stopped || g.live != 0 {
                return Err(DiagnosticDrainErrorV1::Unavailable);
            }
            let State::Active(p) = &mut self.diagnostic_capture else {
                unreachable!()
            };
            let g = p.gateway.as_deref_mut().expect("original bridge");
            self.retention.release(&mut g.hold).map_err(funding)?;
            p.gateway = None;
            p.binding = None;
            Ok(())
        })();
        record(
            r,
            Boundary::DiagnosticDrainRetirement,
            &result,
            timer.finish(),
        );
        result
    }
}
