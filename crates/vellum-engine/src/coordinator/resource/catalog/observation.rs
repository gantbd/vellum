//! Native scoped registry observations. No provider calls or editing-path hooks.
use super::*;
mod shutdown_inventory;
use crate::instrumentation::{Disposition, Observation};
use std::collections::VecDeque;
mod execution;
pub use commands::{
    RESOURCE_REGISTRY_COMMAND_PROFILE_V1, ResourceRegistryCommandFactsV1,
    ResourceRegistryCommandHistoricalSlotV1, ResourceRegistryCommandIdV1,
    ResourceRegistryCommandOperationV1, ResourceRegistryCommandPhaseV1,
    ResourceRegistryCommandProtocolFaultV1, ResourceRegistryCommandReplyV1,
    ResourceRegistryCommandRequestV1, TrustedResourceRegistryCommandAdmissionV1,
};
mod projection;
#[cfg(test)]
mod tests;
#[cfg(test)]
pub(in crate::coordinator) use tests::facts as registry_observation_test_facts;
pub const RESOURCE_REGISTRY_OBSERVATION_PROFILE_V1: &str = "scoped-resource-observation/1";
#[derive(Clone, PartialEq, Eq)]
pub struct ResourceRegistryObservationScopeV1 {
    pub context: SettingsExecutionContextV1,
    pub policy: String,
    pub read_descriptors: bool,
    pub write_descriptors: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceRegistryObservationLimitsV1 {
    pub max_streams: usize,
    pub max_preparations: usize,
    pub max_rows: usize,
    pub max_bindings: usize,
    pub max_issued: usize,
    pub max_root_bytes: usize,
    pub max_profile_bytes: usize,
    pub max_scope_bytes: usize,
    pub max_tag_bytes: usize,
    pub max_packet_bytes: usize,
    pub max_projection_bytes: usize,
    pub max_binding_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_queue: usize,
    pub max_held_bytes: usize,
}
impl ResourceRegistryObservationLimitsV1 {
    pub fn required_pool_bytes(self) -> Result<usize> {
        let per = add(
            add(
                add(
                    self.max_scope_bytes,
                    self.max_projection_bytes
                        .checked_mul(2)
                        .ok_or(ResourceError::Limit)?,
                )?,
                self.max_binding_bytes
                    .checked_mul(2)
                    .ok_or(ResourceError::Limit)?,
            )?,
            self.max_queue
                .checked_add(2)
                .and_then(|n| n.checked_mul(self.max_reply_bytes))
                .ok_or(ResourceError::Limit)?,
        )?;
        let n = add(
            add(
                add(size_of::<Pool>(), self.max_profile_bytes)?,
                self.max_preparations
                    .checked_mul(self.max_packet_bytes)
                    .ok_or(ResourceError::Limit)?,
            )?,
            self.max_streams
                .checked_mul(per)
                .ok_or(ResourceError::Limit)?,
        )?;
        let n = add(
            n,
            self.max_issued
                .checked_mul(size_of::<Option<Packet>>() + size_of::<Option<Observer>>())
                .ok_or(ResourceError::Limit)?,
        )?;
        if n > self.max_held_bytes {
            Err(ResourceError::Limit)
        } else {
            Ok(n)
        }
    }
}
macro_rules! identity {
    ($name:ident) => {
        #[derive(Clone)]
        pub struct $name {
            owner: Arc<()>,
            slot: usize,
        }
        impl PartialEq for $name {
            fn eq(&self, b: &Self) -> bool {
                Arc::ptr_eq(&self.owner, &b.owner) && self.slot == b.slot
            }
        }
        impl Eq for $name {}
    };
}
identity!(ResourceRegistryStreamIdV1);
identity!(ResourceRegistryPreparationIdV1);
#[derive(Clone, PartialEq, Eq)]
pub struct ResourceRegistryGenerationV1 {
    stream: ResourceRegistryStreamIdV1,
    serial: usize,
}
#[derive(Clone, PartialEq, Eq)]
pub struct ResourceRegistrySlotReferenceV1 {
    stream: ResourceRegistryStreamIdV1,
    index: usize,
}
#[derive(Clone)]
pub struct ResourceRegistryHeadIdV1 {
    stream: ResourceRegistryStreamIdV1,
    marker: Arc<()>,
    root: RootStamp,
}
/// Privileged classification input, not a recipient disclosure product.
pub struct ResourceRegistryHistoricalHeadV1 {
    pub head: ResourceRegistryHeadIdV1,
    pub projection: Arc<ResourceRegistryProjectionV1>,
}
#[derive(PartialEq, Eq)]
pub struct ResourceRegistryObservationRowV1 {
    pub reference: ResourceRegistrySlotReferenceV1,
    pub table: ResourceCatalogTableV1,
    /// The closed existing declaration whitelist; never the native configuration.
    pub descriptor: Option<ResourceCatalogCapabilitiesV1>,
    /// Companion field of the permitted writer descriptor. None for reads or
    /// withheld descriptors; this is metadata, never a provider identity grant.
    pub identity_known: Option<bool>,
}
#[derive(PartialEq, Eq)]
pub struct ResourceRegistryProjectionV1 {
    rows: Vec<ResourceRegistryObservationRowV1>,
}
impl ResourceRegistryProjectionV1 {
    pub fn rows(&self) -> &[ResourceRegistryObservationRowV1] {
        &self.rows
    }
    pub fn coverage(&self) -> &'static str {
        "captured-policy-current"
    }
}
#[derive(Clone)]
pub struct ResourceRegistryEventV1 {
    pub before: ResourceRegistryGenerationV1,
    pub after: ResourceRegistryGenerationV1,
    pub projection: Arc<ResourceRegistryProjectionV1>,
}
pub struct ResourceRegistryBaselineV1 {
    pub stream: ResourceRegistryStreamIdV1,
    pub generation: ResourceRegistryGenerationV1,
    pub projection: Arc<ResourceRegistryProjectionV1>,
}
pub enum ResourceRegistryDeliveryV1 {
    Projection(ResourceRegistryEventV1),
    Empty,
    Gap,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceRegistrySampleV1 {
    Unchanged,
    Queued,
    Gap,
}
pub struct ResourceRegistrySnapshotRowFactsV1<'a> {
    pub address: &'a ResourceCatalogAddressV1,
    pub captured_visible: bool,
    pub current_visible: bool,
    pub captured_descriptor: bool,
    pub current_descriptor: bool,
    pub visible_order: Option<usize>,
}
pub struct ResourceRegistryHistoricalRowFactsV1<'a> {
    pub reference: &'a ResourceRegistrySlotReferenceV1,
    pub membership_allowed: bool,
    pub descriptor_allowed: bool,
}
pub struct TrustedResourceRegistryObservationFactsV1<'a> {
    pub scope: &'a ResourceRegistryObservationScopeV1,
    pub native_profile: ResourceCatalogLimitsV1,
    pub authorized: Option<bool>,
    pub scope_current: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub projection_allowed: Option<bool>,
    pub slot_identity_allowed: Option<bool>,
    pub retained: Option<bool>,
    pub resources_ready: Option<bool>,
    pub reply_ready: Option<bool>,
    pub transfer_ready: Option<bool>,
}
pub struct TrustedResourceRegistrySnapshotFactsV1<'a> {
    pub current: &'a TrustedResourceRegistryObservationFactsV1<'a>,
    pub rows: &'a [ResourceRegistrySnapshotRowFactsV1<'a>],
    pub capture_policy_current: Option<bool>,
    pub visibility_complete: Option<bool>,
    pub order_attested: Option<bool>,
}
pub struct TrustedResourceRegistryHistoricalFactsV1<'a> {
    pub current: &'a TrustedResourceRegistryObservationFactsV1<'a>,
    pub rows: &'a [ResourceRegistryHistoricalRowFactsV1<'a>],
    pub historical_release_attested: Option<bool>,
}
pub struct ResourceRegistryObservationControlV1<'a> {
    pub scope: &'a ResourceRegistryObservationScopeV1,
    pub owner_control_allowed: Option<bool>,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResourceRegistryObservationUsageV1 {
    pub streams: usize,
    pub preparations: usize,
    pub queued: usize,
    pub bindings: usize,
    pub gapped: usize,
    pub stopped: usize,
    pub issued_lifetime: usize,
    pub reserved_bytes: usize,
    pub closed: bool,
}
macro_rules! redacted {($($t:ty),+)=>{$(impl std::fmt::Debug for $t {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {f.write_str("<registry-observation:redacted>")}
})+};}
redacted!(
    ResourceRegistryObservationScopeV1,
    ResourceRegistryStreamIdV1,
    ResourceRegistryPreparationIdV1,
    ResourceRegistryGenerationV1,
    ResourceRegistrySlotReferenceV1,
    ResourceRegistryHeadIdV1,
    ResourceRegistryObservationRowV1,
    ResourceRegistryProjectionV1,
    ResourceRegistryEventV1,
    ResourceRegistryBaselineV1,
    ResourceRegistryDeliveryV1,
    ResourceRegistryHistoricalHeadV1
);
mod commands;
pub use commands::{
    NormalizedResourceRegistryCommandArgumentsV1, ResourceRegistryCommandEnvelopeValueV1,
    ResourceRegistryCommandPortV1, normalize_resource_registry_command_envelope_v1,
};
mod receipts;
#[cfg(test)]
pub(in crate::coordinator) use receipts::tests::{
    certificate as registry_receipt_test_certificate, receipt_facts as registry_receipt_test_facts,
    setup as registry_receipt_test_setup,
};
pub use receipts::{
    NormalizedResourceRegistryReceiptArgumentsV1, RESOURCE_REGISTRY_RECEIPT_PROFILE_V1,
    ResourceRegistryReceiptAuthorizationBasisIdV1, ResourceRegistryReceiptAuthorizationBasisV1,
    ResourceRegistryReceiptCeilingV1, ResourceRegistryReceiptContextIdV1,
    ResourceRegistryReceiptEnvelopeValueV1, ResourceRegistryReceiptPortV1,
    ResourceRegistryReceiptReplyV1, ResourceRegistryReceiptUsageV1,
    TrustedResourceRegistryReceiptFactsV1, TrustedResourceRegistryReceiptHistoricalFactsV1,
    normalize_resource_registry_receipt_envelope_v1,
};
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Active,
    Gap,
    Stopped,
}
struct SnapshotRow {
    address: ResourceCatalogAddressV1,
    descriptor: Option<ResourceCatalogCapabilitiesV1>,
    identity_known: Option<bool>,
}
struct Binding {
    address: ResourceCatalogAddressV1,
    reference: ResourceRegistrySlotReferenceV1,
}
struct Pending {
    marker: Arc<()>,
    event: ResourceRegistryEventV1,
}
struct Observer {
    scope: ResourceRegistryObservationScopeV1,
    generation: ResourceRegistryGenerationV1,
    baseline: Option<Arc<ResourceRegistryProjectionV1>>,
    bindings: Vec<Binding>,
    queue: VecDeque<Pending>,
    phase: Phase,
}
enum PacketValue {
    Snapshot(Vec<SnapshotRow>),
    Historical { head: Arc<()>, allowed: bool },
    Command(Box<commands::Record>),
    ReceiptContext(Box<receipts::Context>),
    ReceiptCertificate(receipts::Certificate),
}
#[derive(Clone, PartialEq, Eq)]
enum PacketTarget {
    Establish,
    Stream(ResourceRegistryStreamIdV1),
}
struct Packet {
    stream: PacketTarget,
    scope: ResourceRegistryObservationScopeV1,
    root: RootStamp,
    command_basis: Option<root_identity::NativeRootIdentity>,
    value: PacketValue,
}
pub(in crate::coordinator) struct Pool {
    owner: Arc<()>,
    authority: String,
    engine: String,
    native: ResourceCatalogLimitsV1,
    limits: ResourceRegistryObservationLimitsV1,
    streams: Vec<Option<Observer>>,
    packets: Vec<Option<Packet>>,
    issued: usize,
    reservation: retention::Reservation,
    charge: usize,
    #[cfg(test)]
    command_fault: Option<commands::FaultCut>,
}
#[derive(Default)]
pub(in crate::coordinator) enum State {
    #[default]
    Unconfigured,
    Active(Box<Pool>),
    Closed {
        issued: usize,
    },
}
impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RegistryObservationState(<redacted>)")
    }
}
fn text(v: &str) -> Result<String> {
    let mut s = String::new();
    s.try_reserve_exact(v.len())
        .map_err(|_| ResourceError::Capacity)?;
    s.push_str(v);
    Ok(s)
}
fn parent(e: RetentionErrorV1) -> ResourceError {
    match e {
        RetentionErrorV1::Limit => ResourceError::Limit,
        RetentionErrorV1::Unavailable => ResourceError::Unavailable,
        RetentionErrorV1::Faulted => ResourceError::Faulted,
    }
}
fn report<T>(
    r: &mut Recorder,
    boundary: Boundary,
    result: &Result<T>,
    service_time: Option<std::time::Duration>,
) {
    r.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(ResourceError::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time,
    });
}
impl TrustedResourceRegistryObservationFactsV1<'_> {
    fn required(&self) -> [Option<bool>; 8] {
        [
            self.authorized,
            self.scope_current,
            self.continuous,
            self.live,
            self.not_stopped,
            self.projection_allowed,
            self.slot_identity_allowed,
            self.retained,
        ]
    }
    fn ready(&self) -> bool {
        [self.resources_ready, self.reply_ready, self.transfer_ready]
            .into_iter()
            .all(|v| v == Some(true))
    }
}
impl Observer {
    fn close(&mut self, phase: Phase) {
        self.phase = phase;
        self.queue.clear();
        self.baseline = None;
        if phase == Phase::Stopped {
            self.bindings.clear();
        }
    }
}
