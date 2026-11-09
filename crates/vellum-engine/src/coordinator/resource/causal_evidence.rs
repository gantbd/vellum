//! Protected causal facts have a fixed witness cap, not arbitrary join closure.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
enum Atom {
    Edge(CausalEdge),
    Result(ResultData),
}
#[derive(Debug, Default)]
pub(super) struct CausalKnowledge {
    edge: Option<CausalEdge>,
    conflicts: [Option<Atom>; 2],
}
impl Task {
    pub(super) fn contested(&self) -> bool {
        self.knowledge.conflict.is_some() || self.causal.conflicts.iter().any(Option::is_some)
    }
    fn causal_conflict(&mut self, atom: Atom) -> Result<EvidenceDisposition, ResourceError> {
        let known = match &atom {
            Atom::Edge(e) => self.causal.edge.as_ref() == Some(e),
            Atom::Result(r) => {
                self.knowledge.data == *r
                    || self.knowledge.witnesses.iter().flatten().any(|w| w == r)
            }
        };
        if known || self.causal.conflicts.iter().flatten().any(|w| w == &atom) {
            return Ok(EvidenceDisposition::Duplicate);
        }
        let slot = self
            .causal
            .conflicts
            .iter_mut()
            .find(|s| s.is_none())
            .ok_or(ResourceError::EvidenceCapacity)?;
        *slot = Some(atom);
        Ok(EvidenceDisposition::Contradiction)
    }
    pub(super) fn merge_causal_edge(
        &mut self,
        edge: CausalEdge,
    ) -> Result<EvidenceDisposition, ResourceError> {
        if !self.registration.causal.causal_receipts
            || self.kind != ArtifactKind::Source
            || self.result_version_profile != SaveResultVersionProfileV1::EpochToken
        {
            return Err(ResourceError::Unsupported);
        }
        let launched = self
            .launched_condition
            .as_ref()
            .ok_or(ResourceError::Faulted)?;
        if matches!(launched, DispatchCondition::Absent) {
            // A concrete before-version contradicts the attested absence. Keep
            // the observation as evidence; never manufacture a creation edge.
            return self.causal_conflict(Atom::Edge(edge));
        }
        let DispatchCondition::Present {
            validator: token,
            epoch,
            ..
        } = launched
        else {
            return Err(ResourceError::Unsupported);
        };
        let Some(epoch) = epoch else {
            return Err(ResourceError::Unsupported);
        };
        if self.causal.edge.as_ref() == Some(&edge) {
            return Ok(EvidenceDisposition::Duplicate);
        }
        if edge.before.epoch != *epoch
            || edge.before.token != *token
            || self.causal.edge.is_some()
            || self.knowledge.data.application == Application::NotWritten
            || self.knowledge.data.version.as_ref().is_some_and(|v| {
                *v != edge.after.token
                    || self.knowledge.data.epoch.as_ref() != Some(&edge.after.epoch)
            })
        {
            return self.causal_conflict(Atom::Edge(edge));
        }
        self.causal.edge = Some(edge);
        Ok(EvidenceDisposition::Accepted)
    }
    pub(super) fn merge_causal_result(
        &mut self,
        result: ResultData,
    ) -> Result<EvidenceDisposition, ResourceError> {
        if result.epoch.is_some() && result.version.is_none() {
            return Err(ResourceError::Malformed);
        }
        let typed = self.result_version_profile == SaveResultVersionProfileV1::EpochToken;
        match &self.launched_condition {
            Some(DispatchCondition::Present { epoch, .. }) if epoch.is_some() != typed => {
                return Err(ResourceError::Faulted);
            }
            Some(_) => {}
            None => return Err(ResourceError::Faulted),
        }
        if result.version.is_some() && typed != result.epoch.is_some()
            || result.epoch.is_some() && !self.registration.causal.causal_receipts
        {
            return Err(ResourceError::Unsupported);
        }
        if let Some(edge) = &self.causal.edge
            && (result.application == Application::NotWritten
                || result.version.as_ref().is_some_and(|v| {
                    *v != edge.after.token || result.epoch.as_ref() != Some(&edge.after.epoch)
                }))
        {
            return self.causal_conflict(Atom::Result(result));
        }
        if typed {
            // One shared two-fact contradiction cap across typed result/edge
            // atoms. Trial merge is fixed-size Arc metadata, not payload copying;
            // a full witness store cannot partly mutate accepted result facts.
            let mut candidate = self.knowledge.clone();
            match candidate.merge(result.clone())? {
                EvidenceDisposition::Contradiction => self.causal_conflict(Atom::Result(result)),
                disposition @ (EvidenceDisposition::Accepted | EvidenceDisposition::Duplicate) => {
                    self.knowledge = candidate;
                    Ok(disposition)
                }
            }
        } else {
            self.knowledge.merge(result)
        }
    }
}
impl CausalKnowledge {
    pub(super) fn edge(&self) -> Option<&CausalEdge> {
        self.edge.as_ref()
    }
    pub(super) fn edge_available(&self) -> bool {
        self.edge.is_some()
    }
    pub(super) fn conflicts(&self) -> usize {
        self.conflicts.iter().flatten().count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ready() -> (
        SourceCoordinator,
        Recorder,
        ProviderPort,
        SaveId,
        SaveDispatch,
    ) {
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
        let v = ResourceVersionV1 {
            epoch: b"epoch",
            token: b"v0",
        };
        let d = o
            .load_resource(p.read_result_causal(b"typed", v, b"A").unwrap(), &mut r)
            .unwrap();
        let id = o
            .queue_pinned_save(
                &d,
                SaveRequirements {
                    protection: Protection::BestEffort,
                    acknowledgment: Acknowledgment::Ordinary,
                },
                &mut r,
            )
            .unwrap();
        let dispatch = o
            .dispatch_save(
                &id,
                p.read_result_causal(b"typed", v, b"A").unwrap(),
                &mut r,
            )
            .unwrap();
        (o, r, p, id, dispatch)
    }
    #[test]
    fn immutable_result_profile_drift_refuses_without_overwriting_protected_facts() {
        let (mut o, mut r, p, id, dispatch) = ready();
        let result = p
            .result_causal(
                &dispatch,
                Application::Written,
                1,
                Synchronization::NotRequested,
                Some(ResourceVersionV1 {
                    epoch: b"epoch",
                    token: b"v1",
                }),
            )
            .unwrap();
        o.resources.tasks[id.slot]
            .as_mut()
            .unwrap()
            .result_version_profile = SaveResultVersionProfileV1::Opaque;
        let before = o.save_outcome(&id, &mut r).unwrap();
        assert_eq!(
            o.record_save_evidence(result, &mut r),
            Err(ResourceError::Faulted)
        );
        assert_eq!(o.save_outcome(&id, &mut r).unwrap(), before);
    }
    #[test]
    fn protected_causal_ingress_rejects_missing_epoch_and_accepts_original_facts_under_a_root_fault()
     {
        for faulted in [false, true] {
            let (mut o, mut r, p, id, dispatch) = ready();
            o.faulted = faulted;
            let valid = p
                .result_causal(
                    &dispatch,
                    Application::Written,
                    1,
                    Synchronization::NotRequested,
                    Some(ResourceVersionV1 {
                        epoch: b"epoch",
                        token: b"v1",
                    }),
                )
                .unwrap();
            let mut malformed = valid.clone();
            let EvidenceKind::Result(data) = &mut malformed.kind else {
                panic!("result")
            };
            data.epoch = None;
            let before = o.save_outcome(&id, &mut r).unwrap();
            assert_eq!(
                o.record_save_evidence(malformed, &mut r),
                Err(ResourceError::Unsupported)
            );
            assert_eq!(o.save_outcome(&id, &mut r).unwrap(), before);
            assert_eq!(
                o.record_save_evidence(valid, &mut r),
                Ok(EvidenceDisposition::Accepted)
            );
            assert_eq!(o.is_faulted(), faulted);
        }
    }
    #[test]
    fn native_root_preflight_counts_typed_epoch_payload_before_source_birth() {
        let ready = || {
            let (mut o, mut r, _, _) = super::super::serialization_tests::fixture();
            let p = o
                .register_provider_with_causal_capabilities(
                    ProviderProfile {
                        synchronization_supported: false,
                    },
                    ProviderDestinationCapabilitiesV1::default(),
                    ProviderCausalCapabilitiesV1 {
                        causal_receipts: true,
                        non_reuse_tokens: false,
                    },
                    None,
                    &mut r,
                )
                .unwrap();
            (o, r, p)
        };
        let (mut probe, mut r, p) = ready();
        probe
            .load_resource(
                p.read_result_causal(
                    b"typed",
                    ResourceVersionV1 {
                        epoch: b"epoch",
                        token: b"v0",
                    },
                    b"A",
                )
                .unwrap(),
                &mut r,
            )
            .unwrap();
        let exact = probe.semantic_root_usage(&mut r).unwrap().total_bytes;
        for delta in [-1, 0, 1] {
            let (mut o, mut r, p) = ready();
            o.configure_semantic_root_budget(exact.checked_add_signed(delta).unwrap(), &mut r)
                .unwrap();
            let usage = o.resource_usage();
            let generation = o.generation;
            let result = o.load_resource(
                p.read_result_causal(
                    b"typed",
                    ResourceVersionV1 {
                        epoch: b"epoch",
                        token: b"v0",
                    },
                    b"A",
                )
                .unwrap(),
                &mut r,
            );
            if delta < 0 {
                assert_eq!(result, Err(ResourceError::Limit));
                assert_eq!(o.resource_usage(), usage);
                assert_eq!(o.generation, generation);
            } else {
                result.unwrap();
                assert_eq!(o.semantic_root_usage(&mut r).unwrap().total_bytes, exact);
            }
        }
    }
}
