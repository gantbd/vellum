use super::*;
use crate::settings::*;
pub(in crate::coordinator) fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "engine".into(),
        engine: "local".into(),
        recipient: "author".into(),
        purpose: "edit".into(),
        destination: "client".into(),
        profile: "v1".into(),
        continuity: "session".into(),
    }
}
pub(in crate::coordinator) fn activation() -> SettingsActivationLimitsV1 {
    SettingsActivationLimitsV1 {
        max_changes: 8,
        max_request_nodes: 64,
        max_request_depth: 32,
        max_request_bytes: 16384,
        max_plan_bytes: 65536,
        max_work: 65536,
    }
}
pub(in crate::coordinator) fn limits() -> SettingsDefinitionLimitsV1 {
    SettingsDefinitionLimitsV1 {
        max_request_bytes: 16384,
        max_plan_bytes: 65536,
        max_work: 65536,
    }
}
pub(in crate::coordinator) fn common(
    c: &SettingsExecutionContextV1,
) -> TrustedSettingsBasisFactsV1<'_> {
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
        basis_allowed: None,
        writable_layers: None,
    }
}
pub(in crate::coordinator) fn basis<'a>(
    common: &'a TrustedSettingsBasisFactsV1<'a>,
) -> TrustedSettingsDefinitionBasisFactsV1<'a> {
    TrustedSettingsDefinitionBasisFactsV1 {
        common,
        definition: limits(),
        basis_allowed: Some(true),
    }
}
pub(in crate::coordinator) fn gate<'a>(
    basis: &'a TrustedSettingsDefinitionBasisFactsV1<'a>,
) -> TrustedSettingsDefinitionMutationFactsV1<'a> {
    TrustedSettingsDefinitionMutationFactsV1 {
        basis,
        host_basis_allowed: Some(true),
        schema_writable: Some(true),
        restrictions_writable: Some(true),
        associations_writable: Some(true),
        required_reads: &[],
    }
}
pub(in crate::coordinator) fn control(
    c: &SettingsExecutionContextV1,
) -> SettingsBasisControlV1<'_> {
    SettingsBasisControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(in crate::coordinator) fn reads() -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 256,
        },
    }
}
pub(in crate::coordinator) fn declaration(changed: bool) -> SettingsDeclarationV1 {
    let id = || SettingIdentityV1 {
        authority: "engine".into(),
        id: "target".into(),
        incarnation: 1,
    };
    SettingsDeclarationV1 {
        schemas: vec![SettingSchemaV1 {
            key: "flag".into(),
            value_type: SettingTypeV1::Boolean,
            default: SettingValueV1::Boolean(false),
            domain: SettingDomainV1::Analysis,
            scopes: vec![SettingScopeV1::Host],
            composition: SettingCompositionV1::Replace,
            sensitive: !changed,
        }],
        restrictions: vec![],
        targets: vec![SettingTargetV1 {
            identity: id(),
            layers: vec![id()],
        }],
    }
}
pub(in crate::coordinator) fn setup() -> (SourceCoordinator, Recorder) {
    setup_with_retention_headroom(0)
}
pub(in crate::coordinator) fn setup_with_retention_headroom(
    headroom: usize,
) -> (SourceCoordinator, Recorder) {
    let (mut o, _, _, _, mut r) = read_dependency::configuration_read_test_fixture();
    o.configure_settings_activation(activation(), &mut r)
        .unwrap();
    o.configure_settings_definitions(limits(), &mut r).unwrap();
    let mut pool = SettingsRetentionLimitsV1 {
        max_issued: 8,
        max_retained: 8,
        max_snapshot_bytes: 65536,
        max_pool_bytes: usize::MAX,
    };
    pool.max_pool_bytes = pool.required_pool_bytes().unwrap();
    o.configure_retention_budget(pool.max_pool_bytes.checked_add(headroom).unwrap(), &mut r)
        .unwrap();
    o.configure_settings_retention(pool, &mut r).unwrap();
    o.configure_settings_bases(
        "local",
        SettingsBasisLimitsV1 {
            max_label_bytes: 128,
            max_context_bytes: 4096,
            max_grants: 16,
            max_work: 65536,
        },
        &mut r,
    )
    .unwrap();
    (o, r)
}
#[test]
fn private_read_profile_count_seal_or_native_plan_drift_faults_before_publication() {
    for drift in 0..4 {
        let (mut o, mut r) = setup();
        let c = context();
        let common = common(&c);
        let f = basis(&common);
        let g = gate(&f);
        let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
        let d = declaration(true);
        let reads = reads();
        let req = ScopedSettingsDefinitionRequestV1 {
            basis: &b,
            declaration: &d,
            dependencies: &reads,
        };
        let mut p = o
            .prepare_scoped_settings_definition(&req, &c, &g, &mut r)
            .unwrap();
        match drift {
            0 => p.native.read_count += 1,
            1 => {
                p.native.reads = read_dependency::ReadInput::V1(&ReadDependencyRequest {
                    predicates: vec![],
                    budget: reads.budget,
                })
                .copy(&[])
                .unwrap()
            }
            2 => p.seal = o.capture_scoped_definition(&req, &c, &g).unwrap(),
            _ => {
                let current = o.settings_basis(&mut r).unwrap().interpretation();
                p.native = o
                    .prepare_settings_definition(current, &declaration(false), &reads, &mut r)
                    .unwrap();
            }
        }
        let before = o.generation;
        assert!(matches!(
            o.commit_scoped_settings_definition(p, &c, &g, &mut r),
            Err(Error::Settings(SettingsError::Faulted))
        ));
        assert!(o.is_faulted());
        assert_eq!(o.generation, before);
        assert_eq!(o.settings_retention_usage().open_preparations, 0);
    }
}
#[test]
fn complete_request_and_plan_limits_are_independent_of_the_native_inner_envelope() {
    let (mut o, mut r) = setup();
    let c = context();
    let common = common(&c);
    let f = basis(&common);
    let g = gate(&f);
    let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
    let d = declaration(true);
    let reads = reads();
    let req = ScopedSettingsDefinitionRequestV1 {
        basis: &b,
        declaration: &d,
        dependencies: &reads,
    };
    let capture = o.capture_scoped_definition(&req, &c, &g).unwrap();
    let request_bytes = capture.bytes;
    let p = o
        .prepare_scoped_definition_capture(capture, &c, &g, &mut r)
        .unwrap();
    let full = p.retained_bytes().unwrap();
    assert!(full > p.native.retained_bytes().unwrap());
    drop(p);
    for plan in [false, true] {
        for exact in [false, true] {
            let (mut o, mut r) = setup();
            let mut p = limits();
            if plan {
                p.max_plan_bytes = full - usize::from(!exact);
            } else {
                p.max_request_bytes = request_bytes - usize::from(!exact);
            }
            o.settings_definition_limits = Some(p);
            let mut f = basis(&common);
            f.definition = p;
            let g = gate(&f);
            let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
            let req = ScopedSettingsDefinitionRequestV1 {
                basis: &b,
                declaration: &d,
                dependencies: &reads,
            };
            let result = o.prepare_scoped_settings_definition(&req, &c, &g, &mut r);
            assert_eq!(result.is_ok(), exact);
            if !exact {
                assert!(matches!(result, Err(Error::Settings(SettingsError::Limit))));
            }
            drop(result);
            assert_eq!(o.settings_retention_usage().open_preparations, 0);
        }
    }
}
