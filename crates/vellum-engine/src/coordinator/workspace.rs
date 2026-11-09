//! Logical membership containers, not ownership, paths, focus or settings association.
use super::*;
mod root_accounting;
mod shutdown_inventory;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkspaceLimits {
    pub max_issued: usize,
    pub max_live: usize,
    pub max_memberships: usize,
    pub max_work: usize,
}
#[derive(Clone)]
pub struct WorkspaceId {
    owner: Arc<()>,
    pub(super) slot: usize,
}
impl PartialEq for WorkspaceId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for WorkspaceId {}
impl std::fmt::Debug for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WorkspaceId(<opaque>)")
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceContext {
    pub workspace: WorkspaceId,
    pub generation: u64,
}
pub struct WorkspaceSnapshot {
    pub context: WorkspaceContext,
    pub members: Vec<DocumentId>,
}
impl std::fmt::Debug for WorkspaceSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WorkspaceSnapshot(<redacted>)")
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct WorkspaceUpdate {
    pub context: WorkspaceContext,
    pub changed: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkspaceUsage {
    pub issued_lifetime: usize,
    pub live: usize,
    pub memberships: usize,
}
#[derive(Debug)]
struct Workspace {
    generation: u64,
    members: Vec<DocumentId>,
}
enum WorkspaceRelationChange {
    Create(WorkspaceId),
    Retire(WorkspaceContext),
}
pub(super) struct WorkspaceRelationStage {
    owner: Arc<()>,
    before: u64,
    publication: u64,
    profile: WorkspaceLimits,
    live: usize,
    memberships: usize,
    change: WorkspaceRelationChange,
}
impl WorkspaceRelationStage {
    pub(super) fn receipt(&self) -> RelationReceiptV1 {
        match &self.change {
            WorkspaceRelationChange::Create(id) => RelationReceiptV1::WorkspaceCreated(id.clone()),
            WorkspaceRelationChange::Retire(context) => {
                RelationReceiptV1::WorkspaceRetired(context.workspace.clone())
            }
        }
    }
    pub(super) fn install(
        self,
        owner: &mut SourceCoordinator,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        if !Arc::ptr_eq(&owner.namespace, &self.owner)
            || owner.generation != self.before
            || owner.workspaces.limits != Some(self.profile)
        {
            return Err(CoordinatorError::Faulted);
        }
        if owner.next_publication_generation(None)? != self.publication {
            return Err(CoordinatorError::Faulted);
        }
        match self.change {
            WorkspaceRelationChange::Create(id) => {
                owner.validate_workspace_relation(&RelationCommandV1::CreateWorkspace, r)?;
                if id.slot != owner.workspaces.rows.len()
                    || !Arc::ptr_eq(&id.owner, &owner.namespace)
                    || owner.workspaces.rows.capacity() <= owner.workspaces.rows.len()
                    || owner.workspaces.live.checked_add(1) != Some(self.live)
                    || owner.workspaces.memberships != self.memberships
                {
                    return Err(CoordinatorError::Faulted);
                }
                owner.workspaces.rows.push(Some(Workspace {
                    generation: 1,
                    members: Vec::new(),
                }));
            }
            WorkspaceRelationChange::Retire(context) => {
                owner.validate_workspace_relation(
                    &RelationCommandV1::RetireWorkspace {
                        expected: context.clone(),
                    },
                    r,
                )?;
                let row = owner.workspace_context(&context)?;
                if owner.workspaces.live.checked_sub(1) != Some(self.live)
                    || owner.workspaces.memberships.checked_sub(row.members.len())
                        != Some(self.memberships)
                {
                    return Err(CoordinatorError::Faulted);
                }
                owner.workspaces.rows[context.workspace.slot] = None;
            }
        }
        owner.workspaces.live = self.live;
        owner.workspaces.memberships = self.memberships;
        owner.generation = self.publication;
        Ok(())
    }
}
#[derive(Debug, Default)]
pub(super) struct WorkspaceState {
    limits: Option<WorkspaceLimits>,
    rows: Vec<Option<Workspace>>,
    live: usize,
    memberships: usize,
}
impl WorkspaceState {
    pub(super) fn document_pins(&self, document: &DocumentId) -> Result<usize, CoordinatorError> {
        // This census is a cleanup/diagnostic scan, never a per-keystroke gauge.
        let mut live = 0usize;
        let mut memberships = 0usize;
        let mut pins = 0usize;
        for row in self.rows.iter().flatten() {
            live = live.checked_add(1).ok_or(CoordinatorError::Faulted)?;
            memberships = memberships
                .checked_add(row.members.len())
                .ok_or(CoordinatorError::Faulted)?;
            let count = row
                .members
                .iter()
                .filter(|member| *member == document)
                .count();
            if count > 1 {
                return Err(CoordinatorError::Faulted);
            }
            pins = pins.checked_add(count).ok_or(CoordinatorError::Faulted)?;
        }
        if live != self.live || memberships != self.memberships {
            return Err(CoordinatorError::Faulted);
        }
        Ok(pins)
    }
    pub(super) fn profile(&self) -> Option<WorkspaceLimits> {
        self.limits
    }
}
pub(super) struct StagedWorkspaces {
    updates: Vec<(usize, Workspace)>,
    memberships: usize,
}
impl StagedWorkspaces {
    pub(super) fn install(self, owner: &mut SourceCoordinator) {
        for (slot, row) in self.updates {
            owner.workspaces.rows[slot] = Some(row);
        }
        owner.workspaces.memberships = self.memberships;
    }
}
fn step(work: &mut usize) -> Result<(), CoordinatorError> {
    *work = work.checked_sub(1).ok_or(CoordinatorError::Limit)?;
    Ok(())
}

impl SourceCoordinator {
    #[cfg(test)]
    pub(super) fn change_workspace_relation_profile_for_test(&mut self) {
        self.workspaces.limits.as_mut().unwrap().max_live += 1;
    }
    pub(super) fn workspace_relation_profile(&self) -> Option<WorkspaceLimits> {
        self.workspaces.limits
    }
    pub(super) fn validate_workspace_relation(
        &self,
        command: &RelationCommandV1,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let limits = self
            .workspaces
            .limits
            .ok_or(CoordinatorError::Unavailable)?;
        match command {
            RelationCommandV1::CreateWorkspace => {
                if self.workspaces.rows.len() >= limits.max_issued
                    || self.workspaces.live >= limits.max_live
                {
                    return Err(CoordinatorError::Limit);
                }
                self.check_root_growth(std::mem::size_of::<Option<Workspace>>(), r)?;
            }
            RelationCommandV1::RetireWorkspace { expected } => {
                let row = self.workspace_context(expected)?;
                if row.members.len() > limits.max_work {
                    return Err(CoordinatorError::Limit);
                }
                self.workspaces
                    .live
                    .checked_sub(1)
                    .ok_or(CoordinatorError::Faulted)?;
                self.workspaces
                    .memberships
                    .checked_sub(row.members.len())
                    .ok_or(CoordinatorError::Faulted)?;
            }
            RelationCommandV1::CreateView | RelationCommandV1::RetireView { .. } => {
                return Err(CoordinatorError::Faulted);
            }
        }
        self.next_publication_generation(None)?;
        Ok(())
    }
    pub(super) fn stage_workspace_relation(
        &mut self,
        command: &RelationCommandV1,
        r: &mut Recorder,
    ) -> Result<WorkspaceRelationStage, CoordinatorError> {
        self.validate_workspace_relation(command, r)?;
        let (change, live, memberships) = match command {
            RelationCommandV1::CreateWorkspace => {
                self.workspaces
                    .rows
                    .try_reserve(1)
                    .map_err(|_| CoordinatorError::Capacity)?;
                (
                    WorkspaceRelationChange::Create(WorkspaceId {
                        owner: self.namespace.clone(),
                        slot: self.workspaces.rows.len(),
                    }),
                    self.workspaces
                        .live
                        .checked_add(1)
                        .ok_or(CoordinatorError::Faulted)?,
                    self.workspaces.memberships,
                )
            }
            RelationCommandV1::RetireWorkspace { expected } => {
                let row = self.workspace_context(expected)?;
                (
                    WorkspaceRelationChange::Retire(expected.clone()),
                    self.workspaces
                        .live
                        .checked_sub(1)
                        .ok_or(CoordinatorError::Faulted)?,
                    self.workspaces
                        .memberships
                        .checked_sub(row.members.len())
                        .ok_or(CoordinatorError::Faulted)?,
                )
            }
            RelationCommandV1::CreateView | RelationCommandV1::RetireView { .. } => {
                return Err(CoordinatorError::Faulted);
            }
        };
        Ok(WorkspaceRelationStage {
            owner: self.namespace.clone(),
            before: self.generation,
            publication: self.next_publication_generation(None)?,
            profile: self.workspaces.limits.ok_or(CoordinatorError::Faulted)?,
            live,
            memberships,
            change,
        })
    }
    fn finish_workspace<T>(
        &mut self,
        r: &mut Recorder,
        boundary: Boundary,
        result: Result<T, CoordinatorError>,
    ) -> Result<T, CoordinatorError> {
        if matches!(result, Err(CoordinatorError::Faulted)) {
            self.faulted = true;
        }
        observe(r, boundary, &result);
        result
    }
    pub fn configure_workspaces(
        &mut self,
        limits: WorkspaceLimits,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let result = (|| {
            self.ensure_operational()?;
            if self.workspaces.limits.is_some() {
                return Err(CoordinatorError::Unavailable);
            }
            self.workspaces.limits = Some(limits);
            Ok(())
        })();
        self.finish_workspace(r, Boundary::WorkspaceConfiguration, result)
    }
    pub fn workspace_usage(&self) -> WorkspaceUsage {
        WorkspaceUsage {
            issued_lifetime: self.workspaces.rows.len(),
            live: self.workspaces.live,
            memberships: self.workspaces.memberships,
        }
    }
    fn workspace_row(&self, id: &WorkspaceId) -> Result<&Workspace, CoordinatorError> {
        if !Arc::ptr_eq(&self.namespace, &id.owner) {
            return Err(CoordinatorError::Unavailable);
        }
        self.workspaces
            .rows
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(CoordinatorError::Unavailable)
    }
    pub(super) fn workspace_generation(&self, id: &WorkspaceId) -> Result<u64, CoordinatorError> {
        self.workspace_row(id).map(|row| row.generation)
    }
    fn workspace_context(
        &self,
        context: &WorkspaceContext,
    ) -> Result<&Workspace, CoordinatorError> {
        let row = self.workspace_row(&context.workspace)?;
        if row.generation != context.generation {
            return Err(CoordinatorError::Stale);
        }
        Ok(row)
    }
    pub fn create_workspace(
        &mut self,
        r: &mut Recorder,
    ) -> Result<WorkspaceContext, CoordinatorError> {
        let result = (|| {
            self.ensure_operational()?;
            let stage = self.stage_workspace_relation(&RelationCommandV1::CreateWorkspace, r)?;
            let RelationReceiptV1::WorkspaceCreated(workspace) = stage.receipt() else {
                return Err(CoordinatorError::Faulted);
            };
            let context = WorkspaceContext {
                workspace,
                generation: 1,
            };
            stage.install(self, r)?;
            Ok(context)
        })();
        self.finish_workspace(r, Boundary::WorkspaceLifecycle, result)
    }
    pub fn workspace(
        &self,
        id: &WorkspaceId,
        r: &mut Recorder,
    ) -> Result<WorkspaceSnapshot, CoordinatorError> {
        let result = (|| {
            self.ensure_operational()?;
            let row = self.workspace_row(id)?;
            let limits = self
                .workspaces
                .limits
                .ok_or(CoordinatorError::Unavailable)?;
            if row.members.len() > limits.max_work {
                return Err(CoordinatorError::Limit);
            }
            let mut members = Vec::new();
            members
                .try_reserve_exact(row.members.len())
                .map_err(|_| CoordinatorError::Capacity)?;
            members.extend(row.members.iter().cloned());
            Ok(WorkspaceSnapshot {
                context: WorkspaceContext {
                    workspace: id.clone(),
                    generation: row.generation,
                },
                members,
            })
        })();
        observe(r, Boundary::WorkspaceQuery, &result);
        result
    }
    pub fn set_workspace_membership(
        &mut self,
        context: &WorkspaceContext,
        document: &DocumentId,
        present: bool,
        r: &mut Recorder,
    ) -> Result<WorkspaceUpdate, CoordinatorError> {
        self.set_workspace_membership_for_invocation(context, document, present, None, r)
    }
    pub(super) fn set_workspace_membership_for_invocation(
        &mut self,
        context: &WorkspaceContext,
        document: &DocumentId,
        present: bool,
        invocation: Option<&InvocationId>,
        r: &mut Recorder,
    ) -> Result<WorkspaceUpdate, CoordinatorError> {
        let timing = r.begin_service_timing();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.ensure_operational()?;
            self.document(document)?;
            let row = self.workspace_context(context)?;
            let limits = self
                .workspaces
                .limits
                .ok_or(CoordinatorError::Unavailable)?;
            let mut work = limits.max_work;
            let mut found = None;
            for (i, member) in row.members.iter().enumerate() {
                step(&mut work)?;
                if member == document {
                    found = Some(i);
                }
            }
            if found.is_some() == present {
                if let Some(invocation) = invocation {
                    self.stage_live_relation_candidate(
                        invocation,
                        invocation::live_relation::Candidate::Membership {
                            workspace: context,
                            document,
                            present,
                        },
                        false,
                    )
                    .map_err(|_| CoordinatorError::Faulted)?;
                }
                return Ok(WorkspaceUpdate {
                    context: context.clone(),
                    changed: false,
                });
            }
            let generation = row
                .generation
                .checked_add(1)
                .ok_or(CoordinatorError::VersionExhausted)?;
            let membership_count = if present {
                self.workspaces
                    .memberships
                    .checked_add(1)
                    .filter(|n| *n <= limits.max_memberships)
                    .ok_or(CoordinatorError::Limit)?
            } else {
                self.workspaces
                    .memberships
                    .checked_sub(1)
                    .ok_or(CoordinatorError::Faulted)?
            };
            let publication = self.next_publication_generation(None)?;
            let len = if present {
                row.members
                    .len()
                    .checked_add(1)
                    .ok_or(CoordinatorError::Limit)?
            } else {
                row.members.len() - 1
            };
            let mut members = Vec::new();
            members
                .try_reserve_exact(len)
                .map_err(|_| CoordinatorError::Capacity)?;
            for (i, member) in row.members.iter().enumerate() {
                step(&mut work)?;
                if Some(i) != found {
                    members.push(member.clone());
                }
            }
            if present {
                step(&mut work)?;
                let at = members.partition_point(|m| m.slot < document.slot);
                members.insert(at, document.clone());
            }
            let removed = row
                .members
                .len()
                .checked_mul(std::mem::size_of::<DocumentId>())
                .ok_or(CoordinatorError::Limit)?;
            let added = members
                .len()
                .checked_mul(std::mem::size_of::<DocumentId>())
                .ok_or(CoordinatorError::Limit)?;
            self.check_root_replacement(removed, added, r)?;
            if let Some(invocation) = invocation {
                self.stage_live_relation_candidate(
                    invocation,
                    invocation::live_relation::Candidate::Membership {
                        workspace: context,
                        document,
                        present,
                    },
                    true,
                )
                .map_err(|_| CoordinatorError::Faulted)?;
            }
            self.faulted = true;
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(0),
                "injected workspace membership interruption"
            );
            self.workspaces.rows[context.workspace.slot] = Some(Workspace {
                generation,
                members,
            });
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(1),
                "injected workspace membership interruption"
            );
            self.workspaces.memberships = membership_count;
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(2),
                "injected workspace membership interruption"
            );
            self.generation = publication;
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(3),
                "injected workspace membership interruption"
            );
            self.faulted = false;
            Ok(WorkspaceUpdate {
                context: WorkspaceContext {
                    workspace: context.workspace.clone(),
                    generation,
                },
                changed: true,
            })
        }));
        let result = match result {
            Ok(v) => v,
            Err(panic) => {
                self.faulted = true;
                observe_service(
                    r,
                    Boundary::WorkspaceMembership,
                    &Err::<WorkspaceUpdate, _>(CoordinatorError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(panic)
            }
        };
        if matches!(
            result,
            Err(CoordinatorError::Faulted
                | CoordinatorError::History(HistoryError::Faulted)
                | CoordinatorError::Resource(crate::resource::ResourceError::Faulted))
        ) {
            self.faulted = true;
        }
        observe_service(r, Boundary::WorkspaceMembership, &result, timing.finish());
        result
    }
    pub fn retire_workspace(
        &mut self,
        context: &WorkspaceContext,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let result = self
            .ensure_operational()
            .and_then(|()| {
                self.stage_workspace_relation(
                    &RelationCommandV1::RetireWorkspace {
                        expected: context.clone(),
                    },
                    r,
                )
            })
            .and_then(|stage| stage.install(self, r));
        self.finish_workspace(r, Boundary::WorkspaceLifecycle, result)
    }
    pub(super) fn stage_workspace_disposal(
        &self,
        document: &DocumentId,
        r: &mut Recorder,
    ) -> Result<StagedWorkspaces, CoordinatorError> {
        let result = (|| {
            let Some(limits) = self.workspaces.limits else {
                return Ok(StagedWorkspaces {
                    updates: Vec::new(),
                    memberships: self.workspaces.memberships,
                });
            };
            let mut work = limits.max_work;
            let mut updates = Vec::new();
            let mut memberships = self.workspaces.memberships;
            for (slot, row) in self.workspaces.rows.iter().enumerate() {
                step(&mut work)?;
                let Some(row) = row else { continue };
                let mut found = false;
                for member in &row.members {
                    step(&mut work)?;
                    found |= member == document;
                }
                if !found {
                    continue;
                }
                let generation = row
                    .generation
                    .checked_add(1)
                    .ok_or(CoordinatorError::VersionExhausted)?;
                let mut members = Vec::new();
                members
                    .try_reserve_exact(row.members.len() - 1)
                    .map_err(|_| CoordinatorError::Capacity)?;
                for member in &row.members {
                    step(&mut work)?;
                    if member != document {
                        members.push(member.clone());
                    }
                }
                memberships = memberships
                    .checked_sub(1)
                    .ok_or(CoordinatorError::Faulted)?;
                updates
                    .try_reserve(1)
                    .map_err(|_| CoordinatorError::Capacity)?;
                updates.push((
                    slot,
                    Workspace {
                        generation,
                        members,
                    },
                ));
            }
            Ok(StagedWorkspaces {
                updates,
                memberships,
            })
        })();
        observe(r, Boundary::WorkspaceDisposalStaging, &result);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn document_context_pins_reject_workspace_gauge_drift_and_duplicate_target() {
        let (mut o, d, mut r) = setup(128);
        let ws = o.create_workspace(&mut r).unwrap();
        o.set_workspace_membership(&ws, &d.document, true, &mut r)
            .unwrap();
        o.workspaces.live += 1;
        assert_eq!(
            o.document_context_pins_v1(&d.document, &mut r),
            Err(CoordinatorError::Faulted)
        );
        o.workspaces.live -= 1;
        o.workspaces.memberships += 1;
        assert_eq!(
            o.document_context_pins_v1(&d.document, &mut r),
            Err(CoordinatorError::Faulted)
        );
        o.workspaces.rows[ws.workspace.slot]
            .as_mut()
            .unwrap()
            .members
            .push(d.document.clone());
        // Even a matching total cannot legitimize duplicate membership.
        assert_eq!(
            o.document_context_pins_v1(&d.document, &mut r),
            Err(CoordinatorError::Faulted)
        );
    }
    #[test]
    fn interrupted_actual_membership_installation_must_fence_the_owner() {
        let (mut o, d, mut r) = setup(128);
        let workspace = o.create_workspace(&mut r).unwrap();
        let generation = o.generation();
        o.publication_fault_after = Some(1);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| o.set_workspace_membership(
                &workspace,
                &d.document,
                true,
                &mut r
            )))
            .is_err()
        );
        assert!(
            o.workspaces.rows[workspace.workspace.slot]
                .as_ref()
                .unwrap()
                .members
                .contains(&d.document)
        );
        assert_eq!(o.workspaces.memberships, 0);
        assert_eq!(o.generation(), generation);
        assert!(o.is_faulted());
    }
    #[test]
    fn membership_interruption_cuts_frame_peers_and_preserve_unknown_without_refund() {
        for present in [false, true] {
            for cut in 0..4 {
                for mode in 0..4 {
                    let (mut o, d, mut setup) = setup(256);
                    let other = o.create(b"private peer", &mut setup).unwrap();
                    let workspace = o.create_workspace(&mut setup).unwrap();
                    let peer = o.create_workspace(&mut setup).unwrap();
                    let peer = o
                        .set_workspace_membership(&peer, &other.document, true, &mut setup)
                        .unwrap()
                        .context;
                    let workspace = if present {
                        workspace
                    } else {
                        o.set_workspace_membership(&workspace, &d.document, true, &mut setup)
                            .unwrap()
                            .context
                    };
                    let count = o.workspaces.memberships;
                    let generation = o.generation();
                    o.configure_retention_budget(8192, &mut setup).unwrap();
                    let held = o.retention.reserve(71).unwrap();
                    let usage = o.retention_usage();
                    o.publication_fault_after = Some(cut);
                    let mut r = if mode == 0 {
                        Recorder::disabled()
                    } else {
                        Recorder::try_with_capacity(if mode == 3 { 1 } else { 32 }).unwrap()
                    };
                    if mode >= 2 {
                        r = r.with_native_timing(
                            crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
                        );
                    }
                    if mode == 3 {
                        r.record(Observation {
                            boundary: Boundary::Control,
                            disposition: Disposition::Completed,
                            service_time: None,
                        });
                    }
                    assert!(
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                            || o.set_workspace_membership(&workspace, &d.document, present, &mut r)
                        ))
                        .is_err()
                    );
                    assert!(o.faulted);
                    assert_eq!(o.generation(), generation + u64::from(cut == 3));
                    let row = o.workspaces.rows[workspace.workspace.slot]
                        .as_ref()
                        .unwrap();
                    assert_eq!(
                        row.members.contains(&d.document),
                        if cut == 0 { !present } else { present }
                    );
                    assert_eq!(row.generation, workspace.generation + u64::from(cut >= 1));
                    assert_eq!(
                        o.workspaces.memberships,
                        if cut < 2 {
                            count
                        } else if present {
                            count + 1
                        } else {
                            count - 1
                        }
                    );
                    let peer_row = o.workspaces.rows[peer.workspace.slot].as_ref().unwrap();
                    assert_eq!(peer_row.generation, peer.generation);
                    assert_eq!(peer_row.members.as_slice(), &[other.document]);
                    assert_eq!(o.retention_usage(), usage);
                    assert!(o.retention.validate_exact(&held, 71).is_ok());
                    assert_eq!(
                        o.documents[d.document.slot].as_ref().unwrap().revision,
                        d.revision
                    );
                    assert_eq!(
                        o.create(b"blocked", &mut setup),
                        Err(CoordinatorError::Faulted)
                    );
                    assert_eq!(
                        o.set_workspace_membership(&workspace, &d.document, present, &mut setup),
                        Err(CoordinatorError::Faulted)
                    );
                    let events: Vec<_> = std::iter::from_fn(|| r.pop_front())
                        .filter(|e| e.boundary == Boundary::WorkspaceMembership)
                        .collect();
                    assert_eq!(events.len(), usize::from(mode == 1 || mode == 2));
                    for e in events {
                        assert_eq!(e.disposition, Disposition::Unknown);
                        assert_eq!(e.service_time.is_some(), mode == 2);
                        assert!(!format!("{e:?}").contains("private"));
                    }
                }
            }
        }
    }
    fn setup(work: usize) -> (SourceCoordinator, DocumentVersion, Recorder) {
        let mut o = SourceCoordinator::new(CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 128,
            source: Limits {
                max_source_bytes: 128,
                max_edits: 8,
                max_edit_payload_bytes: 256,
            },
        });
        let mut r = Recorder::disabled();
        o.configure_workspaces(
            WorkspaceLimits {
                max_issued: 8,
                max_live: 4,
                max_memberships: 8,
                max_work: work,
            },
            &mut r,
        )
        .unwrap();
        let a = o.create(b"A", &mut r).unwrap();
        (o, a, r)
    }
    #[test]
    fn last_member_generation_exhaustion_cannot_publish_an_earlier_workspace_or_dispose_source() {
        for retained in [false, true] {
            let (mut o, a, mut r) = setup(128);
            let x = o.create_workspace(&mut r).unwrap();
            let y = o.create_workspace(&mut r).unwrap();
            let x = o
                .set_workspace_membership(&x, &a.document, true, &mut r)
                .unwrap()
                .context;
            o.set_workspace_membership(&y, &a.document, true, &mut r)
                .unwrap();
            o.workspaces.rows[y.workspace.slot]
                .as_mut()
                .unwrap()
                .generation = u64::MAX;
            let g = o.generation();
            let usage = o.workspace_usage();
            let result = if retained {
                let p = o.prepare_retained_disposal(&a, 0, &mut r).unwrap();
                o.commit_retained_disposal(p, &mut r).map(|_| ())
            } else {
                o.dispose(&a.document, &mut r)
            };
            assert_eq!(result, Err(CoordinatorError::VersionExhausted));
            assert_eq!(o.generation(), g);
            assert_eq!(o.workspace_usage(), usage);
            assert_eq!(o.workspace(&x.workspace, &mut r).unwrap().context, x);
            assert_eq!(
                o.workspace(&y.workspace, &mut r).unwrap().members,
                vec![a.document.clone()]
            );
            assert_eq!(o.read(&a.document, &mut r).unwrap().version, a);
        }
    }
    #[test]
    fn exhausted_publication_or_membership_counter_preserves_validated_noops_and_real_state() {
        for publication in [false, true] {
            let (mut o, a, mut r) = setup(128);
            let x = o.create_workspace(&mut r).unwrap();
            let mut x = o
                .set_workspace_membership(&x, &a.document, true, &mut r)
                .unwrap()
                .context;
            if publication {
                o.generation = u64::MAX;
            } else {
                o.workspaces.rows[x.workspace.slot]
                    .as_mut()
                    .unwrap()
                    .generation = u64::MAX;
                x.generation = u64::MAX;
            }
            assert!(
                !o.set_workspace_membership(&x, &a.document, true, &mut r)
                    .unwrap()
                    .changed
            );
            let g = o.generation();
            assert_eq!(
                o.set_workspace_membership(&x, &a.document, false, &mut r)
                    .err(),
                Some(CoordinatorError::VersionExhausted)
            );
            assert_eq!(o.generation(), g);
            if publication {
                let usage = o.workspace_usage();
                assert_eq!(
                    o.create_workspace(&mut r),
                    Err(CoordinatorError::VersionExhausted)
                );
                assert_eq!(
                    o.retire_workspace(&x, &mut r),
                    Err(CoordinatorError::VersionExhausted)
                );
                assert_eq!(o.workspace_usage(), usage);
            }
        }
    }
    #[test]
    fn disposal_work_exhaustion_is_atomic_and_membership_accounting_contradiction_fences_owner() {
        let (mut o, a, mut r) = setup(2);
        let x = o.create_workspace(&mut r).unwrap();
        let x = o
            .set_workspace_membership(&x, &a.document, true, &mut r)
            .unwrap()
            .context;
        let g = o.generation();
        assert_eq!(o.dispose(&a.document, &mut r), Err(CoordinatorError::Limit));
        assert_eq!(o.generation(), g);
        assert_eq!(
            o.workspace(&x.workspace, &mut r).unwrap().members,
            vec![a.document.clone()]
        );
        assert_eq!(o.read(&a.document, &mut r).unwrap().version, a);
        o.workspaces.memberships = 0;
        assert_eq!(
            o.set_workspace_membership(&x, &a.document, false, &mut r)
                .err(),
            Some(CoordinatorError::Faulted)
        );
        assert!(o.is_faulted());
        assert_eq!(o.generation(), g);
        assert_eq!(
            o.documents[a.document.slot].as_ref().unwrap().source.text(),
            "A"
        );
        let (mut o, _, mut r) = setup(128);
        let x = o.create_workspace(&mut r).unwrap();
        o.workspaces.live = 0;
        let g = o.generation();
        assert_eq!(
            o.retire_workspace(&x, &mut r),
            Err(CoordinatorError::Faulted)
        );
        assert!(o.is_faulted());
        assert_eq!(o.generation(), g);
        assert!(o.workspaces.rows[x.workspace.slot].is_some());
    }
}
