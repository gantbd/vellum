//! Shared current transaction predicates. No publication or live reservations.
use super::*;
pub(super) struct TransactionDecision {
    pub changed: usize,
    pub annotations: annotation::AnnotationDecision,
    pub views: views::StagedViews,
}
impl SourceCoordinator {
    pub(super) fn decide_transaction_candidate(
        &self,
        prepared: &PreparedTransaction,
        advisory: bool,
        recorder: &mut Recorder,
    ) -> Result<TransactionDecision, CoordinatorError> {
        self.ensure_admission_unfenced()?;
        if !Arc::ptr_eq(&self.namespace, &prepared.namespace) {
            return Err(CoordinatorError::Unavailable);
        }
        self.validate_configured_line(prepared, recorder)?;
        for read in &prepared.read_dependencies {
            self.validate_version(read)?;
        }
        self.validate_prepared_scopes(prepared, recorder)?;
        self.validate_prepared_reads(prepared, recorder)?;
        if let Some(result) = &prepared.search_dependency {
            self.validate_search_selection(result)
                .map_err(CoordinatorError::Search)?;
        }
        let mut changed = 0usize;
        for candidate in &prepared.candidates {
            let document = self.validate_version(&candidate.expected)?;
            if document.read_only {
                return Err(CoordinatorError::Unavailable);
            }
            if document.policy_generation != candidate.policy_generation {
                return Err(CoordinatorError::Stale);
            }
            let ambiguous = matches!(prepared.history, history::PreparedHistory::Ordinary)
                && candidate
                    .edits
                    .windows(2)
                    .any(|pair| crate::source::edits_ambiguous(&pair[0], &pair[1]));
            if ambiguous
                || !document
                    .source
                    .matches_journal(&candidate.edits, &candidate.source)
            {
                return Err(CoordinatorError::Faulted);
            }
            if document.source != candidate.source {
                document
                    .revision
                    .checked_add(1)
                    .ok_or(CoordinatorError::VersionExhausted)?;
                changed += 1;
            }
        }
        self.validate_reversal_plan(prepared, recorder)
            .map_err(CoordinatorError::History)?;
        if advisory {
            self.validate_advisory_annotation_plan(prepared, recorder)?;
        } else {
            self.validate_ordinary_annotation_plan(prepared, recorder)?;
        }
        let annotations = if advisory {
            self.stage_advisory_annotations(prepared)
        } else {
            self.stage_annotations(prepared)
        };
        if self.annotations.limits.is_some() {
            observe(recorder, Boundary::AnnotationStaging, &annotations);
        }
        let annotations = annotations?;
        let views = self.stage_view_companions(prepared, recorder)?;
        Ok(TransactionDecision {
            changed,
            annotations,
            views,
        })
    }
}
