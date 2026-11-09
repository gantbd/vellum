//! Owned completed-read activation. No read dispatch, reload, root injection or UI.
use super::super::settings::basis::{copy_context, labels};
use super::*;
use crate::resource::{AnnotationReadId, ResourceError};
use crate::resource::{AnnotationRestoration, OrdinaryPairCheck, OrdinaryPairEvidence};
use std::mem::size_of;
/// Descriptive in-process roles, not serialized authority or a wire grammar.
pub const fn native_annotation_load_argument_declarations_v1() -> [NativeArgumentDeclarationV1; 2] {
    [
        NativeArgumentDeclarationV1 {
            contract: "engine-annotation-load-declaration/1",
            variant: NativeHandlerVariantDeclarationV1 {
                operation: "restore",
                fields: &[
                    ("package", "OriginalSealedAnnotationRestoration"),
                    ("pair", "OriginalOrdinaryPairEvidence"),
                ],
            },
        },
        NativeArgumentDeclarationV1 {
            contract: "engine-annotation-load-declaration/1",
            variant: NativeHandlerVariantDeclarationV1 {
                operation: "adopt-baseline",
                fields: &[
                    ("read", "OriginalAnnotationReadId"),
                    ("pair", "OriginalOrdinaryPairEvidence"),
                    ("max_baseline_bytes", "NativeBaselineBound"),
                ],
            },
        },
    ]
}
impl AnnotationLoadRequestV1 {
    pub const fn argument_declaration_v1(&self) -> NativeArgumentDeclarationV1 {
        let [restore, baseline] = native_annotation_load_argument_declarations_v1();
        match self {
            Self::Restore { .. } => restore,
            Self::AdoptBaseline { .. } => baseline,
        }
    }
}

pub enum AnnotationLoadRequestV1 {
    /// Original sealed artifact identity; neither a caller-supplied root nor decoded override.
    Restore {
        package: Arc<AnnotationRestoration>,
        pair: Option<OrdinaryPairEvidence>,
    },
    AdoptBaseline {
        read: AnnotationReadId,
        pair: Option<OrdinaryPairEvidence>,
        max_baseline_bytes: usize,
    },
}
// Original identity/roles survive terminal publication without retaining decoded payload.
enum Intent {
    Restore {
        package: std::sync::Weak<AnnotationRestoration>,
        read: AnnotationReadId,
        document: DocumentId,
        pair: Option<OrdinaryPairEvidence>,
    },
    Baseline {
        read: AnnotationReadId,
        pair: Option<OrdinaryPairEvidence>,
        maximum: usize,
    },
}
impl Intent {
    fn capture(q: &AnnotationLoadRequestV1) -> Self {
        match q {
            AnnotationLoadRequestV1::Restore { package, pair } => Self::Restore {
                package: Arc::downgrade(package),
                read: package.capture.origin().clone(),
                document: package.capture.source().document.clone(),
                pair: copy_pair(pair),
            },
            AnnotationLoadRequestV1::AdoptBaseline {
                read,
                pair,
                max_baseline_bytes,
            } => Self::Baseline {
                read: read.clone(),
                pair: copy_pair(pair),
                maximum: *max_baseline_bytes,
            },
        }
    }
    fn same(&self, q: &AnnotationLoadRequestV1) -> bool {
        match (self, q) {
            (
                Self::Restore {
                    package: a,
                    pair: ap,
                    ..
                },
                AnnotationLoadRequestV1::Restore {
                    package: b,
                    pair: bp,
                },
            ) => std::sync::Weak::ptr_eq(a, &Arc::downgrade(b)) && same_pair(ap, bp),
            (
                Self::Baseline {
                    read: a,
                    pair: ap,
                    maximum,
                },
                AnnotationLoadRequestV1::AdoptBaseline {
                    read: b,
                    pair: bp,
                    max_baseline_bytes,
                },
            ) => a == b && maximum == max_baseline_bytes && same_pair(ap, bp),
            _ => false,
        }
    }
    fn belongs_to(&self, owner: &Arc<()>) -> bool {
        let (read, pair) = match self {
            Self::Restore { read, pair, .. } | Self::Baseline { read, pair, .. } => (read, pair),
        };
        Arc::ptr_eq(&read.owner, owner)
            && pair
                .as_ref()
                .is_none_or(|p| Arc::ptr_eq(&p.check.owner, owner))
    }
    fn accepts(&self, out: &AnnotationLoadOutcomeV1) -> bool {
        match (self, out) {
            (
                Self::Restore { document, .. },
                AnnotationLoadOutcomeV1::Restoration { document: d, .. },
            ) => document == d,
            (Self::Baseline { .. }, AnnotationLoadOutcomeV1::Baseline { .. }) => true,
            _ => false,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotationLoadOutcomeV1 {
    Restoration { document: DocumentId, changed: bool },
    Baseline { document: DocumentId, changed: bool },
}
impl AnnotationLoadOutcomeV1 {
    pub fn document(&self) -> &DocumentId {
        match self {
            Self::Restoration { document, .. } | Self::Baseline { document, .. } => document,
        }
    }
    pub fn changed(&self) -> bool {
        match self {
            Self::Restoration { changed, .. } | Self::Baseline { changed, .. } => *changed,
        }
    }
}
enum Prepared {
    Restore(Box<annotation::PreparedAnnotationRestoration>),
    Baseline(Box<resource::PreparedLoadedBaseline>),
}
fn copy_pair(p: &Option<OrdinaryPairEvidence>) -> Option<OrdinaryPairEvidence> {
    p.as_ref().map(|p| OrdinaryPairEvidence {
        relation: p.relation,
        check: OrdinaryPairCheck {
            owner: p.check.owner.clone(),
            marker: p.check.marker.clone(),
            document: p.check.document.clone(),
            source: p.check.source.clone(),
            annotations: p.check.annotations.clone(),
            proposed: p.check.proposed.clone(),
        },
    })
}
fn same_pair(a: &Option<OrdinaryPairEvidence>, b: &Option<OrdinaryPairEvidence>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.relation == b.relation
                && Arc::ptr_eq(&a.check.owner, &b.check.owner)
                && Arc::ptr_eq(&a.check.marker, &b.check.marker)
                && a.check.document == b.check.document
                && a.check.source == b.check.source
                && a.check.annotations == b.check.annotations
                && a.check.proposed == b.check.proposed
        }
        _ => false,
    }
}
impl AnnotationLoadRequestV1 {
    fn capture_bytes(&self) -> Result<usize, InvocationError> {
        let mut n = size_of::<Self>();
        let pair = match self {
            Self::Restore { package, pair } => {
                n = n
                    .checked_add(
                        package
                            .logical_size()
                            .ok_or(InvocationError::Limit)?
                            .checked_mul(2)
                            .ok_or(InvocationError::Limit)?,
                    )
                    .ok_or(InvocationError::Limit)?;
                pair
            }
            Self::AdoptBaseline { pair, .. } => pair,
        };
        if let Some(p) = pair {
            for key in [&p.check.source, &p.check.annotations, &p.check.proposed]
                .into_iter()
                .flatten()
            {
                n = n
                    .checked_add(
                        key.identity
                            .key
                            .0
                            .len()
                            .checked_mul(2)
                            .ok_or(InvocationError::Limit)?,
                    )
                    .ok_or(InvocationError::Limit)?;
            }
        }
        Ok(n)
    }
    fn names_task(&self, id: &AnnotationReadId) -> bool {
        match self {
            Self::Restore { package, .. } => package.capture.origin() == id,
            Self::AdoptBaseline { read, .. } => read == id,
        }
    }
    fn belongs_to(&self, owner: &Arc<()>) -> bool {
        let (id, pair) = match self {
            Self::Restore { package, pair } => (package.capture.origin(), pair),
            Self::AdoptBaseline { read, pair, .. } => (read, pair),
        };
        Arc::ptr_eq(&id.owner, owner)
            && pair
                .as_ref()
                .is_none_or(|p| Arc::ptr_eq(&p.check.owner, owner))
    }
    fn capture(&self) -> Self {
        match self {
            Self::Restore { package, pair } => Self::Restore {
                package: package.clone(),
                pair: copy_pair(pair),
            },
            Self::AdoptBaseline {
                read,
                pair,
                max_baseline_bytes,
            } => Self::AdoptBaseline {
                read: read.clone(),
                pair: copy_pair(pair),
                max_baseline_bytes: *max_baseline_bytes,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnnotationLoadLimitsV1 {
    pub max_capture_bytes: usize,
    pub max_result_bytes: usize,
}
impl AnnotationLoadLimitsV1 {
    /// Logical capsule, sealed proof and service-copy allowance plus context.
    /// Immutable payloads are shared; producer funding/pins remain separately
    /// owned. This is not allocator/RSS accounting.
    pub fn required_capture_bytes(
        c: &SettingsExecutionContextV1,
        request: &AnnotationLoadRequestV1,
    ) -> Result<usize, InvocationError> {
        labels(c).into_iter().try_fold(
            (size_of::<Capture>() + size_of::<Hold>())
                .checked_add(request.capture_bytes()?)
                .ok_or(InvocationError::Limit)?,
            |n, s| n.checked_add(s.len()).ok_or(InvocationError::Limit),
        )
    }
}
/// Current trusted embedding assertions, never requester credentials.
pub struct TrustedAnnotationLoadFactsV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub interpretation_current: Option<bool>,
    pub resources_ready: Option<bool>,
    pub preparation_reserved: Option<bool>,
    pub owner_control_allowed: Option<bool>,
    pub companions_ready: Option<bool>,
    pub reply_ready: Option<bool>,
    pub disposition_readable: Option<bool>,
    pub annotation_writable: Option<bool>,
    pub restoration_allowed: Option<bool>,
    pub identity_restorable: Option<bool>,
    pub source_evidence_ready: Option<bool>,
    pub aliases_current: Option<bool>,
    pub target_evidence_ready: Option<bool>,
    pub history_restoration_ready: Option<bool>,
    pub annotation_readable: Option<bool>,
    pub binding_coverage_complete: Option<bool>,
    pub writer_coverage_complete: Option<bool>,
    pub baseline_publication_allowed: Option<bool>,
    pub historical_artifact_readable: Option<bool>,
    pub baseline_coverage_complete: Option<bool>,
}
impl TrustedAnnotationLoadFactsV1<'_> {
    fn allowed(&self, request: &AnnotationLoadRequestV1) -> bool {
        let common = [
            self.authorized,
            self.scope,
            self.continuous,
            self.live,
            self.not_stopped,
            self.interpretation_current,
            self.resources_ready,
            self.preparation_reserved,
            self.owner_control_allowed,
            self.companions_ready,
            self.reply_ready,
            self.disposition_readable,
        ]
        .into_iter()
        .all(|v| v == Some(true));
        common
            && match request {
                AnnotationLoadRequestV1::Restore { .. } => [
                    self.annotation_writable,
                    self.restoration_allowed,
                    self.identity_restorable,
                    self.source_evidence_ready,
                    self.aliases_current,
                    self.target_evidence_ready,
                    self.history_restoration_ready,
                ]
                .into_iter()
                .all(|v| v == Some(true)),
                AnnotationLoadRequestV1::AdoptBaseline { .. } => [
                    self.annotation_readable,
                    self.binding_coverage_complete,
                    self.writer_coverage_complete,
                    self.baseline_publication_allowed,
                    self.historical_artifact_readable,
                    self.baseline_coverage_complete,
                ]
                .into_iter()
                .all(|v| v == Some(true)),
            }
    }
    fn authority(&self) -> AnnotationLoadAuthorityFactsV1 {
        AnnotationLoadAuthorityFactsV1 {
            owner_control_allowed: self.owner_control_allowed,
            preparation_reserved: self.preparation_reserved,
            resources_ready: self.resources_ready,
        }
    }
}
pub struct AnnotationLoadAdmissionV1<'a> {
    pub facts: &'a TrustedAnnotationLoadFactsV1<'a>,
    pub limits: AnnotationLoadLimitsV1,
    pub publication_epoch: u64,
    pub publication_allowed: Option<bool>,
    pub result_admission_allowed: Option<bool>,
    pub result_allowed: Option<bool>,
}
pub struct TrustedAnnotationLoadResultV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub publication_epoch: u64,
    pub publication_allowed: Option<bool>,
    pub outcome_readable: Option<bool>,
    pub result_allowed: Option<bool>,
    pub result_context_current: Option<bool>,
    pub historical_document_readable: Option<bool>,
    pub reply_ready: Option<bool>,
    pub resources_ready: Option<bool>,
}
/// Historical identity and exact native disposition, never source/currentness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnotationLoadResultV1 {
    pub invocation: InvocationId,
    pub outcome: AnnotationLoadOutcomeV1,
}
pub(super) struct Capture {
    pub(super) owner: Arc<()>,
    pub(super) context: SettingsExecutionContextV1,
    invocation: InvocationId,
    authority: AnnotationLoadAuthorityV1,
    request: Intent,
    request_bytes: usize,
    profile: CoordinatorLimits,
    limits: AnnotationLoadLimitsV1,
    epoch: u64,
    publication: bool,
    admission: bool,
    result: bool,
    marker: Arc<()>,
}
pub(super) struct Hold {
    reservation: retention::Reservation,
    marker: Arc<()>,
    pub(super) candidate: Option<AnnotationLoadOutcomeV1>,
    pub(super) pending: Option<AnnotationLoadRequestV1>,
}
impl std::fmt::Debug for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationLoad(<redacted>)")
    }
}
impl std::fmt::Debug for Hold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationLoadHold(<owned>)")
    }
}
fn parent(e: RetentionErrorV1) -> InvocationError {
    match e {
        RetentionErrorV1::Faulted => InvocationError::Faulted,
        RetentionErrorV1::Limit => InvocationError::Limit,
        RetentionErrorV1::Unavailable => InvocationError::Unavailable,
    }
}
fn resource(e: ResourceError) -> InvocationError {
    match e {
        ResourceError::Faulted => InvocationError::Faulted,
        ResourceError::Capacity => InvocationError::Capacity,
        ResourceError::Limit => InvocationError::Limit,
        _ => InvocationError::Unavailable,
    }
}
shutdown_capture_reservation!();
impl Capture {
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.authority.names_document(document)
            || match &self.request {
                Intent::Restore { document: d, .. } => d == document,
                Intent::Baseline { .. } => false,
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
            && self.invocation == *id
            && self.authority.belongs_to(owner)
            && self.request.belongs_to(owner)
            && labels(&self.context)
                .into_iter()
                .try_fold(size_of::<Capture>() + size_of::<Hold>(), |n, s| {
                    n.checked_add(s.len())
                })
                .and_then(|n| n.checked_add(self.request_bytes))
                .is_some_and(|n| n <= self.limits.max_capture_bytes)
            && h.as_ref().is_some_and(|h| {
                Arc::ptr_eq(&self.marker, &h.marker)
                    && h.pending.as_ref().is_none_or(|q| {
                        self.request.same(q) && q.capture_bytes().ok() == Some(self.request_bytes)
                    })
                    && ledger
                        .validate_exact(&h.reservation, self.limits.max_capture_bytes)
                        .is_ok()
            })
    }
}
impl SourceCoordinator {
    fn prepare_authorized_annotation_load(
        &mut self,
        authority: &AnnotationLoadAuthorityV1,
        request: &AnnotationLoadRequestV1,
        f: &TrustedAnnotationLoadFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<Prepared, ResourceError> {
        if !f.allowed(request) || !request.belongs_to(&self.namespace) {
            return Err(ResourceError::Unavailable);
        }
        let task = self.annotation_load_task_for_invocation(authority, &f.authority(), r)?;
        if !request.names_task(&task) {
            return Err(ResourceError::Unavailable);
        }
        match request {
            AnnotationLoadRequestV1::Restore { package, pair } => Ok(Prepared::Restore(Box::new(
                self.prepare_annotation_restoration_native(
                    package.copy_for_invocation()?,
                    copy_pair(pair),
                    r,
                )?,
            ))),
            AnnotationLoadRequestV1::AdoptBaseline {
                read,
                pair,
                max_baseline_bytes,
            } => Ok(Prepared::Baseline(Box::new(
                self.prepare_loaded_baseline_native(read, copy_pair(pair), *max_baseline_bytes, r)?,
            ))),
        }
    }
    pub fn submit_annotation_load_v1(
        &mut self,
        id: &InvocationId,
        authority: &AnnotationLoadAuthorityV1,
        request: &AnnotationLoadRequestV1,
        a: &AnnotationLoadAdmissionV1<'_>,
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
            if AnnotationLoadLimitsV1::required_capture_bytes(a.facts.context, request)?
                > l.max_capture_bytes
                || l.max_capture_bytes > self.invocations.limits.max_intent_bytes
                || l.max_result_bytes < size_of::<AnnotationLoadResultV1>()
                || l.max_result_bytes > self.invocations.limits.max_intent_bytes
            {
                return Err(InvocationError::Limit);
            }
            if labels(a.facts.context).iter().any(|s| s.is_empty()) || !a.facts.allowed(request) {
                return Err(InvocationError::Unavailable);
            }
            let (Some(publication), Some(admission), Some(result)) = (
                a.publication_allowed,
                a.result_admission_allowed,
                a.result_allowed,
            ) else {
                return Err(InvocationError::Unavailable);
            };
            if let Some(original) = &s.intent {
                let BoundIntent::AnnotationLoad(c) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                if !c.intact(
                    &self.namespace,
                    id,
                    &s.annotation_load_hold,
                    &self.retention,
                ) {
                    return Err(InvocationError::Faulted);
                }
                if !c.request.same(request)
                    || c.authority != *authority
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
            if !matches!(s.phase, Phase::Issued(_)) {
                return Err(InvocationError::Unavailable);
            }
            // Validate current resolution without promising allocation or retaining a root.
            drop(
                self.prepare_authorized_annotation_load(authority, request, a.facts, r)
                    .map_err(resource)?,
            );
            provisional = Some(
                self.retention
                    .reserve(l.max_capture_bytes)
                    .map_err(parent)?,
            );
            let context = copy_context(a.facts.context).map_err(|e| {
                if e == crate::settings::SettingsError::Capacity {
                    InvocationError::Capacity
                } else {
                    InvocationError::Limit
                }
            })?;
            let marker = Arc::new(());
            let c = Arc::new(Capture {
                owner: self.namespace.clone(),
                context,
                invocation: id.clone(),
                authority: authority.capture_for_invocation(),
                request: Intent::capture(request),
                request_bytes: request.capture_bytes()?,
                profile: self.limits,
                limits: l,
                epoch: a.publication_epoch,
                publication,
                admission,
                result,
                marker: marker.clone(),
            });
            self.invocations.slots[id.slot]
                .as_mut()
                .expect("issued load")
                .annotation_load_hold = Some(Hold {
                reservation: provisional.take().expect("funded load"),
                marker,
                candidate: None,
                pending: Some(request.capture()),
            });
            self.install_invocation_admission(
                id,
                BoundIntent::AnnotationLoad(c.clone()),
                Phase::AnnotationLoadPending(c),
            );
            Ok(InvocationState::Pending)
        })();
        if let Some(mut p) = provisional
            && self.retention.release(&mut p).is_err()
        {
            result = Err(InvocationError::Faulted);
        }
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(
            r,
            Boundary::AnnotationLoadAdmission,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn run_annotation_load_v1(
        &mut self,
        id: &InvocationId,
        f: &TrustedAnnotationLoadFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::AnnotationLoad(c)) = &s.intent else {
                return Err(InvocationError::Unavailable);
            };
            if c.context != *f.context || c.profile != self.limits {
                return Err(InvocationError::Unavailable);
            }
            if !c.intact(
                &self.namespace,
                id,
                &s.annotation_load_hold,
                &self.retention,
            ) {
                return Err(InvocationError::Faulted);
            }
            match &s.phase {
                Phase::Terminal(_) => return Ok(InvocationState::Finished),
                Phase::Running => return Err(InvocationError::Faulted),
                Phase::AnnotationLoadPending(p) if Arc::ptr_eq(c, p) => {}
                _ => return Err(InvocationError::Faulted),
            }
            let c = c.clone();
            if self.service_requested_stop(id, r)? {
                return Ok(InvocationState::Finished);
            }
            let request = self
                .invocation_slot(id)?
                .annotation_load_hold
                .as_ref()
                .and_then(|h| h.pending.as_ref())
                .ok_or(InvocationError::Faulted)?
                .capture();
            // Host denial is retryable before service; native refusal is terminal.
            if !f.allowed(&request) {
                return Err(InvocationError::Unavailable);
            }
            self.invocations.slots[id.slot]
                .as_mut()
                .expect("pending load")
                .phase = Phase::Running;
            let result = self
                .prepare_authorized_annotation_load(&c.authority, &request, f, r)
                .and_then(|p| match p {
                    Prepared::Restore(p) => self
                        .commit_annotation_restoration_for_invocation(*p, Some(id), r)
                        .map(|_| ()),
                    Prepared::Baseline(p) => self
                        .commit_loaded_baseline_for_invocation(*p, Some(id), r)
                        .map(|_| ()),
                });
            match result {
                Ok(_) => {} // The native installer settled the terminal under its fence.
                Err(ResourceError::Faulted) => return Err(InvocationError::Faulted),
                Err(e) => self
                    .install_invocation_terminal(id, InvocationOutcome::AnnotationLoadRejected(e)),
            }
            Ok(InvocationState::Finished)
        }));
        let result = match caught {
            Ok(v) => v,
            Err(p) => {
                self.faulted = true;
                capture_timed(
                    r,
                    Boundary::AnnotationLoadService,
                    &Err::<InvocationState, _>(InvocationError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(p)
            }
        };
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(r, Boundary::AnnotationLoadService, &result, timing.finish());
        result
    }
    pub(in crate::coordinator) fn stage_annotation_load_candidate(
        &mut self,
        id: &InvocationId,
        task: &AnnotationReadId,
        outcome: &AnnotationLoadOutcomeV1,
    ) -> Result<InvocationOutcome, ResourceError> {
        let s = self
            .invocation_slot(id)
            .map_err(|_| ResourceError::Faulted)?;
        let Some(BoundIntent::AnnotationLoad(c)) = &s.intent else {
            return Err(ResourceError::Faulted);
        };
        if !matches!(s.phase, Phase::Running)
            || !c.authority.names_task(task)
            || !c.authority.names_document(outcome.document())
            || !c.request.accepts(outcome)
            || !c.intact(
                &self.namespace,
                id,
                &s.annotation_load_hold,
                &self.retention,
            )
            || s.annotation_load_hold
                .as_ref()
                .is_none_or(|h| h.candidate.is_some() || h.pending.is_none())
        {
            return Err(ResourceError::Faulted);
        }
        let candidate = outcome.clone();
        let terminal = InvocationOutcome::AnnotationLoadCommitted(outcome.clone());
        self.invocations.slots[id.slot]
            .as_mut()
            .expect("running load")
            .annotation_load_hold
            .as_mut()
            .expect("funded load")
            .candidate = Some(candidate);
        Ok(terminal)
    }
    pub fn annotation_load_result_v1(
        &self,
        id: &InvocationId,
        f: &TrustedAnnotationLoadResultV1<'_>,
        r: &mut Recorder,
    ) -> Result<AnnotationLoadResultV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::AnnotationLoad(c)) = &s.intent else {
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
                    f.result_allowed,
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
            if !c.intact(
                &self.namespace,
                id,
                &s.annotation_load_hold,
                &self.retention,
            ) {
                return Err(InvocationError::Faulted);
            }
            match &s.phase {
                Phase::Terminal(Some(InvocationOutcome::AnnotationLoadCommitted(v))) => {
                    if s.annotation_load_hold
                        .as_ref()
                        .and_then(|h| h.candidate.as_ref())
                        != Some(v)
                    {
                        return Err(InvocationError::Faulted);
                    }
                    Ok(AnnotationLoadResultV1 {
                        invocation: id.clone(),
                        outcome: v.clone(),
                    })
                }
                Phase::Running => Err(InvocationError::Faulted),
                _ => Err(InvocationError::OutcomeUnavailable),
            }
        })();
        capture_timed(
            r,
            Boundary::AnnotationLoadResultRelease,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn retire_annotation_load_v1(
        &mut self,
        id: &InvocationId,
        f: &InvocationObservationPolicyControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let s = self.invocation_slot(id)?;
            let Some(BoundIntent::AnnotationLoad(c)) = &s.intent else {
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
            if !c.intact(
                &self.namespace,
                id,
                &s.annotation_load_hold,
                &self.retention,
            ) {
                return Err(InvocationError::Faulted);
            }
            if !(matches!(s.phase, Phase::Terminal(_))
                || self.faulted && matches!(s.phase, Phase::Running))
            {
                return Err(InvocationError::Pending);
            }
            let s = self.invocations.slots[id.slot]
                .as_mut()
                .expect("controlled load");
            self.retention
                .release(
                    &mut s
                        .annotation_load_hold
                        .as_mut()
                        .expect("funded load")
                        .reservation,
                )
                .map_err(parent)?;
            s.annotation_load_hold = None;
            s.intent = Some(BoundIntent::RetiredAnnotationLoad);
            Ok(())
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(
            r,
            Boundary::AnnotationLoadRetirement,
            &result,
            timing.finish(),
        );
        result
    }
}
