use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Boundary;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::settings::*;
use vellum_engine::source::{Limits, Replacement};
#[path = "settings_behaviors/settings_recipient.rs"]
mod settings_recipient;
fn limits() -> SettingsLimitsV1 {
    SettingsLimitsV1 {
        max_keys: 16,
        max_layers: 16,
        max_targets: 16,
        max_identifier_bytes: 128,
        max_schema_nodes: 128,
        max_schema_depth: 64,
        max_value_nodes: 2048,
        max_value_depth: 64,
        max_state_bytes: 1_048_576,
        max_work: 1_048_576,
    }
}
fn id(name: &str) -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: name.into(),
        incarnation: 1,
    }
}
fn definition() -> SettingsDefinitionV1 {
    SettingsDefinitionV1 {
        schemas: vec![SettingSchemaV1 {
            key: "unit".into(),
            value_type: SettingTypeV1::Integer { min: 0, max: 8 },
            default: SettingValueV1::Number(1.0),
            domain: SettingDomainV1::Source,
            scopes: vec![
                SettingScopeV1::Host,
                SettingScopeV1::Workspace,
                SettingScopeV1::Document,
            ],
            composition: SettingCompositionV1::Replace,
            sensitive: false,
        }],
        restrictions: vec![],
        layers: vec![
            SettingLayerV1 {
                identity: id("host"),
                scope: SettingScopeV1::Host,
                generation: 1,
                values: vec![(
                    "unit".into(),
                    SettingContributionV1::Set(SettingValueV1::Number(3.0)),
                )],
            },
            SettingLayerV1 {
                identity: id("workspace"),
                scope: SettingScopeV1::Workspace,
                generation: 1,
                values: vec![],
            },
        ],
        targets: vec![SettingTargetV1 {
            identity: id("document"),
            layers: vec![id("host"), id("workspace")],
        }],
    }
}
#[test]
fn absence_inherits_and_explicit_reset_selects_the_library_default() {
    for reset in [false, true] {
        let mut definition = definition();
        if reset {
            definition.layers[1]
                .values
                .push(("unit".into(), SettingContributionV1::Reset));
        }
        let mut r = Recorder::disabled();
        let compiled = compile_settings_v1(&definition, "engine", limits(), &mut r).unwrap();
        assert_eq!(
            compiled
                .read(&id("document"), "unit")
                .unwrap()
                .value
                .number(),
            Some(if reset { 1.0 } else { 3.0 })
        );
    }
}

#[test]
fn six_closed_types_validate_without_coercion_and_compiled_values_do_not_alias_inputs() {
    let cases = vec![
        (
            SettingTypeV1::Boolean,
            SettingValueV1::Boolean(false),
            SettingValueV1::Boolean(true),
            SettingValueV1::Number(1.0),
        ),
        (
            SettingTypeV1::Integer { min: 0, max: 8 },
            SettingValueV1::Number(1.0),
            SettingValueV1::Number(-0.0),
            SettingValueV1::Number(9.0),
        ),
        (
            SettingTypeV1::Text,
            SettingValueV1::Text("base".into()),
            SettingValueV1::Text("hé🦀\r\n\u{feff}".into()),
            SettingValueV1::Boolean(true),
        ),
        (
            SettingTypeV1::Enum(vec![
                SettingValueV1::Null,
                SettingValueV1::List(vec![SettingValueV1::Number(1.5)]),
            ]),
            SettingValueV1::Null,
            SettingValueV1::List(vec![SettingValueV1::Number(1.5)]),
            SettingValueV1::Number(1.5),
        ),
        (
            SettingTypeV1::List(Box::new(SettingTypeV1::Integer { min: 0, max: 2 })),
            SettingValueV1::List(vec![SettingValueV1::Number(1.0)]),
            SettingValueV1::List(vec![]),
            SettingValueV1::List(vec![SettingValueV1::Text("1".into())]),
        ),
        (
            SettingTypeV1::Map(vec![
                ("left".into(), SettingTypeV1::Boolean),
                ("right".into(), SettingTypeV1::Integer { min: 0, max: 2 }),
            ]),
            SettingValueV1::Map(vec![
                ("left".into(), SettingValueV1::Boolean(false)),
                ("right".into(), SettingValueV1::Number(1.0)),
            ]),
            SettingValueV1::Map(vec![("left".into(), SettingValueV1::Boolean(true))]),
            SettingValueV1::Map(vec![("hidden".into(), SettingValueV1::Boolean(true))]),
        ),
    ];
    for (i, (ty, default, valid, invalid)) in cases.into_iter().enumerate() {
        let mut d = definition();
        d.schemas[0].value_type = ty;
        d.schemas[0].default = default;
        d.schemas[0].sensitive = true;
        if i == 5 {
            d.schemas[0].composition = SettingCompositionV1::MergeMap;
        }
        d.layers[0].values.clear();
        d.layers[1].values = vec![("unit".into(), SettingContributionV1::Set(valid))];
        let mut r = Recorder::disabled();
        let compiled = compile_settings_v1(&d, "engine", limits(), &mut r).unwrap();
        d.layers[1].values = vec![("unit".into(), SettingContributionV1::Set(invalid))];
        assert!(compile_settings_v1(&d, "engine", limits(), &mut r).is_err());
        let read = compiled.read(&id("document"), "unit").unwrap();
        assert!(read.sensitive);
        match i {
            0 => assert_eq!(read.value.boolean(), Some(true)),
            1 => assert!(read.value.number().unwrap().is_sign_negative()),
            2 => assert_eq!(read.value.text(), Some("hé🦀\r\n\u{feff}")),
            3 => assert_eq!(read.value.list_item(0).unwrap().number(), Some(1.5)),
            4 => assert_eq!(read.value.list_len(), Some(0)),
            _ => {
                assert_eq!(read.value.field("left").unwrap().boolean(), Some(true));
                assert_eq!(read.value.field("right").unwrap().number(), Some(1.0));
            }
        }
    }
}

#[test]
fn masked_equal_rank_conflicts_refuse_and_mathematical_zero_equality_preserves_native_bits() {
    let mut d = definition();
    d.layers[1].values = vec![(
        "unit".into(),
        SettingContributionV1::Set(SettingValueV1::Number(2.0)),
    )];
    d.layers.push(SettingLayerV1 {
        identity: id("peer"),
        scope: SettingScopeV1::Workspace,
        generation: 1,
        values: vec![(
            "unit".into(),
            SettingContributionV1::Set(SettingValueV1::Number(4.0)),
        )],
    });
    d.layers.push(SettingLayerV1 {
        identity: id("top"),
        scope: SettingScopeV1::Document,
        generation: 1,
        values: vec![(
            "unit".into(),
            SettingContributionV1::Set(SettingValueV1::Number(6.0)),
        )],
    });
    d.targets[0].layers.extend([id("peer"), id("top")]);
    let mut r = Recorder::disabled();
    assert_eq!(
        compile_settings_v1(&d, "engine", limits(), &mut r).err(),
        Some(SettingsError::Conflict)
    );
    d.layers[2].values = vec![(
        "unit".into(),
        SettingContributionV1::Set(SettingValueV1::Number(2.0)),
    )];
    let c = compile_settings_v1(&d, "engine", limits(), &mut r).unwrap();
    assert_eq!(
        c.read(&id("document"), "unit").unwrap().value.number(),
        Some(6.0)
    );
    d.targets[0].layers = vec![id("workspace"), id("peer")];
    d.layers[1].values = vec![(
        "unit".into(),
        SettingContributionV1::Set(SettingValueV1::Number(-0.0)),
    )];
    d.layers[2].values = vec![(
        "unit".into(),
        SettingContributionV1::Set(SettingValueV1::Number(0.0)),
    )];
    let c = compile_settings_v1(&d, "engine", limits(), &mut r).unwrap();
    let read = c.read(&id("document"), "unit").unwrap();
    assert!(read.value.number().unwrap().is_sign_negative());
    assert_eq!(read.origins.iter(None).unwrap().count(), 2);
    d.schemas[0].value_type = SettingTypeV1::Enum(vec![
        SettingValueV1::Number(0.0),
        SettingValueV1::Number(-0.0),
    ]);
    assert_eq!(
        compile_settings_v1(&d, "engine", limits(), &mut r).err(),
        Some(SettingsError::Duplicate)
    );
}

#[test]
fn merge_maps_use_only_declared_top_fields_with_independent_origins_not_recursive_overlay() {
    let mut d = definition();
    d.schemas[0].value_type = SettingTypeV1::Map(vec![
        ("left".into(), SettingTypeV1::Boolean),
        (
            "nested".into(),
            SettingTypeV1::Map(vec![
                ("one".into(), SettingTypeV1::Boolean),
                ("two".into(), SettingTypeV1::Boolean),
            ]),
        ),
    ]);
    d.schemas[0].default = SettingValueV1::Map(vec![
        ("left".into(), SettingValueV1::Boolean(false)),
        (
            "nested".into(),
            SettingValueV1::Map(vec![
                ("one".into(), SettingValueV1::Boolean(false)),
                ("two".into(), SettingValueV1::Boolean(false)),
            ]),
        ),
    ]);
    d.schemas[0].composition = SettingCompositionV1::MergeMap;
    d.layers[0].values = vec![(
        "unit".into(),
        SettingContributionV1::Set(SettingValueV1::Map(vec![(
            "left".into(),
            SettingValueV1::Boolean(true),
        )])),
    )];
    d.layers[1].values.clear();
    let mut r = Recorder::disabled();
    let c = compile_settings_v1(&d, "engine", limits(), &mut r).unwrap();
    let read = c.read(&id("document"), "unit").unwrap();
    assert_eq!(read.value.field("left").unwrap().boolean(), Some(true));
    assert!(matches!(
        read.origins.iter(Some("left")).unwrap().next(),
        Some(SettingOriginV1::Layer(_))
    ));
    assert!(matches!(
        read.origins.iter(Some("nested")).unwrap().next(),
        Some(SettingOriginV1::Library)
    ));
    d.layers[1].values = vec![(
        "unit".into(),
        SettingContributionV1::Set(SettingValueV1::Map(vec![(
            "nested".into(),
            SettingValueV1::Map(vec![("one".into(), SettingValueV1::Boolean(true))]),
        )])),
    )];
    assert_eq!(
        compile_settings_v1(&d, "engine", limits(), &mut r).err(),
        Some(SettingsError::InvalidValue)
    );
}

#[test]
fn finite_restrictions_apply_to_resolved_values_and_invalid_unused_contributions_still_refuse() {
    let mut d = definition();
    d.restrictions = vec![("unit".into(), vec![SettingValueV1::Number(3.0)])];
    let mut r = Recorder::disabled();
    compile_settings_v1(&d, "engine", limits(), &mut r).unwrap();
    d.layers[1].values = vec![("unit".into(), SettingContributionV1::Reset)];
    assert_eq!(
        compile_settings_v1(&d, "engine", limits(), &mut r).err(),
        Some(SettingsError::RestrictionDenied)
    );
    d.layers[1].values.clear();
    d.restrictions[0].1.push(SettingValueV1::Number(9.0));
    assert_eq!(
        compile_settings_v1(&d, "engine", limits(), &mut r).err(),
        Some(SettingsError::InvalidValue)
    );
    d.restrictions.clear();
    d.targets.clear();
    d.layers[1].values = vec![("theme".into(), SettingContributionV1::Reset)];
    assert_eq!(
        compile_settings_v1(&d, "engine", limits(), &mut r).err(),
        Some(SettingsError::UnknownKey)
    );
    d.layers[1].values = vec![(
        "unit".into(),
        SettingContributionV1::Set(SettingValueV1::Text("3".into())),
    )];
    assert_eq!(
        compile_settings_v1(&d, "engine", limits(), &mut r).err(),
        Some(SettingsError::InvalidValue)
    );
    d.layers[1].scope = SettingScopeV1::View;
    d.layers[1].values = vec![("unit".into(), SettingContributionV1::Reset)];
    assert_eq!(
        compile_settings_v1(&d, "engine", limits(), &mut r).err(),
        Some(SettingsError::IllegalScope)
    );
}

#[test]
fn malformed_identity_duplicate_and_association_products_do_not_compile() {
    let mut r = Recorder::disabled();
    for change in 0..9 {
        let mut d = definition();
        match change {
            0 => d.layers[0].identity.incarnation = 0,
            1 => d.layers[0].generation = 0,
            2 => d.targets[0].identity.authority = "foreign".into(),
            3 => d.targets[0].layers.push(id("host")),
            4 => d.targets[0].layers[0].incarnation = 2,
            5 => d.layers[0]
                .values
                .push(("unit".into(), SettingContributionV1::Reset)),
            6 => d.schemas[0].scopes.push(SettingScopeV1::Host),
            7 => d.schemas[0].composition = SettingCompositionV1::MergeMap,
            _ => {
                d.layers[0].values = vec![(
                    "unit".into(),
                    SettingContributionV1::Set(SettingValueV1::Number(f64::NAN)),
                )]
            }
        }
        assert!(compile_settings_v1(&d, "engine", limits(), &mut r).is_err());
    }
}

#[test]
fn scalar_resolution_matches_a_scope_oracle_for_all_choices_and_association_orders() {
    let scopes = [
        SettingScopeV1::Host,
        SettingScopeV1::Language,
        SettingScopeV1::Workspace,
        SettingScopeV1::Document,
        SettingScopeV1::View,
    ];
    let names = ["host", "language", "workspace", "document-layer", "view"];
    for highest in 0..5 {
        let mut d = definition();
        d.schemas[0].scopes = scopes.to_vec();
        d.layers = scopes
            .iter()
            .enumerate()
            .map(|(i, scope)| SettingLayerV1 {
                identity: id(names[i]),
                scope: *scope,
                generation: 1,
                values: if i <= highest {
                    vec![(
                        "unit".into(),
                        SettingContributionV1::Set(SettingValueV1::Number((i + 2) as f64)),
                    )]
                } else {
                    vec![]
                },
            })
            .collect();
        d.targets[0].layers = names.iter().rev().map(|name| id(name)).collect();
        let mut r = Recorder::disabled();
        let c = compile_settings_v1(&d, "engine", limits(), &mut r).unwrap();
        assert_eq!(
            c.read(&id("document"), "unit").unwrap().value.number(),
            Some((highest + 2) as f64)
        );
    }
    let orders = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    for encoded in 0..125 {
        for order in orders {
            let mut d = definition();
            d.layers.push(SettingLayerV1 {
                identity: id("top"),
                scope: SettingScopeV1::Document,
                generation: 1,
                values: vec![],
            });
            let names = ["host", "workspace", "top"];
            let mut code = encoded;
            let mut expected = 1.0;
            let mut origin = None;
            for (i, layer) in d.layers.iter_mut().enumerate() {
                let choice = code % 5;
                code /= 5;
                layer.values.clear();
                let value = match choice {
                    0 => None,
                    1 => Some((1.0, SettingContributionV1::Reset)),
                    _ => {
                        let value = [0.0, 2.0, 3.0][choice - 2];
                        Some((
                            value,
                            SettingContributionV1::Set(SettingValueV1::Number(value)),
                        ))
                    }
                };
                if let Some((value, entry)) = value {
                    layer.values.push(("unit".into(), entry));
                    expected = value;
                    origin = Some(names[i]);
                }
            }
            d.targets[0].layers = order.into_iter().map(|i| id(names[i])).collect();
            let mut r = Recorder::disabled();
            let c = compile_settings_v1(&d, "engine", limits(), &mut r).unwrap();
            let read = c.read(&id("document"), "unit").unwrap();
            assert_eq!(read.value.number(), Some(expected));
            match (read.origins.iter(None).unwrap().next().unwrap(), origin) {
                (SettingOriginV1::Library, None) => {}
                (SettingOriginV1::Layer(actual), Some(expected)) => assert_eq!(actual.id, expected),
                _ => panic!("wrong effective origin"),
            }
        }
    }
}

#[test]
fn structural_limits_and_deep_data_are_handled_without_recursive_engine_copy_or_drop() {
    let mut r = Recorder::disabled();
    for profile in [
        SettingsLimitsV1 {
            max_keys: 0,
            ..limits()
        },
        SettingsLimitsV1 {
            max_layers: 0,
            ..limits()
        },
        SettingsLimitsV1 {
            max_targets: 0,
            ..limits()
        },
        SettingsLimitsV1 {
            max_schema_nodes: 0,
            ..limits()
        },
        SettingsLimitsV1 {
            max_value_nodes: 0,
            ..limits()
        },
        SettingsLimitsV1 {
            max_state_bytes: 0,
            ..limits()
        },
        SettingsLimitsV1 {
            max_work: 0,
            ..limits()
        },
        SettingsLimitsV1 {
            max_identifier_bytes: 1,
            ..limits()
        },
    ] {
        assert!(compile_settings_v1(&definition(), "engine", profile, &mut r).is_err());
    }
    let baseline = compile_settings_v1(&definition(), "engine", limits(), &mut r).unwrap();
    let bytes = baseline.usage().logical_bytes;
    assert!(
        compile_settings_v1(
            &definition(),
            "engine",
            SettingsLimitsV1 {
                max_state_bytes: bytes - 1,
                ..limits()
            },
            &mut r
        )
        .is_err()
    );
    compile_settings_v1(
        &definition(),
        "engine",
        SettingsLimitsV1 {
            max_state_bytes: bytes,
            ..limits()
        },
        &mut r,
    )
    .unwrap();
    let depth = 4000;
    let mut d = definition();
    d.layers.iter_mut().for_each(|layer| layer.values.clear());
    d.targets[0].layers.clear();
    let mut ty = SettingTypeV1::Boolean;
    let mut value = SettingValueV1::Boolean(true);
    for _ in 0..depth {
        ty = SettingTypeV1::List(Box::new(ty));
        value = SettingValueV1::List(vec![value]);
    }
    d.schemas[0].value_type = ty;
    d.schemas[0].default = value;
    assert!(compile_settings_v1(&d, "engine", limits(), &mut r).is_err());
    let profile = SettingsLimitsV1 {
        max_schema_nodes: depth + 1,
        max_schema_depth: depth,
        max_value_nodes: depth + 1,
        max_value_depth: depth,
        ..limits()
    };
    let c = compile_settings_v1(&d, "engine", profile, &mut r).unwrap();
    let mut read = c.read(&id("document"), "unit").unwrap().value;
    for _ in 0..depth {
        read = read.list_item(0).unwrap();
    }
    assert_eq!(read.boolean(), Some(true));
    drop(c);
    // The caller owns recursive Rust inputs; dismantle its fixture iteratively too.
    let mut ty = std::mem::replace(&mut d.schemas[0].value_type, SettingTypeV1::Boolean);
    while let SettingTypeV1::List(child) = ty {
        ty = *child;
    }
    let mut value = std::mem::replace(&mut d.schemas[0].default, SettingValueV1::Null);
    while let SettingValueV1::List(mut children) = value {
        value = children.pop().unwrap();
    }
}

#[test]
fn initial_owner_installation_and_queries_preserve_sources_and_refuse_replacement_and_foreign_handles()
 {
    for capacity in [None, Some(128), Some(1)] {
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let compiled = compile_settings_v1(&definition(), "engine", limits(), &mut r).unwrap();
        let mut o = SourceCoordinator::new(CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 128,
            source: Limits {
                max_source_bytes: 128,
                max_edits: 8,
                max_edit_payload_bytes: 256,
            },
        });
        let a = o.create(b"A", &mut r).unwrap();
        let p = o
            .prepare(
                &[DocumentEdits {
                    expected: a.clone(),
                    edits: vec![Replacement {
                        start: 0,
                        end: 1,
                        expected: "A".into(),
                        inserted: "a".into(),
                    }],
                }],
                &[],
                &mut r,
            )
            .unwrap();
        let g = o.generation();
        let basis = o.install_initial_settings(compiled, &mut r).unwrap();
        assert_eq!(basis.schema_generation, 1);
        assert_eq!(o.generation(), g + 1);
        let target = o.settings_target(&id("document"), &mut r).unwrap();
        assert_eq!(
            o.settings_read(&target, "unit", &mut r)
                .unwrap()
                .value
                .number(),
            Some(3.0)
        );
        let g = o.generation();
        assert_eq!(
            o.install_initial_settings(
                compile_settings_v1(&definition(), "engine", limits(), &mut r).unwrap(),
                &mut r
            ),
            Err(SettingsError::Unavailable)
        );
        assert_eq!(o.generation(), g);
        assert_eq!(o.read(&a.document, &mut r).unwrap().version, a);
        o.commit(p, &mut r).unwrap();
        let foreign = SourceCoordinator::new(CoordinatorLimits {
            max_document_incarnations: 1,
            max_participants: 1,
            max_staged_bytes: 128,
            source: Limits {
                max_source_bytes: 128,
                max_edits: 1,
                max_edit_payload_bytes: 256,
            },
        });
        assert_eq!(
            foreign.settings_read(&target, "unit", &mut r).err(),
            Some(SettingsError::Unavailable)
        );
        assert_eq!(
            o.settings_read(&target, "unknown", &mut r).err(),
            Some(SettingsError::UnknownKey)
        );
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        assert!(!format!("{events:?}").contains("unit"));
        if capacity == Some(128) {
            assert!(
                events
                    .iter()
                    .any(|e| e.boundary == Boundary::SettingsInstallation)
            );
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
    }
}
