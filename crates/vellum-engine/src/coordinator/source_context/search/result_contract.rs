//! Native search product shape/identity guards, not another matching evaluator.
use super::*;
pub(super) fn candidate_valid(
    text: &str,
    query: &str,
    scopes: &[ByteRange],
    matches: &[ByteRange],
    status: SearchStatus,
    max_matches: usize,
    max_match_bytes: usize,
) -> bool {
    if matches.len() > max_matches
        || matches.len() > max_match_bytes / std::mem::size_of::<ByteRange>()
    {
        return false;
    }
    if query.is_empty() {
        return matches.is_empty() && status == SearchStatus::Complete;
    }
    match status {
        SearchStatus::Exhausted(SearchBudget::Matches) if matches.len() != max_matches => {
            return false;
        }
        SearchStatus::Exhausted(SearchBudget::ReplyBytes)
            if matches.len() != max_match_bytes / std::mem::size_of::<ByteRange>()
                || matches.len() >= max_matches =>
        {
            return false;
        }
        _ => {}
    }
    let mut scope = 0usize;
    let mut end = 0usize;
    for (i, m) in matches.iter().enumerate() {
        if m.end.checked_sub(m.start) != Some(query.len())
            || !text.is_char_boundary(m.start)
            || !text.is_char_boundary(m.end)
            || (i > 0 && m.start < end)
        {
            return false;
        }
        while scope < scopes.len() && scopes[scope].end <= m.start {
            scope += 1;
        }
        let Some(s) = scopes.get(scope) else {
            return false;
        };
        if m.start < s.start || m.end > s.end {
            return false;
        }
        end = m.end;
    }
    true
}
impl SourceCoordinator {
    pub(super) fn guard_search_product(
        &mut self,
        valid: bool,
        boundary: Boundary,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        if self.finish_read_result(
            valid,
            ReadResultInvariantV1::NativeSearchResultInvariant,
            boundary,
            r,
        ) {
            Ok(())
        } else {
            Err(Error::Faulted)
        }
    }
}
pub(super) fn release_valid(
    row: &ResultRow,
    id: &SourceSearchResultId,
    reply: &SourceSearchReplyV1,
) -> bool {
    reply.result == *id
        && reply.version == row.version
        && reply.status == row.status
        && reply.matches == row.matches
}
pub(super) fn occurrence_valid(
    id: &SourceSearchResultId,
    index: usize,
    reply: &SourceSearchOccurrenceId,
) -> bool {
    reply.result == *id && reply.index == index
}
pub(super) fn navigation_valid(
    id: &SourceSearchResultId,
    row: &ResultRow,
    expected: Option<(usize, bool)>,
    reply: &SourceSearchNavigationReplyV1,
) -> bool {
    if reply.result != *id || reply.version != row.version {
        return false;
    }
    match (expected, &reply.found) {
        (None, None) => true,
        (Some((index, wrapped)), Some(found)) => {
            occurrence_valid(id, index, &found.occurrence)
                && row.matches.get(index) == Some(&found.range)
                && found.wrapped == wrapped
        }
        _ => false,
    }
}
#[cfg(test)]
pub(in crate::coordinator) mod tests;
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::coordinator) enum Fault {
    ProducerRange,
    ProducerOrder,
    ProducerStatus,
    ReleaseAlias,
    ReleaseVersion,
    ReleaseRanges,
    ReleaseStatus,
    OccurrenceAlias,
    OccurrenceIndex,
    NavigationAlias,
    NavigationVersion,
    NavigationRange,
    NavigationIndex,
    NavigationWrapped,
    NavigationMissing,
}
