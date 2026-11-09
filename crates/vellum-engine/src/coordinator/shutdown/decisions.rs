//! Exact pre-acceptance document choices. This captures intent, not save
//! readiness, authority to discard, accepted shutdown or completed disposition.
use super::*;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownDocumentChoiceV1 {
    Save,
    Discard,
}
pub struct ShutdownDocumentDecisionV1<'a> {
    /// Exact expectations acknowledged by the host, never a movable latest ID.
    pub expected: &'a ShutdownDocumentRecordV1,
    pub source: Option<ShutdownDocumentChoiceV1>,
    pub annotations: Option<ShutdownDocumentChoiceV1>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShutdownDecisionNeedV1 {
    pub document: DocumentId,
    pub source: bool,
    pub annotations: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShutdownCapturedDecisionV1 {
    pub expected: ShutdownDocumentRecordV1,
    pub source: ShutdownDocumentChoiceV1,
    pub annotations: ShutdownDocumentChoiceV1,
}
pub struct ShutdownDecisionsV1 {
    pub(super) marker: Arc<()>,
    pub(super) original_inventory: Arc<()>,
    pub(super) decisions: Vec<ShutdownCapturedDecisionV1>,
}
impl std::fmt::Debug for ShutdownDecisionsV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ShutdownDecisionsV1(<opaque>)")
    }
}
pub enum ShutdownDecisionCaptureV1 {
    NeedsDecision(Vec<ShutdownDecisionNeedV1>),
    Captured(ShutdownDecisionsV1),
}
impl std::fmt::Debug for ShutdownDecisionCaptureV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ShutdownDecisionCaptureV1(<opaque>)")
    }
}
fn bytes<T>(count: usize, documents: usize, maximum: usize) -> Result<(), CoordinatorError> {
    count
        .checked_mul(size_of::<T>())
        .and_then(|n| {
            n.checked_add(
                documents.checked_mul(size_of::<Option<&ShutdownDocumentDecisionV1<'_>>>())?,
            )
        })
        .and_then(|n| n.checked_add(size_of::<Vec<Option<&ShutdownDocumentDecisionV1<'_>>>>()))
        .and_then(|n| n.checked_add(size_of::<ShutdownDecisionCaptureV1>()))
        .filter(|n| *n <= maximum)
        .map(|_| ())
        .ok_or(CoordinatorError::Limit)
}
impl SourceCoordinator {
    /// The trusted host supplies acknowledged choices for this exact native cut.
    /// Missing choices neither freeze unrelated work nor create implicit saves.
    pub fn capture_shutdown_decisions_v1(
        &self,
        inventory: &ShutdownInventoryV1,
        input: &[ShutdownDocumentDecisionV1<'_>],
        max_capture_bytes: usize,
        f: &TrustedShutdownInventoryFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ShutdownDecisionCaptureV1, CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.validate_shutdown_inventory_current_v1(inventory, f, r)?;
            let cut = self.read_shutdown_inventory_v1(inventory, f, r)?;
            // Bound supplied input before matching; duplicates/foreign/disposed
            // rows are malformed choices, not missing authority for other rows.
            if input.len() > cut.documents.len() {
                return Err(CoordinatorError::Limit);
            }
            bytes::<ShutdownDecisionNeedV1>(0, cut.documents.len(), max_capture_bytes)?;
            let mut indexed = Vec::new();
            indexed
                .try_reserve_exact(cut.documents.len())
                .map_err(|_| CoordinatorError::Capacity)?;
            indexed.resize(cut.documents.len(), None);
            for d in input {
                let index = cut
                    .documents
                    .binary_search_by_key(&d.expected.source.document.slot, |row| {
                        row.source.document.slot
                    })
                    .map_err(|_| CoordinatorError::Unavailable)?;
                let original = &cut.documents[index];
                if original.source.document != d.expected.source.document
                    || original.lifecycle.phase != LifecyclePhase::Live
                    || indexed[index].is_some()
                {
                    return Err(CoordinatorError::Unavailable);
                }
                if original != d.expected {
                    return Err(CoordinatorError::Stale);
                }
                indexed[index] = Some(d);
            }
            let live = cut
                .documents
                .iter()
                .enumerate()
                .filter(|(_, d)| d.lifecycle.phase == LifecyclePhase::Live);
            let missing = live
                .clone()
                .filter(|(i, _)| {
                    indexed[*i].is_none_or(|v| v.source.is_none() || v.annotations.is_none())
                })
                .count();
            if missing != 0 {
                bytes::<ShutdownDecisionNeedV1>(missing, cut.documents.len(), max_capture_bytes)?;
                let mut rows = Vec::new();
                rows.try_reserve_exact(missing)
                    .map_err(|_| CoordinatorError::Capacity)?;
                for (i, d) in live {
                    let choice = indexed[i];
                    let source = choice.is_none_or(|v| v.source.is_none());
                    let annotations = choice.is_none_or(|v| v.annotations.is_none());
                    if source || annotations {
                        rows.push(ShutdownDecisionNeedV1 {
                            document: d.source.document.clone(),
                            source,
                            annotations,
                        });
                    }
                }
                return Ok(ShutdownDecisionCaptureV1::NeedsDecision(rows));
            }
            bytes::<ShutdownCapturedDecisionV1>(
                live.clone().count(),
                cut.documents.len(),
                max_capture_bytes,
            )?;
            let mut decisions = Vec::new();
            decisions
                .try_reserve_exact(live.clone().count())
                .map_err(|_| CoordinatorError::Capacity)?;
            for (i, d) in live {
                let chosen = indexed[i].ok_or(CoordinatorError::Faulted)?;
                decisions.push(ShutdownCapturedDecisionV1 {
                    expected: d.clone(),
                    source: chosen.source.ok_or(CoordinatorError::Faulted)?,
                    annotations: chosen.annotations.ok_or(CoordinatorError::Faulted)?,
                });
            }
            Ok(ShutdownDecisionCaptureV1::Captured(ShutdownDecisionsV1 {
                marker: Arc::new(()),
                original_inventory: inventory.marker.clone(),
                decisions,
            }))
        })();
        observe_service(
            r,
            Boundary::ShutdownDecisionCapture,
            &result,
            timer.finish(),
        );
        result
    }
    /// Fresh consent/readiness/authority is still required at acceptance and
    /// destructive disposition. This predicate proves original-cut correlation
    /// and current captured expectations only, not those later conditions.
    pub fn validate_shutdown_decisions_current_v1(
        &self,
        inventory: &ShutdownInventoryV1,
        decisions: &ShutdownDecisionsV1,
        f: &TrustedShutdownInventoryFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            if !Arc::ptr_eq(&inventory.marker, &decisions.original_inventory) {
                return Err(CoordinatorError::Unavailable);
            }
            self.validate_shutdown_inventory_current_v1(inventory, f, r)?;
            let cut = self.read_shutdown_inventory_v1(inventory, f, r)?;
            let live = cut
                .documents
                .iter()
                .filter(|d| d.lifecycle.phase == LifecyclePhase::Live);
            if !live.eq(decisions.decisions.iter().map(|d| &d.expected)) {
                return Err(CoordinatorError::Stale);
            }
            Ok(())
        })();
        observe_service(
            r,
            Boundary::ShutdownDecisionValidation,
            &result,
            timer.finish(),
        );
        result
    }
}
