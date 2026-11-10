use super::*;
use crate::settings::*;
mod shutdown;
fn target_id(name: &str) -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: name.into(),
        incarnation: 1,
    }
}
pub(in crate::coordinator) fn multi_target(r: &mut Recorder) -> CompiledSettingsV1 {
    let d = SettingsDefinitionV1 {
        schemas: vec![SettingSchemaV1 {
            key: "flag".into(),
            value_type: SettingTypeV1::Boolean,
            default: SettingValueV1::Boolean(false),
            domain: SettingDomainV1::Analysis,
            scopes: vec![SettingScopeV1::Host],
            composition: SettingCompositionV1::Replace,
            sensitive: true,
        }],
        restrictions: vec![],
        layers: [("A", false), ("B", true), ("C", true)]
            .into_iter()
            .map(|(name, value)| SettingLayerV1 {
                identity: target_id(name),
                scope: SettingScopeV1::Host,
                generation: 1,
                values: vec![(
                    "flag".into(),
                    SettingContributionV1::Set(SettingValueV1::Boolean(value)),
                )],
            })
            .collect(),
        targets: ["A", "B", "C"]
            .into_iter()
            .map(|name| SettingTargetV1 {
                identity: target_id(name),
                layers: vec![target_id(name)],
            })
            .collect(),
    };
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
            max_value_nodes: 64,
            max_value_depth: 16,
            max_state_bytes: 16384,
            max_work: 4096,
        },
        r,
    )
    .unwrap()
}
#[test]
fn target_handles_and_read_predicates_follow_identity_not_physical_target_order() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    o.install_initial_settings(multi_target(&mut r), &mut r)
        .unwrap();
    let a = o.settings_target(&target_id("A"), &mut r).unwrap();
    let b = o.settings_target(&target_id("B"), &mut r).unwrap();
    let before = o.settings_basis(&mut r).unwrap();
    let mut candidate = o.settings.as_ref().unwrap().flat_clone_for_test().unwrap();
    candidate.reverse_targets_for_test();
    o.settings = Some(Arc::new(candidate));
    assert_eq!(o.settings_target(&target_id("A"), &mut r).unwrap(), a);
    assert_eq!(o.settings_target(&target_id("B"), &mut r).unwrap(), b);
    assert_eq!(
        o.settings_read(&a, "flag", &mut r).unwrap().value.boolean(),
        Some(false)
    );
    assert_eq!(
        o.settings_read(&b, "flag", &mut r).unwrap().value.boolean(),
        Some(true)
    );
    let reads = ReadDependencyRequestV3 {
        predicates: vec![ReadDependencyV3::ConfigurationDomain {
            target: a.clone(),
            domain: SettingDomainV1::Analysis,
            expected: 1,
            interpretation: before.interpretation(),
        }],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 128,
        },
    };
    o.configure_semantic_root_budget(1_048_576, &mut r).unwrap();
    o.configure_settings_activation(
        SettingsActivationLimitsV1 {
            max_changes: 8,
            max_request_nodes: 64,
            max_request_depth: 16,
            max_request_bytes: 16384,
            max_plan_bytes: 65536,
            max_work: 4096,
        },
        &mut r,
    )
    .unwrap();
    let p = o
        .prepare_settings_change_with_reads(
            &[SettingChangeV1 {
                layer: target_id("A"),
                expected_generation: 1,
                key: "flag".into(),
                operation: SettingOperationV1::Set(SettingValueV1::Boolean(true)),
            }],
            &reads,
            &mut r,
        )
        .unwrap();
    o.commit_settings_change_with_reads(p, &mut r).unwrap();
    assert_eq!(o.settings_generations(&a, &mut r).unwrap().analysis, 2);
    assert_eq!(o.settings_generations(&b, &mut r).unwrap().analysis, 1);
    assert_eq!(
        o.check_read_dependencies_v3(&[], &reads, &mut r),
        Err(CoordinatorError::Stale)
    );
}
#[test]
fn retired_or_reissued_registration_never_retargets_old_handles_or_retained_interpretations() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    o.install_initial_settings(multi_target(&mut r), &mut r)
        .unwrap();
    let a = o.settings_target(&target_id("A"), &mut r).unwrap();
    let b = o.settings_target(&target_id("B"), &mut r).unwrap();
    let p = SettingsRetentionLimitsV1 {
        max_issued: 4,
        max_retained: 4,
        max_snapshot_bytes: 16384,
        max_pool_bytes: 131072,
    };
    o.configure_retention_budget(p.required_pool_bytes().unwrap(), &mut r)
        .unwrap();
    o.configure_settings_retention(p, &mut r).unwrap();
    let old = o.acquire_settings_interpretation(&mut r).unwrap();
    let mut removed = o.settings.as_ref().unwrap().flat_clone_for_test().unwrap();
    removed
        .retire_native_target_for_test(a.native_identity())
        .unwrap();
    let mut restored = o.settings.as_ref().unwrap().flat_clone_for_test().unwrap();
    restored
        .renew_native_target_for_test(a.native_identity())
        .unwrap();
    o.settings = Some(Arc::new(removed));
    assert_eq!(
        o.settings_read(&a, "flag", &mut r).err(),
        Some(SettingsError::Unavailable)
    );
    assert_eq!(
        o.settings_read(&b, "flag", &mut r).unwrap().value.boolean(),
        Some(true)
    );
    o.settings = Some(Arc::new(restored));
    let fresh = o.settings_target(&target_id("A"), &mut r).unwrap();
    assert_ne!(a, fresh);
    assert_eq!(
        o.settings_generations(&a, &mut r),
        Err(SettingsError::Unavailable)
    );
    assert_eq!(
        o.settings_read(&fresh, "flag", &mut r)
            .unwrap()
            .value
            .boolean(),
        Some(false)
    );
    assert_eq!(
        o.settings_interpretation_read(&old, &target_id("A"), "flag", &mut r)
            .unwrap()
            .value
            .boolean(),
        Some(false)
    );
    assert_eq!(o.settings_target(&target_id("B"), &mut r).unwrap(), b);
}
pub(super) fn owner() -> SourceCoordinator {
    SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 4,
        max_participants: 4,
        max_staged_bytes: 128,
        source: Limits {
            max_source_bytes: 128,
            max_edits: 4,
            max_edit_payload_bytes: 256,
        },
    })
}
fn identity() -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: "target".into(),
        incarnation: 1,
    }
}
pub(in crate::coordinator) fn compiled(r: &mut Recorder) -> CompiledSettingsV1 {
    compile_settings_v1(
        &SettingsDefinitionV1 {
            schemas: vec![SettingSchemaV1 {
                key: "flag".into(),
                value_type: SettingTypeV1::Boolean,
                default: SettingValueV1::Boolean(false),
                domain: SettingDomainV1::Analysis,
                scopes: vec![SettingScopeV1::Host],
                composition: SettingCompositionV1::Replace,
                sensitive: true,
            }],
            restrictions: vec![],
            layers: vec![SettingLayerV1 {
                identity: identity(),
                scope: SettingScopeV1::Host,
                generation: 1,
                values: vec![],
            }],
            targets: vec![SettingTargetV1 {
                identity: identity(),
                layers: vec![identity()],
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
            max_work: 4096,
        },
        r,
    )
    .unwrap()
}
#[test]
fn final_publication_exhaustion_preserves_settings_but_identity_needs_no_counter() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    o.install_initial_settings(compiled(&mut r), &mut r)
        .unwrap();
    o.configure_semantic_root_budget(1_048_576, &mut r).unwrap();
    o.configure_settings_activation(
        SettingsActivationLimitsV1 {
            max_changes: 4,
            max_request_nodes: 16,
            max_request_depth: 16,
            max_request_bytes: 4096,
            max_plan_bytes: 65536,
            max_work: 4096,
        },
        &mut r,
    )
    .unwrap();
    let target = o.settings_target(&identity(), &mut r).unwrap();
    let basis = o.settings_basis(&mut r).unwrap();
    let prepared = o
        .prepare_settings_change(
            &[SettingChangeV1 {
                layer: identity(),
                expected_generation: 1,
                key: "flag".into(),
                operation: SettingOperationV1::Set(SettingValueV1::Boolean(true)),
            }],
            &mut r,
        )
        .unwrap();
    o.generation = u64::MAX;
    assert_eq!(
        o.commit_settings_change(prepared, &mut r),
        Err(SettingsError::VersionExhausted)
    );
    assert_eq!(o.settings_basis(&mut r).unwrap(), basis);
    assert_eq!(o.settings_layer_generation(&identity(), &mut r).unwrap(), 1);
    assert_eq!(
        o.settings_read(&target, "flag", &mut r)
            .unwrap()
            .value
            .boolean(),
        Some(false)
    );
    assert!(!o.is_faulted());
    let identity_plan = o.prepare_settings_change(&[], &mut r).unwrap();
    assert!(
        !o.commit_settings_change(identity_plan, &mut r)
            .unwrap()
            .changed
    );
    assert_eq!(o.generation(), u64::MAX);
    assert!(!o.is_faulted());
    let mut altered = o.prepare_settings_change(&[], &mut r).unwrap();
    altered._retention = Arc::new(());
    assert_eq!(
        o.commit_settings_change(altered, &mut r),
        Err(SettingsError::Faulted)
    );
    assert!(o.is_faulted());
}
#[test]
fn exhausted_publication_refuses_initial_install_without_publishing_settings_or_fencing_owner() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    o.generation = u64::MAX;
    assert_eq!(
        o.install_initial_settings(compiled(&mut r), &mut r),
        Err(SettingsError::VersionExhausted)
    );
    assert!(o.settings_usage().is_none());
    assert_eq!(o.generation(), u64::MAX);
    assert!(!o.is_faulted());
    assert_eq!(
        o.settings_target(&identity(), &mut r),
        Err(SettingsError::Unavailable)
    );
}
#[test]
fn forged_slot_and_foreign_owner_cannot_inspect_even_an_identical_compiled_schema() {
    let mut o = owner();
    let mut foreign = owner();
    let mut r = Recorder::disabled();
    o.install_initial_settings(compiled(&mut r), &mut r)
        .unwrap();
    foreign
        .install_initial_settings(compiled(&mut r), &mut r)
        .unwrap();
    let target = o.settings_target(&identity(), &mut r).unwrap();
    assert_eq!(
        foreign.settings_read(&target, "flag", &mut r).err(),
        Some(SettingsError::Unavailable)
    );
    let forged = SettingsTargetId {
        owner: o.namespace.clone(),
        identity: usize::MAX,
    };
    assert_eq!(
        o.settings_read(&forged, "unknown", &mut r).err(),
        Some(SettingsError::Unavailable)
    );
    assert!(!o.is_faulted());
    assert_eq!(
        o.settings_read(&target, "flag", &mut r)
            .unwrap()
            .value
            .boolean(),
        Some(false)
    );
}
