//! Native F03 adapter; shared temporary ownership, distinct disclosure population.
use super::*;
pub(in crate::coordinator) mod arguments;
mod result;
pub use arguments::*;
#[cfg(test)]
pub(in crate::coordinator) mod tests;
#[derive(Clone, Copy)]
pub enum AnnotationFacadePortV1<'a> {
    Single {
        context: &'a AnnotationReadContextId,
        query: &'a AnnotationQueryV1,
        facts: &'a TrustedAnnotationContextFactsV1<'a>,
    },
    VersionSet {
        member: &'a VersionSetMemberId,
        query: &'a VersionSetAnnotationQueryV1,
        facts: &'a TrustedVersionSetContextFactsV1<'a>,
    },
}
#[derive(Clone, Copy)]
pub struct AnnotationFacadeRequestV1<'a> {
    pub port: AnnotationFacadePortV1<'a>,
    pub scope: &'a SourceReadScopeV1,
    pub actor: &'a SettingsExecutionContextV1,
    pub limits: CaptureLimitsV1,
}
pub enum AnnotationFacadeReplyV1 {
    Single(AnnotationQueryReplyV1),
    VersionSet(VersionSetAnnotationReplyV1),
}
macro_rules! redacted {
    ($t:ty) => {
        impl std::fmt::Debug for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("<native-annotation-facade:redacted>")
            }
        }
    };
}
redacted!(AnnotationFacadePortV1<'_>);
redacted!(AnnotationFacadeRequestV1<'_>);
redacted!(AnnotationFacadeReplyV1);
impl SourceCoordinator {
    pub fn query_annotation_facade_v1(
        &mut self,
        request: AnnotationFacadeRequestV1<'_>,
        r: &mut Recorder,
    ) -> Option<AnnotationFacadeReplyV1> {
        self.try_query_annotation_facade_v1(request, r).ok()
    }
    /// Explicit native adapter outcome. Authority refusals remain indistinguishable;
    /// late faults do not imply rollback or discharge of retained obligations.
    pub fn try_query_annotation_facade_v1(
        &mut self,
        request: AnnotationFacadeRequestV1<'_>,
        r: &mut Recorder,
    ) -> Result<AnnotationFacadeReplyV1, SourceReadContextErrorV1> {
        lifetime::execute(self, Family::Annotation, r, |gateway| {
            gateway.dispatch_annotation(request)
        })
    }
}
impl Gateway<'_> {
    fn dispatch_annotation(
        &mut self,
        request: AnnotationFacadeRequestV1<'_>,
    ) -> Result<AnnotationFacadeReplyV1, Error> {
        if let Err(error) = self.admit(request.limits) {
            observation(
                self.recorder,
                Boundary::AnnotationFacadeAdmission,
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
                    .query_annotation_facade_v1(request, self.recorder)
                    .is_none()
            );
        }
        self.dispatch_annotation_admitted(request)
    }
    fn dispatch_annotation_admitted(
        &mut self,
        request: AnnotationFacadeRequestV1<'_>,
    ) -> Result<AnnotationFacadeReplyV1, Error> {
        let maximum = request
            .limits
            .max_bytes
            .min(self.owner.settings_job_parent_limit());
        let limits = CaptureLimitsV1 {
            max_bytes: maximum,
            ..request.limits
        };
        let domain = match request.port {
            AnnotationFacadePortV1::Single { .. } => self.owner.annotation_context_domain()?,
            AnnotationFacadePortV1::VersionSet { .. } => self.owner.version_context_domain()?,
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
        let (operation, limit) = operation(request.port);
        if limit.is_some_and(|n| n == 0 || (n as u128) > 9_007_199_254_740_991) {
            return Err(Error::InvalidRequest);
        }
        let query_fields = [
            ("operation", InertValueV1::Text(operation)),
            (
                "argument",
                limit.map_or(InertValueV1::Null, |v| InertValueV1::Number(v as f64)),
            ),
        ];
        let _envelope = frame(&InertValueV1::Object(&query_fields), limits, self.recorder)?;
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
        scope_fields.push(("document", InertValueV1::Null));
        let _canonical = frame(&InertValueV1::Object(&scope_fields), limits, self.recorder)?;
        drop(scope_fields);
        drop((_actor, _envelope, _canonical));
        if scope_bytes(request.scope)? > maximum {
            return Err(Error::Limit);
        }
        let scope = copy_scope(request.scope)?;
        let produced = match request.port {
            AnnotationFacadePortV1::Single {
                context,
                facts: current,
                ..
            } => {
                let copied = facts::copy_annotation(
                    facts::Input {
                        source: SearchReadContextV1::Combined(context),
                        scope: request.scope,
                        facts: SearchReadFactsV1::Combined(current),
                    },
                    limits,
                    self.owner,
                    self.recorder,
                )?;
                observation(
                    self.recorder,
                    Boundary::AnnotationFacadeCapture,
                    Disposition::Completed,
                );
                #[cfg(test)]
                assert_ne!(
                    self.owner.source_facade_fault,
                    Some(Fault::BeforeNative),
                    "injected pre-handoff fault"
                );
                copied.query_annotations(self.owner, request.port, &scope, self.recorder)
            }
            AnnotationFacadePortV1::VersionSet {
                member,
                facts: current,
                ..
            } => {
                let selected = TrustedVersionSetSearchFactsV1 {
                    member,
                    scope: request.scope,
                    parent: current,
                };
                let copied = facts::copy_annotation(
                    facts::Input {
                        source: SearchReadContextV1::VersionSet(member),
                        scope: request.scope,
                        facts: SearchReadFactsV1::VersionSet(&selected),
                    },
                    limits,
                    self.owner,
                    self.recorder,
                )?;
                observation(
                    self.recorder,
                    Boundary::AnnotationFacadeCapture,
                    Disposition::Completed,
                );
                #[cfg(test)]
                assert_ne!(
                    self.owner.source_facade_fault,
                    Some(Fault::BeforeNative),
                    "injected pre-handoff fault"
                );
                copied.query_annotations(self.owner, request.port, &scope, self.recorder)
            }
        };
        observation(
            self.recorder,
            Boundary::AnnotationFacadeNativeHandoff,
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
                "injected post-handoff fault"
            );
            if self.owner.source_facade_fault == Some(Fault::BadResult) {
                result::corrupt(&mut reply);
            }
        }
        if !self.owner.finish_read_result(
            result::valid(&reply, request.port),
            ReadResultInvariantV1::NativeAnnotationFacadeResultInvariant,
            Boundary::AnnotationFacadeResultValidation,
            self.recorder,
        ) {
            return Err(Error::Faulted);
        }
        Ok(reply)
    }
}
fn operation(port: AnnotationFacadePortV1<'_>) -> (&'static str, Option<usize>) {
    match port {
        AnnotationFacadePortV1::Single { query, .. } => match query {
            AnnotationQueryV1::Enumerate { limit, .. } => ("enumerate", Some(*limit)),
            AnnotationQueryV1::Inspect { .. } => ("inspect", None),
            AnnotationQueryV1::TargetLocation { .. } => ("target-location", None),
        },
        AnnotationFacadePortV1::VersionSet { query, .. } => match query {
            VersionSetAnnotationQueryV1::Enumerate { limit, .. } => ("enumerate", Some(*limit)),
            VersionSetAnnotationQueryV1::Inspect { .. } => ("inspect", None),
            VersionSetAnnotationQueryV1::TargetLocation { .. } => ("target-location", None),
        },
    }
}
