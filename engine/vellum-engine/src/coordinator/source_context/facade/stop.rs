//! Protected native F19 stop composition. Original borrowed capabilities and
//! immutable Rust metadata only; not a wire decoder, read grant or retirement.
use super::*;
mod arguments;
pub use arguments::{
    DecodedReadObservationStopRequestV1, NormalizedReadObservationStopArgumentsV1,
    ReadObservationStopEnvelopeValueV1, ReadObservationStopTargetV1,
    normalize_read_observation_stop_envelope_v1,
};
#[cfg(test)]
mod tests;

#[derive(Clone, Copy)]
pub enum ReadObservationStopPortV1<'a> {
    Source {
        context: &'a SourceReadContextId,
        control: &'a SourceReadContextControlV1<'a>,
    },
    Annotation {
        context: &'a AnnotationReadContextId,
        control: &'a SourceReadContextControlV1<'a>,
    },
    VersionSet {
        target: VersionSetControlTargetV1<'a>,
        controls: &'a [SourceReadContextControlV1<'a>],
    },
    History {
        context: &'a HistoryInspectionIdV1,
        control: &'a HistoryInspectionControlV1<'a>,
    },
}
impl std::fmt::Debug for ReadObservationStopPortV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReadObservationStopPortV1(<redacted>)")
    }
}
impl SourceCoordinator {
    /// Protected metadata control, not a data release: may succeed under an
    /// existing editing fault without healing it. No ordinary read admission,
    /// selector probing, copying, new pool, refund or producer cancellation.
    pub fn stop_read_observation_facade_v1(
        &mut self,
        port: ReadObservationStopPortV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), SourceReadContextErrorV1> {
        let timing = r.begin_service_timing();
        let out = catch_unwind(AssertUnwindSafe(|| {
            #[cfg(test)]
            assert_ne!(
                self.source_facade_fault,
                Some(Fault::BeforeNative),
                "injected stop pre-handoff fault"
            );
            let out = match port {
                ReadObservationStopPortV1::Source { context, control } => {
                    self.stop_source_read_context_result(context, control, r)
                }
                ReadObservationStopPortV1::Annotation { context, control } => {
                    self.stop_annotation_context_result(context, control, r)
                }
                ReadObservationStopPortV1::VersionSet { target, controls } => {
                    self.stop_version_set_context_result(target, controls, r)
                }
                ReadObservationStopPortV1::History { context, control } => self
                    .stop_history_inspection_v1_result(context, control, r)
                    .map_err(history::native_error),
            };
            #[cfg(test)]
            assert_ne!(
                self.source_facade_fault,
                Some(Fault::AfterNative),
                "injected stop post-handoff fault"
            );
            out
        }))
        .unwrap_or(Err(Error::Faulted));
        if matches!(out, Err(Error::Faulted)) {
            self.faulted = true;
        }
        timed_observation(
            r,
            Boundary::ReadObservationStopFacade,
            match &out {
                Ok(()) => Disposition::Completed,
                Err(Error::Faulted) => Disposition::Unknown,
                Err(_) => Disposition::Refused,
            },
            timing.finish(),
        );
        out
    }
}
