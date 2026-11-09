//! Exact-snapshot caller restrictions. These are neither grants nor standing policy.
use super::*;

pub const SOURCE_SCOPE_PROFILE_V1: &str = "bounded-source-scope/1";

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct InsertionSiteRange {
    pub first: usize,
    pub last: usize,
}
impl std::fmt::Debug for InsertionSiteRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InsertionSiteRange(<redacted>)")
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct SourceOperationScope {
    pub expected: DocumentVersion,
    pub replacement_ranges: Vec<ByteRange>,
    pub insertion_sites: Vec<InsertionSiteRange>,
}
impl std::fmt::Debug for SourceOperationScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceOperationScope(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceScopeBudget {
    pub max_scopes: usize,
    /// Aggregate range and insertion-site records, not a per-document allowance.
    pub max_records: usize,
    /// Validation, ordering and coverage steps; not measured CPU time.
    pub max_work: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceScopeError {
    InvalidScope,
    InvalidBoundary,
    ParticipantMismatch,
    OutOfScope,
    Limit,
    Capacity,
}
impl From<SourceScopeError> for CoordinatorError {
    fn from(value: SourceScopeError) -> Self {
        Self::Scope(value)
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct ScopeCapture {
    pub(super) scopes: Vec<SourceOperationScope>,
    pub(super) budget: SourceScopeBudget,
}
fn step(work: &mut usize) -> Result<(), SourceScopeError> {
    *work = work.checked_sub(1).ok_or(SourceScopeError::Limit)?;
    Ok(())
}
fn sort<T, K: Ord>(
    rows: &mut [T],
    key: impl Fn(&T) -> K,
    work: &mut usize,
) -> Result<(), SourceScopeError> {
    for i in 1..rows.len() {
        let mut j = i;
        while j > 0 {
            step(work)?;
            if key(&rows[j - 1]) <= key(&rows[j]) {
                break;
            }
            rows.swap(j - 1, j);
            j -= 1;
        }
    }
    Ok(())
}
impl ScopeCapture {
    pub(super) fn copy(
        scopes: &[SourceOperationScope],
        budget: SourceScopeBudget,
    ) -> Result<Self, SourceScopeError> {
        if scopes.len() > budget.max_scopes {
            return Err(SourceScopeError::Limit);
        }
        let mut records = 0usize;
        for scope in scopes {
            records = records
                .checked_add(scope.replacement_ranges.len())
                .and_then(|n| n.checked_add(scope.insertion_sites.len()))
                .filter(|n| *n <= budget.max_records)
                .ok_or(SourceScopeError::Limit)?;
        }
        let mut work = budget.max_work;
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(scopes.len())
            .map_err(|_| SourceScopeError::Capacity)?;
        for scope in scopes {
            step(&mut work)?;
            let mut ranges = Vec::new();
            let mut sites = Vec::new();
            ranges
                .try_reserve_exact(scope.replacement_ranges.len())
                .map_err(|_| SourceScopeError::Capacity)?;
            sites
                .try_reserve_exact(scope.insertion_sites.len())
                .map_err(|_| SourceScopeError::Capacity)?;
            ranges.extend_from_slice(&scope.replacement_ranges);
            sites.extend_from_slice(&scope.insertion_sites);
            sort(&mut ranges, |r| (r.start, r.end), &mut work)?;
            sort(&mut sites, |s| (s.first, s.last), &mut work)?;
            owned.push(SourceOperationScope {
                expected: scope.expected.clone(),
                replacement_ranges: ranges,
                insertion_sites: sites,
            });
        }
        sort(&mut owned, |s| s.expected.document.slot, &mut work)?;
        Ok(Self {
            scopes: owned,
            budget,
        })
    }
    pub(super) fn retained_bytes(scopes: &[SourceOperationScope]) -> Option<usize> {
        let mut n = std::mem::size_of::<Self>().checked_add(
            scopes
                .len()
                .checked_mul(std::mem::size_of::<SourceOperationScope>())?,
        )?;
        for scope in scopes {
            n = n
                .checked_add(
                    scope
                        .replacement_ranges
                        .len()
                        .checked_mul(std::mem::size_of::<ByteRange>())?,
                )?
                .checked_add(
                    scope
                        .insertion_sites
                        .len()
                        .checked_mul(std::mem::size_of::<InsertionSiteRange>())?,
                )?;
        }
        Some(n)
    }
}
fn coverage(
    source: &Utf8Source,
    scope: &SourceOperationScope,
    edits: &[Replacement],
    work: &mut usize,
) -> Result<(), SourceScopeError> {
    let text = source.text();
    let mut previous = None;
    for range in &scope.replacement_ranges {
        step(work)?;
        if range.start >= range.end || previous.is_some_and(|end| range.start < end) {
            return Err(SourceScopeError::InvalidScope);
        }
        if !text.is_char_boundary(range.start) || !text.is_char_boundary(range.end) {
            return Err(SourceScopeError::InvalidBoundary);
        }
        previous = Some(range.end);
    }
    let mut previous = None;
    for site in &scope.insertion_sites {
        step(work)?;
        if site.first > site.last || previous.is_some_and(|last| site.first <= last) {
            return Err(SourceScopeError::InvalidScope);
        }
        if !text.is_char_boundary(site.first) || !text.is_char_boundary(site.last) {
            return Err(SourceScopeError::InvalidBoundary);
        }
        previous = Some(site.last);
    }
    for edit in edits {
        step(work)?;
        // Ordinary preparation still validates spelling, UTF-8 and batch ambiguity.
        if edit.start > edit.end {
            return Err(SourceScopeError::InvalidScope);
        }
        if !text.is_char_boundary(edit.start) || !text.is_char_boundary(edit.end) {
            return Err(SourceScopeError::InvalidBoundary);
        }
        if edit.start == edit.end {
            let mut found = false;
            for site in &scope.insertion_sites {
                step(work)?;
                if site.first <= edit.start && edit.start <= site.last {
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(SourceScopeError::OutOfScope);
            }
        } else {
            let mut covered = edit.start;
            for range in &scope.replacement_ranges {
                step(work)?;
                if range.end <= covered {
                    continue;
                }
                if range.start > covered {
                    break;
                }
                covered = range.end;
                if covered >= edit.end {
                    break;
                }
            }
            if covered < edit.end {
                return Err(SourceScopeError::OutOfScope);
            }
        }
    }
    Ok(())
}
impl SourceCoordinator {
    pub(super) fn check_scoped_source_for_invocation(
        &self,
        source: &SourceIntent,
        capture: &ScopeCapture,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let result = self.check_scopes(
            capture,
            source.writes.len(),
            source
                .writes
                .iter()
                .map(|w| (&w.expected, w.edits.as_slice())),
        );
        observe(r, Boundary::SourceScopePreparation, &result);
        result
    }
    fn check_scopes<'a>(
        &self,
        capture: &ScopeCapture,
        count: usize,
        participants: impl Iterator<Item = (&'a DocumentVersion, &'a [Replacement])> + Clone,
    ) -> Result<(), CoordinatorError> {
        if capture.scopes.len() > capture.budget.max_scopes
            || capture.scopes.len() > self.limits.max_participants
        {
            return Err(SourceScopeError::Limit.into());
        }
        let mut records = 0usize;
        for scope in &capture.scopes {
            records = records
                .checked_add(scope.replacement_ranges.len())
                .and_then(|n| n.checked_add(scope.insertion_sites.len()))
                .filter(|n| *n <= capture.budget.max_records)
                .ok_or(SourceScopeError::Limit)?;
        }
        if capture.scopes.len() != count {
            return Err(SourceScopeError::ParticipantMismatch.into());
        }
        let mut work = capture.budget.max_work;
        for (i, scope) in capture.scopes.iter().enumerate() {
            step(&mut work)?;
            if i > 0 && capture.scopes[i - 1].expected.document.slot >= scope.expected.document.slot
            {
                return Err(SourceScopeError::InvalidScope.into());
            }
            let document = self.validate_version(&scope.expected)?;
            let mut found = None;
            for (version, edits) in participants.clone() {
                step(&mut work)?;
                if version.document == scope.expected.document {
                    if found.is_some() {
                        return Err(SourceScopeError::ParticipantMismatch.into());
                    }
                    if *version != scope.expected {
                        return Err(CoordinatorError::Stale);
                    }
                    found = Some(edits);
                }
            }
            coverage(
                &document.source,
                scope,
                found.ok_or(SourceScopeError::ParticipantMismatch)?,
                &mut work,
            )?;
        }
        Ok(())
    }
    pub(super) fn capture_scopes(
        &self,
        writes: &[DocumentEdits],
        scopes: &[SourceOperationScope],
        budget: SourceScopeBudget,
    ) -> Result<ScopeCapture, CoordinatorError> {
        self.ensure_operational()?;
        if scopes.len() > self.limits.max_participants {
            return Err(CoordinatorError::Limit);
        }
        let capture = ScopeCapture::copy(scopes, budget)?;
        self.check_scopes(
            &capture,
            writes.len(),
            writes.iter().map(|w| (&w.expected, w.edits.as_slice())),
        )?;
        Ok(capture)
    }
    /// Explicit restrictions for this one original-snapshot source batch only.
    pub fn prepare_scoped(
        &self,
        writes: &[DocumentEdits],
        reads: &[DocumentVersion],
        scopes: &[SourceOperationScope],
        budget: SourceScopeBudget,
        r: &mut Recorder,
    ) -> Result<PreparedTransaction, CoordinatorError> {
        let capture = self.capture_scopes(writes, scopes, budget);
        observe(r, Boundary::SourceScopePreparation, &capture);
        let capture = capture?;
        let mut prepared = self.prepare(writes, reads, r)?;
        prepared.scoped = true;
        prepared.scopes = Some(capture);
        Ok(prepared)
    }
    /// Annotation participation remains independently validated and authorized.
    pub fn prepare_scoped_compound(
        &mut self,
        intent: &CompoundIntent,
        scopes: &[SourceOperationScope],
        budget: SourceScopeBudget,
        r: &mut Recorder,
    ) -> Result<PreparedTransaction, CoordinatorError> {
        let capture = self.capture_scopes(&intent.source.writes, scopes, budget);
        observe(r, Boundary::SourceScopePreparation, &capture);
        let capture = capture?;
        let mut prepared = self.prepare_compound(
            &intent.source.writes,
            &intent.source.reads,
            &intent.annotations,
            r,
        )?;
        prepared.scoped = true;
        prepared.scopes = Some(capture);
        Ok(prepared)
    }
    pub(super) fn validate_prepared_scopes(
        &self,
        prepared: &PreparedTransaction,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let result = match (prepared.scoped, &prepared.scopes) {
            (false, None) => return Ok(()),
            (true, Some(capture)) => self.check_scopes(
                capture,
                prepared.candidates.len(),
                prepared
                    .candidates
                    .iter()
                    .map(|c| (&c.expected, c.edits.as_slice())),
            ),
            _ => Err(CoordinatorError::Faulted),
        };
        observe(r, Boundary::SourceScopeValidation, &result);
        result
    }
}
