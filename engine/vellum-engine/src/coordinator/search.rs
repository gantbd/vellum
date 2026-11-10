//! Exact UTF-8 literal-search/1 over the trusted local source owner. This is not
//! a recipient-safe read context, language service, renderer or async scheduler.
use super::*;
mod shutdown_inventory;
use std::sync::Weak;

#[derive(Clone, Copy, Debug)]
pub struct SearchLimits {
    pub max_sessions_issued: usize,
    pub max_live_sessions: usize,
    /// Includes results held by occurrences, preparations and invocation intents.
    pub max_live_results: usize,
    pub max_root_bytes: usize,
    pub max_scope_ranges: usize,
    pub max_query_bytes: usize,
    pub max_comparisons: usize,
    pub max_matches: usize,
    pub max_match_bytes: usize,
    /// Sum of all expected + inserted bytes, not just the replacement spelling.
    pub max_replacement_bytes: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}
#[derive(Clone, PartialEq, Eq)]
pub enum SearchScope {
    Document,
    Captured(Vec<ByteRange>),
}
impl std::fmt::Debug for SearchScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SearchScope(<redacted>)")
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct SearchRequest {
    pub query: String,
    pub scope: SearchScope,
}
impl std::fmt::Debug for SearchRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SearchRequest(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchBudget {
    Comparisons,
    Matches,
    ReplyBytes,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchStatus {
    Complete,
    Exhausted(SearchBudget),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchError {
    Unavailable,
    Stale,
    Superseded,
    Faulted,
    InvalidScope,
    InvalidBoundary,
    InvalidStart,
    MatchUnavailable,
    Incomplete,
    Limit,
    Capacity,
    VersionExhausted,
}
#[derive(Clone)]
pub struct SearchSession {
    namespace: Arc<()>,
    slot: usize,
}
impl std::fmt::Debug for SearchSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SearchSession(<opaque>)")
    }
}
struct ResultData {
    session: SearchSession,
    generation: u64,
    expected: DocumentVersion,
    query: String,
    scopes: Vec<ByteRange>,
    matches: Vec<ByteRange>,
    status: SearchStatus,
}
/// Owned trusted-host disclosure, not a revocable recipient-facing projection.
/// Old bytes/ranges do not vanish on supersession; engine actions revalidate.
#[derive(Clone)]
pub struct SearchResult {
    inner: Arc<ResultData>,
}
impl std::fmt::Debug for SearchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SearchResult(<redacted>)")
    }
}
impl PartialEq for SearchResult {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}
impl Eq for SearchResult {}
impl SearchResult {
    pub fn status(&self) -> SearchStatus {
        self.inner.status
    }
    pub fn ranges(&self) -> &[ByteRange] {
        &self.inner.matches
    }
    pub fn exact_total(&self) -> Option<usize> {
        (self.inner.status == SearchStatus::Complete).then_some(self.inner.matches.len())
    }
    pub fn generation(&self) -> u64 {
        self.inner.generation
    }
    pub fn version(&self) -> &DocumentVersion {
        &self.inner.expected
    }
    pub fn match_id(&self, index: usize) -> Option<SearchMatch> {
        self.inner.matches.get(index).map(|_| SearchMatch {
            result: self.clone(),
            index,
        })
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct SearchMatch {
    result: SearchResult,
    index: usize,
}
impl std::fmt::Debug for SearchMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SearchMatch(<opaque>)")
    }
}
impl SearchMatch {
    pub(super) fn retained_bytes(&self) -> Option<usize> {
        SearchSelection::Current(self.clone()).retained_bytes()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchSelection {
    All(SearchResult),
    Current(SearchMatch),
}
impl SearchSelection {
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.result().inner.expected.document == *document
    }
    fn result(&self) -> &SearchResult {
        match self {
            Self::All(result) => result,
            Self::Current(occurrence) => &occurrence.result,
        }
    }
    pub(super) fn retained_bytes(&self) -> Option<usize> {
        let data = &self.result().inner;
        std::mem::size_of::<ResultData>()
            .checked_add(data.query.len())?
            .checked_add(
                data.scopes
                    .len()
                    .checked_mul(std::mem::size_of::<ByteRange>())?,
            )?
            .checked_add(
                data.matches
                    .len()
                    .checked_mul(std::mem::size_of::<ByteRange>())?,
            )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchDirection {
    Next,
    Previous,
}
#[derive(Clone, Copy, Debug)]
pub struct SearchNavigation<'a> {
    pub direction: SearchDirection,
    pub start: usize,
    pub current: Option<&'a SearchMatch>,
    pub wrap: bool,
}
#[derive(Clone, Debug)]
pub struct SearchFound {
    pub occurrence: SearchMatch,
    pub range: ByteRange,
    pub wrapped: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchUsage {
    pub issued_lifetime: usize,
    pub live_sessions: usize,
    pub live_results: usize,
    pub max_sessions_issued: usize,
    pub max_live_sessions: usize,
    pub max_live_results: usize,
}
#[derive(Debug)]
struct Session {
    document: DocumentId,
    generation: u64,
}
#[derive(Debug, Default)]
pub(super) struct SearchState {
    pub(super) limits: Option<SearchLimits>,
    sessions: Vec<Option<Session>>,
    results: Vec<Weak<ResultData>>,
}
impl SearchState {
    pub(super) fn document_pins(&self, document: &DocumentId) -> (usize, usize) {
        let Self {
            limits: _,
            sessions,
            results,
        } = self;
        (
            sessions
                .iter()
                .flatten()
                .filter(|s| s.document == *document)
                .count(),
            results
                .iter()
                .filter_map(Weak::upgrade)
                .filter(|r| r.expected.document == *document)
                .count(),
        )
    }
}
fn owner_error(error: CoordinatorError) -> SearchError {
    match error {
        CoordinatorError::Unavailable => SearchError::Unavailable,
        CoordinatorError::Stale => SearchError::Stale,
        CoordinatorError::VersionExhausted => SearchError::VersionExhausted,
        _ => SearchError::Faulted,
    }
}
fn capture<T>(recorder: &mut Recorder, boundary: Boundary, result: &Result<T, SearchError>) {
    recorder.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(SearchError::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time: None,
    });
}
fn copy_text(text: &str) -> Result<String, SearchError> {
    let mut owned = String::new();
    owned
        .try_reserve_exact(text.len())
        .map_err(|_| SearchError::Capacity)?;
    owned.push_str(text);
    Ok(owned)
}
pub(super) fn scopes(
    source: &str,
    requested: &SearchScope,
    limit: usize,
) -> Result<Vec<ByteRange>, SearchError> {
    let whole = [ByteRange {
        start: 0,
        end: source.len(),
    }];
    let input = match requested {
        SearchScope::Document => &whole[..],
        SearchScope::Captured(ranges) => &ranges[..],
    };
    if input.len() > limit {
        return Err(SearchError::Limit);
    }
    let mut ranges = Vec::new();
    ranges
        .try_reserve_exact(input.len())
        .map_err(|_| SearchError::Capacity)?;
    for &range in input {
        if range.start > range.end || range.end > source.len() {
            return Err(SearchError::InvalidScope);
        }
        if !source.is_char_boundary(range.start) || !source.is_char_boundary(range.end) {
            return Err(SearchError::InvalidBoundary);
        }
        if range.start != range.end {
            ranges.push(range);
        }
    }
    ranges.sort_unstable_by_key(|r| (r.start, r.end));
    let mut count = 0;
    for index in 0..ranges.len() {
        let range = ranges[index];
        if count > 0 && range.start < ranges[count - 1].end {
            ranges[count - 1].end = ranges[count - 1].end.max(range.end);
        } else {
            ranges[count] = range;
            count += 1;
        }
    }
    ranges.truncate(count);
    Ok(ranges)
}
pub(super) fn evaluate(
    source: &str,
    query: &str,
    scopes: &[ByteRange],
    limits: SearchLimits,
) -> Result<(Vec<ByteRange>, SearchStatus), SearchError> {
    let mut matches = Vec::new();
    if query.is_empty() {
        return Ok((matches, SearchStatus::Complete));
    }
    let mut comparisons = 0usize;
    for scope in scopes {
        if query.len() > scope.end - scope.start {
            continue;
        }
        let last = scope.end - query.len();
        let mut start = scope.start;
        while start <= last {
            // Traversal is bounded by root bytes even at non-scalar positions;
            // comparisons are separately bounded across every scope.
            if !source.is_char_boundary(start) {
                start += 1;
                continue;
            }
            let mut equal = true;
            for (offset, expected) in query.bytes().enumerate() {
                if comparisons == limits.max_comparisons {
                    return Ok((matches, SearchStatus::Exhausted(SearchBudget::Comparisons)));
                }
                comparisons += 1;
                if source.as_bytes()[start + offset] != expected {
                    equal = false;
                    break;
                }
            }
            if equal {
                if matches.len() == limits.max_matches {
                    return Ok((matches, SearchStatus::Exhausted(SearchBudget::Matches)));
                }
                if matches.len() >= limits.max_match_bytes / std::mem::size_of::<ByteRange>() {
                    return Ok((matches, SearchStatus::Exhausted(SearchBudget::ReplyBytes)));
                }
                matches.try_reserve(1).map_err(|_| SearchError::Capacity)?;
                matches.push(ByteRange {
                    start,
                    end: start + query.len(),
                });
                start += query.len();
            } else {
                start += 1;
            }
        }
    }
    Ok((matches, SearchStatus::Complete))
}
impl SourceCoordinator {
    pub fn search_usage(&self) -> SearchUsage {
        SearchUsage {
            issued_lifetime: self.searches.sessions.len(),
            live_sessions: self.searches.sessions.iter().flatten().count(),
            live_results: self
                .searches
                .results
                .iter()
                .filter(|r| r.strong_count() > 0)
                .count(),
            max_sessions_issued: self.searches.limits.map_or(0, |l| l.max_sessions_issued),
            max_live_sessions: self.searches.limits.map_or(0, |l| l.max_live_sessions),
            max_live_results: self.searches.limits.map_or(0, |l| l.max_live_results),
        }
    }
    fn search_session(&self, session: &SearchSession) -> Result<&Session, SearchError> {
        if !Arc::ptr_eq(&self.namespace, &session.namespace) {
            return Err(SearchError::Unavailable);
        }
        self.searches
            .sessions
            .get(session.slot)
            .and_then(Option::as_ref)
            .ok_or(SearchError::Unavailable)
    }
    pub fn open_search(
        &mut self,
        document: &DocumentId,
        recorder: &mut Recorder,
    ) -> Result<SearchSession, SearchError> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            self.document(document).map_err(owner_error)?;
            let limits = self.searches.limits.ok_or(SearchError::Unavailable)?;
            if self.searches.sessions.len() >= limits.max_sessions_issued
                || self.search_usage().live_sessions >= limits.max_live_sessions
            {
                return Err(SearchError::Limit);
            }
            self.searches
                .sessions
                .try_reserve(1)
                .map_err(|_| SearchError::Capacity)?;
            let session = SearchSession {
                namespace: self.namespace.clone(),
                slot: self.searches.sessions.len(),
            };
            self.searches.sessions.push(Some(Session {
                document: document.clone(),
                generation: 0,
            }));
            Ok(session)
        })();
        capture(recorder, Boundary::SearchControl, &result);
        result
    }
    pub fn close_search(
        &mut self,
        session: &SearchSession,
        recorder: &mut Recorder,
    ) -> Result<(), SearchError> {
        let result = self.search_session(session).map(|_| ()).map(|()| {
            self.searches.sessions[session.slot] = None;
        });
        capture(recorder, Boundary::SearchControl, &result);
        result
    }
    pub fn search(
        &mut self,
        session: &SearchSession,
        expected: &DocumentVersion,
        request: &SearchRequest,
        recorder: &mut Recorder,
    ) -> Result<SearchResult, SearchError> {
        let result = self.search_inner(session, expected, request);
        if result
            .as_ref()
            .is_ok_and(|r| r.status() != SearchStatus::Complete)
        {
            recorder.record(Observation {
                boundary: Boundary::SearchEvaluation,
                disposition: Disposition::Partial,
                service_time: None,
            });
        } else {
            capture(recorder, Boundary::SearchEvaluation, &result);
        }
        result
    }
    fn search_inner(
        &mut self,
        session: &SearchSession,
        expected: &DocumentVersion,
        request: &SearchRequest,
    ) -> Result<SearchResult, SearchError> {
        self.ensure_operational().map_err(owner_error)?;
        let current = self.search_session(session)?;
        if current.document != expected.document {
            return Err(SearchError::Unavailable);
        }
        let generation = current
            .generation
            .checked_add(1)
            .ok_or(SearchError::VersionExhausted)?;
        let limits = self.searches.limits.ok_or(SearchError::Unavailable)?;
        let source = self
            .validate_version(expected)
            .map_err(owner_error)?
            .source
            .text();
        if source.len() > limits.max_root_bytes || request.query.len() > limits.max_query_bytes {
            return Err(SearchError::Limit);
        }
        let slot = self
            .searches
            .results
            .iter()
            .position(|r| r.strong_count() == 0)
            .unwrap_or(self.searches.results.len());
        if slot >= limits.max_live_results {
            return Err(SearchError::Limit);
        }
        let scopes = scopes(source, &request.scope, limits.max_scope_ranges)?;
        let query = copy_text(&request.query)?;
        let (matches, status) = evaluate(source, &query, &scopes, limits)?;
        let inner = Arc::new(ResultData {
            session: session.clone(),
            generation,
            expected: expected.clone(),
            query,
            scopes,
            matches,
            status,
        });
        if slot == self.searches.results.len() {
            self.searches
                .results
                .try_reserve(1)
                .map_err(|_| SearchError::Capacity)?;
            self.searches.results.push(Arc::downgrade(&inner));
        } else {
            self.searches.results[slot] = Arc::downgrade(&inner);
        }
        self.searches.sessions[session.slot]
            .as_mut()
            .expect("validated search session")
            .generation = generation;
        Ok(SearchResult { inner })
    }
    // No health check here: final source publication has already latched its
    // uncertainty fence. Callers outside that protected path check health first.
    pub(super) fn validate_search(&self, result: &SearchResult) -> Result<(), SearchError> {
        self.validate_search_context(result)?;
        if result.inner.status != SearchStatus::Complete {
            return Err(SearchError::Incomplete);
        }
        Ok(())
    }
    fn validate_search_context(&self, result: &SearchResult) -> Result<(), SearchError> {
        let data = &result.inner;
        let current = self.search_session(&data.session)?;
        if current.document != data.expected.document {
            return Err(SearchError::Faulted);
        }
        if current.generation != data.generation {
            return Err(SearchError::Superseded);
        }
        if !self
            .searches
            .results
            .iter()
            .any(|r| r.ptr_eq(&Arc::downgrade(&result.inner)))
        {
            return Err(SearchError::Unavailable);
        }
        self.validate_version(&data.expected).map_err(owner_error)?;
        Ok(())
    }
    pub(super) fn resolve_search_occurrence(
        &self,
        occurrence: &SearchMatch,
        source: &DocumentVersion,
    ) -> Result<ByteRange, SearchError> {
        self.validate_search_context(&occurrence.result)?;
        let data = &occurrence.result.inner;
        if data.expected != *source {
            return Err(SearchError::Stale);
        }
        let range = *data
            .matches
            .get(occurrence.index)
            .ok_or(SearchError::MatchUnavailable)?;
        let current = &self.validate_version(source).map_err(owner_error)?.source;
        if data.query.is_empty()
            || range.start >= range.end
            || current.text().get(range.start..range.end) != Some(data.query.as_str())
        {
            return Err(SearchError::Faulted);
        }
        Ok(range)
    }
    pub(super) fn validate_search_selection(
        &self,
        selected: &SearchSelection,
    ) -> Result<(), SearchError> {
        match selected {
            SearchSelection::All(result) => self.validate_search(result),
            SearchSelection::Current(occurrence) => self
                .resolve_search_occurrence(occurrence, &occurrence.result.inner.expected)
                .map(|_| ()),
        }
    }
    pub fn navigate_search(
        &self,
        result: &SearchResult,
        navigation: SearchNavigation<'_>,
        recorder: &mut Recorder,
    ) -> Result<Option<SearchFound>, SearchError> {
        let found = (|| {
            self.ensure_operational().map_err(owner_error)?;
            self.validate_search(result)?;
            let data = &result.inner;
            let source = self
                .validate_version(&data.expected)
                .map_err(owner_error)?
                .source
                .text();
            if navigation.start > source.len() || !source.is_char_boundary(navigation.start) {
                return Err(SearchError::InvalidStart);
            }
            let current = if let Some(current) = navigation.current {
                if current.result != *result || current.index >= data.matches.len() {
                    return Err(SearchError::MatchUnavailable);
                }
                Some(current.index)
            } else {
                None
            };
            let Some((index, wrapped)) = navigate_ranges(
                &data.matches,
                navigation.direction,
                navigation.start,
                current,
                navigation.wrap,
            ) else {
                return Ok(None);
            };
            Ok(Some(SearchFound {
                occurrence: SearchMatch {
                    result: result.clone(),
                    index,
                },
                range: data.matches[index],
                wrapped,
            }))
        })();
        capture(recorder, Boundary::SearchNavigation, &found);
        found
    }
    pub fn prepare_search_replace(
        &self,
        selected: &SearchSelection,
        replacement: &str,
        recorder: &mut Recorder,
    ) -> Result<PreparedTransaction, CoordinatorError> {
        let result = self.prepare_search_replace_inner(selected, replacement);
        super::observe(recorder, Boundary::SearchReplacement, &result);
        result
    }
    pub(super) fn prepare_search_replace_inner(
        &self,
        selected: &SearchSelection,
        replacement: &str,
    ) -> Result<PreparedTransaction, CoordinatorError> {
        self.ensure_operational()?;
        let result = selected.result();
        self.validate_search_selection(selected)
            .map_err(CoordinatorError::Search)?;
        let data = &result.inner;
        let ranges = match selected {
            SearchSelection::All(_) => &data.matches[..],
            SearchSelection::Current(occurrence) => data
                .matches
                .get(occurrence.index)
                .map(std::slice::from_ref)
                .ok_or(CoordinatorError::Search(SearchError::MatchUnavailable))?,
        };
        let limit = self
            .searches
            .limits
            .ok_or(CoordinatorError::Search(SearchError::Unavailable))?;
        let bytes = data
            .query
            .len()
            .checked_add(replacement.len())
            .and_then(|n| n.checked_mul(ranges.len()))
            .ok_or(CoordinatorError::Search(SearchError::Limit))?;
        if bytes > limit.max_replacement_bytes || ranges.len() > self.limits.source.max_edits {
            return Err(CoordinatorError::Search(SearchError::Limit));
        }
        let mut edits = Vec::new();
        edits
            .try_reserve_exact(ranges.len())
            .map_err(|_| CoordinatorError::Capacity)?;
        for range in ranges {
            edits.push(Replacement {
                start: range.start,
                end: range.end,
                expected: copy_text(&data.query).map_err(CoordinatorError::Search)?,
                inserted: copy_text(replacement).map_err(CoordinatorError::Search)?,
            });
        }
        let mut prepared = self.prepare_inner(
            &[DocumentEdits {
                expected: data.expected.clone(),
                edits,
            }],
            &[],
        )?;
        prepared.search_dependency = Some(selected.clone());
        Ok(prepared)
    }
}

/// Shared finite navigation; callers validate original identity/index and scalar start.
pub(super) fn navigate_ranges(
    matches: &[ByteRange],
    direction: SearchDirection,
    start: usize,
    current: Option<usize>,
    wrap: bool,
) -> Option<(usize, bool)> {
    let position = match (current, direction) {
        (Some(index), SearchDirection::Next) => index.checked_add(1).filter(|i| *i < matches.len()),
        (Some(index), SearchDirection::Previous) => index.checked_sub(1),
        (None, SearchDirection::Next) => matches.iter().position(|r| r.start >= start),
        (None, SearchDirection::Previous) => matches.iter().rposition(|r| r.start <= start),
    };
    match position {
        Some(index) => Some((index, false)),
        None if wrap && !matches.is_empty() => Some((
            match direction {
                SearchDirection::Next => 0,
                SearchDirection::Previous => matches.len() - 1,
            },
            true,
        )),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exhausted_generation_refuses_without_superseding_the_original_result() {
        let mut owner = SourceCoordinator::new(CoordinatorLimits {
            max_document_incarnations: 1,
            max_participants: 1,
            max_staged_bytes: 16,
            source: Limits {
                max_source_bytes: 16,
                max_edits: 4,
                max_edit_payload_bytes: 16,
            },
        });
        owner.searches.limits = Some(SearchLimits {
            max_sessions_issued: 1,
            max_live_sessions: 1,
            max_live_results: 2,
            max_root_bytes: 16,
            max_scope_ranges: 1,
            max_query_bytes: 16,
            max_comparisons: 16,
            max_matches: 4,
            max_match_bytes: 64,
            max_replacement_bytes: 16,
        });
        let mut recorder = Recorder::disabled();
        let version = owner.create(b"a", &mut recorder).unwrap();
        let session = owner.open_search(&version.document, &mut recorder).unwrap();
        owner.searches.sessions[session.slot]
            .as_mut()
            .unwrap()
            .generation = u64::MAX - 1;
        let request = SearchRequest {
            query: "a".into(),
            scope: SearchScope::Document,
        };
        let result = owner
            .search(&session, &version, &request, &mut recorder)
            .unwrap();
        assert_eq!(
            owner
                .search(&session, &version, &request, &mut recorder)
                .unwrap_err(),
            SearchError::VersionExhausted
        );
        assert!(owner.validate_search(&result).is_ok());
        owner.faulted = true;
        assert_eq!(
            owner
                .search(&session, &version, &request, &mut recorder)
                .unwrap_err(),
            SearchError::Faulted
        );
        // Protected local release remains possible after authored integrity loss.
        assert!(owner.close_search(&session, &mut recorder).is_ok());
    }
}
