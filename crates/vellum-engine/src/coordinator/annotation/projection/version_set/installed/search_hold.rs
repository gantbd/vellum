//! Closed first-party search ports. Each member result holds its entire parent.
use super::*;
#[cfg(test)]
mod tests;
#[derive(Clone)]
pub(in crate::coordinator) struct VersionSetSourceHoldId {
    owner: Arc<()>,
    slot: usize,
}
impl VersionSetSourceHoldId {
    pub(in crate::coordinator) fn shutdown_slot(&self) -> usize {
        self.slot
    }
}
pub(super) struct Hold {
    parent: usize,
    member: usize,
}
impl ContextPool {
    pub(in crate::coordinator) fn document_pins(
        &self,
        document: &DocumentId,
    ) -> Result<(usize, usize), CoordinatorError> {
        for h in self.holds.iter().flatten() {
            if self.rows.get(h.parent).and_then(Option::as_ref).is_none() {
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
            // A member search holds its whole coherent parent, not only the
            // searched member. Never split this barrier by per-member counting.
            if row
                .value
                .members
                .iter()
                .any(|m| m.scope.document == *document)
            {
                contexts += 1;
                holds = holds.checked_add(actual).ok_or(CoordinatorError::Faulted)?;
            }
        }
        Ok((contexts, holds))
    }
    pub(super) fn actual_holds(&self, parent: usize) -> Result<usize, Error> {
        let mut count = 0usize;
        for h in self.holds.iter().flatten() {
            let row = self
                .rows
                .get(h.parent)
                .and_then(Option::as_ref)
                .ok_or(Error::Faulted)?;
            let alias = self
                .aliases
                .get(h.member)
                .and_then(Option::as_ref)
                .ok_or(Error::Faulted)?;
            if alias.parent != h.parent
                || row
                    .value
                    .members
                    .get(alias.index)
                    .is_none_or(|m| m.value.source.is_none())
            {
                return Err(Error::Faulted);
            }
            if h.parent == parent {
                count = count.checked_add(1).ok_or(Error::Faulted)?;
            }
        }
        Ok(count)
    }
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn version_context_domain(&self) -> Result<(&str, &str), Error> {
        let p = self.version_context_pool()?;
        self.retention.validate(&p.reservation).map_err(parent)?;
        Ok((&p.authority, &p.engine))
    }
    pub(in crate::coordinator) fn version_search_parent(
        &self,
        id: &VersionSetMemberId,
    ) -> Result<VersionSetContextId, Error> {
        let slot = self.version_parent_slot(VersionSetControlTargetV1::Member(id))?;
        Ok(VersionSetContextId {
            owner: self.namespace.clone(),
            slot,
        })
    }
    pub(in crate::coordinator) fn version_search_snapshot<'a>(
        &'a self,
        id: &VersionSetMemberId,
        s: &SourceReadScopeV1,
    ) -> Result<source_context::SearchSnapshot<'a>, Error> {
        let parent = self.version_parent_slot(VersionSetControlTargetV1::Member(id))?;
        let p = self.version_context_pool()?;
        let row = p.rows[parent].as_ref().ok_or(Error::Faulted)?;
        if row.stopped {
            return Err(Error::Unavailable);
        }
        let alias = p.aliases[id.slot].as_ref().ok_or(Error::Faulted)?;
        let m = row.value.members.get(alias.index).ok_or(Error::Faulted)?;
        if &m.scope != s {
            return Err(Error::Unavailable);
        }
        let (source, version, profile) = m.value.source.as_ref().ok_or(Error::Unavailable)?;
        Ok(source_context::SearchSnapshot {
            source,
            version,
            max_root_bytes: profile.max_source_bytes,
            max_request_bytes: profile.max_request_bytes,
            max_reply_bytes: profile.max_reply_bytes.min(p.limits.max_reply_bytes),
        })
    }
    pub(in crate::coordinator) fn version_search_frame(
        &self,
        id: &VersionSetMemberId,
        s: &SourceReadScopeV1,
        f: &TrustedVersionSetContextFactsV1<'_>,
    ) -> Result<(usize, usize), Error> {
        let (row, m) = self.version_read_member(id, f)?;
        if &m.scope != s || m.value.source.is_none() {
            return Err(Error::Unavailable);
        }
        Ok((
            row.value.context_facts_request_bytes()?,
            self.version_context_pool()?.limits.max_request_bytes,
        ))
    }
    pub(in crate::coordinator) fn version_annotation_frame(
        &self,
        id: &VersionSetMemberId,
        s: &SourceReadScopeV1,
        f: &TrustedVersionSetContextFactsV1<'_>,
    ) -> Result<(usize, usize), Error> {
        let (row, member) = self.version_read_member(id, f)?;
        if &member.scope != s || !member.value.classes.includes_annotations() {
            return Err(Error::Unavailable);
        }
        Ok((
            row.value.context_facts_request_bytes()?,
            self.version_context_pool()?.limits.max_request_bytes,
        ))
    }
    pub(in crate::coordinator) fn check_version_search(
        &mut self,
        id: &VersionSetMemberId,
        s: &SourceReadScopeV1,
        f: &TrustedVersionSetContextFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        self.version_search_frame(id, s, f)?;
        self.check_version_context(VersionSetControlTargetV1::Member(id), f, r)
    }
    pub(in crate::coordinator) fn hold_version_search(
        &mut self,
        id: &VersionSetMemberId,
        s: &SourceReadScopeV1,
        f: &TrustedVersionSetContextFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<VersionSetSourceHoldId, Error> {
        self.check_version_search(id, s, f, r)?;
        self.ensure_admission_unfenced()
            .map_err(source_context::admission_error)?;
        let parent = self.version_parent_slot(VersionSetControlTargetV1::Member(id))?;
        let p = self.version_context_pool()?;
        let row = p.rows[parent].as_ref().ok_or(Error::Faulted)?;
        if p.actual_holds(parent)? != row.holds {
            return Err(Error::Faulted);
        }
        let next = row.holds.checked_add(1).ok_or(Error::Faulted)?;
        if p.holds.iter().flatten().count() >= p.limits.max_holds
            || p.issued()? >= p.limits.max_issued
            || std::mem::size_of::<VersionSetSourceHoldId>() > p.limits.max_selector_bytes
        {
            return Err(Error::Limit);
        }
        if p.holds.capacity() < p.limits.max_issued {
            return Err(Error::Faulted);
        }
        let VersionSetContextState::Active(p) = &mut self.version_set_contexts else {
            return Err(Error::Faulted);
        };
        let hold = VersionSetSourceHoldId {
            owner: self.namespace.clone(),
            slot: p.holds.len(),
        };
        p.holds.push(Some(Hold {
            parent,
            member: id.slot,
        }));
        p.rows[parent].as_mut().ok_or(Error::Faulted)?.holds = next;
        Ok(hold)
    }
    pub(in crate::coordinator) fn validate_version_search_hold(
        &self,
        h: &VersionSetSourceHoldId,
        id: &VersionSetMemberId,
        s: &SourceReadScopeV1,
    ) -> Result<(), Error> {
        let parent = self.version_parent_slot(VersionSetControlTargetV1::Member(id))?;
        let p = self.version_context_pool()?;
        self.retention
            .validate(&p.reservation)
            .map_err(super::parent)?;
        let row = p.rows[parent].as_ref().ok_or(Error::Faulted)?;
        let alias = p.aliases[id.slot].as_ref().ok_or(Error::Faulted)?;
        if row
            .value
            .members
            .get(alias.index)
            .is_none_or(|m| &m.scope != s)
        {
            return Err(Error::Unavailable);
        }
        if !Arc::ptr_eq(&h.owner, &self.namespace)
            || p.holds
                .get(h.slot)
                .and_then(Option::as_ref)
                .is_none_or(|h| h.parent != parent || h.member != id.slot)
            || p.actual_holds(parent)? != row.holds
        {
            return Err(Error::Faulted);
        }
        Ok(())
    }
    fn release_actual_version_hold(&mut self, h: &VersionSetSourceHoldId) -> Result<(), Error> {
        if !Arc::ptr_eq(&h.owner, &self.namespace) {
            return Err(Error::Unavailable);
        }
        let p = self.version_context_pool()?;
        self.retention.validate(&p.reservation).map_err(parent)?;
        let hold = p
            .holds
            .get(h.slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)?;
        let parent = hold.parent;
        let row = p
            .rows
            .get(parent)
            .and_then(Option::as_ref)
            .ok_or(Error::Faulted)?;
        if p.actual_holds(parent)? != row.holds {
            return Err(Error::Faulted);
        }
        let next = row.holds.checked_sub(1).ok_or(Error::Faulted)?;
        let VersionSetContextState::Active(p) = &mut self.version_set_contexts else {
            return Err(Error::Faulted);
        };
        p.holds[h.slot] = None;
        p.rows[parent].as_mut().ok_or(Error::Faulted)?.holds = next;
        Ok(())
    }
    pub(in crate::coordinator) fn release_version_search_hold(
        &mut self,
        h: &VersionSetSourceHoldId,
        controls: &[SourceReadContextControlV1<'_>],
    ) -> Result<(), Error> {
        if !Arc::ptr_eq(&h.owner, &self.namespace) {
            return Err(Error::Unavailable);
        }
        let p = self.version_context_pool()?;
        let hold = p
            .holds
            .get(h.slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)?;
        let row = p
            .rows
            .get(hold.parent)
            .and_then(Option::as_ref)
            .ok_or(Error::Faulted)?;
        if !row.value.controls_match(controls) {
            return Err(Error::Unavailable);
        }
        self.release_actual_version_hold(h)
    }
    /// Only the local unpublished-result error path owns this capability.
    pub(in crate::coordinator) fn discard_unpublished_version_search_hold(
        &mut self,
        h: &VersionSetSourceHoldId,
    ) -> Result<(), Error> {
        self.release_actual_version_hold(h)
    }
}
