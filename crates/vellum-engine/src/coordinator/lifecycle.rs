//! Retained lifecycle under the trusted owner; no UI, filesystem or erasure.
use super::*;
mod reclamation;
pub use reclamation::TrustedDocumentReclamationV1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LifecyclePhase {
    #[default]
    Live,
    DisposedRetained,
    ProvenanceReleased,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LifecycleState {
    pub phase: LifecyclePhase,
    pub generation: u64,
    pub pair_generation: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LifecycleUsage {
    pub issued_lifetime: usize,
    pub live: usize,
    pub disposed_retained: usize,
    pub provenance_released: usize,
}
/// A private exact-root preparation, not a replacement-root or retention grant.
#[derive(Clone, Debug)]
pub struct PreparedDisposal {
    owner: Arc<()>,
    publication: u64,
    source: DocumentVersion,
    set: u64,
    lifecycle: LifecycleState,
}
impl PreparedDisposal {
    pub(super) fn matches_explicit_core(
        &self,
        source: &DocumentVersion,
        set: u64,
        lifecycle: LifecycleState,
    ) -> bool {
        self.source == *source && self.set == set && self.lifecycle == lifecycle
    }
    pub(super) fn same_capture(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
            && self.publication == other.publication
            && self.source == other.source
            && self.set == other.set
            && self.lifecycle == other.lifecycle
    }
    pub(super) fn owned_by(&self, owner: &Arc<()>) -> bool {
        Arc::ptr_eq(&self.owner, owner)
    }
    pub(super) fn document(&self) -> &DocumentId {
        &self.source.document
    }
}
impl SourceCoordinator {
    pub fn lifecycle_usage(&self) -> LifecycleUsage {
        LifecycleUsage {
            issued_lifetime: self.documents.len(),
            live: self
                .documents
                .iter()
                .flatten()
                .filter(|d| d.lifecycle.phase == LifecyclePhase::Live)
                .count(),
            disposed_retained: self
                .documents
                .iter()
                .flatten()
                .filter(|d| d.lifecycle.phase == LifecyclePhase::DisposedRetained)
                .count(),
            provenance_released: self
                .documents
                .iter()
                .flatten()
                .filter(|d| d.lifecycle.phase == LifecyclePhase::ProvenanceReleased)
                .count(),
        }
    }
    /// Historical lifecycle metadata, not current content disclosure.
    pub fn lifecycle_state(
        &self,
        id: &DocumentId,
        recorder: &mut Recorder,
    ) -> Result<LifecycleState, CoordinatorError> {
        let timing = recorder.begin_service_timing();
        let result = self
            .retained_document(id)
            .map(|document| document.lifecycle);
        observe_service(recorder, Boundary::LifecycleQuery, &result, timing.finish());
        result
    }
    pub fn prepare_retained_disposal(
        &self,
        source: &DocumentVersion,
        set: u64,
        recorder: &mut Recorder,
    ) -> Result<PreparedDisposal, CoordinatorError> {
        let timing = recorder.begin_service_timing();
        let result = (|| {
            self.ensure_operational()?;
            let document = self.validate_version(source)?;
            if document.annotation_generation != set {
                return Err(CoordinatorError::Stale);
            }
            self.validate_disposal_obligations(&source.document)
                .map_err(CoordinatorError::Resource)?;
            Ok(PreparedDisposal {
                owner: self.namespace.clone(),
                publication: self.generation,
                source: source.clone(),
                set,
                lifecycle: document.lifecycle,
            })
        })();
        observe_service(
            recorder,
            Boundary::DisposalPreparation,
            &result,
            timing.finish(),
        );
        result
    }
    /// Explicit trusted-owner discard of live editing authority, not payload
    /// purge or effect cancellation. Preserves records, history and bindings.
    pub fn commit_retained_disposal(
        &mut self,
        plan: PreparedDisposal,
        recorder: &mut Recorder,
    ) -> Result<LifecycleState, CoordinatorError> {
        self.commit_retained_disposal_for_invocation(plan, None, recorder)
    }
    pub(super) fn commit_retained_disposal_for_invocation(
        &mut self,
        plan: PreparedDisposal,
        invocation: Option<&InvocationId>,
        recorder: &mut Recorder,
    ) -> Result<LifecycleState, CoordinatorError> {
        self.commit_retained_disposal_impl(plan, invocation, None, recorder)
    }
    pub(super) fn commit_shutdown_retained_disposal(
        &mut self,
        plan: PreparedDisposal,
        fence: &ShutdownFenceIdV1,
        recorder: &mut Recorder,
    ) -> Result<LifecycleState, CoordinatorError> {
        self.commit_retained_disposal_impl(plan, None, Some(fence), recorder)
    }
    fn commit_retained_disposal_impl(
        &mut self,
        plan: PreparedDisposal,
        invocation: Option<&InvocationId>,
        shutdown: Option<&ShutdownFenceIdV1>,
        recorder: &mut Recorder,
    ) -> Result<LifecycleState, CoordinatorError> {
        let timing = recorder.begin_service_timing();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.ensure_operational()?;
            if !Arc::ptr_eq(&plan.owner, &self.namespace) {
                return Err(CoordinatorError::Unavailable);
            }
            if plan.publication != self.generation {
                return Err(CoordinatorError::Stale);
            }
            let document = self.validate_version(&plan.source)?;
            if document.annotation_generation != plan.set || document.lifecycle != plan.lifecycle {
                return Err(CoordinatorError::Stale);
            }
            self.validate_disposal_obligations(&plan.source.document)
                .map_err(CoordinatorError::Resource)?;
            let next = LifecycleState {
                phase: LifecyclePhase::DisposedRetained,
                generation: document
                    .lifecycle
                    .generation
                    .checked_add(1)
                    .ok_or(CoordinatorError::VersionExhausted)?,
                pair_generation: document.lifecycle.pair_generation,
            };
            let publication = if let Some(id) = shutdown {
                self.validate_shutdown_disposition_fence(id)?;
                self.next_retention_publication_generation(None)?
            } else {
                self.next_publication_generation(None)?
            };
            let views = self.stage_view_disposal(&plan.source.document)?;
            let workspaces = self.stage_workspace_disposal(&plan.source.document, recorder)?;
            if let Some(id) = invocation {
                self.stage_retained_disposal_candidate(id, &plan, next)
                    .map_err(|_| CoordinatorError::Faulted)?;
            }
            // All expected refusal precedes installation. Leave the fence set
            // if unwinding interrupts any root or the publication acknowledgment.
            self.faulted = true;
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(0),
                "injected lifecycle publication interruption"
            );
            self.documents[plan.source.document.slot]
                .as_mut()
                .expect("validated live owner")
                .lifecycle = next;
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(1),
                "injected lifecycle publication interruption"
            );
            views.install(self);
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(2),
                "injected lifecycle publication interruption"
            );
            workspaces.install(self);
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(3),
                "injected lifecycle publication interruption"
            );
            self.generation = publication;
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(4),
                "injected lifecycle publication interruption"
            );
            self.faulted = false;
            Ok(next)
        }));
        let result = match result {
            Ok(result) => result,
            Err(panic) => {
                self.faulted = true;
                observe_service(
                    recorder,
                    Boundary::RetainedDisposal,
                    &Err::<LifecycleState, _>(CoordinatorError::Faulted),
                    timing.finish(),
                );
                // Preserve the original panic; diagnostics are not rollback or
                // a known refusal, and no panic payload enters the recorder.
                std::panic::resume_unwind(panic)
            }
        };
        if matches!(
            result,
            Err(CoordinatorError::Faulted
                | CoordinatorError::Resource(crate::resource::ResourceError::Faulted))
        ) {
            self.faulted = true;
        }
        observe_service(
            recorder,
            Boundary::RetainedDisposal,
            &result,
            timing.finish(),
        );
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn owner() -> (SourceCoordinator, DocumentVersion, Recorder) {
        let mut owner = SourceCoordinator::new(CoordinatorLimits {
            max_document_incarnations: 4,
            max_participants: 4,
            max_staged_bytes: 256,
            source: Limits {
                max_source_bytes: 128,
                max_edits: 4,
                max_edit_payload_bytes: 128,
            },
        });
        let mut recorder = Recorder::disabled();
        let source = owner.create(b"private", &mut recorder).unwrap();
        (owner, source, recorder)
    }
    fn linked_owner() -> (SourceCoordinator, DocumentVersion, Recorder) {
        use crate::selection::{Selection, SelectionBudget};
        let (mut o, d, mut r) = owner();
        o.configure_logical_views(
            LogicalViewLimits {
                max_live_views: 2,
                max_issued_views: 2,
                max_total_selections: 8,
                normalization: SelectionBudget {
                    max_input_selections: 4,
                    max_output_selections: 4,
                    max_work: 256,
                },
                max_mapping_steps: 256,
            },
            &mut r,
        )
        .unwrap();
        o.configure_workspaces(
            WorkspaceLimits {
                max_issued: 2,
                max_live: 2,
                max_memberships: 8,
                max_work: 256,
            },
            &mut r,
        )
        .unwrap();
        let view = o.create_logical_view(&mut r).unwrap();
        let stamp = o.logical_view(&view, &mut r).unwrap().stamp;
        let point = Selection {
            anchor: crate::coordinates::BytePosition(0),
            head: crate::coordinates::BytePosition(0),
            affinity: crate::attachment::Affinity::After,
        };
        o.attach_logical_view(&view, stamp, &d, &[point], 0, &mut r)
            .unwrap();
        let workspace = o.create_workspace(&mut r).unwrap();
        o.set_workspace_membership(&workspace, &d.document, true, &mut r)
            .unwrap();
        (o, d, r)
    }
    #[test]
    fn interrupted_retained_disposal_fences_other_mutations_and_reports_unknown_not_completion() {
        let (mut o, d, mut setup) = owner();
        let plan = o.prepare_retained_disposal(&d, 0, &mut setup).unwrap();
        let generation = o.generation();
        o.publication_fault_after = Some(1);
        let mut r = Recorder::try_with_capacity(32)
            .unwrap()
            .with_native_timing(crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || o.commit_retained_disposal(plan, &mut r)
            ))
            .is_err()
        );
        assert_eq!(
            o.retained_document(&d.document).unwrap().lifecycle.phase,
            LifecyclePhase::DisposedRetained
        );
        assert_eq!(o.generation(), generation);
        assert!(o.is_faulted());
        assert_eq!(
            o.create(b"other", &mut setup),
            Err(CoordinatorError::Faulted)
        );
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front())
            .filter(|e| e.boundary == Boundary::RetainedDisposal)
            .collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].disposition, Disposition::Unknown);
        assert!(events[0].service_time.is_some());
    }
    #[test]
    fn each_disposal_installation_cut_fences_partial_roots_with_optional_unknown_instrumentation() {
        use crate::instrumentation::NativeTimingProfileV1;
        for point in 0..=4 {
            for mode in 0..4 {
                let (mut o, d, mut setup) = linked_owner();
                let before = o.generation();
                let plan = o.prepare_retained_disposal(&d, 0, &mut setup).unwrap();
                o.publication_fault_after = Some(point);
                let mut r = if mode == 0 {
                    Recorder::disabled()
                } else {
                    Recorder::try_with_capacity(if mode == 3 { 1 } else { 32 }).unwrap()
                };
                if mode >= 2 {
                    r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
                }
                if mode == 3 {
                    r.record(Observation {
                        boundary: Boundary::Control,
                        disposition: Disposition::Completed,
                        service_time: None,
                    });
                }
                assert!(
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                        || o.commit_retained_disposal(plan, &mut r)
                    ))
                    .is_err()
                );
                assert!(o.is_faulted());
                assert_eq!(o.generation(), before + u64::from(point == 4));
                assert_eq!(o.logical_view_usage().attached, usize::from(point < 2));
                assert_eq!(o.workspace_usage().memberships, usize::from(point < 3));
                assert_eq!(
                    o.retained_document(&d.document).unwrap().lifecycle.phase,
                    if point == 0 {
                        LifecyclePhase::Live
                    } else {
                        LifecyclePhase::DisposedRetained
                    }
                );
                assert_eq!(
                    o.read(&d.document, &mut setup).err(),
                    Some(CoordinatorError::Faulted)
                );
                assert_eq!(
                    o.create(b"other", &mut setup),
                    Err(CoordinatorError::Faulted)
                );
                let mut unknown = 0;
                while let Some(e) = r.pop_front() {
                    if e.boundary == Boundary::RetainedDisposal {
                        unknown += 1;
                        assert_eq!(e.disposition, Disposition::Unknown);
                        assert_eq!(e.service_time.is_some(), mode == 2);
                    }
                    assert!(!format!("{e:?}").contains("private"));
                }
                assert_eq!(unknown, usize::from(mode == 1 || mode == 2));
                if mode == 3 {
                    assert!(r.dropped() > 0);
                }
            }
        }
    }
    #[test]
    fn normal_disposal_clears_its_installation_fence_and_known_stale_refusal_preserves_roots() {
        for stale in [false, true] {
            let (mut o, d, mut r) = linked_owner();
            let plan = o.prepare_retained_disposal(&d, 0, &mut r).unwrap();
            if stale {
                o.create(b"other", &mut r).unwrap();
            }
            let before = o.generation();
            let result = o.commit_retained_disposal(plan, &mut r);
            assert!(!o.is_faulted());
            if stale {
                assert_eq!(result, Err(CoordinatorError::Stale));
                assert_eq!(o.logical_view_usage().attached, 1);
                assert_eq!(o.workspace_usage().memberships, 1);
                assert_eq!(o.generation(), before);
            } else {
                assert_eq!(result.unwrap().phase, LifecyclePhase::DisposedRetained);
                assert_eq!(o.logical_view_usage().attached, 0);
                assert_eq!(o.workspace_usage().memberships, 0);
                assert_eq!(o.generation(), before + 1);
                o.create(b"other", &mut r).unwrap();
            }
        }
    }
    #[test]
    fn disposal_counter_exhaustion_preserves_liveness_content_and_lifetime_inventory() {
        for lifecycle_counter in [true, false] {
            let (mut owner, source, mut recorder) = owner();
            if lifecycle_counter {
                owner.documents[source.document.slot]
                    .as_mut()
                    .unwrap()
                    .lifecycle
                    .generation = u64::MAX;
            } else {
                owner.generation = u64::MAX;
            }
            let plan = owner
                .prepare_retained_disposal(&source, 0, &mut recorder)
                .unwrap();
            let before = owner.lifecycle_usage();
            let state = owner
                .lifecycle_state(&source.document, &mut recorder)
                .unwrap();
            assert_eq!(
                owner.commit_retained_disposal(plan, &mut recorder),
                Err(CoordinatorError::VersionExhausted)
            );
            assert_eq!(owner.lifecycle_usage(), before);
            assert_eq!(
                owner
                    .lifecycle_state(&source.document, &mut recorder)
                    .unwrap(),
                state
            );
            assert_eq!(
                owner
                    .read(&source.document, &mut recorder)
                    .unwrap()
                    .source
                    .text(),
                "private"
            );
        }
    }
    #[test]
    fn a_native_fault_refuses_even_previously_prepared_disposal() {
        let (mut owner, source, mut recorder) = owner();
        let plan = owner
            .prepare_retained_disposal(&source, 0, &mut recorder)
            .unwrap();
        let generation = owner.generation;
        owner.faulted = true;
        assert_eq!(
            owner.commit_retained_disposal(plan, &mut recorder),
            Err(CoordinatorError::Faulted)
        );
        assert_eq!(owner.generation, generation);
        assert_eq!(owner.lifecycle_usage().live, 1);
    }
}
