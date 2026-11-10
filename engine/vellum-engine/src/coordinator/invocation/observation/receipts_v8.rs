//! Original native source-load results; V1–V7 exhaustive profiles stay unchanged.
use super::*;
#[derive(Clone, Copy)]
pub enum InvocationReceiptRequestV8<'a> {
    Existing(InvocationReceiptRequestV7<'a>),
    SourceLoad(&'a TrustedSourceLoadResultV1<'a>),
}
pub enum InvocationReceiptV8<'a> {
    Existing(InvocationReceiptV7<'a>),
    SourceLoad(SourceLoadResultV1),
}
impl InvocationReceiptV8<'_> {
    pub fn invocation(&self) -> &InvocationId {
        match self {
            Self::Existing(v) => v.invocation(),
            Self::SourceLoad(v) => &v.invocation,
        }
    }
    /// Complete conservative native envelope, not wire/RSS accounting.
    pub fn required_reply_bytes(maximum: usize) -> Result<usize, InvocationError> {
        maximum
            .checked_add(size_of::<InvocationReceiptV8<'static>>())
            .ok_or(InvocationError::Limit)
    }
}
impl std::fmt::Debug for InvocationReceiptV8<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationReceiptV8(<redacted>)")
    }
}
impl SourceCoordinator {
    pub fn open_invocation_observation_with_receipts_v8(
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
            ReceiptProfile::V8,
            r,
        )
    }
    pub fn query_invocation_receipt_v8<'a>(
        &'a self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV8<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV8<'a>, InvocationError> {
        let timing = r.begin_service_timing();
        let result = if observer.receipt_profile != ReceiptProfile::V8 {
            Err(InvocationError::Unavailable)
        } else {
            self.query_invocation_receipt_native_v8(observer, request, current, facts, r)
        };
        capture_timed(
            r,
            Boundary::InvocationReceiptRelease,
            &result,
            timing.finish(),
        );
        result
    }
    pub(super) fn query_invocation_receipt_native_v8<'a>(
        &'a self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV8<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV8<'a>, InvocationError> {
        let result = (|| {
            let value = match request {
                InvocationReceiptRequestV8::Existing(q) => InvocationReceiptV8::Existing(
                    self.query_invocation_receipt_native_v7(observer, q, current, facts, r)?,
                ),
                InvocationReceiptRequestV8::SourceLoad(native) => {
                    self.validate_lifecycle_receipt_release(
                        observer,
                        receipts::NativeReceiptFrame {
                            context: native.context,
                            epoch: native.publication_epoch,
                            bytes: size_of::<SourceLoadResultV1>(),
                        },
                        current,
                        facts,
                    )?;
                    InvocationReceiptV8::SourceLoad(self.source_load_result_v1(
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
