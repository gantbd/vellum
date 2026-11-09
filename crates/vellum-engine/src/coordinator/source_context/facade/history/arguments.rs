//! Passive native decoded values with original opaque history references. No wire
//! codec, parser, arbitrary object/getter execution or public acquisition syntax.
use super::*;
use crate::capture::{CaptureUsageV1, CapturedValueV1};
#[cfg(test)]
pub(in crate::coordinator) mod tests;
#[derive(Clone, Copy)]
pub enum HistoryEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, HistoryEnvelopeValueV1<'a, 'h>)]),
    Data(InertValueV1<'a>),
    Context(&'h HistoryInspectionIdV1),
}
impl std::fmt::Debug for HistoryEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HistoryEnvelopeValueV1(<redacted>)")
    }
}
#[derive(Clone, Copy)]
pub struct DecodedHistoryFacadeRequestV1<'a, 'h> {
    pub envelope: &'a HistoryEnvelopeValueV1<'a, 'h>,
    pub actor: &'a HistoryInspectionScopeV1,
    pub limits: CaptureLimitsV1,
    pub facts: &'a TrustedHistoryInspectionFactsV1<'a>,
}
impl std::fmt::Debug for DecodedHistoryFacadeRequestV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DecodedHistoryFacadeRequestV1(<redacted>)")
    }
}
pub struct NormalizedHistoryArgumentsV1<'a> {
    context: &'a HistoryInspectionIdV1,
    actor: HistoryInspectionScopeV1,
    query: CopiedQuery,
    profile: String,
    limits: CaptureLimitsV1,
    usage: [CaptureUsageV1; 3],
    owned_bytes: usize,
}
impl std::fmt::Debug for NormalizedHistoryArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedHistoryArgumentsV1(<redacted>)")
    }
}
impl PartialEq for NormalizedHistoryArgumentsV1<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.context == other.context
            && self.actor == other.actor
            && self.query.query() == other.query.query()
            && self.profile == other.profile
            && self.limits == other.limits
    }
}
impl Eq for NormalizedHistoryArgumentsV1<'_> {}
impl<'a> NormalizedHistoryArgumentsV1<'a> {
    pub fn context(&self) -> &'a HistoryInspectionIdV1 {
        self.context
    }
    pub fn actor(&self) -> &HistoryInspectionScopeV1 {
        &self.actor
    }
    pub fn query(&self) -> HistoryInspectionQueryV1<'_> {
        self.query.query()
    }
    pub fn profile(&self) -> &str {
        &self.profile
    }
    pub fn limits(&self) -> CaptureLimitsV1 {
        self.limits
    }
    /// Native logical copy bounds, not JSON bytes, parent reservation or authority.
    pub fn capture_usage(&self) -> [CaptureUsageV1; 3] {
        self.usage
    }
    pub fn owned_bytes(&self) -> usize {
        self.owned_bytes
    }
}
const COMMON: [&str; 6] = [
    "contract",
    "operation",
    "context",
    "scope",
    "direction",
    "profile",
];
/// Fixed header classification only; no history payload or native admission.
pub(in crate::coordinator) fn installed_selector(
    envelope: &HistoryEnvelopeValueV1<'_, '_>,
) -> Result<&'static str, Error> {
    let outer = fields(envelope, &["route", "request"])?;
    let route = text(field(outer, "route")?)?;
    let keys = match route {
        "typed" => ["family", "variant", "version", "arguments"],
        "dynamic" => ["name", "variant", "version", "arguments"],
        _ => return Err(Error::InvalidRequest),
    };
    let header = fields(field(outer, "request")?, &keys)?;
    if !matches!(
        (route, data(field(header, keys[0])?)?),
        ("typed", InertValueV1::Number(3.0)) | ("dynamic", InertValueV1::Text("inspect-history"))
    ) || !matches!(data(field(header, "version")?)?, InertValueV1::Number(1.0))
    {
        return Err(Error::InvalidRequest);
    }
    match text(field(header, "variant")?)? {
        "frontier" => Ok("frontier"),
        "availability" => Ok("availability"),
        "permitted-blocking-detail" => Ok("permitted-blocking-detail"),
        _ => Err(Error::InvalidRequest),
    }
}
fn fields<'a, 'h>(
    value: &'a HistoryEnvelopeValueV1<'a, 'h>,
    keys: &[&str],
) -> Result<&'a [(&'a str, HistoryEnvelopeValueV1<'a, 'h>)], Error> {
    let HistoryEnvelopeValueV1::Object(rows) = value else {
        return Err(Error::InvalidRequest);
    };
    if rows.len() != keys.len()
        || rows.iter().any(|(k, _)| !keys.contains(k))
        || rows
            .iter()
            .enumerate()
            .any(|(i, (k, _))| rows[..i].iter().any(|(old, _)| old == k))
    {
        return Err(Error::InvalidRequest);
    }
    Ok(rows)
}
fn field<'a, 'h>(
    rows: &'a [(&'a str, HistoryEnvelopeValueV1<'a, 'h>)],
    key: &str,
) -> Result<&'a HistoryEnvelopeValueV1<'a, 'h>, Error> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
        .ok_or(Error::InvalidRequest)
}
fn data<'a, 'h>(v: &HistoryEnvelopeValueV1<'a, 'h>) -> Result<InertValueV1<'a>, Error> {
    match v {
        HistoryEnvelopeValueV1::Data(v) => Ok(*v),
        _ => Err(Error::InvalidRequest),
    }
}
fn text<'a, 'h>(v: &HistoryEnvelopeValueV1<'a, 'h>) -> Result<&'a str, Error> {
    match data(v)? {
        InertValueV1::Text(s) => Ok(s),
        _ => Err(Error::InvalidRequest),
    }
}
fn captured_field<'a>(
    capture: &'a InertCaptureV1,
    object: &'a CapturedValueV1,
    key: &str,
) -> Result<&'a CapturedValueV1, Error> {
    let CapturedValueV1::Object(rows) = object else {
        return Err(Error::Faulted);
    };
    capture
        .node(rows.iter().find(|(k, _)| k == key).ok_or(Error::Faulted)?.1)
        .ok_or(Error::Faulted)
}
fn captured_text(v: &CapturedValueV1) -> Result<&str, Error> {
    match v {
        CapturedValueV1::Text(s) if !s.is_empty() => Ok(s),
        _ => Err(Error::InvalidRequest),
    }
}
fn normalization_template_bytes() -> usize {
    // Simultaneously live fixed outer/header/argument/actor/limit/canonical
    // templates and opaque-binding marker. No caller-sized temporary row vector.
    28 * size_of::<(&str, InertValueV1<'_>)>() + size_of::<[InertValueV1<'_>; 1]>()
}
/// Pure bounded argument normalization. Parent admission and actual owner/current
/// grants are separate; normalized equality grants neither history nor reversal.
pub fn normalize_history_envelope_v1<'a, 'h>(
    envelope: &'a HistoryEnvelopeValueV1<'a, 'h>,
    actor: &HistoryInspectionScopeV1,
    limits: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedHistoryArgumentsV1<'h>, Error> {
    let out = normalize(envelope, actor, limits, maximum, r);
    observation(
        r,
        Boundary::HistoryArgumentNormalization,
        if matches!(&out, Err(Error::Faulted)) {
            Disposition::Unknown
        } else if out.is_ok() {
            Disposition::Completed
        } else {
            Disposition::Refused
        },
    );
    out
}
fn normalize<'a, 'h>(
    envelope: &'a HistoryEnvelopeValueV1<'a, 'h>,
    actor: &HistoryInspectionScopeV1,
    input: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedHistoryArgumentsV1<'h>, Error> {
    let safe = |n: usize| (n as u128) <= 9_007_199_254_740_991;
    if !safe(input.max_bytes)
        || !safe(input.max_nodes)
        || !safe(input.max_depth)
        || !safe(maximum)
        || input.max_nodes == 0
    {
        return Err(Error::InvalidRequest);
    }
    let limits = CaptureLimitsV1 {
        max_bytes: input.max_bytes.min(maximum),
        ..input
    };
    if normalization_template_bytes() > limits.max_bytes {
        return Err(Error::Limit);
    }
    let outer = fields(envelope, &["route", "request"])?;
    let route = text(field(outer, "route")?)?;
    let header_keys = match route {
        "typed" => ["family", "variant", "version", "arguments"],
        "dynamic" => ["name", "variant", "version", "arguments"],
        _ => return Err(Error::InvalidRequest),
    };
    let header = fields(field(outer, "request")?, &header_keys)?;
    let selector = data(field(header, header_keys[0])?)?;
    if !matches!(
        (route, selector),
        ("typed", InertValueV1::Number(3.0)) | ("dynamic", InertValueV1::Text("inspect-history"))
    ) || !matches!(data(field(header, "version")?)?, InertValueV1::Number(1.0))
    {
        return Err(Error::InvalidRequest);
    }
    let variant = text(field(header, "variant")?)?;
    let (kind, keys): (_, &[&str]) = match variant {
        "frontier" => (Kind::Frontier, &COMMON),
        "availability" => (
            Kind::Availability,
            &[
                "contract",
                "operation",
                "context",
                "scope",
                "direction",
                "profile",
                "group",
            ],
        ),
        "permitted-blocking-detail" => (
            Kind::Detail,
            &[
                "contract",
                "operation",
                "context",
                "scope",
                "direction",
                "profile",
                "group",
            ],
        ),
        _ => return Err(Error::InvalidRequest),
    };
    let args = fields(field(header, "arguments")?, keys)?;
    if text(field(args, "contract")?)? != "history-query/1"
        || text(field(args, "operation")?)? != variant
    {
        return Err(Error::InvalidRequest);
    }
    let HistoryEnvelopeValueV1::Context(context) = field(args, "context")? else {
        return Err(Error::InvalidRequest);
    };
    // Fixed declaration order canonicalizes set-like fields; handle remains the
    // original borrowed capability and only null enters ordinary shadow data.
    let mut arguments = [("", InertValueV1::Null); 7];
    for (i, key) in keys.iter().enumerate() {
        arguments[i] = (
            key,
            if *key == "context" {
                InertValueV1::Null
            } else {
                data(field(args, key)?)?
            },
        );
    }
    let argument_object = InertValueV1::Object(&arguments[..keys.len()]);
    let request = [
        (header_keys[0], selector),
        ("variant", InertValueV1::Text(variant)),
        ("version", InertValueV1::Number(1.0)),
        ("arguments", argument_object),
    ];
    let envelope_shadow = [
        ("route", InertValueV1::Text(route)),
        ("request", InertValueV1::Object(&request)),
    ];
    let whole = frame(&InertValueV1::Object(&envelope_shadow), limits, r)?;
    let actor_fields = scope_fields(actor);
    if actor_fields
        .iter()
        .any(|(_, v)| matches!(v,InertValueV1::Text(s) if s.is_empty()))
    {
        return Err(Error::InvalidRequest);
    }
    let actor_shadow = frame(&InertValueV1::Object(&actor_fields), limits, r)?;
    let limit_fields = [
        (
            "maxRequestBytes",
            InertValueV1::Number(input.max_bytes as f64),
        ),
        (
            "maxInputNodes",
            InertValueV1::Number(input.max_nodes as f64),
        ),
        (
            "maxInputDepth",
            InertValueV1::Number(input.max_depth as f64),
        ),
    ];
    let bindings = [InertValueV1::Text("history-context")];
    let canonical = [
        ("request", argument_object),
        ("context", InertValueV1::Object(&actor_fields)),
        ("limits", InertValueV1::Object(&limit_fields)),
        ("opaqueBindings", InertValueV1::Array(&bindings)),
    ];
    let canonical_shadow = frame(&InertValueV1::Object(&canonical), limits, r)?;
    let captured_request = captured_field(
        &whole,
        captured_field(&whole, whole.root(), "request")?,
        "arguments",
    )?;
    let scope = captured_text(captured_field(&whole, captured_request, "scope")?)?;
    let profile = captured_text(captured_field(&whole, captured_request, "profile")?)?;
    let direction = match captured_text(captured_field(&whole, captured_request, "direction")?)? {
        "undo" => HistoryDirection::Undo,
        "redo" => HistoryDirection::Redo,
        _ => return Err(Error::InvalidRequest),
    };
    let group = if matches!(kind, Kind::Frontier) {
        None
    } else {
        match captured_field(&whole, captured_request, "group")? {
            CapturedValueV1::Null => None,
            value => Some(captured_text(value)?),
        }
    };
    let bytes = size_of::<NormalizedHistoryArgumentsV1<'_>>()
        .checked_add(native::scope_bytes(actor).map_err(native_error)?)
        .and_then(|n| {
            n.checked_add(scope.len())?
                .checked_add(profile.len())?
                .checked_add(group.map_or(0, str::len))
        })
        .ok_or(Error::Limit)?;
    if bytes > limits.max_bytes {
        return Err(Error::Limit);
    }
    Ok(NormalizedHistoryArgumentsV1 {
        context,
        actor: native::copy_scope(actor).map_err(native_error)?,
        query: CopiedQuery {
            kind,
            direction,
            scope: owned(scope)?,
            group: group.map(owned).transpose()?,
        },
        profile: owned(profile)?,
        limits: input,
        usage: [
            whole.usage(),
            actor_shadow.usage(),
            canonical_shadow.usage(),
        ],
        owned_bytes: bytes,
    })
}
impl SourceCoordinator {
    pub fn query_decoded_history_facade_v1(
        &mut self,
        request: DecodedHistoryFacadeRequestV1<'_, '_>,
        r: &mut Recorder,
    ) -> Option<HistoryInspectionReplyV1> {
        self.try_query_decoded_history_facade_v1(request, r).ok()
    }
    /// Explicit native adapter outcome. Authority refusals remain indistinguishable;
    /// late faults do not imply rollback or discharge of retained obligations.
    pub fn try_query_decoded_history_facade_v1(
        &mut self,
        request: DecodedHistoryFacadeRequestV1<'_, '_>,
        r: &mut Recorder,
    ) -> Result<HistoryInspectionReplyV1, SourceReadContextErrorV1> {
        lifetime::execute(self, Family::History, r, |gateway| {
            gateway.dispatch_decoded_history(request)
        })
    }
}
impl Gateway<'_> {
    fn dispatch_decoded_history(
        &mut self,
        request: DecodedHistoryFacadeRequestV1<'_, '_>,
    ) -> Result<HistoryInspectionReplyV1, Error> {
        if let Err(e) = self.admit(request.limits) {
            observation(
                self.recorder,
                Boundary::HistoryFacadeAdmission,
                if e == Error::Faulted {
                    Disposition::Unknown
                } else {
                    Disposition::Refused
                },
            );
            return Err(e);
        }
        #[cfg(test)]
        if self.owner.source_facade_fault == Some(Fault::Reentry) {
            assert!(
                self.owner
                    .query_decoded_history_facade_v1(request, self.recorder)
                    .is_none()
            );
        }
        let n = normalize_history_envelope_v1(
            request.envelope,
            request.actor,
            request.limits,
            request.limits.max_bytes,
            self.recorder,
        )?;
        // Unsupported but well-formed profiles survive pure normalization, then
        // refuse before any current-loss observation. Never rewrite to the owner.
        if n.profile != n.actor.context.profile {
            return Err(Error::Unavailable);
        }
        self.dispatch_history_admitted(HistoryFacadeRequestV1 {
            context: n.context,
            scope: &n.actor,
            actor: &n.actor,
            query: n.query.query(),
            limits: request.limits,
            facts: request.facts,
        })
    }
}
