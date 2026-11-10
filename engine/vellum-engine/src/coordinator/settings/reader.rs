//! Funded recipient settings reads. Trusted host facts are not request assertions.
use super::*;
use crate::settings::{
    NativeSettingsProjectionV1, SettingProjectionDeclarationV1, SettingProjectionGrantV1,
    SettingProjectionOriginV1, SettingProjectionOriginsV1, SettingProjectionValueV1,
    SettingsProjectionLimitsV1, project_settings_v1,
};
use std::mem::size_of;
mod controls;
mod ingress;
pub use ingress::{
    NormalizedSettingsReadArgumentsV1, SettingsReadDecoderUsageV1, SettingsReadEnvelopeValueV1,
    SettingsReadReplyV1, normalize_settings_read_envelope_v1,
    settings_read_decode_classification_bytes_v1,
};
mod release;
mod shutdown_inventory;
#[cfg(test)]
pub(in crate::coordinator) mod tests;

pub const SETTINGS_READ_PROFILE_V1: &str = "native-engine-settings-read/1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsReaderLimitsV1 {
    pub max_issued: usize,
    pub max_packets: usize,
    pub max_contexts: usize,
    pub max_origins: usize,
    pub max_packet_bytes: usize,
    pub max_capture_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_context_bytes: usize,
    pub max_label_bytes: usize,
    /// Per-phase logical allowance for projection, origin interning and release
    /// validation/comparison. Not a CPU instruction count or a latency promise.
    pub max_work: usize,
    pub max_depth: usize,
    pub max_pool_bytes: usize,
}
impl SettingsReaderLimitsV1 {
    /// Worst-case logical funding, including tombstones and synchronous workspace.
    /// Not allocator capacity, physical memory, or an authorization grant.
    pub fn required_pool_bytes(self) -> Result<usize, SettingsError> {
        let add = |a: usize, b: usize| a.checked_add(b).ok_or(SettingsError::Limit);
        let mul = |a: usize, b: usize| a.checked_mul(b).ok_or(SettingsError::Limit);
        let capture = add(mul(2, self.max_capture_bytes)?, self.max_context_bytes)?;
        let mut n = add(
            size_of::<Pool>(),
            mul(self.max_issued, size_of::<Option<Row>>())?,
        )?;
        n = add(n, mul(self.max_packets, self.max_packet_bytes)?)?;
        n = add(n, mul(self.max_contexts, capture)?)?;
        n = add(n, mul(2, self.max_capture_bytes)?)?;
        n = add(n, self.max_reply_bytes)?;
        n = add(n, mul(2, self.max_label_bytes)?)?;
        (n <= self.max_pool_bytes)
            .then_some(n)
            .ok_or(SettingsError::Limit)
    }
    fn projection(self) -> SettingsProjectionLimitsV1 {
        SettingsProjectionLimitsV1 {
            max_bytes: self.max_capture_bytes,
            max_work: self.max_work,
            max_depth: self.max_depth,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SettingsReadScopeV1 {
    pub execution: SettingsExecutionContextV1,
    pub policy: String,
}

/// These guards come from the authenticated embedding, independently of arguments.
/// Unknown facts refuse; only recognized loss invalidates an existing context.
#[derive(Clone, Copy)]
pub struct TrustedSettingsReadFactsV1 {
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub retained: Option<bool>,
    pub outside_publication: Option<bool>,
    pub resources_ready: Option<bool>,
    pub transfer_ready: Option<bool>,
    pub reply_ready: Option<bool>,
}
impl TrustedSettingsReadFactsV1 {
    fn ready(&self) -> bool {
        [
            self.authorized,
            self.scope,
            self.continuous,
            self.live,
            self.not_stopped,
            self.retained,
            self.outside_publication,
            self.resources_ready,
            self.transfer_ready,
            self.reply_ready,
        ]
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
}
pub struct TrustedSettingsReadPreparationV1<'a> {
    pub read: &'a TrustedSettingsReadFactsV1,
    pub owner_control_allowed: Option<bool>,
    pub preparation_reserved: Option<bool>,
    pub visibility_attested: Option<bool>,
    pub visibility_complete: Option<bool>,
    pub target_readable: Option<bool>,
    pub keys: &'a [SettingProjectionGrantV1<'a>],
    pub capabilities: bool,
}
pub struct SettingsReadControlV1<'a> {
    pub scope: &'a SettingsReadScopeV1,
    pub owner_control_allowed: Option<bool>,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
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
handle!(SettingsReadCutIdV1);
handle!(SettingsReadContextIdV1);
handle!(SettingsReadTargetRefV1);
handle!(SettingsReadOriginRefV1);

#[derive(Debug)]
pub struct SettingsReadAcquisitionV1 {
    pub context: SettingsReadContextIdV1,
    pub target: SettingsReadTargetRefV1,
}
#[derive(Clone, Copy)]
pub enum SettingsReadOperationV1<'a> {
    EffectiveConfiguration { key: &'a str },
    PermittedOrigin { key: &'a str },
    Capabilities,
}
impl std::fmt::Debug for SettingsReadOperationV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SettingsReadOperationV1(<redacted>)")
    }
}
pub enum SettingsReadOriginsV1 {
    Whole(Vec<SettingsReadOriginRefV1>),
    Fields(Vec<(String, Vec<SettingsReadOriginRefV1>)>),
}
impl SettingsReadOriginsV1 {
    pub const fn coverage(&self) -> &'static str {
        "permitted-only"
    }
}
pub enum SettingsReadResultV1 {
    Value(SettingProjectionValueV1),
    Origins(SettingsReadOriginsV1),
    Capabilities(Vec<SettingProjectionDeclarationV1>),
}
impl SettingsReadResultV1 {
    pub const fn coverage(&self) -> &'static str {
        "permitted-only"
    }
}

struct Cut {
    scope: SettingsReadScopeV1,
    target: SettingsTargetId,
    root: Arc<CompiledSettingsV1>,
    data: NativeSettingsProjectionV1,
}
struct Context {
    scope: SettingsReadScopeV1,
    target: SettingsTargetId,
    marker: Arc<()>,
    data: Option<NativeSettingsProjectionV1>,
    origins: Vec<SettingProjectionOriginV1>,
}
enum Row {
    Cut(Cut),
    Context(Context),
}
pub(in crate::coordinator) struct Pool {
    limits: SettingsReaderLimitsV1,
    authority: String,
    engine: String,
    reservation: retention::Reservation,
    charge: usize,
    issued: usize,
    rows: Vec<Option<Row>>,
    packets: usize,
    contexts: usize,
    ended_contexts: usize,
    decoding: bool,
    decode_charge: usize,
    decoder_faulted: bool,
    #[cfg(test)]
    decode_probe: Option<ingress::Probe>,
    #[cfg(test)]
    decode_probe_report: Option<(bool, bool, bool)>,
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SettingsReaderUsageV1 {
    pub issued_lifetime: usize,
    pub packets: usize,
    pub contexts: usize,
    pub ended_contexts: usize,
    pub reserved_pool_bytes: usize,
    pub closed: bool,
}
impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SettingsReaderState(<redacted>)")
    }
}
impl std::fmt::Debug for SettingsReadScopeV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SettingsReadScopeV1(<redacted>)")
    }
}
impl std::fmt::Debug for SettingsReadResultV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SettingsReadResultV1(<redacted>)")
    }
}

fn string(s: &str) -> Result<String, SettingsError> {
    let mut out = String::new();
    out.try_reserve_exact(s.len())
        .map_err(|_| SettingsError::Capacity)?;
    out.push_str(s);
    Ok(out)
}
fn reserve<T>(v: &mut Vec<T>, n: usize) -> Result<(), SettingsError> {
    v.try_reserve_exact(n).map_err(|_| SettingsError::Capacity)
}
fn scope_bytes(s: &SettingsReadScopeV1, l: SettingsReaderLimitsV1) -> Result<usize, SettingsError> {
    let mut n = size_of::<SettingsReadScopeV1>();
    for label in basis::labels(&s.execution)
        .into_iter()
        .chain([s.policy.as_str()])
    {
        if label.is_empty() || label.len() > l.max_label_bytes {
            return Err(SettingsError::Unavailable);
        }
        n = n.checked_add(label.len()).ok_or(SettingsError::Limit)?;
    }
    (n <= l.max_context_bytes)
        .then_some(n)
        .ok_or(SettingsError::Limit)
}
fn copy_scope(s: &SettingsReadScopeV1) -> Result<SettingsReadScopeV1, SettingsError> {
    Ok(SettingsReadScopeV1 {
        execution: basis::copy_context(&s.execution)?,
        policy: string(&s.policy)?,
    })
}
impl Pool {
    fn delivery_ready(&self) -> Result<(), SettingsError> {
        if self.decoder_faulted {
            Err(SettingsError::Unavailable)
        } else {
            Ok(())
        }
    }
    fn scope(&self, s: &SettingsReadScopeV1) -> Result<usize, SettingsError> {
        let n = scope_bytes(s, self.limits)?;
        if s.execution.authority != self.authority
            || s.execution.engine != self.engine
            || s.execution.profile != SETTINGS_READ_PROFILE_V1
        {
            return Err(SettingsError::Unavailable);
        }
        Ok(n)
    }
    fn counts(&self) -> (usize, usize) {
        (self.packets, self.contexts)
    }
    fn context(&self, id: &SettingsReadContextIdV1) -> Result<&Context, SettingsError> {
        match self.rows.get(id.slot).and_then(Option::as_ref) {
            Some(Row::Context(c)) => Ok(c),
            _ => Err(SettingsError::Unavailable),
        }
    }
    fn end(&mut self, id: &SettingsReadContextIdV1) {
        if let Some(Some(Row::Context(c))) = self.rows.get_mut(id.slot) {
            if c.data.is_some() {
                self.ended_contexts += 1;
            }
            c.data = None;
            c.origins.clear();
        }
    }
}

impl SourceCoordinator {
    fn settings_reader_pool(&self) -> Result<&Pool, SettingsError> {
        let State::Active(pool) = &self.settings_reader else {
            return Err(SettingsError::Unavailable);
        };
        self.retention
            .validate_exact(&pool.reservation, pool.charge)
            .map_err(retained::parent_error)?;
        Ok(pool)
    }
    fn settings_reader_mut(&mut self) -> Result<&mut Pool, SettingsError> {
        self.settings_reader_pool()?;
        let State::Active(pool) = &mut self.settings_reader else {
            return Err(SettingsError::Faulted);
        };
        Ok(pool)
    }
    pub fn configure_settings_reader_v1(
        &mut self,
        authority: &str,
        engine: &str,
        limits: SettingsReaderLimitsV1,
        r: &mut Recorder,
    ) -> Result<bool, SettingsError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(admission_error)?;
            match &self.settings_reader {
                State::Closed { .. } => return Err(SettingsError::Unavailable),
                State::Active(p) => {
                    self.settings_reader_pool()?;
                    return if p.limits == limits && p.authority == authority && p.engine == engine {
                        Ok(false)
                    } else {
                        Err(SettingsError::Unavailable)
                    };
                }
                State::Unconfigured => {}
            }
            if authority.is_empty()
                || engine.is_empty()
                || authority.len() > limits.max_label_bytes
                || engine.len() > limits.max_label_bytes
            {
                return Err(SettingsError::Unavailable);
            }
            let charge = limits.required_pool_bytes()?;
            self.retention
                .check(charge)
                .map_err(retained::parent_error)?;
            let authority = string(authority)?;
            let engine = string(engine)?;
            let mut rows = Vec::new();
            reserve(&mut rows, limits.max_issued)?;
            let reservation = self
                .retention
                .reserve(charge)
                .map_err(retained::parent_error)?;
            self.settings_reader = State::Active(Box::new(Pool {
                limits,
                authority,
                engine,
                reservation,
                charge,
                issued: 0,
                rows,
                packets: 0,
                contexts: 0,
                ended_contexts: 0,
                decoding: false,
                decode_charge: 0,
                decoder_faulted: false,
                #[cfg(test)]
                decode_probe: None,
                #[cfg(test)]
                decode_probe_report: None,
            }));
            Ok(true)
        })();
        self.retained_settings_result_timed(
            r,
            Boundary::SettingsReaderConfiguration,
            result,
            timer.finish(),
        )
    }

    pub fn prepare_settings_read_v1(
        &mut self,
        target: &SettingsTargetId,
        scope: &SettingsReadScopeV1,
        facts: &TrustedSettingsReadPreparationV1<'_>,
        r: &mut Recorder,
    ) -> Result<SettingsReadCutIdV1, SettingsError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(admission_error)?;
            if !facts.read.ready()
                || [
                    facts.owner_control_allowed,
                    facts.preparation_reserved,
                    facts.visibility_attested,
                    facts.visibility_complete,
                    facts.target_readable,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(SettingsError::Unavailable);
            }
            let pool = self.settings_reader_pool()?;
            pool.delivery_ready()?;
            let scope_size = pool.scope(scope)?;
            if pool.issued >= pool.limits.max_issued || pool.counts().0 >= pool.limits.max_packets {
                return Err(SettingsError::Limit);
            }
            let root = self.settings.as_ref().ok_or(SettingsError::Unavailable)?;
            let at = self.owned_settings_target(root, target)?;
            let identity = root.target_identity(at)?;
            let data = project_settings_v1(
                root,
                identity,
                facts.keys,
                facts.capabilities,
                pool.limits.projection(),
                r,
            )?;
            let bytes = root
                .usage()
                .logical_bytes
                .checked_add(data.logical_bytes)
                .and_then(|n| n.checked_add(scope_size))
                .and_then(|n| n.checked_add(size_of::<Cut>()))
                .ok_or(SettingsError::Limit)?;
            if bytes > pool.limits.max_packet_bytes {
                return Err(SettingsError::Limit);
            }
            let cut = Cut {
                scope: copy_scope(scope)?,
                target: target.clone(),
                root: root.clone(),
                data,
            };
            let owner = self.namespace.clone();
            let pool = self.settings_reader_mut()?;
            let id = SettingsReadCutIdV1 {
                owner,
                slot: pool.rows.len(),
            };
            pool.rows.push(Some(Row::Cut(cut)));
            pool.packets += 1;
            pool.issued += 1;
            Ok(id)
        })();
        self.retained_settings_result_timed(
            r,
            Boundary::SettingsReadPreparation,
            result,
            timer.finish(),
        )
    }

    fn take_settings_read_cut(
        &mut self,
        id: &SettingsReadCutIdV1,
        scope: &SettingsReadScopeV1,
    ) -> Result<Cut, SettingsError> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return Err(SettingsError::Unavailable);
        }
        let root = self.settings.as_ref().ok_or(SettingsError::Unavailable)?;
        let pool = self.settings_reader_pool()?;
        pool.scope(scope)?;
        let Some(Some(Row::Cut(cut))) = pool.rows.get(id.slot) else {
            return Err(SettingsError::Unavailable);
        };
        if cut.scope != *scope || !Arc::ptr_eq(root, &cut.root) {
            return Err(SettingsError::Unavailable);
        }
        let pool = self.settings_reader_mut()?;
        match pool.rows[id.slot].take() {
            Some(Row::Cut(cut)) => {
                pool.packets -= 1;
                Ok(cut)
            }
            _ => Err(SettingsError::Faulted),
        }
    }

    pub fn acquire_settings_read_v1(
        &mut self,
        cut: &SettingsReadCutIdV1,
        scope: &SettingsReadScopeV1,
        facts: &TrustedSettingsReadFactsV1,
        r: &mut Recorder,
    ) -> Result<SettingsReadAcquisitionV1, SettingsError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(admission_error)?;
            if !facts.ready() {
                return Err(SettingsError::Unavailable);
            }
            let pool = self.settings_reader_pool()?;
            pool.delivery_ready()?;
            pool.scope(scope)?;
            if pool.counts().1 >= pool.limits.max_contexts {
                return Err(SettingsError::Limit);
            }
            let limits = pool.limits;
            let cut = self.take_settings_read_cut(cut, scope)?;
            let origins = release::capture_origins(&cut.data, limits)?;
            let cost = origins.len().checked_add(2).ok_or(SettingsError::Limit)?;
            let pool = self.settings_reader_pool()?;
            if cost > limits.max_issued.saturating_sub(pool.issued)
                || size_of::<Context>()
                    .checked_add(cut.data.logical_bytes)
                    .is_none_or(|n| n > limits.max_capture_bytes)
                || size_of::<SettingsReadAcquisitionV1>() > limits.max_reply_bytes
            {
                return Err(SettingsError::Limit);
            }
            let owner = self.namespace.clone();
            let pool = self.settings_reader_mut()?;
            let slot = pool.rows.len();
            let marker = Arc::new(());
            let target = SettingsReadTargetRefV1 {
                owner: marker.clone(),
                slot: 0,
            };
            pool.rows.push(Some(Row::Context(Context {
                scope: cut.scope,
                target: cut.target,
                marker,
                data: Some(cut.data),
                origins,
            })));
            pool.issued += cost;
            pool.contexts += 1;
            Ok(SettingsReadAcquisitionV1 {
                context: SettingsReadContextIdV1 { owner, slot },
                target,
            })
        })();
        self.retained_settings_result_timed(
            r,
            Boundary::SettingsReadAcquisition,
            result,
            timer.finish(),
        )
    }
}
