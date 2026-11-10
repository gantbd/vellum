//! Annotation serialization and local I/O orchestration, above native ownership.
use super::*;
use std::sync::{Arc, Mutex};
use vellum_engine::coordinator::AnnotationSnapshot;
use vellum_engine::resource::{
    AnnotationBaselineFact, AnnotationEncoderPort, EncodedAnnotations, SavePurpose,
};
use vellum_host::annotation::{
    AnnotationCodec, AnnotationEncodingFailure, CaptureAssociations, CodecLimits,
};
mod read;
pub use read::{AnnotationReadAttempt, AnnotationReadReport};

/// Existing resolved destination bound to one original source owner. No path-
/// inferred identity, permission grant, auto sidecar name or hidden document.
#[derive(Clone)]
pub struct AnnotationResource<H> {
    resource: H,
    document: DocumentId,
    // Integration guard shared by clones of this exact binding. Only an actual
    // written observation advances it; probing newer bytes never adopts them.
    observed: Arc<Mutex<Option<vellum_engine::resource::ResourceRead>>>,
}
impl<A: ResourceAdapter> LocalHost<A> {
    /// Explicit existing-file association. The actual adapter compares native
    /// resource identities before native activation; source aliases refuse.
    /// This does not read/restore JSON records or establish a saved baseline.
    pub fn bind_annotations(
        &mut self,
        document: &LocalDocument<A::Handle>,
        locator: &A::Locator,
    ) -> Result<AnnotationResource<A::Handle>, SessionError> {
        let (resource, read) = self
            .resource_adapter
            .open(locator, &mut self.diagnostics)
            .map_err(|error| SessionError::Adapter(error.classification()))?;
        let observed = Arc::new(Mutex::new(Some(read.clone())));
        let plan = self.owner.prepare_annotation_binding(
            &document.loaded.document,
            read,
            &mut self.diagnostics,
        )?;
        let pair = self
            .resource_adapter
            .compare_ordinary_pair(plan.pair_check(), &mut self.diagnostics)
            .map_err(|error| SessionError::Adapter(error.classification()))?;
        self.owner
            .activate_annotation_binding(plan, Some(pair), &mut self.diagnostics)?;
        Ok(AnnotationResource {
            resource,
            document: document.loaded.document.clone(),
            observed,
        })
    }

    /// Register the existing native producer once per configured codec owner.
    /// The port is an original privileged handle, not an untrusted plugin API.
    pub fn register_annotation_encoder(&mut self) -> Result<AnnotationEncoderPort, SessionError> {
        Ok(self
            .owner
            .register_annotation_encoder(&mut self.diagnostics)?)
    }

    /// Host-supplied persistent identity/evidence associations are explicit
    /// premises. No aliases, source hashes or history are invented from a path.
    pub fn encode_annotations<C: AnnotationCodec + ?Sized>(
        &mut self,
        codec: &C,
        snapshot: &AnnotationSnapshot,
        associations: &CaptureAssociations,
        limits: CodecLimits,
        encoder: &AnnotationEncoderPort,
    ) -> Result<EncodedAnnotations, SessionError> {
        codec
            .encode(
                snapshot,
                associations,
                limits,
                encoder,
                &mut self.diagnostics,
            )
            .map_err(|error| match error {
                AnnotationEncodingFailure::Codec(error) => SessionError::AnnotationCodec(error),
                AnnotationEncodingFailure::Engine(error) => SessionError::Resource(error),
            })
    }

    /// Admission only. Retain the returned original attempt before service.
    /// Source and annotation saves are separate writes, never a two-file commit.
    pub fn queue_annotation_save(
        &mut self,
        destination: &AnnotationResource<A::Handle>,
        encoded: EncodedAnnotations,
        requirements: SaveRequirements,
    ) -> Result<SaveId, SessionError> {
        let (read, probe) = self
            .resource_adapter
            .probe(&destination.resource, &mut self.diagnostics)
            .map_err(|error| SessionError::Adapter(error.classification()))?;
        drop(probe);
        let observed = destination
            .observed
            .lock()
            .map_err(|_| ResourceError::Unavailable)?;
        let expected = observed.as_ref().ok_or(ResourceError::MissingEvidence)?;
        if !expected.same_observation(&read) {
            return Err(ResourceError::Conflict.into());
        }
        let pair = self.ordinary_pair(&destination.document)?;
        Ok(self.owner.queue_annotation_save(
            encoded,
            read,
            requirements,
            pair,
            &mut self.diagnostics,
        )?)
    }

    pub fn service_annotation_save(
        &mut self,
        destination: &AnnotationResource<A::Handle>,
        task: &SaveId,
    ) -> Result<SaveReport, SessionError> {
        let outcome = self.owner.save_outcome(task, &mut self.diagnostics)?;
        if outcome.captured.document != destination.document
            || outcome.purpose != SavePurpose::OrdinaryAnnotations
        {
            return Err(SessionError::WrongDocument);
        }
        if outcome.phase != vellum_engine::resource::SavePhase::Queued {
            return Err(ResourceError::Unavailable.into());
        }
        let mut observed = destination
            .observed
            .lock()
            .map_err(|_| ResourceError::Unavailable)?;
        // Once service starts, an intake error must not leave a usable old
        // checkpoint pretending that a potentially applied write did not occur.
        *observed = None;
        let (report, written) = self.service_resource_observed(
            &destination.document,
            &destination.resource,
            task,
            true,
        )?;
        *observed = written;
        Ok(report)
    }

    /// Uses the task's prepaid native baseline budget after truthful receipts.
    /// Success is not a source save, file-pair atomicity or crash-durability claim.
    pub fn publish_annotation_save_baseline(
        &mut self,
        destination: &AnnotationResource<A::Handle>,
        task: &SaveId,
    ) -> Result<bool, SessionError> {
        let outcome = self.owner.save_outcome(task, &mut self.diagnostics)?;
        if outcome.captured.document != destination.document
            || outcome.purpose != SavePurpose::OrdinaryAnnotations
        {
            return Err(SessionError::WrongDocument);
        }
        let pair = self.ordinary_pair(&destination.document)?;
        Ok(self
            .owner
            .install_annotation_save_baseline(task, pair, &mut self.diagnostics)?)
    }

    pub fn annotation_baseline(
        &mut self,
        document: &LocalDocument<A::Handle>,
    ) -> Result<Option<AnnotationBaselineFact>, SessionError> {
        let pair = self.ordinary_pair(&document.loaded.document)?;
        Ok(self.owner.annotation_baseline(
            &document.loaded.document,
            pair,
            &mut self.diagnostics,
        )?)
    }
}
