//! Closed native F07 catalog envelopes; opaque roles are never data/label lookup.
use super::*;
use crate::capture::{CaptureLimitsV1, InertCaptureV1, InertValueV1 as D};
#[derive(Clone, Copy)]
pub enum ResourceCatalogReadEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, ResourceCatalogReadEnvelopeValueV1<'a, 'h>)]),
    Data(D<'a>),
    Context(&'h ResourceCatalogReadContextIdV1),
    Entry(&'h ResourceCatalogReadEntryIdV1),
    Continuation(&'h ResourceCatalogReadContinuationIdV1),
}
type V<'a, 'h> = ResourceCatalogReadEnvelopeValueV1<'a, 'h>;
pub struct NormalizedCatalogReadArgumentsV1<'h> {
    typed: bool,
    context: &'h ResourceCatalogReadContextIdV1,
    query: ResourceCatalogReadQueryV1<'h>,
    scope: ResourceCatalogReadScopeV1,
    _record: InertCaptureV1,
}
impl std::fmt::Debug for ResourceCatalogReadEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CatalogReadEnvelope(<redacted>)")
    }
}
impl std::fmt::Debug for NormalizedCatalogReadArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedCatalogReadArguments(<redacted>)")
    }
}
impl<'h> NormalizedCatalogReadArgumentsV1<'h> {
    pub fn is_typed_route(&self) -> bool {
        self.typed
    }
    pub fn query(&self) -> ResourceCatalogReadQueryV1<'h> {
        self.query
    }
}
fn fields<'a, 'h>(v: &V<'a, 'h>, names: &[&str]) -> Result<&'a [(&'a str, V<'a, 'h>)]> {
    let V::Object(rows) = v else {
        return Err(ResourceError::Unavailable);
    };
    if rows.len() != names.len()
        || rows
            .iter()
            .enumerate()
            .any(|(i, (k, _))| !names.contains(k) || rows[..i].iter().any(|(old, _)| old == k))
    {
        return Err(ResourceError::Unavailable);
    }
    Ok(rows)
}
fn get<'a, 'h>(rows: &'a [(&'a str, V<'a, 'h>)], key: &str) -> Result<V<'a, 'h>> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
        .ok_or(ResourceError::Unavailable)
}
fn is_text(v: V<'_, '_>, expected: &str) -> bool {
    matches!(v, V::Data(D::Text(s)) if s == expected)
}
fn parse<'h>(
    e: &V<'_, 'h>,
) -> Result<(
    bool,
    &'h ResourceCatalogReadContextIdV1,
    ResourceCatalogReadQueryV1<'h>,
)> {
    let outer = fields(e, &["route", "request"])?;
    let typed = is_text(get(outer, "route")?, "typed");
    if !typed && !is_text(get(outer, "route")?, "dynamic") {
        return Err(ResourceError::Unavailable);
    }
    let h = fields(
        &get(outer, "request")?,
        &[
            if typed { "family" } else { "name" },
            "variant",
            "version",
            "arguments",
        ],
    )?;
    if !(if typed {
        matches!(get(h, "family")?, V::Data(D::Number(6.0)))
    } else {
        is_text(get(h, "name")?, "inspect-context")
    }) || !matches!(get(h, "version")?, V::Data(D::Number(1.0)))
        || !is_text(get(h, "variant")?, "capabilities")
    {
        return Err(ResourceError::Unavailable);
    }
    let a = fields(
        &get(h, "arguments")?,
        &["contract", "operation", "context", "profile", "request"],
    )?;
    if !is_text(get(a, "contract")?, "resource-catalog-query/1")
        || !is_text(get(a, "operation")?, "capabilities")
        || !is_text(get(a, "profile")?, RESOURCE_CATALOG_READ_PROFILE_V1)
    {
        return Err(ResourceError::Unavailable);
    }
    let V::Context(context) = get(a, "context")? else {
        return Err(ResourceError::Unavailable);
    };
    let raw = get(a, "request")?;
    let V::Object(rows) = raw else {
        return Err(ResourceError::Unavailable);
    };
    // At most three closed fields are scanned; arbitrary inert subgraphs are
    // never walked during classification.
    if rows.len() > 3 {
        return Err(ResourceError::Unavailable);
    }
    let operation = get(rows, "operation")?;
    let query = if is_text(operation, "list") {
        let q = fields(&raw, &["operation", "limit", "continuation"])?;
        let V::Data(D::Number(limit)) = get(q, "limit")? else {
            return Err(ResourceError::Unavailable);
        };
        if !(1.0..=9_007_199_254_740_991.0).contains(&limit) || limit.fract() != 0.0 {
            return Err(ResourceError::Unavailable);
        }
        let continuation = match get(q, "continuation")? {
            V::Data(D::Null) => None,
            V::Continuation(c) => Some(c),
            _ => return Err(ResourceError::Unavailable),
        };
        ResourceCatalogReadQueryV1::List {
            limit: usize::try_from(limit as u64).map_err(|_| ResourceError::Limit)?,
            continuation,
        }
    } else if is_text(operation, "inspect") {
        let q = fields(&raw, &["operation", "entry"])?;
        let V::Entry(entry) = get(q, "entry")? else {
            return Err(ResourceError::Unavailable);
        };
        ResourceCatalogReadQueryV1::Inspect { entry }
    } else {
        return Err(ResourceError::Unavailable);
    };
    Ok((typed, context, query))
}
fn capture(v: D<'_>, limits: CaptureLimitsV1, r: &mut Recorder) -> Result<InertCaptureV1> {
    InertCaptureV1::copy(&v, limits, r).map_err(|e| match e {
        crate::capture::CaptureErrorV1::Capacity => ResourceError::Capacity,
        _ => ResourceError::Limit,
    })
}
pub fn normalize_resource_catalog_read_envelope_v1<'h>(
    envelope: &V<'_, 'h>,
    scope: &ResourceCatalogReadScopeV1,
    requested: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedCatalogReadArgumentsV1<'h>> {
    let timing = r.begin_service_timing();
    let result = (|| {
        if requested.max_nodes == 0
            || [
                requested.max_bytes,
                requested.max_nodes,
                requested.max_depth,
            ]
            .into_iter()
            .any(|n| n as u128 > 9_007_199_254_740_991u128)
        {
            return Err(ResourceError::Limit);
        }
        let limits = CaptureLimitsV1 {
            max_bytes: requested.max_bytes.min(maximum),
            ..requested
        };
        let (typed, context, query) = parse(envelope)?;
        let (op, selector, value) = match query {
            ResourceCatalogReadQueryV1::List { limit, .. } => {
                ("list", "continuation", D::Number(limit as f64))
            }
            ResourceCatalogReadQueryV1::Inspect { .. } => ("inspect", "entry", D::Null),
        };
        let nested = [
            ("operation", D::Text(op)),
            (if op == "list" { "limit" } else { selector }, value),
            (selector, D::Null),
        ];
        let nested = &nested[..if op == "list" { 3 } else { 2 }];
        let args = [
            ("contract", D::Text("resource-catalog-query/1")),
            ("operation", D::Text("capabilities")),
            ("context", D::Null),
            ("profile", D::Text(RESOURCE_CATALOG_READ_PROFILE_V1)),
            ("request", D::Object(nested)),
        ];
        let header = [
            (
                if typed { "family" } else { "name" },
                if typed {
                    D::Number(6.0)
                } else {
                    D::Text("inspect-context")
                },
            ),
            ("variant", D::Text("capabilities")),
            ("version", D::Number(1.0)),
            ("arguments", D::Object(&args)),
        ];
        let whole = [
            ("route", D::Text(if typed { "typed" } else { "dynamic" })),
            ("request", D::Object(&header)),
        ];
        capture(D::Object(&whole), limits, r)?;
        let c = &scope.context;
        let authority = [
            ("authority", D::Text(&c.authority)),
            ("engine", D::Text(&c.engine)),
            ("recipient", D::Text(&c.recipient)),
            ("purpose", D::Text(&c.purpose)),
            ("destination", D::Text(&c.destination)),
            ("policy", D::Text(&scope.policy)),
            ("continuity", D::Text(&c.continuity)),
            ("profile", D::Text(&c.profile)),
        ];
        if authority
            .iter()
            .any(|(_, v)| matches!(v, D::Text(s) if s.is_empty()))
        {
            return Err(ResourceError::Unavailable);
        }
        capture(D::Object(&authority), limits, r)?;
        let profile = [
            ("max-bytes", D::Number(requested.max_bytes as f64)),
            ("max-nodes", D::Number(requested.max_nodes as f64)),
            ("max-depth", D::Number(requested.max_depth as f64)),
        ];
        let roles = [
            D::Text("registration-read-context"),
            D::Text(if op == "list" {
                "registration-continuation"
            } else {
                "registration-entry"
            }),
        ];
        let has_selector = !matches!(
            query,
            ResourceCatalogReadQueryV1::List {
                continuation: None,
                ..
            }
        );
        let record = [
            ("request", D::Object(&args)),
            ("scope", D::Object(&authority)),
            ("limits", D::Object(&profile)),
            (
                "opaque-bindings",
                D::Array(&roles[..if has_selector { 2 } else { 1 }]),
            ),
        ];
        Ok(NormalizedCatalogReadArgumentsV1 {
            typed,
            context,
            query,
            scope: copy_scope(scope)?,
            _record: capture(D::Object(&record), limits, r)?,
        })
    })();
    record(
        r,
        Boundary::ResourceCatalogReadArgumentNormalization,
        &result,
        timing.finish(),
    );
    result
}
pub const fn resource_catalog_read_classification_bytes_v1() -> usize {
    size_of::<CaptureLimitsV1>()
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResourceCatalogReadDecoderUsageV1 {
    pub busy: bool,
    pub declared_workspace_bytes: usize,
    pub faulted: bool,
}
#[cfg(test)]
#[derive(Clone, Copy)]
pub(super) struct Probe {
    pub point: usize,
    pub panic: bool,
}
impl SourceCoordinator {
    pub fn resource_catalog_read_decoder_usage_v1(&self) -> ResourceCatalogReadDecoderUsageV1 {
        match &self.catalog_reads {
            State::Active(p) => ResourceCatalogReadDecoderUsageV1 {
                busy: p.decoding,
                declared_workspace_bytes: p.decode_charge,
                faulted: p.decoder_faulted,
            },
            _ => ResourceCatalogReadDecoderUsageV1::default(),
        }
    }
    pub(in crate::coordinator) fn recognize_resource_catalog_read_request_v1(
        &self,
        e: &V<'_, '_>,
    ) -> Result<()> {
        self.ensure_operational().map_err(admission)?;
        let p = self.catalog_read_pool()?;
        p.ready()?;
        if p.decoding {
            return Err(ResourceError::Unavailable);
        }
        parse(e)?;
        Ok(())
    }
    pub fn query_decoded_resource_catalog_read_v1(
        &mut self,
        envelope: &V<'_, '_>,
        limits: CaptureLimitsV1,
        scope: &ResourceCatalogReadScopeV1,
        cut: &ResourceCatalogVisibilityIdV1,
        facts: &TrustedResourceCatalogReadFactsV1,
        r: &mut Recorder,
    ) -> Result<ResourceCatalogReadReplyV1> {
        let timing = r.begin_service_timing();
        let result = self.decode_catalog_read(envelope, limits, scope, cut, facts, r);
        if matches!(result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        if self.resource_catalog_read_decoder_usage_v1().faulted {
            r.record(Observation {
                boundary: Boundary::ResourceCatalogReadDecodedDispatch,
                disposition: crate::instrumentation::Disposition::Unknown,
                service_time: timing.finish(),
            });
        } else {
            record(
                r,
                Boundary::ResourceCatalogReadDecodedDispatch,
                &result,
                timing.finish(),
            );
        }
        result
    }
    fn decode_catalog_read(
        &mut self,
        envelope: &V<'_, '_>,
        limits: CaptureLimitsV1,
        scope: &ResourceCatalogReadScopeV1,
        cut: &ResourceCatalogVisibilityIdV1,
        facts: &TrustedResourceCatalogReadFactsV1,
        r: &mut Recorder,
    ) -> Result<ResourceCatalogReadReplyV1> {
        self.recognize_catalog_read_owner()?;
        let class = resource_catalog_read_classification_bytes_v1();
        let mut classification = self.retention.reserve(class).map_err(parent)?;
        let p = self.catalog_read_pool_mut()?;
        p.decoding = true;
        p.decode_charge = class;
        let mut workspace = None;
        let result = catch_unwind(AssertUnwindSafe(|| {
            #[cfg(test)]
            self.catalog_decode_probe(0, envelope, limits, scope, cut, facts, r);
            if limits.max_nodes == 0 {
                return Err(ResourceError::Limit);
            }
            let maximum = self.catalog_read_parent_intent_limit();
            let bytes = limits
                .max_bytes
                .min(maximum)
                .checked_mul(7)
                .ok_or(ResourceError::Limit)?;
            workspace = Some(self.retention.reserve(bytes).map_err(parent)?);
            self.catalog_read_pool_mut()?.decode_charge = add(class, bytes)?;
            let n =
                normalize_resource_catalog_read_envelope_v1(envelope, scope, limits, maximum, r)?;
            let facts = *facts;
            #[cfg(test)]
            self.catalog_decode_probe(1, envelope, limits, scope, cut, &facts, r);
            self.catalog_read_pool()?.scope(&n.scope)?;
            #[cfg(test)]
            self.catalog_decode_probe(2, envelope, limits, scope, cut, &facts, r);
            let reply = self
                .query_resource_catalog_read_owned(n.context, &n.scope, cut, n.query, &facts, r)?;
            #[cfg(test)]
            self.catalog_decode_probe(3, envelope, limits, scope, cut, &facts, r);
            if !self.catalog_reply_bound(n.context, n.query, &reply)? {
                return Err(ResourceError::Faulted);
            }
            Ok(reply)
        }));
        let result = match result {
            Ok(Err(ResourceError::Faulted)) | Err(_) => {
                if let State::Active(p) = &mut self.catalog_reads {
                    p.decoder_faulted = true;
                }
                Err(ResourceError::Unavailable)
            }
            Ok(result) => result,
        };
        let extra = workspace
            .as_mut()
            .map(|v| self.retention.release(v))
            .transpose();
        let class_release = self.retention.release(&mut classification);
        let State::Active(p) = &mut self.catalog_reads else {
            self.faulted = true;
            return Err(ResourceError::Faulted);
        };
        p.decoding = false;
        p.decode_charge = 0;
        if extra.is_err() || class_release.is_err() {
            self.faulted = true;
            return Err(ResourceError::Faulted);
        }
        result
    }
    fn recognize_catalog_read_owner(&self) -> Result<()> {
        self.ensure_operational().map_err(admission)?;
        let p = self.catalog_read_pool()?;
        p.ready()?;
        if p.decoding {
            Err(ResourceError::Unavailable)
        } else {
            Ok(())
        }
    }
    pub(super) fn catalog_reply_bound(
        &self,
        id: &ResourceCatalogReadContextIdV1,
        q: ResourceCatalogReadQueryV1<'_>,
        reply: &ResourceCatalogReadReplyV1,
    ) -> Result<bool> {
        let p = self.catalog_read_pool()?;
        let c = p
            .contexts
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(ResourceError::Faulted)?;
        if c.stopped {
            return Ok(false);
        }
        Ok(match (q, reply) {
            (
                ResourceCatalogReadQueryV1::Inspect { entry },
                ResourceCatalogReadReplyV1::Registration {
                    context,
                    entry: actual,
                    table,
                    descriptor,
                },
            ) => {
                context == id
                    && actual == entry
                    && Arc::ptr_eq(&entry.owner, &c.marker)
                    && c.entries.get(entry.slot) == Some(&true)
                    && c.rows.get(entry.slot).is_some_and(|row| {
                        row.address.table == *table && row.descriptor == *descriptor
                    })
            }
            (
                ResourceCatalogReadQueryV1::List {
                    limit,
                    continuation: after,
                },
                ResourceCatalogReadReplyV1::Page {
                    context,
                    entries,
                    continuation,
                },
            ) => {
                let start = match after {
                    None => 0,
                    Some(prior) => {
                        if !Arc::ptr_eq(&prior.owner, &c.marker) {
                            return Ok(false);
                        }
                        let Some(cursor) = c.cursors.get(prior.slot) else {
                            return Ok(false);
                        };
                        if cursor.limit != limit {
                            return Ok(false);
                        }
                        cursor.start
                    }
                };
                let end = start
                    .checked_add(limit)
                    .ok_or(ResourceError::Limit)?
                    .min(c.rows.len());
                context == id
                    && limit > 0
                    && start <= end
                    && entries.len() == end - start
                    && entries.iter().enumerate().all(|(i, e)| {
                        Arc::ptr_eq(&e.reference.owner, &c.marker)
                            && e.reference.slot == start + i
                            && c.entries.get(start + i) == Some(&true)
                            && e.table == c.rows[start + i].address.table
                    })
                    && if end < c.rows.len() {
                        continuation.as_ref().is_some_and(|r| {
                            Arc::ptr_eq(&r.owner, &c.marker)
                                && c.cursors
                                    .get(r.slot)
                                    .is_some_and(|v| v.limit == limit && v.start == end)
                        })
                    } else {
                        continuation.is_none()
                    }
            }
            _ => false,
        })
    }
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    fn catalog_decode_probe(
        &mut self,
        point: usize,
        e: &V<'_, '_>,
        l: CaptureLimitsV1,
        s: &ResourceCatalogReadScopeV1,
        cut: &ResourceCatalogVisibilityIdV1,
        f: &TrustedResourceCatalogReadFactsV1,
        r: &mut Recorder,
    ) {
        let probe = self.catalog_read_pool_mut().unwrap().decode_probe;
        let Some(probe) = probe.filter(|p| p.point == point) else {
            return;
        };
        self.catalog_read_pool_mut().unwrap().decode_probe = None;
        if probe.panic {
            panic!("injected catalog decoder interruption");
        }
        let before = self.retention_usage();
        let reentry = self
            .query_decoded_resource_catalog_read_v1(e, l, s, cut, f, r)
            .is_err();
        let retirement = self.retire_resource_catalog_reader_v1(f, r).is_err()
            && self
                .retire_resource_catalog_visibility_v1(cut, f, r)
                .is_err();
        let unchanged = before == self.retention_usage();
        self.catalog_read_pool_mut().unwrap().decode_probe_report =
            Some((reentry, retirement, unchanged));
    }
}
