use super::*;

/// Bound to one actual retained result, not a range or a caller-selected index.
#[derive(Clone, PartialEq, Eq)]
pub struct SourceSearchOccurrenceId {
    pub(super) result: SourceSearchResultId,
    pub(super) index: usize,
}
impl std::fmt::Debug for SourceSearchOccurrenceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceSearchOccurrenceId(<opaque>)")
    }
}
#[derive(Clone, Copy)]
pub struct SourceSearchNavigationV1<'a> {
    pub direction: SearchDirection,
    pub start: usize,
    pub current: Option<&'a SourceSearchOccurrenceId>,
    pub wrap: bool,
}
pub struct SourceSearchFoundV1 {
    pub occurrence: SourceSearchOccurrenceId,
    pub range: ByteRange,
    pub wrapped: bool,
}
/// Available-but-empty differs from unavailable/incomplete navigation.
pub struct SourceSearchNavigationReplyV1 {
    pub result: SourceSearchResultId,
    pub version: DocumentVersion,
    pub found: Option<SourceSearchFoundV1>,
}
macro_rules! redacted {
    ($name:ident $(<$l:lifetime>)?) => {
        impl std::fmt::Debug for $name $(<$l>)? {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(concat!(stringify!($name), "(<redacted>)"))
            }
        }
    };
}
redacted!(SourceSearchNavigationV1<'_>);
redacted!(SourceSearchFoundV1);
redacted!(SourceSearchNavigationReplyV1);
impl SourceCoordinator {
    /// Owned occurrence from a freshly eligible literal result, including a
    /// retained partial prefix. This is not exact next/previous navigation.
    #[allow(clippy::too_many_arguments)]
    pub fn source_search_occurrence(
        &mut self,
        id: &SourceSearchResultId,
        index: usize,
        s: &SourceReadScopeV1,
        sf: &TrustedSourceReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<SourceSearchOccurrenceId> {
        self.managed_search_occurrence(id, index, s, SearchReadFactsV1::Source(sf), f, r)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn managed_search_occurrence(
        &mut self,
        id: &SourceSearchResultId,
        index: usize,
        s: &SourceReadScopeV1,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<SourceSearchOccurrenceId> {
        self.managed_search_occurrence_result(id, index, s, sf, f, r)
            .ok()
    }
    #[allow(clippy::too_many_arguments)]
    pub(in crate::coordinator) fn managed_search_occurrence_result(
        &mut self,
        id: &SourceSearchResultId,
        index: usize,
        s: &SourceReadScopeV1,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourceSearchOccurrenceId, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let row = self.search_row(id)?;
            if row.scope != *s
                || row.request != *f.request
                || sf.scope() != s
                || !row.source.matches(sf)
                || index >= row.matches.len()
            {
                return Err(Error::Unavailable);
            }
            self.check_search_request(&row.source, s, f.request, sf, std::mem::size_of::<usize>())?;
            self.check_source_search_release(id, s, sf, f, r)?;
            if std::mem::size_of::<Option<SourceSearchOccurrenceId>>()
                > self.search_pool()?.limits.max_reply_bytes.min(
                    self.search_snapshot(&self.search_row(id)?.source, s)?
                        .max_reply_bytes,
                )
            {
                return Err(Error::Limit);
            }
            #[allow(unused_mut)]
            let mut reply = SourceSearchOccurrenceId {
                result: id.clone(),
                index,
            };
            #[cfg(test)]
            match self.search_result_fault {
                Some(result_contract::Fault::OccurrenceAlias) => {
                    reply.result.slot = reply.result.slot.saturating_add(1)
                }
                Some(result_contract::Fault::OccurrenceIndex) => {
                    reply.index = reply.index.saturating_add(1)
                }
                _ => {}
            }
            self.guard_search_product(
                result_contract::occurrence_valid(id, index, &reply),
                Boundary::SourceSearchOccurrenceResultValidation,
                r,
            )?;
            Ok(reply)
        })();
        self.finish_search(
            Boundary::SearchOccurrenceResolution,
            &result,
            timing.finish(),
            r,
        );
        result
    }
    #[allow(clippy::too_many_arguments)]
    pub fn navigate_source_search(
        &mut self,
        id: &SourceSearchResultId,
        s: &SourceReadScopeV1,
        navigation: SourceSearchNavigationV1<'_>,
        sf: &TrustedSourceReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<SourceSearchNavigationReplyV1> {
        self.navigate_managed_search(id, s, navigation, SearchReadFactsV1::Source(sf), f, r)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn navigate_managed_search(
        &mut self,
        id: &SourceSearchResultId,
        s: &SourceReadScopeV1,
        navigation: SourceSearchNavigationV1<'_>,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<SourceSearchNavigationReplyV1> {
        self.navigate_managed_search_result(id, s, navigation, sf, f, r)
            .ok()
    }
    #[allow(clippy::too_many_arguments)]
    pub(in crate::coordinator) fn navigate_managed_search_result(
        &mut self,
        id: &SourceSearchResultId,
        s: &SourceReadScopeV1,
        navigation: SourceSearchNavigationV1<'_>,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourceSearchNavigationReplyV1, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let row = self.search_row(id)?;
            if row.scope != *s
                || row.request != *f.request
                || s != sf.scope()
                || row.stopped
                || !row.source.matches(sf)
            {
                return Err(Error::Unavailable);
            }
            let snapshot = self.search_snapshot(&row.source, s)?;
            let source = snapshot.source;
            if navigation.start > source.text().len()
                || !source.text().is_char_boundary(navigation.start)
            {
                return Err(Error::InvalidRequest);
            }
            if navigation
                .current
                .is_some_and(|v| v.result != *id || v.index >= row.matches.len())
            {
                return Err(Error::InvalidRequest);
            }
            let tail = std::mem::size_of::<SourceSearchNavigationV1<'_>>()
                .checked_add(if navigation.current.is_some() {
                    std::mem::size_of::<SourceSearchOccurrenceId>()
                } else {
                    0
                })
                .ok_or(Error::Limit)?;
            self.check_search_request(&row.source, s, f.request, sf, tail)?;
            // Complete shape/current-occurrence validation precedes stateful loss.
            // Reply limits and incomplete-result refusals must not conceal loss.
            self.check_source_search_release(id, s, sf, f, r)?;
            let row = self.search_row(id)?;
            if row.status != SearchStatus::Complete {
                return Err(Error::Unavailable);
            }
            if std::mem::size_of::<SourceSearchNavigationReplyV1>()
                > self
                    .search_pool()?
                    .limits
                    .max_reply_bytes
                    .min(self.search_snapshot(&row.source, s)?.max_reply_bytes)
            {
                return Err(Error::Limit);
            }
            let expected = kernel::navigate_ranges(
                &row.matches,
                navigation.direction,
                navigation.start,
                navigation.current.map(|v| v.index),
                navigation.wrap,
            );
            let found = expected.map(|(index, wrapped)| SourceSearchFoundV1 {
                occurrence: SourceSearchOccurrenceId {
                    result: id.clone(),
                    index,
                },
                range: row.matches[index],
                wrapped,
            });
            #[allow(unused_mut)]
            let mut reply = SourceSearchNavigationReplyV1 {
                result: id.clone(),
                version: row.version.clone(),
                found,
            };
            #[cfg(test)]
            match self.search_result_fault {
                Some(result_contract::Fault::NavigationAlias) => {
                    reply.result.slot = reply.result.slot.saturating_add(1)
                }
                Some(result_contract::Fault::NavigationVersion) => {
                    reply.version.revision = reply.version.revision.saturating_add(1)
                }
                Some(result_contract::Fault::NavigationRange) => {
                    if let Some(f) = reply.found.as_mut() {
                        f.range.start = f.range.start.saturating_add(1);
                    }
                }
                Some(result_contract::Fault::NavigationIndex) => {
                    if let Some(f) = reply.found.as_mut() {
                        f.occurrence.index = f.occurrence.index.saturating_add(1);
                    }
                }
                Some(result_contract::Fault::NavigationWrapped) => {
                    if let Some(f) = reply.found.as_mut() {
                        f.wrapped = !f.wrapped;
                    }
                }
                Some(result_contract::Fault::NavigationMissing) => reply.found = None,
                _ => {}
            }
            let valid =
                result_contract::navigation_valid(id, self.search_row(id)?, expected, &reply);
            self.guard_search_product(valid, Boundary::SourceSearchNavigationResultValidation, r)?;
            Ok(reply)
        })();
        self.finish_search(
            Boundary::SourceSearchNavigation,
            &result,
            timing.finish(),
            r,
        );
        result
    }
}
