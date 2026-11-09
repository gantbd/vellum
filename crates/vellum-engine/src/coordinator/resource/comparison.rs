//! Pure source-output comparison. No provider probe, task or artifact issuance.
use super::*;
#[cfg(test)]
mod tests;

fn preflight(
    encoded: usize,
    baseline: usize,
    budget: SourceComparisonBudgetV1,
) -> Result<(), ResourceError> {
    let combined = encoded.checked_add(baseline).ok_or(ResourceError::Limit)?;
    if encoded > budget.max_encoded_bytes || combined > budget.max_combined_bytes {
        return Err(ResourceError::Limit);
    }
    Ok(())
}
impl SourceCoordinator {
    /// Exact historical output comparison under caller-selected logical bounds.
    /// Equality establishes neither external freshness nor synchronization.
    pub fn saved_state_with_budget(
        &self,
        document: &DocumentId,
        budget: SourceComparisonBudgetV1,
        recorder: &mut Recorder,
    ) -> Result<SavedState, ResourceError> {
        self.saved_state_checked(document, Some(budget), recorder)
    }

    pub(super) fn saved_state_checked(
        &self,
        document: &DocumentId,
        budget: Option<SourceComparisonBudgetV1>,
        recorder: &mut Recorder,
    ) -> Result<SavedState, ResourceError> {
        let timing = recorder.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            let source = self.document(document).map_err(owner_error)?;
            let Some(binding) = self
                .resources
                .bindings
                .iter()
                .find(|b| b.document == *document)
            else {
                return Ok(SavedState {
                    content: ContentComparison::Unbound,
                    queued: 0,
                    unresolved: 0,
                    contested: false,
                });
            };
            if !source.serialization.valid() {
                return Err(ResourceError::Faulted);
            }
            let baseline = binding
                .baseline
                .as_ref()
                .ok_or(ResourceError::MissingEvidence)?;
            let budget = match budget {
                Some(budget) => budget,
                None => {
                    let per_blob = self
                        .resources
                        .limits
                        .ok_or(ResourceError::Unsupported)?
                        .max_blob_bytes;
                    SourceComparisonBudgetV1 {
                        max_encoded_bytes: per_blob,
                        // The compatibility bound is the sum of two configured
                        // blob ceilings, clamped to the native address space.
                        // Actual combined sizes still use checked arithmetic.
                        max_combined_bytes: per_blob.saturating_mul(2),
                    }
                }
            };
            let length = source
                .serialization
                .serialization
                .encoded_len(&source.source)
                .ok_or(ResourceError::Limit)?;
            preflight(length, baseline.bytes.0.len(), budget)?;
            // Only the intrinsic codec runs here, after sizing; no provider,
            // artifact row, retained reservation or persistent issuance is used.
            let encoded = source
                .serialization
                .serialization
                .encode(&source.source, recorder)
                .map_err(ResourceError::Source)?;
            if encoded.len() != length {
                return Err(ResourceError::Faulted);
            }
            Ok(SavedState {
                content: if encoded.as_slice() == baseline.bytes.0.as_ref() {
                    ContentComparison::Unchanged
                } else {
                    ContentComparison::Modified
                },
                queued: self
                    .resources
                    .tasks
                    .iter()
                    .flatten()
                    .filter(|t| t.identity == binding.identity && t.phase == SavePhase::Queued)
                    .count(),
                unresolved: self
                    .resources
                    .tasks
                    .iter()
                    .flatten()
                    .filter(|t| {
                        t.identity == binding.identity
                            && t.phase == SavePhase::Dispatched
                            && !t.safe()
                    })
                    .count(),
                contested: self.resource_contested(&binding.identity),
            })
        })();
        capture_service(recorder, Boundary::Query, &result, timing.finish());
        result
    }
}
