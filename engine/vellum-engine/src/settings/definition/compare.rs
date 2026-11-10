use super::*;
fn types(
    a: &CompiledSettingsV1,
    at: usize,
    b: &CompiledSettingsV1,
    bt: usize,
    m: &mut Meter,
    exact: bool,
) -> Result<bool, SettingsError> {
    let mut todo = Vec::new();
    reserve(&mut todo, 1)?;
    todo.push((at, bt));
    while let Some((x, y)) = todo.pop() {
        m.tick()?;
        match (&a.types[x], &b.types[y]) {
            (TypeNode::Boolean, TypeNode::Boolean) | (TypeNode::Text, TypeNode::Text) => {}
            (TypeNode::Integer(a, b), TypeNode::Integer(c, d)) if a == c && b == d => {}
            (TypeNode::List(x), TypeNode::List(y)) => {
                reserve(&mut todo, 1)?;
                todo.push((*x, *y));
            }
            (TypeNode::Enum(x), TypeNode::Enum(y)) if x.len() == y.len() => {
                for (x, y) in x.iter().zip(y) {
                    if !resolve::equal_between(a, *x, b, *y, m, exact)? {
                        return Ok(false);
                    }
                }
            }
            (TypeNode::Map(x), TypeNode::Map(y)) if x.len() == y.len() => {
                reserve(&mut todo, x.len())?;
                for (key, child) in x {
                    let mut other = None;
                    for (name, ty) in y {
                        m.tick()?;
                        if name == key {
                            other = Some(*ty);
                            break;
                        }
                    }
                    let Some(other) = other else { return Ok(false) };
                    todo.push((*child, other));
                }
            }
            _ => return Ok(false),
        }
    }
    Ok(true)
}
pub(super) fn schemas(
    a: &CompiledSettingsV1,
    b: &CompiledSettingsV1,
    m: &mut Meter,
    exact: bool,
) -> Result<bool, SettingsError> {
    if a.schemas.len() != b.schemas.len() {
        return Ok(false);
    }
    for (x, y) in a.schemas.iter().zip(&b.schemas) {
        m.tick()?;
        if x.key != y.key
            || x.domain != y.domain
            || x.scopes != y.scopes
            || x.composition != y.composition
            || x.sensitive != y.sensitive
            || !types(a, x.value_type, b, y.value_type, m, exact)?
            || !resolve::equal_between(a, x.default, b, y.default, m, exact)?
        {
            return Ok(false);
        }
    }
    Ok(true)
}
fn values(
    a: &CompiledSettingsV1,
    aa: &[usize],
    b: &CompiledSettingsV1,
    bb: &[usize],
    m: &mut Meter,
    exact: bool,
) -> Result<bool, SettingsError> {
    if aa.len() != bb.len() {
        return Ok(false);
    }
    for (x, y) in aa.iter().zip(bb) {
        if !resolve::equal_between(a, *x, b, *y, m, exact)? {
            return Ok(false);
        }
    }
    Ok(true)
}
pub(super) fn raw_restrictions(
    a: &DefinitionCapture,
    b: &DefinitionCapture,
    m: &mut Meter,
    exact: bool,
) -> Result<bool, SettingsError> {
    if a.restrictions.len() != b.restrictions.len() {
        return Ok(false);
    }
    for (key, v) in &a.restrictions {
        let mut found = None;
        for (name, other) in &b.restrictions {
            m.tick()?;
            if key == name {
                found = Some(other);
                break;
            }
        }
        let Some(other) = found else { return Ok(false) };
        if !values(&a.data, v, &b.data, other, m, exact)? {
            return Ok(false);
        }
    }
    Ok(true)
}
pub(super) fn raw_targets(
    a: &DefinitionCapture,
    b: &DefinitionCapture,
    m: &mut Meter,
) -> Result<bool, SettingsError> {
    if a.targets.len() != b.targets.len() {
        return Ok(false);
    }
    for (x, y) in a.targets.iter().zip(&b.targets) {
        m.tick()?;
        m.steps(x.layers.len())?;
        if x.identity != y.identity || x.layers != y.layers {
            return Ok(false);
        }
    }
    Ok(true)
}
pub(super) fn restrictions(
    before: &CompiledSettingsV1,
    request: &DefinitionCapture,
    m: &mut Meter,
) -> Result<bool, SettingsError> {
    if before.restrictions.len() != request.restrictions.len() {
        return Ok(false);
    }
    for (key, v) in &before.restrictions {
        let mut found = None;
        for (name, other) in &request.restrictions {
            m.tick()?;
            if name == &before.schemas[*key].key {
                found = Some(other);
                break;
            }
        }
        let Some(other) = found else { return Ok(false) };
        if !values(before, v, &request.data, other, m, false)? {
            return Ok(false);
        }
    }
    Ok(true)
}
pub(super) fn targets(
    before: &CompiledSettingsV1,
    request: &DefinitionCapture,
    m: &mut Meter,
) -> Result<bool, SettingsError> {
    target_inputs(before, &request.targets, m)
}
pub(in crate::settings) fn target_inputs(
    before: &CompiledSettingsV1,
    inputs: &[TargetInput],
    m: &mut Meter,
) -> Result<bool, SettingsError> {
    if before.targets.len() != inputs.len() {
        return Ok(false);
    }
    for (x, y) in before.targets.iter().zip(inputs) {
        m.tick()?;
        if x.identity != y.identity || x.layers.len() != y.layers.len() {
            return Ok(false);
        }
        for (index, id) in x.layers.iter().zip(&y.layers) {
            m.tick()?;
            if before.layers[*index].identity != *id {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
pub(super) fn domain_changed(
    before: &CompiledSettingsV1,
    old: usize,
    after: &CompiledSettingsV1,
    new: usize,
    domain: SettingDomainV1,
    m: &mut Meter,
) -> Result<bool, SettingsError> {
    for (i, schema) in before.schemas.iter().enumerate() {
        m.tick()?;
        if schema.domain != domain {
            continue;
        }
        let mut found = None;
        for (j, other) in after.schemas.iter().enumerate() {
            m.tick()?;
            if other.domain == domain && other.key == schema.key {
                found = Some(j);
                break;
            }
        }
        let Some(j) = found else { return Ok(true) };
        if !resolve::equal_between(
            before,
            before.targets[old].resolved[i].value,
            after,
            after.targets[new].resolved[j].value,
            m,
            false,
        )? {
            return Ok(true);
        }
    }
    for schema in &after.schemas {
        m.tick()?;
        if schema.domain != domain {
            continue;
        }
        let mut found = false;
        for old in &before.schemas {
            m.tick()?;
            if old.domain == domain && old.key == schema.key {
                found = true;
                break;
            }
        }
        if !found {
            return Ok(true);
        }
    }
    Ok(false)
}
