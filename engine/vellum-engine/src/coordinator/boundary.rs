//! Runtime exact-source evidence ownership, not a segmenter or conformance proof.
//! The host attests Unicode semantics outside editing. Structural acceptance is
//! deliberately not authority, a provider qualification, or a promise of truth.
use super::*;
mod shutdown_inventory;
use crate::coordinates::BytePosition;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryProfileV1 {
    Unicode18Uax29Revision49,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundaryLimits {
    pub max_issued: usize,
    pub max_retained: usize,
    pub max_source_bytes: usize,
    /// Sum of both grids and two endpoints per significant interval.
    pub max_boundaries: usize,
    pub max_words: usize,
    /// Native record/header, source copy and vector payload bytes per row.
    pub max_evidence_bytes: usize,
    /// Sum of live row bytes. Tombstone slots are bounded by max_issued instead.
    pub max_retained_bytes: usize,
}
#[derive(PartialEq, Eq)]
pub struct BoundaryTableV1 {
    pub profile: BoundaryProfileV1,
    pub graphemes: Vec<BytePosition>,
    /// Raw default-word grid, not dictionary output or significant words alone.
    pub words: Vec<BytePosition>,
    pub significant_words: Vec<ByteRange>,
}
impl std::fmt::Debug for BoundaryTableV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BoundaryTableV1(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryError {
    Unavailable,
    Stale,
    InvalidTable,
    Limit,
    Capacity,
    Faulted,
}
#[derive(Clone)]
pub struct BoundaryCaptureId {
    namespace: Arc<()>,
    slot: usize,
}
impl PartialEq for BoundaryCaptureId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.namespace, &other.namespace) && self.slot == other.slot
    }
}
impl Eq for BoundaryCaptureId {}
impl std::fmt::Debug for BoundaryCaptureId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BoundaryCaptureId(<opaque>)")
    }
}
#[derive(Clone)]
pub struct BoundaryEvidenceId {
    namespace: Arc<()>,
    slot: usize,
}
impl PartialEq for BoundaryEvidenceId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.namespace, &other.namespace) && self.slot == other.slot
    }
}
impl Eq for BoundaryEvidenceId {}
impl std::fmt::Debug for BoundaryEvidenceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BoundaryEvidenceId(<opaque>)")
    }
}
pub struct BoundarySourceRead<'a> {
    pub version: DocumentVersion,
    pub profile: BoundaryProfileV1,
    pub text: &'a str,
}
impl std::fmt::Debug for BoundarySourceRead<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BoundarySourceRead(<redacted>)")
    }
}
pub struct BoundaryEvidenceRead<'a> {
    pub version: DocumentVersion,
    pub table: &'a BoundaryTableV1,
}
impl std::fmt::Debug for BoundaryEvidenceRead<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BoundaryEvidenceRead(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BoundaryUsage {
    pub issued_lifetime: usize,
    pub pending: usize,
    pub ready: usize,
    pub retained_bytes: usize,
}
struct Record {
    version: DocumentVersion,
    text: String,
    table: Option<BoundaryTableV1>,
}
impl std::fmt::Debug for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BoundaryRecord(<redacted>)")
    }
}
impl Record {
    fn bytes(&self) -> Option<usize> {
        std::mem::size_of::<Self>()
            .checked_add(self.text.len())?
            .checked_add(self.table.as_ref().map_or(Some(0), table_bytes)?)
    }
}
#[derive(Debug, Default)]
pub(super) struct BoundaryState {
    limits: Option<BoundaryLimits>,
    rows: Vec<Option<Record>>,
}
impl BoundaryState {
    pub(super) fn document_pins(&self, document: &DocumentId) -> (usize, usize) {
        self.rows
            .iter()
            .flatten()
            .filter(|row| row.version.document == *document)
            .fold((0, 0), |(pending, ready), row| match &row.table {
                None => (pending + 1, ready),
                Some(_) => (pending, ready + 1),
            })
    }
}
fn table_bytes(table: &BoundaryTableV1) -> Option<usize> {
    table
        .graphemes
        .len()
        .checked_add(table.words.len())?
        .checked_mul(std::mem::size_of::<BytePosition>())?
        .checked_add(
            table
                .significant_words
                .len()
                .checked_mul(std::mem::size_of::<ByteRange>())?,
        )
}
fn validate_table(
    text: &str,
    table: &BoundaryTableV1,
    limits: BoundaryLimits,
) -> Result<(), BoundaryError> {
    let count = table
        .graphemes
        .len()
        .checked_add(table.words.len())
        .and_then(|n| {
            table
                .significant_words
                .len()
                .checked_mul(2)
                .and_then(|words| n.checked_add(words))
        })
        .ok_or(BoundaryError::Limit)?;
    if count > limits.max_boundaries || table.significant_words.len() > limits.max_words {
        return Err(BoundaryError::Limit);
    }
    match table.profile {
        BoundaryProfileV1::Unicode18Uax29Revision49 => {}
    }
    for grid in [&table.graphemes, &table.words] {
        if grid.first() != Some(&BytePosition(0))
            || grid.last() != Some(&BytePosition(text.len()))
            || grid.iter().any(|p| !text.is_char_boundary(p.0))
            || grid.windows(2).any(|w| w[0].0 >= w[1].0)
        {
            return Err(BoundaryError::InvalidTable);
        }
    }
    let mut previous = 0;
    for word in &table.significant_words {
        if word.start >= word.end || word.start < previous {
            return Err(BoundaryError::InvalidTable);
        }
        let at = table
            .words
            .binary_search_by_key(&word.start, |p| p.0)
            .map_err(|_| BoundaryError::InvalidTable)?;
        if table.words.get(at + 1) != Some(&BytePosition(word.end)) {
            return Err(BoundaryError::InvalidTable);
        }
        previous = word.end;
    }
    Ok(())
}
fn copy_table(table: &BoundaryTableV1) -> Result<BoundaryTableV1, BoundaryError> {
    fn copy<T: Copy>(values: &[T]) -> Result<Vec<T>, BoundaryError> {
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(values.len())
            .map_err(|_| BoundaryError::Capacity)?;
        owned.extend_from_slice(values);
        Ok(owned)
    }
    Ok(BoundaryTableV1 {
        profile: table.profile,
        graphemes: copy(&table.graphemes)?,
        words: copy(&table.words)?,
        significant_words: copy(&table.significant_words)?,
    })
}
fn error(error: CoordinatorError) -> BoundaryError {
    match error {
        CoordinatorError::Faulted => BoundaryError::Faulted,
        CoordinatorError::Stale => BoundaryError::Stale,
        CoordinatorError::Capacity => BoundaryError::Capacity,
        CoordinatorError::Limit => BoundaryError::Limit,
        _ => BoundaryError::Unavailable,
    }
}
fn observe<T>(r: &mut Recorder, boundary: Boundary, result: &Result<T, BoundaryError>) {
    r.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(BoundaryError::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time: None,
    });
}
impl SourceCoordinator {
    fn finish_boundary<T>(
        &mut self,
        r: &mut Recorder,
        boundary: Boundary,
        result: Result<T, BoundaryError>,
    ) -> Result<T, BoundaryError> {
        if matches!(result, Err(BoundaryError::Faulted)) {
            self.faulted = true;
        }
        observe(r, boundary, &result);
        result
    }
    pub fn configure_boundaries(
        &mut self,
        limits: BoundaryLimits,
        r: &mut Recorder,
    ) -> Result<bool, BoundaryError> {
        let result = (|| {
            self.ensure_operational().map_err(error)?;
            if let Some(old) = self.boundaries.limits {
                return if old == limits {
                    Ok(false)
                } else {
                    Err(BoundaryError::Unavailable)
                };
            }
            if !self.boundaries.rows.is_empty() {
                return Err(BoundaryError::Faulted);
            }
            self.boundaries.limits = Some(limits);
            Ok(true)
        })();
        self.finish_boundary(r, Boundary::BoundaryConfiguration, result)
    }
    /// Content-free diagnostic gauges, never authorization or conformance facts.
    pub fn boundary_usage(&self) -> BoundaryUsage {
        let mut usage = BoundaryUsage {
            issued_lifetime: self.boundaries.rows.len(),
            ..BoundaryUsage::default()
        };
        for row in self.boundaries.rows.iter().flatten() {
            if row.table.is_some() {
                usage.ready += 1;
            } else {
                usage.pending += 1;
            }
            usage.retained_bytes = usage
                .retained_bytes
                .saturating_add(row.bytes().unwrap_or(usize::MAX));
        }
        usage
    }
    fn boundary_inventory(&self) -> Result<(BoundaryLimits, usize, usize), BoundaryError> {
        let limits = self
            .boundaries
            .limits
            .ok_or(if self.boundaries.rows.is_empty() {
                BoundaryError::Unavailable
            } else {
                BoundaryError::Faulted
            })?;
        if self.boundaries.rows.len() > limits.max_issued {
            return Err(BoundaryError::Faulted);
        }
        let (mut bytes, mut count) = (0usize, 0usize);
        for row in self.boundaries.rows.iter().flatten() {
            let size = row.bytes().ok_or(BoundaryError::Faulted)?;
            if !Arc::ptr_eq(&row.version.document.namespace, &self.namespace) {
                return Err(BoundaryError::Faulted);
            }
            if row.text.len() > limits.max_source_bytes || size > limits.max_evidence_bytes {
                return Err(BoundaryError::Faulted);
            }
            if let Some(table) = &row.table {
                validate_table(&row.text, table, limits).map_err(|_| BoundaryError::Faulted)?;
            }
            bytes = bytes.checked_add(size).ok_or(BoundaryError::Faulted)?;
            count += 1;
        }
        if bytes > limits.max_retained_bytes || count > limits.max_retained {
            return Err(BoundaryError::Faulted);
        }
        Ok((limits, bytes, count))
    }
    fn boundary_row(
        &self,
        namespace: &Arc<()>,
        slot: usize,
        ready: bool,
    ) -> Result<&Record, BoundaryError> {
        if !Arc::ptr_eq(namespace, &self.namespace) {
            return Err(BoundaryError::Unavailable);
        }
        let row = self
            .boundaries
            .rows
            .get(slot)
            .and_then(Option::as_ref)
            .ok_or(BoundaryError::Unavailable)?;
        if row.table.is_some() != ready {
            return Err(BoundaryError::Unavailable);
        }
        Ok(row)
    }
    fn boundary_current(&self, row: &Record) -> Result<(), BoundaryError> {
        if !Arc::ptr_eq(&row.version.document.namespace, &self.namespace) {
            return Err(BoundaryError::Faulted);
        }
        let current = self.validate_version(&row.version).map_err(error)?;
        if current.source.text() != row.text {
            return Err(BoundaryError::Faulted);
        }
        Ok(())
    }
    /// Reserve the actual canonical source copy before issuing a non-reused ID.
    /// Provider execution happens in the host, outside authoritative publication.
    pub fn begin_boundary_capture(
        &mut self,
        version: &DocumentVersion,
        r: &mut Recorder,
    ) -> Result<BoundaryCaptureId, BoundaryError> {
        let result = (|| {
            self.ensure_operational().map_err(error)?;
            let (limits, held, count) = self.boundary_inventory()?;
            let document = self.validate_version(version).map_err(error)?;
            let text = document.source.text();
            let size = std::mem::size_of::<Record>()
                .checked_add(text.len())
                .ok_or(BoundaryError::Limit)?;
            if self.boundaries.rows.len() >= limits.max_issued
                || count >= limits.max_retained
                || text.len() > limits.max_source_bytes
                || size > limits.max_evidence_bytes
                || held
                    .checked_add(size)
                    .is_none_or(|n| n > limits.max_retained_bytes)
            {
                return Err(BoundaryError::Limit);
            }
            let mut owned = String::new();
            owned
                .try_reserve_exact(text.len())
                .map_err(|_| BoundaryError::Capacity)?;
            owned.push_str(text);
            self.boundaries
                .rows
                .try_reserve(1)
                .map_err(|_| BoundaryError::Capacity)?;
            let id = BoundaryCaptureId {
                namespace: self.namespace.clone(),
                slot: self.boundaries.rows.len(),
            };
            self.boundaries.rows.push(Some(Record {
                version: version.clone(),
                text: owned,
                table: None,
            }));
            Ok(id)
        })();
        self.finish_boundary(r, Boundary::BoundaryCapture, result)
    }
    /// Trusted-host source disclosure for this original capture, not a grant.
    pub fn boundary_source(
        &self,
        id: &BoundaryCaptureId,
        r: &mut Recorder,
    ) -> Result<BoundarySourceRead<'_>, BoundaryError> {
        let result = (|| {
            self.ensure_operational().map_err(error)?;
            self.boundary_inventory()?;
            let row = self.boundary_row(&id.namespace, id.slot, false)?;
            self.boundary_current(row)?;
            Ok(BoundarySourceRead {
                version: row.version.clone(),
                profile: BoundaryProfileV1::Unicode18Uax29Revision49,
                text: &row.text,
            })
        })();
        observe(r, Boundary::BoundaryInput, &result);
        result
    }
    /// Privileged host attestation: ordered/scalar-safe table validation does not
    /// establish UAX rules, significant-word classification or provider truth.
    pub fn accept_boundary_evidence(
        &mut self,
        id: &BoundaryCaptureId,
        table: &BoundaryTableV1,
        r: &mut Recorder,
    ) -> Result<BoundaryEvidenceId, BoundaryError> {
        let result = (|| {
            self.ensure_operational().map_err(error)?;
            let (limits, held, _) = self.boundary_inventory()?;
            let row = self.boundary_row(&id.namespace, id.slot, false)?;
            self.boundary_current(row)?;
            let old = row.bytes().ok_or(BoundaryError::Faulted)?;
            let size = old
                .checked_add(table_bytes(table).ok_or(BoundaryError::Limit)?)
                .ok_or(BoundaryError::Limit)?;
            if size > limits.max_evidence_bytes
                || held
                    .checked_sub(old)
                    .and_then(|n| n.checked_add(size))
                    .is_none_or(|n| n > limits.max_retained_bytes)
            {
                return Err(BoundaryError::Limit);
            }
            validate_table(&row.text, table, limits)?;
            let owned = copy_table(table)?;
            self.boundaries.rows[id.slot]
                .as_mut()
                .expect("validated capture")
                .table = Some(owned);
            Ok(BoundaryEvidenceId {
                namespace: self.namespace.clone(),
                slot: id.slot,
            })
        })();
        self.finish_boundary(r, Boundary::BoundaryRetention, result)
    }
    pub fn boundary_evidence(
        &self,
        id: &BoundaryEvidenceId,
        r: &mut Recorder,
    ) -> Result<BoundaryEvidenceRead<'_>, BoundaryError> {
        let result = (|| {
            self.ensure_operational().map_err(error)?;
            self.resolve_boundary_evidence(id)
        })();
        observe(r, Boundary::BoundaryQuery, &result);
        result
    }
    pub(super) fn resolve_boundary_evidence(
        &self,
        id: &BoundaryEvidenceId,
    ) -> Result<BoundaryEvidenceRead<'_>, BoundaryError> {
        self.boundary_inventory()?;
        let row = self.boundary_row(&id.namespace, id.slot, true)?;
        self.boundary_current(row)?;
        Ok(BoundaryEvidenceRead {
            version: row.version.clone(),
            table: row.table.as_ref().expect("ready row"),
        })
    }
    /// Protected local cleanup, not cancellation or a provider-quiescence claim.
    pub fn retire_boundary_capture(
        &mut self,
        id: &BoundaryCaptureId,
        r: &mut Recorder,
    ) -> Result<(), BoundaryError> {
        let result = self.retire_boundary_row(&id.namespace, id.slot, false);
        self.finish_boundary(r, Boundary::BoundaryControl, result)
    }
    pub fn retire_boundary_evidence(
        &mut self,
        id: &BoundaryEvidenceId,
        r: &mut Recorder,
    ) -> Result<(), BoundaryError> {
        let result = self.retire_boundary_row(&id.namespace, id.slot, true);
        self.finish_boundary(r, Boundary::BoundaryControl, result)
    }
    fn retire_boundary_row(
        &mut self,
        namespace: &Arc<()>,
        slot: usize,
        ready: bool,
    ) -> Result<(), BoundaryError> {
        self.boundary_row(namespace, slot, ready)?;
        self.boundaries.rows[slot] = None;
        Ok(())
    }
}
