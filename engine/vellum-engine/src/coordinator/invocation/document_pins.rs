//! Original native document references in the one invocation ledger.
use super::*;
#[cfg(test)]
mod tests;

impl SourceIntent {
    pub(super) fn refers_to_document(&self, d: &DocumentId) -> bool {
        self.writes.iter().any(|w| w.expected.document == *d)
            || self.reads.iter().any(|v| v.document == *d)
    }
}
impl CompoundIntent {
    fn refers_to_document(&self, d: &DocumentId) -> bool {
        self.source.refers_to_document(d) || self.annotations.iter().any(|a| mutation_refers(a, d))
    }
}
fn mutation_refers(a: &AnnotationMutation, d: &DocumentId) -> bool {
    match a {
        AnnotationMutation::Create { target, .. } => target.source.document == *d,
        AnnotationMutation::UpdateBody { annotation, .. }
        | AnnotationMutation::Delete { annotation, .. } => annotation.document == *d,
        AnnotationMutation::Reattach {
            annotation, target, ..
        } => annotation.document == *d || target.source.document == *d,
    }
}
fn scope_refers(s: &scope::ScopeCapture, d: &DocumentId) -> bool {
    s.scopes.iter().any(|s| s.expected.document == *d)
}
fn changes_refers(changes: &[SourceChange], d: &DocumentId) -> bool {
    changes
        .iter()
        .any(|c| c.before.document == *d || c.after.document == *d)
}
impl BoundIntent {
    fn refers_to_document(&self, d: &DocumentId) -> bool {
        match self {
            // Fresh creation and weak resource authority have no existing
            // document target. Their allocated outputs are counted separately.
            Self::Scratch(_)
            | Self::RetiredSearchReplacement
            | Self::RetiredAtomicAuthoring
            | Self::SourceLoad(_)
            | Self::ResourceCatalog(_)
            | Self::Settings(_)
            | Self::Relation { .. }
            | Self::RetiredScratch
            | Self::RetiredLiveRelation
            | Self::RetiredRelease
            | Self::RetiredRebind
            | Self::RetiredLoad
            | Self::RetiredSaveAs
            | Self::RetiredAnnotationLoad
            | Self::RetiredDisposal
            | Self::RetiredAnnotation
            | Self::Reversal { .. } => false,
            Self::NativeSettings(c) => c.refers_to_document(d),
            Self::LiveRelation(c) => c.refers_to_document(d),
            Self::RetainedRelease(c) => c.refers_to_document(d),
            Self::SourceRebind(c) => c.refers_to_document(d),
            Self::SourceSaveAs(c) => c.refers_to_document(d),
            Self::AnnotationLoad(c) => c.refers_to_document(d),
            Self::RetainedDisposal(c) => c.refers_to_document(d),
            Self::Annotation(c) => c.refers_to_document(d),
            Self::ManagedSearch(c) => c.refers_to_document(d),
            Self::ReturnedSearchReplacement(c) => c.consumer.refers_to_document(d),
            Self::ScopedLifecycle(c) => c.refers_to_document(d),
            Self::ScopedDefinition(c) => c.refers_to_document(d),
            Self::Serialization(c) => c.refers_to_document(d),
            Self::ScopedSettings(c) => c.refers_to_document(d),
            Self::SettingsLayers(c) => c.refers_to_document(d),
            Self::SettingsDefinition(c) => c.refers_to_document(d),
            Self::DependentSettings(c) => c.refers_to_document(d),
            Self::ConfiguredLine(c) => c.refers_to_document(d),
            Self::Source(s) => s.refers_to_document(d),
            Self::ScopedSource { source, scope } => {
                source.refers_to_document(d) || scope_refers(scope, d)
            }
            Self::ScopedCompound { intent, scope } => {
                intent.refers_to_document(d) || scope_refers(scope, d)
            }
            Self::DependentCompound {
                intent,
                reads,
                scope,
            } => {
                intent.refers_to_document(d)
                    || reads.refers_to_document(d)
                    || scope.as_ref().is_some_and(|s| scope_refers(s, d))
            }
            Self::Compound(c) => c.refers_to_document(d),
            Self::LiteralEdit(c) => c.context.source.document == *d,
            Self::Composition(c) => c.view.source.document == *d,
            Self::ExactBatch(c) => c.refers_to_document(d),
            Self::AtomicAuthoring(c) => c.refers_to_document(d),
            Self::LineEdit(c) => c.context.source.document == *d,
            Self::Unicode(c) => c.context.source.document == *d,
            Self::Selection { context, .. } => context.source.document == *d,
            Self::SearchReplacement { selected, .. } => selected.refers_to_document(d),
            Self::DependentReversal(c) => {
                c.reads.refers_to_document(d)
                    || match &c.target {
                        HistoryReversalTargetV1::Group(_) => false,
                        HistoryReversalTargetV1::SourceFrontier { document, .. } => document == d,
                        HistoryReversalTargetV1::AnnotationFrontier { annotation, .. } => {
                            annotation.document == *d
                        }
                    }
            }
        }
    }
}
impl PreparedTransaction {
    pub(in crate::coordinator) fn refers_to_document(&self, d: &DocumentId) -> bool {
        self.candidates.iter().any(|c| c.expected.document == *d)
            || self.read_dependencies.iter().any(|v| v.document == *d)
            || self.annotations.iter().any(|a| a.id.document == *d)
            || self
                .view_dependencies
                .iter()
                .any(|v| v.refers_to_document(d))
            || self
                .declared_reads
                .as_ref()
                .is_some_and(|r| r.refers_to_document(d))
            || self.scopes.as_ref().is_some_and(|s| scope_refers(s, d))
            || self
                .search_dependency
                .as_ref()
                .is_some_and(|s| s.refers_to_document(d))
            || self.history.refers_to_document(d)
    }
}
impl Phase {
    fn refers_to_document(&self, d: &DocumentId) -> Result<bool, CoordinatorError> {
        Ok(match self {
            Self::Running | Self::AnnotationFaulted => return Err(CoordinatorError::Faulted),
            Self::ScratchPending(_)
            | Self::SourceLoadPending(_)
            | Self::ResourceCatalogPending(_)
            | Self::SettingsPending(_)
            | Self::RelationPending(_) => false,
            Self::LiveRelationPending(c) => c.refers_to_document(d),
            Self::RetainedReleasePending(c) => c.refers_to_document(d),
            Self::SourceRebindPending(c) => c.refers_to_document(d),
            Self::SourceSaveAsPending(c) => c.refers_to_document(d),
            Self::AnnotationLoadPending(c) => c.refers_to_document(d),
            Self::RetainedDisposalPending(c) => c.refers_to_document(d),
            Self::AnnotationPending(p) => p.refers_to_document(d),
            Self::NativeSettingsPending(p) => p.refers_to_document(d),
            Self::ManagedSearchPending(p) => {
                p.capture.refers_to_document(d)
                    || match &p.plan {
                        source_context::ConsumerPlan::Source(p) => {
                            p.as_ref().is_ok_and(|p| p.refers_to_document(d))
                        }
                        source_context::ConsumerPlan::Selection {
                            context, prepared, ..
                        } => {
                            context.source.document == *d
                                || prepared.as_ref().is_ok_and(|p| p.refers_to_document(d))
                        }
                    }
            }
            Self::ScopedLifecyclePending(p) => p.capture.refers_to_document(d),
            Self::ScopedDefinitionPending(p) => p.capture.refers_to_document(d),
            Self::ScopedSettingsPending(p) => p.capture.refers_to_document(d),
            Self::SerializationPending(p) => p.capture.refers_to_document(d),
            Self::SettingsLayersPending(p) => {
                p.as_ref().is_ok_and(|p| p.reads.refers_to_document(d))
            }
            Self::SettingsDefinitionPending(p) => {
                p.as_ref().is_ok_and(|p| p.reads.refers_to_document(d))
            }
            Self::DependentSettingsPending(p) => {
                p.as_ref().is_ok_and(|p| p.reads.refers_to_document(d))
            }
            Self::ConfiguredLinePending { prepared, reserved } => {
                prepared.as_ref().is_ok_and(|p| p.refers_to_document(d))
                    || changes_refers(reserved, d)
            }
            Self::Issued(reserved) => changes_refers(reserved, d),
            Self::Pending { prepared, reserved } => {
                prepared.as_ref().is_ok_and(|p| p.refers_to_document(d))
                    || changes_refers(reserved, d)
            }
            Self::SelectionPending(p) => p.as_ref().is_ok_and(|p| p.refers_to_document(d)),
            Self::UnicodeMovementPending(p) => p.as_ref().is_ok_and(|p| p.refers_to_document(d)),
            Self::Terminal(outcome) => outcome.as_ref().is_some_and(|o| o.refers_to_document(d)),
        })
    }
}
impl InvocationTable {
    /// Retained slots, not captures/allocations or bytes; one incidence per slot.
    pub(in crate::coordinator) fn document_pins(
        &self,
        d: &DocumentId,
    ) -> Result<usize, CoordinatorError> {
        if self.slots.iter().flatten().count() != self.retained {
            return Err(CoordinatorError::Faulted);
        }
        let mut count = 0usize;
        for slot in self.slots.iter().flatten() {
            // These funded captures are paired at their actual admissions and
            // retired together. Missing/mismatched ownership is a surprise,
            // never an empty document contribution after result forgetting.
            let paired = [
                (
                    slot.annotation_hold.is_some(),
                    matches!(slot.intent, Some(BoundIntent::Annotation(_))),
                ),
                (
                    slot.disposal_hold.is_some(),
                    matches!(slot.intent, Some(BoundIntent::RetainedDisposal(_))),
                ),
                (
                    slot.release_hold.is_some(),
                    matches!(slot.intent, Some(BoundIntent::RetainedRelease(_))),
                ),
                (
                    slot.rebind_hold.is_some(),
                    matches!(slot.intent, Some(BoundIntent::SourceRebind(_))),
                ),
                (
                    slot.load_hold.is_some(),
                    matches!(slot.intent, Some(BoundIntent::SourceLoad(_))),
                ),
                (
                    slot.saveas_hold.is_some(),
                    matches!(slot.intent, Some(BoundIntent::SourceSaveAs(_))),
                ),
                (
                    slot.annotation_load_hold.is_some(),
                    matches!(slot.intent, Some(BoundIntent::AnnotationLoad(_))),
                ),
                (
                    slot.live_relation_hold.is_some(),
                    matches!(slot.intent, Some(BoundIntent::LiveRelation(_))),
                ),
                (
                    slot.scratch_hold.is_some(),
                    matches!(slot.intent, Some(BoundIntent::Scratch(_))),
                ),
            ];
            if paired.iter().any(|(hold, capture)| hold != capture)
                || (slot.compound_results.is_some()
                    && !matches!(
                        slot.intent,
                        Some(
                            BoundIntent::Compound(_)
                                | BoundIntent::AtomicAuthoring(_)
                                | BoundIntent::ScopedCompound { .. }
                                | BoundIntent::DependentCompound { .. }
                        )
                    ))
                || (matches!(slot.intent, Some(BoundIntent::AtomicAuthoring(_)))
                    && slot.compound_results.is_none())
            {
                return Err(CoordinatorError::Faulted);
            }
            let Slot {
                observation_policy: _,
                annotation_hold: _,
                compound_results: _,
                disposal_hold: _,
                release_hold: _,
                rebind_hold: _,
                load_hold,
                saveas_hold,
                annotation_load_hold,
                live_relation_hold: _,
                scratch_hold,
                intent,
                phase,
                created_results: _,
                stop_requested: _,
                measurement_cohort: _,
            } = slot;
            // Inspect phase integrity even if an input already matches.
            let phase_refers = phase.refers_to_document(d)?;
            let refers = intent.as_ref().is_some_and(|i| i.refers_to_document(d))
                || phase_refers
                || scratch_hold
                    .as_ref()
                    .and_then(|h| h.candidate.as_ref())
                    .is_some_and(|v| v.document == *d)
                || load_hold
                    .as_ref()
                    .and_then(|h| h.candidate.as_ref())
                    .is_some_and(|v| v.document.document == *d)
                || saveas_hold
                    .as_ref()
                    .and_then(|h| h.candidate.as_ref())
                    .is_some_and(|(v, _)| v == d)
                || annotation_load_hold
                    .as_ref()
                    .and_then(|h| h.candidate.as_ref())
                    .is_some_and(|v| v.document() == d);
            if refers {
                count = count.checked_add(1).ok_or(CoordinatorError::Faulted)?;
            }
        }
        Ok(count)
    }
}
impl InvocationOutcome {
    fn refers_to_document(&self, d: &DocumentId) -> bool {
        match self {
            Self::ScratchCommitted(v) => v.document == *d,
            Self::SourceLoadCommitted(v) => v.document.document == *d,
            Self::SourceSaveAsCommitted(v, _) => v == d,
            Self::AnnotationLoadCommitted(v) => v.document() == d,
            Self::Committed(v) => {
                changes_refers(&v.changes, d)
                    || v.annotation_changes
                        .iter()
                        .any(|a| a.annotation.document == *d)
            }
            Self::ScratchRejected(_)
            | Self::LiveRelationCommitted(_)
            | Self::LiveRelationRejected(_)
            | Self::RetainedReleaseCommitted(_)
            | Self::SourceRebindCommitted(_)
            | Self::SourceRebindRejected(_)
            | Self::SourceLoadRejected(_)
            | Self::SourceSaveAsRejected(_)
            | Self::AnnotationLoadRejected(_)
            | Self::RetainedReleaseRejected(_)
            | Self::RetainedDisposalCommitted(_)
            | Self::RetainedDisposalRejected(_)
            | Self::ResourceCatalogCommitted(_)
            | Self::ResourceCatalogRejected(_)
            | Self::ScopedLifecycleCommitted(_)
            | Self::ScopedLifecycleRejected(_)
            | Self::ScopedDefinitionCommitted(_)
            | Self::ScopedDefinitionRejected(_)
            | Self::SerializationCommitted(_)
            | Self::SerializationRejected(_)
            | Self::ScopedSettingsCommitted(_)
            | Self::ScopedSettingsRejected(_)
            | Self::SettingsLayersCommitted(_)
            | Self::SettingsLayersRejected(_)
            | Self::SettingsDefinitionCommitted(_)
            | Self::SettingsDefinitionRejected(_)
            | Self::SettingsRejectedV2(_)
            | Self::ConfiguredLineRejected(_)
            | Self::SettingsCommitted(_)
            | Self::SettingsRejected(_)
            | Self::RelationCommitted(_)
            | Self::RelationRejected(_)
            | Self::Rejected(_)
            | Self::SelectionCommitted(_)
            | Self::SelectionRejected(_)
            | Self::LineRejected(_)
            | Self::UnicodeMovementCommitted(_)
            | Self::UnicodeRejected(_)
            | Self::StoppedBeforePublication => false,
        }
    }
}
