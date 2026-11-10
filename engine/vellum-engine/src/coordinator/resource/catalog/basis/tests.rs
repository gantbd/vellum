use super::*;
#[test]
fn shutdown_inventory_captures_actual_registration_basis_records_and_original_envelope() {
    use crate::coordinator::shutdown::tests::inventory_owner;
    let (mut o, mut r, c) = setup(&[], profile());
    capture(&mut o, &c, &mut r);
    let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::RegistrationBases, &[]);
    assert_eq!(row.retained_records, Some(1));
    assert_eq!(row.reserved_bytes, Some(profile().max_held_bytes));
}
use crate::coordinator::resource::catalog::tests::{address, bare, config, facts, limits};
mod shutdown;
fn scope() -> ResourceRegistrationBasisScopeV1 {
    ResourceRegistrationBasisScopeV1 {
        authority: "a".into(),
        engine: "engine".into(),
        recipient: "agent".into(),
        purpose: "mutate".into(),
        destination: "local".into(),
        policy: "policy".into(),
        continuity: "session".into(),
        profile: NATIVE_REGISTRATION_BASIS_PROFILE_V1.into(),
    }
}
fn profile() -> ResourceRegistrationBasisLimitsV1 {
    ResourceRegistrationBasisLimitsV1 {
        max_bases: 2,
        max_issued: 3,
        max_root_bytes: 1_000_000,
        max_profile_bytes: 4096,
        max_scope_bytes: 4096,
        max_tag_bytes: 128,
        max_slot_bytes: 4096,
        max_basis_bytes: 8192,
        max_use_bytes: 8192,
        max_reply_bytes: 1024,
        max_held_bytes: 131072,
    }
}
fn setup_facts() -> TrustedResourceRegistrationBasisSetupV1 {
    TrustedResourceRegistrationBasisSetupV1 {
        owner_control_allowed: Some(true),
        resources_ready: Some(true),
        preparation_reserved: Some(true),
    }
}
fn control() -> TrustedResourceRegistrationBasisControlV1 {
    TrustedResourceRegistrationBasisControlV1 {
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
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
fn setup(
    initial: &[ResourceCatalogRowV1],
    p: ResourceRegistrationBasisLimitsV1,
) -> (
    SourceCoordinator,
    Recorder,
    ResourceRegistrationBasisScopeV1,
) {
    let mut o = bare();
    let mut r = Recorder::disabled();
    let c = scope();
    o.configure_resource_catalog_v1(initial, limits(), &facts(), &mut r)
        .unwrap();
    o.configure_retention_budget(1_000_000, &mut r).unwrap();
    o.configure_registration_bases_v1(&c, limits(), p, &setup_facts(), &mut r)
        .unwrap();
    (o, r, c)
}
fn capture(
    o: &mut SourceCoordinator,
    c: &ResourceRegistrationBasisScopeV1,
    r: &mut Recorder,
) -> ResourceRegistrationBasisCaptureV1 {
    o.capture_registration_basis_v1(
        c,
        &address(ResourceCatalogTableV1::Read, "target"),
        &current(c),
        r,
    )
    .unwrap()
}
fn apply(
    o: &mut SourceCoordinator,
    expected: CatalogExpectationV1,
    action: ResourceCatalogActionV1,
    r: &mut Recorder,
) {
    let p = o
        .prepare_resource_catalog_v1(
            &[ResourceCatalogChangeV1 {
                address: address(ResourceCatalogTableV1::Read, "target"),
                expected,
                action,
            }],
            &facts(),
            r,
        )
        .unwrap();
    o.commit_resource_catalog_v1(p, &facts(), r).unwrap();
}

#[test]
fn every_current_gate_denies_false_and_unknown_without_revoking_or_spending() {
    let (mut o, mut r, c) = setup(&[], profile());
    let b = capture(&mut o, &c, &mut r);
    for i in 0..18 {
        for denied in [Some(false), None] {
            let mut f = current(&c);
            let gates = [
                &mut f.authorized,
                &mut f.scope_current,
                &mut f.continuous,
                &mut f.live,
                &mut f.not_stopped,
                &mut f.registration_allowed,
                &mut f.identity_allowed,
                &mut f.registry_coverage_complete,
                &mut f.resources_ready,
                &mut f.retained,
                &mut f.reply_ready,
                &mut f.basis_allowed,
                &mut f.presence_observable,
                &mut f.equality_observable,
                &mut f.slot_allowed,
                &mut f.owner_control_allowed,
                &mut f.preparation_reserved,
                &mut f.interpretation_current,
            ];
            *gates.into_iter().nth(i).unwrap() = denied;
            assert_eq!(
                o.inspect_registration_basis_v1(&b.basis, &c, &f, &mut r),
                Err(ResourceError::Unavailable),
                "gate {i}"
            );
            assert!(matches!(
                o.capture_registration_basis_v1(
                    &c,
                    &address(ResourceCatalogTableV1::Read, "other"),
                    &f,
                    &mut r
                ),
                Err(ResourceError::Unavailable)
            ));
            assert!(matches!(
                o.resolve_registration_basis_for_host_v1(&b.basis, &c, &f, &mut r),
                Err(ResourceError::Unavailable)
            ));
            assert_eq!(o.registration_basis_usage_v1().unwrap().issued_lifetime, 1);
        }
    }
    for denied in [Some(false), None] {
        let mut f = current(&c);
        f.host_basis_allowed = denied;
        assert!(matches!(
            o.resolve_registration_basis_for_host_v1(&b.basis, &c, &f, &mut r),
            Err(ResourceError::Unavailable)
        ));
        assert_eq!(
            o.inspect_registration_basis_v1(&b.basis, &c, &f, &mut r)
                .unwrap(),
            ResourceRegistrationConditionV1::Absent
        );
    }
    assert_eq!(capture(&mut o, &c, &mut r).basis, b.basis);
}

#[test]
fn complete_original_scope_domain_and_native_profile_are_required() {
    let (mut o, mut r, c) = setup(&[], profile());
    let b = capture(&mut o, &c, &mut r);
    let (other, _, _) = setup(&[], profile());
    assert_eq!(
        other.inspect_registration_basis_v1(&b.basis, &c, &current(&c), &mut r),
        Err(ResourceError::Unavailable)
    );
    for i in 0..8 {
        let mut different = c.clone();
        [
            &mut different.authority,
            &mut different.engine,
            &mut different.recipient,
            &mut different.purpose,
            &mut different.destination,
            &mut different.policy,
            &mut different.continuity,
            &mut different.profile,
        ][i]
            .push('x');
        assert_eq!(
            o.inspect_registration_basis_v1(&b.basis, &different, &current(&different), &mut r),
            Err(ResourceError::Unavailable)
        );
        assert_eq!(
            o.inspect_registration_basis_v1(&b.basis, &c, &current(&different), &mut r),
            Err(ResourceError::Unavailable)
        );
    }
    let mut f = current(&c);
    f.native_profile.max_relations += 1;
    assert_eq!(
        o.inspect_registration_basis_v1(&b.basis, &c, &f, &mut r),
        Err(ResourceError::Unavailable)
    );
    let forged = ResourceRegistrationBasisIdV1 {
        owner: o.namespace.clone(),
        marker: Arc::new(()),
    };
    assert_eq!(
        o.inspect_registration_basis_v1(&forged, &c, &current(&c), &mut r),
        Err(ResourceError::Unavailable)
    );
}

#[test]
fn zero_present_full_row_and_nonreuse_prevent_aba_resurrection() {
    let initial = [ResourceCatalogRowV1 {
        address: address(ResourceCatalogTableV1::Read, "target"),
        generation: 0,
        configuration: config(ResourceCatalogTableV1::Read, "key"),
    }];
    let (mut o, mut r, c) = setup(&initial, profile());
    let b = capture(&mut o, &c, &mut r);
    let resolved = o
        .resolve_registration_basis_for_host_v1(&b.basis, &c, &current(&c), &mut r)
        .unwrap();
    assert_eq!(
        resolved.expected,
        CatalogExpectationV1::Present { generation: 0 }
    );
    assert_eq!(b.condition, ResourceRegistrationConditionV1::Present);
    apply(
        &mut o,
        CatalogExpectationV1::Present { generation: 0 },
        ResourceCatalogActionV1::Remove,
        &mut r,
    );
    apply(
        &mut o,
        CatalogExpectationV1::Absent,
        ResourceCatalogActionV1::Install(config(ResourceCatalogTableV1::Read, "key")),
        &mut r,
    );
    assert_eq!(
        o.inspect_registration_basis_v1(&b.basis, &c, &current(&c), &mut r),
        Err(ResourceError::Unavailable)
    );
    let fresh = capture(&mut o, &c, &mut r);
    assert_ne!(fresh.basis, b.basis);
    assert_eq!(o.registration_basis_usage_v1().unwrap().issued_lifetime, 2);
    // Stable native stamp alone is insufficient: compare the full captured row.
    let root = o.resources.catalog.root.as_mut().unwrap();
    let root = Arc::get_mut(root).unwrap();
    root.rows[0].configuration.locator.push('x');
    assert_eq!(
        o.inspect_registration_basis_v1(&fresh.basis, &c, &current(&c), &mut r),
        Err(ResourceError::Unavailable)
    );
}

#[test]
fn bounded_live_and_lifetime_issuance_do_not_refund_or_charge_identical_recapture() {
    let p = ResourceRegistrationBasisLimitsV1 {
        max_bases: 1,
        max_issued: 2,
        ..profile()
    };
    let (mut o, mut r, c) = setup(&[], p);
    let b = capture(&mut o, &c, &mut r);
    assert_eq!(capture(&mut o, &c, &mut r).basis, b.basis);
    assert!(matches!(
        o.capture_registration_basis_v1(
            &c,
            &address(ResourceCatalogTableV1::Write, "target"),
            &current(&c),
            &mut r
        ),
        Err(ResourceError::Limit)
    ));
    let before = o.generation();
    let reserved = o.retention_usage().reserved;
    assert_eq!(
        o.retire_registration_basis_issuer_v1(&control(), &mut r),
        Err(ResourceError::Unavailable)
    );
    o.retire_registration_basis_v1(&b.basis, &control(), &mut r)
        .unwrap();
    assert_eq!(
        o.inspect_registration_basis_v1(&b.basis, &c, &current(&c), &mut r),
        Err(ResourceError::Unavailable)
    );
    let b2 = capture(&mut o, &c, &mut r);
    assert_ne!(b.basis, b2.basis);
    assert_eq!(capture(&mut o, &c, &mut r).basis, b2.basis);
    o.retire_registration_basis_v1(&b2.basis, &control(), &mut r)
        .unwrap();
    assert!(matches!(
        o.capture_registration_basis_v1(
            &c,
            &address(ResourceCatalogTableV1::Read, "target"),
            &current(&c),
            &mut r
        ),
        Err(ResourceError::Limit)
    ));
    assert_eq!(o.registration_basis_usage_v1().unwrap().issued_lifetime, 2);
    assert_eq!(o.retention_usage().reserved, reserved);
    assert_eq!(o.generation(), before);
    o.retire_registration_basis_issuer_v1(&control(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(
        o.configure_registration_bases_v1(&c, limits(), p, &setup_facts(), &mut r),
        Err(ResourceError::Unavailable)
    );
}

#[test]
fn protected_controls_work_after_fault_but_require_all_discharge_facts_and_original_owner() {
    let (mut o, mut r, c) = setup(&[], profile());
    let b = capture(&mut o, &c, &mut r);
    let (mut foreign, _, _) = setup(&[], profile());
    assert_eq!(
        foreign.retire_registration_basis_v1(&b.basis, &control(), &mut r),
        Err(ResourceError::Unavailable)
    );
    o.faulted = true;
    for i in 0..3 {
        for denied in [Some(false), None] {
            let mut f = control();
            [
                &mut f.owner_control_allowed,
                &mut f.retention_released,
                &mut f.delivery_drained,
            ][i]
                .clone_from(&denied);
            assert_eq!(
                o.retire_registration_basis_v1(&b.basis, &f, &mut r),
                Err(ResourceError::Unavailable)
            );
            assert_eq!(
                o.retire_registration_basis_issuer_v1(&f, &mut r),
                Err(ResourceError::Unavailable)
            );
        }
    }
    o.retire_registration_basis_v1(&b.basis, &control(), &mut r)
        .unwrap();
    assert_eq!(
        o.retire_registration_basis_v1(&b.basis, &control(), &mut r),
        Err(ResourceError::Unavailable)
    );
    o.retire_registration_basis_issuer_v1(&control(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert!(o.faulted);
}

#[test]
fn current_catalog_must_be_known_valid_and_profile_meaning_cannot_drift() {
    let mut o = bare();
    let mut r = Recorder::disabled();
    let c = scope();
    o.configure_retention_budget(1_000_000, &mut r).unwrap();
    assert_eq!(
        o.configure_registration_bases_v1(&c, limits(), profile(), &setup_facts(), &mut r),
        Err(ResourceError::Unavailable)
    );
    assert_eq!(o.retention_usage().reserved, 0);
    let (mut o, mut r, c) = setup(&[], profile());
    let b = capture(&mut o, &c, &mut r);
    o.resources.catalog.floor = MAX_STAMP + 1;
    assert_eq!(
        o.inspect_registration_basis_v1(&b.basis, &c, &current(&c), &mut r),
        Err(ResourceError::Faulted)
    );
    assert!(matches!(
        o.capture_registration_basis_v1(
            &c,
            &address(ResourceCatalogTableV1::Read, "other"),
            &current(&c),
            &mut r
        ),
        Err(ResourceError::Faulted)
    ));
    assert_eq!(o.registration_basis_usage_v1().unwrap().issued_lifetime, 1);
    o.retire_registration_basis_v1(&b.basis, &control(), &mut r)
        .unwrap();
    o.retire_registration_basis_issuer_v1(&control(), &mut r)
        .unwrap();
}

#[test]
fn every_native_envelope_bound_is_checked_before_issuance_and_exact_fit_works() {
    let (baseline, _, c) = setup(&[], profile());
    let slot = address(ResourceCatalogTableV1::Read, "target");
    let mut exact = profile();
    exact.max_scope_bytes = c.bytes(profile()).unwrap();
    exact.max_profile_bytes =
        size_of::<State>() + size_of::<Store>() + c.authority.len() + c.engine.len();
    exact.max_root_bytes = baseline.measure_semantic_root().unwrap().total_bytes;
    exact.max_slot_bytes = size_of::<ResourceCatalogAddressV1>() + slot.payload().unwrap();
    exact.max_basis_bytes = Record::bytes(exact.max_scope_bytes, &slot, None).unwrap();
    exact.max_use_bytes = Record::use_bytes(&slot, None).unwrap();
    exact.max_reply_bytes = size_of::<ResourceRegistrationBasisCaptureV1>();
    exact.max_held_bytes = exact.required_pool_bytes().unwrap();
    let (mut o, mut r, c) = setup(&[], exact);
    let b = capture(&mut o, &c, &mut r);
    o.resolve_registration_basis_for_host_v1(&b.basis, &c, &current(&c), &mut r)
        .unwrap();
    assert_eq!(
        o.registration_basis_usage_v1().unwrap().declared_bytes,
        exact.max_held_bytes
    );
    for i in 0..5 {
        let mut p = exact;
        *[
            &mut p.max_root_bytes,
            &mut p.max_slot_bytes,
            &mut p.max_basis_bytes,
            &mut p.max_use_bytes,
            &mut p.max_reply_bytes,
        ][i] -= 1;
        let (mut o, mut r, c) = setup(&[], p);
        assert!(
            matches!(
                o.capture_registration_basis_v1(&c, &slot, &current(&c), &mut r),
                Err(ResourceError::Limit)
            ),
            "bound {i}"
        );
        assert_eq!(o.registration_basis_usage_v1().unwrap().issued_lifetime, 0);
        assert_eq!(o.registration_basis_usage_v1().unwrap().live, 0);
    }
    for i in 0..3 {
        let mut p = exact;
        match i {
            0 => p.max_scope_bytes -= 1,
            1 => p.max_profile_bytes -= 1,
            _ => p.max_held_bytes -= 1,
        }
        let mut o = bare();
        let mut r = Recorder::disabled();
        o.configure_retention_budget(1_000_000, &mut r).unwrap();
        o.configure_resource_catalog_v1(&[], limits(), &facts(), &mut r)
            .unwrap();
        assert_eq!(
            o.configure_registration_bases_v1(&c, limits(), p, &setup_facts(), &mut r),
            Err(ResourceError::Limit)
        );
        assert_eq!(o.retention_usage().reserved, 0);
        assert!(matches!(o.registration_bases, State::Unconfigured));
    }
    let p = ResourceRegistrationBasisLimitsV1 {
        max_tag_bytes: 64,
        ..profile()
    };
    let (mut o, mut r, c) = setup(&[], p);
    let oversized = address(ResourceCatalogTableV1::Read, &"x".repeat(65));
    assert!(matches!(
        o.capture_registration_basis_v1(&c, &oversized, &current(&c), &mut r),
        Err(ResourceError::Malformed)
    ));
    assert_eq!(o.registration_basis_usage_v1().unwrap().issued_lifetime, 0);
    let p = ResourceRegistrationBasisLimitsV1 {
        max_bases: usize::MAX,
        ..profile()
    };
    assert_eq!(p.required_pool_bytes(), Err(ResourceError::Limit));
}

#[test]
fn issuer_uses_shared_headroom_and_basis_controls_frame_root_provider_work_and_other_holds() {
    let c = scope();
    let p = profile();
    let mut o = bare();
    let mut r = Recorder::disabled();
    o.configure_resource_catalog_v1(&[], limits(), &facts(), &mut r)
        .unwrap();
    o.configure_retention_budget(p.max_held_bytes, &mut r)
        .unwrap();
    let mut other = o.retention.reserve(1).unwrap();
    assert_eq!(
        o.configure_registration_bases_v1(&c, limits(), p, &setup_facts(), &mut r),
        Err(ResourceError::Limit)
    );
    assert!(matches!(o.registration_bases, State::Unconfigured));
    o.retention.release(&mut other).unwrap();
    let root = o.native_root_identity().unwrap();
    let root_bytes = o.measure_semantic_root().unwrap();
    let work = o.resource_usage();
    let generation = o.generation();
    o.configure_registration_bases_v1(&c, limits(), p, &setup_facts(), &mut r)
        .unwrap();
    assert!(
        !o.configure_registration_bases_v1(&c, limits(), p, &setup_facts(), &mut r)
            .unwrap()
    );
    let b = capture(&mut o, &c, &mut r);
    assert_eq!(o.native_root_identity().unwrap(), root);
    assert_eq!(o.measure_semantic_root().unwrap(), root_bytes);
    assert_eq!(o.resource_usage(), work);
    assert_eq!(o.generation(), generation);
    o.retire_registration_basis_v1(&b.basis, &control(), &mut r)
        .unwrap();
    o.retire_registration_basis_issuer_v1(&control(), &mut r)
        .unwrap();
    assert_eq!(o.native_root_identity().unwrap(), root);
    assert_eq!(o.measure_semantic_root().unwrap(), root_bytes);
}

#[test]
fn issuer_setup_is_strict_and_cannot_change_the_captured_native_meaning() {
    for i in 0..3 {
        for denied in [Some(false), None] {
            let mut o = bare();
            let mut r = Recorder::disabled();
            let c = scope();
            o.configure_resource_catalog_v1(&[], limits(), &facts(), &mut r)
                .unwrap();
            o.configure_retention_budget(1_000_000, &mut r).unwrap();
            let mut f = setup_facts();
            *[
                &mut f.owner_control_allowed,
                &mut f.resources_ready,
                &mut f.preparation_reserved,
            ][i] = denied;
            assert_eq!(
                o.configure_registration_bases_v1(&c, limits(), profile(), &f, &mut r),
                Err(ResourceError::Unavailable)
            );
            assert_eq!(o.retention_usage().reserved, 0);
        }
    }
    let (mut o, mut r, c) = setup(&[], profile());
    let p = ResourceRegistrationBasisLimitsV1 {
        max_issued: 100,
        ..profile()
    };
    assert_eq!(
        o.configure_registration_bases_v1(&c, limits(), p, &setup_facts(), &mut r),
        Err(ResourceError::Unavailable)
    );
    let mut changed = limits();
    changed.max_tag_bytes += 1;
    assert_eq!(
        o.configure_registration_bases_v1(&c, changed, profile(), &setup_facts(), &mut r),
        Err(ResourceError::Unavailable)
    );
}
