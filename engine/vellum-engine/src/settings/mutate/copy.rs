use super::*;
pub(in crate::settings) fn node_from(
    source: &CompiledSettingsV1,
    root: usize,
    out: &mut CompiledSettingsV1,
    cache: &mut [Option<usize>],
    m: &mut Meter,
) -> Result<usize, SettingsError> {
    if let Some(root) = cache[root] {
        return Ok(root);
    }
    let copied = compile::node(out, m)?;
    cache[root] = Some(copied);
    let mut todo = Vec::new();
    reserve(&mut todo, 1)?;
    todo.push((root, copied));
    while let Some((old, new)) = todo.pop() {
        m.tick()?;
        let mut child = |at: usize,
                         out: &mut CompiledSettingsV1,
                         m: &mut Meter|
         -> Result<usize, SettingsError> {
            if let Some(at) = cache[at] {
                return Ok(at);
            }
            let next = compile::node(out, m)?;
            cache[at] = Some(next);
            reserve(&mut todo, 1)?;
            todo.push((at, next));
            Ok(next)
        };
        out.nodes[new] = match &source.nodes[old] {
            Node::Null => Node::Null,
            Node::Boolean(v) => Node::Boolean(*v),
            Node::Number(v) => Node::Number(*v),
            Node::Text(v) => Node::Text(m.string(v)?),
            Node::List(values) => {
                m.edges::<usize>(values.len())?;
                let mut rows = Vec::new();
                reserve(&mut rows, values.len())?;
                for v in values {
                    m.tick()?;
                    rows.push(child(*v, out, m)?);
                }
                Node::List(rows)
            }
            Node::Map(values) => {
                m.edges::<(String, usize)>(values.len())?;
                let mut rows = Vec::new();
                reserve(&mut rows, values.len())?;
                for (key, v) in values {
                    m.tick()?;
                    rows.push((m.identifier(key)?, child(*v, out, m)?));
                }
                Node::Map(rows)
            }
        };
    }
    Ok(copied)
}
pub(in crate::settings) fn cache(
    source: &CompiledSettingsV1,
    m: &mut Meter,
) -> Result<Vec<Option<usize>>, SettingsError> {
    m.steps(source.nodes.len())?;
    let mut cache = Vec::new();
    reserve(&mut cache, source.nodes.len())?;
    cache.resize(source.nodes.len(), None);
    Ok(cache)
}
pub(super) fn state(
    source: &CompiledSettingsV1,
    changes: &[Change],
    m: &mut Meter,
) -> Result<CompiledSettingsV1, SettingsError> {
    state_changes(source, changes, m)
}
pub(in crate::settings) fn unchanged(
    source: &CompiledSettingsV1,
    m: &mut Meter,
) -> Result<CompiledSettingsV1, SettingsError> {
    state_changes(source, &[], m)
}
pub(in crate::settings) fn declarations(
    source: &CompiledSettingsV1,
    m: &mut Meter,
) -> Result<(CompiledSettingsV1, Vec<Option<usize>>), SettingsError> {
    let mut out = blank(source, m)?;
    let mut nodes = cache(source, m)?;
    m.edges::<TypeNode>(source.types.len())?;
    reserve(&mut out.types, source.types.len())?;
    for ty in &source.types {
        m.tick()?;
        let copied = match ty {
            TypeNode::Boolean => TypeNode::Boolean,
            TypeNode::Text => TypeNode::Text,
            TypeNode::Integer(a, b) => TypeNode::Integer(*a, *b),
            TypeNode::List(v) => TypeNode::List(*v),
            TypeNode::Enum(values) => {
                m.edges::<usize>(values.len())?;
                let mut copied = Vec::new();
                reserve(&mut copied, values.len())?;
                for v in values {
                    copied.push(node_from(source, *v, &mut out, &mut nodes, m)?);
                }
                TypeNode::Enum(copied)
            }
            TypeNode::Map(fields) => {
                m.edges::<(String, usize)>(fields.len())?;
                let mut copied = Vec::new();
                reserve(&mut copied, fields.len())?;
                for (key, ty) in fields {
                    m.tick()?;
                    copied.push((m.identifier(key)?, *ty));
                }
                TypeNode::Map(copied)
            }
        };
        out.types.push(copied);
    }
    m.edges::<Schema>(source.schemas.len())?;
    reserve(&mut out.schemas, source.schemas.len())?;
    for schema in &source.schemas {
        m.tick()?;
        let key = m.identifier(&schema.key)?;
        let default = node_from(source, schema.default, &mut out, &mut nodes, m)?;
        m.edges::<SettingScopeV1>(schema.scopes.len())?;
        let mut scopes = Vec::new();
        reserve(&mut scopes, schema.scopes.len())?;
        scopes.extend_from_slice(&schema.scopes);
        out.schemas.push(Schema {
            key,
            value_type: schema.value_type,
            default,
            domain: schema.domain,
            scopes,
            composition: schema.composition,
            sensitive: schema.sensitive,
        });
    }
    m.edges::<(usize, Vec<usize>)>(source.restrictions.len())?;
    reserve(&mut out.restrictions, source.restrictions.len())?;
    for (key, values) in &source.restrictions {
        m.edges::<usize>(values.len())?;
        let mut copied = Vec::new();
        reserve(&mut copied, values.len())?;
        for v in values {
            m.tick()?;
            copied.push(node_from(source, *v, &mut out, &mut nodes, m)?);
        }
        out.restrictions.push((*key, copied));
    }
    Ok((out, nodes))
}
fn state_changes(
    source: &CompiledSettingsV1,
    changes: &[Change],
    m: &mut Meter,
) -> Result<CompiledSettingsV1, SettingsError> {
    let (mut out, mut nodes) = declarations(source, m)?;
    m.edges::<Layer>(source.layers.len())?;
    reserve(&mut out.layers, source.layers.len())?;
    for (layer_idx, layer) in source.layers.iter().enumerate() {
        m.tick()?;
        let identity = compile::identity(&layer.identity, &source.authority, m)?;
        let mut values = Vec::new();
        for (key, v) in &layer.values {
            let mut replaced = false;
            for change in changes {
                m.tick()?;
                replaced |= change.layer == layer_idx && change.key == *key;
            }
            if replaced {
                continue;
            }
            m.edges::<(usize, Contribution)>(1)?;
            reserve(&mut values, 1)?;
            values.push((
                *key,
                match v {
                    Contribution::Reset => Contribution::Reset,
                    Contribution::Set(v) => {
                        Contribution::Set(node_from(source, *v, &mut out, &mut nodes, m)?)
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
    m.edges::<Target>(source.targets.len())?;
    reserve(&mut out.targets, source.targets.len())?;
    for target in &source.targets {
        m.tick()?;
        let identity = compile::identity(&target.identity, &source.authority, m)?;
        m.edges::<usize>(target.layers.len())?;
        let mut layers = Vec::new();
        reserve(&mut layers, target.layers.len())?;
        layers.extend_from_slice(&target.layers);
        out.targets.push(Target {
            native_identity: target.native_identity,
            identity,
            layers,
            resolved: Vec::new(),
            generations: target.generations,
        });
    }
    Ok(out)
}
