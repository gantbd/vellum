//! Flat owned declarations and live replacement, never initializer installation.
use super::compile::{MAX_SAFE, Meter, reserve};
use super::mutate::copy;
use super::*;
use std::sync::Arc;
pub(super) mod compare;
#[cfg(test)]
mod tests;
pub(super) mod transition;
pub struct SettingsDeclarationV1 {
    pub schemas: Vec<SettingSchemaV1>,
    pub restrictions: Vec<(String, Vec<SettingValueV1>)>,
    pub targets: Vec<SettingTargetV1>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsDefinitionLimitsV1 {
    pub max_request_bytes: usize,
    pub max_plan_bytes: usize,
    pub max_work: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsDefinitionOutcomeV1 {
    pub changed: bool,
    pub basis: SettingsBasisV1,
    pub schema_changed: bool,
    pub restrictions_changed: bool,
    pub associations_changed: bool,
    pub changed_domains: usize,
    pub added_targets: usize,
    pub removed_targets: usize,
}
pub(super) struct TargetInput {
    pub(super) identity: SettingIdentityV1,
    pub(super) layers: Vec<SettingIdentityV1>,
}
pub(crate) struct DefinitionCapture {
    data: CompiledSettingsV1,
    restrictions: Vec<(String, Vec<usize>)>,
    targets: Vec<TargetInput>,
    pub(crate) limits: SettingsDefinitionLimitsV1,
    bytes: usize,
}
impl std::fmt::Debug for DefinitionCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DefinitionCapture(<redacted>)")
    }
}
impl DefinitionCapture {
    pub(crate) fn validate_expected(
        expected: SettingsInterpretationEpochsV1,
    ) -> Result<(), SettingsError> {
        if [expected.schema, expected.restriction, expected.association]
            .into_iter()
            .any(|v| v == 0 || v > MAX_SAFE as u64)
        {
            Err(SettingsError::InvalidIdentity)
        } else {
            Ok(())
        }
    }
    pub(crate) fn from_state(
        input: &SettingsDeclarationV1,
        state: &CompiledSettingsV1,
        p: SettingsDefinitionLimitsV1,
        ceiling: usize,
    ) -> Result<Self, SettingsError> {
        Self::copy(input, &state.authority, state.limits, p, ceiling)
    }
    pub(crate) fn recopy(
        &self,
        input: &SettingsDeclarationV1,
        ceiling: usize,
    ) -> Result<Self, SettingsError> {
        Self::from_state(input, &self.data, self.limits, ceiling)
    }
    pub(crate) fn bytes(&self) -> usize {
        self.bytes
    }
    pub(crate) fn copy(
        input: &SettingsDeclarationV1,
        authority: &str,
        installed: SettingsLimitsV1,
        p: SettingsDefinitionLimitsV1,
        ceiling: usize,
    ) -> Result<Self, SettingsError> {
        let mut m = Meter::new(SettingsLimitsV1 {
            max_state_bytes: p.max_request_bytes.min(ceiling),
            max_work: p.max_work.min(installed.max_work),
            ..installed
        });
        if input.schemas.len() > installed.max_keys
            || input.restrictions.len() > installed.max_keys
            || input.targets.len() > installed.max_targets
        {
            return Err(SettingsError::Limit);
        }
        m.bytes(std::mem::size_of::<Self>())?;
        let mut data = CompiledSettingsV1 {
            layer_incarnation: 0,
            next_target_identity: 0,
            basis: SettingsBasisV1::initial(),
            authority: m.identifier(authority)?,
            limits: installed,
            nodes: Vec::new(),
            types: Vec::new(),
            schemas: Vec::new(),
            restrictions: Vec::new(),
            layers: Vec::new(),
            targets: Vec::new(),
            logical_bytes: 0,
        };
        m.edges::<Schema>(input.schemas.len())?;
        reserve(&mut data.schemas, input.schemas.len())?;
        for s in &input.schemas {
            m.tick()?;
            let key = m.identifier(&s.key)?;
            let value_type = compile::copy_type(&mut data, &s.value_type, &mut m, false)?;
            let default = compile::value(&mut data, &s.default, &mut m)?;
            if s.scopes.len() > 5 {
                return Err(SettingsError::Limit);
            }
            m.edges::<SettingScopeV1>(s.scopes.len())?;
            let mut scopes = Vec::new();
            reserve(&mut scopes, s.scopes.len())?;
            scopes.extend_from_slice(&s.scopes);
            data.schemas.push(Schema {
                key,
                value_type,
                default,
                scopes,
                domain: s.domain,
                composition: s.composition,
                sensitive: s.sensitive,
            });
        }
        let mut restrictions: Vec<(String, Vec<usize>)> = Vec::new();
        m.edges::<(String, Vec<usize>)>(input.restrictions.len())?;
        reserve(&mut restrictions, input.restrictions.len())?;
        for (key, values) in &input.restrictions {
            m.tick()?;
            for (old, _) in &restrictions {
                m.tick()?;
                if old == key {
                    return Err(SettingsError::Duplicate);
                }
            }
            let key = m.identifier(key)?;
            m.edges::<usize>(values.len())?;
            let mut copied = Vec::new();
            reserve(&mut copied, values.len())?;
            for v in values {
                copied.push(compile::value(&mut data, v, &mut m)?);
            }
            restrictions.push((key, copied));
        }
        let targets = copy_targets(&input.targets, &mut m)?;
        data.logical_bytes = m.used_bytes();
        let bytes = m.used_bytes();
        Ok(Self {
            data,
            restrictions,
            targets,
            limits: p,
            bytes,
        })
    }
    pub(crate) fn same(&self, other: &Self) -> Result<bool, SettingsError> {
        if self.limits != other.limits
            || self.data.limits != other.data.limits
            || self.data.authority != other.data.authority
        {
            return Ok(false);
        }
        let mut m = meter(&self.data, self.limits);
        Ok(compare::schemas(&self.data, &other.data, &mut m, true)?
            && compare::raw_restrictions(self, other, &mut m, true)?
            && compare::raw_targets(self, other, &mut m)?)
    }
}
pub(super) fn copy_targets(
    input: &[SettingTargetV1],
    m: &mut Meter,
) -> Result<Vec<TargetInput>, SettingsError> {
    if input.len() > m.limits.max_targets {
        return Err(SettingsError::Limit);
    }
    let mut targets = Vec::new();
    m.edges::<TargetInput>(input.len())?;
    reserve(&mut targets, input.len())?;
    for target in input {
        m.tick()?;
        let identity = compile::identity(&target.identity, &target.identity.authority, m)?;
        if target.layers.len() > m.limits.max_layers {
            return Err(SettingsError::Limit);
        }
        let mut layers = Vec::new();
        m.edges::<SettingIdentityV1>(target.layers.len())?;
        reserve(&mut layers, target.layers.len())?;
        for layer in &target.layers {
            m.tick()?;
            layers.push(compile::identity(layer, &layer.authority, m)?);
        }
        targets.push(TargetInput { identity, layers });
    }
    Ok(targets)
}
fn meter(state: &CompiledSettingsV1, p: SettingsDefinitionLimitsV1) -> Meter {
    Meter::new(SettingsLimitsV1 {
        max_work: p.max_work.min(state.limits.max_work),
        ..state.limits
    })
}
pub(crate) struct DefinitionPlan {
    before: Arc<CompiledSettingsV1>,
    capture: Arc<DefinitionCapture>,
    expected: SettingsInterpretationEpochsV1,
    pub(crate) limits: SettingsDefinitionLimitsV1,
    pub(crate) would_change: bool,
    pub(crate) candidate_logical_bytes: usize,
}
impl std::fmt::Debug for DefinitionPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DefinitionPlan(<redacted>)")
    }
}
impl DefinitionPlan {
    pub(crate) fn retained_bytes(&self) -> Result<usize, SettingsError> {
        self.before
            .logical_bytes
            .checked_add(self.capture.bytes)
            .and_then(|n| n.checked_add(std::mem::size_of::<Self>()))
            .ok_or(SettingsError::Limit)
    }
    pub(crate) fn prepare(
        before: Arc<CompiledSettingsV1>,
        capture: Arc<DefinitionCapture>,
        expected: SettingsInterpretationEpochsV1,
    ) -> Result<Self, SettingsError> {
        DefinitionCapture::validate_expected(expected)?;
        if before.basis.interpretation() != expected {
            return Err(SettingsError::Stale);
        }
        let limits = capture.limits;
        let mut plan = Self {
            before,
            capture,
            expected,
            limits,
            would_change: false,
            candidate_logical_bytes: 0,
        };
        if plan.retained_bytes()? > limits.max_plan_bytes {
            return Err(SettingsError::Limit);
        }
        let (candidate, outcome) = plan.decide(&plan.before)?;
        plan.would_change = outcome.changed;
        plan.candidate_logical_bytes = candidate
            .as_ref()
            .map_or(plan.before.logical_bytes, |c| c.logical_bytes);
        Ok(plan)
    }
    pub(crate) fn matches(
        &self,
        capture: &Arc<DefinitionCapture>,
        expected: SettingsInterpretationEpochsV1,
    ) -> bool {
        self.expected == expected
            && self.limits == capture.limits
            && Arc::ptr_eq(&self.capture, capture)
    }
    pub(crate) fn decide(
        &self,
        current: &CompiledSettingsV1,
    ) -> Result<(Option<CompiledSettingsV1>, SettingsDefinitionOutcomeV1), SettingsError> {
        if current.basis != self.before.basis
            || current.limits != self.before.limits
            || current.next_target_identity != self.before.next_target_identity
            || current.layers.len() != self.before.layers.len()
        {
            return Err(SettingsError::Stale);
        }
        let mut m = meter(current, self.limits);
        for (a, b) in current.layers.iter().zip(&self.before.layers) {
            m.tick()?;
            if a.identity != b.identity || a.generation != b.generation {
                return Err(SettingsError::Stale);
            }
        }
        transition::decide(current, &self.capture, &mut m)
    }
}
