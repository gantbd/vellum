//! Complete engine-owned ordinary pair inventory, one serialized writer.
use super::*;
impl SourceCoordinator {
    fn current_pair_keys(&self, document: &DocumentId) -> (Option<BindingKey>, Option<BindingKey>) {
        (
            self.resources
                .bindings
                .iter()
                .find(|b| b.document == *document)
                .map(Binding::key),
            self.resources
                .annotation_bindings
                .iter()
                .find(|b| b.document == *document)
                .map(|b| b.key.clone()),
        )
    }
    fn pair_check(&self, document: &DocumentId, proposed: Option<BindingKey>) -> OrdinaryPairCheck {
        let (source, annotations) = self.current_pair_keys(document);
        OrdinaryPairCheck {
            owner: self.namespace.clone(),
            marker: Arc::new(()),
            document: document.clone(),
            source,
            annotations,
            proposed,
        }
    }
    fn validate_pair_basis(&self, check: &OrdinaryPairCheck) -> Result<(), ResourceError> {
        if !Arc::ptr_eq(&self.namespace, &check.owner) {
            return Err(ResourceError::Unavailable);
        }
        self.document(&check.document).map_err(owner_error)?;
        let (source, annotations) = self.current_pair_keys(&check.document);
        if source != check.source || annotations != check.annotations {
            return Err(ResourceError::Stale);
        }
        for key in source
            .iter()
            .chain(annotations.iter())
            .chain(check.proposed.iter())
        {
            self.provider_current(&key.registration)?;
        }
        Ok(())
    }
    fn validate_distinction(
        &self,
        check: &OrdinaryPairCheck,
        evidence: Option<&OrdinaryPairEvidence>,
    ) -> Result<(), ResourceError> {
        let annotation = check.proposed.as_ref().or(check.annotations.as_ref());
        let (Some(source), Some(annotation)) = (&check.source, annotation) else {
            return Ok(());
        };
        if source.identity == annotation.identity {
            return Err(ResourceError::Conflict);
        }
        let evidence = evidence.ok_or(ResourceError::MissingEvidence)?;
        if !Arc::ptr_eq(&check.marker, &evidence.check.marker) {
            return Err(ResourceError::Stale);
        }
        match evidence.relation {
            ResourceRelation::Distinct => Ok(()),
            ResourceRelation::Equivalent => Err(ResourceError::Conflict),
            ResourceRelation::Unknown => Err(ResourceError::MissingEvidence),
        }
    }
    pub(super) fn validate_ordinary_pair(
        &self,
        document: &DocumentId,
        evidence: Option<&OrdinaryPairEvidence>,
    ) -> Result<(), ResourceError> {
        let (source, annotations) = self.current_pair_keys(document);
        if source.is_none() || annotations.is_none() {
            return Ok(());
        }
        let evidence = evidence.ok_or(ResourceError::MissingEvidence)?;
        if evidence.check.document != *document || evidence.check.proposed.is_some() {
            return Err(ResourceError::Stale);
        }
        self.validate_pair_basis(&evidence.check)?;
        self.validate_distinction(&evidence.check, Some(evidence))
    }
    pub fn ordinary_pair_check(
        &self,
        document: &DocumentId,
        recorder: &mut Recorder,
    ) -> Result<Option<OrdinaryPairCheck>, ResourceError> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            self.document(document).map_err(owner_error)?;
            let check = self.pair_check(document, None);
            Ok((check.source.is_some() && check.annotations.is_some()).then_some(check))
        })();
        capture(recorder, Boundary::OrdinaryPairCheck, &result);
        result
    }
    pub fn prepare_annotation_binding(
        &self,
        document: &DocumentId,
        destination: ResourceRead,
        recorder: &mut Recorder,
    ) -> Result<AnnotationBindingPlan, ResourceError> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            self.document(document).map_err(owner_error)?;
            self.provider_current(&destination.registration)?;
            let key = BindingKey {
                identity: destination.identity.clone(),
                registration: destination.registration.clone(),
                generation: 1,
            };
            Ok(AnnotationBindingPlan {
                check: self.pair_check(document, Some(key)),
                destination,
            })
        })();
        capture(recorder, Boundary::AnnotationBindingPreparation, &result);
        result
    }
    /// Initial association only. No rebind/unbind, source modification, annotation
    /// restoration or baseline is inferred from a destination read or its JSON.
    pub fn activate_annotation_binding(
        &mut self,
        plan: AnnotationBindingPlan,
        evidence: Option<OrdinaryPairEvidence>,
        recorder: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            self.validate_pair_basis(&plan.check)?;
            self.provider_current(&plan.destination.registration)?;
            self.validate_distinction(&plan.check, evidence.as_ref())?;
            let proposed = plan
                .check
                .proposed
                .as_ref()
                .ok_or(ResourceError::Malformed)?;
            if let Some(existing) = &plan.check.annotations {
                return if existing == proposed {
                    Ok(false)
                } else {
                    Err(ResourceError::Unsupported)
                };
            }
            if self
                .resources
                .bindings
                .iter()
                .any(|b| b.identity == proposed.identity)
                || self
                    .resources
                    .annotation_bindings
                    .iter()
                    .any(|b| b.key.identity == proposed.identity)
            {
                return Err(ResourceError::Conflict);
            }
            if self.source_destination_reserved(&proposed.identity) {
                return Err(ResourceError::Blocked);
            }
            if self.resource_contested(&proposed.identity) {
                return Err(ResourceError::Contested);
            }
            if self.resources.tasks.iter().flatten().any(|t| {
                t.identity == proposed.identity
                    && (!t.safe()
                        || !t.quiescent
                        || t.nonce.as_ref().is_some_and(|n| Arc::strong_count(n) != 1))
            }) {
                return Err(ResourceError::Retained);
            }
            let growth = super::root_accounting::annotation_binding_bytes(proposed)
                .map_err(|_| ResourceError::Limit)?;
            self.check_root_growth(growth, recorder)
                .map_err(owner_error)?;
            let generation = self
                .next_publication_generation(None)
                .map_err(owner_error)?;
            self.resources
                .annotation_bindings
                .try_reserve(1)
                .map_err(|_| ResourceError::Capacity)?;
            self.resources.annotation_bindings.push(AnnotationBinding {
                document: plan.check.document,
                key: proposed.clone(),
                baseline: None,
            });
            self.generation = generation;
            Ok(true)
        })();
        if matches!(result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture(recorder, Boundary::AnnotationBindingPublication, &result);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn binding_counter_exhaustion_is_atomic_and_an_identical_binding_needs_no_counter() {
        let mut owner = SourceCoordinator::new(CoordinatorLimits {
            max_document_incarnations: 2,
            max_participants: 2,
            max_staged_bytes: 128,
            source: Limits {
                max_source_bytes: 128,
                max_edits: 2,
                max_edit_payload_bytes: 128,
            },
        });
        owner.resources = ResourceState::new(ResourceLimits {
            max_providers: 2,
            max_live_tasks: 2,
            max_issued_tasks: 4,
            max_queued_per_destination: 2,
            max_outstanding: 2,
            max_blob_bytes: 128,
            max_key_bytes: 64,
            max_validator_bytes: 64,
            max_encoders: 2,
            max_live_artifacts: 2,
            max_issued_artifacts: 4,
            max_held_artifact_bytes: 256,
            max_future_baseline_bytes: 65536,
            max_live_reads: 8,
            max_issued_reads: 32,
            max_read_chunks: 16,
            max_held_read_bytes: 1048576,
            max_restorers: 4,
            max_restoration_bytes: 8192,
        });
        let mut recorder = Recorder::disabled();
        let doc = owner.create(b"notes", &mut recorder).unwrap();
        let port = owner
            .register_provider(
                ProviderProfile {
                    synchronization_supported: false,
                },
                None,
                &mut recorder,
            )
            .unwrap();
        let plan = owner
            .prepare_annotation_binding(
                &doc.document,
                port.read_result(b"comments", b"v0", b"{}").unwrap(),
                &mut recorder,
            )
            .unwrap();
        owner.generation = u64::MAX;
        let usage = owner.resource_usage();
        assert_eq!(
            owner.activate_annotation_binding(plan, None, &mut recorder),
            Err(ResourceError::VersionExhausted)
        );
        assert_eq!(owner.resource_usage(), usage);
        owner.generation = 1;
        let plan = owner
            .prepare_annotation_binding(
                &doc.document,
                port.read_result(b"comments", b"v0", b"{}").unwrap(),
                &mut recorder,
            )
            .unwrap();
        assert!(
            owner
                .activate_annotation_binding(plan, None, &mut recorder)
                .unwrap()
        );
        owner.generation = u64::MAX;
        let plan = owner
            .prepare_annotation_binding(
                &doc.document,
                port.read_result(b"comments", b"v1", b"changed externally")
                    .unwrap(),
                &mut recorder,
            )
            .unwrap();
        assert!(
            !owner
                .activate_annotation_binding(plan, None, &mut recorder)
                .unwrap()
        );
        assert_eq!(owner.generation, u64::MAX);
    }
}
