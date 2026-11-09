use super::*;

struct Work(usize);
impl Work {
    fn step(&mut self, n: usize) -> Result<(), SettingsError> {
        self.0 = self.0.checked_sub(n).ok_or(SettingsError::Limit)?;
        Ok(())
    }
    fn same_origin(
        &mut self,
        a: &SettingProjectionOriginV1,
        b: &SettingProjectionOriginV1,
    ) -> Result<bool, SettingsError> {
        let bytes = |id: &SettingProjectionOriginV1| match id {
            SettingProjectionOriginV1::Library => 1,
            SettingProjectionOriginV1::Layer(id) => id
                .authority
                .len()
                .saturating_add(id.id.len())
                .saturating_add(1),
        };
        self.step(bytes(a).checked_add(bytes(b)).ok_or(SettingsError::Limit)?)?;
        Ok(a == b)
    }
}
fn visit_origins(
    data: &NativeSettingsProjectionV1,
    mut f: impl FnMut(&SettingProjectionOriginV1) -> Result<(), SettingsError>,
) -> Result<(), SettingsError> {
    for row in &data.rows {
        match &row.origins {
            Some(SettingProjectionOriginsV1::Whole(ids)) => {
                for id in ids {
                    f(id)?;
                }
            }
            Some(SettingProjectionOriginsV1::Fields(fields)) => {
                for (_, ids) in fields {
                    for id in ids {
                        f(id)?;
                    }
                }
            }
            None => {}
        }
    }
    Ok(())
}
pub(super) fn capture_origins(
    data: &NativeSettingsProjectionV1,
    limits: SettingsReaderLimitsV1,
) -> Result<Vec<SettingProjectionOriginV1>, SettingsError> {
    let mut out = Vec::new();
    let mut work = Work(limits.max_work);
    let mut bytes = 0usize;
    visit_origins(data, |id| {
        for old in &out {
            if work.same_origin(id, old)? {
                return Ok(());
            }
        }
        if out.len() >= limits.max_origins {
            return Err(SettingsError::Limit);
        }
        let extra = match id {
            SettingProjectionOriginV1::Library => 0,
            SettingProjectionOriginV1::Layer(id) => id
                .authority
                .len()
                .checked_add(id.id.len())
                .ok_or(SettingsError::Limit)?,
        };
        bytes = bytes
            .checked_add(size_of::<SettingProjectionOriginV1>())
            .and_then(|n| n.checked_add(extra))
            .ok_or(SettingsError::Limit)?;
        if bytes > limits.max_capture_bytes {
            return Err(SettingsError::Limit);
        }
        reserve(&mut out, 1)?;
        out.push(match id {
            SettingProjectionOriginV1::Library => SettingProjectionOriginV1::Library,
            SettingProjectionOriginV1::Layer(id) => {
                SettingProjectionOriginV1::Layer(SettingIdentityV1 {
                    authority: string(&id.authority)?,
                    id: string(&id.id)?,
                    incarnation: id.incarnation,
                })
            }
        });
        Ok(())
    })?;
    Ok(out)
}
fn old_contains(
    old: &SettingProjectionOriginsV1,
    id: &SettingProjectionOriginV1,
    work: &mut Work,
) -> Result<bool, SettingsError> {
    match old {
        SettingProjectionOriginsV1::Whole(ids) => {
            for item in ids {
                if work.same_origin(item, id)? {
                    return Ok(true);
                }
            }
        }
        SettingProjectionOriginsV1::Fields(fields) => {
            for (_, ids) in fields {
                work.step(1)?;
                for item in ids {
                    if work.same_origin(item, id)? {
                        return Ok(true);
                    }
                }
            }
        }
    }
    Ok(false)
}
fn narrow_origins(
    current: &mut SettingProjectionOriginsV1,
    old: &SettingProjectionOriginsV1,
    work: &mut Work,
) -> Result<(), SettingsError> {
    let mut filter = |ids: &mut Vec<SettingProjectionOriginV1>| -> Result<(), SettingsError> {
        for i in (0..ids.len()).rev() {
            if !old_contains(old, &ids[i], work)? {
                work.step(ids.len() - i)?;
                ids.remove(i);
            }
        }
        Ok(())
    };
    match current {
        SettingProjectionOriginsV1::Whole(ids) => filter(ids),
        SettingProjectionOriginsV1::Fields(fields) => {
            for (_, ids) in fields {
                filter(ids)?;
            }
            Ok(())
        }
    }
}
// Compare copied semantic payloads, not native indices, generations, map order
// or logical construction charges. The ceiling retains captured schema order.
fn narrow(
    current: &mut NativeSettingsProjectionV1,
    old: &NativeSettingsProjectionV1,
    work: &mut Work,
) -> Result<bool, SettingsError> {
    work.step(
        current
            .logical_bytes
            .checked_add(old.logical_bytes)
            .ok_or(SettingsError::Limit)?,
    )?;
    if old.capabilities && !current.capabilities {
        return Ok(false);
    }
    current.capabilities = old.capabilities;
    for i in (0..current.rows.len()).rev() {
        let row = &mut current.rows[i];
        let mut found = None;
        for prior in &old.rows {
            work.step(
                prior
                    .key
                    .len()
                    .checked_add(row.key.len())
                    .ok_or(SettingsError::Limit)?,
            )?;
            if prior.key == row.key {
                found = Some(prior);
                break;
            }
        }
        let Some(prior) = found else {
            work.step(current.rows.len() - i)?;
            current.rows.remove(i);
            continue;
        };
        if prior.value.is_some() && row.value.is_none()
            || prior.declaration.is_some() && row.declaration.is_none()
            || prior.origins.is_some() && row.origins.is_none()
        {
            return Ok(false);
        }
        if prior.value.is_none() {
            row.value = None;
        }
        if prior.declaration.is_none() {
            row.declaration = None;
        }
        if let Some(old) = &prior.origins {
            narrow_origins(
                row.origins.as_mut().ok_or(SettingsError::Faulted)?,
                old,
                work,
            )?;
        } else {
            row.origins = None;
        }
    }
    for (at, prior) in old.rows.iter().enumerate() {
        let mut found = None;
        for (i, row) in current.rows.iter().enumerate().skip(at) {
            work.step(
                row.key
                    .len()
                    .checked_add(prior.key.len())
                    .ok_or(SettingsError::Limit)?,
            )?;
            if row.key == prior.key {
                found = Some(i);
                break;
            }
        }
        let Some(i) = found else {
            return Ok(false);
        };
        // Rotation preserves the relative order of the other captured rows.
        work.step(i - at + 1)?;
        current.rows[at..=i].rotate_right(1);
    }
    current.same_recipient_payload(old, &mut work.0, current.logical_bytes)
}
fn origin_refs(
    ids: Vec<SettingProjectionOriginV1>,
    context: &Context,
    work: &mut Work,
) -> Result<Vec<SettingsReadOriginRefV1>, SettingsError> {
    let mut out = Vec::new();
    reserve(&mut out, ids.len())?;
    for id in ids {
        let mut slot = None;
        for (i, prior) in context.origins.iter().enumerate() {
            if work.same_origin(&id, prior)? {
                slot = Some(i);
                break;
            }
        }
        out.push(SettingsReadOriginRefV1 {
            owner: context.marker.clone(),
            slot: slot.ok_or(SettingsError::Faulted)?,
        });
    }
    Ok(out)
}
impl SourceCoordinator {
    #[allow(clippy::too_many_arguments)]
    pub fn query_settings_read_v1(
        &mut self,
        context: &SettingsReadContextIdV1,
        target: &SettingsReadTargetRefV1,
        scope: &SettingsReadScopeV1,
        operation: SettingsReadOperationV1<'_>,
        cut: &SettingsReadCutIdV1,
        facts: &TrustedSettingsReadFactsV1,
        r: &mut Recorder,
    ) -> Result<SettingsReadResultV1, SettingsError> {
        let timer = r.begin_service_timing();
        let result = self.release_settings_read(context, target, scope, operation, cut, facts);
        self.retained_settings_result_timed(
            r,
            Boundary::SettingsReadRelease,
            result,
            timer.finish(),
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn release_settings_read(
        &mut self,
        id: &SettingsReadContextIdV1,
        target: &SettingsReadTargetRefV1,
        scope: &SettingsReadScopeV1,
        operation: SettingsReadOperationV1<'_>,
        cut: &SettingsReadCutIdV1,
        facts: &TrustedSettingsReadFactsV1,
    ) -> Result<SettingsReadResultV1, SettingsError> {
        self.ensure_operational()
            .map_err(|_| SettingsError::Faulted)?;
        if facts.outside_publication != Some(true) || !Arc::ptr_eq(&id.owner, &self.namespace) {
            return Err(SettingsError::Unavailable);
        }
        let pool = self.settings_reader_pool()?;
        pool.delivery_ready()?;
        pool.scope(scope)?;
        let context = pool.context(id)?;
        if context.scope != *scope
            || target.slot != 0
            || !Arc::ptr_eq(&target.owner, &context.marker)
            || context.data.is_none()
        {
            return Err(SettingsError::Unavailable);
        }
        if let SettingsReadOperationV1::EffectiveConfiguration { key }
        | SettingsReadOperationV1::PermittedOrigin { key } = operation
            && (key.is_empty() || key.len() > pool.limits.max_label_bytes)
        {
            return Err(SettingsError::Unavailable);
        }
        if facts.loss() {
            self.settings_reader_mut()?.end(id);
            return Err(SettingsError::Unavailable);
        }
        if !facts.ready() {
            return Err(SettingsError::Unavailable);
        }
        let root = self.settings.as_ref().ok_or(SettingsError::Unavailable)?;
        match self.owned_settings_target(root, &context.target) {
            Ok(_) => {}
            Err(SettingsError::Unavailable) => {
                self.settings_reader_mut()?.end(id);
                return Err(SettingsError::Unavailable);
            }
            Err(error) => return Err(error),
        }
        let limits = pool.limits;
        // Foreign cuts are rejected without consuming or invalidating the context.
        if !Arc::ptr_eq(&cut.owner, &self.namespace) {
            return Err(SettingsError::Unavailable);
        }
        let Some(Some(Row::Cut(prepared))) = pool.rows.get(cut.slot) else {
            return Err(SettingsError::Unavailable);
        };
        if prepared.target != context.target {
            return Err(SettingsError::Unavailable);
        }
        let mut current = self.take_settings_read_cut(cut, scope)?.data;
        let pool = self.settings_reader_pool()?;
        let context = pool.context(id)?;
        let old = context.data.as_ref().ok_or(SettingsError::Unavailable)?;
        let mut work = Work(limits.max_work);
        if !narrow(&mut current, old, &mut work)? {
            self.settings_reader_mut()?.end(id);
            return Err(SettingsError::Unavailable);
        }
        // Conservative full-projection reply bound; never enlarge a captured ceiling.
        if current
            .logical_bytes
            .checked_add(size_of::<SettingsReadResultV1>())
            .is_none_or(|n| n > limits.max_reply_bytes)
        {
            return Err(SettingsError::Limit);
        }
        match operation {
            SettingsReadOperationV1::Capabilities => {
                if !current.capabilities {
                    return Err(SettingsError::Unavailable);
                }
                let mut out = Vec::new();
                reserve(&mut out, current.rows.len())?;
                for row in current.rows {
                    if let Some(d) = row.declaration {
                        out.push(d);
                    }
                }
                Ok(SettingsReadResultV1::Capabilities(out))
            }
            SettingsReadOperationV1::EffectiveConfiguration { key } => {
                for row in current.rows {
                    if row.key == key {
                        return row
                            .value
                            .map(SettingsReadResultV1::Value)
                            .ok_or(SettingsError::Unavailable);
                    }
                }
                Err(SettingsError::Unavailable)
            }
            SettingsReadOperationV1::PermittedOrigin { key } => {
                let origins = current
                    .rows
                    .into_iter()
                    .find(|row| row.key == key)
                    .and_then(|row| row.origins)
                    .ok_or(SettingsError::Unavailable)?;
                let out = match origins {
                    SettingProjectionOriginsV1::Whole(ids) => {
                        SettingsReadOriginsV1::Whole(origin_refs(ids, context, &mut work)?)
                    }
                    SettingProjectionOriginsV1::Fields(fields) => {
                        let mut out = Vec::new();
                        reserve(&mut out, fields.len())?;
                        for (key, ids) in fields {
                            out.push((key, origin_refs(ids, context, &mut work)?));
                        }
                        SettingsReadOriginsV1::Fields(out)
                    }
                };
                Ok(SettingsReadResultV1::Origins(out))
            }
        }
    }
}
