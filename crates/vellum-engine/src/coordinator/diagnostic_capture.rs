//! Actual owned optional marker queue. Separate from the existing richer native
//! Recorder: no implicit mapping or loss of its timing/boundary meaning.
use super::*;
use std::{collections::VecDeque, mem::size_of};
mod types;
pub use types::*;
mod drain;
pub use drain::*;
mod shutdown_inventory;
#[cfg(test)]
mod tests;
type Result<T> = std::result::Result<T, CoordinatorError>;
#[derive(Debug)]
struct Prepared {
    marker: DiagnosticMarkerV1,
    bytes: usize,
    serial: usize,
}
#[derive(Debug)]
struct Queued {
    marker: DiagnosticMarkerV1,
    bytes: usize,
    remaining: usize,
}
#[derive(Debug)]
pub(super) struct Pool {
    scope: DiagnosticScopeV1,
    limits: DiagnosticCaptureLimitsV1,
    hold: retention::Reservation,
    charge: usize,
    generation: DiagnosticGenerationV1,
    mode: DiagnosticModeV1,
    clock: DiagnosticClockV1,
    counters: DiagnosticCountersV1,
    queue: VecDeque<Queued>,
    bytes: usize,
    preparations: Vec<Option<Prepared>>,
    live: usize,
    issued: usize,
    binding: Option<Arc<()>>,
    binding_spent: bool,
    gateway: Option<Box<drain::Gateway>>,
}
#[derive(Debug, Default)]
pub(super) enum State {
    #[default]
    Unconfigured,
    Active(Box<Pool>),
    Closed {
        issued: usize,
    },
}
fn all(f: &[Option<bool>]) -> bool {
    f.iter().all(|v| *v == Some(true))
}
fn parent(e: RetentionErrorV1) -> CoordinatorError {
    match e {
        RetentionErrorV1::Unavailable => CoordinatorError::Unavailable,
        RetentionErrorV1::Limit => CoordinatorError::Limit,
        RetentionErrorV1::Faulted => CoordinatorError::Faulted,
    }
}
impl Pool {
    fn stop(&mut self, mode: DiagnosticModeV1) {
        self.counters
            .discarded
            .increment(self.queue.len(), self.limits.max_counter);
        self.queue.clear();
        self.bytes = 0;
        self.preparations.iter_mut().for_each(|v| *v = None);
        self.live = 0;
        self.mode = mode;
    }
    fn current(&mut self, s: &DiagnosticScopeV1, f: &TrustedDiagnosticFactsV1<'_>) -> Result<()> {
        if self.scope != *s || f.scope != s {
            return Err(CoordinatorError::Unavailable);
        }
        if !all(&[
            f.authorized,
            f.scope_allowed,
            f.continuous,
            f.live,
            f.not_stopped,
        ]) {
            self.stop(DiagnosticModeV1::Stopped);
            return Err(CoordinatorError::Unavailable);
        }
        if self.mode != DiagnosticModeV1::Enabled {
            return Err(CoordinatorError::Unavailable);
        }
        Ok(())
    }
}
impl SourceCoordinator {
    /// Already-decoded finite native record input, not a hostile-object parser or
    /// a wire codec. Invalid offered shapes count only under current collection
    /// permission and never retain arbitrary fields or payloads.
    pub fn prepare_decoded_diagnostic_marker_v1(
        &mut self,
        s: &DiagnosticScopeV1,
        value: &crate::capture::InertValueV1<'_>,
        f: &TrustedDiagnosticFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<DiagnosticPreparationV1> {
        if let Some(marker) = types::parse_marker(value) {
            return self.prepare_diagnostic_marker_v1(s, marker, f, r);
        }
        let result = (|| {
            let p = self.diagnostic_pool_mut()?;
            p.current(s, f)?;
            if all(&[
                f.capture_allowed,
                f.outside_publication_guard,
                f.resources_ready,
                f.preparation_reserved,
            ]) {
                p.counters.invalid.increment(1, p.limits.max_counter);
            }
            Err(CoordinatorError::Unavailable)
        })();
        observe(r, Boundary::DiagnosticMarkerPreparation, &result);
        result
    }
    fn diagnostic_pool(&self) -> Result<&Pool> {
        let State::Active(p) = &self.diagnostic_capture else {
            return Err(CoordinatorError::Unavailable);
        };
        self.retention
            .validate_exact(&p.hold, p.charge)
            .map_err(parent)?;
        Ok(p)
    }
    fn diagnostic_pool_mut(&mut self) -> Result<&mut Pool> {
        self.diagnostic_pool()?;
        let State::Active(p) = &mut self.diagnostic_capture else {
            unreachable!()
        };
        Ok(p)
    }
    pub fn configure_diagnostic_capture_v1(
        &mut self,
        s: &DiagnosticScopeV1,
        limits: DiagnosticCaptureLimitsV1,
        f: &DiagnosticControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<DiagnosticGenerationV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission()?;
            if !matches!(self.diagnostic_capture, State::Unconfigured)
                || f.scope != s
                || s.context.profile != BOUNDED_OPTIONAL_CAPTURE_PROFILE_V1
                || s.policy.is_empty()
                || settings::basis::labels(&s.context)
                    .iter()
                    .any(|v| v.is_empty())
                || !all(&[
                    f.owner_control_allowed,
                    f.resources_ready,
                    f.preparation_reserved,
                ])
            {
                return Err(CoordinatorError::Unavailable);
            }
            let context_bytes = settings::basis::labels(&s.context)
                .into_iter()
                .try_fold(size_of::<DiagnosticScopeV1>(), |n, s| {
                    n.checked_add(s.len()).ok_or(CoordinatorError::Limit)
                })?
                .checked_add(s.policy.len())
                .ok_or(CoordinatorError::Limit)?;
            let charge = limits.required_hold_bytes()?;
            if context_bytes > limits.max_context_bytes {
                return Err(CoordinatorError::Limit);
            }
            let mut hold = self.retention.reserve(charge).map_err(parent)?;
            let staged = (|| {
                let mut queue = VecDeque::new();
                queue
                    .try_reserve_exact(limits.max_records)
                    .map_err(|_| CoordinatorError::Capacity)?;
                let mut preparations = Vec::new();
                preparations
                    .try_reserve_exact(limits.max_preparations)
                    .map_err(|_| CoordinatorError::Capacity)?;
                preparations.resize_with(limits.max_preparations, || None);
                let context = settings::basis::copy_context(&s.context)
                    .map_err(|_| CoordinatorError::Capacity)?;
                let mut policy = String::new();
                policy
                    .try_reserve_exact(s.policy.len())
                    .map_err(|_| CoordinatorError::Capacity)?;
                policy.push_str(&s.policy);
                Ok((queue, preparations, DiagnosticScopeV1 { context, policy }))
            })();
            let (queue, preparations, scope) = match staged {
                Ok(v) => v,
                Err(e) => {
                    self.retention.release(&mut hold).map_err(parent)?;
                    return Err(e);
                }
            };
            let generation = DiagnosticGenerationV1 {
                owner: self.namespace.clone(),
                ordinal: 1,
            };
            self.diagnostic_capture = State::Active(Box::new(Pool {
                scope,
                limits,
                hold,
                charge,
                generation: generation.clone(),
                mode: DiagnosticModeV1::Enabled,
                clock: DiagnosticClockV1::Unobserved,
                counters: Default::default(),
                queue,
                bytes: 0,
                preparations,
                live: 0,
                issued: 0,
                binding: None,
                binding_spent: false,
                gateway: None,
            }));
            Ok(generation)
        })();
        observe_service(
            r,
            Boundary::DiagnosticCaptureConfiguration,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn prepare_diagnostic_marker_v1(
        &mut self,
        s: &DiagnosticScopeV1,
        marker: DiagnosticMarkerV1,
        f: &TrustedDiagnosticFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<DiagnosticPreparationV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let p = self.diagnostic_pool_mut()?;
            p.current(s, f)?;
            if !all(&[
                f.capture_allowed,
                f.outside_publication_guard,
                f.resources_ready,
                f.preparation_reserved,
            ]) {
                return Err(CoordinatorError::Unavailable);
            }
            if p.live >= p.limits.max_preparations || p.issued >= p.limits.max_prepared_issued {
                p.counters
                    .preparation_refused
                    .increment(1, p.limits.max_counter);
                return Err(CoordinatorError::Limit);
            }
            let bytes = marker.canonical_record_bytes();
            if bytes > p.limits.max_record_bytes {
                p.counters.invalid.increment(1, p.limits.max_counter);
                return Err(CoordinatorError::Limit);
            }
            let slot = p
                .preparations
                .iter()
                .position(Option::is_none)
                .ok_or(CoordinatorError::Faulted)?;
            let serial = p.issued;
            p.issued += 1;
            p.live += 1;
            p.preparations[slot] = Some(Prepared {
                marker,
                bytes,
                serial,
            });
            Ok(DiagnosticPreparationV1 {
                generation: p.generation.clone(),
                serial,
                slot,
            })
        })();
        observe_service(
            r,
            Boundary::DiagnosticMarkerPreparation,
            &result,
            timer.finish(),
        );
        result
    }
    /// Token-only, prepaid capture: no original record inspection, allocation,
    /// formatting, callback, clock read, or semantic root/invocation mutation.
    pub fn capture_diagnostic_marker_v1(
        &mut self,
        s: &DiagnosticScopeV1,
        token: &DiagnosticPreparationV1,
        f: &TrustedDiagnosticFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<DiagnosticCaptureDecisionV1> {
        let result = (|| {
            let p = self.diagnostic_pool_mut()?;
            p.current(s, f)?;
            if !all(&[f.capture_allowed, f.capture_ready]) || token.generation != p.generation {
                return Err(CoordinatorError::Unavailable);
            }
            let row = p
                .preparations
                .get_mut(token.slot)
                .ok_or(CoordinatorError::Unavailable)?;
            if row.as_ref().is_none_or(|r| r.serial != token.serial) {
                return Err(CoordinatorError::Unavailable);
            }
            let row = row.take().expect("original preparation");
            p.live -= 1;
            p.counters.offered.increment(1, p.limits.max_counter);
            if p.queue.len() >= p.limits.max_records
                || row.bytes > p.limits.max_queue_bytes.saturating_sub(p.bytes)
            {
                p.counters.dropped.increment(1, p.limits.max_counter);
                return Ok(DiagnosticCaptureDecisionV1::Dropped);
            }
            p.bytes += row.bytes;
            p.queue.push_back(Queued {
                marker: row.marker,
                bytes: row.bytes,
                remaining: p.limits.max_retention_steps,
            });
            p.counters.captured.increment(1, p.limits.max_counter);
            Ok(DiagnosticCaptureDecisionV1::Captured)
        })();
        r.record(Observation {
            boundary: Boundary::DiagnosticMarkerCapture,
            disposition: match &result {
                Ok(DiagnosticCaptureDecisionV1::Captured) => Disposition::Completed,
                Ok(DiagnosticCaptureDecisionV1::Dropped) => Disposition::Partial,
                Err(CoordinatorError::Faulted) => Disposition::Unknown,
                _ => Disposition::Refused,
            },
            service_time: None,
        });
        result
    }
    pub fn drain_diagnostic_markers_v1(
        &mut self,
        s: &DiagnosticScopeV1,
        generation: &DiagnosticGenerationV1,
        limit: usize,
        f: &TrustedDiagnosticFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<DiagnosticBatchV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let p = self.diagnostic_pool_mut()?;
            p.current(s, f)?;
            if !all(&[
                f.release_allowed,
                f.retained,
                f.outside_publication_guard,
                f.transfer_ready,
                f.reply_ready,
                f.resources_ready,
            ]) || *generation != p.generation
                || limit == 0
                || limit > p.limits.max_drain_records
            {
                return Err(CoordinatorError::Unavailable);
            }
            let n = limit.min(p.queue.len());
            let bytes = n
                .checked_mul(size_of::<DiagnosticMarkerV1>())
                .and_then(|n| n.checked_add(size_of::<DiagnosticBatchV1>()))
                .ok_or(CoordinatorError::Limit)?;
            if bytes > p.limits.max_reply_bytes {
                return Err(CoordinatorError::Limit);
            }
            let mut records = Vec::new();
            records
                .try_reserve_exact(n)
                .map_err(|_| CoordinatorError::Capacity)?;
            records.extend(p.queue.iter().take(n).map(|v| v.marker));
            let batch = DiagnosticBatchV1 {
                generation: p.generation.clone(),
                records,
                remaining: n < p.queue.len(),
            };
            for _ in 0..n {
                p.bytes -= p.queue.pop_front().expect("staged prefix").bytes;
            }
            p.counters.drained.increment(n, p.limits.max_counter);
            Ok(batch)
        })();
        observe_service(r, Boundary::DiagnosticMarkerDrain, &result, timer.finish());
        result
    }
    /// Protected metadata-only health. Deliberately emits no observation of itself.
    pub fn read_diagnostic_health_v1(
        &self,
        f: &DiagnosticControlV1<'_>,
    ) -> Result<DiagnosticHealthV1> {
        let p = self.diagnostic_pool()?;
        if p.scope != *f.scope
            || !all(&[
                f.owner_control_allowed,
                f.health_allowed,
                f.outside_publication_guard,
                f.reply_ready,
            ])
        {
            return Err(CoordinatorError::Unavailable);
        }
        Ok(DiagnosticHealthV1 {
            generation: p.generation.clone(),
            mode: p.mode,
            clock: p.clock,
            queued_records: p.queue.len(),
            queued_canonical_bytes: p.bytes,
            counters: p.counters,
        })
    }
    pub fn stop_diagnostic_capture_v1(
        &mut self,
        s: &DiagnosticScopeV1,
        disabled: bool,
        f: &DiagnosticControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<()> {
        let result = (|| {
            let p = self.diagnostic_pool_mut()?;
            if p.scope != *s || f.scope != s || f.owner_control_allowed != Some(true) {
                return Err(CoordinatorError::Unavailable);
            }
            p.stop(if disabled {
                DiagnosticModeV1::Disabled
            } else {
                DiagnosticModeV1::Stopped
            });
            Ok(())
        })();
        observe(r, Boundary::DiagnosticCaptureStop, &result);
        result
    }
    pub fn reset_diagnostic_capture_v1(
        &mut self,
        f: &DiagnosticControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<DiagnosticGenerationV1> {
        let result = (|| {
            let p = self.diagnostic_pool_mut()?;
            if p.scope != *f.scope
                || p.mode == DiagnosticModeV1::Enabled
                || !all(&[
                    f.owner_control_allowed,
                    f.resources_ready,
                    f.preparation_reserved,
                    f.outside_publication_guard,
                ])
                || p.generation.ordinal >= p.limits.max_generations
            {
                return Err(CoordinatorError::Unavailable);
            }
            p.generation.ordinal += 1;
            p.mode = DiagnosticModeV1::Enabled;
            p.clock = DiagnosticClockV1::Unobserved;
            p.counters = Default::default();
            Ok(p.generation.clone())
        })();
        observe(r, Boundary::DiagnosticCaptureReset, &result);
        result
    }
    pub fn advance_diagnostic_retention_v1(
        &mut self,
        steps: usize,
        clock_valid: Option<bool>,
        f: &DiagnosticControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<()> {
        let result = (|| {
            let p = self.diagnostic_pool_mut()?;
            if p.scope != *f.scope
                || p.mode != DiagnosticModeV1::Enabled
                || !all(&[f.owner_control_allowed, f.outside_publication_guard])
                || steps == 0
                || steps > p.limits.max_retention_steps
            {
                return Err(CoordinatorError::Unavailable);
            }
            if clock_valid != Some(true) {
                p.clock = DiagnosticClockV1::Invalid;
                p.stop(DiagnosticModeV1::Stopped);
                return Ok(());
            }
            p.clock = DiagnosticClockV1::Valid;
            let mut discarded = 0;
            p.queue.retain_mut(|q| {
                if steps >= q.remaining {
                    p.bytes -= q.bytes;
                    discarded += 1;
                    false
                } else {
                    q.remaining -= steps;
                    true
                }
            });
            p.counters
                .discarded
                .increment(discarded, p.limits.max_counter);
            Ok(())
        })();
        observe(r, Boundary::DiagnosticCaptureRetention, &result);
        result
    }
    pub fn retire_diagnostic_preparation_v1(
        &mut self,
        token: &DiagnosticPreparationV1,
        f: &DiagnosticControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<()> {
        let result = (|| {
            let p = self.diagnostic_pool_mut()?;
            if p.scope != *f.scope
                || !all(&[f.owner_control_allowed, f.retention_released])
                || token.generation != p.generation
            {
                return Err(CoordinatorError::Unavailable);
            }
            let row = p
                .preparations
                .get_mut(token.slot)
                .ok_or(CoordinatorError::Unavailable)?;
            if row.as_ref().is_none_or(|r| r.serial != token.serial) {
                return Err(CoordinatorError::Unavailable);
            }
            *row = None;
            p.live -= 1;
            Ok(())
        })();
        observe(r, Boundary::DiagnosticMarkerRetirement, &result);
        result
    }
    pub fn bind_diagnostic_drain_v1(
        &mut self,
        f: &DiagnosticControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<DiagnosticDrainBindingV1> {
        let result = (|| {
            self.ensure_ordinary_admission()?;
            let p = self.diagnostic_pool_mut()?;
            if p.scope != *f.scope
                || p.binding_spent
                || !all(&[
                    f.owner_control_allowed,
                    f.outside_publication_guard,
                    f.binding_ready,
                    f.preparation_reserved,
                    f.resources_ready,
                ])
            {
                return Err(CoordinatorError::Unavailable);
            }
            let marker = Arc::new(());
            p.binding = Some(marker.clone());
            p.binding_spent = true;
            Ok(DiagnosticDrainBindingV1 {
                owner: p.generation.owner.clone(),
                marker,
            })
        })();
        observe(r, Boundary::DiagnosticDrainBinding, &result);
        result
    }
    pub fn release_diagnostic_drain_binding_v1(
        &mut self,
        binding: &DiagnosticDrainBindingV1,
        f: &DiagnosticControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<()> {
        let result = (|| {
            let p = self.diagnostic_pool_mut()?;
            if p.scope != *f.scope
                || !Arc::ptr_eq(&binding.owner, &p.generation.owner)
                || p.binding
                    .as_ref()
                    .is_none_or(|m| !Arc::ptr_eq(m, &binding.marker))
                || !all(&[
                    f.owner_control_allowed,
                    f.outside_publication_guard,
                    f.retention_released,
                    f.delivery_drained,
                ])
            {
                return Err(CoordinatorError::Unavailable);
            }
            p.binding = None;
            Ok(())
        })();
        observe(r, Boundary::DiagnosticDrainBindingRetirement, &result);
        result
    }
    pub fn retire_diagnostic_capture_v1(
        &mut self,
        f: &DiagnosticControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<()> {
        let result = (|| {
            let p = self.diagnostic_pool()?;
            if p.scope != *f.scope
                || p.mode == DiagnosticModeV1::Enabled
                || p.live != 0
                || !p.queue.is_empty()
                || p.binding.is_some()
                || p.gateway.is_some()
                || !all(&[
                    f.owner_control_allowed,
                    f.retention_released,
                    f.delivery_drained,
                ])
            {
                return Err(CoordinatorError::Unavailable);
            }
            let State::Active(p) = &mut self.diagnostic_capture else {
                unreachable!()
            };
            self.retention.release(&mut p.hold).map_err(parent)?;
            self.diagnostic_capture = State::Closed { issued: p.issued };
            Ok(())
        })();
        observe(r, Boundary::DiagnosticCaptureRetirement, &result);
        result
    }
}
