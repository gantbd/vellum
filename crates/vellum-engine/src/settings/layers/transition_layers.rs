use super::*;
fn next(v: u64) -> Result<u64, SettingsError> {
    v.checked_add(1)
        .filter(|n| *n <= MAX_SAFE as u64)
        .ok_or(SettingsError::VersionExhausted)
}
fn key(s: &CompiledSettingsV1, key: &str, m: &mut Meter) -> Result<usize, SettingsError> {
    for (i, row) in s.schemas.iter().enumerate() {
        m.tick()?;
        if row.key == key {
            return Ok(i);
        }
    }
    Err(SettingsError::UnknownKey)
}
fn removed(
    input: &LayerCapture,
    id: &SettingIdentityV1,
    m: &mut Meter,
) -> Result<bool, SettingsError> {
    for row in &input.remove {
        m.tick()?;
        if row.identity == *id {
            return Ok(true);
        }
    }
    Ok(false)
}
fn validate(
    before: &CompiledSettingsV1,
    input: &LayerCapture,
    m: &mut Meter,
) -> Result<u64, SettingsError> {
    let mut high = before.layer_incarnation;
    for row in &input.create {
        m.tick()?;
        high = next(high)?;
        if row.identity.authority != before.authority || row.identity.incarnation != high {
            return Err(SettingsError::InvalidIdentity);
        }
    }
    for (i, row) in input.remove.iter().enumerate() {
        m.tick()?;
        if row.identity.authority != before.authority
            || row.expected_generation == 0
            || row.expected_generation > MAX_SAFE as u64
        {
            return Err(SettingsError::InvalidIdentity);
        }
        for old in &input.remove[..i] {
            m.tick()?;
            if old.identity == row.identity {
                return Err(SettingsError::Duplicate);
            }
        }
        let mut found = None;
        for layer in &before.layers {
            m.tick()?;
            if layer.identity == row.identity {
                found = Some(layer);
                break;
            }
        }
        if found.ok_or(SettingsError::UnknownLayer)?.generation != row.expected_generation {
            return Err(SettingsError::Stale);
        }
    }
    if before
        .layers
        .len()
        .checked_sub(input.remove.len())
        .and_then(|n| n.checked_add(input.create.len()))
        .is_none_or(|n| n > before.limits.max_layers)
    {
        return Err(SettingsError::Limit);
    }
    Ok(high)
}
pub(super) fn decide(
    before: &CompiledSettingsV1,
    input: &LayerCapture,
    m: &mut Meter,
) -> Result<(Option<CompiledSettingsV1>, SettingsLayerOutcomeV1), SettingsError> {
    let high = validate(before, input, m)?;
    let associations_changed = !compare::target_inputs(before, &input.targets, m)?;
    let changed = associations_changed || !input.create.is_empty() || !input.remove.is_empty();
    let mut result = SettingsLayerOutcomeV1 {
        changed,
        basis: before.basis,
        layer_incarnation: high,
        created_layers: input.create.len(),
        removed_layers: input.remove.len(),
        associations_changed,
        changed_domains: 0,
        added_targets: 0,
        removed_targets: 0,
    };
    if !changed {
        return Ok((None, result));
    }
    let (mut out, mut old_nodes) = copy::declarations(before, m)?;
    let count = before.layers.len() - input.remove.len() + input.create.len();
    m.edges::<Layer>(count)?;
    reserve(&mut out.layers, count)?;
    for layer in &before.layers {
        m.tick()?;
        if removed(input, &layer.identity, m)? {
            continue;
        }
        let identity = compile::identity(&layer.identity, &before.authority, m)?;
        m.edges::<(usize, Contribution)>(layer.values.len())?;
        let mut values = Vec::new();
        reserve(&mut values, layer.values.len())?;
        for (key, value) in &layer.values {
            m.tick()?;
            values.push((
                *key,
                match value {
                    Contribution::Reset => Contribution::Reset,
                    Contribution::Set(v) => {
                        Contribution::Set(copy::node_from(before, *v, &mut out, &mut old_nodes, m)?)
                    }
                },
            ));
        }
        out.layers.push(Layer {
            identity,
            scope: layer.scope,
            generation: layer.generation,
            values,
        });
    }
    let mut new_nodes = copy::cache(&input.data, m)?;
    for row in &input.create {
        m.tick()?;
        let identity = compile::identity(&row.identity, &before.authority, m)?;
        m.edges::<(usize, Contribution)>(row.values.len())?;
        let mut values = Vec::new();
        reserve(&mut values, row.values.len())?;
        for (name, c) in &row.values {
            m.tick()?;
            let k = key(&out, name, m)?;
            if !out.schemas[k].scopes.contains(&row.scope) {
                return Err(SettingsError::IllegalScope);
            }
            let c = match c {
                Contribution::Reset => Contribution::Reset,
                Contribution::Set(v) => {
                    let v = copy::node_from(&input.data, *v, &mut out, &mut new_nodes, m)?;
                    resolve::typed(
                        &out,
                        out.schemas[k].value_type,
                        v,
                        out.schemas[k].composition == SettingCompositionV1::MergeMap,
                        m,
                    )?;
                    Contribution::Set(v)
                }
            };
            values.push((k, c));
        }
        out.layers.push(Layer {
            identity,
            scope: row.scope,
            generation: 1,
            values,
        });
    }
    transition::install_targets(&mut out, &input.targets, m)?;
    resolve::resolve_all(&mut out, m)?;
    out.layer_incarnation = high;
    out.basis.configuration_generation = next(before.basis.configuration_generation)?;
    if associations_changed {
        out.basis.association_generation = next(before.basis.association_generation)?;
    }
    let mut domain = SettingsDefinitionOutcomeV1 {
        changed: true,
        basis: out.basis,
        schema_changed: false,
        restrictions_changed: false,
        associations_changed,
        changed_domains: 0,
        added_targets: 0,
        removed_targets: 0,
    };
    transition::reconcile_targets(before, &mut out, &mut domain, m)?;
    result.basis = out.basis;
    result.changed_domains = domain.changed_domains;
    result.added_targets = domain.added_targets;
    result.removed_targets = domain.removed_targets;
    out.logical_bytes = m.used_bytes();
    Ok((Some(out), result))
}
