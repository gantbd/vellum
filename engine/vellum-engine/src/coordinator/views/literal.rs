//! Literal selection-derived source intents, not UI input or an interpreter.
use super::*;
#[cfg(test)]
mod tests;
/// Explicit initiating-view selection policy for literal replacement. Other
/// attached views always use the ordinary current-position mapping policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionAfterV1 {
    /// Map the original selection endpoints through the replacement batch.
    Mapped,
    /// Collapse each original selection to the end of its inserted literal,
    /// preserving affinity and primary identity through normalization.
    CaretsAfter,
}
pub(in crate::coordinator) struct SelectionEdit {
    context: LogicalViewContext,
    after: SelectionAfterV1,
    text: String,
}
impl std::fmt::Debug for SelectionEdit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SelectionEdit(<redacted>)")
    }
}
fn owned(text: &str) -> Result<String, SelectionError> {
    let mut value = String::new();
    value
        .try_reserve_exact(text.len())
        .map_err(|_| SelectionError::Capacity)?;
    value.push_str(text);
    Ok(value)
}
fn capture_slot(prepared: &PreparedTransaction) -> Result<Option<&SelectionEdit>, SelectionError> {
    match prepared.derived_edit.as_slice() {
        [] => Ok(None),
        [DerivedEdit::Literal(capture)] => Ok(Some(capture)),
        [_] => Ok(None),
        _ => Err(SelectionError::Faulted),
    }
}
impl SourceCoordinator {
    #[cfg(test)]
    pub(in crate::coordinator) fn corrupt_literal_plan_for_test(
        prepared: &mut PreparedTransaction,
        remove: bool,
    ) {
        if remove {
            prepared.derived_edit.clear();
        } else {
            let DerivedEdit::Literal(capture) = prepared.derived_edit.first_mut().unwrap() else {
                panic!("not literal")
            };
            capture.after = SelectionAfterV1::Mapped;
        }
    }
    /// Compare invocation-owned semantic intent without reacquiring current state.
    pub(in crate::coordinator) fn literal_intent_matches(
        prepared: &PreparedTransaction,
        context: &LogicalViewContext,
        text: &str,
        after: SelectionAfterV1,
    ) -> bool {
        matches!(capture_slot(prepared), Ok(Some(edit))
            if edit.context == *context && edit.text == text && edit.after == after)
    }
    /// Freeze exact selection-derived replacements under the ordinary source
    /// budgets. Commit uses the existing complete source/annotation/view/history
    /// path, not an edit-then-move transaction or a public raw-root replacement.
    pub fn prepare_literal_edit(
        &self,
        context: &LogicalViewContext,
        text: &str,
        after: SelectionAfterV1,
        r: &mut Recorder,
    ) -> Result<PreparedTransaction, SelectionError> {
        let result = (|| {
            self.ensure_operational().map_err(error)?;
            let view = self.view_context(context)?;
            let document = self.validate_version(&context.source).map_err(error)?;
            if document.read_only {
                return Err(SelectionError::Unavailable);
            }
            let selections = view.selections.selections();
            if selections.len() > self.limits.source.max_edits {
                return Err(SelectionError::Source(SourceError::EditLimit));
            }
            let payload = selections
                .iter()
                .try_fold(0usize, |total, selection| {
                    total
                        .checked_add(selection.anchor.0.abs_diff(selection.head.0))
                        .and_then(|n| n.checked_add(text.len()))
                })
                .ok_or(SelectionError::Source(SourceError::PayloadLimit))?;
            if payload > self.limits.source.max_edit_payload_bytes {
                return Err(SelectionError::Source(SourceError::PayloadLimit));
            }
            let mut edits = Vec::new();
            edits
                .try_reserve_exact(selections.len())
                .map_err(|_| SelectionError::Capacity)?;
            for selection in selections {
                let start = selection.anchor.0.min(selection.head.0);
                let end = selection.anchor.0.max(selection.head.0);
                edits.push(Replacement {
                    start,
                    end,
                    expected: owned(&document.source.text()[start..end])?,
                    inserted: owned(text)?,
                });
            }
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
                .push(DerivedEdit::Literal(SelectionEdit {
                    context: context.clone(),
                    after,
                    text: owned(text)?,
                }));
            prepared.derived_kind = Some(DerivedKind::Literal);
            Ok(prepared)
        })();
        capture(r, Boundary::LiteralEditPreparation, &result);
        result
    }
    pub(super) fn verify_literal_edit(
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
            || candidate.edits.len() != before.selections.selections().len()
        {
            return Err(SelectionError::Faulted);
        }
        let source = &self
            .validate_version(&candidate.expected)
            .map_err(error)?
            .source;
        for (selection, replacement) in before.selections.selections().iter().zip(&candidate.edits)
        {
            let start = selection.anchor.0.min(selection.head.0);
            let end = selection.anchor.0.max(selection.head.0);
            if replacement.start != start
                || replacement.end != end
                || replacement.inserted != edit.text
                || source.text().get(start..end) != Some(replacement.expected.as_str())
            {
                return Err(SelectionError::Faulted);
            }
        }
        Ok(())
    }
    pub(super) fn literal_carets_after(
        &self,
        prepared: &PreparedTransaction,
        before: &ViewDependency,
    ) -> Result<bool, SelectionError> {
        Ok(capture_slot(prepared)?.is_some_and(|edit| {
            edit.after == SelectionAfterV1::CaretsAfter && before.id == edit.context.view
        }))
    }
}
