//! Owned native F07 normalization over borrowed Rust inputs; not a wire decoder.
//! Opaque roles are never traversed, cloned as data, or resolved from labels.
use super::*;
use crate::capture::{CaptureLimitsV1, InertCaptureV1, InertValueV1 as D};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Clone, Copy)]
pub enum SettingsReadEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, SettingsReadEnvelopeValueV1<'a, 'h>)]),
    Data(D<'a>),
    Context(&'h SettingsReadContextIdV1),
    Target(&'h SettingsReadTargetRefV1),
}
impl std::fmt::Debug for SettingsReadEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SettingsReadEnvelope(<redacted>)")
    }
}
type V<'a, 'h> = SettingsReadEnvelopeValueV1<'a, 'h>;
struct Parsed<'a, 'h> {
    typed: bool,
    operation: &'static str,
    key: Option<&'a str>,
    profile: &'a str,
    context: &'h SettingsReadContextIdV1,
    target: &'h SettingsReadTargetRefV1,
}
/// Owned ordinary meaning plus the two original opaque roles. Native capture
/// units are logical Rust data/workspace bytes, not reference-model JSON bytes.
pub struct NormalizedSettingsReadArgumentsV1<'h> {
    typed: bool,
    operation: &'static str,
    key: Option<String>,
    profile: String,
    scope: SettingsReadScopeV1,
    context: &'h SettingsReadContextIdV1,
    target: &'h SettingsReadTargetRefV1,
    _record: InertCaptureV1,
}
/// Closed recipient result frames. Opaque clones preserve the original native
/// identity; no label resolution or private-layer identity is introduced.
pub enum SettingsReadReplyV1 {
    Value {
        context: SettingsReadContextIdV1,
        target: SettingsReadTargetRefV1,
        key: String,
        value: SettingProjectionValueV1,
    },
    Origins {
        context: SettingsReadContextIdV1,
        target: SettingsReadTargetRefV1,
        key: String,
        origins: SettingsReadOriginsV1,
    },
    Capabilities {
        context: SettingsReadContextIdV1,
        target: SettingsReadTargetRefV1,
        capabilities: Vec<SettingProjectionDeclarationV1>,
    },
}
impl std::fmt::Debug for SettingsReadReplyV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SettingsReadReplyV1(<redacted>)")
    }
}
impl SettingsReadReplyV1 {
    pub const fn coverage(&self) -> &'static str {
        "permitted-only"
    }
}
impl std::fmt::Debug for NormalizedSettingsReadArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedSettingsReadArguments(<redacted>)")
    }
}
impl NormalizedSettingsReadArgumentsV1<'_> {
    pub fn is_typed_route(&self) -> bool {
        self.typed
    }
    pub fn operation(&self) -> SettingsReadOperationV1<'_> {
        match self.operation {
            "effective-configuration" => SettingsReadOperationV1::EffectiveConfiguration {
                key: self.key.as_deref().expect("validated key"),
            },
            "permitted-origin" => SettingsReadOperationV1::PermittedOrigin {
                key: self.key.as_deref().expect("validated key"),
            },
            "capabilities" => SettingsReadOperationV1::Capabilities,
            _ => unreachable!("closed normalized operation"),
        }
    }
}
fn fields<'a, 'h>(
    v: &V<'a, 'h>,
    keys: &[&str],
) -> Result<&'a [(&'a str, V<'a, 'h>)], SettingsError> {
    let V::Object(rows) = v else {
        return Err(SettingsError::Unavailable);
    };
    if rows.len() != keys.len()
        || rows
            .iter()
            .enumerate()
            .any(|(i, (k, _))| !keys.contains(k) || rows[..i].iter().any(|(prior, _)| prior == k))
    {
        return Err(SettingsError::Unavailable);
    }
    Ok(rows)
}
fn get<'a, 'h>(rows: &'a [(&'a str, V<'a, 'h>)], key: &str) -> Result<V<'a, 'h>, SettingsError> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
        .ok_or(SettingsError::Unavailable)
}
fn text<'a>(v: V<'a, '_>) -> Result<&'a str, SettingsError> {
    match v {
        V::Data(D::Text(v)) => Ok(v),
        _ => Err(SettingsError::Unavailable),
    }
}
fn parse<'a, 'h>(envelope: &V<'a, 'h>) -> Result<Parsed<'a, 'h>, SettingsError> {
    let outer = fields(envelope, &["route", "request"])?;
    let typed = match text(get(outer, "route")?)? {
        "typed" => true,
        "dynamic" => false,
        _ => return Err(SettingsError::Unavailable),
    };
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
        text(get(h, "name")?)? == "inspect-context"
    }) || !matches!(get(h, "version")?, V::Data(D::Number(1.0)))
    {
        return Err(SettingsError::Unavailable);
    }
    let operation = match text(get(h, "variant")?)? {
        "effective-configuration" => "effective-configuration",
        "permitted-origin" => "permitted-origin",
        "capabilities" => "capabilities",
        _ => return Err(SettingsError::Unavailable),
    };
    let a = fields(
        &get(h, "arguments")?,
        if operation == "capabilities" {
            &["contract", "operation", "context", "target", "profile"]
        } else {
            &[
                "contract",
                "operation",
                "context",
                "target",
                "profile",
                "key",
            ]
        },
    )?;
    if text(get(a, "contract")?)? != "configuration-query/1"
        || text(get(a, "operation")?)? != operation
    {
        return Err(SettingsError::Unavailable);
    }
    let profile = text(get(a, "profile")?)?;
    if profile.is_empty() {
        return Err(SettingsError::Unavailable);
    }
    let key = if operation == "capabilities" {
        None
    } else {
        let key = text(get(a, "key")?)?;
        if key.is_empty() {
            return Err(SettingsError::Unavailable);
        }
        Some(key)
    };
    let V::Context(context) = get(a, "context")? else {
        return Err(SettingsError::Unavailable);
    };
    let V::Target(target) = get(a, "target")? else {
        return Err(SettingsError::Unavailable);
    };
    Ok(Parsed {
        typed,
        operation,
        key,
        profile,
        context,
        target,
    })
}
fn capture(
    v: D<'_>,
    limits: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<InertCaptureV1, SettingsError> {
    InertCaptureV1::copy(&v, limits, r).map_err(|e| match e {
        crate::capture::CaptureErrorV1::Capacity => SettingsError::Capacity,
        _ => SettingsError::Limit,
    })
}
pub fn normalize_settings_read_envelope_v1<'h>(
    envelope: &V<'_, 'h>,
    scope: &SettingsReadScopeV1,
    limits: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<NormalizedSettingsReadArgumentsV1<'h>, SettingsError> {
    let timing = r.begin_service_timing();
    let result = (|| {
        if limits.max_nodes == 0 {
            return Err(SettingsError::Limit);
        }
        let p = parse(envelope)?;
        let args = [
            ("contract", D::Text("configuration-query/1")),
            ("operation", D::Text(p.operation)),
            ("context", D::Null),
            ("target", D::Null),
            ("profile", D::Text(p.profile)),
            ("key", D::Text(p.key.unwrap_or(""))),
        ];
        let args = &args[..if p.key.is_some() { 6 } else { 5 }];
        let header = [
            (
                if p.typed { "family" } else { "name" },
                if p.typed {
                    D::Number(6.0)
                } else {
                    D::Text("inspect-context")
                },
            ),
            ("variant", D::Text(p.operation)),
            ("version", D::Number(1.0)),
            ("arguments", D::Object(args)),
        ];
        let whole = [
            ("route", D::Text(if p.typed { "typed" } else { "dynamic" })),
            ("request", D::Object(&header)),
        ];
        capture(D::Object(&whole), limits, r)?;
        let c = &scope.execution;
        let scope_rows = [
            ("authority", D::Text(&c.authority)),
            ("engine", D::Text(&c.engine)),
            ("recipient", D::Text(&c.recipient)),
            ("purpose", D::Text(&c.purpose)),
            ("destination", D::Text(&c.destination)),
            ("policy", D::Text(&scope.policy)),
            ("continuity", D::Text(&c.continuity)),
            ("profile", D::Text(&c.profile)),
        ];
        if scope_rows
            .iter()
            .any(|(_, v)| matches!(v, D::Text(s) if s.is_empty()))
        {
            return Err(SettingsError::Unavailable);
        }
        capture(D::Object(&scope_rows), limits, r)?;
        // Capture exact usize limits separately in the caller-owned typed profile;
        // these descriptive numbers must not silently round the canonical meaning.
        if [limits.max_bytes, limits.max_nodes, limits.max_depth]
            .into_iter()
            .any(|n| n as u128 > 9_007_199_254_740_991u128)
        {
            return Err(SettingsError::Limit);
        }
        let limit_rows = [
            ("max-bytes", D::Number(limits.max_bytes as f64)),
            ("max-nodes", D::Number(limits.max_nodes as f64)),
            ("max-depth", D::Number(limits.max_depth as f64)),
        ];
        let roles = [
            D::Text("settings-read-context"),
            D::Text("settings-read-target"),
        ];
        let record = [
            ("request", D::Object(args)),
            ("scope", D::Object(&scope_rows)),
            ("limits", D::Object(&limit_rows)),
            ("opaque-bindings", D::Array(&roles)),
        ];
        let record = capture(D::Object(&record), limits, r)?;
        Ok(NormalizedSettingsReadArgumentsV1 {
            typed: p.typed,
            operation: p.operation,
            key: p.key.map(string).transpose()?,
            profile: string(p.profile)?,
            scope: copy_scope(scope)?,
            context: p.context,
            target: p.target,
            _record: record,
        })
    })();
    crate::settings::observe_service(
        r,
        Boundary::SettingsReadArgumentNormalization,
        &result,
        timing.finish(),
    );
    result
}
/// Fixed native classification lease, not reference-model JSON bytes or RSS.
pub const fn settings_read_decode_classification_bytes_v1() -> usize {
    size_of::<CaptureLimitsV1>()
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SettingsReadDecoderUsageV1 {
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
    pub fn settings_read_decoder_usage_v1(&self) -> SettingsReadDecoderUsageV1 {
        match &self.settings_reader {
            State::Active(p) => SettingsReadDecoderUsageV1 {
                busy: p.decoding,
                declared_workspace_bytes: p.decode_charge,
                faulted: p.decoder_faulted,
            },
            _ => SettingsReadDecoderUsageV1::default(),
        }
    }
    /// Fixed borrowed recognition used before installed metadata release. No
    /// input graph is traversed, authority applied, cut consumed or state issued.
    pub(in crate::coordinator) fn recognize_settings_read_request_v1(
        &self,
        envelope: &V<'_, '_>,
    ) -> Result<&'static str, SettingsError> {
        self.ensure_operational()
            .map_err(|_| SettingsError::Faulted)?;
        let pool = self.settings_reader_pool()?;
        pool.delivery_ready()?;
        if pool.decoding {
            return Err(SettingsError::Unavailable);
        }
        let p = parse(envelope)?;
        if p.profile != SETTINGS_READ_PROFILE_V1 {
            return Err(SettingsError::Unavailable);
        }
        Ok(p.operation)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn query_decoded_settings_read_v1(
        &mut self,
        envelope: &V<'_, '_>,
        limits: CaptureLimitsV1,
        scope: &SettingsReadScopeV1,
        cut: &SettingsReadCutIdV1,
        facts: &TrustedSettingsReadFactsV1,
        r: &mut Recorder,
    ) -> Result<SettingsReadReplyV1, SettingsError> {
        let timing = r.begin_service_timing();
        let result = self.decode_settings_read(envelope, limits, scope, cut, facts, r);
        // A decoder fence belongs to the reader pool, never the execution latch.
        let faulted = self.settings_read_decoder_usage_v1().faulted;
        r.record(Observation {
            boundary: Boundary::SettingsReadDecodedDispatch,
            disposition: if faulted {
                Disposition::Unknown
            } else {
                match &result {
                    Ok(_) => Disposition::Completed,
                    Err(SettingsError::Faulted) => Disposition::Unknown,
                    _ => Disposition::Refused,
                }
            },
            service_time: timing.finish(),
        });
        result
    }
    #[allow(clippy::too_many_arguments)]
    fn decode_settings_read(
        &mut self,
        envelope: &V<'_, '_>,
        limits: CaptureLimitsV1,
        scope: &SettingsReadScopeV1,
        cut: &SettingsReadCutIdV1,
        facts: &TrustedSettingsReadFactsV1,
        r: &mut Recorder,
    ) -> Result<SettingsReadReplyV1, SettingsError> {
        self.ensure_operational()
            .map_err(|_| SettingsError::Faulted)?;
        let pool = self.settings_reader_pool()?;
        pool.delivery_ready()?;
        if pool.decoding {
            return Err(SettingsError::Unavailable);
        }
        let class = settings_read_decode_classification_bytes_v1();
        let mut classification = self
            .retention
            .reserve(class)
            .map_err(retained::parent_error)?;
        let pool = self.settings_reader_mut()?;
        pool.decoding = true;
        pool.decode_charge = class;
        let mut workspace = None;
        let result = catch_unwind(AssertUnwindSafe(|| {
            #[cfg(test)]
            self.decode_probe(0, envelope, limits, scope, cut, facts, r);
            if limits.max_nodes == 0 {
                return Err(SettingsError::Limit);
            }
            let bytes = limits
                .max_bytes
                .checked_mul(7)
                .ok_or(SettingsError::Limit)?;
            workspace = Some(
                self.retention
                    .reserve(bytes)
                    .map_err(retained::parent_error)?,
            );
            self.settings_reader_mut()?.decode_charge =
                class.checked_add(bytes).ok_or(SettingsError::Limit)?;
            let normalized = normalize_settings_read_envelope_v1(envelope, scope, limits, r)?;
            let facts = *facts;
            #[cfg(test)]
            self.decode_probe(1, envelope, limits, scope, cut, &facts, r);
            if normalized.profile != SETTINGS_READ_PROFILE_V1 {
                return Err(SettingsError::Unavailable);
            }
            #[cfg(test)]
            self.decode_probe(2, envelope, limits, scope, cut, &facts, r);
            // Save the conservative native projection charge before its one-use
            // cut is consumed. Inspecting this private bound does not confer
            // authority; only successful native release can produce a frame.
            let frame_bound = if Arc::ptr_eq(&cut.owner, &self.namespace) {
                self.settings_reader_pool()?
                    .rows
                    .get(cut.slot)
                    .and_then(Option::as_ref)
                    .and_then(|row| match row {
                        Row::Cut(cut) => Some(cut.data.logical_bytes),
                        _ => None,
                    })
            } else {
                None
            };
            let out = self.query_settings_read_v1(
                normalized.context,
                normalized.target,
                &normalized.scope,
                normalized.operation(),
                cut,
                &facts,
                r,
            )?;
            let bytes = frame_bound
                .ok_or(SettingsError::Faulted)?
                .checked_add(size_of::<SettingsReadReplyV1>())
                .and_then(|n| n.checked_add(normalized.key.as_ref().map_or(0, String::len)))
                .ok_or(SettingsError::Limit)?;
            if bytes > self.settings_reader_pool()?.limits.max_reply_bytes {
                return Err(SettingsError::Limit);
            }
            let context = normalized.context.clone();
            let target = normalized.target.clone();
            match (normalized.operation, normalized.key, out) {
                ("effective-configuration", Some(key), SettingsReadResultV1::Value(value)) => {
                    Ok(SettingsReadReplyV1::Value {
                        context,
                        target,
                        key,
                        value,
                    })
                }
                ("permitted-origin", Some(key), SettingsReadResultV1::Origins(origins)) => {
                    Ok(SettingsReadReplyV1::Origins {
                        context,
                        target,
                        key,
                        origins,
                    })
                }
                ("capabilities", None, SettingsReadResultV1::Capabilities(capabilities)) => {
                    Ok(SettingsReadReplyV1::Capabilities {
                        context,
                        target,
                        capabilities,
                    })
                }
                _ => {
                    self.faulted = true;
                    Err(SettingsError::Faulted)
                }
            }
        }));
        let result = match result {
            Ok(result) => result,
            Err(_) => {
                if let State::Active(pool) = &mut self.settings_reader {
                    pool.decoder_faulted = true;
                }
                Err(SettingsError::Unavailable)
            }
        };
        // Return exactly each lease, never a stale absolute parent usage counter.
        let workspace_release = workspace
            .as_mut()
            .map(|v| self.retention.release(v))
            .transpose();
        let classification_release = self.retention.release(&mut classification);
        let State::Active(pool) = &mut self.settings_reader else {
            self.faulted = true;
            return Err(SettingsError::Faulted);
        };
        pool.decoding = false;
        pool.decode_charge = 0;
        if workspace_release.is_err() || classification_release.is_err() {
            self.faulted = true;
            return Err(SettingsError::Faulted);
        }
        result
    }
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    fn decode_probe(
        &mut self,
        point: usize,
        envelope: &V<'_, '_>,
        limits: CaptureLimitsV1,
        scope: &SettingsReadScopeV1,
        cut: &SettingsReadCutIdV1,
        facts: &TrustedSettingsReadFactsV1,
        r: &mut Recorder,
    ) {
        let probe = self.settings_reader_mut().unwrap().decode_probe;
        let Some(probe) = probe.filter(|p| p.point == point) else {
            return;
        };
        self.settings_reader_mut().unwrap().decode_probe = None;
        if probe.panic {
            panic!("injected settings decoder interruption");
        }
        let before = self.retention_usage();
        let reentry = self
            .query_decoded_settings_read_v1(envelope, limits, scope, cut, facts, r)
            .is_err();
        let retire = self.retire_settings_reader_v1(r).is_err();
        let unchanged = before == self.retention_usage();
        self.settings_reader_mut().unwrap().decode_probe_report =
            Some((reentry, retire, unchanged));
    }
}
