//! Closed first-party combined-source ownership ports, not public pins/providers.
use super::*;
#[derive(Clone)]
pub(in crate::coordinator) struct AnnotationSourceHoldId {
    owner: Arc<()>,
    slot: usize,
}
impl AnnotationSourceHoldId {
    pub(in crate::coordinator) fn shutdown_slot(&self) -> usize {
        self.slot
    }
}
pub(super) struct Hold {
    context: usize,
}
impl ContextPool {
    pub(in crate::coordinator) fn document_pins(
        &self,
        document: &DocumentId,
    ) -> Result<(usize, usize), CoordinatorError> {
        for h in self.holds.iter().flatten() {
            if self.rows.get(h.context).and_then(Option::as_ref).is_none() {
                return Err(CoordinatorError::Faulted);
            }
        }
        let mut contexts = 0;
        let mut holds = 0usize;
        for (i, row) in self
            .rows
            .iter()
            .enumerate()
            .filter_map(|(i, r)| r.as_ref().map(|r| (i, r)))
        {
            let actual = self
                .actual_holds(i)
                .map_err(|_| CoordinatorError::Faulted)?;
            if actual != row.holds {
                return Err(CoordinatorError::Faulted);
            }
            if row.scope.document == *document {
                contexts += 1;
                holds = holds.checked_add(actual).ok_or(CoordinatorError::Faulted)?;
            }
        }
        Ok((contexts, holds))
    }
    pub(super) fn actual_holds(&self, context: usize) -> Result<usize, Error> {
        let mut count = 0usize;
        for hold in self.holds.iter().flatten() {
            if self
                .rows
                .get(hold.context)
                .and_then(Option::as_ref)
                .is_none()
            {
                return Err(Error::Faulted);
            }
            if hold.context == context {
                count = count.checked_add(1).ok_or(Error::Faulted)?;
            }
        }
        Ok(count)
    }
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn annotation_context_domain(&self) -> Result<(&str, &str), Error> {
        let p = self.annotation_context_pool()?;
        self.retention.validate(&p.reservation).map_err(parent)?;
        Ok((&p.authority, &p.engine))
    }
    pub(in crate::coordinator) fn annotation_search_snapshot<'a>(
        &'a self,
        id: &AnnotationReadContextId,
        s: &SourceReadScopeV1,
    ) -> Result<source_context::SearchSnapshot<'a>, Error> {
        let p = self.annotation_context_pool()?;
        if !valid_scope(s, p) {
            return Err(Error::Unavailable);
        }
        let row = self.annotation_context_row(id, s)?;
        if row.stopped {
            return Err(Error::Unavailable);
        }
        let (source, version, profile) = row.value.source.as_ref().ok_or(Error::Unavailable)?;
        Ok(source_context::SearchSnapshot {
            source,
            version,
            max_root_bytes: profile.max_source_bytes,
            max_request_bytes: profile.max_request_bytes.min(p.limits.max_request_bytes),
            max_reply_bytes: profile.max_reply_bytes.min(p.limits.max_reply_bytes),
        })
    }
    pub(in crate::coordinator) fn hold_annotation_search_source(
        &mut self,
        id: &AnnotationReadContextId,
        s: &SourceReadScopeV1,
        f: &TrustedAnnotationContextFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<AnnotationSourceHoldId, Error> {
        self.annotation_search_snapshot(id, s)?;
        self.check_annotation_context(id, s, f, r)?;
        self.ensure_admission_unfenced()
            .map_err(source_context::admission_error)?;
        let p = self.annotation_context_pool()?;
        let row = self.annotation_context_row(id, s)?;
        if p.actual_holds(id.slot)? != row.holds {
            return Err(Error::Faulted);
        }
        let next = row.holds.checked_add(1).ok_or(Error::Faulted)?;
        if p.issued()? >= p.limits.max_issued
            || p.holds.iter().flatten().count() >= p.limits.max_holds
        {
            return Err(Error::Limit);
        }
        if p.holds.capacity() < p.limits.max_issued {
            return Err(Error::Faulted);
        }
        let InstalledState::Active(p) = &mut self.annotation_contexts else {
            return Err(Error::Faulted);
        };
        let hold = AnnotationSourceHoldId {
            owner: self.namespace.clone(),
            slot: p.holds.len(),
        };
        p.holds.push(Some(Hold { context: id.slot }));
        p.rows[id.slot].as_mut().ok_or(Error::Faulted)?.holds = next;
        Ok(hold)
    }
    pub(in crate::coordinator) fn validate_annotation_source_hold(
        &self,
        hold: &AnnotationSourceHoldId,
        id: &AnnotationReadContextId,
        s: &SourceReadScopeV1,
    ) -> Result<(), Error> {
        let p = self.annotation_context_pool()?;
        let row = self.annotation_context_row(id, s)?;
        self.retention.validate(&p.reservation).map_err(parent)?;
        if !Arc::ptr_eq(&hold.owner, &self.namespace)
            || p.holds
                .get(hold.slot)
                .and_then(Option::as_ref)
                .is_none_or(|v| v.context != id.slot)
            || p.actual_holds(id.slot)? != row.holds
        {
            return Err(Error::Faulted);
        }
        Ok(())
    }
    pub(in crate::coordinator) fn release_annotation_source_hold(
        &mut self,
        hold: &AnnotationSourceHoldId,
        f: &SourceReadContextControlV1<'_>,
    ) -> Result<(), Error> {
        if !discharge(f) || !Arc::ptr_eq(&hold.owner, &self.namespace) {
            return Err(Error::Unavailable);
        }
        let p = self.annotation_context_pool()?;
        let context = p
            .holds
            .get(hold.slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)?
            .context;
        let id = AnnotationReadContextId {
            owner: self.namespace.clone(),
            slot: context,
        };
        self.validate_annotation_source_hold(hold, &id, f.scope)?;
        let row = self.annotation_context_row(&id, f.scope)?;
        let next = row.holds.checked_sub(1).ok_or(Error::Faulted)?;
        let InstalledState::Active(p) = &mut self.annotation_contexts else {
            return Err(Error::Faulted);
        };
        p.rows[context].as_mut().ok_or(Error::Faulted)?.holds = next;
        p.holds[hold.slot] = None;
        Ok(())
    }
}
