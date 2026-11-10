use super::*;
use crate::coordinator::scoped_definition::tests::{common, context, control, setup};
fn limits() -> SettingsLayerLimitsV1 {
    SettingsLayerLimitsV1 {
        max_changes: 8,
        max_created_value_bytes: 2048,
        max_request_bytes: 16384,
        max_plan_bytes: 65536,
        max_work: 65536,
    }
}
fn facts<'a>(
    common: &'a TrustedSettingsBasisFactsV1<'a>,
) -> TrustedSettingsLifecycleBasisFactsV1<'a> {
    TrustedSettingsLifecycleBasisFactsV1 {
        common,
        lifecycle: limits(),
        basis_allowed: Some(true),
    }
}
fn ready() -> (SourceCoordinator, Recorder) {
    let (mut o, mut r) = setup();
    o.configure_settings_layers(limits(), &mut r).unwrap();
    (o, r)
}
fn empty_ready() -> (SourceCoordinator, Recorder) {
    let (template, _) = ready();
    let RetentionPool::Active(pool) = &template.settings_retention else {
        unreachable!()
    };
    let pool_limits = pool.limits;
    let mut o = SourceCoordinator::with_invocation_limits(
        template.limits,
        InvocationLimits {
            max_lifetime_issued: 8,
            max_retained: 8,
            max_intent_bytes: 16384,
        },
    );
    let mut r = Recorder::disabled();
    let definition = crate::settings::SettingsDefinitionV1 {
        schemas: vec![],
        restrictions: vec![],
        layers: vec![],
        targets: vec![],
    };
    let compiled = crate::settings::compile_settings_v1(
        &definition,
        "engine",
        template.settings.as_ref().unwrap().limits(),
        &mut r,
    )
    .unwrap();
    assert_eq!(compiled.layer_basis().layer_incarnation, 0);
    o.install_initial_settings(compiled, &mut r).unwrap();
    o.configure_settings_activation(template.settings_activation.unwrap(), &mut r)
        .unwrap();
    o.configure_settings_layers(limits(), &mut r).unwrap();
    o.configure_retention_budget(pool_limits.max_pool_bytes, &mut r)
        .unwrap();
    o.configure_settings_retention(pool_limits, &mut r).unwrap();
    o.configure_settings_bases(
        "local",
        template.settings_basis_profile.as_ref().unwrap().limits,
        &mut r,
    )
    .unwrap();
    (o, r)
}
#[test]
fn complete_native_context_record_byte_ceiling_is_exact_without_issuance_on_refusal() {
    let c = context();
    let bytes = labels(&c)
        .into_iter()
        .fold(std::mem::size_of::<LifecycleBasisCapture>(), |n, s| {
            n + s.len()
        });
    for exact in [false, true] {
        let (mut o, mut r) = ready();
        let common = common(&c);
        let f = facts(&common);
        let RetentionPool::Active(pool) = &mut o.settings_retention else {
            unreachable!()
        };
        pool.limits.max_snapshot_bytes = bytes - usize::from(!exact);
        let retained = o.retention_usage().reserved;
        let generation = o.generation;
        assert_eq!(
            o.capture_settings_lifecycle_basis(&c, &f, &mut r).is_ok(),
            exact
        );
        assert_eq!(
            o.settings_retention_usage().issued_lifetime,
            usize::from(exact)
        );
        assert_eq!(o.generation, generation);
        assert_eq!(o.retention_usage().reserved, retained);
        assert!(!o.is_faulted());
    }
}
#[test]
fn zero_or_exhausted_highwater_can_be_conditioned_but_invalid_actual_pair_faults_before_issuance() {
    for highwater in [0, 9_007_199_254_740_991] {
        let (mut o, mut r) = if highwater == 0 {
            empty_ready()
        } else {
            ready()
        };
        let mut root = o.settings.as_ref().unwrap().flat_clone_for_test().unwrap();
        root.set_layer_basis_for_test(SettingsLayerBasisV1 {
            configuration_generation: 9_007_199_254_740_991,
            layer_incarnation: highwater,
        });
        o.settings = Some(Arc::new(root));
        o.generation = u64::MAX;
        let c = context();
        let common = common(&c);
        let f = facts(&common);
        let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
        assert_eq!(
            o.inspect_settings_lifecycle_basis(&b, &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
        assert_eq!(
            o.held_lifecycle_basis(&b, &c)
                .unwrap()
                .expected
                .layer_incarnation,
            highwater
        );
        assert!(!o.is_faulted());
    }
    for pair in [
        SettingsLayerBasisV1 {
            configuration_generation: 0,
            layer_incarnation: 0,
        },
        SettingsLayerBasisV1 {
            configuration_generation: 1,
            layer_incarnation: 9_007_199_254_740_992,
        },
    ] {
        let (mut o, mut r) = ready();
        let mut root = o.settings.as_ref().unwrap().flat_clone_for_test().unwrap();
        root.set_layer_basis_for_test(pair);
        o.settings = Some(Arc::new(root));
        let c = context();
        let common = common(&c);
        let f = facts(&common);
        assert_eq!(
            o.capture_settings_lifecycle_basis(&c, &f, &mut r),
            Err(SettingsError::Faulted)
        );
        assert!(o.is_faulted());
        assert_eq!(o.settings_retention_usage().issued_lifetime, 0);
    }
}
#[test]
fn wrong_row_role_and_out_of_range_slot_refuse_without_poisoning_real_basis() {
    let (mut o, mut r) = ready();
    let c = context();
    let common = common(&c);
    let f = facts(&common);
    let lease = o.acquire_settings_interpretation(&mut r).unwrap();
    let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    for slot in [0, usize::MAX] {
        let forged = SettingsLifecycleBasisId {
            owner: o.namespace.clone(),
            slot,
        };
        assert_eq!(
            o.inspect_settings_lifecycle_basis(&forged, &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Unavailable
        );
        assert!(!o.stop_settings_lifecycle_basis(&forged, &c, &control(&c), &mut r));
        assert!(!o.retire_settings_lifecycle_basis(&forged, &c, &control(&c), &mut r));
    }
    assert_eq!(
        o.inspect_settings_lifecycle_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    o.release_settings_interpretation(&lease, &mut r).unwrap();
}
#[test]
fn protected_drain_requires_exact_owner_and_context_and_remains_available_after_fault() {
    let (mut o, mut r) = ready();
    let c = context();
    let common = common(&c);
    let f = facts(&common);
    let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let usage = o.settings_retention_usage();
    o.faulted = true;
    o.generation = u64::MAX;
    for field in 0..4 {
        let mut ctl = control(&c);
        let mut wrong = c.clone();
        wrong.recipient = "foreign".into();
        match field {
            0 => ctl.owner_control_allowed = None,
            1 => ctl.retention_released = Some(false),
            2 => ctl.delivery_drained = None,
            _ => ctl.context = &wrong,
        };
        assert!(!o.retire_settings_lifecycle_basis(&b, &c, &ctl, &mut r));
        assert_eq!(o.settings_retention_usage(), usage);
    }
    assert!(o.stop_settings_lifecycle_basis(&b, &c, &control(&c), &mut r));
    assert_eq!(o.settings_retention_usage(), usage);
    assert!(o.retire_settings_lifecycle_basis(&b, &c, &control(&c), &mut r));
    assert!(!o.retire_settings_lifecycle_basis(&b, &c, &control(&c), &mut r));
    assert_eq!(o.retire_settings_retention(&mut r), Ok(true));
    assert_eq!(o.retention_usage().reserved, 0);
    assert!(o.is_faulted());
    assert_eq!(o.settings_retention_usage().issued_lifetime, 1);
}
