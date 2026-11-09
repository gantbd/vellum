//! Original scratch allocation identity; prior exhaustive profiles stay unchanged.
use super::*;
#[derive(Clone, Copy)]
pub enum InvocationReceiptRequestV6<'a> {
    Existing(InvocationReceiptRequestV5<'a>),
    Scratch(&'a TrustedScratchResultV1<'a>),
}
pub enum InvocationReceiptV6<'a> {
    Existing(InvocationReceiptV5<'a>),
    Scratch(ScratchResultV1),
}
impl InvocationReceiptV6<'_> {
    pub fn invocation(&self) -> &InvocationId {
        match self {
            Self::Existing(v) => v.invocation(),
            Self::Scratch(v) => &v.invocation,
        }
    }
    /// Complete conservative native envelope, not wire/RSS accounting.
    pub fn required_reply_bytes(maximum: usize) -> Result<usize, InvocationError> {
        maximum
            .checked_add(size_of::<InvocationReceiptV6<'static>>())
            .ok_or(InvocationError::Limit)
    }
}
impl std::fmt::Debug for InvocationReceiptV6<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationReceiptV6(<redacted>)")
    }
}
impl SourceCoordinator {
    pub fn open_invocation_observation_with_receipts_v6(
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
            ReceiptProfile::V6,
            r,
        )
    }
    pub fn query_invocation_receipt_v6<'a>(
        &'a self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV6<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV6<'a>, InvocationError> {
        let timing = r.begin_service_timing();
        let result = if observer.receipt_profile != ReceiptProfile::V6 {
            Err(InvocationError::Unavailable)
        } else {
            self.query_invocation_receipt_native_v6(observer, request, current, facts, r)
        };
        capture_timed(
            r,
            Boundary::InvocationReceiptRelease,
            &result,
            timing.finish(),
        );
        result
    }
    pub(super) fn query_invocation_receipt_native_v6<'a>(
        &'a self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV6<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV6<'a>, InvocationError> {
        let result = (|| {
            let value = match request {
                InvocationReceiptRequestV6::Existing(q) => InvocationReceiptV6::Existing(
                    self.query_invocation_receipt_native_v5(observer, q, current, facts, r)?,
                ),
                InvocationReceiptRequestV6::Scratch(native) => {
                    self.validate_lifecycle_receipt_release(
                        observer,
                        receipts::NativeReceiptFrame {
                            context: native.context,
                            epoch: native.publication_epoch,
                            bytes: size_of::<ScratchResultV1>(),
                        },
                        current,
                        facts,
                    )?;
                    InvocationReceiptV6::Scratch(self.scratch_result_v1(
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
