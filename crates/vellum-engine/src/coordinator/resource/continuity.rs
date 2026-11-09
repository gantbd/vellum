//! Original-anchor own-write continuity; no destination probe refresh or retry.
use super::*;
mod shutdown_inventory;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Native logical metadata projections, not wire sizes or physical preallocation.
/// Codec payloads/condition snapshots keep their existing separate bounds.
pub struct SourceContinuityLimitsV1 {
    /// Maximum written edges in one materialized derivation.
    pub max_edges: usize,
    /// Native proof header, context and actual version/edge byte incidences.
    pub max_derivation_bytes: usize,
    /// Expanded dispatch/evidence metadata, including widest valid causal fields.
    pub max_receipt_bytes: usize,
    /// Sum of all live tickets' derivation plus receipt credit obligations.
    pub max_retained_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shutdown_inventory_accounts_for_original_continuity_credit_not_payload_or_parent_bytes() {
        use crate::coordinator::shutdown::tests::inventory_owner;
        let l = generous();
        let (mut o, mut r, _, d) = ready(l);
        o.queue_own_chain_save_v1(&d, req(), None, &mut r).unwrap();
        let credit = o.resource_usage().continuity_retained_bytes;
        let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::SourceContinuity, &[]);
        assert_eq!(row.retained_records, Some(1));
        assert_eq!(row.reserved_bytes, None);
        assert_eq!(row.local_reserved_bytes, Some(credit));
        assert!(credit > 0);
        assert_eq!(row.held_bytes, None);
        assert_eq!(row.local_byte_limit, Some(l.max_retained_bytes));
    }
    fn v(token: &[u8]) -> ResourceVersionV1<'_> {
        ResourceVersionV1 {
            epoch: b"epoch",
            token,
        }
    }
    fn generous() -> SourceContinuityLimitsV1 {
        SourceContinuityLimitsV1 {
            max_edges: 8,
            max_derivation_bytes: 65536,
            max_receipt_bytes: 65536,
            max_retained_bytes: 262144,
        }
    }
    fn ready(
        l: SourceContinuityLimitsV1,
    ) -> (SourceCoordinator, Recorder, ProviderPort, DocumentVersion) {
        let (mut o, mut r, _, _) = super::super::serialization_tests::fixture();
        let p = o
            .register_provider_with_causal_capabilities(
                ProviderProfile {
                    synchronization_supported: false,
                },
                ProviderDestinationCapabilitiesV1::default(),
                ProviderCausalCapabilitiesV1 {
                    causal_receipts: true,
                    non_reuse_tokens: true,
                },
                None,
                &mut r,
            )
            .unwrap();
        o.configure_source_continuity_v1(l, &mut r).unwrap();
        let d = o
            .load_resource(
                p.read_result_causal(b"typed", v(b"v0"), b"A").unwrap(),
                &mut r,
            )
            .unwrap();
        (o, r, p, d)
    }
    fn req() -> SaveRequirements {
        SaveRequirements {
            protection: Protection::BestEffort,
            acknowledgment: Acknowledgment::Ordinary,
        }
    }
    #[test]
    fn interrupted_own_chain_admission_keeps_installed_credit_and_never_clears_the_fence() {
        for cut in 0..5 {
            let (mut o, mut r, _, d) = ready(generous());
            let tasks = o.resources.tasks.len();
            let artifacts = o.resources.artifacts.len();
            o.publication_fault_after = Some(cut);
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    o.queue_own_chain_save_v1(&d, req(), None, &mut r)
                }))
                .is_err()
            );
            assert!(o.is_faulted());
            assert_eq!(
                o.resources.artifacts.len(),
                artifacts + usize::from(cut >= 1)
            );
            assert_eq!(o.resources.tasks.len(), tasks + usize::from(cut >= 2));
            if cut >= 2 {
                let credit = o.resources.tasks[tasks]
                    .as_ref()
                    .unwrap()
                    .continuity
                    .as_ref()
                    .unwrap()
                    .charge;
                assert!(credit > 0);
                assert_eq!(
                    o.resources.continuity.charged,
                    if cut >= 3 { credit } else { 0 }
                );
            } else {
                assert_eq!(o.resources.continuity.charged, 0);
            }
        }
    }
    #[test]
    fn exact_minimum_derivation_receipt_and_total_credit_boundaries_refuse_before_issuance() {
        let (probe, _, _, d) = ready(generous());
        let b = probe
            .resources
            .bindings
            .iter()
            .find(|b| b.document == d.document)
            .unwrap();
        let c = probe.prepare_continuity(b).unwrap();
        let overhead = std::mem::size_of::<SaveEvidence>()
            + std::mem::size_of::<SaveDispatch>()
            + 4 * b.registration.limits.max_validator_bytes;
        let derivation = (c.charge - overhead) / 2;
        let receipt = derivation + overhead;
        for field in 0..3 {
            for delta in [-1, 0, 1] {
                let mut l = generous();
                let exact = match field {
                    0 => derivation,
                    1 => receipt,
                    _ => c.charge,
                };
                *match field {
                    0 => &mut l.max_derivation_bytes,
                    1 => &mut l.max_receipt_bytes,
                    _ => &mut l.max_retained_bytes,
                } = exact.checked_add_signed(delta).unwrap();
                let (mut o, mut r, _, d) = ready(l);
                let before = o.resource_usage();
                let out = o.queue_own_chain_save_v1(&d, req(), None, &mut r);
                if delta < 0 {
                    assert_eq!(out, Err(ResourceError::Limit));
                    assert_eq!(o.resource_usage(), before);
                } else {
                    let id = out.unwrap();
                    assert_eq!(o.resource_usage().continuity_retained_bytes, c.charge);
                    o.stop_save(&id, &mut r).unwrap();
                    o.retire_save(&id, &mut r).unwrap();
                    assert_eq!(o.resource_usage().continuity_retained_bytes, 0);
                }
            }
        }
    }
    #[test]
    fn expanded_receipt_and_derivation_caps_refuse_without_serial_or_credit_installation() {
        let (probe, _, _, d) = ready(generous());
        let b = probe
            .resources
            .bindings
            .iter()
            .find(|b| b.document == d.document)
            .unwrap();
        let c = probe.prepare_continuity(b).unwrap();
        let overhead = std::mem::size_of::<SaveEvidence>()
            + std::mem::size_of::<SaveDispatch>()
            + 4 * b.registration.limits.max_validator_bytes;
        let derivation = (c.charge - overhead) / 2;
        for receipt in [false, true] {
            let mut l = generous();
            if receipt {
                l.max_receipt_bytes = derivation + overhead
            } else {
                l.max_derivation_bytes = derivation
            }
            let (mut o, mut r, p, d) = ready(l);
            let a = o.queue_pinned_save(&d, req(), &mut r).unwrap();
            let own = o.queue_own_chain_save_v1(&d, req(), None, &mut r).unwrap();
            let a = o
                .dispatch_save(
                    &a,
                    p.read_result_causal(b"typed", v(b"v0"), b"A").unwrap(),
                    &mut r,
                )
                .unwrap();
            for e in [
                p.causal_edge(&a, v(b"v0"), v(b"v1")).unwrap(),
                p.result_causal(
                    &a,
                    Application::Written,
                    1,
                    Synchronization::NotRequested,
                    Some(v(b"v1")),
                )
                .unwrap(),
                p.no_future_effects(&a).unwrap(),
            ] {
                o.record_save_evidence(e, &mut r).unwrap();
            }
            let before = o.resource_usage();
            let serial = o.resources.last_dispatch_serial;
            assert_eq!(
                o.dispatch_own_chain_save_v1(
                    &own,
                    p.read_result_causal(b"typed", v(b"v1"), b"A").unwrap(),
                    None,
                    &TrustedSourceContinuityLaunchV1 {
                        causal_progression_allowed: Some(true)
                    },
                    &mut r
                )
                .unwrap_err(),
                ResourceError::Limit
            );
            assert_eq!(o.resource_usage(), before);
            assert_eq!(o.resources.last_dispatch_serial, serial);
            assert!(o.resource_task(&own).unwrap().launched_condition.is_none());
        }
    }
    #[test]
    fn exhausted_launch_counter_and_continuity_debt_drift_do_not_issue_or_refund() {
        for mode in ["counter", "credit", "root-fault"] {
            let (mut o, mut r, p, d) = ready(generous());
            let own = o.queue_own_chain_save_v1(&d, req(), None, &mut r).unwrap();
            match mode {
                "counter" => o.resources.last_dispatch_serial = u64::MAX,
                "credit" => o.resources.continuity.charged += 1,
                _ => o.faulted = true,
            }
            let charge = o.resources.continuity.charged;
            let artifacts = o.resource_usage().retained_artifacts;
            assert_eq!(
                o.dispatch_own_chain_save_v1(
                    &own,
                    p.read_result_causal(b"typed", v(b"v0"), b"A").unwrap(),
                    None,
                    &TrustedSourceContinuityLaunchV1 {
                        causal_progression_allowed: None
                    },
                    &mut r
                )
                .unwrap_err(),
                if mode == "counter" {
                    ResourceError::VersionExhausted
                } else {
                    ResourceError::Faulted
                }
            );
            assert_eq!(o.resources.continuity.charged, charge);
            assert_eq!(o.resource_usage().retained_artifacts, artifacts);
            assert_eq!(o.resource_task(&own).unwrap().dispatch_serial, None);
            if mode == "credit" {
                assert!(o.is_faulted());
                o.stop_save(&own, &mut r).unwrap();
                assert_eq!(o.retire_save(&own, &mut r), Err(ResourceError::Faulted));
                assert_eq!(o.resources.continuity.charged, charge);
            }
        }
    }
}
#[derive(Clone, Copy)]
pub struct TrustedSourceContinuityLaunchV1 {
    pub causal_progression_allowed: Option<bool>,
}
#[derive(Debug, Default)]
pub(super) struct State {
    pub(super) limits: Option<SourceContinuityLimitsV1>,
    pub(super) charged: usize,
}
#[derive(Debug)]
pub(super) struct Pending {
    pub(super) anchor_serial: u64,
    pub(super) charge: usize,
    pub(super) receipt: Option<Arc<SourceContinuityReceiptV1>>,
}
pub(super) struct Plan {
    pub(super) condition: WriteCondition,
    pub(super) charge: usize,
    edges: usize,
}
fn sum(values: &[usize]) -> Result<usize, ResourceError> {
    values
        .iter()
        .try_fold(0usize, |n, v| n.checked_add(*v).ok_or(ResourceError::Limit))
}
fn version(v: &CausalVersion) -> Result<usize, ResourceError> {
    sum(&[v.epoch.0.len(), v.token.0.len()])
}
fn original(c: &WriteCondition) -> Result<CausalVersion, ResourceError> {
    match c {
        WriteCondition::Present(b) => Ok(CausalVersion {
            epoch: b.epoch.clone().ok_or(ResourceError::Unsupported)?,
            token: b.validator.clone().ok_or(ResourceError::MissingEvidence)?,
        }),
        WriteCondition::Absent => Err(ResourceError::Unsupported),
    }
}
fn costs(
    reg: &Registration,
    key: usize,
    initial: &CausalVersion,
    head: &CausalVersion,
    edge_bytes: usize,
    l: SourceContinuityLimitsV1,
) -> Result<usize, ResourceError> {
    let derivation = sum(&[
        std::mem::size_of::<SourceContinuityReceiptV1>(),
        key,
        version(initial)?,
        version(head)?,
        edge_bytes,
    ])?;
    // Native metadata projection, not JSON/wire/physical allocation. Fund one
    // worst-width valid causal fact (four bounded epoch/token fields), plus the
    // complete capture/dispatch descriptor and actual derivation context.
    let fields = reg
        .limits
        .max_validator_bytes
        .checked_mul(4)
        .ok_or(ResourceError::Limit)?;
    let receipt = sum(&[
        std::mem::size_of::<SaveEvidence>(),
        std::mem::size_of::<SaveDispatch>(),
        derivation,
        fields,
    ])?;
    if derivation > l.max_derivation_bytes || receipt > l.max_receipt_bytes {
        return Err(ResourceError::Limit);
    }
    sum(&[derivation, receipt])
}
fn edge_bytes(edge: &CausalEdge) -> Result<usize, ResourceError> {
    sum(&[
        std::mem::size_of::<SourceContinuityEdgeV1>(),
        version(&edge.before)?,
        version(&edge.after)?,
    ])
}
impl SourceCoordinator {
    pub fn configure_source_continuity_v1(
        &mut self,
        limits: SourceContinuityLimitsV1,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let out = (|| {
            self.ensure_operational().map_err(owner_error)?;
            self.validate_continuity_credit()?;
            match self.resources.continuity.limits {
                Some(old) if old != limits => Err(ResourceError::Unsupported),
                Some(_) => Ok(()),
                None => {
                    self.ensure_admission_unfenced().map_err(owner_error)?;
                    self.resources.continuity.limits = Some(limits);
                    Ok(())
                }
            }
        })();
        capture(r, Boundary::SourceContinuityConfiguration, &out);
        out
    }
    pub fn queue_own_chain_save_v1(
        &mut self,
        expected: &DocumentVersion,
        requirements: SaveRequirements,
        pair: Option<OrdinaryPairEvidence>,
        r: &mut Recorder,
    ) -> Result<SaveId, ResourceError> {
        self.queue_pinned_save_checked(expected, requirements, pair.as_ref(), true, None, r)
    }
    pub fn dispatch_own_chain_save_v1(
        &mut self,
        id: &SaveId,
        probe: ResourceRead,
        pair: Option<OrdinaryPairEvidence>,
        facts: &TrustedSourceContinuityLaunchV1,
        r: &mut Recorder,
    ) -> Result<SaveDispatch, ResourceError> {
        let out = self
            .resource_task(id)
            .and_then(|t| {
                if t.continuity.is_some() {
                    Ok(())
                } else {
                    Err(ResourceError::Unsupported)
                }
            })
            .and_then(|()| {
                self.dispatch_save_checked(
                    id,
                    probe.into_source_export_destination(),
                    pair.as_ref(),
                    None,
                    Some(facts),
                    r,
                )
            });
        capture(r, Boundary::SourceContinuityDerivation, &out);
        out
    }
    fn validate_continuity_credit(&self) -> Result<(), ResourceError> {
        let mut total = 0usize;
        for t in self.resources.tasks.iter().flatten() {
            if let Some(c) = &t.continuity {
                let l = self
                    .resources
                    .continuity
                    .limits
                    .ok_or(ResourceError::Faulted)?;
                if t.purpose != SavePurpose::OrdinarySource {
                    return Err(ResourceError::Faulted);
                }
                if (t.phase == SavePhase::Dispatched) != c.receipt.is_some() {
                    return Err(ResourceError::Faulted);
                }
                let initial = original(&t.condition).map_err(|_| ResourceError::Faulted)?;
                let expected = if let Some(p) = &c.receipt {
                    if p.source != t.captured
                        || Some(p.serialization) != t.serialization
                        || p.binding_generation != t.binding_generation
                        || p.identity != t.identity
                        || !Arc::ptr_eq(&p.registration, &t.registration)
                        || Some(p.serial) != t.dispatch_serial
                        || p.anchor_serial != c.anchor_serial
                        || p.original != initial
                        || p.edges.len() > l.max_edges
                    {
                        return Err(ResourceError::Faulted);
                    }
                    let Some(DispatchCondition::Present {
                        epoch: Some(epoch),
                        validator,
                        ..
                    }) = &t.launched_condition
                    else {
                        return Err(ResourceError::Faulted);
                    };
                    if *epoch != p.derived.epoch || *validator != p.derived.token {
                        return Err(ResourceError::Faulted);
                    }
                    let bytes = p
                        .edges
                        .iter()
                        .try_fold(0usize, |n, e| sum(&[n, edge_bytes(&e.versions)?]))
                        .map_err(|_| ResourceError::Faulted)?;
                    costs(
                        &t.registration,
                        t.identity.key.0.len(),
                        &initial,
                        &p.derived,
                        bytes,
                        l,
                    )
                    .map_err(|_| ResourceError::Faulted)?
                } else {
                    costs(
                        &t.registration,
                        t.identity.key.0.len(),
                        &initial,
                        &initial,
                        0,
                        l,
                    )
                    .map_err(|_| ResourceError::Faulted)?
                };
                if c.charge != expected {
                    return Err(ResourceError::Faulted);
                }
                total = total.checked_add(c.charge).ok_or(ResourceError::Faulted)?;
            }
        }
        if total != self.resources.continuity.charged
            || self
                .resources
                .continuity
                .limits
                .is_some_and(|l| total > l.max_retained_bytes)
        {
            return Err(ResourceError::Faulted);
        }
        Ok(())
    }
    pub(super) fn prepare_continuity(&self, b: &Binding) -> Result<Pending, ResourceError> {
        self.validate_continuity_credit()?;
        let l = self
            .resources
            .continuity
            .limits
            .ok_or(ResourceError::Unsupported)?;
        let baseline = b.baseline.as_ref().ok_or(ResourceError::MissingEvidence)?;
        let v = original(&WriteCondition::Present(baseline.clone()))?;
        let anchor_serial = if let Some(anchor) = &baseline.anchor {
            let t = self
                .resource_task(anchor)
                .map_err(|_| ResourceError::Faulted)?;
            if !t.source_baseline_candidate()
                || t.captured.document != b.document
                || t.binding_generation != b.generation
                || t.identity != b.identity
                || !Arc::ptr_eq(&t.registration, &b.registration)
                || !t.qualified()
                || t.payload != baseline.bytes
                || t.knowledge.data.version != baseline.validator
                || t.knowledge.data.epoch != baseline.epoch
            {
                return Err(ResourceError::Faulted);
            }
            self.validate_source_capture(t)?;
            t.dispatch_serial
                .filter(|n| *n > 0)
                .ok_or(ResourceError::Faulted)?
        } else {
            0
        };
        let charge = costs(&b.registration, b.identity.key.0.len(), &v, &v, 0, l)?;
        if sum(&[self.resources.continuity.charged, charge])? > l.max_retained_bytes {
            return Err(ResourceError::Limit);
        }
        Ok(Pending {
            anchor_serial,
            charge,
            receipt: None,
        })
    }
    pub(super) fn install_continuity_credit(&mut self) {
        let c = self
            .resources
            .tasks
            .last()
            .and_then(Option::as_ref)
            .and_then(|t| t.continuity.as_ref())
            .expect("preflighted continuity admission");
        self.resources.continuity.charged += c.charge;
    }
    fn walk_continuity(
        &self,
        t: &Task,
        pending: &Pending,
        progression: Option<bool>,
        mut visit: impl FnMut(usize, &Task, &CausalEdge) -> Result<(), ResourceError>,
    ) -> Result<(Baseline, usize, usize), ResourceError> {
        let l = self
            .resources
            .continuity
            .limits
            .ok_or(ResourceError::Faulted)?;
        let initial = original(&t.condition)?;
        let WriteCondition::Present(base) = &t.condition else {
            return Err(ResourceError::Unsupported);
        };
        let mut head = initial;
        let mut bytes = base.bytes.clone();
        let mut count = 0usize;
        let mut metadata = 0usize;
        let mut previous = pending.anchor_serial;
        for (slot, peer) in self
            .resources
            .tasks
            .iter()
            .enumerate()
            .filter_map(|(i, t)| t.as_ref().map(|t| (i, t)))
        {
            if peer.identity != t.identity || peer.phase != SavePhase::Dispatched {
                continue;
            }
            let serial = peer
                .dispatch_serial
                .filter(|s| *s > 0)
                .ok_or(ResourceError::Faulted)?;
            if serial <= pending.anchor_serial {
                continue;
            }
            if serial <= previous {
                return Err(ResourceError::Faulted);
            }
            previous = serial;
            if !peer.safe() {
                return Err(ResourceError::Blocked);
            }
            match peer.knowledge.data.application {
                Application::NotWritten => continue,
                Application::Unknown => return Err(ResourceError::Blocked),
                Application::Written => {}
            }
            if progression != Some(true) {
                return Err(ResourceError::Permission);
            }
            if !t.registration.causal.causal_receipts || !t.registration.causal.non_reuse_tokens {
                return Err(ResourceError::Unsupported);
            }
            if peer.kind != ArtifactKind::Source
                || peer.captured.document != t.captured.document
                || peer.binding_generation != t.binding_generation
                || !Arc::ptr_eq(&peer.registration, &t.registration)
            {
                return Err(ResourceError::Conflict);
            }
            self.validate_source_capture(peer)?;
            let edge = peer.causal.edge().ok_or(ResourceError::MissingEvidence)?;
            if edge.before != head
                || edge.before.epoch != edge.after.epoch
                || edge.before.token == edge.after.token
                || peer.knowledge.data.version.as_ref() != Some(&edge.after.token)
                || peer.knowledge.data.epoch.as_ref() != Some(&edge.after.epoch)
            {
                return Err(ResourceError::Conflict);
            }
            let Some(DispatchCondition::Present {
                validator,
                epoch: Some(epoch),
                ..
            }) = &peer.launched_condition
            else {
                return Err(ResourceError::Faulted);
            };
            if *validator != edge.before.token || *epoch != edge.before.epoch {
                return Err(ResourceError::Faulted);
            }
            // Also reject an observed multi-edge token cycle, rather than using
            // the host's non-reuse claim to ignore known reuse in this prefix.
            if original(&t.condition)?.token == edge.after.token
                || self.resources.tasks.iter().flatten().any(|p| {
                    p.identity == t.identity
                        && p.dispatch_serial
                            .is_some_and(|s| s > pending.anchor_serial && s < serial)
                        && p.causal.edge().is_some_and(|e| e.after == edge.after)
                })
            {
                return Err(ResourceError::Conflict);
            }
            count = count.checked_add(1).ok_or(ResourceError::Limit)?;
            if count > l.max_edges {
                return Err(ResourceError::Limit);
            }
            metadata = sum(&[metadata, edge_bytes(edge)?])?;
            visit(slot, peer, edge)?;
            head = edge.after.clone();
            bytes = peer.payload.clone();
        }
        Ok((
            Baseline {
                epoch: Some(head.epoch),
                validator: Some(head.token),
                bytes,
                anchor: None,
            },
            count,
            metadata,
        ))
    }
    pub(super) fn derive_continuity(
        &self,
        _id: &SaveId,
        t: &Task,
        facts: Option<&TrustedSourceContinuityLaunchV1>,
    ) -> Result<Option<Plan>, ResourceError> {
        let Some(c) = &t.continuity else {
            return Ok(None);
        };
        self.validate_continuity_credit()?;
        let (condition, edges, bytes) = self.walk_continuity(
            t,
            c,
            facts.and_then(|f| f.causal_progression_allowed),
            |_, _, _| Ok(()),
        )?;
        let initial = original(&t.condition)?;
        let head = original(&WriteCondition::Present(condition.clone()))?;
        let l = self
            .resources
            .continuity
            .limits
            .ok_or(ResourceError::Faulted)?;
        let charge = costs(
            &t.registration,
            t.identity.key.0.len(),
            &initial,
            &head,
            bytes,
            l,
        )?;
        let current = self
            .resources
            .continuity
            .charged
            .checked_sub(c.charge)
            .ok_or(ResourceError::Faulted)?;
        if sum(&[current, charge])? > l.max_retained_bytes {
            return Err(ResourceError::Limit);
        }
        Ok(Some(Plan {
            condition: WriteCondition::Present(condition),
            charge,
            edges,
        }))
    }
    pub(super) fn materialize_continuity(
        &self,
        t: &Task,
        serial: u64,
        plan: Option<&Plan>,
        facts: Option<&TrustedSourceContinuityLaunchV1>,
    ) -> Result<Option<Arc<SourceContinuityReceiptV1>>, ResourceError> {
        let Some(plan) = plan else {
            return Ok(None);
        };
        let c = t.continuity.as_ref().ok_or(ResourceError::Faulted)?;
        let mut edges = Vec::new();
        edges
            .try_reserve_exact(plan.edges)
            .map_err(|_| ResourceError::Capacity)?;
        let (head, count, _) = self.walk_continuity(
            t,
            c,
            facts.and_then(|f| f.causal_progression_allowed),
            |slot, p, e| {
                edges.push(SourceContinuityEdgeV1 {
                    producer: SaveId {
                        owner: self.namespace.clone(),
                        slot,
                    },
                    serial: p.dispatch_serial.ok_or(ResourceError::Faulted)?,
                    versions: e.clone(),
                });
                Ok(())
            },
        )?;
        if count != plan.edges {
            return Err(ResourceError::Faulted);
        }
        Ok(Some(Arc::new(SourceContinuityReceiptV1 {
            registration: t.registration.clone(),
            identity: t.identity.clone(),
            source: t.captured.clone(),
            serialization: t.serialization.ok_or(ResourceError::Faulted)?,
            binding_generation: t.binding_generation,
            anchor_serial: c.anchor_serial,
            serial,
            original: original(&t.condition)?,
            derived: original(&WriteCondition::Present(head))?,
            edges,
        })))
    }
    pub(super) fn continuity_retirement_remaining(&self, t: &Task) -> Result<usize, ResourceError> {
        self.validate_continuity_credit()?;
        self.resources
            .continuity
            .charged
            .checked_sub(t.continuity.as_ref().map_or(0, |c| c.charge))
            .ok_or(ResourceError::Faulted)
    }
}
