//! Explicit test configuration, not SDK defaults or production authority.
use std::mem::size_of;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
#[derive(Clone, Copy)]
pub struct Profile {
    pub epoch: u64,
    pub maximum: usize,
    pub capabilities: InvocationReceiptCapabilitiesV1,
}
pub(super) fn policy(c: &SettingsExecutionContextV1, epoch: u64) -> InvocationObservationPolicyV1 {
    InvocationObservationPolicyV1 {
        recipient: c.recipient.clone(),
        epoch,
        grant: true,
        publication: true,
        control: true,
        effect: true,
        reasons: true,
        details: false,
        recovery: true,
    }
}
pub(super) fn facts(c: &SettingsExecutionContextV1) -> TrustedInvocationPolicyFactsV1<'_> {
    TrustedInvocationPolicyFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        resources_ready: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        preparation_reserved: Some(true),
        registry_ready: Some(true),
        retention_ready: Some(true),
    }
}
impl Profile {
    pub fn bind(
        self,
        o: &mut SourceCoordinator,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        r: &mut Recorder,
    ) {
        o.bind_invocation_observation_policy_v1(
            id,
            c,
            &policy(c, self.epoch),
            InvocationObservationPolicyLimitsV1 {
                max_context_bytes: 2048,
                max_result_bytes: self.maximum,
            },
            &facts(c),
            r,
        )
        .unwrap();
    }
    pub fn open(
        self,
        o: &mut SourceCoordinator,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        r: &mut Recorder,
    ) -> InvocationObservationContextV1 {
        self.try_open(o, id, c, r).unwrap()
    }
    pub fn try_open(
        self,
        o: &mut SourceCoordinator,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        r: &mut Recorder,
    ) -> Result<InvocationObservationContextV1, InvocationError> {
        let mut limits = InvocationObservationLimitsV1 {
            max_waiters: 0,
            max_issued: 0,
            max_wait_steps: 0,
            max_context_bytes: 2048,
            max_wait_bytes: InvocationObservationLimitsV1::required_wait_bytes(),
            max_reference_bytes: size_of::<InvocationObservationReferenceV1>()
                .max(size_of::<InvocationControlReferenceV1>()),
            max_reply_bytes: InvocationReceiptCapabilitiesV1::required_reply_bytes(self.maximum)
                .unwrap()
                .max(self.maximum + size_of::<InvocationControlObservationV1>())
                .max(size_of::<InvocationWaitReplyV1>()),
            max_control_reply_bytes: size_of::<InvocationControlObservationV1>(),
            max_held_bytes: 0,
        };
        limits.max_held_bytes = limits.required_hold_bytes().unwrap();
        let f = facts(c);
        o.open_invocation_observation_with_receipts_v1(
            id,
            c,
            &policy(c, self.epoch),
            limits,
            InvocationReceiptAdmissionV1 {
                observation: &f,
                capabilities: self.capabilities,
                operation_receipt_admission_allowed: Some(true),
            },
            r,
        )
    }
    pub fn query<'a>(
        self,
        o: &'a SourceCoordinator,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV1<'a>, InvocationError> {
        let f = TrustedInvocationObservationReleaseV1 {
            context: c,
            authorized: Some(true),
            scope: Some(true),
            continuous: Some(true),
            live: Some(true),
            retained: Some(true),
            transfer_ready: Some(true),
            control_ready: Some(true),
        };
        let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
            current: &f,
            ordinary_ready: Some(true),
            resources_ready: Some(true),
        };
        o.query_invocation_receipt_v1(
            observer,
            request,
            &policy(c, self.epoch),
            &TrustedInvocationReceiptReleaseV1 {
                observation: &ordinary,
                operation_receipt_allowed: Some(true),
            },
            r,
        )
    }
}
