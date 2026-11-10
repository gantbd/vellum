//! Explicit atomic annotation receipt profile; prior exhaustive products unchanged.
use super::*;
#[derive(Clone, Copy)]
pub enum InvocationReceiptRequestV4<'a> {
    Existing(InvocationReceiptRequestV3<'a>),
    AtomicAnnotation {
        index: usize,
        native: &'a TrustedCompoundAnnotationResultReleaseV1<'a>,
    },
}
pub enum InvocationReceiptV4<'a> {
    Existing(InvocationReceiptV3<'a>),
    AtomicAnnotation(AnnotationResultV1),
}
impl InvocationReceiptV4<'_> {
    pub fn invocation(&self) -> &InvocationId {
        match self {
            Self::Existing(v) => v.invocation(),
            Self::AtomicAnnotation(v) => &v.invocation,
        }
    }
    /// Complete conservative native envelope, not wire bytes or allocator peak.
    pub fn required_reply_bytes(maximum: usize) -> Result<usize, InvocationError> {
        maximum
            .checked_add(size_of::<InvocationReceiptV4<'static>>())
            .ok_or(InvocationError::Limit)
    }
}
impl std::fmt::Debug for InvocationReceiptV4<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationReceiptV4(<redacted>)")
    }
}
impl SourceCoordinator {
    pub fn open_invocation_observation_with_receipts_v4(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        ceiling: &InvocationObservationPolicyV1,
        limits: InvocationObservationLimitsV1,
        admission: InvocationReceiptAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationObservationContextV1, InvocationError> {
        self.open_invocation_receipts_profile(
            id,
            c,
            ceiling,
            limits,
            admission,
            ReceiptProfile::V4,
            r,
        )
    }
    pub fn query_invocation_receipt_v4<'a>(
        &'a self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV4<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV4<'a>, InvocationError> {
        let timing = r.begin_service_timing();
        let result = if observer.receipt_profile != ReceiptProfile::V4 {
            Err(InvocationError::Unavailable)
        } else {
            self.query_invocation_receipt_native_v4(observer, request, current, facts, r)
        };
        if matches!(result, Err(InvocationError::Faulted)) {
            observer.receipt_faulted = true
        }
        capture_timed(
            r,
            Boundary::InvocationReceiptRelease,
            &result,
            timing.finish(),
        );
        result
    }
    pub(super) fn query_invocation_receipt_native_v4<'a>(
        &'a self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV4<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV4<'a>, InvocationError> {
        let value = match request {
            InvocationReceiptRequestV4::Existing(request) => InvocationReceiptV4::Existing(
                self.query_invocation_receipt_native_v3(observer, request, current, facts, r)?,
            ),
            InvocationReceiptRequestV4::AtomicAnnotation { index, native } => {
                self.validate_native_receipt_release(
                    observer,
                    InvocationReceiptKindV1::AtomicAnnotation,
                    receipts::NativeReceiptFrame {
                        context: native.context,
                        epoch: native.publication_epoch,
                        bytes: AnnotationResultV1::required_bytes(),
                    },
                    current,
                    facts,
                )?;
                InvocationReceiptV4::AtomicAnnotation(self.retained_compound_annotation_result_v1(
                    &observer.invocation,
                    index,
                    native,
                    r,
                )?)
            }
        };
        if value.invocation() != &observer.invocation {
            return Err(InvocationError::Faulted);
        }
        Ok(value)
    }
}
