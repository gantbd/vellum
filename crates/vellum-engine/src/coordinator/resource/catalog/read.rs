//! Recipient-owned metadata frontiers. No provider ports, I/O or mutation grants.
use super::*;
use crate::coordinator::root_identity::NativeRootIdentity;
use std::panic::{AssertUnwindSafe, catch_unwind};
mod controls;
mod ingress;
pub use ingress::{
    NormalizedCatalogReadArgumentsV1, ResourceCatalogReadDecoderUsageV1,
    ResourceCatalogReadEnvelopeValueV1, normalize_resource_catalog_read_envelope_v1,
    resource_catalog_read_classification_bytes_v1,
};
mod projection;
mod query;
mod shutdown_inventory;
#[cfg(test)]
pub(in crate::coordinator) mod tests;
pub const RESOURCE_CATALOG_READ_PROFILE_V1: &str = "scoped-resource-catalog/1";
#[derive(Clone, PartialEq, Eq)]
pub struct ResourceCatalogReadScopeV1 {
    pub context: SettingsExecutionContextV1,
    pub policy: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceCatalogReadLimitsV1 {
    pub max_contexts: usize,
    pub max_preparations: usize,
    pub max_rows: usize,
    pub max_selectors: usize,
    pub max_issued: usize,
    pub max_page_rows: usize,
    pub max_root_bytes: usize,
    pub max_packet_bytes: usize,
    pub max_scope_bytes: usize,
    pub max_capture_bytes: usize,
    pub max_row_bytes: usize,
    pub max_selector_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_pool_bytes: usize,
}
impl ResourceCatalogReadLimitsV1 {
    /// Conservative native envelopes and table headers; not JSON bytes or RSS.
    pub fn required_pool_bytes(self) -> Result<usize> {
        let mul = |a: usize, b: usize| a.checked_mul(b).ok_or(ResourceError::Limit);
        let context = [
            self.max_scope_bytes,
            mul(2, self.max_capture_bytes)?,
            mul(2, self.max_reply_bytes)?,
        ]
        .into_iter()
        .try_fold(0, add)?;
        let n = [
            size_of::<Pool>(),
            self.max_scope_bytes,
            mul(self.max_preparations, self.max_packet_bytes)?,
            mul(self.max_contexts, context)?,
            mul(mul(2, self.max_selectors)?, self.max_selector_bytes)?,
            mul(self.max_preparations, size_of::<Option<Packet>>())?,
            mul(self.max_issued, size_of::<Option<Context>>())?,
        ]
        .into_iter()
        .try_fold(0, add)?;
        (n <= self.max_pool_bytes)
            .then_some(n)
            .ok_or(ResourceError::Limit)
    }
}
/// Complete host-attested slot coverage. Raw addresses stay at the trusted edge.
pub struct ResourceCatalogVisibilityV1<'a> {
    pub address: &'a ResourceCatalogAddressV1,
    pub visible: bool,
    pub descriptor: bool,
    pub order: Option<usize>,
}
#[derive(Clone, Copy, Default)]
pub struct TrustedResourceCatalogReadFactsV1 {
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub resources_ready: Option<bool>,
    pub preparation_reserved: Option<bool>,
    pub visibility_attested: Option<bool>,
    pub visibility_complete: Option<bool>,
    pub projection_order_attested: Option<bool>,
    pub registry_coverage_complete: Option<bool>,
    pub retention_ready: Option<bool>,
    pub transfer_ready: Option<bool>,
    pub registry_ready: Option<bool>,
    pub retained: Option<bool>,
    pub reply_ready: Option<bool>,
    pub owner_control_allowed: Option<bool>,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
}
impl TrustedResourceCatalogReadFactsV1 {
    fn common(&self) -> bool {
        [
            self.authorized,
            self.scope,
            self.continuous,
            self.live,
            self.not_stopped,
            self.resources_ready,
        ]
        .into_iter()
        .all(|v| v == Some(true))
    }
    fn preparation(&self) -> bool {
        self.common()
            && [
                self.preparation_reserved,
                self.visibility_attested,
                self.visibility_complete,
                self.projection_order_attested,
                self.registry_coverage_complete,
            ]
            .into_iter()
            .all(|v| v == Some(true))
    }
    fn acquisition(&self) -> bool {
        self.common()
            && [
                self.retention_ready,
                self.transfer_ready,
                self.registry_ready,
            ]
            .into_iter()
            .all(|v| v == Some(true))
    }
    fn release(&self) -> bool {
        self.common()
            && [self.retained, self.transfer_ready, self.reply_ready]
                .into_iter()
                .all(|v| v == Some(true))
    }
    fn loss(&self) -> bool {
        [
            self.authorized,
            self.scope,
            self.continuous,
            self.live,
            self.retained,
        ]
        .into_iter()
        .any(|v| v == Some(false))
    }
    fn discharge(&self) -> bool {
        [
            self.owner_control_allowed,
            self.retention_released,
            self.delivery_drained,
        ]
        .into_iter()
        .all(|v| v == Some(true))
    }
}
macro_rules! handle {
    ($name:ident) => {
        #[derive(Clone)]
        pub struct $name {
            owner: Arc<()>,
            slot: usize,
        }
        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
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
handle!(ResourceCatalogReadContextIdV1);
handle!(ResourceCatalogReadEntryIdV1);
handle!(ResourceCatalogReadContinuationIdV1);
#[derive(Clone)]
pub struct ResourceCatalogVisibilityIdV1 {
    owner: Arc<()>,
    slot: usize,
    marker: Arc<()>,
}
impl std::fmt::Debug for ResourceCatalogVisibilityIdV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResourceCatalogVisibilityIdV1(<opaque>)")
    }
}
#[derive(PartialEq, Eq)]
pub enum ResourceCatalogReadDescriptorV1 {
    StrictUtf8OneBom {
        atomic_read: bool,
        insertion_line_break: CatalogInsertionLineBreakV1,
    },
    AnnotationsV1 {
        atomic_read: bool,
    },
    Write {
        procedure: String,
        identity_known: bool,
        logical_continuity: bool,
        synchronization_supported: bool,
        conditional_write: bool,
        causal_receipts: bool,
        non_reuse_tokens: bool,
        exclusive_create: bool,
    },
}
#[derive(Clone, Copy)]
pub enum ResourceCatalogReadQueryV1<'a> {
    List {
        limit: usize,
        continuation: Option<&'a ResourceCatalogReadContinuationIdV1>,
    },
    Inspect {
        entry: &'a ResourceCatalogReadEntryIdV1,
    },
}
#[derive(Debug)]
pub struct ResourceCatalogReadPageEntryV1 {
    pub reference: ResourceCatalogReadEntryIdV1,
    pub table: ResourceCatalogTableV1,
}
pub enum ResourceCatalogReadReplyV1 {
    Page {
        context: ResourceCatalogReadContextIdV1,
        entries: Vec<ResourceCatalogReadPageEntryV1>,
        continuation: Option<ResourceCatalogReadContinuationIdV1>,
    },
    Registration {
        context: ResourceCatalogReadContextIdV1,
        entry: ResourceCatalogReadEntryIdV1,
        table: ResourceCatalogTableV1,
        descriptor: Option<ResourceCatalogReadDescriptorV1>,
    },
}
impl ResourceCatalogReadReplyV1 {
    pub const fn coverage(&self) -> &'static str {
        "captured-permitted"
    }
}
macro_rules! redacted {
    ($($ty:ty),+ $(,)?) => { $(impl std::fmt::Debug for $ty {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("<catalog-read:redacted>") }
    })+ };
}
redacted!(
    ResourceCatalogReadScopeV1,
    ResourceCatalogReadDescriptorV1,
    ResourceCatalogReadReplyV1,
    ResourceCatalogReadQueryV1<'_>,
    ResourceCatalogVisibilityV1<'_>
);
struct Snapshot {
    address: ResourceCatalogAddressV1,
    descriptor: Option<ResourceCatalogReadDescriptorV1>,
}
struct Packet {
    marker: Arc<()>,
    root: Arc<Root>,
    stamp: NativeRootIdentity,
    scope: ResourceCatalogReadScopeV1,
    rows: Vec<Snapshot>,
}
struct Cursor {
    limit: usize,
    start: usize,
}
struct Context {
    scope: ResourceCatalogReadScopeV1,
    marker: Arc<()>,
    rows: Vec<Snapshot>,
    entries: Vec<bool>,
    cursors: Vec<Cursor>,
    stopped: bool,
    invalidated: bool,
}
pub(in crate::coordinator) struct Pool {
    limits: ResourceCatalogReadLimitsV1,
    authority: String,
    engine: String,
    reservation: retention::Reservation,
    charge: usize,
    issued: usize,
    packets: Vec<Option<Packet>>,
    contexts: Vec<Option<Context>>,
    live_packets: usize,
    live_contexts: usize,
    live_selectors: usize,
    decoding: bool,
    decode_charge: usize,
    decoder_faulted: bool,
    #[cfg(test)]
    decode_probe: Option<ingress::Probe>,
    #[cfg(test)]
    decode_probe_report: Option<(bool, bool, bool)>,
    #[cfg(test)]
    query_fault: Option<usize>,
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
        f.write_str("CatalogReadOwner(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResourceCatalogReadUsageV1 {
    pub preparations: usize,
    pub contexts: usize,
    pub selectors: usize,
    pub issued_lifetime: usize,
    pub reserved_pool_bytes: usize,
    pub closed: bool,
}
fn parent(error: RetentionErrorV1) -> ResourceError {
    match error {
        RetentionErrorV1::Unavailable => ResourceError::Unavailable,
        RetentionErrorV1::Limit => ResourceError::Limit,
        RetentionErrorV1::Faulted => ResourceError::Faulted,
    }
}
fn admission(error: CoordinatorError) -> ResourceError {
    match error {
        CoordinatorError::Unavailable => ResourceError::Unavailable,
        _ => ResourceError::Faulted,
    }
}
fn reserve<T>(v: &mut Vec<T>, n: usize) -> Result<()> {
    v.try_reserve_exact(n).map_err(|_| ResourceError::Capacity)
}
fn text(s: &str) -> Result<String> {
    let mut out = String::new();
    out.try_reserve_exact(s.len())
        .map_err(|_| ResourceError::Capacity)?;
    out.push_str(s);
    Ok(out)
}
fn scope_bytes(s: &ResourceCatalogReadScopeV1, l: ResourceCatalogReadLimitsV1) -> Result<usize> {
    if s.context.profile != RESOURCE_CATALOG_READ_PROFILE_V1 {
        return Err(ResourceError::Unavailable);
    }
    settings::basis::labels(&s.context)
        .into_iter()
        .chain([s.policy.as_str()])
        .try_fold(size_of::<ResourceCatalogReadScopeV1>(), |n, v| {
            if v.is_empty() {
                return Err(ResourceError::Malformed);
            }
            add(n, v.len()).and_then(|n| {
                (n <= l.max_scope_bytes)
                    .then_some(n)
                    .ok_or(ResourceError::Limit)
            })
        })
}
fn copy_scope(s: &ResourceCatalogReadScopeV1) -> Result<ResourceCatalogReadScopeV1> {
    Ok(ResourceCatalogReadScopeV1 {
        context: settings::basis::copy_context(&s.context).map_err(|_| ResourceError::Capacity)?,
        policy: text(&s.policy)?,
    })
}
impl Pool {
    fn ready(&self) -> Result<()> {
        if self.decoder_faulted {
            Err(ResourceError::Unavailable)
        } else {
            Ok(())
        }
    }
    fn scope(&self, s: &ResourceCatalogReadScopeV1) -> Result<usize> {
        let bytes = scope_bytes(s, self.limits)?;
        if s.context.authority != self.authority || s.context.engine != self.engine {
            return Err(ResourceError::Unavailable);
        }
        Ok(bytes)
    }
    fn end(&mut self, slot: usize, invalidated: bool) {
        if let Some(Some(c)) = self.contexts.get_mut(slot) {
            self.live_selectors -= c.entries.iter().filter(|v| **v).count() + c.cursors.len();
            c.stopped = true;
            c.invalidated |= invalidated;
            c.rows.clear();
            c.entries.clear();
            c.cursors.clear();
        }
    }
}
fn record<T>(
    r: &mut Recorder,
    boundary: Boundary,
    result: &Result<T>,
    time: Option<std::time::Duration>,
) {
    r.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => crate::instrumentation::Disposition::Completed,
            Err(ResourceError::Faulted) => crate::instrumentation::Disposition::Unknown,
            _ => crate::instrumentation::Disposition::Refused,
        },
        service_time: time,
    });
}
impl SourceCoordinator {
    fn catalog_read_pool(&self) -> Result<&Pool> {
        let State::Active(p) = &self.catalog_reads else {
            return Err(ResourceError::Unavailable);
        };
        self.retention
            .validate_exact(&p.reservation, p.charge)
            .map_err(parent)?;
        Ok(p)
    }
    fn catalog_read_pool_mut(&mut self) -> Result<&mut Pool> {
        self.catalog_read_pool()?;
        let State::Active(p) = &mut self.catalog_reads else {
            return Err(ResourceError::Faulted);
        };
        Ok(p)
    }
    pub fn configure_resource_catalog_reader_v1(
        &mut self,
        scope: &ResourceCatalogReadScopeV1,
        limits: ResourceCatalogReadLimitsV1,
        facts: &TrustedResourceCatalogReadFactsV1,
        r: &mut Recorder,
    ) -> Result<bool> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(admission)?;
            scope_bytes(scope, limits)?;
            if [
                facts.owner_control_allowed,
                facts.resources_ready,
                facts.preparation_reserved,
            ]
            .into_iter()
            .any(|v| v != Some(true))
            {
                return Err(ResourceError::Unavailable);
            }
            match &self.catalog_reads {
                State::Closed { .. } => return Err(ResourceError::Unavailable),
                State::Active(p) => {
                    self.catalog_read_pool()?;
                    return if p.limits == limits
                        && p.authority == scope.context.authority
                        && p.engine == scope.context.engine
                    {
                        Ok(false)
                    } else {
                        Err(ResourceError::Unavailable)
                    };
                }
                State::Unconfigured => {}
            }
            self.resources.catalog.current()?;
            let charge = limits.required_pool_bytes()?;
            self.retention.check(charge).map_err(parent)?;
            let authority = text(&scope.context.authority)?;
            let engine = text(&scope.context.engine)?;
            let mut packets = Vec::new();
            reserve(&mut packets, limits.max_preparations)?;
            let mut contexts = Vec::new();
            reserve(&mut contexts, limits.max_issued)?;
            let reservation = self.retention.reserve(charge).map_err(parent)?;
            self.catalog_reads = State::Active(Box::new(Pool {
                limits,
                authority,
                engine,
                reservation,
                charge,
                issued: 0,
                packets,
                contexts,
                live_packets: 0,
                live_contexts: 0,
                live_selectors: 0,
                decoding: false,
                decode_charge: 0,
                decoder_faulted: false,
                #[cfg(test)]
                decode_probe: None,
                #[cfg(test)]
                decode_probe_report: None,
                #[cfg(test)]
                query_fault: None,
            }));
            Ok(true)
        })();
        record(
            r,
            Boundary::ResourceCatalogReadConfiguration,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn prepare_resource_catalog_visibility_v1(
        &mut self,
        scope: &ResourceCatalogReadScopeV1,
        visibility: &[ResourceCatalogVisibilityV1<'_>],
        facts: &TrustedResourceCatalogReadFactsV1,
        r: &mut Recorder,
    ) -> Result<ResourceCatalogVisibilityIdV1> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(admission)?;
            if !facts.preparation() {
                return Err(ResourceError::Unavailable);
            }
            let p = self.catalog_read_pool()?;
            p.ready()?;
            if p.decoding {
                return Err(ResourceError::Unavailable);
            }
            let scope_size = p.scope(scope)?;
            if p.live_packets >= p.limits.max_preparations {
                return Err(ResourceError::Limit);
            }
            let (root, _) = self.resources.catalog.current()?;
            if self
                .measure_semantic_root()
                .map_err(|_| ResourceError::Faulted)?
                .total_bytes
                > p.limits.max_root_bytes
            {
                return Err(ResourceError::Limit);
            }
            let rows = projection::project(root, visibility, p.limits)?;
            let bytes = [
                size_of::<Packet>(),
                rows_bytes(&root.rows)?,
                scope_size,
                projection::bytes(&rows)?,
            ]
            .into_iter()
            .try_fold(0, add)?;
            if bytes > p.limits.max_packet_bytes {
                return Err(ResourceError::Limit);
            }
            let packet = Packet {
                marker: Arc::new(()),
                root: root.clone(),
                stamp: self
                    .native_root_identity()
                    .map_err(|_| ResourceError::Faulted)?,
                scope: copy_scope(scope)?,
                rows,
            };
            let owner = self.namespace.clone();
            let p = self.catalog_read_pool_mut()?;
            let slot = p
                .packets
                .iter()
                .position(Option::is_none)
                .unwrap_or(p.packets.len());
            let id = ResourceCatalogVisibilityIdV1 {
                owner,
                slot,
                marker: packet.marker.clone(),
            };
            if slot == p.packets.len() {
                p.packets.push(Some(packet));
            } else {
                p.packets[slot] = Some(packet);
            }
            p.live_packets += 1;
            Ok(id)
        })();
        record(
            r,
            Boundary::ResourceCatalogReadPreparation,
            &result,
            timing.finish(),
        );
        result
    }
    fn take_catalog_visibility(
        &mut self,
        id: &ResourceCatalogVisibilityIdV1,
        scope: &ResourceCatalogReadScopeV1,
    ) -> Result<Packet> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return Err(ResourceError::Unavailable);
        }
        let p = self.catalog_read_pool()?;
        p.scope(scope)?;
        let packet = p
            .packets
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(ResourceError::Unavailable)?;
        if !Arc::ptr_eq(&id.marker, &packet.marker)
            || packet.scope != *scope
            || self
                .native_root_identity()
                .map_err(|_| ResourceError::Faulted)?
                != packet.stamp
            || !Arc::ptr_eq(&packet.root, self.resources.catalog.current()?.0)
        {
            return Err(ResourceError::Unavailable);
        }
        let p = self.catalog_read_pool_mut()?;
        let packet = p.packets[id.slot].take().ok_or(ResourceError::Faulted)?;
        p.live_packets -= 1;
        Ok(packet)
    }
    pub fn acquire_resource_catalog_read_v1(
        &mut self,
        scope: &ResourceCatalogReadScopeV1,
        cut: &ResourceCatalogVisibilityIdV1,
        facts: &TrustedResourceCatalogReadFactsV1,
        r: &mut Recorder,
    ) -> Result<ResourceCatalogReadContextIdV1> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(admission)?;
            if !facts.acquisition() {
                return Err(ResourceError::Unavailable);
            }
            let p = self.catalog_read_pool()?;
            p.ready()?;
            if p.decoding {
                return Err(ResourceError::Unavailable);
            }
            p.scope(scope)?;
            if p.live_contexts >= p.limits.max_contexts || p.issued >= p.limits.max_issued {
                return Err(ResourceError::Limit);
            }
            let l = p.limits;
            let packet = self.take_catalog_visibility(cut, scope)?;
            if add(
                add(size_of::<Context>(), projection::bytes(&packet.rows)?)?,
                packet.rows.len(),
            )? > l.max_capture_bytes
                || size_of::<ResourceCatalogReadContextIdV1>() > l.max_reply_bytes
            {
                return Err(ResourceError::Limit);
            }
            let mut entries = Vec::new();
            reserve(&mut entries, packet.rows.len())?;
            entries.resize(packet.rows.len(), false);
            let owner = self.namespace.clone();
            let p = self.catalog_read_pool_mut()?;
            let id = ResourceCatalogReadContextIdV1 {
                owner,
                slot: p.contexts.len(),
            };
            p.contexts.push(Some(Context {
                scope: packet.scope,
                marker: Arc::new(()),
                rows: packet.rows,
                entries,
                cursors: Vec::new(),
                stopped: false,
                invalidated: false,
            }));
            p.issued += 1;
            p.live_contexts += 1;
            Ok(id)
        })();
        record(
            r,
            Boundary::ResourceCatalogReadAcquisition,
            &result,
            timing.finish(),
        );
        result
    }
}
