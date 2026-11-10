//! Pure bounded lowering into the existing canonical original-snapshot edit batch.
use super::*;
struct Builder {
    edits: Vec<Replacement>,
    bytes: usize,
    prefix: usize,
    budget: LineEditBudget,
}
impl Builder {
    fn charge(&mut self, bytes: usize) -> Result<(), LineEditError> {
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .filter(|n| *n <= self.budget.max_derived_bytes)
            .ok_or(LineEditError::Limit)?;
        Ok(())
    }
    fn prefix(&mut self, source: &str, start: usize, end: usize) -> Result<usize, LineEditError> {
        let mut at = start;
        while at < end && matches!(source.as_bytes()[at], b' ' | b'\t') {
            self.prefix = self
                .prefix
                .checked_add(1)
                .filter(|n| *n <= self.budget.max_prefix_bytes)
                .ok_or(LineEditError::Limit)?;
            at += 1;
        }
        Ok(at)
    }
    fn push(
        &mut self,
        source: &str,
        start: usize,
        end: usize,
        inserted: &str,
    ) -> Result<(), LineEditError> {
        self.charge(std::mem::size_of::<Replacement>())?;
        self.charge(end - start)?;
        self.charge(inserted.len())?;
        self.edits
            .try_reserve(1)
            .map_err(|_| LineEditError::Capacity)?;
        self.edits.push(Replacement {
            start,
            end,
            expected: copy(&source[start..end])?,
            inserted: copy(inserted)?,
        });
        Ok(())
    }
}
fn columns(
    bytes: &[u8],
    mut value: usize,
    width: usize,
    max: usize,
) -> Result<usize, LineEditError> {
    for byte in bytes {
        let step = if *byte == b'\t' {
            width - value % width
        } else {
            1
        };
        value = value
            .checked_add(step)
            .filter(|n| *n <= max)
            .ok_or(LineEditError::Limit)?;
    }
    Ok(value)
}
fn coordinate(error: crate::coordinates::CoordinateError) -> LineEditError {
    LineEditError::Selection(SelectionError::Coordinate(error))
}
pub(super) fn edits(
    source: &Utf8Source,
    selections: &SelectionSet,
    request: &LineEditV1,
    budget: LineEditBudget,
) -> Result<Vec<Replacement>, LineEditError> {
    if let Some((unit, width)) = request.unit() {
        if unit.len() > budget.max_unit_bytes || width > budget.max_indent_columns {
            return Err(LineEditError::Limit);
        }
        if unit.is_empty() || (unit != "\t" && !unit.bytes().all(|b| b == b' ')) || width == 0 {
            return Err(LineEditError::InvalidSettings);
        }
    }
    let mut out = Builder {
        edits: Vec::new(),
        bytes: 0,
        prefix: 0,
        budget,
    };
    out.charge(std::mem::size_of::<LineEdit>())?;
    out.charge(request.unit().map_or(0, |(unit, _)| unit.len()))?;
    let coordinates = SourceCoordinates::new(source);
    let scan = CoordinateBudget {
        max_scan_bytes: budget.max_source_scan_bytes,
    };
    let text = source.text();
    let mut lines = Vec::new();
    for selection in selections.selections() {
        let lo = selection.anchor.0.min(selection.head.0);
        let hi = selection.anchor.0.max(selection.head.0);
        let from = coordinates
            .byte_to_line(BytePosition(lo), scan)
            .map_err(coordinate)?;
        let to = coordinates
            .byte_to_line(BytePosition(hi), scan)
            .map_err(coordinate)?;
        if let LineEditV1::Newline { delimiter, indent } = request {
            if selections.selections().len() > budget.max_lines {
                return Err(LineEditError::Limit);
            }
            let row = coordinates.line_span(from.line, scan).map_err(coordinate)?;
            let end = if *indent == NewlineIndentV1::CopyPrefix {
                out.prefix(text, row.start.0, lo.min(row.body_end.0))?
            } else {
                row.start.0
            };
            let prefix = &text[row.start.0..end];
            let len = delimiter
                .text()
                .len()
                .checked_add(prefix.len())
                .ok_or(LineEditError::Limit)?;
            // Bound this temporary buffer before allocating, independently of
            // the aggregate request/edit descriptor checked by push.
            if len > budget.max_derived_bytes {
                return Err(LineEditError::Limit);
            }
            let mut inserted = String::new();
            inserted
                .try_reserve_exact(len)
                .map_err(|_| LineEditError::Capacity)?;
            inserted.push_str(delimiter.text());
            inserted.push_str(prefix);
            out.push(text, lo, hi, &inserted)?;
        } else {
            let row = coordinates.line_span(to.line, scan).map_err(coordinate)?;
            let last = if hi > lo && hi == row.start.0 {
                to.line
                    .checked_sub(1)
                    .ok_or(LineEditError::InvalidSettings)?
            } else {
                to.line
            };
            if last
                .checked_sub(from.line)
                .and_then(|n| n.checked_add(1))
                .is_none_or(|n| n > budget.max_lines)
            {
                return Err(LineEditError::Limit);
            }
            for line in from.line..=last {
                if let Err(at) = lines.binary_search(&line) {
                    if lines.len() >= budget.max_lines {
                        return Err(LineEditError::Limit);
                    }
                    lines.try_reserve(1).map_err(|_| LineEditError::Capacity)?;
                    lines.insert(at, line);
                }
            }
        }
    }
    if let Some((unit, width)) = request.unit() {
        for line in lines {
            let row = coordinates.line_span(line, scan).map_err(coordinate)?;
            let end = out.prefix(text, row.start.0, row.body_end.0)?;
            let prefix = &text[row.start.0..end];
            let current = columns(prefix.as_bytes(), 0, width, budget.max_indent_columns)?;
            if matches!(request, LineEditV1::Indent { .. }) {
                let start = columns(unit.as_bytes(), 0, width, budget.max_indent_columns)?;
                columns(prefix.as_bytes(), start, width, budget.max_indent_columns)?;
                out.push(text, row.start.0, row.start.0, unit)?;
            } else {
                let step = if unit == "\t" { width } else { unit.len() };
                let target = current.saturating_sub(step);
                let (tabs, spaces) = if unit == "\t" {
                    (target / width, target % width)
                } else {
                    (0, target)
                };
                let size = tabs.checked_add(spaces).ok_or(LineEditError::Limit)?;
                let mut inserted = String::new();
                inserted
                    .try_reserve_exact(size)
                    .map_err(|_| LineEditError::Capacity)?;
                for _ in 0..tabs {
                    inserted.push('\t');
                }
                for _ in 0..spaces {
                    inserted.push(' ');
                }
                let keep = prefix
                    .bytes()
                    .zip(inserted.bytes())
                    .take_while(|(a, b)| a == b)
                    .count();
                if keep < prefix.len() || keep < inserted.len() {
                    out.push(text, row.start.0 + keep, end, &inserted[keep..])?;
                }
            }
        }
    }
    Ok(out.edits)
}
