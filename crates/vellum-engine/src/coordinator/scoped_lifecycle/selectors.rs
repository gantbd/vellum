//! Opaque ordinary-basis selectors over the existing lifecycle participant.
use super::*;

#[derive(Debug, PartialEq, Eq)]
pub enum BasisSettingLayerReferenceV1 {
    Existing(SettingIdentityV1),
    Create { index: usize },
    Basis { index: usize },
}
pub struct BasisSettingTargetV1 {
    pub identity: SettingIdentityV1,
    pub layers: Vec<BasisSettingLayerReferenceV1>,
}
pub struct BasisSettingsLifecycleRequestV1<'a> {
    pub lifecycle: &'a SettingsLifecycleBasisId,
    pub bases: &'a [SettingsWriteBasisId],
    pub create: &'a [ScopedSettingLayerCreateV1],
    pub remove: &'a [usize],
    pub targets: &'a [BasisSettingTargetV1],
    pub dependencies: &'a ReadDependencyRequestV3,
}
#[derive(PartialEq, Eq)]
enum Form {
    Existing,
    Create(usize),
    Basis(usize),
}
pub(super) struct SelectorCapture {
    bases: Vec<SettingsWriteBasisId>,
    identities: Vec<SettingIdentityV1>,
    remove: Vec<usize>,
    forms: Vec<Form>,
}
impl SelectorCapture {
    fn bytes(&self) -> Option<usize> {
        let mut n = std::mem::size_of::<Self>()
            .checked_add(
                self.bases
                    .len()
                    .checked_mul(std::mem::size_of::<SettingsWriteBasisId>())?,
            )?
            .checked_add(
                self.identities
                    .len()
                    .checked_mul(std::mem::size_of::<SettingIdentityV1>())?,
            )?
            .checked_add(
                self.remove
                    .len()
                    .checked_mul(std::mem::size_of::<usize>())?,
            )?
            .checked_add(self.forms.len().checked_mul(std::mem::size_of::<Form>())?)?;
        for id in &self.identities {
            n = n
                .checked_add(id.authority.len())?
                .checked_add(id.id.len())?;
        }
        Some(n)
    }
}
struct Normalized {
    remove: Vec<SettingIdentityV1>,
    targets: Vec<ScopedSettingTargetV1>,
    forms: Vec<Form>,
}
impl Normalized {
    fn request<'a>(
        &'a self,
        q: &'a BasisSettingsLifecycleRequestV1<'a>,
    ) -> ScopedSettingsLifecycleRequestV1<'a> {
        ScopedSettingsLifecycleRequestV1 {
            basis: q.lifecycle,
            create: q.create,
            remove: &self.remove,
            targets: &self.targets,
            dependencies: q.dependencies,
        }
    }
}
fn normalize(
    q: &BasisSettingsLifecycleRequestV1<'_>,
    ids: &[SettingIdentityV1],
    limits: crate::settings::SettingsLimitsV1,
    p: SettingsLayerLimitsV1,
    ceiling: usize,
) -> Result<Normalized, Error> {
    if q.bases.len() > limits.max_layers
        || q.bases.len() > p.max_work
        || q.targets.len() > limits.max_targets
        || q.targets.len() > p.max_work
        || q.create
            .len()
            .checked_add(q.remove.len())
            .is_none_or(|n| n > p.max_changes)
    {
        return Err(s(SettingsError::Limit));
    }
    let count = q
        .targets
        .iter()
        .try_fold(0usize, |n, t| n.checked_add(t.layers.len()))
        .filter(|n| *n <= p.max_work)
        .ok_or(s(SettingsError::Limit))?;
    let mut m = lower::Lower::new(limits, p, ceiling);
    let mut used = m.rows::<bool>(ids.len())?;
    used.resize(ids.len(), false);
    let mut remove = m.rows::<SettingIdentityV1>(q.remove.len())?;
    for index in q.remove {
        m.step()?;
        let id = ids.get(*index).ok_or(s(SettingsError::InvalidValue))?;
        used[*index] = true;
        remove.push(m.identity(id)?);
    }
    let mut targets = m.rows::<ScopedSettingTargetV1>(q.targets.len())?;
    let mut forms = m.rows::<Form>(count)?;
    for target in q.targets {
        m.step()?;
        if target.layers.len() > limits.max_layers {
            return Err(s(SettingsError::Limit));
        }
        let identity = m.identity(&target.identity)?;
        let mut layers = m.rows::<ScopedSettingLayerReferenceV1>(target.layers.len())?;
        for reference in &target.layers {
            m.step()?;
            layers.push(match reference {
                BasisSettingLayerReferenceV1::Existing(id) => {
                    forms.push(Form::Existing);
                    ScopedSettingLayerReferenceV1::Existing(m.identity(id)?)
                }
                BasisSettingLayerReferenceV1::Create { index } => {
                    forms.push(Form::Create(*index));
                    ScopedSettingLayerReferenceV1::Create { index: *index }
                }
                BasisSettingLayerReferenceV1::Basis { index } => {
                    let id = ids.get(*index).ok_or(s(SettingsError::InvalidValue))?;
                    used[*index] = true;
                    forms.push(Form::Basis(*index));
                    ScopedSettingLayerReferenceV1::Existing(m.identity(id)?)
                }
            });
        }
        targets.push(ScopedSettingTargetV1 { identity, layers });
    }
    if used.contains(&false) {
        return Err(s(SettingsError::InvalidValue));
    }
    Ok(Normalized {
        remove,
        targets,
        forms,
    })
}
impl ScopedLifecycleCapture {
    pub(in crate::coordinator) fn basis_replay_matches(
        &self,
        q: &BasisSettingsLifecycleRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        parent: usize,
    ) -> Result<bool, Error> {
        let Some(selectors) = &self.selectors else {
            return Ok(false);
        };
        if selectors.bases != q.bases || selectors.remove != q.remove {
            return Ok(false);
        }
        let normalized = normalize(
            q,
            &selectors.identities,
            self.layers.captured_limits(),
            self.layers.limits,
            parent,
        )?;
        Ok(normalized.forms == selectors.forms
            && self.native_replay_matches(&normalized.request(q), c, parent)?)
    }
}
impl SourceCoordinator {
    fn lifecycle_selectors_current(
        &self,
        capture: &ScopedLifecycleCapture,
        c: &SettingsExecutionContextV1,
    ) -> Result<(), Error> {
        let Some(selectors) = &capture.selectors else {
            return Ok(());
        };
        for (basis, id) in selectors.bases.iter().zip(&selectors.identities) {
            self.settings_write_condition_current(basis, c).map_err(s)?;
            if self.settings_write_condition(basis, c).map_err(s)?.0 != id {
                return Err(s(SettingsError::Faulted));
            }
        }
        Ok(())
    }
    pub(super) fn validate_lifecycle_selectors(
        &mut self,
        capture: &ScopedLifecycleCapture,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        self.lifecycle_selectors_current(capture, c)?;
        let Some(selectors) = &capture.selectors else {
            return Ok(());
        };
        if selectors.bases.is_empty() {
            return Ok(());
        }
        let facts = f
            .selected_basis_facts
            .ok_or(s(SettingsError::Unavailable))?;
        if !self.basis_context_current(c, facts) {
            return Err(s(SettingsError::Unavailable));
        }
        for basis in &selectors.bases {
            let result = self.validate_settings_write_basis(basis, c, facts);
            crate::settings::observe(r, Boundary::SettingsBasisInspection, &result);
            result.map_err(s)?;
        }
        Ok(())
    }
    pub(in crate::coordinator) fn capture_basis_lifecycle(
        &self,
        q: &BasisSettingsLifecycleRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleMutationFactsV1<'_>,
    ) -> Result<Arc<ScopedLifecycleCapture>, Error> {
        self.ensure_operational().map_err(Error::Reads)?;
        if !self.scoped_lifecycle_gate(c, f) {
            return Err(s(SettingsError::Unavailable));
        }
        let root = self
            .settings
            .as_ref()
            .ok_or(s(SettingsError::Unavailable))?;
        let limits = root.limits();
        let p = f.basis.lifecycle;
        // Reject vector length before copying any selector or request payload.
        if q.bases.len() > limits.max_layers || q.bases.len() > p.max_work {
            return Err(s(SettingsError::Limit));
        }
        let mut m = lower::Lower::new(limits, p, self.settings_job_parent_limit());
        let mut ids = m.rows::<SettingIdentityV1>(q.bases.len())?;
        let mut bases = m.rows::<SettingsWriteBasisId>(q.bases.len())?;
        for basis in q.bases {
            m.step()?;
            self.settings_write_condition_current(basis, c).map_err(s)?;
            let id = self.settings_write_condition(basis, c).map_err(s)?.0;
            for prior in &ids {
                m.step()?;
                if prior == id {
                    return Err(s(SettingsError::Duplicate));
                }
            }
            ids.push(m.identity(id)?);
            bases.push(basis.clone());
        }
        let normalized = normalize(q, &ids, limits, p, self.settings_job_parent_limit())?;
        let mut remove = m.rows::<usize>(q.remove.len())?;
        remove.extend_from_slice(q.remove);
        let mut capture = self.capture_scoped_lifecycle(&normalized.request(q), c, f)?;
        let selectors = SelectorCapture {
            bases,
            identities: ids,
            remove,
            forms: normalized.forms,
        };
        let bytes = selectors.bytes().ok_or(s(SettingsError::Limit))?;
        let inner = Arc::get_mut(&mut capture).ok_or(s(SettingsError::Faulted))?;
        // Option's inline payload is already counted in ScopedLifecycleCapture.
        inner.bytes = inner
            .bytes
            .checked_add(bytes - std::mem::size_of::<SelectorCapture>())
            .ok_or(s(SettingsError::Limit))?;
        if inner
            .bytes
            .checked_add(std::mem::size_of::<Arc<ScopedLifecycleCapture>>())
            .is_none_or(|n| n > self.settings_job_parent_limit() || n > p.max_request_bytes)
        {
            return Err(s(SettingsError::Limit));
        }
        inner.selectors = Some(selectors);
        Ok(capture)
    }
    pub fn prepare_basis_settings_lifecycle(
        &mut self,
        q: &BasisSettingsLifecycleRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedSettingsLifecycleMutationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<PreparedScopedSettingsLifecycleV1, Error> {
        let capture = self.capture_basis_lifecycle(q, c, f);
        observe(r, Boundary::ScopedLifecycleLowering, &capture);
        let result =
            capture.and_then(|capture| self.prepare_scoped_lifecycle_capture(capture, c, f, r));
        if result.as_ref().err().is_some_and(|e| e.faulted()) {
            self.faulted = true;
        }
        observe(r, Boundary::ScopedLifecyclePreparation, &result);
        result
    }
}
