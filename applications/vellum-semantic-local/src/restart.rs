//! Bounded provider-restart policy with no clock, launch, or query authority.
//!
//! The controller is deliberately smaller than a supervisor. Composition owns
//! the provider runtime, terminal cleanup receipt, backoff clock, and launch
//! result. This state machine only prevents overlapping generations, bounds
//! consecutive launch attempts, and permanently fuses when cleanup or launch
//! ownership is uncertain. It never retains or replays a semantic query.

use std::{num::NonZeroU64, time::Duration};

use crate::{
    LocalSemanticRetirementReceiptV1, LocalSemanticRetirementTerminalV1, MAX_DEADLINE_MILLIS_V1,
    SemanticProcessOwnedLaunchFailureV1,
};

pub const MAX_PROVIDER_RESTART_ATTEMPTS_V1: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticProviderRestartPolicyRefusalV1 {
    Empty,
    TooManyAttempts,
    InvalidBackoff,
}

/// Closed backoff schedule. Each entry authorizes exactly one consecutive
/// launch attempt after composition reports that the corresponding delay has
/// elapsed. The controller owns no clock and makes no elapsed-time claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticProviderRestartPolicyV1 {
    backoffs: Box<[Duration]>,
}

impl SemanticProviderRestartPolicyV1 {
    pub fn try_new(
        backoffs: Vec<Duration>,
    ) -> Result<Self, SemanticProviderRestartPolicyRefusalV1> {
        if backoffs.is_empty() {
            return Err(SemanticProviderRestartPolicyRefusalV1::Empty);
        }
        if backoffs.len() > MAX_PROVIDER_RESTART_ATTEMPTS_V1 {
            return Err(SemanticProviderRestartPolicyRefusalV1::TooManyAttempts);
        }
        if backoffs
            .iter()
            .any(|delay| delay.is_zero() || *delay > Duration::from_millis(MAX_DEADLINE_MILLIS_V1))
        {
            return Err(SemanticProviderRestartPolicyRefusalV1::InvalidBackoff);
        }
        Ok(Self {
            backoffs: backoffs.into_boxed_slice(),
        })
    }

    pub fn backoffs(&self) -> &[Duration] {
        &self.backoffs
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticProviderRestartCauseV1 {
    ProviderUnavailable,
    ProviderFailed,
    OperatorRequested,
}

/// Restart invalidates outstanding query work. No variant authorizes replay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticProviderRestartQueryDispositionV1 {
    InvalidateWithoutReplay,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticProviderRestartFuseV1 {
    AttemptsExhausted,
    CleanupUncertain,
    LaunchUncertain,
    GenerationExhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticProviderRestartStatusV1 {
    Running {
        generation: NonZeroU64,
    },
    Cleaning {
        generation: NonZeroU64,
    },
    Backoff {
        attempt: u32,
        delay: Duration,
    },
    Launching {
        attempt: u32,
        generation: NonZeroU64,
    },
    Fused(SemanticProviderRestartFuseV1),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticProviderRestartRefusalV1 {
    Busy,
    Fused(SemanticProviderRestartFuseV1),
    StaleOwner,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LaunchResolution {
    Started,
    DefinitivelyRefused,
    OutcomeUnknown,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SemanticProviderRestartUsageV1 {
    pub restart_requests: u64,
    pub request_refusals: u64,
    pub cleanup_confirmed: u64,
    pub cleanup_uncertain: u64,
    pub backoffs_issued: u64,
    pub launches_admitted: u64,
    pub launches_started: u64,
    pub launches_refused: u64,
    pub launches_uncertain: u64,
    pub fuses: u64,
    pub saturated: bool,
}

#[must_use = "restart cleanup ownership must be reconciled"]
pub struct SemanticProviderRestartCleanupV1 {
    sequence: NonZeroU64,
    generation: NonZeroU64,
    cause: SemanticProviderRestartCauseV1,
}

impl SemanticProviderRestartCleanupV1 {
    pub const fn generation(&self) -> NonZeroU64 {
        self.generation
    }

    pub const fn cause(&self) -> SemanticProviderRestartCauseV1 {
        self.cause
    }

    pub const fn query_disposition(&self) -> SemanticProviderRestartQueryDispositionV1 {
        SemanticProviderRestartQueryDispositionV1::InvalidateWithoutReplay
    }
}

impl std::fmt::Debug for SemanticProviderRestartCleanupV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SemanticProviderRestartCleanupV1")
            .field("generation", &self.generation)
            .field("cause", &self.cause)
            .field("query_disposition", &self.query_disposition())
            .finish()
    }
}

#[must_use = "restart backoff must be elapsed or retained"]
pub struct SemanticProviderRestartBackoffV1 {
    sequence: NonZeroU64,
    attempt: u32,
    delay: Duration,
}

impl SemanticProviderRestartBackoffV1 {
    pub const fn attempt(&self) -> u32 {
        self.attempt
    }

    pub const fn delay(&self) -> Duration {
        self.delay
    }
}

impl std::fmt::Debug for SemanticProviderRestartBackoffV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SemanticProviderRestartBackoffV1")
            .field("attempt", &self.attempt)
            .field("delay", &self.delay)
            .finish()
    }
}

#[must_use = "restart launch admission must resolve exactly once"]
pub struct SemanticProviderRestartLaunchV1 {
    sequence: NonZeroU64,
    attempt: u32,
    generation: NonZeroU64,
}

impl SemanticProviderRestartLaunchV1 {
    pub const fn attempt(&self) -> u32 {
        self.attempt
    }

    pub const fn generation(&self) -> NonZeroU64 {
        self.generation
    }
}

impl std::fmt::Debug for SemanticProviderRestartLaunchV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SemanticProviderRestartLaunchV1")
            .field("attempt", &self.attempt)
            .field("generation", &self.generation)
            .finish()
    }
}

#[must_use = "refused restart transition retains its affine owner"]
pub struct SemanticProviderRestartOwnerFailureV1<T> {
    refusal: SemanticProviderRestartRefusalV1,
    owner: T,
}

impl<T> SemanticProviderRestartOwnerFailureV1<T> {
    pub const fn refusal(&self) -> SemanticProviderRestartRefusalV1 {
        self.refusal
    }

    pub fn into_owner(self) -> T {
        self.owner
    }
}

impl<T> std::fmt::Debug for SemanticProviderRestartOwnerFailureV1<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SemanticProviderRestartOwnerFailureV1")
            .field("refusal", &self.refusal)
            .field("owner", &"<owned>")
            .finish()
    }
}

/// A controller refusal never discards the launch failure or the admission
/// token needed to reconcile it.
#[must_use = "refused owned-launch reconciliation retains both affine owners"]
pub struct SemanticProviderRestartOwnedLaunchFailureV1 {
    refusal: SemanticProviderRestartRefusalV1,
    launch: SemanticProviderRestartLaunchV1,
    failure: SemanticProcessOwnedLaunchFailureV1,
}

impl SemanticProviderRestartOwnedLaunchFailureV1 {
    pub const fn refusal(&self) -> SemanticProviderRestartRefusalV1 {
        self.refusal
    }

    pub fn into_owners(
        self,
    ) -> (
        SemanticProviderRestartLaunchV1,
        SemanticProcessOwnedLaunchFailureV1,
    ) {
        (self.launch, self.failure)
    }
}

impl std::fmt::Debug for SemanticProviderRestartOwnedLaunchFailureV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SemanticProviderRestartOwnedLaunchFailureV1")
            .field("refusal", &self.refusal)
            .field("launch", &"<owned>")
            .field("failure", &"<owned>")
            .finish()
    }
}

#[must_use = "restart decision may retain a backoff obligation"]
pub enum SemanticProviderRestartDecisionV1 {
    Backoff(SemanticProviderRestartBackoffV1),
    Running { generation: NonZeroU64 },
    Fused(SemanticProviderRestartFuseV1),
}

impl std::fmt::Debug for SemanticProviderRestartDecisionV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Backoff(backoff) => formatter.debug_tuple("Backoff").field(backoff).finish(),
            Self::Running { generation } => formatter
                .debug_struct("Running")
                .field("generation", generation)
                .finish(),
            Self::Fused(reason) => formatter.debug_tuple("Fused").field(reason).finish(),
        }
    }
}

enum RestartState {
    Running {
        generation: NonZeroU64,
    },
    Cleaning {
        sequence: NonZeroU64,
        generation: NonZeroU64,
    },
    Backoff {
        sequence: NonZeroU64,
        attempt: u32,
        delay: Duration,
    },
    Launching {
        sequence: NonZeroU64,
        attempt: u32,
        generation: NonZeroU64,
    },
    Fused(SemanticProviderRestartFuseV1),
}

/// Policy-only single-flight provider restart controller.
///
/// Construction begins with one already-running provider. The controller does
/// not launch or retire it; exact cleanup and launch owners remain with the
/// semantic-local composition and are reconciled through the returned tokens.
pub struct SemanticProviderRestartControllerV1 {
    policy: SemanticProviderRestartPolicyV1,
    state: RestartState,
    next_sequence: u64,
    last_generation: NonZeroU64,
    usage: SemanticProviderRestartUsageV1,
}

impl SemanticProviderRestartControllerV1 {
    pub fn new_running(policy: SemanticProviderRestartPolicyV1, generation: NonZeroU64) -> Self {
        Self {
            policy,
            state: RestartState::Running { generation },
            next_sequence: 1,
            last_generation: generation,
            usage: SemanticProviderRestartUsageV1::default(),
        }
    }

    pub const fn usage(&self) -> SemanticProviderRestartUsageV1 {
        self.usage
    }

    pub fn status(&self) -> SemanticProviderRestartStatusV1 {
        match self.state {
            RestartState::Running { generation } => {
                SemanticProviderRestartStatusV1::Running { generation }
            }
            RestartState::Cleaning { generation, .. } => {
                SemanticProviderRestartStatusV1::Cleaning { generation }
            }
            RestartState::Backoff { attempt, delay, .. } => {
                SemanticProviderRestartStatusV1::Backoff { attempt, delay }
            }
            RestartState::Launching {
                attempt,
                generation,
                ..
            } => SemanticProviderRestartStatusV1::Launching {
                attempt,
                generation,
            },
            RestartState::Fused(reason) => SemanticProviderRestartStatusV1::Fused(reason),
        }
    }

    pub fn request_restart(
        &mut self,
        cause: SemanticProviderRestartCauseV1,
    ) -> Result<SemanticProviderRestartCleanupV1, SemanticProviderRestartRefusalV1> {
        let generation = match self.state {
            RestartState::Running { generation } => generation,
            RestartState::Fused(reason) => return self.refuse(reason.into()),
            _ => return self.refuse(SemanticProviderRestartRefusalV1::Busy),
        };
        let Some(sequence) = NonZeroU64::new(self.next_sequence) else {
            return self.fuse_and_refuse(SemanticProviderRestartFuseV1::GenerationExhausted);
        };
        let Some(next_sequence) = self.next_sequence.checked_add(1) else {
            return self.fuse_and_refuse(SemanticProviderRestartFuseV1::GenerationExhausted);
        };
        self.next_sequence = next_sequence;
        self.state = RestartState::Cleaning {
            sequence,
            generation,
        };
        bump(&mut self.usage.restart_requests, &mut self.usage.saturated);
        Ok(SemanticProviderRestartCleanupV1 {
            sequence,
            generation,
            cause,
        })
    }

    pub fn complete_cleanup(
        &mut self,
        owner: SemanticProviderRestartCleanupV1,
        receipt: &LocalSemanticRetirementReceiptV1,
    ) -> Result<
        SemanticProviderRestartDecisionV1,
        SemanticProviderRestartOwnerFailureV1<SemanticProviderRestartCleanupV1>,
    > {
        self.complete_cleanup_terminal(owner, receipt.terminal())
    }

    fn complete_cleanup_terminal(
        &mut self,
        owner: SemanticProviderRestartCleanupV1,
        terminal: LocalSemanticRetirementTerminalV1,
    ) -> Result<
        SemanticProviderRestartDecisionV1,
        SemanticProviderRestartOwnerFailureV1<SemanticProviderRestartCleanupV1>,
    > {
        let matches = matches!(
            self.state,
            RestartState::Cleaning { sequence, generation }
                if sequence == owner.sequence && generation == owner.generation
        );
        if !matches {
            return Err(SemanticProviderRestartOwnerFailureV1 {
                refusal: self.current_refusal(),
                owner,
            });
        }
        match terminal {
            LocalSemanticRetirementTerminalV1::GracefulConfirmed
            | LocalSemanticRetirementTerminalV1::ForcedConfirmed => {
                bump(&mut self.usage.cleanup_confirmed, &mut self.usage.saturated);
                Ok(self.schedule_backoff(1))
            }
            LocalSemanticRetirementTerminalV1::CleanupFailed => {
                bump(&mut self.usage.cleanup_uncertain, &mut self.usage.saturated);
                Ok(self.fuse(SemanticProviderRestartFuseV1::CleanupUncertain))
            }
        }
    }

    pub fn backoff_elapsed(
        &mut self,
        owner: SemanticProviderRestartBackoffV1,
    ) -> Result<
        SemanticProviderRestartLaunchV1,
        SemanticProviderRestartOwnerFailureV1<SemanticProviderRestartBackoffV1>,
    > {
        let matches = matches!(
            self.state,
            RestartState::Backoff { sequence, attempt, delay }
                if sequence == owner.sequence && attempt == owner.attempt && delay == owner.delay
        );
        if !matches {
            return Err(SemanticProviderRestartOwnerFailureV1 {
                refusal: self.current_refusal(),
                owner,
            });
        }
        let Some(raw_generation) = self.last_generation.get().checked_add(1) else {
            let _ = self.fuse(SemanticProviderRestartFuseV1::GenerationExhausted);
            return Err(SemanticProviderRestartOwnerFailureV1 {
                refusal: SemanticProviderRestartRefusalV1::Fused(
                    SemanticProviderRestartFuseV1::GenerationExhausted,
                ),
                owner,
            });
        };
        let generation = NonZeroU64::new(raw_generation).expect("checked successor is nonzero");
        self.last_generation = generation;
        self.state = RestartState::Launching {
            sequence: owner.sequence,
            attempt: owner.attempt,
            generation,
        };
        bump(&mut self.usage.launches_admitted, &mut self.usage.saturated);
        Ok(SemanticProviderRestartLaunchV1 {
            sequence: owner.sequence,
            attempt: owner.attempt,
            generation,
        })
    }

    /// Confirms that composition now owns a started provider for this exact
    /// admission. Failures must use `complete_owned_launch_failure` so an
    /// unresolved launch cannot be weakened into a definitive refusal.
    pub fn confirm_launch_started(
        &mut self,
        owner: SemanticProviderRestartLaunchV1,
    ) -> Result<
        SemanticProviderRestartDecisionV1,
        SemanticProviderRestartOwnerFailureV1<SemanticProviderRestartLaunchV1>,
    > {
        self.complete_launch_resolution(owner, LaunchResolution::Started)
    }

    fn complete_launch_resolution(
        &mut self,
        owner: SemanticProviderRestartLaunchV1,
        outcome: LaunchResolution,
    ) -> Result<
        SemanticProviderRestartDecisionV1,
        SemanticProviderRestartOwnerFailureV1<SemanticProviderRestartLaunchV1>,
    > {
        let matches = matches!(
            self.state,
            RestartState::Launching { sequence, attempt, generation }
                if sequence == owner.sequence
                    && attempt == owner.attempt
                    && generation == owner.generation
        );
        if !matches {
            return Err(SemanticProviderRestartOwnerFailureV1 {
                refusal: self.current_refusal(),
                owner,
            });
        }
        Ok(match outcome {
            LaunchResolution::Started => {
                self.state = RestartState::Running {
                    generation: owner.generation,
                };
                bump(&mut self.usage.launches_started, &mut self.usage.saturated);
                SemanticProviderRestartDecisionV1::Running {
                    generation: owner.generation,
                }
            }
            LaunchResolution::DefinitivelyRefused => {
                bump(&mut self.usage.launches_refused, &mut self.usage.saturated);
                self.schedule_backoff(owner.attempt.saturating_add(1))
            }
            LaunchResolution::OutcomeUnknown => {
                bump(
                    &mut self.usage.launches_uncertain,
                    &mut self.usage.saturated,
                );
                self.fuse(SemanticProviderRestartFuseV1::LaunchUncertain)
            }
        })
    }

    /// Reconciles the launch substrate's owned failure without letting a
    /// caller weaken an unresolved launch into a definitive refusal. The
    /// original failure is returned for cleanup or audit ownership.
    pub fn complete_owned_launch_failure(
        &mut self,
        launch: SemanticProviderRestartLaunchV1,
        failure: SemanticProcessOwnedLaunchFailureV1,
    ) -> Result<
        (
            SemanticProviderRestartDecisionV1,
            SemanticProcessOwnedLaunchFailureV1,
        ),
        SemanticProviderRestartOwnedLaunchFailureV1,
    > {
        let outcome = match &failure {
            SemanticProcessOwnedLaunchFailureV1::Confirmed(_) => {
                LaunchResolution::DefinitivelyRefused
            }
            SemanticProcessOwnedLaunchFailureV1::Unresolved(_) => LaunchResolution::OutcomeUnknown,
        };
        match self.complete_launch_resolution(launch, outcome) {
            Ok(decision) => Ok((decision, failure)),
            Err(refused) => Err(SemanticProviderRestartOwnedLaunchFailureV1 {
                refusal: refused.refusal(),
                launch: refused.into_owner(),
                failure,
            }),
        }
    }

    fn schedule_backoff(&mut self, attempt: u32) -> SemanticProviderRestartDecisionV1 {
        let index = usize::try_from(attempt.saturating_sub(1)).unwrap_or(usize::MAX);
        let Some(delay) = self.policy.backoffs.get(index).copied() else {
            return self.fuse(SemanticProviderRestartFuseV1::AttemptsExhausted);
        };
        let sequence = NonZeroU64::new(self.next_sequence)
            .expect("request restart keeps the sequence nonzero");
        self.next_sequence = self.next_sequence.saturating_add(1);
        if self.next_sequence == u64::MAX && sequence.get() == u64::MAX {
            return self.fuse(SemanticProviderRestartFuseV1::GenerationExhausted);
        }
        self.state = RestartState::Backoff {
            sequence,
            attempt,
            delay,
        };
        bump(&mut self.usage.backoffs_issued, &mut self.usage.saturated);
        SemanticProviderRestartDecisionV1::Backoff(SemanticProviderRestartBackoffV1 {
            sequence,
            attempt,
            delay,
        })
    }

    fn current_refusal(&mut self) -> SemanticProviderRestartRefusalV1 {
        bump(&mut self.usage.request_refusals, &mut self.usage.saturated);
        match self.state {
            RestartState::Fused(reason) => SemanticProviderRestartRefusalV1::Fused(reason),
            _ => SemanticProviderRestartRefusalV1::StaleOwner,
        }
    }

    fn refuse<T>(
        &mut self,
        refusal: SemanticProviderRestartRefusalV1,
    ) -> Result<T, SemanticProviderRestartRefusalV1> {
        bump(&mut self.usage.request_refusals, &mut self.usage.saturated);
        Err(refusal)
    }

    fn fuse_and_refuse<T>(
        &mut self,
        reason: SemanticProviderRestartFuseV1,
    ) -> Result<T, SemanticProviderRestartRefusalV1> {
        let _ = self.fuse(reason);
        self.refuse(SemanticProviderRestartRefusalV1::Fused(reason))
    }

    fn fuse(&mut self, reason: SemanticProviderRestartFuseV1) -> SemanticProviderRestartDecisionV1 {
        self.state = RestartState::Fused(reason);
        bump(&mut self.usage.fuses, &mut self.usage.saturated);
        SemanticProviderRestartDecisionV1::Fused(reason)
    }
}

impl From<SemanticProviderRestartFuseV1> for SemanticProviderRestartRefusalV1 {
    fn from(value: SemanticProviderRestartFuseV1) -> Self {
        Self::Fused(value)
    }
}

fn bump(value: &mut u64, saturated: &mut bool) {
    let prior = *value;
    *value = value.saturating_add(1);
    *saturated |= prior == u64::MAX;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> SemanticProviderRestartPolicyV1 {
        SemanticProviderRestartPolicyV1::try_new(vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
        ])
        .unwrap()
    }

    fn confirmed_cleanup(
        controller: &mut SemanticProviderRestartControllerV1,
        owner: SemanticProviderRestartCleanupV1,
    ) -> SemanticProviderRestartDecisionV1 {
        controller
            .complete_cleanup_terminal(owner, LocalSemanticRetirementTerminalV1::GracefulConfirmed)
            .unwrap()
    }

    #[test]
    fn policy_is_nonempty_bounded_and_uses_explicit_nonzero_delays() {
        assert_eq!(
            SemanticProviderRestartPolicyV1::try_new(Vec::new()),
            Err(SemanticProviderRestartPolicyRefusalV1::Empty)
        );
        assert_eq!(
            SemanticProviderRestartPolicyV1::try_new(vec![Duration::ZERO]),
            Err(SemanticProviderRestartPolicyRefusalV1::InvalidBackoff)
        );
        assert_eq!(
            SemanticProviderRestartPolicyV1::try_new(vec![
                Duration::from_millis(1);
                MAX_PROVIDER_RESTART_ATTEMPTS_V1 + 1
            ]),
            Err(SemanticProviderRestartPolicyRefusalV1::TooManyAttempts)
        );
    }

    #[test]
    fn restart_is_single_flight_and_never_authorizes_query_replay() {
        let mut controller =
            SemanticProviderRestartControllerV1::new_running(policy(), NonZeroU64::new(7).unwrap());
        let cleanup = controller
            .request_restart(SemanticProviderRestartCauseV1::ProviderFailed)
            .unwrap();
        assert_eq!(
            cleanup.query_disposition(),
            SemanticProviderRestartQueryDispositionV1::InvalidateWithoutReplay
        );
        assert!(matches!(
            controller.request_restart(SemanticProviderRestartCauseV1::OperatorRequested),
            Err(SemanticProviderRestartRefusalV1::Busy)
        ));
        let SemanticProviderRestartDecisionV1::Backoff(backoff) =
            confirmed_cleanup(&mut controller, cleanup)
        else {
            panic!("confirmed cleanup must schedule explicit backoff")
        };
        assert_eq!(backoff.delay(), Duration::from_millis(10));
        assert!(matches!(
            controller.request_restart(SemanticProviderRestartCauseV1::ProviderUnavailable),
            Err(SemanticProviderRestartRefusalV1::Busy)
        ));
    }

    #[test]
    fn confirmed_cleanup_and_elapsed_backoff_admit_one_fresh_generation() {
        let mut controller =
            SemanticProviderRestartControllerV1::new_running(policy(), NonZeroU64::new(9).unwrap());
        let cleanup = controller
            .request_restart(SemanticProviderRestartCauseV1::ProviderUnavailable)
            .unwrap();
        let SemanticProviderRestartDecisionV1::Backoff(backoff) =
            confirmed_cleanup(&mut controller, cleanup)
        else {
            panic!()
        };
        let launch = controller.backoff_elapsed(backoff).unwrap();
        assert_eq!(launch.attempt(), 1);
        assert_eq!(launch.generation(), NonZeroU64::new(10).unwrap());
        assert!(matches!(
            controller.status(),
            SemanticProviderRestartStatusV1::Launching { attempt: 1, .. }
        ));
        let decision = controller.confirm_launch_started(launch).unwrap();
        assert!(matches!(
            decision,
            SemanticProviderRestartDecisionV1::Running { generation }
                if generation == NonZeroU64::new(10).unwrap()
        ));
        assert_eq!(controller.usage().launches_started, 1);
    }

    #[test]
    fn definitive_launch_refusals_exhaust_the_finite_schedule() {
        let mut controller =
            SemanticProviderRestartControllerV1::new_running(policy(), NonZeroU64::new(1).unwrap());
        let cleanup = controller
            .request_restart(SemanticProviderRestartCauseV1::ProviderFailed)
            .unwrap();
        let SemanticProviderRestartDecisionV1::Backoff(first) =
            confirmed_cleanup(&mut controller, cleanup)
        else {
            panic!()
        };
        let first = controller.backoff_elapsed(first).unwrap();
        let (decision, retained) = controller
            .complete_owned_launch_failure(
                first,
                SemanticProcessOwnedLaunchFailureV1::Confirmed(
                    crate::SemanticProcessLaunchRefusalV1::SpawnRefused,
                ),
            )
            .unwrap();
        assert_eq!(
            retained.reason(),
            crate::SemanticProcessLaunchRefusalV1::SpawnRefused
        );
        let SemanticProviderRestartDecisionV1::Backoff(second) = decision else {
            panic!()
        };
        assert_eq!(second.attempt(), 2);
        let second = controller.backoff_elapsed(second).unwrap();
        let (decision, retained) = controller
            .complete_owned_launch_failure(
                second,
                SemanticProcessOwnedLaunchFailureV1::Confirmed(
                    crate::SemanticProcessLaunchRefusalV1::SpawnRefused,
                ),
            )
            .unwrap();
        assert_eq!(
            retained.reason(),
            crate::SemanticProcessLaunchRefusalV1::SpawnRefused
        );
        assert!(matches!(
            decision,
            SemanticProviderRestartDecisionV1::Fused(
                SemanticProviderRestartFuseV1::AttemptsExhausted
            )
        ));
        assert!(matches!(
            controller.request_restart(SemanticProviderRestartCauseV1::OperatorRequested),
            Err(SemanticProviderRestartRefusalV1::Fused(
                SemanticProviderRestartFuseV1::AttemptsExhausted
            ))
        ));
    }

    #[test]
    fn uncertain_launch_fuses_without_a_retry_or_reset_path() {
        let mut controller =
            SemanticProviderRestartControllerV1::new_running(policy(), NonZeroU64::new(1).unwrap());
        let cleanup = controller
            .request_restart(SemanticProviderRestartCauseV1::ProviderFailed)
            .unwrap();
        let SemanticProviderRestartDecisionV1::Backoff(backoff) =
            confirmed_cleanup(&mut controller, cleanup)
        else {
            panic!()
        };
        let launch = controller.backoff_elapsed(backoff).unwrap();
        assert!(matches!(
            controller
                .complete_launch_resolution(launch, LaunchResolution::OutcomeUnknown)
                .unwrap(),
            SemanticProviderRestartDecisionV1::Fused(
                SemanticProviderRestartFuseV1::LaunchUncertain
            )
        ));
        assert_eq!(controller.usage().launches_uncertain, 1);
        assert_eq!(controller.usage().fuses, 1);
    }

    #[test]
    fn cleanup_failure_fuses_before_any_backoff_or_launch() {
        let mut controller =
            SemanticProviderRestartControllerV1::new_running(policy(), NonZeroU64::new(3).unwrap());
        let cleanup = controller
            .request_restart(SemanticProviderRestartCauseV1::ProviderFailed)
            .unwrap();
        assert!(matches!(
            controller
                .complete_cleanup_terminal(
                    cleanup,
                    LocalSemanticRetirementTerminalV1::CleanupFailed,
                )
                .unwrap(),
            SemanticProviderRestartDecisionV1::Fused(
                SemanticProviderRestartFuseV1::CleanupUncertain
            )
        ));
        assert_eq!(controller.usage().backoffs_issued, 0);
        assert_eq!(controller.usage().launches_admitted, 0);
    }
}
