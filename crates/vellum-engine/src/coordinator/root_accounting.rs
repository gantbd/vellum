//! Native logical units of the CURRENT semantic root, not allocated heap bytes.
//! See docs/native-root-accounting.md for projection, exclusions and ABI scope.
use super::*;
use std::mem::size_of;
#[cfg(test)]
mod admission_tests;

pub const NATIVE_SEMANTIC_ROOT_PROFILE_V1: &str = "native-semantic-root/1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootAccountingErrorV1 {
    Overflow,
    Limit,
    Unavailable,
    Faulted,
}

/// Privileged owner gauges, not a recipient-safe release or an admission receipt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticRootUsageV1 {
    pub profile: &'static str,
    pub header_bytes: usize,
    pub documents_bytes: usize,
    pub history_bytes: usize,
    pub annotations_bytes: usize,
    pub views_bytes: usize,
    pub workspaces_bytes: usize,
    pub persistence_bytes: usize,
    pub settings_bytes: usize,
    pub total_bytes: usize,
}

/// Trusted-owner demand gauges. Pending obligations are not current root payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RootCapacityUsageV1 {
    pub profile: &'static str,
    pub current_root_bytes: usize,
    pub pending_baseline_growth_bytes: usize,
    pub admission_bytes: usize,
    pub ceiling: Option<usize>,
}

pub(super) type Result<T> = std::result::Result<T, RootAccountingErrorV1>;
#[derive(Default)]
pub(super) struct Bytes(usize);
impl Bytes {
    pub(super) fn add(&mut self, bytes: usize) -> Result<()> {
        self.0 = self
            .0
            .checked_add(bytes)
            .ok_or(RootAccountingErrorV1::Overflow)?;
        Ok(())
    }
    pub(super) fn rows<T>(&mut self, count: usize) -> Result<()> {
        self.add(
            size_of::<T>()
                .checked_mul(count)
                .ok_or(RootAccountingErrorV1::Overflow)?,
        )
    }
    pub(super) fn finish(self) -> usize {
        self.0
    }
}

impl SourceCoordinator {
    pub fn semantic_root_capacity_usage(
        &self,
        recorder: &mut Recorder,
    ) -> Result<RootCapacityUsageV1> {
        let result = (|| {
            let root = self.measure_semantic_root()?.total_bytes;
            let admission =
                self.root_capacity_bytes(root, resource::BaselineRootProjection::default())?;
            Ok(RootCapacityUsageV1 {
                profile: NATIVE_SEMANTIC_ROOT_PROFILE_V1,
                current_root_bytes: root,
                pending_baseline_growth_bytes: admission
                    .checked_sub(root)
                    .ok_or(RootAccountingErrorV1::Faulted)?,
                admission_bytes: admission,
                ceiling: self.root_byte_ceiling,
            })
        })();
        recorder.record(Observation {
            boundary: Boundary::SemanticRootCapacityInspection,
            disposition: match result {
                Ok(_) => Disposition::Completed,
                Err(RootAccountingErrorV1::Faulted) => Disposition::Unknown,
                Err(_) => Disposition::Refused,
            },
            service_time: None,
        });
        result
    }
    pub(super) fn check_root_growth(
        &self,
        additional: usize,
        recorder: &mut Recorder,
    ) -> std::result::Result<(), CoordinatorError> {
        self.check_root_replacement(0, additional, recorder)
    }
    pub(super) fn check_root_replacement(
        &self,
        removed: usize,
        added: usize,
        recorder: &mut Recorder,
    ) -> std::result::Result<(), CoordinatorError> {
        self.check_root_replacement_projected(
            removed,
            added,
            resource::BaselineRootProjection::default(),
            recorder,
        )
    }
    pub(super) fn check_root_replacement_projected(
        &self,
        removed: usize,
        added: usize,
        projection: resource::BaselineRootProjection<'_>,
        recorder: &mut Recorder,
    ) -> std::result::Result<(), CoordinatorError> {
        if self.root_byte_ceiling.is_none() {
            return Ok(());
        }
        let candidate = self.measure_semantic_root().and_then(|root| {
            root.total_bytes
                .checked_sub(removed)
                .ok_or(RootAccountingErrorV1::Faulted)?
                .checked_add(added)
                .ok_or(RootAccountingErrorV1::Overflow)
        });
        self.check_root_candidate_projected(candidate, projection, recorder)
    }

    fn check_root_candidate(
        &self,
        candidate: Result<usize>,
        recorder: &mut Recorder,
    ) -> std::result::Result<(), CoordinatorError> {
        self.check_root_candidate_projected(
            candidate,
            resource::BaselineRootProjection::default(),
            recorder,
        )
    }
    fn root_capacity_bytes(
        &self,
        candidate: usize,
        projection: resource::BaselineRootProjection<'_>,
    ) -> Result<usize> {
        candidate
            .checked_add(self.pending_baseline_root_growth(projection)?)
            .ok_or(RootAccountingErrorV1::Overflow)
    }
    fn check_root_candidate_projected(
        &self,
        candidate: Result<usize>,
        projection: resource::BaselineRootProjection<'_>,
        recorder: &mut Recorder,
    ) -> std::result::Result<(), CoordinatorError> {
        let result = candidate
            .and_then(|bytes| self.root_capacity_bytes(bytes, projection))
            .map_err(|e| match e {
                RootAccountingErrorV1::Faulted => CoordinatorError::Faulted,
                _ => CoordinatorError::Limit,
            })
            .and_then(|bytes| {
                if self.root_byte_ceiling.is_some_and(|limit| bytes > limit) {
                    Err(CoordinatorError::Limit)
                } else {
                    Ok(())
                }
            });
        super::observe(recorder, Boundary::SemanticRootAdmission, &result);
        result
    }

    pub(super) fn check_compound_root(
        &self,
        prepared: &PreparedTransaction,
        annotations: &annotation::AnnotationDecision,
        views: &views::StagedViews,
        history: &history::HistoryDecision,
        recorder: &mut Recorder,
    ) -> std::result::Result<(), CoordinatorError> {
        self.check_compound_root_mode(prepared, annotations, views, history, false, recorder)
    }
    pub(super) fn check_compound_advice_root(
        &self,
        prepared: &PreparedTransaction,
        annotations: &annotation::AnnotationDecision,
        views: &views::StagedViews,
        history: &history::HistoryDecision,
        recorder: &mut Recorder,
    ) -> std::result::Result<(), CoordinatorError> {
        self.check_compound_root_mode(prepared, annotations, views, history, true, recorder)
    }
    #[allow(clippy::too_many_arguments)]
    fn check_compound_root_mode(
        &self,
        prepared: &PreparedTransaction,
        annotations: &annotation::AnnotationDecision,
        views: &views::StagedViews,
        history: &history::HistoryDecision,
        advisory: bool,
        recorder: &mut Recorder,
    ) -> std::result::Result<(), CoordinatorError> {
        if self.root_byte_ceiling.is_none() {
            return Ok(());
        }
        let candidate = (|| {
            let mut root = self.measure_semantic_root()?;
            root.documents_bytes = self.documents_root_bytes(Some((prepared, history)))?;
            root.annotations_bytes = if advisory {
                self.annotations.root_bytes_after_advice(
                    annotations,
                    history,
                    &self.documents,
                    &self.namespace,
                )?
            } else {
                self.annotations.root_bytes_after(
                    annotations,
                    history,
                    &self.documents,
                    &self.namespace,
                )?
            };
            root.views_bytes = self.views.root_bytes_after(views)?;
            root.history_bytes = history.root_bytes_after(&self.history)?;
            root.retotal()
        })();
        self.check_root_candidate(candidate, recorder)
    }
    /// Immutable build-local logical ceiling. Settings preparation requires this
    /// explicit profile. Current native growth routes also preserve pending slot
    /// obligations; future mutation routes must compose the same admission law.
    pub fn configure_semantic_root_budget(
        &mut self,
        max_bytes: usize,
        recorder: &mut Recorder,
    ) -> Result<bool> {
        let result = (|| {
            self.ensure_operational()
                .map_err(|_| RootAccountingErrorV1::Faulted)?;
            if let Some(old) = self.root_byte_ceiling {
                return if old == max_bytes {
                    Ok(false)
                } else {
                    Err(RootAccountingErrorV1::Unavailable)
                };
            }
            if self.root_capacity_bytes(
                self.measure_semantic_root()?.total_bytes,
                resource::BaselineRootProjection::default(),
            )? > max_bytes
            {
                return Err(RootAccountingErrorV1::Limit);
            }
            self.root_byte_ceiling = Some(max_bytes);
            Ok(true)
        })();
        if result == Err(RootAccountingErrorV1::Faulted) {
            self.faulted = true;
        }
        recorder.record(Observation {
            boundary: Boundary::SemanticRootBudgetConfiguration,
            disposition: match result {
                Ok(_) => Disposition::Completed,
                Err(RootAccountingErrorV1::Faulted) => Disposition::Unknown,
                Err(_) => Disposition::Refused,
            },
            service_time: None,
        });
        result
    }
    pub fn semantic_root_byte_ceiling(&self) -> Option<usize> {
        self.root_byte_ceiling
    }

    pub(super) fn check_settings_root_bytes(
        &self,
        candidate_bytes: usize,
        recorder: &mut Recorder,
    ) -> std::result::Result<(), crate::settings::SettingsError> {
        use crate::settings::SettingsError;
        let result = (|| {
            let ceiling = self.root_byte_ceiling.ok_or(SettingsError::Unavailable)?;
            let current = self
                .measure_semantic_root()
                .map_err(|_| SettingsError::Limit)?;
            let total = current
                .total_bytes
                .checked_sub(current.settings_bytes)
                .and_then(|n| n.checked_add(candidate_bytes))
                .ok_or(SettingsError::Limit)?;
            let total = self
                .root_capacity_bytes(total, resource::BaselineRootProjection::default())
                .map_err(|e| {
                    if e == RootAccountingErrorV1::Faulted {
                        SettingsError::Faulted
                    } else {
                        SettingsError::Limit
                    }
                })?;
            if total > ceiling {
                return Err(SettingsError::Limit);
            }
            Ok(())
        })();
        crate::settings::observe(recorder, Boundary::SemanticRootAdmission, &result);
        result
    }
    /// Read-only, allocation-free traversal of the current root. Does not impose
    /// a ceiling, retain a root, mint authority or validate publication health.
    pub fn semantic_root_usage(&self, recorder: &mut Recorder) -> Result<SemanticRootUsageV1> {
        let result = self.measure_semantic_root();
        recorder.record(Observation {
            boundary: Boundary::SemanticRootAccounting,
            disposition: if result.is_ok() {
                Disposition::Completed
            } else {
                Disposition::Refused
            },
            service_time: None,
        });
        result
    }

    pub(in crate::coordinator) fn measure_semantic_root(&self) -> Result<SemanticRootUsageV1> {
        // Exhaustive inventory: adding an owner field requires revisiting its
        // semantic/retained-work classification, even when its cost is zero here.
        let Self {
            diagnostic_capture: _, // Separate funded diagnostic owner, not semantic payload.
            shutdown: _,           // Separately funded admission/control retention.
            registration_bases: _, // Separate prepaid authority retention, not authored root.
            discovery: _,          // Private attested-registry reader, separately sponsored.
            root_byte_ceiling: _,
            configured_line_limits: _,
            retention: _,
            publication_measurements: _,
            release_measurements: _,
            invocation_measurements: _,
            source_contexts: _,
            publication_streams: _, // Parent-funded retained work, not authored root.
            facet_publication_streams: _, // Parent-funded retained work, not authored root.
            collection_publication_streams: _, // Parent-funded retained work, not authored root.
            registry_observations: _, // Parent-funded metadata observations, not authored root.
            history_inspection: _,
            managed_search: _,
            annotation_preparations: _,
            annotation_contexts: _,
            version_set_preparations: _,
            version_set_contexts: _,
            settings_retention: _,
            settings_reader: _, // Separately funded recipient retention.
            catalog_reads: _,   // Separately funded metadata frontier retention.
            settings_plan_life: _,
            settings: _,
            settings_activation: _,
            settings_definition_limits: _,
            settings_layer_limits: _,
            settings_basis_profile: _,
            serialization_profile: _,
            serialization_bases: _,
            workspaces: _,
            boundaries: _,
            views: _,
            namespace: _,
            documents: _,
            generation: _,
            limits: _,
            invocations: _,
            history: _,
            resources: _,
            searches: _,
            annotations: _,
            faulted: _,
            read_result_invariant: _, // bounded owner-health code, not authored payload
            source_facade_busy: _,    // transient native gateway control, not authored payload
            source_facade_workspace: _,
            #[cfg(test)]
                source_facade_fault: _,
            #[cfg(test)]
                search_result_fault: _,
            #[cfg(test)]
                source_read_result_fault: _,
            #[cfg(test)]
                annotation_read_result_fault: _,
            #[cfg(test)]
                acquisition_result_fault: _,
            #[cfg(test)]
                publication_fault_after: _,
            #[cfg(test)]
                reversal_stage_fault: _,
            #[cfg(test)]
                relation_publication_fault: _,
            #[cfg(test)]
                settings_publication_fault: _,
            #[cfg(test)]
                created_result_fault_after: _,
            #[cfg(test)]
                annotation_context_install_fault: _,
            #[cfg(test)]
                version_set_preparation_fault: _,
            #[cfg(test)]
                version_set_install_fault: _,
            #[cfg(test)]
                version_set_selector_fault: _,
            #[cfg(test)]
                annotation_selector_registration_fault: _,
        } = self;
        // Explicit top-level projection: do NOT charge all of SourceCoordinator.
        let header_bytes = size_of::<(
            Arc<()>,
            u64,
            bool,
            Vec<Option<Document>>,
            Option<Arc<crate::settings::CompiledSettingsV1>>,
        )>();
        let mut usage = SemanticRootUsageV1 {
            profile: NATIVE_SEMANTIC_ROOT_PROFILE_V1,
            header_bytes,
            documents_bytes: self.documents_root_bytes(None)?,
            history_bytes: self.history.root_bytes()?,
            annotations_bytes: self
                .annotations
                .root_bytes(&self.documents, &self.namespace)?,
            views_bytes: self.views.root_bytes()?,
            workspaces_bytes: self.workspaces.root_bytes()?,
            persistence_bytes: self.resources.root_bytes()?,
            settings_bytes: self
                .settings
                .as_ref()
                .map_or(0, |s| s.usage().logical_bytes),
            total_bytes: 0,
        };
        usage.total_bytes = usage.retotal()?;
        Ok(usage)
    }
    fn documents_root_bytes(
        &self,
        after: Option<(&PreparedTransaction, &history::HistoryDecision)>,
    ) -> Result<usize> {
        let mut documents = Bytes::default();
        documents.rows::<Option<Document>>(self.documents.len())?;
        for (slot, document) in self.documents.iter().enumerate() {
            let Some(document) = document else {
                continue;
            };
            let Document {
                insertion_policy: _,
                serialization: _,
                lifecycle: _,
                retained_bindings: _,
                source: _,
                revision: _,
                read_only: _,
                policy_generation: _,
                history: _,
                annotation_generation: _,
                annotation_policy: _,
                annotation_read_only: _,
            } = document;
            let source = after
                .and_then(|(prepared, _)| {
                    prepared
                        .candidates
                        .iter()
                        .find(|c| c.expected.document.slot == slot)
                })
                .map_or(&document.source, |c| &c.source);
            documents.add(source.text().len())?;
            let frontier = if let Some((_, history)) = after {
                history.source_frontier_bytes(slot, &document.history)?
            } else {
                document.history.root_payload_bytes()?
            };
            documents.add(frontier)?;
            documents.add(document.retained_bindings.root_payload_bytes()?)?;
        }
        Ok(documents.finish())
    }
}

impl SemanticRootUsageV1 {
    fn retotal(&self) -> Result<usize> {
        let mut total = Bytes::default();
        for n in [
            self.header_bytes,
            self.documents_bytes,
            self.history_bytes,
            self.annotations_bytes,
            self.views_bytes,
            self.workspaces_bytes,
            self.persistence_bytes,
            self.settings_bytes,
        ] {
            total.add(n)?;
        }
        Ok(total.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn candidate_retotal_does_not_trust_the_cached_total_or_wrap_component_sums() {
        let mut usage = SemanticRootUsageV1 {
            profile: NATIVE_SEMANTIC_ROOT_PROFILE_V1,
            header_bytes: usize::MAX,
            documents_bytes: 1,
            history_bytes: 0,
            annotations_bytes: 0,
            views_bytes: 0,
            workspaces_bytes: 0,
            persistence_bytes: 0,
            settings_bytes: 0,
            total_bytes: 0,
        };
        assert_eq!(usage.retotal(), Err(RootAccountingErrorV1::Overflow));
        usage.header_bytes -= 1;
        assert_eq!(usage.retotal(), Ok(usize::MAX));
        assert_eq!(usage.total_bytes, 0);
    }
    #[test]
    fn arithmetic_never_wraps_or_saturates_into_an_admissible_measure() {
        let mut b = Bytes::default();
        b.add(usize::MAX).unwrap();
        assert_eq!(b.add(1), Err(RootAccountingErrorV1::Overflow));
        assert_eq!(b.finish(), usize::MAX);
        assert!(Bytes::default().rows::<u64>(usize::MAX).is_err());
    }
    #[test]
    fn spare_document_allocation_is_not_semantic_state() {
        let mut owner = SourceCoordinator::new(CoordinatorLimits {
            max_document_incarnations: 16,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 4096,
                max_edits: 8,
                max_edit_payload_bytes: 4096,
            },
        });
        let before = owner.measure_semantic_root().unwrap();
        owner.documents.reserve(16);
        assert_eq!(owner.measure_semantic_root().unwrap(), before);
        // A spent issuer slot is semantic even with no retained document body.
        owner.documents.push(None);
        assert_eq!(
            owner.measure_semantic_root().unwrap().documents_bytes,
            before.documents_bytes + size_of::<Option<Document>>()
        );
    }
}
