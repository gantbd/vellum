//! Privileged fixed-port validation bridge, not recipient authority or a new verb.
use super::*;
impl SourceCoordinator {
    #[cfg(test)]
    pub(in crate::coordinator) fn corrupt_search_facade_reply(
        &self,
        reply: &mut SearchFacadeReplyV1,
    ) {
        match reply {
            SearchFacadeReplyV1::Evaluated(id) => id.slot = id.slot.saturating_add(1),
            SearchFacadeReplyV1::Released(v) => v.result.slot = v.result.slot.saturating_add(1),
            SearchFacadeReplyV1::Occurrence(v) => v.index = v.index.saturating_add(1),
            SearchFacadeReplyV1::Navigation(v) => {
                if let Some(found) = v.found.as_mut() {
                    found.wrapped = !found.wrapped;
                } else {
                    v.version.revision = v.version.revision.saturating_add(1);
                }
            }
        }
    }
    pub(in crate::coordinator) fn managed_search_domain(&self) -> Result<(&str, &str), Error> {
        let p = self.search_pool()?;
        Ok((&p.authority, &p.engine))
    }
    pub(in crate::coordinator) fn managed_search_result_matches(
        &self,
        id: &SourceSearchResultId,
        source: SearchReadContextV1<'_>,
        s: &SourceReadScopeV1,
        q: &SearchRequest,
    ) -> bool {
        self.search_row(id)
            .ok()
            .zip(self.own_search_issuer(source).ok())
            .is_some_and(|(row, source)| {
                row.source == source && row.scope == *s && row.request == *q && !row.stopped
            })
    }
    pub(in crate::coordinator) fn managed_search_release_product(
        &self,
        id: &SourceSearchResultId,
        reply: &SourceSearchReplyV1,
    ) -> bool {
        self.search_row(id)
            .is_ok_and(|row| result_contract::release_valid(row, id, reply))
    }
    pub(in crate::coordinator) fn managed_search_occurrence_product(
        &self,
        id: &SourceSearchResultId,
        index: usize,
        reply: &SourceSearchOccurrenceId,
    ) -> bool {
        self.search_row(id).is_ok_and(|row| {
            index < row.matches.len() && result_contract::occurrence_valid(id, index, reply)
        })
    }
    pub(in crate::coordinator) fn managed_search_navigation_product(
        &self,
        id: &SourceSearchResultId,
        reply: &SourceSearchNavigationReplyV1,
    ) -> bool {
        self.search_row(id).is_ok_and(|row| {
            reply.result == *id
                && reply.version == row.version
                && reply.found.as_ref().is_none_or(|f| {
                    f.occurrence.result == *id
                        && row.matches.get(f.occurrence.index) == Some(&f.range)
                })
        })
    }
}
pub(in crate::coordinator) fn navigation_signature(
    reply: &SourceSearchNavigationReplyV1,
) -> Option<(usize, ByteRange, bool)> {
    reply
        .found
        .as_ref()
        .map(|f| (f.occurrence.index, f.range, f.wrapped))
}
