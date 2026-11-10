//! Fixed native F04 adapter and passive decoded arguments; wire codecs stay separate.
use super::*;
use crate::coordinator::history::inspection as native;
pub(in crate::coordinator) mod arguments;
pub use arguments::{
    DecodedHistoryFacadeRequestV1, HistoryEnvelopeValueV1, NormalizedHistoryArgumentsV1,
    normalize_history_envelope_v1,
};
#[cfg(test)]
pub(in crate::coordinator) mod tests;

#[derive(Clone, Copy)]
pub struct HistoryFacadeRequestV1<'a> {
    pub context: &'a HistoryInspectionIdV1,
    pub scope: &'a HistoryInspectionScopeV1,
    pub actor: &'a HistoryInspectionScopeV1,
    pub query: HistoryInspectionQueryV1<'a>,
    pub limits: CaptureLimitsV1,
    pub facts: &'a TrustedHistoryInspectionFactsV1<'a>,
}
impl std::fmt::Debug for HistoryFacadeRequestV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HistoryFacadeRequestV1(<redacted>)")
    }
}
pub(super) fn native_error(e: HistoryError) -> Error {
    match e {
        HistoryError::Faulted => Error::Faulted,
        HistoryError::Capacity => Error::Capacity,
        HistoryError::Limit => Error::Limit,
        _ => Error::Unavailable,
    }
}
fn scope_fields(s: &HistoryInspectionScopeV1) -> [(&str, InertValueV1<'_>); 8] {
    let [a, b, c, d, e, f, g] = labels(&s.context);
    [
        a,
        b,
        c,
        d,
        e,
        f,
        g,
        ("policy", InertValueV1::Text(&s.policy)),
    ]
}
fn copy_scope(
    s: &HistoryInspectionScopeV1,
    l: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<HistoryInspectionScopeV1, Error> {
    let fields = scope_fields(s);
    if size_of_val(&fields) > l.max_bytes
        || native::scope_bytes(s).map_err(native_error)? > l.max_bytes
    {
        return Err(Error::Limit);
    }
    let _shadow = frame(&InertValueV1::Object(&fields), l, r)?;
    native::copy_scope(s).map_err(native_error)
}
#[derive(Clone, Copy)]
enum Kind {
    Frontier,
    Availability,
    Detail,
}
struct CopiedQuery {
    kind: Kind,
    direction: HistoryDirection,
    scope: String,
    group: Option<String>,
}
impl CopiedQuery {
    fn query(&self) -> HistoryInspectionQueryV1<'_> {
        match self.kind {
            Kind::Frontier => HistoryInspectionQueryV1::Frontier {
                scope: &self.scope,
                direction: self.direction,
            },
            Kind::Availability => HistoryInspectionQueryV1::Availability {
                scope: &self.scope,
                direction: self.direction,
                group: self.group.as_deref(),
            },
            Kind::Detail => HistoryInspectionQueryV1::PermittedBlockingDetail {
                scope: &self.scope,
                direction: self.direction,
                group: self.group.as_deref(),
            },
        }
    }
}
fn copy_query(
    q: HistoryInspectionQueryV1<'_>,
    l: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<CopiedQuery, Error> {
    let (kind, scope, direction, group, operation) = match q {
        HistoryInspectionQueryV1::Frontier { scope, direction } => {
            (Kind::Frontier, scope, direction, None, "frontier")
        }
        HistoryInspectionQueryV1::Availability {
            scope,
            direction,
            group,
        } => (Kind::Availability, scope, direction, group, "availability"),
        HistoryInspectionQueryV1::PermittedBlockingDetail {
            scope,
            direction,
            group,
        } => (
            Kind::Detail,
            scope,
            direction,
            group,
            "permitted-blocking-detail",
        ),
    };
    if scope.is_empty() || group.is_some_and(str::is_empty) {
        return Err(Error::InvalidRequest);
    }
    let fields = [
        ("operation", InertValueV1::Text(operation)),
        ("scope", InertValueV1::Text(scope)),
        (
            "direction",
            InertValueV1::Text(match direction {
                HistoryDirection::Undo => "undo",
                HistoryDirection::Redo => "redo",
            }),
        ),
        (
            "group",
            group.map_or(InertValueV1::Null, InertValueV1::Text),
        ),
    ];
    if size_of_val(&fields) > l.max_bytes
        || size_of::<CopiedQuery>()
            .checked_add(scope.len())
            .and_then(|n| n.checked_add(group.map_or(0, str::len)))
            .is_none_or(|n| n > l.max_bytes)
    {
        return Err(Error::Limit);
    }
    let count = if matches!(kind, Kind::Frontier) { 3 } else { 4 };
    let _shadow = frame(&InertValueV1::Object(&fields[..count]), l, r)?;
    Ok(CopiedQuery {
        kind,
        direction,
        scope: owned(scope)?,
        group: group.map(owned).transpose()?,
    })
}
struct CopiedFacts {
    scope: HistoryInspectionScopeV1,
    flags: [Option<bool>; 12],
}
impl CopiedFacts {
    fn facts(&self) -> TrustedHistoryInspectionFactsV1<'_> {
        let [
            granted,
            retained,
            live,
            continuous,
            transfer_ready,
            history_current,
            capture_complete,
            preparation_reserved,
            registry_ready,
            retention_ready,
            not_stopped,
            reply_ready,
        ] = self.flags;
        TrustedHistoryInspectionFactsV1 {
            scope: &self.scope,
            granted,
            retained,
            live,
            continuous,
            transfer_ready,
            history_current,
            capture_complete,
            preparation_reserved,
            registry_ready,
            retention_ready,
            not_stopped,
            reply_ready,
        }
    }
}
fn copy_facts(
    f: &TrustedHistoryInspectionFactsV1<'_>,
    l: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<CopiedFacts, Error> {
    let flags = [
        f.granted,
        f.retained,
        f.live,
        f.continuous,
        f.transfer_ready,
        f.history_current,
        f.capture_complete,
        f.preparation_reserved,
        f.registry_ready,
        f.retention_ready,
        f.not_stopped,
        f.reply_ready,
    ];
    let fields = scope_fields(f.scope);
    let values = flags.map(|v| v.map_or(InertValueV1::Null, InertValueV1::Boolean));
    let scope = InertValueV1::Object(&fields);
    let facts = InertValueV1::Array(&values);
    let packet = [("scope", scope), ("facts", facts)];
    let template = size_of_val(&fields)
        .checked_add(size_of_val(&values))
        .and_then(|n| n.checked_add(size_of_val(&packet)))
        .ok_or(Error::Limit)?;
    if template > l.max_bytes
        || native::scope_bytes(f.scope)
            .map_err(native_error)?
            .checked_add(size_of::<CopiedFacts>())
            .is_none_or(|n| n > l.max_bytes)
    {
        return Err(Error::Limit);
    }
    let _shadow = frame(&InertValueV1::Object(&packet), l, r)?;
    Ok(CopiedFacts {
        scope: native::copy_scope(f.scope).map_err(native_error)?,
        flags,
    })
}
impl SourceCoordinator {
    /// One synchronous fixed native port. None is ordinary unavailable; fault
    /// diagnosis is a separate trusted-owner facet, never a recipient payload.
    pub fn query_history_facade_v1(
        &mut self,
        request: HistoryFacadeRequestV1<'_>,
        r: &mut Recorder,
    ) -> Option<HistoryInspectionReplyV1> {
        self.try_query_history_facade_v1(request, r).ok()
    }
    /// Explicit native adapter outcome. Authority refusals remain indistinguishable;
    /// late faults do not imply rollback or discharge of retained obligations.
    pub fn try_query_history_facade_v1(
        &mut self,
        request: HistoryFacadeRequestV1<'_>,
        r: &mut Recorder,
    ) -> Result<HistoryInspectionReplyV1, SourceReadContextErrorV1> {
        lifetime::execute(self, Family::History, r, |gateway| {
            gateway.dispatch_history(request)
        })
    }
}
impl Gateway<'_> {
    fn dispatch_history(
        &mut self,
        request: HistoryFacadeRequestV1<'_>,
    ) -> Result<HistoryInspectionReplyV1, Error> {
        if let Err(error) = self.admit(request.limits) {
            observation(
                self.recorder,
                Boundary::HistoryFacadeAdmission,
                if error == Error::Faulted {
                    Disposition::Unknown
                } else {
                    Disposition::Refused
                },
            );
            return Err(error);
        }
        #[cfg(test)]
        if self.owner.source_facade_fault == Some(Fault::Reentry) {
            assert!(
                self.owner
                    .query_history_facade_v1(request, self.recorder)
                    .is_none()
            );
        }
        self.dispatch_history_admitted(request)
    }
    fn dispatch_history_admitted(
        &mut self,
        request: HistoryFacadeRequestV1<'_>,
    ) -> Result<HistoryInspectionReplyV1, Error> {
        // History uses its own held/workspace ceilings, not settings intent or
        // history capture maxima as an undeclared substitute request limit.
        let limits = request.limits;
        let actor = copy_scope(request.actor, limits, self.recorder)?;
        let scope = copy_scope(request.scope, limits, self.recorder)?;
        if actor != scope
            || !self.owner.history_facade_scope_matches(&actor)
            || request.facts.scope != request.scope
        {
            return Err(Error::Unavailable);
        }
        let query = copy_query(request.query, limits, self.recorder)?;
        let facts = copy_facts(request.facts, limits, self.recorder)?;
        if query.query() != request.query || facts.scope != scope {
            return Err(Error::Faulted);
        }
        observation(
            self.recorder,
            Boundary::HistoryFacadeCapture,
            Disposition::Completed,
        );
        #[cfg(test)]
        assert_ne!(
            self.owner.source_facade_fault,
            Some(Fault::BeforeNative),
            "injected history pre-handoff fault"
        );
        let produced = self.owner.query_history_inspection_v1(
            request.context,
            query.query(),
            &facts.facts(),
            self.recorder,
        );
        observation(
            self.recorder,
            Boundary::HistoryFacadeNativeHandoff,
            if self.owner.faulted {
                Disposition::Unknown
            } else if matches!(produced, HistoryInspectionReplyV1::Unavailable) {
                Disposition::Refused
            } else {
                Disposition::Completed
            },
        );
        #[cfg(test)]
        assert_ne!(
            self.owner.source_facade_fault,
            Some(Fault::AfterNative),
            "injected history post-handoff fault"
        );
        #[allow(unused_mut)]
        let mut reply = match produced {
            HistoryInspectionReplyV1::Unavailable => {
                return Err(if self.owner.faulted {
                    Error::Faulted
                } else {
                    Error::Unavailable
                });
            }
            value => value,
        };
        #[cfg(test)]
        {
            if self.owner.source_facade_fault == Some(Fault::BadResult) {
                reply = match reply {
                    HistoryInspectionReplyV1::Frontier { context, .. } => {
                        HistoryInspectionReplyV1::Frontier {
                            context,
                            group: Some("unprojected".into()),
                        }
                    }
                    HistoryInspectionReplyV1::Availability { context, .. } => {
                        HistoryInspectionReplyV1::Frontier {
                            context,
                            group: None,
                        }
                    }
                    HistoryInspectionReplyV1::PermittedBlockingDetail { context, .. } => {
                        HistoryInspectionReplyV1::Frontier {
                            context,
                            group: None,
                        }
                    }
                    HistoryInspectionReplyV1::Unavailable => unreachable!(),
                };
            }
        }
        if !self.owner.finish_read_result(
            self.owner.history_facade_result_valid(
                request.context,
                request.scope,
                request.query,
                &reply,
            ),
            ReadResultInvariantV1::NativeHistoryFacadeResultInvariant,
            Boundary::HistoryFacadeResultValidation,
            self.recorder,
        ) {
            return Err(Error::Faulted);
        }
        Ok(reply)
    }
}
