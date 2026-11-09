use super::*;
mod shutdown_inventory;
use crate::capture::{CaptureErrorV1, InertCaptureV1};
use std::mem::size_of;
pub(super) mod builtin;
pub(super) mod decoder;
mod input_schema;
mod queries;
#[cfg(test)]
mod tests;
#[derive(Default)]
pub(in crate::coordinator) enum State {
    #[default]
    Unconfigured,
    Active(Box<Store>),
    Closed,
}
impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DiscoveryOwner(<redacted>)")
    }
}
pub(in crate::coordinator) struct Store {
    authority: String,
    engine: String,
    limits: DiscoveryLimitsV1,
    packets: Vec<Option<Packet>>,
    contexts: Vec<Option<Context>>,
    issued: usize,
    stopped: bool,
    faulted: bool,
    reservation: retention::Reservation,
}
struct Packet {
    origin: Origin,
    marker: Arc<()>,
    root: root_identity::NativeRootIdentity,
    scope: DiscoveryScopeV1,
    rows: Vec<Row>,
    target: Option<Arc<InertCaptureV1>>,
    target_known: bool,
    advice_known: bool,
}
struct Context {
    origin: Origin,
    marker: Arc<()>,
    scope: DiscoveryScopeV1,
    rows: Vec<Row>,
    target: Option<(Arc<()>, Arc<InertCaptureV1>)>,
    stopped: bool,
    selectors: Vec<queries::Selector>,
    decoder: decoder::State,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Origin {
    Generic,
    BuiltinV1,
    BuiltinV2,
    BuiltinV3,
    BuiltinV4,
    NativeContainersV1,
    NativeRetainedReleaseV1,
    NativeRebindV1,
    NativeSourceLoadV1,
    NativeAnnotationLoadV1,
    NativeSaveAsV1,
    NativeOrdinarySaveV1,
    NativeReadAdmissionV1,
    NativeRegistryReceiptV1,
    NativeRegistryCommandV1,
    NativeSourcePublicationV1,
    NativeFacetPublicationV1,
    NativeCollectionPublicationV1,
    NativeObservationRevocationV1,
    NativeObservationControlV1,
    NativeInvocationObservationV1,
    NativeOperationObservationV1,
    NativeInvocationCancellationV1,
    NativeShutdownDispositionV1,
    NativeSettingsReadV1,
    NativeCatalogReadV1,
    NativeSourceReadV1,
    NativeInspectionReadV1,
    NativeSelectionV1,
    NativeCompositionV1,
    NativeReplacementV1,
    NativeDiagnosticDrainV1,
    NativeExactBatchV1,
    NativeLiteralSearchV1,
    NativeAtomicAuthoringV1,
    NativeCompoundAdviceV1,
    NativeDiscoveryQueryV1,
}
impl Origin {
    fn matches_profile(self, profile: &str) -> bool {
        match self {
            Self::Generic => supported_profile(profile),
            Self::BuiltinV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::BuiltinV2 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V2,
            Self::BuiltinV3 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V3,
            Self::BuiltinV4 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V4,
            Self::NativeContainersV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeRetainedReleaseV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeRebindV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeSourceLoadV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeAnnotationLoadV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeSaveAsV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeOrdinarySaveV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeReadAdmissionV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeRegistryReceiptV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeRegistryCommandV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeSourcePublicationV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeFacetPublicationV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeCollectionPublicationV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeObservationRevocationV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeObservationControlV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeInvocationObservationV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeOperationObservationV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeInvocationCancellationV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeShutdownDispositionV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeSettingsReadV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeCatalogReadV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeSourceReadV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeInspectionReadV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeSelectionV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeCompositionV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeReplacementV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeDiagnosticDrainV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeExactBatchV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeLiteralSearchV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeAtomicAuthoringV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeCompoundAdviceV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
            Self::NativeDiscoveryQueryV1 => profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
        }
    }
}
fn supported_profile(profile: &str) -> bool {
    [
        OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
        OWNED_CONTRACT_DISCOVERY_PROFILE_V2,
        OWNED_CONTRACT_DISCOVERY_PROFILE_V3,
        OWNED_CONTRACT_DISCOVERY_PROFILE_V4,
    ]
    .contains(&profile)
}
struct Key {
    authority: String,
    id: String,
    incarnation: u64,
}
struct Variant {
    id: String,
    schema: Option<Arc<InertCaptureV1>>,
    order: Option<usize>,
    enabled: Option<bool>,
}
struct Row {
    key: Key,
    contract: String,
    version: u64,
    kind: DiscoveryExecutionKindV1,
    variants: Vec<Variant>,
    presentation: Option<Arc<InertCaptureV1>>,
    order: Option<usize>,
    schema_visible: bool,
    presentation_visible: bool,
    advice_visible: bool,
}
fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(Error::Limit)
}
fn mul(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_mul(b).ok_or(Error::Limit)
}
fn parent(e: RetentionErrorV1) -> Error {
    match e {
        RetentionErrorV1::Limit => Error::Limit,
        RetentionErrorV1::Unavailable => Error::Unavailable,
        RetentionErrorV1::Faulted => Error::Faulted,
    }
}
fn admission(error: CoordinatorError) -> Error {
    match error {
        CoordinatorError::Unavailable => Error::Unavailable,
        _ => Error::Faulted,
    }
}
fn text(v: &str) -> Result<String, Error> {
    let mut out = String::new();
    out.try_reserve_exact(v.len())
        .map_err(|_| Error::Capacity)?;
    out.push_str(v);
    Ok(out)
}
fn scope_bytes(c: &DiscoveryScopeV1) -> Result<usize, Error> {
    settings::basis::labels(&c.context)
        .into_iter()
        .map(str::len)
        .try_fold(size_of::<DiscoveryScopeV1>(), add)
}
fn scope_copy(c: &DiscoveryScopeV1) -> Result<DiscoveryScopeV1, Error> {
    Ok(DiscoveryScopeV1 {
        context: settings::basis::copy_context(&c.context).map_err(|_| Error::Capacity)?,
        policy_epoch: c.policy_epoch,
    })
}
fn meta(
    v: &InertValueV1<'_>,
    l: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<Arc<InertCaptureV1>, Error> {
    InertCaptureV1::copy(v, l, r)
        .map(Arc::new)
        .map_err(|e| match e {
            CaptureErrorV1::Limit => Error::Limit,
            CaptureErrorV1::Capacity => Error::Capacity,
            _ => Error::InvalidRequest,
        })
}
fn same_key(a: &Key, b: DiscoveryCommandV1<'_>) -> bool {
    a.authority == b.authority && a.id == b.id && a.incarnation == b.incarnation
}
fn natural(v: u64) -> bool {
    v <= 9_007_199_254_740_991
}
fn authority(f: &TrustedDiscoveryFactsV1) -> bool {
    [
        f.authorized,
        f.scope,
        f.continuous,
        f.live,
        f.not_stopped,
        f.discovery_allowed,
        f.interpretation_current,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
impl DiscoveryLimitsV1 {
    pub fn declared_bytes(self) -> Result<usize, Error> {
        let contexts = add(
            self.max_context_bytes,
            add(
                mul(2, self.max_capture_bytes)?,
                mul(2, self.max_reply_bytes)?,
            )?,
        )?;
        [
            self.max_context_bytes,
            mul(self.max_preparations, self.max_packet_bytes)?,
            mul(self.max_contexts, contexts)?,
            mul(mul(2, self.max_selectors)?, self.max_selector_bytes)?,
            size_of::<Store>(),
            size_of::<State>(),
            mul(self.max_preparations, size_of::<Option<Packet>>())?,
            mul(self.max_contexts, size_of::<Option<Context>>())?,
            builtin::workspace_bytes(self.max_context_bytes)?,
        ]
        .into_iter()
        .try_fold(0, add)
    }
}
impl Row {
    fn bytes(&self) -> Result<usize, Error> {
        let mut total = [
            self.key.authority.len(),
            self.key.id.len(),
            self.contract.len(),
            self.presentation
                .as_ref()
                .map_or(0, |m| m.usage().retained_bytes),
        ]
        .into_iter()
        .try_fold(size_of::<Row>(), add)?;
        for v in &self.variants {
            total = add(
                total,
                add(
                    size_of::<Variant>(),
                    add(
                        v.id.len(),
                        v.schema.as_ref().map_or(0, |s| s.usage().retained_bytes),
                    )?,
                )?,
            )?;
        }
        Ok(total)
    }
}
fn rows_copy(
    input: &DiscoveryCutInputV1<'_>,
    l: DiscoveryLimitsV1,
    budget: usize,
    r: &mut Recorder,
) -> Result<Vec<Row>, Error> {
    rows_copy_profile(input, l, budget, OWNED_CONTRACT_DISCOVERY_PROFILE_V1, r)
}
fn rows_copy_profile(
    input: &DiscoveryCutInputV1<'_>,
    l: DiscoveryLimitsV1,
    budget: usize,
    profile: &str,
    r: &mut Recorder,
) -> Result<Vec<Row>, Error> {
    if input.entries.len() > l.max_entries {
        return Err(Error::Limit);
    }
    if mul(input.entries.len(), size_of::<Row>())? > budget {
        return Err(Error::Limit);
    }
    let mut used = 0usize;
    let mut rows: Vec<Row> = Vec::new();
    rows.try_reserve_exact(input.entries.len())
        .map_err(|_| Error::Capacity)?;
    let visible = input.entries.iter().filter(|e| e.order.is_some()).count();
    for e in input.entries {
        if e.command.authority.is_empty()
            || e.command.id.is_empty()
            || e.contract.is_empty()
            || !natural(e.command.incarnation)
            || !natural(e.version)
            || e.version == 0
            || [
                e.command.authority.len(),
                e.command.id.len(),
                e.contract.len(),
            ]
            .into_iter()
            .any(|n| n > l.max_context_bytes)
            || rows.iter().any(|p| same_key(&p.key, e.command))
            || e.order
                .is_some_and(|n| n >= visible || rows.iter().any(|p| p.order == Some(n)))
            || (e.order.is_none()
                && (e.schema_visible
                    || e.presentation_visible
                    || e.advice_visible
                    || e.variants.iter().any(|v| v.order.is_some())))
        {
            return Err(Error::InvalidRequest);
        }
        if e.variants.len() > l.max_variants {
            return Err(Error::Limit);
        }
        let base = [
            e.command.authority.len(),
            e.command.id.len(),
            e.contract.len(),
            mul(e.variants.len(), size_of::<Variant>())?,
        ]
        .into_iter()
        .try_fold(size_of::<Row>(), add)?;
        if base > l.max_row_bytes || add(used, base)? > budget {
            return Err(Error::Limit);
        }
        let mut row_used = base;
        let InertValueV1::Object(presentation) = e.presentation else {
            return Err(Error::InvalidRequest);
        };
        if presentation.len() > 5
            || presentation.iter().any(|(k, _)| {
                !["title", "description", "category", "aliases", "bindings"].contains(k)
            })
        {
            return Err(Error::InvalidRequest);
        }
        let shown = e.variants.iter().filter(|v| v.order.is_some()).count();
        let mut variants: Vec<Variant> = Vec::new();
        variants
            .try_reserve_exact(e.variants.len())
            .map_err(|_| Error::Capacity)?;
        for v in e.variants {
            if v.id.is_empty()
                || v.id.len() > l.max_context_bytes
                || variants.iter().any(|p| p.id == v.id)
                || v.order
                    .is_some_and(|n| n >= shown || variants.iter().any(|p| p.order == Some(n)))
                || v.enabled.is_some() != input.advice_present
            {
                return Err(Error::InvalidRequest);
            }
            let schema = [
                ("input", v.input),
                ("outcomes", v.outcomes),
                ("requirements", v.requirements),
                ("limits", v.limits),
                ("cancellation", v.cancellation),
            ];
            row_used = add(row_used, v.id.len())?;
            let occupied = add(used, row_used)?;
            let remaining = l
                .max_row_bytes
                .checked_sub(row_used)
                .and_then(|n| budget.checked_sub(occupied).map(|m| n.min(m)))
                .ok_or(Error::Limit)?;
            let schema = meta(
                &InertValueV1::Object(&schema),
                CaptureLimitsV1 {
                    max_bytes: l.metadata.max_bytes.min(remaining),
                    ..l.metadata
                },
                r,
            )?;
            // Interpretation is owner-selected by the exact cut scope, never
            // inferred from schema-shaped data. Bound before recognition.
            if !input_schema::recognize_facet(&schema, e.contract, v.id, profile, r) {
                return Err(Error::InvalidRequest);
            }
            row_used = add(row_used, schema.usage().retained_bytes)?;
            variants.push(Variant {
                id: text(v.id)?,
                schema: Some(schema),
                order: v.order,
                enabled: v.enabled,
            });
        }
        let occupied = add(used, row_used)?;
        let remaining = l
            .max_row_bytes
            .checked_sub(row_used)
            .and_then(|n| budget.checked_sub(occupied).map(|m| n.min(m)))
            .ok_or(Error::Limit)?;
        let presentation = meta(
            &e.presentation,
            CaptureLimitsV1 {
                max_bytes: l.metadata.max_bytes.min(remaining),
                ..l.metadata
            },
            r,
        )?;
        let row = Row {
            key: Key {
                authority: text(e.command.authority)?,
                id: text(e.command.id)?,
                incarnation: e.command.incarnation,
            },
            contract: text(e.contract)?,
            version: e.version,
            kind: e.kind,
            variants,
            presentation: Some(presentation),
            order: e.order,
            schema_visible: e.schema_visible,
            presentation_visible: e.presentation_visible,
            advice_visible: e.advice_visible,
        };
        if row.bytes()? > l.max_row_bytes {
            return Err(Error::Limit);
        }
        used = add(used, row.bytes()?)?;
        if used > budget {
            return Err(Error::Limit);
        }
        rows.push(row);
    }
    Ok(rows)
}
fn project(rows: &[Row]) -> Result<Vec<Row>, Error> {
    let mut out = Vec::new();
    out.try_reserve_exact(rows.iter().filter(|r| r.order.is_some()).count())
        .map_err(|_| Error::Capacity)?;
    for order in 0..rows.iter().filter(|r| r.order.is_some()).count() {
        let row = rows
            .iter()
            .find(|r| r.order == Some(order))
            .ok_or(Error::Faulted)?;
        let mut variants = Vec::new();
        variants
            .try_reserve_exact(row.variants.iter().filter(|v| v.order.is_some()).count())
            .map_err(|_| Error::Capacity)?;
        for index in 0..row.variants.iter().filter(|v| v.order.is_some()).count() {
            let v = row
                .variants
                .iter()
                .find(|v| v.order == Some(index))
                .ok_or(Error::Faulted)?;
            variants.push(Variant {
                id: text(&v.id)?,
                schema: if row.schema_visible {
                    v.schema.clone()
                } else {
                    None
                },
                order: Some(index),
                enabled: None,
            });
        }
        out.push(Row {
            key: Key {
                authority: text(&row.key.authority)?,
                id: text(&row.key.id)?,
                incarnation: row.key.incarnation,
            },
            contract: text(&row.contract)?,
            version: row.version,
            kind: row.kind,
            variants,
            presentation: if row.presentation_visible {
                row.presentation.clone()
            } else {
                None
            },
            order: Some(order),
            schema_visible: row.schema_visible,
            presentation_visible: row.presentation_visible,
            advice_visible: row.advice_visible,
        });
    }
    Ok(out)
}
fn projected_bytes(rows: &[Row]) -> Result<usize, Error> {
    let mut total = 0;
    for row in rows.iter().filter(|r| r.order.is_some()) {
        let mut bytes = [
            row.key.authority.len(),
            row.key.id.len(),
            row.contract.len(),
            if row.presentation_visible {
                row.presentation
                    .as_ref()
                    .map_or(0, |p| p.usage().retained_bytes)
            } else {
                0
            },
        ]
        .into_iter()
        .try_fold(size_of::<Row>(), add)?;
        for v in row.variants.iter().filter(|v| v.order.is_some()) {
            bytes = add(
                bytes,
                add(
                    size_of::<Variant>(),
                    add(
                        v.id.len(),
                        if row.schema_visible {
                            v.schema.as_ref().map_or(0, |s| s.usage().retained_bytes)
                        } else {
                            0
                        },
                    )?,
                )?,
            )?;
        }
        total = add(total, bytes)?;
    }
    Ok(total)
}
impl SourceCoordinator {
    fn discovery_root_current(
        &self,
        root: &root_identity::NativeRootIdentity,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = self
            .native_root_identity()
            .map_err(|_| Error::Faulted)
            .and_then(|current| {
                if *root == current {
                    Ok(())
                } else {
                    Err(Error::Unavailable)
                }
            });
        observe(r, Boundary::DiscoveryRootCurrentness, &result);
        result
    }
    fn validate_discovery(&self) -> Result<(), Error> {
        let s = self.discovery_store()?;
        self.retention.validate(&s.reservation).map_err(parent)?;
        if s.limits
            .declared_bytes()
            .ok()
            .is_none_or(|n| n > s.limits.max_held_bytes)
            || s.packets.len() != s.limits.max_preparations
            || s.contexts.len() != s.limits.max_contexts
            || s.issued > s.limits.max_issued
        {
            return Err(Error::Faulted);
        }
        for p in s.packets.iter().flatten() {
            if !p.origin.matches_profile(&p.scope.context.profile)
                || p.scope.context.authority != s.authority
                || p.scope.context.engine != s.engine
            {
                return Err(Error::Faulted);
            }
            let mut bytes = add(size_of::<Packet>(), scope_bytes(&p.scope)?)?;
            for row in &p.rows {
                if row.bytes()? > s.limits.max_row_bytes {
                    return Err(Error::Faulted);
                }
                bytes = add(bytes, row.bytes()?)?;
            }
            bytes = add(
                bytes,
                p.target.as_ref().map_or(0, |v| v.usage().retained_bytes),
            )?;
            if bytes > s.limits.max_packet_bytes || p.rows.len() > s.limits.max_entries {
                return Err(Error::Faulted);
            }
        }
        for c in s.contexts.iter().flatten() {
            if !c.origin.matches_profile(&c.scope.context.profile)
                || c.scope.context.authority != s.authority
                || c.scope.context.engine != s.engine
            {
                return Err(Error::Faulted);
            }
            c.decoder.validate(&self.retention)?;
            let mut bytes = add(size_of::<Context>(), scope_bytes(&c.scope)?)?;
            for row in &c.rows {
                bytes = add(bytes, row.bytes()?)?;
            }
            bytes = add(
                bytes,
                c.target
                    .as_ref()
                    .map_or(0, |(_, t)| t.usage().retained_bytes),
            )?;
            if bytes > s.limits.max_capture_bytes
                || (c.stopped
                    && (!c.rows.is_empty() || c.target.is_some() || !c.selectors.is_empty()))
            {
                return Err(Error::Faulted);
            }
        }
        queries::validate_selectors(s)?;
        Ok(())
    }
    fn discovery_finish<T>(&mut self, r: &mut Recorder, b: Boundary, result: &Result<T, Error>) {
        self.discovery_finish_timed(r, b, result, None);
    }
    fn discovery_finish_timed<T>(
        &mut self,
        r: &mut Recorder,
        b: Boundary,
        result: &Result<T, Error>,
        elapsed: Option<std::time::Duration>,
    ) {
        if matches!(result, Err(Error::Faulted))
            && let State::Active(s) = &mut self.discovery
        {
            s.faulted = true;
        }
        observe_timed(r, b, result, elapsed);
    }
    fn discovery_context(&self, id: &DiscoveryContextIdV1) -> Result<&Context, Error> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return Err(Error::Unavailable);
        }
        let context = self
            .discovery_store()?
            .contexts
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)?;
        if !Arc::ptr_eq(&id.marker, &context.marker) {
            return Err(Error::Unavailable);
        }
        Ok(context)
    }
    pub fn discovery_usage_v1(&self) -> Result<DiscoveryUsageV1, Error> {
        self.validate_discovery()?;
        let s = self.discovery_store()?;
        self.retention.validate(&s.reservation).map_err(parent)?;
        Ok(DiscoveryUsageV1 {
            contexts: s.contexts.iter().flatten().count(),
            preparations: s.packets.iter().flatten().count(),
            issued_lifetime: s.issued,
            reserved_bytes: s.limits.max_held_bytes,
            stopped: s.stopped,
            faulted: s.faulted,
            prepared_advice: s
                .packets
                .iter()
                .flatten()
                .filter(|p| p.advice_known)
                .count(),
            selectors: s.contexts.iter().flatten().map(|c| c.selectors.len()).sum(),
        })
    }
    pub fn discovery_context_usage_v1(
        &self,
        id: &DiscoveryContextIdV1,
    ) -> Result<DiscoveryContextUsageV1, Error> {
        let c = self.discovery_context(id)?;
        let mut bytes = add(size_of::<Context>(), scope_bytes(&c.scope)?)?;
        for row in &c.rows {
            bytes = add(bytes, row.bytes()?)?;
        }
        bytes = add(
            bytes,
            c.target
                .as_ref()
                .map_or(0, |(_, t)| t.usage().retained_bytes),
        )?;
        Ok(DiscoveryContextUsageV1 {
            stopped: c.stopped,
            entries: c.rows.len(),
            variants: c.rows.iter().map(|r| r.variants.len()).sum(),
            target_captured: c.target.is_some(),
            capture_bytes: bytes,
        })
    }
    pub fn retire_discovery_cut_v1(
        &mut self,
        cut: &DiscoveryCutIdV1,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = (|| {
            let s = self.discovery_store()?;
            let packet = s
                .packets
                .get(cut.slot)
                .and_then(Option::as_ref)
                .ok_or(Error::Unavailable)?;
            if !Arc::ptr_eq(&cut.owner, &self.namespace)
                || !Arc::ptr_eq(&cut.marker, &packet.marker)
                || [f.owner_control_allowed, f.retention_released]
                    .into_iter()
                    .any(|v| v != Some(true))
            {
                return Err(Error::Unavailable);
            }
            self.discovery_store_mut()?.packets[cut.slot] = None;
            Ok(())
        })();
        self.discovery_finish(r, Boundary::DiscoveryCutRetirement, &result);
        result
    }
    pub fn stop_discovery_context_v1(
        &mut self,
        id: &DiscoveryContextIdV1,
        scope: &DiscoveryScopeV1,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = (|| {
            let c = self.discovery_context(id)?;
            if c.scope != *scope || f.owner_control_allowed != Some(true) || c.decoder.busy() {
                return Err(Error::Unavailable);
            }
            let c = self.discovery_store_mut()?.contexts[id.slot]
                .as_mut()
                .expect("validated discovery context");
            c.stopped = true;
            c.rows.clear();
            c.target = None;
            c.selectors.clear();
            Ok(())
        })();
        self.discovery_finish(r, Boundary::DiscoveryContextControl, &result);
        result
    }
    pub fn retire_discovery_context_v1(
        &mut self,
        id: &DiscoveryContextIdV1,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = (|| {
            let c = self.discovery_context(id)?;
            if !c.stopped
                || c.decoder.busy()
                || [
                    f.owner_control_allowed,
                    f.retention_released,
                    f.delivery_drained,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(Error::Unavailable);
            }
            c.decoder.validate(&self.retention)?;
            let State::Active(s) = &mut self.discovery else {
                unreachable!("original discovery owner");
            };
            s.contexts[id.slot]
                .as_mut()
                .expect("original context")
                .decoder
                .release(&mut self.retention)?;
            s.contexts[id.slot] = None;
            Ok(())
        })();
        self.discovery_finish(r, Boundary::DiscoveryContextRetirement, &result);
        result
    }
    pub fn stop_discovery_admission_v1(
        &mut self,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = if f.owner_control_allowed == Some(true) {
            self.discovery_store_mut().map(|s| s.stopped = true)
        } else {
            Err(Error::Unavailable)
        };
        self.discovery_finish(r, Boundary::DiscoveryConfiguration, &result);
        result
    }
    pub fn retire_discovery_registry_v1(
        &mut self,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = (|| {
            self.validate_discovery()?;
            let s = self.discovery_store()?;
            if !s.stopped
                || s.contexts.iter().any(Option::is_some)
                || s.packets.iter().any(Option::is_some)
                || [
                    f.owner_control_allowed,
                    f.retention_released,
                    f.delivery_drained,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(Error::Unavailable);
            }
            self.retention.validate(&s.reservation).map_err(parent)?;
            let State::Active(mut s) = std::mem::replace(&mut self.discovery, State::Closed) else {
                unreachable!("validated discovery registry")
            };
            self.retention.release(&mut s.reservation).map_err(parent)
        })();
        self.discovery_finish(r, Boundary::DiscoveryRegistryRetirement, &result);
        result
    }
    fn discovery_store(&self) -> Result<&Store, Error> {
        match &self.discovery {
            State::Active(s) => Ok(s),
            _ => Err(Error::Unavailable),
        }
    }
    fn discovery_store_mut(&mut self) -> Result<&mut Store, Error> {
        match &mut self.discovery {
            State::Active(s) => Ok(s),
            _ => Err(Error::Unavailable),
        }
    }
    fn discovery_scope(&self, c: &DiscoveryScopeV1) -> Result<(), Error> {
        let s = self.discovery_store()?;
        if c.context.authority != s.authority
            || c.context.engine != s.engine
            || !supported_profile(&c.context.profile)
            || settings::basis::labels(&c.context)
                .iter()
                .any(|v| v.is_empty())
            || scope_bytes(c)? > s.limits.max_context_bytes
        {
            return Err(Error::Unavailable);
        }
        Ok(())
    }
    fn discovery_current(
        &self,
        c: &DiscoveryScopeV1,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        self.discovery_scope(c)?;
        self.validate_discovery()?;
        let s = self.discovery_store()?;
        self.retention.validate(&s.reservation).map_err(parent)?;
        if self.faulted || s.faulted || !authority(f) {
            return Err(Error::Unavailable);
        }
        if self
            .semantic_root_usage(r)
            .map_err(|_| Error::Faulted)?
            .total_bytes
            > s.limits.max_root_bytes
        {
            return Err(Error::Limit);
        }
        Ok(())
    }
    pub fn configure_discovery_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        limits: DiscoveryLimitsV1,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = (|| {
            if self.faulted
                || [
                    f.owner_control_allowed,
                    f.resources_ready,
                    f.preparation_reserved,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(Error::Unavailable);
            }
            if matches!(self.discovery, State::Active(_)) {
                self.validate_discovery()?;
            }
            match &self.discovery {
                State::Active(s) => {
                    return if s.limits == limits
                        && s.authority == scope.context.authority
                        && s.engine == scope.context.engine
                        && supported_profile(&scope.context.profile)
                    {
                        Ok(())
                    } else {
                        Err(Error::Unavailable)
                    };
                }
                State::Closed => return Err(Error::Unavailable),
                State::Unconfigured => {}
            }
            self.ensure_admission_unfenced().map_err(admission)?;
            if !supported_profile(&scope.context.profile)
                || settings::basis::labels(&scope.context)
                    .iter()
                    .any(|v| v.is_empty())
                || scope_bytes(scope)? > limits.max_context_bytes
            {
                return Err(Error::InvalidRequest);
            }
            let charge = limits.declared_bytes()?;
            if charge > limits.max_held_bytes {
                return Err(Error::Limit);
            }
            self.retention
                .check(limits.max_held_bytes)
                .map_err(parent)?;
            let mut packets = Vec::new();
            packets
                .try_reserve_exact(limits.max_preparations)
                .map_err(|_| Error::Capacity)?;
            packets.resize_with(limits.max_preparations, || None);
            let mut contexts = Vec::new();
            contexts
                .try_reserve_exact(limits.max_contexts)
                .map_err(|_| Error::Capacity)?;
            contexts.resize_with(limits.max_contexts, || None);
            let authority = text(&scope.context.authority)?;
            let engine = text(&scope.context.engine)?;
            let reservation = self
                .retention
                .reserve(limits.max_held_bytes)
                .map_err(parent)?;
            self.discovery = State::Active(Box::new(Store {
                authority,
                engine,
                limits,
                packets,
                contexts,
                issued: 0,
                stopped: false,
                faulted: false,
                reservation,
            }));
            Ok(())
        })();
        self.discovery_finish(r, Boundary::DiscoveryConfiguration, &result);
        result
    }
    pub fn prepare_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        input: DiscoveryCutInputV1<'_>,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryCutIdV1, Error> {
        self.prepare_discovery_cut_from_source(scope, input, f, Origin::Generic, r)
    }
    fn prepare_discovery_cut_from_source(
        &mut self,
        scope: &DiscoveryScopeV1,
        input: DiscoveryCutInputV1<'_>,
        f: &TrustedDiscoveryFactsV1,
        origin: Origin,
        r: &mut Recorder,
    ) -> Result<DiscoveryCutIdV1, Error> {
        let result = (|| {
            self.discovery_current(scope, f, r)?;
            if [
                f.accepted_registry_attested,
                f.schema_descriptions_attested,
                f.visibility_attested,
                f.visibility_complete,
                f.projection_order_attested,
                f.outside_publication_guard,
                f.resources_ready,
                f.preparation_reserved,
            ]
            .into_iter()
            .any(|v| v != Some(true))
            {
                return Err(Error::Unavailable);
            }
            let s = self.discovery_store()?;
            let l = s.limits;
            let slot = s
                .packets
                .iter()
                .position(Option::is_none)
                .ok_or(Error::Limit)?;
            let prefix = add(size_of::<Packet>(), scope_bytes(scope)?)?;
            let budget = l.max_packet_bytes.checked_sub(prefix).ok_or(Error::Limit)?;
            let rows = if scope.context.profile != OWNED_CONTRACT_DISCOVERY_PROFILE_V1 {
                rows_copy_profile(&input, l, budget, &scope.context.profile, r)?
            } else {
                rows_copy(&input, l, budget, r)?
            };
            let occupied = rows
                .iter()
                .try_fold(prefix, |n, row| add(n, row.bytes()?))?;
            let remaining = l
                .max_packet_bytes
                .checked_sub(occupied)
                .ok_or(Error::Limit)?;
            let target = input
                .target
                .as_ref()
                .map(|t| {
                    if t.identity.is_empty() || t.identity.len() > l.max_context_bytes {
                        return Err(Error::InvalidRequest);
                    }
                    let target = [
                        ("identity", InertValueV1::Text(t.identity)),
                        ("basis", t.basis),
                    ];
                    meta(
                        &InertValueV1::Object(&target),
                        CaptureLimitsV1 {
                            max_bytes: l.metadata.max_bytes.min(remaining),
                            ..l.metadata
                        },
                        r,
                    )
                })
                .transpose()?;
            let mut bytes = prefix;
            for row in &rows {
                bytes = add(bytes, row.bytes()?)?;
            }
            bytes = add(
                bytes,
                target.as_ref().map_or(0, |m| m.usage().retained_bytes),
            )?;
            if bytes > l.max_packet_bytes {
                return Err(Error::Limit);
            }
            let scope = scope_copy(scope)?;
            let marker = Arc::new(());
            let id = DiscoveryCutIdV1 {
                owner: self.namespace.clone(),
                marker: marker.clone(),
                slot,
            };
            let root = self.native_root_identity().map_err(|_| Error::Faulted)?;
            let target_known = target.is_some()
                && f.target_binding_attested == Some(true)
                && f.target_equality_observable == Some(true);
            let advice_known = input.advice_present
                && f.advice_attested == Some(true)
                && f.target_current == Some(true)
                && rows
                    .iter()
                    .flat_map(|row| &row.variants)
                    .all(|v| v.enabled.is_some());
            self.discovery_store_mut()?.packets[slot] = Some(Packet {
                origin,
                marker,
                root,
                scope,
                rows,
                target,
                target_known,
                advice_known,
            });
            Ok(id)
        })();
        self.discovery_finish(r, Boundary::DiscoveryPreparation, &result);
        result
    }
    pub fn acquire_discovery_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        cut: &DiscoveryCutIdV1,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryAcquisitionV1, Error> {
        let result = (|| {
            // Visibility evaluation remains independently current for retained
            // queries. It is not permission to establish another context, and
            // refusal here must not consume a query's prepared evaluation.
            self.ensure_admission_unfenced().map_err(admission)?;
            self.discovery_current(scope, f, r)?;
            if self.discovery_store()?.stopped {
                return Err(Error::Unavailable);
            }
            if [
                f.outside_publication_guard,
                f.retention_ready,
                f.transfer_ready,
                f.registry_ready,
                f.resources_ready,
                f.reply_ready,
            ]
            .into_iter()
            .any(|v| v != Some(true))
            {
                return Err(Error::Unavailable);
            }
            let s = self.discovery_store()?;
            let packet = s
                .packets
                .get(cut.slot)
                .and_then(Option::as_ref)
                .ok_or(Error::Unavailable)?;
            if !Arc::ptr_eq(&cut.owner, &self.namespace)
                || !Arc::ptr_eq(&cut.marker, &packet.marker)
                || packet.scope != *scope
            {
                return Err(Error::Unavailable);
            }
            self.discovery_root_current(&packet.root, r)?;
            // A valid root/scope-matching packet is single-use even if later
            // projection or capacity refuses. Wrong-scope packets are untouched.
            let packet = self.discovery_store_mut()?.packets[cut.slot]
                .take()
                .expect("validated discovery cut");
            let l = self.discovery_store()?.limits;
            let projected = add(
                add(size_of::<Context>(), scope_bytes(scope)?)?,
                projected_bytes(&packet.rows)?,
            )?;
            let projected = add(
                projected,
                if packet.target_known {
                    packet
                        .target
                        .as_ref()
                        .map_or(0, |p| p.usage().retained_bytes)
                } else {
                    0
                },
            )?;
            if projected > l.max_capture_bytes {
                return Err(Error::Limit);
            }
            let rows = project(&packet.rows)?;
            let target = if packet.target_known {
                packet.target.map(|t| (Arc::new(()), t))
            } else {
                None
            };
            let s = self.discovery_store()?;
            let l = s.limits;
            let cost = 1 + usize::from(target.is_some());
            let slot = s
                .contexts
                .iter()
                .position(Option::is_none)
                .ok_or(Error::Limit)?;
            if add(s.issued, cost)? > l.max_issued
                || size_of::<DiscoveryAcquisitionV1>() > l.max_reply_bytes
            {
                return Err(Error::Limit);
            }
            let mut bytes = add(size_of::<Context>(), scope_bytes(scope)?)?;
            for row in &rows {
                bytes = add(bytes, row.bytes()?)?;
            }
            bytes = add(
                bytes,
                target.as_ref().map_or(0, |(_, t)| t.usage().retained_bytes),
            )?;
            if bytes > l.max_capture_bytes {
                return Err(Error::Limit);
            }
            let marker = Arc::new(());
            let context = DiscoveryContextIdV1 {
                owner: self.namespace.clone(),
                marker: marker.clone(),
                slot,
            };
            let reply = DiscoveryAcquisitionV1 {
                context,
                target: target.as_ref().map(|(marker, _)| DiscoveryTargetIdV1 {
                    owner: self.namespace.clone(),
                    marker: marker.clone(),
                    slot,
                }),
            };
            let scope = scope_copy(scope)?;
            let s = self.discovery_store_mut()?;
            s.contexts[slot] = Some(Context {
                origin: packet.origin,
                marker,
                scope,
                rows,
                target,
                stopped: false,
                selectors: Vec::new(),
                decoder: decoder::State::default(),
            });
            s.issued += cost;
            Ok(reply)
        })();
        self.discovery_finish(r, Boundary::DiscoveryAcquisition, &result);
        result
    }
}
