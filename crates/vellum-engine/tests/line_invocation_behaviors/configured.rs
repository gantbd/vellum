use super::*;
#[path = "configured/arguments.rs"]
mod arguments;
use vellum_engine::settings::*;
fn id(name: &str) -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: name.into(),
        incarnation: 1,
    }
}
fn setup() -> (
    SourceCoordinator,
    LogicalViewContext,
    SettingsTargetId,
    Recorder,
) {
    setup_capacity(16, 8)
}
fn setup_capacity(
    issued: usize,
    retained: usize,
) -> (
    SourceCoordinator,
    LogicalViewContext,
    SettingsTargetId,
    Recorder,
) {
    let (mut o, c, mut r) = fixture(16384);
    let mut schemas = vec![
        SettingSchemaV1 {
            key: "prefix".into(),
            value_type: SettingTypeV1::Text,
            default: SettingValueV1::Text("  ".into()),
            domain: SettingDomainV1::Source,
            scopes: vec![SettingScopeV1::Host, SettingScopeV1::Workspace],
            composition: SettingCompositionV1::Replace,
            sensitive: true,
        },
        SettingSchemaV1 {
            key: "columns".into(),
            value_type: SettingTypeV1::Integer { min: 0, max: 16 },
            default: SettingValueV1::Number(4.0),
            domain: SettingDomainV1::Source,
            scopes: vec![SettingScopeV1::Host, SettingScopeV1::Workspace],
            composition: SettingCompositionV1::Replace,
            sensitive: false,
        },
    ];
    for (key, values, default) in [
        ("delimiter", vec!["LF", "CRLF", "CR"], "CRLF"),
        ("auto", vec!["none", "copy-prefix"], "copy-prefix"),
    ] {
        schemas.push(SettingSchemaV1 {
            key: key.into(),
            value_type: SettingTypeV1::Enum(
                values
                    .into_iter()
                    .map(|v| SettingValueV1::Text(v.into()))
                    .collect(),
            ),
            default: SettingValueV1::Text(default.into()),
            domain: SettingDomainV1::Source,
            scopes: vec![SettingScopeV1::Host, SettingScopeV1::Workspace],
            composition: SettingCompositionV1::Replace,
            sensitive: false,
        });
    }
    schemas.push(SettingSchemaV1 {
        key: "analysis".into(),
        value_type: SettingTypeV1::Integer { min: 0, max: 16 },
        default: SettingValueV1::Number(0.0),
        domain: SettingDomainV1::Analysis,
        scopes: vec![SettingScopeV1::Host, SettingScopeV1::Workspace],
        composition: SettingCompositionV1::Replace,
        sensitive: false,
    });
    let d = SettingsDefinitionV1 {
        schemas,
        restrictions: vec![],
        layers: [
            ("host", SettingScopeV1::Host),
            ("left", SettingScopeV1::Workspace),
            ("right", SettingScopeV1::Workspace),
        ]
        .into_iter()
        .map(|(name, scope)| SettingLayerV1 {
            identity: id(name),
            scope,
            generation: 1,
            values: vec![],
        })
        .collect(),
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
    o.install_initial_settings(
        compile_settings_v1(
            &d,
            "engine",
            SettingsLimitsV1 {
                max_keys: 16,
                max_layers: 8,
                max_targets: 8,
                max_identifier_bytes: 64,
                max_schema_nodes: 128,
                max_schema_depth: 32,
                max_value_nodes: 256,
                max_value_depth: 32,
                max_state_bytes: 65536,
                max_work: 1_048_576,
            },
            &mut r,
        )
        .unwrap(),
        &mut r,
    )
    .unwrap();
    o.configure_semantic_root_budget(1_048_576, &mut r).unwrap();
    o.configure_settings_activation(
        SettingsActivationLimitsV1 {
            max_changes: 16,
            max_request_nodes: 128,
            max_request_depth: 32,
            max_request_bytes: 65536,
            max_plan_bytes: 262144,
            max_work: 1_048_576,
        },
        &mut r,
    )
    .unwrap();
    let p = SettingsRetentionLimitsV1 {
        max_issued: issued,
        max_retained: retained,
        max_snapshot_bytes: 65536,
        max_pool_bytes: 1_048_576,
    };
    o.configure_retention_budget(p.required_pool_bytes().unwrap(), &mut r)
        .unwrap();
    o.configure_settings_retention(p, &mut r).unwrap();
    o.configure_configured_line_edits(
        ConfiguredLineLimitsV1 {
            max_identifier_bytes: 64,
            max_request_bytes: 4096,
            max_inner_intent_bytes: 4096,
        },
        &mut r,
    )
    .unwrap();
    let target = o.settings_target(&id("A"), &mut r).unwrap();
    (o, c, target, r)
}
fn request(target: &SettingsTargetId, action: usize) -> ConfiguredLineEditV1 {
    match action {
        0 => ConfiguredLineEditV1::Indent {
            target: target.clone(),
            unit_key: "prefix".into(),
            tab_width_key: "columns".into(),
        },
        1 => ConfiguredLineEditV1::Outdent {
            target: target.clone(),
            unit_key: "prefix".into(),
            tab_width_key: "columns".into(),
        },
        _ => ConfiguredLineEditV1::Newline {
            target: target.clone(),
            delimiter_key: "delimiter".into(),
            auto_indent_key: "auto".into(),
        },
    }
}
fn set(o: &mut SourceCoordinator, layer: &str, key: &str, value: SettingValueV1, r: &mut Recorder) {
    let p = o
        .prepare_settings_change(
            &[SettingChangeV1 {
                layer: id(layer),
                expected_generation: o.settings_layer_generation(&id(layer), r).unwrap(),
                key: key.into(),
                operation: SettingOperationV1::Set(value),
            }],
            r,
        )
        .unwrap();
    o.commit_settings_change(p, r).unwrap();
}
fn text(o: &SourceCoordinator, c: &LogicalViewContext, r: &mut Recorder) -> String {
    o.read(&c.source.document, r).unwrap().source.text().into()
}
#[test]
fn all_configured_actions_use_existing_native_publication_and_exact_history() {
    for (action, expected) in [
        (0, "   \talpha\r\n"),
        (1, "  alpha\r\n"),
        (2, " \r\n \talpha\r\n"),
    ] {
        let (mut o, c, target, mut r) = setup();
        let invocation = o.issue_invocation(&mut r).unwrap();
        o.submit_configured_line_edit(&invocation, &c, &request(&target, action), &mut r)
            .unwrap();
        assert_eq!(o.settings_retention_usage().retained, 1);
        o.run(&invocation, &mut r).unwrap();
        assert_eq!(text(&o, &c, &mut r), expected);
        let InvocationOutcome::Committed(outcome) =
            o.invocation_outcome(&invocation, &mut r).unwrap()
        else {
            panic!("configured publication")
        };
        let group = outcome.history_group.clone().unwrap();
        let config = o.settings_basis(&mut r).unwrap();
        let undo = o
            .prepare_reversal(&group, HistoryDirection::Undo, &mut r)
            .unwrap();
        o.commit(undo, &mut r).unwrap();
        assert_eq!(text(&o, &c, &mut r), " \talpha\r\n");
        assert_eq!(o.settings_basis(&mut r).unwrap(), config);
        o.retire_invocation(&invocation, &mut r).unwrap();
        assert_eq!(o.settings_retention_usage().retained, 0);
    }
}
#[test]
fn original_source_domain_and_selection_footprints_refuse_aba_but_preserve_disjoint_progress() {
    for change in 0..7 {
        let (mut o, c, target, mut r) = setup();
        let invocation = o.issue_invocation(&mut r).unwrap();
        o.submit_configured_line_edit(&invocation, &c, &request(&target, 0), &mut r)
            .unwrap();
        match change {
            0 => set(
                &mut o,
                "right",
                "prefix",
                SettingValueV1::Text("   ".into()),
                &mut r,
            ),
            1 => set(
                &mut o,
                "left",
                "analysis",
                SettingValueV1::Number(1.0),
                &mut r,
            ),
            2 => set(
                &mut o,
                "left",
                "prefix",
                SettingValueV1::Text("  ".into()),
                &mut r,
            ),
            3 => set(
                &mut o,
                "host",
                "prefix",
                SettingValueV1::Text("   ".into()),
                &mut r,
            ),
            4 => {
                set(
                    &mut o,
                    "left",
                    "prefix",
                    SettingValueV1::Text(" ".into()),
                    &mut r,
                );
                set(
                    &mut o,
                    "left",
                    "prefix",
                    SettingValueV1::Text("  ".into()),
                    &mut r,
                );
            }
            5 => {
                o.set_logical_selections(&c, &[point(2)], 0, &mut r)
                    .unwrap();
            }
            _ => set(
                &mut o,
                "left",
                "delimiter",
                SettingValueV1::Text("LF".into()),
                &mut r,
            ),
        }
        let generation = o.generation();
        o.run(&invocation, &mut r).unwrap();
        if change < 3 {
            assert!(matches!(
                o.invocation_outcome(&invocation, &mut r),
                Ok(InvocationOutcome::Committed(_))
            ));
            assert_eq!(text(&o, &c, &mut r), "   \talpha\r\n");
        } else {
            assert!(matches!(
                o.invocation_outcome(&invocation, &mut r),
                Ok(InvocationOutcome::ConfiguredLineRejected(_))
            ));
            assert_eq!(o.generation(), generation);
            assert_eq!(text(&o, &c, &mut r), " \talpha\r\n");
        }
        assert!(!o.is_faulted());
    }
}
#[test]
fn retry_stop_forget_and_protected_retirement_do_not_reacquire_or_refund_issuance() {
    for stopped in [false, true] {
        let (mut o, c, target, mut r) = setup();
        let invocation = o.issue_invocation(&mut r).unwrap();
        let req = request(&target, 0);
        o.submit_configured_line_edit(&invocation, &c, &req, &mut r)
            .unwrap();
        assert_eq!(
            o.release_configured_line_retention(&invocation, &mut r),
            Err(InvocationError::Pending)
        );
        o.submit_configured_line_edit(&invocation, &c, &req, &mut r)
            .unwrap();
        assert_eq!(o.settings_retention_usage().issued_lifetime, 1);
        assert_eq!(
            o.submit_settings_change(&invocation, &[], &mut r),
            Err(InvocationError::IntentMismatch)
        );
        assert_eq!(
            o.submit_line_edit(&invocation, &c, &outdent(), &mut r),
            Err(InvocationError::IntentMismatch)
        );
        assert_eq!(
            o.submit_configured_line_edit(&invocation, &c, &request(&target, 1), &mut r),
            Err(InvocationError::IntentMismatch)
        );
        if stopped {
            o.stop_invocation(&invocation, &mut r).unwrap();
        } else {
            o.run(&invocation, &mut r).unwrap();
        }
        o.forget_invocation_outcome(&invocation, &mut r).unwrap();
        assert_eq!(o.settings_retention_usage().retained, 1);
        assert_eq!(
            o.retire_settings_retention(&mut r),
            Err(SettingsError::Unavailable)
        );
        assert_eq!(
            o.release_configured_line_retention(&invocation, &mut r),
            Ok(true)
        );
        assert_eq!(
            o.release_configured_line_retention(&invocation, &mut r),
            Ok(false)
        );
        o.submit_configured_line_edit(&invocation, &c, &req, &mut r)
            .unwrap();
        o.run(&invocation, &mut r).unwrap();
        assert_eq!(o.settings_retention_usage().issued_lifetime, 1);
        assert_eq!(o.settings_retention_usage().retained, 0);
        o.retire_invocation(&invocation, &mut r).unwrap();
        o.retire_settings_retention(&mut r).unwrap();
    }
}
#[test]
fn invalid_keys_domains_types_values_or_foreign_targets_never_issue_interpretations() {
    for invalid in 0..6 {
        let (mut o, c, mut target, mut r) = setup();
        if invalid == 4 {
            let (foreign, _, t, _) = setup();
            target = t;
            drop(foreign);
        }
        if invalid == 5 {
            set(
                &mut o,
                "left",
                "prefix",
                SettingValueV1::Text("\n".into()),
                &mut r,
            );
        }
        let req = ConfiguredLineEditV1::Indent {
            target,
            unit_key: match invalid {
                0 => "missing",
                1 => "analysis",
                2 => "columns",
                _ => "prefix",
            }
            .into(),
            tab_width_key: if invalid == 3 { "prefix" } else { "columns" }.into(),
        };
        let invocation = o.issue_invocation(&mut r).unwrap();
        let generation = o.generation();
        o.submit_configured_line_edit(&invocation, &c, &req, &mut r)
            .unwrap();
        o.run(&invocation, &mut r).unwrap();
        assert!(matches!(
            o.invocation_outcome(&invocation, &mut r),
            Ok(InvocationOutcome::ConfiguredLineRejected(_))
        ));
        assert_eq!(o.settings_retention_usage().issued_lifetime, 0);
        assert_eq!(o.generation(), generation);
        assert_eq!(text(&o, &c, &mut r), " \talpha\r\n");
    }
}
#[test]
fn no_op_outdent_still_checks_settings_without_fabricating_source_or_history_versions() {
    for stale in [false, true] {
        let (mut o, c, target, mut r) = setup();
        o.set_logical_selections(&c, &[point(9)], 0, &mut r)
            .unwrap();
        let c = o.logical_view(&c.view, &mut r).unwrap().context().unwrap();
        let invocation = o.issue_invocation(&mut r).unwrap();
        o.submit_configured_line_edit(&invocation, &c, &request(&target, 1), &mut r)
            .unwrap();
        if stale {
            set(
                &mut o,
                "left",
                "prefix",
                SettingValueV1::Text(" ".into()),
                &mut r,
            );
        }
        let generation = o.generation();
        o.run(&invocation, &mut r).unwrap();
        if stale {
            assert_eq!(
                o.invocation_outcome(&invocation, &mut r),
                Ok(&InvocationOutcome::ConfiguredLineRejected(
                    ConfiguredLineErrorV1::Coordinator(CoordinatorError::Stale)
                ))
            );
        } else {
            let InvocationOutcome::Committed(outcome) =
                o.invocation_outcome(&invocation, &mut r).unwrap()
            else {
                panic!("no-op")
            };
            assert!(outcome.operation.is_none());
            assert!(outcome.history_group.is_none());
        }
        assert_eq!(o.generation(), generation);
        assert_eq!(o.history_usage().retained_groups, 0);
    }
}
#[test]
fn finite_configured_prefixes_match_independent_column_oracle_and_undo_original_bytes() {
    for unit in [" ", "  ", "   ", "\t"] {
        for width in [1usize, 2, 4, 8] {
            for action in [0, 1] {
                let (mut o, c, target, mut r) = setup();
                set(
                    &mut o,
                    "left",
                    "prefix",
                    SettingValueV1::Text(unit.into()),
                    &mut r,
                );
                set(
                    &mut o,
                    "left",
                    "columns",
                    SettingValueV1::Number(width as f64),
                    &mut r,
                );
                let expected = if action == 0 {
                    format!("{unit} \talpha\r\n")
                } else {
                    let columns = (1 / width + 1) * width;
                    let step = if unit == "\t" { width } else { unit.len() };
                    let n = columns.saturating_sub(step);
                    let prefix = if unit == "\t" {
                        format!("{}{}", "\t".repeat(n / width), " ".repeat(n % width))
                    } else {
                        " ".repeat(n)
                    };
                    format!("{prefix}alpha\r\n")
                };
                let invocation = o.issue_invocation(&mut r).unwrap();
                o.submit_configured_line_edit(&invocation, &c, &request(&target, action), &mut r)
                    .unwrap();
                o.run(&invocation, &mut r).unwrap();
                assert_eq!(text(&o, &c, &mut r), expected);
                let InvocationOutcome::Committed(outcome) =
                    o.invocation_outcome(&invocation, &mut r).unwrap()
                else {
                    panic!("oracle publication")
                };
                let group = outcome.history_group.clone().unwrap();
                let undo = o
                    .prepare_reversal(&group, HistoryDirection::Undo, &mut r)
                    .unwrap();
                o.commit(undo, &mut r).unwrap();
                assert_eq!(text(&o, &c, &mut r), " \talpha\r\n");
            }
        }
    }
}
#[test]
fn quota_or_structural_refusal_has_no_lease_or_publication_prefix() {
    let (mut o, c, target, mut r) = setup_capacity(1, 0);
    let invocation = o.issue_invocation(&mut r).unwrap();
    let generation = o.generation();
    o.submit_configured_line_edit(&invocation, &c, &request(&target, 0), &mut r)
        .unwrap();
    o.run(&invocation, &mut r).unwrap();
    assert_eq!(
        o.invocation_outcome(&invocation, &mut r),
        Ok(&InvocationOutcome::ConfiguredLineRejected(
            ConfiguredLineErrorV1::Settings(SettingsError::Limit)
        ))
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(o.settings_retention_usage().issued_lifetime, 0);
    let empty = ConfiguredLineEditV1::Indent {
        target,
        unit_key: String::new(),
        tab_width_key: "columns".into(),
    };
    let invocation = o.issue_invocation(&mut r).unwrap();
    assert_eq!(
        o.submit_configured_line_edit(&invocation, &c, &empty, &mut r),
        Err(InvocationError::MalformedIntent)
    );
    assert_eq!(
        o.invocation_state(&invocation, &mut r),
        Ok(InvocationState::Issued)
    );
}
#[test]
fn configured_execution_and_retention_are_identical_with_optional_content_free_capture() {
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, c, target, _) = setup();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let invocation = o.issue_invocation(&mut r).unwrap();
        o.submit_configured_line_edit(&invocation, &c, &request(&target, 0), &mut r)
            .unwrap();
        o.run(&invocation, &mut r).unwrap();
        o.retire_invocation(&invocation, &mut r).unwrap();
        assert_eq!(text(&o, &c, &mut r), "   \talpha\r\n");
        assert_eq!(o.settings_retention_usage().retained, 0);
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(128) {
            for boundary in [
                Boundary::ConfiguredLinePreparation,
                Boundary::ConfiguredLineValidation,
                Boundary::SettingsInterpretationAcquisition,
                Boundary::SettingsInterpretationRelease,
                Boundary::InvocationExecution,
            ] {
                assert!(records.iter().any(|record| record.boundary == boundary));
            }
            assert!(!format!("{records:?}").contains("alpha"));
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
        if capacity.is_none() {
            assert!(records.is_empty());
        }
    }
}
