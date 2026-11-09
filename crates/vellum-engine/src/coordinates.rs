//! Bounded projections of an already-disclosed immutable canonical source value.
//! These are not file offsets, grapheme indices, parser points or display cells.
//! No normalization, clamping, I/O, callback or persistent index is involved.
use crate::source::Utf8Source;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BytePosition(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScalarIndex(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Utf16Index(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinePosition {
    pub line: usize,
    pub body_byte_column: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineSpan {
    pub start: BytePosition,
    pub body_end: BytePosition,
    pub end: BytePosition,
}
/// Maximum canonical prefix extent inspected by linear scanning, in UTF-8 bytes.
/// CR lookahead is included. Constant-time coordinate checks are separate; this
/// is not a physical memory, exact CPU-instruction or measured-time promise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoordinateBudget {
    pub max_scan_bytes: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoordinateError {
    InvalidByte,
    InvalidBoundary,
    InvalidScalar,
    InvalidUtf16,
    InvalidLine,
    InvalidColumn,
    InvalidRange,
    UnavailableProjection,
    Limit,
    Capacity,
}
#[derive(Debug)]
pub struct SourceCoordinates<'a> {
    source: &'a Utf8Source,
}
impl<'a> SourceCoordinates<'a> {
    pub fn new(source: &'a Utf8Source) -> Self {
        Self { source }
    }
    fn bytes(&self) -> &[u8] {
        self.source.text().as_bytes()
    }
    fn touch(&self, end: usize, budget: CoordinateBudget) -> Result<(), CoordinateError> {
        if end > budget.max_scan_bytes {
            Err(CoordinateError::Limit)
        } else {
            Ok(())
        }
    }
    pub fn validate_byte(&self, position: BytePosition) -> Result<(), CoordinateError> {
        if position.0 > self.bytes().len() {
            return Err(CoordinateError::InvalidByte);
        }
        if !self.source.text().is_char_boundary(position.0) {
            return Err(CoordinateError::InvalidBoundary);
        }
        Ok(())
    }
    pub fn validate_range(
        &self,
        start: BytePosition,
        end: BytePosition,
    ) -> Result<(), CoordinateError> {
        if start.0 > end.0 {
            return Err(CoordinateError::InvalidRange);
        }
        self.validate_byte(start)?;
        self.validate_byte(end)
    }
    fn delimiter_end(
        &self,
        cursor: usize,
        budget: CoordinateBudget,
    ) -> Result<usize, CoordinateError> {
        self.touch(cursor + 1, budget)?;
        if self.bytes()[cursor] == b'\r' && cursor + 1 < self.bytes().len() {
            self.touch(cursor + 2, budget)?;
            if self.bytes()[cursor + 1] == b'\n' {
                return Ok(cursor + 2);
            }
        }
        Ok(cursor + 1)
    }
    fn line_start(&self, line: usize, budget: CoordinateBudget) -> Result<usize, CoordinateError> {
        if line == 0 {
            return Ok(0);
        }
        let (mut cursor, mut current) = (0, 0);
        while cursor < self.bytes().len() {
            self.touch(cursor + 1, budget)?;
            if matches!(self.bytes()[cursor], b'\r' | b'\n') {
                cursor = self.delimiter_end(cursor, budget)?;
                current += 1;
                if current == line {
                    return Ok(cursor);
                }
            } else {
                cursor += 1;
            }
        }
        Err(CoordinateError::InvalidLine)
    }
    /// LF/CRLF/CR logical lines; an empty or delimiter-terminated source has a
    /// final empty line. Other Unicode separators are ordinary body content.
    pub fn line_span(
        &self,
        line: usize,
        budget: CoordinateBudget,
    ) -> Result<LineSpan, CoordinateError> {
        let start = self.line_start(line, budget)?;
        let mut cursor = start;
        while cursor < self.bytes().len() {
            self.touch(cursor + 1, budget)?;
            if matches!(self.bytes()[cursor], b'\r' | b'\n') {
                return Ok(LineSpan {
                    start: BytePosition(start),
                    body_end: BytePosition(cursor),
                    end: BytePosition(self.delimiter_end(cursor, budget)?),
                });
            }
            cursor += 1;
        }
        Ok(LineSpan {
            start: BytePosition(start),
            body_end: BytePosition(cursor),
            end: BytePosition(cursor),
        })
    }
    pub fn byte_to_line(
        &self,
        position: BytePosition,
        budget: CoordinateBudget,
    ) -> Result<LinePosition, CoordinateError> {
        self.validate_byte(position)?;
        let (mut cursor, mut line, mut start) = (0, 0, 0);
        while cursor < position.0 {
            self.touch(cursor + 1, budget)?;
            if matches!(self.bytes()[cursor], b'\r' | b'\n') {
                let end = self.delimiter_end(cursor, budget)?;
                if end > position.0 {
                    return Err(CoordinateError::UnavailableProjection);
                }
                cursor = end;
                start = end;
                line += 1;
            } else {
                cursor += 1;
            }
        }
        Ok(LinePosition {
            line,
            body_byte_column: position.0 - start,
        })
    }
    pub fn line_to_byte(
        &self,
        position: LinePosition,
        budget: CoordinateBudget,
    ) -> Result<BytePosition, CoordinateError> {
        let start = self.line_start(position.line, budget)?;
        let end = start
            .checked_add(position.body_byte_column)
            .filter(|end| *end <= self.bytes().len())
            .ok_or(CoordinateError::InvalidColumn)?;
        for cursor in start..end {
            self.touch(cursor + 1, budget)?;
            if matches!(self.bytes()[cursor], b'\r' | b'\n') {
                return Err(CoordinateError::InvalidColumn);
            }
        }
        let result = BytePosition(end);
        self.validate_byte(result)?;
        Ok(result)
    }
    pub fn byte_to_scalar(
        &self,
        position: BytePosition,
        budget: CoordinateBudget,
    ) -> Result<ScalarIndex, CoordinateError> {
        self.validate_byte(position)?;
        self.touch(position.0, budget)?;
        Ok(ScalarIndex(
            self.source.text()[..position.0].chars().count(),
        ))
    }
    pub fn byte_to_utf16(
        &self,
        position: BytePosition,
        budget: CoordinateBudget,
    ) -> Result<Utf16Index, CoordinateError> {
        self.validate_byte(position)?;
        self.touch(position.0, budget)?;
        let count =
            self.source.text()[..position.0]
                .chars()
                .try_fold(0usize, |count, scalar| {
                    count
                        .checked_add(scalar.len_utf16())
                        .ok_or(CoordinateError::Limit)
                })?;
        Ok(Utf16Index(count))
    }
    // All callers advance from scalar boundaries over a previously validated
    // Rust UTF-8 string. Inspect the leading byte before admitting the full step.
    fn scalar_end(
        &self,
        cursor: usize,
        budget: CoordinateBudget,
    ) -> Result<Option<usize>, CoordinateError> {
        let Some(&first) = self.bytes().get(cursor) else {
            return Ok(None);
        };
        let width = if first < 0x80 {
            1
        } else if first < 0xe0 {
            2
        } else if first < 0xf0 {
            3
        } else {
            4
        };
        let end = cursor + width;
        self.touch(end, budget)?;
        Ok(Some(end))
    }
    pub fn scalar_to_byte(
        &self,
        position: ScalarIndex,
        budget: CoordinateBudget,
    ) -> Result<BytePosition, CoordinateError> {
        let mut cursor = 0;
        for _ in 0..position.0 {
            cursor = self
                .scalar_end(cursor, budget)?
                .ok_or(CoordinateError::InvalidScalar)?;
        }
        Ok(BytePosition(cursor))
    }
    pub fn utf16_to_byte(
        &self,
        position: Utf16Index,
        budget: CoordinateBudget,
    ) -> Result<BytePosition, CoordinateError> {
        let (mut cursor, mut units) = (0, 0usize);
        while units < position.0 {
            let end = self
                .scalar_end(cursor, budget)?
                .ok_or(CoordinateError::InvalidUtf16)?;
            let next = units
                .checked_add(if end - cursor == 4 { 2 } else { 1 })
                .ok_or(CoordinateError::Limit)?;
            if next > position.0 {
                return Err(CoordinateError::InvalidBoundary);
            }
            cursor = end;
            units = next;
        }
        Ok(BytePosition(cursor))
    }
}
