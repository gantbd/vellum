use super::*;
#[test]
fn target_registration_exhaustion_does_not_consume_capacity_during_existing_value_changes() {
    let mut state = fixture(false);
    Arc::get_mut(&mut state).unwrap().next_target_identity = usize::MAX;
    let old = state.native_target_identity(0).unwrap();
    let changes = [SettingChangeV1 {
        layer: id(),
        expected_generation: 1,
        key: "source".into(),
        operation: SettingOperationV1::Set(SettingValueV1::Number(3.0)),
    }];
    let plan = MutationPlan::prepare(state.clone(), &changes, limits()).unwrap();
    let (candidate, outcome) = plan.decide(&state).unwrap();
    assert!(outcome.changed);
    let mut candidate = candidate.unwrap();
    assert_eq!(candidate.native_target_identity(0).unwrap(), old);
    assert_eq!(candidate.next_target_identity, usize::MAX);
    assert_eq!(
        candidate.issue_target_identity(),
        Err(SettingsError::VersionExhausted)
    );
    assert_eq!(candidate.next_target_identity, usize::MAX);
}
#[test]
fn duplicate_native_target_identity_is_unknown_not_an_arbitrary_first_match() {
    let state = fixture(false);
    let mut duplicate = state.flat_clone_for_test().unwrap();
    let mut target = duplicate.targets.remove(0);
    let id = target.native_identity;
    // A separately owned target header with the same claimed native identity is corrupt.
    target.identity.id = "other".into();
    duplicate.targets.push(target);
    let mut other = state.flat_clone_for_test().unwrap().targets.remove(0);
    other.identity.id = "third".into();
    duplicate.targets.push(other);
    assert_eq!(
        duplicate.native_target_index(id),
        Err(SettingsError::Faulted)
    );
    assert_eq!(
        duplicate.native_target_identity(0),
        Err(SettingsError::Faulted)
    );
}
use crate::instrumentation::Recorder;
fn id() -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: "layer".into(),
        incarnation: 1,
    }
}
fn limits() -> SettingsActivationLimitsV1 {
    SettingsActivationLimitsV1 {
        max_changes: 8,
        max_request_nodes: 128,
        max_request_depth: 16,
        max_request_bytes: 16384,
        max_plan_bytes: 65536,
        max_work: 65536,
    }
}
fn fixture(zero: bool) -> Arc<CompiledSettingsV1> {
    let schemas: Vec<_> = [
        ("source", SettingDomainV1::Source),
        ("serialization", SettingDomainV1::Serialization),
        ("analysis", SettingDomainV1::Analysis),
    ]
    .into_iter()
    .map(|(key, domain)| SettingSchemaV1 {
        key: key.into(),
        value_type: SettingTypeV1::Integer { min: 0, max: 8 },
        default: SettingValueV1::Number(1.0),
        domain,
        scopes: vec![SettingScopeV1::Host],
        composition: SettingCompositionV1::Replace,
        sensitive: false,
    })
    .collect();
    let d = SettingsDefinitionV1 {
        schemas,
        restrictions: vec![],
        layers: vec![SettingLayerV1 {
            identity: id(),
            scope: SettingScopeV1::Host,
            generation: 1,
            values: ["source", "serialization", "analysis"]
                .into_iter()
                .map(|key| {
                    (
                        key.into(),
                        SettingContributionV1::Set(SettingValueV1::Number(if zero {
                            -0.0
                        } else {
                            2.0
                        })),
                    )
                })
                .collect(),
        }],
        targets: vec![SettingTargetV1 {
            identity: SettingIdentityV1 {
                id: "target".into(),
                ..id()
            },
            layers: vec![id()],
        }],
    };
    Arc::new(
        compile_settings_v1(
            &d,
            "engine",
            SettingsLimitsV1 {
                max_keys: 8,
                max_layers: 8,
                max_targets: 8,
                max_identifier_bytes: 64,
                max_schema_nodes: 64,
                max_schema_depth: 16,
                max_value_nodes: 128,
                max_value_depth: 16,
                max_state_bytes: 65536,
                max_work: 65536,
            },
            &mut Recorder::disabled(),
        )
        .unwrap(),
    )
}
#[test]
fn complete_identity_needs_no_unused_version_capacity_even_when_all_counters_are_exhausted() {
    let mut state = fixture(false);
    let s = Arc::get_mut(&mut state).unwrap();
    s.basis.configuration_generation = MAX_SAFE as u64;
    s.layers[0].generation = MAX_SAFE as u64;
    s.targets[0].generations = [MAX_SAFE as u64; 3];
    let request = [SettingChangeV1 {
        layer: id(),
        expected_generation: MAX_SAFE as u64,
        key: "source".into(),
        operation: SettingOperationV1::Set(SettingValueV1::Number(2.0)),
    }];
    let p = MutationPlan::prepare(state.clone(), &request, limits()).unwrap();
    let (candidate, outcome) = p.decide(&state).unwrap();
    assert!(candidate.is_none());
    assert!(!outcome.changed);
    assert_eq!(outcome.basis.configuration_generation, MAX_SAFE as u64);
}
#[test]
fn exhausted_last_domain_never_updates_earlier_domains_or_the_current_layer() {
    let mut state = fixture(false);
    Arc::get_mut(&mut state).unwrap().targets[0].generations[2] = MAX_SAFE as u64;
    let request: Vec<_> = ["source", "serialization", "analysis"]
        .into_iter()
        .map(|key| SettingChangeV1 {
            layer: id(),
            expected_generation: 1,
            key: key.into(),
            operation: SettingOperationV1::Set(SettingValueV1::Number(3.0)),
        })
        .collect();
    assert_eq!(
        MutationPlan::prepare(state.clone(), &request, limits()).err(),
        Some(SettingsError::VersionExhausted)
    );
    assert_eq!(state.basis.configuration_generation, 1);
    assert_eq!(state.layers[0].generation, 1);
    assert_eq!(state.targets[0].generations, [1, 1, MAX_SAFE as u64]);
}
#[test]
fn mathematical_noop_does_not_erase_native_negative_zero_or_exact_intent_distinctions() {
    let state = fixture(true);
    let request = [SettingChangeV1 {
        layer: id(),
        expected_generation: 1,
        key: "source".into(),
        operation: SettingOperationV1::Set(SettingValueV1::Number(0.0)),
    }];
    let p = MutationPlan::prepare(state.clone(), &request, limits()).unwrap();
    assert!(!p.would_change);
    let (candidate, outcome) = p.decide(&state).unwrap();
    assert!(candidate.is_none());
    assert!(!outcome.changed);
    let Contribution::Set(old) = state.layers[0].values[0].1 else {
        unreachable!()
    };
    let Operation::Set(new) = p.request.changes[0].operation else {
        unreachable!()
    };
    let mut m = Meter::new(state.limits);
    assert!(!resolve::equal_between(&state, old, &p.request.values, new, &mut m, true).unwrap());
    assert!(
        f64::from_bits(match state.nodes[old] {
            Node::Number(n) => n,
            _ => unreachable!(),
        })
        .is_sign_negative()
    );
}
