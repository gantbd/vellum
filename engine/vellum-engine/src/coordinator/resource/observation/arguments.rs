//! Passive native F06 values. Opaque capabilities are never traversed or encoded.
use super::*;
use crate::capture::{
    CaptureErrorV1, CaptureLimitsV1, CaptureUsageV1, CapturedValueV1, InertCaptureV1, InertValueV1,
};
macro_rules! declaration {
    ($contract:literal, $operation:literal $(, $field:literal => $role:literal)*) => {
        NativeArgumentDeclarationV1 {
            contract: $contract,
            variant: NativeHandlerVariantDeclarationV1 {
                operation: $operation,
                fields: &[("contract", "RegisteredContractVersion"), ("operation", "ClosedOperation"),
                    ("task", "OriginalNativeOperationTask"), ("context", "OriginalNativeOperationObserver"),
                    ("profile", "ExactAdmittedProfile"), $(($field, $role)),*],
            },
        }
    };
}
static FORMS: [NativeArgumentDeclarationV1; 4] = [
    declaration!("native-operation-observation/1", "status"),
    declaration!("native-operation-receipt/1", "retained-outcome", "facet" => "ExactNativeKind"),
    declaration!("native-operation-observation/1", "minimal-control-status"),
    declaration!("native-operation-observation/1", "bounded-observation-wait", "after" => "OriginalOrdinaryProgress", "budget" => "PositiveSafeInteger"),
];
pub fn native_operation_argument_declarations_v1() -> [&'static NativeArgumentDeclarationV1; 4] {
    [&FORMS[0], &FORMS[1], &FORMS[2], &FORMS[3]]
}
/// Allocation-free installed-route recognition; original roles/current native
/// release still belong to the separately funded decoder and observation owner.
pub fn recognize_native_operation_envelope_v1(
    v: &NativeOperationEnvelopeValueV1<'_, '_>,
) -> Result<&'static NativeArgumentDeclarationV1, NativeOperationObservationErrorV1> {
    let outer = fields(v, &["route", "request"])?;
    let route = text(field(outer, "route")?)?;
    let keys = match route {
        "typed" => ["family", "variant", "version", "arguments"],
        "dynamic" => ["name", "variant", "version", "arguments"],
        _ => return Err(Error::InvalidRequest),
    };
    let header = fields(field(outer, "request")?, &keys)?;
    if !matches!(
        (route, data(field(header, keys[0])?)?),
        ("typed", InertValueV1::Number(5.0)) | ("dynamic", InertValueV1::Text("inspect-execution"))
    ) || !matches!(data(field(header, "version")?)?, InertValueV1::Number(1.0))
    {
        return Err(Error::InvalidRequest);
    }
    let operation = text(field(header, "variant")?)?;
    let d = native_operation_argument_declarations_v1()
        .into_iter()
        .find(|d| d.variant.operation == operation)
        .ok_or(Error::InvalidRequest)?;
    let NativeOperationEnvelopeValueV1::Object(args) = field(header, "arguments")? else {
        return Err(Error::InvalidRequest);
    };
    if !d.fields_match(args.iter().map(|(k, _)| *k))
        || text(field(args, "contract")?)? != d.contract
        || text(field(args, "operation")?)? != operation
    {
        return Err(Error::InvalidRequest);
    }
    Ok(d)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeOperationKindV1 {
    SourceLoad,
    AnnotationLoad,
    SourceSave,
    AnnotationSave,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeOperationTaskV1<'h> {
    SourceLoad(&'h SourceReadIdV1),
    AnnotationLoad(&'h AnnotationReadId),
    Save(&'h SaveId),
}
#[derive(Clone, Copy)]
pub enum NativeOperationEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, NativeOperationEnvelopeValueV1<'a, 'h>)]),
    Data(InertValueV1<'a>),
    Task(NativeOperationTaskV1<'h>),
    Context(&'h NativeOperationObservationIdV1),
    Progress(&'h NativeOperationProgressIdV1),
}
impl std::fmt::Debug for NativeOperationEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeOperationEnvelope(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalizedNativeOperationQueryV1 {
    Status,
    MinimalControlStatus,
    RetainedOutcome(NativeOperationKindV1),
    Wait { budget: usize },
}
pub struct NormalizedNativeOperationArgumentsV1<'h> {
    pub(super) task: NativeOperationTaskV1<'h>,
    pub(super) context: &'h NativeOperationObservationIdV1,
    pub(super) after: Option<&'h NativeOperationProgressIdV1>,
    pub(super) actor: SettingsExecutionContextV1,
    pub(super) profile: String,
    pub(super) query: NormalizedNativeOperationQueryV1,
    pub(super) limits: CaptureLimitsV1,
    pub(super) usage: [CaptureUsageV1; 3],
    pub(super) owned_bytes: usize,
}
impl PartialEq for NormalizedNativeOperationArgumentsV1<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.task == other.task
            && self.context == other.context
            && self.after == other.after
            && self.actor == other.actor
            && self.profile == other.profile
            && self.query == other.query
            && self.limits == other.limits
    }
}
impl Eq for NormalizedNativeOperationArgumentsV1<'_> {}
impl std::fmt::Debug for NormalizedNativeOperationArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedNativeOperationArguments(<redacted>)")
    }
}
impl NormalizedNativeOperationArgumentsV1<'_> {
    pub fn task(&self) -> NativeOperationTaskV1<'_> {
        self.task
    }
    pub fn context(&self) -> &NativeOperationObservationIdV1 {
        self.context
    }
    pub fn after(&self) -> Option<&NativeOperationProgressIdV1> {
        self.after
    }
    pub fn actor(&self) -> &SettingsExecutionContextV1 {
        &self.actor
    }
    pub fn profile(&self) -> &str {
        &self.profile
    }
    pub fn query(&self) -> NormalizedNativeOperationQueryV1 {
        self.query
    }
    pub fn capture_usage(&self) -> [CaptureUsageV1; 3] {
        self.usage
    }
    pub fn owned_bytes(&self) -> usize {
        self.owned_bytes
    }
}
pub fn normalize_native_operation_envelope_v1<'a, 'h>(
    envelope: &'a NativeOperationEnvelopeValueV1<'a, 'h>,
    actor: &SettingsExecutionContextV1,
    limits: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedNativeOperationArgumentsV1<'h>, Error> {
    let result = normalize(envelope, actor, limits, maximum, r);
    observe(r, Boundary::NativeOperationArgumentNormalization, &result);
    result
}
type Value<'a, 'h> = NativeOperationEnvelopeValueV1<'a, 'h>;
fn fields<'a, 'h>(
    value: &'a Value<'a, 'h>,
    keys: &[&str],
) -> Result<&'a [(&'a str, Value<'a, 'h>)], Error> {
    let Value::Object(rows) = value else {
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
fn captured<'a>(
    c: &'a InertCaptureV1,
    v: &'a CapturedValueV1,
    key: &str,
) -> Result<&'a CapturedValueV1, Error> {
    let CapturedValueV1::Object(rows) = v else {
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
fn copied(
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
pub(super) fn valid_limits(l: CaptureLimitsV1) -> bool {
    [l.max_bytes, l.max_nodes, l.max_depth]
        .into_iter()
        .all(|n| (n as u128) <= 9_007_199_254_740_991)
        && l.max_nodes > 0
        && l.max_depth > 0
}
fn natural(v: &CapturedValueV1) -> Result<usize, Error> {
    match v {
        CapturedValueV1::Number(n)
            if n.is_finite()
                && *n > 0.0
                && n.fract() == 0.0
                && *n <= 9_007_199_254_740_991.0
                && (*n as u128) <= usize::MAX as u128 =>
        {
            Ok(*n as usize)
        }
        _ => Err(Error::InvalidRequest),
    }
}
fn kind(v: &CapturedValueV1) -> Result<NativeOperationKindV1, Error> {
    match v {
        CapturedValueV1::Text(v) => match v.as_str() {
            "source-load" => Ok(NativeOperationKindV1::SourceLoad),
            "annotation-load" => Ok(NativeOperationKindV1::AnnotationLoad),
            "source-save" => Ok(NativeOperationKindV1::SourceSave),
            "annotation-save" => Ok(NativeOperationKindV1::AnnotationSave),
            _ => Err(Error::InvalidRequest),
        },
        _ => Err(Error::InvalidRequest),
    }
}
fn normalize<'a, 'h>(
    envelope: &'a Value<'a, 'h>,
    actor: &SettingsExecutionContextV1,
    input: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedNativeOperationArgumentsV1<'h>, Error> {
    if !valid_limits(input) || (maximum as u128) > 9_007_199_254_740_991 {
        return Err(Error::InvalidRequest);
    }
    let l = CaptureLimitsV1 {
        max_bytes: input.max_bytes.min(maximum),
        ..input
    };
    // Fixed outer/header/args/scope/limits/canonical templates and opaque labels.
    if 27usize
        .checked_mul(size_of::<(&str, InertValueV1<'_>)>())
        .and_then(|n| n.checked_add(size_of::<[InertValueV1<'_>; 3]>()))
        .is_none_or(|n| n > l.max_bytes)
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
        ("typed", InertValueV1::Number(5.0)) | ("dynamic", InertValueV1::Text("inspect-execution"))
    ) || !matches!(data(field(header, "version")?)?, InertValueV1::Number(1.0))
    {
        return Err(Error::InvalidRequest);
    }
    let variant = text(field(header, "variant")?)?;
    let keys: &[&str] = match variant {
        "status" | "minimal-control-status" => {
            &["contract", "operation", "task", "context", "profile"]
        }
        "retained-outcome" => &[
            "contract",
            "operation",
            "task",
            "context",
            "profile",
            "facet",
        ],
        "bounded-observation-wait" => &[
            "contract",
            "operation",
            "task",
            "context",
            "profile",
            "after",
            "budget",
        ],
        _ => return Err(Error::InvalidRequest),
    };
    let args = fields(field(header, "arguments")?, keys)?;
    let contract = if variant == "retained-outcome" {
        "native-operation-receipt/1"
    } else {
        "native-operation-observation/1"
    };
    // The installed description and actual owning decoder consume the same
    // closed field/contract declaration; nested native meaning stays below.
    let d = native_operation_argument_declarations_v1()
        .into_iter()
        .find(|d| d.variant.operation == variant)
        .ok_or(Error::InvalidRequest)?;
    if contract != d.contract || !d.fields_match(args.iter().map(|(k, _)| *k)) {
        return Err(Error::InvalidRequest);
    }
    if text(field(args, "contract")?)? != contract || text(field(args, "operation")?)? != variant {
        return Err(Error::InvalidRequest);
    }
    let Value::Task(task) = field(args, "task")? else {
        return Err(Error::InvalidRequest);
    };
    let Value::Context(context) = field(args, "context")? else {
        return Err(Error::InvalidRequest);
    };
    let after = if variant == "bounded-observation-wait" {
        let Value::Progress(p) = field(args, "after")? else {
            return Err(Error::InvalidRequest);
        };
        Some(*p)
    } else {
        None
    };
    let mut argument_rows = [("", InertValueV1::Null); 7];
    for (i, key) in keys.iter().enumerate() {
        argument_rows[i] = (
            key,
            if matches!(*key, "task" | "context" | "after") {
                InertValueV1::Null
            } else {
                data(field(args, key)?)?
            },
        );
    }
    let argument_shadow = InertValueV1::Object(&argument_rows[..keys.len()]);
    let header_shadow = [
        (header_keys[0], selector),
        ("variant", InertValueV1::Text(variant)),
        ("version", InertValueV1::Number(1.0)),
        ("arguments", argument_shadow),
    ];
    let outer_shadow = [
        ("route", InertValueV1::Text(route)),
        ("request", InertValueV1::Object(&header_shadow)),
    ];
    let whole = copied(&InertValueV1::Object(&outer_shadow), l, r)?;
    let names = [
        "authority",
        "engine",
        "recipient",
        "purpose",
        "destination",
        "profile",
        "continuity",
    ];
    let mut scope = [("", InertValueV1::Null); 7];
    for (i, value) in settings::basis::labels(actor).iter().enumerate() {
        if value.is_empty() {
            return Err(Error::InvalidRequest);
        }
        scope[i] = (names[i], InertValueV1::Text(value));
    }
    let bound = copied(&InertValueV1::Object(&scope), l, r)?;
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
        InertValueV1::Text("native-task"),
        InertValueV1::Text("observation-context"),
        InertValueV1::Text("ordinary-progress"),
    ];
    let canonical = [
        ("request", argument_shadow),
        ("context", InertValueV1::Object(&scope)),
        ("limits", InertValueV1::Object(&limit_fields)),
        (
            "opaqueBindings",
            InertValueV1::Array(&bindings[..if after.is_some() { 3 } else { 2 }]),
        ),
    ];
    let canonical = copied(&InertValueV1::Object(&canonical), l, r)?;
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
    let query = match variant {
        "status" => NormalizedNativeOperationQueryV1::Status,
        "minimal-control-status" => NormalizedNativeOperationQueryV1::MinimalControlStatus,
        "bounded-observation-wait" => NormalizedNativeOperationQueryV1::Wait {
            budget: natural(captured(&whole, request, "budget")?)?,
        },
        _ => {
            let facet = captured(&whole, request, "facet")?;
            let CapturedValueV1::Object(rows) = facet else {
                return Err(Error::InvalidRequest);
            };
            if rows.len() != 1 || rows[0].0 != "kind" {
                return Err(Error::InvalidRequest);
            }
            NormalizedNativeOperationQueryV1::RetainedOutcome(kind(captured(
                &whole, facet, "kind",
            )?)?)
        }
    };
    let owned_bytes = settings::basis::labels(actor)
        .into_iter()
        .map(str::len)
        .chain([profile.len()])
        .try_fold(size_of::<NormalizedNativeOperationArgumentsV1<'_>>(), add)?;
    if owned_bytes > l.max_bytes {
        return Err(Error::Limit);
    }
    Ok(NormalizedNativeOperationArgumentsV1 {
        task: *task,
        context,
        after,
        actor: settings::basis::copy_context(actor).map_err(|_| Error::Capacity)?,
        profile: copy_text(profile)?,
        query,
        limits: input,
        usage: [whole.usage(), bound.usage(), canonical.usage()],
        owned_bytes,
    })
}
