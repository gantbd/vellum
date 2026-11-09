use super::*;
fn id(n: u64) -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: "layer".into(),
        incarnation: n,
    }
}
fn state(n: u64) -> CompiledSettingsV1 {
    compile_settings_v1(
        &SettingsDefinitionV1 {
            schemas: vec![SettingSchemaV1 {
                key: "n".into(),
                value_type: SettingTypeV1::Integer { min: 0, max: 8 },
                default: SettingValueV1::Number(1.0),
                domain: SettingDomainV1::Source,
                scopes: vec![SettingScopeV1::Host],
                composition: SettingCompositionV1::Replace,
                sensitive: false,
            }],
            restrictions: vec![],
            layers: if n == 0 {
                vec![]
            } else {
                vec![SettingLayerV1 {
                    identity: id(n),
                    scope: SettingScopeV1::Host,
                    generation: 1,
                    values: vec![],
                }]
            },
            targets: vec![SettingTargetV1 {
                identity: id(1),
                layers: if n == 0 { vec![] } else { vec![id(n)] },
            }],
        },
        "engine",
        SettingsLimitsV1 {
            max_keys: 8,
            max_layers: 8,
            max_targets: 8,
            max_identifier_bytes: 64,
            max_schema_nodes: 64,
            max_schema_depth: 16,
            max_value_nodes: 64,
            max_value_depth: 16,
            max_state_bytes: 16384,
            max_work: 65536,
        },
        &mut Recorder::disabled(),
    )
    .unwrap()
}
fn limits() -> SettingsLayerLimitsV1 {
    SettingsLayerLimitsV1 {
        max_changes: 8,
        max_created_value_bytes: 16384,
        max_request_bytes: 16384,
        max_plan_bytes: 65536,
        max_work: 65536,
    }
}
fn request(n: u64) -> SettingsLayerChangeV1 {
    SettingsLayerChangeV1 {
        create: vec![SettingLayerCreateV1 {
            identity: id(n),
            scope: SettingScopeV1::Host,
            values: vec![],
        }],
        remove: vec![],
        targets: vec![SettingTargetV1 {
            identity: id(1),
            layers: vec![id(1)],
        }],
    }
}
fn plan(s: CompiledSettingsV1, input: &SettingsLayerChangeV1) -> Result<LayerPlan, SettingsError> {
    let e = s.layer_basis();
    let c = Arc::new(LayerCapture::copy(input, &s, limits(), 16384)?);
    LayerPlan::prepare(Arc::new(s), c, e)
}
#[test]
fn initial_high_water_is_all_accepted_incarnations_and_removal_never_refunds_exhaustion() {
    assert_eq!(state(0).layer_basis().layer_incarnation, 0);
    let first = plan(state(0), &request(1)).unwrap();
    assert_eq!(first.decide(&first.before).unwrap().1.layer_incarnation, 1);
    assert_eq!(state(37).layer_basis().layer_incarnation, 37);
    let s = state(MAX_SAFE as u64);
    let mut q = request(MAX_SAFE as u64);
    assert!(matches!(plan(s, &q), Err(SettingsError::VersionExhausted)));
    q.create.clear();
    q.targets[0].layers.clear();
    q.remove.push(SettingLayerRemoveV1 {
        identity: id(MAX_SAFE as u64),
        expected_generation: 1,
    });
    let p = plan(state(MAX_SAFE as u64), &q).unwrap();
    let (out, result) = p.decide(&p.before).unwrap();
    assert_eq!(result.layer_incarnation, MAX_SAFE as u64);
    assert_eq!(out.unwrap().layers.len(), 0);
}
#[test]
fn complete_noop_needs_no_unused_counter_while_changing_counters_refuse_atomically() {
    let mut s = state(1);
    s.basis = SettingsBasisV1 {
        configuration_generation: MAX_SAFE as u64,
        schema_generation: MAX_SAFE as u64,
        restriction_generation: MAX_SAFE as u64,
        association_generation: MAX_SAFE as u64,
    };
    s.layer_incarnation = MAX_SAFE as u64;
    s.next_target_identity = usize::MAX;
    s.targets[0].generations = [MAX_SAFE as u64; 3];
    let mut q = request(2);
    q.create.clear();
    let p = plan(s, &q).unwrap();
    assert!(p.decide(&p.before).unwrap().0.is_none());
    for kind in 0..4 {
        let mut s = state(1);
        let mut q = request(2);
        match kind {
            0 => s.basis.configuration_generation = MAX_SAFE as u64,
            1 => {
                s.basis.association_generation = MAX_SAFE as u64;
                q.targets[0].layers.clear();
            }
            2 => {
                s.targets[0].generations[0] = MAX_SAFE as u64;
                q.targets[0].layers = vec![id(2)];
                q.create[0].values.push((
                    "n".into(),
                    SettingContributionV1::Set(SettingValueV1::Number(2.0)),
                ));
            }
            _ => {
                s.next_target_identity = usize::MAX;
                q.targets[0].identity.incarnation = 2;
            }
        }
        assert!(
            matches!(plan(s, &q), Err(SettingsError::VersionExhausted)),
            "kind {kind}"
        );
    }
    let mut s = state(1);
    s.basis.association_generation = MAX_SAFE as u64;
    s.targets[0].generations = [MAX_SAFE as u64; 3];
    s.next_target_identity = usize::MAX;
    let p = plan(s, &request(2)).unwrap();
    assert!(p.decide(&p.before).unwrap().1.changed);
}
