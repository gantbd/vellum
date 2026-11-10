use super::*;
fn limits() -> SettingsDefinitionLimitsV1 {
    SettingsDefinitionLimitsV1 {
        max_request_bytes: 65536,
        max_plan_bytes: 1_048_576,
        max_work: 1_048_576,
    }
}
fn ready() -> (SourceCoordinator, Recorder) {
    let (mut o, mut r) = setup();
    o.configure_settings_definitions(limits(), &mut r).unwrap();
    (o, r)
}
pub(super) fn declaration() -> SettingsDeclarationV1 {
    SettingsDeclarationV1 {
        schemas: vec![SettingSchemaV1 {
            key: "unit".into(),
            value_type: SettingTypeV1::Integer { min: 0, max: 8 },
            default: SettingValueV1::Number(1.0),
            domain: SettingDomainV1::Source,
            scopes: vec![SettingScopeV1::Host, SettingScopeV1::Workspace],
            composition: SettingCompositionV1::Replace,
            sensitive: false,
        }],
        restrictions: vec![],
        targets: vec![
            SettingTargetV1 {
                identity: id("A"),
                layers: vec![id("host"), id("left")],
            },
            SettingTargetV1 {
                identity: id("B"),
                layers: vec![id("host"), id("right")],
            },
        ],
    }
}
fn reads() -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 4096,
        },
    }
}
fn prepare(
    o: &SourceCoordinator,
    d: &SettingsDeclarationV1,
    r: &mut Recorder,
) -> Result<PreparedSettingsDefinitionV1, SettingsChangeErrorV2> {
    let expected = o.settings_basis(r).unwrap().interpretation();
    o.prepare_settings_definition(expected, d, &reads(), r)
}
fn activate(
    o: &mut SourceCoordinator,
    d: &SettingsDeclarationV1,
    r: &mut Recorder,
) -> SettingsDefinitionOutcomeV1 {
    let p = prepare(o, d, r).unwrap();
    o.commit_settings_definition(p, r).unwrap()
}
#[test]
fn metadata_only_activation_preserves_authored_values_generations_and_native_handles() {
    let (mut o, mut r) = ready();
    let a = o.settings_target(&id("A"), &mut r).unwrap();
    set(&mut o, "left", 4.0, &mut r);
    let layer = o.settings_layer_generation(&id("left"), &mut r).unwrap();
    let domains = o.settings_generations(&a, &mut r).unwrap();
    let basis = o.settings_basis(&mut r).unwrap();
    let mut d = declaration();
    d.schemas[0].sensitive = true;
    let result = activate(&mut o, &d, &mut r);
    assert!(result.changed && result.schema_changed);
    assert!(!result.restrictions_changed && !result.associations_changed);
    assert_eq!(result.changed_domains, 0);
    assert_eq!(result.basis.schema_generation, basis.schema_generation + 1);
    assert_eq!(
        result.basis.restriction_generation,
        basis.restriction_generation
    );
    assert_eq!(
        o.settings_layer_generation(&id("left"), &mut r).unwrap(),
        layer
    );
    assert_eq!(o.settings_generations(&a, &mut r).unwrap(), domains);
    assert_eq!(o.settings_target(&id("A"), &mut r).unwrap(), a);
    assert_eq!(read(&o, "A", &mut r), 4.0);
}
#[test]
fn changed_default_advances_only_targets_whose_effective_mapping_changes() {
    let (mut o, mut r) = ready();
    set(&mut o, "left", 4.0, &mut r);
    let a = o.settings_target(&id("A"), &mut r).unwrap();
    let b = o.settings_target(&id("B"), &mut r).unwrap();
    let ga = o.settings_generations(&a, &mut r).unwrap();
    let gb = o.settings_generations(&b, &mut r).unwrap();
    let mut d = declaration();
    d.schemas[0].default = SettingValueV1::Number(2.0);
    let result = activate(&mut o, &d, &mut r);
    assert_eq!(result.changed_domains, 1);
    assert_eq!(o.settings_generations(&a, &mut r).unwrap(), ga);
    assert_eq!(
        o.settings_generations(&b, &mut r).unwrap().source,
        gb.source + 1
    );
    assert_eq!(read(&o, "A", &mut r), 4.0);
    assert_eq!(read(&o, "B", &mut r), 2.0);
}
#[test]
fn key_addition_removal_and_domain_movement_change_maps_not_just_values() {
    let (mut o, mut r) = ready();
    let mut d = declaration();
    d.schemas.push(SettingSchemaV1 {
        key: "extra".into(),
        value_type: SettingTypeV1::Boolean,
        default: SettingValueV1::Boolean(false),
        domain: SettingDomainV1::Analysis,
        scopes: vec![SettingScopeV1::Host],
        composition: SettingCompositionV1::Replace,
        sensitive: false,
    });
    assert_eq!(activate(&mut o, &d, &mut r).changed_domains, 2);
    d.schemas[1].domain = SettingDomainV1::Serialization;
    assert_eq!(activate(&mut o, &d, &mut r).changed_domains, 4);
    d.schemas.pop();
    assert_eq!(activate(&mut o, &d, &mut r).changed_domains, 2);
}
#[test]
fn association_reordering_preserves_handles_removal_and_restore_never_revive_them() {
    let (mut o, mut r) = ready();
    let old = o.settings_target(&id("A"), &mut r).unwrap();
    let b = o.settings_target(&id("B"), &mut r).unwrap();
    let mut d = declaration();
    d.targets.reverse();
    let result = activate(&mut o, &d, &mut r);
    assert!(result.associations_changed);
    assert_eq!(result.changed_domains, 0);
    assert_eq!(o.settings_target(&id("A"), &mut r).unwrap(), old);
    d.targets.pop();
    assert_eq!(activate(&mut o, &d, &mut r).removed_targets, 1);
    assert_eq!(
        o.settings_read(&old, "unit", &mut r).err(),
        Some(SettingsError::Unavailable)
    );
    assert_eq!(o.settings_target(&id("B"), &mut r).unwrap(), b);
    d.targets.push(SettingTargetV1 {
        identity: id("A"),
        layers: vec![id("host"), id("left")],
    });
    assert_eq!(activate(&mut o, &d, &mut r).added_targets, 1);
    let new = o.settings_target(&id("A"), &mut r).unwrap();
    assert_ne!(new, old);
    assert_eq!(o.settings_generations(&new, &mut r).unwrap().source, 1);
    assert_eq!(
        o.settings_read(&old, "unit", &mut r).err(),
        Some(SettingsError::Unavailable)
    );
}
#[test]
fn reassociation_recomputes_values_without_changing_authored_layers() {
    let (mut o, mut r) = ready();
    set(&mut o, "left", 2.0, &mut r);
    set(&mut o, "right", 3.0, &mut r);
    let mut d = declaration();
    d.targets[0].layers = vec![id("host"), id("right")];
    let result = activate(&mut o, &d, &mut r);
    assert_eq!(result.changed_domains, 1);
    assert_eq!(read(&o, "A", &mut r), 3.0);
    assert_eq!(o.settings_layer_generation(&id("left"), &mut r).unwrap(), 2);
}
#[test]
fn invalid_declarations_and_unused_authored_contributions_refuse_the_entire_candidate() {
    for kind in 0..10 {
        let (mut o, mut r) = ready();
        set(&mut o, "right", 3.0, &mut r);
        let mut d = declaration();
        d.targets.pop(); // right remains authored but unused
        match kind {
            0 => d.schemas.clear(),
            1 => d.schemas[0].value_type = SettingTypeV1::Boolean,
            2 => d.schemas[0].scopes = vec![SettingScopeV1::Host],
            3 => d.schemas[0].value_type = SettingTypeV1::Integer { min: 4, max: 2 },
            4 => d.schemas[0].scopes.clear(),
            5 => d.targets[0].layers.push(id("missing")),
            6 => d.targets[0].layers.push(id("left")),
            7 => d.targets[0].identity.authority = "foreign".into(),
            8 => d.schemas[0].composition = SettingCompositionV1::MergeMap,
            _ => d
                .restrictions
                .push(("unit".into(), vec![SettingValueV1::Number(8.0)])),
        }
        let basis = o.settings_basis(&mut r).unwrap();
        let generation = o.generation();
        assert!(prepare(&o, &d, &mut r).is_err(), "kind {kind}");
        assert_eq!(o.settings_basis(&mut r).unwrap(), basis);
        assert_eq!(o.generation(), generation);
        assert_eq!(read(&o, "B", &mut r), 3.0);
        assert!(!o.is_faulted());
    }
}
#[test]
fn restrictions_change_only_the_restriction_epoch_when_values_are_unchanged() {
    let (mut o, mut r) = ready();
    let mut d = declaration();
    d.restrictions.push((
        "unit".into(),
        vec![SettingValueV1::Number(1.0), SettingValueV1::Number(2.0)],
    ));
    let result = activate(&mut o, &d, &mut r);
    assert!(result.restrictions_changed);
    assert!(!result.schema_changed && !result.associations_changed);
    assert_eq!(result.changed_domains, 0);
    assert_eq!(result.basis.restriction_generation, 2);
}
#[test]
fn complete_identity_publishes_nothing_and_layer_change_stales_even_a_noop() {
    let (mut o, mut r) = ready();
    let d = declaration();
    let generation = o.generation();
    assert!(!activate(&mut o, &d, &mut r).changed);
    assert_eq!(o.generation(), generation);
    let p = prepare(&o, &d, &mut r).unwrap();
    set(&mut o, "right", 3.0, &mut r);
    assert_eq!(
        o.commit_settings_definition(p, &mut r),
        Err(SettingsChangeErrorV2::Settings(SettingsError::Stale))
    );
}
#[test]
fn unrelated_source_progress_is_allowed_but_explicit_source_reads_remain_required() {
    for dependent in [false, true] {
        let (mut o, mut r) = ready();
        let v = o.create(b"S", &mut r).unwrap();
        let mut request = reads();
        if dependent {
            request
                .predicates
                .push(ReadDependencyV3::Base(ReadDependencyV2::Base(
                    ReadDependencyV1::Source(v.clone()),
                )));
        }
        let expected = o.settings_basis(&mut r).unwrap().interpretation();
        let mut d = declaration();
        d.schemas[0].sensitive = true;
        let p = o
            .prepare_settings_definition(expected, &d, &request, &mut r)
            .unwrap();
        super::settings_reads::source_edit(&mut o, &v, &mut r);
        let result = o.commit_settings_definition(p, &mut r);
        if dependent {
            assert_eq!(
                result,
                Err(SettingsChangeErrorV2::Reads(CoordinatorError::Stale))
            );
        } else {
            assert!(result.unwrap().changed);
        }
    }
}
#[test]
fn association_aba_stales_preexisting_value_plans() {
    let (mut o, mut r) = ready();
    let p = o
        .prepare_settings_change(
            &[change(
                "left",
                1,
                SettingOperationV1::Set(SettingValueV1::Number(2.0)),
            )],
            &mut r,
        )
        .unwrap();
    let mut d = declaration();
    d.targets.reverse();
    activate(&mut o, &d, &mut r);
    d.targets.reverse();
    activate(&mut o, &d, &mut r);
    assert_eq!(
        o.commit_settings_change(p, &mut r),
        Err(SettingsError::Stale)
    );
}
#[test]
fn retained_definition_replay_is_historical_and_never_resamples_new_layers() {
    let (mut o, mut r) = ready();
    let mut d = declaration();
    d.schemas[0].sensitive = true;
    let expected = o.settings_basis(&mut r).unwrap().interpretation();
    let ticket = o.issue_invocation(&mut r).unwrap();
    assert_eq!(
        o.submit_settings_definition(&ticket, expected, &d, &reads(), &mut r),
        Ok(InvocationState::Pending)
    );
    assert_eq!(o.settings_retention_usage().open_preparations, 1);
    o.run(&ticket, &mut r).unwrap();
    assert!(
        matches!(o.invocation_outcome(&ticket, &mut r), Ok(InvocationOutcome::SettingsDefinitionCommitted(result)) if result.changed)
    );
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    set(&mut o, "left", 4.0, &mut r);
    let generation = o.generation();
    assert_eq!(
        o.submit_settings_definition(&ticket, expected, &d, &reads(), &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), generation);
    d.schemas[0].sensitive = false;
    assert_eq!(
        o.submit_settings_definition(&ticket, expected, &d, &reads(), &mut r),
        Err(InvocationError::IntentMismatch)
    );
}
#[test]
fn native_refusal_is_retained_not_reprepared_after_root_changes() {
    let (mut o, mut r) = ready();
    let mut d = declaration();
    d.schemas[0].default = SettingValueV1::Boolean(true);
    let expected = o.settings_basis(&mut r).unwrap().interpretation();
    let ticket = o.issue_invocation(&mut r).unwrap();
    o.submit_settings_definition(&ticket, expected, &d, &reads(), &mut r)
        .unwrap();
    set(&mut o, "left", 3.0, &mut r);
    o.run(&ticket, &mut r).unwrap();
    assert!(matches!(
        o.invocation_outcome(&ticket, &mut r),
        Ok(InvocationOutcome::SettingsDefinitionRejected(
            SettingsChangeErrorV2::Settings(SettingsError::InvalidValue)
        ))
    ));
    assert_eq!(
        o.submit_settings_definition(&ticket, expected, &d, &reads(), &mut r),
        Ok(InvocationState::Finished)
    );
}
#[test]
fn old_interpretations_remain_frozen_and_pending_definition_holds_block_pool_retirement() {
    let (mut o, mut r) = ready();
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
    let old = o.acquire_settings_interpretation(&mut r).unwrap();
    let basis = o.settings_interpretation_basis(&old, &mut r).unwrap();
    let mut d = declaration();
    d.schemas[0].default = SettingValueV1::Number(2.0);
    activate(&mut o, &d, &mut r);
    assert_eq!(
        o.settings_interpretation_basis(&old, &mut r).unwrap(),
        basis
    );
    assert_eq!(
        o.settings_interpretation_read(&old, &id("A"), "unit", &mut r)
            .unwrap()
            .value
            .number(),
        Some(1.0)
    );
    o.release_settings_interpretation(&old, &mut r).unwrap();
    let expected = o.settings_basis(&mut r).unwrap().interpretation();
    let ticket = o.issue_invocation(&mut r).unwrap();
    o.submit_settings_definition(&ticket, expected, &d, &reads(), &mut r)
        .unwrap();
    assert_eq!(
        o.retire_settings_retention(&mut r),
        Err(SettingsError::Unavailable)
    );
    o.stop_invocation(&ticket, &mut r).unwrap();
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    o.forget_invocation_outcome(&ticket, &mut r).unwrap();
    assert_eq!(
        o.submit_settings_definition(&ticket, expected, &d, &reads(), &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.retire_settings_retention(&mut r), Ok(true));
    assert!(matches!(
        prepare(&o, &d, &mut r),
        Err(SettingsChangeErrorV2::Settings(SettingsError::Unavailable))
    ));
    o.retire_invocation(&ticket, &mut r).unwrap();
    assert_eq!(
        o.submit_settings_definition(&ticket, expected, &d, &reads(), &mut r),
        Err(InvocationError::Unavailable)
    );
}
#[test]
fn association_activation_refuses_ambiguous_equal_rank_values_without_publishing_a_prefix() {
    let (mut o, mut r) = ready();
    set(&mut o, "left", 2.0, &mut r);
    set(&mut o, "right", 3.0, &mut r);
    let mut d = declaration();
    d.targets[0].layers.push(id("right"));
    let generation = o.generation();
    assert!(matches!(
        prepare(&o, &d, &mut r),
        Err(SettingsChangeErrorV2::Settings(SettingsError::Conflict))
    ));
    assert_eq!(o.generation(), generation);
    assert_eq!(read(&o, "A", &mut r), 2.0);
}
#[test]
fn all_six_declared_types_survive_flat_definition_copy_rebuild_and_identity_comparison() {
    let (mut o, mut r) = ready();
    let mut d = declaration();
    for (i, (value_type, default)) in [
        (SettingTypeV1::Boolean, SettingValueV1::Boolean(true)),
        (
            SettingTypeV1::Integer { min: 0, max: 3 },
            SettingValueV1::Number(2.0),
        ),
        (SettingTypeV1::Text, SettingValueV1::Text("private".into())),
        (
            SettingTypeV1::Enum(vec![
                SettingValueV1::Null,
                SettingValueV1::Text("on".into()),
            ]),
            SettingValueV1::Null,
        ),
        (
            SettingTypeV1::List(Box::new(SettingTypeV1::Boolean)),
            SettingValueV1::List(vec![SettingValueV1::Boolean(false)]),
        ),
        (
            SettingTypeV1::Map(vec![("flag".into(), SettingTypeV1::Boolean)]),
            SettingValueV1::Map(vec![("flag".into(), SettingValueV1::Boolean(true))]),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        d.schemas.push(SettingSchemaV1 {
            key: format!("key{i}"),
            value_type,
            default,
            domain: SettingDomainV1::Analysis,
            scopes: vec![SettingScopeV1::Host],
            composition: SettingCompositionV1::Replace,
            sensitive: true,
        });
    }
    assert_eq!(activate(&mut o, &d, &mut r).changed_domains, 2);
    let generation = o.generation();
    assert!(!activate(&mut o, &d, &mut r).changed);
    assert_eq!(o.generation(), generation);
    d.schemas[0].sensitive = true;
    assert_eq!(activate(&mut o, &d, &mut r).changed_domains, 0);
}
#[test]
fn definition_configuration_preparation_validation_and_publication_diagnostics_are_optional() {
    use vellum_engine::instrumentation::{Boundary, Disposition};
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _) = setup();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        o.configure_settings_definitions(limits(), &mut r).unwrap();
        let mut d = declaration();
        d.schemas[0].sensitive = true;
        assert!(activate(&mut o, &d, &mut r).changed);
        let p = prepare(&o, &d, &mut r).unwrap();
        set(&mut o, "right", 3.0, &mut r);
        assert_eq!(
            o.commit_settings_definition(p, &mut r),
            Err(SettingsChangeErrorV2::Settings(SettingsError::Stale))
        );
        assert_eq!(read(&o, "B", &mut r), 3.0);
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(128) {
            for boundary in [
                Boundary::SettingsDefinitionConfiguration,
                Boundary::SettingsDefinitionPreparation,
                Boundary::SettingsDefinitionValidation,
                Boundary::SettingsDefinitionPublication,
                Boundary::ReadDependencyValidation,
            ] {
                assert!(records.iter().any(|record| record.boundary == boundary));
            }
            assert!(records.iter().any(|record| record.boundary
                == Boundary::SettingsDefinitionPublication
                && record.disposition == Disposition::Refused));
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
        if capacity.is_none() {
            assert!(records.is_empty());
        }
    }
}
