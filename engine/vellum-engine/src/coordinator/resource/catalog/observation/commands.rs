//! Original commands share the actual bounded registry packet owner.
use super::*;
pub const RESOURCE_REGISTRY_COMMAND_PROFILE_V1: &str = "registration-observation-command/1";
mod decoder;
#[cfg(test)]
mod tests;
pub use decoder::*;

identity!(ResourceRegistryCommandIdV1);
#[derive(Clone, PartialEq, Eq)]
pub enum ResourceRegistryCommandOperationV1 {
    Establish,
    Resynchronize { stream: ResourceRegistryStreamIdV1 },
}
impl ResourceRegistryCommandOperationV1 {
    fn target(&self) -> PacketTarget {
        match self {
            Self::Establish => PacketTarget::Establish,
            Self::Resynchronize { stream } => PacketTarget::Stream(stream.clone()),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceRegistryCommandPhaseV1 {
    Issued,
    Executing,
    Settled,
    Faulted,
}
#[derive(Clone, PartialEq, Eq)]
pub enum ResourceRegistryCommandReplyV1 {
    Settled {
        command: ResourceRegistryCommandIdV1,
    },
    ObserveExisting {
        command: ResourceRegistryCommandIdV1,
    },
}
pub struct TrustedResourceRegistryCommandAdmissionV1<'a> {
    pub current: &'a TrustedResourceRegistryObservationFactsV1<'a>,
    pub owner_control_allowed: Option<bool>,
    pub baseline_allowed: Option<bool>,
}
pub struct ResourceRegistryCommandRequestV1<'a> {
    pub operation: &'a ResourceRegistryCommandOperationV1,
    pub scope: &'a ResourceRegistryObservationScopeV1,
    pub preparation: Option<&'a ResourceRegistryPreparationIdV1>,
    pub current: Option<&'a TrustedResourceRegistryObservationFactsV1<'a>>,
    pub owner_control_allowed: Option<bool>,
    pub baseline_allowed: Option<bool>,
}
pub struct ResourceRegistryCommandHistoricalSlotV1 {
    pub reference: ResourceRegistrySlotReferenceV1,
    pub address: ResourceCatalogAddressV1,
}
/// Privileged first-party fact seam. Never an ordinary retained-outcome reply.
pub struct ResourceRegistryCommandFactsV1<'a> {
    pub operation: &'a ResourceRegistryCommandOperationV1,
    pub phase: ResourceRegistryCommandPhaseV1,
    pub native_started: bool,
    pub as_observed: Option<&'a Result<Arc<ResourceRegistryBaselineV1>>>,
    pub historical_slots: &'a [ResourceRegistryCommandHistoricalSlotV1],
}
#[derive(Debug)]
pub struct ResourceRegistryCommandProtocolFaultV1;
redacted!(
    ResourceRegistryCommandIdV1,
    ResourceRegistryCommandOperationV1,
    ResourceRegistryCommandReplyV1,
    ResourceRegistryCommandRequestV1<'_>,
    ResourceRegistryCommandHistoricalSlotV1,
    ResourceRegistryCommandFactsV1<'_>
);
pub(super) struct Record {
    operation: ResourceRegistryCommandOperationV1,
    root: root_identity::NativeRootIdentity,
    generation: Option<ResourceRegistryGenerationV1>,
    phase: ResourceRegistryCommandPhaseV1,
    native_started: bool,
    result: Option<Result<Arc<ResourceRegistryBaselineV1>>>,
    historical: Vec<ResourceRegistryCommandHistoricalSlotV1>,
}
impl Record {
    fn valid(&self) -> bool {
        use ResourceRegistryCommandPhaseV1 as P;
        match self.phase {
            P::Issued => {
                !self.native_started && self.result.is_none() && self.historical.is_empty()
            }
            P::Executing => self.result.is_none(),
            P::Faulted => {
                self.native_started || (self.result.is_none() && self.historical.is_empty())
            }
            P::Settled => {
                self.native_started
                    && self.result.as_ref().is_some_and(|result| match result {
                        Err(_) => self.historical.is_empty(),
                        Ok(b) => {
                            self.historical.len() == b.projection.rows.len()
                                && self
                                    .historical
                                    .iter()
                                    .zip(&b.projection.rows)
                                    .all(|(h, r)| {
                                        h.reference == r.reference && h.address.table == r.table
                                    })
                        }
                    })
            }
        }
    }
}
#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FaultCut {
    BeforeNative,
    AfterNative,
    NativeRefusal,
}
impl SourceCoordinator {
    pub(super) fn registry_command_ordinary_v1(&self) -> Result<()> {
        let p = self.registry_observation_pool()?;
        if p.packets.iter().flatten().any(|p|matches!(&p.value,PacketValue::Command(c) if !c.valid()||(c.phase==ResourceRegistryCommandPhaseV1::Faulted&&c.native_started))) {return Err(ResourceError::Faulted);}
        Ok(())
    }
    fn registry_command_packet_v1(
        &self,
        id: &ResourceRegistryCommandIdV1,
        s: &ResourceRegistryObservationScopeV1,
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
        if packet.scope != *s {
            return Err(ResourceError::Unavailable);
        }
        let PacketValue::Command(c) = &packet.value else {
            return Err(ResourceError::Unavailable);
        };
        if !c.valid() || packet.stream != c.operation.target() {
            return Err(ResourceError::Faulted);
        }
        projection::packet_bytes(packet, p.limits)?;
        Ok(packet)
    }
    fn registry_command_v1(
        &self,
        id: &ResourceRegistryCommandIdV1,
        s: &ResourceRegistryObservationScopeV1,
    ) -> Result<&Record> {
        let PacketValue::Command(c) = &self.registry_command_packet_v1(id, s)?.value else {
            unreachable!()
        };
        Ok(c)
    }
    fn registry_command_mut_v1(&mut self, id: &ResourceRegistryCommandIdV1) -> &mut Record {
        let State::Active(p) = &mut self.registry_observations else {
            unreachable!()
        };
        let PacketValue::Command(c) =
            &mut p.packets[id.slot].as_mut().expect("retained command").value
        else {
            unreachable!()
        };
        c
    }
    fn registry_command_basis_v1(&self, c: &Record) -> Result<bool> {
        if c.root
            != self
                .native_root_identity()
                .map_err(|_| ResourceError::Faulted)?
        {
            return Ok(false);
        }
        Ok(match &c.operation {
            ResourceRegistryCommandOperationV1::Establish => c.generation.is_none(),
            ResourceRegistryCommandOperationV1::Resynchronize { stream } => {
                self.registry_observer(stream).is_ok_and(|o| {
                    o.phase != Phase::Stopped && Some(&o.generation) == c.generation.as_ref()
                })
            }
        })
    }
    pub fn issue_resource_registry_command_v1(
        &mut self,
        operation: &ResourceRegistryCommandOperationV1,
        s: &ResourceRegistryObservationScopeV1,
        f: &TrustedResourceRegistryCommandAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResourceRegistryCommandIdV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(owner_error)?;
            self.registry_command_ordinary_v1()?;
            let p = self.registry_observation_pool()?;
            let l = p.limits;
            projection::scope_bytes(s, l)?;
            if self.source_facade_busy
                || f.current.scope != s
                || s.context.authority != p.authority
                || s.context.engine != p.engine
                || f.current.native_profile != p.native
                || f.current.required().into_iter().any(|v| v != Some(true))
                || !f.current.ready()
                || f.owner_control_allowed != Some(true)
                || f.baseline_allowed != Some(true)
            {
                return Err(ResourceError::Unavailable);
            }
            if l.max_preparations < 2
                || l.max_reply_bytes < size_of::<ResourceRegistryCommandReplyV1>()
            {
                return Err(ResourceError::Limit);
            }
            let generation = match operation {
                ResourceRegistryCommandOperationV1::Establish => None,
                ResourceRegistryCommandOperationV1::Resynchronize { stream } => {
                    let o = self.registry_observer(stream)?;
                    if o.scope != *s || o.phase == Phase::Stopped {
                        return Err(ResourceError::Unavailable);
                    }
                    Some(o.generation.clone())
                }
            };
            if self
                .semantic_root_usage(r)
                .map_err(|_| ResourceError::Faulted)?
                .total_bytes
                > l.max_root_bytes
            {
                return Err(ResourceError::Limit);
            }
            let root = self
                .native_root_identity()
                .map_err(|_| ResourceError::Faulted)?;
            let packet = Packet {
                stream: operation.target(),
                scope: projection::copy_scope(s)?,
                root: self.resources.catalog.stamp()?,
                command_basis: None,
                value: PacketValue::Command(Box::new(Record {
                    operation: operation.clone(),
                    root,
                    generation,
                    phase: ResourceRegistryCommandPhaseV1::Issued,
                    native_started: false,
                    result: None,
                    historical: Vec::new(),
                })),
            };
            let issued = self.registry_issue_packet(packet)?;
            Ok(ResourceRegistryCommandIdV1 {
                owner: issued.owner,
                slot: issued.slot,
            })
        })();
        report(r, Boundary::RegistryCommandIssue, &result, timer.finish());
        result
    }
    pub fn prepare_resource_registry_command_snapshot_v1(
        &mut self,
        id: &ResourceRegistryCommandIdV1,
        s: &ResourceRegistryObservationScopeV1,
        f: &TrustedResourceRegistrySnapshotFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResourceRegistryPreparationIdV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            self.registry_command_ordinary_v1()?;
            let c = self.registry_command_v1(id, s)?;
            if c.phase != ResourceRegistryCommandPhaseV1::Issued
                || !self.registry_command_basis_v1(c)?
            {
                return Err(ResourceError::Unavailable);
            }
            let basis = c.root.clone();
            let target = c.operation.target();
            let p = self.registry_observation_pool()?;
            if self.source_facade_busy
                || f.current.scope != s
                || f.current.native_profile != p.native
                || f.current.required().into_iter().any(|v| v != Some(true))
                || f.current.resources_ready != Some(true)
            {
                return Err(ResourceError::Unavailable);
            }
            if p.issued >= p.limits.max_issued
                || p.packets.iter().flatten().count() >= p.limits.max_preparations
            {
                return Err(ResourceError::Limit);
            }
            let (root, native) = self.resources.catalog.current()?;
            let rows = projection::snapshot(root, s, f, p.limits, native)?;
            self.registry_issue_packet(Packet {
                stream: target,
                scope: projection::copy_scope(s)?,
                root: self.resources.catalog.stamp()?,
                command_basis: Some(basis),
                value: PacketValue::Snapshot(rows),
            })
        })();
        report(
            r,
            Boundary::RegistryCommandPreparation,
            &result,
            timer.finish(),
        );
        result
    }
    fn registry_command_preparation_v1(
        &self,
        id: &ResourceRegistryCommandIdV1,
        s: &ResourceRegistryObservationScopeV1,
        preparation: &ResourceRegistryPreparationIdV1,
    ) -> Result<&Packet> {
        let p = self.registry_observation_pool()?;
        let command = self.registry_command_packet_v1(id, s)?;
        let PacketValue::Command(c) = &command.value else {
            unreachable!()
        };
        if !Arc::ptr_eq(&p.owner, &preparation.owner) {
            return Err(ResourceError::Unavailable);
        }
        let packet = p
            .packets
            .get(preparation.slot)
            .and_then(Option::as_ref)
            .ok_or(ResourceError::Unavailable)?;
        if packet.scope != *s
            || packet.stream != c.operation.target()
            || packet.root != command.root
            || packet.root != self.resources.catalog.stamp()?
            || packet.command_basis.as_ref() != Some(&c.root)
            || !matches!(packet.value, PacketValue::Snapshot(_))
        {
            return Err(ResourceError::Unavailable);
        }
        projection::packet_bytes(packet, p.limits)?;
        Ok(packet)
    }
    pub fn request_resource_registry_command_v1(
        &mut self,
        id: &ResourceRegistryCommandIdV1,
        q: &ResourceRegistryCommandRequestV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResourceRegistryCommandReplyV1> {
        let mut timer = Some(r.begin_service_timing());
        let mut lookup = false;
        let result = (|| {
            let c = self.registry_command_v1(id, q.scope)?;
            if c.operation != *q.operation {
                return Err(ResourceError::Unavailable);
            }
            if c.phase == ResourceRegistryCommandPhaseV1::Settled {
                lookup = true;
                if q.owner_control_allowed != Some(true) {
                    return Err(ResourceError::Unavailable);
                }
                return Ok(ResourceRegistryCommandReplyV1::ObserveExisting {
                    command: id.clone(),
                });
            }
            if c.phase != ResourceRegistryCommandPhaseV1::Issued {
                return Err(ResourceError::Unavailable);
            }
            self.ensure_operational().map_err(owner_error)?;
            if matches!(q.operation, ResourceRegistryCommandOperationV1::Establish) {
                self.ensure_admission_unfenced().map_err(owner_error)?;
            }
            self.registry_command_ordinary_v1()?;
            let preparation = q.preparation.ok_or(ResourceError::Unavailable)?;
            if !self.registry_command_basis_v1(c)? {
                return Err(ResourceError::Unavailable);
            }
            self.registry_command_preparation_v1(id, q.scope, preparation)?;
            self.registry_command_mut_v1(id).phase = ResourceRegistryCommandPhaseV1::Executing;
            let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let f = q.current.ok_or(ResourceError::Unavailable)?;
                if self.source_facade_busy
                    || q.owner_control_allowed != Some(true)
                    || q.baseline_allowed != Some(true)
                    || f.scope != q.scope
                    || f.native_profile != self.registry_observation_pool()?.native
                    || f.required().into_iter().any(|v| v != Some(true))
                    || !f.ready()
                {
                    return Err(ResourceError::Unavailable);
                }
                self.registry_command_ordinary_v1()?;
                let c = self.registry_command_v1(id, q.scope)?;
                if !self.registry_command_basis_v1(c)? {
                    return Err(ResourceError::Unavailable);
                }
                let packet = self.registry_command_preparation_v1(id, q.scope, preparation)?;
                let PacketValue::Snapshot(rows) = &packet.value else {
                    unreachable!()
                };
                let p = self.registry_observation_pool()?;
                let l = p.limits;
                let old = match &c.operation {
                    ResourceRegistryCommandOperationV1::Establish => &[][..],
                    ResourceRegistryCommandOperationV1::Resynchronize { stream } => {
                        &self.registry_observer(stream)?.bindings
                    }
                };
                let added = projection::stage_admission(rows, old, l)?;
                if p.issued
                    .checked_add(1)
                    .and_then(|n| n.checked_add(added))
                    .is_none_or(|n| n > l.max_issued)
                    || matches!(c.operation, ResourceRegistryCommandOperationV1::Establish)
                        && p.streams.iter().flatten().count() >= l.max_streams
                {
                    return Err(ResourceError::Limit);
                }
                let expected_stream = match &c.operation {
                    ResourceRegistryCommandOperationV1::Establish => ResourceRegistryStreamIdV1 {
                        owner: p.owner.clone(),
                        slot: p.streams.len(),
                    },
                    ResourceRegistryCommandOperationV1::Resynchronize { stream } => stream.clone(),
                };
                let expected_generation = ResourceRegistryGenerationV1 {
                    stream: expected_stream.clone(),
                    serial: match &c.generation {
                        None => 0,
                        Some(g) => g.serial.checked_add(1).ok_or(ResourceError::Limit)?,
                    },
                };
                let mut addresses = Vec::new();
                addresses
                    .try_reserve_exact(rows.len())
                    .map_err(|_| ResourceError::Capacity)?;
                let mut historical = Vec::new();
                historical
                    .try_reserve_exact(rows.len())
                    .map_err(|_| ResourceError::Capacity)?;
                let mut bytes = 0usize;
                for row in rows {
                    bytes = add(
                        add(bytes, size_of::<ResourceRegistryCommandHistoricalSlotV1>())?,
                        row.address.payload()?,
                    )?;
                    addresses.push(row.address.copy()?);
                }
                if bytes > l.max_binding_bytes {
                    return Err(ResourceError::Limit);
                }
                #[cfg(test)]
                self.registry_command_fault_v1(FaultCut::BeforeNative);
                self.registry_command_mut_v1(id).native_started = true;
                let native = match q.operation {
                    ResourceRegistryCommandOperationV1::Establish => {
                        let packet = self.registry_take_packet(preparation);
                        let PacketValue::Snapshot(rows) = packet.value else {
                            unreachable!()
                        };
                        let native_timer = r.begin_service_timing();
                        let result = self.registry_open_rows_v1(q.scope, &rows, f);
                        report(
                            r,
                            Boundary::RegistryObservationOpen,
                            &result,
                            native_timer.finish(),
                        );
                        result
                    }
                    ResourceRegistryCommandOperationV1::Resynchronize { stream } => self
                        .resync_resource_registry_observation_v1(
                            stream,
                            q.scope,
                            preparation,
                            f,
                            q.baseline_allowed,
                            r,
                        ),
                };
                #[cfg(test)]
                self.registry_command_fault_v1(FaultCut::AfterNative);
                let native = match native {
                    Ok(baseline) => {
                        let o = self
                            .registry_observer(&baseline.stream)
                            .unwrap_or_else(|_| {
                                std::panic::panic_any(ResourceRegistryCommandProtocolFaultV1)
                            });
                        if baseline.stream != expected_stream
                            || baseline.generation != expected_generation
                            || baseline.projection.rows.len() != addresses.len()
                            || o.scope != *q.scope
                            || o.generation != baseline.generation
                            || !o
                                .baseline
                                .as_ref()
                                .is_some_and(|p| Arc::ptr_eq(p, &baseline.projection))
                        {
                            std::panic::panic_any(ResourceRegistryCommandProtocolFaultV1);
                        }
                        for (address, row) in addresses.into_iter().zip(&baseline.projection.rows) {
                            if address.table != row.table
                                || !o
                                    .bindings
                                    .iter()
                                    .any(|b| b.reference == row.reference && b.address == address)
                            {
                                std::panic::panic_any(ResourceRegistryCommandProtocolFaultV1);
                            }
                            historical.push(ResourceRegistryCommandHistoricalSlotV1 {
                                address,
                                reference: row.reference.clone(),
                            });
                        }
                        Ok(Arc::new(baseline))
                    }
                    Err(e) => Err(e),
                };
                let c = self.registry_command_mut_v1(id);
                c.result = Some(native);
                c.historical = historical;
                c.phase = ResourceRegistryCommandPhaseV1::Settled;
                Ok(ResourceRegistryCommandReplyV1::Settled {
                    command: id.clone(),
                })
            }));
            match caught {
                Ok(result) => {
                    if result.is_err() {
                        if self.registry_command_mut_v1(id).native_started {
                            self.registry_command_protocol_fault_v1(
                                id,
                                r,
                                timer.take().expect("one command timing").finish(),
                            );
                        }
                        self.registry_command_mut_v1(id).phase =
                            ResourceRegistryCommandPhaseV1::Issued;
                    }
                    result
                }
                Err(_) => self.registry_command_protocol_fault_v1(
                    id,
                    r,
                    timer.take().expect("one command timing").finish(),
                ),
            }
        })();
        report(
            r,
            if lookup {
                Boundary::RegistryCommandLookup
            } else {
                Boundary::RegistryCommandExecution
            },
            &result,
            timer.take().expect("one command timing").finish(),
        );
        result
    }
    fn registry_command_protocol_fault_v1(
        &mut self,
        id: &ResourceRegistryCommandIdV1,
        r: &mut Recorder,
        time: Option<std::time::Duration>,
    ) -> ! {
        self.registry_command_mut_v1(id).phase = ResourceRegistryCommandPhaseV1::Faulted;
        r.record(Observation {
            boundary: Boundary::RegistryCommandProtocolFault,
            disposition: Disposition::Unknown,
            service_time: None,
        });
        report::<()>(
            r,
            Boundary::RegistryCommandExecution,
            &Err(ResourceError::Faulted),
            time,
        );
        std::panic::resume_unwind(Box::new(ResourceRegistryCommandProtocolFaultV1))
    }
    /// Original retained native result and private historical slots. Control
    /// permission does not make this a recipient baseline disclosure endpoint.
    pub fn resource_registry_command_facts_v1(
        &self,
        id: &ResourceRegistryCommandIdV1,
        s: &ResourceRegistryObservationScopeV1,
        r: &mut Recorder,
    ) -> Result<ResourceRegistryCommandFactsV1<'_>> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let c = self.registry_command_v1(id, s)?;
            Ok(ResourceRegistryCommandFactsV1 {
                operation: &c.operation,
                phase: c.phase,
                native_started: c.native_started,
                as_observed: c.result.as_ref(),
                historical_slots: &c.historical,
            })
        })();
        report(
            r,
            Boundary::RegistryCommandHostFacts,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn retire_resource_registry_command_v1(
        &mut self,
        id: &ResourceRegistryCommandIdV1,
        c: &ResourceRegistryObservationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<()> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let record = self.registry_command_v1(id, c.scope)?;
            if [
                c.owner_control_allowed,
                c.retention_released,
                c.delivery_drained,
            ]
            .into_iter()
            .any(|v| v != Some(true))
                || record.phase == ResourceRegistryCommandPhaseV1::Executing
                || (record.phase == ResourceRegistryCommandPhaseV1::Faulted
                    && record.native_started)
            {
                return Err(ResourceError::Unavailable);
            }
            if receipts::command_claimed(self.registry_observation_pool()?, id) {
                return Err(ResourceError::Retained);
            }
            let State::Active(p) = &mut self.registry_observations else {
                unreachable!()
            };
            p.packets[id.slot] = None;
            Ok(())
        })();
        report(
            r,
            Boundary::RegistryCommandRetirement,
            &result,
            timer.finish(),
        );
        result
    }
    #[cfg(test)]
    fn registry_command_fault_v1(&mut self, cut: FaultCut) {
        let State::Active(p) = &mut self.registry_observations else {
            unreachable!()
        };
        if p.command_fault == Some(cut) {
            p.command_fault = None;
            std::panic::panic_any(ResourceRegistryCommandProtocolFaultV1);
        }
    }
}
