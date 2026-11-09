//! Logical command consumers over retained, host-attested exact-source tables.
//! No segmentation implementation, UI binding, dictionary or provider invocation.
use super::*;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnicodeUnitV1 {
    Grapheme,
    WordStart,
    WordEnd,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnicodeDirectionV1 {
    Forward,
    Backward,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnicodeCommandV1 {
    pub unit: UnicodeUnitV1,
    pub direction: UnicodeDirectionV1,
    pub count: usize,
    pub extend: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnicodeCommandBudget {
    /// Requested count times original selection count, even at terminal edges.
    pub max_steps: usize,
    /// Native capture/selection or replacement headers and derived payloads.
    /// Existing source/companion bounds remain separate; not allocator peak.
    pub max_derived_bytes: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnicodeCommandError {
    InconsistentProfile,
    InvalidRequest,
    InvalidBoundary,
    Limit,
    Capacity,
    Faulted,
    Boundary(BoundaryError),
    Selection(SelectionError),
    Coordinator(CoordinatorError),
}
impl UnicodeCommandError {
    pub(in crate::coordinator) fn is_faulted(&self) -> bool {
        faulted(self)
    }
}
pub(in crate::coordinator) struct UnicodeEdit {
    context: LogicalViewContext,
    evidence: BoundaryEvidenceId,
    request: UnicodeCommandV1,
    budget: UnicodeCommandBudget,
}
impl std::fmt::Debug for UnicodeEdit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("UnicodeEdit(<redacted>)")
    }
}
/// Owned native preparation, not an invocation ticket or recipient grant.
pub struct PreparedUnicodeMovement {
    capture: UnicodeEdit,
    before: SelectionSet,
    after: SelectionSet,
}
impl PreparedUnicodeMovement {
    pub(in crate::coordinator) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.capture.context.source.document == *document
    }
    pub(in crate::coordinator) fn matches(
        &self,
        context: &LogicalViewContext,
        evidence: &BoundaryEvidenceId,
        request: UnicodeCommandV1,
        budget: Option<UnicodeCommandBudget>,
    ) -> bool {
        self.capture.context == *context
            && self.capture.evidence == *evidence
            && self.capture.request == request
            && Some(self.capture.budget) == budget
    }
}
impl std::fmt::Debug for PreparedUnicodeMovement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreparedUnicodeMovement(<redacted>)")
    }
}
fn faulted(error: &UnicodeCommandError) -> bool {
    matches!(
        error,
        UnicodeCommandError::Faulted
            | UnicodeCommandError::Boundary(BoundaryError::Faulted)
            | UnicodeCommandError::Selection(SelectionError::Faulted)
            | UnicodeCommandError::Coordinator(
                CoordinatorError::Faulted | CoordinatorError::History(HistoryError::Faulted)
            )
    )
}
fn observe<T>(r: &mut Recorder, boundary: Boundary, result: &Result<T, UnicodeCommandError>) {
    r.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(e) if faulted(e) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time: None,
    });
}
fn as_selection(error: UnicodeCommandError) -> SelectionError {
    match error {
        UnicodeCommandError::Boundary(BoundaryError::Unavailable) => SelectionError::Unavailable,
        UnicodeCommandError::Boundary(BoundaryError::Stale) => SelectionError::Stale,
        UnicodeCommandError::Boundary(BoundaryError::Limit) | UnicodeCommandError::Limit => {
            SelectionError::Limit
        }
        UnicodeCommandError::Boundary(BoundaryError::Capacity) | UnicodeCommandError::Capacity => {
            SelectionError::Capacity
        }
        UnicodeCommandError::Selection(e) => e,
        UnicodeCommandError::Coordinator(e) => super::error(e),
        _ => SelectionError::Faulted,
    }
}
fn charge(
    total: &mut usize,
    n: usize,
    budget: UnicodeCommandBudget,
) -> Result<(), UnicodeCommandError> {
    *total = total
        .checked_add(n)
        .filter(|n| *n <= budget.max_derived_bytes)
        .ok_or(UnicodeCommandError::Limit)?;
    Ok(())
}
fn check(
    before: &SelectionSet,
    request: UnicodeCommandV1,
    budget: UnicodeCommandBudget,
    deleting: bool,
) -> Result<(), UnicodeCommandError> {
    if request.count == 0 || deleting && request.extend {
        return Err(UnicodeCommandError::InvalidRequest);
    }
    if request
        .count
        .checked_mul(before.selections().len())
        .is_none_or(|n| n > budget.max_steps)
    {
        return Err(UnicodeCommandError::Limit);
    }
    Ok(())
}
fn check_grid(table: &BoundaryTableV1, unit: UnicodeUnitV1) -> Result<(), UnicodeCommandError> {
    for word in &table.significant_words {
        let endpoint = match unit {
            UnicodeUnitV1::Grapheme => return Ok(()),
            UnicodeUnitV1::WordStart => word.start,
            UnicodeUnitV1::WordEnd => word.end,
        };
        if table
            .graphemes
            .binary_search_by_key(&endpoint, |p| p.0)
            .is_err()
        {
            return Err(UnicodeCommandError::InconsistentProfile);
        }
    }
    Ok(())
}
fn advance(table: &BoundaryTableV1, at: usize, request: UnicodeCommandV1, len: usize) -> usize {
    if request.unit == UnicodeUnitV1::Grapheme {
        match request.direction {
            UnicodeDirectionV1::Forward => table
                .graphemes
                .get(table.graphemes.partition_point(|p| p.0 <= at))
                .map_or(len, |p| p.0),
            UnicodeDirectionV1::Backward => table
                .graphemes
                .partition_point(|p| p.0 < at)
                .checked_sub(1)
                .and_then(|n| table.graphemes.get(n))
                .map_or(0, |p| p.0),
        }
    } else {
        let endpoint = |word: &ByteRange| {
            if request.unit == UnicodeUnitV1::WordStart {
                word.start
            } else {
                word.end
            }
        };
        match request.direction {
            UnicodeDirectionV1::Forward => table
                .significant_words
                .get(
                    table
                        .significant_words
                        .partition_point(|w| endpoint(w) <= at),
                )
                .map_or(len, endpoint),
            UnicodeDirectionV1::Backward => table
                .significant_words
                .partition_point(|w| endpoint(w) < at)
                .checked_sub(1)
                .and_then(|n| table.significant_words.get(n))
                .map_or(0, endpoint),
        }
    }
}
fn target(
    table: &BoundaryTableV1,
    selection: Selection,
    request: UnicodeCommandV1,
    len: usize,
    deleting: bool,
) -> Result<usize, UnicodeCommandError> {
    let (lo, hi) = (
        selection.anchor.0.min(selection.head.0),
        selection.anchor.0.max(selection.head.0),
    );
    if deleting && lo != hi {
        return Ok(lo);
    }
    if deleting
        && table
            .graphemes
            .binary_search_by_key(&selection.head.0, |p| p.0)
            .is_err()
    {
        return Err(UnicodeCommandError::InvalidBoundary);
    }
    let mut at = selection.head.0;
    let mut steps = request.count;
    if lo != hi && !request.extend {
        at = if request.direction == UnicodeDirectionV1::Forward {
            hi
        } else {
            lo
        };
        steps -= 1;
    }
    for _ in 0..steps {
        at = advance(table, at, request, len);
    }
    if deleting && table.graphemes.binary_search_by_key(&at, |p| p.0).is_err() {
        return Err(UnicodeCommandError::InvalidBoundary);
    }
    Ok(at)
}
fn movement(
    source: &Utf8Source,
    before: &SelectionSet,
    table: &BoundaryTableV1,
    request: UnicodeCommandV1,
    budget: UnicodeCommandBudget,
    normalization: SelectionBudget,
) -> Result<SelectionSet, UnicodeCommandError> {
    check(before, request, budget, false)?;
    check_grid(table, request.unit)?;
    let mut bytes = std::mem::size_of::<PreparedUnicodeMovement>();
    let payload = before
        .selections()
        .len()
        .checked_mul(std::mem::size_of::<Selection>())
        .and_then(|n| n.checked_mul(2))
        .ok_or(UnicodeCommandError::Limit)?;
    charge(&mut bytes, payload, budget)?;
    let mut records = Vec::new();
    records
        .try_reserve_exact(before.selections().len())
        .map_err(|_| UnicodeCommandError::Capacity)?;
    for selection in before.selections() {
        let at = BytePosition(target(
            table,
            *selection,
            request,
            source.text().len(),
            false,
        )?);
        records.push(Selection {
            anchor: if request.extend { selection.anchor } else { at },
            head: at,
            affinity: selection.affinity,
        });
    }
    normalize_selections(source, &records, before.primary(), normalization)
        .map_err(UnicodeCommandError::Selection)
}
fn deletion(
    source: &Utf8Source,
    before: &SelectionSet,
    table: &BoundaryTableV1,
    request: UnicodeCommandV1,
    budget: UnicodeCommandBudget,
) -> Result<Vec<Replacement>, UnicodeCommandError> {
    check(before, request, budget, true)?;
    check_grid(table, request.unit)?;
    let mut bytes = std::mem::size_of::<UnicodeEdit>();
    charge(
        &mut bytes,
        before
            .selections()
            .len()
            .checked_mul(std::mem::size_of::<Replacement>())
            .ok_or(UnicodeCommandError::Limit)?,
        budget,
    )?;
    let mut edits = Vec::new();
    edits
        .try_reserve_exact(before.selections().len())
        .map_err(|_| UnicodeCommandError::Capacity)?;
    for selection in before.selections() {
        let at = target(table, *selection, request, source.text().len(), true)?;
        let (lo, hi) = (
            selection.anchor.0.min(selection.head.0),
            selection.anchor.0.max(selection.head.0),
        );
        let (start, end) = if lo != hi {
            (lo, hi)
        } else {
            (selection.head.0.min(at), selection.head.0.max(at))
        };
        charge(&mut bytes, end - start, budget)?;
        let mut expected = String::new();
        expected
            .try_reserve_exact(end - start)
            .map_err(|_| UnicodeCommandError::Capacity)?;
        expected.push_str(&source.text()[start..end]);
        edits.push(Replacement {
            start,
            end,
            expected,
            inserted: String::new(),
        });
    }
    Ok(edits)
}
fn capture_slot(prepared: &PreparedTransaction) -> Result<Option<&UnicodeEdit>, SelectionError> {
    match prepared.derived_edit.as_slice() {
        [] => Ok(None),
        [DerivedEdit::Unicode(edit)] => Ok(Some(edit)),
        [_] => Ok(None),
        _ => Err(SelectionError::Faulted),
    }
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn unicode_command_profile(&self) -> Option<UnicodeCommandBudget> {
        self.views.unicode_limits
    }
    pub(in crate::coordinator) fn unicode_deletion_intent_matches(
        prepared: &PreparedTransaction,
        context: &LogicalViewContext,
        evidence: &BoundaryEvidenceId,
        request: UnicodeCommandV1,
        budget: Option<UnicodeCommandBudget>,
    ) -> bool {
        prepared.derived_kind == Some(DerivedKind::Unicode)
            && matches!(capture_slot(prepared),Ok(Some(edit)) if edit.context==*context && edit.evidence==*evidence && edit.request==request && Some(edit.budget)==budget)
    }
    fn finish_unicode<T>(
        &mut self,
        r: &mut Recorder,
        boundary: Boundary,
        result: Result<T, UnicodeCommandError>,
    ) -> Result<T, UnicodeCommandError> {
        if result.as_ref().err().is_some_and(faulted) {
            self.faulted = true;
        }
        observe(r, boundary, &result);
        result
    }
    pub fn configure_unicode_commands(
        &mut self,
        budget: UnicodeCommandBudget,
        r: &mut Recorder,
    ) -> Result<bool, UnicodeCommandError> {
        let result = (|| {
            self.ensure_operational()
                .map_err(UnicodeCommandError::Coordinator)?;
            match self.views.unicode_limits {
                Some(old) if old == budget => Ok(false),
                Some(_) => Err(UnicodeCommandError::Selection(SelectionError::Unavailable)),
                None => {
                    self.views.unicode_limits = Some(budget);
                    Ok(true)
                }
            }
        })();
        self.finish_unicode(r, Boundary::UnicodeConfiguration, result)
    }
    fn unicode_table(
        &self,
        context: &LogicalViewContext,
        evidence: &BoundaryEvidenceId,
    ) -> Result<&BoundaryTableV1, UnicodeCommandError> {
        let data = self
            .resolve_boundary_evidence(evidence)
            .map_err(UnicodeCommandError::Boundary)?;
        if data.version != context.source {
            return Err(UnicodeCommandError::Boundary(BoundaryError::Stale));
        }
        Ok(data.table)
    }
    /// Freeze bounded logical motion over this retained exact-source table.
    /// This consumes host-attested semantics; it does not run a segmenter.
    pub fn prepare_unicode_movement(
        &self,
        context: &LogicalViewContext,
        evidence: &BoundaryEvidenceId,
        request: UnicodeCommandV1,
        r: &mut Recorder,
    ) -> Result<PreparedUnicodeMovement, UnicodeCommandError> {
        let result = (|| {
            self.ensure_operational()
                .map_err(UnicodeCommandError::Coordinator)?;
            let view = self
                .view_context(context)
                .map_err(UnicodeCommandError::Selection)?;
            let budget = self
                .views
                .unicode_limits
                .ok_or(UnicodeCommandError::Selection(SelectionError::Unavailable))?;
            let table = self.unicode_table(context, evidence)?;
            let source = &self
                .validate_version(&context.source)
                .map_err(UnicodeCommandError::Coordinator)?
                .source;
            let normalization = self
                .views
                .limits
                .ok_or(UnicodeCommandError::Faulted)?
                .normalization;
            let after = movement(
                source,
                &view.selections,
                table,
                request,
                budget,
                normalization,
            )?;
            Ok(PreparedUnicodeMovement {
                capture: UnicodeEdit {
                    context: context.clone(),
                    evidence: evidence.clone(),
                    request,
                    budget,
                },
                before: view
                    .selections
                    .copy()
                    .map_err(UnicodeCommandError::Selection)?,
                after,
            })
        })();
        observe(r, Boundary::UnicodePreparation, &result);
        result
    }
    /// Revalidate evidence and the target before the shared selection publisher.
    /// No source-write permission, source revision or history group is needed.
    pub fn commit_unicode_movement(
        &mut self,
        plan: PreparedUnicodeMovement,
        r: &mut Recorder,
    ) -> Result<LogicalViewUpdate, UnicodeCommandError> {
        let result = self
            .ensure_operational()
            .map_err(UnicodeCommandError::Coordinator)
            .and_then(|_| self.publish_unicode_movement(plan, r));
        let changed = result.as_ref().is_ok_and(|update| update.changed);
        let result = self.finish_unicode(r, Boundary::UnicodePublication, result);
        if changed {
            r.record(Observation {
                boundary: Boundary::Publication,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        result
    }
    /// Eligible native publication under the shared invocation fault sentinel.
    pub(in crate::coordinator) fn publish_unicode_movement(
        &mut self,
        plan: PreparedUnicodeMovement,
        r: &mut Recorder,
    ) -> Result<LogicalViewUpdate, UnicodeCommandError> {
        let validation = (|| {
            let capture = &plan.capture;
            let view = self
                .view_context(&capture.context)
                .map_err(UnicodeCommandError::Selection)?;
            if view.selections != plan.before {
                return Err(UnicodeCommandError::Faulted);
            }
            if self.views.unicode_limits != Some(capture.budget) {
                return Err(UnicodeCommandError::Selection(SelectionError::Stale));
            }
            let table = self.unicode_table(&capture.context, &capture.evidence)?;
            let source = &self
                .validate_version(&capture.context.source)
                .map_err(UnicodeCommandError::Coordinator)?
                .source;
            let normalization = self
                .views
                .limits
                .ok_or(UnicodeCommandError::Faulted)?
                .normalization;
            let expected = movement(
                source,
                &plan.before,
                table,
                capture.request,
                capture.budget,
                normalization,
            )
            .map_err(|e| {
                if matches!(
                    e,
                    UnicodeCommandError::Capacity
                        | UnicodeCommandError::Selection(SelectionError::Capacity)
                ) {
                    e
                } else {
                    UnicodeCommandError::Faulted
                }
            })?;
            if expected != plan.after {
                return Err(UnicodeCommandError::Faulted);
            }
            Ok(())
        })();
        observe(r, Boundary::UnicodeValidation, &validation);
        validation.and_then(|_| {
            self.publish_selection_value(&plan.capture.context, plan.after, r)
                .map_err(UnicodeCommandError::Selection)
        })
    }
    /// Exact explicit selections stay exact; implicit deletion requires grapheme
    /// endpoints. Canonical ambiguity and every source companion use normal commit.
    pub fn prepare_unicode_deletion(
        &self,
        context: &LogicalViewContext,
        evidence: &BoundaryEvidenceId,
        request: UnicodeCommandV1,
        r: &mut Recorder,
    ) -> Result<PreparedTransaction, UnicodeCommandError> {
        let result = (|| {
            self.ensure_operational()
                .map_err(UnicodeCommandError::Coordinator)?;
            let view = self
                .view_context(context)
                .map_err(UnicodeCommandError::Selection)?;
            let source = self
                .validate_version(&context.source)
                .map_err(UnicodeCommandError::Coordinator)?;
            if source.read_only {
                return Err(UnicodeCommandError::Coordinator(
                    CoordinatorError::Unavailable,
                ));
            }
            let budget = self
                .views
                .unicode_limits
                .ok_or(UnicodeCommandError::Selection(SelectionError::Unavailable))?;
            let edits = deletion(
                &source.source,
                &view.selections,
                self.unicode_table(context, evidence)?,
                request,
                budget,
            )?;
            let mut prepared = self
                .prepare_inner(
                    &[DocumentEdits {
                        expected: context.source.clone(),
                        edits,
                    }],
                    &[],
                )
                .map_err(UnicodeCommandError::Coordinator)?;
            prepared
                .derived_edit
                .try_reserve_exact(1)
                .map_err(|_| UnicodeCommandError::Capacity)?;
            prepared
                .derived_edit
                .push(DerivedEdit::Unicode(UnicodeEdit {
                    context: context.clone(),
                    evidence: evidence.clone(),
                    request,
                    budget,
                }));
            prepared.derived_kind = Some(DerivedKind::Unicode);
            Ok(prepared)
        })();
        observe(r, Boundary::UnicodePreparation, &result);
        result
    }
    pub(super) fn verify_unicode_edit(
        &self,
        prepared: &PreparedTransaction,
        r: &mut Recorder,
    ) -> Result<(), SelectionError> {
        let Some(edit) = capture_slot(prepared)? else {
            return Ok(());
        };
        let result = (|| {
            if prepared.candidates.len() != 1
                || !prepared.read_dependencies.is_empty()
                || !prepared.annotations.is_empty()
                || !matches!(prepared.history, history::PreparedHistory::Ordinary)
            {
                return Err(UnicodeCommandError::Faulted);
            }
            if self.views.unicode_limits != Some(edit.budget) {
                return Err(UnicodeCommandError::Selection(SelectionError::Stale));
            }
            let before = prepared
                .view_dependencies
                .iter()
                .find(|v| v.id == edit.context.view)
                .ok_or(UnicodeCommandError::Faulted)?;
            let candidate = &prepared.candidates[0];
            if before.source != edit.context.source
                || before.stamp != edit.context.stamp
                || candidate.expected != edit.context.source
            {
                return Err(UnicodeCommandError::Faulted);
            }
            let table = self.unicode_table(&edit.context, &edit.evidence)?;
            let source = &self
                .validate_version(&candidate.expected)
                .map_err(UnicodeCommandError::Coordinator)?
                .source;
            let expected = deletion(source, &before.selections, table, edit.request, edit.budget)
                .map_err(|e| {
                if e == UnicodeCommandError::Capacity {
                    e
                } else {
                    UnicodeCommandError::Faulted
                }
            })?;
            if expected != candidate.edits {
                return Err(UnicodeCommandError::Faulted);
            }
            Ok(())
        })();
        observe(r, Boundary::UnicodeValidation, &result);
        result.map_err(as_selection)
    }
    pub(super) fn unicode_carets_after(
        &self,
        prepared: &PreparedTransaction,
        before: &ViewDependency,
    ) -> Result<bool, SelectionError> {
        Ok(capture_slot(prepared)?.is_some_and(|edit| edit.context.view == before.id))
    }
}
