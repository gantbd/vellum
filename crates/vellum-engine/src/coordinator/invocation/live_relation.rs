//! Owned pure live relations over existing native publishers, not UI or I/O.
use super::super::settings::basis::{copy_context, labels};
use super::*;
use crate::selection::{Selection, SelectionError};
use std::mem::size_of;
#[cfg(test)]
mod tests;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiveRelationOperationV1 {
    Attach {
        view: LogicalViewId,
        expected: LogicalViewStamp,
        source: DocumentVersion,
        selections: Vec<Selection>,
        primary: usize,
    },
    Detach {
        view: LogicalViewId,
        expected: LogicalViewStamp,
    },
    Membership {
        workspace: WorkspaceContext,
        present: bool,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveRelationRequestV1 {
    pub document: DocumentId,
    pub expected_lifecycle: LifecycleState,
    pub operation: LiveRelationOperationV1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveRelationKindV1 {
    Attach,
    Detach,
    Membership,
}
impl LiveRelationRequestV1 {
    fn kind(&self) -> LiveRelationKindV1 {
        match self.operation {
            LiveRelationOperationV1::Attach { .. } => LiveRelationKindV1::Attach,
            LiveRelationOperationV1::Detach { .. } => LiveRelationKindV1::Detach,
            LiveRelationOperationV1::Membership { .. } => LiveRelationKindV1::Membership,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiveRelationLimitsV1 {
    pub max_capture_bytes: usize,
    pub max_result_bytes: usize,
}
impl LiveRelationLimitsV1 {
    pub fn required_capture_bytes(
        c: &SettingsExecutionContextV1,
        q: &LiveRelationRequestV1,
    ) -> Result<usize, InvocationError> {
        let mut b = size_of::<Capture>()
            .checked_add(size_of::<Hold>())
            .ok_or(InvocationError::Limit)?;
        for s in labels(c) {
            b = b.checked_add(s.len()).ok_or(InvocationError::Limit)?;
        }
        if let LiveRelationOperationV1::Attach { selections, .. } = &q.operation {
            b = b
                .checked_add(
                    selections
                        .len()
                        .checked_mul(size_of::<Selection>())
                        .ok_or(InvocationError::Limit)?,
                )
                .ok_or(InvocationError::Limit)?;
        }
        Ok(b)
    }
}
pub struct TrustedLiveRelationFactsV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub interpretation_current: Option<bool>,
    pub resources_ready: Option<bool>,
    pub companions_ready: Option<bool>,
    pub versions_available: Option<bool>,
    pub reply_ready: Option<bool>,
    pub lifecycle_allowed: Option<bool>,
    pub coverage_complete: Option<bool>,
    pub view_writable: Option<bool>,
    pub source_readable: Option<bool>,
    pub membership_writable: Option<bool>,
    pub workspace_readable: Option<bool>,
}
pub struct LiveRelationAdmissionV1<'a> {
    pub facts: &'a TrustedLiveRelationFactsV1<'a>,
    pub limits: LiveRelationLimitsV1,
    pub publication_epoch: u64,
    pub publication_allowed: Option<bool>,
    pub result_admission_allowed: Option<bool>,
    pub result_identifiers_allowed: Option<bool>,
}
pub struct TrustedLiveRelationResultV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub publication_epoch: u64,
    pub publication_allowed: Option<bool>,
    pub outcome_readable: Option<bool>,
    pub result_readable: Option<bool>,
    pub result_context_current: Option<bool>,
    pub historical_document_readable: Option<bool>,
    pub reply_ready: Option<bool>,
    pub resources_ready: Option<bool>,
    pub max_result_bytes: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveRelationResultV1 {
    pub invocation: InvocationId,
    pub document: DocumentId,
    pub operation: LiveRelationKindV1,
    pub changed: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiveRelationErrorV1 {
    Coordinator(CoordinatorError),
    Selection(SelectionError),
}
impl LiveRelationErrorV1 {
    pub(super) fn faulted(&self) -> bool {
        matches!(
            self,
            Self::Coordinator(
                CoordinatorError::Faulted
                    | CoordinatorError::History(HistoryError::Faulted)
                    | CoordinatorError::Resource(crate::resource::ResourceError::Faulted)
            ) | Self::Selection(SelectionError::Faulted)
        )
    }
}
pub(super) struct Capture {
    pub(super) owner: Arc<()>,
    pub(super) context: SettingsExecutionContextV1,
    invocation: InvocationId,
    request: LiveRelationRequestV1,
    limits: LiveRelationLimitsV1,
    epoch: u64,
    publication: bool,
    admission: bool,
    result: bool,
    views: Option<LogicalViewLimits>,
    workspaces: Option<WorkspaceLimits>,
    marker: Arc<()>,
}
pub(super) struct Hold {
    reservation: retention::Reservation,
    marker: Arc<()>,
    pub(super) candidate: Option<bool>,
}
impl std::fmt::Debug for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LiveRelation(<redacted>)")
    }
}
impl std::fmt::Debug for Hold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LiveRelationHold(<owned>)")
    }
}
pub(in crate::coordinator) enum Candidate<'a> {
    Attach {
        view: &'a LogicalViewId,
        expected: LogicalViewStamp,
        source: &'a DocumentVersion,
        selections: &'a [Selection],
        primary: usize,
    },
    Detach {
        view: &'a LogicalViewId,
        expected: LogicalViewStamp,
        source: Option<&'a DocumentVersion>,
    },
    Membership {
        workspace: &'a WorkspaceContext,
        document: &'a DocumentId,
        present: bool,
    },
}
fn parent(e: RetentionErrorV1) -> InvocationError {
    match e {
        RetentionErrorV1::Faulted => InvocationError::Faulted,
        RetentionErrorV1::Limit => InvocationError::Limit,
        RetentionErrorV1::Unavailable => InvocationError::Unavailable,
    }
}
fn allowed(f: &TrustedLiveRelationFactsV1<'_>, kind: LiveRelationKindV1) -> bool {
    [
        f.authorized,
        f.scope,
        f.continuous,
        f.live,
        f.not_stopped,
        f.interpretation_current,
        f.resources_ready,
        f.companions_ready,
        f.versions_available,
        f.reply_ready,
        f.lifecycle_allowed,
        f.coverage_complete,
    ]
    .into_iter()
    .all(|v| v == Some(true))
        && match kind {
            LiveRelationKindV1::Attach => {
                f.view_writable == Some(true) && f.source_readable == Some(true)
            }
            LiveRelationKindV1::Detach => f.view_writable == Some(true),
            LiveRelationKindV1::Membership => {
                f.membership_writable == Some(true) && f.workspace_readable == Some(true)
            }
        }
}
shutdown_capture_reservation!();
impl Capture {
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.request.document == *document
            || match &self.request.operation {
                LiveRelationOperationV1::Attach { source, .. } => source.document == *document,
                LiveRelationOperationV1::Detach { .. }
                | LiveRelationOperationV1::Membership { .. } => false,
            }
    }
    pub(super) fn intact(
        &self,
        owner: &Arc<()>,
        id: &InvocationId,
        h: &Option<Hold>,
        ledger: &retention::RetentionState,
    ) -> bool {
        Arc::ptr_eq(&self.owner, owner)
            && Arc::ptr_eq(&self.request.document.namespace, owner)
            && self.invocation == *id
            && h.as_ref().is_some_and(|h| {
                Arc::ptr_eq(&self.marker, &h.marker)
                    && ledger
                        .validate_exact(&h.reservation, self.limits.max_capture_bytes)
                        .is_ok()
            })
    }
}
impl SourceCoordinator {
    fn validate_live_relation_owner(
        &self,
        q: &LiveRelationRequestV1,
    ) -> Result<(), LiveRelationErrorV1> {
        let d = self
            .document(&q.document)
            .map_err(LiveRelationErrorV1::Coordinator)?;
        if d.lifecycle != q.expected_lifecycle {
            return Err(LiveRelationErrorV1::Coordinator(CoordinatorError::Stale));
        }
        Ok(())
    }
    fn validate_live_relation(&self, q: &LiveRelationRequestV1) -> Result<(), LiveRelationErrorV1> {
        self.validate_live_relation_owner(q)?;
        match &q.operation {
            LiveRelationOperationV1::Attach { .. } | LiveRelationOperationV1::Detach { .. } => self
                .validate_live_view_relation(&q.document, &q.operation)
                .map_err(LiveRelationErrorV1::Selection)?,
            LiveRelationOperationV1::Membership { workspace, .. } => {
                if self
                    .workspace_generation(&workspace.workspace)
                    .map_err(LiveRelationErrorV1::Coordinator)?
                    != workspace.generation
                {
                    return Err(LiveRelationErrorV1::Coordinator(CoordinatorError::Stale));
                }
            }
        }
        Ok(())
    }
    fn validate_live_relation_service(
        &self,
        q: &LiveRelationRequestV1,
    ) -> Result<(), LiveRelationErrorV1> {
        self.validate_live_relation_owner(q)?;
        // Native publishers recheck targeted generations and normalize once at
        // service. Only the owned detach's stricter association needs an extra
        // precondition: the raw trusted detach also accepts an unattached no-op.
        if let LiveRelationOperationV1::Detach { .. } = &q.operation {
            self.validate_live_view_relation(&q.document, &q.operation)
                .map_err(LiveRelationErrorV1::Selection)?;
        }
        Ok(())
    }
    pub fn submit_live_relation_v1(
        &mut self,
        id: &InvocationId,
        q: &LiveRelationRequestV1,
        a: &LiveRelationAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        let timing = r.begin_service_timing();
        let mut provisional = None;
        let mut result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let s = self.invocation_slot(id)?;
            let l = a.limits;
            if l.max_capture_bytes == 0
                || l.max_capture_bytes > self.invocations.limits.max_intent_bytes
                || l.max_result_bytes < size_of::<LiveRelationResultV1>()
                || l.max_result_bytes > self.invocations.limits.max_intent_bytes
                || LiveRelationLimitsV1::required_capture_bytes(a.facts.context, q)?
                    > l.max_capture_bytes
            {
                return Err(InvocationError::Limit);
            }
            if labels(a.facts.context).iter().any(|s| s.is_empty()) || !allowed(a.facts, q.kind()) {
                return Err(InvocationError::Unavailable);
            }
            let (Some(publication), Some(admission), Some(result)) = (
                a.publication_allowed,
                a.result_admission_allowed,
                a.result_identifiers_allowed,
            ) else {
                return Err(InvocationError::Unavailable);
            };
            if let Some(original) = &s.intent {
                let BoundIntent::LiveRelation(c) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                if !c.intact(&self.namespace, id, &s.live_relation_hold, &self.retention) {
                    return Err(InvocationError::Faulted);
                }
                if c.request != *q
                    || c.context != *a.facts.context
                    || c.limits != l
                    || c.epoch != a.publication_epoch
                    || c.publication != publication
                    || c.admission != admission
                    || c.result != result
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return s.phase.state();
            }
            provisional = Some(
                self.retention
                    .reserve(l.max_capture_bytes)
                    .map_err(parent)?,
            );
            self.validate_live_relation(q).map_err(|e| {
                if e.faulted() {
                    InvocationError::Faulted
                } else {
                    InvocationError::Unavailable
                }
            })?;
            let context = copy_context(a.facts.context).map_err(|_| InvocationError::Capacity)?;
            let operation = match &q.operation {
                LiveRelationOperationV1::Attach {
                    view,
                    expected,
                    source,
                    selections,
                    primary,
                } => {
                    let mut owned = Vec::new();
                    owned
                        .try_reserve_exact(selections.len())
                        .map_err(|_| InvocationError::Capacity)?;
                    owned.extend_from_slice(selections);
                    LiveRelationOperationV1::Attach {
                        view: view.clone(),
                        expected: *expected,
                        source: source.clone(),
                        selections: owned,
                        primary: *primary,
                    }
                }
                other => other.clone(),
            };
            let marker = Arc::new(());
            let c = Arc::new(Capture {
                owner: self.namespace.clone(),
                context,
                invocation: id.clone(),
                request: LiveRelationRequestV1 {
                    document: q.document.clone(),
                    expected_lifecycle: q.expected_lifecycle,
                    operation,
                },
                limits: l,
                epoch: a.publication_epoch,
                publication,
                admission,
                result,
                views: if matches!(q.operation, LiveRelationOperationV1::Membership { .. }) {
                    None
                } else {
                    self.views.profile()
                },
                workspaces: if matches!(q.operation, LiveRelationOperationV1::Membership { .. }) {
                    self.workspaces.profile()
                } else {
                    None
                },
                marker: marker.clone(),
            });
            self.invocations.slots[id.slot]
                .as_mut()
                .expect("issued live relation")
                .live_relation_hold = Some(Hold {
                reservation: provisional.take().expect("funded live relation"),
                marker,
                candidate: None,
            });
            self.install_invocation_admission(
                id,
                BoundIntent::LiveRelation(c.clone()),
                Phase::LiveRelationPending(c),
            );
            Ok(InvocationState::Pending)
        })();
        if let Some(mut p) = provisional
            && self.retention.release(&mut p).is_err()
        {
            result = Err(InvocationError::Faulted)
        }
        if result == Err(InvocationError::Faulted) {
            self.faulted = true
        }
        capture_timed(r, Boundary::LiveRelationAdmission, &result, timing.finish());
        result
    }
    pub fn run_live_relation_v1(
        &mut self,
        id: &InvocationId,
        f: &TrustedLiveRelationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::LiveRelation(c)) = &s.intent else {
                return Err(InvocationError::Unavailable);
            };
            if c.context != *f.context {
                return Err(InvocationError::Unavailable);
            }
            if !c.intact(&self.namespace, id, &s.live_relation_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            match &s.phase {
                Phase::Terminal(_) => return Ok(InvocationState::Finished),
                Phase::LiveRelationPending(p) if Arc::ptr_eq(c, p) => {}
                _ => return Err(InvocationError::Faulted),
            }
            let c = c.clone();
            if self.service_requested_stop(id, r)? {
                return Ok(InvocationState::Finished);
            }
            if !allowed(f, c.request.kind())
                || c.views.is_some_and(|v| self.views.profile() != Some(v))
                || c.workspaces
                    .is_some_and(|v| self.workspaces.profile() != Some(v))
            {
                return Err(InvocationError::Unavailable);
            }
            self.invocations.slots[id.slot]
                .as_mut()
                .expect("pending live relation")
                .phase = Phase::Running;
            let outcome = self
                .validate_live_relation_service(&c.request)
                .and_then(|()| match &c.request.operation {
                    LiveRelationOperationV1::Attach {
                        view,
                        expected,
                        source,
                        selections,
                        primary,
                    } => self
                        .attach_logical_view_for_invocation(
                            view,
                            *expected,
                            source,
                            selections,
                            *primary,
                            Some(id),
                            r,
                        )
                        .map(|u| u.changed)
                        .map_err(LiveRelationErrorV1::Selection),
                    LiveRelationOperationV1::Detach { view, expected } => self
                        .detach_logical_view_for_invocation(view, *expected, Some(id), r)
                        .map(|u| u.changed)
                        .map_err(LiveRelationErrorV1::Selection),
                    LiveRelationOperationV1::Membership { workspace, present } => self
                        .set_workspace_membership_for_invocation(
                            workspace,
                            &c.request.document,
                            *present,
                            Some(id),
                            r,
                        )
                        .map(|u| u.changed)
                        .map_err(LiveRelationErrorV1::Coordinator),
                });
            let outcome = match outcome {
                Ok(changed) => {
                    if self
                        .invocation_slot(id)?
                        .live_relation_hold
                        .as_ref()
                        .and_then(|h| h.candidate)
                        != Some(changed)
                    {
                        return Err(InvocationError::Faulted);
                    }
                    InvocationOutcome::LiveRelationCommitted(changed)
                }
                Err(e) if e.faulted() => return Err(InvocationError::Faulted),
                Err(e) => InvocationOutcome::LiveRelationRejected(e),
            };
            self.install_invocation_terminal(id, outcome);
            Ok(InvocationState::Finished)
        }));
        let result = match caught {
            Ok(v) => v,
            Err(p) => {
                self.faulted = true;
                capture_timed(
                    r,
                    Boundary::LiveRelationService,
                    &Err::<InvocationState, _>(InvocationError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(p)
            }
        };
        if result == Err(InvocationError::Faulted) {
            self.faulted = true
        }
        capture_timed(r, Boundary::LiveRelationService, &result, timing.finish());
        result
    }
    pub(in crate::coordinator) fn stage_live_relation_candidate(
        &mut self,
        id: &InvocationId,
        candidate: Candidate<'_>,
        changed: bool,
    ) -> Result<(), InvocationError> {
        let s = self.invocation_slot(id)?;
        let Some(BoundIntent::LiveRelation(c)) = &s.intent else {
            return Err(InvocationError::Faulted);
        };
        if !matches!(s.phase, Phase::Running)
            || !c.intact(&self.namespace, id, &s.live_relation_hold, &self.retention)
            || s.live_relation_hold
                .as_ref()
                .is_none_or(|h| h.candidate.is_some())
        {
            return Err(InvocationError::Faulted);
        }
        let valid = match (&c.request.operation, candidate) {
            (
                LiveRelationOperationV1::Attach {
                    view,
                    expected,
                    source,
                    selections,
                    primary,
                },
                Candidate::Attach {
                    view: v,
                    expected: e,
                    source: t,
                    selections: ss,
                    primary: p,
                },
            ) => {
                view == v
                    && expected == &e
                    && source == t
                    && source.document == c.request.document
                    && selections == ss
                    && primary == &p
                    && changed
            }
            (
                LiveRelationOperationV1::Detach { view, expected },
                Candidate::Detach {
                    view: v,
                    expected: e,
                    source,
                },
            ) => {
                view == v
                    && expected == &e
                    && source.map(|s| &s.document) == Some(&c.request.document)
                    && changed
            }
            (
                LiveRelationOperationV1::Membership { workspace, present },
                Candidate::Membership {
                    workspace: w,
                    document,
                    present: p,
                },
            ) => workspace == w && present == &p && document == &c.request.document,
            _ => false,
        };
        if !valid
            || self
                .document(&c.request.document)
                .map_err(|_| InvocationError::Faulted)?
                .lifecycle
                != c.request.expected_lifecycle
        {
            return Err(InvocationError::Faulted);
        }
        self.invocations.slots[id.slot]
            .as_mut()
            .expect("running live relation")
            .live_relation_hold
            .as_mut()
            .expect("funded live relation")
            .candidate = Some(changed);
        Ok(())
    }
    pub fn live_relation_result_v1(
        &self,
        id: &InvocationId,
        f: &TrustedLiveRelationResultV1<'_>,
        r: &mut Recorder,
    ) -> Result<LiveRelationResultV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::LiveRelation(c)) = &s.intent else {
                return Err(InvocationError::OutcomeUnavailable);
            };
            if c.context != *f.context
                || c.epoch != f.publication_epoch
                || !c.publication
                || !c.admission
                || !c.result
                || [
                    f.publication_allowed,
                    f.outcome_readable,
                    f.result_readable,
                    f.result_context_current,
                    f.historical_document_readable,
                    f.reply_ready,
                    f.resources_ready,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(InvocationError::Unavailable);
            }
            if f.max_result_bytes < size_of::<LiveRelationResultV1>() {
                return Err(InvocationError::Limit);
            }
            if !c.intact(&self.namespace, id, &s.live_relation_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            let Phase::Terminal(Some(InvocationOutcome::LiveRelationCommitted(changed))) = &s.phase
            else {
                return Err(InvocationError::OutcomeUnavailable);
            };
            if s.live_relation_hold.as_ref().and_then(|h| h.candidate) != Some(*changed) {
                return Err(InvocationError::Faulted);
            }
            Ok(LiveRelationResultV1 {
                invocation: id.clone(),
                document: c.request.document.clone(),
                operation: c.request.kind(),
                changed: *changed,
            })
        })();
        capture_timed(
            r,
            Boundary::LiveRelationResultRelease,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn retire_live_relation_v1(
        &mut self,
        id: &InvocationId,
        f: &InvocationObservationPolicyControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::LiveRelation(c)) = &s.intent else {
                return Err(InvocationError::Unavailable);
            };
            if c.context != *f.context
                || [
                    f.owner_control_allowed,
                    f.retention_released,
                    f.delivery_drained,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(InvocationError::Unavailable);
            }
            if !c.intact(&self.namespace, id, &s.live_relation_hold, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            if !(matches!(s.phase, Phase::Terminal(_))
                || self.faulted && matches!(s.phase, Phase::Running))
            {
                return Err(InvocationError::Pending);
            }
            let s = self.invocations.slots[id.slot]
                .as_mut()
                .expect("controlled live relation");
            self.retention
                .release(
                    &mut s
                        .live_relation_hold
                        .as_mut()
                        .expect("funded live relation")
                        .reservation,
                )
                .map_err(parent)?;
            s.live_relation_hold = None;
            s.intent = Some(BoundIntent::RetiredLiveRelation);
            Ok(())
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true
        }
        capture_timed(
            r,
            Boundary::LiveRelationRetirement,
            &result,
            timing.finish(),
        );
        result
    }
}
