//! Inert typed semantic configuration. No UI domains, callbacks or JSON Schema VM.
mod capture;
mod definition;
mod layers;
mod write_footprint;
pub(crate) use definition::{DefinitionCapture, DefinitionPlan};
pub use definition::{
    SettingsDeclarationV1, SettingsDefinitionLimitsV1, SettingsDefinitionOutcomeV1,
};
pub(crate) use layers::{LayerCapture, LayerCreateInput, LayerPlan};
pub use layers::{
    SettingLayerCreateV1, SettingLayerRemoveV1, SettingsLayerBasisV1, SettingsLayerChangeV1,
    SettingsLayerLimitsV1, SettingsLayerOutcomeV1,
};
pub(crate) use write_footprint::WriteFootprint;
mod compile;
mod mutate;
mod projection;
mod resolve;
pub use projection::{
    NativeSettingsProjectionV1, SettingProjectionDeclarationV1, SettingProjectionGrantV1,
    SettingProjectionOriginV1, SettingProjectionOriginsV1, SettingProjectionRowV1,
    SettingProjectionTypeKindV1, SettingProjectionTypeRefV1, SettingProjectionTypeV1,
    SettingProjectionValueV1, SettingsProjectionLimitsV1, project_settings_v1,
};
#[cfg(test)]
mod projection_tests;
pub(crate) use capture::SettingsRequestCapture;
pub(crate) use mutate::MutationPlan;
pub(crate) use mutate::SettingChangeInput;
pub use mutate::{
    SettingChangeV1, SettingOperationV1, SettingsActivationLimitsV1, SettingsUpdateV1,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsBasisV1 {
    pub configuration_generation: u64,
    pub schema_generation: u64,
    pub restriction_generation: u64,
    pub association_generation: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsInterpretationEpochsV1 {
    pub schema: u64,
    pub restriction: u64,
    pub association: u64,
}
impl SettingsBasisV1 {
    pub fn interpretation(self) -> SettingsInterpretationEpochsV1 {
        SettingsInterpretationEpochsV1 {
            schema: self.schema_generation,
            restriction: self.restriction_generation,
            association: self.association_generation,
        }
    }
    pub(crate) fn initial() -> Self {
        Self {
            configuration_generation: 1,
            schema_generation: 1,
            restriction_generation: 1,
            association_generation: 1,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsGenerationsV1 {
    pub source: u64,
    pub serialization: u64,
    pub analysis: u64,
}
use crate::instrumentation::{Boundary, Disposition, Observation, Recorder};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingDomainV1 {
    Source,
    Serialization,
    Analysis,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SettingScopeV1 {
    Host,
    Language,
    Workspace,
    Document,
    View,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingCompositionV1 {
    Replace,
    MergeMap,
}
pub enum SettingValueV1 {
    Null,
    Boolean(bool),
    Number(f64),
    Text(String),
    List(Vec<SettingValueV1>),
    Map(Vec<(String, SettingValueV1)>),
}
impl std::fmt::Debug for SettingValueV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SettingValueV1(<redacted>)")
    }
}
pub enum SettingTypeV1 {
    Boolean,
    Integer { min: i64, max: i64 },
    Text,
    Enum(Vec<SettingValueV1>),
    List(Box<SettingTypeV1>),
    Map(Vec<(String, SettingTypeV1)>),
}
#[derive(PartialEq, Eq)]
pub struct SettingIdentityV1 {
    pub authority: String,
    pub id: String,
    pub incarnation: u64,
}
impl std::fmt::Debug for SettingIdentityV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SettingIdentityV1(<redacted>)")
    }
}
pub struct SettingSchemaV1 {
    pub key: String,
    pub value_type: SettingTypeV1,
    pub default: SettingValueV1,
    pub domain: SettingDomainV1,
    pub scopes: Vec<SettingScopeV1>,
    pub composition: SettingCompositionV1,
    pub sensitive: bool,
}
pub enum SettingContributionV1 {
    Set(SettingValueV1),
    Reset,
}
pub struct SettingLayerV1 {
    pub identity: SettingIdentityV1,
    pub scope: SettingScopeV1,
    pub generation: u64,
    pub values: Vec<(String, SettingContributionV1)>,
}
pub struct SettingTargetV1 {
    pub identity: SettingIdentityV1,
    pub layers: Vec<SettingIdentityV1>,
}
pub struct SettingsDefinitionV1 {
    pub schemas: Vec<SettingSchemaV1>,
    pub restrictions: Vec<(String, Vec<SettingValueV1>)>,
    pub layers: Vec<SettingLayerV1>,
    pub targets: Vec<SettingTargetV1>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsLimitsV1 {
    pub max_keys: usize,
    pub max_layers: usize,
    pub max_targets: usize,
    pub max_identifier_bytes: usize,
    pub max_schema_nodes: usize,
    pub max_schema_depth: usize,
    pub max_value_nodes: usize,
    pub max_value_depth: usize,
    /// Logical headers, edges and UTF-8 bytes, not allocator/transient peak memory.
    pub max_state_bytes: usize,
    pub max_work: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsError {
    InvalidDefinition,
    InvalidValue,
    InvalidIdentity,
    UnknownKey,
    UnknownLayer,
    Duplicate,
    IllegalScope,
    Conflict,
    RestrictionDenied,
    Limit,
    Capacity,
    Unavailable,
    Faulted,
    VersionExhausted,
    Stale,
}
enum Node {
    Null,
    Boolean(bool),
    Number(u64),
    Text(String),
    List(Vec<usize>),
    Map(Vec<(String, usize)>),
}
enum TypeNode {
    Boolean,
    Integer(i64, i64),
    Text,
    Enum(Vec<usize>),
    List(usize),
    Map(Vec<(String, usize)>),
}
struct Schema {
    key: String,
    value_type: usize,
    default: usize,
    domain: SettingDomainV1,
    scopes: Vec<SettingScopeV1>,
    composition: SettingCompositionV1,
    sensitive: bool,
}
enum Contribution {
    Set(usize),
    Reset,
}
struct Layer {
    identity: SettingIdentityV1,
    scope: SettingScopeV1,
    generation: u64,
    values: Vec<(usize, Contribution)>,
}
struct Target {
    native_identity: usize,
    generations: [u64; 3],
    identity: SettingIdentityV1,
    layers: Vec<usize>,
    resolved: Vec<Resolved>,
}
enum Origins {
    Whole(Vec<Option<usize>>),
    Fields(Vec<(String, Vec<Option<usize>>)>),
}
struct Resolved {
    value: usize,
    origins: Origins,
}
pub struct CompiledSettingsV1 {
    layer_incarnation: u64,
    next_target_identity: usize,
    basis: SettingsBasisV1,
    authority: String,
    limits: SettingsLimitsV1,
    nodes: Vec<Node>,
    types: Vec<TypeNode>,
    schemas: Vec<Schema>,
    restrictions: Vec<(usize, Vec<usize>)>,
    layers: Vec<Layer>,
    targets: Vec<Target>,
    logical_bytes: usize,
}
impl std::fmt::Debug for CompiledSettingsV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CompiledSettingsV1(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsUsageV1 {
    pub keys: usize,
    pub layers: usize,
    pub targets: usize,
    pub schema_nodes: usize,
    pub value_nodes: usize,
    pub logical_bytes: usize,
}
#[derive(Clone, Copy)]
pub struct SettingValueRef<'a> {
    nodes: &'a [Node],
    node: usize,
}
impl std::fmt::Debug for SettingValueRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SettingValueRef(<redacted>)")
    }
}
impl<'a> SettingValueRef<'a> {
    pub fn is_null(self) -> bool {
        matches!(self.nodes[self.node], Node::Null)
    }
    pub fn boolean(self) -> Option<bool> {
        if let Node::Boolean(v) = self.nodes[self.node] {
            Some(v)
        } else {
            None
        }
    }
    pub fn number(self) -> Option<f64> {
        if let Node::Number(v) = self.nodes[self.node] {
            Some(f64::from_bits(v))
        } else {
            None
        }
    }
    pub fn text(self) -> Option<&'a str> {
        if let Node::Text(v) = &self.nodes[self.node] {
            Some(v)
        } else {
            None
        }
    }
    pub fn list_len(self) -> Option<usize> {
        if let Node::List(v) = &self.nodes[self.node] {
            Some(v.len())
        } else {
            None
        }
    }
    pub fn list_item(self, index: usize) -> Option<Self> {
        if let Node::List(v) = &self.nodes[self.node] {
            v.get(index).map(|node| Self {
                nodes: self.nodes,
                node: *node,
            })
        } else {
            None
        }
    }
    pub fn map_len(self) -> Option<usize> {
        if let Node::Map(v) = &self.nodes[self.node] {
            Some(v.len())
        } else {
            None
        }
    }
    pub fn field_at(self, index: usize) -> Option<(&'a str, Self)> {
        if let Node::Map(v) = &self.nodes[self.node] {
            v.get(index).map(|(key, node)| {
                (
                    key.as_str(),
                    Self {
                        nodes: self.nodes,
                        node: *node,
                    },
                )
            })
        } else {
            None
        }
    }
    pub fn field(self, key: &str) -> Option<Self> {
        if let Node::Map(v) = &self.nodes[self.node] {
            v.iter().find(|(k, _)| k == key).map(|(_, node)| Self {
                nodes: self.nodes,
                node: *node,
            })
        } else {
            None
        }
    }
}
#[derive(Debug)]
pub enum SettingOriginV1<'a> {
    Library,
    Layer(&'a SettingIdentityV1),
}
pub struct SettingOriginsRef<'a> {
    state: &'a CompiledSettingsV1,
    origins: &'a Origins,
}
impl<'a> SettingOriginsRef<'a> {
    pub fn iter(
        &self,
        field: Option<&str>,
    ) -> Option<impl Iterator<Item = SettingOriginV1<'a>> + 'a> {
        let rows = match (self.origins, field) {
            (Origins::Whole(rows), None) => rows,
            (Origins::Fields(fields), Some(field)) => &fields.iter().find(|(k, _)| k == field)?.1,
            _ => return None,
        };
        let state = self.state;
        Some(rows.iter().map(move |index| {
            index.map_or(SettingOriginV1::Library, |index| {
                SettingOriginV1::Layer(&state.layers[index].identity)
            })
        }))
    }
}
pub struct SettingReadV1<'a> {
    pub value: SettingValueRef<'a>,
    pub origins: SettingOriginsRef<'a>,
    pub domain: SettingDomainV1,
    pub sensitive: bool,
}
impl CompiledSettingsV1 {
    #[cfg(test)]
    pub(crate) fn reverse_targets_for_test(&mut self) {
        self.targets.reverse();
    }
    #[cfg(test)]
    pub(crate) fn set_layer_basis_for_test(&mut self, basis: SettingsLayerBasisV1) {
        self.basis.configuration_generation = basis.configuration_generation;
        self.layer_incarnation = basis.layer_incarnation;
    }
    #[cfg(test)]
    pub(crate) fn retire_native_target_for_test(&mut self, id: usize) -> Result<(), SettingsError> {
        let index = self.native_target_index(id)?;
        self.targets.remove(index);
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn renew_native_target_for_test(
        &mut self,
        id: usize,
    ) -> Result<usize, SettingsError> {
        let index = self.native_target_index(id)?;
        let fresh = self.issue_target_identity()?;
        self.targets[index].native_identity = fresh;
        Ok(fresh)
    }
    pub(crate) fn issue_target_identity(&mut self) -> Result<usize, SettingsError> {
        let id = self.next_target_identity;
        self.next_target_identity = id.checked_add(1).ok_or(SettingsError::VersionExhausted)?;
        Ok(id)
    }
    pub(crate) fn native_target_identity(&self, index: usize) -> Result<usize, SettingsError> {
        let id = self
            .targets
            .get(index)
            .ok_or(SettingsError::Unavailable)?
            .native_identity;
        if id >= self.next_target_identity {
            return Err(SettingsError::Faulted);
        }
        if self.native_target_index(id)? != index {
            return Err(SettingsError::Faulted);
        }
        Ok(id)
    }
    pub(crate) fn native_target_index(&self, id: usize) -> Result<usize, SettingsError> {
        if id >= self.next_target_identity {
            return Err(SettingsError::Unavailable);
        }
        let mut found = None;
        for (index, target) in self.targets.iter().enumerate() {
            if target.native_identity >= self.next_target_identity {
                return Err(SettingsError::Faulted);
            }
            if target.native_identity == id {
                if found.is_some() {
                    return Err(SettingsError::Faulted);
                }
                found = Some(index);
            }
        }
        found.ok_or(SettingsError::Unavailable)
    }
    pub fn basis(&self) -> SettingsBasisV1 {
        self.basis
    }
    pub(crate) fn generations(
        &self,
        target: usize,
    ) -> Result<SettingsGenerationsV1, SettingsError> {
        let g = self
            .targets
            .get(target)
            .ok_or(SettingsError::Unavailable)?
            .generations;
        Ok(SettingsGenerationsV1 {
            source: g[0],
            serialization: g[1],
            analysis: g[2],
        })
    }
    pub fn limits(&self) -> SettingsLimitsV1 {
        self.limits
    }
    pub fn layer_generation(&self, layer: &SettingIdentityV1) -> Result<u64, SettingsError> {
        self.layers
            .iter()
            .find(|l| l.identity == *layer)
            .map(|l| l.generation)
            .ok_or(SettingsError::Unavailable)
    }
    pub fn usage(&self) -> SettingsUsageV1 {
        SettingsUsageV1 {
            keys: self.schemas.len(),
            layers: self.layers.len(),
            targets: self.targets.len(),
            schema_nodes: self.types.len(),
            value_nodes: self.nodes.len(),
            logical_bytes: self.logical_bytes,
        }
    }
    pub(crate) fn target_index(&self, target: &SettingIdentityV1) -> Result<usize, SettingsError> {
        if target.authority != self.authority {
            return Err(SettingsError::Unavailable);
        }
        self.targets
            .iter()
            .position(|t| t.identity == *target)
            .ok_or(SettingsError::Unavailable)
    }
    pub(crate) fn target_identity(&self, at: usize) -> Result<&SettingIdentityV1, SettingsError> {
        self.targets
            .get(at)
            .map(|t| &t.identity)
            .ok_or(SettingsError::Unavailable)
    }
    pub fn read(
        &self,
        target: &SettingIdentityV1,
        key: &str,
    ) -> Result<SettingReadV1<'_>, SettingsError> {
        self.read_index(self.target_index(target)?, key)
    }
    pub(crate) fn read_index(
        &self,
        target: usize,
        key: &str,
    ) -> Result<SettingReadV1<'_>, SettingsError> {
        let target = self.targets.get(target).ok_or(SettingsError::Unavailable)?;
        let schema = self
            .schemas
            .iter()
            .position(|s| s.key == key)
            .ok_or(SettingsError::UnknownKey)?;
        let result = target.resolved.get(schema).ok_or(SettingsError::Faulted)?;
        Ok(SettingReadV1 {
            value: SettingValueRef {
                nodes: &self.nodes,
                node: result.value,
            },
            origins: SettingOriginsRef {
                state: self,
                origins: &result.origins,
            },
            domain: self.schemas[schema].domain,
            sensitive: self.schemas[schema].sensitive,
        })
    }
}
pub fn compile_settings_v1(
    definition: &SettingsDefinitionV1,
    authority: &str,
    limits: SettingsLimitsV1,
    r: &mut Recorder,
) -> Result<CompiledSettingsV1, SettingsError> {
    let result = compile::compile(definition, authority, limits);
    observe(r, Boundary::SettingsCompilation, &result);
    result
}
pub(crate) fn observe<T>(r: &mut Recorder, boundary: Boundary, result: &Result<T, SettingsError>) {
    observe_service(r, boundary, result, None);
}
pub(crate) fn observe_service<T>(
    r: &mut Recorder,
    boundary: Boundary,
    result: &Result<T, SettingsError>,
    service_time: Option<std::time::Duration>,
) {
    r.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(SettingsError::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time,
    });
}
