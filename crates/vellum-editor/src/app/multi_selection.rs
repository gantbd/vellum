//! Presentation grapheme/cell movement over original native selections. Native
//! normalization/publication remains authoritative; this is not Unicode18 policy.
use super::*;
use vellum_engine::{
    attachment::Affinity,
    coordinates::BytePosition,
    coordinator::SelectionAfterV1,
    selection::{Selection, SelectionError, normalize_selections},
};

fn moved(
    text: &str,
    value: Selection,
    movement: Movement,
    extend: bool,
    grid: Option<&[BytePosition]>,
) -> Selection {
    let cursor = value.head.0;
    let lo = value.anchor.0.min(cursor);
    let hi = value.anchor.0.max(cursor);
    let head = match movement {
        Movement::Left if !extend && lo != hi => lo,
        Movement::Right if !extend && lo != hi => hi,
        Movement::Left => match grid {
            Some(grid) => grid[grid.partition_point(|p| p.0 < cursor).saturating_sub(1)].0,
            None => text[..cursor]
                .grapheme_indices(true)
                .next_back()
                .map_or(0, |(i, _)| i),
        },
        Movement::Right => unicode::clusters(text, cursor, text.len(), grid)
            .next()
            .map_or(cursor, |(i, g)| i + g.len()),
        Movement::Home | Movement::End | Movement::Up | Movement::Down => {
            let lines = line_ranges(text);
            let row = lines.iter().rposition(|l| l.0 <= cursor).unwrap_or(0);
            let (start, end) = lines[row];
            match movement {
                Movement::Home => start,
                Movement::End => end,
                _ => {
                    let column = visual_width(text, start, cursor.min(end), grid);
                    let next = if matches!(movement, Movement::Up) {
                        row.saturating_sub(1)
                    } else {
                        (row + 1).min(lines.len() - 1)
                    };
                    let (a, b) = lines[next];
                    let (mut col, mut at) = (0, a);
                    for (offset, g) in unicode::clusters(text, a, b, grid) {
                        let width = cell_width(g, col);
                        if col + width > column {
                            break;
                        }
                        col += width;
                        at = offset + g.len();
                    }
                    at
                }
            }
        }
    };
    Selection {
        anchor: if extend {
            value.anchor
        } else {
            BytePosition(head)
        },
        head: BytePosition(head),
        affinity: value.affinity,
    }
}
impl Editor {
    pub fn selection_count(&self) -> usize {
        self.tabs[self.active].selection_count
    }
    pub(super) fn move_selections(
        &mut self,
        movement: Movement,
        extend: bool,
    ) -> Result<(), EditorError> {
        #[cfg(feature = "native-unicode")]
        if self.native_unicode.is_some() && matches!(movement, Movement::Left | Movement::Right) {
            return self.unicode_move(matches!(movement, Movement::Left), extend);
        }
        let grid = self.unicode_grid()?;
        let grid = grid.as_deref().map(Vec::as_slice);
        if let Some(snapshot) = self.native_selection()? {
            let context = snapshot
                .context()
                .ok_or(SessionError::Selection(SelectionError::Unavailable))?;
            let read = self.host.read(&self.tabs[self.active].document)?;
            let mut values = Vec::new();
            values
                .try_reserve_exact(snapshot.selections.selections().len())
                .map_err(|_| EditorError::Capacity)?;
            for value in snapshot.selections.selections() {
                values.push(moved(read.source.text(), *value, movement, extend, grid));
            }
            self.host
                .set_logical_selections(&context, &values, snapshot.selections.primary())?;
            self.sync_native_selection()
        } else {
            let cursor = self.cursor();
            let anchor = self.tabs[self.active].anchor.unwrap_or(cursor);
            let next = moved(
                self.text()?,
                Selection {
                    anchor: BytePosition(anchor),
                    head: BytePosition(cursor),
                    affinity: Affinity::After,
                },
                movement,
                extend,
                grid,
            );
            self.tabs[self.active].cursor = next.head.0;
            self.tabs[self.active].anchor = (next.anchor != next.head).then_some(next.anchor.0);
            Ok(())
        }
    }
    /// One original native selection-derived transaction, including every range
    /// or caret. Never turn a multi-selection request into a primary-only edit.
    pub fn replace_selections(
        &mut self,
        expected: &DocumentVersion,
        text: &str,
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
        let result =
            self.host
                .replace_logical_selections(&context, text, SelectionAfterV1::CaretsAfter);
        self.collect_diagnostics();
        let outcome = result?;
        if let Some(group) = outcome.history_group {
            self.groups.push(group);
        }
        self.clamp_cursor()?;
        self.status = "Edited native selection set".into();
        Ok(())
    }
    /// UI-local grapheme deletion lowers all original ranges before one native
    /// publication. Pure native positional normalization merges overlapping UI
    /// targets; no view mutation or source edit occurs while deriving them.
    pub fn delete_selections(
        &mut self,
        expected: &DocumentVersion,
        backward: bool,
    ) -> Result<(), EditorError> {
        self.live()?;
        #[cfg(feature = "native-unicode")]
        if self.native_unicode.is_some() {
            return self.unicode_delete(Some(expected), backward);
        }
        let snapshot = self
            .native_selection()?
            .ok_or(SessionError::Selection(SelectionError::Unavailable))?;
        let context = snapshot
            .context()
            .ok_or(SessionError::Selection(SelectionError::Unavailable))?;
        if &context.source != expected {
            return Err(SessionError::Selection(SelectionError::Stale).into());
        }
        let budget = self
            .selection_budget
            .ok_or(SessionError::Selection(SelectionError::Unavailable))?;
        let read = self.host.read(&self.tabs[self.active].document)?;
        let source = read.source.text();
        let mut targets = Vec::new();
        targets
            .try_reserve_exact(snapshot.selections.selections().len())
            .map_err(|_| EditorError::Capacity)?;
        for selected in snapshot.selections.selections() {
            let mut lo = selected.anchor.0.min(selected.head.0);
            let mut hi = selected.anchor.0.max(selected.head.0);
            if lo == hi {
                if backward {
                    lo = source[..lo]
                        .grapheme_indices(true)
                        .next_back()
                        .map_or(0, |(i, _)| i);
                } else {
                    hi += source[hi..].graphemes(true).next().map_or(0, str::len);
                }
            }
            targets.push(Selection {
                anchor: BytePosition(lo),
                head: BytePosition(hi),
                affinity: selected.affinity,
            });
        }
        let normalized =
            normalize_selections(read.source, &targets, snapshot.selections.primary(), budget)
                .map_err(SessionError::Selection)?;
        let mut edits = Vec::new();
        edits
            .try_reserve_exact(normalized.selections().len())
            .map_err(|_| EditorError::Capacity)?;
        for target in normalized.selections() {
            let start = target.anchor.0.min(target.head.0);
            let end = target.anchor.0.max(target.head.0);
            if start != end {
                edits.push(Replacement {
                    start,
                    end,
                    expected: source[start..end].into(),
                    inserted: String::new(),
                });
            }
        }
        if edits.is_empty() {
            return Ok(());
        }
        self.groups
            .try_reserve(1)
            .map_err(|_| EditorError::Capacity)?;
        let outcome = self.host.edit(expected, edits)?;
        if let Some(group) = outcome.history_group {
            self.groups.push(group);
        }
        self.clamp_cursor()?;
        self.status = "Deleted through native selection set".into();
        self.collect_diagnostics();
        Ok(())
    }
    pub(super) fn project_native_graphemes(&mut self) -> Result<bool, EditorError> {
        let Some(snapshot) = self.native_selection()? else {
            return Ok(false);
        };
        let context = snapshot
            .context()
            .ok_or(SessionError::Selection(SelectionError::Unavailable))?;
        let grid = self.unicode_grid()?;
        let read = self.host.read(&self.tabs[self.active].document)?;
        let text = read.source.text();
        let project = |point| {
            if let Some(grid) = &grid {
                return grid[grid.partition_point(|p| p.0 < point).min(grid.len() - 1)].0;
            }
            text.grapheme_indices(true)
                .map(|(i, _)| i)
                .find(|&i| i >= point)
                .unwrap_or(text.len())
        };
        let mut projected = Vec::new();
        projected
            .try_reserve_exact(snapshot.selections.selections().len())
            .map_err(|_| EditorError::Capacity)?;
        for value in snapshot.selections.selections() {
            projected.push(Selection {
                anchor: BytePosition(project(value.anchor.0)),
                head: BytePosition(project(value.head.0)),
                affinity: value.affinity,
            });
        }
        if projected != snapshot.selections.selections() {
            self.host.set_logical_selections(
                &context,
                &projected,
                snapshot.selections.primary(),
            )?;
        }
        self.sync_native_selection()?;
        Ok(true)
    }
}
