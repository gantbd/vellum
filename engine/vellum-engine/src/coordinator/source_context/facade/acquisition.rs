//! Fixed native F02 acquisitions. No public identity codec or preparation verb.
use super::*;
pub(in crate::coordinator) mod arguments;
mod capture;
pub use arguments::*;
mod dispatch;
mod result;
#[cfg(test)]
pub(in crate::coordinator) mod tests;
#[derive(Clone, Copy)]
pub enum ReadAcquisitionFacadePortV1<'a> {
    SourceCurrent {
        scope: &'a SourceReadScopeV1,
        facts: &'a TrustedSourceReadFactsV1<'a>,
    },
    SourceRetained {
        scope: &'a SourceReadScopeV1,
        original: &'a SourceReadContextId,
        facts: &'a TrustedSourceReadFactsV1<'a>,
    },
    AnnotationCurrent {
        scope: &'a SourceReadScopeV1,
        classes: AnnotationProjectionClassesV1,
        prepared: &'a PreparedAnnotationProjectionId,
        facts: &'a TrustedPreparedAnnotationFactsV1<'a>,
    },
    AnnotationRetained {
        scope: &'a SourceReadScopeV1,
        classes: AnnotationProjectionClassesV1,
        original: &'a AnnotationReadContextId,
        facts: &'a TrustedAnnotationContextFactsV1<'a>,
        preparation_ready: Option<bool>,
    },
    VersionSetCurrent {
        participants: &'a [VersionSetParticipantV1<'a>],
        prepared: &'a PreparedVersionSetId,
        facts: &'a TrustedPreparedVersionSetFactsV1<'a>,
    },
    VersionSetRetained {
        participants: &'a [VersionSetParticipantV1<'a>],
        original: &'a VersionSetContextId,
        facts: &'a TrustedVersionSetContextFactsV1<'a>,
        preparation_ready: Option<bool>,
    },
}
#[derive(Clone, Copy)]
pub struct ReadAcquisitionFacadeRequestV1<'a> {
    pub port: ReadAcquisitionFacadePortV1<'a>,
    pub actor: &'a SettingsExecutionContextV1,
    pub limits: CaptureLimitsV1,
}
pub enum ReadAcquisitionFacadeReplyV1 {
    Source(SourceReadContextId),
    Annotation(AnnotationContextAcquisitionV1),
    VersionSet(VersionSetAcquisitionV1),
}
macro_rules! redacted {
    ($t:ty) => {
        impl std::fmt::Debug for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("<native-read-acquisition:redacted>")
            }
        }
    };
}
redacted!(ReadAcquisitionFacadePortV1<'_>);
redacted!(ReadAcquisitionFacadeRequestV1<'_>);
redacted!(ReadAcquisitionFacadeReplyV1);
impl<'a> ReadAcquisitionFacadePortV1<'a> {
    fn count(self) -> usize {
        match self {
            Self::VersionSetCurrent { participants, .. }
            | Self::VersionSetRetained { participants, .. } => participants.len(),
            _ => 1,
        }
    }
    fn at(self, index: usize) -> (&'a SourceReadScopeV1, AnnotationProjectionClassesV1) {
        match self {
            Self::SourceCurrent { scope, .. } | Self::SourceRetained { scope, .. } => {
                (scope, AnnotationProjectionClassesV1::Source)
            }
            Self::AnnotationCurrent { scope, classes, .. }
            | Self::AnnotationRetained { scope, classes, .. } => (scope, classes),
            Self::VersionSetCurrent { participants, .. }
            | Self::VersionSetRetained { participants, .. } => {
                (participants[index].scope, participants[index].data)
            }
        }
    }
    fn fact_input(self) -> facts::FrameInput<'a> {
        match self {
            Self::SourceCurrent { facts, .. } | Self::SourceRetained { facts, .. } => {
                facts::FrameInput::Source(facts)
            }
            Self::AnnotationCurrent { facts, .. } => facts::FrameInput::PreparedAnnotation(facts),
            Self::AnnotationRetained { facts, .. } => facts::FrameInput::Annotation(facts),
            Self::VersionSetCurrent { facts, .. } => facts::FrameInput::PreparedVersionSet(facts),
            Self::VersionSetRetained { facts, .. } => facts::FrameInput::VersionSet(facts),
        }
    }
}
impl SourceCoordinator {
    pub fn acquire_read_facade_v1(
        &mut self,
        request: ReadAcquisitionFacadeRequestV1<'_>,
        r: &mut Recorder,
    ) -> Option<ReadAcquisitionFacadeReplyV1> {
        self.try_acquire_read_facade_v1(request, r).ok()
    }
    /// Explicit native adapter outcome. Authority refusals remain indistinguishable;
    /// late faults do not imply rollback or discharge of retained obligations.
    pub fn try_acquire_read_facade_v1(
        &mut self,
        request: ReadAcquisitionFacadeRequestV1<'_>,
        r: &mut Recorder,
    ) -> Result<ReadAcquisitionFacadeReplyV1, SourceReadContextErrorV1> {
        lifetime::execute(self, Family::Acquisition, r, |gateway| {
            gateway.dispatch_acquisition(request)
        })
    }
}
impl Gateway<'_> {
    fn dispatch_acquisition(
        &mut self,
        request: ReadAcquisitionFacadeRequestV1<'_>,
    ) -> Result<ReadAcquisitionFacadeReplyV1, Error> {
        if let Err(error) = self.admit(request.limits) {
            observation(
                self.recorder,
                Boundary::AcquisitionFacadeAdmission,
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
                    .acquire_read_facade_v1(request, self.recorder)
                    .is_none()
            );
        }
        self.dispatch_acquisition_admitted(request)
    }
    fn dispatch_acquisition_admitted(
        &mut self,
        request: ReadAcquisitionFacadeRequestV1<'_>,
    ) -> Result<ReadAcquisitionFacadeReplyV1, Error> {
        use ReadAcquisitionFacadePortV1 as Port;
        let limits = CaptureLimitsV1 {
            max_bytes: request
                .limits
                .max_bytes
                .min(self.owner.settings_job_parent_limit()),
            ..request.limits
        };
        let domain = match request.port {
            Port::SourceCurrent { .. } | Port::SourceRetained { .. } => {
                let pool = self.owner.source_pool()?;
                (&*pool.authority, &*pool.engine)
            }
            Port::AnnotationCurrent { .. } | Port::AnnotationRetained { .. } => {
                self.owner.annotation_context_domain()?
            }
            Port::VersionSetCurrent { .. } | Port::VersionSetRetained { .. } => {
                self.owner.version_context_domain()?
            }
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
        let ordinary = capture::copy(request.port, limits, self.recorder)?;
        let matched = match request.port {
            Port::AnnotationCurrent {
                scope,
                classes,
                prepared,
                ..
            } => self
                .owner
                .prepared_annotation_matches(prepared, scope, classes),
            Port::VersionSetCurrent {
                participants,
                prepared,
                ..
            } => self
                .owner
                .prepared_version_set_matches(prepared, participants),
            _ => true,
        };
        observation(
            self.recorder,
            Boundary::AcquisitionFacadeBinding,
            if matched {
                Disposition::Completed
            } else {
                Disposition::Refused
            },
        );
        if !matched {
            return Err(Error::Unavailable);
        }
        drop(_actor); // validated shadow no longer needed during facts staging
        let (rows, _facts_shadow) =
            facts::copy_rows(request.port.fact_input(), limits, self.recorder)?;
        observation(
            self.recorder,
            Boundary::AcquisitionFacadeCapture,
            Disposition::Completed,
        );
        #[cfg(test)]
        assert_ne!(
            self.owner.source_facade_fault,
            Some(Fault::BeforeNative),
            "injected pre-acquisition fault"
        );
        let produced = dispatch::run(
            self.owner,
            request.port,
            &ordinary.rows,
            &rows,
            self.recorder,
        );
        observation(
            self.recorder,
            Boundary::AcquisitionFacadeNativeHandoff,
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
                "injected post-acquisition fault"
            );
            if self.owner.source_facade_fault == Some(Fault::BadResult) {
                result::corrupt(&mut reply, request.port);
            }
        }
        if !self.owner.finish_read_result(
            result::valid(&reply, request.port),
            ReadResultInvariantV1::NativeAcquisitionFacadeResultInvariant,
            Boundary::AcquisitionFacadeResultValidation,
            self.recorder,
        ) {
            return Err(Error::Faulted);
        }
        Ok(reply)
    }
}
