//! Native F02 argument binding. Documents and snapshots are borrowed capabilities,
//! not serialized identity records. No wire/JSON/hostile-object codec is implied.
use super::*;
use crate::capture::{CaptureUsageV1, CapturedValueV1};
#[cfg(test)]
mod tests;

#[derive(Clone, Copy)]
pub enum SourceEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, SourceEnvelopeValueV1<'a, 'h>)]),
    Data(InertValueV1<'a>),
    Snapshot(SearchReadContextV1<'h>),
    Document(&'h DocumentId),
}
impl std::fmt::Debug for SourceEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceEnvelopeValueV1(<redacted>)")
    }
}
#[derive(Clone, Copy)]
pub struct DecodedSourceFacadeRequestV1<'a, 'h> {
    pub envelope: &'a SourceEnvelopeValueV1<'a, 'h>,
    pub actor: &'a SettingsExecutionContextV1,
    pub scope: &'a SourceReadScopeV1,
    pub limits: CaptureLimitsV1,
    pub facts: SearchReadFactsV1<'a>,
}
impl std::fmt::Debug for DecodedSourceFacadeRequestV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DecodedSourceFacadeRequestV1(<redacted>)")
    }
}
pub struct NormalizedSourceArgumentsV1<'h> {
    source: SearchReadContextV1<'h>,
    document: &'h DocumentId,
    actor: SettingsExecutionContextV1,
    profile: String,
    query: SourceContextQueryV1,
    limits: CaptureLimitsV1,
    usage: [CaptureUsageV1; 3],
    owned_bytes: usize,
}
impl std::fmt::Debug for NormalizedSourceArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedSourceArgumentsV1(<redacted>)")
    }
}
impl PartialEq for NormalizedSourceArgumentsV1<'_> {
    fn eq(&self, other: &Self) -> bool {
        let same_source = match (self.source, other.source) {
            (SearchReadContextV1::Source(a), SearchReadContextV1::Source(b)) => a == b,
            (SearchReadContextV1::Combined(a), SearchReadContextV1::Combined(b)) => a == b,
            (SearchReadContextV1::VersionSet(a), SearchReadContextV1::VersionSet(b)) => a == b,
            _ => false,
        };
        same_source
            && self.document == other.document
            && self.actor == other.actor
            && self.profile == other.profile
            && self.query == other.query
            && self.limits == other.limits
    }
}
impl Eq for NormalizedSourceArgumentsV1<'_> {}
impl<'h> NormalizedSourceArgumentsV1<'h> {
    pub fn source(&self) -> SearchReadContextV1<'h> {
        self.source
    }
    pub fn document(&self) -> &'h DocumentId {
        self.document
    }
    pub fn actor(&self) -> &SettingsExecutionContextV1 {
        &self.actor
    }
    pub fn profile(&self) -> &str {
        &self.profile
    }
    pub fn query(&self) -> SourceContextQueryV1 {
        self.query
    }
    pub fn limits(&self) -> CaptureLimitsV1 {
        self.limits
    }
    /// Logical native copy usage, not wire bytes or physical allocation evidence.
    pub fn capture_usage(&self) -> [CaptureUsageV1; 3] {
        self.usage
    }
    pub fn owned_bytes(&self) -> usize {
        self.owned_bytes
    }
}
fn fields<'a, 'h>(
    v: &'a SourceEnvelopeValueV1<'a, 'h>,
    keys: &[&str],
) -> Result<&'a [(&'a str, SourceEnvelopeValueV1<'a, 'h>)], Error> {
    let SourceEnvelopeValueV1::Object(rows) = v else {
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
    rows: &'a [(&'a str, SourceEnvelopeValueV1<'a, 'h>)],
    key: &str,
) -> Result<&'a SourceEnvelopeValueV1<'a, 'h>, Error> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
        .ok_or(Error::InvalidRequest)
}
fn data<'a>(v: &SourceEnvelopeValueV1<'a, '_>) -> Result<InertValueV1<'a>, Error> {
    match v {
        SourceEnvelopeValueV1::Data(v) => Ok(*v),
        _ => Err(Error::InvalidRequest),
    }
}
fn text<'a>(v: &SourceEnvelopeValueV1<'a, '_>) -> Result<&'a str, Error> {
    match data(v)? {
        InertValueV1::Text(s) if !s.is_empty() => Ok(s),
        _ => Err(Error::InvalidRequest),
    }
}
/// Fixed-size header classification only; the funded decoder remains mandatory.
pub(in crate::coordinator) fn installed_selector(
    envelope: &SourceEnvelopeValueV1<'_, '_>,
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
        ("typed", InertValueV1::Number(1.0)) | ("dynamic", InertValueV1::Text("read-document"))
    ) || !matches!(data(field(header, "version")?)?, InertValueV1::Number(1.0))
    {
        return Err(Error::InvalidRequest);
    }
    match text(field(header, "variant")?)? {
        "bounded-range" => Ok("bounded-range"),
        "line" => Ok("line"),
        _ => Err(Error::InvalidRequest),
    }
}
pub(super) fn captured<'a>(
    c: &'a InertCaptureV1,
    object: &'a CapturedValueV1,
    key: &str,
) -> Result<&'a CapturedValueV1, Error> {
    let CapturedValueV1::Object(rows) = object else {
        return Err(Error::Faulted);
    };
    c.node(rows.iter().find(|(k, _)| k == key).ok_or(Error::Faulted)?.1)
        .ok_or(Error::Faulted)
}
pub(super) fn natural(v: &CapturedValueV1) -> Result<usize, Error> {
    match v {
        CapturedValueV1::Number(n)
            if n.is_finite()
                && *n >= 0.0
                && n.fract() == 0.0
                && *n <= 9_007_199_254_740_991.0
                && (*n as u128) <= usize::MAX as u128 =>
        {
            Ok(*n as usize)
        }
        _ => Err(Error::InvalidRequest),
    }
}
/// Pure native binding, not authority. Both opaque roles remain original borrowed
/// references; ordinary values are owned and bounded. Wire document triples are
/// deliberately not accepted until a separate identity codec is selected.
pub fn normalize_source_envelope_v1<'a, 'h>(
    envelope: &'a SourceEnvelopeValueV1<'a, 'h>,
    actor: &SettingsExecutionContextV1,
    input: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedSourceArgumentsV1<'h>, Error> {
    let out = normalize(envelope, actor, input, maximum, r);
    observation(
        r,
        Boundary::SourceArgumentNormalization,
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
    envelope: &'a SourceEnvelopeValueV1<'a, 'h>,
    actor: &SettingsExecutionContextV1,
    input: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedSourceArgumentsV1<'h>, Error> {
    let safe = |n: usize| (n as u128) <= 9_007_199_254_740_991;
    if [input.max_bytes, input.max_nodes, input.max_depth, maximum]
        .iter()
        .any(|n| !safe(*n))
        || input.max_nodes == 0
    {
        return Err(Error::InvalidRequest);
    }
    let limits = CaptureLimitsV1 {
        max_bytes: input.max_bytes.min(maximum),
        ..input
    };
    // Fixed live templates: outer2/header4/args7/actor7/limits3/canonical4/bindings2.
    if 27 * size_of::<(&str, InertValueV1<'_>)>() + size_of::<[InertValueV1<'_>; 2]>()
        > limits.max_bytes
    {
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
        ("typed", InertValueV1::Number(1.0)) | ("dynamic", InertValueV1::Text("read-document"))
    ) || !matches!(data(field(header, "version")?)?, InertValueV1::Number(1.0))
    {
        return Err(Error::InvalidRequest);
    }
    let variant = text(field(header, "variant")?)?;
    let keys: &[&str] = match variant {
        "bounded-range" => &[
            "contract",
            "operation",
            "snapshot",
            "document",
            "profile",
            "lo",
            "hi",
        ],
        "line" => &[
            "contract",
            "operation",
            "snapshot",
            "document",
            "profile",
            "line",
        ],
        _ => return Err(Error::InvalidRequest),
    };
    let args = fields(field(header, "arguments")?, keys)?;
    if text(field(args, "contract")?)? != "source-query/1"
        || text(field(args, "operation")?)? != variant
    {
        return Err(Error::InvalidRequest);
    }
    let SourceEnvelopeValueV1::Snapshot(source) = field(args, "snapshot")? else {
        return Err(Error::InvalidRequest);
    };
    let SourceEnvelopeValueV1::Document(document) = field(args, "document")? else {
        return Err(Error::InvalidRequest);
    };
    let mut arguments = [("", InertValueV1::Null); 7];
    for (i, key) in keys.iter().enumerate() {
        arguments[i] = (
            key,
            if matches!(*key, "snapshot" | "document") {
                InertValueV1::Null
            } else {
                data(field(args, key)?)?
            },
        );
    }
    let argument_object = InertValueV1::Object(&arguments[..keys.len()]);
    let header_shadow = [
        (header_keys[0], selector),
        ("variant", InertValueV1::Text(variant)),
        ("version", InertValueV1::Number(1.0)),
        ("arguments", argument_object),
    ];
    let outer_shadow = [
        ("route", InertValueV1::Text(route)),
        ("request", InertValueV1::Object(&header_shadow)),
    ];
    let whole = frame(&InertValueV1::Object(&outer_shadow), limits, r)?;
    let actor_fields = labels(actor);
    if actor_fields
        .iter()
        .any(|(_, v)| matches!(v,InertValueV1::Text(s) if s.is_empty()))
    {
        return Err(Error::InvalidRequest);
    }
    let bound = frame(&InertValueV1::Object(&actor_fields), limits, r)?;
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
    let bindings = [
        InertValueV1::Text("source-snapshot"),
        InertValueV1::Text("source-document"),
    ];
    let canonical = [
        ("request", argument_object),
        ("context", InertValueV1::Object(&actor_fields)),
        ("limits", InertValueV1::Object(&limit_fields)),
        ("opaqueBindings", InertValueV1::Array(&bindings)),
    ];
    let canonical = frame(&InertValueV1::Object(&canonical), limits, r)?;
    let request = captured(
        &whole,
        captured(&whole, whole.root(), "request")?,
        "arguments",
    )?;
    let CapturedValueV1::Text(profile) = captured(&whole, request, "profile")? else {
        return Err(Error::InvalidRequest);
    };
    if profile.is_empty() {
        return Err(Error::InvalidRequest);
    }
    let query = if variant == "line" {
        SourceContextQueryV1::Line {
            index: natural(captured(&whole, request, "line")?)?,
        }
    } else {
        let lo = natural(captured(&whole, request, "lo")?)?;
        let hi = natural(captured(&whole, request, "hi")?)?;
        if lo > hi {
            return Err(Error::InvalidRequest);
        }
        SourceContextQueryV1::Range {
            start: BytePosition(lo),
            end: BytePosition(hi),
        }
    };
    let bytes = labels(actor)
        .iter()
        .try_fold(size_of::<NormalizedSourceArgumentsV1<'_>>(), |n, (_, v)| {
            let InertValueV1::Text(s) = v else {
                return None;
            };
            n.checked_add(s.len())
        })
        .and_then(|n| n.checked_add(profile.len()))
        .ok_or(Error::Limit)?;
    if bytes > limits.max_bytes {
        return Err(Error::Limit);
    }
    Ok(NormalizedSourceArgumentsV1 {
        source: *source,
        document,
        actor: settings::basis::copy_context(actor).map_err(|_| Error::Capacity)?,
        profile: owned(profile)?,
        query,
        limits: input,
        usage: [whole.usage(), bound.usage(), canonical.usage()],
        owned_bytes: bytes,
    })
}
impl SourceCoordinator {
    pub fn query_decoded_source_facade_v1(
        &mut self,
        request: DecodedSourceFacadeRequestV1<'_, '_>,
        r: &mut Recorder,
    ) -> Option<SourceFacadeReplyV1> {
        self.try_query_decoded_source_facade_v1(request, r).ok()
    }
    /// Explicit native adapter outcome. Authority refusals remain indistinguishable;
    /// late faults do not imply rollback or discharge of retained obligations.
    pub fn try_query_decoded_source_facade_v1(
        &mut self,
        request: DecodedSourceFacadeRequestV1<'_, '_>,
        r: &mut Recorder,
    ) -> Result<SourceFacadeReplyV1, SourceReadContextErrorV1> {
        lifetime::execute(self, Family::Source, r, |gateway| {
            gateway.dispatch_decoded_source(request)
        })
    }
}
impl Gateway<'_> {
    fn dispatch_decoded_source(
        &mut self,
        request: DecodedSourceFacadeRequestV1<'_, '_>,
    ) -> Result<SourceFacadeReplyV1, Error> {
        if let Err(e) = self.admit(request.limits) {
            observation(
                self.recorder,
                Boundary::SourceFacadeAdmission,
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
                    .query_decoded_source_facade_v1(request, self.recorder)
                    .is_none()
            );
        }
        let n = normalize_source_envelope_v1(
            request.envelope,
            request.actor,
            request.limits,
            request
                .limits
                .max_bytes
                .min(self.owner.settings_job_parent_limit()),
            self.recorder,
        )?;
        // Malformed/unsupported targeting must never lend a current-loss packet
        // to a different native context. Actor labels do not replace read scope.
        if n.document != &request.scope.document || n.profile != request.scope.context.profile {
            return Err(Error::Unavailable);
        }
        self.dispatch_source_admitted(SourceFacadeRequestV1 {
            source: n.source,
            scope: request.scope,
            actor: &n.actor,
            query: n.query,
            limits: request.limits,
            facts: request.facts,
        })
    }
}
