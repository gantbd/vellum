use super::*;
pub const BOUNDED_OPTIONAL_CAPTURE_PROFILE_V1: &str = "bounded-optional-capture/1";
#[derive(Clone, PartialEq, Eq)]
pub struct DiagnosticScopeV1 {
    pub context: SettingsExecutionContextV1,
    pub policy: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticSiteV1 {
    Ins01,
    Ins02,
    Ins03,
    Ins04,
    Ins05,
    Ins06,
    Ins07,
    Ins08,
    Ins09,
    Ins10,
    Ins11,
    Ins12,
    Ins13,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticMilestoneV1 {
    Offered,
    Admitted,
    Completed,
    Rejected,
    Cancelled,
    Discarded,
}
impl DiagnosticMilestoneV1 {
    pub fn code(self) -> &'static str {
        match self {
            Self::Offered => "offered",
            Self::Admitted => "admitted",
            Self::Completed => "completed",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
            Self::Discarded => "discarded",
        }
    }
}
impl DiagnosticSiteV1 {
    pub fn code(self) -> &'static str {
        match self {
            Self::Ins01 => "INS-01",
            Self::Ins02 => "INS-02",
            Self::Ins03 => "INS-03",
            Self::Ins04 => "INS-04",
            Self::Ins05 => "INS-05",
            Self::Ins06 => "INS-06",
            Self::Ins07 => "INS-07",
            Self::Ins08 => "INS-08",
            Self::Ins09 => "INS-09",
            Self::Ins10 => "INS-10",
            Self::Ins11 => "INS-11",
            Self::Ins12 => "INS-12",
            Self::Ins13 => "INS-13",
        }
    }
}
pub(super) fn parse_marker(value: &crate::capture::InertValueV1<'_>) -> Option<DiagnosticMarkerV1> {
    use crate::capture::InertValueV1 as D;
    let D::Object(rows) = value else { return None };
    if rows.len() != 3
        || ["schema", "site", "milestone"]
            .iter()
            .any(|k| rows.iter().filter(|(name, _)| name == k).count() != 1)
    {
        return None;
    }
    let text = |name| match rows.iter().find(|(k, _)| *k == name)?.1 {
        D::Text(s) => Some(s),
        _ => None,
    };
    if text("schema")? != "boundary-marker/1" {
        return None;
    }
    let site = match text("site")? {
        "INS-01" => DiagnosticSiteV1::Ins01,
        "INS-02" => DiagnosticSiteV1::Ins02,
        "INS-03" => DiagnosticSiteV1::Ins03,
        "INS-04" => DiagnosticSiteV1::Ins04,
        "INS-05" => DiagnosticSiteV1::Ins05,
        "INS-06" => DiagnosticSiteV1::Ins06,
        "INS-07" => DiagnosticSiteV1::Ins07,
        "INS-08" => DiagnosticSiteV1::Ins08,
        "INS-09" => DiagnosticSiteV1::Ins09,
        "INS-10" => DiagnosticSiteV1::Ins10,
        "INS-11" => DiagnosticSiteV1::Ins11,
        "INS-12" => DiagnosticSiteV1::Ins12,
        "INS-13" => DiagnosticSiteV1::Ins13,
        _ => return None,
    };
    let milestone = match text("milestone")? {
        "offered" => DiagnosticMilestoneV1::Offered,
        "admitted" => DiagnosticMilestoneV1::Admitted,
        "completed" => DiagnosticMilestoneV1::Completed,
        "rejected" => DiagnosticMilestoneV1::Rejected,
        "cancelled" => DiagnosticMilestoneV1::Cancelled,
        "discarded" => DiagnosticMilestoneV1::Discarded,
        _ => return None,
    };
    Some(DiagnosticMarkerV1 { site, milestone })
}
/// The schema is intrinsically boundary-marker/1. Descriptive caller observation,
/// not operation evidence; no arbitrary metadata, identity, duration or content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticMarkerV1 {
    pub site: DiagnosticSiteV1,
    pub milestone: DiagnosticMilestoneV1,
}
impl DiagnosticMarkerV1 {
    /// Canonical UTF-8 marker JSON units for queue pressure only. This is not a
    /// serialization of opaque generations, context, task handles or Rust replies.
    pub fn canonical_record_bytes(self) -> usize {
        b"{\"schema\":\"boundary-marker/1\",\"site\":\"INS-01\",\"milestone\":\"".len()
            + self.milestone.code().len()
            + 2
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticCaptureLimitsV1 {
    pub max_records: usize,
    pub max_queue_bytes: usize,
    pub max_preparations: usize,
    pub max_prepared_issued: usize,
    pub max_generations: usize,
    pub max_record_bytes: usize,
    pub max_context_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_health_reply_bytes: usize,
    pub max_drain_records: usize,
    pub max_counter: u64,
    pub max_retention_steps: usize,
    pub max_held_bytes: usize,
}
impl DiagnosticCaptureLimitsV1 {
    /// Checked native logical reservation: fixed slot capacity, canonical marker
    /// budgets, copied context and two separate data/health staging allowances.
    /// No numeric defaults, allocator/RSS or JSON-reply/wire bound is implied.
    pub fn required_hold_bytes(self) -> Result<usize> {
        if self.max_generations == 0
            || self.max_retention_steps == 0
            || self.max_drain_records == 0
            || self.max_record_bytes
                < (DiagnosticMarkerV1 {
                    site: DiagnosticSiteV1::Ins01,
                    milestone: DiagnosticMilestoneV1::Discarded,
                })
                .canonical_record_bytes()
            || self.max_reply_bytes
                < size_of::<DiagnosticBatchV1>() + size_of::<DiagnosticMarkerV1>()
            || self.max_health_reply_bytes < size_of::<DiagnosticHealthV1>()
        {
            return Err(CoordinatorError::Limit);
        }
        let mut n = size_of::<Pool>();
        for (count, bytes) in [
            (self.max_records, size_of::<Queued>()),
            (self.max_preparations, size_of::<Option<Prepared>>()),
            (1, self.max_context_bytes),
            (1, self.max_queue_bytes),
            (self.max_preparations, self.max_record_bytes),
            (2, self.max_reply_bytes),
            (2, self.max_health_reply_bytes),
        ] {
            n = n
                .checked_add(count.checked_mul(bytes).ok_or(CoordinatorError::Limit)?)
                .ok_or(CoordinatorError::Limit)?;
        }
        if n > self.max_held_bytes {
            Err(CoordinatorError::Limit)
        } else {
            Ok(n)
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticModeV1 {
    Enabled,
    Stopped,
    Disabled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticClockV1 {
    Unobserved,
    Valid,
    Invalid,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticCaptureDecisionV1 {
    Captured,
    Dropped,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticCounterV1 {
    pub value: u64,
    pub exact: bool,
}
impl Default for DiagnosticCounterV1 {
    fn default() -> Self {
        Self {
            value: 0,
            exact: true,
        }
    }
}
impl DiagnosticCounterV1 {
    pub(super) fn increment(&mut self, n: usize, maximum: u64) {
        let amount = u64::try_from(n).ok();
        if amount.is_none_or(|n| n > maximum.saturating_sub(self.value)) {
            self.value = maximum;
            self.exact = false;
        } else {
            self.value += amount.expect("bounded amount");
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticCountersV1 {
    pub offered: DiagnosticCounterV1,
    pub captured: DiagnosticCounterV1,
    pub drained: DiagnosticCounterV1,
    pub dropped: DiagnosticCounterV1,
    pub invalid: DiagnosticCounterV1,
    pub discarded: DiagnosticCounterV1,
    pub preparation_refused: DiagnosticCounterV1,
}
impl Default for DiagnosticCountersV1 {
    fn default() -> Self {
        let zero = DiagnosticCounterV1 {
            value: 0,
            exact: true,
        };
        Self {
            offered: zero,
            captured: zero,
            drained: zero,
            dropped: zero,
            invalid: zero,
            discarded: zero,
            preparation_refused: zero,
        }
    }
}
#[derive(Clone)]
pub struct DiagnosticGenerationV1 {
    pub(super) owner: Arc<()>,
    pub(super) ordinal: usize,
}
impl PartialEq for DiagnosticGenerationV1 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.ordinal == other.ordinal
    }
}
impl Eq for DiagnosticGenerationV1 {}
#[derive(Clone, PartialEq, Eq)]
pub struct DiagnosticPreparationV1 {
    pub(super) generation: DiagnosticGenerationV1,
    pub(super) serial: usize,
    pub(super) slot: usize,
}
/// Unique native consumer handle. Move it into its actual consumer owner; it is
/// deliberately not Clone. Dropping it does not silently release the queue hold.
pub struct DiagnosticDrainBindingV1 {
    pub(super) owner: Arc<()>,
    pub(super) marker: Arc<()>,
}
#[derive(Debug, PartialEq, Eq)]
pub struct DiagnosticBatchV1 {
    pub generation: DiagnosticGenerationV1,
    pub records: Vec<DiagnosticMarkerV1>,
    pub remaining: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticHealthV1 {
    pub generation: DiagnosticGenerationV1,
    pub mode: DiagnosticModeV1,
    pub clock: DiagnosticClockV1,
    pub queued_records: usize,
    pub queued_canonical_bytes: usize,
    pub counters: DiagnosticCountersV1,
}
#[derive(Clone, Copy)]
pub struct TrustedDiagnosticFactsV1<'a> {
    pub scope: &'a DiagnosticScopeV1,
    pub authorized: Option<bool>,
    pub scope_allowed: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub capture_allowed: Option<bool>,
    pub capture_ready: Option<bool>,
    pub release_allowed: Option<bool>,
    pub retained: Option<bool>,
    pub transfer_ready: Option<bool>,
    pub reply_ready: Option<bool>,
    pub resources_ready: Option<bool>,
    pub preparation_reserved: Option<bool>,
    pub outside_publication_guard: Option<bool>,
}
pub struct DiagnosticControlV1<'a> {
    pub scope: &'a DiagnosticScopeV1,
    pub owner_control_allowed: Option<bool>,
    pub resources_ready: Option<bool>,
    pub preparation_reserved: Option<bool>,
    pub binding_ready: Option<bool>,
    pub outside_publication_guard: Option<bool>,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
    pub health_allowed: Option<bool>,
    pub reply_ready: Option<bool>,
}
macro_rules! redacted { ($($t:ty),*) => { $(impl std::fmt::Debug for $t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("<diagnostic:opaque>") }
})* }; }
redacted!(
    DiagnosticScopeV1,
    DiagnosticGenerationV1,
    DiagnosticPreparationV1,
    DiagnosticDrainBindingV1
);
