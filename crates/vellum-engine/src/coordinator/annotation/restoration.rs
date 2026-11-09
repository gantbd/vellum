//! Complete initial hydration from a sealed actual read, not an authored edit.
use super::super::resource::owner_error;
use super::*;
use crate::coordinator::root_identity::NativeRootIdentity;
use crate::resource::{
    AnnotationRestoration, AnnotationRestorationOutcome, OrdinaryPairEvidence, ResourceError,
};
pub(in crate::coordinator) struct PreparedAnnotationRestoration {
    before: NativeRootIdentity,
    package: AnnotationRestoration,
    pair: Option<OrdinaryPairEvidence>,
    records: Vec<Record>,
    issued_documents: Vec<DocumentId>,
    outcome: AnnotationRestorationOutcome,
    retained: AnnotationRestorationOutcome,
    generation: u64,
    growth: usize,
}
impl SourceCoordinator {
    pub fn restore_annotations(
        &mut self,
        package: AnnotationRestoration,
        pair: Option<OrdinaryPairEvidence>,
        recorder: &mut Recorder,
    ) -> Result<AnnotationRestorationOutcome, ResourceError> {
        let timing = recorder.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let plan = self.prepare_annotation_restoration_native(package, pair, recorder)?;
            self.commit_annotation_restoration_native(plan, recorder)
        }));
        let result = match caught {
            Ok(v) => v,
            Err(p) => {
                self.faulted = true;
                recorder.record(Observation {
                    boundary: Boundary::AnnotationRestorationPublication,
                    disposition: Disposition::Unknown,
                    service_time: timing.finish(),
                });
                std::panic::resume_unwind(p)
            }
        };
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        recorder.record(Observation {
            boundary: Boundary::AnnotationRestorationPublication,
            disposition: match &result {
                Ok(_) => Disposition::Completed,
                Err(ResourceError::Faulted) => Disposition::Unknown,
                Err(_) => Disposition::Refused,
            },
            service_time: timing.finish(),
        });
        result
    }
    pub(in crate::coordinator) fn prepare_annotation_restoration_native(
        &mut self,
        package: AnnotationRestoration,
        pair: Option<OrdinaryPairEvidence>,
        recorder: &mut Recorder,
    ) -> Result<PreparedAnnotationRestoration, ResourceError> {
        self.ensure_ordinary_admission().map_err(owner_error)?;
        self.validate_restoration_read(&package, pair.as_ref(), recorder)?;
        let source = package.capture.source();
        let doc = self.validate_version(source).map_err(owner_error)?;
        let limits = self.annotations.limits.ok_or(ResourceError::Unsupported)?;
        if doc.annotation_read_only {
            return Err(ResourceError::Permission);
        }
        if self.annotations.issued_documents.len() != self.annotations.records.len()
            || self.annotations.retired.len() != self.annotations.records.len()
        {
            return Err(ResourceError::Faulted);
        }
        if self.annotations.issued_documents.contains(&source.document)
            || self.annotation_history_retains_document(&source.document)
        {
            return Err(ResourceError::Retained);
        }
        if package
            .charge
            .checked_add(
                package
                    .records
                    .len()
                    .checked_mul(
                        std::mem::size_of::<Record>()
                            + std::mem::size_of::<AnnotationFact>()
                            + std::mem::size_of::<DocumentId>(),
                    )
                    .ok_or(ResourceError::Limit)?,
            )
            .is_none_or(|n| n > self.limits.max_staged_bytes)
        {
            return Err(ResourceError::Limit);
        }
        let current = package
            .sources
            .iter()
            .find(|s| s.source() == source)
            .ok_or(ResourceError::MissingEvidence)?;
        if current.source_text() != doc.source.text() {
            // A managed witness naming this exact current source version
            // cannot legitimately have different canonical text.
            return Err(ResourceError::Faulted);
        }
        if current.set_version() != package.capture.set_version() {
            return Err(ResourceError::Stale);
        }
        for (i, witness) in package.sources.iter().enumerate() {
            if !self.knows_annotation_snapshot(witness)
                || witness.source().document != source.document
            {
                return Err(ResourceError::Unavailable);
            }
            if witness.source().revision > source.revision
                || package.sources[..i]
                    .iter()
                    .any(|s| s.source() == witness.source())
            {
                return Err(ResourceError::Malformed);
            }
        }
        let usage = self.annotation_usage();
        let count = package.records.len();
        if self
            .annotations
            .records
            .len()
            .checked_add(count)
            .is_none_or(|n| n > limits.max_issued)
            || usage
                .retained_records
                .checked_add(count)
                .is_none_or(|n| n > limits.max_records)
        {
            return Err(ResourceError::Limit);
        }
        let bodies = package
            .records
            .iter()
            .try_fold(usage.body_bytes, |n, r| n.checked_add(r.body.len()))
            .ok_or(ResourceError::Limit)?;
        if bodies > limits.max_retained_body_bytes {
            return Err(ResourceError::Limit);
        }
        let mut records = Vec::new();
        records
            .try_reserve_exact(count)
            .map_err(|_| ResourceError::Capacity)?;
        let mut facts = Vec::new();
        facts
            .try_reserve_exact(count)
            .map_err(|_| ResourceError::Capacity)?;
        let mut retained_facts = Vec::new();
        retained_facts
            .try_reserve_exact(count)
            .map_err(|_| ResourceError::Capacity)?;
        for (i, row) in package.records.iter().enumerate() {
            if row.content_version == 0
                || row.target_generation == 0
                || row.authored.basis != TargetBasis::Before
            {
                return Err(ResourceError::Malformed);
            }
            if row.body.len() > limits.max_body_bytes {
                return Err(ResourceError::Limit);
            }
            let authored = package
                .sources
                .iter()
                .find(|s| s.source() == &row.authored.source)
                .ok_or(ResourceError::MissingEvidence)?;
            crate::attachment::validate_target(authored.source_text(), row.authored.target)
                .map_err(|_| ResourceError::Malformed)?;
            let revision = match row.placement {
                Placement::Located { target, .. } => {
                    crate::attachment::validate_target(doc.source.text(), target)
                        .map_err(|_| ResourceError::Malformed)?;
                    Some(source.revision)
                }
                Placement::Detached => None,
                Placement::Unavailable => return Err(ResourceError::Unsupported),
            };
            let value = Value {
                id: AnnotationId {
                    namespace: self.namespace.clone(),
                    slot: self.annotations.records.len() + i,
                    document: source.document.clone(),
                },
                restored_identity: Some(row.identity.clone()),
                body: row.body.clone(),
                content: row.content_version,
                target_generation: row.target_generation,
                placement_generation: 1,
                target: TargetState {
                    authored: row.authored.clone(),
                    placement: row.placement,
                    revision,
                },
                present: true,
            };
            facts.push(value.fact());
            retained_facts.push(value.fact());
            records.push(Record {
                value,
                history: history::Frontiers::default(),
            });
        }
        let growth = super::root_accounting::restored_root_growth(&records)
            .map_err(|_| ResourceError::Limit)?;
        self.check_root_growth(growth, recorder)
            .map_err(owner_error)?;
        let changed = count != 0;
        let set = doc
            .annotation_generation
            .checked_add(u64::from(changed))
            .ok_or(ResourceError::VersionExhausted)?;
        let generation = if changed {
            self.next_publication_generation(None)
                .map_err(owner_error)?
        } else {
            self.generation
        };
        self.annotations
            .records
            .try_reserve(count)
            .map_err(|_| ResourceError::Capacity)?;
        self.annotations
            .issued_documents
            .try_reserve(count)
            .map_err(|_| ResourceError::Capacity)?;
        self.annotations
            .retired
            .try_reserve(count)
            .map_err(|_| ResourceError::Capacity)?;
        let outcome = AnnotationRestorationOutcome {
            source: source.clone(),
            set_version: set,
            changed,
            context: package.registration.context.clone(),
            annotations: facts,
            evidence_contested: false,
            loaded_baseline_adopted: false,
        };
        let retained = AnnotationRestorationOutcome {
            source: outcome.source.clone(),
            set_version: set,
            changed,
            context: outcome.context.clone(),
            annotations: retained_facts,
            evidence_contested: false,
            loaded_baseline_adopted: false,
        };

        let mut issued_documents = Vec::new();
        issued_documents
            .try_reserve_exact(count)
            .map_err(|_| ResourceError::Capacity)?;
        issued_documents.extend(std::iter::repeat_with(|| source.document.clone()).take(count));
        let before = self
            .native_root_identity()
            .map_err(|_| ResourceError::Faulted)?;
        Ok(PreparedAnnotationRestoration {
            before,
            package,
            pair,
            records,
            issued_documents,
            outcome,
            retained,
            generation,
            growth,
        })
    }
    pub(in crate::coordinator) fn commit_annotation_restoration_native(
        &mut self,
        plan: PreparedAnnotationRestoration,
        recorder: &mut Recorder,
    ) -> Result<AnnotationRestorationOutcome, ResourceError> {
        self.commit_annotation_restoration_for_invocation(plan, None, recorder)
    }
    pub(in crate::coordinator) fn commit_annotation_restoration_for_invocation(
        &mut self,
        plan: PreparedAnnotationRestoration,
        invocation: Option<&InvocationId>,
        recorder: &mut Recorder,
    ) -> Result<AnnotationRestorationOutcome, ResourceError> {
        self.ensure_ordinary_admission().map_err(owner_error)?;
        if self
            .native_root_identity()
            .map_err(|_| ResourceError::Faulted)?
            != plan.before
        {
            return Err(ResourceError::Stale);
        }
        self.validate_restoration_read(&plan.package, plan.pair.as_ref(), recorder)?;
        self.check_root_growth(plan.growth, recorder)
            .map_err(owner_error)?;
        let terminal = invocation
            .map(|id| {
                self.stage_annotation_load_candidate(
                    id,
                    plan.package.capture.origin(),
                    &AnnotationLoadOutcomeV1::Restoration {
                        document: plan.outcome.source.document.clone(),
                        changed: plan.outcome.changed,
                    },
                )
            })
            .transpose()?;
        // All result/provenance/document handles and vector capacities are prepaid.
        self.faulted = true;
        #[cfg(test)]
        assert_ne!(
            self.publication_fault_after,
            Some(0),
            "injected restoration interruption"
        );
        for (record, document) in plan.records.into_iter().zip(plan.issued_documents) {
            self.annotations.issued_documents.push(document);
            self.annotations.records.push(Some(record));
            self.annotations.retired.push(None);
        }
        #[cfg(test)]
        assert_ne!(
            self.publication_fault_after,
            Some(1),
            "injected restoration interruption"
        );
        self.documents[plan.outcome.source.document.slot]
            .as_mut()
            .expect("validated document")
            .annotation_generation = plan.outcome.set_version;
        #[cfg(test)]
        assert_ne!(
            self.publication_fault_after,
            Some(2),
            "injected restoration interruption"
        );
        self.generation = plan.generation;
        #[cfg(test)]
        assert_ne!(
            self.publication_fault_after,
            Some(3),
            "injected restoration interruption"
        );
        self.finish_annotation_restoration(plan.package, plan.retained);
        #[cfg(test)]
        assert_ne!(
            self.publication_fault_after,
            Some(4),
            "injected restoration interruption"
        );
        if let (Some(id), Some(terminal)) = (invocation, terminal) {
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(5),
                "injected restoration terminal interruption"
            );
            self.install_invocation_terminal(id, terminal);
        }
        self.faulted = false;
        Ok(plan.outcome)
    }
}
