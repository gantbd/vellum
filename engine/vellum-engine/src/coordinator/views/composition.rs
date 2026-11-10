//! One accepted literal replacement. No preedit, focus or platform index state.
use super::*;

pub(in crate::coordinator) struct CompositionEdit {
    context: LogicalViewContext,
    replacement: Replacement,
}
impl std::fmt::Debug for CompositionEdit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CompositionEdit(<redacted>)")
    }
}
pub(in crate::coordinator) fn copy_replacement(
    edit: &Replacement,
) -> Result<Replacement, SelectionError> {
    fn text(value: &str) -> Result<String, SelectionError> {
        let mut result = String::new();
        result
            .try_reserve_exact(value.len())
            .map_err(|_| SelectionError::Capacity)?;
        result.push_str(value);
        Ok(result)
    }
    Ok(Replacement {
        start: edit.start,
        end: edit.end,
        expected: text(&edit.expected)?,
        inserted: text(&edit.inserted)?,
    })
}
fn capture_slot(
    prepared: &PreparedTransaction,
) -> Result<Option<&CompositionEdit>, SelectionError> {
    match prepared.derived_edit.as_slice() {
        [] => Ok(None),
        [DerivedEdit::Composition(edit)] => Ok(Some(edit)),
        [_] => Ok(None),
        _ => Err(SelectionError::Faulted),
    }
}
impl SourceCoordinator {
    /// Freeze exactly one original-snapshot UTF-8 replacement, which need not
    /// equal the primary selection. Commit atomically maps all companions and
    /// collapses only this view's original primary before normalization.
    /// Trusted embedding API: this method does not authenticate its caller.
    pub fn prepare_composition_replacement_v1(
        &self,
        context: &LogicalViewContext,
        replacement: &Replacement,
        r: &mut Recorder,
    ) -> Result<PreparedTransaction, SelectionError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(error)?;
            self.view_context(context)?;
            let document = self.validate_version(&context.source).map_err(error)?;
            if document.read_only {
                return Err(SelectionError::Unavailable);
            }
            if self.limits.source.max_edits == 0 {
                return Err(SelectionError::Source(SourceError::EditLimit));
            }
            let payload = replacement
                .expected
                .len()
                .checked_add(replacement.inserted.len())
                .ok_or(SelectionError::Source(SourceError::PayloadLimit))?;
            if payload > self.limits.source.max_edit_payload_bytes {
                return Err(SelectionError::Source(SourceError::PayloadLimit));
            }
            let mut edits = Vec::new();
            edits
                .try_reserve_exact(1)
                .map_err(|_| SelectionError::Capacity)?;
            edits.push(copy_replacement(replacement)?);
            let mut prepared = self
                .prepare_inner(
                    &[DocumentEdits {
                        expected: context.source.clone(),
                        edits,
                    }],
                    &[],
                )
                .map_err(error)?;
            prepared
                .derived_edit
                .try_reserve_exact(1)
                .map_err(|_| SelectionError::Capacity)?;
            prepared
                .derived_edit
                .push(DerivedEdit::Composition(CompositionEdit {
                    context: context.clone(),
                    replacement: copy_replacement(replacement)?,
                }));
            prepared.derived_kind = Some(DerivedKind::Composition);
            Ok(prepared)
        })();
        capture_view_service(
            r,
            Boundary::CompositionReplacementPreparation,
            &result,
            timer.finish(),
        );
        result
    }
    pub(in crate::coordinator) fn composition_intent_matches(
        prepared: &PreparedTransaction,
        context: &LogicalViewContext,
        replacement: &Replacement,
    ) -> bool {
        prepared.derived_kind == Some(DerivedKind::Composition)
            && matches!(capture_slot(prepared), Ok(Some(edit)) if edit.context == *context && edit.replacement == *replacement)
    }
    pub(super) fn verify_composition_edit(
        &self,
        prepared: &PreparedTransaction,
    ) -> Result<(), SelectionError> {
        let Some(edit) = capture_slot(prepared)? else {
            return Ok(());
        };
        if prepared.candidates.len() != 1
            || !prepared.read_dependencies.is_empty()
            || !prepared.annotations.is_empty()
            || !matches!(prepared.history, history::PreparedHistory::Ordinary)
            || prepared.scoped
            || prepared.reads_required
            || prepared.configured_required
            || prepared.declared_reads.is_some()
            || prepared.scopes.is_some()
            || prepared.search_dependency.is_some()
            || prepared.configured_line.is_some()
        {
            return Err(SelectionError::Faulted);
        }
        let before = prepared
            .view_dependencies
            .iter()
            .find(|before| before.id == edit.context.view)
            .ok_or(SelectionError::Faulted)?;
        let candidate = &prepared.candidates[0];
        if before.source != edit.context.source
            || before.stamp != edit.context.stamp
            || candidate.expected != edit.context.source
            || candidate.edits.as_slice() != std::slice::from_ref(&edit.replacement)
        {
            return Err(SelectionError::Faulted);
        }
        Ok(())
    }
    pub(super) fn collapse_composition_primary(
        &self,
        prepared: &PreparedTransaction,
        before: &ViewDependency,
        candidate: &Candidate,
        records: &mut [Selection],
        remaining: &mut usize,
    ) -> Result<(), SelectionError> {
        let Some(edit) = capture_slot(prepared)? else {
            return Ok(());
        };
        if before.id != edit.context.view {
            return Ok(());
        }
        *remaining = remaining.checked_sub(1).ok_or(SelectionError::Limit)?;
        if records.len() != before.selections.selections().len() {
            return Err(SelectionError::Faulted);
        }
        let at = edit
            .replacement
            .start
            .checked_add(edit.replacement.inserted.len())
            .ok_or(SelectionError::Faulted)?;
        if !candidate.source.text().is_char_boundary(at) {
            return Err(SelectionError::Faulted);
        }
        let primary = before.selections.primary();
        let original = before
            .selections
            .selections()
            .get(primary)
            .ok_or(SelectionError::Faulted)?;
        let target = records.get_mut(primary).ok_or(SelectionError::Faulted)?;
        *target = Selection {
            anchor: BytePosition(at),
            head: BytePosition(at),
            affinity: original.affinity,
        };
        Ok(())
    }
}
