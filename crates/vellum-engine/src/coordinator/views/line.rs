//! Explicit prefix transformations for prose/code, not grammar or UI policy.
use super::*;
mod derive;
#[cfg(test)]
mod tests;
#[cfg(test)]
pub(in crate::coordinator) use tests::fixture as line_test_fixture;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineEditBudget {
    pub max_lines: usize,
    pub max_prefix_bytes: usize,
    pub max_indent_columns: usize,
    pub max_unit_bytes: usize,
    /// Logical request/Replacement headers and payload bytes, not allocator peak.
    pub max_derived_bytes: usize,
    /// Per-coordinate-scan prefix extent; selection/line counts bound scan count.
    pub max_source_scan_bytes: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneratedNewlineV1 {
    Lf,
    CrLf,
    Cr,
}
impl GeneratedNewlineV1 {
    fn text(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
            Self::Cr => "\r",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewlineIndentV1 {
    None,
    CopyPrefix,
}
#[derive(PartialEq, Eq)]
pub enum LineEditV1 {
    Indent {
        unit: String,
        tab_width: usize,
    },
    Outdent {
        unit: String,
        tab_width: usize,
    },
    Newline {
        delimiter: GeneratedNewlineV1,
        indent: NewlineIndentV1,
    },
}
impl std::fmt::Debug for LineEditV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LineEditV1(<redacted>)")
    }
}
impl LineEditV1 {
    fn unit(&self) -> Option<(&str, usize)> {
        match self {
            Self::Indent { unit, tab_width } | Self::Outdent { unit, tab_width } => {
                Some((unit, *tab_width))
            }
            Self::Newline { .. } => None,
        }
    }
    pub(in crate::coordinator) fn unit_bytes(&self) -> usize {
        self.unit().map_or(0, |(unit, _)| unit.len())
    }
    pub(in crate::coordinator) fn copy(&self) -> Result<Self, LineEditError> {
        Ok(match self {
            Self::Indent { unit, tab_width } => Self::Indent {
                unit: copy(unit)?,
                tab_width: *tab_width,
            },
            Self::Outdent { unit, tab_width } => Self::Outdent {
                unit: copy(unit)?,
                tab_width: *tab_width,
            },
            Self::Newline { delimiter, indent } => Self::Newline {
                delimiter: *delimiter,
                indent: *indent,
            },
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineEditError {
    InvalidSettings,
    Limit,
    Capacity,
    Selection(SelectionError),
    Coordinator(CoordinatorError),
}
fn copy(text: &str) -> Result<String, LineEditError> {
    let mut out = String::new();
    out.try_reserve_exact(text.len())
        .map_err(|_| LineEditError::Capacity)?;
    out.push_str(text);
    Ok(out)
}
pub(in crate::coordinator) struct LineEdit {
    context: LogicalViewContext,
    request: LineEditV1,
    budget: LineEditBudget,
}
impl std::fmt::Debug for LineEdit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LineEdit(<redacted>)")
    }
}
fn capture_slot(prepared: &PreparedTransaction) -> Result<Option<&LineEdit>, SelectionError> {
    match prepared.derived_edit.as_slice() {
        [] => Ok(None),
        [DerivedEdit::Line(edit)] => Ok(Some(edit)),
        [_] => Ok(None),
        _ => Err(SelectionError::Faulted),
    }
}
impl SourceCoordinator {
    #[cfg(test)]
    pub(in crate::coordinator) fn tighten_line_unit_for_test(&mut self) {
        self.views.line_limits.as_mut().unwrap().max_unit_bytes = 0;
    }
    pub(in crate::coordinator) fn configured_line_context<'a>(
        &self,
        prepared: &'a PreparedTransaction,
    ) -> Result<&'a LogicalViewContext, CoordinatorError> {
        capture_slot(prepared)
            .map_err(|_| CoordinatorError::Faulted)?
            .map(|edit| &edit.context)
            .ok_or(CoordinatorError::Faulted)
    }
    pub(in crate::coordinator) fn line_edit_profile(&self) -> Option<LineEditBudget> {
        self.views.line_limits
    }
    pub(in crate::coordinator) fn line_intent_matches(
        prepared: &PreparedTransaction,
        context: &LogicalViewContext,
        request: &LineEditV1,
        budget: Option<LineEditBudget>,
    ) -> bool {
        matches!(capture_slot(prepared), Ok(Some(edit)) if edit.context==*context && edit.request==*request && Some(edit.budget)==budget)
    }
    /// One-time trusted-owner explicit prefix profile. Settings stay per request;
    /// this does not activate configured settings, providers or default keymaps.
    pub fn configure_line_edits(
        &mut self,
        budget: LineEditBudget,
        r: &mut Recorder,
    ) -> Result<bool, LineEditError> {
        let result = (|| {
            self.ensure_operational()
                .map_err(LineEditError::Coordinator)?;
            match self.views.line_limits {
                Some(old) if old == budget => Ok(false),
                Some(_) => Err(LineEditError::Coordinator(CoordinatorError::Unavailable)),
                None => {
                    self.views.line_limits = Some(budget);
                    Ok(true)
                }
            }
        })();
        capture_line(r, Boundary::LineEditConfiguration, &result);
        result
    }
    pub fn prepare_line_edit(
        &self,
        context: &LogicalViewContext,
        request: &LineEditV1,
        r: &mut Recorder,
    ) -> Result<PreparedTransaction, LineEditError> {
        let result = (|| {
            self.ensure_operational()
                .map_err(LineEditError::Coordinator)?;
            let view = self
                .view_context(context)
                .map_err(LineEditError::Selection)?;
            let document = self
                .validate_version(&context.source)
                .map_err(LineEditError::Coordinator)?;
            if document.read_only {
                return Err(LineEditError::Coordinator(CoordinatorError::Unavailable));
            }
            let budget = self
                .views
                .line_limits
                .ok_or(LineEditError::Coordinator(CoordinatorError::Unavailable))?;
            let edits = derive::edits(&document.source, &view.selections, request, budget)?;
            let mut prepared = self
                .prepare_inner(
                    &[DocumentEdits {
                        expected: context.source.clone(),
                        edits,
                    }],
                    &[],
                )
                .map_err(LineEditError::Coordinator)?;
            prepared
                .derived_edit
                .try_reserve_exact(1)
                .map_err(|_| LineEditError::Capacity)?;
            prepared.derived_edit.push(DerivedEdit::Line(LineEdit {
                context: context.clone(),
                request: request.copy()?,
                budget,
            }));
            prepared.derived_kind = Some(DerivedKind::Line);
            Ok(prepared)
        })();
        capture_line(r, Boundary::LineEditPreparation, &result);
        result
    }
    /// Resolve the immutable document birth default, then use the same explicit
    /// newline derivation, current selection, companion and history machinery.
    pub fn prepare_document_newline_v1(
        &self,
        context: &LogicalViewContext,
        indent: NewlineIndentV1,
        r: &mut Recorder,
    ) -> Result<PreparedTransaction, LineEditError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let delimiter = self
                .source_insertion_policy_v1(&context.source, r)
                .map_err(LineEditError::Coordinator)?;
            self.prepare_line_edit(context, &LineEditV1::Newline { delimiter, indent }, r)
        })();
        r.record(Observation {
            boundary: Boundary::DocumentNewlinePreparation,
            disposition: match &result {
                Ok(_) => Disposition::Completed,
                Err(LineEditError::Coordinator(CoordinatorError::Faulted)) => Disposition::Unknown,
                Err(_) => Disposition::Refused,
            },
            service_time: timer.finish(),
        });
        result
    }
    pub(super) fn verify_line_edit(
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
        if self.views.line_limits != Some(edit.budget) {
            return Err(SelectionError::Stale);
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
        {
            return Err(SelectionError::Faulted);
        }
        let source = &self
            .validate_version(&candidate.expected)
            .map_err(error)?
            .source;
        let expected = derive::edits(source, &before.selections, &edit.request, edit.budget)
            .map_err(|e| {
                if e == LineEditError::Capacity {
                    SelectionError::Capacity
                } else {
                    SelectionError::Faulted
                }
            })?;
        if expected != candidate.edits {
            return Err(SelectionError::Faulted);
        }
        Ok(())
    }
    pub(super) fn line_carets_after(
        &self,
        prepared: &PreparedTransaction,
        before: &ViewDependency,
    ) -> Result<bool, SelectionError> {
        Ok(capture_slot(prepared)?.is_some_and(|edit| {
            edit.context.view == before.id && matches!(edit.request, LineEditV1::Newline { .. })
        }))
    }
}
fn capture_line<T>(r: &mut Recorder, boundary: Boundary, result: &Result<T, LineEditError>) {
    r.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(
                LineEditError::Coordinator(CoordinatorError::Faulted)
                | LineEditError::Selection(SelectionError::Faulted),
            ) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time: None,
    });
}
