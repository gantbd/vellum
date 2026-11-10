//! Native fixed-port source reads and passive native arguments. No wire codec.
use super::*;
use crate::capture::{CaptureLimitsV1, InertCaptureV1, InertValueV1};
use std::mem::size_of;
use std::panic::{AssertUnwindSafe, catch_unwind};
pub(in crate::coordinator) mod acquisition;
pub use acquisition::{
    DecodedReadAcquisitionFacadeRequestV1, NormalizedReadAcquisitionArgumentsV1,
    ReadAcquisitionEnvelopeValueV1, ReadAcquisitionParticipantV1, ReadAcquisitionSelectionV1,
    normalize_read_acquisition_envelope_v1,
};
pub(in crate::coordinator) mod arguments;
pub use arguments::{
    DecodedSourceFacadeRequestV1, NormalizedSourceArgumentsV1, SourceEnvelopeValueV1,
    normalize_source_envelope_v1,
};
pub(in crate::coordinator) mod annotations;
pub use acquisition::{
    ReadAcquisitionFacadePortV1, ReadAcquisitionFacadeReplyV1, ReadAcquisitionFacadeRequestV1,
};
mod facts;
pub(in crate::coordinator) mod history;
pub use history::{
    DecodedHistoryFacadeRequestV1, HistoryEnvelopeValueV1, HistoryFacadeRequestV1,
    NormalizedHistoryArgumentsV1, normalize_history_envelope_v1,
};
mod lifetime;
mod search;
mod stop;
pub use annotations::{
    AnnotationArgumentFactsV1, AnnotationArgumentQueryV1, AnnotationArgumentTargetV1,
    AnnotationEnvelopeValueV1, DecodedAnnotationFacadeRequestV1, NormalizedAnnotationArgumentsV1,
    normalize_annotation_envelope_v1,
};
pub use annotations::{AnnotationFacadePortV1, AnnotationFacadeReplyV1, AnnotationFacadeRequestV1};
use lifetime::{Family, Gateway};
pub use search::{SearchFacadePortV1, SearchFacadeReplyV1, SearchFacadeRequestV1};
pub use stop::ReadObservationStopPortV1;
pub use stop::{
    DecodedReadObservationStopRequestV1, NormalizedReadObservationStopArgumentsV1,
    ReadObservationStopEnvelopeValueV1, ReadObservationStopTargetV1,
    normalize_read_observation_stop_envelope_v1,
};
#[cfg(test)]
mod tests;
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::coordinator) enum Fault {
    Classification,
    Workspace,
    BeforeNative,
    AfterNative,
    BadResult,
    RefundDrift,
    Reentry,
}

/// Native handles stay outside inert copying. Actor labels are not scope grants.
#[derive(Clone, Copy)]
pub struct SourceFacadeRequestV1<'a> {
    pub source: SearchReadContextV1<'a>,
    pub scope: &'a SourceReadScopeV1,
    pub actor: &'a SettingsExecutionContextV1,
    pub query: SourceContextQueryV1,
    pub limits: CaptureLimitsV1,
    pub facts: SearchReadFactsV1<'a>,
}
pub enum SourceFacadeReplyV1 {
    Source(SourceContextReplyV1),
    Combined(AnnotationSourceReplyV1),
    VersionSet(VersionSetSourceReplyV1),
}
impl std::fmt::Debug for SourceFacadeRequestV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceFacadeRequestV1(<redacted>)")
    }
}
impl std::fmt::Debug for SourceFacadeReplyV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceFacadeReplyV1(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceFacadeUsageV1 {
    pub busy: bool,
    pub workspace_bytes: usize,
}
pub type ReadFacadeUsageV1 = SourceFacadeUsageV1;
fn observation(r: &mut Recorder, boundary: Boundary, disposition: Disposition) {
    timed_observation(r, boundary, disposition, None);
}
fn timed_observation(
    r: &mut Recorder,
    boundary: Boundary,
    disposition: Disposition,
    service_time: Option<std::time::Duration>,
) {
    r.record(Observation {
        boundary,
        disposition,
        service_time,
    });
}
fn frame(
    value: &InertValueV1<'_>,
    limits: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<InertCaptureV1, Error> {
    InertCaptureV1::copy(value, limits, r).map_err(|e| match e {
        crate::capture::CaptureErrorV1::Capacity => Error::Capacity,
        _ => Error::InvalidRequest,
    })
}
fn labels(c: &SettingsExecutionContextV1) -> [(&str, InertValueV1<'_>); 7] {
    [
        ("authority", InertValueV1::Text(&c.authority)),
        ("engine", InertValueV1::Text(&c.engine)),
        ("recipient", InertValueV1::Text(&c.recipient)),
        ("purpose", InertValueV1::Text(&c.purpose)),
        ("destination", InertValueV1::Text(&c.destination)),
        ("profile", InertValueV1::Text(&c.profile)),
        ("continuity", InertValueV1::Text(&c.continuity)),
    ]
}
fn copy_scope(s: &SourceReadScopeV1) -> Result<SourceReadScopeV1, Error> {
    Ok(SourceReadScopeV1 {
        document: s.document.clone(),
        context: settings::basis::copy_context(&s.context).map_err(|_| Error::Capacity)?,
        policy: owned(&s.policy)?,
    })
}
impl SourceCoordinator {
    /// Shared read-adapter state (including history); no recipient authority follows.
    pub fn read_facade_usage(&self) -> ReadFacadeUsageV1 {
        self.source_facade_usage()
    }
    /// Trusted-owner gauges, not a recipient-safe result or read authority.
    pub fn source_facade_usage(&self) -> SourceFacadeUsageV1 {
        SourceFacadeUsageV1 {
            busy: self.source_facade_busy,
            workspace_bytes: self.source_facade_workspace,
        }
    }
    /// One fixed synchronous native read port; no provider callbacks or fallback.
    pub fn query_source_facade_v1(
        &mut self,
        request: SourceFacadeRequestV1<'_>,
        r: &mut Recorder,
    ) -> Option<SourceFacadeReplyV1> {
        self.try_query_source_facade_v1(request, r).ok()
    }
    /// Explicit native adapter outcome. Authority refusals remain indistinguishable;
    /// late faults do not imply rollback or discharge of retained obligations.
    pub fn try_query_source_facade_v1(
        &mut self,
        request: SourceFacadeRequestV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourceFacadeReplyV1, SourceReadContextErrorV1> {
        lifetime::execute(self, Family::Source, r, |gateway| gateway.dispatch(request))
    }
}
impl Gateway<'_> {
    fn dispatch(
        &mut self,
        request: SourceFacadeRequestV1<'_>,
    ) -> Result<SourceFacadeReplyV1, Error> {
        if let Err(error) = self.admit(request.limits) {
            observation(
                self.recorder,
                Boundary::SourceFacadeAdmission,
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
                    .query_source_facade_v1(request, self.recorder)
                    .is_none()
            );
        }
        self.dispatch_source_admitted(request)
    }
    fn dispatch_source_admitted(
        &mut self,
        request: SourceFacadeRequestV1<'_>,
    ) -> Result<SourceFacadeReplyV1, Error> {
        let maximum = request
            .limits
            .max_bytes
            .min(self.owner.settings_job_parent_limit());
        let limits = CaptureLimitsV1 {
            max_bytes: maximum,
            ..request.limits
        };
        let domain = match request.source {
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
        ) != domain
            || labels(request.actor)
                .iter()
                .any(|(_, v)| matches!(v, InertValueV1::Text(v) if v.is_empty()))
        {
            return Err(Error::Unavailable);
        }
        let actor_fields = labels(request.actor);
        let _actor = frame(&InertValueV1::Object(&actor_fields), limits, self.recorder)?;
        let safe = |n: usize| (n as u128) <= 9_007_199_254_740_991;
        let fields = match request.query {
            SourceContextQueryV1::Range { start, end }
                if start.0 <= end.0 && safe(start.0) && safe(end.0) =>
            {
                [
                    ("operation", InertValueV1::Text("bounded-range")),
                    ("lo", InertValueV1::Number(start.0 as f64)),
                    ("hi", InertValueV1::Number(end.0 as f64)),
                ]
            }
            SourceContextQueryV1::Line { index } if safe(index) => [
                ("operation", InertValueV1::Text("line")),
                ("line", InertValueV1::Number(index as f64)),
                ("reserved", InertValueV1::Null),
            ],
            _ => return Err(Error::InvalidRequest),
        };
        let count = if matches!(request.query, SourceContextQueryV1::Line { .. }) {
            2
        } else {
            3
        };
        let _envelope = frame(
            &InertValueV1::Object(&fields[..count]),
            limits,
            self.recorder,
        )?;
        if size_of::<Vec<(&str, InertValueV1<'_>)>>() + 9 * size_of::<(&str, InertValueV1<'_>)>()
            > maximum
        {
            return Err(Error::Limit);
        }
        let mut scope_fields = Vec::new();
        scope_fields
            .try_reserve_exact(9)
            .map_err(|_| Error::Capacity)?;
        scope_fields.extend(labels(&request.scope.context));
        scope_fields.push(("policy", InertValueV1::Text(&request.scope.policy)));
        scope_fields.push(("document", InertValueV1::Null)); // actual native handle remains separate
        let _canonical = frame(&InertValueV1::Object(&scope_fields), limits, self.recorder)?;
        drop(scope_fields); // release template before facts staging: at most seven live frames
        // These validation shadows carry no authority and are no longer read.
        // Discharge them before native/facts staging so a retained normalized
        // argument frame does not add an eighth live workspace frame.
        drop((_actor, _envelope, _canonical));
        if scope_bytes(request.scope)? > maximum {
            return Err(Error::Limit);
        }
        let scope = copy_scope(request.scope)?;
        let facts = facts::copy(request, limits, self.owner, self.recorder)?;
        observation(
            self.recorder,
            Boundary::SourceFacadeCapture,
            Disposition::Completed,
        );
        #[cfg(test)]
        assert_ne!(
            self.owner.source_facade_fault,
            Some(Fault::BeforeNative),
            "injected pre-handoff fault"
        );
        let produced = facts.query(
            self.owner,
            request.source,
            &scope,
            request.query,
            self.recorder,
        );
        observation(
            self.recorder,
            Boundary::SourceFacadeNativeHandoff,
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
        #[cfg(test)]
        {
            assert_ne!(
                self.owner.source_facade_fault,
                Some(Fault::AfterNative),
                "injected late fault"
            );
            if self.owner.source_facade_fault == Some(Fault::BadResult) {
                let content = match &mut reply {
                    SourceFacadeReplyV1::Source(v) => &mut v.content,
                    SourceFacadeReplyV1::Combined(v) => &mut v.content,
                    SourceFacadeReplyV1::VersionSet(v) => &mut v.content,
                };
                match content {
                    SourceContextContentV1::Range(v) => v.end.0 ^= 1,
                    SourceContextContentV1::Line(v) => v.line ^= 1,
                }
            }
        }
        if !self.owner.finish_read_result(
            valid_reply(&reply, request),
            ReadResultInvariantV1::NativeSourceFacadeResultInvariant,
            Boundary::SourceFacadeResultValidation,
            self.recorder,
        ) {
            return Err(Error::Faulted);
        }
        Ok(reply)
    }
}
fn valid_reply(reply: &SourceFacadeReplyV1, request: SourceFacadeRequestV1<'_>) -> bool {
    let content = match (request.source, reply) {
        (SearchReadContextV1::Source(id), SourceFacadeReplyV1::Source(v)) if id == &v.context => {
            &v.content
        }
        (SearchReadContextV1::Combined(id), SourceFacadeReplyV1::Combined(v))
            if id == &v.context =>
        {
            &v.content
        }
        (SearchReadContextV1::VersionSet(id), SourceFacadeReplyV1::VersionSet(v))
            if id == &v.snapshot =>
        {
            &v.content
        }
        _ => return false,
    };
    match (request.query, content) {
        (SourceContextQueryV1::Range { start, end }, SourceContextContentV1::Range(v)) => {
            v.source.document == request.scope.document
                && v.start == start
                && v.end == end
                && end.0.checked_sub(start.0) == Some(v.bytes().len())
        }
        (SourceContextQueryV1::Line { index }, SourceContextContentV1::Line(v)) => {
            v.source.document == request.scope.document
                && v.line == index
                && v.span.body_end.0.checked_sub(v.span.start.0) == Some(v.body().len())
                && v.span.end.0.checked_sub(v.span.body_end.0) == Some(v.delimiter().len())
                && matches!(v.delimiter(), b"" | b"\n" | b"\r" | b"\r\n")
        }
        _ => false,
    }
}
