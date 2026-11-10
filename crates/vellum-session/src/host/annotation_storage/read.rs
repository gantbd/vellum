//! Original provider-attributed reads, inert JSON decoding and native activation.
use super::*;
use vellum_engine::resource::{
    AnnotationBaselineEvidence, AnnotationReadId, AnnotationReadStatus,
    AnnotationRestorationOutcome, AnnotationRestorerPort, ContentComparison, ReadStrength,
};
use vellum_host::annotation::{
    AnnotationCodec, AnnotationRestorationFailure, CodecLimits, DeclaredContext, DecodeContext,
    DecodedAnnotationRead, RestorationAssociations,
};

/// The original native read owner plus its source attribution. Retain this
/// attempt before service; decode/activation errors never erase its obligations.
pub struct AnnotationReadAttempt {
    id: AnnotationReadId,
    document: DocumentId,
}
impl std::fmt::Debug for AnnotationReadAttempt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationReadAttempt(<opaque>)")
    }
}
impl AnnotationReadAttempt {
    pub fn native_id(&self) -> &AnnotationReadId {
        &self.id
    }
}
#[derive(Debug)]
pub struct AnnotationReadReport {
    pub status: AnnotationReadStatus,
    pub failure: Option<AdapterFailure>,
}
impl<A: ResourceAdapter> LocalHost<A> {
    /// Captured-read strength only; this local adapter does not promise an atomic
    /// resource snapshot. Exact source/set versions and native binding are checked.
    pub fn queue_annotation_read(
        &mut self,
        resource: &AnnotationResource<A::Handle>,
        expected: &DocumentVersion,
        expected_set: u64,
    ) -> Result<AnnotationReadAttempt, SessionError> {
        if expected.document != resource.document {
            return Err(SessionError::WrongDocument);
        }
        let target = self
            .resource_adapter
            .annotation_read_target(&resource.resource)
            .map_err(|error| SessionError::Adapter(error.classification()))?;
        let pair = self.ordinary_pair(&resource.document)?;
        let id = self.owner.admit_annotation_read(
            expected,
            expected_set,
            target,
            ReadStrength::CapturedRead,
            pair,
            &mut self.diagnostics,
        )?;
        Ok(AnnotationReadAttempt {
            id,
            document: resource.document.clone(),
        })
    }

    /// Service once, ingest each original attributed receipt, then return status.
    /// Failure and quiescence are distinct; no automatic retry or decode follows.
    pub fn service_annotation_read(
        &mut self,
        attempt: &AnnotationReadAttempt,
    ) -> Result<AnnotationReadReport, SessionError> {
        let pair = self.ordinary_pair(&attempt.document)?;
        let dispatch =
            self.owner
                .submit_annotation_read(&attempt.id, pair, &mut self.diagnostics)?;
        let completion = self
            .resource_adapter
            .read_annotation(&dispatch, &mut self.diagnostics)
            .map_err(|error| SessionError::Adapter(error.classification()))?;
        for receipt in completion.receipts {
            self.owner
                .record_annotation_read(receipt, &mut self.diagnostics)?;
        }
        self.owner
            .record_annotation_read(completion.quiescence, &mut self.diagnostics)?;
        Ok(AnnotationReadReport {
            status: self
                .owner
                .annotation_read_status(&attempt.id, &mut self.diagnostics)?,
            failure: completion
                .failure
                .as_ref()
                .map(ClassifiedAdapterError::classification),
        })
    }

    /// Original capture validated against current owner/binding. The context is
    /// an explicit host assertion, never identities selected as grants by JSON.
    pub fn inspect_annotations<C: AnnotationCodec + ?Sized>(
        &mut self,
        codec: &C,
        attempt: &AnnotationReadAttempt,
        limits: CodecLimits,
    ) -> Result<DeclaredContext, SessionError> {
        let pair = self.ordinary_pair(&attempt.document)?;
        let capture =
            self.owner
                .annotation_read_capture(&attempt.id, pair, &mut self.diagnostics)?;
        codec
            .inspect(capture.bytes(), limits)
            .map_err(SessionError::AnnotationCodec)
    }
    pub fn decode_annotations<C: AnnotationCodec + ?Sized>(
        &mut self,
        codec: &C,
        attempt: &AnnotationReadAttempt,
        context: &DecodeContext,
        limits: CodecLimits,
    ) -> Result<DecodedAnnotationRead, SessionError> {
        let pair = self.ordinary_pair(&attempt.document)?;
        let capture =
            self.owner
                .annotation_read_capture(&attempt.id, pair, &mut self.diagnostics)?;
        codec
            .decode(capture, context, limits, &mut self.diagnostics)
            .map_err(SessionError::AnnotationCodec)
    }

    /// Original native restorer captures the intended serialization context. It
    /// is not an encoder port or authority inferred from a stored identity.
    pub fn register_annotation_restorer(
        &mut self,
        encoder: &AnnotationEncoderPort,
    ) -> Result<AnnotationRestorerPort, SessionError> {
        Ok(self
            .owner
            .register_annotation_restorer_for_encoding(&encoder.context(), &mut self.diagnostics)?)
    }

    /// Consumes decoded inert data and explicit approved source witnesses/record
    /// identities. Native activation validates the original read and allocates
    /// new native identities; old-process handles cannot be revived by JSON.
    pub fn restore_annotations<C: AnnotationCodec + ?Sized>(
        &mut self,
        codec: &C,
        decoded: DecodedAnnotationRead,
        associations: &RestorationAssociations,
        restorer: &AnnotationRestorerPort,
        limits: CodecLimits,
    ) -> Result<AnnotationRestorationOutcome, SessionError> {
        let document = codec.decoded_document(&decoded).clone();
        let package = codec
            .restore(
                decoded,
                associations,
                restorer,
                limits,
                &mut self.diagnostics,
            )
            .map_err(|error| match error {
                AnnotationRestorationFailure::Codec(error) => SessionError::AnnotationCodec(error),
                AnnotationRestorationFailure::Engine(error) => SessionError::Resource(error),
            })?;
        let pair = self.ordinary_pair(&document)?;
        Ok(self
            .owner
            .restore_annotations(package, pair, &mut self.diagnostics)?)
    }

    pub fn annotation_read_status(
        &mut self,
        attempt: &AnnotationReadAttempt,
    ) -> Result<AnnotationReadStatus, SessionError> {
        Ok(self
            .owner
            .annotation_read_status(&attempt.id, &mut self.diagnostics)?)
    }
    pub fn annotation_restoration_outcome(
        &mut self,
        attempt: &AnnotationReadAttempt,
    ) -> Result<AnnotationRestorationOutcome, SessionError> {
        Ok(self
            .owner
            .annotation_restoration_outcome(&attempt.id, &mut self.diagnostics)?)
    }
    pub fn stop_annotation_read(
        &mut self,
        attempt: &AnnotationReadAttempt,
    ) -> Result<(), SessionError> {
        Ok(self
            .owner
            .stop_annotation_read(&attempt.id, &mut self.diagnostics)?)
    }
    /// Can refuse Retained: decoded captures, restoration provenance and loaded
    /// baselines remain real owners. Stop/Drop never implies their retirement.
    pub fn retire_annotation_read(
        &mut self,
        attempt: &AnnotationReadAttempt,
    ) -> Result<(), SessionError> {
        Ok(self
            .owner
            .retire_annotation_read(&attempt.id, &mut self.diagnostics)?)
    }

    /// Explicit historical-result loss after an independently acknowledged,
    /// current annotation checkpoint replaces the loaded baseline. Native read
    /// loans, live dependencies and unsettled writes still refuse retirement.
    pub fn retire_checkpointed_annotation_read(
        &mut self,
        attempt: &AnnotationReadAttempt,
    ) -> Result<(), SessionError> {
        Ok(self
            .owner
            .retire_superseded_annotation_read(&attempt.id, &mut self.diagnostics)?)
    }

    /// Alternative protected discharge after the original document's bindings,
    /// history and provenance are actually released. Disposal alone is not enough.
    pub fn retire_disposed_annotation_read(
        &mut self,
        attempt: &AnnotationReadAttempt,
    ) -> Result<(), SessionError> {
        Ok(self
            .owner
            .retire_disposed_annotation_read(&attempt.id, &mut self.diagnostics)?)
    }

    /// Remove owner-wide read/baseline endpoints only after native consumers and
    /// producing reads discharge. This is not a whole-engine shutdown receipt.
    pub fn disable_annotation_read_baselines(&mut self) -> Result<bool, SessionError> {
        Ok(self
            .owner
            .detach_annotation_read_baselines(&mut self.diagnostics)?)
    }

    /// Explicit owner-wide installation of the existing loaded-baseline endpoint.
    /// This enables later adoption; it does not mark any read as saved.
    pub fn enable_annotation_read_baselines(&mut self) -> Result<bool, SessionError> {
        Ok(self
            .owner
            .attach_annotation_read_baselines(&mut self.diagnostics)?)
    }
    pub fn adopt_loaded_annotation_baseline(
        &mut self,
        attempt: &AnnotationReadAttempt,
        max_baseline_bytes: usize,
    ) -> Result<bool, SessionError> {
        let pair = self.ordinary_pair(&attempt.document)?;
        Ok(self.owner.adopt_loaded_annotation_baseline(
            &attempt.id,
            pair,
            max_baseline_bytes,
            &mut self.diagnostics,
        )?)
    }
    /// Written/Loaded remain distinct facts. An owned baseline does not by itself
    /// prove current content equals it or authorize disclosure to another caller.
    pub fn annotation_baseline_evidence(
        &mut self,
        document: &LocalDocument<A::Handle>,
    ) -> Result<Option<AnnotationBaselineEvidence>, SessionError> {
        let pair = self.ordinary_pair(&document.loaded.document)?;
        Ok(self.owner.annotation_baseline_evidence(
            &document.loaded.document,
            pair,
            &mut self.diagnostics,
        )?)
    }
    pub fn compare_annotations(
        &mut self,
        document: &LocalDocument<A::Handle>,
        current: &EncodedAnnotations,
        max_combined_bytes: usize,
    ) -> Result<ContentComparison, SessionError> {
        let pair = self.ordinary_pair(&document.loaded.document)?;
        Ok(self.owner.compare_annotation_baseline(
            &document.loaded.document,
            current,
            pair,
            max_combined_bytes,
            &mut self.diagnostics,
        )?)
    }

    /// Discharge the disposed annotation binding separately from source binding,
    /// original reads and history. This is not deletion of the annotation file.
    pub fn release_annotation_binding(
        &mut self,
        document: &LocalDocument<A::Handle>,
        expected: LifecycleState,
    ) -> Result<LifecycleState, SessionError> {
        self.owner.release_disposed_annotation_binding(
            &document.loaded.document,
            expected,
            &mut self.diagnostics,
        )?;
        self.lifecycle(document)
    }
}
