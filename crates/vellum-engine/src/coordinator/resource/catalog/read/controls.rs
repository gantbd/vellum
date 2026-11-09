use super::*;
impl SourceCoordinator {
    /// Host supervision only, not a recipient existence oracle.
    pub fn resource_catalog_read_usage_v1(&self) -> ResourceCatalogReadUsageV1 {
        match &self.catalog_reads {
            State::Unconfigured => ResourceCatalogReadUsageV1::default(),
            State::Closed { issued } => ResourceCatalogReadUsageV1 {
                issued_lifetime: *issued,
                closed: true,
                ..ResourceCatalogReadUsageV1::default()
            },
            State::Active(p) => ResourceCatalogReadUsageV1 {
                preparations: p.live_packets,
                contexts: p.live_contexts,
                selectors: p.live_selectors,
                issued_lifetime: p.issued,
                reserved_pool_bytes: p.charge,
                closed: false,
            },
        }
    }
    pub fn retire_resource_catalog_visibility_v1(
        &mut self,
        id: &ResourceCatalogVisibilityIdV1,
        facts: &TrustedResourceCatalogReadFactsV1,
        r: &mut Recorder,
    ) -> Result<()> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if !Arc::ptr_eq(&id.owner, &self.namespace)
                || facts.owner_control_allowed != Some(true)
                || facts.retention_released != Some(true)
            {
                return Err(ResourceError::Unavailable);
            }
            let p = self.catalog_read_pool_mut()?;
            if p.decoding {
                return Err(ResourceError::Unavailable);
            }
            let packet = p
                .packets
                .get(id.slot)
                .and_then(Option::as_ref)
                .ok_or(ResourceError::Unavailable)?;
            if !Arc::ptr_eq(&id.marker, &packet.marker) {
                return Err(ResourceError::Unavailable);
            }
            p.packets[id.slot] = None;
            p.live_packets -= 1;
            Ok(())
        })();
        record(
            r,
            Boundary::ResourceCatalogReadRetirement,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn stop_resource_catalog_read_v1(
        &mut self,
        id: &ResourceCatalogReadContextIdV1,
        scope: &ResourceCatalogReadScopeV1,
        facts: &TrustedResourceCatalogReadFactsV1,
        r: &mut Recorder,
    ) -> Result<()> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if !Arc::ptr_eq(&id.owner, &self.namespace) || facts.owner_control_allowed != Some(true)
            {
                return Err(ResourceError::Unavailable);
            }
            let p = self.catalog_read_pool_mut()?;
            p.scope(scope)?;
            let c = p
                .contexts
                .get(id.slot)
                .and_then(Option::as_ref)
                .ok_or(ResourceError::Unavailable)?;
            if c.scope != *scope {
                return Err(ResourceError::Unavailable);
            }
            p.end(id.slot, false);
            Ok(())
        })();
        record(
            r,
            Boundary::ResourceCatalogReadStop,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn retire_resource_catalog_read_v1(
        &mut self,
        id: &ResourceCatalogReadContextIdV1,
        facts: &TrustedResourceCatalogReadFactsV1,
        r: &mut Recorder,
    ) -> Result<()> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if !Arc::ptr_eq(&id.owner, &self.namespace) || !facts.discharge() {
                return Err(ResourceError::Unavailable);
            }
            let p = self.catalog_read_pool_mut()?;
            if p.decoding {
                return Err(ResourceError::Unavailable);
            }
            if !p
                .contexts
                .get(id.slot)
                .and_then(Option::as_ref)
                .is_some_and(|c| c.stopped)
            {
                return Err(ResourceError::Unavailable);
            }
            p.contexts[id.slot] = None;
            p.live_contexts -= 1;
            Ok(())
        })();
        record(
            r,
            Boundary::ResourceCatalogReadRetirement,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn retire_resource_catalog_reader_v1(
        &mut self,
        facts: &TrustedResourceCatalogReadFactsV1,
        r: &mut Recorder,
    ) -> Result<bool> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if !facts.discharge() {
                return Err(ResourceError::Unavailable);
            }
            match &mut self.catalog_reads {
                State::Unconfigured => Err(ResourceError::Unavailable),
                State::Closed { .. } => Ok(false),
                State::Active(p) => {
                    if p.decoding
                        || p.live_packets != 0
                        || p.live_contexts != 0
                        || p.live_selectors != 0
                    {
                        return Err(ResourceError::Unavailable);
                    }
                    let issued = p.issued;
                    self.retention.release(&mut p.reservation).map_err(parent)?;
                    self.catalog_reads = State::Closed { issued };
                    Ok(true)
                }
            }
        })();
        record(
            r,
            Boundary::ResourceCatalogReadRetirement,
            &result,
            timing.finish(),
        );
        result
    }
}
