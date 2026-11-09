//! Explicit expanded native result profile. The six-case V1 enum stays closed.
use super::*;
#[derive(Clone, Copy)]
pub enum InvocationReceiptRequestV2<'a> {
    Existing(InvocationReceiptRequestV1<'a>),
    RetainedDisposal(&'a TrustedRetainedDisposalResultV1<'a>),
}
/// Existing native families preserve their V1 shapes and release requirements.
/// RetainedDisposal does not advertise other document-lifecycle operations.
pub enum InvocationReceiptV2<'a> {
    Existing(InvocationReceiptV1<'a>),
    RetainedDisposal(RetainedDisposalResultV1),
}
impl InvocationReceiptV2<'_> {
    pub fn invocation(&self) -> &InvocationId {
        match self {
            Self::Existing(v) => v.invocation(),
            Self::RetainedDisposal(v) => &v.invocation,
        }
    }
    /// Complete conservative native wrapper plus the original payload ceiling.
    pub fn required_reply_bytes(native_maximum: usize) -> Result<usize, InvocationError> {
        native_maximum
            .checked_add(size_of::<InvocationReceiptV2<'static>>())
            .ok_or(InvocationError::Limit)
    }
}
impl std::fmt::Debug for InvocationReceiptV2<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationReceiptV2(<redacted>)")
    }
}
impl SourceCoordinator {
    pub fn open_invocation_observation_with_receipts_v2(
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
            ReceiptProfile::V2,
            r,
        )
    }
    pub fn query_invocation_receipt_v2<'a>(
        &'a self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV2<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV2<'a>, InvocationError> {
        let timing = r.begin_service_timing();
        let result = if observer.receipt_profile != ReceiptProfile::V2 {
            Err(InvocationError::Unavailable)
        } else {
            self.query_invocation_receipt_native_v2(observer, request, current, facts, r)
        };
        capture_timed(
            r,
            Boundary::InvocationReceiptRelease,
            &result,
            timing.finish(),
        );
        result
    }
    pub(super) fn query_invocation_receipt_native_v2<'a>(
        &'a self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV2<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV2<'a>, InvocationError> {
        let result = (|| {
            let value = match request {
                InvocationReceiptRequestV2::Existing(request) => InvocationReceiptV2::Existing(
                    self.query_invocation_receipt_native(observer, request, current, facts, r)?,
                ),
                InvocationReceiptRequestV2::RetainedDisposal(native) => {
                    self.validate_lifecycle_receipt_release(
                        observer,
                        receipts::NativeReceiptFrame {
                            context: native.context,
                            epoch: native.publication_epoch,
                            bytes: size_of::<RetainedDisposalResultV1>(),
                        },
                        current,
                        facts,
                    )?;
                    InvocationReceiptV2::RetainedDisposal(self.retained_disposal_result_v1(
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
