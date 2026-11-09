//! Loaded evidence is an actual consumed read, never a fabricated write receipt.
use super::*;
use crate::coordinator::root_identity::NativeRootIdentity;
#[cfg(test)]
pub(super) mod commit_tests;
pub(in crate::coordinator) struct PreparedLoadedBaseline {
    before: NativeRootIdentity,
    id: AnnotationReadId,
    pair: Option<OrdinaryPairEvidence>,
    maximum: usize,
    index: usize,
    slot: AnnotationBaselineSlot,
    generation: u64,
    changed: bool,
}
impl AnnotationBaselineSlot {
    pub(super) fn written(&self) -> Option<&AnnotationBaseline> {
        match self {
            Self::Written(b) => Some(b),
            Self::Loaded(_) => None,
        }
    }
    #[cfg(test)]
    pub(super) fn written_mut(&mut self) -> Option<&mut AnnotationBaseline> {
        match self {
            Self::Written(b) => Some(b),
            Self::Loaded(_) => None,
        }
    }
    pub(super) fn bytes(&self) -> &Blob {
        match self {
            Self::Written(b) => &b.bytes,
            Self::Loaded(b) => &b.bytes,
        }
    }
    pub(super) fn encoding_context(&self) -> &AnnotationEncodingContext {
        match self {
            Self::Written(b) => &b.fact.encoding_context,
            Self::Loaded(b) => &b.fact.encoding_context,
        }
    }
    pub(super) fn retained_size(&self) -> Option<usize> {
        match self {
            Self::Written(b) => b.retained_size(),
            Self::Loaded(b) => b.retained_size(),
        }
    }
}
impl LoadedAnnotationBaseline {
    pub(super) fn retained_size(&self) -> Option<usize> {
        std::mem::size_of::<AnnotationBaselineSlot>()
            .checked_add(self.bytes.0.len())?
            .checked_add(self.version.0.len())?
            .checked_add(self.association.binding.identity.key.0.len())?
            .checked_add(
                self.association
                    .source_binding
                    .as_ref()
                    .map_or(0, |b| b.identity.key.0.len()),
            )
    }
    fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.attachment, &other.attachment)
            && self.fact == other.fact
            && self.association == other.association
            && Arc::ptr_eq(&self.encoding, &other.encoding)
            && Arc::ptr_eq(&self.registration, &other.registration)
            && self.bytes == other.bytes
            && self.version == other.version
    }
}
impl SourceCoordinator {
    /// One actual read registry and writer share this native owner. Explicit
    /// attachment enables loaded evidence and charges its singleton relation;
    /// another root's raw reader/handle cannot be supplied or attached here.
    pub fn attach_annotation_read_baselines(
        &mut self,
        recorder: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            self.ensure_annotation_reader_open()?;
            let limits = self.resources.limits.ok_or(ResourceError::Unsupported)?;
            if let Some(a) = &self.resources.read_baseline_attachment {
                a.current(&self.namespace)?;
                if self.resources.read_baseline_generation == u64::MAX {
                    return Err(ResourceError::Faulted);
                }
                return Ok(false);
            }
            self.ensure_admission_unfenced().map_err(owner_error)?;
            self.resources
                .read_baseline_generation
                .checked_add(2)
                .ok_or(ResourceError::Limit)?;
            let generation = self.resources.read_baseline_generation + 1;
            let charge = std::mem::size_of::<ReadBaselineEndpoint>();
            let held = self
                .resources
                .held_read_bytes
                .checked_add(charge)
                .ok_or(ResourceError::Limit)?;
            if held > limits.max_held_read_bytes {
                return Err(ResourceError::Limit);
            }
            let writer_held = self
                .resources
                .held_artifact_bytes
                .checked_add(charge)
                .ok_or(ResourceError::Limit)?;
            if writer_held > limits.max_held_artifact_bytes {
                return Err(ResourceError::Limit);
            }
            let incarnation = Arc::new(());
            self.resources.read_baseline_attachment = Some(ReadBaselineAttachment {
                reader: ReadBaselineEndpoint {
                    owner: self.namespace.clone(),
                    incarnation: incarnation.clone(),
                },
                writer: ReadBaselineEndpoint {
                    owner: self.namespace.clone(),
                    incarnation,
                },
            });
            self.resources.held_read_bytes = held;
            self.resources.held_artifact_bytes = writer_held;
            self.resources.read_baseline_generation = generation;
            Ok(true)
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture(recorder, Boundary::LoadedBaselineAttachment, &result);
        result
    }
    pub(super) fn validate_loaded_baseline(
        &self,
        binding: &AnnotationBinding,
        baseline: &LoadedAnnotationBaseline,
    ) -> Result<(), ResourceError> {
        if binding.document != baseline.fact.source.document
            || binding.key != baseline.association.binding
        {
            return Err(ResourceError::Stale);
        }
        if self.resource_contested(&binding.key.identity) {
            return Err(ResourceError::Contested);
        }
        let actual = self.original_loaded_baseline(&baseline.fact.origin)?;
        if !baseline.same(&actual) {
            return Err(ResourceError::Faulted);
        }
        Ok(())
    }
    pub(super) fn validate_annotation_baseline_slot(
        &self,
        binding: &AnnotationBinding,
        baseline: &AnnotationBaselineSlot,
    ) -> Result<(), ResourceError> {
        match baseline {
            AnnotationBaselineSlot::Written(b) => self.validate_annotation_baseline(binding, b),
            AnnotationBaselineSlot::Loaded(b) => self.validate_loaded_baseline(binding, b),
        }
    }
    fn loaded_writer_guard(&self, identity: &Identity) -> Result<(), ResourceError> {
        if self.resource_contested(identity) {
            return Err(ResourceError::Contested);
        }
        if self.resources.tasks.iter().flatten().any(|t| {
            t.identity == *identity
                && (t.phase == SavePhase::Queued
                    || !t.safe()
                    || !t.quiescent
                    || t.knowledge.data.application == Application::Written)
        }) {
            return Err(ResourceError::Blocked);
        }
        Ok(())
    }
    /// Initial-only historical adoption of the actual consumed read. Does not
    /// import write acknowledgments, refresh input or mark later edits saved.
    pub fn adopt_loaded_annotation_baseline(
        &mut self,
        id: &AnnotationReadId,
        pair: Option<OrdinaryPairEvidence>,
        max_baseline_bytes: usize,
        recorder: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        let timing = recorder.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let plan =
                self.prepare_loaded_baseline_native(id, pair, max_baseline_bytes, recorder)?;
            self.commit_loaded_baseline_native(plan, recorder)
        }));
        let result = match caught {
            Ok(v) => v,
            Err(p) => {
                self.faulted = true;
                capture_service(
                    recorder,
                    Boundary::LoadedBaselineAdoption,
                    &Err::<bool, _>(ResourceError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(p)
            }
        };
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(
            recorder,
            Boundary::LoadedBaselineAdoption,
            &result,
            timing.finish(),
        );
        result
    }
    pub(in crate::coordinator) fn prepare_loaded_baseline_native(
        &mut self,
        id: &AnnotationReadId,
        pair: Option<OrdinaryPairEvidence>,
        max_baseline_bytes: usize,
        recorder: &mut Recorder,
    ) -> Result<PreparedLoadedBaseline, ResourceError> {
        let (index, slot, generation, changed) = self.validate_loaded_baseline_adoption_native(
            id,
            pair.as_ref(),
            max_baseline_bytes,
            recorder,
        )?;
        let before = self
            .native_root_identity()
            .map_err(|_| ResourceError::Faulted)?;
        Ok(PreparedLoadedBaseline {
            before,
            id: id.clone(),
            pair,
            maximum: max_baseline_bytes,
            index,
            slot,
            generation,
            changed,
        })
    }
    fn validate_loaded_baseline_adoption_native(
        &self,
        id: &AnnotationReadId,
        pair: Option<&OrdinaryPairEvidence>,
        max_baseline_bytes: usize,
        recorder: &mut Recorder,
    ) -> Result<(usize, AnnotationBaselineSlot, u64, bool), ResourceError> {
        self.ensure_operational().map_err(owner_error)?;
        let baseline = self.original_loaded_baseline(id)?;
        self.validate_ordinary_pair(&baseline.fact.source.document, pair)?;
        self.loaded_writer_guard(&baseline.association.binding.identity)?;
        let index = self
            .resources
            .annotation_bindings
            .iter()
            .position(|b| b.document == baseline.fact.source.document)
            .ok_or(ResourceError::Unbound)?;
        let size = baseline.retained_size().ok_or(ResourceError::Limit)?;
        let limit = self.loaded_companion_limit(id)?;
        if size > max_baseline_bytes
            || self
                .loaded_companion_size(id)?
                .checked_add(size)
                .is_none_or(|n| n > limit)
        {
            return Err(ResourceError::Limit);
        }
        if let Some(old) = &self.resources.annotation_bindings[index].baseline {
            self.validate_annotation_baseline_slot(
                &self.resources.annotation_bindings[index],
                old,
            )?;
            return match old {
                AnnotationBaselineSlot::Loaded(old) if old.same(&baseline) => Ok((
                    index,
                    AnnotationBaselineSlot::Loaded(baseline),
                    self.generation,
                    false,
                )),
                _ => Err(ResourceError::Conflict),
            };
        }
        // This is qualified bookkeeping for the original already-restored read,
        // not fresh hydration or write admission. Preserve every other pending
        // baseline's publication credit while allowing settlement under a fence.
        let generation = self
            .next_retention_publication_generation(None)
            .map_err(owner_error)?;
        let slot = AnnotationBaselineSlot::Loaded(baseline);
        let added = slot
            .root_payload_bytes()
            .map_err(|_| ResourceError::Limit)?;
        self.check_root_replacement_projected(
            0,
            added,
            BaselineRootProjection {
                changed: Some((&self.resources.annotation_bindings[index].document, added)),
                ..BaselineRootProjection::default()
            },
            recorder,
        )
        .map_err(owner_error)?;
        Ok((index, slot, generation, true))
    }
    pub(in crate::coordinator) fn commit_loaded_baseline_native(
        &mut self,
        plan: PreparedLoadedBaseline,
        recorder: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        self.commit_loaded_baseline_for_invocation(plan, None, recorder)
    }
    pub(in crate::coordinator) fn commit_loaded_baseline_for_invocation(
        &mut self,
        plan: PreparedLoadedBaseline,
        invocation: Option<&InvocationId>,
        recorder: &mut Recorder,
    ) -> Result<bool, ResourceError> {
        self.ensure_operational().map_err(owner_error)?;
        if self
            .native_root_identity()
            .map_err(|_| ResourceError::Faulted)?
            != plan.before
        {
            return Err(ResourceError::Stale);
        }
        let (index, current, generation, changed) = self.validate_loaded_baseline_adoption_native(
            &plan.id,
            plan.pair.as_ref(),
            plan.maximum,
            recorder,
        )?;
        let (AnnotationBaselineSlot::Loaded(expected), AnnotationBaselineSlot::Loaded(actual)) =
            (&plan.slot, &current)
        else {
            return Err(ResourceError::Faulted);
        };
        if index != plan.index
            || generation != plan.generation
            || changed != plan.changed
            || !expected.same(actual)
        {
            return Err(ResourceError::Stale);
        }
        let terminal = invocation
            .map(|id| {
                self.stage_annotation_load_candidate(
                    id,
                    &plan.id,
                    &AnnotationLoadOutcomeV1::Baseline {
                        document: expected.fact.source.document.clone(),
                        changed,
                    },
                )
            })
            .transpose()?;
        if !changed && invocation.is_none() {
            return Ok(false);
        }
        self.faulted = true;
        #[cfg(test)]
        assert_ne!(
            self.publication_fault_after,
            Some(0),
            "injected loaded baseline interruption"
        );
        if changed {
            self.resources.annotation_bindings[index].baseline = Some(plan.slot);
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(1),
                "injected loaded baseline interruption"
            );
            self.mark_loaded_baseline_adopted(&plan.id);
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(2),
                "injected loaded baseline interruption"
            );
            self.generation = generation;
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(3),
                "injected loaded baseline interruption"
            );
        }
        if let (Some(id), Some(terminal)) = (invocation, terminal) {
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(4),
                "injected baseline terminal interruption"
            );
            self.install_invocation_terminal(id, terminal);
        }
        self.faulted = false;
        Ok(changed)
    }
    /// Typed historical metadata after current binding/evidence validation.
    /// Neither variant establishes current external freshness.
    pub fn annotation_baseline_evidence(
        &mut self,
        document: &DocumentId,
        pair: Option<OrdinaryPairEvidence>,
        recorder: &mut Recorder,
    ) -> Result<Option<AnnotationBaselineEvidence>, ResourceError> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            self.document(document).map_err(owner_error)?;
            self.validate_ordinary_pair(document, pair.as_ref())?;
            let Some(binding) = self
                .resources
                .annotation_bindings
                .iter()
                .find(|b| b.document == *document)
            else {
                return Ok(None);
            };
            let Some(baseline) = &binding.baseline else {
                return Ok(None);
            };
            self.validate_annotation_baseline_slot(binding, baseline)?;
            Ok(Some(match baseline {
                AnnotationBaselineSlot::Written(b) => {
                    AnnotationBaselineEvidence::Written(b.fact.clone())
                }
                AnnotationBaselineSlot::Loaded(b) => {
                    AnnotationBaselineEvidence::Loaded(b.fact.clone())
                }
            }))
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture(recorder, Boundary::AnnotationBaselineQuery, &result);
        result
    }
}
#[cfg(test)]
mod identity_tests {
    use super::*;
    #[test]
    fn attachment_generation_reserves_detach_before_admission_and_cleanup_ignores_global_exhaustion()
     {
        for remaining in 0..4 {
            let (mut o, mut r, _, _) = super::super::serialization_tests::fixture();
            let start = u64::MAX - remaining;
            o.resources.read_baseline_generation = start;
            let held = (o.resources.held_read_bytes, o.resources.held_artifact_bytes);
            let identity = o.native_root_identity().unwrap();
            let result = o.attach_annotation_read_baselines(&mut r);
            if remaining < 2 {
                assert_eq!(result, Err(ResourceError::Limit));
                assert_eq!(o.native_root_identity().unwrap(), identity);
                assert_eq!(
                    (o.resources.held_read_bytes, o.resources.held_artifact_bytes),
                    held
                );
                assert!(o.resources.read_baseline_attachment.is_none() && !o.is_faulted());
                continue;
            }
            assert_eq!(result, Ok(true));
            assert_eq!(o.resources.read_baseline_generation, start + 1);
            assert_eq!(o.attach_annotation_read_baselines(&mut r), Ok(false));
            // Original cleanup must not depend on another publication unit or
            // disclosure health. Its own last generation step was prepaid.
            o.generation = u64::MAX;
            o.faulted = true;
            assert_eq!(o.detach_annotation_read_baselines(&mut r), Ok(true));
            assert_eq!(o.resources.read_baseline_generation, start + 2);
            assert_eq!(
                (o.resources.held_read_bytes, o.resources.held_artifact_bytes),
                held
            );
            assert_eq!(o.detach_annotation_read_baselines(&mut r), Ok(false));
            assert!(o.is_faulted());
            o.faulted = false; // Test-only removal of the unrelated injected fault.
            assert_eq!(
                o.attach_annotation_read_baselines(&mut r),
                Err(ResourceError::Limit)
            );
        }
    }
    #[test]
    fn corrupt_active_attachment_generation_cannot_erase_endpoint_debt_or_fabricate_detach() {
        let (mut o, mut r, _, _) = super::super::serialization_tests::fixture();
        o.attach_annotation_read_baselines(&mut r).unwrap();
        let marker = o
            .resources
            .read_baseline_attachment
            .as_ref()
            .unwrap()
            .current(&o.namespace)
            .unwrap()
            .clone();
        let held = (o.resources.held_read_bytes, o.resources.held_artifact_bytes);
        o.resources.read_baseline_generation = u64::MAX;
        assert_eq!(
            o.native_root_identity(),
            Err(RootAccountingErrorV1::Faulted)
        );
        assert_eq!(
            o.detach_annotation_read_baselines(&mut r),
            Err(ResourceError::Faulted)
        );
        assert!(o.is_faulted());
        assert_eq!(
            (o.resources.held_read_bytes, o.resources.held_artifact_bytes),
            held
        );
        assert!(Arc::ptr_eq(
            &marker,
            o.resources
                .read_baseline_attachment
                .as_ref()
                .unwrap()
                .current(&o.namespace)
                .unwrap()
        ));
    }
}
