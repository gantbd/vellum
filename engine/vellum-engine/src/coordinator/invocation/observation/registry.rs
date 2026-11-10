//! Engine-retained provenance for host-owned contexts, never drop-triggered refunds.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, LocalOwnerSnapshot};

pub(in crate::coordinator::invocation) struct Record {
    context: InvocationObservationContextIdV1,
    invocation: InvocationId,
    handle: std::sync::Weak<()>,
    limits: InvocationObservationLimitsV1,
    charge: usize,
}
#[derive(Default)]
pub(in crate::coordinator::invocation) struct Registry {
    rows: Vec<Option<Record>>,
    retained: usize,
    issued: usize,
}
impl std::fmt::Debug for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationObserverRegistry(<owned:redacted>)")
    }
}
impl Registry {
    pub(super) fn prepare(&mut self) -> Result<(usize, usize), InvocationError> {
        let next = self.issued.checked_add(1).ok_or(InvocationError::Limit)?;
        if self.retained > self.rows.len()
            || self.rows.len() > self.issued
            || self.rows.iter().flatten().count() != self.retained
        {
            return Err(InvocationError::Faulted);
        }
        let slot = self
            .rows
            .iter()
            .position(Option::is_none)
            .unwrap_or(self.rows.len());
        if slot == self.rows.len() {
            self.rows
                .try_reserve(1)
                .map_err(|_| InvocationError::Capacity)?;
        }
        Ok((slot, next))
    }
    pub(super) fn register(&mut self, observer: &InvocationObservationContextV1, next: usize) {
        let row = Some(Record {
            context: observer.context_id_v1(),
            invocation: observer.invocation.clone(),
            handle: Arc::downgrade(&observer.handle_life),
            limits: observer.limits,
            // Already validated and reserved before any context escapes.
            charge: observer
                .limits
                .required_hold_bytes()
                .expect("validated observer envelope"),
        });
        if observer.registry_slot == self.rows.len() {
            self.rows.push(row);
        } else {
            self.rows[observer.registry_slot] = row;
        }
        self.retained += 1;
        self.issued = next;
    }
    pub(super) fn validate(
        &self,
        observer: &InvocationObservationContextV1,
    ) -> Result<(), InvocationError> {
        if self.retained == 0 || self.retained > self.rows.len() || self.rows.len() > self.issued {
            return Err(InvocationError::Faulted);
        }
        let row = self
            .rows
            .get(observer.registry_slot)
            .and_then(Option::as_ref)
            .ok_or(InvocationError::Faulted)?;
        if row.context != observer.context_id_v1()
            || row.invocation != observer.invocation
            || row.limits != observer.limits
            || row.charge != observer.limits.required_hold_bytes()?
            || !std::sync::Weak::ptr_eq(&row.handle, &Arc::downgrade(&observer.handle_life))
        {
            return Err(InvocationError::Faulted);
        }
        Ok(())
    }
    pub(super) fn release(&mut self, observer: &InvocationObservationContextV1) {
        self.rows[observer.registry_slot] = None;
        self.retained -= 1;
        if self.retained == 0 {
            self.rows = Vec::new();
        }
    }
    pub(super) fn validate_retirement(
        &self,
        observer: &InvocationObservationContextV1,
    ) -> Result<(), InvocationError> {
        self.validate(observer)?;
        if self.rows.iter().flatten().count() != self.retained {
            return Err(InvocationError::Faulted);
        }
        Ok(())
    }
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_invocation_observers(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let registry = &self.invocations.observers;
        let retained = b.slots(&registry.rows)?;
        if retained != registry.retained || registry.rows.len() > registry.issued {
            return Err(CoordinatorError::Faulted);
        }
        let mut bytes = 0usize;
        for row in registry.rows.iter().flatten() {
            b.step()?;
            if !Arc::ptr_eq(&row.invocation.namespace, &self.namespace)
                || row.invocation.slot >= self.invocations.slots.len()
                || row.limits.required_hold_bytes().ok() != Some(row.charge)
            {
                return Err(CoordinatorError::Faulted);
            }
            bytes = bytes
                .checked_add(row.charge)
                .ok_or(CoordinatorError::Faulted)?;
            b.observation_context(ShutdownObservationContextRecordV1 {
                context: row.context.clone(),
                invocation: row.invocation.clone(),
                reserved_bytes: row.charge,
                host_handle_retained: row.handle.strong_count() != 0,
            })?;
        }
        if bytes > self.retention_usage().reserved {
            return Err(CoordinatorError::Faulted);
        }
        self.record_shutdown_local_owner(
            b,
            ShutdownOwnerRoleV1::InvocationObservers,
            LocalOwnerSnapshot {
                configured: registry.issued != 0,
                issued: Some(registry.issued),
                retained,
                held: None,
                reserved: None,
                limit: None,
                additional_reserved: Some(bytes),
            },
        )
    }
}
