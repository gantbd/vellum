//! Pure, privileged projection building from an already readable compiled root.
//! Supplied visibility is normalized data, not authenticated authority. This is
//! NOT an F07 recipient endpoint, read context, currentness check or release grant.
//! An engine reader must validate scope/current authority and replace private
//! origin identities with context-local opaque references before public release.
use super::compile::{Meter, reserve};
use super::*;
use std::mem::size_of;
mod equivalence;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsProjectionLimitsV1 {
    pub max_bytes: usize,
    pub max_work: usize,
    /// Native value/type graph edge depth; the root is at depth zero.
    pub max_depth: usize,
}
#[derive(PartialEq, Eq)]
pub enum SettingProjectionOriginV1 {
    Library,
    Layer(SettingIdentityV1),
}
pub struct SettingProjectionGrantV1<'a> {
    pub key: &'a str,
    pub visible: bool,
    pub value: bool,
    pub origins: bool,
    pub declaration: bool,
    pub origin_ids: &'a [SettingProjectionOriginV1],
}
#[derive(PartialEq, Eq)]
pub enum SettingProjectionOriginsV1 {
    Whole(Vec<SettingProjectionOriginV1>),
    Fields(Vec<(String, Vec<SettingProjectionOriginV1>)>),
}
impl SettingProjectionOriginsV1 {
    pub const fn coverage(&self) -> &'static str {
        "permitted-only"
    }
}
/// Accepted declaration only. Deliberately has no default, layer generation or
/// private association fields. Declaration permission covers literal type members.
pub struct SettingProjectionDeclarationV1 {
    pub key: String,
    pub value_type: SettingProjectionTypeV1,
    pub domain: SettingDomainV1,
    pub scopes: Vec<SettingScopeV1>,
    pub composition: SettingCompositionV1,
    pub sensitive: bool,
}
pub struct SettingProjectionRowV1 {
    pub key: String,
    pub value: Option<SettingProjectionValueV1>,
    pub origins: Option<SettingProjectionOriginsV1>,
    pub declaration: Option<SettingProjectionDeclarationV1>,
}
/// Owned filtered data for a later guarded reader, not recipient release.
/// Origin identities remain privileged here, exactly as on `CompiledSettingsV1::read`.
pub struct NativeSettingsProjectionV1 {
    pub rows: Vec<SettingProjectionRowV1>,
    pub capabilities: bool,
    /// Conservative construction charge including traversal frames, headers,
    /// edges and UTF-8. Not exact retained bytes or allocator/RSS measurement.
    pub logical_bytes: usize,
}
/// Owned flat value graph. Both engine copy and drop are nonrecursive.
pub struct SettingProjectionValueV1 {
    nodes: Vec<Node>,
    root: usize,
}
impl SettingProjectionValueV1 {
    pub fn read(&self) -> SettingValueRef<'_> {
        SettingValueRef {
            nodes: &self.nodes,
            node: self.root,
        }
    }
}
/// Owned flat declaration graph, with a separate flat graph for enum literals.
pub struct SettingProjectionTypeV1 {
    types: Vec<TypeNode>,
    values: Vec<Node>,
    root: usize,
}
#[derive(Clone, Copy)]
pub struct SettingProjectionTypeRefV1<'a> {
    state: &'a SettingProjectionTypeV1,
    node: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingProjectionTypeKindV1 {
    Boolean,
    Integer { min: i64, max: i64 },
    Text,
    Enum,
    List,
    Map,
}
impl SettingProjectionTypeV1 {
    pub fn read(&self) -> SettingProjectionTypeRefV1<'_> {
        SettingProjectionTypeRefV1 {
            state: self,
            node: self.root,
        }
    }
}
impl<'a> SettingProjectionTypeRefV1<'a> {
    pub fn kind(self) -> SettingProjectionTypeKindV1 {
        match self.state.types[self.node] {
            TypeNode::Boolean => SettingProjectionTypeKindV1::Boolean,
            TypeNode::Integer(min, max) => SettingProjectionTypeKindV1::Integer { min, max },
            TypeNode::Text => SettingProjectionTypeKindV1::Text,
            TypeNode::Enum(_) => SettingProjectionTypeKindV1::Enum,
            TypeNode::List(_) => SettingProjectionTypeKindV1::List,
            TypeNode::Map(_) => SettingProjectionTypeKindV1::Map,
        }
    }
    pub fn enum_len(self) -> Option<usize> {
        if let TypeNode::Enum(v) = &self.state.types[self.node] {
            Some(v.len())
        } else {
            None
        }
    }
    pub fn enum_value(self, index: usize) -> Option<SettingValueRef<'a>> {
        if let TypeNode::Enum(v) = &self.state.types[self.node] {
            v.get(index).map(|n| SettingValueRef {
                nodes: &self.state.values,
                node: *n,
            })
        } else {
            None
        }
    }
    pub fn list_item(self) -> Option<Self> {
        if let TypeNode::List(n) = self.state.types[self.node] {
            Some(Self {
                state: self.state,
                node: n,
            })
        } else {
            None
        }
    }
    pub fn field_count(self) -> Option<usize> {
        if let TypeNode::Map(v) = &self.state.types[self.node] {
            Some(v.len())
        } else {
            None
        }
    }
    pub fn field_at(self, index: usize) -> Option<(&'a str, Self)> {
        if let TypeNode::Map(v) = &self.state.types[self.node] {
            v.get(index).map(|(k, n)| {
                (
                    k.as_str(),
                    Self {
                        state: self.state,
                        node: *n,
                    },
                )
            })
        } else {
            None
        }
    }
    pub fn field(self, key: &str) -> Option<Self> {
        if let TypeNode::Map(v) = &self.state.types[self.node] {
            v.iter().find(|(k, _)| k == key).map(|(_, n)| Self {
                state: self.state,
                node: *n,
            })
        } else {
            None
        }
    }
}
impl NativeSettingsProjectionV1 {
    pub const fn coverage(&self) -> &'static str {
        "permitted-only"
    }
}
macro_rules! redacted {($($t:ty),+) => {$(impl std::fmt::Debug for $t {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {f.write_str("<settings-projection:redacted>")}
})+};}
redacted!(
    SettingProjectionOriginV1,
    SettingProjectionGrantV1<'_>,
    SettingProjectionOriginsV1,
    SettingProjectionDeclarationV1,
    SettingProjectionRowV1,
    NativeSettingsProjectionV1,
    SettingProjectionValueV1,
    SettingProjectionTypeV1,
    SettingProjectionTypeRefV1<'_>
);

type Frame = (usize, usize, usize);
fn node_slot(nodes: &mut Vec<Node>, m: &mut Meter) -> Result<usize, SettingsError> {
    m.bytes(size_of::<Node>())?;
    // Reachable graph size is not known up front. Amortized growth avoids one
    // realloc per node; logical cost is not an allocator-capacity/RSS claim.
    nodes.try_reserve(1).map_err(|_| SettingsError::Capacity)?;
    let index = nodes.len();
    nodes.push(Node::Null);
    Ok(index)
}
fn type_slot(types: &mut Vec<TypeNode>, m: &mut Meter) -> Result<usize, SettingsError> {
    m.bytes(size_of::<TypeNode>())?;
    types.try_reserve(1).map_err(|_| SettingsError::Capacity)?;
    let index = types.len();
    types.push(TypeNode::Boolean);
    Ok(index)
}
fn frames(stack: &mut Vec<Frame>, count: usize, m: &mut Meter) -> Result<(), SettingsError> {
    m.steps(count)?;
    m.edges::<Frame>(count)?;
    stack
        .try_reserve(count)
        .map_err(|_| SettingsError::Capacity)
}
fn copy_value(
    s: &CompiledSettingsV1,
    index: usize,
    nodes: &mut Vec<Node>,
    m: &mut Meter,
) -> Result<usize, SettingsError> {
    let root = node_slot(nodes, m)?;
    let mut stack = Vec::new();
    frames(&mut stack, 1, m)?;
    stack.push((index, root, 0));
    while let Some((source, at, depth)) = stack.pop() {
        m.tick()?;
        if depth > m.limits.max_value_depth {
            return Err(SettingsError::Limit);
        }
        let next = depth.checked_add(1).ok_or(SettingsError::Limit)?;
        nodes[at] = match s.nodes.get(source).ok_or(SettingsError::Faulted)? {
            Node::Null => Node::Null,
            Node::Boolean(v) => Node::Boolean(*v),
            Node::Number(v) => {
                if !f64::from_bits(*v).is_finite() {
                    return Err(SettingsError::Faulted);
                }
                Node::Number(*v)
            }
            Node::Text(v) => Node::Text(m.string(v)?),
            Node::List(rows) => {
                frames(&mut stack, rows.len(), m)?;
                m.edges::<usize>(rows.len())?;
                let mut out = Vec::new();
                reserve(&mut out, rows.len())?;
                for source in rows {
                    let child = node_slot(nodes, m)?;
                    out.push(child);
                    stack.push((*source, child, next));
                }
                Node::List(out)
            }
            Node::Map(rows) => {
                frames(&mut stack, rows.len(), m)?;
                m.edges::<(String, usize)>(rows.len())?;
                let mut out = Vec::new();
                reserve(&mut out, rows.len())?;
                for (key, source) in rows {
                    let child = node_slot(nodes, m)?;
                    out.push((m.string(key)?, child));
                    stack.push((*source, child, next));
                }
                Node::Map(out)
            }
        };
    }
    Ok(root)
}
fn value(
    s: &CompiledSettingsV1,
    index: usize,
    m: &mut Meter,
) -> Result<SettingProjectionValueV1, SettingsError> {
    let mut nodes = Vec::new();
    let root = copy_value(s, index, &mut nodes, m)?;
    Ok(SettingProjectionValueV1 { nodes, root })
}
fn value_type(
    s: &CompiledSettingsV1,
    index: usize,
    m: &mut Meter,
) -> Result<SettingProjectionTypeV1, SettingsError> {
    let mut types = Vec::new();
    let mut values = Vec::new();
    let root = type_slot(&mut types, m)?;
    let mut stack = Vec::new();
    frames(&mut stack, 1, m)?;
    stack.push((index, root, 0));
    while let Some((source, at, depth)) = stack.pop() {
        m.tick()?;
        if depth > m.limits.max_schema_depth {
            return Err(SettingsError::Limit);
        }
        let next = depth.checked_add(1).ok_or(SettingsError::Limit)?;
        types[at] = match s.types.get(source).ok_or(SettingsError::Faulted)? {
            TypeNode::Boolean => TypeNode::Boolean,
            TypeNode::Integer(min, max) => TypeNode::Integer(*min, *max),
            TypeNode::Text => TypeNode::Text,
            TypeNode::Enum(rows) => {
                m.steps(rows.len())?;
                m.edges::<usize>(rows.len())?;
                let mut out = Vec::new();
                reserve(&mut out, rows.len())?;
                for row in rows {
                    out.push(copy_value(s, *row, &mut values, m)?);
                }
                TypeNode::Enum(out)
            }
            TypeNode::List(source) => {
                frames(&mut stack, 1, m)?;
                let child = type_slot(&mut types, m)?;
                stack.push((*source, child, next));
                TypeNode::List(child)
            }
            TypeNode::Map(rows) => {
                frames(&mut stack, rows.len(), m)?;
                m.edges::<(String, usize)>(rows.len())?;
                let mut out = Vec::new();
                reserve(&mut out, rows.len())?;
                for (key, source) in rows {
                    let child = type_slot(&mut types, m)?;
                    out.push((m.string(key)?, child));
                    stack.push((*source, child, next));
                }
                TypeNode::Map(out)
            }
        };
    }
    Ok(SettingProjectionTypeV1 {
        types,
        values,
        root,
    })
}
fn origin_matches(
    s: &CompiledSettingsV1,
    row: Option<usize>,
    id: &SettingProjectionOriginV1,
) -> Result<bool, SettingsError> {
    Ok(match (row, id) {
        (None, SettingProjectionOriginV1::Library) => true,
        (Some(i), SettingProjectionOriginV1::Layer(id)) => {
            s.layers.get(i).ok_or(SettingsError::Faulted)?.identity == *id
        }
        _ => false,
    })
}
fn actual_contains(
    s: &CompiledSettingsV1,
    origins: &Origins,
    id: &SettingProjectionOriginV1,
    m: &mut Meter,
) -> Result<bool, SettingsError> {
    if let Origins::Fields(fields) = origins {
        m.steps(fields.len())?;
    }
    let mut contains = false;
    let mut visit = |rows: &[Option<usize>]| -> Result<(), SettingsError> {
        for row in rows {
            m.tick()?;
            contains |= origin_matches(s, *row, id)?;
        }
        Ok(())
    };
    match origins {
        Origins::Whole(rows) => visit(rows)?,
        Origins::Fields(fields) => {
            for (_, rows) in fields {
                visit(rows)?;
            }
        }
    }
    Ok(contains)
}
fn copy_origins(
    s: &CompiledSettingsV1,
    rows: &[Option<usize>],
    allowed: &[SettingProjectionOriginV1],
    m: &mut Meter,
) -> Result<Vec<SettingProjectionOriginV1>, SettingsError> {
    // Count before allocating so hidden origins do not inflate the output or
    // its declared byte charge. Work remains bounded over the complete source.
    let permitted = |row: Option<usize>, m: &mut Meter| -> Result<bool, SettingsError> {
        for id in allowed {
            m.tick()?;
            if origin_matches(s, row, id)? {
                return Ok(true);
            }
        }
        Ok(false)
    };
    let mut n = 0usize;
    for row in rows {
        m.tick()?;
        n += usize::from(permitted(*row, m)?);
    }
    m.edges::<SettingProjectionOriginV1>(n)?;
    let mut out = Vec::new();
    reserve(&mut out, n)?;
    for row in rows {
        m.tick()?;
        if !permitted(*row, m)? {
            continue;
        }
        out.push(match row {
            None => SettingProjectionOriginV1::Library,
            Some(i) => SettingProjectionOriginV1::Layer(compile::identity(
                &s.layers.get(*i).ok_or(SettingsError::Faulted)?.identity,
                &s.authority,
                m,
            )?),
        });
    }
    Ok(out)
}
fn project(
    s: &CompiledSettingsV1,
    target: &SettingIdentityV1,
    grants: &[SettingProjectionGrantV1<'_>],
    capabilities: bool,
    limits: SettingsProjectionLimitsV1,
) -> Result<NativeSettingsProjectionV1, SettingsError> {
    if grants.len() != s.schemas.len() {
        return Err(SettingsError::Unavailable);
    }
    let mut profile = s.limits;
    profile.max_state_bytes = limits.max_bytes;
    profile.max_work = limits.max_work;
    profile.max_value_depth = profile.max_value_depth.min(limits.max_depth);
    profile.max_schema_depth = profile.max_schema_depth.min(limits.max_depth);
    let mut m = Meter::new(profile);
    m.bytes(size_of::<NativeSettingsProjectionV1>())?;
    m.steps(s.targets.len())?;
    let target = &s.targets[s.target_index(target)?];
    for (i, g) in grants.iter().enumerate() {
        m.steps(
            i.checked_add(s.schemas.len().checked_mul(2).ok_or(SettingsError::Limit)?)
                .ok_or(SettingsError::Limit)?,
        )?;
        if grants[..i].iter().any(|p| p.key == g.key)
            || !s.schemas.iter().any(|schema| schema.key == g.key)
            || (!g.visible && (g.value || g.origins || g.declaration || !g.origin_ids.is_empty()))
            || (!g.origins && !g.origin_ids.is_empty())
            || g.origin_ids.len() > s.layers.len().checked_add(1).ok_or(SettingsError::Limit)?
        {
            return Err(SettingsError::Unavailable);
        }
        let key = s
            .schemas
            .iter()
            .position(|schema| schema.key == g.key)
            .ok_or(SettingsError::Faulted)?;
        let actual = &target
            .resolved
            .get(key)
            .ok_or(SettingsError::Faulted)?
            .origins;
        for (j, id) in g.origin_ids.iter().enumerate() {
            m.steps(j)?;
            if g.origin_ids[..j].contains(id) || !actual_contains(s, actual, id, &mut m)? {
                return Err(SettingsError::Unavailable);
            }
        }
    }
    let count = grants.iter().filter(|g| g.visible).count();
    m.steps(grants.len())?;
    m.edges::<SettingProjectionRowV1>(count)?;
    let mut rows = Vec::new();
    reserve(&mut rows, count)?;
    for (i, schema) in s.schemas.iter().enumerate() {
        m.steps(grants.len())?;
        let g = grants
            .iter()
            .find(|g| g.key == schema.key)
            .ok_or(SettingsError::Faulted)?;
        if !g.visible {
            continue;
        }
        let actual = target.resolved.get(i).ok_or(SettingsError::Faulted)?;
        let data = if g.value {
            Some(value(s, actual.value, &mut m)?)
        } else {
            None
        };
        let origins = if g.origins {
            Some(match &actual.origins {
                Origins::Whole(ids) => {
                    SettingProjectionOriginsV1::Whole(copy_origins(s, ids, g.origin_ids, &mut m)?)
                }
                Origins::Fields(fields) => {
                    m.edges::<(String, Vec<SettingProjectionOriginV1>)>(fields.len())?;
                    let mut out = Vec::new();
                    reserve(&mut out, fields.len())?;
                    for (key, ids) in fields {
                        m.tick()?;
                        out.push((m.string(key)?, copy_origins(s, ids, g.origin_ids, &mut m)?));
                    }
                    SettingProjectionOriginsV1::Fields(out)
                }
            })
        } else {
            None
        };
        let declaration = if g.declaration {
            m.bytes(size_of::<SettingProjectionDeclarationV1>())?;
            m.edges::<SettingScopeV1>(schema.scopes.len())?;
            let mut scopes = Vec::new();
            reserve(&mut scopes, schema.scopes.len())?;
            m.steps(schema.scopes.len())?;
            scopes.extend_from_slice(&schema.scopes);
            Some(SettingProjectionDeclarationV1 {
                key: m.string(&schema.key)?,
                value_type: value_type(s, schema.value_type, &mut m)?,
                domain: schema.domain,
                scopes,
                composition: schema.composition,
                sensitive: schema.sensitive,
            })
        } else {
            None
        };
        rows.push(SettingProjectionRowV1 {
            key: m.string(&schema.key)?,
            value: data,
            origins,
            declaration,
        });
    }
    Ok(NativeSettingsProjectionV1 {
        rows,
        capabilities,
        logical_bytes: m.used_bytes(),
    })
}
/// All-or-error pure data projection from an already privileged compiled root.
/// Complete key facts, origin subsets and independent value/origin/declaration
/// selections are checked; visibility order never changes accepted schema order.
/// Neither these Boolean facts nor sensitivity metadata establish authority.
pub fn project_settings_v1(
    s: &CompiledSettingsV1,
    target: &SettingIdentityV1,
    grants: &[SettingProjectionGrantV1<'_>],
    capabilities: bool,
    limits: SettingsProjectionLimitsV1,
    r: &mut Recorder,
) -> Result<NativeSettingsProjectionV1, SettingsError> {
    let timer = r.begin_service_timing();
    let result = project(s, target, grants, capabilities, limits);
    r.record(Observation {
        boundary: Boundary::SettingsProjection,
        disposition: match &result {
            Ok(_) => Disposition::Completed,
            Err(SettingsError::Faulted) => Disposition::Unknown,
            _ => Disposition::Refused,
        },
        service_time: timer.finish(),
    });
    result
}
