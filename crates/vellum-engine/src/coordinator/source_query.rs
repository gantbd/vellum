//! Trusted-owner source-query values. source_context separately supplies selected
//! owned mediated scopes/lifetimes; public wire-envelope assembly remains open.
//! Both paths share materialization below. No host I/O.
use super::*;
pub(in crate::coordinator) mod result_contract;
use crate::coordinates::*;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceQueryBudget {
    pub max_scan_bytes: usize,
    /// Complete returned content bytes, not a global allocator/wire-envelope cap.
    pub max_result_bytes: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceQueryError {
    Owner(CoordinatorError),
    Coordinate(CoordinateError),
}
impl From<CoordinatorError> for SourceQueryError {
    fn from(error: CoordinatorError) -> Self {
        Self::Owner(error)
    }
}
impl From<CoordinateError> for SourceQueryError {
    fn from(error: CoordinateError) -> Self {
        Self::Coordinate(error)
    }
}
/// Closed executable read vocabulary within this explicit version. These are
/// canonical logical/native units, not grapheme, parser, file or visual movement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoordinateRequestV1 {
    ByteToLine(BytePosition),
    LineToByte(LinePosition),
    ByteToScalar(BytePosition),
    ScalarToByte(ScalarIndex),
    ByteToUtf16(BytePosition),
    Utf16ToByte(Utf16Index),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoordinateResultV1 {
    Byte(BytePosition),
    Line(LinePosition),
    Scalar(ScalarIndex),
    Utf16(Utf16Index),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoordinateReplyV1 {
    pub source: DocumentVersion,
    pub result: CoordinateResultV1,
}
#[derive(Clone, PartialEq, Eq)]
pub struct SourceLineReply {
    pub source: DocumentVersion,
    pub line: usize,
    pub span: LineSpan,
    body: Vec<u8>,
    delimiter: Vec<u8>,
}
impl SourceLineReply {
    pub(super) fn from_source(
        source: &Utf8Source,
        version: &DocumentVersion,
        line: usize,
        span: LineSpan,
    ) -> Result<Self, SourceQueryError> {
        let bytes = source.text().as_bytes();
        Ok(Self {
            source: version.clone(),
            line,
            span,
            body: copy(&bytes[span.start.0..span.body_end.0])?,
            delimiter: copy(&bytes[span.body_end.0..span.end.0])?,
        })
    }
    pub fn body(&self) -> &[u8] {
        &self.body
    }
    pub fn delimiter(&self) -> &[u8] {
        &self.delimiter
    }
}
impl std::fmt::Debug for SourceLineReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceLineReply(<redacted>)")
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct SourceRangeReply {
    pub source: DocumentVersion,
    pub start: BytePosition,
    pub end: BytePosition,
    bytes: Vec<u8>,
}
impl SourceRangeReply {
    pub(super) fn from_source(
        source: &Utf8Source,
        version: &DocumentVersion,
        start: BytePosition,
        end: BytePosition,
    ) -> Result<Self, SourceQueryError> {
        Ok(Self {
            source: version.clone(),
            start,
            end,
            bytes: copy(&source.text().as_bytes()[start.0..end.0])?,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
impl std::fmt::Debug for SourceRangeReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceRangeReply(<redacted>)")
    }
}
fn copy(bytes: &[u8]) -> Result<Vec<u8>, SourceQueryError> {
    let mut owned = Vec::new();
    owned
        .try_reserve_exact(bytes.len())
        .map_err(|_| CoordinateError::Capacity)?;
    owned.extend_from_slice(bytes);
    Ok(owned)
}
fn capture<T>(recorder: &mut Recorder, boundary: Boundary, result: &Result<T, SourceQueryError>) {
    recorder.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(SourceQueryError::Owner(CoordinatorError::Faulted)) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time: None,
    });
}
impl SourceCoordinator {
    /// Complete owned body/delimiter product under an exact current revision.
    /// No total line count, source length or unrelated content is added.
    pub fn read_source_line(
        &self,
        expected: &DocumentVersion,
        line: usize,
        budget: SourceQueryBudget,
        recorder: &mut Recorder,
    ) -> Result<SourceLineReply, SourceQueryError> {
        let result = (|| {
            self.ensure_operational()?;
            let document = self.validate_version(expected)?;
            let span = SourceCoordinates::new(&document.source).line_span(
                line,
                CoordinateBudget {
                    max_scan_bytes: budget.max_scan_bytes,
                },
            )?;
            if span.end.0 - span.start.0 > budget.max_result_bytes {
                return Err(CoordinateError::Limit.into());
            }
            SourceLineReply::from_source(&document.source, expected, line, span)
        })();
        capture(recorder, Boundary::SourceLineQuery, &result);
        result
    }
    /// Scalar-boundary byte ranges; CRLF interior remains a valid canonical
    /// boundary. Copies are independent disclosures, not edit grants or leases.
    pub fn read_source_range(
        &self,
        expected: &DocumentVersion,
        start: BytePosition,
        end: BytePosition,
        max_result_bytes: usize,
        recorder: &mut Recorder,
    ) -> Result<SourceRangeReply, SourceQueryError> {
        let result = (|| {
            self.ensure_operational()?;
            let document = self.validate_version(expected)?;
            SourceCoordinates::new(&document.source).validate_range(start, end)?;
            if end.0 - start.0 > max_result_bytes {
                return Err(CoordinateError::Limit.into());
            }
            SourceRangeReply::from_source(&document.source, expected, start, end)
        })();
        capture(recorder, Boundary::SourceRangeQuery, &result);
        result
    }
    pub fn convert_source_coordinate(
        &self,
        expected: &DocumentVersion,
        request: CoordinateRequestV1,
        budget: CoordinateBudget,
        recorder: &mut Recorder,
    ) -> Result<CoordinateReplyV1, SourceQueryError> {
        let result = (|| {
            self.ensure_operational()?;
            let document = self.validate_version(expected)?;
            let coordinates = SourceCoordinates::new(&document.source);
            let result = match request {
                CoordinateRequestV1::ByteToLine(position) => {
                    CoordinateResultV1::Line(coordinates.byte_to_line(position, budget)?)
                }
                CoordinateRequestV1::LineToByte(position) => {
                    CoordinateResultV1::Byte(coordinates.line_to_byte(position, budget)?)
                }
                CoordinateRequestV1::ByteToScalar(position) => {
                    CoordinateResultV1::Scalar(coordinates.byte_to_scalar(position, budget)?)
                }
                CoordinateRequestV1::ScalarToByte(position) => {
                    CoordinateResultV1::Byte(coordinates.scalar_to_byte(position, budget)?)
                }
                CoordinateRequestV1::ByteToUtf16(position) => {
                    CoordinateResultV1::Utf16(coordinates.byte_to_utf16(position, budget)?)
                }
                CoordinateRequestV1::Utf16ToByte(position) => {
                    CoordinateResultV1::Byte(coordinates.utf16_to_byte(position, budget)?)
                }
            };
            Ok(CoordinateReplyV1 {
                source: expected.clone(),
                result,
            })
        })();
        capture(recorder, Boundary::SourceCoordinateConversion, &result);
        result
    }
}
