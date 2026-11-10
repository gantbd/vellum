//! Explicit positional selections, not agent-batch normalization or geometry.
use crate::attachment::Affinity;
use crate::coordinates::{BytePosition, CoordinateError, SourceCoordinates};
use crate::source::{SourceError, Utf8Source};
pub const POSITIONAL_SELECTION_PROFILE_V1: &str = "explicit-positional-selection/1";
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: BytePosition,
    pub head: BytePosition,
    pub affinity: Affinity,
}
impl std::fmt::Debug for Selection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Selection(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectionBudget {
    pub max_input_selections: usize,
    pub max_output_selections: usize,
    pub max_work: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionError {
    Unavailable,
    Stale,
    InvalidSelection,
    InvalidBoundary,
    Limit,
    Capacity,
    VersionExhausted,
    Faulted,
    Source(SourceError),
    Coordinate(CoordinateError),
}
#[derive(Clone, PartialEq, Eq)]
pub struct SelectionSet {
    records: Vec<Selection>,
    primary: usize,
}
impl std::fmt::Debug for SelectionSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SelectionSet(<redacted>)")
    }
}
impl SelectionSet {
    pub fn selections(&self) -> &[Selection] {
        &self.records
    }
    pub fn primary(&self) -> usize {
        self.primary
    }
    pub(crate) fn detached() -> Self {
        Self {
            records: Vec::new(),
            primary: 0,
        }
    }
    pub(crate) fn copy(&self) -> Result<Self, SelectionError> {
        let mut records = Vec::new();
        records
            .try_reserve_exact(self.records.len())
            .map_err(|_| SelectionError::Capacity)?;
        records.extend_from_slice(&self.records);
        Ok(Self {
            records,
            primary: self.primary,
        })
    }
}
fn step(remaining: &mut usize) -> Result<(), SelectionError> {
    *remaining = remaining.checked_sub(1).ok_or(SelectionError::Limit)?;
    Ok(())
}
#[derive(Clone, Copy)]
struct Row {
    lo: usize,
    hi: usize,
    owner: usize,
    primary: bool,
}
fn rank(affinity: Affinity) -> usize {
    match affinity {
        Affinity::Before => 0,
        Affinity::After => 1,
    }
}
fn order(row: &Row, input: &[Selection]) -> (usize, usize, usize) {
    (row.lo, row.hi, rank(input[row.owner].affinity))
}
fn sort(
    rows: &mut [Row],
    input: &[Selection],
    remaining: &mut usize,
) -> Result<(), SelectionError> {
    for i in 1..rows.len() {
        let mut j = i;
        while j > 0 {
            step(remaining)?;
            if order(&rows[j - 1], input) <= order(&rows[j], input) {
                break;
            }
            rows.swap(j - 1, j);
            j -= 1;
        }
    }
    Ok(())
}
fn merge(last: &mut Row, next: Row) {
    last.hi = last.hi.max(next.hi);
    if next.primary || (!last.primary && next.owner < last.owner) {
        last.owner = next.owner;
    }
    last.primary |= next.primary;
}
/// LF/CRLF structure and grapheme semantics are independent of this positional
/// profile. Work counts validations/comparisons/cluster and emission steps.
pub fn normalize_selections(
    source: &Utf8Source,
    input: &[Selection],
    primary: usize,
    budget: SelectionBudget,
) -> Result<SelectionSet, SelectionError> {
    if input.is_empty() || primary >= input.len() {
        return Err(SelectionError::InvalidSelection);
    }
    if input.len() > budget.max_input_selections {
        return Err(SelectionError::Limit);
    }
    let mut remaining = budget.max_work;
    let mut rows = Vec::new();
    rows.try_reserve_exact(input.len())
        .map_err(|_| SelectionError::Capacity)?;
    for (owner, selection) in input.iter().enumerate() {
        step(&mut remaining)?;
        let coordinates = SourceCoordinates::new(source);
        coordinates
            .validate_byte(selection.anchor)
            .map_err(|_| SelectionError::InvalidBoundary)?;
        coordinates
            .validate_byte(selection.head)
            .map_err(|_| SelectionError::InvalidBoundary)?;
        rows.push(Row {
            lo: selection.anchor.0.min(selection.head.0),
            hi: selection.anchor.0.max(selection.head.0),
            owner,
            primary: owner == primary,
        });
    }
    sort(&mut rows, input, &mut remaining)?;
    let mut ranges: Vec<Row> = Vec::new();
    let mut carets: Vec<Row> = Vec::new();
    ranges
        .try_reserve_exact(input.len())
        .map_err(|_| SelectionError::Capacity)?;
    carets
        .try_reserve_exact(input.len())
        .map_err(|_| SelectionError::Capacity)?;
    for row in rows {
        step(&mut remaining)?;
        if row.lo < row.hi {
            if let Some(last) = ranges.last_mut()
                && row.lo < last.hi
            {
                merge(last, row);
            } else {
                ranges.push(row);
            }
        } else if let Some(last) = carets.last_mut()
            && last.lo == row.lo
            && input[last.owner].affinity == input[row.owner].affinity
        {
            merge(last, row);
        } else {
            carets.push(row);
        }
    }
    let count = ranges
        .len()
        .checked_add(carets.len())
        .ok_or(SelectionError::Limit)?;
    if count > budget.max_output_selections {
        return Err(SelectionError::Limit);
    }
    ranges
        .try_reserve_exact(carets.len())
        .map_err(|_| SelectionError::Capacity)?;
    ranges.extend(carets);
    sort(&mut ranges, input, &mut remaining)?;
    let mut records = Vec::new();
    records
        .try_reserve_exact(count)
        .map_err(|_| SelectionError::Capacity)?;
    let mut normalized_primary = None;
    for row in ranges {
        step(&mut remaining)?;
        if row.primary {
            normalized_primary = Some(records.len());
        }
        let owner = input[row.owner];
        let backward = owner.anchor.0 > owner.head.0;
        records.push(Selection {
            anchor: BytePosition(if backward { row.hi } else { row.lo }),
            head: BytePosition(if backward { row.lo } else { row.hi }),
            affinity: owner.affinity,
        });
    }
    Ok(SelectionSet {
        records,
        primary: normalized_primary.ok_or(SelectionError::Faulted)?,
    })
}
