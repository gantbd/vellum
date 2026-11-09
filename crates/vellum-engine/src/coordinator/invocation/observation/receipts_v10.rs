//! Original Save As activation results; published V1–V9 enums stay exhaustive.
use super::*;
#[derive(Clone, Copy)]
pub enum InvocationReceiptRequestV10<'a> {
    Existing(InvocationReceiptRequestV9<'a>),
    SourceSaveAs(&'a TrustedSourceSaveAsResultV1<'a>),
}
pub enum InvocationReceiptV10<'a> {
    Existing(InvocationReceiptV9<'a>),
    SourceSaveAs(SourceSaveAsResultV1),
}
impl InvocationReceiptV10<'_> {
    pub fn invocation(&self) -> &InvocationId {
        match self {
            Self::Existing(v) => v.invocation(),
            Self::SourceSaveAs(v) => &v.invocation,
        }
    }
    /// Complete native envelope, not wire or allocator/RSS accounting.
    pub fn required_reply_bytes(maximum: usize) -> Result<usize, InvocationError> {
        maximum
            .checked_add(size_of::<InvocationReceiptV10<'static>>())
            .ok_or(InvocationError::Limit)
    }
}
impl std::fmt::Debug for InvocationReceiptV10<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationReceiptV10(<redacted>)")
    }
}
impl SourceCoordinator {
    pub fn open_invocation_observation_with_receipts_v10(
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
            ReceiptProfile::V10,
            r,
        )
    }
    pub fn query_invocation_receipt_v10<'a>(
        &'a self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV10<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV10<'a>, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if observer.receipt_profile != ReceiptProfile::V10 {
                return Err(InvocationError::Unavailable);
            }
            let value = match request {
                InvocationReceiptRequestV10::Existing(q) => InvocationReceiptV10::Existing(
                    self.query_invocation_receipt_native_v9(observer, q, current, facts, r)?,
                ),
                InvocationReceiptRequestV10::SourceSaveAs(native) => {
                    self.validate_lifecycle_receipt_release(
                        observer,
                        receipts::NativeReceiptFrame {
                            context: native.context,
                            epoch: native.publication_epoch,
                            bytes: size_of::<SourceSaveAsResultV1>(),
                        },
                        current,
                        facts,
                    )?;
                    InvocationReceiptV10::SourceSaveAs(self.source_save_as_result_v1(
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
        capture_timed(
            r,
            Boundary::InvocationReceiptRelease,
            &result,
            timing.finish(),
        );
        result
    }
}
