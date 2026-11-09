//! Explicit test configuration and trusted premises, not SDK defaults/authority.
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::source::Limits;
pub(crate) fn owner() -> (SourceCoordinator, Recorder) {
    owner_with_retention_budget(1048576)
}
pub(crate) fn owner_with_retention_budget(bytes: usize) -> (SourceCoordinator, Recorder) {
    let mut o = SourceCoordinator::with_invocation_and_history_limits(
        CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 4096,
                max_edits: 16,
                max_edit_payload_bytes: 8192,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 32,
            max_retained: 32,
            max_intent_bytes: 32768,
        },
        HistoryLimits {
            max_retained_groups: 16,
            max_journal_bytes: 65536,
            max_pins: 8,
        },
    );
    let mut r = Recorder::disabled();
    o.configure_semantic_root_budget(1048576, &mut r).unwrap();
    o.configure_retention_budget(bytes, &mut r).unwrap();
    (o, r)
}
pub(crate) fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "p".into(),
        destination: "d".into(),
        profile: "owned-scratch/1".into(),
        continuity: "c".into(),
    }
}
pub(crate) fn request(source: &[u8]) -> ScratchDocumentRequestV1<'_> {
    ScratchDocumentRequestV1 {
        source,
        serialization: SourceSerializationV1 {
            profile: SourceSerializationProfileV1::StrictUtf8OneBom,
            bom: false,
        },
        insertion_policy: GeneratedNewlineV1::CrLf,
    }
}
pub(crate) fn facts(c: &SettingsExecutionContextV1) -> TrustedScratchFactsV1<'_> {
    TrustedScratchFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        interpretation_current: Some(true),
        resources_ready: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        lifecycle_allowed: Some(true),
        coverage_complete: Some(true),
        creation_allowed: Some(true),
    }
}
pub(crate) fn admission<'a>(f: &'a TrustedScratchFactsV1<'a>) -> ScratchInvocationAdmissionV1<'a> {
    ScratchInvocationAdmissionV1 {
        facts: f,
        limits: ScratchInvocationLimitsV1 {
            max_capture_bytes: 8192,
            max_result_bytes: 512,
        },
        publication_epoch: 7,
        publication_allowed: Some(true),
        result_admission_allowed: Some(true),
        result_identifiers_allowed: Some(true),
    }
}
pub(crate) fn release(c: &SettingsExecutionContextV1) -> TrustedScratchResultV1<'_> {
    TrustedScratchResultV1 {
        context: c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        outcome_readable: Some(true),
        result_readable: Some(true),
        result_context_current: Some(true),
        historical_document_readable: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        max_result_bytes: 512,
    }
}
pub(crate) fn control(c: &SettingsExecutionContextV1) -> InvocationObservationPolicyControlV1<'_> {
    InvocationObservationPolicyControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
