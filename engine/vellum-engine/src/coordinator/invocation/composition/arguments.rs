//! Closed native F11 arguments. Opaque roles are borrowed, never reconstructed.
use super::super::literal::arguments::{bind_header, field, natural, string};
use super::*;
use crate::capture::{CaptureErrorV1, InertCaptureV1, InertValueV1 as D};
pub type CompositionEnvelopeValueV1<'a, 'h> = LiteralEditEnvelopeValueV1<'a, 'h>;
type V<'a, 'h> = CompositionEnvelopeValueV1<'a, 'h>;
type Result<T> = std::result::Result<T, InvocationError>;
pub const PRIMARY_COMPOSITION_REPLACEMENT_PROFILE_V1: &str = "primary-composition-replacement/1";
static ARGUMENTS: NativeArgumentDeclarationV1 = NativeArgumentDeclarationV1 {
    contract: "composition-commit/1",
    variant: NativeHandlerVariantDeclarationV1 {
        operation: "accepted-composition-commit",
        fields: &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedCompositionVariant"),
            ("invocation", "InvocationIdentity"),
            ("document", "DocumentIdentity"),
            ("view", "LogicalViewIdentity"),
            ("sourceRevision", "SafeNatural"),
            ("expectedSelection", "SafeNatural"),
            ("profile", "ExecutionProfileRef"),
            ("replacement", "ExactReplacementSpan"),
            ("text", "LiteralText"),
        ],
    },
};
pub struct NormalizedCompositionArgumentsV1<'h> {
    pub(super) invocation: &'h InvocationId,
    pub(super) view: &'h LogicalViewContext,
    pub(super) replacement: Replacement,
    pub(super) actor: SettingsExecutionContextV1,
    pub(super) limits: CaptureLimitsV1,
}
impl std::fmt::Debug for NormalizedCompositionArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedCompositionArguments(<redacted>)")
    }
}
impl<'h> NormalizedCompositionArgumentsV1<'h> {
    pub(in crate::coordinator) const ARGUMENT_DECLARATION: &'static NativeArgumentDeclarationV1 =
        &ARGUMENTS;
    pub fn invocation(&self) -> &'h InvocationId {
        self.invocation
    }
    pub fn context(&self) -> &'h LogicalViewContext {
        self.view
    }
    pub fn replacement(&self) -> &Replacement {
        &self.replacement
    }
    pub fn execution_context(&self) -> &SettingsExecutionContextV1 {
        &self.actor
    }
    pub fn limits(&self) -> CaptureLimitsV1 {
        self.limits
    }
    pub fn argument_declaration_v1(&self) -> &'static NativeArgumentDeclarationV1 {
        &ARGUMENTS
    }
}
pub(super) fn context_labels(c: &SettingsExecutionContextV1) -> [&str; 7] {
    [
        &c.authority,
        &c.engine,
        &c.recipient,
        &c.purpose,
        &c.destination,
        &c.profile,
        &c.continuity,
    ]
}
const SAFE: u64 = 9_007_199_254_740_991;
fn safe(value: usize) -> Result<f64> {
    let n = u64::try_from(value).map_err(|_| InvocationError::Limit)?;
    if n > SAFE {
        return Err(InvocationError::Limit);
    }
    Ok(n as f64)
}
/// Meter the canonical request/context/three-limit bundle before owned payload
/// copying. Opaque role referents are excluded, not serialized as fake labels.
/// The complete prepared native envelope is checked separately at admission.
#[allow(clippy::too_many_arguments)]
pub(super) fn validate_capture(
    view: &LogicalViewContext,
    lo: usize,
    hi: usize,
    expected: &str,
    text: &str,
    actor: &SettingsExecutionContextV1,
    effective: CaptureLimitsV1,
    original: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<()> {
    if lo > hi
        || expected.len() != hi - lo
        || context_labels(actor).iter().any(|v| v.is_empty())
        || view.source.revision > SAFE
        || view.stamp.selection_generation > SAFE
    {
        return Err(InvocationError::MalformedIntent);
    }
    let replacement = [
        ("lo", D::Number(safe(lo)?)),
        ("hi", D::Number(safe(hi)?)),
        ("expected", D::Text(expected)),
    ];
    let request = [
        ("contract", D::Text(ARGUMENTS.contract)),
        ("operation", D::Text(ARGUMENTS.variant.operation)),
        ("invocation", D::Null),
        ("document", D::Null),
        ("view", D::Null),
        ("sourceRevision", D::Number(view.source.revision as f64)),
        (
            "expectedSelection",
            D::Number(view.stamp.selection_generation as f64),
        ),
        (
            "profile",
            D::Text(PRIMARY_COMPOSITION_REPLACEMENT_PROFILE_V1),
        ),
        ("replacement", D::Object(&replacement)),
        ("text", D::Text(text)),
    ];
    let context = [
        ("authority", D::Text(&actor.authority)),
        ("engine", D::Text(&actor.engine)),
        ("recipient", D::Text(&actor.recipient)),
        ("purpose", D::Text(&actor.purpose)),
        ("destination", D::Text(&actor.destination)),
        ("profile", D::Text(&actor.profile)),
        ("continuity", D::Text(&actor.continuity)),
    ];
    let limits = [
        ("maxRequestBytes", D::Number(safe(original.max_bytes)?)),
        ("maxInputNodes", D::Number(safe(original.max_nodes)?)),
        ("maxInputDepth", D::Number(safe(original.max_depth)?)),
    ];
    let canonical = [
        ("request", D::Object(&request)),
        ("context", D::Object(&context)),
        ("limits", D::Object(&limits)),
    ];
    InertCaptureV1::copy(&D::Object(&canonical), effective, r).map_err(|e| match e {
        CaptureErrorV1::Capacity => InvocationError::Capacity,
        CaptureErrorV1::Limit => InvocationError::Limit,
        _ => InvocationError::MalformedIntent,
    })?;
    Ok(())
}
fn owned(s: &str) -> Result<String> {
    let mut text = String::new();
    text.try_reserve_exact(s.len())
        .map_err(|_| InvocationError::Capacity)?;
    text.push_str(s);
    Ok(text)
}
fn normalize<'a, 'h>(
    value: V<'a, 'h>,
    view: &'h LogicalViewContext,
    actor: &SettingsExecutionContextV1,
    limits: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<NormalizedCompositionArgumentsV1<'h>> {
    let V::Object(rows) = value else {
        return Err(InvocationError::MalformedIntent);
    };
    let invocation = bind_header(
        rows,
        view,
        &ARGUMENTS,
        PRIMARY_COMPOSITION_REPLACEMENT_PROFILE_V1,
    )?;
    let V::Data(D::Object(span)) = field(rows, "replacement")? else {
        return Err(InvocationError::MalformedIntent);
    };
    if span.len() != 3
        || ["lo", "hi", "expected"]
            .iter()
            .any(|name| span.iter().filter(|(key, _)| key == name).count() != 1)
    {
        return Err(InvocationError::MalformedIntent);
    }
    let get = |key| {
        span.iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| V::Data(*v))
            .ok_or(InvocationError::MalformedIntent)
    };
    let start = usize::try_from(natural(get("lo")?)?).map_err(|_| InvocationError::Limit)?;
    let end = usize::try_from(natural(get("hi")?)?).map_err(|_| InvocationError::Limit)?;
    let expected = string(get("expected")?)?;
    let inserted = string(field(rows, "text")?)?;
    validate_capture(
        view, start, end, expected, inserted, actor, limits, limits, r,
    )?;
    let actor = basis::copy_context(actor).map_err(|_| InvocationError::Capacity)?;
    Ok(NormalizedCompositionArgumentsV1 {
        invocation,
        view,
        replacement: Replacement {
            start,
            end,
            expected: owned(expected)?,
            inserted: owned(inserted)?,
        },
        actor,
        limits,
    })
}
/// Passive native normalization; the original full view and execution contexts
/// are supplied independently. This is neither authorization nor a wire decoder.
pub fn normalize_composition_arguments_v1<'a, 'h>(
    value: V<'a, 'h>,
    view: &'h LogicalViewContext,
    actor: &SettingsExecutionContextV1,
    limits: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<NormalizedCompositionArgumentsV1<'h>> {
    let timer = r.begin_service_timing();
    let result = normalize(value, view, actor, limits, r);
    capture_timed(
        r,
        Boundary::CompositionArgumentNormalization,
        &result,
        timer.finish(),
    );
    result
}
/// F11/version-1 typed or dynamic outer envelope over the same native arguments.
pub fn normalize_composition_envelope_v1<'a, 'h>(
    value: V<'a, 'h>,
    view: &'h LogicalViewContext,
    actor: &SettingsExecutionContextV1,
    limits: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<NormalizedCompositionArgumentsV1<'h>> {
    let timer = r.begin_service_timing();
    let result = (|| {
        let V::Object(outer) = value else {
            return Err(InvocationError::MalformedIntent);
        };
        if outer.len() != 2
            || outer.iter().filter(|(k, _)| *k == "route").count() != 1
            || outer.iter().filter(|(k, _)| *k == "request").count() != 1
        {
            return Err(InvocationError::MalformedIntent);
        }
        let route = string(field(outer, "route")?)?;
        let V::Object(header) = field(outer, "request")? else {
            return Err(InvocationError::MalformedIntent);
        };
        let key = match route {
            "typed" => "family",
            "dynamic" => "name",
            _ => return Err(InvocationError::MalformedIntent),
        };
        if header.len() != 4
            || [key, "variant", "version", "arguments"]
                .iter()
                .any(|k| header.iter().filter(|(name, _)| name == k).count() != 1)
        {
            return Err(InvocationError::MalformedIntent);
        }
        if (route == "typed" && natural(field(header, key)?)? != 10)
            || (route == "dynamic" && string(field(header, key)?)? != "change-source")
            || string(field(header, "variant")?)? != "accepted-composition-commit"
            || natural(field(header, "version")?)? != 1
        {
            return Err(InvocationError::MalformedIntent);
        }
        normalize(field(header, "arguments")?, view, actor, limits, r)
    })();
    capture_timed(
        r,
        Boundary::CompositionArgumentNormalization,
        &result,
        timer.finish(),
    );
    result
}
