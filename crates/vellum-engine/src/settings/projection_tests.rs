use super::*;
use crate::instrumentation::{Boundary, NativeTimingProfileV1, Recorder};
fn id(s: &str) -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: s.into(),
        incarnation: 1,
    }
}
fn state(hidden: bool, r: &mut Recorder) -> CompiledSettingsV1 {
    compile_settings_v1(
        &SettingsDefinitionV1 {
            schemas: vec![
                SettingSchemaV1 {
                    key: "public".into(),
                    value_type: SettingTypeV1::Enum(vec![
                        SettingValueV1::Text("a".into()),
                        SettingValueV1::Text("b".into()),
                    ]),
                    default: SettingValueV1::Text("a".into()),
                    domain: SettingDomainV1::Source,
                    scopes: vec![SettingScopeV1::Host],
                    composition: SettingCompositionV1::Replace,
                    sensitive: true,
                },
                SettingSchemaV1 {
                    key: "private".into(),
                    value_type: SettingTypeV1::Boolean,
                    default: SettingValueV1::Boolean(hidden),
                    domain: SettingDomainV1::Analysis,
                    scopes: vec![SettingScopeV1::Host],
                    composition: SettingCompositionV1::Replace,
                    sensitive: false,
                },
            ],
            layers: vec![SettingLayerV1 {
                identity: id("host"),
                scope: SettingScopeV1::Host,
                generation: 1,
                values: vec![(
                    "public".into(),
                    SettingContributionV1::Set(SettingValueV1::Text("b".into())),
                )],
            }],
            targets: vec![SettingTargetV1 {
                identity: id("target"),
                layers: vec![id("host")],
            }],
            restrictions: vec![],
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
            max_work: 8192,
        },
        r,
    )
    .unwrap()
}
fn limits() -> SettingsProjectionLimitsV1 {
    SettingsProjectionLimitsV1 {
        max_bytes: 16384,
        max_work: 8192,
        max_depth: 16,
    }
}
#[test]
fn projection_is_complete_ordered_independently_filtered_owned_and_bounded() {
    for hidden in [false, true] {
        for mode in 0..4 {
            let mut setup = Recorder::disabled();
            let state = state(hidden, &mut setup);
            let origin = id("host");
            // Facts are deliberately not in schema order. The compiler's accepted order wins.
            let grants = [
                SettingProjectionGrantV1 {
                    key: "private",
                    visible: false,
                    value: false,
                    origins: false,
                    declaration: false,
                    origin_ids: &[],
                },
                SettingProjectionGrantV1 {
                    key: "public",
                    visible: true,
                    value: true,
                    origins: true,
                    declaration: true,
                    origin_ids: &[SettingProjectionOriginV1::Layer(origin)],
                },
            ];
            let mut r = if mode == 0 {
                Recorder::disabled()
            } else {
                Recorder::try_with_capacity(if mode == 3 { 1 } else { 64 }).unwrap()
            };
            if mode >= 2 {
                r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            }
            if mode == 3 {
                r.record(crate::instrumentation::Observation {
                    boundary: Boundary::Query,
                    disposition: crate::instrumentation::Disposition::Completed,
                    service_time: None,
                });
            }
            let result =
                project_settings_v1(&state, &id("target"), &grants, true, limits(), &mut r)
                    .unwrap();
            assert_eq!(result.rows.len(), 1);
            let row = &result.rows[0];
            assert_eq!(row.key, "public");
            assert_eq!(row.value.as_ref().unwrap().read().text(), Some("b"));
            assert!(row.declaration.as_ref().unwrap().sensitive);
            let ty = row.declaration.as_ref().unwrap().value_type.read();
            assert_eq!(ty.enum_len(), Some(2));
            assert_eq!(ty.enum_value(0).unwrap().text(), Some("a"));
            let Some(SettingProjectionOriginsV1::Whole(origins)) = &row.origins else {
                panic!()
            };
            assert_eq!(origins, &grants[1].origin_ids);
            assert!(format!("{result:?}").contains("redacted"));
            let bytes = result.logical_bytes;
            assert!(
                project_settings_v1(
                    &state,
                    &id("target"),
                    &grants,
                    true,
                    SettingsProjectionLimitsV1 {
                        max_bytes: bytes - 1,
                        ..limits()
                    },
                    &mut r
                )
                .is_err()
            );
            assert_eq!(
                project_settings_v1(
                    &state,
                    &id("target"),
                    &grants,
                    true,
                    SettingsProjectionLimitsV1 {
                        max_bytes: bytes,
                        ..limits()
                    },
                    &mut r
                )
                .unwrap()
                .logical_bytes,
                bytes
            );
            assert!(
                project_settings_v1(
                    &state,
                    &id("target"),
                    &grants,
                    true,
                    SettingsProjectionLimitsV1 {
                        max_work: 0,
                        ..limits()
                    },
                    &mut r
                )
                .is_err()
            );
            let mut successful = 0;
            while let Some(event) = r.pop_front() {
                if event.boundary == Boundary::SettingsProjection {
                    assert_eq!(event.service_time.is_some(), mode == 2);
                    successful += usize::from(
                        event.disposition == crate::instrumentation::Disposition::Completed,
                    );
                }
            }
            if mode == 1 || mode == 2 {
                assert_eq!(successful, 2);
            }
            if mode == 3 {
                assert!(r.dropped() > 0);
            }
            if mode == 0 {
                assert_eq!(r.dropped(), 0);
            }
        }
    }
}
#[test]
fn projection_rejects_incomplete_duplicate_unknown_and_fabricated_origin_facts() {
    let mut r = Recorder::disabled();
    let state = state(false, &mut r);
    for case in 0..6 {
        let origin = [SettingProjectionOriginV1::Library];
        let mut grants = vec![
            SettingProjectionGrantV1 {
                key: "public",
                visible: true,
                value: true,
                origins: false,
                declaration: false,
                origin_ids: &[],
            },
            SettingProjectionGrantV1 {
                key: "private",
                visible: false,
                value: false,
                origins: false,
                declaration: false,
                origin_ids: &[],
            },
        ];
        match case {
            0 => {
                grants.pop();
            }
            1 => grants[1].key = "public",
            2 => grants[1].key = "unknown",
            3 => grants[1].value = true,
            4 => grants[0].origin_ids = &origin,
            5 => {
                grants[0].origins = true;
                grants[0].origin_ids = &origin;
            }
            _ => unreachable!(),
        }
        assert!(
            project_settings_v1(&state, &id("target"), &grants, false, limits(), &mut r).is_err()
        );
    }
    let grants = [
        SettingProjectionGrantV1 {
            key: "public",
            visible: true,
            value: false,
            origins: false,
            declaration: false,
            origin_ids: &[],
        },
        SettingProjectionGrantV1 {
            key: "private",
            visible: false,
            value: false,
            origins: false,
            declaration: false,
            origin_ids: &[],
        },
    ];
    let result =
        project_settings_v1(&state, &id("target"), &grants, false, limits(), &mut r).unwrap();
    assert_eq!(result.rows.len(), 1);
    assert!(
        result.rows[0].value.is_none()
            && result.rows[0].origins.is_none()
            && result.rows[0].declaration.is_none()
    );
    assert!(!result.capabilities);
    for mask in 0..8 {
        let origin = [SettingProjectionOriginV1::Layer(id("host"))];
        let grants = [
            SettingProjectionGrantV1 {
                key: "private",
                visible: true,
                value: true,
                origins: false,
                declaration: false,
                origin_ids: &[],
            },
            SettingProjectionGrantV1 {
                key: "public",
                visible: true,
                value: mask & 1 != 0,
                origins: mask & 2 != 0,
                declaration: mask & 4 != 0,
                origin_ids: if mask & 2 != 0 { &origin } else { &[] },
            },
        ];
        let projection =
            project_settings_v1(&state, &id("target"), &grants, true, limits(), &mut r).unwrap();
        assert_eq!(
            projection
                .rows
                .iter()
                .map(|r| r.key.as_str())
                .collect::<Vec<_>>(),
            ["public", "private"]
        );
        assert_eq!(projection.rows[0].value.is_some(), mask & 1 != 0);
        assert_eq!(projection.rows[0].origins.is_some(), mask & 2 != 0);
        assert_eq!(projection.rows[0].declaration.is_some(), mask & 4 != 0);
        assert_eq!(
            projection.rows[1].value.as_ref().unwrap().read().boolean(),
            Some(false)
        );
    }
}

#[test]
fn projection_filters_merge_map_origins_per_field_and_copies_nested_declarations() {
    let mut r = Recorder::disabled();
    let profile = state(false, &mut r).limits;
    let s = compile_settings_v1(
        &SettingsDefinitionV1 {
            schemas: vec![SettingSchemaV1 {
                key: "map".into(),
                value_type: SettingTypeV1::Map(vec![
                    ("a".into(), SettingTypeV1::Integer { min: 0, max: 9 }),
                    (
                        "b".into(),
                        SettingTypeV1::List(Box::new(SettingTypeV1::Text)),
                    ),
                ]),
                default: SettingValueV1::Map(vec![
                    ("a".into(), SettingValueV1::Number(0.0)),
                    (
                        "b".into(),
                        SettingValueV1::List(vec![SettingValueV1::Text("library".into())]),
                    ),
                ]),
                domain: SettingDomainV1::Source,
                scopes: vec![SettingScopeV1::Host],
                composition: SettingCompositionV1::MergeMap,
                sensitive: false,
            }],
            restrictions: vec![],
            layers: vec![SettingLayerV1 {
                identity: id("host"),
                scope: SettingScopeV1::Host,
                generation: 1,
                values: vec![(
                    "map".into(),
                    SettingContributionV1::Set(SettingValueV1::Map(vec![(
                        "a".into(),
                        SettingValueV1::Number(3.0),
                    )])),
                )],
            }],
            targets: vec![SettingTargetV1 {
                identity: id("target"),
                layers: vec![id("host")],
            }],
        },
        "engine",
        profile,
        &mut r,
    )
    .unwrap();
    let origin = [SettingProjectionOriginV1::Library];
    let grants = [SettingProjectionGrantV1 {
        key: "map",
        visible: true,
        value: true,
        origins: true,
        declaration: true,
        origin_ids: &origin,
    }];
    let projection =
        project_settings_v1(&s, &id("target"), &grants, true, limits(), &mut r).unwrap();
    let row = &projection.rows[0];
    let values = row.value.as_ref().unwrap().read();
    assert_eq!(values.map_len(), Some(2));
    assert_eq!(values.field_at(0).unwrap().0, "a");
    assert_eq!(values.field("a").unwrap().number(), Some(3.0));
    assert_eq!(
        values.field("b").unwrap().list_item(0).unwrap().text(),
        Some("library")
    );
    let Some(SettingProjectionOriginsV1::Fields(origins)) = &row.origins else {
        panic!()
    };
    assert_eq!(origins[0].0, "a");
    assert!(origins[0].1.is_empty());
    assert_eq!(origins[1].0, "b");
    assert_eq!(origins[1].1, origin);
    assert_eq!(row.origins.as_ref().unwrap().coverage(), "permitted-only");
    assert_eq!(
        row.declaration
            .as_ref()
            .unwrap()
            .value_type
            .read()
            .field("b")
            .unwrap()
            .list_item()
            .unwrap()
            .kind(),
        SettingProjectionTypeKindV1::Text
    );
    assert!(
        project_settings_v1(
            &s,
            &id("target"),
            &grants,
            true,
            SettingsProjectionLimitsV1 {
                max_depth: 1,
                ..limits()
            },
            &mut r
        )
        .is_err()
    );
    assert!(
        project_settings_v1(
            &s,
            &id("target"),
            &grants,
            true,
            SettingsProjectionLimitsV1 {
                max_depth: 3,
                ..limits()
            },
            &mut r
        )
        .is_ok()
    );
}

#[test]
fn deep_projection_success_refusal_and_drop_use_flat_storage_on_a_small_stack() {
    let depth = 4000;
    let mut ty = SettingTypeV1::Boolean;
    let mut data = SettingValueV1::Boolean(true);
    for _ in 0..depth {
        ty = SettingTypeV1::List(Box::new(ty));
        data = SettingValueV1::List(vec![data]);
    }
    let mut definition = SettingsDefinitionV1 {
        schemas: vec![SettingSchemaV1 {
            key: "deep".into(),
            value_type: ty,
            default: data,
            domain: SettingDomainV1::Source,
            scopes: vec![SettingScopeV1::Host],
            composition: SettingCompositionV1::Replace,
            sensitive: true,
        }],
        restrictions: vec![],
        layers: vec![],
        targets: vec![SettingTargetV1 {
            identity: id("target"),
            layers: vec![],
        }],
    };
    let mut r = Recorder::disabled();
    let compiled = compile_settings_v1(
        &definition,
        "engine",
        SettingsLimitsV1 {
            max_keys: 1,
            max_layers: 1,
            max_targets: 1,
            max_identifier_bytes: 64,
            max_schema_nodes: depth + 1,
            max_schema_depth: depth,
            max_value_nodes: depth + 1,
            max_value_depth: depth,
            max_state_bytes: 16_777_216,
            max_work: 1_000_000,
        },
        &mut r,
    )
    .unwrap();
    // Recursive input ownership stays at the caller edge; dismantle this fixture
    // without asking the standard recursive enum destructor to walk its depth.
    let mut ty = std::mem::replace(
        &mut definition.schemas[0].value_type,
        SettingTypeV1::Boolean,
    );
    while let SettingTypeV1::List(child) = ty {
        ty = *child;
    }
    let mut data = std::mem::replace(&mut definition.schemas[0].default, SettingValueV1::Null);
    while let SettingValueV1::List(mut children) = data {
        data = children.pop().unwrap();
    }
    drop(definition);
    std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(move || {
            let grants = [SettingProjectionGrantV1 {
                key: "deep",
                visible: true,
                value: true,
                origins: false,
                declaration: true,
                origin_ids: &[],
            }];
            let limits = SettingsProjectionLimitsV1 {
                max_bytes: 16_777_216,
                max_work: 1_000_000,
                max_depth: depth,
            };
            let mut r = Recorder::disabled();
            assert!(
                project_settings_v1(
                    &compiled,
                    &id("target"),
                    &grants,
                    true,
                    SettingsProjectionLimitsV1 {
                        max_depth: depth - 1,
                        ..limits
                    },
                    &mut r
                )
                .is_err()
            );
            let projection =
                project_settings_v1(&compiled, &id("target"), &grants, true, limits, &mut r)
                    .unwrap();
            drop(compiled); // Output must own its data, not borrow the old root.
            let mut value = projection.rows[0].value.as_ref().unwrap().read();
            let mut ty = projection.rows[0]
                .declaration
                .as_ref()
                .unwrap()
                .value_type
                .read();
            for _ in 0..depth {
                value = value.list_item(0).unwrap();
                ty = ty.list_item().unwrap();
            }
            assert_eq!(value.boolean(), Some(true));
            assert_eq!(ty.kind(), SettingProjectionTypeKindV1::Boolean);
            drop(projection); // Both successful and partial-failure graphs drop flat.
        })
        .unwrap()
        .join()
        .unwrap();
}
