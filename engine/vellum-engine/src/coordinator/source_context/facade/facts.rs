//! Separately bounded native facts; native handles remain original opaque identities.
use super::*;
pub(super) type Flags = [Option<bool>; 12];
pub(super) type FactRows = Vec<(SourceReadScopeV1, Flags)>;
#[derive(Clone, Copy)]
pub(super) enum FrameInput<'a> {
    Source(&'a TrustedSourceReadFactsV1<'a>),
    Annotation(&'a TrustedAnnotationContextFactsV1<'a>),
    PreparedAnnotation(&'a TrustedPreparedAnnotationFactsV1<'a>),
    VersionSet(&'a TrustedVersionSetContextFactsV1<'a>),
    PreparedVersionSet(&'a TrustedPreparedVersionSetFactsV1<'a>),
}
enum Kind<'a> {
    Source,
    Combined(&'a AnnotationReadContextId),
    VersionSet(&'a VersionSetContextId),
}
pub(super) struct CopiedFacts<'a> {
    kind: Kind<'a>,
    rows: Vec<(SourceReadScopeV1, Flags)>,
    _shadow: InertCaptureV1,
}
pub(super) fn copy<'a>(
    request: SourceFacadeRequestV1<'a>,
    limits: CaptureLimitsV1,
    owner: &SourceCoordinator,
    r: &mut Recorder,
) -> Result<CopiedFacts<'a>, Error> {
    copy_inner(
        Input {
            source: request.source,
            scope: request.scope,
            facts: request.facts,
        },
        limits,
        owner,
        r,
        false,
    )
}
pub(super) struct Input<'a> {
    pub(super) source: SearchReadContextV1<'a>,
    pub(super) scope: &'a SourceReadScopeV1,
    pub(super) facts: SearchReadFactsV1<'a>,
}
pub(super) fn copy_annotation<'a>(
    request: Input<'a>,
    limits: CaptureLimitsV1,
    owner: &SourceCoordinator,
    r: &mut Recorder,
) -> Result<CopiedFacts<'a>, Error> {
    copy_inner(request, limits, owner, r, true)
}
fn copy_inner<'a>(
    request: Input<'a>,
    limits: CaptureLimitsV1,
    owner: &SourceCoordinator,
    r: &mut Recorder,
    annotation: bool,
) -> Result<CopiedFacts<'a>, Error> {
    if annotation && matches!(request.source, SearchReadContextV1::Source(_)) {
        return Err(Error::Unavailable);
    }
    let (kind, _count) = match (request.source, request.facts) {
        (SearchReadContextV1::Source(_), SearchReadFactsV1::Source(f))
            if f.scope == request.scope =>
        {
            (Kind::Source, 1)
        }
        (SearchReadContextV1::Combined(id), SearchReadFactsV1::Combined(f))
            if id == f.context && f.scope == request.scope =>
        {
            (Kind::Combined(f.context), 1)
        }
        (SearchReadContextV1::VersionSet(id), SearchReadFactsV1::VersionSet(f))
            if id == f.member && f.scope == request.scope =>
        {
            if annotation {
                owner.version_annotation_frame(id, request.scope, f.parent)?;
            } else {
                owner.version_search_frame(id, request.scope, f.parent)?;
            }
            (Kind::VersionSet(f.parent.context), f.parent.members.len())
        }
        _ => return Err(Error::Unavailable),
    };
    let input = match request.facts {
        SearchReadFactsV1::Source(f) => FrameInput::Source(f),
        SearchReadFactsV1::Combined(f) => FrameInput::Annotation(f),
        SearchReadFactsV1::VersionSet(f) => FrameInput::VersionSet(f.parent),
    };
    let (rows, shadow) = copy_rows(input, limits, r)?;
    Ok(CopiedFacts {
        kind,
        rows,
        _shadow: shadow,
    })
}
pub(super) fn copy_search<'a>(
    request: Input<'a>,
    limits: CaptureLimitsV1,
    owner: &SourceCoordinator,
    r: &mut Recorder,
) -> Result<CopiedFacts<'a>, Error> {
    copy_inner(request, limits, owner, r, false)
}
impl CopiedFacts<'_> {
    /// Private first-party dispatch closure; no caller/provider callback enters.
    pub(super) fn with_search_facts<T>(
        &self,
        source: SearchReadContextV1<'_>,
        selected_scope: &SourceReadScopeV1,
        call: impl for<'a> FnOnce(SearchReadFactsV1<'a>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let (scope, flags) = self.rows.first().ok_or(Error::Unavailable)?;
        match (&self.kind, source) {
            (Kind::Source, SearchReadContextV1::Source(_)) => {
                let facts = TrustedSourceReadFactsV1 {
                    scope,
                    authorized: flags[0],
                    scope_allowed: flags[1],
                    continuous: flags[2],
                    live: flags[3],
                    not_stopped: flags[4],
                    source_readable: flags[6],
                    retained: flags[8],
                    resources_ready: flags[9],
                    reply_ready: flags[10],
                    transfer_ready: flags[11],
                };
                call(SearchReadFactsV1::Source(&facts))
            }
            (Kind::Combined(original), SearchReadContextV1::Combined(id)) if *original == id => {
                let facts = TrustedAnnotationContextFactsV1 {
                    context: original,
                    scope,
                    authorized: flags[0],
                    scope_allowed: flags[1],
                    continuous: flags[2],
                    live: flags[3],
                    not_stopped: flags[4],
                    annotation_readable: flags[5],
                    source_readable: flags[6],
                    projection_current: flags[7],
                    retained: flags[8],
                    resources_ready: flags[9],
                    reply_ready: flags[10],
                    transfer_ready: flags[11],
                };
                call(SearchReadFactsV1::Combined(&facts))
            }
            (Kind::VersionSet(parent), SearchReadContextV1::VersionSet(member)) => {
                let mut members = Vec::new();
                members
                    .try_reserve_exact(self.rows.len())
                    .map_err(|_| Error::Capacity)?;
                for (scope, flags) in &self.rows {
                    members.push(TrustedVersionSetMemberFactsV1 {
                        scope,
                        authorized: flags[0],
                        scope_allowed: flags[1],
                        continuous: flags[2],
                        live: flags[3],
                        not_stopped: flags[4],
                        annotation_readable: flags[5],
                        source_readable: flags[6],
                        projection_current: flags[7],
                        retained: flags[8],
                        resources_ready: flags[9],
                        reply_ready: flags[10],
                        transfer_ready: flags[11],
                    });
                }
                let parent = TrustedVersionSetContextFactsV1 {
                    context: parent,
                    members: &members,
                };
                let facts = TrustedVersionSetSearchFactsV1 {
                    member,
                    scope: selected_scope,
                    parent: &parent,
                };
                call(SearchReadFactsV1::VersionSet(&facts))
            }
            _ => Err(Error::Unavailable),
        }
    }
}
pub(super) fn copy_rows(
    input: FrameInput<'_>,
    limits: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<(FactRows, InertCaptureV1), Error> {
    let count = match input {
        FrameInput::Source(_) | FrameInput::Annotation(_) | FrameInput::PreparedAnnotation(_) => 1,
        FrameInput::VersionSet(f) => f.members.len(),
        FrameInput::PreparedVersionSet(f) => f.members.len(),
    };
    let count_values = count.checked_mul(22).ok_or(Error::Limit)?;
    if count_values
        .checked_add(1)
        .is_none_or(|n| n > limits.max_nodes)
        || limits.max_depth < 2
        || count_values
            .checked_mul(size_of::<InertValueV1<'_>>())
            .and_then(|n| n.checked_add(size_of::<Vec<InertValueV1<'_>>>()))
            .is_none_or(|n| n > limits.max_bytes)
    {
        return Err(Error::Limit);
    }
    let get = |index: usize| -> (&SourceReadScopeV1, Flags) {
        match input {
            FrameInput::Source(f) => (
                f.scope,
                [
                    f.authorized,
                    f.scope_allowed,
                    f.continuous,
                    f.live,
                    f.not_stopped,
                    None,
                    f.source_readable,
                    None,
                    f.retained,
                    f.resources_ready,
                    f.reply_ready,
                    f.transfer_ready,
                ],
            ),
            FrameInput::Annotation(f) => (
                f.scope,
                [
                    f.authorized,
                    f.scope_allowed,
                    f.continuous,
                    f.live,
                    f.not_stopped,
                    f.annotation_readable,
                    f.source_readable,
                    f.projection_current,
                    f.retained,
                    f.resources_ready,
                    f.reply_ready,
                    f.transfer_ready,
                ],
            ),
            FrameInput::PreparedAnnotation(f) => (
                f.scope,
                [
                    f.authorized,
                    f.scope_allowed,
                    f.continuous,
                    f.live,
                    f.not_stopped,
                    f.annotation_readable,
                    f.source_readable,
                    f.projection_current,
                    f.retained,
                    f.resources_ready,
                    f.reply_ready,
                    f.transfer_ready,
                ],
            ),
            FrameInput::VersionSet(_) | FrameInput::PreparedVersionSet(_) => {
                let f = match input {
                    FrameInput::VersionSet(f) => &f.members[index],
                    FrameInput::PreparedVersionSet(f) => &f.members[index],
                    _ => unreachable!(),
                };
                (
                    f.scope,
                    [
                        f.authorized,
                        f.scope_allowed,
                        f.continuous,
                        f.live,
                        f.not_stopped,
                        f.annotation_readable,
                        f.source_readable,
                        f.projection_current,
                        f.retained,
                        f.resources_ready,
                        f.reply_ready,
                        f.transfer_ready,
                    ],
                )
            }
        }
    };
    let mut bytes = size_of::<CopiedFacts<'_>>()
        .checked_add(
            count
                .checked_mul(size_of::<Flags>() + size_of::<TrustedVersionSetMemberFactsV1<'_>>())
                .ok_or(Error::Limit)?,
        )
        .ok_or(Error::Limit)?;
    for i in 0..count {
        bytes = bytes
            .checked_add(scope_bytes(get(i).0)?)
            .ok_or(Error::Limit)?;
    }
    if bytes > limits.max_bytes {
        return Err(Error::Limit);
    }
    let mut values = Vec::new();
    values
        .try_reserve_exact(count_values)
        .map_err(|_| Error::Capacity)?;
    for i in 0..count {
        let (s, flags) = get(i);
        for (_, v) in labels(&s.context) {
            values.push(v);
        }
        values.push(InertValueV1::Text(&s.policy));
        // Original document and role identities stay outside the inert graph.
        values.push(InertValueV1::Null);
        values.push(InertValueV1::Null);
        for flag in flags {
            values.push(flag.map_or(InertValueV1::Null, InertValueV1::Boolean));
        }
    }
    let shadow = frame(&InertValueV1::Array(&values), limits, r)?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(count).map_err(|_| Error::Capacity)?;
    for i in 0..count {
        let (scope, flags) = get(i);
        rows.push((copy_scope(scope)?, flags));
    }
    Ok((rows, shadow))
}
impl CopiedFacts<'_> {
    pub(super) fn query_annotations(
        &self,
        owner: &mut SourceCoordinator,
        port: annotations::AnnotationFacadePortV1<'_>,
        scope: &SourceReadScopeV1,
        r: &mut Recorder,
    ) -> Result<annotations::AnnotationFacadeReplyV1, Error> {
        use annotations::{AnnotationFacadePortV1 as Port, AnnotationFacadeReplyV1 as Reply};
        match (&self.kind, port) {
            (Kind::Combined(original), Port::Single { context, query, .. })
                if *original == context =>
            {
                let (facts_scope, flags) = self.rows.first().ok_or(Error::Unavailable)?;
                let facts = TrustedAnnotationContextFactsV1 {
                    context: original,
                    scope: facts_scope,
                    authorized: flags[0],
                    scope_allowed: flags[1],
                    continuous: flags[2],
                    live: flags[3],
                    not_stopped: flags[4],
                    annotation_readable: flags[5],
                    source_readable: flags[6],
                    projection_current: flags[7],
                    retained: flags[8],
                    resources_ready: flags[9],
                    reply_ready: flags[10],
                    transfer_ready: flags[11],
                };
                owner
                    .query_annotation_context_result(context, scope, query, &facts, r)
                    .map(Reply::Single)
            }
            (Kind::VersionSet(parent), Port::VersionSet { member, query, .. }) => {
                let mut members = Vec::new();
                members
                    .try_reserve_exact(self.rows.len())
                    .map_err(|_| Error::Capacity)?;
                for (scope, flags) in &self.rows {
                    members.push(TrustedVersionSetMemberFactsV1 {
                        scope,
                        authorized: flags[0],
                        scope_allowed: flags[1],
                        continuous: flags[2],
                        live: flags[3],
                        not_stopped: flags[4],
                        annotation_readable: flags[5],
                        source_readable: flags[6],
                        projection_current: flags[7],
                        retained: flags[8],
                        resources_ready: flags[9],
                        reply_ready: flags[10],
                        transfer_ready: flags[11],
                    });
                }
                let facts = TrustedVersionSetContextFactsV1 {
                    context: parent,
                    members: &members,
                };
                owner
                    .query_version_set_annotations_result(member, query, &facts, r)
                    .map(Reply::VersionSet)
            }
            _ => Err(Error::Unavailable),
        }
    }

    pub(super) fn query(
        &self,
        owner: &mut SourceCoordinator,
        source: SearchReadContextV1<'_>,
        scope: &SourceReadScopeV1,
        query: SourceContextQueryV1,
        r: &mut Recorder,
    ) -> Result<SourceFacadeReplyV1, Error> {
        let (facts_scope, flags) = self.rows.first().ok_or(Error::Unavailable)?;
        match (&self.kind, source) {
            (Kind::Source, SearchReadContextV1::Source(id)) => {
                let facts = TrustedSourceReadFactsV1 {
                    scope: facts_scope,
                    authorized: flags[0],
                    scope_allowed: flags[1],
                    continuous: flags[2],
                    live: flags[3],
                    not_stopped: flags[4],
                    source_readable: flags[6],
                    retained: flags[8],
                    resources_ready: flags[9],
                    reply_ready: flags[10],
                    transfer_ready: flags[11],
                };
                owner
                    .query_source_read_context_result(id, scope, query, &facts, r)
                    .map(SourceFacadeReplyV1::Source)
            }
            (Kind::Combined(original), SearchReadContextV1::Combined(id)) if *original == id => {
                let facts = TrustedAnnotationContextFactsV1 {
                    context: original,
                    scope: facts_scope,
                    authorized: flags[0],
                    scope_allowed: flags[1],
                    continuous: flags[2],
                    live: flags[3],
                    not_stopped: flags[4],
                    annotation_readable: flags[5],
                    source_readable: flags[6],
                    projection_current: flags[7],
                    retained: flags[8],
                    resources_ready: flags[9],
                    reply_ready: flags[10],
                    transfer_ready: flags[11],
                };
                owner
                    .query_annotation_context_source_result(id, scope, query, &facts, r)
                    .map(SourceFacadeReplyV1::Combined)
            }
            (Kind::VersionSet(parent), SearchReadContextV1::VersionSet(id)) => {
                let mut members = Vec::new();
                members
                    .try_reserve_exact(self.rows.len())
                    .map_err(|_| Error::Capacity)?;
                for (scope, flags) in &self.rows {
                    members.push(TrustedVersionSetMemberFactsV1 {
                        scope,
                        authorized: flags[0],
                        scope_allowed: flags[1],
                        continuous: flags[2],
                        live: flags[3],
                        not_stopped: flags[4],
                        annotation_readable: flags[5],
                        source_readable: flags[6],
                        projection_current: flags[7],
                        retained: flags[8],
                        resources_ready: flags[9],
                        reply_ready: flags[10],
                        transfer_ready: flags[11],
                    });
                }
                let facts = TrustedVersionSetContextFactsV1 {
                    context: parent,
                    members: &members,
                };
                owner
                    .query_version_set_source_result(id, query, &facts, r)
                    .map(SourceFacadeReplyV1::VersionSet)
            }
            _ => Err(Error::Unavailable),
        }
    }
}
