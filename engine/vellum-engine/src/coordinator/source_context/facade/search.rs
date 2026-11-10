//! Fixed native managed-search ports. No wire vocabulary, parser or UI workflow.
use super::*;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy)]
pub enum SearchFacadePortV1<'a> {
    Evaluate,
    Release {
        result: &'a SourceSearchResultId,
    },
    Occurrence {
        result: &'a SourceSearchResultId,
        index: usize,
    },
    Navigate {
        result: &'a SourceSearchResultId,
        navigation: SourceSearchNavigationV1<'a>,
    },
}
#[derive(Clone, Copy)]
pub struct SearchFacadeRequestV1<'a> {
    pub source: SearchReadContextV1<'a>,
    pub scope: &'a SourceReadScopeV1,
    pub actor: &'a SettingsExecutionContextV1,
    pub port: SearchFacadePortV1<'a>,
    pub query: &'a SearchRequest,
    pub current: &'a TrustedSourceSearchFactsV1<'a>,
    pub facts: SearchReadFactsV1<'a>,
    pub limits: CaptureLimitsV1,
}
pub enum SearchFacadeReplyV1 {
    Evaluated(SourceSearchResultId),
    Released(SourceSearchReplyV1),
    Occurrence(SourceSearchOccurrenceId),
    Navigation(SourceSearchNavigationReplyV1),
}
macro_rules! redact {($($t:ty),+)=>{$(impl std::fmt::Debug for $t {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.write_str("<native-search-facade:redacted>")}
})+};}
redact!(
    SearchFacadePortV1<'_>,
    SearchFacadeRequestV1<'_>,
    SearchFacadeReplyV1
);
impl SourceCoordinator {
    /// Compatibility projection. Prefer `try_search_facade_v1` when the caller
    /// needs the explicit native refusal/fault classification.
    pub fn search_facade_v1(
        &mut self,
        request: SearchFacadeRequestV1<'_>,
        r: &mut Recorder,
    ) -> Option<SearchFacadeReplyV1> {
        self.try_search_facade_v1(request, r).ok()
    }
    /// Explicit native outcome; errors do not grant access or prove that no
    /// retained result was installed before an owner fault. No wire codec.
    /// `Unavailable` deliberately does not identify a failed authority premise.
    pub fn try_search_facade_v1(
        &mut self,
        request: SearchFacadeRequestV1<'_>,
        r: &mut Recorder,
    ) -> Result<SearchFacadeReplyV1, SourceReadContextErrorV1> {
        lifetime::execute(self, Family::Search, r, |gateway| {
            gateway.dispatch_search(request)
        })
    }
}
fn ranges(q: &SearchRequest) -> &[ByteRange] {
    match &q.scope {
        SearchScope::Document => &[],
        SearchScope::Captured(rows) => rows,
    }
}
fn copy_query(
    q: &SearchRequest,
    flags: Option<&[Option<bool>; 6]>,
    limits: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<SearchRequest, Error> {
    let rows = ranges(q);
    let safe = |n: usize| (n as u128) <= 9_007_199_254_740_991;
    let template = rows
        .len()
        .checked_mul(size_of::<[InertValueV1<'_>; 2]>() + size_of::<InertValueV1<'_>>())
        .and_then(|n| {
            n.checked_add(
                size_of::<Vec<[InertValueV1<'_>; 2]>>() + size_of::<Vec<InertValueV1<'_>>>(),
            )
        })
        .ok_or(Error::Limit)?;
    let fixed = 5 * size_of::<(&str, InertValueV1<'_>)>() + size_of::<[InertValueV1<'_>; 6]>();
    let bytes = size_of::<SearchRequest>()
        .checked_add(q.query.len())
        .and_then(|n| n.checked_add(rows.len().checked_mul(size_of::<ByteRange>())?))
        .and_then(|n| {
            n.checked_add(if flags.is_some() {
                size_of::<[Option<bool>; 6]>() + size_of::<TrustedSourceSearchFactsV1<'_>>()
            } else {
                0
            })
        })
        .ok_or(Error::Limit)?;
    if rows.len() > limits.max_nodes
        || template > limits.max_bytes
        || fixed > limits.max_bytes
        || bytes > limits.max_bytes
    {
        return Err(Error::Limit);
    }
    if rows
        .iter()
        .any(|b| b.start > b.end || !safe(b.start) || !safe(b.end))
    {
        return Err(Error::InvalidRequest);
    }
    let mut pairs = Vec::new();
    pairs
        .try_reserve_exact(rows.len())
        .map_err(|_| Error::Capacity)?;
    for b in rows {
        pairs.push([
            InertValueV1::Number(b.start as f64),
            InertValueV1::Number(b.end as f64),
        ]);
    }
    let mut values = Vec::new();
    values
        .try_reserve_exact(rows.len())
        .map_err(|_| Error::Capacity)?;
    values.extend(pairs.iter().map(|p| InertValueV1::Array(p)));
    let fields = [
        ("query", InertValueV1::Text(&q.query)),
        (
            "scope",
            InertValueV1::Text(if matches!(q.scope, SearchScope::Document) {
                "document"
            } else {
                "captured"
            }),
        ),
        ("ranges", InertValueV1::Array(&values)),
    ];
    let flag_values = flags
        .copied()
        .unwrap_or([None; 6])
        .map(|v| v.map_or(InertValueV1::Null, InertValueV1::Boolean));
    let packet = [
        ("request", InertValueV1::Object(&fields)),
        ("facts", InertValueV1::Array(&flag_values)),
    ];
    let shadow = frame(
        &if flags.is_some() {
            InertValueV1::Object(&packet)
        } else {
            InertValueV1::Object(&fields)
        },
        limits,
        r,
    )?;
    let scope = match &q.scope {
        SearchScope::Document => SearchScope::Document,
        SearchScope::Captured(rows) => {
            let mut copy = Vec::new();
            copy.try_reserve_exact(rows.len())
                .map_err(|_| Error::Capacity)?;
            copy.extend_from_slice(rows);
            SearchScope::Captured(copy)
        }
    };
    let out = SearchRequest {
        query: owned(&q.query)?,
        scope,
    };
    drop(shadow);
    Ok(out)
}
fn capture_port(
    port: SearchFacadePortV1<'_>,
    limits: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<(), Error> {
    let safe = |n: usize| (n as u128) <= 9_007_199_254_740_991;
    let (name, index, start, current, wrap, direction) = match port {
        SearchFacadePortV1::Evaluate => ("evaluate", None, None, false, false, "none"),
        SearchFacadePortV1::Release { .. } => ("release", None, None, false, false, "none"),
        SearchFacadePortV1::Occurrence { index, .. } if safe(index) => {
            ("occurrence", Some(index), None, false, false, "none")
        }
        SearchFacadePortV1::Navigate { navigation: n, .. } if safe(n.start) => (
            "navigate",
            None,
            Some(n.start),
            n.current.is_some(),
            n.wrap,
            match n.direction {
                SearchDirection::Next => "next",
                SearchDirection::Previous => "previous",
            },
        ),
        _ => return Err(Error::InvalidRequest),
    };
    let fields = [
        ("port", InertValueV1::Text(name)),
        ("result", InertValueV1::Null),
        ("context", InertValueV1::Null),
        (
            "index",
            index.map_or(InertValueV1::Null, |n| InertValueV1::Number(n as f64)),
        ),
        (
            "start",
            start.map_or(InertValueV1::Null, |n| InertValueV1::Number(n as f64)),
        ),
        ("current", InertValueV1::Boolean(current)),
        ("wrap", InertValueV1::Boolean(wrap)),
        ("direction", InertValueV1::Text(direction)),
    ];
    if size_of_val(&fields) > limits.max_bytes {
        return Err(Error::Limit);
    }
    frame(&InertValueV1::Object(&fields), limits, r)?;
    Ok(())
}
impl<'a> SearchFacadePortV1<'a> {
    fn result(self) -> Option<&'a SourceSearchResultId> {
        match self {
            Self::Evaluate => None,
            Self::Release { result }
            | Self::Occurrence { result, .. }
            | Self::Navigate { result, .. } => Some(result),
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn run(
        self,
        o: &mut SourceCoordinator,
        source: SearchReadContextV1<'_>,
        s: &SourceReadScopeV1,
        q: &SearchRequest,
        sf: SearchReadFactsV1<'_>,
        current: &TrustedSourceSearchFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SearchFacadeReplyV1, Error> {
        match self {
            Self::Evaluate => o
                .query_managed_search_result(source, s, q, sf, current, r)
                .map(SearchFacadeReplyV1::Evaluated),
            Self::Release { result } => o
                .release_managed_search_result(result, s, sf, current, r)
                .map(SearchFacadeReplyV1::Released),
            Self::Occurrence { result, index } => o
                .managed_search_occurrence_result(result, index, s, sf, current, r)
                .map(SearchFacadeReplyV1::Occurrence),
            Self::Navigate { result, navigation } => o
                .navigate_managed_search_result(result, s, navigation, sf, current, r)
                .map(SearchFacadeReplyV1::Navigation),
        }
    }
}
enum Expected {
    Evaluation(SourceSearchResultId),
    Release,
    Occurrence,
    Navigation(Option<(usize, ByteRange, bool)>),
}
impl Expected {
    fn capture(reply: &SearchFacadeReplyV1) -> Self {
        match reply {
            SearchFacadeReplyV1::Evaluated(id) => Self::Evaluation(id.clone()),
            SearchFacadeReplyV1::Released(_) => Self::Release,
            SearchFacadeReplyV1::Occurrence(_) => Self::Occurrence,
            SearchFacadeReplyV1::Navigation(v) => {
                Self::Navigation(super::super::search::navigation_signature(v))
            }
        }
    }
}
fn valid(
    o: &SourceCoordinator,
    request: SearchFacadeRequestV1<'_>,
    reply: &SearchFacadeReplyV1,
    expected: &Expected,
) -> bool {
    if request.port.result().is_some_and(|id| {
        !o.managed_search_result_matches(id, request.source, request.scope, request.query)
    }) {
        return false;
    }
    match (request.port, reply, expected) {
        (
            SearchFacadePortV1::Evaluate,
            SearchFacadeReplyV1::Evaluated(id),
            Expected::Evaluation(original),
        ) => {
            id == original
                && o.managed_search_result_matches(id, request.source, request.scope, request.query)
        }
        (
            SearchFacadePortV1::Release { result },
            SearchFacadeReplyV1::Released(v),
            Expected::Release,
        ) => o.managed_search_release_product(result, v),
        (
            SearchFacadePortV1::Occurrence { result, index },
            SearchFacadeReplyV1::Occurrence(v),
            Expected::Occurrence,
        ) => o.managed_search_occurrence_product(result, index, v),
        (
            SearchFacadePortV1::Navigate { result, .. },
            SearchFacadeReplyV1::Navigation(v),
            Expected::Navigation(original),
        ) => {
            o.managed_search_navigation_product(result, v)
                && *original == super::super::search::navigation_signature(v)
        }
        _ => false,
    }
}
impl Gateway<'_> {
    fn dispatch_search(
        &mut self,
        request: SearchFacadeRequestV1<'_>,
    ) -> Result<SearchFacadeReplyV1, Error> {
        if let Err(e) = self.admit(request.limits) {
            observation(
                self.recorder,
                Boundary::SearchFacadeAdmission,
                if e == Error::Faulted {
                    Disposition::Unknown
                } else {
                    Disposition::Refused
                },
            );
            return Err(e);
        }
        #[cfg(test)]
        if self.owner.source_facade_fault == Some(Fault::Reentry) {
            assert!(
                self.owner
                    .search_facade_v1(request, self.recorder)
                    .is_none()
            );
        }
        let safe = |n: usize| (n as u128) <= 9_007_199_254_740_991;
        if [
            request.limits.max_bytes,
            request.limits.max_nodes,
            request.limits.max_depth,
        ]
        .iter()
        .any(|n| !safe(*n))
            || request.limits.max_nodes == 0
        {
            return Err(Error::InvalidRequest);
        }
        let limits = CaptureLimitsV1 {
            max_bytes: request
                .limits
                .max_bytes
                .min(self.owner.settings_job_parent_limit()),
            ..request.limits
        };
        if size_of::<Expected>() > limits.max_bytes
            || scope_bytes(request.scope)? > limits.max_bytes
        {
            return Err(Error::Limit);
        }
        let issuer_domain = match request.source {
            SearchReadContextV1::Source(_) => {
                let p = self.owner.source_pool()?;
                (&*p.authority, &*p.engine)
            }
            SearchReadContextV1::Combined(_) => self.owner.annotation_context_domain()?,
            SearchReadContextV1::VersionSet(_) => self.owner.version_context_domain()?,
        };
        if (
            request.actor.authority.as_str(),
            request.actor.engine.as_str(),
        ) != self.owner.managed_search_domain()?
            || (
                request.actor.authority.as_str(),
                request.actor.engine.as_str(),
            ) != issuer_domain
            || labels(request.actor)
                .iter()
                .any(|(_, v)| matches!(v,InertValueV1::Text(s) if s.is_empty()))
        {
            return Err(Error::Unavailable);
        }
        frame(
            &InertValueV1::Object(&labels(request.actor)),
            limits,
            self.recorder,
        )?;
        capture_port(request.port, limits, self.recorder)?;
        if request.port.result().is_some_and(|id| {
            !self.owner.managed_search_result_matches(
                id,
                request.source,
                request.scope,
                request.query,
            )
        }) {
            return Err(Error::Unavailable);
        }
        let [a, b, c, d, e, f, g] = labels(&request.scope.context);
        let scope_fields = [
            a,
            b,
            c,
            d,
            e,
            f,
            g,
            ("policy", InertValueV1::Text(&request.scope.policy)),
            ("document", InertValueV1::Null),
        ];
        if size_of_val(&scope_fields) > limits.max_bytes {
            return Err(Error::Limit);
        }
        frame(&InertValueV1::Object(&scope_fields), limits, self.recorder)?;
        let scope = copy_scope(request.scope)?;
        let query = copy_query(request.query, None, limits, self.recorder)?;
        let flags = [
            request.current.grant,
            request.current.scope,
            request.current.continuity,
            request.current.capacity,
            request.current.retained,
            request.current.not_stopped,
        ];
        let current_query =
            copy_query(request.current.request, Some(&flags), limits, self.recorder)?;
        let [
            grant,
            scope_flag,
            continuity,
            capacity,
            retained,
            not_stopped,
        ] = flags;
        let current = TrustedSourceSearchFactsV1 {
            request: &current_query,
            grant,
            scope: scope_flag,
            continuity,
            capacity,
            retained,
            not_stopped,
        };
        let facts = facts::copy_search(
            facts::Input {
                source: request.source,
                scope: request.scope,
                facts: request.facts,
            },
            limits,
            self.owner,
            self.recorder,
        )?;
        observation(
            self.recorder,
            Boundary::SearchFacadeCapture,
            Disposition::Completed,
        );
        #[cfg(test)]
        assert_ne!(
            self.owner.source_facade_fault,
            Some(Fault::BeforeNative),
            "injected search pre-handoff fault"
        );
        let produced = facts.with_search_facts(request.source, &scope, |sf| {
            request.port.run(
                self.owner,
                request.source,
                &scope,
                &query,
                sf,
                &current,
                self.recorder,
            )
        });
        observation(
            self.recorder,
            Boundary::SearchFacadeNativeHandoff,
            if self.owner.faulted {
                Disposition::Unknown
            } else if produced.is_ok() {
                Disposition::Completed
            } else {
                Disposition::Refused
            },
        );
        #[allow(unused_mut)]
        let mut reply = produced?;
        let expected = Expected::capture(&reply);
        #[cfg(test)]
        {
            assert_ne!(
                self.owner.source_facade_fault,
                Some(Fault::AfterNative),
                "injected search late fault"
            );
            if self.owner.source_facade_fault == Some(Fault::BadResult) {
                self.owner.corrupt_search_facade_reply(&mut reply);
            }
        }
        if !self.owner.finish_read_result(
            valid(self.owner, request, &reply, &expected),
            ReadResultInvariantV1::NativeSearchFacadeResultInvariant,
            Boundary::SearchFacadeResultValidation,
            self.recorder,
        ) {
            return Err(Error::Faulted);
        }
        Ok(reply)
    }
}
