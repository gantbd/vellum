//! Explicit native test profiles, not SDK defaults or production authority.
#![allow(dead_code)]
use vellum_engine::coordinator::*;
#[path = "annotation_read_fixture.rs"]
mod fixture;
pub(crate) use fixture::Fixture;
pub(crate) fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "host".into(),
        engine: "engine".into(),
        recipient: "owner".into(),
        purpose: "release".into(),
        destination: "local".into(),
        profile: "native".into(),
        continuity: "session".into(),
    }
}
pub(crate) fn facts(c: &SettingsExecutionContextV1) -> TrustedRetainedReleaseFactsV1<'_> {
    TrustedRetainedReleaseFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        release_allowed: Some(true),
        resources_ready: Some(true),
        preparation_reserved: Some(true),
    }
}
pub(crate) fn admission<'a>(
    f: &'a TrustedRetainedReleaseFactsV1<'a>,
) -> RetainedReleaseAdmissionV1<'a> {
    RetainedReleaseAdmissionV1 {
        facts: f,
        limits: RetainedReleaseLimitsV1 {
            max_capture_bytes: 4096,
            max_result_bytes: 512,
        },
        publication_epoch: 7,
        publication_allowed: Some(true),
        result_admission_allowed: Some(true),
        result_allowed: Some(true),
    }
}
pub(crate) fn release(c: &SettingsExecutionContextV1) -> TrustedRetainedReleaseResultV1<'_> {
    TrustedRetainedReleaseResultV1 {
        context: c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        outcome_readable: Some(true),
        result_allowed: Some(true),
        result_context_current: Some(true),
        historical_document_readable: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
    }
}
pub(crate) fn disposed() -> Fixture {
    disposed_with_retention_budget(131072)
}
pub(crate) fn disposed_with_retention_budget(maximum: usize) -> Fixture {
    let mut f = Fixture::with_invocations();
    f.owner
        .configure_retention_budget(maximum, &mut f.recorder)
        .unwrap();
    let p = f
        .owner
        .prepare_retained_disposal(&f.doc, 0, &mut f.recorder)
        .unwrap();
    f.owner
        .commit_retained_disposal(p, &mut f.recorder)
        .unwrap();
    f
}
pub(crate) fn request(
    f: &mut Fixture,
    operation: RetainedReleaseOperationV1,
) -> RetainedReleaseRequestV1 {
    RetainedReleaseRequestV1 {
        document: f.doc.document.clone(),
        expected: f
            .owner
            .lifecycle_state(&f.doc.document, &mut f.recorder)
            .unwrap(),
        operation,
    }
}
