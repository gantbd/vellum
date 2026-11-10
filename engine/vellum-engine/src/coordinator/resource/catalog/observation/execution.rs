use super::*;
impl SourceCoordinator {
    /// First-party installed-source check, not recipient authority or inventory.
    pub(in crate::coordinator) fn registry_observation_source_v1(
        &self,
        authority: &str,
        engine: &str,
    ) -> Result<()> {
        let p = self.registry_observation_pool()?;
        self.registry_command_ordinary_v1()?;
        if p.authority != authority || p.engine != engine {
            return Err(ResourceError::Unavailable);
        }
        Ok(())
    }
    pub(super) fn registry_observation_pool(&self) -> Result<&Pool> {
        let State::Active(p) = &self.registry_observations else {
            return Err(ResourceError::Unavailable);
        };
        self.retention
            .validate_exact(&p.reservation, p.charge)
            .map_err(parent)?;
        if p.streams
            .len()
            .checked_add(p.packets.len())
            .is_none_or(|n| n > p.issued)
            || p.issued > p.limits.max_issued
            || p.streams.iter().flatten().count() > p.limits.max_streams
            || p.packets.iter().flatten().count() > p.limits.max_preparations
        {
            return Err(ResourceError::Faulted);
        }
        Ok(p)
    }
    pub(super) fn registry_observer_control(
        &self,
        id: &ResourceRegistryStreamIdV1,
    ) -> Result<&Observer> {
        let p = self.registry_observation_pool()?;
        if !Arc::ptr_eq(&p.owner, &id.owner) {
            return Err(ResourceError::Unavailable);
        }
        p.streams
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(ResourceError::Unavailable)
    }
    pub(super) fn registry_observer(&self, id: &ResourceRegistryStreamIdV1) -> Result<&Observer> {
        let o = self.registry_observer_control(id)?;
        let p = self.registry_observation_pool()?;
        if !projection::valid(o, id, p.limits, p.issued) {
            return Err(ResourceError::Faulted);
        }
        Ok(o)
    }
    pub(super) fn registry_observer_mut(
        &mut self,
        id: &ResourceRegistryStreamIdV1,
    ) -> &mut Observer {
        let State::Active(p) = &mut self.registry_observations else {
            unreachable!()
        };
        p.streams[id.slot]
            .as_mut()
            .expect("validated registry observer")
    }
    fn registry_gap(&mut self, id: &ResourceRegistryStreamIdV1, r: &mut Recorder) {
        self.registry_observer_mut(id).close(Phase::Gap);
        r.record(Observation {
            boundary: Boundary::RegistryObservationGap,
            disposition: Disposition::Completed,
            service_time: None,
        });
    }
    fn registry_gate(
        &mut self,
        id: &ResourceRegistryStreamIdV1,
        claimed: &ResourceRegistryObservationScopeV1,
        f: &TrustedResourceRegistryObservationFactsV1<'_>,
    ) -> Result<()> {
        self.ensure_operational().map_err(owner_error)?;
        self.registry_command_ordinary_v1()?;
        if self.source_facade_busy {
            return Err(ResourceError::Unavailable);
        }
        let o = self.registry_observer_control(id)?;
        if o.scope != *claimed || o.scope != *f.scope {
            return Err(ResourceError::Unavailable);
        }
        let o = self.registry_observer(id)?;
        if o.phase == Phase::Stopped {
            return Err(ResourceError::Unavailable);
        }
        let p = self.registry_observation_pool()?;
        if f.required().contains(&Some(false)) || f.native_profile != p.native {
            self.registry_observer_mut(id).close(Phase::Stopped);
            return Err(ResourceError::Unavailable);
        }
        if f.required().into_iter().any(|v| v != Some(true)) {
            return Err(ResourceError::Unavailable);
        }
        let (_, native) = self.resources.catalog.current()?;
        if native != p.native {
            self.registry_observer_mut(id).close(Phase::Stopped);
            return Err(ResourceError::Unavailable);
        }
        Ok(())
    }
    fn registry_packet(
        &self,
        id: &ResourceRegistryPreparationIdV1,
        stream: &ResourceRegistryStreamIdV1,
        scope: &ResourceRegistryObservationScopeV1,
    ) -> Result<&Packet> {
        let p = self.registry_observation_pool()?;
        if !Arc::ptr_eq(&p.owner, &id.owner) {
            return Err(ResourceError::Unavailable);
        }
        let packet = p
            .packets
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(ResourceError::Unavailable)?;
        if packet.stream != PacketTarget::Stream(stream.clone())
            || packet.scope != *scope
            || packet.root != self.resources.catalog.stamp()?
        {
            return Err(ResourceError::Unavailable);
        }
        projection::packet_bytes(packet, p.limits)?;
        Ok(packet)
    }
    pub(super) fn registry_take_packet(&mut self, id: &ResourceRegistryPreparationIdV1) -> Packet {
        let State::Active(p) = &mut self.registry_observations else {
            unreachable!()
        };
        p.packets[id.slot]
            .take()
            .expect("validated registry packet")
    }
    fn registry_snapshot_original(
        &self,
        packet: &ResourceRegistryPreparationIdV1,
        id: &ResourceRegistryStreamIdV1,
        s: &ResourceRegistryObservationScopeV1,
    ) -> Result<()> {
        if self.registry_observer_control(id)?.scope != *s {
            return Err(ResourceError::Unavailable);
        }
        if !matches!(
            self.registry_packet(packet, id, s)?.value,
            PacketValue::Snapshot(_)
        ) {
            return Err(ResourceError::Unavailable);
        }
        Ok(())
    }
    fn registry_head_original(
        &self,
        id: &ResourceRegistryStreamIdV1,
        s: &ResourceRegistryObservationScopeV1,
        marker: &Arc<()>,
    ) -> Result<()> {
        let o = self.registry_observer(id)?;
        if o.scope != *s
            || o.queue
                .front()
                .is_none_or(|q| !Arc::ptr_eq(&q.marker, marker))
        {
            return Err(ResourceError::Unavailable);
        }
        Ok(())
    }
    fn registry_certificate_original(
        &self,
        packet: &ResourceRegistryPreparationIdV1,
        id: &ResourceRegistryStreamIdV1,
        s: &ResourceRegistryObservationScopeV1,
    ) -> Result<()> {
        let p = self.registry_packet(packet, id, s)?;
        let PacketValue::Historical { head, .. } = &p.value else {
            return Err(ResourceError::Unavailable);
        };
        self.registry_head_original(id, s, head)
    }
    pub(super) fn registry_issue_packet(
        &mut self,
        packet: Packet,
    ) -> Result<ResourceRegistryPreparationIdV1> {
        let p = self.registry_observation_pool()?;
        projection::packet_bytes(&packet, p.limits)?;
        if p.issued >= p.limits.max_issued
            || p.packets.iter().flatten().count() >= p.limits.max_preparations
        {
            return Err(ResourceError::Limit);
        }
        let State::Active(p) = &mut self.registry_observations else {
            unreachable!()
        };
        let id = ResourceRegistryPreparationIdV1 {
            owner: p.owner.clone(),
            slot: p.packets.len(),
        };
        p.packets.push(Some(packet));
        p.issued += 1;
        Ok(id)
    }
    pub fn configure_resource_registry_observation_v1(
        &mut self,
        authority: &str,
        engine: &str,
        l: ResourceRegistryObservationLimitsV1,
        c: &SourceReadPoolControlV1,
        r: &mut Recorder,
    ) -> Result<()> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(owner_error)?;
            if c.owner_control_allowed != Some(true)
                || c.resources_ready != Some(true)
                || !matches!(self.registry_observations, State::Unconfigured)
                || authority.is_empty()
                || engine.is_empty()
                || authority.len() > l.max_tag_bytes
                || engine.len() > l.max_tag_bytes
                || l.max_queue == 0
            {
                return Err(ResourceError::Unavailable);
            }
            let (_, native) = self.resources.catalog.current()?;
            if add(
                add(size_of::<ResourceCatalogLimitsV1>(), authority.len())?,
                engine.len(),
            )? > l.max_profile_bytes
            {
                return Err(ResourceError::Limit);
            }
            let charge = l.required_pool_bytes()?;
            self.retention.check(charge).map_err(parent)?;
            let authority = text(authority)?;
            let engine = text(engine)?;
            let mut streams = Vec::new();
            let mut packets = Vec::new();
            streams
                .try_reserve_exact(l.max_issued)
                .map_err(|_| ResourceError::Capacity)?;
            packets
                .try_reserve_exact(l.max_issued)
                .map_err(|_| ResourceError::Capacity)?;
            let reservation = self.retention.reserve(charge).map_err(parent)?;
            self.registry_observations = State::Active(Box::new(Pool {
                owner: Arc::new(()),
                authority,
                engine,
                native,
                limits: l,
                streams,
                packets,
                issued: 0,
                reservation,
                charge,
                #[cfg(test)]
                command_fault: None,
            }));
            Ok(())
        })();
        report(
            r,
            Boundary::RegistryObservationConfiguration,
            &result,
            timer.finish(),
        );
        result
    }
    pub(super) fn registry_open_rows_v1(
        &mut self,
        s: &ResourceRegistryObservationScopeV1,
        rows: &[SnapshotRow],
        f: &TrustedResourceRegistryObservationFactsV1<'_>,
    ) -> Result<ResourceRegistryBaselineV1> {
        #[cfg(test)]
        {
            let State::Active(p) = &mut self.registry_observations else {
                unreachable!()
            };
            if p.command_fault == Some(commands::FaultCut::NativeRefusal) {
                p.command_fault = None;
                return Err(ResourceError::Unavailable);
            }
        }
        self.ensure_ordinary_admission().map_err(owner_error)?;
        self.registry_command_ordinary_v1()?;
        if self.source_facade_busy
            || f.scope != s
            || f.required().into_iter().any(|v| v != Some(true))
            || !f.ready()
        {
            return Err(ResourceError::Unavailable);
        }
        let p = self.registry_observation_pool()?;
        let l = p.limits;
        projection::scope_bytes(s, l)?;
        if s.context.authority != p.authority
            || s.context.engine != p.engine
            || f.native_profile != p.native
        {
            return Err(ResourceError::Unavailable);
        }
        if self.resources.catalog.current()?.1 != p.native {
            return Err(ResourceError::Unavailable);
        }
        if p.streams.iter().flatten().count() >= l.max_streams
            || p.packets.iter().flatten().count() >= l.max_preparations
            || p.issued >= l.max_issued
        {
            return Err(ResourceError::Limit);
        }
        let id = ResourceRegistryStreamIdV1 {
            owner: p.owner.clone(),
            slot: p.streams.len(),
        };
        let stage = projection::stage(&id, rows, &[], l)?;
        if p.issued
            .checked_add(1)
            .and_then(|n| n.checked_add(stage.added))
            .is_none_or(|n| n > l.max_issued)
        {
            return Err(ResourceError::Limit);
        }
        let scope = projection::copy_scope(s)?;
        let mut queue = VecDeque::new();
        queue
            .try_reserve_exact(l.max_queue)
            .map_err(|_| ResourceError::Capacity)?;
        let generation = ResourceRegistryGenerationV1 {
            stream: id.clone(),
            serial: 0,
        };
        let projection = stage.projection;
        let State::Active(p) = &mut self.registry_observations else {
            unreachable!()
        };
        p.issued += 1 + stage.added;
        p.streams.push(Some(Observer {
            scope,
            generation: generation.clone(),
            baseline: Some(projection.clone()),
            bindings: stage.bindings,
            queue,
            phase: Phase::Active,
        }));
        Ok(ResourceRegistryBaselineV1 {
            stream: id,
            generation,
            projection,
        })
    }
    /// Raw native establishment. Does not implement original-command retry/replay.
    pub fn open_resource_registry_observation_v1(
        &mut self,
        s: &ResourceRegistryObservationScopeV1,
        f: &TrustedResourceRegistrySnapshotFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResourceRegistryBaselineV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(owner_error)?;
            self.registry_command_ordinary_v1()?;
            if self.source_facade_busy
                || f.current.scope != s
                || f.current.required().into_iter().any(|v| v != Some(true))
                || !f.current.ready()
            {
                return Err(ResourceError::Unavailable);
            }
            let p = self.registry_observation_pool()?;
            let l = p.limits;
            projection::scope_bytes(s, l)?;
            if s.context.authority != p.authority
                || s.context.engine != p.engine
                || f.current.native_profile != p.native
            {
                return Err(ResourceError::Unavailable);
            }
            if p.streams.iter().flatten().count() >= l.max_streams
                || p.packets.iter().flatten().count() >= l.max_preparations
                || p.issued >= l.max_issued
            {
                return Err(ResourceError::Limit);
            }
            let (root, native) = self.resources.catalog.current()?;
            if native != p.native {
                return Err(ResourceError::Unavailable);
            }
            let rows = projection::snapshot(root, s, f, l, native)?;
            self.registry_open_rows_v1(s, &rows, f.current)
        })();
        report(
            r,
            Boundary::RegistryObservationOpen,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn prepare_resource_registry_snapshot_v1(
        &mut self,
        id: &ResourceRegistryStreamIdV1,
        s: &ResourceRegistryObservationScopeV1,
        f: &TrustedResourceRegistrySnapshotFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResourceRegistryPreparationIdV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.registry_gate(id, s, f.current)?;
            if f.current.resources_ready != Some(true) {
                return Err(ResourceError::Unavailable);
            }
            let p = self.registry_observation_pool()?;
            let l = p.limits;
            if p.issued >= l.max_issued || p.packets.iter().flatten().count() >= l.max_preparations
            {
                return Err(ResourceError::Limit);
            }
            let (root, native) = self.resources.catalog.current()?;
            let rows = projection::snapshot(root, s, f, l, native)?;
            self.registry_issue_packet(Packet {
                stream: PacketTarget::Stream(id.clone()),
                scope: projection::copy_scope(s)?,
                root: self.resources.catalog.stamp()?,
                command_basis: None,
                value: PacketValue::Snapshot(rows),
            })
        })();
        report(
            r,
            Boundary::RegistryObservationSnapshotPreparation,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn sample_resource_registry_observation_v1(
        &mut self,
        id: &ResourceRegistryStreamIdV1,
        s: &ResourceRegistryObservationScopeV1,
        packet: &ResourceRegistryPreparationIdV1,
        f: &TrustedResourceRegistryObservationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResourceRegistrySampleV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.registry_snapshot_original(packet, id, s)?;
            self.registry_gate(id, s, f)?;
            if self.registry_observer(id)?.phase == Phase::Gap {
                return Ok(ResourceRegistrySampleV1::Gap);
            }
            if f.resources_ready != Some(true) {
                self.registry_gap(id, r);
                return Ok(ResourceRegistrySampleV1::Gap);
            }
            if !matches!(
                self.registry_packet(packet, id, s)?.value,
                PacketValue::Snapshot(_)
            ) {
                return Err(ResourceError::Unavailable);
            }
            let packet = self.registry_take_packet(packet);
            let PacketValue::Snapshot(rows) = packet.value else {
                unreachable!()
            };
            let o = self.registry_observer(id)?;
            if projection::matches(&rows, o) {
                return Ok(ResourceRegistrySampleV1::Unchanged);
            }
            let p = self.registry_observation_pool()?;
            let l = p.limits;
            if o.queue.len() >= l.max_queue {
                self.registry_gap(id, r);
                return Ok(ResourceRegistrySampleV1::Gap);
            }
            let stage = match projection::stage(id, &rows, &o.bindings, l) {
                Ok(v) => v,
                Err(ResourceError::Limit | ResourceError::Capacity) => {
                    self.registry_gap(id, r);
                    return Ok(ResourceRegistrySampleV1::Gap);
                }
                Err(e) => return Err(e),
            };
            if p.issued
                .checked_add(1)
                .and_then(|n| n.checked_add(stage.added))
                .is_none_or(|n| n > l.max_issued)
            {
                self.registry_gap(id, r);
                return Ok(ResourceRegistrySampleV1::Gap);
            }
            let before = o.generation.clone();
            let after = ResourceRegistryGenerationV1 {
                stream: id.clone(),
                serial: before.serial.checked_add(1).ok_or(ResourceError::Limit)?,
            };
            let State::Active(p) = &mut self.registry_observations else {
                unreachable!()
            };
            p.issued += 1 + stage.added;
            let o = p.streams[id.slot].as_mut().unwrap();
            o.bindings = stage.bindings;
            o.baseline = Some(stage.projection.clone());
            o.generation = after.clone();
            o.queue.push_back(Pending {
                marker: Arc::new(()),
                event: ResourceRegistryEventV1 {
                    before,
                    after,
                    projection: stage.projection,
                },
            });
            Ok(ResourceRegistrySampleV1::Queued)
        })();
        report(
            r,
            Boundary::RegistryObservationSample,
            &result,
            timer.finish(),
        );
        result
    }
    /// Privileged classification input like native catalog metadata, NOT an
    /// ordinary recipient handoff. F19 must use certificate-checked delivery.
    pub fn inspect_resource_registry_head_v1(
        &self,
        id: &ResourceRegistryStreamIdV1,
        r: &mut Recorder,
    ) -> Result<Option<ResourceRegistryHistoricalHeadV1>> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let o = self.registry_observer(id)?;
            let root = self.resources.catalog.stamp()?;
            if let Some(q) = o.queue.front()
                && add(
                    size_of::<ResourceRegistryHistoricalHeadV1>(),
                    projection::projection_bytes(&q.event.projection)?,
                )? > self.registry_observation_pool()?.limits.max_packet_bytes
            {
                return Err(ResourceError::Limit);
            }
            Ok(o.queue.front().map(|q| ResourceRegistryHistoricalHeadV1 {
                head: ResourceRegistryHeadIdV1 {
                    stream: id.clone(),
                    marker: q.marker.clone(),
                    root: root.clone(),
                },
                projection: q.event.projection.clone(),
            }))
        })();
        report(
            r,
            Boundary::RegistryObservationHostHead,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn prepare_resource_registry_historical_release_v1(
        &mut self,
        id: &ResourceRegistryStreamIdV1,
        s: &ResourceRegistryObservationScopeV1,
        head: &ResourceRegistryHeadIdV1,
        f: &TrustedResourceRegistryHistoricalFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResourceRegistryPreparationIdV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            if head.stream != *id || head.root != self.resources.catalog.stamp()? {
                return Err(ResourceError::Unavailable);
            }
            self.registry_head_original(id, s, &head.marker)?;
            self.registry_gate(id, s, f.current)?;
            if f.current.resources_ready != Some(true)
                || f.historical_release_attested != Some(true)
                || head.stream != *id
            {
                return Err(ResourceError::Unavailable);
            }
            let p = self.registry_observation_pool()?;
            let l = p.limits;
            if p.issued >= l.max_issued || p.packets.iter().flatten().count() >= l.max_preparations
            {
                return Err(ResourceError::Limit);
            }
            let o = self.registry_observer(id)?;
            let q = o.queue.front().ok_or(ResourceError::Unavailable)?;
            if !Arc::ptr_eq(&q.marker, &head.marker)
                || f.rows.len() != q.event.projection.rows.len()
                || f.rows.len() > l.max_rows
            {
                return Err(ResourceError::Unavailable);
            }
            for (i, fact) in f.rows.iter().enumerate() {
                if (!fact.membership_allowed && fact.descriptor_allowed)
                    || f.rows[..i].iter().any(|p| p.reference == fact.reference)
                    || q.event
                        .projection
                        .rows
                        .iter()
                        .filter(|r| r.reference == *fact.reference)
                        .count()
                        != 1
                {
                    return Err(ResourceError::Malformed);
                }
            }
            let allowed = q.event.projection.rows.iter().all(|row| {
                f.rows.iter().any(|fact| {
                    *fact.reference == row.reference
                        && fact.membership_allowed
                        && (row.descriptor.is_none() || fact.descriptor_allowed)
                })
            });
            let packet = Packet {
                stream: PacketTarget::Stream(id.clone()),
                scope: projection::copy_scope(s)?,
                root: self.resources.catalog.stamp()?,
                command_basis: None,
                value: PacketValue::Historical {
                    head: head.marker.clone(),
                    allowed,
                },
            };
            self.registry_issue_packet(packet)
        })();
        report(
            r,
            Boundary::RegistryObservationHistoricalPreparation,
            &result,
            timer.finish(),
        );
        result
    }
    /// Privileged borrowed historical classification input, never a recipient
    /// slot resolver or grant. The exact native head and current root must match.
    pub fn resource_registry_historical_address_v1(
        &self,
        head: &ResourceRegistryHeadIdV1,
        reference: &ResourceRegistrySlotReferenceV1,
        r: &mut Recorder,
    ) -> Result<&ResourceCatalogAddressV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            if head.root != self.resources.catalog.stamp()? {
                return Err(ResourceError::Unavailable);
            }
            let o = self.registry_observer(&head.stream)?;
            let q = o.queue.front().ok_or(ResourceError::Unavailable)?;
            if !Arc::ptr_eq(&q.marker, &head.marker)
                || !q
                    .event
                    .projection
                    .rows
                    .iter()
                    .any(|row| row.reference == *reference)
            {
                return Err(ResourceError::Unavailable);
            }
            o.bindings
                .iter()
                .find(|b| b.reference == *reference)
                .map(|b| &b.address)
                .ok_or(ResourceError::Faulted)
        })();
        report(
            r,
            Boundary::RegistryObservationHostSlot,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn release_resource_registry_event_v1(
        &mut self,
        id: &ResourceRegistryStreamIdV1,
        s: &ResourceRegistryObservationScopeV1,
        certificate: Option<&ResourceRegistryPreparationIdV1>,
        f: &TrustedResourceRegistryObservationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResourceRegistryDeliveryV1> {
        self.release_resource_registry_event_frame_v1(id, s, certificate, f, 0, r)
    }
    pub(in crate::coordinator) fn release_resource_registry_event_frame_v1(
        &mut self,
        id: &ResourceRegistryStreamIdV1,
        s: &ResourceRegistryObservationScopeV1,
        certificate: Option<&ResourceRegistryPreparationIdV1>,
        f: &TrustedResourceRegistryObservationFactsV1<'_>,
        extra_header: usize,
        r: &mut Recorder,
    ) -> Result<ResourceRegistryDeliveryV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            if let Some(certificate) = certificate {
                self.registry_certificate_original(certificate, id, s)?;
            }
            self.registry_gate(id, s, f)?;
            if !f.ready() {
                return Err(ResourceError::Unavailable);
            }
            let o = self.registry_observer(id)?;
            let reply_limit = self.registry_observation_pool()?.limits.max_reply_bytes;
            if extra_header != 0
                && size_of::<ResourceRegistryDeliveryV1>()
                    .checked_add(extra_header)
                    .is_none_or(|n| n > reply_limit)
            {
                return Err(ResourceError::Limit);
            }
            let Some(certificate) = certificate else {
                return if o.phase == Phase::Gap {
                    Ok(ResourceRegistryDeliveryV1::Gap)
                } else if o.queue.is_empty() {
                    Ok(ResourceRegistryDeliveryV1::Empty)
                } else {
                    Err(ResourceError::Unavailable)
                };
            };
            let head = o.queue.front().ok_or(ResourceError::Unavailable)?;
            let packet = self.registry_packet(certificate, id, s)?;
            let PacketValue::Historical {
                head: marker,
                allowed,
            } = &packet.value
            else {
                return Err(ResourceError::Unavailable);
            };
            if !Arc::ptr_eq(marker, &head.marker) {
                return Err(ResourceError::Unavailable);
            }
            let allowed = *allowed;
            if allowed
                && projection::frame_bytes(&head.event.projection)?
                    .checked_add(extra_header)
                    .is_none_or(|n| n > reply_limit)
            {
                return Err(ResourceError::Limit);
            }
            self.registry_take_packet(certificate);
            if !allowed {
                self.registry_gap(id, r);
                return Ok(ResourceRegistryDeliveryV1::Gap);
            }
            Ok(ResourceRegistryDeliveryV1::Projection(
                self.registry_observer_mut(id)
                    .queue
                    .pop_front()
                    .unwrap()
                    .event,
            ))
        })();
        report(
            r,
            Boundary::RegistryObservationRelease,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn resync_resource_registry_observation_v1(
        &mut self,
        id: &ResourceRegistryStreamIdV1,
        s: &ResourceRegistryObservationScopeV1,
        packet: &ResourceRegistryPreparationIdV1,
        f: &TrustedResourceRegistryObservationFactsV1<'_>,
        baseline_allowed: Option<bool>,
        r: &mut Recorder,
    ) -> Result<ResourceRegistryBaselineV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.registry_snapshot_original(packet, id, s)?;
            self.registry_gate(id, s, f)?;
            if !f.ready() || baseline_allowed != Some(true) {
                return Err(ResourceError::Unavailable);
            }
            if !matches!(
                self.registry_packet(packet, id, s)?.value,
                PacketValue::Snapshot(_)
            ) {
                return Err(ResourceError::Unavailable);
            }
            let packet = self.registry_take_packet(packet);
            let PacketValue::Snapshot(rows) = packet.value else {
                unreachable!()
            };
            let o = self.registry_observer(id)?;
            let p = self.registry_observation_pool()?;
            let stage = projection::stage(id, &rows, &o.bindings, p.limits)?;
            if p.issued
                .checked_add(1)
                .and_then(|n| n.checked_add(stage.added))
                .is_none_or(|n| n > p.limits.max_issued)
            {
                return Err(ResourceError::Limit);
            }
            let generation = ResourceRegistryGenerationV1 {
                stream: id.clone(),
                serial: o
                    .generation
                    .serial
                    .checked_add(1)
                    .ok_or(ResourceError::Limit)?,
            };
            let projection = stage.projection;
            let State::Active(p) = &mut self.registry_observations else {
                unreachable!()
            };
            p.issued += 1 + stage.added;
            let o = p.streams[id.slot].as_mut().unwrap();
            o.bindings = stage.bindings;
            o.queue.clear();
            o.baseline = Some(projection.clone());
            o.generation = generation.clone();
            o.phase = Phase::Active;
            Ok(ResourceRegistryBaselineV1 {
                stream: id.clone(),
                generation,
                projection,
            })
        })();
        report(
            r,
            Boundary::RegistryObservationResync,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn stop_resource_registry_observation_v1(
        &mut self,
        id: &ResourceRegistryStreamIdV1,
        c: &ResourceRegistryObservationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<()> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let o = self.registry_observer_control(id)?;
            if o.scope != *c.scope || c.owner_control_allowed != Some(true) {
                return Err(ResourceError::Unavailable);
            }
            self.registry_observer_mut(id).close(Phase::Stopped);
            Ok(())
        })();
        report(
            r,
            Boundary::RegistryObservationStop,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn retire_resource_registry_observation_v1(
        &mut self,
        id: &ResourceRegistryStreamIdV1,
        c: &ResourceRegistryObservationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<()> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let o = self.registry_observer_control(id)?;
            if o.scope != *c.scope
                || o.phase != Phase::Stopped
                || [
                    c.owner_control_allowed,
                    c.retention_released,
                    c.delivery_drained,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(ResourceError::Unavailable);
            }
            let State::Active(p) = &mut self.registry_observations else {
                unreachable!()
            };
            p.streams[id.slot] = None;
            Ok(())
        })();
        report(
            r,
            Boundary::RegistryObservationRetirement,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn retire_resource_registry_preparation_v1(
        &mut self,
        id: &ResourceRegistryPreparationIdV1,
        c: &ResourceRegistryObservationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<()> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let p = self.registry_observation_pool()?;
            if !Arc::ptr_eq(&p.owner, &id.owner)
                || [
                    c.owner_control_allowed,
                    c.retention_released,
                    c.delivery_drained,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(ResourceError::Unavailable);
            }
            let packet = p
                .packets
                .get(id.slot)
                .and_then(Option::as_ref)
                .ok_or(ResourceError::Unavailable)?;
            if packet.scope != *c.scope
                || matches!(
                    packet.value,
                    PacketValue::Command(_) | PacketValue::ReceiptContext(_)
                )
            {
                return Err(ResourceError::Unavailable);
            }
            let State::Active(p) = &mut self.registry_observations else {
                unreachable!()
            };
            p.packets[id.slot] = None;
            Ok(())
        })();
        report(
            r,
            Boundary::RegistryObservationPreparationRetirement,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn retire_resource_registry_observation_pool_v1(
        &mut self,
        c: &SourceReadPoolControlV1,
        r: &mut Recorder,
    ) -> Result<()> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let p = self.registry_observation_pool()?;
            if [
                c.owner_control_allowed,
                c.retention_released,
                c.delivery_drained,
            ]
            .into_iter()
            .any(|v| v != Some(true))
                || p.streams.iter().any(Option::is_some)
                || p.packets.iter().any(Option::is_some)
            {
                return Err(ResourceError::Unavailable);
            }
            let State::Active(p) = &mut self.registry_observations else {
                unreachable!()
            };
            self.retention.release(&mut p.reservation).map_err(parent)?;
            self.registry_observations = State::Closed { issued: p.issued };
            Ok(())
        })();
        report(
            r,
            Boundary::RegistryObservationPoolRetirement,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn resource_registry_observation_usage_v1(
        &self,
    ) -> Result<ResourceRegistryObservationUsageV1> {
        match &self.registry_observations {
            State::Unconfigured => Ok(ResourceRegistryObservationUsageV1::default()),
            State::Closed { issued } => Ok(ResourceRegistryObservationUsageV1 {
                issued_lifetime: *issued,
                closed: true,
                ..Default::default()
            }),
            State::Active(_) => {
                let p = self.registry_observation_pool()?;
                let mut u = ResourceRegistryObservationUsageV1 {
                    issued_lifetime: p.issued,
                    reserved_bytes: p.charge,
                    ..Default::default()
                };
                for (slot, o) in p
                    .streams
                    .iter()
                    .enumerate()
                    .filter_map(|(i, o)| o.as_ref().map(|o| (i, o)))
                {
                    let id = ResourceRegistryStreamIdV1 {
                        owner: p.owner.clone(),
                        slot,
                    };
                    if !projection::valid(o, &id, p.limits, p.issued) {
                        return Err(ResourceError::Faulted);
                    }
                    u.streams += 1;
                    u.queued = add(u.queued, o.queue.len())?;
                    u.bindings = add(u.bindings, o.bindings.len())?;
                    match o.phase {
                        Phase::Gap => u.gapped += 1,
                        Phase::Stopped => u.stopped += 1,
                        Phase::Active => {}
                    }
                }
                u.preparations = p.packets.iter().flatten().count();
                Ok(u)
            }
        }
    }
}
