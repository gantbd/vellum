//! Native owned-operation observation. No provider callback, dispatch or body query.
use super::*;
mod shutdown_inventory;
use std::mem::size_of;
use std::panic::{AssertUnwindSafe, catch_unwind};
mod arguments;
mod decoder;
mod wait;
pub use arguments::*;
pub use decoder::*;
pub use wait::*;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeOperationObservationErrorV1 {
    Unavailable,
    InvalidRequest,
    Limit,
    Capacity,
    Faulted,
}
type Error = NativeOperationObservationErrorV1;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeOperationObservationPoolLimitsV1 {
    pub max_contexts: usize,
    pub max_issued: usize,
    pub max_pool_bytes: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeOperationObservationLimitsV1 {
    pub max_waiters: usize,
    pub max_issued: usize,
    pub max_wait_steps: usize,
    pub max_context_bytes: usize,
    pub max_wait_bytes: usize,
    pub max_reference_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_control_reply_bytes: usize,
    pub max_held_bytes: usize,
    pub max_label_bytes: usize,
}
#[derive(Clone, PartialEq, Eq)]
pub struct NativeOperationObservationPolicyV1 {
    pub recipient: String,
    pub epoch: u64,
    pub grant: bool,
    pub publication: bool,
    pub control: bool,
    pub effect: bool,
    pub reasons: bool,
    pub details: bool,
    pub recovery: bool,
}
impl std::fmt::Debug for NativeOperationObservationPolicyV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeOperationObservationPolicyV1(<redacted>)")
    }
}
pub struct NativeOperationObservationFactsV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub retained: Option<bool>,
    pub transfer_ready: Option<bool>,
    pub resources_ready: Option<bool>,
    pub preparation_reserved: Option<bool>,
    pub registry_ready: Option<bool>,
    pub retention_ready: Option<bool>,
    pub control_allowed: Option<bool>,
    pub control_ready: Option<bool>,
    pub ordinary_ready: Option<bool>,
    pub operation_receipt_admission_allowed: Option<bool>,
    pub operation_receipt_allowed: Option<bool>,
    pub source_readable: Option<bool>,
    pub annotation_readable: Option<bool>,
    pub disposition_readable: Option<bool>,
    pub read_allowed: Option<bool>,
    pub reply_ready: Option<bool>,
    pub wait_capacity: Option<bool>,
    pub wait_clock_valid: Option<bool>,
    pub owner_control_allowed: Option<bool>,
}
pub struct NativeOperationObservationControlV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub owner_control_allowed: Option<bool>,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
}
macro_rules! identity {
    ($name:ident) => {
        #[derive(Clone)]
        pub struct $name {
            owner: Arc<()>,
            context: Arc<()>,
            marker: Arc<()>,
        }
        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                Arc::ptr_eq(&self.owner, &other.owner)
                    && Arc::ptr_eq(&self.context, &other.context)
                    && Arc::ptr_eq(&self.marker, &other.marker)
            }
        }
        impl Eq for $name {}
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(concat!(stringify!($name), "(<opaque>)"))
            }
        }
    };
}
identity!(NativeOperationProgressIdV1);
identity!(NativeOperationWaitIdV1);
#[derive(Clone)]
pub struct NativeOperationObservationIdV1 {
    owner: Arc<()>,
    marker: Arc<()>,
    slot: usize,
}
impl PartialEq for NativeOperationObservationIdV1 {
    fn eq(&self, other: &Self) -> bool {
        self.slot == other.slot
            && Arc::ptr_eq(&self.owner, &other.owner)
            && Arc::ptr_eq(&self.marker, &other.marker)
    }
}
impl Eq for NativeOperationObservationIdV1 {}
impl std::fmt::Debug for NativeOperationObservationIdV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeOperationObservationIdV1(<opaque>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeSaveKindV1 {
    SourceSave,
    AnnotationSave,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeOperationObservationPhaseV1 {
    Pending,
    Terminal,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeSaveWritePhaseV1 {
    Queued,
    Dispatched,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeSaveRequiredAcknowledgmentV1 {
    Written,
    Synchronized,
}
#[derive(Clone, PartialEq, Eq)]
pub struct ObservedSaveVersionV1 {
    epoch: Option<Blob>,
    token: Blob,
}
impl ObservedSaveVersionV1 {
    /// None is an opaque version, not absence or a fabricated epoch.
    pub fn epoch(&self) -> Option<&[u8]> {
        self.epoch.as_ref().map(|b| b.0.as_ref())
    }
    pub fn token(&self) -> &[u8] {
        &self.token.0
    }
}
impl std::fmt::Debug for ObservedSaveVersionV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ObservedSaveVersionV1(<redacted>)")
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeSaveReceiptV1 {
    pub kind: NativeSaveKindV1,
    pub phase: NativeSaveWritePhaseV1,
    pub stopped: bool,
    pub quiescent: bool,
    pub contested: bool,
    /// Original no-future-effect attestation; not inferred known application.
    pub safe_disposition: bool,
    pub destination_faulted: bool,
    pub application: Application,
    pub observed_bytes: usize,
    pub synchronization: Synchronization,
    pub version: Option<ObservedSaveVersionV1>,
    pub required_acknowledgment: NativeSaveRequiredAcknowledgmentV1,
    pub requested_strength_satisfied: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeOperationReceiptV1 {
    Save(NativeSaveReceiptV1),
    AnnotationLoad(NativeAnnotationReadReceiptV1),
    SourceLoad(NativeSourceReadReceiptV1),
}
impl NativeOperationReceiptV1 {
    fn bits(&self) -> (bool, bool) {
        match self {
            Self::Save(v) => (v.stopped, v.quiescent),
            Self::AnnotationLoad(v) => (v.stopped, v.quiescent),
            Self::SourceLoad(v) => (v.stopped, v.quiescent),
        }
    }
}
#[derive(Clone)]
enum Target {
    Save(SaveId, NativeSaveKindV1),
    AnnotationLoad(AnnotationReadId),
    SourceLoad(SourceReadIdV1),
}
impl Target {
    fn matches(&self, receipt: &NativeOperationReceiptV1) -> bool {
        match (self, receipt) {
            (Self::Save(_, kind), NativeOperationReceiptV1::Save(v)) => *kind == v.kind,
            (Self::AnnotationLoad(_), NativeOperationReceiptV1::AnnotationLoad(_)) => true,
            (Self::SourceLoad(_), NativeOperationReceiptV1::SourceLoad(_)) => true,
            _ => false,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeOperationObservationQueryV1 {
    Status,
    RetainedOutcome,
    MinimalControlStatus,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeOperationObservationReplyV1 {
    Status {
        phase: NativeOperationObservationPhaseV1,
        stopped: bool,
        quiescent: bool,
        receipt: Option<NativeOperationReceiptV1>,
        observation: NativeOperationProgressIdV1,
    },
    Receipt {
        receipt: NativeOperationReceiptV1,
        observation: NativeOperationProgressIdV1,
    },
}
impl NativeOperationObservationReplyV1 {
    pub fn observation(&self) -> &NativeOperationProgressIdV1 {
        match self {
            Self::Status { observation, .. } | Self::Receipt { observation, .. } => observation,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeOperationObservationUsageV1 {
    pub live: bool,
    pub busy: bool,
    pub receipt_faulted: bool,
    pub ordinary_issued: usize,
    pub control_issued: usize,
    pub references: usize,
    pub waiters: usize,
    pub waiting: usize,
    pub ended_waiters: usize,
    pub declared_held_bytes: usize,
    pub reserved_ceiling_bytes: usize,
}
/// Trusted-owner gauges, not recipient-safe discovery or effect evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeOperationObservationPoolUsageV1 {
    pub issued_lifetime: usize,
    pub retained_contexts: usize,
    pub stopped: bool,
    pub reserved_pool_bytes: usize,
}
#[derive(Default)]
pub(super) enum State {
    #[default]
    Unconfigured,
    Active(Box<Pool>),
    Closed,
}
impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeOperationObservationState(<owned>)")
    }
}
pub(super) struct Pool {
    limits: NativeOperationObservationPoolLimitsV1,
    rows: Vec<Option<Record>>,
    stopped: bool,
    reservation: retention::Reservation,
}
struct Record {
    decoder: decoder::State,
    marker: Arc<()>,
    target: Target,
    context: SettingsExecutionContextV1,
    ceiling: NativeOperationObservationPolicyV1,
    limits: NativeOperationObservationLimitsV1,
    reservation: retention::Reservation,
    charge: usize,
    live: bool,
    busy: bool,
    receipt_faulted: bool,
    ordinary_issued: usize,
    control_issued: usize,
    refs: Vec<Reference>,
    last_ordinary: Option<(Signature, NativeOperationProgressIdV1)>,
    last_control: Option<((bool, bool), NativeOperationProgressIdV1)>,
    waiters: Vec<Option<Waiter>>,
    #[cfg(test)]
    fault: Option<Fault>,
}
struct Reference {
    id: NativeOperationProgressIdV1,
    control: bool,
}
#[derive(Clone, PartialEq, Eq)]
struct Signature {
    control: Option<(bool, bool)>,
    receipt: Option<NativeOperationReceiptV1>,
}
#[cfg(test)]
#[derive(Clone, Copy)]
enum Fault {
    Panic,
    WrongKind,
}
fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(Error::Limit)
}
fn parent(e: RetentionErrorV1) -> Error {
    match e {
        RetentionErrorV1::Limit => Error::Limit,
        RetentionErrorV1::Unavailable => Error::Unavailable,
        RetentionErrorV1::Faulted => Error::Faulted,
    }
}
fn resource(e: ResourceError) -> Error {
    if e == ResourceError::Faulted {
        Error::Faulted
    } else {
        Error::Unavailable
    }
}
fn observe<T>(r: &mut Recorder, boundary: Boundary, v: &Result<T, Error>) {
    r.record(Observation {
        boundary,
        disposition: match v {
            Ok(_) => Disposition::Completed,
            Err(Error::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time: None,
    });
}
fn kind(k: ArtifactKind) -> NativeSaveKindV1 {
    match k {
        ArtifactKind::Source => NativeSaveKindV1::SourceSave,
        ArtifactKind::Annotations => NativeSaveKindV1::AnnotationSave,
    }
}
fn receipt_allowed(target: &Target, f: &NativeOperationObservationFactsV1<'_>) -> bool {
    [
        f.operation_receipt_allowed,
        f.reply_ready,
        match target {
            Target::Save(_, _) => f.disposition_readable,
            Target::AnnotationLoad(_) | Target::SourceLoad(_) => f.read_allowed,
        },
        match target {
            Target::Save(_, NativeSaveKindV1::SourceSave) | Target::SourceLoad(_) => {
                f.source_readable
            }
            Target::Save(_, NativeSaveKindV1::AnnotationSave) | Target::AnnotationLoad(_) => {
                f.annotation_readable
            }
        },
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn input_bytes(
    c: &SettingsExecutionContextV1,
    p: &NativeOperationObservationPolicyV1,
) -> Result<usize, Error> {
    settings::basis::labels(c)
        .into_iter()
        .map(str::len)
        .chain([p.recipient.len()])
        .try_fold(size_of::<Record>(), add)
}
impl NativeOperationObservationPoolLimitsV1 {
    pub fn required_pool_bytes(self) -> Result<usize, Error> {
        add(
            add(size_of::<Pool>(), size_of::<State>())?,
            self.max_issued
                .checked_mul(size_of::<Option<Record>>())
                .ok_or(Error::Limit)?,
        )
    }
}
impl NativeOperationObservationLimitsV1 {
    pub fn declared_held_bytes(self) -> Result<usize, Error> {
        [
            (5, self.max_reply_bytes),
            (2, self.max_control_reply_bytes),
            (3, self.max_context_bytes),
            (add(self.max_waiters, 1)?, self.max_wait_bytes),
            (add(self.max_issued, 4)?, self.max_reference_bytes),
        ]
        .into_iter()
        .try_fold(0, |n, (a, b)| add(n, a.checked_mul(b).ok_or(Error::Limit)?))
    }
}
impl Record {
    fn close(&mut self) {
        self.live = false;
        self.refs.clear();
        self.last_ordinary = None;
        self.last_control = None;
        for w in self.waiters.iter_mut().flatten() {
            w.waiting = false;
        }
    }
}
impl SourceCoordinator {
    fn native_operation_control(
        &self,
        target: &Target,
        context: &SettingsExecutionContextV1,
    ) -> Result<(bool, bool), Error> {
        match target {
            Target::Save(task, kind) => self.native_save_control(task, context, *kind),
            Target::AnnotationLoad(task) => self
                .owned_annotation_read_control(task, context)
                .map_err(resource),
            Target::SourceLoad(task) => self
                .owned_source_read_control(task, context)
                .map_err(resource),
        }
    }
    fn native_operation_receipt(
        &self,
        target: &Target,
        context: &SettingsExecutionContextV1,
    ) -> Result<NativeOperationReceiptV1, Error> {
        match target {
            Target::Save(task, kind) => self
                .native_save_receipt(task, context, *kind)
                .map(NativeOperationReceiptV1::Save),
            Target::AnnotationLoad(task) => self
                .owned_annotation_read_receipt(task, context)
                .map(NativeOperationReceiptV1::AnnotationLoad)
                .map_err(resource),
            Target::SourceLoad(task) => self
                .owned_source_read_receipt(task, context)
                .map(NativeOperationReceiptV1::SourceLoad)
                .map_err(resource),
        }
    }
    fn observation_pool(&self) -> Result<&Pool, Error> {
        match &self.resources.observation {
            State::Active(p) => Ok(p),
            _ => Err(Error::Unavailable),
        }
    }
    fn observation_pool_mut(&mut self) -> Result<&mut Pool, Error> {
        match &mut self.resources.observation {
            State::Active(p) => Ok(p),
            _ => Err(Error::Unavailable),
        }
    }
    fn observation_row(&self, id: &NativeOperationObservationIdV1) -> Result<&Record, Error> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return Err(Error::Unavailable);
        }
        let row = self
            .observation_pool()?
            .rows
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)?;
        if !Arc::ptr_eq(&row.marker, &id.marker) {
            return Err(Error::Unavailable);
        }
        Ok(row)
    }
    fn observation_row_mut(
        &mut self,
        id: &NativeOperationObservationIdV1,
    ) -> Result<&mut Record, Error> {
        self.observation_row(id)?;
        Ok(self.observation_pool_mut()?.rows[id.slot]
            .as_mut()
            .expect("validated observer"))
    }
    pub fn configure_native_operation_observation_v1(
        &mut self,
        limits: NativeOperationObservationPoolLimitsV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = (|| {
            self.ensure_operational().map_err(|_| Error::Faulted)?;
            if self.resources.limits.is_none() {
                return Err(Error::Unavailable);
            }
            match &self.resources.observation {
                State::Active(p) => {
                    self.retention.validate(&p.reservation).map_err(parent)?;
                    return if p.limits == limits {
                        Ok(())
                    } else {
                        Err(Error::Unavailable)
                    };
                }
                State::Closed => return Err(Error::Unavailable),
                State::Unconfigured => {}
            }
            self.ensure_admission_unfenced()
                .map_err(|e| resource(owner_error(e)))?;
            let bytes = limits.required_pool_bytes()?;
            if limits.max_contexts > limits.max_issued || bytes > limits.max_pool_bytes {
                return Err(Error::Limit);
            }
            self.retention.check(bytes).map_err(parent)?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(limits.max_issued)
                .map_err(|_| Error::Capacity)?;
            let reservation = self.retention.reserve(bytes).map_err(parent)?;
            self.resources.observation = State::Active(Box::new(Pool {
                limits,
                rows,
                stopped: false,
                reservation,
            }));
            Ok(())
        })();
        observe(
            r,
            Boundary::NativeOperationObservationConfiguration,
            &result,
        );
        result
    }
    fn native_save_control(
        &self,
        task: &SaveId,
        context: &SettingsExecutionContextV1,
        expected: NativeSaveKindV1,
    ) -> Result<(bool, bool), Error> {
        if kind(
            self.owned_save_observation_kind(task, context)
                .map_err(resource)?,
        ) != expected
        {
            return Err(Error::Faulted);
        }
        let t = self.resource_task(task).map_err(resource)?;
        Ok((t.stop_requested, t.quiescent))
    }
    fn native_save_receipt(
        &self,
        task: &SaveId,
        context: &SettingsExecutionContextV1,
        expected: NativeSaveKindV1,
    ) -> Result<NativeSaveReceiptV1, Error> {
        self.native_save_control(task, context, expected)?;
        let t = self.resource_task(task).map_err(resource)?;
        let data = &t.knowledge.data;
        let valid_version = match &data.version {
            None => data.epoch.is_none(),
            Some(token) => {
                !token.0.is_empty()
                    && token.0.len() <= t.registration.limits.max_validator_bytes
                    && (data.epoch.is_some()
                        == (t.result_version_profile == SaveResultVersionProfileV1::EpochToken))
                    && data.epoch.as_ref().is_none_or(|e| {
                        !e.0.is_empty() && e.0.len() <= t.registration.limits.max_validator_bytes
                    })
            }
        };
        if !valid_version
            || data.observed > t.payload.0.len()
            || (data.application == Application::Written && data.observed != t.payload.0.len())
            || (data.application == Application::NotWritten && data.observed != 0)
        {
            return Err(Error::Faulted);
        }
        let faulted = self.resource_contested(&t.identity);
        Ok(NativeSaveReceiptV1 {
            kind: expected,
            phase: if t.phase == SavePhase::Dispatched {
                NativeSaveWritePhaseV1::Dispatched
            } else {
                NativeSaveWritePhaseV1::Queued
            },
            stopped: t.stop_requested,
            quiescent: t.quiescent,
            contested: t.contested(),
            safe_disposition: t.no_future,
            destination_faulted: faulted,
            application: data.application,
            observed_bytes: data.observed,
            synchronization: data.synchronization,
            version: data.version.as_ref().map(|v| ObservedSaveVersionV1 {
                epoch: data.epoch.clone(),
                token: v.clone(),
            }),
            required_acknowledgment: if t.synchronize {
                NativeSaveRequiredAcknowledgmentV1::Synchronized
            } else {
                NativeSaveRequiredAcknowledgmentV1::Written
            },
            requested_strength_satisfied: t.qualified() && !faulted,
        })
    }
    pub fn open_native_save_observation_v1(
        &mut self,
        task: &SaveId,
        policy: &NativeOperationObservationPolicyV1,
        limits: NativeOperationObservationLimitsV1,
        f: &NativeOperationObservationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<NativeOperationObservationIdV1, Error> {
        let k = match self
            .owned_save_observation_kind(task, f.context)
            .map_err(resource)
        {
            Ok(k) => kind(k),
            Err(error) => {
                let result = Err(error);
                observe::<NativeOperationObservationIdV1>(
                    r,
                    Boundary::NativeOperationObservationAdmission,
                    &result,
                );
                return result;
            }
        };
        self.open_native_operation_observation(Target::Save(task.clone(), k), policy, limits, f, r)
    }
    pub fn open_native_annotation_read_observation_v1(
        &mut self,
        task: &AnnotationReadId,
        policy: &NativeOperationObservationPolicyV1,
        limits: NativeOperationObservationLimitsV1,
        f: &NativeOperationObservationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<NativeOperationObservationIdV1, Error> {
        self.open_native_operation_observation(
            Target::AnnotationLoad(task.clone()),
            policy,
            limits,
            f,
            r,
        )
    }
    pub fn open_native_source_read_observation_v1(
        &mut self,
        task: &SourceReadIdV1,
        policy: &NativeOperationObservationPolicyV1,
        limits: NativeOperationObservationLimitsV1,
        f: &NativeOperationObservationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<NativeOperationObservationIdV1, Error> {
        self.open_native_operation_observation(
            Target::SourceLoad(task.clone()),
            policy,
            limits,
            f,
            r,
        )
    }
    fn open_native_operation_observation(
        &mut self,
        target: Target,
        policy: &NativeOperationObservationPolicyV1,
        limits: NativeOperationObservationLimitsV1,
        f: &NativeOperationObservationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<NativeOperationObservationIdV1, Error> {
        let result = (|| {
            self.ensure_admission_unfenced()
                .map_err(|e| resource(owner_error(e)))?;
            let pool = self.observation_pool()?;
            self.retention.validate(&pool.reservation).map_err(parent)?;
            if pool.stopped
                || pool.rows.len() >= pool.limits.max_issued
                || pool.rows.iter().flatten().count() >= pool.limits.max_contexts
            {
                return Err(Error::Limit);
            }
            if pool.rows.len() >= pool.rows.capacity() {
                return Err(Error::Faulted);
            }
            let charge = limits.declared_held_bytes()?;
            let evidence = add(
                if matches!(target, Target::SourceLoad(_)) {
                    size_of::<NativeSourceReadReceiptV1>()
                } else {
                    size_of::<SaveEvidence>().max(size_of::<NativeAnnotationReadReceiptV1>())
                },
                self.resources
                    .limits
                    .ok_or(Error::Unavailable)?
                    .max_validator_bytes
                    .checked_mul(if matches!(target, Target::SourceLoad(_)) {
                        4
                    } else {
                        2
                    })
                    .ok_or(Error::Limit)?,
            )?;
            if charge > limits.max_held_bytes
                || input_bytes(f.context, policy)? > limits.max_context_bytes
                || limits.max_reply_bytes < limits.max_context_bytes
                || limits.max_reply_bytes < add(size_of::<NativeOperationWaitReplyV1>(), evidence)?
                || limits.max_control_reply_bytes < size_of::<NativeOperationObservationReplyV1>()
                || limits.max_reference_bytes
                    < add(
                        size_of::<Reference>(),
                        size_of::<NativeOperationProgressIdV1>(),
                    )?
                || limits.max_wait_bytes
                    < add(size_of::<Waiter>(), size_of::<NativeOperationWaitIdV1>())?
                || settings::basis::labels(f.context)
                    .iter()
                    .any(|s| s.is_empty() || s.len() > limits.max_label_bytes)
                || policy.recipient.is_empty()
                || policy.recipient.len() > limits.max_label_bytes
            {
                return Err(Error::Limit);
            }
            if !policy.grant || policy.publication || policy.recipient != f.context.recipient {
                return Err(Error::Unavailable);
            }
            self.native_operation_control(&target, f.context)?;
            if [
                f.authorized,
                f.scope,
                f.continuous,
                f.live,
                f.resources_ready,
                f.retained,
                f.transfer_ready,
                f.preparation_reserved,
                f.registry_ready,
                f.retention_ready,
            ]
            .into_iter()
            .any(|v| v != Some(true))
                || (policy.control && f.control_allowed != Some(true))
                || (policy.effect
                    && (f.operation_receipt_admission_allowed != Some(true)
                        || !receipt_allowed(&target, f)))
            {
                return Err(Error::Unavailable);
            }
            self.retention
                .check(limits.max_held_bytes)
                .map_err(parent)?;
            let context = settings::basis::copy_context(f.context).map_err(|_| Error::Capacity)?;
            let ceiling = NativeOperationObservationPolicyV1 {
                recipient: copy_text(&policy.recipient)?,
                epoch: policy.epoch,
                grant: policy.grant,
                publication: policy.publication,
                control: policy.control,
                effect: policy.effect,
                reasons: policy.reasons,
                details: policy.details,
                recovery: policy.recovery,
            };
            let mut refs = Vec::new();
            refs.try_reserve_exact(add(limits.max_issued, 4)?)
                .map_err(|_| Error::Capacity)?;
            let mut waiters = Vec::new();
            waiters
                .try_reserve_exact(limits.max_waiters)
                .map_err(|_| Error::Capacity)?;
            waiters.resize_with(limits.max_waiters, || None);
            // Actual retained parent/task is checked again before attachment.
            self.native_operation_control(&target, &context)?;
            let marker = Arc::new(());
            let slot = self.observation_pool()?.rows.len();
            let id = NativeOperationObservationIdV1 {
                owner: self.namespace.clone(),
                marker: marker.clone(),
                slot,
            };
            let reservation = self
                .retention
                .reserve(limits.max_held_bytes)
                .map_err(parent)?;
            self.observation_pool_mut()?.rows.push(Some(Record {
                decoder: Default::default(),
                marker,
                target,
                context,
                ceiling,
                limits,
                reservation,
                charge,
                live: true,
                busy: false,
                receipt_faulted: false,
                ordinary_issued: 0,
                control_issued: 0,
                refs,
                last_ordinary: None,
                last_control: None,
                waiters,
                #[cfg(test)]
                fault: None,
            }));
            Ok(id)
        })();
        observe(r, Boundary::NativeOperationObservationAdmission, &result);
        result
    }
    fn observation_current(
        &mut self,
        id: &NativeOperationObservationIdV1,
        p: &NativeOperationObservationPolicyV1,
        f: &NativeOperationObservationFactsV1<'_>,
        control: bool,
    ) -> Result<(bool, bool, (bool, bool)), Error> {
        let row = self.observation_row(id)?;
        self.retention
            .validate(&self.observation_pool()?.reservation)
            .map_err(parent)?;
        self.retention.validate(&row.reservation).map_err(parent)?;
        if !row.live
            || row.context != *f.context
            || row.ceiling.recipient != p.recipient
            || row.ceiling.epoch != p.epoch
        {
            return Err(Error::Unavailable);
        }
        if input_bytes(f.context, p)? > row.limits.max_context_bytes {
            return Err(Error::InvalidRequest);
        }
        if !p.grant
            || [f.authorized, f.scope, f.continuous, f.live, f.retained]
                .into_iter()
                .any(|v| v == Some(false))
        {
            self.observation_row_mut(id)?.close();
            return Err(Error::Unavailable);
        }
        if [
            f.authorized,
            f.scope,
            f.continuous,
            f.live,
            f.retained,
            f.transfer_ready,
        ]
        .into_iter()
        .any(|v| v != Some(true))
            || if control {
                f.control_ready != Some(true)
            } else {
                f.ordinary_ready != Some(true) || f.resources_ready != Some(true)
            }
        {
            return Err(Error::Unavailable);
        }
        let selected_control = row.ceiling.control && p.control && f.control_allowed == Some(true);
        let selected_effect = !control
            && row.ceiling.effect
            && p.effect
            && !row.receipt_faulted
            && receipt_allowed(&row.target, f);
        let bits = match self.native_operation_control(&row.target, &row.context) {
            Ok(bits) => bits,
            Err(Error::Unavailable) => {
                self.observation_row_mut(id)?.close();
                return Err(Error::Unavailable);
            }
            Err(e) => return Err(e),
        };
        Ok((selected_control, selected_effect, bits))
    }
    fn query_observation_inner(
        &mut self,
        id: &NativeOperationObservationIdV1,
        q: NativeOperationObservationQueryV1,
        p: &NativeOperationObservationPolicyV1,
        f: &NativeOperationObservationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<NativeOperationObservationReplyV1, Error> {
        let control = q == NativeOperationObservationQueryV1::MinimalControlStatus;
        let (control_allowed, effect, bits) = self.observation_current(id, p, f, control)?;
        if (q == NativeOperationObservationQueryV1::RetainedOutcome && !effect)
            || (q != NativeOperationObservationQueryV1::RetainedOutcome && !control_allowed)
        {
            return Err(Error::Unavailable);
        }
        let row = self.observation_row(id)?;
        let native = if effect {
            self.native_operation_receipt(&row.target, &row.context)
                .map(Some)
        } else {
            Ok(None)
        };
        observe(r, Boundary::NativeOperationObservationSnapshot, &native);
        let mut receipt = native?;
        #[cfg(test)]
        if effect {
            match row.fault {
                Some(Fault::Panic) => panic!("injected observation fault"),
                Some(Fault::WrongKind) => {
                    if let Some(v) = &mut receipt {
                        match v {
                            NativeOperationReceiptV1::Save(v) => {
                                v.kind = match v.kind {
                                    NativeSaveKindV1::SourceSave => {
                                        NativeSaveKindV1::AnnotationSave
                                    }
                                    NativeSaveKindV1::AnnotationSave => {
                                        NativeSaveKindV1::SourceSave
                                    }
                                }
                            }
                            NativeOperationReceiptV1::AnnotationLoad(_) => {
                                panic!("injected wrong read kind")
                            }
                            NativeOperationReceiptV1::SourceLoad(_) => {
                                panic!("injected wrong source read kind")
                            }
                        }
                    }
                }
                None => {}
            }
        }
        let validation = if receipt
            .as_ref()
            .is_some_and(|v| !row.target.matches(v) || v.bits() != bits)
        {
            Err(Error::Faulted)
        } else {
            Ok(())
        };
        observe(
            r,
            Boundary::NativeOperationObservationResultValidation,
            &validation,
        );
        validation?;
        let signature = Signature {
            control: control_allowed.then_some(bits),
            receipt: receipt.clone(),
        };
        let old = if control {
            row.last_control
                .as_ref()
                .filter(|(s, _)| *s == bits)
                .map(|(_, id)| id)
        } else {
            row.last_ordinary
                .as_ref()
                .filter(|(s, _)| *s == signature)
                .map(|(_, id)| id)
        };
        let fresh = old.is_none();
        if fresh
            && (if control {
                row.control_issued >= 4
            } else {
                row.ordinary_issued >= row.limits.max_issued
            })
        {
            return Err(Error::Limit);
        }
        let token = old.cloned().unwrap_or_else(|| NativeOperationProgressIdV1 {
            owner: self.namespace.clone(),
            context: row.marker.clone(),
            marker: Arc::new(()),
        });
        let frame = if q == NativeOperationObservationQueryV1::RetainedOutcome {
            NativeOperationObservationReplyV1::Receipt {
                receipt: receipt.take().ok_or(Error::Faulted)?,
                observation: token.clone(),
            }
        } else {
            NativeOperationObservationReplyV1::Status {
                phase: phase(bits),
                stopped: bits.0,
                quiescent: bits.1,
                receipt: if control { None } else { receipt },
                observation: token.clone(),
            }
        };
        let bytes = reply_bytes(&frame)?;
        if bytes
            > (if control {
                row.limits.max_control_reply_bytes
            } else {
                row.limits.max_reply_bytes
            })
        {
            return Err(Error::Limit);
        }
        if fresh {
            let row = self.observation_row_mut(id)?;
            if row.refs.len() >= row.refs.capacity() {
                return Err(Error::Faulted);
            }
            row.refs.push(Reference {
                id: token.clone(),
                control,
            });
            if control {
                row.control_issued += 1;
                row.last_control = Some((bits, token));
            } else {
                row.ordinary_issued += 1;
                row.last_ordinary = Some((signature, token));
            }
        }
        Ok(frame)
    }
    fn observation_service<T>(
        &mut self,
        id: &NativeOperationObservationIdV1,
        r: &mut Recorder,
        call: impl FnOnce(&mut Self, &mut Recorder) -> Result<T, Error>,
    ) -> Result<T, Error> {
        if self.observation_row(id)?.busy
            || self.observation_row(id)?.decoder.blocks_native_service()
        {
            return Err(Error::Unavailable);
        }
        self.observation_row_mut(id)?.busy = true;
        let result =
            catch_unwind(AssertUnwindSafe(|| call(self, r))).unwrap_or(Err(Error::Faulted));
        if let Ok(row) = self.observation_row_mut(id) {
            row.busy = false;
            if result.as_ref().err() == Some(&Error::Faulted) {
                row.receipt_faulted = true;
            }
        }
        result
    }
    pub fn query_native_operation_observation_v1(
        &mut self,
        id: &NativeOperationObservationIdV1,
        q: NativeOperationObservationQueryV1,
        p: &NativeOperationObservationPolicyV1,
        f: &NativeOperationObservationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<NativeOperationObservationReplyV1, Error> {
        let result =
            self.observation_service(id, r, |o, r| o.query_observation_inner(id, q, p, f, r));
        observe(r, Boundary::NativeOperationObservationQuery, &result);
        result
    }
    pub fn native_operation_observation_usage_v1(
        &self,
        id: &NativeOperationObservationIdV1,
    ) -> Result<NativeOperationObservationUsageV1, Error> {
        let r = self.observation_row(id)?;
        Ok(NativeOperationObservationUsageV1 {
            live: r.live,
            busy: r.busy,
            receipt_faulted: r.receipt_faulted,
            ordinary_issued: r.ordinary_issued,
            control_issued: r.control_issued,
            references: r.refs.len(),
            waiters: r.waiters.iter().flatten().count(),
            waiting: r.waiters.iter().flatten().filter(|w| w.waiting).count(),
            ended_waiters: r.waiters.iter().flatten().filter(|w| !w.waiting).count(),
            declared_held_bytes: r.charge,
            reserved_ceiling_bytes: r.limits.max_held_bytes,
        })
    }
    pub fn native_operation_observation_pool_usage_v1(
        &self,
    ) -> Result<NativeOperationObservationPoolUsageV1, Error> {
        let p = self.observation_pool()?;
        self.retention.validate(&p.reservation).map_err(parent)?;
        Ok(NativeOperationObservationPoolUsageV1 {
            issued_lifetime: p.rows.len(),
            retained_contexts: p.rows.iter().flatten().count(),
            stopped: p.stopped,
            reserved_pool_bytes: p.limits.required_pool_bytes()?,
        })
    }
    pub fn stop_native_operation_observation_v1(
        &mut self,
        id: &NativeOperationObservationIdV1,
        f: &NativeOperationObservationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = (|| {
            let row = self.observation_row(id)?;
            if row.busy
                || row.decoder.busy()
                || row.context != *f.context
                || f.owner_control_allowed != Some(true)
            {
                return Err(Error::Unavailable);
            }
            self.observation_row_mut(id)?.close();
            Ok(())
        })();
        observe(r, Boundary::NativeOperationObservationClose, &result);
        result
    }
    pub fn retire_native_operation_observation_v1(
        &mut self,
        id: &NativeOperationObservationIdV1,
        f: &NativeOperationObservationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = (|| {
            let row = self.observation_row(id)?;
            if row.busy
                || row.decoder.busy()
                || row.context != *f.context
                || !discharged(f)
                || row.waiters.iter().any(Option::is_some)
            {
                return Err(Error::Unavailable);
            }
            self.retention.validate(&row.reservation).map_err(parent)?;
            row.decoder.validate(&self.retention)?;
            let mut row = self.observation_pool_mut()?.rows[id.slot]
                .take()
                .expect("validated observer");
            self.retention
                .release(&mut row.reservation)
                .map_err(parent)?;
            row.decoder.release(&mut self.retention)?;
            Ok(())
        })();
        observe(r, Boundary::NativeOperationObservationRetirement, &result);
        result
    }
    pub fn stop_native_operation_observation_admission_v1(
        &mut self,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = self.observation_pool_mut().map(|p| p.stopped = true);
        observe(
            r,
            Boundary::NativeOperationObservationConfiguration,
            &result,
        );
        result
    }
    pub fn retire_native_operation_observation_pool_v1(
        &mut self,
        owner_control_allowed: Option<bool>,
        retention_released: Option<bool>,
        delivery_drained: Option<bool>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = (|| {
            let p = self.observation_pool()?;
            if !p.stopped
                || p.rows.iter().any(Option::is_some)
                || [owner_control_allowed, retention_released, delivery_drained]
                    .into_iter()
                    .any(|v| v != Some(true))
            {
                return Err(Error::Unavailable);
            }
            self.retention.validate(&p.reservation).map_err(parent)?;
            let State::Active(mut p) =
                std::mem::replace(&mut self.resources.observation, State::Closed)
            else {
                unreachable!("validated pool")
            };
            self.retention.release(&mut p.reservation).map_err(parent)?;
            Ok(())
        })();
        observe(r, Boundary::NativeOperationObservationRetirement, &result);
        result
    }
}
fn copy_text(s: &str) -> Result<String, Error> {
    let mut v = String::new();
    v.try_reserve_exact(s.len()).map_err(|_| Error::Capacity)?;
    v.push_str(s);
    Ok(v)
}
fn phase(bits: (bool, bool)) -> NativeOperationObservationPhaseV1 {
    if bits.0 || bits.1 {
        NativeOperationObservationPhaseV1::Terminal
    } else {
        NativeOperationObservationPhaseV1::Pending
    }
}
fn reply_bytes(frame: &NativeOperationObservationReplyV1) -> Result<usize, Error> {
    let receipt = match frame {
        NativeOperationObservationReplyV1::Receipt { receipt, .. } => Some(receipt),
        NativeOperationObservationReplyV1::Status { receipt, .. } => receipt.as_ref(),
    };
    let extra = match receipt {
        Some(NativeOperationReceiptV1::Save(r)) => r.version.as_ref().map_or(Ok(0), |v| {
            add(v.token.0.len(), v.epoch.as_ref().map_or(0, |e| e.0.len()))
        })?,
        Some(NativeOperationReceiptV1::AnnotationLoad(r)) => {
            r.witnesses.iter().flatten().try_fold(0, |n, w| match w {
                NativeReadTerminalWitnessV1::End { version, .. } => add(n, version.bytes().len()),
                NativeReadTerminalWitnessV1::Failure(_) => Ok(n),
            })?
        }
        Some(NativeOperationReceiptV1::SourceLoad(r)) => {
            r.witnesses.iter().flatten().try_fold(0, |n, w| match w {
                NativeSourceReadTerminalWitnessV1::End { version, .. } => add(
                    n,
                    add(
                        version.token().len(),
                        version.epoch().map_or(0, <[u8]>::len),
                    )?,
                ),
                NativeSourceReadTerminalWitnessV1::Failure(_) => Ok(n),
            })?
        }
        None => 0,
    };
    add(size_of::<NativeOperationObservationReplyV1>(), extra)
}
fn discharged(f: &NativeOperationObservationControlV1<'_>) -> bool {
    [
        f.owner_control_allowed,
        f.retention_released,
        f.delivery_drained,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
