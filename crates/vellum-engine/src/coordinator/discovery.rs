//! Native F01 scoped root cuts, context ownership and queries. Metadata is
//! literal, never executable registration or an external registry attestation.
use super::*;
use crate::capture::{CaptureLimitsV1, InertValueV1};
mod arguments;
mod ownership;
pub use arguments::*;
pub use ownership::builtin::*;
pub use ownership::decoder::{DiscoveryDecoderLimitsV1, DiscoveryDecoderUsageV1};
pub const OWNED_CONTRACT_DISCOVERY_PROFILE_V1: &str = "owned-contract-discovery/1";
pub const OWNED_CONTRACT_DISCOVERY_PROFILE_V2: &str = "owned-contract-discovery/2";
pub const OWNED_CONTRACT_DISCOVERY_PROFILE_V3: &str = "owned-contract-discovery/3";
pub const OWNED_CONTRACT_DISCOVERY_PROFILE_V4: &str = "owned-contract-discovery/4";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveryErrorV1 {
    Unavailable,
    InvalidRequest,
    Limit,
    Capacity,
    Faulted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiscoveryLimitsV1 {
    pub max_contexts: usize,
    pub max_preparations: usize,
    pub max_entries: usize,
    pub max_variants: usize,
    pub max_selectors: usize,
    pub max_issued: usize,
    pub max_page_rows: usize,
    pub max_root_bytes: usize,
    pub max_packet_bytes: usize,
    pub max_context_bytes: usize,
    pub max_capture_bytes: usize,
    pub max_row_bytes: usize,
    pub max_selector_bytes: usize,
    pub max_reply_bytes: usize,
    pub metadata: CaptureLimitsV1,
    pub max_held_bytes: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveryScopeV1 {
    pub context: SettingsExecutionContextV1,
    pub policy_epoch: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveryExecutionKindV1 {
    Query,
    LocalCommand,
    EffectRequest,
    EffectObservation,
}
#[derive(Clone, Copy)]
pub struct DiscoveryCommandV1<'a> {
    pub authority: &'a str,
    pub id: &'a str,
    pub incarnation: u64,
}
pub struct DiscoveryVariantInputV1<'a> {
    pub id: &'a str,
    pub input: InertValueV1<'a>,
    pub outcomes: InertValueV1<'a>,
    pub requirements: InertValueV1<'a>,
    pub limits: InertValueV1<'a>,
    pub cancellation: InertValueV1<'a>,
    /// None is hidden; visible order must be dense and independently attested.
    pub order: Option<usize>,
    pub enabled: Option<bool>,
}
pub struct DiscoveryEntryInputV1<'a> {
    pub command: DiscoveryCommandV1<'a>,
    pub contract: &'a str,
    pub version: u64,
    pub kind: DiscoveryExecutionKindV1,
    pub variants: &'a [DiscoveryVariantInputV1<'a>],
    pub presentation: InertValueV1<'a>,
    pub order: Option<usize>,
    pub schema_visible: bool,
    pub presentation_visible: bool,
    pub advice_visible: bool,
}
pub struct DiscoveryTargetInputV1<'a> {
    pub identity: &'a str,
    pub basis: InertValueV1<'a>,
}
pub struct DiscoveryCutInputV1<'a> {
    pub entries: &'a [DiscoveryEntryInputV1<'a>],
    pub target: Option<DiscoveryTargetInputV1<'a>>,
    pub advice_present: bool,
}
/// Trusted producer/host assertions, not claims sourced from descriptor text.
#[derive(Clone, Copy, Default)]
pub struct TrustedDiscoveryFactsV1 {
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub discovery_allowed: Option<bool>,
    pub interpretation_current: Option<bool>,
    pub accepted_registry_attested: Option<bool>,
    pub schema_descriptions_attested: Option<bool>,
    pub visibility_attested: Option<bool>,
    pub visibility_complete: Option<bool>,
    pub projection_order_attested: Option<bool>,
    pub outside_publication_guard: Option<bool>,
    pub resources_ready: Option<bool>,
    pub preparation_reserved: Option<bool>,
    pub retention_ready: Option<bool>,
    /// Existing capture remains authorized for release; unknown is not loss.
    pub retained: Option<bool>,
    pub transfer_ready: Option<bool>,
    pub registry_ready: Option<bool>,
    pub reply_ready: Option<bool>,
    pub target_binding_attested: Option<bool>,
    pub target_equality_observable: Option<bool>,
    pub advice_attested: Option<bool>,
    pub target_current: Option<bool>,
    pub owner_control_allowed: Option<bool>,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
}
macro_rules! opaque {
    ($name:ident) => {
        #[derive(Clone)]
        pub struct $name {
            owner: Arc<()>,
            marker: Arc<()>,
            slot: usize,
        }
        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                Arc::ptr_eq(&self.owner, &other.owner)
                    && Arc::ptr_eq(&self.marker, &other.marker)
                    && self.slot == other.slot
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
opaque!(DiscoveryCutIdV1);
opaque!(DiscoveryContextIdV1);
opaque!(DiscoveryTargetIdV1);
opaque!(DiscoveryEntryIdV1);
opaque!(DiscoveryVariantIdV1);
opaque!(DiscoveryCursorIdV1);
/// Native role-bearing arguments; no identifier serialization or wire codec.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveryQueryV1<'a> {
    List {
        limit: usize,
        continuation: Option<&'a DiscoveryCursorIdV1>,
    },
    Inspect {
        entry: &'a DiscoveryEntryIdV1,
    },
    AdvisoryEnablement {
        entry: &'a DiscoveryEntryIdV1,
        variant: &'a DiscoveryVariantIdV1,
        target: &'a DiscoveryTargetIdV1,
    },
}
#[derive(Debug)]
pub struct DiscoveryListEntryV1 {
    pub reference: DiscoveryEntryIdV1,
    pub version: u64,
    pub kind: DiscoveryExecutionKindV1,
}
#[derive(Debug)]
pub struct DiscoveryVariantDescriptorV1 {
    pub reference: DiscoveryVariantIdV1,
    /// All five literal schema facets as one immutable object, or none.
    /// This is descriptive data, not an executable schema or permission.
    pub schema: Option<Arc<crate::capture::InertCaptureV1>>,
}
#[derive(Debug)]
pub struct DiscoveryDescriptorV1 {
    pub reference: DiscoveryEntryIdV1,
    pub version: u64,
    pub kind: DiscoveryExecutionKindV1,
    pub variants: Vec<DiscoveryVariantDescriptorV1>,
    pub presentation: Option<Arc<crate::capture::InertCaptureV1>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveryCoverageV1 {
    CapturedPermitted,
}
#[derive(Debug)]
pub enum DiscoveryReplyV1 {
    Page {
        context: DiscoveryContextIdV1,
        coverage: DiscoveryCoverageV1,
        entries: Vec<DiscoveryListEntryV1>,
        continuation: Option<DiscoveryCursorIdV1>,
    },
    Descriptor {
        context: DiscoveryContextIdV1,
        descriptor: DiscoveryDescriptorV1,
    },
    Advice {
        context: DiscoveryContextIdV1,
        entry: DiscoveryEntryIdV1,
        variant: DiscoveryVariantIdV1,
        target: DiscoveryTargetIdV1,
        enabled: bool,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveryAcquisitionV1 {
    pub context: DiscoveryContextIdV1,
    pub target: Option<DiscoveryTargetIdV1>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiscoveryUsageV1 {
    pub contexts: usize,
    pub preparations: usize,
    pub issued_lifetime: usize,
    pub reserved_bytes: usize,
    pub stopped: bool,
    pub faulted: bool,
    pub prepared_advice: usize,
    pub selectors: usize,
}
/// Trusted-owner capture gauges, not recipient-safe discovery products.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiscoveryContextUsageV1 {
    pub stopped: bool,
    pub entries: usize,
    pub variants: usize,
    pub target_captured: bool,
    pub capture_bytes: usize,
}
pub(super) use ownership::State;
type Error = DiscoveryErrorV1;
fn observe<T>(r: &mut Recorder, b: Boundary, v: &Result<T, Error>) {
    observe_timed(r, b, v, None);
}
fn observe_timed<T>(
    r: &mut Recorder,
    b: Boundary,
    v: &Result<T, Error>,
    elapsed: Option<std::time::Duration>,
) {
    r.record(Observation {
        boundary: b,
        disposition: match v {
            Ok(_) => Disposition::Completed,
            Err(Error::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time: elapsed,
    });
}
