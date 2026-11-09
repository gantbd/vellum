use super::*;
pub const OWNED_DIAGNOSTIC_DRAIN_PROFILE_V1: &str = "owned-diagnostic-drain/1";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticDrainErrorV1 {
    Unavailable,
    IntentMismatch,
    Limit,
    Capacity,
    Faulted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticDrainPhaseV1 {
    Issued,
    Pending,
    Terminal,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticDrainDispositionV1 {
    Performed,
    NotPerformed,
    Unknown,
}
#[derive(Clone)]
pub struct DiagnosticDrainTaskV1 {
    pub(super) owner: Arc<()>,
    pub(super) slot: usize,
    pub(super) serial: usize,
}
impl PartialEq for DiagnosticDrainTaskV1 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
            && self.slot == other.slot
            && self.serial == other.serial
    }
}
impl Eq for DiagnosticDrainTaskV1 {}
impl std::fmt::Debug for DiagnosticDrainTaskV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DiagnosticDrainTask(<opaque>)")
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticDrainIssuedV1 {
    pub task: DiagnosticDrainTaskV1,
    pub generation: DiagnosticGenerationV1,
}
#[derive(Debug, PartialEq, Eq)]
pub struct DiagnosticDrainResultV1 {
    pub task: DiagnosticDrainTaskV1,
    pub admitted: bool,
    pub drain: DiagnosticDrainDispositionV1,
    /// Null payload is not a claim of nonperformance, delivery or durable export.
    pub batch: Option<DiagnosticBatchV1>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticDrainControlReplyV1 {
    pub task: DiagnosticDrainTaskV1,
    pub phase: DiagnosticDrainPhaseV1,
    pub admitted: bool,
    pub drain: Option<DiagnosticDrainDispositionV1>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticDrainLimitsV1 {
    pub max_tasks: usize,
    pub max_issued: usize,
    pub max_context_bytes: usize,
    pub max_binding_bytes: usize,
    pub max_request_bytes: usize,
    pub max_batch_bytes: usize,
    pub max_result_bytes: usize,
    pub max_control_reply_bytes: usize,
    pub max_held_bytes: usize,
}
impl DiagnosticDrainLimitsV1 {
    /// Native logical storage/staging, not caller input, JSON replies or RSS.
    /// Actual queue maxima and both complete contexts are checked at open.
    pub fn required_hold_bytes(self) -> Result<usize> {
        let mut n = size_of::<Gateway>();
        for (count, bytes) in [
            (self.max_tasks, size_of::<Option<Row>>()),
            (2, self.max_context_bytes),
            (1, self.max_binding_bytes),
        ] {
            n = n
                .checked_add(
                    count
                        .checked_mul(bytes)
                        .ok_or(DiagnosticDrainErrorV1::Limit)?,
                )
                .ok_or(DiagnosticDrainErrorV1::Limit)?;
        }
        let per_task = self
            .max_request_bytes
            .checked_add(self.max_batch_bytes)
            .and_then(|n| n.checked_add(self.max_result_bytes.checked_mul(2)?))
            .and_then(|n| n.checked_add(self.max_control_reply_bytes.checked_mul(2)?))
            .ok_or(DiagnosticDrainErrorV1::Limit)?;
        n = n
            .checked_add(
                self.max_tasks
                    .checked_mul(per_task)
                    .ok_or(DiagnosticDrainErrorV1::Limit)?,
            )
            .ok_or(DiagnosticDrainErrorV1::Limit)?;
        if n > self.max_held_bytes {
            Err(DiagnosticDrainErrorV1::Limit)
        } else {
            Ok(n)
        }
    }
}
pub struct TrustedDiagnosticDrainFactsV1<'a> {
    pub scope: &'a DiagnosticScopeV1,
    pub authorized: Option<bool>,
    pub scope_allowed: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub outside_publication_guard: Option<bool>,
    pub drain_allowed: Option<bool>,
    pub resources_ready: Option<bool>,
    pub bookkeeping_reserved: Option<bool>,
    pub completion_reserved: Option<bool>,
    pub queue_interpretation_current: Option<bool>,
    pub drain_interpretation_current: Option<bool>,
    pub scheduler_owned: Option<bool>,
    pub result_allowed: Option<bool>,
    pub queue_release_allowed: Option<bool>,
    pub retained: Option<bool>,
    pub transfer_ready: Option<bool>,
    pub reply_ready: Option<bool>,
}
pub struct DiagnosticDrainControlV1<'a> {
    pub scope: &'a DiagnosticScopeV1,
    pub owner_control_allowed: Option<bool>,
    pub outside_publication_guard: Option<bool>,
    pub resources_ready: Option<bool>,
    pub preparation_reserved: Option<bool>,
    pub binding_ready: Option<bool>,
    pub queue_interpretation_current: Option<bool>,
    pub drain_interpretation_current: Option<bool>,
    pub control_allowed: Option<bool>,
    pub control_ready: Option<bool>,
    pub transfer_ready: Option<bool>,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
}
