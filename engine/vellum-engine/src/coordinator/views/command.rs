//! Closed headless selection requests and privately frozen native preparations.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionDirectionV1 {
    Forward,
    Backward,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchSelectionModeV1 {
    /// Replace the view's entire selection set with this one occurrence.
    /// This does not select or replace every search match.
    ReplaceAll,
    /// Replace the original primary, then normalize with that input primary.
    ReplacePrimary,
    /// Append this occurrence as primary, then normalize the selection set.
    AddPrimary,
}

/// Original raw list order/primary and endpoint interpretation budgets are intent,
/// even when different requests would normalize to the same resulting selection.
#[derive(Debug, PartialEq, Eq)]
pub enum SelectionCommandV1 {
    /// Resolve the owned original occurrence; never accept a range or rerun search.
    SearchMatch {
        occurrence: SearchMatch,
        direction: SelectionDirectionV1,
        affinity: Affinity,
        mode: SearchSelectionModeV1,
    },
    Set {
        selections: Vec<Selection>,
        primary: usize,
    },
    Endpoint {
        endpoint: LogicalEndpointV1,
        extend: bool,
        budget: CoordinateBudget,
    },
}
impl SelectionCommandV1 {
    pub(in crate::coordinator) fn retained_bytes(&self) -> Option<usize> {
        std::mem::size_of::<Self>().checked_add(match self {
            Self::SearchMatch { occurrence, .. } => occurrence.retained_bytes()?,
            Self::Set { selections, .. } => selections
                .len()
                .checked_mul(std::mem::size_of::<Selection>())?,
            Self::Endpoint { .. } => 0,
        })
    }
    pub(in crate::coordinator) fn copy(&self) -> Result<Self, SelectionError> {
        Ok(match self {
            Self::SearchMatch {
                occurrence,
                direction,
                affinity,
                mode,
            } => Self::SearchMatch {
                occurrence: occurrence.clone(),
                direction: *direction,
                affinity: *affinity,
                mode: *mode,
            },
            Self::Set {
                selections,
                primary,
            } => {
                let mut owned = Vec::new();
                owned
                    .try_reserve_exact(selections.len())
                    .map_err(|_| SelectionError::Capacity)?;
                owned.extend_from_slice(selections);
                Self::Set {
                    selections: owned,
                    primary: *primary,
                }
            }
            Self::Endpoint {
                endpoint,
                extend,
                budget,
            } => Self::Endpoint {
                endpoint: *endpoint,
                extend: *extend,
                budget: *budget,
            },
        })
    }
}
#[derive(Debug)]
pub(in crate::coordinator) struct PreparedSelectionCommand {
    context: LogicalViewContext,
    request: SelectionCommandV1,
    before: SelectionSet,
    after: SelectionSet,
}
impl PreparedSelectionCommand {
    pub(in crate::coordinator) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.context.source.document == *document
    }
    #[cfg(test)]
    pub(in crate::coordinator) fn corrupt_result_for_test(&mut self) {
        self.after = self.before.copy().unwrap();
    }
    pub(in crate::coordinator) fn matches(
        &self,
        context: &LogicalViewContext,
        request: &SelectionCommandV1,
    ) -> bool {
        self.context == *context && self.request == *request
    }
}
pub(super) fn derive_endpoint(
    source: &Utf8Source,
    before: &SelectionSet,
    endpoint: LogicalEndpointV1,
    extend: bool,
    budget: CoordinateBudget,
) -> Result<Vec<Selection>, SelectionError> {
    let coordinates = SourceCoordinates::new(source);
    let mut selections = Vec::new();
    selections
        .try_reserve_exact(before.selections().len())
        .map_err(|_| SelectionError::Capacity)?;
    for selection in before.selections() {
        let target = match endpoint {
            LogicalEndpointV1::DocumentStart => BytePosition(0),
            LogicalEndpointV1::DocumentEnd => BytePosition(source.text().len()),
            LogicalEndpointV1::LineStart | LogicalEndpointV1::LineEnd => {
                let line = coordinates
                    .byte_to_line(selection.head, budget)
                    .map_err(SelectionError::Coordinate)?;
                if endpoint == LogicalEndpointV1::LineStart {
                    BytePosition(selection.head.0 - line.body_byte_column)
                } else {
                    coordinates
                        .line_span(line.line, budget)
                        .map_err(SelectionError::Coordinate)?
                        .body_end
                }
            }
        };
        selections.push(Selection {
            anchor: if extend { selection.anchor } else { target },
            head: target,
            affinity: selection.affinity,
        });
    }
    Ok(selections)
}
fn derive(
    source: &Utf8Source,
    before: &SelectionSet,
    request: &SelectionCommandV1,
    budget: SelectionBudget,
    matched: Option<ByteRange>,
) -> Result<SelectionSet, SelectionError> {
    match request {
        SelectionCommandV1::SearchMatch {
            direction,
            affinity,
            mode,
            ..
        } => {
            let range = matched.ok_or(SelectionError::Faulted)?;
            let (anchor, head) = match direction {
                SelectionDirectionV1::Forward => (range.start, range.end),
                SelectionDirectionV1::Backward => (range.end, range.start),
            };
            let selected = Selection {
                anchor: BytePosition(anchor),
                head: BytePosition(head),
                affinity: *affinity,
            };
            let count = match mode {
                SearchSelectionModeV1::ReplaceAll => 1,
                SearchSelectionModeV1::ReplacePrimary => before.selections().len(),
                SearchSelectionModeV1::AddPrimary => before
                    .selections()
                    .len()
                    .checked_add(1)
                    .ok_or(SelectionError::Limit)?,
            };
            if count > budget.max_input_selections {
                return Err(SelectionError::Limit);
            }
            let mut records = Vec::new();
            records
                .try_reserve_exact(count)
                .map_err(|_| SelectionError::Capacity)?;
            let primary = match mode {
                SearchSelectionModeV1::ReplaceAll => {
                    records.push(selected);
                    0
                }
                SearchSelectionModeV1::ReplacePrimary => {
                    records.extend_from_slice(before.selections());
                    records[before.primary()] = selected;
                    before.primary()
                }
                SearchSelectionModeV1::AddPrimary => {
                    records.extend_from_slice(before.selections());
                    records.push(selected);
                    count - 1
                }
            };
            normalize_selections(source, &records, primary, budget)
        }
        SelectionCommandV1::Set {
            selections,
            primary,
        } => normalize_selections(source, selections, *primary, budget),
        SelectionCommandV1::Endpoint {
            endpoint,
            extend,
            budget: scan,
        } => {
            let selections = derive_endpoint(source, before, *endpoint, *extend, *scan)?;
            normalize_selections(source, &selections, before.primary(), budget)
        }
    }
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn prepare_managed_search_selection(
        &self,
        context: &LogicalViewContext,
        range: ByteRange,
        direction: SelectionDirectionV1,
        affinity: Affinity,
        mode: SearchSelectionModeV1,
        r: &mut Recorder,
    ) -> Result<(SelectionCommandV1, PreparedSelectionCommand), SelectionError> {
        let view = self.view_context(context)?;
        let (anchor, head) = match direction {
            SelectionDirectionV1::Forward => (range.start, range.end),
            SelectionDirectionV1::Backward => (range.end, range.start),
        };
        let selected = Selection {
            anchor: BytePosition(anchor),
            head: BytePosition(head),
            affinity,
        };
        let count = match mode {
            SearchSelectionModeV1::ReplaceAll => 1,
            SearchSelectionModeV1::ReplacePrimary => view.selections.selections().len(),
            SearchSelectionModeV1::AddPrimary => view
                .selections
                .selections()
                .len()
                .checked_add(1)
                .ok_or(SelectionError::Limit)?,
        };
        if count
            > self
                .views
                .limits
                .ok_or(SelectionError::Unavailable)?
                .normalization
                .max_input_selections
        {
            return Err(SelectionError::Limit);
        }
        let mut selections = Vec::new();
        selections
            .try_reserve_exact(count)
            .map_err(|_| SelectionError::Capacity)?;
        let primary = match mode {
            SearchSelectionModeV1::ReplaceAll => {
                selections.push(selected);
                0
            }
            SearchSelectionModeV1::ReplacePrimary => {
                selections.extend_from_slice(view.selections.selections());
                selections[view.selections.primary()] = selected;
                view.selections.primary()
            }
            SearchSelectionModeV1::AddPrimary => {
                selections.extend_from_slice(view.selections.selections());
                selections.push(selected);
                count - 1
            }
        };
        let command = SelectionCommandV1::Set {
            selections,
            primary,
        };
        let prepared = self.prepare_selection_command(context, &command, r)?;
        Ok((command, prepared))
    }
    fn selection_occurrence(
        &self,
        context: &LogicalViewContext,
        request: &SelectionCommandV1,
        r: &mut Recorder,
    ) -> Result<Option<ByteRange>, SelectionError> {
        let SelectionCommandV1::SearchMatch { occurrence, .. } = request else {
            return Ok(None);
        };
        let result = self
            .resolve_search_occurrence(occurrence, &context.source)
            .map(Some)
            .map_err(|e| match e {
                SearchError::Stale | SearchError::Superseded => SelectionError::Stale,
                SearchError::Capacity => SelectionError::Capacity,
                SearchError::Limit => SelectionError::Limit,
                SearchError::Faulted => SelectionError::Faulted,
                _ => SelectionError::Unavailable,
            });
        capture(r, Boundary::SearchOccurrenceResolution, &result);
        result
    }
    pub(in crate::coordinator) fn prepare_selection_command(
        &self,
        context: &LogicalViewContext,
        request: &SelectionCommandV1,
        r: &mut Recorder,
    ) -> Result<PreparedSelectionCommand, SelectionError> {
        let result = (|| {
            self.ensure_operational().map_err(error)?;
            let view = self.view_context(context)?;
            let source = &self
                .validate_version(&context.source)
                .map_err(error)?
                .source;
            let budget = self
                .views
                .limits
                .ok_or(SelectionError::Unavailable)?
                .normalization;
            let matched = self.selection_occurrence(context, request, r)?;
            let after = derive(source, &view.selections, request, budget, matched)?;
            Ok(PreparedSelectionCommand {
                context: context.clone(),
                request: request.copy()?,
                before: view.selections.copy()?,
                after,
            })
        })();
        capture(r, Boundary::LogicalSelectionPreparation, &result);
        result
    }
    /// Internal publication under the invocation runner's fault sentinel. All
    /// fallible validation/allocation precedes the two infallible root assignments.
    pub(in crate::coordinator) fn publish_selection_command(
        &mut self,
        prepared: PreparedSelectionCommand,
        r: &mut Recorder,
    ) -> Result<LogicalViewUpdate, SelectionError> {
        let view = self.view_context(&prepared.context)?;
        if view.selections != prepared.before {
            return Err(SelectionError::Faulted);
        }
        let source = &self
            .validate_version(&prepared.context.source)
            .map_err(error)?
            .source;
        let budget = self
            .views
            .limits
            .ok_or(SelectionError::Faulted)?
            .normalization;
        let matched = self.selection_occurrence(&prepared.context, &prepared.request, r)?;
        let expected = derive(source, &prepared.before, &prepared.request, budget, matched)
            .map_err(|e| {
                if e == SelectionError::Capacity {
                    e
                } else {
                    SelectionError::Faulted
                }
            })?;
        if expected != prepared.after {
            return Err(SelectionError::Faulted);
        }
        self.publish_selection_value(&prepared.context, prepared.after, r)
    }
}
