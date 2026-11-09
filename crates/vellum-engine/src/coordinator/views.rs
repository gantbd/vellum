//! Headless logical selection contexts. No keymaps, geometry, focus or rendering.
use super::*;
mod command;
pub(in crate::coordinator) mod composition;
mod line;
mod root_accounting;
mod shutdown_inventory;
#[cfg(test)]
pub(in crate::coordinator) use line::line_test_fixture;
mod literal;
mod unicode;
pub use unicode::{
    PreparedUnicodeMovement, UnicodeCommandBudget, UnicodeCommandError, UnicodeCommandV1,
    UnicodeDirectionV1, UnicodeUnitV1,
};
#[derive(Debug)]
pub(super) enum DerivedEdit {
    Literal(literal::SelectionEdit),
    Line(line::LineEdit),
    Unicode(unicode::UnicodeEdit),
    Composition(composition::CompositionEdit),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DerivedKind {
    Literal,
    Line,
    Unicode,
    Composition,
}
impl DerivedEdit {
    fn kind(&self) -> DerivedKind {
        match self {
            Self::Literal(_) => DerivedKind::Literal,
            Self::Line(_) => DerivedKind::Line,
            Self::Unicode(_) => DerivedKind::Unicode,
            Self::Composition(_) => DerivedKind::Composition,
        }
    }
}
use crate::attachment::{Affinity, AttachmentError};
use crate::coordinates::{BytePosition, CoordinateBudget, SourceCoordinates};
use crate::selection::{
    Selection, SelectionBudget, SelectionError, SelectionSet, normalize_selections,
};
pub(super) use command::PreparedSelectionCommand;
pub use command::SelectionCommandV1;
pub use command::{SearchSelectionModeV1, SelectionDirectionV1};
pub use line::{GeneratedNewlineV1, LineEditBudget, LineEditError, LineEditV1, NewlineIndentV1};
pub use literal::SelectionAfterV1;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogicalViewLimits {
    pub max_live_views: usize,
    pub max_issued_views: usize,
    pub max_total_selections: usize,
    pub normalization: SelectionBudget,
    pub max_mapping_steps: usize,
}
#[derive(Clone)]
pub struct LogicalViewId {
    owner: Arc<()>,
    slot: usize,
}
impl PartialEq for LogicalViewId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for LogicalViewId {}
impl std::fmt::Debug for LogicalViewId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LogicalViewId(<opaque>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogicalViewStamp {
    pub attachment_generation: u64,
    pub selection_generation: u64,
    pub viewport_generation: u64,
}
impl LogicalViewStamp {
    fn relation_next(self) -> Result<Self, SelectionError> {
        Ok(Self {
            attachment_generation: next(self.attachment_generation)?,
            selection_generation: next(self.selection_generation)?,
            viewport_generation: next(self.viewport_generation)?,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogicalViewContext {
    pub view: LogicalViewId,
    pub source: DocumentVersion,
    pub stamp: LogicalViewStamp,
}
#[derive(Debug)]
pub struct LogicalViewSnapshot {
    pub view: LogicalViewId,
    pub source: Option<DocumentVersion>,
    pub stamp: LogicalViewStamp,
    pub selections: SelectionSet,
}
impl LogicalViewSnapshot {
    pub fn context(&self) -> Option<LogicalViewContext> {
        self.source.as_ref().map(|source| LogicalViewContext {
            view: self.view.clone(),
            source: source.clone(),
            stamp: self.stamp,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogicalViewUpdate {
    pub changed: bool,
    pub stamp: LogicalViewStamp,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogicalViewUsage {
    pub issued_lifetime: usize,
    pub live: usize,
    pub attached: usize,
    pub selections: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogicalEndpointV1 {
    LineStart,
    LineEnd,
    DocumentStart,
    DocumentEnd,
}
#[derive(Debug)]
struct View {
    source: Option<DocumentVersion>,
    stamp: LogicalViewStamp,
    selections: SelectionSet,
}
enum ViewRelationChange {
    Create(LogicalViewId),
    Retire(LogicalViewId, LogicalViewStamp),
}
pub(super) struct ViewRelationStage {
    owner: Arc<()>,
    before: u64,
    publication: u64,
    profile: LogicalViewLimits,
    change: ViewRelationChange,
}
impl ViewRelationStage {
    pub(super) fn receipt(&self) -> RelationReceiptV1 {
        match &self.change {
            ViewRelationChange::Create(id) => RelationReceiptV1::ViewCreated(id.clone()),
            ViewRelationChange::Retire(id, _) => RelationReceiptV1::ViewRetired(id.clone()),
        }
    }
    pub(super) fn install(
        self,
        owner: &mut SourceCoordinator,
        r: &mut Recorder,
    ) -> Result<(), SelectionError> {
        if !Arc::ptr_eq(&owner.namespace, &self.owner)
            || owner.generation != self.before
            || owner.views.limits != Some(self.profile)
        {
            return Err(SelectionError::Faulted);
        }
        if owner.next_publication_generation(None).map_err(error)? != self.publication {
            return Err(SelectionError::Faulted);
        }
        match self.change {
            ViewRelationChange::Create(id) => {
                if id.slot != owner.views.rows.len()
                    || !Arc::ptr_eq(&id.owner, &owner.namespace)
                    || owner.views.rows.capacity() <= owner.views.rows.len()
                {
                    return Err(SelectionError::Faulted);
                }
                owner.validate_view_relation(&RelationCommandV1::CreateView, r)?;
                owner.views.rows.push(Some(View {
                    source: None,
                    stamp: LogicalViewStamp {
                        attachment_generation: 1,
                        selection_generation: 1,
                        viewport_generation: 1,
                    },
                    selections: SelectionSet::detached(),
                }));
            }
            ViewRelationChange::Retire(id, expected) => {
                owner.validate_view_relation(
                    &RelationCommandV1::RetireView {
                        view: id.clone(),
                        expected,
                    },
                    r,
                )?;
                owner.views.rows[id.slot] = None;
            }
        }
        owner.generation = self.publication;
        Ok(())
    }
}
#[derive(Debug, Default)]
pub(super) struct ViewState {
    limits: Option<LogicalViewLimits>,
    line_limits: Option<LineEditBudget>,
    unicode_limits: Option<UnicodeCommandBudget>,
    rows: Vec<Option<View>>,
}
impl ViewState {
    pub(super) fn document_pins(&self, document: &DocumentId) -> usize {
        self.rows
            .iter()
            .flatten()
            .filter(|row| {
                row.source
                    .as_ref()
                    .is_some_and(|source| source.document == *document)
            })
            .count()
    }
    pub(super) fn profile(&self) -> Option<LogicalViewLimits> {
        self.limits
    }
}
pub(in crate::coordinator) type ViewRootIdentity = (
    Option<LogicalViewLimits>,
    Option<LineEditBudget>,
    Option<UnicodeCommandBudget>,
);
#[derive(Debug, PartialEq, Eq)]
pub(super) struct ViewDependency {
    id: LogicalViewId,
    source: DocumentVersion,
    stamp: LogicalViewStamp,
    selections: SelectionSet,
}
impl ViewDependency {
    pub(in crate::coordinator) fn selection_count(&self) -> usize {
        self.selections.selections().len()
    }
    pub(super) fn refers_to_document(&self, d: &DocumentId) -> bool {
        self.source.document == *d
    }
}
pub(super) struct StagedViews {
    updates: Vec<(usize, View)>,
}
impl StagedViews {
    pub(super) fn changed(&self) -> bool {
        !self.updates.is_empty()
    }
    pub(super) fn install(self, owner: &mut SourceCoordinator) {
        for (slot, view) in self.updates {
            owner.views.rows[slot] = Some(view);
        }
    }
}
fn next(value: u64) -> Result<u64, SelectionError> {
    value.checked_add(1).ok_or(SelectionError::VersionExhausted)
}
fn error(error: CoordinatorError) -> SelectionError {
    match error {
        CoordinatorError::Faulted
        | CoordinatorError::History(HistoryError::Faulted)
        | CoordinatorError::Resource(crate::resource::ResourceError::Faulted) => {
            SelectionError::Faulted
        }
        CoordinatorError::Stale => SelectionError::Stale,
        CoordinatorError::Limit => SelectionError::Limit,
        CoordinatorError::Capacity => SelectionError::Capacity,
        CoordinatorError::VersionExhausted => SelectionError::VersionExhausted,
        CoordinatorError::Source(error) => SelectionError::Source(error),
        _ => SelectionError::Unavailable,
    }
}
pub(super) fn owner_error(error: SelectionError) -> CoordinatorError {
    match error {
        SelectionError::Stale => CoordinatorError::Stale,
        SelectionError::Limit => CoordinatorError::Limit,
        SelectionError::Capacity => CoordinatorError::Capacity,
        SelectionError::VersionExhausted => CoordinatorError::VersionExhausted,
        SelectionError::Unavailable => CoordinatorError::Unavailable,
        SelectionError::Source(error) => CoordinatorError::Source(error),
        _ => CoordinatorError::Faulted,
    }
}
fn capture<T>(r: &mut Recorder, boundary: Boundary, result: &Result<T, SelectionError>) {
    capture_view_service(r, boundary, result, None);
}
fn capture_view_service<T>(
    r: &mut Recorder,
    boundary: Boundary,
    result: &Result<T, SelectionError>,
    service_time: Option<std::time::Duration>,
) {
    r.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(SelectionError::Faulted) => Disposition::Unknown,
            _ => Disposition::Refused,
        },
        service_time,
    });
}
impl SourceCoordinator {
    #[cfg(test)]
    pub(super) fn change_view_relation_profile_for_test(&mut self) {
        self.views.limits.as_mut().unwrap().max_live_views += 1;
    }
    pub(super) fn view_relation_profile(&self) -> Option<LogicalViewLimits> {
        self.views.limits
    }
    pub(super) fn validate_view_relation(
        &self,
        command: &RelationCommandV1,
        r: &mut Recorder,
    ) -> Result<(), SelectionError> {
        let limits = self.views.limits.ok_or(SelectionError::Unavailable)?;
        match command {
            RelationCommandV1::CreateView => {
                if self.views.rows.len() >= limits.max_issued_views
                    || self.logical_view_usage().live >= limits.max_live_views
                {
                    return Err(SelectionError::Limit);
                }
                self.check_root_growth(std::mem::size_of::<Option<View>>(), r)
                    .map_err(error)?;
            }
            RelationCommandV1::RetireView { view, expected } => {
                if self.view(view)?.stamp != *expected {
                    return Err(SelectionError::Stale);
                }
            }
            RelationCommandV1::CreateWorkspace | RelationCommandV1::RetireWorkspace { .. } => {
                return Err(SelectionError::Faulted);
            }
        }
        self.next_publication_generation(None).map_err(error)?;
        Ok(())
    }
    pub(super) fn stage_view_relation(
        &mut self,
        command: &RelationCommandV1,
        r: &mut Recorder,
    ) -> Result<ViewRelationStage, SelectionError> {
        self.validate_view_relation(command, r)?;
        let change = match command {
            RelationCommandV1::CreateView => {
                self.views
                    .rows
                    .try_reserve(1)
                    .map_err(|_| SelectionError::Capacity)?;
                ViewRelationChange::Create(LogicalViewId {
                    owner: self.namespace.clone(),
                    slot: self.views.rows.len(),
                })
            }
            RelationCommandV1::RetireView { view, expected } => {
                ViewRelationChange::Retire(view.clone(), *expected)
            }
            RelationCommandV1::CreateWorkspace | RelationCommandV1::RetireWorkspace { .. } => {
                return Err(SelectionError::Faulted);
            }
        };
        Ok(ViewRelationStage {
            owner: self.namespace.clone(),
            before: self.generation,
            publication: self.next_publication_generation(None).map_err(error)?,
            profile: self.views.limits.ok_or(SelectionError::Faulted)?,
            change,
        })
    }
    fn finish_view<T>(
        &mut self,
        r: &mut Recorder,
        boundary: Boundary,
        result: Result<T, SelectionError>,
    ) -> Result<T, SelectionError> {
        if matches!(result, Err(SelectionError::Faulted)) {
            self.faulted = true;
        }
        capture(r, boundary, &result);
        result
    }
    /// One-time native profile activation; no source/view creation or I/O.
    pub fn configure_logical_views(
        &mut self,
        limits: LogicalViewLimits,
        r: &mut Recorder,
    ) -> Result<bool, SelectionError> {
        let result = (|| {
            self.ensure_operational().map_err(error)?;
            if let Some(old) = self.views.limits {
                return if old == limits {
                    Ok(false)
                } else {
                    Err(SelectionError::Unavailable)
                };
            }
            if !self.views.rows.is_empty() {
                return Err(SelectionError::Faulted);
            }
            self.views.limits = Some(limits);
            Ok(true)
        })();
        self.finish_view(r, Boundary::LogicalViewLifecycle, result)
    }
    pub fn logical_view_usage(&self) -> LogicalViewUsage {
        LogicalViewUsage {
            issued_lifetime: self.views.rows.len(),
            live: self.views.rows.iter().flatten().count(),
            attached: self
                .views
                .rows
                .iter()
                .flatten()
                .filter(|v| v.source.is_some())
                .count(),
            selections: self
                .views
                .rows
                .iter()
                .flatten()
                .map(|v| v.selections.selections().len())
                .sum(),
        }
    }
    fn view(&self, id: &LogicalViewId) -> Result<&View, SelectionError> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return Err(SelectionError::Unavailable);
        }
        let view = self
            .views
            .rows
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(SelectionError::Unavailable)?;
        if view.stamp.attachment_generation == 0
            || view.stamp.selection_generation == 0
            || view.stamp.viewport_generation == 0
        {
            return Err(SelectionError::Faulted);
        }
        if let Some(source) = &view.source {
            let document = self
                .validate_version(source)
                .map_err(|_| SelectionError::Faulted)?;
            let normalized = normalize_selections(
                &document.source,
                view.selections.selections(),
                view.selections.primary(),
                self.views
                    .limits
                    .ok_or(SelectionError::Faulted)?
                    .normalization,
            )
            .map_err(|e| {
                if e == SelectionError::Capacity {
                    e
                } else {
                    SelectionError::Faulted
                }
            })?;
            if normalized != view.selections {
                return Err(SelectionError::Faulted);
            }
        } else if !view.selections.selections().is_empty() || view.selections.primary() != 0 {
            return Err(SelectionError::Faulted);
        }
        Ok(view)
    }
    fn view_context(&self, context: &LogicalViewContext) -> Result<&View, SelectionError> {
        let view = self.view(&context.view)?;
        if view.stamp != context.stamp || view.source.as_ref() != Some(&context.source) {
            return Err(SelectionError::Stale);
        }
        self.validate_version(&context.source).map_err(error)?;
        Ok(view)
    }
    pub fn create_logical_view(
        &mut self,
        r: &mut Recorder,
    ) -> Result<LogicalViewId, SelectionError> {
        let result = (|| {
            self.ensure_operational().map_err(error)?;
            let stage = self.stage_view_relation(&RelationCommandV1::CreateView, r)?;
            let RelationReceiptV1::ViewCreated(id) = stage.receipt() else {
                return Err(SelectionError::Faulted);
            };
            stage.install(self, r)?;
            Ok(id)
        })();
        self.finish_view(r, Boundary::LogicalViewLifecycle, result)
    }
    pub fn logical_view(
        &self,
        id: &LogicalViewId,
        r: &mut Recorder,
    ) -> Result<LogicalViewSnapshot, SelectionError> {
        let result = (|| {
            self.ensure_operational().map_err(error)?;
            let view = self.view(id)?;
            if let Some(source) = &view.source {
                self.validate_version(source).map_err(error)?;
            }
            Ok(LogicalViewSnapshot {
                view: id.clone(),
                source: view.source.clone(),
                stamp: view.stamp,
                selections: view.selections.copy()?,
            })
        })();
        capture(r, Boundary::LogicalViewQuery, &result);
        result
    }
    /// Validated borrowed semantic view state; no geometry or new snapshot allocation.
    pub(in crate::coordinator) fn publication_view(
        &self,
        id: &LogicalViewId,
    ) -> Result<(Option<&DocumentVersion>, LogicalViewStamp, &SelectionSet), SelectionError> {
        self.ensure_operational().map_err(error)?;
        let v = self.view(id)?;
        Ok((v.source.as_ref(), v.stamp, &v.selections))
    }
    fn view_selection_capacity(&self, old: usize, new: usize) -> Result<(), SelectionError> {
        let held = self
            .logical_view_usage()
            .selections
            .checked_sub(old)
            .and_then(|n| n.checked_add(new))
            .ok_or(SelectionError::Faulted)?;
        if held
            > self
                .views
                .limits
                .ok_or(SelectionError::Unavailable)?
                .max_total_selections
        {
            Err(SelectionError::Limit)
        } else {
            Ok(())
        }
    }
    pub fn attach_logical_view(
        &mut self,
        id: &LogicalViewId,
        expected: LogicalViewStamp,
        source: &DocumentVersion,
        selections: &[Selection],
        primary: usize,
        r: &mut Recorder,
    ) -> Result<LogicalViewUpdate, SelectionError> {
        self.attach_logical_view_for_invocation(id, expected, source, selections, primary, None, r)
    }
    pub(super) fn validate_live_view_relation(
        &self,
        document: &DocumentId,
        operation: &LiveRelationOperationV1,
    ) -> Result<(), SelectionError> {
        match operation {
            LiveRelationOperationV1::Attach {
                view,
                expected,
                source,
                selections,
                primary,
            } => {
                let v = self.view(view)?;
                if v.stamp != *expected {
                    return Err(SelectionError::Stale);
                }
                if v.source.is_some() || source.document != *document {
                    return Err(SelectionError::Unavailable);
                }
                let d = self.validate_version(source).map_err(error)?;
                normalize_selections(
                    &d.source,
                    selections,
                    *primary,
                    self.views
                        .limits
                        .ok_or(SelectionError::Unavailable)?
                        .normalization,
                )?;
            }
            LiveRelationOperationV1::Detach { view, expected } => {
                let v = self.view(view)?;
                if v.stamp != *expected {
                    return Err(SelectionError::Stale);
                }
                if v.source.as_ref().map(|s| &s.document) != Some(document) {
                    return Err(SelectionError::Unavailable);
                }
            }
            LiveRelationOperationV1::Membership { .. } => return Err(SelectionError::Unavailable),
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn attach_logical_view_for_invocation(
        &mut self,
        id: &LogicalViewId,
        expected: LogicalViewStamp,
        source: &DocumentVersion,
        selections: &[Selection],
        primary: usize,
        invocation: Option<&InvocationId>,
        r: &mut Recorder,
    ) -> Result<LogicalViewUpdate, SelectionError> {
        let timing = r.begin_service_timing();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.ensure_operational().map_err(error)?;
            let view = self.view(id)?;
            if view.stamp != expected {
                return Err(SelectionError::Stale);
            }
            if view.source.is_some() {
                return Err(SelectionError::Unavailable);
            }
            let document = self.validate_version(source).map_err(error)?;
            let input = selections;
            let selections = normalize_selections(
                &document.source,
                selections,
                primary,
                self.views
                    .limits
                    .ok_or(SelectionError::Unavailable)?
                    .normalization,
            )?;
            self.view_selection_capacity(0, selections.selections().len())?;
            self.check_selection_root(&view.selections, &selections, r)?;
            let stamp = expected.relation_next()?;
            let generation = self.next_publication_generation(None).map_err(error)?;
            if let Some(invocation) = invocation {
                self.stage_live_relation_candidate(
                    invocation,
                    invocation::live_relation::Candidate::Attach {
                        view: id,
                        expected,
                        source,
                        selections: input,
                        primary,
                    },
                    true,
                )
                .map_err(|_| SelectionError::Faulted)?;
            }
            self.faulted = true;
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(0),
                "injected view relation interruption"
            );
            self.views.rows[id.slot] = Some(View {
                source: Some(source.clone()),
                stamp,
                selections,
            });
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(1),
                "injected view relation interruption"
            );
            self.generation = generation;
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(2),
                "injected view relation interruption"
            );
            self.faulted = false;
            Ok(LogicalViewUpdate {
                changed: true,
                stamp,
            })
        }));
        let result = match result {
            Ok(v) => v,
            Err(panic) => {
                self.faulted = true;
                capture_view_service(
                    r,
                    Boundary::LogicalViewLifecycle,
                    &Err::<LogicalViewUpdate, _>(SelectionError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(panic)
            }
        };
        if matches!(result, Err(SelectionError::Faulted)) {
            self.faulted = true;
        }
        capture_view_service(r, Boundary::LogicalViewLifecycle, &result, timing.finish());
        result
    }
    pub fn detach_logical_view(
        &mut self,
        id: &LogicalViewId,
        expected: LogicalViewStamp,
        r: &mut Recorder,
    ) -> Result<LogicalViewUpdate, SelectionError> {
        self.detach_logical_view_for_invocation(id, expected, None, r)
    }
    pub(super) fn detach_logical_view_for_invocation(
        &mut self,
        id: &LogicalViewId,
        expected: LogicalViewStamp,
        invocation: Option<&InvocationId>,
        r: &mut Recorder,
    ) -> Result<LogicalViewUpdate, SelectionError> {
        let timing = r.begin_service_timing();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.ensure_operational().map_err(error)?;
            let view = self.view(id)?;
            if view.stamp != expected {
                return Err(SelectionError::Stale);
            }
            if view.source.is_none() {
                if invocation.is_some() {
                    return Err(SelectionError::Faulted);
                }
                return Ok(LogicalViewUpdate {
                    changed: false,
                    stamp: expected,
                });
            }
            self.check_selection_root(&view.selections, &SelectionSet::detached(), r)?;
            let stamp = expected.relation_next()?;
            let generation = self.next_publication_generation(None).map_err(error)?;
            if let Some(invocation) = invocation {
                let source = view.source.clone();
                self.stage_live_relation_candidate(
                    invocation,
                    invocation::live_relation::Candidate::Detach {
                        view: id,
                        expected,
                        source: source.as_ref(),
                    },
                    true,
                )
                .map_err(|_| SelectionError::Faulted)?;
            }
            self.faulted = true;
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(0),
                "injected view relation interruption"
            );
            self.views.rows[id.slot] = Some(View {
                source: None,
                stamp,
                selections: SelectionSet::detached(),
            });
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(1),
                "injected view relation interruption"
            );
            self.generation = generation;
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(2),
                "injected view relation interruption"
            );
            self.faulted = false;
            Ok(LogicalViewUpdate {
                changed: true,
                stamp,
            })
        }));
        let result = match result {
            Ok(v) => v,
            Err(panic) => {
                self.faulted = true;
                capture_view_service(
                    r,
                    Boundary::LogicalViewLifecycle,
                    &Err::<LogicalViewUpdate, _>(SelectionError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(panic)
            }
        };
        if matches!(result, Err(SelectionError::Faulted)) {
            self.faulted = true;
        }
        capture_view_service(r, Boundary::LogicalViewLifecycle, &result, timing.finish());
        result
    }
    pub fn retire_logical_view(
        &mut self,
        id: &LogicalViewId,
        expected: LogicalViewStamp,
        r: &mut Recorder,
    ) -> Result<(), SelectionError> {
        let result = self
            .ensure_operational()
            .map_err(error)
            .and_then(|()| {
                self.stage_view_relation(
                    &RelationCommandV1::RetireView {
                        view: id.clone(),
                        expected,
                    },
                    r,
                )
            })
            .and_then(|stage| stage.install(self, r));
        self.finish_view(r, Boundary::LogicalViewLifecycle, result)
    }
    pub fn set_logical_selections(
        &mut self,
        context: &LogicalViewContext,
        selections: &[Selection],
        primary: usize,
        r: &mut Recorder,
    ) -> Result<LogicalViewUpdate, SelectionError> {
        let result = (|| {
            self.ensure_operational().map_err(error)?;
            self.view_context(context)?;
            let document = self.validate_version(&context.source).map_err(error)?;
            let selections = normalize_selections(
                &document.source,
                selections,
                primary,
                self.views
                    .limits
                    .ok_or(SelectionError::Unavailable)?
                    .normalization,
            )?;
            self.publish_selection_value(context, selections, r)
        })();
        self.finish_view(r, Boundary::LogicalSelectionPublication, result)
    }
    /// Single callback-free publication path for direct and retained selection
    /// commands. Callers supply an already validated canonical selection set.
    fn publish_selection_value(
        &mut self,
        context: &LogicalViewContext,
        selections: SelectionSet,
        r: &mut Recorder,
    ) -> Result<LogicalViewUpdate, SelectionError> {
        let view = self.view_context(context)?;
        self.check_selection_root(&view.selections, &selections, r)?;
        if selections == view.selections {
            return Ok(LogicalViewUpdate {
                changed: false,
                stamp: view.stamp,
            });
        }
        self.view_selection_capacity(
            view.selections.selections().len(),
            selections.selections().len(),
        )?;
        let stamp = LogicalViewStamp {
            selection_generation: next(view.stamp.selection_generation)?,
            ..view.stamp
        };
        let generation = self.next_publication_generation(None).map_err(error)?;
        let view = self.views.rows[context.view.slot]
            .as_mut()
            .expect("validated view");
        view.stamp = stamp;
        view.selections = selections;
        self.generation = generation;
        Ok(LogicalViewUpdate {
            changed: true,
            stamp,
        })
    }
    pub fn move_logical_selections(
        &mut self,
        context: &LogicalViewContext,
        endpoint: LogicalEndpointV1,
        extend: bool,
        budget: CoordinateBudget,
        r: &mut Recorder,
    ) -> Result<LogicalViewUpdate, SelectionError> {
        let result = (|| {
            self.ensure_operational().map_err(error)?;
            let view = self.view_context(context)?;
            let document = self.validate_version(&context.source).map_err(error)?;
            let selections = command::derive_endpoint(
                &document.source,
                &view.selections,
                endpoint,
                extend,
                budget,
            )?;
            let primary = view.selections.primary();
            Ok((selections, primary))
        })();
        capture(r, Boundary::LogicalSelectionPreparation, &result);
        match result {
            Ok((selections, primary)) => {
                self.set_logical_selections(context, &selections, primary, r)
            }
            Err(error) => {
                if error == SelectionError::Faulted {
                    self.faulted = true;
                }
                Err(error)
            }
        }
    }
    fn derived_post_selections(
        &self,
        prepared: &PreparedTransaction,
        before: &ViewDependency,
        candidate: &Candidate,
        remaining: &mut usize,
    ) -> Result<Option<Vec<Selection>>, SelectionError> {
        if !self.literal_carets_after(prepared, before)?
            && !self.line_carets_after(prepared, before)?
            && !self.unicode_carets_after(prepared, before)?
        {
            return Ok(None);
        }
        if candidate.edits.len() != before.selections.selections().len() {
            return Err(SelectionError::Faulted);
        }
        let mut records = Vec::new();
        records
            .try_reserve_exact(candidate.edits.len())
            .map_err(|_| SelectionError::Capacity)?;
        let (mut old_end, mut new_end) = (0, 0usize);
        for (replacement, selection) in candidate.edits.iter().zip(before.selections.selections()) {
            *remaining = remaining.checked_sub(1).ok_or(SelectionError::Limit)?;
            let gap = replacement
                .start
                .checked_sub(old_end)
                .ok_or(SelectionError::Faulted)?;
            let at = new_end
                .checked_add(gap)
                .and_then(|n| n.checked_add(replacement.inserted.len()))
                .ok_or(SelectionError::Faulted)?;
            if !candidate.source.text().is_char_boundary(at) {
                return Err(SelectionError::Faulted);
            }
            records.push(Selection {
                anchor: BytePosition(at),
                head: BytePosition(at),
                affinity: selection.affinity,
            });
            old_end = replacement.end;
            new_end = at;
        }
        Ok(Some(records))
    }
    pub(super) fn capture_view_dependencies(
        &self,
        candidates: &[Candidate],
    ) -> Result<Vec<ViewDependency>, CoordinatorError> {
        let result = (|| {
            let mut dependencies = Vec::new();
            if self.views.limits.is_none() && !self.views.rows.is_empty() {
                return Err(SelectionError::Faulted);
            }
            let affected = self
                .views
                .rows
                .iter()
                .flatten()
                .filter(|view| {
                    view.source.as_ref().is_some_and(|source| {
                        candidates
                            .iter()
                            .any(|candidate| candidate.expected.document == source.document)
                    })
                })
                .count();
            dependencies
                .try_reserve_exact(affected)
                .map_err(|_| SelectionError::Capacity)?;
            for (slot, view) in self
                .views
                .rows
                .iter()
                .enumerate()
                .filter_map(|(slot, view)| view.as_ref().map(|v| (slot, v)))
            {
                let Some(source) = &view.source else {
                    continue;
                };
                if !candidates
                    .iter()
                    .any(|c| c.expected.document == source.document)
                {
                    continue;
                }
                let document = self
                    .validate_version(source)
                    .map_err(|_| SelectionError::Faulted)?;
                let normalized = normalize_selections(
                    &document.source,
                    view.selections.selections(),
                    view.selections.primary(),
                    self.views
                        .limits
                        .ok_or(SelectionError::Faulted)?
                        .normalization,
                )?;
                if normalized != view.selections {
                    return Err(SelectionError::Faulted);
                }
                dependencies.push(ViewDependency {
                    id: LogicalViewId {
                        owner: self.namespace.clone(),
                        slot,
                    },
                    source: source.clone(),
                    stamp: view.stamp,
                    selections: normalized,
                });
            }
            Ok(dependencies)
        })();
        result.map_err(owner_error)
    }
    pub(super) fn stage_view_companions(
        &self,
        prepared: &PreparedTransaction,
        r: &mut Recorder,
    ) -> Result<StagedViews, CoordinatorError> {
        let result = (|| {
            let current = self
                .capture_view_dependencies(&prepared.candidates)
                .map_err(error)?;
            if current.iter().any(|actual| {
                prepared.view_dependencies.iter().any(|before| {
                    before.id == actual.id
                        && before.source == actual.source
                        && before.stamp == actual.stamp
                        && before.selections != actual.selections
                })
            }) {
                return Err(SelectionError::Faulted);
            }
            if current != prepared.view_dependencies {
                return Err(SelectionError::Stale);
            }
            let actual_kind = match prepared.derived_edit.as_slice() {
                [] => None,
                [edit] => Some(edit.kind()),
                _ => return Err(SelectionError::Faulted),
            };
            if prepared.derived_kind != actual_kind {
                return Err(SelectionError::Faulted);
            }
            match prepared.derived_edit.as_slice() {
                [] => {}
                [edit] => match edit {
                    DerivedEdit::Literal(_) => self.verify_literal_edit(prepared)?,
                    DerivedEdit::Line(_) => self.verify_line_edit(prepared)?,
                    DerivedEdit::Unicode(_) => self.verify_unicode_edit(prepared, r)?,
                    DerivedEdit::Composition(_) => self.verify_composition_edit(prepared)?,
                },
                _ => return Err(SelectionError::Faulted),
            }
            let mut updates = Vec::new();
            updates
                .try_reserve_exact(current.len())
                .map_err(|_| SelectionError::Capacity)?;
            let Some(limits) = self.views.limits else {
                return Ok(StagedViews { updates });
            };
            let mut remaining = limits.max_mapping_steps;
            for dependency in current {
                let candidate = prepared
                    .candidates
                    .iter()
                    .find(|c| c.expected.document == dependency.source.document)
                    .ok_or(SelectionError::Faulted)?;
                let original = self.validate_version(&candidate.expected).map_err(error)?;
                let mut records = if let Some(records) =
                    self.derived_post_selections(prepared, &dependency, candidate, &mut remaining)?
                {
                    records
                } else {
                    let mut points = Vec::new();
                    points
                        .try_reserve_exact(
                            dependency
                                .selections
                                .selections()
                                .len()
                                .checked_mul(2)
                                .ok_or(SelectionError::Limit)?,
                        )
                        .map_err(|_| SelectionError::Capacity)?;
                    for selection in dependency.selections.selections() {
                        let (a, h) = if selection.anchor == selection.head {
                            (selection.affinity, selection.affinity)
                        } else if selection.anchor.0 < selection.head.0 {
                            (Affinity::Before, Affinity::After)
                        } else {
                            (Affinity::After, Affinity::Before)
                        };
                        points.push((selection.anchor.0, a));
                        points.push((selection.head.0, h));
                    }
                    let mapped = crate::attachment::map_positions_validated(
                        &original.source,
                        &candidate.source,
                        &candidate.edits,
                        &points,
                        &mut remaining,
                    )
                    .map_err(|e| match e {
                        AttachmentError::Limit => SelectionError::Limit,
                        AttachmentError::Capacity => SelectionError::Capacity,
                        _ => SelectionError::Faulted,
                    })?;
                    let mut records = Vec::new();
                    records
                        .try_reserve_exact(dependency.selections.selections().len())
                        .map_err(|_| SelectionError::Capacity)?;
                    for (selection, points) in dependency
                        .selections
                        .selections()
                        .iter()
                        .zip(mapped.chunks_exact(2))
                    {
                        records.push(Selection {
                            anchor: BytePosition(points[0]),
                            head: BytePosition(points[1]),
                            affinity: selection.affinity,
                        });
                    }
                    records
                };
                self.collapse_composition_primary(
                    prepared,
                    &dependency,
                    candidate,
                    &mut records,
                    &mut remaining,
                )?;
                let selections = normalize_selections(
                    &candidate.source,
                    &records,
                    dependency.selections.primary(),
                    limits.normalization,
                )?;
                let changed = selections != dependency.selections;
                let source_changed = original.source != candidate.source;
                if changed || source_changed {
                    updates.push((
                        dependency.id.slot,
                        View {
                            source: Some(DocumentVersion {
                                document: dependency.source.document,
                                revision: if source_changed {
                                    next(dependency.source.revision)?
                                } else {
                                    dependency.source.revision
                                },
                            }),
                            stamp: LogicalViewStamp {
                                selection_generation: if changed {
                                    next(dependency.stamp.selection_generation)?
                                } else {
                                    dependency.stamp.selection_generation
                                },
                                ..dependency.stamp
                            },
                            selections,
                        },
                    ));
                }
            }
            Ok(StagedViews { updates })
        })();
        if self.views.limits.is_some() {
            capture(r, Boundary::LogicalSelectionMapping, &result);
        }
        result.map_err(owner_error)
    }
    pub(super) fn stage_view_disposal(
        &self,
        document: &DocumentId,
    ) -> Result<StagedViews, CoordinatorError> {
        let result = (|| {
            let mut updates = Vec::new();
            let affected = self
                .views
                .rows
                .iter()
                .flatten()
                .filter(|view| {
                    view.source
                        .as_ref()
                        .is_some_and(|source| source.document == *document)
                })
                .count();
            updates
                .try_reserve_exact(affected)
                .map_err(|_| SelectionError::Capacity)?;
            for (slot, view) in self
                .views
                .rows
                .iter()
                .enumerate()
                .filter_map(|(slot, view)| view.as_ref().map(|view| (slot, view)))
            {
                if view
                    .source
                    .as_ref()
                    .is_some_and(|source| source.document == *document)
                {
                    self.view(&LogicalViewId {
                        owner: self.namespace.clone(),
                        slot,
                    })?;
                    updates.push((
                        slot,
                        View {
                            source: None,
                            stamp: view.stamp.relation_next()?,
                            selections: SelectionSet::detached(),
                        },
                    ));
                }
            }
            Ok(StagedViews { updates })
        })();
        result.map_err(owner_error)
    }
}
