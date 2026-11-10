//! Native authored-layer lifecycle. Serialized envelopes and recipient grants are separate.
use super::compile::{MAX_SAFE, Meter, reserve};
use super::definition::{TargetInput, compare, transition};
use super::mutate::copy;
use super::*;
use std::sync::Arc;
#[cfg(test)]
mod tests;
mod transition_layers;
pub struct SettingLayerCreateV1 {
    pub identity: SettingIdentityV1,
    pub scope: SettingScopeV1,
    pub values: Vec<(String, SettingContributionV1)>,
}
#[derive(Clone, Copy)]
pub(crate) struct LayerCreateInput<'a> {
    pub(crate) identity: &'a SettingIdentityV1,
    pub(crate) scope: SettingScopeV1,
    pub(crate) values: &'a [(String, SettingContributionV1)],
}
pub struct SettingLayerRemoveV1 {
    pub identity: SettingIdentityV1,
    pub expected_generation: u64,
}
pub struct SettingsLayerChangeV1 {
    pub create: Vec<SettingLayerCreateV1>,
    pub remove: Vec<SettingLayerRemoveV1>,
    pub targets: Vec<SettingTargetV1>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsLayerLimitsV1 {
    pub max_changes: usize,
    /// Native logical created-value object bytes, not a selected JSON/wire encoding.
    pub max_created_value_bytes: usize,
    pub max_request_bytes: usize,
    pub max_plan_bytes: usize,
    pub max_work: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsLayerBasisV1 {
    pub configuration_generation: u64,
    pub layer_incarnation: u64,
}
impl SettingsLayerBasisV1 {
    pub(crate) fn validate(self) -> Result<(), SettingsError> {
        if self.configuration_generation == 0
            || self.configuration_generation > MAX_SAFE as u64
            || self.layer_incarnation > MAX_SAFE as u64
        {
            Err(SettingsError::InvalidIdentity)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsLayerOutcomeV1 {
    pub changed: bool,
    pub basis: SettingsBasisV1,
    pub layer_incarnation: u64,
    pub created_layers: usize,
    pub removed_layers: usize,
    pub associations_changed: bool,
    pub changed_domains: usize,
    pub added_targets: usize,
    pub removed_targets: usize,
}
impl CompiledSettingsV1 {
    pub(crate) fn layer_basis(&self) -> SettingsLayerBasisV1 {
        SettingsLayerBasisV1 {
            configuration_generation: self.basis.configuration_generation,
            layer_incarnation: self.layer_incarnation,
        }
    }
}
struct Create {
    identity: SettingIdentityV1,
    scope: SettingScopeV1,
    values: Vec<(String, Contribution)>,
}
pub(crate) struct LayerCapture {
    data: CompiledSettingsV1,
    create: Vec<Create>,
    remove: Vec<SettingLayerRemoveV1>,
    targets: Vec<TargetInput>,
    pub(crate) limits: SettingsLayerLimitsV1,
    bytes: usize,
}
impl std::fmt::Debug for LayerCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LayerCapture(<redacted>)")
    }
}
fn meter(state: &CompiledSettingsV1, p: SettingsLayerLimitsV1) -> Meter {
    Meter::new(SettingsLimitsV1 {
        max_work: p.max_work.min(state.limits.max_work),
        ..state.limits
    })
}
impl LayerCapture {
    pub(crate) fn captured_limits(&self) -> SettingsLimitsV1 {
        self.data.limits
    }
    pub(crate) fn created_identities(&self) -> impl ExactSizeIterator<Item = &SettingIdentityV1> {
        self.create.iter().map(|r| &r.identity)
    }
    pub(crate) fn removals(&self) -> &[SettingLayerRemoveV1] {
        &self.remove
    }
    pub(crate) fn recopy_borrowed(
        &self,
        create: &[LayerCreateInput<'_>],
        targets: &[SettingTargetV1],
        ceiling: usize,
    ) -> Result<Self, SettingsError> {
        Self::copy_borrowed(
            create,
            &self.remove,
            targets,
            &self.data,
            self.limits,
            ceiling,
        )
    }
    pub(crate) fn copy_borrowed(
        create: &[LayerCreateInput<'_>],
        remove: &[SettingLayerRemoveV1],
        targets: &[SettingTargetV1],
        state: &CompiledSettingsV1,
        p: SettingsLayerLimitsV1,
        ceiling: usize,
    ) -> Result<Self, SettingsError> {
        Self::copy_rows(
            create.len(),
            create.iter().copied(),
            remove,
            targets,
            state,
            p,
            ceiling,
        )
    }
    pub(crate) fn bytes(&self) -> usize {
        self.bytes
    }
    pub(crate) fn recopy(
        &self,
        input: &SettingsLayerChangeV1,
        ceiling: usize,
    ) -> Result<Self, SettingsError> {
        Self::copy(input, &self.data, self.limits, ceiling)
    }
    pub(crate) fn copy(
        input: &SettingsLayerChangeV1,
        state: &CompiledSettingsV1,
        p: SettingsLayerLimitsV1,
        ceiling: usize,
    ) -> Result<Self, SettingsError> {
        Self::copy_rows(
            input.create.len(),
            input.create.iter().map(|r| LayerCreateInput {
                identity: &r.identity,
                scope: r.scope,
                values: &r.values,
            }),
            &input.remove,
            &input.targets,
            state,
            p,
            ceiling,
        )
    }
    fn copy_rows<'a>(
        create_count: usize,
        creates: impl Iterator<Item = LayerCreateInput<'a>>,
        removes: &[SettingLayerRemoveV1],
        target_rows: &[SettingTargetV1],
        state: &CompiledSettingsV1,
        p: SettingsLayerLimitsV1,
        ceiling: usize,
    ) -> Result<Self, SettingsError> {
        if create_count
            .checked_add(removes.len())
            .is_none_or(|n| n > p.max_changes)
            || create_count > state.limits.max_layers
            || removes.len() > state.limits.max_layers
        {
            return Err(SettingsError::Limit);
        }
        let mut m = Meter::new(SettingsLimitsV1 {
            max_state_bytes: p.max_request_bytes.min(ceiling),
            max_work: p.max_work.min(state.limits.max_work),
            ..state.limits
        });
        m.bytes(std::mem::size_of::<Self>())?;
        let mut data = CompiledSettingsV1 {
            layer_incarnation: 0,
            next_target_identity: 0,
            basis: SettingsBasisV1::initial(),
            authority: m.identifier(&state.authority)?,
            limits: state.limits,
            nodes: vec![],
            types: vec![],
            schemas: vec![],
            restrictions: vec![],
            layers: vec![],
            targets: vec![],
            logical_bytes: 0,
        };
        m.edges::<Create>(create_count)?;
        let mut create = Vec::new();
        reserve(&mut create, create_count)?;
        let mut value_bytes = 0usize;
        for row in creates {
            m.tick()?;
            let identity = compile::identity(row.identity, &row.identity.authority, &mut m)?;
            if row.values.len() > state.limits.max_keys {
                return Err(SettingsError::Limit);
            }
            let start = m.used_bytes();
            m.bytes(std::mem::size_of::<Vec<(String, Contribution)>>())?;
            m.edges::<(String, Contribution)>(row.values.len())?;
            let mut values: Vec<(String, Contribution)> = Vec::new();
            reserve(&mut values, row.values.len())?;
            for (key, contribution) in row.values {
                m.tick()?;
                for (old, _) in &values {
                    m.tick()?;
                    if old == key {
                        return Err(SettingsError::Duplicate);
                    }
                }
                let key = m.identifier(key)?;
                let c = match contribution {
                    SettingContributionV1::Reset => Contribution::Reset,
                    SettingContributionV1::Set(v) => {
                        Contribution::Set(compile::value(&mut data, v, &mut m)?)
                    }
                };
                values.push((key, c));
            }
            value_bytes = value_bytes
                .checked_add(m.used_bytes() - start)
                .filter(|n| *n <= p.max_created_value_bytes)
                .ok_or(SettingsError::Limit)?;
            create.push(Create {
                identity,
                scope: row.scope,
                values,
            });
        }
        let mut remove = Vec::new();
        m.edges::<SettingLayerRemoveV1>(removes.len())?;
        reserve(&mut remove, removes.len())?;
        for row in removes {
            m.tick()?;
            if row.expected_generation == 0 || row.expected_generation > MAX_SAFE as u64 {
                return Err(SettingsError::InvalidIdentity);
            }
            remove.push(SettingLayerRemoveV1 {
                identity: compile::identity(&row.identity, &row.identity.authority, &mut m)?,
                expected_generation: row.expected_generation,
            });
        }
        let targets = definition::copy_targets(target_rows, &mut m)?;
        let bytes = m.used_bytes();
        data.logical_bytes = bytes;
        Ok(Self {
            data,
            create,
            remove,
            targets,
            limits: p,
            bytes,
        })
    }
    pub(crate) fn same(&self, other: &Self) -> Result<bool, SettingsError> {
        if self.limits != other.limits
            || self.data.limits != other.data.limits
            || self.data.authority != other.data.authority
            || self.create.len() != other.create.len()
            || self.remove.len() != other.remove.len()
            || self.targets.len() != other.targets.len()
        {
            return Ok(false);
        }
        let mut m = meter(&self.data, self.limits);
        for (a, b) in self.create.iter().zip(&other.create) {
            m.tick()?;
            if a.identity != b.identity || a.scope != b.scope || a.values.len() != b.values.len() {
                return Ok(false);
            }
            for (key, v) in &a.values {
                let mut found = None;
                for (other, value) in &b.values {
                    m.tick()?;
                    if key == other {
                        found = Some(value);
                        break;
                    }
                }
                match (v, found) {
                    (Contribution::Reset, Some(Contribution::Reset)) => {}
                    (Contribution::Set(a), Some(Contribution::Set(b)))
                        if resolve::equal_between(
                            &self.data,
                            *a,
                            &other.data,
                            *b,
                            &mut m,
                            true,
                        )? => {}
                    _ => return Ok(false),
                }
            }
        }
        for (a, b) in self.remove.iter().zip(&other.remove) {
            m.tick()?;
            if a.identity != b.identity || a.expected_generation != b.expected_generation {
                return Ok(false);
            }
        }
        for (a, b) in self.targets.iter().zip(&other.targets) {
            m.tick()?;
            m.steps(a.layers.len())?;
            if a.identity != b.identity || a.layers != b.layers {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
pub(crate) struct LayerPlan {
    before: Arc<CompiledSettingsV1>,
    capture: Arc<LayerCapture>,
    expected: SettingsLayerBasisV1,
    pub(crate) limits: SettingsLayerLimitsV1,
    pub(crate) would_change: bool,
    pub(crate) candidate_logical_bytes: usize,
}
impl std::fmt::Debug for LayerPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LayerPlan(<redacted>)")
    }
}
impl LayerPlan {
    pub(crate) fn captured_intent(&self) -> (&Arc<LayerCapture>, SettingsLayerBasisV1) {
        (&self.capture, self.expected)
    }
    pub(crate) fn retained_bytes(&self) -> Result<usize, SettingsError> {
        self.before
            .logical_bytes
            .checked_add(self.capture.bytes)
            .and_then(|n| n.checked_add(std::mem::size_of::<Self>()))
            .ok_or(SettingsError::Limit)
    }
    pub(crate) fn prepare(
        before: Arc<CompiledSettingsV1>,
        capture: Arc<LayerCapture>,
        expected: SettingsLayerBasisV1,
    ) -> Result<Self, SettingsError> {
        expected.validate()?;
        if before.layer_basis() != expected {
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
        capture: &Arc<LayerCapture>,
        expected: SettingsLayerBasisV1,
    ) -> bool {
        self.expected == expected
            && self.limits == capture.limits
            && Arc::ptr_eq(&self.capture, capture)
    }
    pub(crate) fn decide(
        &self,
        current: &CompiledSettingsV1,
    ) -> Result<(Option<CompiledSettingsV1>, SettingsLayerOutcomeV1), SettingsError> {
        if current.basis != self.before.basis
            || current.layer_basis() != self.expected
            || current.limits != self.before.limits
            || current.next_target_identity != self.before.next_target_identity
        {
            return Err(SettingsError::Stale);
        }
        transition_layers::decide(current, &self.capture, &mut meter(current, self.limits))
    }
}
