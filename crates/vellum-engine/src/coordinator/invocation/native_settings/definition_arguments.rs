//! Native conditions with the shared closed declaration codec; no basis issuance.
use super::super::scoped_definition::arguments::{
    declaration_copy, declaration_cost, declaration_workspace,
};
use super::super::settings_scoped::arguments::{add, copy, exact, field, text};
use super::arguments::version;
use super::*;
use crate::capture::{
    CaptureErrorV1, CaptureLimitsV1, CaptureUsageV1, InertCaptureV1, InertValueV1 as D,
};
use crate::settings::{
    SettingsDeclarationV1, SettingsDefinitionLimitsV1, SettingsInterpretationEpochsV1,
};
use std::mem::size_of;
#[cfg(test)]
mod tests;
pub type NativeDefinitionEnvelopeValueV1<'a, 'h> = NativeSettingsEnvelopeValueV1<'a, 'h>;
pub struct NormalizedNativeDefinitionArgumentsV1<'h> {
    invocation: &'h InvocationId,
    dependencies: &'h ReadDependencyRequestV3,
    activation: SettingsActivationLimitsV1,
    profile: SettingsDefinitionLimitsV1,
    execution: String,
    expected: SettingsInterpretationEpochsV1,
    declaration: SettingsDeclarationV1,
    usage: CaptureUsageV1,
    owned: usize,
    peak: usize,
}
impl std::fmt::Debug for NormalizedNativeDefinitionArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedNativeDefinitionArguments(<redacted>)")
    }
}
impl<'h> NormalizedNativeDefinitionArgumentsV1<'h> {
    pub fn invocation(&self) -> &'h InvocationId {
        self.invocation
    }
    pub fn dependencies(&self) -> &'h ReadDependencyRequestV3 {
        self.dependencies
    }
    pub fn activation_profile(&self) -> SettingsActivationLimitsV1 {
        self.activation
    }
    pub fn native_profile(&self) -> SettingsDefinitionLimitsV1 {
        self.profile
    }
    pub fn execution_profile(&self) -> &str {
        &self.execution
    }
    pub fn expected(&self) -> SettingsInterpretationEpochsV1 {
        self.expected
    }
    pub fn declaration(&self) -> &SettingsDeclarationV1 {
        &self.declaration
    }
    pub fn capture_usage(&self) -> CaptureUsageV1 {
        self.usage
    }
    /// Conservative logical output including schema/type/value root overlap.
    pub fn owned_bytes(&self) -> usize {
        self.owned
    }
    /// Joint inert/output/conversion envelope, not borrowed roles, caller ingress,
    /// stack, allocator/RSS or global sponsorship.
    pub fn normalization_peak_bytes(&self) -> usize {
        self.peak
    }
    pub fn request(&self) -> NativeSettingsDefinitionRequestV1<'_> {
        NativeSettingsDefinitionRequestV1 {
            expected: self.expected,
            declaration: &self.declaration,
            dependencies: self.dependencies,
        }
    }
}
type Result<T> = std::result::Result<T, InvocationError>;
fn bad<T>() -> Result<T> {
    Err(InvocationError::MalformedIntent)
}
fn normalize<'h>(
    envelope: &NativeDefinitionEnvelopeValueV1<'_, 'h>,
    activation: SettingsActivationLimitsV1,
    profile: SettingsDefinitionLimitsV1,
    limits: CaptureLimitsV1,
    label_limit: usize,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedNativeDefinitionArgumentsV1<'h>> {
    use NativeDefinitionEnvelopeValueV1 as V;
    let V::Object(rows) = envelope else {
        return bad();
    };
    super::declaration::header(
        rows,
        NormalizedNativeDefinitionArgumentsV1::ARGUMENT_DECLARATION,
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
        ("definition", data("definition")?),
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
    exact(
        expected,
        &[
            "schemaGeneration",
            "restrictionGeneration",
            "associationGeneration",
        ],
    )?;
    let expected = SettingsInterpretationEpochsV1 {
        schema: version(field(&c, expected, "schemaGeneration")?)?,
        restriction: version(field(&c, expected, "restrictionGeneration")?)?,
        association: version(field(&c, expected, "associationGeneration")?)?,
    };
    let workspace = declaration_workspace(&c)?;
    if add(c.usage().retained_bytes, workspace)? > maximum {
        return Err(InvocationError::Limit);
    }
    let d = field(&c, c.root(), "definition")?;
    let owned = add(
        add(
            size_of::<NormalizedNativeDefinitionArgumentsV1>() - size_of::<SettingsDeclarationV1>(),
            execution.len(),
        )?,
        declaration_cost(&c, d, label_limit)?,
    )?;
    let peak = c
        .usage()
        .peak_bytes
        .max(add(add(c.usage().retained_bytes, workspace)?, owned)?);
    if peak > maximum {
        return Err(InvocationError::Limit);
    }
    let declaration = declaration_copy(&c, d)?;
    Ok(NormalizedNativeDefinitionArgumentsV1 {
        invocation,
        dependencies,
        activation,
        profile,
        execution: copy(execution)?,
        expected,
        declaration,
        usage: c.usage(),
        owned,
        peak,
    })
}
pub fn normalize_native_definition_envelope_v1<'h>(
    envelope: &NativeDefinitionEnvelopeValueV1<'_, 'h>,
    activation: SettingsActivationLimitsV1,
    profile: SettingsDefinitionLimitsV1,
    limits: CaptureLimitsV1,
    label_limit: usize,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedNativeDefinitionArgumentsV1<'h>> {
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
        Boundary::NativeDefinitionArgumentNormalization,
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
    pub fn submit_normalized_native_definition_v1(
        &mut self,
        q: &NormalizedNativeDefinitionArgumentsV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedNativeSettingsDefinitionInvocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        let timer = r.begin_service_timing();
        let result = if q.activation != f.common.activation
            || q.profile != f.profile
            || q.execution != c.profile
        {
            Err(InvocationError::Unavailable)
        } else {
            self.submit_native_settings_definition_v1(q.invocation, &q.request(), c, f, r)
        };
        record(
            r,
            Boundary::NativeDefinitionDecodedAdmission,
            &result,
            timer.finish(),
        );
        result
    }
}
