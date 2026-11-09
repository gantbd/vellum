//! Authored records at the trusted owner boundary, not recipient-safe projections.
use super::*;
mod shutdown_inventory;
pub use projection::{
    CollectionFacetCeilingV1, CollectionVisibilityCaptureV1, CollectionVisibilityPreparationIdV1,
    TrustedCollectionVisibilityFactsV1,
};
mod declaration;
mod plan_validation;
mod reversal_validation;
pub use declaration::native_annotation_authoring_declaration_v1;
pub(super) mod projection;
mod restoration;
pub(in crate::coordinator) use restoration::PreparedAnnotationRestoration;
mod reclamation;
mod retirement;
pub use retirement::TrustedAnnotationRetirementV1;
mod root_accounting;
use crate::attachment::{AttachmentTarget, LocationEvidence, Placement};

/// Borrow-only private candidate. No conversion into a publishable transaction.
pub(super) struct AdviceCandidate(PreparedTransaction);
impl AdviceCandidate {
    pub(super) fn prepared(&self) -> &PreparedTransaction {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnnotationLimits {
    pub max_issued: usize,
    /// Present + retained-absent records, not only currently displayed comments.
    /// Retired tombstones release this bound, never lifetime issuance.
    pub max_records: usize,
    pub max_body_bytes: usize,
    pub max_retained_body_bytes: usize,
    pub max_mapping_steps: usize,
    pub max_live_snapshots: usize,
    /// Logical source/record/string bytes per retained capture, not allocator peak.
    pub max_snapshot_bytes: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnnotationUsage {
    pub issued_lifetime: usize,
    pub retained_records: usize,
    pub present_records: usize,
    pub body_bytes: usize,
    pub retained_snapshots: usize,
}
/// Trusted current-document metadata; no source/body copy or snapshot loan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnnotationDocumentSummaryV1 {
    pub set_version: u64,
    pub present_records: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationError {
    Unavailable,
    Stale,
    Permission,
    Limit,
    Capacity,
    InvalidTarget,
    Retained,
    VersionExhausted,
    Faulted,
}
#[derive(Clone)]
pub struct AnnotationId {
    pub(super) namespace: Arc<()>,
    pub(super) slot: usize,
    pub(super) document: DocumentId,
}
impl PartialEq for AnnotationId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.namespace, &other.namespace)
            && self.slot == other.slot
            && self.document == other.document
    }
}
impl Eq for AnnotationId {}
impl std::fmt::Debug for AnnotationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationId(<opaque>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetBasis {
    Before,
    PreparedSource,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceTarget {
    pub source: DocumentVersion,
    pub target: AttachmentTarget,
    pub basis: TargetBasis,
}
#[derive(Clone, PartialEq, Eq)]
pub enum AnnotationMutation {
    Create {
        target: SourceTarget,
        body: String,
    },
    UpdateBody {
        annotation: AnnotationId,
        expected_content: u64,
        body: String,
    },
    Reattach {
        annotation: AnnotationId,
        expected_target: u64,
        target: SourceTarget,
    },
    Delete {
        annotation: AnnotationId,
        expected_content: u64,
        expected_target: u64,
    },
}
impl std::fmt::Debug for AnnotationMutation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationMutation(<redacted>)")
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnotationFact {
    pub annotation: AnnotationId,
    pub present: bool,
    pub content_version: u64,
    pub target_generation: u64,
    pub placement_generation: u64,
}
pub struct AnnotationRead<'a> {
    pub restoration_identity: Option<&'a crate::resource::RestorationIdentity>,
    pub body: &'a str,
    pub content_version: u64,
    pub target_generation: u64,
    pub placement_generation: u64,
    pub authored: &'a SourceTarget,
    pub placement: Placement,
    pub placement_revision: Option<u64>,
}
impl std::fmt::Debug for AnnotationRead<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationRead(<redacted>)")
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TargetState {
    authored: SourceTarget,
    placement: Placement,
    revision: Option<u64>,
}
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Authored {
    body: Arc<str>,
    target: TargetState,
}
impl std::fmt::Debug for Authored {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Authored(<redacted>)")
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Value {
    restored_identity: Option<crate::resource::RestorationIdentity>,
    pub(super) id: AnnotationId,
    body: Arc<str>,
    content: u64,
    target_generation: u64,
    target: TargetState,
    placement_generation: u64,
    pub(super) present: bool,
}
impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationValue(<redacted>)")
    }
}
impl Value {
    fn authored(&self) -> Authored {
        Authored {
            body: self.body.clone(),
            target: self.target.clone(),
        }
    }
    fn fact(&self) -> AnnotationFact {
        AnnotationFact {
            annotation: self.id.clone(),
            present: self.present,
            content_version: self.content,
            target_generation: self.target_generation,
            placement_generation: self.placement_generation,
        }
    }
}
#[derive(Debug)]
pub(super) struct Record {
    pub(super) value: Value,
    pub(super) history: history::Frontiers,
}
/// Issued identity/floors only. No authored body, target or mapping payload.
#[derive(Debug)]
struct RetiredAnnotation {
    fact: AnnotationFact,
    restored_identity: Option<crate::resource::RestorationIdentity>,
    /// Whole-owner retirement preserves the last membership fact, not an
    /// invented authored delete transition on a disposed document.
    document_reclaimed: bool,
}
#[derive(Debug, Default)]
pub(super) struct AnnotationState {
    pub(super) limits: Option<AnnotationLimits>,
    pub(super) records: Vec<Option<Record>>,
    retired: Vec<Option<RetiredAnnotation>>,
    issued_documents: Vec<DocumentId>,
    snapshots: Vec<std::sync::Weak<SnapshotData>>,
}
impl AnnotationState {
    pub(super) fn document_snapshot_pins(&self, document: &DocumentId) -> usize {
        self.snapshots
            .iter()
            .filter_map(std::sync::Weak::upgrade)
            .filter(|snapshot| snapshot.source.document == *document)
            .count()
    }
}
#[derive(Clone)]
pub struct AnnotationSnapshotRecord {
    restored_identity: Option<crate::resource::RestorationIdentity>,
    pub annotation: AnnotationId,
    pub content_version: u64,
    pub target_generation: u64,
    body: Arc<str>,
    pub authored: SourceTarget,
    pub placement: Placement,
    pub placement_revision: Option<u64>,
}
impl std::fmt::Debug for AnnotationSnapshotRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationSnapshotRecord(<redacted>)")
    }
}
impl AnnotationSnapshotRecord {
    pub fn restoration_identity(&self) -> Option<&crate::resource::RestorationIdentity> {
        self.restored_identity.as_ref()
    }
    pub fn body(&self) -> &str {
        &self.body
    }
}
struct SnapshotData {
    source: DocumentVersion,
    source_text: String,
    set: u64,
    publication: u64,
    records: Vec<AnnotationSnapshotRecord>,
}
/// Complete immutable trusted-holder disclosure. Not a recipient grant, durable
/// identity, live annotation edit token or proof that an encoder wrote anything.
#[derive(Clone)]
pub struct AnnotationSnapshot {
    inner: Arc<SnapshotData>,
}
impl std::fmt::Debug for AnnotationSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationSnapshot(<redacted>)")
    }
}
impl AnnotationSnapshot {
    pub(crate) fn logical_bytes(&self) -> Option<usize> {
        let mut n = std::mem::size_of::<SnapshotData>().checked_add(self.source_text().len())?;
        for r in self.records() {
            n = n
                .checked_add(std::mem::size_of::<AnnotationSnapshotRecord>())?
                .checked_add(r.body.len())?
                .checked_add(r.restored_identity.as_ref().map_or(0, |i| i.key().len()))?;
        }
        Some(n)
    }
    pub(crate) fn belongs_to(&self, owner: &Arc<()>) -> bool {
        Arc::ptr_eq(&self.inner.source.document.namespace, owner)
    }
    pub fn source(&self) -> &DocumentVersion {
        &self.inner.source
    }
    pub fn source_text(&self) -> &str {
        &self.inner.source_text
    }
    pub fn set_version(&self) -> u64 {
        self.inner.set
    }
    pub fn publication_generation(&self) -> u64 {
        self.inner.publication
    }
    pub fn records(&self) -> &[AnnotationSnapshotRecord] {
        &self.inner.records
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(super) enum Wish {
    Create {
        body: Arc<str>,
        target: SourceTarget,
    },
    Body {
        expected: u64,
        body: Arc<str>,
    },
    Target {
        expected: u64,
        target: SourceTarget,
    },
    Delete {
        content: u64,
        target: u64,
    },
    RestoreBody {
        expected: u64,
        body: Arc<str>,
    },
    RestoreTarget {
        expected: u64,
        state: TargetState,
        source: DocumentVersion,
        linked: bool,
    },
    RestoreMembership {
        content: u64,
        target: u64,
        present: bool,
        desired: Option<Authored>,
        source: DocumentVersion,
        linked: bool,
    },
}
#[derive(Clone, PartialEq, Eq)]
pub(super) struct AnnotationPlan {
    pub(super) id: AnnotationId,
    wish: Wish,
    policy: u64,
}
impl std::fmt::Debug for AnnotationPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationPlan(<redacted>)")
    }
}
#[derive(Clone, Debug)]
pub(super) enum Delta {
    Membership {
        before: Option<Authored>,
        after: Option<Authored>,
    },
    Fields {
        body: Option<(Arc<str>, Arc<str>)>,
        target: Option<(TargetState, TargetState)>,
    },
}
// A custom journal Debug avoids formatting body pairs in Fields.
#[derive(Clone)]
pub(super) struct AnnotationJournal {
    pub(super) id: AnnotationId,
    delta: Delta,
}
impl std::fmt::Debug for AnnotationJournal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationJournal(<redacted>)")
    }
}
impl AnnotationJournal {
    pub(super) fn charge(&self) -> Result<usize, HistoryError> {
        let payload = match &self.delta {
            Delta::Membership { before, after } => before
                .iter()
                .chain(after)
                .try_fold(0usize, |n, a| n.checked_add(a.body.len())),
            Delta::Fields { body, .. } => body
                .as_ref()
                .map_or(Some(0), |(a, b)| a.len().checked_add(b.len())),
        }
        .ok_or(HistoryError::Limit)?;
        std::mem::size_of::<Self>()
            .checked_add(std::mem::size_of::<Authored>() * 2)
            .and_then(|n| n.checked_add(payload))
            .ok_or(HistoryError::Limit)
    }
}
pub(super) struct AnnotationDecision {
    updates: Vec<Value>,
    sets: Vec<(DocumentId, u64)>,
    pub(super) journals: Vec<AnnotationJournal>,
    pub(super) facts: Vec<AnnotationFact>,
}
impl AnnotationDecision {
    pub(super) fn component_fact(
        &self,
        owner: &SourceCoordinator,
        id: &AnnotationId,
    ) -> Result<AnnotationFact, CoordinatorError> {
        let mut matching = self.updates.iter().filter(|v| v.id == *id);
        let fact = matching.next().map(Value::fact).or_else(|| {
            owner
                .annotations
                .records
                .get(id.slot)?
                .as_ref()
                .map(|r| r.value.fact())
        });
        if matching.next().is_some() {
            return Err(CoordinatorError::Faulted);
        }
        fact.filter(|f| f.annotation == *id)
            .ok_or(CoordinatorError::Faulted)
    }
    /// Private scalar decision facts, captured before installation. No authored
    /// snapshot or later-row reconstruction is retained by the receipt.
    pub(super) fn standalone_fact(
        &self,
        owner: &SourceCoordinator,
        id: &AnnotationId,
    ) -> Result<AnnotationFact, CoordinatorError> {
        if self.updates.len() > 1 || self.updates.iter().any(|v| v.id != *id) {
            return Err(CoordinatorError::Faulted);
        }
        self.updates
            .first()
            .map(Value::fact)
            .or_else(|| {
                owner
                    .annotations
                    .records
                    .get(id.slot)?
                    .as_ref()
                    .map(|r| r.value.fact())
            })
            .filter(|f| f.annotation == *id)
            .ok_or(CoordinatorError::Faulted)
    }
    pub(super) fn changed(&self) -> bool {
        !self.updates.is_empty()
    }
    pub(super) fn install(self, owner: &mut SourceCoordinator) {
        for value in self.updates {
            let slot = value.id.slot;
            if let Some(record) = &mut owner.annotations.records[slot] {
                record.value = value;
            } else {
                owner.annotations.records[slot] = Some(Record {
                    value,
                    history: history::Frontiers::default(),
                });
            }
        }
        for (document, version) in self.sets {
            owner.documents[document.slot]
                .as_mut()
                .expect("validated annotation document")
                .annotation_generation = version;
        }
    }
}
fn copy_body(body: &str, limits: AnnotationLimits) -> Result<Arc<str>, CoordinatorError> {
    if body.len() > limits.max_body_bytes {
        return Err(CoordinatorError::Annotation(AnnotationError::Limit));
    }
    let mut text = String::new();
    text.try_reserve_exact(body.len())
        .map_err(|_| CoordinatorError::Capacity)?;
    text.push_str(body);
    Ok(Arc::from(text))
}
fn next(version: u64) -> Result<u64, CoordinatorError> {
    version
        .checked_add(1)
        .ok_or(CoordinatorError::VersionExhausted)
}
fn mapping_error(error: crate::attachment::AttachmentError) -> CoordinatorError {
    match error {
        crate::attachment::AttachmentError::Limit => CoordinatorError::Limit,
        crate::attachment::AttachmentError::Capacity => CoordinatorError::Capacity,
        _ => CoordinatorError::Annotation(AnnotationError::InvalidTarget),
    }
}
impl PreparedTransaction {
    pub fn created_annotations(&self) -> impl Iterator<Item = &AnnotationId> {
        self.annotations
            .iter()
            .filter(|p| matches!(p.wish, Wish::Create { .. }))
            .map(|p| &p.id)
    }
}
impl SourceCoordinator {
    pub(super) fn annotation_dependency_versions(
        &self,
        id: &AnnotationId,
    ) -> Result<(u64, u64), CoordinatorError> {
        let value = &self.annotation_record(id)?.value;
        if !value.present {
            return Err(CoordinatorError::Unavailable);
        }
        Ok((value.content, value.target_generation))
    }
    pub(super) fn knows_annotation_snapshot(&self, snapshot: &AnnotationSnapshot) -> bool {
        snapshot.belongs_to(&self.namespace)
            && self
                .annotations
                .snapshots
                .iter()
                .any(|s| s.ptr_eq(&Arc::downgrade(&snapshot.inner)))
    }
    /// Trusted-owner resource facts, not disclosure grants or default labels.
    pub fn annotation_usage(&self) -> AnnotationUsage {
        let mut usage = AnnotationUsage {
            issued_lifetime: self.annotations.records.len(),
            retained_records: 0,
            present_records: 0,
            body_bytes: 0,
            retained_snapshots: self
                .annotations
                .snapshots
                .iter()
                .filter(|s| s.strong_count() > 0)
                .count(),
        };
        for record in self.annotations.records.iter().flatten() {
            usage.retained_records += 1;
            usage.present_records += usize::from(record.value.present);
            usage.body_bytes += record.value.body.len();
        }
        usage
    }
    pub fn annotation_document_summary_v1(
        &self,
        document: &DocumentId,
        recorder: &mut Recorder,
    ) -> Result<AnnotationDocumentSummaryV1, CoordinatorError> {
        let result = (|| {
            self.ensure_operational()?;
            if self.annotations.limits.is_none() {
                return Err(CoordinatorError::Unavailable);
            }
            let d = self.document(document)?;
            Ok(AnnotationDocumentSummaryV1 {
                set_version: d.annotation_generation,
                present_records: self
                    .annotations
                    .records
                    .iter()
                    .flatten()
                    .filter(|r| r.value.present && r.value.id.document == *document)
                    .count(),
            })
        })();
        observe(recorder, Boundary::AnnotationQuery, &result);
        result
    }
    /// Bounded current membership/version facts, not a body/source snapshot,
    /// mutable edit token or recipient-authorized collection projection.
    pub fn annotation_document_facts_v1(
        &self,
        source: &DocumentVersion,
        expected_set: u64,
        recorder: &mut Recorder,
    ) -> Result<Vec<AnnotationFact>, CoordinatorError> {
        let result = (|| {
            self.ensure_operational()?;
            if self.annotations.limits.is_none() {
                return Err(CoordinatorError::Unavailable);
            }
            if self.validate_version(source)?.annotation_generation != expected_set {
                return Err(CoordinatorError::Stale);
            }
            let rows = || {
                self.annotations
                    .records
                    .iter()
                    .flatten()
                    .filter(|r| r.value.present && r.value.id.document == source.document)
            };
            let mut facts = Vec::new();
            facts
                .try_reserve_exact(rows().count())
                .map_err(|_| CoordinatorError::Capacity)?;
            facts.extend(rows().map(|r| r.value.fact()));
            Ok(facts)
        })();
        observe(recorder, Boundary::AnnotationQuery, &result);
        result
    }
    pub fn capture_annotations(
        &mut self,
        source: &DocumentVersion,
        expected_set: u64,
        recorder: &mut Recorder,
    ) -> Result<AnnotationSnapshot, CoordinatorError> {
        let result = (|| {
            self.ensure_ordinary_admission()?;
            let document = self.validate_version(source)?;
            if document.annotation_generation != expected_set {
                return Err(CoordinatorError::Stale);
            }
            let limits = self
                .annotations
                .limits
                .ok_or(CoordinatorError::Unavailable)?;
            let slot = self
                .annotations
                .snapshots
                .iter()
                .position(|s| s.strong_count() == 0)
                .unwrap_or(self.annotations.snapshots.len());
            if slot >= limits.max_live_snapshots {
                return Err(CoordinatorError::Limit);
            }
            let count = self
                .annotations
                .records
                .iter()
                .flatten()
                .filter(|r| r.value.present && r.value.id.document == source.document)
                .count();
            let mut bytes = std::mem::size_of::<SnapshotData>()
                .checked_add(document.source.text().len())
                .ok_or(CoordinatorError::Limit)?;
            for record in self
                .annotations
                .records
                .iter()
                .flatten()
                .filter(|r| r.value.present && r.value.id.document == source.document)
            {
                bytes = bytes
                    .checked_add(std::mem::size_of::<AnnotationSnapshotRecord>())
                    .and_then(|n| n.checked_add(record.value.body.len()))
                    .and_then(|n| {
                        n.checked_add(
                            record
                                .value
                                .restored_identity
                                .as_ref()
                                .map_or(0, |i| i.key().len()),
                        )
                    })
                    .ok_or(CoordinatorError::Limit)?;
            }
            if bytes > limits.max_snapshot_bytes {
                return Err(CoordinatorError::Limit);
            }
            let mut text = String::new();
            text.try_reserve_exact(document.source.text().len())
                .map_err(|_| CoordinatorError::Capacity)?;
            text.push_str(document.source.text());
            let mut records = Vec::new();
            records
                .try_reserve_exact(count)
                .map_err(|_| CoordinatorError::Capacity)?;
            for record in self
                .annotations
                .records
                .iter()
                .flatten()
                .filter(|r| r.value.present && r.value.id.document == source.document)
            {
                let value = &record.value;
                records.push(AnnotationSnapshotRecord {
                    restored_identity: value.restored_identity.clone(),
                    annotation: value.id.clone(),
                    content_version: value.content,
                    target_generation: value.target_generation,
                    body: value.body.clone(),
                    authored: value.target.authored.clone(),
                    placement: value.target.placement,
                    placement_revision: value.target.revision,
                });
            }
            let inner = Arc::new(SnapshotData {
                source: source.clone(),
                source_text: text,
                set: expected_set,
                publication: self.generation,
                records,
            });
            if slot == self.annotations.snapshots.len() {
                self.annotations
                    .snapshots
                    .try_reserve(1)
                    .map_err(|_| CoordinatorError::Capacity)?;
                self.annotations.snapshots.push(Arc::downgrade(&inner));
            } else {
                self.annotations.snapshots[slot] = Arc::downgrade(&inner);
            }
            Ok(AnnotationSnapshot { inner })
        })();
        observe(recorder, Boundary::AnnotationCapture, &result);
        result
    }
    pub(super) fn annotation_record(&self, id: &AnnotationId) -> Result<&Record, CoordinatorError> {
        if !Arc::ptr_eq(&self.namespace, &id.namespace) {
            return Err(CoordinatorError::Unavailable);
        }
        self.document(&id.document)?;
        self.annotations
            .records
            .get(id.slot)
            .and_then(Option::as_ref)
            .filter(|r| r.value.id == *id)
            .ok_or(CoordinatorError::Unavailable)
    }
    pub fn annotation(
        &self,
        id: &AnnotationId,
        recorder: &mut Recorder,
    ) -> Result<AnnotationRead<'_>, CoordinatorError> {
        let result = (|| {
            self.ensure_operational()?;
            let value = &self.annotation_record(id)?.value;
            if !value.present {
                return Err(CoordinatorError::Unavailable);
            }
            Ok(AnnotationRead {
                restoration_identity: value.restored_identity.as_ref(),
                body: &value.body,
                content_version: value.content,
                target_generation: value.target_generation,
                placement_generation: value.placement_generation,
                authored: &value.target.authored,
                placement: value.target.placement,
                placement_revision: value.target.revision,
            })
        })();
        observe(recorder, Boundary::AnnotationQuery, &result);
        result
    }
    /// Borrow actual named membership/payload without collection enumeration.
    /// A retired authored-absent identity stays absent; a foreign identity is not absence.
    pub(in crate::coordinator) fn publication_annotation(
        &self,
        id: &AnnotationId,
    ) -> Result<Option<AnnotationRead<'_>>, CoordinatorError> {
        self.ensure_operational()?;
        if !Arc::ptr_eq(&self.namespace, &id.namespace) {
            return Err(CoordinatorError::Unavailable);
        }
        self.document(&id.document)?;
        if let Some(record) = self
            .annotations
            .records
            .get(id.slot)
            .and_then(Option::as_ref)
        {
            if record.value.id != *id {
                return Err(CoordinatorError::Faulted);
            }
            if !record.value.present {
                return Ok(None);
            }
            let v = &record.value;
            return Ok(Some(AnnotationRead {
                restoration_identity: v.restored_identity.as_ref(),
                body: &v.body,
                content_version: v.content,
                target_generation: v.target_generation,
                placement_generation: v.placement_generation,
                authored: &v.target.authored,
                placement: v.target.placement,
                placement_revision: v.target.revision,
            }));
        }
        if self
            .annotations
            .retired
            .get(id.slot)
            .and_then(Option::as_ref)
            .is_some_and(|r| r.fact.annotation == *id && !r.fact.present && !r.document_reclaimed)
        {
            return Ok(None);
        }
        Err(CoordinatorError::Unavailable)
    }
    pub fn annotation_set_version(
        &self,
        document: &DocumentId,
        recorder: &mut Recorder,
    ) -> Result<u64, CoordinatorError> {
        let result = self
            .ensure_operational()
            .and_then(|()| self.document(document))
            .map(|d| d.annotation_generation);
        observe(recorder, Boundary::AnnotationQuery, &result);
        result
    }
    pub fn set_annotations_read_only(
        &mut self,
        document: &DocumentId,
        read_only: bool,
        recorder: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let result = (|| {
            self.ensure_operational()?;
            let current = self.document(document)?;
            if current.annotation_read_only == read_only {
                return Ok(());
            }
            let policy = next(current.annotation_policy)?;
            let generation = self.next_publication_generation(None)?;
            let current = self.documents[document.slot]
                .as_mut()
                .expect("validated document");
            current.annotation_read_only = read_only;
            current.annotation_policy = policy;
            self.generation = generation;
            Ok(())
        })();
        observe(recorder, Boundary::AnnotationControl, &result);
        result
    }
    pub fn prepare_compound(
        &mut self,
        writes: &[DocumentEdits],
        reads: &[DocumentVersion],
        mutations: &[AnnotationMutation],
        recorder: &mut Recorder,
    ) -> Result<PreparedTransaction, CoordinatorError> {
        self.prepare_compound_allocation(writes, reads, mutations, false, true, recorder)
    }
    pub(super) fn prepare_compound_advice_candidate(
        &mut self,
        writes: &[DocumentEdits],
        reads: &[DocumentVersion],
        mutations: &[AnnotationMutation],
        scope: Option<scope::ScopeCapture>,
        declared_reads: Option<read_dependency::ReadCapture>,
        recorder: &mut Recorder,
    ) -> Result<AdviceCandidate, CoordinatorError> {
        let mut plan =
            self.prepare_compound_allocation(writes, reads, mutations, true, true, recorder)?;
        plan.scoped = scope.is_some();
        plan.scopes = scope;
        plan.reads_required = declared_reads.is_some();
        plan.declared_reads = declared_reads;
        Ok(AdviceCandidate(plan))
    }
    #[cfg(test)]
    pub(super) fn corrupt_advice_annotation_registry_for_test(&mut self) {
        self.annotations.issued_documents.clear();
    }
    fn prepare_compound_allocation(
        &mut self,
        writes: &[DocumentEdits],
        reads: &[DocumentVersion],
        mutations: &[AnnotationMutation],
        advisory: bool,
        capture_views: bool,
        recorder: &mut Recorder,
    ) -> Result<PreparedTransaction, CoordinatorError> {
        let result = (|| {
            let mut prepared = if capture_views {
                self.prepare_inner(writes, reads)?
            } else {
                self.prepare_source_candidates(writes, reads)?
            };
            if writes
                .len()
                .checked_add(reads.len())
                .and_then(|n| n.checked_add(mutations.len()))
                .is_none_or(|n| n > self.limits.max_participants)
            {
                return Err(CoordinatorError::Limit);
            }
            let limits = self
                .annotations
                .limits
                .ok_or(CoordinatorError::Unavailable)?;
            let mut private_claims = 0usize;
            prepared
                .annotations
                .try_reserve_exact(mutations.len())
                .map_err(|_| CoordinatorError::Capacity)?;
            let mut validation_steps = limits.max_mapping_steps;
            for mutation in mutations {
                let (id, wish) = match mutation {
                    AnnotationMutation::Create { target, body } => {
                        if self.document(&target.source.document)?.annotation_read_only {
                            return Err(CoordinatorError::Annotation(AnnotationError::Permission));
                        }
                        self.resolve_annotation_target(&prepared, target, &mut validation_steps)?;
                        let body = copy_body(body, limits)?;
                        let slot = self
                            .annotations
                            .records
                            .len()
                            .checked_add(private_claims)
                            .ok_or(CoordinatorError::Limit)?;
                        if slot >= limits.max_issued {
                            return Err(CoordinatorError::Limit);
                        }
                        let slot_bytes = std::mem::size_of::<Option<Record>>()
                            .checked_add(std::mem::size_of::<DocumentId>())
                            .and_then(|n| {
                                n.checked_add(std::mem::size_of::<Option<RetiredAnnotation>>())
                            })
                            .ok_or(CoordinatorError::Limit)?;
                        self.check_root_growth(
                            slot_bytes
                                .checked_mul(private_claims + 1)
                                .ok_or(CoordinatorError::Limit)?,
                            recorder,
                        )?;
                        if advisory {
                            private_claims += 1;
                        } else {
                            self.annotations
                                .records
                                .try_reserve(1)
                                .map_err(|_| CoordinatorError::Capacity)?;
                            self.annotations
                                .issued_documents
                                .try_reserve(1)
                                .map_err(|_| CoordinatorError::Capacity)?;
                            self.annotations
                                .retired
                                .try_reserve(1)
                                .map_err(|_| CoordinatorError::Capacity)?;
                            self.annotations.records.push(None);
                            self.annotations.retired.push(None);
                            self.annotations
                                .issued_documents
                                .push(target.source.document.clone());
                        }
                        let id = AnnotationId {
                            namespace: self.namespace.clone(),
                            slot,
                            document: target.source.document.clone(),
                        };
                        (
                            id,
                            Wish::Create {
                                body,
                                target: target.clone(),
                            },
                        )
                    }
                    AnnotationMutation::UpdateBody {
                        annotation,
                        expected_content,
                        body,
                    } => (
                        annotation.clone(),
                        Wish::Body {
                            expected: *expected_content,
                            body: copy_body(body, limits)?,
                        },
                    ),
                    AnnotationMutation::Reattach {
                        annotation,
                        expected_target,
                        target,
                    } => (
                        annotation.clone(),
                        Wish::Target {
                            expected: *expected_target,
                            target: target.clone(),
                        },
                    ),
                    AnnotationMutation::Delete {
                        annotation,
                        expected_content,
                        expected_target,
                    } => (
                        annotation.clone(),
                        Wish::Delete {
                            content: *expected_content,
                            target: *expected_target,
                        },
                    ),
                };
                let document = self.document(&id.document)?;
                if document.annotation_read_only {
                    return Err(CoordinatorError::Annotation(AnnotationError::Permission));
                }
                for previous in prepared.annotations.iter().filter(|p| p.id == id) {
                    let compatible = matches!(
                        (&previous.wish, &wish),
                        (Wish::Body { .. }, Wish::Target { .. })
                            | (Wish::Target { .. }, Wish::Body { .. })
                    );
                    if !compatible {
                        return Err(CoordinatorError::DuplicateParticipant);
                    }
                }
                prepared.annotations.push(AnnotationPlan {
                    id,
                    wish,
                    policy: document.annotation_policy,
                });
            }
            self.stage_annotations_mode(&prepared, advisory)?; // current eligibility and full bounds
            Ok(prepared)
        })();
        observe(recorder, Boundary::AnnotationPreparation, &result);
        result
    }
    pub(super) fn prepare_atomic_authoring_candidates(
        &mut self,
        writes: &[DocumentEdits],
        mutations: &[AnnotationMutation],
        r: &mut Recorder,
    ) -> Result<PreparedTransaction, CoordinatorError> {
        self.prepare_compound_allocation(writes, &[], mutations, false, false, r)
    }
    fn resolve_annotation_target(
        &self,
        prepared: &PreparedTransaction,
        target: &SourceTarget,
        remaining: &mut usize,
    ) -> Result<TargetState, CoordinatorError> {
        let document = self.validate_version(&target.source)?;
        let candidate = prepared
            .candidates
            .iter()
            .find(|c| c.expected == target.source);
        let (placement, revision, authored) = match target.basis {
            TargetBasis::Before => {
                crate::attachment::validate(&document.source, target.target)
                    .map_err(mapping_error)?;
                let initial = Placement::Located {
                    target: target.target,
                    evidence: LocationEvidence::Located,
                };
                let (placement, revision) = if let Some(candidate) = candidate {
                    let mut output = crate::attachment::map_validated(
                        &document.source,
                        &candidate.source,
                        &candidate.edits,
                        &[initial],
                        remaining,
                    )
                    .map_err(mapping_error)?;
                    (
                        output.remove(0),
                        document
                            .revision
                            .checked_add(u64::from(document.source != candidate.source))
                            .ok_or(CoordinatorError::VersionExhausted)?,
                    )
                } else {
                    (initial, document.revision)
                };
                (placement, revision, target.clone())
            }
            TargetBasis::PreparedSource => {
                let candidate = candidate
                    .ok_or(CoordinatorError::Annotation(AnnotationError::InvalidTarget))?;
                crate::attachment::validate(&candidate.source, target.target)
                    .map_err(mapping_error)?;
                let revision = document
                    .revision
                    .checked_add(u64::from(document.source != candidate.source))
                    .ok_or(CoordinatorError::VersionExhausted)?;
                (
                    Placement::Located {
                        target: target.target,
                        evidence: LocationEvidence::Located,
                    },
                    revision,
                    SourceTarget {
                        source: DocumentVersion {
                            document: target.source.document.clone(),
                            revision,
                        },
                        target: target.target,
                        basis: TargetBasis::Before,
                    },
                )
            }
        };
        Ok(TargetState {
            authored,
            placement,
            revision: matches!(placement, Placement::Located { .. }).then_some(revision),
        })
    }
    fn restore_annotation_target(
        &self,
        prepared: &PreparedTransaction,
        id: &AnnotationId,
        state: &TargetState,
        source: &DocumentVersion,
        linked: bool,
    ) -> Result<TargetState, CoordinatorError> {
        let document = self.validate_version(source)?;
        if source.document != id.document {
            return Err(CoordinatorError::Faulted);
        }
        let candidate = prepared.candidates.iter().find(|c| c.expected == *source);
        let bytes = candidate.map_or(&document.source, |c| &c.source);
        if matches!(state.placement, Placement::Located { .. })
            && !linked
            && state.revision != Some(document.revision)
            && state.authored.target != AttachmentTarget::Document
        {
            return Err(CoordinatorError::History(HistoryError::Unavailable));
        }
        if let Placement::Located { target, .. } = state.placement {
            crate::attachment::validate(bytes, target).map_err(mapping_error)?;
        }
        let revision = document
            .revision
            .checked_add(u64::from(bytes != &document.source))
            .ok_or(CoordinatorError::VersionExhausted)?;
        let mut restored = state.clone();
        if matches!(restored.placement, Placement::Located { .. }) {
            restored.revision = Some(revision);
        }
        // Only placement receives the new current source context. The authored
        // selector retains its original historical choice and coordinate basis.
        Ok(restored)
    }
    pub(super) fn stage_annotations(
        &self,
        prepared: &PreparedTransaction,
    ) -> Result<AnnotationDecision, CoordinatorError> {
        self.stage_annotations_mode(prepared, false)
    }
    pub(super) fn stage_advisory_annotations(
        &self,
        prepared: &PreparedTransaction,
    ) -> Result<AnnotationDecision, CoordinatorError> {
        self.stage_annotations_mode(prepared, true)
    }
    fn stage_annotations_mode(
        &self,
        prepared: &PreparedTransaction,
        advisory: bool,
    ) -> Result<AnnotationDecision, CoordinatorError> {
        let mut decision = AnnotationDecision {
            updates: Vec::new(),
            sets: Vec::new(),
            journals: Vec::new(),
            facts: Vec::new(),
        };
        let Some(limits) = self.annotations.limits else {
            return if prepared.annotations.is_empty() {
                Ok(decision)
            } else {
                Err(CoordinatorError::Unavailable)
            };
        };
        let mut remaining = limits.max_mapping_steps;
        let capacity = self
            .annotations
            .records
            .len()
            .checked_add(prepared.annotations.len())
            .ok_or(CoordinatorError::Limit)?;
        decision
            .updates
            .try_reserve_exact(capacity)
            .map_err(|_| CoordinatorError::Capacity)?;
        decision
            .journals
            .try_reserve_exact(prepared.annotations.len())
            .map_err(|_| CoordinatorError::Capacity)?;
        for candidate in &prepared.candidates {
            let document = self.validate_version(&candidate.expected)?;
            if document.annotation_generation != candidate.annotation_generation {
                return Err(CoordinatorError::Stale);
            }
            for record in
                self.annotations.records.iter().flatten().filter(|r| {
                    r.value.present && r.value.id.document == candidate.expected.document
                })
            {
                let mut value = record.value.clone();
                let mut output = crate::attachment::map_validated(
                    &document.source,
                    &candidate.source,
                    &candidate.edits,
                    &[value.target.placement],
                    &mut remaining,
                )
                .map_err(|error| match error {
                    crate::attachment::AttachmentError::Limit => CoordinatorError::Limit,
                    crate::attachment::AttachmentError::Capacity => CoordinatorError::Capacity,
                    // Existing private placements and source journals must already
                    // satisfy their invariants; drift is not a caller rejection.
                    _ => CoordinatorError::Faulted,
                })?;
                value.target.placement = output.remove(0);
                value.target.revision = matches!(value.target.placement, Placement::Located { .. })
                    .then_some(
                        document
                            .revision
                            .checked_add(u64::from(document.source != candidate.source))
                            .ok_or(CoordinatorError::VersionExhausted)?,
                    );
                if value.target != record.value.target {
                    value.placement_generation = next(value.placement_generation)?;
                    decision.updates.push(value);
                }
            }
        }
        for plan in &prepared.annotations {
            if self
                .annotations
                .retired
                .get(plan.id.slot)
                .is_some_and(Option::is_some)
            {
                return Err(CoordinatorError::Unavailable);
            }
            let document = self.document(&plan.id.document)?;
            if document.annotation_read_only {
                return Err(CoordinatorError::Annotation(AnnotationError::Permission));
            }
            if document.annotation_policy != plan.policy {
                return Err(CoordinatorError::Stale);
            }
            let old = self
                .annotations
                .records
                .get(plan.id.slot)
                .and_then(Option::as_ref)
                .map(|r| &r.value);
            if !matches!(plan.wish, Wish::Create { .. }) {
                self.annotation_record(&plan.id)?;
            }
            let index = decision.updates.iter().position(|v| v.id == plan.id);
            let mut value = index
                .map(|i| decision.updates[i].clone())
                .or_else(|| old.cloned());
            let mut delta = None;
            match &plan.wish {
                Wish::Create { body, target } => {
                    if old.is_some() || target.source.document != plan.id.document {
                        return Err(CoordinatorError::Unavailable);
                    }
                    let target =
                        self.resolve_annotation_target(prepared, target, &mut remaining)?;
                    let fresh = Value {
                        restored_identity: None,
                        id: plan.id.clone(),
                        body: body.clone(),
                        content: 1,
                        target_generation: 1,
                        placement_generation: 1,
                        target,
                        present: true,
                    };
                    delta = Some(Delta::Membership {
                        before: None,
                        after: Some(fresh.authored()),
                    });
                    value = Some(fresh);
                }
                Wish::Body { expected, body } | Wish::RestoreBody { expected, body } => {
                    let current = value.as_mut().ok_or(CoordinatorError::Unavailable)?;
                    if !current.present {
                        return Err(CoordinatorError::Unavailable);
                    }
                    if current.content != *expected {
                        return Err(CoordinatorError::Stale);
                    }
                    if current.body != *body {
                        delta = Some(Delta::Fields {
                            body: Some((current.body.clone(), body.clone())),
                            target: None,
                        });
                        current.body = body.clone();
                        current.content = next(current.content)?;
                    }
                }
                Wish::Target { expected, target } => {
                    let current = value.as_mut().ok_or(CoordinatorError::Unavailable)?;
                    if !current.present || target.source.document != plan.id.document {
                        return Err(CoordinatorError::Unavailable);
                    }
                    if current.target_generation != *expected {
                        return Err(CoordinatorError::Stale);
                    }
                    let target =
                        self.resolve_annotation_target(prepared, target, &mut remaining)?;
                    if current.target != target {
                        delta = Some(Delta::Fields {
                            body: None,
                            target: Some((
                                old.expect("live original").target.clone(),
                                target.clone(),
                            )),
                        });
                        current.target = target;
                        current.target_generation = next(current.target_generation)?;
                        current.placement_generation = next(current.placement_generation)?;
                    }
                }
                Wish::Delete { content, target } => {
                    let current = value.as_mut().ok_or(CoordinatorError::Unavailable)?;
                    if !current.present {
                        return Err(CoordinatorError::Unavailable);
                    }
                    if current.content != *content || current.target_generation != *target {
                        return Err(CoordinatorError::Stale);
                    }
                    delta = Some(Delta::Membership {
                        before: Some(old.expect("live original").authored()),
                        after: None,
                    });
                    current.present = false;
                }
                Wish::RestoreTarget {
                    expected,
                    state,
                    source,
                    linked,
                } => {
                    let current = value.as_mut().ok_or(CoordinatorError::Unavailable)?;
                    if !current.present || current.target_generation != *expected {
                        return Err(CoordinatorError::Stale);
                    }
                    current.target =
                        self.restore_annotation_target(prepared, &plan.id, state, source, *linked)?;
                    current.target_generation = next(current.target_generation)?;
                    current.placement_generation = next(current.placement_generation)?;
                }
                Wish::RestoreMembership {
                    content,
                    target,
                    present,
                    desired,
                    source,
                    linked,
                } => {
                    let current = value.as_mut().ok_or(CoordinatorError::Unavailable)?;
                    if current.present != *present
                        || current.content != *content
                        || current.target_generation != *target
                    {
                        return Err(CoordinatorError::Stale);
                    }
                    if let Some(authored) = desired {
                        current.body = authored.body.clone();
                        current.target = self.restore_annotation_target(
                            prepared,
                            &plan.id,
                            &authored.target,
                            source,
                            *linked,
                        )?;
                        current.content = next(current.content)?;
                        current.target_generation = next(current.target_generation)?;
                        current.placement_generation = next(current.placement_generation)?;
                        current.present = true;
                    } else {
                        current.present = false;
                    }
                }
            }
            if let Some(delta) = delta {
                if let Some(journal) = decision.journals.iter_mut().find(|j| j.id == plan.id) {
                    match (&mut journal.delta, delta) {
                        (Delta::Fields { body, target }, Delta::Fields { body: b, target: t }) => {
                            if b.is_some() {
                                *body = b;
                            }
                            if t.is_some() {
                                *target = t;
                            }
                        }
                        _ => return Err(CoordinatorError::Faulted),
                    }
                } else {
                    decision.journals.push(AnnotationJournal {
                        id: plan.id.clone(),
                        delta,
                    });
                }
            }
            let value = value.ok_or(CoordinatorError::Faulted)?;
            if old != Some(&value) {
                if let Some(index) = index {
                    decision.updates[index] = value;
                } else {
                    decision.updates.push(value);
                }
            }
        }
        for value in &mut decision.updates {
            if !advisory && value.id.slot >= self.annotations.records.len() {
                return Err(CoordinatorError::Faulted);
            }
            if let Some(old) = self
                .annotations
                .records
                .get(value.id.slot)
                .and_then(Option::as_ref)
                .map(|r| &r.value)
            {
                let changed = old.target.placement != value.target.placement
                    || old.target.revision != value.target.revision;
                value.placement_generation = if changed {
                    next(old.placement_generation)?
                } else {
                    old.placement_generation
                };
            }
        }
        decision.updates.retain(|value| {
            self.annotations
                .records
                .get(value.id.slot)
                .and_then(Option::as_ref)
                .is_none_or(|r| r.value != *value)
        });
        let mut records = 0usize;
        let mut bytes = 0usize;
        for (slot, record) in self.annotations.records.iter().enumerate() {
            let value = decision
                .updates
                .iter()
                .find(|v| v.id.slot == slot)
                .or_else(|| record.as_ref().map(|r| &r.value));
            if let Some(value) = value {
                records += 1;
                bytes = bytes
                    .checked_add(value.body.len())
                    .ok_or(CoordinatorError::Limit)?;
            }
        }
        if advisory {
            for value in decision
                .updates
                .iter()
                .filter(|v| v.id.slot >= self.annotations.records.len())
            {
                records = records.checked_add(1).ok_or(CoordinatorError::Limit)?;
                bytes = bytes
                    .checked_add(value.body.len())
                    .ok_or(CoordinatorError::Limit)?;
            }
        }
        if records > limits.max_records || bytes > limits.max_retained_body_bytes {
            return Err(CoordinatorError::Limit);
        }
        decision
            .facts
            .try_reserve_exact(decision.updates.len())
            .map_err(|_| CoordinatorError::Capacity)?;
        decision
            .sets
            .try_reserve_exact(decision.updates.len())
            .map_err(|_| CoordinatorError::Capacity)?;
        for value in &decision.updates {
            decision.facts.push(value.fact());
            if !decision.sets.iter().any(|(id, _)| *id == value.id.document) {
                decision.sets.push((
                    value.id.document.clone(),
                    next(self.document(&value.id.document)?.annotation_generation)?,
                ));
            }
        }
        Ok(decision)
    }
    pub(super) fn annotation_reversal_plans(
        &self,
        journals: &[AnnotationJournal],
        direction: HistoryDirection,
        prepared: &mut PreparedTransaction,
    ) -> Result<(), HistoryError> {
        prepared
            .annotations
            .try_reserve_exact(journals.len() * 2)
            .map_err(|_| HistoryError::Capacity)?;
        for journal in journals {
            let record = self
                .annotation_record(&journal.id)
                .map_err(|_| HistoryError::Unavailable)?;
            let current = &record.value;
            let document = self
                .document(&journal.id.document)
                .map_err(|_| HistoryError::Unavailable)?;
            let source = DocumentVersion {
                document: journal.id.document.clone(),
                revision: document.revision,
            };
            let linked = prepared
                .candidates
                .iter()
                .any(|c| c.expected.document == journal.id.document);
            let mut push = |wish| {
                prepared.annotations.push(AnnotationPlan {
                    id: journal.id.clone(),
                    wish,
                    policy: document.annotation_policy,
                });
            };
            match &journal.delta {
                Delta::Membership { before, after } => {
                    let (expected, desired) = match direction {
                        HistoryDirection::Undo => (after, before),
                        HistoryDirection::Redo => (before, after),
                    };
                    if current.present != expected.is_some() {
                        return Err(HistoryError::Faulted);
                    }
                    push(Wish::RestoreMembership {
                        content: current.content,
                        target: current.target_generation,
                        present: current.present,
                        desired: desired.clone(),
                        source,
                        linked,
                    });
                }
                Delta::Fields { body, target } => {
                    if !current.present {
                        return Err(HistoryError::Unavailable);
                    }
                    if let Some((before, after)) = body {
                        let (expected, desired) = match direction {
                            HistoryDirection::Undo => (after, before),
                            HistoryDirection::Redo => (before, after),
                        };
                        if current.body != *expected {
                            return Err(HistoryError::Faulted);
                        }
                        push(Wish::RestoreBody {
                            expected: current.content,
                            body: desired.clone(),
                        });
                    }
                    if let Some((before, after)) = target {
                        let (expected, desired) = match direction {
                            HistoryDirection::Undo => (after, before),
                            HistoryDirection::Redo => (before, after),
                        };
                        if current.target.authored.target != expected.authored.target {
                            return Err(HistoryError::Faulted);
                        }
                        push(Wish::RestoreTarget {
                            expected: current.target_generation,
                            state: desired.clone(),
                            source,
                            linked,
                        });
                    }
                }
            }
        }
        self.stage_annotations(prepared).map_err(|e| match e {
            CoordinatorError::History(error) => error,
            CoordinatorError::VersionExhausted => HistoryError::VersionExhausted,
            CoordinatorError::Limit => HistoryError::Limit,
            CoordinatorError::Capacity => HistoryError::Capacity,
            _ => HistoryError::Unavailable,
        })?;
        Ok(())
    }
}

#[cfg(test)]
pub(in crate::coordinator) mod tests {
    use super::*;
    #[test]
    fn shutdown_inventory_tracks_annotation_records_and_actual_snapshot_lifetime_without_new_pins()
    {
        use crate::coordinator::shutdown::tests::inventory_owner;
        let mut r = Recorder::disabled();
        let (mut o, d, _) = setup(&mut r);
        let set = o.validate_version(&d).unwrap().annotation_generation;
        let snapshot = o.capture_annotations(&d, set, &mut r).unwrap();
        let before = o.annotation_usage();
        let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::Annotations, &[&d.document]);
        assert_eq!(
            row.retained_records,
            Some(before.retained_records + before.retained_snapshots)
        );
        assert_eq!(o.annotation_usage(), before);
        drop(snapshot);
        let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::Annotations, &[&d.document]);
        assert_eq!(row.retained_records, Some(before.retained_records));
    }
    pub(in crate::coordinator) fn exhaust_content_for_test(
        owner: &mut SourceCoordinator,
        annotation: &AnnotationId,
    ) {
        owner.annotations.records[annotation.slot]
            .as_mut()
            .unwrap()
            .value
            .content = u64::MAX;
    }
    pub(in crate::coordinator) fn setup(
        recorder: &mut Recorder,
    ) -> (SourceCoordinator, DocumentVersion, AnnotationId) {
        let mut owner = SourceCoordinator::configured(CoordinatorConfig {
            limits: CoordinatorLimits {
                max_document_incarnations: 4,
                max_participants: 8,
                max_staged_bytes: 4096,
                source: Limits {
                    max_source_bytes: 4096,
                    max_edits: 16,
                    max_edit_payload_bytes: 8192,
                },
            },
            invocation: Some(InvocationLimits {
                max_lifetime_issued: 8,
                max_retained: 8,
                max_intent_bytes: 32768,
            }),
            history: Some(HistoryLimits {
                max_retained_groups: 16,
                max_journal_bytes: 65536,
                max_pins: 8,
            }),
            resources: None,
            search: None,
            annotations: Some(AnnotationLimits {
                max_issued: 16,
                max_records: 16,
                max_body_bytes: 1024,
                max_retained_body_bytes: 8192,
                max_mapping_steps: 8192,
                max_live_snapshots: 8,
                max_snapshot_bytes: 65536,
            }),
        });
        let source = owner.create(b"abc", recorder).unwrap();
        let prepared = owner
            .prepare_compound(
                &[],
                &[],
                &[AnnotationMutation::Create {
                    body: "PRIVATE_OLD".into(),
                    target: SourceTarget {
                        source: source.clone(),
                        target: AttachmentTarget::ContentRange { start: 0, end: 1 },
                        basis: TargetBasis::Before,
                    },
                }],
                recorder,
            )
            .unwrap();
        let id = prepared.created_annotations().next().unwrap().clone();
        owner.commit(prepared, recorder).unwrap();
        (owner, source, id)
    }
    #[test]
    fn exhausted_annotation_versions_allow_complete_noop_but_refuse_source_companion_change_atomically()
     {
        let mut recorder = Recorder::disabled();
        let (mut owner, source, id) = setup(&mut recorder);
        let value = &mut owner.annotations.records[id.slot].as_mut().unwrap().value;
        value.content = u64::MAX;
        value.target_generation = u64::MAX;
        value.placement_generation = u64::MAX;
        owner.documents[source.document.slot]
            .as_mut()
            .unwrap()
            .annotation_generation = u64::MAX;
        let generation = owner.generation();
        let noop = owner
            .prepare_compound(
                &[],
                &[],
                &[AnnotationMutation::UpdateBody {
                    annotation: id.clone(),
                    expected_content: u64::MAX,
                    body: "PRIVATE_OLD".into(),
                }],
                &mut recorder,
            )
            .unwrap();
        assert!(
            owner
                .commit(noop, &mut recorder)
                .unwrap()
                .operation
                .is_none()
        );
        assert_eq!(owner.generation(), generation);
        let prepared = owner
            .prepare(
                &[DocumentEdits {
                    expected: source.clone(),
                    edits: vec![Replacement {
                        start: 0,
                        end: 0,
                        expected: "".into(),
                        inserted: "X".into(),
                    }],
                }],
                &[],
                &mut recorder,
            )
            .unwrap();
        assert_eq!(
            owner.commit(prepared, &mut recorder),
            Err(CoordinatorError::VersionExhausted)
        );
        assert!(!owner.is_faulted());
        assert_eq!(
            owner.read(&source.document, &mut recorder).unwrap().version,
            source
        );
        assert_eq!(
            owner.annotation(&id, &mut recorder).unwrap().body,
            "PRIVATE_OLD"
        );
    }
    #[test]
    fn interrupted_mixed_publication_retains_unknown_and_fences_every_public_domain() {
        let mut recorder = Recorder::try_with_capacity(128).unwrap();
        let (mut owner, source, id) = setup(&mut recorder);
        let invocation = owner.issue_invocation(&mut recorder).unwrap();
        let intent = CompoundIntent {
            source: SourceIntent {
                writes: vec![DocumentEdits {
                    expected: source.clone(),
                    edits: vec![Replacement {
                        start: 0,
                        end: 0,
                        expected: "".into(),
                        inserted: "X".into(),
                    }],
                }],
                reads: vec![],
            },
            annotations: vec![AnnotationMutation::UpdateBody {
                annotation: id.clone(),
                expected_content: 1,
                body: "PRIVATE_NEW".into(),
            }],
        };
        owner
            .submit_compound(&invocation, &intent, &mut recorder)
            .unwrap();
        while recorder.pop_front().is_some() {}
        owner.publication_fault_after = Some(1);
        let fault = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            owner.run(&invocation, &mut recorder)
        }));
        assert!(fault.is_err());
        assert!(owner.is_faulted());
        assert_eq!(
            owner.invocation_outcome(&invocation, &mut recorder),
            Err(InvocationError::Faulted)
        );
        assert!(owner.read(&source.document, &mut recorder).is_err());
        assert!(owner.annotation(&id, &mut recorder).is_err());
        assert_eq!(
            &*owner.annotations.records[id.slot]
                .as_ref()
                .unwrap()
                .value
                .body,
            "PRIVATE_OLD"
        );
        while let Some(record) = recorder.pop_front() {
            assert!(!matches!(
                record.boundary,
                Boundary::Publication | Boundary::AnnotationPublication
            ));
        }
    }
    #[test]
    fn owned_attachment_drift_is_an_integrity_fault_not_a_retained_caller_rejection() {
        let mut recorder = Recorder::disabled();
        let (mut owner, source, id) = setup(&mut recorder);
        owner.annotations.records[id.slot]
            .as_mut()
            .unwrap()
            .value
            .target
            .placement = Placement::Located {
            target: AttachmentTarget::ContentRange { start: 0, end: 999 },
            evidence: LocationEvidence::Located,
        };
        let invocation = owner.issue_invocation(&mut recorder).unwrap();
        let intent = SourceIntent {
            writes: vec![DocumentEdits {
                expected: source.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 0,
                    expected: "".into(),
                    inserted: "X".into(),
                }],
            }],
            reads: vec![],
        };
        owner.submit(&invocation, &intent, &mut recorder).unwrap();
        assert_eq!(
            owner.run(&invocation, &mut recorder),
            Err(InvocationError::Faulted)
        );
        assert!(owner.is_faulted());
        assert_eq!(
            owner.invocation_outcome(&invocation, &mut recorder),
            Err(InvocationError::Faulted)
        );
        assert_eq!(
            owner.documents[source.document.slot]
                .as_ref()
                .unwrap()
                .source
                .text(),
            "abc"
        );
    }
}
