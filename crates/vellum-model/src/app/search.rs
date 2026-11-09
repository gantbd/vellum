//! UI search ownership over original native sessions/results, not a second scan.
use super::*;
use vellum_engine::coordinator::{
    SearchDirection, SearchError, SearchMatch, SearchNavigation, SearchRequest, SearchResult,
    SearchScope, SearchSelection, SearchSession,
};

pub(super) struct State {
    session: SearchSession,
    result: Option<SearchResult>,
    current: Option<SearchMatch>,
}
impl Editor {
    /// Select ranges from this original complete result. Never rerun a query,
    /// reinterpret partial totals or allocate an unbounded selection list.
    pub fn select_matches(&mut self) -> Result<(), EditorError> {
        let result = (|| {
            self.live()?;
            let state = self.tabs[self.active]
                .search
                .as_ref()
                .ok_or(SessionError::Search(SearchError::Unavailable))?;
            let result = state
                .result
                .as_ref()
                .ok_or(SessionError::Search(SearchError::Unavailable))?;
            // Original native owner validates session/supersession and source
            // before this host interprets ranges. This does not move UI state.
            self.host.navigate_search(
                result,
                SearchNavigation {
                    direction: SearchDirection::Next,
                    start: 0,
                    current: None,
                    wrap: true,
                },
            )?;
            if result.status() != vellum_engine::coordinator::SearchStatus::Complete {
                return Err(SessionError::Search(SearchError::Incomplete).into());
            }
            if result.ranges().is_empty() {
                return Err(SessionError::Search(SearchError::MatchUnavailable).into());
            }
            let budget = self.selection_budget.ok_or(SessionError::Selection(
                vellum_engine::selection::SelectionError::Unavailable,
            ))?;
            if result.ranges().len() > budget.max_input_selections {
                return Err(SessionError::Selection(
                    vellum_engine::selection::SelectionError::Limit,
                )
                .into());
            }
            let mut values = Vec::new();
            values
                .try_reserve_exact(result.ranges().len())
                .map_err(|_| EditorError::Capacity)?;
            for range in result.ranges() {
                values.push(vellum_engine::selection::Selection {
                    anchor: vellum_engine::coordinates::BytePosition(range.start),
                    head: vellum_engine::coordinates::BytePosition(range.end),
                    affinity: vellum_engine::attachment::Affinity::After,
                });
            }
            let version = result.version().clone();
            let snapshot = self.native_selection()?.ok_or(SessionError::Selection(
                vellum_engine::selection::SelectionError::Unavailable,
            ))?;
            let context = snapshot.context().ok_or(SessionError::Selection(
                vellum_engine::selection::SelectionError::Unavailable,
            ))?;
            if context.source != version {
                return Err(SessionError::Search(SearchError::Stale).into());
            }
            self.host.set_logical_selections(&context, &values, 0)?;
            self.sync_native_selection()?;
            self.status = format!(
                "{} native selections from complete literal matches",
                self.selection_count()
            );
            Ok(())
        })();
        self.collect_diagnostics();
        result
    }
    /// An explicit new query captures the current source. Navigation/replacement
    /// below never silently issue a new query after a source change.
    pub fn find(&mut self, query: &str) -> Result<(), EditorError> {
        let result = (|| {
            self.live()?;
            let limits = self
                .search_limits
                .ok_or(SessionError::Search(SearchError::Unavailable))?;
            if query.len() > limits.max_query_bytes {
                return Err(SessionError::Search(SearchError::Limit).into());
            }
            let mut owned = String::new();
            owned
                .try_reserve_exact(query.len())
                .map_err(|_| EditorError::Capacity)?;
            owned.push_str(query);
            let tab = &mut self.tabs[self.active];
            if tab.search.is_none() {
                let session = self.host.open_search(&tab.document)?;
                tab.search = Some(State {
                    session,
                    result: None,
                    current: None,
                });
            }
            let state = tab.search.as_mut().expect("installed search owner");
            let version = self.host.read(&tab.document)?.version;
            let found = self.host.search(
                &state.session,
                &version,
                &SearchRequest {
                    query: owned,
                    scope: SearchScope::Document,
                },
            )?;
            let status = found.status();
            let count = found.ranges().len();
            state.result = Some(found);
            state.current = None;
            if status == vellum_engine::coordinator::SearchStatus::Complete {
                self.navigate_match(SearchDirection::Next, true)?;
            }
            self.status = format!("Literal search: {status:?}; {count} retained matches");
            Ok(())
        })();
        self.collect_diagnostics();
        result
    }
    pub fn navigate_match(
        &mut self,
        direction: SearchDirection,
        wrap: bool,
    ) -> Result<(), EditorError> {
        let mut moved = false;
        let result = (|| {
            self.live()?;
            let tab = &mut self.tabs[self.active];
            let state = tab
                .search
                .as_mut()
                .ok_or(SessionError::Search(SearchError::Unavailable))?;
            let result = state
                .result
                .as_ref()
                .ok_or(SessionError::Search(SearchError::Unavailable))?;
            let found = self.host.navigate_search(
                result,
                SearchNavigation {
                    direction,
                    start: tab.cursor,
                    current: state.current.as_ref(),
                    wrap,
                },
            )?;
            if let Some(found) = found {
                moved = true;
                tab.cursor = found.range.start;
                tab.anchor = None;
                state.current = Some(found.occurrence);
                self.status = if found.wrapped {
                    "Search match (wrapped)"
                } else {
                    "Search match"
                }
                .into();
            } else {
                self.status = "No further retained search match".into();
            }
            Ok(())
        })();
        let result = result.and_then(|()| {
            if moved {
                self.publish_ui_selection()
            } else {
                Ok(())
            }
        });
        self.collect_diagnostics();
        result
    }
    /// Replace the original current match or explicitly all complete results.
    /// The native publisher enforces freshness, edit policy, budgets and history.
    pub fn replace_match(&mut self, replacement: &str, all: bool) -> Result<(), EditorError> {
        let result = (|| {
            self.live()?;
            self.groups
                .try_reserve(1)
                .map_err(|_| EditorError::Capacity)?;
            let state = self.tabs[self.active]
                .search
                .as_ref()
                .ok_or(SessionError::Search(SearchError::Unavailable))?;
            let selected = if all {
                SearchSelection::All(
                    state
                        .result
                        .as_ref()
                        .ok_or(SessionError::Search(SearchError::Unavailable))?
                        .clone(),
                )
            } else {
                SearchSelection::Current(
                    state
                        .current
                        .as_ref()
                        .ok_or(SessionError::Search(SearchError::MatchUnavailable))?
                        .clone(),
                )
            };
            let outcome = self.host.replace_search(&selected, replacement)?;
            if let Some(group) = outcome.history_group {
                self.groups.push(group);
            }
            self.clamp_cursor()?;
            self.status = "Replacement published; search again for current matches".into();
            Ok(())
        })();
        self.collect_diagnostics();
        result
    }
    pub fn clear_search(&mut self) -> Result<(), EditorError> {
        self.live()?;
        if let Some(state) = &self.tabs[self.active].search {
            self.host.close_search(&state.session)?;
        }
        self.tabs[self.active].search = None;
        self.status = "Search closed".into();
        self.collect_diagnostics();
        Ok(())
    }
    pub(super) fn clear_all_searches(&mut self) -> Result<(), EditorError> {
        for tab in &mut self.tabs {
            if let Some(state) = &tab.search {
                self.host.close_search(&state.session)?;
            }
            tab.search = None;
        }
        Ok(())
    }
}
