use super::*;
fn id() -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: "target".into(),
        incarnation: 1,
    }
}
fn declaration(value: f64) -> SettingsDeclarationV1 {
    SettingsDeclarationV1 {
        schemas: vec![SettingSchemaV1 {
            key: "n".into(),
            value_type: SettingTypeV1::Integer { min: 0, max: 1 },
            default: SettingValueV1::Number(value),
            domain: SettingDomainV1::Source,
            scopes: vec![SettingScopeV1::Host],
            composition: SettingCompositionV1::Replace,
            sensitive: false,
        }],
        restrictions: vec![],
        targets: vec![SettingTargetV1 {
            identity: id(),
            layers: vec![id()],
        }],
    }
}
fn limits() -> SettingsDefinitionLimitsV1 {
    SettingsDefinitionLimitsV1 {
        max_request_bytes: 16384,
        max_plan_bytes: 65536,
        max_work: 65536,
    }
}
fn state() -> CompiledSettingsV1 {
    let d = declaration(-0.0);
    compile_settings_v1(
        &SettingsDefinitionV1 {
            schemas: d.schemas,
            restrictions: d.restrictions,
            targets: d.targets,
            layers: vec![SettingLayerV1 {
                identity: id(),
                scope: SettingScopeV1::Host,
                generation: 1,
                values: vec![],
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
fn plan(
    state: CompiledSettingsV1,
    d: &SettingsDeclarationV1,
) -> Result<DefinitionPlan, SettingsError> {
    let e = state.basis.interpretation();
    let capture = Arc::new(DefinitionCapture::from_state(d, &state, limits(), 16384)?);
    DefinitionPlan::prepare(Arc::new(state), capture, e)
}
#[test]
fn complete_mathematical_identity_needs_no_unused_counter_or_native_identity() {
    let mut s = state();
    s.basis = SettingsBasisV1 {
        configuration_generation: MAX_SAFE as u64,
        schema_generation: MAX_SAFE as u64,
        restriction_generation: MAX_SAFE as u64,
        association_generation: MAX_SAFE as u64,
    };
    s.targets[0].generations = [MAX_SAFE as u64; 3];
    s.next_target_identity = usize::MAX;
    let p = plan(s, &declaration(0.0)).unwrap();
    let (candidate, result) = p.decide(&p.before).unwrap();
    assert!(candidate.is_none());
    assert!(!result.changed);
    assert_eq!(
        p.before
            .read_index(0, "n")
            .unwrap()
            .value
            .number()
            .unwrap()
            .to_bits(),
        (-0.0f64).to_bits()
    );
}
#[test]
fn only_changing_counters_are_required_and_exhaustion_is_atomic() {
    for counter in 0..6 {
        let mut s = state();
        let mut d = declaration(0.0);
        match counter {
            0 => {
                s.basis.configuration_generation = MAX_SAFE as u64;
                d.schemas[0].sensitive = true;
            }
            1 => {
                s.basis.schema_generation = MAX_SAFE as u64;
                d.schemas[0].sensitive = true;
            }
            2 => {
                s.basis.restriction_generation = MAX_SAFE as u64;
                d.restrictions
                    .push(("n".into(), vec![SettingValueV1::Number(0.0)]));
            }
            3 => {
                s.basis.association_generation = MAX_SAFE as u64;
                d.targets[0].layers.clear();
            }
            4 => {
                s.targets[0].generations[0] = MAX_SAFE as u64;
                d.schemas[0].default = SettingValueV1::Number(1.0);
            }
            _ => {
                s.next_target_identity = usize::MAX;
                d.targets[0].identity.id = "new".into();
            }
        }
        assert!(
            matches!(plan(s, &d), Err(SettingsError::VersionExhausted)),
            "counter {counter}"
        );
    }
    let mut s = state();
    s.targets[0].generations = [MAX_SAFE as u64; 3];
    s.next_target_identity = usize::MAX;
    let mut d = declaration(0.0);
    d.schemas[0].sensitive = true;
    let p = plan(s, &d).unwrap();
    assert_eq!(p.decide(&p.before).unwrap().1.changed_domains, 0);
}
