use super::*;
pub(super) const MAX_SAFE: i64 = 9_007_199_254_740_991;
pub(super) struct Meter {
    pub(super) limits: SettingsLimitsV1,
    work: usize,
    bytes: usize,
}
impl Meter {
    pub(super) fn new(limits: SettingsLimitsV1) -> Self {
        Self {
            limits,
            work: limits.max_work,
            bytes: 0,
        }
    }
    pub(super) fn used_bytes(&self) -> usize {
        self.bytes
    }
    pub(super) fn tick(&mut self) -> Result<(), SettingsError> {
        self.steps(1)
    }
    pub(super) fn steps(&mut self, n: usize) -> Result<(), SettingsError> {
        self.work = self.work.checked_sub(n).ok_or(SettingsError::Limit)?;
        Ok(())
    }
    pub(super) fn bytes(&mut self, n: usize) -> Result<(), SettingsError> {
        self.bytes = self
            .bytes
            .checked_add(n)
            .filter(|n| *n <= self.limits.max_state_bytes)
            .ok_or(SettingsError::Limit)?;
        Ok(())
    }
    pub(super) fn edges<T>(&mut self, n: usize) -> Result<(), SettingsError> {
        self.bytes(
            n.checked_mul(std::mem::size_of::<T>())
                .ok_or(SettingsError::Limit)?,
        )
    }
    fn label(&self, s: &str) -> Result<(), SettingsError> {
        if s.is_empty() || s.len() > self.limits.max_identifier_bytes {
            Err(SettingsError::InvalidIdentity)
        } else {
            Ok(())
        }
    }
    pub(super) fn string(&mut self, s: &str) -> Result<String, SettingsError> {
        self.bytes(s.len())?;
        let mut out = String::new();
        out.try_reserve_exact(s.len())
            .map_err(|_| SettingsError::Capacity)?;
        out.push_str(s);
        Ok(out)
    }
    pub(super) fn identifier(&mut self, s: &str) -> Result<String, SettingsError> {
        self.label(s)?;
        self.string(s)
    }
}
pub(super) fn reserve<T>(rows: &mut Vec<T>, n: usize) -> Result<(), SettingsError> {
    rows.try_reserve_exact(n)
        .map_err(|_| SettingsError::Capacity)
}
fn identity_valid(id: &SettingIdentityV1, authority: &str, m: &Meter) -> Result<(), SettingsError> {
    m.label(&id.authority)?;
    m.label(&id.id)?;
    if id.authority != authority || id.incarnation == 0 || id.incarnation > MAX_SAFE as u64 {
        return Err(SettingsError::InvalidIdentity);
    }
    Ok(())
}
pub(super) fn identity(
    id: &SettingIdentityV1,
    authority: &str,
    m: &mut Meter,
) -> Result<SettingIdentityV1, SettingsError> {
    identity_valid(id, authority, m)?;
    Ok(SettingIdentityV1 {
        authority: m.identifier(&id.authority)?,
        id: m.identifier(&id.id)?,
        incarnation: id.incarnation,
    })
}
pub(super) fn node(state: &mut CompiledSettingsV1, m: &mut Meter) -> Result<usize, SettingsError> {
    if state.nodes.len() >= m.limits.max_value_nodes {
        return Err(SettingsError::Limit);
    }
    m.bytes(std::mem::size_of::<Node>())?;
    reserve(&mut state.nodes, 1)?;
    let at = state.nodes.len();
    state.nodes.push(Node::Null);
    Ok(at)
}
fn child_count(current: usize, extra: usize, limit: usize) -> Result<(), SettingsError> {
    if current.checked_add(extra).is_none_or(|n| n > limit) {
        Err(SettingsError::Limit)
    } else {
        Ok(())
    }
}
pub(super) fn value(
    state: &mut CompiledSettingsV1,
    input: &SettingValueV1,
    m: &mut Meter,
) -> Result<usize, SettingsError> {
    let root = node(state, m)?;
    let mut stack = Vec::new();
    reserve(&mut stack, 1)?;
    stack.push((input, root, 0usize));
    while let Some((input, at, depth)) = stack.pop() {
        m.tick()?;
        if depth > m.limits.max_value_depth {
            return Err(SettingsError::Limit);
        }
        state.nodes[at] = match input {
            SettingValueV1::Null => Node::Null,
            SettingValueV1::Boolean(v) => Node::Boolean(*v),
            SettingValueV1::Number(v) => {
                if !v.is_finite() {
                    return Err(SettingsError::InvalidValue);
                }
                Node::Number(v.to_bits())
            }
            SettingValueV1::Text(v) => Node::Text(m.string(v)?),
            SettingValueV1::List(values) => {
                child_count(state.nodes.len(), values.len(), m.limits.max_value_nodes)?;
                m.edges::<usize>(values.len())?;
                let mut children = Vec::new();
                reserve(&mut children, values.len())?;
                reserve(&mut stack, values.len())?;
                for input in values {
                    m.tick()?;
                    let child = node(state, m)?;
                    children.push(child);
                    stack.push((
                        input,
                        child,
                        depth.checked_add(1).ok_or(SettingsError::Limit)?,
                    ));
                }
                Node::List(children)
            }
            SettingValueV1::Map(values) => {
                child_count(state.nodes.len(), values.len(), m.limits.max_value_nodes)?;
                m.edges::<(String, usize)>(values.len())?;
                let mut children: Vec<(String, usize)> = Vec::new();
                reserve(&mut children, values.len())?;
                reserve(&mut stack, values.len())?;
                for (key, input) in values {
                    m.tick()?;
                    for (old, _) in &children {
                        m.tick()?;
                        if old == key {
                            return Err(SettingsError::Duplicate);
                        }
                    }
                    let key = m.identifier(key)?;
                    let child = node(state, m)?;
                    children.push((key, child));
                    stack.push((
                        input,
                        child,
                        depth.checked_add(1).ok_or(SettingsError::Limit)?,
                    ));
                }
                Node::Map(children)
            }
        };
    }
    Ok(root)
}
fn type_node(state: &mut CompiledSettingsV1, m: &mut Meter) -> Result<usize, SettingsError> {
    if state.types.len() >= m.limits.max_schema_nodes {
        return Err(SettingsError::Limit);
    }
    m.bytes(std::mem::size_of::<TypeNode>())?;
    reserve(&mut state.types, 1)?;
    let at = state.types.len();
    state.types.push(TypeNode::Boolean);
    Ok(at)
}
fn value_type(
    state: &mut CompiledSettingsV1,
    input: &SettingTypeV1,
    m: &mut Meter,
) -> Result<usize, SettingsError> {
    copy_type(state, input, m, true)
}
pub(super) fn copy_type(
    state: &mut CompiledSettingsV1,
    input: &SettingTypeV1,
    m: &mut Meter,
    checked: bool,
) -> Result<usize, SettingsError> {
    let root = type_node(state, m)?;
    let mut stack = Vec::new();
    reserve(&mut stack, 1)?;
    stack.push((input, root, 0usize));
    while let Some((input, at, depth)) = stack.pop() {
        m.tick()?;
        if depth > m.limits.max_schema_depth {
            return Err(SettingsError::Limit);
        }
        state.types[at] = match input {
            SettingTypeV1::Boolean => TypeNode::Boolean,
            SettingTypeV1::Text => TypeNode::Text,
            SettingTypeV1::Integer { min, max } => {
                if checked && (min > max || *min < -MAX_SAFE || *max > MAX_SAFE) {
                    return Err(SettingsError::InvalidDefinition);
                }
                TypeNode::Integer(*min, *max)
            }
            SettingTypeV1::Enum(values) => {
                if checked && values.is_empty() {
                    return Err(SettingsError::InvalidDefinition);
                }
                child_count(state.nodes.len(), values.len(), m.limits.max_value_nodes)?;
                m.edges::<usize>(values.len())?;
                let mut options = Vec::new();
                reserve(&mut options, values.len())?;
                for input in values {
                    let v = value(state, input, m)?;
                    if checked {
                        for old in &options {
                            if super::resolve::equal(state, *old, v, m, false)? {
                                return Err(SettingsError::Duplicate);
                            }
                        }
                    }
                    options.push(v);
                }
                TypeNode::Enum(options)
            }
            SettingTypeV1::List(input) => {
                let child = type_node(state, m)?;
                reserve(&mut stack, 1)?;
                stack.push((
                    input.as_ref(),
                    child,
                    depth.checked_add(1).ok_or(SettingsError::Limit)?,
                ));
                TypeNode::List(child)
            }
            SettingTypeV1::Map(fields) => {
                child_count(state.types.len(), fields.len(), m.limits.max_schema_nodes)?;
                m.edges::<(String, usize)>(fields.len())?;
                let mut children: Vec<(String, usize)> = Vec::new();
                reserve(&mut children, fields.len())?;
                reserve(&mut stack, fields.len())?;
                for (key, input) in fields {
                    for (old, _) in &children {
                        m.tick()?;
                        if old == key {
                            return Err(SettingsError::Duplicate);
                        }
                    }
                    let key = m.identifier(key)?;
                    let child = type_node(state, m)?;
                    children.push((key, child));
                    stack.push((
                        input,
                        child,
                        depth.checked_add(1).ok_or(SettingsError::Limit)?,
                    ));
                }
                TypeNode::Map(children)
            }
        };
    }
    Ok(root)
}
fn schema_key(
    state: &CompiledSettingsV1,
    key: &str,
    m: &mut Meter,
) -> Result<usize, SettingsError> {
    for (i, schema) in state.schemas.iter().enumerate() {
        m.tick()?;
        if schema.key == key {
            return Ok(i);
        }
    }
    Err(SettingsError::UnknownKey)
}
pub(super) fn compile(
    d: &SettingsDefinitionV1,
    authority: &str,
    limits: SettingsLimitsV1,
) -> Result<CompiledSettingsV1, SettingsError> {
    let mut m = Meter {
        limits,
        work: limits.max_work,
        bytes: 0,
    };
    m.tick()?;
    m.bytes(std::mem::size_of::<CompiledSettingsV1>())?;
    if d.schemas.len() > limits.max_keys
        || d.restrictions.len() > limits.max_keys
        || d.layers.len() > limits.max_layers
        || d.targets.len() > limits.max_targets
    {
        return Err(SettingsError::Limit);
    }
    let mut state = CompiledSettingsV1 {
        layer_incarnation: 0,
        next_target_identity: 0,
        basis: SettingsBasisV1::initial(),
        authority: m.identifier(authority)?,
        limits,
        nodes: Vec::new(),
        types: Vec::new(),
        schemas: Vec::new(),
        restrictions: Vec::new(),
        layers: Vec::new(),
        targets: Vec::new(),
        logical_bytes: 0,
    };
    m.edges::<Schema>(d.schemas.len())?;
    reserve(&mut state.schemas, d.schemas.len())?;
    for input in &d.schemas {
        m.tick()?;
        for old in &state.schemas {
            m.tick()?;
            if old.key == input.key {
                return Err(SettingsError::Duplicate);
            }
        }
        let key = m.identifier(&input.key)?;
        if input.scopes.is_empty() || input.scopes.len() > 5 {
            return Err(SettingsError::IllegalScope);
        }
        for (i, scope) in input.scopes.iter().enumerate() {
            m.tick()?;
            if input.scopes[..i].contains(scope) {
                return Err(SettingsError::Duplicate);
            }
        }
        let value_type = value_type(&mut state, &input.value_type, &mut m)?;
        let default = value(&mut state, &input.default, &mut m)?;
        if input.composition == SettingCompositionV1::MergeMap
            && !matches!(state.types[value_type], TypeNode::Map(_))
        {
            return Err(SettingsError::InvalidDefinition);
        }
        super::resolve::typed(&state, value_type, default, false, &mut m)?;
        m.edges::<SettingScopeV1>(input.scopes.len())?;
        let mut scopes = Vec::new();
        reserve(&mut scopes, input.scopes.len())?;
        scopes.extend_from_slice(&input.scopes);
        state.schemas.push(Schema {
            key,
            value_type,
            default,
            domain: input.domain,
            scopes,
            composition: input.composition,
            sensitive: input.sensitive,
        });
    }
    m.edges::<(usize, Vec<usize>)>(d.restrictions.len())?;
    reserve(&mut state.restrictions, d.restrictions.len())?;
    for (key, inputs) in &d.restrictions {
        let schema = schema_key(&state, key, &mut m)?;
        for (old, _) in &state.restrictions {
            m.tick()?;
            if *old == schema {
                return Err(SettingsError::Duplicate);
            }
        }
        if inputs.is_empty() {
            return Err(SettingsError::InvalidDefinition);
        }
        child_count(state.nodes.len(), inputs.len(), limits.max_value_nodes)?;
        m.edges::<usize>(inputs.len())?;
        let mut values = Vec::new();
        reserve(&mut values, inputs.len())?;
        for input in inputs {
            let v = value(&mut state, input, &mut m)?;
            super::resolve::typed(&state, state.schemas[schema].value_type, v, false, &mut m)?;
            values.push(v);
        }
        state.restrictions.push((schema, values));
    }
    m.edges::<Layer>(d.layers.len())?;
    reserve(&mut state.layers, d.layers.len())?;
    for input in &d.layers {
        m.tick()?;
        for old in &state.layers {
            m.tick()?;
            if old.identity == input.identity {
                return Err(SettingsError::Duplicate);
            }
        }
        if input.generation == 0 || input.generation > MAX_SAFE as u64 {
            return Err(SettingsError::InvalidIdentity);
        }
        if input.values.len() > limits.max_keys {
            return Err(SettingsError::Limit);
        }
        let identity = identity(&input.identity, authority, &mut m)?;
        state.layer_incarnation = state.layer_incarnation.max(identity.incarnation);
        m.edges::<(usize, Contribution)>(input.values.len())?;
        let mut values = Vec::new();
        reserve(&mut values, input.values.len())?;
        for (key, input_value) in &input.values {
            let schema = schema_key(&state, key, &mut m)?;
            for (old, _) in &values {
                m.tick()?;
                if *old == schema {
                    return Err(SettingsError::Duplicate);
                }
            }
            if !state.schemas[schema].scopes.contains(&input.scope) {
                return Err(SettingsError::IllegalScope);
            }
            let contribution = match input_value {
                SettingContributionV1::Reset => Contribution::Reset,
                SettingContributionV1::Set(input) => {
                    let node = value(&mut state, input, &mut m)?;
                    super::resolve::typed(
                        &state,
                        state.schemas[schema].value_type,
                        node,
                        state.schemas[schema].composition == SettingCompositionV1::MergeMap,
                        &mut m,
                    )?;
                    Contribution::Set(node)
                }
            };
            values.push((schema, contribution));
        }
        state.layers.push(Layer {
            identity,
            scope: input.scope,
            generation: input.generation,
            values,
        });
    }
    m.edges::<Target>(d.targets.len())?;
    reserve(&mut state.targets, d.targets.len())?;
    for input in &d.targets {
        m.tick()?;
        for old in &state.targets {
            m.tick()?;
            if old.identity == input.identity {
                return Err(SettingsError::Duplicate);
            }
        }
        let identity = identity(&input.identity, authority, &mut m)?;
        if input.layers.len() > limits.max_layers {
            return Err(SettingsError::Limit);
        }
        m.edges::<usize>(input.layers.len())?;
        let mut layers = Vec::new();
        reserve(&mut layers, input.layers.len())?;
        for layer in &input.layers {
            identity_valid(layer, authority, &m)?;
            let mut found = None;
            for (i, old) in state.layers.iter().enumerate() {
                m.tick()?;
                if old.identity == *layer {
                    found = Some(i);
                    break;
                }
            }
            let layer = found.ok_or(SettingsError::UnknownLayer)?;
            for old in &layers {
                m.tick()?;
                if *old == layer {
                    return Err(SettingsError::Duplicate);
                }
            }
            layers.push(layer);
        }
        let native_identity = state.issue_target_identity()?;
        state.targets.push(Target {
            native_identity,
            generations: [1; 3],
            identity,
            layers,
            resolved: Vec::new(),
        });
    }
    super::resolve::resolve_all(&mut state, &mut m)?;
    state.logical_bytes = m.bytes;
    Ok(state)
}
