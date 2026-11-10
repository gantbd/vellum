use super::*;
impl SourceCoordinator {
    /// Privileged host supervision, not an ordinary recipient existence oracle.
    pub fn settings_reader_usage_v1(&self) -> SettingsReaderUsageV1 {
        match &self.settings_reader {
            State::Unconfigured => SettingsReaderUsageV1::default(),
            State::Closed { issued } => SettingsReaderUsageV1 {
                issued_lifetime: *issued,
                closed: true,
                ..SettingsReaderUsageV1::default()
            },
            State::Active(pool) => {
                let (packets, contexts) = pool.counts();
                let ended_contexts = pool.ended_contexts;
                SettingsReaderUsageV1 {
                    issued_lifetime: pool.issued,
                    packets,
                    contexts,
                    ended_contexts,
                    reserved_pool_bytes: pool.charge,
                    closed: false,
                }
            }
        }
    }
    pub fn discard_settings_read_cut_v1(
        &mut self,
        id: &SettingsReadCutIdV1,
        control: &SettingsReadControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<bool, SettingsError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            if !Arc::ptr_eq(&id.owner, &self.namespace)
                || control.owner_control_allowed != Some(true)
                || control.retention_released != Some(true)
            {
                return Err(SettingsError::Unavailable);
            }
            let pool = self.settings_reader_mut()?;
            pool.scope(control.scope)?;
            match pool.rows.get_mut(id.slot) {
                Some(row @ Some(Row::Cut(_))) => {
                    if !matches!(row, Some(Row::Cut(c)) if c.scope == *control.scope) {
                        return Err(SettingsError::Unavailable);
                    }
                    *row = None;
                    pool.packets -= 1;
                    Ok(true)
                }
                _ => Err(SettingsError::Unavailable),
            }
        })();
        self.retained_settings_result_timed(
            r,
            Boundary::SettingsReadRetirement,
            result,
            timer.finish(),
        )
    }
    pub fn stop_settings_read_v1(
        &mut self,
        id: &SettingsReadContextIdV1,
        control: &SettingsReadControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<bool, SettingsError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            if !Arc::ptr_eq(&id.owner, &self.namespace)
                || control.owner_control_allowed != Some(true)
            {
                return Err(SettingsError::Unavailable);
            }
            let pool = self.settings_reader_mut()?;
            pool.scope(control.scope)?;
            let context = pool.context(id)?;
            if context.scope != *control.scope {
                return Err(SettingsError::Unavailable);
            }
            let changed = context.data.is_some();
            pool.end(id);
            Ok(changed)
        })();
        self.retained_settings_result_timed(r, Boundary::SettingsReadStop, result, timer.finish())
    }
    pub fn retire_settings_read_v1(
        &mut self,
        id: &SettingsReadContextIdV1,
        control: &SettingsReadControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<bool, SettingsError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            if !Arc::ptr_eq(&id.owner, &self.namespace)
                || [
                    control.owner_control_allowed,
                    control.retention_released,
                    control.delivery_drained,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(SettingsError::Unavailable);
            }
            let pool = self.settings_reader_mut()?;
            if pool.decoding {
                return Err(SettingsError::Unavailable);
            }
            pool.scope(control.scope)?;
            match pool.rows.get_mut(id.slot) {
                Some(row @ Some(Row::Context(_))) => {
                    if !matches!(row, Some(Row::Context(c)) if c.scope == *control.scope && c.data.is_none())
                    {
                        return Err(SettingsError::Unavailable);
                    }
                    *row = None;
                    pool.contexts -= 1;
                    pool.ended_contexts -= 1;
                    Ok(true)
                }
                _ => Err(SettingsError::Unavailable),
            }
        })();
        self.retained_settings_result_timed(
            r,
            Boundary::SettingsReadRetirement,
            result,
            timer.finish(),
        )
    }
    /// Protected host-owner teardown, not an ordinary recipient disclosure verb.
    pub fn retire_settings_reader_v1(&mut self, r: &mut Recorder) -> Result<bool, SettingsError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let State::Active(pool) = &mut self.settings_reader else {
                return match self.settings_reader {
                    State::Closed { .. } => Ok(false),
                    _ => Err(SettingsError::Unavailable),
                };
            };
            if pool.decoding || pool.rows.iter().any(Option::is_some) {
                return Err(SettingsError::Unavailable);
            }
            let issued = pool.issued;
            self.retention
                .release(&mut pool.reservation)
                .map_err(retained::parent_error)?;
            self.settings_reader = State::Closed { issued };
            Ok(true)
        })();
        self.retained_settings_result_timed(
            r,
            Boundary::SettingsReadRetirement,
            result,
            timer.finish(),
        )
    }
}
