//! Inert native settings data lowers only through the context-bearing port.
use super::super::settings_scoped::arguments::{
    add, copy, exact, field, index, mul, node, reserve, text, value, value_cost,
};
use super::*;
use crate::capture::{
    CaptureErrorV1, CaptureLimitsV1, CaptureUsageV1, CapturedValueV1 as C, InertCaptureV1,
    InertValueV1 as D,
};
use crate::settings::{SettingIdentityV1, SettingOperationV1, SettingsActivationLimitsV1};
use std::mem::size_of;
#[cfg(test)]
mod tests;
pub type NativeSettingsEnvelopeValueV1<'a, 'h> = ScopedSettingsEnvelopeValueV1<'a, 'h>;
pub struct NormalizedNativeSettingsArgumentsV1<'h> {
    invocation: &'h InvocationId,
    dependencies: &'h ReadDependencyRequestV3,
    profile: SettingsActivationLimitsV1,
    execution: String,
    changes: Vec<SettingChangeV1>,
    usage: CaptureUsageV1,
    owned: usize,
    peak: usize,
}
impl std::fmt::Debug for NormalizedNativeSettingsArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedNativeSettingsArguments(<redacted>)")
    }
}
impl<'h> NormalizedNativeSettingsArgumentsV1<'h> {
    pub fn invocation(&self) -> &'h InvocationId {
        self.invocation
    }
    pub fn dependencies(&self) -> &'h ReadDependencyRequestV3 {
        self.dependencies
    }
    pub fn native_profile(&self) -> SettingsActivationLimitsV1 {
        self.profile
    }
    pub fn execution_profile(&self) -> &str {
        &self.execution
    }
    pub fn changes(&self) -> &[SettingChangeV1] {
        &self.changes
    }
    pub fn capture_usage(&self) -> CaptureUsageV1 {
        self.usage
    }
    /// Conservative logical output, including set-value root-header overlap.
    pub fn owned_bytes(&self) -> usize {
        self.owned
    }
    /// Joint inert/output/conversion envelope; excludes borrowed role referents,
    /// caller ingress, stack, allocator/RSS and global sponsorship.
    pub fn normalization_peak_bytes(&self) -> usize {
        self.peak
    }
}
type Result<T> = std::result::Result<T, InvocationError>;
fn bad<T>() -> Result<T> {
    Err(InvocationError::MalformedIntent)
}
fn label(v: &C, limit: usize) -> Result<&str> {
    let s = text(v)?;
    if s.is_empty() || s.len() > limit {
        return bad();
    }
    Ok(s)
}
pub(super) fn version(v: &C) -> Result<u64> {
    let C::Number(n) = v else { return bad() };
    if !n.is_finite() || *n < 1.0 || n.fract() != 0.0 || *n > 9_007_199_254_740_991.0 {
        return bad();
    }
    Ok(*n as u64)
}
#[derive(PartialEq, Eq)]
struct Identity<'a> {
    authority: &'a str,
    id: &'a str,
    incarnation: u64,
}
struct Change<'a> {
    layer: Identity<'a>,
    expected: u64,
    key: &'a str,
    operation: &'a str,
    value: usize,
}
fn change<'a>(c: &'a InertCaptureV1, v: &C, limit: usize) -> Result<Change<'a>> {
    exact(
        v,
        &["layer", "expectedGeneration", "key", "operation", "value"],
    )?;
    let layer = field(c, v, "layer")?;
    exact(layer, &["authority", "id", "incarnation"])?;
    let operation = text(field(c, v, "operation")?)?;
    let value = index(v, "value")?;
    match operation {
        "set" => {}
        "remove" | "reset" if matches!(node(c, value)?, C::Null) => {}
        _ => return bad(),
    }
    Ok(Change {
        layer: Identity {
            authority: label(field(c, layer, "authority")?, limit)?,
            id: label(field(c, layer, "id")?, limit)?,
            incarnation: version(field(c, layer, "incarnation")?)?,
        },
        expected: version(field(c, v, "expectedGeneration")?)?,
        key: label(field(c, v, "key")?, limit)?,
        operation,
        value,
    })
}
fn normalize<'h>(
    envelope: &NativeSettingsEnvelopeValueV1<'_, 'h>,
    profile: SettingsActivationLimitsV1,
    limits: CaptureLimitsV1,
    label_limit: usize,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedNativeSettingsArgumentsV1<'h>> {
    use NativeSettingsEnvelopeValueV1 as V;
    let V::Object(rows) = envelope else {
        return bad();
    };
    super::declaration::header(
        rows,
        NormalizedNativeSettingsArgumentsV1::ARGUMENT_DECLARATION,
    )?;
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
    let c = InertCaptureV1::copy(
        &D::Object(&input),
        CaptureLimitsV1 {
            max_bytes: limits.max_bytes.min(maximum),
            ..limits
        },
        r,
    )
    .map_err(|e| match e {
        CaptureErrorV1::Capacity => InvocationError::Capacity,
        CaptureErrorV1::Limit => InvocationError::Limit,
        _ => InvocationError::MalformedIntent,
    })?;
    let execution = label(field(&c, c.root(), "profile")?, label_limit)?;
    let C::Array(rows) = field(&c, c.root(), "changes")? else {
        return bad();
    };
    if rows.len() > profile.max_changes {
        return Err(InvocationError::Limit);
    }
    let mut owned = add(
        add(
            size_of::<NormalizedNativeSettingsArgumentsV1>(),
            execution.len(),
        )?,
        mul(rows.len(), size_of::<SettingChangeV1>())?,
    )?;
    for (i, n) in rows.iter().enumerate() {
        let ch = change(&c, node(&c, *n)?, label_limit)?;
        owned = add(
            owned,
            add(
                add(ch.layer.authority.len(), ch.layer.id.len())?,
                ch.key.len(),
            )?,
        )?;
        if ch.operation == "set" {
            owned = add(owned, value_cost(&c, ch.value)?)?;
        }
        for p in &rows[..i] {
            let p = change(&c, node(&c, *p)?, label_limit)?;
            if ch.layer == p.layer && (ch.key == p.key || ch.expected != p.expected) {
                return bad();
            }
        }
    }
    let workspace = mul(
        c.usage().nodes,
        size_of::<Option<crate::settings::SettingValueV1>>(),
    )?;
    let peak = c
        .usage()
        .peak_bytes
        .max(add(add(c.usage().retained_bytes, owned)?, workspace)?);
    if peak > maximum {
        return Err(InvocationError::Limit);
    }
    let mut changes = Vec::new();
    reserve(&mut changes, rows.len())?;
    for n in rows {
        let ch = change(&c, node(&c, *n)?, label_limit)?;
        changes.push(SettingChangeV1 {
            layer: SettingIdentityV1 {
                authority: copy(ch.layer.authority)?,
                id: copy(ch.layer.id)?,
                incarnation: ch.layer.incarnation,
            },
            expected_generation: ch.expected,
            key: copy(ch.key)?,
            operation: match ch.operation {
                "set" => SettingOperationV1::Set(value(&c, ch.value)?),
                "remove" => SettingOperationV1::Remove,
                "reset" => SettingOperationV1::Reset,
                _ => return Err(InvocationError::Faulted),
            },
        });
    }
    Ok(NormalizedNativeSettingsArgumentsV1 {
        invocation,
        dependencies,
        profile,
        execution: copy(execution)?,
        changes,
        usage: c.usage(),
        owned,
        peak,
    })
}
pub fn normalize_native_settings_envelope_v1<'h>(
    envelope: &NativeSettingsEnvelopeValueV1<'_, 'h>,
    profile: SettingsActivationLimitsV1,
    limits: CaptureLimitsV1,
    label_limit: usize,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedNativeSettingsArgumentsV1<'h>> {
    let timer = r.begin_service_timing();
    let result = normalize(envelope, profile, limits, label_limit, maximum, r);
    record(
        r,
        Boundary::NativeSettingsArgumentNormalization,
        &result,
        timer.finish(),
    );
    result
}
fn record<T>(
    r: &mut Recorder,
    boundary: Boundary,
    result: &Result<T>,
    service_time: Option<std::time::Duration>,
) {
    r.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(InvocationError::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time,
    });
}
impl SourceCoordinator {
    pub fn submit_normalized_native_settings_v1(
        &mut self,
        q: &NormalizedNativeSettingsArgumentsV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedNativeSettingsInvocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        let timer = r.begin_service_timing();
        let result = if q.profile != f.common.activation || q.execution != c.profile {
            Err(InvocationError::Unavailable)
        } else {
            self.submit_native_settings_change_v1(q.invocation, &q.changes, q.dependencies, c, f, r)
        };
        record(
            r,
            Boundary::NativeSettingsDecodedAdmission,
            &result,
            timer.finish(),
        );
        result
    }
}
