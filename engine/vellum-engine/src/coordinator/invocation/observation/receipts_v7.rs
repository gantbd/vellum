//! Original native source-rebind results; V1–V6 exhaustive profiles stay unchanged.
use super::*;
#[derive(Clone, Copy)]
pub enum InvocationReceiptRequestV7<'a> {
    Existing(InvocationReceiptRequestV6<'a>),
    SourceRebind(&'a TrustedSourceRebindResultV1<'a>),
}
pub enum InvocationReceiptV7<'a> {
    Existing(InvocationReceiptV6<'a>),
    SourceRebind(SourceRebindResultV1),
}
impl InvocationReceiptV7<'_> {
    pub fn invocation(&self) -> &InvocationId {
        match self {
            Self::Existing(v) => v.invocation(),
            Self::SourceRebind(v) => &v.invocation,
        }
    }
    /// Complete conservative native envelope, not wire/RSS accounting.
    pub fn required_reply_bytes(maximum: usize) -> Result<usize, InvocationError> {
        maximum
            .checked_add(size_of::<InvocationReceiptV7<'static>>())
            .ok_or(InvocationError::Limit)
    }
}
impl std::fmt::Debug for InvocationReceiptV7<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationReceiptV7(<redacted>)")
    }
}
impl SourceCoordinator {
    pub fn open_invocation_observation_with_receipts_v7(
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
            ReceiptProfile::V7,
            r,
        )
    }
    pub fn query_invocation_receipt_v7<'a>(
        &'a self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV7<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV7<'a>, InvocationError> {
        let timing = r.begin_service_timing();
        let result = if observer.receipt_profile != ReceiptProfile::V7 {
            Err(InvocationError::Unavailable)
        } else {
            self.query_invocation_receipt_native_v7(observer, request, current, facts, r)
        };
        capture_timed(
            r,
            Boundary::InvocationReceiptRelease,
            &result,
            timing.finish(),
        );
        result
    }
    pub(super) fn query_invocation_receipt_native_v7<'a>(
        &'a self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV7<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV7<'a>, InvocationError> {
        let result = (|| {
            let value = match request {
                InvocationReceiptRequestV7::Existing(q) => InvocationReceiptV7::Existing(
                    self.query_invocation_receipt_native_v6(observer, q, current, facts, r)?,
                ),
                InvocationReceiptRequestV7::SourceRebind(native) => {
                    self.validate_lifecycle_receipt_release(
                        observer,
                        receipts::NativeReceiptFrame {
                            context: native.context,
                            epoch: native.publication_epoch,
                            bytes: size_of::<SourceRebindResultV1>(),
                        },
                        current,
                        facts,
                    )?;
                    InvocationReceiptV7::SourceRebind(self.source_rebind_result_v1(
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
