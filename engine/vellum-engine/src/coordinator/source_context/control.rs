use super::*;
pub(super) fn discharge(f: &SourceReadContextControlV1<'_>) -> bool {
    [
        f.owner_control_allowed,
        f.retention_released,
        f.delivery_drained,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
impl SourceCoordinator {
    pub fn stop_source_read_context(
        &mut self,
        id: &SourceReadContextId,
        f: &SourceReadContextControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        self.stop_source_read_context_result(id, f, r).is_ok()
    }
    pub(in crate::coordinator) fn stop_source_read_context_result(
        &mut self,
        id: &SourceReadContextId,
        f: &SourceReadContextControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = (|| {
            if f.owner_control_allowed != Some(true) {
                return Err(Error::Unavailable);
            }
            self.source_context_row(id, f.scope)?;
            let State::Active(p) = &mut self.source_contexts else {
                return Err(Error::Unavailable);
            };
            p.close_context(id.slot)?;
            Ok(())
        })();
        if result == Err(Error::Faulted) {
            self.faulted = true;
        }
        observe(r, Boundary::SourceContextStop, &result);
        result
    }
    /// Privileged internal lifetime port, not an additional public pin/read grant.
    pub fn hold_source_read_context(
        &mut self,
        id: &SourceReadContextId,
        s: &SourceReadScopeV1,
        f: &SourceReadHoldFactsV1,
        r: &mut Recorder,
    ) -> Option<SourceReadHoldId> {
        let result = (|| {
            if self.faulted {
                return Err(Error::Faulted);
            }
            self.ensure_admission_unfenced().map_err(admission_error)?;
            let row = self.source_context_row(id, s)?;
            if row.closed
                || self.document(&s.document).is_err()
                || [f.owner_control_allowed, f.retained, f.resources_ready]
                    .into_iter()
                    .any(|v| v != Some(true))
            {
                return Err(Error::Unavailable);
            }
            if row.source.is_none() {
                return Err(Error::Faulted);
            }
            let next = row.holds.checked_add(1).ok_or(Error::Faulted)?;
            let p = self.source_pool()?;
            if p.actual_holds(id.slot)? != row.holds {
                return Err(Error::Faulted);
            }
            p.can_issue()?;
            self.retention.validate(&p.reservation).map_err(parent)?;
            if held(&p.holds) >= p.limits.max_holds {
                return Err(Error::Limit);
            }
            let State::Active(p) = &mut self.source_contexts else {
                return Err(Error::Faulted);
            };
            let hold = SourceReadHoldId {
                owner: self.namespace.clone(),
                slot: p.holds.len(),
            };
            p.holds.push(Some(Hold { context: id.slot }));
            p.rows[id.slot].as_mut().ok_or(Error::Faulted)?.holds = next;
            Ok(hold)
        })();
        if result == Err(Error::Faulted) {
            self.faulted = true;
        }
        observe(r, Boundary::SourceContextHold, &result);
        result.ok()
    }
    pub fn release_source_read_hold(
        &mut self,
        id: &SourceReadHoldId,
        f: &SourceReadContextControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let result = (|| {
            if !Arc::ptr_eq(&id.owner, &self.namespace) || !discharge(f) {
                return Err(Error::Unavailable);
            }
            let p = self.source_pool()?;
            let hold = p
                .holds
                .get(id.slot)
                .and_then(Option::as_ref)
                .ok_or(Error::Unavailable)?;
            let context = hold.context;
            let row = p
                .rows
                .get(context)
                .and_then(Option::as_ref)
                .ok_or(Error::Faulted)?;
            if row.scope != *f.scope {
                return Err(Error::Unavailable);
            }
            if p.actual_holds(context)? != row.holds {
                return Err(Error::Faulted);
            }
            let remaining = row.holds.checked_sub(1).ok_or(Error::Faulted)?;
            self.retention.validate(&p.reservation).map_err(parent)?;
            let State::Active(p) = &mut self.source_contexts else {
                return Err(Error::Faulted);
            };
            let row = p.rows[context].as_mut().ok_or(Error::Faulted)?;
            row.holds = remaining;
            if row.closed && remaining == 0 {
                row.source = None;
            }
            p.holds[id.slot] = None;
            Ok(())
        })();
        if result == Err(Error::Faulted) {
            self.faulted = true;
        }
        observe(r, Boundary::SourceContextRelease, &result);
        result.is_ok()
    }
    pub fn retire_source_read_context(
        &mut self,
        id: &SourceReadContextId,
        f: &SourceReadContextControlV1<'_>,
        r: &mut Recorder,
    ) -> bool {
        let result = (|| {
            if !discharge(f) {
                return Err(Error::Unavailable);
            }
            let row = self.source_context_row(id, f.scope)?;
            if !row.closed || row.holds != 0 {
                return Err(Error::Unavailable);
            }
            let p = self.source_pool()?;
            if p.actual_holds(id.slot)? != row.holds {
                return Err(Error::Faulted);
            }
            self.retention.validate(&p.reservation).map_err(parent)?;
            let State::Active(p) = &mut self.source_contexts else {
                return Err(Error::Faulted);
            };
            p.rows[id.slot] = None;
            Ok(())
        })();
        if result == Err(Error::Faulted) {
            self.faulted = true;
        }
        observe(r, Boundary::SourceContextRetirement, &result);
        result.is_ok()
    }
    pub fn retire_source_read_contexts(
        &mut self,
        f: &SourceReadPoolControlV1,
        r: &mut Recorder,
    ) -> Result<bool, Error> {
        let result = (|| {
            if [
                f.owner_control_allowed,
                f.retention_released,
                f.delivery_drained,
            ]
            .into_iter()
            .any(|v| v != Some(true))
            {
                return Err(Error::Unavailable);
            }
            let State::Active(p) = &mut self.source_contexts else {
                return Ok(false);
            };
            if live(&p.rows) != 0 || held(&p.holds) != 0 {
                return Err(Error::Unavailable);
            }
            let issued = p.issued()?;
            self.retention.release(&mut p.reservation).map_err(parent)?;
            self.source_contexts = State::Closed { issued };
            Ok(true)
        })();
        if result == Err(Error::Faulted) {
            self.faulted = true;
        }
        observe(r, Boundary::SourceContextPoolRetirement, &result);
        result
    }
}
