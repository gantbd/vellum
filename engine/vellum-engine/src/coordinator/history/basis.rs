//! Opaque trusted-owner history metadata footprint, not a recipient context/grant.
use super::*;
mod shutdown_inventory;
use std::mem::size_of;
#[cfg(test)]
mod tests;
#[derive(Clone, PartialEq, Eq)]
pub enum HistoryBasisScopeV1 {
    Source(DocumentId),
    Annotation(AnnotationId),
}
impl std::fmt::Debug for HistoryBasisScopeV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HistoryBasisScopeV1(<opaque>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistoryBasisLimitsV1 {
    /// Bounds selected and actual retained group metadata; unrelated pressure may refuse.
    pub max_groups: usize,
    /// Bounds root scope metadata and the complete relevant participant union.
    pub max_scopes: usize,
    pub max_participants: usize,
    pub max_basis_bytes: usize,
}
/// Trusted-owner accounting, not permitted participant/count disclosure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistoryBasisUsageV1 {
    pub groups: usize,
    pub scopes: usize,
    pub frontier_entries: usize,
    pub logical_bytes: usize,
}
pub(super) struct GroupBasis {
    id: HistoryGroupId,
    pub(super) valid: bool,
    pub(super) scopes: Vec<HistoryBasisScopeV1>,
}
#[derive(PartialEq, Eq)]
pub(super) struct Stacks {
    pub(super) undo: Vec<HistoryGroupId>,
    pub(super) redo: Vec<HistoryGroupId>,
    pub(super) undo_lost: bool,
    pub(super) redo_lost: bool,
}
struct ScopeBasis {
    scope: HistoryBasisScopeV1,
    stacks: Option<Stacks>,
}
pub struct HistoryBasisV1 {
    owner: Arc<()>,
    limits: HistoryBasisLimitsV1,
    groups: Vec<GroupBasis>,
    scopes: Vec<ScopeBasis>,
    usage: HistoryBasisUsageV1,
}
impl std::fmt::Debug for HistoryBasisV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HistoryBasisV1(<opaque>)")
    }
}
impl HistoryBasisV1 {
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        let refers = |s: &HistoryBasisScopeV1| match s {
            HistoryBasisScopeV1::Source(id) => id == document,
            HistoryBasisScopeV1::Annotation(id) => &id.document == document,
        };
        self.scopes.iter().any(|s| refers(&s.scope))
            || self.groups.iter().any(|g| g.scopes.iter().any(refers))
    }
    pub(super) fn scope_stacks(&self, scope: &HistoryBasisScopeV1) -> Option<&Stacks> {
        self.scopes
            .iter()
            .find(|s| s.scope == *scope)?
            .stacks
            .as_ref()
    }
    pub(super) fn selected_group(&self, id: &HistoryGroupId) -> Option<&GroupBasis> {
        self.groups.iter().find(|g| g.id == *id)
    }
    /// No source/body bytes or inverse journals are retained or pinned.
    pub fn usage(&self) -> HistoryBasisUsageV1 {
        self.usage
    }
}
impl HistoryBasisScopeV1 {
    fn key(&self) -> (usize, usize, usize) {
        match self {
            Self::Source(id) => (0, id.slot, 0),
            Self::Annotation(id) => (1, id.document.slot, id.slot),
        }
    }
    pub(super) fn owned(&self, owner: &SourceCoordinator) -> bool {
        match self {
            Self::Source(id) => Arc::ptr_eq(&id.namespace, &owner.namespace),
            Self::Annotation(id) => {
                Arc::ptr_eq(&id.namespace, &owner.namespace)
                    && Arc::ptr_eq(&id.document.namespace, &owner.namespace)
            }
        }
    }
}
fn participants(group: &Group) -> impl Iterator<Item = HistoryBasisScopeV1> + '_ {
    group
        .members
        .iter()
        .map(|m| HistoryBasisScopeV1::Source(m.document.clone()))
        .chain(
            group
                .annotations
                .iter()
                .map(|m| HistoryBasisScopeV1::Annotation(m.id.clone())),
        )
}
fn size_participants(group: &Group) -> Result<usize, HistoryError> {
    group
        .members
        .len()
        .checked_add(group.annotations.len())
        .ok_or(HistoryError::Limit)
}
fn unique_group<'a>(
    owner: &'a SourceCoordinator,
    id: &HistoryGroupId,
) -> Result<Option<&'a Group>, HistoryError> {
    let mut matches = owner.history.groups.iter().filter(|group| group.id == *id);
    let found = matches.next();
    if matches.next().is_some() {
        return Err(HistoryError::Faulted);
    }
    Ok(found)
}
fn frontiers<'a>(
    owner: &'a SourceCoordinator,
    scope: &HistoryBasisScopeV1,
) -> Option<&'a Frontiers> {
    match scope {
        // Retained metadata presence is distinct from source/reversal liveness.
        HistoryBasisScopeV1::Source(id) => owner.retained_document(id).ok().map(|d| &d.history),
        HistoryBasisScopeV1::Annotation(id) => owner
            .annotations
            .records
            .get(id.slot)
            .and_then(Option::as_ref)
            .filter(|r| r.value.id == *id)
            .map(|r| &r.history),
    }
}
fn pressure(owner: &SourceCoordinator, limits: HistoryBasisLimitsV1) -> Result<(), HistoryError> {
    if owner.history.limits.is_none() {
        return Err(HistoryError::Unavailable);
    }
    let scopes = owner
        .documents
        .iter()
        .flatten()
        .count()
        .checked_add(owner.annotations.records.iter().flatten().count())
        .ok_or(HistoryError::Limit)?;
    if owner.history.groups.len() > limits.max_groups
        || scopes > limits.max_scopes
        || owner
            .history
            .groups
            .iter()
            .any(|g| !size_participants(g).is_ok_and(|n| n <= limits.max_participants))
    {
        return Err(HistoryError::Limit);
    }
    Ok(())
}
fn add(bytes: &mut usize, n: usize, maximum: usize) -> Result<(), HistoryError> {
    *bytes = bytes
        .checked_add(n)
        .filter(|n| *n <= maximum)
        .ok_or(HistoryError::Limit)?;
    Ok(())
}
fn rows<T>(bytes: &mut usize, count: usize, maximum: usize) -> Result<(), HistoryError> {
    add(
        bytes,
        count
            .checked_mul(size_of::<T>())
            .ok_or(HistoryError::Limit)?,
        maximum,
    )
}
fn reserve<T>(v: &mut Vec<T>, count: usize) -> Result<(), HistoryError> {
    v.try_reserve_exact(count)
        .map_err(|_| HistoryError::Capacity)
}
fn record<T>(r: &mut Recorder, boundary: Boundary, result: &Result<T, HistoryError>) {
    capture(r, boundary, result);
}
impl SourceCoordinator {
    /// Privileged metadata capture, not F04 issuance, alias projection or inverse retention.
    pub fn capture_history_basis_v1(
        &mut self,
        groups: &[HistoryGroupId],
        scopes: &[HistoryBasisScopeV1],
        limits: HistoryBasisLimitsV1,
        r: &mut Recorder,
    ) -> Result<HistoryBasisV1, HistoryError> {
        let result = self.build_history_basis(groups, scopes, limits);
        if matches!(result, Err(HistoryError::Faulted)) {
            self.faulted = true;
        }
        record(r, Boundary::HistoryBasisCapture, &result);
        result
    }
    fn build_history_basis(
        &self,
        selected: &[HistoryGroupId],
        requested: &[HistoryBasisScopeV1],
        limits: HistoryBasisLimitsV1,
    ) -> Result<HistoryBasisV1, HistoryError> {
        self.ensure_operational().map_err(owner_error)?;
        pressure(self, limits)?;
        if selected.len() > limits.max_groups || requested.len() > limits.max_scopes {
            return Err(HistoryError::Limit);
        }
        if selected
            .iter()
            .enumerate()
            .any(|(i, id)| selected[..i].contains(id))
            || requested
                .iter()
                .enumerate()
                .any(|(i, s)| !s.owned(self) || requested[..i].contains(s))
        {
            return Err(HistoryError::Unavailable);
        }
        // Charge output headers and a traversal/union workspace before ownership copies.
        let mut bytes = size_of::<HistoryBasisV1>();
        add(
            &mut bytes,
            size_of::<Vec<HistoryBasisScopeV1>>(),
            limits.max_basis_bytes,
        )?;
        rows::<GroupBasis>(&mut bytes, selected.len(), limits.max_basis_bytes)?;
        rows::<HistoryBasisScopeV1>(&mut bytes, requested.len(), limits.max_basis_bytes)?;
        let mut union = Vec::new();
        reserve(&mut union, requested.len())?;
        union.extend(requested.iter().cloned());
        let mut groups = Vec::new();
        reserve(&mut groups, selected.len())?;
        for id in selected {
            let group = unique_group(self, id)?.ok_or(HistoryError::Unavailable)?;
            let count = size_participants(group)?;
            if count == 0 {
                return Err(HistoryError::Faulted);
            }
            rows::<HistoryBasisScopeV1>(&mut bytes, count, limits.max_basis_bytes)?;
            let mut members = Vec::new();
            reserve(&mut members, count)?;
            for scope in participants(group) {
                if !scope.owned(self) || members.contains(&scope) {
                    return Err(HistoryError::Faulted);
                }
                if !union.contains(&scope) {
                    if union.len() >= limits.max_scopes {
                        return Err(HistoryError::Limit);
                    }
                    rows::<HistoryBasisScopeV1>(&mut bytes, 1, limits.max_basis_bytes)?;
                    reserve(&mut union, 1)?;
                    union.push(scope.clone());
                }
                members.push(scope);
            }
            members.sort_unstable_by_key(HistoryBasisScopeV1::key);
            groups.push(GroupBasis {
                id: id.clone(),
                valid: group.state != GroupState::Invalidated,
                scopes: members,
            });
        }
        groups.sort_unstable_by_key(|g| g.id.original.generation);
        union.sort_unstable_by_key(HistoryBasisScopeV1::key);
        rows::<ScopeBasis>(&mut bytes, union.len(), limits.max_basis_bytes)?;
        let mut scopes = Vec::new();
        reserve(&mut scopes, union.len())?;
        let mut frontier_entries = 0usize;
        for scope in union {
            let stacks = match frontiers(self, &scope) {
                None => None,
                Some(f) => {
                    let count = f
                        .undo
                        .len()
                        .checked_add(f.redo.len())
                        .ok_or(HistoryError::Limit)?;
                    frontier_entries = frontier_entries
                        .checked_add(count)
                        .ok_or(HistoryError::Limit)?;
                    rows::<HistoryGroupId>(&mut bytes, count, limits.max_basis_bytes)?;
                    Some(Stacks {
                        undo: copy_vec(&f.undo)?,
                        redo: copy_vec(&f.redo)?,
                        undo_lost: f.undo_lost,
                        redo_lost: f.redo_lost,
                    })
                }
            };
            scopes.push(ScopeBasis { scope, stacks });
        }
        let usage = HistoryBasisUsageV1 {
            groups: groups.len(),
            scopes: scopes.len(),
            frontier_entries,
            logical_bytes: bytes,
        };
        Ok(HistoryBasisV1 {
            owner: self.namespace.clone(),
            limits,
            groups,
            scopes,
            usage,
        })
    }
    /// Endpoint metadata equality only. A future retained context must latch observed loss.
    pub fn history_basis_is_current_v1(
        &mut self,
        basis: &HistoryBasisV1,
        r: &mut Recorder,
    ) -> Result<bool, HistoryError> {
        let result = basis.compare(self);
        if matches!(result, Err(HistoryError::Faulted)) {
            self.faulted = true;
        }
        r.record(Observation {
            boundary: Boundary::HistoryBasisValidation,
            disposition: match result {
                Ok(true) => Disposition::Completed,
                Ok(false) => Disposition::Refused,
                Err(HistoryError::Faulted) => Disposition::Unknown,
                Err(_) => Disposition::Refused,
            },
            service_time: None,
        });
        result
    }
}
impl HistoryBasisV1 {
    pub(super) fn compare(&self, owner: &SourceCoordinator) -> Result<bool, HistoryError> {
        owner.ensure_operational().map_err(owner_error)?;
        if !Arc::ptr_eq(&owner.namespace, &self.owner) {
            return Err(HistoryError::Unavailable);
        }
        pressure(owner, self.limits)?;
        for captured in &self.groups {
            let Some(current) = unique_group(owner, &captured.id)? else {
                return Ok(false);
            };
            if captured.valid != (current.state != GroupState::Invalidated)
                || size_participants(current)? != captured.scopes.len()
                || !captured.scopes.iter().all(|s| {
                    current.contains(&match s {
                        HistoryBasisScopeV1::Source(id) => Scope::Source(id.clone()),
                        HistoryBasisScopeV1::Annotation(id) => Scope::Annotation(id.clone()),
                    })
                })
            {
                return Ok(false);
            }
        }
        for captured in &self.scopes {
            match (&captured.stacks, frontiers(owner, &captured.scope)) {
                (None, None) => (),
                (Some(old), Some(now))
                    if old.undo == now.undo
                        && old.redo == now.redo
                        && old.undo_lost == now.undo_lost
                        && old.redo_lost == now.redo_lost => {}
                _ => return Ok(false),
            }
        }
        Ok(true)
    }
}
