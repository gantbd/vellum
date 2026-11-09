//! Explicit native receipt expansion for the three retained-release operations.
use super::*;
#[derive(Clone, Copy)]
pub enum InvocationReceiptRequestV3<'a> {
    Existing(InvocationReceiptRequestV2<'a>),
    RetainedRelease(&'a TrustedRetainedReleaseResultV1<'a>),
}
/// Prior exhaustive profiles remain unchanged. No generic lifecycle fallback.
pub enum InvocationReceiptV3<'a> {
    Existing(InvocationReceiptV2<'a>),
    RetainedRelease(RetainedReleaseResultV1),
}
impl InvocationReceiptV3<'_> {
    pub fn invocation(&self) -> &InvocationId {
        match self {
            Self::Existing(v) => v.invocation(),
            Self::RetainedRelease(v) => &v.invocation,
        }
    }
    pub fn required_reply_bytes(maximum: usize) -> Result<usize, InvocationError> {
        maximum
            .checked_add(size_of::<InvocationReceiptV3<'static>>())
            .ok_or(InvocationError::Limit)
    }
}
impl std::fmt::Debug for InvocationReceiptV3<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationReceiptV3(<redacted>)")
    }
}
impl SourceCoordinator {
    pub fn open_invocation_observation_with_receipts_v3(
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
            ReceiptProfile::V3,
            r,
        )
    }
    pub fn query_invocation_receipt_v3<'a>(
        &'a self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV3<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV3<'a>, InvocationError> {
        let timing = r.begin_service_timing();
        let result = if observer.receipt_profile != ReceiptProfile::V3 {
            Err(InvocationError::Unavailable)
        } else {
            self.query_invocation_receipt_native_v3(observer, request, current, facts, r)
        };
        if matches!(result, Err(InvocationError::Faulted)) {
            observer.receipt_faulted = true;
        }
        capture_timed(
            r,
            Boundary::InvocationReceiptRelease,
            &result,
            timing.finish(),
        );
        result
    }
    pub(super) fn query_invocation_receipt_native_v3<'a>(
        &'a self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV3<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV3<'a>, InvocationError> {
        let value = match request {
            InvocationReceiptRequestV3::Existing(request) => InvocationReceiptV3::Existing(
                self.query_invocation_receipt_native_v2(observer, request, current, facts, r)?,
            ),
            InvocationReceiptRequestV3::RetainedRelease(native) => {
                self.validate_lifecycle_receipt_release(
                    observer,
                    receipts::NativeReceiptFrame {
                        context: native.context,
                        epoch: native.publication_epoch,
                        bytes: size_of::<RetainedReleaseResultV1>(),
                    },
                    current,
                    facts,
                )?;
                InvocationReceiptV3::RetainedRelease(self.retained_release_result_v1(
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
    }
}
