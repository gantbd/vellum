//! UI hints project the original native view; no replacement selection owner.
use super::*;
use vellum_engine::{
    attachment::Affinity,
    coordinates::BytePosition,
    coordinator::{CommitOutcome, LogicalViewSnapshot, LogicalViewUsage, SelectionAfterV1},
    selection::{Selection, SelectionError},
};
impl Editor {
    /// Original native line intent. Preparation/publication remain in the
    /// engine; source, views and history advance together. No UI prefix scan.
    pub fn edit_lines(
        &mut self,
        expected: &DocumentVersion,
        request: &vellum_engine::coordinator::LineEditV1,
    ) -> Result<(), EditorError> {
        self.live()?;
        let snapshot = self
            .native_selection()?
            .ok_or(SessionError::Selection(SelectionError::Unavailable))?;
        let context = snapshot
            .context()
            .ok_or(SessionError::Selection(SelectionError::Unavailable))?;
        if &context.source != expected {
            return Err(SessionError::Selection(SelectionError::Stale).into());
        }
        self.groups
            .try_reserve(1)
            .map_err(|_| EditorError::Capacity)?;
        let result = self.host.edit_logical_lines(&context, request);
        self.collect_diagnostics();
        let outcome = result?;
        if let Some(group) = outcome.history_group {
            self.groups.push(group);
        }
        self.clamp_cursor()?;
        self.status = "Native line edit published".into();
        Ok(())
    }
    pub fn native_views_enabled(&self) -> bool {
        self.native_views
    }
    pub(super) fn attach_tab_view(&mut self, index: usize) -> Result<(), EditorError> {
        if !self.native_views {
            return Ok(());
        }
        if self.tabs[index].view.is_none() {
            // Keep the original handle before the next fallible lifecycle step.
            self.tabs[index].view = Some(self.host.create_logical_view()?);
        }
        let view = self.tabs[index]
            .view
            .as_ref()
            .expect("tracked original view");
        let current = self.host.logical_view(view)?;
        if current.source.is_none() {
            let source = self.host.read(&self.tabs[index].document)?.version;
            let head = self.tabs[index].cursor;
            self.host.attach_logical_view(
                view,
                current.stamp,
                &source,
                &[Selection {
                    anchor: BytePosition(head),
                    head: BytePosition(head),
                    affinity: Affinity::After,
                }],
                0,
            )?;
        }
        Ok(())
    }
    /// Trusted application disclosure of the actual native owner, not a grant,
    /// UI geometry or qualification of a Unicode movement provider.
    pub fn native_selection(&mut self) -> Result<Option<LogicalViewSnapshot>, EditorError> {
        if !self.native_views {
            return Ok(None);
        }
        let view = self.tabs[self.active]
            .view
            .as_ref()
            .ok_or(SessionError::Selection(SelectionError::Unavailable))?;
        Ok(Some(self.host.logical_view(view)?))
    }
    /// Native logical gauges, not physical memory or diagnostic event counts.
    pub fn native_view_usage(&self) -> LogicalViewUsage {
        self.host.logical_view_usage()
    }
    pub(super) fn sync_native_selection(&mut self) -> Result<(), EditorError> {
        let Some(view) = self.native_selection()? else {
            return Ok(());
        };
        if view.source.is_none() {
            return Err(SessionError::Selection(SelectionError::Unavailable).into());
        }
        let primary = view
            .selections
            .selections()
            .get(view.selections.primary())
            .ok_or(SessionError::Selection(SelectionError::Unavailable))?;
        self.tabs[self.active].cursor = primary.head.0;
        self.tabs[self.active].anchor =
            (primary.anchor != primary.head).then_some(primary.anchor.0);
        self.tabs[self.active].selection_count = view.selections.selections().len();
        Ok(())
    }
    /// A visual gesture resolves to this original source target. The native
    /// normalizer validates it; stale input never silently picks a new source.
    pub fn set_selection(
        &mut self,
        expected: &DocumentVersion,
        anchor: usize,
        head: usize,
    ) -> Result<(), EditorError> {
        self.live()?;
        let read = self.host.read(&self.tabs[self.active].document)?;
        if &read.version != expected {
            return Err(SessionError::Selection(SelectionError::Stale).into());
        }
        let text = read.source.text();
        if anchor > text.len()
            || head > text.len()
            || !text.is_char_boundary(anchor)
            || !text.is_char_boundary(head)
        {
            return Err(EditorError::Position);
        }
        if let Some(snapshot) = self.native_selection()? {
            let context = snapshot
                .context()
                .ok_or(SessionError::Selection(SelectionError::Unavailable))?;
            if &context.source != expected {
                return Err(SessionError::Selection(SelectionError::Stale).into());
            }
            self.host.set_logical_selections(
                &context,
                &[Selection {
                    anchor: BytePosition(anchor),
                    head: BytePosition(head),
                    affinity: Affinity::After,
                }],
                0,
            )?;
        }
        self.tabs[self.active].cursor = head;
        self.tabs[self.active].anchor = (anchor != head).then_some(anchor);
        self.tabs[self.active].selection_count = 1;
        Ok(())
    }
    pub(super) fn publish_ui_selection(&mut self) -> Result<(), EditorError> {
        if !self.native_views {
            return Ok(());
        }
        let version = self.version()?;
        let tab = &self.tabs[self.active];
        self.set_selection(&version, tab.anchor.unwrap_or(tab.cursor), tab.cursor)
    }
    pub(super) fn publish_native_edit(
        &mut self,
        expected: &DocumentVersion,
        edits: Vec<Replacement>,
    ) -> Result<CommitOutcome, EditorError> {
        if let Some(snapshot) = self.native_selection()? {
            let context = snapshot
                .context()
                .ok_or(SessionError::Selection(SelectionError::Unavailable))?;
            if &context.source != expected {
                return Err(SessionError::Selection(SelectionError::Stale).into());
            }
            if let ([selected], [edit]) = (snapshot.selections.selections(), edits.as_slice()) {
                let start = selected.anchor.0.min(selected.head.0);
                let end = selected.anchor.0.max(selected.head.0);
                // Preserve the explicit caller's fragment precondition. The
                // derived route may not erase it by reading newer text.
                if (start, end) == (edit.start, edit.end)
                    && self
                        .host
                        .read(&self.tabs[self.active].document)?
                        .source
                        .text()
                        .get(start..end)
                        == Some(edit.expected.as_str())
                {
                    return Ok(self.host.replace_logical_selections(
                        &context,
                        &edit.inserted,
                        SelectionAfterV1::CaretsAfter,
                    )?);
                }
            }
        }
        // Explicit derived grapheme/range batches retain original byte targets;
        // the same publisher maps every attached view with source/history.
        Ok(self.host.edit(expected, edits)?)
    }
    pub(super) fn release_native_views(&mut self) -> Result<(), EditorError> {
        for tab in &mut self.tabs {
            if let Some(view) = &tab.view {
                let current = self.host.logical_view(view)?;
                let stamp = if current.source.is_some() {
                    self.host.detach_logical_view(view, current.stamp)?.stamp
                } else {
                    current.stamp
                };
                self.host.retire_logical_view(view, stamp)?;
                tab.view = None;
            }
        }
        Ok(())
    }
}
