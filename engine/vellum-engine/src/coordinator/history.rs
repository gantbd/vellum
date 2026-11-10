//! Source/annotation-scoped linked frontier history. Journals preserve lineage; no
//! document/policy snapshot is reinstalled, no external effect is replayed.

use super::*;
mod declaration;
#[cfg(test)]
mod reversal_guard_tests;
pub(in crate::coordinator) mod reversal_validation;
mod shutdown_inventory;
pub use declaration::native_history_reversal_declaration_v1;
mod basis;
pub mod inspection;
mod root_accounting;
pub use basis::{HistoryBasisLimitsV1, HistoryBasisScopeV1, HistoryBasisUsageV1, HistoryBasisV1};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistoryLimits {
    pub max_retained_groups: usize,
    /// Logical retained group/member/record headers, payloads and frontier links.
    /// Excludes allocator overhead and transient preparation copies.
    pub max_journal_bytes: usize,
    pub max_pins: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn owner() -> SourceCoordinator {
        SourceCoordinator::with_history_limits(
            CoordinatorLimits {
                max_document_incarnations: 8,
                max_participants: 8,
                max_staged_bytes: 1024,
                source: Limits {
                    max_source_bytes: 128,
                    max_edits: 8,
                    max_edit_payload_bytes: 1024,
                },
            },
            HistoryLimits {
                max_retained_groups: 8,
                max_journal_bytes: 65536,
                max_pins: 8,
            },
        )
    }
    fn write(version: DocumentVersion, from: &str, to: &str) -> DocumentEdits {
        DocumentEdits {
            expected: version,
            edits: vec![Replacement {
                start: 0,
                end: from.len(),
                expected: from.into(),
                inserted: to.into(),
            }],
        }
    }
    #[test]
    fn exhausted_history_version_cannot_publish_an_earlier_linked_participant() {
        let mut owner = owner();
        let mut recorder = Recorder::disabled();
        let a = owner.create(b"A", &mut recorder).unwrap();
        let b = owner.create(b"B", &mut recorder).unwrap();
        owner.documents[b.document.slot]
            .as_mut()
            .unwrap()
            .history
            .version = u64::MAX;
        let prepared = owner
            .prepare(
                &[write(a.clone(), "A", "a"), write(b.clone(), "B", "b")],
                &[],
                &mut recorder,
            )
            .unwrap();
        let before = owner.generation();
        assert_eq!(
            owner.commit(prepared, &mut recorder),
            Err(CoordinatorError::History(HistoryError::VersionExhausted))
        );
        assert_eq!(owner.generation(), before);
        assert!(!owner.is_faulted());
        assert_eq!(owner.history_usage().retained_groups, 0);
        assert_eq!(
            owner
                .read(&a.document, &mut recorder)
                .unwrap()
                .source
                .text(),
            "A"
        );
    }
    #[test]
    fn owned_journal_drift_is_a_fenced_integrity_fault_not_a_known_rejection() {
        let mut owner = owner();
        let mut recorder = Recorder::try_with_capacity(16).unwrap();
        let a = owner.create(b"A", &mut recorder).unwrap();
        let mut prepared = owner
            .prepare(&[write(a.clone(), "A", "a")], &[], &mut recorder)
            .unwrap();
        prepared.candidates[0].edits[0].inserted = "different private intent".into();
        assert_eq!(
            owner.commit(prepared, &mut recorder),
            Err(CoordinatorError::Faulted)
        );
        assert!(owner.is_faulted());
        assert_eq!(owner.history_usage().retained_groups, 0);
        assert_eq!(
            owner.documents[a.document.slot]
                .as_ref()
                .unwrap()
                .source
                .text(),
            "A"
        );
        assert_eq!(
            owner.history_status(&a.document, HistoryDirection::Undo, &mut recorder),
            Err(HistoryError::Faulted)
        );
        assert!(std::iter::from_fn(||recorder.pop_front()).any(|r|r.boundary==Boundary::Admission && r.disposition==Disposition::Unknown));
    }
    #[test]
    fn source_noop_needs_no_unused_history_or_operation_capacity() {
        let mut owner = owner();
        let mut recorder = Recorder::disabled();
        owner.history.limits = Some(HistoryLimits {
            max_retained_groups: 0,
            max_journal_bytes: 0,
            max_pins: 0,
        });
        let a = owner.create(b"A", &mut recorder).unwrap();
        owner.generation = u64::MAX;
        let prepared = owner
            .prepare(&[write(a, "A", "A")], &[], &mut recorder)
            .unwrap();
        let outcome = owner.commit(prepared, &mut recorder).unwrap();
        assert!(outcome.changes.is_empty());
        assert!(outcome.operation.is_none());
        assert_eq!(owner.history_usage().retained_groups, 0);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryGroupId {
    original: OperationId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryDirection {
    Undo,
    Redo,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryAvailability {
    Empty,
    Eligible(HistoryGroupId),
    Blocked(HistoryGroupId),
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryError {
    Unavailable,
    Blocked,
    Permission,
    Stale,
    Limit,
    Capacity,
    VersionExhausted,
    Pinned,
    Retained,
    Faulted,
    Source(SourceError),
}

#[derive(Clone)]
pub struct HistoryPin {
    namespace: Arc<()>,
    marker: Arc<()>,
}
impl std::fmt::Debug for HistoryPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HistoryPin(<opaque>)")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistoryUsage {
    pub retained_groups: usize,
    pub invalidated_groups: usize,
    pub journal_bytes: usize,
    pub active_pins: usize,
    pub max_retained_groups: usize,
    pub max_journal_bytes: usize,
    pub max_pins: usize,
    pub enabled: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Frontiers {
    undo: Vec<HistoryGroupId>,
    redo: Vec<HistoryGroupId>,
    version: u64,
    undo_lost: bool,
    redo_lost: bool,
}

fn copy_vec<T: Clone>(values: &[T]) -> Result<Vec<T>, HistoryError> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(values.len())
        .map_err(|_| HistoryError::Capacity)?;
    copy.extend(values.iter().cloned());
    Ok(copy)
}

impl Frontiers {
    pub(super) fn released(&self) -> bool {
        self.undo.is_empty() && self.redo.is_empty()
    }
    fn stack(&self, direction: HistoryDirection) -> &[HistoryGroupId] {
        match direction {
            HistoryDirection::Undo => &self.undo,
            HistoryDirection::Redo => &self.redo,
        }
    }
    fn copy(&self) -> Result<Self, HistoryError> {
        Ok(Self {
            undo: copy_vec(&self.undo)?,
            redo: copy_vec(&self.redo)?,
            version: self.version,
            undo_lost: self.undo_lost,
            redo_lost: self.redo_lost,
        })
    }
}

#[derive(Debug)]
pub(super) enum PreparedHistory {
    Ordinary,
    Reversal {
        group: HistoryGroupId,
        direction: HistoryDirection,
        stamps: Vec<(DocumentId, u64)>,
        annotation_stamps: Vec<(AnnotationId, u64)>,
    },
}
impl PreparedHistory {
    pub(super) fn refers_to_document(&self, d: &DocumentId) -> bool {
        match self {
            Self::Ordinary => false,
            Self::Reversal {
                stamps,
                annotation_stamps,
                ..
            } => {
                stamps.iter().any(|(v, _)| v == d)
                    || annotation_stamps.iter().any(|(v, _)| v.document == *d)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GroupState {
    Applied,
    Undone,
    Invalidated,
}

#[derive(Debug)]
struct Member {
    document: DocumentId,
    forward: Vec<Replacement>,
    inverse: Vec<Replacement>,
}

#[derive(Debug)]
struct Group {
    id: HistoryGroupId,
    members: Vec<Member>,
    annotations: Vec<annotation::AnnotationJournal>,
    state: GroupState,
    bytes: usize,
}

#[derive(Debug)]
struct Pin {
    token: HistoryPin,
    group: HistoryGroupId,
}

#[derive(Debug)]
pub(super) struct HistoryState {
    limits: Option<HistoryLimits>,
    groups: Vec<Group>,
    pins: Vec<Pin>,
    bytes: usize,
}
impl HistoryState {
    pub(super) fn document_pins(
        &self,
        document: &DocumentId,
    ) -> Result<(usize, usize), CoordinatorError> {
        let Self {
            limits: _,
            groups,
            pins,
            bytes: _,
        } = self;
        let refers = |g: &Group| {
            g.members.iter().any(|m| m.document == *document)
                || g.annotations.iter().any(|m| m.id.document == *document)
        };
        let mut held = 0;
        for pin in pins {
            let group = groups
                .iter()
                .find(|g| g.id == pin.group)
                .ok_or(CoordinatorError::Faulted)?;
            if refers(group) {
                held += 1;
            }
        }
        Ok((groups.iter().filter(|g| refers(g)).count(), held))
    }
    pub(super) fn captured_limits(&self) -> Option<HistoryLimits> {
        self.limits
    }
    pub(super) fn disabled() -> Self {
        Self {
            limits: None,
            groups: Vec::new(),
            pins: Vec::new(),
            bytes: 0,
        }
    }
    pub(super) fn new(limits: HistoryLimits) -> Self {
        Self {
            limits: Some(limits),
            ..Self::disabled()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Scope {
    Source(DocumentId),
    Annotation(AnnotationId),
}
#[derive(Debug)]
struct FrontierUpdate {
    scope: Scope,
    frontier: Frontiers,
}
impl FrontierUpdate {
    fn install(self, owner: &mut SourceCoordinator) {
        match self.scope {
            Scope::Source(id) => {
                owner.documents[id.slot]
                    .as_mut()
                    .expect("live source scope")
                    .history = self.frontier
            }
            Scope::Annotation(id) => {
                owner.annotations.records[id.slot]
                    .as_mut()
                    .expect("retained annotation scope")
                    .history = self.frontier
            }
        }
    }
}
impl Group {
    fn contains(&self, scope: &Scope) -> bool {
        match scope {
            Scope::Source(id) => self.members.iter().any(|m| m.document == *id),
            Scope::Annotation(id) => self.annotations.iter().any(|m| m.id == *id),
        }
    }
}

#[derive(Debug)]
pub(super) struct HistoryDecision {
    group: Option<Group>,
    invalidated: Vec<HistoryGroupId>,
    reversal: Option<(HistoryGroupId, GroupState)>,
    updates: Vec<FrontierUpdate>,
    outcome_group: Option<HistoryGroupId>,
}
impl HistoryDecision {
    fn empty() -> Self {
        Self {
            group: None,
            invalidated: Vec::new(),
            reversal: None,
            updates: Vec::new(),
            outcome_group: None,
        }
    }
    pub(super) fn group_id(&self) -> Option<HistoryGroupId> {
        self.outcome_group.clone()
    }
    pub(super) fn install(self, owner: &mut SourceCoordinator) {
        for group in &mut owner.history.groups {
            if self.invalidated.contains(&group.id) {
                group.state = GroupState::Invalidated;
            }
            if let Some((id, state)) = &self.reversal
                && *id == group.id
            {
                group.state = *state;
            }
        }
        for update in self.updates {
            update.install(owner);
        }
        if let Some(group) = self.group {
            owner.history.bytes += group.bytes;
            owner.history.groups.push(group); // storage reserved before publication
        }
    }
}

fn source_error(error: SourceError) -> HistoryError {
    match error {
        SourceError::Capacity => HistoryError::Capacity,
        SourceError::StagingLimit => HistoryError::Limit,
        other => HistoryError::Source(other),
    }
}
fn owner_error(error: CoordinatorError) -> HistoryError {
    match error {
        CoordinatorError::Faulted => HistoryError::Faulted,
        CoordinatorError::Stale => HistoryError::Stale,
        CoordinatorError::VersionExhausted => HistoryError::VersionExhausted,
        CoordinatorError::Capacity => HistoryError::Capacity,
        CoordinatorError::Source(error) => source_error(error),
        CoordinatorError::History(error) => error,
        CoordinatorError::Limit => HistoryError::Limit,
        _ => HistoryError::Unavailable,
    }
}
fn capture<T>(recorder: &mut Recorder, boundary: Boundary, result: &Result<T, HistoryError>) {
    recorder.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(HistoryError::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time: None,
    });
}

impl SourceCoordinator {
    pub fn with_history_limits(limits: CoordinatorLimits, history: HistoryLimits) -> Self {
        let mut owner = Self::new(limits);
        owner.history = HistoryState::new(history);
        owner
    }

    pub fn with_invocation_and_history_limits(
        limits: CoordinatorLimits,
        invocation: InvocationLimits,
        history: HistoryLimits,
    ) -> Self {
        let mut owner = Self::with_invocation_limits(limits, invocation);
        owner.history = HistoryState::new(history);
        owner
    }

    /// Trusted-owner gauges, never authority or proof that history can be dropped.
    pub fn history_usage(&self) -> HistoryUsage {
        let limits = self.history.limits.unwrap_or(HistoryLimits {
            max_retained_groups: 0,
            max_journal_bytes: 0,
            max_pins: 0,
        });
        HistoryUsage {
            retained_groups: self.history.groups.len(),
            invalidated_groups: self
                .history
                .groups
                .iter()
                .filter(|g| g.state == GroupState::Invalidated)
                .count(),
            journal_bytes: self.history.bytes,
            active_pins: self.history.pins.len(),
            max_retained_groups: limits.max_retained_groups,
            max_journal_bytes: limits.max_journal_bytes,
            max_pins: limits.max_pins,
            enabled: self.history.limits.is_some(),
        }
    }

    pub(super) fn history_retains_document(&self, document: &DocumentId) -> bool {
        self.history.groups.iter().any(|g| {
            g.members.iter().any(|m| m.document == *document)
                || g.annotations.iter().any(|m| m.id.document == *document)
        })
    }
    pub(super) fn annotation_history_retains_document(&self, document: &DocumentId) -> bool {
        self.history
            .groups
            .iter()
            .any(|g| g.annotations.iter().any(|m| m.id.document == *document))
    }
    pub(super) fn history_retains_annotation(&self, id: &AnnotationId) -> bool {
        // Invalidated groups still own their journals until actual retirement.
        self.history
            .groups
            .iter()
            .any(|g| g.annotations.iter().any(|m| m.id == *id))
    }

    fn group(&self, id: &HistoryGroupId) -> Result<&Group, HistoryError> {
        self.history
            .groups
            .iter()
            .find(|g| g.id == *id)
            .ok_or(HistoryError::Unavailable)
    }

    fn ready_group(
        &self,
        id: &HistoryGroupId,
        direction: HistoryDirection,
    ) -> Result<&Group, HistoryError> {
        let group = self.group(id)?;
        if group.state == GroupState::Invalidated {
            return Err(HistoryError::Unavailable);
        }
        let expected = match direction {
            HistoryDirection::Undo => GroupState::Applied,
            HistoryDirection::Redo => GroupState::Undone,
        };
        if group.state != expected {
            return Err(HistoryError::Blocked);
        }
        for member in &group.members {
            let document = self.document(&member.document).map_err(owner_error)?;
            if document.history.stack(direction).last() != Some(id) {
                return Err(HistoryError::Blocked);
            }
        }
        for member in &group.annotations {
            let record = self.annotation_record(&member.id).map_err(owner_error)?;
            if record.history.stack(direction).last() != Some(id) {
                return Err(HistoryError::Blocked);
            }
        }
        Ok(group)
    }

    /// Frontier advice only. Current write permission is checked at reversal.
    pub fn history_status(
        &self,
        document: &DocumentId,
        direction: HistoryDirection,
        recorder: &mut Recorder,
    ) -> Result<HistoryAvailability, HistoryError> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            let document = self.document(document).map_err(owner_error)?;
            if self.history.limits.is_none() {
                return Ok(HistoryAvailability::Unavailable);
            }
            let Some(top) = document.history.stack(direction).last() else {
                let lost = match direction {
                    HistoryDirection::Undo => document.history.undo_lost,
                    HistoryDirection::Redo => document.history.redo_lost,
                };
                return Ok(if lost {
                    HistoryAvailability::Unavailable
                } else {
                    HistoryAvailability::Empty
                });
            };
            Ok(match self.ready_group(top, direction) {
                Ok(_) => HistoryAvailability::Eligible(top.clone()),
                Err(HistoryError::Blocked) => HistoryAvailability::Blocked(top.clone()),
                Err(_) => HistoryAvailability::Unavailable,
            })
        })();
        capture(recorder, Boundary::HistoryQuery, &result);
        result
    }

    pub fn annotation_history_status(
        &self,
        annotation: &AnnotationId,
        direction: HistoryDirection,
        recorder: &mut Recorder,
    ) -> Result<HistoryAvailability, HistoryError> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            let frontier = &self
                .annotation_record(annotation)
                .map_err(owner_error)?
                .history;
            if self.history.limits.is_none() {
                return Ok(HistoryAvailability::Unavailable);
            }
            let Some(top) = frontier.stack(direction).last() else {
                let lost = match direction {
                    HistoryDirection::Undo => frontier.undo_lost,
                    HistoryDirection::Redo => frontier.redo_lost,
                };
                return Ok(if lost {
                    HistoryAvailability::Unavailable
                } else {
                    HistoryAvailability::Empty
                });
            };
            Ok(match self.ready_group(top, direction) {
                Ok(_) => HistoryAvailability::Eligible(top.clone()),
                Err(HistoryError::Blocked) => HistoryAvailability::Blocked(top.clone()),
                _ => HistoryAvailability::Unavailable,
            })
        })();
        capture(recorder, Boundary::HistoryQuery, &result);
        result
    }
    pub fn prepare_reversal(
        &self,
        id: &HistoryGroupId,
        direction: HistoryDirection,
        recorder: &mut Recorder,
    ) -> Result<PreparedTransaction, HistoryError> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            let group = self.ready_group(id, direction)?;
            if group
                .members
                .len()
                .checked_add(group.annotations.len())
                .is_none_or(|n| n > self.limits.max_participants)
            {
                return Err(HistoryError::Limit);
            }
            let mut candidates = Vec::new();
            let mut stamps = Vec::new();
            candidates
                .try_reserve_exact(group.members.len())
                .map_err(|_| HistoryError::Capacity)?;
            stamps
                .try_reserve_exact(group.members.len())
                .map_err(|_| HistoryError::Capacity)?;
            let mut staged = 0usize;
            for member in &group.members {
                let document = self.document(&member.document).map_err(owner_error)?;
                if document.read_only {
                    return Err(HistoryError::Permission);
                }
                let journal = match direction {
                    HistoryDirection::Undo => &member.inverse,
                    HistoryDirection::Redo => &member.forward,
                };
                let source = document
                    .source
                    .prepare_journal(
                        journal,
                        self.limits.source,
                        self.limits.max_staged_bytes - staged,
                    )
                    .map_err(source_error)?;
                staged = staged
                    .checked_add(source.text().len())
                    .ok_or(HistoryError::Limit)?;
                stamps.push((member.document.clone(), document.history.version));
                candidates.push(Candidate {
                    annotation_generation: document.annotation_generation,
                    expected: DocumentVersion {
                        document: member.document.clone(),
                        revision: document.revision,
                    },
                    policy_generation: document.policy_generation,
                    source,
                    edits: crate::source::copy_replacements(journal).map_err(source_error)?,
                });
            }
            let annotation_stamps = group
                .annotations
                .iter()
                .map(|m| {
                    self.annotation_record(&m.id)
                        .map(|r| (m.id.clone(), r.history.version))
                        .map_err(owner_error)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let view_dependencies = self
                .capture_view_dependencies(&candidates)
                .map_err(owner_error)?;
            let mut prepared = PreparedTransaction {
                configured_required: false,
                configured_line: None,
                reads_required: false,
                declared_reads: None,
                scoped: false,
                scopes: None,
                derived_edit: Vec::new(),
                derived_kind: None,
                view_dependencies,
                annotations: Vec::new(),
                search_dependency: None,
                namespace: self.namespace.clone(),
                candidates,
                read_dependencies: Vec::new(),
                history: PreparedHistory::Reversal {
                    group: id.clone(),
                    direction,
                    stamps,
                    annotation_stamps,
                },
            };
            self.annotation_reversal_plans(&group.annotations, direction, &mut prepared)?;
            Ok(prepared)
        })();
        capture(recorder, Boundary::HistoryPreparation, &result);
        result
    }

    /// Dependency closure: removing a frontier entry makes earlier undo history
    /// or later redo continuations below it unusable, across all linked scopes.
    fn history_closure(
        &self,
        seeds: &[HistoryGroupId],
        undo_too: bool,
    ) -> Result<Vec<HistoryGroupId>, HistoryError> {
        let mut removed = Vec::new();
        removed
            .try_reserve_exact(self.history.groups.len())
            .map_err(|_| HistoryError::Capacity)?;
        for seed in seeds {
            self.group(seed)?;
            if !removed.contains(seed) {
                removed.push(seed.clone());
            }
        }
        loop {
            let before = removed.len();
            for frontier in self.documents.iter().flatten().map(|d| &d.history).chain(
                self.annotations
                    .records
                    .iter()
                    .flatten()
                    .map(|r| &r.history),
            ) {
                let stacks = [&frontier.redo, &frontier.undo];
                for stack in stacks.into_iter().take(if undo_too { 2 } else { 1 }) {
                    for (index, group) in stack.iter().enumerate() {
                        if removed.contains(group) {
                            for dependency in &stack[..index] {
                                self.group(dependency)?;
                                if !removed.contains(dependency) {
                                    removed.push(dependency.clone());
                                }
                            }
                        }
                    }
                }
            }
            if removed.len() == before {
                return Ok(removed);
            }
        }
    }

    fn stage_frontiers(
        &self,
        removed: &[HistoryGroupId],
        retiring: bool,
        append: Option<&Group>,
        reversal: Option<(&Group, HistoryDirection)>,
    ) -> Result<Vec<FrontierUpdate>, HistoryError> {
        let mut updates = Vec::new();
        let capacity = self
            .documents
            .len()
            .checked_add(self.annotations.records.len())
            .ok_or(HistoryError::Limit)?;
        updates
            .try_reserve_exact(capacity)
            .map_err(|_| HistoryError::Capacity)?;
        let mut scopes = Vec::new();
        scopes
            .try_reserve_exact(capacity)
            .map_err(|_| HistoryError::Capacity)?;
        for (slot, _) in self
            .documents
            .iter()
            .enumerate()
            .filter(|(_, d)| d.is_some())
        {
            scopes.push(Scope::Source(DocumentId {
                namespace: self.namespace.clone(),
                slot,
            }));
        }
        for record in self.annotations.records.iter().flatten() {
            scopes.push(Scope::Annotation(record.value.id.clone()));
        }
        if let Some(group) = append {
            for member in &group.annotations {
                let scope = Scope::Annotation(member.id.clone());
                if !scopes.contains(&scope) {
                    scopes.push(scope);
                }
            }
        }
        for scope in scopes {
            let empty = Frontiers::default();
            let current = match &scope {
                Scope::Source(id) => &self.retained_document(id).map_err(owner_error)?.history,
                Scope::Annotation(id) => self
                    .annotations
                    .records
                    .get(id.slot)
                    .and_then(Option::as_ref)
                    .map_or(&empty, |r| &r.history),
            };
            let add = append.is_some_and(|g| g.contains(&scope));
            let reverse = reversal.filter(|(g, _)| g.contains(&scope));
            let remove_undo = retiring && current.undo.iter().any(|g| removed.contains(g));
            let remove_redo = current.redo.iter().any(|g| removed.contains(g));
            if !add && reverse.is_none() && !remove_undo && !remove_redo {
                continue;
            }
            let mut frontier = current.copy()?;
            if remove_undo {
                frontier.undo.retain(|g| !removed.contains(g));
                frontier.undo_lost = true;
            }
            if remove_redo {
                frontier.redo.retain(|g| !removed.contains(g));
                frontier.redo_lost = true;
            }
            if let Some(group) = append.filter(|_| add) {
                frontier
                    .undo
                    .try_reserve(1)
                    .map_err(|_| HistoryError::Capacity)?;
                frontier.undo.push(group.id.clone());
            }
            if let Some((group, direction)) = reverse {
                let (from, to) = match direction {
                    HistoryDirection::Undo => (&mut frontier.undo, &mut frontier.redo),
                    HistoryDirection::Redo => (&mut frontier.redo, &mut frontier.undo),
                };
                to.try_reserve(1).map_err(|_| HistoryError::Capacity)?;
                if from.pop().as_ref() != Some(&group.id) {
                    return Err(HistoryError::Blocked);
                }
                to.push(group.id.clone());
            }
            frontier.version = frontier
                .version
                .checked_add(1)
                .ok_or(HistoryError::VersionExhausted)?;
            updates.push(FrontierUpdate { scope, frontier });
        }
        Ok(updates)
    }

    pub(super) fn stage_history(
        &mut self,
        prepared: &PreparedTransaction,
        generation: u64,
        journals: &[annotation::AnnotationJournal],
    ) -> Result<HistoryDecision, HistoryError> {
        let decision = self.decide_history(prepared, generation, journals)?;
        if decision.group.is_some() {
            self.history
                .groups
                .try_reserve(1)
                .map_err(|_| HistoryError::Capacity)?;
        }
        Ok(decision)
    }
    /// Private candidate only: does not reserve storage in the live journal.
    pub(super) fn decide_history(
        &self,
        prepared: &PreparedTransaction,
        generation: u64,
        journals: &[annotation::AnnotationJournal],
    ) -> Result<HistoryDecision, HistoryError> {
        let Some(limits) = self.history.limits else {
            return Ok(HistoryDecision::empty());
        };
        if let PreparedHistory::Reversal {
            group,
            direction,
            stamps,
            annotation_stamps,
        } = &prepared.history
        {
            let actual = self.ready_group(group, *direction)?;
            if stamps.len() != actual.members.len() {
                return Err(HistoryError::Unavailable);
            }
            for (document, version) in stamps {
                if self
                    .document(document)
                    .map_err(owner_error)?
                    .history
                    .version
                    != *version
                {
                    return Err(HistoryError::Stale);
                }
            }
            if annotation_stamps.len() != actual.annotations.len() {
                return Err(HistoryError::Unavailable);
            }
            for (id, version) in annotation_stamps {
                if self
                    .annotation_record(id)
                    .map_err(owner_error)?
                    .history
                    .version
                    != *version
                {
                    return Err(HistoryError::Stale);
                }
            }
            let updates = self.stage_frontiers(&[], false, None, Some((actual, *direction)))?;
            return Ok(HistoryDecision {
                reversal: Some((
                    group.clone(),
                    match direction {
                        HistoryDirection::Undo => GroupState::Undone,
                        HistoryDirection::Redo => GroupState::Applied,
                    },
                )),
                updates,
                outcome_group: Some(group.clone()),
                ..HistoryDecision::empty()
            });
        }
        if journals.is_empty()
            && prepared.candidates.iter().all(|c| {
                self.document(&c.expected.document)
                    .is_ok_and(|d| d.source == c.source)
            })
        {
            return Ok(HistoryDecision::empty());
        }
        if self.history.groups.len() >= limits.max_retained_groups {
            return Err(HistoryError::Limit);
        }
        let mut bytes = std::mem::size_of::<Group>();
        let remaining = limits
            .max_journal_bytes
            .checked_sub(self.history.bytes)
            .ok_or(HistoryError::Limit)?;
        let mut charge = |n: usize| -> Result<(), HistoryError> {
            bytes = bytes
                .checked_add(n)
                .filter(|b| *b <= remaining)
                .ok_or(HistoryError::Limit)?;
            Ok(())
        };
        charge(0)?;
        for candidate in &prepared.candidates {
            if self
                .document(&candidate.expected.document)
                .map_err(owner_error)?
                .source
                == candidate.source
            {
                continue;
            }
            charge(std::mem::size_of::<Member>())?;
            charge(std::mem::size_of::<HistoryGroupId>())?;
            for edit in &candidate.edits {
                for _ in 0..2 {
                    charge(std::mem::size_of::<Replacement>())?;
                    charge(edit.expected.len())?;
                    charge(edit.inserted.len())?;
                }
            }
        }
        for journal in journals {
            charge(journal.charge()?)?;
            charge(std::mem::size_of::<HistoryGroupId>())?;
        }
        let id = HistoryGroupId {
            original: OperationId {
                namespace: self.namespace.clone(),
                generation,
            },
        };
        let mut group = Group {
            id: id.clone(),
            members: Vec::new(),
            annotations: copy_vec(journals)?,
            state: GroupState::Applied,
            bytes,
        };
        group
            .members
            .try_reserve_exact(prepared.candidates.len())
            .map_err(|_| HistoryError::Capacity)?;
        let mut seeds = Vec::new();
        seeds
            .try_reserve_exact(self.history.groups.len())
            .map_err(|_| HistoryError::Capacity)?;
        for candidate in &prepared.candidates {
            let document = self
                .document(&candidate.expected.document)
                .map_err(owner_error)?;
            if document.source == candidate.source {
                continue;
            }
            for redo in &document.history.redo {
                if !seeds.contains(redo) {
                    seeds.push(redo.clone());
                }
            }
            let forward =
                crate::source::copy_replacements(&candidate.edits).map_err(source_error)?;
            let mut inverse =
                crate::source::copy_replacements(&candidate.edits).map_err(source_error)?;
            let (mut source_cursor, mut destination_cursor) = (0usize, 0usize);
            for edit in &mut inverse {
                let end = edit.end;
                let start = destination_cursor
                    .checked_add(
                        edit.start
                            .checked_sub(source_cursor)
                            .ok_or(HistoryError::Unavailable)?,
                    )
                    .ok_or(HistoryError::Limit)?;
                let next = start
                    .checked_add(edit.inserted.len())
                    .ok_or(HistoryError::Limit)?;
                std::mem::swap(&mut edit.expected, &mut edit.inserted);
                edit.start = start;
                edit.end = next;
                source_cursor = end;
                destination_cursor = next;
            }
            group.members.push(Member {
                document: candidate.expected.document.clone(),
                forward,
                inverse,
            });
        }
        for journal in journals {
            if let Ok(record) = self.annotation_record(&journal.id) {
                for redo in &record.history.redo {
                    if !seeds.contains(redo) {
                        seeds.push(redo.clone());
                    }
                }
            }
        }
        let invalidated = self.history_closure(&seeds, false)?;
        let updates = self.stage_frontiers(&invalidated, false, Some(&group), None)?;
        Ok(HistoryDecision {
            group: Some(group),
            invalidated,
            updates,
            outcome_group: Some(id),
            ..HistoryDecision::empty()
        })
    }

    #[cfg(test)]
    pub(super) fn advice_history_capacity_for_test(&self) -> usize {
        self.history.groups.capacity()
    }
    /// Retention promise only: pinning does not preserve reversal eligibility.
    pub fn pin_history(
        &mut self,
        group: &HistoryGroupId,
        recorder: &mut Recorder,
    ) -> Result<HistoryPin, HistoryError> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            let limits = self.history.limits.ok_or(HistoryError::Unavailable)?;
            if self.group(group)?.state == GroupState::Invalidated {
                return Err(HistoryError::Unavailable);
            }
            if self.history.pins.len() >= limits.max_pins {
                return Err(HistoryError::Limit);
            }
            self.history
                .pins
                .try_reserve(1)
                .map_err(|_| HistoryError::Capacity)?;
            let token = HistoryPin {
                namespace: self.namespace.clone(),
                marker: Arc::new(()),
            };
            self.history.pins.push(Pin {
                token: token.clone(),
                group: group.clone(),
            });
            Ok(token)
        })();
        capture(recorder, Boundary::HistoryControl, &result);
        result
    }

    pub fn release_history_pin(
        &mut self,
        pin: &HistoryPin,
        recorder: &mut Recorder,
    ) -> Result<(), HistoryError> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            if !Arc::ptr_eq(&self.namespace, &pin.namespace) {
                return Err(HistoryError::Unavailable);
            }
            let index = self
                .history
                .pins
                .iter()
                .position(|p| Arc::ptr_eq(&p.token.marker, &pin.marker))
                .ok_or(HistoryError::Unavailable)?;
            self.history.pins.swap_remove(index);
            Ok(())
        })();
        capture(recorder, Boundary::HistoryControl, &result);
        result
    }

    /// Explicit complete-group retirement. No automatic eviction or promised
    /// floor beyond actual retained pins is offered by this concrete profile.
    pub fn retire_history_group(
        &mut self,
        group: &HistoryGroupId,
        recorder: &mut Recorder,
    ) -> Result<usize, HistoryError> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            self.group(group)?;
            let removed = self.history_closure(std::slice::from_ref(group), true)?;
            if self.history.pins.iter().any(|p| removed.contains(&p.group)) {
                return Err(HistoryError::Pinned);
            }
            let updates = self.stage_frontiers(&removed, true, None, None)?;
            let bytes = self
                .history
                .groups
                .iter()
                .filter(|g| removed.contains(&g.id))
                .try_fold(0usize, |n, g| {
                    n.checked_add(g.bytes).ok_or(HistoryError::Limit)
                })?;
            let generation = self
                .next_retention_publication_generation(None)
                .map_err(|_| HistoryError::VersionExhausted)?;
            self.faulted = true;
            for update in updates {
                update.install(self);
            }
            self.history.groups.retain(|g| !removed.contains(&g.id));
            self.history.bytes -= bytes;
            self.generation = generation;
            self.faulted = false;
            Ok(removed.len())
        })();
        capture(recorder, Boundary::HistoryControl, &result);
        result
    }
}
