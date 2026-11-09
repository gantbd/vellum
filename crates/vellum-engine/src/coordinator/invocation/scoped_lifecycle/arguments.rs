//! Closed passive lifecycle data; issuer conditions are a separate original role.
use super::super::settings_scoped::arguments::{
    add, copy, exact, field, index, mul, natural, node, reserve, text, value, value_cost,
};
use super::*;
use crate::capture::{
    CaptureErrorV1, CaptureLimitsV1, CaptureUsageV1, CapturedValueV1 as C, InertCaptureV1,
    InertValueV1 as D,
};
use crate::settings::{
    SettingContributionV1, SettingIdentityV1, SettingScopeV1, SettingValueV1,
    SettingsActivationLimitsV1, SettingsLayerLimitsV1,
};
use std::mem::size_of;
#[cfg(test)]
mod tests;
pub type ScopedLifecycleEnvelopeValueV1<'a, 'h> = ScopedSettingsEnvelopeValueV1<'a, 'h>;
static ARGUMENTS: NativeArgumentDeclarationV1 = NativeArgumentDeclarationV1 {
    contract: "scoped-engine-settings-layers/1",
    variant: NativeHandlerVariantDeclarationV1 {
        operation: "semantic-settings",
        fields: &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedSettingsVariant"),
            ("invocation", "InvocationIdentity"),
            ("profile", "ExecutionProfileRef"),
            ("create", "BoundedScopedLayerCreations"),
            ("remove", "BoundedScopedLayerRemovals"),
            ("targets", "CompleteScopedTargetAssociations"),
            ("dependencies", "BoundedContextDependencyVector"),
        ],
    },
};
pub struct NormalizedScopedLifecycleArgumentsV1<'h> {
    invocation: &'h InvocationId,
    dependencies: &'h ReadDependencyRequestV3,
    activation: SettingsActivationLimitsV1,
    profile: SettingsLayerLimitsV1,
    execution: String,
    create: Vec<ScopedSettingLayerCreateV1>,
    remove: Vec<SettingIdentityV1>,
    targets: Vec<ScopedSettingTargetV1>,
    usage: CaptureUsageV1,
    owned: usize,
    peak: usize,
}
impl std::fmt::Debug for NormalizedScopedLifecycleArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedScopedLifecycleArguments(<redacted>)")
    }
}
impl<'h> NormalizedScopedLifecycleArgumentsV1<'h> {
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
    pub fn native_profile(&self) -> SettingsLayerLimitsV1 {
        self.profile
    }
    pub fn create(&self) -> &[ScopedSettingLayerCreateV1] {
        &self.create
    }
    pub fn remove(&self) -> &[SettingIdentityV1] {
        &self.remove
    }
    pub fn targets(&self) -> &[ScopedSettingTargetV1] {
        &self.targets
    }
    pub fn capture_usage(&self) -> CaptureUsageV1 {
        self.usage
    }
    /// Conservative logical output envelope, including set-value root overlap.
    pub fn owned_bytes(&self) -> usize {
        self.owned
    }
    /// Joint inert/output/conversion envelope, not borrowed roles, caller ingress,
    /// stack, allocator/RSS or global sponsorship.
    pub fn normalization_peak_bytes(&self) -> usize {
        self.peak
    }
    pub fn request<'a>(
        &'a self,
        basis: &'a SettingsLifecycleBasisId,
    ) -> ScopedSettingsLifecycleRequestV1<'a> {
        ScopedSettingsLifecycleRequestV1 {
            basis,
            create: &self.create,
            remove: &self.remove,
            targets: &self.targets,
            dependencies: self.dependencies,
        }
    }
}
type Result<T> = std::result::Result<T, InvocationError>;
fn bad<T>() -> Result<T> {
    Err(InvocationError::MalformedIntent)
}
pub(in crate::coordinator::invocation) fn array(v: &C) -> Result<&[usize]> {
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
#[derive(PartialEq, Eq)]
pub(in crate::coordinator::invocation) struct Identity<'a> {
    authority: &'a str,
    id: &'a str,
    incarnation: u64,
}
impl Identity<'_> {
    pub(in crate::coordinator::invocation) fn payload(&self) -> Result<usize> {
        add(self.authority.len(), self.id.len())
    }
    pub(in crate::coordinator::invocation) fn copy(&self) -> Result<SettingIdentityV1> {
        Ok(SettingIdentityV1 {
            authority: copy(self.authority)?,
            id: copy(self.id)?,
            incarnation: self.incarnation,
        })
    }
}
pub(in crate::coordinator::invocation) fn identity<'a>(
    c: &'a InertCaptureV1,
    v: &C,
    limit: usize,
) -> Result<Identity<'a>> {
    exact(v, &["authority", "id", "incarnation"])?;
    let n = natural(field(c, v, "incarnation")?)?;
    if n == 0 {
        return bad();
    }
    Ok(Identity {
        authority: label(field(c, v, "authority")?, limit)?,
        id: label(field(c, v, "id")?, limit)?,
        incarnation: n as u64,
    })
}
pub(in crate::coordinator::invocation) fn scope(v: &C) -> Result<SettingScopeV1> {
    Ok(match text(v)? {
        "host" => SettingScopeV1::Host,
        "language" => SettingScopeV1::Language,
        "workspace" => SettingScopeV1::Workspace,
        "document" => SettingScopeV1::Document,
        "view" => SettingScopeV1::View,
        _ => return bad(),
    })
}
// None is explicit reset, Some points to a set value; absent contributions have
// no entry. The three meanings cannot alias through null or a missing value.
fn contribution(c: &InertCaptureV1, v: &C) -> Result<Option<usize>> {
    match text(field(c, v, "kind")?)? {
        "reset" => {
            exact(v, &["kind"])?;
            Ok(None)
        }
        "set" => {
            exact(v, &["kind", "value"])?;
            Ok(Some(index(v, "value")?))
        }
        _ => bad(),
    }
}
pub(in crate::coordinator::invocation) fn values_cost(
    c: &InertCaptureV1,
    v: &C,
    limit: usize,
) -> Result<usize> {
    let rows = object(v)?;
    let mut cost = mul(rows.len(), size_of::<(String, SettingContributionV1)>())?;
    for (k, n) in rows {
        if k.is_empty() || k.len() > limit {
            return bad();
        }
        cost = add(cost, k.len())?;
        if let Some(i) = contribution(c, node(c, *n)?)? {
            cost = add(cost, value_cost(c, i)?)?;
        }
    }
    Ok(cost)
}
pub(in crate::coordinator::invocation) fn values_copy(
    c: &InertCaptureV1,
    v: &C,
) -> Result<Vec<(String, SettingContributionV1)>> {
    let rows = object(v)?;
    let mut values = Vec::new();
    reserve(&mut values, rows.len())?;
    for (k, i) in rows {
        values.push((
            copy(k)?,
            match contribution(c, node(c, *i)?)? {
                None => SettingContributionV1::Reset,
                Some(n) => SettingContributionV1::Set(value(c, n)?),
            },
        ));
    }
    Ok(values)
}
fn creation_cost(c: &InertCaptureV1, v: &C, limit: usize) -> Result<usize> {
    exact(v, &["id", "kind", "values"])?;
    scope(field(c, v, "kind")?)?;
    add(
        label(field(c, v, "id")?, limit)?.len(),
        values_cost(c, field(c, v, "values")?, limit)?,
    )
}
fn creation(c: &InertCaptureV1, v: &C) -> Result<ScopedSettingLayerCreateV1> {
    Ok(ScopedSettingLayerCreateV1 {
        id: copy(text(field(c, v, "id")?)?)?,
        scope: scope(field(c, v, "kind")?)?,
        values: values_copy(c, field(c, v, "values")?)?,
    })
}
#[derive(PartialEq, Eq)]
enum Reference<'a> {
    Existing(Identity<'a>),
    Create(usize),
}
fn reference<'a>(
    c: &'a InertCaptureV1,
    v: &C,
    count: usize,
    limit: usize,
) -> Result<Reference<'a>> {
    let rows = object(v)?;
    if rows.len() == 1 && rows[0].0 == "create" {
        let i = natural(node(c, rows[0].1)?)?;
        if i >= count {
            return bad();
        }
        Ok(Reference::Create(i))
    } else {
        Ok(Reference::Existing(identity(c, v, limit)?))
    }
}
fn target_cost(c: &InertCaptureV1, v: &C, count: usize, limit: usize) -> Result<usize> {
    exact(v, &["target", "layers"])?;
    let mut cost = identity(c, field(c, v, "target")?, limit)?.payload()?;
    let rows = array(field(c, v, "layers")?)?;
    cost = add(
        cost,
        mul(rows.len(), size_of::<ScopedSettingLayerReferenceV1>())?,
    )?;
    for (i, n) in rows.iter().enumerate() {
        let r = reference(c, node(c, *n)?, count, limit)?;
        for prior in &rows[..i] {
            if reference(c, node(c, *prior)?, count, limit)? == r {
                return bad();
            }
        }
        if let Reference::Existing(id) = r {
            cost = add(cost, id.payload()?)?;
        }
    }
    Ok(cost)
}
fn target(c: &InertCaptureV1, v: &C, count: usize, limit: usize) -> Result<ScopedSettingTargetV1> {
    let rows = array(field(c, v, "layers")?)?;
    let mut layers = Vec::new();
    reserve(&mut layers, rows.len())?;
    for n in rows {
        layers.push(match reference(c, node(c, *n)?, count, limit)? {
            Reference::Create(index) => ScopedSettingLayerReferenceV1::Create { index },
            Reference::Existing(id) => ScopedSettingLayerReferenceV1::Existing(id.copy()?),
        });
    }
    Ok(ScopedSettingTargetV1 {
        identity: identity(c, field(c, v, "target")?, limit)?.copy()?,
        layers,
    })
}
fn normalize<'h>(
    envelope: &ScopedLifecycleEnvelopeValueV1<'_, 'h>,
    activation: SettingsActivationLimitsV1,
    profile: SettingsLayerLimitsV1,
    limits: CaptureLimitsV1,
    label_limit: usize,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedScopedLifecycleArgumentsV1<'h>> {
    use ScopedLifecycleEnvelopeValueV1 as V;
    let V::Object(rows) = envelope else {
        return bad();
    };
    let declaration = NormalizedScopedLifecycleArgumentsV1::ARGUMENT_DECLARATION;
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
    let execution = label(field(&c, c.root(), "profile")?, label_limit)?;
    let create_rows = array(field(&c, c.root(), "create")?)?;
    let remove_rows = array(field(&c, c.root(), "remove")?)?;
    let target_rows = array(field(&c, c.root(), "targets")?)?;
    if add(create_rows.len(), remove_rows.len())? > profile.max_changes {
        return Err(InvocationError::Limit);
    }
    let mut owned = add(
        size_of::<NormalizedScopedLifecycleArgumentsV1>(),
        execution.len(),
    )?;
    owned = add(
        owned,
        mul(create_rows.len(), size_of::<ScopedSettingLayerCreateV1>())?,
    )?;
    owned = add(
        owned,
        mul(remove_rows.len(), size_of::<SettingIdentityV1>())?,
    )?;
    owned = add(
        owned,
        mul(target_rows.len(), size_of::<ScopedSettingTargetV1>())?,
    )?;
    for n in create_rows {
        owned = add(owned, creation_cost(&c, node(&c, *n)?, label_limit)?)?;
    }
    for (i, n) in remove_rows.iter().enumerate() {
        let id = identity(&c, node(&c, *n)?, label_limit)?;
        for prior in &remove_rows[..i] {
            if identity(&c, node(&c, *prior)?, label_limit)? == id {
                return bad();
            }
        }
        owned = add(owned, id.payload()?)?;
    }
    for (i, n) in target_rows.iter().enumerate() {
        let t = node(&c, *n)?;
        owned = add(owned, target_cost(&c, t, create_rows.len(), label_limit)?)?;
        let id = identity(&c, field(&c, t, "target")?, label_limit)?;
        for prior in &target_rows[..i] {
            if identity(&c, field(&c, node(&c, *prior)?, "target")?, label_limit)? == id {
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
    reserve(&mut create, create_rows.len())?;
    let mut remove = Vec::new();
    reserve(&mut remove, remove_rows.len())?;
    let mut targets = Vec::new();
    reserve(&mut targets, target_rows.len())?;
    for n in create_rows {
        create.push(creation(&c, node(&c, *n)?)?);
    }
    for n in remove_rows {
        remove.push(identity(&c, node(&c, *n)?, label_limit)?.copy()?);
    }
    for n in target_rows {
        targets.push(target(&c, node(&c, *n)?, create_rows.len(), label_limit)?);
    }
    Ok(NormalizedScopedLifecycleArgumentsV1 {
        invocation,
        dependencies,
        activation,
        profile,
        execution: copy(execution)?,
        create,
        remove,
        targets,
        usage: c.usage(),
        owned,
        peak,
    })
}

impl NormalizedScopedLifecycleArgumentsV1<'_> {
    pub(in crate::coordinator) const ARGUMENT_DECLARATION: &'static NativeArgumentDeclarationV1 =
        &ARGUMENTS;
    pub fn argument_declaration_v1(&self) -> &'static NativeArgumentDeclarationV1 {
        Self::ARGUMENT_DECLARATION
    }
}
pub fn normalize_scoped_lifecycle_envelope_v1<'h>(
    envelope: &ScopedLifecycleEnvelopeValueV1<'_, 'h>,
    activation: SettingsActivationLimitsV1,
    profile: SettingsLayerLimitsV1,
    limits: CaptureLimitsV1,
    label_limit: usize,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedScopedLifecycleArgumentsV1<'h>> {
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
        Boundary::ScopedLifecycleArgumentNormalization,
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
    pub fn submit_normalized_scoped_lifecycle_v1(
        &mut self,
        q: &NormalizedScopedLifecycleArgumentsV1<'_>,
        basis: &SettingsLifecycleBasisId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        let timer = r.begin_service_timing();
        let result = if q.execution != c.profile
            || q.activation != f.basis.common.activation
            || q.profile != f.basis.lifecycle
        {
            Err(InvocationError::Unavailable)
        } else {
            self.submit_scoped_settings_lifecycle(q.invocation, &q.request(basis), c, f, r)
        };
        record(
            r,
            Boundary::ScopedLifecycleDecodedAdmission,
            &result,
            timer.finish(),
        );
        result
    }
}
