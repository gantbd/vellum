//! Original native annotation-load results; V1–V8 exhaustive profiles stay unchanged.
use super::*;
#[derive(Clone, Copy)]
pub enum InvocationReceiptRequestV9<'a> {
    Existing(InvocationReceiptRequestV8<'a>),
    AnnotationLoad(&'a TrustedAnnotationLoadResultV1<'a>),
}
pub enum InvocationReceiptV9<'a> {
    Existing(InvocationReceiptV8<'a>),
    AnnotationLoad(AnnotationLoadResultV1),
}
impl InvocationReceiptV9<'_> {
    pub fn invocation(&self) -> &InvocationId {
        match self {
            Self::Existing(v) => v.invocation(),
            Self::AnnotationLoad(v) => &v.invocation,
        }
    }
    /// Complete conservative native envelope, not wire/RSS accounting.
    pub fn required_reply_bytes(maximum: usize) -> Result<usize, InvocationError> {
        maximum
            .checked_add(size_of::<InvocationReceiptV9<'static>>())
            .ok_or(InvocationError::Limit)
    }
}
impl std::fmt::Debug for InvocationReceiptV9<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationReceiptV9(<redacted>)")
    }
}
impl SourceCoordinator {
    pub fn open_invocation_observation_with_receipts_v9(
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
            ReceiptProfile::V9,
            r,
        )
    }
    pub fn query_invocation_receipt_v9<'a>(
        &'a self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV9<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV9<'a>, InvocationError> {
        let timing = r.begin_service_timing();
        let result = if observer.receipt_profile != ReceiptProfile::V9 {
            Err(InvocationError::Unavailable)
        } else {
            self.query_invocation_receipt_native_v9(observer, request, current, facts, r)
        };
        capture_timed(
            r,
            Boundary::InvocationReceiptRelease,
            &result,
            timing.finish(),
        );
        result
    }
    pub(super) fn query_invocation_receipt_native_v9<'a>(
        &'a self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV9<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV9<'a>, InvocationError> {
        let result = (|| {
            let value = match request {
                InvocationReceiptRequestV9::Existing(q) => InvocationReceiptV9::Existing(
                    self.query_invocation_receipt_native_v8(observer, q, current, facts, r)?,
                ),
                InvocationReceiptRequestV9::AnnotationLoad(native) => {
                    self.validate_lifecycle_receipt_release(
                        observer,
                        receipts::NativeReceiptFrame {
                            context: native.context,
                            epoch: native.publication_epoch,
                            bytes: size_of::<AnnotationLoadResultV1>(),
                        },
                        current,
                        facts,
                    )?;
                    InvocationReceiptV9::AnnotationLoad(self.annotation_load_result_v1(
                        &observer.invocation,
                        native,
                        r,
                    )?)
                }
            };
            if value.invocation() != &observer.invocation {
                return Err(InvocationError::Faulted);
            }
            Ok(value)
        })();
        if matches!(result, Err(InvocationError::Faulted)) {
            observer.receipt_faulted = true;
        }
        result
    }
}
