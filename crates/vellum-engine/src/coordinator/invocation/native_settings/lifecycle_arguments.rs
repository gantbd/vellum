//! Native lifecycle data; explicit conditions/identities do not grant writes.
use super::super::scoped_definition::arguments::{same_identity, target, target_cost};
use super::super::scoped_lifecycle::arguments::{array, identity, scope, values_copy, values_cost};
use super::super::settings_scoped::arguments::{add, copy, exact, field, mul, node, reserve, text};
use super::arguments::version;
use super::*;
use crate::capture::{
    CaptureErrorV1, CaptureLimitsV1, CaptureUsageV1, CapturedValueV1 as C, InertCaptureV1,
    InertValueV1 as D,
};
use crate::settings::{
    SettingLayerCreateV1, SettingLayerRemoveV1, SettingTargetV1, SettingValueV1,
    SettingsLayerBasisV1, SettingsLayerChangeV1, SettingsLayerLimitsV1,
};
use std::mem::size_of;
#[cfg(test)]
mod tests;
pub type NativeLifecycleEnvelopeValueV1<'a, 'h> = NativeSettingsEnvelopeValueV1<'a, 'h>;
pub struct NormalizedNativeLifecycleArgumentsV1<'h> {
    invocation: &'h InvocationId,
    dependencies: &'h ReadDependencyRequestV3,
    activation: SettingsActivationLimitsV1,
    profile: SettingsLayerLimitsV1,
    execution: String,
    expected: SettingsLayerBasisV1,
    changes: SettingsLayerChangeV1,
    usage: CaptureUsageV1,
    owned: usize,
    peak: usize,
}
impl std::fmt::Debug for NormalizedNativeLifecycleArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedNativeLifecycleArguments(<redacted>)")
    }
}
impl<'h> NormalizedNativeLifecycleArgumentsV1<'h> {
    pub fn invocation(&self) -> &'h InvocationId {
        self.invocation
    }
    pub fn dependencies(&self) -> &'h ReadDependencyRequestV3 {
        self.dependencies
    }
    pub fn activation_profile(&self) -> SettingsActivationLimitsV1 {
        self.activation
    }
    pub fn native_profile(&self) -> SettingsLayerLimitsV1 {
        self.profile
    }
    pub fn execution_profile(&self) -> &str {
        &self.execution
    }
    pub fn expected(&self) -> SettingsLayerBasisV1 {
        self.expected
    }
    pub fn changes(&self) -> &SettingsLayerChangeV1 {
        &self.changes
    }
    pub fn capture_usage(&self) -> CaptureUsageV1 {
        self.usage
    }
    /// Conservative logical output, including contribution value-root overlap.
    pub fn owned_bytes(&self) -> usize {
        self.owned
    }
    /// Joint inert/output/value workspace, not borrowed roles, caller ingress,
    /// stack, allocator/RSS or global sponsorship.
    pub fn normalization_peak_bytes(&self) -> usize {
        self.peak
    }
    pub fn request(&self) -> NativeSettingsLifecycleRequestV1<'_> {
        NativeSettingsLifecycleRequestV1 {
            expected: self.expected,
            changes: &self.changes,
            dependencies: self.dependencies,
        }
    }
}
type Result<T> = std::result::Result<T, InvocationError>;
fn bad<T>() -> Result<T> {
    Err(InvocationError::MalformedIntent)
}
fn natural(v: &C) -> Result<u64> {
    if matches!(v,C::Number(n) if *n == 0.0) {
        Ok(0)
    } else {
        version(v)
    }
}
fn normalize<'h>(
    envelope: &NativeLifecycleEnvelopeValueV1<'_, 'h>,
    activation: SettingsActivationLimitsV1,
    profile: SettingsLayerLimitsV1,
    limits: CaptureLimitsV1,
    label_limit: usize,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedNativeLifecycleArgumentsV1<'h>> {
    use NativeLifecycleEnvelopeValueV1 as V;
    let V::Object(rows) = envelope else {
        return bad();
    };
    super::declaration::header(
        rows,
        NormalizedNativeLifecycleArgumentsV1::ARGUMENT_DECLARATION,
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
    let input = [
        ("profile", data("profile")?),
        ("expected", data("expected")?),
        ("create", data("create")?),
        ("remove", data("remove")?),
        ("targets", data("targets")?),
    ];
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
    let execution = text(field(&c, c.root(), "profile")?)?;
    if execution.is_empty() || execution.len() > label_limit {
        return bad();
    }
    let expected = field(&c, c.root(), "expected")?;
    exact(expected, &["generation", "layerIncarnation"])?;
    let expected = SettingsLayerBasisV1 {
        configuration_generation: version(field(&c, expected, "generation")?)?,
        layer_incarnation: natural(field(&c, expected, "layerIncarnation")?)?,
    };
    let creates = array(field(&c, c.root(), "create")?)?;
    let removes = array(field(&c, c.root(), "remove")?)?;
    let targets = array(field(&c, c.root(), "targets")?)?;
    if add(creates.len(), removes.len())? > profile.max_changes {
        return Err(InvocationError::Limit);
    }
    let mut owned = add(
        size_of::<NormalizedNativeLifecycleArgumentsV1>(),
        execution.len(),
    )?;
    owned = add(
        owned,
        mul(creates.len(), size_of::<SettingLayerCreateV1>())?,
    )?;
    owned = add(
        owned,
        mul(removes.len(), size_of::<SettingLayerRemoveV1>())?,
    )?;
    owned = add(owned, mul(targets.len(), size_of::<SettingTargetV1>())?)?;
    for (i, n) in creates.iter().enumerate() {
        let row = node(&c, *n)?;
        exact(row, &["layer", "kind", "values"])?;
        let id = identity(&c, field(&c, row, "layer")?, label_limit)?;
        scope(field(&c, row, "kind")?)?;
        owned = add(
            owned,
            add(
                id.payload()?,
                values_cost(&c, field(&c, row, "values")?, label_limit)?,
            )?,
        )?;
        for p in &creates[..i] {
            if identity(&c, field(&c, node(&c, *p)?, "layer")?, label_limit)? == id {
                return bad();
            }
        }
    }
    for (i, n) in removes.iter().enumerate() {
        let row = node(&c, *n)?;
        exact(row, &["layer", "expectedGeneration"])?;
        let id = identity(&c, field(&c, row, "layer")?, label_limit)?;
        version(field(&c, row, "expectedGeneration")?)?;
        owned = add(owned, id.payload()?)?;
        for p in creates.iter().chain(removes[..i].iter()) {
            if identity(&c, field(&c, node(&c, *p)?, "layer")?, label_limit)? == id {
                return bad();
            }
        }
    }
    for (i, n) in targets.iter().enumerate() {
        let row = node(&c, *n)?;
        owned = add(owned, target_cost(&c, row, label_limit)?)?;
        for p in &targets[..i] {
            if same_identity(
                &c,
                field(&c, row, "target")?,
                field(&c, node(&c, *p)?, "target")?,
            )? {
                return bad();
            }
        }
    }
    let workspace = mul(c.usage().nodes, size_of::<Option<SettingValueV1>>())?;
    let peak = c
        .usage()
        .peak_bytes
        .max(add(add(c.usage().retained_bytes, owned)?, workspace)?);
    if peak > maximum {
        return Err(InvocationError::Limit);
    }
    let mut create = Vec::new();
    reserve(&mut create, creates.len())?;
    let mut remove = Vec::new();
    reserve(&mut remove, removes.len())?;
    let mut copied_targets = Vec::new();
    reserve(&mut copied_targets, targets.len())?;
    for n in creates {
        let row = node(&c, *n)?;
        create.push(SettingLayerCreateV1 {
            identity: identity(&c, field(&c, row, "layer")?, label_limit)?.copy()?,
            scope: scope(field(&c, row, "kind")?)?,
            values: values_copy(&c, field(&c, row, "values")?)?,
        });
    }
    for n in removes {
        let row = node(&c, *n)?;
        remove.push(SettingLayerRemoveV1 {
            identity: identity(&c, field(&c, row, "layer")?, label_limit)?.copy()?,
            expected_generation: version(field(&c, row, "expectedGeneration")?)?,
        });
    }
    for n in targets {
        copied_targets.push(target(&c, node(&c, *n)?)?);
    }
    Ok(NormalizedNativeLifecycleArgumentsV1 {
        invocation,
        dependencies,
        activation,
        profile,
        execution: copy(execution)?,
        expected,
        changes: SettingsLayerChangeV1 {
            create,
            remove,
            targets: copied_targets,
        },
        usage: c.usage(),
        owned,
        peak,
    })
}
pub fn normalize_native_lifecycle_envelope_v1<'h>(
    envelope: &NativeLifecycleEnvelopeValueV1<'_, 'h>,
    activation: SettingsActivationLimitsV1,
    profile: SettingsLayerLimitsV1,
    limits: CaptureLimitsV1,
    label_limit: usize,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedNativeLifecycleArgumentsV1<'h>> {
    let timer = r.begin_service_timing();
    let result = normalize(
        envelope,
        activation,
        profile,
        limits,
        label_limit,
        maximum,
        r,
    );
    record(
        r,
        Boundary::NativeLifecycleArgumentNormalization,
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
    pub fn submit_normalized_native_lifecycle_v1(
        &mut self,
        q: &NormalizedNativeLifecycleArgumentsV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedNativeSettingsLifecycleInvocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        self.submit_normalized_native_lifecycle_input(q, c, f, None, r)
    }
    /// Opt in before original admission; replay uses the ordinary submission port.
    /// The original result owner still funds, captures and releases every reference.
    pub fn submit_normalized_native_lifecycle_with_created_results_v1(
        &mut self,
        q: &NormalizedNativeLifecycleArgumentsV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedNativeSettingsLifecycleInvocationV1<'_>,
        result: &TrustedCreatedLayerResultAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        self.submit_normalized_native_lifecycle_input(q, c, f, Some(result), r)
    }
    fn submit_normalized_native_lifecycle_input(
        &mut self,
        q: &NormalizedNativeLifecycleArgumentsV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedNativeSettingsLifecycleInvocationV1<'_>,
        result: Option<&TrustedCreatedLayerResultAdmissionV1<'_>>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        let timer = r.begin_service_timing();
        let result = if q.activation != f.common.activation
            || q.profile != f.profile
            || q.execution != c.profile
        {
            Err(InvocationError::Unavailable)
        } else {
            match result {
                Some(result) => self.submit_native_lifecycle_with_created_results_v1(
                    q.invocation,
                    &q.request(),
                    c,
                    f,
                    result,
                    r,
                ),
                None => {
                    self.submit_native_settings_lifecycle_v1(q.invocation, &q.request(), c, f, r)
                }
            }
        };
        record(
            r,
            Boundary::NativeLifecycleDecodedAdmission,
            &result,
            timer.finish(),
        );
        result
    }
}
