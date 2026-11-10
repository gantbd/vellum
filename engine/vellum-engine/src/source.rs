//! Pure source-value preparation; not an authorized engine facade.
//!
//! Values are owned and immutable through this API, with no document identity,
//! revision, policy, history or publication. A coordinator must independently
//! validate all of those before adopting a candidate. No I/O or callbacks run.

const BOM: &[u8] = b"\xef\xbb\xbf";

fn observe_codec<T>(
    r: &mut crate::instrumentation::Recorder,
    boundary: crate::instrumentation::Boundary,
    result: &Result<T, SourceError>,
) {
    use crate::instrumentation::{Disposition, Observation};
    r.record(Observation {
        boundary,
        disposition: if result.is_ok() {
            Disposition::Completed
        } else {
            Disposition::Refused
        },
        service_time: None,
    });
}

/// Shared original-coordinate geometry check for sorted complete records.
pub(crate) fn edits_ambiguous(left: &Replacement, right: &Replacement) -> bool {
    right.start < left.end
        || (right.start == left.end && (left.start == left.end || right.start == right.end))
}

pub(crate) fn copy_replacements(edits: &[Replacement]) -> Result<Vec<Replacement>, SourceError> {
    let copy = |value: &str| -> Result<String, SourceError> {
        let mut owned = String::new();
        owned
            .try_reserve_exact(value.len())
            .map_err(|_| SourceError::Capacity)?;
        owned.push_str(value);
        Ok(owned)
    };
    let mut owned = Vec::new();
    owned
        .try_reserve_exact(edits.len())
        .map_err(|_| SourceError::Capacity)?;
    for edit in edits {
        owned.push(Replacement {
            start: edit.start,
            end: edit.end,
            expected: copy(&edit.expected)?,
            inserted: copy(&edit.inserted)?,
        });
    }
    Ok(owned)
}

/// Logical preparation bounds, not physical allocation or deployment budgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Decoded UTF-8 text bytes, excluding the serialization BOM.
    pub max_source_bytes: usize,
    pub max_edits: usize,
    /// Sum of all expected and inserted UTF-8 bytes in the request.
    pub max_edit_payload_bytes: usize,
}

/// One complete original-source replacement. Endpoints are half-open byte offsets.
#[derive(Clone, PartialEq, Eq)]
pub struct Replacement {
    pub start: usize,
    pub end: usize,
    pub expected: String,
    pub inserted: String,
}

impl std::fmt::Debug for Replacement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Replacement")
            .field("expected", &"<redacted>")
            .field("inserted", &"<redacted>")
            .finish_non_exhaustive()
    }
}

/// Preparation refusals carry no source fragments or arbitrary host messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceError {
    InvalidUtf8,
    SourceLimit,
    StagingLimit,
    EditLimit,
    PayloadLimit,
    InvalidRange,
    InvalidBoundary,
    ExpectedMismatch,
    AmbiguousEdits,
    AmbiguousBom,
    Capacity,
}

/// An independent source value, never a mutable coordinator-owned document.
///
/// References borrow this immutable value. Preparation returns a new value and
/// cannot invalidate the original. Possession already exposes text; this type is
/// not an authorization-aware snapshot handle. Debug intentionally omits content.
#[derive(Clone, PartialEq, Eq)]
pub struct Utf8Source {
    text: String,
    bom: bool,
}

impl std::fmt::Debug for Utf8Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Utf8Source")
            .field("text", &"<redacted>")
            .field("bom", &self.bom)
            .finish()
    }
}

impl Utf8Source {
    /// Logical authored UTF-8, not a file byte import. In particular every
    /// leading U+FEFF remains text; output signature intent belongs to the owner.
    pub(crate) fn from_logical_utf8_observed(
        bytes: &[u8],
        maximum: usize,
        r: &mut crate::instrumentation::Recorder,
    ) -> Result<Self, SourceError> {
        let result = (|| {
            if bytes.len() > maximum {
                return Err(SourceError::SourceLimit);
            }
            let decoded = std::str::from_utf8(bytes).map_err(|_| SourceError::InvalidUtf8)?;
            let mut text = String::new();
            text.try_reserve_exact(decoded.len())
                .map_err(|_| SourceError::Capacity)?;
            text.push_str(decoded);
            Ok(Self { text, bom: false })
        })();
        observe_codec(
            r,
            crate::instrumentation::Boundary::SourceAuthoringValidation,
            &result,
        );
        result
    }
    /// Fallible immutable copy without reinterpreting a literal leading U+FEFF.
    pub(crate) fn copy_owned(&self) -> Result<Self, SourceError> {
        let mut text = String::new();
        text.try_reserve_exact(self.text.len())
            .map_err(|_| SourceError::Capacity)?;
        text.push_str(&self.text);
        Ok(Self {
            text,
            bom: self.bom,
        })
    }
    /// Allocation-free candidate/journal consistency check at publication.
    /// Explicit history order is preserved; all pieces address the same source.
    pub(crate) fn matches_journal(&self, edits: &[Replacement], candidate: &Self) -> bool {
        fn consume(output: &str, cursor: &mut usize, piece: &str) -> bool {
            let Some(end) = cursor.checked_add(piece.len()) else {
                return false;
            };
            if output.get(*cursor..end) != Some(piece) {
                return false;
            }
            *cursor = end;
            true
        }
        if self.bom != candidate.bom {
            return false;
        }
        let (mut source_cursor, mut destination_cursor) = (0usize, 0usize);
        for edit in edits {
            if edit.start < source_cursor || edit.start > edit.end {
                return false;
            }
            let Some(unchanged) = self.text.get(source_cursor..edit.start) else {
                return false;
            };
            if self.text.get(edit.start..edit.end) != Some(edit.expected.as_str()) {
                return false;
            }
            if !consume(&candidate.text, &mut destination_cursor, unchanged)
                || !consume(&candidate.text, &mut destination_cursor, &edit.inserted)
            {
                return false;
            }
            source_cursor = edit.end;
        }
        let Some(tail) = self.text.get(source_cursor..) else {
            return false;
        };
        consume(&candidate.text, &mut destination_cursor, tail)
            && destination_cursor == candidate.text.len()
    }

    /// Same preparation with optional bounded diagnostic capture afterward.
    /// No host clock, subscriber or exporter is invoked, and capture cannot
    /// change the returned candidate/refusal. This is not publication evidence.
    pub fn prepare_observed(
        &self,
        edits: &[Replacement],
        limits: Limits,
        recorder: &mut crate::instrumentation::Recorder,
    ) -> Result<Self, SourceError> {
        use crate::instrumentation::{Boundary, Disposition, Observation};
        let result = self.prepare(edits, limits);
        recorder.record(Observation {
            boundary: Boundary::Preparation,
            disposition: if result.is_ok() {
                Disposition::Completed
            } else {
                Disposition::Refused
            },
            service_time: None,
        });
        result
    }

    pub fn decode(bytes: &[u8], max_text_bytes: usize) -> Result<Self, SourceError> {
        let (bytes, bom) = match bytes.strip_prefix(BOM) {
            Some(text) => (text, true),
            None => (bytes, false),
        };
        if bytes.len() > max_text_bytes {
            return Err(SourceError::SourceLimit);
        }
        let decoded = std::str::from_utf8(bytes).map_err(|_| SourceError::InvalidUtf8)?;
        let mut text = String::new();
        text.try_reserve_exact(decoded.len())
            .map_err(|_| SourceError::Capacity)?;
        text.push_str(decoded);
        Ok(Self { text, bom })
    }
    /// Engine-managed byte edge only; completion is not document publication.
    pub(crate) fn decode_observed(
        bytes: &[u8],
        max_text_bytes: usize,
        r: &mut crate::instrumentation::Recorder,
    ) -> Result<Self, SourceError> {
        let result = Self::decode(bytes, max_text_bytes);
        observe_codec(r, crate::instrumentation::Boundary::SourceDecode, &result);
        result
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn initial_bom(&self) -> bool {
        self.bom
    }
    pub(crate) fn encoded_len_with_bom(&self, bom: bool) -> Option<usize> {
        self.text.len().checked_add(if bom { BOM.len() } else { 0 })
    }

    pub fn encode(&self) -> Result<Vec<u8>, SourceError> {
        self.encode_with_bom(self.bom)
    }
    /// Coordinator output intent is separate from this standalone value's decode hint.
    pub(crate) fn encode_with_bom(&self, bom: bool) -> Result<Vec<u8>, SourceError> {
        if !bom && self.text.starts_with('\u{feff}') {
            return Err(SourceError::AmbiguousBom);
        }
        let length = self
            .text
            .len()
            .checked_add(if bom { BOM.len() } else { 0 })
            .ok_or(SourceError::Capacity)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| SourceError::Capacity)?;
        if bom {
            bytes.extend_from_slice(BOM);
        }
        bytes.extend_from_slice(self.text.as_bytes());
        Ok(bytes)
    }
    /// Engine-managed capture/comparison edge, not effect evidence or host I/O.
    pub(crate) fn encode_with_bom_observed(
        &self,
        bom: bool,
        r: &mut crate::instrumentation::Recorder,
    ) -> Result<Vec<u8>, SourceError> {
        let result = self.encode_with_bom(bom);
        observe_codec(r, crate::instrumentation::Boundary::SourceEncode, &result);
        result
    }

    /// Checks every record, then builds one candidate using original coordinates.
    /// Source equality here does not classify a whole-engine semantic no-op.
    pub fn prepare(&self, edits: &[Replacement], limits: Limits) -> Result<Self, SourceError> {
        self.prepare_with_staging_limit(edits, limits, usize::MAX)
    }

    /// Coordinator staging bound is checked before allocating the candidate text.
    pub(crate) fn prepare_with_staging_limit(
        &self,
        edits: &[Replacement],
        limits: Limits,
        staging_limit: usize,
    ) -> Result<Self, SourceError> {
        self.prepare_records(edits, limits, staging_limit, false)
    }

    /// Only owned history journals may supply explicit resolved record order.
    /// Coincident inverse insertions follow their original forward-record lineage;
    /// this is not a public ambiguity bypass for caller-provided change sets.
    pub(crate) fn prepare_journal(
        &self,
        edits: &[Replacement],
        limits: Limits,
        staging_limit: usize,
    ) -> Result<Self, SourceError> {
        self.prepare_records(edits, limits, staging_limit, true)
    }

    fn prepare_records(
        &self,
        edits: &[Replacement],
        limits: Limits,
        staging_limit: usize,
        journal: bool,
    ) -> Result<Self, SourceError> {
        if self.text.len() > limits.max_source_bytes {
            return Err(SourceError::SourceLimit);
        }
        if edits.len() > limits.max_edits {
            return Err(SourceError::EditLimit);
        }
        let mut payload = 0usize;
        let mut removed = 0usize;
        let mut inserted = 0usize;
        for edit in edits {
            payload = payload
                .checked_add(edit.expected.len())
                .and_then(|n| n.checked_add(edit.inserted.len()))
                .filter(|n| *n <= limits.max_edit_payload_bytes)
                .ok_or(SourceError::PayloadLimit)?;
            if edit.start > edit.end || edit.end > self.text.len() {
                return Err(SourceError::InvalidRange);
            }
            if !self.text.is_char_boundary(edit.start) || !self.text.is_char_boundary(edit.end) {
                return Err(SourceError::InvalidBoundary);
            }
            if self.text[edit.start..edit.end] != edit.expected {
                return Err(SourceError::ExpectedMismatch);
            }
            removed = removed
                .checked_add(edit.end - edit.start)
                .ok_or(SourceError::Capacity)?;
            inserted = inserted
                .checked_add(edit.inserted.len())
                .ok_or(SourceError::Capacity)?;
        }
        let mut ordered = Vec::new();
        ordered
            .try_reserve_exact(edits.len())
            .map_err(|_| SourceError::Capacity)?;
        ordered.extend(edits.iter());
        if !journal {
            ordered.sort_unstable_by_key(|edit| (edit.start, edit.end));
        }
        for pair in ordered.windows(2) {
            let (left, right) = (pair[0], pair[1]);
            if if journal {
                right.start < left.end
            } else {
                edits_ambiguous(left, right)
            } {
                return Err(SourceError::AmbiguousEdits);
            }
        }
        let length = self
            .text
            .len()
            .checked_sub(removed)
            .and_then(|n| n.checked_add(inserted))
            .filter(|n| *n <= limits.max_source_bytes)
            .ok_or(SourceError::SourceLimit)?;
        if length > staging_limit {
            return Err(SourceError::StagingLimit);
        }
        let mut text = String::new();
        text.try_reserve_exact(length)
            .map_err(|_| SourceError::Capacity)?;
        let mut cursor = 0;
        for edit in ordered {
            text.push_str(&self.text[cursor..edit.start]);
            text.push_str(&edit.inserted);
            cursor = edit.end;
        }
        text.push_str(&self.text[cursor..]);
        Ok(Self {
            text,
            bom: self.bom,
        })
    }
}
