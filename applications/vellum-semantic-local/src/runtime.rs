//! Embedded semantic composition, explicit deadline activation, and provider
//! replacement orchestration.
//!
//! The runtime owns no clock. Composition supplies opaque monotonic instants
//! from one clock domain and separately registers exact `(client, request)`
//! deadlines. Each drive expires only a bounded prefix and performs one bounded
//! provider pump; it never retries a query.
//!
//! Replacement is staged: a fresh supervised adapter must independently reach
//! ready state, then the semantic service replays all exact snapshots before a
//! single swap. Failure returns the candidate. Success returns the old provider
//! with its exact close plan; composition drives protocol shutdown or explicitly
//! escalates it, and terminal outcomes require transport cleanup evidence.

use std::{
    collections::{BTreeMap, BTreeSet},
    os::unix::ffi::OsStrExt as _,
    path::Path,
};

use vellum_adapter_semantic_lsp::{
    LspAbortOutcomeV1, LspAdapterProfileV1, LspAdapterV1, LspFrameDecoderV1,
    LspFreshStartedAdapterV1, LspIndexReadinessPortV1, LspInitializationProfileV1, LspLimitsV1,
    LspProviderQualificationEvidenceV1, LspSessionStateV1, LspShutdownOutcomeV1, LspStartOutcomeV1,
    LspTransportPortV1, LspTransportTerminalV1, LspUsageV1, LspVirtualDocumentIssuerV1,
    SemanticGenerationIssuerV1,
};
use vellum_host::semantic::{SemanticCapabilitySetV1, SemanticLimitsV1, SemanticRequestRefV1};
use vellum_semantic_client::{SemanticClientV1, SemanticServicePortV1, bind_semantic_client_v1};
use vellum_semantic_service::{
    SemanticBridgeForcedAbandonmentReceiptV1, SemanticBridgeForcedRetiringProviderV1,
    SemanticBridgeRetiringProviderV1, SemanticClientRefV1, SemanticProviderPortV1,
    SemanticProviderReadinessV1, SemanticProviderReplacementFailureV1,
    SemanticProviderReplacementRefusalV1, SemanticProviderRetirementStateV1,
    SemanticProviderRetirementV1, SemanticServiceLimitsV1, SemanticServiceRefusalV1,
    SemanticServiceUsageV1, SemanticServiceV1,
};

use crate::{
    LocalLspTransportV1, SemanticProcessLaunchRefusalV1, SemanticProviderProfileV1,
    launch_local_lsp_v1,
};

fn workspace_initialization_v1(
    workspace_root: &Path,
    limits: LspLimitsV1,
) -> Option<LspInitializationProfileV1> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let path = workspace_root.as_os_str().as_bytes();
    let mut uri = String::with_capacity(path.len().saturating_mul(3).saturating_add(7));
    uri.push_str("file://");
    for byte in path.iter().copied() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'.' | b'_' | b'~') {
            uri.push(char::from(byte));
        } else {
            uri.push('%');
            uri.push(char::from(HEX[usize::from(byte >> 4)]));
            uri.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        if uri.len() > limits.max_uri_bytes {
            return None;
        }
    }
    LspInitializationProfileV1::try_workspace(uri, "vellum-workspace".to_owned(), None, limits)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalSemanticRuntimeRefusalV1 {
    Launch(SemanticProcessLaunchRefusalV1),
    InvalidAdapter,
    StartUnavailable,
    StartRefused,
    StartCleanupUnconfirmed(LspStartOutcomeV1),
    InvalidService,
    InvalidScheduler,
}

/// Failed launch product. When startup cleanup is not confirmed, this owns the
/// only adapter evidence and must be driven or explicitly escalated.
#[must_use = "startup cleanup may still require observation or escalation"]
pub struct LocalSemanticRuntimeFailureV1<G, R, U> {
    refusal: LocalSemanticRuntimeRefusalV1,
    cleanup: Option<LocalSemanticStartupCleanupV1<G, R, U>>,
}

impl<G, R, U> LocalSemanticRuntimeFailureV1<G, R, U> {
    pub const fn refusal(&self) -> LocalSemanticRuntimeRefusalV1 {
        self.refusal
    }

    pub fn into_cleanup(self) -> Option<LocalSemanticStartupCleanupV1<G, R, U>> {
        self.cleanup
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalSemanticRuntimeStateV1 {
    pub readiness: SemanticProviderReadinessV1,
    pub capabilities: SemanticCapabilitySetV1,
}

pub const MAX_LOCAL_SEMANTIC_DEADLINES_V1: usize = 4_096;
pub const MAX_LOCAL_SEMANTIC_EXPIRATIONS_PER_DRIVE_V1: usize = 256;

/// Opaque monotonic tick from one composition-selected clock domain. Values
/// from different clocks, processes, units or epochs must never be mixed.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LocalSemanticInstantV1(u64);

impl LocalSemanticInstantV1 {
    pub const fn from_composition_tick(tick: u64) -> Self {
        Self(tick)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalSemanticSchedulerLimitsV1 {
    pub max_deadlines: usize,
    pub max_expirations_per_drive: usize,
}

impl LocalSemanticSchedulerLimitsV1 {
    pub const fn reference() -> Self {
        Self {
            max_deadlines: MAX_LOCAL_SEMANTIC_DEADLINES_V1,
            max_expirations_per_drive: MAX_LOCAL_SEMANTIC_EXPIRATIONS_PER_DRIVE_V1,
        }
    }

    pub const fn is_valid(self) -> bool {
        self.max_deadlines > 0
            && self.max_deadlines <= MAX_LOCAL_SEMANTIC_DEADLINES_V1
            && self.max_expirations_per_drive > 0
            && self.max_expirations_per_drive <= MAX_LOCAL_SEMANTIC_EXPIRATIONS_PER_DRIVE_V1
            && self.max_expirations_per_drive <= self.max_deadlines
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalSemanticDeadlineAdmissionV1 {
    Scheduled,
    AlreadyScheduled,
    Conflict,
    NotPending,
    Isolation,
    Capacity,
    Elapsed,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LocalSemanticSchedulerUsageV1 {
    pub scheduled: u64,
    pub expired: u64,
    pub no_longer_pending: u64,
    pub expiry_refusals: u64,
    pub retryable_expiry_refusals: u64,
    pub terminal_expiry_refusals: u64,
    pub cleared_before_deadline: u64,
    pub clock_regressions: u64,
    /// Maximum scheduler activation lateness in the caller's opaque tick unit.
    pub activation_lateness_ticks_max: u64,
    pub current: usize,
    pub high_water: usize,
    pub saturated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalSemanticDriveRefusalV1 {
    ClockRegressed,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LocalSemanticDriveV1 {
    pub attempted: usize,
    pub expired: usize,
    pub no_longer_pending: usize,
    pub refused: usize,
    pub retained_retryable: usize,
    pub remaining_due: bool,
}

struct LocalSemanticDeadlineSchedulerV1 {
    limits: LocalSemanticSchedulerLimitsV1,
    deadlines_by_key: BTreeMap<(SemanticClientRefV1, SemanticRequestRefV1), LocalSemanticInstantV1>,
    deadlines_by_due: BTreeSet<(
        LocalSemanticInstantV1,
        SemanticClientRefV1,
        SemanticRequestRefV1,
    )>,
    fair_cursor: Option<(
        LocalSemanticInstantV1,
        SemanticClientRefV1,
        SemanticRequestRefV1,
    )>,
    last_now: Option<LocalSemanticInstantV1>,
    usage: LocalSemanticSchedulerUsageV1,
}

trait SemanticDeadlineCoreV1 {
    fn deadline_is_pending_for(
        &self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> Result<bool, SemanticServiceRefusalV1>;
    fn deadline_expire(
        &mut self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> Result<bool, SemanticServiceRefusalV1>;
    fn deadline_drive_once(&mut self);
}

impl LocalSemanticDeadlineSchedulerV1 {
    fn try_new(limits: LocalSemanticSchedulerLimitsV1) -> Option<Self> {
        limits.is_valid().then_some(Self {
            limits,
            deadlines_by_key: BTreeMap::new(),
            deadlines_by_due: BTreeSet::new(),
            fair_cursor: None,
            last_now: None,
            usage: LocalSemanticSchedulerUsageV1::default(),
        })
    }

    fn schedule<C: SemanticDeadlineCoreV1>(
        &mut self,
        core: &C,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
        deadline: LocalSemanticInstantV1,
    ) -> LocalSemanticDeadlineAdmissionV1 {
        if self.last_now.is_some_and(|now| deadline <= now) {
            return LocalSemanticDeadlineAdmissionV1::Elapsed;
        }
        match core.deadline_is_pending_for(client, request) {
            Ok(true) => {}
            Ok(false) => return LocalSemanticDeadlineAdmissionV1::NotPending,
            Err(_) => return LocalSemanticDeadlineAdmissionV1::Isolation,
        }
        let key = (client, request);
        if let Some(current) = self.deadlines_by_key.get(&key) {
            return if *current == deadline {
                LocalSemanticDeadlineAdmissionV1::AlreadyScheduled
            } else {
                LocalSemanticDeadlineAdmissionV1::Conflict
            };
        }
        if self.deadlines_by_key.len() == self.limits.max_deadlines {
            return LocalSemanticDeadlineAdmissionV1::Capacity;
        }
        self.deadlines_by_key.insert(key, deadline);
        assert!(self.deadlines_by_due.insert((deadline, client, request)));
        self.usage.current = self.deadlines_by_key.len();
        self.usage.high_water = self.usage.high_water.max(self.usage.current);
        inc_runtime(&mut self.usage.scheduled, &mut self.usage.saturated);
        LocalSemanticDeadlineAdmissionV1::Scheduled
    }

    fn drive<C: SemanticDeadlineCoreV1>(
        &mut self,
        core: &mut C,
        now: LocalSemanticInstantV1,
    ) -> Result<LocalSemanticDriveV1, LocalSemanticDriveRefusalV1> {
        if self.last_now.is_some_and(|prior| now < prior) {
            inc_runtime(&mut self.usage.clock_regressions, &mut self.usage.saturated);
            return Err(LocalSemanticDriveRefusalV1::ClockRegressed);
        }
        self.last_now = Some(now);
        let due: Vec<_> = self
            .deadlines_by_due
            .iter()
            .take_while(|(deadline, _, _)| *deadline <= now)
            .copied()
            .collect();
        let start = self
            .fair_cursor
            .map_or(0, |cursor| due.partition_point(|entry| *entry <= cursor));
        let selected: Vec<_> = due[start..]
            .iter()
            .chain(due[..start].iter())
            .take(self.limits.max_expirations_per_drive)
            .copied()
            .collect();
        let mut report = LocalSemanticDriveV1::default();
        let mut last_attempted = None;
        for (deadline, client, request) in selected {
            last_attempted = Some((deadline, client, request));
            self.usage.activation_lateness_ticks_max = self
                .usage
                .activation_lateness_ticks_max
                .max(now.0.saturating_sub(deadline.0));
            report.attempted += 1;
            match core.deadline_expire(client, request) {
                Ok(true) => {
                    self.remove(client, request, deadline);
                    report.expired += 1;
                    inc_runtime(&mut self.usage.expired, &mut self.usage.saturated);
                }
                Ok(false) => {
                    self.remove(client, request, deadline);
                    report.no_longer_pending += 1;
                    inc_runtime(&mut self.usage.no_longer_pending, &mut self.usage.saturated);
                }
                Err(_) if matches!(core.deadline_is_pending_for(client, request), Ok(true)) => {
                    report.refused += 1;
                    report.retained_retryable += 1;
                    inc_runtime(&mut self.usage.expiry_refusals, &mut self.usage.saturated);
                    inc_runtime(
                        &mut self.usage.retryable_expiry_refusals,
                        &mut self.usage.saturated,
                    );
                }
                Err(_) => {
                    self.remove(client, request, deadline);
                    report.refused += 1;
                    inc_runtime(&mut self.usage.expiry_refusals, &mut self.usage.saturated);
                    inc_runtime(
                        &mut self.usage.terminal_expiry_refusals,
                        &mut self.usage.saturated,
                    );
                }
            }
        }
        self.usage.current = self.deadlines_by_key.len();
        report.remaining_due = self
            .deadlines_by_due
            .first()
            .is_some_and(|(deadline, _, _)| *deadline <= now);
        self.fair_cursor = if report.remaining_due {
            Some(last_attempted.expect("a due deadline produces at least one bounded attempt"))
        } else {
            None
        };
        core.deadline_drive_once();
        Ok(report)
    }

    fn clear(&mut self, client: SemanticClientRefV1, request: SemanticRequestRefV1) -> bool {
        let Some(deadline) = self.deadlines_by_key.get(&(client, request)).copied() else {
            return false;
        };
        self.remove(client, request, deadline);
        {
            self.usage.current = self.deadlines_by_key.len();
            inc_runtime(
                &mut self.usage.cleared_before_deadline,
                &mut self.usage.saturated,
            );
        }
        true
    }

    fn remove(
        &mut self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
        deadline: LocalSemanticInstantV1,
    ) {
        assert_eq!(
            self.deadlines_by_key.remove(&(client, request)),
            Some(deadline)
        );
        assert!(self.deadlines_by_due.remove(&(deadline, client, request)));
    }
}

pub struct LocalSemanticRuntimeV1<G, R, U> {
    core: SemanticRuntimeCoreV1<LspAdapterV1<LocalLspTransportV1, G, R, U>>,
    scheduler: LocalSemanticDeadlineSchedulerV1,
    service_limits: SemanticServiceLimitsV1,
    runtime_usage: LocalSemanticRuntimeUsageV1,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LocalSemanticRuntimeUsageV1 {
    pub replacement_attempts: u64,
    pub replacements_installed: u64,
    pub replacements_refused: u64,
    pub saturated: bool,
}

type LocalAdapterV1<G, R, U> = LspAdapterV1<LocalLspTransportV1, G, R, U>;

/// Fresh, started provider owned by trusted composition until it becomes ready
/// and is offered to an existing runtime. Owner loss requests an abort but is
/// unconfirmed cleanup; use `abort` and drive to terminal evidence instead.
#[must_use = "dropping a candidate requests an unconfirmed owner-loss abort"]
pub struct LocalSemanticCandidateV1<G, R, U> {
    adapter: LocalAdapterV1<G, R, U>,
    lsp_limits: LspLimitsV1,
    semantic_limits: SemanticLimitsV1,
}

/// Adapter retained after startup failed without confirmed cleanup. Owner loss
/// requests an abort but cannot report its result; explicit drive/escalation is
/// required for terminal evidence.
#[must_use = "startup cleanup is not confirmed until terminal evidence is observed"]
pub struct LocalSemanticStartupCleanupV1<G, R, U> {
    adapter: LocalAdapterV1<G, R, U>,
}

impl<G, R, U> LocalSemanticStartupCleanupV1<G, R, U>
where
    G: SemanticGenerationIssuerV1 + Send,
    R: LspIndexReadinessPortV1 + Send,
    U: LspVirtualDocumentIssuerV1 + Send,
{
    pub fn drive_once(&mut self) -> LspTransportTerminalV1 {
        let _ = self.adapter.poll();
        self.adapter.transport_terminal_state()
    }

    pub fn escalate(&mut self) -> LspAbortOutcomeV1 {
        self.adapter.abort()
    }

    pub fn state(&self) -> LspTransportTerminalV1 {
        self.adapter.transport_terminal_state()
    }
}

impl<G, R, U> LocalSemanticCandidateV1<G, R, U>
where
    G: SemanticGenerationIssuerV1 + Send,
    R: LspIndexReadinessPortV1 + Send,
    U: LspVirtualDocumentIssuerV1 + Send,
{
    pub fn drive_once(&mut self) {
        let _ = self.adapter.poll();
    }

    pub fn state(&self) -> LocalSemanticRuntimeStateV1 {
        let state = self.adapter.state();
        LocalSemanticRuntimeStateV1 {
            readiness: state.readiness,
            capabilities: state.capabilities,
        }
    }

    pub fn abort(&mut self) -> LspAbortOutcomeV1 {
        self.adapter.abort()
    }

    pub fn cleanup_state(&self) -> LspTransportTerminalV1 {
        self.adapter.transport_terminal_state()
    }

    /// Converts an uninstalled candidate into the same exact forced-cleanup
    /// driver used for rejected replacements.
    pub fn into_retirement(self) -> LocalSemanticRetirementV1<G, R, U> {
        SemanticRetirementDriverV1::from_uninstalled_adapter(self.adapter)
    }
}

#[must_use = "the failed replacement candidate remains owned for cleanup or retry"]
pub struct LocalSemanticReplacementFailureV1<G, R, U> {
    candidate: LocalSemanticCandidateV1<G, R, U>,
    refusal: LocalSemanticReplacementRefusalV1,
}

pub type LocalSemanticReplacementResultV1<G, R, U> =
    Result<LocalSemanticRetirementV1<G, R, U>, Box<LocalSemanticReplacementFailureV1<G, R, U>>>;
pub type LocalSemanticCandidateLaunchResultV1<G, R, U> =
    Result<LocalSemanticCandidateV1<G, R, U>, Box<LocalSemanticRuntimeFailureV1<G, R, U>>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalSemanticReplacementRefusalV1 {
    IncompatibleLimits,
    Service(SemanticProviderReplacementRefusalV1),
}

impl<G, R, U> LocalSemanticReplacementFailureV1<G, R, U> {
    pub const fn refusal(&self) -> LocalSemanticReplacementRefusalV1 {
        self.refusal
    }

    pub fn into_candidate(self) -> LocalSemanticCandidateV1<G, R, U> {
        self.candidate
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalSemanticRetirementStateV1 {
    Draining { remaining: usize },
    ProtocolShutdown,
    ForcedShutdownPending,
    GracefulConfirmed,
    ForcedConfirmed,
    CleanupFailed,
}

/// Closed terminal class carried by an affine retirement receipt. A failed
/// cleanup is evidence of failure, never a successful retirement claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalSemanticRetirementTerminalV1 {
    GracefulConfirmed,
    ForcedConfirmed,
    CleanupFailed,
}

/// Consuming evidence that the retirement driver reached one of its three
/// terminal states. The receipt deliberately retains only fixed-cardinality
/// usage and the count of provider-coupled correlation residue.
#[must_use = "terminal provider-retirement evidence must be retained"]
#[derive(Eq, PartialEq)]
pub struct LocalSemanticRetirementReceiptV1 {
    terminal: LocalSemanticRetirementTerminalV1,
    terminal_evidence: LspTransportTerminalV1,
    quarantined_requests: usize,
    forced_abandonment: Option<SemanticBridgeForcedAbandonmentReceiptV1>,
    provider_qualification: LspProviderQualificationEvidenceV1,
    provider_usage: LspUsageV1,
    usage: LocalSemanticRetirementUsageV1,
}

impl LocalSemanticRetirementReceiptV1 {
    pub const fn terminal(&self) -> LocalSemanticRetirementTerminalV1 {
        self.terminal
    }

    pub const fn quarantined_requests(&self) -> usize {
        self.quarantined_requests
    }

    pub const fn terminal_evidence(&self) -> LspTransportTerminalV1 {
        self.terminal_evidence
    }

    pub const fn forced_abandonment(&self) -> Option<SemanticBridgeForcedAbandonmentReceiptV1> {
        self.forced_abandonment
    }

    pub const fn provider_qualification(&self) -> LspProviderQualificationEvidenceV1 {
        self.provider_qualification
    }

    pub const fn provider_usage(&self) -> LspUsageV1 {
        self.provider_usage
    }

    pub const fn usage(&self) -> LocalSemanticRetirementUsageV1 {
        self.usage
    }
}

impl std::fmt::Debug for LocalSemanticRetirementReceiptV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalSemanticRetirementReceiptV1")
            .field("terminal", &self.terminal)
            .field("terminal_evidence", &self.terminal_evidence)
            .field("quarantined_requests", &self.quarantined_requests)
            .field("forced_abandonment", &self.forced_abandonment)
            .field("provider_qualification", &self.provider_qualification)
            .field("provider_usage", &self.provider_usage)
            .field("usage", &self.usage)
            .finish()
    }
}

/// Old provider retirement after the new provider is already authoritative.
/// Every call is bounded; composition supplies repetition and escalation time.
/// Owner loss requests an abort but cannot confirm its cleanup outcome.
#[doc(hidden)]
#[must_use = "retirement must reach terminal evidence or be explicitly escalated"]
pub struct SemanticRetirementDriverV1<T: LspTransportPortV1, G, R, U> {
    retirement: Option<SemanticProviderRetirementV1<LspAdapterV1<T, G, R, U>>>,
    adapter: Option<LspAdapterV1<T, G, R, U>>,
    forced_shutdown: bool,
    outcome: Option<LocalSemanticRetirementStateV1>,
    terminal_evidence: Option<LspTransportTerminalV1>,
    forced_abandonment: Option<SemanticBridgeForcedAbandonmentReceiptV1>,
    usage: LocalSemanticRetirementUsageV1,
    quarantined_requests: usize,
}

pub type LocalSemanticRetirementV1<G, R, U> =
    SemanticRetirementDriverV1<LocalLspTransportV1, G, R, U>;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LocalSemanticRetirementUsageV1 {
    pub drives: u64,
    pub escalations: u64,
    pub protocol_shutdown_refusals: u64,
    pub graceful_confirmed: u64,
    pub forced_confirmed: u64,
    pub cleanup_failed: u64,
    pub forced_only_starts: u64,
    pub saturated: bool,
}

impl<T, G, R, U> SemanticRetirementDriverV1<T, G, R, U>
where
    T: LspTransportPortV1,
    G: SemanticGenerationIssuerV1,
    R: LspIndexReadinessPortV1,
    U: LspVirtualDocumentIssuerV1,
{
    /// Starts forced-only terminal cleanup for an adapter that never entered a
    /// bridge service. This is the lossless exit for fresh-start refusal; it
    /// cannot produce a graceful retirement claim because no checked bridge
    /// extraction occurred.
    pub fn from_uninstalled_adapter(mut adapter: LspAdapterV1<T, G, R, U>) -> Self {
        let _ = adapter.abort();
        let _ = adapter.poll();
        Self {
            retirement: None,
            adapter: Some(adapter),
            forced_shutdown: true,
            outcome: None,
            terminal_evidence: None,
            forced_abandonment: None,
            usage: LocalSemanticRetirementUsageV1 {
                forced_only_starts: 1,
                ..LocalSemanticRetirementUsageV1::default()
            },
            quarantined_requests: 0,
        }
    }

    /// Starts forced-only cleanup for a fresh adapter rejected before bridge
    /// installation. Consuming the fresh wrapper prevents later reuse.
    pub fn from_uninstalled_fresh_adapter(adapter: LspFreshStartedAdapterV1<T, G, R, U>) -> Self {
        Self::from_uninstalled_adapter(adapter.into_adapter_for_retirement())
    }

    /// Owns the displaced provider and exact remaining snapshot-close plan
    /// after an ordered replacement. Converting the fresh-start wrapper cannot
    /// restore installation authority; the product remains retirement-only.
    pub fn from_replacement_retirement(
        retirement: SemanticProviderRetirementV1<LspFreshStartedAdapterV1<T, G, R, U>>,
    ) -> Self {
        Self {
            retirement: Some(
                retirement.map_provider(LspFreshStartedAdapterV1::into_adapter_for_retirement),
            ),
            adapter: None,
            forced_shutdown: false,
            outcome: None,
            terminal_evidence: None,
            forced_abandonment: None,
            usage: LocalSemanticRetirementUsageV1::default(),
            quarantined_requests: 0,
        }
    }

    /// Starts terminal retirement from a source-free ordered bridge provider.
    /// The fresh-started wrapper is consumed and can never return to service;
    /// its exact provider-lifetime quarantine count is retained in the final
    /// receipt.
    pub fn from_bridge_provider(
        provider: SemanticBridgeRetiringProviderV1<LspFreshStartedAdapterV1<T, G, R, U>>,
    ) -> Self {
        let quarantined_requests = provider.quarantined_requests();
        let adapter = provider
            .into_provider_for_retirement()
            .into_adapter_for_retirement();
        Self {
            retirement: None,
            adapter: Some(adapter),
            forced_shutdown: false,
            outcome: None,
            terminal_evidence: None,
            forced_abandonment: None,
            usage: LocalSemanticRetirementUsageV1::default(),
            quarantined_requests,
        }
    }

    /// Starts forced-only retirement while retaining the exact bridge
    /// abandonment evidence. No protocol-shutdown or graceful path is exposed.
    pub fn from_forced_bridge_provider(
        provider: SemanticBridgeForcedRetiringProviderV1<LspFreshStartedAdapterV1<T, G, R, U>>,
    ) -> Self {
        let (provider, forced_abandonment) = provider
            .into_forced_retirement_parts()
            .into_provider_and_abandonment();
        let quarantined_requests = forced_abandonment.quarantined_requests;
        let mut adapter = provider.into_adapter_for_retirement();
        let _ = adapter.abort();
        let _ = adapter.poll();
        let usage = LocalSemanticRetirementUsageV1 {
            forced_only_starts: 1,
            ..LocalSemanticRetirementUsageV1::default()
        };
        Self {
            retirement: None,
            adapter: Some(adapter),
            forced_shutdown: true,
            outcome: None,
            terminal_evidence: None,
            forced_abandonment: Some(forced_abandonment),
            usage,
            quarantined_requests,
        }
    }
}

impl<T, G, R, U> SemanticRetirementDriverV1<T, G, R, U>
where
    T: LspTransportPortV1 + Send,
    G: SemanticGenerationIssuerV1 + Send,
    R: LspIndexReadinessPortV1 + Send,
    U: LspVirtualDocumentIssuerV1 + Send,
{
    fn observe_terminal(&mut self) -> bool {
        let adapter = self.adapter.as_ref().expect("adapter follows drain phase");
        let terminal_evidence = adapter.transport_terminal_state();
        let outcome = match terminal_evidence {
            LspTransportTerminalV1::GracefulClosed => {
                if self.forced_abandonment.is_none()
                    && !self.forced_shutdown
                    && adapter.session_state() == LspSessionStateV1::Closed
                {
                    LocalSemanticRetirementStateV1::GracefulConfirmed
                } else {
                    LocalSemanticRetirementStateV1::CleanupFailed
                }
            }
            LspTransportTerminalV1::ForcedClosed
            | LspTransportTerminalV1::FailedCleanupConfirmed => {
                LocalSemanticRetirementStateV1::ForcedConfirmed
            }
            LspTransportTerminalV1::Failed => LocalSemanticRetirementStateV1::CleanupFailed,
            LspTransportTerminalV1::Active | LspTransportTerminalV1::Unobserved => return false,
        };
        self.terminal_evidence = Some(terminal_evidence);
        self.outcome = Some(outcome);
        match outcome {
            LocalSemanticRetirementStateV1::GracefulConfirmed => inc_runtime(
                &mut self.usage.graceful_confirmed,
                &mut self.usage.saturated,
            ),
            LocalSemanticRetirementStateV1::ForcedConfirmed => {
                inc_runtime(&mut self.usage.forced_confirmed, &mut self.usage.saturated)
            }
            LocalSemanticRetirementStateV1::CleanupFailed => {
                inc_runtime(&mut self.usage.cleanup_failed, &mut self.usage.saturated)
            }
            LocalSemanticRetirementStateV1::Draining { .. }
            | LocalSemanticRetirementStateV1::ProtocolShutdown
            | LocalSemanticRetirementStateV1::ForcedShutdownPending => {
                unreachable!("terminal observation has only closed outcomes")
            }
        }
        true
    }

    pub fn state(&self) -> LocalSemanticRetirementStateV1 {
        if let Some(outcome) = self.outcome {
            return outcome;
        }
        if self.forced_shutdown {
            return LocalSemanticRetirementStateV1::ForcedShutdownPending;
        }
        if let Some(retirement) = &self.retirement {
            return match retirement.state() {
                SemanticProviderRetirementStateV1::Drained => {
                    LocalSemanticRetirementStateV1::ProtocolShutdown
                }
                SemanticProviderRetirementStateV1::Pending { remaining } => {
                    LocalSemanticRetirementStateV1::Draining { remaining }
                }
            };
        }
        LocalSemanticRetirementStateV1::ProtocolShutdown
    }

    pub fn drive_once(&mut self) -> LocalSemanticRetirementStateV1 {
        if self.outcome.is_some() {
            return self.state();
        }
        inc_runtime(&mut self.usage.drives, &mut self.usage.saturated);
        if let Some(retirement) = self.retirement.as_mut() {
            if matches!(
                retirement.retry_drain(),
                SemanticProviderRetirementStateV1::Pending { .. }
            ) {
                return self.state();
            }
            let retirement = self.retirement.take().expect("retirement phase present");
            self.adapter = Some(
                retirement
                    .into_provider_if_drained()
                    .expect("drained state was just observed"),
            );
        }
        if self.observe_terminal() {
            return self.state();
        }
        let adapter = self.adapter.as_mut().expect("adapter follows drain phase");
        match adapter.session_state() {
            LspSessionStateV1::Closed | LspSessionStateV1::ShuttingDown { .. } => {}
            LspSessionStateV1::Aborted | LspSessionStateV1::Failed => {}
            _ => match adapter.begin_shutdown() {
                LspShutdownOutcomeV1::Started
                | LspShutdownOutcomeV1::Busy
                | LspShutdownOutcomeV1::AlreadyTerminal => {}
                LspShutdownOutcomeV1::Unavailable | LspShutdownOutcomeV1::Refused => {
                    inc_runtime(
                        &mut self.usage.protocol_shutdown_refusals,
                        &mut self.usage.saturated,
                    );
                }
            },
        }
        let _ = adapter.poll();
        self.state()
    }

    /// Abandons any still-refused close plan and explicitly escalates the old
    /// provider. This enters `ForcedShutdownPending`; only later transport
    /// terminal evidence may confirm forced cleanup or report cleanup failure.
    pub fn escalate(&mut self) -> LocalSemanticRetirementStateV1 {
        if self.outcome.is_some() {
            return self.state();
        }
        if let Some(retirement) = self.retirement.take() {
            self.adapter = Some(retirement.into_provider());
        }
        if self.observe_terminal() {
            return self.state();
        }
        if self.forced_abandonment.is_some() {
            if let Some(adapter) = self.adapter.as_mut() {
                let _ = adapter.poll();
            }
            let _ = self.observe_terminal();
            return self.state();
        }
        inc_runtime(&mut self.usage.escalations, &mut self.usage.saturated);
        if let Some(adapter) = self.adapter.as_mut() {
            let abort = adapter.abort();
            self.forced_shutdown = abort == LspAbortOutcomeV1::Aborted
                || adapter.session_state() == LspSessionStateV1::Aborted;
            let _ = adapter.poll();
        }
        self.state()
    }

    pub const fn usage(&self) -> LocalSemanticRetirementUsageV1 {
        self.usage
    }

    /// Consumes only terminal driver states into affine cleanup evidence. A
    /// nonterminal driver is returned unchanged so cleanup authority cannot be
    /// discarded by observing `state()` alone.
    pub fn try_into_terminal_receipt(self) -> Result<LocalSemanticRetirementReceiptV1, Box<Self>> {
        let terminal = match self.outcome {
            Some(LocalSemanticRetirementStateV1::GracefulConfirmed) => {
                LocalSemanticRetirementTerminalV1::GracefulConfirmed
            }
            Some(LocalSemanticRetirementStateV1::ForcedConfirmed) => {
                LocalSemanticRetirementTerminalV1::ForcedConfirmed
            }
            Some(LocalSemanticRetirementStateV1::CleanupFailed) => {
                LocalSemanticRetirementTerminalV1::CleanupFailed
            }
            _ => return Err(Box::new(self)),
        };
        let adapter = self
            .adapter
            .as_ref()
            .expect("terminal outcome retains its exact adapter");
        let provider_qualification = adapter.qualification_evidence();
        let provider_usage = adapter.usage();
        Ok(LocalSemanticRetirementReceiptV1 {
            terminal,
            terminal_evidence: self
                .terminal_evidence
                .expect("terminal outcome retains exact transport evidence"),
            quarantined_requests: self.quarantined_requests,
            forced_abandonment: self.forced_abandonment,
            provider_qualification,
            provider_usage,
            usage: self.usage,
        })
    }
}

struct SemanticRuntimeCoreV1<P> {
    service: SemanticServiceV1<P>,
}

impl<P: SemanticProviderPortV1 + Send> SemanticRuntimeCoreV1<P> {
    fn admit_client(
        &mut self,
        client: SemanticClientRefV1,
    ) -> Result<(), SemanticServiceRefusalV1> {
        self.service.admit_client(client)
    }

    fn bind_client(
        &mut self,
        client: SemanticClientRefV1,
    ) -> SemanticClientV1<impl SemanticServicePortV1 + '_> {
        bind_semantic_client_v1(&mut self.service, client)
    }

    fn expire(
        &mut self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> Result<bool, SemanticServiceRefusalV1> {
        self.service.expire(client, request)
    }

    fn is_pending_for(
        &self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> Result<bool, SemanticServiceRefusalV1> {
        self.service.is_pending_for(client, request)
    }

    fn drive_once(&mut self) {
        self.service.pump();
    }

    fn state(&self) -> LocalSemanticRuntimeStateV1 {
        let state = self.service.provider_state();
        LocalSemanticRuntimeStateV1 {
            readiness: state.readiness,
            capabilities: state.capabilities,
        }
    }

    const fn usage(&self) -> SemanticServiceUsageV1 {
        self.service.usage()
    }

    fn try_replace_provider(
        &mut self,
        candidate: P,
    ) -> Result<
        SemanticProviderRetirementV1<P>,
        vellum_semantic_service::SemanticProviderReplacementFailureV1<P>,
    > {
        self.service.try_replace_provider(candidate)
    }
}

impl<P: SemanticProviderPortV1 + Send> SemanticDeadlineCoreV1 for SemanticRuntimeCoreV1<P> {
    fn deadline_is_pending_for(
        &self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> Result<bool, SemanticServiceRefusalV1> {
        self.is_pending_for(client, request)
    }

    fn deadline_expire(
        &mut self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> Result<bool, SemanticServiceRefusalV1> {
        self.expire(client, request)
    }

    fn deadline_drive_once(&mut self) {
        self.drive_once();
    }
}

struct SemanticRuntimeStartFailureV1<A> {
    refusal: LocalSemanticRuntimeRefusalV1,
    cleanup: Option<A>,
}

type SemanticRuntimeStartResultV1<T, G, R, U> = Result<
    SemanticRuntimeCoreV1<LspAdapterV1<T, G, R, U>>,
    Box<SemanticRuntimeStartFailureV1<LspAdapterV1<T, G, R, U>>>,
>;

fn try_start_adapter_v1<T, G, R, U>(
    mut adapter: LspAdapterV1<T, G, R, U>,
    service_limits: SemanticServiceLimitsV1,
) -> SemanticRuntimeStartResultV1<T, G, R, U>
where
    T: LspTransportPortV1 + Send,
    G: SemanticGenerationIssuerV1 + Send,
    R: LspIndexReadinessPortV1 + Send,
    U: LspVirtualDocumentIssuerV1 + Send,
{
    match adapter.start() {
        LspStartOutcomeV1::Started => {}
        outcome => return Err(Box::new(refuse_started_adapter(adapter, outcome))),
    }
    let service = SemanticServiceV1::try_new(adapter, service_limits).map_err(|_| {
        Box::new(SemanticRuntimeStartFailureV1 {
            refusal: LocalSemanticRuntimeRefusalV1::InvalidService,
            cleanup: None,
        })
    })?;
    Ok(SemanticRuntimeCoreV1 { service })
}

fn refuse_started_adapter<T, G, R, U>(
    mut adapter: LspAdapterV1<T, G, R, U>,
    outcome: LspStartOutcomeV1,
) -> SemanticRuntimeStartFailureV1<LspAdapterV1<T, G, R, U>>
where
    T: LspTransportPortV1,
    G: SemanticGenerationIssuerV1,
    R: LspIndexReadinessPortV1,
    U: LspVirtualDocumentIssuerV1,
{
    let _ = adapter.abort();
    let _ = adapter.poll();
    let confirmed = matches!(
        adapter.transport_terminal_state(),
        LspTransportTerminalV1::GracefulClosed
            | LspTransportTerminalV1::ForcedClosed
            | LspTransportTerminalV1::FailedCleanupConfirmed
    );
    let refusal = if confirmed {
        match outcome {
            LspStartOutcomeV1::Unavailable => LocalSemanticRuntimeRefusalV1::StartUnavailable,
            LspStartOutcomeV1::Refused | LspStartOutcomeV1::AlreadyStarted => {
                LocalSemanticRuntimeRefusalV1::StartRefused
            }
            LspStartOutcomeV1::Started => unreachable!("started adapters are not refused"),
        }
    } else {
        LocalSemanticRuntimeRefusalV1::StartCleanupUnconfirmed(outcome)
    };
    SemanticRuntimeStartFailureV1 {
        refusal,
        cleanup: (!confirmed).then_some(adapter),
    }
}

fn runtime_failure<G, R, U>(
    refusal: LocalSemanticRuntimeRefusalV1,
) -> Box<LocalSemanticRuntimeFailureV1<G, R, U>> {
    Box::new(LocalSemanticRuntimeFailureV1 {
        refusal,
        cleanup: None,
    })
}

fn map_start_failure<G, R, U>(
    failure: SemanticRuntimeStartFailureV1<LocalAdapterV1<G, R, U>>,
) -> Box<LocalSemanticRuntimeFailureV1<G, R, U>> {
    Box::new(LocalSemanticRuntimeFailureV1 {
        refusal: failure.refusal,
        cleanup: failure
            .cleanup
            .map(|adapter| LocalSemanticStartupCleanupV1 { adapter }),
    })
}

impl<G, R, U> LocalSemanticRuntimeV1<G, R, U>
where
    G: SemanticGenerationIssuerV1 + Send,
    R: LspIndexReadinessPortV1 + Send,
    U: LspVirtualDocumentIssuerV1 + Send,
{
    pub fn try_launch(
        profile: SemanticProviderProfileV1,
        generations: G,
        readiness: R,
        documents: U,
        lsp_limits: LspLimitsV1,
        semantic_limits: SemanticLimitsV1,
        service_limits: SemanticServiceLimitsV1,
    ) -> Result<Self, Box<LocalSemanticRuntimeFailureV1<G, R, U>>> {
        Self::try_launch_with_scheduler(
            profile,
            generations,
            readiness,
            documents,
            lsp_limits,
            semantic_limits,
            service_limits,
            LocalSemanticSchedulerLimitsV1::reference(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn try_launch_with_scheduler(
        profile: SemanticProviderProfileV1,
        generations: G,
        readiness: R,
        documents: U,
        lsp_limits: LspLimitsV1,
        semantic_limits: SemanticLimitsV1,
        service_limits: SemanticServiceLimitsV1,
        scheduler_limits: LocalSemanticSchedulerLimitsV1,
    ) -> Result<Self, Box<LocalSemanticRuntimeFailureV1<G, R, U>>> {
        // Validate every pure bound before process launch. Once the child exists,
        // constructor refusal is reserved for transport/startup facts rather than
        // caller-supplied malformed limits.
        if LspFrameDecoderV1::try_new(lsp_limits).is_err() || !semantic_limits.is_valid() {
            return Err(runtime_failure(
                LocalSemanticRuntimeRefusalV1::InvalidAdapter,
            ));
        }
        if !service_limits.is_valid()
            || service_limits.peer != semantic_limits
            || service_limits.max_documents > lsp_limits.max_documents
            || service_limits.max_total_snapshot_bytes > lsp_limits.max_total_source_bytes
        {
            return Err(runtime_failure(
                LocalSemanticRuntimeRefusalV1::InvalidService,
            ));
        }
        let scheduler = LocalSemanticDeadlineSchedulerV1::try_new(scheduler_limits)
            .ok_or_else(|| runtime_failure(LocalSemanticRuntimeRefusalV1::InvalidScheduler))?;
        let provenance = profile.provenance.clone();
        let initialization = workspace_initialization_v1(&profile.workspace_root, lsp_limits)
            .ok_or_else(|| runtime_failure(LocalSemanticRuntimeRefusalV1::InvalidAdapter))?;
        let transport = launch_local_lsp_v1(profile)
            .map_err(|refusal| runtime_failure(LocalSemanticRuntimeRefusalV1::Launch(refusal)))?;
        let adapter = LspAdapterV1::try_new_with_profile(
            transport,
            generations,
            readiness,
            documents,
            LspAdapterProfileV1::try_new(provenance, lsp_limits, semantic_limits, initialization)
                .ok_or_else(|| runtime_failure(LocalSemanticRuntimeRefusalV1::InvalidAdapter))?,
        )
        .ok_or_else(|| runtime_failure(LocalSemanticRuntimeRefusalV1::InvalidAdapter))?;
        let core = try_start_adapter_v1(adapter, service_limits)
            .map_err(|failure| map_start_failure(*failure))?;
        Ok(Self {
            core,
            scheduler,
            service_limits,
            runtime_usage: LocalSemanticRuntimeUsageV1::default(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn try_launch_replacement(
        profile: SemanticProviderProfileV1,
        generations: G,
        readiness: R,
        documents: U,
        lsp_limits: LspLimitsV1,
        semantic_limits: SemanticLimitsV1,
    ) -> LocalSemanticCandidateLaunchResultV1<G, R, U> {
        if LspFrameDecoderV1::try_new(lsp_limits).is_err() || !semantic_limits.is_valid() {
            return Err(runtime_failure(
                LocalSemanticRuntimeRefusalV1::InvalidAdapter,
            ));
        }
        let provenance = profile.provenance.clone();
        let initialization = workspace_initialization_v1(&profile.workspace_root, lsp_limits)
            .ok_or_else(|| runtime_failure(LocalSemanticRuntimeRefusalV1::InvalidAdapter))?;
        let transport = launch_local_lsp_v1(profile)
            .map_err(|refusal| runtime_failure(LocalSemanticRuntimeRefusalV1::Launch(refusal)))?;
        let mut adapter = LspAdapterV1::try_new_with_profile(
            transport,
            generations,
            readiness,
            documents,
            LspAdapterProfileV1::try_new(provenance, lsp_limits, semantic_limits, initialization)
                .ok_or_else(|| runtime_failure(LocalSemanticRuntimeRefusalV1::InvalidAdapter))?,
        )
        .ok_or_else(|| runtime_failure(LocalSemanticRuntimeRefusalV1::InvalidAdapter))?;
        match adapter.start() {
            LspStartOutcomeV1::Started => {}
            outcome => {
                return Err(map_start_failure(refuse_started_adapter(adapter, outcome)));
            }
        }
        Ok(LocalSemanticCandidateV1 {
            adapter,
            lsp_limits,
            semantic_limits,
        })
    }

    pub fn try_install_replacement(
        &mut self,
        candidate: LocalSemanticCandidateV1<G, R, U>,
    ) -> LocalSemanticReplacementResultV1<G, R, U> {
        inc_runtime(
            &mut self.runtime_usage.replacement_attempts,
            &mut self.runtime_usage.saturated,
        );
        if !replacement_limits_compatible(
            self.service_limits,
            candidate.lsp_limits,
            candidate.semantic_limits,
        ) {
            inc_runtime(
                &mut self.runtime_usage.replacements_refused,
                &mut self.runtime_usage.saturated,
            );
            return Err(Box::new(LocalSemanticReplacementFailureV1 {
                candidate,
                refusal: LocalSemanticReplacementRefusalV1::IncompatibleLimits,
            }));
        }
        let LocalSemanticCandidateV1 {
            adapter,
            lsp_limits,
            semantic_limits,
        } = candidate;
        match self.core.try_replace_provider(adapter) {
            Ok(retirement) => {
                inc_runtime(
                    &mut self.runtime_usage.replacements_installed,
                    &mut self.runtime_usage.saturated,
                );
                Ok(SemanticRetirementDriverV1 {
                    retirement: Some(retirement),
                    adapter: None,
                    forced_shutdown: false,
                    outcome: None,
                    terminal_evidence: None,
                    forced_abandonment: None,
                    usage: LocalSemanticRetirementUsageV1::default(),
                    quarantined_requests: 0,
                })
            }
            Err(failure) => {
                inc_runtime(
                    &mut self.runtime_usage.replacements_refused,
                    &mut self.runtime_usage.saturated,
                );
                Err(Box::new(map_replacement_failure(
                    lsp_limits,
                    semantic_limits,
                    failure,
                )))
            }
        }
    }

    pub fn admit_client(
        &mut self,
        client: SemanticClientRefV1,
    ) -> Result<(), SemanticServiceRefusalV1> {
        self.core.admit_client(client)
    }

    pub fn bind_client(
        &mut self,
        client: SemanticClientRefV1,
    ) -> SemanticClientV1<impl SemanticServicePortV1 + '_> {
        self.core.bind_client(client)
    }

    pub fn expire(
        &mut self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> Result<bool, SemanticServiceRefusalV1> {
        self.core.expire(client, request)
    }

    pub fn drive_once(&mut self) {
        self.core.drive_once();
    }

    /// Installs an exact composition-owned deadline tick. Ticks are opaque
    /// monotonic units; this runtime never reads a clock or retries a request.
    pub fn schedule_deadline(
        &mut self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
        deadline: LocalSemanticInstantV1,
    ) -> LocalSemanticDeadlineAdmissionV1 {
        self.scheduler
            .schedule(&self.core, client, request, deadline)
    }

    /// Applies at most the configured expiration budget, then performs one
    /// bounded semantic-service pump.
    pub fn drive_at(
        &mut self,
        now: LocalSemanticInstantV1,
    ) -> Result<LocalSemanticDriveV1, LocalSemanticDriveRefusalV1> {
        self.scheduler.drive(&mut self.core, now)
    }

    /// Releases a retained deadline after composition has terminal evidence
    /// such as cancellation or delivery. This does not cancel provider work.
    pub fn clear_deadline(
        &mut self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> bool {
        self.scheduler.clear(client, request)
    }

    pub fn state(&self) -> LocalSemanticRuntimeStateV1 {
        self.core.state()
    }

    pub const fn usage(&self) -> SemanticServiceUsageV1 {
        self.core.usage()
    }

    pub const fn scheduler_usage(&self) -> LocalSemanticSchedulerUsageV1 {
        self.scheduler.usage
    }

    pub const fn runtime_usage(&self) -> LocalSemanticRuntimeUsageV1 {
        self.runtime_usage
    }
}

fn inc_runtime(value: &mut u64, saturated: &mut bool) {
    let next = value.saturating_add(1);
    *saturated |= next == u64::MAX;
    *value = next;
}

fn replacement_limits_compatible(
    service: SemanticServiceLimitsV1,
    lsp: LspLimitsV1,
    semantic: SemanticLimitsV1,
) -> bool {
    semantic == service.peer
        && lsp.max_documents >= service.max_documents
        && lsp.max_total_source_bytes >= service.max_total_snapshot_bytes
}

fn map_replacement_failure<G, R, U>(
    lsp_limits: LspLimitsV1,
    semantic_limits: SemanticLimitsV1,
    failure: SemanticProviderReplacementFailureV1<LocalAdapterV1<G, R, U>>,
) -> LocalSemanticReplacementFailureV1<G, R, U> {
    let refusal = failure.refusal();
    LocalSemanticReplacementFailureV1 {
        candidate: LocalSemanticCandidateV1 {
            adapter: failure.into_candidate(),
            lsp_limits,
            semantic_limits,
        },
        refusal: LocalSemanticReplacementRefusalV1::Service(refusal),
    }
}

#[cfg(test)]
mod tests;
