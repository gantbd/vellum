//! Conditional layer dependencies, without a public generation projection.
use super::compile::{Meter, reserve};
use super::*;
use std::sync::Arc;
pub(crate) struct WriteFootprint {
    before: Arc<CompiledSettingsV1>,
    selected: usize,
    contributors: Vec<usize>,
}
impl WriteFootprint {
    pub(crate) fn capture(
        before: Arc<CompiledSettingsV1>,
        layer: &SettingIdentityV1,
        work: usize,
    ) -> Result<Self, SettingsError> {
        let mut m = Meter::new(SettingsLimitsV1 {
            max_work: work.min(before.limits.max_work),
            ..before.limits
        });
        let mut selected = None;
        for (i, row) in before.layers.iter().enumerate() {
            m.tick()?;
            if row.identity == *layer {
                selected = Some(i);
                break;
            }
        }
        let selected = selected.ok_or(SettingsError::Unavailable)?;
        let mut included = Vec::new();
        reserve(&mut included, before.layers.len())?;
        included.resize(before.layers.len(), false);
        included[selected] = true;
        for target in &before.targets {
            m.tick()?;
            m.steps(target.layers.len())?;
            if target.layers.contains(&selected) {
                for layer in &target.layers {
                    m.tick()?;
                    included[*layer] = true;
                }
            }
        }
        let mut contributors = Vec::new();
        for (i, flag) in included.into_iter().enumerate() {
            m.tick()?;
            if flag {
                reserve(&mut contributors, 1)?;
                contributors.push(i);
            }
        }
        Ok(Self {
            before,
            selected,
            contributors,
        })
    }
    pub(crate) fn retained_bytes(&self) -> Result<usize, SettingsError> {
        self.contributors
            .len()
            .checked_mul(std::mem::size_of::<usize>())
            .and_then(|n| n.checked_add(self.before.logical_bytes))
            .and_then(|n| n.checked_add(std::mem::size_of::<Self>()))
            .ok_or(SettingsError::Limit)
    }
    pub(crate) fn layer(&self) -> &SettingIdentityV1 {
        &self.before.layers[self.selected].identity
    }
    pub(crate) fn generation(&self) -> u64 {
        self.before.layers[self.selected].generation
    }
    pub(crate) fn current(
        &self,
        current: &CompiledSettingsV1,
        work: usize,
    ) -> Result<bool, SettingsError> {
        if self.before.basis.interpretation() != current.basis.interpretation()
            || self.before.limits != current.limits
        {
            return Ok(false);
        }
        let mut m = Meter::new(SettingsLimitsV1 {
            max_work: work.min(current.limits.max_work),
            ..current.limits
        });
        for index in &self.contributors {
            m.tick()?;
            let old = &self.before.layers[*index];
            let mut actual = None;
            for row in &current.layers {
                m.tick()?;
                if old.identity == row.identity {
                    if actual.is_some() {
                        return Err(SettingsError::Faulted);
                    }
                    actual = Some(row);
                }
            }
            if actual.is_none_or(|row| row.generation != old.generation) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
impl CompiledSettingsV1 {
    pub(crate) fn settings_authority(&self) -> &str {
        &self.authority
    }
}
