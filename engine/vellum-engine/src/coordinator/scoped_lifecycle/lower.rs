//! Bounded temporary normalization. No recursive value copy or live-state replay.
use super::*;
pub(super) struct Lower {
    bytes: usize,
    ceiling: usize,
    work: usize,
    ids: usize,
}
impl Lower {
    pub(super) fn new(
        limits: crate::settings::SettingsLimitsV1,
        p: SettingsLayerLimitsV1,
        ceiling: usize,
    ) -> Self {
        Self {
            bytes: 0,
            ceiling: ceiling.min(p.max_request_bytes),
            work: limits.max_work.min(p.max_work),
            ids: limits.max_identifier_bytes,
        }
    }
    pub(super) fn step(&mut self) -> Result<(), Error> {
        self.work = self.work.checked_sub(1).ok_or(s(SettingsError::Limit))?;
        Ok(())
    }
    fn charge(&mut self, n: usize) -> Result<(), Error> {
        self.bytes = self
            .bytes
            .checked_add(n)
            .filter(|n| *n <= self.ceiling)
            .ok_or(s(SettingsError::Limit))?;
        Ok(())
    }
    pub(super) fn rows<T>(&mut self, n: usize) -> Result<Vec<T>, Error> {
        self.charge(
            n.checked_mul(std::mem::size_of::<T>())
                .ok_or(s(SettingsError::Limit))?,
        )?;
        let mut out = Vec::new();
        out.try_reserve_exact(n)
            .map_err(|_| s(SettingsError::Capacity))?;
        Ok(out)
    }
    fn label(&mut self, value: &str) -> Result<String, Error> {
        self.step()?;
        if value.is_empty() || value.len() > self.ids {
            return Err(s(SettingsError::InvalidIdentity));
        }
        self.charge(value.len())?;
        let mut out = String::new();
        out.try_reserve_exact(value.len())
            .map_err(|_| s(SettingsError::Capacity))?;
        out.push_str(value);
        Ok(out)
    }
    pub(super) fn identity(
        &mut self,
        value: &SettingIdentityV1,
    ) -> Result<SettingIdentityV1, Error> {
        Ok(SettingIdentityV1 {
            authority: self.label(&value.authority)?,
            id: self.label(&value.id)?,
            incarnation: value.incarnation,
        })
    }
    fn targets(
        &mut self,
        request: &ScopedSettingsLifecycleRequestV1<'_>,
        created: &[&SettingIdentityV1],
        limits: crate::settings::SettingsLimitsV1,
    ) -> Result<(Vec<SettingTargetV1>, Vec<Option<usize>>), Error> {
        if request.targets.len() > limits.max_targets {
            return Err(s(SettingsError::Limit));
        }
        let mut out = self.rows::<SettingTargetV1>(request.targets.len())?;
        let count = request
            .targets
            .iter()
            .try_fold(0usize, |n, t| n.checked_add(t.layers.len()))
            .ok_or(s(SettingsError::Limit))?;
        let mut recipe = self.rows::<Option<usize>>(count)?;
        for row in request.targets {
            self.step()?;
            if row.layers.len() > limits.max_layers {
                return Err(s(SettingsError::Limit));
            }
            let identity = self.identity(&row.identity)?;
            let mut layers = self.rows::<SettingIdentityV1>(row.layers.len())?;
            for reference in &row.layers {
                self.step()?;
                let id = match reference {
                    ScopedSettingLayerReferenceV1::Existing(id) => {
                        recipe.push(None);
                        id
                    }
                    ScopedSettingLayerReferenceV1::Create { index } => {
                        recipe.push(Some(*index));
                        created
                            .get(*index)
                            .copied()
                            .ok_or(s(SettingsError::InvalidIdentity))?
                    }
                };
                layers.push(self.identity(id)?);
            }
            out.push(SettingTargetV1 { identity, layers });
        }
        Ok((out, recipe))
    }
}
pub(super) fn initial(
    request: &ScopedSettingsLifecycleRequestV1<'_>,
    state: &crate::settings::CompiledSettingsV1,
    expected: SettingsLayerBasisV1,
    p: SettingsLayerLimitsV1,
    ceiling: usize,
) -> Result<(LayerCapture, Vec<Option<usize>>), Error> {
    let limits = state.limits();
    counts(request, limits, p)?;
    let current = state.layer_basis();
    current.validate().map_err(|_| s(SettingsError::Faulted))?;
    if current != expected {
        return Err(s(SettingsError::Stale));
    }
    let mut m = Lower::new(limits, p, ceiling);
    let mut ids = m.rows::<SettingIdentityV1>(request.create.len())?;
    for (i, row) in request.create.iter().enumerate() {
        let incarnation = expected
            .layer_incarnation
            .checked_add(u64::try_from(i).map_err(|_| s(SettingsError::VersionExhausted))?)
            .and_then(|n| n.checked_add(1))
            .filter(|n| *n <= 9_007_199_254_740_991)
            .ok_or(s(SettingsError::VersionExhausted))?;
        ids.push(SettingIdentityV1 {
            authority: m.label(state.settings_authority())?,
            id: m.label(&row.id)?,
            incarnation,
        });
    }
    let mut create = m.rows::<LayerCreateInput<'_>>(ids.len())?;
    for (row, identity) in request.create.iter().zip(&ids) {
        m.step()?;
        create.push(LayerCreateInput {
            identity,
            scope: row.scope,
            values: &row.values,
        });
    }
    let mut remove = m.rows::<SettingLayerRemoveV1>(request.remove.len())?;
    for identity in request.remove {
        m.step()?;
        remove.push(SettingLayerRemoveV1 {
            identity: m.identity(identity)?,
            expected_generation: state.layer_generation(identity).map_err(s)?,
        });
    }
    let mut refs = m.rows::<&SettingIdentityV1>(ids.len())?;
    refs.extend(ids.iter());
    let (targets, recipe) = m.targets(request, &refs, limits)?;
    let capture =
        LayerCapture::copy_borrowed(&create, &remove, &targets, state, p, ceiling).map_err(s)?;
    Ok((capture, recipe))
}
fn counts(
    request: &ScopedSettingsLifecycleRequestV1<'_>,
    limits: crate::settings::SettingsLimitsV1,
    p: SettingsLayerLimitsV1,
) -> Result<(), Error> {
    if request
        .create
        .len()
        .checked_add(request.remove.len())
        .is_none_or(|n| n > p.max_changes)
        || request.create.len() > limits.max_layers
        || request.remove.len() > limits.max_layers
        || request.targets.len() > limits.max_targets
        || request.targets.len() > p.max_work
    {
        return Err(s(SettingsError::Limit));
    }
    Ok(())
}
pub(super) fn replay(
    original: &ScopedLifecycleCapture,
    request: &ScopedSettingsLifecycleRequestV1<'_>,
    ceiling: usize,
) -> Result<bool, Error> {
    let native = &original.layers;
    let limits = native.captured_limits();
    counts(request, limits, native.limits)?;
    if request.create.len() != native.created_identities().len()
        || request.remove.len() != native.removals().len()
        || request
            .create
            .iter()
            .zip(native.created_identities())
            .any(|(r, id)| r.id != id.id)
        || request
            .remove
            .iter()
            .zip(native.removals())
            .any(|(r, row)| *r != row.identity)
    {
        return Ok(false);
    }
    let mut m = Lower::new(limits, native.limits, ceiling);
    let mut ids = m.rows::<&SettingIdentityV1>(request.create.len())?;
    ids.extend(native.created_identities());
    let mut create = m.rows::<LayerCreateInput<'_>>(ids.len())?;
    for (row, identity) in request.create.iter().zip(&ids) {
        m.step()?;
        create.push(LayerCreateInput {
            identity,
            scope: row.scope,
            values: &row.values,
        });
    }
    let (targets, recipe) = m.targets(request, &ids, limits)?;
    if recipe != original.recipe {
        return Ok(false);
    }
    let copy = native
        .recopy_borrowed(&create, &targets, ceiling)
        .map_err(s)?;
    native.same(&copy).map_err(s)
}
