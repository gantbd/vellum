//! Trusted native logical views. No focus, keymap, geometry, UI text buffer,
//! identity registry, implicit target refresh or selected Unicode provider.
use super::*;
use vellum_engine::{
    coordinates::CoordinateBudget,
    coordinator::{
        CommitOutcome, LineEditBudget, LineEditV1, LogicalEndpointV1, LogicalViewContext,
        LogicalViewId, LogicalViewLimits, LogicalViewSnapshot, LogicalViewStamp, LogicalViewUpdate,
        LogicalViewUsage, SelectionAfterV1,
    },
    selection::Selection,
};
impl<A: ResourceAdapter> LocalHost<A> {
    /// Explicit native profile; reconfiguration obeys the original owner's rules.
    /// This is not a deployment default or permission grant.
    pub fn configure_logical_views(
        &mut self,
        limits: LogicalViewLimits,
    ) -> Result<bool, SessionError> {
        Ok(self
            .owner
            .configure_logical_views(limits, &mut self.diagnostics)?)
    }
    pub fn configure_line_edits(&mut self, budget: LineEditBudget) -> Result<bool, SessionError> {
        Ok(self
            .owner
            .configure_line_edits(budget, &mut self.diagnostics)?)
    }
    pub fn create_logical_view(&mut self) -> Result<LogicalViewId, SessionError> {
        Ok(self.owner.create_logical_view(&mut self.diagnostics)?)
    }
    /// Original native context/selections, not a recipient-authorized disclosure.
    pub fn logical_view(
        &mut self,
        view: &LogicalViewId,
    ) -> Result<LogicalViewSnapshot, SessionError> {
        Ok(self.owner.logical_view(view, &mut self.diagnostics)?)
    }
    /// Caller supplies the original detached stamp and exact source version.
    /// No attach-then-set transaction, path inference or automatic retargeting.
    pub fn attach_logical_view(
        &mut self,
        view: &LogicalViewId,
        expected: LogicalViewStamp,
        source: &DocumentVersion,
        selections: &[Selection],
        primary: usize,
    ) -> Result<LogicalViewUpdate, SessionError> {
        Ok(self.owner.attach_logical_view(
            view,
            expected,
            source,
            selections,
            primary,
            &mut self.diagnostics,
        )?)
    }
    pub fn detach_logical_view(
        &mut self,
        view: &LogicalViewId,
        expected: LogicalViewStamp,
    ) -> Result<LogicalViewUpdate, SessionError> {
        Ok(self
            .owner
            .detach_logical_view(view, expected, &mut self.diagnostics)?)
    }
    /// Detached original only; retirement neither disposes its source nor resets
    /// lifetime issuance. Stale/foreign/retained state remains a native refusal.
    pub fn retire_logical_view(
        &mut self,
        view: &LogicalViewId,
        expected: LogicalViewStamp,
    ) -> Result<(), SessionError> {
        Ok(self
            .owner
            .retire_logical_view(view, expected, &mut self.diagnostics)?)
    }
    pub fn logical_view_usage(&self) -> LogicalViewUsage {
        self.owner.logical_view_usage()
    }
    pub fn set_logical_selections(
        &mut self,
        context: &LogicalViewContext,
        selections: &[Selection],
        primary: usize,
    ) -> Result<LogicalViewUpdate, SessionError> {
        Ok(self.owner.set_logical_selections(
            context,
            selections,
            primary,
            &mut self.diagnostics,
        )?)
    }
    pub fn move_logical_selections(
        &mut self,
        context: &LogicalViewContext,
        endpoint: LogicalEndpointV1,
        extend: bool,
        budget: CoordinateBudget,
    ) -> Result<LogicalViewUpdate, SessionError> {
        Ok(self.owner.move_logical_selections(
            context,
            endpoint,
            extend,
            budget,
            &mut self.diagnostics,
        )?)
    }
    /// One existing native publication: selected source edits, initiating/peer
    /// selections, annotations and history advance together. The initiating
    /// selection-after policy is explicit; other views retain native mapping.
    pub fn replace_logical_selections(
        &mut self,
        context: &LogicalViewContext,
        literal: &str,
        after: SelectionAfterV1,
    ) -> Result<CommitOutcome, SessionError> {
        let prepared =
            self.owner
                .prepare_literal_edit(context, literal, after, &mut self.diagnostics)?;
        Ok(self.owner.commit(prepared, &mut self.diagnostics)?)
    }
    /// Explicit bounded prose/code prefix/newline semantics, not grammar policy
    /// or a second widget indentation algorithm. Untouched delimiters survive.
    pub fn edit_logical_lines(
        &mut self,
        context: &LogicalViewContext,
        request: &LineEditV1,
    ) -> Result<CommitOutcome, SessionError> {
        let prepared = self
            .owner
            .prepare_line_edit(context, request, &mut self.diagnostics)?;
        Ok(self.owner.commit(prepared, &mut self.diagnostics)?)
    }
}
