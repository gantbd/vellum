use super::*;
fn next(v: u64) -> Result<u64, SettingsError> {
    v.checked_add(1)
        .filter(|n| *n <= MAX_SAFE as u64)
        .ok_or(SettingsError::VersionExhausted)
}
fn key(state: &CompiledSettingsV1, name: &str, m: &mut Meter) -> Result<usize, SettingsError> {
    for (i, s) in state.schemas.iter().enumerate() {
        m.tick()?;
        if s.key == name {
            return Ok(i);
        }
    }
    Err(SettingsError::UnknownKey)
}
fn validate_declarations(state: &CompiledSettingsV1, m: &mut Meter) -> Result<(), SettingsError> {
    for ty in &state.types {
        m.tick()?;
        match ty {
            TypeNode::Integer(a, b) if a > b || *a < -MAX_SAFE || *b > MAX_SAFE => {
                return Err(SettingsError::InvalidDefinition);
            }
            TypeNode::Enum(v) => {
                if v.is_empty() {
                    return Err(SettingsError::InvalidDefinition);
                }
                for (i, x) in v.iter().enumerate() {
                    for y in &v[..i] {
                        if resolve::equal(state, *x, *y, m, false)? {
                            return Err(SettingsError::Duplicate);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    for (i, s) in state.schemas.iter().enumerate() {
        m.tick()?;
        for other in &state.schemas[..i] {
            m.tick()?;
            if other.key == s.key {
                return Err(SettingsError::Duplicate);
            }
        }
        if s.scopes.is_empty() {
            return Err(SettingsError::IllegalScope);
        }
        for (i, scope) in s.scopes.iter().enumerate() {
            m.tick()?;
            if s.scopes[..i].contains(scope) {
                return Err(SettingsError::Duplicate);
            }
        }
        if s.composition == SettingCompositionV1::MergeMap
            && !matches!(state.types[s.value_type], TypeNode::Map(_))
        {
            return Err(SettingsError::InvalidDefinition);
        }
        resolve::typed(state, s.value_type, s.default, false, m)?;
    }
    Ok(())
}
fn candidate(
    before: &CompiledSettingsV1,
    request: &DefinitionCapture,
    m: &mut Meter,
) -> Result<CompiledSettingsV1, SettingsError> {
    let mut out = copy::unchanged(&request.data, m)?;
    out.basis = before.basis;
    out.next_target_identity = before.next_target_identity;
    out.layer_incarnation = before.layer_incarnation;
    validate_declarations(&out, m)?;
    let mut incoming = copy::cache(&request.data, m)?;
    m.edges::<(usize, Vec<usize>)>(request.restrictions.len())?;
    reserve(&mut out.restrictions, request.restrictions.len())?;
    for (name, allowed) in &request.restrictions {
        m.tick()?;
        let k = key(&out, name, m)?;
        if allowed.is_empty() {
            return Err(SettingsError::InvalidDefinition);
        }
        m.edges::<usize>(allowed.len())?;
        let mut copied = Vec::new();
        reserve(&mut copied, allowed.len())?;
        for value in allowed {
            let id = copy::node_from(&request.data, *value, &mut out, &mut incoming, m)?;
            resolve::typed(&out, out.schemas[k].value_type, id, false, m)?;
            copied.push(id);
        }
        out.restrictions.push((k, copied));
    }
    let mut old_nodes = copy::cache(before, m)?;
    m.edges::<Layer>(before.layers.len())?;
    reserve(&mut out.layers, before.layers.len())?;
    for layer in &before.layers {
        m.tick()?;
        let identity = compile::identity(&layer.identity, &before.authority, m)?;
        m.edges::<(usize, Contribution)>(layer.values.len())?;
        let mut values = Vec::new();
        reserve(&mut values, layer.values.len())?;
        for (old, c) in &layer.values {
            let k = key(&out, &before.schemas[*old].key, m)?;
            if !out.schemas[k].scopes.contains(&layer.scope) {
                return Err(SettingsError::IllegalScope);
            }
            let c = match c {
                Contribution::Reset => Contribution::Reset,
                Contribution::Set(v) => {
                    let v = copy::node_from(before, *v, &mut out, &mut old_nodes, m)?;
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
            scope: layer.scope,
            generation: layer.generation,
            values,
        });
    }
    install_targets(&mut out, &request.targets, m)?;
    resolve::resolve_all(&mut out, m)?;
    Ok(out)
}
pub(in crate::settings) fn install_targets(
    out: &mut CompiledSettingsV1,
    targets: &[TargetInput],
    m: &mut Meter,
) -> Result<(), SettingsError> {
    m.edges::<Target>(targets.len())?;
    reserve(&mut out.targets, targets.len())?;
    for target in targets {
        m.tick()?;
        if target.identity.authority != out.authority {
            return Err(SettingsError::InvalidIdentity);
        }
        for old in &out.targets {
            m.tick()?;
            if old.identity == target.identity {
                return Err(SettingsError::Duplicate);
            }
        }
        let identity = compile::identity(&target.identity, &out.authority, m)?;
        let mut layers = Vec::new();
        m.edges::<usize>(target.layers.len())?;
        reserve(&mut layers, target.layers.len())?;
        for id in &target.layers {
            m.tick()?;
            if id.authority != out.authority {
                return Err(SettingsError::InvalidIdentity);
            }
            let mut found = None;
            for (i, layer) in out.layers.iter().enumerate() {
                m.tick()?;
                if layer.identity == *id {
                    found = Some(i);
                    break;
                }
            }
            let i = found.ok_or(SettingsError::UnknownLayer)?;
            m.steps(layers.len())?;
            if layers.contains(&i) {
                return Err(SettingsError::Duplicate);
            }
            layers.push(i);
        }
        out.targets.push(Target {
            native_identity: 0,
            generations: [1; 3],
            identity,
            layers,
            resolved: Vec::new(),
        });
    }
    Ok(())
}
pub(super) fn decide(
    before: &CompiledSettingsV1,
    request: &DefinitionCapture,
    m: &mut Meter,
) -> Result<(Option<CompiledSettingsV1>, SettingsDefinitionOutcomeV1), SettingsError> {
    let schema_changed = !compare::schemas(before, &request.data, m, false)?;
    let restrictions_changed = !compare::restrictions(before, request, m)?;
    let associations_changed = !compare::targets(before, request, m)?;
    let changed = schema_changed || restrictions_changed || associations_changed;
    let mut outcome = SettingsDefinitionOutcomeV1 {
        changed,
        basis: before.basis,
        schema_changed,
        restrictions_changed,
        associations_changed,
        changed_domains: 0,
        added_targets: 0,
        removed_targets: 0,
    };
    if !changed {
        return Ok((None, outcome));
    }
    let mut after = candidate(before, request, m)?;
    after.basis.configuration_generation = next(before.basis.configuration_generation)?;
    if schema_changed {
        after.basis.schema_generation = next(before.basis.schema_generation)?;
    }
    if restrictions_changed {
        after.basis.restriction_generation = next(before.basis.restriction_generation)?;
    }
    if associations_changed {
        after.basis.association_generation = next(before.basis.association_generation)?;
    }
    reconcile_targets(before, &mut after, &mut outcome, m)?;
    after.logical_bytes = m.used_bytes();
    outcome.basis = after.basis;
    Ok((Some(after), outcome))
}
pub(in crate::settings) fn reconcile_targets(
    before: &CompiledSettingsV1,
    after: &mut CompiledSettingsV1,
    outcome: &mut SettingsDefinitionOutcomeV1,
    m: &mut Meter,
) -> Result<(), SettingsError> {
    for index in 0..after.targets.len() {
        let mut old = None;
        for (j, target) in before.targets.iter().enumerate() {
            m.tick()?;
            if target.identity == after.targets[index].identity {
                old = Some(j);
                break;
            }
        }
        if let Some(old) = old {
            after.targets[index].native_identity = before.targets[old].native_identity;
            after.targets[index].generations = before.targets[old].generations;
            for (domain, d) in [
                SettingDomainV1::Source,
                SettingDomainV1::Serialization,
                SettingDomainV1::Analysis,
            ]
            .into_iter()
            .enumerate()
            {
                if compare::domain_changed(before, old, after, index, d, m)? {
                    after.targets[index].generations[domain] =
                        next(before.targets[old].generations[domain])?;
                    outcome.changed_domains = outcome
                        .changed_domains
                        .checked_add(1)
                        .ok_or(SettingsError::Limit)?;
                }
            }
        } else {
            let id = after.issue_target_identity()?;
            after.targets[index].native_identity = id;
            outcome.added_targets = outcome
                .added_targets
                .checked_add(1)
                .ok_or(SettingsError::Limit)?;
        }
    }
    for old in &before.targets {
        m.tick()?;
        let mut present = false;
        for new in &after.targets {
            m.tick()?;
            if old.identity == new.identity {
                present = true;
                break;
            }
        }
        if !present {
            outcome.removed_targets = outcome
                .removed_targets
                .checked_add(1)
                .ok_or(SettingsError::Limit)?;
        }
    }
    Ok(())
}
