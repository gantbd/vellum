use super::compile::{self, MAX_SAFE, Meter, reserve};
use super::*;
use std::sync::Arc;
pub(super) mod copy;
#[cfg(test)]
mod tests;
#[cfg(test)]
impl CompiledSettingsV1 {
    pub(crate) fn flat_clone_for_test(&self) -> Result<Self, SettingsError> {
        let p = SettingsActivationLimitsV1 {
            max_changes: 0,
            max_request_nodes: 0,
            max_request_depth: 0,
            max_request_bytes: self.limits.max_state_bytes,
            max_plan_bytes: usize::MAX,
            max_work: self.limits.max_work,
        };
        let empty = request(self, &[], p)?;
        let mut meter = Meter::new(self.limits);
        let mut copied = copy::state(self, &empty.changes, &mut meter)?;
        resolve::resolve_all(&mut copied, &mut meter)?;
        copied.logical_bytes = meter.used_bytes();
        Ok(copied)
    }
}
pub enum SettingOperationV1 {
    Set(SettingValueV1),
    Remove,
    Reset,
}
pub struct SettingChangeV1 {
    pub layer: SettingIdentityV1,
    pub expected_generation: u64,
    pub key: String,
    pub operation: SettingOperationV1,
}
#[derive(Clone, Copy)]
pub(crate) struct SettingChangeInput<'a> {
    pub layer: &'a SettingIdentityV1,
    pub expected_generation: u64,
    pub key: &'a str,
    pub operation: &'a SettingOperationV1,
}
impl<'a> From<&'a SettingChangeV1> for SettingChangeInput<'a> {
    fn from(value: &'a SettingChangeV1) -> Self {
        Self {
            layer: &value.layer,
            expected_generation: value.expected_generation,
            key: &value.key,
            operation: &value.operation,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsActivationLimitsV1 {
    pub max_changes: usize,
    pub max_request_nodes: usize,
    pub max_request_depth: usize,
    pub max_request_bytes: usize,
    pub max_plan_bytes: usize,
    pub max_work: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsUpdateV1 {
    pub changed: bool,
    pub basis: SettingsBasisV1,
    pub changed_layers: usize,
    pub changed_domains: usize,
}
#[derive(Clone, Copy)]
enum Operation {
    Set(usize),
    Remove,
    Reset,
}
#[derive(Clone, Copy)]
struct Change {
    layer: usize,
    expected: u64,
    key: usize,
    operation: Operation,
}
struct Request {
    changes: Vec<Change>,
    values: CompiledSettingsV1,
}
struct RequestView<'a> {
    changes: &'a [Change],
    values: &'a CompiledSettingsV1,
}
pub(crate) struct MutationPlan {
    before: Arc<CompiledSettingsV1>,
    request: Request,
    needed: Vec<usize>,
    pub(crate) limits: SettingsActivationLimitsV1,
    pub(crate) would_change: bool,
    pub(crate) candidate_logical_bytes: usize,
}
impl std::fmt::Debug for MutationPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MutationPlan(<redacted>)")
    }
}
fn blank(source: &CompiledSettingsV1, m: &mut Meter) -> Result<CompiledSettingsV1, SettingsError> {
    m.bytes(std::mem::size_of::<CompiledSettingsV1>())?;
    Ok(CompiledSettingsV1 {
        layer_incarnation: source.layer_incarnation,
        next_target_identity: source.next_target_identity,
        basis: source.basis,
        authority: m.identifier(&source.authority)?,
        limits: source.limits,
        nodes: Vec::new(),
        types: Vec::new(),
        schemas: Vec::new(),
        restrictions: Vec::new(),
        layers: Vec::new(),
        targets: Vec::new(),
        logical_bytes: 0,
    })
}
fn request(
    before: &CompiledSettingsV1,
    changes: &[SettingChangeV1],
    limits: SettingsActivationLimitsV1,
) -> Result<Request, SettingsError> {
    request_inputs(before, changes.iter().map(SettingChangeInput::from), limits)
}
fn request_inputs<'a>(
    before: &CompiledSettingsV1,
    changes: impl ExactSizeIterator<Item = SettingChangeInput<'a>>,
    limits: SettingsActivationLimitsV1,
) -> Result<Request, SettingsError> {
    if changes.len() > limits.max_changes {
        return Err(SettingsError::Limit);
    }
    let mut m = Meter::new(SettingsLimitsV1 {
        max_value_nodes: limits.max_request_nodes.min(before.limits.max_value_nodes),
        max_value_depth: limits.max_request_depth.min(before.limits.max_value_depth),
        max_state_bytes: limits.max_request_bytes,
        max_work: limits.max_work.min(before.limits.max_work),
        ..before.limits
    });
    let mut values = blank(before, &mut m)?;
    m.edges::<Change>(changes.len())?;
    let mut owned: Vec<Change> = Vec::new();
    reserve(&mut owned, changes.len())?;
    for input in changes {
        m.tick()?;
        m.bytes(
            input
                .layer
                .authority
                .len()
                .checked_add(input.layer.id.len())
                .and_then(|n| n.checked_add(input.key.len()))
                .ok_or(SettingsError::Limit)?,
        )?;
        if input.expected_generation == 0 || input.expected_generation > MAX_SAFE as u64 {
            return Err(SettingsError::InvalidIdentity);
        }
        let mut layer = None;
        for (i, l) in before.layers.iter().enumerate() {
            m.tick()?;
            if l.identity == *input.layer {
                layer = Some(i);
                break;
            }
        }
        let layer = layer.ok_or(SettingsError::UnknownLayer)?;
        if before.layers[layer].generation != input.expected_generation {
            return Err(SettingsError::Stale);
        }
        let mut key = None;
        for (i, s) in before.schemas.iter().enumerate() {
            m.tick()?;
            if s.key == input.key {
                key = Some(i);
                break;
            }
        }
        let key = key.ok_or(SettingsError::UnknownKey)?;
        if !before.schemas[key]
            .scopes
            .contains(&before.layers[layer].scope)
        {
            return Err(SettingsError::IllegalScope);
        }
        for old in &owned {
            m.tick()?;
            if old.layer == layer && old.key == key {
                return Err(SettingsError::Duplicate);
            }
        }
        let operation = match input.operation {
            SettingOperationV1::Remove => Operation::Remove,
            SettingOperationV1::Reset => Operation::Reset,
            SettingOperationV1::Set(value) => {
                Operation::Set(compile::value(&mut values, value, &mut m)?)
            }
        };
        owned.push(Change {
            layer,
            expected: input.expected_generation,
            key,
            operation,
        });
    }
    values.logical_bytes = m.used_bytes();
    Ok(Request {
        changes: owned,
        values,
    })
}
fn footprint(
    before: &CompiledSettingsV1,
    request: &Request,
    m: &mut Meter,
) -> Result<Vec<usize>, SettingsError> {
    if request.changes.is_empty() {
        return Ok(Vec::new());
    }
    let mut flags = Vec::new();
    reserve(&mut flags, before.layers.len())?;
    flags.resize(before.layers.len(), false);
    for change in &request.changes {
        m.tick()?;
        flags[change.layer] = true;
    }
    for target in &before.targets {
        let mut affected = false;
        for layer in &target.layers {
            for change in &request.changes {
                m.tick()?;
                affected |= *layer == change.layer;
            }
        }
        if affected {
            for layer in &target.layers {
                m.tick()?;
                flags[*layer] = true;
            }
        }
    }
    let mut needed = Vec::new();
    for (i, flag) in flags.into_iter().enumerate() {
        m.tick()?;
        if flag {
            reserve(&mut needed, 1)?;
            needed.push(i);
        }
    }
    Ok(needed)
}
fn next(value: u64) -> Result<u64, SettingsError> {
    value
        .checked_add(1)
        .filter(|v| *v <= MAX_SAFE as u64)
        .ok_or(SettingsError::VersionExhausted)
}
fn layers_equal(
    a: &CompiledSettingsV1,
    layer: usize,
    b: &CompiledSettingsV1,
    m: &mut Meter,
) -> Result<bool, SettingsError> {
    let a_rows = &a.layers[layer].values;
    let b_rows = &b.layers[layer].values;
    if a_rows.len() != b_rows.len() {
        return Ok(false);
    }
    for (key, value) in a_rows {
        let mut found = None;
        for (other, value) in b_rows {
            m.tick()?;
            if other == key {
                found = Some(value);
                break;
            }
        }
        let Some(other) = found else { return Ok(false) };
        match (value, other) {
            (Contribution::Reset, Contribution::Reset) => {}
            (Contribution::Set(a_node), Contribution::Set(b_node)) => {
                if !resolve::equal_between(a, *a_node, b, *b_node, m, false)? {
                    return Ok(false);
                }
            }
            _ => return Ok(false),
        }
    }
    Ok(true)
}
fn decide(
    current: &CompiledSettingsV1,
    request: RequestView<'_>,
    limits: SettingsActivationLimitsV1,
) -> Result<(Option<CompiledSettingsV1>, SettingsUpdateV1), SettingsError> {
    let mut m = Meter::new(SettingsLimitsV1 {
        max_work: limits.max_work.min(current.limits.max_work),
        ..current.limits
    });
    let mut identity = true;
    for change in request.changes {
        m.tick()?;
        if current.layers[change.layer].generation != change.expected {
            return Err(SettingsError::Stale);
        }
        let mut old = None;
        for (key, value) in &current.layers[change.layer].values {
            m.tick()?;
            if *key == change.key {
                old = Some(value);
                break;
            }
        }
        identity &= match (&change.operation, old) {
            (Operation::Remove, None) | (Operation::Reset, Some(Contribution::Reset)) => true,
            (Operation::Set(new), Some(Contribution::Set(old))) => {
                resolve::equal_between(current, *old, request.values, *new, &mut m, false)?
            }
            _ => false,
        };
    }
    if identity {
        return Ok((
            None,
            SettingsUpdateV1 {
                changed: false,
                basis: current.basis,
                changed_layers: 0,
                changed_domains: 0,
            },
        ));
    }
    let mut candidate = copy::state(current, request.changes, &mut m)?;
    let mut nodes = copy::cache(request.values, &mut m)?;
    for change in request.changes {
        m.tick()?;
        if current.layers[change.layer].generation != change.expected {
            return Err(SettingsError::Stale);
        }
        let contribution = match change.operation {
            Operation::Remove => continue,
            Operation::Reset => Contribution::Reset,
            Operation::Set(value) => {
                let value =
                    copy::node_from(request.values, value, &mut candidate, &mut nodes, &mut m)?;
                resolve::typed(
                    &candidate,
                    candidate.schemas[change.key].value_type,
                    value,
                    candidate.schemas[change.key].composition == SettingCompositionV1::MergeMap,
                    &mut m,
                )?;
                Contribution::Set(value)
            }
        };
        m.edges::<(usize, Contribution)>(1)?;
        reserve(&mut candidate.layers[change.layer].values, 1)?;
        candidate.layers[change.layer]
            .values
            .push((change.key, contribution));
    }
    resolve::resolve_all(&mut candidate, &mut m)?;
    let mut changed_layers = 0usize;
    for layer in 0..current.layers.len() {
        m.tick()?;
        if !layers_equal(current, layer, &candidate, &mut m)? {
            candidate.layers[layer].generation = next(current.layers[layer].generation)?;
            changed_layers = changed_layers.checked_add(1).ok_or(SettingsError::Limit)?;
        }
    }
    if changed_layers == 0 {
        return Ok((
            None,
            SettingsUpdateV1 {
                changed: false,
                basis: current.basis,
                changed_layers: 0,
                changed_domains: 0,
            },
        ));
    }
    candidate.basis.configuration_generation = next(current.basis.configuration_generation)?;
    let mut changed_domains = 0usize;
    for target in 0..current.targets.len() {
        let mut flags = [false; 3];
        for key in 0..current.schemas.len() {
            m.tick()?;
            if !resolve::equal_between(
                current,
                current.targets[target].resolved[key].value,
                &candidate,
                candidate.targets[target].resolved[key].value,
                &mut m,
                false,
            )? {
                let domain = match current.schemas[key].domain {
                    SettingDomainV1::Source => 0,
                    SettingDomainV1::Serialization => 1,
                    SettingDomainV1::Analysis => 2,
                };
                flags[domain] = true;
            }
        }
        for (domain, changed) in flags.into_iter().enumerate() {
            if changed {
                candidate.targets[target].generations[domain] =
                    next(current.targets[target].generations[domain])?;
                changed_domains = changed_domains.checked_add(1).ok_or(SettingsError::Limit)?;
            }
        }
    }
    candidate.logical_bytes = m.used_bytes();
    let outcome = SettingsUpdateV1 {
        changed: true,
        basis: candidate.basis,
        changed_layers,
        changed_domains,
    };
    Ok((Some(candidate), outcome))
}
impl MutationPlan {
    pub(crate) fn retained_bytes(&self) -> Result<usize, SettingsError> {
        self.before
            .logical_bytes
            .checked_add(self.request.values.logical_bytes)
            .and_then(|n| n.checked_add(std::mem::size_of::<Self>()))
            .and_then(|n| {
                self.needed
                    .len()
                    .checked_mul(std::mem::size_of::<usize>())
                    .and_then(|extra| n.checked_add(extra))
            })
            .ok_or(SettingsError::Limit)
    }
    pub(crate) fn matches(&self, captured: &SettingsRequestCapture) -> Result<bool, SettingsError> {
        if self.limits != captured.profile || self.request.changes.len() != captured.changes.len() {
            return Ok(false);
        }
        let mut m = Meter::new(SettingsLimitsV1 {
            max_work: self.limits.max_work,
            ..self.before.limits
        });
        for (a, b) in self.request.changes.iter().zip(&captured.changes) {
            m.tick()?;
            if self.before.layers[a.layer].identity != b.layer
                || a.expected != b.expected
                || self.before.schemas[a.key].key != b.key
            {
                return Ok(false);
            }
            match (&a.operation, &b.operation) {
                (Operation::Remove, capture::Operation::Remove)
                | (Operation::Reset, capture::Operation::Reset) => {}
                (Operation::Set(a), capture::Operation::Set(b)) => {
                    if !resolve::equal_between(
                        &self.request.values,
                        *a,
                        &captured.values,
                        *b,
                        &mut m,
                        true,
                    )? {
                        return Ok(false);
                    }
                }
                _ => return Ok(false),
            }
        }
        Ok(true)
    }
    pub(crate) fn prepare(
        before: Arc<CompiledSettingsV1>,
        changes: &[SettingChangeV1],
        limits: SettingsActivationLimitsV1,
    ) -> Result<Self, SettingsError> {
        let request = request(&before, changes, limits)?;
        Self::prepare_request(before, request, limits)
    }
    pub(crate) fn prepare_inputs(
        before: Arc<CompiledSettingsV1>,
        changes: &[SettingChangeInput<'_>],
        limits: SettingsActivationLimitsV1,
    ) -> Result<Self, SettingsError> {
        let request = request_inputs(&before, changes.iter().copied(), limits)?;
        Self::prepare_request(before, request, limits)
    }
    fn prepare_request(
        before: Arc<CompiledSettingsV1>,
        request: Request,
        limits: SettingsActivationLimitsV1,
    ) -> Result<Self, SettingsError> {
        let mut m = Meter::new(SettingsLimitsV1 {
            max_work: limits.max_work.min(before.limits.max_work),
            ..before.limits
        });
        let needed = footprint(&before, &request, &mut m)?;
        let bytes = before
            .logical_bytes
            .checked_add(request.values.logical_bytes)
            .and_then(|n| n.checked_add(std::mem::size_of::<Self>()))
            .and_then(|n| {
                needed
                    .len()
                    .checked_mul(std::mem::size_of::<usize>())
                    .and_then(|x| n.checked_add(x))
            })
            .ok_or(SettingsError::Limit)?;
        if bytes > limits.max_plan_bytes {
            return Err(SettingsError::Limit);
        }
        let (candidate, outcome) = decide(
            &before,
            RequestView {
                changes: &request.changes,
                values: &request.values,
            },
            limits,
        )?;
        let candidate_logical_bytes = candidate
            .as_ref()
            .map_or(before.logical_bytes, |c| c.logical_bytes);
        Ok(Self {
            before,
            request,
            needed,
            limits,
            would_change: outcome.changed,
            candidate_logical_bytes,
        })
    }
    pub(crate) fn decide(
        &self,
        current: &CompiledSettingsV1,
    ) -> Result<(Option<CompiledSettingsV1>, SettingsUpdateV1), SettingsError> {
        let a = self.before.basis;
        let b = current.basis;
        if (
            a.schema_generation,
            a.restriction_generation,
            a.association_generation,
        ) != (
            b.schema_generation,
            b.restriction_generation,
            b.association_generation,
        ) || self.before.limits != current.limits
        {
            return Err(SettingsError::Stale);
        }
        let mut m = Meter::new(SettingsLimitsV1 {
            max_work: self.limits.max_work.min(current.limits.max_work),
            ..current.limits
        });
        for layer in &self.needed {
            m.tick()?;
            let old = &self.before.layers[*layer];
            let mut actual = None;
            for row in &current.layers {
                m.tick()?;
                if row.identity == old.identity {
                    actual = Some(row);
                    break;
                }
            }
            if actual.is_none_or(|row| old.generation != row.generation) {
                return Err(SettingsError::Stale);
            }
        }
        let mut changes = Vec::new();
        m.edges::<Change>(self.request.changes.len())?;
        reserve(&mut changes, self.request.changes.len())?;
        for change in &self.request.changes {
            let mut position = None;
            for (i, row) in current.layers.iter().enumerate() {
                m.tick()?;
                if row.identity == self.before.layers[change.layer].identity {
                    position = Some(i);
                    break;
                }
            }
            changes.push(Change {
                layer: position.ok_or(SettingsError::Stale)?,
                ..*change
            });
        }
        decide(
            current,
            RequestView {
                changes: &changes,
                values: &self.request.values,
            },
            self.limits,
        )
    }
}
