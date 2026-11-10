//! Separately authorized historical command receipts, not replay/control data.
use super::*;
mod decoder;
#[cfg(test)]
pub(super) mod tests;
pub use decoder::*;

pub const RESOURCE_REGISTRY_RECEIPT_PROFILE_V1: &str = "registration-observation-receipt/1";
identity!(ResourceRegistryReceiptContextIdV1);
/// Native host correlation only: not a grant, recipient selector or managed hold.
pub struct ResourceRegistryReceiptAuthorizationBasisIdV1 {
    context: ResourceRegistryReceiptContextIdV1,
    command: ResourceRegistryCommandIdV1,
    result: Arc<ResourceRegistryBaselineV1>,
    root: root_identity::NativeRootIdentity,
}
/// Privileged original slots with the exact engine state observed by their classifier.
pub struct ResourceRegistryReceiptAuthorizationBasisV1<'a> {
    pub slots: &'a [ResourceRegistryCommandHistoricalSlotV1],
    pub basis: ResourceRegistryReceiptAuthorizationBasisIdV1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceRegistryReceiptCeilingV1 {
    pub allowed: bool,
}
pub struct TrustedResourceRegistryReceiptFactsV1<'a> {
    pub current: &'a TrustedResourceRegistryObservationFactsV1<'a>,
    pub receipt_allowed: Option<bool>,
    pub receipt_grant_active: Option<bool>,
    /// Current validity of the exact historical authorization basis; root equality alone is insufficient.
    pub authorization_current: Option<bool>,
}
pub struct TrustedResourceRegistryReceiptHistoricalFactsV1<'a> {
    pub basis: &'a ResourceRegistryReceiptAuthorizationBasisIdV1,
    pub current: &'a TrustedResourceRegistryReceiptFactsV1<'a>,
    pub rows: &'a [ResourceRegistryHistoricalRowFactsV1<'a>],
    pub historical_release_attested: Option<bool>,
}
pub struct ResourceRegistryReceiptReplyV1 {
    pub receipt: Arc<ResourceRegistryBaselineV1>,
}
impl ResourceRegistryReceiptReplyV1 {
    pub fn status(&self) -> &'static str {
        "operation-receipt"
    }
    pub fn facet(&self) -> &'static str {
        "registry-baseline"
    }
}
redacted!(
    ResourceRegistryReceiptAuthorizationBasisIdV1,
    ResourceRegistryReceiptAuthorizationBasisV1<'_>,
    ResourceRegistryReceiptContextIdV1,
    ResourceRegistryReceiptReplyV1
);
pub(super) struct Context {
    command: ResourceRegistryCommandIdV1,
    result: Arc<ResourceRegistryBaselineV1>,
    ceiling: ResourceRegistryReceiptCeilingV1,
    stopped: bool,
}
pub(super) struct Certificate {
    context: ResourceRegistryReceiptContextIdV1,
    command: ResourceRegistryCommandIdV1,
    result: Arc<ResourceRegistryBaselineV1>,
    allowed: bool,
    valid: bool,
}
pub(super) fn command_claimed(p: &Pool, command: &ResourceRegistryCommandIdV1) -> bool {
    p.packets
        .iter()
        .flatten()
        .any(|p| matches!(&p.value,PacketValue::ReceiptContext(c) if c.command==*command))
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResourceRegistryReceiptUsageV1 {
    pub contexts: usize,
    pub stopped: usize,
    pub certificates: usize,
}
impl SourceCoordinator {
    fn registry_receipt_control_v1(
        &self,
        id: &ResourceRegistryReceiptContextIdV1,
        s: &ResourceRegistryObservationScopeV1,
    ) -> Result<&Context> {
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
        let PacketValue::ReceiptContext(c) = &packet.value else {
            return Err(ResourceError::Unavailable);
        };
        Ok(c)
    }
    fn registry_receipt_mut_v1(&mut self, id: &ResourceRegistryReceiptContextIdV1) -> &mut Context {
        let State::Active(p) = &mut self.registry_observations else {
            unreachable!()
        };
        let PacketValue::ReceiptContext(c) = &mut p.packets[id.slot]
            .as_mut()
            .expect("retained receipt context")
            .value
        else {
            unreachable!()
        };
        c
    }
    fn registry_receipt_original_v1(
        &self,
        id: &ResourceRegistryReceiptContextIdV1,
        command: &ResourceRegistryCommandIdV1,
        s: &ResourceRegistryObservationScopeV1,
    ) -> Result<&Context> {
        let c = self.registry_receipt_control_v1(id, s)?;
        if c.command != *command {
            return Err(ResourceError::Unavailable);
        }
        Ok(c)
    }
    fn registry_receipt_gate_v1(
        &mut self,
        id: &ResourceRegistryReceiptContextIdV1,
        command: &ResourceRegistryCommandIdV1,
        s: &ResourceRegistryObservationScopeV1,
        f: &TrustedResourceRegistryReceiptFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<()> {
        let c = self.registry_receipt_original_v1(id, command, s)?;
        if c.stopped || !c.ceiling.allowed || f.current.scope != s {
            return Err(ResourceError::Unavailable);
        }
        let retained = self.resource_registry_command_facts_v1(command, s, r);
        let intact = retained.as_ref().is_ok_and(|p| {
            p.phase == ResourceRegistryCommandPhaseV1::Settled
                && p.as_observed
                    .is_some_and(|result| result.as_ref().is_ok_and(|b| Arc::ptr_eq(b, &c.result)))
        });
        if !intact {
            self.registry_receipt_mut_v1(id).stopped = true;
            return Err(ResourceError::Faulted);
        }
        let current = f.current;
        if [
            current.authorized,
            current.scope_current,
            current.continuous,
            current.live,
            current.not_stopped,
            current.retained,
            f.receipt_grant_active,
        ]
        .contains(&Some(false))
            || current.native_profile != self.registry_observation_pool()?.native
        {
            self.registry_receipt_mut_v1(id).stopped = true;
            r.record(Observation {
                boundary: Boundary::RegistryReceiptClosure,
                disposition: Disposition::Completed,
                service_time: None,
            });
            return Err(ResourceError::Unavailable);
        }
        self.ensure_operational().map_err(owner_error)?;
        self.registry_command_ordinary_v1()?;
        if self.source_facade_busy
            || current.required().into_iter().any(|v| v != Some(true))
            || f.receipt_grant_active != Some(true)
        {
            return Err(ResourceError::Unavailable);
        }
        if self.resources.catalog.current()?.1 != current.native_profile {
            return Err(ResourceError::Unavailable);
        }
        Ok(())
    }
    fn registry_receipt_certificate_v1(
        &self,
        packet: &ResourceRegistryPreparationIdV1,
        id: &ResourceRegistryReceiptContextIdV1,
        command: &ResourceRegistryCommandIdV1,
        s: &ResourceRegistryObservationScopeV1,
    ) -> Result<&Certificate> {
        let c = self.registry_receipt_original_v1(id, command, s)?;
        let p = self.registry_observation_pool()?;
        if !Arc::ptr_eq(&p.owner, &packet.owner) {
            return Err(ResourceError::Unavailable);
        }
        let packet = p
            .packets
            .get(packet.slot)
            .and_then(Option::as_ref)
            .ok_or(ResourceError::Unavailable)?;
        let PacketValue::ReceiptCertificate(cert) = &packet.value else {
            return Err(ResourceError::Unavailable);
        };
        if packet.scope != *s
            || cert.context != *id
            || cert.command != *command
            || !Arc::ptr_eq(&cert.result, &c.result)
            || packet.root != self.resources.catalog.stamp()?
            || packet.command_basis.as_ref()
                != Some(
                    &self
                        .native_root_identity()
                        .map_err(|_| ResourceError::Faulted)?,
                )
        {
            return Err(ResourceError::Unavailable);
        }
        projection::packet_bytes(packet, p.limits)?;
        Ok(cert)
    }
    pub fn open_resource_registry_receipt_v1(
        &mut self,
        command: &ResourceRegistryCommandIdV1,
        s: &ResourceRegistryObservationScopeV1,
        ceiling: ResourceRegistryReceiptCeilingV1,
        f: &TrustedResourceRegistryReceiptFactsV1<'_>,
        admission_allowed: Option<bool>,
        r: &mut Recorder,
    ) -> Result<ResourceRegistryReceiptContextIdV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(owner_error)?;
            self.registry_command_ordinary_v1()?;
            let host = self.resource_registry_command_facts_v1(command, s, r)?;
            if host.phase != ResourceRegistryCommandPhaseV1::Settled {
                return Err(ResourceError::Unavailable);
            }
            let result = host
                .as_observed
                .and_then(|r| r.as_ref().ok())
                .ok_or(ResourceError::Unavailable)?
                .clone();
            let p = self.registry_observation_pool()?;
            projection::scope_bytes(s, p.limits)?;
            if self.source_facade_busy
                || !ceiling.allowed
                || admission_allowed != Some(true)
                || f.current.scope != s
                || f.current.native_profile != p.native
                || f.current.required().into_iter().any(|v| v != Some(true))
                || !f.current.ready()
                || f.receipt_grant_active != Some(true)
            {
                return Err(ResourceError::Unavailable);
            }
            if projection::frame_bytes(&result.projection)? > p.limits.max_reply_bytes {
                return Err(ResourceError::Limit);
            }
            if p.issued >= p.limits.max_issued
                || p.packets.iter().flatten().count() >= p.limits.max_preparations
            {
                return Err(ResourceError::Limit);
            }
            let stream = PacketTarget::Stream(result.stream.clone());
            let packet = Packet {
                stream,
                scope: projection::copy_scope(s)?,
                root: self.resources.catalog.stamp()?,
                command_basis: None,
                value: PacketValue::ReceiptContext(Box::new(Context {
                    command: command.clone(),
                    result,
                    ceiling,
                    stopped: false,
                })),
            };
            let p = self.registry_issue_packet(packet)?;
            Ok(ResourceRegistryReceiptContextIdV1 {
                owner: p.owner,
                slot: p.slot,
            })
        })();
        report(
            r,
            Boundary::RegistryReceiptAdmission,
            &result,
            timer.finish(),
        );
        result
    }
    /// Privileged original-slot input for the historical authorizer, never a disclosure grant.
    pub fn resource_registry_receipt_authorization_basis_v1(
        &self,
        id: &ResourceRegistryReceiptContextIdV1,
        command: &ResourceRegistryCommandIdV1,
        s: &ResourceRegistryObservationScopeV1,
        r: &mut Recorder,
    ) -> Result<ResourceRegistryReceiptAuthorizationBasisV1<'_>> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let c = self.registry_receipt_original_v1(id, command, s)?;
            if c.stopped {
                return Err(ResourceError::Unavailable);
            }
            let host = self.resource_registry_command_facts_v1(command, s, r)?;
            if host.phase != ResourceRegistryCommandPhaseV1::Settled
                || !host
                    .as_observed
                    .is_some_and(|r| r.as_ref().is_ok_and(|b| Arc::ptr_eq(b, &c.result)))
            {
                return Err(ResourceError::Faulted);
            }
            Ok(ResourceRegistryReceiptAuthorizationBasisV1 {
                slots: host.historical_slots,
                basis: ResourceRegistryReceiptAuthorizationBasisIdV1 {
                    context: id.clone(),
                    command: command.clone(),
                    result: c.result.clone(),
                    root: self
                        .native_root_identity()
                        .map_err(|_| ResourceError::Faulted)?,
                },
            })
        })();
        report(
            r,
            Boundary::RegistryReceiptHostBasis,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn prepare_resource_registry_receipt_v1(
        &mut self,
        id: &ResourceRegistryReceiptContextIdV1,
        command: &ResourceRegistryCommandIdV1,
        s: &ResourceRegistryObservationScopeV1,
        f: &TrustedResourceRegistryReceiptHistoricalFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResourceRegistryPreparationIdV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            // Correlate the authorizer's input before applying supplied loss facts.
            let c = self.registry_receipt_original_v1(id, command, s)?;
            if f.basis.context != *id
                || f.basis.command != *command
                || !Arc::ptr_eq(&f.basis.result, &c.result)
                || f.basis.root
                    != self
                        .native_root_identity()
                        .map_err(|_| ResourceError::Faulted)?
            {
                return Err(ResourceError::Unavailable);
            }
            self.registry_receipt_gate_v1(id, command, s, f.current, r)?;
            if f.current.receipt_allowed != Some(true)
                || f.current.authorization_current != Some(true)
                || f.historical_release_attested != Some(true)
                || f.current.current.resources_ready != Some(true)
            {
                return Err(ResourceError::Unavailable);
            }
            let p = self.registry_observation_pool()?;
            if p.issued >= p.limits.max_issued
                || p.packets.iter().flatten().count() >= p.limits.max_preparations
            {
                return Err(ResourceError::Limit);
            }
            let c = self.registry_receipt_original_v1(id, command, s)?;
            let result = c.result.clone();
            if f.rows.len() != result.projection.rows.len() || f.rows.len() > p.limits.max_rows {
                return Err(ResourceError::Malformed);
            }
            for (i, fact) in f.rows.iter().enumerate() {
                if (!fact.membership_allowed && fact.descriptor_allowed)
                    || f.rows[..i].iter().any(|p| p.reference == fact.reference)
                    || result
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
            let allowed = result.projection.rows.iter().all(|row| {
                f.rows.iter().any(|f| {
                    *f.reference == row.reference
                        && f.membership_allowed
                        && (row.descriptor.is_none() || f.descriptor_allowed)
                })
            });
            let packet = Packet {
                stream: PacketTarget::Stream(result.stream.clone()),
                scope: projection::copy_scope(s)?,
                root: self.resources.catalog.stamp()?,
                command_basis: Some(
                    self.native_root_identity()
                        .map_err(|_| ResourceError::Faulted)?,
                ),
                value: PacketValue::ReceiptCertificate(Certificate {
                    context: id.clone(),
                    command: command.clone(),
                    result,
                    allowed,
                    valid: true,
                }),
            };
            self.registry_issue_packet(packet)
        })();
        report(
            r,
            Boundary::RegistryReceiptPreparation,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn release_resource_registry_receipt_v1(
        &mut self,
        id: &ResourceRegistryReceiptContextIdV1,
        command: &ResourceRegistryCommandIdV1,
        s: &ResourceRegistryObservationScopeV1,
        packet: &ResourceRegistryPreparationIdV1,
        f: &TrustedResourceRegistryReceiptFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResourceRegistryReceiptReplyV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.registry_receipt_certificate_v1(packet, id, command, s)?;
            self.registry_receipt_gate_v1(id, command, s, f, r)?;
            if f.authorization_current == Some(false) {
                let State::Active(p) = &mut self.registry_observations else {
                    unreachable!()
                };
                let PacketValue::ReceiptCertificate(cert) =
                    &mut p.packets[packet.slot].as_mut().unwrap().value
                else {
                    unreachable!()
                };
                if cert.valid {
                    cert.valid = false;
                    r.record(Observation {
                        boundary: Boundary::RegistryReceiptCertificateInvalidation,
                        disposition: Disposition::Completed,
                        service_time: None,
                    });
                }
            }
            if f.receipt_allowed != Some(true)
                || f.authorization_current != Some(true)
                || !f.current.ready()
            {
                return Err(ResourceError::Unavailable);
            }
            let cert = self.registry_receipt_certificate_v1(packet, id, command, s)?;
            if !cert.valid {
                return Err(ResourceError::Unavailable);
            }
            let p = self.registry_observation_pool()?;
            if projection::frame_bytes(&cert.result.projection)? > p.limits.max_reply_bytes {
                self.registry_receipt_mut_v1(id).stopped = true;
                return Err(ResourceError::Faulted);
            }
            let cert = self.registry_take_packet(packet);
            let PacketValue::ReceiptCertificate(cert) = cert.value else {
                unreachable!()
            };
            if !cert.allowed {
                return Err(ResourceError::Unavailable);
            }
            Ok(ResourceRegistryReceiptReplyV1 {
                receipt: cert.result,
            })
        })();
        report(r, Boundary::RegistryReceiptRelease, &result, timer.finish());
        result
    }
    /// Correlation/integrity only; this validates no recipient permission.
    pub fn validate_resource_registry_receipt_reply_v1(
        &mut self,
        id: &ResourceRegistryReceiptContextIdV1,
        command: &ResourceRegistryCommandIdV1,
        s: &ResourceRegistryObservationScopeV1,
        reply: &ResourceRegistryReceiptReplyV1,
        r: &mut Recorder,
    ) -> Result<()> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let c = self.registry_receipt_original_v1(id, command, s)?;
            if c.stopped {
                return Err(ResourceError::Unavailable);
            }
            let host = self.resource_registry_command_facts_v1(command, s, r)?;
            if host.phase != ResourceRegistryCommandPhaseV1::Settled
                || !host
                    .as_observed
                    .is_some_and(|r| r.as_ref().is_ok_and(|b| Arc::ptr_eq(b, &c.result)))
                || !Arc::ptr_eq(&reply.receipt, &c.result)
                || projection::frame_bytes(&reply.receipt.projection)?
                    > self.registry_observation_pool()?.limits.max_reply_bytes
            {
                self.registry_receipt_mut_v1(id).stopped = true;
                return Err(ResourceError::Faulted);
            }
            Ok(())
        })();
        report(
            r,
            Boundary::RegistryReceiptReplyValidation,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn stop_resource_registry_receipt_v1(
        &mut self,
        id: &ResourceRegistryReceiptContextIdV1,
        c: &ResourceRegistryObservationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<()> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.registry_receipt_control_v1(id, c.scope)?;
            if c.owner_control_allowed != Some(true) {
                return Err(ResourceError::Unavailable);
            }
            self.registry_receipt_mut_v1(id).stopped = true;
            Ok(())
        })();
        report(r, Boundary::RegistryReceiptStop, &result, timer.finish());
        result
    }
    pub fn retire_resource_registry_receipt_v1(
        &mut self,
        id: &ResourceRegistryReceiptContextIdV1,
        c: &ResourceRegistryObservationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<()> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let context = self.registry_receipt_control_v1(id, c.scope)?;
            if !context.stopped
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
            let p = self.registry_observation_pool()?;
            if p.packets.iter().flatten().any(
                |p| matches!(&p.value,PacketValue::ReceiptCertificate(cert) if cert.context==*id),
            ) {
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
            Boundary::RegistryReceiptRetirement,
            &result,
            timer.finish(),
        );
        result
    }
    /// Trusted native pressure gauges, not scoped recipient progress or grants.
    pub fn resource_registry_receipt_usage_v1(&self) -> Result<ResourceRegistryReceiptUsageV1> {
        let p = self.registry_observation_pool()?;
        let mut u = ResourceRegistryReceiptUsageV1::default();
        for packet in p.packets.iter().flatten() {
            match &packet.value {
                PacketValue::ReceiptContext(c) => {
                    u.contexts = add(u.contexts, 1)?;
                    if c.stopped {
                        u.stopped = add(u.stopped, 1)?;
                    }
                }
                PacketValue::ReceiptCertificate(_) => u.certificates = add(u.certificates, 1)?,
                _ => {}
            }
        }
        Ok(u)
    }
}
