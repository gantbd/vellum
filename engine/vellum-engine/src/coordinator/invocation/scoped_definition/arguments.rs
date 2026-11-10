//! Passive declaration data over the existing conditional definition port.
use super::super::settings_scoped::arguments::{
    add, copy, exact, field, index, mul, natural, node, reserve, text, value, value_cost,
};
use super::*;
use crate::capture::{
    CaptureErrorV1, CaptureLimitsV1, CaptureUsageV1, CapturedValueV1 as C, InertCaptureV1,
    InertValueV1 as D,
};
use crate::settings::{
    SettingCompositionV1, SettingDomainV1, SettingIdentityV1, SettingSchemaV1, SettingScopeV1,
    SettingTargetV1, SettingTypeV1, SettingValueV1, SettingsActivationLimitsV1,
    SettingsDeclarationV1, SettingsDefinitionLimitsV1,
};
use std::mem::size_of;
#[cfg(test)]
mod tests;
mod types;
pub type ScopedDefinitionEnvelopeValueV1<'a, 'h> = ScopedSettingsEnvelopeValueV1<'a, 'h>;
static ARGUMENTS: NativeArgumentDeclarationV1 = NativeArgumentDeclarationV1 {
    contract: "scoped-engine-settings-definition/1",
    variant: NativeHandlerVariantDeclarationV1 {
        operation: "supported-registry",
        fields: &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedSettingsVariant"),
            ("invocation", "InvocationIdentity"),
            ("profile", "ExecutionProfileRef"),
            ("definition", "ScopedSettingsDefinition"),
            ("dependencies", "BoundedContextDependencyVector"),
        ],
    },
};
pub struct NormalizedScopedDefinitionArgumentsV1<'h> {
    invocation: &'h InvocationId,
    dependencies: &'h ReadDependencyRequestV3,
    activation: SettingsActivationLimitsV1,
    profile: SettingsDefinitionLimitsV1,
    execution: String,
    declaration: SettingsDeclarationV1,
    usage: CaptureUsageV1,
    owned: usize,
    peak: usize,
}
impl std::fmt::Debug for NormalizedScopedDefinitionArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedScopedDefinitionArguments(<redacted>)")
    }
}
impl<'h> NormalizedScopedDefinitionArgumentsV1<'h> {
    pub fn invocation(&self) -> &'h InvocationId {
        self.invocation
    }
    pub fn dependencies(&self) -> &'h ReadDependencyRequestV3 {
        self.dependencies
    }
    pub fn execution_profile(&self) -> &str {
        &self.execution
    }
    pub fn activation_profile(&self) -> SettingsActivationLimitsV1 {
        self.activation
    }
    pub fn native_profile(&self) -> SettingsDefinitionLimitsV1 {
        self.profile
    }
    pub fn declaration(&self) -> &SettingsDeclarationV1 {
        &self.declaration
    }
    pub fn capture_usage(&self) -> CaptureUsageV1 {
        self.usage
    }
    /// Conservative logical output envelope, including schema/value-root overlap.
    pub fn owned_bytes(&self) -> usize {
        self.owned
    }
    /// Joint inert/output/type/value conversion bounds, not borrowed referents,
    /// caller ingress, stack, allocator/RSS or global sponsorship.
    pub fn normalization_peak_bytes(&self) -> usize {
        self.peak
    }
    pub fn request<'a>(
        &'a self,
        basis: &'a SettingsDefinitionBasisId,
    ) -> ScopedSettingsDefinitionRequestV1<'a> {
        ScopedSettingsDefinitionRequestV1 {
            basis,
            declaration: &self.declaration,
            dependencies: self.dependencies,
        }
    }
}
type Result<T> = std::result::Result<T, InvocationError>;
fn bad<T>() -> Result<T> {
    Err(InvocationError::MalformedIntent)
}
fn array(v: &C) -> Result<&[usize]> {
    if let C::Array(v) = v { Ok(v) } else { bad() }
}
fn object(v: &C) -> Result<&[(String, usize)]> {
    if let C::Object(v) = v { Ok(v) } else { bad() }
}
fn label(v: &C, limit: usize) -> Result<&str> {
    let s = text(v)?;
    if s.is_empty() || s.len() > limit {
        return bad();
    }
    Ok(s)
}
fn identity(c: &InertCaptureV1, v: &C, limit: usize) -> Result<usize> {
    exact(v, &["authority", "id", "incarnation"])?;
    if natural(field(c, v, "incarnation")?)? == 0 {
        return bad();
    }
    add(
        label(field(c, v, "authority")?, limit)?.len(),
        label(field(c, v, "id")?, limit)?.len(),
    )
}
fn identity_copy(c: &InertCaptureV1, v: &C) -> Result<SettingIdentityV1> {
    Ok(SettingIdentityV1 {
        authority: copy(text(field(c, v, "authority")?)?)?,
        id: copy(text(field(c, v, "id")?)?)?,
        incarnation: natural(field(c, v, "incarnation")?)? as u64,
    })
}
pub(in crate::coordinator::invocation) fn same_identity(
    c: &InertCaptureV1,
    a: &C,
    b: &C,
) -> Result<bool> {
    Ok(
        text(field(c, a, "authority")?)? == text(field(c, b, "authority")?)?
            && text(field(c, a, "id")?)? == text(field(c, b, "id")?)?
            && natural(field(c, a, "incarnation")?)? == natural(field(c, b, "incarnation")?)?,
    )
}
fn scope(v: &C) -> Result<SettingScopeV1> {
    Ok(match text(v)? {
        "host" => SettingScopeV1::Host,
        "language" => SettingScopeV1::Language,
        "workspace" => SettingScopeV1::Workspace,
        "document" => SettingScopeV1::Document,
        "view" => SettingScopeV1::View,
        _ => return bad(),
    })
}
fn domain(v: &C) -> Result<SettingDomainV1> {
    Ok(match text(v)? {
        "source" => SettingDomainV1::Source,
        "serialization" => SettingDomainV1::Serialization,
        "analysis" => SettingDomainV1::Analysis,
        _ => return bad(),
    })
}
fn composition(v: &C) -> Result<SettingCompositionV1> {
    Ok(match text(v)? {
        "replace" => SettingCompositionV1::Replace,
        "merge-map" => SettingCompositionV1::MergeMap,
        _ => return bad(),
    })
}
fn boolean(v: &C) -> Result<bool> {
    if let C::Boolean(v) = v { Ok(*v) } else { bad() }
}
fn schema_cost(c: &InertCaptureV1, v: &C, limit: usize) -> Result<usize> {
    exact(
        v,
        &[
            "key",
            "type",
            "default",
            "domain",
            "scopes",
            "composition",
            "sensitivity",
        ],
    )?;
    let mut cost = label(field(c, v, "key")?, limit)?.len();
    cost = add(cost, types::cost(c, index(v, "type")?, limit)?)?;
    cost = add(cost, value_cost(c, index(v, "default")?)?)?;
    domain(field(c, v, "domain")?)?;
    composition(field(c, v, "composition")?)?;
    boolean(field(c, v, "sensitivity")?)?;
    let rows = array(field(c, v, "scopes")?)?;
    if rows.is_empty() {
        return bad();
    }
    cost = add(cost, mul(rows.len(), size_of::<SettingScopeV1>())?)?;
    for (i, n) in rows.iter().enumerate() {
        let s = scope(node(c, *n)?)?;
        for p in &rows[..i] {
            if scope(node(c, *p)?)? == s {
                return bad();
            }
        }
    }
    Ok(cost)
}
fn schema(c: &InertCaptureV1, v: &C) -> Result<SettingSchemaV1> {
    let rows = array(field(c, v, "scopes")?)?;
    let mut scopes = Vec::new();
    reserve(&mut scopes, rows.len())?;
    for n in rows {
        scopes.push(scope(node(c, *n)?)?);
    }
    Ok(SettingSchemaV1 {
        key: copy(text(field(c, v, "key")?)?)?,
        value_type: types::build(c, index(v, "type")?)?,
        default: value(c, index(v, "default")?)?,
        domain: domain(field(c, v, "domain")?)?,
        scopes,
        composition: composition(field(c, v, "composition")?)?,
        sensitive: boolean(field(c, v, "sensitivity")?)?,
    })
}
pub(in crate::coordinator::invocation) fn target_cost(
    c: &InertCaptureV1,
    v: &C,
    limit: usize,
) -> Result<usize> {
    exact(v, &["target", "layers"])?;
    let mut cost = identity(c, field(c, v, "target")?, limit)?;
    let rows = array(field(c, v, "layers")?)?;
    cost = add(cost, mul(rows.len(), size_of::<SettingIdentityV1>())?)?;
    for (i, n) in rows.iter().enumerate() {
        let id = node(c, *n)?;
        cost = add(cost, identity(c, id, limit)?)?;
        for p in &rows[..i] {
            if same_identity(c, id, node(c, *p)?)? {
                return bad();
            }
        }
    }
    Ok(cost)
}
pub(in crate::coordinator::invocation) fn target(
    c: &InertCaptureV1,
    v: &C,
) -> Result<SettingTargetV1> {
    let rows = array(field(c, v, "layers")?)?;
    let mut layers = Vec::new();
    reserve(&mut layers, rows.len())?;
    for n in rows {
        layers.push(identity_copy(c, node(c, *n)?)?);
    }
    Ok(SettingTargetV1 {
        identity: identity_copy(c, field(c, v, "target")?)?,
        layers,
    })
}
pub(in crate::coordinator::invocation) fn declaration_workspace(
    c: &InertCaptureV1,
) -> Result<usize> {
    mul(
        c.usage().nodes,
        add(
            add(
                size_of::<Option<SettingTypeV1>>(),
                size_of::<Option<SettingValueV1>>(),
            )?,
            mul(2, size_of::<usize>())?,
        )?,
    )
}
pub(in crate::coordinator::invocation) fn declaration_cost(
    c: &InertCaptureV1,
    d: &C,
    label_limit: usize,
) -> Result<usize> {
    exact(d, &["schemas", "restrictions", "targets"])?;
    let schema_rows = array(field(c, d, "schemas")?)?;
    let restrictions = object(field(c, d, "restrictions")?)?;
    let target_rows = array(field(c, d, "targets")?)?;
    let mut owned = size_of::<SettingsDeclarationV1>();
    owned = add(owned, mul(schema_rows.len(), size_of::<SettingSchemaV1>())?)?;
    owned = add(
        owned,
        mul(
            restrictions.len(),
            size_of::<(String, Vec<SettingValueV1>)>(),
        )?,
    )?;
    owned = add(owned, mul(target_rows.len(), size_of::<SettingTargetV1>())?)?;
    for (i, n) in schema_rows.iter().enumerate() {
        let s = node(c, *n)?;
        owned = add(owned, schema_cost(c, s, label_limit)?)?;
        let key = text(field(c, s, "key")?)?;
        for p in &schema_rows[..i] {
            if text(field(c, node(c, *p)?, "key")?)? == key {
                return bad();
            }
        }
    }
    for (key, n) in restrictions {
        if key.is_empty() || key.len() > label_limit {
            return bad();
        }
        owned = add(owned, key.len())?;
        for n in array(node(c, *n)?)? {
            owned = add(owned, value_cost(c, *n)?)?;
        }
    }
    for (i, n) in target_rows.iter().enumerate() {
        let t = node(c, *n)?;
        owned = add(owned, target_cost(c, t, label_limit)?)?;
        let id = field(c, t, "target")?;
        for p in &target_rows[..i] {
            if same_identity(c, id, field(c, node(c, *p)?, "target")?)? {
                return bad();
            }
        }
    }
    Ok(owned)
}
pub(in crate::coordinator::invocation) fn declaration_copy(
    c: &InertCaptureV1,
    d: &C,
) -> Result<SettingsDeclarationV1> {
    let schema_rows = array(field(c, d, "schemas")?)?;
    let restrictions = object(field(c, d, "restrictions")?)?;
    let target_rows = array(field(c, d, "targets")?)?;
    let mut schemas = Vec::new();
    reserve(&mut schemas, schema_rows.len())?;
    let mut copied_restrictions = Vec::new();
    reserve(&mut copied_restrictions, restrictions.len())?;
    let mut targets = Vec::new();
    reserve(&mut targets, target_rows.len())?;
    for n in schema_rows {
        schemas.push(schema(c, node(c, *n)?)?);
    }
    for (key, n) in restrictions {
        let rows = array(node(c, *n)?)?;
        let mut values = Vec::new();
        reserve(&mut values, rows.len())?;
        for n in rows {
            values.push(value(c, *n)?);
        }
        copied_restrictions.push((copy(key)?, values));
    }
    for n in target_rows {
        targets.push(target(c, node(c, *n)?)?);
    }
    Ok(SettingsDeclarationV1 {
        schemas,
        restrictions: copied_restrictions,
        targets,
    })
}
fn normalize<'h>(
    envelope: &ScopedDefinitionEnvelopeValueV1<'_, 'h>,
    activation: SettingsActivationLimitsV1,
    profile: SettingsDefinitionLimitsV1,
    limits: CaptureLimitsV1,
    label_limit: usize,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedScopedDefinitionArgumentsV1<'h>> {
    use ScopedDefinitionEnvelopeValueV1 as V;
    let V::Object(rows) = envelope else {
        return bad();
    };
    let declaration = NormalizedScopedDefinitionArgumentsV1::ARGUMENT_DECLARATION;
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
    let input = [
        ("profile", data("profile")?),
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
    let workspace = declaration_workspace(&c)?;
    if add(c.usage().retained_bytes, workspace)? > maximum {
        return Err(InvocationError::Limit);
    }
    let execution = label(field(&c, c.root(), "profile")?, label_limit)?;
    let d = field(&c, c.root(), "definition")?;
    let owned = add(
        add(
            size_of::<NormalizedScopedDefinitionArgumentsV1>() - size_of::<SettingsDeclarationV1>(),
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
    Ok(NormalizedScopedDefinitionArgumentsV1 {
        invocation,
        dependencies,
        activation,
        profile,
        execution: copy(execution)?,
        declaration,
        usage: c.usage(),
        owned,
        peak,
    })
}

impl NormalizedScopedDefinitionArgumentsV1<'_> {
    pub(in crate::coordinator) const ARGUMENT_DECLARATION: &'static NativeArgumentDeclarationV1 =
        &ARGUMENTS;
    pub fn argument_declaration_v1(&self) -> &'static NativeArgumentDeclarationV1 {
        Self::ARGUMENT_DECLARATION
    }
}
pub fn normalize_scoped_definition_envelope_v1<'h>(
    envelope: &ScopedDefinitionEnvelopeValueV1<'_, 'h>,
    activation: SettingsActivationLimitsV1,
    profile: SettingsDefinitionLimitsV1,
    limits: CaptureLimitsV1,
    label_limit: usize,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedScopedDefinitionArgumentsV1<'h>> {
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
        Boundary::ScopedDefinitionArgumentNormalization,
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
    pub fn submit_normalized_scoped_definition_v1(
        &mut self,
        q: &NormalizedScopedDefinitionArgumentsV1<'_>,
        basis: &SettingsDefinitionBasisId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsDefinitionMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        let timer = r.begin_service_timing();
        let result = if q.execution != c.profile
            || q.activation != f.basis.common.activation
            || q.profile != f.basis.definition
        {
            Err(InvocationError::Unavailable)
        } else {
            self.submit_scoped_settings_definition(q.invocation, &q.request(basis), c, f, r)
        };
        record(
            r,
            Boundary::ScopedDefinitionDecodedAdmission,
            &result,
            timer.finish(),
        );
        result
    }
}
