use super::*;
use crate::{attachment::Affinity, selection::SelectionError};

/// Existing F11/F12 semantic operations, not a new runtime or UI workflow.
pub struct TrustedSourceSearchConsumerFactsV1<'a> {
    pub scope: &'a SourceReadScopeV1,
    pub command: &'a SourceSearchConsumerV1,
    /// Independently attributed authorization for this exact F11/F12 mutation.
    pub mutation_allowed: Option<bool>,
    pub resources_ready: Option<bool>,
}
#[derive(PartialEq, Eq)]
pub enum SourceSearchConsumerV1 {
    ReplaceAll {
        result: SourceSearchResultId,
        replacement: String,
    },
    ReplaceCurrent {
        occurrence: SourceSearchOccurrenceId,
        replacement: String,
    },
    SelectMatch {
        occurrence: SourceSearchOccurrenceId,
        view: LogicalViewContext,
        direction: SelectionDirectionV1,
        affinity: Affinity,
        mode: SearchSelectionModeV1,
    },
}
impl std::fmt::Debug for SourceSearchConsumerV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceSearchConsumerV1(<redacted>)")
    }
}
impl SourceSearchConsumerV1 {
    /// Passive descriptor construction only. The owning capture validates the
    /// actual returned result and occurrence before any native preparation.
    pub(in crate::coordinator) fn replacement_value(
        result: &SourceSearchResultId,
        index: Option<usize>,
        text: &str,
    ) -> Result<Self, Error> {
        let replacement = owned(text)?;
        Ok(match index {
            None => Self::ReplaceAll {
                result: result.clone(),
                replacement,
            },
            Some(index) => Self::ReplaceCurrent {
                occurrence: SourceSearchOccurrenceId {
                    result: result.clone(),
                    index,
                },
                replacement,
            },
        })
    }
    pub(in crate::coordinator) fn result(&self) -> &SourceSearchResultId {
        match self {
            Self::ReplaceAll { result, .. } => result,
            Self::ReplaceCurrent { occurrence, .. } | Self::SelectMatch { occurrence, .. } => {
                &occurrence.result
            }
        }
    }
    pub(in crate::coordinator) fn index(&self) -> Option<usize> {
        match self {
            Self::ReplaceAll { .. } => None,
            Self::ReplaceCurrent { occurrence, .. } | Self::SelectMatch { occurrence, .. } => {
                Some(occurrence.index)
            }
        }
    }
    fn bytes(&self) -> Result<usize, Error> {
        std::mem::size_of::<Self>()
            .checked_add(match self {
                Self::ReplaceAll { replacement, .. } | Self::ReplaceCurrent { replacement, .. } => {
                    replacement.len()
                }
                _ => 0,
            })
            .ok_or(Error::Limit)
    }
    fn copy(&self) -> Result<Self, Error> {
        Ok(match self {
            Self::ReplaceAll {
                result,
                replacement,
            } => Self::ReplaceAll {
                result: result.clone(),
                replacement: owned(replacement)?,
            },
            Self::ReplaceCurrent {
                occurrence,
                replacement,
            } => Self::ReplaceCurrent {
                occurrence: occurrence.clone(),
                replacement: owned(replacement)?,
            },
            Self::SelectMatch {
                occurrence,
                view,
                direction,
                affinity,
                mode,
            } => Self::SelectMatch {
                occurrence: occurrence.clone(),
                view: view.clone(),
                direction: *direction,
                affinity: *affinity,
                mode: *mode,
            },
        })
    }
}
#[derive(Debug)]
// Fixed inline native plans share the already bounded invocation staging slot;
// no extra heap indirection or independently escaping publisher is needed.
#[allow(clippy::large_enum_variant)]
pub(in crate::coordinator) enum ConsumerPlan {
    Source(Result<PreparedTransaction, CoordinatorError>),
    Selection {
        context: LogicalViewContext,
        command: SelectionCommandV1,
        prepared: Result<views::PreparedSelectionCommand, SelectionError>,
    },
}
pub(in crate::coordinator) struct ConsumerCapture {
    issuer: Issuer,
    /// Immutable attribution only; no extra source lease or copied member content.
    peer_scopes: Vec<SourceReadScopeV1>,
    pub owner: Arc<()>,
    pub scope: SourceReadScopeV1,
    pub request: SearchRequest,
    pub command: SourceSearchConsumerV1,
    version: DocumentVersion,
    ranges: Vec<ByteRange>,
    selection: Option<SelectionCommandV1>,
}
impl std::fmt::Debug for ConsumerCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ConsumerCapture(<redacted>)")
    }
}
impl ConsumerCapture {
    /// Complete retained replacement capture and source candidate projection.
    /// History storage and query/issuer leases keep their existing owner bounds.
    pub(in crate::coordinator) fn replacement_envelope_bytes(
        &self,
        plan: &ConsumerPlan,
    ) -> Result<usize, Error> {
        use std::mem::size_of;
        let ConsumerPlan::Source(Ok(p)) = plan else {
            return Err(Error::Unavailable);
        };
        if !self.intact(plan) {
            return Err(Error::Faulted);
        }
        let mut n = request_bytes(&self.scope, &self.request)?;
        let mut add = |v: usize| {
            n = n.checked_add(v).ok_or(Error::Limit)?;
            Ok::<_, Error>(())
        };
        add(size_of::<Self>())?;
        add(size_of::<ConsumerPlan>())?;
        add(self.command.bytes()?)?;
        for scope in &self.peer_scopes {
            add(scope_bytes(scope)?)?;
        }
        add(self
            .ranges
            .len()
            .checked_mul(size_of::<ByteRange>())
            .ok_or(Error::Limit)?)?;
        for candidate in &p.candidates {
            add(size_of::<Candidate>())?;
            add(candidate.source.text().len())?;
            for edit in &candidate.edits {
                add(size_of::<Replacement>())?;
                add(edit.expected.len())?;
                add(edit.inserted.len())?;
            }
        }
        for view in &p.view_dependencies {
            add(size_of::<views::ViewDependency>())?;
            add(view
                .selection_count()
                .checked_mul(size_of::<crate::selection::Selection>())
                .ok_or(Error::Limit)?)?;
        }
        // Source-only replacement has no authored annotation plan or derived
        // operation. Do not silently admit new unaccounted preparation fields.
        if !p.annotations.is_empty() || p.configured_required || p.configured_line.is_some() {
            return Err(Error::Faulted);
        }
        Ok(n)
    }
    pub(in crate::coordinator) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.scope.document == *document
            || self.version.document == *document
            || self.peer_scopes.iter().any(|s| s.document == *document)
            || match &self.command {
                SourceSearchConsumerV1::SelectMatch { view, .. } => {
                    view.source.document == *document
                }
                SourceSearchConsumerV1::ReplaceAll { .. }
                | SourceSearchConsumerV1::ReplaceCurrent { .. } => false,
            }
    }
    pub(in crate::coordinator) fn read_matches(&self, f: SearchReadFactsV1<'_>) -> bool {
        self.issuer.matches(f)
            && match f {
                SearchReadFactsV1::VersionSet(f) => {
                    f.parent.members.len() == self.peer_scopes.len()
                        && f.parent
                            .members
                            .iter()
                            .zip(&self.peer_scopes)
                            .all(|(f, s)| f.scope == s)
                }
                _ => self.peer_scopes.is_empty(),
            }
    }
    pub(in crate::coordinator) fn intact(&self, plan: &ConsumerPlan) -> bool {
        match (plan, &self.command) {
            (
                ConsumerPlan::Source(Err(_)),
                SourceSearchConsumerV1::ReplaceAll { .. }
                | SourceSearchConsumerV1::ReplaceCurrent { .. },
            ) => true,
            (
                ConsumerPlan::Source(Ok(p)),
                SourceSearchConsumerV1::ReplaceAll { replacement, .. }
                | SourceSearchConsumerV1::ReplaceCurrent { replacement, .. },
            ) => {
                Arc::ptr_eq(&p.namespace, &self.owner)
                    && p.candidates.len() == 1
                    && p.candidates[0].expected == self.version
                    && p.candidates[0].edits.len() == self.ranges.len()
                    && p.candidates[0]
                        .edits
                        .iter()
                        .zip(&self.ranges)
                        .all(|(e, r)| {
                            e.start == r.start
                                && e.end == r.end
                                && e.expected == self.request.query
                                && e.inserted == *replacement
                        })
                    && p.read_dependencies.is_empty()
                    && !p.scoped
                    && p.scopes.is_none()
                    && !p.reads_required
                    && p.declared_reads.is_none()
                    && p.derived_kind.is_none()
                    && p.derived_edit.is_empty()
                    && p.search_dependency.is_none()
            }
            (
                ConsumerPlan::Selection {
                    context,
                    command,
                    prepared,
                },
                SourceSearchConsumerV1::SelectMatch { view, .. },
            ) => {
                context == view
                    && self.selection.as_ref() == Some(command)
                    && matches!(command, SelectionCommandV1::Set { .. })
                    && prepared
                        .as_ref()
                        .map_or(true, |p| p.matches(context, command))
            }
            _ => false,
        }
    }
}
impl SourceCoordinator {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::coordinator) fn capture_source_search_consumer(
        &mut self,
        command: &SourceSearchConsumerV1,
        s: &SourceReadScopeV1,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        cf: &TrustedSourceSearchConsumerFactsV1<'_>,
        limit: usize,
        r: &mut Recorder,
    ) -> Result<(Arc<ConsumerCapture>, ConsumerPlan), Error> {
        let row = self.search_row(command.result())?;
        if row.scope != *s
            || row.request != *f.request
            || sf.scope() != s
            || !row.source.matches(sf)
            || cf.scope != s
            || cf.command != command
        {
            return Err(Error::Unavailable);
        }
        if command.index().is_some_and(|i| i >= row.matches.len()) {
            return Err(Error::InvalidRequest);
        }
        if let SourceSearchConsumerV1::SelectMatch { view, .. } = command
            && view.source != row.version
        {
            return Err(Error::Unavailable);
        }
        let bytes = self.check_search_request(&row.source, s, f.request, sf, command.bytes()?)?;
        if bytes > limit {
            return Err(Error::Limit);
        }
        self.check_source_search_release(command.result(), s, sf, f, r)?;
        if cf.mutation_allowed != Some(true) || cf.resources_ready != Some(true) {
            return Err(Error::Unavailable);
        }
        let row = self.search_row(command.result())?;
        if matches!(command, SourceSearchConsumerV1::ReplaceAll { .. })
            && row.status != SearchStatus::Complete
        {
            return Err(Error::Unavailable);
        }
        let selected = match command.index() {
            Some(i) => &row.matches[i..i + 1],
            None => &row.matches[..],
        };
        let capture_bytes = bytes
            .checked_add(std::mem::size_of::<ConsumerCapture>())
            .and_then(|n| n.checked_add(std::mem::size_of::<ConsumerPlan>()))
            .and_then(|n| {
                n.checked_add(
                    selected
                        .len()
                        .checked_mul(std::mem::size_of::<ByteRange>())?,
                )
            })
            .ok_or(Error::Limit)?;
        if capture_bytes > limit {
            return Err(Error::Limit);
        }
        // The complete peer scopes are already charged by the full native request
        // envelope above. Preserve attribution even after protected parent retirement.
        let mut peer_scopes = Vec::new();
        if let SearchReadFactsV1::VersionSet(sf) = sf {
            peer_scopes
                .try_reserve_exact(sf.parent.members.len())
                .map_err(|_| Error::Capacity)?;
            for f in sf.parent.members {
                peer_scopes.push(SourceReadScopeV1 {
                    document: f.scope.document.clone(),
                    context: settings::basis::copy_context(&f.scope.context)
                        .map_err(|_| Error::Capacity)?,
                    policy: owned(&f.scope.policy)?,
                });
            }
        }
        let mut capture = Arc::new(ConsumerCapture {
            issuer: row.source.clone(),
            peer_scopes,
            owner: self.namespace.clone(),
            scope: SourceReadScopeV1 {
                document: s.document.clone(),
                context: settings::basis::copy_context(&s.context).map_err(|_| Error::Capacity)?,
                policy: owned(&s.policy)?,
            },
            request: SearchRequest {
                query: owned(&f.request.query)?,
                scope: match &f.request.scope {
                    SearchScope::Document => SearchScope::Document,
                    SearchScope::Captured(v) => SearchScope::Captured(copy_ranges(v)?),
                },
            },
            command: command.copy()?,
            version: row.version.clone(),
            ranges: copy_ranges(selected)?,
            selection: None,
        });
        let plan = match &capture.command {
            SourceSearchConsumerV1::ReplaceAll { replacement, .. }
            | SourceSearchConsumerV1::ReplaceCurrent { replacement, .. } => {
                let bytes = capture
                    .request
                    .query
                    .len()
                    .checked_add(replacement.len())
                    .and_then(|n| n.checked_mul(capture.ranges.len()))
                    .ok_or(Error::Limit)?;
                if bytes
                    > self
                        .limits
                        .source
                        .max_edit_payload_bytes
                        .min(self.search_pool()?.limits.max_replacement_bytes)
                    || capture.ranges.len() > self.limits.source.max_edits
                {
                    return Err(Error::Limit);
                }
                let mut edits = Vec::new();
                edits
                    .try_reserve_exact(capture.ranges.len())
                    .map_err(|_| Error::Capacity)?;
                for range in &capture.ranges {
                    edits.push(Replacement {
                        start: range.start,
                        end: range.end,
                        expected: owned(&capture.request.query)?,
                        inserted: owned(replacement)?,
                    });
                }
                ConsumerPlan::Source(self.prepare(
                    &[DocumentEdits {
                        expected: capture.version.clone(),
                        edits,
                    }],
                    &[],
                    r,
                ))
            }
            SourceSearchConsumerV1::SelectMatch {
                view,
                direction,
                affinity,
                mode,
                ..
            } => {
                let (command, prepared) = self
                    .prepare_managed_search_selection(
                        view,
                        capture.ranges[0],
                        *direction,
                        *affinity,
                        *mode,
                        r,
                    )
                    .map_err(|e| match e {
                        SelectionError::Faulted => Error::Faulted,
                        SelectionError::Capacity => Error::Capacity,
                        SelectionError::Limit => Error::Limit,
                        _ => Error::Unavailable,
                    })?;
                if capture_bytes
                    .checked_add(
                        command
                            .retained_bytes()
                            .ok_or(Error::Limit)?
                            .checked_mul(3)
                            .ok_or(Error::Limit)?,
                    )
                    .is_none_or(|n| n > limit)
                {
                    return Err(Error::Limit);
                }
                let stored = command.copy().map_err(|_| Error::Capacity)?;
                let context = view.clone();
                Arc::get_mut(&mut capture).ok_or(Error::Faulted)?.selection = Some(stored);
                ConsumerPlan::Selection {
                    context,
                    command,
                    prepared: Ok(prepared),
                }
            }
        };
        if matches!(
            &plan,
            ConsumerPlan::Source(Err(
                CoordinatorError::Faulted | CoordinatorError::History(HistoryError::Faulted)
            ))
        ) {
            return Err(Error::Faulted);
        }
        Ok((capture, plan))
    }
    pub(in crate::coordinator) fn validate_source_search_consumer(
        &mut self,
        c: &ConsumerCapture,
        s: &SourceReadScopeV1,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        if c.scope != *s || c.request != *f.request || sf.scope() != s {
            return Err(Error::InvalidRequest);
        }
        self.check_source_search_release(c.command.result(), s, sf, f, r)
    }
}
