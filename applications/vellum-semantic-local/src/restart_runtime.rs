//! Opt-in hot-restart composition for the embedded semantic runtime.
//!
//! The old runtime remains the sole snapshot authority until a fresh provider
//! is ready and the semantic service has replayed every exact snapshot. The
//! runtime is not exposed while restart is in flight. Outstanding queries are
//! invalidated by contract and are never submitted to the replacement.

use std::{num::NonZeroU64, time::Duration};

use vellum_adapter_semantic_lsp::{
    LspIndexReadinessPortV1, LspLimitsV1, LspVirtualDocumentIssuerV1, SemanticGenerationIssuerV1,
};
use vellum_host::semantic::SemanticLimitsV1;
use vellum_semantic_service::SemanticProviderReadinessV1;

use crate::{
    LocalSemanticCandidateV1, LocalSemanticRetirementReceiptV1, LocalSemanticRetirementV1,
    LocalSemanticRuntimeFailureV1, LocalSemanticRuntimeV1, SemanticProcessLaunchRefusalV1,
    SemanticProcessOwnedLaunchFailureV1, SemanticProviderProfileV1,
    SemanticProviderRestartBackoffV1, SemanticProviderRestartCauseV1,
    SemanticProviderRestartControllerV1, SemanticProviderRestartDecisionV1,
    SemanticProviderRestartFuseV1, SemanticProviderRestartPolicyV1,
    SemanticProviderRestartQueryDispositionV1, SemanticProviderRestartRefusalV1,
    SemanticProviderRestartStatusV1,
};

pub const MAX_RESTART_DRIVES_V1: usize = 65_536;

/// Composition-owned bounded wait. `false` means elapsed time is uncertain and
/// permanently fuses the restart; it is never treated as an elapsed delay.
pub trait LocalSemanticRestartClockV1 {
    fn wait(&mut self, delay: Duration) -> bool;
}

pub struct LocalSemanticThreadRestartClockV1;

impl LocalSemanticRestartClockV1 for LocalSemanticThreadRestartClockV1 {
    fn wait(&mut self, delay: Duration) -> bool {
        std::thread::sleep(delay);
        true
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalSemanticRestartLimitsV1 {
    pub candidate_drives: usize,
    pub graceful_cleanup_drives: usize,
    pub forced_cleanup_drives: usize,
    pub drive_delay: Duration,
}

impl LocalSemanticRestartLimitsV1 {
    pub const fn reference() -> Self {
        Self {
            candidate_drives: 2_048,
            graceful_cleanup_drives: 2_048,
            forced_cleanup_drives: 2_048,
            drive_delay: Duration::from_millis(2),
        }
    }

    fn is_valid(self) -> bool {
        self.candidate_drives > 0
            && self.candidate_drives <= MAX_RESTART_DRIVES_V1
            && self.graceful_cleanup_drives > 0
            && self.graceful_cleanup_drives <= MAX_RESTART_DRIVES_V1
            && self.forced_cleanup_drives > 0
            && self.forced_cleanup_drives <= MAX_RESTART_DRIVES_V1
            && !self.drive_delay.is_zero()
    }
}

#[must_use = "restart failure can retain exact provider cleanup ownership"]
pub enum LocalSemanticRestartFailureV1<G, R, U> {
    InvalidLimits,
    Refused(SemanticProviderRestartRefusalV1),
    Fused(SemanticProviderRestartFuseV1),
    CleanupFailed {
        reason: SemanticProviderRestartFuseV1,
        receipt: LocalSemanticRetirementReceiptV1,
    },
    Launch {
        failure: Box<LocalSemanticRuntimeFailureV1<G, R, U>>,
    },
    CandidateCleanup {
        cleanup: LocalSemanticRetirementV1<G, R, U>,
    },
    RetirementCleanup {
        cleanup: LocalSemanticRetirementV1<G, R, U>,
    },
}

impl<G, R, U> std::fmt::Debug for LocalSemanticRestartFailureV1<G, R, U> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLimits => formatter.write_str("InvalidLimits"),
            Self::Refused(refusal) => formatter.debug_tuple("Refused").field(refusal).finish(),
            Self::Fused(reason) => formatter.debug_tuple("Fused").field(reason).finish(),
            Self::CleanupFailed { reason, .. } => formatter
                .debug_struct("CleanupFailed")
                .field("reason", reason)
                .field("receipt", &"<owned>")
                .finish(),
            Self::Launch { failure } => formatter
                .debug_struct("Launch")
                .field("refusal", &failure.refusal())
                .field("cleanup", &"<owned>")
                .finish(),
            Self::CandidateCleanup { .. } => formatter
                .debug_struct("CandidateCleanup")
                .field("cleanup", &"<owned>")
                .finish(),
            Self::RetirementCleanup { .. } => formatter
                .debug_struct("RetirementCleanup")
                .field("cleanup", &"<owned>")
                .finish(),
        }
    }
}

/// Actual embedded composition root for opt-in provider replacement. It owns
/// both the semantic runtime and the committed single-flight restart policy.
pub struct LocalSemanticRestartingRuntimeV1<G, R, U> {
    runtime: LocalSemanticRuntimeV1<G, R, U>,
    controller: SemanticProviderRestartControllerV1,
}

impl<G, R, U> LocalSemanticRestartingRuntimeV1<G, R, U>
where
    G: SemanticGenerationIssuerV1 + Clone + Send,
    R: LspIndexReadinessPortV1 + Clone + Send,
    U: LspVirtualDocumentIssuerV1 + Clone + Send,
{
    pub fn new(
        runtime: LocalSemanticRuntimeV1<G, R, U>,
        policy: SemanticProviderRestartPolicyV1,
        generation: NonZeroU64,
    ) -> Self {
        Self {
            runtime,
            controller: SemanticProviderRestartControllerV1::new_running(policy, generation),
        }
    }

    pub fn status(&self) -> SemanticProviderRestartStatusV1 {
        self.controller.status()
    }

    /// The runtime is unavailable while replay, swap, or displaced-provider
    /// cleanup is in flight. This prevents observations from claiming readiness
    /// before exact cleanup reconciliation completes.
    pub fn runtime(&mut self) -> Option<&mut LocalSemanticRuntimeV1<G, R, U>> {
        matches!(
            self.controller.status(),
            SemanticProviderRestartStatusV1::Running { .. }
        )
        .then_some(&mut self.runtime)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restart<C: LocalSemanticRestartClockV1>(
        &mut self,
        cause: SemanticProviderRestartCauseV1,
        profile: &SemanticProviderProfileV1,
        generations: &G,
        readiness: &R,
        documents: &U,
        lsp_limits: LspLimitsV1,
        semantic_limits: SemanticLimitsV1,
        limits: LocalSemanticRestartLimitsV1,
        clock: &mut C,
    ) -> Result<LocalSemanticRetirementReceiptV1, Box<LocalSemanticRestartFailureV1<G, R, U>>> {
        if !limits.is_valid() {
            return Err(Box::new(LocalSemanticRestartFailureV1::InvalidLimits));
        }
        let mut decision = self
            .controller
            .request_replacement(cause)
            .map_err(|refusal| Box::new(LocalSemanticRestartFailureV1::Refused(refusal)))?;
        loop {
            let SemanticProviderRestartDecisionV1::Backoff(backoff) = decision else {
                let SemanticProviderRestartDecisionV1::Fused(reason) = decision else {
                    unreachable!("replacement begins with backoff")
                };
                return Err(Box::new(LocalSemanticRestartFailureV1::Fused(reason)));
            };
            let launch = self.elapse_backoff(backoff, clock)?;
            let candidate = match LocalSemanticRuntimeV1::try_launch_replacement(
                profile.clone(),
                generations.clone(),
                readiness.clone(),
                documents.clone(),
                lsp_limits,
                semantic_limits,
            ) {
                Ok(candidate) => candidate,
                Err(failure) => {
                    if matches!(
                        failure.refusal(),
                        crate::LocalSemanticRuntimeRefusalV1::StartCleanupUnconfirmed(_)
                    ) {
                        let _ = self.controller.fuse_launch_uncertain(launch);
                        return Err(Box::new(LocalSemanticRestartFailureV1::Launch { failure }));
                    }
                    let reason = match failure.refusal() {
                        crate::LocalSemanticRuntimeRefusalV1::Launch(reason) => reason,
                        _ => SemanticProcessLaunchRefusalV1::RuntimeUnavailable,
                    };
                    if matches!(
                        reason,
                        SemanticProcessLaunchRefusalV1::LaunchTimedOut
                            | SemanticProcessLaunchRefusalV1::CleanupUnconfirmed
                    ) {
                        let _ = self.controller.fuse_launch_uncertain(launch);
                        return Err(Box::new(LocalSemanticRestartFailureV1::Launch { failure }));
                    }
                    let (next, retained) = self
                        .controller
                        .complete_owned_launch_failure(
                            launch,
                            SemanticProcessOwnedLaunchFailureV1::Confirmed(reason),
                        )
                        .expect("fresh launch admission is exact");
                    drop(retained);
                    decision = next;
                    continue;
                }
            };
            return self.install_ready_candidate(launch, candidate, limits, clock);
        }
    }

    fn elapse_backoff<C: LocalSemanticRestartClockV1>(
        &mut self,
        backoff: SemanticProviderRestartBackoffV1,
        clock: &mut C,
    ) -> Result<crate::SemanticProviderRestartLaunchV1, Box<LocalSemanticRestartFailureV1<G, R, U>>>
    {
        if !clock.wait(backoff.delay()) {
            let decision = self
                .controller
                .fuse_backoff_uncertain(backoff)
                .expect("fresh backoff owner is exact");
            let SemanticProviderRestartDecisionV1::Fused(reason) = decision else {
                unreachable!("uncertain clock permanently fuses")
            };
            return Err(Box::new(LocalSemanticRestartFailureV1::Fused(reason)));
        }
        Ok(self
            .controller
            .backoff_elapsed(backoff)
            .expect("fresh backoff owner is exact"))
    }

    fn install_ready_candidate<C: LocalSemanticRestartClockV1>(
        &mut self,
        launch: crate::SemanticProviderRestartLaunchV1,
        mut candidate: LocalSemanticCandidateV1<G, R, U>,
        limits: LocalSemanticRestartLimitsV1,
        clock: &mut C,
    ) -> Result<LocalSemanticRetirementReceiptV1, Box<LocalSemanticRestartFailureV1<G, R, U>>> {
        for _ in 0..limits.candidate_drives {
            candidate.drive_once();
            if matches!(
                candidate.state().readiness,
                SemanticProviderReadinessV1::Ready { .. }
            ) {
                let retirement = match self.runtime.try_install_replacement(candidate) {
                    Ok(retirement) => retirement,
                    Err(failure) => {
                        let cleanup = failure.into_candidate().into_retirement();
                        let _ = self.controller.fuse_launch_uncertain(launch);
                        return Err(Box::new(LocalSemanticRestartFailureV1::CandidateCleanup {
                            cleanup,
                        }));
                    }
                };
                let cleanup_owner = self
                    .controller
                    .confirm_replacement_installed(launch)
                    .expect("fresh replacement admission is exact");
                return self.finish_retirement(cleanup_owner, retirement, limits, clock);
            }
            if !clock.wait(limits.drive_delay) {
                break;
            }
        }
        let cleanup = candidate.into_retirement();
        let _ = self.controller.fuse_launch_uncertain(launch);
        Err(Box::new(LocalSemanticRestartFailureV1::CandidateCleanup {
            cleanup,
        }))
    }

    fn finish_retirement<C: LocalSemanticRestartClockV1>(
        &mut self,
        cleanup_owner: crate::SemanticProviderRestartCleanupV1,
        mut retirement: LocalSemanticRetirementV1<G, R, U>,
        limits: LocalSemanticRestartLimitsV1,
        clock: &mut C,
    ) -> Result<LocalSemanticRetirementReceiptV1, Box<LocalSemanticRestartFailureV1<G, R, U>>> {
        for _ in 0..limits.graceful_cleanup_drives {
            retirement.drive_once();
            match retirement.try_into_terminal_receipt() {
                Ok(receipt) => return self.reconcile_receipt(cleanup_owner, receipt),
                Err(returned) => retirement = *returned,
            }
            if !clock.wait(limits.drive_delay) {
                break;
            }
        }
        retirement.escalate();
        for _ in 0..limits.forced_cleanup_drives {
            retirement.drive_once();
            match retirement.try_into_terminal_receipt() {
                Ok(receipt) => return self.reconcile_receipt(cleanup_owner, receipt),
                Err(returned) => retirement = *returned,
            }
            if !clock.wait(limits.drive_delay) {
                break;
            }
        }
        let _ = self.controller.fuse_cleanup_uncertain(cleanup_owner);
        Err(Box::new(LocalSemanticRestartFailureV1::RetirementCleanup {
            cleanup: retirement,
        }))
    }

    fn reconcile_receipt(
        &mut self,
        cleanup_owner: crate::SemanticProviderRestartCleanupV1,
        receipt: LocalSemanticRetirementReceiptV1,
    ) -> Result<LocalSemanticRetirementReceiptV1, Box<LocalSemanticRestartFailureV1<G, R, U>>> {
        let decision = self
            .controller
            .complete_cleanup(cleanup_owner, &receipt)
            .expect("fresh cleanup owner is exact");
        match decision {
            SemanticProviderRestartDecisionV1::Running { .. } => Ok(receipt),
            SemanticProviderRestartDecisionV1::Fused(reason) => {
                Err(Box::new(LocalSemanticRestartFailureV1::CleanupFailed {
                    reason,
                    receipt,
                }))
            }
            SemanticProviderRestartDecisionV1::Backoff(_) => {
                unreachable!("replacement cleanup never schedules another launch")
            }
        }
    }

    pub const fn query_disposition() -> SemanticProviderRestartQueryDispositionV1 {
        SemanticProviderRestartQueryDispositionV1::InvalidateWithoutReplay
    }
}
