//! Owned engine metadata jobs; provider execution and recipient wire remain out.
#[path = "support/invocation_observation_fixture.rs"]
mod observation_support;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1, Recorder};
use vellum_engine::source::Limits;
#[path = "resource_registration_behaviors/arguments.rs"]
mod arguments;
#[path = "resource_registration_behaviors/context_dispatch.rs"]
mod context_dispatch;
#[path = "support/discovered_context_fixture.rs"]
mod discovered_context;
#[path = "support/scoped_registration_envelope.rs"]
mod envelope;
#[path = "support/resource_catalog_fixture.rs"]
mod support;

#[test]
fn f06_receipt_assembly_keeps_host_registration_delivery_separate_from_scoped_references() {
    let (mut o, mut r) = setup();
    o.configure_retention_budget(131072, &mut r).unwrap();
    let c = context();
    let id = o.issue_invocation(&mut r).unwrap();
    let profile = observation_support::Profile {
        epoch: 7,
        maximum: 8192,
        capabilities: InvocationReceiptCapabilitiesV1 {
            host_registration: true,
            ..Default::default()
        },
    };
    profile.bind(&mut o, &id, &c, &mut r);
    let native = support::facts();
    let batch = [support::install(
        ResourceCatalogTableV1::Read,
        "PRIVATE_ROW",
    )];
    o.submit_resource_registration_v1(&id, &batch, &c, &host(&c, &native), &mut r)
        .unwrap();
    let mut observer = profile.open(&mut o, &id, &c, &mut r);
    o.run_resource_registration_v1(&id, &c, &host(&c, &native), &mut r)
        .unwrap();
    let f = release(&c);
    let InvocationReceiptV1::HostRegistration(out) = profile
        .query(
            &o,
            &mut observer,
            InvocationReceiptRequestV1::HostRegistration(&f),
            &c,
            &mut r,
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(out.invocation, id);
    assert_eq!(out.result.registrations.len(), 1);
}

#[test]
fn f06_receipt_assembly_refuses_a_native_registration_ceiling_larger_than_original_policy_funding()
{
    let (mut o, mut r) = setup();
    o.configure_retention_budget(131072, &mut r).unwrap();
    let c = context();
    let id = o.issue_invocation(&mut r).unwrap();
    let profile = observation_support::Profile {
        epoch: 7,
        maximum: 8191,
        capabilities: InvocationReceiptCapabilitiesV1 {
            host_registration: true,
            ..Default::default()
        },
    };
    profile.bind(&mut o, &id, &c, &mut r);
    let native = support::facts();
    let batch = [support::install(
        ResourceCatalogTableV1::Read,
        "PRIVATE_ROW",
    )];
    o.submit_resource_registration_v1(&id, &batch, &c, &host(&c, &native), &mut r)
        .unwrap();
    let held = o.retention_usage().reserved;
    assert_eq!(
        profile.try_open(&mut o, &id, &c, &mut r).unwrap_err(),
        InvocationError::Limit
    );
    assert_eq!(o.retention_usage().reserved, held);
    assert_eq!(
        o.invocation_state(&id, &mut r),
        Ok(InvocationState::Pending)
    );
}
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "engine".into(),
        recipient: "host".into(),
        purpose: "register".into(),
        destination: "host".into(),
        continuity: "c".into(),
        profile: NATIVE_RESOURCE_REGISTRATION_PROFILE_V1.into(),
    }
}
fn host<'a>(
    c: &'a SettingsExecutionContextV1,
    n: &'a TrustedResourceCatalogFactsV1,
) -> TrustedResourceRegistrationInvocationV1<'a> {
    TrustedResourceRegistrationInvocationV1 {
        context: c,
        profile: support::limits(),
        max_result_bytes: 8192,
        interpretation_current: Some(true),
        host_result_allowed: Some(true),
        native: n,
    }
}
fn release(c: &SettingsExecutionContextV1) -> TrustedResourceRegistrationResultV1<'_> {
    TrustedResourceRegistrationResultV1 {
        context: c,
        max_result_bytes: 8192,
        outcome_readable: Some(true),
        registration_result_readable: Some(true),
        registration_result_context_current: Some(true),
        host_result_allowed: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
    }
}
fn setup() -> (SourceCoordinator, Recorder) {
    let mut o = SourceCoordinator::with_invocation_limits(
        CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 16,
                max_edit_payload_bytes: 4096,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 32,
            max_retained: 16,
            max_intent_bytes: 32768,
        },
    );
    let mut r = Recorder::disabled();
    o.configure_resource_catalog_v1(&[], support::limits(), &support::facts(), &mut r)
        .unwrap();
    (o, r)
}

#[test]
fn given_an_owned_registration_when_unrelated_state_changes_then_service_reprepares_and_replay_never_reexecutes()
 {
    let (mut o, mut r) = setup();
    let c = context();
    let n = support::facts();
    let batch = [support::install(
        ResourceCatalogTableV1::Read,
        "annotations",
    )];
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_resource_registration_v1(&id, &batch, &c, &host(&c, &n), &mut r)
        .unwrap();
    let source = o.create(b"English / Deutsch", &mut r).unwrap();
    let p = o
        .prepare_resource_catalog_v1(
            &[support::install(ResourceCatalogTableV1::Write, "other")],
            &n,
            &mut r,
        )
        .unwrap();
    o.commit_resource_catalog_v1(p, &n, &mut r).unwrap();
    let before = o.generation();
    o.run_resource_registration_v1(&id, &c, &host(&c, &n), &mut r)
        .unwrap();
    assert_eq!(o.generation(), before + 1);
    assert_eq!(
        o.read(&source.document, &mut r).unwrap().source.text(),
        "English / Deutsch"
    );
    let result = o
        .retained_resource_registration_result_v1(&id, &release(&c), &mut r)
        .unwrap();
    assert_eq!(result.invocation, id);
    assert_eq!(result.result.registrations[0].generation, Some(2));
    assert_eq!(
        o.submit_resource_registration_v1(&id, &batch, &c, &host(&c, &n), &mut r),
        Ok(InvocationState::Finished)
    );
    o.run_resource_registration_v1(&id, &c, &host(&c, &n), &mut r)
        .unwrap();
    assert_eq!(o.generation(), before + 1);
    let mut denied = release(&c);
    denied.registration_result_readable = None;
    assert!(
        o.retained_resource_registration_result_v1(&id, &denied, &mut r)
            .is_err()
    );
    assert!(
        o.retained_resource_registration_result_v1(&id, &release(&c), &mut r)
            .is_ok()
    );
}

#[test]
fn given_a_queued_registration_when_protected_stop_is_requested_then_lost_edit_rights_do_not_prevent_stop_settlement()
 {
    let (mut o, mut r) = setup();
    let c = context();
    let n = support::facts();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_resource_registration_v1(
        &id,
        &[support::install(ResourceCatalogTableV1::Read, "r")],
        &c,
        &host(&c, &n),
        &mut r,
    )
    .unwrap();
    let before = o.generation();
    o.request_invocation_stop(
        &id,
        &TrustedInvocationControlV1 {
            context: &c,
            owner_control_allowed: Some(true),
            control_context_current: Some(true),
            control_scope_valid: Some(true),
            protected_control_ready: Some(true),
        },
        &mut r,
    )
    .unwrap();
    let lost = TrustedResourceCatalogFactsV1::default();
    o.run_resource_registration_v1(&id, &c, &host(&c, &lost), &mut r)
        .unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r),
        Ok(InvocationOutcome::StoppedBeforePublication)
    ));
    assert_eq!(o.generation(), before);
    assert!(
        o.resource_catalog_registration_v1(
            &support::address(ResourceCatalogTableV1::Read, "r"),
            &mut r
        )
        .unwrap()
        .is_none()
    );
}

#[test]
fn given_disabled_saturated_or_available_diagnostics_when_a_registration_runs_then_publication_and_result_are_once_only()
 {
    for capacity in [0, 1, 256] {
        let (mut o, _) = setup();
        let mut r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let c = context();
        let n = support::facts();
        let batch = [support::install(ResourceCatalogTableV1::Read, "r")];
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_resource_registration_v1(&id, &batch, &c, &host(&c, &n), &mut r)
            .unwrap();
        o.run_resource_registration_v1(&id, &c, &host(&c, &n), &mut r)
            .unwrap();
        let result = o
            .retained_resource_registration_result_v1(&id, &release(&c), &mut r)
            .unwrap();
        assert_eq!(result.result.status, ResourceCatalogStatusV1::Applied);
        assert_eq!(result.result.registrations[0].generation, Some(1));
        o.run_resource_registration_v1(&id, &c, &host(&c, &n), &mut r)
            .unwrap();
        assert!(r.queued() <= capacity);
        if capacity == 256 {
            let mut stages = Vec::new();
            let mut timed = false;
            while let Some(e) = r.pop_front() {
                stages.push(e.boundary);
                if e.boundary == Boundary::ResourceRegistrationService {
                    timed |= e.service_time.is_some();
                }
            }
            for b in [
                Boundary::InvocationAdmission,
                Boundary::ResourceRegistrationService,
                Boundary::ResourceRegistrationResult,
                Boundary::InvocationReplay,
            ] {
                assert!(stages.contains(&b));
            }
            assert_eq!(
                stages
                    .iter()
                    .filter(|b| **b == Boundary::ResourceCatalogPublication)
                    .count(),
                1
            );
            assert!(timed);
        }
    }
}
