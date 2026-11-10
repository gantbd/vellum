//! Trusted integration-owned plain transfer tasks (§21.1), not an OS clipboard
//! adapter, cryptographic attestation system or structured-transfer registry.
use super::*;
mod cut;
mod paste;
pub use paste::{
    PasteAdmissionFactsV1, PasteDispatchFactsV1, PasteEvidenceFactsV1, PasteEvidenceV1,
    PasteLimitsV1, PastePublicationFactsV1, PasteReportV1,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use vellum_engine::coordinator::{
    InvocationId, InvocationOutcome, LogicalViewContext, SelectionAfterV1,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextTransferKindV1 {
    Copy,
    Cut,
    Paste,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferLocalDispositionV1 {
    Pending,
    Applied,
    Unchanged,
    NotApplied,
    Stopped,
    Unknown,
}
/// One serialized snapshot of current external evidence and the historical
/// native local decision. This is not an external/local atomic transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CutReportV1 {
    pub external: CopyStatusV1,
    pub local: TransferLocalDispositionV1,
    pub copied_not_cut: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CutAdmissionFactsV1 {
    pub transfer: CopyAdmissionFactsV1,
    pub edit_allowed: Option<bool>,
    pub view_write_allowed: Option<bool>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CutPublicationFactsV1 {
    pub edit_allowed: Option<bool>,
    pub view_write_allowed: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyDestinationV1 {
    SystemClipboard,
    PlatformSelection,
    SessionClipboard,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CopyProviderRegistrationV1 {
    pub destination: CopyDestinationV1,
    pub readable: bool,
    pub writable: bool,
    /// Declared semantics only, never proof of physical transfer/durability.
    pub acknowledgment_profile: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CopyLimitsV1 {
    /// Lifetime registrations; retirement does not reset issuance.
    pub max_provider_registrations: usize,
    pub max_profile_bytes: usize,
    pub max_tasks: usize,
    /// Logical headers + profile/capture data, not allocator/RSS peaks.
    /// Retired provider tombstone headers remain charged until host disposal.
    pub max_held_bytes: usize,
    pub capture: TextTransferCaptureLimitsV1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyErrorV1 {
    Unavailable,
    Denied,
    Limit,
    Capacity,
    IntentMismatch,
    AlreadyDispatched,
    Outstanding,
}
/// Supplied trusted-host premises, not evaluated credentials or inferred grants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CopyAdmissionFactsV1 {
    pub read_allowed: Option<bool>,
    pub disclosure_allowed: Option<bool>,
    pub retention_allowed: Option<bool>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CopyDispatchFactsV1 {
    pub launch_allowed: Option<bool>,
    pub disclosure_allowed: Option<bool>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CopyReleaseFactsV1 {
    pub release_allowed: Option<bool>,
    pub disclosure_allowed: Option<bool>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CopyEvidenceFactsV1 {
    pub authentic: Option<bool>,
    pub evidence_allowed: Option<bool>,
    pub ingress_ready: Option<bool>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CopyRetirementFactsV1 {
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
    pub session_references_released: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyEvidenceV1 {
    Accepted,
    NotApplied,
    Quiescent,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyEvidenceDispositionV1 {
    Recorded,
    Duplicate,
    Contested,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyEffectV1 {
    NotDispatched,
    Unknown,
    Accepted,
    NotApplied,
    Contested,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CopyStatusV1 {
    pub operation: TextTransferKindV1,
    pub effect: CopyEffectV1,
    pub no_export: bool,
    pub stopped: bool,
    pub quiescent: bool,
    pub dispatch_age: Option<Duration>,
    pub first_result_after: Option<Duration>,
    pub quiescent_after: Option<Duration>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyOperationV1 {
    RegisterProvider,
    RevokeProvider,
    Admit,
    Dispatch,
    ReleasePayload,
    RecordEvidence,
    Stop,
    RetireTask,
    RetireProvider,
    AdmitCut,
    ContinueCut,
    AdmitPaste,
    DispatchPaste,
    PasteEvidence,
    ContinuePaste,
}
impl CopyOperationV1 {
    fn index(self) -> usize {
        match self {
            Self::RegisterProvider => 0,
            Self::RevokeProvider => 1,
            Self::Admit => 2,
            Self::Dispatch => 3,
            Self::ReleasePayload => 4,
            Self::RecordEvidence => 5,
            Self::Stop => 6,
            Self::RetireTask => 7,
            Self::RetireProvider => 8,
            Self::AdmitCut => 9,
            Self::ContinueCut => 10,
            Self::AdmitPaste => 11,
            Self::DispatchPaste => 12,
            Self::PasteEvidence => 13,
            Self::ContinuePaste => 14,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CopyOperationUsageV1 {
    pub offers: u64,
    pub refusals: u64,
}
/// Fixed content/path/identity-free counters. Mutable API calls join attempts
/// and refusals; queries do not. Logical utilization is not physical utilization.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CopyUsageV1 {
    operations: [CopyOperationUsageV1; 15],
    pub attempts: u64,
    pub refusals: u64,
    pub admitted: u64,
    pub request_replays: u64,
    pub dispatched: u64,
    pub payload_releases: u64,
    pub receipts_recorded: u64,
    pub receipts_duplicated: u64,
    pub conflicts: u64,
    pub stopped: u64,
    pub retired: u64,
    pub cut_applied: u64,
    pub cut_not_applied: u64,
    pub cut_unknown: u64,
    pub cut_fenced: bool,
    pub paste_applied: u64,
    pub paste_unchanged: u64,
    pub paste_not_applied: u64,
    pub paste_unknown: u64,
    pub paste_fenced: bool,
    pub held_tasks: usize,
    pub held_bytes: usize,
    pub provider_registrations_issued: usize,
    pub providers_retained: usize,
    pub unknown_tasks: usize,
    pub contested_tasks: usize,
    pub provider_work: usize,
    pub oldest_unknown: Option<Duration>,
    pub oldest_provider_work: Option<Duration>,
}
impl CopyUsageV1 {
    /// Separate populations: these per-operation counters partition attempts,
    /// not additional requests to sum on top of the aggregate rollup.
    pub fn operation(&self, operation: CopyOperationV1) -> CopyOperationUsageV1 {
        self.operations[operation.index()]
    }
}
/// Private, non-cloneable identity of this registry's exact full registration.
pub struct CopyProviderPortV1 {
    identity: Arc<()>,
}
/// Original native invocation identity; cloning is neither authority nor retry.
#[derive(Clone)]
pub struct CopyTaskV1 {
    origin: InvocationId,
}
impl CopyTaskV1 {
    pub fn origin(&self) -> &InvocationId {
        &self.origin
    }
}
/// Issued once. Opaque task/registration/capture binding without payload or raw
/// source root. Loss leaves unknown work; it does not authorize re-dispatch.
pub struct CopyDispatchV1 {
    origin: InvocationId,
    provider: Arc<()>,
}
macro_rules! redacted {
    ($($t:ty),+ $(,)?) => { $(impl std::fmt::Debug for $t {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(concat!(stringify!($t), "(<redacted>)"))
        }
    })+ };
}
redacted!(CopyProviderPortV1, CopyTaskV1, CopyDispatchV1);
struct Provider {
    identity: Arc<()>,
    registration: Option<CopyProviderRegistrationV1>,
    active: bool,
}
enum Capture {
    Export(TextTransferCaptureV1),
    Target {
        context: LogicalViewContext,
        selections: vellum_engine::selection::SelectionSet,
    },
}
impl Capture {
    fn context(&self) -> &LogicalViewContext {
        match self {
            Self::Export(c) => c.context(),
            Self::Target { context, .. } => context,
        }
    }
    fn plain(&self) -> Option<&str> {
        match self {
            Self::Export(c) => c.plain(),
            Self::Target { .. } => None,
        }
    }
}
struct Task {
    operation: TextTransferKindV1,
    local_started: bool,
    local_faulted: bool,
    claim: InvocationClaimV1,
    capture: Capture,
    paste: Option<paste::Payloads>,
    provider: usize,
    charge: usize,
    dispatched: bool,
    stopped: bool,
    accepted: bool,
    not_applied: bool,
    contested: bool,
    quiescent: bool,
    start: Option<Instant>,
    first_result_after: Option<Duration>,
    quiescent_after: Option<Duration>,
}
impl Task {
    fn effect(&self) -> CopyEffectV1 {
        if self.contested {
            return CopyEffectV1::Contested;
        }
        match (self.dispatched, self.accepted, self.not_applied) {
            (false, _, _) => CopyEffectV1::NotDispatched,
            (true, false, false) => CopyEffectV1::Unknown,
            (true, true, false) => CopyEffectV1::Accepted,
            (true, false, true) => CopyEffectV1::NotApplied,
            (true, true, true) => CopyEffectV1::Contested,
        }
    }
}
pub(super) struct CopyTransfers {
    limits: CopyLimitsV1,
    providers: Vec<Provider>,
    tasks: Vec<Task>,
    usage: CopyUsageV1,
    timing: NativeTimingProfileV1,
    paste_limits: Option<PasteLimitsV1>,
}
fn fail(error: CopyErrorV1) -> SessionError {
    SessionError::CopyTransfer(error)
}
fn yes(value: Option<bool>) -> bool {
    value == Some(true)
}
fn add(a: usize, b: usize) -> Result<usize, SessionError> {
    a.checked_add(b).ok_or(fail(CopyErrorV1::Limit))
}
// Private local clock only. Failure is missing timing, not task failure.
fn instant() -> Option<Instant> {
    std::panic::catch_unwind(Instant::now).ok()
}
fn age(start: Option<Instant>) -> Option<Duration> {
    let start = start?;
    instant()?.checked_duration_since(start)
}
impl CopyTransfers {
    pub(super) fn has_obligations(&self) -> bool {
        !self.tasks.is_empty() || self.providers.iter().any(|p| p.registration.is_some())
    }
    fn provider(&self, port: &CopyProviderPortV1) -> Result<usize, SessionError> {
        self.providers
            .iter()
            .position(|p| p.registration.is_some() && Arc::ptr_eq(&p.identity, &port.identity))
            .ok_or(fail(CopyErrorV1::Unavailable))
    }
    fn task(&self, task: &CopyTaskV1) -> Result<usize, SessionError> {
        self.tasks
            .iter()
            .position(|t| t.claim.origin() == &task.origin)
            .ok_or(fail(CopyErrorV1::Unavailable))
    }
    fn dispatch(
        &self,
        port: &CopyProviderPortV1,
        dispatch: &CopyDispatchV1,
    ) -> Result<usize, SessionError> {
        let provider = self.provider(port)?;
        if !Arc::ptr_eq(&dispatch.provider, &port.identity) {
            return Err(fail(CopyErrorV1::Unavailable));
        }
        self.tasks
            .iter()
            .position(|t| {
                t.dispatched && t.claim.origin() == &dispatch.origin && t.provider == provider
            })
            .ok_or(fail(CopyErrorV1::Unavailable))
    }
    fn charge(&self, extra: usize) -> Result<usize, SessionError> {
        let total = add(self.usage.held_bytes, extra)?;
        if total > self.limits.max_held_bytes {
            return Err(fail(CopyErrorV1::Limit));
        }
        Ok(total)
    }
}
impl<A: ResourceAdapter> LocalHost<A> {
    /// Configure once explicitly; no product limits or production provider are
    /// selected. Native invocation/view profiles are also needed for admission.
    pub fn configure_copy_transfers(&mut self, limits: CopyLimitsV1) -> Result<(), SessionError> {
        if self.copy_transfers.is_some() {
            return Err(fail(CopyErrorV1::Unavailable));
        }
        self.copy_transfers = Some(CopyTransfers {
            limits,
            providers: Vec::new(),
            tasks: Vec::new(),
            usage: CopyUsageV1::default(),
            timing: self.diagnostics.native_timing_profile(),
            paste_limits: None,
        });
        Ok(())
    }
    fn copies(&self) -> Result<&CopyTransfers, SessionError> {
        self.copy_transfers
            .as_ref()
            .ok_or(fail(CopyErrorV1::Unavailable))
    }
    fn copies_mut(&mut self) -> Result<&mut CopyTransfers, SessionError> {
        self.copy_transfers
            .as_mut()
            .ok_or(fail(CopyErrorV1::Unavailable))
    }
    fn copy_result<T>(
        &mut self,
        operation: CopyOperationV1,
        result: Result<T, SessionError>,
    ) -> Result<T, SessionError> {
        if let Some(copies) = &mut self.copy_transfers {
            copies.usage.attempts = copies.usage.attempts.saturating_add(1);
            let phase = &mut copies.usage.operations[operation.index()];
            phase.offers = phase.offers.saturating_add(1);
            if result.is_err() {
                copies.usage.refusals = copies.usage.refusals.saturating_add(1);
                phase.refusals = phase.refusals.saturating_add(1);
            }
        }
        result
    }
    pub fn register_copy_provider(
        &mut self,
        registration: CopyProviderRegistrationV1,
    ) -> Result<CopyProviderPortV1, SessionError> {
        let result = (|| {
            let copies = self.copies_mut()?;
            if registration.acknowledgment_profile.is_empty()
                || registration.acknowledgment_profile.len() > copies.limits.max_profile_bytes
                || copies.providers.len() >= copies.limits.max_provider_registrations
            {
                return Err(fail(CopyErrorV1::Limit));
            }
            let total = copies.charge(add(
                std::mem::size_of::<Provider>(),
                registration.acknowledgment_profile.len(),
            )?)?;
            copies
                .providers
                .try_reserve_exact(1)
                .map_err(|_| fail(CopyErrorV1::Capacity))?;
            let identity = Arc::new(());
            copies.providers.push(Provider {
                identity: identity.clone(),
                registration: Some(registration),
                active: true,
            });
            copies.usage.held_bytes = total;
            copies.usage.provider_registrations_issued = copies.providers.len();
            copies.usage.providers_retained += 1;
            Ok(CopyProviderPortV1 { identity })
        })();
        self.copy_result(CopyOperationV1::RegisterProvider, result)
    }
    pub fn copy_provider_registration(
        &self,
        port: &CopyProviderPortV1,
    ) -> Result<&CopyProviderRegistrationV1, SessionError> {
        let copies = self.copies()?;
        copies.providers[copies.provider(port)?]
            .registration
            .as_ref()
            .ok_or(fail(CopyErrorV1::Unavailable))
    }
    /// Permanent revocation stops launch/release, not authentic late evidence.
    /// A new registration has a new identity; equal fields cannot revive this one.
    pub fn revoke_copy_provider(&mut self, port: &CopyProviderPortV1) -> Result<(), SessionError> {
        let result = (|| {
            let c = self.copies_mut()?;
            let i = c.provider(port)?;
            c.providers[i].active = false;
            Ok(())
        })();
        self.copy_result(CopyOperationV1::RevokeProvider, result)
    }
    /// Matching replay retrieves the original immutable capture/task, not new
    /// focus or source. All-empty copy is retained as no-export, never clearing.
    pub fn admit_copy(
        &mut self,
        origin: &InvocationId,
        context: &LogicalViewContext,
        provider: &CopyProviderPortV1,
        facts: CopyAdmissionFactsV1,
    ) -> Result<CopyTaskV1, SessionError> {
        self.admit_export(origin, context, provider, facts, TextTransferKindV1::Copy)
    }
    /// Same original export task, with a private publication-capable claim.
    /// No source edit is prepared or published before acknowledgment.
    pub fn admit_cut(
        &mut self,
        origin: &InvocationId,
        context: &LogicalViewContext,
        provider: &CopyProviderPortV1,
        facts: CutAdmissionFactsV1,
    ) -> Result<CopyTaskV1, SessionError> {
        if self.copies()?.usage.cut_fenced {
            return self.copy_result(
                CopyOperationV1::AdmitCut,
                Err(fail(CopyErrorV1::Unavailable)),
            );
        }
        if !yes(facts.edit_allowed) || !yes(facts.view_write_allowed) {
            return self.copy_result(CopyOperationV1::AdmitCut, Err(fail(CopyErrorV1::Denied)));
        }
        self.admit_export(
            origin,
            context,
            provider,
            facts.transfer,
            TextTransferKindV1::Cut,
        )
    }
    fn admit_export(
        &mut self,
        origin: &InvocationId,
        context: &LogicalViewContext,
        provider: &CopyProviderPortV1,
        facts: CopyAdmissionFactsV1,
        operation: TextTransferKindV1,
    ) -> Result<CopyTaskV1, SessionError> {
        let result = (|| {
            if !yes(facts.read_allowed)
                || !yes(facts.disclosure_allowed)
                || !yes(facts.retention_allowed)
            {
                return Err(fail(CopyErrorV1::Denied));
            }
            let copies = self.copies_mut()?;
            let provider = copies.provider(provider)?;
            if let Some(task) = copies.tasks.iter().find(|t| t.claim.origin() == origin) {
                if task.provider != provider
                    || task.capture.context() != context
                    || task.operation != operation
                {
                    return Err(fail(CopyErrorV1::IntentMismatch));
                }
                copies.usage.request_replays = copies.usage.request_replays.saturating_add(1);
                return Ok(CopyTaskV1 {
                    origin: origin.clone(),
                });
            }
            let p = &copies.providers[provider];
            if !p.active || !p.registration.as_ref().is_some_and(|r| r.writable) {
                return Err(fail(CopyErrorV1::Unavailable));
            }
            if copies.tasks.len() >= copies.limits.max_tasks {
                return Err(fail(CopyErrorV1::Limit));
            }
            let limits = copies.limits.capture;
            if operation == TextTransferKindV1::Cut && !self.owner.can_retain_literal_intent_v1(0) {
                return Err(fail(CopyErrorV1::Limit));
            }
            let capture = self.capture_text_transfer(origin, context, limits)?;
            let charge = add(
                std::mem::size_of::<Task>(),
                capture.usage().logical_bytes - std::mem::size_of::<TextTransferCaptureV1>(),
            )?;
            let copies = self.copies_mut()?;
            let total = copies.charge(charge)?;
            copies
                .tasks
                .try_reserve_exact(1)
                .map_err(|_| fail(CopyErrorV1::Capacity))?;
            let kind = match operation {
                TextTransferKindV1::Copy => InvocationClaimKindV1::SourceNeutral,
                TextTransferKindV1::Cut => InvocationClaimKindV1::SourcePublication,
                TextTransferKindV1::Paste => return Err(fail(CopyErrorV1::Unavailable)),
            };
            let claim = self.claim_invocation(origin, kind)?;
            // Allocation/capture precede installation. No source mutation,
            // provider I/O or callback can intervene in this exclusive call.
            let copies = self.copies_mut()?;
            copies.tasks.push(Task {
                operation,
                local_started: false,
                local_faulted: false,
                claim,
                capture: Capture::Export(capture),
                paste: None,
                provider,
                charge,
                dispatched: false,
                stopped: false,
                accepted: false,
                not_applied: false,
                contested: false,
                quiescent: true,
                start: None,
                first_result_after: None,
                quiescent_after: None,
            });
            copies.usage.held_tasks = copies.tasks.len();
            copies.usage.held_bytes = total;
            copies.usage.admitted = copies.usage.admitted.saturating_add(1);
            Ok(CopyTaskV1 {
                origin: origin.clone(),
            })
        })();
        self.copy_result(
            match operation {
                TextTransferKindV1::Copy => CopyOperationV1::Admit,
                TextTransferKindV1::Cut => CopyOperationV1::AdmitCut,
                TextTransferKindV1::Paste => CopyOperationV1::AdmitPaste,
            },
            result,
        )
    }
    /// One dispatch token, never an implicit external retry. No OS I/O occurs.
    pub fn dispatch_copy(
        &mut self,
        task: &CopyTaskV1,
        facts: CopyDispatchFactsV1,
    ) -> Result<CopyDispatchV1, SessionError> {
        let result = (|| {
            if !yes(facts.launch_allowed) || !yes(facts.disclosure_allowed) {
                return Err(fail(CopyErrorV1::Denied));
            }
            let c = self.copies_mut()?;
            let i = c.task(task)?;
            let t = &c.tasks[i];
            let p = &c.providers[t.provider];
            if t.operation == TextTransferKindV1::Paste
                || !p.active
                || !p.registration.as_ref().is_some_and(|r| r.writable)
                || t.stopped
                || t.capture.plain().is_none()
            {
                return Err(fail(CopyErrorV1::Unavailable));
            }
            if t.dispatched {
                return Err(fail(CopyErrorV1::AlreadyDispatched));
            }
            let token = CopyDispatchV1 {
                origin: t.claim.origin().clone(),
                provider: p.identity.clone(),
            };
            let t = &mut c.tasks[i];
            t.dispatched = true;
            t.quiescent = false;
            t.start = match c.timing {
                NativeTimingProfileV1::Off => None,
                NativeTimingProfileV1::ProcessMonotonicV1 => instant(),
            };
            c.usage.dispatched = c.usage.dispatched.saturating_add(1);
            Ok(token)
        })();
        self.copy_result(CopyOperationV1::Dispatch, result)
    }
    /// Bounded copied plain data, no raw source/history loan. The trusted
    /// recipient owns this new allocation; task accounting cannot bound all
    /// copies retained outside this host. Release rechecks current named rights.
    pub fn release_copy_plain(
        &mut self,
        provider: &CopyProviderPortV1,
        dispatch: &CopyDispatchV1,
        facts: CopyReleaseFactsV1,
    ) -> Result<String, SessionError> {
        let result = (|| {
            if !yes(facts.release_allowed) || !yes(facts.disclosure_allowed) {
                return Err(fail(CopyErrorV1::Denied));
            }
            let c = self.copies_mut()?;
            let i = c.dispatch(provider, dispatch)?;
            let t = &c.tasks[i];
            if t.operation == TextTransferKindV1::Paste
                || !c.providers[t.provider].active
                || t.stopped
                || t.accepted
                || t.not_applied
                || t.quiescent
            {
                return Err(fail(CopyErrorV1::Unavailable));
            }
            let plain = t.capture.plain().ok_or(fail(CopyErrorV1::Unavailable))?;
            let mut out = String::new();
            out.try_reserve_exact(plain.len())
                .map_err(|_| fail(CopyErrorV1::Capacity))?;
            out.push_str(plain);
            c.usage.payload_releases = c.usage.payload_releases.saturating_add(1);
            Ok(out)
        })();
        self.copy_result(CopyOperationV1::ReleasePayload, result)
    }
    /// Authenticate/correlate before duplicate/conflict handling. Fixed-shape
    /// Accepted/NotApplied witnesses occupy two bounded bits; contradiction is
    /// retained as contested, never overwritten. Stop/revoke do not erase facts.
    pub fn record_copy_evidence(
        &mut self,
        provider: &CopyProviderPortV1,
        dispatch: &CopyDispatchV1,
        evidence: CopyEvidenceV1,
        facts: CopyEvidenceFactsV1,
    ) -> Result<CopyEvidenceDispositionV1, SessionError> {
        let result = (|| {
            if !yes(facts.authentic) || !yes(facts.evidence_allowed) || !yes(facts.ingress_ready) {
                return Err(fail(CopyErrorV1::Denied));
            }
            let c = self.copies_mut()?;
            let i = c.dispatch(provider, dispatch)?;
            let t = &mut c.tasks[i];
            if t.operation == TextTransferKindV1::Paste {
                return Err(fail(CopyErrorV1::Unavailable));
            }
            let duplicate = match evidence {
                CopyEvidenceV1::Accepted => t.accepted,
                CopyEvidenceV1::NotApplied => t.not_applied,
                CopyEvidenceV1::Quiescent => t.quiescent,
            };
            if duplicate {
                c.usage.receipts_duplicated = c.usage.receipts_duplicated.saturating_add(1);
                return Ok(CopyEvidenceDispositionV1::Duplicate);
            }
            match evidence {
                CopyEvidenceV1::Accepted | CopyEvidenceV1::NotApplied => {
                    if !t.accepted && !t.not_applied {
                        t.first_result_after = age(t.start);
                    }
                    match evidence {
                        CopyEvidenceV1::Accepted => t.accepted = true,
                        _ => t.not_applied = true,
                    }
                }
                CopyEvidenceV1::Quiescent => {
                    t.quiescent = true;
                    t.quiescent_after = age(t.start);
                }
            }
            c.usage.receipts_recorded = c.usage.receipts_recorded.saturating_add(1);
            if evidence != CopyEvidenceV1::Quiescent && t.accepted && t.not_applied {
                c.usage.conflicts = c.usage.conflicts.saturating_add(1);
                Ok(CopyEvidenceDispositionV1::Contested)
            } else {
                Ok(CopyEvidenceDispositionV1::Recorded)
            }
        })();
        self.copy_result(CopyOperationV1::RecordEvidence, result)
    }
    /// Stop denies launch/release; dispatched unknown work stays unresolved.
    pub fn stop_copy(&mut self, task: &CopyTaskV1) -> Result<(), SessionError> {
        let result = (|| {
            let c = self.copies_mut()?;
            let i = c.task(task)?;
            if !c.tasks[i].stopped {
                c.tasks[i].stopped = true;
                c.usage.stopped = c.usage.stopped.saturating_add(1);
            }
            Ok(())
        })();
        self.copy_result(CopyOperationV1::Stop, result)
    }
    pub fn copy_status(&self, task: &CopyTaskV1) -> Result<CopyStatusV1, SessionError> {
        let c = self.copies()?;
        let t = &c.tasks[c.task(task)?];
        Ok(CopyStatusV1 {
            operation: t.operation,
            effect: t.effect(),
            no_export: t.operation != TextTransferKindV1::Paste && t.capture.plain().is_none(),
            stopped: t.stopped,
            quiescent: t.quiescent,
            dispatch_age: age(t.start),
            first_result_after: t.first_result_after,
            quiescent_after: t.quiescent_after,
        })
    }
    /// Unknown/contested effects and live work cannot retire. Separate host
    /// retention/drain/session truth is not inferred from acknowledgment.
    pub fn retire_copy(
        &mut self,
        task: &CopyTaskV1,
        facts: CopyRetirementFactsV1,
    ) -> Result<(), SessionError> {
        let result = (|| {
            let is_paste = {
                let c = self.copies()?;
                c.tasks[c.task(task)?].operation == TextTransferKindV1::Paste
            };
            if is_paste
                && matches!(
                    self.paste_report(task)?.local,
                    TransferLocalDispositionV1::Pending | TransferLocalDispositionV1::Unknown
                )
            {
                return Err(fail(CopyErrorV1::Outstanding));
            }
            let is_cut = {
                let c = self.copies()?;
                c.tasks[c.task(task)?].operation == TextTransferKindV1::Cut
            };
            if is_cut
                && matches!(
                    self.cut_disposition(task)?,
                    TransferLocalDispositionV1::Pending | TransferLocalDispositionV1::Unknown
                )
            {
                return Err(fail(CopyErrorV1::Outstanding));
            }
            if !yes(facts.retention_released)
                || !yes(facts.delivery_drained)
                || !yes(facts.session_references_released)
            {
                return Err(fail(CopyErrorV1::Denied));
            }
            let Self {
                owner,
                diagnostics,
                copy_transfers,
                invocation_claims,
                ..
            } = self;
            let c = copy_transfers
                .as_mut()
                .ok_or(fail(CopyErrorV1::Unavailable))?;
            let i = c.task(task)?;
            let t = &c.tasks[i];
            if !t.quiescent
                || matches!(t.effect(), CopyEffectV1::Unknown | CopyEffectV1::Contested)
                || (!t.dispatched && !t.stopped && t.capture.plain().is_some())
            {
                return Err(fail(CopyErrorV1::Outstanding));
            }
            owner.retire_invocation(t.claim.origin(), diagnostics)?;
            // No callback/I/O/allocation between native retirement and removal
            // of the private task/claim. Refusal above preserves both owners.
            let removed = c.tasks.swap_remove(i);
            invocation_claims.remove_retired(removed.claim.origin());
            c.usage.held_bytes -= removed.charge;
            c.usage.held_tasks = c.tasks.len();
            c.usage.retired = c.usage.retired.saturating_add(1);
            Ok(())
        })();
        self.copy_result(CopyOperationV1::RetireTask, result)
    }
    pub fn retire_copy_provider(
        &mut self,
        port: &CopyProviderPortV1,
        facts: CopyRetirementFactsV1,
    ) -> Result<(), SessionError> {
        let result = (|| {
            if !yes(facts.retention_released)
                || !yes(facts.delivery_drained)
                || !yes(facts.session_references_released)
            {
                return Err(fail(CopyErrorV1::Denied));
            }
            let c = self.copies_mut()?;
            let i = c.provider(port)?;
            if c.tasks.iter().any(|t| t.provider == i) {
                return Err(fail(CopyErrorV1::Outstanding));
            }
            let registration = c.providers[i]
                .registration
                .take()
                .ok_or(fail(CopyErrorV1::Unavailable))?;
            c.providers[i].active = false;
            c.usage.held_bytes -= registration.acknowledgment_profile.len();
            c.usage.providers_retained -= 1;
            Ok(())
        })();
        self.copy_result(CopyOperationV1::RetireProvider, result)
    }
    /// Local monotonic wait criteria are independent of bounded trace pressure.
    /// Off/clock failure is None, not zero. Counts never label content or IDs.
    pub fn copy_usage(&self) -> Result<CopyUsageV1, SessionError> {
        let c = self.copies()?;
        let mut u = c.usage;
        for t in &c.tasks {
            if t.effect() == CopyEffectV1::Unknown {
                u.unknown_tasks += 1;
                u.oldest_unknown = u.oldest_unknown.max(age(t.start));
            }
            if t.effect() == CopyEffectV1::Contested {
                u.contested_tasks += 1;
            }
            if t.dispatched && !t.quiescent {
                u.provider_work += 1;
                u.oldest_provider_work = u.oldest_provider_work.max(age(t.start));
            }
        }
        Ok(u)
    }
}
