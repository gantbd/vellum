//! Passive scoped settings data; original bases remain a separate typed port.
use super::*;
use crate::capture::{
    CaptureErrorV1, CaptureLimitsV1, CaptureUsageV1, CapturedValueV1 as C, InertCaptureV1,
    InertValueV1 as D,
};
use crate::settings::{SettingOperationV1, SettingValueV1, SettingsActivationLimitsV1};
use std::mem::size_of;

static ARGUMENTS: NativeArgumentDeclarationV1 = NativeArgumentDeclarationV1 {
    contract: "scoped-semantic-settings/1",
    variant: NativeHandlerVariantDeclarationV1 {
        operation: "semantic-settings",
        fields: &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedSettingsVariant"),
            ("invocation", "InvocationIdentity"),
            ("profile", "ExecutionProfileRef"),
            ("changes", "BoundedScopedSettingChanges"),
            ("dependencies", "BoundedContextDependencyVector"),
        ],
    },
};

#[derive(Clone, Copy)]
pub enum ScopedSettingsEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, Self)]),
    Data(D<'a>),
    Invocation(&'h InvocationId),
    Dependencies(&'h ReadDependencyRequestV3),
}
impl std::fmt::Debug for ScopedSettingsEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ScopedSettingsEnvelope(<redacted>)")
    }
}
pub struct NormalizedScopedSettingsArgumentsV1<'h> {
    invocation: &'h InvocationId,
    dependencies: &'h ReadDependencyRequestV3,
    profile: SettingsActivationLimitsV1,
    execution: String,
    changes: Vec<ScopedSettingChangeV1>,
    usage: CaptureUsageV1,
    owned: usize,
    peak: usize,
}
impl std::fmt::Debug for NormalizedScopedSettingsArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedScopedSettingsArguments(<redacted>)")
    }
}
impl<'h> NormalizedScopedSettingsArgumentsV1<'h> {
    pub(in crate::coordinator) const ARGUMENT_DECLARATION: &'static NativeArgumentDeclarationV1 =
        &ARGUMENTS;
    pub fn argument_declaration_v1(&self) -> &'static NativeArgumentDeclarationV1 {
        Self::ARGUMENT_DECLARATION
    }
    pub fn invocation(&self) -> &'h InvocationId {
        self.invocation
    }
    pub fn dependencies(&self) -> &'h ReadDependencyRequestV3 {
        self.dependencies
    }
    pub fn changes(&self) -> &[ScopedSettingChangeV1] {
        &self.changes
    }
    pub fn execution_profile(&self) -> &str {
        &self.execution
    }
    pub fn native_profile(&self) -> SettingsActivationLimitsV1 {
        self.profile
    }
    pub fn capture_usage(&self) -> CaptureUsageV1 {
        self.usage
    }
    /// Conservative logical output envelope, including value-root header overlap.
    pub fn owned_bytes(&self) -> usize {
        self.owned
    }
    /// Joint inert/output/conversion workspace; excludes borrowed role referents,
    /// caller ingress, stack, allocator/RSS and global memory sponsorship.
    pub fn normalization_peak_bytes(&self) -> usize {
        self.peak
    }
}
type Result<T> = std::result::Result<T, InvocationError>;
fn bad<T>() -> Result<T> {
    Err(InvocationError::MalformedIntent)
}
pub(in crate::coordinator::invocation) fn add(a: usize, b: usize) -> Result<usize> {
    a.checked_add(b).ok_or(InvocationError::Limit)
}
pub(in crate::coordinator::invocation) fn mul(a: usize, b: usize) -> Result<usize> {
    a.checked_mul(b).ok_or(InvocationError::Limit)
}
pub(in crate::coordinator::invocation) fn reserve<T>(v: &mut Vec<T>, n: usize) -> Result<()> {
    v.try_reserve_exact(n)
        .map_err(|_| InvocationError::Capacity)
}
pub(in crate::coordinator::invocation) fn copy(s: &str) -> Result<String> {
    let mut out = String::new();
    out.try_reserve_exact(s.len())
        .map_err(|_| InvocationError::Capacity)?;
    out.push_str(s);
    Ok(out)
}
pub(in crate::coordinator::invocation) fn node(c: &InertCaptureV1, i: usize) -> Result<&C> {
    c.node(i).ok_or(InvocationError::Faulted)
}
pub(in crate::coordinator::invocation) fn exact(v: &C, keys: &[&str]) -> Result<()> {
    let C::Object(rows) = v else { return bad() };
    if rows.len() != keys.len() || rows.iter().any(|(k, _)| !keys.contains(&k.as_str())) {
        return bad();
    }
    Ok(())
}
pub(in crate::coordinator::invocation) fn index(v: &C, key: &str) -> Result<usize> {
    let C::Object(rows) = v else { return bad() };
    rows.iter()
        .find(|(k, _)| k == key)
        .map(|(_, i)| *i)
        .ok_or(InvocationError::MalformedIntent)
}
pub(in crate::coordinator::invocation) fn field<'a>(
    c: &'a InertCaptureV1,
    v: &C,
    key: &str,
) -> Result<&'a C> {
    node(c, index(v, key)?)
}
pub(in crate::coordinator::invocation) fn text(v: &C) -> Result<&str> {
    if let C::Text(s) = v { Ok(s) } else { bad() }
}
pub(in crate::coordinator::invocation) fn natural(v: &C) -> Result<usize> {
    let C::Number(n) = v else { return bad() };
    if !n.is_finite()
        || *n < 0.0
        || n.fract() != 0.0
        || *n > 9_007_199_254_740_991.0
        || *n >= usize::MAX as f64
    {
        return bad();
    }
    Ok(*n as usize)
}
fn range(c: &InertCaptureV1, start: usize) -> Result<std::ops::Range<usize>> {
    // Inert capture is a preorder tree. Children are later, contiguous nodes.
    let mut end = add(start, 1)?;
    let mut i = start;
    while i < end {
        let last = match node(c, i)? {
            C::Array(v) => v.last().copied(),
            C::Object(v) => v.last().map(|(_, n)| *n),
            _ => None,
        };
        if let Some(last) = last {
            if last <= i {
                return Err(InvocationError::Faulted);
            }
            end = end.max(add(last, 1)?);
        }
        i += 1;
    }
    Ok(start..end)
}
pub(in crate::coordinator::invocation) fn value_cost(
    c: &InertCaptureV1,
    start: usize,
) -> Result<usize> {
    let mut bytes = 0;
    for i in range(c, start)? {
        bytes = add(bytes, size_of::<SettingValueV1>())?;
        match node(c, i)? {
            C::Text(s) => bytes = add(bytes, s.len())?,
            C::Object(rows) => {
                bytes = add(bytes, mul(rows.len(), size_of::<String>())?)?;
                for (s, _) in rows {
                    bytes = add(bytes, s.len())?;
                }
            }
            _ => {}
        }
    }
    Ok(bytes)
}
pub(in crate::coordinator::invocation) fn value(
    c: &InertCaptureV1,
    start: usize,
) -> Result<SettingValueV1> {
    let span = range(c, start)?;
    let mut out = Vec::new();
    reserve(&mut out, span.len())?;
    out.resize_with(span.len(), || None);
    // Bottom-up, nonrecursive conversion. No callbacks or coercing deserializer.
    for i in span.clone().rev() {
        let v = match node(c, i)? {
            C::Null => SettingValueV1::Null,
            C::Boolean(b) => SettingValueV1::Boolean(*b),
            C::Number(n) => SettingValueV1::Number(*n),
            C::Text(s) => SettingValueV1::Text(copy(s)?),
            C::Array(rows) => {
                let mut values = Vec::new();
                reserve(&mut values, rows.len())?;
                for n in rows {
                    values.push(out[*n - start].take().ok_or(InvocationError::Faulted)?);
                }
                SettingValueV1::List(values)
            }
            C::Object(rows) => {
                let mut values = Vec::new();
                reserve(&mut values, rows.len())?;
                for (k, n) in rows {
                    values.push((
                        copy(k)?,
                        out[*n - start].take().ok_or(InvocationError::Faulted)?,
                    ));
                }
                SettingValueV1::Map(values)
            }
        };
        out[i - start] = Some(v);
    }
    out[0].take().ok_or(InvocationError::Faulted)
}
struct Change<'a> {
    basis: usize,
    key: &'a str,
    operation: &'a str,
    value: usize,
}
fn change<'a>(c: &'a InertCaptureV1, v: &C, label_limit: usize) -> Result<Change<'a>> {
    exact(v, &["basis", "key", "operation", "value"])?;
    let key = text(field(c, v, "key")?)?;
    if key.is_empty() || key.len() > label_limit {
        return bad();
    }
    let operation = text(field(c, v, "operation")?)?;
    let value = index(v, "value")?;
    match operation {
        "set" => {}
        "remove" | "reset" if matches!(node(c, value)?, C::Null) => {}
        _ => return bad(),
    }
    Ok(Change {
        basis: natural(field(c, v, "basis")?)?,
        key,
        operation,
        value,
    })
}
fn normalize<'h>(
    envelope: &ScopedSettingsEnvelopeValueV1<'_, 'h>,
    profile: SettingsActivationLimitsV1,
    limits: CaptureLimitsV1,
    label_limit: usize,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedScopedSettingsArgumentsV1<'h>> {
    use ScopedSettingsEnvelopeValueV1 as V;
    let V::Object(rows) = envelope else {
        return bad();
    };
    let declaration = NormalizedScopedSettingsArgumentsV1::ARGUMENT_DECLARATION;
    if !declaration.fields_match(rows.iter().map(|(key, _)| *key)) {
        return bad();
    }
    let get = |key| {
        rows.iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
            .ok_or(InvocationError::MalformedIntent)
    };
    let data = |key| match get(key)? {
        V::Data(v) => Ok(*v),
        _ => bad(),
    };
    if !matches!(data("contract")?, D::Text(s) if s == declaration.contract)
        || !matches!(data("operation")?, D::Text(s) if s == declaration.variant.operation)
    {
        return bad();
    }
    let V::Invocation(invocation) = get("invocation")? else {
        return bad();
    };
    let V::Dependencies(dependencies) = get("dependencies")? else {
        return bad();
    };
    if dependencies.predicates.len() > dependencies.budget.max_predicates {
        return Err(InvocationError::Limit);
    }
    let input = [("profile", data("profile")?), ("changes", data("changes")?)];
    // Native request/value accounting uses different representations and is
    // enforced by the typed handler. Do not reinterpret it as envelope cost.
    let ceiling = maximum;
    let capture = InertCaptureV1::copy(
        &D::Object(&input),
        CaptureLimitsV1 {
            max_bytes: limits.max_bytes.min(ceiling),
            max_nodes: limits.max_nodes,
            max_depth: limits.max_depth,
        },
        r,
    )
    .map_err(|e| match e {
        CaptureErrorV1::Capacity => InvocationError::Capacity,
        CaptureErrorV1::Limit => InvocationError::Limit,
        _ => InvocationError::MalformedIntent,
    })?;
    let execution = text(field(&capture, capture.root(), "profile")?)?;
    if execution.is_empty() || execution.len() > label_limit {
        return bad();
    }
    let C::Array(rows) = field(&capture, capture.root(), "changes")? else {
        return bad();
    };
    if rows.len() > profile.max_changes {
        return Err(InvocationError::Limit);
    }
    // All schema/duplicate/workspace checks precede typed output allocation.
    let mut owned = add(
        size_of::<NormalizedScopedSettingsArgumentsV1>(),
        execution.len(),
    )?;
    owned = add(owned, mul(rows.len(), size_of::<ScopedSettingChangeV1>())?)?;
    for (i, n) in rows.iter().enumerate() {
        let ch = change(&capture, node(&capture, *n)?, label_limit)?;
        owned = add(owned, ch.key.len())?;
        // Finite quadratic scan bounded by both max_changes and captured nodes.
        for prior in &rows[..i] {
            let p = change(&capture, node(&capture, *prior)?, label_limit)?;
            if p.basis == ch.basis && p.key == ch.key {
                return bad();
            }
        }
        if ch.operation == "set" {
            owned = add(owned, value_cost(&capture, ch.value)?)?;
        }
    }
    let workspace = mul(capture.usage().nodes, size_of::<Option<SettingValueV1>>())?;
    let peak = capture
        .usage()
        .peak_bytes
        .max(add(add(capture.usage().retained_bytes, owned)?, workspace)?);
    if peak > ceiling {
        return Err(InvocationError::Limit);
    }
    let mut changes = Vec::new();
    reserve(&mut changes, rows.len())?;
    for n in rows {
        let ch = change(&capture, node(&capture, *n)?, label_limit)?;
        changes.push(ScopedSettingChangeV1 {
            basis: ch.basis,
            key: copy(ch.key)?,
            operation: match ch.operation {
                "set" => SettingOperationV1::Set(value(&capture, ch.value)?),
                "remove" => SettingOperationV1::Remove,
                "reset" => SettingOperationV1::Reset,
                _ => return Err(InvocationError::Faulted),
            },
        });
    }
    Ok(NormalizedScopedSettingsArgumentsV1 {
        invocation,
        dependencies,
        profile,
        execution: copy(execution)?,
        changes,
        usage: capture.usage(),
        owned,
        peak,
    })
}
pub fn normalize_scoped_settings_envelope_v1<'h>(
    envelope: &ScopedSettingsEnvelopeValueV1<'_, 'h>,
    profile: SettingsActivationLimitsV1,
    limits: CaptureLimitsV1,
    label_limit: usize,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedScopedSettingsArgumentsV1<'h>> {
    let timer = r.begin_service_timing();
    let result = normalize(envelope, profile, limits, label_limit, maximum, r);
    record(
        r,
        Boundary::ScopedSettingsArgumentNormalization,
        &result,
        timer.finish(),
    );
    result
}
fn record<T>(
    r: &mut Recorder,
    boundary: Boundary,
    out: &Result<T>,
    service_time: Option<std::time::Duration>,
) {
    r.record(Observation {
        boundary,
        disposition: match out {
            Ok(_) => Disposition::Completed,
            Err(InvocationError::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time,
    });
}
impl SourceCoordinator {
    pub fn submit_normalized_scoped_settings_v1(
        &mut self,
        q: &NormalizedScopedSettingsArgumentsV1<'_>,
        bases: &[SettingsWriteBasisId],
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        let timer = r.begin_service_timing();
        let result = if q.profile != f.basis.activation || q.execution != c.profile {
            Err(InvocationError::Unavailable)
        } else {
            self.submit_scoped_settings_change(
                q.invocation,
                &ScopedSettingsRequestV1 {
                    bases,
                    changes: &q.changes,
                    dependencies: q.dependencies,
                },
                c,
                f,
                r,
            )
        };
        record(
            r,
            Boundary::ScopedSettingsDecodedAdmission,
            &result,
            timer.finish(),
        );
        result
    }
}
