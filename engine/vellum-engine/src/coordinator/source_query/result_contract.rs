//! Native producer binding/shape guards; not a wire codec or independent byte oracle.
use super::*;
#[cfg(test)]
pub(in crate::coordinator) mod tests;
use crate::coordinator::source_context::Resolved;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadResultInvariantV1 {
    NativeSearchFacadeResultInvariant,
    NativeSearchResultInvariant,
    NativeHistoryResultInvariant,
    NativeHistoryFacadeResultInvariant,
    NativeSourceResultInvariant,
    NativeAnnotationResultInvariant,
    NativeReadAcquisitionResultInvariant,
    NativeSourceFacadeResultInvariant,
    NativeAnnotationFacadeResultInvariant,
    NativeAcquisitionFacadeResultInvariant,
}
impl ReadResultInvariantV1 {
    pub fn code(self) -> &'static str {
        match self {
            Self::NativeSearchFacadeResultInvariant => "NATIVE_SEARCH_FACADE_RESULT_INVARIANT",
            Self::NativeSearchResultInvariant => "NATIVE_SEARCH_RESULT_INVARIANT",
            Self::NativeHistoryResultInvariant => "NATIVE_HISTORY_RESULT_INVARIANT",
            Self::NativeHistoryFacadeResultInvariant => "NATIVE_HISTORY_FACADE_RESULT_INVARIANT",
            Self::NativeAcquisitionFacadeResultInvariant => {
                "NATIVE_ACQUISITION_FACADE_RESULT_INVARIANT"
            }
            Self::NativeSourceResultInvariant => "NATIVE_SOURCE_RESULT_INVARIANT",
            Self::NativeAnnotationResultInvariant => "NATIVE_ANNOTATION_RESULT_INVARIANT",
            Self::NativeReadAcquisitionResultInvariant => {
                "NATIVE_READ_ACQUISITION_RESULT_INVARIANT"
            }
            Self::NativeSourceFacadeResultInvariant => "NATIVE_SOURCE_FACADE_RESULT_INVARIANT",
            Self::NativeAnnotationFacadeResultInvariant => {
                "NATIVE_ANNOTATION_FACADE_RESULT_INVARIANT"
            }
        }
    }
}
fn valid(
    expected: Resolved,
    version: &DocumentVersion,
    content: &SourceContextContentV1,
    bound: bool,
) -> bool {
    if !bound {
        return false;
    }
    match (expected, content) {
        (Resolved::Range(a, b), SourceContextContentV1::Range(v)) => {
            v.source == *version
                && v.start == a
                && v.end == b
                && b.0.checked_sub(a.0) == Some(v.bytes().len())
        }
        (Resolved::Line(index, span), SourceContextContentV1::Line(v)) => {
            v.source == *version
                && v.line == index
                && v.span == span
                && span.body_end.0.checked_sub(span.start.0) == Some(v.body().len())
                && span.end.0.checked_sub(span.body_end.0) == Some(v.delimiter().len())
                && matches!(v.delimiter(), b"" | b"\n" | b"\r" | b"\r\n")
        }
        _ => false,
    }
}
impl SourceCoordinator {
    /// Trusted-owner diagnosis, not a recipient-safe data or permission product.
    pub fn read_result_invariant(&self) -> Option<ReadResultInvariantV1> {
        self.read_result_invariant
    }
    pub(in crate::coordinator) fn guard_source_result(
        &mut self,
        expected: Resolved,
        version: &DocumentVersion,
        content: &SourceContextContentV1,
        bound: bool,
        r: &mut Recorder,
    ) -> Result<(), SourceReadContextErrorV1> {
        let accepted = valid(expected, version, content, bound);
        if self.finish_read_result(
            accepted,
            ReadResultInvariantV1::NativeSourceResultInvariant,
            Boundary::SourceReadResultValidation,
            r,
        ) {
            Ok(())
        } else {
            Err(SourceReadContextErrorV1::Faulted)
        }
    }
    pub(in crate::coordinator) fn finish_read_result(
        &mut self,
        accepted: bool,
        cause: ReadResultInvariantV1,
        boundary: Boundary,
        r: &mut Recorder,
    ) -> bool {
        if !accepted {
            self.faulted = true;
            if self.read_result_invariant.is_none() {
                self.read_result_invariant = Some(cause);
            }
        }
        r.record(Observation {
            boundary,
            disposition: if accepted {
                Disposition::Completed
            } else {
                Disposition::Unknown
            },
            service_time: None,
        });
        accepted
    }
}
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(in crate::coordinator) enum AcquisitionResultFault {
    Context,
    Classes,
    Roster,
    MemberAlias,
    Duplicate,
    Document,
    StagedBinding,
    RetainedParent,
}
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(in crate::coordinator) enum AnnotationResultFault {
    Snapshot,
    RecordAlias,
    Duplicate,
    Continuation,
    MissingContent,
    Version,
    Location,
    Truncate,
}
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(in crate::coordinator) enum SourceResultFault {
    Alias,
    Reference,
    Extent,
    Length,
    Delimiter,
    Variant,
    LineIndex,
}
#[cfg(test)]
pub(in crate::coordinator) fn corrupt(
    content: &mut SourceContextContentV1,
    fault: Option<SourceResultFault>,
) {
    let bump = |n: usize| n.checked_add(1).unwrap_or(0);
    match fault {
        Some(SourceResultFault::Reference) => match content {
            SourceContextContentV1::Range(v) => {
                v.source.revision = v.source.revision.checked_add(1).unwrap_or(0)
            }
            SourceContextContentV1::Line(v) => {
                v.source.revision = v.source.revision.checked_add(1).unwrap_or(0)
            }
        },
        Some(SourceResultFault::Extent) => match content {
            SourceContextContentV1::Range(v) => v.end.0 = bump(v.end.0),
            SourceContextContentV1::Line(v) => v.span.end.0 = bump(v.span.end.0),
        },
        Some(SourceResultFault::Length) => match content {
            SourceContextContentV1::Range(v) => {
                v.bytes.pop();
            }
            SourceContextContentV1::Line(v) => {
                v.body.pop();
            }
        },
        Some(SourceResultFault::Delimiter) => {
            if let SourceContextContentV1::Line(v) = content
                && let Some(b) = v.delimiter.first_mut()
            {
                *b = b'x';
            }
        }
        Some(SourceResultFault::LineIndex) => {
            if let SourceContextContentV1::Line(v) = content {
                v.line = bump(v.line);
            }
        }
        Some(SourceResultFault::Variant) => {
            let other = match content {
                SourceContextContentV1::Range(v) => SourceContextContentV1::Line(SourceLineReply {
                    source: v.source.clone(),
                    line: 0,
                    span: LineSpan {
                        start: v.start,
                        body_end: v.end,
                        end: v.end,
                    },
                    body: std::mem::take(&mut v.bytes),
                    delimiter: Vec::new(),
                }),
                SourceContextContentV1::Line(v) => {
                    SourceContextContentV1::Range(SourceRangeReply {
                        source: v.source.clone(),
                        start: v.span.start,
                        end: v.span.body_end,
                        bytes: std::mem::take(&mut v.body),
                    })
                }
            };
            *content = other;
        }
        _ => {}
    }
}
