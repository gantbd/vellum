//! Actual diagnostic queue obligations and funding, including an opaque consumer.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_diagnostic_owner(
        &self,
        b: &mut Builder,
    ) -> Result<()> {
        let role = ShutdownOwnerRoleV1::DiagnosticCapture;
        let p = match &self.diagnostic_capture {
            State::Unconfigured => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Unconfigured);
            }
            State::Closed { issued } => {
                return self.record_shutdown_pool(
                    b,
                    role,
                    PoolSnapshot::Closed {
                        issued: Some(*issued),
                    },
                );
            }
            State::Active(p) => p,
        };
        let live = b.slots(&p.preparations)?;
        let mut bytes = 0usize;
        for q in &p.queue {
            b.step()?;
            if q.bytes != q.marker.canonical_record_bytes()
                || q.remaining == 0
                || q.remaining > p.limits.max_retention_steps
            {
                return Err(CoordinatorError::Faulted);
            }
            bytes = bytes
                .checked_add(q.bytes)
                .ok_or(CoordinatorError::Faulted)?;
        }
        if live != p.live
            || live > p.limits.max_preparations
            || p.issued > p.limits.max_prepared_issued
            || p.queue.len() > p.limits.max_records
            || bytes != p.bytes
            || bytes > p.limits.max_queue_bytes
            || p.generation.ordinal == 0
            || p.generation.ordinal > p.limits.max_generations
            || !Arc::ptr_eq(&p.generation.owner, &self.namespace)
            || p.binding.is_some() && !p.binding_spent
            || p.mode != DiagnosticModeV1::Enabled && (!p.queue.is_empty() || p.live != 0)
        {
            return Err(CoordinatorError::Faulted);
        }
        for row in p.preparations.iter().flatten() {
            if row.serial >= p.issued
                || row.bytes != row.marker.canonical_record_bytes()
                || row.bytes > p.limits.max_record_bytes
            {
                return Err(CoordinatorError::Faulted);
            }
        }
        let (additional_reserved, extra_retained, extra_issued) = match &p.gateway {
            Some(g) => g.shutdown_obligations(p, &self.retention, b)?,
            None => (0, 0, 0),
        };
        self.record_shutdown_pool(
            b,
            role,
            PoolSnapshot::Active {
                issued: p
                    .issued
                    .checked_add(extra_issued)
                    .ok_or(CoordinatorError::Limit)?,
                retained: 1usize
                    .checked_add(live)
                    .and_then(|n| n.checked_add(p.queue.len()))
                    .and_then(|n| n.checked_add(usize::from(p.binding.is_some())))
                    .and_then(|n| n.checked_add(extra_retained))
                    .ok_or(CoordinatorError::Limit)?,
                charge: p.charge,
                expected: p.limits.required_hold_bytes()?,
                reservation: &p.hold,
                stopped: p.mode != DiagnosticModeV1::Enabled,
                additional_reserved,
            },
        )
    }
}
