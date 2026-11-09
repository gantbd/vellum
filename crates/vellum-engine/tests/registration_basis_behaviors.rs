//! Conditional metadata authority, not provider execution or recipient raw reads.
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, Disposition, NativeTimingProfileV1, Recorder};
use vellum_engine::resource::ResourceError;
use vellum_engine::source::Limits;
#[path = "support/resource_catalog_fixture.rs"]
mod support;
use support::*;

fn owner() -> SourceCoordinator {
    SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 8,
        max_participants: 8,
        max_staged_bytes: 4096,
        source: Limits {
            max_source_bytes: 1024,
            max_edits: 16,
            max_edit_payload_bytes: 4096,
        },
    })
}
fn scope() -> ResourceRegistrationBasisScopeV1 {
    ResourceRegistrationBasisScopeV1 {
        authority: "a".into(),
        engine: "engine".into(),
        recipient: "agent".into(),
        purpose: "edit".into(),
        destination: "local".into(),
        policy: "policy".into(),
        continuity: "session".into(),
        profile: NATIVE_REGISTRATION_BASIS_PROFILE_V1.into(),
    }
}
fn basis_limits() -> ResourceRegistrationBasisLimitsV1 {
    ResourceRegistrationBasisLimitsV1 {
        max_bases: 4,
        max_issued: 8,
        max_root_bytes: 1_000_000,
        max_profile_bytes: 4096,
        max_scope_bytes: 4096,
        max_tag_bytes: 128,
        max_slot_bytes: 4096,
        max_basis_bytes: 16384,
        max_use_bytes: 8192,
        max_reply_bytes: 1024,
        max_held_bytes: 262144,
    }
}
fn setup_facts() -> TrustedResourceRegistrationBasisSetupV1 {
    TrustedResourceRegistrationBasisSetupV1 {
        owner_control_allowed: Some(true),
        resources_ready: Some(true),
        preparation_reserved: Some(true),
    }
}
fn current(c: &ResourceRegistrationBasisScopeV1) -> TrustedResourceRegistrationBasisFactsV1<'_> {
    TrustedResourceRegistrationBasisFactsV1 {
        scope: c,
        native_profile: limits(),
        authorized: Some(true),
        scope_current: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        registration_allowed: Some(true),
        identity_allowed: Some(true),
        registry_coverage_complete: Some(true),
        resources_ready: Some(true),
        retained: Some(true),
        reply_ready: Some(true),
        basis_allowed: Some(true),
        presence_observable: Some(true),
        equality_observable: Some(true),
        slot_allowed: Some(true),
        owner_control_allowed: Some(true),
        preparation_reserved: Some(true),
        interpretation_current: Some(true),
        host_basis_allowed: Some(true),
    }
}
fn prepare_owner() -> (
    SourceCoordinator,
    Recorder,
    ResourceRegistrationBasisScopeV1,
) {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let c = scope();
    o.configure_retention_budget(1_000_000, &mut r).unwrap();
    o.configure_resource_catalog_v1(&[], limits(), &facts(), &mut r)
        .unwrap();
    o.configure_registration_bases_v1(&c, limits(), basis_limits(), &setup_facts(), &mut r)
        .unwrap();
    (o, r, c)
}

#[test]
fn given_an_absent_basis_when_the_target_cycles_then_absence_is_a_predicate_not_an_epoch() {
    let (mut o, mut r, c) = prepare_owner();
    let slot = address(ResourceCatalogTableV1::Read, "target");
    let b = o
        .capture_registration_basis_v1(&c, &slot, &current(&c), &mut r)
        .unwrap();
    assert_eq!(b.condition, ResourceRegistrationConditionV1::Absent);
    let duplicate = o
        .capture_registration_basis_v1(&c, &slot, &current(&c), &mut r)
        .unwrap();
    assert_eq!(b.basis, duplicate.basis);
    assert_eq!(o.registration_basis_usage_v1().unwrap().issued_lifetime, 1);
    let p = o
        .prepare_resource_catalog_v1(
            &[install(ResourceCatalogTableV1::Read, "target")],
            &facts(),
            &mut r,
        )
        .unwrap();
    let out = o.commit_resource_catalog_v1(p, &facts(), &mut r).unwrap();
    assert_eq!(
        o.inspect_registration_basis_v1(&b.basis, &c, &current(&c), &mut r),
        Err(ResourceError::Unavailable)
    );
    let p = o
        .prepare_resource_catalog_v1(
            &[ResourceCatalogChangeV1 {
                address: slot,
                expected: CatalogExpectationV1::Present {
                    generation: out.registrations[0].generation.unwrap(),
                },
                action: ResourceCatalogActionV1::Remove,
            }],
            &facts(),
            &mut r,
        )
        .unwrap();
    o.commit_resource_catalog_v1(p, &facts(), &mut r).unwrap();
    assert_eq!(
        o.inspect_registration_basis_v1(&b.basis, &c, &current(&c), &mut r)
            .unwrap(),
        ResourceRegistrationConditionV1::Absent
    );
}

#[test]
fn given_a_present_basis_when_unrelated_source_or_catalog_changes_then_only_its_full_row_matters() {
    let (mut o, mut r, c) = prepare_owner();
    let p = o
        .prepare_resource_catalog_v1(
            &[install(ResourceCatalogTableV1::Write, "target")],
            &facts(),
            &mut r,
        )
        .unwrap();
    o.commit_resource_catalog_v1(p, &facts(), &mut r).unwrap();
    let b = o
        .capture_registration_basis_v1(
            &c,
            &address(ResourceCatalogTableV1::Write, "target"),
            &current(&c),
            &mut r,
        )
        .unwrap();
    o.create(b"prose", &mut r).unwrap();
    let p = o
        .prepare_resource_catalog_v1(
            &[install(ResourceCatalogTableV1::Read, "other")],
            &facts(),
            &mut r,
        )
        .unwrap();
    o.commit_resource_catalog_v1(p, &facts(), &mut r).unwrap();
    assert_eq!(
        o.inspect_registration_basis_v1(&b.basis, &c, &current(&c), &mut r)
            .unwrap(),
        ResourceRegistrationConditionV1::Present
    );
    let mut denied = current(&c);
    denied.host_basis_allowed = None;
    assert!(matches!(
        o.resolve_registration_basis_for_host_v1(&b.basis, &c, &denied, &mut r),
        Err(ResourceError::Unavailable)
    ));
    let value = o
        .resolve_registration_basis_for_host_v1(&b.basis, &c, &current(&c), &mut r)
        .unwrap();
    assert_eq!(value.row.unwrap().generation, 1);
    assert!(!format!("{b:?}").contains("target"));
}

#[test]
fn given_disabled_or_saturated_capture_when_bases_are_used_then_authority_and_publication_are_unchanged()
 {
    for capacity in [0, 1, 128] {
        let mut o = owner();
        let mut r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let c = scope();
        o.configure_retention_budget(1_000_000, &mut r).unwrap();
        o.configure_resource_catalog_v1(&[], limits(), &facts(), &mut r)
            .unwrap();
        let before = o.generation();
        o.configure_registration_bases_v1(&c, limits(), basis_limits(), &setup_facts(), &mut r)
            .unwrap();
        let b = o
            .capture_registration_basis_v1(
                &c,
                &address(ResourceCatalogTableV1::Read, "PRIVATE_TARGET"),
                &current(&c),
                &mut r,
            )
            .unwrap();
        assert_eq!(
            o.inspect_registration_basis_v1(&b.basis, &c, &current(&c), &mut r)
                .unwrap(),
            ResourceRegistrationConditionV1::Absent
        );
        let mut denied = current(&c);
        denied.equality_observable = None;
        assert_eq!(
            o.inspect_registration_basis_v1(&b.basis, &c, &denied, &mut r),
            Err(ResourceError::Unavailable)
        );
        o.resolve_registration_basis_for_host_v1(&b.basis, &c, &current(&c), &mut r)
            .unwrap();
        let f = TrustedResourceRegistrationBasisControlV1 {
            owner_control_allowed: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        };
        o.retire_registration_basis_v1(&b.basis, &f, &mut r)
            .unwrap();
        o.retire_registration_basis_issuer_v1(&f, &mut r).unwrap();
        assert_eq!(o.generation(), before);
        assert_eq!(o.retention_usage().reserved, 0);
        assert!(r.queued() <= capacity);
        if capacity == 128 {
            let mut seen = Vec::new();
            let mut refused = false;
            while let Some(e) = r.pop_front() {
                assert!(!format!("{e:?}").contains("PRIVATE_TARGET"));
                if matches!(
                    e.boundary,
                    Boundary::RegistrationBasisConfiguration
                        | Boundary::RegistrationBasisCapture
                        | Boundary::RegistrationBasisInspection
                        | Boundary::RegistrationBasisHostResolution
                        | Boundary::RegistrationBasisRetirement
                ) {
                    assert!(e.service_time.is_some());
                    seen.push(e.boundary);
                    refused |= e.disposition == Disposition::Refused;
                }
            }
            assert!(refused);
            for boundary in [
                Boundary::RegistrationBasisConfiguration,
                Boundary::RegistrationBasisCapture,
                Boundary::RegistrationBasisInspection,
                Boundary::RegistrationBasisHostResolution,
                Boundary::RegistrationBasisRetirement,
            ] {
                assert!(seen.contains(&boundary));
            }
        }
    }
}
