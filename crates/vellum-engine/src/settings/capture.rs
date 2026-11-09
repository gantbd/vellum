//! Owned inert invocation intent, independent of key registration and current state.
use super::compile::{Meter, reserve};
use super::*;
pub(super) enum Operation {
    Set(usize),
    Remove,
    Reset,
}
pub(super) struct Change {
    pub(super) layer: SettingIdentityV1,
    pub(super) expected: u64,
    pub(super) key: String,
    pub(super) operation: Operation,
}
pub(crate) struct SettingsRequestCapture {
    pub(super) changes: Vec<Change>,
    pub(super) values: CompiledSettingsV1,
    pub(crate) profile: SettingsActivationLimitsV1,
}
impl std::fmt::Debug for SettingsRequestCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SettingsRequestCapture(<redacted>)")
    }
}
impl SettingsRequestCapture {
    pub(crate) fn retained_bytes(&self) -> usize {
        self.values.logical_bytes
    }
    pub(crate) fn copy(
        input: &[SettingChangeV1],
        profile: SettingsActivationLimitsV1,
        max_bytes: usize,
    ) -> Result<Self, SettingsError> {
        Self::copy_input_iter(
            input.iter().map(SettingChangeInput::from),
            profile,
            max_bytes,
        )
    }
    pub(crate) fn copy_inputs(
        input: &[SettingChangeInput<'_>],
        profile: SettingsActivationLimitsV1,
        max_bytes: usize,
    ) -> Result<Self, SettingsError> {
        Self::copy_input_iter(input.iter().copied(), profile, max_bytes)
    }
    pub(crate) fn condition_at(&self, index: usize) -> Option<(&SettingIdentityV1, u64)> {
        self.changes.get(index).map(|c| (&c.layer, c.expected))
    }
    pub(crate) fn layers(&self) -> impl ExactSizeIterator<Item = &SettingIdentityV1> {
        self.changes.iter().map(|c| &c.layer)
    }
    fn copy_input_iter<'a>(
        input: impl ExactSizeIterator<Item = SettingChangeInput<'a>>,
        profile: SettingsActivationLimitsV1,
        max_bytes: usize,
    ) -> Result<Self, SettingsError> {
        if input.len() > profile.max_changes {
            return Err(SettingsError::Limit);
        }
        let limits = SettingsLimitsV1 {
            max_keys: 0,
            max_layers: 0,
            max_targets: 0,
            max_identifier_bytes: profile.max_request_bytes,
            max_schema_nodes: 0,
            max_schema_depth: 0,
            max_value_nodes: profile.max_request_nodes,
            max_value_depth: profile.max_request_depth,
            max_state_bytes: max_bytes.min(profile.max_request_bytes),
            max_work: profile.max_work,
        };
        let mut m = Meter::new(limits);
        m.bytes(std::mem::size_of::<Self>())?;
        m.edges::<Change>(input.len())?;
        let mut values = CompiledSettingsV1 {
            layer_incarnation: 0,
            next_target_identity: 0,
            basis: SettingsBasisV1::initial(),
            authority: String::new(),
            limits,
            nodes: Vec::new(),
            types: Vec::new(),
            schemas: Vec::new(),
            restrictions: Vec::new(),
            layers: Vec::new(),
            targets: Vec::new(),
            logical_bytes: 0,
        };
        let mut changes = Vec::new();
        reserve(&mut changes, input.len())?;
        for change in input {
            m.tick()?;
            let layer = SettingIdentityV1 {
                authority: m.string(&change.layer.authority)?,
                id: m.string(&change.layer.id)?,
                incarnation: change.layer.incarnation,
            };
            let key = m.string(change.key)?;
            let operation = match change.operation {
                SettingOperationV1::Set(value) => {
                    Operation::Set(compile::value(&mut values, value, &mut m)?)
                }
                SettingOperationV1::Remove => Operation::Remove,
                SettingOperationV1::Reset => Operation::Reset,
            };
            changes.push(Change {
                layer,
                key,
                expected: change.expected_generation,
                operation,
            });
        }
        // Admission must leave the configured per-pass work ceiling sufficient
        // for a complete matching retry, even for a captured native refusal.
        // Map lookup visits each unique field position once per comparison.
        let mut comparison_work = changes.len();
        for node in &values.nodes {
            m.tick()?;
            let edges = match node {
                Node::List(rows) => rows.len(),
                Node::Map(rows) => {
                    let n = rows.len();
                    n.checked_add(
                        n.checked_mul(n.checked_add(1).ok_or(SettingsError::Limit)?)
                            .ok_or(SettingsError::Limit)?
                            / 2,
                    )
                    .ok_or(SettingsError::Limit)?
                }
                _ => 0,
            };
            comparison_work = comparison_work
                .checked_add(1)
                .and_then(|n| n.checked_add(edges))
                .filter(|n| *n <= profile.max_work)
                .ok_or(SettingsError::Limit)?;
        }
        values.logical_bytes = m.used_bytes();
        Ok(Self {
            changes,
            values,
            profile,
        })
    }
    pub(crate) fn same(&self, other: &Self) -> Result<bool, SettingsError> {
        if self.profile != other.profile || self.changes.len() != other.changes.len() {
            return Ok(false);
        }
        let mut m = Meter::new(self.values.limits);
        for (a, b) in self.changes.iter().zip(&other.changes) {
            m.tick()?;
            if a.layer != b.layer || a.expected != b.expected || a.key != b.key {
                return Ok(false);
            }
            match (&a.operation, &b.operation) {
                (Operation::Remove, Operation::Remove) | (Operation::Reset, Operation::Reset) => {}
                (Operation::Set(a), Operation::Set(b)) => {
                    if !resolve::equal_between(&self.values, *a, &other.values, *b, &mut m, true)? {
                        return Ok(false);
                    }
                }
                _ => return Ok(false),
            }
        }
        Ok(true)
    }
}
