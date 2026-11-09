use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::settings::*;
use vellum_engine::source::Limits;
#[path = "settings_activation_behaviors/context_dispatch.rs"]
mod context_dispatch;
#[path = "support/discovered_context_fixture.rs"]
mod discovered_context;
#[path = "support/inspection_read_fixture.rs"]
mod inspection_read;
#[path = "support/invocation_observation_fixture.rs"]
mod observation_support;
#[path = "support/selection_discovery_fixture.rs"]
mod selection_discovery;

fn native_service_recorder() -> Recorder {
    Recorder::try_with_capacity(256)
        .unwrap()
        .with_native_timing(
            vellum_engine::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
        )
}

fn assert_native_service_records(
    r: &mut Recorder,
    expected: &[(
        vellum_engine::instrumentation::Boundary,
        vellum_engine::instrumentation::Disposition,
    )],
) {
    let mut actual = vec![];
    while let Some(v) = r.pop_front() {
        if v.service_time.is_some() {
            actual.push((v.boundary, v.disposition));
        }
    }
    assert_eq!(actual, expected);
}
#[path = "settings_activation_behaviors/cancellation.rs"]
mod cancellation;
#[path = "settings_activation_behaviors/created_results.rs"]
mod created_results;
#[path = "settings_activation_behaviors/definition.rs"]
mod definition;
#[path = "settings_activation_behaviors/layers.rs"]
mod layers;
#[path = "settings_activation_behaviors/lifecycle_basis.rs"]
mod lifecycle_basis;
#[path = "settings_activation_behaviors/native_definition.rs"]
mod native_definition;
#[path = "settings_activation_behaviors/native_guarded.rs"]
mod native_guarded;
#[path = "settings_activation_behaviors/native_lifecycle.rs"]
mod native_lifecycle;
#[path = "settings_activation_behaviors/reads.rs"]
mod reads;
#[path = "settings_activation_behaviors/retained.rs"]
mod retained;
#[path = "settings_activation_behaviors/root_admission.rs"]
mod root_admission;
#[path = "settings_activation_behaviors/scoped.rs"]
mod scoped;
#[path = "settings_activation_behaviors/scoped_basis.rs"]
mod scoped_basis;
#[path = "settings_activation_behaviors/scoped_definition.rs"]
mod scoped_definition;
#[path = "settings_activation_behaviors/scoped_lifecycle.rs"]
mod scoped_lifecycle;
#[path = "settings_activation_behaviors/settings_reads.rs"]
mod settings_reads;
#[path = "settings_activation_behaviors/source_context.rs"]
mod source_context;
fn id(name: &str) -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: name.into(),
        incarnation: 1,
    }
}
fn profile() -> SettingsActivationLimitsV1 {
    SettingsActivationLimitsV1 {
        max_changes: 16,
        max_request_nodes: 128,
        max_request_depth: 64,
        max_request_bytes: 65536,
        max_plan_bytes: 1_048_576,
        max_work: 1_048_576,
    }
}
fn setup() -> (SourceCoordinator, Recorder) {
    setup_with(profile(), false)
}
fn setup_with(
    activation: SettingsActivationLimitsV1,
    all_domains: bool,
) -> (SourceCoordinator, Recorder) {
    setup_with_root_budget(activation, all_domains, Some(1_048_576))
}
fn setup_with_root_budget(
    activation: SettingsActivationLimitsV1,
    all_domains: bool,
    root_budget: Option<usize>,
) -> (SourceCoordinator, Recorder) {
    let mut o = SourceCoordinator::configured(CoordinatorConfig {
        limits: CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 1024,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 16,
                max_edit_payload_bytes: 4096,
            },
        },
        invocation: Some(InvocationLimits {
            max_lifetime_issued: 16,
            max_retained: 16,
            max_intent_bytes: 65536,
        }),
        annotations: Some(AnnotationLimits {
            max_issued: 8,
            max_records: 8,
            max_body_bytes: 128,
            max_retained_body_bytes: 1024,
            max_mapping_steps: 4096,
            max_live_snapshots: 8,
            max_snapshot_bytes: 4096,
        }),
        history: None,
        resources: None,
        search: None,
    });
    let mut r = Recorder::disabled();
    let mut definition = SettingsDefinitionV1 {
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
        layers: vec![
            SettingLayerV1 {
                identity: id("host"),
                scope: SettingScopeV1::Host,
                generation: 1,
                values: vec![],
            },
            SettingLayerV1 {
                identity: id("left"),
                scope: SettingScopeV1::Workspace,
                generation: 1,
                values: vec![],
            },
            SettingLayerV1 {
                identity: id("right"),
                scope: SettingScopeV1::Workspace,
                generation: 1,
                values: vec![],
            },
        ],
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
    };
    if all_domains {
        for (key, domain) in [
            ("codec", SettingDomainV1::Serialization),
            ("analysis", SettingDomainV1::Analysis),
        ] {
            definition.schemas.push(SettingSchemaV1 {
                key: key.into(),
                value_type: SettingTypeV1::Integer { min: 0, max: 8 },
                default: SettingValueV1::Number(1.0),
                domain,
                scopes: vec![SettingScopeV1::Host, SettingScopeV1::Workspace],
                composition: SettingCompositionV1::Replace,
                sensitive: false,
            });
        }
    }
    let compiled = compile_settings_v1(
        &definition,
        "engine",
        SettingsLimitsV1 {
            max_keys: 16,
            max_layers: 16,
            max_targets: 16,
            max_identifier_bytes: 128,
            max_schema_nodes: 128,
            max_schema_depth: 64,
            max_value_nodes: 128,
            max_value_depth: 64,
            max_state_bytes: 65536,
            max_work: 1_048_576,
        },
        &mut r,
    )
    .unwrap();
    o.install_initial_settings(compiled, &mut r).unwrap();
    if let Some(bytes) = root_budget {
        o.configure_semantic_root_budget(bytes, &mut r).unwrap();
    }
    o.configure_settings_activation(activation, &mut r).unwrap();
    (o, r)
}
fn change(layer: &str, expected: u64, operation: SettingOperationV1) -> SettingChangeV1 {
    SettingChangeV1 {
        layer: id(layer),
        expected_generation: expected,
        key: "unit".into(),
        operation,
    }
}
#[test]
fn existing_target_handles_keep_their_meaning_through_repeated_actual_root_rebuilds() {
    let (mut o, mut r) = setup();
    let a = o.settings_target(&id("A"), &mut r).unwrap();
    let b = o.settings_target(&id("B"), &mut r).unwrap();
    for step in 0..32 {
        let value = f64::from(2 + step % 7);
        set(&mut o, "left", value, &mut r);
        assert_eq!(o.settings_target(&id("A"), &mut r).unwrap(), a);
        assert_eq!(o.settings_target(&id("B"), &mut r).unwrap(), b);
        assert_eq!(
            o.settings_read(&a, "unit", &mut r).unwrap().value.number(),
            Some(value)
        );
        assert_eq!(
            o.settings_read(&b, "unit", &mut r).unwrap().value.number(),
            Some(1.0)
        );
        assert_eq!(
            o.settings_generations(&a, &mut r).unwrap().source,
            2 + u64::try_from(step).unwrap()
        );
        assert_eq!(o.settings_generations(&b, &mut r).unwrap().source, 1);
    }
    assert_eq!(o.settings_retention_usage().retained, 0);
    assert_eq!(o.settings_retention_usage().issued_lifetime, 0);
}
#[test]
fn settings_invocation_copies_intent_executes_once_and_replay_never_resamples() {
    let (mut o, mut r) = setup();
    let invocation = o.issue_invocation(&mut r).unwrap();
    let mut changes = [change(
        "left",
        1,
        SettingOperationV1::Set(SettingValueV1::Number(2.0)),
    )];
    assert_eq!(
        o.submit_settings_change(&invocation, &changes, &mut r),
        Ok(InvocationState::Pending)
    );
    changes[0].operation = SettingOperationV1::Set(SettingValueV1::Number(3.0));
    assert_eq!(
        o.submit_settings_change(&invocation, &changes, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    assert_eq!(read(&o, "A", &mut r), 1.0);
    changes[0].operation = SettingOperationV1::Set(SettingValueV1::Number(2.0));
    assert_eq!(
        o.submit_settings_change(&invocation, &changes, &mut r),
        Ok(InvocationState::Pending)
    );
    o.run(&invocation, &mut r).unwrap();
    let generation = o.generation();
    let InvocationOutcome::SettingsCommitted(outcome) =
        o.invocation_outcome(&invocation, &mut r).unwrap()
    else {
        panic!("settings outcome")
    };
    assert!(outcome.changed);
    assert_eq!(outcome.changed_layers, 1);
    assert_eq!(outcome.basis.configuration_generation, 2);
    set(&mut o, "left", 4.0, &mut r);
    assert_eq!(
        o.submit_settings_change(&invocation, &changes, &mut r),
        Ok(InvocationState::Finished)
    );
    o.run(&invocation, &mut r).unwrap();
    assert_eq!(o.generation(), generation + 1);
    assert_eq!(read(&o, "A", &mut r), 4.0);
    let InvocationOutcome::SettingsCommitted(original) =
        o.invocation_outcome(&invocation, &mut r).unwrap()
    else {
        panic!("historical settings outcome")
    };
    assert_eq!(original.basis.configuration_generation, 2);
}
#[test]
fn settings_jobs_merge_disjoint_service_state_and_retain_stale_refusals() {
    for layer in ["right", "host", "left"] {
        let (mut o, mut r) = setup();
        let invocation = o.issue_invocation(&mut r).unwrap();
        let changes = [change(
            "left",
            1,
            SettingOperationV1::Set(SettingValueV1::Number(2.0)),
        )];
        o.submit_settings_change(&invocation, &changes, &mut r)
            .unwrap();
        set(&mut o, layer, 3.0, &mut r);
        let generation = o.generation();
        o.run(&invocation, &mut r).unwrap();
        if layer == "right" {
            assert!(matches!(
                o.invocation_outcome(&invocation, &mut r),
                Ok(InvocationOutcome::SettingsCommitted(_))
            ));
            assert_eq!(read(&o, "A", &mut r), 2.0);
            assert_eq!(read(&o, "B", &mut r), 3.0);
            assert_eq!(o.generation(), generation + 1);
        } else {
            assert_eq!(
                o.invocation_outcome(&invocation, &mut r),
                Ok(&InvocationOutcome::SettingsRejected(SettingsError::Stale))
            );
            assert_eq!(o.generation(), generation);
            o.submit_settings_change(&invocation, &changes, &mut r)
                .unwrap();
            o.run(&invocation, &mut r).unwrap();
            assert_eq!(o.generation(), generation);
        }
        assert!(!o.is_faulted());
    }
}
#[test]
fn settings_retry_preserves_signed_zero_change_order_and_unordered_map_meaning() {
    let (mut o, mut r) = setup();
    let invocation = o.issue_invocation(&mut r).unwrap();
    let minus = [change(
        "left",
        1,
        SettingOperationV1::Set(SettingValueV1::Number(-0.0)),
    )];
    let plus = [change(
        "left",
        1,
        SettingOperationV1::Set(SettingValueV1::Number(0.0)),
    )];
    o.submit_settings_change(&invocation, &minus, &mut r)
        .unwrap();
    assert_eq!(
        o.submit_settings_change(&invocation, &plus, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    o.run(&invocation, &mut r).unwrap();
    assert!(read(&o, "A", &mut r).is_sign_negative());
    let map = |reverse| {
        let mut fields = vec![
            ("a".into(), SettingValueV1::Number(1.0)),
            ("b".into(), SettingValueV1::Number(2.0)),
        ];
        if reverse {
            fields.reverse()
        }
        [change(
            "left",
            2,
            SettingOperationV1::Set(SettingValueV1::Map(fields)),
        )]
    };
    let invocation = o.issue_invocation(&mut r).unwrap();
    o.submit_settings_change(&invocation, &map(false), &mut r)
        .unwrap();
    assert_eq!(
        o.submit_settings_change(&invocation, &map(true), &mut r),
        Ok(InvocationState::Pending)
    );
    o.run(&invocation, &mut r).unwrap();
    assert_eq!(
        o.invocation_outcome(&invocation, &mut r),
        Ok(&InvocationOutcome::SettingsRejected(
            SettingsError::InvalidValue
        ))
    );
    let invocation = o.issue_invocation(&mut r).unwrap();
    let mut changes = [
        change("left", 2, SettingOperationV1::Remove),
        change("right", 1, SettingOperationV1::Reset),
    ];
    o.submit_settings_change(&invocation, &changes, &mut r)
        .unwrap();
    changes.reverse();
    assert_eq!(
        o.submit_settings_change(&invocation, &changes, &mut r),
        Err(InvocationError::IntentMismatch)
    );
}
#[test]
fn settings_stop_forget_retire_and_cross_family_checks_never_reopen_work() {
    let (mut o, mut r) = setup();
    let invocation = o.issue_invocation(&mut r).unwrap();
    let changes = [change("left", 1, SettingOperationV1::Reset)];
    o.submit_settings_change(&invocation, &changes, &mut r)
        .unwrap();
    assert_eq!(
        o.retire_invocation(&invocation, &mut r),
        Err(InvocationError::Pending)
    );
    assert_eq!(
        o.submit_relation(&invocation, &RelationCommandV1::CreateWorkspace, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    o.stop_invocation(&invocation, &mut r).unwrap();
    assert_eq!(
        o.invocation_outcome(&invocation, &mut r),
        Ok(&InvocationOutcome::StoppedBeforePublication)
    );
    o.forget_invocation_outcome(&invocation, &mut r).unwrap();
    assert_eq!(
        o.submit_settings_change(&invocation, &changes, &mut r),
        Ok(InvocationState::Finished)
    );
    o.run(&invocation, &mut r).unwrap();
    assert_eq!(
        o.invocation_outcome(&invocation, &mut r),
        Err(InvocationError::OutcomeUnavailable)
    );
    assert_eq!(read(&o, "A", &mut r), 1.0);
    o.retire_invocation(&invocation, &mut r).unwrap();
    assert_eq!(
        o.submit_settings_change(&invocation, &changes, &mut r),
        Err(InvocationError::Unavailable)
    );
    let (mut foreign, mut fr) = setup();
    assert_eq!(
        foreign.submit_settings_change(&invocation, &changes, &mut fr),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(o.invocation_usage().issued_lifetime, 1);
    assert_eq!(o.invocation_usage().retained, 0);
}
#[test]
fn equal_default_origin_is_metadata_change_not_source_semantic_change() {
    let (mut o, mut r) = setup();
    let target = o.settings_target(&id("A"), &mut r).unwrap();
    let before = o.settings_generations(&target, &mut r).unwrap();
    let p = o
        .prepare_settings_change(
            &[change(
                "left",
                1,
                SettingOperationV1::Set(SettingValueV1::Number(1.0)),
            )],
            &mut r,
        )
        .unwrap();
    let result = o.commit_settings_change(p, &mut r).unwrap();
    assert!(result.changed);
    assert_eq!(o.settings_generations(&target, &mut r).unwrap(), before);
    assert_eq!(o.settings_layer_generation(&id("left"), &mut r).unwrap(), 2);
    assert_eq!(result.basis.configuration_generation, 2);
}
fn set(o: &mut SourceCoordinator, layer: &str, value: f64, r: &mut Recorder) -> SettingsUpdateV1 {
    let generation = o.settings_layer_generation(&id(layer), r).unwrap();
    let p = o
        .prepare_settings_change(
            &[change(
                layer,
                generation,
                SettingOperationV1::Set(SettingValueV1::Number(value)),
            )],
            r,
        )
        .unwrap();
    o.commit_settings_change(p, r).unwrap()
}
fn read(o: &SourceCoordinator, target: &str, r: &mut Recorder) -> f64 {
    let target = o.settings_target(&id(target), r).unwrap();
    o.settings_read(&target, "unit", r)
        .unwrap()
        .value
        .number()
        .unwrap()
}
#[test]
fn disjoint_plans_merge_into_current_state_while_changed_contributors_stale() {
    for layer in ["right", "host", "left"] {
        let (mut o, mut r) = setup();
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
        set(&mut o, layer, 3.0, &mut r);
        let g = o.generation();
        let result = o.commit_settings_change(p, &mut r);
        if layer == "right" {
            assert!(result.unwrap().changed);
            assert_eq!(read(&o, "A", &mut r), 2.0);
            assert_eq!(read(&o, "B", &mut r), 3.0);
            assert_eq!(
                o.settings_basis(&mut r).unwrap().configuration_generation,
                3
            );
        } else {
            assert_eq!(result, Err(SettingsError::Stale));
            assert_eq!(o.generation(), g);
        }
    }
}
#[test]
fn remove_inherits_reset_selects_library_and_identity_preserves_all_versions() {
    let (mut o, mut r) = setup();
    set(&mut o, "host", 3.0, &mut r);
    set(&mut o, "left", 2.0, &mut r);
    let target = o.settings_target(&id("A"), &mut r).unwrap();
    let before = o.settings_generations(&target, &mut r).unwrap();
    let basis = o.settings_basis(&mut r).unwrap();
    let g = o.generation();
    let p = o
        .prepare_settings_change(
            &[change(
                "left",
                2,
                SettingOperationV1::Set(SettingValueV1::Number(2.0)),
            )],
            &mut r,
        )
        .unwrap();
    assert!(!o.commit_settings_change(p, &mut r).unwrap().changed);
    assert_eq!(o.generation(), g);
    assert_eq!(o.settings_basis(&mut r).unwrap(), basis);
    assert_eq!(o.settings_generations(&target, &mut r).unwrap(), before);
    let p = o
        .prepare_settings_change(&[change("left", 2, SettingOperationV1::Remove)], &mut r)
        .unwrap();
    o.commit_settings_change(p, &mut r).unwrap();
    assert_eq!(read(&o, "A", &mut r), 3.0);
    let p = o
        .prepare_settings_change(&[change("left", 3, SettingOperationV1::Reset)], &mut r)
        .unwrap();
    o.commit_settings_change(p, &mut r).unwrap();
    assert_eq!(read(&o, "A", &mut r), 1.0);
    let g = o.generation();
    let p = o
        .prepare_settings_change(&[change("right", 1, SettingOperationV1::Remove)], &mut r)
        .unwrap();
    assert!(!o.commit_settings_change(p, &mut r).unwrap().changed);
    assert_eq!(o.generation(), g);
}
#[test]
fn each_changed_domain_advances_once_and_peer_target_versions_are_preserved() {
    let (mut o, mut r) = setup_with(profile(), true);
    let a = o.settings_target(&id("A"), &mut r).unwrap();
    let b = o.settings_target(&id("B"), &mut r).unwrap();
    let b_before = o.settings_generations(&b, &mut r).unwrap();
    let changes: Vec<_> = [("unit", 2.0), ("codec", 3.0), ("analysis", 4.0)]
        .into_iter()
        .map(|(key, value)| SettingChangeV1 {
            key: key.into(),
            ..change(
                "left",
                1,
                SettingOperationV1::Set(SettingValueV1::Number(value)),
            )
        })
        .collect();
    let p = o.prepare_settings_change(&changes, &mut r).unwrap();
    let outcome = o.commit_settings_change(p, &mut r).unwrap();
    assert_eq!(outcome.changed_layers, 1);
    assert_eq!(outcome.changed_domains, 3);
    assert_eq!(o.settings_layer_generation(&id("left"), &mut r).unwrap(), 2);
    assert_eq!(
        o.settings_generations(&a, &mut r).unwrap(),
        SettingsGenerationsV1 {
            source: 2,
            serialization: 2,
            analysis: 2
        }
    );
    assert_eq!(o.settings_generations(&b, &mut r).unwrap(), b_before);
}
#[test]
fn multi_layer_failure_never_leaves_an_allowed_prefix_or_refreshes_expected_generations() {
    let (mut o, mut r) = setup();
    let g = o.generation();
    let before = o.settings_basis(&mut r).unwrap();
    assert_eq!(
        o.prepare_settings_change(
            &[
                change(
                    "left",
                    1,
                    SettingOperationV1::Set(SettingValueV1::Number(2.0))
                ),
                change(
                    "right",
                    1,
                    SettingOperationV1::Set(SettingValueV1::Number(9.0))
                )
            ],
            &mut r
        )
        .err(),
        Some(SettingsError::InvalidValue)
    );
    assert_eq!(o.generation(), g);
    assert_eq!(o.settings_basis(&mut r).unwrap(), before);
    assert_eq!(
        o.prepare_settings_change(
            &[
                change("left", 1, SettingOperationV1::Reset),
                change("left", 1, SettingOperationV1::Remove)
            ],
            &mut r
        )
        .err(),
        Some(SettingsError::Duplicate)
    );
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
    set(&mut o, "left", 3.0, &mut r);
    set(&mut o, "left", 1.0, &mut r);
    let g = o.generation();
    assert_eq!(
        o.commit_settings_change(p, &mut r),
        Err(SettingsError::Stale)
    );
    assert_eq!(o.generation(), g);
    assert!(!o.is_faulted());
}
#[test]
fn recompaction_does_not_accumulate_obsolete_value_nodes_across_updates() {
    let (mut o, mut r) = setup();
    let first = set(&mut o, "left", 2.0, &mut r);
    assert!(first.changed);
    let usage = o.settings_usage().unwrap();
    for i in 0..100 {
        set(&mut o, "left", if i % 2 == 0 { 3.0 } else { 2.0 }, &mut r);
        assert_eq!(o.settings_usage().unwrap(), usage);
    }
}
#[test]
fn request_and_plan_bounds_are_explicit_and_refusals_leave_current_state_unchanged() {
    for p in [
        SettingsActivationLimitsV1 {
            max_changes: 0,
            ..profile()
        },
        SettingsActivationLimitsV1 {
            max_request_nodes: 0,
            ..profile()
        },
        SettingsActivationLimitsV1 {
            max_request_bytes: 0,
            ..profile()
        },
        SettingsActivationLimitsV1 {
            max_plan_bytes: 0,
            ..profile()
        },
        SettingsActivationLimitsV1 {
            max_work: 0,
            ..profile()
        },
    ] {
        let (o, mut r) = setup_with(p, false);
        let g = o.generation();
        assert!(
            o.prepare_settings_change(
                &[change(
                    "left",
                    1,
                    SettingOperationV1::Set(SettingValueV1::Number(2.0))
                )],
                &mut r
            )
            .is_err()
        );
        assert_eq!(o.generation(), g);
        assert_eq!(read(&o, "A", &mut r), 1.0);
    }
}
#[test]
fn foreign_preparation_and_source_interleavings_respect_engine_boundaries_and_optional_capture() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _) = setup();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let source = o.create(b"A", &mut r).unwrap();
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
        let source_plan = o
            .prepare(
                &[DocumentEdits {
                    expected: source.clone(),
                    edits: vec![vellum_engine::source::Replacement {
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
        o.commit(source_plan, &mut r).unwrap();
        o.commit_settings_change(p, &mut r).unwrap();
        assert_eq!(o.read(&source.document, &mut r).unwrap().source.text(), "a");
        assert_eq!(read(&o, "A", &mut r), 2.0);
        let (mut foreign, _) = setup();
        let p = o.prepare_settings_change(&[], &mut r).unwrap();
        assert_eq!(
            foreign.commit_settings_change(p, &mut r),
            Err(SettingsError::Unavailable)
        );
        assert!(!foreign.is_faulted());
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(128) {
            for boundary in [
                Boundary::SettingsChangePreparation,
                Boundary::SettingsChangeValidation,
                Boundary::SettingsChangePublication,
            ] {
                assert!(records.iter().any(|event| event.boundary == boundary));
            }
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
    }
}
#[test]
fn finite_set_remove_reset_traces_match_independent_metadata_and_domain_counters() {
    for retained in [false, true] {
        for encoded in 0..125 {
            let (mut o, mut r) = setup();
            let target = o.settings_target(&id("A"), &mut r).unwrap();
            let mut code = encoded;
            let mut authored: Option<Option<u8>> = None;
            let mut effective = 1u8;
            let mut metadata = 1u64;
            let mut domain = 1u64;
            for _ in 0..3 {
                let choice = code % 5;
                code /= 5;
                let next = match choice {
                    0 => None,
                    1 => Some(None),
                    _ => Some(Some((choice - 1) as u8)),
                };
                let value = next.flatten().unwrap_or(1);
                let changed = authored != next;
                let operation = match next {
                    None => SettingOperationV1::Remove,
                    Some(None) => SettingOperationV1::Reset,
                    Some(Some(value)) => {
                        SettingOperationV1::Set(SettingValueV1::Number(f64::from(value)))
                    }
                };
                let changes = [change("left", metadata, operation)];
                let outcome = if retained {
                    let invocation = o.issue_invocation(&mut r).unwrap();
                    o.submit_settings_change(&invocation, &changes, &mut r)
                        .unwrap();
                    o.run(&invocation, &mut r).unwrap();
                    let InvocationOutcome::SettingsCommitted(outcome) =
                        o.invocation_outcome(&invocation, &mut r).unwrap()
                    else {
                        panic!("finite settings success")
                    };
                    *outcome
                } else {
                    let p = o.prepare_settings_change(&changes, &mut r).unwrap();
                    o.commit_settings_change(p, &mut r).unwrap()
                };
                assert_eq!(outcome.changed, changed);
                if changed {
                    metadata += 1;
                }
                if value != effective {
                    domain += 1;
                }
                assert_eq!(outcome.basis.configuration_generation, metadata);
                assert_eq!(
                    o.settings_layer_generation(&id("left"), &mut r).unwrap(),
                    metadata
                );
                assert_eq!(
                    o.settings_generations(&target, &mut r).unwrap().source,
                    domain
                );
                assert_eq!(read(&o, "A", &mut r), f64::from(value));
                assert_eq!(read(&o, "B", &mut r), 1.0);
                authored = next;
                effective = value;
            }
        }
    }
}
#[test]
fn settings_jobs_preserve_results_with_disabled_available_or_saturated_diagnostics() {
    use vellum_engine::instrumentation::{Boundary, Disposition};
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _) = setup();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let source = o.create(b"source unchanged", &mut r).unwrap();
        let invocation = o.issue_invocation(&mut r).unwrap();
        let changes = [change(
            "left",
            1,
            SettingOperationV1::Set(SettingValueV1::Number(2.0)),
        )];
        o.submit_settings_change(&invocation, &changes, &mut r)
            .unwrap();
        // Source publication is an independent frame, not an implicit settings dependency.
        let p = o
            .prepare(
                &[DocumentEdits {
                    expected: source.clone(),
                    edits: vec![vellum_engine::source::Replacement {
                        start: 0,
                        end: 6,
                        expected: "source".into(),
                        inserted: "SOURCE".into(),
                    }],
                }],
                &[],
                &mut r,
            )
            .unwrap();
        o.commit(p, &mut r).unwrap();
        o.run(&invocation, &mut r).unwrap();
        o.submit_settings_change(&invocation, &changes, &mut r)
            .unwrap();
        o.run(&invocation, &mut r).unwrap();
        assert_eq!(read(&o, "A", &mut r), 2.0);
        assert_eq!(
            o.read(&source.document, &mut r).unwrap().source.text(),
            "SOURCE unchanged"
        );
        let bad = o.issue_invocation(&mut r).unwrap();
        let mut changes = [change("left", 2, SettingOperationV1::Remove)];
        changes[0].key = "unknown-sensitive-name".into();
        o.submit_settings_change(&bad, &changes, &mut r).unwrap();
        o.run(&bad, &mut r).unwrap();
        assert_eq!(
            o.invocation_outcome(&bad, &mut r),
            Ok(&InvocationOutcome::SettingsRejected(
                SettingsError::UnknownKey
            ))
        );
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(128) {
            for boundary in [
                Boundary::InvocationAdmission,
                Boundary::SettingsChangePreparation,
                Boundary::SettingsChangeValidation,
                Boundary::SettingsChangePublication,
                Boundary::InvocationExecution,
                Boundary::InvocationReplay,
                Boundary::Outcome,
            ] {
                assert!(records.iter().any(|record| record.boundary == boundary));
            }
            assert!(records.iter().any(|record| record.boundary
                == Boundary::SettingsChangePublication
                && record.disposition == Disposition::Refused));
            assert!(!format!("{records:?}").contains("unknown-sensitive-name"));
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
        if capacity.is_none() {
            assert!(records.is_empty());
        }
    }
}
