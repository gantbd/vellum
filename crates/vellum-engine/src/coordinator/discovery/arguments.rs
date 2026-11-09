//! Passive F01 native values, not JSON/FFI or an opaque-identity wire codec.
use super::*;
use crate::capture::{CaptureErrorV1, CaptureUsageV1, CapturedValueV1 as Captured, InertCaptureV1};
use std::mem::size_of;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveryOperationV1 {
    List,
    Inspect,
    AdvisoryEnablement,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeDiscoveryInputFormV1 {
    pub family: usize,
    pub name: &'static str,
    pub version: u64,
    pub contract: &'static str,
    pub operation: DiscoveryOperationV1,
    pub fields: &'static [&'static str],
    pub opaque_roles: &'static [&'static str],
}
/// Closed native reply forms, not a wire encoding of opaque references.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveryResultFormV1 {
    Page,
    Descriptor,
    Advice,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeDiscoveryResultFormV1 {
    pub form: DiscoveryResultFormV1,
    pub name: &'static str,
    pub fields: &'static [(&'static str, &'static str)],
}
pub(super) const SCHEMA_FACETS: &[&str] = &[
    "input",
    "outcomes",
    "requirements",
    "limits",
    "cancellation",
];
pub(super) const PRESENTATION_FIELDS: &[&str] =
    &["title", "description", "category", "aliases", "bindings"];
impl DiscoveryOperationV1 {
    pub fn result_declaration(self) -> NativeDiscoveryResultFormV1 {
        let (form, name, fields): (_, _, &[(&str, &str)]) = match self {
            Self::List => (
                DiscoveryResultFormV1::Page,
                "Page",
                &[
                    ("context", "OriginalDiscoveryContext"),
                    ("coverage", "CapturedPermitted"),
                    (
                        "entries",
                        "BoundedOriginalEntryReferencesWithVersionAndExecutionKind",
                    ),
                    ("continuation", "NullOrOriginalCursorBoundToPageLimit"),
                ],
            ),
            Self::Inspect => (
                DiscoveryResultFormV1::Descriptor,
                "Descriptor",
                &[
                    ("context", "OriginalDiscoveryContext"),
                    (
                        "descriptor",
                        "OriginalEntryVersionKindAndBoundedOriginalVariants;EachSchemaAbsentOrAllFiveFacets;OptionalWhitelistedPresentation",
                    ),
                ],
            ),
            Self::AdvisoryEnablement => (
                DiscoveryResultFormV1::Advice,
                "Advice",
                &[
                    ("context", "OriginalDiscoveryContext"),
                    ("entry", "OriginalEntry"),
                    ("variant", "OriginalVariantOfEntry"),
                    ("target", "OriginalCapturedTarget"),
                    ("enabled", "Boolean"),
                ],
            ),
        };
        NativeDiscoveryResultFormV1 { form, name, fields }
    }
}
impl DiscoveryReplyV1 {
    pub fn result_form(&self) -> DiscoveryResultFormV1 {
        match self {
            Self::Page { .. } => DiscoveryResultFormV1::Page,
            Self::Descriptor { .. } => DiscoveryResultFormV1::Descriptor,
            Self::Advice { .. } => DiscoveryResultFormV1::Advice,
        }
    }
}
impl DiscoveryOperationV1 {
    pub fn name(self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Inspect => "inspect",
            Self::AdvisoryEnablement => "advisory-enablement",
        }
    }
    pub fn declaration(self) -> NativeDiscoveryInputFormV1 {
        let (fields, opaque_roles): (&[&str], &[&str]) = match self {
            Self::List => (
                &[
                    "contract",
                    "operation",
                    "context",
                    "profile",
                    "limit",
                    "continuation",
                ],
                &["context", "continuation"],
            ),
            Self::Inspect => (
                &["contract", "operation", "context", "profile", "entry"],
                &["context", "entry"],
            ),
            Self::AdvisoryEnablement => (
                &[
                    "contract",
                    "operation",
                    "context",
                    "profile",
                    "entry",
                    "variant",
                    "target",
                ],
                &["context", "entry", "variant", "target"],
            ),
        };
        NativeDiscoveryInputFormV1 {
            family: 0,
            name: "discover-contracts",
            version: 1,
            contract: "contract-discovery/1",
            operation: self,
            fields,
            opaque_roles,
        }
    }
}
/// Owner inspection of the actual decoder declarations, not an accepted registry,
/// installed decoder capability, self-discovery profile or recipient-safe result.
pub fn native_discovery_input_forms_v1() -> [NativeDiscoveryInputFormV1; 3] {
    [
        DiscoveryOperationV1::List,
        DiscoveryOperationV1::Inspect,
        DiscoveryOperationV1::AdvisoryEnablement,
    ]
    .map(DiscoveryOperationV1::declaration)
}
impl DiscoveryQueryV1<'_> {
    pub fn operation(self) -> DiscoveryOperationV1 {
        match self {
            Self::List { .. } => DiscoveryOperationV1::List,
            Self::Inspect { .. } => DiscoveryOperationV1::Inspect,
            Self::AdvisoryEnablement { .. } => DiscoveryOperationV1::AdvisoryEnablement,
        }
    }
}
#[derive(Clone, Copy)]
pub enum DiscoveryEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, DiscoveryEnvelopeValueV1<'a, 'h>)]),
    Data(InertValueV1<'a>),
    Context(&'h DiscoveryContextIdV1),
    Entry(&'h DiscoveryEntryIdV1),
    Variant(&'h DiscoveryVariantIdV1),
    Target(&'h DiscoveryTargetIdV1),
    Cursor(&'h DiscoveryCursorIdV1),
}
impl std::fmt::Debug for DiscoveryEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DiscoveryEnvelope(<redacted>)")
    }
}
pub struct NormalizedDiscoveryArgumentsV1<'h> {
    context: &'h DiscoveryContextIdV1,
    scope: DiscoveryScopeV1,
    profile: String,
    query: DiscoveryQueryV1<'h>,
    limits: CaptureLimitsV1,
    usage: [CaptureUsageV1; 3],
    owned_bytes: usize,
}
impl std::fmt::Debug for NormalizedDiscoveryArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedDiscoveryArguments(<redacted>)")
    }
}
impl PartialEq for NormalizedDiscoveryArgumentsV1<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.context == other.context
            && self.scope == other.scope
            && self.profile == other.profile
            && self.query == other.query
            && self.limits == other.limits
    }
}
impl Eq for NormalizedDiscoveryArgumentsV1<'_> {}
impl<'h> NormalizedDiscoveryArgumentsV1<'h> {
    pub fn context(&self) -> &'h DiscoveryContextIdV1 {
        self.context
    }
    pub fn scope(&self) -> &DiscoveryScopeV1 {
        &self.scope
    }
    pub fn profile(&self) -> &str {
        &self.profile
    }
    pub fn query(&self) -> DiscoveryQueryV1<'h> {
        self.query
    }
    pub fn limits(&self) -> CaptureLimitsV1 {
        self.limits
    }
    pub fn capture_usage(&self) -> [CaptureUsageV1; 3] {
        self.usage
    }
    pub fn owned_bytes(&self) -> usize {
        self.owned_bytes
    }
}
type Value<'a, 'h> = DiscoveryEnvelopeValueV1<'a, 'h>;
/// Allocation-free installed selector recognition. Native decoder validation,
/// funding and original-role checks remain mandatory before query execution.
pub(super) fn recognize_operation(v: &Value<'_, '_>) -> Result<DiscoveryOperationV1, Error> {
    let outer = fields(v, &["route", "request"])?;
    let route = text(field(outer, "route")?)?;
    let keys = match route {
        "typed" => ["family", "variant", "version", "arguments"],
        "dynamic" => ["name", "variant", "version", "arguments"],
        _ => return Err(Error::InvalidRequest),
    };
    let header = fields(field(outer, "request")?, &keys)?;
    let variant = text(field(header, "variant")?)?;
    let form = native_discovery_input_forms_v1()
        .into_iter()
        .find(|f| f.operation.name() == variant)
        .ok_or(Error::InvalidRequest)?;
    let selected = match (route, data(field(header, keys[0])?)?) {
        ("typed", InertValueV1::Number(n)) => n == form.family as f64,
        ("dynamic", InertValueV1::Text(n)) => n == form.name,
        _ => false,
    };
    if !selected
        || !matches!(data(field(header, "version")?)?, InertValueV1::Number(n) if n == form.version as f64)
    {
        return Err(Error::InvalidRequest);
    }
    let args = fields(field(header, "arguments")?, form.fields)?;
    if text(field(args, "contract")?)? != form.contract
        || text(field(args, "operation")?)? != form.operation.name()
    {
        return Err(Error::InvalidRequest);
    }
    Ok(form.operation)
}
fn fields<'a, 'h>(
    v: &'a Value<'a, 'h>,
    keys: &[&str],
) -> Result<&'a [(&'a str, Value<'a, 'h>)], Error> {
    let Value::Object(rows) = v else {
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
fn field<'a, 'h>(rows: &'a [(&str, Value<'a, 'h>)], key: &str) -> Result<&'a Value<'a, 'h>, Error> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
        .ok_or(Error::InvalidRequest)
}
fn data<'a>(v: &Value<'a, '_>) -> Result<InertValueV1<'a>, Error> {
    if let Value::Data(v) = v {
        Ok(*v)
    } else {
        Err(Error::InvalidRequest)
    }
}
fn text<'a>(v: &Value<'a, '_>) -> Result<&'a str, Error> {
    if let InertValueV1::Text(v) = data(v)?
        && !v.is_empty()
    {
        Ok(v)
    } else {
        Err(Error::InvalidRequest)
    }
}
pub(super) fn valid_limits(l: CaptureLimitsV1) -> bool {
    [l.max_bytes, l.max_nodes, l.max_depth]
        .into_iter()
        .all(|v| (v as u128) <= 9_007_199_254_740_991)
        && l.max_nodes > 0
        && l.max_depth > 0
}
pub(super) fn template_bytes() -> usize {
    size_of::<[(&str, InertValueV1<'_>); 32]>() + size_of::<[InertValueV1<'_>; 6]>()
}
pub(super) fn copied(
    v: &InertValueV1<'_>,
    l: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<InertCaptureV1, Error> {
    InertCaptureV1::copy(v, l, r).map_err(|e| match e {
        CaptureErrorV1::Limit => Error::Limit,
        CaptureErrorV1::Capacity => Error::Capacity,
        _ => Error::InvalidRequest,
    })
}
fn captured<'a>(c: &'a InertCaptureV1, v: &'a Captured, key: &str) -> Result<&'a Captured, Error> {
    let Captured::Object(rows) = v else {
        return Err(Error::InvalidRequest);
    };
    c.node(
        rows.iter()
            .find(|(k, _)| k == key)
            .ok_or(Error::InvalidRequest)?
            .1,
    )
    .ok_or(Error::Faulted)
}
/// Three independent native bounded copies with null role shadows. Scope policy
/// epoch stays lossless u64; its two-u32 budget shadow is not a wire encoding.
pub fn normalize_discovery_arguments_v1<'a, 'h>(
    envelope: &'a Value<'a, 'h>,
    scope: &DiscoveryScopeV1,
    input: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedDiscoveryArgumentsV1<'h>, Error> {
    let result = normalize(envelope, scope, input, maximum, r);
    observe(r, Boundary::DiscoveryArgumentNormalization, &result);
    result
}
fn normalize<'a, 'h>(
    envelope: &'a Value<'a, 'h>,
    scope: &DiscoveryScopeV1,
    input: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedDiscoveryArgumentsV1<'h>, Error> {
    if !valid_limits(input) || (maximum as u128) > 9_007_199_254_740_991 {
        return Err(Error::InvalidRequest);
    }
    let l = CaptureLimitsV1 {
        max_bytes: input.max_bytes.min(maximum),
        ..input
    };
    if l.max_bytes < template_bytes() {
        return Err(Error::Limit);
    }
    let recognized = recognize_operation(envelope)?;
    let outer = fields(envelope, &["route", "request"])?;
    let route = text(field(outer, "route")?)?;
    let header_keys = match route {
        "typed" => ["family", "variant", "version", "arguments"],
        "dynamic" => ["name", "variant", "version", "arguments"],
        _ => return Err(Error::InvalidRequest),
    };
    let header = fields(field(outer, "request")?, &header_keys)?;
    let variant = text(field(header, "variant")?)?;
    let form = native_discovery_input_forms_v1()
        .into_iter()
        .find(|f| f.operation.name() == variant)
        .ok_or(Error::InvalidRequest)?;
    if form.operation != recognized {
        return Err(Error::Faulted);
    }
    let selected = match (route, data(field(header, header_keys[0])?)?) {
        ("typed", InertValueV1::Number(v)) => v == form.family as f64,
        ("dynamic", InertValueV1::Text(v)) => v == form.name,
        _ => false,
    };
    if !selected
        || !matches!(data(field(header, "version")?)?, InertValueV1::Number(v) if v == form.version as f64)
    {
        return Err(Error::InvalidRequest);
    }
    let args = fields(field(header, "arguments")?, form.fields)?;
    if text(field(args, "contract")?)? != form.contract
        || text(field(args, "operation")?)? != variant
    {
        return Err(Error::InvalidRequest);
    }
    let Value::Context(context) = field(args, "context")? else {
        return Err(Error::InvalidRequest);
    };
    let mut rows = [("", InertValueV1::Null); 7];
    for (i, key) in form.fields.iter().enumerate() {
        rows[i] = (
            key,
            if form.opaque_roles.contains(key) {
                InertValueV1::Null
            } else {
                data(field(args, key)?)?
            },
        );
    }
    let shadow = InertValueV1::Object(&rows[..form.fields.len()]);
    let header_shadow = [
        (header_keys[0], data(field(header, header_keys[0])?)?),
        ("variant", InertValueV1::Text(variant)),
        ("version", InertValueV1::Number(form.version as f64)),
        ("arguments", shadow),
    ];
    let whole = copied(
        &InertValueV1::Object(&[
            ("route", InertValueV1::Text(route)),
            ("request", InertValueV1::Object(&header_shadow)),
        ]),
        l,
        r,
    )?;
    let names = [
        "authority",
        "engine",
        "recipient",
        "purpose",
        "destination",
        "profile",
        "continuity",
    ];
    let mut actor = [("", InertValueV1::Null); 8];
    for (i, value) in settings::basis::labels(&scope.context).iter().enumerate() {
        if value.is_empty() {
            return Err(Error::InvalidRequest);
        }
        actor[i] = (names[i], InertValueV1::Text(value));
    }
    let words = [
        InertValueV1::Number((scope.policy_epoch >> 32) as f64),
        InertValueV1::Number((scope.policy_epoch as u32) as f64),
    ];
    actor[7] = ("policyEpochWords", InertValueV1::Array(&words));
    let bound = copied(&InertValueV1::Object(&actor), l, r)?;
    let limits = [
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
    let mut roles = [InertValueV1::Null; 4];
    for (i, role) in form.opaque_roles.iter().enumerate() {
        roles[i] = InertValueV1::Text(role);
    }
    let canonical = copied(
        &InertValueV1::Object(&[
            ("request", shadow),
            ("context", InertValueV1::Object(&actor)),
            ("limits", InertValueV1::Object(&limits)),
            (
                "opaqueBindings",
                InertValueV1::Array(&roles[..form.opaque_roles.len()]),
            ),
        ]),
        l,
        r,
    )?;
    let request = captured(
        &whole,
        captured(&whole, whole.root(), "request")?,
        "arguments",
    )?;
    let Captured::Text(profile) = captured(&whole, request, "profile")? else {
        return Err(Error::InvalidRequest);
    };
    if profile.is_empty() {
        return Err(Error::InvalidRequest);
    }
    let query = match form.operation {
        DiscoveryOperationV1::List => {
            let Captured::Number(n) = captured(&whole, request, "limit")? else {
                return Err(Error::InvalidRequest);
            };
            if !n.is_finite()
                || *n < 1.0
                || n.fract() != 0.0
                || *n > 9_007_199_254_740_991.0
                || (*n as u128) > usize::MAX as u128
            {
                return Err(Error::InvalidRequest);
            }
            let continuation = match field(args, "continuation")? {
                Value::Data(InertValueV1::Null) => None,
                Value::Cursor(v) => Some(*v),
                _ => return Err(Error::InvalidRequest),
            };
            DiscoveryQueryV1::List {
                limit: *n as usize,
                continuation,
            }
        }
        DiscoveryOperationV1::Inspect => {
            let Value::Entry(entry) = field(args, "entry")? else {
                return Err(Error::InvalidRequest);
            };
            DiscoveryQueryV1::Inspect { entry }
        }
        DiscoveryOperationV1::AdvisoryEnablement => {
            let (Value::Entry(entry), Value::Variant(variant), Value::Target(target)) = (
                field(args, "entry")?,
                field(args, "variant")?,
                field(args, "target")?,
            ) else {
                return Err(Error::InvalidRequest);
            };
            DiscoveryQueryV1::AdvisoryEnablement {
                entry,
                variant,
                target,
            }
        }
    };
    if query.operation() != form.operation {
        return Err(Error::Faulted);
    }
    let owned_bytes = settings::basis::labels(&scope.context)
        .into_iter()
        .map(str::len)
        .chain([profile.len()])
        .try_fold(size_of::<NormalizedDiscoveryArgumentsV1<'_>>(), |a, b| {
            a.checked_add(b).ok_or(Error::Limit)
        })?;
    if owned_bytes > l.max_bytes {
        return Err(Error::Limit);
    }
    let profile = {
        let mut v = String::new();
        v.try_reserve_exact(profile.len())
            .map_err(|_| Error::Capacity)?;
        v.push_str(profile);
        v
    };
    Ok(NormalizedDiscoveryArgumentsV1 {
        context,
        scope: ownership_scope_copy(scope)?,
        profile,
        query,
        limits: input,
        usage: [whole.usage(), bound.usage(), canonical.usage()],
        owned_bytes,
    })
}
fn ownership_scope_copy(scope: &DiscoveryScopeV1) -> Result<DiscoveryScopeV1, Error> {
    Ok(DiscoveryScopeV1 {
        context: settings::basis::copy_context(&scope.context).map_err(|_| Error::Capacity)?,
        policy_epoch: scope.policy_epoch,
    })
}
