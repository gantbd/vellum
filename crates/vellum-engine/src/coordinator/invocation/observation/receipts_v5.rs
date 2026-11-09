//! Explicit historical live-relation results; prior exhaustive profiles unchanged.
use super::*;
#[derive(Clone, Copy)]
pub enum InvocationReceiptRequestV5<'a> {
    Existing(InvocationReceiptRequestV4<'a>),
    LiveRelation(&'a TrustedLiveRelationResultV1<'a>),
}
pub enum InvocationReceiptV5<'a> {
    Existing(InvocationReceiptV4<'a>),
    LiveRelation(LiveRelationResultV1),
}
impl InvocationReceiptV5<'_> {
    pub fn invocation(&self) -> &InvocationId {
        match self {
            Self::Existing(v) => v.invocation(),
            Self::LiveRelation(v) => &v.invocation,
        }
    }
    /// Complete conservative native envelope; not wire/RSS accounting.
    pub fn required_reply_bytes(maximum: usize) -> Result<usize, InvocationError> {
        maximum
            .checked_add(size_of::<InvocationReceiptV5<'static>>())
            .ok_or(InvocationError::Limit)
    }
}
impl std::fmt::Debug for InvocationReceiptV5<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationReceiptV5(<redacted>)")
    }
}
impl SourceCoordinator {
    pub fn open_invocation_observation_with_receipts_v5(
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
            ReceiptProfile::V5,
            r,
        )
    }
    pub fn query_invocation_receipt_v5<'a>(
        &'a self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV5<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV5<'a>, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if observer.receipt_profile != ReceiptProfile::V5 {
                return Err(InvocationError::Unavailable);
            }
            self.query_invocation_receipt_native_v5(observer, request, current, facts, r)
        })();
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
    pub(super) fn query_invocation_receipt_native_v5<'a>(
        &'a self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV5<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV5<'a>, InvocationError> {
        let value = match request {
            InvocationReceiptRequestV5::Existing(request) => InvocationReceiptV5::Existing(
                self.query_invocation_receipt_native_v4(observer, request, current, facts, r)?,
            ),
            InvocationReceiptRequestV5::LiveRelation(native) => {
                self.validate_lifecycle_receipt_release(
                    observer,
                    receipts::NativeReceiptFrame {
                        context: native.context,
                        epoch: native.publication_epoch,
                        bytes: size_of::<LiveRelationResultV1>(),
                    },
                    current,
                    facts,
                )?;
                InvocationReceiptV5::LiveRelation(self.live_relation_result_v1(
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
