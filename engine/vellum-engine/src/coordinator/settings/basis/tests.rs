use super::*;
fn layer() -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: "target".into(),
        incarnation: 1,
    }
}
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "engine".into(),
        engine: "local".into(),
        recipient: "reader".into(),
        purpose: "edit".into(),
        destination: "client".into(),
        profile: "v1".into(),
        continuity: "session".into(),
    }
}
fn activation() -> SettingsActivationLimitsV1 {
    SettingsActivationLimitsV1 {
        max_changes: 8,
        max_request_nodes: 64,
        max_request_depth: 32,
        max_request_bytes: 16384,
        max_plan_bytes: 65536,
        max_work: 65536,
    }
}
fn setup() -> (SourceCoordinator, Recorder) {
    setup_with_work(65536)
}
fn setup_with_work(work: usize) -> (SourceCoordinator, Recorder) {
    let (mut o, _, _, _, mut r) = read_dependency::configuration_read_test_fixture();
    o.configure_settings_activation(activation(), &mut r)
        .unwrap();
    let mut pool = SettingsRetentionLimitsV1 {
        max_issued: 4,
        max_retained: 4,
        max_snapshot_bytes: 65536,
        max_pool_bytes: usize::MAX,
    };
    pool.max_pool_bytes = pool.required_pool_bytes().unwrap();
    o.configure_retention_budget(pool.max_pool_bytes, &mut r)
        .unwrap();
    o.configure_settings_retention(pool, &mut r).unwrap();
    o.configure_settings_bases(
        "local",
        SettingsBasisLimitsV1 {
            max_label_bytes: 128,
            max_context_bytes: 4096,
            max_grants: 8,
            max_work: work,
        },
        &mut r,
    )
    .unwrap();
    (o, r)
}
fn facts<'a>(
    c: &'a SettingsExecutionContextV1,
    layers: &'a [SettingIdentityV1],
) -> TrustedSettingsBasisFactsV1<'a> {
    TrustedSettingsBasisFactsV1 {
        context: c,
        activation: activation(),
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        interpretation_current: Some(true),
        resources_ready: Some(true),
        owner_control_allowed: Some(true),
        settings_writable: Some(true),
        basis_allowed: Some(true),
        writable_layers: Some(layers),
    }
}
fn control(c: &SettingsExecutionContextV1) -> SettingsBasisControlV1<'_> {
    SettingsBasisControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
#[test]
fn full_basis_envelope_is_charged_and_failed_bounds_issue_no_slot() {
    let (mut o, mut r) = setup();
    let c = context();
    let layers = [layer()];
    let f = facts(&c, &layers);
    let basis = o
        .capture_settings_write_basis(&layer(), &c, &f, &mut r)
        .unwrap();
    let bytes = o.held_write_basis(&basis, &c).unwrap().bytes;
    assert!(bytes > o.settings_usage().unwrap().logical_bytes);
    assert!(o.retire_settings_write_basis(&basis, &c, &control(&c), &mut r));
    let RetentionPool::Active(pool) = &mut o.settings_retention else {
        unreachable!()
    };
    pool.limits.max_snapshot_bytes = bytes - 1;
    assert_eq!(
        o.capture_settings_write_basis(&layer(), &c, &f, &mut r),
        Err(SettingsError::Limit)
    );
    assert_eq!(o.settings_retention_usage().issued_lifetime, 1);
    assert_eq!(o.settings_retention_usage().retained, 0);
    let RetentionPool::Active(pool) = &mut o.settings_retention else {
        unreachable!()
    };
    pool.limits.max_snapshot_bytes = bytes;
    let basis = o
        .capture_settings_write_basis(&layer(), &c, &f, &mut r)
        .unwrap();
    assert_eq!(o.settings_retention_usage().retained_snapshot_bytes, bytes);
    assert_eq!(o.settings_retention_usage().issued_lifetime, 2);
    assert!(o.retire_settings_write_basis(&basis, &c, &control(&c), &mut r));
}
#[test]
fn fabricated_wrong_kind_or_spent_slots_cannot_reinterpret_or_retire_a_real_interpretation() {
    let (mut o, mut r) = setup();
    let c = context();
    let layers = [layer()];
    let f = facts(&c, &layers);
    let basis = o
        .capture_settings_write_basis(&layer(), &c, &f, &mut r)
        .unwrap();
    let interpretation = o.acquire_settings_interpretation(&mut r).unwrap();
    for slot in [1, usize::MAX] {
        let forged = SettingsWriteBasisId {
            owner: o.namespace.clone(),
            slot,
        };
        assert_eq!(
            o.inspect_settings_write_basis(&forged, &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Unavailable
        );
        assert!(!o.stop_settings_write_basis(&forged, &c, &control(&c), &mut r));
        assert!(!o.retire_settings_write_basis(&forged, &c, &control(&c), &mut r));
    }
    assert_eq!(o.settings_retention_usage().retained, 2);
    assert!(
        o.settings_interpretation_basis(&interpretation, &mut r)
            .is_ok()
    );
    assert!(o.retire_settings_write_basis(&basis, &c, &control(&c), &mut r));
    assert_eq!(
        o.inspect_settings_write_basis(&basis, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
    assert!(!o.retire_settings_write_basis(&basis, &c, &control(&c), &mut r));
    assert!(
        o.settings_interpretation_basis(&interpretation, &mut r)
            .is_ok()
    );
}
#[test]
fn unknown_owner_fault_refuses_inspection_but_protected_retirement_preserves_fault_and_issuance() {
    let (mut o, mut r) = setup();
    let c = context();
    let layers = [layer()];
    let f = facts(&c, &layers);
    let b = o
        .capture_settings_write_basis(&layer(), &c, &f, &mut r)
        .unwrap();
    o.faulted = true;
    assert_eq!(
        o.inspect_settings_write_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
    assert!(!o.held_write_basis(&b, &c).unwrap().closed);
    assert!(o.retire_settings_write_basis(&b, &c, &control(&c), &mut r));
    assert_eq!(o.settings_retention_usage().issued_lifetime, 1);
    assert!(o.is_faulted());
    assert_eq!(o.retire_settings_retention(&mut r), Ok(true));
    assert!(o.is_faulted());
    assert_eq!(o.retention_usage().reserved, 0);
}
#[test]
fn foreign_parent_funding_faults_inspection_and_cannot_be_discharged_as_our_reservation() {
    let (mut o, mut r) = setup();
    let c = context();
    let layers = [layer()];
    let f = facts(&c, &layers);
    let b = o
        .capture_settings_write_basis(&layer(), &c, &f, &mut r)
        .unwrap();
    let (mut other, _) = setup();
    let forged = other.retention.reserve(0).unwrap();
    let RetentionPool::Active(pool) = &mut o.settings_retention else {
        unreachable!()
    };
    let original = std::mem::replace(&mut pool.reservation, forged);
    assert_eq!(
        o.inspect_settings_write_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
    assert!(o.is_faulted());
    assert!(!o.retire_settings_write_basis(&b, &c, &control(&c), &mut r));
    let RetentionPool::Active(pool) = &mut o.settings_retention else {
        unreachable!()
    };
    pool.reservation = original;
    assert!(o.retire_settings_write_basis(&b, &c, &control(&c), &mut r));
    assert!(o.is_faulted());
}
#[test]
fn bounded_work_refusal_after_real_unrelated_growth_does_not_assert_irreversible_footprint_loss() {
    use crate::settings::{
        SettingLayerCreateV1, SettingLayerRemoveV1, SettingScopeV1, SettingTargetV1,
        SettingsLayerChangeV1, SettingsLayerLimitsV1,
    };
    let (mut o, mut r) = setup_with_work(8);
    let c = context();
    let layers = [layer()];
    let f = facts(&c, &layers);
    let b = o
        .capture_settings_write_basis(&layer(), &c, &f, &mut r)
        .unwrap();
    o.configure_settings_layers(
        SettingsLayerLimitsV1 {
            max_changes: 8,
            max_created_value_bytes: 16384,
            max_request_bytes: 16384,
            max_plan_bytes: 65536,
            max_work: 65536,
        },
        &mut r,
    )
    .unwrap();
    let id = |incarnation| SettingIdentityV1 {
        authority: "engine".into(),
        id: "target".into(),
        incarnation,
    };
    let mut q = SettingsLayerChangeV1 {
        create: (2..=8)
            .map(|n| SettingLayerCreateV1 {
                identity: id(n),
                scope: SettingScopeV1::Host,
                values: vec![],
            })
            .collect(),
        remove: vec![],
        targets: vec![SettingTargetV1 {
            identity: layer(),
            layers: vec![layer()],
        }],
    };
    let reads = ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 256,
        },
    };
    let e = o.settings_layer_basis(&mut r).unwrap();
    let p = o.prepare_settings_layers(e, &q, &reads, &mut r).unwrap();
    o.commit_settings_layers(p, &mut r).unwrap();
    assert_eq!(
        o.inspect_settings_write_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
    assert!(!o.held_write_basis(&b, &c).unwrap().closed);
    assert!(!o.is_faulted());
    q.create.clear();
    q.remove = (2..=8)
        .map(|n| SettingLayerRemoveV1 {
            identity: id(n),
            expected_generation: 1,
        })
        .collect();
    let e = o.settings_layer_basis(&mut r).unwrap();
    let p = o.prepare_settings_layers(e, &q, &reads, &mut r).unwrap();
    o.commit_settings_layers(p, &mut r).unwrap();
    assert_eq!(
        o.inspect_settings_write_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
}
